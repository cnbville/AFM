# The Data section's tallies: how it works today

Checked against main at 68355bb with lane NB1 (Data overhaul item 1, item 2 and minor bug 183) on 2026-11-04.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `.github/`. "Proposed, not confirmed" marks Claude's own picks in this lane, which Cain hasn't answered.

## Quick answers

- What are the tallies? Counts the world keeps as its games are played, for the Data section's pages, so they don't need the world log (about 40 GB for a 30-year world) (`world/tally/mod.rs`, module notes).
- Which kinds? Five, each its own file per season: `players`, `teams`, `games`, `classes`, `people` (`world/tally/mod.rs`, `KINDS`).
- Where do they live? In a folder beside the world file or save, `name.afmdata`, one file per kind and season: `name.afmdata/<kind>/<season>.tally` (`world/tally/mod.rs`, `EXT`, `path`; `crates/afm-game/src/tallies.rs`, `beside`).
- And the season being played? In the world itself (`World::tallies`), so a save carries it (`world/mod.rs`).
- When is a season written? When the football year closes, the day after the Super Bowl, after the world log's year end. The open store then starts over, so memory stays flat (`world/clock.rs`, `rollover`; `world/tally/mod.rs`, `tally_close`).
- Since when? From the first recorded season, as the record books; the spin-up years count nothing (`World::tallying`: recording and not off).
- Do they change the world? No. They make no random draws and nothing in the world reads them. `tests/tallies.rs` plays two copies of one world day by day, tallies on and off, and checks every person and the main stores match every day (`tests/tallies.rs`, `the_tallies_only_watch_and_add_up`).
- Can they be turned off? `AFM_TALLIES=off` when a world is built (`world/tally/mod.rs`, `Tallies::new`); `World::set_tallies_off` for tools.
- Which games are counted? Every engine game at every level: college (`world/college.rs`, `play_day`), the NFL's regular season, playoffs and Super Bowl (`world/nfl_year.rs`; `world/pro/games.rs`, `tally_game`), high school (`world/hs_year.rs`). Games on the stand-in are not. The other leagues are not counted (proposed, not confirmed).
- Is the work done on one core? No: each game's part is built on the core that played it, then added in order (`world/tally/games.rs`, `part`; `World::tally_game`).
- What if no folder is attached? Closed seasons wait in memory until one is (`Tallies::held`, `World::attach_tallies`). The game always attaches one.
- How big is a season? See "Sizes" below.

## The players file

