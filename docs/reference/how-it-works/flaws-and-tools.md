# Flaws and tools: how it works today

Checked against main at 09047af on 2026-11-15 (individual practice PR 4: the Development tab; PR 3, working on them, at 05f50b1; PR 2, the engine layer, at 1bb7575). Every size on this page is proposed, not confirmed.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`.

## Quick answers

- What are they? 56 named pieces of technique, 37 flaws and 19 tools, each owned by one rating and each read only in its own situation, like Dynamic Links (`craft.rs`, `Craft`, `INFO`). A flaw costs its owner rating there; a tool adds to it.
- How big is one? A full one is 5 rating points of its owner in its situation (`craft.rs`, `FULL`). Each held one has a size from 0 to 1 (`Item.sev`); the engine reads sign × 5 × size (`Crafts::worn`).
- Read against what? Against his level's typical starter at his position, as ratings are: the engine reads his points minus the typical starter's mean points in it (0 for the men without it), measured on 64 synthetic teams a level (`game/synthetic.rs`, `typical_craft`; `Sim::craft`, `GamePlayer::craft_vs`). A level plays as it did and only how men differ changes: with and without the layer, 2,000 games a level, NFL points 24.87 and 24.86, FBS 23.82 and 23.94, high school 18.59 and 18.46, every other match_check row within its noise. A coach's field goal estimate reads the kicker's own (`fg_accuracy`).
- Where do they live? On the player (`Player.craft`, at most 6, `SLOTS`), and as the engine reads them on `GamePlayer.craft` (`Worn`, refreshed in `GamePlayer::from_player` and `refresh`).
- Where do they come from? Drawn once as a raw 14-year-old from his body, ratings and habits, then grown or shrunk every year at his checkpoint (`craft.rs`, `draw_raw`, `lean`, `year`; called from `development.rs`, `develop_season`).
- Who fixes them? His staff, every year: the better the coaching, the more coachable he is and the younger, the faster (`craft::year`). A poor staff (under 7 on 1-20) lets a flaw grow and now and then adds a new one. A focus on a flaw comes with part 3 (not built yet).
- Do they come back under pressure? Yes. A fixed flaw comes back up to a third of the way toward where it was under stress, the same stress as tendencies: `clamp(stakes + 0.8 × tired + 0.05 × (10 − poise), 0, 1)` (`game/sim.rs`, `Sim::craft`; `Worn::pts`). Where it was fades by 65% a year (`craft::year`, `was`).
- What does a fresh fix cost? The learning dip: in its first 3 weeks a flaw being fixed costs a quarter more (`DIP`, `DIP_WEEKS`, `Crafts::worn`). Practice snaps read the practice side, so a fix shows there first (`Worn::to_practice`, `game/flow.rs`, `practice_snaps`).
- Does the tape see them? Every rep where one was in play is tagged on the play record: who, which, and whether he won the rep (`game/sim.rs`, `Sim::tag`, `Sim::craft_at`; `PlayRecord.craft`, `PlayRecord::craft_tags`). The tags are read when the game ends and never stored, so game logs and saves keep their size. Block reps are decided with the block; other tags take the play's outcome for his side (`success`, EPA over 0).
- A position move? The old position's craft goes; he picks up raw flaws of the new spot, softened (`craft::moved`, from `athlete.rs`, `set_position`).
- Old saves? A world file from before them draws every person's on load (`save.rs`, `read_world`, `craft::ensure`).
- How many does a player carry? See "Counts by stage" below (`crates/afm-tools/src/bin/craft_check.rs`).
- Test players? The synthetic rosters grow theirs to their age through an average staff (`game/synthetic.rs`, `craft::ensure`). The A/B check sets one without and full on every man at its positions (`ab_check craft`, `Knob::Craft`).

## The 56 and their situations

Each line: the name, its owner rating, and where the engine reads it.

Quarterback (`game/scrimmage.rs`): Drops his eyes to the rush (Pocket Presence, his feel once a rusher is close, `quarterback`); Throws off his back foot (Footwork, the pressure miss read as poise, `throw`); Locks onto his first read (Decisions, half its points on his read speed past the first read, `quarterback`); Stares down his target (Eye Manipulation, a defender's anticipation on a short throw, `jump_route`); Long motion in traffic (Quick Release, the release with a rusher under a second away, `quarterback`); Pump fake that moves safeties (Eye Manipulation, the look-off on the first read, `quarterback`); Hard count that draws flags (Football IQ, a quarter step a point on the hard count, `game/flow.rs`); Back-shoulder throw (Touch, placement on a covered throw near the sideline, `throw`).

Backs and carriers: Ducks his head in pickup (Pass Blocking, a back's pass set against a man off the line, `game/sim.rs`, `block_craft`); Runs too upright (Leg Drive, the first half second of contact, `fight_tick`); Slow through the mesh (Ball Carrier Vision, his press speed after the mesh, `game/scrimmage.rs`); Jump cut (Elusiveness, against a linebacker behind the line, `first_hit`); Stiff arm (Leg Drive, his balance through an arm tackle at speed, `first_hit`); Carries it loose (Ball Security, with two or more defenders within 2 yards, `hit_effects`).

Receivers and tight ends: Rounds his breaks (Route Running, outs and comebacks); Tips his breaks (Route Running, every break); Sudden stop (Route Running, comebacks and hitches): all three on man coverage's reaction lag by the route's first break (`game/scrimmage.rs`, `route_craft`, `man_job`). Body catcher (Catching, a ball off his frame or with a man on him, `catch_point`); One release against press and Release package (Release, the jam at the line, `assign` press section); Late hands (Contested Catch, the contested catch, `catch_point`).

Blockers (`game/sim.rs`, `block_craft`, `update_blocks`, `lands_block`): Oversets (Pass Blocking, against speed and the counter); Catches instead of punching (Anchor, against power); Over his toes (Run Blocking, against a finesse defender); High pad level (Run Blocking, short yardage and double teams); Heavy feet on the climb (Blocking in Space, landing a block on a linebacker); Re-fits his hands (Hand Use, a pass rep past 0.3 that started to go wrong).

Pass rushers (`rush_craft`, `update_blocks`, `anchor`): No plan and Real counter (Finesse Rush, the counter's chance and its edge); High pad level (Anchor, against the double team); Lets the blocker into his chest (Block Shedding, the run shed); Eyes in the backfield (Play Recognition, his read on traps and misdirection, `game/scrimmage.rs`, read times); Long arm (Power Rush, in full against a shorter-armed blocker); Spin (Finesse Rush, in full against a tackle who oversets).

Linebackers: Catches blocks and Shock and shed (Block Shedding, a linebacker on a lineman in the run game, `rush_craft`); Overruns the ball (Pursuit, his pursuit angle, `pursue_cap`); No depth in his drop (Zone Coverage, a ball behind him in zone, `catch_point`); Loses the back on the wheel (Man Coverage, a back's deep route, `route_craft`); A-gap rip (Blitzing, rushing against a guard, `rush_craft`).

Corners and safeties: Peeks into the backfield (Man Coverage, against double moves and play action, `jump_route`); Opens his hips early (Man Coverage, an inside break, `route_craft`); Lunges at the jam (Press, the jam, and beaten clean when he loses it); Mirror press (Press, the jam); Doesn't play the hands (Catching, his breakup at the catch point, `catch_point`); Bait coverage (Anticipation, jumping a route, `jump_route`); Poor angles (Pursuit, his angle and open-field tackles, `pursue_cap`, `first_hit`); Eyes stuck on the quarterback (Play Recognition, the look-off's pull on him); Late rotation (Disguise, the shell's read, `game/flow.rs`); Robber bait (Anticipation, jumping a route in front of him, `jump_route`).

Specialists (`game/kicking.rs`): Plant foot drift (Long Kick Accuracy, the long share of a field goal, `fg_accuracy`); Drop inconsistency (Punt Power, a wider spread on the punt's distance); High snaps in the wet (Long Snapping, in rain or snow, `wet_snaps`); Pooch and coffin corner (Punt Placement, pinning from midfield); Onside touch (Kickoff Technique, the onside kick).

Tacklers: Dives at ankles (Tackling, at a runner moving over 5 yards a second, `first_hit`).

## Where they come from

- The raw draw at 14: each flaw that fits his position with chance `min(3.6 / flaws at the position, 0.72) × lean`, size `0.35 + 0.5 × u × lean` (0.2 to 1); each tool at `0.04 × lean`, size 0.15 to 0.4; the biggest 6 kept (`draw_raw`).
- Lean (0.7 to 1.4): his body, his ratings against his own overall, and his habits, as `2^(v + own)` (`lean`). A flaw leans toward his weak spot, a tool toward his strong one. Examples: a tall tackle oversets more, a short-handed back carries it loose more, a gunslinger stares down more.
- His own keyed stream (id, age, salt), so nothing else in the world draws differently (`stream`).

## The year

- The staff's own points (topic 10): each year his staff works two flaws on the weekly rule below (`pace`), over `STAFF_WEEKS` (12) weeks at the usual 30 minutes shared between them, times the level factor and `0.6 + 0.8u` (`craft::year`, `staff_points`). An adapting staff (style mould 1) picks his two biggest; a moulding one (20) weighs each by its rating's weight at his spot, so a tackle's pass-set flaws come before his climb to the second level. Your own players skip this: your week works their points (below).
- A step up a level exposes habits the level below never punished: on his first year at a higher level (college, the pros) each flaw he doesn't carry comes with chance `raw share × 0.4 × lean` (`EXPOSED`), size 0.15 to 0.6, while a slot is free (`Crafts.lvl` keeps the highest level).
- The rest move slowly (`BACKGROUND`, 0.75 of the old rule): `fix = 0.75 × 0.11 × coach × take × settle × level`, coach = `sqrt(clamp((coaching − 4) / 8, 0, 2))`, take = `clamp(0.6 + 0.02 × coachability, 0.6, 1)`, settle = `clamp((26.5 − age) / 4.5, 0.12, 1)`, level HS 0.75, college 1.0, NFL 1.1. Each flaw shrinks by `fix × ease × (0.4 + 1.2u) × …`, ease his own for that flaw (0.4 to 1.6, keyed on his id and the flaw); under 0.06 it is gone (`GONE`).
- A poor staff (coaching under 7): each flaw grows `0.06 × (7 − coaching) / 6` a year, and a new one comes with chance `0.08 × (7 − coaching) / 6`.
- Tools grow `(0.12 + 0.18 × playing) × min(coach, 1.5) × take × (0.5 + u) × max(settle, 0.4)` a year; playing is 1 for a starter, 0.6 rotation, 0.3 backup, 0.1 otherwise (`development.rs`).
- A new tool: chance `0.3 × gift × min(coach, 1.5) × (0.4 + 0.6 × playing) × take × max(settle, 0.3) / (1 + tools held)`, gift the mean lean toward the ones left to learn, squared; picked by lean.

## Working on them (your seat)

Your team's week (`training/seat.rs`, `week`, then `world/craft_work.rs`, `week`):

- Points of emphasis: one or two a player, each a flaw, a tool or a plain rating (`craft::Point`, stored in `World.seat_focus` as one byte: a rating's index, or 100 plus the craft's). Set with `World::set_points`; the old `set_focus` sets the first point to a rating and keeps a second. Every point grows its rating (a flaw's or tool's owner) on the part 1 rule; two points split his minutes (`development::Focus::two`, `minutes_on`). With none of yours, his position coach works the two he has named that cost him most on tape in the last eight weeks (`staff_points_of`, weighted by style as above); a staff point grows no rating, as AI staffs' don't.
- Minutes: his week's individual minutes on the field and in the film room (`focus_minutes`, plain and mental), split between his points; a film-room flaw (`Info.mental`) works on the film minutes.
- A week's work (`craft::work_week`): practice moves first by `pace`, games follow at 0.3 of the gap a week, and the first three weeks carry the learning dip (`DIP`). `pace = 0.04 × teach × take × age × minutes/30 × mind × ease × buy-in`: teach `clamp((teaching − 4) / 11, 0.1, 1.1)^1.5`, take `clamp((coachability − 2) / 14, 0.2, 1.15)^1.2`, age `1 − 0.0625 × (age − 22)` from 0.5 to 1, mind (film-room flaws) `0.5 + (IQ − 40) / 50` from 0.5 to 1.3. Measured (`tests/craft.rs`): a typical overset (0.5) on a coachable (16) 20-year-old with a strong teacher (15) at 30 minutes holds in games after 13 weeks (sizes item 2: 12 to 16); at 30 he takes 24; a poor teacher (7) or a low coachability (6) takes over 40.
- Stages (`craft::stage`): Learning, Works in practice (past the dip and practice under two thirds of where it was), Fixed in games (games under 0.08, `HOLD`); a tool holds at 0.4 (`TOOL_HOLD`). A tool he doesn't have starts from nothing. A flaw he doesn't carry (the staff named the wrong one) moves nothing, though its rating still grows.
- Against an AI staff: your week works a point every week of the year at his real minutes, where an AI staff's year counts 12 weeks at the usual minutes (kept that low so the world's counts hold). So a point you give real minutes moves several times as fast as the same teacher's own point would (not measured on a season yet; proposed, not confirmed, and an open choice for Cain).
- The year leaves what the weeks worked (`Item.wk` top bit), and only its stress return fades.
- Who teaches (`craft_work::room`): the position coach at a college, the coordinator of his side at an NFL club or a high school, else the head coach; his teaching in that group and his eye. An empty room reads as 8.
- Buy-in (`craft::buy_in`): `0.55 + 0.02 × (coachability − 10) + 0.4 × (trust − 0.5) − 0.015 × (ego − 10) × veteran`, 0.2 to 1, veteran rising from 24 to 30.
- He can refuse (`set_points`): a new flaw point on a veteran with an ego past 12 is turned down with chance `0.6 × (ego − 12)/8 × veteran × star × (1 − trust)`, at most 0.55 (star 1 for a starter, 0.4 otherwise). The point is left off, his trust in you drops 0.03 and his mood 0.03.
- He feels it: when a point reaches practice or holds in games, his position coach's note says so in the week's notes, and a hold lifts his mood 0.03.
- The staff's eye (`craft_work::week`): each week each flaw or tool he carries and nobody has named is found with chance `0.06 × eye/10 × (0.3 + size) × (1 + 0.5 × reps on tape this week)`, eye/10 from 0.3 to 1.8. A weak eye names a neighbour instead (same position, same rating group, one he doesn't carry) with chance `0.32 − 0.016 × eye`, 0.04 to 0.3. A wrong name fades after four weeks, 0.05 × eye/10 a week, as the tape never shows it (`World.seat_seen`).
- The tape book (`World.seat_book`, `craft_work::Tag`): for your players, one row a game (from `keep_seat_game`) or practice week (`practice::seat_battles`) and craft: reps it showed in and reps it lost him, kept two seasons.

- Last week's minutes are kept for the screens (`World.seat_minutes`: on the field, in the film room; `minutes_of` gives the usual 30 and 30 before his first week).

Not built yet: private trainers and camps (topic 7.3-7.4), a mentor passing a tool on (7.2), a player asking for a tool (8.3), the world's scouts and press reading a fix (9.5).

## On screen (the Development tab)

The player page's Development tab (`lane_training/craft.rs`, `training.js`; the layout in `docs/game.md`) reads all of this through your staff:

- Gains: a flaw's size times 5 (a tool's, to 0.6), read with a lean and a half-width of `0.5 + (20 − eye)/12` points (`gain_read`). A flaw the staff named wrong is shown and projected at a believed size of 0.3 to 0.7 (`believed`).
- Rates: the tape book counts every rep in the flaw's situation and whether he lost it, so the screen shows the share lost in his games (last 56 days on Focus; last four games against his first four on Progress), and the chart plots that share a game (solid) or a practice week (dashed). A fixed flaw moves the share only a few points (a test career's held flaw went 55% to 51%), since most reps in its situation are decided by everything else on the play.
- When it holds (`craft::weeks_to_hold`): `work_week` run on a copy of him at last week's minutes split over his points, his room's teaching and his buy-in, up to 156 weeks; shown as a date range of ±(0.15 + (20 − eye)/40) of the weeks, with the seasons of payoff left (a season counts when it holds before October 15 of it; college from `eligibility::seasons_left`, high school from his class, none for a pro), and how much sooner 15 more minutes lands it.
- The rating with it and without (`craft_work::focus_projection`): `develop_season` on two copies of him with the same dice, his room's teaching as coaching, his last role, `seat_plan` on; a year for "by the year's end" and his seasons left (1 to 4) for "by his last season". Each moves the staff's own read of him now by what the copy grew, widened by `(2 + (20 − eye)/5) × 0.5` for the last season.
- How sure: `eye/20 + min(reps seen/30, 1)/2`: sure at 1, fairly sure at 0.7, not sure yet at 0.45, else only a guess. Spots flaws: sharp at 15, good at 11, fair at 7, else weak. On board: buy-in fully at 0.85, mostly 0.65, half 0.45.
- Setting points: `practice_points` (codes as `Point::code`), checked against his position and your controls; a refusal comes back as a toast. The Squad's Development view shows his points (`points_now`: yours, else the staff's named flaws, else its rating focus).

## Counts by stage

Measured with `craft_check 300 97 <staff>` (300 players a position, every position, grown as starters through high school, college from 18 and the pros from 22; staff on 1-20). Flaws and tools a player, with their mean size:

| Staff | At 14 | At 18 (freshman) | At 22 | At 26 | At 30 |
| --- | --- | --- | --- | --- | --- |
| 11 | 3.1 flaws (0.63) | 3.1 (0.43), 0.6 tools | 2.6 (0.26), 1.0 tools | 1.7 (0.21), 1.1 | 1.3 (0.21), 1.2 |
| 16 | 3.1 (0.63) | 2.9 (0.35), 0.7 | 1.4 (0.22), 1.1 | 0.7 (0.18), 1.3 | 0.4 (0.18), 1.3 |
| 5 | 3.1 (0.63) | 3.2 (0.64), 0.3 | 3.5 (0.61), 0.5 | 3.7 (0.61), 0.6 | 3.7 (0.67), 0.7 |

With part 3 (2026-11-15: the staff's two points, the step-up exposure and the slower background). Part 2 alone gave 2.4, 1.5 and 1.4 flaws at 22, 26 and 30 under staff 11, and 1.7 at 22 under staff 16: a strong staff now cleans a player up faster, and a poor one's players pick up new habits at each step up.

Kickers, punters and snappers carry under 1; the rest 3 to 4.4 at 14. A flaw's ease of fixing is his own and fixed (0.4 to 1.6), so some habits outlast a good staff.

## What each one does in games (A/B)

`ab_check craft nfl 2000 1` (2026-11-14): each one without and full on every man at its positions, 2,000 games an arm, same seeds; headline row first, 95% bands.

- Clear and the right way at full size (19): drops his eyes (+0.6 pts sacks a pressured dropback), runs too upright (−0.03 yards after contact a carry), rounds his breaks and tips his breaks (−0.04 yards of separation), body catcher, release package (+0.03 separation), late hands, oversets, catches instead of punching, over his toes (−1.0 pts run blocks won), high pad level (blocker), heavy feet on the climb, re-fits his hands (−1.2 pts pass blocks lost), lets the blocker into his chest (−2.9 pts run sheds), long arm, doesn't play the hands, plant foot drift (−3.0 pts on 50+ yard kicks), drop inconsistency (−0.32 net punt yards), dives at ankles (+0.3 pts missed tackles).
- Right way but only clear at four times size (a local probe that scaled every one by four, not in the code): throws off his back foot (picks under pressure), long motion, stiff arm (yards after contact), no plan, real counter, spin, catches blocks, lunges, poor angles, the coffin corner, carries it loose.
- Too rare or too small to show in 2,000 games even at four times size: locks onto his first read, stares down his target, pump fake, back-shoulder throw, ducks his head in pickup, slow through the mesh, jump cut, one release, sudden stop, high pad level (rusher), eyes in the backfield, overruns, no depth, the wheel, shock and shed, A-gap rip, peeks, opens his hips, mirror press, bait, eyes on the quarterback, late rotation, robber bait. Minor bug 386.
- No row for their situation: the hard count, high snaps in the wet (A/B games are dry), the onside touch.
- None moved clearly the wrong way.

## Where the knobs are

`craft.rs`: `FULL`, `SLOTS`, `GONE`, `DIP`, `DIP_WEEKS`, `raw_share`, `draw_raw`, `lean`, `year`, `moved`, `BACKGROUND`, `STAFF_WEEKS`, `EXPOSED`, `PACE`, `HOLD`, `TOOL_HOLD`, `pace`, `buy_in`, `refuse_chance`, `staff_points`. `world/craft_work.rs`: the eye's chances, `BOOK_DAYS`. The situation weights sit at each hook in `game/sim.rs`, `game/scrimmage.rs`, `game/kicking.rs` and `game/flow.rs` (search `individual practice`).

## Related pages

[development.md](development.md), [training.md](training.md), [match-engine.md](match-engine.md), [tendencies.md](tendencies.md).
