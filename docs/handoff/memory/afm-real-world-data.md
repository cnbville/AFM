---
name: afm-real-world-data
description: AFM data/real-world dataset re-researched and merged 2026-10-11 (PRs #7-#11); what's in it, open data sources that work, and remaining gaps
metadata:
  type: project
  modified: 2026-10-11T23:17:56.793Z
---

On 2026-10-11 a thread replaced the placeholder dataset in data/real-world/ (merged to main, PRs #7-#11). README there lists every source and gap.
- colleges.csv 426 rows (138 FBS, 128 FCS, 160 D2, 2026 conferences), 2021-25 win %, titles, stadium, AAU/R1/R2.
- talent_by_state.csv, rates.json draft rates and recruits per star are COMPUTED from open datasets (method in README).
- nfl_rules.json 2026 + rookie_scale_2026.csv (all 257 picks). hs_states.csv uses MaxPreps 2026 counts/top 5 + SI playoff guide.
- Five-star draft target stays 63% (On3); 247Sports 70% kept as unverified alternate; own ESPN-based count gives 62.2%.
- 2026-10-13 (PR #12): HS transfer rules filled for all 51; 9 more D2 stadiums.
- Gaps: NIL per school; JUCO/prep signee counts (no public source); 7 D2 stadium capacities; G5/FCS/D2 NIL.

**Why:** design sections need real figures; Raichel asked for completeness with a URL per figure.
**How to apply:** curl can't reach most sites here, but raw.githubusercontent.com and GitHub release downloads work: sportsdataverse-data (cfb_schedules, cfb_recruits), cfbfastR-data rosters/csv, nflverse-data (draft_picks, contracts). WebFetch works for Wikipedia and MaxPreps. Related: [[afm-trial3-calibration]]
