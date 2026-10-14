# AFM handoff (account move, 2026-10-14)

Raichel moved AFM from one Claude account to another. The old project's chat, its 20 threads, its memory, its shared files and its published pages lived only on the old account, which is closing. This folder holds copies of all of them, plus a readable summary written from those copies.

The first version of this folder (merged 2026-10-14 12:25) said the new project "has everything". That was wrong: it did not copy the replies inside the threads, where most decisions were made. They are now copied word for word in [transcripts/](transcripts/), and the decisions, open questions and rules were rewritten from them. What still could not be copied is listed under "Not carried over" at the end.

## Starting the new project

1. On the new account, connect GitHub as `cnbville` and create a project with the repo https://github.com/cnbville/AFM.
2. Paste [project-instructions.md](project-instructions.md) into the project's instructions.
3. Send [starting-prompt.md](starting-prompt.md) as the first message. It makes the new Claude read this folder and seed the new project's memory from [memory-seed.md](memory-seed.md).
4. Optional: republish the pages in [artifacts/](artifacts/) on the new account. The old links stop working when the old account closes.

## Where things stand (2026-10-14)

| Section | State |
|---|---|
| 1. Player model engine (Rust) | Done. |
| 2. The world | Built (`crates/afm-engine/src/world/`, `docs/world-model.md`). Trials 1-3 in `docs/trials/`. |
| 3. Match engine and stats database | Design done; engine built and merged (`game/`, `docs/match-engine.md`). Every college game runs on the engine, history included. Not built: the career stats record, and high school games on the engine. |
| Engine physics fixes | **Spec agreed, not built** (`docs/design/engine-physics-fixes.md`). Raichel: "don't build". |
| 4. Season calendar | **Design done, spec on main** (`docs/design/calendar.md`, all 11 topics). Not built. |
| Plays and tactics database | **Design done, spec on main** (`docs/design/plays-and-tactics.md`, all 9 topics). Not built. |
| 5 to 11 | Not designed (team management, people, training, scouting and draft, first playable, game day, screens). See `docs/roadmap.md`. |

All work is merged to `main`. No branch holds unmerged work and no pull request is open.

## Next steps, in order

1. **Build the engine physics fixes only when Raichel says so.** The five fixes are agreed: QB play, line play, depth and rotation, the RB bug, and coaching by Raichel's rule (a call wins or loses through the matchups; coach quality drives the calls the user doesn't make, plus adjustments, prep and scheme install). The checks listed in the spec are Claude's proposal and still need Raichel's OK.
2. **Then run world trial 4** on the fixed engine (15 years, every college game on the engine), refreshing both the debug dashboard and the player pages.
3. **Then Raichel's calls** on world trial 3 findings 2-9 and the rest of [open-questions.md](open-questions.md).

## What's in this folder

The readable layer, written from the transcripts:
- [decisions.md](decisions.md): every decision Raichel made, by section and topic, in their own words, with dates, pointers into the transcripts, and superseded decisions marked.
- [open-questions.md](open-questions.md): everything still waiting on Raichel, with Claude's proposed picks; what is decided but not built; parked features; data gaps.
- [working-rules.md](working-rules.md): every working-style rule and correction from Raichel, with quotes.
- [memory-seed.md](memory-seed.md): the memories to create in the new project, up to date on 2026-10-14.
- [project-instructions.md](project-instructions.md) and [starting-prompt.md](starting-prompt.md): for setting up the new project.
- [project-settings.md](project-settings.md): the old project's settings, shared folder, uploads, pages, pull requests, branches and threads.
- [threads.md](threads.md): every thread, its state and where its result lives.
- [completeness-check.md](completeness-check.md): the thread-by-thread check that every decision and open question made it into the readable layer.

The word-for-word record:
- [transcripts/](transcripts/): the project chat (`00`) and all 20 threads (`01`-`20`), every message with its author, time and full text, oldest first. `20b` is the coordinator's brief for the handoff thread. `raw/` has the same messages as fetched (JSON).
- [raichel-messages.md](raichel-messages.md): every message Raichel wrote and every decision-card pick, oldest first, each with its transcript and the Claude message it answered.

The old project's own files:
- [memory/](memory/): the old project's memory, copied as is. `MEMORY.md` is the index. It was a little behind on 2026-10-14; `memory-seed.md` is current.
- [project-files/](project-files/): the old shared folder, byte for byte.
  - `design/`: trial feedback, fix lists, the old calendar draft, copies of specs, the coach promotion check, the stats proposal, the world trial 3 candidates.
  - `trials/world-trial3.md`.
  - `code-backups/`: old bundles and patches from trials 2-5. Everything in them was later merged or replaced, except `trial5-film-measure.ts`, a small script that was never committed.
  - `uploads/hearth/`: the 7 files Raichel attached in the chat, under their original names (no extensions).
- [uploads/](uploads/): the same 7 files with extensions, so they open on GitHub. [project-settings.md](project-settings.md) says what each one is and which message it came with.
- [artifacts/](artifacts/): the published pages as HTML. Open them in a browser, or republish them on the new account.
  - `world-mockup-dashboard.html`: the mock GUI and world trial 3 debug dashboard (was https://claude.ai/artifact/HaZ7FBAwE1FGCNmCWyC4Fg).
  - `player-pages.html`: full player pages from world trial 3 (was https://claude.ai/artifact/3997w39vMdTQvdS6UJKzHR).
  - `trial6-cards.html`: the trial 6 player cards (was https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP).
  - The trial tools rebuild the data: `AFM_DASH=dash.json AFM_CARDS=cards.json cargo run --release --bin world_trial -- 2026 15`.

## Not carried over

- **The coordinator's start-up briefs for threads 01-19.** When the coordinator started a thread, it sent that thread a private brief. Threads never show these in the chat, and the only way to read them is each session's internal event log, which can only be paged backward from the newest event and is mostly tool output. The brief for this handoff thread was copied (`transcripts/20b`). The others were not. What they carried is covered elsewhere: each brief quoted Raichel's messages, which are in the transcripts, and the coordinator's later notes to threads appear in the thread copies as "Internal note between Claude sessions". The exact wording of those briefs is lost.
- **Each session's internal tool log:** the commands, file reads and test runs behind each thread's messages. Their results are in the repo (code, docs, trial reports), but the step-by-step logs are not.
- **Earlier versions of the three published pages.** Only the latest version of each was saved.
- **Two memory bookkeeping files** (`.memory-sync`, `.memory-sync-basis`). They only track syncing and hold no project content.
- **Project settings beyond what the threads could read.** A direct settings read was not available. The name, topic, visibility, repo and shared folder are in [project-settings.md](project-settings.md); there were no project instructions.
- **The live project itself:** the chat, the threads and the sessions can't move between accounts. Their content is in `transcripts/`.
