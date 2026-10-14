# Research area 1: engine and tactics targets

Researched 2026-10-14 by Claude for Raichel. Research only: nothing here changes the engine or a design spec. Where a finding points to a design change, it is listed under "Proposals for Raichel" and is **proposed, not confirmed**.

It feeds: the engine physics fixes (`docs/design/engine-physics-fixes.md`, especially its "Checks before the next trial"), the plays and tactics spec (`docs/design/plays-and-tactics.md`, its "research still to do" list), and trial 3 finding 6 (scoring, home edge).

## What kind of evidence each figure is

Every figure carries one of three labels:
- **[published]**: from a study or official source someone else wrote; the link is next to it.
- **[computed]**: Claude calculated it from raw real data (the dataset is named). Real data, Claude's arithmetic; the script is in `data/real-world/scripts/` so it can be checked.
- **[estimated]**: Claude's own judgment or model on top of real data, such as an upper limit or a guess at a cause. Treat it as a reasoned guess.

Every "Engine target" line is **[estimated]**: it is Claude's reading of what the real numbers mean for the engine. Every item under "Proposals for Raichel" is Claude's idea, not data.

Words used here:
- **EPA** (expected points added): how many points a play adds to the offense's expected score. +0.1 per play is a big edge; over 62 plays it is about 6 points a game.
- **Success rate**: share of plays that gain enough (40% of the distance on 1st down, 60% on 2nd, all of it on 3rd and 4th).
- **Box**: defenders lined up near the line, inside the tight ends, at the snap.
- **Pressure**: the QB is hurried, hit or sacked.

## Where the numbers come from

All NFL numbers are computed from open nflverse data, regular seasons 2021-2025 unless a line says otherwise. The scripts are in `data/real-world/scripts/` so anyone can rerun them:
- `engine_targets.py` writes `data/real-world/engine_targets.json` (box counts, pressure, coverage, routes, play-action, snap shares, scheme mix, red zone, returns, player spread, contracts, NFL home edge).
- `coach_effect.py` writes `data/real-world/coach_effect_offense.json` and `coach_effect_defense.json` (the play-caller gap).
- `college_scores.py` writes `data/real-world/college_scores.json` (scoring, margins and home edge for FBS, FCS, D2 and D3).

Data files used (all open):
- Play-by-play with EPA: https://github.com/nflverse/nflverse-data/releases/tag/pbp
- Participation (box count, rushers, pressure, coverage, routes, from NFL Next Gen Stats): https://github.com/nflverse/nflverse-data/releases/tag/pbp_participation
- FTN charting (play-action, RPO, screens, motion, blitzers), 2022-2025: https://github.com/nflverse/nflverse-data/releases/tag/ftn_charting
- Snap counts (Pro-Football-Reference): https://github.com/nflverse/nflverse-data/releases/tag/snap_counts
- Contracts (OverTheCap): https://github.com/nflverse/nflverse-data/releases/tag/contracts
- NFL games with closing lines, 1999-2025: https://raw.githubusercontent.com/nflverse/nfldata/master/data/games.csv
- College scores with D2 and D3 (CFBD): https://github.com/sportsdataverse/sportsdataverse-data/releases/tag/cfb_schedules

## 1. How much each position is worth

Three independent views agree on the order. The QB is first by a wide gap.

**[published] PFF WAR** (wins above replacement), mean per season for regular players, 2006-2018 (Eager et al., Sloan 2020, Table 2): https://cdn.prod.website-files.com/5f1af76ed86d6771ad48324b/5f6a67e0957a471f8a59fdf5_Eager_PFF_WAR.pdf

| QB | WR | CB | S | TE | LB | RB | G | C | T | DI | EDGE |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1.63 | 0.28 | 0.23 | 0.23 | 0.18 | 0.11 | 0.10 | 0.10 | 0.10 | 0.09 | 0.06 | 0.06 |

