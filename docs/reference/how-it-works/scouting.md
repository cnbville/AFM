# Scouting: how it works today

Checked against main at dfb8b02 with update 2 lane 8 on 2026-10-26.

## Quick answers

- How much does the head coach's eye count in AI college recruiting? 40% of the staff eye that sets the misread width; the position coach's is 60%. Width = clamp(3.5 - 0.15 × (0.6 × position coach + 0.4 × head coach - 10), 1, 6) rating points. The lasting lean comes from the position coach (`world/recruiting.rs`, `run_signing_stage`, the `noise_sd` and `read` lines).
- And in your own college scouting file? The head coach is one reader among the people who looked. His weight in the centre is say 1.0, the same as a position coach (1.5 if the head coach is you). He looks at a recruit on visits, and on evaluation days and at camps only when the player's group has no position coach (`world/scouting/readers.rs`, `coach_reader`; `world/scouting/dept.rs`, `college_day`, `evaluator_for`).
- What is a coach's "eye"? `evaluation()` = 10 + 0.3 × (recruiting - 10) + 0.3 × (mean teaching - 10) + 3 × a fixed personal draw, clamped 1-20. Your own coach uses his set value (`world/coach.rs`, `Coach::evaluation`).
- Does a better eye narrow the range or move the centre? Both. Each reader's misread scales with `sharp(r)` = 1.4 - 0.04 × r (1 → 1.36, 10 → 1.0, 20 → 0.6) (`world/scouting/file.rs`, `sharp`).
- Do more looks always help? Only against noise. 60% of a reader's misread is his lasting lean on that player; only the other 40% shrinks with his look count (by √n). A second person helps more than a tenth look by the same one (`file.rs`, `read`, `LEAN`/`NOISE`).
- How wide is a range on screen? Centre ± the combined sd (at least 1), floored and ceiled to 1-99 (`file.rs`, `AttrRead::range`). Own players on the simple player page: ±2 to ±4.3 by the coach's eye (`crates/afm-game/src/reads.rs`, `attributes`).
- Are small schools harder to read? Yes. Misreads × 1.0 at FBS, 1.2 FCS, 1.5 below; × 0.7 for pros; × 1.25 high school, 1.55 for a small high school before 2008 (`file.rs`, `visibility`).
- Is potential ever shown as a number? No. Only role words, "sure / fairly sure / unsure", a floor phrase and hints (`file.rs`, `tier_words`, `sure_words`, `floor_words`, `hints`; `crates/afm-game/src/reads.rs`, `potential_words`).
- Who sees hints? Only when the best potential reader on the file is 13+; notice chance 0.3 + 0.08 per point above 13 (cap 0.85), right 60% at 13 up to 90% (`file.rs`, `hints`).
- Does the combine measure speed for everyone? Every club gets the same numbers. Error in rating points: speed 1.5, acceleration 2.0, jumping 2.0, agility 2.5, strength 2.5; a pro day adds 1.0 (`file.rs`, `measures`).
- Do pro day times run fast? Yes: +2.5 points on speed, acceleration and agility, less the staff's test-reading discount (`file.rs`, `read`, the `hand` line).
- What does a scout's bias do? Tilts his read by 0.33 × his misread sd (for size, testing, tape and speed, times a z clamped to ±1.5, so up to about 0.5 sd), never decides it. Seven biases: size, small schools, testing, tape, one conference, his home region, speed (`world/pro/scouts.rs`, `Bias`; `file.rs`, `read`).
- Does character get read? Yes, 13 traits on 1-20, each through the looks that show it, shaded by the source (warm, true, defends him, cold) (`file.rs`, `trait_reads`, `trait_sees`, `Shade`).
- Does medical get read? Yes: past injuries from the record, a hidden-proneness note only if your doctors examined him, and a grade Cleared / Minor / Major / Failed (`file.rs`, `medical_read`).
- How do AI NFL clubs read prospects? From their own files, by attribute category, with the same lean-plus-noise model, blended with the public grade (`world/pro/scouts.rs`, `read`, `board_from`).
- How do AI NFL clubs read pros? Overall plus a keyed misread of `draft_board_noise(skill)` × 0.6 / (1 + 0.4 × seasons) (`world/pro/mod.rs`, `grade`; `career.rs`, `draft_board_noise`).
- Do the services' errors average out? No. A service's misread is lasting (sd 9 points plus 1.6 per year older than his class) and each new look moves its projection only halfway (`world/recruiting.rs`, `service_bias`, `run_signing_stage`).
- Does a high school coach's call to colleges get a kid stars? No. It raises his visibility by up to 0.12, scaled by your credibility; staffs still judge his film (`world/scouting/hs.rs`, `promos`).
- Does your staff keep its read of a prospect from year to year? Yes, once a season for the top 300 of your scouting board, the players you keep notes on and your roster: the simple read (`reads.rs`) the player page shows, not the scouting file's (`crates/afm-game/src/lane_squadboards.rs`, `followed`, `read_now`; the ratings page, "Kept reads").

## Two read systems on screen

There are two ways the game turns truth into a range. Know which screen uses which.

