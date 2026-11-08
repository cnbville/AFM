#!/usr/bin/env python3
"""The line bank run (line bank run proposal, Cain "yes on all" 2026-11-07):
about 100,000 new versions for the game's talks, written by Claude Sonnet 5.5
through the Message Batches API, checked by code (check.py) and by one
model checker pass, then merged into crates/afm-engine/data/lines/*_run.txt.

    run.py key                 is the key there, and does a free call accept it
    run.py dry [--pilot]       build round 1's requests and show their size and cost; no API call
    run.py pilot               the pilot: about 2,000 lines over every family, then a report
    run.py run                 the full run: rounds until every part is full or the cap is reached
    run.py status              what the state folder holds
    run.py merge               write the 8 <family>_run.txt files from the kept lines
    run.py report              write report.md (counts, keep rate, cost, sample lines)

The key is read from AFM_LINEBANK_KEY only (never ANTHROPIC_API_KEY, so a
Claude Code session can't take it as its own login) and never printed. The
state (batch ids, money spent, kept and dropped lines) lives in --state, so a
run that stops resumes where it was. Nothing is ever faked: no key, no run."""
import argparse
import collections
import json
import math
import os
import random
import re
import sys
import time
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import bank as B  # noqa: E402
import check as K  # noqa: E402
from parts import FACT_WORDS, FILES, FIRST, NEVER, PARTS, PRONOUN_WORDS, TAG_WORDS, allocation  # noqa: E402

MODEL = 'claude-sonnet-5-5'
BASE_URL = 'https://api.anthropic.com'      # set, because ANTHROPIC_BASE_URL is set on thread machines
KEY_VAR = 'AFM_LINEBANK_KEY'
PRICE_IN, PRICE_OUT = 1.00, 5.00            # $ per million tokens, Sonnet 5.5 through the Batches API (half price)
PER_REQ = 150                               # versions asked in one request (one part per request)
MAX_TOKENS = 5000                           # 150 versions are about 3,300 tokens
SHOW = 600                                  # at most this many existing versions shown per request
CHECK_PER_REQ = 400                         # lines one checker request reads
CHARS_PER_TOKEN = 2.7                       # until a batch measures it (the cost model says 3.0)
DONE_SHARE = 0.02                           # a part is full within 2% of its target
DEFAULT_STATE = '/mnt/project-files/afm/line-bank-api/run'

# The pilot: about 2,000 lines over every family, its parts picked to cover
# every kind of speaker (Claude's pick, proposed, not confirmed).
PILOT = {
    'talk.open.role': 150, 'talk.reply.praise.bad': 150, 'talk.close.flat': 120, 'gameday.open.fiery': 120,
    'gameday.praise': 150, 'gameday.word.straight': 120, 'gameday.captain.us': 120, 'gameday.quote': 100,
    'gameday.staff.tight': 100, 'staffmeet.case.run': 100, 'homevisit.ask.playing': 120,
    'hscall.ask.grades': 100, 'combine.press.bristles': 80, 'boss.mid.open': 100, 'interview.ask.plan': 80,
    'booster.want.rival': 70, 'takeover.leader.ego': 100, 'agent.far': 120,
}

BRIEFS = os.path.join(HERE, 'briefs')

