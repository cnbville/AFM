# The world reader's R8 definitions

World reader lane R8 wrote its panels, checks against real, consistency rules and metrics as Python, which `run_gen.py` turns into `crates/afm-reader/src/defs_games.rs`, `defs_nfl.rs` and `defs_world.rs`.

- `lib.py`: `panel`, `check`, `rule`, `metric`, `col` and the real-figure helpers (`Key`, `Fixed`, `Memory`, `NoReal`), and the Rust writer.
- `common.py`: shared SQL pieces (team names, the game log's views).
- `shapes.py`: each consistency rule's failures as the four columns the book wants.
- `g_a.py` to `g_d.py`: games and stats (G1-G38). `g_n.py`: the NFL (N1-N29). `g_w.py`, `g_x.py`, `g_y.py`: money, world and time (N30-N44, N48, T9).

Edit the Python, then run `python3 tools/reader-defs/run_gen.py` from anywhere and `cargo fmt`. An edit made straight to the `.rs` files is lost the next time the generator runs.

## Lane R7

`r7/` holds lane R7's definitions the same way: `programs.py` (Programs, Money's C14, the conference pages), `hs.py` (High school), `coaches.py` (Coaches and the NFL page's S34), `common.py` (shared SQL pieces) and `gen.py`, which writes `crates/afm-reader/src/defs_programs.rs` and runs `rustfmt` on it. Run `python3 tools/reader-defs/r7/gen.py` from anywhere.
