# How the match engine works

Section 3, built 2026-10-13/14. The design is `docs/design/match-engine.md`; this page says what the code does today, what is calibrated, and what is still off. Code: `crates/afm-engine/src/game/`.

## The pieces

| File | What it does |
|---|---|
| `sim.rs` | The tracked play. 22 agents, ten updates a second. Movement is physics (sprint curve, braking, cutting), players react to what they can see, blocks are running battles, tackles play out step by step. |
| `scrimmage.rs` | One snap: alignment, assignments, QB reads, routes, coverage, run fits, the throw and the catch. |
| `kicking.rs` | Kickoffs, punts, field goals, extra points. Ball flight from leg, wind, cold and altitude; returns are full tracked plays. |
| `playbook.rs` | Personnel, formations, routes, pass and run concepts, fronts, coverages, pressures. |
| `calling.rs` | Coordinators call plays from real situational rates (nflverse), bent by scheme, aggressiveness and in-game learning. Fourth downs and the clock are coaching skill. |
| `flow.rs` | The game: downs, clock, scoring, penalties, overtime, substitutions, fatigue, injuries, momentum. Every game replays from its seed. |
| `rules.rs` | Real rules per level (quarter length, kickoff rules, overtime). Only rules differ by level; the quality of play comes from the players. |
| `stats.rs` | The play log and every stat computed from it: classic, advanced (EPA, CPOE, pressure and win rates, separation, YAC), grades, and the high school "as reported" view. |
| `tables.rs` | Real NFL expected points, pass rates, fourth-down go rates and win probability (2021-25). |
| `roster.rs`, `synthetic.rs` | Game-day players from the player model, staff and schemes; synthetic teams for the checks. |

## How a pass play works (short version)

1. Both sides line up from the call. Each defender knows his job (zone spot, man, rush, run fit).
2. The QB drops (set time by drop depth) and reads his progressions. For each receiver he plays the route forward with that receiver's own speed, acceleration and cutting, then asks how much room the nearest defender will have when the ball gets there. He throws when that window beats his threshold (caution, fatigue and panic move it).
3. The throw has an error from accuracy, distance, pressure, throwing on the move, wind and his consistency.
4. At the catch point: tipped balls in the lane, contested catches, interceptions, drops, pass interference. Then the run after the catch.

## Kickers, punters and long snappers (fix list topic 1, 2026-10-15)

Code: `roster.rs` (`depth_chart`) and `flow.rs` (`fg_kicker`, `punter`, `kickoff_kicker`).
- **Two kicking jobs, two kickers.** The field goal and extra point kicker is the first K on the depth chart. The kickoff goes to the strongest leg (kick power) among the kickers, so a team with a big-legged backup uses him on kickoffs (1.8).
- **One leg for both jobs.** The kicking jobs go to the kickers and punters a team carries. A kicker kicks and a punter punts; the other one is only the backup leg at that job. A team with only a kicker has him punt, and a team with only a punter has him kick, rated at each job from the same leg (1.9). Only a team with neither asks the best leg on the roster, so position players no longer kick field goals (in the first check run, 38% of FBS field goal tries were by position players). The old flat fallbacks (a 20-point kicker, a 30-yard punt) are gone.
- **The long snapper** is a position (1.7). His rating at the job is his hidden snapping trait (75%, five points a step) plus blocking and covering (25%). He snaps on punts, field goals and extra points, and plays nowhere else. A team with no listed snapper uses its best snapper who is not the starting QB, K or P, as before.
- **The holder** is the punter, unless the punter is the one kicking or snapping; otherwise the backup QB.
- The game-day chart dresses two at K, P and LS. Synthetic teams above high school carry one long snapper (his own random stream, so the rest of the team is unchanged); high school teams snap with a player from another position.

## Calibration (development only)

The checks compare synthetic-league games to real 2021-25 play-by-play. They never feed back into the game. Run `cargo run --release --bin match_check -- nfl 400` (or `fbs`, `hs`). The automatic version is `crates/afm-tools/tests/match_engine.rs`, with wide bands.

