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
