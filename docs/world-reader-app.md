# The World Reader app (lane R5)

The reader's Mac app and its browser twin: the shell, the library, search, filters, the year slider, hover cards, "Copy for Claude", the hidden-truths switch, Compare, and the Overview, Tools, Your career and player pages. Spec: `afm/world-reader/spec.md` sections 1, 5, 6 and 7 in the project folder; the look is the mock-up canvas https://claude.ai/artifact/NtrNC4UTjVcqCaFKnsrjPr. The book itself (lane R4) is described in `docs/world-reader.md`.

## Where it lives
- `crates/afm-reader-app`: the commands, `Reader::call(cmd, args)`, as the game's `Game::call`. A debug tool: it links afm-engine with the `debug` feature. The game never links it.
  - `book.rs`: the book (SQLite) as the app reads it. It reads the book's own schema, so a table a later lane adds shows up without a change here: tables and columns, rows paged with filters and the year slider, search, links between tables, hover cards from `defs`.
  - `pages.rs`: every page as blocks (`hero`, `tiles`, `list`, `table`, `rows`, `kv`, `chart`, `pair`, `replay`, `links`, `note`). Overview, the topic pages, Tools, Your career, any table, and every entity's page with the player page as the template.
  - `compare.rs`: two worlds, two years, two groups, a player against real players like him.
  - `play.rs`: replaying a logged game snap by snap from the book, and playing any two programs now on the engine from the world file.
  - `text.rs`: "Copy for Claude", any page as short text.
  - `library.rs`: books on the worlds page (`cnbville/afm-worlds`, assets `name.afmbook` or parts `.001`, `.002`), books on this Mac, and the game's world files and saves.
  - `demo.rs`: a stand-in book with made-up numbers, for the tests and for looking at the screens.
  - `bin/afm_reader_web.rs`: the reader in a browser (`cargo run --release --bin afm_reader_web -- 7710 reader/ui`); `afm_reader_web demo <path>` writes the stand-in book.
- `reader/ui`: the screens (one page, plain JS, Archivo as in the game). `reader/src-tauri`: the Tauri 2 app, outside the workspace like the game's. `.github/workflows/reader-app.yml` builds the universal `.dmg`; on `main` it goes on a release `reader-<run>` (never marked latest, so the game keeps that link) and on the public downloads page as `reader-latest`, where the installed reader looks for its updates.
- The reader's folder: `~/Library/Application Support/AFM Reader` on a Mac (`~/.afm-reader` elsewhere, `AFM_READER_HOME` overrides): `books/` and `settings.json` (the switch and the last book). It reads the game's folder (`AFM_HOME` as the game does) for its worlds and saves.

## What the app reads from a book
The app finds each kind of table by name (`book.rs`, `names_for`), and each column by the first name it has:
- `meta` (key, value): `name`, `seed`, `commit`, `history_years`, `first`, `last`, `built`, `has_log`, `missing`, `summary`, `world_path`, and the options a compare checks for fairness (`scale`, `engine_from`, `hs_engine_years`, `yearly_growth`, `names`).
- `checks`: `id`, `page` or `area` (an inventory code such as G24; the page comes from the code, spec section 7), `label`, `world`, `real`, `unit`, `source`, `years`, `mark` (or worked out from the two numbers), `why`, `file`, `function`, `vital`, `link`; a `year` column makes the slider pick a year's checks; `level` and the other filter columns narrow them.
- `odd`: `text`, `why`, `mark`, `page`, `scope`, `entity` and `entity_id` (opens that row's page), `rank`.
- `issues`, `knobs`, `coverage` (`tbl`, `page`: which tables a page shows), `real`, `fitted`, `whys`, `consistency`: shown on the Tools tabs as tables.
- `defs` (`tbl`, `col`, `label`, `what`, `unit`, `file`, `function`, `source`, `years`, `debug`): every column's hover card and whether it is a hidden truth.
- `search` (`kind`, `id`, `name`, `detail`) when the book has one; otherwise every table with an `id` and a name is searched.
- Links: an `<x>_id` column opens the row of the table it names (`person_id` to `people`, `home_team_id` to `teams`, ...); every id in a table shows as its name.
- Filters apply to a table only when it has the column (`level`, `division`, `conference`, `position`, `stars`, `state`; `year` or `season` for the years). The year slider shows the world as it stood: a snapshot table's year (`as_of`), season rows up to that year, people active then (`first_year` to `last_year`).
- Hidden truths: columns marked `debug` in `defs`, or named for them (`potential`, `ceiling`, `true_*`, `hidden*`, `*_debug`), and tables named `hidden` or `debug`. With the switch off they never leave the reader.

## Measured (stand-in book, browser)
- Every page opens in under 70 ms from the stand-in book. A real book's numbers come with R4's first world.

## Picks (proposed, not confirmed)
- The marks' thresholds are the Debug Lab's: near within 5% of real, off within 15%, far off beyond (the canvas's own note).
- The app keeps the last book open between runs, and the hidden-truths switch starts on (the reader is a debug tool).
- Compare years and groups show each numeric column's mean and its 10th, 50th and 90th percentiles; a player against real players like him uses the same position and a draft slot within 8 picks.
- Playing two teams now uses an average road trip's crowd (0.65) and the world as it stands at its last save; it loads the world once (about a minute and 4 GB on the M1) and keeps it while the app runs.
- The reader's own version runs 0.1.<build>, apart from the game's 0.2.<build>, and it shares the game's update signing key.