The best single seasons: QB 5.54 (Brees 2011), WR 1.36, CB 1.19, S 0.86, TE 0.68, DI 0.57, G and C 0.46, EDGE 0.42, T 0.37, RB 0.34. PFF's college version (top 32 per position, 2020) also ranks QB first by double (18.2 against WR 9.1): https://www.pff.com/news/nfl-using-pro-adjusted-wins-above-average-to-examine-positional-value-in-the-nfl-draft

**[computed] What teams pay** (OverTheCap contracts via nflverse; top 5 contracts as a share of the cap, active deals signed 2022 or later, `contracts_cap_pct`): QB 23.4%, EDGE 15.8%, WR 13.6%, interior DL 11.7%, CB 10.8%, LT 10.0%, RT 9.7%, guards 8-9%, S 8.1%, LB 7.5%, RB 7.4%, C 6.7%, TE 6.3%, K 2.3%, P 1.5%. The market rates pass rushers and tackles higher than PFF WAR does. That gap is a known weakness of grade-based WAR for linemen, so the engine should not copy WAR alone.

**[computed] What wins** (nflverse play-by-play and nfldata games; `what_wins`, 160 team seasons): a team's pass offense EPA per play correlates 0.82 with its point margin; run offense 0.53; pass defense -0.64; run defense -0.38. Teams also differ more in passing (spread 0.12 EPA per play) than in running (0.07).

**[computed] How much players really differ, per game** (`player_spread`, 10th to 90th percentile, not causal because it mixes the player with his team):
- QBs: -0.15 to +0.18 EPA per dropback. At 37.5 dropbacks a game that is about **12 points a game**.
- RBs: -0.17 to +0.05 EPA per carry, about **3 points a game** at 15 carries.
- WR and TE: about **3.6 points a game** at 7 targets.
- How much of it carries over to the next season (signal, not luck): QB 0.43, WR/TE 0.28, RB 0.05. A running back's efficiency is mostly his line and the defense he faces.

**Engine target [estimated].** Points per rating point should rank QB clearly first (several times any other single player), then the pass rush, the receivers, the corners and the offensive line as groups, with RB and TE near the bottom. A good band to aim for: a QB from the 10th to the 90th percentile of starters moves a team about 10-12 points a game; an RB about 2-3; a single WR about 3. Today the engine has QB +10 rating at about +0.2 points (`engine-physics-fixes.md`), so it is far off.

## 2. Snap shares and rotation (depth)

**[computed]** from Pro-Football-Reference snap counts via nflverse, per team game (`snap_shares`). "Rank 1" is whoever played most at the position that game.

| Position | Rank 1 | Rank 2 | Rank 3 | Rank 4 | Rank 5 | Players with 10%+ |
|---|---|---|---|---|---|---|
| QB | 96% | | | | | 1.1 |
| RB | 65% | 32% | 11% | | | 2.6 |
| WR | 87% | 76% | 56% | 24% | 9% | 4.3 |
| TE | 74% | 40% | 16% | | | 2.7 |
| OL | 100% | 100% | 99% | 98% | 88% | 5.6 (6th man 14%) |
| Interior DL | 68% | 50% | 31% | 16% | | 3.5 |
| EDGE | 67% | 45% | 26% | 13% | | 3.2 |
| Off-ball LB | 96% | 82% | 53% | 33% | | 4.7 |
| CB | 97% | 89% | 58% | 18% | | 3.6 |
| S | 98% | 86% | 28% | 5% | | 2.8 |

Notes: the EDGE row mixes 4-3 ends and 3-4 outside linebackers (PFR labels), and its rank 1 ranges from 41% to 88% (10th to 90th percentile), because some teams ride one star rusher. The OL rank 5 at 88% includes injury games.

**[computed]** (nflverse play-by-play) Carries: the lead back takes **70% of his team's RB carries in a game** (10th-90th: 50-92%) and **59% over a season** (43-76%; lower because he misses games). The second back takes 26% over a season. QBs take 6% of designed runs. Targets: the top target gets 23.5% of the team's targets over a season, the second 17%, the third 13%. By position: WR 59%, TE 22%, RB 18%.