- The scouting file (`world/scouting/file.rs`, `read`): built from the looks your staff took. Used by the scouting file screen, recruit and portal lists, the department, the draft board (`crates/afm-game/src/lane_scouting.rs`, `scout_file`, `brief_read`, `recruit_scouting`).
- The simple read (`crates/afm-game/src/reads.rs`): used by roster rows, the player page, training and the college front-office lane (`views.rs`, `row`; `lane_core/pages.rs`; `lane_training.rs`; `lane_frontoffice/college.rs`, `staff_read`). It reads no looks.

### The simple read (`crates/afm-game/src/reads.rs`)

- Eye: your position coach's `evaluation()` at college, else the side's coordinator, else the head coach; at the NFL the coordinator, else the head coach; at high school the head coach (`crates/afm-game/src/views.rs`, `eye_for`).
- Own player, each attribute: half-width = 2 + (20 - eye) × 0.12 (eye 20 → ±2, eye 1 → ±4.3) (`reads.rs`, `attributes`).
- Another team's player: physical attributes ±(half + 4); every other attribute "Not seen" (`reads.rs`, `attributes`). Overall ±6 (`reads.rs`, `overall`). Here the eye is fixed at 10 (`views.rs`, `row`).
- Centre = truth + lean × half × 0.6, the lean a fixed -1..1 hash of reader, player and attribute, so it holds day to day (`reads.rs`, `range`, `lean`).
- A recruit your college staff is working: overall ±(3 + (20 - eye) × 0.2), else ±7; potential words only when worked (`lane_frontoffice/college.rs`, `staff_read`).
- Dynamic Links: noise 0.06 + (20 - eye) × 0.006 when measured or own, 0.25 from film; a boy under 21 not measured is read on his projection with noise 0.3 (less 0.008 × eye if own) (`reads.rs`, `links`).
- Potential words: `potential_outlook` with the eye as accuracy, drawn from a stream keyed by reader, player and season (`reads.rs`, `potential_words`).

## The scouting file: looks

A file holds only looks: who, what kind, when (`world/scouting/mod.rs`, `SeatLook`; `world/pro/scouts.rs`, `Look`). The read is rebuilt from them each time a screen asks (`world/scouting/readers.rs`, `read`).

- 31 kinds of look (`world/scouting/mod.rs`, `Kind::ALL`). Each shows some attributes and not others, with a base misread in rating points for an average eye at FBS (`file.rs`, `sees`):
  - Film (film study, high school film, college film, pro film, cross-check, director, GM): physical 8 (7.5 for non-measured), skills 7, the film mind (play recognition, anticipation, decisions, discipline, motor, competitiveness, physicality, toughness, concentration) 7.5, composure 8, football IQ 10; communication not seen.
  - Coach's film and drills: as film, skills 6.5.
  - NFL school call: physical 6.5, skills 6.5, film mind 7, talk mind (football IQ, communication, composure) 9.
  - Practice or game in person: measured physical 5.5, other physical 6, skills 7.5, motor/competitiveness/toughness/physicality 6.5, football IQ 9.5.
  - Camp, workout, local day, tryout: measured physical 4.5, drills 6 (workout 5.5), other skills 8.5.
  - Combine and pro day: drill skills 7.5; the combine's cognitive test reads football IQ and play recognition at 9. They see no physical attribute by eye; their times come in as measurements.
  - Interview: football IQ and communication 7, composure 8, play recognition 8.5, decisions 9.5.
  - Visits (top-30, junior day, official, unofficial, home): football IQ and communication 7.5 at a top-30, else 8.5; composure 8.5.
  - Calls to his coach, his old staff: motor, discipline, concentration (and competitiveness on a call) 8.5-9; football IQ 9-9.5.
  - Medical, background, social media, transcript: no attributes.
- Measurements (`file.rs`, `measures`): speed 1.5, acceleration 2.0, jumping 2.0, agility 2.5, strength 2.5 at the combine; +1.0 pro day; +1.5 camp or showcase (no strength); +2.0 local day, workout, tryout. Combine, pro day and showcase are shared and written the same day (`mod.rs`, `Kind::instant`).
- Test numbers shown are seeded per player, kind and year, so every club sees the same combine; pro day 40s run 0.05 s fast, a camp laser on grass 0.04 s slow (`file.rs`, `tests`).
- School listings run tall and heavy: about an inch and 3-9 pounds (`file.rs`, `listed`).
- A report is written 4-21 days after the look: 7 + 0.7 × (20 - work rate), ± a 3-day draw. Until then the look doesn't count (`file.rs`, `Reader::lag`, `read`). A file with no report written yet is not read at all: "Not seen", no ranges and no role, even with looks taken or filed ahead (a declared underclassman's film is filed a week ahead) (`readers.rs`, `read`).

## The scouting file: who reads, and their weight

Each reader has a judge, potential, athletic, character, network and work rating (1-20) and a say (`file.rs`, `Reader`; built in `readers.rs`, `reader`).

