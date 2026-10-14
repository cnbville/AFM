# Debug dashboard v2 (AFM Debug Lab)

Trial and debug tooling only: it shows hidden values (potential, coach ratings).

1. Run world trials with the raw export on:
   `AFM_RAW=runs/seed-2026/raw AFM_DASH=runs/seed-2026/dash.json AFM_CARDS=runs/seed-2026/cards.json cargo run --release --bin world_trial -- 2026 30 > runs/seed-2026/report.txt`
   (about 35 minutes per 30-year world on 4 cores). The raw export is every college game's box score, every player season, play-level situation totals (league and FBS team), lineups, programs, coaches, alumni, active players, NFL rosters and news, as JSON lines. `gzip` them to keep them; the builder reads `.jsonl` or `.jsonl.gz`.
2. Build the dashboard data: `python3 tools/dashboard/build_data.py OUT/data runs/seed-2026 runs/seed-2027 runs/seed-2028`. Writes `seed-<n>.json` per world, `real.json` (real-football references, each with its source) and `index.json`. Optional: `OUT/data/notes.json` with Claude's sanity-check notes (`ideas` and `missing` lists) for the Idea board.
3. Copy `index.html` to `OUT/` and open it through any static server, or publish it as an artifact with the `data/` files beside it.

Tabs: Health (about 30 vitals against real, with 30-year sparklines), Game engine, Balance, What wins, Situations (play level), Player paths, Recruiting, Coaches, Draft, Records, Names, Anomalies, Idea board, Seed lab, Explorer (any two numbers against each other) and Pages (program and player pages).

The health thresholds (near, off, far off) are Claude's proposal, not confirmed.
