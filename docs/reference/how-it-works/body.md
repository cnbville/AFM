# Body and combine: how it works today

Checked against main at 94a6553 on 2026-10-25.

## Quick answers

- Units? Height, arm, hand, wingspan in inches; weight in pounds (`body.rs`, `Body`).
- Is the stored body his body now? No. `Player.body` is the adult body he grows into; `body_now` turns it into his body at his age (`body.rs`, `body_now`). The engine, the combine and the player page all read `body_now` (`game/roster.rs`, `GamePlayer::from_player`; `body.rs`, `Drills::of`; `crates/afm-game/src/lane_core/pages.rs`, `measurables`).
- When does he reach full height? At 19 + 0.6 × bloom; before that he is up to 4 inches short, linearly (`body.rs`, `body_now`).
- How much weight a year? From 70% of adult weight at 13 to 95% at 22 (+0.5 × bloom), about 2.8% of adult weight a year, plus up to 5% more as strength reaches its ceiling (`body.rs`, `body_now`).
- How is the forty computed? 5.56 - 0.0124 × (0.75 × speed + 0.25 × acceleration) + 0.0033 × (weight - 220), then a stable track offset sd 0.04 and run noise sd 0.03, clamped 4.20-6.00, rounded to 0.01 (`body.rs`, `Drills::means`, `combine`, `tested`).
- Is the speed rating "speed for his size"? In effect, yes. The rating itself is never adjusted for size, but the stopwatch takes weight off on top: 0.0033 s per pound over 220 in the forty, and the same weight terms in the engine's top speed, burst and cuts (`body.rs`, `Drills::means`; `game/roster.rs`, `forty`, `sprint_tau`, `accel`, `cut`). So two 80-speed players run different forties at different weights, and one speed point is worth 0.0093 s, so 20 lb costs about 7 speed points of forty. A player who adds 20 lb while his speed rating grows 7 points runs the same forty (what the 2026-10-24 measurement found by class). Size also lowers speed at creation: a big frame gets a lineman's speed shape (`athlete.rs`, `draw_shape`).
- Does the match engine use the same forty? Yes, the same formula without noise, then top speed = 40 / max(forty - sprint_tau + 0.06, 3.4) yd/s (`game/roster.rs`, `forty`, `top_speed`).
- What reads arm length? The bench (-0.8 rep per inch over 32.5) (`body.rs`, `Drills::means`); also the engine and links (`game/roster.rs`, `GamePlayer.arm`).
- What reads height in fit? Position fit: height has no slack, weight has 25 lb (`athlete.rs`, `misfit`).
- Does lifting add weight? Only through the strength share in `body_now` (up to 5% of adult weight) and, for your own players, the body plan (`training/seat.rs`, `body_change`). The yearly development step never changes `body.weight` except the bad-weight event (`development.rs`, `develop_season`).
- What is the bad-weight event? Ages 18-32, 2.5% a year × a laxness factor: +8-25 lb, speed, acceleration and agility -1 to -3 now and ceilings -0.5 to -2 (`development.rs`, `develop_season`, `WeightGain`).
- Do high schoolers show arms and hands? No. The page shows arm, hand and wingspan only from college on, combine drills only for declared and pro players who were invited or drafted (`pages.rs`, `measurables`).
- Lefties? 10% throw left; 60% of lefties and 12% of righties kick left (`body.rs`, `LEFT_HANDED`, `RIGHTY_LEFT_FOOTED`).
- What is a "combine moment"? The invited player whose combine forty beats 4.25 + 0.0042 × (weight - 180)₊ by the most makes one news item, if he beats it by more than 0.12 s (`world/scouting/predraft.rs`, `combine_moments`).

## The body record

`Body` fields: `height`, `weight` (adult playing weight, moved by position and plans), `arm`, `hand`, `wingspan`, `frame_weight` (the natural weight before any position program; never changed after creation), `left_handed`, `left_footed` (`body.rs`, `Body`).

Position sizes come from the real combine (nflverse 2000-2025, `data/real-world/combine_by_position.csv`, read into `SIZES`): e.g. QB 74.66 in / 221.6 lb, RB 70.72 / 217.5, WR 72.75 / 201.2, OT 77.69 / 315.6, IOL 75.81 / 310.6, DT 74.99 / 306.8, CB 71.35 / 192.7 (`body.rs`, `combine_size`). The CSV has no LS row, so LS falls back to the default 73 in (sd 2) / 230 lb (sd 20) (`body.rs`, `SIZES`).

## How a body is created

Normal creation is athlete first (`player.rs`, `generate_prospect` calls `athlete::create`):

