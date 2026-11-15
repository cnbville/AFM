# Engine fix list (waiting until after the world builds)

Items for the match engine that Cain wants kept on a list, not built yet. Nothing here changes results until he says go, and nothing is touched before the next world builds are done (his call, 2026-11-14).

## E1. The engine reads leg drive for offensive and defensive linemen
Cain, 2026-11-14, after the position-aware attribute hover (PR #383) showed that the engine reads leg drive only for the ball carrier's wrap-up fight (`game/sim.rs`, `fight_tick`, 0.005 a point of resistance). For OT and IOL (run block family) and EDGE and DT (power family) it counts toward the overall but nothing in a game uses it. The hover says so plainly until this is built.

To settle when it is picked up (proposed, not confirmed):
- Where a lineman's leg drive would act: the run block's push behind a drive block (`block_power`), the double team and the bull rush.
- Sizes measured with `ab_check` against the real slopes, like the other skill tests.

## Other skills the hover marks as not read yet
Found with the same hover (`lane_help/roles.rs`, `UNREAD`), from the 800-game read tally behind the Attributes page lists (`/mnt/project-files/attributes-page/default-lists.md`). Listed here so they are not lost; none is a decision.
- Ball tracking for corners and safeties.
- Decisions for linebackers and safeties.
- Anticipation for receivers and tight ends.
- Motor, play recognition and communication for offensive tackles; leg drive, motor and play recognition for guards and centers.
- A punter's kick trajectory, kick power, short kick accuracy and ball handling.
