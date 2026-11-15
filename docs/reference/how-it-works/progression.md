# Progression: how it works today

Checked against the progression update steps 1-2 branch over main 69db832 on 2026-11-15.

The split ceiling: the limit, the knacks, everyday closing, breakouts that close room, skill projects and the trims (designs `docs/design/progression.md` and `docs/design/progression-everyday.md`). Paths are relative to `crates/afm-engine/src/` unless they start with `crates/`. Everything here is in `progression.rs` unless named.

## Quick answers

- Which skills have a limit? 17: man and zone coverage, press, tackling, block shedding, contested catch, route running, release, power and finesse rush, pass and run blocking, footwork, leg drive, pursuit, blitzing, throw on the move (`LIMITED`). Every other skill grows as it did before, with no limit.
- What is his limit? What players with his tools reach at his position, plus 4, plus the whole gap at 14 when that level sat above his own ceiling. It is never under his ceiling (`limit`, `MARGIN`, `ensure`).
- What are "his tools"? Two-thirds body (his physical ceilings, plus arm length for rushers and blockers) and one-third makeup (his makeup ceilings), read for each skill (`spec`, `parts`).
- Does the limit move? Yes. It reads his ceilings as they are now, minus how much his class drifts by his age at his position, so it moves only with his own growth past his class's (`mapped`, `drift`).
- Can a freak be capped? No. Above the best tools in the fit sample, the map keeps rising at the top tenth's slope (`mapped`).
- What closes room every year? Everyday coaching: room × 0.10 × knack pace × coaching × snaps × stage (`everyday`).
- What is the knack? Six hidden technique knacks (coverage, routes, blocking, rushing, passing, tackling) and one body knack. Each is drawn at 14 and about a third of a technique knack is coachability (`ensure`, `knack`).
- How fast does a knack close room? Pace is Φ(1.4 × (z − 0.3)). About 1 in 5 has the knack (z 0.84 or more), half are middling, and about 1 in 3 has none (z under −0.43) (`pace`, `tier`).
- Does a breakout close room? Yes, on one skill: the one his position needs that has the most room. It closes 6 to 10 points, never past the limit (`breakout`, `CLICK`).
- Does a natural rise get held back? No. A ceiling that rises past the limit lifts the limit with it (`settle`).
- Does a falling body take ratings away? No. It only stops new gains (`limit` floors at his ceiling).
- How big is a project year? Room × 0.30 × pace × coaching × U(0.7, 1.3) × stage, plus a click chance of 0.25 × pace that adds 6-10 points, half of it shown on his rating (`project_year`, `PROJECT`, `CLICK_CHANCE`).
- What does a project cost? His other technique skills at the position grow at 0.75 of their rate. The project skill grows at the 60-minute focus multiplier (`PROJECT_COST`, `cost_mask`; `development.rs`, `GrowthCtx::rate`).
- How many projects can a room run? QB 1, RB 1, WR 2, TE 2, OL 2, DL 2, LB 1, CB 2, S 1. Kickers, punters and snappers get none (`Room::slots`).
- How do AI staffs pick? By their own lean, moved by their seat, reading room and the knack's clues a bit less sharply than your staff (`Lean::of_coach`, `ai_room`, `AI_SHARP` 0.6).
- What are the trims? Points taken off natural ceilings of 60+ in one skill at one position, the first year he plays that spot (age 22 or under). The limit stays where it was, so investment can win them back (`TRIMS`, `land_trims`).
- Is there a yearly growth limit? No. The old 12-point yearly cap is gone (physical study item 16; `development.rs`).

## The limit

- The rank maps: 60,000 boys from `athlete::create` on a fixed stream, fitted once per run. For each limited skill and position, his tools score is standardised against the sample there, and its 41 quantiles (every 2.5%) are mapped onto the skill ceiling's quantiles (`maps`, `RankMap`, `FIT_SAMPLE`, `Q`).
- His read: each part's ceiling, minus the class's drift for that part at his position and age, standardised against the map's mean and spread, weighted (two-thirds split evenly over the body parts, one-third over the makeup parts), then read off the map with linear steps between quantiles (`mapped`, `mapped_on`).
- The class's drift: 24,000 generator boys grown through `develop_season` from 14 to 30, with the split ceiling off for them (high school to 17, college to 21, then the pros, a coach of 10, rotation snaps). It is stored as each part's mean ceiling at each age minus its mean at 14, per position (`drift`, `Drift`, `DRIFT_SAMPLE`). It takes about 2 seconds, once per run, on its own thread pool (`off_pool`). At 22 a corner's speed ceiling has drifted about +3.5 and his play recognition about +1.0 (`prog_check drift`).
- A convert reads the position that leans on the skill most when his own doesn't use it (`map_position`).
- Each limit's own part (`Prog.over`, half points): 4, plus max(map − ceiling, 0) at the draw, plus every natural rise past the limit since (`ensure`, `settle`).
- `settle` lifts the limit only by the ceiling's own rise that year, and only past the higher of his limit when the year opened and now. So a one-year dip in his read never ratchets it up (`settle`).
- An older save draws once, from where the player is now, with no trims pending (`ensure`; `player.rs`, `#[serde(default)] prog`).

## Everyday closing

- Every year, every limited skill his position works: ceiling += room × 0.10 × pace × coach_mult × snaps_mult × stage_mult (`everyday`, `EVERYDAY`).
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

## The trims

- Design values, before the build sizes them on a world: DT power rush 2; edge finesse rush, RB pass blocking, LB zone coverage, CB man coverage and QB footwork 3.5; WR routes 4; OT and IOL pass blocking, TE run blocking and S tackling 5 (`TRIM_TABLE`).
- They are pending for every boy drawn at 15 or younger (`ensure_new`) and land the first year he plays a spot with a trim, if his ceiling there is 60+ (`TRIM_FROM`). They are dropped after 22 (`TRIM_UNTIL`). His rating comes down with the ceiling if it sat above it (`land_trims`).

## What it does not do yet

- The weight room inside the leashes, the body knack's own effect, the strength split, and the physical study items are steps 3-4 of the build.
- The screens show none of it yet (step 6).
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

- `progression.rs`: `LIMITED`, `spec`, `MARGIN` 4, `FIT_SAMPLE` 60,000, `DRIFT_SAMPLE` 24,000, `EVERYDAY` 0.10, `CLICK` (6, 10), `PROJECT` 0.30, `CLICK_CHANCE` 0.25, `PROJECT_COST` 0.25, `Room::slots`, `Lean`, `AI_SHARP` 0.6, `YOUR_SHARP` 1.0, `BET_FLOOR` 3, `TRIM_TABLE`, `TRIM_FROM` 60, `TRIM_UNTIL` 22.
- Tooling switches: `set_enabled`, `set_parts` (everyday 1, breakouts 2, projects 4, trims 8), `set_trims`. `crates/afm-tools/src/bin/prog_check.rs` (`AFM_PROG_PARTS`, `AFM_TRIM_ALL`, `fit`, `drift`).

## Related pages

- [development.md](development.md): growth toward the ceiling, breakouts, focus.
- [training.md](training.md): the focus and its minutes.
- [body.md](body.md): physical ceilings and the weight room.
- [flaws-and-tools.md](flaws-and-tools.md): the technique layer under the ratings.
