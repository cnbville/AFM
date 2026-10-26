# Training: how it works today

Checked against main at a423e02 on 2026-10-26.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`. Nearly every size in the training week and in prep is marked in code as Claude's (proposed, not confirmed); `docs/world-model.md`, "Training: the week's frame and prep", lists them.

## Quick answers

- Does training decide how much a player grows? No. The yearly step still decides the year's growth rate; training only decides when it lands and moves each family by at most about 15% a week (`training/mod.rs` module notes; `training/work.rs`, `share`, `PLAN_REACH` 0.15). A run of poor prep also slows technique, kicking and mental growth by up to 12% (`training/prep.rs`, `slow`).
- When does training run? Every Monday, for every team in the world (`world/clock.rs`, the day loop, `training::week::monday`). Off when `options.yearly_growth` is true (default false) (`training/week.rs`, `monday`; `world/mod.rs`).
- What are the families? Strength, speed, conditioning, technique, contact, kicking, mental (`training/mod.rs`, `Family`, `family_of`). The screens add recovery and team, which grow nothing (`training/plan.rs`, `Kind::shown_family`).
- Does a plan change potential? The practice plan doesn't: shares only land growth toward the existing ceilings (`development.rs`, `grow_share`). A body plan's weight change moves six attributes and their ceilings (`training/seat.rs`, `body_change`).
- Does the AI follow the rules? Always. Its weeks pass through `fit` and `fit_block` (`training/plan.rs`). You can push a week past them on purpose (`training/seat.rs`, `save_this_week` with `push`).
- NCAA hour limits? 20 countable hours a week, 4 a day, one day off in season, spring and bowl weeks from 1991; 8 hours a week out of season with up to 2 of film from 2013 (`training/rules.rs`, `book`).
- NFL CBA limits? From 2011: phase 1 strength only, phase 2 no offense against defense, OTAs no pads and no live contact, 4 practice days a week; camp 4 hours on the field; season 3 hours, 14 padded practices (`training/rules.rs`, `book`, `block_limits`).
- High school contact limits? California from 2015 (two full-contact days, 90 min each, none out of season), Texas from 2017 (90 min a week in season), other states the NFHS 2014 guidance (90 min a week in season) (`training/rules.rs`, `hs_contact`).
- Does coach quality change what a practice teaches? No. Coach quality enters through the yearly step's coaching boost, stored in the year's growth (`development.rs`, `develop_season`; `training/mod.rs`, `GrowthYear.coach`). Practice work reads only minutes, kind (and a meeting's sub-kind), contact, tempo and reps (`training/work.rs`, `per_minute`, `period_work`).
- Does a player's plan focus work for AI teams? No. Only your seat sets a focus; AI players land with no focus (`training/week.rs`, `monday`, `settle(.., None, ..)`; `world/seat.rs`, `focus_by_person`).
- Is the staff's suggested focus applied when you set none? No. Growth reads only a focus you set. The screens label the staff's pick as the focus "by staff" (`crates/afm-game/src/lane_training.rs`, `plan_json`), but `focus_by_person` returns only your picks (`world/seat.rs`).
- Do practice reps move the depth chart? Yes, a little: the practice grade adds up to ±3 rating points in the staff's depth order (`training/mod.rs`, `impression`; `game/roster.rs`, `staff_depth_chart`).
- Do AI teams practise on the engine? College and NFL teams run a sample of snaps each week (camp 24, spring 16, season 8, OTAs 6); high schools other than yours get a fitted model instead (`training/practice.rs`, `ai_cap`, `world_battles`, `school_grades`).
- Do practices injure AI college and high school players? No. Practice injuries run for your college or high school team only, and for every NFL club; your own NFL club's practice risk comes from your week (`training/seat.rs`, `week`, `practice_injuries`, `my_risk`; `world/pro/games.rs`, `practice_injuries`). See [injuries.md](injuries.md).
- Does a heavy week tire the team on game day? Yes, for your team only: kickoff energy down to 0.85 (`training/work.rs`, `freshness`; `training/seat.rs`, `prepare`; `game/flow.rs`, side energy).
- What is a player's prep? Each Monday every player in the world gets one word from what he really did that week against an average staff's week for his role: Unprepared, Underprepared, Patchy, Prepared or Sharp; None in a week the staff may run nothing (`training/prep.rs`, `read`, `word_of`, `WORDS`).
- What does prep do in a game? Only mental errors, for every team: busts and run misfits × 1.33 (Unprepared) to × 0.93 (Sharp), a later read, a quarterback slower on each read. It never cuts a rating (`training/prep.rs`, `game_factor`; `game/scrimmage.rs`, `run_snap`, `quarterback`).
- Does an average staff's week leave players prepared? Yes. In the test week Prepared is the most common word for starters and backups, Unprepared and Underprepared together are under 12%, and the mean game factor is 0.97-1.03 (`tests/training.rs`, `an_average_week_leaves_the_two_deep_prepared`).
- What is rust? Two poor weeks in a row start taking points off his reading (play recognition, anticipation) and his position's two main technique ratings, up to 3 each; prepared and sharp weeks win them back (`training/prep.rs`, `record`).
- Does weather move practice? Your team's only. With an indoor field, severe weather, lightning, snow, bitter cold and high wind send the team inside; without one, severe weather costs the day's field work and lightning half an hour (`training/seat.rs`, `practice_weather`, `weather_cuts`).

## How growth lands (the hand-off from development)

- At each checkpoint the yearly step works out the year as before (ceilings, form, injuries, ageing). With `defer` on, it skips the growth itself for every attribute his position uses that is not declining, and stores the year's rate in `player.train` (`development.rs`, `develop_season`, `ctx.defer`; `training/mod.rs`, `Train::open`, `defers`).
- `defer` is `!options.yearly_growth`, set for college and high school in `world/season.rs`, `develop`, and for pros in `world/pro/mod.rs`, `camp` (the pros' yearly checkpoint).
- What is stored (`training/mod.rs`, `GrowthYear`; bloom and position on `Train`): age, flags (high school, college, redshirt), coaching boost (`0.5 + v/250`), snaps multiplier (`v/200`), morale multiplier (`0.8 + v/600`), physical multiplier (`0.9 + v/1200`), form times injury multiplier (`v/4000`), his bloom and his checkpoint position.
- Opening the new year keeps his practice grade, his last five prep words and his rust (`development.rs`, `develop_season`); `close_year` keeps his load, prep words and rust (`training/mod.rs`, `close_year`).
- Attributes his position does not use (a receiver's tackling) and ones past their growth window or declining settle at the checkpoint as before (`training/mod.rs`, `deferred`).
- Each week the level's calendar gives each family a base share (`training/rules.rs`, `weekly_base`: the block's share over its weeks). `cover` caps the year at 1.0 per family (`training/mod.rs`, `cover`).
- The share is moved by the week's work: `s = base × (1 + 0.15 × tanh(raw / 0.15)) × attend`, `raw = PRACTICE_DRIVEN[f] × (clamp(r, 0, 1.8) − 1)`, where `r` is the week's work against the neutral week's (`training/work.rs`, `share`, `ratio`).
- `PRACTICE_DRIVEN` per family: strength 0.55, speed 0.3, conditioning 0.7, technique 0.5, contact 0.6, kicking 0.5, mental 0.35 (`training/work.rs`).
- Then a run of poor prep multiplies the technique, kicking and mental shares by his growth factor (see "Rust and slower growth"), for your players and the AI's alike (`training/prep.rs`, `slow`; `training/seat.rs`, `week`; `training/week.rs`, `monday`). Like a share below the base, the cut is not made up later: `cover` counts the calendar's base, and `close_year` lands only what the calendar never reached (`training/mod.rs`, `cover`, `close_year`).
- `land` applies a share to each deferred attribute with the stored rate, its age factor and noise `0.5 × sqrt(share) × normal` (`training/mod.rs`, `land_after`; `development.rs`, `grow_share`). Shares adding to one give exactly the yearly step's growth (`development.rs`, `grow_share` notes).
- Your players land every week (`training/seat.rs`, `week`). AI players save growth in `due` and land it when a family has 167/2000 (about a twelfth of a year) waiting, or in a block's last week (`training/mod.rs`, `owe`, `settle`; `training/week.rs`, `monday`).
- At the next checkpoint `close_year` lands what waits, then lands the part of the year the calendar never reached at the default, and sets last season's row to the overall he really ended on (`training/mod.rs`, `close_year`; called before `develop_season` in `world/season.rs`, `develop` and `world/pro/mod.rs`).
- Until then the season row shows `projected_overall`: the whole year at the default, no noise (`training/mod.rs`, `projected_overall`; `development.rs`, `develop_season`).
- The rate itself (`development.rs`, `GrowthCtx::rate`): profile growth rate × age factor × coaching^coachability-exponent × snaps^experience × work (`0.75 + 0.025 × work ethic`) × form × redshirt lift (1.35 on physical) × high school lag or college teaching (1.3 on non-physical) × morale × focus × physical (physical group only), clamped 0-0.5. How each input is set is on [development.md](development.md).

## The year: blocks per level

Blocks: winter, spring, quiet, summer, camp, season, postseason, NFL phase 1, phase 2, OTAs, break (`training/rules.rs`, `Block`). A week's block is Wednesday's (`training/week.rs`, `block_of_week`).

- College frame: winter from the spring semester, spring practice to the spring game (none at Division III), quiet weeks after, summer from summer workouts, camp from fall camp to 5 days before the first game, season to the division's last regular Saturday, postseason after (`training/rules.rs`, `Frame::college`, `Frame::block`; `training/week.rs`, `Frames::new`).
- NFL frame: phase 1 the first 14 days of the offseason program (two weeks earlier for a new head coach), phase 2 to OTAs, OTAs to the minicamp's last day, break to veterans' report, camp to 4 days before kickoff, season to the day before Black Monday, winter after (`training/rules.rs`, `Frame::pro`). The world pass uses `new_coach = false` for every club (`training/week.rs`, `Frames::new`).
- High school frame: camp opens 21 days before the state's first game, season to the state final, spring ball May 1-31 where the state allows it, summer from June 1 (`training/rules.rs`, `Frame::high`).
- `Frame.big` marks FBS programs and the NFL; it decides which newer period kinds exist (`training/rules.rs`, `Frame`; `training/week.rs`, `Frames::new`; `training/plan.rs`, `available`).
- Each block holds a share of each family's year; each column sums to 1 over the year (test `each_family_adds_to_one_year`) (`training/rules.rs`, `block_share`). Examples, college (strength, speed, conditioning, technique, contact, kicking, mental): winter 0.35/0.35/0.25/0.03/0/0.10/0.07; camp 0.10/0.10/0.15/0.25/0.35/0.20/0.20; season 0.12/0.05/0.15/0.30/0.30/0.45/0.45. Division III moves spring's body share to winter and its football share to camp.
- Blocks the staff can't run (`Quiet`, `Break`, NFL winter, and college summer before 2013) are the players' own time (`training/rules.rs`, `Block::own`, `book`).
- Testing days (TR5): college on the first Monday a week or more into the spring semester, the first Monday on or after 12 days before spring practice, and the first Monday on or after 10 days before fall camp; the NFL on the offseason program's first day and on veterans' report day; a high school on the Monday a week before the first Monday of May and the first Monday on or after 10 days before camp. On a testing day (not a game day) the day's weight room is dropped and 60 minutes of testing added (`training/plan.rs`, `extras`, `testing_day`).
- In the NFL's summer break every week gets two 90-minute players' own throwing sessions, Tuesday and Thursday (`training/plan.rs`, `extras`).

## The rule books

`book(tier, block, season, day, state, heat_rule)` returns what a day allows: coaches allowed, on-field football, offense against defense, most gear and contact, countable minutes a week and day, on-field minutes a day, film a week, days off, practice days, full-contact limits, back-to-back full contact, two-a-days, voluntary, and whether the frame is big (`training/rules.rs`, `book`, `Book`; `big` set from the frame in `training/week.rs`, `books`).

- College camp: acclimatization 5 days from 2003, 7 from 2021 (helmets and bags the first 2 days, shells and control after); two-a-days allowed after it until 2017; from 2021 75 minutes of full contact a session (`book`).
- College spring: 15 practices, 12 contact, 3 scrimmages; from 2022 no full contact on back-to-back days and 75 minutes a session; no two-a-days (`book`, `block_limits`).
- College camp counts from 2021: 25 practices, 18 contact, 9 padded, 2 scrimmages; before, 29 practices. Bowl practice: 15 (`block_limits`).
- NFL OTAs from 2011: 13 practices, none with contact or pads. Camp from 2020: 16 padded, 5-day acclimation. Season from 2011: 14 padded (`block_limits`, `book`).
- NFL phases 1-3 are voluntary (minicamp included in code: `voluntary` is true for the whole OTA block) (`book`). High school summer and college summer before 2013 are voluntary too (`book`).
- High school camp from 2011 or with a state heat rule: helmets 2 days, shells to day 5, 3 hours on the field, no two-a-days the first 5 days. Season: 3 hours on the field, 1 day off, no two-a-days. Summer: helmets only, voluntary (`book`).
- `check_day` and `check_week` list every breach in words; `Counts::over` checks a block's practice counts (`training/rules.rs`).
- `check_day` lets the players' own sessions and study hall stand in a block where nothing may be required, and lets the strength staff's work (conditioning, speed, mat drills, testing, mobility) and the players' own sessions stand on a day with no on-field football (`training/rules.rs`, `check_day`; `training/plan.rs`, `Kind::strength_staff`, `Period::players_own`).
- A day counts as a practice in the block's counts only with a staff practice in it; the players' own throwing doesn't count (`training/rules.rs`, `Counts::add_week`; `training/plan.rs`, `Period::practice`).

## The practice day and week

- A practice is a list of periods: kind (35 kinds), minutes, gear (none, helmets, shells, pads), contact (air, bags, control, thud, live), tempo (walk, normal, fast), who (all, ones, young), `what` and `noise` (`training/plan.rs`, `Period`, `Kind`, `Gear`, `Contact`, `Tempo`, `Who`).
- The 17 older kinds: warm-up, individual, inside run, 7-on-7, one-on-ones, team, scrimmage, situations, special teams, walkthrough, conditioning, speed and agility, weight room, meetings, film, young players, treatment and recovery. The 18 kinds of update 2 (TR5-TR35): mat drills, testing, install, study hall, routes on air, ball security, kicking stations, mobility and yoga, vision and reaction, VR reps, mental skills, rules clinic, game plan test, nutrition, leadership council, team building, signals and communication, home study (`training/plan.rs`, `Kind::ALL`, `Kind::label`).
- `what` is the situation for team and situation periods (`SITUATIONS`; for a team period 0 is base work and 1+ this week's install against the opponent's looks), and the sub-kind elsewhere: meetings (team, unit, position), film (the opponent, self-scout, practice tape, situation cut-ups, the new system, the game graded), walkthroughs (jog-through, mock game, hotel walkthrough), installs (base downs, third down, red zone and goal line, two-minute and situations, the new system), outings (team dinner, community service, pasta dinner, team outing), recovery (treatment, rehab group), the weight room (in season), routes on air (1 is the players' own session with no staff) (`MEETINGS`, `FILMS`, `WALKS`, `INSTALLS`, `OUTINGS`, `RECOVERY`, `Kind::whats`, `Period::own`).
- `noise` is crowd noise on the speakers (TR28); the screen allows it on field periods only (`Period.noise`; `crates/afm-game/src/lane_training.rs`, `parse_period`).
- Gear caps contact: helmets bags, shells thud, pads live (`Gear::most_contact`). Thud and live are full contact (`Contact::full`).
- Counted against the rules (the NCAA's countable hours): every period except treatment, study hall, nutrition, the leadership council, team building, mental skills, home study and the players' own throwing; the code marks this reading as proposed, not confirmed (`Kind::counted`, `Period::counted`). A game day counts 180 minutes flat, its own periods inside them (`Day::countable`). A rest day has no game and nothing counted, so treatment, study hall and outings can fill it (`Day::rest`).
- On the field: warm-up, individual, inside run, 7-on-7, one-on-ones, team, scrimmage, situations, special teams, walkthrough, conditioning, speed, young players, routes on air, ball security, kicking stations (`Kind::on_field`). The rules' film cap counts meetings, film, walkthroughs, installs, VR, the rules clinic, the plan test and signals (`Kind::film`).
- Kinds by level and era (proposed, not confirmed): VR reps from 2015 and vision boards from 2010, at FBS programs and NFL clubs only; mental skills from 2005 in the NFL and FBS, 2015 elsewhere in college, never at a high school; nutrition from 2000 in the NFL, 2005 at FBS, 2015 elsewhere in college, 2010 at a high school; mobility from 2000 in the NFL, 2005 in college, 2012 at a high school (`training/plan.rs`, `available`). The planner offers only the kinds your team has (`crates/afm-game/src/lane_training.rs`, `options`).
- A week is seven days from a template by slot: game, bye, camp, spring, winter, summer, bowl, phase 1-3, own (`training/plan.rs`, `Slot`, `Slot::of`). A season week with no game is a bye; postseason without a bowl, quiet weeks and the NFL's break are own time (`Slot::of`). Game weeks count back from the game, so a Thursday game shifts the template; a long week fills from the bye template; the template's own game day keeps its periods (`week_from`).
- Neutral templates per level and slot are the average staff's week (`training/plan.rs`, `neutral`). Example college game week: Sunday treatment, a recovery lift, the game graded on film, mobility, the young players' practice in shells, nutrition; Monday the required day off, with treatment and home study; Tuesday the base-down install and a padded practice with thud, the team period with crowd noise; Wednesday the third-down install, padded, situations and team with crowd noise; Thursday the red-zone install, the plan test, the rules clinic, the mock game in shells and mental skills; Friday meetings, cut-ups, a walkthrough, VR and the team dinner; Saturday the game with the hotel walkthrough and the council.
- College and NFL neutral game weeks have two crowd-noise days and a high school's none; college and high school have 10 minutes of rules clinic in a game week and the NFL none (`neutral`).
- In own time the college template keeps two 60-minute players' own throwing sessions (Tuesday, Thursday); other levels have nothing (`neutral`, `Slot::Own`).
- `fit` drops kinds the level or era doesn't have; on a day the staff may run nothing it keeps only the players' own sessions and study hall; on a day with no on-field football it keeps the strength staff's work and the players' own sessions; over the week's countable minutes it scales only practice days (a game day stays 180); the days off the rules require keep only uncounted periods (`training/plan.rs`, `fit`).

## The time budget: what a minute works

- Work per minute by kind and family (`training/work.rs`, `per_minute`). Examples: individual 1.0 technique, 0.35 contact; inside run 1.0 contact, 0.45 technique, 0.4 mental; weight room 1.0 strength; conditioning 1.2 conditioning; speed 1.0 speed.
- The classroom grows only the mental side: film 0.9, install 0.8, unit and position meetings 0.7 (a team meeting 0.3), signals 0.7, vision 0.7, home study 0.6, VR 0.5, plan test 0.5, mental skills 0.4, rules clinic 0.3 (`per_minute`).
- Other new kinds: mat drills 1.0 conditioning, 0.35 speed, 0.2 contact, 0.1 strength; testing 0.3 strength, 0.3 speed, 0.1 conditioning; routes on air 0.35 technique, 0.35 mental; ball security 0.5 technique, 0.5 contact; mobility 0.3 speed; kicking stations 0.2 technique, 0.15 kicking. Treatment, study hall, nutrition, the council and team building work nothing (`per_minute`).
- Specialists have their own table: individual and special teams 1.0 kicking, kicking stations 1.2 (`per_minute`).
- Contact column × contact factor: air 0, bags 0.35, control 0.65, thud 0.9, live 1.0; mat drills' contact isn't scaled (`contact_factor`, `period_work`).
- Tempo: walk 0.1 conditioning, 0.3 skills, no contact or speed work, 0.9 mental; fast 1.4 conditioning, 1.15 skills (`period_work`).
- Reps by role (`reps`): a period for all gives everyone 1.0. A ones' period gives starters 0.75 (0.6 in camp), backups 0.4 (0.5 in camp), young players 0.35 as the scout team. The young players' period gives young 1.0, backups 0.5, starters 0.
- Scout-team reps teach only 40% of the mental work (they run the opponent's plays) (`period_work`).
- Daily absorb cap per family: `cap × (1 − e^(−raw/cap))`, caps 60/30/60/60/35/50/90 work-minutes (`DAY_CAP`, `day_work`). Technique, kicking and mental caps × `absorb_factor` = clamp(0.75 + 0.005 × (football IQ − 50) + 0.02 × (work ethic − 10) + 0.015 × min(age − 17, 8), 0.6, 1.3).
- Absorb is used for your players' ratio (`training/seat.rs`, `week`). The AI's ratio is worked out once per week shape with absorb 1.0 (`training/week.rs`, `team_week`).
- Roles: starter, backup (two-deep), young (outside it), specialist (`training/work.rs`, `Role`). AI college and high school take it from last season's playing time; pros from their depth slot; you from your depth chart (`Role::of`; `training/week.rs`, `monday`; `training/seat.rs`, `roles`).

## AI staffs' plans

- An AI week is the neutral template moved by the head coach's style (`training/plan.rs`, `styled`). Each style is read as `(v − 10.5) / 9.5`, clamped −1..1.
- Freedom: individual and walkthrough minutes × (1 − 0.2 × free); team and 7-on-7 × (1 + 0.2 × free). Physical: inside run and one-on-ones × (1 + 0.25 × phys); thud becomes control below −0.4; inside-run thud becomes live above 0.55. Risk above 0.4: team, 7-on-7 and situations go fast.
- Weeks are shared: teams with the same tier, division or state, style bucket (each of physical, freedom, risk: below 8, 8-13, above 13), game days and bowl status share one worked-out week (`training/week.rs`, `bucket`, `Key`, `monday`).
- Every AI week, and the neutral week it is measured against, gets the week's dated extras (testing days, the NFL's summer throwing), then is fitted to the day's books, then to the block's counts left (earlier weeks of the block rebuilt the same way) (`training/week.rs`, `staff_week`; `training/plan.rs`, `extras`, `fit`, `fit_block`).
- Each shared week also works out what it gives each role for prep against the neutral week (`training/week.rs`, `team_week`, `TeamWeek.prep`; `training/prep.rs`, `prep_week`).
- AI staffs leave reasons from their styles for the screens (`training/offseason.rs`, `staff_reasons`).

## Weekly prep: what he really did (TR36, TR37)

- What a week gives a role (`training/prep.rs`, `parts`), in minutes: reps (team, situations and 7-on-7 in a game week), installs and meetings (a team meeting at half), film (the rules clinic and vision at half a minute each), walkthroughs, study (plan test, signals, VR, home study), work (every counted practice, strength-staff and weight-room minute he is in, by his reps), the install's size (install minutes) and the days with anything counted.
- In a week with a game only this week's plan counts, each minute × `prep_weight` (below); out of one every minute counts in full (`parts`; `training/work.rs`, `prep_weight`).
- Reps: his own share of a period, plus half of the rest as mental reps behind the play; a young player's own reps in a ones' period (the scout team, running the opponent's plays) count 0 (`rep_share`).
- Each part against the neutral week's, (mine + pad) / (neutral + pad), capped: reps pad 5, cap 1.2, half against his role's neutral reps and half against a starter's; installs, film, walkthroughs and study pad 10, cap 1.2; work pad 30, cap 1.15; the install's size pad 15, cap 2 (`against`).
- Days he misses: each counted day, chance clamp(0.012 + 0.008 × max(10 − work ethic, 0) − 0.012 × professionalism z, 0, 0.14); kept = 1 − missed / days (`miss_chance`, `read`).
- Own study = clamp(0.82 + 0.025 × (work ethic − 10) + 0.12 × professionalism z + era, 0.45, 1.15); era +0.08 in college and the NFL from 2011 (the tablet), +0.06 at a high school from 2012 (Hudl) (`own_study`). Engagement = clamp(0.88 + 0.018 × (work ethic − 10) + 0.07 × professionalism z + 0.03 × ambition z, 0.6, 1.08) (`engaged`).
- The mental side, game week: 0.35 installs + 0.25 film + 0.2 walkthroughs + 0.2 × (0.5 study + 0.5 own study). Out of a game week: 0.3 installs + 0.2 film + 0.1 walkthroughs + 0.4 × (0.25 study + 0.75 own study). m = that × engagement × the classroom share he is there for × kept (`read`).
- An install too full to take in: capacity = 1.15 + 0.008 × (football IQ − the level's norm) + 0.04 × min(years in the system, 3), − 0.2 if new; norms NFL 66, college 56, high school 44; took = clamp(capacity / max(install size ratio, 0.5), 0.4, 1) (`iq_norm`, `read`). Years: a high schooler's age − 14, college seasons, NFL seasons (`training/week.rs`, `monday`; `training/seat.rs`, `prep_detail`).
- Score, game week: (0.3 × reps × field share × kept + 0.7 × m) × (0.6 + 0.4 × took). Out of a game week: (0.4 × work × field share × kept + 0.6 × m) × (0.85 + 0.15 × took) (`read`).
- Words: Sharp 0.92 and up, Prepared 0.8, Patchy 0.66, Underprepared 0.5, Unprepared below; None in a week the staff may run nothing, before he arrived, or with no team week (`word_of`, `Prep::none`; `training/week.rs`, `monday`).
- Status and the share of the field work and classroom he is there for (`Status::there`): full and new 1 and 1; limited 0.55 and 1; rested 1 and 1 (his rest days are already out of his week); out 0 and 0.75; absent 0 and 0; away at another sport 1 and 1 (read against the share he agreed to, so the sport alone never makes him poorly prepared) (`read`).
- Who gets which status. Everyone else's players (`training/week.rs`, `monday`): out (a pro out 2+ weeks), absent (missed a voluntary week, `attends`), away (a high schooler in his other sport's season), else full; never limited, rested or new. College and high school game injuries end at the final whistle, so they never make a player out ([injuries.md](injuries.md)). Your players (`training/seat.rs`, `prep_detail`): hurt (a practice injury, the non-football list or a pro out 2+ weeks) is limited when his practice or non-football injury ends within 3 days of Monday, else out; then absent, away (high school), new (arrived in the last 14 days), rested (rest days in his plan in a week with field work), else full.
- An NFL injury heals on Tuesday, after Monday's pass, so a pro with one week left is read as practising (`out_this_week`: out_weeks > 1).
- Worked out from these formulas: an average player out hurt in a game week scores about 0.46 (Unprepared), and at best Patchy (about 0.68) with a heavy classroom week and top work habits, so a long injury rusts him (`read`).
- Recorded every Monday for every player: your team in `training/seat.rs`, `week`; everyone else in `training/week.rs`, `monday`. The last five words sit in `Train.prep`, three bits each; bit 15 marks a word read while he was out hurt, so a page later in the week reads his week as the pass did (`training/prep.rs`, `record`, `mark_out`, `was_out`, `words`; `training/mod.rs`, `Train`).
- Your staff's read of the word, on screens: exact in a week with a game plan test, otherwise 12% one word higher and 12% one lower (never past Sharp or below Unprepared) (`staff_read`). Your position coach leaves a note when a player is poor two weeks running, with the reason (`training/seat.rs`, `week`; `Prep::why`).

## Prep on game day (TR38)

- Each game player carries the factor of his latest word: None 1.0, Unprepared 1.33, Underprepared 1.2, Patchy 1.08, Prepared 0.975, Sharp 0.93 (`training/prep.rs`, `game_factor`; `game/roster.rs`, `prep_of`, `GamePlayer::from_player`, `refresh`). The college teams kept for the season refresh it every Monday (`training/week.rs`, `refresh_college_teams`).
- It multiplies his bust chance and, for linebackers and DBs on runs, his misfit chance before its clamp (`game/scrimmage.rs`, `run_snap`).
- It moves his read time by 0.1 × (factor − 1) seconds: +0.033 Unprepared, −0.007 Sharp (`run_snap`).
- A quarterback's time on each read × (1 + 0.3 × (factor − 1)): about 10% longer Unprepared (`game/scrimmage.rs`, `quarterback`).
- What it does not touch in the game: his ratings. Only these four checks read `GamePlayer.prep` (`game/scrimmage.rs`). Rust is separate: it moves the ratings themselves (see below).
- Practice battles read it too: your team's after this week's words are recorded, the AI's from last week's, since their battles run before the pass (`training/week.rs`, `monday`).
- Details of each check are on [match-engine.md](match-engine.md).

## Rust and slower growth (TR39, TR40)

- Rust starts only when this week and last week are both poor (Unprepared or Underprepared): +1.0 for an Unprepared week, +0.6 for an Underprepared one (`training/prep.rs`, `record`).
- It splits by what he lacked: reading gets add × (0.5 + 0.5 × mental share), technique add × (0.5 + 0.5 × (1 − mental share)); mental share = (1 − m)₊ / ((1 − m)₊ + (1 − reps or work)₊), 0.5 when he lacked neither (`record`, `read`).
- Each side is capped at 3 points and kept in tenths in `Train.rust` (`record`, `rust`).
- Won back each week: Patchy 0.3, Prepared 1.2, Sharp 1.5, a week with no prep 0.25; poor weeks win back nothing (`record`).
- Rust moves the ratings themselves: reading is play recognition and anticipation; technique is the two technique-family ratings his position weighs most. Winning back never lifts a rating past his ceiling (unless he is already past it); rust never takes a rating below 1 (`READING`, `technique`, `shift`).
- A position change gives all rust back first, since it took his old position's technique (`unrust`; `athlete.rs`, `set_position`).
- Slower growth: factor = 1 − 0.024 × (Unprepared weeks + 0.6 × Underprepared weeks) among the last five, on the technique, kicking and mental shares; five Unprepared weeks cost 12% (`growth_factor`, `slows`, `slow`).
- Test: five Unprepared weeks give 2.5-3 points of rust on each side and a growth factor of 0.85-0.9; three Prepared weeks win it all back (`tests/training.rs`, `a_run_of_poor_prep_rusts_and_comes_back`).

## Your team's week

- Planned week: this week's edits, else your template for the slot with the week's extras, else your staff's styled default with them; then fitted to the rules unless you pushed it; then fitted to the block's counts, with practices lost to a case taken off; then the weather's cuts (`training/seat.rs`, `planned`).
- Templates are checked before they save; a forbidden period can't be saved (`save_template`). This week's plan saves with breaches only with `push` (`save_this_week`).
- Your ratio per player is his own week (plan extras added, rest days and injury taken off) against the neutral week, at his absorb factor (`training/seat.rs`, `week`, `own_week`). A hurt player's week keeps every off-field period except mat drills and testing, plus 45 minutes of treatment on each day with any (`own_week`).
- A player absent from a voluntary week works at his own effort, as on his own time (`training/seat.rs`, `week`).
- New arrivals (TR8): your players' arrival dates are kept; one who arrived in the last 14 days reads as new. The first week with this list counts everyone as arrived a year ago (`SeatTraining.arrived`, `is_new`, `week`).

### Practice weather (TR15)

- Your team's town (the NFL club's city, the college's, the high school's), at 11:00 for the NFL, 9:00 in camp, 15:00 otherwise (`training/seat.rs`, `practice_town`, `practice_weather`; `calendar/weather.rs`, `weather_in`).
- Indoor field: every NFL club; FBS programs with facilities over 0.35, FCS over 0.7, the rest over 0.9; high schools with resources over 0.85 (`indoor_field`).
- Severe weather: inside, or the day's field work is lost. Lightning: inside, or a delay that takes 30 minutes off the longest field periods. Snow, under 20 F or wind over 30 mph: inside when it can, else outside as planned. Rain: inside half the time with an indoor field. Days with no staff field work, and game days, have no practice weather (`practice_weather`, `weather_cuts`).
- The cuts hit your week only, not the neutral week it is measured against (`planned`). AI teams have no practice weather.

### Testing days (TR5)

- On a testing day, and in your staff's first week with the players, the strength staff records each player's bench, squat and clean, reps at 225, the 40 and its 10-yard split, the short shuttle, the vertical and his weight; the last three days are kept (`training/seat.rs`, `week`, `TestDay`, `test_row`).
- The numbers are the combine's expected results for his ratings and body now with a test day's noise: reps sd 1.5, 40 sd 0.03 (clamped 4.2-6.5), split 0.375 × 40 − 0.15, shuttle sd 0.04, vertical sd 0.8 (`test_row`; `body.rs`, `Drills::expected`).
- The bench max reads his strength at 220 lb: 225 × (1 + reps at 220 lb / 30) × (weight / 220)^0.67, or a share of his weight when he can't rep 225; squat about 1.45 times it, clean about 0.88; proposed, not confirmed (`test_row`).

### What the week's other periods do (proposed, not confirmed)

All against your staff's neutral week, for your team only (`training/seat.rs`, `team_effects`, `limber`, `week`):

- Mat drills (TR6): past two mornings a week, each extra one is a −0.02 mood event for every player, shown as "the grind of the week: mat drills at dawn" (`team_effects`; `world/folk/talks.rs`, `Reason::Practice`, `Reason::words`).
- Routes on air (TR20): air = clamp(0.97 × air + 0.001 × extra minutes / 30, −0.05, 0.1), added to the chemistry of your quarterback, receivers, tight ends and backs when your quarterback starts (`team_effects`; `world/chemistry.rs`, `apply`).
- Outings and the leadership council (TR32, TR33): bonds += 0.01 × extra minutes / 60, clamped −0.15 to 0.25, reset at fall camp's friendship draw; friendships on your roster form × clamp(1 + bonds, 0.85, 1.25). Each player's mood also moves clamp(0.004 × extra minutes / 60, −0.01, 0.015) a week (`team_effects`; `world/folk/ties.rs`, `draw`, `day`).
- Study hall (TR14): each study-hall period more than the neutral week's moves a flagged player's GPA +0.003, each one fewer −0.003; flagged is under 2.3, or in his first college season; not in the NFL (`team_effects`). The practice screen's who's-where list flags a high schooler only under 2.0 (`crates/afm-game/src/lane_training/week.rs`).
- Mobility (TR23): limber = clamp(1 − 0.1 × (mobility minutes − neutral) / (neutral + 15), 0.9, 1.1); your college or high school practice injury chance × (0.77 + 0.23 × limber) and the hamstring weight × limber; your NFL club's practice risk × (0.77 + 0.23 × limber) (`limber`, `practice_injuries`, `week`).
- Nutrition (TR31): fuel = clamp(1 + 0.15 × (nutrition minutes − neutral) / (neutral + 30), 0.9, 1.15) on each body plan's weekly rate (`week`).
- Your NFL club's practice risk is your own week's: clamp(sqrt((full-contact minutes + 10) / (neutral + 10)) × ((field minutes + 30) / (neutral + 30))^0.3, 0.5, 2) × (0.77 + 0.23 × limber); it replaces your club's entry in `club_risk` (`training/seat.rs`, `week`, `my_risk`; `training/week.rs`, `monday`).

### Screens

- Commands: `practice`, `practice_week`, `practice_template`, `practice_plan`, `player_dev`, `plans`, `plans_set`, `squad`, `boards` (`crates/afm-game/src/lane_training.rs`).
- `practice` adds the week's frame: the title and the block's rule, your staff's reasons for the week, each day's clock (early work, classes, the day's start, kickoff), weather and place, the required and chosen days off, four cards (hours, contact, game prep minutes against your staff's, prep and effort), each player's prep row, the week's minutes by the nine shown families, the lift groups, body targets by group, the last testing day's numbers, who's where (rehab steps, new arrivals, other sports, study hall) and across the league (`lane_training.rs`, `practice`; `crates/afm-game/src/lane_training/week.rs`).
- A player's prep row: status, his work, installs, film, walkthroughs and own study against his role's usual, your staff's word, five weeks of words, his rust in points and what your staff says. On Monday before the pass it shows the live read; after it, the recorded word (`lane_training/week.rs`, `prep_json`, `says`).
- Across the league: up to two of this week's opponent's starters with 1.5+ points of rust, read straight from their rust, and up to three OTA absences in the news of the last 14 days (`lane_training/week.rs`, `league`; `lane_training.rs`, `practice`).
- The who's-where list flags high schoolers under a 2.0 GPA; the study-hall effect itself reads 2.3 at every level (`lane_training/week.rs`, `wheres`; `training/seat.rs`, `team_effects`).
- Game day's hub has "Your players' prep": your two-deep without the hurt-out players, counted by word, and up to ten Unprepared, Underprepared or Patchy players, worst first (`lane_training.rs`, `prep_report`; `crates/afm-game/src/lane_gameday.rs`, `hub`).

## Individual plans

Set per player through `plans_set` / `set_plan` (`crates/afm-game/src/lane_training.rs`; `training/seat.rs`, `Plan`).

- Focus: one attribute grows up to 1 + 0.5 × take; others in its group give up 3.5% × take; take = clamp(0.6 + 0.02 × coachability, 0.6, 1.0) (`development.rs`, `GrowthCtx::rate`; `training/mod.rs`, `Train::ctx`).
- Habit: a tendency toward a target, `habit_rate / 30` a week times his obey chance, only on staff weeks and when not hurt (a practice injury, the non-football list, or a pro out 2+ weeks). `habit_rate` = clamp(0.375 × (teaching − 8) × (0.45 + 0.037 × coachability), 0, 3.4) × max(settle, 0.35), settle = clamp((26.5 − age) / 4.5, 0.12, 1) (`training/seat.rs`, `week`, `habit_rate`, `staff_teaching`; `tendencies.rs`, `obey_chance`).
- Body: a weight target, 1.5 lb a week up or 1.2 down, × clamp(0.7 + 0.03 × work ethic, 0.7, 1.3) × fuel (nutrition, 0.9-1.15). Each pound: strength +0.05, anchor +0.04, speed −0.03, acceleration −0.02, agility −0.03, stamina −0.02, ceilings too. Emphasis moves the week's ratio: strength ×1.08 and speed ×0.95, or the reverse, or lean 0.98/1.02 (`training/seat.rs`, `week`, `body_change`).
- Position move: progress + clamp(9 + 0.1 × (football IQ − 60), 5, 14) a week with field work; at 100 he is moved, his rust given back first (`training/seat.rs`, `week`; `athlete.rs`, `set_position`).
- Extra work (film 20 min, individual 15, lifting 20 on each field day): he does it with chance clamp(0.35 + 0.04 × work ethic + 0.1 × professionalism z, 0.1, 0.98) (`training/seat.rs`, `does_extra`, `own_week`). The extra film counts in his prep (`prep_detail`).
- Rest: up to 3 practice days a week off the field (`own_week`; capped in `crates/afm-game/src/lane_training.rs`, `set_plan`). He reads as rested in a week with field work (`prep_detail`).

## Who teaches

- The year's coaching input (set at the checkpoint, read by every week's landing) (`world/season.rs`, `develop`):
  - College: 0.2 × head coach + 0.35 × coordinator + 0.3 × position coach + 0.15 × (6 + 12 × facilities), each coach's `teaching(group)` on 1-20; missing coaches fall back up the chain.
  - High school: 0.6 × head coach's teaching + 0.4 × staff (your school's real assistants, else 4 + 12 × resources).
  - NFL: the club's `coaching` = 0.5 × head coach value + 0.25 × each coordinator's side quality (`world/staff.rs`, `set_nfl_coaching`, `side_quality`; used in `world/pro/mod.rs`).
- Then `coaching_mult = 0.75 + 0.025 × coaching`, `boost = 1 + (mult − 1) × (0.6 + 0.04 × coachability)` (`development.rs`, `develop_season`).
- The strength staff and professionalism make `physical` (−1..1, up to 8% on physical growth): 0.6 × strength staff z + 0.4 × professionalism z; high school 0.4 × professionalism − 0.2 (`world/people.rs`, `support_ctx`; `development.rs`, `develop_season`).
- Habits use the position coach's teaching for your seat, the head coach's at an NFL club (`training/seat.rs`, `staff_teaching`); the screen shows who teaches in words (`crates/afm-game/src/lane_training.rs`, `teacher`, `teaching_words`).

## Load

- Load per minute by kind × contact (air 0.85 to live 1.35) × tempo (walk 0.3, fast 1.35); a game day is 90 units (20 for young players) (`training/work.rs`, `load_per_min`, `day_load`). Examples: scrimmage and mat drills 1.1, inside run and conditioning 1.0, testing 0.6, routes on air and ball security 0.5, kicking stations 0.3; treatment −0.2 and mobility −0.05 take a little off; the classroom and outings 0. A day's load is never below 0 (`load_per_min`, `day_load`).
- Stored in tens: acute = units / 10, chronic = 0.75 × chronic + 0.25 × acute (about four weeks) (`training/mod.rs`, `add_load`).
- Spike = acute / chronic; 1.6 if chronic is under 3 and acute over 6 (`Train::spike`). Trainers' note above 1.45 (`training/seat.rs`, `week`). Screen words in `crates/afm-game/src/lane_training.rs`, `load_words`.
- A week with no team plan (own time, no team, a pro out 2+ weeks) loads 150 units in the world pass (`training/week.rs`, `monday`).
- A player absent from a voluntary week still takes the team week's load, in the world pass and on your team (`training/week.rs`, `monday`; `training/seat.rs`, `week`).
- What load does: practice injury odds through the spike (see [injuries.md](injuries.md)). It does not slow growth.

## Practice battles and grades

- Team, 7-on-7, inside run, one-on-ones, scrimmage and situation periods (not walk tempo) give snaps: 1.5 a minute for team, inside run and situations, 1.8 for 7-on-7, 2.5 one-on-ones, 1.1 scrimmage, 1.2 young; fast ×1.25 (`training/plan.rs`, `Kind::snaps_per_min`; `training/practice.rs`, `team_snaps`).
- Snaps run on the match engine, the team against itself, with the engine's fatigue and injuries cleared each snap (`game/flow.rs`, `practice_snaps`). Ones' periods go 80/20 to ones and twos, all-periods 60/40 (`training/practice.rs`, `plan_snaps`).
- Your team runs up to 150 snaps a week (`seat_battles`); AI college and NFL a sample (`ai_cap`, `world_battles`).
- Grade = 60 + 95 × avg × sqrt(n / (n + 25)), clamped 20-99.9 (`grade`). What staff see adds 0.25 × (10 − consistency) + 0.15 × (temperament − 10) + 2 × mood (`seen`). The running grade is 0.55 old + 0.45 week (`blend`).
- Other high schools: 60 + 0.55 × (his overall − his position group's mean at his school) + noise of 6 × sqrt(12 / half the snaps) (`school_grades`, `FIT_SLOPE`, `FIT_SPREAD`).
- Impression on the depth chart: (grade − 60) / 10 × 1.2, ±3 (`training/mod.rs`, `impression`; `game/roster.rs`).
- Your week also logs the five best and worst graders, one-on-one battles and camp news at NFL clubs and FBS programs (`training/practice.rs`, `battles`; `training/seat.rs`, `camp_news`).

## Game day from your week

- For your team only (`training/seat.rs`, `prepare`, called from `world/season.rs`, `world/hs_year.rs`, `world/pro/games.rs`):
  - Your players hurt in practice or on the non-football list sit: `prepare` drops them, and the game builders leave them out (`SeatTraining::hurt_out`; `world/college.rs`, `play_day`, `seat_setup`; `world/hs_year.rs`, `can_play`; `world/pro/games.rs`, `actives`, `preseason_side`).
  - Kickoff energy: clamp(1 − 0.06 × (late load / neutral late load − 1), 0.85, 1.0), late load = the three days before the game (`training/work.rs`, `late_load`, `freshness`).
  - Install: the staff's install × sqrt(your prep minutes / neutral prep minutes) clamped 0.75-1.15, then 0.3-1.0 (`training/work.rs`, `prep_minutes`).
  - Prep minutes weigh each minute by how much it readies this week's plan (`prep_weight`): a team period on this week's install 1.2 (base work 0.5), situations 1.0, 7-on-7 0.6 (base 0.3), the mock game 1.0, a jog-through 0.8, the hotel walkthrough 0.6, installs 1.0, the opponent's film and cut-ups 0.8, practice tape 0.3, unit and position meetings 0.5 (a team meeting 0.2), the plan test 0.6, VR 0.6, signals 0.5, home study 0.3. The new system's install and film, self-scout and last game's tape count 0 (bug T2).
  - Playbook installs: team, situations, 7-on-7 and walkthrough minutes × 1.5 reps split over the call sheet's families and your designs; level = 1 − e^(−reps/60). A designed play goes on the sheet only at 0.35 or more (`training/seat.rs`, `install_week`, `prepare`).
  - Then your guidelines and the week's plan (`world/guide.rs`, `seat_apply`; `world/plans.rs`, `prepare`): the call sheets your coordinators drafted for this game (15 boxes, `SHEET`; each split by hash, `game/sheet.rs`, `key`), extra film tonight +0.04 and an extra walkthrough +0.03 on the player's knowledge of every family for this game, base downs only, snap limits, the line rotation, the packages' 2nd and 3rd strings, and the instructions for this opponent (taken back after it). How the engine uses them is on [play-calling.md](play-calling.md).
  - The packages' boards include Jumbo (three tight ends outside the goal line) (`training/seat.rs`, `Board::Jumbo`).
  - Crowd noise (TR28): noise = clamp(1 + 0.075 × (the neutral week's loud days − yours), 0.9, 1.15), against the neutral game week before fitting; it multiplies the home crowd against your offense on the road (`week_habits`; `game/flow.rs`, `home_crowd`).
  - Rules clinic (TR29): rules = clamp(1 − 0.1 × (clinic minutes − neutral) / (neutral + 10), 0.9, 1.08); it multiplies the crew's pre-snap foul rate for your side (`week_habits`; `game/flow.rs`, `pre_snap_foul`). The NFL's neutral week has no clinic, so 10 minutes there already gives 0.9.
  - AI teams keep noise and rules at 1 (`game/roster.rs`, `Prep::default`).
- Every player's prep word reaches the game for every team (see "Prep on game day").

## Offseason and own time

- Own effort: clamp(1 + 0.12 × professionalism z + 0.06 × ambition z + 0.01 × (work ethic − 10) − 0.08 if just paid + 0.06 in a contract year, 0.55, 1.45). It is the ratio for every family in an own-time week (`training/offseason.rs`, `own_effort`; `training/week.rs`, `monday`).
- Voluntary weeks: he comes with chance clamp(0.9 + 0.015 × (work ethic − 10) − 0.04 × max(age − 29, 0), 0.45, 0.99), one draw a week from the week's prep key, for your team and the world alike; skipping uses own effort and reads Absent, an Unprepared week (`training/week.rs`, `attends`, `monday`; `training/seat.rs`, `prep_detail`).
- In an OTA week the three best absent pro starters at 82 overall or more (your own players excepted) make small NFL news: "... is not at voluntary OTAs with the ... this week." (`training/week.rs`, `absent_news`).
- Pros out 2+ weeks: ratio 0.6/0.4/0.4/0.1/0/0.2/0.5 at 70% attendance (`training/week.rs`, `monday`).
- High schoolers' other sport: about 42% (specialists 20%) by a hash of his id, picked by body (track, basketball, wrestling, baseball). In its season and outside football's camp and season, it multiplies the week (e.g. basketball strength 0.65, speed 1.06, conditioning 1.12) and he reads as away for prep (`training/offseason.rs`, `other_sport`, `Sport::ratio`, `sport_ratio`).
- Your team only: signs of offseason work through the facility, social posts, strength coach, trainers, teammates and agents, each with its own bias (`training/offseason.rs`, `sign`; `training/seat.rs`, `offseason_week`); report day weigh-in and conditioning test (fail chance clamp(base × e^(−4(effort−1)) × e^(−0.04(stamina−70)), 0.003, 0.6), base 3% NFL, 2.5% college, 2% high school; failing is 3-10 days on the non-football list) (`offseason.rs`, `fail_chance`; `seat.rs`, `report_day`).
- High school heat (your school, July to September practice days with a 3 pm heat index of 104 F or more): with a state heat rule only a note is written (the week's work is not cut in code); without one each player risks heat illness, 0.0033 × 1.8 if 260 lb or more × max(1.2 − 0.004 × stamina, 0.6) × (1 + (index − 104) / 10), out 3 days (`training/seat.rs`, `heat_week`).

## Breaking the rules

- A pushed week is found out with chance base × (1 + 0.25 × (breaches − 1)) × 1.6 if someone got hurt, capped 0.9; base NFL 0.3, college 0.2, high school 0.15 (`training/offseason.rs`, `found_out`; `training/seat.rs`, `practice_case`).
- Penalties: NFL a week of next year's OTAs and fines ($200k club, $100k coach; doubled again); NCAA Level III then Level II with practices cut; state warning then probation. Practices lost in the next block of that kind: NFL 3 (1 in season), college 3 then 6, high school 2 (`training/offseason.rs`, `penalty`).

## Code against the docs

- `docs/world-model.md` (lane 4's picks) says a poorly prepared player reads about a tenth of a second late per word below prepared and a quarterback's reads run up to 30% slower. The code adds 0.1 × (factor − 1) seconds (0.033 s for Unprepared) and × (1 + 0.3 × (factor − 1)) on each read (about 10%) (`game/scrimmage.rs`, `run_snap`, `quarterback`).

## Where the knobs are

- `training/rules.rs`: `block_share` (the year's split), `book`, `hs_contact`, `block_limits`.
- `training/plan.rs`: `neutral` (templates), `styled` (style moves), `available` (kinds by level and era), `extras` and `testing_day`, `Kind::counted`, `Kind::snaps_per_min`.
- `training/work.rs`: `per_minute`, `contact_factor`, `reps`, `DAY_CAP`, `absorb_factor`, `PRACTICE_DRIVEN`, `PLAN_REACH`, `load_per_min`, `freshness`, `prep_minutes`, `prep_weight`.
- `training/prep.rs`: `Status::there`, the pads and caps in `against`, `iq_norm` (66/56/44), `own_study`, `engaged`, `miss_chance`, the weights in `read`, `word_of` (0.92/0.8/0.66/0.5), `game_factor`, `growth_factor` (0.024, 0.6), the rust steps in `record`, `staff_read` (12%/12%).
- `training/practice.rs`: `ai_cap`, `plan_snaps` splits, `grade`, `seen`, `blend`, `FIT_SLOPE`, `FIT_SPREAD`.
- `training/mod.rs`: `impression`, `owe` threshold (167), `GrowthYear` packing.
- `training/offseason.rs`: `own_effort`, `Sport::ratio`, `other_sport` (42%), `fail_chance`, `found_out`, `penalty`.
- `training/seat.rs`: `habit_rate`, `body_change`, body rates and fuel, position-move rate, extra-work chance, `heat_week`, install mastery (60 reps), `indoor_field`, `practice_weather`, `test_row`, `team_effects` (mats, air, bonds, room, study hall), `limber`, `week_habits`.
- `training/week.rs`: `attends`, `bucket` thresholds (8, 13), hurt ratio, `absent_news` (82 overall, three a week).
- `game/scrimmage.rs`: the prep terms in `run_snap` (0.1 on the read) and `quarterback` (0.3 on each read).
- `world/mod.rs`, `options.yearly_growth`: turns the weekly landing off.

## Related pages

- [development.md](development.md): the yearly step, rates, ceilings and how coaching, snaps and age set the year.
- [match-engine.md](match-engine.md): busts, misfits, reads, the crowd and pre-snap fouls that prep moves.
- [injuries.md](injuries.md): practice and game injuries, recovery, carry-over.
- [people.md](people.md): mood, friendships and chemistry the week moves.
- [tendencies.md](tendencies.md): habits, styles, obedience.
- [staff-and-coaches.md](staff-and-coaches.md): coach teaching ratings and support staffs.
- [body.md](body.md): weight, the body as built and the combine fit behind testing days.
- [play-calling.md](play-calling.md): call sheets and installs on game day.
