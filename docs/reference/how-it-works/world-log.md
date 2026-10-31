# The world log: how it works today

Checked against main at 1bb5507 on 2026-10-28, with world reader lane R2; every game's rows, snaps and situation totals added by lane R1 on 2026-10-29; the recruiting and college bugs before the rebuild (minor bugs 86, 88, 99-105, 118, 126, 130, 131, 150) on 2026-10-30 and 31.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`.

The world log is world reader lane R2: what the world works out and then throws away, overwrites or deletes, written to disk as the days go by, so the world reader can show every year and every why, not only the moment a world file holds.

## Quick answers

- Does the log change the world? No. Its hooks only read: no random draws, no changes, no reordering. `tests/world_log.rs` plays two copies of one small world side by side for two years (read back the way the game opens a world), one with the log, and compares every person's fingerprint and the other stores every day; on the full 2026 world, February to September in the game ends on the same fingerprint with the log on and off (`world/wlog/mod.rs`, module doc).
- Where is it? A folder beside the world file or save: `name.afmlog` next to `name.afmworld` or `name.afmsave` (`crates/afm-game/src/world_log.rs`, `beside`).
- What is in the folder? `meta.json` and one folder per kind holding segments `YYYY-NNNN.mpz` (football year, then session order). A segment is a run of zstd frames of named MessagePack records (`world/wlog/mod.rs`, `WorldLog::put`, `append_frame`).
- Does it grow the game's memory? No. Records are packed and compressed a frame (1 MB) at a time by the thread that writes them (at the book, every core) and a writer thread appends them; at most four frames wait (`world/wlog/mod.rs`, `FRAME`, `send`, `start`).
- When is logging on? Once a log is attached and the world is recording history: the 12 spin-up years are not logged (`World::logging`).
- Which kinds are there? people, players, growth, coaches, programs, schools, clubs, exits, events, news, snapshots, whys, dropped, world, and lane R1's games-college, plays-college, games-nfl, plays-nfl, games-other, plays-other, games-hs, situations, team-situations (`world/wlog/mod.rs`, `KINDS`, which `meta.json` repeats in words).
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
| exits | the book | The person as in `players`, every note he has and why he left (`rows.rs`, `Exit`) |
| events | year end | Every calendar event of the year and every resolved answer (`rows.rs`, `EventRow`, `Resolved`) |
| news | year end | Every news item added this year (`rows.rs`, `NewsRow`) |
| snapshots | key dates | Whole stores (`snap.rs`) |
| whys | as it happens | Dated events with their why (`why.rs`, `Why`) |
| dropped | as it happens | What a store drops to stay small (`why.rs`, `log_dropped`) |
| world | year end | Counters, rule book and changes, champions, fitted numbers, a fingerprint of each store (`rows.rs`, `WorldRow`; `why.rs`, `fingerprints`) |
| games-college, games-nfl, games-other, games-hs | each game | The game row (`games.rs`, `game_row`, `standin_row`; see "Games, snaps and situations") |
| plays-college, plays-nfl, plays-other | each engine game | Every snap (`games.rs`, `plays`) |
| situations, team-situations | year end and each save | The season's situation totals by group and by team (`games.rs`, `write_totals`) |

## The whys

Each `Why` names people by `Player::id` and coaches, programs, schools and NFL teams by index (`world/wlog/why.rs`).

