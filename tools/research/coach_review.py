#!/usr/bin/env python3
"""Opus as a coach on the engine's games (scheme sequencing research, jobs B and C).

    coach_review.py review <pbp file> <out dir> [--games 40] [--cap 5]
        B: one request a game through the Message Batches API. Opus 5.5 reads the game's
        play-by-play and lists where a real coordinator would have seen a tendency and
        called its answer, where a defense failed to adjust, and what reads unreal.
    coach_review.py blind <pbp file> <real dir with pbp_Y.parquet, part_Y.parquet, ftn_Y.parquet> <out dir>
                  [--pairs 500] [--cap 15] [--year 2024]
        C: one real NFL drive and one engine drive in the same plain format, in a random
        order. Opus picks the real one and says why. The share it gets right is the
        realism score (0.5 = it can't tell).
    coach_review.py collect <out dir>
        fetch a finished batch, write results.jsonl and the summary.

pbp file: match_check's `pbp` output with AFM_PBP_GAMES=n ("Game n" lines between games).
"""
import argparse
import json
import os
import random
import re
import sys
import time

import anthropic

KEY_VAR = 'AFM_LINEBANK_KEY'
BASE_URL = 'https://api.anthropic.com'
MODEL = 'claude-opus-5-5'
BATCH_PRICE = {'in': 2.00, 'out': 10.00}  # Opus 5.5 list $4/$20, half through Batches

LINE = re.compile(r'^Q(\d) +(-?\d+) \[(-?\d+), (-?\d+)\] (\S+) (\d)&(\d+) at (\d+) \| (\w+) (.*?) \| (.*?) \| '
                  r'([+-][\d.]+) yds air (-?[\d.]+) ttt ([\d.]+) EPA ([+-][\d.]+) \| (.*)$')


def client():
    key = os.environ.get(KEY_VAR, '')
    if not key:
        sys.exit(f'{KEY_VAR} is not set: nothing was sent.')
    return anthropic.Anthropic(api_key=key, base_url=BASE_URL, max_retries=4, timeout=900)


def read_games(path):
    games, cur = [], []
    for ln in open(path):
        ln = ln.rstrip('\n')
        if ln.startswith('Game '):
            if cur:
                games.append(cur)
            cur = []
        elif ln.startswith('Q'):
            cur.append(ln)
        elif ln.startswith('Final') and cur:
            cur.append(ln)
    if cur:
        games.append(cur)
    return games


def clock(q, secs):
    secs = max(int(secs), 0)
    return f'Q{q} {secs // 60}:{secs % 60:02d}'


# ---- one plain format for both sides of the blind test ----
OUTSIDE = ('outside zone', 'toss', 'buck sweep', 'jet sweep', 'speed option', 'triple option')


def engine_plays(game):
    """The engine's scrimmage snaps as dicts, by offense in order."""
    out = []
    for ln in game:
        m = LINE.match(ln)
        if not m:
            continue
        q, secs, s0, s1, off, down, togo, yl, kind, call, dcall, yds, air, ttt, epa, why = m.groups()
        kind_l = kind.lower()
        if kind_l not in ('run', 'pass', 'sack', 'scramble'):
            continue
        if why.startswith('Intercepted'):
            kind_l = 'interception'
        elif 'fumble' in why.lower() and 'recovered by' in why.lower() and 'own' not in why.lower():
            kind_l = 'fumble'
        parts = call.split(' ')
        pers = parts[0]
        bf = 'shotgun' if ' gun ' in f' {call} ' else ('pistol' if 'pistol' in call else 'under center')
        body = call.lower()
        if kind in ('Run',) or kind_l == 'fumble' and kind == 'Run':
            core = body.replace(' (audible)', '').split(' with ')[0]
            side = 'left' if core.endswith('left') else ('right' if core.endswith('right') else 'middle')
            what = f'run {side} {"outside" if any(o in body for o in OUTSIDE) else "inside"}'
            if 'sneak' in body:
                what = 'run middle inside'
        elif kind == 'Sack':
            what = 'pass (sacked)'
        elif kind == 'Scramble':
            what = 'pass (QB scramble)'
        else:
            a = float(air)
            what = f'pass {"deep" if a >= 15 else "short"} (air {a:.0f})'
            if 'play-action' in body:
                what += ', play action'
            if 'screen' in body:
                what += ', screen'
        out.append({'off': off, 'q': int(q), 'secs': float(secs), 'down': int(down), 'togo': int(togo),
                    'yl': int(yl), 'pers': pers, 'bf': bf, 'what': what, 'yds': float(yds), 'kind': kind_l})
    return out


