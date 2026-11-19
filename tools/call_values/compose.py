#!/usr/bin/env python3
"""The coaches' call table while minor bug 405 is open (scheme-fit lane SF1, 2026-11-17).

Rerun on main, `call_values`'s straight pass cells had drifted from the table, and its two fixed
teams throw a storm of picks into the Cover 1 robber, so the straight pass rows wait for lane SF5 or
SF10. This builds the table from the committed one and a fresh run of the tool:

- the straight pass rows stay as committed, but the three play-action concepts, which SF1 changed,
  come from the new run;
- each faked row ("play-action <name>") is its committed straight row plus what the fake changes on
  the same snaps (the tool plays each concept with and without the fake on the same seeds);
- the run rows come from the new run.

Usage: python3 tools/call_values/compose.py <committed.csv> <new run.csv> <out.csv>
(the new run: `cargo run --release --bin call_values -- 2000 > new.csv`). Once 405 is fixed, write the
tool's output straight to crates/afm-engine/data/call_values.csv again.
"""
import csv
import sys

PA = ("play-action shot", "play-action boot", "play-action dig")


def load(p):
    rows = list(csv.reader(open(p)))
    return rows[0], [r[0] for r in rows[1:]], {r[0]: r[1:] for r in rows[1:]}


def main():
    hm, om, m = load(sys.argv[1])
    hn, on, n = load(sys.argv[2])
    assert hm == hn, "headers differ"
    out = [hm]
    for k in on:
        row = n[k]
        if row[0] == "" or k in PA:
            # A run row (no pass cells), or a play-action concept.
            out.append([k] + row)
        elif k.startswith("play-action "):
            base = k[len("play-action "):]
            cells = ["" if x == "" else f"{float(x) + float(y) - float(z):.2f}" for x, y, z in zip(m[base], row, n[base])]
            out.append([k] + cells)
        else:
            out.append([k] + m[k])
    missing = [k for k in om if k not in on and not k.startswith("play-action ")]
    assert not missing, missing
    with open(sys.argv[3], "w", newline="") as f:
        csv.writer(f, lineterminator="\n").writerows(out)


if __name__ == "__main__":
    main()
