# AFM handoff (account move, 2026-10-14)

Raichel moved AFM from one Claude account to another. The old project's memory, shared files, chat and mock pages lived only on the old account, so they are copied here. With this folder and the rest of the repo, the new project has everything.

**To start the new project:** connect GitHub as `cnbville`, add this repo, and paste [starting-prompt.md](starting-prompt.md) as the project instructions or the first message.

## Where things stand

| Section | State |
|---|---|
| 1. Player model engine (Rust) | Done. |
| 2. The world | Built (`crates/afm-engine/src/world/`, `docs/world-model.md`). Trials 1-3 in `docs/trials/`. |
| 3. Match engine and stats database | Design done, engine built and merged (`game/`, `docs/match-engine.md`). Every college game, history included, runs on the engine. |
| 3b. Engine physics fixes | **Spec agreed, not built** (`docs/design/engine-physics-fixes.md`). This is the next build. |
| 4. Season calendar | **Design done, spec on main** (`docs/design/calendar.md`, all 11 topics). Not built. |
| Plays and tactics database | **Design done, spec on main** (`docs/design/plays-and-tactics.md`, all 9 topics). Not built. |
| 5 to 11 | Not started (team management, people, training, scouting/draft, first playable, game day, screens). See `docs/roadmap.md`. |

All work is merged to `main`. No branch holds unmerged work and no pull request is open (checked 2026-10-14).

## Next steps (agreed)
1. **Build the engine physics fixes, only when Raichel says so.** The five fixes are agreed: QB play, line play, depth and rotation, the RB bug, and coaching by Raichel's rule (a call wins or loses through the matchups; coach quality drives the calls the user doesn't make, plus adjustments, prep and scheme install). The checks listed in the spec are Claude's proposal and still need Raichel's OK.
2. **Then run world trial 4** on the fixed engine, refreshing both the debug dashboard and the player pages.

## Still open
- **World trial 3 findings** (`docs/trials/world-trial3.md`, `project-files/design/world-trial3-candidates.md`): dynasties (cause traced to the physics, see above), three P4 programs with no winning season, only 6 Black FBS head coaches in 2025 (real 16, cause not traced), #1 pick is a QB 45% of the time (real 73%), three-star and two-star draft rates low, too much offense and no home edge, unreal stat lines (lead RB ~92% of carries, a kicker with 94 sacks), K/P/QB too athletic, 46.5% repeated full names. All logged as candidates for Raichel's calls.
- **Section 3 leftovers:** the career stats record, and high school games on the engine.
- **Section 2 realism questions 2-7** (Q2, high school classifications and playoffs per state, was proposed and not answered).
- **Plays and tactics:** research before building (best vs worst play-caller gap, play-action and box-count splits, the real 2026 scheme mix per level) and when to build it (schemes in section 5, live calling in section 9). Both proposed, not confirmed.
- **Calendar:** chances for each off-field rule change are not set. Parked for later design: watching players' offseason work (probably 6b), and a possible NCAA coach career.

## What's in this folder
- [starting-prompt.md](starting-prompt.md): the prompt for the new project, with all of Raichel's working rules.
- [memory/](memory/): the old project's full memory, copied as-is. `MEMORY.md` is the index. Note: memory was a little behind on 2026-10-14 (it doesn't yet say the calendar and plays specs were written or that the physics spec is on main). This README is the up-to-date state.
- [project-files/](project-files/): the old shared folder, copied as-is.
  - `design/`: trial feedback, fix lists, the calendar draft, copies of the specs, the coach promotion check, and the world trial 3 candidates.
  - `trials/world-trial3.md`.
  - `code-backups/`: old bundles and patches from trials 2-5. Everything in them was later merged or replaced, so they are kept only for reference.
- [artifacts/](artifacts/): the published pages as HTML files. Open them in a browser, or republish them as artifacts on the new account.
  - `world-mockup-dashboard.html`: the mock GUI and world trial 3 debug dashboard (was https://claude.ai/artifact/HaZ7FBAwE1FGCNmCWyC4Fg).
  - `player-pages.html`: full player pages from world trial 3 (was https://claude.ai/artifact/3997w39vMdTQvdS6UJKzHR).
  - `trial6-cards.html`: the trial 6 player cards (was https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP).
  - The trial tools rebuild the data: `AFM_DASH=dash.json AFM_CARDS=cards.json cargo run --release --bin world_trial -- 2026 15`.
- [uploads/](uploads/): files Raichel uploaded to the old project (screenshots, and a consolidated world data JSON and ZIP from 2026-10-11, on the same topics as `data/real-world/`).
- [project-chat.md](project-chat.md): the full top-level project chat, oldest first.
- [threads.md](threads.md): every thread in the old project and where its result lives.