Latest numbers (400 games each, 2026-10-14, after the physics fixes; the figures before them are in brackets):

| Per team game | NFL engine | NFL real | FBS engine | FBS real | HS engine |
|---|---|---|---|---|---|
| Points | 29.2 (27.6) | 22.5 | 29.0 (31.5) | 27.0 | 29.3 (32.0) |
| Completion % | 65.0 (66.2) | 64.5 | 53.7 (56.7) | 60.6 | 45.0 (47.2) |
| Yards per attempt | 8.2 (7.4) | 7.05 | 7.25 (7.3) | 7.3 | 5.9 (6.1) |
| Yards per carry | 3.75 (3.9) | 4.5 | 4.9 (5.1) | 5.1 | 5.6 (6.0) |
| Third-down conversion | 40.5% (39.7) | 39.8% | 37.9% (37.1) | 40.3% | 37.2% (33.3) |
| Sack rate | 4.3% (4.1) | 6.8% | 3.8% (2.3) | 6.1% | 4.7% (2.8) |
| Interception rate | 2.3% (1.5) | 2.3% | 2.0% (1.4) | 2.6% | 2.0% (1.3) |
| Time to throw | 2.57 s | 2.71 s | 2.74 s | | 2.96 s |
| Plays | 68 (70) | 62 | 74 (76) | 67 | 79 (83) |

There is no high school play-by-play dataset yet, so the HS column has no real figure next to it.

