# The world log: how it works today

Checked against main at COMMIT on 2026-10-28.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`.

The world log is world reader lane R2: what the world works out and then throws away, overwrites or deletes, written to disk as the days go by, so the world reader can show every year and every why, not only the moment a world file holds.

## Quick answers

- Does the log change the world? No. Its hooks only read: no random draws, no changes, no reordering. `tests/world_log.rs` plays two worlds side by side, one with the log, and compares every person's fingerprint and the other stores every day (`world/wlog/mod.rs`, module doc).
- Where is it? A folder beside the world file or save: `name.afmlog` next to `name.afmworld` or `name.afmsave` (`crates/afm-game/src/world_log.rs`, `beside`).
- What is in the folder? `meta.json` and one folder per kind holding segments `YYYY-NNNN.mpz` (football year, then session order). A segment is a run of zstd frames of named MessagePack records (`world/wlog/mod.rs`, `WorldLog::put`, `append_frame`).
- Does it grow the game's memory? No. Records are packed and compressed a frame (1 MB) at a time by the thread that writes them (at the book, every core) and a writer thread appends them; at most four frames wait (`world/wlog/mod.rs`, `FRAME`, `send`, `start`).
- When is logging on? Once a log is attached and the world is recording history: the 12 spin-up years are not logged (`World::logging`).
- Which kinds are there? people, players, growth, coaches, programs, schools, clubs, exits, events, news, snapshots, whys, dropped, world (`world/wlog/mod.rs`, `KINDS`, which `meta.json` repeats in words).
- How exact are the numbers? Every `f64` is written as an `f32`, seven significant digits (`world/wlog/narrow.rs`, `Narrow`). Ratings, ceilings and tendencies are written in tenths, as whole numbers (`attributes_x10`: 734 is 73.4; `world/wlog/rows.rs`, `tenths`).
- When are the yearly rows written? At the signing day book, before leavers are dropped (`world/clock.rs`, `signing_day_and_book`, calls `wlog::book`); clubs, events, news and the world row when the football year closes (`world/clock.rs`, `rollover`, calls `wlog::year_end`).
- What does a player row hold? Everyone in football (high school, prep school, college, the draft pool, the NFL, time away) gets one each year: the whole person but what never changes, cut to this year: every rating and ceiling, hidden trait, tendency, body, background, mood, fame, money, stars, visibility, schooling, calendar state, training state, the latest season row and the pro career's year, plus his true overall and potential (`world/wlog/rows.rs`, `PlayerRow`, `Lean::of`).
- Where are his name, family and face? In `people`, written once, at the first book he is in: name, heritage, birth date, home state, first school and class, family, childhood team, face. Ids only grow, so `meta.json` keeps the first id not written yet (`people_mark`) and a save's log carries on from it (`world/wlog/rows.rs`, `Static`; `WorldLog::set_people_mark`).
- Where is the why of a player's growth? The growth kind: the context the world gave the step and every term it drew, with whose teaching made the coaching (`world/wlog/why.rs`, `GrowthRow`; `development.rs`, `GrowthWhy`, `take_why`).
- Is anyone lost when he leaves football? No. Everyone who leaves gets an exit row instead of that year's player row: the same lean person, every note he has, and why he left, written before `World::compact` drops him (`world/wlog/rows.rs`, `Exit`, `exit_why`, `in_football`).
- How do I read a log? `cargo run --release --bin afm_log -- sizes <log>` for records and bytes by kind and year; `dump <log> <kind> [year]` for JSON lines (`crates/afm-tools/src/bin/afm_log.rs`). In code: `world/wlog/read.rs` (`segments`, `kind`, `each`, `each_json`).
- How does a save keep its log? Saving closes the open segments and mirrors the folder beside the save by hard links, so it costs almost no disk; later records go into new segments (`WorldLog::save_to`, `mirror`).
- Can it be turned off? `AFM_WORLD_LOG=off` for the game and the world build (`crates/afm-game/src/world_log.rs`, `on`).

## The kinds

| Kind | When | What |
|---|---|---|
| people | the book, once | What never changes about a person (`rows.rs`, `Static`) |
| players | the book | One row per person in football (`rows.rs`, `book`) |
| growth | each development step | Context, staff ids and every drawn term (`why.rs`, `growth`) |
| coaches | the book | The whole coach (`rows.rs`, `CoachRow`) |
| programs | the book | The whole program and its season's result (`rows.rs`, `ProgramRow`) |
| schools | the book | The whole high school, seasons cut to the last, and its result (`rows.rs`, `SchoolRow`) |
| clubs | year end | Club, league team and NFL season row (`rows.rs`, `ClubRow`) |
| exits | the book | The whole person and why he left (`rows.rs`, `Exit`) |
| events | year end | Every calendar event of the year and every resolved answer (`rows.rs`, `EventRow`, `Resolved`) |
| news | year end | Every news item added this year (`rows.rs`, `NewsRow`) |
| snapshots | key dates | Whole stores (`snap.rs`) |
| whys | as it happens | Dated events with their why (`why.rs`, `Why`) |
| dropped | as it happens | What a store drops to stay small (`why.rs`, `log_dropped`) |
| world | year end | Counters, rule book and changes, champions, fitted numbers, a fingerprint of each store (`rows.rs`, `WorldRow`; `why.rs`, `fingerprints`) |

## The whys

Each `Why` names people by `Player::id` and coaches, programs, schools and NFL teams by index (`world/wlog/why.rs`).

- `PositionMove`: from, to, both fits and who moved him (the staff's reconsideration, `world/season.rs`, `reconsider_positions`; the pro projection, `world/recruiting.rs`, `draft_begin`).
- `PortalEntry`: the lean, each event that built it with its size and date, the reason, his role, whether he was held (`world/recruiting.rs`, `enter_portal`).
- `TransferSeason`: a transfer's role before and in his first season (`world/season.rs`).
- `Choice`: a commitment or signing, the three schools that pulled most with their appeal and the seven reasons from `recruiting::reason_parts`, each as (appeal points, the school's score). It is the day's appeal: the signing utility's position room and the year's relationship are not in it (`world/recruiting.rs`, `announce_commitments`, signing).
- `Review`: an athletic director's yearly review of a college head coach: the read and its parts, the bar, the chance of firing, the outcome (`world/staff.rs`, `college_review`).
- `Search`: a coaching search: the employer's read spread, the top 20 candidates called with their scores, who was hired and why (`world/staff.rs`, `fill_opening`). Each candidate's score parts are not split out.
- `Combine`: whether he skipped the workouts and the numbers his body and ratings make (`world/recruiting.rs`; `body.rs`, `Drills::expected`).
- `Market`: a free agent's day with offers: his ideal, every offer and its score, the best, and what he did (`world/pro/market.rs`, `market_day`).
- `Transaction`: drafted, signed undrafted, signed, extended, moved, released or waived, retired, with the why and the contract after (`world/pro/mod.rs`, `world/pro/market.rs`).
- The recruiting log's new rows (portal moves, where the five-stars went) and every alumnus as written.

## Snapshots

Taken first thing in the morning, before any system acts that day (`world/wlog/snap.rs`, `day`, called from `world/clock.rs`, `advance_day`).

- College kickoff: the college season (schedule, lines, coaches) and the deals.
- Selection day: the college season and the voters. After the title game: the college season.
- Draft morning: the desks' boards and mocks (scouting), clubs, the league, the market and the draft pool with each prospect's reads.
- After the draft and the undrafted rush: scouting and the market. After cutdown: clubs, the league, the market. NFL kickoff: clubs and the NFL season. Early signing day: scouting.
- The college season's team sheets (every engine player, the film, the staff) are only in the year-end copy; the other college snapshots set them aside for the moment they are written and put them back as they were (`snap.rs`, `day`).
- Year end (`snap.rs`, `year_end`): college (with the team sheets), high school and NFL seasons (polls every week, standings, brackets, awards), the league, scouting, market, voters, folk, finance, stories, storylines, charts, other leagues, formers, deals, searches, realignment and the NFL book.

## In the game

- Building a world (the game's button and `afm_world`) writes `name.afmlog` beside `name.afmworld` from the same run and commit (`crates/afm-game/src/jobs.rs`, `build_world_file_named`).
- Importing a world brings the log beside it along and drops one from an older build (`crates/afm-game/src/files.rs`, `import_world`).
- While you play, the log is written into `logs/live.afmlog` in the game folder. Opening a world or a save links its log there and writes on after it, a new session in `meta.json`; one without a log starts a new log that day (`crates/afm-game/src/world_log.rs`, `on_open`).
- Saving links the live log beside the save; renaming and deleting a save do the same to its log (`world_log.rs`, `on_save`, `on_rename`, `on_delete`).

## Sizes and speed

SIZES

## What it does not keep yet

- Each search candidate's score parts (only the score), and the signing utility's position room and relationship terms in `Choice`.
- Notes per year (cleared from player rows; the person's notes are in the exit row and the world file), mood history beyond each year's row.
- Game-level records: each game's plays and box score are lane R1's game log.

## Debug accessors

For the reader, with the `debug` feature only (`debug.rs`): `person_hidden` (visibility, grade trend, the role a transfer left), `coach_hidden` (learning ceiling, home state, decline start) and `world_store`, any of the world's inner stores in `WORLD_STORES` as JSON.

## Where the knobs are

- `world/wlog/mod.rs`: `FRAME` (bytes per frame, 1 MB), `LEVEL` (zstd level 3), `FORMAT`.
- `world/wlog/snap.rs`: the key dates.

## Related pages

[development.md](development.md) (the growth step whose terms are logged), [recruiting.md](recruiting.md), [staff-and-coaches.md](staff-and-coaches.md), [nfl.md](nfl.md).
