# Ratings: how it works today

Checked against main at 7933e49 with update 3 lane 7 on 2026-10-28 and lane 6 on 2026-10-29.

## Quick answers

- What scale are attributes on? 1-99 (`MIN_RATING` 1.0, `MAX_RATING` 99.0) (`attributes.rs`).
- How many attributes? 67 visible in 8 groups, plus 5 hidden game-day/body traits (`attributes.rs`, `Attr::ALL`, `ATTRIBUTE_COUNT`; `player.rs`, `GameTraits`, `Player`).
- Are attributes whole numbers? No. Stored as `f32`, read and written as `f64`; screens round (`attributes.rs`, `Attributes`).
- What is "overall"? The weighted mean of his attributes at his listed position, weights normalised to sum to 1 (`positions.rs`, `overall_for`; `player.rs`, `Player::overall`).
- What is "potential"? The same formula on his hidden ceilings (`player.rs`, `Player::potential`). Never shown; screens show words (`crates/afm-game/src/reads.rs`, `potential_words`).
- Does overall read his body (height, weight)? No. Only attributes (`positions.rs`, `overall_for`). Body enters position fit (`athlete.rs`, `misfit`) and the nose tackle spot (`spots.rs`, `lean_value`).
- What does "89 among seniors" mean? His overall placed against his position, then ranked against every high schooler his age nationally, on an EA-style 1-99 curve (`level_rating.rs`; `world/season.rs`, `attach_level_ratings`; `crates/afm-game/src/lane_core/pages.rs`, `level_rating`).
- Do level ratings feed growth or the engine? No. Label only (`level_rating.rs` module notes).
- Does the NFL get a level rating? No. Pros are their overall, rounded and clamped 1-99 (`level_rating.rs`, `level_rating`); the page shows none for NFL rows (`pages.rs`, `level_rating`).
- What is `trait_norm`? The typical value of a trait-like rating at a level (NFL 67, FBS 48, FCS 42, small college 37, HS 29, plus per-trait offsets). The engine reads discipline, composure and the like against it on a 1-20 scale (`game/rules.rs`, `trait_norm`; `game/roster.rs`, `trait20`).
- How does a 1-99 rating turn into 1-20? Two ways: `trait20` = 10 + (rating - trait_norm) / 2.6, clamped 1-20 (`game/roster.rs`); `on20` = rating / 5, clamped 1-20 (`game/roster.rs`).
- What does the user see for a rating? A range from his staff, never the number: own players 4-8.6 wide by the coach's eye; other teams' physical ratings 4 wider each side; other teams' skills "Not seen" (`crates/afm-game/src/reads.rs`, `attributes`).
- What does the overall range look like? Own player: ±2 to ±4.3; anyone else: ±6 (`reads.rs`, `overall`).
- What do "Elite / Starter level / Backup level" on the hover card mean? Bands against this season's two-deeps at his level and position: elite = 90th percentile of starters, typical starter = median starter, typical backup = median backup (`world/norms.rs`, `marks`; `crates/afm-game/src/lane_ratings.rs`, `band`).
- Are the hidden 1-20 traits shown? No. Consistency, temperament and recovery are never shown (`player.rs`, `GameTraits`).
- Stars? Half-stars 0.5-5 from a rating: (rating - 40) / 5, rounded, halved; 50 = 1 star, 70 = 3, 90 = 5 (`scouting.rs`, `to_stars`).
- What is a high school squad read against on the squad boards? His state and classification with every graduating class together (a varsity's starters), not his own class as the hover card does (`world/norms.rs`, `Pool::Hs` grade 0; `crates/afm-game/src/lane_squadboards.rs`, `team_pool`).
- Does the game remember last season's read of a player? Yes, from update 2's lane 9A: once a season, for every player your staff follows (`world/seat.rs`, `Career::kept_reads`; see "Kept reads" below).

## The 67 visible attributes

All on 1-99. The enum order is the storage order (`attributes.rs`, `Attr`, `Attr::ALL`). Keys are camelCase (`ATTRIBUTE_KEYS`), page names in `ATTRIBUTE_LABELS`. Groups for the player page come from `Attr::category` (`attributes.rs`).

- Physical (8): speed, acceleration, agility, strength (raw strength, what the bench reads), balance, flexibility (bend), jumping, stamina.
- Passing (11): arm strength, short accuracy (0-9 air yards), medium accuracy (10-19), deep accuracy (20+), touch, throwing on the move, release speed (`QuickRelease`), footwork, pocket presence, ball handling (fakes, handoffs), eye manipulation.
- Running (4): ball security, vision (`BallCarrierVision`), elusiveness, leg drive.
- Receiving (6): catching, contested catches, ball tracking, body control, route running, release off the line.
- Line play (8): pass blocking, run blocking, blocking in space, hand use, anchor, power rush, finesse rush, block shedding.
- Defense (8): man coverage, zone coverage, press, disguise, tackling, pursuit, blitzing, moving through traffic.
- Kicking and snapping (10): kick power, short kick accuracy (inside about 40 yards), long kick accuracy (40+), kick trajectory, kickoff technique, punt power, punt placement, get-off time, long snapping, shotgun snapping.
- Mental (12): football IQ, play recognition, anticipation, decisions, composure, concentration, discipline, communication, toughness, motor, competitiveness, physicality.

Each attribute's one-line meaning is its doc comment in `attributes.rs`, `Attr`.

### Growth profile per attribute

Every attribute has an `AttributeProfile` (`attributes.rs`, `ATTRIBUTE_PROFILES`):
- `group`: Physical (the 8 physical), Technical, or Mental. Vision is a Mental profile though it shows under Running.
- `maturity_at_14`: share of the ceiling a 14-year-old has (speed 0.80, strength 0.48, eye manipulation 0.35, competitiveness 0.68).
- `growth_rate`: share of the gap closed in a prime year (speed 0.24, strength 0.20, mental 0.12-0.13, competitiveness 0.06).
- `growth_window`: ages at full rate (speed 14-20, strength 15-24, pocket presence 16-31).
- `coachability` and `experience`: how much coaching and game snaps drive growth (0-1). Physical attributes all have experience 0.1.
- `decline_age` and `decline_rate`: speed 27 at 0.9 a year, strength 31 at 0.6, short kick accuracy 38 at 0.4.
- `injury_sensitive`: true for speed, acceleration, agility, balance, flexibility, jumping (not strength or stamina).
- `fatigue`: share lost on the field when spent (legs 0.10, strength 0.12, technical 0.06, mental 0.02-0.10).

How these drive growth is the development page's job (`development.rs`, `GrowthCtx::rate`, `develop_season`).

## The hidden traits

The code comment names five hidden: consistency, injury proneness, recovery, longevity, temperament (`attributes.rs` module notes).

- Consistency, temperament, recovery: 1-20, drawn normal(10, 3.5), rounded, clamped, from the player's id stream (`player.rs`, `GameTraits::for_id`). Yearly drift: under 26, consistency +0-0.25 and temperament +0-0.3; from 30, recovery -0.1 to -0.4 (`player.rs`, `GameTraits::age_one_year`).
- Injury proneness: 1-20, normal(10, 3.5) at creation (`player.rs`, `generate_prospect`, `trait_1_20`). A major injury adds 1, capped at 20 (`development.rs`, `develop_season`). It scales injury chance by 0.55 + 0.045 × proneness (`development.rs`, `develop_season`).
- Longevity: not 1-20 in code. It is years added to every decline age, normal(0, 1.3) clamped -3 to 3.5 (`player.rs`, `generate_prospect`; used in `development.rs`, `declines`). The design doc lists it as 1-20; the code wins.
- Also hidden but separate: personality on 1-20 (work ethic, coachability, competitiveness, love of the game, each normal(10, 3.5)) and the section 6 traits (ambition, loyalty, greed, temper, ego, leadership, professionalism, adaptability, aura), 1-20 stored as bytes (`player.rs`, `Personality`, `Traits::draw`). Bloom (years his growth window shifts, normal(0, 1.1) clamped -2.5 to 3) and wear also sit on `Player` (`player.rs`).

## Stored vs shown

- Stored: `Attributes` is `[f32; 67]` per player for current ability and the same for hidden ceilings (`attributes.rs`, `Attributes`; `player.rs`, `Player.attributes`, `Player.ceilings`). Default fill is 1.0.
- Every rating write is clamped to 1-99 at the end of a development year (`development.rs`, `develop_season`, the "Tidy up" loop). A rating above its ceiling lifts the ceiling.
- Shown: screens never show a true attribute. `reads::range` centres on truth + lean × half × 0.6, where lean is a fixed -1..1 per reader, player and attribute (so a range doesn't jump day to day), then rounds lo and hi and clamps to 1-99 (`crates/afm-game/src/reads.rs`, `range`, `lean`).
  - Half-width for your own players: 2 + (20 - eye) × 0.12, eye being the reading coach's 1-20 (so ±2 to ±4.28) (`reads.rs`, `attributes`).
  - Other teams: physical attributes at that half-width + 4 (measurables are public, their meaning is read); everything else `NotSeen` (`reads.rs`, `attributes`).
  - Overall: own ±(2 + (20 - eye) × 0.12), others ±6 (`reads.rs`, `overall`).
- Potential: words only (tier, confidence, hints) from `scouting::potential_outlook` (`reads.rs`, `potential_words`). Tiers: under 52 high school only, 52+ college, 65+ NFL roster, 75+ starter, 85+ Pro Bowl, 92+ elite (`scouting.rs`, `PotentialTier::from_rating`; `career.rs`, `NFL_LEVELS`).
- Scout reports (engine side): overall error sd 2.5 × skill × visibility, skill = 1.4 - 0.04 × accuracy (1-20), visibility HS 1.25, small college 1.5, FCS 1.2, FBS 1.0, NFL 0.7; potential range rounded out to 5-point bands (`POTENTIAL_BAND`) (`scouting.rs`, `scout_player`, `level_visibility`; `development.rs`, `POTENTIAL_BAND`).

## Overall and position ratings

- 14 positions: QB, RB, WR, TE, OT, IOL, EDGE, DT, LB, CB, S, K, P, LS (`positions.rs`, `Position`).
- Formula: overall = Σ (w_a / Σw) × attribute_a over the position's weight list. Unlisted attributes count 0 (`positions.rs`, `overall_for`, `DERIVED`). `weight(pos, attr)` gives the normalised weight; `raw_weight` the listed one.
- Biggest raw weights per position (`positions.rs`, `POSITION_PROFILES`):
  - QB: decisions 2.5, arm strength 2.2, short and medium accuracy 2.0, pocket presence 2.0. Speed 0.4.
  - RB: vision 3.0, speed, acceleration and elusiveness 2.5 each.
  - WR: catching 3.0, route running 3.0, speed 2.5.
  - TE: catching 2.2, run blocking 2.0, route running 1.6.
  - OT: pass blocking 3.5, run blocking 2.0. IOL: run blocking 3.2, pass blocking 2.4, strength 2.0.
  - EDGE: finesse rush 2.4, power rush 2.2, acceleration 2.2. DT: block shedding 2.6, strength 2.4, anchor 2.2.
  - LB: tackling 2.4, play recognition 2.4. CB: man coverage 2.6, speed 2.5. S: zone coverage 2.2, play recognition 2.0, speed 2.0.
  - K: kick power 3.5, short and long accuracy 2.4 each, composure 2.2. P: punt power 3.5, placement 2.6, composure 1.6.
  - LS: long snapping 9.0 of a 13.0 total (about 69%).
- Each profile also has `elite_shape` (attributes of an elite player there; unlisted = 45), `aging_offset` (years added to decline: QB +3, K/P/LS +2, RB -2.5; physical attributes take at most 1.5 of it), `injury_risk` (RB 1.4, K/P/LS 0.25) and `share` of HS players (WR 0.15, IOL 0.13, LS 0) (`positions.rs`, `PositionProfile`).
- Overall at another position: the same formula with that position's weights (`game/roster.rs`, `overall_at`; `world/norms.rs`, `Rated::At`).
- Spots inside a position (LT/RT, NT, nickel and so on): position overall + 0.35 × (his lean at this spot - the sibling spots' average) / 2. A nose tackle's lean reads weight: 0.8 × lean + 0.2 × clamp(weight - 240, 20, 99). Fullback = half TE overall, half a lead-block mix (`spots.rs`, `spot_rating`, `lean_value`, `LEAN`). A spot he has never been listed at costs `fill_in` points (center 6, MLB 4, QB/K/P/LS 8, FS↔SS 1.5, DT↔NT 0, else 3) (`spots.rs`, `fill_in`).

## Position fit (where a staff places him)

- Fit at a position = overall there on the staff's perceived ceilings × (1 - misfit) (`athlete.rs`, `fits`).
- Perceived ceilings: true ceiling + bloom illusion (physical only: -3 × bloom × (1 - insight) × clamp((19 - age)/5, 0, 1)) + a sticky misread normal(0, noise × clamp(1 - 0.1 × (age - 14), 0.5, 1)) (`athlete.rs`, `perceived`). It reads ceilings, not current ability.
- Staff eye: coach of skill s (1-20): insight 0.25 + 0.5 × s/20, noise 7 - 5 × s/20. NFL scouts: insight 0.65, noise 2.5 (`athlete.rs`, `Eye::coach`, `Eye::NFL_SCOUTS`).
- Misfit (body against the combine): h = (height - pos mean)/sd; w = max(|weight - pos mean| - 25, 0)/sd; misfit = 0.04 × (|h|-1)₊² + 0.04 × (|w|-1)₊², clamped 0-0.5. Weight gets 25 lb of free slack, height none (`athlete.rs`, `misfit`). It is measured on his draft-prospect frame (`player_misfit`, which takes his `talent_size` shift back off his height and frame weight).
- Placement of a class: best athletes first; score = fit / his best fit + need (+0.08 if 0.5+ open spots, 0 if about full, 0.12 × open if overfilled) + premium (OT and EDGE 0.015, CB 0.01) (`athlete.rs`, `place_class`, `premium`). Long snapper is never placed (`PLACED`).
- Moving later: moves when the best spot beats the current one by `margin` (3 × margin if he has played there before); never to or from K, P, LS (`athlete.rs`, `better_position`, `reconsider`).
- At creation the natural position is the argmax of overall × (1 - misfit) × `FIT_SCALE`, a per-position factor calibrated once on 40,000 athletes so natural positions land in the real HS shares (`athlete.rs`, `best_fit`, `FIT_SCALE`). Details on the body page.

## Level ratings ("89 among seniors")

Pipeline in the world, after each season (`world/season.rs`, `attach_level_ratings`):
1. Group each last season: high schoolers by age (`LevelGroup::HS(age)`), all college players together, NFL none (`level_rating.rs`, `level_group`).
2. Only HS and FBS seasons set a scale (`sets_level_scale`). FCS and small college are rated on the FBS scale after taking 1.0 and 5.0 overall points off (in `attach_level_ratings`; same numbers as `career.rs`, `PIPELINE.competition_discount` [0, 1, 5, 0]).
3. Align positions: each position with 30+ players and a p99-median gap of 0.5+ is mapped so its median and 99th percentile line up with the whole group's: aligned = all_mid + (overall - pos_mid) × (all_p99 - all_mid)/(pos_p99 - pos_mid) (`level_rating.rs`, `PositionAligner::new`, `align`, `quantiles`). The aligner is this season's only.
4. Anchors (aligned overall at each point → rating) (`level_rating.rs`, `anchors`, `season_anchors`):
   - College: 1st percentile → 55, median → 72, No. 700 → 85, No. 100 → 90, No. 8 → 95, No. 2 → 96.5.
   - HS (per age): 1st percentile → 5, median → 50, No. 2000 → 80, No. 100 → 90, No. 8 → 95, No. 2 → 96.5.
   - Ranks scale by the world's class share when the simulated class is smaller (rank × share, rounded, at least 1).
5. Pool the last 5 seasons (`SCALE_SEASONS`): each anchor sits at its average overall over those seasons, so a strong year shows more 90s (`level_rating.rs`, `pooled_level_scale`, `ScaleHistory::push`).
6. Read: linear between anchors; above the No. 2 anchor +0.4 per overall point (`ABOVE_TOP_SLOPE`), so 99 (98.5 before rounding) needs 5 overall points clear of No. 2; below the bottom, the first segment's slope extended; round half up, clamp 1-99 (`level_rating.rs`, `level_rating`, `clamp_rating`).
7. Two values per season: `level_rating` from `overall_start` (what he played at) and `level_rating_end` from `overall` (`player.rs`, `SeasonRecord`).

Shown (update 3, lane 7): the badge reads your staff's read, not the true rating. The centre of the staff's overall read is placed on the same pooled scale as his level group, "FBS level 73" for any college player, "Senior level 89" (Junior, Sophomore, Freshman) for a high schooler by his class, nothing for the NFL; not seen, no badge (`crates/afm-game/src/lane_core/pages.rs`, `level_rating`, `level_read`; `world/season.rs`, `level_from_read`, `level_pool`). It reads the last finished season's scale, so it is filled all season. Hover shows its marks (`level_marks`, `level_rating.rs`, `marks`). High schoolers group by class, not engine age (`level_rating.rs`, `level_group`). The standalone class sim does the same without pooling (`career.rs`, `attach_level_ratings`, `build_level_scale`).

Draft boards use `PositionAligner::for_draft`: K, P and LS are read off their own class ladder (best K at the grade of pick 45% of the slots, P 50%, LS 70%, each next one 25% of the draft lower, capped at the grade at 18%/27%/70%) (`level_rating.rs`, `for_draft`, `Ladder::read`).

## Rating range hover card (RR1, RR4)

- The pool: HS by state classification and graduating class, widened to the state, then the nation (about 1,500 schools sampled) when under 12 starters; college by FBS, FCS, smaller colleges; NFL as one league (`world/norms.rs`, `pool_of`, `marks`, `MIN_STARTERS`, `NATIONAL_SCHOOLS`).
- Two-deep: each team's best at the position by overall there, starters = `game::roster::starters(pos)` (1 QB, 3 WR, 2 CB...), backups the next as many (`world/norms.rs`, `two_deep`).
- Marks: backup = median backup, starter = median starter, elite = 90th percentile of starters (`norms.rs`, `marks`). Cached per season (`lane_ratings.rs`, `marks_cached`).
- Band by the range's middle: Elite (≥ elite), A good starter (≥ halfway starter-elite), Starter level, Between a backup and a starter (≥ backup), Backup level (≥ backup - max(starter - backup, 3)), Below a typical backup. Plus "Ahead of about X-Y% of ..." rounded to 5%, and a straddle note (`lane_ratings.rs`, `band`, `round5`).
- For a physical rating the card adds what it stands for at the combine at his weight (see the body page) (`lane_ratings.rs`, `combine_line`).
- A high school pool with grade 0 keeps every graduating class (`world/norms.rs`, `collect`). The squad boards use it for a high school's players and unit cards, so a sophomore starter is read against the state classification's starters of every grade (`crates/afm-game/src/lane_squadboards.rs`, `team_pool`, `marks`).

## Help hovers (update 3, lane 6)

- An attribute's name (H1): its plain words, what it does on the field, how it grows (its group's growth window, coachability, whether experience grows it, its decline age, whether injuries cut it) and the top four positions by `positions::weight` (from 0.03) (`crates/afm-game/src/lane_help.rs`, `attr_help`, `words`, `grows`, `matters`; commands `help_attr`, `help_attrs`).
- An attribute's value (H2): the range card adds the change since the last kept read (see Kept reads): the key rating's kept range if it was one of the four, else its family's kept middle; the middle's move gives Up clearly (3+), Up a little (1.5+), About the same, Down a little, Down clearly; up to 3 reasons from his last season row's events, your or the staff's focus (your players only), weight change for a physical rating, age against the decline age or growth window, his role and his teacher (`lane_help.rs`, `last_year`, `reasons`; `lane_ratings.rs`, `card`).
- A level rating (H3): the badge from `lane_core::pages::level_read` (your staff's read placed on the scale of his newest season's group, lane 7), the group, the 5-season pool with the scale's marks from `level_rating::marks` (the bottom 1% left out), and how many of last season's rated players in the same group are above the badge's number (`lane_help.rs`, `level_help`). A row counts as last season's when its age + 1 is his age now, as in `attach_level_ratings`. A player your staff hasn't read, or a pro, has no card.
- Another position's read (H4's "who else could play it"): lane 7's best four positions from the same file, else the file's attribute middles weighed for the spot (an unseen rating at the mean of the seen ones), a point wider (`lane_help.rs`, `read_at`).
- Football terms (H7) and the card itself are screen code only (`app/ui/screens/help.js`).

## Kept reads (the career arc)

- What is kept: once a season, your staff's read of each player it follows as the football year turns: the overall range, the four ratings that matter most at his position (attribute index, low, high), the middle of each of his position's eight skill families, his position and his weight. About 30 bytes a player a season (`world/seat.rs`, `KeptRead`; `crates/afm-game/src/lane_squadboards.rs`, `read_now`).
- When: the first day after the football year turns (the day after the Super Bowl), for the season just ended (`lane_squadboards.rs`, `after_days`, called from `lane_core.rs`, `after_days`).
- Who: your roster, everyone you keep notes on, and the top 300 of your scouting board, while they are still in football (`lane_squadboards.rs`, `followed`).
- The read is the same one the screens show that day: your own players through their position coach's eye, anyone else the public read (`lane_squadboards.rs`, `any_read`; `crates/afm-game/src/reads.rs`, `overall`, `attributes`).
- Never made up after the fact: a new career, or a save from before lane 9A, marks the season it starts in and keeps from the next turn (`after_days`, `kept_through` 0). A second read of the same season replaces the first (`world/seat.rs`, `Career::keep_read`).
- Where it shows: the career arc (the Career tab's chart and table), the shape tab's "since the end of" card and the body tab's weight over time (`lane_squadboards/arc.rs`, `shape.rs`, `body.rs`).
- A season row of the record is dated from his birth date: the engine age on a row is his age on 31 August as that season starts (`lane_squadboards/arc.rs`, `record_season`; `calendar/born.rs`).

## trait_norm and the 1-20 reads in the engine

- `trait_norm(level, attr)`: base NFL 67, FBS 48, FCS 42, small college 37, HS 29; offsets toughness +3, motor +1.5, composure +1, concentration +4.5, competitiveness +5.5, physicality -0.5, others 0 (`game/rules.rs`, `trait_norm`). Marked "proposed, not confirmed" in code.
- `TRAIT_STEP` 2.6: rating points per 1-20 step (spread sd about 9 over the old traits' sd 3.5) (`game/rules.rs`).
- `trait20(a, level)` = clamp(10 + (rating - trait_norm)/2.6, 1, 20): 10 is the level's typical player, so an NFL and an FBS line draw about the same flags (`game/roster.rs`, `trait20`, `composure`).
- `on20(a)` = clamp(rating/5, 1, 20), not level-relative (`game/roster.rs`, `on20`).
- Other reads of `trait_norm`: play recognition's level gap against FBS (`game/scrimmage.rs`, near line 461), catching's gap against the NFL (`game/scrimmage.rs`, near line 4358), tendencies' speed norm (+8) and football IQ (`tendencies.rs`).
- Elsewhere discipline is read as rating/5 on 1-20 (`development.rs`, weight-gain chance; `training/seat.rs`, `obey_chance` call).

## Where the knobs are

- `attributes.rs`: `MIN_RATING`, `MAX_RATING`, `ATTRIBUTE_PROFILES`.
- `positions.rs`: `POSITION_PROFILES` (weights, elite shapes, aging offsets, injury risk, shares), `DEFAULT_SHAPE` 45.
- `level_rating.rs`: `anchors`, `ABOVE_TOP_SLOPE` 0.4, `SCALE_SEASONS` 5, the 30-player and 0.5-gap aligner limits, `for_draft` shares.
- `world/season.rs`, `attach_level_ratings`: FCS 1.0 and small college 5.0 discounts.
- `game/rules.rs`: `trait_norm` bases and offsets, `TRAIT_STEP`.
- `crates/afm-game/src/reads.rs`: half-widths (2 + 0.12 per eye point, +4 for others' physicals, ±6 others' overall), lean 0.6.
- `world/norms.rs`: `MIN_STARTERS` 12, `NATIONAL_SCHOOLS` 1500, elite at the 90th percentile.
- `crates/afm-game/src/lane_squadboards.rs`: `followed` (the board's top 300), `read_now` (what a kept read holds), `key_attrs` (the top four by position weight over 0.02).
- `scouting.rs`: `to_stars`, `level_visibility`, tier cut 52; `career.rs`, `NFL_LEVELS` (65/75/85/92).
- `spots.rs`: `LEAN` 0.35, `fill_in`.
- `athlete.rs`: `misfit` (0.04, 25 lb slack, cap 0.5), `premium`, `Eye::coach`.

## Related pages

- [body.md](body.md): height, weight, combine, the forty, athlete-first creation.
