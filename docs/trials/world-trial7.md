# World trial 7: the full build, one 30-year world

Seed 2026, 30 recorded years (1996-2025) after a 12-year spin-up from 1984. Every college, high school and NFL game ran on the match engine. Built on main after PR #43 (section 5, the living NFL).

- Time: the world built in 5,647 s (94 min); with the exports the run took 101m52s wall and 212 min of CPU on 4 cores. Trial 6 took about 41 min a world.
- Raw output: project folder `afm/trials-7/seed-2026/` (report, cards, raw export, the profile in `stderr.txt`).
- Debug Lab: https://claude.ai/artifact/3SAzca1V9NBFKkW5pAPTSt (new version, one world). Player pages (117 players): `docs/trials/artifacts/world-trial7-player-pages.html`, also at https://claude.ai/artifact/CGhQ8ZM8xefZmTUtV8sfoy.
- One checker pass re-measured the 18 claims the findings rest on; its corrections are made in place (see "Spot checks").

Raichel's answer (2026-10-18): fix 4, 5, 6, 7, 11-18 now, with quick checks and no trial (built 2026-10-18; `docs/world-model.md`, "Trial 7 fix-now list"). Parked until the systems that feed them are built: 1, 2, 3, 8, 9, 10 and part D 4.

## Part A of trial 6 (F1-F17), measured

| | Fix | Trial 6 | Trial 7 | Verdict |
|---|---|---|---|---|
| F1 | A camp redshirt who plays is shown as a redshirt | 11,303 of 13,407 burned-redshirt rows played 5-16 games while labelled redshirt | 1 of 58,781 redshirt rows (2018-2025) has 5+ games | works |
| F2 | Early enrollee's spring step on his high school row | spring steps on the last high school row | 2,477 early enrollees: 621 spring steps on the first college row, 0 on the high school row | works |
| F3 | "a EDGE", "a OT" | many | 0 notes in alumni and pros | works |
| F4 | Signing news filed under the next year | most years | 1,150 of 1,168 February signing posts filed under the class's season; the 18 others are the pre-2018 class-ranking posts | works (ranking post left) |
| F5 | Specialist notes repeat the star count | repeated | 0 of 1,160 | works |
| F6 | Long snappers log no games | seasons uncounted | 3,596 of 3,787 LS seasons since 2016 have 5+ games | works |
| F7 | Senior Bowl and Shrine Bowl tiers | wrong tiers | Senior Bowl players drafted 89% (2,920 of 3,274), Shrine Bowl 49%, NFLPA Bowl 26% | works |
| F8 | Same award winners every year | repeats | 9 repeats in 227 position-award wins (4%) | works |
| F9 | Two Commander-in-Chief's Trophy holders | two at once | one holder line a year | works |
| F10 | A team's huge game posted as Medium | some | in code with a test; the export has no per-team stake to count | not measured |
| F11 | Momentum on interception returns | skipped | in code; not separable from the other engine changes (FBS one-score games now 34.0%, trial 6 28.5-28.9%) | not separable |
| F12 | Player page rows show no events | none | 680 of 1,449 season rows on the 117 cards carry events | works |
| F13 | Dashboard day-one share mixes windows | mixed | split by window (coach, winter, spring); first-day share 7-21% | works |
| F14 | Redshirt role only for freshmen the staff means to sit | 91-94% of first-year players | FBS first-year players with the redshirt role 92% / 87% / 84% / 83% (2022-2025); 191 / 282 / 214 play 5+ games (2023-2025, trial 6 about 135); 5-stars 10 of 20, 20 of 29, 15 of 32 | partly |
| F15 | One age rule for notes | 12,100 pages with a signing at 18 then an all-star game at 17 | 0 of 29,488 alumni pages have a note younger than the note before it; 86 pages show two spring breakouts at the same age | works |
| F16 | Toronto before London | wrong order | games abroad match the real table every year 2005-2025 (Toronto 2008-2013, none in 2020) | works |
| F17 | Signing-day fallback walk-on places | none placed | not measurable: signed recruits who have not enrolled carry no college in the export | not measured |

