# World trial 9: one 50-year world, the bug check before section 8

Seed 2026: 30 recorded years of history (1996-2025, after the spin-up from 1984), then 20 seasons into the future (2026-2045). Every college, high school and NFL game ran on the match engine. Built on commit `be00ee5` (build 9: the section 5 leftovers, sections 6 and 7 on the world side, fix list S6.1-S6.3 and the code bug check), the code merged as PR #59.

Cain asked for it on 2026-10-19: "full build, thena bug check -> 50 year world trial -> fix found bugs -> build the fixes -> then get ready for building the game." It ran overnight with no questions asked. Plain bugs it found are fixed (below, PR #60), checked by the tests only, with no second world run. Tuning and realism findings (13) wait for a yes or no.

- Time: the history built in 3,634 s (61 min; trial 8 6,132 s); the 20 future seasons took 102-110 s each. The whole run took 5,748 s (96 min) plus the exports. Peak memory 7.62 GB with the trial's game log on (trial 8 7.77 GB); 5.93 GB after the history, then about 60 MB more each future season.
- The first start crashed in 1995: a club that joined the league had no scouting department yet (`scouts.rs:442`). Fixed before the trial (commit `b3e8c9f`, with a 1990-2010 world test through the franchise changes), and the trial restarted from scratch.
- Raw output: project folder `afm/trials-9/seed-2026/` (report, cards, raw export gzipped, the profile in `stderr.txt`, memory samples in `rss.txt`).
- Debug Lab: https://claude.ai/artifact/3SAzca1V9NBFKkW5pAPTSt (version 9). Player pages (148 players): `docs/trials/artifacts/world-trial9-player-pages.html`, also at https://claude.ai/artifact/XmJk7Vjkv3p6nZ2nPYWMY8.
- One checker pass re-measured 10 claims from the raw files with its own scripts (see "Spot checks").

Numbers are 1996-2045 unless stated. "Traced" means read in the code; "inferred" means not confirmed in the code. Real NFL figures are nflverse 2021-25; real FBS figures 2022-24.

## The trial 8 fixes, measured

| | Fix | Trial 8 | Trial 9 | Verdict |
|---|---|---|---|---|
| 1 | Injured reserve ends with the season | 124 pros on reserve 2+ straight full seasons; 16-27 full-season reserve players a year | 0 pros; 1-3 a year | works |
| 2 | Tendencies spread again | NFL sd 1.1-2.8, FBS 1.5-2.9 (agreed 3.2 and 3.8) | NFL 0.96-2.37, FBS 1.40-3.20 | did not work (finding 3) |
| 3 | Tendency averages on the middle | owners' means NFL 8.0-13.0, FBS 7.5-13.5 | NFL 10.6-12.3, FBS 10.6-12.9 | partly: the far ends closed in, most sit 1-2 points above 10.5 (finding 3) |
| 4 | Coach freedom around the middle | head coaches FBS 6.0, NFL 5.8 | FBS 10.4, NFL 10.5, high school 10.3 (sd 2.7-3.3) | works |
| 5 | FBS walk-on places filled from home | roster median 82, in-state share 25% | median 80 (68-91), in-state 25% (real 37%) | did not work (finding 4) |
| 6 | NFL pace and scoring | 31.2 points, 69.5 plays a team game | 30.8 points, 67.9 plays (real 22.5, 61.9) | barely moved; kept for now by Cain's answer |
| 7 | FBS runs | 6.18 yards a carry | 4.85 (real 5.12); 0-4 2,000-yard rushers a year (real 0-2), 4 seasons past the 2,628 record (trial 8 82) | works, now a little short (finding 2) |
| 8 | Comp picks | 12-27 formula, 5-20 supplemental (2021-45) | future 15-26 formula (mean 21), 6-17 supplemental (mean 11) | better |
| 9 | One snapper on scholarship | 1.70 on scholarship a team | 0.71 (1.94 a roster) | works |
| 10 | Summer signing speed | history 6,132 s, future seasons 154-171 s | 3,634 s and 102-110 s | works: 41% faster |
| 11 | Memory reported | not measured | after the history and every season (above) | works |
| | Names' race shares | Dashawn 61% White | Dashawn, Antwan, Jaquan 100% Black among the players who carry them; Connor 71% White | works |
| | Raw export of pros' detail | missing | in `pros.jsonl` | works |

## Fix list S6.1-S6.3 (built with section 6), measured

- **S6.1 run fits.** FBS runs gain 4.85 yards a carry (real 5.12), NFL runs 3.99 (real 4.50; trial 8 5.04). The run game went from too easy to a little too hard at both levels (finding 2).
- **S6.2 plays ending out of bounds.** Not read in the world export. NFL teams run 67.9 plays a game (trial 8 69.5, real 61.9).
- **S6.3 bonus money paid once.** Seasons paying one player over 30% of the cap, 2026-2045: 41 (trial 8 144); the most 50% (trial 8 139%). The biggest deals now are 17-23% of the cap a year (real top five quarterbacks 23.4%). Still high: first-year cash (finding 6).

## Sections 5, 6 and 7 in the world

Trial 9 is the first world with the section 5 leftovers and the world side of sections 6 and 7.

- **Scouting departments.** 32 directors, 96 national and 292 area scouts now (9.1 area scouts a club, 8-10); judging mean 9.0 (sd 2.9). 295,000-363,000 scout looks a year. Each bias (size, small schools, testing, tape, conference, region, speed) is held by 56-67 scouts.
- **The draft.** 14-35 draft-day trades a year (real about 20-30, from memory, to verify). Picks a year 242-260. Each pick's note names the club's own grade and the public rank.
- **The undrafted rush.** 152-246 signed a year (history mean 201, future 183). Trial 8's pipeline brought 450-706 undrafted players into the league a year; bug 4 below.
- **The room.** NFL clubs name 5-6 captains, FBS programs 3.9. FBS team moods -0.11 (South Carolina) to +0.32 (Notre Dame); every NFL club sits between -0.18 and -0.06 (finding 9).
- **Moods.** College mean +0.06 (2.2% low, 9.4% high); NFL mean -0.10 (18.1% low, 0.3% high). Main reasons in the NFL: settled 1,524, role 347, money 257, happy with his role 10.
- **Off the field.** 23-63 NFL trouble cases a year and 4-34 suspensions (history mean 17, future 27); 72 pros hold a grudge against a former team (strongest 7).
- **Power.** 20 clubs give the GM the final say, 12 the head coach.
- **Support staff.** NFL heads 11.0 of 20, FBS 12.1, FCS 9.3, smaller colleges about 8.
- **Practice squad.** Elevations 0-13 a year from 2020 (finding 11); poaching 0 in 25 years (finding 5); injury settlements 0-1 a year, 12 in 50 years (bug 7).
- **Contracts.** 5-15 franchise tags a year, holdouts 0-14 a year. Cap use in 2046: 74% to 108% (median 92%).
- **Trades.** 3-18 player trades a year plus 2-13 at the deadline (finding 10).

## Parked items, measured

These were parked until sections 6 and 7. Their world side is now built, so each is in the findings as a yes or no.

- **Dynasties (finding 1).** 50 FBS titles: Texas 18, Ohio State 14, Georgia 6. The 20 future titles: Texas 12, Ohio State 7, LSU 1 (three champions); Texas won 10 in one 15-year window (bar 6). 12 of the 20 future champions went 17-0. Texas's prestige rose from 0.70 to 1.00.
- **Round 1 positions (finding 8).** Receivers 20.7% (real 13.4%), edge rushers 20.4% (13.5%), quarterbacks 11.8% (10.5%), tackles 6.2% (12.1%), corners 8.4% (11.9%).
- **5-star draft rate (finding 8).** 84.6% (1,228 of 1,452; real 62%).
- **Trade volume (finding 10).** Above.

Still parked (no answer needed): awards. Comeback Player of the Year went to long snappers 8 times and punters 7; MVP to a quarterback 35 times.

## Tendencies in the world

Owners of each habit, at the end of 2045. Agreed spreads (sd): high school 4.8, FBS 3.8, NFL 3.2.

| Tendency | HS mean / sd | FBS mean / sd | NFL mean / sd |
|---|---|---|---|
| Bounces runs | 10.8 / 3.4 | 12.9 / 2.0 | 12.1 / 1.6 |
| Scrambles | 10.7 / 3.3 | 12.7 / 1.9 | 12.0 / 2.1 |
| Gunslinger | 11.1 / 3.3 | 11.4 / 1.5 | 11.7 / 1.0 |
| Holds for the big play | 10.4 / 3.3 | 11.4 / 1.4 | 11.4 / 1.1 |
| Patient or downhill | 10.9 / 3.7 | 10.6 / 2.4 | 10.6 / 2.2 |
| Returner risk | 10.8 / 3.4 | 12.3 / 1.6 | 11.8 / 1.5 |
| Rushes upfield | 10.1 / 3.6 | 11.9 / 1.9 | 12.0 / 1.3 |
| Times the snap | 10.8 / 3.4 | 12.0 / 1.8 | 12.3 / 1.8 |
| Gets his hands up | 10.9 / 4.0 | 11.2 / 3.1 | 10.9 / 2.2 |
| Goes for the strip | 10.6 / 3.4 | 11.6 / 1.5 | 11.8 / 1.2 |
| Big hit | 10.7 / 3.3 | 12.2 / 1.8 | 12.2 / 1.6 |
| Jumps routes | 10.8 / 3.4 | 12.1 / 1.6 | 12.3 / 1.5 |
| Flies downhill | 10.7 / 3.3 | 11.9 / 1.5 | 11.9 / 1.4 |
| Reaches the ball out | 10.6 / 3.4 | 11.4 / 1.9 | 11.8 / 1.7 |
| Contact willingness | 9.9 / 3.9 | 10.7 / 3.2 | 11.0 / 2.4 |

The future barely moved them: at most 0.8 points at any level over 20 years. The season-line checks of trial 8 (habit against stats) were not repeated.

## Bugs found and fixed (PR #60)

Each was traced in the code, fixed, and given a test (the 3-year test world, or a unit test). No world was run after the fixes.

1. **5-stars signed with a junior college.** 285 5-stars signed with a junior college, nearly all with Navarro; 11-20 a class from 2027 (0-8 before). Greyson Green (QB, 4.00 core GPA, 32 offers led by Alabama and Georgia) signed with Navarro and still went first overall. Why (traced): once an FBS class was full, its staff stopped offering anyone, even a 5-star whose family made him wait for February; the junior college recruiting at the end took them. Fix: an FBS or FCS staff may go two over its class (and two over its plan at the position) for a 5-star (`recruiting.rs`, `OVER_CLASS`). Test: no junior college signs more than two 5-stars in one class.
2. **The combine every year.** 454 pros carry two or more "at the combine" notes; 2,473 combine notes were written after a player turned pro. Why (traced): the invite flag was never cleared (`combine_invites`), so anyone invited once was measured at every later combine. Fix: cleared each year, and only declared prospects test. Test: nobody has two combine notes.
3. **A future first for a late-round move.** 11 of 242 trade-ups (in the years the event log keeps) moved to a pick past 64 for a next-year round 1 pick. Why (traced): the future-pick search took the earliest round that covered the value. Fix: the latest round that covers it (`scouts.rs`, `trade_on_clock`). Test: no trade to a pick past 64 gives a round 1 pick.
4. **The undrafted rush stopped after one round of calls.** 152-246 undrafted rookies signed a year, against 450-706 in trial 8 (real: each club signs about 15-20, from memory, to verify). Why (traced): each club called its best marked players once; a player who chose another club left that place empty for good. Fix: the rush runs in rounds; a club with room keeps calling down its list until nobody signs (`scouts.rs`, `rush`). Test: at least 300 sign in a year of the test world.
5. **An extension weeks after signing, and a tag replaced by one year.** 445 pros signed in the spring market and were extended by the same team at the same age (93 more after re-signing); 31 tags were "replaced" by a 1-year deal. Why (traced): extension talks skipped only deals signed in a later league year, not the same one, and a tag's replacement could be one year. Fix: no talks in the league year a deal was signed (tags excepted), and a tag is only replaced by two years or more (`market.rs`, `extension_talks`). Tests for both.
6. **NFL pay read against the wrong market.** A pro's mood read his pay against a straight line drawn from the top five's pay at his position, so most players looked underpaid (money was the main reason for 257 NFL players; NFL mood mean -0.10). Fix: his worth is the market's own price for his grade and service (`people.rs`, `pay_and_worth`). Test: the middle NFL player of the test world is paid 0.8-1.25 times that price (the old line's read is printed beside it).
7. **Injury settlements almost never happened.** 12 in 50 years (real: a common cutdown move for hurt young players). Why (traced): every camp injury of 6+ weeks went straight to injured reserve before cutdown, so cutdown never saw a hurt player to settle with. Fix: only vested veterans (4+ accrued seasons) go on reserve in camp; the rest reach cutdown, where a club keeps a hurt player it wants on reserve and settles with the others (`games.rs`, `pro/mod.rs`). Tests: a unit test hurts a whole club before the cutdown (the kept go on reserve, the cut are settled with, none join the practice squad), and in the test world every player hurt in camp and cut is settled with. Settlements will stay rare until camp injuries are as common as real ones (finding 13).
8. **"The Jaguars's camp."** 17,569 notes read "the Jaguars's roster", "camp" or "practice squad". Fix: a possessive for team names ("the Jaguars' camp").
9. **"Alabama fire head coach."** 896 coach-contract news lines used a plural verb for a school ("Alabama extend"). Fix: "fires", "hires", "lets", "gives", "extends".
10. **Python cache files committed.** Removed and ignored.
11. **One deal paid 52.6% of the cap in its first year.** Found by the test world in CI after the fixes above (trial 9's own highest was 50%): Mckenzie Tupou Jr., five years at $62.03M a year with a $111.5M signing bonus, took $146.7M in 2025 against a $279.2M cap, past the S6.3 test's line of half the cap. Real highs are about 35-38% (Rodgers 2018, Prescott 2024; from memory, to verify). Why (traced): the signing bonus was 65% of the guarantee with no limit on year one. Fix: year one's bonus and pay stay at or under 38% of the cap, for every signing; the rest of the bonus becomes guaranteed base in the later years, so the money and the guarantee don't change (`money.rs`, `limit_first_year`). Tests: a unit test on that deal, and the test world's S6.3 check.

## Findings (yes or no each)

1. **Dynasties** (parked finding 3, now measurable). Texas 12 and Ohio State 7 of the 20 future titles; Texas 10 in one 15-year window; 12 future champions went 17-0. Real: no program has won more than 6 in 15 years since 1990 (Alabama 2009-2020). Why: not traced. Fix: trace what keeps Texas on top (recruiting, the portal, NIL, coaching), then add the real levelers it bypasses.
2. **Runs are now a little too hard** (S6.1 overshot). FBS 4.85 yards a carry (real 5.12), NFL 3.99 (real 4.50). Fix: retune the run fits toward 5.1 and 4.5 with `match_check` on the world's rosters.
3. **Tendencies still too alike** (fix 2 fell short). NFL sd 0.96-2.37 (agreed 3.2), FBS 1.40-3.20 (3.8); most averages sit 1-2 points above 10.5. Why (traced): inside the 3-point band only a small drift moves a habit, and outside it everything is pulled to the band's edge, so the spread can't get much past the band itself. Fix: widen the band to 5 points and let the style and scheme pull stop at it too; check with `tendency_check`.
4. **FBS rosters still short** (fix 5 fell short). Median 80 (real near 105 since 2025); linebackers 8.4 a roster (12.8), safeties 7.1 (10.5); in-state share 25% (real 37%). Why (partly traced): the final-period walk-on offers go only to local seniors nobody holds, and by then most hold a Division II, III or NAIA offer they keep. Fix: a local senior weighs a Division I walk-on place against what he holds, as real preferred walk-ons often take it over a small-college offer.
5. **Poaching never happens.** 0 practice squad players poached in 25 years. Why (traced): a club looking for a player always finds a better-rated street free agent at the minimum. Fix: clubs weigh a practice squad player's age and fit to their scheme against the street veteran.
6. **First-year cash too high.** 41 seasons since 2026 paid one player over 30% of the cap, up to 50% (Ezra Davis 2045, Derrick Corbin 2040: signing bonuses of $500-700M); restructures stack cap hits up to 37%. Real top first-year cash is about a third of the cap (from memory, to verify). Bug 11 now stops any deal at 38% of the cap in year one, but bonuses are still sized as 65% of the guarantee, so top deals will crowd that line. Fix: size signing bonuses like real top deals (about 25-35% of new money) and limit restructures to what a real club converts.
7. **One-year extensions.** 995 of 4,384 extensions add a single year; some top the market (Beau Ness, EDGE, one year at 21.6% of the cap). Real one-year extensions are rare and small. Fix: an extension adds at least two years, except for players over 32.
8. **Draft by position and 5-stars** (parked, section 7 now built). Round 1: receivers 20.7% (real 13.4%), edge rushers 20.4% (13.5%), tackles 6.2% (12.1%); 84.6% of 5-stars drafted (real 62%). Fix: boards weigh positional value the way real clubs do (tackles and corners up, receivers down), then re-measure the 5-star rate.
9. **NFL players are never happy with their role.** Only 10 of 2,141 NFL players read "happy with role" against 13,168 in college; every NFL team mood is below zero. Why (traced): an NFL starter's role part can't go above zero. Fix: an NFL player playing above what he thinks he's worth gets a lift, as in college.
10. **Few trades.** 3-18 player trades a year plus 2-13 at the deadline (real several dozen, from memory, to verify). Fix: trace why clubs rarely find a match, then measure against a real count.
11. **Few practice squad elevations.** 0-13 a year since 2020 (real: hundreds a season, up to two a team a game). Fix: clubs elevate for injured spots and special teams each week.
12. **Memory grows with the years.** 5.93 GB after the history, 7.07 GB by 2045, with the game log on. Fix: measure a world without the game log and set a budget before section 8 (see `docs/design/section8-readiness.md`).
13. **NFL injuries are too rare.** Only 10-41 NFL players a year spend any time on injured reserve (of about 2,700), and trial 9 has 78 camp injuries of six weeks or more in 50 years. Real: each club puts about 15-20 players on reserve a season, several hundred in all (from memory, to verify). Why: not traced (the engine's injury rate, about 0.3-0.5 a team game in the match checks, or how long injuries last). Fix: measure injuries a game and weeks missed against real NFL injury data, then tune.

Measured only, already answered (no answer needed): NFL scoring 30.8 points a team game (real 22.5) and passing 7.8 net yards a dropback, kept for now; FBS scoring 25.3 (real 27.8), kept for now.

## Slumps

33,158 slumps: 12,417 rating drops and 30,012 form slumps (the Debug Lab's Slumps list shows the 400 biggest).

- Alex Howell (CB, 37, NFL backup, 2025): 65.2 to 57.9 (-7.3). A major injury, 14 games missed, age-37 decline.
- Orlando Pantoja (WR, 29, NFL starter, 2013): 84.6 to 78.0 (-6.6), played at 70.6. A major injury, 12 games missed, and a form slump.
- Christopher Butts (CB, 37, backup, 2012): 82.5 to 75.9 (-6.6). A moderate injury, a form slump and age-37 decline.
- Kaleb Lockett Jr. (RB, 27, rotation, 2022): 73.5 to 67.0 (-6.5). A major injury, 9 games missed, a form slump.
- Joseph Powell (QB, 24, backup, 2020): 71.8 to 67.2 (-4.6), played at 57.3, a form slump and teaching that cost ability (9 of the 400 carry that reason).
- Biggest form slump: Adrain Warfield (QB, 23, NFL backup, 2026) played at 51.4 against a 66.9 rating, in his first year in the league with a moderate injury.
- Below the NFL: Kameron Johnson (WR, 20, FBS backup, 2016), 56.9 to 52.6 (a major injury, 12 games missed); Eli Thomas Jr. (TE, 15, high school starter, 2039), 40.5 to 36.3 (a major injury and teaching that cost ability); Emmanuel Thornton (CB, 20, FCS starter, 2044), 51.8 to 47.6 (a major injury, his body stopped developing).

Why they read right: big drops come with major injuries and age past 30, as in real football; young players mostly lose form, not rating. 371 of the 400 biggest are NFL players, where ratings are highest and aging bites. No slump names a low mood as its reason, though mood now moves development up to 10% (see "Doubts").

## Spot checks

One checker pass re-measured 10 claims from the raw files with its own script (`spot.py`, 50 s), a different way from the first scripts where one existed. All held; two counts differ from the first count by method, as noted.

- NFL 30.8 points a team game and 4.5% of team games at 50+ (from the final scores of 13,948 games).
- 0 pros with two straight seasons of 17+ reserve weeks and no game.
- Head coach freedom FBS 10.4 and NFL 10.5 (from each coach's last job).
- NFL quarterbacks' gunslinger: mean 11.66, sd 0.96 over 124.
- 12 settlement notes in 50 years.
- 454 pros with two or more combine notes.
- 11 of 242 trade-ups to a pick past 64 for a future first.
- 5-stars to junior colleges: 285 by the signing note itself (Navarro), people and alumni; the first count (239) read first colleges only.
- Wrong team possessives: 17,569 in the pros' notes (the first grep, 18,005, also counted the news).
- FBS rushing: 0-4 2,000-yard seasons a year and 4 past the record (from every FBS season line).

## Doubts

- Mood moves development, but the slump labels don't name it, so a slump driven by a bad mood reads as "a form slump".
- Memory was measured with the trial's game log on; the game itself does not keep it.
- Real counts marked "from memory, to verify" (draft-day trades, trades, elevations, first-year cash) need a source before tuning.
- The calendar keeps every event only for the last five recorded years (`world/clock.rs`), so trade-up counts from events cover those years only.
- The bug fixes change the world (who signs where, who reaches the NFL), so trial 10 will not match trial 9 year by year.

## Speed

The profile (`AFM_PROFILE=1`, `stderr.txt`), a future season: college games 38 s, the NFL 17 s, summer signing 14 s, NFL people 10 s, high school 9 s, coaching searches 5 s, signing day 4 s. The trial tooling: the Debug Lab data took 146 s to build, the player pages 8 s, the checker pass 50 s.
