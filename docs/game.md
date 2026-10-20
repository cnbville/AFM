# The game (section 8, first playable): how it works

Built 2026-10-20/21 overnight (thread "Build the program", PR #75), from `docs/design/first-playable.md`, `docs/design/training.md` (6b) and `docs/design/screens.md` (section 10, on Showtime).

## How to play

- **On a Mac:** download the `AFM-mac` artifact from the latest "Mac app" workflow run on GitHub (Actions tab), open the `.dmg` and drag AFM to Applications. The app isn't signed with an Apple developer ID, so the first time macOS asks: right-click AFM, choose Open, then Open again (or System Settings, Privacy and Security, Open Anyway). It is one universal app for Apple silicon (the M1) and Intel Macs.
- **In a browser** (any machine with Rust): `cargo run --release --bin afm_play`, then open http://127.0.0.1:7700. Arguments: port, the screens folder (`app/ui`), the game folder.
- **The game folder** holds `worlds/` (the library), `saves/` and `bugs/`. On a Mac it is `~/Library/Application Support/AFM`, elsewhere `~/.afm`; `AFM_HOME` overrides it.
- **Importing a world:** on the start screen, "Choose a file…" (or paste its path and press Open). An `.afmworld` file is copied into the library and opened; from then on it is listed under "The world library", and every career started from it begins on the same February morning with fresh randomness. Dropping the file into `worlds/` by hand works too.

## The parts

- `crates/afm-engine/src/save.rs`: the world file (format 1, agreed with the world-building thread): `AFMWORLD`, the version, a JSON header (seed, history years, start year, commit, written, world date), then the whole world as named MessagePack through zstd. New fields load as their default, so an older world file opens in a newer game; `World::after_load` fills in what a newer game adds.
- `crates/afm-engine/src/world/seat.rs`: your seat in the world. `start_career` (fresh randomness mixed from the career seed, you take the head coach's job and he goes to the market), your games kept whole (`SeatGame`, the last 40), your depth order (`seat_depth`, played by the engine before the staff's order), your training focus (`seat_focus`), your notes, your high school staff (`HsAssistant`), and plain reads for the screens (fixtures, records).
- `crates/afm-game`: the game. One `Game::call(cmd, args)` the screens use: `library`, `import_world`, `open_world`, `load`, `save`, `build_world`, `setup`, `jobs`, `start_career`, `status`, `continue`, `day`, `holiday`, `home`, `inbox`, `news`, `calendar`, `team`, `roster`, `depth`, `set_depth`, `training`, `set_focus`, `recruiting`, `contracts`, `schedule`, `standings`, `staff`, `coach`, `player`, `games`, `game`, `search`, `you`, `note_add`, `note_delete`, `bug`. `reads.rs` turns true ratings into your staff's reads, `views.rs` builds every screen's data, `setup.rs` the setup and the library, `files.rs` the files.
- `app/ui`: the screens, one page in plain HTML and JS on Showtime (`showtime.css`, Archivo fonts). A frosted left sidebar, the top bar with back and forward, search (⌘K), the date, what's next and Continue (space bar).
- `app/src-tauri`: the desktop app on Tauri 2. One command, `call`, runs the game on a worker thread so the window never freezes. Not in the Cargo workspace; built by `.github/workflows/mac-app.yml`.

## What a career has now

- **Setup:** your name, age, home state and heritage; 11 roads into coaching, each with a reputation, a pool of rating points and a starting spread; your 15 ratings on the 1-20 scale every coach uses; your five style sliders; any head coaching job at any NFL club, college program or high school.
- **Continue** runs day by day and stops on a decision, a huge moment anywhere, a big one at your level or for your team, and **your own game day**, which opens that game's page.
- **Screens:** home, inbox, news (everything or yours), calendar (your games, your level's fixed dates and the big moments ahead), squad (unit, class and injured filters, sortable, grouped by position), depth chart, training, recruiting (a college's offers, visits, commitments and signings; a high school's players with college interest), contracts (an NFL club's books against the cap), schedule, game pages (box score and play by play with each call and its reason), standings, staff, coach pages, team pages, player pages (overview, attributes, career, tendencies, recruiting, your notes), search, your profile, save, a bug pack.
- **What you see is your staff's read:** your own players' ratings as ranges whose width comes from the coach who reads them; other teams' players show only what is public. Potential appears only as words. Hidden coach ability is never shown, except your own ratings.
- **High school seat:** your school's games always run on the engine (even in a stand-in season) with full game pages; the schedule, your classification's standings, and a staff of teacher-coaches sized by enrollment who teach your players (below).
- **Training (6b topic 5, part):** a main focus per player. The focus grows up to half as fast again (1.5 times its share at full coachability) and his other skills of the same kind pay for it. Measured (`a_training_focus_grows_its_skill_and_costs_the_rest_a_little`, 400 players): the focus grows 1.29 times as fast and the year as a whole moves 1.2%, inside choice 2's 15%.

## Memory (step 1)

Ratings and tendencies are stored as 4-byte floats instead of 8, and the people list keeps a third spare instead of doubling. On a 4-year quick world: a person went from 2,408 to 1,752 bytes; heap 5.60 to 4.59 GB, resident 4.09 to 3.53 GB, peak 6.08 to 4.99 GB. The full 30-year measurement is in "Doubts" below.

## Not built yet (section 8's design, for the next builds)

- The choice screens where the AI still decides for your team: recruiting board and offers, the portal and NIL; contracts, the cap, free agency, trades and waivers; the draft board and war room; game plans, playbook and installs; talks; team rules; press conferences; bosses and goals; hiring and firing staff; scouting assignments.
- Training beyond the focus: the practice day and week, the time budget, a habit to work on, body plans, position moves, practice injuries, growth landing every week (choice 1) and practice snaps on the engine (choice 3).
- High school: varsity, JV and freshman squads, the principal and AD as people, the booster club, your teaching job. AI high schools still have no staff people (the built stand-in from resources stays).
- Starting without a job and assistant jobs; family and looks in the setup; the inbox's pinned answers; holiday mode on screen; faces, the 2D view and charts (section 10).

## Proposed, not confirmed

- Staff read widths: the range around the true rating narrows as the reading coach's eye rises (`reads.rs`).
- The 11 backgrounds' reputations, pools and spreads (`setup.rs`).
- High school staffs: two assistants plus one per 400 students (3 to 15), ability around what the school's resources buy, a stipend of about $2,500-8,000, 85% teach (weighted toward PE and history); a group is taught by its own coach, or by the coordinator of its side at 1.5 points less.
- The training focus's cost: each other skill of the same kind gives up 3.5% at full coachability; how much of a plan sticks rises with coachability (60% to 100%).
- Time of possession on game pages is taken from the snap clock (the engine's own total counts only live-ball seconds).
- A bug pack saves the world as it is when you report, not from the morning of that day.

## Doubts

- Storing ratings as 4-byte floats changes results per seed slightly (rounding), so a seed no longer rebuilds a pre-build world exactly. Accepted because the section 8 build changes the world anyway.
- Season records are still 8-byte; compacting old seasons (topic 8) isn't done.
- Your school's games run on the engine in a stand-in season while its opponents' other games don't; their records are mixed (the engine's scoring is close to the stand-in's odds).
- Continue stops for every huge moment anywhere (calendar N58), so a high school coach still stops on ranked college games in November.
