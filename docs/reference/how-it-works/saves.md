# Saves and world files: how it works today

Checked against main at d41bed5 on 2026-10-28 (update 3, lane 19). The tallies folder beside each save over main a0dfde3 on 2026-11-04 (lane NB1). Deleting worlds over main 4d881d5 on 2026-11-06. The Jan 1 start (lane A) checked on 2026-11-08, before it merged to main. The notebook beside each save over main 5247199 on 2026-11-12; per career, never shared, over main 8e62944 on 2026-11-13. The plan ahead file beside each save over main 0ca3b43 on 2026-11-14. The autosave over main ddc308e on 2026-11-14. A newer world on an older game over main fc57156 on 2026-11-15. Going back to an older version added on 2026-11-16 (before it merged to main). Days wait for a running autosave added on 2026-11-17 over main 75c5a35. Opening closes the open world first, over main dd1fdf1 on 2026-11-17. People kept in parts (memory diet lane M1) added on 2026-11-17 over main b8438f5, before it merged.

## Quick answers

- What is in a save? The whole `World`, every random stream included, so a loaded save plays on exactly as the world it came from (`save.rs`, `write_world`).
- What is left out? The trial game log (`World::game_log`, `#[serde(skip)]`) and other skipped caches that `World::after_load` rebuilds (`world/seat.rs`, `after_load`).
- Which format does the game write? Format 2, in parts (`save.rs`, `FORMAT_VERSION`).
- Do old files still open? Yes: format 1 (one zstd stream, every file before update 3) loads through the same `read_world` (`save.rs`, `read_world`).
- Does a new save open in an older game? No: an older game refuses format 2 with "update the game" (`save.rs`, `header`).
- What does an older game say about a world from a newer one in the same format? A newer world can hold what the older game has never heard of (a new career stage such as `CflPool`), and the reader stops on it. Every world file's header carries its writer's code time, the time of the commit the engine was built from (`code`, from `save::CODE_TIME`, stamped by `crates/afm-engine/build.rs`; files before 2026-11-15 have none). When a world won't load, the game compares the writer's code time with its own: later, it says the world was made by a newer version of AFM and to update the game; the same or earlier (a save this game wrote that won't load is a bug), it says the file is damaged or unreadable. A file without the stamp is judged by its "written" time instead, and with no code time at all (a build without git) the game says the world was most likely made by a newer version. The reader's own words follow, short (`crates/afm-game/src/files.rs`, `unreadable`; test `a_world_from_a_newer_game_says_update`). Games before this change show the reader's error as it is.
- How is the world encoded? Named MessagePack (field names kept), so a newer game reads an older file by name; a new field needs `#[serde(default)]` (`save.rs`, module notes).
- Which compression? zstd level 3, through a 1 MB buffer both ways (`save.rs`, `LEVEL`, `BUFFER`).
- How many cores does a save use? All of them: each part is packed on its own core, one more thread writes them (`save.rs`, `on_every_core`, `write_world`).
- How many cores does a load use? All of them for format 2: every part is read on its own file handle (`save.rs`, `read_parts`). A format 1 file reads on one core.
- What sits beside a save? Its world log (`name.afmlog`) and the Data section's tallies (`name.afmdata`, one file per kind and closed season; the season being played is in the save). Both are linked from the game folder's live copies, not copied (`crates/afm-game/src/tallies.rs`, `on_save`; [tallies.md](tallies.md)).
- Where is the notebook (the top bar's notes)? In its own small JSON file beside the save, `name.afmnotes`, so the save format is untouched and old saves load as before. Before a career's first save it is `notebook/unsaved.afmnotes` in the game folder; the first save moves it beside the save, a "save as" copies it, a rename and a delete follow the save, and a new world or a loaded save clears the unsaved file (`crates/afm-game/src/notebook.rs`, `on_open`, `on_save`, `on_rename`, `on_delete`); the notes on players, coaches and letters stay in the save (`crates/afm-game/src/lane_core/desk.rs`, `note_write`). The notebook is per career, never shared: each save reads and writes only its own file (`notebook.rs`, `path_for`; test `each_career_has_its_own_notebook`), and the screen drops its search, category filter and notes when you leave the career (`app/ui/screens/notebook.js`, `notebookForget`).
- Where is my plan ahead on the depth chart? In its own small JSON file beside the save, `name.afmplan` (before the first save `depthplan/unsaved.afmplan` in the game folder), written at once on every drop, and never in the save or the world, so nothing in the game can read it. It follows save, save as, rename and delete like the notebook (`crates/afm-game/src/depth_plan.rs`, `on_open`, `on_save`, `on_rename`, `on_delete`). What it holds and how a planned fall starts: [docs/game.md](../../game.md), "Plan ahead on the depth chart".
- Is a note written when I make it? Yes: each add, edit or delete rewrites the file at once (written to `.partial`, then renamed), not only when you save. A file that can't be read is kept as `.afmnotes.broken` and the notebook starts empty (`notebook.rs`, `write`, `read`). Caps: 2000 notes, 4000 letters a note, 40 categories of 32 letters (`MAX_NOTES`, `MAX_TEXT`, `MAX_CATEGORIES`, `MAX_CATEGORY_LEN`).
- Can the game delete them? Saves: Load game, Delete (`setup.rs`, `delete_save`). Library worlds: the World step, "Delete this world…", which takes its card, log and tallies beside it and never a save (`setup.rs`, `delete_world`). Both only touch files in their own folder of the game folder (`~/Library/Application Support/AFM` on a Mac).
- What is a played-on world? A library world the game played on from February 9 to Jan 1 of its next winter (the World step's Jan 1 start), kept as its own file `<name>-jan<year>.afmworld` with its card and tallies (hard-linked from the old one, then the season added) and no world log; the old world stays as it was. Its start year, history years and high school engine years each go up one, as the season it played is now its history's last (`crates/afm-game/src/jobs.rs`, `play_on`, `play_on_file`). The card says `jan1` when the world stands on Jan 1 (`cards.rs`).
- Does the game save by itself? Yes, about every 10 minutes of play, in the background: the screens ask `autosave_tick` every 30 seconds, and once 10 minutes have passed since the last save of any kind and something was done since, the game forks itself and the copy writes the file while the game goes on (`crates/afm-game/src/autosave.rs`, `tick`, `start`, `child`; `app/ui/app.js`, `autosaveTick`). See "Autosave" below.
- Does opening a save while another world is open hold both? No (fixed 2026-11-17): `open_world` and `load` drop the world you had open (and end a running autosave) before they read the file, so a 30-year world (about 6 GB on the seed 181 world) is never in memory twice. Before, the new file was read first, which needed about 12 GB and made seed 181 saves fail to load on an 8 GB Mac. A file that can't be read now leaves no world open, and the start pages forget the closed career (`crates/afm-game/src/lib.rs`, the `open_world` arm; `app/ui/pregame.js`, `afterFailedOpen`; test `open_order`). The screens ask you to close an open career first (`leaveCareer`).
- Is a half-written save possible? No: the game writes `name.partial` and renames it over the old save when done (`crates/afm-game/src/files.rs`, `write_world`).
- Is the file the same bytes each time? No: parts land in the order they finish and hash maps keep no fixed order. The table says where each part is (`save.rs`, `write_world`).
- How long does it take? On the 30-year world at the first game, 4-core Linux: save 17-18 s, load 14.4 s with `after_load`, peak 3.77 GB (`docs/game.md`, "Lane 19").

## People in files (memory diet lane M1)

- In memory a person keeps his college-and-later fields (`Later`) and the rest of his calendar (`CalMore`) only once he uses them (`world/stored.rs`, `Stored`). In files nothing changed: a person and his calendar are written field by field, in the old order and shape, through a flat view (`world/person_form.rs`, `view::Person`, `view::PersonCal`), and read through a flat file form (`file::Person`, `file::PersonCal`) that is split as it comes in. A part that reads blank is not kept (`Stored::new`).
- So any file opens in this game, and the game's files open in older games of format 2 as before. The world log's store prints and the World Reader's tables see the same fields.
- A new field on `Person` or `PersonCal` goes in three places: the struct (or `Later`/`CalMore`), `file::*` and `view::*`; the conversions name every field, so a field missing from one doesn't build. `tests/compact.rs` holds the field order files have.

## Autosave

- Why a fork? A copy of the world is 3 to 4 GB, too much for an 8 GB Mac. The forked child shares every page with the game until one of them changes it, so the child sees the world exactly as it stood at the tick and costs memory only for what the game changes while it writes (`autosave.rs`, module notes).
- When does the snapshot happen? Inside the `autosave_tick` command, so between two commands, never in the middle of a day; the tick answers at once (`autosave.rs`, `tick`).
- What is written? `saves/Autosave - <coach name>.afmsave`, one per coach, replaced each time through the same `write_world` as a save by hand (`name.partial`, then a rename). The world log and tallies are linked beside it and the notebook and depth plan copied, all at the moment of the tick; a career's face pictures are not (`autosave.rs`, `start`; `notebook.rs` and `depth_plan.rs`, `on_autosave`).
- When does it start? 10 minutes after the last save of any kind (or the career's start), only if some command other than the idle ones ran since, never while a world is being built, downloaded or played on, and only in a career (`autosave.rs`, `EVERY`, `is_idle_call`, `job_running`).
- Never two at once: a running autosave blocks the next, and a save by hand, a bug pack, opening a world or starting a career ends it (SIGKILL, its `.partial` removed) before going on (`lib.rs`, `autosave.cancel`, `autosave.reset`). Loaded or started, the clock starts again.
- If it fails or takes over 20 minutes: the old autosave stays, the sidebar says "Autosave failed" and it tries again after the next 10 minutes (`autosave.rs`, `STUCK`).
- Priority: the child runs at nice 10 (`autosave.rs`, `NICE`).
- Days wait for it: Continue, a day and a holiday wait until a running autosave is written before they play, up to 90 seconds; past that the autosave is ended (its old file kept) and the days play (`autosave.rs`, `plays_days`, `finish_first`, `WAIT`; `lib.rs`, `call`, and the `continue` arm, which waits only once no meeting stops it, so a meeting's answer comes at once). Why: a day changes about half the world, and every page the game changes while the child still shares it is copied. Measured 2026-11-17 on the 30-year world (Linux, 4 cores): one Continue during an autosave copied 1.9 GB and took 11.7 s instead of 0.2 to 1 s, which on an 8 GB Mac sends the game into swap and froze the app for minutes. Screens that only read change a few MB and don't wait. The Continue button says "Finishing the autosave…" while it waits (`app/ui/app.js`, `cont`).
- On the screen: one quiet line over the Save button, "Saving in the background", then "Autosaved 14:32" (`app/ui/app.js`, `autosaveHtml`).
- Where it does not run: not on Windows (no fork); the tick answers `off`.

## Format 2

- Header: `AFMWORLD`, the version as a little-endian u32, a u32 length and that much JSON (seed, history years, start year, commit, written, world date, the career and the game's card) (`save.rs`, `WorldMeta`).
- Then a u32 count and the table: per part an 8-byte name, the item count, the offset and the length in bytes, each a little-endian u64 (`save.rs`, `ENTRY`, `Part`).
- The parts: `rest` (the world with the lists below written empty), `alumni`, `news`, `seasons` (the stat book's player seasons, 16 pieces) and `people` (128 pieces, in order). Each is its own zstd stream (`save.rs`, `Kind`, `PEOPLE_PIECES`, `SEASON_PIECES`).
- The lists are left out of `rest` by `#[serde(serialize_with = "crate::save::in_parts")]` on `World::people`, `alumni`, `news` and `StatBook::seasons`; it writes them empty only on the thread packing `rest` (`save.rs`, `in_parts`, `IN_PARTS`).
- Writing: the parts are packed on every core, the big single parts first; a writer thread writes each one whole where the file has got to, with at most two packed parts waiting; the table is filled in last (`save.rs`, `write_world`).
- Reading: the table, then every part on every core, longest first. People and seasons go straight into one list each: the list's room is cut into one run per piece, and a piece that is shorter or longer than the table says is an error that drops what it read (`save.rs`, `read_parts`, `Fill`).
- Static text (`&'static str` fields) is kept once per program: each reading thread remembers what it has met, then the shared pool (`save.rs`, `intern`, `pooled`).

## Format 1

- Header as above, then the whole world as one zstd stream of named MessagePack. Read through the 1 MB buffer (`save.rs`, `read_world`).

## Where the knobs are

- `save.rs`: `LEVEL` (3), `BUFFER` (1 MB), `PEOPLE_PIECES` (128), `SEASON_PIECES` (16), the writer's queue of 2 in `write_world`.
- `notebook.rs`: `MAX_NOTES` (2000), `MAX_TEXT` (4000), `MAX_CATEGORIES` (40), `MAX_CATEGORY_LEN` (32).

## Related pages

- [README.md](README.md)
- [tallies.md](tallies.md)

## Going back to an older version

- Where do I go back to an older build? Settings, "Other versions…" (Mac app): the list of installable builds with Install on each (`app/ui/versions.js`; the install itself is `install_version` in `app/src-tauri/src/main.rs`).
- Which builds are listed? Those whose release on the AFM repository carries a `downgrade.json`, the signed update pointed at that release's own app file; `.github/scripts/mac-publish.sh` writes one for each build that has an app file (builds before 486 have none).
- Are my saves safe? The page keeps every file of the saves folder in `saves-backup/before-<version>-<time>/` first: `.afmsave` files by hard link (saves are written to a temporary file and renamed, never in place, so the old bytes survive being written over), other files copied up to 64 MB (`crates/afm-game/src/versions.rs`, `backup`; test `the_backup_keeps_a_save_whole_when_a_newer_one_is_written_over_it`). Nothing in `saves/` is moved.
- How does it know what the older build can't open? A save's or library world's header holds the writer's code time (`code`); later than the target build's commit time means newer (`versions.rs`, `check`; test `the_check_names_what_a_build_cannot_read`). Files before 2026-11-15 have no stamp and are never counted. This is a warning only; the older game's own refusal is in the answers above.
- Do updates undo it? Not on a build that has this change: going back writes `update-pin.json` in the game folder (`versions.rs`, `pin_set`) and `checkUpdate` (`app/ui/app.js`) skips while it names the build running. A build from before this change doesn't read it. The downloads page can be held at a build (`.github/app-hold`, `mac-publish.sh`, `app-hold.yml`); there is no hold unless a build number is in that file.