| Reader | judge | potential | athletic | character | say |
|---|---|---|---|---|---|
| A coach (head coach, coordinator, position coach) | 0.6 eval + 0.4 teaching of the player's group | 0.7 eval + 0.3 teaching | 0.5 eval + 0.5 teaching | relationships | 1.0 (1.5 if he is you) |
| NFL scout (area, national, director) | 0.6 judge + 0.4 group knowledge | 0.7 potential + 0.3 knowledge | athletic | character | 0.8 + 0.02 × conviction, × 1.2 for the director |
| NFL GM | skill | skill | 10 | 10 | 1.4 if the GM holds the final say, else 1.1 |
| College personnel staffer | drawn | drawn | drawn | drawn | 0.9 |
| High school assistant | teaching | 0.8 teaching + 1 | teaching | 12 | 0.8 |
| Trainer, academic, analytics, physician | 10 | 10 | his rating | his rating | 0.6 |
| Club security | 10 | 10 | 10 | 13 | 0.7 |
| A shared measurement | - | - | - | - | weighted 1/sd² only |

Sources: `readers.rs`, `coach_reader`, `scout_reader`, `club_reader`, `staffer_reader`, `personnel`. A coach's work rate is 0.5 × recruiting + 5.

- College personnel (`readers.rs`, `personnel`): FBS gets a director of player personnel, a director of recruiting and an analyst; two more past money 0.45, two more past 0.65, a general manager from 2023 past 0.55. FCS two posts, others one. Before 2012 FBS keeps two, others one. Money = 0.4 × facilities/20 + 0.6 × prestige/100. Both are stored 0-1 (`world/program.rs`), so money is at most about 0.026: in practice every FBS staff gets the first three posts only, no general manager, and ratings drawn N(about 8, 2.8). Each post turns over about every five years.
- Which skill reads what (`file.rs`, `read`): physical attributes by athletic; mental by 0.6 judge + 0.4 character; everything else by judge.
- One reader's misread on one attribute: sd = sharp(skill) × best base misread among his looks × visibility. Lean = 0.7 × his lean on the category + 0.714 × his lean on that attribute (keyed, never moves). Noise is keyed by the year of his latest look, so a new football year gives a new draw (`file.rs`, `read`, `noise_year`).
- Misread = sd × (0.6 × lean + 0.4 × noise / √n) / 0.7211; variance = sd² × (0.36 + 0.16/n) / 0.52.
- The staff's centre = truth + Σ(say × misread / var) / Σ(say / var), with measurements added at 1/sd². The shown sd = √(1 / Σ(1/var)): say moves the centre, not the width (`file.rs`, `read`).
- What it does not read: rank or job title beyond say. A head coach is not weighted above his position coach unless the head coach is you.
- The overall now: unseen attributes get a wide prior (9 × visibility); overall sd = √(0.49 × Σ by category (Σ weight × sd)² + 0.51 × Σ weight² × sd²) (`file.rs`, `read`).

### Who looks at whom

- Your own players: the file adds eight weeks of practice, one look a week by his position coach (college) or head coach (NFL), one by you, and his games in September-December (`readers.rs`, `own_roster`). The high school seat has only you.
- College seat (`dept.rs`, `college_day`): the board is your offers, visits, commits, and the best 3-star-plus or nearby recruits for the next two classes, plus portal players (320 FBS, 200 FCS, 120 others) (`dept.rs`, `watch`).
  - Mondays: personnel study a quarter of the board on film. Before 2008 a small high school's tape gets through 35% of the time.
  - Tuesdays in contact, evaluation and quiet periods: the area recruiter calls a sixth of the board (old staff for portal players).
  - Evaluation days (Thursday practice, Friday game in the fall; Thursdays in the spring window): the position coach (`evaluator_for`), your assignments first, then your asks, then the board's unseen. Limits 42 fall days (33 from 2023) and 168 spring (`dept.rs`, `eval_limits`).
  - June camps (2nd and 3rd Saturdays): board recruits within 400 miles, 45% chance each, read by the position coach or head coach.
  - Visits on campus and home visits are filed under the head coach. October 1: transcripts (academic staff) and social media (personnel) for the board's seniors.
- NFL seat: the built club year below, plus your assignments every second Wednesday (school calls in the fall, film after) and the director's five extra film looks a week on a focus group (`dept.rs`, `nfl_day`). The other instructions (small schools, scheme fit, ask before taking off, analytics) are kept but nothing in the engine reads them.
- Asks from the file screen: only "Evaluate in person" (a practice look) is turned into a look by the engine; the others, and the private workout and medical recheck lists, wait on the ask list (`dept.rs`, `college_day`; `crates/afm-game/src/lane_scouting.rs`, `ask`, `board_list`).

## The NFL clubs' files (AI and yours)

The scouting year per club (`world/pro/scouts.rs`):

