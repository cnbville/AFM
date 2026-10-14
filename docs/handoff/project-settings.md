# The old project's settings and inventory

What the AFM project on the old account looked like on 2026-10-14, so the new project can be set up the same way. Taken from the session context the old project gave its threads, `list_thread_sessions`, `list_project_prs`, `list_project_artifacts`, `git` and the shared folder. A direct settings read (`get_project_settings`) was not available to this thread, so anything not shown below could not be read.

## The project

| Setting | Value |
|---|---|
| Name | AFM |
| Id (old account) | `chan_015NJfb6NRUve4Wa2buMtZKs` |
| Visibility | Private: one person's project |
| Owner | Raichel (GitHub `cnbville`) |
| Repository | https://github.com/cnbville/AFM |
| Project instructions | None were set. Memory said "No project instructions set yet." A suggested set is in [project-instructions.md](project-instructions.md). |
| Created | 2026-10-10 (first message 19:43 UTC) |

**Topic (the project description), word for word:**

> Create an equivalent game to football manager 27 but then focused on American Football. Realism, stats and dynamic potential is extremely important. It should have an patient flow that doesn't rush climaxes. It should have everything from player cards, High school players that have realistic improvements upto the NFL.

## Shared folder (`/mnt/project-files`)

Copied byte for byte to [project-files/](project-files/) and checked with `cmp` on 2026-10-14. There was no `.notes/` folder.

| Path | What it is |
|---|---|
| `design/calendar.md` | Section 4 spec draft (same as `docs/design/calendar.md`) |
| `design/coach-promotion-and-team-growth-check.md` | Thread 12's check, fixes A-C |
| `design/engine-physics-fixes.md` | The physics spec (same as `docs/design/engine-physics-fixes.md`) |
| `design/open-bugs-after-trial4.md` | Player model bug list after trial 4 |
| `design/player-model-decisions.md` | The first decision list (2026-10-10) |
| `design/players30-final.json` | The 30-player trial cards |
| `design/season-calendar-and-pacing.md` | The old calendar draft (superseded) |
| `design/section3-topic8-stats-proposal.md` | The full stats proposal from section 3 topic 8 |
| `design/trial5-handoff.md`, `trial5-talk.md` | Trial 5 notes |
| `design/trial6-fix-list.md` | The 17 trial 6 fixes |
| `design/world-trial1-feedback.md` | Raichel's world trial 1 list (items 1-9) |
| `design/world-trial3-candidates.md` | Trial 2 flags and items 1-15 for trial 3 |
| `trials/world-trial3.md` | World trial 3 report |
| `code-backups/` | Old bundles and patches from trials 2-5. All of it was later merged or replaced, except `trial5-film-measure.ts`, a small measuring script that was never committed. |
| `uploads/hearth/` | The 7 files Raichel attached in the chat (no extensions). Copies with extensions are in [uploads/](uploads/). |

The uploads, matched to their messages by upload time (the match is inferred):

| File | What it is | Sent with |
|---|---|---|
| `c2625333-...png` | The Next Gen Stats pass chart Raichel gave as the highlight style | [T02] 2026-10-10 20:15 |
| `edc79438-...png` | A screenshot of Claude's list "What I wasn't sure about, so I marked it unconfirmed" | [T00] 2026-10-11 20:02 |
| `907dfe5e-...json` | Consolidated world data Raichel supplied | [T00] 2026-10-11 20:17 |
| `128d811e-...zip` | A ZIP of a real-world dataset (NFL teams, HS states, talent, rates, rules, colleges) | [T00] 2026-10-11 22:34 |
| `6fffb7ef-...png` | A player page panel: "Outlook at signing: true potential 96..." | [T11] 2026-10-13 12:08 |
| `6a354a20-...png` | An app screenshot ("Pontificating 27s", a Create PR bar) | [T17] 2026-10-14 11:19 |
| `a6d3910d-...png` | The paused-thread icon Raichel asked about | [T00] 2026-10-14 11:22 |

## Published pages (artifacts)

Saved as HTML in [artifacts/](artifacts/), copied after their last update. The links stop working when the old account closes. Earlier versions of each page were not kept.

| Page | Old link | Last updated | Saved as |
|---|---|---|---|
| AFM World Mockup (mock GUI and world trial 3 debug dashboard) | https://claude.ai/artifact/HaZ7FBAwE1FGCNmCWyC4Fg | 2026-10-14 09:06 | `world-mockup-dashboard.html` |
| AFM Player Pages (world trial 3) | https://claude.ai/artifact/3997w39vMdTQvdS6UJKzHR | 2026-10-14 09:06 | `player-pages.html` |
| AFM Trial 6 (player cards) | https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP | 2026-10-11 19:11 | `trial6-cards.html` |

## Pull requests

All 20 are merged (from `list_project_prs` and the merge commits on `main`). No pull request is open.

