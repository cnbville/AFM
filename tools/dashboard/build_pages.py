#!/usr/bin/env python3
"""Build the player pages page from world trial cards.

Usage: build_pages.py TEMPLATE.html OUT.html TITLE RUN_DIR [RUN_DIR ...]

TEMPLATE is an earlier player pages file (for example
docs/trials/artifacts/world-trial5-player-pages.html): its `const CARDS = [...]`
line is replaced by the cards of the given runs. Each RUN_DIR holds the
cards.json that world_trial writes with AFM_CARDS; the world's seed is read
from the folder name (seed-2026) and put in front of each card's tag.
Trial and debug tooling only: the cards carry hidden potential.
"""

import json
import os
import re
import sys


def main():
    if len(sys.argv) < 5:
        sys.exit(__doc__)
    template, out, title, runs = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
    cards = []
    for run in runs:
        seed = re.sub(r"\D", "", os.path.basename(os.path.normpath(run))) or run
        for c in json.load(open(os.path.join(run, "cards.json"))):
            tag = c.get("tag") or ""
            if not tag.startswith("World "):
                c["tag"] = f"World {seed} · {tag}" if tag else f"World {seed}"
            cards.append(c)
    lines = open(template).read().split("\n")
    done = False
    for i, line in enumerate(lines):
        if line.startswith("const CARDS = "):
            lines[i] = "const CARDS = " + json.dumps(cards, separators=(",", ":")) + ";"
            done = True
        elif line.startswith("<title>"):
            lines[i] = f"<title>{title}</title>"
    if not done:
        sys.exit("template has no `const CARDS = ` line")
    open(out, "w").write("\n".join(lines))
    print(f"{len(cards)} cards -> {out}")


if __name__ == "__main__":
    main()
