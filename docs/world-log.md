# The world log: how it works

The world reader's log (spec `afm/world-reader/spec.md`, section 3). The world file holds one moment; the log keeps what the world works out and then throws away. Lane R1 (the game log) built the format and every game's rows; lane R2 adds its own streams (events, yearly rows, snapshots, exits, meta) to the same folder in the same format. Lane R4 reads it into the book.

## Where it is and when it is written

- The world build writes it beside the world file: `<world>.afmworld` and `<world>.afmlog/` (`crates/afm-game/src/jobs.rs`, `build_world_file_named`). It is written as `<world>.afmlog.part/` while the history plays and renamed when the world file is done; a log that failed stays `.part` and the build prints the error.
- `world_trial` writes it with `AFM_LOG=<folder>` (history and future seasons).
- Any world can start one with `World::open_log(dir)` and close it with `World::finish_log()` (`crates/afm-engine/src/world/mod.rs`). `WorldOptions::log_dir` opens it when the recorded history starts (the 12 spin-up seasons are not logged). The game's career saves turn it on in lane R2.
- Only recorded seasons are logged (`World::logging`: a log is open and `recording` is on).

## The folder

- `meta.json`: `format` (1), `what`, `commit`, `engine`, the world's `options`, `first_season`, `streams` (what each holds), and after the log closes `files` (each file's bytes on disk and before compression) and `bytes` (the total).
- One folder per season, `<season>/`, with one file per stream: `<stream>.jsonl.zst`. Each file is JSON lines compressed with zstd (level 3). A file can hold several zstd frames (a log opened again appends); `zstd -dc` and every zstd library read through them.
- A season's files close when the next football year starts (`world/clock.rs`, `rollover`, calling `WorldLog::close_before`); the situation totals are written then.
- The season is the football season a game belongs to: a January bowl and February's Super Bowl go under the year the season started.

## Reading rows

- A field that is zero, false, empty or null is left out: read a missing field as zero, false or empty. Arrays keep every place (a score of `[0, 21]` stays).
- Decimals are rounded: one place from 10 up, three places below 10; a whole decimal is written as a whole number (`world/wlog.rs`, `compact`, `round`).
- Enums are written by name (`"Bruise"`, `"OffensiveHolding"`).
- Every row has ids: a game's `id` is its place in its season and level (0, 1, 2... in the order the world played them); `teams` are program ids (college), club indices (NFL), franchise numbers (other leagues) or high school ids; players are player ids, the same as the world file's `Person.player.id`.
- Quick look: `zstd -dc 2025/games-college.jsonl.zst | head -1 | jq .`

## The game streams (lane R1)

Written by `world/log_games.rs` from each game's result, at the places the world records its games: college (`world/college.rs`, `play_day`), NFL (`world/nfl_year.rs`, the regular season, the playoff rounds and the Super Bowl; `world/pro/mod.rs`, `preseason_week`; all through `world/pro/games.rs`, `log_game`), the other leagues (`world/pro/other.rs`, `log_game`) and high school (`world/hs_year.rs`, `engine_games`, `play_week`, `play_round`).

| Stream | One line per |
|---|---|
| `games-college` | college game |
| `plays-college` | college engine game: `{"id", "plays": [...]}` |
| `games-nfl` | NFL game (preseason, regular, wild card, divisional, conference title, super bowl) |
| `plays-nfl` | NFL game's snaps |
| `games-other` | other-league game (CFL, UFL and the rest; `league`, `names`) |
| `plays-other` | other-league game's snaps |
| `games-hs` | high school game, engine or stand-in |
| `situations` | (level, group, key) for the season: college groups are divisions (`cross-division` for buy games), NFL `NFL`, other leagues by league, high school by state |
| `team-situations` | (level, team, key) for the season |

### A game row