# Each request of a part gets one angle, so the requests of one round don't
# all write the same lines.
ANGLES = {
    'player': ["high school players (tag hs), every position", "college players (tag college), stage tags "
               "where the words need them", "NFL players (tag nfl), vets, rookies and the practice squad",
               "untagged lines that fit any player at any level", "one personality tag per line, spread over "
               "all fifteen", "a position or group tag per line, with words that fit everyone under it",
               "short lines, two to eight words", "a light regional touch (one region tag per line) and plain, "
               "unshowy speech"],
    'coach': ["high school coaches (tag hs)", "college coaches (tag college)", "NFL coaches (tag nfl)",
              "untagged lines that fit any coach", "one personality tag per line (hot, calm, climber, loyal, "
              "old, young, expro, strict, warm, cold)", "short and to the point", "a light regional touch"],
    'cast': ["high school (tag hs or the role)", "college (tag college or the role)", "the NFL (tag nfl or the "
             "role)", "untagged lines that fit every level", "one personality tag per line (ego, hot, warm, "
             "cold, blunt, smooth, impatient, patient, meddler, tight, old, she)", "a light regional touch"],
    'you': ["high school (tag hs)", "college (tag college)", "the NFL (tag nfl)", "untagged lines that fit "
            "every level", "short lines", "a light regional touch"],
    'parent': ["untagged lines that fit every family", "one style tag per line", "a light regional touch",
               "short lines"],
    'agent': ["untagged lines that fit every agent", "straight agents", "slick agents", "hot or calm agents",
              "big agencies and one-man shops"],
}


# ---------------------------------------------------------------------------
# State

class State:
    def __init__(self, path):
        self.dir = path
        os.makedirs(os.path.join(path, 'pool'), exist_ok=True)
        os.makedirs(os.path.join(path, 'dropped'), exist_ok=True)
        os.makedirs(os.path.join(path, 'replies'), exist_ok=True)
        self.file = os.path.join(path, 'state.json')
        self.s = json.load(open(self.file)) if os.path.exists(self.file) else {
            'spent': 0.0, 'tokens_in': 0, 'tokens_out': 0, 'chars_in': 0, 'round': 0, 'batches': {},
            'written': 0, 'kept_code': 0, 'kept': 0}

    def save(self):
        tmp = self.file + '.tmp'
        json.dump(self.s, open(tmp, 'w'), indent=1)
        os.replace(tmp, self.file)

    def pool(self):
        out = {}
        d = os.path.join(self.dir, 'pool')
        for f in os.listdir(d):
            if f.endswith('.txt'):
                out[f[:-4]] = [x.rstrip('\n') for x in open(os.path.join(d, f)) if x.strip()]
        return out

    def add_pool(self, key, lines):
        with open(os.path.join(self.dir, 'pool', key + '.txt'), 'a') as fo:
            for x in lines:
                fo.write(x + '\n')

    def log(self, msg):
        line = time.strftime('%Y-%m-%d %H:%M:%S UTC ', time.gmtime()) + msg
        print(line, flush=True)
        with open(os.path.join(self.dir, 'log.txt'), 'a') as fo:
            fo.write(line + '\n')


# ---------------------------------------------------------------------------
# The API

def client():
    key = os.environ.get(KEY_VAR, '')
    if not key:
        sys.exit(f'{KEY_VAR} is not set in this session: nothing was sent. (A session sees a new environment '
                 'variable only once it starts fresh.)')
    import anthropic
    return anthropic.Anthropic(api_key=key, base_url=BASE_URL, max_retries=4)


def probe(c):
    """A free call (GET /v1/models): the key works, or the run stops."""
    import anthropic
    try:
        c.models.list(limit=1)
    except anthropic.AuthenticationError:
        sys.exit(f'The API turned the key down (401): check {KEY_VAR}. Nothing was spent.')
    except anthropic.APIConnectionError as e:
        sys.exit(f'Could not reach {BASE_URL}: {e}. Nothing was spent.')


def cost(tin, tout):
    return tin * PRICE_IN / 1e6 + tout * PRICE_OUT / 1e6


# ---------------------------------------------------------------------------
# Prompts

def system_prompt(family):
    rules = open(os.path.join(BRIEFS, 'rules.md')).read()
    fam = open(os.path.join(BRIEFS, family + '.md')).read()
    return rules.strip() + '\n\n' + fam.strip()


