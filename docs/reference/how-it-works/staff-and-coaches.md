# Staff and coaches: how it works today

Checked against main at 94a6553 on 2026-10-25.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`. Most of the world side is `world/coach.rs` (the coach) and `world/staff.rs` (jobs, the carousel); your own side is `world/folk/` and `world/front_office/`.

## Quick answers

- What scale are coach ratings on? 1-20 for every ability part, personality trait and reputation (`world/coach.rs`, `Ability`, `CoachPersonality`, `Coach`).
- Which ratings are hidden? All of `Ability` (teaching per group, scheme, game management, motivation, discipline, recruiting, network, relationships), the learning ceiling, sportsmanship, home state, decline age and rate, systems knowledge. The world shows reputation (`world/coach.rs`, `Coach` field notes).
- What does the game show of a coach? Other teams': reputation in words ("one of the best in the game" ≥ 17 ... "unproven" < 5) (`world/folk/mod.rs`, `rep_words`). Your own staff: ranges on the hidden 1-20 that narrow with seasons worked (`world/staff_read.rs`).
- How good are coaches by level? Core ~ N(mean, sd): HS 8.0/2.6, small college 9.5/2.3, FCS 10.5/2.2, FBS 12.0/2.3, NFL 13.5/2.0; each part = core + its own noise (`level_ability`, `generate_coach`).
- How does a coach affect development? Through one coaching number 1-20 per player: college = 0.2 HC + 0.35 coordinator + 0.3 position coach teaching of his group + 0.15 × facilities term (`world/season.rs`, the stage match in the season step). Growth multiplier 0.75 + 0.025 × coaching, scaled by coachability (`development.rs`, `develop_season`).
- Does a head coach's overall matter on game day? Game management adds (GM − 10) × 0.12 to team strength in the quick results; on the engine the play callers' skill comes from scheme and game management (`world/season.rs`, `college_results`, `coach_caller`).
- Who calls plays? The coordinator, or the head coach with none; you if you choose to call a side (`world/season.rs`, `play_caller`).
- What does recruiting read from coaches? Head coach recruiting and relationships, position coach recruiting, every staffer's ties to high schools, the HS coach's network for exposure, and each coach's eye for boards (`world/recruiting.rs`, `run_signing_stage`; `Coach::evaluation`).
- What's a coach's "eye"? `evaluation` = 10 + 0.3 × (recruiting − 10) + 0.3 × (mean teaching − 10) + 3 × a fixed personal normal, 1-20; your own coach has his own number (`world/coach.rs`, `evaluation`).
- How does reputation move? Each season 0.3 × (head_coach_value + 12 × vs-expectation − rep), half speed for assistants, minus 0.6 × locker-room trouble; titles +3 (conference/state) or +5 (national) × headroom × stage (`season_reputation`, `title_reputation`).
- How are coaches hired? Ranked by `candidate_score` (reputation with recency bias + noisy reads of the parts the job needs), then each says yes or no by `weigh_offer`; the search calls up to 20 names (`world/staff.rs`, `fill_opening`).
- When are coaches fired? College: the AD's review after the regular season (`college_review`); FBS in-season firings on Sundays late Sep - mid Nov (`in_season_firings`); NFL on Black Monday (`nfl_firing`); high school in late January (`hs_review`).
- Is there a hiring bias? Yes, a hidden one against coaches of color in AI searches, biggest for head jobs (`world/ethnicity.rs`, `hiring_bias`; applied in `fill_opening`). Your own hires never carry it.
- What's a coaching style? Five 1-20 dials (risk, freedom, physical, ball hunting, mould) from an AI coach's personality and ability, or set by you (`world/coach.rs`, `style`).
- Do coaches get worse with age? Yes. Learning stops at 58; decline starts at a personal age ~N(59, 3.5) and speeds up 15% a year (`age_one_year`).

## The coach

- `Ability` (`world/coach.rs`): `teaching[8]` per group (QB, RB, receivers, OL, DL, LB, DB, specialists), `scheme`, `game_management`, `motivation`, `discipline`, `recruiting`, `network`, `relationships`.
- Generation (`generate_coach`): half come up on offense; their side's teaching +1, the other −1. `came_up` is his best non-special group on his side. Network = core − 1 + N(0, 3). Ex-pros (HS 4%, NFL 35%, else 12%): network +3 to +6, teaching −0 to −1.5, ambition +2, reputation +1.
- Personality: ambition, loyalty, temper, each N(10, 3.5) (`CoachPersonality`).
- Learning ceiling = core + N(2, 2) − 0.05 per year over 30 (`learning_ceiling`). Your own coach's is 20 (`world/staff.rs`, `take_seat_as`).
- Reputation starts at head_coach_value + N(0, 2) (+1 ex-pro) (`generate_coach`).
- `head_coach_value` = 0.3 mean teaching + 0.2 scheme + 0.15 game management + 0.15 motivation + 0.1 discipline + 0.1 relationships (recruiting and network not in it) (`Ability::head_coach_value`).
- Sportsmanship (how soon starters come out of a decided game): 10 + 3.5 × keyed normal (`world/staff.rs`, `new_coach`).
- Your coach adds four ratings only you have as numbers: play calling, adaptation, evaluation, negotiating (`Yours`). An AI coach's calling comes from scheme and game management; his GM negotiates.

## Ageing and learning

- Each year (`age_one_year`, run in `world/staff.rs`, `age_coaches` after the high school carousel): every part grows by learn × min(room to ceiling / 6, 1) + N(0, 0.15), learn = 0.45 under 35, 0.2 to 47, 0.05 to 57, 0 after.
- Decline past `decline_from` (~N(59, 3.5), 52-68) at rate 0.08-0.35 × (1 + 0.15 × years past): recruiting ×1.2, motivation 1.0, game management 0.8, relationships 0.6, discipline 0.5, teaching 0.4. From 58 scheme loses 0.1-0.5 a year. Network never declines.
- Under a better head coach an assistant picks up 6% of each gap in teaching, scheme and network a year, and 15% of the gap in each system's knowledge (`age_one_year`; `world/systems.rs`, `learn_from`). Network +0.08 each year in a job.
- Visible teaching records fade ×0.85 a year; high school ties ×0.9 (dropped under 0.02) (`age_coaches`). Out-of-work coaches over 66 retire.

## Roles and what each one does

- Roles: head coach, offensive coordinator, defensive coordinator, a position coach per group (`Role`, `Group::coach_title`). College staffs keep all eleven as people; NFL teams keep HC, OC, DC; high schools keep the head coach (`world/staff.rs`, `college_roles`, `nfl_roles`).
- Jobs the world doesn't keep (NFL position coaches and QC, college analysts and GAs, every AI high school's assistants) are drawn people with names, ages and pay, never stored (`world/folk/staffing.rs`, `drawn_staff`, `draw_person`).
- Support heads (strength, medical, analytics, academic, player engagement) are one 1-20 rating each, drawn ~N(5.5 + 7 × budget + 2 × era, 2.8); about one in eight moves on a year (`world/people.rs`, `Support`, `staff_support`). Engagement cuts player trouble by up to 12%.

### Development

- High school: coaching = 0.6 × head coach teaching of his group + 0.4 × staff, staff = 4 + 12 × school resources (or your real assistants at your school); no coach → 7 (`world/season.rs`, season step `Stage::HighSchool`).
- College: 0.2 × HC teaching + 0.35 × coordinator on that side + 0.3 × position coach + 0.15 × (6 + 12 × facilities). Missing coaches fall back down the chain (no PC → coordinator → HC → 9) (`world/season.rs`).
- Prep school 12; a mission or military year 2.
- NFL: team coaching = 0.5 × HC head_coach_value + 0.25 × OC side quality + 0.25 × DC side quality, side quality = 0.7 × mean teaching on that side + 0.3 × scheme (no coordinator → 11) (`world/staff.rs`, `set_nfl_coaching`, `side_quality`; read in `world/pro/mod.rs`).
- In `development.rs`, `develop_season`: growth × (1 + (0.75 + 0.025 × coaching − 1) × (0.6 + 0.04 × coachability)); ceiling drive gets +0.35 × (coaching − 10)/10 and a coachability × coaching term; coaching under 8 risks a "poor coaching" year (4% per point under 8, up to 20%) that costs technique. See [development.md](development.md).
- Style and scheme pull habits: the head coach's style blended with the coordinator's (`tendencies::Style::blend` in the season step). See [tendencies.md](tendencies.md).
- What development does not read: recruiting, network, motivation (motivation goes to player trust, below), reputation.

### Games

- Play caller skill (`world/season.rs`, `coach_caller`): base 0.6 scheme + 0.4 game management, × (0.7 + 0.3 × how well he knows the installed system). Reading 0.7 scheme + 0.3 GM, situational 0.7 GM + 0.3 scheme, creativity = scheme, each ± a fixed personal spread; nerve and aggressiveness from ambition.
- `GameStaff` also carries the head coach's game management, discipline (strictness), sportsmanship and style; film study = 0.35 × (OC + DC scheme) + 7 × facilities (`game_team`).
- Systems: each spring the coordinator (or HC) installs the system he knows ≥ 0.5 that suits his players best, his read noise 0.8 × (20 − scheme)/10; a switch costs a year of learning (`world/systems.rs`, `spring_install`, `draw`).
- Quick results (no engine): team strength + (HC game management − 10) × 0.12 (`college_results`).
- NFL season: staff edge (coaching − 13) / 2.5 (`world/nfl_year.rs`).

### Recruiting and scouting

- Recruiting relationships: HC skill = 0.6 recruiting + 0.4 relationships; position coach recruiting; staff ties to his high school +0.03 × tie. Board noise from the position coach's and HC's eye (`world/recruiting.rs`, `run_signing_stage`). See [recruiting.md](recruiting.md).
- High school coach's network/20 is 0.2 of a prospect's visibility; a strong local coach (relationships ≥ 13, reputation ≥ 11) keeps his stars from powerhouse pulls (×0.35) (`high_school_transfers`).
- Ties: an impact player (all-conference, drafted, signed as a pro) adds 0.2 × weight × room to the HC's, coordinator's and position coach's ties to his high school; ties travel with the coach (`credit_impact`).
- HC relationships sway rotation players to stay before the winter portal: lean × (1 − 0.03 × (relationships − 10)), 0.75-1.25 (`portal_open`).
- Position changes: a staff's eye from the HC's head_coach_value (`world/season.rs`, `reconsider_positions`; `world/mod.rs`, `hs_eye`).
- Your scouting department's reads are their own people and ratings: see [scouting.md](scouting.md).

### The room

- Motivation adds 0.03 × (motivation − 10)/10 to each player's trust a year (`world/people.rs`, `read`).
- NIL jealousy among top earners takes up to 0.3 × jealousy off the head coach's reputation (`world/recruiting.rs`, `nil_payouts`).

## Reputation and the visible record

- `recent` holds the last six seasons' wins above expectation as a share of the season (`vs_expectation` = (wins − expected)/games) (`world/season.rs`, `TeamResult::vs_expectation`).
- Season update, at the end of the regular season (`world/college.rs`, `record_regular_season`; `world/hs_year.rs`; `world/nfl_year.rs`): rep += speed × 0.3 × (head_coach_value + 12 × vs − rep) − speed × 0.6 × locker room; speed 1 head, 0.5 assistant. College locker room = incidents / roster × 8, 0-1. NFL coordinators are judged on 0.5 × vs + a unit term from their side quality.
- Titles (`title_reputation`): +3 conference or state, +5 national, × headroom min((20 − rep)/8, 1) × (0.5 + 0.5 × stage).
- Hiring buzz = reputation + 10 × last season + 4 × the one before + stat buzz (`hiring_buzz`). Hotness adds 3 × employer prestige, +4 national / +2 conference title in his last three seasons there, +100 × three-year prestige rise (−1 to +3); NFL: 6 × (win share − 0.5), +2 with a title (`world/staff.rs`, `hotness`).
- Unit records: how each group's players grew against everyone at their level and age (position coach credit 1, coordinator 0.5, HC 0.25; a high school HC 1 for every group), and how many moved up. `growth_read` = growth / (seasons + 8) (`world/coach.rs`, `UnitRecord`; `world/staff.rs`, `credit_teaching`; `credit_impact`). High school coaches get 1 / 0.5 / 0.25 "moved up" for FBS / FCS / other signees (`run_signing_stage`).

## The coaching ladder and the carousel

- When: college carousel the Sunday after rivalry week; NFL firings on Black Monday, hires after the conference title games; high school carousel in late January, then everyone ages (`world/clock.rs`; `world/nfl_year.rs`; `world/staff.rs`, `Scope`).
- Each run (`carousel`): NCAA enforcement first (a show-cause order ends a college job); then retirements, early exits and reviews; then every open seat is queued and filled best job first; every hire opens the job he left (`hire_all`, `fill_opening`).
- Retirement: 60+, chance 0.04 + 0.03 per year over 60 (+ health scare). Under 60: health 0.08% + 0.025% per year over 40; burnout 0.8% × pressure² × tenure/10 × intensity × (1 + 0.5 per national title here, to 3). A 55-59 coach with 2+ national titles here walks away on top at 2% per title a year (`carousel`, `early_exit`).
- College searches decide on their own day: FBS head 4-22 days (2-10 after the regular season), FCS head 10-35 (5-21 after), smaller 14-45, coordinator 7-30, position coach 10-40; all close by fall camp (`schedule_search`, `college_searches_day`).
- Who's on the list (`candidate_pool`):
  - College head job: head coaches at the same level or one below, coordinators at the same level; the 15 hottest first, 40 random draws; for FBS also the 3 hottest NFL coordinators; D2/JUCO head jobs also 6 top high school coaches (prestige > 0.7); every college job except an FBS head job also draws 3 high school coaches with buzz (a state title in his last two seasons and 90%+ wins) (`hs_buzz`).
  - Coordinator: coordinators a level up, same or down on that side, head coaches a level down; 10 hottest first.
  - Position coach: same group sideways or a level either way, coordinators a level down on that side, state high school coaches with a good record at that group, 12% chance of a new ex-pro.
  - NFL: every NFL coordinator for a head job (same side for a coordinator job), out-of-work NFL coaches, the 8 (head) or 5 (coordinator) hottest FBS names, the 2 hottest other-league head coaches.
  - Everyone: 8 random out-of-work coaches under 66 (a former FBS/FCS head coach won't take a high school job). Show-cause coaches are barred.
- Score (`world/coach.rs`, `candidate_score`, `hiring_needs`): hiring buzz + needs.network × (read network − 10) × 1.2 + needs.teaching × (read teaching − 10) × 0.6 + needs.recruiting × (read recruiting − 10) × 0.6 + 2 × needs.network for an ex-pro + N(0, 1). Reads are ±2.5 noise.
  - Needs: high school network 0.6 × (1 − prestige) + 0.05, teaching 0.3 + 0.6 × prestige; college network 0.25 × (1 − prestige), teaching 0.35 + 0.25 × prestige, recruiting 0.4 − 0.1 × prestige; NFL teaching 0.6 only.
  - Then (`fill_opening`): teaching seen = true − 10 + noise (SD 3.5 − 0.15 × (hiring HC network − 10), 1.5-5) + 2.5 × growth read + 0.8 × ln(1 + moved up). Coordinators +0.5 × side mean, −1 if he came up on the other side; position coaches +0.8 × group, +0.6 came up there; head jobs −0.5 per year over 58, −0.8 per year over 54 for a first head job.
  - NFL doubts a college-only man: −4 for a head job (−1.5 per national title, to 2), half for a coordinator. College doubts an NFL-only man −2. +1.5 from the hiring HC's own tree. The bias: rung weight (head 1, OC 0.7, DC 0.4, PC 0.15) × `hiring_bias` (4.2 easing to 3.32 by 2016, slowly after 2026, × (0.5 + prestige)).
- Saying yes (`weigh_offer`): pull = prestige gain + rung (head 0.1, coordinator 0.05) + 0.12 first head job (more if ambitious) + 0.05 × ln(pay ratio) + NFL pull 0.04 × ambition − 0.02 × recruiter + 0.03 per national title (to 2) + −0.03 × young core left behind − 0.003 per year over 52 − 0.0015 per year settled + going home 0.03 / leaving home −0.02 + hot seat up to 0.03. Needs pull > 0.03; then chance 0.35 + 0.04 (ambition − 10) − 0.03 (loyalty − 10) + 1.5 × pull, 5-95%. Unemployed coaches always say yes; a coach hired this cycle or still in the NFL playoffs won't move.
- Nobody says yes: a head job goes to the better-regarded own coordinator; else a new long-time assistant (head 36-52, NFL head 40-55, position coach 25-36) with reputation −2 (−3 position coach) (`fill_opening`).
- A new college head coach keeps each coordinator 40% and position coach 50% of the time (75% if promoted from the staff); a new NFL head coach keeps each coordinator 30% (`fill_opening`, `nfl_firing`).
- Mentor: an assistant hired under a head coach joins his tree (`hire`).
- A coach of color hired from an NFL coordinator job to a head job earns his old team the diversity picks (`hire`).

## Job security

### College (the athletic director)

- The AD has judgement and patience, each N(10, 3.5); 6% turnover a year, up to 18% when prestige slid over four years, +4% after ten years (`new_ad`, `ad_turnover`).
- The read (`college_review`): −(1.5 × last vs + 0.7 × the one before, his seasons here only) + N(0, 0.06 − 0.003 × (judgement − 10)) − direction × (0.1 in years 1-3, else 0.04) + (0.3 + 0.5 × prestige) × (0.4 − two-year win %) from year 3. Direction = half roster strength against what he inherited (4 points full), half his classes against the three before him (25% full).
- The bar: 0.06 + 0.012 × (patience − 10) − 0.04 × (prestige − 0.5), −0.03 for a new AD who didn't hire him.
- P(fire) = (read − bar) × (2 + 4 × prestige), max 0.85; year 1 only 10% after a 0.35 collapse; year 2 half and only after 0.22; year 3 × 0.75. FCS × 0.5, lower levels × 0.35. Buyout factor clamp(1.2 − buyout / (0.2 × athletic budget + boosters), 0.15, 1). Never while still playing for a title.
- Prove-it year: a loyal winner (5+ seasons, rarely left jobs, a title in his last eight seasons here or 70% over 50+ games) facing a firing gets an extension instead with chance 0.6 + 0.35 × excuse (draft losses, a young roster); a bad next year fires him (`earned_patience`, `excuse`).
- Contracts: 6 years at FBS prestige > 0.6, 5 at > 0.35, 4 FBS/FCS, 3 D2/NAIA/D3, 2 JUCO; pay = salary × (0.8 + 0.4 × rep/20); buyout share 70% big FBS, 50% FBS, 30% FCS, 20% D2/NAIA, 10% D3/JUCO, a fifth of that through 1995, then ramping to full by 2015 (`contract_years`, `buyout_share`, `hire`). A good year with two or fewer left renews with a 15% raise; an expiring deal after a poor year isn't renewed.
- In-season firing (FBS, from year 2): 5+ games, ≤ 30%, 0.15+ under expectation, already on a hot seat or prove-it; chance 0.35 × afford × era (0.4 before 2010) × (0.6 + 0.8 × prestige), half in year 2 (`in_season_firings`).
- A firing or departure reopens the head coach's verbal commitments (`world/recruiting.rs`, `decommit_after_firing`) and, from 2022, a coaching-change portal window.

### NFL (the owner)

- Badness = 1.6 × (0.5 − last win share) + 0.6 × (0.5 − the one before) − 0.5 × vs-expectation; patience 1 − 0.03 × (owner − 10), 0.6-1.4. P = (badness − 0.1) × 3.2 × patience, max 0.9, × 0 / 0.3 / 0.7 / 1 for tenure 0/1/2/3+. A kept coach fires each coordinator with chance (0.42 − win share) × 1.5, max 0.35 (`nfl_firing`).

### High school

- Badness = −(1.5 × last + 0.7 × before) + stories; P = (badness − 0.06) × (2 + 4 × prestige), max 0.85, × 0.1 / 0.4 / 0.75 / 1 by tenure, then halved (`hs_review`). Your own seat is decided by your principal and AD's trust (`world/front_office/school.rs`, `verdict`).

### Your own job

- Your boss is a person with trust in you (0-1) and goals set each season (wins a little above the last three years, postseason, title, recruiting class rank, grades, clean program, beat the rival, crowds) (`world/folk/boss.rs`, `draft_goals`, `trust`). Meetings before the season, at midseason and after it (`day`).
- Trust moves your college review by 0.1 × (0.5 − trust) + 0.015 × (goals missed − met), ±0.1 (`review_push`). It also moves a big booster's chance of leaving by up to +6 points (`booster_push`).

## Pay

- Salary by employer and role, lo × (hi/lo)^prestige (`world/staff.rs`, `salary`): FBS head $0.6-12M, coordinator $0.25-2.5M, position coach $0.15-1.0M; FCS $0.15-0.7M / $80-250K / $50-120K; down to D3; high school $50-160K (teaching plus stipend); NFL head $5-14M, coordinator $1.2-4M, position $0.5-1.2M by owner, up to 2× in the 1990s.
- Assistant deals (`world/folk/staffing.rs`, `deal_terms`, `sign`): pay × (0.8 + 0.4 × rep/20); P4 coordinators 3 years with 50% buyout, P4 position coaches 2 years 30%, other colleges 1 year, NFL 2-3 years. Renewed on signing day each year at +3%.

## The staff market (your side)

- Candidates for your staff (`world/folk/staffing.rs`, `candidates`): coaches who came up at that group or hold that role, scored reputation + 4 × warmth (your network) + 1 if free + step up; top 24. NFL clubs can block a lateral move (70%).
- Your read of a candidate's teaching is a range 1.2-5.5 wide, narrower for people you know and those you interviewed (`candidates`). An interview reads true teaching + 0.3 × (ambition − 10) + 0.2 × (relationships − 10) + noise: a confident talker interviews better than he coaches (`interview_read`).
- Offers (`offer`): score = 0.8 × money (vs his pay, −0.5 to +1) + 0.5 × step + 0.4 × warmth + 0.3 × (your rep − 0.5) + years term − (0.25 + 0.2 × loyalty if employed); his team counters 40% for a coach with rep ≥ 11; a high school needs a teaching opening (55%).
- Firing: you owe the rest of his deal (× buyout share, or 25%); the room reacts by his relationships and growth record (`fire`).
- Your staff brings you proposals (start a young player, rest a starter) about one week in two (`proposals_week`).
- Your own job hunt: searches score you with the same formula plus a bonus for someone on the staff who knows you (1 + 1.5 × strength), home state +0.5, alma mater +0.6, and Aura for head jobs (`world/front_office/market.rs`, `bonus`, `candidate`).
- Taking over a job: two weeks of dated meetings; you keep or let go each holdover assistant; a college hire opens your players' coaching-change window (`world/folk/takeover.rs`, `begin`, `coach_change_window`, `holdover_answer`).
- Your read of your own staff: a range per rating, about 6 wide at first, ~3 after three seasons, its centre settling toward the truth (`world/staff_read.rs`).

## Coaching styles

- `Style` dials, 1-20 (`world/coach.rs`, `style`): risk = ambition; freedom = 21 − discipline + level norm + 1.5 × keyed normal; physical = 10.5 + 0.4 × (temper − 10) + 2.5 × normal; ball hunting = 10.5 + 3 × normal; mould = 10.5 − 0.25 × (relationships − 10) + 2.5 × normal. Freedom is read against the average discipline of head coaches at his level (`set_freedom_norms`). Your coach's style is the one you set (`style_set`).
- What styles do to players' habits: see [tendencies.md](tendencies.md).

## Where the knobs are

- `world/coach.rs`: `level_ability`, `generate_coach` spreads, `head_coach_value` weights, `age_one_year` learn and decline weights, `season_reputation` (0.3, 12, 0.6), `title_reputation` (3, 5), `hiring_buzz` (10, 4), `hiring_needs`, `candidate_score`, `evaluation`, `style`.
- `world/staff.rs`: `salary`, `hotness`, `weigh_offer` part sizes, `early_exit`, `nfl_firing`, `college_review`, `hs_review`, `in_season_firings`, `contract_years`, `buyout_share`, `athletic_budget`, `new_ad`, `ad_turnover`, `candidate_pool` list sizes (15, 10, 40, 3, 8, 5, 2, 20 calls), `schedule_search` lengths, keep shares in `fill_opening`.
- `world/ethnicity.rs`: `hiring_bias`. `world/systems.rs`: `draw`, `learn_from`, `spring_install`.
- `world/season.rs`: the coaching blend per stage, `coach_caller`. `development.rs`: `coaching_mult`, `coachability_lean`.
- `world/folk/staffing.rs`: `deal_terms`, `candidates`, `offer`, `interview_read`. `world/folk/boss.rs`: `review_push`, `booster_push`, `draft_goals`.

## Related pages

- [development.md](development.md): how the coaching number turns into growth.
- [play-calling.md](play-calling.md): what the caller's skills do on each snap.
- [recruiting.md](recruiting.md): the recruiting side of coach ratings.
- [scouting.md](scouting.md): scouting staffs and their reads.
- [tendencies.md](tendencies.md): coaching styles and habits.
- [people.md](people.md): trust, morale and the room.
