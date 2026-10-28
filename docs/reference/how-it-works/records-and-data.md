# Records and the Data section: how it works today

Checked against main at d41bed5 with update 3 lane L20 on 2026-10-28.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`. "Proposed, not confirmed" numbers are Claude's; the code says which.

## Quick answers

- Where are the record books? `w.stats.books`, keyed by (scope, book): a college division, a conference, a state's high schools or the NFL, and a list kind (`world/records.rs`, `Scope`, `Book`).
- How deep is each list? A division's, the NFL's and a state's best 25; a conference's 10 (`world/records.rs`, `DIVISION_TOP`, `CONFERENCE_TOP`).
- What does a team's own book keep? Career, season and single-game marks in the 13 record stats: a college program's and an NFL club's best 10, a high school's best 3 (`world/marks.rs`, `Tier::team_top`, `Marks`).
- Do bowls and playoffs count? College counts its bowls and playoff games, high school its playoffs; the NFL counts the regular season only (`world/marks.rs`, module notes; `nfl_game` skips once `NflSeason::booked`).
- When do single-game marks go in? After every engine game: college in `world/college.rs` (after the streaks), the NFL in `world/pro/games.rs`, `record`, high school in `world/hs_year.rs`'s game loop (`world/marks.rs`, `college_game`, `nfl_game`, `hs_game`).
- What is the era column? 100 is what an average regular at his position and level would have done in the same games: his number over the season's per-game norm times his games (`world/marks.rs`, `era`; college's norms `w.stats.norms`, the NFL's and the states' `w.marks.norms`, `norms_of`).
- Who counts as a regular for the norms? NFL: 8+ games and 15+ snaps a game; high school: 5+ games and 15+ snaps a game (`world/marks.rs`, `norms_of`, `nfl_close`, `hs_norms`).
- What are the rate minimums? NFL seasons: 14 attempts, 6.25 carries, 1.875 catches and one field goal try a team game; careers 1,500 attempts, 750 carries, 200 catches, 100 field goals. College and high school seasons use 15 attempts a game and the NFL's other per-game marks; careers 325 completions, 300 carries, 100 catches, 40 field goal tries (high school half). The college and high school numbers are proposed, not confirmed (`world/marks.rs`, `season_qualifies`, `career_qualifies`).
- Is college's passer rating the NFL's? No: the rate book uses the NCAA's passing efficiency (`world/analytics.rs`, `passing_efficiency`; `world/marks.rs`, `rate`).
- When does a record count as fallen? When a game takes first place from someone else, or a season or career running total passes the holder on the day, once a level's books have five seasons behind them; each list and player is told once a season (`world/marks.rs`, `tell`, `check_running`, `Marks::established`).
- Are a high school's own fallen records told? No, only the state's (`world/marks.rs`, `game_marks`, `check_running`).
- How long are fallen records kept? This season and last (`world/marks.rs`, `end_season`, `hs_new_season`).
- What does "chasing now" show? This season's paces at least three games in that reach nine tenths of the season record, and careers still being played within a quarter of the career record (proposed, not confirmed; `world/marks.rs`, `chasing`).
- Where do a departed player's names come from? The people still in the world, then the store's own names (kept before `World::compact` drops anyone a team book or analytics row still needs), then the stat book's honors (`world/marks.rs`, `name_of`, `keep_names`).
- Does any of this change the games? No. The store only reads games and seasons; nothing in the simulation reads it (`world/marks.rs`, module notes).

## The level books

- College: season, career and streak lists per division since before lane L20; single games, rate seasons and rate careers added (`world/records.rs`, `record_books`; `world/marks.rs`, `college_game`, `college_close`, `season_rates`, `career_rates`).
- Conferences: seasons and single games, no careers (`world/marks.rs`, `college_scopes`).
- The NFL: seasons, careers and single games of the regular season, rate records, each year's leader and runner-up, filled at Black Monday after the league's own record book (`world/pro/honors.rs`, `season_records`; `world/marks.rs`, `nfl_close`).
- High school: each state's season and career lists (seasons in the state), single games, rate records and each year's leader, filed when the state's season ends (`world/hs_year.rs`, `file_lines`; `world/marks.rs`, `hs_close`).
- An entry keeps its value, holder, team, season (a career's first and last), games, position, era and, for a single game, the date, opponent, home or away and score (`world/records.rs`, `RecordEntry`, `GameRef`).
- Entries filed before lane L20 carry no games, position or era; a state's old entries have the school in the name and no team.

## Team books

- Kept per team in `w.marks.college`, `nfl` and `hs`, each sorted by list (kind and stat), best first (`world/marks.rs`, `Mark`, `team_list`, `place_mark`).
- A career mark is his total with that team; a career still being played is marked live, and the mark clears when he leaves (`world/marks.rs`, `refresh_live`, `refresh_live_schools`).
- An NFL club's career totals are kept per player and club in `club_careers`; they go when he leaves football (`world/marks.rs`, `nfl_close`, `keep_names`).

## Leaders

- College's leader and runner-up each year are read from the stat book, which keeps every season (`crates/afm-game/src/lane_data/books.rs`, `leaders`).
- The NFL's and each state's are kept in `w.marks.leaders`, since those seasons' lines don't all stay in the world (`world/marks.rs`, `leaders_of`).

## Analytics

- The position groups and their cards: QB, RB, WR, TE, OL, DL, EDGE, LB, CB, S, K, P; the long snapper has none (`world/analytics.rs`, `GROUPS`, `group_of`).
- Two cards per group: the charted one (EPA, CPOE, pressures, separation, trench wins, coverage) and the box score one (`world/analytics.rs`, `charted`, `boxed`).
- Who sees the charted card: every seat for the NFL (league-wide tracking), a college seat for its own division, an NFL seat for FBS and FCS; high school is box scores everywhere. The NFL seat's college charting and every seat's NFL tracking are proposed, not confirmed (`crates/afm-game/src/lane_data/mod.rs`, `charted`, `sources`).
- A season qualifies on its group's volume: dropbacks, carries, routes, pass-block snaps, pass rushes, defensive snaps, coverage snaps, field goal tries, punts; the default is about two thirds of a starter's (NFL 1.35 times), cut to the share of the season played while it runs (`world/analytics.rs`, `default_min`; `crates/afm-game/src/lane_data/analytics.rs`, `min_for`).
- Percentiles are among that season's qualifiers at the level (a division, the NFL, a state), turned so 100 is always the best (`crates/afm-game/src/lane_data/analytics.rs`, `percentiles`).
- The NFL keeps each regular season's charted card values for every player in `w.marks.pro_rows`, so past seasons outlive the players (`world/marks.rs`, `ProRow`, `nfl_close`).
- Seasons like his: every other season's qualifiers at the level, each read against its own field; closeness is the mean gap in percentiles over the numbers both have (at least half the card), shown as 100 minus the gap; the best five, one season per player (proposed, not confirmed; `crates/afm-game/src/lane_data/analytics.rs`, `card`).

## History's tabs

- Conference champions: each program season with `conference_title`, named by the conference it won that season (`world/program.rs`, `ProgramSeason::conference_won`; older seasons name today's conference).
- State champions: each school season with `state_title`, named by the bracket it won (`world/high_school.rs`, `HsSeason::title`; older seasons name the school's class today).
- Every winner of one award: the college honors and the NFL's awards (`crates/afm-game/src/lane_data/champs.rs`, `award`).

## Where the knobs are

- `world/records.rs`: `DIVISION_TOP`, `CONFERENCE_TOP`, `RECORD_STATS`.
- `world/marks.rs`: `Tier::team_top`, `RATES`, `season_qualifies`, `career_qualifies`, `norms_of` (regular's bar), `Marks::established` (five seasons), `chasing` (three games, nine tenths, a quarter), `season_games`.
- `world/analytics.rs`: `charted`, `boxed`, `volume`, `box_volume`, `default_min`.
- `crates/afm-game/src/lane_data/books.rs`: `REAL_NFL`, `REAL_CMP` (the real NFL records shown on top, through the 2025 season).

## Related pages

- [nfl.md](nfl.md): the NFL's honors and its own record book.
- [match-engine.md](match-engine.md): the counts a box score and the charting come from.
- [oddities.md](oddities.md): code that disagrees with its design.
