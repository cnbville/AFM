# Progression: how it works today

Checked against the progression update steps 1-2 branch over main 69db832 on 2026-11-15; step 3 (the strength split, the bench, the weight room by body knack) over c2f99ef the same day; step 3c (body projects) over c2baa49 on 2026-11-16; the thinned limit (step 4c) over main 66cf9e1 the same day, before it merged; slower closing near the top and the fitted trims (step 5) over main 1cc9d78 on 2026-11-17.

The split ceiling: the limit, the knacks, everyday closing, breakouts that close room, skill projects and the trims (designs `docs/design/progression.md` and `docs/design/progression-everyday.md`). Paths are relative to `crates/afm-engine/src/` unless they start with `crates/`. Everything here is in `progression.rs` unless named.

## Quick answers

- Which skills have a limit? 17: man and zone coverage, press, tackling, block shedding, contested catch, route running, release, power and finesse rush, pass and run blocking, footwork, leg drive, pursuit, blitzing, throw on the move (`LIMITED`). Every other skill grows as it did before, with no limit.
- What is his limit? What players with his tools reach at his position, plus 4, plus the whole gap at 14 when that level sat above his own ceiling. It is never under his ceiling (`limit`, `MARGIN`, `ensure`).
- What are "his tools"? Two-thirds body (his physical ceilings, plus arm length for rushers and blockers) and one-third makeup (his makeup ceilings), read for each skill (`spec`, `parts`).
- Does the limit move? Yes. It reads his ceilings as they are now, minus how much his class drifts by his age at his position, so it moves only with his own growth past his class's (`mapped`, `drift`).
- Can a freak be capped? No. Above the best tools in the fit sample, the map keeps rising at the top tenth's slope (`mapped`).
- What closes room every year? Everyday coaching: room × 0.10 × knack pace × coaching × snaps × stage × near_top (`everyday`).
- Does closing slow near the top? Yes. Past a ceiling of 55 each point is harder: a 60 closes at half pace, a 70 at a quarter, an 80 at a sixth (1 / (1 + (ceiling − 55) / 5)). It never reaches zero. Everyday closing, breakouts and project years (clicks too) all take it; the weight room and natural rises don't (`near_top`, `SLOW_FROM`, `TOP_SLOW`).
- What is the knack? Six hidden technique knacks (coverage, routes, blocking, rushing, passing, tackling) and one body knack. Each is drawn at 14 and about a third of a technique knack is coachability (`ensure`, `knack`).
- How fast does a knack close room? Pace is Φ(1.4 × (z − 0.3)). About 1 in 5 has the knack (z 0.84 or more), half are middling, and about 1 in 3 has none (z under −0.43) (`pace`, `tier`).
- Does a breakout close room? Yes, on one skill: the one his position needs that has the most room. It closes 6 to 10 points, never past the limit (`breakout`, `CLICK`).
- Does a natural rise get held back? No. A ceiling that rises past the limit lifts the limit with it (`settle`).
- Does a falling body take ratings away? No. It only stops new gains (`limit` floors at his ceiling).
- How big is a project year? Room × 0.30 × pace × coaching × U(0.7, 1.3) × stage, plus a click chance of 0.25 × pace that adds 6-10 points, half of it shown on his rating (`project_year`, `PROJECT`, `CLICK_CHANCE`).
- What does a project cost? His other technique skills at the position grow at 0.75 of their rate. The project skill grows at the 60-minute focus multiplier (`PROJECT_COST`, `cost_mask`; `development.rs`, `GrowthCtx::rate`).
- How many projects can a room run? QB 1, RB 1, WR 2, TE 2, OL 2, DL 2, LB 1, CB 2, S 1. Kickers, punters and snappers get none (`Room::slots`).
- How do AI staffs pick? By their own lean, moved by their seat, reading room and the knack's clues a bit less sharply than your staff (`Lean::of_coach`, `ai_room`, `AI_SHARP` 0.6).
- What are the trims? Points taken off natural ceilings of 60+ in one skill at one position, the first year he plays that spot (age 22 or under). The size depends on his natural ceiling there: most in the 65-80 band, almost nothing near 60, and in most families little or nothing for a natural 85+. The limit stays where it was, so investment can win them back (`TRIMS`, `trim_at`, `land_trims`).
- What is a body project? The strength staff rebuilding his body on its own leash: a strength project moves his upper and lower strength ceilings (up to 14 each) and balance (up to 3) and puts on weight; a speed project moves speed (1), burst (1.5 each for acceleration and jumping) and agility and hips (2 each). His body knack opens Φ(1.2 z) of each leash, so a typical responder gets about half and a big one nearly all (`BodyKind`, `body_share`, `body_leash`).
- Can a body project and a skill project run together? Yes: the body side is the strength staff's time, the skill side the room's. A body project year adds 0.25 wear (`BODY_WEAR`).
- Is there a yearly growth limit? No. The old 12-point yearly cap is gone (physical study item 16; `development.rs`).

