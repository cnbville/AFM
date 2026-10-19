# World trial 8: one 50-year world, with tendencies

Seed 2026: 30 recorded years of history (1996-2025, after the spin-up from 1984), then 20 seasons into the future (2026-2045). Every college, high school and NFL game ran on the match engine. Built on main after PR #50 (the tendencies build), with the trial 7 fix-now list (PR #46 and before) and the 67 attributes.

Raichel asked for it on 2026-10-18: "Run a single 50 year world test as soon as the tendencies build is finished so go into the future but wait for the tendencie build", and picked 30 + 20 on the card. It ran overnight with no questions asked. Nothing below is fixed: every finding waits for a yes or no.

- Time: the history built in 6,132 s (102 min); the 20 future seasons took 3,218 s (154-171 s each). The run took about 2 h 55 min wall with the exports. Peak memory 7.77 GB with the trial's game log on.
- Raw output: project folder `afm/trials-8/seed-2026/` (report, cards, raw export gzipped, the profile in `stderr.txt`, memory samples in `rss.txt`).
- Debug Lab: https://claude.ai/artifact/3SAzca1V9NBFKkW5pAPTSt (version 8, one world, with a new Tendencies tab). Player pages (137 players, 117 with a tendencies box): `docs/trials/artifacts/world-trial8-player-pages.html`, also at https://claude.ai/artifact/XmJk7Vjkv3p6nZ2nPYWMY8.
- One checker pass re-measured 11 claims from the raw files with its own scripts (see "Spot checks").
- The five engine bugs the tendencies build reported (`afm/tendencies-build/results.md`) are marked "already reported" where they show here; they are not new findings.

Numbers are 1996-2045 unless stated. "Traced" means read in the code; "inferred" means not confirmed in the code. Real NFL figures are nflverse 2021-25; real FBS figures 2022-24.

## The trial 7 fix-now list, measured

| | Fix | Trial 7 | Trial 8 | Verdict |
|---|---|---|---|---|
| 4 | Blue-chip walk-ons | 27% of 4-stars and 12% of 5-stars who reached college walked on; 26.5% of picks | 2.3% of 4-stars (363 of 15,817) and 0.2% of 5-stars (3 of 1,533), classes 1996 on; 5.3% of picks (682 of 12,800) | works (rosters shrank: finding 5) |
| 5 | The uncapped 2010 year | 0 signings, trades, claims, restructures, cuts | 81 signed with a new team, 12 trades, 46 claims, 270 restructures, 166 cap cuts, 122 RFA tenders; 49 UFAs (2009: 168; six accrued seasons needed, as real) | works |
| 6 | Franchise tags | 11 in 30 years, none in 21 | 7-23 every year of 50 (history 8-23, future 7-16) | works |
| 7 | Comp picks | 0-5 a year from the formula, 27-32 supplemental | 2021-2045: 12-27 from the formula (mean 19), 5-20 supplemental (mean 13); the history's formula from 4 | partly (finding 8) |
| 11 | Repeated extensions | 60 pros extended two to four times at one age | 41 pros in 50 years show two extensions at one listed age; the one case traced was a re-up in the next league year | works |
| 12 | Non-quarterbacks at quarterback | about 20 FBS seasons with a non-QB throwing 250-456 passes | 0 FBS seasons; 6 junior college seasons (54-man rosters) | works at FBS |
| 13 | Class rankings | a junior college in the top three of 28 of 31 | 0 of 80 ranking posts with a non-FBS program in the top three | works |
| 14 | Long snappers | 4.57 a roster, 3.87 on scholarship | 1.96 a roster (real 2.5), 1.70 on scholarship (real usually 1) | count works; scholarships high (finding 9) |
| 15 | Player pages | 0 of 24 cards before 2002 had college stats | 28 of 30 cards of classes before 2006 have them; the offers note not re-measured | works |
| 16 | Speed | 94 min for the history | 102 min for the same history | slower (finding 10) |
| 17 | Text and export | wrong plurals, trade texts, era names, duplicate pros | 0 wrong plurals found, 0 of 591 trade texts, 0 Commanders before 2020, 0 duplicate pros of 41,179 | works |
| 18 | NFL teams off the engine | 3 team-seasons | 0 team-seasons short of the schedule | works |