def part_card(key):
    p = PARTS[key]
    facts = sorted(f for f in p.facts if not f.startswith('coach_'))
    lines = [f'Part: {key}', f'What it is: {p.note}', f'Who says it: {p.who}. Tags: {TAG_WORDS[p.who]}.']
    tags = sorted(p.tags - {'social', 'modern', 'nil'})
    lines.append('Tags this part can carry (nothing else): ' + ' '.join(tags) + ', and modern or nil for words '
                 'of those eras' + (' (agents: never)' if p.who == 'agent' else ''))
    if p.when:
        lines.append('When-checks you may add (fact=value, at most two): ' + '; '.join(
            f'{k}=' + '|'.join(sorted(v)) for k, v in sorted(p.when.items())))
    if facts:
        lines.append('Facts you may use (nothing else): ' + '; '.join(
            '{' + f + '} ' + FACT_WORDS.get(f, '') for f in facts)
            + ('; ' + PRONOUN_WORDS if 'coach' in p.facts else ''))
    else:
        lines.append('Facts: none. Never write a {fact} in this part.')
    if p.levels:
        lines.append('It is said only at: ' + ', '.join(sorted(p.levels)) + '.')
    lines.append(f'Length: {p.words[0]} to {p.words[1]} words.')
    return '\n'.join(lines)


def user_prompt(key, n, angle, shown):
    out = [part_card(key), '', f"This request's angle: {angle}. Keep the part's meaning first.", '']
    if shown:
        out.append(f'Versions this part already has ({len(shown)} shown; never repeat, echo or lightly reword '
                   'them, and open your lines differently):')
        out += shown
    else:
        out.append('This part has no versions yet.')
    out += ['', f'Write {n} new versions of {key}. One per line: the words, then " ~ " and its tags when it has '
            'any. Nothing else: no numbers, no headings, no blank lines, no notes.']
    return '\n'.join(out)


def checker_system():
    rules = open(os.path.join(BRIEFS, 'rules.md')).read()
    chk = open(os.path.join(BRIEFS, 'checker.md')).read()
    return rules.strip() + '\n\n' + chk.strip()


def checker_prompt(key, lines):
    fam = open(os.path.join(BRIEFS, PARTS[key].family + '.md')).read()
    out = [fam.strip(), '', part_card(key), '', 'The lines to check:']
    out += [f'{i + 1}. {x}' for i, x in enumerate(lines)]
    out += ['', 'Answer with one line per line that breaks a rule: its number, a colon and the rule in a few '
            'words. Nothing else. If every line is fine, answer: none']
    return '\n'.join(out)


# ---------------------------------------------------------------------------
# Plans

def shortfall(st, targets):
    pool = st.pool()
    return {k: t - len(pool.get(k, [])) for k, t in targets.items()}


def plan_round(st, targets, per_part, keep_rate, rnd):
    """The write requests of one round: [(custom_id, key, n, angle)]."""
    out = []
    short = shortfall(st, targets)
    i = 0
    # The parts the proposal fills first go first, so a batch trimmed at the
    # cap keeps them.
    for key in sorted(short, key=lambda k: (not k.startswith(FIRST), -short[k])):
        miss = short[key]
        if miss <= max(2, DONE_SHARE * targets[key]):
            continue
        want = math.ceil(miss / max(keep_rate, 0.3))
        reqs = min(per_part, math.ceil(want / PER_REQ))
        reach = PARTS[key].levels or {'hs', 'college', 'nfl'}
        angles = [a for a in ANGLES[PARTS[key].who]
                  if not any(f'(tag {lv}' in a for lv in {'hs', 'college', 'nfl'} - set(reach))]
        for r in range(reqs):
            n = min(PER_REQ, math.ceil(want / reqs))
            angle = angles[(r + rnd * reqs) % len(angles)]
            out.append((f'w{rnd}-{i:05d}', key, n, angle))
            i += 1
    return out


