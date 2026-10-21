# Lane 2: Game day (section 9)

Read first: `CLAUDE.md`, `../plan.md` (your files: `crates/afm-game/src/lane_gameday.rs`, `app/ui/screens/gameday.js/.css`), `../list.md` (G1-G5), `docs/design/game-day.md` (your spec, all 12 topics, with Cain's answers), `docs/design/screens.md` (look, rule 0, topic 17, game pages), `docs/match-engine.md`, `docs/game.md`, `docs/design/plays-and-tactics.md`.

You own, end to end (engine, world, commands, screens):
- G1: the engine pauses between snaps for your call. Today `game::play_game` runs a whole game in one loop (`game/flow.rs`, `game/sim.rs`). Make the game a resumable state machine: start, step one snap (with your call or the staff's), finish; identical results to the one-loop run when every call is delegated (prove it with a test). Continue stops at kickoff of your game, then game day opens.
- G2: officials make real mistakes (missed and phantom calls at real rates by crew quality), so replay challenges matter; challenges and booth reviews by each level's real rules and eras.
- G3: a real coin toss choice (receive, defer, kick, a goal, wind).
- G4: the tape's hints (advance scouting: film and staff tags plus hints of its own).
- G5: the realism and learning settings (which snaps stop for you, time for a call), as Cain chose.
- Everything else in game-day.md: opponent report, game plan, game morning, between plays, in-game decisions (4th downs, 2-pt, timeouts, clock), injuries, the sideline, halftime adjustments, delegated games (your own ratings call the snaps you don't take), after the game, your play-calling rating and replays.
- The game page: scoreboard hero with win probability; Key plays, Advanced, Drive chart, Snaps, Charts tabs, plus the live play-calling screen. Calm, dense, luxurious, every name a link, fast with mouse and keyboard.
- Expose each snap's frames per step so Lane 13 (2D/3D views) can plug in; don't build those views.

Fully delegated games keep identical results per seed; agreed new mechanics (officials' mistakes) run on their own random stream. No whole-world per-snap stores. Saving mid-game must work, or saving is offered only between games: pick the more polished behaviour you can finish and say which. Run `cargo run --release --bin match_check -- nfl 200` before and after. Play a whole game in the browser by calling plays and fix what feels wrong. Update `docs/game.md` and `docs/match-engine.md`.
