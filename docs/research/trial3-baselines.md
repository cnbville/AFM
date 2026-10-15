# Research area 3: real baselines for world trial 3, and data gaps

Researched 2026-10-14 by Claude for Raichel. Research only. Labels: **[published]** (someone else's study or official source, linked), **[computed]** (Claude's arithmetic on raw real data, dataset named; script `data/real-world/scripts/trial3_baselines.py`, output `data/real-world/trial3_baselines.json`), **[estimated]** (Claude's judgment). "Means" lines and proposals are Claude's reading, not data.

It feeds: the world trial 3 findings (`docs/trials/world-trial3.md`, items 1-9) and the gaps in `data/real-world/README.md`.

Data: CFBD scores 2001-2025 (https://github.com/sportsdataverse/sportsdataverse-data/releases/tag/cfb_schedules), ESPN recruit ratings of signees, classes 2010-2021 (https://github.com/sportsdataverse/sportsdataverse-data/releases/tag/cfb_recruits), nflverse draft picks, combine and players (https://github.com/nflverse/nflverse-data/releases), CFBD 2024 rosters (https://github.com/sportsdataverse/cfbfastR-data/tree/main/rosters/csv).

## Finding 1: dynasties (trial: Tennessee won 9 titles in 15 years)

**[computed]** Title game winners 2001-2025 (BCS title game to 2013, CFP after), from CFBD scores:
- The most titles by one school in any 15 years: **6** (Alabama, 2006-2020 through 2009-2023). Usually 4-5.
- **7 to 9 different champions** in every 15-year window. 2011-2025: 8 (Alabama 5; Ohio State, Clemson and Georgia 2; Florida State, LSU, Michigan, Indiana 1).
- Best 15-year record, 2011-2025: Alabama 183-26 (.876), Ohio State 171-28 (.859), Clemson .816, Georgia .815, Oregon .766.
- Unbeaten seasons (10+ wins, no loss), all of FBS, 2011-2025: **9 in total, and no school had more than one.**
- A team's win percentage carries over from one season to the next with a correlation of 0.54 (all FBS, 2001-2025). Good teams stay good, but not reliably.

Trial 3: Tennessee went 198-13 (.938) over 21 years with 6 unbeaten seasons.

**Means:** the world should give a top program at most about 6 titles in 15 years, about 8 different champions, and roughly one unbeaten season per program per 15 years at most. These make a clear check for world trial 4.

## Finding 2: droughts (trial: 3 Power 4 programs never had a winning season in 15 years)

**[computed]** The 68 current SEC, Big Ten, Big 12 and ACC members plus Notre Dame, 2001-2025 (a winning season = more wins than losses; some seasons were spent in other conferences):
- **No program went 15 years without a winning season**, in any 15-year window since 2001.
- Fewest winning seasons in 2011-2025: Kansas 1, Colorado 3, then Indiana, Illinois, Vanderbilt, Virginia and Purdue with 4.
- Longest run without a winning season: Kansas 14, Duke 12, Indiana 11, Colorado, Illinois and Vanderbilt 10.

**Means:** a 15-year run with no winning season should be close to impossible, and 10-14 years should be rare (a handful of programs). The trial's Boston College, Iowa State and Wake Forest all had real winning seasons in that period.

## Finding 3: coaches of color (trial: 6 Black FBS head coaches in 2025)

**[published]** 14 Black head coaches at 133 FBS schools in 2023, and 16 at 134 heading into 2024 (CBS Sports, 2024-05-16: https://www.cbssports.com/college-football/news/whats-next-for-black-representation-in-college-football-coaching-key-figures-weigh-in-amid-slight-increase/). In 2025, 8 of the 68 Power 4 head coaches were Black (EduLedger, undated 2025 piece: https://www.theeduledger.com/recap-newsletter-only/article/15755239/high-expectations-black-head-coaches-and-the-2025-college-football-season).
- Gap: no published FBS-wide count for the 2025 season was found. The trial's "real 16" is the 2024 figure.

## Finding 4: the first overall pick (trial: a QB 45% of the time)

**[computed]** Already in the dataset: QBs went first overall in **73.1%** of drafts 2000-2025, EDGE 19.2%, OT 7.7% (`data/real-world/draft_positions_2000_2025.csv`). Nothing new was needed.

## Finding 5: three-stars and two-stars (trial: drafted 4.6% and 1.1%)

**[computed]** Already in the dataset (`rates.json`, ESPN ratings, classes 2013-2021): 5-star 62.2%, 4-star 24.4%, **3-star 6.0%, 2-star 2.1%** drafted.

**[computed] New: where draft picks come from.** NFL draft picks 2013-2025, matched by name to ESPN-rated signees (classes 2010-2021):

| Stars as a recruit | Share of all picks | Share of first-round picks |
|---|---|---|
| 5 | 10% | 26% |
| 4 | 23% | 26% |
| 3 | 36% | 28% |
| 2 | 6% | 3% |
| Not matched (unrated, walk-on, JUCO, prep, or a name mismatch) | 25% | 18% |

So **two of every three draft picks were not 4- or 5-star recruits**, and even in round 1, 3-stars are as common as 5-stars. Name matching misses some players, so the "not matched" row includes some rated players.

**Means:** the trial's guess holds. Real stars miss late bloomers and overlooked players often. Hidden growth after high school (and scouting misses) should be big enough that about a third of draft picks come from 3-star recruits and about a quarter from outside the rated pool.

## Finding 7 and 8: kickers, punters and QBs (trial: drafted kickers run 4.54, QBs 4.57)

**[computed]** NFL combine 2000-2025 (nflverse):

| Position | Weight | 40 time (mean) | 40, fastest 10% | Share who ran the 40 |
|---|---|---|---|---|
| K | 197 | 4.91 | 4.74 | 50% |
| P | 214 | 4.91 | 4.70 | 69% |
| QB | 222 | 4.81 | 4.59 | 87% |
| TE | 253 | 4.76 | 4.58 | 88% |
| DE | 269 | 4.83 | 4.65 | 97% |
| RB | 213 | 4.55 | 4.40 | 92% |
| WR | 201 | 4.51 | 4.37 | 93% |

Half of kickers don't run at all, and those who do are likely the faster ones, so the real kicker average is probably slower than 4.91 **[estimated]**. Even the fastest 10% of kickers (4.74) are slower than the trial's average kicker (4.54). The trial's QB average (4.57) is near the real fastest 10% of QBs.

**Means:** kickers, punters and QBs need their own athletic spread, well below skill players. A kicker should almost never rate as a pass rusher.

## Finding 9: repeated names (trial: 46.5% of 779,551 players share a full name)

**[computed]** Real rates depend heavily on the size of the pool:
- All 22,837 players on 2024 college rosters (FBS and FCS, CFBD): **4.4%** share their full name with another player. In random samples of 10,000 it is 1.9%, of 5,000 1.1%, of 1,000 0.4%. Most common: Jayden Williams (7), Isaiah Johnson (6), Jaden Robinson, Cam Smith, Jalen Smith (5).
- All 24,854 NFL players in nflverse (all eras): 7.8%. One season of NFL rosters (2024, 3,216 players): 1.3%.
- **[estimated]** The share grows about as fast as the pool, so a world of about 780,000 players (high school included) should show a far higher share than 4.4%. A rough extrapolation gives somewhere around half. So the trial's 46.5% may not be wrong at that scale.

**Means:** compare like with like. The fair test is the trial's FBS and FCS rosters alone (about 23,000 players) against the real 4.4%, and the trial's NFL rosters in one season against the real 1.3%. This needs a recount, not necessarily bigger name pools.

## Dataset gaps (from `data/real-world/README.md`)

- **NIL or roster money per school:** still no official per-school figure. **[published]** Schools paid **$1.77 billion** in revenue sharing in 2025-26, to 34,915 athletes at 307 schools; most of it went to football and basketball (NCAA and CSC, via Bleacher Report, 2026-10-01: https://bleacherreport.com/articles/25505747-college-athletes-earned-177b-schools-2025-26-additional-500m-nil-contracts). Outside NIL deals approved since June 2025: 46,478 deals, $582M+. Estimates only: Ohio State's football roster budget $49-54M (The Athletic, via the same article) and Texas at about $48M (StickToTheModel model, updated 2026-10-13: https://sticktothemodel.com/college-roster-spend). A Senate bill would raise the school pay cap to about $48M; it still needs the House.
- **JUCO-to-FBS signees per year:** still no source.
- **Portal:** **[published]** FBS portal entries fell 23% in January 2026 compared with the record 4,201 in the 2025 cycle (NCAA's Charlie Baker and On3, via Front Office Sports, 2026-01-14: https://frontofficesports.com/article/college-football-transfers-down-23-after-schools-started-paying-players/).
- **High school NIL by state:** already filled on 2026-10-13 (`hs_states.csv`).

## Proposals for Raichel (Claude's ideas, proposed, not confirmed)

1. **World trial 4 checks for dynasties and droughts:** at most about 6 titles for one program in 15 years, about 8 different champions, no more than about one unbeaten season per program, and no Power 4 program without a winning season over 15 years. These follow the physics fixes, which aim at the cause.
2. **Late bloomers:** more hidden growth after high school (or more scouting misses at the 3-star level), aiming for about a third of draft picks from 3-stars and a quarter from outside the rated pool, while keeping the five-star rate at 63%.
3. **Specialists and QBs get their own athletic spread**, using the combine table above.
4. **Measure name repeats at the same scale** as the real figures (college rosters alone against 4.4%) before changing the name pools.
