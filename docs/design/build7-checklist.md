# Build 7 checklist

Everything agreed and not built on 2026-10-16, built in one thread ("Build everything agreed, then trial 7"), then trial 7. The inventory with where each item was agreed is in the project folder at `afm/build7/inventory.md`. Status: **built**, **moved** (built with a later step that rebuilds the same system), **finding** (measured, still short of real; goes to trial 7's list for Cain), or **waits** (agreed to wait on a measurement).

Claude's own picks at forks real life and earlier decisions leave open are marked "proposed, not confirmed".

## Step 1: speed-ups (identical results)
- Built (PR #39): compact body-separation scan, carrier headings table, signing-stage reads on all cores. Items 12 and 13 measured and left (see `afm/engine-speed/speed-ups.md`).

## Step 2: the engine

| Item | What | Status |
|---|---|---|
| Dynasty RB value, passing side | QB drives passing: time to throw with the throwing motion, ball placement by depth, man coverage timing, the sack and pressure model (`game/scrimmage.rs`) | built (98aedea), value gap is a finding |
| Coverage shells | Cover 2 corners press and squat, flat zones sit outside the numbers, underneath zones lag the route by the defender's read (`scrimmage.rs` alignment, `zone`, `Job::Zone`) | built; Cover 2 still the stingiest shell (finding) |
| FBS short completion | flat zone moved off the hitch | built; quick outs still short of real (finding) |
| Home field (trial 4 topic 3) | crowd, silent count, travel, time zones, altitude (`data/real-world/college_altitude.csv`), cold | built (1f40a19): FBS 2.4 points (real 2.4) |
| College calling (trial 4 topic 5) | real FBS pass rate and fourth-down tables (`data/real-world/scripts/college_situations.py`) | built (1f40a19) |
| Returns, sacks, interceptions | sacks land, fumbles, kickoff blocking, end-line catches, roughing and personal fouls | built (98aedea) |
| S8 nickel habit | each DC's own nickel rate | built |
| S9 fronts | one-gap and two-gap fits | built |
| S10 tempo | no-huddle stops substitution, breath comes back with time | built |
| D3 one-score games | margins | finding (NFL 45% within 8 against real 54%) |
| Fix list topic 3 (3.1-3.18): position spots | 21 spots inside the 14 positions (`spots.rs`): LT/RT, G/C, DT/NT, MLB/OLB, CB/NB, FS/SS, FB; ratings per spot from attributes; staffs set spots each spring (`systems::set_spots`); first time at a spot plays below rating (`fill_in`); lists stay inside a position group (`roster.rs` `depth_chart`); TE at fullback; CB/WR two-way exception only; emergencies across groups logged per play (`PlayRecord.emergency`, `GameSummary.emergency_snaps`); old skills fade (`development.rs` `OLD_SKILL_FADE`); a moved player rarely moves back (`athlete.rs` `reconsider`) | built |
| Call values | `crates/afm-engine/data/call_values.csv` rebuilt after step 5 | built (step 5) |

## Step 3: the world (built before step 2 while the attribute question is open)

Moved to step 6, where section 5 rebuilds the NFL as a living league (topic 1: every NFL player a person, every game on the engine, 28 teams to 1994, 30 from 1995, 31 from 1999, 32 from 2002, every move and name on its real date): part B 39 (era-true NFL names and league size), part D 30 (Christmas games), 31 (roster limits and cutdowns by era), 35 (flexes), 36 (prime-time caps), 37 (combine invite bar), 38 (undrafted signings), and part B 21 (pro season export: the world has no per-player pro careers until then).

| Item | What | Status |
|---|---|---|
| Part B (38) | See `docs/trials/world-trial6-proposals.md` | 33 built in earlier commits on this branch; B10 below; B7 and D4 wait on trial 7's measurement of B6; B24 (profile one world) runs in trial 7 |
| B10 | Portal size by era: entries counted by division (`CalendarLog.portal_by_division`) and the entry rate tuned to NCAA counts | built |
| Part C (9) | All built | built |
| Part D (42) and Cain's 3 calls | D40 game tags exported (`raw.rs` games: tags, loud, emergency snaps); D41 high school game rows need high school games on the engine; D42 traced in trial 7 | built except D41 (finding), D42 (trial 7) |
| S1-S7, S11 | Coaching systems (`world/systems.rs`): home system from the coach's group, systems spread through trees, fit to players, better scheme coaches read fit, a switch costs a year of learning, recruiting values fit and showcase systems, fit worth 1-3 points. Fit is read against the typical player at the position (`system_norms`), so no system is a worse fit for every roster | built |
| Topic 2 (2.1-2.8) | Home pull and pipelines | built |
| Topic 4 (4.1-4.7) | Volatile development; 4.7 check leaves out jobs lost to injury (`afm-tools/src/stats.rs`) | built; production drops need a real target (finding) |
| Topic 5 (5.1-5.7) | Walk-ons; 5.2 walk-ons local (`recruiting.rs` `walk_on_cost`, a scholarship-less spot away from home costs out-of-state tuition, College Board 2024-25); 5.7 walk-on and unrated picks reported (`world_trial` "Walk-ons and unrated players in the draft") | built |
| Trial 4 topic 6 | Draft board by surplus value: veteran market value by position from real top-5 cap shares, rookie scale by pick, QB board risk, GM need-versus-best-player style (`recruiting.rs` `draft_position_value`, `rookie_cap_share`) | built |
| Trial 4 topics 4, 10 | Firing with contracts and buyouts; coaches of color by rung | built (earlier commits) |
| 5-star spread | A 5-star's pull to title programs (`recruiting.rs` `FIVE_STAR_PULL`, prestige cubed) | built; trial 7 measures programs per class (real 15-17) |
| Real JUCO, D3, NAIA | `data/real-world/colleges_lower.csv` | built |

## Step 4: section 3 leftovers

| Item | What | Status |
|---|---|---|
| Career stats record (topic 8) | `world/records.rs` `StatBook`: every college season at every division kept forever (65 stats from the game lines, program, position, games, schedule strength); career pages (`career`); percentiles among division and position peers (`percentile`, `Metric`); each season's norms and the era-adjusted view (`era_adjusted`); the opponent-adjusted view (`opponent_adjusted`) | built |
| Record books | Single season and career per division (a career counts that division's seasons), single season per conference, streaks (touchdown passes, 100-yard rushing games, catches), program and high school winning streaks; a new division record is news once the books have ten seasons | built |
| All-America teams | First team by position, 25 a level (`recruiting.rs` `ALL_AMERICA`), honors kept for the Hall of Fame | built |
| Hall of Fame | College Football Hall of Fame vote each season: first-team All-Americans, 10 to 50 seasons after their last, 12 top-level and 4 divisional players; coaches retired three seasons with 100 games and a .600 record | built |
| Coach buzz from stats (world item 5) | Offense and defense EPA a play against the division and conference (`records::unit_buzz`), fading by half a season, in `Coach::hiring_buzz` | built for college; high school coach buzz needs high school games on the engine (finding) |
| Hidden-gem footprint (world item 9) | Top 1% of a division and position at a footprint metric, and testing outliers; a flagged small-school player gets a truer December draft read and a note (`recruiting.rs` `draft_reads`) | built for college; high school footprints need high school games on the engine (finding) |
| High school games on the engine | About 77,000 high school games a year at about 8 ms each on 4 cores: about 10 minutes a season, 5 hours a 30-year world | finding |
| NFL records and the Pro Football Hall of Fame | Need per-player pro careers | moved to step 6 |

## Step 5: plays and tactics (`docs/design/plays-and-tactics.md`)

| Topic | What | Status |
|---|---|---|
| 1. Plays from real parts | Sets (doubles, trips, bunch, stack, empty, wing), motion (jet, orbit, across, shift), fronts with real techniques (over, under, wide nine, odd, tite, bear), stunts, fire zones, Cover 1 robber and Cover 3 match, RPOs with the conflict defender's read, the option pitch, run fits that can be missed; level-only plays (the triple option lives in high school and the service academies) (`game/playbook.rs`, `scrimmage.rs`) | built |
| 2. What beats what | No counter table decides a snap. The coaches call from `data/call_values.csv`, each concept against each coverage and pressure and each run against each front, measured on the engine (`afm-tools` `call_values`); the odd and tite fronts are measured with a 3-4 | built |
| 3. Before the snap | Motion shows man or zone, defenses travel or pass it off; disguise and the QB's pre-snap read by IQ; kill calls and checks by the coach's trust; a young defender can miss the check | built |
| 4. Learning playbooks | 14 concept families (`Family`); each player's knowledge per family (`GamePlayer.know`), a step slow when still learning, busts by what he knows; line chemistry from snaps together and stunts passed off by it | built |
| 4.6 and calendar T6.2: the install | The coordinator sizes the book by what his players can absorb; a short week cuts the install (Thursday games 0.65, a bye 1.15, `roster::prep_week`); playing past what was installed adds busts (`Tactics.load`, `roster::absorb`) (proposed, not confirmed sizes) | built |
| 5. The weekly game plan | Film breakdown by the coordinators and quality control (`Film`), the matchup plan (weak corner, bracket, chip, spy), weather | built |
| 6. Play callers | Four calling skills, 1-20: reading, situations, creativity, adjustment; pressure changes callers by personality; the caller's reason on every call (`PlayRecord.why`) | built; the good-against-bad caller swing is measured with `dynasty_check` |
| 7. Tendencies | Every snap goes on film by look; a predictable caller's tells let defenders read a beat sooner; self-scouting by creative callers; last season's tape at less weight | built |
| 8. The play designer | `game/designer.rs`: a play from the library's parts, legal by each level's rules, the coordinator's review at his skill, a noisy scout-team read; a saved design is a concept like any other | built (engine side; the screen comes with section 10) |
| 9. Ideas spreading | `world/trends.rs`: each division's family shares and pass rate by season (the trend lines), a family that grew fast is new to defenses and read a beat late, a rare one is new to every opponent | built |

### Step 5j: calibration after plays and tactics

Plays and tactics pushed NFL scoring to 32.2 points (real 22.5) and kickoff return touchdowns to 0.11 a team game (real 0.012). What changed (each proposed, not confirmed):
- The kicker kicks from between the hashes and trails the coverage; the safeties stay behind the wave. Kickoff return touchdowns 0.11 → 0.02.
- One open-field lunge for every tackle in space, 2.5 yards (it was 2.2, and 1.6 on returns). Runs of 40+ were twice real.
- Drops: quick 1.25 s to the top of the drop (was 1.05), reads start a tenth after it (was a quarter before). Throws under 1.6 s 21% → 6% (real 10%).
- Quick game is one picture: the QB keys the flat defender and throws whichever of the first two routes is open. Read one at a time he forced the stick into the flat defender (48% complete against Cover 3; now 87%).
- Man coverage lag 0.12 s at the break.
- Every NFL (400 games): points 32.2 → 30.0, completion 69.2% → 65.8% (real 64.5%), sacks 5.2% → 6.1% (6.8%), time to throw 2.39 → 2.56 s (2.71).
- FBS fell with it: points 28.7 → 24.3 (real 27.0), completion 60.8% → 57.1% (60.6%), yards per attempt 7.0 → 6.5 (7.3). High school sacks rose from 10.9% to 16.9% of dropbacks and points from 27.5 to 21.6 (no real high school play-by-play; high school games still run on the stand-in in the world). The engine scores more in the NFL than in FBS, the reverse of real football. All three are findings for trial 7.

### Found by the full test run
- The old-skill fade (fix list 3.8) also faded skills from a high school freshman's first position, which put 9 first-rounders a year from outside the consensus top 50 (real 1-3). It now counts only positions held from 18 on: 3.
- The move-back rule (3.9) left 8 retired quarterbacks with three or more seasons in the class sim, with a median last age of 28.5 against 27 for backs. An unsigned quarterback now waits longer for a call (every team carries two or three): median 31 (proposed, not confirmed size).
- Under the House limit the world test found Power 4 rosters under 105. Three causes, all fixed: the class plan didn't count recruits who signed in an earlier period and hadn't enrolled; walk-ons signed before the January portal window took places the portal should have; and the 2024-25 walk-ons the settlement names as designated student-athletes counted against the limit (real: they never do). Under the limit a class now fills what the plan leaves with walk-ons in the final period (proposed, not confirmed).
- With the rosters fixed, the test found 5 FBS head jobs still open in March. A head search after the regular season is now a sprint: FBS 2-10 days, FCS 5-21 (real January searches: Michigan 2 days in 2024, Washington 2, Arizona 5, San Jose State 4; proposed, not confirmed sizes).

## Step 6: section 5 in the world (`docs/design/team-management.md`)

The world's living NFL for every AI team; the manager's own screens come with section 8. The class sim (`career.rs`) keeps its own pro model.

| Item | What | Status |
|---|---|---|
| Part B 39: franchises by era | `world/franchise.rs`: 28 teams to 1994, Carolina and Jacksonville in 1995, the Browns' move to Baltimore in 1996 with Cleveland dark until 1999, Houston in 2002 and the eight divisions; every name, city, division and roof on its real dates; expansion drafts; draft rounds and order by era | built |
| Topic 1: pros as people | `world/pro/`: every pro a person from high school and college; the league filled at the start with generated veterans spread like a real roster; drafted and undrafted players join and stay people; era roster limits, practice squad, game-day actives, IR by era | built |
| NFL games on the engine | `pro/games.rs`: actives from the depth chart, the staff's calling and installed systems, the stadium's weather; preseason by era; box score onto each player's season | built |
| Pro development and retirement | Snaps against real opponents drive pro growth; the unsigned veteran's weekly choice; retirement by position and age | built |
| Topic 2: contract value | `pro/money.rs` and `market.rs` `worth`: the cap by year, minimums, real contract parts (base, proration, guarantees, roster, workout and per-game bonuses, incentives, void years), dead money, the rookie scale from 2011 and the rookie pool before it; each team prices a player through its GM's biases, scheme fit, age curve and need | built |
| Topic 3: agents | `market.rs` `Agent`: agencies big and small, skill and honesty, client lists, standing with each team; players hire and fire them | built |
| Topic 4: negotiations | The player's hidden ideal package, offers scored part by part, the perfect offer signs at once, 85% opens a 7-day window, bluffs, verbal agreements wait for the league year | built |
| Topic 5: free agency | UFA at 4 accrued seasons, RFA at 3, ERFA below; tags from 1993 with repeat prices; RFA tenders and offer sheets with a 5-day match; the tampering window from 2013; Plan B before 1993; fifth-year options; rookie extensions after the third season from 2011; holdouts and hold-ins; restructures, cap casualties and June 1 designations (A1, A2); waivers and claims; reserve/future deals; the cash floor; comp picks by the real method (`pro/trades.rs`) | built |
| Topic 6: trades | `pro/trades.rs`: each GM's pick chart (classic or surplus), player value over his remaining years minus his cost, windows from the league year to the deadline, dead money to the trading team, a physical can void a deal, shopping rumors | built |
| Topic 7: AI front offices | `pro/office.rs`: GM style and hidden biases, owner traits and meddling (draft overrides, pet players), fan mood, contend or rebuild each year | built |
| Topic 8: scheme fit | Fit to the staff's installed systems in each team's price | built |
| Other pro leagues | `pro/leagues.rs`: CFL, USFL, Arena, World League and NFL Europe allocations, XFL, UFL, AAF on their real dates; the International Player Pathway from 2017 | built |
| A1-A8 contract dates | Roster bonus due dates, June 1, workout bonuses, camp report day, hold-ins, the RFA deadline, per-game bonuses, the incentive cut-off | built |
| A15 and NFL records | `pro/honors.rs`: Pro Bowl, All-Pro, season awards, Super Bowl MVP, the NFL record book from the real 1983 marks, the Pro Football Hall of Fame vote | built |
| Part D 30, 35, 36 | Christmas games by the real count per season; flexes only for a losing night-game team and a better game; prime-time games capped per team | built |
| Part D 31 | Roster limits and cutdowns by era | built |
| Part D 37, 38 | Combine invites: the draftable players plus about 60 more; undrafted signings sized by the room left on rosters | built |
| Part B 21 | Pro seasons in the export (`raw.rs`: pros, market, trades, clubs, honors, agents) and an NFL section in the trial report | built |