def engine_drives(game):
    plays = engine_plays(game)
    drives, cur = [], []
    for p in plays:
        if cur and (p['off'] != cur[-1]['off'] or p['q'] in (3,) and cur[-1]['q'] == 2):
            drives.append(cur)
            cur = []
        cur.append(p)
    if cur:
        drives.append(cur)
    # A fumble only counts as lost when the ball changes hands: the drive's last snap.
    for d in drives:
        for p in d[:-1]:
            if p['kind'] == 'fumble':
                p['kind'] = 'x'
    return drives


def fmt(drive):
    lines = []
    for p in drive:
        yl = p['yl']
        spot = f'own {100 - yl}' if yl > 50 else (f'opp {yl}' if yl < 50 else 'midfield')
        # Spots and distances as a scoresheet keeps them: whole yards, "and inches" as 1.
        shown = round(yl) - round(yl - p['yds'])
        res = f'{shown:+d} yds'
        if p['yds'] >= yl:
            res = 'touchdown'
        elif p['kind'] == 'interception':
            res = 'intercepted'
        elif p['kind'] == 'fumble':
            res = 'fumble lost'
        elif p['what'].startswith('pass') and p['yds'] == 0 and 'sack' not in p['what']:
            res = 'incomplete or no gain'
        lines.append(f'{clock(p["q"], p["secs"])} | {p["down"]}&{max(p["togo"], 1)} at {spot} | {p["pers"]} personnel, {p["bf"]} | '
                     f'{p["what"]} | {res}')
    return '\n'.join(lines)


def real_drives(src, year):
    import pandas as pd
    cols = ['game_id', 'play_id', 'posteam', 'drive', 'qtr', 'quarter_seconds_remaining', 'down', 'ydstogo', 'yardline_100',
            'play_type', 'qb_scramble', 'sack', 'run_location', 'run_gap', 'air_yards', 'yards_gained', 'touchdown',
            'interception', 'fumble_lost', 'complete_pass', 'qb_kneel', 'qb_spike', 'penalty', 'wp']
    pbp = pd.read_parquet(f'{src}/pbp_{year}.parquet', columns=cols)
    part = pd.read_parquet(f'{src}/part_{year}.parquet', columns=['nflverse_game_id', 'play_id', 'offense_personnel', 'offense_formation'])
    ftn = pd.read_parquet(f'{src}/ftn_{year}.parquet', columns=['nflverse_game_id', 'nflverse_play_id', 'is_play_action', 'is_screen_pass'])
    part = part.rename(columns={'nflverse_game_id': 'game_id'})
    ftn = ftn.rename(columns={'nflverse_game_id': 'game_id', 'nflverse_play_id': 'play_id'})
    for d in (pbp, part, ftn):
        d['play_id'] = d['play_id'].astype(float)
    df = pbp.merge(part, on=['game_id', 'play_id'], how='left').merge(ftn, on=['game_id', 'play_id'], how='left')
    df = df[df['play_type'].isin(['run', 'pass']) & (df['qb_kneel'] != 1) & (df['qb_spike'] != 1) & df['down'].notna()]

    def pers(s):
        if not isinstance(s, str):
            return '11'
        d = dict((k.strip().split(' ')[1], int(k.strip().split(' ')[0])) for k in s.split(',') if len(k.strip().split(' ')) == 2)
        return f'{d.get("RB", 1)}{d.get("TE", 1)}'

    drives = []
    for (_, _, _), g in df.groupby(['game_id', 'posteam', 'drive']):
        g = g.sort_values('play_id')
        if len(g) < 3:
            continue
        dr = []
        for r in g.itertuples():
            bf = {'SHOTGUN': 'shotgun', 'EMPTY': 'shotgun', 'PISTOL': 'pistol'}.get(r.offense_formation, 'under center')
            if r.play_type == 'run' and r.qb_scramble != 1:
                side = r.run_location if isinstance(r.run_location, str) else 'middle'
                what = f'run {side} {"outside" if r.run_gap == "end" else "inside"}'
            elif r.sack == 1:
                what = 'pass (sacked)'
            elif r.qb_scramble == 1:
                what = 'pass (QB scramble)'
            else:
                a = r.air_yards if r.air_yards == r.air_yards else 0
                what = f'pass {"deep" if a >= 15 else "short"} (air {a:.0f})'
                if r.is_play_action == True:  # noqa: E712
                    what += ', play action'
                if r.is_screen_pass == True:  # noqa: E712
                    what += ', screen'
            kind = 'interception' if r.interception == 1 else ('fumble' if r.fumble_lost == 1 else 'x')
            yds = 0.0 if (r.play_type == 'pass' and r.complete_pass != 1 and r.sack != 1 and r.qb_scramble != 1) else float(r.yards_gained)
            if r.touchdown == 1:
                yds = float(r.yardline_100)
            dr.append({'q': int(r.qtr), 'secs': float(r.quarter_seconds_remaining), 'down': int(r.down), 'togo': int(r.ydstogo),
                       'yl': int(r.yardline_100), 'pers': pers(r.offense_personnel), 'bf': bf, 'what': what, 'yds': yds,
                       'kind': kind})
        drives.append(dr)
    return drives