- Department size: 6 area scouts to 1994, 7 to 2004, 8 to 2014, 9 after; ±1 for owners 15+ or 5-; nationals 1, 2 (1995-2009), then 3; one director (`sizes`). Area coverage (`covers`): with eight area scouts or fewer each covers the regions r with r % areas = his region; a ninth man shares the Deep South and a tenth Texas and Louisiana, the region's schools alternating between its two men (before update 2 lane 8 the ninth and tenth covered nothing). Ratings N(12 director / 11 national / 9 area + 3 × (budget - 0.6), 2.8) (`hire`).
- Turnover after the rush: 10% leave, +12% × (1 - budget) for a judge 15+, +50% at 64+; vacancies promoted from below 60% of the time (`staff`).
- Fall (`fall`, run with the December reads): the list is the top 5 × draft slots by last December's read. Each area scout makes round(38 × (0.75 + 0.025 × work) × 0.85 if he covers the West or mountains) school calls from September, big programs (3+ listed seniors) twice. An unlisted senior is noted if overall + 5 × sharp(judge) × a draw ≥ the list's median overall and a 0.4 + 0.03 × (network - 10) chance.
- December 1 cross-checks: nationals on the club's top 150 (a second national on the top 60), the director on the top 80 (`cross_checks`).
- January: declared underclassmen get an area scout's film look, and a national cross-check if projected round 1-3 (`underclassmen`). All-star week: the region's area scout, plus a national at the Senior Bowl (`all_star`).
- Combine: shared measurements unless he skipped the workouts; 60 interviews (45 from 2020) by the director, your war room's list first (`combine`).
- Pro days: untested prospects measured; a club sends its OC (offense) or DC (defense), the head coach for specialists, to any it grades 66+ (`pro_day`). Only FBS and FCS programs hold pro days (`world/nfl_year.rs`, `new_season`).
- Spring: cross-checks (100/40/60) and the GM on the top 40; OC or DC film on the top 100; the head coach on the top 30; 30 top-30 visits filed under the GM, from about ten places above the club's first pick (`spring`).
- Files are cleared each fall and after the rush (`fall`, `after_rush`).

How an AI club reads its file (`world/pro/scouts.rs`, `read`, `eye`). Same lean-plus-noise model as your file, but by eight attribute categories, not 67 attributes:

- Base misread: school call and all-star physical 6, other 7; film, cross-check, director, GM, coach physical 8, other 7; interview and visit mental 8 only. Combine 2, pro day 3 on physical (`LookKind::sees`, `measured`).
- Coaches' eye here is their teaching of the player's group for judge and potential (not `evaluation()`), athletic 10, character = relationships; the head coach's say is 1.5 when the coach holds the final say, else 1.0. GM and scouts as in the table above.
- Grade = now + q × (potential - now) + gift - competition discount - red flags, q = 0.25 + 0.2 × youth, youth = clamp((23 - age)/3, 0, 1). Gift: 2.0 points per full Dynamic Link once tested, 1.0 on film. Discount: FCS 1, D2/JUCO 5, NAIA 6, D3 7. Flags: 4 per suspension, 8 per dismissal (`DRAFT_GIFT`, `competition_discount`, `flags`; `career.rs`, `PIPELINE.red_flag_discount`).
- A category nobody saw gets a prior of 9 × visibility; potential nobody read gets (3 + 7 × plasticity) × 1.6 × visibility.
- Your NFL seat's file screen uses `readers.rs` (coach eye = `evaluation()`), while its draft board grades come from this `scouts::read` (coach eye = teaching). The two can differ for the same coach.

## Potential, hints and comparisons