| PR | Title or content | Thread |
|---|---|---|
| #1 | Player potential and development model | 02 |
| #2 | Player model through trial 5, plus design docs | 04 |
| #3 | Port the player model to a Rust engine | 06 |
| #4, #5 | Full spec; six roadmap and world items confirmed | 07 |
| #6-#12 | Real-world data and its re-research | 08, 09 |
| #13-#15 | World spec and its follow-ups | 10 |
| #16-#18 | Build the world, trial 1 list, first-round calibration | 11 |
| #19 | Section 3 spec | 14 |
| #20 | Athlete-first player creation | 15 |
| (direct merges) | Match engine, trial 3 fix list, world trial 3, plays spec, physics spec, calendar spec, this handoff | 15-20 |

## Branches

Every `claude/*` branch is fully merged into `main` (0 commits ahead, checked 2026-10-14). They can stay or be deleted; nothing is lost either way.

| Branch | Last commit | Thread |
|---|---|---|
| `claude/project-thread-yn5oqf` | fed83d9 Add player potential and development model | 02 |
| `claude/project-thread-aqjblw` | e69f36b Trial 6: build the 17 fixes, league sim and 20-player trial | 04 |
| `claude/project-thread-08hzjq` | e69f36b (same commit) | 05 |
| `claude/project-thread-m1lkzk` | 7ca2f37 Port the player model to a Rust engine | 06 |
| `claude/project-thread-uamm0g` | f074ded Mark six roadmap and world items confirmed | 07 |
| `claude/project-thread-m3u53a` | 988a8f0 Fill HS transfer rules for all states and 9 D2 stadium sizes | 08 |
| `claude/project-thread-8tbsvu` | 372701d Replace hs_states.csv with 2026 programs, classes, playoffs | 09 |
| `claude/project-thread-5a50p7` | 0734492 World spec: confirm escaping to a powerhouse as a rare pull | 10 |
| `claude/project-thread-w94grb` | e9e2aa1 Draft: calibrate the whole first round by position and team need | 11 |
| `claude/project-thread-ey247o` | 3ce1f1e Add the section 3 spec | 14 |
| `claude/section3-match-engine-7n0c6b` | 6e022d9 World trial: debug dashboard export and engine season totals | 15 |
| `claude/project-thread-ata5eh` | 79dc9bb Mark the engine physics spec as the next needed fix | 17 |
| `claude/project-thread-4vtifz` | 512e24b Plays and tactics database: design spec | 18 |
| `claude/project-thread-b4cxtj` | 5f92598 Add section 4 calendar spec | 19 |
| `claude/project-thread-r04fgx` | this handoff | 20 |

## Threads

Each thread's full text is in [transcripts/](transcripts/). State as `list_thread_sessions` reported it on 2026-10-14.

| # | Thread | Session | State |
|---|---|---|---|
| 01 | Map AFM repo toward the football sim goal | cse_01LEmQd3y4SoycFTYBQdeC9p | Done |
| 02 | Build the player potential and development model | cse_01JQgqp7rz4kKMexyFjLRnjv | Resolved |
| 03 | Design the season calendar and pacing | cse_01RA5nJwD1qWN1TYVxrCxFXS | Done (superseded) |
| 04 | Trial 5 league sim | cse_01MRmdzVaLb3F8SVDEtFUqDN | Resolved |
| 05 | Trial 6 final build | cse_015VBbu8bJa8eYssvn6d6xbK | Done |
| 06 | Rewrite engine in Rust | cse_014avbKjX5MZyt6rEcJEuDLK | Resolved |
| 07 | Full spec to GitHub | cse_01UvYthR9yCcyy9LS6E2Me8Q | Resolved |
| 08 | Real-world dataset check | cse_01USAwRncXf1VifrAUUjGRd6 | Resolved |
| 09 | Real-world data research | cse_01AKu4Y6KCzjTCv1ynuCurNc | Resolved |
| 10 | World spec to GitHub | cse_01Shai4HHz5vgR31uQEzBjww | Resolved |
| 11 | Build the world | cse_01G96jrN3B94YnYnEskn3SXt | Done |
| 12 | Coach promotions and team growth check | cse_01Jcd8Wnq8Y6W5Uib2KoFt8i | Resolved |
| 13 | Section 3 match engine design | cse_01Mc5UM89eCWAE54iWkDnDZP | Resolved |
| 14 | Match engine spec to GitHub | cse_01AGBAH3WquE9zvkr1FWVYBU | Resolved |
| 15 | Build the match engine | cse_01SAgLuQuUWcRWjKkDYiAk5j | Resolved |
| 16 | Section 4 calendar design | cse_01627ATntpEZsEZcbL9TfbGL | Resolved |
| 17 | World Trial 3 | cse_0122q4bNMhujTxyEguVeMSG4 | Waiting on Raichel (the physics build and the findings) |
| 18 | Plays and tactics design | cse_01N2HSUVCp41h5PV8EdmyGzR | Done |
| 19 | Calendar spec to GitHub | cse_01NavLXQkEuePuqbPixU1NWg | Resolved |
| 20 | Account move handoff | cse_01Ex3afWzMLjJnEsVVmWTTUp | This thread |

The coordinator (the session that ran the project chat) was `session_018ttM73RpNgEsKygeTj4pUK`.

[T00]: transcripts/00-project-chat.md
[T02]: transcripts/02-build-the-player-potential-and-development-model.md
[T11]: transcripts/11-build-the-world.md
[T17]: transcripts/17-world-trial-3.md
