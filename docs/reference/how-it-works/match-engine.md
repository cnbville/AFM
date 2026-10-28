# Match engine: how it works today

Checked against main at b54ac1a with update 2 lane 8 on 2026-10-27; drives, the clock held and the world log added by world reader lane R1 on 2026-10-28.

Paths are under `crates/afm-engine/src/`. Ratings are 1-99 unless a line says "20" (the 1-20 read against the level, see the quick answers). In a block, w runs from -1 (the blocker has won) to +1 (the defender has shed him). U is a uniform 0-1 draw, N(m, sd) a normal one.

## Quick answers

- Does the elusiveness check read weight? No. The first-contact miss roll is 0.5 Elusiveness + 0.3 Agility + 0.2 Speed (+ Sudden) against 0.6 Tackling + 0.2 Agility + 0.2 Pursuit (+ Sudden); neither man's weight is in it (`game/sim.rs`, `first_hit`). Weight comes in once he is wrapped up: the broken-tackle roll and the pile (`game/sim.rs`, `fight_tick`).
- Where does weight count? Top speed, burst and cut (`game/roster.rs`, `forty`, `sprint_tau`, `accel`, `cut`); momentum and hit force (`game/sim.rs`, `hit_effects`); broken tackles, 0.08 a pound from 215 lb, and the wrap-up, (mass - 215) / 300 on both sides, and the pile's push by mass (`fight_tick`); blocks, 0.08 a pound from 300 lb, 280 for rushers and shedders (`block_base`, `rush_power`, `shed_power`, `anchor`); the jump at the catch point, -0.035 inch a pound over 220 (`game/scrimmage.rs`, `high_reach`); a contact-shy tackler passing on a man 15+ lb heavier (`game/sim.rs`, `tackles`).
- How fine is the sim? 22 tracked bodies, 10 ticks a second (`game/geom.rs`, `DT`; `game/scrimmage.rs`, `run_snap`).
- What decides a sack? A free rusher within 1.25 yards, or one winning his block (w > 0.45) within 1.2, then an escape roll: 0.4 Elusiveness + 0.3 Agility + 0.3 Strength against 0.6 Tackling + 0.2 Strength + 0.2 Agility (`game/scrimmage.rs`, `sack_check`).
- What decides a catch? A base miss of max(0.15 - 0.00095 × Catching, 0.012), then how far off his frame the ball is, the fight within 1.4 yards, weather and a hard ball (`game/scrimmage.rs`, `catch_point`).
- What counts as a drop? An incompletion on a ball inside 0.8 of his catch radius with no defender within a yard of it (`catch_point`).
- Does the catch read Concentration? No; nor Toughness, Route Running or Speed (`catch_point`).
- How does a pick happen? Five ways: a tip in the throwing lane, a ball off target into a defender, a defender in front at the catch point, a breakup that sticks (6% of breakups), a ball off the receiver's hands (`catch_point`).
- What sets the fumble chance? 0.018 × (hit force / 1700)^1.5, scaled by the hitter's 0.5 Strength + 0.5 Tackling against the carrier's Ball Security, a wet ball and the Strip tendency (`game/sim.rs`, `hit_effects`).
- What does fatigue cost? Each rating loses its fatigue share × tiredness (speed 10%, strength 12%, skills 6%), top speed 8%, burst and cut 10% when fully spent (`game/sim.rs`, `Agent::new`).
- Is there momentum? Yes: -3 to +2 rating points on all his ratings, after turnovers, drops, sacks and touchdowns, softened by Composure (`game/flow.rs`, `edge_hit`, `decay_edges`).
- What does playing hurt cost? A flat 3 points off every rating (`game/flow.rs`, `run_scrimmage_call`). `InjuryKind::hampers` lists per-injury costs but nothing calls it (`game/sim.rs`, `hampers`).
- How are Discipline, Composure, Concentration, Motor, Toughness read? On a 1-20 scale against the level: 10 + (rating - norm) / 2.6; norms NFL 67, FBS 48, FCS 42, small college 37, high school 29, plus Toughness +3, Motor +1.5, Composure +1, Concentration +4.5, Competitiveness +5.5, Physicality -0.5 (`game/rules.rs`, `trait_norm`; `game/roster.rs`, `trait20`).
- Are field goals tracked? No: a roll of the kick angle against the posts and the leg against the distance (`game/kicking.rs`, `field_goal`). Kick and punt returns are tracked plays (`run_return`).
- How do tendencies act? Each check reads lean = (tendency - 10.5) / 9.5, from -1 to +1; a followed instruction sets the tendency, stress slides it up to a third back toward his old habit (`game/sim.rs`, `tend`, `lean`).
- How much is a Dynamic Link worth? 5 rating points at full strength, only in its own situations; stacked links count the strongest in full, the next at half, a third at a quarter (`links.rs`, `edge`, `FULL`).
- When does a tired starter come out? Below his position's energy line (EDGE/DT 0.84, WR/TE 0.74, LB/CB/S 0.68, RB 0.60, others 0.5), if a fresh backup is within 60 × (line - energy) rating points (`game/flow.rs`, `threshold`, `pick`).
- Does a player's week of practice matter? Through his prep word, for every team: his busts and run misfits × 1.33 (Unprepared) to × 0.93 (Sharp), his read up to 0.033 s later, a quarterback up to about 10% longer on each read. It changes no rating (`game/roster.rs`, `GamePlayer.prep`; `training/prep.rs`, `game_factor`; `game/scrimmage.rs`, `run_snap`, `quarterback`). See [training.md](training.md).
- Is the win chance on screen what the coaches read? No: the screens' chance is its own model (`game/win_chance.rs`, `chance`); play calls still read `live::win_prob`, unchanged. See "The game boards' records and the win chance" below.

## How a snap runs

- `run_snap` builds the agents (offense first), aligns the defense, reads the play, rolls busts and misfits, gives every man a job, then ticks blocks, the quarterback, the defense, routes, the carrier, tackles and movement until the ball is dead (`game/scrimmage.rs`, `run_snap`).
- His ratings for the snap: rating × (1 - fatigue share × tiredness) + edge, clamped 1-99. The edge is momentum, -3 if he plays hurt, minus the fill-in cost of a spot he doesn't know (`game/sim.rs`, `Agent::new`; `game/flow.rs`, `run_scrimmage_call`, `unfamiliar`).
- Top speed, burst and cut come from his raw ratings and weight; the edge doesn't move them, fatigue does (`game/sim.rs`, `Agent::new`).
- Road noise: every offensive player loses 11 × crowd × focus points of Football IQ, focus = clamp(1.6 - 0.06 × Concentration20, 0.4, 1.6) (`run_snap`, `NOISE_LOAD`).
- Crowd, wherever this page uses it: the home crowd × the road offense's `prep.noise`, 0 at home or on a neutral site. `prep.noise` is 1 for AI teams; your week's crowd-noise days set yours, 0.9-1.15 (`game/flow.rs`, `home_crowd`; `training/seat.rs`, `week_habits`).
- Seeing: a cone of half-angle 1.2 + 0.006 × (rating - 50) radians, read with Pocket Presence for the passer, Ball Carrier Vision for the carrier, Play Recognition for everyone else; anything within 2 yards is felt (`game/sim.rs`, `sees`).
- `pro()`: some quarterback, catch-radius and decision reads count every point from 68 up and a quarter of the gap below 68 (`game/scrimmage.rs`, `pro`).
- Chemistry is 0-1 with 0.45 as typical (`game/roster.rs`, `TYPICAL_CHEM`).
- The score moves habits (`game/flow.rs`, `mood`). late = clamp((quarter length - game left) / (0.67 × quarter length), 0, 1). Gamble: +2.5 × late down 9+, +1.5 × late behind with under a third of a quarter left, -2 × late up 9+, -1.5 × late ahead with under a third left. It adds to every tendency except Patient or downhill, Contact willingness and Gets his hands up (`tendencies.rs`, `Tend::gamble`; `game/sim.rs`, `tend`).
- Stakes = 0.5 in a playoff game + 0.4 × late when within 8 points with under 0.55 of a quarter left; stress = clamp(stakes + 0.8 × tiredness + 0.05 × (10 - Composure20), 0, 1) slides a habit stress / 3 of the way back to his old one (`mood`; `game/sim.rs`, `tend`).

### Reading the play

- Each defender's read time = max(0.12, 0.22 + 0.0075 × (100 - Play Recognition + level gap) + 0.12 × U - 0.14 × lean(Downhill) + fake + film + job + prep + motion + 0.12 × novelty) seconds (`run_snap`).
  - Level gap: the Play Recognition norm at his level minus FBS's (NFL +19, high school -19), so a typical defender reads alike at every level (`run_snap`; `game/rules.rs`, `trait_norm`).
  - Fake (play action or a draw): (0.15 + 0.012 × (100 - PR) × U) × sell × bite × max(1 + 0.9 × lean(Downhill), 0.1); sell = clamp(1 + 0.01 × (QB Ball Handling - 60), 0.6, 1.4); bite = clamp(1.5 - 0.05 × Discipline20, 0.5, 1.5).
  - Film: ±0.16 × keyed, sooner when what the look tells matches the play; keyed = belief lean × (0.5 + 0.5 × film study / 20) × clamp(PR / 80, 0.3, 1.2) (`run_snap`; `game/calling.rs`, `belief`).
  - Job: 0.12 × (0.9 - his knowledge of the front or coverage family). Jet or orbit motion: +0.10 × bite for linebackers.
  - Prep: 0.1 × (his prep factor - 1): +0.033 s Unprepared, +0.02 Underprepared, +0.008 Patchy, -0.0025 Prepared, -0.007 Sharp (`run_snap`; `training/prep.rs`, `game_factor`).
  - Downhill applies to linebackers and safeties only.

