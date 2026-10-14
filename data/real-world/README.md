# Real-world data

Raw research data for section 2 (the world). The first pass came from an outside research run (afm_dataset.zip, 2026-10-11) and was mostly placeholders. On 2026-10-11 Claude re-researched every file. Each figure now has a source URL in its row or in this README. "Computed" values are derived from open datasets with the method written below. Every remaining gap is listed by name.

## Status per file

| File | Rows (asked) | State |
|---|---|---|
| `nfl_teams.csv` | 32 (32) | Usable. Market ranks, Bills surface and SoFi roof fixed 2026-10-11 with sources. |
| `nfl_rules.json` | full | **Re-researched 2026-10-11.** 2026 cap, minimums, tags, option rules, practice squad, roster limits, calendar. |
| `rookie_scale_2026.csv` | 257 (257) | **New.** Every 2026 pick's rookie contract. |
| `rates.json` | most groups | **Re-researched 2026-10-11.** Draft rates by star, recruits per star, NIL, portal. JUCO and prep counts are gaps. |
| `hs_states.csv` | 51 (51) | **Re-researched 2026-10-11.** Programs, classes, playoffs and top teams for every state and DC. Transfer rules for all 51 added 2026-10-13. High school NIL rules for all 51 added 2026-10-13. |
| `talent_by_state.csv` | 51 (50 + DC) | **Re-researched 2026-10-11.** Computed from public recruiting, roster and draft datasets. Method below. |
| `colleges.csv` | 426 (426) | **Re-researched 2026-10-11.** Real data, a source URL per figure. Gaps listed below. |

## Notes and gaps per file

**hs_states.csv** (re-researched 2026-10-11)
- `approx_programs`: number of teams in MaxPreps' 2026 state rankings (includes 8-man, 9-man and 6-man teams). Florida's count is from page 2 of its rankings because page 1 would not load.
- `classifications`, `number_of_classes`, `playoff_format`: the 2026 playoff guide for all 50 states (SI). It gives classes, start dates, final dates and sites, but rarely how teams qualify.
- `top_5_powerhouses`: the top 5 of MaxPreps' 2026 state rankings on the date in `top_5_as_of`. This is current strength, not all-time history. Florida uses the final 2025 ranking (IMG Academy first) because the 2026 page would not load.

Gaps (exact):
- `transfer_rule_summary`: all 51 filled (the other 48 added 2026-10-13, one source per state in `transfer_source_url`). Where possible it's the association's own handbook, bylaw or minutes; otherwise news or a school district page. Several states changed rules in 2024-2026 (one-time free transfers in DE, NJ, NM, OK, SC, SD, TN, WA, WV and NV law); each summary names the year where known.
- Weaker rows (secondary source or exact length not confirmed): AL, IL, HI, ME, MT, NE, OR, PA, VT, and KS (2026 bill status unknown). TN's sit-out length isn't stated.
- DC: `classifications`, `number_of_classes` and `playoff_format` are blank (the SI guide covers the 50 states only).
- Playoff qualification rules (how many teams make it) are missing for most states.
- `hs_nil_allowed`, `hs_nil_note`, `hs_nil_source_url` (added 2026-10-13): whether high school athletes may earn NIL money. Base source: the Business of College Sports tracker (updated 2025-08-27). Changes since then come from On3 (2026-05-04: "46 states plus D.C." allow it once Indiana starts in 2026-27; Alabama, Hawaii, Mississippi and Wyoming don't), WCPO (Ohio, 2025) and KWCH (Kansas, November 2025). 47 rows say yes.
- Weaker NIL rows: AR, CO, MO and TX are "yes" only because On3's count of 46 states requires it; their exact current scope isn't confirmed (TX was limited to athletes 17+ signing with colleges; AR and MO to athletes committed in-state).

**colleges.csv** (re-researched 2026-10-11, 2026 season membership)
- Rows: 138 FBS + 128 FCS + 160 D2 = 426. FCS includes 4 schools moving up from D2 (Mercyhurst, New Haven, West Florida, West Georgia; see `notes`). FBS includes North Dakota State and Sacramento State (new in 2026).
- Conferences are 2026: Pac-12 rebuilt (Boise State, Colorado State, Fresno State, Oregon State, San Diego State, Texas State, Utah State, Washington State); Northern Illinois, UTEP and North Dakota State in the Mountain West; Louisiana Tech in the Sun Belt; Sacramento State in the MAC.
- `national_titles`: titles at the school's current level. FBS uses the Wikipedia table of titles from NCAA-designated major selectors (through Indiana's 2025 title; totals are disputed by nature). FCS counts NCAA I-AA/FCS titles 1978-2025. D2 counts NCAA D2 titles 1973-2025. `lower_division_titles` lists titles won at a lower level (e.g. North Dakota State "FCS 10; D2 5").
- `last_5yr_win_pct`: all completed games 2021-2025, postseason included, ties count half, from the CFBD-based `cfb_schedules` dataset (sportsdataverse). Spot checks: Indiana 36-29, Alabama 56-14, Miami (FL) 42-24. For schools that changed level, the record mixes levels. `record_seasons` shows the seasons covered; a few schools have fewer than 5 (they joined the NCAA level recently).
- `academic_tier`: AAU member, else Carnegie 2025 R1 or R2, else "other".
- Each figure has its own source column: `enrollment_source_url`, `stadium_source_url`, `titles_source_url`, `record_source_url`, `academic_source_url`. Enrollment figures are as Wikipedia gives them (different years per school); Ivy League figures are undergraduate plus graduate.

