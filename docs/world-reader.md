# The world reader's book

World reader lane R4 (spec `afm/world-reader/spec.md` in the project folder, sections 2, 4 and 6; brief `briefs/r4.md`). The book is one SQLite file per world that holds everything the reader shows. The app (lane R5), `afm_read` and any thread all read the same numbers from it. How it works fact by fact, with files and functions: `docs/reference/how-it-works/book.md`.

## What goes in

- **The world file** (`name.afmworld` or a save). It is streamed store by store, never loaded whole, so a book builds from a world written by any older build. Every store becomes tables. Lists of records become one row per record, nested structs become columns joined by `__` (`player__body__weight`), and lists inside a record become child tables (`people__player__history`, joined on `_parent = people._row`). People also get `x_overall` and `x_potential`, worked out by the engine's own debug functions.
- **The world log** (lane R2) and **the game log** (lane R1), from the `name.afmlog` folder beside the world when there is one. Each kind becomes a `log_*` table. Big kinds (plays, yearly player rows, growth, snapshots) keep a few key columns and the whole record packed in `_more`, readable in SQL as `more('log_players', _more)`.
- **The engine's own answers**: the rule book and the dates for every season (`engine_rules`, `engine_dates`), the attribute list (`engine_attributes`).
- **The real index** (`data/real-world/index.json`) and every real data file, as the reader was built.

## What gets worked out

- `checks`: every check against real. Each has the world's value, the real value with its source and years, a mark (near, off, far off, no data, no real), the tolerance, a "what it says" line in plain words, the SQL behind it and the known issues that touch it. `check_years` holds the same number year by year.
- `dist` and `hist`: distributions with percentiles (p1 to p99), by year and by group (level or division, conference, position, stars, state, age, stage, role).
- `consistency`: the checks between stores (inventory T11), such as a drafted player's college existing, or a coach not holding two head jobs at once.
- `odd`: the odd-things list at three levels. League: checks off or far off. One record: consistency failures with their facts, and rare values with their percentile against the world. Data: what the book could not count (a missing log) and the known issues still live in this world.
- `known_issues`: the 28 code oddities, the trial findings waiting for a yes or no and the parked items, each marked live or fixed for this world by its commit (inventory T1).
- `knobs`: every tuning constant the how-it-works pages name, with its value, file and whether it is confirmed (inventory T2).
- `coverage`: every one of the inventory's 207 areas with its tables, checks and a status: checked, data with no check yet, needs a log this book lacks, or stored nowhere (inventory T4). `coverage_stores` lists each store with its rows.
- `search` (every person, alumnus, coach, program, high school and NFL club by name) and `defs` (what each number is, for hover cards).
- `catalog` (every table, where it came from, its rows and columns) and `book_meta` (the world's seed, years, commits, dates, the logs read and what is missing).

## A book without a log

A book always builds from a world file alone. Checks that need a log show "no data" and say which log they need, and the overview and the odd list name what is missing.

## afm_read

- `afm_read build <world> [log folder] [book]` builds the book. The log defaults to `name.afmlog` beside the world (`-` for none) and the book to `name.afmbook`.
- `afm_read <book> [page] [key=value ...]` prints a page as short text, the same text the app's "Copy for Claude" gives. Pages: `overview`, the topic pages (`people`, `recruiting`, `programs`, `high-school`, `coaches`, `games`, `nfl`, `money`, `world`, `career`, `tools`), `checks`, `check <id>`, `odd`, `issues`, `knobs`, `coverage`, `tables`, `table <name>`, `dist <metric>`, `record <table> <row>`, `person|alumnus|coach|program|school|club <id>`, `search <text>` and `sql <query>`.
- Filters: `year=`, `from=`, `to=`, `level=`, `division=`, `conference=`, `pos=`, `stars=`, `state=`, `limit=`, `debug` (shows true potential and ceilings), and on table pages any column, `where=<sql>` and `order=<sql>`.

The library gives the same pages to the app: `Book::open`, `book.text(page, &Filters)` and `book.page(page, &Filters)` (JSON).

## On GitHub

`build-world.yml` builds the book after the world and its log, puts its overview in the run's summary, and publishes it beside the world on the worlds page as `name.afmbook.zst` (zstd), in 1.9 GB parts when it is bigger than 2 GB. With `check` set to a worlds page release, it builds a book for that world (world file only) and adds it to that release.

## Claude's picks, proposed, not confirmed

- The marks' tolerances are the Debug Lab's thresholds per check (near, off, floor for near-zero reals).
- Big log kinds are packed (key columns plus `_more`) instead of spread into columns, to keep the book small; plays, yearly player rows, growth, exits, coaches, programs, schools, clubs, snapshots and the world row.
- Floats from the world file are kept to seven significant digits, as the world log keeps them.
- The book is compressed with zstd for the worlds page and split at 1.9 GB.
- Outliers show the top 0.5% to 1% of a measure (per outlier), at most 50 rows each; consistency failures keep 200 rows each.
- Where a check has no sourced real figure yet, it uses a figure from memory and says so ("The real figure is from memory").
