# World trial 3

Seed 2026, 15 years of history (2011-2025), every college game played on the match engine (42,427 games). The world built in 14 minutes on 4 cores, about 40 seconds a season. Full output: `world-trial3-output.txt`. Run it with `AFM_DASH=dash.json AFM_CARDS=cards.json cargo run --release --bin world_trial -- 2026 15`.

New for this trial: the world can keep a game log (`WorldOptions::keep_game_log`, trial tooling only). It records each college game's box score totals and every player's season stat line. It only watches the games, so a seed plays out the same with it on. The dashboard reads it for the engine table, the stat leaders and the college stat lines on player pages.

Each result says why. "Inferred" marks a cause I read from the numbers but did not trace in the code.

## What is off

### 1. Dynasties are back: Tennessee won 9 of 15 titles
Tennessee went 198-13 under Jamie Fee (2004-2024), with six unbeaten seasons. Real high over any 15 years: 6 (Alabama 2009-2023).
- Why: Tennessee's lineup was the best or near the best every year (strength 69-73, ranked #1-#4), and the match engine turns a small lineup edge into wins much more reliably than the old stand-in did. In 2016 and 2017 Tennessee ranked only #3 and #4 by lineup and still went 15-0 and 16-0. Lineup strength and win share correlate at 0.62 across FBS. The fix list's "capped at 6 titles" was measured with the stand-in history, so it never saw this.
- Inferred: the extra edge likely comes from Fee's coaching (calling and game management feed the engine). He retired in 2024, so his hidden ratings aren't in this dashboard's coach list.

### 2. Three Power 4 programs never had a winning season in 15 years
Boston College, Iowa State and Wake Forest went the whole history without one (Arkansas and Kansas 12, Maryland 11). Real high is about 16 seasons over 30 years.
- Why: their lineups averaged 59-61 against Tennessee's 70+, and the engine rarely lets the weaker team win (25% of FBS games end 28+ points apart, average margin 19.5). Their prestige slid (0.62-0.67 to 0.54-0.58), so recruiting couldn't climb back. Their head coaches kept getting fired, and each new one inherited the same thin roster.

### 3. Coaches of color fell well short
6 Black FBS head coaches in 2025 (real 16), 2 at the low point in 2015, 7 in 2011 (real 2009: 7). In 2025 every FBS head coach is Black or white. Coordinators of color are 11%, position coaches 12%.
- Why: the coordinator pool is the problem. With 11% of coordinators being coaches of color, head-job searches rarely find one. The fix list's check run gave 17-20 in 2025, but that run used the stand-in history.
- Not found: why the engine history changes the pipeline this much. Needs a trace before any fix.

### 4. The first overall pick is a QB only 45% of the time
7 of 15 here, 45% over the full record (real 73%). EDGE 36% (real 19%), OT 18% (real 8%).
- Why: the best EDGE or OT on the board often grades clearly above the best QB, and the QB premium at pick 1 doesn't close a big gap. Picks 1-5 look right (QB 29% vs 28%).
- Round 1: CB 16% (real 12%) and EDGE 13% (real 17%) are still off.

### 5. Three-stars and two-stars are drafted too rarely
Five-stars 62.5% (real 63%, good). Four-stars 23.0%. Three-stars 4.6% (real 6.0%). Two-stars 1.1% (real 2.1%).
- Why (inferred): scouts' star ratings track true ability closely, so few low-rated players turn out to be draftable. Real three-stars include late bloomers and players scouts missed. 11 players with two stars or fewer reached rounds 1-2 from below FBS, which shows the path exists but is narrow.

### 6. The match engine in the world: too much offense
Per team game, FBS (real 2021-25 in brackets): points 35.2 (27.0), plays 76 (67), pass rate 62% (50%), completion 58.6% (60.6%), yards per carry 5.85 (5.1), sack rate 2.9% (6.1%), INT rate 1.6% (2.6%), third down 40.3% (40.3%, good), kick return 10.3 yards (about 21), home teams win 49.8% (real about 57%). FCS, D2 and JUCO look the same.
- Why: these are the known engine gaps in `docs/match-engine.md`. College teams call plays from NFL tables, so they pass like the NFL. Sacks and interceptions are low at every level. College kickers are weak, so kickoffs land short and returns are short. New here: there is no home-field edge (`quick_game` uses a neutral crowd).

### 7. College stat lines are unreal
The FBS leaders in the seasons checked (2015, 2020, 2025): 5,800-7,400 passing yards (record 5,967), 3,950-4,350 rushing yards (record 2,628), 56-97 sacks (record 24), 2,000-2,400 receiving yards (record 2,060).
- Rushing: Utah's lead back took 92% of the team's carries in 2025 (real lead backs take about 60%). The depth chart has no rotation for backs.
- Sacks: the 2025 leader is a kicker at California with 94 sacks. The depth chart picks players by their rating at each spot, and generated kickers are too athletic (drafted kickers run a 4.54 forty against the real 4.91), so he rates as a pass rusher. California's defense had 115 sacks in 14 games.
- Passing: Georgia threw for 10.6 yards per attempt in 2025. Strong teams against weak ones run up totals (see item 2).

### 8. Measurables
Drafted kickers 209 lb (real 197) and punters 222 lb (real 214), and both run like skill players. Quarterbacks run 4.57 (real 4.82) and broad jumps run 5-12 inches long at most positions.
- Why: the combine reads athletic attributes, and kickers and quarterbacks get the same athletic spread as everyone else. This also feeds item 7.

### 9. Names repeat
46.5% of 779,551 players share a full name with another active player.
- Why: the name pools by ethnicity, state and birth year are smaller than a real-size world needs.

## What looks right
- Five-star draft rate 62.5% (real 63%). FCS players are 7.0% of picks (real about 6%). D2 gives about one pick a year, all late.
- FBS high school signees by home state track the real shares (TX 13.5% vs 14.2%, CA 8.9% vs 8.9%).
- Third-down conversion 40.3%, the same as real FBS.
- Rises have causes: Georgia Tech rose 0.69 to 0.84 after two hires (Stephen Taylor is 55-9). North Dakota State fell 0.57 to 0.43 through seven head coaches in 15 years.
- Five-star busts have reasons on their pages. JUCO paths work: Preston Owens went JUCO for grades (1.72 core GPA), then Tennessee, then pick 4.

## Candidates for the next round (not fixed, per the working rules)
1. Dynasty strength under the engine (item 1), and why one coach turns small edges into near-perfect seasons.
2. Long Power 4 droughts (item 2): blowout share and margin.
3. The coach-of-color pipeline under engine history (item 3).
4. Pick-1 QB share and round-1 CB/EDGE (item 4).
5. Three-star and two-star draft rates (item 5).
6. Engine: college play calling, sacks, interceptions, kick returns, home-field edge (item 6).
7. Depth charts: rotation for backs, and specialists playing defense (item 7).
8. Specialist and QB athleticism (item 8).
9. Bigger name pools (item 9).
10. The career stats record and high school games on the engine are still unbuilt.
