#!/usr/bin/env python3
"""The series and counters library (scheme sequencing research, job A).

One request a topic (series_topics.json): Sonnet 5.5 reads public coaching and
analytics writing with web search and web fetch, and writes one JSON entry:
the base play, the defender it stresses, his tell, the answers, what decides
each one in the battles, how a defense answers back, when coaches call it,
real numbers and its sources. Research only, never decisions.

    series_library.py run [--only O01,O02] [--cap 45] [--workers 6]
    series_library.py status
    series_library.py merge [--cap 6]
        one Opus 5.5 pass over every entry: the engine map (what each series needs the
        engine to model, what it already has, what's missing) and the library's index

State and raw replies go to --out (default the project folder). It stops
starting new requests once the spend reaches the cap.
"""
import argparse
import concurrent.futures as cf
import json
import os
import re
import sys
import threading
import time

import anthropic

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = '/mnt/project-files/afm/scheme-sequencing/library'
MODEL = 'claude-sonnet-5-5'
PRICE = {'in': 2.00, 'out': 10.00, 'cache_write': 2.50, 'cache_read': 0.10, 'search': 0.01}
KEY_VAR = 'AFM_LINEBANK_KEY'
BASE_URL = 'https://api.anthropic.com'

ENGINE = """The game's play library (use these names where they fit):
Passes: slant-flat, stick, hitches, mesh, smash, flood, curl-flat, dagger, four verticals, Y-cross, play-action shot,
play-action boot, play-action dig, RB screen, bubble screen, speed outs, fades, drive, shallow cross, levels, spacing,
snag, mills, Y-option, Texas, slot wheel, tunnel screen, RPO glance, RPO bubble, RPO slant.
Runs: inside zone, outside zone, power, counter, draw, QB sneak, zone read, toss, buck sweep, duo, trap, iso, jet sweep,
QB power, speed option, triple option.
Coverages: Cover 0, Cover 1, Cover 2, Tampa 2, Cover 3, Cover 4, Cover 6, 2-man. Fronts, blitzes, stunts, spies.
Ratings that exist (examples): Play Recognition, Discipline, Football IQ, Anticipation, Pursuit, Block Shedding,
Tackling, Man Coverage, Zone Coverage, Speed, Acceleration, Agility, Route Running, Release, Ball Handling (the QB's fake),
Eye Manipulation, Decisions, Throw on the Move, Run Blocking, Pass Blocking, Vision, Elusiveness, and habits such as
Downhill (how hard a defender flies at the run), Bounce, Blitzing."""

SYSTEM = """You research real American football for a realistic football management game. You read public coaching
writing (clinic notes, coaches' articles and books excerpts, X's and O's sites, coaching podcasts' write-ups) and public
analytics (nflverse, FTN, Next Gen Stats articles, Football Outsiders, academic papers). You never invent: every claim
comes from a page you actually read in this conversation, or you mark it as general coaching knowledge with
confidence "low". Prefer specific mechanics over slogans: which defender, which key, which alignment, which step.
Numbers only with their source. Plain English, short sentences.

""" + ENGINE + """

Answer with one JSON object in a ```json fenced block at the very end, and nothing after it. Shape:
{
 "id": "<topic id>",
 "title": "<short title>",
 "side": "offense|defense|situation",
 "summary": "<3-5 plain sentences: how it works in real football>",
 "base": {"play": "...", "looks": "personnel, formation, motion it usually comes from", "attacks": "what it attacks"},
 "stressed": [{"who": "position and alignment", "key": "what he reads", "job": "his assignment"}],
 "tells": ["what the stressed defender or the defense does when it is cheating or over-playing it, and how a coach sees it on film or in-game"],
 "answers": [{"play": "...", "when": "the tell that calls for it", "why": "the mechanism", "decides": "which battle or man decides it",
              "ratings": ["ratings or habits that decide it, both sides"]}],
 "defense_back": [{"answer": "how a good defense answers the answer", "cost": "what that gives up"}],
 "timing": "when and how often real coordinators call the answer in a game, how many reps a set-up takes, how long it lasts",
 "levels": "how it differs in the NFL, college and high school",
 "evidence": [{"claim": "a measured fact", "number": "...", "source": "url"}],
 "doubts": ["claims coaches make that analytics doubts or did not find, with the source"],
 "engine_notes": "what a simulation must model for this to work through the players and their positions, not a bonus",
 "sources": [{"url": "...", "title": "...", "supports": "what it supports"}],
 "confidence": "high|medium|low"
}"""

