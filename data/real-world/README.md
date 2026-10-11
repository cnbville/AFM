# Real-world data (first pass)

Raw research data for section 2 (the world). It came from an outside research run (afm_dataset.zip, 2026-10-11) using the strict data prompt posted in the project chat. It was checked against that prompt but **not re-researched**. Most of it is placeholder text and must not be used as real figures yet.

"Est" in a cell means the research run estimated or skipped it. Treat any cell reading "Est ...", "TBD" or "(est)" as **missing**.

## Status per file

| File | Rows (asked) | State |
|---|---|---|
| `nfl_teams.csv` | 32 (32) | Usable. Market ranks, Bills surface and SoFi roof fixed 2026-10-11 with sources. |
| `nfl_rules.json` | thin | Only the cap, two minimum salaries, practice squad size and two roster limits are numbers. Rookie scale, 5th-year option and tags are one-line descriptions. |
| `rates.json` | thin | One value per group; most marked "est". |
| `hs_states.csv` | 51 (51) | Rows complete, data not. Only `governing_body` and `state` are filled; every other column is a copied placeholder. |
| `talent_by_state.csv` | 51 (50 + DC) | **Re-researched 2026-10-11.** Computed from public recruiting, roster and draft datasets. Method below. |
| `colleges.csv` | 426 (426) | **Re-researched 2026-10-11.** Real data, a source URL per figure. Gaps listed below. |

## Known gaps

**colleges.csv** (re-researched 2026-10-11, 2026 season membership)
- Rows: 138 FBS + 128 FCS + 160 D2 = 426. FCS includes 4 schools moving up from D2 (Mercyhurst, New Haven, West Florida, West Georgia; see `notes`). FBS includes North Dakota State and Sacramento State (new in 2026).
- Conferences are 2026: Pac-12 rebuilt (Boise State, Colorado State, Fresno State, Oregon State, San Diego State, Texas State, Utah State, Washington State); Northern Illinois, UTEP and North Dakota State in the Mountain West; Louisiana Tech in the Sun Belt; Sacramento State in the MAC.
- `national_titles`: titles at the school's current level. FBS uses the Wikipedia table of titles from NCAA-designated major selectors (through Indiana's 2025 title; totals are disputed by nature). FCS counts NCAA I-AA/FCS titles 1978-2025. D2 counts NCAA D2 titles 1973-2025. `lower_division_titles` lists titles won at a lower level (e.g. North Dakota State "FCS 10; D2 5").
- `last_5yr_win_pct`: all completed games 2021-2025, postseason included, ties count half, from the CFBD-based `cfb_schedules` dataset (sportsdataverse). Spot checks: Indiana 36-29, Alabama 56-14, Miami (FL) 42-24. For schools that changed level, the record mixes levels. `record_seasons` shows the seasons covered; a few schools have fewer than 5 (they joined the NCAA level recently).
- `academic_tier`: AAU member, else Carnegie 2025 R1 or R2, else "other".
- Each figure has its own source column: `enrollment_source_url`, `stadium_source_url`, `titles_source_url`, `record_source_url`, `academic_source_url`. Enrollment figures are as Wikipedia gives them (different years per school); Ivy League figures are undergraduate plus graduate.

Gaps (exact):
- `nil_estimate_usd` is blank for all 426. No public source gives a dollar NIL figure per school. (The 2025-26 revenue-sharing cap of about $20.5M per school is a cap, not an estimate.)
- `stadium_capacity` blank for 16 D2 schools (no capacity in any source found): Allen, Ashland, Barton, Chowan, Edward Waters, Erskine, Ferrum, Jamestown, Kentucky Wesleyan, Lake Erie, Northwood, Ohio Dominican, Roosevelt, Shorter, Thomas More, Walsh.
- `last_5yr_win_pct` blank for Chicago State (no completed games in the data).
- Ferrum enrollment is undergraduates only (Wikipedia infobox, no year).

**talent_by_state.csv** (computed 2026-10-11; every row is marked "est: computed")
- No public page lists FBS signees or draftees for all 50 states, so these are computed from three open datasets and the method is given here so anyone can redo it.
- `avg_fbs_signees_per_year`: rated signees (ESPN ratings, high school and JUCO) who signed with an FBS school, classes 2022-2026, from the sportsdataverse `cfb_recruits` release. Home state comes from matching each signee by name to CFBD rosters (cfbfastR-data `rosters/csv`), which list home state. 86-93% of signees matched per class; each class is scaled up by its match rate. Total about 2,900 per year.
- `avg_nfl_draftees_per_year`: picks in the 2022-2026 drafts (nflverse `draft_picks`), home state from the same roster match (94% matched; the rest, about 16 picks a year, are unmatched or from outside the US).
- `top_hometowns`: the three home cities with the most FBS signees in the state, 2022-2026. These are cities, not metro areas.
- DC is included as a 51st row.
- Limits: name matching can mix up two players with the same name; ESPN star ratings differ a little from 247Sports composite ratings.
- Sanity check: Texas, Florida, Georgia and California lead both columns, matching NFL.com's 2026 count of NFL players by home state (TX 192, FL 185, CA 159, GA 148): https://www.nfl.com/news/state-of-texas-three-south-florida-high-schools-produce-most-nfl-players-on-kickoff-weekend-rosters

**rates.json**
- Five-star draft rate: 70% here (247Sports article on the 2023 draft). Earlier project figure was about 63%. Both are real numbers from different class windows; which one the engine calibrates to is open.
- NIL ranges: only Power 4 QB. Missing every other level and position group.
- Portal entrants: FBS only (3,310, est). FCS and D2 missing.
- JUCO-to-FBS (450) and prep-school signees (150) are estimates; the cited page is a commit list, not the count.
- Recruits per star: five-star 32, four-star 400 (est). Three-star and below missing.
- Draft rates for 2-star and unrated players missing.

**nfl_rules.json**
- Minimum salaries only for rookies and 1 year of experience. Missing 2 to 7+ years.
- Rookie scale by pick: missing (description only).
- 5th-year option and franchise/transition tags: rules not written out.
- Practice squad: size only; no veteran or international exemption rules.
- Roster limits: 90 and 53 only; no 2026 cutdown dates or game-day actives.
- No years attached to most values.

**nfl_teams.csv** (checked 2026-10-11)
- `market_size_rank` is now the Nielsen 2024-25 TV market (DMA) rank for the team's market, one source for all 32 (Wikipedia list of US TV markets). Teams sharing a market share its rank (Giants/Jets 1, Rams/Chargers 2). Arizona = Phoenix, Carolina = Charlotte, New England = Boston, Tennessee = Nashville, Washington = Washington DC.
- Bills: the new Highmark Stadium (opened 2026) has natural grass (Kentucky bluegrass), so `surface` is now grass.
- SoFi Stadium `roof` is now "canopy": a fixed translucent roof with open sides.
- Earlier fixes kept: Browns' Huntington Bank Field and Commanders' Northwest Stadium.
- Gap: `altitude_ft` still has no per-figure source.

## Sources named in the files
- NFL team pages: nfl.com/teams/...
- College team pages: collegefootballdata.com/team/..., ncaa.com/schools/...
- High school: nfhs.org/associations/...
- Recruiting: 247sports.com (2023 draft star article, 2025 commits, 2026 rankings)
- NIL and portal: on3.com
- NFL rules: nfl.com (2026 cap), overthecap.com (minimums, draft, options, tenders), operations.nfl.com (2026 calendar)

These URLs were not opened or verified.
