# Saves and world files: how it works today

Checked against main at d41bed5 on 2026-10-28 (update 3, lane 19). The tallies folder beside each save over main a0dfde3 on 2026-11-04 (lane NB1). Deleting worlds over main 4d881d5 on 2026-11-06. The Jan 1 start (lane A) checked on 2026-11-08, before it merged to main. The notebook beside each save over main 5247199 on 2026-11-12.

## Quick answers

- What is in a save? The whole `World`, every random stream included, so a loaded save plays on exactly as the world it came from (`save.rs`, `write_world`).
- What is left out? The trial game log (`World::game_log`, `#[serde(skip)]`) and other skipped caches that `World::after_load` rebuilds (`world/seat.rs`, `after_load`).
- Which format does the game write? Format 2, in parts (`save.rs`, `FORMAT_VERSION`).
- Do old files still open? Yes: format 1 (one zstd stream, every file before update 3) loads through the same `read_world` (`save.rs`, `read_world`).
- Does a new save open in an older game? No: an older game refuses format 2 with "update the game" (`save.rs`, `header`).
- How is the world encoded? Named MessagePack (field names kept), so a newer game reads an older file by name; a new field needs `#[serde(default)]` (`save.rs`, module notes).
- Which compression? zstd level 3, through a 1 MB buffer both ways (`save.rs`, `LEVEL`, `BUFFER`).
- How many cores does a save use? All of them: each part is packed on its own core, one more thread writes them (`save.rs`, `on_every_core`, `write_world`).
- How many cores does a load use? All of them for format 2: every part is read on its own file handle (`save.rs`, `read_parts`). A format 1 file reads on one core.
- What sits beside a save? Its world log (`name.afmlog`) and the Data section's tallies (`name.afmdata`, one file per kind and closed season; the season being played is in the save). Both are linked from the game folder's live copies, not copied (`crates/afm-game/src/tallies.rs`, `on_save`; [tallies.md](tallies.md)).
- Where is the notebook (the top bar's notes)? In its own small JSON file beside the save, `name.afmnotes`, so the save format is untouched and old saves load as before. Before a career's first save it is `notebook/unsaved.afmnotes` in the game folder; the first save moves it beside the save, a "save as" copies it, a rename and a delete follow the save, and a new world or a loaded save clears the unsaved file (`crates/afm-game/src/notebook.rs`, `on_open`, `on_save`, `on_rename`, `on_delete`); the notes on players, coaches and letters stay in the save (`crates/afm-game/src/lane_core/desk.rs`, `note_write`).
- Is a note written when I make it? Yes: each add, edit or delete rewrites the file at once (written to `.partial`, then renamed), not only when you save. A file that can't be read is kept as `.afmnotes.broken` and the notebook starts empty (`notebook.rs`, `write`, `read`). Caps: 2000 notes, 4000 letters a note, 40 categories of 32 letters (`MAX_NOTES`, `MAX_TEXT`, `MAX_CATEGORIES`, `MAX_CATEGORY_LEN`).
- Can the game delete them? Saves: Load game, Delete (`setup.rs`, `delete_save`). Library worlds: the World step, "Delete this world…", which takes its card, log and tallies beside it and never a save (`setup.rs`, `delete_world`). Both only touch files in their own folder of the game folder (`~/Library/Application Support/AFM` on a Mac).
- What is a played-on world? A library world the game played on from February 9 to Jan 1 of its next winter (the World step's Jan 1 start), kept as its own file `<name>-jan<year>.afmworld` with its card and tallies (hard-linked from the old one, then the season added) and no world log; the old world stays as it was. Its start year, history years and high school engine years each go up one, as the season it played is now its history's last (`crates/afm-game/src/jobs.rs`, `play_on`, `play_on_file`). The card says `jan1` when the world stands on Jan 1 (`cards.rs`).
- Is a half-written save possible? No: the game writes `name.partial` and renames it over the old save when done (`crates/afm-game/src/files.rs`, `write_world`).
- Is the file the same bytes each time? No: parts land in the order they finish and hash maps keep no fixed order. The table says where each part is (`save.rs`, `write_world`).
- How long does it take? On the 30-year world at the first game, 4-core Linux: save 17-18 s, load 14.4 s with `after_load`, peak 3.77 GB (`docs/game.md`, "Lane 19").

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