TOOLS = [
    {'type': 'web_search_20260209', 'name': 'web_search', 'max_uses': 8},

]

LOCK = threading.Lock()


def client():
    key = os.environ.get(KEY_VAR, '')
    if not key:
        sys.exit(f'{KEY_VAR} is not set: nothing was sent.')
    return anthropic.Anthropic(api_key=key, base_url=BASE_URL, max_retries=4, timeout=900)


def cost(u):
    stu = getattr(u, 'server_tool_use', None)
    searches = (getattr(stu, 'web_search_requests', 0) or 0) if stu else 0
    return ((u.input_tokens or 0) * PRICE['in'] + (u.output_tokens or 0) * PRICE['out']
            + (getattr(u, 'cache_creation_input_tokens', 0) or 0) * PRICE['cache_write']
            + (getattr(u, 'cache_read_input_tokens', 0) or 0) * PRICE['cache_read']) / 1e6 + searches * PRICE['search'], searches


def load_state(out):
    p = os.path.join(out, 'state.json')
    if os.path.exists(p):
        return json.load(open(p))
    return {'spent': 0.0, 'done': {}}


def save_state(out, st):
    p = os.path.join(out, 'state.json')
    tmp = p + '.tmp'
    json.dump(st, open(tmp, 'w'), indent=1)
    os.replace(tmp, p)


def ask(c, tid, side, topic):
    msgs = [{'role': 'user', 'content': f'Topic {tid} ({side}): {topic}\n\nResearch it and write the entry.'}]
    total, searches, texts = 0.0, 0, []
    for _ in range(6):  # pause_turn continuations
        r = c.messages.create(model=MODEL, max_tokens=16000, system=SYSTEM, tools=TOOLS, messages=msgs,
                              output_config={'effort': 'medium'})
        dc, s = cost(r.usage)
        total += dc
        searches += s
        texts += [b.text for b in r.content if b.type == 'text']
        if r.stop_reason == 'pause_turn':
            msgs = msgs + [{'role': 'assistant', 'content': r.content}]
            continue
        return r.stop_reason, '\n'.join(texts), total, searches
    return 'too_many_pauses', '\n'.join(texts), total, searches


def parse(text):
    m = re.findall(r'```json\s*(\{.*?\})\s*```', text, re.S)
    if not m:
        return None
    try:
        return json.loads(m[-1])
    except json.JSONDecodeError:
        return None


def run(args):
    out = args.out
    os.makedirs(os.path.join(out, 'raw'), exist_ok=True)
    topics = json.load(open(os.path.join(HERE, 'series_topics.json')))
    if args.only:
        keep = set(args.only.split(','))
        topics = [t for t in topics if t[0] in keep]
    st = load_state(out)
    todo = [t for t in topics if t[0] not in st['done']]
    c = client()
    print(f'{len(todo)} topics to do, spent so far ${st["spent"]:.2f}, cap ${args.cap:.2f}', flush=True)

    def one(t):
        tid, side, topic = t
        with LOCK:
            if st['spent'] >= args.cap:
                return tid, 'skipped_cap'
        t0 = time.time()
        try:
            stop, text, dollars, searches = ask(c, tid, side, topic)
        except anthropic.APIError as e:
            return tid, f'error {e}'
        entry = parse(text)
        json.dump({'id': tid, 'stop': stop, 'text': text, 'cost': dollars, 'searches': searches},
                  open(os.path.join(out, 'raw', tid + '.json'), 'w'), indent=1)
        with LOCK:
            st['spent'] += dollars
            if entry:
                st['done'][tid] = {'cost': round(dollars, 4), 'searches': searches, 'secs': round(time.time() - t0)}
                json.dump(entry, open(os.path.join(out, tid + '.json'), 'w'), indent=1)
            save_state(out, st)
        return tid, f'{"ok" if entry else "no json"} ${dollars:.3f} {searches} searches {time.time() - t0:.0f}s (total ${st["spent"]:.2f})'

    with cf.ThreadPoolExecutor(args.workers) as ex:
        for tid, msg in ex.map(one, todo):
            print(tid, msg, flush=True)
    print(f'done: {len(st["done"])} entries, spent ${st["spent"]:.2f}', flush=True)


