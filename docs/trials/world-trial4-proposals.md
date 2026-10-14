# World trial 4: what I found and how I'd fix it

Written 2026-10-14 for Raichel, replacing the short proposal list in `world-trial4.md`. Every fix here is **proposed, not confirmed**.

Each topic has the same five parts: what I saw, why it happens, how I'd fix it, what it should look like after, and what I need from you. "Traced" means I found the cause in the code (file and line given). "Inferred" means I read it from the numbers but haven't proven it in the code. "Claude's reading" marks a real-world figure that isn't in our data yet.

**A correction first.** My first post said to add randomness to the battles so the better team wins less often. That was wrong. Game-to-game luck in the world is already bigger than in real college football (18 points of spread around the expected result against 13). More luck would hide the problem, not fix it. The real causes are topics 1 to 3 below.

**Where the real numbers come from.** Every FBS-against-FBS game from 2011 to 2025 (11,269 games, the cfbfastR schedule files), the college play-by-play targets already in `data/real-world/college_game_targets.json`, and the real drafts 2000-2025. The world numbers are every FBS-against-FBS game of trial 4 (12,580 games).

## The picture

| | World | Real FBS |
|---|---|---|
| Points per team per game | 36.0 | 27.8 |
| Games decided by 8 or less | 27.9% | 36.7% |
| Games decided by 28 or more | 31.1% | 21.1% |
| Decided by 40 or more | 17.1% | 6.7% |
| Decided by 60 or more | 5.5% | 0.4% |
| Best team's average margin, each year | +42 to +79 | +22 to +39 |
| Team-seasons averaging +35 or more | 62 | 1 |
| Highest-scoring team-season | 88.3 a game (Ohio State 2025) | 51.5 (Louisiana Tech 2012) |
| Spread of team quality (sd of a team's average margin) | 15.7 | 11.2 |
| Game-to-game luck (sd around the expected margin) | 18.0 | 13.0 |
| Home teams win | 50.7% | 58.4% |
| Value of playing at home | 0.7 points | 2.5 points |
| Most titles by one program in 15 years | 10 (Ohio State) | 6 (Alabama 2009-2023) |
| Programs with no winning season in 15 years | 12 | 0 |

The better team wins too often and by far too much, home field barely exists, and weak programs never recover. Topics 1 to 3 are the three causes of the first two. Topic 4 is the third.

---

## Topic 1. A better lineup is worth about twice too much

### What I saw
- Each point of whole-lineup rating is worth 4.9 points of final margin in the world. A 73-rated lineup against the FBS average of 61 wins by about 58 points on average.
- How the favourite does, by the gap between the two lineups:

| Lineup gap | Favourite wins | Average margin |
|---|---|---|
| 0-2 | 57% | +3.7 |
| 2-4 | 72% | +12.6 |
| 4-6 | 86% | +23.0 |
| 6-9 | 96% | +36.1 |
| 9-13 | 98% | +52.7 |
| 13+ | 100% | +76.1 |

- Ohio State kept a 69-74 lineup for a decade under William Myers (hidden recruiting 20.0, the top of the scale), won 10 titles and averaged +78.5 in 2025.

### Why it happens (traced)
- The match engine doc measures each position's value: raise one starter by 10 rating points and see how the margin moves over 1,000 games (`docs/match-engine.md`, "The measured position values"). Adding up all 22 starters gives 4.83 points of margin per lineup point. That matches the 4.9 the world shows, so this is the cause.
- The quarterback (0.55 points per rating point) is right: first by a clear gap. Almost everyone else is worth too much compared with him:

| Position | Share of the QB's value now | Band I proposed earlier (never confirmed) |
|---|---|---|
| RB | 0.74 | 0.05-0.15 |
| EDGE | 0.60 | 0.15-0.30 |
| S | 0.45 | 0.08-0.18 |
| WR | 0.44 | 0.15-0.30 |
| LB | 0.42 | 0.05-0.15 |
| DT | 0.41 | 0.10-0.25 |
| TE | 0.39 | 0.07-0.18 |
| CB | 0.37 | 0.12-0.25 |
| IOL | 0.18 | 0.08-0.18 |
| OT | 0.13 | 0.15-0.30 (too low) |

- If every position sat in the middle of its band, a lineup point would be worth about 2.4 points of margin, and the spread of team quality from players would be about 8 points. Add the coaching edge and it lands near the real 11.2.
- Why backs, linebackers and safeties are so high (inferred, not yet measured snap by snap): ten rating points make a back faster, quicker and stronger on every carry and catch, and a linebacker or safety a surer tackler on every run. In real football most of a run's yards come from the blocking and the box count. The back's own skill shows mostly in yards after contact, broken tackles and the rare long run.
- One more reason real stars are worth less than the engine says: real defenses take the best player away (double the pass rusher, bracket the receiver, spy the running QB). Those matchup plans are in the tactics design (topic 5.4) but not built.

### How I'd fix it (proposed, not confirmed)
1. **Check each position against real per-player numbers, not a team total.** For each position, compare how much better a top NFL starter is than a weak one in real life with how much better a 10-point-better player is in the engine:
   - Backs: rushing yards over expected per carry (NFL Next Gen Stats), yards after contact and broken tackles (Pro Football Reference).
   - Receivers and tight ends: separation and yards after catch over expected (Next Gen Stats), drops and broken tackles (PFR).
   - Pass rushers: pressures and sacks (PFR).
   - Linebackers and safeties: missed tackle rate, and completions and yards allowed in coverage (PFR).
   - Corners: completions and passer rating allowed per target (PFR).
   - Quarterbacks: completion over expected (Next Gen Stats), on-target and bad-throw rates (PFR).
   - Lines: no free per-lineman data exists, so team pressure and sack rates allowed against line strength.
   All of this is free in the nflverse data releases. College has no per-player tracking data, so college follows from the same physics.
2. **Measure the engine the way Next Gen Stats does.** The engine can replay the same snap with the same luck and swap one player for an average one. The difference is that player's exact "yards over expected".
3. **Fix the physics where the engine's spread is too wide, never with a flat factor.** For backs: the blocks and the box set the run's base yards; the back's ratings decide the cut (vision), yards after contact, broken tackles and the breakaway. For tacklers: angle and pursuit decide whether he gets there; his rating decides the finish. Everything stays attribute-driven, so a great back can still take over a game.
4. **Re-measure** the position table and the lineup value. Target about 2.4 points per lineup point, with the quarterback unchanged. The bands become a check on the result, not the target.
5. When the matchup plans from the tactics design are built, run the checks again, since part of a star's real limit is the defense's attention.

### What it should look like after
- A 73 lineup against a 61 wins by about 25-30, not 58 (before topics 2 and 3 take more off).
- The favourite still wins most games, but the underdog in a 4-6 point lineup gap wins about one in five instead of one in seven (an estimate: half the margin, with real game-to-game luck).
- A back's rushing average follows his line more than his own rating; star backs still show in long runs and broken tackles.

### What I need from you
- Use the real per-player spreads as the target for each position, with my old bands as a check only? Yes, or change it.

---

## Topic 2. Nobody takes their foot off the gas

### What I saw
- 5.5% of games end 60+ points apart (real 0.4%) and 17.1% end 40+ apart (real 6.7%).
- The margin keeps growing with the lineup gap: +52.7 at a 9-13 gap, +76.1 at 13+ (table in topic 1). In real football the margin flattens at the top, because the winner stops trying to score.
- Ohio State scored 88 a game in 2025. The stat leaders on the dashboard carry the same inflation.

### Why it happens (traced)
- Starters come out only when tired or hurt (`game/flow.rs`, substitution). No coach pulls his starters when the game is won.
- The leading team slows down only in the 4th quarter (38 seconds between snaps, `game/flow.rs:1785`). Its pass rate follows the NFL's habit by score. Nothing changes in the 3rd quarter of a 35-point game.
- No defense plays differently with a huge lead.
- High school has its mercy running clock (35 points). College and the NFL have nothing.

### How I'd fix it (proposed, not confirmed)
1. **The head coach decides when the game is won**, series by series, from the lead and the time left. His game management sets how accurate that read is: a poor game manager pulls starters too early and lets a team back in, or leaves his quarterback in too long.
2. **Starters come out in the real order**: the quarterback first (the injury risk to the most valuable man), then the skill players, then the lines; the defense the same way. Backups play with their own ratings, so the margin stops growing on its own. Nobody "eases up" by a flat factor.
3. **Backups get real snaps**, and the snaps count as game experience for them.
4. **The leading team plays to end the game**: more runs, the full play clock from the 3rd quarter in a big lead, no deep shots, no timeouts, kneel-downs late, punts or field goals instead of going for it on 4th down. Its defense plays softer and blitzes less.
5. **The trailing team**: in a close game it hurries and goes for it (already in). In a lost game its coach pulls his starters too, unless he is fighting for his job or a bowl spot.
6. **Each coach is different.** A new hidden coach trait, sportsmanship (1-20): a low one keeps his starters in and keeps throwing. The situation pushes it too: a playoff race where margins impress the committee, a rival, a grudge against the other coach (say, the boss who fired him). A Heisman candidate may stay in for his numbers. A senior walk-on gets snaps on senior day.
7. **Levels**: high school pulls starters earliest and keeps its running clock; college as above; NFL coaches keep starters in longer but pull the quarterback in a decided 4th quarter.
8. **Real checks**: from the real college play-by-play (cfbfastR, already used for our targets), the leader's 4th-quarter points and pass rate in games led by 28+ after three quarters. From the nflverse snap counts, NFL starters' snap shares in blowouts.

### What it should look like after
- Margins flatten past about 35. Games decided by 60+ become rare (near the real 0.4%).
- The best team's average margin lands in the real range (+22 to +39).
- Star stat lines shrink to real levels, because stars sit in the 4th quarter of blowouts.

### What I need from you
- Agree to the plan? And to the new hidden coach trait, sportsmanship?

---

## Topic 3. Playing at home is worth almost nothing

### What I saw
- Home teams win 50.7% (real 58.4%).
- Rating every team from all its results, home field is worth 0.7 points in the world and 2.5 in real FBS (the same fit run on the 2011-2025 real results).

### Why it happens (traced)
- The world plays every game through `quick_game` (`game/flow.rs:1976`), with one setting for everything: crowd 0.5, mild weather, sea level, no neutral sites. Bowls and the playoff are played as home games.
- The crowd only causes pre-snap fouls by the visiting offense (`game/flow.rs:1117-1147`): false starts and delays. That is the whole 0.7 points.
- There is no travel, no time zone, no familiarity, no weather by place and date, no altitude effect on stamina and no stadium size.

### How I'd fix it (proposed, not confirmed)
1. **Each game gets its real setting**: the home stadium and crowd (the world already has stadium size and fans), the city's weather for that date (temperature, wind, rain, snow, from real climate data by city and month), altitude, grass or turf, dome; neutral sites for bowls, the playoff and real neutral-site games. The engine already handles weather and altitude in kicks, catches and ball handling; the world just never sends it.
2. **Crowd noise** grows with stadium size, fan passion and the moment (a close 4th quarter, a night game, a rival). On the road the offense uses a silent count: the linemen get off the ball a beat later, the hard count stops working (tactics design, topic 3.9) and false starts rise. High awareness and poise in the quarterback and line lose less.
3. **Travel**: distance and time zones tire a team a little (a West Coast team at an early East Coast kickoff), more on a short week.
4. **Familiarity**: the home team knows its field, wind and weather. A warm-weather team in a November Big Ten game handles the cold worse.
5. **Altitude**: visitors tire faster at Air Force, Wyoming or Colorado (through the engine's existing fatigue).
6. **Officials**: a small home lean on close calls (Claude's reading of the research; to check).
7. **Levels**: smaller crowds in FCS, D2 and high school; the NFL home edge has shrunk to about 1.5 points in recent years (Claude's reading, to check with nflverse).
8. **Target**: about 2.5 points at FBS from the real results, with big loud stadiums worth more than small ones (to check against real home records by stadium size from the same data).

### What it should look like after
- Home teams win about 58%. A night game in a 100,000 stadium is a hard place to play. A cold, windy November game plays differently from September in Florida.

### What I need from you
- Agree to the plan?

---

## Topic 4. Coaches get fired too fast, and weak programs never recover

### What I saw
- 45 FBS head coaches were fired after two seasons or fewer in 15 years, about 3 a year among 138 jobs. Example: Kevin Dick, Toledo 2018-2020, 10-14, fired.
- Real: a handful in 15 years (Claude's reading: Turner Gill at Kansas 2011, Willie Taggart at Florida State and Chad Morris at Arkansas 2019, Biff Poggi at Charlotte 2024, DeShaun Foster at UCLA 2025).
- 12 FBS programs had no winning season in 15 years, five of them Power 4 (real: none). A weak roster loses, the coach is fired, and the next one inherits the same roster.

### Why it happens (traced, `world/staff.rs:350-425`)
- The board looks only at the last two seasons' wins against expectation, weighting the last 1.5 and the one before 0.7. Year 1 carries a tenth of the full firing chance, year 2 four tenths.
- Luck alone is enough. A season's wins swing about 1.5 either way by chance. A coach who falls 1.5 wins short of expectation two years running has about a one-in-three chance of being fired after year 2 at a middling program.
- From year 3, any record under 40% adds a penalty, whatever roster he was handed (`staff.rs:357`, "losing is losing").
- The expectation still comes from the old stand-in's win curve (`world/season.rs:583`), not from what the engine now does.
- Nothing costs anything: no contract, no buyout, no credit for recruiting or player growth, no athletic director.

### How I'd fix it (proposed, not confirmed)
1. **Contracts**: each head coach signs a real contract (years, salary, buyout) sized by the program's money and his reputation. Firing him costs the rest of the buyout. A program with little money can't afford to fire early; a rich one with big boosters can (Texas A&M paid Jimbo Fisher's buyout of about $76 million; Claude's reading, to check). The world already has each program's NIL money, boosters, stadium and fans.
2. **An athletic director judges the coach against the roster he got**: expected wins from the lineup and the schedule, using the engine's real win curve, read through the AD's own judgement (some ADs read rosters better than others). The AD is a new person in the world.
3. **Direction counts as much as wins in years 1-3**: are his recruiting classes better than the ones before him, are his young players developing, are players transferring out?
4. **A honeymoon**: in years 1-2 a coach is fired only for a collapse (far below the roster, heavy losses to rivals) or a scandal.
5. **Pressure**: fans, boosters and attendance push the AD; a blue blood runs out of patience sooner; rivalry results weigh more; the AD's own job is at stake too.
6. **Levels**: FCS and D2 boards are patient (little money, few candidates). NFL owners come with a later section.
7. Firing in mid-season with an interim coach comes with the season calendar (section 4).

### What it should look like after
- Almost no one fired after two seasons (one every few years across FBS). Most coaches get three to five years.
- A coach who beats a bad roster's expectation keeps his job. Weak programs still change coaches, but slower, and a good hire can turn one around. Together with topic 1, programs with no winning season in 15 years should be rare.

### What I need from you
- Agree to the plan, including contracts with buyouts and an athletic director as a new person?

---

## Topic 5. College offenses pass like the NFL and miss more

### What I saw (FBS)

| | World | Real |
|---|---|---|
| Pass rate | 59.5% | 49.6% |
| Plays per team game | 74.5 | 66.6 |
| Completion | 54.9% | 60.6% |
| Points per team game | 36.0 | 27.0 (2022-24 play-by-play) |

### Why it happens
- College coordinators call from the NFL's habits by down, distance, score and time (`game/calling.rs:118`, `real_pass_rate`), so college teams pass like NFL teams. Traced.
- The "what beats what" table the coordinators use (`call_values.csv`) was built from average NFL teams. A college coordinator picks concepts as if he had an NFL quarterback. Traced.
- A college passer's throws are about 20% wider than an average pro's even after his accuracy is softened below the pro range (`game/scrimmage.rs:2079` and `:2147`), and nobody calls around that. Traced.
- Routine catches fail 5.3% of the time for a receiver with 55 hands against 2.9% at 80 (`game/scrimmage.rs:2766`). A smaller piece. Traced.
- More incompletions stop the clock more often, which adds plays; more plays and passes add points. Inferred.

### How I'd fix it (proposed, not confirmed)
1. **College habits from real college play-by-play** (cfbfastR, which we already use): pass rate by down, distance, score, quarter and field position; seconds between snaps and hurry-up share. High school has no public play-by-play, so it uses the college habits under its own rules (shorter quarters, running clock).
2. **A "what beats what" table for each level**, built by playing average teams of that level. A college coordinator then learns that quick throws and screens are worth more with a college passer.
3. **Coordinators call to their own players** (the creativity skill in the tactics design, topic 6.2): a weak-armed but accurate quarterback gets the quick game, a big arm the deep shots, a running quarterback the option and run-pass options, a great back more carries.
4. **Re-measure completion.** Only if it is still short, fix the throw and catch physics, checked against real NFL drop and bad-throw rates (PFR) and the real college completion rate.

### What it should look like after
- FBS pass rate about 50%, about 67 plays, completion about 60%, points near 27-28 (with topics 1 and 2).

### What I need from you
- Agree to the plan?

---

## Topic 6. The draft board undervalues quarterbacks

### What I saw
- 7 of the 15 first picks were quarterbacks (47%); real 2000-2025: 73%. Picks 1-5: edge rushers 35% (real 21%), quarterbacks 31% (real 28%).
- Examples: in 2024 a 91-rated tackle went first and the best quarterback (74) went 14th. In 2026 an 84 edge went first and the best quarterback (77) went 5th. In 2015 the best quarterback (77) went 22nd.

### Why it happens (traced, `world/recruiting.rs:242-260` and `:476-500`)
- The board adds a flat bonus by position: a quarterback up to +11.3 points, an edge up to +7.1, a tackle up to +3.6. A quarterback gets all of his only at a board grade of 82 or more, and it shrinks fast below that (at 77 he gets +4.4). An edge gets all of his from 86. So a quarterback graded 80 sits about 8 points behind an edge graded 90 on the board.
- The bonus isn't tied to what a position is worth on the field or what it costs to sign.
- I haven't checked why the best quarterbacks graded below the top edges and tackles in these particular classes (the board grade blends what a player is now and what scouts project).
- A second problem: in the engine's NFL games a quarterback is barely worth more than an edge rusher (0.35 against 0.32 points per rating point, `docs/match-engine.md`). In real football the quarterback matters far more (Claude's reading). So the board can't just read the engine's values until topic 1 is fixed.

### How I'd fix it (proposed, not confirmed)
1. **Each team values a prospect by what he adds to that team**: wins over the player he would replace, over the years of his rookie contract, using the engine's value per rating point at his position (after topic 1).
2. **The money side**: a starting quarterback on a rookie deal saves a team a fortune against the veteran price; an edge rusher less; a back little. The real rookie scale is already in `data/real-world/rookie_scale_2026.csv`; veteran prices by position from OverTheCap until the salary cap section exists.
3. **Risk**: a quarterback's projection is less certain, so scouts disagree more about him.
4. **Team context**: the worst teams pick first and usually lack a quarterback; teams trade up for quarterbacks (already in).
5. **The GM's style**: some draft for need, some take the best player.

### What it should look like after
- A quarterback goes first about 70% of the time; an edge or a tackle when no quarterback is worth it. Every pick says why ("a quarterback on a rookie deal is worth this much more than the alternative").

### What I need from you
- Agree to the plan? It waits on topic 1.

---

## Topic 7. Kickoffs and returns are short

### What I saw (match engine, FBS, 200 games)
- Kickoff returns average 10.1 yards (real FBS about 21, Claude's reading). Touchbacks 4.6% (the real FBS rate still to be measured; the NFL's was 55%).
- Punt returns 6.5 (real NFL 9.5). The NFL is short too: kickoff returns 16.7 (real 26.5 under the old rule), punt returns 7.3.

### Why it happens
- College kicks land short. Kick distance is 44 + 0.23 × kick power yards (`game/kicking.rs:193`), and college kickers' kick power sits well below the pros', so most kicks land near the 10 and almost none are touchbacks. Formula traced; I haven't yet checked the world's kicker ratings.
- Returns are short at every level, the NFL included. Not traced. My guess: blockers on returns don't hold their blocks the way the scrimmage line now does, so the coverage gets through.
- A college rule is missing: a fair catch of a kickoff inside the 25 is a touchback at the 25 (NCAA, since 2018; Claude's reading).
- The best returners sit: a starting back, receiver or corner doesn't return kicks when two backups can (my pick in the physics build, never confirmed).

### How I'd fix it (proposed, not confirmed)
1. **Check first**: the world's kickers and kick physics by level against real kick distance, hang time and touchback rate (college play-by-play, nflverse for the NFL), and trace 20 returns to see where and by whom the returner is stopped.
2. **Return schemes**: a wall or middle return on kickoffs; gunners against jammers on punts (often double-teamed); fair-catch decisions from hang time and how close the coverage is.
3. **Rules per level**: the college fair catch on kickoffs; the NFL's newer kickoff (already in the engine); high school's own rules.
4. **The returner choice**: the coach weighs return value against injury risk and the backup's skill. A dangerous starter returns in big moments; in high school stars return more.

### What it should look like after
- FBS kickoff returns near 21, NFL returns and punt returns near real, touchback rates near real.

### What I need from you
- OK to trace first and come back with the exact fix?

---

## Topic 8. Sacks and interceptions are still a little low

### What I saw
- FBS sack rate 4.3% (real 6.1%), interception rate 2.3% (real 2.6%). Both moved toward real since trial 3 (2.9% and 1.6%).

### Why
- Not traced. Topics 1, 2 and 5 all move them: how often teams pass, who is on the field and how the passer compares with the rush.

### How I'd fix it
- Re-measure after topics 1, 2 and 5, and fix only what is still off.

### What I need from you
- OK to wait?

---

## Topic 9. Too few coaches of color

### What I saw
- 9 of 138 FBS head coaches (7%) in 2025; real about 15% (Claude's reading). Coordinators 13%, position coaches 14%.

### Why
- Not traced this round. The share is already low at the bottom rung (position coaches), so the main loss is probably in who enters coaching, not only at the head-coach hire. Inferred.

### How I'd fix it (proposed, not confirmed)
- Trace the ladder by ethnicity, from former players into coaching and up to head coach, and compare each rung with the real shares in the college sport racial and gender report card (TIDES). Then fix the rung that's off through the existing hiring bias and networks, never a quota.

### What I need from you
- OK to trace it?

---

## Order
Topics 1 to 3 first: they are one problem (dynasties and blowouts). I'd build them together, rerun the world trial, and then come back with topics 4 to 9. Still unbuilt from section 3: the career stats record and high school games on the engine.