## The limit

- The rank maps: 60,000 boys from `athlete::create` on a fixed stream, fitted once per run. For each limited skill and position, his tools score is standardised against the sample there, and its 41 quantiles (every 2.5%) are mapped onto the skill ceiling's quantiles (`maps`, `RankMap`, `FIT_SAMPLE`, `Q`).
- His read: each part's ceiling, minus the class's drift for that part at his position and age, standardised against the map's mean and spread, weighted (two-thirds split evenly over the body parts, one-third over the makeup parts), then read off the map with linear steps between quantiles (`mapped`, `mapped_on`).
- The class's drift: 24,000 generator boys grown through `develop_season` from 14 to 30, with the split ceiling off for them (high school to 17, college to 21, then the pros, a coach of 10, rotation snaps). It is stored as each part's mean ceiling at each age minus its mean at 14, per position (`drift`, `Drift`, `DRIFT_SAMPLE`). It takes about 2 seconds, once per run, on its own thread pool (`off_pool`). At 22 a corner's speed ceiling has drifted about +3.5 and his play recognition about +1.0 (`prog_check drift`).
- A convert reads the position that leans on the skill most when his own doesn't use it (`map_position`).
- Each limit's own part (`Prog.over`, half points): 4, plus max(map − ceiling, 0) at the draw, plus every natural rise past the limit since (`ensure`, `settle`).
- The limit is the map plus its own part taken as a rise from the map, so past 95 it thins like every rating (`limit_of`, `athlete::top_up`; step 4c, physical study item 17). The maps read strength's fat top (rushing, blocking, tackling), and without it a strong man's power rush or press limit ran to 150. `settle` keeps the part on the same scale (`raw_top`).
- `settle` lifts the limit only by the ceiling's own rise that year, and only past the higher of his limit when the year opened and now. So a one-year dip in his read never ratchets it up (`settle`).
- An older save draws once, from where the player is now, with no trims pending (`ensure`; `player.rs`, `#[serde(default)] prog`).

## Everyday closing

- Every year, every limited skill his position works: ceiling += room × 0.10 × pace × coach_mult × snaps_mult × stage_mult × near_top(ceiling) (`everyday`, `EVERYDAY`, `near_top`).
- coach_mult = 0.5 + coaching / 20, from 0.5 to 1.5 (`coach_mult`).
- snaps_mult: starter 1, rotation 0.8, redshirt 0.6, other 0.5. A suspended or dismissed year closes nothing (`snaps_mult`).
- stage_mult: 1 in high school and every college season, whatever his age. In the pros it is 0.5 in seasons 1-2, 0.2 in seasons 3-4, and 0 after (`stage_mult`).
- It runs inside `develop_season`, after the breakout lift and before growth toward the ceiling (`development.rs`).

## Skill projects

