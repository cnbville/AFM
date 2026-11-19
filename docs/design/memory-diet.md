# Memory diet: the season under about 4.25 GB

Research thread "Find what makes the game memory heavy", 2026-11-17. Cain: "fresh thread to do bloated memory research ... It's starting to consistently lag behind on clicks". He agreed the same evening to items 1-11 and 13 (no to 12): "okay build a proposal but wait until all the engine changes are done. The coordinator or i will remind/wake you". **Lane M1 (items 1, 2, 4, 7, 8) built 2026-11-17** beside the engine lanes, by Cain's "go ahead" the same evening: on the seed 181 world the heap at the first game went from 6.44 to 5.19 GB and resident memory from 6.43 to 5.61 GB, with every store's print the same as main (`docs/game.md`, "Memory diet, lane M1"). **Lane M2 (items 5 and 11, built 2026-11-19, Cain: M2 right after M1)**: season rows 104 bytes instead of 144 and the leavers' ledgers dropped at the year-end cleanup, with every store's print the same as main on a small world (`docs/game.md`, "Memory diet, lane M2"); items 3, 6, 9 and 10 built the same day after Cain's "agree on all" (the people and coaches prints may move while results stay the same): alumni players packed (the pro careers stay whole for the Data pages), leavers' recruiting lists dropped weekly, retired coaches without their working part, and the open tallies' idle rows packed; the same fingerprint and the other 15 prints as main through to 20 February 2027 on a small world. M3 waits until the engine lanes (scheme fit SF1-SF11, coach fire, engine gaps EG1-EG7) are done, then re-measure, since they change the stores.

The measurements, raw output and tools are in the project folder at `afm/memory-research/` (`report.md`; `mem_weigh.rs`, a copy of `memory_season` that drops every store of the world, with modes `open`, `season`, `counts`, `lean`, `hs`, `sizes`; `mem_sizes.rs`, an engine test that prints every person, player, calendar and coach field's size).

## What it does

- In season the game holds about 4.25 GB instead of 6.5 GB (seed 181 world, Iowa, Continue from 1 January to the first game). On an 8 GB Mac that leaves room for macOS and the app's page, so clicks stop waiting on memory squeezed out to the disk.
- Nothing changes in play for items 1-11: the same people, the same numbers, the same result from every seed. Item 13 changes which world a seed builds (like a different seed would), not how realistic it is.
- Nothing shows anything about potential: the stores are split by what you can already see (recruited or not, in college or not, left football, retired).

## Where the memory is (seed 181, main 7bc88f6, 4-core Linux)

| Moment | Resident | Peak so far |
|---|---|---|
| Open | 5.85 GB | 5.86 GB |
| First game (4 September) | 6.54 GB | 6.86 GB |

On 22 October the 30-year world sat at 3.81 GB in season (lane 1, agreed budget 4 GB). Since then a person grew from 1,752 to 2,112 bytes (NIL, cut, home visits and recruit cases in the calendar about 130 B; flaws and tools plus progression 80 B; tests, coach looks and reps 60 B; bigger tendency, body and training parts about 90 B), and new stores came: tallies (0.47 GB in January), the results book (0.29), marks, times and others.

At the first game: the people list 2.59 GB (1.16 million people, 986,000 of them high schoolers), the calendar's lists 0.73, season rows 0.51, the stat book 0.39, coaches 0.31, the results book 0.29, alumni 0.26, tallies 0.15, notes, high school lines and tests 0.28, the rest about 0.9.

## The items (agreed 2026-11-17)

Savings are at the first game, worked out from the measured sizes and counts (estimates until built).

| # | Change | Saves | Results |
|---|---|---|---|
| 1 | Hand back the portal-lean list's room each season: `PersonCal::season_reset` empties it with `clear()`, which keeps the room (20.6 million entries of room for 1.76 million) | 0.60 GB | same |
| 2 | Store the college-and-later fields (about 430 B: the portal, draft read, NIL, cut, lean, college seasons, snaps, reps, path, college staff, pro, roads, looks...) only once a man reaches college | 0.42 GB | same |
| 3 | Alumni's full records (player, pro career, notes) on disk beside the world, a small index in memory; pages read the record from disk | 0.23 GB | same once the world's readers of `alumni[..].player` (following, stories/traditions, stories/careers, ground/areas, ground/standing, scouting/file) are shown to need only the index, or keep what they need |
| 4 | Store the recruiting calendar only once a boy is recruited (667,000 of 986,000 high schoolers have a blank one in January) | 0.16 GB | same |
| 5 | Compact season rows: the depth label as a code, the pro team and small whole numbers in fewer bytes | 0.14 GB | same, if every depth label comes from a fixed list (check first) |
| 6 | Keep men who leave football compact until the year-end cleanup (`World::compact`) drops them | 0.13 GB | same once their readers (`former::intake` and the others reading `Stage::Done`) get what they read |
| 7 | Grow the stat book's rows one at a time, as season rows already do (room 3.49 million for 1.81 million) | 0.11 GB | same |
| 8 | A person's indexes and optional numbers in fewer bytes with the same values (`usize` indexes as `u32`/`u16`, `Option<f64>` as `OptF64`) | 0.10 GB | same |
| 9 | Retired coaches compact (112,000 of 215,000; they never come back): keep the record, history, mentor, renown and what the Hall and the trees read | 0.10 GB | same once their readers are checked |
| 10 | Pack the finished weeks of the season being played in the tallies | 0.1 GB, 0.3 GB December to February | same; this season's pages get a little slower |
| 11 | Drop the results ledgers of men who left football (238,000 of 1.07 million) | 0.07 GB | must prove same: the map's order changes; if the fingerprint moves at all, drop the item |
| 12 | Ratings, ceilings and habits as 2-byte numbers | 0.39 GB | **No** (Cain, 2026-11-17): growth that comes in pieces under 0.01 (a game's results, a week's training) would round away |
| 13 | The other 8-byte numbers (background, bloom, longevity, wear, injury proneness, NIL money and the like) in 4 bytes | 0.13 GB | changes which world a seed builds; seven digits keep every step the game takes |

Items 1-11 and 13 together: about 2.3 GB, from 6.54 to about 4.25 GB in season.

Also from the research, proposed, not confirmed: keep only the newest day's portal read in the lean list (it lowers the high during portal windows); build the coming people (support departments, about 90,000 high school coaches) compact or drawn when needed; re-measure with `mem_weigh season` after every lane that adds people or yearly stores and give the number in its final reply.

## Lanes (after the engine lanes)

1. **M1, same results, the biggest part (built 2026-11-17):** items 1, 2, 4, 7, 8. Item 8 left the high school, home state and college indexes at 8 bytes (about 2,100 places read them, about 0.03 GB); item 2 kept in the person the fields high schoolers use (role, last performance, reps, sit-out share). Proof: `memory_season`'s fingerprint the same before and after on the newest world; `mem_weigh season` before and after.
2. **M2, same results with reader checks:** items 3, 5, 6, 9, 10, 11. The same proof; each reader of a dropped part listed in the PR with what it reads.
3. **M3, item 13:** on a new seed, the world's book checks against real data and the trial numbers before and after, within the run-to-run spread.

Each lane updates `docs/reference/how-it-works/` (saves, tallies, people) and `docs/game.md`'s memory section with the new numbers.
