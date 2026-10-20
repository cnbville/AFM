# Getting ready for section 8 (the first playable)

Written 2026-10-20 at the end of build 9 (thread "Full build, bug check and trial 9"), from the code on `main` and trial 9 (`docs/trials/world-trial9.md`). Section 8 is not designed yet; this note lists what its build can stand on and what it still needs, so the design rounds start from facts. Everything marked "proposed" is Claude's, not confirmed.

Section 8 in the roadmap: new-game setup (team choice at any high school, college or NFL team, the name toggle, your background and editable starting stats), save and load, and plain text screens; you start playing and debugging there.

## What is ready

- **The world runs every day on its own.** `World::new` builds the 30-year history, then `advance_day`, `continue_until_needed` (stops on a decision or a big moment) and `holiday_to` drive the save (`world/mod.rs`, `world/clock.rs`). Trial 9 ran 20 seasons past 2026 without anything breaking, and its 10 plain bugs are fixed (PR #60).
- **Every AI decision has a world side to replace.** Depth charts, game plans and play calling, contracts, tags, trades, waivers, free agency, the draft board and draft-day trades, the undrafted rush, recruiting boards and offers, the portal, NIL, staff hiring and firing, captains and the room. Section 8 adds your screens on top: where you choose, the AI's choice is skipped.
- **A seat for you.** `Seat` (level, NFL team, program) decides which news stops the clock for you. It has no high school seat yet.
- **The same seed gives the same world on any machine and core count.** New test in build 9 (`a_seed_gives_the_same_world_on_any_core_count`), so a save can be rebuilt and a bug reproduced.
- **A long save keeps working.** Dates, rule books and the cap are checked to 2200 (new test).
- **Player pages exist.** The trial's player pages (cards) and the Debug Lab show everything a text screen needs; the game version hides true potential (`debug` feature off).

## What section 8 still needs

1. **Save and load.** Nothing in the engine is serialisable yet (no `serde`). Proposed: derive it for the world and write a compressed binary save (`bincode` plus `zstd`), with a version number so old saves can be upgraded.
2. **New-game setup time.** Building the 30-year history with every college game on the engine took 102 minutes on 4 cores in trial 8 and 61 minutes in trial 9, after the summer signing speed-up; a future season takes 102-110 s. Cain's M1 has 8 cores (4 fast); a guess, not measured, is 40-50 minutes there. Options to decide in design: ship prebuilt histories for a few seeds, run the history's games on the quick stand-in (much faster, but the history is then not the same football), or a shorter history. This is a decision for Raichel.
3. **Memory.** With the trial's game log on, trial 9 used 5.93 GB after the history and grew about 60 MB a season to 7.07 GB by 2045 (peak 7.62 GB; trial 8 7.77 GB). The game does not keep the game log, but the world's own size has not been measured without it. Cain's M1 has 8 GB. Proposed: measure the world without the log, set a budget (4 GB) and trim the biggest stores to it before section 8 ships.
4. **The UI technology.** Not chosen (roadmap Q49: a desktop app over the Rust engine). Proposed: Tauri (the Rust engine in-process, screens in HTML, which the player pages and the Debug Lab already are; small, runs on M1 and Intel Macs). This is a decision for Raichel.
5. **High school seat.** From "Your career at every level": high school games on the engine (about 12 more minutes a season), high school game length (83 plays a team now), a high school staff, and the school's side of the job (principal or athletic director, boosters, job security).
6. **Your own side of sections 5, 6 and 7.** Built as world sides only (`docs/design/build9-plan.md`): your talks, press, rulebook, war room, board, scouting assignments, recruiting board, hires and profile come with the screens.
7. **Your coach profile.** The points pool from your background (Cain, 2026-10-19) and your growing ratings are designed (`docs/design/people.md`, topic 15), not built.

## Open from trial 9

The trial's findings that wait for Raichel are in `docs/trials/world-trial9.md`. None of them blocks section 8's design.
