#!/usr/bin/env python3
"""Build the player pages page from world trial cards.

Usage: build_pages.py TEMPLATE.html OUT.html TITLE RUN_DIR [RUN_DIR ...]

TEMPLATE is an earlier player pages file (for example
docs/trials/artifacts/world-trial5-player-pages.html): its `const CARDS = [...]`
line is replaced by the cards of the given runs. Each RUN_DIR holds the
cards.json that world_trial writes with AFM_CARDS; the world's seed is read
from the folder name (seed-2026) and put in front of each card's tag.
Each season row gets a "What happened" cell in plain words (injuries with
the games they cost, slumps, breakouts, the first-year dip, the spring gain);
coaching quality shows only in the debug view.
Trial and debug tooling only: the cards carry hidden potential.
"""

import json
import os
import re
import sys

# Season events in plain words (the names are DevelopmentEvent::name).
EVENT_WORDS = {
    "growthSpurt": "growth spurt",
    "bodyStalled": "his body stopped developing",
    "breakout": "breakout year",
    "stagnation": "a flat year, little growth",
    "lostSeason": "a lost season",
    "suspended": "suspended or academically ineligible",
    "dismissed": "dismissed from the team",
    "slump": "slump: his production fell hard",
}
INJURY_WORDS = {"minorInjury": "minor", "moderateInjury": "moderate", "majorInjury": "major"}

# The template's season table, and the column added to it.
HEAD_OLD = '<th class="n dbgcol">Potential</th></tr>'
HEAD_NEW = '<th class="n dbgcol">Potential</th><th>What happened</th></tr>'
ROW_OLD = '<td class="n dbgcol">${s.pot}</td></tr>'
ROW_NEW = (
    '<td class="n dbgcol">${s.pot}</td><td>${s.what||"·"}'
    '${s.coach!=null?`<span class="hidden-val">Coaching quality ${s.coach} of 20</span>`:""}</td></tr>'
)
NOTE_OLD = '("89 among seniors").</p>'
NOTE_NEW = (
    '("89 among seniors"). "What happened" gives the reasons behind a season: injuries with the games'
    " they cost, slumps, breakouts, the first-year dip at a new level (0 is ready at once, 1 a very"
    " hard first year), and a spring breakout, which comes after the season except an early enrollee's"
    " first college spring, before his first college season"
    ' (its gain is already in "Rating out").</p>'
)


def season_words(s):
    """A season row's events, games missed, dip and spring gain in plain words."""
    ev = s.get("ev") or []
    words = [EVENT_WORDS[e] for e in ev if e in EVENT_WORDS]
    hurt = [INJURY_WORDS[e] for e in ev if e in INJURY_WORDS]
    missed = s.get("missed") or 0
    if hurt or missed:
        kind = f"{hurt[-1]} injury" if hurt else "hurt"
        games = f"missed {missed} game{'' if missed == 1 else 's'}" if missed else "missed no games"
        words.append(f"{kind}, {games}")
    if "firstYear" in ev:
        dip = s.get("dip")
        size = f" (dip {dip:.2f} of 1)" if dip is not None else ""
        if s.get("perf") is None:
            words.append(f"first year at this level, adjusting{size}")
        else:
            words.append(f"first year at this level: he played below his rating while he adjusted{size}")
    # An early enrollee's first college spring step opens his first college
    # row ("spring_open", before the season); any other part of the row's
    # spring gain came after the season. Older cards have no "spring_open".
    spring = s.get("spring") or 0
    opening = s.get("spring_open") or 0
    after = spring - opening
    if opening:
        words.append(f"spring breakout in spring practice before the season, +{opening:.1f}")
    if after > 0.05 or ("springBreakout" in ev and not opening):
        gain = f", +{after:.1f}" if after > 0.05 else ""
        words.append(f"spring breakout after the season{gain}")
    text = "; ".join(words)
    return text[:1].upper() + text[1:]


def show_events(line):
    """Add the "What happened" column to the template's season table, once."""
    if HEAD_NEW not in line:
        line = line.replace(HEAD_OLD, HEAD_NEW)
    if ROW_NEW not in line:
        line = line.replace(ROW_OLD, ROW_NEW)
    if NOTE_NEW not in line:
        line = line.replace(NOTE_OLD, NOTE_NEW)
    return line


def main():
    if len(sys.argv) < 5:
        sys.exit(__doc__)
    template, out, title, runs = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
    cards = []
    for run in runs:
        seed = re.sub(r"\D", "", os.path.basename(os.path.normpath(run))) or run
        for c in json.load(open(os.path.join(run, "cards.json"))):
            tag = c.get("tag") or ""
            if not tag.startswith("World "):
                c["tag"] = f"World {seed} · {tag}" if tag else f"World {seed}"
            for s in c.get("seasons", []):
                s["what"] = season_words(s)
            cards.append(c)
    lines = open(template).read().split("\n")
    done = False
    for i, line in enumerate(lines):
        if line.startswith("const CARDS = "):
            lines[i] = "const CARDS = " + json.dumps(cards, separators=(",", ":")) + ";"
            done = True
        elif line.startswith("<title>"):
            lines[i] = f"<title>{title}</title>"
        else:
            lines[i] = show_events(line)
    if not done:
        sys.exit("template has no `const CARDS = ` line")
    open(out, "w").write("\n".join(lines))
    print(f"{len(cards)} cards -> {out}")


if __name__ == "__main__":
    main()