## The other checks the plan named

- **B6 (ratings drift between passes):** of the 613 summer and fall 5-star commitments (classes 1997-2021) matched to a player, 50% end as 5-stars, 46% as 4-stars and 4% lower. Trial 6: 25-28% stayed 5-stars and 35-38% fell to 3 stars or fewer. Works.
- **B7 (4-stars who never play college):** 148 of 7,403 final 4-stars of classes 1997-2017 (2.0%). Trial 6: 15.6%. Works.
- **D4 (5-star draft rate, re-measured after B6):** 85.9% of all 5-stars drafted, 39.9% in round 1 (only those who played FBS: 86.8% and 40.2%; real 62% and 21%; trial 6 70-74% and 31-32%). Worse, as expected: B6 makes the services' stars follow true upside more closely, so fewer 5-stars miss. Part D 4 already settles the next step: widen the services' noise at the top of the class.
- **B24 (profile one world):** see "Speed".
- **D42 (cards without offers notes or college stats):** traced. All 24 cards of classes before 2002 (and the 2003 class) have no college stats, and the 2004-2005 classes lose their seasons before 2006, because `crates/afm-tools/src/dashboard.rs` `college_stats` reads only the last 20 years (2006-2025). 9 of 100 4-5 star cards with college have no "He had N offers" note: the early-period and walk-on signing paths do not write it.
- **Dynasty bar (no program above 6 titles in 15 years):** fails. Georgia won 14 FBS titles and Texas 9 in 30 years; Georgia won 9 in 1998-2012 and again in 2003-2017. Only 8 programs won a title (real 1996-2025: 15).

## Positives

1. FBS games look like FBS: 28.5 points a team game (real 27.8), one-score games 34.0% (real 36.7%, trial 6 28.5-28.9%), third downs 41.6% (real 40.3%), home wins 59.2% (real 58.4%).
2. Recruiting spread holds: the top program signs 14.1% of FBS 5-stars and 19.8 programs sign one a year (real 14% and 15-17); in-state share median 38% (real 37%).
3. B6 and B7 work (above): stars no longer swing, and 4-stars play college.
4. The NFL cap holds in every capped year: no team over the cap at kickoff, median use 0.98; dead money and carryover are on the books; the biggest deals look real (a QB at 18.8% of the cap).
5. NFL age 26.0-26.8 from 2005 on (real about 26); the first pick is a QB 72% of the time (real 73%).
6. Games abroad match the real NFL table year by year.
7. The part A text and dashboard fixes all show in the data (F1-F9, F12, F13, F15, F16).
8. Section 5 runs every year: 25,284 pro careers, 7,991 NFL games, contracts, tags, tenders, options, extensions, restructures, cuts, claims, trades, comp picks, other leagues and honors.

## Findings (yes or no each)

Numbers are 1996-2025 unless stated. "Traced" means read in the code; "inferred" means not confirmed in the code.