**Engine target [estimated].** The spec's bands are right for DL (starters 60-75%) and RB (lead back 55-65% per season). Add: OL, QB, corners and safeties near 100% for starters; off-ball LB rank 3 about 50% (it is the nickel swap); WR3 about 56%. Depth should come from these snap shares, so a backup's value is capped by how little he plays.

## 3. Runs by box count

**[computed]** from NFL Next Gen Stats box counts (nflverse participation). Designed runs only (`runs_by_box`). In "neutral" downs (1st or 2nd and 5+, between the 20s, game still open), so heavy boxes aren't just short yardage:

| Defenders in box | Share of runs | Yards per carry | Success | EPA | Runs for 0 or less |
|---|---|---|---|---|---|
| 5 or fewer | 3% | 5.12 | 39% | -0.07 | 14% |
| 6 | 48% | 4.85 | 39% | -0.06 | 15% |
| 7 | 37% | 4.68 | 36% | -0.09 | 16% |
| 8 | 11% | 4.39 | 33% | -0.14 | 21% |
| 9 | 1% | 3.89 | 30% | -0.22 | 17% |

Over all downs the drop is steeper (6 in the box 4.61 yards, 8 in the box 3.69, 9 in the box 2.30), but that includes goal line and short yardage.

**[computed]** Box minus blockers (linemen + tight ends + fullbacks) gives an almost flat line (4.5 to 4.9 yards at every count). **[estimated]** The likely reason is that offenses already pick runs into good looks and check out of bad ones, so the plays that are run into "+2" boxes are the ones the offense still liked. This is an inference, not measured.

**Engine target [estimated].** Running into an 8-man box should cost about 0.5 yards per carry and 5 points of success rate against a 6-man box in neutral downs; a 9-man box about 1 yard and 9 points. The stuff rate climbs from 15% to about 21%. The spec's "+2 box loses through the numbers" should land near these splits, not much harsher, because real runs into heavy boxes still average 4+ yards.

## 4. Pressure, rushers and blitzing

**[computed]** from NGS participation and FTN charting (nflverse), 102,020 dropbacks (`dropbacks`):
- Pressure on **28%** of dropbacks. Without pressure: +0.22 EPA, 70% completions, 7.5 yards per attempt, 2.0% interceptions. With pressure: -0.30 EPA, 49% completions, 6.0 yards per attempt, 3.2% interceptions, 13.9% sacks.
- Rushers: 4 on 67% of dropbacks (pressure 27%), 5 on 20% (37%), 6 on 5% (45%), 3 on 5% (20%). Each extra rusher adds pressure but EPA barely moves (4 rushers +0.03, 5 rushers +0.02, 6 rushers -0.01), because the offense gets open receivers.
- Blitz (FTN, any blitzer): 27% of dropbacks; sacks 8.3% with against 5.9% without; EPA 0.00 against +0.04.
- Time to throw: average 2.71 seconds; 10th percentile 1.6, median 2.5, 90th percentile 3.9.
- 41% of sacks are charted as the QB's fault (FTN). 18% of dropbacks leave the pocket.
- 3.5% of dropbacks have an interception-worthy throw; 5% of catchable balls are dropped.

**Engine target [estimated].** Pressure rate about 28%, with the split above. This also answers the sack gap (engine 4.1% against real 6.8%): real sacks come almost entirely from pressured snaps (13.9%), so the engine needs more pressure, not more sacks per pressure. That fits physics fix 2.

## 5. Coverage and concept against coverage

**[computed]** (NGS participation) Coverage mix (NFL 2025 dropbacks): Cover 3 28%, Cover 2 23%, Cover 1 22%, Cover 4 10%, Cover 6 5%, 2-man 5%, Cover 0 4%, Cover 9 3%. Man coverage 31% of snaps (teams range 18-45%). The labels changed in 2023-24 (Cover 1 jumps to 32-37%), so compare seasons with care.