def shown_for(key, have, rnd, r):
    rng = random.Random(zlib.crc32(f'{key}|{rnd}|{r}'.encode()))
    if len(have) <= SHOW:
        xs = list(have)
        rng.shuffle(xs)
        return xs
    return rng.sample(have, SHOW)


def build_requests(st, plan, rnd):
    old = B.bank(skip=set(FILES.values()))
    pool = st.pool()
    reqs, meta = [], {}
    for cid, key, n, angle in plan:
        have = [t + (' ~ ' + ' '.join(g) if g else '') for t, g, _ in old.get(key, [])] + pool.get(key, [])
        params = {
            'model': MODEL,
            'max_tokens': MAX_TOKENS,
            # The lowest thinking setting on Sonnet 5.5 ("disabled" is a 400).
            'thinking': {'type': 'between_tools'},
            'system': system_prompt(PARTS[key].family),
            'messages': [{'role': 'user', 'content': user_prompt(key, n, angle, shown_for(key, have, rnd, cid))}],
        }
        reqs.append({'custom_id': cid, 'params': params})
        meta[cid] = {'key': key, 'n': n, 'angle': angle}
    return reqs, meta


def chars(req):
    p = req['params']
    return len(p['system']) + sum(len(m['content']) for m in p['messages'])


def estimate(st, reqs):
    """(likely cost, worst cost) of a batch: the worst counts every request
    writing to its token limit."""
    cpt = st.s['chars_in'] / st.s['tokens_in'] * 0.95 if st.s['tokens_in'] > 50000 else CHARS_PER_TOKEN
    tin = sum(chars(r) for r in reqs) / cpt
    likely_out = sum(min(r['params']['max_tokens'], 26 * meta_n(r) + 120) for r in reqs)
    worst_out = sum(r['params']['max_tokens'] for r in reqs)
    return cost(tin, likely_out), cost(tin, worst_out)


def meta_n(r):
    m = re.search(r'Write (\d+) new versions', r['params']['messages'][0]['content'])
    return int(m.group(1)) if m else 40


def fit_cap(st, reqs, cap):
    """Trim a batch until its worst case stays under the cap."""
    while reqs:
        _, worst = estimate(st, reqs)
        if st.s['spent'] + worst <= cap:
            return reqs
        reqs = reqs[:int(len(reqs) * 0.9)]
    return reqs


# ---------------------------------------------------------------------------
# Batches

def submit(c, st, reqs, meta, kind, rnd):
    likely, worst = estimate(st, reqs)
    b = c.messages.batches.create(requests=reqs)
    st.s['batches'][b.id] = {'kind': kind, 'round': rnd, 'meta': meta, 'ended': False,
                             'chars': sum(chars(r) for r in reqs), 'likely': likely, 'worst': worst}
    st.save()
    st.log(f'{kind} batch {b.id}: {len(reqs)} requests, likely ${likely:.2f}, at most ${worst:.2f} '
           f'(spent so far ${st.s["spent"]:.2f})')
    return b.id


def wait(c, st, bid):
    while True:
        b = c.messages.batches.retrieve(bid)
        if b.processing_status == 'ended':
            return b
        rc = b.request_counts
        st.log(f'batch {bid}: {rc.processing} processing, {rc.succeeded} done, {rc.errored} errored')
        time.sleep(60)