### Known gaps (trial candidates, not fixed on purpose)
- **Points and plays run high** at every level. NFL teams pass too often (66% against 58%), and college teams pass like the NFL (64% against 50%) because calling starts from NFL tables at every level.
- **Sacks and interceptions are low** at every level.
- **NFL runs are short** (3.9 against 4.5): too many runs for no gain, too few of 10-19 yards, a few too many of 40+.
- **Red zone**: 65% touchdowns against 56%. Slants and smash near the goal line score too often against man coverage.
- **Deep throws** are a bit rare at the NFL level (9% of throws travel 20+ yards against 11.5%).
- **Kickoffs**: NFL returns average 18 against 26.5. College and high school returns average about 10, because the synthetic kickers there are weak and their kicks land short. The real kickoff data predates the NFL's 2025 rule.
- **High school plays** (83 a team) are far above real high school games (12-minute quarters).
- **FBS completion** is about 7 points under the real figure (it was 4 before the physics fixes made the quarterback count; high school fell from 47% to 45% the same way). The lower levels' receivers and coverage are as far behind as their quarterbacks, so the fix is likely in the catch and coverage, not the throw.
- **Explosive runs are rare** at the NFL level (3.6% of carries gain 10+, real 11.6%): with the line deciding runs, a won block seals a defender for good but the back's breakaways through the second level got rarer.
- **Returns fell** with the physics fixes: NFL punt returns average 7.3 (9.5 real, 8.7 before), kickoffs 16.7 (18.2 before). Starters no longer return kicks, so the returners are backups (a Claude pick, proposed, not confirmed; returns are on Raichel's open list).
- **Passing yards run high** at the NFL level (8.2 a throw against 7.05, 11.6% explosive against 8.8%): coordinators who know what beats what pick good passes, and the coverage doesn't yet take everything back.

## Choices Claude made (proposed, not confirmed)

These are calibration picks inside the confirmed design. Raichel has not signed them off.
- **Sprint model**: speed builds as 1 - e^(-t/tau), tau 0.55-1.0 seconds from acceleration and weight; top speed comes from the 40 time, so 40 times and 10-yard splits match the combine.
- **Below the pro range, gaps narrow**: under 60, each point of acceleration or agility counts less, so a high school athlete still moves like an athlete. Above 60 every point counts fully.
- **Throw error** grows gently as accuracy falls (about 10% wider per 20 points).
- **The QB knows his receivers**: he leads a receiver by how that receiver really runs the route.
- **The ball carrier runs at 95% of top speed** while he reads blocks.
- **Blocks in space can whiff** (about 22% base, less for better blockers, more against fast defenders).
- **Tempo**: about 37.5 seconds between snaps (33 for Air Raid and spread teams).
- **Fourth-down success** falls with distance (70% on 4th and 1, about 12% at long distance), checked against nflverse.
- **Screens**: linemen release after 0.9 seconds; defenders can "smell" a screen with play recognition.
- **Kick coverage**: the widest man on each side keeps contain.
- **Goal-line coverage**: zones and alignment compress toward the end line.
- **Long snapper rating**: 75% the snapping trait at five points a step, 25% blocking and coverage. The holder rule (punter first, then QB2). Two dressed at K, P and LS (fix list topic 1).

## Physics fixes (2026-10-14)

Built from `docs/design/engine-physics-fixes.md` after world trial 3 showed that quarterbacks and the lines barely mattered, depth counted more than starters, a running back swung games alone and play callers changed nothing. Measured with `cargo run --release --bin dynasty_check -- value [games] [seed]` (a team plays itself with one position's starters raised 10 points everywhere; `AFM_LEVEL=nfl|hs`, `AFM_ONLY=QB,coach` narrows it) and `AFM_SNAPS=1 match_check`. Real anchors: `data/real-world/position_value.json`.

### 1. QB play drives passing
- **Throw error** now grows about 2% per accuracy point through the pro range (it was 0.5%). Below 68 the gap narrows to a quarter (the `pro()` knee), because the receivers and coverage a college or high school quarterback plays with are as far behind as he is. Proposed, not confirmed.
- **Decisions**: the noise on reading a window is 1.6 minus 0.017 per point of football mind (0.6 awareness, 0.4 vision), so a sharp quarterback finds the open man and a dull one throws into coverage. Panic under pressure scales with poise, clamped so a cool passer barely feels it.
- **Getting it off**: the throwing motion takes 0.22-0.5 s by release quality. A free rusher who will arrive before the ball is out stops the throw; he has to escape, eat it or get it away. Seeing a man open and letting it go takes a beat that shrinks with football mind (anticipation).
- **The internal clock**: even in a clean pocket a quarterback gets the ball out by a little over three seconds (3.3 s, sooner for a quick mind, later for a slow one, a beat more on a shot play). Past it he takes the checkdown in front of him, a beat later forces it to whoever is least covered, and throws it away only when nobody is there or he is outside the pocket. This came out of the build: with the clock missing, four verticals against single-high man was a free 25-yard throw at five seconds, because the receivers' scramble drill began on a timer and the man defenders overran them. The drill now needs a real breakdown (the quarterback out of the pocket, or 4.6 s). Time to throw now rises from the NFL (2.57 s) to college (2.74) to high school (2.96), as in real life. Proposed, not confirmed.
- **Deep balls need the arm**: a weak arm puts more air under a 25+ yard throw, so the error grows.
- **Coverage tightened** so the quarterback's edge is a real edge: deep zone cushions shrank (1.5 yards plus 0.4 per yard per second of the receiver's speed, was 2.5 plus 1.1), and an open receiver can come back or lay out for a loose ball (0.4 yard of extra reach per yard of separation, up to 4). A beaten man defender who sees the back blocking comes up for the run.

### 2. Line play decides runs and pressure
- Run blocks: the defender's edge counts 0.05 per rating point (was 0.032), the first contact is rated for the blocker (-0.3) and the drift noise is lower (0.6, was 0.75), so the better man wins more often and sooner. A won run block is sustained: the blocker stays on his man, who is only put on the ground now and then (0.25 a half-second), instead of being released every time. A defender fights off a block only when he has leverage on it (edge above 0.3).
- Pass blocks wear the rusher more than the blocker (1.0 against 0.5 work a second).

### 3. Depth and rotation
- Fatigue is real now: energy drains six times faster (0.041 a play per unit of work), running costs 0.35 of the old work, and taking hits wears the runner (30 work a second in a tackle fight). Starting defensive linemen come out below 0.84 energy, backs below 0.86. Snap shares sit in the real bands: defensive tackles about 73%, edges about 82%, the lead back about 65% of carries, linemen and corners stay in.
- Backups are worth less than starters because the depth chart lists a player's own position first and a cross-listed man gives up 4 rating points for the spot he doesn't know (`CROSS_LIST`).

### 4. The RB bug
- Tackling in space: a free defender in the open field reaches 2.2 yards (1.6 among bodies and on returns), the tackle logit uses (evade - tackle)/50 (was /30) and a dive counts double, so a back doesn't shed every open-field tackler. Acceleration counts 0.035 a point (was 0.045) and agility 0.011 (was 0.014) above 60. Strength and elusiveness count half as much in a tackle fight, and breaking a tackle uses (back - tackler)/60 (was /35).
- Starters at running back, receiver and corner no longer return kicks when the team has two backups to do it (proposed, not confirmed). This is where the running back's hidden value was: the lead back was also the returner.

### 5. Coaching through the battles
- No flat bonus. Coordinators call from a table of what really beats what: every pass and run concept against every look, played on the engine between two average NFL teams (`crates/afm-engine/data/call_values.csv`, rebuilt with `cargo run --release --bin call_values -- 400`). The offensive coordinator weighs each concept by its expected gain against the coverages he expects, exp(0.2 × sharpness × (value − mean)), where sharpness is (skill + adaptation)/40; a poor caller barely uses it and falls back on the league's habits. The defensive coordinator does the mirror: he leans away from the coverages the passes he has seen beat.
- Game prep: each staff studies the other's film before kickoff, as if it had seen 20 × skill/20 snaps of the opponent's coverage mix and blitz rate (offense) and pass lean (defense). In-game memory then adds what it sees.
- The call still wins or loses on the field: the table only says what the geometry has already decided. Play callers 18 against 4 with the same players: 62% wins, +4.4 points a game (was +0.0 before; Claude proposes a real band of 2-5 points, not confirmed).

### The measured position values
See the table below (points of margin per rating point per starter, FBS, 1000 games, same seed; real bands are Claude's reading of draft and contract value).

| Position | Per rating point per starter | Share of QB | Band (proposed) | Before the fixes |
|---|---|---|---|---|
| QB | 0.554 | 1.00 | 1.00 | about 0.02 (last) |
| OT | 0.072 | 0.13 | 0.15-0.30 | |
| IOL | 0.098 | 0.18 | 0.08-0.18 | |
| WR | 0.243 | 0.44 | 0.15-0.30 | |
| TE | 0.215 | 0.39 | 0.07-0.18 | |
| RB | 0.412 | 0.74 | 0.05-0.15 | about -0.75 to +0.68 a game, one back |
| EDGE | 0.332 | 0.60 | 0.15-0.30 | |
| DT | 0.227 | 0.41 | 0.10-0.25 | |
| LB | 0.230 | 0.42 | 0.05-0.15 | |
| CB | 0.205 | 0.37 | 0.12-0.25 | |
| S | 0.249 | 0.45 | 0.08-0.18 | |

What this says, and why:
- **The quarterback is first by a clear gap** (1.3 times the next man, 1.7 times the pass rush). Before the fixes he was last. Met.
- **The lines are not near the top.** Tackles sit just under their band, interior linemen inside it. Line play now decides the blocks (a won block is sustained, the better man wins sooner), but five linemen share one unit of outcome, so each one's ten points move the game less than one skill player's. Not met for the tackles; Raichel's call whether the band or the engine is wrong.
- **The running back and tight end are not near the bottom.** A back raised ten points everywhere is a faster, quicker, stronger athlete every carry and every catch, and the engine pays him for it even with the open-field tackle fixed and starters off returns (which took him from 0.58 to about 0.4). The real band comes from contracts and the draft, which price how easy a back is to replace, not what ten rating points do on the field. The two are not the same question; Claude's reading, proposed, not confirmed. Linebackers and safeties sit above their bands for the same reason: they tackle on every run.
- **Backups are worth less than starters**: a cross-listed man gives up 4 points at a spot he doesn't know, and the depth chart lists each position's own players first, so a strong second string no longer beats a strong first string (trial 3's rotation finding).
- At the NFL level (600 games) the order changes: QB 0.35, EDGE 0.32, RB 0.29, CB 0.24, LB 0.22, S 0.18, TE 0.16, OT 0.14, WR 0.12, DT 0.08, IOL 0.03. The quarterback's lead is thin there because NFL coverage and pass rush are closer to him, and the pass rush is worth almost as much. Proposed reading, not confirmed.

### Depth and box count (NFL, 200 games)
- Starters' snap shares, all inside the real bands: RB 0.66 (0.55-0.70), WR 0.78 (0.75-0.90), TE 0.79 (0.70-0.85), OT 0.99, IOL 0.97, EDGE 0.82 (0.65-0.85), DT 0.73 (0.55-0.75), LB 0.91 (0.80-0.98), CB 0.99, S 0.96. The lead back takes 0.68 of the carries, a touch over the band (0.55-0.65).
- Runs into a stacked box lose on the numbers, whoever called them: 3.9 yards a carry against 6 or fewer in the box, 3.8 against 7, 2.3 against 8 or more (real 4.73, 4.22, 3.40). The stacked box costs more than in real life (a third against a quarter), part of the short-run gap above.


## Garbage time (dynasty item 3, 2026-10-15)
Raichel's yes on trial 4 topic 2 ("go ahead an execute", 2026-10-15). Code: `game/garbage.rs`, with the hooks in `game/flow.rs` and `game/calling.rs`. Every number below is fitted to real play-by-play and is proposed, not confirmed. Real targets: `data/real-world/garbage_time.json` (FBS vs FBS 2022-24, NFL 2018-25; scripts in `data/real-world/scripts/`). Check: `cargo run --release --bin garbage_check -- fbs|nfl|hs [games]`.

How it works:
- **The read.** At each new drive and each new quarter (from the 3rd quarter; the 2nd in high school) the head coach reads the comeback chance: z = lead / (10 × √(time left / game length) + 3.25), fitted to the real NFL win-probability table. His game management sets how far off his read is in this game (sd 0.03 × (21 − gm)) and at each look (sd 0.015 × (21 − gm)).
- **Who comes out, in steps.** Step 1: the quarterback, lead back, three receivers, two linebackers, three corners and two safeties. Step 2: the rest of the first team. Step 3: the second string. Kickers, punters and the snapper stay. The lead back already sits a little before step 1 (real: his share of 4th-quarter carries falls by a third up 21-27 while the quarterback still plays). College steps sit at z 4.3, 4.8 and 5.7; the NFL at 4.4 and 5.4 and never pulls the second string; high school at 3.4, 3.9 and 4.7. A coach puts his starters back only when the game is clearly alive again (0.8 z below the step).
- **Who he is.** A new hidden coach trait, sportsmanship (1-20, mean 10, sd 3.5). Each point below 10 moves his steps 0.15 z later when ahead. A late-season contender (one loss or fewer) reads as 2 points lower; facing the program that fired him, 4 lower.
- **The leader plays to end it.** From z 1.2 to 2.4 (NFL 1.4 to 2.8) his "ease" grows from 0 to 1: the pass rate moves toward 22% (twice that on 3rd and long, half on short yardage), fewer deep shots, the full play clock, no hurry-up, no timeouts, no two-point tries or onside kicks, punts instead of going for it in his own half. With his backups in, a field-goal try becomes a 4th-down try 55% of the time (25% in the NFL), more for a good sport. His defense plays two-high, blitzes less and drops Cover 0.
- **The beaten team.** It stops trying to win before it pulls anyone: from z 3.4 (NFL 5.2, high school 3.0) it runs more, takes its time, stops calling timeouts, goes for it in field-goal range 70% of the time (three points change nothing) and stops the late must-score gambles. In 45% of games (NFL 60%, high school 35%) its coach plays it out with his starters whatever the score; otherwise he pulls them like a leader, give or take 0.8 z. Fighting for his job adds 0.8 z, one or two wins short of a bowl 0.5.
- **Snaps are game experience.** The world counts every player's offensive and defensive snaps from the engine. Experience in the engine is his career college snaps in units of a starter's season (750 snaps), not his seasons on the roster, so a backup's garbage-time snaps add up slowly. Player pages show snaps per season.
- **Why lines.** Each pull is logged in plain words, e.g. "Q4 2:19: T5 pulls the rest of his first team ahead by 27, as he reads it a comeback is about 1 in 2,800,000."

Results (FBS, 3000 games between 48 synthetic teams, engine | real):

| After 3 quarters | QB1 plays in the 4th, leader | QB1, trailer | Leader Q4 pass | Leader Q4 points | Trailer Q4 points | Trailer secs/play |
|---|---|---|---|---|---|---|
| 21-27 | 0.91 / 0.90 | 0.92 / 0.85 | 0.25 / 0.26 | 6.8 / 6.9 | 5.2 / 5.7 | 25.3 / 23.6 |
| 28-34 | 0.70 / 0.68 | 0.71 / 0.73 | | | | |
| 35-41 | 0.33 / 0.37 | 0.63 / 0.65 | | | | |
| 42+ | 0.07 / 0.13 | 0.41 / 0.52 | | | | |
| 28+ | 0.44 / 0.48 | 0.61 / 0.66 | 0.23 / 0.28 | 7.5 / 6.7 | 5.1 / 4.7 | 27.7 / 27.7 |

Where the build differs from the approved wording, because the real data says so (proposed, not confirmed):
- The leader's pass rate in a rout stays about 0.23-0.27, not "almost never throws": real FBS leaders up 28+ pass 28% in the 4th quarter.
- The skill players come out with the quarterback or before him (the lead back first), not after him: that is the real order.
- With his backups in, a leader in field-goal range usually goes for it instead of kicking (real FBS up 28+: 61% go).
- Deep shots fall by up to 30%, not to none; the big change is the run rate. Claude's reading, not measured against real air yards yet.
- The trailer's coach often keeps his starters in a lost game (real FBS: the quarterback plays the 4th quarter in 52% of games down 42+), but stops trying to win well before that.

Known gaps, outside item 3: close-game pass rate is high at both levels (FBS 0.50 against 0.39 for the leader), the lead back takes too many carries in every bucket (FBS close games 0.58 against 0.42), and long field-goal tries from midfield are about three times real (college calling, dynasty item 6, and kickers, fix list topic 1). NFL trailers down 21-27 score 8.3 points in the 4th quarter (real 5.7). The NFL and high school are not on the engine in the world yet. High school has no real target; it pulls earliest by design.

Open single questions for Raichel: (a) should snaps also drive development (today development uses the depth chart role); (b) the real four-game redshirt rule, so a redshirt's garbage-time snaps don't cost him his year; (c) rivals, senior day and award candidates wait for the calendar, walk-ons and awards.

## Not built yet
- Running every game in the world through the engine (the world still uses its own season results), and the season and career record across levels with percentiles and era adjustment.
- Coach buzz from stats, hidden-gem stat flags, awards and records. These come with the world trial 3 list and later sections.
- The 2D replay view (section 10).

## Debug tools
- `cargo run --release --bin match_check -- nfl|fbs|hs [games] [seed] [pbp]`: averages against real data; `pbp` prints the first game play by play with its whys.
- `cargo run --release --bin dynasty_check -- value [games] [seed]`: what each position and the play callers are worth (`AFM_LEVEL`, `AFM_ONLY`). `cargo run --release --bin call_values -- 400`: rebuilds the coaches' what-beats-what table (`AFM_CELL=concept:Coverage` shows one cell's outcomes).
- `cargo run --release --bin garbage_check -- fbs|nfl|hs [games] [quality sd] [seed]`: garbage time against real play-by-play (who plays the 4th quarter by lead, run-out, 4th downs, sample why lines).
- `cargo run --release --bin play_trace -- <run|pass|N> [seed] [sweep]`: one snap traced tick by tick, or a sweep of seeds. `AFM_LEVEL=fbs|hs`, `AFM_COV=0-7`, `AFM_LOS=yards` and `AFM_BASE=1` set the level, coverage, spot and personnel. `AFM_TRACE=1` prints every agent.