By coverage (all dropbacks 2021-25): man gives fewer completions (57% against 69%) and more sacks (4.7% against 3.7%), but about the same EPA (+0.06 against +0.07). Cover 0 allows only 4.6 yards per attempt but 53% completions; Cover 3 is the softest (+0.10 EPA).

Route of the targeted receiver (EPA per target; NGS route labels):
- Best overall: post +0.53, corner +0.42, dig +0.32, wheel +0.32, go +0.29. Worst: swing -0.10, screen -0.05, flat +0.02, quick out +0.06.
- Against man and zone: go +0.20 man / +0.38 zone; corner +0.30 / +0.54; hitch/curl +0.04 / +0.20; flat +0.12 / -0.02; crosser +0.29 / +0.19; quick out +0.10 / +0.03.
- Against a coverage: corner route against Cover 3 +0.70 (the classic Cover 3 beater) against Cover 2-man +0.16; go against Cover 2 +0.54 (the hole between safeties) against Cover 0 +0.03; post against Cover 0 +1.09 (no deep safety); crosser against Cover 1 +0.35 against Cover 2 -0.06; hitch/curl against Cover 1 +0.03 against Cover 3 +0.21.

Full tables by route and coverage are in `engine_targets.json` (`targets_by_route_and_coverage`, at least 150 targets per cell). These are routes of the targeted receiver, not whole concepts; no open data labels full concepts. Real playbook sizes per level: no public source found.

**Engine target [estimated].** The engine should reproduce these classic pairings through geometry (the corner route wins against Cover 3, crossers win against man, curls die against man, the go route beats Cover 2 between the safeties). They work as development checks, as the tactics spec says.

## 6. Play-action, RPO, motion, screens

**[computed]** from FTN charting 2022-2025 via nflverse (`play_action`, `ftn_flags`):
- Play-action on **22%** of dropbacks (2025: 23%, teams 15-35%). EPA +0.12 with against 0.00 without; 8.1 against 6.8 yards per attempt; sacks 5.9% against 6.7%. Air yards are the same (7.8), so the gain comes from open receivers, not longer throws.
- **It works whether or not the run game works.** Splitting teams into quarters by their designed-run EPA: play-action gains +0.10 for the worst run teams, +0.16, +0.07, and +0.12 for the best. This confirms the spec's "the fake fools defenders through keys and discipline, not through rushing stats".
- RPO: 2.6% of dropbacks (passes off RPOs); 6.2% of all 2025 plays were RPOs. Pass RPOs: 79% completions, 2.9% sacks, +0.07 EPA.
- Screens: 8.8% of dropbacks, 85% completions, -0.07 EPA.
- Motion: 45% of dropbacks (55% of all 2025 plays, teams 39-71%). Passes with motion +0.04 against +0.01. Runs with motion and without are the same (4.3 yards).
- No huddle: 12% of dropbacks, almost no EPA change.

## 7. The real play-caller gap