ENGINE_HAS = """What the engine already models (docs/reference/how-it-works/play-calling.md and match-engine.md):
- In-game memory: the defense keeps each look's runs and passes, the concepts, quick throws, deep and run EPA; defenders
  read the play 0.16 s x his key sooner when the look's tell is right, later when it lies (Memory::belief).
- Self-scout by look (a look run 75%+ flips to a pass, 15% or less to a run, chance 0.45 x creativity/20).
- Adaptation from today's EPA (run-pass lean, each run concept's results, deep results); the defense's shells and fronts lean
  against what hurt it (deep hurt, run hurt) and toward the look's run belief.
- Film study before the game: tape snaps, tells (a look on 8+ snaps over 2+ games, 75% one kind), habits targeted, a matchup plan,
  call sheets drafted from the opponent's film.
- A what-beats-what call table measured in the engine (concept by coverage and front), plus the roster's lean (what his own men win).
- Play action on any dropback; the fake's hold reads each defender's Play Recognition, Discipline and Downhill habit and the QB's
  Ball Handling; misdirection and jet motion raise run-fit misfits; RPOs; kill calls; audibles against the box.
- Tracked players: real run fits, blocks won and lost, sheds, pursuit angles, coverage jobs, busts, the pass rush, the QB's reads.
What it lacks: memory of run direction; defenders who cheat toward a side; base plays linked to answers in series; the defense's
answers in series; self-scout by direction, down or concept; halftime adjustments; tendency breakers chosen on purpose."""

MERGE_SYSTEM = """You are the lead designer of a realistic American football management game's match engine. You get a research
library of real play series, counters and situations. Write the engine map. For each entry: which engine plays and coverages it
uses, what the engine already has for it, what is missing, the battles and ratings that must decide it (never a bonus), and how a
test would show it works (an A/B on the same seeds). Then group the missing pieces into a short ordered list of engine topics for
the design rounds, each with the entries it serves. Plain English, short sentences. Mark anything the research did not support
as a guess.

""" + ENGINE + "\n\n" + ENGINE_HAS + """

Answer with only one ```json block, keys in this order: {"topics": [{"name": "...", "why": "...", "entries": [...],
"real_targets": "..."}], "warnings": ["claims in the library that look wrong or unsupported"], "entries": [{"id": "...",
"uses": [...], "has": "...", "missing": "...", "decides": "...", "test": "..."}]}. Keep each entry's fields to one or two
short sentences."""


def merge(args):
    out = args.out
    ids = sorted(f[:-5] for f in os.listdir(out) if re.match(r'^[ODS]\d\d\.json$', f))
    lib = [json.load(open(os.path.join(out, i + '.json'))) for i in ids]
    for e in lib:
        e.pop('sources', None)
    text = json.dumps(lib, separators=(',', ':'))
    c = client()
    est = len(text) / 3.5 * 4.00 / 1e6 + 40000 * 20.0 / 1e6
    print(f'{len(lib)} entries, about {len(text) / 3.5:.0f} tokens in, estimate ${est:.2f}')
    if est > args.cap:
        sys.exit('over the cap: nothing sent')
    with c.messages.stream(model='claude-opus-5-5', max_tokens=128000, system=MERGE_SYSTEM,
                           output_config={'effort': 'high'},
                           messages=[{'role': 'user', 'content': 'The library:\n' + text}]) as s:
        r = s.get_final_message()
    u = r.usage
    dollars = (u.input_tokens * 4.00 + u.output_tokens * 20.00) / 1e6
    body = '\n'.join(b.text for b in r.content if b.type == 'text')
    open(os.path.join(out, 'engine_map.md'), 'w').write(body)
    m = parse(body)
    if m:
        json.dump(m, open(os.path.join(out, 'engine_map.json'), 'w'), indent=1)
    st = load_state(out)
    st['spent'] += dollars
    st['merge'] = {'cost': round(dollars, 3), 'entries': len(lib)}
    save_state(out, st)
    print(f'merge ${dollars:.2f}, json {"ok" if m else "missing"}')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('cmd', choices=['run', 'status', 'merge'])
    ap.add_argument('--only')
    ap.add_argument('--cap', type=float, default=45.0)
    ap.add_argument('--workers', type=int, default=6)
    ap.add_argument('--out', default=OUT)
    args = ap.parse_args()
    if args.cmd == 'merge':
        merge(args)
        return
    if args.cmd == 'status':
        st = load_state(args.out)
        print(f'{len(st["done"])} entries, spent ${st["spent"]:.2f}')
        return
    run(args)


if __name__ == '__main__':
    main()
