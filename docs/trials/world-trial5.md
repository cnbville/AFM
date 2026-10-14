# World trial 5: three 30-year worlds

Three worlds (seeds 2026, 2027), each with 30 years of history (1996-2025) and every college game played on the match engine, about 85,000 college games per world. Each world took about 33 minutes on 4 cores. The engine is the trial 4 engine with no physics change; names were rebuilt before this trial (`data/names/README.md`). The new debug dashboard (AFM Debug Lab) shows all three worlds side by side: (artifact link). Its builder and page are in `tools/dashboard/`.

The full raw output of each world is in the project folder at `afm/trials-30yr/seed-<n>/` (gzipped JSON lines: every game's box score, every player season, play-level situation totals for the league and every FBS team, programs, coaches, alumni, active players, NFL rosters and news; `report.txt` is the usual trial report). Rebuild the dashboard from them without re-running: `python3 tools/dashboard/build_data.py OUT/data <run dirs>`.

Each result says why. "Traced" means read in the code; "inferred" means read from the numbers. Everything proposed here is proposed, not confirmed.

## New findings

### 1. Most FBS teams have no kicker or punter
In the last season, 99-103 of 138 FBS teams list neither a kicker nor a punter; all of FBS has about 23 kickers. The kicking game follows from it:

| | World 2026 | World 2027 | Real |
|---|---|---|---|
| Field goal tries per team game | 0.54 | 0.52 | 1.56 (cfbfastR 2022-24) |
| Two-point tries per team game | 1.77 | 1.81 | rare (no figure in the data folder) |
| Punt gross (yards) | 35.9 | 35.8 | 42.3 |
| Kickoff touchbacks (%) | 4.8 | 4.7 | no college figure in the data folder |
| Red-zone fourth downs punted (%) | 34.6 | 35.5 | near 0 |

- Why (traced): college offers rank recruits by rating, with 8 points off for specialists (`recruiting.rs`), there are no position needs and no walk-ons, so few kickers get a spot. With no kicker on the depth chart the engine's field goal estimate is 0.2 (`flow.rs`, `fg_make_estimate`), so the coach punts or goes for it, and whoever is listed first on the roster kicks off and punts.
- Proposed fix: programs keep position needs (real rosters carry about two kickers, two punters and a long snapper) and fill them through recruiting, preferred walk-ons and tryouts; specialists are judged against specialists.
- This also feeds scoring: fewer field goals and more fourth-down tries and two-point tries.

### 2. Flagships don't hold their own state
Share of each roster from the school's own state, last season of world 2026: LSU 1%, Georgia 10%, Ohio State 10%, Alabama 6%, Notre Dame 2%, while Texas (42%) and Texas A&M (45%) keep Texans because Texas has so many. The median FBS roster is about 8% home-state.
- Why (traced in part): a recruit weighs closeness by miles on a 1,500-mile scale (`recruiting.rs`, `utility`), so a school 300 miles away is nearly as close as the one at home; there is no extra pull toward the home flagship, and staffs offer by rating, not home state first.
- Real check: I have no real roster home-state data in the repo. My memory says flagships run about 40-70% in-state; to verify against CFBD rosters.
- Proposed fix: a home-state pull per recruit (stronger for flagships, family ties and kids who grew up fans), staffs recruiting their own state hardest, and mostly local walk-ons.

### 3. Players play another position all season without being moved
3.3% / 3.8% of FBS seasons with 100 or more passes were thrown by a player listed elsewhere. In world 2026 an interior lineman threw 533 passes and 74 touchdowns for Missouri (1996). Offensive players had 5 or more sacks in a season 25 / 13 times in 30 years (a receiver had 16).
- Why (traced): the depth chart lists a player at another spot when he rates 4 points better there (`roster.rs`, `CROSS_LIST`), but his listed position never follows, and with no position needs some quarterback rooms run thin. Two-way play is allowed when a player beats the starters by the margin.
- Proposed fix: a player who plays another spot for a season is moved there; position needs (item 1) keep rooms full; two-way starters at FBS become a rare coach's choice.

### 4. Almost nobody gets worse
Fewer than 0.3% of player seasons lose more than a rating point at any age (dashboard, Player paths).
- Why (inferred): the yearly step's slumps slow growth but rarely take ability away.
- A design question: real declines after a major injury, weight gain, lost interest or a bad coaching fit.

### 5. Few draft picks come from unrated players
Unrated players are about 8% of picks (real 25%, but the real figure includes walk-ons, JUCO players and name mismatches, so the true gap is smaller); two-stars about 16% (real 6%).
- Why (inferred): no walk-ons (item 1), and stars follow rating closely, so late bloomers still get a star.

## Known problems, now measured across three worlds

### 6. The dynasty is in every world
- World 2026: Ole Miss 9, Ohio State 8, Miami (FL) 5, SMU 2.
- World 2027: Ohio State 13, Alabama 5, Michigan State 4, Tennessee 2.

| | World 2026 | World 2027 | Real |
|---|---|---|---|
| Most titles in any 15 years | 8 | 8 | 4-6 |
| Fewest distinct champions in a 15-year window | 5 | 4 | 7-9 |
| Best team's average margin | 53.9 | 52.0 | +22 to +39 |
| Win rate carried into next season (corr.) | 0.62 | 0.60 | 0.54 |
| Unbeaten FBS teams a year | 0.77 | 0.70 | 0.6 |
| FBS programs with no winning season in 15 years | 13 | 10 | 0 |
| Prestige rank correlation with five years before | 0.943 | 0.933 | no figure |

- Why: the dynasty re-trace on main (`docs/trials/world-trial4-proposals.md`, last section): a whole-lineup edge pays about twice real spreads, some positions are overvalued, schemes add a fixed edge and nobody eases off. Three seeds now show it is the engine, not one seed's luck; the re-trace asked for exactly this multi-seed check.
- Indiana had no winning season in 30 years in every world (real longest Power 4 run without one, 2001-2025: Kansas 14).

### 7. Game shape and stat lines
| | World 2026 | World 2027 | Real |
|---|---|---|---|
| Points per team game | 36.2 | 36.2 | 27.8 |
| Plays per team game | 74.4 | 74.6 | 66.6 |
| Pass rate (%) | 59.5 | 59.7 | 49.6 |
| Completion (%) | 55.1 | 55.2 | 60.6 |
| Yards per carry | 5.75 | 5.71 | 5.12 |
| Home wins (%) | 51.0 | 51.4 | 58.4 |
| Games decided by 28+ (%) | 32.9 | 31.8 | 21.1 |
| One-score games (%) | 27.7 | 27.3 | 36.7 |
| Game luck (sd, points) | 22.1 | 21.9 | 13 |
| Median yearly passing yards leader | 6,448 | 6,436 | 3,932-4,191 |
| Median yearly rushing yards leader | 3,229 | 3,056 | 1,479-2,358 |
| 4,000-yard passers a year | 64 | 67 | no figure in the data folder |

- Why: the same lineup-margin cause, every world game played at a neutral site, no garbage time, and the NFL's calling tables at every level (open trial 4 topics 1-3 and items 3, 4 and 6 of the re-trace list).
- The single-season records are far past real (for example 120 passing TDs in world 2026; the real FBS record is 62, written from memory). The Anomalies tab lists every one.

## What looks right
| | World 2026 | World 2027 | Real |
|---|---|---|---|
| Five-stars drafted (%) | 68.3 | 68.8 | 62.2 |
| Four-stars drafted (%) | 22.9 | 23.4 | 24.4 |
| Three-stars drafted (%) | 4.8 | 4.8 | 6.0 |
| FCS players among draft picks (%) | 6.5 | 6.0 | about 6 |
| Black FBS head coaches, last season (%) | 12.3 | 12.3 | 11.9 (CBS 2024) |
| Third-down conversion (%) | 40.2 | 40.2 | 40.3 |
| Shared full names, 22,843 college players (%) | 5.99 | 6.11 | 4.4 (CFBD 2024) |

- Names: trial 3 had 46.5% of its 779,551 players sharing a full name (a much bigger pool, so not directly comparable); on a real-size college pool it is now about 6%, against 4.4% on real 2024 rosters. The rest of the gap is probably surnames drawn from one national table per ethnicity (inferred); regional surname data would close it.
- Black FBS head coaches reached the real share in these worlds (trial 4 had 7%).

## Doubts for Cain (my picks tonight, proposed, not confirmed)
1. **Names.** The ethnicity shares and the name model are mine: first names from SSA births by decade, ethnicity from Tzioumis (via predictrace) where known and a letter-pattern model otherwise; surnames from the 2000/2010 Census. Suffixes (Jr., II, III), hyphenated surnames and initials (like "CJ") are added at rates I picked (`data/names/README.md`).
2. **The dashboard's health thresholds** (near, off, far off) are my proposal.
3. **Situation buckets** (down and distance, field zones, score states, time-to-throw bands) are mine.
4. **Real references written from memory**: the FBS single-season records and "real flagships keep 40-70% in-state" are unchecked. Every other real figure names its source in the dashboard (`real.json`).
5. **The raw export keeps** every college player and every high school player with three or more stars; two-star and unrated high school players are left out to keep the files small. College and NFL coaches are kept in full.
6. **I fixed two dashboard reference bugs** found in the check: star draft rates now divide by the signing classes (the world keeps only drafted players and four- and five-stars), and the real penalty figure was half what it should be.
7. **Nothing in the engine or the world was changed.** Every finding above is for Cain's call.