## Findings 1 and 2 after the 67 attributes and the retune

- **NFL scoring (finding 1): better, still too high.** 31.2 points a team game (trial 7 37.8; real 22.5): 30.2-33.4 a year in the history and 29.2-32.4 in the future. Plays 69.5 (61.9), 7.69 yards a pass (7.05), completion 60.4% (64.5%), 5.04 a carry (4.50), interceptions 2.66% (2.27%), sacks 6.4% (6.75%), third downs 42.4% (39.8%), one-score games 48.8% (53.5%). 5.2% of team games reach 50 points (trial 7 15.9%). The passing leader throws for 5,805-8,310 yards and 36-76 touchdowns (real records 5,477 and 55; season lines include playoff games), and 25-31 quarterbacks pass for 4,000 yards in each future year. The engine alone plays NFL games at 26.1 points and 68.0 plays on synthetic rosters (`match_check nfl 300`, run this morning), so pace is about 10% high in both and the world adds about 5 points more (not traced).
- **College stat lines (finding 2): committees work, the lines still run past the records.** The lead back takes 52% of his team's backs' carries (team-seasons with 250+). Yet 3-14 FBS backs rush for 2,000 yards a year (mean 8.3; real 0-2), 82 seasons beat the 2,628-yard record, in 41 of 50 years (trial 7: 55 seasons in 30 years), and the leader has 2,265-4,582 yards on 243-473 carries. Sack leaders reach 20-56 (record 24), tackle-for-loss leaders 34-76. FBS runs gain 6.18 yards a carry (real 5.12) with a 51% success rate (real 42%), so the run game itself is too easy.

## Parked items, measured only

Nobody tunes these until the systems that feed them are built.

- **Dynasties (finding 3).** 50 FBS titles went to 13 programs: Georgia 13, Georgia Tech 11, Alabama 8, Florida 4, Texas A&M 3. Georgia Tech won 10 of the 20 future titles and 9 in one 15-year window (bar 6); only 7 programs won a title in the 20 future years. The history alone: Georgia 10, Alabama 8, 9 champions.
- **Trades (finding 8).** 5-17 a year in the history, 7-18 in the future. Offensive linemen are 20% of traded players, safeties 18%, edge rushers 16%.
- **Awards (finding 9).** Comeback Player of the Year went to a long snapper 16-17 times, a punter 8 and a kicker 3-4 in 50 years (trial 7: 9 specialists in 30). MVP: a quarterback 39 times, a back 6. Offensive Player of the Year: a back 26 times. Defensive Player of the Year: linebackers 17, tackles 11, edge rushers 10. Defensive Rookie: edge rushers 17, safeties 15.
- **Round 1 positions (finding 10).** Receivers 20.6% (real 13.4%), edge rushers 15.7%, quarterbacks 11.4%, corners 10.4% (11.9%), tackles 6.5% (12.1%), tight ends 2.5% (3.6%). No kicker, punter or snapper in round 1 in 50 drafts (trial 7: 4).
- **5-star draft rate (part D 4).** 88.4% of 5-stars drafted (1,284 of 1,452), 42.9% in round 1 (real 62% and 21%; trial 7 85.9% and 39.9%).

## Tendencies in the world

Measured on the players each habit drives (its "owners"), at the end of 2045. Agreed spreads (sd): high school 4.8, FBS 3.8, NFL 3.2.