- One row a player and level a season: college and NFL. Team (his last that season), position, games, starts, snaps on offense and defense, the game keys he played (into that season's games file) and his numbers split by situation (`world/tally/players.rs`, `Season`, `Open::add`).
- Splits: down (1st to 4th), distance (short 1-3, medium 4-7, long 8+), field zone (backed up inside his own 20, open field, red zone, goal to go), quarter (1-4, overtime), the two-minute drill, and the score (ahead 9+, within 8, behind 9+) (`players.rs`, `SPLITS`, `codes`).
- For each split: passing (dropbacks, attempts, completions, yards, TDs, picks, sacks, EPA, successes, first downs, scramble EPA), rushing (carries, yards, TDs, EPA, successes, first downs), receiving (targets, catches, yards, TDs, EPA, first downs) and defense (tackles, missed tackles, pressures, sacks, targets, completions and yards allowed, breakups, picks) (`PASS`, `RUSH`, `RECV`, `DEF`).
- A start: on the field for his side's first snap with the ball (offense) or first snap without it (defense) (proposed, not confirmed; `world/tally/games.rs`, `starters`).
- High school players get no rows: their games are counted with facts and boxes only (proposed, not confirmed).

## The teams file

- One row a program or club a season: games, wins, losses, points for and against, the advanced numbers for and against, penalties, and situation cells for both sides of the ball (`world/tally/teams.rs`, `Season`).
- Each cell keeps plays, yards (a kick's distance on kicks), EPA, successes, first downs, touchdowns, turnovers and one extra count by kind: completions on passes, sacks, missed tackles on runs, kicks made, touchbacks (`teams.rs`, `Situ`, `values`).
- Keys start `o|` (with the ball) or `d|` (without it), then the split: play kind, down, down and distance, field zone, quarter, two minute, score, red zone, goal to go, third and fourth down, personnel, backfield, motion, RPO, tempo, men in the box, defensive package, the defense's call, explosive; on dropbacks also blitz by down, pressured, play action, coverage and throw depth (`teams.rs`, `keys`).
- Plays wiped by a penalty and no-plays are left out of the cells (`teams.rs`, `game`).

## The games file

- Every engine game at every level: level, season, date, kind (regular, conference title, bowl, playoff, Super Bowl, state final and so on), the two teams, group (division or the NFL's conferences), neutral site, conference game, the score by quarter, overtime, weather and both team boxes (`world/tally/games.rs`, `GameRow`, `Facts`).
- College and NFL games also carry each side's advanced numbers (runs and run EPA and success, stuffs, line yards, dropbacks and pass EPA and success, havoc made, drives, trips inside the 40 and their TDs and field goals, three and outs, average start), the home side's win chance before every scrimmage snap (0-100) and every player's line (`Adv`, `adv`, `wp_chart`, `Line`).
- A line keeps the stat book's numbers: counts whole, rates and EPA to the hundredth (`games.rs`, `quantize`; `world/records.rs`, `Stat::fraction`).
- During the season the games are packed into the store as they come (zstd level 3, frames of about 256 KB), so the open season stays small (`world/tally/file.rs`, `LEVEL_OPEN`, `FRAME_RAW`; `games.rs`, `Open::add`).
- Games since a date at a level can be read back while the season runs (`World::tally_games_since`), which the charting delay uses.

## The classes file

- Every draft pick since recorded history began, and every Division I (FBS and FCS) signee straight from high school, each with what became of him so far: college and NFL seasons, games, starts and snaps, his draft slot, transfers and the season counted through (`world/tally/classes.rs`, `Pick`, `Signee`, `Outcome`).
- Signees join their class when the season of their high school class closes; junior college players and players without a Division I first program are left out (proposed, not confirmed; `classes.rs`, `Store::close`).
- Each season's file is the whole store as that season closed, so a page reads one file for every class (`classes.rs`, module notes; `tally_close`).
- Honors (All-America, Pro Bowls, All-Pro) are left to the world's own books (`StatBook::honors`, `NflBook`).

## The people file

- Each coordinator's unit and each head coach's margins a season, with ten numbers and their rank among the level's units that season: points a game, yards a play, EPA a play, success rate, explosive play rate, turnovers a game, third down rate, points a scoring chance (7 a TD and 3 a field goal per trip inside the 40), havoc rate, three and outs a game (`world/tally/people.rs`, `STATS`, `UP`, `rows`).
- Who had the unit comes from the world's staff charges (`world/units.rs`, `Charge`).

## The file

- `AFMTALLY`, format 1 (u32), a u32 length and that much JSON (kind, season, records), a u32 count and a table of frames (first and last key, count, offset and length), then the frames: each one zstd stream of records laid out as key (u64), length (u32) and the record in named MessagePack (`world/tally/file.rs`, `Header`, `Frames::file_bytes`).
- Records are in key order, so a page reads one frame to find one player, team or game (`file.rs`, `Opened::get`, `get_many`).
- Closed files are packed at zstd level 9 (`LEVEL_CLOSE`).
- A file is written as `.tally.partial` and renamed into place, so it is whole or untouched, and a written file is never changed (`file.rs`, `write`).

## In the game

- While you play, closed seasons go into `data/live.afmdata` in the game folder. Opening a library world or a save links its folder there (hard links, a copy where links can't be made) and the career's seasons go on after it (`crates/afm-game/src/tallies.rs`, `on_open`, `mirror`).
- Saving links the live folder beside the save; renaming or deleting a save moves or deletes its folder (`tallies.rs`, `on_save`, `on_rename`, `on_delete`).
- Building a world in the game writes the history's seasons straight into `name.afmdata.part` beside the world file, renamed when the build ends (`crates/afm-game/src/jobs.rs`, `build_world_file_named`; `World::new_recorded`).
- Importing a world brings the folder beside it, or its tar file (`name.afmdata.tar`, or in parts `.tar.001`, `.002`, ...), into the library (`crates/afm-game/src/files.rs`, `import_world`; `tallies.rs`, `import`, `untar_parts`). Only `<kind>/<season>.tally` files are taken from a tar.
- The worlds page carries the folder as one tar file, split past 2 GB, and the download fetches it with the world (`.github/workflows/build-world.yml`, publish step; `crates/afm-game/src/jobs.rs`, `data_assets`). Publishing it as a tar is proposed, not confirmed.

## Charting depth and delay (item 2)

- Your analytics head's rating sets how deep your charting goes: 15 and up gives tracking-style charting (separation, run block and pass rush wins), 8 and up film charting (CPOE, depth of target, pressures, missed tackles, coverage, grades), less play-by-play numbers only (EPA, success and the box) (proposed, not confirmed; `world/analytics.rs`, `charting`, `depth`).
- And how soon it arrives: 17 and up the morning after a game, 12 and up two days after, 7 and up three days, less four (proposed, not confirmed; `charting`).
- It only changes what you see: the card and table leave out the numbers past your depth, and this season's lines leave out the games not charted yet (`crates/afm-game/src/lane_data/analytics.rs`, `card_of`, `charted_now`).
- The NFL's own tracking reaches every seat at full depth the morning after (proposed, not confirmed; `crates/afm-game/src/lane_data/mod.rs`, `charting`).
- The Sources line names your analytics head and says what you get and when (`lane_data/mod.rs`, `sources`). The rating itself is never shown.

## Sizes

- Measured with `memory_check` on a 0.1 world; see `docs/game.md`, "Lane NB1".

## Where the knobs are

- `world/tally/file.rs`: `FRAME_RAW` (256 KB), `LEVEL_OPEN` (3), `LEVEL_CLOSE` (9).
- `world/tally/players.rs`: `SPLITS`, `PASS`, `RUSH`, `RECV`, `DEF`.
- `world/tally/people.rs`: `STATS`, `UP`.
- `world/analytics.rs`: `charting` (depth and delay by rating), `depth` (which numbers need which depth).

## Related pages

- [records-and-data.md](records-and-data.md): the record books and Player analytics.
- [world-log.md](world-log.md): the world log, the trial tooling's full record.
- [saves.md](saves.md): the world and save files.