- One per player at most, on one limited skill (`Prog.project`). The 60-minute focus lift is applied to that skill, and the other technique skills pay the cost (`development.rs`, `GrowthCtx.project`, `cost_mask`; `training/mod.rs`, `ctx`).
- A world's AI staffs decide at each year's checkpoint, before growth. College programs and high schools work through their rooms, except your own team (`world/season.rs`, `ai_projects`). NFL clubs do it at camp with a balanced lean and their own coaching eye (`world/pro/mod.rs`).
- `ai_room`: it ends dead ends first (`ai_keeps`: under a point and a half a year after two years, or under 1 point of room left). Then it fills the open slots, best read first, at the chance `Lean::fill` = 0.45 × lean (from 0.05 to 0.95) × the staff's time (high school 0.3 + 0.5 × resources). A bet needs a read of at least 3 points (`BET_FLOOR`). A win-now staff only bets on the top third by overall.
- The lean: from the head coach's id, 3 in 10 are developers (2.0), 4 in 10 balanced (1.0) and 3 in 10 win-now (0.25). The coach's security, −1 to 1 from his contract years left, halves or doubles it (`Lean::of_coach`).
- The staff's read: the room it sees, plus noise that shrinks with its eye (sd 9 − 0.3 × eye, from 2 to 9), times the pace it reads. Before any work, the read is coachability and a sharper staff sees more of the knack (`staff_read`, `best_bet`).
- The class sim has no rooms. Each boy's chance a year is 0.03 in college or 0.02 × coaching / 10 in high school, × slots / 2 × his lean (`ai_single`; `career.rs`, `class_lean`).

## Body projects