| Tendency | HS mean / sd | FBS mean / sd | NFL mean / sd | NFL owners coached down 2+ |
|---|---|---|---|---|
| Bounces runs | 10.7 / 3.2 | 13.5 / 2.6 | 13.0 / 2.0 | 35% |
| Scrambles | 10.7 / 3.2 | 13.3 / 2.6 | 12.5 / 2.7 | 46% |
| Gunslinger or checkdown | 10.9 / 3.1 | 11.1 / 1.5 | 11.4 / 1.1 | 30% |
| Holds for the big play | 9.6 / 3.2 | 9.9 / 1.7 | 10.3 / 1.5 | 50% |
| Patient or downhill | 10.0 / 3.7 | 7.5 / 2.1 | 8.0 / 2.1 | 44% |
| Returner risk | 10.5 / 3.2 | 11.2 / 1.9 | 10.3 / 1.6 | 62% |
| Rushes upfield | 10.0 / 3.5 | 11.1 / 2.8 | 9.7 / 2.8 | 62% |
| Times the snap | 9.6 / 3.6 | 8.2 / 2.3 | 8.8 / 2.3 | 55% |
| Gets his hands up | 11.0 / 3.9 | 11.0 / 2.8 | 10.9 / 1.6 | 0% |
| Goes for the strip | 9.8 / 3.3 | 9.5 / 1.8 | 9.7 / 1.7 | 61% |
| Big hit | 9.8 / 3.3 | 10.1 / 2.1 | 10.2 / 2.1 | 64% |
| Jumps routes | 9.7 / 3.5 | 8.8 / 2.0 | 9.9 / 2.0 | 56% |
| Contact willingness | 9.9 / 3.9 | 10.6 / 2.9 | 10.7 / 1.8 | 2% |

- **They show in the stat lines.** Bottom fifth against top fifth of each habit, 2045 season lines: upfield edge rushers 0.06 against 0.16 sacks a game (FBS, r 0.27) and 0.09 against 0.28 (NFL, r 0.33); route-jumping corners 0.03 against 0.11 interceptions a game (FBS, r 0.34) and 0.04 against 0.15 (NFL, r 0.36); downhill linebackers make 8.0% against 13.4% of their tackles behind the line (FBS, r 0.35); scrambling NFL quarterbacks run on 5% against 13% of their plays (r 0.84, 32 quarterbacks); hands-up tackles bat or defend 0.07 against 0.15 passes a game (FBS, r 0.20). Two show the wrong sign: gunslinger against interceptions (FBS r -0.17, NFL -0.03) and holding against sacks (FBS -0.20); see "Doubts".
- **Slow backs who bounce don't reach the pros with the habit.** NFL alumni backs, slowest quarter by speed and acceleration: bounce 8.4 now, 13.2 before coaching, 1% at 14 or more; fastest quarter 14.1 (40% at 14+). At FBS the slow quarter is still at 11.5.
- **Habits settle with age.** NFL scrambles: 14.0 at 22-24, 12.4 at 25-28, 10.5 at 29 and up (lost speed takes it down). NFL bounce: 14.4 at 22-24, 12.4 at 25-28.
- **Coaching styles.** Current head coaches, mean (sd): risk, physical, ball hunting and moulding sit at 9.3-11.7 at every level. Freedom does not: high school 10.9 (3.3), FCS 8.8, junior college 7.2, FBS 6.0 (2.9), NFL 5.8 (2.2). See finding 4.

## The future, 2026-2045

- The world held for 20 seasons: every team played every game on the engine, the market ran every year (tags, extensions, trades, comp picks), and 15 different teams won the 20 future Super Bowls (Buffalo 4).
- NFL scoring in the future 29.2-32.4 a year (history 30.2-33.4); yards a pass 7.56-7.71; sacks 2.6-3.1 a team game.
- Tendency averages barely moved: at most 0.4 points at any level over the 20 years (NFL scrambles 12.1 to 12.5, NFL bounce 12.8 to 13.0 with a yearly range of 12.7-13.3; high school under 0.05). Spreads changed by 0.3 at most.
- FBS titles concentrated further (Georgia Tech 10 of 20; parked finding 3).
- Coaching diversity: FBS head coaches of color rose from 4 of 120 (2009) and 10 of 125 (2016) to 38 of 136 in 2045 (trial 7 ended at 15 of 133 in 2025); coordinators 40% and position coaches 52% now, with FBS players 48% Black.
- Names: 28.0% of 847,697 players share a full name with another active player (trial 7: 28.8%).