def collect(c, st, bid):
    """{custom_id: text} for a finished batch, its usage added to the spend."""
    rec = st.s['batches'][bid]
    texts, tin, tout, bad = {}, 0, 0, collections.Counter()
    with open(os.path.join(st.dir, 'replies', bid + '.jsonl'), 'w') as fo:
        for r in c.messages.batches.results(bid):
            res = r.result
            if res.type != 'succeeded':
                bad[res.type] += 1
                continue
            m = res.message
            u = m.usage
            tin += (u.input_tokens or 0) + (getattr(u, 'cache_creation_input_tokens', 0) or 0) \
                + (getattr(u, 'cache_read_input_tokens', 0) or 0)
            tout += u.output_tokens or 0
            if m.stop_reason == 'refusal':
                bad['refusal'] += 1
                continue
            text = '\n'.join(b.text for b in m.content if b.type == 'text')
            texts[r.custom_id] = text
            fo.write(json.dumps({'id': r.custom_id, 'stop': m.stop_reason, 'in': u.input_tokens,
                                 'out': u.output_tokens, 'text': text}) + '\n')
    spent = cost(tin, tout)
    st.s['spent'] += spent
    st.s['tokens_in'] += tin
    st.s['tokens_out'] += tout
    st.s['chars_in'] += rec['chars']
    rec.update({'ended': True, 'cost': spent, 'tokens_in': tin, 'tokens_out': tout, 'failed': dict(bad)})
    st.save()
    st.log(f'batch {bid} ended: {len(texts)} replies, {dict(bad) or "no failures"}, ${spent:.2f} '
           f'(spent ${st.s["spent"]:.2f})')
    return texts


def code_check(st, rec, texts, targets):
    """Every reply line through check.py: {key: [kept lines]}, drops logged."""
    ck = K.Checker(keys=sorted({m['key'] for m in rec['meta'].values()}), pool=st.pool(), target=targets)
    kept = collections.defaultdict(list)
    why = collections.Counter()
    written = 0
    with open(os.path.join(st.dir, 'dropped', f'round{rec["round"]}-code.tsv'), 'a') as fo:
        for cid, text in texts.items():
            key = rec['meta'][cid]['key']
            for raw in text.splitlines():
                if not K.clean(raw):
                    continue
                written += 1
                line, w = ck.check(key, raw)
                if line is None:
                    why[re.sub(r'"[^"]*"|\{[^}]*\}|\([^)]*\)|\d+', '_', w[0])] += 1
                    fo.write(f'{key}\t{"; ".join(w)}\t{raw.strip()}\n')
                else:
                    kept[key].append(line)
    n = sum(len(v) for v in kept.values())
    st.s['written'] += written
    st.s['kept_code'] += n
    st.save()
    st.log(f'code check: {n} of {written} lines kept; top drops {why.most_common(6)}')
    return kept


def model_check(c, st, kept, rnd, cap):
    """One checker pass (Sonnet 5.5 reads every kept line once): flagged
    lines are dropped, the rest join the pool."""
    reqs, meta = [], {}
    i = 0
    for key, lines in sorted(kept.items()):
        for a in range(0, len(lines), CHECK_PER_REQ):
            chunk = lines[a:a + CHECK_PER_REQ]
            cid = f'c{rnd}-{i:05d}'
            i += 1
            reqs.append({'custom_id': cid, 'params': {
                'model': MODEL, 'max_tokens': 3000, 'thinking': {'type': 'between_tools'},
                'system': checker_system(),
                'messages': [{'role': 'user', 'content': checker_prompt(key, chunk)}]}})
            meta[cid] = {'key': key, 'lines': chunk, 'n': len(chunk)}
    if not reqs:
        return {}
    fitted = fit_cap(st, reqs, cap)
    if len(fitted) < len(reqs):
        st.log(f'the cap leaves room to check {len(fitted)} of {len(reqs)} chunks; the rest wait unchecked')
        reqs = fitted
        meta = {r['custom_id']: meta[r['custom_id']] for r in reqs}
    if not reqs:
        return {}
    bid = submit(c, st, reqs, meta, 'check', rnd)
    wait(c, st, bid)
    texts = collect(c, st, bid)
    out = collections.defaultdict(list)
    flagged = 0
    with open(os.path.join(st.dir, 'dropped', f'round{rnd}-checker.tsv'), 'a') as fo:
        for cid, m in meta.items():
            if cid not in texts:
                continue            # no answer: these lines wait for the next pass
            bad = {}
            for ln in texts[cid].splitlines():
                g = re.match(r'\s*(\d+)\s*[:.)-]\s*(.*)', ln)
                if g and 1 <= int(g.group(1)) <= m['n']:
                    bad[int(g.group(1)) - 1] = g.group(2).strip()
            for j, line in enumerate(m['lines']):
                if j in bad:
                    flagged += 1
                    fo.write(f'{m["key"]}\t{bad[j]}\t{line}\n')
                else:
                    out[m['key']].append(line)
    n = sum(len(v) for v in out.values())
    st.s['kept'] += n
    st.save()
    st.log(f'checker pass: {flagged} flagged, {n} kept')
    return out


