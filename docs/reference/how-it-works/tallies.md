# The Data section's tallies: how it works today

Checked against main at a0dfde3 with lane NB1 (Data overhaul item 1, item 2 and minor bug 183) on 2026-11-04. Lane NB4's depth counts and game splits over main fbf431c on 2026-11-04. Lane NB7's squads file and the expected pass model over main 57b7009 on 2026-11-04. Lane NB8 (Data overhaul items 24-27: the calls file, the analysts' model, garbage time, concepts against coverages, the advanced box extras and the teams' charting delay) over main c6e3194 on 2026-11-04. Lane NB11 (Data overhaul items 33-41: the prospects, moves and money files) over main b2dae7d on 2026-11-05.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `.github/`. "Proposed, not confirmed" marks Claude's own picks in this lane, which Cain hasn't answered.

## Quick answers

- What are the tallies? Counts the world keeps as its games are played, for the Data section's pages, so they don't need the world log (about 40 GB for a 30-year world) (`world/tally/mod.rs`, module notes).
- Which kinds? Ten, each its own file per season: `players`, `teams`, `games`, `classes`, `people`, `squads` (lane NB7), `calls` (lane NB8), `prospects`, `moves` and `money` (lane NB11) (`world/tally/mod.rs`, `KINDS`).
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
- Game splits (lane NB4): home, road, neutral site, rain or snow, wind 15+ mph, cold (40 F or below), indoors; college against a ranked or unranked team (the AP poll before kickoff); the NFL by opponent club (code 100 plus the club). The weather lines are proposed, not confirmed (`players.rs`, `SPLITS`, `game_codes`; `world/college.rs`, `ap_ranked`).
- His snaps on offense and defense in each game of `played` (`game_snaps`), which the opponent-adjusted numbers weigh games by.
- A start: on the field for his side's first snap with the ball (offense) or first snap without it (defense) (proposed, not confirmed; `world/tally/games.rs`, `starters`).
- High school players get no rows: their games are counted with facts and boxes only (proposed, not confirmed).

## The depth counts (lane NB4)