## Positives

1. 11 of the 12 trial 7 fixes work in the world (table above); comp picks partly.
2. Twenty future seasons run without anything breaking or drifting (above).
3. NFL scoring fell from 37.8 to 31.2 points a team game, and 50-point games from 15.9% to 5.2%.
4. Tendencies show in whole seasons, not only in the match checks (above).
5. Slow backs who bounce get coached out of it before the pros, as Raichel asked.
6. FBS games stay close to real: 29.1 points a team game (real 27.8), third downs 42.0% (40.3%), home wins 57.3% (58.4%).
7. No specialists drafted in round 1 in 50 drafts.
8. Coaching diversity grows from a realistic start in the history.

## Findings (yes or no each)

1. **Injured reserve never ends** (plain bug, new). 124 pros spent two or more straight full seasons on injured reserve (60 two, 32 three, up to 9). Jason Ridgeway (RB, Giants) went on reserve in 1998 and sat there all of 1999-2002 (22-23 reserve weeks a year) until his contract ran out. Full-season reserve players: 16 in 2001, 27 in 2006 and 2011. Why: traced: a healed player leaves reserve only while his team has returns left (`pro/mod.rs`, `heal_week`; `games.rs`, `ir_returns`: none before 2012, one in 2012-16, two in 2017-19, eight from 2022). The new league year resets that count (`mod.rs:1097`) but leaves him on reserve, and cutdown skips reserve players (`mod.rs:1494`). Real: the reserve lists end with the season. Fix: at the new league year every healed player on reserve goes back to the 90-man roster; one still hurt moves to the reserve/PUP list and comes back when healed.
2. **Tendencies are too alike at FBS and in the NFL** (plain bug against the agreed spread, new). NFL sd 1.1-2.8 (agreed 3.2): gunslinger 1.1, holds 1.5, returner risk 1.6, hands up 1.6, strip 1.7, contact 1.8. FBS 1.5-2.9 (agreed 3.8). NFL quarterbacks' gunslinger runs only 10-13 from the 5th to the 95th percentile. Why: traced: every year to about 26, three pulls work and nothing pushes out (`tendencies.rs`, `develop_year`): results teach about a point a year toward what his tools support, coaching closes the rest of any gap over a point (30-69% of NFL owners were coached down 2+ points), and the staff's style and scheme pull everyone on a team toward one target. By the pros a habit mostly mirrors his ratings. Fix: results teach only the part of the gap beyond 3 points (inside it a habit is his style, not a mistake); coaching takes a habit only to the edge of that band; the style and scheme pull stops at the same band.
3. **Tendency averages sit off the engine's middle** (plain bug, new). Owners' averages: bounce FBS 13.5, NFL 13.0; scrambles FBS 13.3, NFL 12.5 (14.0 for NFL quarterbacks aged 22-24); patience FBS 7.5, NFL 8.0; reaching the ball out NFL 8.3; snap timing FBS 8.2. The engine reads 10.5 as the real NFL average. Why: traced: `ideal()` measures every attribute against the level's speed norm plus 8 (`tendencies.rs:345`), so fast positions sit above the middle, and the engine reads the raw number with no level centring (`game/sim.rs:725`). Synthetic teams in the match checks are centred on 10.5 (`Tendencies::at_level`); the world is not. Inferred: this adds to the already-reported bug that backs run outside twice as often as real. Fix: `ideal()` reads his attributes against the average at his position and level, so the average back sits at 10.5 and a faster one above it.
4. **Nearly every FBS and NFL head coach gives little freedom** (design choice, new). Freedom among current head coaches: FBS 6.0, NFL 5.8 (78% of each under 8), FCS 8.8, high school 10.9; the other four settings sit near 10.5 at every level. Why: traced: freedom is read as 21 minus his discipline (`world/coach.rs`, `style`), and top jobs go to disciplined coaches. Fix: read freedom against the head coaches at his level (21 minus discipline, plus the level's average discipline minus 10.5), so each level has strict and free staffs around the middle. The style rules are all proposed, not confirmed.
5. **FBS rosters are 20% short after the walk-on fix** (plain bug, new). Rosters going into 2046: median 82 (70-89; trial 7 92; the cap is 105). Against the real table scaled to 105: QB 4.1 (5.1), WR 10.4 (14.2), LB 8.8 (12.8), S 7.3 (10.5). In-state share of FBS signees: median 25% (trial 7 38%, real 37%). Why: inferred, not traced: fix 4 took blue-chips out of walk-on places, and the places they left are not filled by the local unrated players who used to make up much of each roster and of the in-state share. Fix: trace why those places stay empty, then let staffs offer them to local unrated and 2-star players until the roster reaches its plan (the walk-on cost unchanged).
6. **NFL games still score too much** (plain bug, trial 7 finding 1). As above: 31.2 points and 69.5 plays a team game (real 22.5 and 61.9). Already reported and likely part of it: a won pass rush barely raises pressure; quarterbacks scramble more than real. Fix: pace first (plays per game to real: the clock between plays and each team's tempo), which alone takes off about 10%; then trace the world's extra 5 points over the synthetic check, toward the agreed 20-25.
7. **FBS runs are too easy** (plain bug, trial 7 finding 2). 6.18 yards a carry (real 5.12) and a 51% success rate (42%); the stat lines above follow. Already reported and likely part of it: backs reach the line 0.7 s too fast and run outside twice as often as real. Fix: retune FBS run blocking against run defense to about 5.1 a carry, then check the leaders against the real FBS leaders (1,479-2,358 yards) and the record (2,628).
8. **Comp picks are still a third supplemental** (plain bug, fix 7 partly). 2021-2045: 12-27 formula picks a year (mean 19) and 5-20 supplemental (mean 13). Real: the formula gives nearly all 32 and supplemental picks fill a small gap (from memory, to verify). Why: not traced. Fix: trace the qualifying contract bar and the net-loss count against the real formula (the yearly pay bar, and losses cancelled by signings), then measure.
9. **Too many long snappers on scholarship** (plain bug, fix 14 partly). 1.96 snappers a roster (real 2.5) but 1.70 on scholarship (real usually 1). Fix: a snapper gets a scholarship only when the team has none on one; the others walk on.
10. **A season takes 160 s** (design choice). The history took 102 min (trial 7 94 min) with the 67 attributes and tendencies added; a future season 154-171 s on 4 cores: college games 59-65 s, summer signing 22-26 s, the NFL 15-17 s, high school 14-16 s, coaching searches 7-10 s, signing days 6-7 s. Fix (proposed): read each recruit's summer choice on all cores from his own stream (same results), as the trial 7 speed fix did for position moves and searches.
11. **Memory peaked at 7.8 GB** (design choice). 7.77 GB resident with the trial's game log on (box scores, stat lines, play-level situations; trial tooling only). Raichel's M1 has 8 GB. Why: not measured without the game log. Fix: measure a world without the game log at the next trial, then set a budget (proposed 4 GB for the world) and trim the biggest stores to it.

Parked, measured only (no answer needed): 12 dynasties, 13 trades, 14 awards, 15 round 1 positions, 16 the 5-star draft rate (all above).

## Slumps

14,100 rating drops and 35,686 form slumps (the Debug Lab's Slumps list shows the 400 biggest, now with "teaching that cost ability" as a reason). The biggest:

- Javier Mattos (RB, 29, NFL backup, 2031): 57.5 to 51.2 (-6.3), played at 49.3. A form slump a year after a season without a team; his ceiling also fell 5.3 that year, the early aging of backs, which the label only names from 30.
- Mitchell Kamakana (CB, 37, backup, 2035): 83.3 to 77.0 (-6.3). A moderate injury, a form slump and age-37 decline.
- Sylvan Woodson (RB, 31, backup, 2024): 65.1 to 58.9 (-6.2). A major injury, 15 games missed, age-31 decline.
- Diego Herrera (WR, 28, backup, 2028): 72.7 to 66.6 (-6.1), played at 59.6. A major injury, 11 games missed, a form slump.
- Cade Allen (QB, 25, backup, 2041): 70.0 to 64.3 (-5.7), played at 56.9. A form slump and teaching that cost ability: his coaches rated 13.4, so it came from a bad situation (inferred: a scheme that did not fit him), and his coachability of 7 left him exposed; unsigned the next year. 20 of the 400 biggest slumps carry that reason.
- Biggest form slumps: Hamza Rademacher (QB, 23, NFL rookie, 2029) played at 56.4 against a 72.9 rating; Damion Chapman (QB, 25, 2002) at 56.4 against 72.8.
- Below the NFL: the biggest drop is 4.5 (Aaron Decant III, WR, 16, high school starter, 2043: a major injury, 10 games missed, a form slump). At FBS: Brandon Salgado (S, 21, backup, 2045), 50.0 to 45.6 (a major injury, 9 games missed, a form slump).

Why they read right: big drops come with major injuries and age past 30 (backs a little earlier), as in real football; young players lose form, not rating.

## Spot checks

One checker pass re-measured 11 claims from the raw files with its own scripts, a different way from the first scripts where one existed. 9 held; 2 needed a correction, made above.

- Held: NFL 31.2 points and 5.2% 50-point games (from the final scores of 13,728 games, not the stat rows); 124 pros with 2+ straight full seasons on reserve, Jason Ridgeway's four; walk-ons 2.3% of 4-stars and 0.2% of 5-stars (read from the stars in the signing note itself); 7-23 tags every year; the comp-pick split (from the pick list); no FBS non-quarterback with 250+ passes (8 rows with no listed position are quarterbacks by first position); the top three title winners (also from the programs' season flags); NFL gunslinger sd 1.10 over 113 QBs and bounce 13.02 over 144 backs; head coach freedom FBS 6.0 and NFL 5.8; roster median 82 and 1.96 snappers.
- Corrected: record-breaking rushing seasons are 82 in 41 of 50 years (the first count gave the years); Comeback Player specialists 16 snappers, 8 punters, 3 kickers by name match (first count 17 and 4).

## Doubts

- Season correlations mix tendencies with ratings. The wrong-sign pairs (gunslinger and interceptions, holding and sacks) are not proof the engine ignores them: with the narrow spread (finding 2) there is little habit left to see, and the match checks prove each effect with ratings held fixed.
- Memory was measured with the trial's game log on; the game itself does not keep it.
- The slow backs' fall from 13.2 to 8.4 mixes coaching with the aging rule (lost speed takes the bounce with it).
- 63 pros rated 85+ retired unsigned (17 long snappers, 7 receivers, 7 tight ends): not traced; it may be the specialists' rating scale.
- Real counts marked "from memory, to verify" (the comp pick split, trades) need a source before tuning.
- The calendar keeps every event only for the last five recorded years (`world/clock.rs:548`), so 2041-2045 show about 8,200 events a year against about 1,900 before: the export window, not a change in the world.
- The programs' season flags count 5 FCS-era titles of programs now in FBS; this report and the Debug Lab use the title news (50 FBS titles).

## Speed

The profile (`AFM_PROFILE=1`, `stderr.txt`) is in finding 10. The trial tooling: the Debug Lab data took 5 min to build, the player pages 21 s, the checker pass 57 s. The Debug Lab's player pages now load from their own file (`seed-2026-players.json`, 13.6 MB), since an artifact file may hold 16 MB and a 50-year world's data came to 20.8 MB.
