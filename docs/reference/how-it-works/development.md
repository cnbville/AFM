# Development: how it works today

Checked against main at 6200524 with update 3 lanes 1 (bodies and the speed core), 3 and 4, and world reader lane R2 on 2026-10-29, the playoff snaps (minor bug 53) on 2026-11-01, and lane NB12 (growth from the depth chart, minor bug 247) on eab31aa, 2026-11-05. A college season's row and role after the season (minor bug 313) over main d994f96 on 2026-11-07. The strength staff's mix (minor bug 302) checked with that change on 2026-11-07. Growth from game reps and results (the results lane) checked over main e2a6533 on 2026-11-08.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`.

## Quick answers

- When does a player develop? Once a year, one checkpoint covering the season and the offseason after it, then he is a year older (`development.rs`, `develop_season`). High school players at their state's season start (`world/hs_year.rs`, calls `season::develop`), college players at fall camp (`world/college.rs`, calls `season::develop`), pros at camp (`world/pro/mod.rs`, `camp`).
- Does growth land all at once? No, by default. The checkpoint decides the year, then the growth of the attributes his position uses lands week by week through training (`SeasonContext.defer` = `!WorldOptions.yearly_growth`, default false; `training/mod.rs`, `land`, `close_year`).
- What is potential? The position overall of his ceilings: `overall_for(position, ceilings)` (`player.rs`, `Player::potential`). Every one of the 67 attributes has its own hidden ceiling (`player.rs`, `Player.ceilings`).
- Can I see why a step went the way it did? Yes, in the world log: the step keeps every term it drew (injury, form, the year's event and its size, the drive and its parts, veteran learning, growth rates, decline, fades) in `GrowthWhy`, taken by `take_why` right after `develop_season` on the same thread; it changes nothing (`development.rs`; `world/wlog/why.rs`, `growth`; [world-log.md](world-log.md)).
- How far can potential move in a year? The yearly drift is capped at 5 overall points, one band (`development.rs`, `MAX_POTENTIAL_SHIFT`). Injuries, a stalled body, a breakout lift, veteran learning and outgrowing a ceiling are outside the cap (`develop_season`).
- Until what age does potential move? Full plasticity to 17, 0.75 at 18-20, 0.5 at 21-23, 0.2 at 24-26, 0 from 27 (`development.rs`, `potential_plasticity`). After that ceilings only fall with ageing.
- Does a skill his position doesn't use still grow? Only if it is related (A6). A skill is clearly unrelated when neither his position nor any spot he really moves to uses it (`development.rs`, `unrelated`, `moves_to`: QB to WR, RB, S, TE; RB to WR, CB, S, LB; WR to CB, S, TE, RB; TE to WR, OT, EDGE, LB; OT to IOL, TE, DT; IOL to OT, DT; EDGE to LB, DT, TE; DT to EDGE, IOL, OT; LB to EDGE, S, RB, TE; CB to S, WR; S to CB, LB, WR; K and P to each other; LS to IOL, TE, LB). An unrelated skill gets no growth and no veteran learning, so a tackle's man coverage stays where it was at 14. The body and elusiveness (rated for his size, A2) always grow; in high school kicking and snapping always grow, since high school kickers and snappers play another spot. Related skills grow toward their ceilings; the ones his position uses land through training, the rest at the checkpoint (`training/mod.rs`, `defers`).
- Do high school corners learn ball skills? Yes (A7): a high school corner plays offense or returns kicks, so each high school year his elusiveness and vision ceilings rise 2.5% and grow without high school's technique lag (`develop_season`, `carries`).
- Do old-position skills fade? Yes, 1.5 points a year off rating and ceiling, for a non-physical skill his current position doesn't use (weight <= 0.01) but an old position held from age 18 used (weight >= 0.04) (`develop_season`, "Old skills fade"; `OLD_SKILL_FADE`).
- Breakout odds? 3.5% a year to 23, 2% to 27, 1% after, times personality and role multipliers (`develop_season`, `breakout_chance`). No breakout in a year with no games or a major injury.
- Slump odds? 6% a year, 16% after a hot year (`develop_season`, `slump_chance`). It costs 2-6 points on every non-physical attribute, half for K, P and LS.
- Does coachability matter? Yes, it scales how far coaching moves him from average, good or bad: `coaching_boost = 1 + (coaching_mult - 1) * (0.6 + 0.04 * coachability)` (`develop_season`).
- What is bloom? Years his growth windows shift: development age = age - bloom, drawn N(0, 1.1) clamped -2.5 to 3.0 (`player.rs`, `generate_prospect`; `develop_season`).
- When does decline start? Per attribute: `decline_age + longevity + position offset`, physical offsets clamped to ±1.5 (F15: a back's speed from 25.5) (`development.rs`, `decline_onset`, `declines`). Speed, acceleration, agility and flexibility from 27, strength 31, most technique early 30s, the mental reads 34-36 (`attributes.rs`, `ATTRIBUTE_PROFILES`).
- Does speed wait for his birthday? No (S4): before 21, speed and acceleration are lifted to at least (0.66 + 0.30 × grown) of their ceilings each checkpoint, grown = his height growth by the year's end (`speed_with_body`), so seniors have about 95-96% of their top speed by spring and late bloomers less.
- Does a season wear him? Yes (F14): `season_wear` × his share of a starter's season (his real snaps share, else starter 1, rotation 0.5, none without games) × level (NFL 1, college 0.5, high school 0): backs 1.0, quarterbacks 0.6 if their legs (mean of speed, acceleration, agility) are 78+ else 0.15, kickers, punters and snappers 0, the rest 0.35. Added after the year's decline, in the world and the class sim alike.
- Why can a page say "Starter" for a player who barely played? Before minor bug 313 (2026-11-07) a college season's row kept his fall camp role, the staff's camp ranking, which holds first- and second-year players back (see "What the world feeds in"); the game-day chart ranks by rating and practice alone, so a weaker upperclassman could be a camp starter and sit behind them all year. On seed 73 (scale 0.2, one season on the engine) 46 of 390 FBS camp-starter receivers caught 5 balls or fewer, and 15% of them played under 15 snaps a game. Now the row and his role say what he really played once the season is over (`world/college.rs`, `finish`, `real_role`): on the same world none of the rows' FBS starters at quarterback, back or receiver played under 15 snaps a game, 1% at tight end (`crates/afm-tools/src/bin/role_check.rs`).
- Does a player grow from how he plays? Yes, with growth from results on (`WorldOptions.results`, on by default): after each engine game the skills his plays test move toward the level his own results showed against what his own ratings should produce in this engine, skill by skill, and his snaps land in the week he plays them (`results/mod.rs`, `world_game`, `apply`, `reps`). See "Growth from results" below.
- Does playing time matter? Yes, for growth (`playing_time_growth`, or `snaps_growth` from his real snaps last season) and for potential's direction (`playing_time_potential`, `snaps_potential`).
- Who decides the playing time a step reads? The depth chart and the snaps he really played (lane NB12, minor bug 247). His role is his place on the depth chart at the step: on your team your order goes first at each position you set, then the staff's ranking for the rest, as on game day; other teams use their staff's camp ranking (`world/season.rs`, `college_roles`, `hs_roles`, `your_order`). His growth, potential's lean and veteran learning read last season's real snaps at the same level, as the NFL does: a first season at a level reads the role alone (`world/season.rs`, `reps_read`; `world/college.rs`, `finish`; `world/hs_year.rs`, `keep_reps`). Details under "What the world feeds in".
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
10. Speed with the body (S4), a high school corner's ball skills (A7), then growth toward the ceilings (unrelated skills skipped, A6).
11. Slump and poor-coaching losses land (`late_loss`), after growth, so growth can't hide them.
12. Ageing decline.
13. Old skills fade.
13a. The weight room (A4, A14; see [body.md](body.md)) and the season's wear (F14).
14. Tidy up: settled attributes halve their unreachable ceiling room; ratings and ceilings clamp 1-99; a rating above its ceiling lifts the ceiling.
15. Tendencies' yearly change (`tendencies.rs`, `develop_year`; see [tendencies.md](tendencies.md)).
16. Season row pushed (`SeasonRecord`), age + 1, and `grow` (the part of a year the daily clock has already given his body, A16) one less, never below 0. If the overall fell, a `Breakout` event is removed.

What it reads from the world: `SeasonContext` (`development.rs`): level, coaching (1-20), playing time, trouble, adjustment (first-year dip 0-1), situation (-1..1), payday, real snaps share (NFL, college and high school), style, scheme, tendency centres, morale, physical, medical, physical coaching (the strength staff's mix, 1-20, None without a strength staff) and strength_dev (his strength head against the level's typical one, rating points), focus, defer (minor bug 302).

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
- `pt_term`: Starter +0.15, Rotation +0.10, Redshirt 0, Practice squad -0.10, Backup -0.25, Unsigned -0.40 (`playing_time_potential`); from real snaps `-0.25 + 0.4 * clamp(share, 0, 1.15)` (`snaps_potential`). An NFL backup or practice-squad player aged 24 or under gets 0 instead (no sag).
- `setback` = 1.8 in a suspended, dismissed or lost season, else 0.
- Each attribute moves `drive * 1.7 * plasticity + N(0, 1.2 * plasticity)`. Physical attributes read the strength staff's mix (physical coaching) in place of coach in the two coaching terms (minor bug 302).
- Growth spurt: age 17 or under, 9% a year: physical ceilings + U(2, 6), times 1.3 for strength and 0.8 for the rest. Event `GrowthSpurt`.
- A stalled body takes no positive physical move.
- Cap: if the overall of the moves exceeds 5, all moves scale down so the shift is exactly 5 (`MAX_POTENTIAL_SHIFT`).
- Veteran learning, ages 21-31, not in trouble years: each attribute with `experience >= 0.6` (all reads and most technique) gains ceiling `U(0, 1.2) * experience * snaps_share * snaps_factor` and rating 0.8 of that. snaps_share: real snaps clamped 0.2-1.0, else Starter 1.0, Rotation 0.6, other 0.2. Not capped.
- Breakout lift: see "Breakouts". Not capped.
- Scouting reads the same plasticity to widen potential reads (`world/scouting/file.rs` and `world/pro/scouts.rs`, `potential_plasticity`).

## Growth toward the ceiling

`develop_season` and `GrowthCtx::rate`.

- Per attribute per year: `add = min(gap, gap * rate + 1.6 * rate, 12) + N(0, 0.5)` (`FINISHING_STEP` 1.6, `MAX_YEARLY_GAIN` 12). Skipped when gap <= 0 or age factor <= 0.01.
- `rate = growth_rate * age_factor * boost^coachability_p * snaps_mult^experience_p * work_mult * form * growth_multiplier * redshirt_lift * hs_lag * morale_mult * focus_mult * physical_mult`, clamped 0-0.5. boost is `coaching_boost` for technique and mental attributes and `physical_boost` for physical ones (minor bug 302). The `_p` exponents are the attribute's own profile values (`attributes.rs`, `AttributeProfile.coachability`, `.experience`).
- Age factor (`growth_age_factor`, on development age = age - bloom): 0.75 before the window, 1.0 inside it, `0.55^(years past the end)` after.
- Windows (`ATTRIBUTE_PROFILES`): speed 14-20, acceleration and agility 14-21, strength 15-24, stamina 14-24, accuracy and footwork 15-27/28, pocket presence 16-31, mental reads 16-30, competitiveness and motor 14-22.
- Base `growth_rate`: physical 0.18-0.24, technique 0.12-0.20, mental 0.06-0.14 (competitiveness 0.06 lowest).
- `coaching_mult = 0.75 + 0.025 * coach` (1 → 0.775, 10 → 1.0, 20 → 1.25); `coaching_boost = 1 + (coaching_mult - 1) * (0.6 + 0.04 * CO)`. `physical_boost` the same from physical coaching (coach where there is none).
- `work_mult = 0.75 + 0.025 * WE`.
- `snaps_mult = playing_time_growth * (0.7 + 0.3 * snaps_factor) * level_growth`. playing_time_growth: Starter 1.15, Rotation 1.0, Backup 0.8, Redshirt 0.85, Practice squad 0.85, Unsigned 0.55. From real snaps: `0.8 + 0.35 * clamp(share, 0, 1.15)` (`snaps_growth`).
- Physical attributes have experience 0.1, so playing time and D2 barely touch them (`attributes.rs`, `physical`).
- `redshirt_lift` 1.35 on physical attributes in a redshirt year.
- `hs_lag`: in high school physical 1.0, technical 0.65, mental 0.7, strength 0.6 (`hs_group_growth`, `HS_STRENGTH_GROWTH`). In college technical and mental 1.3 (`COLLEGE_TEACHING`).
- `morale_mult = 1 + 0.10 * clamp(morale, -1.5, 1.5)`; morale = his mood + 0.5 * room mood (`world/people.rs`, `morale`).
- `physical_mult = 1 + 0.08 * clamp(physical - 0.06 * strength_dev, -1, 1)`, physical attributes only. physical = 0.6 * z(strength staff) + 0.4 * z(professionalism); with no staff (high school) 0.4 * z(professionalism) - 0.2 (`world/people.rs`, `support_ctx`). The strength head's distance from the level's typical one comes out here, since it counts through physical coaching (minor bug 302), so this term keeps the level's typical head and his professionalism.
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

- Onset per attribute: `decline_age + longevity + offset`; for physical attributes the offset is clamped to -1.5..1.5 (`decline_onset`).
- Position offsets (`positions.rs`, `aging_offset`): QB +3.0, OT and IOL +1.5, K/P/LS +2.0, TE and DT +0.5, WR, EDGE and LB 0, CB and S -0.5, RB -2.5.
- Loss each year past onset: `decline_rate * years_past * decline_resistance * logNormal(0.3) * wear_mult` (wear_mult on physical only), off both rating and ceiling. `years_past = age + 1 - onset`, so the loss grows each year.
- `decline_resistance = 1.15 - 0.015 * WE` (10 → 1.0, 20 → 0.85).
- `wear_mult = 1 + 0.06 * max(wear - 3, 0) * clamp(1 - 0.15 * longevity, 0.5, 1.5)`.
- Wear rises from major injuries (+1.5) and from every season played (`season_wear`, see Quick answers; F14), in the world and the class sim alike (the class sim's own `add_wear` is gone).
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
- College coaching (C57): `0.2 * HC teaching(group) + 0.25 * the coordinator's side_quality + 0.4 * the room's teaching + 0.15 * (6 + 12 * facilities)`; the room is his position coach, half and half with a coordinator who came up in that group, the coordinator or head coach without one (`world/season.rs`, `develop`, `room_teaching`). Situation `(facilities - 0.5) * 0.15` plus background push.
- College physical coaching (minor bug 302): the same mix with the strength staff in the room's 0.4: `typical room teaching(group) + strength head - typical strength head`, where typical is the division's mean over programs with players (`develop`, `typical`, `strength_head`). At a typical staff it equals the coaching mix, so each level's physical averages stay where they were; the position coach no longer moves strength, speed or conditioning.
- Prep school: coaching 12, Rotation. Away (mission, military): coaching 2 at D2 level (`world/season.rs`, `develop`).
- Off-field trouble chance (college base `PIPELINE.college_trouble.chance` 0.03, high school 0.012) scaled by work ethic, coachability, professionalism, temper, background and engagement staff; trouble turns role into Backup (`world/season.rs`, `develop`).
- College and high school roles (lane NB12): the staff's camp ranking (overall plus `1.5 * N(0, 1)`; in college minus a first-year block of `4 + 6 * prestige`, `2 + 3 * prestige` in the second year, and `12 * pending dip`), by `depth_slots` per position, a redshirt only below the travel squad (`world/season.rs`, `college_roles`, `hs_roles`, `depth_chart`). On your team your order replaces it at each position you set: your players first in your order, then the staff's ranking for the rest, as the game-day chart takes it (`game/roster.rs`, `depth_chart`). At your high school it is your varsity chart: a player you moved to JV or the freshman team keeps the staff's place, which `squad_roles` turns into his squad's role (`hs_roles`). A player you list at another position keeps the better of the two places. A player you list in the two-deep (`two_deep`: twice the starters) isn't made a camp redshirt; one listed deeper keeps the staff's call (`world/season.rs`, `your_order`).
- A college season's row after the season (minor bug 313): when the season ends (`world/college.rs`, `finish`), each player at a team that played most of its games on the engine gets the role he really played on his row and as `last_role`: Starter when he started at least half the games he played and either a quarter of his team's engine games or every game he played (a starter hurt early), or when he played half a full starter's snaps (snaps over `SEASON_SNAPS` scaled to his team's games, as his reps are: a third receiver or a nickel corner the first snap's personnel leaves off); Rotation from a fifth of those snaps; Backup below. A start is being on the field for his side's first snap with the ball or its first without it (`CollegeSeason::starts`, counted with `world/tally/games.rs`, `starters`). A player who never got on the field keeps his camp role when he was out hurt at some point of the season (`training.hurt`, `others_hurt`); otherwise (benched all year, or sitting out a transfer or academic year the camp ranking still placed) he was a backup. A camp redshirt (settled with his eligibility, `played_as_redshirt`) and a kicker, punter or snapper keep the camp role (`real_role`; thresholds proposed, not confirmed). The step at the next camp reads his snaps, not this role; the role feeds the pages, the finder's college-starter counts, the staff's growth book and the off-season's people (mood, talks, a new coach's read). The high school row still keeps its camp role.
- College and high school snaps (lane NB12): when a season played on the engine ends, each player keeps his offensive and defensive snaps as a share of a full starter's season, `Person.reps` (`world/mod.rs`, `Reps`). College: `snaps / SEASON_SNAPS` (750), times the games of the team he played for (his camp step's, `played_for`: a January transfer has moved by the title game) over the ones it played on the engine (a team short of 30 players plays on the stand-in), none when fewer than half were (`world/college.rs`, `finish`, `engine_share`). High school: varsity snaps over `HS_STARTER_SNAPS` (550), not scaled: in a season on the engine every game is played there and a forfeit is a game nobody played; a school that played no game keeps none. JV and freshman games don't count, since only your school's are played (`world/hs_year.rs`, `keep_reps`). None for a kicker, punter or snapper, for a planned redshirt year of four games or fewer (high school: a transfer's sit year with no games), a suspension that kept him out of every game (`world/season.rs`, `sat_season`), a player with no college step that season (an early enrollee), or a season on the stand-in. The next step reads it as `SeasonContext.snaps` only at the same level, the season before, in a year he plays (no redshirt, sit or suspension); otherwise his role decides (`world/season.rs`, `reps_read`, `develop`). There is no opponent-strength weight below the NFL. On the seed 68 check world (scale 0.1), FBS every-down starters play about 800-900 snaps, the median camp starter 641; high school's median 432 and every-down starters about 550 (`crates/afm-tools/src/bin/growth_check.rs`).
- Measured on 4,000 synthetic prospects (talent z 1.5, to 18 with the test helpers; `tests/growth_reps.rs`): a true freshman you start +3.78 overall, the same year as a backup +3.00; a returning starter after a full season +3.78, after a backup's season (0.05) +3.12; a benched starter after a full season +3.70. Four years from 18 read a year behind: starting all four +11.05, sitting one +9.92, sitting two +9.42 (the year after sitting reads the bench's snaps).
- NFL: coaching = team coaching; situation = `reliance * (0.5 * cast + 0.5 * organisation)`, reliance QB 1.0, WR/TE/RB 0.7, others 0.4; snaps = last season's snaps, playoffs included, / `NFL_SEASON_SNAPS` x clamp(1 + 0.1 * regular-season opponent strength, 0.8, 1.2) plus spring snaps; role from the depth chart; unsigned coaching 6 (`world/pro/mod.rs`, `camp`). NFL physical coaching (minor bug 302): `0.6 * team coaching + 0.4 * (league mean coaching + strength head - league mean head)`, over the clubs playing this year; an unsigned player has none.
- First-year dip from `career.rs`, `first_year_adjustment`: step size x position difficulty, technique and mental readiness, body readiness, make-up, fit and luck, clamped 0-1. Set for rookies and players on a new team.
- A holdout's missed camp adds 0.3 to his dip, kept apart from the adjustment so it marks no first year: dip = clamp(adjustment + missed camp, 0, 1), so his form falls x0.955 and his performance 2.1 points (`world/pro/mod.rs`, `camp`; `development.rs`, `SeasonContext.missed_camp`, `develop_season`).
- The class sim (`career.rs`, `simulate_cohort` and its steps) calls the same `develop_season` with only level, coaching, role, trouble, dip, situation and payday set.

## In-season steps outside the checkpoint

- Bowl practices: FBS teams' first postseason game, up to 15 practices. Players aged 22 or under on their position's attributes close `room * 0.0027 * practices * youth * coach_mult * work_mult * U(0.5, 1.5)`, youth = clamp((23 - age)/5, 0, 1) (`development.rs`, `extra_practices`; `world/college.rs`).
- Spring breakouts: noticed players whose spring mix lands in the top share close 12-30% of their gap (each attribute `share * U(0.7, 1.3)`, max 0.9) on position attributes; under 0.5 overall it fizzles (`development.rs`, `spring_jump`; `world/hs_year.rs`, `jumps`). Recorded on the row (`record_spring_step`, `open_row_with_spring_step`). A spring step doesn't count as a "hot year" for slump odds.

## Growth from results

Cain's design of 2026-11-07 (brief `afm/game-reps/proposal.md` in the project folder). Code in `results/mod.rs` and `results/table.rs`; the fit in `crates/afm-tools/src/bin/results_fit.rs`; the A/B in `crates/afm-tools/src/bin/results_ab.rs`. Sizes not in the brief are Claude's, proposed, not confirmed: engine credibility as the weight, the season's share as the dose, the own normal, the league centre, `CENTRE_PRIOR` and `BAND`, `CHANGE`, the ceiling at his top and `AT_TOP`, `CAP`, `BIG_GAME`, `BIG_SEASON`, the fit's `SHIFT` 8 and t of 4.

- Switches (`results::Mode`, `WorldOptions.results`): `on` (default true), `dice` (item 14: `Kept`, the yearly dice stay with results on top, the default; `Replaced`, the dice on the skills results test give way to their average), `pull_down` (item 15: a season below his ratings pulls the skill down; default true), `practice` (item 16, waiting on Cain: practice snaps count as results at `training::practice::PRACTICE_RESULTS` 0.25 of a game snap; default false), `scale` (1.0).
- Which games: every engine game in the world, high school, college and NFL (`world/clock.rs`, `world/hs_year.rs`, `world/pro/games.rs` call `results::world_game`). Games on the stand-in have no plays and change nothing; nor does a player the world doesn't keep.
- The evidence (`trials`, `evidence`): each play gives each man on it trials on the skills it tests, 30 reads in all (`Read`): QB short, intermediate, deep and on-the-move throws (completion over expected), pocket presence (sacks per pressure), decisions (interceptions), throws under pressure; RB vision (rush yards over expected), broken tackles, yards after contact; ball security, hands, yards after the catch (RB, WR, TE); route running, release, contested catches, deep ball tracking (WR, TE); pass and run blocking (OT, IOL, TE), anchor (OT, IOL); power and finesse rush (EDGE, DT); block shedding (EDGE, DT, LB); man and zone coverage, run stops (LB, S, CB); tackling (front seven and secondary); short and long field goals; punt placement. Personal stats only. The plays record a `moving` flag so throws on the move are known (`game/stats.rs`, `PlayRecord.moving`).
- What his ratings should produce (`Fit`, `table::FIT` from `data/results_fit.csv`): per level (high school, FBS, FCS, small colleges, NFL), read and position, `E = a + w · (his 67 ratings, height, weight, arms, consistency, temperament, recovery) + d · z + e · cx`, fitted by least squares on the engine's own games (`results_fit -- 2000`: 2,000 games a level on synthetic teams, every man's ratings moved N(0, 8) game by game so each skill's own slope shows, and in the second half every man on another team). `b` is the weight on the read's own skill, `z` the opponent's strength in standard deviations of its level (`strength_z`: the mean overall of its 22 best men, `TEAM`), `cx` the play's context (the kick's distance, the punt's spot). It also keeps a trial's spread, a starter's season of trials, what the model leaves in a man's season (`tau`, and `tp` the part that followed him to another team) and the skill's spread among the level's regulars (`sk`).
- A read is on at a level and position only when its result moves with its rating in this engine (`b` > 0 with t of 4 or more, `Fit::on`). On 1831b94: high school 14 reads, FBS 17, FCS 12, small colleges 11, the NFL 15. Most QB passing reads (all but the NFL's short throws), the receivers' route, catch and contested reads and coverage are off because their stats barely move with the rating (minor bug 283); they turn on after a refit once that is fixed.
- His shown level on a trial (`excess`): (his result − E for him as he is) / `b`, in rating points of the skill. A man who plays exactly to his ratings shows 0.
- Judged against the league as it plays (`Book::centre`, `world_game`): in the world each game's shown level is taken against his level's season so far at his read and position among his peers on the read's skill (the same `BAND` of 10 rating points: 60-69, 70-79 and so on; every such man's mean, leaning on the fit's 0 until `CENTRE_PRIOR` 2 starter's seasons of trials are in; practice keeps its own), as real stats over expected are each season. After each game his whole season so far is read against the latest centre (`apply_in`, `Entry.c`), and his past seasons keep it too, so his own normal is read against his league. A shift his peers share moves nobody. Peers by skill matter because the fit is a straight line: without them the best pass rushers read under their ratings season after season, NFL starters lost rush and sacks a dropback fell 7 and 11% on seeds 77 and 78. Practice trials keep their own centre in the same season (`Entry.np`, `Entry.cp`, `Entry::centred`), and practice counts only in a season his checkpoint has opened (spring practice before it is left out, so it can't roll last season's ledger early). Without the centre at all, on seed 75 (scale 0.1, 4 seasons) the NFL's interior linemen fell and its defensive tackles rose season after season, each side's move feeding the other's, and NFL scoring fell from 20.9 to 17.6 points a team game.
- The move (`target`, `apply`): `scale × (mean shown level − his own normal) × weight(n) × min(1, n / season)`, at most `CAP` 12 points either way, with `pull_down` off never below 0. `n` counts trials weighted by the opponent (`opp_weight`: 1 + 0.1 × z, 0.8 to 1.2; item 5 at every level). `weight(n) = c² b² / (c² b² + tau² + sd² / n)`, `c` the most a season really changes a skill (`CHANGE` 3 rating points, `Mode.change`; the skill's spread among regulars, `sk`, where that is smaller): how much of what he showed a real change in his skill explains against teammates, scheme and luck, so small samples and noisy stats count less. A season's move then spreads at most `c / 2` however noisy the read. The first build had `sk` (about 9 on the line) here: on seed 75 seasons moved a line skill up to 4 points each way mostly on luck, seasons added up like a random walk, and NFL starters' power rush, finesse rush and block shedding spread 25 to 57% wider in 16 seasons (NFL scoring 20.9 to 20.2 points a team game). The last factor is the share of a starter's season he has played, so he grows snap by snap and one big game moves him only its part. The move is worked out on the season so far after every game; each game applies the change from what was already moved (what really moved: a rating at 1 or 99 moves less, `Entry.applied`), so game by game gives the same end as the season at once.
- His own normal (`Ledger::own`): the mean of his past seasons' shown level on the read, shrunk by how much of a season's mean follows the man (`(tp/b)²` against what his team and the noise add over his seasons). A man whose make-up always beats or trails his ratings is judged against that, so it doesn't lift or sink him every year. A first season at the read has none. Past seasons are kept while he plays (`Ledger::past`, rolled at each checkpoint by `Ledger::roll`).
- No ceiling limit: a skill pushed past its ceiling takes the ceiling with it (`apply_in`). At his top (within `AT_TOP` 0.5 of his ceiling) a move takes the ceiling with it either way; below it the ceiling stays, so a move is growth early or late that the yearly step evens out. Before, the ceiling only rose: every lucky season at the top stayed and every unlucky one grew back, a ratchet (NFL starters' block shedding rose 6 points in 16 seasons on seed 75). The body (speed, strength and the like) has no read and doesn't grow from results.
- Snaps land in the week he plays them (item 1, `reps`, `training::reps_land`): with results on, the yearly step opens a player's year at a backup's snaps rate (`development.rs`, `reps_base`: `snaps_growth(0)`; a redshirt, practice squad or unsigned year keeps its own) and each game's snaps, as a share of a starter's season (`season_snaps`: high school 550, college 750, the NFL 1,000) times the opponent weight, land the part of the year the rise in the rate gives (`reps_ratio` raised to each skill's experience weight). A full starter's season lands the old yearly snaps term in season instead of a year late; no snaps give no bonus. His plan's focus counts on this part as on the rest (`reps_land`'s `focus`, your team's plans). An NFL player's spring snaps land at camp (`world/pro/mod.rs`, `camp`). Kickers, punters and snappers keep the yearly step's role term (minor bug 264). A season whose snaps did not land in season (results off, a stand-in season, unsigned at camp, a save from before this) is read at the next opening as before, spring snaps with it, so its snaps count once (`Train` keeps flags bit 3 through `close_year`; `develop_season`). NFL camp opens the year this way only when the NFL's games run on the engine that season (`engine_from`).
- Item 14 with `Dice::Replaced` (`development.rs`, `develop_season`): on the skills results test at his level and position (`results::tested`), the year's form, slump loss and breakout lift take the dice's average instead of the draw; a lost season, trouble and injuries stay as they are.
- Named moments (`DevelopmentEvent`): `BigGame` when one game moves one skill `BIG_GAME` (0.3) points up, whatever else fell (`Moved.up`) (on his season row; in the news for college and NFL players and your high school's, "had the game of his season", `world_game`); `BigSeason` and `RoughSeason` when the season moved one skill `BIG_SEASON` (1.0) point either way (named on the row at his next checkpoint, `close`). Practice results name nothing. The season row keeps the overall points results moved him (`SeasonRecord.results`).
- Where it is kept: `World.results` (`results::Book`), one `Ledger` per player who played an engine game since his checkpoint, and each level's season so far per read and position (`Book.league`), saved with the world. The checkpoint closes the season (`close`), rolls it into his past and keeps it (`world/season.rs`, `develop`; `world/pro/mod.rs`, `camp`); a ledger two seasons old goes, and last season's league centres (`Book::sweep`, `world/clock.rs`).
- Measured (`results_ab 20000 1`, 20,000 synthetic careers from 14 through the NFL, results drawn from the fits): with `Kept`, pull down and `CHANGE` 3, a season's results move his overall with a spread of 0.07 points; a starter's top skill moves about 1 point either way at the 90th and 10th percentiles; a named big game comes in 17% of starter seasons, a big or rough season in 9% each; growth by level and role and rating levels by stage stay within 0.1 of today. In worlds (`results_check 4 <seed> 0.1`, off against on, seeds 77 and 78): growth by level and role within 0.15 a season, rating levels by stage within 0.3 (one 99th percentile 2 points). The reports are `afm/game-reps/ab-report.md` and `afm/game-reps/world-check.md` in the project folder.

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
- `career.rs`: `PIPELINE.college_trouble`, `first_year_adjustment`.
- `development.rs` (update 3, lane 1): `moves_to`, `unrelated`, `speed_with_body`, `season_wear`, the weight room and the corners' 2.5%.
- `world/mod.rs`: `WorldOptions.yearly_growth` (trial tooling), `WorldOptions.results` (growth from results).
- `results/mod.rs`: `CAP` (12), `CHANGE` (3), `AT_TOP` (0.5), `BIG_GAME` (0.3), `BIG_SEASON` (1.0), `CENTRE_PRIOR` (2), `BAND` (10), `opp_weight`, `Mode` (`change`); `data/results_fit.csv` (refit with `results_fit` after engine changes that move the reads); `training/practice.rs`: `PRACTICE_RESULTS` (0.25).

## Code against the docs

- `docs/development-model.md` says a major injury costs speed, acceleration, agility and jumping: the code also cuts balance and flexibility (`injury_sensitive`).

## Related pages

- [ratings.md](ratings.md): the attributes, overall, position weights.
- [body.md](body.md): body growth and the weight room.
- [training.md](training.md): how the year's growth lands week by week.
- [injuries.md](injuries.md): game and practice injuries.
- [tendencies.md](tendencies.md): habits' yearly change, run from the same step.
- [scouting.md](scouting.md): potential reads and hints.
- [people.md](people.md): traits, morale, trouble.