1. Frame, as if he were a draft prospect (`athlete.rs`, `draw_frame`): h = normal(0,1).
   - 29%: lineman build, height 75.9 + 1.5h, weight 311 + 4h + normal(0, 15).
   - 71%: lean build, height 73.1 + 2.1h, weight 220 + 26 × (0.55h + 0.83 × normal(0,1)).
2. Talent shrinks it (`athlete.rs`, `talent_size`): size = clamp((talent - 72)/12, -2.5, 0.5); inches added = 0.7 × size × 1.6; pounds added = (0.3 + 1.3 × size) × 14. Talent 45: -2.5 in, -37 lb. Talent 72: +0 in, +4 lb. Talent 78+: +0.56 in, +13 lb.
3. Height = clamp(frame + inches, 64, 82); weight = clamp(frame + pounds, 140, 380) (`athlete.rs`, `create`).
4. Arms, hands, wingspan follow height (`body.rs`, `Body::grown`): heavy = weight ≥ 285.
   - arm = 0.425 × height + (0.3 if heavy) + normal(0, 0.8).
   - hand = 9.45 + 0.08 × (height - 74) + (0.35 if heavy) + normal(0, 0.35).
   - wingspan = 2.03 × arm + 12.3 + normal(0, 1.0).
5. A coach then places him; `set_position` moves his weight toward the position (see below).

Fixed-position creation (tests and tools, `ProspectOptions.position` set) uses `adult_body` instead, on its own stream `id ^ 0x5EEDB0D1` (`body.rs`, `adult_body`): size as above from potential; h_z = 0.7 × size + normal(0,1); w_z = 0.3 + 1.3 × size + 0.45 × strength_z - 0.3 × speed_z + 0.25 × h_z + normal(0, 0.8), where strength_z and speed_z are his ceilings' gaps from the position's scaled elite shape / 4 (`player.rs`, `generate_prospect`). Height = mean + sd × h_z (64-82), weight = mean + sd × w_z (150-380). Arm offset +0.3 linemen (OT, IOL, DT, EDGE), -0.5 QB; hand +0.35 linemen.

## Weight by position

- On placement or a position move: weight = frame + clamp(0.6 × (target - frame), -25, +30), target = position combine mean + talent pounds (`athlete.rs`, `set_position`). A receiver moved to tight end eats; a tight end moved to tackle eats more.
- K and P keep their frame weight. LS is clamped to 220-265 lb (`athlete.rs`, `set_position`, `LS_MIN_WEIGHT`, `LS_MAX_WEIGHT`); older saves are clamped on load (`world/seat.rs`, `World::after_load`).
- Listed sizes on scouting screens run tall and heavy: round(height + 0.25 + u) and round(weight + 3 + 6u), u uniform 0-1 per player (`world/scouting/file.rs`, `listed`; shown by `crates/afm-game/src/lane_scouting.rs`). The player page's measurables are true `body_now` values (`pages.rs`, `measurables`).

## How a body grows by age

`body_now(p)` (`body.rs`):
- grown = clamp((age - 13 - 0.6 × bloom)/6, 0, 1); height = adult height - (1 - grown) × 4.
- filled = clamp((age - 13 - 0.5 × bloom)/9, 0, 1); weight = adult weight × (0.70 + 0.25 × filled + 0.05 × clamp(strength / strength ceiling, 0, 1)).
- arm and wingspan scale by height / adult height; hand × (0.9 + 0.1 × grown).
- Examples with bloom 0: at 14, 3.3 in short and about 73-75% of adult weight; at 17, 1.3 in short; full height at 19; the age part of weight is done at 22.
- Bloom is normal(0, 1.1), clamped -2.5 to 3 (`player.rs`, `generate_prospect`). A late bloomer (+2) reaches full height at about 20, full fill at 23.
- What it does not read: level, the weight room, coaching or nutrition. Only age, bloom and strength progress.

## Physical growth in the yearly step

