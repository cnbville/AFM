# NFL scoring creep: what the future run's book says (realism audit item 25, finding 2)

Read-only, 2026-11-26 (thread "Fix NFL scoring creep"), on the book `future-seed321-20261125-7071c19` (main `7071c19`, before P1, the rushing lane, the NIL lanes and the blitz lane). No new world run. Raw reads: `future-run-seed321-raw/creep-*.txt`. Nothing in the game was changed: the cause is not isolated yet (see "Not explained").

## The creep is real and it is the running game

- NFL points a team game by season, 2026 to 2035: 25.6, 24.3, 24.7, 25.5, 24.0, 24.6, 25.5, 25.2, 26.2, 26.7. A weighted line through them rises 0.158 a year (se 0.048, 3 sigma). Real 2016-25 is flat at 21.7 to 24.8 (mean 22.9), so the level was already about 1.5 over in 2025-26 (known) and the drift adds to it.
- It is not volume. Plays a game 62.5 to 62.3, drives 11.4 to 11.5, pass rate 0.549 to 0.544, starting field position 31.5 to 31.7, turnovers 0.98 to 1.07, penalties flat.
- It is not passing. Gross yards an attempt 7.30 (2026) to 7.24 (2035) in the box score, completion rate 66.2% to 66.7%, EPA a pass 0.156 to 0.134, passes' share of third downs flat. (The check's 7.34 to 7.64 adds sack yards and a 2025 year of 32 games; it is not the passing game.)
- It is the run. Yards a carry 4.52 (2026) to 4.91 (2034/35); yards before contact 2.60 to 2.99; explosive runs 10.2% to 11.3%; EPA a carry -0.038 to +0.007; points a drive 2.11 to 2.32. Team yards a run play: 4.53 (2026), 4.59 (2029), 4.59 (2032), 4.69 (2033), 4.87 (2034), 4.81 (2035); the slope is 0.034 a year (se about 0.005).

## What it is not

- **Not the call mix.** Run shares by concept barely move (inside zone 40% to 37%, outside zone 12% to 12-17%, zone read 3.7% to 6.8%), and every concept gains: inside zone 4.07 to 4.32 early to late, outside zone 4.70 to 4.88, zone read 3.73 to 3.91, duo 3.79 to 4.02, counter 4.70 to 4.86, draw 4.12 to 4.21, power 4.55 to 4.63. The defense's box is the same (6.57 to 6.58 men), but runs against 6 and 7 men gain more (6 men 5.01 to 5.20, 7 men 4.71 to 4.99); 8 or more is flat.
- **Not coaches.** NFL head coaches' scheme and game management 16.4 / 16.2 both ends; coordinators' ratings within 0.5; club coaching 15.7 every year; no coach ratchet.
- **Not play ideas spreading.** `world/trends.rs` records and applies college divisions only; NFL clubs keep novelty 0 (`world/season.rs`, `game_team`; `world/pro/games.rs`, `build_side`). So the old candidate in GAPS.md can't be it.
- **Not the starters' overall.** Unit means move little: OL starters 81.9 to 82.3 in the team panel, 80.8 to 82.5 in the league series (2021 to 2035); DL 80.4 to 80.2; LB 80.0 to 79.7; DB 80.1 to 79.5; RB 78.0 to 79.6; QB 85.0 to 83.1. Draftees' first-season overall by unit: no pattern. Starters are 0.5 years older.
- **Not craft.** Flaw totals fall by 0.05 (DL 0.42 to 0.37, DB 0.42 to 0.37, OL 0.24 to 0.22) and tools by less: worth about 0.25 of a rating point.
- **Not conditions or field position.** Wind, rain, kick returns, touchbacks, punt nets flat. Turf rises 50% to 68% of games, but the engine reads turf for non-contact injuries only; temperature reads only in kicking.
- **Not tendencies.** Means move by 0.5 of a point on a 20 scale at most, in both directions.

## What does move

- The blockers win more, steadily: run blocks won on run plays 0.839 (2027-29) to 0.860 (2034); OL run-block win rate in the season stats 0.832 to 0.845; pass-block win rate 0.855 to 0.882; defensive linemen's pass-rush win rate 0.163 to 0.124-0.135 (2034-35). Pressures (7.5 to 7.3) and sacks (2.0 to 2.1) a game stay flat.
- The defense softens a little at the top: players over 76 overall: DL Strength -2.3, Anchor -1.3, Block Shedding -1.3 (83.5 to 82.2 by 2034); OL run blocking flat (83.3 to 83.4), OL speed +1.6.
- Team-year panel (229 NFL team seasons, 2026-35, starters' overall by unit against the team's yards a run play): offense run +0.037 a point of OL, +0.019 RB, +0.019 TE; defense run -0.073 a point of DL, -0.044 LB, -0.026 DB. The units' drift (DL -0.7, LB -0.6, DB -0.8 over 2026-34) is worth about +0.1 of the +0.37 a carry; ratings alone explain about a quarter.

## Not explained

