# Dynasty proof (2026-10-15)

Cain asked how we can be sure the dynasty fixes work, after earlier trials promised a fix and the dynasty stayed. This file tests rough versions of trial 4 proposals 1-3 before anything is built. Every fix here is **proposed, not confirmed**. Nothing in the game changed: the rough versions are switches that are off by default, so every result with them off is the same as before.

## What "fixed" means, from real results

All FBS-against-FBS games 2011-2025 (sportsdataverse `cfb_schedules`, CFBD data), rated per season from the scores (each team gets a rating, with home field fitted as well). 2020 left out.

| | Real FBS | World trial 5 (three worlds) |
|---|---|---|
| Best team, average margin against the rest of the top 12 | +10.7 (highest +15.8, Florida State 2013) | +22 to +23 |
| Best team over the median FBS team | +31 | +65 to +67 |
| Best team's title odds (semifinal against #4, final against #2 or #3) | 0.46 | 0.55 to 0.60 |
| Game luck (sd around the expected margin) | 13.2 | 18 |
| Home field | 3.0 points | 0.3 to 0.7 |
| Most titles in any 15 years | 6 (Alabama 2009-2023) | 8, 8, 7 |

So the world's best team is about twice as far ahead of everyone as the real one. Its title odds in a single season are only a little higher. The extra titles come from that gap and from the same program staying on top for years.

## The rough versions