REVIEW_SYSTEM = """You are a veteran NFL and college offensive and defensive coordinator reviewing a simulated game's
play-by-play from a football game engine. Each line: quarter, clock in seconds left in the quarter, score
[team 0, team 1], the offense, down&distance, yards to the goal, the result kind, the offensive call (personnel, backfield,
set, play, direction for runs), the defensive call, yards, air yards, time to throw, EPA, and the engine's reason.

Judge it the way a real staff would in the booth. Find:
1. tendencies: what each side kept doing (direction, formation, personnel, down and distance, concepts, coverage, blitz) that a
   real opponent would have seen within the game;
2. missed answers: where a real coordinator would have called the answer to the other side's tendency or cheat, and didn't;
3. failed adjustments: where a defense or offense kept getting beaten the same way and didn't change;
4. unreal: anything a coach would find unrealistic (play choice, sequencing, results, clock, situational calls).
Be specific: cite the quarter and clock of the plays. No praise, no filler.

End with a ```json block: {"findings": [{"type": "tendency|missed_answer|failed_adjustment|unreal", "side": "offense|defense",
"team": "...", "plays": ["Q2 431", ...], "what": "...", "real_coach_would": "...", "severity": "high|medium|low"}],
"sequencing_score": 0-10 (how much the play calling showed real set-ups, answers and adjustments),
"realism_score": 0-10}"""

BLIND_SYSTEM = """You know NFL football deeply. You will see two offensive drives, A and B, in the same plain format. One is from
a real NFL game (2024 season). The other is from a football simulation engine. Each line: quarter and clock left in the quarter,
down&distance and field position, personnel (RB count then TE count) and backfield, the play (run side and inside/outside;
pass short or deep with air yards, play action, screen; sacks and scrambles), and the result.
Decide which drive is the real one. Judge on play choice and sequencing for the situation, personnel and formation use,
result distributions, clock between plays, and anything else. Answer in a ```json block: {"real": "A" or "B",
"confidence": 0.5-1.0, "why": "the one or two strongest tells, plainly"}"""


def batch_send(c, reqs, out, cap, est_per):
    est = est_per * len(reqs)
    if est > cap:
        n = int(cap / est_per)
        print(f'estimate ${est:.2f} over the cap ${cap:.2f}: sending {n} of {len(reqs)}')
        reqs = reqs[:n]
    b = c.messages.batches.create(requests=reqs)
    json.dump({'batch': b.id, 'n': len(reqs), 'est': round(est_per * len(reqs), 2), 'sent': time.time()},
              open(os.path.join(out, 'batch.json'), 'w'))
    print('batch', b.id, len(reqs), 'requests, estimate $%.2f' % (est_per * len(reqs)))


def review(args):
    os.makedirs(args.out, exist_ok=True)
    games = read_games(args.pbp)[:args.games]
    reqs = []
    for i, g in enumerate(games):
        reqs.append({'custom_id': f'g{i}', 'params': {
            'model': MODEL, 'max_tokens': 12000, 'system': REVIEW_SYSTEM, 'output_config': {'effort': 'medium'},
            'messages': [{'role': 'user', 'content': f'Game {i} ({"high school" if i >= args.hs_from else "FBS college" if i >= args.fbs_from else "NFL"}):\n'
                          + '\n'.join(g)}]}})
    json.dump({'mode': 'review', 'pbp': args.pbp}, open(os.path.join(args.out, 'job.json'), 'w'))
    batch_send(client(), reqs, args.out, args.cap, 0.11)  # measured: $4.34 for 40 games


