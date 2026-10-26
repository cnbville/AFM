# Recruiting: how it works today

Checked against main at dfb8b02 with update 2 lane 8 on 2026-10-26.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`. "Recruiting" below is `world/recruiting.rs`.

## Quick answers

- How many stars go out a class? Fixed quotas of a 250,000 class (times world scale): 33 five-stars, 350 four-stars, 2,000 three-stars, 4,000 two-stars; everyone else 0 ("unranked") (`career.rs`, `PIPELINE.star_shares`; recruiting, `run_signing_stage`, `service_stars`).
- What do the services rank on? 0.25 × film + 0.75 × their upside projection, minus 12 × (1 - visibility)², plus a small physical-gift term (recruiting, `run_signing_stage`, `star_score`).
- Do stars read true ability? Partly. Film is his season's play (`last_performance`, else overall). The projection is a 12-accuracy scouting read of potential plus a lasting service bias (`film_raw`, `service_bias`; `scouting.rs`, `scout_player`).
- Do the services see a physical freak? Late and weakly: 0.15 film-SD per full Dynamic Link, times visibility. Staffs see it at 0.5 SD (`SERVICE_GIFT`, `STAFF_GIFT`).
- Does exposure matter? Yes. Visibility (0-1) cuts his star score and decides which staffs see him at all (`run_signing_stage`, `visibility`, `see`).
- Are kickers rated by the services? Capped at 3 stars. Their real stars come from the kicking and snapping camps (`SERVICE_SPECIALIST_MAX`, `camp_rankings`, `recruiting_stars`).
- How does a recruit pick a school? He holds the best offer by a utility of level, brand, money, closeness, playing time, academics, development and ties, plus room at his position (`utility_fit`, `RosterView::fit`, the deferred-acceptance loop in `run_signing_stage`).
- Whose wants decide? His decider's (`family.decider_wants`): himself, a parent, grandparents or a handler. In the portal he uses his own wants (`portal_day`, `own_wants`).
- How far does distance matter? Closeness = 1 / (1 + (miles/300)²): 1 at home, 0.5 at 300 miles, 0.12 at 800 (`closeness`).
- Does the head coach's recruiting rating matter? Yes, in the relationship a staff builds: 0.006 × (HC skill - 10) × time, HC skill = 0.6 recruiting + 0.4 relationships; the position coach's recruiting at 0.003 (`run_signing_stage`, `relationships`).
- Does the coach's network matter? Yes, for exposure: a high school coach's network / 20 is 0.2 of a recruit's visibility (`run_signing_stage`, `network`).
- When do recruits commit? Each gets a timing from personality and family: one of five summer/fall waves (Mar 20, Apr 30, Jun 15, Aug 31, Oct 31), the early period, or February (`commit_timing`; `world/clock.rs`, `commit_wave`).
- What's a decommit? A commitment that loses its place or is beaten at a signing stage, or a fired coach's commit reopening: 45% of 5-stars, 35% 4, 25% 3, 15% others, times loyalty and closeness (`decommit_after_firing`).
- What decides a portal entry? A yearly "lean" built from role, how buried he is and dated events, then a uniform draw against lean × era factor (`world/college.rs`, `lean_checkpoint`; recruiting, `portal_open`).
- How is NIL paid? On July 1, each collective's budget × 0.6 is split over the roster by role, position value and stars (`nil_payouts`, `nil_weight`).
- What NIL does a recruit get offered? Position market rate × a star factor × (budget / $10M)^0.8, capped by the collective and the position's top rate (`offer_for`, `position_nil`; `world/ncaa.rs`, `position_nil_max`).
- What grades does he need? Division I 2.3 core GPA to qualify, 2.0-2.299 academic redshirt, under 2.0 can't enroll; Division II 2.2; JUCO anyone (`ncaa_tier`).
- When do the services first rank a class? From its sophomore year: four updates a year (21 Jan, 12 May, 14 Jul, 20 Oct) for the classes behind the one the signing stages rate, which they take over at 20 March of its junior year (`world/scouting/services.rs`, `update_days`, `underclasses`; recruiting, `rate_underclass`).
- Is there a composite? From 2012, a 0.70-1.00 rating read off each class's national order: 5 stars from .9834, 4 from .89, 3 from .797, 2 from .70 (`world/scouting/services.rs`, `composite`, `BANDS`, `COMPOSITE_FROM`).
- Who predicts where a recruit goes? Each service's insider, from 2011, every Tuesday (`world/scouting/services.rs`, `insiders_week`).
- Do walk-ons exist? Yes: planned preferred walk-ons, local walk-on offers in February, open tryouts, and fall-camp scholarships for walk-ons who play (`class_plan`, `run_signing_stage`, `camp_walk_ons`).

## High schools and being seen

- Each state has its real top programs plus generated schools in real towns. Prestige 0-1; a powerhouse is prestige ≥ 0.75. Real schools start at 0.8 + 0.04 per rank (top five) + 0.08 if national (`world/high_school.rs`, `generate_state_schools`, `HighSchool::is_powerhouse`).
- Eight national powerhouses (IMG Academy, Mater Dei, St. John Bosco, Bishop Gorman, St. Frances, Duncanville, Buford, St. Thomas Aquinas) pull from out of state (`NATIONAL_POWERHOUSES`).
- High school transfers (yearly, `hs_transfer_day`): standouts are the top 3% of their state's age 15-17. Pull = 0.3 × prestige gain, ×1.4 in an NIL state, cut for boarding cost (IMG: family money, aid for the very best), cut to 0.35× by a strong local coach (relationships ≥ 13 and reputation ≥ 11), cut by home stability, private-school cost, a crowded depth chart (×0.5), lifted by competitiveness. Capped at 40%. The state's rule sets how much of a season he sits (`recruiting`, `high_school_transfers`; `world/high_school.rs`, `transfer_rule`).
- Homesick boarders go home (up to 30% a year); buried powerhouse backups at 16-17 transfer out 12% of the time; 0.4% reclassify up, 0.5% repeat a year (`high_school_transfers`).
- Visibility, 0-1, recomputed at every signing stage (`run_signing_stage`):
  `0.05 + 0.3 × school exposure + 0.2 × coach network/20 + 0.15 × camps + 0.25 × tape + traffic + viral + bloodline + honors + your promos`.
  - School exposure: his school's prestige (prep school 0.6, JUCO mover 0.7).
  - Coach network: his high school coach's (JUCO: the college head coach's); 0.4 with none.
  - Camps: 0.3 + 0.7 × family money. Tape: how far his film sits above the class mean, 0-1. Traffic: 0.1 × (state culture + 0.6).
  - Viral clip: +0.25, about 2% of kids (6% with a social-media parent); it gets a first look only.
  - Bloodline: NFL father +0.15, college +0.05. All-state honors: +0.04 each, up to 2.
  - Your calls as a high school coach (`world/scouting/hs.rs`, `promos`).
- What visibility does not read: his true potential, his body directly (the gift is separate), his grades.

## Star ratings and the services

- Film is aligned by position: each position's film is rescaled to the class-wide mean and SD, so every position's top looks the same (`run_signing_stage`, `pos_mean_sd`).
- The services' projection: a scouting read at accuracy 12, HS level, plus `service_bias` = 1.6 points per year older than the middle of his class (±1.5 years) + a lasting 9-point-SD misread keyed to the player. Each new look moves halfway toward what they now see; a JUCO mover is a fresh read (`service_bias`, `SERVICE_MATURITY`, `SERVICE_MISREAD`).
- Ratings happen at each summer wave and at the early period (or January enrollees before 2017); each rating replaces the last. February adds no new stars (`run_signing_stage`, `rate_now`). The younger classes are rated at the services' own four updates (next section).
- Specialists: K and P are ranked by the camps on their overall + 1.5 SD noise among those who attend (attendance 0.3 + 0.7 × money). Camp tiers by national rank: 5 stars top 25, 4 to 100, 3 to 250, 2 to 500. Long snappers: top 12/40/100/250. A school's best non-QB/K/P snapper with 0 service stars and LS build is found once a class and moved to LS (`camp_rankings`, `CAMP_TIERS`, `SNAP_TIERS`).
- Composite class rankings (signing days): 5★ = 99, 4★ = 92, 3★ = 85, 2★ = 78, else 70; a class scores Σ rating × exp(-n² / 200) down its signees, best first. Walk-ons and specialists don't count. Only FBS is headlined. The score is kept per program in `class_scores`, which athletic directors read (`class_rankings_news`).

## The services before the signing stages

`world/scouting/services.rs` and recruiting, `rate_underclass`:

- Update days: 21 January (winter), 12 May (spring), 14 July (summer), 20 October (fall). Each ranks every class from the one after the signing stages' class down to this year's sophomores (`update_days`, `underclasses`, `underclass_update`).
- Who is looked at: the class's top 8% by film, plus every kid with stars, camp stars, all-state honors, an offer or a commitment; a class under 10 kids is skipped. Each is read the way a senior is (film and projection, each new look moving halfway), on keyed streams that leave the kids' own untouched (`rate_underclass`).
- How deep a list goes, as shares of a full class's 5/4/3/2-star quotas: a sophomore class 0.6/0.6/0.25/0, after its sophomore spring and summer 1.0/0.9/0.5/0.1, a junior class 1.0/1.0/0.7/0.25 (`reach`).
- Each class's national order (stars, then the services' read) is kept with the one before, for the rank movement, and each service's list with it (`record`, `ClassRanks`). An old save catches up once, on loading only, rated as of the latest update (`catch_up`); a page never rates.
- Each service's own list, made at each rating and kept until the next (`boards_of`; read by `service_boards`): the composite plus its lean minus the services' mean lean on the kid (so the composite is their average), the lean a lasting misread of SD 0.012 + 0.0015 × (20 − skill) plus a little of the update's own, and its bias (big-program kids, with camp stars of 4+ or 15+ offers, +0.006; a speed of 80+ +0.006; size up to ±0.006; a 10-game last season +0.004). Stars by the composite's bands on that rating (`stars_of`), a specialist 3 at most; 60-100% as deep into the 2-stars as the shared list (`lean`).
- Underclass kickers, punters and snappers: no services' stars; the kicking camps rank those they saw on the camps' tiers, as short as the services' list (`recruiting::underclass_camps`).
- Insiders: picks on the open 4- and 5-stars and on 3-stars with 6+ offers (no specialists), in the two classes signing next. The read is the kid's race (`world/front_office/college.rs`, `race_at`) plus the insider's misread of each school (SD 0.018 + 0.0025 × (20 − skill), mostly lasting). A first pick needs a 0.08 lead (0.025 within 75 days of his decision), a change 0.06; a pick on a school that fell out of his top three is moved like a first pick or taken back. Confidence 50 + 550 × the lead, capped 50-95 (`insiders_week`). Judged when he commits; six years of records (`judge`, `insider_record`).
- Top schools: announced within 90 days of his decision day when 6+ schools offer and he is a 4-star or has 10+: 10 schools (14+ offers), 8 (10+) or 5; cut to 5 or 3 in the last 30 days. The list is kept by program, never in his order (`announced`). A list whose leader is now off it, or more than half gone from his race, is dropped (`insiders_week`).
- His decision day: his commit timing's date (a summer wave, the early signing period's Wednesday from `calendar::anchors::early_signing`, or February's first Wednesday) (`decision_day`). Screens show it only as a read: nothing without a signal, then a window (`crates/afm-game/src/lane_find/race.rs`, `decides_read`).

## Offers before the waves

- Five dated offer passes per class (`early_offers`, `EARLY_OFFER_PASSES`; dates in `world/clock.rs`): Jan 15 of the sophomore year, Jun 15 camps, Sep 1 junior year, Jan 15 after the junior season, junior days + 14. Only the top 6% of a class on film is looked at, ranked 0.6 film + 0.4 service read.
- Offers held by rank r: floor(most × e^(-r / scale)): by the last pass 24 for the top prospect, 21 for the 100th, 7 for the 1,000th, 0 for the 3,000th (the code comment says about 8 and 1, before the floor). The first three passes carry a quarter of their offers through 2000, then about 0.067 more a year, all from 2012 (the comment says before 2005 and by 2015).
- Which programs offer: FBS and FCS only, scored -6 × (how far the program sits above his projected level) + 1.5 × footprint in his state + 0.5 × closeness + noise.
- The first school to offer adds +0.02 to the relationship, later early offers +0.01 (`run_signing_stage`, `early_offer`).

## How a staff builds its board

- Who gets looked at: the top 25% of the class by max(film, projection - 6), plus anyone committed or rated 2+ stars (`run_signing_stage`, `interest`).
- Who a staff sees (`see`): (footprint[state] + 1.2 × local) × visibility × 1.6 + bonus, where local = max(HS pipeline, 0.6 × area pipeline, 0.7 × staff ties to his school); bonus 1.0 for 4-5 stars, else 0.15 × stars. Below 1.0 it's a roll. A staff always sees its own commits and every 2+ star in its own state.
- Skipped: D2/NAIA/D3 don't board 4-5 stars, FCS doesn't board 5-stars; NCAA non-qualifiers; below the school's own admissions bar unless a special admit (qualifier, GPA trending up, work ethic and coachability reads, stable home or mentor, no trouble, and the school has special admits left).
- Evaluation (`eval`): 0.6 × (film + noise) + 0.4 × (projection + noise) + 0.3 × film-SD × system fit + 0.5 × film-SD × gift seen × eye factor − 4 × grade risk × (1 − academic support) × (1 − 0.6 × hot seat) + 5 film-SD for its own commits.
  - Noise SD = 3.5 − 0.15 × (0.6 × position coach eye + 0.4 × HC eye − 10), clamped 1-6. Eye = `Coach::evaluation` (`world/coach.rs`).
  - Grade risk = (2.6 − GPA)/0.6 clamped 0-1; academic support = 0.3 + 0.7 × facilities (`academic_support`).
- Boards are per position, up to 40 × (planned signees + walk-ons + 1) deep. A JUCO takes at most 15 out-of-state recruits (`JUCO_OUT_OF_STATE_OFFERS`).

## Class plans (how many a staff signs)

- Target depth per position = real FBS roster counts (QB 5.8, RB 8.5, WR 16.2, TE 7.4, OT 9.4, IOL 10.1, EDGE 9.1, DT 8.4, LB 14.6, CB 10.2, S 12.0, K 3.5, P 2.0, LS 2.5; total 119.7) scaled to the program's limit (`real_roster`, `class_plan`).
- Expected returners: each player counts 1 − chance he's gone (out of seasons, early NFL entry read from last draft's 64th/138th overall bars: 85% / 35% / 4%, backup portal 0.18, rotation 0.07) (`leaving`, `gone`).
- Signees per position = max(this year's hole, half the hole two years out) (`plan_count`). K and P: one each on scholarship; the rest of the specialists walk on (`specialist_scholarships`).
- Slots are bounded by room under the limit, the yearly signing cap, and from 2025 a usual class of 30 (`HOUSE_CLASS_MAX`).
- Limits (`world/ncaa.rs`, `scholarship_rules`, `staff_cap`, `roster_limits`): FBS 85 with 25 a year (95 before 1992); from 2025 a 105 whole-roster limit; FCS 85 with 30 a year (105 from 2025 unless one of the 35 opted-out programs); D2 and NAIA 110, D3 115, JUCO 85. With no NCAA cap a staff signs at most FBS 39, FCS 33, D2/NAIA 42, D3 45, JUCO 56. NCAA penalties cut both.

## What a recruit weighs

Utility of a school (`utility_fit`), each grade on 0-1 (`world/program.rs`, `GradeCat`):

- Level: 1.6 × level value × (0.5 + wants.prestige + wants.development). Level value: FBS 1.0, FCS 0.55, JUCO 0.35, D2 0.3, NAIA 0.25, D3 0.2 (`level_value`).
- Brand: (0.25 + wants.prestige) × prestige_rel (capped at 1) × (0.4 + 0.6 × level value).
- Prestige grades: wants.prestige × mean(title chances, stadium, tradition, brand exposure). Facilities: 0.1 × grade.
- Money: wants.money × need × (1 − 0.35 × ambition drive) × 1.5 × (NIL / $1M)^0.6. Need = 0.25 + 0.75 × (1 − family money) (+0.2 after a family crisis).
- Ambitious families instead weigh the path to the pros: 0.35 × drive × wants.money on development and playing time.
- Closeness: (wants.close_to_home, +0.15 after a death or lost job) × closeness(miles).
- Playing time grade × (0.6 + 0.4 × his standing on that board); academics grade × (0.5 + 0.5 × GPA factor); development = mean(pro pipeline, coach prestige).
- Small pulls: Pioneer League −0.8 × (1 − 0.6 × money), Ivy −0.4; HS pipeline +0.12 × strength; the program's week of stories (`story_push`); his own stay-home lean (0 for 20% of kids, else 0.04-0.40) in his home state; his childhood team +0.18; an older brother there +0.15 (or −0.05); a system that shows him off up to +0.06; 5-stars get +0.25 × brand³ × level value (`FIVE_STAR_PULL`).
- Room at his position (`RosterView::fit`): he judges himself at overall + 6, against players with ≤ 2 seasons left who'll be 2 points better. Room = 1 − 0.5 × ahead/starters, or negative when buried. Weight (0.06 + 0.2 × wants.playing_time) × early weight (1 + 0.08 per star over 2). Plus the supporting cast his position leans on (a QB's receivers and line), weight 0.04 + 0.15 × wants.development.
- Ties on top: the relationship the staff built, his own feel for the school (SD 0.07, `FEEL_SD`), and the pull of a commitment (`fixed_at`, `pull`).
- Walk-on places carry no money, cost tuition (0.8 × (1 − 0.6 × money), ×1.77 out of state; free at PFL/Ivy) and, against a real scholarship, 1.4 × level value of what he turns down (specialists exempt) (`walk_on_cost`, `scholarship_given_up`).
- What it does not read: coach ratings directly (only through coach prestige grade, relationships and pipelines), the program's true roster strength beyond the units, his own true potential.

Who decides (`world/background.rs`, `generate_family`): himself 45%, a parent (or grandparents) 45%, a handler 6-12%. A parent adds money, closeness and academics to the boy's wants; grandparents closeness and academics; a handler money. A signing note says "the choice was his parent's" when his own wants would have picked another offer by 0.06 (`run_signing_stage`, `steered`). Steered players get a one-time 45% portal chance after year one (`portal_open`).

## Relationships and visits

- Each staff recruits 3 per planned spot + 3 at each position (`RECRUITED_PER_SPOT`, `RECRUITED_EXTRA`). The head coach's own targets are the top of each spot.
- Relationship = known × (0.025 × HC time + 0.006 × (HC skill − 10) × HC time + 0.003 × (PC recruiting − 10) × PC time) + 0.03 × staff tie to his school + 0.02 if committed there + early-offer bonus + 0.004 × contacts + chemistry (SD 0.03). Known = 0.35 + 0.65 × visibility. Home-state effort 1.4, out of state 0.85. Visited (official, any contact, or within 250 miles) gives full time (`run_signing_stage`, `relationships`).
- Official visits: five max. He takes them to the five schools he likes best, or the ones he really visited on their dates. A Summer(0) committer took none; Summer(1-3) took none before 2018 (junior officials), Summer(1) two after (`officials`).
- Dated visits (`recruiting_day`): official visits on Fridays outside dead periods, for the next class from Sep 1 (juniors Apr 1 - Jun 30 from 2018); fall officials go to a program at home that Saturday; 12% a weekend. Game-day unofficials within 250 miles 25%; drop-ins in evaluation weeks 20% (40% for his commit); junior days on Saturdays Feb-Mar to early offerers within 500 miles, 30%.
- Periods (`calendar/mod.rs`, `recruiting_period`): Feb-Mar quiet, mid-April contact, late May dead, late June-July dead, Sep-Nov evaluation, late Nov - mid Dec contact, dead around each signing day, January contact.

## Signing: the stages

- Stages (`SignStage`, dates `world/clock.rs`): five commitment waves Summer(0-4) (the enum's comment says 0-3; the code has five) on Mar 20, Apr 30, Jun 15, Aug 31, Oct 31; Early (early signing day; third Wednesday of December 2017-23, first Wednesday from 2024); JanuaryEnrollees (before 2017 only); Final (first Wednesday of February).
- Each stage runs the same matching: programs offer down their boards in rounds; each recruit holds the best offer and trades up; an offer at a full position is made only for a recruit about (1 − 0.75 × prestige_rel) film-SD better than the best left at an open spot, at most 2 over the plan; a D-I staff always finds 2 extra places for a 5-star (`next_offer`, `OVER_PLAN`, `OVER_CLASS`).
- Bidding: a school near the top of its board (rank < 0.3) can raise its NIL offer 1.5× to beat a held offer, within its cap (`run_signing_stage`, bidding block).
- Who decides at each stage: only recruits whose `commit_timing` has come. Commitment timing z = 0.8 × noise + 0.25 decides himself − 0.3 social-media parent − 0.3 handler + 0.03 × (love of game − 10) + 0.15 older brother + 4★ 0.15 / ≤2★ −0.2; cut points 1.34, 0.81, −0.11, −0.35, −0.57, −1.1 (`commit_timing`). A summer commitment is announced on a day of its own before the next wave (`announce_commitments`).
- A commitment holds +10 utility through the summer waves; at signing stages it holds by stickiness 0.18 × loyalty (halved when the head coach changed) (`pull`, `stickiness`).
- Waits for February (`waits_for_february`): 50% with a new head coach, 60% borderline students, 40% of 5-stars with a show-minded family, 3% others.
- January enrollees: era 3% to 15% (2000-17), 30% + 2.5%/yr to 45% after; ×1.6 5★, 1.3 4★, 0.6 ≤2★; ×0.35 under a 2.6 GPA; FCS ×0.4 (`early_enrollee_chance`).
- February only: a commit whose place ran out signs where there's room (no NIL), or walks on; then D-I staffs offer open walk-on places to the best unsigned ≤2★ seniors in their state (prestige order) (`run_signing_stage`, `scrambled`, preferred walk-ons).
- Not signed: JUCO movers stop after two years; prep school if good film (2.2 SD above mean) and GPA under 2.3, chance 0.25 + 0.4 × money; otherwise done (`run_signing_stage`, `None` branch).
- Enrollment: January enrollees in January, everyone else in June; a JUCO mover keeps at most 2 seasons used (`enroll`).

## Walk-ons

- Planned walk-ons fill a real squad for the level (FBS 119.7, FCS 108, D2 115, D3 107, NAIA 100, JUCO 80) under a scholarship limit; under the House limit they fill what the class leaves (`level_roster`, `class_plan`).
- Fall camp: walk-ons who started or rotated go on scholarship where the limit has room, at most one snapper on aid (not D3, Ivy, PFL). Open tryouts: 8% of FBS/FCS programs a year, 16% below, take the best unsigned new graduate in their state (`camp_walk_ons`, `tryout_sticks`).
- House era: 2024-25 walk-ons and the 2025 class's promised walk-ons don't count against the limit (`takes_a_place`).

## Academics and eligibility

- Core GPA tiers (`ncaa_tier`); test scores matter through the class of 2020 on a sliding scale 2.3 + (980 − score)/464 (`eligibility_gpa`).
- Summer school: 3+ star seniors at 1.85-2.0 or 2.15-2.3 are pushed over the line with chance 0.3 + 0.03 × (work ethic − 10) + 0.15 if 4+★ + 0.15 × school resources (`run_signing_stage`).
- Admissions bars and special admits per school (`world/program.rs`, `admissions`, `STRICT_ADMISSIONS`).
- In college: spring and fall term checks make a player ineligible with chance 0.06 × ((2.5 − GPA)/0.5) × (1.3 − support) (`world/college.rs`, `spring_term_check`, `fall_term_check`).
- Your own team's study hall moves GPA: each study-hall period in your week beyond an average staff's adds 0.003 to a flagged player's GPA (under 2.3, or in his first college season), each one fewer takes 0.003; college and high school only (`training/seat.rs`, `team_effects`; see [training.md](training.md)).
- Seasons: four in a five-year clock; from 2026-27 FBS/FCS get five seasons in five years (`world/eligibility.rs`, `seasons_allowed`, `clock_last`). Medical hardship: ≤ max(3, 30%) games (`hardship_games`). COVID year 2020 returners by role: starter 60%, rotation 45%, backup 25% (`covid_return_chance`). Missions and military pause the clock (`summer_roads`).

## The transfer portal

- The lean (`world/college.rs`, `lean_checkpoint`): set at fall camp from role (backup 0.18, rotation 0.07, starter 0.07/3, redshirt 0.12 × early weight), × buried (1 + 0.12 × (overall − level starter median), 0.6-2), × competitiveness, × 1.12 homesick (900+ miles). Midseason: 4+ games ×0.85, none ×1.15, mood and loyalty. Coaching change ×1.8 (`PIPELINE.portal_entry.coaching_change`). Bowl ×0.97, no bowl ×1.08. A blue-chip signed at his spot ×1.2. A poor NIL deal at your program ×1.35 (`nil_payouts`).
- Before the winter window the head coach talks rotation players into staying: × (1 − 0.03 × (relationships − 10)), 0.75-1.25 (`portal_open`, `sway`).
- Who enters (`portal_open`): dismissed players always; a school dropping football frees everyone; a standout below FBS moving up (`PIPELINE.transfer_chance` 0.15, ×1.6 when his collective is under $2M) when his play, level-discounted, beats the level above's starter median + 1; steered players 45%; anyone else on u < lean × era; an FBS starter at a program whose collective is under $1M leaves for money with chance 0.24 × his money want (NIL era only). Teammates from his high school leaving lift the lean ×1.15.
- Era factor (`era_factor`): permission era 0.5, early portal 0.8-0.9, 2021-22 1.0, 2023 1.2, 2024 1.45, January-window era 1.45 × 1.3.
- Windows (`world/clock.rs`, `portal_windows`): none before 2022 (players leave after their last game); 2022-24 a December window and an April one; from 2025 Jan 2-16 only. A head coach change opens a 30-day (2022-24) or 15-day (2025+) window five days after the carousel. Graduate transfers after May commencement. Players on teams still in the playoff are held until they're out.
- Landing (`portal_day`): a program wants him if he'd make its two-deep (three-deep with roster room) or beats its backups at an open starting spot, adds ≤ 10, under its limit; his play is read with a level discount (FCS 1, D2/JUCO 5, NAIA 6, D3 7) (`competition_discount`). Bar: the level's weakest-quarter starter − 2 (−3 more for a step down). He picks by own-wants utility + room; first choice falls through at ~1% per day waited. No school: returns home (40% if he played near the median, else 15%) or quits.
- Sitting out: before 2021, a transfer into FBS sits a year unless from JUCO, a graduate from 2006, or his school dropped football (`portal_day`, `sits`).

## NIL money

- Era: none before July 2021; market scale 0.39 (2021), 0.67, 0.84, 1.0 from 2024 (`world/ncaa.rs`, `nil_market`). Budget = market × boosters + revenue share (P4 0.75 × $20.5M × 1.04^yrs from 2025; G5 0.75 × $3.33M) (`nil_budget`, `revenue_share`).
- Boosters' target: P4 $4M + $14M × fans × max(recent, 0.3); other FBS $0.6M + $2.5M × fans; FCS $0.08M + $0.5M × fans; below $0.02M + $0.1M × fans. Base moves with win % and prestige, drifts 8%/yr to target, capped at $20M (`world/program.rs`, `nil_target`, `update_collective`; `COLLECTIVE_CEILING`).
- Recruit offer (`offer_for`): position rate (QB $650K, OL/DL $425K, RB $250K, LB $200K, WR $187.5K, DB $172.5K, TE $150K, specialists $25K) × star factor (5★ 2.5, 4★ 1.0, 3★ 0.3, 2★ 0.08, else 0.03) × (1.3 − 0.6 × board rank) × (budget/$10M)^0.8; cap 0.25 × 0.6 × budget (0.35 for a rising rich program's reach, ×1.6); never above the position max (QB $800K ... specialists $50K).
- Portal offer (`portal_nil`): budget × 0.6 × his weight / (roster weight + his), × (1 + 0.04 × points above the level median), 0.6-1.6.
- Payday (`nil_payouts`): weight = role (starter 3, rotation 1.2, else 0.3) × (1 + 0.25 × position value) × (1 + 0.1 × stars). Brand deals (`brand`, fame) are added. A top-3 earner over $250K who plays below the team median loses stability and dings the head coach's reputation; 8% get complacent (work ethic −1). Entering the portal ends his deal (`enter_portal`).
- Before NIL: rogue boosters (2% × fans a year at FBS, ×1.5 with a big booster) pay $150K/$50K/$15K by stars from a $50-300K pot; found with chance 1 − e^(−paid/$8M) over four years; penalties: 1 scholarship per $15K (3-30) over three years, bowl bans at $100K/$300K, a show-cause order of 1 + paid/$100K years (`world/ncaa.rs`, `before_matching`, `enforcement`, `ncaa_case`).

## Program grades, prestige and pipelines

- Prestige 0-1 = 0.15 tradition + 0.3 recent + 0.12 × coach rep/20 × √level scale + 0.12 facilities + 0.11 × √(boosters/$25M) + 0.12 fans + 0.08 conference (`world/program.rs`, `prestige_of`). prestige_rel = prestige / the division leader's prestige when history began (`world/staff.rs`, `update_programs`).
- Twelve grades, re-ranked within each division to 55-99 each year; fast grades (title chances, playing time, coach prestige, NIL) move 80% of the way, steady ones 15% (`update_programs`, `move_grade`, `GradeCat::speed`).
- Footprint per state: max(local reach, national reach, overflow spill) + 0.6 × pipeline. Reach in miles = 150 + 1100 × level reach × prestige_rel² + 250 × fans (`rebuild_footprint`).
- Pipelines grow only by impact (all-conference, a draft pick, an undrafted signing): state +0.12 × weight × room, high school +0.25, area +0.12; they fade 0.03 a year after 3 quiet years. The same impact adds 0.2 × weight to the staff's ties to that high school (`record_impact`, `fade_pipelines`; recruiting, `credit_impact`).

## Your desk

- Your college board, visits, portal offers and NIL promises replace your staff's in the same matching; recruits decide by the same code (`world/front_office/college.rs`; recruiting, `stage_board` use in `run_signing_stage`). `appeal` and `usual_nil_offer` are the reads your screens show.
- The race log: every Monday your office logs each followed recruit's top 6 schools by appeal and your staff's read of his interest in each, at most 400 recruits (a college follows its board's high school, prep and JUCO recruits; a high school coach his players with an offer or commitment). Interest 0-100 is logistic: 50 sits 0.10 below his best school's appeal in the first logged week, 0.06 appeal a step; the read adds a lasting lean (2.5 your school, 4.5 a rival, 2 a coach's own player) and a week's own (3.5, 5, 1.5), each 8% narrower a contact down to 40% (`world/front_office/race.rs`, `day`, `followed`, `interest`, `read`, `reader`, `SCHOOLS`, `MOST`).
- A school's appeal split into seven reasons (close to home, playing time, NIL money, coaching and development, winning and the name, academics, ties and feel; they add up to its appeal) (`world/front_office/college.rs`, `reasons`). The race page reads them as your staff's ranges (`crates/afm-game/src/lane_find/race.rs`, `weight_read`), and scores the seventh from public ties only (`ties_score`).
- Find recruits reads only your staff's files and public values (stars, composite, offers, visits, the public NIL value), never a true rating (`crates/afm-game/src/lane_find/fields.rs`, `value`). Core courses and academic risk read the transcript once your staff has it, the profile's GPA before (`core_behind`, `risk`). A name filter matches whole names (`crates/afm-game/src/lane_find/query.rs`, `names_match`). The public NIL value is $3,000 × e^(17 × (composite − .70)) × a position factor (QB 1.8; WR, RB, EDGE 1.1; CB 1.0; OT 0.85; IOL 0.75; K, P, LS 0.4; others 0.9) + $1.20 a follower (`nil_value`).
- A preferred walk-on signs no letter: your screens say "Walk-on" or "Walking on at" (`crates/afm-game/src/lane_frontoffice/college.rs`, `interest`).

## The fresh look's part (update 2, lane 8)

- Film and the services read through the same tape as every staff (SK1-SK5): each recruit's film carries his footage lean (dominance over weak schedules, a reel's tilt × (1 - visibility)) minus 0.3 × luck, and the services' projection the same lean (`Recruit.wide`, `Recruit.svc`); a staff's misread on film is × `wide` (`film_factor`) (`world/recruiting.rs`, the pool and `run_signing_stage`).
- A head coach seen at a recruit's game adds 0.05 visibility a trip, 0.1 at most for a year (`own::seen_lift`, in the pool's visibility sum).
- A program's pro day pull (SK20): `Program.pro_day` (up to 0.03) × (0.6 + development want + path weight) in `utility_fit`.
- Hard minimums (SK17): a head coach's height or weight line (keyed by the coach, `own::ai_minimums`; yours when set to drop) takes a recruit off his staff's board unless he is already committed there.
- How each works: [scouting.md](scouting.md), "The fresh look".

## Where the knobs are

- `career.rs`, `PIPELINE`: `star_shares`, `portal_entry` (backup 0.18, rotation 0.07, coaching change 1.8), `transfer_chance` 0.15, `college_trouble`.
- `world/recruiting.rs`: `STAFF_GIFT`, `SERVICE_GIFT`, `SERVICE_MATURITY`, `SERVICE_MISREAD`, `CAMP_TIERS`, `SNAP_TIERS`, `SERVICE_SPECIALIST_MAX`, `FEEL_SD`, `RECRUITED_PER_SPOT`, `RECRUITED_EXTRA`, `OFFICIAL_VISITS`, `CHILDHOOD_LEAN`, `FIVE_STAR_PULL`, `SHOWCASE`, `FIT_EVAL`, `WALK_ON_GIVE_UP`, `HOUSE_CLASS_MAX`, `JUCO_OUT_OF_STATE_OFFERS`, `REDSHIRT_PORTAL`, `EARLY_OFFER_PASSES`, `real_roster`, `level_roster`, `level_value`, `position_nil`, `nil_scale`, `competition_discount`, `era_factor`.
- `world/ncaa.rs`: `scholarship_rules`, `staff_cap`, `nil_market`, `revenue_share`, `position_nil_max`, `COLLECTIVE_CEILING`, `ROGUE_ARRIVAL`, `ROGUE_QUITS`, `DETECTION_SCALE`, `FCS_OPTED_OUT`.
- `world/program.rs`: `nil_target`, `prestige_of`, `division_scale`, `move_grade`, `STRICT_ADMISSIONS`.
- `world/scouting/services.rs`: `update_days`, `reach`, `BANDS`, `COMPOSITE_FROM`, `INSIDERS_FROM`, the insiders' thresholds in `insiders_week`, `lean`. `world/front_office/race.rs`: `SCHOOLS`, `MOST`, `BELOW`, `SPREAD`.
- `world/clock.rs`: `commit_wave`, `portal_windows`, early-offer dates. `calendar/anchors.rs`: signing days. `calendar/mod.rs`: `recruiting_period`.

## Related pages

- [scouting.md](scouting.md): how the scouting reads behind projections and your staff's ranges work.
- [staff-and-coaches.md](staff-and-coaches.md): coach recruiting, network, evaluation, relationships.
- [draft.md](draft.md): declaring and the draft bars the class plans read.
- [people.md](people.md): personality, families, fame and brand money.
- [ratings.md](ratings.md): overall and the scales behind film.