- **Facts** (by level): `level`, `season`, `date` (`YYYY-MM-DD`), `week` (college: weeks since the season's first game day; NFL: the schedule week, playoffs after it; high school: the state's week, playoffs after it), `kind` (`regular`, `conference title`, `bowl`, `playoff`, `title`, `preseason`, `wild card`, `divisional`, `super bowl`, `state final`), `round`, `name` (a bowl's), `teams` [home, away], `divisions` and `cross` (college), `conference_game`, `rivalry` (its name), `site` (a neutral site: state, town, indoor), `neutral`, `kickoff` (Eastern for college and the NFL, local for high school), `tv` (the NFL's window), `line` [home spread, total], `attendance`, `paid` (college: the guarantee or the neutral site's payout), `weather` (college: the played block, temperature, wind, rain, snow, indoor, lightning delay minutes, moved; high school: the town's weather), `conditions` (the engine's: temperature, wind, rain, snow, crowd...), `tags`, `loud`, `why`, `huge_for`, `senior_day`, `homecoming` (college), `state` (high school), `league`, `names` (other leagues).
- **The game**: `score`, `quarters` (points by quarter, overtime periods after the fourth), `ot`, `snaps` (records in the play log), `toss` (winner and choice), `timeouts_left` (each side at the end of each half).
- **`box`** [home, away]: every field of the engine's `TeamStats` (points, plays, yards, passing and rushing yards, first downs, third and fourth downs, penalties, turnovers, `time_of_possession` (ball in play), `clock_possession` (game clock), sacks, `red_zone_trips`, `red_zone_tds`, EPA, successes, explosive plays, dropbacks, pressures, drives, start_sum, punts, field goals, attempts, completions, carries, interceptions, fumbles lost) and what the lines and plays add: pass, rush and receiving TDs, sack yards, kick and punt returns (count, yards, TDs), extra point kicks, two-point tries and good, kickoffs and touchbacks, punt gross and net yards, inside the 20, touchbacks, QB hits, tackles for loss, interceptions, passes broken up, forced fumbles and recoveries, field goals by distance (`fg_att_u30` ... `fg_made_50`), defensive TDs, kicks it blocked, emergency snaps.
- **`players`** [home list, away list]: every player's full game line (`StatLine`: snaps on offense, defense and special teams, passing, rushing, receiving, blocking, pass rush with wins by speed, power and finesse, tackling, coverage, kicking by distance, punting, returns, penalties and busts, the grade sum and plays graded, top speed) with his `id` and `pos` (listed position that day).
- **`drives`**: team, quarter, clock, start and end (yards from its own goal line), plays, seconds, result, points, first and last play numbers, `red_zone` (`game/drives.rs`, `drives`).
- **`scoring`**: each score's play, quarter, clock, team, what (touchdown, field goal, safety, single), the passer, carrier, target, kicker, returner or interceptor, the try after it and the score after.
- **`injuries`**: team, player, kind, weeks out, whether he played on, the play and quarter.
- **`penalties`**: play, quarter, team, player, kind, yards, accepted, pre-snap (no play).
- **`officiating`**: close calls, wrong rulings, booth reviews, replay assists, each side's challenges and challenges won, overturned, wrong calls left standing, each call (play, kind, wrong, reviewer, overturned, against, the note) and the crew's draw.
- **`between`**: timeouts, challenges and the other moments between plays (after which play, which team, what). **`decisions`**: garbage-time calls. **`trust`**: each player's change of trust in his coach.
- **Stand-in games** (a season or level not on the engine, and forfeits): the facts, `standin: true`, `home_won`, the stand-in's `score` (college) and `forfeit`.

### A play

Every field of the engine's `PlayRecord` (`game/stats.rs`), compacted: the number, quarter, clock, offense (0 home), down, to go, yard line (from the opponent's goal), score, kind, the call and its why, the defense's call and package, personnel, box, blitz, play action, coverage, the route, yards, first down, touchdown, turnover, every player's role (passer, target, carrier, tacklers, missed tackles, sackers, hits, pressures, the pass breakup, the defender, the interceptor, fumbles), air yards, after the catch, before contact, time to throw, separation, thrown away, dropped, pressured, scramble, out of bounds, nullified, seconds, the kick (kicker, returner, distance, return, good, touchback, fair catch, inside 20, blocked, bad snap, rush pressure), penalties, every block (blocker, defender, pass, won, seconds, gave up pressure), rush wins with the move, busts, freelancing, injuries, the 22 men on the field, the fastest man and his speed, the why, after a turnover, each side's garbage-time step, emergency snaps, the tendencies' notes (where a run crossed the line, reaches, slides, muffs), your calls, the officials' close call, the throw's path across the field, the fourth-down model's chances, timeouts, the hash, no huddle, the defensive package. Added by the log: `ep_after` (`ep_before` + `epa`) and `wp_after` (the offense's win chance at the next snap, or the result: 1, 0, 0.5). `wp_before` is the engine's simple chance for the side with the ball (`game/flow.rs`, `new_record`).

### Situation keys

`game_log.rs`, `play_keys` and `game_situations`; each row's `v` is [plays, yards (kick distance for kicks), EPA, successes, first downs, touchdowns, turnovers, one extra: completions on passes, sacks on sacks, makes on kicks, touchbacks on kickoffs and punts, missed tackles on runs].

- The trial keys: `kind`, `dd|down|to go|kind`, `field|zone|kind`, `fourth|zone|to go|decision`, `state|quarter|margin|kind`, `two minute`, `call`, `defense`, `personnel`, `box`, `blitz`, `coverage`, `after turnover`, `play action`, `pressured`, `time to throw`, `depth`, `drop`, `throw away`, `kick|kind|distance`, `return`, `penalty|kind|accepted`, `injury`, `missed tackles`.
- Added by R1: `real pass rate|down|to go|score|time|kind` and `real fourth|to go|yard line by tens|score|decision` in the buckets of `data/real-world/nfl_situations.json` (to go 1, 3, 6, 10, 15; score -15, -8, -1, 0, 7, 14, 15; time normal, late (the fourth quarter's last 7 minutes), two_minute (a half's last 2)); `real air yards|-5|0|10|20`; `q4|lead or trail bucket|kind` by the score after three quarters in `garbage_time.json`'s buckets; `run gap|side|gap` (`left|end` ... `middle|middle`, from where the carrier crossed the line: inside a yard the middle, 2.5 the guard, 4 the tackle, else the end; proposed, not confirmed); `route|route` and `route coverage|route|man or zone`; `backfield|gun, pistol or under center|kind`; `motion|kind of motion|kind`; `rpo|kind`; `tempo|huddle or no huddle|kind`; `garbage|offense step|defense step|kind`; `out of bounds|kind`; `kick result|kind|blocked, muffed, bad snap, fair catch, touchback, inside 20`; `penalty site|kind|home or away`; `officiating|call|right or wrong|reviewer|stood or overturned`.
- Team keys (`o|` with the ball, `d|` defending): `kind`, `third|to go|kind`, `fourth|decision`, `red zone|kind`, `pressured`, `blitz`. On disk for every team at every level but high school; in the trial log for FBS only.

## What it costs

Measured in lane R1 (see the PR and `docs/world-model.md`'s note): the log's size per season and level, and the build's time and memory with it on against off.