Physical attributes grow like the rest (`development.rs`, `GrowthCtx::rate`, `develop_season`), with these physical-only parts:
- High school: physical growth at full rate (×1.0) except strength ×0.6 (`hs_group_growth`, `HS_STRENGTH_GROWTH`). Technical ×0.65, mental ×0.7.
- College: the ×1.3 teaching boost (`COLLEGE_TEACHING`) skips physical attributes.
- Redshirt year: physical growth ×1.35.
- Strength staff and his professionalism: physical growth × (1 + 0.08 × `ctx.physical`, -1..1) (`develop_season`, `physical_mult`).
- Growth by development age (age - bloom): ×0.75 before the window, ×1.0 inside it, ×0.55^(years past) after (`development.rs`, `growth_age_factor`). Windows: speed 14-20, acceleration and agility 14-21, strength 15-24, stamina 14-24 (`attributes.rs`, `ATTRIBUTE_PROFILES`).
- Growth spurt: age ≤ 17, 9% a year: physical ceilings +2-6 (strength ×1.3, others ×0.8) (`develop_season`, `GrowthSpurt`).
- Body stalls: ages 18-20, chance 2.5% × e^(-0.7 × bloom) a year (early bloomers more): each physical ceiling drops to at most current + 0-2 and never rises again (`STALL_CHANCE`, `BodyStalled`).
- Major injury: each injury-sensitive attribute (speed, acceleration, agility, balance, flexibility, jumping) loses 1.5-5 × age hit now (age hit 1.0 under 24, 1.2 at 24-27, 1.6 at 28+); 55% heal fully (back next year), 30% keep a lasting 1-3 off the ceiling, 15% lose 3-7 for good (`LostBurst`) (`develop_season`, `injury_age_hit`).
- Decline: onset = decline age + longevity + min(position aging offset, 1.5) for physical attributes. Speed 27, acceleration and agility 27, strength 31 (`development.rs`, `declines`, `develop_season`). Physical loss × wear multiplier 1 + 0.06 × (wear - 3)₊ × clamp(1 - 0.15 × longevity, 0.5, 1.5); a major injury adds 1.5 wear.
- Bad weight (18-32): chance 2.5% × lax, lax = clamp(1 + 0.12 × (10 - work ethic) + 0.06 × (10 - discipline/5), 0.3, 2.5). Effects in Quick answers (`develop_season`, `WeightGain`).

## The weight room and body plans (your own players)

- Practice families: Strength (strength and balance: the weight room), Speed (speed, acceleration, agility, flexibility, jumping), Conditioning (stamina) and others (`training/mod.rs`, `Family`). How much practice drives each family's growth: `PRACTICE_DRIVEN` [0.55, 0.3, 0.7, 0.5, 0.6, 0.5, 0.35]; a plan moves a family's year by at most 15% (`training/work.rs`, `PRACTICE_DRIVEN`, `PLAN_REACH`, `share`). Details belong to the training page.
- Body plan, each practice week: step toward the target weight of up to 1.5 lb (gain) or 1.2 lb (loss) × clamp(0.7 + 0.03 × work ethic, 0.7, 1.3) (`training/seat.rs`, `week`).
- Each pound changes (`training/seat.rs`, `body_change`): adult weight by lbs × adult/now (clamped 120-400), and attributes and ceilings by strength +0.05, anchor +0.04, speed -0.03, acceleration -0.02, agility -0.03, stamina -0.02. Ten pounds: +0.5 strength, -0.3 speed.

## Combine drills

The expected result from ratings and his body now (`body.rs`, `Drills::of`, `Drills::means`), with w = weight now - 220:

- Forty = 5.56 - 0.0124 × (0.75 speed + 0.25 acceleration) + 0.0033 w.
- Vertical = 20.9 + 0.182 × burst - 0.035 w, burst = max(jumping, 0.95 × acceleration).
- Broad = 87.6 + 0.448 × burst - 0.089 w (inches).
- Bench (225 lb reps) = 8.05 + 0.137 × strength + 0.073 w - 0.8 × (arm - 32.5).
- 3-cone = 7.58 - 0.007 × bend + 0.0062 w, bend = 0.55 agility + 0.25 acceleration + 0.2 flexibility.
- Shuttle = 4.59 - 0.0045 × quick + 0.0039 w, quick = 0.7 agility + 0.3 acceleration.

What they do not read: balance, stamina, any skill attribute, height (only through weight and arm).

Worked forties (no noise): speed and acceleration 90 at 200 lb = 4.38; 95 at 190 lb = 4.28; 60 at 310 lb = 5.11; 75 at 245 lb = 4.71.

Test day (`body.rs`, `combine`): forty + track offset (normal(0, 0.04), fixed per player from `id ^ 0x407AC4`, so some always run faster than they play) + normal(0, 0.03); vertical sd 1.5; broad sd 3.0; bench sd 2.0; 3-cone sd 0.08; shuttle sd 0.06. Then `tested` clamps and rounds: forty 4.20-6.00 and cone 6.40-9.00 and shuttle 3.80-5.50 to 0.01, bench 0-50 whole reps, vertical 18-46 to the half inch, broad 80-145 whole inches.

- `Drills::expected` is the same without any noise: what a rating range means on the hover card (format: "About a 4.48-4.55 forty at 205 lb"). It plugs the range's ends into one rating and your read's middle into the others (`crates/afm-game/src/lane_ratings.rs`, `combine_line`).
- Which day: the player page uses one fixed day seeded `id ^ 0x00C0FFEE` (`pages.rs`, `measurables`). Scouting files seed per kind and year; a pro day forty runs 0.05 s quicker, a camp laser forty 0.04 s slower (`world/scouting/file.rs`, `tests`).
- The doc comment says kickers skip agility drills (None), but `Combine` fields are plain numbers: every drill is computed for every position (`body.rs`, `Combine`).
- `combine_bench_mean` holds 13 values for 14 positions (no LS entry) (`body.rs`); only the `positions` tool calls it.

