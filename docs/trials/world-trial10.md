# World trial 10: the world for the first saves (2026-10-20/21)

One world, seed 2026: the spin-up from 1984, 30 recorded years of history (1996-2025) and one future season (2026). Code: main at commit 7594062 (build 10, PR #66; high school games on the engine, PR #69; save and load, PR #74). Every college and NFL game ran on the match engine. Every high school game ran on it for the history's last 12 seasons (2014-2025) and the future season (Cain, section 8 choice 2). Asked by Cain on 2026-10-20 08:55 ("run a 12 year world") and 11:04 ("an entire world we can use for our first few gameplay saves, meaning ncaa and nfl too").

Nothing in this trial was fixed. The numbered list at the end waits for Cain's yes or no. Real figures: nflverse 2021-25, FBS 2022-24. High school real figures are from memory and marked so.

- Debug Lab (Verdict tab has the list): https://claude.ai/artifact/3SAzca1V9NBFKkW5pAPTSt
- Player pages (129 players; 73 with high school stat lines): https://claude.ai/artifact/XmJk7Vjkv3p6nZ2nPYWMY8
- Raw output, report and timings: the project folder at `afm/trials-10/seed-2026/`

## The world file

The world after the history (February 2026) is the first save: `/mnt/project-files/afm/worlds/world-2026-seed2026.afmworld.001` to `.005` (the shared folder drops files near 2 GB, so it is split in 450 MB parts with a `.sha256` of the whole; the game's importer joins them).

- 1.98 GB, written in 63 s (format 1: `crates/afm-engine/src/save.rs`).
- Loaded with `world_file` in 73 s at 3.84 GB: 851,571 people, 28,576 alumni, 72,436 coaches, 14,078 high schools, 899 programs, 693,332 people with high school lines.
- The loaded world then played the 2026 season in 642 s: 5.61 GB after it, 6.41 GB peak (game log off).
- `a_saved_world_plays_on_identically` (`crates/afm-tools/tests/world.rs`) proves a loaded world plays the same games as the one written.

## Speed and memory

| | time |
|---|---|
| History, spin-up and 1996-2025 | 10,359 s (2.9 h) |
| A season before 2014 (no high school engine games) | 86-111 s |
| A season with high school engine games (76,300-76,850 games) | 603-646 s |
| The 2026 season | 638 s |
| The whole run with the trial exports | 10,999 s plus the exports |

Memory with the trial's game log on: 6.35 GB after the history, 6.71 GB peak. Without the log, a loaded world sits at 3.84 GB and peaks at 6.41 GB while a season plays. The 4 GB budget is section 8's first build step.

## High school games on the engine

- 13 seasons, 76,298-76,853 engine games each, nothing broke.
- A team game: 15.5 points, 53.5 plays, 258 yards (122 passing, 136 rushing), 0.89 turnovers. Every season within 0.1 of these.
- The state record books fill from the engine seasons. Best seasons across all states: passing 3,892 yards, rushing 4,594 yards and 64 touchdowns (Keondre Major, Fort Worth North TX 2023), receiving 1,134 yards, 72 catches, 7 interceptions, 86.5 sacks and 180 tackles for loss.
- Longest winning streaks: Mater Dei (Santa Ana) CA 248 straight (2011-2026), King (Houston) TX 150 and running.

What reads wrong (findings 2-6): scoring is low; the best pass rushers pile up impossible sacks (Daniel Bates, a DT at Leesburg Hills GA who also threw 79 passes: 86.5 sacks on 653 defensive snaps in 2021); receivers and interceptions are far below real leaders; a 248-game streak beats the real record (151, De La Salle).

## Build 10's findings, measured

| Finding | Trial 9 | Trial 10 | Real |
|---|---|---|---|
| Weekly elevations | 0-13 a year | 339-606 a year since 2020 | hundreds (from memory) |
| Practice squad poaching | 0 in 25 years | 5-27 a year | a few dozen |
| NFL players happy with their role | 10 of 2,141 | 7.8% in a good mood | |
| Round 1 WR / EDGE / OT | 20.7 / 20.4 / 6.2% | 13.6 / 15.0 / 11.7% | 13.4 / 13.5 / 12.1% |
| FBS yards a carry | 4.85 | 4.94 | 5.12 |
| NFL yards a carry | 3.99 | 4.29 | 4.50 |
| Trades | 3-18 plus 2-13 at the deadline | 18-34 plus 7-17 | |
| Tendency spread, NFL sd | 0.96-2.37 | 1.34-3.18 | agreed 3.2 |
| Tendency spread, FBS sd | 1.40-3.20 | 1.96-3.61 | agreed 3.8 |
| FBS roster median | 80 | 86 | near 105 |
| 5-stars drafted | 84.6% | 87.6% | 62% |
| FBS titles, most by one program in the history | Texas 18 of 50 | Georgia 18 of 31 | no one more than 6 in 15 years |
| NFL points a team game | 30.8 | 30.9 | kept for now (Cain) |

## Dynasties (finding 1)

Georgia won 18 and Texas 10 of the 31 FBS titles (1996-2026); Michigan 2 and USC 1 took the rest. 12 of the last 15 champions went unbeaten. Georgia took 223 of 1,362 5-star arrivals (16%; Texas 158), and its 2004-now head coach is the most overrated FBS head coach in the world (hidden head-coach value 13.6, record 299-18). Its starters were only 1-3 points ahead of the next best team in most title years (69.2, second best, in 1996). So a small edge turns into near-sure titles. Not traced further.

## Slumps

28,025 slumps: 9,572 rating drops and 26,013 form slumps (the Debug Lab's Slumps tab lists the 400 biggest).

- Gregg White (CB, 30, NFL rotation, 2021): 85.6 to 79.4 (-6.2), played at 80.6. A major injury, 12 games missed, a form slump and age-30 decline.
- Ronnell Young (CB, 33, NFL backup, 2017): 81.0 to 74.9 (-6.1), played at 68.6. A form slump and age-33 decline.
- Shawn Williams (EDGE, 30, NFL starter, 2003): 87.4 to 81.3 (-6.1). A major injury, 12 games missed, a form slump, age-30 decline.
- Marshall Dickerson (RB, 28, NFL backup, 2012): 65.6 to 60.0 (-5.6), a form slump alone.
- Biggest form slump: Darryl Eads (OT, 22, NFL backup, 1999) played at 53.7 against 71.5, his first year in the league with a major injury.
- Below the NFL: Philip Baber II (QB, 15, high school starter, 2012), 36.5 to 31.9 (a form slump, teaching that cost ability, poor coaching); Ethan Archie (EDGE, 15, high school starter, 2021), 41.6 to 37.4 (a major injury, 10 games missed).

Why they read right: big drops come with major injuries and age past 30. 339 of the 400 biggest are NFL players.

## Spot checks

- Georgia's 5-stars: 223 of 1,362 5-star program arrivals, counted from every player season line.
- The 86.5-sack season is the engine's own count: Daniel Bates's raw line has 86.5 sacks on 653 defensive snaps in 15 games; the record book halves sacks correctly (they are stored in halves).
- High school receiving: 1,095 yards tops the exported lines (88,555 people), 1,134 the full record book.
- NFL runs 4.29 a carry and 30.9 points a team game, from all 16,618 NFL team games' box scores.
- The file loads: 73 s, 3.84 GB, 851,571 people.

## Bugs found in the trial tooling (fixed in this PR)

- The player cards wrote the high school stat lines and the school's name under the same key, so the lines were lost. They are now `hs_stats` (`crates/afm-tools/src/dashboard.rs`, `tools/dashboard/build_pages.py`); this trial's cards got their lines matched from the raw export by name, school and years.
- The Debug Lab kept Cain's ticked answers under one key for every trial, so trial 9's ticks would show on trial 10's items. The key now names the trial.

## Doubts

- High school real figures (points, sacks, receiving, interceptions) are from memory and need a source before tuning.
- College scouting doesn't read high school stat lines yet; section 8 wires that.
- The world file holds every random stream. Careers started from it get fresh randomness only if the game reseeds them, as section 8's choice 1 says.

## Findings (yes or no each)

1. **Dynasties.** Georgia 18 and Texas 10 of 31 titles, though Georgia's starters were only 1-3 points ahead. Fix: trace why a small starter edge turns into near-sure titles (depth, the engine's spread of results, playoff seeding), then add the real leveler it bypasses.
2. **High school scoring.** 15.5 points a team game on 4.8 yards a play (real about 26, from memory). Fix: tune high school offense with `match_check hs` toward real points and yards a play, after a real source is found.
3. **Help against the best pass rusher.** 86.5 sacks in one season, 222 seasons of 30+ in the sample (real record about 40, from memory). Fix: high school offenses slide, chip and double the best rusher, as real coaches do.
4. **The best receiver gets the ball.** Top season 1,134 yards in 13 years (real: many 1,500-yard receivers a year). Fix: high school targets lean toward the best receiver.
5. **High school interceptions.** Best season 7 (real leaders 10-15). Fix: re-measure after 2 and 4, then tune throws into coverage.
6. **Winning streaks end.** Mater Dei 248 straight (real record 151). Fix: trace what keeps a top school unbeaten for 16 years and add the real leveler.
7. **5-stars drafted too often.** 87.6% (real 62%). Fix: trace where 5-stars beat real (development, busts, injuries) and tune the one that's off.
8. **FBS rosters still short.** Median 86 (real near 105); in-state share 28% (real 37%). Fix: more local walk-ons stay on, as in real programs.
