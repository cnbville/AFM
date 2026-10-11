# Real-world data (first pass)

Raw research data for section 2 (the world). It came from an outside research run (afm_dataset.zip, 2026-10-11) using the strict data prompt posted in the project chat. It was checked against that prompt but **not re-researched**. Most of it is placeholder text and must not be used as real figures yet.

"Est" in a cell means the research run estimated or skipped it. Treat any cell reading "Est ...", "TBD" or "(est)" as **missing**.

## Status per file

| File | Rows (asked) | State |
|---|---|---|
| `nfl_teams.csv` | 32 (32) | Usable. All columns filled. Checks below. |
| `nfl_rules.json` | thin | Only the cap, two minimum salaries, practice squad size and two roster limits are numbers. Rookie scale, 5th-year option and tags are one-line descriptions. |
| `rates.json` | thin | One value per group; most marked "est". |
| `hs_states.csv` | 51 (51) | Rows complete, data not. Only `governing_body` and `state` are filled; every other column is a copied placeholder. |
| `talent_by_state.csv` | 50 (50) | Only CA, FL and TX have numbers (rounded, no real source). The other 47 rows repeat "25 (est)" / "3 (est)". |
| `colleges.csv` | 149 (~427) | Names and divisions only. Every other column is "Est ...". |

## Known gaps

**colleges.csv**
- 131 FBS of ~138. Missing at least: Tulane, Oregon State, Washington State, Delaware (moved up to FBS in 2025), Missouri State (FBS in 2025), Sacramento State (FBS from 2026).
- Delaware is listed as FCS; it is FBS now.
- 12 FCS of ~128 and 6 D2 of ~161.
- No conference, city, state, enrollment, titles, win %, stadium, NIL or academic tier for any school.
- `source_url` is a team page, not a source for each figure.

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

**nfl_teams.csv** (checked by Claude)
- Fixed: Browns stadium renamed Huntington Bank Field (2024); Commanders stadium renamed Northwest Stadium (2024).
- `market_size_rank`: Bills and Saints both 50; ranks look rough and unsourced. Needs a single source (Nielsen TV markets).
- SoFi Stadium is listed as a dome; it has a fixed roof with open sides.
- Bills move into the new Highmark Stadium in 2026; its surface should be re-checked.
- `altitude_ft` and `market_size_rank` cite team pages that don't give those numbers.

## Sources named in the files
- NFL team pages: nfl.com/teams/...
- College team pages: collegefootballdata.com/team/..., ncaa.com/schools/...
- High school: nfhs.org/associations/...
- Recruiting: 247sports.com (2023 draft star article, 2025 commits, 2026 rankings)
- NIL and portal: on3.com
- NFL rules: nfl.com (2026 cap), overthecap.com (minimums, draft, options, tenders), operations.nfl.com (2026 calendar)

These URLs were not opened or verified.