## Speed and size in the match engine

`GamePlayer` takes height, weight and arm from `body_now` (`game/roster.rs`, `from_player`).
- `forty()`: the combine formula, no noise or clamp.
- `top_speed()` = 40 / max(forty - sprint_tau + 0.06, 3.4) yd/s; a 4.35 is about 10.7 yd/s.
- `sprint_tau()` = clamp(0.78 + 0.003 × (60 - acceleration) + 0.0006 × (weight - 220), 0.55, 1.0) s.
- `accel()` = 4.65 + 0.008 × (acc - 60) at 60+, 4.65 + 0.0046 × (acc - 60) below, minus 0.006 × (weight - 220), clamped 2.2-7.5 yd/s².
- `cut()` = 0.99 + 0.004 × (agility - 60) at 60+, 0.99 + 0.0022 × (agility - 60) below, minus 0.002 × (weight - 220), clamped 0.4-1.7.
So weight costs a big man straight-line speed, burst and cutting, with the same speed rating.

## Athletic traits at creation (athlete first)

`athlete.rs`, `draw_shape` builds an "elite shape" (what each attribute would be if he became elite) before any position:
- Physical shape: a blend of the 11 field positions' elite shapes, each weighted by body likeness × the position's HS share, likeness = exp(-0.5 × (dh² + dw²)) in combine sds (`body_likeness`). A 310-lb frame gets a lineman's speed shape (55-66), a 190-lb frame something near the corner's and receiver's mid 90s.
- Then his own explosiveness e = normal(0,1): speed, acceleration, jumping +4e + normal(0,3); agility +2.5e + normal(0, 3.5); balance +1.5e + normal(0,4); flexibility +1e + normal(0,5); strength and stamina + normal(0,5).
- Skill leanings: 39 independent gifts (arm, placement, hands, cover, leg, IQ, compete...) mixed per attribute, mapped by `leaning(z)` = 38 + 59 × Φ(1.2 × (z - 0.3)) (about 40-97). A kicking leg is shifted -0.4 and helped by a kicker's or punter's body; long snapping is shifted -0.6.
- Natural position: argmax over K, P and the field positions of overall(shape) × (1 - misfit) × `FIT_SCALE` (`best_fit`).
- Ceilings (`create`): scale = talent / fit. Attributes his position uses: talent + (shape - fit) × spread, spread = min(0.5 × scale, (94 - talent)/(top - fit)); others shape × scale; all + normal(0, 4), clamped 1-99. Then skills only another position uses are trimmed until no other position rates over 95% of his natural one.
- Talent = clamp(45 + 6.5z + 2.75 × (z - 1.6)₊², 20, 99) (`player.rs`, `talent_from_z`).
- Ratings at his first age: maturity = clamp(maturity_at_14 - 0.035 × bloom + 0.03 × (age - 14) + normal(0, 0.05), 0.2, 0.98); raw = ceiling × maturity; above 40 it is halved: 40 + (raw - 40) × 0.5; capped at the ceiling (`player.rs`, `generate_prospect`).

## Where the knobs are

- `body.rs`: `SIZES` (from the combine CSV), `LEFT_HANDED` 0.1, `RIGHTY_LEFT_FOOTED` 0.12, arm/hand/wingspan coefficients in `Body::grown` and `adult_body`, `body_now` (4 in, 6 and 9 years, 0.70/0.25/0.05), `Drills::means` coefficients, test-day sds in `combine`, clamps in `tested`.
- `athlete.rs`: `draw_frame` (29% linemen), `talent_size`, `leaning`, `misfit`, `set_position` (0.6, -25/+30), `LS_MIN_WEIGHT`/`LS_MAX_WEIGHT`.
- `development.rs`: `HS_STRENGTH_GROWTH` 0.6, `COLLEGE_TEACHING` 1.3, redshirt 1.35, `STALL_CHANCE` 0.025, spurt 9%, weight gain 2.5% and 8-25 lb, `injury_age_hit`.
- `training/seat.rs`: `body_change` per-pound table, body plan 1.5/1.2 lb a week. `training/work.rs`: `PRACTICE_DRIVEN`, `PLAN_REACH`.
- `game/roster.rs`: `forty`, `top_speed`, `sprint_tau`, `accel`, `cut`.
- `world/scouting/file.rs`: pro day -0.05, camp +0.04, `listed`. `world/scouting/predraft.rs`: the combine-moment line 4.25 + 0.0042/lb and the 0.12 margin.

## Related pages

- [ratings.md](ratings.md): the 67 attributes, overall, position fit, level ratings, rating ranges.
