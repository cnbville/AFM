# Tendencies: how it works today

Checked against main at 94a6553 on 2026-10-25.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`.

## Quick answers

- What is a tendency? How often a player picks one option when the play gives him a choice, never how well it works; attributes decide that (`tendencies.rs`, module doc).
- How many? 15, in 8 groups (`tendencies.rs`, `Tend`, `TEND_COUNT`).
- Scale? 1-20, middle 10.5 = the real NFL average (`tendencies.rs`, `MID`). Stored as `f32` in `Tendencies.v` (`tendencies.rs`, `Tendencies`).
- Does the overall rating include them? No. `overall_for` reads attributes only. `team_fit` adds a fit value, but nothing in the world or game calls it today (`tendencies.rs`, `team_fit`, `fit_note`).
- Where do they come from? At 14: N(10.5, 4.4) plus small leans from body and mind (each clamped to ±3), recentred so a class averages 10.5 (`tendencies.rs`, `Tendencies::at_14`, `drawn`).
- Do they change? Yes, once a year inside the development step: results, coaching, the staff's style, the scheme, age and a major injury (`tendencies.rs`, `develop_year`, called from `development.rs`, `develop_season`).
- When do habits stop changing? Change is scaled by `settle = clamp((26.5 - age) / 4.5, 0.12, 1.0)`: full to 22, about half at 24, the 0.12 floor from 26 (`develop_year`).
- How does the engine read one? `Sim::tend(i, t)`: an instruction's number if he obeys this play, else his number, pulled up to a third of the way back to his old habit under stress, plus the game-situation gamble shift (`game/sim.rs`, `Sim::tend`). Most checks use `Sim::lean` = (tend - 10.5) / 9.5, from -1 to about +1 (`game/sim.rs`, `Sim::lean`).
- Do instructions always work? No. Obey chance `0.86 + 0.022*(disc20 - 10) + 0.012*(coachability - 10) + 0.12*(trust - 0.5) + 0.006*(strictness - 10)`, clamped 0.45-0.99 (`tendencies.rs`, `obey_chance`).
- Who gives instructions? AI head coaches, from their style and the habits they see, before every game at every level (`game/roster.rs`, `GameTeam::give_orders`, called in `world/season.rs`, `world/hs_year.rs`, `world/pro/games.rs`, `world/pro/other.rs`).
- How many coaching style settings? Five, each 1-20: risk, freedom, physical, ball hunting, mould (`tendencies.rs`, `Style`). The scheme pull and the head coach/coordinator blend are the design doc's items 5 and 6 (it numbers risk 0), not `Style` fields (`scheme_target`, `Style::blend`).
- Where does an AI coach's style come from? His traits and ability: risk = ambition, freedom from discipline against his level, physical from temper, mould from relationships, plus keyed noise (`world/coach.rs`, `Coach::style`). Your own style is `style_set` when set.
- Does the scouted read show the true number? No. A range on 1-20 and a word, widened by little film and a weak eye (`tendencies.rs`, `scout`). The true number is debug only.
- Does a major injury change contact willingness? Yes: +U(2, 4) to a dip taken off Contact, which keeps 40% each year (`develop_year`; read in `Tendencies::get`).
- Does losing speed change habits? Yes: Scramble, Bounce and Return risk each drop 0.25 per rating point of speed lost that year (`develop_year`).

## The 15 tendencies

`tendencies.rs`, `Tend`, `Tend::owned_by`, `Tend::ends`, `Tend::stat`, `Tend::gamble`. Every player carries all 15; "owners" are the positions whose page shows it.

| Group | Tendency (`Tend`) | Low end / high end | Owners | Real stat it is checked against | Gamble? |
|---|---|---|---|---|---|
| Ball carriers | Bounce | hits the called hole / bounces runs | RB | carries outside the tackles | yes |
| Ball carriers | Contact | avoids contact / seeks contact | QB RB WR TE LB CB S | runs ending out of bounds | no |
| Ball carriers | Reach | never reaches / always reaches | QB RB WR TE LB CB S | fumbles near the line | yes |
| Backs | Patience | downhill / patient | RB | time behind the line | no |
| Quarterbacks | Scramble | stays in the pocket / leaves it | QB | scramble rate | yes |
| Quarterbacks | Gunslinger | checks it down / gunslinger | QB | tight-window throws | yes |
| Quarterbacks | Hold | gets it out / holds for the big play | QB | time to throw | yes |
| Returners | ReturnRisk | plays it safe / brings everything out | RB WR CB S | kicks brought out of the end zone | yes |
| Pass rushers | Upfield | keeps contain / rushes upfield | EDGE DT | scramble yards allowed | yes |
| Pass rushers | SnapTiming | waits for the ball / guesses the count | EDGE DT | offside and neutral zone flags | yes |
| Pass rushers | HandsUp | rushes through / gets his hands up | EDGE DT | batted passes | no |
| Tacklers | Strip | wraps up / goes for the strip | EDGE DT LB CB S | forced fumbles per tackle | yes |
| Tacklers | BigHit | sure tackle / big hit | EDGE DT LB CB S | missed tackle rate | yes |
| LBs and safeties | Downhill | reads first / flies downhill | LB S | tackles for loss | yes |
| Coverage | JumpRoutes | stays home / jumps routes | LB CB S | interceptions per target | yes |

## Where they start (age 14)

`tendencies.rs`, `Tendencies::at_14` and `drawn`, on the player's own stream (`Rng::new(id ^ 0x7E4D_3C21)`), called from `player.rs`, `generate_prospect`.

- Each tendency: `N(10.5, 4.4) + clamp(lean, -3, 3)`, then minus `CENTER_14[k]` and rounded, clamped 1-20.
- Leans read his ceilings against 70, in steps of 12 (`c(a) = (ceiling - 70) / 12`), plus competitiveness `(comp - 10) / 3.5`, weight `(lb - 215) / 40`, height `(in - 75) / 2.5`:
  - Bounce `1.6*fast + 0.5*elusiveness`; Scramble `1.8*fast`; fast = mean of speed and acceleration.
  - Contact `1.2*big + 0.8*strength + 0.8*toughness`; BigHit `1.0*physicality + 0.5*big`.
  - Reach `0.8*compete - 0.5*discipline`; Gunslinger `0.8*arm + 0.6*compete - 0.6*discipline`; Hold `0.6*strength + 0.5*compete`.
  - Patience `1.0*vision - 0.8*acceleration`; ReturnRisk `1.0*fast + 0.6*compete - 0.5*discipline`.
  - Upfield `1.2*fast - 0.5*discipline`; SnapTiming `0.9*anticipation - 0.9*discipline`; HandsUp `(height - 75)/2.5`.
  - Strip `0.7*compete`; Downhill `0.9*fast - 0.6*discipline`; JumpRoutes `0.8*anticipation + 0.5*compete - 0.8*discipline`.
- Synthetic players made for a level (engine tools, synthetic teams) use `Tendencies::at_level`: the same draw squeezed by `level_sd(level) / 4.8` and centred with that level's `CENTER_*` table. `level_sd`: NFL 3.2, FBS 3.8, FCS 4.0, D2 4.2, HS 4.8. The world's players don't use it after 14.

## What his tools support (the ideal)

`tendencies.rs`, `ideal`, `misfit`, `Centres`.

- `ideal(t, attrs, level)` is the number where the habit neither costs nor pays at his level. It reads attributes against `norm = trait_norm(level, Speed) + 8` in steps of 6 (`rel(x) = (x - norm) / 6`; `trait_norm` base NFL 67, FBS 48, FCS 42, D2 37, HS 29, `game/rules.rs`).
- Examples: Bounce `10.5 + 1.8*rel(0.4 spd + 0.3 acc + 0.3 elus) + 0.6*rel(vision)`; Scramble `10.5 + 2.0*rel(0.6 spd + 0.4 throw on move)`; Gunslinger reads medium accuracy, arm and anticipation, minus a penalty for poor decisions; BigHit `10.5 + 1.5*rel(tackling) - 0.5`; Reach `10.5 + 1.2*rel(ball security) - 1.0`.
- Contact and HandsUp have no ideal: results and coaching never move them (only style pulls, drift, injury).
- `Centres` averages each ideal at each level and position over that year's players, so the average man at each position sits at 10.5. Built in `world/season.rs`, `develop` (high school and college) and `world/pro/mod.rs`, `camp` (NFL); passed as `SeasonContext.tend_centre`.

## Yearly change

`tendencies.rs`, `develop_year(t, attrs, personality, age, ctx, id)`, run at the end of `develop_season` on its own stream. Inputs (`YearCtx`): level, teaching (the same 1-20 coaching value growth uses), style (head coach and coordinator blend), scheme, played (0 with no games, starter 1.0, rotation 0.7, other 0.3), major injury, speed lost, centre.

For each tendency with an ideal: `gap = ideal - centre - v`; `beyond = max(|gap| - 5, 0)` (`STYLE_BAND` 5: inside it the habit is his style, untouched).

- Results teach: `learn = clamp(1 + (IQ - trait_norm(level, IQ)) / 25, 0.4, 1.6) * played`; natural move `sign(gap) * min(beyond, learn)`, halved when beyond <= 1.
- Coaching: `coach = clamp(0.375 * (teaching - 8) * (0.45 + 0.037 * coachability), -0.5, 3.4)`. Teaching 16 with coachability 15 gives about 3.0 a year; teaching under 8 is negative.
  - coach >= 0: moves toward the ideal by `min(beyond - |natural|, coach)`, so it stops at the band's edge.
  - coach < 0: moves away from the ideal by `-coach` (up to 0.5) when the habit is already outside the band.
- Style and scheme pull, to the band's edge: `pull = (max(coach, 0.5) / 3) * mould_factor`, toward each target with `sign(gap) * min(max(|gap| - 5, 0), pull)`.
- Drift: `N(0, 0.35) * settle` added to the natural part.
- Total: `dv = (natural + coached) * settle` (so drift is effectively x settle twice). Then Scramble, Bounce and ReturnRisk lose `0.25 * speed_lost`. Clamped 1-20.
- The old habit: `was` keeps only the moves he made on his own once coaching has touched the tendency; untouched, `was` follows `v` (`Tendencies.was`).
- Major injury: `contact_dip = contact_dip * 0.4 + U(2, 4)`; below 0.1 it resets to 0. `Tendencies::get(Contact)` subtracts it.

## Coaching styles

`tendencies.rs`, `Style`, `Style::blend`, `Style::mould_factor`, `Style::target`, `scheme_target`.

- Settings, each 1-20, 10.5 neutral: `risk` (conservative to aggressive), `freedom` (structure to freedom), `physical` (protective to physical), `ball_hunting` (sound to ball hunting), `mould` (adapt to mould).
- What each pulls (`Style::target`, target = clamp(10.5 + 0.8 * (setting - 10.5), 2, 19)):
  - risk: ReturnRisk, Gunslinger, Hold, Reach.
  - freedom: Bounce, Scramble, SnapTiming, Downhill.
  - physical: Contact, BigHit.
  - ball hunting: Strip, HandsUp.
  - JumpRoutes: half freedom, half ball hunting.
  - Patience and Upfield: no style pull.
- Mould factor `2^((mould - 10.5) / 9.5)`: 0.5 at 1, 1.0 at 10.5, 2.0 at 20. It scales the pull only, not the harm fix.
- Blend: unit style = head coach + (coordinator - head coach) / 3 per setting (`Style::blend`). Development uses the blend (college `world/season.rs`, `develop`; NFL `world/pro/mod.rs`, `camp`); high school uses the head coach alone. In games `GameStaff.style` is the head coach's alone (`world/season.rs`, `world/hs_year.rs`, `world/pro/games.rs`).
- Scheme targets (`scheme_target`): Hold: West Coast 7.5, Air Raid 9.0, Wide Zone and Power Gap 13.0. Gunslinger: West Coast 8.5, Wide Zone and Power Gap 12.0. Scramble: Spread Option 13.5. Upfield and Downhill: 4-3 front 12.5, 3-4 front 8.5. Wing-T and the rest: none.
- AI coach style (`world/coach.rs`, `Coach::style`): risk = ambition; freedom = clamp(21 - discipline + freedom_norm + 1.5 n, 1, 20); physical = clamp(10.5 + 0.4 (temper - 10) + 2.5 n); ball hunting = clamp(10.5 + 3 n); mould = clamp(10.5 - 0.25 (relationships - 10) + 2.5 n); n are keyed normals per coach. `freedom_norm` = the average discipline of head coaches at his level minus 10.5, recomputed at each development step (`world/coach.rs`, `set_freedom_norms`).

## Instructions and obedience

`tendencies.rs`, `Instruction`, `Instruction::sets`, `Orders`, `obey_chance`.

| Instruction | Sets (when obeyed) |
|---|---|
| KneelDeeperThan(d) | ReturnRisk 3; kicking also forces the kneel deeper than d |
| FairCatchInsideTen | ReturnRisk 3; kicking also forces fair catch / let it bounce |
| Slide | Contact 1 |
| TakeShots | Gunslinger 18, Hold 16 |
| ProtectBall | Gunslinger 3, Hold 5 |
| HighAndTight | Reach 1 |
| KeepContain | Upfield 2 |
| NoSnapGuessing | SnapTiming 1 |
| WrapUp | Strip 1, BigHit 2 |
| StayHome | JumpRoutes 1 |

- AI orders (`game/roster.rs`, `GameTeam::give_orders`), using the head coach's style and `eye = 3.5 - 0.1 * (film_study - 10)` against `misfit`:
  - RB/WR/CB/S returners: kneel (1 yd if risk <= 5, else 4) and fair catch if risk <= 7 or ReturnRisk misfit > eye.
  - QB: ProtectBall if risk <= 8 and Gunslinger misfit > eye/2; TakeShots if risk >= 14 and misfit < -eye/2; Slide if physical <= 10 and Contact >= 14.
  - QB/RB/WR/TE: HighAndTight if risk < 15 and Reach misfit > eye.
  - EDGE: KeepContain if Upfield misfit > eye, or freedom <= 6 and Upfield >= 14.
  - EDGE/DT: NoSnapGuessing if SnapTiming misfit > eye, or freedom <= 5 and SnapTiming >= 14.
  - LB/S/CB/EDGE/DT: WrapUp if physical <= 12 and (BigHit misfit > eye, or ball hunting <= 7 and Strip >= 15).
  - LB/S/CB: StayHome if freedom <= 12 and JumpRoutes misfit > eye.
- Obedience in a tracked play: `Sim::obeys` decides once per player per play from a key (agent id and line of scrimmage), not a draw. Reads discipline as a 1-20 trait against the level (`GamePlayer::trait20`), coachability, `GamePlayer.trust` (built at 0.5 in `GamePlayer::from_player`) and the head coach's strictness (`Mood.strict`).
- On kicks: `GamePlayer::follows` draws from the rng with the same `obey_chance` (`game/roster.rs`; `game/kicking.rs`).
- Your seat's plan can coach one habit toward a target: each practice week it moves `habit_rate(teaching, coachability, age) / 30 * obey` toward it, with obey from discipline/5, coachability, trust 0.5, strictness 10 (`training/seat.rs`, `week`, `habit_rate`). It writes through `Tendencies::set`, which also resets `was`, so this coaching leaves no old-habit trace.

## How the engine reads them

`game/sim.rs`, `Sim::tend`:

`tend = clamp(base + stress * (was - base) / 3 + gamble, 1, 20)`, unless an instruction sets this tendency and he obeys (then the set number, nothing else).

- `stress = clamp(stakes + 0.8 * tired + 0.05 * (10 - composure20), 0, 1)`.
- `gamble` applies only to gamble tendencies (`Tend::gamble`). From `game/flow.rs`, `mood`, with `late = clamp((quarter - game_left) / (0.67 * quarter), 0, 1)`: down 9+ → +2.5 late; behind with under a third of a quarter left → +1.5 late; up 9+ → -2.0 late; ahead with under a third left → -1.5 late.
- `stakes` = 0.5 in a playoff game + 0.4 * late when within 8 points with under 0.55 of a quarter left.
- Outside tracked plays (kicks) the same formula without the order check: `GamePlayer::tend_now` (`game/roster.rs`), with tiredness = 1 - energy.

Where each is read (all through `lean` unless noted):

| Tendency | Check | Source |
|---|---|---|
| Downhill | LB/safety read speed on the run fake: -0.14 base, bites x(1 + 0.9 lean) on a fake | `game/scrimmage.rs`, `run_snap` |
| Downhill | LB fills his gap from further off (1.8 + 1.5 lean yards) | `game/scrimmage.rs`, `defend` |
| Contact (raw `tend`) | carrier slides early under 9; above 12 lowers his shoulder | `game/scrimmage.rs`, `slide` |
| Contact | carrier at the sideline steps out or turns up | `game/sim.rs`, `run_carrier` |
| Contact | a shy tackler skips a bigger, moving carrier (0.35 x shyness) | `game/sim.rs`, `tackles` |
| Contact | yards after contact in the fight (+0.15 lean), injury exposure x(1 + 0.4 lean), dives on fumbles x(1 + 0.25 lean) | `game/sim.rs`, `fight_tick`, `hit_effects`, `fumble` |
| Contact | receiver short-arms over the middle with a hitter closing | `game/scrimmage.rs`, `catch_point` |
| Hold | time per read x(1 + 0.6 lean); throw clock + 1.0 lean seconds | `game/scrimmage.rs`, `quarterback` |
| Gunslinger | room he wants before throwing, more downfield | `game/scrimmage.rs`, `quarterback` |
| Scramble | how readily he leaves the pocket, x(1 + 1.2 lean) | `game/scrimmage.rs`, `quarterback` |
| Upfield | contain rusher's arc and the QB's escape lanes; deeper run squeeze | `game/scrimmage.rs`, `quarterback`, `defend`; `game/sim.rs`, `rush_power` (arc 10 x lean) |
| SnapTiming | get-off + 12 x lean | `game/sim.rs`, `engage` |
| HandsUp | batted pass chance x clamp(1 + 1.1 lean, 0.15, 2.4); rush power - 2.5 x lean | `game/scrimmage.rs`, `batter`; `game/sim.rs`, `rush_power` |
| JumpRoutes | jump chance 0.07 x (1 + 4 lean), after `freelance_ok` passes | `game/scrimmage.rs`, `jump_route` |
| BigHit, Strip | miss chance x(1 + 0.7 BigHit + 0.25 Strip) | `game/sim.rs`, `first_hit` |
| BigHit | hit force x(1 + 0.2 share x lean); personal-foul chance x(1 + 0.6 lean) | `game/sim.rs`, `hit_effects`, `tackled` |
| Strip | fumble chance x(1 + 1.15 lean); -0.12 lean in the tackle fight | `game/sim.rs`, `hit_effects`, `fight_tick` |
| Patience | back waits for the crease or commits early | `game/sim.rs`, `run_carrier` |
| Bounce | leans off the called hole while behind the line | `game/sim.rs`, `run_carrier` |
| Reach | reaches the ball out when 0.05-1.6 yd short: chance clamp(0.25 + 0.55 lean, 0, 0.97) | `game/sim.rs`, `reach` |
| ReturnRisk (`tend_now`) | kickoff kneel-or-return judgment (+0.9 yd of expected return per point over 10.5); fielding punts inside the 10 | `game/kicking.rs` |

- `freelance_ok` (coverage jumps, broken-play freelancing) reads football IQ, play recognition, trust and strictness, not a tendency (`game/scrimmage.rs`, `freelance_ok`).
- ReturnRisk's set number 3 from a kick order is not used: kicks read `tend_now`, which ignores orders, and apply the kneel and fair-catch orders directly (`game/kicking.rs`).

## The scouted read

`tendencies.rs`, `scout`, `shown`, `position_mean`, `word`.

- `scout(value, games, accuracy)`: `sd = clamp(5 / sqrt(1 + games) * (1.4 - 0.04 * accuracy) * 2.2, 0.35, 4.0)`; estimate = value + N(0, sd); range = estimate ± max(1.1 sd, 0.5), rounded, 1-20; word from the estimate.
- Words: under 4.5 "almost never", under 8 "rarely", under 13 "sometimes", under 16.5 "often", else "almost always".
- `shown` reads a number against his position's mean: only Contact has position means (QB -1.5, RB +1.5, WR -1.0, TE +1.0, linemen and LB +2.0, CB -1.5, S +1.0, K/P/LS -4.0); every other tendency shows raw.
- Scouting file page: owned tendencies only, `scout(shown, film * 1.5, max(best reader's judge, 6))`, film = the count of Film and Live looks, once the player is seen (`crates/afm-game/src/lane_scouting.rs`, `scout_file`).
- Your own player's page: all 15, read as a 1-99 range `reads::range(v * 99 / 20, half 6, ...)`, not through `scout` (`crates/afm-game/src/views.rs`, `player`).
- Scouting hints read the true number once: a CB or S with JumpRoutes >= 14 gets "gambles in coverage: jumps routes" (`world/scouting/file.rs`, `hints`).

## Where the knobs are

- `tendencies.rs`: `MID` (10.5), `STYLE_BAND` (5), `level_sd`, `CENTER_14` and the level `CENTER_*` tables, the leans in `drawn`, `ideal`, `scheme_target`, `Style::target` (0.8 and the 2-19 clamp), `mould_factor`, the coach, learn and pull formulas in `develop_year`, `obey_chance`, `scout`, `word`, `position_mean`, `Instruction::sets`.
- `game/roster.rs`: `give_orders` thresholds, `tend_now`.
- `game/flow.rs`: `mood` (gamble and stakes sizes).
- `game/sim.rs`: `Sim::tend` (stress weights, the 1/3 slip back), each check's lean multiplier.
- `world/coach.rs`: `Coach::style`.
- `training/seat.rs`: `habit_rate`.

## Code against the design doc

- `docs/design/tendencies.md` says a vertical offense pulls quarterbacks toward holding for the shot: in code the Air Raid pulls Hold down (9.0) and the play-action offenses (Wide Zone, Power Gap) pull it up (13.0).
- It says the read is a 1-20 range everywhere: your own player's page shows a 1-99 range.
- It says fit notes show on recruiting and draft boards: `fit_note` and `team_fit` exist but nothing calls them yet.

## Related pages

- [development.md](development.md): the yearly step that runs `develop_year`.
- [match-engine.md](match-engine.md): the checks that read `lean`.
- [staff-and-coaches.md](staff-and-coaches.md): coach ratings behind style and teaching.
- [scouting.md](scouting.md): readers, film and judge.
- [training.md](training.md): your plan's habit coaching.
- [ratings.md](ratings.md): trait-like ratings read against the level (`trait_norm`).
