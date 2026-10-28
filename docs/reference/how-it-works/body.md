# Body and combine: how it works today

Checked against main at LANE1_SHA on 2026-10-28 (update 3, lane 1: bodies and the speed core).

## Quick answers

- Units? Height, arm, hand, wingspan in inches; weight in pounds (`body.rs`, `Body`).
- Is the stored body his body now? No. `Player.body` is the adult body he grows into; `body_now` turns it into his body at his age (`body.rs`, `body_now`). The engine, the combine and the player page all read `body_now` (`game/roster.rs`, `GamePlayer::from_player`; `body.rs`, `Drills::of`; `crates/afm-game/src/lane_core/pages.rs`, `measurables`).
- When does he reach full height? At 19 + 0.6 × bloom; before that he is up to 4 inches short, linearly (`body.rs`, `body_now`).
- How much weight a year? By position group (A12): backs, receivers, defensive backs, kickers 80% at 14, 90% at 17, 94% as a freshman (18), full at 21; quarterbacks, tight ends, linebackers, edge rushers, snappers 89%, 93%, full at 21; tackles, guards, centers and defensive tackles 87%, 90%, 94% at 19, 97% at 20, full at 22. Each player has his own offset from his group's share (A13) that fades to nothing by full (`body.rs`, `fill_share`, `FILL_SKILL`, `FILL_MID`, `FILL_LINE`, `fill_offset`).
- His own offset? sd 2 points of grown weight (3.2 for linemen), six tenths of it from his bloom (late bloomers light), and one player in seven drawn 2.5 times wider; fixed for life on the stream `id ^ 0xF111_0FF5`. So a Metcalf-type receiver arrives at 98% and a late-growing tackle at 78% who adds about 60 lb in college (`fill_offset`).
- How is the forty computed? 5.56 - 0.0124 × (0.75 × speed + 0.25 × acceleration) + 0.0033 × (max(weight, 175) - 220), then a stable track offset sd 0.04 and run noise sd 0.03, clamped 4.20-6.00, rounded to 0.01 (`body.rs`, `Drills::means`, `forty_weight`, `combine`, `tested`). Below 175 lb being lighter stops making him faster (S13); the engine's forty and the links read the same `forty_weight` (`game/roster.rs`, `forty`; `links.rs`, `forty`).
- Is speed tied to football talent? No (S1). Speed and acceleration are his own roll: his explosiveness on top of what boys built like him run (`athlete.rs`, `create`, `speed_base`, `SPEED_MIX`). At a spot whose overall counts speed, the roll leans toward his talent by just enough that among FBS players speed says nothing about skills (`speed_lean`): quarterbacks 0.25, tackles 0.32, defensive tackles 0.56, tight ends and linebackers 0.76, safeties 0.83, edge rushers 0.87, receivers and corners 0.89, backs 0.91. His other skills make up the rest of his talent, so a slow boy's skills sit higher and a raw burner's lower.
- How fast does he play? One number per player, from burst (acceleration) and top speed by position (S2b): interior linemen 70/30, edge and linebackers 60/40, backs, tight ends and quarterbacks 50/50, receivers, corners and safeties 40/60, none for kickers, punters and snappers (`body.rs`, `play_speed`). Stars, offers and depth charts read it from lane 4 on.
- Is the speed rating "speed for his size"? In effect, yes. The rating itself is never adjusted for size, but the stopwatch takes weight off on top: 0.0033 s per pound over 220 in the forty, and the same weight terms in the engine's top speed, burst and cuts (`body.rs`, `Drills::means`; `game/roster.rs`, `forty`, `sprint_tau`, `accel`, `cut`). So two 80-speed players run different forties at different weights, and one speed point is worth 0.0093 s, so 20 lb costs about 7 speed points of forty. A player who adds 20 lb while his speed rating grows 7 points runs the same forty (what the 2026-10-24 measurement found by class). Size also lowers speed at creation: boys built like linemen start from a lineman's mix of what their bodies run (`athlete.rs`, `speed_base`).
- Does the match engine use the same forty? Yes, the same formula without noise, then top speed = 40 / max(forty - sprint_tau + 0.06, 3.4) yd/s (`game/roster.rs`, `forty`, `top_speed`).
- What reads arm length? The bench (-0.8 rep per inch over 32.5) (`body.rs`, `Drills::means`); also the engine and links (`game/roster.rs`, `GamePlayer.arm`).
- What reads height in fit? Position fit: height has no slack, weight has 25 lb (`athlete.rs`, `misfit`).
- Does lifting add weight? Yes (A4, A14). Each year from 15 to 25 every player's weight room pulls his adult weight toward his position's weight (the combine average, less what lower talent takes off), a third of the remaining gap a year in college and the pros and an eighth in high school, × clamp(0.7 + 0.03 × work ethic, 0.7, 1.3) × (1 + 0.15 × strength staff support). It only adds: at most 30 lb past where his frame put him at placement and never past the position's weight, so a naturally big receiver keeps his size. Kickers, punters and snappers skip it (`development.rs`, "The weight room"; `athlete.rs`, `position_weight`, `placed_weight`). Your own players' body plans move them on top of it (`training/seat.rs`, `body_change`). The bad-weight event still adds 8-25 lb.
- What is the bad-weight event? Ages 18-32, 2.5% a year × a laxness factor: +8-25 lb, speed, acceleration and agility -1 to -3 now and ceilings -0.5 to -2 (`development.rs`, `develop_season`, `WeightGain`).
- Do high schoolers show arms and hands? No. The page shows arm, hand and wingspan only from college on, combine drills only for declared and pro players who were invited or drafted (`pages.rs`, `measurables`).
- Lefties? 10% throw left; 60% of lefties and 12% of righties kick left (`body.rs`, `LEFT_HANDED`, `RIGHTY_LEFT_FOOTED`).
- Where does the Body and testing tab get each number? The first known of: your staff's measuring and testing days (own players), the combine (declared and pro players who were invited, or a pro day for a drafted man who was not), the latest camp, showcase, pro day or combine your scouts saw, then the school's or team's listing (self-reported, height and weight only) (`crates/afm-game/src/lane_squadboards/body.rs`, `known`).
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

