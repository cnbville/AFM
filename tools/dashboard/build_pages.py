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


# The pro career box (section 5): status, draft, teams, earnings, agent and
# deal, then every NFL season with its box score, pay and honors.
PRO_OLD = 'document.getElementById("main").innerHTML = `'
PRO_JS = (
    'const pr=c.pro;const money=v=>v==null?"·":(v>=1e6?"$"+(v/1e6).toFixed(2)+"M":"$"+Math.round(v/1e3)+"K");'
    'const proT=pr?`<div class="box scroll" style="margin-top:12px"><h2>Pro career</h2><div class="kv">'
    '<span>Status</span><span>${pr.status}${pr.team?" · "+pr.team:""}</span>'
    '<span>Drafted</span><span>${pr.draft?`${pr.draft[0]}, round ${pr.draft[1]}, pick ${pr.draft[2]} (${pr.draft[3]})`:"undrafted"}</span>'
    '<span>Teams</span><span>${pr.teams.join(", ")||"·"}</span><span>Accrued seasons</span><span>${pr.accrued}</span>'
    '<span>Career earnings</span><span>${money(pr.earnings)}</span><span>Agent</span><span>${pr.agent??"none"}</span>'
    '${pr.tags?`<span>Tagged</span><span>${pr.tags} time${pr.tags>1?"s":""}</span>`:""}</div>'
    '${pr.deal?`<p style="margin:8px 0">Deal: ${pr.deal.kind}, ${pr.deal.years} year${pr.deal.years>1?"s":""} through ${pr.deal.last},'
    ' ${money(pr.deal.apy)} a year, ${money(pr.deal.guaranteed)} guaranteed, signed ${pr.deal.signed} with the ${pr.deal.team}'
    '${pr.deal.voids?`, ${pr.deal.voids} void year${pr.deal.voids>1?"s":""}`:""}${pr.deal.no_trade?", no-trade clause":""}.</p>`:""}'
    '<table><tr><th class="n">Year</th><th>Team</th><th class="n">G</th><th class="n">GS</th><th class="n">Snaps O/D/ST</th>'
    '<th class="n">PS wks</th><th class="n">IR wks</th><th class="n">Pay</th><th class="n">Cmp/Att</th><th class="n">Pass yds</th>'
    '<th class="n">TD</th><th class="n">INT</th><th class="n">Car</th><th class="n">Rush yds</th><th class="n">TD</th><th class="n">Rec</th>'
    '<th class="n">Rec yds</th><th class="n">TD</th><th class="n">Tkl</th><th class="n">Sacks</th><th class="n">INT</th><th class="n">PBU</th>'
    '<th class="n">Grade</th><th>Honors</th></tr>${pr.seasons.map(s=>`<tr><td class="n">${s.year}</td><td>${s.team??"·"}</td>'
    '<td class="n">${s.g}</td><td class="n">${s.gs}</td><td class="n">${s.snaps.join("/")}</td><td class="n">${s.ps_weeks||""}</td>'
    '<td class="n">${s.ir_weeks||""}</td><td class="n">${money(s.pay)}</td><td class="n">${s.pass[0]}/${s.pass[1]}</td>'
    '<td class="n">${s.pass[2]}</td><td class="n">${s.pass[3]}</td><td class="n">${s.pass[4]}</td><td class="n">${s.rush[0]}</td>'
    '<td class="n">${s.rush[1]}</td><td class="n">${s.rush[2]}</td><td class="n">${s.rec[0]}</td><td class="n">${s.rec[1]}</td>'
    '<td class="n">${s.rec[2]}</td><td class="n">${s.def[0]}</td><td class="n">${s.def[1]}</td><td class="n">${s.def[2]}</td>'
    '<td class="n">${s.def[3]}</td><td class="n">${s.grade??"·"}</td><td>${[s.pro_bowl?"Pro Bowl":"",'
    's.all_pro?`All-Pro (${s.all_pro==1?"first":"second"} team)`:"",...(s.awards||[])].filter(Boolean).join(", ")}</td></tr>`).join("")}'
    '</table><p style="color:var(--mute);font-size:12px">Every NFL season from the match engine: games, starts, snaps on offense,'
    ' defense and special teams, practice squad and injured reserve weeks, and what he was paid that season.</p></div>`:"";\n  '
)
END_OLD = "${statT}`;"
END_NEW = "${statT}${proT}`;"


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
    if PRO_OLD in line and "const proT=" not in line:
        line = line.replace(PRO_OLD, PRO_JS + PRO_OLD)
    if END_NEW not in line:
        line = line.replace(END_OLD, END_NEW)
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