# ---------------------------------------------------------------------------
# Commands

def targets_for(pilot):
    if pilot:
        return dict(PILOT)
    al = allocation()
    return {k: v for k, v in al.items() if v > 0 and k not in NEVER}


def keep_rate(st):
    return st.s['kept'] / st.s['written'] if st.s['written'] > 500 else 0.77


def rounds(c, st, targets, args):
    for _ in range(args.rounds):
        rnd = st.s['round'] + 1
        plan = plan_round(st, targets, args.per_part, keep_rate(st), rnd)
        if not plan:
            st.log('every part is full')
            return True
        reqs, meta = build_requests(st, plan, rnd)
        fitted = fit_cap(st, reqs, args.cap - 2.0)      # room left for the checker pass
        if not fitted:
            st.log(f'stopped at the cap: ${st.s["spent"]:.2f} spent of ${args.cap:.2f}')
            return False
        if len(fitted) < len(reqs):
            st.log(f'the cap leaves room for {len(fitted)} of {len(reqs)} requests this round')
            meta = {r['custom_id']: meta[r['custom_id']] for r in fitted}
        st.s['round'] = rnd
        st.save()
        bid = submit(c, st, fitted, meta, 'write', rnd)
        wait(c, st, bid)
        texts = collect(c, st, bid)
        kept = code_check(st, st.s['batches'][bid], texts, targets)
        good = model_check(c, st, kept, rnd, args.cap)
        for key, lines in good.items():
            room = targets.get(key, 0) - len(st.pool().get(key, []))
            st.add_pool(key, lines[:max(0, room)])
        report(st, targets)
        if len(fitted) < len(reqs):
            st.log(f'stopped at the cap: ${st.s["spent"]:.2f} spent of ${args.cap:.2f}')
            return False
    return False


def resume(c, st, targets, args):
    """A batch that was in flight when the run stopped: finish it first."""
    for bid, rec in list(st.s['batches'].items()):
        if rec['ended']:
            continue
        st.log(f'resuming {rec["kind"]} batch {bid}')
        wait(c, st, bid)
        texts = collect(c, st, bid)
        if rec['kind'] == 'write':
            kept = code_check(st, rec, texts, targets)
            good = model_check(c, st, kept, rec['round'], args.cap)
            for key, lines in good.items():
                room = targets.get(key, 0) - len(st.pool().get(key, []))
                st.add_pool(key, lines[:max(0, room)])
        else:
            st.log('a checker batch was in flight: its lines are checked again next round')


def report(st, targets):
    pool = st.pool()
    fam = collections.defaultdict(lambda: [0, 0])
    for k, t in targets.items():
        fam[PARTS[k].family][0] += len(pool.get(k, []))
        fam[PARTS[k].family][1] += t
    out = ['# Line bank run: where it stands', '',
           f'Spent ${st.s["spent"]:.2f} ({st.s["tokens_in"]:,} tokens in, {st.s["tokens_out"]:,} out) over '
           f'{st.s["round"]} rounds. Written {st.s["written"]:,}, kept by the code check {st.s["kept_code"]:,}, '
           f'kept after the checker pass {st.s["kept"]:,} ({100 * keep_rate(st):.0f}%).', '',
           '| Family | Kept | Target |', '|---|---|---|']
    for f, (k, t) in sorted(fam.items()):
        out.append(f'| {f} | {k:,} | {t:,} |')
    out += ['', '## Sample lines', '']
    rng = random.Random(7)
    for k in sorted(targets):
        xs = pool.get(k, [])
        if xs:
            out.append(f'**{k}** ({len(xs)} of {targets[k]})')
            out += [f'- {x}' for x in rng.sample(xs, min(5, len(xs)))]
            out.append('')
    path = os.path.join(st.dir, 'report.md')
    open(path, 'w').write('\n'.join(out) + '\n')
    st.log(f'report written: {path}')