- On placement or a position move: weight = frame + clamp(0.6 × (target - frame), -25, +30), target = position combine mean + talent pounds (`athlete.rs`, `set_position`, `placed_weight`, `position_weight`). A receiver moved to tight end eats; a tight end moved to tackle eats more. Both ways, unlike the yearly weight room.
- K and P keep their frame weight. LS is clamped to 220-265 lb (`athlete.rs`, `set_position`, `LS_MIN_WEIGHT`, `LS_MAX_WEIGHT`); older saves are clamped on load (`world/seat.rs`, `World::after_load`).
- Listed sizes on scouting screens run tall and heavy: round(height + 0.25 + u) and round(weight + 3 + 6u), u uniform 0-1 per player (`world/scouting/file.rs`, `listed`; shown by `crates/afm-game/src/lane_scouting.rs`). The player page's measurables are true `body_now` values (`pages.rs`, `measurables`).

## How a body grows by age

`body_now(p)` (`body.rs`):
- grown = clamp((age - 13 - 0.6 × bloom)/6, 0, 1); height = adult height - (1 - grown) × 4.
- weight = adult weight × `fill_share(p, age)`: his position group's curve at his age (straight between whole years; 4 points less a year before 14) plus his own offset × fade, fade = clamp((full age - age)/(full age - 18), 0, 1), clamped 0.6-1.0. See Quick answers for the curves.
- arm and wingspan scale by height / adult height; hand × (0.9 + 0.1 × grown).
- Examples with bloom 0: at 14, 3.3 in short; at 17, 1.3 in short and a receiver at 90% of his weight, a tackle at 87%; full height at 19; receivers full at 21, tackles at 22.
- Bloom is normal(0, 1.1), clamped -2.5 to 3 (`player.rs`, `generate_prospect`). A late bloomer (+2) reaches full height at about 20 and arrives in college about 2 points lighter (3.5 for a lineman).
- What it does not read: level, coaching or nutrition (the weight room moves his adult weight instead).

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
- Speed with the body (S4): before 21 (and not after a stall), speed and acceleration are lifted to at least (0.66 + 0.30 × grown) of their ceilings at each checkpoint, grown = his height growth at the end of the year; the year's growth adds the rest. So a senior has about 95-96% of his top speed by senior spring and a late bloomer less (`development.rs`, `speed_with_body`).
- Decline: onset = decline age + longevity + the position aging offset clamped to ±1.5 for physical attributes, so a back's speed declines from 25.5, not 24.5 (F15). Speed 27, acceleration and agility 27, strength 31 (`development.rs`, `decline_onset`, `declines`). Physical loss × wear multiplier 1 + 0.06 × (wear - 3)₊ × clamp(1 - 0.15 × longevity, 0.5, 1.5); a major injury adds 1.5 wear, and every season played adds wear (F14; see [development.md](development.md)).
- Bad weight (18-32): chance 2.5% × lax, lax = clamp(1 + 0.12 × (10 - work ethic) + 0.06 × (10 - discipline/5), 0.3, 2.5). Effects in Quick answers (`develop_season`, `WeightGain`).