No public study gives the gap between the best and worst play callers in points (checked: PFF, a 2021 play-calling study at https://mvpsportstalkcom.wordpress.com/2021/01/27/nfl-study-quantifying-offense-defense-play-calling/ which only estimates talent at about 1.5 times as important as coaching, and a 2024 play-caller ranking at https://pattonanalytics365.substack.com/p/evaluating-play-callers-part-two with no point figures). So Claude estimated it from 20 seasons of play-by-play (`coach_effect.py`).

Method in plain words: for each team game, offensive EPA per play is explained by the season, the main QB and the head coach. Because QBs and coaches move between teams, a coach is judged against what his QBs did elsewhere. Then the same fit runs on odd and even games separately; how well the two halves agree says how much of the spread is real and how much is luck.

**[estimated]** (Claude's own model, run on real nflverse play-by-play) Results (2006-2025, 10,398 team games, 85 head coaches with 48+ games):
- **Offense:** the real spread between head coaches is 0.044 EPA per play, about **2.7 points a game** (one standard deviation). From a 10th-percentile to a 90th-percentile coach: about **7 points a game**. Half the raw spread is real (split-half agreement 0.55).
- **Defense:** 0.030 EPA per play, about **1.9 points a game**; 10th to 90th percentile about **5 points**.
- This is an **upper bound** for play calling alone. The "coach" here carries everything that moves with him: his coordinators, scheme, game prep, and partly the roster his team builds. The head coach is not always the play caller.
- Compare with talent: a 10th to 90th percentile QB is about 12 points a game (section 1).

**Engine target [estimated].** A best-against-worst play caller with the same players should swing about 3-7 points a game in total (offense and defense together, with the play calling itself as only part of the coach effect). It should clearly move off zero (today it is about 0), but stay well under a great QB. This is the band the tactics spec asked for.

## 8. Home-field edge

**[computed] NFL** (nfldata games.csv; `home_field_nfl`, neutral sites removed): home teams won 57.3% with a 2.6-point margin in 1999-2009, 56.7% and 2.3 in 2010-2019, and **53.8% and 2.1 points in 2021-2025**. The betting market prices it lower: 1.6 points in 2021-25 against 2.5 in 1999-2009. 2020 (mostly no fans) was +0.1. So the edge has shrunk over time and is about 1.5-2 points today.

**[computed] College** (CFBD scores via sportsdataverse; `college_scores.json`, same-division games, 2014-2025 for FBS and FCS, 2021-25 for D2, 2022-25 for D3). Conference games are the fair test, because non-conference home games are often paid games against weaker teams:

| Level | Home win (conference) | Home margin (conference) | Home win (all) | Home margin (all) |
|---|---|---|---|---|
| FBS | 55.4% | 2.4 | 58.5% | 4.4 |
| FCS | 54.0% | 2.0 | 56.2% | 3.5 |
| D2 | 52.2% | 1.7 | 52.8% | 2.2 |
| D3 | 53.0% | 2.7 | 53.8% | 2.8 |

High school: no open dataset; no figure found.

**Engine target [estimated].** About 2 points at NFL, FBS and FCS, a little less at D2. It should come from real causes (crowd noise on the road offense's snap count and pre-snap penalties, travel), not a flat bonus. Trial 3 found the engine has none (finding 6).

## 9. Per-game baselines per level

NFL, FBS and FCS play-by-play baselines already exist (`nfl_game_targets.json`, `college_game_targets.json`). New here:

**[computed]** NFL extras (nflverse play-by-play; `red_zone_and_returns`): red-zone trips 4.0 per team game; **TD on 60% of trips** (scoring 85%). Kick returns averaged 22.1 yards (2021), 22.8, 19.9, 27.5 (2024 rule) and 25.9 (2025); 75% of kickoffs were returned in 2025 against 25-40% before. Punt returns: 70% returned, 5.8 yards per return (the field position effect of fair catches is in the data).

**[computed]** College scoring and margins (CFBD scores; `college_scores.json`, regular season, same-division games):

| Level | Points per team game | Decided by 8 or less | Decided by 28+ | Team shutouts |
|---|---|---|---|---|
| FBS (2014-25) | 27.6 | 37% | 21% | 1.5% |
| FCS (2014-25) | 26.5 | 37% | 21% | 2.2% |
| D2 (2021-25) | 26.4 | 34% | 28% | 3.4% |
| D3 (2022-25) | 26.7 | 26% | 37% | 5.7% |

FBS scoring fell from 29.1 (2016) to 26.2 (2025). The NFL has 54% of games decided by 8 or less (`nfl_game_targets.json`). Lower levels have the same scoring but far more blowouts, because the gap between teams is wider.

**[published]** High school: MaxPreps (Cal Ryan McKnight's score database, 5.2 million games, article 2026-09-16, https://www.maxpreps.com/news/tFMYruGoRkGg_8HK_0tcYw/high-school-football-spread-offenses,-passing-game-have-led-to-dramatic-increase-in-scoring,-winning-margins.htm): in the 2020s the winner scores nearly 39 and the loser nearly 13, so about **26 points per team** and an average **winning margin of nearly 26**. In the 1940s-1980s the margin was about 18. State scoring differs (2013 season, both teams together: Texas 55.5, Arizona 53.9, Maryland 45.3; https://www.maxpreps.com/news/lAg6K1RUPEG8rbKhcSN0CQ/comparing-high-school-football-scoring-rates-from-state-to-state.htm). Mercy rules usually start a running clock at a 35-point lead in the second half, set state by state.

Gaps, said plainly:
- D2 and D3 plays, pass rate, sacks, interceptions, red zone: no open play-by-play. NCAA.com stats pages would not load here.
- NAIA and JUCO: no open score or stat dataset found.
- High school plays, pass rate and sacks: no source. Only scores.

**Engine target [estimated].** High school teams should score about 26 per game with a winning margin near 26 (the engine gives 32 per team on 83 plays). D3 should blow out far more often than FBS. The other per-level stats stay as in the existing targets files.

## 10. The 2026 scheme mix

**[computed] NFL, 2025 season** (NGS participation and FTN charting; `scheme_mix_nfl`): personnel 11 (1 RB, 1 TE, 3 WR) 63%, 12 personnel 27%, 13 personnel 5%, 21 personnel 2%. Shotgun 61%, under center 34%, pistol 5% (teams range 41-88% shotgun). Pass rate 60% of plays. Motion 55%. Play-action 23% of dropbacks. RPO 6% of plays. No huddle 10%. Defense: nickel (5 DBs) 60%, base 30%, dime 9%; Cover 3 and Cover 2 are the top shells; man 31% (teams 18-45%); blitz 29% (teams 21-51%).

**College:** [computed] FBS passes on 50% of plays (`college_game_targets.json`). [published] Tempo: 33% of FBS snaps start with under 10 seconds on the play clock; West Virginia averaged 19.3 seconds left and USF went at tempo on 48% of plays (PFF, 2026-07-20, https://www.pff.com/news/college-football-the-5-fastest-operating-offenses-in-the-country). One team study (Nebraska 2025, https://black41flashreverse.substack.com/p/tendencies-2025-offense) shows RPOs at 21% of plays and shotgun above 80%, which is typical of college but not a national figure. No open source gives national college RPO, motion or coverage rates.

**High school:** no survey gives the share of offenses by system (spread, wing-T, flexbone, power-I). MaxPreps' scoring study credits the spread and the passing game for the rise in scoring. This is a gap.

## Proposals for Raichel (Claude's ideas, proposed, not confirmed)

1. **Use these as the "checks before the next trial"** in the physics fix spec: position values (section 1 bands), snap shares (section 2 table), box splits (section 3), pressure rate 28% with its split (section 4), play-caller swing of about 3-7 points (section 7). This needs your OK, because that spec lists the checks as proposed.
2. **Sacks through pressure.** The engine's low sack rate looks like too little pressure, not too few sacks per pressure. Fix it inside physics fix 2 (line play) rather than as a separate sack tweak.
3. **Home edge from real causes** at about 2 points (NFL, FBS, FCS), a little less at D2, using crowd noise on the road offense (false starts, snap count) and travel, not a bonus.
4. **High school scoring and margins:** target about 26 points per team with a winning margin near 26, and add the mercy-rule running clock at 35 points where a state uses it.
5. **Play-action stays independent of rushing success** (confirmed by data, matches the tactics spec).
6. **Kick returns:** the 2025 NFL rule makes 75% of kickoffs live with a 26-yard average; the engine's 18 yards is short.