About 0.25 yards a carry of the rise is not in the starters' ratings, bodies as far as the book keeps them, tendencies, craft, the calls, the coaches or the conditions. It is spread over every concept, so it sits in how blocks and tackles come out, not in what is called. A step of +0.25 between 2033 and 2034 is about two sampling errors on top of the trend.

The book can't separate rosters from the engine: it has the games, not the rosters, and it was played before the lanes that changed rushing (#641, #668) and the run lean (`LeagueMen`), so main today may creep differently.

## Proposed next (proposed, not confirmed)

1. **Keep the rosters of a future run.** Have the play-on tool (`afm_future`) write a world file on 1 January of 2026, 2030 and 2035, so the engine can play the same 32 clubs' rosters at each date with `match_check` (`AFM_AB_WORLD`) on one engine and one set of seeds. If 2035 rosters run 4.9 and 2026 rosters 4.5, the creep is in the men (then `AFM_TYPICAL` moves one unit at a time to the 2026 level and names it: OL, DL, LB, DB, RB bodies, hidden traits or tendencies). If both run the same, it is in how the world feeds the game (coaching, the call lean, staffs, injuries). It costs one 10-season world run on current main (about 6 hours of runner time) and is the same run as audit item 25's re-run on a rebuilt world.
2. **If it is the men, fix the growth cause and not the scoring.** A flat cap on points is out. The fix would be where DL and LB lose strength and anchor and the line gains (position-level growth and the draft), measured on the class sim like the other growth lanes.

## Run started (2026-11-27, Cain: "agree" to both items)

Item 1 is built: `afm_future` takes `AFM_KEEP=<folder>` and hard-links each date's world file there, and `future-run.yml` keeps three dates (1 Jan 2026 start world, 2031 after the first job, 2036 after the second, one artifact each) and has three `creep` jobs, one a date. Each plays that date's 32 NFL clubs with `match_check` (`AFM_AB_WORLD`, seed 7, 1,500 games an arm, this commit's engine) in eight arms: the clubs as they are, every position moved to the league's typical starter (`AFM_TYPICAL=ALL`: the engine and the feed alone), and one unit moved at a time (OL, DL, LB, DB, RB, QB/WR/TE). The run is also audit item 5's re-run on a rebuilt main, so its book is read for the other drifts too.

How to read it (proposed, not confirmed): if 2036's plain arm runs more yards a carry than 2026's and the typical-everywhere arms run the same at every date, it is the men, and the unit arm that closes the gap names the unit. If the plain arms match across dates, the creep is not in the rosters but in how the world feeds the game (staffs, the call lean, injuries), and the book's play-by-play reads decide.

## Result: the January rosters do not creep (2026-11-28, run 46080381856)

The three kept worlds (1 Jan 2026, 2031, 2036, the 10-season run 46010118001 on main `f84145c`) were each played by main `b3c90da`'s engine: the 32 NFL clubs, `match_check nfl 1500 7` with `AFM_AB_WORLD`, the same seed at every date, healthy depth charts. Standard error on points a team game is about 0.18 (sd 10 over 3,000 team games), on yards a carry about 0.03.

| arm (moved to the typical starter) | points 2026 / 2031 / 2036 | yards a carry 2026 / 2031 / 2036 |
|---|---|---|
| none (the clubs as they are) | 26.09 / 25.71 / 25.35 | 4.573 / 4.534 / 4.538 |
| every position (`ALL`) | 25.26 / 25.38 / 24.99 | 4.141 / 4.223 / 4.124 |
| OL | 24.58 / 24.12 / 23.99 | 4.248 / 4.273 / 4.239 |
| DL (edge and tackle) | 27.82 / 27.40 / 26.17 | 4.637 / 4.570 / 4.483 |
| LB | 25.94 / 25.97 / 25.54 | 4.556 / 4.568 / 4.591 |
| DB (corner and safety) | 25.02 / 24.14 / 24.17 | 4.632 / 4.524 / 4.576 |
| RB | 26.25 / 26.23 / 25.53 | 4.677 / 4.667 / 4.759 |
| QB, WR, TE | 26.01 / 25.92 / 25.55 | 4.399 / 4.312 / 4.312 |

Plain arm, the other rows: yards an attempt 7.80 / 7.74 / 7.59, explosive run rate 0.095 / 0.096 / 0.096, run EPA -0.046 / -0.049 / -0.042, pass rate 0.553 / 0.581 / 0.555.

What it says:

- **The same engine plays the 2036 clubs no better than the 2026 ones.** Points fall a little (26.1 to 25.3, about two errors), yards a carry are flat (4.57, 4.53, 4.54). The future run's creep (24.1 to 26.7 points, 4.52 to 4.91 a carry) is not in the January rosters. The men are not the cause, so item 2 (fix growth and draft for DL and LB strength and anchor) has nothing to fix, and I have not touched growth.
- **The starters' ratings are flat too.** Overall against the level's typical starter at 2026 / 2031 / 2036: edge 79.6 / 79.5 / 79.5, tackle 80.8 / 80.4 / 79.9, linebacker 78.5 / 80.1 / 79.9, tackle (OT) 82.5 / 81.8 / 82.2, guard and center 80.1 / 80.5 / 80.5, running back 77.6 / 77.3 / 76.8, quarterback 83.6 / 82.9 / 82.8. Linebackers' strength is up (79.3 / 83.8 / 84.9), not down. The earlier read (defenders' strength, anchor and shed down 1 to 2 points) came from the men on the field in the played games, not from the January depth chart.
- **Moving everyone to the typical man costs the clubs 0.6 to 0.9 points and 0.4 yards a carry at every date**, the same at each, so the world's men are a little better than the typical ones all through and not more so later.
- **The engine on healthy full-strength clubs scores 25 to 26 at every date, above real (22.5)** and above the 2025 played season (24.1). That is a separate, flat gap (passing 7.6 to 7.8 a throw against real 7.05, third downs 0.43 to 0.45 against 0.40); it is where the engine's passing sits on world rosters (audit item 4 and `GAPS.md`), not the creep.

