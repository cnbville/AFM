# The seed 321 world played 10 seasons on (realism audit item 25)

Read-only. Nothing was fixed; each finding has its likely cause and waits for Cain's yes or no.
Asked 2026-11-24 ("agreed"), run 2026-11-24/25, read 2026-11-25/26.

## What was run

- The published seed 321 world (1 January 2026), played to 1 January 2036 with the world log on and every level's games on the match engine: `afm_future` (the game's own play-on, `World::play_on_to`), in two jobs (3 h 11 min and 2 h 33 min), then the book job. Workflows `future-run.yml` (play and book) and `future-read.yml` (reads a book into the repo).
- The book is on the worlds page as the prerelease `future-seed321-20261125-7071c19` (book only, no world file).
- **Which main:** `7071c19` (PR #628), the audit's own engine (`2930ba6`) plus the play-on tool. It does **not** hold the audit lanes that came after: program fortunes P1 (#634), FBS rushing (#641), the NIL split and market refit (#649, #654, #658), the blitz lane (#659). So this read is how the audit-time world drifts, not how today's main drifts.
- The world file was built before the NIL scheme's later items, so the first future year (2026) has one step in the NIL and revenue-share numbers; the years after it are the trend.
- The reader's "against real" checks were built for 2026 and do not age, so every drift below is read from the book's own year series (`read-*.txt` beside this file).

## What holds (no drift)

| Read | 2021-25 or 2025 | 2031-35 or 2035 | Real |
|---|---|---|---|
| NFL starters' mean overall (8+ starts) | 80.8-81.2 | 80.9-81.2 | n/a (kept) |
| Round 1 draftees' overall in their first season | 78.6-81.9 | 79.1-81.4 | n/a |
| FBS program strength, mean (spread) | 61.8 (54.4 to 73.0) | 62.4 (53.9 to 72.3) | n/a |
| FCS program strength, mean | 56.4 | 56.4 | n/a |
| FBS points a team game | 28.7 | 28.8 | 27.0 |
| FBS yards a carry | 5.77 | 5.84 | 5.12 (item 1's fix is not in this run) |
| FBS completion rate | 60.4% | 60.4% | 60.6% |
| High school points a team game | 20.5 | 20.6 | 26.0 (a known gap) |
| Prestige, median of all schools | 0.272 (2025) | 0.269 (2034) | n/a |
| Prestige, top tenth / bottom tenth | 0.416 / 0.169 | 0.403 / 0.173 | n/a |
| Conference strength | flat, +0.1 a year | | n/a |
| Colleges with a draft pick | 119 | 98 | 97 (comes to real) |

The audit's "no rating drift" holds for ten more seasons. Every age band of NFL players gains and loses at the same yearly rates in 2032-35 as in 2022-25 (age 25 +0.53 against +0.45, age 30 -0.68 against -0.69). The all-player means in the log rows fall (FBS 63.4 to 55.1, NFL 73.0 to 70.3), but that is the log's coverage (older seasons keep only players who lasted), not the players: FBS history rows go from 3,800 to 14,400 a year.

## What drifts

1. **The NFL cap outruns the money** (`future_cap`, 8% a year and a 14% media-deal jump in 2030, is about 10.9% a year; revenue grows 6.0%).
   - League revenue 26.0 billion (2026) to 41.3 billion (2034); per club 812 to 1,291 million (+59%).
   - Club costs 611 to 1,105 million (+81%). Players' share of club revenue 43.7% to 55.5%; costs 75% to 86% of revenue.
   - Real: the cap is about 48% of league revenue and follows it. The reader's own check already reads 52.5 against 44.
   - Revenue's own growth is near real (real 2014-24 about 7.2% a year, the world 6.0%).
2. **NFL offense creeps up.** Points a team game 24.1 (2025) to 26.7 (2035), a rise of about 0.25 a year (real 22.5). Passing yards an attempt 7.34 to 7.64 (real 7.05). The FBS passing leader 4,680 to about 5,300 yards (real 4,191). Starters are flat. Measured 2026-11-26 (`nfl-scoring-creep.md`): it is the running game (yards a carry 4.52 to 4.91), not passing, calls or coaches; ratings explain a quarter; the rest is not located.
3. **FBS portal entrants fall, signees swell.** Entrants 3,330 (2026) to 2,450 (2034), -26%; FBS signees 3,290 to 3,860 (real about 2,900). **Traced 2026-11-26 (thread "Fix falling FBS portal entries"): the pool shrinks, the rate does not, and real fell too; see "Finding 3 traced" below.**
4. **FBS champions concentrate over the decade.** Different FBS champions in 15 years 12 (2022) to 6 (2034) (real 7 to 9). 2026-2035: Miami (FL) 4 national titles and 7 ACC titles, Hawaii 6 Mountain West titles, Maryland 5 Big Ten titles. The most titles by one school stays at 4 or 5 (real 4 to 6). Unbeaten FBS teams 0.4 a year in 2021-35 (real 0.6; the 2035 row reads 1.0 on a partial year).
5. **NIL (pre-item-14 code).**
   - FBS collectives' budgets, mean per school: 7.9 million (2025), 16.5 (2026, the first year of the new scheme), then 16.0, 16.7, 17.3, 18.5, 19.6, 20.4, 21.2, 22.0, 22.8 (2035): +3.7% a year after the step. School revenue grew 3.9% a year, the revenue-share cap 4% a year, the third-party pot 5%.
   - FBS booster money falls 5.6% (7.84 to 7.40 million a collective) while revenue grows. In 2035, 121 of 141 FBS collectives are "Steady", 19 "Growing", 1 "Falling": the trend field barely moves.
   - Power 4 starters' NIL bands in 2035 against real: under $10k 20.8% (real 66.5%); $10k-50k 2.8% (16.1%); $50k-100k 3.4% (7.4%); $100k-500k 48.3% (9.1%); $500k-1M 15.0% (0.6%); $1M and over 9.7% (0.3%). Same shape as the audit's 2026 read: 24% under $10k, 44% of SEC and Big Ten at $100k or more. The skew did not grow, and did not shrink.
   - Median by position in 2035: QB $1.68M, EDGE $0.82M, DT $0.79M, OT $0.75M, WR $0.62M, K $0.09M, LS $0.08M. The top 50 earners are mostly quarterbacks, led by $12.1M.
6. **The House cap.** Largest revenue share paid to a player in the latest season 28.1 million (real cap 20.5 in 2025-26). The cap itself is coded at $21.58M in 2026-27, +4% a year (`ncaa.rs`, `revenue_cap`), so about $29M by 2034. That is what the real settlement says for its first three years; the real cap is then reset from revenue (22% of the Power 4's average), which this world does not model. Here school revenue grows 3.9% a year, so the two stay together and nothing breaks.
7. **College money.** Revenue per school 2025 to 2034 (million): SEC and Big Ten 178 to 251 (+3.9% a year), ACC and Big 12 111 to 160, Group of 5 57 to 80, FCS 21 to 29, Division II 9.5 to 12.9, Division III 2.3 to 3.2. Every tier spends what it earns (cost within 1% of revenue each year), so no program ever runs a surplus or a deficit. Real Power 4 revenue grows faster (media deals), about 6% to 8%.

## Finding 3 traced: FBS portal entrants (2026-11-26)

Read from the same book (`future-seed321-20261125-7071c19`) and the Jan 2026 world's own book, with `future-read.yml` (its new `start_tag` input reads both; raw text in `future-run-seed321-raw/read-5-portal.txt`). Nothing was built: it is not a bug.

**What fell is the pool, not the rate.**

| FBS college players | 1 Jan 2026 (start) | 1 Jan 2036 (end) |
|---|---|---|
| Players a program (min to max) | 130.0 (96 to 151) | 104.0 (87 to 115) |
| Players in all | 17,676 | 14,346 |
| Backups (walk-ons among them) | 7,958 (37.8%) | 5,137 (1.3%) |
| Redshirts / rotation / starters | 4,126 / 1,823 / 3,769 | 3,676 / 1,562 / 3,971 |
| Walk-ons, all roles | about 4,100 (23%) | about 480 (3%) |

- At the start the rosters sit at 130 because the House settlement's designated walk-ons (2024-25 and the 2025 class) stand outside the 105 limit (`recruiting::takes_a_place`; the squad check already counts 26 of them in February 2026). They age out by about 2029, and nothing replaces them: under the House limit a walk-on takes a place like anyone.
- Each role's yearly entry rate is the same before and after. Entrants a role's players that Jan: backups 25.3% (2010 of 7,958, label 2025) then about 23.7% (1,217 of 5,137, label 2034); redshirts 21.5% then 22.8%; rotation 8.5% then 10.8%; starters 4.5% then 5.6%. All roles together 18.2% then 17.1%.
- So entrants follow the stock: 3,327 (label 2026), 2,887, 2,710, 2,573, 2,506, 2,389, 2,609, 2,491, 2,448. It falls for the five years the designated walk-ons take to leave and is flat from 2030 (2,389 to 2,609, no trend). Backup entrants carry it: 2,202 (2026) to about 1,260; redshirts rise from 684 to about 860 as the freshmen share grows; starters and rotation are flat. Cuts (184 to 114-300), "not in the plans" (538 to about 330) and "playing time" (1,671 to about 1,150) fall with the pool; coaching-change entries hold (about 650). By school: Power 4 2,060 to 1,457 (-29%), Group of 5 1,267 to about 950 (-25%).

**Real did the same step.** NCAA president Charlie Baker said FBS portal entries were down 23% in the Jan 2026 window, about 3,000 against the record 4,201 a year before (a second NCAA read says 18%): the first cycle under the House roster limit and paid rosters. So "real keeps rising" in finding 3 was out of date: the rise was to 2025, and real fell after. The world's windows read 3,213 (Jan 2025, real 4,201), 3,223 (Jan 2026, real about 3,000), then 2,887 and down to 2,450. The first is 24% low (audit item 20), the second at real; whether real keeps falling as its own designated walk-ons run out is not known yet (no real count after Jan 2026).

**Signees.** 4,485 (2024), 3,935 (2025), 3,288 (2026), then 3,664 to 3,950 and 3,863 (2034). The 2026 dip is the trim year (a class sized to holes on rosters of 130 over a limit of 105); the rise since is a return to about 3,800, which is not above 2025. It is above real: 28 a program against 2,913 in the pre-House real data, with walk-ons at 3% of rosters (the earlier squad measure put real at 7.6 a roster) and fewer transfers landing in FBS (957 for 2025, then 600 to 860). The class plan fills every open place under the House limit with scholarship signees before walk-ons (`class_plan`, `plan.slots`, `plan.walk_on_room`). Not changed: there are no real House-era class counts yet to size it against.

**Sources for the real figures:** [Front Office Sports on Baker's 23%](https://frontofficesports.com/article/college-football-transfers-down-23-after-schools-started-paying-players/), [ESPN's 2026 portal trends](https://africa.espn.com/college-football/story/_/id/47624150/2026-college-football-transfer-portal-trends-prices-qbs), [On3's 2025 count via Fifth Quarter](https://fifthquarter.net/article/the-price-of-stability-why-the-portal-finally-cooled-off).

## Gaps added to `GAPS.md`

Engine: NFL offense creeps up. Logic: the NFL cap outruns the money; FBS portal entrants fall and signees swell; FBS champions concentrate; collectives' booster money shrinks. Tools: panel text prints headings and no rows. The old entry "Nothing is measured past 2026" is replaced by one that points here.

## What it cannot say

- NIL here is the code before the NIL lanes (#649, #654, #658), so it says nothing about whether those fixed the spread. Run it again on a world rebuilt on a later main.
- The log's per-player rows are thinned for older seasons, so a drift in players' ratings is read from starters' seasons and program strength, which are complete.
- FBS strength and NFL starters sit inside the game's own scale; a drift of one point a decade would be inside the year-to-year noise (about 0.2 for NFL starters).
