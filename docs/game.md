# The game (section 8, first playable): how it works

Built 2026-10-20/21 overnight (thread "Build the program", PR #75), from `docs/design/first-playable.md`, `docs/design/training.md` (6b) and `docs/design/screens.md` (section 10, on Showtime).

## How to play

- **On a Mac:** download `AFM_universal.dmg` from the public downloads page, https://github.com/cnbville/afm-worlds/releases/tag/app-latest (no GitHub sign-in needed), open it and drag AFM to Applications. The app isn't signed with an Apple developer ID, so the first time macOS asks: right-click AFM, choose Open, then Open again (or System Settings, Privacy and Security, Open Anyway). It is one universal app for Apple silicon (the M1) and Intel Macs. Every build on `main` is also on the AFM repository's releases page (sign-in needed, the repository is private).
- **Updates (Cain 2026-10-21):** the app updates itself. At start and every three hours it asks the downloads page (`latest.json` in the `app-latest` release) for a newer build, downloads and installs it in the background, and restarts by itself when no career is open; with a career open it asks to save first and offers Restart now. Each build is version 0.2.<run number>. Updates are signed: the "Mac app" workflow signs them with the `TAURI_SIGNING_PRIVATE_KEY` secret and puts the matching public key (the `TAURI_UPDATER_PUBKEY` variable) into the build, and the app checks every update against it. AFM must run from Applications (not from the open dmg) to update itself.
- **Setting it up once:** the AFM repository needs one secret, `WORLDS_TOKEN`: a fine-grained token that may write contents to `cnbville/afm-worlds` and secrets and variables to `cnbville/AFM`. The "Set up updates" workflow (run once by hand) then makes the signing key inside GitHub, stores it as the `TAURI_SIGNING_PRIVATE_KEY` secret and the public key as the `TAURI_UPDATER_PUBKEY` variable; the key never leaves GitHub. Without them builds still work but make no update.
- **In a browser** (any machine with Rust): `cargo run --release --bin afm_play`, then open http://127.0.0.1:7700. Arguments: port, the screens folder (`app/ui`), the game folder.
- **The game folder** holds `worlds/` (the library), `saves/` and `bugs/`. On a Mac it is `~/Library/Application Support/AFM`, elsewhere `~/.afm`; `AFM_HOME` overrides it.
- **Getting a world (Cain 2026-10-21: both ways):** the start screen's "Get a world" lists the worlds on the worlds page, the public repository `cnbville/afm-worlds` (world files only, no game code); one click downloads a world into the library, joining its parts. The "Build a world" workflow (Actions tab, Run workflow, a seed and years) builds a world on GitHub's machines in about three hours and publishes it there; it needs the `WORLDS_TOKEN` secret, a token that may write to `cnbville/afm-worlds`. "Build a world on this Mac" builds one beside the game with its progress shown (`jobs.rs`; `afm_world <seed> <years> <folder>` does the same from the command line); a 30-year world takes hours and peaks at about 6.5 GB, tight on an 8 GB Mac.
- **Importing a world:** on the start screen, "Choose a file…" (or paste its path and press Open). An `.afmworld` file is copied into the library and opened; from then on it is listed under "The world library", and every career started from it begins on the same February morning with fresh randomness. Dropping the file into `worlds/` by hand works too. A world split into parts (`name.afmworld.001`, `.002`, ...) is imported by choosing the `.001` part, or the folder that holds the parts ("Choose a world folder…"): the game joins the parts back into one file.

## The parts

- `crates/afm-engine/src/save.rs`: the world file (format 1, agreed with the world-building thread): `AFMWORLD`, the version, a JSON header (seed, history years, start year, commit, written, world date), then the whole world as named MessagePack through zstd. New fields load as their default, so an older world file opens in a newer game; `World::after_load` fills in what a newer game adds.
- `crates/afm-engine/src/world/seat.rs`: your seat in the world. `start_career` (fresh randomness mixed from the career seed, you take the head coach's job and he goes to the market), your games kept whole (`SeatGame`, the last 40), your depth order (`seat_depth`, played by the engine before the staff's order), your training focus (`seat_focus`), your notes, your high school staff (`HsAssistant`), and plain reads for the screens (fixtures, records).
- `crates/afm-game`: the game. One `Game::call(cmd, args)` the screens use: `library`, `import_world`, `open_world`, `load`, `save`, `build_world`, `setup`, `jobs`, `start_career`, `status`, `continue`, `day`, `holiday`, `home`, `inbox`, `news`, `calendar`, `team`, `roster`, `depth`, `set_depth`, `training`, `set_focus`, `recruiting`, `contracts`, `schedule`, `standings`, `staff`, `coach`, `player`, `games`, `game`, `search`, `you`, `note_add`, `note_delete`, `bug`. `reads.rs` turns true ratings into your staff's reads, `views.rs` builds every screen's data, `setup.rs` the setup and the library, `files.rs` the files.
- `app/ui`: the screens, one page in plain HTML and JS on Showtime (`showtime.css`, Archivo fonts). A frosted left sidebar, the top bar with back and forward, search (⌘K), the date, what's next and Continue (space bar).
- `app/src-tauri`: the desktop app on Tauri 2. One command, `call`, runs the game on a worker thread so the window never freezes. Not in the Cargo workspace; built by `.github/workflows/mac-app.yml`.

## What a career has now

- **Setup:** your name, age, home state and heritage; 11 roads into coaching, each with a reputation, a pool of rating points and a starting spread; your 15 ratings on the 1-20 scale every coach uses; your five style sliders; any head coaching job at any NFL club, college program or high school.
- **Continue** runs day by day and stops on a decision, a huge moment anywhere, a big one at your level or for your team, and **your own game day**, which opens that game's page. It also stops **the morning of** your game day and of your level's decision days (NFL: the tag deadline, the new league year, the draft, cutdown day, the trade deadline; college: the early signing day, signing day, the portal opening), before the day is played; the note says which morning. The top bar names your next game.
- **The main menu** (sidebar) goes back to the start screen. Closing a career that is open asks first and says how long since you saved; Back never leaves the game.
- **Questions in the game's own dialogs:** the Mac app's window (WKWebView through Tauri) has no `prompt` or `confirm`, so saving, a bug report, closing a career and building a world ask in the page itself. Saving and a bug report show their progress, and errors come through as plain words.
- **Records:** your team's record, your own record (career and at this job) and every staff record come from the season's results. From the new football year's first day (the day after the Super Bowl) to the next kickoff, home and the team page show last season's record as "9-8 in 2025".
- **Inbox:** what is about you (your program, club or school, and you), your decisions, huge moments, the big ones you care about, the final of each of your games and your college staff's own messages, newest first. News has the same "yours" rule.
- **Depth chart:** the order the engine plays next game (your order first, then the staff's), with the injured and the suspended listed after the starters, not as starters; an NFL club's list leaves out its practice squad.
- **NFL screens:** the schedule shows each final score and the kickoff time ("1:00 ET"); the team page keeps a book of seasons (record, head coach, how far it went) from this world's next season on, and its Super Bowls. Contracts count the cap as the league does: before the season only the top 51 contracts (the rule of 51) plus dead money, with last year's carryover added to the room; players in their last year carry a "Free agent soon" tag.
- **College jobs:** the job list holds every program that plays this year and can dress a team (22 players or more), and searching by state name works ("Texas").
- **Saves** are named after the career ("Name, Team"), with the world date and when you saved. **Build a world on this Mac** asks first, with the time and memory it takes, and has a Stop button.
- **Screens:** home, inbox, news (everything or yours), calendar (your games, your level's fixed dates and the big moments ahead), squad (unit, class and injured filters, sortable, grouped by position), depth chart, training, recruiting (a college's offers, visits, commitments and signings; a high school's players with college interest), contracts (an NFL club's books against the cap), schedule, game pages (box score and play by play with each call and its reason), standings, staff, coach pages, team pages, player pages (overview, attributes, career, tendencies, recruiting, your notes), search, your profile, save, a bug pack.
- **What you see is your staff's read:** your own players' ratings as ranges whose width comes from the coach who reads them; other teams' players show only what is public. Potential appears only as words. Hidden coach ability is never shown, except your own ratings.
- **High school seat:** your school's games always run on the engine (even in a stand-in season) with full game pages; the schedule, your classification's standings, and a staff of teacher-coaches sized by enrollment who teach your players (below).
- **Training (6b topic 5, part):** a main focus per player. The focus grows up to half as fast again (1.5 times its share at full coachability) and his other skills of the same kind pay for it. Measured (`a_training_focus_grows_its_skill_and_costs_the_rest_a_little`, 400 players): the focus grows 1.29 times as fast and the year as a whole moves 1.2%, inside choice 2's 15%.

## Memory (step 1)

Ratings and tendencies are stored as 4-byte floats instead of 8, and the people list keeps a third spare instead of doubling. On a 4-year quick world: a person went from 2,408 to 1,752 bytes; heap 5.60 to 4.59 GB, resident 4.09 to 3.53 GB, peak 6.08 to 4.99 GB. 
The 30-year world for first saves (seed 2026, `afm/worlds/` in the project folder, 1.98 GB in five parts), measured in `afm_play` on a 4-core Linux machine on 2026-10-21:

| Step | Time | Resident memory |
|---|---|---|
| Import (join the five parts into the library) | 76 s | |
| Open | 51 s (73 s on the old code) | 3.32 GB (3.84 GB on the old code) |
| Start a career at Iowa | 0.2 s | 3.32 GB |
| Continue from 9 February to the first game (stops on 24 April and 2, 4 and 6 September) | 148 s | 5.10 GB, peak 5.17 GB |
| Save | 74 s | 5.10 GB |

## Not built yet (section 8's design, for the next builds)

- The choice screens where the AI still decides for your team: recruiting board and offers, the portal and NIL; contracts, the cap, free agency, trades and waivers; the draft board and war room; game plans, playbook and installs; talks; team rules; press conferences; bosses and goals; hiring and firing staff; scouting assignments.
- Training beyond the focus: the practice day and week, the time budget, a habit to work on, body plans, position moves, practice injuries, growth landing every week (choice 1) and practice snaps on the engine (choice 3).
- High school: varsity, JV and freshman squads, the principal and AD as people, the booster club, your teaching job. AI high schools still have no staff people (the built stand-in from resources stays).
- Starting without a job and assistant jobs; family and looks in the setup; the inbox's pinned answers; holiday mode on screen; faces, the 2D view and charts (section 10).

## Proposed, not confirmed

- Staff read widths: the range around the true rating narrows as the reading coach's eye rises (`reads.rs`).
- The 11 backgrounds' reputations, pools and spreads (`setup.rs`).
- Which decision days get a morning stop (the NFL's five and college's three above, `World::morning_stop`), and a sack with no rusher on the quarterback credited to the man who tackled him.
- High school staffs: two assistants plus one per 400 students (3 to 15), ability around what the school's resources buy, a stipend of about $2,500-8,000, 85% teach (weighted toward PE and history); a group is taught by its own coach, or by the coordinator of its side at 1.5 points less.
- The training focus's cost: each other skill of the same kind gives up 3.5% at full coachability; how much of a plan sticks rises with coachability (60% to 100%).
- Time of possession on game pages is taken from the snap clock (the engine's own total counts only live-ball seconds).
- A bug pack saves the world as it is when you report, not from the morning of that day.

## Doubts

- Memory in season is 5.1 GB, over the agreed 4 GB budget (opening is under it at 3.3 GB). The season's stores (game logs, the year's records) grow from the first day; compacting them is the next memory step. An 8 GB M1 runs it, with little room for other apps.
- Continue stops the morning of your game, then plays the whole day in one step and opens the game page; there is no stop at kickoff itself.

- Storing ratings as 4-byte floats changes results per seed slightly (rounding), so a seed no longer rebuilds a pre-build world exactly. Accepted because the section 8 build changes the world anyway.
- Season records are still 8-byte; compacting old seasons (topic 8) isn't done.
- Your school's games run on the engine in a stand-in season while its opponents' other games don't; their records are mixed (the engine's scoring is close to the stand-in's odds).
- Continue stops for every huge moment anywhere (calendar N58), so a high school or NFL coach still stops on ranked college rivalry games in November.
- Engine stats: a throwaway is no longer a target for any receiver, and a sack with no rusher on the quarterback (he went down to a tackle) goes to the tackler, so the box score's sacks match the play by play.
- Free-agent prices ignore age: a 37-year-old quarterback can still ask about $64M a year. A pricing change is a proposal for Cain, not built.
