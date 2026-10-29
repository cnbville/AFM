# Saves and world files: how it works today

Checked against main at d41bed5 on 2026-10-28 (update 3, lane 19).

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

## Related pages

- [README.md](README.md)
