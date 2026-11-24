# Multi-season realism audit: the seed 321 world (2026-11-24)

Read-only. Nothing was fixed and no world was built. Thread "Audit multi-season realism".

Each root cause is the likely one from the docs and a grep of the code. A patch thread measures it before it changes anything.

## Cain's answer (2026-11-24)

"agreed", to the list as proposed:

- **Yes to items 1 to 22, 24 and 25.** No to item 23 (eras in history: history only).
- **Build order:** FBS rushing (1) and freak defenders (3) first, measured on the world's own rosters, since awards (5) and top receivers (8) move with them. Frozen programs (2) as its own design thread. NIL spread (14) after research on the real split by sport.
- **The future run (25) now:** this world played on 10 seasons with the log, about 4 to 5 runner hours plus 40 minutes for its book.
- Each item is one lane in its own fresh thread. The causes and any sizes stay proposed, not confirmed, until its thread has measured them.

## What was read

- The world: seed 321, 30 seasons of history (1996-2025), stopped on 1 January 2026, built 2026-11-22 on main `094635f` (after every scheme-fit and engine-gaps lane). Release `world-2026-seed321-jan2026-094635f` on the worlds page.
- Its book (the World Reader's SQLite file, 7.8 GB): 330 checks against real (122 near, 53 off, 25 far off, 107 with no data because the world has no game log, 23 with no real figure), plus my own queries over every program season, every player season in the stat book, every retired pro's year-by-year history, the last two seasons' games, the money books and the recruiting log.
- Real figures: `data/real-world/` (named with each number). Where I had no sourced figure it says "from memory".

## What this world can't answer

- **Anything after 2026.** The world stops on 1 January 2026. So NIL growth under the House settlement, revenue sharing, the 105 roster limit and rating drift over future seasons are not in it. Collectives carry one year of House-era money and a flat trend.
- **Year-by-year player ratings.** The world log was off. Rating drift is read from program strength by season, NFL starters by season and the careers of kept players, not from every player's yearly row.
- **Game-by-game checks before 2024.** Only the last two seasons' games are stored. Earlier seasons are team and player totals.
- **What a run would cost:** playing this world on 10 seasons on the match engine with the world log on is about 4 to 5 hours of one GitHub runner, plus about 40 minutes for its book (my estimate from the rebuild's 17 minutes a season). It needs Cain's yes first. It would answer NIL and collective growth, the House cap, prestige drift and future rating drift.

## What looks right

These need no patch.

- **FBS games.** 28.4 points a team game (real 27.0 to 27.6), completion 59.5% (60.6%), 7.13 yards an attempt (7.29), 68.2 plays (66.6). Points sd 13.5 (14.0), one-score games 33.5% (36.7%), wins by 28+ 19.7% (21.1%). Win shares miss the Pythagorean line by 0.086 (0.080).
- **No rating drift in college.** FBS program strength averages 61.6 in 1996-99 and 61.9 in 2025, with the same spread (sd 4.2 then, 4.0 now). FCS, Division II and III are as flat.
- **No rating drift in the NFL after the warm-up.** NFL starters (8+ starts) average 80.3 to 80.9 overall in every three-year span from 2004 to 2025. The first years of history start lower (77.6 in 1996) and settle by 2004; that is the hidden history warming up.
- **Development curves.** Retired pros with four or more NFL seasons peak at 28.0 (QB), 27.2 (OL), 26.7 (LB), 26.5 (DL), 26.0 (WR, TE, DB) and 24.6 (RB). Backs fall 1.4 to 2.8 points a year from 28, linemen 0.2 to 1.5 from 30. Growth per year is 5.0 at 18, 2.4 at 21, 1.0 at 23. Starters' ages by position sit within a few points of real (CBs 30 or older 14.5%, real 12.5%).
- **The title count.** 12 different FBS champions in 29 seasons, the most by one school 7 (Georgia), 6 in any 15 years (real 5). No champion from outside the power conferences (real 0).
- **Coaches.** FBS head coach tenure median 4 seasons (real 4), 17.1% of FBS programs with a new head coach a year (17.6%), NFL 7.5 changes a year (6.5).
- **NFL money.** The cap matches every year, league revenue $23.2B in 2024 (real $23.0B), club value $5.84B ($5.9B), top QB deal 21.9% of the cap (21.5%), dead money 8.3% (about 7.5%).
- **College money.** SEC and Big Ten revenue $167M a school (about $200M), Group of 5 $48M ($55M), FBS crowds 38,400 (41,000).
- **Recruiting and the portal.** 2,944 FBS signees a year (real 2,913), 275 four-stars (301) and 41 five-stars (46) a class, FBS portal entrants 2,656 in 2021-22 (2,724) with 62.4% finding a school (62.8%), in-state share 38% (37%).

## The ranked list

Impact is for the player's seat: high (he sees it every season), medium (he sees it in the record books or the market), low (history or a small number).

### High

**1. FBS backs gain 0.7 more a carry in the world than real.**
- World: 205.8 rushing yards a team game and 5.79 a carry (real 171.8 and 5.12, cfbfastR 2022-24). Regular backs (100+ carries, 2021-24): 5.7 a carry at the median, 4.6 at the 10th percentile (real 4.9 and 3.5). 90 thousand-yard backs a year (real 45 to 51). The rushing leader averages 2,272 yards (real 1,809) and 1.9 backs a year pass 2,000 (real about one every two years).
- Likely cause: the engine is tuned on the synthetic FBS test league, where a carry gains 4.76 to 4.82 (`docs/match-engine.md`; `game/synthetic.rs`, `typical_starter`). The world's rosters play 1.0 above that. `GAPS.md` already says "The typical starter isn't the world's" for the NFL; the same holds at FBS. `match_check` can play a world's college rosters (`AFM_ROSTERS`, `tools/rosters/export_rosters.py`), but the engine lanes measured on the test league and on a world's NFL clubs.
- Touches: the match engine, the stat book and records, awards, draft stock for backs, play calling's values (`data/call_values.csv` is built on the typical team).
- Status: new at this size. Trial 7's finding 2 (college stat lines) is parked.

**2. Program fortunes are frozen for 30 years.**
- World: an FBS program's strength correlates 0.90 with its own strength a year later, 0.79 ten years later and 0.76 twenty-five years later. Win share: 0.60 at one year (real 0.536), 0.37 at ten, 0.30 at twenty-five. Texas went 358-41 (.896) and Georgia 350-56 (.859) over 30 seasons (real best about .83, Ohio State, from memory). Ten programs with 28+ FBS seasons sit at .300 or under (real: one or two, from memory). Michigan State had no winning season in all 30 years, Kansas none in 28, Kansas State none in 25. 1.13 unbeaten FBS teams a year (real 0.6), 69% of champions unbeaten (52%), a 65-game winning streak (real 47).
- Likely cause: the one the dynasty diagnosis named on 2026-10-28 (`afm/dynasty-layers/diagnosis.md`): talent follows the program, not the coach. A recruit's fixed pulls (home, brand, `FIVE_STAR_PULL`, `utility_fit` in `world/recruiting.rs`) outweigh the staff (`FEEL_SD` 0.07 against a coach's 0.06). Prestige is 39% parts that never move (tradition, fans, facilities in `world/program.rs`, `prestige_of`). Titles themselves are spread, so the count hid it.
- Touches: recruiting, the portal, prestige, coach hiring and firing, schedules and odds, the campaign's goals.
- Status: known (trial 7 finding 3 parked, trial 10 finding 1 waiting). The home fade (`HOME_FADE_FLOOR`) and lane 5 were built since; this is the first read of the whole 30 years on the finished engine.

**3. A few defenders a year post impossible seasons.**
- World (FBS, 2010-24): the sack leader averages 29.9 (real leaders 14 to 17; the record is 24). 3.4 players a year reach 20 sacks and one every three years reaches 30. One defensive tackle had 84.5 sacks in 12 games, after 67.5 as a freshman. The tackles-for-loss leader averages 52.7 (record 32). The tackle leader averages 196 (real 150 to 168), and 15.9 players a year pass 150. The 99th percentile is fine for sacks (10, real 10); it is the top handful.
- Likely cause: the best linemen's edge over ordinary FBS blockers turns into wins too steeply at the very top (the top 1% get 5.2 sacks per 100 rushes and the top three 9.1; the best real seasons are about 3.5 to 4, my estimate from 17 sacks on about 450 rushes). The help rule exists but doesn't bring him back (`game/calling.rs`, `help_on`, `HELP_GAP`; `game/scrimmage.rs`, `slide_to`, `CHIP_W`): one slide or one chip, and the passer doesn't throw sooner. Edges also don't rotate by plan (`GAPS.md`, "Edges' third and fourth men still play only on fatigue"). Tackles not traced.
- Touches: the match engine, records, awards (a defender's line is worth `6 × sacks` to voters, `world/college.rs`, `award_stats`), draft stock.
- Status: trial 10 finding 3 (high school, waiting) is the same at FBS.

**4. Quarterbacks go first overall less than half as often as real.**
- World: 31% of first picks are quarterbacks (real 73.1%), 11.3% of top-10 picks (19.1%). Several drafts have no first-round quarterback.
- Likely cause: a quarterback is 2 to 3 points behind other positions at draft age and grows longest (among pros with six or more seasons: 68.7 at 21 against 70.6 to 71.8 for linemen and backs, then 78 at 31). The board prices a position by today's grade (`world/recruiting.rs`, `draft_value`, `draft_position_value`: market value rises with `(grade - 66) / 24`), so the quarterback premium arrives after the draft.
- Touches: the draft, mocks (the best mock hits 1 first-round pick, real about 10), rookie deals, quarterback careers.
- Status: trial 7 finding 10, parked.

**5. The Heisman and the MVP go to quarterbacks half as often as real.**
- World: 38% of Heisman winners are quarterbacks (real 80%, from memory) and 24% of NFL MVPs (84%). 46% of Defensive Players of the Year come from the front seven (80%). 0.45 quarterbacks in the jersey top 10 (4).
- Likely cause: the voters' weights are placeholders (`world/college.rs`, `heisman_kind`: QB 1.0, RB 0.84, defenders 0.75, "proposed, not confirmed"), and they read box scores that items 1 and 3 inflate for backs and defenders.
- Touches: awards, fame and NIL, the Hall of Fame, storylines.
- Status: trial 7 finding 9, parked. Measure again after 1 and 3.

### Medium

**6. The lower divisions score 4 to 6 points too many and have too few blowouts.**
- World: FCS 29.2 points a team game (real 26.3), Division II 31.2 (25.9), Division III 31.5 (27.2), junior college 32.5 on 6.8 yards a carry. Wins by 28 or more: FCS 12% (real 21%), Division II 11% (28%), Division III 10% (37%). Average margin in Division III 13.4 (real 23.9).
- Likely cause: two things. The rushing excess of item 1 grows as the level falls. And teams inside a lower division are too alike: Division III strength runs 46 to 57, where real Division III holds everything from Mount Union to winless programs.
- Touches: the match engine, the small-college world, late-bloomer stat footprints the draft reads.
- Status: new.

**7. High school scoring is still 5.5 points short and its record seasons are off.**
- World: 20.5 points a team game (real about 26, from memory; 15.5 at trial 10). Best sack season 82 (real about 40), best receiving season 1,601 yards (real about 2,500), most interceptions 9 (12).
- Likely cause: not traced. Trial 10 findings 2 to 5 are still waiting for a yes or no.
- Touches: high school games, recruits' stat lines, the high school seat.
- Status: known, waiting.

**8. The best FBS receivers draw too little.**
- World: 17 thousand-yard receivers a year (real 28 to 33). Among receivers with 25+ catches the 90th percentile is 834 yards (real 922) and the 99th 1,183 (1,353).
- Likely cause: the passer spreads the ball (`GAPS.md`: "The passer throws to his second read more than his first", "Fast receivers draw too few deep targets").
- Touches: the match engine, receivers' draft stock and awards.
- Status: the two gaps are listed; the world count is new.

**9. Group of 5 head coaches are paid 2.5 to 4 times real.**
- World: MAC head coaches average $3.86M, Conference USA $3.73M, the Sun Belt $3.55M, the Mountain West $4.33M, the American $5.10M (real about $0.8M, $0.9M, $1.0M, $1.6M and $2.3M, USA Today's database from memory). The SEC ($7.15M) and Big Ten ($6.65M) are near real.
- Likely cause: the job's line is `lo × (hi / lo) ^ prestige` with $0.6M to $12M at FBS (`world/staff.rs`, `salary`), so a prestige of 0.5 already pays $2.7M, and item 11 pushes every prestige up.
- Touches: coach contracts and buyouts, college budgets (a Group of 5 school has $48M in all), the carousel (why leave for a small raise).
- Status: new.

**10. Prestige drifts up and bunches.**
- World: FBS prestige averages 0.51 in 1995 and 0.60 in 2024. The spread shrinks from 0.19 to 0.14 and the lowest rises from 0.1 to 0.3.
- Likely cause: prestige reads booster money in plain dollars, `sqrt(boosters / $25M)` (`world/program.rs`, `prestige_of`), and money grows with every era.
- Touches: recruiting reach, coach pay and contract length, searches, the footprint. Another 20 seasons of the same drift would crowd the top.
- Status: new.

**11. NFL champions repeat too often.**
- World: 6 back-to-back champions in 28 seasons with two three-peats (real 3 and none). Win share carried to the next season 0.39 (real 0.329).
- Likely cause: not traced. Clubs keep too few of their own free agents and good veterans go unsigned (both in `GAPS.md`), so the same strong rosters stay strong by other means; the draft and the cap don't pull them back.
- Touches: the NFL seat, the cap, free agency, the draft order's worth.
- Status: new as a pattern.

**12. General managers are almost never replaced.**
- World: the average GM has held his job 13.9 years (real 4 to 6, from memory).
- Likely cause: few callers of `replace_gm` (`world/pro/office.rs`); owners fire coaches, rarely the GM.
- Touches: front offices, the NFL seat's job security, item 11.
- Status: new.

**13. Too few undrafted players in the NFL.**
- World: 27.9% of 53-man rosters went undrafted (real 40.8%, nflverse), 351 undrafted signings a year (about 550), 312 unrestricted free agents a year (about 550).
- Likely cause: not traced. Drafted players last too long (round-one careers 11 years at the median, real 9), so fewer places open.
- Touches: the undrafted rush, free agency, pro scouting, careers.
- Status: new.

**14. NIL is spread too evenly across a power roster.**
- World: 24% of Power 4 players earn under $10,000 a year (real 66.5%, Opendorse 2025) and 44% of SEC and Big Ten players earn $100,000 or more (real about 10%). A power program pays $27M to $31M in all; the Power 4 total of $1.94B matches Opendorse's $1.9B for 2025, but that figure is every sport.
- Likely cause: not traced; the paid set and its floors (`world/ncaa.rs`, `nil_market`; `world/program.rs`, `third_party`).
- Touches: NIL, recruiting pulls, the portal, your budget.
- Status: researched 2026-11-24, `docs/research/nil-split-by-sport.md` (the real figures here are 2024-25, before revenue sharing; the narrower gap is the low end of the paid set; proposals wait for Cain).

**15. NFL games missed read at a quarter of real.**
- World: 0.28 to 0.34 games missed a player-season (real 1.2).
- Likely cause: a season row leaves out games on injured reserve, where the long absences are (`docs/reference/how-it-works/injuries.md`; `world/pro/games.rs`, `record`). So part of this is the count, not the injuries. Measure both before changing either.
- Touches: injuries, careers, depth, the medical read.
- Status: known as a count.

**16. NFL passing and scoring are still hot.**
- World: 24.9 points a team game in 2025 (real 22.5), 7.8 yards an attempt (7.05), QB EPA 0.147 a dropback (0.070), 11.8 passers a year over 4,000 yards (9.4), time to throw 2.28 s (2.71), pressure rate 21.4% (29.5%).
- Likely cause: in `GAPS.md` ("NFL scoring runs 4 to 7 points over real"). Cain chose on 2026-10-19 to leave NFL passing as it was for now.
- Touches: the match engine, NFL stat lines, contracts by production.
- Status: known. Listed only because every engine lane is now built.

### Low

**17. Stars against the draft.** 5.5% of draft picks were unrated recruits (real 25.2%), 47% were three-stars (35.9%), 72% of five-stars are drafted (63%) and 16% of four-stars (22.1%). Five-stars go to 22.8 programs a class (real 16). Likely cause: the services rate nearly everyone who signs, and the real data has holes for junior college and late players (`world/recruiting.rs`, `star_score`). Status: trial 10 finding 7, waiting.

**18. Kickers play too long.** Drafted kickers last 14 years at the median (real 6). They rise to 84 and stay; nobody loses his job to a bad year. Not traced. Status: new.

**19. The FBS home edge is short, and so are finals by 3 and 7.** Home teams win 54.9% (real 58.5%). 6.7% of games end by 3 (real 9.9%) and 6.2% by 7 (8.6%). Not traced. Status: new.

**20. FBS portal entrants lag the latest real rise.** 3,097 in 2024 (real about 3,700 in 2025, Hudl). Near real for 2021-22. Status: new.

**21. Draft picks come from too many colleges.** 118 colleges with a pick a year (real 97), 8.2% of picks from FCS (real 6%). Status: new.

**22. Too few in-season NFL trades.** 9.2 a year (real 17.6). Trades in all are near (85, real 91). Status: trial 7 finding 8, parked.

**23. History plays one era.** FBS teams throw 29.6 passes a game in 1996-99 and 31.2 in 2020-24, and complete 58.8% then and 60.3% now. Real football changed more. History only. Status: new.

**24. Reader housekeeping (tools).** The known-issues list still shows odd-28 as open, but the code reads the game team now (`world/pro/games.rs`, `record`). "Different FBS champions in 15 years" reads 11.5 because it counts titles of programs that were FCS then. "Median peak age of quarterbacks" reads 24 because it counts every quarterback who ever reached a camp; established ones peak at 28.0. "FBS roster size" reads 1 January, before the trim to 105. The stat book's Division II and III scoring has data in program seasons, but the checks wait for the game log.

**25. The future run.** Play this world on 10 seasons with the log, to read what it can't answer (above). About 4 to 5 runner hours. Needs Cain's yes.

## Systems each high item touches, in short

| Item | Engine | Recruiting and portal | Money | Draft and NFL | Records and awards |
|---|---|---|---|---|---|
| 1 FBS rushing | yes | stat footprints | | backs' stock | yes |
| 2 Frozen programs | | yes | prestige, pay | | streaks |
| 3 Freak defenders | yes | | | linemen's stock | yes |
| 4 Quarterbacks in the draft | | | rookie deals | yes | |
| 5 Awards | | fame, NIL | | | yes |

## Method notes

- Program strength, wins and totals: `programs__seasons` (25,946 rows). Player seasons: `stats__seasons` (1.79 million rows) with `engine_stats` for the columns. Careers: `alumni__player__history` and `people__player__history`. Games: `college__games` and `college_prev__games` (2025 and 2024). Money: `programs` (collectives), `coaches` (contracts), `finance__league`, `market__years`. Flows: `log_history`.
- Real season lines: `college_players.json` (ESPN box scores, 2021-24 means). Real scores: `college_scores.json`, `college_history.json`, `nfl_game_targets.json`, `nfl_history.json`. Real careers: `nfl_careers.json`, `development_injuries.json`. Real NIL and portal: `rates.json`.
- One world is one draw. Title counts and single records swing between seeds; the rates over 3,500 program seasons and 1.79 million player seasons do not.