### Busts and misfits

- Bust chance = ((1 - know) × 0.05 + (60 - FIQ)₊ × 0.0006 + noise + check + install + lapse) × call × chem × prep (`run_snap`).
  - noise (road offense) = 0.03 × crowd × max(1.3 - FIQ / 100, 0.3); check (defense against trips, bunch, empty or motion) = 0.04 × (1 - know).
  - install = 0.04 × (plan load - absorb)₊, absorb = clamp((FIQ - 30) / 45, 0.2, 1.3) (`game/roster.rs`, `absorb`, `plan`).
  - lapse = 0.03 × (1 - energy) × (10 - Concentration20) / 9.
  - call = clamp(1 - 0.006 × (Communication - 60), 0.75, 1.25) of the QB (offense), the center (the line), the best communicator among linebackers and safeties (defense).
  - chem (WR, TE, RB, CB, S) = 1 - 0.45 × (chemistry - 0.45).
  - prep = his week's prep factor from his latest word: None 1, Unprepared 1.33, Underprepared 1.2, Patchy 1.08, Prepared 0.975, Sharp 0.93 (`game/roster.rs`, `prep_of`; `training/prep.rs`, `game_factor`).
- Run-fit misfit (linebackers and DBs on runs) = clamp((0.035 + 0.002 × max(65 - PR, -15) + 0.06 × (1 - front knowledge) + 0.003 × (10 - Discipline20) + 0.04 misdirection + 0.03 jet + 0.05 × novelty) × prep, 0.01, 0.35). A misfit fills the wrong gap or comes late (`run_snap`, `defend`).

## Bodies and movement

- Forty = 5.56 - 0.0124 × (0.75 Speed + 0.25 Acceleration) + 0.0033 × (weight - 220), the combine fit without noise (`game/roster.rs`, `forty`).
- Top speed = 40 / max(forty - sprint_tau + 0.06, 3.4) yd/s; sprint_tau = clamp(0.78 + 0.003 × (60 - Acc) + 0.0006 × (weight - 220), 0.55, 1.0) (`top_speed`, `sprint_tau`).
- Accel = 4.65 + 0.008 a point of Acceleration above 60 (0.0046 below) - 0.006 × (weight - 220), clamped 2.2-7.5 yd/s² (`accel`).
- Cut = 0.99 + 0.004 a point of Agility above 60 (0.0022 below) - 0.002 × (weight - 220), clamped 0.4-1.7; Sudden adds 0.02 × share (`cut`; `game/sim.rs`, `Agent::new`).
- Steering: speeding up follows the sprint curve, braking is accel × (1.2 + 0.7 × cut), a turn spends accel × cut sideways; Bend helps a tight turn at speed (`game/sim.rs`, `Agent::steer`, `Agent::brake`).
- Bodies push apart below 0.8 yards, scaled by clamp(1 - 0.008 × (Traffic - 60), 0.55, 1.35) (`Agent::traffic`, `advance`).
- Burner: +1.25% top speed × share for the carrier 2+ yards past the line or a returner, a chaser behind the carrier, a route past 9 yards, a man defender trailing deep, the target of a ball in the air past 9 yards (`link_speeds`, `BURNER`).

## Line blocking (run)

What it reads: run blocking, strength, hands, balance, burst and mass against shedding, hands, anchor, strength, recognition and mass. What it does not read: arm length in inches (only the Length link), height, Speed, the blocker's Agility or Football IQ.

- Start: get-off = 0.6 Acceleration + 0.4 Anticipation + 12 × lean(Snap timing) + Get-off + Length (defender); run set = 0.5 Run Blocking + 0.3 Acceleration + 0.2 Strength + Get-off + Length (blocker); w0 = -0.1 + 0.006 × (get-off + silent count - set) + front fit + N(0, 0.18) (`game/sim.rs`, `engage`).
  - Silent count: a road line in its first second gives the defender 24 × crowd × focus, focus = clamp(1 - 0.05 × (Concentration20 - 10), 0.5, 1.5) (`engage`, `SILENT_COUNT`).
  - Front fit (DE/DT only): a two-gap DT (a 3-4 front) +0.05 against gap runs, -0.02 against zone; a one-gap lineman -0.02 against gap runs, +0.03 against zone (`front_fit`). Duo counts as zone here (`game/scrimmage.rs`, `run_snap`).
- Each tick: edge = shed power - block power; w += 0.035 × edge × 0.1 + 0.6 × √0.1 × N(0, 1) (`update_blocks`, `RUN_EDGE`).
  - Block power = 0.45 Run Blocking + 0.2 Strength + 0.15 Hand Use + 0.1 Balance + 0.1 Acceleration + 0.08 × (weight - 300) + Low center/Anchor (`block_base`, `block_gift`).
  - Shed power = 0.4 Block Shedding + 0.2 Hand Use + 0.15 Anchor + 0.1 Strength + 0.15 Play Recognition + 0.08 × (weight - 280) (`shed_power`).
  - Before his read time a run defender drifts 0.12 a second toward losing (`update_blocks`).
  - The runner within 2.2 yards and w > 0: +2 × 0.5 × max(0.5 + w, 0.2) a second, the shed to make the play (`update_blocks`, `SHED_RUNNER`).
  - The runner 1.5+ yards past him: he lets go with chance 0.18 × clamp(1 + 0.04 × (Motor20 - 10) - 0.006 × (blocker Hand Use - 60), 0.5, 1.6) a tick.
- Double teams: edge -= 0.6 × the helper's block power - 20 - 0.3 × (anchor - 60); anchor = 0.6 Anchor + 0.2 Strength + 0.2 Balance + 0.08 × (weight - 300) + Anchor/Big man (`update_blocks`, `anchor`).
  - Combos: 2 on inside zone, zone read, duo and draw; 1 on outside zone, power, counter and iso (`game/scrimmage.rs`, `run_blocking`).
  - The helper climbs to his linebacker when w < -0.15 after 0.35 s, after 1.0 s, or when the linebacker is within 2 yards of the line or 2.5 of him (`run_snap`).
- w ≥ 1 is a shed. w ≤ -1 is a sealed block held at -1, or a pancake with chance 0.05 × finish a tick, finish = clamp(1 + 0.04 × (Physicality20 - 10) + 0.006 × (Strength - 60) - 0.008 × (defender Balance - 60), 0.4, 1.8) (`update_blocks`).
- The pair moves toward the carrier at 1.6 × w yd/s (`update_blocks`).
- Blocks in space (climbs, pulls, screens, returns): a set, square blocker misses with clamp(0.06 + 0.004 × (slip - skill), 0.02, 0.25), any other with clamp(0.17 + 0.006 × (slip - skill) + 0.05 × (speed across - 2)₊, 0.05, 0.55); skill = 0.5 Blocking in Space + 0.5 Agility + Big man; slip = 0.5 Agility + 0.5 Block Shedding + Sudden. A DL moving under 3 yd/s, or anyone under 2, is always hit (`game/sim.rs`, `lands_block`).

## Pass rush and pressure

What it reads: the rush move's ratings against the matching set, mass on the bull rush, Football IQ for the move pick, Motor for the counter. What it does not read: height, arm inches, the rusher's Speed outside the speed rush.

- Start: w0 = -0.42 + 0.006 × (get-off + silent - pass set) + front fit + N(0, 0.18), pass set = 0.5 Pass Blocking + 0.3 Agility + 0.2 Football IQ + Get-off + Length; a two-gap DT starts -0.03 (`engage`, `front_fit`).
- The move: the rusher takes the best of speed, power and finesse by (rush - block) + N(0, max(11 - 0.09 × FIQ, 2)) (`pick_move`).
  - Speed: 0.4 Finesse Rush + 0.2 Speed + 0.15 Acc + 0.15 Flexibility + 0.1 Hand Use + Bend, against 0.5 Pass Blocking + 0.2 Agility + 0.15 Flexibility + 0.15 Hand Use + Big man (`rush_power`, `block_base`, `block_gift`).
  - Power: 0.4 Power Rush + 0.25 Strength + 0.15 Hand Use + 0.1 Balance + 0.1 Block Shedding + 0.08 × (weight - 280) + Anchor/Battering ram, against 0.4 PB + 0.3 Anchor + 0.15 Strength + 0.15 Balance + 0.08 × (weight - 300) + Anchor.
  - Finesse: 0.45 Finesse Rush + 0.25 Agility + 0.2 Hand Use + 0.1 Block Shedding + Big man/Sudden, against 0.45 PB + 0.3 Hand Use + 0.15 Agility + 0.1 Football IQ + Big man/Length.
  - An edge on contain adds 10 × lean(Upfield); every rusher loses 2.5 × lean(Hands up) (`rush_power`).
