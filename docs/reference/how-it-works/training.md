# Training: how it works today

Checked against main at 94a6553 on 2026-10-25.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`. A training build lane may be changing this system on another branch; this page is `main` only.

## Quick answers

- Does training decide how much a player grows? No. The yearly step still decides the year's growth rate; training only decides when it lands and moves each family by at most about 15% a week (`training/mod.rs` module notes; `training/work.rs`, `share`, `PLAN_REACH` 0.15).
- When does training run? Every Monday, for every team in the world (`world/clock.rs`, the day loop, `training::week::monday`). Off when `options.yearly_growth` is true (default false) (`training/week.rs`, `monday`; `world/mod.rs`).
- What are the families? Strength, speed, conditioning, technique, contact, kicking, mental (`training/mod.rs`, `Family`, `family_of`).
- Does a plan change potential? The practice plan doesn't: shares only land growth toward the existing ceilings (`development.rs`, `grow_share`). A body plan's weight change moves six attributes and their ceilings (`training/seat.rs`, `body_change`).
- Does the AI follow the rules? Always. Its weeks pass through `fit` and `fit_block` (`training/plan.rs`). You can push a week past them on purpose (`training/seat.rs`, `save_this_week` with `push`).
- NCAA hour limits? 20 countable hours a week, 4 a day, one day off in season, spring and bowl weeks from 1991; 8 hours a week out of season with up to 2 of film from 2013 (`training/rules.rs`, `book`).
- NFL CBA limits? From 2011: phase 1 strength only, phase 2 no offense against defense, OTAs no pads and no live contact, 4 practice days a week; camp 4 hours on the field; season 3 hours, 14 padded practices (`training/rules.rs`, `book`, `block_limits`).
- High school contact limits? California from 2015 (two full-contact days, 90 min each, none out of season), Texas from 2017 (90 min a week in season), other states the NFHS 2014 guidance (90 min a week in season) (`training/rules.rs`, `hs_contact`).
- Does coach quality change what a practice teaches? No. Coach quality enters through the yearly step's coaching boost, stored in the year's growth (`development.rs`, `develop_season`; `training/mod.rs`, `GrowthYear.coach`). Practice work reads only minutes, kind, contact, tempo and reps (`training/work.rs`, `period_work`).
- Does a player's plan focus work for AI teams? No. Only your seat sets a focus; AI players land with no focus (`training/week.rs`, `monday`, `settle(.., None, ..)`; `world/seat.rs`, `focus_by_person`).
- Is the staff's suggested focus applied when you set none? No. Growth reads only a focus you set. The screens label the staff's pick as the focus "by staff" (`crates/afm-game/src/lane_training.rs`, `plan_json`), but `focus_by_person` returns only your picks (`world/seat.rs`).
- Do practice reps move the depth chart? Yes, a little: the practice grade adds up to ±3 rating points in the staff's depth order (`training/mod.rs`, `impression`; `game/roster.rs`, `staff_depth_chart`).
- Do AI teams practise on the engine? College and NFL teams run a sample of snaps each week (camp 24, spring 16, season 8, OTAs 6); high schools other than yours get a fitted model instead (`training/practice.rs`, `ai_cap`, `world_battles`, `school_grades`).
- Do practices injure AI college and high school players? No. Practice injuries run for your college or high school team only, and for every NFL club (`training/seat.rs`, `week`, `practice_injuries`; `world/pro/games.rs`, `practice_injuries`). See [injuries.md](injuries.md).
- Does a heavy week tire the team on game day? Yes, for your team only: kickoff energy down to 0.85 (`training/work.rs`, `freshness`; `training/seat.rs`, `prepare`; `game/flow.rs`, side energy).

## How growth lands (the hand-off from development)

- At each checkpoint the yearly step works out the year as before (ceilings, form, injuries, ageing). With `defer` on, it skips the growth itself for every attribute his position uses that is not declining, and stores the year's rate in `player.train` (`development.rs`, `develop_season`, `ctx.defer`; `training/mod.rs`, `Train::open`, `defers`).
- `defer` is `!options.yearly_growth`, set for college and high school in `world/season.rs`, `develop`, and for pros in `world/pro/mod.rs`, `camp` (the pros' yearly checkpoint).
- What is stored (`training/mod.rs`, `GrowthYear`; bloom and position on `Train`): age, flags (high school, college, redshirt), coaching boost (`0.5 + v/250`), snaps multiplier (`v/200`), morale multiplier (`0.8 + v/600`), physical multiplier (`0.9 + v/1200`), form times injury multiplier (`v/4000`), his bloom and his checkpoint position.
- Attributes his position does not use (a receiver's tackling) and ones past their growth window or declining settle at the checkpoint as before (`training/mod.rs`, `deferred`).
- Each week the level's calendar gives each family a base share (`training/rules.rs`, `weekly_base`: the block's share over its weeks). `cover` caps the year at 1.0 per family (`training/mod.rs`, `cover`).
- The share is moved by the week's work: `s = base × (1 + 0.15 × tanh(raw / 0.15)) × attend`, `raw = PRACTICE_DRIVEN[f] × (clamp(r, 0, 1.8) − 1)`, where `r` is the week's work against the neutral week's (`training/work.rs`, `share`, `ratio`).
- `PRACTICE_DRIVEN` per family: strength 0.55, speed 0.3, conditioning 0.7, technique 0.5, contact 0.6, kicking 0.5, mental 0.35 (`training/work.rs`).
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
- Each block holds a share of each family's year; each column sums to 1 over the year (test `each_family_adds_to_one_year`) (`training/rules.rs`, `block_share`). Examples, college (strength, speed, conditioning, technique, contact, kicking, mental): winter 0.35/0.35/0.25/0.03/0/0.10/0.07; camp 0.10/0.10/0.15/0.25/0.35/0.20/0.20; season 0.12/0.05/0.15/0.30/0.30/0.45/0.45. Division III moves spring's body share to winter and its football share to camp.
- Blocks the staff can't run (`Quiet`, `Break`, NFL winter, and college summer before 2013) are the players' own time (`training/rules.rs`, `Block::own`, `book`).

## The rule books

`book(tier, block, season, day, state, heat_rule)` returns what a day allows: coaches allowed, on-field football, offense against defense, most gear and contact, countable minutes a week and day, on-field minutes a day, film a week, days off, practice days, full-contact limits, back-to-back full contact, two-a-days, voluntary (`training/rules.rs`, `book`, `Book`).

- College camp: acclimatization 5 days from 2003, 7 from 2021 (helmets and bags the first 2 days, shells and control after); two-a-days allowed after it until 2017; from 2021 75 minutes of full contact a session (`book`).
- College spring: 15 practices, 12 contact, 3 scrimmages; from 2022 no full contact on back-to-back days and 75 minutes a session; no two-a-days (`book`, `block_limits`).
- College camp counts from 2021: 25 practices, 18 contact, 9 padded, 2 scrimmages; before, 29 practices. Bowl practice: 15 (`block_limits`).
- NFL OTAs from 2011: 13 practices, none with contact or pads. Camp from 2020: 16 padded, 5-day acclimation. Season from 2011: 14 padded (`block_limits`, `book`).
- NFL phases 1-3 are voluntary (minicamp included in code: `voluntary` is true for the whole OTA block) (`book`).
- High school camp from 2011 or with a state heat rule: helmets 2 days, shells to day 5, 3 hours on the field, no two-a-days the first 5 days. Season: 3 hours on the field, 1 day off, no two-a-days. Summer: helmets only, voluntary (`book`).
- `check_day` and `check_week` list every breach in words; `Counts::over` checks a block's practice counts (`training/rules.rs`).

## The practice day and week

- A practice is a list of periods: kind (17 kinds), minutes, gear (none, helmets, shells, pads), contact (air, bags, control, thud, live), tempo (walk, normal, fast), who (all, ones, young) and the situation (`training/plan.rs`, `Period`, `Kind`, `Gear`, `Contact`, `Tempo`, `Who`, `SITUATIONS`).
- Gear caps contact: helmets bags, shells thud, pads live (`Gear::most_contact`). Thud and live are full contact (`Contact::full`).
- NCAA countable minutes: everything but treatment, plus 180 for a game (`Day::countable`).
- A week is seven days from a template by slot: game, bye, camp, spring, winter, summer, bowl, phase 1-3, own (`training/plan.rs`, `Slot`, `Slot::of`). Game weeks count back from the game, so a Thursday game shifts the template; a long week fills from the bye template (`week_from`).
- Neutral templates per level and slot are the average staff's week (`training/plan.rs`, `neutral`). Example college game week: Sunday recovery, film and a young players' period; Monday off; Tuesday padded with thud; Wednesday padded; Thursday shells; Friday walkthrough; Saturday game.

## The time budget: what a minute works

- Work per minute by kind and family (`training/work.rs`, `per_minute`). Examples: individual 1.0 technique, 0.35 contact; inside run 1.0 contact, 0.45 technique, 0.4 mental; weight room 1.0 strength; film 0.9 mental; conditioning 1.2 conditioning; speed 1.0 speed. Specialists have their own table (individual and special teams 1.0 kicking).
- Contact column × contact factor: air 0, bags 0.35, control 0.65, thud 0.9, live 1.0 (`contact_factor`).
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
- Every AI week is fitted to the day's books, then to the block's counts left (earlier weeks of the block rebuilt the same way) (`training/week.rs`, `staff_week`; `training/plan.rs`, `fit`, `fit_block`).
- AI staffs leave reasons from their styles for the screens (`training/offseason.rs`, `staff_reasons`).

## Your team's week

- Planned week: this week's edits, else your template for the slot, else your staff's styled default; then fitted to the rules unless you pushed it; then fitted to the block's counts, with practices lost to a case taken off (`training/seat.rs`, `planned`).
- Templates are checked before they save; a forbidden period can't be saved (`save_template`). This week's plan saves with breaches only with `push` (`save_this_week`).
- Your ratio per player is his own week (plan extras added, rest days and injury taken off) against the neutral week, at his absorb factor (`training/seat.rs`, `week`, `own_week`).
- Screens: `crates/afm-game/src/lane_training.rs` commands `practice`, `practice_week`, `practice_template`, `practice_plan`, `player_dev`, `plans`, `plans_set`, `squad`, `boards`.

## Individual plans

Set per player through `plans_set` / `set_plan` (`crates/afm-game/src/lane_training.rs`; `training/seat.rs`, `Plan`).

- Focus: one attribute grows up to 1 + 0.5 × take; others in its group give up 3.5% × take; take = clamp(0.6 + 0.02 × coachability, 0.6, 1.0) (`development.rs`, `GrowthCtx::rate`; `training/mod.rs`, `Train::ctx`).
- Habit: a tendency toward a target, `habit_rate / 30` a week times his obey chance, only on staff weeks and when healthy. `habit_rate` = clamp(0.375 × (teaching − 8) × (0.45 + 0.037 × coachability), 0, 3.4) × max(settle, 0.35), settle = clamp((26.5 − age) / 4.5, 0.12, 1) (`training/seat.rs`, `week`, `habit_rate`, `staff_teaching`; `tendencies.rs`, `obey_chance`).
- Body: a weight target, 1.5 lb a week up or 1.2 down, × clamp(0.7 + 0.03 × work ethic, 0.7, 1.3). Each pound: strength +0.05, anchor +0.04, speed −0.03, acceleration −0.02, agility −0.03, stamina −0.02, ceilings too. Emphasis moves the week's ratio: strength ×1.08 and speed ×0.95, or the reverse, or lean 0.98/1.02 (`training/seat.rs`, `week`, `body_change`).
- Position move: progress + clamp(9 + 0.1 × (football IQ − 60), 5, 14) a week with field work; at 100 he is moved (`training/seat.rs`, `week`; `athlete.rs`, `set_position`).
- Extra work (film 20 min, individual 15, lifting 20 on each field day): he does it with chance clamp(0.35 + 0.04 × work ethic + 0.1 × professionalism z, 0.1, 0.98) (`training/seat.rs`, `week`, `own_week`).
- Rest: up to 3 practice days a week off the field (`own_week`; capped in `crates/afm-game/src/lane_training.rs`, `set_plan`).

## Who teaches

- The year's coaching input (set at the checkpoint, read by every week's landing) (`world/season.rs`, `develop`):
  - College: 0.2 × head coach + 0.35 × coordinator + 0.3 × position coach + 0.15 × (6 + 12 × facilities), each coach's `teaching(group)` on 1-20; missing coaches fall back up the chain.
  - High school: 0.6 × head coach's teaching + 0.4 × staff (your school's real assistants, else 4 + 12 × resources).
  - NFL: the club's `coaching` = 0.5 × head coach value + 0.25 × each coordinator's side quality (`world/staff.rs`, `set_nfl_coaching`, `side_quality`; used in `world/pro/mod.rs`).
- Then `coaching_mult = 0.75 + 0.025 × coaching`, `boost = 1 + (mult − 1) × (0.6 + 0.04 × coachability)` (`development.rs`, `develop_season`).
- The strength staff and professionalism make `physical` (−1..1, up to 8% on physical growth): 0.6 × strength staff z + 0.4 × professionalism z; high school 0.4 × professionalism − 0.2 (`world/people.rs`, `support_ctx`; `development.rs`, `develop_season`).
- Habits use the position coach's teaching for your seat, the head coach's at an NFL club (`training/seat.rs`, `staff_teaching`); the screen shows who teaches in words (`crates/afm-game/src/lane_training.rs`, `teacher`, `teaching_words`).

## Load

- Load per minute by kind × contact (air 0.85 to live 1.35) × tempo (walk 0.3, fast 1.35); a game day is 90 units (20 for young players) (`training/work.rs`, `load_per_min`, `day_load`).
- Stored in tens: acute = units / 10, chronic = 0.75 × chronic + 0.25 × acute (about four weeks) (`training/mod.rs`, `add_load`).
- Spike = acute / chronic; 1.6 if chronic is under 3 and acute over 6 (`Train::spike`). Trainers' note above 1.45 (`training/seat.rs`, `week`). Screen words in `crates/afm-game/src/lane_training.rs`, `load_words`.
- A week with no team plan (own time, no team, a pro out hurt) loads 150 units in the world pass (`training/week.rs`, `monday`).
- What load does: practice injury odds through the spike (see [injuries.md](injuries.md)). It does not slow growth.

## Practice battles and grades

- Team, 7-on-7, inside run, one-on-ones, scrimmage and situation periods (not walk tempo) give snaps: 1.5 a minute for team, inside run and situations, 1.8 for 7-on-7, 2.5 one-on-ones, 1.1 scrimmage, 1.2 young; fast ×1.25 (`training/plan.rs`, `Kind::snaps_per_min`; `training/practice.rs`, `team_snaps`).
- Snaps run on the match engine, the team against itself, with the engine's fatigue and injuries cleared each snap (`game/flow.rs`, `practice_snaps`). Ones' periods go 80/20 to ones and twos, all-periods 60/40 (`training/practice.rs`, `plan_snaps`).
- Your team runs up to 150 snaps a week (`seat_battles`); AI college and NFL a sample (`ai_cap`, `world_battles`).
- Grade = 60 + 95 × avg × sqrt(n / (n + 25)), clamped 20-99.9 (`grade`). What staff see adds 0.25 × (10 − consistency) + 0.15 × (temperament − 10) + 2 × mood (`seen`). The running grade is 0.55 old + 0.45 week (`blend`).
- Other high schools: 60 + 0.55 × (his overall − his position group's mean at his school) + noise of 6 × sqrt(12 / half the snaps) (`school_grades`, `FIT_SLOPE`, `FIT_SPREAD`).
- Impression on the depth chart: (grade − 60) / 10 × 1.2, ±3 (`training/mod.rs`, `impression`; `game/roster.rs`).
- Your week also logs the five best and worst graders, one-on-one battles and camp news at NFL clubs and FBS programs (`training/practice.rs`, `battles`; `training/seat.rs`, `camp_news`).

## Game prep

- For your team only (`training/seat.rs`, `prepare`, called from `world/season.rs`, `world/hs_year.rs`, `world/pro/games.rs`):
  - Players hurt in practice sit.
  - Kickoff energy: clamp(1 − 0.06 × (late load / neutral late load − 1), 0.85, 1.0), late load = the three days before the game (`training/work.rs`, `late_load`, `freshness`).
  - Install: the staff's install × sqrt(your prep minutes / neutral prep minutes) clamped 0.75-1.15, then 0.3-1.0 (`training/work.rs`, `prep_minutes`).
  - Playbook installs: team, situations, 7-on-7 and walkthrough minutes × 1.5 reps split over the call sheet's families and your designs; level = 1 − e^(−reps/60). A designed play goes on the sheet only at 0.35 or more (`training/seat.rs`, `install_week`, `prepare`).

## Offseason and own time

- Own effort: clamp(1 + 0.12 × professionalism z + 0.06 × ambition z + 0.01 × (work ethic − 10) − 0.08 if just paid + 0.06 in a contract year, 0.55, 1.45). It is the ratio for every family in an own-time week (`training/offseason.rs`, `own_effort`; `training/week.rs`, `monday`).
- Voluntary weeks: he comes with chance clamp(0.9 + 0.015 × (work ethic − 10) − 0.04 × max(age − 29, 0), 0.45, 0.99); skipping uses own effort (`training/week.rs`, `attends`).
- Pros out hurt: ratio 0.6/0.4/0.4/0.1/0/0.2/0.5 at 70% attendance (`training/week.rs`, `monday`).
- High schoolers' other sport: about 42% (specialists 20%) by a hash of his id, picked by body (track, basketball, wrestling, baseball). In its season and outside football's camp and season, it multiplies the week (e.g. basketball strength 0.65, speed 1.06, conditioning 1.12) (`training/offseason.rs`, `other_sport`, `Sport::ratio`, `sport_ratio`).
- Your team only: signs of offseason work through the facility, social posts, strength coach, trainers, teammates and agents, each with its own bias (`training/offseason.rs`, `sign`; `training/seat.rs`, `offseason_week`); report day weigh-in and conditioning test (fail chance clamp(base × e^(−4(effort−1)) × e^(−0.04(stamina−70)), 0.003, 0.6), base 3% NFL, 2.5% college, 2% high school; failing is 3-10 days on the non-football list) (`offseason.rs`, `fail_chance`; `seat.rs`, `report_day`).
- High school heat (your school, July to September practice days with a 3 pm heat index of 104 F or more): with a state heat rule only a note is written (the week's work is not cut in code); without one each player risks heat illness, 0.0033 × 1.8 if 260 lb or more × max(1.2 − 0.004 × stamina, 0.6) × (1 + (index − 104) / 10), out 3 days (`training/seat.rs`, `heat_week`).

## Breaking the rules

- A pushed week is found out with chance base × (1 + 0.25 × (breaches − 1)) × 1.6 if someone got hurt, capped 0.9; base NFL 0.3, college 0.2, high school 0.15 (`training/offseason.rs`, `found_out`; `training/seat.rs`, `practice_case`).
- Penalties: NFL a week of next year's OTAs and fines ($200k club, $100k coach; doubled again); NCAA Level III then Level II with practices cut; state warning then probation. Practices lost in the next block of that kind: NFL 3 (1 in season), college 3 then 6, high school 2 (`training/offseason.rs`, `penalty`).

## Where the knobs are

- `training/rules.rs`: `block_share` (the year's split), `book`, `hs_contact`, `block_limits`.
- `training/plan.rs`: `neutral` (templates), `styled` (style moves), `Kind::snaps_per_min`.
- `training/work.rs`: `per_minute`, `contact_factor`, `reps`, `DAY_CAP`, `absorb_factor`, `PRACTICE_DRIVEN`, `PLAN_REACH`, `load_per_min`, `freshness`, `prep_minutes`.
- `training/practice.rs`: `ai_cap`, `plan_snaps` splits, `grade`, `seen`, `blend`, `FIT_SLOPE`, `FIT_SPREAD`.
- `training/mod.rs`: `impression`, `owe` threshold (167), `GrowthYear` packing.
- `training/offseason.rs`: `own_effort`, `Sport::ratio`, `other_sport` (42%), `fail_chance`, `found_out`, `penalty`.
- `training/seat.rs`: `habit_rate`, `body_change`, body rates, position-move rate, extra-work chance, `heat_week`, install mastery (60 reps).
- `training/week.rs`: `attends`, `bucket` thresholds (8, 13), hurt ratio.
- `world/mod.rs`, `options.yearly_growth`: turns the weekly landing off.

## Related pages

- [development.md](development.md): the yearly step, rates, ceilings and how coaching, snaps and age set the year.
- [injuries.md](injuries.md): practice and game injuries, recovery, carry-over.
- [tendencies.md](tendencies.md): habits, styles, obedience.
- [staff-and-coaches.md](staff-and-coaches.md): coach teaching ratings and support staffs.
- [body.md](body.md): weight and the body as built.
- [play-calling.md](play-calling.md): call sheets and installs on game day.