- **1. Position values**: in every game, each player's attributes move toward his position's average, so a rating point at that position is worth the middle of the proposed band instead of what the engine pays now (RB 0.74 of a QB's value down to 0.10, WR 0.44 to 0.23, CB 0.37 to 0.19, and so on). Quarterbacks and linemen are left alone. A blunt stand-in: the real fix changes the physics, position by position.
- **2. Schemes**: every team on the same schemes (the limit of "no scheme wins by itself").
- **3. Garbage time**: up 28 in the 3rd quarter or 21 in the 4th, or down 28 in the 4th, a team plays its backups; the leader runs (pass rate at most 20%) and uses the full play clock.

## Test 1: replay the 2021 and 2025 rosters (seed 2026, 15-year world)

Top 12 lineups in a round robin (40 games a pairing), plus every FBS team against 24 random FBS opponents.

| 2021 (Texas A&M on top) | Leader's edge on the top 12 | Title odds | Best over median | Team quality sd | Decided by 28+ | 40+ | 60+ |
|---|---|---|---|---|---|---|---|
| As is | +37.0 | 0.87 | 84.7 | 19.9 | 37.6% | 23.1% | 9.3% |
| 1 only | +20.0 | 0.57 | 43.8 | 10.8 | 23.9% | 10.9% | 2.2% |
| 3 only | +30.8 | 0.84 | 66.0 | 17.7 | 33.2% | 19.5% | 6.4% |
| 1 + 3 | +19.4 | 0.58 | 35.8 | 9.8 | 22.4% | 8.3% | 1.4% |
| 1 + 2 + 3 | +22.1 | 0.63 | 41.7 | 9.8 | 23.0% | 9.5% | 1.6% |

| 2025 (Texas on top) | Leader's edge on the top 12 | Title odds | Best over median | Team quality sd | Decided by 28+ | 40+ | 60+ |
|---|---|---|---|---|---|---|---|
| As is | +21.0 | 0.49 | 61.1 | 19.4 | 37.9% | 23.2% | 9.4% |
| 1 only | +10.9 | 0.35 | 35.9 | 10.7 | 24.0% | 11.0% | 2.2% |
| 3 only | +18.4 | 0.47 | 57.8 | 17.7 | 33.3% | 19.4% | 7.5% |
| 1 + 3 | +9.9 | 0.37 | 31.7 | 10.0 | 23.7% | 9.7% | 1.3% |
| 1 + 2 + 3 | +9.4 | 0.24 | 32.1 | 10.8 | 25.2% | 11.0% | 1.9% |
| **Real** | **+10.7 (max 15.8)** | **0.46** | **31.1** | **11.2** | **21.1%** | **6.7%** | **0.4%** |

- 2025 passes: the leader's edge and title odds fall to the real best team's.
- 2021 fails: the edge halves but stays above the real highest (+19 to +22 against +15.8). Why (inferred, not traced): that Texas A&M roster is an outlier in talent, and part of its edge sits at quarterback and on the lines, which the rough version leaves alone.
- Proposal 1 does most of the work. Proposal 3 alone takes off a fifth of the margin but barely changes who wins. Proposal 2 adds little once 1 and 3 are in.
- Still off with all three: game luck stays near 18 (real 13) and 60-point games at 1.4-1.9% (real 0.4%). Why (inferred): too many points per game (36 against 28: college calling, proposal 6) and no home field (proposal 4).

## Test 2: three 30-year worlds with 1 + 2 + 3 switched on (seeds 2026-2028)

| | World 2026 | World 2027 | World 2028 | Trial 5 (as is) | Real |
|---|---|---|---|---|---|
| Most titles in any 15 years | **6** (Ohio State) | **7** (Texas Tech) | **7** (Pittsburgh) | 8, 8, 7 | 6 |
| Fewest distinct champions in a 15-year window | 7 | 5 | 6 | 5, 4, 4 | 7-9 |
| Best team's average margin (mean of 30 seasons) | +27.7 | +29.0 | +30.4 | +53.9, +52.0, +57.3 | +22 to +39 |
| FBS programs with no winning season in the last 15 years | 3 | 2 | 5 | 13, 10, 13 | 0 |
| Titles in 30 years, top program | Ohio State 8 | Texas Tech 9 | Pittsburgh 11 | 9, 13, 10 | Alabama 6 (1996-2025) |

**Result: the test fails.** The three fixes bring the games to real (margins, blowouts, spread of team quality, far fewer hopeless programs), but two of three worlds still give one program 7 titles in 15 years, and Pittsburgh wins 11 in 30.

### Why the dynasty survives (traced in worlds 2027 and 2028)

| Texas Tech 2010-2024 | |
|---|---|
| Head coach | Rickey Lombardo, 2010-2023 (hidden recruiting 16.1, game management 16.8, scheme 17.1, all near the top of 1-20) |
| Lineup rank among FBS | 1-3 in 13 of 14 seasons under him |
| Titles | 7 in 15 |

| Pittsburgh 2000-2014 | |
|---|---|
| Head coach | John Tokarski, 2002-2014+ (hidden recruiting 19.6 of 20, game management 16.0) |
| Lineup rank among FBS | 1 in 8 of 11 seasons from 2005 |
| Titles | 7 in 15 (11 in 30) |

The second cause sits outside the match engine. An elite recruiter builds the #1 roster at any program, even a mid-prestige one like Pittsburgh, stays for 12-14 years, and reloads every year. Real football has a Saban too (Alabama, 6 titles in 15 years), but at a blue blood, and elite coaches at smaller jobs get hired away. This trace shows the pattern; it doesn't yet show which pieces carry it: how much of a class the coach decides against the program's pull, whether bigger jobs and the NFL try to hire him, and whether recruits avoid a crowded depth chart. That needs its own trace (new item 11).

## Tools and data
- `dynasty_proof` (test 1), `dynasty_worlds` (test 2) and `game/experiment.rs` (the switches, off by default): patch `dynasty-proof.patch` in this folder, branch `claude/dynasty-fix-5frp1p` (not pushed).
- `dynasty-proof.py` reads test 1's CSV; `real_edge.py` and `world_edge.py` measure the real and world tables above.
- Raw outputs in this folder: `proof2026.csv`, `worlds-2026.md`, `worlds-2027.md` (with the Texas Tech trace), `worlds-2028.md`.