1. **NFL games score far too much** (plain bug). 37.8 points a team game (35.3-41.3 a year; real 22.5), 9.0 yards a pass (real 7.05), 2.6 punts a game, and 15.9% of team games reach 50 points. Season lines follow: the passing leader throws for 5,781-10,174 yards and 52-100 touchdowns every year (real records 5,477 and 55; the season lines include playoff games), and 28-32 QBs pass for 4,000 yards a year from 2004. Why: traced in part. The standalone engine already plays NFL games at 29.9 points and 290 net pass yards (`match_check nfl`), inside the test band of 18-30 points; the world's pro rosters push it further (offensive line starters average 86, defensive line 81; inferred). Fix: retune the NFL level of the engine on the world's own pro rosters to the real 2021-25 numbers, and narrow the test band to 20-25 points.
2. **College stat lines run past the real records** (plain bug). 8-16 FBS backs rush for 2,000 yards a year except 2001, 2003 and 2004 (4, 2 and 7; real 0-2); the leader has 2,439-4,212 yards on 316-485 carries, and 55 seasons beat the FBS record of 2,628. Sack leaders reach 16-35 (record 24) and tackle-for-loss leaders 31-76 (real about 25-30). Team totals are near real, so the yards pile onto single players. Why: inferred: the lead back takes nearly every carry and pass-rush credit gathers on one player. Fix: real back committees (the lead back about 55-65% of the backs' carries) and spread sacks over the rushers who beat their blocks; check against the real FBS leaders (rushing 1,479-2,358 in 2022-24).
3. **Two programs win most titles** (plain bug against the agreed bar). Georgia 14 and Texas 9 of 30; 9 in one 15-year window (bar: 6); 8 different champions (real 15). Recruiting is spread like real (positive 2), so the gap opens after signing day. Why: not traced. Fix: trace where the two pull away after signing (development, depth, coaching, the playoff), then tune to the bar.
4. **Blue-chip recruits walk on** (plain bug, new since trial 6). 27% of 4-stars who reached college walked on (2,872 of 10,538) and 12% of 5-stars (120 of 988); they had a median of 9 offers; 26.5% of all draft picks walked on. Real: blue-chips sign scholarships; preferred walk-ons are mostly unranked. Why: traced in part: in the main signing match a program whose scholarships at a position are full offers a walk-on place, and the recruit's walk-on cost does not count the scholarships he turns down; only 68 came through the F17 fallback. Fix: a recruit with a scholarship offer takes a walk-on place only when the walk-on cost includes the scholarship he gives up, so blue-chips sign.
5. **The uncapped 2010 year freezes the NFL market** (plain bug). 2010: 0 signings with a new team, 0 trades, 0 waiver claims, 0 restructures, 0 cuts. Real 2010 had free agency (six accrued seasons for a UFA), trades and claims. Why: traced: in an uncapped year `cap_moves()` returns at once (`pro/market.rs:1615`), so no team cuts or restructures; payroll counts every contract, not the top 51; and the budget is the $123M placeholder cap times 0.95 + 0.02 × (owner − 10) with no carryover (`pro/mod.rs:424`). Teams opened 2010 at 0.88-1.40 of that budget (median 1.13; 2009 and 2011 about 0.99), and signings, claims and trades all need room. Fix: in an uncapped year, budget on each owner's spending over the 2009 cap, count contracts the top-51 way and let teams cut and restructure, keeping the real 2010 rules (six accrued seasons for a free agent).
6. **Franchise tags are almost never used** (plain bug). 11 in 30 years: none in 21 of them, 3 in 2006 (real about 6-14 a year; from memory, to verify). Why: inferred from the code: a tag needs cap room above the tag price at the tag window, when teams sit near 98% of the cap. Fix: let a team clear room for a tag with the cuts and restructures it would make anyway, as real teams do.
7. **Comp picks are mostly fillers** (plain bug). 0-5 a year come from the free-agent formula and 27-32 are supplemental (real: nearly all 32 come from the formula). Why: not traced. Fix: check the qualifying contract bar and the net-loss count against the real formula.
8. **Too few trades, too many of them safeties** (realism). 5-17 trades a year; safeties are 25% of traded players. Real: roughly 30-60 player trades a year in the 2010s-2020s (from memory, to verify), spread over positions. Why: not traced. Fix: trace the trade search and set its volume and position mix from a real trade list by year.
9. **NFL awards go to the wrong positions** (plain bug). Comeback Player of the Year went to long snappers 5 times, punters 2 and kickers 2 (real never). Defensive Player of the Year went to an edge rusher 3 times (real about 13 of 30); Defensive Rookie of the Year to a safety 10 times (real about 3); Offensive Player of the Year to a back 25 times (real about 11). Why: not traced (inferred: the vote reads raw production, which the inflated engine piles on backs, tacklers and safeties). Fix: judge each award from the season against the position's own best, as F8 did for college awards, and keep specialists out of the comeback vote.
10. **Round 1 takes too many receivers** (realism). Round 1: WR 24% (real 13.4%), OT 7% (12.1%), CB 8% (11.9%), TE 2% (3.6%). Kickers and punters went in round 1 four times: a kicker in 1999, two punters in 2005 (picks 12 and 16) and one in 2024 (real once since 1996). Why: not traced. Fix: trace the draft board's value by position and set it from real draft capital by position.
11. **One-year extensions repeat** (plain bug). 60 pros were extended two to four times at the same age ("Extended by the Seattle Seahawks at 34: 1 more years" three times), and each counts as an extension; 105 of the 108 repeats follow a one-year extension. Why: traced: extension talks run four times a summer (June 1, the tag deadline, the day before veterans report, three days before kickoff; `pro/mod.rs:886`), and `extend()` keeps the years up to this one and adds the new ones from next year, so after a one-year extension the deal still has one year left and the next talk extends it again, replacing next year's deal. Fix: no second extension talk in the same league year.
12. **Non-quarterbacks play quarterback all season** (realism). About 20 FBS seasons have a WR, RB, EDGE, LB, OT or CB throwing 250-456 passes (an OT threw 456 for Central Michigan in 2008). Why: traced (the anomaly note): the depth chart moves a player when his rating there beats the listed players by 4 points. Fix: a team plays a non-QB at quarterback only when it has no healthy QB or the player played QB before.
13. **Class rankings list junior colleges and FCS programs** (plain bug). A junior college is in the top three of 28 of the 31 national signing day rankings (Navarro 23 times, Georgia Military 11), and in 18 of them no FBS program makes the top three. FCS programs reach a top three only 7 times (once each). Real services rank FBS classes, with junior colleges ranked apart. Why: traced: the class score (`recruiting.rs`, `class_rankings_news`) does not look at the division. Fix: rank FBS programs only, by the services' weighted points.
14. **Too many long snappers on FBS rosters** (plain bug). 4.6 a roster (real 2.5), 3.9 of them on scholarship (real usually 1). Why: inferred: the camps rate 100 snappers a class and the roster plans take them like any position. Fix: plan one or two snapper places a roster, one on scholarship.
15. **Cards miss old college stats and some offers notes** (plain bug, D42). As traced above. Fix: read every recorded year; write the offers note in the early-period and walk-on paths.
16. **A world takes 94 minutes** (design choice). Trial 6 took about 41. Only 2.1 of 4 cores are busy on average. A recorded year takes about 134 s: college games 48-54 s, summer signing 19-26 s, the NFL day 16-18 s (people 12-14 s, of which the daily waiver scan 5.5 s; games 3-4 s), high school 8-9 s, coaching searches 6-9 s, signing days 6-7 s. The 12-year spin-up adds 6 years. Fix (proposed): run summer signing, the NFL people day and the coaching searches on all cores, and scan waivers only on days with a release.
17. **Small text and export bugs** (plain bugs, one bundle): the trade text names the buying team as the player's team in all 271 trades ("trade Nasir Haralson (S, 23, Philadelphia Eagles) to the Philadelphia Eagles"); "1 more years", "1 sacks", "1 five-stars", "1 games missed"; the report counts a program's old FCS titles as FBS (UConn 1998, Texas State 1999/2001/2005, James Madison 2012, Jacksonville State 2016) because seasons carry no division (fixed in the Debug Lab, which now reads the title news); the report's real draft-position and combine columns are blank (it reads `data/` from the folder it runs in); NFL events carry today's franchise name (Washington Commanders in 1996); the raw export writes 86 pros twice (identical rows).

18. **NFL teams too short-handed to play on the engine** (plain bug, new). Three team-seasons played most games off the engine: the 1997 Raiders 5 engine games of 16, the 2002 Bengals 4 of 16, the 2021 Cowboys 7 of 17. Their other games have no box score and no player stats. Why: traced in part: a team with fewer than 30 healthy players on its roster gets no engine side (`pro/games.rs`, `build_side`) and the stand-in decides the game; the 2021 Cowboys had released several players rated 74-81 as cap casualties and did not refill (inferred from the notes). Real teams always dress 46-48 and sign minimum players, making room by restructuring or cutting. Fix: a team always fills to 53 with minimum deals (making the room as real teams do), so every game runs on the engine.

Already agreed, for the next fix build: part D 4 (widen the services' noise at the top of the class), since the 5-star draft rate is still high.

## Slumps

8,326 rating drops and 26,365 form slumps over the history (the Debug Lab's Slumps list shows the 400 biggest). The biggest:

- Aaron Reddick (CB, 35, unsigned, 2024): 74.9 to 66.5 (-8.4). Age-35 decline, no team, no snaps.
- Sean Hurst (CB, 30, rotation, 2010): 78.0 to 70.7 (-7.3). Major injury, 15 games missed, age-30 decline.
- Kevin Roque (CB, 32, starter, 2018): 83.6 to 76.7 (-6.9). A form slump and age-32 decline.
- Kasim Garner (RB, 29, backup, 2019): 64.5 to 57.6 (-6.9). Major injury, 11 games missed.
- Zachary Fleming (RB, 29, starter, 2022): 83.7 to 77.6 (-6.1). Major injury, 17 games missed.
- Biggest form slumps: Hunter Marshall (QB, 23, NFL rookie, 2023) played at 50.7 against a 66.0 rating; Gabriel Dollar (QB, 25, NFL starter, 2006) at 62.4 against 77.2.
- College: the biggest drop is 4.5 (Jason Mackay, QB, 21, FBS backup: a minor injury and a form slump). All 32 drops of 5+ points are NFL veterans.

Why they read right: big drops come with age past 30 or major injuries, as in real football; young players lose form, not rating.

## Spot checks

One checker pass re-measured the 18 claims from the raw files with its own scripts. 6 held as written; 12 needed a correction, now made above. Three changed the reading: the cause of the 2010 freeze (finding 5), the cause of the repeated extensions (finding 11), and FCS programs in the class rankings (rare, finding 13). It also found finding 18 and the duplicate pros rows.

- Held: FBS titles (Georgia 14, Texas 9, 8 champions); walk-ons (4-stars 27.3%, 5-stars 12.1%, picks 26.5%, 68 via F17, median 9 offers; classes from 1996 alone give the same); comp-pick columns (0-5 formula, 27-32 supplemental, all round 7; the fourth column is the diversity picks); the trade texts (all 271; `describe()` runs after `transfer()`, `pro/trades.rs:434`); part A text checks (a, b, c, e) and LS games; B6 and B7.
- Corrected, small: NFL points 35.3-41.3 a year (not 36-41); passing leaders from 5,781 yards (1996) and 28-32 QBs past 4,000 (33 came from duplicate rows); 2,000-yard rushers also only 7 in 2004; tags 11 in 30 years (not 9); DPOY edge rushers 3 (not 1) and OPOY backs 25 (not 24), with 24 early winners placed by their All-Pro rows; extensions 60 pros (not 64); specialists in round 1 four times (not 7); D42 window 2006-2025 and 9 of 100 cards; D4's rates are for all 5-stars; F7's Shrine Bowl 49%; B6 613 matched players; college games 48-54 s a year from 1996.
- Corrected, cause: finding 5 (the cap moves stop in an uncapped year and the budget is a placeholder; the 5% reserve is not a cause); finding 11 (four talk windows a summer); finding 13 (junior colleges hold, FCS only 7 times).

## Doubts

- Real counts marked "from memory, to verify" (tags, trades) need a source before tuning.
- F10 and F17 could not be measured from the export.
- The FBS freshmen numbers for F14 come from players still in the world (2022-2025); older classes are archived without ids.
- "Unsigned pros now: 523; in other leagues: 0" at the end of the run: the other leagues' seasons start in spring, after the run ends.
- 24 NFL award winners of 1996-2025 have no row in the pros export, so the export does not hold every early pro (inferred: an export gap, not a world bug).
- The world starts in 1984: counts not limited to 1996 on mix in the warm-up years (checked where it matters: classes from 1996 alone give the same walk-on and draft rates).
- Season lines for NFL players include playoff games, so the leaders' totals are partly postseason.

## Speed (B24)

The profile (`AFM_PROFILE=1`, `stderr.txt`) is in finding 16. College games stay the biggest part (about 39% of a year); the NFL adds about 17 s a year; the world uses about half of the four cores.
