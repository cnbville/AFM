# World trial 4

Seed 2026, 15 years of history (2011-2025), every college game played on the match engine with the physics fixes (42,455 games). The world built in 17 minutes on 4 cores, about 70 seconds a season. Full output: `world-trial4-output.txt`. The debug dashboard (the mock GUI) is https://claude.ai/artifact/CAxYfhCyfqBCEkxVNb4Rqz and the full player pages are https://claude.ai/artifact/TSJSFtznVRwQptjjf3DJRn (copies in `artifacts/`). Run it with `AFM_DASH=dash.json AFM_CARDS=cards.json cargo run --release --bin world_trial -- 2026 15`.

This trial asks one question: with the engine physics fixed (`docs/design/engine-physics-fixes.md`, how it works in `docs/match-engine.md` under "Physics fixes"), do dynasties and droughts look real? The answer is no, and the trial says why. Each result says why; "Inferred" marks a cause read from the numbers but not traced in the code.

## What is off

### 1. The dynasty got stronger: Ohio State won 10 of 15 titles, the last 10 in a row
Ohio State went 189-5 under William Myers (hired 2012), with eleven unbeaten seasons. Trial 3's Tennessee had 9 titles; the real high over any 15 years is 6.
- Why, the lineup: Ohio State's lineup ranked #1 in six of the title years and #2-#6 in the other four (2016: #6 at 68.8, 15-0). Myers's hidden recruiting is 20.0, the top of the scale, so the roster stayed 69-74 for a decade.
- Why, the staff: Ohio State's defensive coordinator Aaron Cummings has game management 19 and reputation 20.0 (the best in the game); the offensive coordinator John Smith has scheme 17. Play calling now moves games (18 against 4 is worth about +4.4 points with the same players), and Ohio State had the best callers and the best players at once. Tennessee's lineup was as good in 2016 (71.0, 6-6) under a staff with game management 12 and 16.
- Why, the engine: every starter now counts. Adding the measured values (`docs/match-engine.md`, the position table) gives about 4.8 points of margin per rating point of whole-lineup strength, so a 73 lineup against the FBS average of 62 is a 50-point favourite. Ohio State scored 88 points a game in 2025 (1,325 in 15 games) and gave up 10. Real top offenses score about 45. The fixes made the better team win far more reliably, which is what trial 3 asked for one player at a time, but across a whole lineup the edge is about twice what real spreads say (a 30-point spread between #1 and #60 in college).
- Lineup strength and win share correlate at 0.61 across FBS (trial 3: 0.62), so the lineup still decides about the same share; what changed is how big the margin gets and how rarely the weaker side wins.

### 2. Blowouts rose: 31% of FBS games ended 28+ points apart
Trial 3 had 25%; real FBS is about 20%. One-score games fell to 28% (trial 3: 31%; real about 40%).
- Why: the same cause as item 1. With the quarterback, the lines and the callers all worth something, a strength gap turns into points on every drive, and the weaker side's chance of a win in a close game shrank. Twelve unbeaten FBS seasons in 15 years (real: about five).

### 3. Droughts got longer
Twelve FBS programs had no winning season in 15 years, five of them Power 4 (Arizona, Arkansas, NC State, Northwestern, Cincinnati). Trial 3 had three Power 4.
- Why: item 1's flip side. A 58-60 lineup now loses to 65 nearly every time, prestige slides, recruiting follows, and the head coach is fired into the same roster. 45 FBS head coaches were fired within two seasons.

### 4. The match engine in the world
Per team game, every FBS game of the history (trial 3 in brackets, real FBS after):

| | Points | Plays | Comp % | YPA | YPC | Sack % | Int % | 3rd down | Blowouts |
|---|---|---|---|---|---|---|---|---|---|
| FBS | 36.0 (35.2) | 74.5 (76.2) | 54.9 (58.6) | 7.70 (7.72) | 5.72 (5.85) | 4.3 (2.9) | 2.3 (1.6) | 40.4 (40.3) | 31% (25%) |
| Real FBS | 27.0 | 67 | 60.6 | 7.29 | 5.12 | 6.1 | 2.6 | 40.3 | about 20% |

- Sacks and interceptions moved toward real (why: the internal clock makes a quarterback hold, force or eat the ball instead of waiting for a free man; a forced throw gets picked). Completion fell 4 points (why: the quarterback's accuracy and decisions count now, and college passers are 10-15 points below the pro range; the receivers and coverage there are as far behind, so the fix belongs in the catch and the coverage, not the throw). Points stayed high (why: blowouts; the engine's calling tables are the NFL's at every level, the trial 3 finding that is still open).
- FCS, D2 and JUCO sit a step below FBS in every column, as they should: completion 52.5, 51.4 and 50.0; the lower the level the more sacks (4.5, 4.7, 5.2), because the lines are thinner there.

### 5. College stat lines are still unreal at the top
Ohio State's 2025 offense gained 10,824 yards and scored 88 a game. The leaders on the dashboard carry the same inflation.
- Why: item 1; a lineup that wins every snap piles up yards. Not a stats bug: the box scores add up (plays 1,136, yards 10,824, 9.5 a play).

### 6. The first overall pick is a QB 55% of the time
Trial 3: 45%. Real 2000-2025: 73%. EDGE 27% (real 19%). Picks 1-5: EDGE 35%, QB 31% (real QB 28%, EDGE 21%).
- Why: the draft board still prices the edge rusher's athletic grade above the quarterback's, though the engine now values the quarterback most on the field (0.55 points per rating point at FBS against 0.33 for an edge). The board doesn't read the engine's values; on Raichel's open list.

### 7. Coaches of color still fall short
9 of 138 FBS head coaches (7%) in 2025; real about 15%. Coordinators 13%, position coaches 14%. Trial 3's finding, unchanged (not in this build's scope).

## What looks right
- Five-star draft rate 66% (real 63%), four-star 22.9%, three-star 4.8%, two-star 1.0%. FCS players are 7.7% of picks (real about 6%). D2 gives one pick a year.
- FBS high school signees by home state track the real shares (TX 13.4% vs 14.2%, CA 8.7% vs 8.9%, OH 4.8% vs 4.2%).
- Third-down conversion 40.4% (real 40.3%).
- Coaching matters in the right way: Utah's Peter Johnston (game management 19.1) went 107-15 with six conference titles at a 0.77-prestige program; the most underrated coach, Donald Long (hidden 18-19 across the board), is 2-10 at Virginia with a bare roster, which is how the engine should treat a great coach with no players.
- The hidden-gem path works (7 players drafted in rounds 1-2 from two stars or fewer, all starting below FBS). Keith Butcher went Lincoln (MO) to Iowa to pick 18.
- Rises and falls have causes: Bowling Green rose 0.46 to 0.63 with five MAC titles; James Madison fell 0.53 to 0.38.

## Candidates for the next round (not fixed, per the working rules)
1. **Lineup margin.** A whole-lineup edge is worth about twice what real spreads say (item 1). Proposal for Raichel: keep each position's value where it is relative to the others and scale the whole down (the margin per rating point of lineup strength from about 4.8 to about 2.5), through the noise in the battles (block drift, throw error, tackle chance), not through a flat factor. Proposed, not confirmed.
2. Blowouts and one-score games (item 2), the same cause.
3. Droughts (item 3): the same cause, plus the firing loop.
4. College completion (item 4): receivers' catch radius and coverage at the lower levels.
5. College calling tables, sacks and returns (trial 3 item 6, still open).
6. The draft board's quarterback price (item 6).
7. The coach-of-color pipeline (item 7).
8. The career stats record and high school games on the engine are still unbuilt.
