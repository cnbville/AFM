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
| Call values | `crates/afm-engine/data/call_values.csv` rebuilt after step 5 | moved to step 5 |

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