- Each player's season also keeps his depth counts: about 180 scalar counts and five grids, sparse by code, counted off every snap of his engine games at college and in the NFL (`world/tally/depth.rs`, `Dc`, `game`; `players.rs`, `Season::depth`).
- Scalars: passing on dropbacks (attempts, completions, yards, sacks, EPA, successes, scrambles, air yards, time to throw, tight windows, expected completions, throwaways, big-time and turnover-worthy throws), deep throws (20+ air yards), play action, the blitz and pressure; designed runs (yards before and after contact, expected yards, stuffs, runs of 10+ and 15+, missed tackles, eight-man boxes, fumbles, runs beating their expected yards); targets (catches, air yards, YAC and expected YAC, separation, contested, drops, routes, the team's targets and air yards in his games); blocking (pass and run snaps and wins, pressures, sacks, hits and hurries allowed, penalties, blown assignments); the pass rush and run defense (rushes, wins by speed, power or finesse, pressures, sacks, hits, hurries, get-off, time to pressure, unblocked, blitzes, run stops); coverage (targets, completions, yards, TDs, picks, breakups, separation, EPA, YAC, air yards, forced incompletions); tackles, misses, stops and box snaps; kicks, punts and returns (hang times, touchbacks, inside the 20, fair catches, returns, longest).
- Grids: throws by role (passer, target, man covering) x 4 depth rows x 3 thirds of the field x (attempts, completions, yards, EPA); runs by gap (5); targets by route (20); pressures by the blocker beaten (8 spots); facet grades (10 facets, sum and plays); snaps at each offensive and defensive spot; field goals by distance band (`THROW`, `GAP`, `ROUTE`, `PMAP`, `FACET`, `ALIGN_O`, `ALIGN_D`, `FG`).
- What the play record keeps for them: each man's spot on the snap, a kick's hang time, the time of the first pressure and each rusher's first contact with a blocker (`game/stats.rs`, `PlayRecord::roles`, `hang`, `pressure_at`, `contact`; `game/sim.rs`, `Notes::contact`). Counting only: no new draws.
- The charting delay: the open season keeps the last four days' games' depth counts (`Open::recent`), so a page can leave out the games your charting hasn't reached yet (`World::tally_depth_since`).
- Codes are kept by number, append only: a new count goes at the end of `Dc`.

## The teams file

- One row a program or club a season: games, wins, losses, points for and against, the advanced numbers for and against, penalties, and situation cells for both sides of the ball (`world/tally/teams.rs`, `Season`).
- Each cell keeps plays, yards (a kick's distance on kicks), EPA, successes, first downs, touchdowns, turnovers and one extra count by kind: completions on passes, sacks, missed tackles on runs, kicks made, touchbacks (`teams.rs`, `Situ`, `values`).
- Keys start `o|` (with the ball) or `d|` (without it), then the split: play kind, down, down and distance, field zone, quarter, two minute, score, red zone, goal to go, third and fourth down, personnel, backfield, motion, RPO, tempo, men in the box, defensive package, the defense's call, explosive; on dropbacks also blitz by down, pressured, play action, coverage and throw depth (`teams.rs`, `keys`).
- Plays wiped by a penalty and no-plays are left out of the cells (`teams.rs`, `game`).
- Lane NB8 keys: `garbage|kind` for snaps in garbage time (`tally/decide.rs`, `garbage`); on runs and dropbacks `cc|concept|coverage`, the play's concept against the coverage the defense played, and `cov3|coverage` on 3rd downs (`decide.rs`, `concept_of`, `coverage_of`; `teams.rs`, `keys`).
- Garbage time: in college and high school Bill Connelly's margins (more than 38 points in the 2nd quarter, 28 in the 3rd, 22 in the 4th, never the 1st or overtime); in the NFL the offense's win chance under 10% or over 90% (rbsdm.com's filter) (`decide.rs`, `garbage`).
- The concept is read from the call's words: a run's scheme (the longest run concept name the run part ends with, "other run" if none), a pass's concept (the part after "with" in an RPO; a library or designed play's name; "other pass" if none). The coverage is the defense call's first part ("Cover 3", "2-man", "Tampa 2") (`decide.rs`, `concept_of`, `coverage_of`; `game/playbook.rs`, `designed_named`).
- The charting delay (lane NB8): the open season keeps each team's latest game on its own (`Season::last`), so a page can leave it out while your staff is still charting it (`Season::charted_by`). It is cleared before the season's file is written (`tally_close`).

## The squads file (lane NB7)

- One small row a team a season at every level, high school too, so the team pages, leaderboards and the power rating read many seasons without the teams file's heavy cells (`world/tally/squads.rs`, `Season`).
- It keeps games, wins, losses and points; both boxes summed (`BOX`: the team's and its opponents', every box number); the advanced numbers for and against; the fumbles and passes defensed that expected turnovers read (`LUCK`); its kicks and returns (`ST`: punts with gross and net yards and inside the 20, kickoffs and touchbacks, kick and punt returns, field goals and extra points, kicks blocked); and every man who started a game, with his starts on offense and defense (`starters_by_unit`).
- College and the NFL also keep the offense's snaps, dropbacks and expected dropbacks in 21 situations (`TEND`: every snap, nflfastR's neutral early downs, down and distance bands, red zone, goal to go, the score, the last two minutes of a half, each quarter and overtime), for pass rate over expected (`tend_codes`; `world/tally/expect.rs`, `expected_pass`).
- Set when the season closes: the closing power rating from the season's scores (offense, defense, games; `world/power.rs`, `closing`) and what the roster brought: at college the recruiting stars of the men who took a snap on offense or defense, on the whole, offense and defense, how many, and the share rated four or five stars; at high school its regular starters (half its games or more) and the seniors among them (`world/tally/mod.rs`, `close_talent`; `squads.rs`, `TALENT_COLLEGE`, `TALENT_HS`, `regular`).
- Read with `World::tally_squad` (one team) and `World::tally_squads` (a season).

## The expected pass model (lane NB7)

- The chance a snap is a dropback, by the level's own play calling in that situation, as nflfastR's expected pass rate: logistic regression on the down, the log of the yards to go, two yards or less, the yards to the goal, inside the ten, the margin in touchdowns and the margin times the share of the game gone, the last two minutes of a half, the win chance and three yards or less on third or fourth down (proposed, not confirmed: the features; `world/tally/expect.rs`, `xpass_features`, `XPASS`).
- Fitted on the engine's own snaps at each level (high school, FBS, FCS, small colleges, NFL) with `xpass_fit -- 600 7` on 2026-11-04: every down-and-distance band within 2.5 points of the engine's own rate, the lead and deficit bands within 3.5 (`crates/afm-tools/src/bin/xpass_fit.rs`). Rerun after engine changes that move play calling.

## The games file

- Every engine game at every level: level, season, date, kind (regular, conference title, bowl, playoff, Super Bowl, state final and so on), the two teams, group (division or the NFL's conferences), neutral site, conference game, the score by quarter, overtime, weather and both team boxes (`world/tally/games.rs`, `GameRow`, `Facts`).
- College and NFL games also carry each side's advanced numbers (runs and run EPA and success, stuffs, line yards, dropbacks and pass EPA and success, havoc made, drives, trips inside the 40 and their TDs and field goals, three and outs (a drive that ends in a punt within four plays, the punt one of them; it counted three until minor bug 262, so almost none counted), average start; lane NB8 adds explosive plays (runs of 10+, dropbacks of 20+), the EPA of the successful snaps and red zone trips with their TDs and field goals), the home side's win chance before every scrimmage snap (0-100) and every player's line (`Adv`, `adv`, `wp_chart`, `Line`).
- The win chance is the game boards' model with the game's closing line where the books hung one, the home field otherwise (1.7 points in the NFL, 2.5 elsewhere, none at a neutral site) (lane NB8; `games.rs`, `wp_chart`; `decide.rs`, `Model::new`). Before NB8 it was each play's own `wp_before`.
- A line keeps the stat book's numbers: counts whole, rates and EPA to the hundredth (`games.rs`, `quantize`; `world/records.rs`, `Stat::fraction`).
- During the season the games are packed into the store as they come (zstd level 3, frames of about 256 KB), so the open season stays small (`world/tally/file.rs`, `LEVEL_OPEN`, `FRAME_RAW`; `games.rs`, `Open::add`).
- Games since a date at a level can be read back while the season runs (`World::tally_games_since`), which the charting delay uses.

## The calls file (lane NB8)

- One row a team a season at every level: its 4th downs and tries weighed against the analysts' model, its timeouts and its challenges (`world/tally/calls.rs`, `Season`, `Sums`, `game`).
- Sums: 4th downs, those weighed, went for it, converted, the times the model said go and went then, choices that agreed with the model, win chance lost; tries, two-point tries and those made, the times the model said two and went then, tries that agreed, win chance lost on tries; timeouts used and in the last two minutes of a half, timeouts left at the end of losses by 8 or less; challenges thrown and won; wrong rulings against it that a challenge would have overturned and the booth didn't review (`Sums`).
- A grid of every 4th down by yards to go (1, 2, 3, 4-5, 6-10, 11+) and spot (own 1-19, 20-39, 40-49, their 50-35, 34-20, 19-1): how many, how many it went for, how many the model said go (`GRID_ROWS`, `GRID_ZONES`, `grid_key`).
- College and NFL teams also keep each 4th down, try and challenge as a row: the game's key, date, opponent, site, quarter and clock, the lead, yards to go and the spot, what it did, the model's win chance going, punting and kicking (0-100, -1 where it is no choice) and the chance to convert, the play, what happened, who carried, caught or kicked, and the final score (`Fourth`, `Try`, `Chal`). High school keeps the sums and grid only.
- 4th downs: runs, dropbacks, punts and field goals on 4th down; kneels and spikes are left out, and so is college and high school overtime, which starts from a spot with no clock (`calls.rs`, `game`; `decide.rs`, `Model::untimed`).
- The model agrees when the choice is within 1 point of win chance of its best; it says go when going beats the next choice by 1 point or more (proposed, not confirmed; `calls.rs`, `CLEAR`, `judge`, `judge_try`).
- Rows and sums agree: each 4th down is weighed at the precision its row keeps (0.1 of a point), and `sums_of` gives the sums back from the rows, which the pages use to leave out games not charted yet (`calls.rs`, `kept`, `sums_of`).

## The analysts' model (lane NB8)

- Every 4th down is weighed as nfl4th weighs the NFL's: the win chance after going (converting at the level's rate from that distance and gaining the level's average yards on a conversion, else the other side's ball at the spot), punting (the other side's ball where the level's punts from that spot leave it) and kicking (made at the level's rate from that distance, else the other side's ball at the spot of the kick, at least its own 20) (`tally/decide.rs`, `Model::fourth_at`, `hand_over`, `after_td`, `fg_make`).
- The win chance is the game boards' model (`game/win_chance.rs`, `chance`) with the level's numbers for the season and the game's line. Each choice takes six seconds off the clock (proposed, not confirmed; `PLAY_SECS`).
- Tries: the win chance kicking the extra point and going for two, each made at the level's rate, then a kickoff (`Model::try_after`).
- Its tables are fitted on the match engine's own plays: 3rd and 4th down conversion and the gain on a conversion by yards to go (open field and goal to go, garbage time and overtime left out), where a punt leaves the other side by the punt's spot, field goals by distance, extra points and two-point tries, at each level (`crates/afm-tools/src/bin/decide_fit.rs`; `decide.rs`, `CONV`, `CONV_GOAL`, `GAIN`, `PUNT`, `FG`, `XP`, `TWO`). Rerun the fit after engine changes that move them.
- Post-game win expectancy: a logistic fit of winning on the difference of the two sides' five factors (success rate, explosiveness, field position, points a trip inside the 40, turnovers) plus the home field, fitted on the same engine games (`decide.rs`, `pgwe`, `pgwe_features`, `five`, `PGWE`). A neutral site drops the home field.

## The classes file

- Every draft pick since recorded history began, and every Division I (FBS and FCS) signee straight from high school, each with what became of him so far: college and NFL seasons, games, starts and snaps, his draft slot, transfers and the season counted through (`world/tally/classes.rs`, `Pick`, `Signee`, `Outcome`).
- Signees join their class when the season of their high school class closes; junior college players and players without a Division I first program are left out (proposed, not confirmed; `classes.rs`, `Store::close`).
- Each season's file is the whole store as that season closed, so a page reads one file for every class (`classes.rs`, module notes; `tally_close`).
- Honors (All-America, Pro Bowls, All-Pro) are left to the world's own books (`StatBook::honors`, `NflBook`).

## The prospects file (lane NB11)

- Two kinds of record (`world/tally/prospects.rs`, `Row`). A kid's season (`Prospect`, keyed by his id): each high school game he played on the match engine with his line (date, school, opponent, home, away or neutral, the score, whether he took a snap, his counts in `HsStat` codes, sacks and tackles for loss in halves) and the game's key in the season's games file; and each time the services ranked his class that season (stars, national, position and state rank, the composite rating in ten-thousandths).
- A Division I signee's last ranking before he enrolled (`Signed`, keyed after every kid by class and id), written the season his class joins the classes file, so class rankings can be worked out on the composite as the services do (`Open::close`).
- Who is kept: a high schooler or prep school player with stars from the services or a college offer on the day of the game (proposed, not confirmed: the kids recruiting analysts follow; every high schooler would be about ten times the size) (`followed`). The services' lists are kept for every ranked kid.
- The game lines come from the high school day (`world/hs_year.rs`, the engine game's `lines`; `World::tally_hs_lines`); the rankings from each services update (`world/scouting/services.rs`, `record`; `World::tally_ranks`, which reads `class_ranks` and `composite`). A same-day update replaces that day's entry.
- Each ranked kid's latest ranking carries from season to season until his class has signed and gone (`Open::last`).

## The moves file (lane NB11)

- Every transfer portal entry since recorded history began, each with what came of it: the offseason, the day he entered and decided, the old program and division, where he landed (none if he found no school or went back), why, his role the season before, position and stars, his season at the old school from the stat book (games, offensive and defensive snaps) and his seasons at the new one from the players file (games, starts, snaps) (`world/tally/moves.rs`, `Move`).
- The entries are the world's own portal records (`World::recruiting_log`); the store runs in the world and the save like the classes, and each season's file is the store as that season closed (`Store::close`; keys by offseason, then order).
- A world from before the lane takes in every portal record it holds at its first close.

## The money file (lane NB11)

- What each team paid in a season, kept when it closes (`world/tally/money.rs`, `rows`):
- an NFL club: the cap, the carryover, the cap charges, dead money, the cash paid that league year and its players on the books, each with his cap hit, the cash he was paid that league year and his deal's average a year (`NFL_VALS`, `Pay`);
- a college program with money: the collective's budget, the school's revenue share inside it from July 2025 (`ncaa::school_pot`), the boosters, what was paid and each player's money that year split into the school's share and the collective's deals in the program's ratio, with his brand deals apart (`COLLEGE_VALS`; proposed, not confirmed: the split by the program's ratio);
- a high school: its budget and books so far line by line (`finance::hs::LINES`), the fee to play and the crowd (`HS_EXTRA`).
- The season being played is worked out from the world when a page asks (`World::tally_money`, `tally_money_level`).

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
- The worlds page carries the folder as a tar file in parts of 1.9 GB (`name.afmdata.tar.001`, ...), packed as the folder is removed, and the download fetches it with the world (`.github/workflows/build-world.yml`, publish step; `crates/afm-game/src/jobs.rs`, `data_assets`). Publishing it as a tar is proposed, not confirmed.

## Charting depth and delay (item 2)

- Your analytics head's rating sets how deep your charting goes: 15 and up gives tracking-style charting (separation, run block and pass rush wins), 8 and up film charting (CPOE, depth of target, pressures, missed tackles, coverage, grades), less play-by-play numbers only (EPA, success and the box) (proposed, not confirmed; `world/analytics.rs`, `charting`, `depth`).
- And how soon it arrives: 17 and up the morning after a game, 12 and up two days after, 7 and up three days, less four (proposed, not confirmed; `charting`).
- It only changes what you see: the card and table leave out the numbers past your depth, and this season's lines leave out the games not charted yet (`crates/afm-game/src/lane_data/analytics.rs`, `card_of`, `charted_now`).
- The NFL's own tracking reaches every seat at full depth the morning after (proposed, not confirmed; `crates/afm-game/src/lane_data/mod.rs`, `charting`).
- The Sources line names your analytics head and says what you get and when (`lane_data/mod.rs`, `sources`). The rating itself is never shown.

## Sizes

- Measured with `memory_check` on a 0.1 world (seed 2026, 6 years of history and one more season, 4-core Linux), 2026-11-04, after lane NB4's depth counts. A closed season is about 30.5 MB on disk: players 14.5 MB (the depth counts add about 8.4 MB to the 6.1 MB before them), teams 6.2 MB, games 9.4 MB (2,603 college, 285 NFL, about 6,850 high school), classes 0.6 MB and growing about 0.1 MB a season (the store holds every class), people 0.12 MB.
- In the games file a college game is about 2.6 KB packed (lines and the win chance), an NFL game 2.2 KB, a high school game 0.3 KB (facts and boxes).
- The open season at its biggest (the end of the football year) packs to 85 MB before compression (63 MB before the depth counts); just after the close the tallies weigh 5 MB of heap. Memory stays flat from season to season: closed seasons live only on disk. The whole world's heap was 0.91 GB after the future season and its peak resident 1.67 GB.
- The squads file (lane NB7, not measured): a college or NFL row is about 2.5 KB before compression (the boxes, the 21 situations and some 30 starters), a high school row about 1.5 KB; at 0.1 that is roughly 1.5 MB a season before compression and well under 1 MB on disk (Claude's estimate from the row's fields).
- At full size (not measured): college and NFL counts barely change with scale, high school games are about ten times as many and college player rows somewhat more, so a season comes to roughly 55-65 MB and a 30-year world's folder to about 1.8 GB; the open season adds roughly 0.2 GB of heap at the football year's end, after the summer's high (Claude's estimate from the 0.1 numbers).

- Lane NB11's files (not measured, Claude's estimate from the rows): prospects about 4 MB a season at full size before compression (about 20,000 followed kids, a dozen games and a few rankings each), moves about 0.6 MB and growing about 0.03 MB a season (the store holds every move), money about 1.5 MB (some 1,700 NFL contracts, the programs' players with money and 16,000 high school budgets).

## Where the knobs are

- `world/tally/file.rs`: `FRAME_RAW` (256 KB), `LEVEL_OPEN` (3), `LEVEL_CLOSE` (9).
- `world/tally/players.rs`: `SPLITS`, `PASS`, `RUSH`, `RECV`, `DEF`, `RECENT_DAYS`.
- `world/tally/depth.rs`: `Dc` and the grids; `world/tally/expect.rs`: the expected rush and YAC models.
- `world/tally/people.rs`: `STATS`, `UP`.
- `world/tally/decide.rs`: the analysts' tables (`CONV`, `CONV_GOAL`, `GAIN`, `PUNT`, `FG`, `XP`, `TWO`, `PGWE`, refitted by `decide_fit`), `PLAY_SECS`, the garbage time lines (`garbage`).
- `world/tally/calls.rs`: `CLEAR` (1 point of win chance), the grid's rows and zones.
- `world/tally/squads.rs`: `BOX`, `LUCK`, `ST`, `TEND`, `TALENT_COLLEGE`, `TALENT_HS`, `regular` (half its games); `world/tally/expect.rs`: `XPASS`.
- `world/analytics.rs`: `charting` (depth and delay by rating), `depth` (which numbers need which depth).
- `world/tally/prospects.rs`: `followed` (who is kept); `world/tally/money.rs`: `NFL_VALS`, `COLLEGE_VALS`, `HS_EXTRA` (append only).

## Related pages

- [records-and-data.md](records-and-data.md): the record books and Player analytics.
- [world-log.md](world-log.md): the world log, the trial tooling's full record.
- [saves.md](saves.md): the world and save files.