def merge(st):
    pool = st.pool()
    by = collections.defaultdict(dict)
    for k, xs in pool.items():
        if k in PARTS:
            by[PARTS[k].family][k] = xs
    for fam, fname in FILES.items():
        path = os.path.join(B.LINES, fname)
        head = open(path).read().split('\n@key', 1)[0].rstrip('\n') if os.path.exists(path) else ''
        out = [head] if head else []
        for k in sorted(by.get(fam, {})):
            out += ['', f'@key {k}'] + by[fam][k]
        open(path, 'w').write('\n'.join(out).lstrip('\n') + '\n')
        print(f'{fname}: {sum(len(v) for v in by.get(fam, {}).values())} versions')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('cmd', choices=['key', 'dry', 'pilot', 'run', 'status', 'merge', 'report'])
    ap.add_argument('--state', default=DEFAULT_STATE)
    ap.add_argument('--cap', type=float, default=40.0, help='stop before this many dollars, all batches counted')
    ap.add_argument('--rounds', type=int, default=6)
    ap.add_argument('--per-part', type=int, default=12, help='requests per part in one round')
    ap.add_argument('--pilot', action='store_true')
    args = ap.parse_args()
    st = State(args.state)
    if args.cmd == 'status':
        print(json.dumps({k: v for k, v in st.s.items() if k != 'batches'}, indent=1))
        for bid, r in st.s['batches'].items():
            print(bid, r['kind'], 'round', r['round'], 'ended' if r['ended'] else 'IN FLIGHT', f'${r.get("cost", 0):.2f}')
        return
    if args.cmd == 'merge':
        return merge(st)
    if args.cmd == 'report':
        return report(st, targets_for(args.pilot))
    if args.cmd == 'dry':
        targets = targets_for(args.pilot)
        plan = plan_round(st, targets, args.per_part, keep_rate(st), st.s['round'] + 1)
        reqs, _ = build_requests(st, plan, st.s['round'] + 1)
        likely, worst = estimate(st, reqs)
        print(f'round {st.s["round"] + 1}: {len(reqs)} requests for {len({p[1] for p in plan})} parts, '
              f'{sum(p[2] for p in plan):,} versions asked, likely ${likely:.2f}, at most ${worst:.2f}')
        print(f'targets: {sum(targets.values()):,} versions over {len(targets)} parts')
        if reqs:
            r = reqs[0]['params']
            print('\n--- system (first request) ---\n' + r['system'][:3000])
            print('\n--- user (first request, cut) ---\n' + r['messages'][0]['content'][:3000])
        return
    c = client()
    probe(c)
    if args.cmd == 'key':
        print(f'{KEY_VAR} is set and the API accepts it (a free models call). Nothing was spent.')
        return
    targets = targets_for(args.cmd == 'pilot')
    if args.cmd == 'pilot':
        args.cap = min(args.cap, 3.0)
        args.rounds = 1
    st.log(f'{args.cmd}: {sum(targets.values()):,} versions over {len(targets)} parts, cap ${args.cap:.2f}')
    resume(c, st, targets, args)
    done = rounds(c, st, targets, args)
    report(st, targets)
    st.log(f'{args.cmd} {"finished" if done else "stopped"}: ${st.s["spent"]:.2f} spent')


if __name__ == '__main__':
    main()