- `PositionMove`: from, to, both fits and who moved him (the staff's reconsideration, `world/season.rs`, `reconsider_positions`; the pro projection, `world/recruiting.rs`, `draft_begin`).
- `PortalEntry`: the lean, each event that built it with its size and date, the reason, his role, whether he was held (`world/recruiting.rs`, `enter_portal`).
- `TransferSeason`: a transfer's role before and in his first season (`world/season.rs`).
- `Choice`: a commitment or signing, the three schools that pulled most with their appeal and the seven reasons from `recruiting::reason_parts`, each as (appeal points, the school's score). It is the day's appeal: the signing utility's position room and the year's relationship are not in it (`world/recruiting.rs`, `announce_commitments`, signing). It is read as he chose, before the commitment or signature marks him settled (minor bug 118: read after, the +10 hold of a signed or committed recruit sat in "his ties").
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
- The "Build a world" workflow writes the world and its log on the runner's big disk and publishes the log beside the world as `name.afmlog.tar.001`, `.002`, ... (`.github/workflows/build-world.yml`). The game does not download it.

## Sizes and speed

Measured 2026-10-28 on the 2026 world file (851,571 people) and on a 1-in-20 world (`afm_log`, `memory_season`).

- A year's book at full size: written in 17 s on 4 cores, 0.36 GB of memory while it runs. Player rows 439 MB (515 bytes each), the first `people` rows 63 MB (74 bytes each, once), coaches 21 MB, programs 6 MB, schools 5 MB, the year-end snapshots 59 MB (`afm_log book`).
- The growth rows at full size: 1.06 million steps from February to September, 179 MB (170 bytes each).
- A whole football year at full size comes to about 0.9 GB: players 440 MB, growth 180, exits about 140, snapshots about 70, coaches 21, the new people about 20, the rest about 30. A 30-year world's log is about 27 GB, and a season you play adds about 0.9 GB beside your saves (each save links the same files, so saves add nothing).
- Speed in the game: February 10 to the first game on September 5 took 339 s with the log and 334 s without (1.5%), and the world's fingerprint matched (`memory_season`). The book day adds its 17 s.
- A 1-in-20 world (scale 0.05, 3 history years and 2 more): 155 to 178 MB a year, peak memory 1.02 GB against 0.88 GB without (`afm_log build`; measured before the snapshots and exits were trimmed and before compression moved to every core).

## Games, snaps and situations (lane R1)

Written by `world/wlog/games.rs` from each game's result, where the world records its games: college (`world/college.rs`, `play_day`), the NFL (`world/nfl_year.rs`: regular season, playoff rounds, Super Bowl; `world/pro/mod.rs`, `preseason_week`; both through `world/pro/games.rs`, `log_game`), the other leagues (`world/pro/other.rs`, `log_game`) and high school (`world/hs_year.rs`, engine and stand-in games). The rows are built on the cores that play the games; nothing is drawn, so results stay the same (`tests/world_log.rs` plays the world with and without the log, and checks the game kinds hold every level, the snaps, red zone and clock totals and the new situation keys).

- **Reading a row**: a field that is zero, false, empty or null is left out, except in the team boxes (read a missing field as zero, false or empty); arrays keep every place (a score of `[0, 21]` stays). Decimals: one place from 10 up, three below. Enums by name (`"Bruise"`, `"OffensiveHolding"`); an enum with a value keeps its name even when the value is zero (`{"Challenge": 0}`: the home side challenged). Boxes, player lines, drives and snaps are built that way in one pass on the core that played the game (`wlog/lean.rs`, `lean`), the rest of the row is compacted after (`games.rs`, `compact`, `round`), and each record is packed there too, so the world's own thread only adds them in order (`wlog/mod.rs`, `pack`, `put_packed`). Players by `Player::id`; `teams` [home, away] are program ids (college), club indices (NFL), franchise numbers (other leagues) or high school ids.
- **Naming a game**: its `level`, `season`, `date` and `teams` (and `league` in the other leagues). A snaps record carries the same keys, so it joins its row. The season is the football season: a January bowl and February's Super Bowl go under the year the season started.

### A game row

- **Facts** (by level): `level`, `season`, `date`, `week` (college: weeks since the season's first game day; NFL: the schedule week, playoffs after it; high school: the state's week), `kind` (`regular`, `conference title`, `bowl`, `playoff`, `title`, `preseason`, `wild card`, `divisional`, `super bowl`, `state final`), `round`, `name` (a bowl's), `divisions` and `cross` (college), `conference_game`, `rivalry`, `site` (neutral site: state, town, indoor), `neutral`, `kickoff` (Eastern for college and the NFL, local for high school), `tv` (the NFL's window), `line` [home spread, total], `attendance`, `paid` (college guarantee or neutral-site payout), `weather` (college: block, temperature, wind, rain, snow, indoor, lightning delay, moved; high school: the town's), `conditions` (the engine's), `tags`, `loud`, `why`, `huge_for`, `senior_day`, `homecoming`, `state`, `league`, `names`.
- **The game**: `score`, `quarters` (points by quarter, overtime after the fourth), `ot`, `snaps`, `toss` (winner and choice), `timeouts_left` (each side at the end of each half).
- **`box`** [home, away], every field kept, zeros too (so sums and averages over boxes see them): every field of the engine's `TeamStats` (yards, first downs, third and fourth downs, penalties, turnovers, `time_of_possession` (ball in play) and `clock_possession` (game clock), sacks, `red_zone_trips`, `red_zone_tds`, EPA, successes, explosives, pressures, drives, punts, field goals and the rest) and what the lines and plays add: touchdowns by kind, sack yards, kick and punt returns, extra points and two-point tries, kickoffs and touchbacks, punt gross and net, inside the 20, QB hits, tackles for loss, passes broken up, forced fumbles and recoveries, field goals by distance (`fg_att_u30` ... `fg_made_50`), defensive touchdowns, blocked kicks, emergency snaps.
- **`players`** [home, away]: every player's full `StatLine` (snaps on offense, defense and special teams, passing, rushing, receiving, blocking, pass rush with wins by speed, power and finesse, tackling, coverage, kicking by distance, punting, returns, penalties and busts, grade, top speed) with his `id` and `pos` that day.
- **`drives`**: team, quarter, clock, start and end (yards from its own goal), plays, seconds, result, points, first and last play, `red_zone` (`game/drives.rs`, `drives`).
- **`scoring`**: play, quarter, clock, team, what (touchdown, field goal, safety, single, two point), who (passer, carrier, target, kicker, returner, interceptor), the try after it and the score after. A defensive touchdown (a pick six, a fumble return) counts; only a touchdown takes a try; an overtime round's two-point play that scores, with no touchdown before it (college from the third overtime, the other leagues' shootouts), is its own "two point" entry (minor bug 133).
- **`injuries`**: team, player, kind, weeks out, played on, play and quarter. **`penalties`**: play, quarter, team, player, kind, yards, accepted, pre-snap.
- **`officiating`**: close calls, wrong rulings, booth reviews, replay assists, challenges and won, overturned, wrong calls left standing, each call (play, kind, wrong, reviewer, overturned, against, note) and the crew's draw.
- **`between`** (timeouts, challenges and other moments between plays), **`decisions`** (garbage-time calls), **`trust`** (each player's change of trust in his coach).
- **Stand-in games** (a season or level not on the engine, forfeits): the facts, `standin: true`, `home_won`, the stand-in's `score` (college) and `forfeit`.
- **High school engine games** keep the row with box and player lines, not the snaps.

### A snap

Every field of the engine's `PlayRecord` (`game/stats.rs`), compacted: number, quarter, clock, offense (0 home), down, to go, yard line (from the opponent's goal), score, kind, the call and its why, the defense's call and package, personnel, box, blitz, play action, coverage, route, yards, first down, touchdown, turnover, every role (passer, target, carrier, tacklers, missed tackles, sackers, hits, pressures, breakup, defender, interceptor, fumbles), air yards, after the catch, before contact, time to throw, separation, thrown away, dropped, pressured, scramble, out of bounds, nullified, seconds, the kick (kicker, returner, distance, return, good, touchback, fair catch, inside 20, blocked, bad snap, rush pressure), penalties, every block (blocker, defender, pass, won, seconds, pressure given up), rush wins with the move, busts, freelancing, injuries, the 22 on the field, the fastest man and his speed, the why, garbage-time steps, the tendencies' notes, your calls, the close call, the throw's path, the fourth-down model's chances, timeouts, the hash, no huddle. Added by the log: `ep_after` (`ep_before` + `epa`) and `wp_after` (the offense's win chance at the next snap, or the result: 1, 0, 0.5). `wp_before` is the engine's simple chance for the side with the ball (`game/flow.rs`, `new_record`).

### Situation totals

Every engine game's plays summed by key for the season (`world/game_log.rs`, `play_keys`, `game_situations`), held in memory and written at year end and at each save, then dropped (`games.rs`, `add_situations`, `write_totals`): a season's totals are the sum of its records. Each key's `v` is [plays, yards (kick distance for kicks), EPA, successes, first downs, touchdowns, turnovers, one extra: completions on passes, sacks on sacks, makes on kicks, touchbacks on kickoffs and punts, missed tackles on runs].

- `situations`: one record per season, level, group and key, its totals in `v`. College groups are divisions (`cross-division` for buy games), NFL `NFL`, other leagues by league, high school by state, with high school's play names (`call|`, `defense|` and the like) under `all` for the whole country.
- `team-situations`: one record per season, level, team and key, college and the pro leagues (high school's 14,000 schools would hold 1.2 million keys in memory all season). Keys `o|` with the ball, `d|` defending: `kind`, `third|to go|kind`, `fourth|decision`, `red zone|kind`, `pressured`, `blitz`.
- The trial keys: `kind`, `dd|down|to go|kind`, `field|zone|kind`, `fourth|zone|to go|decision`, `state|quarter|margin|kind`, `two minute`, `call`, `defense`, `personnel`, `box`, `blitz`, `coverage`, `after turnover`, `play action`, `pressured`, `time to throw`, `depth`, `drop`, `throw away`, `kick|kind|distance`, `return`, `penalty|kind|accepted`, `injury`, `missed tackles`.
- Added by R1, in the real data's buckets: `real pass rate|down|to go|score|time|kind` and `real fourth|to go|yard line by tens|score|decision` (`data/real-world/nfl_situations.json`: to go 1, 3, 6, 10, 15; score -15, -8, -1, 0, 7, 14, 15; time normal, late (the fourth quarter's last 7 minutes), two_minute (a half's last 2)); `real air yards|-5|0|10|20`; `q4|lead or trail bucket|kind` (`garbage_time.json`); `run gap|side|gap` (where the carrier crossed the line: inside a yard the middle, 2.5 the guard, 4 the tackle, else the end); `route|route`, `route coverage|route|man or zone`; `backfield|gun, pistol or under center|kind`; `motion|kind|play kind`; `rpo|kind`; `tempo|huddle or no huddle|kind`; `garbage|offense step|defense step|kind`; `out of bounds|kind`; `kick result|kind|blocked, muffed, bad snap, fair catch, touchback, inside 20`; `penalty site|kind|home or away`; `officiating|call|right or wrong|reviewer|stood or overturned`.

### What the games cost

Measured 2026-10-29 on the full-size world, seed 2026, its 2025 season recorded and 2026 played on, every level on the engine (`AFM_ENGINE_FROM=2025 AFM_HS_YEARS=1 afm_log build 2026 1 1.0 <folder> 1`, and the same with `AFM_WORLD_LOG=off`), 4 cores.

- Results: the same fingerprint with the log on and off (`aec1ce7a26168d3b`).
- Size a season: high school games 375 MB (72,700 games, 5.2 KB each), college games 27 MB (4,770, 5.7 KB) and their snaps 133 MB (28 KB a game), NFL games 1.7 MB and snaps 9.2 MB (333 games), the other leagues 0.7 and 3.8 MB (129 games), situations 4.0 MB and team situations 1.1 MB: 556 MB a season with high school on the engine, about 190 MB without. With R2's kinds a season's whole log came to 1.48 to 1.56 GB.
- A 30-year world (the last 12 seasons with high school on the engine): about 12 x 556 MB + 18 x 190 MB = 10 GB of games beside R2's rows.
- Time, the whole log (R2's rows too): 2025 took 1,159 s against 948 s with the log off, 2026 1,113 s against 926 s (+20 to 22%). The first build of the game log took +53%; building rows lean and packing them on the cores cut it.
- Memory: resident 5.32 GB against 5.26 GB after 2026, the live heap 2.98 GB against 2.96 GB. The world's peak (7.3 GB) comes in a spin-up season, before logging starts.

## What it does not keep yet

- Each search candidate's score parts (only the score), and the signing utility's position room and relationship terms in `Choice`.
- Notes per year (cleared from player rows; the person's notes are in the exit row and the world file), mood history beyond each year's row.
- High school snaps: high school engine games keep the row, box and player lines, not each snap (the brief's ask; 72,000 games a year).

## Picks made in the build (proposed, not confirmed)

- The snapshot dates above, and the college team sheets once a year.
- Ratings, ceilings and tendencies in tenths and every other number to seven digits.
- The game writes the log by default, beside every save.
- R1: the run gap's yard bands, high school play names counted for the whole country, no team totals for high schools, games named by date and teams.

## Debug accessors

For the reader, with the `debug` feature only (`debug.rs`): `person_hidden` (visibility, grade trend, the role a transfer left), `coach_hidden` (learning ceiling, home state, decline start) and `world_store`, any of the world's inner stores in `WORLD_STORES` as JSON.

## Where the knobs are

- `world/wlog/mod.rs`: `FRAME` (bytes per frame, 1 MB), `LEVEL` (zstd level 3), `FORMAT`.
- `world/wlog/snap.rs`: the key dates.

## Related pages

[development.md](development.md) (the growth step whose terms are logged), [recruiting.md](recruiting.md), [staff-and-coaches.md](staff-and-coaches.md), [nfl.md](nfl.md).
