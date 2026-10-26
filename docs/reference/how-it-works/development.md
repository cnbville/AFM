# Development: how it works today

Checked against main at 9a53e5d on 2026-10-26.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`.

## Quick answers

- When does a player develop? Once a year, one checkpoint covering the season and the offseason after it, then he is a year older (`development.rs`, `develop_season`). High school players at their state's season start (`world/hs_year.rs`, calls `season::develop`), college players at fall camp (`world/college.rs`, calls `season::develop`), pros at camp (`world/pro/mod.rs`, `camp`).
- Does growth land all at once? No, by default. The checkpoint decides the year, then the growth of the attributes his position uses lands week by week through training (`SeasonContext.defer` = `!WorldOptions.yearly_growth`, default false; `training/mod.rs`, `land`, `close_year`).
- What is potential? The position overall of his ceilings: `overall_for(position, ceilings)` (`player.rs`, `Player::potential`). Every one of the 67 attributes has its own hidden ceiling (`player.rs`, `Player.ceilings`).
- How far can potential move in a year? The yearly drift is capped at 5 overall points, one band (`development.rs`, `MAX_POTENTIAL_SHIFT`). Injuries, a stalled body, a breakout lift, veteran learning and outgrowing a ceiling are outside the cap (`develop_season`).
- Until what age does potential move? Full plasticity to 17, 0.75 at 18-20, 0.5 at 21-23, 0.2 at 24-26, 0 from 27 (`development.rs`, `potential_plasticity`). After that ceilings only fall with ageing.
- Does a skill his position doesn't use still grow? Yes. The growth loop runs over all 67 attributes toward their ceilings, whatever the position weight (`develop_season`, "Growth towards the ceiling"). Only the attributes his position uses are held back to land through training; the rest land at the checkpoint (`training/mod.rs`, `defers`).
- Do old-position skills fade? Yes, 1.5 points a year off rating and ceiling, for a non-physical skill his current position doesn't use (weight <= 0.01) but an old position held from age 18 used (weight >= 0.04) (`develop_season`, "Old skills fade"; `OLD_SKILL_FADE`).
- Breakout odds? 3.5% a year to 23, 2% to 27, 1% after, times personality and role multipliers (`develop_season`, `breakout_chance`). No breakout in a year with no games or a major injury.
- Slump odds? 6% a year, 16% after a hot year (`develop_season`, `slump_chance`). It costs 2-6 points on every non-physical attribute, half for K, P and LS.
- Does coachability matter? Yes, it scales how far coaching moves him from average, good or bad: `coaching_boost = 1 + (coaching_mult - 1) * (0.6 + 0.04 * coachability)` (`develop_season`).
- What is bloom? Years his growth windows shift: development age = age - bloom, drawn N(0, 1.1) clamped -2.5 to 3.0 (`player.rs`, `generate_prospect`; `develop_season`).
- When does decline start? Per attribute: `decline_age + longevity + position offset` (physical offsets capped at +1.5) (`development.rs`, `declines` and "Ageing"). Speed, acceleration, agility and flexibility from 27, strength 31, most technique early 30s, the mental reads 34-36 (`attributes.rs`, `ATTRIBUTE_PROFILES`).
- Does playing time matter? Yes, for growth (`playing_time_growth`, or `snaps_growth` from a pro's real snaps) and for potential's direction (`playing_time_potential`, `snaps_potential`).
- Does the level matter? Small-college players (`Level::SmallCollege`: D2, D3, NAIA and JUCO, `world/mod.rs`; "D2" below means all of them) grow game-learned skills slower: `level_growth` = 0.72 there, 1.0 elsewhere. High school slows technique (0.65), mental (0.7) and strength (0.6); college speeds technique and mental by 1.3 (`GrowthCtx::rate`).
- Can bad coaching make a player worse? Yes. "Poor coaching" takes 1-3 points off each technical attribute his position uses (`develop_season`, "Teaching that costs ability").
- Does morale move growth? Yes, `1 + 0.10 * morale` with morale clamped to -1.5..1.5, so up to 15% either way (`develop_season`, `morale_mult`; `world/people.rs`, `morale`).

## The yearly step in order

All in `development.rs`, `develop_season(player, ctx, rng)`. The steps run in this order on one random stream per player.

1. Hidden game traits drift (`player.rs`, `GameTraits::age_one_year`). See "Hidden traits".
2. Rehab payback: if last year's row had a major injury and not a lost burst, each injury-sensitive attribute regains `min(room under ceiling, U(1.5, 5.0) * injury_age_hit(age - 1))`.
3. First year at a new level: `ctx.adjustment > 0` adds the `FirstYear` event (a mark only).
4. Injury roll (below).
5. Body stall check (below).
6. Form and the year's event: suspension/dismissal, breakout, lost season, slump or stagnation (below).
7. Weight gain roll (below).
8. Poor-coaching roll (below).
9. Dynamic potential drift, then veteran learning, then the breakout lift.
10. Growth toward the ceilings.
11. Slump and poor-coaching losses land (`late_loss`), after growth, so growth can't hide them.
12. Ageing decline.
13. Old skills fade.
14. Tidy up: settled attributes halve their unreachable ceiling room; ratings and ceilings clamp 1-99; a rating above its ceiling lifts the ceiling.
15. Tendencies' yearly change (`tendencies.rs`, `develop_year`; see [tendencies.md](tendencies.md)).
16. Season row pushed (`SeasonRecord`), age + 1. If the overall fell, a `Breakout` event is removed.

What it reads from the world: `SeasonContext` (`development.rs`): level, coaching (1-20), playing time, trouble, adjustment (first-year dip 0-1), situation (-1..1), payday, pro snaps share, style, scheme, tendency centres, morale, physical, medical, focus, defer.

## Ceilings and potential at creation

- Ceilings come first: athlete-first creation (`athlete.rs`, `create`) or, for a fixed position, the position's elite shape scaled to talent plus N(0, 4) (`player.rs`, `generate_prospect`).
- Talent: `45 + 6.5 z + 2.75 * max(z - 1.6, 0)^2`, clamped 20-99 (`player.rs`, `talent_from_z`).
- Starting ratings at 14: `ceiling * maturity`, maturity = `maturity_at_14 - 0.035 * bloom + 0.03 * (age - 14) + N(0, 0.05)`, clamped 0.2-0.98; above 40 the excess is halved (`generate_prospect`). Early bloomers (negative bloom) start more mature.
- `maturity_at_14` per attribute: speed 0.80, strength 0.48, mental reads about 0.38-0.45, competitiveness 0.68 (`attributes.rs`, `ATTRIBUTE_PROFILES`).
- `original_potential` keeps the potential at creation, debug only (`player.rs`).

## Dynamic potential (ceiling drift)

`develop_season`, "Dynamic potential". Runs only while `potential_plasticity(age) > 0`.

- Drive, one number per year:
  `drive = 0.75 * (WE - 10)/10 + 0.25 * (CO - 10)/10 * ((coach - 10)/10 + 0.5) + 0.35 * (coach - 10)/10 + 0.25 * situation + pt_term * snaps_factor - setback + N(0, 0.9)`.
  WE work ethic, CO coachability, coach the 1-20 coaching value.
- `pt_term`: Starter +0.15, Rotation +0.10, Redshirt 0, Practice squad -0.10, Backup -0.25, Unsigned -0.40 (`playing_time_potential`); from pro snaps `-0.25 + 0.4 * clamp(share, 0, 1.15)` (`snaps_potential`). An NFL backup or practice-squad player aged 24 or under gets 0 instead (no sag).
- `setback` = 1.8 in a suspended, dismissed or lost season, else 0.
- Each attribute moves `drive * 1.7 * plasticity + N(0, 1.2 * plasticity)`.
- Growth spurt: age 17 or under, 9% a year: physical ceilings + U(2, 6), times 1.3 for strength and 0.8 for the rest. Event `GrowthSpurt`.
- A stalled body takes no positive physical move.
- Cap: if the overall of the moves exceeds 5, all moves scale down so the shift is exactly 5 (`MAX_POTENTIAL_SHIFT`).
- Veteran learning, ages 21-31, not in trouble years: each attribute with `experience >= 0.6` (all reads and most technique) gains ceiling `U(0, 1.2) * experience * snaps_share * snaps_factor` and rating 0.8 of that. snaps_share: pro snaps clamped 0.2-1.0, else Starter 1.0, Rotation 0.6, other 0.2. Not capped.
- Breakout lift: see "Breakouts". Not capped.
- Scouting reads the same plasticity to widen potential reads (`world/scouting/file.rs` and `world/pro/scouts.rs`, `potential_plasticity`).

## Growth toward the ceiling

`develop_season` and `GrowthCtx::rate`.

- Per attribute per year: `add = min(gap, gap * rate + 1.6 * rate, 12) + N(0, 0.5)` (`FINISHING_STEP` 1.6, `MAX_YEARLY_GAIN` 12). Skipped when gap <= 0 or age factor <= 0.01.
- `rate = growth_rate * age_factor * coaching_boost^coachability_p * snaps_mult^experience_p * work_mult * form * growth_multiplier * redshirt_lift * hs_lag * morale_mult * focus_mult * physical_mult`, clamped 0-0.5. The `_p` exponents are the attribute's own profile values (`attributes.rs`, `AttributeProfile.coachability`, `.experience`).
- Age factor (`growth_age_factor`, on development age = age - bloom): 0.75 before the window, 1.0 inside it, `0.55^(years past the end)` after.
- Windows (`ATTRIBUTE_PROFILES`): speed 14-20, acceleration and agility 14-21, strength 15-24, stamina 14-24, accuracy and footwork 15-27/28, pocket presence 16-31, mental reads 16-30, competitiveness and motor 14-22.
- Base `growth_rate`: physical 0.18-0.24, technique 0.12-0.20, mental 0.06-0.14 (competitiveness 0.06 lowest).
- `coaching_mult = 0.75 + 0.025 * coach` (1 → 0.775, 10 → 1.0, 20 → 1.25); `coaching_boost = 1 + (coaching_mult - 1) * (0.6 + 0.04 * CO)`.
- `work_mult = 0.75 + 0.025 * WE`.
- `snaps_mult = playing_time_growth * (0.7 + 0.3 * snaps_factor) * level_growth`. playing_time_growth: Starter 1.15, Rotation 1.0, Backup 0.8, Redshirt 0.85, Practice squad 0.85, Unsigned 0.55. From pro snaps: `0.8 + 0.35 * clamp(share, 0, 1.15)` (`snaps_growth`).
- Physical attributes have experience 0.1, so playing time and D2 barely touch them (`attributes.rs`, `physical`).
- `redshirt_lift` 1.35 on physical attributes in a redshirt year.
- `hs_lag`: in high school physical 1.0, technical 0.65, mental 0.7, strength 0.6 (`hs_group_growth`, `HS_STRENGTH_GROWTH`). In college technical and mental 1.3 (`COLLEGE_TEACHING`).
- `morale_mult = 1 + 0.10 * clamp(morale, -1.5, 1.5)`; morale = his mood + 0.5 * room mood (`world/people.rs`, `morale`).
- `physical_mult = 1 + 0.08 * clamp(physical, -1, 1)`, physical attributes only. physical = 0.6 * z(strength staff) + 0.4 * z(professionalism); with no staff (high school) 0.4 * z(professionalism) - 0.2 (`world/people.rs`, `support_ctx`).
- Focus (your plan): the focus attribute `* (1 + 0.5 * take)`, others in its group `* (1 - 0.035 * take)`, take = clamp(0.6 + 0.02 * CO, 0.6, 1.0).
- Physical vs skill: the same formula, but physical attributes start more mature, grow faster early, have narrow windows, ignore snaps, get the redshirt lift and the strength-staff term, and are the ones a stall freezes and a major injury cuts.

## Form, breakouts, slumps and lost seasons

`develop_season`, "Form for the year". At most one event a year, checked in this order (an else-if chain).

- Base form: `logNormal(0.5) * (1 - 0.15 * dip) * (1 + 0.12 * situation * youth)`, youth 2 at 25 or under, else 1. `logNormal(sd)` is `exp(N(0, sd))` (`rng.rs`).
- On-field performance: `N(0, 2) - 7 * dip + 2.5 * situation * youth`, plus event swings; stored on the row as the season mean rating plus this (`SeasonRecord.performance`; empty in a no-games year).
- Trouble (suspended or dismissed): form x0.4, setback 1.8, performance -U(3, 7).
- Breakout: chance `base * (1 + 0.05 * (comp - 10) + 0.05 * (WE - 10)) * role`. Base 0.035 to 23, 0.02 to 27, 0.01 after. Role 1.5 for a starter aged 22 or under, 0.6 for a backup, else 1. Needs games and `snaps_factor >= 0.65`. Effect: form x2.2, performance + U(3, 7). If potential - overall < 6, ceilings on his position's attributes rise `U(2, 8) * age mult * U(0.6, 1.2)` (age mult 1.0 to 23, 0.6 to 27, 0.35 after; not physical if stalled).
- Lost season: chance `max(0.03 + 0.004 * (10 - WE) + 0.002 * (10 - comp), 0.005) * payday_mult`, only at 17-27 or after a payday. payday_mult = clamp(2.5 + 0.25 * (10 - WE), 1, 4.5) after a big NIL deal or contract (`ctx.payday`), adding `Complacent`. Effect: form x0.4, setback 1.8, performance -U(3, 7).
- Slump: needs games. Chance 0.16 if last year was hot (a breakout, or overall +6 over the year before not counting a spring step), else 0.06. Effect: form x0.6, performance -U(4, 9), and every non-physical attribute loses U(2, 6) after growth (x0.5 for K, P, LS).
- Stagnation: chance `0.05 - 0.003 * (WE - 10) - 0.0015 * (comp - 10)`. Form x0.3.
- A breakout row whose overall still fell loses the `Breakout` label (a career year on the field).

## Other yearly setbacks

- Injury roll (the yearly step's own; game injuries are on [injuries.md](injuries.md)): chance `level_rate * position injury_risk * (0.55 + 0.045 * injury_proneness) * role`. level_rate HS 0.13, D2 0.16, FCS 0.17, FBS 0.19, NFL 0.24 (`level_injury_rate`). role 0.5 with no games or trouble, 1.15 starter, 0.9 else. injury_risk RB 1.4, TE and LB 1.1, QB 0.7, K/P/LS 0.25 (`positions.rs`, `POSITION_PROFILES`).
  - Severity: minor 62% (growth x0.95, snaps factor 0.9, 0-2 games), moderate 27% (x0.85, 0.65, 2-6 games), major 11% (x0.5, 0.2, half to all the season's games). Minor and moderate games missed `x (1.3 - 0.03 * recovery) x (1 - 0.15 * medical)`.
  - Major: on the injury-sensitive attributes (speed, acceleration, agility, balance, flexibility, jumping): 55% heal fully (lose U(1.5, 5) now, regained next year), 30% keep a lasting ceiling loss U(1, 3), 15% lose their burst (lose U(3, 7) now and on the ceiling, event `LostBurst`). All x `injury_age_hit`: 1.0 under 24, 1.2 at 24-27, 1.6 from 28. Also injury proneness +1 (max 20) and wear +1.5.
  - Season games per level for this: HS 10, D2/FCS 11, FBS 12, NFL 17 (`season_games`).
- Body stall: ages 18-20, chance `0.025 * exp(-0.7 * bloom)` a year (`STALL_CHANCE`). Physical ceilings drop to rating + U(0, 2) and never rise again (event `BodyStalled`, checked with `has_had`).
- Weight gain: ages 18-32, chance `0.025 * lax`, lax = clamp(1 + 0.12 * (10 - WE) + 0.06 * (10 - Discipline/5), 0.3, 2.5). Adds 8-25 lb; speed, acceleration and agility lose U(1, 3), their ceilings U(0.5, 2).
- Poor coaching: chance `(clamp(0.04 * (8 - coach), 0, 0.2) + clamp(0.15 * (-situation - 0.4), 0, 0.09)) * clamp(1.5 - 0.05 * CO, 0.5, 1.5)`. Takes U(1, 3) off each technical attribute his position uses.

## Ageing and decline

`develop_season`, "Ageing"; `declines`.

- Onset per attribute: `decline_age + longevity + offset`; for physical attributes the offset is `min(position offset, 1.5)`, so the cap only limits late decline.
- Position offsets (`positions.rs`, `aging_offset`): QB +3.0, OT and IOL +1.5, K/P/LS +2.0, TE and DT +0.5, WR, EDGE and LB 0, CB and S -0.5, RB -2.5.
- Loss each year past onset: `decline_rate * years_past * decline_resistance * logNormal(0.3) * wear_mult` (wear_mult on physical only), off both rating and ceiling. `years_past = age + 1 - onset`, so the loss grows each year.
- `decline_resistance = 1.15 - 0.015 * WE` (10 → 1.0, 20 → 0.85).
- `wear_mult = 1 + 0.06 * max(wear - 3, 0) * clamp(1 - 0.15 * longevity, 0.5, 1.5)`.
- Wear in the world rises only from major injuries (+1.5). The class sim also adds wear per season played (`career.rs`, `add_wear`: RB 1.0, running QB 0.6, other QBs 0.15, specialists 0, everyone else 0.35 a starter season; half that in a rotation season, none as a backup).
- Settled attributes (age factor <= 0.01 or past onset) halve the room between rating and ceiling each year, except injury-sensitive ones in a rehab year (`develop_season`, "Tidy up").
- `longevity` is N(0, 1.3) clamped -3 to 3.5 at creation (`generate_prospect`). Retirement also reads it (`world/pro/mod.rs`, `season_end`, with `career.rs`, `retire_age`).

## Hidden traits and personality in development

| Trait (1-20 unless noted) | What it does here | Source |
|---|---|---|
| Work ethic | growth (`work_mult`), potential drive (0.75 weight), breakout +5%/pt, stagnation, lost season, payday risk, slower decline, weight gain, bowl practices | `develop_season`, `extra_practices` |
| Coachability | how much coaching sticks, potential drive term, protects from poor coaching, focus take | `develop_season` |
| Competitiveness | breakout +5%/pt, stagnation, lost season | `develop_season` |
| Love of the game | nothing in the yearly step (retirement only) | `world/pro/mod.rs`, `season_end` |
| Injury proneness | injury chance x(0.55 + 0.045 * ip); +1 per major | `develop_season` |
| Recovery | games missed for minor and moderate injuries; falls U(0.1, 0.4) a year from 30 | `develop_season`, `GameTraits::age_one_year` |
| Consistency, temperament | only drift: + U(0, 0.25) and + U(0, 0.3) a year under 26 | `player.rs`, `GameTraits::age_one_year` |
| Bloom (years) | shifts growth windows, stall chance, maturity at 14 | `develop_season`, `generate_prospect` |
| Longevity (years) | decline onset, wear multiplier | `develop_season` |
| Professionalism | physical growth term (0.4 weight), off-field trouble chance | `world/people.rs`, `support_ctx`; `world/season.rs`, `develop` |
| Temper | off-field trouble chance | `world/season.rs`, `develop` |

- Traits mature yearly at 18-32: professionalism +1 and temper -1 at each person's own pace (0.08-0.2 a year), Pro Bowl or All-Pro seasons may grow ego (`world/people.rs`, `year_book`).
- A big NIL payout to a struggling top earner can cost 1 point of work ethic (8%) (`world/recruiting.rs`, `nil_payouts`).

## What the world feeds in

- High school coaching: `0.6 * head coach teaching(group) + 0.4 * staff`, staff = `4 + 12 * resources` or your seat school's real staff; situation `(resources - 0.5) * 0.1` plus background push (`world/season.rs`, `develop`).
- College coaching: `0.2 * HC + 0.35 * coordinator + 0.3 * position coach + 0.15 * (6 + 12 * facilities)`, each a coach's `teaching(group)`; situation `(facilities - 0.5) * 0.15` plus background push (`world/season.rs`, `develop`).
- Prep school: coaching 12, Rotation. Away (mission, military): coaching 2 at D2 level (`world/season.rs`, `develop`).
- Off-field trouble chance (college base `PIPELINE.college_trouble.chance` 0.03, high school 0.012) scaled by work ethic, coachability, professionalism, temper, background and engagement staff; trouble turns role into Backup (`world/season.rs`, `develop`).
- NFL: coaching = team coaching; situation = `reliance * (0.5 * cast + 0.5 * organisation)`, reliance QB 1.0, WR/TE/RB 0.7, others 0.4; snaps = last season's snaps / `NFL_SEASON_SNAPS` x clamp(1 + 0.1 * opponent strength, 0.8, 1.2) plus spring snaps; role from the depth chart; unsigned coaching 6 (`world/pro/mod.rs`, `camp`).
- First-year dip from `career.rs`, `first_year_adjustment`: step size x position difficulty, technique and mental readiness, body readiness, make-up, fit and luck, clamped 0-1. Set for rookies and players on a new team.
- A holdout subtracts 0.3 from the adjustment (`world/pro/mod.rs`, `camp`). As a negative dip this raises form x1.045 and performance +2.1 (`develop_season` reads `dip` straight).
- The class sim (`career.rs`, `simulate_cohort` and its steps) calls the same `develop_season` with only level, coaching, role, trouble, dip, situation and payday set.

## In-season steps outside the checkpoint

- Bowl practices: FBS teams' first postseason game, up to 15 practices. Players aged 22 or under on their position's attributes close `room * 0.0027 * practices * youth * coach_mult * work_mult * U(0.5, 1.5)`, youth = clamp((23 - age)/5, 0, 1) (`development.rs`, `extra_practices`; `world/college.rs`).
- Spring breakouts: noticed players whose spring mix lands in the top share close 12-30% of their gap (each attribute `share * U(0.7, 1.3)`, max 0.9) on position attributes; under 0.5 overall it fizzles (`development.rs`, `spring_jump`; `world/hs_year.rs`, `jumps`). Recorded on the row (`record_spring_step`, `open_row_with_spring_step`). A spring step doesn't count as a "hot year" for slump odds.

## Hand-off to training

- With `defer`, `develop_season` skips growth for attributes his position weights (`training/mod.rs`, `defers`) and not declining this year, still taking the noise draw. It stores the year's rate inputs in `player.train` (`Train::open`) and the row's end is `projected_overall`.
- Each week `training::land` lands a share of each family's year with `grow_share`; shares summing to 1 give the yearly step's growth exactly (`development.rs`, `grow_share`).
- A run of poor weekly prep multiplies that week's technique, kicking and mental shares by 1 − 0.024 × (Unprepared + 0.6 × Underprepared weeks among the last five), at most 12% off. The cut is lost for the year: `close_year` lands only the part of the calendar the weeks never reached (`training/prep.rs`, `growth_factor`, `slow`; `training/mod.rs`, `cover`, `close_year`).
- Rust from poor prep moves ratings outside the yearly step: up to 3 points off play recognition and anticipation and off his position's two main technique ratings, won back by prepared weeks; a position change gives it back first (`training/prep.rs`, `record`, `unrust`; `athlete.rs`, `set_position`).
- Opening the year keeps his practice grade, his last five prep words and his rust (`develop_season`, the `defer` step); `close_year` keeps his load, prep words and rust (`training/mod.rs`, `close_year`).
- At the next checkpoint `close_year` lands the rest at the default and sets the last row's end overall (`training/mod.rs`, `close_year`, called just before `develop_season` in `world/season.rs` and `world/pro/mod.rs`).
- Everything else (ceilings, injuries, events, ageing, fades, tendencies) settles at the checkpoint. Details on [training.md](training.md).

## Where the knobs are

- `development.rs`: `POTENTIAL_BAND`, `MAX_POTENTIAL_SHIFT` (5), `OLD_SKILL_FADE` (1.5), `FINISHING_STEP` (1.6), `MAX_YEARLY_GAIN` (12), `STALL_CHANCE` (0.025), `HS_STRENGTH_GROWTH` (0.6), `COLLEGE_TEACHING` (1.3), `level_injury_rate`, `level_growth`, `playing_time_growth`, `playing_time_potential`, `snaps_growth`, `snaps_potential`, `hs_group_growth`, `potential_plasticity`, `growth_age_factor`, `injury_age_hit`, plus the inline odds in `develop_season`.
- `attributes.rs`: `ATTRIBUTE_PROFILES` (maturity at 14, growth rate, window, coachability, experience, decline age and rate, injury sensitivity per attribute).
- `positions.rs`: `aging_offset`, `injury_risk` per position.
- `player.rs`: `talent_from_z`, bloom and longevity draws in `generate_prospect`, `GameTraits::age_one_year`.
- `career.rs`: `PIPELINE.college_trouble`, `first_year_adjustment`, `add_wear`.
- `world/mod.rs`: `WorldOptions.yearly_growth` (trial tooling).

## Code against the docs

- `docs/development-model.md` says RBs decline 1.5 years early and physical decline moves at most 1.5 years: the code caps only the later side, so RB physical decline starts 2.5 years early.
- It says a major injury costs speed, acceleration, agility and jumping: the code also cuts balance and flexibility (`injury_sensitive`).

## Related pages

- [ratings.md](ratings.md): the attributes, overall, position weights.
- [body.md](body.md): body growth and the weight room.
- [training.md](training.md): how the year's growth lands week by week.
- [injuries.md](injuries.md): game and practice injuries.
- [tendencies.md](tendencies.md): habits' yearly change, run from the same step.
- [scouting.md](scouting.md): potential reads and hints.
- [people.md](people.md): traits, morale, trouble.