- One per player at most (`Prog.body`, `body_years`, `body_used`, `body_lbs`, `body_yours`). It runs at his checkpoint inside `develop_season`, after the normal weight room (`body_year`; `development.rs`).
- A year closes 0.45 × coach_mult(strength staff) × U(0.7, 1.3) of what's left of each rating's leash, on the ceiling (`BODY_YEAR`). His rating then grows toward it like any other ceiling. A full leash ends an AI staff's project.
- His frame sets how far a strength project goes: 1 + 0.15 × (height z − frame weight z) + 0.05 × arm z against his position's combine sizes, from 0.6 to 1.05. A tall, light frame gets the whole leash, a short, heavy one about 60% of it (`frame_room`).
- Pounds follow the strength built: 1.2 lb a point (the mean of the upper and lower body's gains), more on a bigger frame by frame weight / 190, at most 25 lb, through `training::seat::body_change` (so each pound costs speed and agility as the weight plan does) (`BODY_LBS_PER_POINT`, `BODY_LBS`).
- The body's clock, not the school's: it stops after 25 at any level, a high school staff runs one only if it's yours, and a suspended or dismissed year does nothing (`BODY_UNTIL`).
- AI staffs: a college program's strength staff runs one a room at a time (`BODY_SLOTS` 1, about nine a program), on the man whose upper and lower strength trail the typical FBS starter at his spot most by its read (error sd 6 × (1.5 − eye / 20)), times his frame room, if that read is 8 or more (`body_bet`, `BODY_BET_FLOOR`), at the lean's fill chance. NFL clubs run the same through `ai_room`. The class sim gives a collegian with a read of 8+ a 0.08 × lean chance a year (`ai_single`).
- Measured (`tests/progression.rs`, 120 receivers, FBS starters at 18-22 with a staff of 12): a big responder (body knack z 1.6) gains about +13 on his strength ceiling, carries about 13 lb more and runs about 0.03 s slower (his added strength wins some of the weight's cost back); a non-responder (z −1.6) gains about +0.5 and no weight.
- The weight program's room (`development.rs`, the weight room) counts a body project's pounds as past it, so it never takes them back.

## The trims

- Each family (one skill at one position) has two slopes `[a, b]`: a points for each point of natural ceiling past 60, and b more for each point past 75 (most often negative, so the trim eases off), never under 0. A higher natural ceiling always keeps a higher trimmed one (`trim_at`, `TRIM_TABLE`, `TRIM_STEEP` 75).
- Sizes at a natural 70 and 80 (fitted on the class sim, `prog_check fit`): OT pass blocking 5.9 and 8.2; RB leg drive 5.3 and 4.9; WR routes 3.2 and 1.8; CB man coverage 2.1 and 2.7; QB footwork 1.1 and 1.4; edge finesse rush 0.9 and 1.9; LB pursuit 0.1 and none. The rest are in `TRIM_TABLE`.
- They are pending for every boy drawn at 15 or younger (`ensure_new`) and land the first year he plays a spot with a trim, if his ceiling there is 60+ (`TRIM_FROM`). They are dropped after 22 (`TRIM_UNTIL`). His rating comes down with the ceiling if it sat above it (`land_trims`).

## What it does not do yet

- The physical study's items 1-3 and 6-18 are step 4 of the build (4a bodies, 4b the drills and the split, 4c no 99 wall: see [ratings.md](ratings.md), "Past 99"). Your own strength staff's body projects have no command yet: `start_body` with `yours` set is the hook. The weight room inside the leashes, the body knack's own effect there and the strength split are built (step 3): see [body.md](body.md), "The normal weight room by body knack" and "Is strength one rating?".
- The screens show none of it yet, apart from reads past 99 showing "99+" (step 6, [ratings.md](ratings.md)).
- Your own staff's projects have no command yet: `start_project` with `yours` set is the hook.

## Measured (class sim, `prog_check 250000 progA`)

Counts are collegians whose best college season reached 70 or 80, off > everyday only > every part with the design trims:

| Family | 70+ | 80+ |
|---|---|---|
| CB man coverage | 100 > 145 > 145 | 11 > 13 > 29 |
| Edge finesse rush | 53 > 91 > 85 | 6 > 16 > 22 |
| WR routes | 101 > 131 > 159 | 13 > 17 > 29 |
| LB zone coverage | 98 > 146 > 132 | 10 > 17 > 21 |
| OT pass blocking | 44 > 55 > 87 | 5 > 6 > 12 |
| QB footwork | 29 > 46 > 48 | 6 > 6 > 8 |

Projects run in 2-7% of college seasons by position. The design trims don't hold today's star count: they get sized again in step 5, after the physical items.

## Where the knobs are

- `progression.rs`: `LIMITED`, `spec`, `MARGIN` 4, `FIT_SAMPLE` 60,000, `DRIFT_SAMPLE` 24,000, `EVERYDAY` 0.10, `CLICK` (6, 10), `PROJECT` 0.30, `CLICK_CHANCE` 0.25, `PROJECT_COST` 0.25, `Room::slots`, `Lean`, `AI_SHARP` 0.6, `YOUR_SHARP` 1.0, `BET_FLOOR` 3, `BodyKind::ratings`, `BODY_YEAR` 0.45, `BODY_LBS` 25, `BODY_LBS_PER_POINT` 1.2, `BODY_UNTIL` 25, `BODY_WEAR` 0.25, `BODY_BET_FLOOR` 8, `BODY_SLOTS` 1, `SLOW_FROM` 55, `TOP_SLOW` 5, `TRIM_TABLE`, `TRIM_FROM` 60, `TRIM_STEEP` 75, `TRIM_UNTIL` 22.
- Tooling switches: `set_enabled`, `set_parts` (everyday 1, breakouts 2, projects 4, trims 8, body 16), `set_trims`, `set_slow_from`, `set_top_slow`, `set_trim_limit_share`. `crates/afm-tools/src/bin/prog_check.rs` (`AFM_PROG_PARTS`, `AFM_TRIM_ALL`, `AFM_SLOW_FROM`, `AFM_TOP_SLOW`, `AFM_TRIM_LIMIT`, `fit`, `drift`).

## Related pages

- [development.md](development.md): growth toward the ceiling, breakouts, focus.
- [training.md](training.md): the focus and its minutes.
- [body.md](body.md): physical ceilings and the weight room.
- [flaws-and-tools.md](flaws-and-tools.md): the technique layer under the ratings.
