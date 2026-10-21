# The final build: lanes and shared rules (2026-10-21/22, overnight)

Cain's brief (2026-10-21 22:47): "this is the final build ... it has to be the finished outlay, product ... you'll fix bugs as you go, and polish. Previously getting 95% was acceptable but not anymore. You need to think intuitive, think about aesthetics. Make it a AAA game ... don't just focus on code, focus on making it a good game." No okays are possible overnight: every fork is decided by the build, picking the most realistic option, and logged.

Scope: everything in `list.md` (this folder), plus the unfinished-sweep picks (coordinator, 2026-10-21 22:48): build 9's named "later" leftovers are in scope (`docs/design/build9-plan.md`), real poll and award voters replace the placeholders, every high school game in a live save runs on the engine within the Friday budget, f/g/h as picked (check the NFL draft withdrawal date against a source; the ~290 proposed numbers stay as starting values; fix stale docs). Parked tuning (trial 7's six, NFL 7.9 ypa, FBS 24.5, NFL 26.5 points) is NOT tuned in this build; its numbers are logged on the fresh world.

Supporting files here: `taking-over.md` (T1-T23), `game-view.md` (A14 nicknames and the career story page), `first-play-notes.md` (Cain's play notes: inbox 10a-10i, Home 9a-9e), `play-bugs.md`.

## How the lanes divide (vertical slices)

The agreed order (foundations, engine, world, screens, gated visuals) is kept for merging. To use the night, each lane owns a feature end to end (engine, game command, screen), and lanes run side by side. A lane that needs another lane's field rebases on `main` once that lane merges.

| Lane | Owns |
|---|---|
| 1 Foundations | A1 birth dates (Sept-Aug classes), A2 held back / reclassified (~1 in 10), Aura (F15, X6, F16), L2 events store the ids they name, memory compaction (in-season under 4 GB), save fields and `after_load` for old worlds |
| 2 Game day | Section 9 whole (`docs/design/game-day.md`): G1 snap-by-snap pause, G2 officials' mistakes and replay challenges, G3 coin toss, G4 tape hints, G5 realism/learning settings, opponent report, game plan, game morning, between plays, in-game decisions, injuries, sideline, halftime, delegated games, after the game, play-calling rating; the game page (scoreboard hero with win probability; Key plays, Advanced, Drive chart, Snaps, Charts tabs) |
| 3 Engine physics | Dynamic Links D1-D4 (`dynamic-links.md`), HS tuning A7-A10, A5 check (lower-division stars' lines come from ratings), every HS game in a live save on the engine within the Friday budget |
| 4 Money and leagues | X1 finances at every level (`later-extras.md` topic 1), P1 NFLPA report cards, A4 UCF stadium, conference moves 1a/1b, realignment by media money, C8 betting lines; X7 other leagues on the engine with their rule books, the Canadian pool, their jobs in the carousel |
| 5 World fixes | A3 long college roads, A6 and A11 levelers, A12 5-star draft rate, A13 walk-ons, U-d HS playoff formats per state, U-e town weather, S11 5-in-5 eligibility, real poll and award voters, draft withdrawal date, stale docs (h) |
| 6 People | Build 9 people leftovers (friends, mentors, family ties, full morale, pros' life events, courts and appeals, AI team rules, assistants as people and AI HS staffs, staff contracts, agents and permission rules, bosses as people, coaches' families and health), takeover T1-T23 and T-B, network N1-N7, P2-P4, P8; the screens: talks, press conferences, team rules, meetings with your boss and his goals, Contacts |
| 7 Scouting and draft | Build 9 scouting leftovers (personnel staffs and public evaluators, minicamp tryouts, area recruiters and camps, character and medical reads, potential words, look-backs, pro scouting, portal and JUCO evaluation, the HS coach's side), X5 (pre-draft choices, smokescreens, mocks, supplemental draft, chemistry, social media); screens: scouting file (7 tabs), department, draft board and war room, the live draft room |
| 8 Stories | Creative C1-C17, P5-P7, A14 earned nicknames and the career story page |
| 9 Screens core | Inbox 10a-10i, Home 9a-9e, linking L1-L8 (topic 17) on every screen, news feeds, search S6, tables everywhere (S8 and the table list), moving around (S10, right-click, pinned items, ⌘[ ⌘]), staff read hovers, player page S1-S3 and its tabs, standings brackets, history (leaderboards, record books, Hall of Fame, awards, carousel, drafts by year), big moments, holiday mode, Settings |
| 10 Front office screens | NFL: negotiating, tags, free agency, trades, waivers, cap sheet by year; college: recruiting board, offers, visits, portal, NIL; staff hiring and firing, coordinators' proposals; HS seat (varsity/JV/freshman, principal, AD, boosters, teaching job, school and feeder page); setup leftovers (start without a job, assistant jobs, people you know, fictional names, family and looks read by the world) |
| 11 Training and plans | Training 6b whole (`training.md`), L8 Development tab, game plan, call sheet, playbook, installs, play designer screens, squad six views and Columns, the depth chart formation board |
| 12 Faces | F3-F12, F14 (drawn heads behind a switch, comparison picture vs the approved v7 ten), X4 photo folder import, S4 silhouettes |
| 13 Game views | X3 2D view (dots default, figures setting), X2 3D coaches' view behind a switch with a clip, X8 film-room tools, the game page's Watch tab |

## Shared rules for every lane

- Read `CLAUDE.md`, `docs/handoff/working-rules.md` (skim), this file, `list.md`, and every spec your lane names before coding. Real life is the default. Outcomes come from attributes and battles. Potential and hidden ratings never reach the game crate.
- Polish rule: everything your lane owns is built and working on screen. No "later", no stubs, no placeholder text. If something truly can't be finished, say exactly what in your report.
- Rule 0 and topic 17 (`docs/design/screens.md`): screens full, dense, labelled, calm and luxurious, on Showtime (`ui/showtime/`, `app/ui/showtime.css`). Every noun and number with a story is a link. Each screen has a "Go to" line.
- Code layout, to keep lanes from colliding: new game commands go in a new file `crates/afm-game/src/<area>.rs`, registered with one line each in `Game::call` (`lib.rs`). New screens go in `app/ui/screens/<area>.js` (loaded by `app/ui/index.html`, registering `screens.<name>`), styles in `app/ui/screens/<area>.css`. Use the helpers in `app.js` (`api`, `go`, `frame`, `table`, `playerLink`, `teamLink`, `range`). Touch shared files only where needed.
- Saves: every new stored field is `#[serde(default)]`; an old world file must still open (`World::after_load` fills what's new). New randomness runs on its own stream, so existing results per seed don't move unless the change is an agreed fix.
- Memory gate: in-season memory must not grow. No per-snap or per-game stores kept for the whole world; keep stores compact (u8/u16/f32, ids not strings).
- Checks before a PR: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and your lane's tests (add tests for new mechanics). The full suite takes ~25 min; run the tests your change touches, CI runs the rest.
- Docs: update `docs/game.md`, `docs/world-model.md` or `docs/match-engine.md` with how your part works, in plain short sentences. Forks you decided go under "Decided in the final build (Claude's pick)" with one line on what you weighed.
- Git: rebase on the latest `origin/main` before opening the PR and fix any conflict. Open one PR to `main` per lane (several PRs are fine if the lane is big). Do not merge; the build thread merges when CI is green.