So the creep is in what the season does to the men or who plays, not the 1 January depth chart: something between the January starters and the players on the field in a played season gets weaker for the defense or better for the offense as the years pass. The candidates, all in the book of the replay (run 46080381856, release `future-seed321-<date>-b3c90da`, read with `future-read.yml`):

1. Who plays: the share of defensive snaps by depth order (starter, second, third string) by season, and the overall, strength, anchor and shed of the men on the field by season against the January starters.
2. Injuries: games missed by starters by season and position, and how long they last, since the engine's in-season games carry injuries that `match_check` does not.
3. Roster size and depth by position in week 1 and week 18 by season (cap-to-revenue, #669, may thin depth as the cap outruns revenue).
4. Rest, weather and the schedule's mix by season.

Next: read the book when the replay finishes (about 03:00 UTC on 28 November and after) and name which of the four moves.

## Result 2: the rebuilt main does not creep (2026-11-28, book `future-seed321-20261128-b3c90da`)

The replay's book (the same seed 321 world played 10 seasons on 1 January 2026 to 2036 by main `b3c90da`, with the rushing lanes #641 and #668 and the league-relative run lean that the first book (`7071c19`) was played before) was read with `future-read.yml`. Raw reads: `future-run-seed321-raw/creep-b3c90da-r46111846493-2.txt` and `-3.txt`.

- **Yards a carry are flat.** By the plays: 4.75 (2026), 4.71, 4.55, 4.65, 4.63, 4.60, 4.63, 4.61, 4.63, 4.77 (2035), a slope of -0.001 a year (se 0.003; the first book rose 0.034 a year). By the box scores: 4.66 to 4.70, slope +0.001 (se 0.004). Yards before contact 2.64 to 2.65, explosive runs a game 5.6 to 5.6, box count 6.55 to 6.61.
- **Points are flat within noise.** 25.4, 25.6, 25.9, 25.7, 24.9, 24.9, 25.3, 27.2, 26.2, 25.3 (2026-35 regular season, 544 team games a year, se about 0.43 a year): slope +0.05 a year (se 0.05; the first book 0.16, se 0.05). The 2033 and 2034 seasons are the high ones (27.2 and 26.2) and then it comes back to 25.3, which is noise, not a ratchet. Plays 62.2 to 61.6, pass rate 0.541 to 0.536, yards an attempt 7.14 to 7.13.
- **So the creep was the old rushing engine, which the lanes fixed.** It is the same drift the first book showed, in the same world, gone on the rebuilt engine, and the January rosters play flat on one engine (result above) and in a played season. Nothing was changed for it in this thread; the earlier lanes did it.
- **Who plays does not move.** Snap-weighted overall of the men on the field, 2026 to 2034: DL 78.0 to 77.7, LB 77.9 to 77.6, CB 78.8 to 77.4, S 78.9 to 77.8, OL 81.8 to 81.5, WR 80.2 to 79.7, RB 76.6 to 77.0, TE 78.2 to 79.2 (up), QB 83.5 to 82.7. Substitutes' share of defensive snaps 35% to 41% (CB), 41% to 43% (DL), 38% to 46% (LB), 24% to 24% (S). Starters' games missed 0.47 to 0.56 a season and injuries a game 4.0 to 4.2 (about 3 weeks each): flat. Players with a game per club rise 114 to 134 (churn, not scoring).
- **What is still open, and it is not the creep:** the level. Played NFL scoring runs 25.4 against the real 22.5 (2025's 24.2 in the same book), passing 7.1 to 7.3 gross yards an attempt against 7.05 real (the match_check row, which adds sack yards back, reads 7.6 to 7.8) and the third-down rate is 3 points over. That is audit item 4 and the engine gaps in `GAPS.md`, a flat gap each season.

Decision: the thread's two items are closed. Item 1 (keep rosters) is built and measured, item 2 (fix growth and draft for DL/LB strength) is not needed because the men are not the cause and the creep is gone. No code change and no cap on scoring.