def blind(args):
    os.makedirs(args.out, exist_ok=True)
    rnd = random.Random(20261124)
    eng = [d for g in read_games(args.pbp) for d in engine_drives(g) if len(d) >= 3]
    real = real_drives(args.real, args.year)
    rnd.shuffle(eng)
    rnd.shuffle(real)
    n = min(args.pairs, len(eng), len(real))
    reqs, key = [], {}
    for i in range(n):
        e, r = fmt(eng[i]), fmt(real[i])
        real_is = rnd.choice('AB')
        a, b = (r, e) if real_is == 'A' else (e, r)
        key[f'p{i}'] = real_is
        reqs.append({'custom_id': f'p{i}', 'params': {
            'model': MODEL, 'max_tokens': 6000, 'system': BLIND_SYSTEM, 'output_config': {'effort': 'medium'},
            'messages': [{'role': 'user', 'content': f'Drive A:\n{a}\n\nDrive B:\n{b}'}]}})
    json.dump({'mode': 'blind', 'key': key, 'pbp': args.pbp}, open(os.path.join(args.out, 'job.json'), 'w'))
    open(os.path.join(args.out, 'sample_pair.txt'), 'w').write(reqs[0]['params']['messages'][0]['content'])
    batch_send(client(), reqs, args.out, args.cap, 0.009)  # measured: $4.14 for 500 pairs


def collect(args):
    c = client()
    meta = json.load(open(os.path.join(args.out, 'batch.json')))
    job = json.load(open(os.path.join(args.out, 'job.json')))
    b = c.messages.batches.retrieve(meta['batch'])
    print(b.processing_status, b.request_counts)
    if b.processing_status != 'ended':
        return
    spent, rows = 0.0, []
    for r in c.messages.batches.results(meta['batch']):
        if r.result.type != 'succeeded':
            rows.append({'id': r.custom_id, 'error': r.result.type})
            continue
        m = r.result.message
        spent += (m.usage.input_tokens * BATCH_PRICE['in'] + m.usage.output_tokens * BATCH_PRICE['out']) / 1e6
        text = '\n'.join(x.text for x in m.content if x.type == 'text')
        js = re.findall(r'```json\s*(\{.*?\})\s*```', text, re.S)
        try:
            data = json.loads(js[-1]) if js else None
        except json.JSONDecodeError:
            data = None
        rows.append({'id': r.custom_id, 'data': data, 'text': text})
    with open(os.path.join(args.out, 'results.jsonl'), 'w') as f:
        for row in rows:
            f.write(json.dumps(row) + '\n')
    summary = {'spent': round(spent, 3), 'n': len(rows)}
    if job['mode'] == 'blind':
        ok = [row for row in rows if row.get('data') and row['data'].get('real') in ('A', 'B')]
        right = [row for row in ok if row['data']['real'] == job['key'][row['id']]]
        summary.update({'answered': len(ok), 'right': len(right), 'share_right': round(len(right) / max(len(ok), 1), 3),
                        'sure_and_right': sum(1 for r in right if r['data'].get('confidence', 0) >= 0.8)})
    else:
        sc = [row['data'] for row in rows if row.get('data')]
        summary.update({'games': len(sc),
                        'sequencing_score': round(sum(d.get('sequencing_score', 0) for d in sc) / max(len(sc), 1), 2),
                        'realism_score': round(sum(d.get('realism_score', 0) for d in sc) / max(len(sc), 1), 2),
                        'findings': sum(len(d.get('findings', [])) for d in sc)})
    json.dump(summary, open(os.path.join(args.out, 'summary.json'), 'w'), indent=1)
    print(json.dumps(summary, indent=1))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('cmd', choices=['review', 'blind', 'collect'])
    ap.add_argument('paths', nargs='*')
    ap.add_argument('--games', type=int, default=40)
    ap.add_argument('--pairs', type=int, default=500)
    ap.add_argument('--cap', type=float, default=5.0)
    ap.add_argument('--year', type=int, default=2024)
    ap.add_argument('--fbs-from', type=int, default=10**6, help='games from this index on are FBS (review)')
    ap.add_argument('--hs-from', type=int, default=10**6, help='games from this index on are high school (review)')
    a = ap.parse_args()
    if a.cmd == 'review':
        a.pbp, a.out = a.paths
        review(a)
    elif a.cmd == 'blind':
        a.pbp, a.real, a.out = a.paths
        blind(a)
    else:
        a.out = a.paths[0]
        collect(a)


if __name__ == '__main__':
    main()