## The weight room and body plans (your own players)

- Practice families: Strength (strength and balance: the weight room), Speed (speed, acceleration, agility, flexibility, jumping), Conditioning (stamina) and others (`training/mod.rs`, `Family`). How much practice drives each family's growth: `PRACTICE_DRIVEN` [0.55, 0.3, 0.7, 0.5, 0.6, 0.5, 0.35]; a plan moves a family's year by at most 15% (`training/work.rs`, `PRACTICE_DRIVEN`, `PLAN_REACH`, `share`). Details belong to the training page.
- Body plan, each practice week: step toward the target weight of up to 1.5 lb (gain) or 1.2 lb (loss) × clamp(0.7 + 0.03 × work ethic, 0.7, 1.3) × fuel; fuel = clamp(1 + 0.15 × (nutrition minutes − an average staff's) / (theirs + 30), 0.9, 1.15) from the week's sessions with the dietitian (`training/seat.rs`, `week`).
- Each pound changes (`training/seat.rs`, `body_change`): adult weight by lbs × adult/now (clamped 120-400), and attributes and ceilings by strength +0.05, anchor +0.04, speed -0.03, acceleration -0.02, agility -0.03, stamina -0.02. Ten pounds: +0.5 strength, -0.3 speed.
- Your strength staff's testing days use the combine fit: `Drills::expected` for his ratings and body now plus a test day's noise (reps sd 1.5, forty sd 0.03, shuttle sd 0.04, vertical sd 0.8), the 10-yard split 0.375 × forty − 0.15, and lifts from a bench max read at 220 lb and scaled by (weight / 220)^0.67 (squat about 1.45 times it, clean 0.88) (`training/seat.rs`, `test_row`). Details on [training.md](training.md).

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
- Speed and acceleration ceilings (S1, S2, S11, S12; `create`): roll = lean × z(talent) + √(1 - lean²) × e, lean from `speed_lean` (see Quick answers). mix = the body-likeness blend of `SPEED_MIX` (what boys built like each position run before anyone picks them, set so drafted players run the real combine's forty), lead = the blend of each position's elite acceleration minus speed. Speed = mix - 0.25 × lead + 13 × roll; acceleration = mix + 0.75 × lead + 13 × roll. Natural kickers and punters use the kicker's (46.3) or punter's (50.8) mix with no lead and no lean, so they run about 4.91 like real ones.
- Other ceilings: scale = talent / fit. Attributes his position uses: talent + (shape - fit) × spread, spread = min(0.5 × scale, (94 - talent)/(top - fit)), then all of them move by the same amount so his overall at his natural spot is his talent; others shape × scale; all + normal(0, 4), clamped 1-99. Then skills only another position uses are trimmed until no other position rates over 95% of his natural one; the body and elusiveness are never trimmed (A2: elusiveness is rated for his size, so a Sewell keeps his moves).
- Talent = clamp(45 + 6.5z + 2.75 × (z - 1.6)₊², 20, 99) (`player.rs`, `talent_from_z`).
- Ratings at his first age: maturity = clamp(maturity_at_14 - 0.035 × bloom + 0.03 × (age - 14) + normal(0, 0.05), 0.2, 0.98); raw = ceiling × maturity; above 40 it is halved: 40 + (raw - 40) × 0.5; capped at the ceiling (`player.rs`, `generate_prospect`).

## The Body and testing tab (update 2, lane 9A)

What the game knows of a player's body, against three groups (`crates/afm-game/src/lane_squadboards/body.rs`):
- Sources, first known wins: your staff's measurements (height and weight; arms, hands and wingspan from college on) and its latest testing day (forty, split, vertical, shuttle, bench reps, and the lifts); the combine (or a pro day for a drafted man it did not invite), seeded `id ^ 0x00C0FFEE` as on the player page; the latest camp, showcase, pro day or combine your scouts saw (`world/scouting/file.rs`, `tests`); the school's listing (`file.rs`, `listed`) or the team's. Only the listing is self-reported; the rest is marked verified (`known`).
- His level: his position at his level, from every player there through `Drills::expected` (no test-day noise): high schoolers in his state and graduating class, the nation's class when the state has under 40; college by FBS, FCS or smaller; NFL players on a roster (`level_pool`).
- Drafted: every drafted player at his position the world remembers in full (alumni with a record, drafted pros), each with his combine (`drafted_pool`).
- The real combine: the position's mean and spread from `data/real-world/combine_by_position.csv` (2000-2025), read as a normal curve (`REAL`, `phi`). Arms, hands, wingspan and the split have no real column.
- Percentiles: the share of the group his number beats (faster is better on the clock), ties counted half (`pct_in`).
- Athletic score on 0-10: the mean percentile over size (height, weight), speed (forty), burst (vertical, broad), agility (3-cone, shuttle) and bench, divided by 10, with at least three numbers known; "better than" scores each of the group the same way (`view`).
- Built like him: the five drafted players nearest in height, weight, forty, vertical and 3-cone, each in the drafted pool's spreads, with at least three shared; match = 100 - 18 × distance (`view`).
- Weight over time: his weight in each season's kept read (the ratings page, "Kept reads"), each testing day and today. Games missed hurt and seasons with an injury count only seasons played out (`view`, `record_season`).
- What it does not read: true ratings. The level cloud is drawn from each player's expected drills, the norms of his level, with no names.

## Where the knobs are

- `crates/afm-game/src/lane_squadboards/body.rs`: `TESTS` (the tab's tests and the CSV columns), 40 players for a state's pool, 10 for a percentile, 3 numbers for a score or a match, the 18-point match step.
- `body.rs`: `SIZES` (from the combine CSV), `LEFT_HANDED` 0.1, `RIGHTY_LEFT_FOOTED` 0.12, arm/hand/wingspan coefficients in `Body::grown` and `adult_body`, `body_now` (4 in, 6 years), `FILL_SKILL`/`FILL_MID`/`FILL_LINE`, `fill_offset` (sd 0.02/0.032, 15% wide ×2.5, 0.6 bloom), `forty_weight` (175 lb), `play_speed` weights, `Drills::means` coefficients, test-day sds in `combine`, clamps in `tested`.
- `athlete.rs`: `draw_frame` (29% linemen), `talent_size`, `leaning`, `misfit`, `set_position` and `placed_weight` (0.6, -25/+30), `LS_MIN_WEIGHT`/`LS_MAX_WEIGHT`, `SPEED_SD` 13, `FBS_TALENT_SHARE` 0.13, `SPEED_MIX`, `speed_lean`.
- `development.rs`: the weight room (15-25, a third a year, an eighth in high school, +30 lb), `speed_with_body` (0.66 + 0.30 × grown).
- `development.rs` (yearly step): `HS_STRENGTH_GROWTH` 0.6, `COLLEGE_TEACHING` 1.3, redshirt 1.35, `STALL_CHANCE` 0.025, spurt 9%, weight gain 2.5% and 8-25 lb, `injury_age_hit`.
- `training/seat.rs`: `body_change` per-pound table, body plan 1.5/1.2 lb a week and the nutrition fuel (0.9-1.15), `test_row`. `training/work.rs`: `PRACTICE_DRIVEN`, `PLAN_REACH`.
- `game/roster.rs`: `forty`, `top_speed`, `sprint_tau`, `accel`, `cut`.
- `world/scouting/file.rs`: pro day -0.05, camp +0.04, `listed`. `world/scouting/predraft.rs`: the combine-moment line 4.25 + 0.0042/lb and the 0.12 margin.

## Related pages

- [ratings.md](ratings.md): the 67 attributes, overall, position fit, level ratings, rating ranges.