- Ceiling read in the file (`file.rs`, `read`): per reader sd = (3 + 7 × plasticity) × sharp(potential) × visibility, × 1.15 unless he had a talk look (interview, visit, call, background). Same 60/40 lean and noise, weighted by say. Shown as role words, never a number. Plasticity: ≤17 1.0, 18-20 0.75, 21-23 0.5, 24-26 0.2, 27+ 0 (`development.rs`, `potential_plasticity`).
- Role words (`file.rs`, `tier_words`). Recruit: 75+ "a future NFL player", 68+ Power Four starter, 61+ FBS starter, 54+ FCS player, else small college. Prospect or pro: 92+ one of the best, 85+ Pro Bowl, 75+ starter, 65+ backup, else camp body.
- Sure words from the ceiling sd: ≤3.5 sure, ≤6 fairly sure, else unsure (`sure_words`). Floor phrases: boom or bust, raw but the tools are there, what you see is what you get, high floor limited ceiling, a year or two away, pro-ready, a steady climb (`floor_words`).
- File hints (`file.rs`, `hints`): 14 facts tied to a true cause (still growing, early bloomer, body done growing, frame to add 20 pounds, work ethic either way, soaks up coaching, love of the game either way, young or old for his class, jumps routes, bend for his size, a safety's body at QB). Only the best potential reader decides; none below 13.
- `potential_outlook` (`scouting.rs`), used by the simple read: tier from the report's central potential (92/85/75/65/52), confidence from the shown range width (≤10 sure, ≤20 fairly sure), six hints from accuracy 13, a false one instead 0.25 - 0.03 per point above 13 (floor 0.03).
- `scout_player` (`scouting.rs`): skill = 1.4 - 0.04 × accuracy; overall sd 2.5 × skill × level visibility; potential sd (2.5 + 7 × plasticity) × skill × visibility; level visibility HS 1.25, small college 1.5, FCS 1.2, FBS 1.0, NFL 0.7 (`level_visibility`). Range rounded out to 5-point bands. Stars = (rating - 40)/5 in half stars, 0.5-5 (`to_stars`).
- Comparison "plays like" (`file.rs`, `comparison`): same position, proven pros (3+ accrued seasons) and the last 4,000 drafted alumni; distance = attribute shape + 0.6 × body + fuzz of 0.6 × sharp(eye). Needs 6 seen attributes. Never matched on potential.

## Character and medical

- Traits (`file.rs`, `TRAIT_NAMES`, `trait_value`): work ethic, coachability, competitiveness, love of the game, ambition, loyalty, money, temper, ego, leadership, professionalism, adaptability, handling setbacks (his background's stability).
- Which look shows which, base error in trait points (`trait_sees`): school call 3.0, interview 2.6, top-30 or local visit 2.4, background 2.2, social 3.0, practice and game 3.0, camp/workout 3.2, campus visits 2.8, home visit 2.4, calls 3.0, old staff 3.0, all-star 3.0, director/GM 3.2, coach 3.4, film 4.0 (competitiveness and love only), transcript 4.0 (professionalism). sd = base × sharp(character).
- Shading (`Shade`, `kind_shade`): calls, practice and games run warm (+1.5 toward good); campus and home visits defend him (+2.0); his old staff runs cold (-1.6). A reader sees through clamp((character + network - 16)/24, 0, 0.8) of it. Trait reads weigh by 1/variance only; say plays no part (`trait_reads`).
- AI clubs' character read (`world/pro/scouts.rs`, `read`, `true_risk`): risk = clamp((0.6 × low professionalism + 0.5 × temper, in z) / 0.28, -3, 4) in sd; per-look error school 1.3, interview 0.9, visit 0.8, director/GM 1.2, × sharp(character). Nobody asked: 0.3 × true risk + 1.2 × a draw.
- Medical (`file.rs`, `medical_read`): examined only with a medical, top-30 visit or local day look; records come from school calls, calls, old staff, background, official visits and the combine. Concern = 0.45 × (proneness - 10)/5 + 0.8 × majors + 0.25 × moderates - 0.2 × longevity/2 + 0.3 × wear, misread sd (0.35 examined, 0.8 not) × sharp(medical rating). Grades: 0.6 minor, 1.4 major, 2.4 failed. Proneness and longevity notes appear only when examined. Medical rating: the club's or program's medical head, 7 at a high school, 0 read as 10 (`readers.rs`, `medical_rating`).

## How reads sharpen over time

- More looks by one reader shrink only his 40% noise share; a different reader averages out the lean (`file.rs`, `read`).
- A new football year re-draws each reader's noise (`noise_year`); the lean stays.
- `upto` and the lag let the screen show how a range narrowed look by look (`file.rs`, `read`, `today`).
- Services' recruit projection moves halfway toward each new read (`world/recruiting.rs`, `run_signing_stage`). The December pro read is redone each December; all-star week moves the grade 0.3 × (accuracy-16 read - grade), capped ±2 (`world/recruiting.rs`, `draft_reads`, `all_star_game`).
- Look-backs (`world/scouting/lookback.rs`, `verdict`): three years on, each graded player gets an outcome rank; held, too high or too low by the out-on-a-limb distance (12 places or 35% of the rank), or "no fair read" under 3 looks. The staffer with the most looks carries it (`lead_of`).

## AI college staffs reading recruits

All in `world/recruiting.rs`, `run_signing_stage` (the board per position):

- Film = his last performance (or overall), aligned by position onto the class's scale (`film_raw`, the `film` line). Projection = the services' shared read.
- Staff read = 0.6 × (film + read 1) + 0.4 × (projection + read 2), plus scheme fit (0.3 film sd, `FIT_EVAL`), plus Dynamic Links seen (0.5 film sd per full link × clamp(0.6 + 0.04 × (position coach eye - 10), 0.3, 1.0), `STAFF_GIFT`), minus a grade risk of 4 × clamp((2.6 - GPA)/0.6) × (1 - academic support) × (1 - 0.6 × hot seat).
- Read width as in Quick answers: eyes 10/10 → 3.5; 20/20 → 2.0; 1/1 → 4.85. Each read = width × (0.6 × the position coach's lean + 0.4 × the program's lean) / 0.7211. No position coach: the head coach's eye fills both.
- What it does not read: coordinators, personnel staff, or looks. AI staffs have no looks; only your seat does.
- Services (`world/recruiting.rs`): `scout_player` at accuracy 12 at HS level plus a lasting service bias (9 × draw + 1.6 per year old for his class) (`service_bias`, `SERVICE_MISREAD`, `SERVICE_MATURITY`).

## Public desks

`world/scouting/public.rs`:

- Five services (from 1998, 2006, 2010, 2021; one ends 2017) and four draft analysts (1978, 2004-2017, 2010, 2014), made-up names, hidden skill N(12, 2) clamped 7-17 (analysts 8-17), each with a lean (`staff_desks`).
- Analyst grade = December score + 3.5 × (1.4 - 0.04 × skill) × (0.8 × lasting lean + 0.35 × weekly noise), plus its lean: big programs +1.0 (FBS with prestige ≥ 60; prestige is stored 0-1, so this never fires), size 0.8 × z, viral 0.6 × speed z, production 0.6 × last performance clamped to ±1.5 (a rating, so in practice +0.9) (`desk_grade`).
- Big boards on the 1st of September-December, then Tuesdays from mid-December; mocks Thursdays from mid-December (round 1; two rounds in the last week), picking best grade + 0.8 × need. The season runs for the draft the boards are for (`draft_year`): any day September-December, and January-April only in that draft's own year, since a football year's dates hold last spring's draft until the Super Bowl turns it. Consensus top 50 = sum of (100 - rank) (`day`, `mock`). Risers of 10+ places make the news (`risers_and_fallers`). Mocks scored the morning after (`score_mocks`), with each desk's final top 100 against the first 100 picks, per 100 (`Desk::board_hits`, ten drafts; `board_hit_rate`: a small world's top N against its first N picks). Before the season sets the draft order, mocks pick in today's standings' order (`world/nfl_year.rs`, `mock_order`).
- A service's recruit stars stay within ±0.6 of the shared stars (`service_read`). The recruit search and the high school coach's page show each service's own stars and rank as kept at its last update (`world/scouting/services.rs`, `service_boards`; see [recruiting.md](recruiting.md), "The services before the signing stages").
- The search screens (Find recruits, Find prospects) filter and sort on what your staff's file and the public know: a kid with no file reads "Not seen", and a range filter takes a "how sure" floor (a fair read spreads 6 points or less, a narrow one 3.5) (`crates/afm-game/src/lane_find/fields.rs`, `value`, `Sure`). An NFL prospect's marks count as verified after the combine (if invited) or his school's pro day, read from the coming draft's own dates (`verified`, `draft_dates`). Find prospects' "Scheme fit" reads your scouts' attribute reads against your club's systems (`scheme_fit`; `world/systems.rs`, `read_fit`). The search's day cache keys on the day, the files' looks and who reads (`crates/afm-game/src/lane_find/staff.rs`, `cache_key`).

## The high school coach's side

`world/scouting/hs.rs`:

- Credibility with a college: starts at 0.35 + 0.02 × (your network - 10), clamped 0.1-0.7 (`credibility`).
- A call ("worth a look", "can play for you", "a sure thing") adds (0.03 + 0.03 × strength) × (0.5 for one college, 1.0 for the area) × (0.4 + credibility) to his recruiting visibility, capped 0.12 (`promos`; read in `world/recruiting.rs`, the `visibility` sum).
- Settled two seasons on (700 days, on September 1): played (150+ college snaps or a pro) +0.04 + 0.03 × strength; else -0.03 - 0.05 × strength; clamped 0.05-0.95 (`day`).

## Pro scouting

- Your NFL seat: a pro director and 3 pro scouts to 1999, 4 to 2014, 5 after (+1 for an owner 15+), each covering a share of the other clubs. Spring grade on March Mondays (a quarter of each scout's clubs a week); in season one club a week, Mondays September-January; pro film looks kept two years (`world/scouting/pro.rs`, `staff`, `day`).
- Tuesday tryouts: a tryout look by your head coach (`pro.rs`, `tryout`).
- AI clubs: no pro files; `world/pro/mod.rs`, `grade`. sd = (1 + (20 - GM skill) × 0.25) × 0.6 / (1 + 0.4 × pro seasons), keyed by club, player and year.

## Your read of your own staff

`world/staff_read.rs` (for coaches and support staff, not players):

- Each rating shows as a 1-20 range: half = 0.75 + 2.6 / (1 + 0.6 × evidence); centre = truth + your fixed lean × half × 0.6 (`read`, `lean`).
- Evidence = what you knew on day one (0.4 if kept on; else 1.5 × network warmth + 0.6 if interviewed) + rate × years with you × room size (teaching only) (`evidence`, `members`). Rates per season: his group's teaching 1.0 (0.6 for a head coach or coordinator), calling 1.2, scheme 1.0 for a head coach or coordinator (0.35 for a position coach), game management 1.0 for a head coach (0.6 for a coordinator), motivation, discipline and relationships 0.7, recruiting 0.8, network 0.5 (`coach_facets`).
- Words: width ≤2 sure, 3 fairly sure, 4-5 not sure, wider too early (`sure_words`). Other teams' staffs stay reputation only.

## Opponent film

`world/tape.rs`: your coming opponents' last 6 games are kept as tapes for the game-day report. Your staff's film study = 0.35 × (OC scheme + DC scheme) + 7 × facilities (college), + 6.3 (NFL), or 0.7 × coach scheme + 7 × resources (high school), clamped 1-20 (`World::seat_film_study`). It sizes how often the report's tags are wrong. It does not feed player files.

## The fresh look (update 2, lane 8)

What the tape holds (SK1-SK5), read the same by every staff:

- A player's footage comes from his season records (`world/scouting/footage.rs`, `footage`). College: snaps over the last two seasons, starts = snaps / 700; role snaps from the record (pass sets, run blocks, rushes, coverage, routes, dropbacks, carries); man and zone shares from his program's shell (`man_share`: man-pressure 0.7, quarters 0.35, Cover 3 0.25, Cover 2 0.2); a two-gap tackle in a 3-4 counts rush snaps × 0.4. High school: `hs_lines`, starts = snaps / 450, reels from 2008.
- The schedule (`comp`): the snap-weighted strength of schedule plus a division offset (`div_offset`: FCS -4, D2 and JUCO -8, NAIA -9, D3 -10); high school -1.5 × classification + 3 × the state's culture - 3.
- Dominance (`dom`): his grade over the last two seasons, 60 + 95 × the per-snap grade × confidence, then clamp((grade - 70) / 15, 0, 1.5).
- Context (`context`): his scheme for his spot plus his unit against his conference's (`unit_rel`, cached per day), clamped ±2.
- Luck: last season's form against his true overall, ±8.
- Film narrows a read by `film_factor` = clamp(1.55 - 0.27 × starts, 0.75, 1.55) on every tape look (1.0 with no record) (`file.rs`, `read`; `world/pro/scouts.rs`, `read`).
- A role skill under 60 snaps of its role (`ROLE_MIN`) stays unread on tape (`role_seen`); the AI boards blend that share with the position prior (`untested_shares`).
- The shared tilt (`scout_tilt`) = 2.2 × dom × weak schedule (× 0.4 once seen against good players: an all-star week, the combine, a showcase) + `numbers_lean(judge)` × 2.5 × context (a judge under 14 leans up to 60%) + 2.5 for a reel alone + 0.3 × luck. In your file it is weighted by each reader's share of tape looks, half on physical skills (`file.rs`, `read`, `lean_of`; `tape_flags`).
- The same footage feeds the AI clubs' boards (`scouts::read`, a per-board cache in `Ctx.foot`), the college staffs' film read and the services (`recruiting.rs`, `Recruit.wide` and `svc`: a reel's tilt × (1 - visibility), minus 0.3 × luck on film that is already his last season), the December read (`tape_lean` at judge 12) and the desks (`public.rs`, `desk_grade`, the spring months against a judge-12 baseline).
- The file shows the reasons in words (`footage::reasons`) on its "Your look" tab.

The analytics staff (SK6): a numbers reader (`readers.rs`, `numbers_reader`; judge = its rating) at NFL clubs from 2003 and FBS programs from 2016 (`has_analytics`), one `Kind::Numbers` look each December 10 of a record year, added only to a file that already has looks (`numbers_looks`). It sees role skills with the snaps and four mental ones (`numbers_see`), with a misread of 8 × clamp(√(2100 / snaps), 0.8, 3) (`analytics_sd`) and a tilt of 0.8 × luck + 0.75 × context (`analytics_tilt`). Its own order of the 300 you grade on draft day is kept (`lookback.rs`, `Graded.numbers`, `numbers_record`).

The scouts as people (SK7-SK10, `world/pro/scouts.rs`, `staff`): in a turnover year each club, richest first, loses about 0.1 of its men plus 0.12 × (1 - budget) for a judge of 15 and up, and half of those 64 and up retire. The leavers go to `Scouting::market` (up to 240, sorted by standing, one in five leaves it each year), with about 1.5 newcomers a club each spring (`newcomers`). Posts are filled by `sign`: from the market within 3 + 6 × budget names, a step up for a fit man (a director from a national of 12+, a national from an area of 11+), else half the time another club's man a step below (filled in place), else a new hire (`hire`). Each kept man has six AI look-backs settle a year at 0.35 + 0.025 × (judge - 10) (`ai_lookbacks`) and grows (`grow`): a chance of 0.32 × youth (1.0 under 38, then 0.6, 0.3, 0.1) × (0.5 + share held) for a point of judge, 80% of that for potential, a point of one group's knowledge; past 56 a 35% chance to lose a point of work. An area scout's contacts grow by (1 - c) × 0.22 × (0.6 + 0.04 × network) a year and fade 15% a year out of an area role; `place_areas` keeps him on his region, a move starts him at 0.05. Contacts cut a character read's misread by up to 35% and add 0.2 × contacts of see-through on background, call, school and old-staff looks of players at programs in his region (`file.rs`, `trait_reads`; `scouts.rs`, `Eye.contacts`). Bug 19: `staff_ethnicity` draws seven backgrounds by era. `climb_to_gm` (from `office.rs`, `replace_gm`): 35% of GM openings from 1990 go to the best director aged 36-60 of another club, yours included; skill 0.5 × judge + 0.5 × potential + 1, then half the read; a non-White director's old club gets the 2020 diversity picks (`trades::note_diversity_hire`).

Your own side (`world/scouting/own.rs`; the commands in `crates/afm-game/src/lane_evaluate.rs`):

- The May review (SK11): opens instead of your club's turnover (`open_review`, due June 1); your calls (renew, raise +12%, promote, let go), signings from the market into your let-go posts or up to two new ones (`sign`), then `close_review`: the AI's leave rule (a raise keeps 60% of the sharp men who'd go), growth, the director's post filled, the areas placed, look-back leads of changed posts cleared.
- College personnel (SK7): `personnel_market` (12 keyed candidates with their past programs and look-backs), `hire_personnel`, `let_go_personnel`; your staff overrides `readers::personnel` once changed (`keyed_personnel` is the program's own); a year on each July 1 (`grow_personnel`).
- Your days out (SK12-SK13): `trip_kinds` and `trip_day` (the seat's rules above), `go`, `take_trips` (the look under your code, or the man you sent). Seen (`leak`): an NFL head man's trip plants a `predraft::Leak`; a college coach's adds 0.05 visibility a trip, 0.1 at most for a year (`seen_lift`, read in `recruiting.rs`'s pool).
- Your eye (SK14): `credit_eye` each September 2, 0.04 for a settled look of yours that held, 0.06 for a miss, three points at most, added to your evaluation.
- Interviews (SK15): `rooms` (combine 15 minutes, 18 from 2020, for a declared player on your interview list; top-30 visit 60 in March and April; official visit 45), `interview` (answers from `lines::interview_answer` by `answer_band`; the whiteboard quiz out of 6 on football IQ and recognition), `interview_traits` (each topic's traits, misread 2.4 × √(6 / minutes) × sharp(your relationships), shaded by 0.6 × `polish`), applied in `readers::read` for your seat.
- Meetings (SK16): `meeting_today`, `hold_meeting` (top 60, 4+ points apart, 12 items: an NFL room down its board, a college staff the twelve its high men read highest; the presenter's conviction shows only as words, "pounds the table" at 15 and up, "hedges" at 6 and under), `settle` (three board places a point; college through `front_office::college::reorder`), `meeting_record`.
- Minimums (SK17): `set_minimum`, `under` (measured at the combine, a pro day, your camp or a workout, else the listed height and weight); AI GMs' by key (`ai_minimums`, a third hold one, 12% two) in `scouts::board_from` (do not draft); college head coaches' height and weight lines in `run_signing_stage`; yours at your club or program the same way when set to drop.
- Services (SK18): `service_of` (36% National, 30% BLESTO by club and decade; yours once set), `service_grade` (7 + (read - division - 78) / 4 on 1.0-9.0), the member clubs' fall lists in `scouts::fall` (the service's top `slots × 5`), a lent scout at 60% of his calls, non-members at 112%.
- Your word (SK19): `tell`, `word_lift` ((0.5 + 0.6 × strength) × credit, for 400 days, added in `scouts::read`), `settle_words` after the draft, `cred`; `set_openness` and `openness_factor` (school calls on your players × 1.15 closed, × 0.9 with the film room).
- Pro days (SK20): `scouts::pro_day` counts each program's (a club with a read of 60+ sends a scout, 74+ a head man) into `Scouting::pro_days`, and sets `Program.pro_day` = `pro_day_pull` (0.03 × min((clubs + 2 × heads) / 40, 1), last two springs), read in `utility_fit` × (0.6 + development + path).
- The advisory grades (SK21): `recruiting.rs`, `advisory_grades` (by era, above), `own::request_advisory`, `stay_or_go`, `talk_factor` (trust 0.35 + 0.02 × (relationships - 10) + 0.015 × (coachability - 10) + 0.015 × (loyalty - 10), 0.05-0.75; stay × (1 - 0.6 × trust), go × (1 + 0.2 × trust)).
- Athletes (SK22): `athletes` (keyed, 2 to 7 by enrollment), `recruit_athlete` (chance = his openness + 0.025 × (relationships - 10), less 0.1 for a senior), `World::add_athlete` (`world/mod.rs`, made by `new_kid` with his talent, his sport's tools +6, football skills × 0.8).
- Bug 18: `own::voice` and `lines::voice` (his range from `reader_overall`, the room's centre, his looks, the tier, the fit from `footage::scheme_asks`), in the file's and the war room's voices.

## Where the knobs are

- `world/scouting/file.rs`: `LEAN` 0.6, `NOISE` 0.4, `MIX_SD`, `sharp`, `sees` (base misreads), `measures`, `trait_sees`, `Shade::tilt`, `visibility`, the 0.33 bias tilt, the 9 × visibility prior, the 1.15 no-talk factor, medical cut-offs 0.6/1.4/2.4, `hints` notice and hit rates.
- `world/scouting/readers.rs`: each reader's weights and say, `personnel` sizes and means.
- `world/pro/scouts.rs`: `FILM_SD` 7, `LookKind::sees`, `measured`, `character`, `sizes`, `hire`, `eye`, `DRAFT_GIFT` 2.0, `competition_discount`, call counts in `fall`, cross-check depths in `fall` and `spring`.
- `scouting.rs`: `level_visibility`, `HINT_MIN_ACCURACY` 13, `potential_outlook` rates.
- `crates/afm-game/src/reads.rs`: half-widths 2 + 0.12 × (20 - eye), +4 for strangers, ±6 overall.
- `world/recruiting.rs`: the college `noise_sd` formula, `FIT_EVAL`, `STAFF_GIFT`, `SERVICE_MISREAD`, `SERVICE_MATURITY`.
- `world/scouting/dept.rs`: `eval_limits`, board sizes in `watch`, camp radius and odds.
- `world/scouting/public.rs`: desk lists and skills, grade spread 3.5, the season's months in `day`.
- `world/scouting/services.rs`: each service's `lean` on its own list.
- `world/coach.rs`: `Coach::evaluation`.
- `world/scouting/footage.rs` and `world/scouting/own.rs`: the fresh look's sizes (listed in `docs/game.md`, update 2 lane 8, Claude's picks).

## Related pages

- [draft.md](draft.md): how these reads become boards, picks and the rush.
- [recruiting.md](recruiting.md): stars, offers and signing that the college reads feed.
- [ratings.md](ratings.md): the attributes, overall and ranges on screen.
- [development.md](development.md): potential, ceilings and plasticity.
- [staff-and-coaches.md](staff-and-coaches.md): coach ratings behind each eye.
- [people.md](people.md): the personality traits the character read aims at.
