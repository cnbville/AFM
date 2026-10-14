# Research area 4: player development, careers and injuries

Researched 2026-10-14 by Claude for Raichel. Research only. Labels: **[published]** (someone else's study, linked), **[computed]** (Claude's arithmetic on raw real data, dataset named; script `data/real-world/scripts/development_injuries.py`, output `data/real-world/development_injuries.json`), **[estimated]** (Claude's judgment). "Means" lines and proposals are Claude's reading, not data.

It feeds: section 6b (training, development and injuries), the player model (`docs/development-model.md`), the match engine's in-game injuries (`docs/design/match-engine.md`), and roadmap Q29 and Q39 (medical staff, return dates).

Data: nflverse (https://github.com/nflverse/nflverse-data/releases): snap counts 2012-2025, game-status injury reports 2012-2025, play-by-play 2006-2025, player birth dates, and draft picks with career totals (Pro-Football-Reference).

## 1. When players are at their best

**[computed] Starter ages** (player-seasons with 8+ games and 60%+ of the unit's snaps, 2012-2025, age on Sept 1):

| Position | Starter seasons | Median age | Most common age | Share 30+ | Share 33+ |
|---|---|---|---|---|---|
| QB | 446 | 27 | 24 | **35%** | **21%** |
| OL | 2,114 | 26 | 25 | 20% | 4.4% |
| EDGE (DE/OLB) | 474 | 26 | 24 | 19% | 4.0% |
| Interior DL | 391 | 27 | 26 | 19% | 3.6% |
| TE | 381 | 26 | 24 | 19% | 6.0% |
| S | 875 | 26 | 26 | 16% | 4.5% |
| LB | 1,076 | 26 | 26 | 15% | 3.7% |
| WR | 999 | 25 | 25 | 13% | 2.5% |
| CB | 1,097 | 26 | 24 | 13% | 2.4% |
| RB | 192 | 25 | 25 | **3%** | **0%** |

(Few running backs reach 60% of snaps, so the RB row is small.)

**[computed] Chance a starter is still a starter next season**: about 70-75% for most positions in their early-to-mid 20s. It falls to about 50% at 30 for corners, linebackers and receivers, and about 60% for linemen at 30.

**[computed] QB efficiency by age** (the same QB's change in EPA per dropback from one season to the next, 2006-2025): about flat from 23 to 31 (ups and downs of ±0.02), then a clear fall: -0.03 from 32 to 33 and -0.035 from 33 to 34. This counts only QBs who kept playing, so the real decline is a bit steeper.
- Not used: the same method for RBs, WRs and TEs gives a steady fall from age 21 on. That is mostly an artifact (per-play efficiency for these spots is noisy, and a career-best season is usually followed by a worse one), so it is not reported as an age curve. **[estimated]**

**[published]** PFF's fantasy aging study (2000-2017, RBs and WRs with a 1,000-yard season; Barrett, 2018-03-31: https://www.pff.com/news/fantasy-football-metrics-that-matter-aging-curves-by-position): WRs score 74% of their career average as rookies, beat it by year 2, peak around their fifth season, and fall below it by year 8. RBs score 88% as rookies and are most valuable early in their careers. TEs score just 33% of their average as rookies and 94% in year 2.

**Means:** a real peak window by position. RBs peak earliest and are almost gone by 30. WRs and CBs decline from about 29-30. Linemen and pass rushers last a little longer. QBs hold their level into their early 30s and are the only position where a third of starters are 30+. Receivers (and especially TEs) have a big jump from year 1 to year 2.

## 2. How long careers last

**[computed]** Drafted players 2000-2015, years from the draft to their last season (PFR):

| Round | Median years | Never played a game | 5+ years | Ever a Pro Bowl |
|---|---|---|---|---|
| 1 | 9 | 0% | 89% | 45% |
| 2 | 7 | 1% | 80% | 21% |
| 3 | 6 | 3% | 63% | 10% |
| 4 | 5 | 6% | 55% | 8% |
| 5 | 4 | 13% | 47% | 6% |
| 6 | 4 | 19% | 39% | 5% |
| 7 | 3 | 25% | 29% | 2% |

By position (median years): punters 8.5, kickers 6, OL, EDGE and TE 6, QB 5.5, CB, LB, RB, DL and S 5, WR 4. Ten or more seasons: punters 47%, kickers 34%, QBs 28%, EDGE 22%, interior DL 22%, OL 21%; RBs 11%. Undrafted players' careers are shorter; there is no open count.

## 3. Breakouts

**[computed]** Receivers who had under 500 yards in a season, and reached 900 or 1,200 the next season (2006-2025):

| NFL year | Receivers under 500 | Reached 900 | Reached 1,200 |
|---|---|---|---|
| 1 to 2 | 358 | 2.5% | 0.6% |
| 2 to 3 | 313 | 2.9% | 1.0% |
| 3 to 4 | 250 | 2.4% | 0.8% |
| 4 to 5 | 196 | 5.1% | 1.0% |
| 5 to 6 | 150 | 2.0% | 0% |

**Means:** a sudden jump from a minor role to a 900-yard season happens to about 1 receiver in 40 in any year of his career, and a jump to 1,200 to about 1 in 100. Breakouts are real but rare, and not tied to one "breakout year".

**[computed] Late bloomers from college** (area 3): about 36% of NFL picks were 3-star recruits and 25% were unrated or not matched. So the biggest real development swings happen between high school and the NFL, not inside an NFL career.

## 4. Injuries in the NFL

**[computed]** From game-status reports 2012-2025. An "episode" is a player listed Out or Doubtful after not being listed the week before. Games missed = team games with no snap for him until he plays again, counted to the season's end. Players put straight on injured reserve without a report entry are missed, so long injuries are undercounted.

By body part:

| Body part | Share of episodes | Mean games missed | 4+ games missed | Out for rest of season |
|---|---|---|---|---|
| Knee | 17% | 3.5 | 32% | 32% |
| Ankle | 14% | 2.8 | 24% | 25% |
| Hamstring | 13% | 2.6 | 20% | 14% |
| Concussion | 11% | 2.2 | 14% | 24% |
| Shoulder | 6% | 3.2 | 28% | 39% |
| Foot | 4% | 4.2 | 43% | 37% |
| Groin | 4% | 2.7 | 22% | 17% |
| Calf | 4% | 2.7 | 21% | 25% |
| Back | 3% | 3.9 | 35% | 42% |
| Neck | 2% | 4.1 | 40% | 47% |

(The "out for rest of season" share includes injuries late in a season, so it overstates truly season-ending injuries.)

By position, episodes per player-season (players with 4+ games): CB 0.51, WR 0.47, S 0.45, TE 0.44, RB 0.44, OL 0.43, LB 0.42, EDGE 0.39, QB 0.38, interior DL 0.35, K 0.12, P 0.06. Games missed per player-season: CB 1.5, OL 1.5, QB, TE and WR 1.4, RB 1.3, LB 1.2, S 1.2, EDGE 1.2, DL 1.0, K 0.6, P 0.25.

**Means:** a regular player has about a 40-50% chance a season of an injury that keeps him out, and misses about 1-1.5 games a season on average. Corners and receivers (speed and hamstrings) get hurt most; specialists rarely.

## 5. After a major injury

- **[published] ACL, NFL RBs and WRs** (Manoharan et al., Orthopaedic Journal of Sports Medicine, 2021: https://journals.sagepub.com/doi/pdf/10.1177/2325967120974743): 76 players, 2009-2016. **62% returned** (RBs 65%, WRs 60%), on average **13.6 months** later; none in under 9 months. Returning WRs fell clearly (yards per catch 12.7 to 9.9, catches per game 2.9 to 2.0). Players with more games before the injury returned more often (72 career games for returners, 40 for those who didn't).
- **[published] Achilles, all pro sports** (systematic review, Foot & Ankle International, April 2021, via AOFAS 2021-07-13: https://www.aofas.org/news/press-releases/2021/07/13/the-impact-of-achilles-tendon-ruptures-on-professional-athletes-careers): 333 athletes; **about 24% never returned**; returners needed about 11 months; performance, durability and career length were lower afterwards, notably in the NFL.

## 6. Injuries in college and high school

**[published]** NCAA and high school injury surveillance (Kerr et al., Journal of Athletic Training, 2018: https://pmc.ncbi.nlm.nih.gov/articles/PMC6188086; data 2004-2014 college, 2005-2014 high school). Injuries that kept a player out at least a day, per 1,000 athlete-exposures (one player in one practice or game):
- High school: 4.0 overall; 12.7 in games, 2.3 in practice.
- College: 7.3 overall; 32.1 in games, 4.9 in practice.
- Most injuries heal within a week (40-53%). 20-26% last more than 3 weeks.
- Most common: sprains. Concussions are 20% of high school game injuries. Knees lead in college games (20%).

**Means:** games are 5-7 times more dangerous than practice, and college games far more than high school games, which fits harder hitting at higher levels.

## Gaps
- No open data on rookie-to-peak development in college (no per-player college grades are open).
- Exact injury types (ACL vs other knee) are not in the injury reports; the type-level return rates come from studies.
- Undrafted players' career lengths.

## Proposals for Raichel (Claude's ideas, proposed, not confirmed)
1. **Peak windows by position** from section 1: RBs peak at 24-26 and fall fast after 28, WRs and CBs decline from about 29-30, linemen hold a little longer, QBs hold into their early 30s and decline from about 32. Tie them to the athletic attributes (speed fades first) and the mental ones (they keep growing), so the curves come out of the attributes, not from flat rules.
2. **A big year 1 to year 2 jump** for receivers and tight ends (learning the playbook), as in PFF's study.
3. **Injuries:** use the real body-part mix and games missed, with risk by position (corners and receivers highest) and games 5-7 times riskier than practice. Major knee and Achilles injuries take about a year, and about a quarter to a third of players never get back, more often for players with fewer games played before.
4. **Careers by draft round:** use the round table as a check for the world (for example, about 25% of 7th-rounders never play a game).
