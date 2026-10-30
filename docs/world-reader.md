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

## People and recruiting (lane R6)

World reader lane R6 (brief `briefs/r6.md`) fills the People and Recruiting pages: inventory areas P1-P32, T8, C18, C28-C41 and T7. Each area has the four layers: a verdict line, checks against real as tiles, panels (tables and distributions), and a way down to rows and pages.

- **Panels** (`panels`): 193 pieces of SQL worked out when the book is built, each a kind the app draws: `bars` (world against real by label), `hist` (a spread against a real or design curve), `line` (by year, or by age or rating band, with the real band shaded), `matrix`, `funnel`, `map` (by state) and `table` (rows that open pages). A `grp` column becomes buttons, a `year` column follows the year slider, `says` gives the verdict line. A panel keeps at most 5,000 rows.
- **Checks**: 45 more against real (birth months, held-back boys, draft age, heritage, lefties, combine invites, overall by level, peak ages, slumps and breakouts, career length, missed games, in-state rosters, signees and picks by state, stars, decommitments, the portal, NIL bands, five-star spread, births, trouble), and 4 consistency rules (ratings above their ceilings, roster caps, repeated note lines, All-Pros listed as backups).
- **Class and state pages**: `class <year>` and `state <XX>` pages are the panels filed under them, cut to that one class or state, with their own verdict line.
- **Dynamic Links**: the book works out each person's 12 links with the engine's own `links_of` (`people.x_links`, 0 to 1 in `engine_links` order), since the world stores none.
- **Small or early worlds**: a child table such a world never filled (no five-stars yet, no grudges) makes its panels and checks "no data", not an error. A newer world's columns that an older one lacks (birth dates, followers) are added empty.
- `afm_read refresh <book>` works out the checks and panels again with this reader's definitions, without reading the world again.
- **Time and memory**: on the trial 10 world the panels take about 3.5 minutes of the book's 11 and stay under the world read's 0.7 GB memory peak; `afm_read refresh` takes under 5 minutes at 0.36 GB. Each panel's time is in `panels.seconds`.

## Programs, high school and coaches (lane R7)

World reader lane R7 (brief `briefs/r7.md`) fills the Programs, High school and Coaches pages: inventory areas C1-C17, C19-C27, C42, T13 and S1-S34, with R6's four layers. The definitions are in `defs_programs.rs`.

- **Panels**: 204 more. Programs (62): programs by level against the real count, every program and its story, prestige and its spread by year, the 12 grades, points and win rates by level, droughts, champions and how many different ones, conference titles and strength, crowds and game days, rivalries, polls, bowls and playoffs, roster limits and NCAA cases, long roads and admissions, rosters by position, the strongest 22s, draft picks by program and conference, school records, news, moves between levels, NIL collectives. High school (41): schools per state against the real count, the real top five of each state, powerhouses and the national ones, prestige and money, champions and brackets, streaks, all-state picks, spring ball, coaches' years and titles, how often a school changes head coach, transfers and the sit-out rule, points by state, season leaders and the state record books, spending. Coaches (88): age, heritage and where each came up, the hidden parts by level and age (debug), personality against moves and health, reputation against true value, buzz, how long jobs last and how they end, the carousel against real, every search and review with its why, trees, unit records, poor coaching, systems and what teams run, play callers, each team's calls, the engine's what-beats-what table, ideas spreading, pay and contracts, NFL staffs, new staff, support staff, retirements and health, ties to high schools, the Hall of Fame, other leagues, the rule books, prep and rust, agents. Money (4) and NFL (2) pages get the collectives and NFL staffs.
- **Checks**: 22 more against real (FBS programs, champions unbeaten and from outside the power conferences, conference titles in 15 years, points, postseason games, crowds, quarterbacks a roster, colleges with a pick, FCS moving up, high schools, state champions, the longest streak, high school season records, new FBS head coaches, tenure, second FBS jobs, NFL head coach changes and tenure) and 9 consistency rules (college and high school wins equal losses each season, grades within 55-99, every national title has its news, each school's title count matches its seasons, one champion per class, a coach's head jobs never overlap by more than a season, his college record equals his program seasons, and his age counts from his birth date).
- **Conference pages**: `conference <name>` is the panels filed under it (its line, members, standings, champions, titles by member, members by season, strength), cut to that conference, with its own verdict line.
- **Packed log columns**: the coaches, programs and schools log rows now keep the columns these pages read (a coach's role, employer, reputation, retirement and hidden parts; a program's expected and real wins; a school's prestige), so a career reads without unpacking.
- **Indexes**: `people(hs)`, `people(college)`, `coaches(mentor)` and `news__refs` by program, coach and club. Child tables' `_parent` is untyped, so a lookup against a row number is written `x._parent = +p._row` to use the index.
- **The engine's what-beats-what table**: `engine_call_values` (concept, look, yards) is `crates/afm-engine/data/call_values.csv`, read when the book is built.
- **Not stored**: S15 (head coach guidelines) and S33 (road marks) are worked out by the engine and kept nowhere; the coverage page says so.
- **Time and memory**: on the trial 10 world (no logs) the book takes 7 min 10 s (5 min 27 s before this lane), 4.11 GB, at a 0.72 GB memory peak; this lane's panels and checks take about 75 s of it. `afm_read refresh` takes 3 min 16 s.

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

### Lane R6's picks, proposed, not confirmed

- A season row's year comes from his birth date; in a world without birth dates, from his class: his last high school season was the fall before his class year, which places a held-back boy right. Without either, his class less 18 plus his age.
- Draft rates by stars divide each class's draft picks by the year's signing class counts (`signing_classes`), for classes whose draft years have all passed.
- Chemistry is not stored, so the reader rebuilds it: each FBS program's best quarterback with its three best receivers and tight ends by true overall, seasons together from the staff rows before the world's year plus half for this season, read through the engine's curve 1 - e^(-0.55 x seasons).
- Morale's pull on growth is checked among college starters only (role drives both mood and growth), with today's mood.
- Conduct cases by kind are read from the words of career-note lines (DUI, assault, domestic, drugs, weapons, theft, PEDs, gambling, dismissed).
- Keyed tables keep the top rows per key: the class page's 60 best 4- and 5-stars, the state page's 40 strongest pipelines.
- Where a check's real figure is the design's target and not a sourced number (college trouble 3%, high school 1.2%, morale's 15%, specialists' stars), the source says "design".

### Lane R7's picks, proposed, not confirmed

- How long a head coach lasts is counted from runs of the season's coach in the program and NFL season rows, not from his job records, whose years overlap a season at each end (and by two after a move before fall camp: minor bug 128).
- A coach's college record is checked against his program seasons, leaving out men fired during a season and allowing the one bowl game a December hire may coach.
- A retired coach's age stops in the world file, so ages by year count from his birth date, else from the year he was last aged.
- A power program by era is the engine's own list (`world/voters.rs` power_conf) plus Notre Dame.
- The real number of high school champions a year (350) is counted from `hs_playoff_formats.csv`: each state's classes, doubled where public and private schools crown their own, plus open divisions.
- Where only a design target exists (the engine's caller swing, a coach's value weights), it shows as a design column, never as real.
- High school season records (sacks about 40, interceptions 12) and the longest streak (151, De La Salle) are from memory, as trial 10 had them.
- A national powerhouse's out-of-state players are counted by home state: a boarder keeps his home, a family that moves takes the new state. The moves the career notes name show where the rest came from.
- The roster against the limit is a panel, not a rule: the ledger counts the roster on signing day, and real programs trim to the House settlement's 105 before the season.
