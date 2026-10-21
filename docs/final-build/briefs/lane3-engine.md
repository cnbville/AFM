# Lane 3: Engine physics

Read first: `CLAUDE.md`, `../plan.md`, `../list.md` (D1-D4, A5, A7-A10), `docs/design/dynamic-links.md` (main spec), `docs/match-engine.md`, `docs/design/match-engine.md`, `docs/design/attributes.md`, `docs/trials/world-trial10.md` (HS findings), `docs/research/engine-and-tactics.md`. Work in progress: `claude/final-wip-lane3` (D1/D2 started).

- D1: contested catches count both men's size (reach = height + arm + vertical at the catch point; today `game/scrimmage.rs` `fight` ignores size and the defender's reach skips arms).
- D2: the QB sees a size mismatch, so a covered big receiver can be "open" for a jump ball or back shoulder (today `window()` reads separation only).
- D3: staffs value uncoachable traits (today `positions.rs` `overall_for` is a linear sum and size counts only as a scout bias; recruiting stars are 0.25 film + 0.75 projection). Evaluation, stars and draft value see size, speed and gifts the way real staffs do.
- D4: the Dynamic Links as written in dynamic-links.md: 12 stackable physical gift links, each a real, modest edge in its own situations, never a big bonus, no new limits; drawn at creation on their own random stream; in the player page data (`views.rs` player) with a small block on the existing player screen (Lane 9 owns its layout).
- A7: HS scoring 15.5 a team game is too low: tune HS offense toward real points and yards a play (real HS data in `docs/research/` or `data/real-world/`). A8: offenses slide protection, chip and double the best rusher (top HS rusher had 86.5 sacks). A9: targets lean toward the best receiver (top had only 1,134 yards). A10: measure HS interceptions after A7/A9 and tune.
- A5: a lower-division player's huge numbers (the D3 DT with 34 sacks) must come from ratings that are truly elite for his level, never an engine quirk; find the cause and fix it if the ratings don't back it. Cain loves the outlier if it's earned.
- Every high school game in a live save runs on the engine (today only your school's; `docs/game.md` Doubts; `world/hs_year.rs`), within the Friday night budget (a whole Friday under about a minute on 4 cores). Speed-ups keep results per seed.

`match_check` must stay in the real bands for NFL and FBS (`match_check -- nfl 400`, `fbs 400`, `hs 400`, before and after). Parked tuning (NFL 26.5 pts, 7.9 ypa, FBS 24.5) is not yours: report it only. Update `docs/match-engine.md` with each change, its real target and the measured result.