- Each tick w += 0.032 × edge × 0.1 + 0.58 × √0.1 × N(0, 1) (`update_blocks`).
- Counter: once, when w < -0.2 after 0.8 s, with chance clamp(0.45 + 0.008 × (FIQ - 60) + 0.03 × (Motor20 - 10), 0.15, 0.85); kept only if it beats the move he is on (`update_blocks`).
- A shed inside 2.5 s, with the QB still holding the ball, is a rush win. The beaten blocker can't square him again for clamp(0.8 + 0.15 if a speed rush - 0.010 × (Agility - 60) - 0.006 × (PB - 60), 0.35, 1.3) s and until he is back in front (`update_blocks`, `can_square`).
- The pocket: an engaged pair moves toward the QB at (0.25 + 0.65 × (w + 1) / 2) yd/s, × 1.4 on a bull rush, × 0.7 otherwise (`update_blocks`).
- Protection: each lineman takes the nearest rusher within 3.2 yards across; backs and tight ends take the rest (`game/scrimmage.rs`, `pass_protection`).
  - Help on the plan's rusher: a spare blocker slides to him; with none, a back or tight end within 5 yards across chips him, 0.6 s into his route, taking 0.25 × clamp(0.6 + PB / 150, 0.6, 1.3) off w and halving his speed (`slide_to`, `run_route`, `CHIP_TIME`, `CHIP_W`).
  - A stunt is passed off with clamp(0.35 + 0.45 × line chemistry + 0.004 × (the pair's FIQ - 60), 0.2, 0.92); the looper waits 0.45 s (`stunt_pickup`, `assign_defense`).
  - A back picks up a blitzer with clamp(0.85 + 0.01 × (FIQ - 60) + 0.004 × (PB - 60) - 0.008 × (blitzer's Blitzing - 60), 0.4, 0.98) (`picks_up`).
- Pressure on the QB: each free rusher he sees (or feels, chance 0.15 + 0.005 × Pocket Presence) within 4.5 yards adds (4.5 - d) / 4.5; an engaged rusher winning (w > 0.3) within 3 yards adds 0.6 × winning × (3 - d) / 3. Pressured is logged when the nearest gets inside 2.6 yards (`quarterback`).
- Batted balls: each rusher in the lane 1-8 yards ahead bats with 0.17 × (1 - offset) × reach × passer × arc × hands (`batter`, `BAT`).
  - reach = clamp(1 + 0.015 (Jumping - 60) + 0.08 (arm - 33.5) + 0.06 (height - 75) + 0.01 (Anticipation - 60) + 0.015 Length, 0.3, 2); passer = clamp(1 - 0.06 (QB height - 75) - 0.008 (Quick Release + Pocket frame - 60), 0.4, 1.8); arc = clamp(1.3 - distance / 30, 0.3, 1.2); hands = clamp(1 + 1.1 × lean(Hands up), 0.15, 2.4).

## Sacks

What it reads: who gets there (the blocks above), the QB's elusiveness, agility and strength against the rusher's tackling. What it does not read: either man's weight, the QB's Speed, the rush ratings.

- A rusher tries when he is free within 1.25 yards, or winning his block (w > 0.45) within 1.2, and hasn't tried in 0.6 s (`game/scrimmage.rs`, `sack_check`).
- Escape chance = sigmoid(-2.4 + (escape - tackle) / 12 - 1 if unseen + 1.2 if the rusher is still engaged) (`sack_check`).
  - escape = 0.4 Elusiveness + 0.3 Agility + 0.3 Strength (+ Pocket frame against an engaged rusher); tackle = 0.6 Tackling + 0.2 Strength + 0.2 Agility + Length.
- Strip sack = (0.06 seen, 0.16 unseen) × (1.3 - Ball Security / 120) × (1.2 from the blind side, 0.8 otherwise); the blind side is his left as a righty, his right as a lefty (`sack_check`).
- A second rusher within 1.1 yards shares the sack. QB injury check at share 0.8 seen, 1.3 unseen, force = rusher mass × speed (`sack_check`).

## The quarterback: reads, decisions, the throw

What it reads: Play Recognition, Decisions, Pocket Presence, Composure, Football IQ, Eye Manipulation, Quick Release, Footwork, Anticipation, Arm Strength, the three accuracies, Touch, Throw on the Move, Speed (scrambles), hidden consistency, chemistry. What it does not read: Concentration, Toughness, height (except batted balls and the Pocket frame link).

- The drop: top of the drop at quick 1.25 s, medium 1.6, deep 2.1, boot 2.2, screen 1.4, bubble 0.75, +0.35 with play action, + clamp(-0.0015 × (Footwork - 60), -0.06, 0.08) (`game/scrimmage.rs`, `run_snap`).
  - A bad shotgun snap costs 0.3-0.6 s, chance clamp(0.004 - 0.0001 × (center's Shotgun Snap - 60), 0.001, 0.012); a fumbled snap is not modelled (`run_snap`).
- First read: the concept's order, except the best receiver is fed first with chance min(0.1 + 0.02 × his rating edge, 0.55), or the plan's target corner's man 50% of the time (`run_snap`, `FEED_BASE`, `FEED_MAX`).
- Time on each read = clamp(0.62 - 0.0045 × (iq - 50), 0.3, 0.95) × shown × (1 + 0.6 × lean(Hold)) × prep; iq = 0.5 PR + 0.5 Decisions; shown = (1 - 0.2 × (pre-snap read - 0.45)) × 1.15 if a disguise fooled him; prep = 1 + 0.3 × (his prep factor - 1), 1.099 Unprepared to 0.979 Sharp (`quarterback`).
- His clock = clamp(3.6 - 0.01 × (Decisions - 60) - 0.004 × (Pocket Presence - 60), 3.1, 4.1) + 0.3 on a deep drop + 1.0 × lean(Hold) (`quarterback`).
- Hot read: pressure over 0.6 before 2 s sends him back to read 1 (always above 45 Football IQ, else 40%) (`quarterback`).
- Urgent: the first free rusher he knows of is under 0.85 - 0.004 × (Composure - 60) s away (`quarterback`).
- How open a man looks = window + size room + noise × N(0, 1) (`quarterback`, `window`, `size_window`).
  - window: the gap the nearest defender can't close in the ball's flight (no reaction time for his man defender, 0.3 s for others) and anyone under the throwing lane, capped at 8; ball speed 16.5 + 0.13 × Arm Strength yd/s; flight 0.28 + distance / (speed × (1 - 0.005 × (distance - 20)₊)); a spot past 25 + 0.45 × Arm is never open (`window`).
  - size room: on downfield and red-zone throws, 0.07 yd an inch of high reach past 2 (max 1.2) × sense × hands; sense = clamp(0.6 + 0.01 × (0.5 FIQ + 0.5 Dec - 50), 0.4, 1.2), hands = clamp(0.6 + Contested Catch / 150, 0.6, 1.3) (`size_window`, `high_reach`).
  - noise = max(1.6 - 0.017 × pro(iq + Pocket frame), 0.2) × (1 + 0.6 × panic) × (1 - 0.15 × (pre-read - 0.45)) + 0.2 if fooled; panic = pressure × clamp(1.6 - 1.4 × Composure / 100, 0.15, 1.5) (`quarterback`).
- He throws when it looks above need (`quarterback`):
  - normal = 0.3 + 0.008 × (Decisions - 60) - 0.18 × seconds since set - 0.44 × panic + 0.1 on read 1; quick game reads the first two routes as one picture.
  - late or out of reads = 1.35 - 0.2 × seconds since set - 0.44 × panic; urgent = 0.3 + caution - 0.44 × panic - 0.2.
  - need falls 0.05 a yard past 12 yards deep (max 1.2) and by lean(Gunslinger) (+0.4 × lean deeper than 12); before he is late, a holder wants 0.5 × lean(Hold) more, 0.9 more under 10 yards; a gunslinger prefers the deeper of two open men.
- Checkdown: past clock + 0.7 × min(lean(Gunslinger), 0) with nobody open he takes a man within 7 yards with window ≥ 0.3; past clock + 0.5 - 0.4 × max(lean, 0) he forces it to anyone with window ≥ -1.2 (`quarterback`).
- Release: clamp(0.34 - 0.00125 × (0.75 Quick Release + 0.25 Footwork - 60), 0.26, 0.42) s, 0.6× on screens. A free rusher who will reach him within it stops the throw (`quarterback`).
  - He lets it go clamp(0.25 - 0.0025 × (Anticipation - 50) + 0.12 × (0.45 - chemistry), 0, 0.6) s after he decides; heavy pressure skips the wait.
- Scramble: with nobody open or pressure > 0.5, a lane and 2+ s, chance clamp(0.03 + 0.004 × (pro(Speed) - 50), 0.01, 0.2) × max(1 + 1.2 × lean(Scramble), 0.1); with pressure > 1 and pro(Speed) > 75, 0.25 × that lean factor. An edge rushing upfield can run past him; a contain edge shuts his side from 3 + 0.8 × (-lean)₊ yards (`quarterback`).
- Throwaway: 0.3 × (0.06 + 0.003 × pro(Decisions) + 0.12 outside the pocket); inside the pocket it is intentional grounding with 0.08 × clamp(1 - 0.02 × (0.5 Dec + 0.5 FIQ - 60), 0.4, 1.8) (`quarterback`).
- Accuracy: Short to 5 air yards, Medium at 15, Deep from 25, blended between; on a medium throw over a defender, Touch takes up to 0.3 of it (`pass_accuracy`, `throw`).
- Placement: the aim moves 0.45 × accuracy / 100 yards away from the nearest defender and stays 1.5 yards inside the sideline (`throw`).
- Miss sd = (0.25 + 0.036 × distance + 0.0007 × downfield²) × 1.3 × e^(0.005 × (110 - pro(accuracy))) yards (`aim_sd`), then × (`throw`):
  - pressure over 0.3: 1 + 0.9 × min(pressure, 1.5) × (1.35 - Composure / 100); moving over 3 yd/s: 1.25 - 0.002 × Throw on the Move.
  - wind 1 + wind × (0.004 + 0.0004 × distance); rain or snow 1.1; cold 1 + 0.08 × unused-to-cold.
  - hidden consistency 1.15 - 0.015 × consistency; chemistry 1 - 0.12 × (chemistry - 0.45).
  - along the line × (1 + 0.022 × (downfield - 12)₊), and past 25 yards × 1.1 × clamp(1 + 0.01 × (75 - Arm), 0.8, 1.4).
- Roughing the passer: a free rusher closing who lands more than 0.4 s after the release draws it at 0.1, sooner at 0.015, × (1.5 - Discipline20 / 20) × crew (`throw`).

## Routes against coverage (separation)

What it reads: Route Running, Release, Agility, Strength against Man/Zone Coverage, Press, Play Recognition, Anticipation, Agility, Flexibility. What it does not read: the receiver's height or weight (until the catch point), Speed beyond his own top speed.

- Routes run at 0.88 × top speed; he moves to the next point 0.6 + 0.012 × RR yards early; a break slows him to top × (1 - (0.62 - 0.004 × (RR - 50)) × min(turn / 1.57, 1.4)) (`game/scrimmage.rs`, `route_pace`, `route_step`).
- He settles in zone holes with room 1.5 × clamp(1 + 0.015 × (0.6 RR + 0.4 FIQ - 60), 0.6, 1.4) yards (`run_route`).
- Press: cushion 1.2 yards in Cover 0, 1 and 2-man, 5 in Cover 2 and Tampa 2, 6.5 otherwise (`align_defense`). Within 2 yards it is a jam: x = (0.55 Release + 0.25 Agility + 0.2 Strength + Sudden/Length) - (0.55 Press + 0.25 Strength + 0.2 Agility + Length/Sudden) + N(0, 9); below -9 his release is 0.45 s late, above 9 the defender's lag grows 0.3 s (`assign_defense`).
- Man: the defender trails his man's path by lag = clamp(0.12 + 0.0065 × (RR - Man Coverage) + 0.003 × (Agility - (0.6 Agility + 0.4 Flexibility of the defender)), 0.1, 0.75) s (`man_job`).
  - Bunch or stack: the defenders switch with clamp(0.25 + 0.01 × (mean CB PR - 60), 0.05, 0.75); if not, +0.15-0.25 s. Motion across adds 0.1 s (`assign_defense`).
- Zone: a defender plays where the route was clamp(0.3 - 0.004 × (0.6 Zone Coverage + 0.4 PR - 60), 0.1, 0.6) s ago, leans clamp(0.004 × (Anticipation - 40), 0, 0.3) toward the QB's eyes; a deep man looked off is pulled 0.3 × look-off × bite × the gap, bite = clamp(1 - 0.01 × (Antic - 60), 0.3, 1.5); look-off = clamp((Eye Manipulation - 60) / 30, 0, 1) on read 1's first 0.6 s (`cover_inner`, `quarterback`).
- Jumping the route: once the QB has committed to a man within 4.5 yards, if he may freelance and with chance clamp(0.07 × (1 + 4 × lean(Jump routes)), 0, 0.9); right with clamp(0.35 + 0.012 × (Antic - 60) + 0.006 × (PR - 60) - 0.25 × the plan's double moves, 0.1, 0.9). Wrong, his man turns vertical and he needs clamp(0.9 - 0.008 × (Antic - 40), 0.5, 0.9) s to turn back (`jump_route`, `bitten`).
  - Freelancing is allowed with clamp(0.25 + 0.008 × (0.5 FIQ + 0.5 PR - 50) + 0.5 × (trust - 0.5) - 0.025 × (strictness - 10), 0.05, 0.95) (`freelance_ok`).
- Reaction to the throw: a defender who sees the QB reacts after 0.16 + 0.004 × (100 - PR) + 0.1 × U; one who doesn't, at the arrival - 0.45 s (`throw`).
- The receiver reacts after 0.1 s (facing the QB) or 0.25 + 0.003 × (100 - Ball Tracking), or later on a long ball: (flight - 0.6)₊ × clamp(0.55 - 0.002 × (0.5 BT + 0.5 Catching - 60), 0.4, 0.7) (`throw`).
- Scramble drill: after 4.6 s or a scramble, receivers break off if they may freelance (`run_route`, `freelance_ok`).

## The catch and the contested catch

What it reads: Catching, Body Control, Jumping, height, arm length, Contested Catch, Strength, the Length and Highpointer links, Contact tendency over the middle, weather, the passer's Touch. What it does not read: Concentration, Toughness, Route Running, Speed, weight (except through the jump).

- In the lane: a defender within 1.1 yards of a short, flat ball's path touches it with 0.6 × (1 - gap / 1.1) × (0.5 + coverage / 200) × (1, or 0.35 without eyes on the QB) (`game/scrimmage.rs`, `catch_point`).
- Catch radius = 0.95 + 0.015 × (height - 72) + 0.004 × (pro(Jumping) - 50) + 0.02 × (arm - 32) + 0.0015 × (pro(Body Control) - 60) yards (`catch_point`).
- Off target: past radius + reach (1.3 + 0.4 × clamp(nearest defender - 1.5, 0, 4)), out of bounds, or a jump ball over his high point by 2+ inches (`catch_point`).
- High reach = 0.82 × height + arm + 4 + vert × (1 - 0.35 × pace²) × (0.6 if he can't see the ball) inches; vert = max(20.9 + 0.182 × (max(Jumping, 0.95 × Acceleration) + Highpointer) - 0.035 × (weight - 220), 8) (`high_reach`).
- The defender at the catch point (nearest to the ball, reacted, within his reach 1.05 + 0.012 × (height - 72) + 0.004 × (Jumping - 50) + 0.02 × (arm - 31.5), +0.6 if he jumped the route): size = e^(-0.5 × va) × (0.3 if the ball is over him), va = clamp((receiver high reach - his) / 10, ±1.5) × clamp((ball height - 72) / 30, 0, 1) (`catch_point`).
  - Breakup = clamp(closeness × (0.42 + 0.004 × (coverage - 60) + 0.003 × (Contested Catch + Highpointer - 60) + 0.003 × (Jumping - 60)) × size, 0, 0.75); 6% of breakups are picks.
- The catch = 1 - max(0.15 - 0.00095 × Catching, 0.012), then (`catch_point`):
  - away from his frame (past 0.9 of the radius): × (1 - 0.6 × clamp(1 - 0.005 × (Body Control + Length - 60), 0.75, 1.25) × clamp((reach share - 0.9) / 1.6, 0, 1)).
  - contested (defender within 1.4 yards of the ball): × (1 - 0.32 × (1 - d / 1.4) × (1.25 - fight / 100)), fight = 0.5 × (Contested Catch + Highpointer) + 0.25 Strength + 0.25 Jumping + 18 × va; over the middle a contact-shy man also loses 0.12 × (1 - d / 1.4) × (-lean(Contact))₊.
  - rain or snow × 0.97; cold × (1 - 0.03 × unused); a hard ball (over 24 yd/s, within a yard) × (1 - 0.015 × clamp(1 - 0.02 × (Touch - 60), 0.4, 1.6)).
- At the sideline he gets his feet down with clamp(0.35 + 0.006 × (0.6 Body Control + 0.4 FIQ) - 0.15 × (feet needed - 1), 0.1, 0.95); the NFL needs 2 (`run_snap`; `game/rules.rs`, `for_level`).
- Pass interference (air 4+ yards, a contest within 1.4 yards or a breakup): base 0.8 trailing and not facing the ball, 0.33 trailing, 0.075 otherwise, × (1.35 - Discipline20 / 20) × crew × (1.2 on a breakup, 0.7 otherwise) × (0.6 + air / 40); offensive PI 0.029 × (1.3 - receiver Discipline20 / 20) × crew (`pi_check`).

## Drops

- A drop is a missed catch with the ball inside 0.8 of his radius and no defender within a yard (`game/scrimmage.rs`, `catch_point`). It reads the same Catching base, so a 99 still misses 5.6% of such balls and a 60 9.3%.
- A drop costs him 1 point of momentum (`game/flow.rs`, `resolve`, `edge_hit`).

## Interceptions

What it reads: the defender's hands (Catching, raised half the gap from his level's norm to the NFL's), coverage, Jumping, height, arms, whether he sees the ball or jumped the route. What it does not read: the defender's Anticipation outside the route jump, the QB's Decisions after the throw.

- Hands = Catching + 0.5 × (Catching norm at the NFL - at his level) (`game/scrimmage.rs`, `ball_hands`, `HANDS_LEVEL`).
- A lane touch is a pick with 0.3 × hands / 70; an off-target ball within 0.8 × (1 + 0.01 × (Jumping - 50)) of a defender with 0.18 × hands / 70 (`catch_point`).
- In front at the catch point: clamp(0.22 × closeness × (0.45 + hands / 110) × (0.45 if he doesn't see the ball) × (3.5 if he jumped the route) × size, 0, 0.7) (`catch_point`).
- Off the receiver's hands to a defender within 3 yards: 0.13 × (1 - d / 3) × (0.5 + hands / 100) (`catch_point`).
- The thrower loses 1 point of momentum, the interceptor gains 1 (`game/flow.rs`, `resolve`).

## Run lanes and the ball carrier

What it reads: Ball Carrier Vision, his speed and cut, the Patience, Bounce and Contact tendencies, the blocks. What it does not read: weight, Strength or Elusiveness while he picks his path.

- Handoff timing: draw 1.5 s, toss 0.45, power 1.35 under center / 1.2, counter 1.55 / 1.4, others 1.2 / 1.0, sneak at once (`game/scrimmage.rs`, `run_snap`).
- A missed mesh: when the back isn't within 1.3 yards of the mesh 0.4 s past the handoff time (a back split wide, as in an empty set checked to a run), the quarterback keeps it and runs, and the back blocks (`game/scrimmage.rs`, `run_snap`, the `Job::Handoff` arm; proposed, not confirmed).
- Bad exchange 0.0025 × (1.6 - (0.65 back's Ball Security + 0.35 QB Ball Handling) / 100); a pitch on the ground 0.012 × (1.6 - Catching / 100) (`run_snap`).
- Press: he presses the line at the play's pace (inside and outside zone, zone read 4.1 yd/s; power, counter, buck sweep 4.7; duo, trap, iso 4.55; QB power 7.0; draw 3.9; others full speed) × (1 - 0.25 × lean(Patience)), and commits 0.5 + 1.4 × min(lean, 0) yards from the line, sooner with a free defender within 3 (`game/sim.rs`, `run_carrier`; `game/playbook.rs`, `press`).
- The hole read under 2.5 yards from the line: the called hole (-0.9), the cutback and the bounce (-0.6 × lean(Bounce)), each costed by who gets there first, with read noise 0.8 × max(1 - 0.008 × Vision, 0.2) (`run_carrier`).
- Each tick he scores 15 headings: 1.5 × gain - 1.1 × threat - sideline + keep + bounce lean - off his lane + noise; threat sums e^((his time - defender's time) / 0.35) (max 6 each) over defenders within sight 7 + 0.12 × Vision (+6), engaged ones weighted 0.15-0.5 (`run_carrier`).
- Near the sideline (within 10 yards, threat > 0.3) he steps out by (0.5 - 0.6 × lean(Contact))₊ (`run_carrier`).
- He runs at 0.95 × top speed with the ball (`run_carrier`).
- Reads: zone read right with clamp(0.55 + 0.005 × Decisions, 0.5, 0.97); RPO pull right with clamp(0.6 + 0.0045 × (0.6 Dec + 0.4 PR), 0.6, 0.97); option pitch right with clamp(0.6 + 0.004 × the same iq, 0.6, 0.96) once the key is within 3.2 yards (`read_option`, `rpo_pull`, `option_pitch`).
- A scrambling QB slides when a free defender is within 2.2 + max((9 - Contact) / 8, 0) yards, if past the sticks or Contact under 12 (`slide`). In the hurry-up the carrier heads for the sideline with a defender within 4 (`hurry_run`).

## YAC

- YAC = the spot minus where he caught it; yards before contact = first contact minus the line (`game/scrimmage.rs`, `run_snap`).
- After the catch he keeps his speed from the route and runs with the same path choice and tackling checks as any carrier (`run_snap`; `game/sim.rs`, `run_carrier`, `tackles`). There is no separate YAC roll.

## Pursuit, elusiveness, broken tackles and tackling

- Pursuit angle: iq = 0.5 PR + 0.5 Pursuit; he leads the intercept by clamp(0.92 + 0.004 × (iq - 60), 0.75, 1.05), the last man in front by at least 1.1 + 0.003 × (iq - 50) (`game/sim.rs`, `pursue_cap`).
  - In front within 5 yards he breaks down: forward clamp(1.5 + 0.04 × (feet - 50), 0.8, 3.5) yd/s, feet = 0.5 Agility + 0.3 Tackling + 0.2 Pursuit; facing within 4 yards, speed × clamp(0.55 + 0.004 × feet, 0.6, 0.95).
  - Motor20 under 9, 4+ yards behind and 8+ away: effort 0.8 + 0.02 × Motor20.
- Who gets a try: reach 1.6 yards, or 2.5 for a free man in space more than 3 yards past the line (+ clamp(0.01 × (Tackling - 60), ±0.4)), + 0.03 × (arm - 32) + 0.05 × Length share; one try each 0.7 s (`tackles`, `OPEN_REACH`).
  - An engaged defender grabs only with w ≥ -0.2, chance 0.25 + 0.3 × w, inside reach - 0.55: an arm tackle.
  - A free defender with Contact lean below -0.15 facing a carrier 15+ lb heavier moving over 4 yd/s passes with 0.35 × shyness.
- The miss (first hit), what it reads: evade = 0.5 Elusiveness + 0.3 Agility + 0.2 Speed + Sudden; tackle = 0.6 Tackling + 0.2 Agility + 0.2 Pursuit + Sudden; angle, closing speed, the carrier's speed, arm-only, lunge; Big hit and Strip. What it does not read: either man's weight, Strength, Balance, height, Ball Carrier Vision (`first_hit`).
  - p_miss = min(style × sigmoid(-3 + (evade - tackle) / 50 + angle + fast + space + arm + dive), 0.95).
  - angle -0.3 head-on, +0.3 from behind; fast +0.25 closing over 9 yd/s; space +0.25 carrier over 5 yd/s, else -0.2; arm-only +1.5; dive 2 × yards lunged; style = max(1 + 0.7 × lean(Big hit) + 0.25 × lean(Strip), 0.2).
  - After a miss he keeps 0.72 of his speed (side or behind) or 0.88 (head-on), ± clamp(0.002 × (Balance + Low center - 60), ±0.08).
  - An arm tackle he runs through with 0.6 × clamp(1 + 0.008 × (Balance + Low center (+ Battering ram over 4 yd/s) - 60), 0.75, 1.25) × clamp(1 - 0.025 × (tackler Physicality20 - 10), 0.8, 1.2), max 0.9, if moving over 3 yd/s, keeping 0.6 of his speed.
- Contact: speeds mix by mass × velocity; force = (m × v of both) × 0.5 × (1 + 0.2 × lean(Big hit)), then the fumble and injury checks; a Battering ram carrier adds 2 × share work to the tackler (`hit_effects`).
- The wrap-up (each tick), what it reads: the carrier's Strength, Leg Drive, Balance, Elusiveness, Competitiveness20, Contact, mass; each tackler's Tackling, Strength, Competitiveness20, Strip, mass (`fight_tick`).
  - resist = 1 + 0.003 × (Strength - 60) + 0.005 × (Leg Drive - 60) + 0.003 × (Balance - 60) + 0.002 × (Elusiveness - 60) + 0.006 × (Comp20 - 10) + 0.15 × lean(Contact) + (mass - 215) / 300 + 0.003 × (Low center + Battering ram).
  - each tackler f = 1 + 0.03 × (Tackling + Battering ram - 60) + 0.012 × (Strength - 60) + 0.01 × (Comp20 - 10) - 0.12 × lean(Strip) + (mass - 215) / 300.
  - Down with chance 1 - e^(-Σ 3.1 × max(f, 0.3) / max(resist, 0.4) × 0.1) a tick.
- Broken tackle: one tackler only, after 0.15 s, 0.026 × max(1 + (rb - tk) / 60, 0.05) a tick; rb = 0.5 Strength + 0.5 Elusiveness + 0.08 × (mass - 215) + Battering ram/Low center; tk = 0.6 Tackling + 0.4 Strength + 0.08 × (mass - 215) + Battering ram/Length (`fight_tick`).
- The pile moves at (m × (2.2 + 0.03 × (Strength - 60)) - 0.35 × hold) / (m + 0.6 × hold), hold = Σ tackler mass × f; a runner in a pile adds 30 work a second (`fight_tick`).
- Down: he falls up to 1.5 yards with the pile's momentum plus up to 0.9 yards of body forward (`tackled`).
- Pushed out: within 6 yards of the sideline, chance min((6 - distance) / 4.5, 1) × (0.5 + 0.5 × the pile's push outward) (`tackled`).
- Reaching for the line: within 0.05-1.6 yards of the line to gain or the goal line, with chance clamp(0.25 + 0.55 × lean(Reach), 0, 0.97); it gets there if inside 0.75 + 0.03 × (arm - 32) yards (`reach`).

## Fumbles

What it reads: hit force (both masses and speeds), the hitter's Strength and Tackling, the carrier's Ball Security, rain, snow and cold, the Strip tendency. What it does not read: the carrier's Strength or Toughness.

- In a hit: 0.018 × (force / 1700)^1.5 × clamp(1 + 0.025 × (0.5 Str + 0.5 Tackling - Ball Security), 0.25, 2.5) × (1.35 in rain or snow) × (1 + 0.3 × unused-to-cold) × max(1 + 1.15 × lean(Strip), 0.1) (`game/sim.rs`, `hit_effects`).
- Reaching the ball out: 0.035 × clamp(1 + 0.025 × (hit - Ball Security), 0.3, 2.5) (`reach`).
- Strip sacks, bad handoffs and pitches: see Sacks and Run lanes. Punt muffs: see Kicking.
- The ball bounces up to 3 yards each way; each player's claim = e^(-distance / 1.8) × (0.6 + (Anticipation + Competitiveness) / 400) × (1 + 0.25 × lean(Contact)), summed by side (`fumble`).

## Penalties

Each crew draws four multipliers, N(1, 0.15) clamped 0.6-1.5: holding, pass interference, pre-snap, personal fouls (`game/sim.rs`, `Crew::draw`). Yards: holding, illegal block and OPI 10; defensive holding 5 + first down; PI 15 + first down (NFL a spot foul, others spot capped at 15); roughing, unnecessary roughness, face mask 15 + first down; pre-snap fouls and ineligible downfield 5; grounding 10; unsportsmanlike 15 (`FoulKind::yards`; `game/flow.rs`, `resolve`). The offended side takes the better of the play and the penalty by expected points (`resolve`).

- Offensive holding: one check when a block first passes w 0.75: 0.044 × (1.7 - Discipline20 / 14) × clamp(1 + 0.012 × beat, 0.5, 1.6), beat = 0.5 × (rush or shed + defender Hand Use) - 0.5 × (blocking + blocker Hand Use); a hold resets w to 0.35 and is called at 0.42 × crew (`update_blocks`).
- Defensive holding: a man defender within 0.9 yards, 0.8-2.6 s, 0.0105 × crew × (1.6 - Discipline20 / 14) a tick before the throw (`game/scrimmage.rs`, `cover_inner`).
- PI and OPI: see the catch (`pi_check`). Roughing the passer: see the throw.
- Face mask on a tackle 0.0087 × crew × max(1.6 - Discipline20 / 12, 0.2) × 0.6; else unnecessary roughness 0.023 × crew × max(1.6 - hidden temperament / 12, 0.2) × 0.6 × max(1 + 0.6 × lean(Big hit), 0.2) (`game/sim.rs`, `tackled`).
- Illegal block in the back on returns and escorts: a block from behind, 0.075 × crew × clamp(1 - 0.05 × (Discipline20 - 10) - 0.006 × (Blocking in Space - 60), 0.4, 1.6) (`game/scrimmage.rs`, `escort`).
- Ineligible downfield, only on an RPO thrown past the line: a lineman past 1 yard (NFL), 2 (HS) or 3 (college), 0.6 × clamp(1 - 0.015 × (FIQ - 60), 0.4, 1.4) (`run_snap`; `game/flow.rs`, `run_scrimmage_call`).
- Pre-snap, before each snap (`game/flow.rs`, `pre_snap_foul`). Crew here is the crew's pre-snap draw × the fouling side's `prep.rules`: the offense's for false starts, delay of game and illegal formation, the defense's for offside. `prep.rules` is 1 for AI teams; your week's rules clinic sets yours, 0.9-1.08 (`training/seat.rs`, `week_habits`):
  - False start, each lineman and receiver: 0.00162 × max(1.6 - focus / 15, 0.3) × (1 + 0.32 × road crowd × clamp(1.5 - 0.05 × Conc20, 0.5, 1.5)) × crew × (1 + tiredness) × (1.25 - 0.25 × playbook knowledge), focus = 0.6 Disc20 + 0.4 Conc20.
  - Offside, each down lineman: 0.00172 × max(1.6 - Disc20 / 15, 0.3) × clamp(1 + 0.04 × (Anticipation20 - 10), 0.6, 1.5) × hard count × guessing × crew. Hard count on 3rd/4th and 2 or less (more downs when the plan targets snap guessers): clamp(1.5 + 0.05 × (QB FIQ20 - 10), 1, 2.5) × (1 + 0.5 × plan); guessing = max(1 + 1.1 × lean(Snap timing), 0.1) × hard^(0.8 × lean₊) (a followed "no snap guessing" order sets the tendency).
  - Delay of game 0.005 × (1.3 - QB FIQ / 120) × (1 + 0.52 × crowd) × crew; illegal formation 0.0078 × crew × (1.3 - 0.3 × QB knowledge) × clamp(1 - 0.008 × (line and QB FIQ - 60), 0.6, 1.4).
- Celebrations: a scorer's ego sets an unsportsmanlike chance (NFL 0.004 from 2017, 0.012 before; college 0.008, HS 0.006, × showman); in college from 2011 taunting on a 15+ yard score takes the touchdown off (`resolve`).

## Kicking, punting and returns

What it reads: the kicker's Kick Power, Short and Long Kick Accuracy, Kick Trajectory, Get-off, Kickoff Technique, hidden consistency and Composure20; the punter's Punt Power, Punt Placement, Get-off, Composure20 and kicking foot; the snapper's Long Snapping (read as rating / 5); the holder's Catching and Concentration; the returner's Speed, Elusiveness, Decisions, Ball Tracking, Catching and Return risk. What it does not read: the hash for a field goal.

- Field goal accuracy blends Short Kick Accuracy (to 30 yards) into Long (from 50) (`game/kicking.rs`, `fg_accuracy`).
- Field goal (`field_goal`):
  - bad snap 0.004 × (1 + max(10 - LS/5, -5) × 0.18), 60% of them lost; bad hold 0.004 × (1.6 - (0.5 Catching + 0.5 Concentration) / 100).
  - block = (0.008 + 0.0005 × (distance - 35)₊) × clamp(1 - 0.01 × (Trajectory - 60), 0.6, 1.4) × clamp(1 - 0.008 × (Get-off - 60), 0.6, 1.4) × max(1 + (rush - protect) / 30, 0.3) × 3 after a bad snap or hold; rush = 0.3 Power Rush + 0.2 Finesse Rush + 0.3 Strength + 0.2 Jumping, protect = 0.6 Pass Blocking + 0.4 Strength (`kick_unit_strength`).
  - angle sd = (2.4 + 0.014 × (distance - 30)) × max(1.82 - 0.0105 × accuracy, 0.55) × (1.18 - 0.018 × consistency) × pressure × 1.6 (bad snap or hold) × (1 + 0.012 × wind) degrees; good if inside the posts and the leg 40 + 0.3 × Kick Power + N(0, 3), with weather, reaches.
  - pressure factor = 1 + pressure × clamp(1.15 - Composure20 / 15, 0, 1) × 0.9 (`pressure_factor`); the game sets pressure 0.8 when the kick decides it late, 0.4 late and close, 0.1 otherwise, +0.15 in the playoffs, +0.1 when iced (50% when it decides) (`game/flow.rs`, `field_goal`).
- Leg distance = (base + per × power) × (1 + 0.000015 × altitude) × (1 - 0.0018 × (55 - °F)₊) + 0.35 × wind along, × 0.96 in rain or snow (`leg`).
- Kickoff (`kickoff`):
  - leg 49 + 0.23 × Kick Power + N(0, 3); land spread 2.5 × clamp(1 - 0.01 × (Kickoff Technique - 60), 0.6, 1.4); hang clamp(3.4 + 0.014 × (KP - 60) + 0.006 × (KT - 60) + N(0, 0.2), 2.8, 4.8) s; out of bounds 0.006 × (1.4 - KT / 100).
  - NFL 2025 rules: aims into the landing zone; against a dangerous returner (0.5 Speed + 0.5 Elusiveness over 80) kicks deep with clamp(0.2 + 0.012 × danger, 0.08, 0.45). Elsewhere a big leg aims for the touchback.
  - Deep in the end zone the returner kneels by a logistic of where he thinks he'll get, land + 22 + 0.1 × (Speed - 70) + 0.9 × (Return risk - 10.5), against the touchback, width clamp(5.5 - 0.04 × Decisions, 1.5, 4.5); a followed kneel order decides it. College: a fair catch inside the 25 is a touchback 30% of the time.
  - Onside recovered with clamp(0.08 + 0.0025 × (KT - 50) - 0.02 (NFL) or + 0.03, 0.03, 0.25), 11-14 yards.
- Punt (`punt`):
  - bad snap 0.005 × (1 + max(10 - LS/5, -5) × 0.2); operation = clamp(0.75 - 0.003 × (LS - 60), 0.62, 0.95) + clamp(1.3 - 0.004 × (Get-off - 60), 1.12, 1.55) + 0.7 (bad snap) + N(0, 0.08) s; block 0.0028 × (operation / 2.05)^8.
  - leg 17 + 0.215 × Punt Power + 14 + N(0, 4.5); from inside 62 yards of the goal he pins, aiming 8 + N(0, 6 - 0.04 × Placement) from the goal line toward the numbers; hang clamp(3.5 + 0.018 × (PP - 60) + 0.006 × (Placement - 60) + N(0, 0.25) - 0.2 pinning, 2.9, 5.2).
  - shank or out of bounds 0.03 × (1.3 - Composure20 / 20).
  - coverage gap = clamp(4.6 - 1.6 × (hang - 4.2) - 0.08 × (gunner - jammer) + N(0, 2), 0, 15); gunner = 0.5 Speed + 0.3 Release + 0.2 Strength, jammer = 0.4 Press + 0.3 Strength + 0.3 Speed.
  - inside his 10 the returner fields it with clamp((0.1 + 0.004 × (60 - Decisions)) × max(1 + 1.3 × lean(Return risk), 0.1), 0.02, 0.7), else it rolls N(2, 7).
  - fair catch with clamp(0.95 - 0.004 × (Decisions - 50) - 0.3 × lean, 0.45, 0.99) when coverage is within 5 - 1.5 × lean yards (a followed fair-catch order: 0.99 within 7).
- Muff = 0.006 × (2 in rain or snow) × (1.2 for a left-footed punter) × (1.5 - (0.6 Ball Tracking + 0.4 Catching) / 100) × (1 + 0.02 × wind); twice the tries with coverage within 5 yards (`muff`, `punt`).
- Returns are tracked plays with return blocking and the same tackling (`run_return`). Kick units: return men by 0.3 Speed + 0.4 Blocking in Space + 0.3 Strength, cover men by 0.4 Speed + 0.35 Tackling + 0.25 Block Shedding, starters 10 points down (`game/flow.rs`, `kick_unit`).
- Range: a coach kicks within 38 + 0.3 × Kick Power - 4 yards, 3 more allowed (`game/flow.rs`, `kicker_range`, `scrimmage`); his make estimate uses sd (2.95 + 0.012 × (d - 30)) × max(1.82 - 0.0105 × acc, 0.55) (`fg_make_estimate`).

## Fatigue

- Ratings fall by share × tiredness (tiredness = 1 - energy): Speed, Acceleration, Agility, Balance, Concentration 0.10; Strength 0.12; Flexibility, Jumping, Stamina, Discipline, Communication 0.06; Ball Carrier Vision and the reads (FIQ, PR, Anticipation, Decisions, Composure) 0.08; technical skills 0.06 (`attributes.rs`, `ATTRIBUTE_PROFILES`; `game/sim.rs`, `Agent::new`).
- Top speed × (1 - 0.08 × tiredness), accel × (1 - 0.1 × tiredness), sprint build-up × (1 + 0.1 × tiredness) (`Agent::new`).
- Work in a snap: running 0.035 × (0.2 + 0.8 × speed / 9) a tick; pass block 0.5 a second for the blocker, 1.0 for the rusher; run block 0.9 each; a runner in a pile 30 a second (`Agent::steer`, `update_blocks`, `fight_tick`).
- After the snap: energy -= 0.041 × work × max(1.55 - 0.011 × (Stamina + Lungs), 0.4) × (1 + road wear), floor 0.25 (`game/flow.rs`, `run_scrimmage_call`; `game/roster.rs`, `drain`).
- Recovery each snap: pace × (1 + 0.1 × Lungs share) × (0.012 on the field, 0.05 on the bench), pace = seconds between snaps / 37.5; +0.05 after the 1st and 3rd quarters, +0.35 at halftime, +0.15 for overtime, +0.02 at the two-minute warning, the defense's clock-stopping timeout and your timeouts; the offense's hurry-up timeout gives none (`run_scrimmage_call`, `recover`, `end_of_period`, `start_overtime`, `tick`, `after_snap_clock`, `user_timeout`).
- Kickoff energy is your week's `prep.fresh`, clamped 0.5-1 (1 for AI teams) (`game/flow.rs`, `Side::new`).
- Late in a drive a tired, unfocused player busts more (see Busts).

## Depth and rotation

- The staff orders each position by overall at the position + practice impression; a player from another position in the group is listed only if he beats the others after the fill-in cost; your order goes first where you set one (`game/roster.rs`, `depth_chart`, `staff_depth_chart`).
- Game-day counts: QB 2, RB 3, WR 6, TE 3, OT 3, IOL 4, EDGE 4, DT 4, LB 5, CB 5, S 3, K/P/LS 2 (`wanted`).
- Two-way play: high school lists a player on the other side when he beats its starters by 4; college and the NFL never (99), except a corner or receiver by 3 on the other side (`game/flow.rs`, `GameSetup::new`; `game/roster.rs`, `CROSS_LIST`).
- Subs: a man under his position's energy line sits if a fresh backup is within 60 × (line - energy) points; a strict coach lowers the line 0.006 a point over 10 (`game/flow.rs`, `threshold`, `pick`).
- A no-huddle snap freezes the defense's substitutions (`run_scrimmage_call`).
- Thin rosters: a rested man from the group, then pulled starters, then the best athlete left (`pick`).
- Garbage time benches starters in steps (see play-calling.md).

## Momentum

- -1 for a foul, an interception thrown, a drop or a fumble; -0.6 a missed tackle; +1 for an interception, a sack, a touchdown carry (`game/flow.rs`, `resolve`).
- A loss counts amount × 1.5 × (1.25 - Composure20 / 20), a gain amount × 0.8; the edge stays within -3 and +2 (`edge_hit`).
- Each snap a negative edge shrinks to × max(0.8 - 0.03 × Composure20, 0.2), a positive one × 0.7 (`decay_edges`).

## In-game injuries

- Every hit checks both men: injury_scale (1.0 in games) × 0.022 × share × (force / 1700)^1.6 × (0.55 + proneness / 18) × 1.12 on turf; share 1 + 0.4 × lean(Contact) for the carrier, 0.7 for the tackler, 0.8 or 1.3 for a sacked QB (`game/sim.rs`, `injury_check`, `hit_effects`; `game/scrimmage.rs`, `sack_check`).
- Kinds by weight: bruise 22, ankle sprain 13, knee 10, shoulder 9, concussion 8, hand 6, hamstring 6, high ankle 5, ribs 5, fracture 3, back 3, ACL 2.2 (`injury_check`).
- Weeks out = lo + (hi - lo) × U² × (1.3 - 0.03 × hidden recovery): bruise 0-1, ankle 0-3, high ankle 2-7, hamstring 1-5, knee 1-6, ACL 36-56, shoulder 0-8, concussion 0.5-3, ribs 0-4, hand 0-5, fracture 4-12, back 0-4 (`injury_weeks`).
- He plays on with clamp(0.15 + 0.035 × Toughness20 - 0.12 × weeks, 0, 0.95), never with a concussion, ACL or fracture; a concussion is caught 92% in the NFL, 80% in college, 60% in high school (`injury_check`; `game/flow.rs`, `apply_injuries`).
- Playing on costs 3 points off every rating (`run_scrimmage_call`).
- Code against the design doc: `docs/design/match-engine.md` says a hurt player loses the attributes the injury hits (an ankle cuts speed and cutting); the code takes the flat 3 points and never calls `InjuryKind::hampers`.
- Code against the docs: `docs/world-model.md` (training lane 4's picks) says a poorly prepared player reads about a tenth of a second late per word below prepared and a quarterback's reads run up to 30% slower; the code adds 0.1 × (factor - 1) s (0.033 s Unprepared) and × (1 + 0.3 × (factor - 1)) (about 10%) (`run_snap`, `quarterback`).

## The game boards' records and the win chance

- **The line chart**, on your games only: a snap keeps a line chart when its game keeps its setup or is played live (`game/flow.rs`, `run_scrimmage_call`; `game/scrimmage.rs`, `Snap::chart`). Each block's rep keeps both men, the leverage when it ended (× 50), its length in tenths of a second, the defender's last move (0 speed, 1 power, 2 finesse), pass, shed, double and pancake flags and both men's spots (`game/chart.rs`, `Rep::of`, called from `game/sim.rs`, `Sim::release`). Each pass rusher keeps his gap 0-4 from the line's spots and whether he was a down lineman, picked up at the snap or in the stunt, with the protection (man, slide left or right, max at 7+ blockers) (`game/scrimmage.rs`, `chart_rush`; `game/chart.rs`, `gap_at`). It draws no random numbers, so a game plays the same with it.
- **Timeouts**: every logged snap keeps both sides' timeouts (`game/flow.rs`, `new_record`; `game/stats.rs`, `PlayRecord::timeouts`).
- **Where a throw went**: every pass keeps where across the field it was snapped and where it was aimed, in yards from the left sideline (`game/scrimmage.rs`, `PlayRecord::throw_y`); an aim past a sideline is kept as aimed.
- **The film's throws**: your coming opponents' film keeps each pass's air yards, yards after the catch, where across the field it was snapped and got to (half yards), time to throw, separation, thrown away and dropped (`world/tape.rs`, `Throw::of`).
- **Names in your kept games**: at the year's turn, before the men who left football are dropped from the world, each of your kept games keeps the id, name and position of its men among them, so last season's pages still name them (`world/seat_names.rs`, `keep`, called from `world/mod.rs` beside `storylines::keep_names`).
- **Each level's numbers**: each engine game adds both sides to its level's tally for the season (dropbacks, pressure, sacks, throws by depth, air yards, time to throw, runs, yards before contact, stuffed, 10+), this season and last (`world/charts.rs`, `Tally::add_game`, `Charts::add`).
- **Drives, red zone trips and the clock held** (counts only): after every engine game the play log is cut into drives (each drive ends in a touchdown, a field goal or a miss, a punt, a safety, an interception, a fumble, a return for a score, downs, the half or the game) (`game/drives.rs`, `drives`, called from `game/flow.rs`, `play_game_ctl`, through `GameStats::count_drives`). A drive with a snap at the opponent's 20 or closer is a red zone trip; a trip ending in a touchdown is a red zone TD (`TeamStats::red_zone_trips`, `red_zone_tds`). The game clock between snaps goes to the side with the ball (a kickoff's time to the side that receives it; tries left out; the quarter's last snap gets the rest of the quarter), kept as `TeamStats::clock_possession` beside `time_of_possession`, the ball in play (`clock_possession`). Field goals and punts keep a bad snap (`PlayRecord::bad_snap`). The game day page reads the same drives.
- **The world log**: with a log open, every game at every level is written to disk as it is recorded (the game row, every snap for college, NFL and other-league engine games, situation totals by season), and memory does not grow; it draws nothing, so a world plays the same with it on or off (`world/wlog.rs`, `world/log_games.rs`; the format in `docs/world-log.md`). The college book keeps 19 more of the game line's numbers: field goals by distance band, kickoffs and touchbacks, punt yards, punts inside the 20 and touchbacks, QB hits, hurries, rush wins by speed, power and finesse, and the time to throw (`world/records.rs`, `stats!`, `StatSet::from_line`).
- **The win chance the screens show** is its own model, not the coaches' (`live::win_prob` is unchanged and still what calls read) (`game/win_chance.rs`, `chance`):
  - The drive on the field first: touchdown chance from the spot 0.15 + 0.65 × e^(-yards to go / 28); a field goal try 0.2 + 0.4 / (1 + e^((yards - (range - 8)) / 6)) times the level's make at that distance; both scaled so the drive is worth the down and distance's expected points, read between the table's 5-yard rows (`drive`, `ep_at`).
  - Where the other side starts after a stall: a punt netting 42 (a touchback at the 20) from outside range, the spot of the kick inside it, a mix of the two within 6 yards either side of range (`drive`).
  - A short clock, from the real spot: a touchdown needs one snap per 6 yards; after the first, a snap takes 6 s of play and 16 s more of running clock unless a timeout (or the two-minute warning, easing in over its last 30 s) stops it; the chance kept is 1 - e^(-1.6 × time left / time needed) (`drive`, `in_time`, `warning_stop`).
  - The half's last snap (when a second snap won't fit): a kick from the spot at the level's make there, or a shot at 0.55 × e^(-yards / 14), whichever is worth more (down 3 in the fourth, a tying kick counts half) (`drive`).
  - Late habits blend in with the clock: the leader or a tied side burns it from 7:00 of the fourth to fully at 3:00 (`MILK_FROM`, `MILK_RAMP`); the side behind in the fourth takes the best of three paces (usual, hurry, slow); from 6:00 of the half to 4:00 a side short of time plays for the kick (`KICK_FROM`, `KICK_RAMP`): given no touchdown it kicks once in range, from outside with chance e^(-yards past range / 25), × 0.45 on fourth down (`REACH`, `FOURTH_REACH`) (`chance`, `drive`).
  - The kick's make by distance, falling away as 1 - s² over the last 7 yards to the longest kick, 0 from there (`Params::fg_make`).
  - How long each ending takes: a touchdown drive 1 + yards / 8 snaps, a field goal drive the snaps to range plus two, a stall 4.5 snaps; 27 s a snap usual, 14 s in a hurry (then the short clock's own mean time to score), 36 s less 30 per timeout the other side spends when slow (the side behind in the fourth: the play clock plus 4 s a snap, less the play clock minus 2 per timeout, and a drive as long as one from its probe spot); a side burning the clock runs four downs before it scores (`drive`, `time_to_score`).
  - The rest of the game: the final margin is normal around the lead plus the pregame line × the share of the game left, with a spread of σ × √(time left × (1 + b) / game), b rising from 0 to 1 over the last ten minutes (`rest_sd`, `LATE_SPREAD`); σ 13.5 NFL, 16 FBS, 16.5 FCS and small college, 18 high school. Each ending's time is read at seven lengths, 0.55 to 1.45 of its usual, weighted 1-2-3-4-3-2-1 (`LENGTHS`) (`from_spot`, `Params::for_level`).
  - A spot's split is never read as worth less than one up to 25 yards further back, the clock still from the real spot: the drive's split has small kinks where kicks give way to punts (`chance`).
  - A leader in the fourth quarter whose kneels outlast the clock against the other side's timeouts (and the two-minute warning) has won: 0.999 (`kneel_burn`).
  - Overtime: the NFL's timed period as the end of a game; college and high school possession for possession from the level's spot (`overtime`). The log is followed round by round: a possession ends with its try, its kick or a turnover, or when the other side has the ball, and a side with the ball again starts a round (`Rounds`, `home_series`, `ot_rounds`). An answering side level wins with any score; a try ends the possession, so the other side is read from the spot, and an answering side's try settles the round (XP at the level's try worth, a two at 0.475) (`try_value`).
  - The game day bar: the same model before the next snap, try or kickoff (`home_now`, called from `crates/afm-game/src/lane_gameday.rs`, `live_wp`).
  - The pregame line: stored with each kept game when it is kept (the closing line, the playoff and Super Bowl lines included; 0 at a neutral site without one) (`world/seat.rs`, `seat_line`, `SeatGame::spread`); for an older game the home side's season page's line; else 1.7 points of home field in the NFL and 2.5 elsewhere, none at a neutral site (`crates/afm-game/src/lane_boards/flow.rs`, `Wp::of_game`, `Wp::new`, `home_field`).
  - With no line (high school and the other leagues always): the gap between the sides is a normal prior with sd `gap_sd` (6 NFL, 14 college, 17 high school), updated by the lead beyond the line's share so far against `sigma`; its mean moves the rest of the game's margin and its remaining spread widens it (`Params::without_line`, `from_spot`).

## Where the knobs are

- `game/geom.rs`: `DT` 0.1.
- `game/sim.rs`: `OPEN_REACH` 2.5, `RUN_EDGE` 0.035, `SHED_RUNNER` 0.5, `SILENT_COUNT` 24, `BURNER` 0.0125; block drifts 0.032 and noise 0.58/0.6 in `update_blocks`; the miss logit in `first_hit`; hazard 3.1 and break 0.026 in `fight_tick`; fumble 0.018 in `hit_effects`; injury 0.022 and the kind weights in `injury_check`; `injury_weeks`; `Crew::draw`; `FoulKind::yards`.
- `game/scrimmage.rs`: `NOISE_BUSTS` 0.03, `NOISE_LOAD` 11, `CHIP_TIME` 0.6, `CHIP_W` 0.25, `READ_NEED` 0.3, `READ_START` 0.1, `PRE_READ` 0.45, `FEED_BASE` 0.1, `FEED_PER_POINT` 0.02, `FEED_MAX` 0.55, `BAT` 0.17, `HANDS_LEVEL` 0.5; `aim_sd`; drop times and mesh times in `run_snap`; the prep terms (0.1 on the read in `run_snap`, 0.3 on each read in `quarterback`).
- `training/prep.rs`: `game_factor`. `training/seat.rs`: `week_habits` (crowd noise 0.075 a day, 0.9-1.15; rules clinic 0.9-1.08).
- `game/roster.rs`: `forty`, `top_speed`, `sprint_tau`, `accel`, `cut`, `drain`, `RECOVER_ON_FIELD` 0.012, `RECOVER_ON_BENCH` 0.05, `TYPICAL_CHEM` 0.45, `CROSS_LIST` 3, `INSTALL` 0.75, `wanted`.
- `game/rules.rs`: `trait_norm`, `TRAIT_STEP` 2.6.
- `game/kicking.rs`: the leg bases in `field_goal`, `kickoff`, `punt`; `muff`.
- `game/flow.rs`: `threshold`, `pre_snap_foul`, `edge_hit`, `decay_edges`, `apply_injuries`, the tempo gaps in `after_snap_clock`.
- `game/playbook.rs`: run `press` and `aim`.
- `links.rs`: `FULL` 5. `attributes.rs`: each attribute's `fatigue` share.
- `game/win_chance.rs`: the margin spread per level and field goal makes in `Params::for_level`; the touchdown and kick curves, the punt net 42 and the short clock (6 s a snap, 16 s running) in `drive`; the 25-yard envelope in `chance`.
- Tuning tools: `crates/afm-tools/src/bin/match_check.rs`, `play_trace.rs`.

## Related pages

- [play-calling.md](play-calling.md): who calls what, game plans, garbage time, level rules.
- [ratings.md](ratings.md): the 67 attributes and the 1-20 reads.
- [body.md](body.md): height, weight, arms and the forty.
- [tendencies.md](tendencies.md): the 15 tendencies and instructions.
- [injuries.md](injuries.md): injuries outside the game and recovery.
- [staff-and-coaches.md](staff-and-coaches.md): the coach ratings the engine reads.
- [training.md](training.md): practice impressions, packages, kickoff energy, each player's prep word, crowd-noise days and the rules clinic.
- The long explanation with calibration history: `docs/match-engine.md`.
