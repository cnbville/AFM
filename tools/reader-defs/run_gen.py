import importlib, lib, os, shapes
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../crates/afm-reader/src/")
GROUPS = [
    ("defs_games.rs", ["g_a", "g_b", "g_c", "g_d"], "//! World reader lane R8: the games and stats page (G1-G38): panels, checks against\n//! real, consistency rules and distributions over the game log and the stat\n//! books. Written by `tools/reader-defs/run_gen.py`: edit the Python there and\n//! run it again. Each item says what it reads and why."),
    ("defs_nfl.rs", ["g_n"], "//! World reader lane R8: the NFL page (N1-N29): franchises, seasons, the\n//! schedule, rosters, careers, contracts and the market, trades, the draft\n//! and scouting, agents, other leagues, discipline and coaches' lives. Written by\n//! `tools/reader-defs/run_gen.py`: edit the Python there and run it again."),
    ("defs_world.rs", ["g_w", "g_x", "g_y"], "//! World reader lane R8: the money page (N30, N31, N33), high school budgets\n//! (N32) and the world and time page (N34-N44, N48, T9): club and department\n//! books, betting lines, dates, rule books, events, weather, births, the\n//! calendar log, news, stories, traditions, media, storylines and pop culture. Written\n//! by `tools/reader-defs/run_gen.py`: edit the Python there and run it again."),
]
for path, mods, doc in GROUPS:
    for L in (lib.P, lib.C, lib.R, lib.M, lib.COLS): L.clear()
    for m in mods:
        importlib.import_module(m)
    if path == "defs_games.rs":
        import common  # its columns were registered on first import
    shapes.apply()
    lib.gen(OUT + path, doc)
    print(path, len(lib.P), "panels", len(lib.C), "checks", len(lib.R), "rules", len(lib.M), "metrics", len(set(lib.COLS)), "columns")
