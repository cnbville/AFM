# The world reader's book: how it works today

Checked against main at R4's merge on 2026-10-28, with world reader lanes R4, R6 (2026-10-29) and R7 (2026-10-30).

Paths are relative to `crates/afm-reader/src/` unless they start with `crates/`, `data/` or `.github/`.

The book is world reader lane R4: one SQLite file per world (`name.afmbook`) holding every record of the world file and its logs as tables, plus everything the reader works out over them. The app (lane R5), `afm_read` and the threads all read the same numbers from it. The design and Claude's unconfirmed picks are in `docs/world-reader.md`.

## Quick answers

- How is a book built? `afm_read build <world> [log] [book]` (`bin/afm_read.rs`), which calls `make::build`: the world file, then the log folder, then the engine's tables and the real index, then the catalog, indexes, known issues, knobs, distributions, checks, consistency checks and odd things, coverage, search and hover cards, and `book_meta` last (`make.rs`, `build`). It writes `name.part` and renames it when done.
- Does it load the world into memory? No. The world file is read store by store with a streaming MessagePack reader; lists go into the book 256 records at a time (`world_in.rs`, `read`, `BATCH`). The trial 10 world (1.98 GB, 851,571 people) makes a 4.1 GB book in about 11 minutes on 4 cores (about 4 reading the world, 3.5 working out the panels) with a 0.7 GB memory peak; each step prints its time and peak (`make.rs`, `build`).
- Which world file formats does it read? Both: format 1 (one zstd stream) and format 2 (update 3's parts). In format 2 it reads the `rest` part, and where a list is written empty there (people, alumni, news, the stat book's seasons) it reads that list's pieces in table order, numbered on as one list (`world_in.rs`, `read`, `Pieces`, `items`). `book_meta.world_format` says which.
- Can it read a world from an older build? Yes for the tables: they are built from field names, not from the engine's types. True overall and potential need the engine to read each person whole; when it can't, the book says so in `book_meta.missing` and keeps the overall from the player part where that reads (`world_in.rs`, `person_extra`, `PEOPLE_UNREAD`).
- How do records become tables? Scalars become columns. Nested structs become columns joined by `__`. Lists of records become child tables named `parent__field`, with `_parent` (the parent's `_row`) and `_i` (place in the list). Maps keyed by ids become child rows with `_key`. Lists of plain values stay as JSON text (`sink.rs`, `put`, `children`, `dynamic`).
- How are the biggest log kinds kept? Packed: a few key columns, and the whole record as zstd-compressed MessagePack in `_more`, with a dictionary trained on the first records (`sink.rs`, `put_packed`, `train`). SQL reads it with `more('table', _more)` as JSON text (`book.rs`, `Book::with`).
- Which log kinds go where? Every game log stream and world log kind has a table: `log_games` (with `log_games__box`, `__players`, `__drives`), `log_plays` (packed), `log_situations`, `log_team_situations`, `log_people`, `log_events`, `log_news`, `log_whys`, `log_dropped`, and packed `log_players`, `log_exits`, `log_growth`, `log_coaches`, `log_programs`, `log_schools`, `log_clubs`, `log_snapshots`, `log_world` (`log_in.rs`, `route`, `packed_keys`).
- How exact are the numbers? Floats are kept to seven significant digits, as the world log keeps them (`json.rs`, `num`).
- What does the reader add to people? `x_overall` (the engine's `Player::overall`), `x_potential` (`debug::person_potential`) and `x_links` (his 12 Dynamic Links today, 0 to 1, from `links::links_of`); alumni get `x_potential` (`debug::alumnus_potential`) (`world_in.rs`, `person_extra`, `player_cols`, `alumnus_extra`).
- Where do real figures come from? `data/real-world/index.json` and every real data file, built into the reader (`build.rs`, `real_files`; `real_in.rs`, `read`, `figure`).
- What are panels? Pieces of SQL worked out once when the book is built and kept with their rows (`panels.rs`, `Panel`, `write_all`, `run`): tables and distributions for the topic pages, each with a kind the app draws (bars, hist, line, matrix, funnel, map, table), at most `MAX_ROWS` rows. Those that need a log the book lacks say so. The people and recruiting panels are `defs_people.rs`; the programs, high school, coaches and conference panels (and the Money page's C14, the NFL page's agents) are `defs_programs.rs`.
- What does a check or panel do when its table is missing? A child table a small or early world never filled is not in its book, so the check or panel says "no data", not "SQL failed" (`checks.rs`, `run`; `panels.rs`, `run`). Columns a newer world added and an older one lacks are added empty first (`COLUMNS` in `defs_list.rs`, `defs_people.rs` and `defs_programs.rs`; `make.rs`, `build`, `refresh`).
- Can a book's checks be worked out again without the world? Yes: `afm_read refresh <book>` reruns every check, distribution, consistency check and panel with this reader's definitions (`make.rs`, `refresh`).
- How do panels read states and events? Two small lookups the build writes: `state_codes` (a state's index and letters) and `event_bits` (a season's event bits in words) (`panels.rs`, `LOOKUPS`).
- What does the world log's yearly coach, program and school row keep as columns? Besides the packed record: a coach's job (role, position coach group, employer, since), age, ethnicity, retired, reputation, buzz, learning ceiling, mentor, record, titles, prove-it year, salary and every hidden part (`coach__ability__teaching__0` to `7`, scheme, game management and the rest); a program's names, level, conference, prestige, head coach and that season's expected wins, wins and losses; a school's name, state, class, enrollment, private, prestige, head coach and the season's wins, losses and state title (`log_in.rs`, `packed_keys`). A dotted key's number picks a list's item (`json.rs`, `get`). Year-by-year panels read these columns, not the packed record.
- Which other indexes does the build make? `_parent` on child tables, `id` and `player__id`, the log keys, and `people.hs`, `people.college`, `coaches.mentor` and `news__refs` by program, coach and club for the school, program and coach pages, the coaching trees and their news (`make.rs`, `indexes`). A child table's `_parent` has no type, so SQL that looks one up from a parent's `_row` writes `_parent = +p._row` to keep the index.
- What is `engine_call_values`? The coaches' what-beats-what table the engine compiles in (`crates/afm-engine/data/call_values.csv`): one row per concept or run and look with its mean yards (`world_in.rs`, `engine_tables`).
- Why is there an index on `people.stage`? Most people panels read one stage, and college and pro players are about a tenth of the table, so the panels step makes `ix_people_stage` before it runs them (`panels.rs`, `write_all`). Each panel's time is kept in `panels.seconds`.
- Are the real CSVs in the book? Each one under `CSV_MAX` bytes becomes its own table, `real_<name>`, one row per line (`build.rs`, `real_files`; `real_in.rs`, `read`).
- How is a check marked? By its gap from real against its tolerance: near, off or far off; "no data" when the world has no value or the book lacks the log it needs; "no real" when no real figure exists (`marks.rs`, `mark`, `gap`; `checks.rs`, `run`).
- Which checks exist? The lists are `defs_list.rs` and, for the people and recruiting pages, `defs_people.rs`, for the programs, high school and coaches pages `defs_programs.rs` (`CHECKS`, `METRICS`, `RULES`, `OUTLIERS`), each one SQL over the book's tables, so any of them can be rerun by hand with `afm_read <book> sql <query>`.
- How are known issues marked live? A fix counts for this world when the fix's commit is an ancestor of the world's commit, or, without git, when the world was written on or after the fix date (`issues.rs`, `has_fix`). The list is `crates/afm-reader/data/known_issues.json`.
- Where do the knobs come from? Every backticked constant in each how-it-works page's "Where the knobs are", with its value and its comment's "proposed, not confirmed", read when the reader is built (`build.rs`; `knobs.rs`).
- What is the coverage map? Every inventory area with the book tables its data is in, the checks and distributions over it, and a status: checked, data with no check yet, needs the log, or not stored (`coverage.rs`, `write`). The inventory is `crates/afm-reader/data/inventory.json`.
- What does a page print? The same `View` for both: `Book::text` for `afm_read` and "Copy for Claude", `Book::page` for the app as JSON (`view.rs`).
- What happens without a log? Every check that needs one says "no data" and names the log; the overview and the odd list name what is missing (`make.rs`, `build`, `missing`).
- Where is it published? `build-world.yml` builds the book after the world, adds its overview to the run's summary, compresses it with zstd and uploads it beside the world, in 1.9 GB parts when bigger than 2 GB (`.github/workflows/build-world.yml`, "Build the book").

## The tables it adds

- `book_meta(key, value)`: format, the reader's and the world's commits, world date and year, seed, years, first and last season, the log read and its streams, what is missing, the world file's header, rows, seconds and peak memory (`make.rs`).
- `catalog(tbl, source, parent, rows, ncols, cols, what)`: every table (`make.rs`; `coverage.rs`, `what`).
- `checks` and `check_years`: one row per check and one per check and year (`checks.rs`, `write_all`).
- `dist`, `hist`, `metrics`: n, mean, sd, min, p1, p5, p10, p25, p50, p75, p90, p95, p99 and max by metric, year and group; 20-bin histograms over all years (`dist.rs`, `write_all`; `stats.rs`).
- `consistency` and `odd`: the consistency checks with their failures, and the odd things at the league, one-record and data levels (`odd.rs`, `write_all`).
- `known_issues`, `knobs`, `coverage`, `coverage_stores`, `search`, `defs` (`issues.rs`, `knobs.rs`, `coverage.rs`, `search.rs`).
- `engine_rules`, `engine_dates`, `engine_attributes`, `engine_links`: the rule book and dates for every season, the attribute list, and the 12 Dynamic Links with their parts and edge (`world_in.rs`, `engine_tables`).
- `panels(id, page, area, title, kind, note, needs, open, cols, rows, n, more, says, status, sql, seconds)`, `state_codes`, `event_bits` (`panels.rs`).
- `real_<name>`: each real CSV under `CSV_MAX` bytes (`real_in.rs`).
- `real_figures`, `real_files`, `real_data` and the index's other lists (`real_in.rs`, `read`).

## Where the knobs are

- `BATCH` (`crates/afm-reader/src/world_in.rs`): records read and written together.
- `MAX_COLS`, `MAX_DEPTH`, `SAMPLES`, `SAMPLE_BYTES`, `COMMIT_EVERY` (`crates/afm-reader/src/sink.rs`): columns before the rest goes to `_extra`, child-table depth, dictionary samples and bytes, rows between commits.
- `KEEP` (`crates/afm-reader/src/odd.rs`): consistency failures kept per rule.
- `MAX_ROWS` (`crates/afm-reader/src/panels.rs`): rows a panel keeps.
- `CSV_MAX` (`crates/afm-reader/build.rs`): the biggest real CSV built in as a table.
- `FORMAT` (`crates/afm-reader/src/make.rs`): the book's format number.
- Each check's tolerance is its `tol` in `crates/afm-reader/src/defs_list.rs`.

## Related pages

- [world-log.md](world-log.md): the world log the book reads.
- [oddities.md](oddities.md): the code oddities the known-issues layer marks.