Gaps (exact):
- `nil_estimate_usd` is blank for all 426. No public source gives a dollar NIL figure per school. (The 2025-26 revenue-sharing cap of about $20.5M per school is a cap, not an estimate.)
- `stadium_capacity`: 9 of the 16 missing D2 figures were added 2026-10-13 from team sites, Wikipedia and Stadium Journey (Ashland, Barton, Chowan, Edward Waters, Erskine, Ferrum, Kentucky Wesleyan, Ohio Dominican, Walsh). Still blank for 7, because no page gives a capacity: Allen (plays at Bolden Stadium / Benedict's stadium), Jamestown, Lake Erie, Northwood, Roosevelt, Shorter, Thomas More.
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

**rates.json** (re-researched 2026-10-11)
- Five-star draft rate: the target stays at 63% (On3, drafts 2017-2021). The 247Sports 70% figure is kept as the alternate, but its page would not open, so it is unverified. Other sources agree with 63%: 247Sports' 2016/17 class 62.3%, and Claude's own count from ESPN ratings (classes 2013-2021) 62.2%.
- Computed draft rates (ESPN ratings, classes 2013-2021, drafted within 2-7 years): 5-star 62.2% any round / 21.0% first round; 4-star 24.4% / 3.4%; 3-star 6.0% / 0.5%; 2-star 2.1% / 0.1%. Matching is by player name, so a few same-name mix-ups are possible.
- Recruits per star per class: ESPN counts for 2013-2026.
- NIL: position ranges for starters at major programs (CBS via SI, 2024); the Opendorse Power 4 earnings spread and totals (2025); Power 4 budget share by position per conference.
- Portal: official NCAA counts for 2022 (FBS 2,918, FCS 2,085); Hudl's 2025 counts by level and destination.

Gaps (exact):
- JUCO-to-FBS and prep-school signees per year: still no source (checked again 2026-10-13). The open recruit dataset has no recruit-type column, and the CFBD API needs a key. Only clue: of the top 50 JUCO players, 43 went to power programs in 2015 and 33 around 2025 (https://www.outkick.com/sports/juco-no-mo-how-transfer-portal-killed-junior-college-ranks).
- NIL dollar ranges for Group of 5, FCS and D2: none found.
- D2 football portal entrants: no official football count (NCAA gives D2 all-sport totals only).
- The college revenue-sharing cap (about $20.5M in 2025-26) was not verified on a page.

**nfl_rules.json** (re-researched 2026-10-11)
- Fixed: the first pass had 2022-era minimum salaries ($795,000 rookie). 2026 minimums are $885,000 (0 credited seasons) up to $1,300,000 (7+).
- Added: 2026 franchise and transition tags for every position, fifth-year option tiers, free agency rules and dates, practice squad size and pay, cutdown rules, 2026 calendar.
- `rookie_scale_2026.csv`: all 257 picks (years, total value, average per year, guaranteed) from the OverTheCap contract data in the nflverse `contracts` release.
- Filled 2026-10-14 (research area 2): RFA tenders, game day and elevations, emergency third QB, practice squad eligibility, IR, comp picks, cap mechanics, 2026-27 dates. Gaps now listed in the file itself.

**nfl_teams.csv** (checked 2026-10-11)
- `market_size_rank` is now the Nielsen 2024-25 TV market (DMA) rank for the team's market, one source for all 32 (Wikipedia list of US TV markets). Teams sharing a market share its rank (Giants/Jets 1, Rams/Chargers 2). Arizona = Phoenix, Carolina = Charlotte, New England = Boston, Tennessee = Nashville, Washington = Washington DC.
- Bills: the new Highmark Stadium (opened 2026) has natural grass (Kentucky bluegrass), so `surface` is now grass.
- SoFi Stadium `roof` is now "canopy": a fixed translucent roof with open sides.
- Earlier fixes kept: Browns' Huntington Bank Field and Commanders' Northwest Stadium.
- Gap: `altitude_ft` still has no per-figure source.

**Research area 1 files** (added 2026-10-14, explained in `docs/research/engine-and-tactics.md`):
- `engine_targets.json` (`scripts/engine_targets.py`): NFL 2021-25 runs by box count, pressure and rushers, coverage and route results, play-action, RPO, motion, snap shares, carry and target shares, red zone, returns, player spread by position, contracts by position, home edge 1999-2025, and the 2025 scheme mix. From nflverse pbp, participation (NGS), FTN charting, snap counts, contracts and nfldata games.csv.
- `coach_effect_offense.json`, `coach_effect_defense.json` (`scripts/coach_effect.py`): how much head coaches move EPA per play once the QB is held fixed, 2006-2025.
- `college_scores.json` (`scripts/college_scores.py`): points, margins, blowouts and home edge for FBS, FCS, D2 and D3 from the sportsdataverse `cfb_schedules` release.
- Gaps: no open play-by-play for D2, D3, NAIA, JUCO or high school; no NAIA or JUCO scores; no high school scheme survey.

## Main sources
- Wikipedia lists: FBS, FCS and D2 programs; FBS and FCS stadiums; D2 institutions; conference member pages; national championship pages; AAU members; Carnegie R1/R2 list; US TV markets; current NFL stadiums.
- Open datasets: sportsdataverse `cfb_schedules` and `cfb_recruits` releases, cfbfastR-data `rosters/csv`, nflverse `draft_picks` and `contracts` releases.
- MaxPreps 2026 state rankings; SI 2026 state playoff guide.
- On3, 247Sports, Axios, Hudl, NCAA.org, NIL Wire (Opendorse), SI/CBS, NFL.com, Bengals.com, Yahoo Sports, NBC Sports.

**combine_by_position.csv** (added 2026-10-13): mean and standard deviation per position for height, weight, 40, bench, vertical, broad jump, 3-cone and shuttle. From the nflverse `combine` release, 2000-2025. Used by `crates/afm-engine/src/body.rs`.

**draft_positions_2000_2025.csv** (added 2026-10-13): each position's share of pick 1, picks 1-5, round 1 and later bands, from the nflverse `draft_picks` release, 2000-2025. Used to calibrate positional draft value. Example: QBs went first overall 73.1% of the time, EDGE 19.2%, OT 7.7%.
- Gap: no open dataset has arm length and hand size for every player. Their formulas in `body.rs` come from the 2025 combine OL and QB lists (CBS) and are approximate.

**nfl_game_targets.json** (added 2026-10-13): real NFL game numbers for the match engine checks (section 3, topic 10), from nflverse play-by-play, regular seasons 2021-2025 (2,718 team games). Per team game: plays, passing, rushing, sacks, penalties (accepted, by team), third and fourth downs, EPA. Rates: completion 64.5%, sack rate 6.75%, 4.50 yards per carry, 22.5 points per team game. Field goals by distance (98% under 30 down to 69% from 50+), punts, kickoff touchbacks, season leaders, and an expected-points table by down, distance and field position (nflfastR's EP, averaged per cell) that the engine uses for EPA. Built by `scripts/nfl_game_targets.py`.

**college_game_targets.json** (added 2026-10-13): the same for FBS-vs-FBS and FCS-vs-FCS regular-season games, 2022-2024, from cfbfastR play-by-play (sportsdataverse-data release `cfbfastR_cfb_pbp`). FBS: 27.0 points per team game, completion 60.6%, 5.12 yards per carry. Built by `scripts/college_game_targets.py`.
- Penalties there count accepted flags on plays by either team, per team game (about 5.5, so about 11 per game).
- Gap: no open play-by-play for high school. High school checks will use MaxPreps scoring and blowout rates when researched.

**position_value.json** (added 2026-10-14): what real NFL play says about position value and depth, for the engine physics fixes. From nflverse play-by-play and participation data, 2021-2025: designed-run yards per carry (4.29) and the yardage buckets, yards per carry by box count (6 or fewer 4.73, 7 4.22, 8 or more 3.40), the spread of QB EPA per dropback (sd 0.12) and CPOE (sd 3.0), the spread of RB yards per carry (sd 0.57), pressure rate (29.5%), sack rate, sacks per pressure and time to throw (2.71 s). Built by `scripts/position_value.py`. The engine's position values are measured with `cargo run --release --bin dynasty_check -- value`; the real bands it is held to (QB first by a clear gap, lines and pass rush near the top, RB and TE near the bottom) are Claude's reading of the draft and contract markets, proposed, not confirmed.
- Gap: snap-share bands (defensive line starters 60-75% of snaps, the lead back 55-65% of carries) come from published participation summaries, not from a dataset in this folder.

**Engine-made table, not real-world data**: `crates/afm-engine/data/call_values.csv` is what each pass and run concept gains against each defensive look, played on the match engine between two average NFL teams (`cargo run --release --bin call_values -- 400 > crates/afm-engine/data/call_values.csv`). It is the football knowledge coordinators call from. Rebuild it after any engine change.
