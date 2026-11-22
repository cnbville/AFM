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
- **Wide runs are stuffed half as often as real** at the NFL level: 11.6% of the backs' wide runs go for no gain or a loss on the test rosters and 8.0% on world seed 1's clubs (real 23.2%), and few reach the sideline (1-4 yard wide runs 0.1% out of bounds, real 12.8%), so wide runs gain 5.6 to 6.4 yards a carry (real 4.72). The long runs themselves are at real since lane SF9b (2026-11-21: backs' 10+ 10.6% of carries, 20+ 2.26%, real 10.4% and 2.26%; 3.6% of carries gained 10+ when this list was written), in "The open-field tackle" below.
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

## Deep passing and scheme balance (dynasty items 1-2, 2026-10-15)
Raichel's yes on trial 4 topics 1 and 2. Every number below is proposed, not confirmed. Real targets: nflverse play-by-play and participation, 2023-24 (35,841 throws): air yards by route GO 25.2 (time to throw 2.69 s), POST 20.7, CORNER 19.4, IN/DIG 12.4, DEEP OUT 12.9, QUICK OUT 2.7; 20+ yard throws average 29.2 air yards, 35% complete. Route shares from `data/real-world/engine_targets.json`.

What was wrong: at the same time to throw, the engine's go balls were caught 12 yards deeper than real ones, 20+ yard throws were 19% of attempts (real 11.5%), some "throws" went 60-90 yards, and the quick out barely existed (1% of targets, real 17%).

What changed (`game/scrimmage.rs`, `game/playbook.rs`, `game/calling.rs`):
- **Defenders keep moving while they react.** When the QB judges a window, a defender already running with the route loses no ground during his reaction time. Deep safeties carrying a vertical now close the window they really close.
- **No throws past his arm.** A spot farther than 25 + 0.45 × throw power yards is never open to him.
- **The post breaks at 45 degrees** (was 25 yards deep for 12 across).
- **Routes are run under control**: 88% of top speed until the ball is thrown, then full speed to it.
- **A deep ball needs less room**: the QB's need for an open window drops 0.05 yards per yard past 12 (at most 1.2). An incompletion deep costs a down; a catch is worth a chunk. Real QBs throw 50-50 balls deep far more than underneath.
- **The receiver reads a long ball late.** On a throw longer than 0.6 s in the air he keeps running his route for 55% of the extra flight time (less with good hands and awareness), then adjusts. Before, he knew the landing spot at once and could run under any miss.
- **Speed outs**: a new quick-game concept (outside receivers out at four yards, the slot sitting over the ball).
- **Screens are called more**: about 9% of real NFL dropbacks are screens, whatever their average gain; the base weight for screen calls went from 0.9 to 2.25 (0.6 to 1.5 on long yardage).
- `call_values.csv` rebuilt.

Results (400 games each, engine | real NFL):

| | Before | After | Real |
|---|---|---|---|
| NFL points per team game | 30.3 | 26.9 | 22.5 |
| NFL yards per attempt | 8.09 | 7.68 | 7.05 |
| NFL 20+ yard share | 0.190 | 0.112 | 0.115 |
| NFL 20+ completion | 0.43 | 0.50 | 0.36 |
| Go route air yards | 41.9 | about 32 | 25.2 |
| Post air yards | 42.3 | about 31 | 20.7 |
| FBS points | 29.0 | 25.2 | about 28 (real FBS) |
| FBS yards per attempt | 7.09 | 6.34 | 7.29 (real FBS) |
| FBS completion | 0.542 | 0.569 | 0.606 (real FBS) |

Scheme balance (FBS, each scheme against every scheme on the other side, `AFM_MIX=1 dynasty_check schemes`, two seeds, margin against West Coast or Cover 2; noise about ±0.4; measured before the call-values rebuild):

| | Before (against the base only) | After |
|---|---|---|
| Air raid | +2.44 | -0.35 |
| Wide zone | +1.16 | +0.65 |
| Power gap | +0.37 | -0.2 |
| Spread option | +1.09 | -1.5 |
| Cover 3 | -0.59 | -0.4 |
| Quarters | -0.47 | -0.9 |
| Man pressure | +0.93 | +0.1 |

Position values (FBS, 2000 games, +10 on every attribute of a position's starters; lineup point = sum over all 22 starters):

| Position | Per rating point per starter | Share of QB |
|---|---|---|
| QB | 0.507 | 1.00 |
| OT | 0.073 | 0.14 |
| IOL | 0.049 | 0.10 |
| WR | 0.129 | 0.25 |
| TE | 0.268 | 0.53 |
| RB | 0.403 | 0.79 |
| EDGE | 0.214 | 0.42 |
| DT | 0.128 | 0.25 |
| LB | 0.223 | 0.44 |
| CB | 0.147 | 0.29 |
| S | 0.160 | 0.32 |

A lineup point is now worth 3.6 points of margin (was 4.8; target about 2.4). The QB is unchanged. Real per-player spreads (nflverse 2021-25) put a back at 0.27 of the QB and a receiver at 0.30, so the back is still about three times too valuable. Split by attribute (+10 on just these): speed and acceleration 1.5 points, strength and carrying 1.0, vision and awareness 1.0, agility and elusiveness 0.6, catching and route running 0.5, pass blocking 0.5.

Still open, measured after these changes:
- The back, tight end and linebacker sit well above their real spreads (see above).
- Wide zone and spread option: outside zone gains about 0.8 yards a carry more than inside zone and zone read (play_trace, FBS, three coverages), and spread option has no outside zone in its run mix. Cover 3 stops inside zone hard (2.6 yards).
- FBS short passes are completed too rarely (engine screens about 67-82%, real NFL screens 87%), so FBS scores below real now that deep balls no longer overpay. Real FBS completion 60.6%, engine 56.9%.
- Deep balls in the NFL still land about 7-10 yards deeper than real and are completed 50% of the time (real 36%).
- Flats and checkdowns are 32% of NFL targets (real flat and swing 9.6%, quick out 17%, screen 10%), with sacks (4.0%, real 6.8%) and throwaways too rare. Sacks are trial 4 topic 9, not approved yet.

## Build 7 engine work (2026-10-17)

- **Passing (dynasty RB value, passing side).** The ball leaves at the end of the throwing motion; the QB clock runs 0.3 s longer; roughing the passer, unnecessary roughness and face masks at a real 6 fouls a team game; beaten blockers drag the QB down; man coverage reacts a tenth sooner and misses more with depth; a catch past the end line is incomplete; kickoffs with wall and man blocking.
- **Coverage shells.** Press corners in Cover 0, 1 and 2-man; Cover 2 and Tampa 2 corners squat at 5 yards, others play off at 6.5. Flat defenders sit outside the numbers, so a hitch by the outside receiver is not undercut. Underneath zone defenders read the route late by their play recognition (0.1-0.6 s).
- **Home field (trial 4 topic 3).** Crowd noise from the stadium, night games, rivalries and stakes; the road offense works on a silent count, misses checks and false-starts at the real road ratio; travel, time zones, the body clock and altitude (`data/real-world/college_altitude.csv`) tire the visitors; warm-climate teams handle cold worse. FBS home margin 2.5 points (real 2.4); NFL 0.9 (real 2.1, finding). Minor bug 143 (2026-10-30): over 6,000 `match_check` games a level the edge had drifted to 0.8 NFL and 1.2 FBS points; a louder snap count (36), more missed calls at the line (0.06) and road receivers leaving late on the silent count (`SILENT_RELEASE`) give 2.2 and 2.5, mostly through the road passing game, as in real games (proposed, not confirmed). Minor bug 135 (the same night) gave every receiver a release off the line, which cut the NFL edge to 1.5; a road receiver now stands at the line on the silent count before his release, 0.38 s at full noise, for 2.3 NFL points (se 0.21) and 2.8 FBS (se 0.25) over 6,000 games with test staffs like the world's (below, "Throw depth and passing volume").
- **College calling (trial 4 topic 5).** Pass rate and fourth-down tables from real FBS play-by-play (cfbfastR 2023-24, `data/real-world/scripts/college_situations.py`).
- **S8-S10.** Each DC's own nickel habit; one-gap and two-gap fronts fit runs and the rush differently; no-huddle snaps stop the defense substituting, and breath comes back with time between snaps.
- **Position spots (fix list topic 3).** The engine plays each man at his listed spot: tackles by side, guards and center, nose and three-technique, middle and outside linebackers, nickel, free and strong safety, fullback (`flow.rs` `lineup`, `spots.rs`). A man at a spot he has never been listed at plays below his rating there; a man from another position group is an emergency (`PlayRecord.emergency`). High school players still go both ways.

Measured after steps 2-4 (`match_check`, 600 games): FBS 28.8 points (real FBS 27.0), completion 61.1% (real 60.6%), 7.0 yards an attempt (real 7.3), games within 8 points 38% (NFL real 54%); NFL 32.0 points (real 22.5), completion 69% (real 64.5%).

- **Plays and tactics (step 5).** Sets, motion, fronts with techniques, stunts, fire zones, robber and match coverages, RPOs, the option pitch, missed run fits; motion and disguise before the snap; knowledge per concept family and line chemistry; film, the matchup plan, tendencies and self-scouting; four calling skills; the play designer (`game/designer.rs`). The checklist is `docs/design/build7-checklist.md`, step 5.
- **The install (calendar T6.2).** `GameTeam::plan()`: the share of the book installed this week, from the coordinator's install (what his players absorb, `roster::absorb`) times the week (`roster::prep_week`: a Thursday game 0.65, a normal week 1.0, a bye 1.15). A smaller plan calls fewer concepts; calls past the install add busts (`Tactics.load`).
- **Calibration (step 5j, each proposed, not confirmed).** Kickoffs: the kicker kicks from between the hashes and trails, the safeties stay behind the wave (return touchdowns 0.11 → 0.02 a team game, real 0.012). Tackling in space: one open-field lunge of 2.5 yards (`sim::OPEN_REACH`; 2.0 on scrimmage plays since minor bug 162, `SCRIMMAGE_REACH`, below). Drops: quick 1.25 s, reads a tenth after the top of the drop (throws under 1.6 s 21% → 6%, real 10%). Quick game is one picture: the QB keys the flat defender and throws the open one of the first two routes (stick against Cover 3: 48% → 87% complete). Man coverage lags 0.12 s at the break.

Measured after step 5 (`match_check`, 400 games each):

| Per team game | NFL engine | NFL real | FBS engine | FBS real | HS engine |
|---|---|---|---|---|---|
| Points | 30.0 | 22.5 | 24.3 | 27.0 | 21.6 |
| Completion % | 65.8 | 64.5 | 57.1 | 60.6 | 49.2 |
| Yards per attempt | 7.7 | 7.05 | 6.5 | 7.3 | 4.5 |
| Yards per carry | 4.4 | 4.5 | 5.0 | 5.1 | 5.7 |
| Third-down conversion | 43.2% | 39.8% | 38.6% | 40.3% | 35.4% |
| Sack rate | 6.1% | 6.8% | 7.8% | 6.1% | 16.9% |
| Time to throw | 2.56 s | 2.71 s | 2.75 s | | 2.99 s |

Trial 7 findings from these: the engine scores more in the NFL than in FBS, the reverse of real football (the offense gains more from better players than the defense does); NFL red zone completion 63% (real 53%); yards after the catch 3.8 (real 5.2); targets to tight ends 13% and backs 12% (real 22% and 18%); flats 17% of targets (real 9%); runs of 10-19 yards too rare; Cover 4 and Cover 6 leave the flat too open (stick gains 15 yards a play against them on the engine); high school sacks 17% of dropbacks.

## The 67 attributes and the retune (2026-10-18)

Raichel agreed to 67 visible and 5 hidden attributes and a retune for trial 7 findings 1 (NFL scoring) and 2 (college stat lines). What each attribute drives is in `docs/design/attributes.md`. The retune, each proposed, not confirmed:
- **Synthetic NFL rosters look like the world's.** Each NFL position is shifted by where the world's starters sat in trial 7 (offensive line and corners high, linebackers low; `synthetic.rs`, `nfl_offset`). Inferred, not measured on a world with the new list.
- **Placement.** Each accuracy point is worth 0.5% less miss (was 1.7%). Footwork, release and anticipation are read at half their first size: NFL quarterbacks rate far above 60 in all three and threw in 2.41 s.
- **Back committees.** The more worn a starter is, the bigger the drop a coach accepts to spell him (`flow.rs`, `pick`). It used to be a flat 9 points, so a star back took nine of ten carries. The lead back now takes about 65% of the backs' carries in an NFL game (real 70%, and 59% over a season with the games he misses) and 53% in FBS; a star 15 points better about 82% (real stars about 80-85%).
- **Holding** reads the blocker against the man in front of him. Read on its own, it ran 0.7 a team game in the NFL and 2.6 in high school (real about 1.3 at each).
- **Fumbles** read the hitter's strength and tackling against the carrier's ball security. Read on their own, NFL carriers almost never fumbled (0.27 lost a team game).
- **Trait norms.** The NFL norm is 67 (was 64), measured on the shifted rosters.

Measured with `match_check`, 400 games each:

| Per team game | NFL engine | NFL real | FBS engine | FBS real | HS engine |
|---|---|---|---|---|---|
| Points | 26.5 | 22.5 | 23.9 | 27.0 | 23.0 |
| Completion % | 63.8 | 64.5 | 59.4 | 60.6 | 51.5 |
| Yards per attempt | 7.0 | 7.05 | 6.6 | 7.3 | 4.9 |
| Yards per carry | 4.1 | 4.5 | 4.9 | 5.1 | 5.8 |
| Third-down conversion | 41.1% | 39.8% | 37.9% | 40.3% | 36.9% |
| Sack rate | 4.9% | 6.8% | 8.1% | 6.1% | 15.6% |
| Time to throw | 2.58 s | 2.71 s | 2.77 s | | 3.01 s |
| Penalties | 5.0 | 6.0 | 6.1 | | 6.7 |
| Fumbles lost | 0.44 | 0.45 | 0.43 | | 0.37 |
| Lead back's share of the backs' carries | 65% | 70% | 53% | | 55% |

On main the same check gave NFL 30.0 points, 65.8% and 7.7 a throw on balanced rosters. With the NFL rosters left balanced the engine now gives about 25.7.

**Not reached: NFL points 20-25.** Passing per throw is at real, but the engine still scores 4 points over real. The yards come in the wrong shape: throws of 10-19 yards complete 59% (real 54.5%) and 20+ 41% (real 36%), yards after the catch are 3.5 (real 5.2), explosive runs are 4% of carries (real 11.6%), and red zone passes complete 62% (real 53%). The offense moves the chains steadily and rarely needs a big play. Tested and not kept: a miss that grows faster with distance put NFL scoring at 25.0, but completion fell to 62%, 6.5 a throw, and FBS to 6.2 a throw (real 7.3). Fixing the shape (run after the catch, breakaway runs) is a bigger job than this build. The NFL test band is 20-28 for now, not the agreed 20-25. The next trial measures the world's own NFL, which ran 40 points a team game in 2021-25 in trial 7 (9.2 a throw): most of finding 1 sits in the world's rosters, not in these synthetic ones.

**Finding 2 left for the next trial.** The college stat leaders against the real FBS leaders need a world, so the trial measures them. On the synthetic checks the top man's share of a team's sacks is 0.33 (real about 0.25-0.35). FBS sacks still run high, 3.1 a team game (real 2.0), as on main.

## Player tendencies (2026-10-19)

The design is `docs/design/tendencies.md`. Every player carries 15 tendencies, 1 to 20 (how often he tries something, not how well). The engine reads them through `Sim::tend`: his number, pulled back toward his old habit (`was`) under stress (late and close, playoffs, tired, less with composure), shifted by up to 2.5 on the gamble ones when his team must gamble or protect a lead, and replaced by an instruction's number on the snaps he obeys it. Obedience is decided once a play from a key, not a draw, so asking changes nothing else. `lean` is the number as -1 to +1.

Every effect is linear in the lean, so a league whose players sit at the middle plays as before, and a league at the real spread averages out (an exponential multiplier raised the averages; each was replaced). Where each one acts:

- **Bounces runs:** behind the line on a designed run, how hard he holds the called hole and how much he likes a lateral heading.
- **Contact willingness:** at the sideline with a tackler closing, steps out or turns up; quarterback slides; leg drive in the pile; his share of injuries in a hit; a shy defender sometimes passes on a bigger man.
- **Reaches the ball out:** short of the line to gain or the goal line with a tackler on him, he stretches for it (`Sim::reach`): it gets there if the line is within his arm; 3.5% (more against a strong tackler, less with ball security) it comes loose.
- **Patient or downhill:** the press pace to the line (up to a quarter slower or faster), a downhill back committing a yard behind it, a patient one reading the crease cleaner and stopping his wait when a free defender is within 3 yards.
- **Scrambles:** how readily he leaves the pocket (1 + 1.2 lean on the scramble chance).
- **Gunslinger:** how much room he needs before he throws, more downfield, the deeper man preferred when two are open, forcing it sooner; a checkdown man dumps it off sooner.
- **Holds for the big play:** his clock (up to a second longer), how long he stays on each read, and early on how much more than an open man underneath he wants.
- **Returner risk:** kickoffs in the end zone (his guess of where he will get to against the touchback, a logistic call sharpened by judgment), punts inside the 10, fair catches with coverage close.
- **Rushes upfield or keeps contain:** an edge's rush power (up to 10 rating points), whether he has run past the quarterback's depth so a step-up gets out (`free_near`), how far a contain man shuts his side, how deep he squeezes a run.
- **Times the snap:** his get-off (12 rating points at the ends) and his offside chance, a hard count getting him most.
- **Gets his hands up:** batted balls, at a little rush.
- **Goes for the strip:** fumbles forced in a hit (1 + 1.15 lean), at some missed tackles.
- **Big hit or sure tackle:** missed tackles (scaled straight on the miss chance), force in the hit, roughness flags, injuries both ways.
- **Flies downhill:** a linebacker's or safety's read time and how hard he bites on a play fake.
- **Jumps routes:** when the quarterback commits to a throw near him (`jump_route`), he breaks on it: guessed right, he plays the ball at once with more reach and a real interception chance; wrong, he bit on the fake and finds the ball late. The coach's strictness and trust still decide whether he may.

**Instructions** (`tendencies::Instruction`, ten of them): kneel deeper than N yards, fair catch inside the 10, slide, take shots, protect the ball, high and tight, keep contain, no snap guessing, wrap up, stay home. Each sets its tendency on the snaps he obeys: 0.86 for a typical player, about 0.7 at discipline 3 of 20, more under a strict coach and with trust. AI staffs give them before every game (`GameTeam::give_orders`).

**Plans against habits.** Each staff reads the other side's habits from film (`calling::read_habit`): a hard count against snap guessers, double moves against route jumpers, kicking deep against a returner who brings everything out, more draws against upfield rushers.

### What each one does (measured 2026-10-19)

`cargo run --release --bin tendency_check -- nfl 272 1 all`: 816 NFL games for the spread, then a 17-game season for 32 teams (272 games) per arm, every owner at 4 and then 16 on the same seed. 151 s on 4 threads, about 4 s an arm. The real spreads are NFL regulars 2016-2025 (`data/real-world/tendency_spreads.json`).

| Tendency | Tied stat | At 4 | At 16 | Real p5-p95 | Cost at 16 |
|---|---|---|---|---|---|
| Bounces runs | carries outside the tackles | 28.6% | 62.3% | 8.8-34.5% | stuffed 19.9 to 21.7% |
| Contact willingness | QB scrambles ending in a slide | 62% | 17% | none | injuries 0.31 to 0.52 a team game |
| Reaches the ball out | reaches a team game | 0.00 | 2.17 (42% get there) | none | 0.08 fumbles a team game |
| Patient or downhill | RB time to the line | 1.83 s | 2.15 s | 2.54-3.00 s | stuffed 16 to 26%, 5.50 to 5.68 a carry |
| Scrambles | scramble rate | 2.3% | 9.9% | 0.6-9.0% | scramble yards 5 to 27 a game |
| Gunslinger | tight-window throws | 13.1% | 16.4% | 11.4-21.2% | air yards 9.6 to 11.6; INT 2.2 to 3.1% |
| Holds for the big play | time to throw | 2.41 s | 2.74 s | 2.49-3.04 s | sacks 4.2 to 5.9%; 20+ completions 8.8 to 10.5% |
| Returner risk (FBS rules) | end-zone kicks brought out | 3.5% | 42% | 6.5-36% | start 25.0 to 23.3 |
| Returner risk | punt fair-catch rate | 56% | 18% | 23-56% | fielded inside the 10, 3 to 12% |
| Rushes upfield | scramble yards allowed | 18.7 | 20.1 | none | rush wins 0.15 to 0.32 a dropback, sacks flat |
| Times the snap | DL offside per 17 games | 0.20 | 3.61 | 0-4.25 | pressure flat |
| Gets his hands up | batted passes per DL per 17 | 0.33 | 2.02 | 0-4 | sacks 5.3 to 4.8% |
| Goes for the strip | forced fumbles per 100 tackles | 0.44 | 2.02 | 0-4.65 | missed tackles 8.7 to 11.5% |
| Big hit or sure tackle | missed tackle rate | 6.1% | 13.7% | 2.3-16.2% | roughness 0.31 to 0.62, injuries 0.32 to 0.51 |
| Flies downhill | LB and S tackles for loss per 17 | 5.2 | 6.4 | 0-15 | play action 9.6 to 10.4 a throw |
| Jumps routes | CB and S interceptions a target | 4.4% | 5.0% | 0-7.3% | 2.7 jumps a team game, 14% picked when right |

Fit: backs who bounce at 16 against 4 gain 1.10 yards a carry when fast (5.46 to 6.56) and 0.78 when slow (4.73 to 5.51), with speed, burst and elusiveness 12 points apart.

Instructions (the habit at 17, or 4 for "take shots"; free, ordered, ordered at discipline 3): slide 20/63/55% of scrambles; protect the ball INT 3.6/2.5/2.7%; take shots air yards 9.2/11.6/11.2; high and tight reaches 2.36/0.48/0.75 a game; keep contain rush wins 0.33/0.15/0.19; no snap guessing offside 4.16/0.68/2.11 per 17; wrap up missed tackles 15.8/4.5/7.1%; stay home jumps 2.25/0.17/0.72 a game; fair catch inside the 10: fielded 15/2/4%; kneel deeper than 3 (FBS rules) brought out 46/36/39%.

Levels (`match_check` 1600 games, seed 7, against main): NFL 25.6 to 25.5 points a team game, 4.22 to 4.34 a carry; FBS 23.9 to 24.0; high school 23.3 to 23.9. Sacks, interceptions, completions and scrambles stay within noise. NFL punt returns fell from 8.0 to 6.9 yards (risk-takers field more punts with coverage close).

**Not reached.** Two tied stats sit outside the real spread because of the engine, not the tendency: backs run outside about twice as often as real at every setting, and almost never end out of bounds (0.7%, real 4.2%), so contact willingness cannot show at the sideline. Time to the line runs 0.7 s under the real Next Gen number at every setting (the spread, 0.3 s, is two-thirds of real). Route jumping moves interceptions 14%, not the threefold of the real best (much of that is luck), and its cost (big plays allowed) does not show. Rushing upfield doesn't raise sacks: a won pass rush barely moves pressure in this engine (rush power plus or minus 58 points moved pressure under a point), because the beaten blocker squares the rusher up again a tick later. (The trial 8 fixes below deal with the time to the line, the sideline, route jumping and the pass rush.)

## Trial 8 fixes (2026-10-19)

Raichel agreed to every finding of world trial 8 and the five engine bugs the tendencies build reported (`docs/trials/world-trial8.md`). No trial measured these; the numbers below are `match_check` and `tendency_check`. The world side is in `docs/world-model.md`, "Trial 8 fixes".

- **A won pass rush keeps its edge (bug 1).** A beaten pass blocker can't square the same rusher up again until he has his feet back (0.35-1.3 s, by agility and pass blocking) and is between him and the quarterback again. A quarterback with a free man due in under about 0.85 s (less for a poised one, `scrimmage::quarterback`) leaves his reads and throws the best window he has; a throw already in motion comes out. Upfield rushers at 16 against 4: rush wins 0.19 to 0.33 a dropback, sacks 5.2 to 5.8% (before: flat). Pressure still moves under a point. On the world's FBS rosters a dropback ends in a sack 6.4% of the time (real 6.0%).
- **Scrambles to real (bug 4).** With the quarterback getting rid of it under a free rusher, NFL quarterbacks scramble on 4.9% of dropbacks (real 4.9%; was 7.8%).
- **Backs reach the line at the real time (bug 3).** Handoffs are 0.4 s later (zone 1.0 s from the gun, power 1.2, counter 1.4) and the press pace is about two-thirds of what it was (`RunConcept::press`), so a back reaches the line at 2.65 s (Next Gen 2.76), patient backs 2.90 and downhill 2.40 (real p5-p95 2.54-3.00). QB power keeps its old pace: the quarterback has the ball from the snap.
- **The hole read (bug 2, item 13).** A back pressing a designed run reads the called hole about 2.5 yards from the line and picks his lane: the hole if it is there, the cutback or the bounce otherwise (a bouncer leans off the called hole, vision sharpens the read). He then stays in that lane to the line instead of drifting to wherever the bodies are fewest. With no gain left at the sideline he steps out (more if he avoids contact); taken down there, he lands out of bounds when the fall crosses the line. A back who bounces at 16 now takes 37% of carries outside (62% before), but the middle of the league still runs 38% outside (real 19%). Out of bounds 2.2% of carries (real 4.2%; was 0.7%). Both still short.
- **Falling forward (item 7).** A tackled back's spot was his hips. Now the pile falls the way its weight goes (a quarter second of it, at most 1.5 yards) and the ball lands ahead of his hips by the fall plus the reach of his body, up to most of a yard more. Driven back, the spot is where he got. Yards after contact went from 1.2 to about 2.0 a carry (real 2.0, PFR 2021-25).
- **FBS run blocking (item 7).** A run block starts with the defender a little ahead (`w0` 0.15; it started with the blocker well ahead, -0.3): the get-off against the read decides the first leverage. Climbing blockers take an angle to a linebacker on the move; a linebacker fills to the line and leaves it for the runner only when he is nearly on him (1.8 yards, more if he flies downhill; he went from 4 and made 9.6 tackles for loss a season, real 5.3); a lunge for a block misses a little less often (0.17). On the world's FBS rosters (2045 of trial 8, `AFM_ROSTERS`, tendencies centred as fix 3 does), counting plays as below: 5.08 yards a carry (real 5.12), 24.5 points a team game (real 27.0 over 2022-24, when 2022 still stopped the clock on every first down), 63.0 plays (66.6), 7.6 yards an attempt (7.3). The block start was first set at 0.05 for 5.14 a carry, before plays wiped out by a penalty left the count: those carries had counted for no yards, and without them 0.05 gave 5.52.
- **What is still wrong with runs (item 13, not fixed).** The shape. A back is first hit either behind the line or 3 or more yards past it: on the world's FBS rosters 31% of runs gain nothing (real 18%), 19% gain 1-3 yards (real 37%), 37% gain 4-9 (real 34%). Linebackers make 18% of first tackles (real 37%), defensive backs 42% (real 30%). Why (traced): blocks are one on one, with no double teams that hold a tackle and then climb to a linebacker, so a lineman either sheds early (a stuff) or the back is through to the safeties. Moving the block start trades one for the other (at the old start: 6.0 a carry, 19% stuffs). Synthetic NFL teams now run 3.39 a carry with 43% stuffs (real 4.50 and 18%); the world's NFL ran 0.7 a carry above its synthetic check in trial 8, so about 4.1 is expected there. Needs a run-fit build: combination blocks and linebacker fits (Raichel agreed 2026-10-19; it is in the fix list for the section 6 build, `docs/design/fix-list.md`).
- **Route jumpers give up big plays (bug 5).** A jumper who guesses wrong drives for the break he guessed; his man turns it upfield past him (the sight adjustment receivers are taught), the throw leads the man deep, and the jumper needs 0.5-0.9 s (by anticipation) to plant and turn before he can chase. Every coverage player at 16 against 4: completions of 20+ yards 10.4 to 11.2% of attempts (before: 10.4 to 10.7), yards a throw on a wrong guess 9.7 (8.6), interceptions 4.1 to 5.1% of targets.
- **The clock between plays (item 6, pace).** The clock waited for the snap after every stoppage, so a play that stopped it was followed by the next snap 5.6 s of game time later (real 15.2, nflverse 2021-25) and NFL teams ran 70.0 plays a game (real 61.9). Now it starts again on the referee's ready signal where the rules say so (NFL rule 4-3, NCAA and high school rule 3-3): after an accepted penalty on a play that would have left it running, once the penalty is marked off, with the 25-second play clock (15 of the usual 40 gone); after an NFL runner or receiver goes out of bounds, once the ball is spotted (7 seconds, proposed, not confirmed); after the chains are set on a first down (high school always, college in the last two minutes); after a pre-snap foul with the clock running. Late in a half (the last two minutes of each, and at the NFL the last five of the game) it waits for the snap. Running plays already used about real time (38.2 s snap to snap, real 36.9). Still short: 9.2 s after a stopped play (real 15.2); few NFL plays end out of bounds in normal time (1.4% of plays; in the fix list for the section 6 build). `match_check` shows each snap-to-snap time against nflverse's.
- **Plays wiped out by a penalty (item 6, count).** A play an accepted penalty wipes out (holding, a defensive foul taken instead of the play, offsetting fouls) counted as a play, a pass attempt, a completion for no yards, a carry for no yards, a sack or an interception. Real box scores and nflverse count it as a no-play, with only the penalty. It now counts only its snaps, the penalty and its time (`PlayRecord::nullified`). A personal foul after a gain, marked off from where the play ended, and intentional grounding (an incomplete pass) leave the play standing, with its yards. This took about 3.5 plays a team game off the NFL count and put third downs at real (12.8 a team game, real 12.7). It also showed that NFL test teams throw for 7.9 yards an attempt (real 7.05): the wiped-out attempts had counted for no yards.
- **NFL test teams** (`synthetic::nfl_offset`) now sit where trial 8's NFL starters sat in its final year (QB +6.9 over FBS, OT +5.3, EDGE +3.8 and so on), so the NFL check plays rosters like the world's: 26.2 points a team game (real 22.5), 64.0 plays (61.9), 7.87 yards an attempt (7.05), 2.13 touchdown passes (1.46), 1.03 interceptions (0.76). The rest of the world's extra points is passing on world-like rosters; the world's NFL ran 31.2 points in trial 8 and the next trial measures it with these fixes (item 6).

Checks (`match_check`, 600 games): NFL 26.2 points, 64.0 plays, 3.39 a carry, 5.6% sacks, 64.0% completions; FBS 21.2 points, 62.8 plays, 3.79 a carry; high school 14.0 points, 51.3 plays, 4.48 a carry (high school stopped the clock until the snap after every first down; there is no real high school target in the data). The test bands in `tests/match_engine.rs` widened with these numbers (NFL points to 30, NFL carries from 3.0, FBS points from 18, FBS third downs from 0.29, FBS carries from 3.3: test backs run a 4.78 forty against the world's 4.57, and the world's FBS runs 1.3 a carry above them).

New in `match_check`: `AFM_ROSTERS=file.json` plays the exported rosters of a division instead of synthetic teams (`tools/rosters/export_rosters.py` writes the file from a trial's raw export); `AFM_DUMP=file.json` writes the synthetic teams in the same form; rows for dropbacks with a rush win and for who makes the first tackle on runs (line, linebackers, backs) and the gain when they do (real from nflverse 2021-25).

## Fix list S6.1 and S6.2 (build 9, 2026-10-19)

Raichel put both in the fix list for the section 6 build (2026-10-19 17:13). Checked with `match_check` only, no trial; trial 9 measures the world.

- **Run fits (S6.1).** Run blocks were all one on one, so a lineman either shed early (a stuff) or the back was through to the safeties. Now zone, draw and duo plays call two combination blocks and gap plays one (`scrimmage::run_blocking`): an uncovered lineman with a linebacker to climb to helps a neighbour on a down lineman, play side first. The helper adds his power to the block (`Engagement::helper`), and climbs (`Agent::climb`) once the down man is controlled (leverage past -0.15 for 0.35 s), after a second whatever happens, or at once when the linebacker comes downhill. With the doubles holding, a run block starts even (`w0` 0.0; it was 0.15). On the world's FBS rosters (trial 8, 2045): runs gaining nothing 31% to 27.7% (real 18.2%), 1-3 yards 19% to 22.3% (37.1%), 4-9 yards 37% to 37.6% (34.2%), 10-19 yards 10.0% (8.2%); linebackers make 29.9% of first tackles (was 18%, real 37%), defensive backs 41.2% (was 42%, real 30%); 5.02 yards a carry (real 5.12). Linebacker gap fits (each linebacker owning a gap and filling it) were tried in two forms and dropped: both lowered linebackers' tackles and raised the carry. The shape is still short: too many stuffs and too few 1-3 yard runs. Synthetic NFL teams run 3.48 a carry (real 4.50).
- **Plays ending out of bounds (S6.2).** NFL plays in normal time (outside the last two minutes of a half and the last five of the game) ended out of bounds 1.6% of the time (nflverse 2023: 10.7%; completions 4.2% against 20.7%, designed runs 0.1% against 5.5%). Two changes, both proposed, not confirmed: a carrier within 10 yards of the sideline (was 6) reads the angle sooner and takes the sideline rather than a hit for nothing (step-out weight 10, was 6; threat 0.3, was 0.5); and a man taken down within 6 yards of the sideline is driven out by the tackle, surely inside a yard and a half, more when the pile is going that way (`Sim::tackled`). NFL (400 games): plays out of bounds 5.3%, completions 13.9%, designed runs 0.5%; the clock after a stopped play 11.6 s snap to snap (was 8.7, real 15.2); 3.48 a carry (3.52 before, the same within noise); 26.6 points (was 27.5). Wider step-out settings reached 11.9% of completions but cost a quarter of a yard a carry. What is left, traced: only 3.5% of tackles happen within 3 yards of the sideline and 61% within 9 yards of the middle. Short outs are 5.0% of targets (real 17.4%), and designed runs almost never reach the edge. Both are route and run-path geometry, not the sideline rule; they are on the list for Raichel. `match_check` now shows the three out-of-bounds rows against nflverse at the NFL.

## Trial 9 findings (build 10, 2026-10-20)

- **The run game (finding 2).** S6.1 left runs a little too hard: in trial 9, 4.85 yards a carry in FBS (real 5.12) and 3.99 in the NFL (real 4.50). Two causes, two changes (`match_check` on trial 9's 2045 rosters, 400 games each):
  - Defenders read run or pass on the raw play recognition scale, while the offense's timing (the blocks, the back's path) is the same at every level, so an average NFL defender knew a run 0.14 s sooner than an average FBS one and NFL runs were stuffed 33-36% of the time (real 18%). Recognition is now read against the level, anchored at FBS (`scrimmage.rs`, `level_read`; high school defenders read later). NFL 3.84 to 4.13 a carry; FBS unchanged.
  - A run block starts with the blocker a little ahead (`w0` -0.1, was 0.0). NFL 4.13 to 4.32, FBS 5.09 to 5.24.
  - The world ran 0.24 below these checks in FBS and 0.15 above in the NFL in trial 9 (schedules, depth, weather), so the world should land near 5.05 and 4.45. Synthetic rosters: NFL 3.88, FBS 4.12, high school 4.87. The shape is still off: runs gaining nothing 31% in the NFL check (real 18.2%), 1-3 yards 23% (37.1%); a front-seven tackler stops the back 1.5-1.7 yards past the line (real 2.4-3.4).
- **Injuries (finding 13).** A hit injured at 0.0042 times its force term, which gave 0.20 injuries costing a week or more a team game in the NFL and 0.17 in FBS, against about 1.0 and 0.9 real (game-status reports 2012-2025; NCAA surveillance, `docs/research/development-and-injuries.md`). Now 0.022 (`sim.rs`, `injury_check`): NFL 1.05 a team game, FBS 0.77. Scoring barely moves (NFL synthetic 26.4, FBS 21.2). Practice injuries are on the world side (`docs/world-model.md`).

## Live games, officials and the toss (final build lane 2, 2026-10-22)

Game day (section 9, `docs/design/game-day.md`) needs a game that stops between snaps for your call. The screens' side is in `docs/game.md`, "Game day".

- **A game that pauses (`game/live.rs`, `LiveGame`).** `play_game_ctl` is the same loop as `play_game` with a `Control`: your team, your settings (which snaps stop, the clock), your answers keyed by the number of the point they answered, and your week's work (`Prep`). At a point that stops for you the game records what the staff would do (the call and why, the next choices from the same box, the analytics numbers, the replay assistant's word) and pauses. Answering replays the game from kickoff with every answer so far, about 35 ms for a whole NFL game, so a pause holds no half-played state and a save keeps only the seed and the answers. Points before the latest answer never stop again, so changing the settings mid-game doesn't move an answered snap.
- **Identical when the staff calls it.** With every answer "the staff's", a paused game gives the same result as the one-loop game, play for play (`tests/live_game.rs`; the career test `afm-game/tests/gameday.rs` checks the day keeps the game the morning built). Your options (the next choices, the plays for a go on 4th down) are drawn on their own streams, so offering them changes nothing.
- **Your calls.** A call you pick replaces the staff's; a play off the week's sheet is less practiced (bust chance as for a play with 0.35 less install). Your 4th-down chart and two-point chart decide your team's choices on the snaps that don't stop for you; the AI coaches keep their own judgment.
- **Officials who make mistakes (G2).** Each play can be a close call (a catch at the sideline, a fumble or down by contact, the ball across the goal line, the spot against the line to gain, an interception or a trapped ball). A crew of average eye gets 16% of close calls wrong (`WRONG_SHARE`); each crew's eye is drawn per game (0.7 sharp to 1.4 poor) on the officials' own stream, so no other draw moves. A wrong ruling stands unless replay sees it and the video is clear (78%, `EVIDENCE`). Replay by level and era (`Replay::of`): NFL coach's challenges from 1999 (two, a third from 2004 when both win; a lost one costs a timeout), the booth on scoring plays (2011) and turnovers (2012) and inside two minutes, replay assist from 2023; college booth reviews of any play from 2006 and one coach's challenge; high school none. AI coaches challenge by their replay assistant's read and the stakes. The replay assistant reads a wrong ruling right clamp(0.80 + 0.025 × (game management - the level's typical head coach's, `coach_norm`), 0.55, 0.92) of the time (minor bug 140: read against 10, NFL staffs sat at the cap, threw only on sure things, 0.29 challenges a game won 54%), and says "throw it" on a right one about a third as often as he misses. Measured over 1,000 NFL games (`match_check nfl 1000 7`): 0.89 wrong rulings a game (target about 0.8), 0.38 coach's challenges a game won 38% (real 2024: 0.47 a game, 50 of 133 won), 0.85 booth reviews a game with 56% reversed (real about 0.7, a little over half), 0.28 wrong rulings left standing; most close calls are catches, then goal-line rulings, spots, fumbles and interceptions. The challenges run a little rare and the booth a little busy: proposed, not confirmed. Each close call is kept on its play record (`Officiating`) for the game page.
- **The coin toss (G3).** A real choice on its own stream: receive, defer, kick or take the wind; the winner of a deferred toss chooses again at halftime. AI winners in the NFL defer about 82% (more with better game management) from 2008 (before, they receive); college 78%, high school 60%. In overtime NFL winners receive (88%); college and high school winners take defense first (90%). The wind changes ends each quarter.
- **Records for the game page.** Each throw keeps where across the field it was snapped and where it arrived (`throw_y`), for the pass chart. Each 4th down keeps the model's true win chance of going, punting and kicking and the chance to convert (`fourth`), for the game page's ledger. Both are output only.
- **Measured.** `match_check nfl 200` before and after the lane: points 27.19 to 27.27 a team game, plays 65.3 to 64.9, interceptions 1.01 to 0.96, penalties 5.99 to 5.91; within the noise of the officials' rulings, the only change to results. Toss winners defer 78% in the NFL (target 82%), 76% in FBS (78%) and 56% in high school (60%). `match_check` now prints the officials and the toss.
- **Box score turnovers** count interceptions and lost fumbles only (a missed field goal, a turnover on downs or an onside kick recovered by the kickers used to count), each to the team that gave the ball away.

## Final build, lane 3: engine physics (2026-10-21/22)

Agreed items D1-D4 (`docs/design/dynamic-links.md`) and A5, A7-A10 (`docs/final-build/list.md`). `match_check`, 1,000 games a level, on main before the lane and after it:

| | NFL before | NFL after | FBS before | FBS after | HS before | HS after | Real |
|---|---|---|---|---|---|---|---|
| Points a team game | 27.4 | 27.7 | 22.1 | 22.3 | 15.6 | 17.2 | NFL 22.5, FBS 27.0, HS about 26 |
| Completion | 64.2% | 64.5% | 57.9% | 59.4% | 51.2% | 54.9% | NFL 64.5%, FBS 60.6% |
| Sack rate | 5.5% | 5.5% | 8.3% | 7.0% | 13.2% | 12.6% | NFL 6.8% |
| Interceptions a game | 1.01 | 0.98 | 0.71 | 0.80 | 0.35 | 0.48 | NFL 0.76, FBS 0.80 |

NFL points (agreed band 20-25) and FBS points stay parked tuning, not this lane's.

- **D1, size at the catch point.** Each man's highest reach is his standing reach (0.82 of his height plus his arm, plus 4 inches) plus the jump he has left: his vertical (the combine's fit from jumping and burst for his weight) loses up to a third at full speed and 40% when he can't see the ball (`scrimmage.rs`, `high_reach`). A ball thrown up high goes to the longer man more often: the reach difference counts like 18 contested-catch points per 10 inches, only as the ball arrives above 6 feet. A ball over the defender's reach is one he can't play. The defender's reach at the catch point now counts his arms too.
- **D2, the quarterback sees the size mismatch.** On a vertical or breaking route 8+ yards down (anything but the flat in the red zone), a covered receiver who reaches 2+ inches higher than the man on him counts as open by about 0.07 yards per inch past two, more for a quarterback who reads it (football IQ, decisions) and a receiver who wins contested balls (`size_window`). Thrown for that reason, it is a jump ball put just under his high point; a back-shoulder throw against a trailing defender who can't see it.
- **D4, the 12 Dynamic Links** (`links.rs`). Each is 2-4 body traits measured against his position on the real combine (`combine_by_position.csv`; the athletic traits the combine doesn't test are read against a combine class the player model makes). The weakest part sets the strength, smooth from the top 30% to full at the top 5%. Nothing is drawn: a link is read from his body and traits as they are, so weight programs, age and injuries move it. A full link is worth 5 rating points in its own situations only; in one situation the strongest counts in full, the next at half, a third at a quarter. Where each acts: Highpointer the reach and timing at the catch point; Burner 1.25% top speed on deep routes, past the line, on returns and chasing from behind (`link_speeds`); Sudden the cut, the first man missed, slipping hands in space, the release; Low center of gravity balance through contact, run-block leverage; Battering ram through arm tackles at speed, broken tackles, the thumper's hits and wearing tacklers down; Length first contact in blocks and rushes, the jam, batted balls, tackling reach and the catch window; Big man who moves pass sets against speed, reaching the second level, finesse rushes, splitting doubles; Anchor the bull rush and doubles on both sides; Get-off the first step; Bend the speed rush and the tight turn at speed; Lungs fatigue and recovery (`flow.rs`); Pocket frame batted balls, the read over the line and shrugging the arm tackle in the pocket. How common (the unit test, synthetic rosters, each link at least partial): high school 0%, FBS 0.1%, NFL 10.8% on average per link. Measured on today's body, a high school link is very rare because boys' athletic traits are still far below their grown ones; staffs therefore recruit the projected link (D3).
- **A7, high school scoring** (15.6 to 17.2). Three causes found, each fixed where the engine read a rating on the pros' scale with nothing on the other side to match it: the receiver's catch radius (jumping and body control) now uses the passer's `pro` scale, as his aim does (41% of high school throws missed their man, 22% in the NFL; completion 51% to 55%); the quarterback's choice to scramble or throw it away when nothing is open does too (high school quarterbacks scrambled at a quarter of the pros' rate and took the sack: scrambles 1.3% to 4.8% of dropbacks, real NFL 4.9%; sack rate 13.2% to 12.6% with the wider team spread); and `match_check`'s high school teams now differ as real ones do (quality sd 6, not 2.5: real winners average a 26-point margin). Still short of about 26: the rest is pressure (43% of dropbacks against 28% NFL, as high school rushers' burst outruns blockers' technique), returns (17 yards a kickoff return) and fewer drives (9.4). Not tuned further with flat knobs; the next trial measures the world.
- **A8, help against the best rusher.** Each staff's plan names the other team's best rusher when he is clearly (3+ points, as its film study sees it) better than its tackles (`calling.rs`, `help_on`). On a pass a spare lineman or back slides to him and doubles him first; with no spare man, a back or tight end on his side chips him (0.25 of a block's win, 0.6 s) before his route (`slide_to`). A high school defensive tackle 20 points better than everyone: 62% of his team's sacks and 3.4 a game before, 54% and 2.3 after.
- **A9, the best receiver gets the ball.** When the best receiver in a pass's progression (by his receiving rating) isn't its first read, the play's first look goes to him 10% plus 2% per rating point of edge over the next man, up to 55% (`FEED_*`).
- **A10, interceptions.** A defender's hands on a ball he can play are read half against his level (`ball_hands`): the throws below the NFL are already worse, so a high school ball hawk is one among high school passers. High school interception rate 1.5% to 2.3% of throws; FBS 0.71 to 0.80 a game (real 0.80).
- **A5, lower-division outliers.** A dominant rusher's line came partly from offenses never adjusting to him; A8 makes them. What is left comes from his ratings against his level, and his staff and scouts see those ratings.
- **High school games in a live save.** Every high school game of a live season runs on the engine (`hs_year.rs`, `hs_on_engine`); a school that can't dress a team forfeits. `friday_check` (small world, scale 0.05): the busiest Friday plays about 300 games in 3.1 s on 4 cores, heap flat at 0.46 GB through the season. A full-scale Friday is about 20 times that, near a minute.

The coaches' call table (`call_values.csv`) was not rebuilt: rebuilt on this engine it lifts NFL scoring from 27.7 to 28.7 and drops the NFL sack rate to 4.1% (real 6.8%), further from real, so it waits for the parked NFL tuning.

### Decided in the final build (Claude's pick)
- Links are read, not drawn: a "random stream" would have made a link a label; reading it from the body keeps it real and lets it change over a career.
- A full link's 5 points go into the composite of its situation (the rush power, the block, the evade), not into one attribute under it.
- Burner 1.25% top speed: 5 speed points take about 0.047 s off a forty.
- Help on a rusher when he is 3+ points better than the tackles; the chip costs 0.25 of a block's win.
- Feeding the best receiver: 10% plus 2% a point, at most 55%.
- Defenders' hands half against the level (all of it doubled high school interceptions past the real rate).
- High school match checks use a team quality sd of 6.

## Frames for the views (final build, lane 13)

The 2D and 3D game views (later extras topics 2 and 3) watch a game by playing it again from its seed with frames on. How the film room works on screen is in `docs/game.md`, "The film room".

- **Frames are output only** (`pose.rs`). Each tracked tick (0.1 s) of a recorded play keeps every man's spot, his facing, his action (stance, run, block and the man he blocks, engaged, rush, cover, pocket, throw, catch, carry, tackle and whom, wrapped, fall, down, get up, celebrate, a fake or the mesh), the ball's spot, height (a pass's arc from release to the catch point, a kick's hang) and mode, and who holds it. They are read from the play as the engine already has it and never draw a number or change a player: `tests/views.rs` plays NFL, FBS and high school games with and without frames and gets the same game snap for snap.
- **Frames only for a game a screen shows.** Nothing records frames during the world's games. A game of yours that you keep (the last 40) keeps its setup (`replay.rs`, `ReplaySetup`: both teams as dressed, the rules, the conditions, the seed, the kickoff hour and roof for the views), packed with MessagePack and zstd to about 50 KB. Watching plays it again with frames on.
- **A game you called live replays with your calls.** The setup keeps the live game's answers (`Control`), so the replay makes your calls at the same snaps; the test plays a live game with calls in the first half and gets it back exactly, film and all, and a different game without the answers.
- **An engine change makes old setups play a different game.** The film room checks the replay against the kept box score (score, plays, each play's kind and yards) and says the game can't be played back when they differ.
- **Receivers near the goal line** (found by the film checks: a go route from the 10 ran through the back of the end zone, and a man was tracked 15 yards past the end line). A route's points now stop a yard inside the end line (a go route becomes a fade to the back line), and a receiver still running at the end line slides along it toward the middle of the field. This changes results slightly in the red zone.

## The other leagues' rule books (final build lane 4, X7)

`game/books.rs` gives each other league its rules by season: `Rules::for_book(book, year)`, with the rule book's own settings in `Rules.code`. An American game (`Book::American`, every NFL, college and high school game) takes the engine's old paths, so its results per seed don't move. The field of the game running on a thread is `geom::field()`, set for a game by `FieldGuard`. Everything that knew the field was 100 by 53.3 yards now reads `goal()`, `end_line()` and `width()`. Where a league's published rule book couldn't be checked line by line, the rule below is Claude's reading of its reported rules, marked "(reading)".

- **CFL** (`cfl`):
  - **Field and players:** 12 a side on a field 110 yards long and 65 wide, with 20-yard end zones from 1986 (25 before). From 2027 the field is 100 yards with 15-yard end zones and the posts at the back. Three downs. The hash marks are 24 yards in.
  - **Before the snap:** the defensive line sits a yard off the ball. Every back may be moving and receivers may run forward at the snap.
  - **Kicks:** no fair catch (a 5-yard halo). The single (rouge) scores for a kick not brought out of the end zone; under the 2026 rule only one that settles there scores, and one through the end zone comes out to the 40. A missed field goal is a live kick.
  - **The convert:** a kick from the 5 and two from the 5; from 2015 the kick is from the 25 (a 32-yard try) and two from the 3.
  - **The clock:** the three-minute warning, and the clock stopping after every play in the last three minutes. The play clock is 20 seconds from the referee's whistle (about 30 from the end of the play), 35 from the end of the play from 2026.
  - **Challenges and timeouts:** none to 2006, one to 2013, then two. One timeout a game, two from 2013.
  - **Overtime:** possessions from the other team's 35, at most two rounds in the regular season, and a two-point convert required after a touchdown from 2010.
  - **Calling:** the league's pass lean is -0.16. It brings the CFL's three-down passing to 40.2 attempts a team game on synthetic rosters (it was 42).
- **Arena football** (`arena`):
  - **Field and players:** 8 a side on a 50-yard field 85 feet wide, with 8-yard end zones, posts 9 feet wide at the end line and rebound nets beside them. Most players go both ways until 2006 (`ironman`).
  - **Kicks:** no punting; drop kicks allowed. Kickoffs go from the goal line. A kick off the nets stays live, and one over the side walls comes to the 20.
  - **Tries:** a kick from the 3 or a play from the 3.
  - **Defense:** man coverage only (a zone call becomes the man look with the same safeties), and at most four rushers.
  - **Motion and clock:** one receiver in motion toward the line. The clock runs in the last minute (reading).
  - **Overtime:** one possession each, then the next score wins.
  - **Calling:** the pass lean is +0.28.
- **The World League and NFL Europe:** the NFL rules of the era, plus the two-point try all along and four points for a field goal from 50 yards. 15-minute quarters and sudden death.
- **The XFL of 2001:**
  - **Tries:** no kick; one point for a play from the 2, and from the playoffs two from the 5 and three from the 10.
  - **Punts:** no fair catch (5 yards for the returner), and a punt past 25 yards is a live ball.
  - **Other:** one back may run toward the line at the snap, and the play clock is 35 seconds.
  - **Overtime:** four downs from the 20 each, and only a touchdown counts.
- **The XFL of 2020 and 2023 and the UFL from 2024:**
  - **Kickoff:** the XFL kickoff, with both lines set five yards apart and nobody moving until the catch. A short kick comes out to the 45, one into the end zone to the 35.
  - **Tries:** plays for one, two or three from the 2, 5 and 10.
  - **Punts:** a punt touchback comes out to the 35.
  - **Onside alternative:** fourth and 15 from the 25 in 2023, fourth and 12 from the 28 in the UFL.
  - **Clock and timeouts:** two timeouts a half, and a 25-second clock from the spot.
  - **Overtime:** a shootout of tries from the 5, five rounds in 2020 and three after.
  - **The UFL of 2026:** a kicked point from the 15 again, two from the 2 and three from the 8. Four points for a field goal from 60 yards, no punting from past midfield outside the last two minutes of a half, and one foot in bounds.
- **The AAF (2019):**
  - **No kickoffs:** the ball starts at the 25.
  - **Tries:** no kicked try; a two-point play from the 2.
  - **Onside alternative:** fourth and 12 from the 28 when behind by 17 or in the last five minutes.
  - **Rushers:** at most five.
  - **Overtime:** one round from the 10, with a two-point try after a touchdown.
- **The USFL of 1983-85:** NFL rules with the two-point try from the 2, and sudden death.
- **The USFL of 2022-23:**
  - **Kickoff:** from the 25.
  - **Tries:** a kicked point, or plays for two from the 2 and three from the 10.
  - **Onside alternative:** fourth and 12 from the own 33.
  - **Overtime:** a best-of-three shootout from the 2.
- **The UFL of 2009-12:** NFL rules with the two-point try.

**What the leagues' plays share.** The players and the play come from the same engine: only the rules change.
- **Personnel:** with 12 a side the offense adds a receiver and the defense a defensive back. With 8 a side the offense and defense are cut to arena football's sets (`for_players`).
- **Tries:** a two-point (or one- or three-point) try is booked as a try, never as a scrimmage touchdown. This fixed the American game's two-point try too: it was booked as a six-point play in the stats.
- **Playoffs:** a playoff game plays its overtime out (`GameSetup.playoff`), so no title game ends tied.
- **The other fields:** every place that read a 100-yard field now reads the game's field. That covers fourth-down and field-goal choices, the expected points behind your own calls and replay rulings, the officials' spots near the goal, and kicking distances (`kick_add`).

**Measured** (`match_check <book> 300`, synthetic NFL-level rosters, points a team game against each league's real average): CFL 27.6 (24), arena 51.6 (52), NFL Europe 25.6 (21), XFL 2001 26.3 (20), XFL 2020/23 27.1 (20.5), AAF 27.4 (20.2), USFL 1983 24.7 (23.5), USFL 2022 27.5 (20.4), UFL 2009 25.3 (21), UFL 27.7 (20.5). The engine's NFL on the same rosters runs 21.9% over real points, 31% over passing yards and 12% over attempts, so the spring leagues are about 10% above that bias. Their real rosters were thinner at quarterback than synthetic NFL rosters; in the world (`leagues_check 2025 5`) the CFL scores 22-27 and arena 50-57, but the spring leagues still score 26-30 (`docs/world-model.md`, "The other leagues as full leagues").

**Thin rosters:** an arena team dresses 20-odd men, so a blowout can pull nearly all of them. The pulled starters come back for a kick unit before it takes the field short, as they already did on offense and defense (`Side::kick_unit`); a test world once crashed on a kickoff with nobody to field it.

**Known gaps:** CFL teams punt about 9.7 times a game (real about 6-7). The spring leagues run 14-18% more plays than real. The CFL's best passers throw for more than the league's real record (6,619 yards).

## The game boards' records (update 2, lane 9)

What the six game boards read (`docs/game.md`, "Lane 9: the game boards") that the engine didn't keep before. New records are kept only for your own games and your coming opponents' film, in small fixed-size structs, this season and last at most; nothing is kept for the rest of the world. Every new field reads empty from a save made before them.

- **The line chart** (`game/chart.rs`, `PlayRecord::line`). On each snap of a game that keeps its setup (your games) or is played live: every block's rep (the two men, the leverage when it ended, how long it held, the defender's last move, whether it was a pass block, a shed, a double team or a pancake, where each man lined up), every pass rusher (the gap he aimed at from the line's spots, whether he was a down lineman, picked up at the snap or in the stunt) and the protection (man, a slide either way, max) with the men kept in. 16 bytes a rep and 8 a rusher in memory: about 50 KB a game. No other game charts. It only reads what the snap already did, so a game plays the same with it: `tests/win_chance.rs` plays NFL and high school games with and without it and gets the same plays and score.
- **Timeouts at the snap** (`PlayRecord::timeouts`), for the win chance.
- **The film's throws** (`world/tape.rs`, `Throw`). Each pass on your coming opponents' tapes keeps its air yards and yards after the catch, where across the field it was snapped and where it got to (half yards), the time to throw, the receiver's separation, and whether it was thrown away or dropped: 8 bytes. Your own passer's throws come from your games' play logs.
- **Each level's numbers** (`world/charts.rs`, `Tally`). Every engine game adds both sides to its level's tally (the NFL, FBS, FCS, the small colleges, high school): dropbacks, pressure and sacks, throws at a man by depth with their air yards and time to throw, designed runs with their yards, yards before contact, stuffed runs and runs of 10 or more. This season's and last season's, about 100 bytes a level.
- **Your state's high school polls** (`charts.state_polls`). With a high school seat, each poll of your state (preseason, weekly, final) keeps its top 25 and your school, all classes and your class, this season and last. The college polls and the NFL standings were already kept; the NFL's playoff picture week by week is rebuilt from the season's results by its own seeding rule (`World::nfl_race`).
- **Pair chemistry** (`charts::pair_chemistry`, `chemistry::pair`): the seasons two men have played together, read as the engine's chemistry, for the locker room.
- **Names in your kept games** (`world/seat_names.rs`, `SeatGame::gone`). A player who leaves football at the year's turn leaves the world, but last season's games are still kept. Before he goes, each of your kept games keeps his id, name and position, so its pages and boards still name him (only the men in those games: a few dozen at most).

### The win chance

`game/win_chance.rs`: the chance to win before each snap, for the boards, the game page's chart and key plays and the game day screen. The engine's own quick read (`live::win_prob`, what the coaches weigh) is untouched, so games play out the same.

- **The drive first, then the rest of the game**, the way the public models do (Pro Football Reference's, nflfastR's). The drive's touchdown and field goal chances come from the spot, scaled so the drive is worth the down, distance and spot's real expected points (read between the table's 5-yard rows), and shrink when the half's clock is too short to get there: about six yards and six seconds a snap in a hurry, plus about sixteen seconds of running clock unless a timeout stops it. A drive that stalls near the edge of range is part punt, part missed kick. Each ending takes its own time: a scoring drive about eight yards a snap, a stall about four and a half snaps. The drive's split has small kinks where kicks give way to punts, so a spot's split is never read as worth less than one up to 25 yards further back, the clock still running from the real spot. Late, the habits blend in with the clock rather than switching on at one moment (the old switch at 5:00 moved the chance in one second and left two-score comebacks near zero): a side level or ahead burns the clock from 7:00 of the fourth, fully from 3:00 (its four downs first, about 36 seconds a snap less 30 for each timeout the other side spends); the side behind in the fourth picks its pace, usual (27 seconds a snap), in a hurry (14, scoring as soon as the short clock lets it) or slow (to leave the other side little time: the play clock run down every snap, and never forced to score sooner than from up to 25 yards further back); anyone short of time plays for the kick from 6:00 of the half, fully from 4:00, and from outside range must get there first (the chance falls off over 25 yards; a fourth down out there is one snap). On the half's last snap a side kicks from where it is, nothing past the longest kick, or throws for the end zone. A kick's make falls away to nothing over the last seven yards to the longest one. The margin moves more a minute in the last ten (the side behind goes for it and kicks onside), and a drive's length is read at seven lengths around its usual one, so no moment of the clock is a step. A field goal's make comes from the distance by level (the NFL's and FBS's from their real 2021-25 kicks). After the drive, the final margin is about normal around the lead plus the pregame line spread over the time left, with a spread that shrinks with the square root of the time left (Stern 1991). With no line (high school, the other leagues, a game the books skip) the gap between the sides is unknown, so the score so far also hints at it: a normal prior on the gap, as wide as the books' lines spread at the level, updated by the lead beyond what the home field said, which moves the rest of the game's mean and widens its spread (`Params::without_line`). A leader with the ball who can kneel out the clock against the other side's timeouts has won. Overtime plays by its level's rule (the NFL's timed period, college's possessions from the 25, high school's from the 10). In college and high school the log is followed round by round (`home_series`): the side that answers knows what it needs (level, any score wins it), and a try ends the possession, so the other side's chance is read from the spot, not a kickoff; an answering side's try settles the round. The game day screen's bar reads the same model before the next snap, try or kickoff (`home_now`).
- **Per level**: the quarter's length, the two-minute warning, the play clock, field goal makes, the try's worth, where a kickoff starts and the margin's spread (13.5 points over a game in the NFL, 16 FBS, 16.5 FCS and small colleges, 18 high school), fitted to each level's games on the engine.
- **The line.** The pregame spread is the closing line where the books hung one (stored with each kept game, playoff and Super Bowl lines included), else the home field (1.7 points in the NFL, 2.5 elsewhere, none at a neutral site), with the score read as a hint of the gap (above).
- **Checked** (`tests/win_chance.rs`): sane at known states at every level (the opening snap near even, a 7-point favourite well ahead, a kneel-out won, down 14 with two minutes left nearly lost, up 10 at the half well ahead), never falling as the lead grows and never by more than one percentage point as the ball moves closer (the expected points table has small kinks); on the last second a kick only from range and as often as it is good (down 2 at midfield reads under 8%, no cliff a yard apart); for the side behind in the fourth, no step at any second (never more than 2 points), from its own 25 never a gain from 6:00 to 5:00 (it can use all that time itself), and at most 1.2 points in one second (a side whose drive would use the clock anyway gains a little as the other side's time to answer shrinks, as in real play); college and high school overtime possessions and tries read as above; and calibrated on 600 engine games in the NFL and 240 each in FBS and high school (each pairing both ways, no line, the home field as the boards read it; lane 4 raised it from 60, where one engine change moved the NFL's 20-40% fifth by 0.2): a Brier score of 0.130 in the NFL over 101,773 snaps, 0.162 in FBS over 41,903 and 0.078 in high school over 32,387. In the NFL and FBS each fifth of the range the side won within 0.08 of the chance said (the test holds 0.15); the NFL's side behind wins a little less than said (0.31 said, 0.23 won). High school's synthetic league is far more lopsided than real high school (final margins about 36 points from even, real about 25), so a lead there decides more than in life: its middle fifths read 0.30 and 0.70 where the side won 0.06 and 0.81. Before the deeper game plan (lane 5) changed the engine's games, the same league's 30 games read 0.101 with every fifth inside 0.15.

## The deeper game plan (update 2, lane 5, 2026-10-25)

G1-G23 reach the engine through each team's staff (`Staff::guide`) and the week's prep (`roster::Prep`); the screens are in `docs/game.md`, "Lane 5", and every team plays by the same rules (the AI's guidelines: `docs/world-model.md`, "The head coach's guidelines").

- **Guidelines** (`game/guide.rs`, G1-G2). A head coach's base front, man share, pressure rate, system, personnel group, tempo, run share and stick to the card, each with a firmness that pulls the coordinator's own habit toward it: firm 85%, a lean 55%, his call 15% (`expected`). `call_defense` moves its coverage weights to the expected man share inside man and inside zone (`steer_man`, so a third and long still gets its coverage), sends five or more toward the expected rate and plays the base front or nickel against base personnel as asked; against three or more receivers a 4-3 team still goes to nickel. `call_offense` leans its run share, personnel group and no-huddle share the same way, and the down, the distance and the score still rule. The spring system pick weighs the guideline (`world::guide::install_bonus`), and a coordinator calls what he is asked as well as he knows it (`systems.rs` knowledge through his calling fit), never with a flat penalty. Tested in `tests/gameplan.rs`: a man guideline's share rises strictly with its firmness and lands near halfway at a lean; a firm 4-3 still plays nickel against 11 personnel most of the time.
- **The call sheet** (`game/sheet.rs`, G7-G13). Both play callers call from the sheet when they stick to it (`Prep::stick`, `dstick`, each his own habit moved by the program's stick guideline). Offense (`off_call`): the opening script in its order on normal early downs, not when the clock or the plan forces a run or a throw; a per-side counter moves only when a script call is snapped, so an audible doesn't repeat one and a pre-snap foul doesn't skip one (`Side::scripted`). Shot plays between the 35s, never in the hurry-up, on a two-point try, in the four-minute offense, in a decided game or on a forced call. Then the first box for the situation (`boxes`): the two-minute box only when the offense must hurry (`Situation::hurry`), the four-minute box when it leads late (the same window as the four-minute plan's tempo). A box reads the ball's hash list and then its either-hash plays; a ball in the middle reads both hash lists in turn, then the either-hash plays (`box_list`). The play caller has read the snap as a run or a throw: a box without that kind leaves his own call standing (the two-point list is called whole). A box is his menu, not a ranking (proposed, not confirmed): his own call stands when its play is on the box; else he thinks the snap over the way he calls any snap, up to 8 times (`REREAD`), until his call is on it; else he takes the box's play most like his own (the same play, then the same depth with or without play action or the same run family, then the same kind), the box's order breaking ties (each next 0.85 as likely, `ORDER`). So the order counts in full only on the shot plays, and a box changes his calls only where it leaves plays out. A shot play replaces a throw he called, never a run. The wet version in bad weather.
- **The defensive sheet** (`def_call`, G8): inside the coordinator's own read. He has called man or zone and a four-man or bigger rush. His call stands when the box has it (with his pressures on it on every down when he is sending pressure); else he thinks it over, keeping his rush of four or of more, up to 8 times; else the box's call of his kind closest to his (the same coverage, then the same shell), and with none of his kind his call stands. The pressure package comes out on passing downs only when he was sending pressure anyway. A decided game (his easing) or a call tipped by a man with your words stays his own. The defense's script (up to 12 calls, 8 drafted) runs in order on early downs, counted like the offense's. Every staff drafts one (below).
- **The week against one opponent** (G15-G20). The answer to each personnel group (package, man share, five-man rush: `Memory::plan.answers`). Snap limits, base downs only and the line's rotation by series (`Side::held`) count on scrimmage snaps only: the limit and base downs on both sides, the rotation only while the team defends, so a two-way man's offensive snaps and the kick units are never rested by it; "every other series" splits its men into two turns by their place on the board, half sitting odd series and half even. The instructions for this opponent (through the tendencies' instructions). The quarterback's four checks at the line: out of a run into a loaded box; out of a throw, to the run, against a light box with two deep; a hot throw against Cover 0 (not when the shell fooled him); to the run against two high on early downs. The "if they do this, we go to that" lines, checked at each new drive and fired once when the trigger shows in this game's snaps (40% blitz on passing downs, three scrambles, 60 yards to their top receiver, 5 yards a carry after 10 carries, six or fewer in the box on 60% of snaps, two catches of 15 yards on a corner).
- **Every team's checks and lines** (Cain's rule: the same rules for every team). A team without a week's plan (every AI team) has its staff read the quarterback's checks and the "if" lines off the opponent's film at kickoff (`roster::staff_checks`, `staff_ifs`): the hot throw when Cover 0 shows on one snap in twelve or more, the run against two high when half their coverages are two-deep; bracket the top receiver and attack a corner always, screens and draws when they blitz a quarter of their defensive snaps, a safety down when they run 45% of the time. Your staff's picks on the week's plan are the same reading, which you can change.
- **Special teams** (G18; `game/kicking.rs`, `game/flow.rs`). A squib is a ground ball: aimed about their 24-32 (12-18, inside the NFL's landing zone), an up-back fields it 2-6 yards on, 3.5-4 seconds after the kick, with the coverage running all that time and no fair catch. The coffin corner is tried from their 45 in and aims 19 yards from the middle; farther out it is a punt like any other. A fake punt or a surprise onside kick "if the look shows" goes when the return unit shows it and the kicking staff reads it: on a punt, nobody home, from the returner's ReturnRisk tendency and the front men's Discipline against the level's norm (about one punt in seven from an average unit, 4-35%); on a kickoff, the front line leaving early, from its Discipline (about one in fifteen, 2-16%); the staff reads it at 0.45 + 0.025 × its film study (50-95%) (`return_look`, `sees_look`). The look is drawn from its own key, so the game's stream is unchanged, and kept in the play log (`PlayRecord::look`) so the next opponent's tape shows it. "At the first chance" goes once. A surprise onside kick is logged and spent only once your onside answer stands. The fake is played from the punt formation against the return unit's own men (R25; `Game::fake_punt_play`, `Snap::fake_punt`; `scrimmage.rs`, `punt_spot`, `return_spots`, `return_jobs`). The punt team: the snapper over the ball, its four heaviest on the line, its two fastest wide as gunners, the punter 15 yards deep (14 in college, 12 in high school) and its best runner (ball carrier vision, ball security and speed) as the up-back 5.5 yards back, who takes the snap and runs a power. The return unit lines up as it does for a punt: the returner 40 yards deep (38, 32), never inside 10 yards of the goal line, its two fastest jamming the gunners, its four heaviest on the line, two more on the edges and two 5 yards off the ball. Without the look, the line and edges rush the punter and the two off the ball sit in short zones. With it (nobody home) the front has already turned to set up the return, bailing to about 15 yards downfield, and reads the run 0.25 s late, times 0.5-1.5 by Discipline. Proposed, not confirmed: the roles from the units, the depths, the bail and the delay. `match_check` with `AFM_FAKE=2` (every team fakes at the first chance) and `AFM_FAKE=1` (only when the look shows), 400 games: in the NFL a fake into a set return unit converts 31% for 2.5 yards a try (271 fakes), one against the look 67-73% for 5.9 (39-41); in FBS 43% for 4.5 (327) and 75-78% for 5.7-6.9 (45-47). There is no real fake-punt rate in the research yet.
- **The four-minute plan** (G4). Run it out (39.5 s and a run 60% of the time) and keep attacking use one window for the tempo and the calls: the 4th quarter, ahead by 1-16, 5:00 or less, the last two minutes included (`sheet::four_minute`).
- **Call words** (G5). A man who left with your words reads a call like a tell (`DefCtx::tip` for their defense, a defense's disguise off for their offense) about one snap in nine, three in ten at most. He counts from the side's own words: a coach on the side he came up on, so new words for one side close that side's leak only (`world/plans.rs`, `knowers`). The week new words go in, each man's bust chance gains 1/770 × max(1.3 - IQ/100, 0.3) / 0.7 (`Prep::learn`; `game/scrimmage.rs`): about one more bust in 70 snaps for a side of average minds (60), more for a slow learner, less for a sharp one.
- **Measured** (`match_check nfl 400` and `fbs 400`; the synthetic teams draw guidelines the way the world does). Before the lane, after it, and after the review round (AI staffs reading checks and "if" lines off film, the rotation on defense only, the squib and coffin corner, the look for fakes and onside kicks): NFL points 27.78, 27.46, 27.21 a team game; passing yards 292.8, 286.6, 284.7; rushing 101.7, 104.1, 105.3; sacks 2.17, 2.21, 2.20; interceptions 1.005, 0.981, 0.921; man share 27.6%, 27.3%, 27.2%; pressure 30.7%, 30.7%, 30.6%; field goals from 40-49 84.5%, 87.2%, 87.5%; 4th-down tries 1.85 a game in both later runs. FBS points 22.03, 21.86, 21.65; passing 239.9, 240.6, 239.4; rushing 123.7, 121.8, 122.9; interceptions 0.736, 0.795, 0.820; man share 27.5%, 27.3%, 27.1%. Every move is inside the run-to-run noise of 400 games.
- **Every staff drafts a call sheet** (lane 5 follow-up; Cain's rule that every team plays by the same rules, and every real staff drafts one). A team with no week's plan (every AI team) has its coordinators draft one at kickoff, the same draft as your seat's (`world/plans.rs`, `kickoff_card`), from the other team's film and the tells in its own film (`film_tells`), kept on the game's side (`Side::card`) and dropped with the game, so the world stores nothing more. A draft costs 4-6 ms on one core (22 offense lists of 21-25 plays, 15 defense boxes of 11 calls) against 17-21 ms for a game without one (`match_check`: 20.9 ms a game with no drafts, 29.8 with two, NFL). A season of history played on the engine at every level, a tenth of the world (`memory_check 2026 2 0 0.1` with `AFM_ENGINE_FROM=2025`): 391 s with no drafts, 371 s with college and NFL staffs drafting, 558 s with high school staffs too (6,884 high school games), peak memory 0.96-0.97 GB in all three. So college and NFL staffs draft at kickoff and high school AI staffs don't (`sheet::KICKOFF`, proposed, not confirmed): their drafts would add about 43% to every season on the engine, and a sheet is no edge (below), so your high school team's own sheet stays fair against them. Proposed, not confirmed: a box is the play caller's menu (above); a box holds the plays behind 98% of his drafted calls there, one line a play, at least its own size and at most 30 a list (12 a defense box); the script is his first 15 calls in its situations in the order he made them; a shot play replaces only a throw he called. Why: the earlier short boxes (3-8 plays, the first drawn most) cut the tail of his calls and left his most-valued plays, worth 1.0-1.7 points a team game when every team drafted (NFL 28.2-28.8 against 27.1-27.4 with none), and the sorted script and the shots that replaced runs added their own; a real sheet holds more plays than a game uses, and the engine's play callers were set against real games, sheets and all. `match_check` (`AFM_SHEET=every`, every team drafting at every level; unset, the game's college and NFL drafts; `none`; or `home`, the home team only), 400 games: NFL 27.37 points a team game with every team drafting, 27.21 with none (1,200 games: 27.70 and 27.38), the home margin 0.52 (se 0.83) and 0.33 (se 0.81); FBS 22.18 and 21.65 (1,200 games: 22.28 and 21.98), margin 1.46 (se 0.95) and 1.20 (se 0.89); high school 16.86 and 16.67, margin 0.62 (se 1.31) and 1.73 (se 1.23). The home team alone with a sheet: margin 1.45 in the NFL, 0.74 in FBS and 1.43 in high school, against 0.33, 1.20 and 1.73 with none, inside the noise both ways. The play caller's own call is on the box on 92-93% of box snaps, a second thought finds it on 6-7%, the closest play stands in on about 1%; on defense 84%, 9-10% and 6-7%. The defense's man share 32.1% with sheets and 31.2% without, its blitz share 20.9% and 20.7% (NFL).


## Throw depth and passing volume (minor bugs 135 and 93, 2026-10-30)

Cain said yes to both on 2026-10-30, knowing the fix lowers NFL yards an attempt, which he had kept as it was on 2026-10-19.

- **Why NFL throws went too deep (135).** Receivers left the line at a full sprint and the slant broke at 45 degrees, so at the real time to throw every route was caught deeper than real, and deep balls were caught too often. Over 6,000 `match_check nfl` games at the branch's start: 15.5% of targets at 20+ air yards (real 11.5%, nflverse 2021-2025), 36.2% at 0-9 (real 50.1%), 32.0% at 10-19 (real 20.7%); 10-19 throws complete 59% (real 54.5%) and 20+ 42% (real 36%).
- **The release** (`game/scrimmage.rs`, `footwork`, `RELEASE` 0.5, `RELEASE_PACE` 0.25). A wide receiver or tight end starts each route with 0.5 × clamp(1.3 - 0.006 × Release, 0.7, 1.3) s of footwork at a quarter of his top speed, straight ahead: the stance, the release move and the first steps of the stem. None for a man in motion at the snap (moving 1.5 yd/s or more), a bubble or a tunnel screen; a lost jam adds to it; the quarterback doesn't read him until it ends. Backs keep their own start (the blitz check, the check-release, the screen's wait).
- **The slant** (`game/playbook.rs`, `Route::shape`) breaks at about 30 degrees: real NFL slants are caught 6.2 air yards past the line (NGS 2023-2025, nflverse participation, `data/real-world/nfl_route_depth.json`), the old shape at about 12.
- **Deep balls** (`throw`): the error along the line grows 3.5% a yard past 12 yards downfield (was 2.2%).
- **The world threw deeper still (135).** After the release, the test teams threw 13.6% of NFL passes at 20+ air yards, but a 15-season world on the same engine threw 19.4% (the book's `G8.nfl_deep_share`), with completion 58.7% (real 64.5%). The world's play callers read at about 14 of 20 (their coordinators: NFL 14.4, FBS 14.1, FCS 12.0, high school 9.9) and the test staffs at 10, so the world's trusted the coaches' table of what beats what (`trust`) about twice as much. That table was measured on an older engine and rated the deep and intermediate concepts 3-5 yards a snap above the rest against every look (dagger 11.2, Y-cross 11.0, play-action dig 11.0 against mesh 1.8), so the sharper the caller, the more of them he called.
- **The table, rebuilt and centered** (`game/calling.rs`, `values`; `crates/afm-engine/data/call_values.csv`). Rebuilt on this engine at 2,000 snaps a cell (it was 400, at which one cell could be off by a yard or more), and each pass concept's row is now centered on its own average over the looks, as the runs' rows already were: a sharp caller leans toward what beats the looks he expects, not toward the concepts that gain the most everywhere. The defense's coverage choice is unchanged (its costs all move by the same amount). Proposed, not confirmed.
- **Test staffs like the world's** (`game/synthetic.rs`, `finish_team`). The test teams' play callers now read and adapt around the level's own (`coach_norm` - 1: NFL 14, FBS 13, FCS 12, small colleges 11, high school 8; spread 2.5), as their head coaches have since minor bug 140, so `match_check` sees what the world's callers do. With the old table this alone took the test teams back to 15.6% deep. Proposed, not confirmed.
- **The silent count stands first.** A road receiver waits at the line for `SILENT_RELEASE` (0.38 s × crowd × focus) before his footwork; it was 0.3 s and he then ran, which the release made worth less (above, "Home field").
- **Season lines (93).** The world added each pro's playoff games into his season line, so a Super Bowl team's passer threw for 21 games and counted them in his 4,000-yard seasons, records and career totals. Playoff lines are now kept apart (`ProSeason::playoffs`; `world/pro/games.rs`, `record`), as real season lines are.
- **`match_check`.** Yards an attempt are now gross, before sacks, as the real figure is (the team's passing yards are net of sacks); a table of targets by route group against NGS 2023-2025 (share, catch rate, yards a target, air yards, time to throw; `data/real-world/scripts/route_depth.py`).

Measured over 6,000 `match_check` games a level (seed 11). Base: the branch's start (3f875ea) with the old test staffs; release: the release, slant and deep-ball changes with the old test staffs; now: everything, with test staffs like the world's.

| NFL | base | release | now | real |
|---|---|---|---|---|
| points a team game | 25.0 | 22.4 | 20.7 | 22.5 |
| pass attempts | 33.6 | 33.4 | 33.9 | 33.3 |
| completion | 64.0% | 64.1% | 65.7% | 64.5% |
| yards an attempt (gross) | about 8.0 | 7.21 | 6.64 | 7.05 |
| passing touchdowns | 1.84 | 1.52 | 1.35 | 1.46 |
| air yards < 0 / 0-9 / 10-19 / 20+ | 16.2 / 36.2 / 32.0 / 15.5% | 16.1 / 46.6 / 23.7 / 13.6% | 19.6 / 48.8 / 20.7 / 10.9% | 17.6 / 50.1 / 20.7 / 11.5% |
| completion by depth | 79 / 71 / 59 / 42% | 80 / 72 / 53 / 39% | 80 / 72 / 53 / 36% | 79 / 71 / 55 / 36% |
| sack rate | 6.2% | 6.1% | 5.6% | 6.8% |
| time to throw | 2.55 s | 2.55 s | 2.47 s | 2.71 s |
| pressure | 32.8% | 32.7% | 33.9% | 28.3% |
| tight end targets | 11.5% | 13.2% | 13.5% | 21.9% |
| home margin | 2.2 (se 0.22) | 2.2 (se 0.22) | 2.3 (se 0.21) | 2.1 |

Targets by route now (share, real): go, fade and seam 4.5% (8.1%), post 4.8% (4.7%), corner 4.0% (4.2%), dig, in and cross 9.2% (9.0%), slant 5.8% (6.2%), drag 6.2% (6.8%), screen 9.9% (10.5%); with the old table and world-like staffs the posts were 9.2% and the digs 15.2%.

FBS now: points 18.4 a team game (real 27.0, mostly the run game: 4.0 yards a carry against 5.1, which Cain kept as it is on 2026-10-19), yards an attempt 6.65 gross (real 7.29; college sacks count as runs, so real college passing yards are gross), passing touchdowns 1.18 (real 1.57), completion 60.9% (real 60.6%), 20+ air yards 10.3% of targets, home margin 2.8 (se 0.25; real 2.4); at the base 21.1 points, about 7.7 yards an attempt, 1.48 touchdowns, 58.5%. High school (2,000 games): points 16.8 at the base, 15.8 now. Yards an attempt in `match_check` before this branch were net of sacks, so the bases above are estimated from its sack counts at about 6 yards a sack.

Why the test teams now pass for less than real with the depth mix at real: yards after the catch, 4.3 a completion against the real 5.2 (the world's 4.0), short on every route (slants 1.2 against 4.6, screens 7.4 against 9.6, from `nfl_route_depth.json`'s yards a target, catch rate and air yards). The deep throws had covered it: minor bug 162, fixed next (below, "Yards after the catch").

The world (93 and 135): a 15-season world at a quarter's scale (seed 2026), against the same world at the branch's start. NFL throws at 20+ air yards 19.0% to 12.7% of attempts (real 11.5%), air yards an attempt 12.6 to 9.1, completion 60% to 63% (real 64.5%), yards an attempt 8.6 to 6.7 (qualified passers; real 7.15), 4,000-yard passers 17-19 to 4-5 a year in 2022-2025 (real 6-10, mean 7.75), the top passing season 6,500-7,080 yards to 4,500-5,060 (real record 5,477), 1,000-yard receivers 46-59 to 10-16 (real 20-28), sacks 6.2-6.6% to 4.6-5.1% of dropbacks (real 6.8%). FBS points 25.7 to 21.1 a team game (real 27.0), completion 58% to 61% (real 60.6%). So passing volume went from far over real to a little under it, for the yards after the catch (4.2-4.4 a catch in the world, real 5.2): minor bug 162 (below). The world before this fix (release and slant only, the old table) threw 19.4% at 20+ yards (the book's `G8.nfl_deep_share`) with 6-13 4,000-yard passers a year.

Still off, not touched: go routes caught at 33 air yards (real 25) and thrown to 4.5% of the time (real 8.1%), posts and corners at 23-25 (real 19-20), digs and crossers at 15.5 (real 12.5); time to throw 2.47 s (real 2.71) and pressure 33.9% (real 28.3%); tight end targets (minor bug 137); NFL yards a carry 3.9 on the test rosters (real 4.5; the world's runs sit near real). Proposed, not confirmed: the release's time and pace, the slant's shape, the deep ball's 3.5% a yard, the 0.38 s silent count, the centered table and the test staffs' level.

## Yards after the catch (minor bug 162, 2026-10-31)

Found while fixing 135: with NFL throws at their real depth, passing ran a little under real because yards after the catch ran a yard short, at every depth.

- **Why.** A free defender in space more than 3 yards past the line got his tackle try from 2.5 yards (`OPEN_REACH`, plus up to 0.4 for a sure tackler and his arms): a full-length dive, from anywhere around the runner. So the nearest defender tackled a receiver at the catch from up to three yards away (a slant receiver caught 2.4-2.7 yards from a linebacker sitting behind him went down where he caught it: in the world 43% of slant catches gained under a yard after the catch), and second-level tackles on runs came as early. The reach was raised from 2.2 to 2.5 against long runs (above), which hid the yards lost everywhere else.
- **The fix** (`game/sim.rs`, `SCRIMMAGE_REACH` 2.0): on scrimmage plays a free man's reach in space is 2.0 yards, arm's length and a lunge step; returns keep 2.5, which the return fixes of minor bug 139 were measured on. Everything else in the try and the miss is unchanged. Proposed, not confirmed.

Measured over 6,000 `match_check nfl` games (seed 11), before and after:

| NFL | before | after | real |
|---|---|---|---|
| yards after the catch | 4.29 | 4.86 | 5.20 |
| by air yards < 0 / 0-9 / 10-19 / 20+ | 7.30 / 3.13 / 2.70 / 3.61 | 7.69 / 3.62 / 3.34 / 4.55 | 9.05 / 4.12 / 3.69 / 5.61 |
| yards an attempt (gross) | 6.63 | 6.98 | 7.05 |
| yards a carry | 3.87 | 4.37 | 4.50 |
| runs of 10+ yards | 4.4% | 6.5% | 11.6% |
| runs of 20-39 / 40+ yards | 0.8 / 0.5% | 1.1 / 0.9% | 1.8 / 0.5% |
| points a team game | 20.6 | 23.1 | 22.5 |
| passing / rushing touchdowns | 1.34 / 0.66 | 1.54 / 0.84 | 1.46 / 0.91 |
| third downs converted | 37.5% | 38.3% | 39.8% |

Real yards after the catch by depth from nflverse 2021-2025 (`nfl_situations.json`, `pass_by_air_yards`). FBS (3,000 games): points 18.2 to 21.2 (real 27.0), yards a carry 3.95 to 4.48 (real 5.12), yards an attempt 6.63 to 7.04 (real 7.29). High school (2,000 games): points 15.7 to 17.8, yards a carry 4.80 to 5.39.

The world: the same 15-season quarter world as above (seed 2026), before and after. NFL yards after the catch 4.4 to 4.9 a catch in 2025 (0-9 air yards 3.3 to 3.9, 20+ 3.6 to 4.9), points 19.9 to 22.8 a team game (real 22.5), yards an attempt 6.71 to 7.06 (real 7.05), yards a carry 3.99 to 4.69 (real 4.50; against 8+ in the box 4.27 to 4.76, real 3.50, minor bug 136), third downs 36.4% to 38.5% (real 39.8%), 4,000-yard passers 4-7 a year in 2022-2025 (real 6-10), 1,000-yard receivers 11-20 (real 20-28), the receiving leader 1,274 to 1,371 yards (real 1,811). FBS points 21.1 to 24.1 (real 27.0), yards a carry 4.73 to 5.33 (real 5.12), the receiving leader 912 to 1,328 (real 1,360). In 2025's NFL plays, slants gain 2.0 to 2.8 yards after the catch, drags 2.7 to 3.7, digs 3.2 to 3.7.

Tried and dropped (each over 4,000 games): a reach that shrinks with the runner's speed away from the tackler (yards after the catch at real by depth, but return touchdowns doubled); with the gap's opening speed instead (almost no effect); a reach that grows with the tackler's own speed toward the runner (return touchdowns to real, but punt returns of 20+ yards fell from 3.8% to 2.5%, real 8%).

Still off: runs past 20 yards go past 40 about four times in ten, as they already did at 2.5 (real about two in ten). The shorter reach lifts all runs, so 40+ runs now come 0.9% of carries against 0.5% real. Not the blocks downfield (a defender coming off his block near the runner lowered all long runs alike), not the deep safety's fit depth (deeper fills only moved tackles from 4-9 yards to 10-19) and not the pursuit angle: past the second level the chasers are behind the runner and close about a yard a second. Logged as minor bug 163 and fixed in the next section. Also still short: slants (about 1.4 yards after the catch against 4.6; caught at 7.7 air yards against 6.2), drags and screens.

## Long runs (minor bug 163, 2026-10-31)

Of NFL designed runs that reach 20 yards, 45% went past 40 in the engine; real is 21%.

- **Real long runs** (nflverse play-by-play 2022-2024, regular season, designed runs without kneels): 2.34% of carries reach 20 yards and 0.50% reach 40. Of the runs past 20, 40% end in the next five yards, 30% go out of bounds and 20% score. Inside runs are no different: up the middle or over a guard, 21% of the runs past 20 go past 40 and 37-44% end before 25. Runs to the end reach 20 most often (3.8% of them, against 1.5% up the middle).
- **The engine's** (4,000 `match_check nfl` games, every designed run traced at the 10 and the 20): the tosses and jet sweeps, run at the sideline, were already real (21% of their runs past 20 went past 40, 35% ended out of bounds). The inside runs were the bug: 41-64% of inside zone, power, counter, draw, trap and iso runs past 20 went past 40, 3-14% went out of bounds and 36-53% scored. On the runs that went 40+, the runner at the 10 had 0.12 free defenders ahead of him; the corners and the nickel had been in a receiver's block on nine in ten of them and half were still in it, behind him. At the 20 every defender was 7 to 10 yards behind him, so it was a footrace he won. The open-field tackling itself is real: free defenders miss about 10% of their tries on backs past the line, at every depth, like PFF's missed tackle rates (from memory).
- **The check:** the same games with backs that no block can hold bring the 40+ share to 20%, real (but runs of 10+ yards fall to 2.9%), so the blocks on the backs are the cause, not the chase.
- **The fix** (`game/sim.rs`, proposed, not confirmed): a block in space holds a back only while the runner comes at him. Once the runner is past the line and more than 3 yards to his side (`SPACE_BLOCK_GAP`), a corner, nickel or safety in a block works off it, at 0.6 a tick (`SPACE_BLOCK_LEAVE`, about a tenth of a second), and pursues: a receiver's stalk block mirrors a man and can't hold one who runs around it. The last man in front of the runner takes a deeper angle, 1.4 times the meeting point along the runner's path instead of 1.1 (`LAST_MAN_LEAD`), so he keeps the runner in front of him and inside while the pursuit comes.

Measured over 6,000 `match_check nfl` games (seed 11), before and after:

| NFL | before | after | real |
|---|---|---|---|
| designed runs of 10-19 / 20-39 / 40+ yards | 5.7 / 1.1 / 0.9% | 5.4 / 1.2 / 0.5% | 8.2 / 1.8 / 0.48% |
| share of runs past 20 that go past 40 | 45% | 29% | 21% |
| yards a carry | 4.37 | 4.20 | 4.50 (designed runs 4.29) |
| kickoff / punt return touchdowns a team game | 0.020 / 0.019 | 0.014 / 0.009 | 0.012 / 0.012 |
| yards after the catch | 4.86 | 4.88 | 5.20 |
| yards an attempt (gross) | 6.98 | 6.96 | 7.05 |
| points a team game | 23.1 | 22.6 | 22.5 |

The deeper angle also brings return touchdowns to real (minor bug 139: kick returns 0.022-0.028 against 0.012). FBS (4,000 games): runs of 40+ 0.8% to 0.5% of designed runs, the 40+ share of runs past 20 36% to 28%, yards a carry 4.49 to 4.31 (real 5.12), points 21.4 to 20.6 (real 27.0; these rosters score low, see above).

The world: the same 15-season quarter world as above (seed 2026), before and after. In 2025's NFL designed runs, 40+ yards 0.9% to 0.4% (real 0.48%), 20-39 1.6% to 1.3% (real 1.8%), 10-19 7.1% to 7.6% (real 8.2%), the 40+ share of runs past 20 25% (real 21%; 31% for inside runs, 18% outside), 21% of runs past 20 score (real 20%) and 16% end out of bounds (real 30%), designed yards a carry 4.70 to 4.20 (real 4.29). The book: NFL yards a carry 4.69 to 4.33 (real 4.50; the gap left is the stuffs, minor bug 164), return touchdowns 0.042 to 0.023 a team game (real 0.024), plays ending out of bounds 8.2% (real 10.7%; 5.8% on the quarter world minor bug 141 was found on), the spread of backs' yards a carry 0.70 (real 0.57; 0.81 after minor bug 136's fix), against 8+ in the box 4.76 to 4.65 (real 3.50), points 22.8 to 22.6 (real 22.5), yards an attempt 7.06 to 7.16 (real 7.05), third downs 38.5% to 39.9% (real 39.8%). FBS yards a carry 5.33 to 5.12 (real 5.12), FBS points 24.1 to 22.9 (real 27.0).

Tried and dropped (each over 4,000 games): the runner's pace with the ball (0.90 of his top speed instead of 0.95 brings the 40+ share to 32% and 0.85 to 13%, but real ball carriers reach about their top speed on long runs, from memory of Next Gen Stats); taking the pursuers' meeting time past 4 seconds or cutting it to 2 (no change); a chaser's try from behind made surer (42%); the deep safety breaking down later or coming forward faster (40-44%); escorts that block pursuers less often past 10 yards (43-45%); backs shedding toward a runner within 4-6 yards whoever leads the block (36-40%, and yards a carry fell to 3.1-4.1); run blocks that stop driving the man back after a second (41-43%, and yards a carry rose to 4.8-4.9).

Still off (minor bug 164): the short end. 30.5% of designed runs gain nothing against 18.2% real and 22.9% gain 1-3 yards against 37.1%, while 4-9 yards run 39.5% against 34.2%. When a lineman makes the first tackle the run has gained 1.0 yard (real 2.44), a linebacker 1.7 (real 3.35), and linebackers make 26% of first tackles against 37% (backs 44% against 30%). With the short end short, runs of 10-19 yards (5.4% against 8.2%) and 20-39 (1.2% against 1.8%) stay short too.

## The run game: stuffs, first tackles and the box (minor bugs 164 and 136, 2026-11-04)

NFL designed runs were stuffed 30% of the time (real 17.6%), linemen and linebackers made their stops a yard or two too short, an eighth man in the box barely mattered, and backs' yards a carry spread too wide (0.79-0.84 against 0.57). Real numbers are nflverse play-by-play and participation 2021-24, backs' designed runs, regular season, the first tackler grouped by the role PFF lists (interior and edge are the line, off-ball linebackers the linebackers): line 43.1% of first tackles at 2.50 yards, linebackers 27.8% at 3.69, backs 29.1% at 6.96. `match_check`'s rows used the roster position before, which put 3-4 edge rushers with the linebackers (line 33.0%, linebackers 37.1%), while the engine's EDGE is a lineman.

What the run diagnostic (`AFM_RUNDIAG=1`) showed on main: 48% of backs' runs met their first tackler behind the line; linemen hit at a yard behind it and linebackers at the line, so the yards came in two humps (nothing, and 5-7 yards to a safety) with a hole at 2-4; receivers took defensive ends in their stalk blocks; linebackers were blocked to the whistle 1.5 times a run; a won block drove its man back for as long as it lasted; and a safety rolled into the box fitted at 6.5 yards like any safety, so the extra man made no stops.

What changed (`game/sim.rs`, `game/scrimmage.rs`; every number proposed, not confirmed):

- **Blocks fade.** A man driven back goes back for 2 seconds at most (`RUN_PUSH_BACK`, at most 1.6 yards), and a defender driving his blocker into the backfield does it for 1 second, still sliding toward the runner (`RUN_PUSH_UP`).
- **Linemen make the stop as the runner goes by.** Any block not sealed pulls the defender toward the runner within 2.2 yards at a full 2 a second at least (`SHED_RUNNER` 1.0, with leverage only and at 0.2-1.5 before); a second-level man feels him from 3.5 yards and pulls harder (`SHED_SECOND`). The blocker starts a little further ahead (w0 -0.4, was -0.1) so the runs still go.
- **Linebackers fill past the line.** They fill to 2 yards past it (was half a yard), hold 1.5 past it once they leave the fill instead of chasing the back into the backfield, and stop at their floor and let him come. The phone booth (always hit) is now only within 1.5 yards of the line, so a waiting linebacker can slip a climbing blocker (`PHONE_BOOTH`). Receivers no longer stalk down linemen.
- **The box counts.** A back inside 7 yards and 7.5 to the side of the ball (the box count) fits at 4.5 and attacks once the runner is 1.5 past the line; the other deep backs fit at 8.5 and attack at 3.5, the post's numbers (were 6.5 and 2.5).
- **The fall forward.** The pile moves at 3.0 (was 2.2), the runner falls 0.3 s of its momentum (was 0.25) and his body reaches up to 1.05 yards (was 0.9).

6,000 `match_check nfl` games (seed 11), main and after:

| NFL | main | after | real |
|---|---|---|---|
| backs' designed runs stuffed (0 or less) | 29.9% | 16.7% | 17.6% |
| runs of 1-3 / 4-9 / 10-19 / 20-39 / 40+ yards | 24.1 / 36.4 / 5.5 / 1.3 / 0.6% | 35.3 / 37.9 / 6.0 / 1.0 / 0.4% | 37.1 / 34.2 / 8.2 / 1.8 / 0.5% |
| first tackle: line, share at yards | 29.5% at 0.81 | 42.4% at 2.08 | 43.1% at 2.50 |
| first tackle: linebackers | 28.8% at 1.50 | 21.3% at 2.80 | 27.8% at 3.69 |
| first tackle: backs | 41.7% at 6.67 | 36.3% at 6.51 | 29.1% at 6.96 |
| yards a carry (backs' designed runs) | 4.13 (3.83) | 4.38 (4.11) | 4.50 (4.29) |
| run EPA | -0.168 | -0.085 | -0.050 |
| neutral downs, 6 / 7 / 8 / 9 in the box | 4.11 / 3.99 / 3.85 / 3.36 | 4.39 / 4.29 / 3.95 / 3.76 | 4.75 / 4.64 / 4.27 / 3.76 |
| 2 or less to go, 6 / 7 / 8 / 9 in the box | 4.23 / 3.88 / 3.37 / 1.33 | 4.32 / 4.08 / 3.42 / 1.73 | 4.18 / 3.45 / 2.79 / 1.13 |
| all runs, 6 or fewer against 8 or more | -0.56 | -0.71 | -1.26 |
| team seasons' yards a carry, sd | 0.77 | 0.59 | 0.45 |
| backs' 204-carry blocks, sd | 0.84 | 0.67 | 0.57 |
| yards after the catch | 4.81 | 5.09 | 5.20 |
| yards an attempt | 7.03 | 7.18 | 7.05 |
| third downs | 38.7% | 41.7% | 39.8% |
| points a team game | 22.9 | 23.9 | 22.5 |

Why the points rose: main matched real points only because its runs lost 0.17 EPA a carry; with runs near real (-0.085), the passing side's small excess shows (yards an attempt and third downs). The fall forward also gives receivers about a quarter yard after the catch (bug 162 had them a yard short).

The world: the released seed-1 world played one season on the engine (`play_on ... 365 ... engine`), main and after: NFL backs with 120+ carries 37 and 49 (real about 40 a season), their yards a carry 3.97 and 4.19 (real 4.32), spread 0.79 and 0.56 (real 0.57); the league's yards a carry 4.14 and 4.36, yards an attempt 7.25 and 7.33.

FBS (2,000 games): yards a carry 4.46 to 4.79 (real 5.12), run EPA -0.142 to -0.059, points 20.9 to 22.4 (real 27.0), yards an attempt 7.06 to 7.28 (real 7.29), third downs 35.2% to 38.8% (real 40.3%). High school (2,000 games): yards a carry 5.16 to 5.74, points 16.8 to 17.9 (no real target).

Tried and dropped (1,000 games each): a wider or surer arm grab from a blocked man (no change: the runner rarely passes within its reach); a head start for every second-level block (more early sheds everywhere, linebackers 22%); a slide of the whole fight toward the runner's track (more stuffs by linemen); leading the linebacker's fill by the runner's sideways speed (fewer yards, no more linebacker stops); a safety in the box chasing straight at the runner (stuffs by backs 15%, real 11%).

Still off: linebackers make 21% of first tackles (real 28%) and their stops come 0.9 yards short, so backs make too many (36%, real 29%); runs of 10-39 yards are still short (7.0% against 10.0%), which is most of the 0.18 yards a carry still missing; short yardage gains too much against 7 and 8 in the box; light boxes gain too little (5 in the box 4.17, real 5.75), so light against stacked is -0.71 against -1.26.

## Passing, the sideline and the pocket (minor bugs 137, 138 and 141, 2026-11-04)

Three passing gaps from the trial 10 book. NFL wide receivers drew 68.7% of targets in `match_check` (real 59.3%), tight ends 12.8% (real 21.9%), backs 18.5% (real 18.1%). Quarterbacks threw the ball away on 0.4% of attempts (real about 3%: FTN charting 2022-24 has 3.5% of dropbacks), FBS teams took 2.20 sacks a game (real 2.00, cfbfastR 2022-24) and college sacks per pressure ran 0.21 against the NFL's 0.15. NFL plays ended out of bounds 5.9% of the time (real 10.7%: completions 16.2% against 20.7%, designed runs 0.8% against 5.5%). Real NFL numbers are nflverse play-by-play and participation 2021-25 unless a line says otherwise.

What was wrong:

- **Tight ends ran the wrong routes.** They ran a route on 20% of pass snaps but drew 13% of targets (0.13 targets a route, wide receivers 0.22) and were the first read 5% of the time. The library gave them sits, flats and blocks: they ran no posts (real NFL targets on posts: 19% to tight ends), 2% of corners (real 26%) and no wheels (real 27%).
- **NFL teams went four wide like a college spread.** 19% of NFL dropbacks came from 10 personnel (real 1.0%, nflverse participation 2023-24), so a tight end wasn't even on the field.
- **Nobody gave up on a play.** Past his clock a passer forced the ball to anyone within a step of his man (window -1.2 or more). The throwaway chance was 0.3 of the formula, and a rusher who could reach him within the whole throwing motion stopped it, so almost every throwaway try in the pocket failed.
- **No scramble drill.** A passer who didn't run stayed in the pocket: there was no way to break out with his eyes downfield, so nothing ended outside the pocket but scrambles and sacks.
- **Tacklers pushed runners out only from 6 yards of the sideline.**

What changed (`game/playbook.rs`, `game/calling.rs`, `game/scrimmage.rs`, `game/sim.rs`; every number proposed, not confirmed):

- **Tight end routes.** Eight concepts give the tight end a real route: slant-flat (slant), flood (the 11-yard out, read first), Y-cross (read first), the play-action shot (read second), the play-action dig (a 10-yard post), fades (slant), drive (the drag, read first) and shallow cross (the 10-yard dig, read second).
- **The NFL's 10 personnel.** NFL play callers weigh 10 personnel at a tenth of college's and stay in 11 when they hurry (95%, was 70%) (`pick_personnel`).
- **Giving up.** With his best man still covered (window below 0) when he would force it, he gives up with chance clamp(0.8 + 0.015 × (sense - 75), 0.05, 0.95), sense the mean of Decisions and Football IQ on the passer's scale (`GIVE_UP`). From then he throws it away as soon as he can, or runs if a lane opens (three times as readily).
- **The flick.** A throwaway is stopped only by a rusher who reaches him within half the throwing motion, and the chance a tenth of a second is the full 0.06 + 0.003 × pro(Decisions) + 0.12 outside the pocket (was 0.3 × that).
- **The scramble drill.** With a lane, nothing open or pressure above 0.5, and 1.6+ seconds gone, a passer breaks out of the pocket at 0.15 a tenth of a second × clamp(1 - 0.5 × lean(Scramble), 0.3, 1.7) (a runner mostly takes off instead), toward the side with no free rusher, never within 5 yards of the sideline (`EXTEND`). Out there he takes whoever comes open, throws it away legally, or after 1.2 s runs three times as readily (`EXTEND_RUN`).
- **Pushed out.** A tackler within 8 yards of the sideline can drive the runner out (`PUSH_OUT`, was 6).
- The coaches' call table is rebuilt for the new routes (2,000 snaps a cell).

2,000 `match_check` games at each level, main and after:

| | main | after | real |
|---|---|---|---|
| NFL targets to WR / TE / RB | 68.7 / 12.8 / 18.5% | 65.6 / 17.2 / 17.2% | 59.3 / 21.9 / 18.1% |
| NFL pass snaps in 10 personnel | 18.9% | about 3% | 1.0% |
| NFL plays ending out of bounds | 5.9% | 6.6% | 10.7% |
| completions / designed runs out of bounds | 16.2 / 0.8% | 18.2 / 1.0% | 20.7 / 5.5% |
| NFL throwaways an attempt | 0.4% | 2.3% | about 3% |
| NFL intentional grounding a team game | 0.003 | 0.024 | 0.070 |
| NFL scramble rate | 5.0% | 4.8% | 4.9% |
| NFL sacks a team game | 1.87 | 1.90 | 2.41 |
| NFL sacks per pressure | 0.150 | 0.153 | 0.139 |
| NFL completion % | 66.0% | 65.0% | 64.5% |
| NFL yards an attempt | 7.17 | 7.14 | 7.05 |
| NFL interceptions a team game | 0.85 | 0.81 | 0.76 |
| NFL plays a team game | 62.4 | 63.4 | 61.9 |
| NFL points a team game | 23.7 | 24.4 | 22.5 |
| FBS sacks a team game | 2.20 | 2.01 | 2.00 |
| FBS sacks per pressure | 0.207 | 0.190 | |
| FBS throwaways an attempt | 0.4% | 2.8% | |
| FBS targets to TE | 12.6% | 15.1% | |
| FBS completion % / yards an attempt | 61.3% / 7.28 | 59.9% / 7.12 | 60.6% / 7.29 |
| FBS points a team game | 22.4 | 22.4 | 27.0 |
| High school sacks a team game | 2.31 | 2.21 | |
| High school throwaways an attempt | 0.8% | 2.8% | |

Why: a throwaway is an incomplete attempt, so completion % and yards an attempt fall by about its share, which put the NFL's completion % at real. FBS sacks fell to real because the passer who used to hold it until the rush got home now gives up first or breaks out of the pocket. NFL points rose 0.7: more plays stop the clock (36.0%, was 33.9%), so the NFL runs a play more a team game (stopped-clock time between snaps was already short: 12.6 s against real 15.2), and the rebuilt call table moves callers toward the concepts that gain more now (shallow cross with the tight end's dig went from 5.9 to 9.5 yards a snap in the table; yards an attempt 7.08 before the table, 7.14 after). Scrambles first fell to 4.4% with the give-up alone (the passer threw it away when he used to run); letting a passer who has given up run when a lane opens put them back at 4.8%.

Tried and dropped (400-1,000 games each): pursuers taking an inside-out angle to herd the runner to the sideline (points 27.8 to 38.3); runners stepping out more readily near the sideline (fewer yards a carry and explosive runs); pushing out from 10 yards (completions out 22.4%, past real 20.7%).

Still off:

- **Designed runs end out of bounds 1.0% (real 5.5%).** Wide runs are 27% of runs and only 2.4% of them end out (real end runs about 20%): the carrier's path turns upfield and ends 15+ yards from the sideline about two times in three. Fixing it means bending the run path to the sideline, which reworks the run game tuned in minor bugs 164 and 136.
- **Tight ends draw 17% of targets (real 22%).** Their targets go 9 air yards deep (real 6.4), so most of the targets still missing are short ones. NFL 12 personnel is 12.8% of pass snaps (real 17.6%) and 21 is 8.2% (real 4.9%).
- **NFL sacks 1.90 a team game (real 2.41)**, from before this lane: the ball comes out too soon (time to throw 2.49 s against 2.71, thrown after 3.94 s 2.5% against 10%), the gap minor bug 135 left for the trial.
- **NFL time between snaps on a stopped clock is short** (12.6 s against 15.2, late in halves 6.0 against 8.4), so the NFL runs 63.4 plays a team game (real 61.9), from before this lane; it is minor bug 254.
- In the last diagnostic build the NFL's throwaways came in the pocket about 1.9% of dropbacks against real 1.4 and outside it about 0.5% against 2.1 (FTN charting): the scramble drill gets few passers out before the play is dead.

## What bodies and ratings change: the A/B test fixes (minor bugs 274-289, 2026-11-07)

The A/B tests (`ab_check`, `afm/ab-tests/report.md` in the project folder) move one body number or rating for every player at a position, play the same games on the same seeds, and set the change in a stat against the real NFL's slope (nflverse, PFR and MockDraftable 2018-25, held equal on draft slot) or against how much real regulars differ. Bugs 274-287 are their misses; 288 and 289 are Cain's two yeses to the report's proposals (2026-11-07). Every size here is proposed, not confirmed, fitted so the A/B row lands in the real band.

What was wrong and what changed (`game/scrimmage.rs`, `game/sim.rs`, `game/flow.rs`, `game/kicking.rs`, `game/synthetic.rs`, `links.rs`):

- **A corner's arms barely moved the catch rate he allowed (274).** His reach at the catch point read 0.012 yard an inch of height and 0.02 an inch of arm; now 0.025 and 0.10 (`D_REACH_HEIGHT`, `D_REACH_ARM`). The cornerback research (2026-11-07) corrected the real target: with the depth of target held, height moves the catch rate allowed only -0.13 pts an inch (tall corners cover the deeper routes, bug 300), arms -0.67.
- **A tall, long receiver caught more of his targets; real ones catch fewer (275).** Real tall receivers get the harder balls. The passer now trusts a big man in a tighter window 5+ yards downfield and in the red zone, a window unit every 4 inches of height (arm counting twice) over his level's typical man at his position (`BIG_TRUST`), and the receiver's own catch radius reads half what it did an inch (0.0075 and 0.01).
- **A fast quarterback took fewer sacks; real fast ones take more (276).** Real scramblers hold the ball to extend plays. His clock gains 0.2 s and his hurry waits 0.07 s more a 10 Speed points over his level's typical passer (`LEGS_HOLD`, `LEGS_WAIT`); his choice to take off waits half of that (`LEGS_RUN`): all of it and his scrambles came late and lost their extra yards, none of it and they rose 1.8 times real's slope while his sacks fell to 0.4 times real's.
- **A tall quarterback had fewer batted balls and completed more over the middle (277).** Real batted balls don't move with height. The batted-ball check drops his own height (his level's typical height stays in it), and a tall passer in the pocket trusts a tighter window inside, 0.7 of a window unit every 2 inches over his level's typical passer (`OVER_LINE`).
- **A heavy carrier forced fewer missed tackles; real ones force as many (279).** The first tackler's whiff check read 0.5 Elusiveness, 0.3 Agility and his weight; now 0.8 Elusiveness and 0.2 Speed, with weight and agility left to his burst, braking and cut.
- **A fast receiver drew too few deep targets (280).** Past 15 yards the passer trusts a man who can run under it, 0.4 a 10 points of deep speed over his level's typical receiver, and his own arm the same over his level's typical passer (`DEEP_LEGS`, `DEEP_ARM`, the arm for bug 283).
- **Three ratings swung their stat more than real regulars differ (282).** Corner Discipline in defensive holding (×0.7), the hitter's part in forced fumbles (0.020 a point against the carrier's 0.025, `FUMBLE_HIT`) and Punt Power (0.14 yard a point, was 0.215; the net punt's swing is 0.90 of real regulars' spread, was 1.28).
- **Many ratings barely moved their stat (283), the ones with a clear place:** Arm Strength on deep throws (above), Decisions in the room a passer wants (0.02 a point, was 0.008), Pocket Presence in the sack escape (0.5 on the logit a 10 points), Route Running against man (0.02 s of the defender's lag a point of each man's distance from his position's typical man, was 0.0065 on the raw gap) and through the break (`BREAK_RR`).
- **Defensive tackles were credited 8.6 times PFR's pressures (284).** A pressure was logged when the nearest rusher got within 2.6 yards from the snap, with everyone within 2.8 credited, so a tackle lined up over the center counted next to a quarterback under center (60% of all pressures). Now the nearest rusher within 3.2 yards once 0.6 s are gone, crediting the nearest and any free or winning rusher within 2.6 (`PRESS_NEAR`, `PRESS_FROM`, `PRESS_CAUSE`). It is a label: play reads the passer's felt pressure, not this. Team pressure is 23.3% of dropbacks (PFR 22.2%), and the tackles' rate a snap is 0.6 of the edges' (real 0.6).
- **Heavy defensive tackles pressured more; real heavy ones pressure less (285).** An interior rusher's get-off now reads his Acceleration as his 10-yard split shows it at his weight: 0.38 a pound off his level's typical man at his position. Improved, not closed (below).
- **An edge's 10-yard split drove his pressures; real shows it doesn't once talent is known (286).** An edge on contain takes 0.15 of his get-off from raw Acceleration (was 0.6, the rest to Finesse Rush and Flexibility, `EDGE_BURST`) and half his speed rush's Speed and Acceleration (the rest to Finesse Rush, Flexibility and Hand Use, `EDGE_ARC_LEGS`), both read about his level's typical edge, who rushes as before. Improved, not closed.
- **Consistency wasn't game-to-game form (287).** Each man now draws his form once a game, N(0, 3) rating points on everything, 4.1 at Consistency 4 and 1.9 at 16 (`FORM_SD`). The engine's passers already varied from game to game a little more than real ones (true sd of EPA a dropback 0.167 against nflverse's 0.154), so form stays small: at 5 it added 1.4 points of runs stuffed at the line.
- **A tackle's arm length gave no reach in the hand fight (288, Cain's yes).** A rating point an inch over 34 in his pass set against speed and finesse in the NFL (`block_base`); at the other levels the pivot moves with their typical tackle's arm, so their typical tackle blocks as before.
- **The Scramble habit didn't enter the spy (289, Cain's yes).** The running threat a defense spies adds 0.5 × (Scramble - 10.5) / 9.5 (`qb_spy`).

Two rules keep the levels where they were:

- **Each new read is about the level's typical player** at the position (`synthetic::typical`, `typical_body`: mean ratings and body on 64 synthetic teams a level, the rosters `trait_norm` was measured on). Centred on NFL players, the deep reads took 2 window units off every FBS deep throw and the legs read 0.26 s off every NFL clock: FBS yards an attempt fell from 7.12 to 6.77 and NFL sacks from 1.90 to 1.69. Old reads whose slope changed keep their old value at the level's typical man (the corner's reach, the catch radius, the fumble, the punt, Decisions, the man lag).
- **A base off the trust a passer has in a big or fast man** (0.35 and 0.1 window units, `BIG_BASE`, `DEEP_BASE`; high school takes none off the big man, where 0.35 cut 0.3 points a team game and sent more throws behind the line, `big_base`). He takes his best target, so a trust that differs from man to man makes him throw more often even where it averages zero: without the base the NFL's 20+ yard share went from 11.1% to 12.2%, interceptions from 0.81 to 0.85 a team game and throwaways from 2.3% to 1.7%.

2,000 `match_check` games a level, main and after (real NFL from nflverse 2021-25, PFR for pressure; FBS real from cfbfastR 2022-24):

| | main | after | real |
|---|---|---|---|
| NFL points a team game / sd | 24.4 / 10.45 | 24.5 / 10.83 | 22.5 / 9.9 |
| NFL sacks a team game | 1.90 | 1.89 | 2.41 |
| NFL pressure rate (PFR's count) | 31.8% | 23.3% | 22.2% |
| NFL sacks per pressure | 0.15 | 0.20 | 0.28 |
| NFL completion % / yards an attempt | 65.0% / 7.14 | 65.2% / 7.20 | 64.5% / 7.05 |
| NFL interceptions a team game | 0.81 | 0.81 | 0.76 |
| NFL 20+ yard share of throws | 11.1% | 11.2% | 11.5% |
| NFL scramble rate / throwaways an attempt | 4.8% / 2.3% | 5.0% / 2.1% | 4.9% / about 3% |
| NFL targets to tight ends | 17.2% | 17.8% | 21.9% |
| NFL runs for 0 or less | 19.4% | 20.2% | 18.2% |
| NFL offensive holding a team game | 1.42 | 1.49 | 1.26 |
| FBS points a team game | 22.4 | 22.4 | 27.0 |
| FBS sacks a team game | 2.01 | 2.11 | 2.00 |
| FBS completion % / yards an attempt | 59.9% / 7.12 | 60.3% / 7.15 | 60.6% / 7.29 |
| FBS interceptions a team game | 0.66 | 0.69 | 0.80 |
| High school points / yards an attempt / sacks | 18.3 / 6.42 / 2.21 | 17.7 / 6.39 / 2.22 | |

The A/B rows after the fixes, NFL, 2,000 games an arm (high against low; real held equal):

| Bug | What moves | Stat | Engine | Real (band) | |
|---|---|---|---|---|---|
| 274 | corner height, an inch | catch rate allowed | -0.20 pts | -0.13 (-0.42 to +0.17), depth held | in band |
| 274 | corner arm, an inch | catch rate allowed | -0.87 pts | -0.67 (-0.14 to -1.21), depth held | in band |
| 274 | corner arm, an inch | breakups | +0.93 pts | +0.44 (+0.06 to +0.83) | at the edge |
| 275 | receiver height / arm, an inch | catch rate | -0.51 / -1.00 pts | -0.61 / -1.31 | in band |
| 275 | receiver height, an inch | red zone touchdowns a target | +0.17 pts | +0.57 (+0.10 to +1.04) | in band |
| 276 | QB forty, 0.1 s slower | scrambles / yards a scramble / sacks | -0.99 pts / -0.12 / -0.25 pts | -0.82 / -0.22 / -0.43 | all in band |
| 277 | QB height, an inch | batted passes / completion over the middle | +0.01 / -0.59 pts | +0.01 / -0.64 | flat, in band |
| 278 | back's forty, 0.1 s slower | runs of 20+ | -0.16 pts | -0.48 | 0.3× (waits on 292) |
| 279 | back's weight, 10 lb | missed tackles forced / yards after contact | +0.05 pts / +0.04 | +0.20 / +0.06 | flat, in band |
| 279 | back's weight, 10 lb | yards before contact | -0.04 | -0.12 (-0.05 to -0.20) | 0.3× (main -0.07, bug 304) |
| 279 | back's 3-cone, 0.1 s slower | missed tackles forced | -0.20 pts | +0.29 (-0.05 to +0.64) | miss (main -0.64) |
| 280 | receiver forty, 0.1 s slower | share of targets 20+ yards | -2.11 pts | -2.85 (-1.86 to -3.85) | in band |
| 281 | corner forty, 0.1 s slower | depth of targets on him | +0.01 (flat) | -0.33 | miss (role) |
| 282 | corner Discipline / safety Hitting / Punt Power ±15 | holding / forced fumbles / net punt | 0.85 / 0.80 / 0.85 of real regulars' spread | 0.1 to 1.0 | in range |
| 283 | Arm Strength ±15 | air yards / 20+ share | 0.35 / 0.41 of the spread | 0.1 to 1.0 | in range |
| 283 | Pocket Presence, Route Running ±15 | sacks / separation | 0.11 / 0.11 | 0.1 to 1.0 | in range |
| 283 | Decisions ±15 | time to throw / picks / sacks | 0.05 / 0.07 / 0.01 | 0.1 to 1.0 | still small |
| 285 | DT weight, 10 lb | pressures a defense snap | +0.42 pts | -0.22 (-0.15 to -0.29) | miss |
| 286 | edge 10-yard split, 0.1 s slower | pressures a defense snap | -1.92 pts | +0.49 (-0.01 to +0.99) | miss |
| 286 | edge forty / weight | pressures a defense snap | -0.18 pts a 0.1 s / -0.42 a 10 lb | -0.04 / -0.02 | miss |
| 287 | everyone's Consistency, 4 against 16 | QB's game-to-game EPA form | -0.012 | | no clear effect yet |
| 288 | tackle arm, 32.5 to 35.5 in | his man's pressures a pass block | -0.25 pts | | the right way (was flat) |

Raw rows: `afm/ab-tests/batch9/` in the project folder.

Expected completions (`game::stats::XCOMP`) refitted at every level with `xcomp_fit -- 600`: the pressure flag moved, so the old weights read CPOE -0.5 to +0.3. The analysts' tables (`decide_fit -- 1500 1`), expected rush and YAC (`xyards_fit -- 600`) and expected pass rate (`xpass_fit -- 600 7`) were refitted on the same engine; none of them feeds play. The coaches' call table was not rebuilt: it is played between two average NFL teams, where every new read is zero by construction.

Still off:

- **285, heavy defensive tackles still pressure more** (+0.42 pts a 10 lb, real -0.22). From 284 to 307 lb the slope is +0.11; the step comes above that (3.05% to 4.71% of snaps at 330 lb), from the Anchor and Battering ram links and the power rush's 0.08 a pound, which by design are the bull rush. Real's negative slope is likely role: heavy tackles are two-gap noses who come off the field on passing downs. A passing-down package is the likely fix.
- **286, the edge's split still drives his pressures** (-1.92 pts a 0.1 s, main -1.97; real +0.49), and his weight now costs more (-0.42 a 10 lb, main -0.24, real -0.02): the new pressure count credits the rusher who wins, and a heavy edge's power wins come slower than a light one's arc. Moving his get-off and arc to rush craft barely moved the split, so the rest likely sits in how fast he covers the ground, not in the rush scores.
- **278, a fast back's long runs** waits on bug 292 (safeties' run tackles, proposal 4).
- **281, fast corners see shallower targets**: real fast corners are the ones sent to cover deep threats; the engine's corners keep their spots whatever their speed, so it belongs with who covers whom (bug 300, proposal C1 of the cornerback research).
- **A heavy back's yards before contact** fall 0.04 a 10 lb (real 0.12, main 0.07): his weight used to cost him the first tackler's whiff, which the real broken-tackle numbers say it shouldn't (279). Real heavy backs reach the line slower; logged as minor bug 304.
- **Long corners gain at the catch without their flag cost** (bug 301: real long and heavy corners draw more holding). The cornerback research says fix them together (proposals C2 and C3); C3 waits with C1.
- **FBS sacks 2.11 a team game** (main 2.01, real 2.00): fast passers holding the ball add more sacks than slow ones save. NFL sacks stay under real (1.89 against 2.41), the gap minor bug 135 left.
- **The score spread widened** (NFL sd 10.83, main 10.45, real 9.9): more of a game rides on who the players are (the trust reads, form).
- **High school points fell 0.5** (18.3 to 17.7, no real target): form is a weakest-link cost (a bad day on the line hurts more than a good day helps), 0.4 of it in high school; the rest is noise at 2,000 games.
- **Runs stuffed at the line rose 0.8 points** (form; real 18.2%) and NFL offensive holding 1.49 a team game (main 1.42, real 1.26).

## Inside runs, the box and the carrier's spread (minor bugs 253, 342 and 334, 2026-11-08)

The batch before the Jan 1 rebuild (gate row 13). Real numbers are nflverse play-by-play and participation 2021-25 (`data/real-world/engine_targets.json`): designed runs by where they crossed (outside the tackles 5.06 a carry on 23.1% of runs, inside 4.05) and by the box (6 or fewer / 7 / 8 or more: 45.9 / 36.3 / 16.0% of runs at 4.73 / 4.22 / 3.40 a carry).

What was wrong on main (4,000 `match_check nfl` games, `AFM_RUNDIAG=1`):

- **Outside runs gained 3 yards a carry more than inside ones (253):** outside 6.05, inside 3.08 (real 1.0 apart). Nobody turned a wide run back in: the back on that side kept his deep fit while the runner got outside the box, and inside runs died at the line (linemen made 40% of first tackles at 1.96 yards, real 43% at 2.50).
- **Eight or more in the box on a quarter of runs (342):** 34.2 / 41.1 / 24.7%. Motion did it: against jet or across motion the strong safety spun down to 5.5 yards, inside the box count, whatever his depth before; and in zone the defender over the jet man lined up where he was in mid-motion, inside the tackles.
- **A back's broken tackles followed him from block to block about twice as much as real (334):** over two 182-carry blocks, r 0.44 (real 0.21), sd 0.041 a carry (real 0.030, chance alone 0.029). The miss and break-tackle checks read the whole gap between his evade and strength scores and the typical back's.

What changed (`game/scrimmage.rs`, `game/sim.rs`; every size proposed, not confirmed):

- **The force.** Once the runner is 4 yards outside the ball on a back's side (`FORCE_WIDE`), that back (not a deep zone) takes the box fit and attacks the edge, and a corner on that side stops holding his outside leverage (`defend`; 3 and 5 yards moved outside runs about 0.1 a carry either way).
- **The run block starts further ahead** (w -0.75 for runs, was -0.4, `RUN_BLOCK_START`) and **a lineman on a block sheds a little less as the runner goes by** (`SHED_RUNNER` 0.9, was 1.0), so the stops at the line come later. A start of -0.8 alone barely moved more (4.44 a carry); `SHED_RUNNER` 0.5-0.7 alone took linemen to 33-36% of first tackles.
- **The strong safety spins at his own depth** (`ss_depth`, the depth he had before the motion: the typical man just outside the box, a hitter inside it), and **zone defenders set to the jet man's spot before the motion** (`align_defense`).
- **The carrier's spread:** the miss and broken-tackle checks read 0.45 of his gap from his level's typical man at his position (`CARRIER_SPREAD`), who evades and breaks tackles as before.

4,000 `match_check nfl` games, main and after:

| NFL designed runs | main | after | real |
|---|---|---|---|
| yards a carry | 4.28 | 4.52 | 4.50 |
| inside / outside the tackles | 3.08 / 6.05 | 3.64 / 5.81 | 4.05 / 5.06 |
| crossing outside the tackles | 29.1% | 27.6% | 23.1% |
| 0 or less / 1-3 / 4-9 yards | 22.6 / 35.2 / 32.1% | 18.6 / 33.4 / 38.5% | 18.2 / 37.1 / 34.2% |
| 10-19 / 20-39 / 40+ yards | 8.9 / 1.0 / 0.3% | 8.3 / 0.9 / 0.3% | 8.2 / 1.8 / 0.5% |
| box 6 or fewer / 7 / 8 or more, share | 34.2 / 41.1 / 24.7% | 42.4 / 43.2 / 14.4% | 45.9 / 36.3 / 16.0% |
| box 6 or fewer / 7 / 8 or more, a carry | 4.23 / 3.99 / 3.45 | 4.47 / 4.29 / 3.39 | 4.73 / 4.22 / 3.40 |
| first tackle: line | 39.9% at 1.96 | 39.7% at 2.34 | 43.1% at 2.50 |
| first tackle: linebackers | 23.4% at 3.45 | 22.6% at 3.49 | 27.8% at 3.69 |
| first tackle: backs | 31.6% at 6.48 | 32.9% at 6.75 | 29.1% at 6.96 |
| backs' broken tackles over two 182-carry blocks, r | 0.44 | 0.27 | 0.21 |
| yards an attempt | 7.37 | 7.33 | 7.05 |
| points a team game | 24.5 | 25.1 | 22.5 |

Points rise 0.6 with the runs: the passing side's excess (yards an attempt) is the rest, and NFL passing stays as it is for now (Raichel, 2026-10-19). FBS (2,000 games): yards a carry 4.66 to 4.95 (real 5.12), inside 3.33 to 4.05 and outside 6.69 to 6.40, points 22.6 to 23.5 (real 27.0; these rosters score low). High school (2,000 games): 5.54 to 5.86 a carry, points 17.4 to 18.5 (no real target).

Tried and dropped: a contain end who widens with a runner outside him (no change); the carrier's pace in the open field (little change); a pursuer who leads a fast carrier as the typical man runs (no change); the press of the back's legs into the hole, read from his 10-yard split (no change); a linebacker who scrapes over the top by his speed and a slow one who fits under (bug 306: its A/B rows got worse).

Still off: inside runs 0.4 a carry short and outside runs 0.75 long, too many runs cross outside, runs of 20+ are half of real (bug 278), and linebackers still make too few first tackles. The coaches' call table and the expected rush model are refit on the new runs with the other engine lanes before the Jan 1 rebuild (its step 1: `call_values`, `xyards_fit`, `decide_fit`, `xpass_fit`), so the numbers above are on the old table.

After merging the rest of minor bug 283 (#287) and the edge lane (#279, main 1a0024d), 4,000 `match_check nfl` games give the same run rows within noise: 4.50 a carry, inside 3.62 and outside 5.81, the box 42.4 / 43.3 / 14.3% at 3.37 a carry against 8 or more, backs' broken tackles r 0.26 and 7.37 yards an attempt; points fall to 24.3 with the edge lane's sacks (2.33 a team game); FBS 4.94 a carry. Growth from results is refit on this engine (`results_fit -- 2000`): reads on in high school 18, FBS 19, FCS 20, small colleges 20, the NFL 19 (17, 20, 20, 20 and 19 on #287's fit), 32 to 36 rows a level; the rows that flipped sit near t 4 and flip both ways between refits (`docs/reference/how-it-works/development.md`). `results_ab 20000 1`, arm B: a season's overall moves 0.06 points sd (0.07 on #287's fit), a starter season's top skill +0.88 and -0.87 at the 90th and 10th percentiles (±0.93), a named big game in 13.4% of starter seasons (15.5%), a big season in 7.7% (8.5%) and a rough one in 7.6% (8.8%); the NFL's short throws spread 0.0457 (0.047, real 0.037) and carry 0.72 into the next season (0.74, real 0.23).

After merging the edge sack tail (#290, main 6f5f85d), 4,000 `match_check nfl` games give the same run rows within noise once more: 4.53 a carry, inside 3.64 and outside 5.85, the box 42.5 / 43.3 / 14.2% at 3.38 a carry against 8 or more, backs' broken tackles r 0.27 and 7.34 yards an attempt; sacks are 2.17 a team game after that lane (2.33 before) and points 24.7; FBS 4.96 a carry. Growth from results is refit on this engine too: reads on in high school 18, FBS 17, FCS 19, small colleges 19, the NFL 20, 32 to 36 rows a level, and among the 165 rows on in both fits the slope moved 5% at the median. The `results_ab` sizes above were measured on the fit one lane earlier and were not run again.

Measured in the same batch, not fixed (`ab_check`, 2,000 NFL games an arm, seed 1, after the changes above):

- **A fast back's long runs (278):** from a 4.78 to a 4.38 forty, runs of 20+ go 0.80% to 1.28% of his carries, 0.3× real's slope (real held equal +0.48 pts a 0.1 s). The run changes above left it where it was; the carrier's open-field pace and the pursuers' angle on a fast man did not move it.
- **A heavy back's yards before contact (304):** -0.048 yards a carry a 10 lb (real -0.124, band -0.050 to -0.198), just outside the band, as on main (-0.047); missed tackles forced stay flat with weight, as real. The press of his legs into the hole did not move it.
- **A linebacker's speed and run tackles (306):** across the whole position the fast men still collect the run tackles (-0.0017 a defense snap a 0.1 s slower, real +0.0037). Read within the field, against the linebackers beside him (`lb_tk_within`, the way real's same-draft-slot slope is a cross-section), the engine gives -0.2 to -1.0 pts a snap a 0.1 s slower, real +0.37: the miss is in the engine, not the measurement.
- **A corner's speed and target depth (281):** read within the field, against the corners beside him (`cv_adot_within`), the engine gives -0.32 to -0.54 yards a 0.1 s slower (real -0.33, band -0.15 to -0.51), so the miss was the A/B's whole-position read, which can't show a cross-section.

## Edge rushers: bend, arms, strength and rare freaks (E1-E3, minor bugs 286, 328, 330 and 339, 2026-11-08)

The edge research (`afm/ab-tests/research-edge.md` in the project folder) set the engine's edges against 978 real edge seasons (2018-25). Cain said yes to its three proposals on 2026-11-08 with one rule: "it's important to not limit strength or force into this build. A 6'0, lean Aaron Donald should still exist." So nothing here caps a rating, a body or what a combination can reach: every new read is a tilt about the level's typical edge (`synthetic::typical`, `typical_body`), and the size terms add for one body without taking from another. Every size is proposed, not confirmed.

What changed (`game/sim.rs`, `game/scrimmage.rs`, `body.rs`, `links.rs`, `athlete.rs`):

- **E1, bend finishes the rush.** The sack check gives an edge on contain extra reach to the quarterback for Flexibility over the typical edge's, 0.2 yard a 10 points, from -0.3 to +0.6 (`BEND_REACH`): a man who bends the corner flattens to the quarterback. His get-off, arc and speed move read less Flexibility (`EDGE_GETOFF_BEND`, `EDGE_ARC_BEND`, `EDGE_SPEED_BEND`), the rest as Finesse Rush. A tall man's bend counts more at the corner, above the typical edge's height only (`EDGE_TALL_BEND`); a short man's keeps its value. The 3-cone reads more bend (0.45 Agility, 0.2 Acceleration, 0.35 Flexibility; was 0.2 Flexibility), and an edge's 3-cone and shuttle knacks go together 0.7 (`EDGE_KNACK_TOGETHER`), so drafted edges' two drills go together 0.60 (real 0.62; 0.42 before).
- **E2, arms finish for the power rusher and close for the quick one.** Still on his blocker, an edge reaches 0.18 yard an inch of arm over the typical edge's, scaled by his weight (none at 13 lb under the typical, up to three times at 26 over; `ARM_REACH`). On a run, a quick edge's long arms reach 0.15 yard an inch farther as he closes, one step for each 0.15 s his forty beats the typical edge's, up to two (`close_reach`, `EDGE_CLOSE_REACH`). Short-armed and slow men lose nothing. The run reach is half the first size: at 0.3 yard every edge's arms added +0.27 pts of run tackles a defense snap an inch (main +0.10, real +0.08, band -0.04 to +0.21).
- **E3, strength pays with mass.** The power move adds 0.3 a Strength point over the typical edge's for each 10 lb over the typical edge's weight, up to 20 lb (`EDGE_STRENGTH_MASS`, `EDGE_MASS_SPAN`). At or below the typical weight Strength keeps its full value. Without the 20 lb span, 295 lb men at edge reached 34 sacks a season.
- **339, too few pressures ended in a sack.** Every rusher's reach in the sack check now adds a finish: 0.2 yard a 10 points of Tackling and Pursuit over 60, read half his own and half the typical man's (`FINISH_REACH`, `FINISH_MARK`, `FINISH_OWN`). Read on his own in full, the best finishers ran away with it (34 sacks a season). An NFL edge is about 74 in both, an FBS one 54 and a high school one 33, so the lower levels barely move.
- **286, the 10-yard split drove an edge's pressures.** His get-off no longer reads raw Acceleration (`EDGE_BURST` 0; the typical edge's burst, plus 0.8 of his Finesse Rush and 0.2 of his Flexibility about the typical edge's), and his arc keeps a quarter of his raw speed and burst (`EDGE_ARC_LEGS`, was half). On a pass rush an edge slower off the mark than the typical edge builds up speed as the typical edge does; a quicker one keeps his own burst (`rush_feet`). His weight, turn and top speed stay his own: reading every edge's feet as the typical edge's brought the split row to +0.3 but failed the freak check (heavy edges took 35% of the top seasons, light ones none).
- **328, the agility drills drove pressures too hard.** Agility's share of an edge's finesse move is 0.12 (was 0.25), the rest read as Finesse Rush and Hand Use about the typical edge's (`EDGE_FINESSE_AGILITY`). A 99 Agility man still beats a 90.
- **330, the generator's tall edges rushed worse at the same overall.** It gave a 6-5½ edge about 1.2 points an inch more coverage and recognition and less rush craft than a 6-2 one. Now 1.4 points an inch over the combine's height move back from Zone Coverage, Play Recognition, Pursuit and Anticipation to Hand Use, Finesse Rush and Power Rush, the overall kept (`EDGE_TALL_LEAN`). The spread is untouched.

The freak check (`freak_check`, new): ten seasons of 32 synthetic NFL teams on combine bodies, every edge season with 200+ defense snaps, and one 6-0, 240 lb edge given 99 Strength, Agility and Finesse Rush (team 0's best edge, his other ratings his own). Seeds 82 and 83; main is c0bc6f9, after is this lane merged with dd9ac18 (real: 2018-25 edges, the top 9-10% of pressure seasons):

| | main 82 | after 82 | main 83 | after 83 | real |
|---|---|---|---|---|---|
| under 250 lb, top 10% / all | 10.6% / 7.7% | 11.7% / 7.8% | 17.0% / 6.4% | 13.3% / 6.2% | 18% / 19% |
| 290 lb or more, top 10% / all | 5.8% / 10.6% | 11.7% / 10.7% | 0% / 17.0% | 0% / 17.1% | |
| arms under 33 in, top 10% / all | 67.3% / 67.4% | 69.9% / 66.9% | 81.1% / 68.1% | 83.8% / 68.5% | 32% / 24% |
| 6-5 or taller, top 10% / all | 10.6% / 18.7% | 11.7% / 18.7% | 12.3% / 20.6% | 4.8% / 20.8% | 21% / 29% |
| best sack season | 21 | 27 | 20 | 29 | 22.5 and 23 |
| sacks a season, top 1% / 5% | 16 / 11 | 22 / 15 | 17 / 12 | 22 / 16 | |
| the freak's median rank in pressures (best) | top 6.5% (0.2%) | top 2.5% (0.1%) | top 22% (5.6%) | top 31% (3.3%) | with the best |
| the freak's sacks a season | 6-18 | 11-23 | 6-15 | 5-20 | |
| sacks a pressure, all edges | 22.2% | 27.4% | 22.2% | 28.1% | 27.3% |

Light and short-armed edges hold at least their share of the top seasons, and nothing caps the freak: on seed 82 he is a top 2.5% rusher most years with 11-23 sacks, more than on main. The seed 83 freak has lower Acceleration, Flexibility and Speed (88, 79, 80); his pressures rank lower than on main, his sacks higher. One man's rank swings a lot from season to season (top 2% to top 60% for the same ratings), so a median over ten seasons moves about 10 points from noise. The synthetic league's bodies come from the combine, so its shares of light (7%) and short-armed (67%) edges differ from real regulars'; the check is the top share against the league's own.

2,000 `match_check` games a seed, NFL averaged over seeds 82-85 (main dd9ac18 and this lane merged with it; real NFL from nflverse 2021-25, PFR for pressure; FBS from cfbfastR 2022-24):

| | main | after | real |
|---|---|---|---|
| NFL sacks a team game | 1.90 | 2.39 | 2.41 |
| NFL sack rate a dropback | 5.2% | 6.6% | 6.8% |
| NFL pressure rate (PFR's count) | 23.3% | 23.8% | 22.2% |
| NFL sacks a pressure | 0.216 | 0.266 | 0.283 |
| NFL time to throw | 2.48 s | 2.46 s | 2.71 s |
| NFL points a team game / sd | 23.6 / 10.71 | 23.1 / 10.77 | 22.5 / 9.9 |
| NFL completion % / yards an attempt | 66.4% / 7.32 | 66.5% / 7.34 | 64.5% / 7.05 |
| NFL interceptions a team game | 0.73 | 0.71 | 0.76 |
| NFL scramble rate | 4.6% | 4.3% | 4.9% |
| FBS sacks a team game (seed 82) | 2.11 | 2.20 | 2.00 |
| FBS points a team game | 22.9 | 22.6 | 27.0 |
| High school sacks / sack rate / points (seed 82) | 2.39 / 10.2% / 17.5 | 2.46 / 10.6% / 17.4 | |

The A/B rows at the NFL (`ab_check`, 4,000 games an arm, seed 82, on the lane before main was merged in, the arm row after; pure moves the body or drill with ratings fixed, against real held equal on draft slot; as recruited uses the generator's athletes at the same overall, against real raw):

| Bug | What moves | Stat | main | after | real (band) |
|---|---|---|---|---|---|
| 286 | split, 0.1 s slower | pressures a defense snap, pure / recruited | -1.68 / -2.01 | -0.89 / -1.16 | +0.49 (-0.01 to +0.99) |
| 286 | forty, 0.1 s slower | pressures a defense snap, pure | – | -0.11 | -0.04 (-0.15 to +0.08) |
| 286 | weight, 10 lb | pressures a defense snap, pure / recruited | -0.41 / – | -0.37 / -0.23 | -0.02 (-0.13 to +0.08) |
| 328 | 3-cone, 0.1 s slower | pressures a defense snap, pure | -0.45 | -0.31 | -0.03 (-0.11 to +0.06) |
| 328 | 3-cone, 0.1 s slower | sacks a pressure, pure (E1) | +0.25 | -0.63 | -0.40 (-0.82 to +0.02) |
| 328 | shuttle, 0.1 s slower | pressures a defense snap, pure | -0.52 | -0.48 | -0.17 (-0.32 to -0.02) |
| 330 | height, an inch | pressures a defense snap, recruited | -0.46 | -0.16 | +0.02 (flat) |
| 330 | arm, an inch | pressures a defense snap, recruited | -0.26 | -0.06 | flat |
| E2 | arm, an inch | sacks a pressure / run tackles a defense snap, pure | +0.01 / +0.10 | +0.68 / +0.20 | +0.65 (-0.24 to +1.54) / +0.08 (-0.04 to +0.21) |
| E1 | height, an inch | pressures a defense snap, pure | – | +0.01 | -0.03 (flat) |

The combination grids (2,000 games a cell): a heavy edge's arms now add sacks a pressure (20.5% to 24.4% across the arm rows at 285 lb) and a light one's stay flat (E2); five more bench reps add pressures for a heavy edge (6.73% to 7.56% at 285 lb) and not for a light one (10.02% to 9.56% at 243 lb), the real shape (E3). Every row in the table is at or closer to real than main; the ones that got worse are below. The main rows are c0bc6f9, except the arm row's (the research run on 1831b94).

Still off:

- **The split, weight, 3-cone and shuttle still move pressures 2-4 times real** (bugs 286 and 328). Each fix that closed them took the quick, light edges out of the top seasons, which Cain's rule puts first. The cause is wider: an edge's skills (Finesse Rush, Hand Use) move his pressures less than his athleticism, and a speed win needs the feet because NFL passers throw quickly (2.46 s a dropback, real 2.71). A rush craft lane would weigh the skills up and the feet down together, holding the freak check.
- **The best sack seasons reach 27-29** (real 22.5 and 23; the top 1% of seasons 22 sacks), most of them heavy power rushers. Sacks a team game at the NFL sit at real, so the tail, not the level, is long. The synthetic league plays fixed depth charts, with no rotation by body.
- **Tall edges are under their share of the top seasons** (5-12% of the top, 19-21% of all; real 21% of 29%; main 11-12%).
- **As recruited, a long-armed edge's pressures end in a sack too often** (+2.2 pts of sacks a pressure an inch, real raw +0.66), though pure arms are at real (+0.68, real +0.65).
- **FBS sacks rose to 2.20 a team game** (main 2.11, real 2.00): the finish reads FBS edges' Tackling and Pursuit, about 54, a little over 60 for the best.

## Ratings that move their own stats: the rest of minor bug 283 (2026-11-08)

Minor bug 283 (from the A/B tests' spread check): on main Decisions, Football IQ and Play Recognition moved picks and sacks 0.01 to 0.12 of real starters' spread and EPA not at all (Decisions the wrong way), Short Accuracy moved completion 0.05, and Man and Zone Coverage moved the catch rate allowed 0.01 to 0.09. Growth from results reads those stats, so it barely rewarded passers, receivers and the secondary. Every size below is proposed, not confirmed, fitted with `ab_check` (NFL, +15 against -15, 2,000 games an arm, real held equal) to land in the agreed band of 0.1 to 1.0 of the spread, with `match_check` holding each level's totals.

- **The typical starter.** Every new slope sits about his level's typical starter at his position, the mean of the men the depth charts start on the 64 synthetic teams (`synthetic::typical_starter`). Starters rate above the dressed average: about the dressed man the steeper slopes lifted every starting secondary and passer, and picks ran 2.6% a throw against 2.3%.
- **Decisions** (`DECISIONS_TRUST`, `READ_DECIDE`, `MISREAD_EYES`, `MISREAD_LANE`, `PLACE_READ`, `GIVE_UP_DECIDE`, `THROW_AWAY_DECIDE`): his read of the field is 0.7 Decisions and 0.3 Play Recognition (half each before). A poor reader sees more room than there is: he misses the man sitting in his lane (1 yard of window a 10 points) and the defender reading his eyes (0.3 s); a good one sees it as it is and so trusts a tighter window (0.06 less room a point; the old +0.02, more room for the better man, made him throw short and hold it into sacks, and EPA a dropback fell). With a defender within 4 yards of the spot a good reader puts the ball away from him (1 yard a 10 points). His clock no longer shortens with Decisions within a level.
- **Football IQ** (`BLITZ_ID`, `BLITZ_ID_LOOK`, `GIVE_UP_IQ`): a passer who knows the pressure sees the free rusher 0.1 s sooner a 10 points (0.6 Football IQ, 0.4 Play Recognition) and gets rid of it.
- **Accuracy** (`AIM_SHORT`, `AIM_MEDIUM`, `AIM_DEEP`, `AIM_BASE`): the miss falls 2%, 2.5% and 1.4% a point of accuracy about the typical starter at short, medium and deep depth (main had 0.5% a point), sized on real starters' spread by depth (nflverse 2023-24: completion under 10 air yards p5 68.7% to p95 77.8%, 10-19 45.5% to 65.0%, 20+ 26.9% to 49.3%). A wider spread of passers costs more catches than it gains, so the base came down from 1.3 to 1.25 to hold each level's completion.
- **Coverage** (`MAN_LAG_MC`, `ZONE_LAG`, `BALL_COVER`, `MAN_FEEL`, `BREAKUP_COVER`): the man defender's lag is 0.03 s a point of Man Coverage (0.02 before, with no 0.1 s floor); the zone defender's 0.012 s a point of his zone read (0.004, floor 0.1 to 0.05); a better cover man breaks on the ball 0.15 s sooner a 10 points of his job's coverage; a man defender with his back turned feels the throw 0.25 s before his man's hands go up (man coverage was completed 67-68% of the time in the NFL against about 57% real; now 66%, with the aim base holding the league); the breakup reads his job's coverage at 0.012 a point.
- **Where picks come from** (`FRONT_PICK` 0.27, was 0.22; `TIP_PICK` 0.065, was 0.13): FTN charting (NFL 2023-24) has 87% of picks on throws it calls interception-worthy, 53% of those picked and 0.33% of the rest. The engine picked a ball off a receiver's hands as often as one thrown to a defender in front of it, so a careful passer saved few picks.
- **The coverage read in growth from results** (`results::pass_trials`): the catch on his man against what an open man catches at that depth and pressure; it was against the receiver's separation, which leaves out most of what coverage does.

The A/B rows after the fix (NFL, +15 against -15, 2,000 games an arm; the share is of real starters' p5 to p95):

| Rating | Stat | Main | Now |
|---|---|---|---|
| Decisions | picks an attempt | -0.15 pts (0.12) | -0.32 pts (0.26) |
| Decisions | sacks a dropback | +0.17 pts (the wrong way) | -0.22 pts (0.04, the right way) |
| Decisions | EPA a dropback | -0.008 | +0.021 |
| Decisions | completion / intended air yards | | +1.65 pts / -0.56 |
| Play Recognition | picks an attempt / EPA | -0.04 (no clear effect) / +0.005 | -0.49 pts (0.39) / +0.011 |
| Football IQ | sacks a dropback | +0.05 (no clear effect) | -0.89 pts (0.15) |
| Short Accuracy | completion (under 10 air yards) | 0.05 of the spread | +3.19 pts (0.40); +4.71 under 10 air yards |
| Medium Accuracy | completion (10-19) | | +4.25 pts (0.54); +5.26 at 10-19 |
| Deep Accuracy | completion (20+) | | +3.38 pts (0.43); +11.8 at 20+ |
| Corner Man Coverage | catch rate / yards a target allowed | 0.01-0.09 | -2.31 pts (0.15) / -0.38 (0.23) |
| Corner Zone Coverage | catch rate / yards a target allowed | | -8.99 pts (0.58) / -1.33 (0.79) |
| Linebacker Zone Coverage | catch rate allowed | | -6.48 pts (0.50) |
| Safety Zone Coverage | catch rate allowed | | -8.11 pts (0.62) |

League totals on synthetic rosters (`match_check`, 1,000 games a level; main in brackets): NFL 24.9 points (25.0), completion 65.2% (65.6%, real 64.5%), 7.33 yards an attempt (7.33), picks 2.3% (2.3%), dropback EPA 0.098 (0.103); FBS 22.6 (22.5), 60.5% (60.4%, real 60.6%), 7.28 (7.33), 1.9% (2.0%); high school 18.0 (17.2), 55.7% (56.0%), 6.45 (6.42), 1.6% (1.9%).


Growth from results after the refit (`results_fit -- 2000`, then `results_ab 20000 1`):

- **Reads on** (`Fit::on`: b > 0 with t of 4 or more): high school 17 reads, FBS 20, FCS 20, small colleges 20, the NFL 19 (13, 13, 17, 13 and 14 before), on 31 to 35 read-and-position rows a level (27 to 31). Rows near t 4 flip between refits: refit again after merging main (the NFL clock of minor bug 254 and the kickers' season form of 334 changed every game), the NFL's linebacker zone coverage (t 4.7 to 3.5), receivers' separation and high school backs' ball security went off and the NFL's backs' hands and tight ends' separation came on. The passers' intermediate, deep and pressure reads and the man and zone coverage reads came on.
- **Regulars in the fit** are men who play half their side's snaps a game or more, with the snaps a side counted from the team's own plays. It was 0.6 of a full-season starter's snaps a game, which no FCS or small-college back reached, so their reads had no spread among regulars and stayed off.
- **What results do to growth** (`results_ab 20000 1`, arm B, results on top of the dice and pulling down, against the game reps lane's run on main): results move a season's overall by 0.07 points sd (0.07); a starter's season moves his top skill +0.91 and -0.93 at the 90th and 10th percentiles (±0.93); a named big game in 15.5% of starter seasons (16.1%), a big season in 8.5% (8.6%), a rough one in 8.8% (8.7%); growth a season by level and role within 0.03 points of the dice alone (the 21 top programs' redshirt passers 0.12).
- **The NFL's short throws in `results_ab`**: completion over expected spreads 0.047 across NFL starter seasons (0.033 before, real 0.037) and carries 0.74 into the next season (0.48, real 0.23). In the fit's own regulars Short Accuracy alone spreads it 1.1 points (`b` 0.0016 a point times `sk` 6.6, real about 1.4 true), so the extra most likely comes from `results_ab`'s synthetic careers, whose starters spread their ratings wider than the depth charts' regulars, and from the other passing ratings that now move it too.
- **Practice**: neither `results_fit` nor `results_ab` plays practice; `Mode.practice` is read only by the world's practice days (`training/practice.rs`), so these numbers are the same with practice on or off. The practice world check runs on this code in the game reps lane.

Still off:

- **Linebacker Man Coverage** still shows no clear effect on the catch rate allowed (+0.09 pts, ±0.85), though it moves yards a target (-0.45, 0.24 of the spread): a linebacker's man is mostly a back or tight end on short routes, where the lag matters little.
- **Time to throw** barely moves with Decisions, Play Recognition or Football IQ (0.01-0.06 of the spread), and sacks with Decisions and Play Recognition (0.03-0.04).
- **High school picks** fell from 1.9% to 1.6%: high school balls off receivers' hands were a larger share of its picks, and those are rarer now. FBS picks were already short of real (0.60 a team game against 0.80).
- **The Decisions read in growth from results stays off**: picks are too rare for one season of games to show his rating (t under 2 at every level in `results_fit`), though the A/B shows it.

## The edge sack tail: shared sacks, an edge's weight, edges rotating (minor bugs 358-360, 2026-11-09)

The edge lane's freak check left three bugs: the best edge sack seasons ran past real's (358), tall edges looked under their share of the top seasons (359), and as recruited a long-armed edge's pressures ended in a sack too often (360). All three are measured here on new seeds 86-95, pooled, since one seed's top seasons are a few men. Real numbers are PFR and nflverse snap counts, 2018-25, edges with 200+ defense snaps: 983 seasons from PFR's season totals, so a traded man's stints add up (the edge research split them by team, 978 seasons). Nothing caps a rating, a body or a season (Cain 2026-11-08: no caps on freaks). Every size is proposed, not confirmed.

What was wrong:

- **No sack was ever shared.** The second man had to be one of the tacklers within 1.1 yards, and none ever was: 0.0% of sacks were shared, real 10.8%. Every sack went whole to one man.
- **The best edges never came off.** AI and synthetic teams rotate the line only on fatigue. At the edge's 0.84 line and 60 rating points a unit of energy, the best edges played 88-90% of their team's snaps (real 90th percentile 77%). The top sack seasons were volume.
- **Heavy edges finished more of their pressures.** Pure, 10 lb added +0.53 pts of sacks a pressure; real held equal is -0.49. The best seasons (34-51 sacks) were 290 lb power rushers.
- **NFL sacks rose to 2.57 a team game** with the bug 283 merge (#287; real 2.41), the pocket collapsing on a quarterback who held on.
- **359 is seed noise.** Pooled over ten seeds, 6-5 and taller edges were at or over their share of the top pressure seasons (1.09 of their share, band 0.91-1.29). One seed's top 10% is about 100 seasons of a few dozen men.
- **360 is mostly the A/B tool.** `ab::recruit` never ran the world's weight program (`athlete::set_position`), so drawn edges played at their frame weight (sd 30 lb), and the long-armed ones were heavy men. Placing them alone takes the row from +1.98 to +1.57 pts an inch (in the real band).

What changed (`game/scrimmage.rs`, `game/flow.rs`, `crates/afm-tools`):

- **Shared sacks.** The nearest other rusher within 2.25 yards of the quarterback when he goes down, free or winning his own pass block (w > 0.45), shares the sack, half to each (`SHARE_RADIUS`). The man who brings him down has the tackle and the second an assist, as the NFL and the NCAA keep it. 10.8% of sacks are shared (real 10.8%).
- **An edge's weight in his reach.** An edge's reach to the quarterback, free or on his block, loses 0.07 yards every 10 lb over the typical edge and gains it every 10 lb under (`WEIGHT_REACH`). It reads no strength or force: a heavy man still sheds and drives the pocket as before.
- **Edges rotate.** An edge's energy line is 0.90 and a coach gives up 600 rating points a unit of energy under it to spell him (`EDGE_LINE`, `EDGE_ALLOW`; DTs and every other spot unchanged). A user's own rotation plan (G15) still overrides it.
- **One backup spells one starter** (every position; found in this lane's review). A tired starter sat whenever any fresh backup was within his allowance, even one already named to spell another starter, so one fresh edge sat both tired starters and the fourth edge played, outside the second starter's allowance. At 60 points a unit it almost never happened; at the edge's 600 it did on most such snaps (`pick`).
- **Pocket feel.** A rusher winning his block within 3 yards weighs 1.0 (was 0.6) on the quarterback's feel for the pocket at full win (`POCKET_FEEL`), so he slides or gets rid of it a beat sooner.
- **The A/B tool** places each drawn athlete on his position's weight program, as the world places a class, and counts a shared sack as half in its sack rows (`ab::recruit`, `sacks_by`). As recruited rows from before this change were unplaced.
- **`freak_check`** takes a seed range (`freak_check 10 86-95`) and pools it. It gives bands by man (400 resamples; a world file's men count once however many seeds replay them), cuts 6-5 at 76.5 in (real heights are whole inches), counts PFR pressures (hurries, hits and every sack he is in on, a shared one whole to each man, as PFR does), and prints the sack tail, the share of team snaps the top seasons play and each line's rotation. `AFM_FREAK_DUMP=file.csv` writes every season.

The freak check (`freak_check 10 86-95`, ten seasons of 32 synthetic NFL teams a seed, every edge season with 200+ defense snaps, the 6-0, 240 lb freak left out of the pool; main is 1a0024d; real shares are of the top 10% by pressures a defense snap):

| | main | after | real |
|---|---|---|---|
| shared sacks | 0.0% | 10.8% | 10.8% |
| sacks a season, top 1% (band) | 24.0 (23-26) | 19.5 (18.5-20.5) | 17.5 |
| sacks a season, top 5% / 10% / 50% | 17.0 / 15.0 / 6.0 | 15.0 / 12.5 / 6.0 | 12.5 / 10.5 / 4.0 |
| seasons of 20+ sacks a 1,000 (band) | 29.0 (22.8-35.5) | 9.4 (6.2-12.6) | 2.0 |
| best season | 51 | 32.5 | 22.5 and 23 |
| sacks a defense snap, top 1% | 2.57% | 2.44% | 2.40% |
| team snaps the top 1% / 10% of sack seasons play | 90% / 86% | 79% / 76% | 77% / 74% |
| edges with 200+ snaps a team season; median / 90th percentile share | 3.21; 76% / 88% | 3.12; 69% / 79% | 3.84; 47% / 77% |
| under 250 lb, top 10% by pressures against their share (band) | 1.33 (0.74-1.99) | 1.27 (0.56-2.03) | 0.84 |
| arms under 33 in, the same | 0.96 (0.85-1.06) | 0.96 (0.87-1.05) | 1.58 |
| 6-5 or taller, the same | 1.09 (0.91-1.29) | 1.10 (0.90-1.30) | 0.69 |
| 290 lb or more, the same | 1.70 (1.24-2.13) | 1.62 (1.25-2.05) | (0.6% of real edges) |
| the freak's best season, by seed | top 0.1-5.1% | top 0.1-3.3% | |
| the freak's median season, by seed | top 2.4-24.4% | top 3.1-19.1% | |
| the freak's sacks a season | 7-37 | 4-38 | |
| the freak's share of team snaps | 82-94% | 68-80% | |

The freak still ranks with the best on every seed: he loses snaps to the rotation, not reach. Light edges hold more than their share of the top (1.27, real 0.84); short-armed ones sit just under theirs (0.96, band 0.87-1.05), short of real's 1.58, though the engine's arms run an inch short (bug 370), so its short-armed men are two thirds of all edges and real's a quarter.

2,000 `match_check` games a seed, NFL averaged over seeds 86-89 (real NFL from nflverse 2021-25, PFR for pressure; FBS from cfbfastR 2022-24):

| | main | after | real |
|---|---|---|---|
| NFL sacks a team game | 2.57 | 2.39 | 2.41 |
| NFL sack rate a dropback | 7.0% | 6.4% | 6.8% |
| NFL pressure rate (PFR's count) | 24.7% | 24.4% | 22.2% |
| NFL sacks a pressure | 0.274 | 0.258 | 0.283 |
| NFL time to throw | 2.46 s | 2.46 s | 2.71 s |
| NFL points a team game | 23.76 | 24.03 | 22.51 |
| NFL completion % / yards an attempt | 66.0% / 7.50 | 66.0% / 7.49 | 64.5% / 7.05 |
| NFL scramble rate | 4.4% | 4.5% | 4.9% |
| FBS sacks a team game (seed 86) | 2.49 | 2.34 | 2.00 |
| High school sacks / sack rate (seed 86) | 2.48 / 9.8% | 2.38 / 9.3% | |

The A/B rows (`ab_check`, NFL, 2,000 games an arm, seed 86; seed 87 in brackets; per 10 lb or an inch; pure moves the body with ratings fixed, against real held equal on draft slot; as recruited uses the generator's athletes at the same overall, against real raw; main's as recruited rows are unplaced):

| What moves | Stat | main | after | real (band) |
|---|---|---|---|---|
| weight, pure | sacks a pressure | +0.53 | -0.41 | -0.49 (-1.26 to +0.28) |
| weight, as recruited | sacks a pressure | +0.43 | -0.55 | -0.62 (-1.42 to +0.18) |
| weight, pure | sacks a defense snap | -0.05 | -0.10 | -0.03 (-0.06 to +0.01) |
| arm, as recruited | sacks a pressure | +1.98 | +1.03 [+1.00] | +0.66 (-0.28 to +1.61) |
| arm, as recruited | sacks a defense snap | +0.14 | +0.08 [+0.09] | 0.00 (-0.06 to +0.06) |
| arm, pure | sacks a pressure | +0.78 | +0.64 [+0.80] | +0.65 (-0.24 to +1.54) |
| height, as recruited | pressures a defense snap | -0.15 | -0.05 | +0.02 (-0.08 to +0.11) |

Still off:

- **The sack tail is shorter but still long**: the top 1% of edge seasons 19.5 sacks (real 17.5), 20+ sack seasons 9.4 a 1,000 (real 2), the best 32.5 (the same 295 lb power rusher three times on seed 94). What is left is pressure, not the finish. Over back-to-back seasons, an edge's pressures a snap go together at r 0.75 (real 0.46), while his sacks a pressure barely carry over, as real (r 0.04; real -0.02, band -0.10 to 0.07; main 0.08). So reading less of his own finish (`FINISH_OWN`) would not help. A rush craft lane (the rest of bugs 286 and 328) would weigh the rush skills up and the feet down and narrow the pressure spread.
- **Heavy and tall edges are still over their share of the top pressure seasons** (290 lb or more 1.62; 6-5 or taller 1.10, real 0.69). Part of it is the bodies: the generator's NFL edges are bigger than real ones (bug 369).
- **Teams still use too few edges** (3.12 with 200+ snaps a team season, real 3.84; the median one plays 69% of snaps, real 47%) and **DTs don't rotate** (2.99 a team season, the 90th percentile on 85% of snaps; real 4.35 and 70%; their line and allowance don't move it). Real staffs rotate the line by plan, AI teams here only on fatigue (bug 368).
- **Pure weight takes sacks a defense snap down too steeply** (-0.10 per 10 lb, real -0.03): the weight reach sits on top of bug 286's pressure slope (-0.40 pts of pressures a defense snap per 10 lb, real -0.02).
- **As recruited, long arms still add sacks a defense snap** (+0.08 to +0.09 an inch, real 0), and on seed 87 pressures too (+0.14, real -0.07).
- **NFL sacks a pressure fell to 0.258** (real 0.283) with the pocket feel, while sacks a team game are at real. NFL points rose 0.27 (24.03, real 22.51), about what 0.18 fewer sacks a game is worth.
- **FBS sacks are still over real** (2.34 a team game, real 2.00).

## Coach fire (2026-11-17)

Design: `docs/design/coach-fire.md` (Cain "yes", 2026-11-17). How it works: `docs/reference/how-it-works/match-engine.md`, "Fire on the field". Every size is Claude's, proposed, not confirmed, except the caps (3 points of power and effort, 3 of fine and thinking work, 3 tendency points).

What was wrong:

- **A talk lifted every rating the same way.** Toward his position's pitch, up to 1.5 points on all 67 ratings through his momentum edge; overshooting cost focus only. A fired-up safety threw no harder a hit and a fired-up quarterback threw no worse.
- **No habit read a talk.** Big hit, strip, downhill, contact, snap guessing and jumping routes were his own all game.
- **No targeting.** Every hit foul was unnecessary roughness at every level, and the league office's fines, hit suspensions and college ejections were draws of their own, not the games' flags.
- **A big hit did nothing** past the fumble and injury checks: no lift on the sideline, no flinch in the man hit.

What changed: see the how-it-works page. In short, each man has his own pitch (his position's, moved by his temper, competitiveness, Physicality, Composure and, for a quarterback, how much of a gunslinger and a scrambler he is); fire toward it is power and effort, past it costs fine and thinking work and pushes the fire habits; hits draw targeting by each level's rule and era, flagrant ones an ejection; the league office rules on the games' flags; a big hit, a forced fumble or a 4th down stop sparks the sideline and the man hit hears footsteps; the play log shows it.

The big hit's line and what fire does in a team game (`fire_check hits`, 400 games a level, seed 1, the engine's test rosters, each head coach's own read):

| A team game | NFL | FBS | FCS | High school |
|---|---|---|---|---|
| hardest hit on a play, 97th percentile (the big hit's line) | 2,111 | 1,804 | 1,677 | 1,358 |
| big hits | 1.40 | 1.53 | 1.51 | 1.26 |
| sparks (a big hit, a forced fumble or a 4th down stop) | 2.53 | 2.76 | 2.69 | 2.13 |
| unnecessary roughness (accepted) | 0.432 | 0.449 | 0.422 | 0.371 |
| targeting (accepted) | the NFL has none | 0.040 | 0.036 | 0.030 |
| ejections | 0.009 | 0.058 | 0.054 | 0.007 |
| hot plays | 0.001 | 0.003 | 0.005 | 0.006 |
| heard footsteps | 0.004 | 0.006 | 0.001 | 0.003 |

Real: FBS targeting 0.031 to 0.043 a team game (cfbfastR 2022-2025); the NFL throws out about 6 men a season for hits (21 ejections of every kind in 2023, Football Zebras), 0.011 a team game, and the test league's 0.009 is about 5. College ejections are targeting's, nearly all of them since 2013, and a few flagrant hits. High school throws out only the flagrant ones.

League totals against main (`match_check`, 400 games a seed, seeds 1-12 at each level; the branch less main, with its standard error over the seeds). Penalties a team game hold: NFL +0.00 (± 0.04), FBS -0.03 (± 0.04), high school +0.02 (± 0.03). The hit fouls hold, the college ones now split into targeting and unnecessary roughness (FBS 0.488 together, main 0.485). What moved two standard errors or more, all within 5%:

| Level | Stat | main | after | real |
|---|---|---|---|---|
| NFL | completion % | 65.75 | 65.57 (± 0.07) | |
| NFL | points a team game | 24.67 | 24.48 (± 0.10) | 22.51 |
| FBS | pass interference | 0.287 | 0.275 (± 0.006) | |
| FBS | missed tackles a run | 0.174 | 0.171 (± 0.001) | |
| FBS | yards a carry | 4.82 | 4.79 (± 0.014) | 5.12 |
| high school | offside | 0.460 | 0.483 (± 0.012) | |
| high school | fumbles lost | 0.247 | 0.257 (± 0.005) | |
| high school | sacks | 2.19 | 2.23 (± 0.02) | |

Why: the AI head coaches' own read is mostly the captain's calm word (44 of 64 NFL pregame picks on seed 1, `fire_check talks`; main's worth would pick it 64 times). On main it settled the room and lifted every rating a little; now its settling reads only as composure, and its fire works by the kind of work, which for a calm word means a little less power under his pitch and a little fine work back past it. The habits it moves are under a tenth of a point.

Speed (`match_check`, 400 games, a quiet machine, main and after in turn): NFL 9.3 ms a game on main against 9.5 after (five runs each, 8.7 to 10.9), FBS 9.7 against 9.5 and high school 6.7 against 6.7 (two runs each). No change past the noise.

The rooms (`fire_check run`, 2,720 NFL games an arm, seed 1). Team 0's best safety made a Branch type (temper 3, competitiveness 17, Physicality 17 on the level's scale, Big hit 16: his pitch 0.62) and its best quarterback a calm pocket passer (temper 16, rarely scrambles, takes the checkdown: his pitch 0.38), both under each prototype coach's room talk (pregame, halftime, and the tone of his sideline words), against his own coach's read (the captain's calm word before the game). First what each pregame talk leaves on them, in a regular-season game (the test league's stakes 0.25) and a playoff game (stakes 0.8):

| Room (the talk) | safety: fire, power, fine, habits | passer: fire, power, fine, habits | playoffs: safety fine, habits; passer fine |
|---|---|---|---|
| own read (one draw of it: the captain, calm, us) | 0.50 to 0.49, -0.2, 0, -0.1 | 0.34 to 0.32, -0.2, 0, -0.1 | +0.2, -0.2; +0.2 |
| Firebreather (challenge, fiery, them) | 0.50 to 0.63, +1.5, -0.1, +0.6 | 0.34 to 0.45, +0.5, -0.8, +1.0 | -1.5, +1.5 (heat +0.25); -1.3 |
| Drill Sergeant (criticize, fiery, us) | 0.50 to 0.58, +0.9, 0, +0.3 | 0.34 to 0.41, +0.5, -0.3, +0.5 | -0.9, +0.9; -0.8 |
| Needler (criticize, firm, us) | 0.50 to 0.53, +0.4, 0, +0.1 | 0.34 to 0.38, +0.4, 0, +0.2 | -0.4, +0.4; -0.4 |
| CEO (straight, calm, the plan) | 0.50 to 0.47, -0.4, 0, -0.1 | 0.34 to 0.30, -0.4, 0, -0.2 | +0.4, -0.4; +0.4 |
| Players' Coach (praise, warm, us) | 0.50 to 0.51, +0.1, 0, 0 | 0.34 to 0.35, +0.1, 0, 0 | -0.1, +0.1; -0.1 |
| Preacher (straight, fiery, the stakes) | 0.50 to 0.58, +1.0, 0, +0.3 | 0.34 to 0.41, +0.5, -0.4, +0.6 | -1.1, +1.1 (heat +0.15); -1.0 |
| Professor (ask, calm, the plan) | 0.50 to 0.46, -0.5, 0, -0.2 | 0.34 to 0.29, -0.6, 0, -0.2 | +0.5, -0.5; +0.6 |

In the playoffs both come in past their pitch (the safety at 0.70, the passer at 0.56), so a talk gives no power and fiery words only cost fine work.

What it did, each room less his own coach's read (± the standard error over 40 blocks of games):

| Room | S big hits a game | S forced fumbles a tackle | S personal fouls a game | S hot plays a game | QB completion over expected, points | QB interceptions a dropback |
|---|---|---|---|---|---|---|
| own read, regular season | 1.43 | 0.0198 | 0.156 | 0 | +8.0 | 0.0146 |
| Firebreather | +0.17 (0.03) | +0.0033 (0.0015) | +0.000 (0.012) | 0.11 | +0.17 (0.21) | +0.0001 (0.0006) |
| Drill Sergeant | +0.08 (0.03) | +0.0021 (0.0014) | -0.011 (0.010) | 0.09 | +0.44 (0.20) | 0.0000 (0.0006) |
| Preacher | +0.09 (0.03) | +0.0023 (0.0015) | -0.007 (0.012) | 0.08 | +0.40 (0.20) | +0.0003 (0.0006) |
| CEO | -0.05 (0.04) | +0.0023 (0.0014) | -0.027 (0.012) | 0 | +0.04 (0.20) | +0.0006 (0.0006) |
| Professor | -0.08 (0.04) | -0.0008 (0.0012) | -0.009 (0.012) | 0 | +0.10 (0.24) | +0.0001 (0.0006) |
| own read, playoffs | 1.37 | 0.0207 | 0.147 | 0 | +8.4 | 0.0146 |
| Firebreather | +0.25 (0.03) | +0.0013 (0.0011) | +0.045 (0.009) | 0.82 | -0.47 (0.21) | +0.0008 (0.0007) |
| Drill Sergeant | +0.11 (0.03) | +0.0023 (0.0012) | +0.020 (0.010) | 0.11 | -0.16 (0.23) | +0.0002 (0.0005) |
| Preacher | +0.18 (0.03) | +0.0030 (0.0012) | +0.022 (0.010) | 0.26 | -0.44 (0.23) | -0.0002 (0.0006) |
| CEO | -0.02 (0.03) | -0.0013 (0.0010) | -0.003 (0.010) | 0 | +0.09 (0.17) | +0.0004 (0.0005) |
| Professor | -0.04 (0.03) | -0.0011 (0.0010) | -0.006 (0.011) | 0 | +0.22 (0.24) | +0.0002 (0.0006) |

(The Needler and the Players' Coach move little either way; the full tables print with the tool.) Why:

- **The safety's big hits follow the room's fire**: +12% under a Firebreather in a regular-season game, +19% in a playoff game, -4% to -5% under the calm rooms. Fire toward his pitch is Strength, Lower Body and Pursuit, 1% on his hit a point, and his Big hit habit; a calm room takes him further under his pitch.
- **His flags come with the hits only past his pitch.** In a regular-season game he comes in at 0.50, under his 0.62, and the Firebreather takes him just to it: his Discipline holds (fine work -0.1) and the heat is small, so his personal fouls hold (+0.000 ± 0.012). In a playoff game he comes in at 0.70; the Firebreather takes him to 0.83: 1.5 points off his Discipline and reads, 1.5 on the habits, heat +0.25: personal fouls +30%, big hits +19%, 0.8 plays a game his fire carried, 0.4 more points a game against his team. That is the Brian Branch pattern the design asked for.
- **His forced fumbles rise with the fiery rooms** (+17% under a Firebreather in a regular-season game, ± 8%): the Strip habit and the harder hit.
- **The pocket passer pays only in big games.** In a regular-season game he comes in at 0.34, under his 0.38 pitch; a Firebreather costs him 0.8 points of accuracy, Touch, Decisions and reads, but lifts the power of his line and receivers (who come in under theirs), and his completion over expected doesn't fall. In a playoff game the whole room comes in past its pitch: a Firebreather costs him 1.3 more points and his teammates gain nothing, and he loses half a point of completion over expected (-0.47 ± 0.21; the Preacher -0.44 ± 0.23); his interceptions may rise a little (+5% ± 5%). The calm rooms give him back +0.1 to +0.2.
- **The CEO's room took 17% off the safety's personal fouls in a regular-season game** (± 8%), where its pregame talk alone would take well under 1% (his habits 0.06 points lower than under his own coach's read). The Professor's calm room took 5% (± 8%). Not settled: part of it may be the calm tone of the room's sideline words and halftime talk, part noise.

Still off:

- **The AI coaches rarely fire up a room.** Their own read is mostly the captain's calm word, before and after this lane, so in the world fire's big effects come from your talks and the rare fiery AI room. Whether the AI should read its own players' pitches better is the talks' job, not this lane's.
- **Flags on a sliding quarterback are a guess** (`SLIDE_HIT`): no real count was found. About 1 in 100 of the test league's slides draws one.
- **"Heard footsteps" is a rare tag** (0.001 to 0.006 a team game): the flinch reads on Contact on every play, but the tag needs a short-armed ball over the middle with a man close, and big hits on receivers are few.
- **The NFL throws out about 5 men a season for hits on the test rosters**, against about 6 real (small counts: 7 ejections in 800 team games).

`fire_check` (`crates/afm-tools/src/bin/fire_check.rs`) is the lane's tool: `hits <level>` for the big hit's line and the fire counts a team game, `talks <level>` for the AI coaches' picks against main's, `run [games] [seed] [stakes]` for the rooms (stakes 0.8 for a playoff game).

## A blocked man is out of the play (scheme-fit lane SF3, 2026-11-17)

Scheme-fit item 7 (`docs/design/scheme-fit.md`), in Cain's words: a defender in a block he isn't winning can't tackle the runner, and he has to shed first; one who is beating his blocker can still get an arm out. The target: the league's run game stays where it is, and the line explains real-sized gaps (backs' yards before contact have a true spread of 0.36 a carry between seasons, PFR 2018-25).

What was wrong on main (4,000 `match_check nfl` games on seed 11, `AFM_RUNDIAG=1`, with a count at the hit):

- **The shed as the runner comes read nobody.** Every run block that wasn't sealed got the same pull off it as the runner came within 2.2 yards (`SHED_RUNNER`, minor bugs 164 and 253), whoever was winning it, so a back met the same men at the same spots behind a good line or a bad one. The line's edge (its five blockers' Run Blocking less the other side's four linemen's Block Shedding) moved the backs' yards before contact 0.030 a carry a point, and on world seed 1's NFL clubs nothing (-0.006).
- **The tackle from a block was small.** The audit's 34% (first tackles by a man in a block he never shed) reads the play log's block records, which also hold a man put down earlier or picked up by a blocker after his hit (minor bug 404). Counted at the hit, 4.9% of the hits that start the tackle on designed runs came from a man on a block, and 0.6% from one he wasn't winning (the gate let him try from w -0.2, at 0.25 + 0.3 × w).
- **The runner counted a sealed man as 0.15 of a free one** when he picked his lane.

What changed (`game/sim.rs`; every size proposed, not confirmed):

- **The shed reads the block** (`update_blocks`, `SHED_EDGE`): the pull is scaled by e^(edge / 9), the edge being the defender's lead in rating points (his shed power against the blocker's block power, a helper's and both men's flaws and tools). A man beating his blocker by 9 points comes off 2.7 times as hard as one on an even block, and one losing by 9 at 0.37 of it.
- **The tackle** (`tackles`, `ARM_OUT`): a man on a block tries only while he is winning it (w above 0), with chance 0.55 × w a try (the same 0.55 at the shed as before), inside his reach less 0.55 yards.
- **The runner's read** (`run_carrier`): a man on a block counts as 0.5 × (w + 1) / 2 of a free man: nothing sealed, a quarter on an even block, a half at the shed (0.15 to 0.5 before).
- **Tools:** `match_check` prints who starts the tackle by his block at the hit (`sim::FIRST_TACKLE_BLOCK`), the line's edge against yards before contact and the backs' lasting spread before contact, and plays a world's 32 NFL clubs with `AFM_AB_WORLD`.

4,000 `match_check nfl` games, main and after (with the call table's run rows rebuilt, below), on the test rosters (seed 11) and on world seed 1's NFL clubs (seed 1):

| NFL | test, main | test, after | world, main | world, after | real |
|---|---|---|---|---|---|
| yards a carry | 4.45 | 4.50 | 4.46 | 4.50 | 4.50 |
| designed runs for 0 or less | 18.4% | 19.3% | 18.7% | 19.5% | 18.2% |
| inside / outside the tackles | 3.56 / 5.38 | 3.72 / 5.39 | 3.60 / 6.26 | 3.71 / 6.32 | 4.05 / 5.06 |
| 8 or more in the box, a carry | 3.18 | 3.20 | 3.26 | 3.27 | 3.40 |
| first tackle: line / linebackers / backs | 43.8 / 21.0 / 35.2% | 42.4 / 21.8 / 35.8% | 42.9 / 22.3 / 34.8% | 40.8 / 24.0 / 35.2% | 43.1 / 27.8 / 29.1% |
| the hit that starts the tackle from a man on a block | 4.9% | 2.2% | 3.9% | 1.8% | – |
| the same, a block he isn't winning | 0.6% | 0 | 0.5% | 0 | – |
| backs' yards before contact a point of the line's edge | 0.030 | 0.065 | -0.006 | 0.020 | – |
| backs' yards before contact, lasting spread (204-carry blocks) | 0.34 | 0.45 | 0.32 | 0.37 | 0.36 between seasons |
| backs' yards a carry, sd over 204-carry blocks | 0.56 | 0.68 | 0.51 | 0.56 | 0.57 |
| missed tackles a run | 0.143 | 0.125 | 0.118 | 0.105 | – |
| points a team game | 24.0 | 24.3 | 26.2 | 26.2 | 22.5 |
| points sd / games within 8 | 11.1 / 41.5% | 11.4 / 40.1% | 9.6 / 56.6% | 9.9 / 52.9% | 9.9 / 53.5% |

The two rows on the hit that starts the tackle are counted after coach fire (main 010fd83 against this branch merged with it), on every designed run but an RPO pulled into a throw. A code review found the first count read the run's pace, so it took in pulled RPOs and left out tosses, jet sweeps, sneaks and options; it gave 4.9% to 2.1% on the test rosters and 3.9% to 1.7% on the world's clubs. The count changes no play: the same 4,000 games come out the same, row for row. The lasting spread is the covariance of one back's consecutive 204-carry blocks (a real season's carries), so it is what stays with a back and his team. In `match_check` the rosters never change, so it keeps all of the line's part, where real seasons lose some to line changes. The test rosters are wider apart than the world's clubs or the real NFL (points sd 11.1, real 9.9; 41.5% of games within 8, real 53.5%), so the line's new reach shows larger there: their backs' yards a carry spread 0.68 over 204-carry blocks against real 0.57, where the world's clubs come to 0.56. On the world's clubs yards a carry, inside runs, the 8-man box, the linebackers' first tackles, both spreads and close games move toward real; the stuffs, outside runs (a little) and the line's share of first tackles move away. Fewer missed tackles: a man on a block tried with an arm (the miss logit +1.5), and those tries are fewer.

FBS (4,000 games, seed 11): yards a carry 4.76 to 4.76 (real 5.12), runs for 0 or less 16.4% to 17.9%, first tackles by the line 43.9% to 43.8%, the line's edge 0.037 to 0.080 a point, points 23.2 to 23.5 (real 27.0), points sd 12.1 to 12.9 and games within 8 35.2% to 31.5%. High school (2,000 games): 5.74 to 5.77 a carry, the line's first tackles 44.2% to 44.8%, points 18.6 to 18.8, points sd 15.0 to 16.3. Kick and punt returns don't move (NFL 25.4 and 8.2 yards a return before, 25.4 and 8.3 after).

The coaches' call table (`call_values.csv`, `call_values 2000`, 2,000 snaps a cell): SF3 moves its run cells by -0.41 yards on average (spread 0.81, up to 3.2) and its pass cells by almost nothing (spread 0.06, up to 0.32), so its 15 run rows are rebuilt and the pass rows kept. The pass rows were already off on main: the tool's pass cells had drifted from the table by a spread of 3.3 yards a cell (up to 17.6), and in its two fixed teams Stick and Mesh into Cover1Robber come out at -9 and -13 (the table has +6 and +4) from a storm of picks that full games don't show (minor bug 405). The numbers above are on the new table; on the old one the league came out the same within noise (NFL test rosters 4.50 a carry and 24.1 points, world clubs 4.52 a carry).

Speed: the same within noise. `match_check nfl 4000` takes 29.6 s against 29.9 s on main after coach fire, and took 33.0 s against 32.0 s in a pair before it (two runs each, 4 cores, the same build).

Every table above is over main b9fa58d, before coach fire merged. On main 010fd83, with coach fire, the same runs move the same way: the NFL test rosters 4.42 to 4.52 a carry, the line's edge 0.030 to 0.065 a point, points 23.8 to 24.1; world seed 1's clubs 4.46 to 4.50 a carry (real 4.50), the backs' lasting spread before contact 0.28 to 0.38 (real 0.36), yards a carry sd over 204-carry blocks 0.49 to 0.57 (real 0.57), points sd 9.6 to 9.9 (real 9.9) and games within 8 55.7% to 54.1% (real 53.5%); FBS 4.76 a carry both, the line's edge 0.037 to 0.080, points 23.5 to 23.3; high school 5.70 to 5.76 a carry, points 18.2 to 19.0, points sd 14.8 to 16.4. The call table's cells come out the same on it, cell for cell. `fit_check` on FBS seed 151 again (3,000 games an arm, main 010fd83 against this branch merged with it): a lineman's Run Blocking -0.02 to +0.07 a carry on main (3 of 13 arms past the noise) and +0.01 to +0.15 after (11 of 13), a defensive tackle's Block Shedding -0.04 to +0.02 allowed (1 of 5) and -0.12 to -0.08 (5 of 5), an edge's -0.05 to -0.03 and -0.12 to -0.11, the Mike's -0.06 to -0.04 and -0.11 to -0.07, and the back's Speed +0.08 to +0.10 and +0.07 to +0.11 a carry.

`fit_check` (team 0 the typical team, each arm one change on the same games; FBS seed 151 at 3,000 games an arm and NFL seed 161 at 2,400; * past the noise), main and after:

| +15 on team 0 | FBS 151, main | FBS 151, after | NFL 161, main | NFL 161, after |
|---|---|---|---|---|
| a lineman's Run Blocking: yards a carry | -0.00 to +0.08 (5 of 13 arms*) | +0.02 to +0.14 (8 of 13*) | -0.02 to +0.04 (0 of 13*) | -0.01 to +0.15 (10 of 13*) |
| the same: run blocks won, of 100 | +7.3 to +11.6 | +8.4 to +16.9 | +6.0 to +10.3 | +5.9 to +14.1 |
| the same: team run success, points | -0.0 to +0.8 | +0.5 to +1.7 | -0.1 to +0.7 | +0.4 to +1.6 |
| a tight end's Run Blocking: yards a carry | +0.01 to +0.06 | +0.05 to +0.09 | +0.01 to +0.10 | +0.05 to +0.15 |
| a defensive tackle's Block Shedding: yards a carry allowed | -0.02 to -0.01 (0 of 5*) | -0.12 to -0.08 (5 of 5*) | -0.03 to +0.01 (0 of 5*) | -0.13 to -0.06 (5 of 5*) |
| the same: his first tackles on runs a season | -2.8 to +2.0 | -0.1 to +7.3 | -6.9 to +2.1 | -1.7 to +7.2 |
| an edge's Block Shedding: yards a carry allowed | -0.05 to -0.03 | -0.13 to -0.09 | -0.03 to -0.00 | -0.14 to -0.13 |
| the Mike's Block Shedding: yards a carry allowed | -0.05 to -0.04 | -0.11 to -0.09 | -0.07 to -0.05 | -0.10 to -0.07 |
| the back's Speed: yards a carry | +0.12 to +0.13 | +0.07 to +0.11 | +0.09 to +0.11 | +0.12 to +0.14 |
| the back's Ball Carrier Vision: yards a carry | +0.03 to +0.11 | +0.03 to +0.04 | +0.02 to +0.06 | +0.04 to +0.07 |

The arms are the center, left guard and left tackle in power gap, West Coast and wide zone, the right guard and right tackle in the first two, the tight ends in 12 and 13 personnel and West Coast, and the defensive tackles, edges and the Mike in the West Coast and 3-4 contexts (`fit_check list`). Points move a little: a lineman's +15 is -0.1 to +0.8 points a game (8 of 26 arms past the noise, 1 before) and a defensive tackle's -0.6 to -0.3 allowed (-0.5 to +0.1 before), since one man is a fifth of the line and runs are under half the snaps. The right tackle moves the least (-0.01 to +0.09 a carry). On FBS seed 171, the repeat seed, the same: a lineman's Run Blocking -0.03 to +0.07 a carry before (5 of 13 past the noise) and +0.02 to +0.15 after (10 of 13), a defensive tackle's Block Shedding -0.03 to -0.00 allowed (0 of 5) and -0.11 to -0.05 (5 of 5).

The back's own ratings keep what they had. Speed: +0.07 to +0.14 a carry. Ball Carrier Vision averages +0.04 a carry over its eight arms after (wide zone and power gap in four runs: FBS 151, FBS 171, NFL 161 and FBS 151 after coach fire) against +0.05 before, inside the noise: one arm's read carries about ±0.03 a carry, so two builds' reads of it differ by about 0.04. In wide zone it first read lower after (+0.04, -0.01 and +0.04 at FBS 151, FBS 171 and NFL 161, from +0.11, +0.06 and +0.06), but main itself fell from +0.11 to +0.03 at FBS 151 when coach fire merged, which doesn't touch the back's read, and on that main this change takes it to +0.06.

Tried and dropped (NFL test rosters, seed 11):

- **The tackle rule alone** (the gate and `ARM_OUT`): the line's edge 0.031 a point, 4.48 a carry. The stop came from the shed, not the tackle.
- **The gate and the runner's read:** 0.034 a point.
- **No floor on the pull** (2 × 0.9 × (0.5 + w)₊, so a man well behind on his block gets none): 5.03 a carry, linemen 33.9% of first tackles (real 43.1%).
- **An added drift of 0.2 × the edge while the runner is near:** 0.058 a point but 4.58 a carry and 2.54 yards before contact, so the league's run game moves.
- **e^(edge / 6) and e^(edge / 12):** 0.070 and 0.058 a point, 20.2% and 19.0% of runs for 0 or less; 9 sits between.
- **A blocker can't pick up a man already in the tackle** (minor bug 404): the play log's count falls from 37.5% to 24.8% and yards a carry rise to 4.47 with the pile and yards after contact, a change outside item 7, so it is left for its own call.

Still off: inside runs 0.3 a carry short and outside runs 0.3 long (world clubs 1.3), linebackers make too few first tackles, and runs of 20+ are half of real (item 9, lane SF9). Short yardage (item 8) is measured again after this lane, as its spec asks: team 0's backs convert 77-78% of designed runs on 3rd and 4th and 2 or less at the NFL (78-80% before) and 81-84% at FBS (82-84% before), against 67% real, and its quarterbacks 89-95% (real 79%), so it stays lane SF9's. Runs of 20+ don't move (NFL 0.9-1.2% of the backs' carries, FBS 1.3-1.8%, real 2.2%).

## Play action that works (scheme-fit lane SF1, item 3, 2026-11-17)

Design: `docs/design/scheme-fit.md`, item 3 (Cain: yes, "result breaking"), and his pick on 2026-11-17, "fake on any dropback play": play action is a tag any dropback concept can carry, and the coaches' table learns each concept with and without it. How it works: `docs/reference/how-it-works/match-engine.md`, "Play action", and `play-calling.md`, "Play action". Real numbers are FTN charting with nflverse play-by-play, RPOs left out: 2024 for the spec's targets (screens left out too), 2022-2025 for the call habit (screens in, since a screen can come off a fake). Every size is Claude's, proposed, not confirmed.

What was wrong:

- **The fake was a straight drop with a pause.** The quarterback backed up for 0.35 s more with no back meeting him, the line pass-set as on any dropback, and the fake reached only the coverage: later reads, and until they read it, men not in a deep zone or on a wide receiver stepped toward the line. The rush came at once into a quarterback who was later to his set: 22.9% of play-action dropbacks were sacks (real 6.6%), worth -0.09 EPA a dropback on the test league (real +0.15).
- **The boot ran into the rush.** It kept the standard protection, and the quarterback set 7 yards toward the strength side, where his tackle was already blocking: 47% sacked (real 5.7% on boots and rollouts).
- **Only three concepts could fake** (the shot, the dig and the boot), so play action was 7.9% of NFL dropbacks (real 21.2%).

What changed (`game/scrimmage.rs`, `game/sim.rs`, `game/calling.rs`, `game/playbook.rs`, `game/sheet.rs`, `game/tables.rs`, `game/flow.rs`; the panel in `afm-game`, `lane_gameplan/likeus.rs`):

- **The tag and the call.** Any dropback concept but the RPO halves and the three play-action concepts (which fake already) can carry the fake; screens too (27% of real screens come off a fake). It is decided with the throw, before the look, so a fake comes from the run's own personnel and backfield. How often: the league's real habit for the down, distance, score, clock and field (`data/real-world/nfl_play_action_rate.csv`), leaned on the odds by the scheme (wide zone 1.75, wing-T 1.6, power 1.35, West Coast 1.0, spread option 0.7, Air Raid 0.55), by your word to a player promised play action, and by what the coaches' table says the fake adds against the looks this defense shows. A throw off the call sheet fakes at the caller's own habit too: it keeps the fake of his own call when that was a throw, and when it replaces a run (which has no fake to keep) it fakes at his habit for the spot.
- **The fake.** The back takes the run's path through the mesh (0.8 s under center, 0.7 in the pistol, 0.6 in the shotgun), the quarterback rides it and pulls it, and the back carries the fake 0.45 s into the line before he takes his job in the pass. The quarterback finishes his drop at least 2 yards past the mesh (a deeper drop keeps its depth, and the boot keeps its own spot), 0.45 s later to his set than on a straight drop.
- **Who it holds.** Everyone who keys the backfield plays the run he is shown until he reads the pass (his own read time, which the fake already lengthens by his Play Recognition, Discipline and Downhill habit and the passer's Ball Handling against his level's typical starter): the rushers, the zone defenders, and a man on a tight end or a back. A man on a wide receiver has his eyes on his man. A fooled rusher in a block plays it as the run: he gets no nearer the passer, and the block stands where it is until he reads the pass, or the quarterback throws or tucks it and runs (rolling out on the boot or breaking the pocket to throw on the move does not end it).
- **The protection.** With the back in it, a half-slide (not on the boot, below, where the back has no man of his own and blocks only a free rusher who comes into him on his path): the guard and tackle on the side he fakes to block the down linemen over them (the tackle has the end), the center and the far side slide a gap away, and the back has the first rusher left on his side from the center out. He blocks a free rusher who comes into him on his path, and from the mesh on his own man if that man comes and he sees him (Football IQ and Pass Blocking against the blitzer's Blitzing); otherwise he carries the fake, then blocks or releases. When he runs a route, the standard protection.
- **The hot throw.** Once his eyes are back from the mesh (0.2 s after it under center, at once in the shotgun and the pistol), a passer with a free man coming can throw before he sets, by his own poise and blitz reading (`urgent`); the hot read's window runs the fake's 0.45 s later. A throw he decides on in that window goes when it would have, even if the blitzer is picked up first; outside it he reads nothing new until a tenth of a second after he sets, and the back still carrying out the fake is nobody's target.
- **The boot.** The guard on the boot side pulls to the end man there; the blocker who had the end takes the guard's man, and if nobody had the end, a lineman with no man of his own blocks back on the guard's man. The quarterback keeps rolling outside and back toward the line at 0.48 of his top speed (about 4.8 yd/s, 10.7 mph, for the NFL's typical starter; a quick passer gets outside sooner), eyes downfield, and a back lined up on the ball runs his route to the side the quarterback rolls to (the back who fakes the run away from the roll runs his later, from where the fake left him).
- **The coaches' table** (`crates/afm-engine/data/call_values.csv`, `call_values`) has a play-action row for every concept that can carry the fake, played from the shotgun against every look, and a caller weighs the fake against the look as he weighs the concept. The straight pass rows stay as on main (minor bug 405: the tool's fixed teams throw a storm of picks into the Cover 1 robber), so each faked row is its straight row plus what the fake changes on the same snaps (`tools/call_values/compose.py`); the three play-action concepts' rows and the runs are rebuilt on this lane (it moves only the draw, by the fake's sell, at most 0.28 a cell).
- **The line chart** counts a back who blocks after the fake in the protection, with the man the half-slide gave him, so a seven-man play-action protection is charted as Max (the usage promises read it).
- **The game plan's "teams like us" panel** reads a faked play as the play the sheet names ("play-action flood" is flood), and a play with motion too.

The league's play action (`match_check`, 2,000 games at each level, seed 1, every team on its own staff, college and NFL teams on their own call sheet too (high school staffs draft none, as in the game); main with SF3 merged, then after; real is the NFL, and no college charting is on file):

| | NFL | FBS | High school | Real |
|---|---|---|---|---|
| play action, share of dropbacks | 7.9% to 19.6% | 7.1% to 19.9% | 8.7% to 19.9% | 21.2% |
| the same on 1st, 2nd, and 3rd and 4th down | 10.5%, 6.6%, 5.8% to 31.2%, 20.0%, 3.3% | 9.3%, 6.2%, 5.1% to 31.7%, 19.5%, 2.8% | 11.9%, 6.5%, 6.7% to 32.5%, 19.2%, 2.7% | 36.6%, 20.8%, 4.2% |
| EPA a dropback, play action | -0.086 to +0.189 | -0.281 to +0.070 | -0.423 to +0.066 | +0.149 |
| EPA a dropback, straight drops | +0.119 to +0.112 | +0.071 to +0.056 | -0.081 to -0.103 | +0.031 |
| play action over a straight drop | -0.205 to +0.077 | -0.352 to +0.014 | -0.342 to +0.169 | +0.118 |
| sacked, play action | 22.9% to 4.7% | 20.7% to 5.6% | 27.2% to 5.7% | 6.6% |
| sacked, straight drops | 4.7% to 4.8% | 5.7% to 5.9% | 8.9% to 9.1% | 7.3% |
| sacked, the boot | 46.9% to 2.4% | 44.8% to 2.6% | 39.5% to 2.0% | 5.7% (every play action that leaves the pocket) |
| the boot, share of play action | 35.1% to 2.1% | 29.2% to 1.9% | 41.5% to 5.5% | (36% leave the pocket, flushed passers too) |
| pressured, play action against straight drops | 2.00 to 0.93 times | 1.75 to 0.92 | 1.56 to 0.75 | 1.19 |
| time to throw, play action and straight (s) | 3.20 and 2.41 to 2.82 and 2.42 | 3.33 and 2.50 to 2.93 and 2.52 | 3.58 and 2.58 to 3.10 and 2.58 | 3.15 and 2.66 |
| yards an attempt, play action and straight | 11.02 and 7.38 to 8.59 and 7.28 | 9.41 and 7.18 to 7.95 and 7.13 | 8.92 and 6.44 to 7.77 and 6.34 | 8.60 and 6.93 |
| air yards a play-action attempt | 20.1 to 11.8 | 20.4 to 11.3 | 18.3 to 10.0 | 9.2 |
| play-action throws behind the line | 0.4% to 5.7% | 0.8% to 6.5% | 1.1% to 6.5% | 14.8% |
| play action from under center | 40.7% to 56.9% | 37.6% to 50.2% | 59.9% to 65.7% | 58.1% |
| straight drops from under center | 26.6% to 25.8% | 20.5% to 20.8% | 26.3% to 26.2% | 4.0% |
| sacks a team game | 2.08 to 1.65 | 2.05 to 1.85 | 2.24 to 1.85 | 2.41 (FBS 2.00) |

Why it moved: the fake holds the rushers and the box until their reads, so play-action pressure fell below a straight drop's and the sacks fell with it; the half-slide gives the back a man of his own; on the boot the guard has the end man, so the roll is rarely sacked (2.4% at the NFL), though a third of boots are still pressured. The call habit puts the fake on early downs from run looks (57% of NFL play action from under center, real 58%), and air yards a play-action attempt fell by 41% at the NFL (45% at FBS and in high school) because any concept, short ones too, can carry the fake now.

The typical team (`fit_check`, the spec's own measure: team 0 of each test league is the level's typical team, here in the West Coast context; FBS seeds 151 and 171 at 3,000 games an arm, the NFL's 161 at 2,400; main with SF3 merged, then after):

| Measure (West Coast) | FBS 151 | FBS 171 | NFL 161 | Real |
|---|---|---|---|---|
| play action, share of dropbacks | 3.5% to 21.7% | 3.3% to 20.8% | 4.2% to 20.7% | 21.2% |
| EPA a dropback, play action | -0.211 to +0.151 | -0.195 to +0.125 | -0.032 to +0.210 | +0.149 |
| EPA a dropback, straight drops | +0.195 to +0.197 | +0.172 to +0.164 | +0.163 to +0.168 | +0.031 |
| play action over a straight drop | -0.406 to -0.046 | -0.367 to -0.039 | -0.195 to +0.042 | +0.118 |
| sacked, play action | 19.9% to 5.2% | 19.1% to 5.1% | 21.9% to 5.0% | 6.6% |
| sacked, straight drops | 3.9% to 3.9% | 3.7% to 3.8% | 4.2% to 4.3% | 7.3% |
| the boot, share of dropbacks | 1.1% to 0.3% | 1.1% to 0.3% | 1.3% to 0.4% | |
| sacked, the boot | 44.5% to 2.0% | 39.7% to 2.4% | 51.3% to 1.8% | 5.7% (every play action that leaves the pocket) |

By system, after: the share of dropbacks with a fake, then play action over a straight drop (EPA a dropback):

| System | FBS 151 | FBS 171 | NFL 161 |
|---|---|---|---|
| Air Raid | 15.2%, -0.022 | 14.6%, -0.024 | 14.5%, +0.026 |
| spread option | 16.7%, +0.026 | 15.8%, +0.011 | 15.3%, +0.049 |
| West Coast | 21.7%, -0.046 | 20.8%, -0.039 | 20.7%, +0.042 |
| power gap | 25.2%, +0.016 | 24.1%, +0.004 | 23.9%, +0.054 |
| wide zone | 30.2%, +0.069 | 28.8%, +0.016 | 28.3%, +0.096 |

Play action itself is near real on the typical team at FBS (+0.15 and +0.13 EPA a dropback) and above it at the NFL (+0.21; real NFL +0.15); its straight drops run far above real (+0.16 to +0.20, real +0.03), so the gap between them comes out short (below, "Left open").

The quarterback's two play-action ratings, re-measured (+15 on team 0's quarterback, points a game ± one standard error; main with SF3 merged, then after):

| +15 on team 0's quarterback | FBS 151 | FBS 171 | NFL 161 |
|---|---|---|---|
| Ball Handling, West Coast | -0.18 ± 0.22 to +0.37 ± 0.27 | -0.20 ± 0.21 to +0.40 ± 0.26 | +0.08 ± 0.17 to +0.11 ± 0.20 |
| Ball Handling, wide zone | -0.12 ± 0.15 to +0.26 ± 0.23 | -0.03 ± 0.17 to -0.10 ± 0.23 | +0.06 ± 0.17 to +0.05 ± 0.19 |
| Ball Handling, spread option | -0.24 ± 0.27 to -0.06 ± 0.27 | -0.06 ± 0.18 to -0.23 ± 0.23 | +0.09 ± 0.20 to +0.25 ± 0.24 |
| Throw on the Move, West Coast | -0.02 ± 0.10 to +0.22 ± 0.10 | +0.02 ± 0.09 to +0.14 ± 0.10 | +0.16 ± 0.09 to -0.08 ± 0.11 |
| Throw on the Move, wide zone | -0.07 ± 0.13 to +0.04 ± 0.16 | +0.11 ± 0.09 to -0.03 ± 0.10 | +0.10 ± 0.11 to -0.14 ± 0.11 |

One cell of the fifteen is past the noise (Throw on the Move in the West Coast at FBS seed 151, +0.22 ± 0.10; +0.14 ± 0.10 on seed 171 and -0.08 ± 0.11 at the NFL), about what chance gives, and each arm moves EPA a dropback by under 0.02. +15 Ball Handling makes the fake's own part of each fooled man's read 16% longer (`SELL_PER_POINT`), about 0.05 s or 5% of his whole read for a typical starter, and only on the 15-30% of dropbacks with a fake (and on draws): over the nine cells it averages about a tenth of a point a game. Throw on the Move reads only on a throw at over 3 yd/s, and the boot is 2% of play action in the league (left open, below; on the typical team 0.3-0.4% of dropbacks in the West Coast, 1.2-1.5% in wide zone), so few throws read it.

The league totals that moved (`match_check`, seeds 1-12 at 400 games each, at every level; main with SF3 merged, then after, with the change ± its standard error over the 12 seeds). Every total stays inside its band in `tests/match_engine.rs`.

| A team game | NFL | FBS | High school | Real |
|---|---|---|---|---|
| sacks | 2.11 to 1.64 (-0.47 ± 0.03) | 2.04 to 1.76 (-0.28 ± 0.02) | 2.18 to 1.88 (-0.30 ± 0.03) | 2.41 (FBS 2.00) |
| pressure rate | 22.3% to 20.2% | 24.2% to 22.8% | 31.4% to 29.4% | 22.2% |
| pass attempts | 33.5 to 34.6 (+1.09 ± 0.07) | 31.6 to 32.9 (+1.28 ± 0.09) | 21.2 to 22.0 (+0.77 ± 0.07) | 33.3 (FBS 31.0) |
| net pass yards | 234.8 to 247.5 (+12.7 ± 0.7) | 219.3 to 232.4 (+13.1 ± 0.9) | 122.8 to 132.6 (+9.8 ± 0.8) | 218.6 |
| yards an attempt | 7.38 to 7.44 (+0.06 ± 0.01) | 7.32 to 7.39 (+0.07 ± 0.03) | 6.45 to 6.58 (+0.13 ± 0.02) | 7.05 (FBS 7.29) |
| points | 24.73 to 25.61 (+0.88 ± 0.10) | 23.54 to 24.32 (+0.78 ± 0.18) | 17.76 to 18.41 (+0.65 ± 0.10) | 22.51 (FBS 26.97) |
| EPA a play | +0.036 to +0.057 | +0.016 to +0.030 | -0.011 to +0.004 | -0.004 |
| snaps under center | 40.3% to 43.4% | 31.8% to 34.3% | 43.5% to 45.9% | 31.9% |
| tight ends' share of targets | 17.1% to 16.3% | 13.5% to 13.3% | 13.5% to 13.7% | 21.9% |

Why: the sacks play action gave away are gone (above), so those snaps are throws now (0.47 a team game at the NFL, 0.28 at FBS, 0.30 in high school). With fewer scrambles and more plays a game (+0.47, +0.88 and +0.48; why is not measured), that is about one more attempt, 10 to 13 more net yards and most of a point a team game at each level. The NFL test league already scored above real (24.7, real 22.5); FBS moves toward its real 27.0. Straight drops were sacked short of real before this lane and still are (4.9% at the NFL over the 12 seeds, real 7.3%), so the NFL's sacks are further below real now (1.64, real 2.41), and FBS's went from about real (2.04, real 2.00) to below it (1.76) (left open, below). Snaps under center rise because a fake comes from the run's own backfield, on top of straight drops that already came from under center too often (left open). Tight ends' share of targets slips at the NFL (17.1% to 16.3%, real 21.9%); why is not measured, and their routes are lane SF2's.

Seed 181's 32 NFL clubs (the world file from main 61ab9cc, `AFM_AB_WORLD`): `match_check` 2,000 games with every club on its own staff and call sheet, and `fit_check` with club 1 as team 0 (NFL seed 161, 2,400 games an arm, the West Coast context); main with SF3 merged, then after:

| | Every club (`match_check`) | Club 1 (`fit_check`) | Real |
|---|---|---|---|
| play action, share of dropbacks | 6.8% to 16.2% | 4.3% to 17.7% | 21.2% |
| EPA a dropback, play action | -0.092 to +0.197 | -0.269 to +0.099 | +0.149 |
| EPA a dropback, straight drops | +0.090 to +0.094 | +0.013 to +0.030 | +0.031 |
| play action over a straight drop | -0.182 to +0.103 | -0.282 to +0.069 | +0.118 |
| sacked, play action | 22.0% to 3.8% | 28.8% to 5.1% | 6.6% |
| sacked, straight drops | 4.3% to 4.2% | 5.9% to 5.7% | 7.3% |
| sacked, the boot | 44.0% to 1.2% | 54.1% to 6.1% | 5.7% |
| sacks a team game | 1.75 to 1.36 | | 2.41 |
| points a team game | 23.58 to 24.66 | | 22.51 |
| net pass yards a team game | 225.3 to 237.1 | | 218.6 |

On the world's clubs the gap is near real (+0.103, real +0.118). They fake on 16.2% of dropbacks, the test league's teams on 19.6%. Partly the schemes: 12 of the world's 32 clubs run the Air Raid, which fakes least, 10 the West Coast, 5 wide zone, 3 power gap and 2 the spread option; the test league has 9 Air Raid, 6 West Coast, 5 wide zone and 12 power gap teams. On the typical team's shares by system that mix gives about half of the 3.4 points; the rest is not measured. Their points stay inside the world's agreed 20-25.

Speed (`match_check`, 2,000 games at seed 1, the machine otherwise quiet, main with SF3 merged and after in turn, three rounds; the games' own time): NFL 15.4 s on main against 16.2 after (+5.6%), FBS 16.2 against 17.1 (+5.1%), high school 11.6 against 12.1 (+4.3%). Every round was slower after (+2.6% to +10.5%).

Left open:

- **The league's sacks.** Play action is sacked near real now (4.7% at the NFL, real 6.6%), but straight drops were short of real before this lane and still are: 4.8% (real 7.3%), and 5.0% on 3rd and 4 or more on the 240 games below (real 10.6%). Sacks a team game are 1.64 at the NFL over the 12 seeds (real 2.41), 1.36 on seed 181's clubs. The pass rush is lane SF8's.
- **The gap is short of real** at the NFL (+0.077 EPA a dropback, real +0.118) and at FBS (+0.014; no college charting on file). Play action itself is at real: +0.177 on 240 NFL test-league games (seeds 1001-1240) without screens and RPOs, real +0.150 (2024); their straight drops are +0.131 (real +0.032), a gap of +0.046 on those games. The straight drops are too good, most of all on 3rd down, three tenths of them: +0.263 on 3rd and 1 to 3 (real -0.127) and +0.159 on 3rd and 4 or more (real -0.034; real is 2024's 3rd downs alone). With those at real the gap on those games would be about +0.12. This lane leaves straight drops about as they were (+0.119 to +0.112 at the NFL); they are the pass rush's and the coverage's lanes (SF8, SF5 and SF7).
- **The boot, the shot and the dig** (minor bug 410, fixed 2026-11-18): the coaches' table centered the faked row of each by its own mean, so their own edge went out with their level (they fake on every snap and have no straight row) and each weighed one of 27 off a fake, about 2% of play action. The faked row is now shifted by the mean of the straight concepts of its own depth, as an ordinary concept's is by its own straight row, so the edge the fake gives them counts. On the NFL test league (`match_check nfl 2000`, seed 1): the boot 2.1% to 7.1% of play action, the shot 2.2% to 4.8%, the dig 2.3% to 5.8%; play action's EPA a dropback +0.189 to +0.214, its air yards an attempt 11.8 to 12.5 (real 9.2; the shot and the dig go deep, still the left-open short throws below). Real boots and rollouts are a larger share still (36% of play action out of the pocket, flushed passers too), so the boot is still below real.
- **Short play-action throws.** 5.7% of play-action throws are behind the line (real 14.8%), and air yards a play-action attempt are 11.8 (real 9.2).
- **Pressure.** Play action is pressured 0.93 times as often as a straight drop (real 1.19), and its time to throw is 2.82 s (real 3.15).
- **Straight drops from under center** (minor bug 411, fixed 2026-11-18): the gun draw added the system's gun lean and the caller's tell to a drop, and 21 and 22 personnel went under center whatever the play, 26% of straight drops (real 4%). A straight drop is now under center by a tenth of the system's under-center lean, led by the caller's tell, and a two-back package throws from the gun. NFL test league: straight drops under center 25.8% to 5.9% (a league with more under-center systems than the world's), dropbacks 31.9% to 15.4% (real 14.1%), snaps 44.1% to 33.6% (real 31.9%), runs 61.3% to 60.4% and play action 56.9% to 52.1% (real 58.0% and 58.1%). Screens, called after the look, go from 41.1% to 24.0% (real 18.7%). Straight drops from the gun are quicker and sacked less, which uncovers the pass rush's gap (SF8): sacks 1.64 to 1.43 a team game at the NFL (real 2.41), points 25.5 to 26.1 (test band 20-28), FBS 23.8 to 24.4, high school 18.6 to 18.9.
- **The two-high floor test** reads 87% two-high under a 0.95 floor (main 91%): the coordinator leans off two-high against an offense that fakes and runs well from run looks.
- **The scramble drill** (minor bug 409, fixed 2026-11-18): the passer who breaks out of the pocket headed for a spot three yards away, so the spot's distance set his pace (2.2 a yard, 6.7 yd/s) for every passer. He now aims three steps ahead at 0.67 of his top speed, the same 6.7 yd/s for the NFL's typical starter, 6.2 for a 5.2 forty and 7.1 for a 4.5 (`EXTEND_PACE`).
- **Faked throws count as play action** (minor bug 412, fixed 2026-11-18): `OffCall::family` reads the fake, `Film.faked` keeps how many throws of each concept came off one, and `Film::families` moves them from their concept's family (a faked flood is no longer a flood) to play action, screens excepted; so the film's mix, the ideas that spread from it, a new idea's surprise and how well a man knows his job (busts) see the fake. A new field on the film, empty in older saves.

## The inside line's rotation and the weather's size (engine gaps items 29 and 30, 2026-11-18)

Items 29 and 30 of `docs/design/engine-gaps.md`, one small-fixes lane. Every size is Claude's under Cain's proxy (2026-11-18), proposed, not confirmed. Real numbers: nflverse play-by-play 2021-25 (regular season, outdoor and open-roof games) and snap counts 2021-25 (2023-24 for the 4-3 split); the scripts are in the project folder at `afm/engine-gaps/tools/small_fixes_*.py.txt`.

What was wrong on main:
- **The inside line never rotated.** A tired man sits only for a backup above the same energy line (`pick`). Once the room has played a few snaps nobody is above it, so the starters play on: the top two tackles played 82% and 74% of their team's snaps, the fourth 5%. Moving the line or the allowance did nothing (0.90 and 600, 0.95 and 3,000, 0.99 and 9,000 all left 82-89% and 69-77%), as bug 368's sweep found. Letting a backup in when he is only fresher than the starter moved it 1-3 points. Real staffs rotate tackles by plan.
- **The wind's miss grew evenly with its speed** (0.010 a mph). Real dry games barely move to 10 mph and then fall fast, so main charged too much for a breeze and too little for a strong wind: 17 mph cost 1.7 points a team against the default 5 mph (real 15-19 mph against under 10: 2.7).
- **A wet ball's miss was a flat 10%.** Real rain hits long throws hardest (20+ air yards: 37.2% to 31.3% complete; under 10: 72.8% to 70.0%). Main's rain cost 1.7 points a team and 0.47 yards an attempt (real 3.4 and 0.75; 2.9 points in rain under 10 mph, as rain games are windier).

What changed (`game/flow.rs`, `game/scrimmage.rs`):
- **The staff rotates the inside line in waves** (`line_waves`, `DT_WAVE_SPREAD` 40, `DT_WAVE_STAY` 2). Each tackle's planned share of the room's snaps is ∝ e^(-gap / 40), gap his points behind the room's best man; the men furthest behind their share go in first, and a man on the field counts 2 snaps ahead, so they come in waves of a few snaps. The energy rule still spells a man who is spent. Not on kick units, not against a no-huddle (the men on stay on), and not when your own rotation plan (G15) names a man of the room. A star 20 points better than his backups still plays most snaps; a room of equals splits them.
- **Wind: 0.00129 × wind² / (1 + (wind / 20)²) × (1 + 0.01 × distance)** on the miss (`WIND_MISS`, `WIND_GALE`). A passer allows for a breeze but not a gale's gusts, and in a gale he throws lower and harder, so it levels off past 20 mph (real 20+ mph games, 18 of them at 23 mph on average, lose 0.46 yards an attempt, less than 15-19 mph's 0.65).
- **Rain or snow: 1 + 0.007 × distance** on the miss (`WET_THROW`): 3.5% at 5 yards, 21% at 30. The catch's ×0.97 in the wet is unchanged.
- Tried and dropped: footing (both sides' burst and cut 10% lower on a wet field raised yards after the catch by 0.5 a catch, real -0.37, because tacklers lose their closing burst more than carriers lose their cut) and a slower first step after a wet catch (no change in yards after the catch). Both are in `GAPS.md`, with ball placement after the catch.

`match_check` against main fe3e2b3 (the game code is the same as d6f1d71's), same seeds (seed 1; NFL and FBS 2,000 games, high school 1,000; `AFM_RAIN=1` and `AFM_WIND` runs at the NFL; `AFM_SNAPS=1` prints the rank shares, new here):

| NFL | main | after | real |
|---|---|---|---|
| tackles' snaps a game, ranks 1-4 | 82%, 74%, 38%, 5% | 71%, 63%, 48%, 14% | 68%, 50%, 31%, 16% (4-3 teams 68%, 54%, 36%, 23%) |
| points a team game, default (5 mph across) | 24.73 | 24.84 | 22.51 |
| rain against default: points | -1.72 | -2.36 | -3.45 (se 0.96); -2.87 under 10 mph |
| rain: yards an attempt | -0.47 | -0.68 | -0.75 |
| rain: completion, all / 20+ air / under 10 | -3.3 / -4.1 / -3.8 pts | -4.1 / -7.0 / -4.5 | -3.5 / -5.9 / -2.8 |
| wind 10 mph against none: points | -1.23 | -1.38 | -1.36 (10-14 against 0-4) |
| wind 17 against none: points / yards an attempt | -2.45 / -0.53 | -3.11 / -0.69 | -3.03 / -0.65 (15-19, se 0.15) |
| wind 22 against none: points / yards an attempt | -3.56 / -0.64 | -4.05 / -0.84 | -1.17 (se 2.0) / -0.46 (se 0.27; 20+) |
| wind 17 against the default | -1.70 | -2.34 | -2.69 (15-19 against under 10, se 0.8) |
| sacks / pressure rate | 2.08 / 0.221 | 2.07 / 0.219 | 2.41 / 0.222 |

- FBS: tackles 77% to 66% (top two); ranks 70%, 62%, 49%, 25%; points 23.32 to 23.66 (noise is about 0.3). High school (three tackles a roster): top two 82% to 68%, ranks 71%, 65%, 54%; points 18.21 to 18.19. No real snap counts for college or high school.
- Every other count in `AFM_GAPS=1` moved inside noise (injuries by position vary by about 0.02 a team game between seeds).
- The second tackle stays above real because the engine's fronts always field two tackles (three in a 3-4 base): real 4-3 teams line up 1.83 inside men a snap, sliding edges inside on passing downs, and the engine has no such package (`GAPS.md`).
- Rain's short completions fall 1.7 points more than real, all from the catch's ×0.97, which this lane kept; its long ones 1.1 more. Both sit inside rain's 60 games.
- `match_check nfl 2000` takes 14.8 s (main 14.5 s) on 4 cores.
- The coaches' call table and `decide_fit` were not rerun: in the default games the change is inside noise (points +0.11, yards an attempt +0.05), and the weather moves only bad-weather games, which the tables don't model.

Measured again over main ea85aad (with scheme-fit lane SF1's play action), the same seeds:

| NFL | main | after | real |
|---|---|---|---|
| points a team game, default | 25.49 | 25.80 | 22.51 |
| yards an attempt / a carry, default | 7.29 / 4.28 | 7.40 / 4.33 | 7.05 / 4.52 |
| rain against default: points / yards an attempt | -1.85 / -0.40 | -2.89 / -0.70 | -3.45 / -0.75 |
| wind 17 against none: points / yards an attempt | -2.79 / -0.58 | -3.13 / -0.72 | -3.03 / -0.65 |
| sacks a team game | 1.65 | 1.64 | 2.41 |

- Tackles by rank are the same as before (71%, 63%, 48%, 14%); FBS top two 77% to 66%, high school 82% to 68%.
- Over this main the rotation costs the defense more than before: points +0.31 at the NFL, +0.32 at FBS and +0.43 in high school, yards a carry +0.03 to +0.07. The backups play more snaps, and the rest the starters get doesn't yet win it back (`GAPS.md`).

## Personnel that fits the play and tight ends who run routes (scheme-fit lane SF2, items 1 and 2, 2026-11-18)

Design: `docs/design/scheme-fit.md`, items 1 and 2 (Cain: "personnel is a preference but scheme should still decide the play"; the backup tight ends' routes "a gamebreaking miss", because they get no receiving stats to grow on). Built on lane SF1, which decides play action with the throw, before the look. How it works: `docs/reference/how-it-works/play-calling.md` (the order of the call, personnel, the guidelines, a man with no route) and `docs/reference/how-it-works/match-engine.md` (the first read). Real numbers are 2024 NFL, nflverse play-by-play with its participation data and FTN charting (each target's man and the snap's personnel from the participation data); the league's totals (below) keep `match_check`'s own real columns: nflverse 2021-25 for the targets, FTN charting 2022-25 for where the quarterback lines up and for screens off a fake. Every size is Claude's, proposed, not confirmed.

What was wrong (on main, with SF1):

- **A preference took the group's place on any call.** The call fit its groups to the play (heavier on runs and fakes, lighter on straight dropbacks, by the caller's tell), then the head coach's group replaced the one drawn with chance 0.55 × his firmness, whatever the call: 47% of normal downs for a firm want. 13 personnel had no fit of its own, since it came only from that or from short yardage. So a power team set firmly to 13 threw on 41% to 46% of its 13 snaps (real 36%) and on 34% to 39% of its 12 snaps (real 46%): the heavier group threw more.
- **No concept read the second or third tight end.** The library draws the second tight end a route in 22 concepts, but none had him in its reads, and the quarterback throws only to his reads (feeding the best man and the scramble drill pick among them too). So the man in that role was never thrown to, and the third ran no route; the second and third on the depth chart caught the ball only on the snaps they played the first's role. With every team of the engine's test league set firmly to 13, tight ends drew 31% of the targets in 13 personnel at the NFL and 35% at FBS (real 56%) and the backs 30% (real 17%); set to 12, 21% of 12's (real 33%).

What changed (`game/calling.rs`, `game/playbook.rs`, `game/scrimmage.rs`, `game/flow.rs`, `game/sheet.rs`, `world/plans.rs`):

- **The group fits the play.** Each receiver a group trades for a blocker leans it toward runs and throws off a run fake by the caller's tell, and away from a straight dropback by the same: each weight × h^heavy, h = 1 ± 2 × tell (at least 0.2), heavy 10 personnel -1, 11 0, 12 and 21 1.25, 13 1.75, 22 2.75 (`Personnel::heavy_fit`; main had 12, 21 and 22 at 1 and 13 at none). A predictable caller (tell 0.33) matches his groups to his calls; a creative one (0.07) shows the same looks for both, so he still throws from heavy sets. For the NFL's typical caller (creativity about 14, tell 0.15), 12 and 21 come out on straight dropbacks 0.47 times as often as 11 for their share of the runs and fakes, 13 0.34 times and 22 0.19 (real 2024 NFL on first and ten, screens, RPOs and sneaks left out: 0.46, 0.47, 0.34 and 0.20). They are sized on first and ten, where the down doesn't pick the group. The fit is the fielded group's: a lean to 13 with two healthy tight ends fits like 12 (`available`).
- **A preference is a lean.** The head coach's group, or the one your word needs, gains its firmness × the scheme table's whole weight before the fit (`Groups::of`), so it comes out on every kind of call but mostly on the plays it fits. On normal downs and on 3rd and 1, a power team set to 13 passes as often as one with no preference (a unit test, `personnel_fit.rs`). Short yardage and the hurry-up keep their own groups.
- **The extra tight ends take the missing receivers' places.** Each receiver a concept reads that the group lacks gives his place in the reads to the group's next tight end past the first, in the concept's read order (12 and 22: the second; 13: the second, then the third; `stand_in`, `reads_in`). In the first inside place (the slot's) he keeps the slot's route only when it is a tight end's own, the seam, the corner or the drag (real playbooks' Y-seam, Y-corner and mesh), and otherwise runs the concept's own short route for the second tight end (the flat in 14 of the 22 concepts that draw one), or the slot's in Texas, which draws him none. In the flanker's place he runs the flanker's route when he lines up on the flanker's side (13's second tight end), and his own route when he lines up across the ball from it (22's second, inline on the weak side, and 13's third): the flanker's route is drawn for the other side of the play, the flood's corner on the boot, which rolls away from him (`route_in`, `strong_side`). Across the ball from the slot's place he keeps its route only when it works from either side (the seam, the drag, or a route the concept draws in both inside places, smash's two corners), so in snag 12 and 22's second tight end runs his own sit, not a corner away from its triangle. And he runs his own job wherever the borrowed route would finish within three yards of another receiver's (`lands_on`, the designer's rule that one defender covers both): from the wing beside the first tight end, the flanker's slant, hitch and curl and the slot's seam and drag ran on top of the tight end's own, so 13's second and 22's from the wing run their own in slant-flat, hitches, curl-flat, four verticals and mesh, and 12's from the wing in four verticals and mesh. Max protection keeps them in, a screen keeps its own men, and an RPO keeps its own read and routes. 11, 10 and 21 personnel read as before.
- **The split end steps off the line beside an inline tight end** (`align`, `off_line`). With a tight end attached on his side, the split end, and the flanker atop a stack, line up 1.2 yards off the ball (0.8 on it before), as in real two-tight-end sets, so the tight end is the end man of the line and eligible (`designer::legal`). Before, 12 and 22's second tight end and 13's third, now read and thrown to, stood covered inside the split end, and so did the tight end inside a stack's flanker (a unit test runs every concept from every heavy set through the designer's rules). The designer itself still counts a man exactly a yard off the ball, where the slots stand, as on the line, so it reads 11 personnel's slot as an end man (only your own designs see it); and in 10 personnel's doubles, trips and empty sets the flanker lines up off the ball, which leaves that group six on the line by the rule book's yard (atop a stack he is on it, and in a bunch the point man is; minor bug 416).
- **The concept fits the group as the rule book fields it**: it is drawn among the concepts that have one of their reads on the field (`readable`, `readable_library`). 13 and 22 personnel have no flanker for the tunnel screen; the CFL's 12-man sets and arena football's 8-man set for them do, and the call reads them (`OffCtx::players`, `Personnel::for_players`). One read on the field is enough, so a group with no slot can still call the bubble screen on the split end alone, a hurried throw at the screen's timing (minor bug 417, the same before this lane).
- **Third down reads the routes the group runs.** The caller's check for routes that reach the sticks takes the first two reads as his group lines up, so in 12 personnel Y-option's second read is the tight end's flat, not the slot's 10-yard curl (`sticks_depth`). The coordinators' kickoff sheet keeps its weights in a memo, which on those downs keys them by the group and, for 12, 13 and 22, by the set, which moves the extra tight ends' routes (`route_set`: 22's second tight end runs drive's 12-yard in from the wing and his own flat from across the ball), so it draws the same calls as working them out each time (two unit tests, one over every scheme, group and roster).
- **A sheet's throw fakes as its look says** (`sheet::faked`, `calling::sheet_fake_chance`). A sheet line keeps the throw's group, backfield, set and concept, not its fake, and the drafted call fitted the group, the backfield and the set to its fake, so the game reads the fake back from them: Bayes' rule on the call's own chances of that look off a fake and off a straight drop, in the snap's situation. Before (SF1's rule), a sheet's throw took his chance of a fake on any throw, or his own throw's fake, whatever the line's look, so the opening script's 13 under center faked no more often than its four wide in the gun. A unit test checks the read-back against the call's own throws, look by look.
- **An RPO's throw goes to a man on the field** (`pick_rpo`, `rpo_chance`): it is drawn among the RPO throws the group can run as the rule book fields it. 12 and 21 personnel, which have no slots, drew the RPO bubble on a third of their RPOs, and the snap played as a plain run filmed as an RPO.
- **The game plan's identity board** (`afm-game`, `lane_gameplan/identity.rs`) reads the personnel guideline as the engine applies it: on normal downs firmness k closes k / (1 + k) of the gap to the wanted group, 35% at Lean, where it showed main's swap (0.55k, 30%).
- **Your sheet in another league.** Your coordinators' call sheet for the week in the CFL or arena football (`draft_with`, drafted on plan day) is drafted on that league's own rule book, its players a side and its two-point line (`world/plans.rs`, `seat_book`); it used to take the NFL's (`level_of`). The sheet drafted at kickoff already read the game's rules; it now reads its players a side too, and so does `match_check`'s timing of it under another league's book.
- **Speed.** The groups sit on the stack, the runnable library concepts are a bit mask worked out once a group (`readable_library`), a group that runs the whole library draws its concept as before without asking each one (`LIBRARY_ALL`), a man who isn't an extra tight end leaves the stand-in check at once, third down reads its two routes one at a time (`reads_of`), and the fit has no `powf`.

Item 1, a power team set firmly to 13 personnel or to 12 (`fit_check`'s `pg13` and `pg12` beside `pg`, the same team with no preference; the audit's seeds, 3,000 games an arm at FBS and 2,400 at the NFL; SF1's engine head, then after; real 2024 NFL league-wide):

| | FBS 151 | FBS 171 | NFL 161 | real |
|---|---|---|---|---|
| set to 13: its snaps in 13 | 36% > 36% | 36% > 36% | 36% > 35% | |
| set to 13: pass rate from 11 / 12 / 13 | 65 / 37 / 44% > 69 / 41 / 40% | 63 / 34 / 41% > 67 / 38 / 37% | 70 / 39 / 46% > 75 / 43 / 40% | 65 / 46 / 36% |
| set to 13: the tight ends' share of 13's targets | 32% > 63% | 29% > 61% | 28% > 57% | 56% |
| set to 13: points a game | 26.29 > 28.13 | 25.93 > 26.95 | 23.63 > 25.35 | |
| no preference: points a game | 27.95 > 28.29 | 26.75 > 27.37 | 24.97 > 25.59 | |
| set to 12: pass rate from 12 | 42% > 43% | 40% > 41% | 45% > 44% | 46% |
| set to 12: the tight ends' share of 12's targets | 19% > 31% | 18% > 31% | 19% > 32% | 33% |
| set to 12: points a game | 27.54 > 28.43 | 26.42 > 27.07 | 24.95 > 25.52 | |

Why: on main the want replaced the group on 47% of normal downs whatever the call, so 13 threw at the team's whole pass rate and took straight dropbacks from 12. Now 13 gets its weight before the fit, which leans it toward runs and fakes, so the straight dropbacks go back to 12 and 11. Its share of the snaps is the same, because the lean is as big as the old swap. Points: on main a power team set to 13 scored 0.8 to 1.7 a game less than the same team with no preference, since its second and third tight ends' places were never thrown to and it threw from 13 at its whole pass rate; after, 0.2 to 0.4 less (one arm's noise is about 0.2 to 0.25 points a game, from the repeat table's z-scores). The typical team with no preference scores more too: 0.5 to 0.6 points a game in the West Coast context, the audit's base, which lines up in 12 personnel on 22% of its snaps, where its tight ends now draw 36% of the targets (22% to 23% before). The test league's own teams line up in 12 on 11% of their snaps (the league tally with no team set to a group), so the league's points rise less (`match_check`, below).

Item 2, the targets by group, league-wide (the engine's test league with every team set firmly to 12 or to 13, 2,000 games a level, seed 1; each team's tight ends ranked by their snaps, each target by his rank among the tight ends on the field; SF1's engine head, then after; real 2024 NFL):

| | NFL | FBS | real |
|---|---|---|---|
| 12 personnel: targets to the receivers / tight ends / backs | 58 / 21 / 21% > 53 / 33 / 14% | 58 / 21 / 21% > 55 / 31 / 14% | 33% to the tight ends |
| 12: the second tight end's share of the tight ends' targets | 0% > 53% | 1% > 50% | 43% |
| 13 personnel: targets to the receivers / tight ends / backs | 38 / 31 / 30% > 28 / 55 / 16% | 36 / 35 / 30% > 26 / 59 / 15% | 28 / 56 / 17% |
| 13: the first / second / third tight end's share of the tight ends' | 86 / 14 / 0% > 32 / 48 / 21% | 77 / 22 / 1% > 33 / 44 / 23% | 47 / 29 / 24% |
| air yards a target, 12: the first / second tight end | 9.5 / 7.1 > 10.1 / 8.0 | 9.2 / 8.9 > 9.7 / 7.8 | 7.0 / 6.3 |
| air yards a target, 13: the first / second / third | 8.5 / 7.4 / none > 9.0 / 7.9 / 6.9 | 8.7 / 7.4 / 6.6 > 8.9 / 7.6 / 6.9 | 7.6 / 6.5 / 4.5 |

With no team set to a group, the same: the tight ends draw 20% > 31% of 12 personnel's targets at the NFL and 21% > 31% at FBS, and 31% > 57% and 37% > 68% of 13's, where 13 comes only from short yardage and the goal line. With every team set to 12, on main the second tight end's few targets (46 at the NFL in 12 personnel, of 11,909 to tight ends) came on the snaps he played the first's role. The second tight end's targets in 12 personnel (NFL, after, every team set to 12): the flat 57%, the sit 19%, the drag 8%, the corner 6%, the seam 6%, the out 5%. Real: the quick out 34%, the hitch and curl 16%, the drag 9%, the corner 8%, the in and dig 8% (FTN's route names). The third's in 13 (NFL, after, every team set to 13): the flat 54%, the sit 25%, the out 10%, the seam 3%.

A backup tight end has a line of his own now, and his own skills move it (`fit_check`, the power team set to 12 or 13 on the audit's seeds; Catching +15 for that one man; a season is 12 games at FBS and 17 at the NFL; SF1's engine head, then after):

| | FBS 151 | FBS 171 | NFL 161 |
|---|---|---|---|
| set to 12, the second tight end: targets a game, yards a season | 0.94, 83 > 2.67, 267 | 0.80, 71 > 2.50, 250 | 0.51, 64 > 2.61, 365 |
| Catching +15: targets, yards | +0.08, +3 > +0.24, +33 | +0.07, +5 > +0.22, +24 | +0.06, +10 > +0.26, +50 |
| set to 13, the second: targets a game, yards a season | 1.79, 152 > 3.71, 362 | 1.48, 123 > 3.36, 334 | 0.86, 108 > 3.31, 481 |
| Catching +15: targets, yards | +0.05, +6 > +0.25, +48 | +0.00, +7 > +0.28, +34 | +0.04, +6 > +0.15, +38 |
| set to 13, the third: targets a game, yards a season | 0.07, 8 > 1.94, 193 | 0.06, 6 > 1.80, 176 | 0.04, 7 > 1.29, 183 |
| Catching +15: targets, yards | +0.00, +1 > +0.32, +32 | +0.00, +1 > +0.32, +27 | +0.00, -0 > +0.24, +37 |

Every Catching cell after is past the noise; before, only the second tight end's in 12 personnel were (+0.06 to +0.08 targets a game on all three seeds, and +10 yards a season at the NFL). The third's Catching +15 is the repeat table's claim for item 2 (`fit_check repeat`): +32, +27 and +37 yards a season on the three seeds, against +0.6, +0.6 and -0.2 (inside the noise) on SF1's head.

The league's totals hold (`match_check`, 2,000 games at each level on seeds 1-4, every team on its own staff, college and NFL teams on their own call sheet (high school teams draft none); SF1's engine head, then after, the change ± its standard error over the four seeds). Every total stays inside its band in `tests/match_engine.rs`, and the coaches' table doesn't move: `call_values` writes the same file on SF1's engine head and after, so the committed table stands.

| | NFL | FBS | high school |
|---|---|---|---|
| points a team game | 25.63 > 25.76 (+0.13 ± 0.07) | 24.16 > 24.40 (+0.24 ± 0.11) | 17.91 > 18.03 (+0.12 ± 0.02) |
| yards an attempt | 7.48 > 7.51 (+0.03 ± 0.01) | 7.42 > 7.48 (+0.06 ± 0.01) | 6.59 > 6.65 (+0.07 ± 0.02) |
| interceptions a team game | 0.736 > 0.732 (-0.004 ± 0.006) | 0.613 > 0.621 (+0.008 ± 0.004) | 0.342 > 0.338 (-0.004 ± 0.002) |
| targets to the wide receivers / tight ends / backs | 68.1 / 16.1 / 15.8% > 67.3 / 18.2 / 14.5% | 70.1 / 13.9 / 15.8% > 69.8 / 15.5 / 14.6% | 63.1 / 14.9 / 19.0% > 62.3 / 17.4 / 17.5% |

Real NFL targets: 59.3 / 21.9 / 18.1%. The tight ends move toward it; the backs move away, because on main they drew the throws the missing tight ends' places would have (30% of 13 personnel's targets, real 17%; now 16%). The league's backs were short of real before this lane, in 11 personnel, which it doesn't touch: 13% of 11's targets at the NFL on main and after. FBS points move toward their real 27.0. The sheet's fake (above) moves where play action comes from: at the NFL 56.0% > 60.4% of play action comes from under center (real 58.1%), at FBS 47.8% > 51.1%. Straight drops from under center fall too, 26.5% > 25.1% at the NFL (real 4.0%; minor bug 411 then took the rest, see "Play action that works", left open, fixed) and 18.7% > 17.4% at FBS, and not from the sheet alone: high school teams draft no kickoff sheet, and there straight drops from under center fall 22.6% > 22.0% (on all four seeds; 22.4% with the first build's fit sizes) while play action from under center holds (63.8% > 64.0%). That part is likely the group fit's: 21 and 22 personnel line up under center but in the hurry-up, and the fit sends them out on fewer straight drops. Screens off a fake rise from 34.2% to 36.9% of the NFL's screens and from 31.3% to 33.0% of FBS's (real 27.3%): the sheet's screens now fake as often as the same calls from the same look do when called live, and those fake more often than real (SF1's sizes). In high school they fall a little (24.7% > 24.1%); neither the concept filter nor the fit's sizes, each turned back off alone, moves that, and its cause is not traced. High school's points rise too over SF1's head (+0.12 ± 0.02 a team game; +0.03 ± 0.06 over main 349eefb, below), and the cause is not traced. Its teams line up in 12 personnel on 10% of their snaps and in 22 on 5% (the league tally with no team set to a group, 2,000 games at seed 1), and their second tight end drew 6% of 12's targets and 7% of 22's on SF1's head, 18% and 30% after, while the backs fell from 26% to 17% and from 34% to 21%; pass yards rise 2.1 a team game (± 0.5). But the lane's first build already threw to him (19% of 12's targets and 26% of 22's; the backs 17% and 23%), and there high school's points fell 0.06 (± 0.05). The rise came with the second review's fixes (below) together, +0.18 ± 0.05 over the first build, though each of five of them, turned back off alone, moves high school points 0.06 or less (the split end off the line and the fit's sizes the most, -0.06 ± 0.05 and -0.06 ± 0.03). Item 1 and item 2 were not turned off alone.

Measured again over main 349eefb, which has engine gaps items 29 and 30 (the inside line's rotation by plan and the weather at its real size, above; they change the games), the same way (2,000 games at each level on seeds 1-4; main 349eefb, then the lane merged into it at cc4e7a7):

| | NFL | FBS | high school |
|---|---|---|---|
| points a team game | 25.81 > 26.02 (+0.22 ± 0.09) | 24.44 > 24.63 (+0.20 ± 0.09) | 18.17 > 18.20 (+0.03 ± 0.06) |
| yards an attempt | 7.57 > 7.60 (+0.03 ± 0.02) | 7.52 > 7.56 (+0.04 ± 0.03) | 6.68 > 6.69 (+0.01 ± 0.03) |
| interceptions a team game | 0.741 > 0.740 (-0.001 ± 0.006) | 0.595 > 0.612 (+0.017 ± 0.007) | 0.334 > 0.338 (+0.004 ± 0.002) |
| targets to the wide receivers / tight ends / backs | 68.0 / 16.2 / 15.8% > 67.1 / 18.3 / 14.6% | 70.2 / 13.9 / 15.8% > 69.7 / 15.5 / 14.6% | 63.2 / 14.8 / 18.9% > 61.9 / 17.3 / 17.7% |

The tight ends gain as they did over SF1's head (+2.1, +1.6 and +2.5 points of the targets at the NFL, FBS and in high school). At the NFL and FBS the wide receivers and backs give up about the same shares as before; in high school more of it comes from the wide receivers here (-1.3 ± 0.2 points, -0.8 ± 0.1 over SF1's head) and less from the backs (-1.25 ± 0.15, -1.6 ± 0.05). High school's points barely move over this main: its pass yards still rise (+1.5 ± 0.6 a team game, +2.1 ± 0.5 over SF1's head), but its rush yards fall a little (-0.6 ± 0.4, -0.1 ± 0.4 over SF1's head) and its passing touchdowns hold (+0.001 ± 0.010, +0.017 ± 0.007 over SF1's head). Four seeds give only a rough spread: high school's change was ± 0.02 over SF1's head and ± 0.06 here. Over all eight pairs of runs, both mains, the lane adds 0.18 ± 0.05 points a team game at the NFL, 0.22 ± 0.07 at FBS and 0.07 ± 0.04 in high school.

Speed (`match_check`, 2,000 games at each level on seeds 1-4, SF1's engine head and after in turn on an otherwise quiet machine, two rounds; the games' own time): the NFL +2.3% and +2.7% (12.90 s to 13.25 on the second round), FBS +3.0% and +2.4% (13.78 to 14.10), high school -1.5% and +0.8% (9.32 to 9.40). Of the 8 pairs of runs at each level, the NFL and FBS were slower after in 6, high school in 2 (and tied in 1). Counted in instructions (callgrind, 160 FBS games at seed 1 on one thread): the games +2.1%, and 86% of that is the coordinators' kickoff sheet, which each college and NFL team drafts at kickoff and high school teams don't (+11.6%: each of its 3,080 calls fits the groups to the call, draws the concept among the ones the group can run and, on third and fourth down, reads the routes the group runs); the plays themselves +0.4% (the extra tight ends run their routes and are read). The speed fix (a group that runs the whole library draws its concept without asking each one, `LIBRARY_ALL`; a man who isn't an extra tight end leaves the stand-in check at once; third down reads its two routes one at a time, `reads_of`) plays the same games as the lane before it, line for line, on all 12 runs; the mask of the concepts a group can run (`readable_library`) and the fit without `powf` were in the lane from its first build.

The routes a tight end runs in a receiver's place are Claude's rule, proposed, not confirmed. Three were tried on the second tight end in 12 personnel (the league tally, FBS, 400 games, on the lane's first build):

- the slot's own routes: 9.8 air yards a target (real 6.3), with too few flats and outs;
- the library's own route for the second tight end alone: 5.1, with no corners or seams;
- the slot's route when it is a tight end's own (the seam, the corner, the drag), else the library's (built): 7.8 at FBS and 8.0 at the NFL on the final build (every team set to 12, 2,000 games), about two yards under the first tight end (9.7 and 10.1; real 0.7).

The rule across the ball came from a code review: on the boot, 22's second tight end (inline on the weak side) ran the flanker's corner, drawn for the flood on the strong side, toward the weak sideline, away from the roll, so the quarterback rolled away from his own read. He runs the concept's own job for him now (the drag back across into the roll), and so does 13's third in the flanker's place. A second review found the routes that finished on the first tight end's, snag's corner, the covered spots, the sizes worked out at creativity 10 rather than the level's typical caller, the RPO bubble from 12 and 21, the sheet's fake and the identity board's old rule; the tables here are measured after those fixes.

Still off:

- **A better tight end is fed more but scores no more.** Catching or Route Running +15 on the first tight end adds 0.19 to 0.45 targets a game, past the noise in all 18 cells (the West Coast, `pg12` and `pg13` on three seeds), and moves his team's points -0.38 to +0.44, none of the 18 past the noise (SF1's head: -0.32 to +0.43, one past it). His run blocking feeds him nearly as much as his hands (Run Blocking +15: +0.16 to +0.40 targets). On some dropbacks (10% plus 2% a point of his edge over the next man, at most 55%) the quarterback's first look goes to the man rated best at his own spot, whatever made him best, ahead of the concept's own order, and with more tight ends in the reads that feed picks them more (item 25, lane SF4).
- **In 13 personnel the second tight end out-draws the first** (48% of the tight ends' targets at the NFL against the first's 32%; real 29% and 47%). He takes the flanker's place, which many concepts read before the slot's, while the first keeps his own routes, and where the flanker's route would land on the first's he runs his own short job, which the quarterback finds more often. Who the quarterback reads first is lane SF4's (items 5, 6 and 25).
- **Tight ends' targets run deep.** Across the league, 1.1 to 3.1 air yards a target deeper than real at every rank (12 personnel at the NFL: the first 10.1, real 7.0; the second 8.0, real 6.3), and the power team's backups deeper still (the second 8.7 to 10.8, the third 8.0 to 10.6). Main's first tight end already ran 9.5; his is up because the second now draws many of the short throws he used to draw. The second's corners and seams are 12% of his targets in 12 personnel (real corners 8%, and no seam in FTN's top five for him). The depth of each route is the engine's own, not this lane's.
- **A team that lives in 13 still throws more of its 13 snaps than real** (the power team 37% to 40%; with every team of the test league set to 13, 48.8% at the NFL and 42.0% at FBS; real 36% league-wide). Real 13 personnel is a situational group used mostly for runs, and the fit leans a call only by the caller's tell. That a team in 13 on a third of its snaps throws from it more than the league's 36% is likely, but there is no real number on file for such teams.

`fit_check` (`run`, then `read`) gives the item 1 and backup tight end tables: `_personnel.md` for the groups, `TE1.md` to `TE3.md` for the men; `fit_check repeat` the repeat table. The league tally is a scratch tool of the lane (not committed): the engine's test league with every team's guide set to 12 or 13, the targets by group and by each team's tight end ranks.

## Ball security: the loose ball, snaps, strips and backs (engine gaps lane EG2, 2026-11-18)

Engine gaps items 4-7 (`docs/design/engine-gaps.md`; Cain agreed to the audit on 2026-11-17, and the sizes were picked under his proxy): a fumble stays live under each level's recovery and advance rules, snaps go on the ground, sacks strip the ball at the real rate, and backs fumble at the real rate. The targets are nflverse 2021-25 from scrimmage: 0.983 fumbles a team game, 45.5% lost (0.447 a team game), 8.5% out of bounds, 28% of the defense's recoveries advanced and 13.6% for 10 yards or more, return touchdowns 0.032 a team game; fumbled snaps 0.185 a team game (0.467% of snaps under center, 0.228% in the shotgun), 75% kept; 12.4% of sacks, 0.75% of designed runs and 1.13% of catches fumbled.

What was wrong on main (2,000 `AFM_GAPS=1 match_check nfl` games, seed 1):

- **Every fumble was dead where it fell.** The ball bounced up to 3 yards and a weighted roll gave it to one side (nearness, Anticipation and Competitiveness, `fumble`): no return, no ball out of bounds, no scoop and score, no touchback or safety from a fumble.
- **No snap went on the ground** (real 0.185 a team game).
- **4.9% of sacks fumbled** (real 12.4%). The sack's own check read his Ball Security raw and neither the rusher's force nor his strip habit.
- **Backs fumbled twice the real rate:** 1.39% of designed runs (real 0.75%). The tackle's rate was fitted to the league's lost fumbles (0.45 a team game) while snaps never went on the ground and strips ran under two fifths of real, so the carries made up the missing fumbles. The carrier's Ball Security was read against the hitter's Strength and Tackling rather than a typical carrier's, and college and high school hits against the NFL's force.

What changed (every size proposed, not confirmed; the formulas are in `docs/reference/how-it-works/match-engine.md`, "Fumbles"):

- **The loose ball** (`game/sim.rs`, `fumble_going`, `loose_tick`, `loose_react`, `loose_grab`, `scoop_chance`, `loose_won`, `loose_end`; item 4). A fumble is `Ball::Loose`: it pops off the pile at 1-12 yards a second on top of the pile's own way, bounces, slows and skids, and every man on his feet goes for where it will be once he has seen it (his Anticipation and Play Recognition against his level's typical starter's, later with his back to it). The man who lost it and his tacklers are on the ground first. Nearest first, a man in traffic falls on it and one with room runs onto it and scoops it (his Catching and Agility); one who can't hold it knocks it on. Traffic is a man of the other side within 2 yards in the NFL, where a man who falls on it untouched gets up and runs; in college and high school, where falling on it ends the play, only a man of the other side less than half a yard farther from it than he is (`LOOSE_RACE`), so he scoops whenever he is first to it. In a pile the other side can come out with it (Strength and Competitiveness). A man on the ground is down in college and high school, and in the NFL when the other side touches him before he is up. Up with it, he runs like any carrier, his side blocking. The fumbling side covers its own ball. Out of bounds it is the fumbling team's (back at the spot of the fumble when it went forward); in an end zone a touchdown, a touchback or a safety by who put it there and who has it, with the momentum rule. A kick return's fumble plays the same.
- **Each level's rules** (`game/rules.rs`, `FumbleRule`, `Rules::fumble_rule`; `game/flow.rs`, `FumbleSnap`; `game/books.rs`). The NFL: only the fumbler may advance his own team's fumble on fourth down, on a try and after the two-minute warning of either half (since 1979, every era the world plays). College: fourth down and a try. High school: anyone, and a try ends when the defense has it; a state on college rules plays the NCAA's; a forward fumble out of bounds comes back to the spot from 2025 (NFHS 2025 rule change). The CFL: anyone, and a ball out of bounds belongs to the team that touched it last. A muffed snap is anyone's.
- **Fumbled snaps** (`game/scrimmage.rs`, `run_snap`; item 5). Every snap: under center 0.49% (the quarterback's Ball Handling 70%, the center 30%), in the shotgun 0.247% (40% and 60% the center's Shotgun Snap), each against the level's typical starter, more with a wet ball; half of the center's share under center and a quarter in the shotgun never reach the quarterback. The ball goes down at his feet or off his hands, live. A quarterback who picks up his own snap behind the line on a pass goes on with the play. A handoff goes on the ground 0.06% of the time for typical hands, by the back's Ball Security and the quarterback's Ball Handling (one in 500 before, read raw).
- **Strip sacks** (`sack_check`, `saw_coming`; item 6). 6.35% of sacks at the level's typical sack, 2.3 times when he never saw the rusher, 1.6 times in his throwing motion, by the rusher's force and strip habit and the passer's Ball Security against the level's typical starting back's. A strip in the motion is a fumble as he throws, kept on the sack.
- **The tackle's fumble** (`hit_effects`, `hit_scale`, `reach`; item 7). 0.0064 × (force / (1,700 × the level's share))^1.5, by the carrier's Ball Security against the level's typical starting back's (one scale for every carrier), the hitter's Strength and Tackling against his position's typical man, rain, snow and cold and the strip habit; 2.6 times inside half a second of a catch, before it is tucked away. The level's share of the NFL's typical hit (`hit_scale`, measured on `match_check`) is 0.82 at college and 0.62 at high school, 0.96 and 0.76 for a sack.
- **What a fumble ends as** (`game/flow.rs`, `resolve`; `game/stats.rs`). A lost fumble's yards run to where it came out, and the return is the defense's; a fumble on an interception return is a new series; the replay's overturn takes the loose ball with it (`officiate`); the play log's why names who lost it, who came up with it, which side and how far he ran; the recovering man's line counts `fumble_rec`.
- **A flag on the return** (`resolve`). Only the side with the ball blocks, so a holding or an illegal block by the defense is a foul on its own return, an interception's included: the ball stays its own, marked back from the spot of the foul (or from where the return ended, if that is short of it), half the distance to its own goal at most, and a score on the return comes off the board. Before, the offense could take that foul and the ball back with it, and a score on the return still counted (CI's loose ball test found one); a defense's score now counts as 7 points against the penalty, as the offense's does. A strip sack's time to throw ends when the ball comes out, not when its return does (`run_snap`).
- **Tools.** `AFM_GAPS=1 match_check` counts fumbles by how the ball came out and the fumbler's position, by nflverse's kinds (lost, out of bounds, advanced, 10+, scores and who came up with it: the defense, the fumbler or a teammate), and each kind's rate by the sacker's, the carrier's or the catcher's position. `tests/loose_ball.rs` checks the rules by level and that loose balls play out (returns, scores, out of bounds, both kinds of snaps; a lost fumble is always the defense's ball or out of bounds).

2,000 `AFM_GAPS=1 match_check nfl` games on seed 1, main (70c8cbf: play action and personnel that fits the play, lanes SF1 and SF2, the play action leftovers, minor bugs 409-412, the inside line's rotation, engine gaps items 29 and 30, and the 14-year-olds' recentred habits) and after (with the call table's run rows rebuilt and the flag on the return, below and above), against nflverse 2021-25:

| NFL, from scrimmage | main | after | real |
|---|---|---|---|
| fumbles a team game | 0.762 | 0.860 | 0.983 |
| lost a team game (share) | 0.374 (49.1%) | 0.385 (44.8%) | 0.447 (45.5%) |
| out of bounds | 0 | 3.8% | 8.5% |
| the defense's recoveries advanced / 10+ yards | 0 / 0 | 30.8% / 12.7% | 28.0% / 13.6% |
| fumble return touchdowns a team game | 0 | 0.0192 | 0.032 |
| who comes up with it: the defense / the fumbler / a teammate | – | 46.4 / 22.6 / 27.3% | 44.8 / 21.4 / 25.3% |
| sacks fumbled; a team game, lost | 4.9%; 0.067, 41% | 13.1%; 0.190, 44% | 12.4%; 0.298, 51% |
| designed runs fumbled; a team game, lost | 1.39%; 0.353, 48% | 0.69%; 0.174, 60% | 0.75%; 0.197, 58% |
| catches fumbled; a team game, lost | 1.26%; 0.309, 55% | 1.18%; 0.289, 57% | 1.13%; 0.244, 50% |
| snaps fumbled under center / in the shotgun | 0 / 0 | 0.517% / 0.239% | 0.467% / 0.228% |
| fumbled snaps a team game, lost | 0 | 0.173, 15% | 0.185, 25% |
| points a team game | 26.68 | 26.63 | 22.51 |
| interceptions a team game | 0.802 | 0.820 | 0.756 |
| carries / pass attempts a team game | 25.97 / 35.40 | 25.93 / 35.09 | 26.22 / 33.28 |

The totals move with the seed: a game's draws part from the first snap, so the arms play different games. Over seeds 1-7 (2,000 games each) the lane takes lost fumbles from 0.362 to 0.371 a team game (+0.009, standard error 0.003), interceptions from 0.754 to 0.769 (+0.015 ± 0.006: the ones a flag by the defense on its own return no longer hands back, above) and points from 26.71 to 26.78 (+0.07 ± 0.04, with 0.13 points of return touchdowns, 0.0192 a team game). Each kind's rate a play over the seven seeds: 12.5% of sacks, 0.69% of designed runs, 1.19% of catches, 0.511% of snaps under center and 0.249% in the shotgun.

Lost fumbles stay 17% short of real (0.371 a team game over the seven seeds, real 0.447; main 0.362), but for another reason. On main, backs fumbled twice the real rate, which made up for the snaps and strips that were missing. Now each kind fumbles at its real rate a play, and the shortfall is the sacks: the engine has 43% fewer (1.37 a team game over the seven seeds, real 2.41: lane SF1 fixed play action's own 23% sack rate, 2.10 a team game before it, and since minor bug 411 lined straight drops up in the gun as real, a straight drop is sacked 3.9% of the time, real 7.3%, and play action 4.7%, real 6.6%; the pass rush is lane SF8's, the quarterback off script lane EG7's). On seed 1 (0.385 lost a team game) sacks fumble 0.190 a team game against 0.298 and are lost 44% of the time (real 51%: the defense comes up with 46%, the passer himself 21%), 0.068 a team game short of real; fumbled snaps are 0.020 short (kept 85% of the time, real 75%) and designed runs 0.011, and catches 0.042 over (0.164 lost a team game, real 0.122: 1.18% of catches fumble, real 1.13%, but the engine completes more passes, and 57% of those fumbles are lost, real 50%). Before play action merged, the same seed lost 0.423 a team game against main's 0.373, and the flag on the return took interceptions from 0.749 to 0.771 (on the same build without it): before it, a flag by the defense on its own return let the offense take the ball back. Each kind's rate by the man's position (the seven seeds pooled): sacks by edges fumble 16.2% of the time, by linebackers 9.3% (3,715 sacks), by defensive tackles 8.4% (real: the edge's spots 12.6-15.7%, inside linebackers 7.2%, tackles 7.7-9.3%); catches by backs 0.94%, tight ends 1.29%, receivers 1.01% (real 1.14%, 1.23%, 1.09%); designed runs by backs 0.69%, quarterbacks 0.66% (15,385 runs; real 0.70% and 2.12%, below).

FBS (2,000 games): fumbles 0.805 to 0.950 a team game, lost 0.388 to 0.397 (real, FBS against FBS in 2023 from cfbfastR's play text, 0.43), return touchdowns 0 to 0.021 (real 0.044), the defense's recoveries advanced 34%, sacks fumbled 5.0% to 12.9%, designed runs 1.33% to 0.79%, catches 1.21% to 1.22%, snaps 0.475% under center and 0.237% in the shotgun, points 24.8 to 25.0. High school (1,000 games): fumbles 0.588 to 0.724, lost 0.269 to 0.315, return touchdowns 0.0145, sacks fumbled 5.5% to 10.6%, designed runs 1.24% to 0.73%, catches 0.96% to 1.17%, snaps 0.493% and 0.205%, points 19.5 to 19.2. College and high school have no full real count of fumbles (`GAPS.md`).

The coaches' call table (`call_values.csv`, `call_values 2000`, 2,000 snaps a cell, played on main and on the lane over 1f3e8b4, and again over 70c8cbf): the lane moves its run cells +0.17 yards on average (up to 0.68), fewer fumbles on runs, so the 15 run rows are rebuilt (over 1f3e8b4; over 349eefb and dcdde89 the lane's tool writes the same rows, and over 70c8cbf, whose recentred 14-year-old habits redraw the tool's two teams' flaws, rows within 0.07 yards of them, 0.012 on average). Every pass row is kept. Over 70c8cbf the tool's straight pass cells move -0.14 on average (strip sacks: four verticals -0.7 a cell, the longest to throw), the three play-action concepts' -0.09 (the shot and the dig against Cover 0 -1.1 and -1.5) and the faked rows' +0.03, but cell by cell that is mostly noise (a row's cells spread 0.43 yards around its own mean), the tool's straight pass cells are still off from minor bug 405, and composing the faked rows on the kept straight rows, as SF1 did, would have handed the fakes the straight rows' strip-sack losses (+0.17 a cell, play-action four verticals +2.1; `docs/reference/how-it-works/play-calling.md`). The analysts' fourth-down, try and win expectancy tables (`world::tally::decide`, `decide_fit 1500` on seeds 1 and 2): the lane's fit is as far from main's as two fits of one engine on the two seeds are from each other (on main 70c8cbf, the lane against main on each seed, then main's and the lane's own two seeds: conversions a mean 0.008-0.009 a cell against 0.010-0.011, goal-to-go 0.035-0.041 against 0.035-0.041, gains 0.29-0.33 yards against 0.31, punts 0.47-0.51 yards against 0.79-0.88, field goals 0.016-0.020 against 0.021). One table moves one way on both seeds: a converted play gains 0.15 and 0.12 yards more (of 13.6), likely the runs that no longer end in a fumble (the call table's run cells, +0.17), but each cell's change is inside the seeds' spread, and over 349eefb and dcdde89 the same mean went both ways (-0.01 and +0.09, +0.02 and -0.08). So the tables stay: none of them feeds play, and the rebuild's first step refits them after the engine lanes.

Speed: the same. `match_check nfl 4000` takes 41.4 and 41.5 s against 41.3 and 41.6 s on main 70c8cbf (back to back, 4 cores, the same build settings; the same over dcdde89 and 349eefb, and 25.9 and 26.4 s against 26.0 and 26.2 s on a faster machine before play action merged).

Tried and dropped (NFL, seed 1):

- **The first loose ball:** a pop of 1-6 yards a second carried a third of the pile's way, slowing by e^(-1.5 t), and every man within 1.5 yards going for it at once. The fumbling side, nearest the pile, covered most of them: the defense got 33% (real 45.5%) and scored 0.0095 a team game. A livelier ball (1-12 yards a second with the pile's full way, slowing by e^(-0.5 t)) gave 40% and 0.027, and a man with his back to it going 0.6 s later and a fall on it that holds 60% of the time brought it to 43.5%. The NFL's touch on the ground (a man within a yard before he is up) took the defense's recoveries advanced from 51% to 32%.
- **Fewer get-ups in the NFL:** a touch within 2 or 2.5 yards: 21-29% of the defense's recoveries advanced, but return touchdowns fell to 0.019. A slower get-up (1 s): 31% and 0.024. A touch on the ground within 1.5 yards: 25%.
- **Breaking up blocks within 4 yards of the ball (3 kept), and the fumbler going after it at 0.7 s (0.55 kept):** no gain.
- **The hit he never sees** (1.6-2 times the tackle's fumble from outside his view): catches fumbled about as often as runs (0.89% and 0.83%, real 1.13% and 0.75%). The ball not yet tucked away after a catch splits them (`FRESH_CATCH`).
- **College's traffic:** the NFL's 2 yards: FBS ran back 0.015 a team game for scores. 1 yard: 0.034, but lost fumbles fell to 0.40 as defenders ran at balls the offense dove on. A man of the other side as near as him plus a quarter yard: 0.030 and 0.42; plus a yard: 0.015 and 0.42. Half a yard keeps lost fumbles at real.

Still off (`GAPS.md`): fumbles go out of bounds less than half as often as real (3.8%, real 8.5%: plays seldom end near the sideline, designed runs out of bounds 1.1% against 5.5%); the defense gets 46% of sack fumbles (real 50%) and the passer 21% (15%); catch fumbles are run back 44% of the time (real 31%) and sacks' 22% (33%); fumbled snaps are lost 15% of the time (25%); a passer hit as he throws never fumbles (passes without a catch fumble 0.021 a team game, real 0.036, all on interception returns); a quarterback's designed runs fumble a third of the real rate (0.66%, real 2.12%); a muffed punt is still a roll (lane EG3); a fumbled return the return team downs in its own end zone is a touchback, not a safety; and FBS runs back 0.021 a team game for scores against 0.044 real.

## Each level's rule book and the flags (engine gaps lane EG5, 2026-11-18)

Engine gaps items 23-28 (`docs/design/engine-gaps.md`): penalty yards and first downs by level, state and era, half the distance near the goal lines, kick rules by level, ineligible downfield (minor bug 406), pass interference at the real rate and size, the fouls the engine lacked, and illegal blocks in the back, which ran about ten times real. Cain agreed to the engine gaps on 2026-11-17 and gave full proxy; every size below is proposed, not confirmed. How each piece works is on the how-it-works page (`docs/reference/how-it-works/match-engine.md`, "Penalties", the pass rush, the throw and the catch).

What was wrong on main (4b7fb86, with scheme-fit lane SF1; 2,000 `AFM_GAPS=1 match_check` games at the NFL and FBS and 1,000 at high school, seed 1; the NFL's real is nflverse 2021-25 on scrimmage plays):

- **One table marked every foul at every level** (`FoulKind::yards`): no book and no era; half the distance only when the full yards reached the goal line, and then from the line of scrimmage; no safety for the offense's foul in its own end zone (0 a team game, real NFL 0.0044); the defense's holding 5 yards at every level; pass interference near the goal at the 1 at every level; and a personal foul after a gain booked the play's gain in its penalty yards too (19.5 yards an accepted unnecessary roughness, real 13.1).
- **Fouls the engine never called:** illegal contact, illegal use of hands, neutral zone infractions, the defense's twelfth man, kickoffs out of bounds and short of the landing zone (all 0; real NFL 0.121, 0.110, 0.226, 0.068, 0.070 and 0.079 a team game), and unsportsmanlike conduct after the play (0.003, real 0.025). Every offside killed the play, so there was no free play (0.002 a team game).
- **Ineligible downfield** (minor bug 406): 0.339 a team game at the NFL (real 0.106), 0.482 at FBS and 0.656 at high school, all on RPOs. Every lineman past the limit was flagged, a won pass block drove its man back downfield, pass blockers walked up to meet a blitzer and took him on 2-3 yards downfield (a pass thrown past the line found one there on a dropback in seven), and an RPO lineman moving on a slant had to turn to stop at the limit and ran a yard or two past it on a fifth of RPO throws.
- **Illegal blocks in the back** 0.281 a team game (real 0.031): every block from behind was thrown, and 7.5% of them were flagged.
- **Pass interference** was flagged 0.667 a team game and accepted 0.372 (real 0.561 and 0.497): the contact changed no catch, so the offense declined 44% of the flags, and the accepted ones came out 11.7 yards (real 15.8).
- **The rest off:** the offense's holding 1.141 and the defense's 0.506 (real 0.893 and 0.322), offside 0.524 (0.253), illegal formation 0.422 (0.224), false starts 0.935 (1.131) and delays 0.206 (0.295), with third and fourth down's false starts at first down's rate (real 1.76 and 4.7 times it), and intentional grounding 0.017 (0.077), a flat 8% of the pocket's throwaways and never outside it.
- **High school in the world** played today's book in every season (`Rules::for_level`), so targeting was called before 2013, and Texas and Massachusetts got only college hash marks (Massachusetts in every season), not the college book.

What changed (`game/fouls.rs`, new; `game/flow.rs`, `game/scrimmage.rs`, `game/sim.rs`, `game/kicking.rs`, `game/rules.rs`):

- **Three books** (`fouls.rs`): the NFL's, the NCAA's and the NFHS's, each foul's yards, first down and loss of down by season (`penalty`). Texas high schools play the NCAA's every season and Massachusetts through 2018 (`Rules::high_school`); the other leagues keep the NFL's. World high school games read the season's rules (`world/hs_year.rs`, `world/front_office/school.rs`).
- **Marking** (`defense`, `offense`, `half`): the previous spot, or the end of the run for a defensive foul during a run that ended past the line and for any personal foul after a gain; the offense's hold or block from the previous spot or its own spot as the book says; half the distance at every level; pass interference's own spots near the goal; a safety for the offense's hold, illegal block or grounding in its own end zone; live-ball fouls at the snap (the offside a free play in the NFL and college, encroachment under NFHS rules).
- **Kicks:** a missed field goal by book (`missed_fg_spot`), a kickoff out of bounds by book and era (`kick_out_spot`), both kickoff fouls booked.
- **Ineligible downfield at its roots:** a pass blocker never sets past 0.4 yards behind the line and lets a blitzer come to him (`PASS_SET`); a won pass block holds its man where they met; an RPO's line stops 0.3 yards short of the limit give or take his feel (`RPO_MARGIN`, `RPO_FEEL`, `RPO_IQ`), slowing along his own heading (`hold_at`); every pass thrown past the line is checked; the crew flags a man plainly past the limit (`IDF_SEEN`, `IDF_CLEAR`), with no Football IQ in its eye.
- **Pass interference:** from a yard past the line, sized by air yards (`PI_DEPTH`); the contact hinders the catch (`PI_HINDER`) and is flagged when the ball falls incomplete and on 12% of the catches made through it (`PI_CAUGHT`); the push-off helps the catch and is always flagged (`OPI_HELP`).
- **The hand fight** in man coverage: one draw for holding, illegal use of hands and (NFL) illegal contact (`HAND_FIGHT`, `DEF_HANDS`, `CHUCK`); the offense's flagged holds include his hands to the face (`OFF_HANDS`).
- **Illegal blocks in the back:** a blocker lets a man with his back turned go unless he throws it (`IBB_THROW`), and one in two he throws is flagged (`IBB_SEEN`).
- **Pre-snap:** the twelfth man (`TWELVE_MEN`, `TWELVE_DEAD`); the jump split into neutral zone infractions before the snap and offside at it (`JUMP_EARLY`, `JUMP_EARLY_HARD`); delays by down and by the quarterback's Football IQ (`DELAY`); false starts by the cadence on third and fourth down (`CADENCE`, and `CADENCE_COLLEGE` under the NCAA's and the NFHS's books); the delay to back up a punt from the other side's 30-50 by distance (`BACK_UP_MID`, `BACK_UP_SLOPE`, `BACK_UP_COLLEGE`); fouls before a kick's snap (`kick_snap_foul`); and the hard count on fourth and short (`hard_count_gambit`).
- **The hit fouls:** the NFL's men, who are fined, draw 0.7 as many of them on the same hits (`fouls::hit_restraint`); face masks 0.0122 and late hits 0.0196 a tackle before the man's own read (0.0087 and 0.023), the flagged hits at the catch point and on a slide at 0.85 of before (`CATCH_HIT_FOUL`, `SLIDE_HIT`); roughing the passer by the book's share of the same hits (`fouls::roughing_share`).
- **Targeting:** the tackle's helmet-first share is 0.09 (`HIGH_ON_TACKLE`, 0.07 before). At 0.07, with the late hits at FBS's real count, FBS targeting fell to 0.029 a team game, under its real band (cfbfastR 2022-2025, 0.031 to 0.043); at 0.10 it came to 0.042, and at 0.09 it is 0.037 (0.040 over main's SF1). Nearly all of it comes from tackles: the catch point gives 0.002 flagged hits a team game and scrambles 0.012. The NFL has no targeting foul, so it plays the same games either way.
- **Unsportsmanlike conduct after the play** (`after_play_conduct`), and grounding read from the passer's sense and where the ball goes, in the pocket and outside it (`GROUND_POCKET`, `FLICK_CARRY`, `GROUND_SEEN`).
- **Tools:** `match_check`'s penalty rows are the real kinds, each on its own (nflverse 2021-25; the 2025 counts for the kickoff fouls), and `AFM_GAPS=1` adds "Rule books and flags": each kind flagged, accepted, declined, its yards and first downs against real; safeties on the offense's foul; pass interference by air yards; ineligible downfield by call and where the lineman stood; kickoffs; free plays; throwaways and grounding; pre-snap fouls a snap by down; delays on fourth down; fouls before a kick's snap; the hard count; the defense's hit fouls by where.

2,000 `AFM_GAPS=1 match_check nfl` games, main and after (seed 1, the test rosters; real is nflverse 2021-25 on scrimmage plays, and 2025 for the kickoff fouls):

| NFL, a team game | main | after | real |
|---|---|---|---|
| accepted penalties on scrimmage plays | 5.62 | 5.08 | 5.24 |
| their yards | 48.1 | 40.5 | 42.6 |
| offensive holding | 1.141 | 0.917 | 0.893 |
| false start | 0.935 | 1.145 | 1.131 |
| pass interference, accepted (flagged) | 0.372 (0.667) | 0.565 (0.600) | 0.497 (0.561) |
| the same, yards an accepted one | 11.7 | 13.9 | 15.8 |
| defensive holding | 0.506 | 0.324 | 0.322 |
| illegal contact | 0 | 0.114 | 0.121 |
| illegal use of hands | 0 | 0.099 | 0.110 |
| offside | 0.524 | 0.229 | 0.253 |
| neutral zone infraction | 0 | 0.255 | 0.226 |
| delay of game | 0.206 | 0.269 | 0.295 |
| unnecessary roughness | 0.431 | 0.275 | 0.271 |
| the same, yards an accepted one | 19.5 | 12.7 | 13.1 |
| roughing the passer | 0.226 | 0.202 | 0.181 |
| illegal formation | 0.422 | 0.239 | 0.224 |
| offensive pass interference | 0.112 | 0.128 | 0.128 |
| face mask | 0.116 | 0.116 | 0.124 |
| ineligible man downfield | 0.339 | 0.053 | 0.106 |
| too many men on the field | 0 | 0.063 | 0.068 |
| intentional grounding | 0.017 | 0.040 | 0.077 |
| illegal block in the back | 0.281 | 0.033 | 0.031 |
| unsportsmanlike conduct | 0.003 | 0.022 | 0.025 |
| kickoff out of bounds | 0 | 0.081 | 0.070 |
| kickoff short of the landing zone | 0 | 0.085 | 0.079 |
| safeties on the offense's foul | 0 | 0.0037 | 0.0044 |
| false starts a snap, downs 1-4 | 0.0135, 0.0136, 0.0142, 0.0120 | 0.0127, 0.0137, 0.0252, 0.0500 | 0.0132, 0.0125, 0.0232, 0.0620 |
| delays a snap, downs 1-4 | 0.0031, 0.0029, 0.0029, 0.0030 | 0.0015, 0.0023, 0.0056, 0.0441 | 0.0014, 0.0022, 0.0053, 0.0816 |
| points | 25.49 | 26.09 | 22.51 |
| sacks; pressure, of dropbacks | 1.65; 20.2% | 1.54; 20.0% | 2.41; 22.2% |
| third and fourth down converted | 42.5%, 53.6% | 42.7%, 52.7% | 39.8%, 53.7% |

Before a kick's snap, a kick snap (false start, delay, the other side's jump; main had none): punts 0.0137, 0.0060, 0.0010 (real 0.0128, 0.0055, 0.0015), field goals 0.0029, 0.0047, 0.0025 (0.0039, 0.0043, 0.0026), tries 0.0029, 0.0038, 0.0028 (0.0036, 0.0036, 0.0015). The hard count on fourth and short: 0.025 a team game, 0.024 of them ending in a delay (real NFL delays of this kind 0.024). Free plays (a live-ball offside and the play ran): 0.361 a team game, the offense taking the offside on 63% of them.

Points rise because the offense's drive-killing fouls fall to real: its holding, ineligible downfield, illegal formation and blocks in the back come to 1.24 a team game against main's 2.18, and the defense's accepted pass interference gives 7.9 yards a team game against main's 4.4. On seeds 2-4 (2,000 games each) main to after is +0.51, +0.39 and +0.71 points (+0.60 on seed 1), so EG5 adds about half a point a team game; before main's SF1 merge it was +0.54 to +0.72 on seeds 2-4. Points were already high on main (25.5, real 22.5), which is EG6's (the red zone: 64.9% completions on main and 64.0% after, real 53.0%). Sacks fall 0.05-0.10 a team game at every seed: a pass blocker now lets a blitzer come to him, so fewer blitzers beat a blocker who had walked up to meet them (sacks a pressure 0.209 to 0.199, real 0.283); sacks are SF8's and EG7's, and SF1 already took them below real (1.65 on main, real 2.41). Accepted penalties on scrimmage plays come to 4.98-5.08 a team game over the four seeds (real 5.24).

FBS (2,000 games, seed 1; real is cfbfastR 2023-24, FBS against FBS): accepted penalties 6.73 a team game to 5.86 (real 5.25 in 2023 and 5.54 in 2024, cfbfastR's penalty flag) and their yards 54.9 to 48.0 (40.6 and 45.5, but 0.45-0.68 accepted flags a team game carry no yards in cfbfastR, so the real yards run higher). By kind, main to after against the plays whose text names the foul (kick plays in, so a rough count): holding by both sides 2.05 to 1.56 (about 1.15), false starts 1.11 to 1.33 (1.19), pass interference by both sides 0.38 to 0.59 (0.67), ineligible downfield 0.48 to 0.07 (0.11), illegal blocks in the back 0.37 to 0.04 (illegal blocks of every kind 0.08), delays 0.33 to 0.30 (0.31), unsportsmanlike conduct 0.01 to 0.17 (0.23), face masks 0.13 to 0.19 (0.21), roughing the passer 0.22 to 0.11 (0.12), targeting 0.038 to 0.040 (0.031-0.043 over 2022-25), grounding 0.044 to 0.048 (0.080). False starts by down: 0.48, 0.35, 0.37 and 0.11 a team game (real 0.40, 0.32, 0.31 and 0.15; main 0.49, 0.36, 0.23 and 0.03); delays on fourth down 0.10 (0.15), 0.03 of them to back up a punt (0.05). Points 23.9 to 25.5 (real 27.0), third downs converted 39.6% to 41.2% (40.3%), sacks 1.85 to 1.71 (2.00).

High school (1,000 games, seed 1, the NFHS book; no real count by kind): accepted penalties 6.18 a team game to 5.12 and their yards 49.8 to 42.5; ineligible downfield 0.66 to 0.06, illegal blocks in the back 0.34 to 0.04, holding by both sides 1.88 to 1.51, illegal formation 0.55 to 0.24, encroachment (booked as offside) 0.44 to 0.51, unsportsmanlike conduct 0.01 to 0.13, targeting 0.036 to 0.034, and the defense's twelfth man, illegal participation at 15 yards, 0.06. Points 18.6 to 19.8, third downs converted 36.4% to 38.2%. Texas and (through 2018) Massachusetts take the NCAA's book (`Rules::high_school`), which `match_check hs` doesn't play.

The coaches' call table (`call_values 2000`, 2,000 snaps a cell, main and after): EG5 moves its run cells by -0.005 on average with a spread of 0.18 (at most 0.47), the noise of a build's new draws, and its straight pass cells by -0.09 with a spread of 1.16 (up to 9.5): little against four rushers (a spread of 0.4-0.7 by coverage) and most against blitzes (Cover 0 3.1, the man blitzes 1.4-1.8), where a pass blocker now waits for the blitzer. SF1's faked rows move the same way: what the fake adds (a faked cell less its straight one) moves by a spread of 0.5-0.9 against four rushers and 1.3-3.3 against blitzes. The run rows stay. The pass rows, straight and faked, stay too: the straight rows already differed from the tool by a spread of 3.4 on main, each faked row is built on its straight row (`tools/call_values/compose.py`), and minor bug 405 holds their refit for SF5 or SF10 (the storm of picks in the tool's two fixed teams), which comes after this lane. The analysts' tables (`world/tally/decide.rs`, `decide_fit -- 1500 1`) are refitted on the merged branch; no play reads them.

Speed: the same within noise. Back to back on the same machine over main 4b7fb86 (main, then the branch, twice at each level, `AFM_GAPS=1`, 4 cores), the NFL took 9.2 and 9.2 ms a game on main against 9.2 and 9.3 after, FBS 10.3 and 9.8 against 10.2 and 10.0, and high school 6.9 and 7.4 against 6.9 and 7.0. Before main's SF1 merge it was 9.0 and 9.0 against 8.9 and 9.1 at the NFL, 9.6 and 9.3 against 9.6 and 9.4 at FBS, and 6.7 and 6.9 against 6.7 and 7.0 in high school.

Measured again over main 569e61f (with engine gaps items 29 and 30, the inside line's rotation and the weather's size, scheme-fit lane SF2, personnel that fits the play, minor bugs 409-412, the play action leftovers, the 14-year-olds' recentred habits, which redraw the test rosters' flaws, and engine gaps lane EG2, ball security, which merged first), the same games and seeds; the real seasons' range is nflverse 2021-25, each season on its own:

| NFL, a team game | main | after | real |
|---|---|---|---|
| accepted penalties on scrimmage plays | 5.63 | 5.01 | 5.24 (4.95-5.62) |
| their yards | 48.5 | 40.3 | 42.6 (40.0-45.2) |
| offensive holding | 1.113 | 0.902 | 0.893 (0.825-1.028) |
| defensive holding | 0.472 | 0.303 | 0.322 (0.287-0.387) |
| pass interference, accepted (flagged) | 0.388 (0.702) | 0.560 (0.597) | 0.497 (0.561) |
| the same, yards an accepted one | 11.7 | 14.4 | 15.8 |
| ineligible man downfield | 0.367 | 0.051 | 0.106 |
| illegal block in the back | 0.300 | 0.032 | 0.031 |
| unnecessary roughness; yards an accepted one | 0.426; 19.7 | 0.259; 12.9 | 0.271; 13.1 |
| points, seeds 1-4 | 26.63, 27.54, 26.90, 25.91 | 27.73, 28.33, 27.59, 26.55 | 22.51 |
| sacks; third downs converted (seed 1) | 1.38; 44.1% | 1.25; 45.2% | 2.41; 39.8% |

- On the branch no kind of foul moves more than 0.048 a team game from the runs over ad17262 at any level (FBS's unnecessary roughness), and 0.044 from those over 4b7fb86, so the tables and counts above stand. EG2's flag on the defense's own return is marked by the book's yards (10 at every level) and kept out of the books' marking of the down. On main, SF2's RPO throws and the play action leftovers lift ineligible downfield at FBS from 0.48 to 0.59 and in high school from 0.66 to 0.71; the branch's are 0.088 and 0.062.
- The holding pair rides the test league's man coverage. Seed 1's league plays man on 31% of dropbacks, as real does (NGS 2025), and its holdings sit in the real seasons' range (the offense's 0.902, the defense's 0.303). Seeds 2-4 play man on 25-28%, so the defense's holding comes to 0.20-0.26 and the offense's to 0.98-1.06 (main 0.31-0.38 and 1.24-1.31). The engine's defense holds only in man coverage, while real holds come against zone and on runs too, so the count moves with the man share more than real (`GAPS.md`). The hand fight stays sized on seed 1.
- Over the four seeds, accepted penalties on scrimmage plays come to 4.95-5.05 a team game and their yards to 40.3-40.9, at the low end of the real seasons, where 2022 had 4.95 and 40.0. EG5 adds 0.64-1.10 points a team game at the NFL, 0.81 on average (0.39-0.71 over 4b7fb86, 0.63-0.76 over 590101e, 0.58-0.98 over dcdde89, 0.40-0.91 over ad17262), and takes 0.06-0.13 sacks off. Safeties on the offense's foul come to 0.0013-0.0040 a seed, 0.0028 on average (real 0.0044, from 12 such safeties in five seasons).
- FBS: accepted penalties 6.89 to 5.83 a team game (seeds 2 and 3 after: 5.80 and 5.97), their yards 56.4 to 47.7, points 25.04 to 26.25 (+1.21 to +1.52 over seeds 1-3; real 27.0), and targeting 0.035, 0.039 and 0.041 on seeds 1-3, 0.038 on average, inside its real band (0.031-0.043; main 0.040, 0.033 and 0.039).
- High school: accepted penalties 6.16 to 5.11 and their yards 49.2 to 42.6; points 19.2 to 20.9.
- The call table over EG2's rebuilt run rows: EG5 moves the run cells by -0.05 on average with a spread of 0.18, the most BuckSweep's six (-0.39, up to -0.76), and the pass cells as before, little against four rushers (a spread of 0.5-0.6) and most against blitzes (1.3-1.7 against the linebacker blitzes, 3.4 against Cover 0's), where the pass rows wait for minor bug 405. It stays. The analysts' tables are refitted again on this merge (`decide_fit -- 1500 1`).
- Speed, back to back (main, then the branch, twice at each level): the NFL 6.6 and 6.6 ms a game on main against 6.5 and 6.6 after, FBS 7.1 and 7.1 against 7.1 and 7.3, high school 4.7 and 5.0 against 4.9 and 5.1.

Tried and dropped (seed 1):

- **A flat chance to back up a punt** (0.32 a punt from the 30-50): it backed up from the 48 as often as from the 31, where real coaches do it half the time from the 30-35 and one time in twenty from the 46-50.
- **Stopping an RPO lineman by his run along the field alone:** he kept his heading, so on a slant he had to turn to stop and ran past the limit; he now slows along his own heading.
- **Football IQ in the crew's eye for ineligible downfield** (main's 0.4-1.4): the crew sees where he is, not what he knows; his Football IQ now keeps him back (`RPO_IQ`).
- **The first sizes for the fouls before a kick:** the kick unit's false starts came out 30-50% high, and one delay share for every kind of kick missed real's split (punts most, tries least).
- **The NFL's cadence at FBS:** third down's false starts came out 32% high (real FBS third down is 1.65 times first down, the NFL 1.76).

Still off, with their owners (each in `GAPS.md`):

- **Ineligible downfield** 0.051 (real 0.106; 0.053 over 4b7fb86): RPOs draw 0.042 and screens 0.020, nearly all play-action screens, whose fake (SF1) holds the throw while the line releases; dropbacks, other play action and scrambles give almost none, since a pass blocker sets behind the line, SF1's play-action line pass-sets behind it too, and nobody drifts downfield while the passer scrambles (EG7).
- **Intentional grounding** 0.036 (0.077; 0.040 over 4b7fb86): flagged at the real rate a throwaway, but throwaways are a third of real (EG7).
- **Delays on fourth down** 0.075 (0.142; 0.084 over 4b7fb86): the back-ups are 0.020 (0.066), at the real share from each spot but with fewer punts from the other side's 30-50 (EG3's long field goals, EG4's fourth-down sums).
- **Fouls on kick plays** are drawn only before the snap and on kickoffs, so all accepted penalties come to 5.18 a team game (real 6.00; 5.25 over 4b7fb86): the live-ball kick fouls and bug 403's return fouls are EG3's.
- **Roughing the passer** 0.202 (0.181), up from 0.179 before main's SF1, whose play action turns sacks into throws; the hits on the passer are SF8's.
- **Neutral zone infractions** 0.255 (0.226) and **kickoffs out of bounds** 0.081 (0.070) run 13-16% high.
- **The hit at the catch point** barely draws a flag, so college targeting comes almost all from tackles (SF5's catch point).
- **FBS:** holding high and unsportsmanlike conduct low against cfbfastR's play text, and false starts 10-20% high before fourth down.
- **Pass interference** shares the catch point with SF5: whichever merges second measures it again.
- **The defense holds only in man coverage**: real holds also come against zone (22% of the defense's holds on dropbacks, with 2023-24's labels, which call 45% of dropbacks man) and on runs (15% of all its holds; nflverse with FTN's labels, 2023-24). They belong with the coverage jobs (SF7).
## The kicking game: long kicks, blocks, returns and the weather (engine gaps lane EG3, 2026-11-19)

Engine gaps items 11-14 and minor bugs 403 and 408 (`docs/design/engine-gaps.md`; Cain agreed to the audit on 2026-11-17, and the sizes are proposed, not confirmed). Long field goals are made at the real rate and the coach's estimate is the kick's own; a kick's rush plays out against its protection and a blocked kick is a live ball under each level's kick rules; returns break long ones more often; the range reads the weather; and a return's fouls are enforced. How each part works is in `docs/reference/how-it-works/match-engine.md`, "Kicking", with its formulas.

What was wrong on main (2,000 `AFM_GAPS=1 match_check nfl` games, seed 1):

- **The coach's make estimate was its own model.** `fg_make` had a wider spread at short range, a longer leg and no weather, so the coach tried twice the real number of 55-yard kicks, which the roll made 71% of the time (real 59.7%). FBS kickers made three in four from 40-49 (real two in three).
- **The punt's value read a touchback from inside the other side's 41** (`fourth_down`), so a coach at their 40 weighed a 57-yard try against a punt to the 20, where real punts from there leave them at the 12.
- **A block was a formula** on the rush unit against the protection, and a blocked punt landed at a random spot, where the punting team fell on it 40% of the time and got a first down (bug 408). Nobody ever ran a blocked kick back.
- **Returns kept the full dive.** The scrimmage tackle's reach came down from 2.5 to 2.0 yards with minor bug 162; returns kept 2.5 (`OPEN_REACH`), so the first wave closed every crease: kickoff returns of 40+ were 1.6% of returns (real 5.0%), punt returns of 20+ 2.6% (8.0%).
- **The range read only his leg's rating**, so a coach tried 15% more long kicks in a 17 mph wind (real 13% fewer).
- **A return's flag was thrown away** (bug 403): no yards, no ejection, no fine, no spark.

What changed:

- **One model for the kick and the estimate** (items 11 and 14; `kicking::fg_spread`, `fg_reach`; `flow::fg_make_in`, `kicker_range`). The kick's miss spread grows with the distance and, once a kick needs more than 80% of his reach, with the square of the extra share (`KICK_STRAIN` 28, `KICK_EASY` 0.8); the wind still widens it 1.2% a mph. His reach is his Kick Power in the day's air, cold, rain and the wind along the kick. The staff's estimate reads the same spread and reach (and leaves out only his nerve, his season's form and a bad snap), and his range is six yards inside his reach in today's weather. High school posts are 23 feet 4 inches apart (NFHS), wider than college's and the NFL's 18 feet 6 inches.
- **Long kicks picked like real coaches pick them** (`calling::fourth_down`, `punt_start`, `LONG_KICK_CAUTION`). A punt leaves the other team where real ones do: a 41-yard net, and about 88 yards out once the punter is past midfield (nflverse 2024-25). Past 50 yards a kick must beat the punt by 0.04 points a yard, half to one and a half times that by the coach's boldness: real NFL coaches kick from their 39-41 a quarter of the time where the expected points say kick.
- **The rush on a field goal or extra point** (item 12; `kicking::place_block`; the units in `flow::kick`). The operation time comes from the snapper's Long Snapping and the kicker's Get Off; the ball rises at an angle from his Kick Trajectory, lower on a long kick and off a bad snap. The two heaviest men inside push the middle back by their power against the wall's, and block it when a hand reaches above the ball where it passes; the two fastest of the rest come off the edges, a clean beat against the end man by their rush and his blocking, then a race to the kick point against the operation. On an all-out rush (15% of extra points; field goals from 30% at 35 yards, 2.5% more a yard, 3-70%) a corner or safety overloads one side, two against one. The rush is three tackles, two edges, two linebackers, and the corners and safeties behind it.
- **The rush on a punt** (`kicking::punt_block`). The front (the snapper and the four heaviest) and the shield behind it: a rusher beats his man cleanly by rush against blocking, the shield picks up a free man by its blocking against his, and a free man blocks it if he reaches the kick point (the punter's depth, 15 yards in the NFL, 14 in college, 12 in high school, less his two steps) before the ball is off. On 15% of punts the rush comes all out; on a return call half the front only holds its man up.
- **A blocked kick is live** (item 12; `kicking::blocked_live`; `flow::blocked_kick`, `blocked_try`; bug 408). The ball comes off the blocker's hand and plays out on lane EG2's loose ball with all 22 men. Behind the line either side may pick it up and advance it; the kicking team keeps a fourth-down kick it recovers there only if it reaches the line to gain (on an earlier down it has the next down). Past the line it is a kick again: only the receiving team advances it, and the kicking team keeps it only after the receiving team touched it. On a try the defense scores two by running it back where the level's era allows (the NFL from 2015, college from 1988, high school never), and a kicking team downed in its own end zone gives one.
- **Returns read the scrimmage tackle's reach** (item 13; `kicking::kick_sim`, `SCRIMMAGE_REACH`). The kickoff coverage (two safeties and the kicker behind the first wave) is unchanged, so the average return stays near real.
- **Fouls on returns** (bug 403; `flow::return_fouls`; `kicking::run_return`, `RET_GRAB_KICKOFF` 1.2, `RET_GRAB_PUNT` 5.4, `RET_IBB` 6.5, `RET_PERSONAL` 0.6). A beaten return blocker grabs 1.2 times as often as on a scrimmage down on a kickoff and 5.4 times on a punt (the jammers on the gunners), throws a block in the back 6.5 times as often as lane EG5's share on a scrimmage down (cover men run past a return's blockers), and 60% of a scrimmage down's personal fouls stand. The receiving team's foul is marked from its spot (or where the return ended, if short of it), half the distance at most, a safety from its own end zone, and a score on the return comes off; the kicking team's from the end of the return. Ejections, fines and sparks come as on any snap.
- **Tools.** `AFM_GAPS=1 match_check` prints the field goals by distance with the block share, blocked kicks with what became of them, return yards by percentile, the accepted fouls on kicks by kind, the 55+ tries late in a half, and fourth downs from the other side's 36-44; `AFM_TEMP` sets the temperature, as `AFM_WIND` sets the wind. `tests/kick_game.rs` checks that the estimate falls with the distance and into the wind, the punt's value and the all-out share.

2,000 `AFM_GAPS=1 match_check nfl` games on seed 1, main (569e61f, the same engine as ccdb976) and the lane, against nflverse 2021-25 (fouls 2025, fourth downs 2024-25):

| NFL | main | after | real |
|---|---|---|---|
| field goals made under 30 / 30-39 / 40-49 | 99.1 / 94.4 / 83.5% | 97.9 / 91.9 / 81.3% | 98.0 / 93.4 / 79.8% |
| 50-54 / 55+ | 76.1 / 71.1% | 70.5 / 66.0% | 73.4 / 59.7% |
| tries of 55+ a team game | 0.277 | 0.184 | 0.15 |
| fourth downs from their 39-41: kick / punt / go | – | 0.37 / 0.28 / 0.34 | 0.24 / 0.33 / 0.43 |
| blocked: field goals / punts / extra points | 0.48 / 0.21 / 0.34% | 1.82 / 0.51 / 0.62% | 1.89 / 0.48 / 0.70% |
| blocked kicks a team game | 0.026 | 0.069 | 0.072 |
| blocked punts run back for a score / safeties / kept by the punting team | 0 / 0 / 37% | 19.7 / 4.5 / 0% | 33 / 6% / – |
| blocked field goals advanced / for a score | 0 / 0 | 33 / 2.9% | – / 7.9% |
| kickoff return average / 40+ | 25.6 / 1.6% | 26.9 / 2.9% | 25.9 / 5.0% |
| punt return average / 20+ | 8.2 / 2.6% | 9.4 / 4.7% | 9.45 / 8.0% |
| return touchdowns a team game | 0.013 | 0.029 | 0.024 |
| accepted fouls on kicks a team game: kickoffs / punts returned | 0 / 0 | 0.365 / 0.194 | 0.361 / 0.195 |
| 50+ tries in a 17 mph wind, against 5 mph | +15% | -3% | -13% (15+ mph) |
| made in that wind | 68.8% | 62.1% | 78.0% |
| 50+ tries at 25 F, against 60 F | +3% | -25% | -26% (32 F or colder) |
| points a team game | 26.63 | 26.36 | 22.51 |

FBS (2,000 games, against the audit's real FBS rates): field goals made under 30 97.5% to 92.6% (real 92.1%), 30-39 89.2% to 78.6% (80.7%), 40-49 78.1% to 65.0% (65.2%), 50+ 65.2% to 54.9% (50.6%); tries of 50+ 0.163 to 0.051 a team game; field goals blocked 0.98% to 1.89%, punts 0.30% to 0.81%; kickoff returns of 40+ 0.2% to 1.0% (most kickoffs are touchbacks), punt returns of 20+ 3.4% to 5.2%, the punt return average 8.8 to 9.8; points 25.04 to 24.49. High school (1,000 games): field goals made under 30 91.1% to 87.5%, 30-39 80.4% to 70.4%, 40-49 65.6% to 51.9%; field goals blocked 1.4% to 8.2%, punts 0.55% to 1.70%; points 19.22 to 19.23 (with no real high school count by distance). Real NFL kicks that are not returned draw 0.34 more fouls a team game (kickoffs out of bounds or short of the landing zone, and holding on fair catches), which the engine doesn't flag.

Measured again over main a7c9bf6, with lane EG5's rule books and flags (2,000 NFL games, seed 1). EG5's block in the back (a blocker lets a man with his back to him go unless he throws it, `IBB_THROW`) took a return's blocks in the back to a seventh of real (0.005 a team game on kickoffs, real 0.033; 0.009 on punts, real 0.053 with blindside blocks), and its lower holding share left punt returns' holding at 0.073 (real 0.088). A return's blocker now throws it 6.5 times as often as on a scrimmage down (`RET_IBB`: cover men run past a return's blockers), and a punt's beaten blocker grabs 5.4 times (`RET_GRAB_PUNT`, was 4.5). The kicking game on the two lanes together: field goals made under 30 / 30-39 / 40-49 / 50-54 / 55+ 97.7 / 93.0 / 81.5 / 70.3 / 65.6%, tries of 55+ 0.185 a team game; field goals, punts and extra points blocked 1.93, 0.42 and 0.74%, blocked kicks 0.071 a team game; kickoff returns 26.4 yards and 2.5% of 40+, punt returns 9.2 yards and 4.4% of 20+, return touchdowns 0.022 a team game; fouls on returned kicks: kickoffs' holding 0.225 (real 0.223), blocks in the back 0.034 (0.033), illegal use of hands 0.008 (0.007) and the coverage's unnecessary roughness 0.017 (0.018), punts' holding 0.084 (0.088), blocks in the back 0.059 (0.053) and the coverage's face masks 0.003 (0.012); points 27.15 (27.73 on EG5's own run over 569e61f; a game's draws part from the first snap). FBS: field goals made 91.4 / 79.6 / 62.8 / 58.3% (real 92.1 / 80.7 / 65.2 / 50.6%), blocked 2.4%, punts 0.8%, points 26.0. High school: 88.6 / 68.2 / 51.4%, blocked 8.2%, punts 1.9%, points 20.6.

The analysts' tables (`world::tally::decide`, `decide_fit 1500 1` over EG5): the kick tables are refitted on the lane, since it moves them. Field goals by distance from under 30 to 50 and more: the NFL 0.981, 0.954, 0.909, 0.834, 0.769 and 0.663 (EG5's fit 0.988 to 0.741), FBS 0.932 to 0.569 (0.974 to 0.659), high school 0.872 to 0.452 (0.943 to 0.658); extra points the NFL 0.955 (0.968), FBS 0.966 (0.989); where a punt leaves the other side moves up to 3.6 yards a band. The conversion, gain, two-point and win expectancy tables stay EG5's fit: the lane changes no scrimmage play. One FCS cell comes out uneven (45-49 yards 0.600, 40-44 0.588), from few kicks.

The coaches' call table (`call_values.csv`) is kept: the lane changes no scrimmage play, and a fourth down's call is the coach's sums, not the table.

Speed: the same. Over main 569e61f, `match_check nfl 2000` took 16.8 s against main's 17.0 s, back to back on the same machine; over EG5 the same run took 19.2 s (main a7c9bf6 not timed here).

Still off (`GAPS.md`): kickoff returns of 40+ and punt returns of 20+ at about half of real; no fouls on punts that are not returned; the cover men's face masks on punts at a fifth of real; the half's last kick reads the range alone (55+ tries in the last 30 seconds 0.030 a team game, real 0.060, lane EG4); long kicks in the wind tried 3% less (real 13%) and made 62% (78%); NFL kicks of 55+ made 66% (59.7%); blocked kicks run back for scores below real; high school field goals blocked 8.2% of tries, with no real rate to check; FBS tries few kicks of 50+ (no real count yet).

## The end of halves and game management (engine gaps lane EG4, 2026-11-19)

Engine gaps items 15-22 (`docs/design/engine-gaps.md`; Cain agreed to the audit on 2026-11-17, and the sizes were picked under his proxy: every size below is proposed, not confirmed). The targets are nflverse 2021-25.

What was wrong on main (`AFM_GAPS=1 match_check nfl 2000`, seed 1):

- **A half's last snap was an ordinary call.** Out of range the last snap was the usual two-minute call, with no Hail Mary and no laterals, and a tied team never knelt.
- **The clock before halftime.** The defense never called a timeout in the second quarter (0.009 a game in its last 4:00, real 1.35), and every offense hurried in its last 2:00 whatever its spot or score. Your own timeout in a live game gave back no seconds (minor bug 407).
- **Nobody protected a late lead.** The carrier's sideline choice never read the score or the clock: catches by the side ahead in the last 5:00 went out of bounds 22.6% of the time (real 14.7%).
- **Two-point tries** were 4.9% of tries (real 9.8%), from a fourth-quarter chart and a flat lean.
- **Fourth-down sums** were the NFL's at every level: a conversion chance from the yards to go only, a 41-yard net punt and the NFL's spot for a miss.
- **AI staffs never faked a kick**, and no fake field goal existed.
- **No untimed down at 0:00** after a foul, and no 10-second runoff.

What changed (the formulas are in `docs/reference/how-it-works/play-calling.md`, "Fourth down, tries, onside kicks and the clock", and `match-engine.md`, "The half's last play"):

- **The last snap's call** (`game/flow.rs`, `last_call`; item 15). With 6 s or less, or in range with the clock stopped and no timeouts: a kick in range (or a little past it when the kick is worth more than a Hail Mary), a kneel when the half is better run out (the second quarter, and a tied side at the end of regulation, by the spot and the time), fades from inside 30 yards out, a Hail Mary as far as the passer's arm gets the ball to the end zone, and, behind at the end of the game and too far for that, the laterals. With the clock running and no timeout the kicking unit can't get on, so the offense spikes it first and then kicks.
- **The Hail Mary** (`game/scrimmage.rs`, `hail_route`, `hail_defense`, `hail_throw`, `crowd_ball`, `longest_throw`). Four verticals flood the end zone against a dime Cover 4 that drops everyone to the goal line, the passer holds it until they get there, and the ball comes down in a crowd where the best high point gets a hand on it first. A typical NFL arm throws 65 yards in the air, 60 in college and 52 at high school.
- **The laterals** (`lateral`, `Job::PitchMan`; item 16). After a short cross the other backs and receivers trail the ball, and the man with it pitches back when a tackler closes; a pitch he can't gather is a live ball (item 4).
- **The last play has no clock to save** (`run_scrimmage_call`): no routes bent to the sideline, no carrier heading out of bounds and no check at the line. Before that, the laterals lost the trailers to the sideline routes (2% of the passes had a lateral).
- **The clock before halftime** (`after_snap_clock`, `Game::half_go`; item 17). The second quarter's last 2:00 are a hurry only for a side that can get to its kicker's range in the time (its timeouts, the two-minute warning and, in college and high school, the chains' stops counted); the rest play the half out. The defense saves time for its own possession with its timeouts after in-bounds plays on later downs. Your live timeout gives back the seconds since the whistle, less 3 for the sideline to call it (minor bug 407, fixed; `user_timeout`).
- **Protecting a late lead** (`game/flow.rs`, `clock_run`; `game/sim.rs`, `ClockRun`; item 18). The side ahead in the last 5:00 keeps the clock running: its carrier doesn't step out and shies from the sideline, and when a first down wins the game he goes down short of the goal line.
- **Two-point tries** (`game/calling.rs`, `go_for_two`; `game/tables.rs`, `real_two_point_habit`; item 19). The real habit by the score after the touchdown and the time (`data/real-world/nfl_two_point_habit.csv`), moved toward the numbers by a sharper or bolder coach the way fourth downs are.
- **Fourth-down sums** (`fourth_read`, `fourth_conv`, `fourth_ev_read`; item 20). The conversion chance reads the matchup at the point of attack, the punt is the punter's own net at his level's real gross punt, and a miss goes back the level's way (`miss_back`, the NFL's spot until item 25).
- **Fakes** (`fake_punt`, `staff_fake_punt`, `fake_field_goal`, `fake_fg_play`, `scrimmage::punt_spot`, `kick_spot`; item 21). AI staffs fake punts at the NFL's rate by yards to go, score and quarter, only when the return team shows the look and they read it; thrown fakes are the punter's. Every staff has a fake field goal.
- **The rules at 0:00** (`game/rules.rs`, `PeriodEnd`; `ends_period`, `runoff`; item 22). Each level's untimed down after an accepted foul as the clock runs out, by era, and the 10-second runoff.
- **The win chance** credits the throw the engine makes on the last snap (`game/win_chance.rs`, `last_shot`): fades, the Hail Mary to the typical arm's reach, then the laterals.
- **Tools.** `AFM_GAPS=1 match_check` adds the last-snap pool by call, Hail Marys by distance, laterals on the fourth quarter's last passes, timeouts in each half's last 4:00, the sideline when ahead late, fakes and untimed snaps. `play_trace` takes `AFM_HAIL=1` (the Hail Mary against its defense) and `AFM_HAIL=2` (the laterals).

2,000 `AFM_GAPS=1 match_check nfl` games on seed 1, main (a7c9bf6, with lane EG5, and this lane's counts) and after, against nflverse 2021-25:

| NFL | main | after | real |
|---|---|---|---|
| Hail Marys (35+ air yards, a half's last snap, 30-70 out): scored / picked | none called | 6.6% / 26.9% at the real mix of distances | 6.5% / 29.0% (62 throws) |
| Q4 last-snap passes from 40+ out with a lateral (tied or down 1-8) | 0 of 162 | 27.6% of 170 | 22% of 74 |
| tied at the end of regulation: kneels, share of the pool | 0 | 4.5% | 5.0% |
| the pool's snaps a game; touchdowns on them | 0.88; 0.7% | 0.50; 1.1% | 0.74; 2.5% |
| Q2 last 4:00 timeouts a game: defense / offense; games with the defense's | 0.013 / 2.50; 1.3% | 1.25 / 1.82; 70.9% | 1.35 / 1.54; 74.8% |
| Q4 last 4:00 timeouts a game: defense / offense | 2.00 / 0.54 | 1.92 / 0.74 | 1.83 / 0.77 |
| points a team game in the last 2:00 of Q2 | 3.29 | 3.24 | 3.05 |
| ahead in the last 5:00, out of bounds: designed runs / catches | 1.7% / 24.4% | 0.6% / 13.3% | 2.3% / 14.7% |
| the first three quarters, out of bounds: designed runs / catches | 2.1% / 26.0% | 1.5% / 24.8% | 5.8% / 21.3% |
| two-point tries a team game (share of tries), made | 0.164 (5.1%), 48.9% | 0.292 (9.0%), 46.3% | 0.244 (9.8%), 47.5% |
| fake punts a team game, converted | 0 | 0.027, 53% | 0.036, 55% |
| fake field goals a team game, converted | 0 | 0.0050, 6 of 20 | 0.0044, 1 of 12 |
| untimed downs at 0:00 | 1 in 2,000 games | 1 in 143 | 1 in 68 |
| plays a team game | 63.19 | 62.23 | 61.91 |
| points a team game | 27.73 | 27.92 | 22.51 |

The pool is a half's last snap with 5 s or less: every second-quarter one, and the fourth quarter's when tied or down 1-8. Over main 569e61f, before lane EG5, the lane's numbers were the same within the seeds' noise (Hail Marys 6.2% / 28.7%, laterals 26.7% of 165, untimed downs 1 in 286: EG5's fouls doubled those).

FBS (2,000 games): plays 68.17 to 66.71 a team game (real 66.57), points 26.25 to 26.28, two-point tries 0.139 to 0.301 a team game, the defense's Q2 timeouts 0.009 to 1.34 a game, fake punts 0.037 a team game (61% converted), untimed downs 1 in 111 games, Hail Marys scored 6.3% from 30-39 out, 4.8% from 40-49 and none of 89 from 50-59, laterals on 43% of the fourth quarter's last passes from 40+ out. High school (1,000 games): plays 52.70 to 51.37, points 20.94 to 20.80, two-point tries 0.520 to 0.579, untimed downs 1 in 38 games (every quarter, either team's fouls), laterals on 55% of 49 such passes.

Over lane EG3 (the kicking game), which merged first, the same seeds give at the NFL: Hail Marys 10.5% scored and 27.4% picked at the real mix of distances (446 throws; the real 6.5% is 4 touchdowns in 62), touchdowns on 2.4% of the pool's snaps (real 2.5%), laterals on 29.2% of 144 passes, two-point tries 9.3% of tries, fake punts 0.034 a team game (55% converted), untimed downs 1 in 100 games, 62.15 plays and 27.28 points a team game (EG3 alone 27.15). FBS plays 66.72, high school 51.48. EG3's 55+ field goal tries in a half's last 30 seconds stay at 0.030 a team game (real 0.060): the last snap kicks up to 6 yards past the range when the kick beats a Hail Mary, so the shortfall is in reaching a long kick with time left, not the last call.

The analysts' tables (`world/tally/decide.rs`) stay as main has them (EG3's kick tables, the rest from lane EG5). Refitted over this lane before EG3 (`decide_fit -- 1500 1`), they move about as much as two seeds of one engine do (the NFL's 7-10-to-go conversions 0.342 to 0.349, two-point tries made 0.469 to 0.498 with twice the tries), and that flips the model's call on 4th and 9 at their 12 early in a tied game from the kick to going for it, where a test pins nfl4th's kick (`tests::decide`). The call sits on a near tie because the model scores any conversion as a touchdown once the mean gain of converted plays reaches the goal (17.1 yards for 7-10 to go), which overstates going near the goal line (`GAPS.md`); the world rebuild refits the tables after the engine lanes. The coaches' call table (`call_values.csv`) stays: the tool plays two average NFL teams' ordinary snaps, and nothing this lane changed reaches one (the last play's calls, the fakes, the carrier's clock read late in the fourth and the two-point habit never come up in it).

Speed: the same. `match_check nfl 2000` takes 12.9 and 13.2 s against 13.2 and 13.2 s on main (back to back, 4 cores, the same build settings).

Tried and dropped (NFL, seed 1):

- **The first Hail Mary:** the depth error of a long throw scaled up as on any other (up to 2.4 times), the other receivers kept their routes and a catch's chance went to nothing with three defenders near: almost none scored from 35 out in `play_trace`. Then picks at 0.5 and catches at 0.35: 2.4% scored and 35% were picked.
- **A 60-yard NFL arm** (56 college, 48 high school): 2.1% scored from 50-59 yards out, where real Hail Marys scored 3 times in 26, and the second quarter threw them from 60-69 yards out 7 to 16 times as often as real teams (6 in five seasons; a side threw it if it came down within 10 yards of the goal line).
- **Laterals with the trailers escorting ahead** of the man with the ball: no laterals at all. With the hurry-up's habits still on the last play: 2% of the passes.
- **The defense's Q2 timeout on any down:** 3.06 a game.
- **Fake punts without the look** at the real rate: 13% converted (real 55%); thrown by the up-back, 4%.
- **The half's sum without the chains' stops** in college and high school: FBS plays fell to 64.9 a team game (real 66.6).

Still off (`GAPS.md`, over lane EG3): a half's last 5 s take one snap where real teams often fit two (a quick throw at the end zone, then the kick), so of the pool's second-quarter snaps the engine kicks 35% (real 51%) and kneels 51% (18%), and the pool is 0.47 snaps a game (real 0.74); spikes are 0.9% of the pool (real 8.5%); untimed downs come one in 100 games (real one in 68), since the Hail Mary's crowd (`crowd_ball`) skips the catch point where lane EG5 reads pass interference (item 27); the fourth quarter's pool throws deep more and kicks less than real teams' (deep shots 9.7% of the pool, real 2.7%; field goals 5.1%, real 11.1%), likely from where its late drives stand when time runs out rather than the call itself (not measured); laterals come a little often at the NFL (29%, real 22%) and in college and high school (41% and 58%, unmeasured against real); and few plays reach the sideline, the rest of S6.2, which this lane left to the receivers' routes (SF4) and long runs (SF9): designed runs end out of bounds 1.2% of the time (real 5.5%).

## The QB's reads and the receivers (scheme-fit lane SF4, items 5, 6 and 25, 2026-11-19)

Design: `docs/design/scheme-fit.md`, items 5, 6 and 25, in Cain's words: Decisions makes a better passer, not just a careful one, keeping the split (Football IQ stays the pre-snap and protection read, Decisions becomes picking the right throw); route running and hands drive a receiver's results ("absolutely, that diversity is pivotal"); and the quarterback feeds his receivers by their receiving. The targets: fewer picks without losing depth, and more points; route running beats zone too; hands decide drops and contested balls at their real size; speed pays deep and after the catch, not in separation; height pays in contested catches and the red zone; who is looked at first reads hands, routes and getting open, each man against his own position's typical starter. Built on lanes SF1 and SF2 and engine gaps items 29 and 30 (main dcdde89), measured again over main 36b6367 (minor bugs 409-412, the play action leftovers, and the recentred 14-year-old habits, which redraw every synthetic roster's flaws), over main 569e61f, which brings engine gaps lane EG2 (ball security: live fumbles, fumbled snaps, strip sacks), over main a7c9bf6, which brings lane EG5 (each level's rule book and the flags), over main 0972bef, which brings lane EG3 (the kicking game), and over main b4c6206, which brings lane EG4 (the end of halves: the Hail Mary, laterals and the clock); every number below is over b4c6206. Merging EG4 in turned up minor bug 423, fixed here after the measuring; the fix moves nothing else (below). It takes SF2's hand-off: the second tight end out-drew the first in 13 personnel. How it works: `docs/reference/how-it-works/match-engine.md` (the first read, the window, the reaction to the throw, the catch point). Every size is Claude's, proposed, not confirmed.

How it was measured: `fit_check` (team 0 the typical team, one change an arm on the same games; FBS seeds 151 and 171 at 3,000 games an arm, NFL seed 161 at 2,400; "past the noise" is |z| of 1.96 or more), main b4c6206 and after, on the lane's 33 arms. The league: `match_check` with `AFM_RECDIAG=1` and `AFM_GAPS=1` on the test rosters (4,000 games at the NFL on seeds 11, 12 and 13 and at FBS on seed 11, 2,000 in high school on seed 11) and on world seed 1's NFL clubs (4,000 games on each of game seeds 1-4), and SF2's league tally for the tight ends (the test league with every team set firmly to 12 or 13, 2,000 NFL games, seed 1).

One caution on `fit_check` and the feed: on the typical team every receiver reads the same as a receiver (each at his position's typical starter), so any rating the feed reads breaks the tie for the man who changes, and his chance of the first look goes from none to 10% plus 2% a point of his edge (19% for Catching or Route Running +15, which give him 4.5 points). In a league somebody is always the best receiver, and a point of edge is worth 0.02 there. So the feed's effects in these tables are bigger than a point is in a league. Main's feed had no such tie: it read each man's overall at his spot, which differs by position, so main's typical team fed its best-rated receiver, and the base arms differ between main and the lane partly for that (`GAPS.md`, "`fit_check`'s typical team feeds nobody").

What was wrong on main:

- **A better reader was a careful one.** The misread worked both ways (minor bug 283): a passer above his level's typical starter's field read saw every window tighter than it was, each defender who saw his eyes breaking on the ball sooner than he would and each man in his lane taking more room than he did. So Decisions +15 cut picks (-0.04 to -0.26 points of throws over the 12 arms, 7 past the noise) by throwing shorter (-0.20 to -0.35 yards of air a throw, all 12) and holding the ball longer (+0.004 to +0.018 s, 10 of 12 past the noise), and scored no more (-0.33 to +0.29 points a game, -0.05 on average, none past the noise).
- **Against zone, route running only read where he sat down.** Route Running +15 drew 0.7 to 3.6 more targets a 100 routes against zone (all 21 arms) into tighter windows: his catch rate against zone moved -3.7 to -0.2 points (16 past the noise, all falls) and his separation against zone -0.14 to +0.07 yards. Against man it gave him 0.18 to 0.87 yards (all 21).
- **Hands were small.** The catch read 0.00095 a point of Catching from none, so Catching +15 took 0.31 to 1.01 drops off 100 targets (12 of 15 arms past the noise). On world seed 1's NFL clubs (four game seeds) the drops a target spread 0.62 points between wide receivers (men with 40 or more targets, the binomial noise taken out; real 0.54, FTN charting 2022-24), 0.57 between tight ends (real 1.01) and 0.59 between backs (real 1.12), and Catching alone gave only 0.34, 0.43 and 0.56 of it.
- **A contested ball barely read the receiver's own skill.** The breakup read the defender's Contested Catch, Jumping and coverage and both men's reach; the receiver's Contested Catch came in only in the fight after the ball was out. So Contested Catch +15 moved his contested catch rate -0.5 to +4.4 points (1 of the 9 arms past the noise), and on world seed 1's NFL clubs wide receivers' contested catch rate spread 5.3 points between them (men with 12 or more contested balls; real 8.9).
- **The feed read a man's spot rating.** A back's leans on his running, so any of his running ratings +15 raised his targets a route by half (+5.1 to +6.3 a 100 routes on 9.9 to 12.6, all 9 arms; +0.66 to +1.02 targets a game), and a tight end's Run Blocking +15 fed him 0.6 to 1.3 more a 100 routes (all 6 arms past the noise), as much as his Catching, Contested Catch or Route Running +15 did (0.4 to 1.3).
- **In 13 personnel the second tight end out-drew the first** (SF2's hand-off). With every team of the test league set firmly to 13, the first, second and third tight ends on the field drew 32%, 47% and 21% of the tight ends' targets (NFL, 2,000 games; real 47%, 29% and 24%): the depth chart's second takes the flanker's place, which many concepts read before the first tight end's. Set to 12, the second drew 52% (real 43%).

What changed (`game/scrimmage.rs`, `game/flow.rs`; every size proposed, not confirmed):

- **The feed reads a man as a receiver** (`run_snap`, `receiving_edge`, `FEED_READ`): 0.3 Catching, 0.3 Route Running, 0.15 Release, 0.1 Contested Catch, 0.1 Ball Tracking and 0.05 Body Control, each about his own position's typical starter at the level. The weights sum to one, so a point of edge counts as a point of his old spot rating did, and `FEED_BASE`, `FEED_PER_POINT` and `FEED_MAX` keep their sizes. Speed stays out of it: a deep threat's first looks come from the concept, and reading his forty on a deep route made his targets deeper but no better (tried, below).
- **A better reader sees the field as it is** (`window`, `READ_SHARP`, `LURK_NEAR`, `ball_break`). A passer above the typical starter's field read sees each defender who sees his eyes break on the ball as the throw will make him break (his Play Recognition and his job's coverage, the sums `throw` uses), all of it at 10 points above the typical starter, and each lane as it is. A poorer reader keeps the misread as it was. The one man a better reader still gives the old respect is a defender underneath reading his eyes, between him and the spot within 5 yards of the throw's line: the man who picks it.
- **Route running turns the zone defender near him** (`throw`, `zone_turn`, `ZONE_TURN`, `TURN_NEAR`): a zone defender within 6 yards of the target on his route breaks on the throw 0.1 s later a 10 points of the target's Route Running and route craft about his level's typical starter at his position (sooner for a poorer route runner). A better reader's window reads the same break; the typical and poorer readers' windows keep their fixed reaction, so they find the open man by the play, not by reading the turn.
- **Hands at their real size** (`catch_point`, `CATCH_HANDS`): the miss on a catchable ball falls 0.0016 a point of Catching about his level's typical starter at his position (0.00095 a point from none before), so the typical starter keeps his catch. It is sized on world seed 1's NFL clubs, where the drops' spread between men is what real data measures (below). The first size, 0.0029, came from a spread of Catching between real starters of about 6 points; the world's receivers with 40 or more targets spread about 10, and at 0.0029 their drops spread 1.49 points (real 0.54), tight ends' 1.24 (real 1.01) and backs' 2.18 (real 1.12; one game seed, on the lane's build over main dcdde89).
- **Contested balls** (`catch_point`, `contest_edge`, `CONTEST_PBU`): the receiver's Contested Catch about his level's typical starter at his position, with Highpointer and Late hands, takes 0.02 a point off the breakup's base. His jump and frame already count in the reach and his Strength in the fight after the ball, so they stay out of it.
- **The best receiving tight end where the throw reads first** (`game/flow.rs`, `lineup`; SF2's hand-off). On a throw whose reads hold a tight end past the first (12, 13 and 22 personnel, where the extra tight ends take missing receivers' places), the staff puts its best receiving tight end (by the feed's read, `receiving_edge`) in the tight ends' place the play reads first, at the feed's chance: 10% plus 2% a point of his edge over the man the depth chart puts there, at most 55% (a keyed draw, so the game's own stream is untouched). By the depth chart alone the second tight end always took the first missing receiver's place, the flanker's in 13 personnel, which many concepts read before the first tight end's. A run, an RPO and a throw whose reads hold no tight end past the first keep the depth chart's places.
- **No slide on the last play's laterals** (`run_snap`, the slide; minor bug 423, found by the quick check's depth test once lane EG4 was merged in). A quarterback slides rather than take a hit he doesn't need, and he slid on the last play's laterals too: when a lateral came back to him after a catch, the slide made the play his scramble, dead where he stood as he pitched it, so the box score counted the catch's target and the players' tally didn't. He no longer slides on the last play's laterals (`s.call.laterals`), where going down ends the game. The numbers in this section were measured just before the fix. On the same `match_check` runs it changes only those snaps: points and interceptions stay the same to the third decimal at every level, and about 0.001 a team game of scrambles become throws.
- **Tools:** `match_check` with `AFM_RECDIAG=1` prints the receivers' spreads against real (catch rate, contested catch rate and drops between men, an inch of height against contested balls, separation and its slope on Speed, drops' slope on Catching, and the forty's slopes of aDOT and of deep targets). `fit_check` reads each receiver's catch rate against man and zone, his targets and yards a route against zone, his contested balls (under a yard of separation; picks are left out, since a pick has no separation on the play record) and his touchdowns on red zone targets.

Item 5, the passer (`fit_check`: Decisions +15 in the West Coast, Air Raid, wide zone and spread option contexts, Football IQ +15 in the West Coast, on the three seeds; main b4c6206, then after; each cell is the range over the arms and seeds, with how many are past the noise):

| | main | after |
|---|---|---|
| Decisions +15: points a game | -0.33 to +0.29 (0 of 12) | -0.01 to +0.81 (4 of 12) |
| interceptions, points of throws | -0.26 to -0.04 (7 of 12) | -0.14 to +0.06 (1 of 12) |
| air yards a throw | -0.35 to -0.20 (all 12) | -0.02 to +0.19 (6 of 12) |
| completion rate, points | -0.0 to +0.9 (8 of 12) | -0.1 to +0.3 (0 of 12) |
| time to throw, s | +0.004 to +0.018 (10 of 12) | -0.030 to -0.015 (all 12) |
| sacks, points of dropbacks | -0.04 to +0.19 (3 of 12) | -0.54 to -0.20 (all 12) |
| team EPA a dropback | -0.010 to +0.013 (0 of 12) | +0.003 to +0.024 (7 of 12) |
| Football IQ +15: points a game | -0.46 to +0.74 (1 of 3) | +0.31 to +0.45 (0 of 3) |
| sacks, points of dropbacks | -0.81 to -0.41 (all 3) | -0.60 to -0.59 (all 3) |
| pressure, points of dropbacks | -2.0 to -1.9 (all 3) | -1.6 to -1.6 (all 3) |
| interceptions, points of throws | -0.06 to -0.01 (0 of 3) | -0.16 to +0.04 (1 of 3) |
| air yards a throw | -0.21 to -0.09 (all 3) | -0.17 to -0.07 (2 of 3) |

Why: on main a better reader saw every window tighter than it was, so he threw short and held the ball. Now he sees the breaks and the lanes as they are, so he keeps his depth, gets the ball out sooner and takes fewer sacks (all 12 arms). His team scores more on 11 of the 12 (+0.35 a game on average, main -0.05), and the 4 past the noise are all gains. His picks barely move (-0.03 points of throws on average; the 1 past the noise is fewer), where main's fell 0.14 by throwing short. Most of the throws a passer has picked are ones he saw open (`GAPS.md`, "Picks come from the wrong throws"), so seeing the field better can't cut them; that part of item 5 is lane SF5's, where picks and breakups come from.

Football IQ stays the pre-snap and protection read: its +15 cuts sacks and pressure about as much as on main. Its points come out +0.33, +0.31 and +0.45 a game on FBS 151, FBS 171 and NFL 161 (main +0.74, +0.30 and -0.46), the lane's none past the noise and main's first one past it: lower than main on FBS 151, the same on FBS 171 and higher at the NFL, where main's own arm lost points (inside the noise). Over main a7c9bf6 the lane's had come out 0.17 a game under main's on average, and over main dcdde89 they had fallen further (+0.25 to +0.61 a game on main, -0.11 to +0.16 on the lane). Putting main's feed back on that build (each man's overall at his spot, so the typical team feeds its best-rated receiver) brought about half of it back (+0.34, -0.13 and +0.41 a game on the three seeds): a typical team that feeds nobody moves what other arms measure too (`GAPS.md`, "`fit_check`'s typical team feeds nobody").

Item 6, the receivers (the X, Z, slot, first tight end and back in the contexts the audit gives each, on the three seeds):

| | main | after |
|---|---|---|
| Route Running +15: separation against zone, yards | -0.14 to +0.07 (4 of 21) | -0.13 to +0.27 (12 of 21) |
| catch rate against zone, points | -3.7 to -0.2 (16 of 21) | -4.5 to +1.3 (13 of 21) |
| yards a route against zone | +0.01 to +0.34 (20 of 21) | +0.19 to +0.52 (all 21) |
| separation against man, yards | +0.18 to +0.87 (all 21) | +0.33 to +0.95 (all 21) |
| team EPA a dropback | -0.005 to +0.014 (0 of 21) | -0.004 to +0.027 (5 of 21) |
| points a game | -0.26 to +0.42 (0 of 21) | -0.40 to +0.81 (3 of 21) |
| Catching +15: drops a 100 targets | -1.01 to -0.31 (12 of 15) | -1.15 to -0.74 (all 15) |
| catch rate, points | -2.1 to +3.3 (13 of 21) | -1.7 to +4.1 (14 of 21) |
| yards a route | +0.04 to +0.36 (20 of 21) | +0.14 to +0.55 (all 21) |
| points a game | -0.27 to +0.81 (3 of 21) | -0.54 to +0.46 (2 of 21) |
| Contested Catch +15: contested catch rate, points | -0.5 to +4.4 (1 of 9) | +4.3 to +9.7 (all 9) |
| contested share of his targets, points | -0.6 to +1.0 (4 of 9) | -0.1 to +1.3 (5 of 9) |
| yards a route | -0.04 to +0.23 (7 of 9) | +0.10 to +0.38 (all 9) |
| red zone touchdowns a target, points | -3.0 to +1.9 (0 of 9) | -1.8 to +2.9 (0 of 9) |
| Speed +15: air yards a target | +0.82 to +1.89 (all 6) | +0.26 to +0.86 (all 6) |
| targets 20+ yards downfield a season | +2.5 to +9.2 (all 6) | +0.8 to +3.7 (all 6) |
| yards after the catch | +0.37 to +0.87 (all 6) | +0.62 to +0.93 (all 6) |
| separation against man, yards | +0.56 to +0.96 (all 6) | +0.69 to +1.08 (all 6) |
| separation against zone, yards | -0.28 to -0.03 (5 of 6) | -0.10 to +0.01 (2 of 6) |
| catch rate, points | -4.1 to -1.5 (all 6) | -2.4 to -0.7 (4 of 6) |
| 6'4" X: contested share of his targets, points | +0.8 to +1.3 (all 3) | +1.0 to +1.3 (all 3) |
| contested catch rate, points | -2.9 to +0.3 (1 of 3) | -2.3 to -0.5 (0 of 3) |
| red zone touchdowns a target, points | -1.5 to +0.2 (0 of 3) | +0.0 to +0.3 (0 of 3) |
| 5'10" X: contested share, points | -1.1 to -0.3 (2 of 3) | -1.5 to -0.1 (1 of 3) |
| contested catch rate, points | +0.4 to +4.3 (1 of 3) | -0.9 to -0.4 (0 of 3) |

Why, by measure:

- **Route running against zone.** The yards a route against zone are the claim: +0.19 to +0.52 on all 21 arms, against +0.01 to +0.34 on main. In the slot and at tight end, where a zone defender sits close when the ball goes, Route Running +15 now gains 0.06 to 0.27 yards of separation against zone (7 of 9 arms past the noise; main -0.09 to +0.03) and loses less of his catch rate there (-0.3 points on average; main -1.1). Outside, the X and Z gain none (-0.13 to +0.18, -0.01 on average), and their catch rate against zone falls 0.8 to 4.5 points (main 0.7 to 3.7), while their yards a route against zone rise more than on main (+0.27 to +0.46, main +0.17 to +0.34) from more targets. Likely two causes, neither traced: their zone defender is mostly a deep corner more than 6 yards off (`TURN_NEAR`), so the turn rarely reaches him, and Route Running now feeds them (19% of the first looks on the typical team, the caution above), so more of their targets come by the plan, not by an open window (`GAPS.md`, "Outside receivers' route running buys no separation against zone"). Against man it gains as on main (+0.33 to +0.95 yards, main +0.18 to +0.87).
- **Hands.** Catching +15 takes 0.74 to 1.15 drops off 100 targets (all 15 arms; main 0.31 to 1.01), and since the feed reads Catching he draws more targets too (+2.0 to +9.9 a 100 routes, main +0.4 to +5.8). His team's points move -0.54 to +0.46 a game (main -0.27 to +0.81). The receivers' and tight ends' arms score about as on main (+0.12 a game on average, main +0.09; the one past the noise is a gain, the slot in the West Coast on FBS 151, +0.46). The back's arms score a little less (-0.13 a game on average, main +0.10; the one past the noise is the back in the power gap at the NFL, -0.54): on the typical team the tie hands him 19% of the first looks, and a back gains 0.63 to 1.05 yards a route where a receiver gains 1.50 to 1.95.
- **Contested balls.** Contested Catch +15 wins 4.3 to 9.7 more points of his contested balls (all 9 arms past the noise; main -0.5 to +4.4, 1 past the noise), draws a few more of them (-0.1 to +1.3 points of his targets; the 5 past the noise all more) and gains 0.10 to 0.38 yards a route (all 9; main -0.04 to +0.23).
- **Speed** still pays deep and after the catch: Speed +15 adds 0.26 to 0.86 air yards a target, 0.8 to 3.7 targets 20+ yards a season and 0.62 to 0.93 yards after the catch (all 6 arms), and buys separation against man (+0.69 to +1.08) but not against zone (-0.10 to +0.01; real: speed doesn't buy separation, `AFM_RECDIAG`). He draws fewer extra targets than on main (+1.5 to +2.2 a 100 routes, main +4.3 to +5.3): Speed fed him through his spot rating on main, so part of main's deep targets came from the feed's first looks.
- **Height** pays in the targets, not yet in the catch. A 6'4" X draws 1.0 to 1.3 points more of his targets contested (all 3 past the noise) and wins them no more often (-2.3 to -0.5 points, none past the noise; main -2.9 to +0.3), and his red zone touchdowns a target move +0.0 to +0.3 points (none past the noise; main -1.5 to +0.2). His team's points move 0.00 to +0.43 a game (1 of 3 past the noise), about as on main (+0.02 to +0.52, 1 past). A 5'10" X is the mirror: fewer of his targets contested (0.1 to 1.5 points), a catch rate 0.1 to 1.7 points higher, and his contested balls won about as often (-0.9 to -0.4 points, inside the noise). The catch point is lane SF5's (`GAPS.md`, "Height pays in contested targets").

Item 25, who is looked at first:

| | main | after |
|---|---|---|
| a back's Ball Carrier Vision, Speed or Elusiveness +15: targets a 100 routes | +5.1 to +6.3 (all 9) | -0.1 to +0.4 (0 of 9) |
| targets a game | +0.66 to +1.02 (all 9) | +0.01 to +0.14 (6 of 9) |
| a tight end's Run Blocking +15: targets a 100 routes | +0.6 to +1.3 (all 6) | -0.2 to +0.1 (0 of 6) |
| targets a game | +0.19 to +0.36 (all 6) | +0.02 to +0.08 (1 of 6) |
| the first tight end's Catching, Contested Catch or Route Running +15: targets a game | +0.17 to +0.41 (all 9) | +0.34 to +1.15 (all 9) |
| yards a route | +0.04 to +0.14 (8 of 9) | +0.10 to +0.43 (all 9) |
| points a game | -0.32 to +0.31 (0 of 9) | -0.22 to +0.12 (0 of 9) |
| a back's Catching +15: targets a game | +0.05 to +1.29 (all 6) | +0.52 to +2.17 (all 6) |
| the X's Release +15: targets a 100 routes | +2.4 to +2.9 (all 3) | +3.0 to +3.5 (all 3) |
| the X's Ball Tracking +15: targets a 100 routes | +2.2 to +2.5 (all 3) | +2.4 to +3.3 (all 3) |
| the X's Speed +15: targets a 100 routes | +4.3 to +5.3 (all 6) | +1.5 to +2.2 (all 6) |

Why: the feed reads a man as a receiver now, so a back's running and a tight end's blocking no longer feed him (targets a route inside the noise on all 15 arms). A better runner or blocker draws at most 0.14 more targets a game, because he plays more snaps (0.4 to 2.5 points more of them, 13 of the 15 arms past the noise; at FBS a back also carries 0.09 to 0.78 more a game). His receiving feeds him instead: the first tight end's Catching, Contested Catch or Route Running +15 draws 0.34 to 1.15 more targets a game (main 0.17 to 0.41), and a back's Catching 0.52 to 2.17 (main 0.05 to 1.29). Release and Ball Tracking +15 feed the X a little more than on main, and Speed less (above). These are the typical team's tie (the caution above); in a league a point of edge is worth 0.02 of the first looks.

SF2's hand-off, the tight ends (the test league with every team set firmly to 13 or 12, 2,000 NFL games, seed 1; main, then after, and the swap tried on every throw; real 2024 NFL as in lane SF2):

| | main | after | always the best (tried) | real |
|---|---|---|---|---|
| 13 personnel: the first / second / third tight end's share of the tight ends' targets | 32 / 47 / 21% | 40 / 41 / 19% | 60 / 28 / 11% | 47 / 29 / 24% |
| 13: the tight ends' share of the targets | 60% | 59% | 56% | 56% |
| 13: air yards a target, the first / second / third | 9.4 / 7.9 / 6.7 | 9.6 / 7.9 / 7.0 | 9.8 / 7.7 / 6.4 | 7.6 / 6.5 / 4.5 |
| 12 personnel: the second tight end's share of the tight ends' targets | 52% | 48% | 36% | 43% |
| 12: air yards a target, the first / second | 10.0 / 7.8 | 10.4 / 7.8 | 10.9 / 6.9 | 7.0 / 6.3 |

The swap reads the best man's edge over the one the depth chart puts in the first place, at the feed's chance. Putting the best receiving tight end there on every throw overshot (the first drew 60%, real 47%); at the feed's chance the staff leans on its best man as the passer's first look does, without always going to him. The first tight end still draws less than real and the second more: the second stands in for the flanker, whom many concepts read before the first tight end, and runs the short job the quarterback finds most often (SF2's read order; `GAPS.md`, "In 13 personnel the first tight end still draws less than real").

The receivers across a league (`match_check` with `AFM_RECDIAG=1`: the NFL test rosters, the mean of seeds 11, 12 and 13 at 4,000 games each, and world seed 1's NFL clubs, the mean of game seeds 1-4 at 4,000 games each; men with 40 or more targets, 12 or more contested balls for the contested rate, each man's binomial noise taken out; main, then after; real NFL receiver seasons, FTN charting 2022-24 with nflverse):

| | test rosters | world seed 1's clubs | real |
|---|---|---|---|
| drops a target, the spread between men, wide receivers / tight ends / backs (points) | 0.36 / 0.43 / 0.88 > 0.43 / 0.81 / 0.69 | 0.62 / 0.57 / 0.59 > 0.70 / 0.62 / 1.07 | 0.54 / 1.01 / 1.12 |
| the drops' spread Catching alone gives (its slope × its spread) | 0.11 / 0.22 / 0.27 > 0.28 / 0.44 / 0.12 | 0.34 / 0.43 / 0.56 > 0.55 / 0.66 / 0.70 |  |
| drops, % of targets | 2.82 / 2.55 / 4.27 > 2.74 / 2.59 / 4.37 | 3.05 / 2.92 / 4.72 > 3.10 / 2.99 / 4.78 | 3.45 / 3.69 / 4.87 |
| catch rate, the spread between men (points) | 4.9 / 4.3 / 3.4 > 5.4 / 4.4 / 3.1 | 5.7 / 5.2 / 3.1 > 6.5 / 5.1 / 3.7 | 5.7 / 3.3 / 3.6 |
| contested catch rate, the spread between men, wide receivers / tight ends (points) | 3.8 / 1.5 > 6.3 / 6.1 | 5.3 / 1.9 > 9.8 / 7.9 | 8.9 / 11.3 |
| an inch of a wide receiver's height: his contested catch rate (points) | +0.32 > +0.36 | +0.32 > +0.58 | +0.72 |
| an inch: his share of contested targets (points) | +0.62 > +0.52 | +0.62 > +0.57 | +1.07 |
| a wide receiver's 0.1 s slower forty: air yards a target | -0.63 > -0.62 | -0.92 > -0.79 | -0.79 |
| the same: targets 20+ yards, points of his targets | -0.72 > -0.63 | -2.35 > -2.06 | -2.85 |
| a wide receiver's separation at the catch, the spread between men (yards) | 0.48 > 0.47 | 0.45 > 0.46 | 0.42 |

The hands are sized on the world: its men are drawn as the game draws them, and its receivers spread 9 to 10 points of Catching between them at each position (the test rosters 6.7 to 8.1). There drops a target now spread 0.70, 0.62 and 1.07 points between wide receivers, tight ends and backs (main 0.62, 0.57 and 0.59), against real spreads of 0.54, 1.01 and 1.12, and Catching alone gives 0.55, 0.66 and 0.70 of it (main 0.34, 0.43 and 0.56). The backs come near real. The tight ends' spread barely moves, but it is the noisiest (0.19 to 1.37 over the four game seeds, main's 0 to 0.82, where 0 means the binomial noise alone was larger than the spread), and Catching alone now gives more of it. The wide receivers' spread was past real on main already, with little of it from their hands, and runs further past it: a miss counts as a drop only with nobody within a yard, so a more open man has more balls he can drop (`GAPS.md`, "Wide receivers' drops spread wider than real in the world"). One game seed's spread is noisy (the wide receivers' 0.54 to 0.87 over the four, main's 0.48 to 0.74; over main a7c9bf6 the lane's four gave 0.56 to 0.98), so the table reads their mean. The contested catch rate's spread between wide receivers comes to about real (9.8, real 8.9; main 5.3). Both builds' spread rose about 2 points when lanes EG3 and EG4 merged (over main a7c9bf6 main's was 3.2 and the lane's 8.1), so that rise isn't this lane's; it isn't traced further. Height's slope moves inside its noise (single seeds -0.58 to +1.08 points an inch), and the forty's slope of deep targets eases a little away from real (-2.06 points a 0.1 s on the world's clubs, main -2.35, real -2.85), since Speed no longer feeds a man (`GAPS.md`, "Fast receivers draw too few deep targets").

The league's totals hold (`match_check`; the NFL test rosters on seeds 11, 12 and 13 at 4,000 games each, FBS on seed 11 at 4,000, high school on seed 11 at 2,000, and world seed 1's NFL clubs on game seeds 1-4 at 4,000 each; main, then after, and for the NFL and the world the change's range over the seeds). Every total stays inside its band in `tests/match_engine.rs`.

| | NFL, 3 seeds | FBS | high school | world seed 1's NFL clubs | real NFL |
|---|---|---|---|---|---|
| points a team game | 27.42 > 27.53 (+0.05 to +0.19) | 25.62 > 25.71 | 21.41 > 21.28 | 29.35 > 29.04 (-0.49 to -0.12) | 22.51 |
| yards an attempt | 7.61 > 7.61 (-0.01 to +0.01) | 7.50 > 7.46 | 6.89 > 6.81 | 8.10 > 8.05 (-0.07 to -0.02) | 7.05 |
| completion rate | 65.6 > 65.3 (-0.4 to -0.2) | 60.5 > 60.0 | 57.4 > 56.8 | 67.8 > 68.1 (+0.2 to +0.5) | 64.5 |
| interceptions a team game | 0.819 > 0.848 (+0.017 to +0.041) | 0.668 > 0.690 | 0.333 > 0.350 | 0.675 > 0.666 (-0.021 to -0.002) | 0.756 |
| air yards an attempt | 9.30 > 9.36 (+0.04 to +0.09) | 8.86 > 8.94 | 7.43 > 7.65 | 9.19 > 8.91 (-0.31 to -0.24) |  |
| drops, % of attempts | 3.3 > 3.2 (-0.1 to +0.0) | 3.7 > 3.6 | 4.5 > 4.5 | 3.5 > 3.6 (+0.1 to +0.1) |  |
| broken up, % of attempts | 8.6 > 8.8 (+0.2 to +0.3) | 5.9 > 6.3 | 4.3 > 4.7 | 9.4 > 9.5 (+0.0 to +0.1) |  |
| red zone touchdowns, % of attempts | 28.2 > 27.9 (-0.4 to -0.3) | 27.9 > 27.8 | 27.7 > 27.8 | 29.9 > 29.5 (-0.7 to -0.1) | 19.0 |
| explosive pass rate | 8.7 > 8.7 (+0.0 to +0.1) | 8.9 > 8.9 | 7.6 > 7.4 | 10.1 > 10.0 (-0.1 to -0.1) | 8.8 |
| sack rate | 3.4 > 3.3 (-0.1 to +0.0) | 4.3 > 4.0 | 7.2 > 7.2 | 3.4 > 3.2 (-0.3 to -0.1) | 6.8 |
| targets to the wide receivers | 64.8 > 65.0 (+0.0 to +0.6) | 69.4 > 69.6 | 61.8 > 62.7 | 65.8 > 63.3 (-2.7 to -2.4) | 59.3 |
| targets to the tight ends | 21.0 > 21.0 (-0.1 to +0.1) | 16.1 > 15.8 | 17.9 > 18.1 | 17.5 > 17.1 (-0.5 to -0.3) | 21.9 |
| targets to the backs | 13.5 > 13.2 (-0.5 to -0.2) | 13.4 > 13.3 | 17.3 > 16.1 | 13.5 > 15.4 (+1.9 to +2.1) | 18.1 |

Why: interceptions rise 3% to 5% on the test rosters (NFL 0.819 to 0.848 a team game, real 0.756; FBS 0.668 to 0.690; high school 0.333 to 0.350) and balls broken up 0.2 to 0.4 points of attempts. A passer at or below his level's typical field read keeps the fixed reaction in his window (above), but the throw itself now reads the target: a zone defender breaks sooner on a throw to a poorer route runner (`zone_turn`), and the breakup comes easier against a poorer contested catcher (`contest_edge`). Those windows close faster than he reads, and the test rosters' receivers spread both ways about the typical starter, where the tight windows cost more than the open ones give back. On world seed 1's NFL clubs picks hold (0.675 to 0.666 a team game) and points fall 0.31 a team game toward real (29.35 to 29.04, real 22.51): their backs read as better receivers than the typical starter at their position (`GAPS.md`, "The typical starter isn't the world's"), so the feed sends them 1.9 points more of the targets (15.4%, real 18.1%), and those throws are short (air yards an attempt 9.19 to 8.91). Pass interference (lane EG5's) rises with the breakups and the depth: accepted on defenders 0.506 to 0.569 a team game over the NFL's three seeds on main and 0.520 to 0.578 after (real 0.497; flagged 0.534 to 0.601 and 0.548 to 0.609, real 0.561), 0.433 to 0.459 at FBS and 0.249 to 0.252 in high school, since `pi_contact` finds contact more often on a ball the defender broke up (1.2 against 0.7) and on a deeper throw. EG5's other flags move inside their noise (0.02 a team game or less, as much as the false starts, which this lane can't touch). The engine gaps audit's other counts (`AFM_GAPS=1` on the same runs) hold: points by quarter, injuries, kicks and returns (lane EG3's), fumbles (lane EG2's, by kind and by position) and the half's last snap (lane EG4's: its touchdowns, Hail Marys and laterals) move inside their noise, and scrambles fall a little (1.02 to 1.13 a team game at the NFL on main, 0.92 to 1.07 after; FBS 1.09 to 1.05, high school 0.70 to 0.66), since a better reader throws sooner. One count leans: a few more of the NFL's last snaps of a half end in a fourth-quarter field goal (6.5% of them, main 5.5%, up on all three seeds; FBS 4.4%, main 3.7%) and fewer in a second-quarter one (22.2%, main 23.6%), so about as many end in a field goal in all (28.7%, main 29.1%); not traced.

The coaches' table (`call_values` over main b4c6206, whose run rows lane EG2 rebuilt and whose pass rows lanes EG5, EG3 and EG4 kept; the tool's output on main is the same as over a7c9bf6, cell for cell): the lane moves the tool's pass cells by 0.23 yards a cell (the mean of |change|), with no shift on average (+0.02), and the run rows not at all. The straight pass rows already wait for minor bug 405 (the tool sits 2.4 yards a cell off the committed table on main itself), and the rows the table takes from a new run (the three play-action concepts and the fakes' changes, `tools/call_values/compose.py`) move 0.29 yards a cell with the lane, less than main's own new run moves them off the committed table (1.15, against 0.27 over main 569e61f, so most of it is lane EG5's), so the committed table stands. One cell jumped: spacing against Cover 1 robber was picked 10.1% of the time (2.2% on main), because the tool's fixed team has a first tight end 17.4 points under the typical starter's Catching and 7.8 under his Route Running, and the receiving reads now make him worse (`GAPS.md`, "The call table's fixed teams aren't typical").

Speed (`match_check`, 2,000 games at each level on seeds 1 and 2, main b4c6206 and the lane in turn on an otherwise quiet machine, two rounds with the order swapped; the games' own time): the NFL -0.6% (12.72 s to 12.65 a run; pairs -1.6% to +0.0%), FBS -1.3% (13.43 to 13.25; -3.0% to +1.5%), high school -1.4% (9.07 to 8.95; -5.2% to +1.2%). Over main 0972bef the same runs gave +0.8%, +1.1% and -0.3%, over a7c9bf6 +1.4%, +1.3% and +1.6%, over 569e61f +1.8%, +0.2% and -0.8%, over 36b6367 -0.2%, -1.4% and +1.6%, and over dcdde89 +0.6%, +2.1% and +1.1%: the lane costs at most about 2%. The feed reads six ratings about the typical starter for each man in the reads, a zone defender's break reads the target's route, and a throw that reads a second tight end works out its reads once.

Tried and dropped (`fit_check` on FBS seed 151, the West Coast and Air Raid arms, on the lane's first builds before SF2 merged, unless a line says otherwise):

- **The window as it is for a better reader, with no exception** (each defender's break and each lane as they are): +0.3 to +0.6 points a game, but he threw deeper than the typical starter (+0.4 to +0.5 yards of air a throw) at a lower completion rate (-0.2 to -0.8 points) and his picks didn't fall (-0.06 to +0.04). He stopped respecting the one man who picks a throw, the underneath defender reading his eyes, so the lurking man came back as the one exception.
- **The lurking man within 3 yards of the throw's line** in place of 5: picks +0.02 and +0.15 points of throws (the second past the noise).
- **A better reader seeing where a defender drifts as the ball goes** (his motion carried to the catch): picks up 0.11 points of throws in Air Raid.
- **More trust in tight windows for a better decider** (`DECISIONS_TRUST` at 0.03): alone, the old trade grew (0.9 to 1.0 yards shorter, 0.6 to 0.8 more sacks a 100 dropbacks, -0.3 to -1.1 points a game); on top of the final, picks -0.07 to -0.10 points of throws, but 0.3 yards shorter and no more points (-0.1 to -0.2): the trade item 5 takes out.
- **The read's noise falling past its floor** (the read noise about the typical starter, without the 0.2 floor): the same careful passer as main (picks -0.15 to -0.20, 0.4 to 0.5 yards shorter). The floor is real (`GAPS.md`) but isn't what made him careful.
- **A better decider leaning to the deeper of two open men:** no change past the noise.
- **Route running, other levers against zone:** the zone defender's read lag growing with the route (0.006 to 0.02 s a point), the deep defender's lag on the deep threat, a wider room where he sits down (0.04 a point about the typical starter), and the throw's timing on his route. None gave both separation against zone and his catch rate there; the wider room took his catch rate against zone down 2.5 to 3.0 points, the throw's timing took separation away. `ZONE_TURN` at 0.05 moved less than 0.1 (the slot's separation against zone +0.07 yards against +0.20).
- **The contested edge read with his Strength and Jumping** (0.5 Contested Catch, 0.25 Strength, 0.25 Jumping, at 0.02 to 0.05 a point): on the test rosters the same as the final, but world seed 1's NFL clubs' breakups fell 16% and their completion rate rose 1.3 points, since their receivers carry 23.6 points more Strength than the typical starter (`GAPS.md`).
- **Height:** the high reach read on every ball, not just a ball up high (its floor at 0.5 and at 1, with the edge above and the fight doubled): a 6'4" X's contested catch rate +1.6 and +3.2 points against the typical man, but it reads his jump on a chest-high ball, where nobody jumps; his frame (height and arms) read on lower balls at 0.3 a 10 inches: +1.4, inside the noise; the fight after the ball at four times its spread about the typical starter: +0.2, and it reads his Strength. None is in.
- **The feed reading his forty on a deep route** (a share k of his read, 0.15 to 0.35, from his forty about the typical starter's, on a route that goes 15 yards or more; the lane's build over main dcdde89): a fast X drew 3.6 to 4.4 more targets 20+ yards a season (1.2 without), and his targets ran 1.4 to 1.7 yards deeper (0.3 without) at a catch rate 2.4 to 3.4 points lower (0.9 without), and he scored no more (+0.12 to +0.28 a game, +0.37 without; FBS seed 151). Across the league the slope of deep targets on the forty barely moved (world seed 1's NFL clubs -1.78, -1.84 and -1.90 points a 0.1 s at 0, 0.15 and 0.25; real -2.85).
- **The best receiving tight end in the first place on every throw that reads a second one:** in 13 personnel the first tight end drew 60% of the tight ends' targets (real 47%), and in 12 the second 36% (real 43%; the table above).
- **Hands at 0.0029 a point** (sized on a spread of Catching between real starters of about 6 points): wide receivers' drops spread 1.49 points between men on world seed 1's NFL clubs (real 0.54), tight ends' 1.24 (real 1.01) and backs' 2.18 (real 1.12).

Still off (`GAPS.md`, Engine): picks come from the wrong throws (item 5's fewer picks, lane SF5); height pays in contested targets, not in the catch (item 6, lane SF5); outside receivers' route running buys no separation against zone; fast receivers draw too few deep targets (lane SF10); in 13 personnel the first tight end still draws less than real (lane SF10); wide receivers' drops spread wider than real in the world; the typical starter isn't the world's; `fit_check`'s typical team feeds nobody; and the call table's fixed teams aren't typical. The passer still throws to his second read more than his first, and the read's noise has a floor (both found in this lane, before it).

## Injuries on every collision, strains and the sideline (engine gaps lane EG1, 2026-11-18)

Engine gaps items 1-3 (`docs/design/engine-gaps.md`, Cain's yes 2026-11-17): every collision can hurt both men by its force, men get hurt with nobody touching them, and a hurt man goes to the sideline and comes back after a few snaps, a series or not at all, with the concussion rules of his level and era. The targets: each position's share of the injuries that cost a week or more near real (nflverse game-status reports 2012-2025: offensive linemen 17.7%, backs 7.4%), the total where main has it (1.081 a team game in the NFL; real about 1.10), the body parts kept at nflverse's, and about a quarter of injuries with nobody touching him (the mix's own non-contact shares).

What was wrong on main (`AFM_GAPS=1 match_check`, NFL and FBS 2,000 games, high school 1,000, the default seeds):

- **Only the tackle and the sack hurt anyone**: the man taken down, his tacklers and the sacked quarterback. Backs took 35.3% of the NFL's week-plus injuries (real 7.4%) and offensive linemen 0.1% (real 17.7%). The hit at the catch point, the hit on a throw, the sacker, a late hitter, the pile and blocks were never checked, nor a tackle that forced a fumble or an arm tackle the runner broke.
- **Nobody got hurt with nobody touching him**: hamstrings, groins and calves came only from hits, and there was no torn Achilles.
- **A hurt man played on or was out for the game**, and the world dropped every injury a man finished the game on. Concussions had one rule at every level and era.

What changed (every size proposed, not confirmed; how it works, with every size, in `docs/reference/how-it-works/injuries.md`, "In games"):

- **Every collision checks both men** (`game/sim.rs`, `injury_check`, each with a `Cause`): the tackle (before the fumble roll), an arm tackle the runner breaks at 0.6 of the force, the sack and the sacker, the hit as he throws and a late hit on a slide (flagged or not), the hit at the catch point that knocks the ball loose, the pile (every man within 1.5 yards of a man going down, either side, by the weight going down), a block as it starts and a pancake. Each kind has its share of a hit's chance (`CARRIER_SHARE` 0.18, `TACKLER_SHARE` 0.26 and the rest).
- **The block's grind** (`Agent::grind`, `Sim::wear`): each man's seconds in blocks and the weight on him, rolled at the play's end. Without it the line stayed near half its real share.
- **Strains and plants with nobody touching him** (`Agent::sprint`, `plant`, from `Agent::steer`; `Sim::wear`): seconds near top speed, and the speed braked, turned and pushed off with, by his momentum at top speed (big fast men load their legs more), tired legs (1 + 1.2 × tired) and turf (× 1.45, now on these only: contact injuries are about even on turf, Mack et al. 2019). The mix is split by each kind's real non-contact part (`non_contact_share`: hamstrings 95%, groins and calves 85%, ACLs 70%), with a torn Achilles as its own kind.
- **Each collision leans the kind** (`Cause::lean`): piles knees and ankles, blocks hands, shoulders and backs, tacklers heads, necks and shoulders, read against the average collision (`lean_norm` on `CAUSE_SHARES`), so the body parts stay at nflverse's.
- **The sideline** (`game/flow.rs`, `apply_injuries`, `sideline_returns`): he plays on hurt as before, or goes off and comes back after a few plays to a series or two by how bad it is and his toughness, or is done; a torn ACL, a torn Achilles or a broken bone never comes back. Back in, he plays hampered.
- **Concussions by level and era** (`game/rules.rs`, `Concussion::for_year`): out for the day from the NFL's 2009 rule and college's and high school's 2010; before them about half went back in and their time out was shorter (Pellman et al. 2005, Casson et al. 2010). Men pulled for a check and cleared come back after about 13 plays (high school sits them); the NFL's count is sized to 2025's 538 in-game evaluations for 168 game concussions.
- **The world** keeps every game injury after the game, one he finished the game on too (`world/pro/games.rs`, `record`, `record_preseason`; `world/pro/other.rs`; `training/seat.rs`, `game_injuries`), and a torn Achilles is always a major injury that heals worse than a torn ACL (`development.rs`, `hurt_class`, `major_heal`).
- **Tools**: `AFM_GAPS=1 match_check` prints each position's week-plus share against nflverse, a position by cause grid, injuries by cause, the body parts against nflverse, the sideline, and concussions and checks (`injury_counts`); the world log's injury rows keep `back` and `cause` (`world/wlog/games.rs`); the World Reader shows "back in after N plays"; game day's halftime line says when a man came back in.

`AFM_GAPS=1 match_check`, 2,000 NFL games on the default seed, main (fe3e2b3) and after:

| NFL, a team game | main | after | real |
|---|---|---|---|
| injuries costing a week or more | 1.081 | 1.084 | about 1.10 |
| all injuries | 2.050 | 1.994 | – |
| of the week-plus ones: offensive line | 0.1% | 17.6% | 17.7% |
| backs | 35.3% | 8.0% | 7.4% |
| receivers / tight ends | 16.9 / 4.4% | 12.2 / 5.8% | 13.2 / 7.4% |
| quarterbacks | 4.1% | 3.0% | 3.1% |
| interior line / edges | 3.0 / 4.8% | 9.9 / 11.4% | 8.9 / 8.0% |
| linebackers | 11.9% | 12.0% | 10.7% |
| corners / safeties | 9.2 / 9.6% | 11.1 / 8.5% | 13.7 / 9.1% |
| with nobody touching him, of all | 0 | 25.3% | 24.6% (the mix's shares) |
| knee / ankle / hamstring, of the week-plus | 16.1 / 13.8 / 13.6% | 17.8 / 13.9 / 13.1% | 17.2 / 14.3 / 12.9% |
| concussion / shoulder | 10.6 / 6.0% | 10.9 / 5.6% | 11.1 / 5.6% |
| of all injuries: played on / back in / out for the game | 33.4 / 0 / 66.6% | 33.8 / 11.7 / 54.5% | – |
| men back in the same game | 0 | 0.233, after 12.3 plays | – |
| concussions a game (caught) | 0.446 (92.7%) | 0.437 (92.0%) | 0.45 in 2024, 0.59 in 2025 |
| concussion evaluations a game concussion | 0.93 | 3.23 | 3.2 (2025: 538 for 168) |
| points | 24.73 | 24.84 | 22.5 |

Where the NFL's week-plus injuries come from after, a team game: blocks 0.350 (the grind most of it), plants 0.213, sprints 0.174, tackling 0.147, the man tackled 0.100, piles 0.081, the passer 0.019, the hit at the catch point under 0.001. The line's come from blocks (0.129 of its 0.190), plants (0.035) and sprints (0.015); the backs' from being tackled (0.061 of 0.087); the receivers' from sprints (0.054 of 0.133); the defensive line's from blocks (0.076 of 0.107 inside, 0.056 of 0.123 on the edge).

FBS (2,000 games) and high school (1,000), main and after:

| a team game | FBS main | FBS after | HS main | HS after |
|---|---|---|---|---|
| injuries costing a week or more | 0.956 | 0.957 | 0.558 | 0.588 |
| of them: offensive line / backs | 0.2 / 38.6% | 17.4 / 6.1% | 1.3 / 37.6% | 18.6 / 7.9% |
| with nobody touching him, of all | 0 | 24.0% | 0 | 23.1% |
| men back in the same game | 0 | 0.256 | 0 | 0.157 |
| concussion checks cleared a game | 0 | 0.336 | 0 | 0.075 (all sit the day) |
| points | 23.32 | 23.59 | 18.21 | 18.33 |

On four seeds (the default and 2-4, the same games on both builds) the default seed is the branch's best in the NFL. Week-plus injuries a team game, main (fe3e2b3) and after: NFL 1.091 and 1.061 (2.7% fewer), FBS 0.947 and 0.981 (3.6% more), high school 0.544 and 0.584 (7.3% more). The levels draw together: after, the NFL runs 1.08 times FBS (main 1.15) and FBS 1.68 times high school (main 1.74), where real college games run 2.5 times high school's (Kerr et al. 2018: 32.1 and 12.7 injuries costing a day per 1,000 players a game). On main the levels' order came from the man tackled alone, which falls with lighter, slower men (seed 2: FBS 0.85 of the NFL's week-plus injuries, high school 0.50). After, blocks and piles carry about 40% of them, and piles fall least: on the default seed FBS piles run 1.14 times the NFL's and high school's 1.09 (1.31 a play, as more men reach the ball), against 0.90 and 0.47 for blocks. No share was sized for one level, which would be a fudge (`GAPS.md`).

Re-measured after main took in SF1's play action and items 29 and 30 (main 590101e against the branch merged with it; the same four seeds and games on both builds, the mean of the four): week-plus injuries a team game 1.098 to 1.058 in the NFL, 0.953 to 1.004 at FBS and 0.542 to 0.599 in high school. Of the NFL's, offensive linemen 0.2% to 16.6% (real 17.7%) and backs 35.8% to 7.3% (real 7.4%); 25.5% of all injuries come with nobody touching him. The NFL runs 1.05 times FBS (main 1.15) and FBS 1.68 times high school (main 1.76). Points 25.81 to 25.96 (NFL), 24.44 to 24.52 (FBS) and 18.21 to 18.07 (high school). The games take 4-5% longer: 2,000 NFL games 12.6 s to 13.15 s, 2,000 FBS games 13.25 s to 13.9 s, 1,000 high school games 4.7 s to 4.9 s. Again over SF2 (main dcdde89) and over minor bugs 409-412 (main ad17262), the same seeds and games: week-plus injuries a team game 1.105 to 1.069 and 1.080 to 1.070 in the NFL, 0.954 to 0.998 and 0.957 to 1.007 at FBS, 0.558 to 0.587 and 0.544 to 0.609 in high school. Over ad17262 the NFL's offensive linemen go from 0.1% to 17.0% of its week-plus injuries and its backs from 34.6% to 7.8%, the NFL runs 1.06 times FBS (main 1.13) and FBS 1.65 times high school (main 1.76), points go 26.78 to 26.96 (NFL), 25.08 to 25.07 (FBS) and 18.39 to 18.45 (high school), and the games take 3-6% longer (NFL 13.1 s to 13.5 s, FBS 14.0 s to 14.5 s, high school 4.8 s to 5.1 s). And over EG2's ball security (main 569e61f), the same seeds and games: week-plus injuries a team game 1.103 to 1.079 in the NFL, 0.953 to 0.993 at FBS and 0.562 to 0.610 in high school; the NFL's offensive linemen 0.2% to 16.5% and its backs 34.6% to 7.5%; the NFL 1.09 times FBS (main 1.16) and FBS 1.63 times high school (main 1.70); points 26.74 to 26.97 (NFL), 25.13 to 25.38 (FBS) and 18.60 to 18.62 (high school); the games take 5-7% longer (NFL 12.6 s to 13.25 s, FBS 13.45 s to 14.35 s, high school 4.65 s to 4.95 s). The loose ball, strip sacks and fumbled snaps leave the injuries by cause where they were over ad17262 (all injuries a team game in the NFL: sprints 0.21 against 0.21, plants 0.29 against 0.30, blocks 0.76 against 0.75). And over EG5's rule books and flags (main a7c9bf6), the same seeds and games: week-plus injuries a team game 1.097 to 1.063 in the NFL, 0.963 to 1.008 at FBS and 0.554 to 0.610 in high school; the NFL's offensive linemen 0.2% to 16.8% and its backs 35.6% to 7.5%; the NFL 1.05 times FBS (main 1.14) and FBS 1.65 times high school (main 1.74); points 27.55 to 27.78 (NFL), 26.60 to 26.66 (FBS) and 20.00 to 20.06 (high school); the games take 3-6% longer (NFL 12.35 s to 12.88 s, FBS 13.23 s to 13.63 s, high school 4.53 s to 4.80 s). EG5's fouls leave the injuries by cause where they were over 569e61f (all injuries a team game in the NFL: sprints 0.20 against 0.21, plants 0.30 against 0.29, blocks 0.76 against 0.76). And over EG3's kicking game (main 0972bef), the same seeds and games: week-plus injuries a team game 1.109 to 1.063 in the NFL, 0.957 to 1.005 at FBS and 0.547 to 0.610 in high school; the NFL's offensive linemen 0.2% to 16.5% and its backs 35.4% to 7.4%; the NFL 1.06 times FBS (main 1.16) and FBS 1.65 times high school (main 1.75); points 27.33 to 27.44 (NFL), 26.06 to 25.91 (FBS) and 19.88 to 19.59 (high school: each of the four seeds 0.18 to 0.41 lower; over EG5 the same rise in injuries left two seeds higher and the mean up 0.06, so this is likely the games' own spread, not measured further); the injuries by cause again where they were (sprints 0.21, plants 0.29, blocks 0.75); the games take 3-6% longer (NFL 18.70 s to 19.30 s, FBS 19.85 s to 20.45 s, high school 6.78 s to 7.15 s; the machine ran slower that hour, not EG3: main a7c9bf6's binary took 18.5 s beside main 0972bef's 18.3 s, back to back). EG3's blocked kicks played live go through the same sideline calls, and their play's work is rolled at its end as on a return (`kicking::blocked_live`, `flow::blocked_notes`); the rush on a kick nobody blocks is worked out, not tracked, and hurts nobody (`GAPS.md`). And over EG4's end of halves and SF4's reads and receivers together (main 60baecd), the same seeds and games: week-plus injuries a team game 1.078 to 1.056 in the NFL, 0.934 to 0.983 at FBS and 0.524 to 0.578 in high school; the NFL's offensive linemen 0.3% to 16.5% and its backs 34.7% to 7.2%; the NFL 1.07 times FBS (main 1.15) and FBS 1.70 times high school (main 1.78); points 27.54 to 27.70 (NFL), 25.88 to 25.87 (FBS) and 19.61 to 19.57 (high school); the injuries by cause again where they were (sprints 0.21, plants 0.30, blocks 0.74); the games take 2-8% longer (NFL 20.20 s to 20.93 s, FBS 22.05 s to 22.48 s, high school 7.20 s to 7.80 s, one high school seed 7.1 s to 8.5 s and the other three about 5% on average; the machine ran slower after a restart). EG4's laterals and fakes are tackled and piled as any play and get the same checks, and a man who goes down on his own late (`ClockRun::Down`) ends the play with no tackle; the Hail Mary's jump ball is settled by each man's reach and hands, so the crowd going up for it is not checked (`GAPS.md`). SF4 touches no injury check, and a passer its minor bug 423 keeps from sliding on the last play's laterals is tackled and checked as anyone.

Games missed by position on a small world, seed 271 (new; scale 0.05, three years of history with the log on from 2023 and one season on; the same seed on main fe3e2b3 and after). Counted from each regular-season NFL game's injuries, 2023-2026: every game of his team inside the time he is out (his weeks × 7 days from the day after, the medical staff's work left out), for player-seasons of 4 or more games, as nflverse counts them. The real count takes in practice, illness and every other cause too, so it sits above what game injuries alone should give.

| NFL world, games missed a player-season | main | after | real (every cause) |
|---|---|---|---|
| offensive line | 0.01 | 0.86 | 1.48 |
| backs | 3.09 | 0.79 | 1.32 |
| receivers / tight ends | 1.07 / 0.59 | 0.82 / 0.69 | 1.36 / 1.38 |
| quarterbacks | 0.87 | 0.65 | 1.38 |
| interior line / edges | 0.24 / 0.27 | 0.89 / 0.92 | 0.96 / 1.16 |
| linebackers | 0.76 | 0.86 | 1.22 |
| corners / safeties | 0.81 / 1.01 | 0.98 / 1.13 | 1.50 / 1.21 |
| kickers / punters | 0.08 / 0.03 | 0.10 / 0.06 | 0.56 / 0.25 |
| all | 0.72 | 0.83 | 1.2 |
| injuries that cost a game, a player-season | 0.31 | 0.32 | about 0.42 |
| game injuries costing a week or more, a team game | 0.897 | 0.984 | about 1.10 |

Main's world put the games missed on the backs (3.09 a player-season, 2.3 times real) and almost none on the line. After, every position but the kickers and punters sits at 47-94% of its real count: quarterbacks (47%), tight ends (50%) and the line (58%) farthest under, the interior line and safeties (93%) closest. The world's NFL games run fewer injuries than the test league on both builds (0.984 a team game after, against 1.084). The world's own season rows give 0.37 games missed a player-season on main and 0.39 after: they leave out games on injured reserve, which nflverse's count keeps (minor bug 415).

The world's college and high school games rise more than the test leagues. Week-plus injuries a team game: college 0.963 to 1.103 and high school 0.664 to 0.792, where the FBS test league went 0.956 to 0.957 and the high school one 0.558 to 0.588. Measured on the college games, about 60% of the rise is on grass and 40% the turf's lean:

- **On grass**, main's world ran 5% under the FBS test league (0.910 a team game, all of it the man tackled) and the world after runs 4% over it (0.991). Its piles hurt 0.25 men a team game against the test league's 0.18, on the same 67 plays a team game; every other cause is within 4%. Why the world's piles run higher is not measured yet (`GAPS.md`).
- **On turf** (60% of the world's college games; `match_check` plays every game on grass), week-plus injuries run 1.19 times grass after, against 1.10 on main: strains and plants, about 36% of them on grass, take the whole 1.45. All injuries run 1.13 times grass after and 1.11 on main, the total the move kept.

Speed: `match_check`'s games take 2-4% longer in the NFL and FBS and 10% longer in high school. Two runs each on an idle machine (4 cores, the same build settings): 2,000 NFL games 13.75 s on main and 14.3 s after, 2,000 FBS games 14.7 s and 15.05 s, 1,000 high school games 5.05 s and 5.55 s; the whole NFL run 16.6 s and 17.1 s.

Tried and dropped:

- **The first sizes** gave 1.56 week-plus injuries a team game in the NFL: plants, summed over every cut of a play, outweighed everything, the man tackled was rolled again in his own pile, and the line still came short. The man himself left his own pile, strains and plants went on his momentum at top speed, and the block's grind came in; then the shares were sized down to main's total.
- **A hit's force on the change of speed or on the lighter man's side**: lighter, slower high school players would take more of it, where real high school rates run below college's, so a hit's force stays both men's momentum.
- **A pile at every pancake**: almost no weight goes down there, so it changed nothing worth its code.
- **Strains on speed alone**: high school legs would strain as often as the NFL's; his momentum at top speed keeps the levels in order.

Still off:

- Edges take 11.4% of the NFL's week-plus injuries (real 8.0%), corners 11.1% (13.7%) and tight ends 5.8% (7.4%). Part may be the real side: nflverse lists many edge rushers as linebackers, and the real share weighs each position's rate by an unsourced estimate of how many men a team plays there (`match_check`). A share for one position would be a fudge, so none was added.
- Knees 17.8% of the week-plus ones (real 17.2%).
- High school week-plus injuries rose 5% on the default seed (0.558 to 0.588), 7% on four and 10% on four over main 590101e, most of it piles, where more men are around the ball (0.088 a team game, against 0.081 in the NFL).
- The levels draw together on four seeds (the NFL 1.08 times FBS after, 1.15 on main; FBS 1.68 times high school, 1.74 on main; over main 590101e 1.05 and 1.68; real college games 2.5 times high school's): piles fall least from level to level.
- In the world, college and high school week-plus injuries rose 15% and 19% (above): turf, and piles that run higher than the test league's.
- The world's NFL season rows read 0.39 games missed a player-season against nflverse's 1.2, as they leave out games on injured reserve (minor bug 415).
- The hit at the catch point almost never hurts anyone (under 0.001 a team game): the engine rarely knocks a ball loose with a hit, and a catch held on to is a tackle.
- The NFL's game concussions are 0.44 a game (real 0.45 in 2024, 0.59 in 2025).
- The sideline's sizes (11.7% of injuries back in, after about 12 plays) have no real count on file to check against.
- Playing on hurt or back in costs nothing past the game, the sideline reads only the injury and his toughness, and its play count runs through halftime (`GAPS.md`).

## Where picks and breakups come from (scheme-fit lane SF5, items 15, 16 and 17, 2026-11-20)

Design: `docs/design/scheme-fit.md`, items 15, 16 and 17, each by the floor: a deep defender can play the ball from depth (30+ yard throws picked 6.7% of the time, safeties 39% of the picks and about 43% behind man, the league's total held), a rusher in a block doesn't pick passes through the lane, and breakups by linebackers and defensive backs come to real (8.2 a 100 throws, picks left out). Built under Cain's proxy on 2026-11-20.

How it was measured: a new read-out in `match_check` (`AFM_PICKS=1`, `pick_diag`) over every targeted throw: by air yards (share, picked, complete, broken up by the line and by the back seven) against the 2024 NFL play-by-play; picks by the interceptor's position (all, behind man coverage, on 30+ yard throws) against FTN charting; the line's picks made in a block; and breakups a 100 throws by position, picks left out. The test rosters at 2,000 games on seeds 1 and 2 at each level, main 60baecd and after, with `AFM_GAPS=1` for pass interference. `fit_check` on NFL seed 161 (2,400 games an arm) and FBS seed 151 (3,000), main and after, on 32 arms: the bases, the passer's Decisions, the corners' and the free safety's coverage, hands and jump, a 6'2" corner, and the X's Contested Catch, Catching and height.

What was wrong on main (NFL test rosters, seeds 1 and 2):

- **Rushers picked passes through the lane.** The lane check let any defender within 1.1 yards of a short ball's path touch it, and a rusher faces the passer, so he counted as a man with his eyes on the throw. The line made 14.3% and 18.1% of the picks (real 4%, plus 5% by 3-4 outside linebackers), 88% and 91% of them while in a block, and broke up 3.2 and 3.3 throws a 100 (real 2.0), on top of the batted balls at the line. At FBS the line made 17% to 19% of the picks, at high school 19% to 22%.
- **A deep defender seldom played the ball.** Only a man facing the passer at the catch read as seeing the ball. A deep defender runs to the ball's spot facing where he runs, so the man nearest a 30+ yard ball on target had it in his eyes 15% of the time, and played it at 0.45 of the pick chance with less of his jump. 30+ yard throws were picked 4.46% and 4.64% of the time (real 6.72%), 20-29 yard throws 5.05% and 4.21% (real 6.06%). Safeties made 25.9% and 26.8% of the picks (real 39%), and 18.3% and 18.0% behind man (real 43%).
- **Breakups nobody was credited with.** A ball the contest at the catch took from the receiver fell incomplete as nobody's. An off-target ball that came to a defender fell incomplete with no play for him. The second man at the ball never got a play when the first didn't end it. And a man got a hand to a ball only inside the reach he catches it in. The back seven broke up 4.89 and 4.20 throws a 100 (real 8.2).

What changed (`game/scrimmage.rs`, `catch_point`; `game/sim.rs`, `Agent::saw_throw`; every size proposed, not confirmed):

- **A rusher reads no lane** (item 16). A man still rushing the passer, one who hasn't read the screen and peeled off for the back, gets his hands to a pass only at the line, as the batter (`batter`), which is near real on its own.
- **The ball in his eyes** (item 15; `ball_in_eyes`, `FIND_BALL`). A defender who saw the ball leave the passer's hand (`throw`, `Sim::sees`) and has had 1.2 s since he broke on it has found it over his shoulder as he runs to its spot. So he plays it as a man facing it: the full pick chance and his full jump. A man who only felt the throw from his receiver's eyes and hands plays the hands. On a short throw the 1.2 s seldom come, so there he plays the hands too.
- **The second man at the ball** (item 15). After the nearest defender, the next nearest gets his own play: the man over the top of a long ball, or the second man of a bracket. Each has his own reach, pick and breakup.
- **A fingertip for a breakup** (item 17; `BREAKUP_REACH`). A man with the ball in his eyes gets a hand to it 0.3 yards past the reach he catches it in. Picks and the hit at the catch still need his catch reach.
- **The contest's share is his breakup** (item 17). What the contest at the catch takes off the catch is now the nearest defender's breakup (6% of them picked, as `PBU_PICK`). It is rolled ahead of the catch, so the catch rate is the same.
- **An off-target ball that comes to him** (item 17). If it comes within his breakup reach and under his high reach, the nearest defender knocks it away with the breakup's chance (`breakup_hands`), with no receiver to fight him for it.
- **The read-out** stays in `match_check` (`AFM_PICKS=1`) for the lanes after this one.

Where picks come from, NFL test rosters (seeds 1 and 2, main > after; the band rates are of the targeted throws in each):

| | main | after | real |
|---|---|---|---|
| picked, % of targeted throws | 2.47, 2.23 | 2.40, 2.17 | 2.3 |
| behind the line | 0.92, 0.88 | 0.57, 0.57 | 0.80 |
| 0-9 air yards | 1.91, 1.74 | 1.49, 1.29 | 1.22 |
| 10-19 | 3.80, 3.39 | 4.20, 3.77 | 3.39 |
| 20-29 | 5.05, 4.21 | 6.58, 5.89 | 6.06 |
| 30+ | 4.46, 4.64 | 5.32, 5.62 | 6.72 |
| the line's share of the picks | 14.3, 18.1 | 3.4, 4.6 | 4 (plus 5 by 3-4 outside linebackers) |
| of the line's picks, made in a block | 88, 91 | 79, 85 | |
| the safeties' share | 25.9, 26.8 | 30.5, 32.9 | 39 |
| behind man | 18.3, 18.0 | 26.5, 27.7 | 43 |
| on 30+ yard throws | 51.9, 56.0 | 54.7, 61.6 | 56-61 |
| breakups a 100 throws, the back seven | 4.89, 4.20 | 7.33, 6.39 | 8.2 |
| the line | 3.20, 3.30 | 1.72, 1.85 | 2.0 |

Breakups by air yards after (the line and the back seven, a 100 throws in each; real 2024): behind the line 4.14 and 3.70 (7.38), 0-9 7.65 and 6.79 (8.21), 10-19 13.30 and 12.41 (13.60), 20-29 15.75 and 15.07 (15.91), 30+ 12.98 and 12.22 (18.84).

FBS (seeds 1 and 2): picked 2.05% and 1.96% to 2.00% and 1.86% of targeted throws; the line's share 17.3% and 18.5% to 4.5% and 4.4%; the safeties' 25.4% and 29.6% to 33.9% and 35.9% (behind man 21.4% and 25.0% to 29.8% and 32.2%); 30+ yard throws picked 2.76% and 3.32% to 3.97% and 3.74%; breakups a 100 throws by the back seven 3.10 and 2.70 to 5.51 and 4.81, by the line 2.77 and 2.77 to 1.31 and 1.46. High school: picked 1.70% and 1.56% to 1.46% and 1.37%; the line's share 19.4% and 21.9% to 4.3% and 3.4%; the back seven's breakups 1.64 and 1.57 to 4.02 and 3.79, the line's 2.69 and 2.82 to 1.17 and 1.31.

The league (2,000 games on seeds 1 and 2 at each level, the mean, main > after, the range of the change between the seeds):

| | NFL | FBS | high school | real NFL |
|---|---|---|---|---|
| points a team game | 28.11 > 27.77 (-0.42 to -0.28) | 26.00 > 25.77 | 19.85 > 20.19 | 22.51 |
| yards an attempt | 7.72 > 7.70 (-0.02 to -0.01) | 7.74 > 7.81 | 6.99 > 7.10 | 7.05 |
| completion rate | 66.5 > 67.1 (+0.5 to +0.8) | 61.7 > 62.5 | 58.6 > 59.7 | 64.5 |
| interceptions a team game | 0.817 > 0.787 (-0.030 to -0.029) | 0.649 > 0.617 | 0.356 > 0.310 | 0.756 |
| pick sixes a team game | 0.037 > 0.036 | 0.025 > 0.022 | 0.011 > 0.006 | 0.050 |
| broken up, % of attempts | 8.7 > 9.9 (+1.0 to +1.3) | 6.2 > 7.4 | 4.8 > 5.8 | |
| third down conversion | 45.7 > 46.1 (+0.3 to +0.4) | 42.3 > 42.6 | 40.5 > 40.8 | 39.8 |
| pass interference accepted a team game | 0.557 > 0.585 (+0.016 to +0.041) | 0.498 > 0.503 | 0.296 > 0.302 | 0.497 |
| explosive pass rate | 8.8 > 8.6 | 9.2 > 9.2 | 7.5 > 7.6 | 8.8 |
| red zone touchdowns, % of attempts | 28.6 > 28.3 | 29.0 > 29.1 | 27.4 > 28.0 | 19.0 |
| sack rate | 3.2 > 3.3 | 4.1 > 4.1 | 5.8 > 5.6 | 6.8 |

Why:

- **Picks hold at the NFL** (2.35% to 2.29% of targeted throws over the two seeds, real 2.3%; 0.817 to 0.787 a team game, real 0.756). The line's picks fall from 0.35 and 0.40 points of the throws to 0.08 and 0.10, and the long balls and the second man give most of it back from the throws a defender can play: 20+ yard throws are picked about a quarter more often, throws short of 10 yards a quarter to a third less.
- **Completions rise 0.5 to 0.8 points** (66.5% to 67.1% at the NFL, real 64.5%). The rushers' touches in the lane took about 2% of the throws, and the new breakups win back more than half of what that gives the offense. The rest is the short game's own: throws short of 10 yards are still completed 3 to 4 points above real (still off, below).
- **FBS and high school picks fall** (FBS 0.649 to 0.617 a team game, real about 0.80; high school 0.356 to 0.310). The line made more of their picks than the NFL's (17% to 22%), and they throw fewer long balls. FBS was short of real before (still off, below).
- **Pass interference rises 3% to 9%** (NFL 0.557 to 0.585 accepted a team game over main 60baecd, +3% to +7% by seed; 0.559 to 0.596 over c0beb23; real 0.497). Each breakup at the catch checks the contact for interference (`pi_contact`, at 1.2 times the rate), and the fingertip and the second man add breakups there. A target it is at real: the real rates by air yards on the engine's own targets give 0.62 a team game, and the lane draws 94% to 98% of that (main 90%), 16-20 yard throws above real (0.035 a target against 0.027) and 21 yards and more below. The total is above real because the test rosters throw more passes past the line (35 attempts a team game against 33.3, 14% of targets behind the line against 18%), so the breakup's share of interference stays (still off, below).
- **Points fall 0.3 to 0.4 a team game at the NFL** and 0.2 at FBS, toward real, and rise 0.3 at high school, where picks fall most.

`fit_check` (NFL seed 161, 2,400 games an arm, main 60baecd and after, the typical team's 12 contexts; "past the noise" is |z| of 1.96 or more, marked *):

| item | measure | main | after | real |
|---|---|---|---|---|
| 15 | 30+ yard throws picked, against team 0 | 4.0 to 5.6 | 4.9 to 7.4 (11 of the 12 at 6.2 or more) | 6.7% |
| 14 | the safeties' share of team 0's picks | 21.4 to 25.1 | 25.2 to 30.3 | 39% |
| 14 | behind man | 9.5 to 19.0 | 14.5 to 25.3 | 43% |
| 16 | the linemen's share | 13.2 to 16.8 | 2.9 to 4.6 | 4% (plus 5% by 3-4 outside linebackers) |
| 16 | of theirs, made while in a block | 86.6 to 91.5 | 73.6 to 87.8 | |
| 17 | breakups by linebackers and backs, a 100 targeted throws | 4.9 to 5.5 | 7.7 to 8.8 | 8.2 |

What a man's ratings are worth (the change a season against the typical arm, main > after):

- **A free safety's Zone Coverage +15** picks +0.81*, +0.57* and +0.85* > +1.21*, +0.87* and +1.01* more (Cover 2, man pressure, West Coast) and breaks up +1.73*, +1.75* and +2.20* > +3.53*, +2.54* and +3.27* more. His Jumping +15 picks +0.27* > +0.33*, his Catching +15 +0.25* > +0.23*. The typical free safety picks 1.48 > 1.93 a season and breaks up 2.7 > 5.1 (West Coast).
- **A corner's Man Coverage +15** in man pressure picks +0.33* and +0.26 > +0.55* and +0.67* (the first and second corner) and takes 1.9* and 3.0* > 2.3* and 4.4* points off the catch rate on him. His Catching +15 picks +0.24* > +0.37*; his Jumping +15 +0.34* > +0.13. The typical corner picks 2.15 > 2.58 a season and breaks up 4.9 > 7.8 (West Coast).
- **A passer's Decisions +15** changes his picks by -0.01, +0.03, -0.12 and +0.06 > -0.10, -0.07, +0.00 and -0.18* points of throws in the four schemes: with the rushers' picks in the lane gone, more of his picks come from where he throws.
- **A receiver's Contested Catch +15** wins +7.6* and +7.7* > +11.5* and +10.7* points of his contested balls (Air Raid, West Coast): there are more breakups for his skill to beat. A 6'4" X wins them -0.5 > +0.3 points more often (neither past the noise), so height at the catch (item 6) stays open.

At FBS (seed 151, 3,000 games an arm): 30+ yard throws picked 3.1% to 4.3% > 4.0% to 5.8%, the safeties' share of the picks 21.7% to 24.5% > 26.6% to 30.2% (behind man 10.7% to 19.8% > 15.7% to 22.4%), the line's 15.3% to 18.2% > 3.3% to 4.9%, and breakups by linebackers and backs 2.8 to 3.3 > 5.3 to 5.9 a 100 throws. A free safety's Zone Coverage +15 picks +0.40*, +0.30* and +0.51* > +0.52*, +0.44* and +0.61* more and breaks up +0.94*, +0.80* and +1.05* > +2.13*, +1.64* and +2.01* more; a passer's Decisions +15 changes his picks by -0.05 points of throws on average, as on main.

Measured again over main c0beb23 (engine gaps lane EG1, injuries on every collision), the same seeds and games, with the same picture (main > the lane, seeds 1 and 2). At the NFL the league's picks 2.43% and 2.28% > 2.43% and 2.18% of targeted throws, 30+ yard throws picked 4.33% and 5.11% > 5.77% and 5.50%, the line's share of the picks 15.4% and 15.1% > 3.6% and 4.5%, the safeties' 25.2% and 27.5% > 31.6% and 32.7% (behind man 20.1% and 22.3% > 24.3% and 27.7%), the back seven's breakups 4.91 and 4.24 > 7.41 and 6.33 a 100 throws, completions 65.9% and 67.1% > 66.6% and 67.9%, pass interference 0.558 and 0.560 > 0.609 and 0.583 accepted a team game. At FBS interceptions 0.672 and 0.633 > 0.623 and 0.574 a team game, the line's share of them 17.4% and 17.3% > 3.6% and 4.1%, the back seven's breakups 3.03 and 2.62 > 5.56 and 4.79, pass interference 0.477 and 0.425 > 0.502 and 0.478. In high school interceptions 0.350 and 0.370 > 0.306 and 0.337, the line's share 20.3% and 23.3% > 3.7% and 3.3%, the back seven's breakups 1.62 and 1.54 > 3.94 and 3.81, pass interference 0.258 and 0.279 > 0.265 and 0.307. The games' time with the read-outs on moved -5% to +3% by level, inside the noise.

Speed (`match_check`, 2,000 games at each level on seeds 1 and 2, main 60baecd and the lane in turn on an otherwise quiet machine, two rounds with the order swapped; the games' own time): the NFL -2.0% (16.52 s to 16.20 a run; pairs -5.7% to +1.2%), FBS -1.3% (17.12 to 16.90; -8.3% to +7.8%), high school -2.1% (11.70 to 11.45; -13.8% to +6.2%), inside a pair's noise.

Tried and dropped (`match_check nfl 2000 1`, the lane's earlier builds):

- **A defender driving to the ball's spot** to arrive with it at full speed, where he now eases in near it (`go_to`): picks 3.06% of targeted throws (main 2.47%) and 10-19 yard throws completed 49.7% (real 54.5%).
- **Finding the ball in 0.8 s:** 10-19 yard throws picked 4.57% (real 3.39%), the league's picks 2.55%. In 1.5 s: 20-29 yard throws picked 5.13% (real 6.06%).
- **A pick for a defender level with the receiver on a long ball** (in the air 1.2 s or more and 20 yards or more: a share of the chance from 0.15 yards closer to the ball than him down to 0.15 or 0.35 yards farther): the league's picks 2.34% to 2.48% and 2.60% (at 0.3 and 0.5 yards), the 10-19 yard throws' with them, and the safeties' share fell 1.5 points: it favors the corner level with his man, not the man from depth.
- **A fingertip for every defender,** not only the one with the ball in his eyes: breakups by the back seven 7.65 a 100 on seed 1 at 0.3 yards, 7.1 and 8.0 at 0.5 and 0.8, with 10-19 yard throws completed 51.4%, 50.7% and 49.8% (real 54.5%).
- **The breakup's base at 0.50** (from 0.39): the back seven 6.22 a 100, and it moves every breakup, not the missing ones.

Also fixed: the fake field goal (lane EG4) seated its wall's guards and tackles by the depth chart, so a center you named by hand played right guard on it. It now seats them where your order names them, as on any snap (`fake_fg_play`, `pick_line`), the long snapper at center. The lane's new game stream drew one in the line-order test (`tests::line_order`). That test's share of snaps with the five named linemen on together now leaves out the snaps a named man is off hurt: since lane EG1 it swung with how many of them got hurt in its 24 games (on the lane's stream the NFL's five were on together on 76% of snaps with those snaps and 97% without, against the test's 80%), while the order's own check (a named man only ever at his spot) still reads every snap.

Still off (`GAPS.md`, Engine): long balls are off target too often (58% of 30+ yard throws; their completions 20% to 23%, real 28.9%, and their breakups 12 to 13 a 100, real 18.8), so they are picked 5.3% to 5.8% (real 6.7%); the safeties' share of picks (31% to 33%, real 39%; behind man 27% to 28%, real 43%) needs the free safety at the ball, scheme-fit items 12-14 (lane SF6); throws behind the line are broken up half as often as real (3.6 to 4.1 a 100, real 7.4) and short throws completed 3 to 4 points above real; pass interference at the NFL above real from the throws past the line (at real a target); FBS picks short of real; picks still come from throws the passer saw open (on the lane's last measured build 18% to 20% of them came from throws seen open by 2 yards or more, main 25% and 29%).

## The ballhawk safety (scheme-fit lane SF6, items 4, 12, 13 and 14, 2026-11-20)

Design: `docs/design/scheme-fit.md`, items 4 and 12-14. Eye Manipulation moves the safety off the throw, in a duel with his read of the routes (Play Recognition, and Football IQ with item 12) and of the passer's commit (Anticipation), the eyes-on-the-QB flaw biting more (item 4). A free safety's Football IQ reads the concept (item 12); his Anticipation reads the passer, so he breaks when the passer commits, before the ball is out, and breaking early on a fake burns him (item 13); his Ball Tracking reads the ball from depth (item 14, with the fix list's "Ball tracking for corners and safeties"). Zone Coverage and Play Recognition keep their jobs. Real (2024 NFL, FTN charting): safeties make 39% of the picks, 43% behind Cover 1. Built under Cain's proxy on 2026-11-20.

How it was measured: `fit_check` on the audit's seeds, main 28c7563 and after: the free safety's arms in the four defenses (Cover 2, West Coast's Cover 3, man pressure, quarters) at the NFL (seed 161, 2,400 games an arm) and FBS (seed 151, 3,000), and the passer's Eye Manipulation +15 (West Coast); `match_check` at each level on seeds 1 and 2 (2,000 games, `AFM_PICKS=1`, which now also splits each band's picks by position).

What was wrong on main:

- **The look-off pulled the safety toward the throw.** For his first 0.6 s on his first read, the passer's look-off pulled a deep man toward where he looked, his first read, which is often the throw. It read Eye Manipulation from 60, so the NFL's typical passer had 0.73 of a full one and college and high school passers next to none, and the pull stopped at 4 yards. Eye Manipulation +15 scored -0.19 points a game at the NFL and +0.14 at FBS (neither past the noise).
- **A deep man's Football IQ, Anticipation and Ball Tracking did nothing.** Football IQ was read for a defender only on busts and in a rusher's move, a deep man's Anticipation only leaned him toward the passer's eyes, and Ball Tracking was never read for a defender. A free safety's +15 in any of them changed his picks by -0.16 to +0.11 a season at the NFL and -0.05 to +0.10 at FBS, none past the noise.
- **Safeties made 31.6% and 32.8% of the picks** on the NFL test rosters (real 39%), 24.3% and 27.7% behind man (real 43%).

What changed (`game/scrimmage.rs`; every size proposed, not confirmed):

- **The look-off moves the deep man off the throw** (item 4; `decoy`, `look_bite`, `LOOK_OFF_TYPICAL`, `LOOK_ACROSS`). While the passer's eyes are off his first read, they go to the read farthest across the field from it, 8 yards or more, and pull a deep man's spot toward that read by 0.3 × look-off × bite × the gap, as far as his feet take him while it lasts; his lean to the first read falls by the look-off. The look-off reads Eye Manipulation against the level's typical starter: the typical passer has half a full one at every level, +15 a full one, and a better one pulls harder still. The bite is the deep man's side of the duel: half Anticipation, a quarter Play Recognition and a quarter Football IQ against his typical starter's, from the bite the NFL's typical safety had (0.757), more with his eyes stuck on the passer. With every read on one side the eyes only hold him.
- **Football IQ reads the concept** (item 12; `concept_lead`, `route_read`, `CONCEPT_LEAD`). A deep man reads each route 0.15 s ahead along its points a 10 points of Football IQ over his typical starter (behind, where it was, under him): which routes are in his area, which is the deepest and how he splits two verticals.
- **Anticipation reads the passer** (item 13; `passer_read`, `motion_break`, `MOTION_ANTIC`, `Sim::triggers`). A deep man who sees the passer breaks on the throw 0.12 s sooner a 10 points of Anticipation over his typical starter (later under him). When that lead beats his read of the ball, he breaks on the throwing motion, no sooner than 0.1 s into it, for the spot the passer leads his man to: before the ball is out. If it then goes to another man, he is out of place and reads it as it comes. The passer's window reads the lead as a sharp reader does (`window`, `ball_break`). The level's typical deep man breaks where he did.
- **Ball Tracking reads the ball** (item 14; `find_ball`, `FIND_TRACK`). A defender who saw the throw finds the ball over his shoulder 0.2 s sooner a 10 points of Ball Tracking over his typical starter (from lane SF5's 1.2 s, `FIND_BALL`), and from then he plays it as a man facing it: the full pick chance and his jump.

What a free safety's ratings are worth (`fit_check`, the change a season against the typical arm, main > after, in Cover 2, West Coast, man pressure and quarters; FBS played the single ratings in the first two; * past the noise, |z| of 1.96 or more):

| arm | measure | NFL main | NFL after | FBS main | FBS after |
|---|---|---|---|---|---|
| Anticipation +15 | picks | -0.04, -0.05, -0.16, +0.11 | +0.55*, +0.48*, +0.60*, +0.96* | -0.05, +0.10 | +0.47*, +0.42* |
| | breakups | -0.1, +0.0, +0.0, -0.1 | +1.5*, +1.2*, +1.6*, +1.7* | -0.0, +0.1 | +0.7*, +0.9* |
| | catch rate on him, points | +0.7, +0.9, +0.4, -0.3 | -2.1*, -1.4, -2.7, -3.6* | +0.7, -0.6 | +0.5, -1.9 |
| Ball Tracking +15 | picks | -0.01, +0.01 | +0.07, +0.14* | +0.00, +0.00 | +0.05*, +0.03 |
| | catch rate on him, points | +0.1, -0.0 | -0.8*, -0.8* | +0.1, +0.1 | -0.4*, -0.4 |
| Football IQ +15 | picks | -0.01, +0.01, -0.02, +0.00 | -0.15, -0.12, +0.01, +0.11 | +0.00, +0.01 | +0.05, +0.18* |
| Football IQ -15 | picks | +0.00 (West Coast) | -0.07 | -0.01 | +0.11 |
| Zone Coverage, Play Recognition, Football IQ and Anticipation +15 | picks | +1.05*, +1.23*, +0.84*, +1.12* | +1.50*, +1.62*, +1.49*, +1.86* | +0.82*, +0.83*, +0.73*, +0.78* | +1.03*, +1.30*, +0.96*, +1.45* |
| | points allowed a game | -0.73*, -0.83*, -0.24, -0.47 | -1.20*, -1.17*, -1.00*, -1.70* | -0.81*, -0.53*, -0.81*, -0.65* | -1.18*, -1.12*, -0.80*, -1.37* |
| the same four -15 (West Coast) | picks | -0.80* | -1.36* | -0.47* | -0.46* |
| | points allowed a game | +0.17 | +1.08* | +0.57* | +0.36 |
| Zone Coverage +15 | picks | +0.92*, +1.04*, +0.60*, +0.93* | +0.76*, +0.48*, +0.63*, +0.88* | +0.59*, +0.73* | +0.52*, +0.65* |

A passer's Eye Manipulation +15 (West Coast): at the NFL -0.19 > +0.41* points a game and +0.00 > +0.09* yards of separation on his targets; at FBS +0.14 > +0.60* points, -0.003 > +0.014* EPA a dropback and +0.02 > +0.11* yards.

The typical team (team 0 at its level's typical starter everywhere): its offense scores 27.87 > 28.40 a game at the NFL (West Coast) and 31.41 > 32.05 at FBS. Its defense allows 22.42, 23.35, 21.68 and 23.42 > 22.98, 23.20, 22.39 and 23.99 at the NFL, and 22.02, 21.86, 20.92 and 22.16 > 22.25, 22.80, 21.25 and 22.96 at FBS. Its safeties make 28.7%, 28.5%, 27.7% and 28.3% > 27.8%, 27.6%, 25.3% and 26.5% of its picks at the NFL (behind man 16.9%, 21.2%, 22.8% and 18.1% > 21.8%, 17.2%, 19.1% and 21.5%), and 29.4%, 28.4%, 26.9% and 29.3% > 28.6%, 27.0%, 25.6% and 25.5% at FBS.

The league (2,000 games on seeds 1 and 2 at each level, the mean, main > after, the change's range between the seeds):

| | NFL | FBS | high school | real NFL |
|---|---|---|---|---|
| points a team game | 28.00 > 28.64 (+0.47 to +0.81) | 25.96 > 26.48 (+0.39 to +0.65) | 20.22 > 20.54 (+0.27 to +0.36) | 22.51 |
| yards an attempt | 7.73 > 7.85 | 7.80 > 7.92 | 7.09 > 7.20 | 7.05 |
| completion rate | 67.2 > 67.3 | 62.5 > 62.7 | 59.8 > 59.8 | 64.5 |
| interceptions a team game | 0.796 > 0.790 | 0.599 > 0.605 | 0.322 > 0.330 | 0.756 |
| picked, % of targeted throws | 2.31 > 2.29 | 1.87 > 1.88 | 1.46 > 1.50 | 2.3 |
| 30+ yard throws picked | 5.64 > 6.02 | 3.84 > 3.97 | 3.27 > 3.29 | 6.72 |
| the safeties' share of the picks | 32.2 > 32.3 | 34.8 > 34.5 | 31.9 > 31.1 | 39 |
| behind man | 26.0 > 23.9 | 32.4 > 30.7 | 29.3 > 31.1 | 43 |
| third down conversion | 46.1 > 46.6 | 42.8 > 43.3 | 41.3 > 41.2 | 39.8 |
| explosive pass rate | 8.7 > 9.0 | 9.2 > 9.6 | 7.4 > 7.9 | 8.8 |
| pass interference accepted a team game | 0.596 > 0.584 | 0.489 > 0.470 | 0.286 > 0.281 | 0.497 |

Why:

- **Anticipation and Ball Tracking now make picks** (items 13 and 14). A free safety with Anticipation +15 breaks on the throw 0.18 s sooner, and on the throwing motion when that beats his read of the ball, so he gets to more balls: 0.48 to 0.96 more picks and 1.2 to 1.7 more breakups a season at the NFL, and up to 3.6 points off the catch rate on him. Ball Tracking +15 finds the ball 0.3 s sooner, so more of the balls he reaches he plays as a man facing them: 0.07 and 0.14 more picks and 0.8 points off the catch rate on him. His four reading ratings together are worth 1.5 to 1.9 picks a season and 1.0 to 1.7 points a game at the NFL (main 0.8 to 1.2 and 0.2 to 0.8), and as much the other way at -15.
- **Football IQ still moves nothing** (item 12, still off). At +15 and at -15, in every defense at both levels, his picks, the catches on him and the points allowed stay inside the noise. So where a deep man reads the routes before the throw is not what decides the deep ball at this size; his break on the throw (Anticipation, Zone Coverage) and the catch point (Ball Tracking) are, and both pay. The typical deep man already reads every route where it is now (underneath men read where it was, `ZONE_LAG`), and a route 0.2 s ahead is about 2 yards on at a receiver's speed.
- **Eye Manipulation pays** (item 4): 0.41 more points a game at the NFL and 0.60 at FBS, with 0.09 and 0.11 yards more separation. With the 4-yard stop kept (the lane's first build) it paid nothing (+0.12 and -0.22): a +15 passer's pull reached the stop on any read 18 yards across, the typical passer's only past 35.
- **Points rise 0.3 to 0.6 a team game** (NFL 28.00 to 28.64, real 22.51; FBS 25.96 to 26.48; high school 20.22 to 20.54). The typical passer's look-off now moves the deep man off the throw, where on main it pulled him toward it at the NFL and was next to none below, so the typical offense scores 0.53 more a game at the NFL and 0.64 at FBS. Every total stays inside its test band. The points were already high from elsewhere: sacks at half the real rate (3.2% of dropbacks, real 6.8%) and red zone touchdowns well above it (28.9% of attempts, real 19.0%), lanes SF8 and EG6.
- **The league's picks hold** (NFL 2.31% to 2.29% of targeted throws, real 2.3%): the typical deep man breaks where he did, and only better and worse readers than the level's typical starter break sooner or later. 30+ yard throws are picked a little more (5.64% to 6.02%, real 6.72%).
- **The safeties' share stays where it was** (NFL 32.2% to 32.3%, real 39%; behind man 26.0% to 23.9%, real 43%; the typical team's 25% to 28%). Two things hold it there (still off, below). Throws short of 20 yards are picked 13% to 15% above real (0-9 yards 1.38%, real 1.22%; 10-19 3.89%, real 3.39%), and linebackers and corners make 82% and 63% of those picks. And the typical deep man breaks on the ball, not on the throwing motion. Every deep man reading the motion (tried, below) lifted the safeties' share to 34% but put the league's picks at 2.7% of targeted throws, so the short picks come first. On the typical team the look-off takes a point or two off the safeties' share, as it moves them off the throws it opens.
- **Pass interference falls 2% to 4%** (NFL 0.596 to 0.584 accepted a team game, real 0.497), with the breakups a little down (10.0% to 9.8% of attempts at the NFL): each breakup checks the contact for interference (lane SF5).

The coaches' table (`call_values 2000` on main and after; its run rows don't move): the lane moves the tool's straight pass cells by 1.14 yards a cell (the mean of |change|; +0.47 on average), most of all fades against two-high safeties (against Cover 2 -0.65 to 11.19 yards a snap, against Cover 6 1.38 to 11.17: the look-off no longer pulls the half safety onto the throw) and the shallow cross against Cover 4 (24.28 to 12.12). Those rows already wait for minor bug 405. The rows the table takes from a new run (the three play-action concepts and the fakes' changes, `tools/call_values/compose.py`) move 1.12 yards a cell with the lane, less than main's own new run moves them off the committed table (1.37), so the committed table stands, as at lane SF4.

Speed (`match_check`, 2,000 games at each level on seeds 1 and 2, main 28c7563 and the lane in turn on an otherwise quiet machine, two rounds with the order swapped; the whole run's time): the NFL +1.3% (20.20 s to 20.46 a run; pairs -0.4% to +4.9%), FBS -0.4% (21.53 to 21.44; -2.9% to +1.4%), high school +2.2% (14.96 to 15.29; -2.9% to +6.9%), inside a pair's noise.

Tried and dropped (`match_check nfl 2000 1`, the lane's earlier builds):

- **Every deep man reading the throwing motion**, the level's typical one too: the league's picks 2.67% to 2.70% of targeted throws (main 2.43%, real 2.3%) and the safeties' share 34%; with passers whose window knew it, the throws went short instead (0-9 yards 57.7% of them, real 49.7%; 20 and over 8.7%, real 11.4%).
- **The look-off's 4-yard stop kept:** Eye Manipulation +15 +0.12 points a game at the NFL and -0.22 at FBS, neither past the noise.

Still off (`GAPS.md`, Engine): the safeties' share of picks (the short throws' picks and the typical deep man's break), a deep man's Football IQ moving nothing (item 12) and Ball Tracking at the catch point (a defender runs to where the ball comes down as if he knew it, whatever his tracking).

## Coverage jobs (scheme-fit lane SF7, items 18, 19 and 20, 2026-11-20)

Design: `docs/design/scheme-fit.md`, items 18-20. Man Coverage keeps paying at the top, by the floor and by Cain's rule never to cap freaks (item 18). A blown coverage leaves a man free (item 19). The strong safety's depth and fits are the coordinator's call, and his skills decide how well he plays them, by Cain's rule that the coach picks the job and the engine reflects the player (item 20).

How it was measured: `fit_check` on the audit's seeds, main ffa2cfd and after: the first and second corners' Man Coverage in man pressure and West Coast (+15 and -15 for the first), the strong safety's Man Coverage +15 in both and his Tackling +15 (West Coast), and the Mike's Football IQ -15 (West Coast), at the NFL (seed 161, 2,400 games an arm) and FBS (seed 151, 3,000), with the tool's new item 19 rows (dropbacks with a bust in team 0's coverage, the EPA of those and of the rest); `match_check` at the NFL and FBS on seeds 1 and 2 and at high school on seeds 1 and 2 (2,000 games, `AFM_GAPS=1`); `ab_check s_weight nfl` (a safety's weight, the box read).

What was wrong on main:

- **Man Coverage stopped paying at the top.** A man defender trailed his man's break by a lag that fell 0.03 s a point of Man Coverage and stopped at 0, which a corner reached 3 to 5 points above his level's typical starter against a typical starting receiver. In man pressure the first corner's Man Coverage +15 saved 0.021 EPA a dropback at the NFL where -15 cost 0.037.
- **A bust in a zone cost nothing.** A busted zone defender played his own area 5 yards shallower and 40% narrower, so a dropback with a bust in West Coast's zones went for no more than the rest (NFL 0.075 EPA against 0.090, FBS 0.105 against 0.142). A busted man defender played a hook zone, so his man was free there (man pressure: NFL 0.139 against 0.069, FBS 0.146 against 0.111). A Mike with Football IQ -15 busted 2.2 more times a season at the NFL and 8.0 more at FBS, and his defense allowed no more points.
- **A safety's ratings set his depth and fit.** The rotated-down safety's depth and both safeties' run fits read his hitter read (weight, Tackling, Strength and Play Recognition against Speed, Zone and Man Coverage), so a better cover man stood deeper and fit later: his Man Coverage +15 cost 0.82 run stops a season at the NFL in man pressure and 0.41 and 0.54 at FBS.

What changed (`game/scrimmage.rs`; every size proposed, not confirmed):

- **No floor on the trail** (item 18; `man_job`, `cover_inner`, `route_ahead`). Below a lag of 0 a man defender reads his man's break before it comes, as the trail's mirror: he plays where the route has his man -lag s ahead along its points at his speed now, less the route's heading there times -lag, so on a straight stem he runs even with his man and at a break he turns onto the break's line -lag s early. The lag's top (0.75 s) stays.
- **A bust leaves his man or his area free** (item 19; `assign_defense`). A coverage man who busts takes the job of the nearest coverage man who didn't: the same man, with his own matchup's lag (`man_job`), or the same zone, so two play one and his man or area is nobody's. With nobody to copy he plays a short zone where he stands. Busts are drawn as before.
- **A safety's depth and fit are his job's** (item 20; `ss_depth`, the read loop, `synthetic::typical_hitter`). The rotated-down safety stands 8.1 + 0.9 × j yards deep whoever plays it, and each safety fits the run with his slot's lean (the level's typical free or strong safety's hitter read). The hitter read is his staff's read of him, and only the call reads it (the shell lean).

What the ratings are worth (`fit_check`, the change against the typical arm, main > after; * past the noise, |z| of 1.96 or more):

| arm | measure | NFL main | NFL after | FBS main | FBS after |
|---|---|---|---|---|---|
| CB1 Man Coverage +15, man pressure | EPA a dropback allowed | -0.021* | -0.030* | -0.015* | -0.021* |
| | points allowed a game | -0.40 | -0.59* | -0.47 | -0.29 |
| | catch rate on him, points | -3.2* | -5.7* | -3.3* | -6.1* |
| | yards a target on him | -0.47* | -1.03* | -0.61* | -0.91* |
| CB1 Man Coverage -15, man pressure | EPA a dropback allowed | +0.037* | +0.029* | +0.013 | +0.033* |
| | points allowed a game | +0.55 | +0.77* | +0.06 | +0.74* |
| CB2 Man Coverage +15, man pressure | EPA a dropback allowed | -0.009 | -0.028* | -0.024* | -0.028* |
| | points allowed a game | -0.20 | -0.68* | -0.69* | -0.71* |
| CB1 Man Coverage +15, West Coast | EPA a dropback allowed | -0.001 | -0.007 | -0.012* | -0.018* |
| | catch rate on him, points | -0.7 | -1.8* | -1.3 | -1.6* |
| SS Man Coverage +15 (man pressure, West Coast) | run stops a season | -0.82*, -0.11 | -0.41, -0.10 | -0.41, -0.54 | -0.24, +0.16 |
| | first tackles on runs a season | -1.49, -0.41 | -0.56, +0.49 | -0.48, -1.66* | -0.45, -0.08 |
| SS Tackling +15, West Coast | run stops a season | +2.07* | +1.98* | +2.19* | +2.11* |
| | first tackles on runs a season | +4.22* | +4.19* | +3.12* | +3.08* |
| Mike Football IQ -15, West Coast | busts a season | +2.24* | +2.71* | +7.96* | +8.36* |
| | points allowed a game | +0.24 | -0.25 | +0.10 | -0.25 |

Dropbacks with a bust in the typical team's coverage (West Coast, man pressure): at the NFL 3.5% and 3.5% > 3.3% and 3.4% of dropbacks, going for 0.075 and 0.139 > 0.193 and 0.128 EPA, the rest 0.090 and 0.069 > 0.088 and 0.067; at FBS 7.3% and 7.4% (both), 0.105 and 0.146 > 0.198 and 0.161, the rest 0.142 and 0.111 > 0.139 and 0.096.

The league (2,000 games a seed, main > after):

| | NFL seed 1 | NFL seed 2 | FBS seed 1 | FBS seed 2 | high school seed 1 | high school seed 2 | real NFL |
|---|---|---|---|---|---|---|---|
| points a team game | 28.32 > 27.63 | 28.97 > 28.71 | 26.48 > 26.29 | 26.48 > 26.51 | 21.07 > 20.85 | 20.02 > 20.13 | 22.51 |
| completion rate | 66.7 > 66.3 | 67.9 > 67.8 | 62.3 > 62.3 | 63.0 > 62.8 | 59.4 > 59.3 | 60.2 > 60.3 | 64.5 |
| yards an attempt | 7.70 > 7.61 | 7.99 > 8.00 | 7.78 > 7.81 | 8.07 > 8.03 | 7.12 > 7.19 | 7.27 > 7.36 | 7.05 |
| interceptions a team game | 0.838 > 0.836 | 0.741 > 0.738 | 0.626 > 0.669 | 0.583 > 0.590 | 0.317 > 0.321 | 0.343 > 0.325 | 0.756 |
| third down conversion | 45.8 > 45.8 | 47.3 > 46.9 | 43.2 > 43.2 | 43.4 > 43.4 | 41.0 > 41.3 | 41.3 > 41.5 | 39.8 |
| yards a carry | 4.41 > 4.43 | 4.28 > 4.24 | 4.78 > 4.79 | 4.68 > 4.66 | 5.63 > 5.62 | 5.25 > 5.26 | 4.50 |
| defensive holding a team game | 0.284 > 0.279 | 0.179 > 0.188 | 0.317 > 0.318 | 0.221 > 0.213 | 0.203 > 0.215 | 0.197 > 0.202 | 0.322 |
| EPA a target against Cover 1 | 0.266 > 0.236 | 0.322 > 0.285 | 0.285 > 0.275 | 0.310 > 0.286 | 0.196 > 0.228 | 0.271 > 0.273 | 0.179 |
| against Cover 0 | 0.583 > 0.537 | 0.609 > 0.564 | 0.606 > 0.550 | 0.628 > 0.544 | 0.490 > 0.529 | 0.572 > 0.559 | 0.171 |

Why:

- **Man Coverage pays at the top** (item 18). Past the old floor a better man defender reads his man's break early, so the first corner's Man Coverage +15 in man pressure takes 5.7 to 6.1 points off the catch rate on him (main 3.2 and 3.3) and 0.030 EPA a dropback off his defense at the NFL (main 0.021), about what -15 costs (0.029). The second corner's +15 gains as much. Behind West Coast's zones he plays man less, so it moves less. A typical corner sits above the floor and plays as he did.
- **A bust costs the dropback** (item 19). In a zone the busted man doubles a mate's area and leaves his own, so a dropback with a bust in West Coast's zones goes for 0.06 to 0.10 more EPA than the rest (on main 0.015 to 0.037 less). In man it costs about what it did (0.06 at both levels, main 0.07 and 0.04), since on main he left his man for a hook zone. The Mike's extra busts still don't show in points: 2.7 more a season at the NFL and 8.4 at FBS, at about 0.08 EPA each on a dropback, are under a point a season, inside the noise, and the passer finds the free man only when his reads come to him (still off).
- **A safety's coverage no longer costs his run fits** (item 20). He stands and fits where his job says, so his Man Coverage +15 moves his run stops and first tackles inside the noise at both levels, and his Tackling +15 pays what it did. His weight now places him only through the call (below: the box slope falls short of real).
- **The league holds.** Points fall up to 0.7 a team game at the NFL and move within 0.2 at FBS and high school, every total inside its test band. Man coverage allows less at the NFL and FBS, toward real (a target against Cover 1 0.01 to 0.04 less EPA, against Cover 0 0.05 to 0.08 less): the test rosters' better corners now read their men's breaks. Defensive holding stays at main's. At high school the coverages move up to 0.05 EPA a dropback either way, the other way on the other seed.

A safety's weight (`ab_check s_weight nfl`: every safety at 193 against 221 lb, the same ratings, up to 16,000 games an arm; each slope a 10 lb, main > after, real from nflverse in `scripts/ab_real.py` and the play by play in the project folder's `afm/ab-tests/proposal-4-safeties.md`): his weight now places him only through the call's shell lean, so men in the box on runs move +0.024 > +0.005 (real +0.08) and the yards a carry allowed -0.080 > -0.035 (real -0.11). His run tackles after 10+ yards come into the real band (-0.043 > -0.011 of his run tackles, real -0.024), his run tackles at 3 yards or less fall out of it (+0.0009 > +0.0003 a defense snap, real +0.0018), and his run tackles a snap now go the wrong way (+0.0002 > -0.0006, real +0.0029), as his tackles a snap did on main (-0.0005 > -0.0014, real +0.0032). The pass rows hold (catch rate allowed +0.0096 > +0.0090, in the real band). Real staffs walk a heavy safety down into the box; by Cain's rule that is now the call's to do, and the call has no job that puts its rotated-down safety inside the box count (still off).

The coaches' call table (`call_values 2000` on main and after): the lane moves its run cells 0.04 yards a cell (the mean of |change|; at most 0.29) and its straight pass cells 0.12 (at most 9.0), nearly all against man: Cover 0 -0.33 yards a snap on average, Cover 1 +0.48 with four rushers and +0.18 with the linebacker blitz, every other look within 0.04. The rows the table takes from a new run (the three play-action concepts, the fakes' changes and the run rows, `tools/call_values/compose.py`) move 0.21 yards a cell with the lane, less than main's own new run moves them off the committed table (1.49), so the committed table stands, as at lanes SF4 and SF6; the straight pass rows wait for minor bug 405.

Speed (`match_check`, 2,000 games at each level on seeds 1 and 2, main ffa2cfd and the lane in turn on an otherwise quiet machine, two rounds with the order swapped; the tool's own time): the NFL -0.4% (18.82 s to 18.75 a run; pairs -4.9% to +7.9%), FBS -1.9% (19.77 to 19.40; -4.9% to +1.0%), high school -0.9% (13.35 to 13.22; -5.2% to +3.0%), inside a pair's noise.

Tried and dropped (the lane's first build, the same seeds):

- **Running for the point ahead.** A man defender below 0 ran for where the route had his man -lag s ahead, so on a straight stem the best corners ran in front of their men: defensive holding fell by about a quarter (NFL 0.284 to 0.209 and 0.179 to 0.153, FBS 0.317 to 0.254 and 0.221 to 0.184, high school 0.203 to 0.161), and at FBS the first corner's Man Coverage +15 in man pressure saved 1.38 points a game, about three times what -15 cost.

Still off (`GAPS.md`, Engine): the passer finds a bust's free man only on his normal reads, so busts still barely move points; a heavy safety is no longer walked into the box, since the call has no job for it; the defense's holds against zone and on runs (from lane EG5) are not built, and the hand fight is still sized on seed 1.

## Blitzing and the quick 3-technique (scheme-fit lane SF8, items 21 and 22, 2026-11-20)

Design: `docs/design/scheme-fit.md`, items 21 and 22. Blitzing decides how fast a blitzer gets home and whether he beats the back's pickup: real off-ball linebackers get 0.140 pressures a blitz with a true spread of 0.051 (PFR 2018-24), so a good blitzer gets about a third more (item 21). A quick 3-technique's wins become pressure (item 22).

How it was measured: `fit_check` on the audit's seeds, main 78f55db and after: Blitzing +15 for the Mike, Will, Sam, strong safety and nickel in West Coast and man pressure, and Power Rush and Finesse Rush +15 for the first edge and the first tackle (West Coast), at the NFL (seed 161, 2,400 games an arm) and FBS (seed 151, 3,000), with the tool's new item 21 and 22 rows (a man's blitzes and his own pressures on them, from the play record's new `rushed` bits, and the share of a rusher's won reps that became his pressure); `match_check` at the NFL, FBS and high school on seeds 1 and 2 (2,000 games, `AFM_GAPS=1`); and a scratch read-out on the NFL test league (300 and 1,000 games, not committed): each move's wins and stalled reps and what became of them, the yards to the passer at the shed, and each blitz by slot and by who picked him up.

What was wrong on main:

- **Blitzing didn't decide the blitz.** It was read for who goes (`lb_blitzer`) and for the back's pickup (`picks_up`) only. Once sent, a blitzer rushed on his own Finesse Rush and Power Rush and left at the snap like every other. The Mike's Blitzing +15 sent him 2.3 and 8.8 more times a season at the NFL (West Coast, man pressure) and moved his pressures a blitz by -0.007 and -0.000; the Will's and the Sam's moved within 0.004 at both levels.
- **A won finesse rep came off in front of the blocker.** Every pass rep stood where the two men met, so a speed or finesse win was shed with the blocker still between the rusher and the passer, and came off at the bull rush's pace. On the NFL test league (300 games) a tackle's finesse win came off 5.6 yards from the passer (a bull rush win 4.5), and 16% of them became pressure (bull rush wins 48%); an edge's finesse and speed wins 14% and 18% (his bull rush wins 23%).
- **A stalled speed or finesse rep gave half the ground.** The pocket's drive was scaled whole by the move (1.4 on a bull rush, 0.7 otherwise), so a rep nobody was winning pushed the pocket at 0.18 yards a second on a speed or finesse move and 0.35 on a bull rush: a tackle's stalled bull rush became pressure on 3.2% of his reps, his stalled finesse move on 0.9%. Given Finesse Rush +15, a tackle went to the move more, won 3.8 more reps a hundred and got 4.0 fewer pressures a season at the NFL (2.3 and 2.7 at FBS).

What changed (`game/sim.rs` and `game/scrimmage.rs`; every size proposed, not confirmed):

- **A blitzer's moves read his Blitzing** (item 21; `blitzer`, `rush_moves`). A man rushing from off the line (a linebacker, corner, nickel or safety by the spot he lines up in) reads his Finesse Rush and Power Rush in every move as his level's typical man's at his position plus his Blitzing over the typical man's, against a lineman or a back alike, and his snap read in the get-off is the typical man's Anticipation plus the same. The typical man rushes as before, and the back's pickup still reads it (`picks_up`).
- **He times the snap** (item 21; `blitz_timing`, `BLITZ_JUMP` 0.1 s). A blitzer above the typical man is already moving at the snap, 0.1 s of his sprint for each 10 points over, toward his lane at the line; one below leaves as late, creeping until then (`hold_until`).
- **A speed or finesse win works round the blocker** (item 22; `around`, `HIP_TURN` 1.5, `HIP_EASE` 0.25 s). The rusher stands 0.9 yards from his blocker, square in front while the rep is even or lost, and works round his shoulder toward his own side as he wins it: at the blocker's hip two thirds of the way to the shed, past it toward the blocker's back at the shed. A bull rush stays in front, driving through the man, and a rusher fooled by a fake or doubled stays where they meet. A speed or finesse win comes off toward the passer at the speed win's pace (0.6 × 0.4 × his top speed, a bull rush 0.3).
- **The set gives ground whatever the move** (item 22; `POCKET_GIVE` 0.35 yd/s). An engaged pass block gives 0.35 yards a second toward the passer on any move, and what the rusher wins on top of it, up to 0.65 at the shed, is driven at 1.4 on a bull rush and 0.7 otherwise. The bull rush's drive is the same as before.
- **Who rushed** (`PlayRecord::rushed`). Each dropback keeps a bit for each defender who rushed, so a blitz counts whether or not he was blocked; it changes nothing in the play.

What the ratings are worth (`fit_check`, the change against the typical arm, main > after; * past the noise, |z| of 1.96 or more; two numbers are West Coast and man pressure):

| arm | measure | NFL main | NFL after | FBS main | FBS after |
|---|---|---|---|---|---|
| DT1 Finesse Rush +15 | pressures a season | -4.0* | +3.5* | -2.7* | +0.5 |
| | sacks a season | -0.65* | +2.18* | -0.43* | +0.84* |
| | rush wins a 100 reps | +3.8* | +3.2* | +2.3* | +2.1* |
| DT1 Power Rush +15 | pressures a season | +13.7* | +10.9* | +10.1* | +9.1* |
| | sacks a season | +2.68* | +1.77* | +2.15* | +1.81* |
| EDGE1 Finesse Rush +15 | pressures a season | +2.4* | +6.3* | +3.0* | +5.2* |
| | sacks a season | +0.68* | +1.70* | +0.72* | +1.31* |
| EDGE1 Power Rush +15 | pressures a season | +1.0* | +0.5 | +1.3* | +0.7* |
| | sacks a season | +0.45* | -0.10 | +0.26* | +0.23* |
| Mike Blitzing +15 | pressures a blitz | -0.007, -0.000 | +0.040*, +0.067* | +0.000, -0.001 | +0.044*, +0.042* |
| | pressures a season | -0.1, +0.9* | +2.4*, +9.2* | +0.1, +0.3* | +1.7*, +3.8* |
| | sacks a season | -0.18, +0.47* | +0.85*, +3.52* | +0.03, +0.09 | +0.43*, +1.08* |
| Will Blitzing +15 | pressures a blitz | +0.001, -0.002 | +0.023*, +0.027* | -0.003*, +0.001 | +0.024*, +0.034* |
| Sam Blitzing +15 | pressures a blitz | -0.003, +0.001 | +0.051*, +0.055* | +0.004, +0.002 | +0.036*, +0.031* |
| SS Blitzing +15 | pressures a blitz | -0.001, -0.002 | +0.006, +0.004 | +0.001, +0.001 | +0.005, +0.013* |
| Nickel Blitzing +15 | pressures a blitz | +0.002, +0.000 | +0.011*, +0.012* | +0.003*, +0.002* | +0.013*, +0.014* |

No arm moves points allowed past the noise at either level (the largest -0.31 and +0.25 a game); the first tackle's Power Rush +15 at FBS saved 0.57 points a game on main (past it) and 0.26 after.

The typical team (main > after; two numbers are West Coast and man pressure): its first tackle's pressures a season NFL 14.6 > 17.4 and FBS 12.7 > 13.5, its first edge's 7.8 > 10.2 and 6.6 > 8.1 (West Coast); a rush win that became the rusher's own pressure, tackles NFL 38.9 and 42.3% > 50.4 and 53.2%, FBS 44.0 and 47.6% > 50.8 and 53.0%, edges NFL 14.5 and 16.5% > 26.7 and 31.2%, FBS 20.3 and 23.2% > 30.6 and 34.1%; the Mike's pressures a blitz NFL 0.086 and 0.096 > 0.097 and 0.109, FBS 0.057 and 0.067 > 0.063 and 0.077, the linebackers' together NFL 0.052 and 0.059 > 0.058 and 0.068, FBS 0.048 and 0.054 > 0.054 and 0.066 (real 0.140); the team's pressure rate NFL 15.9 and 21.5% > 19.6 and 25.6% (real 22.2%), its sack rate 3.1 and 5.0% > 4.5 and 6.7% (real 6.8%).

The league (2,000 games a seed, main > after):

| | NFL seed 1 | NFL seed 2 | FBS seed 1 | FBS seed 2 | high school seed 1 | high school seed 2 | real NFL |
|---|---|---|---|---|---|---|---|
| points a team game | 27.63 > 26.95 | 28.71 > 28.05 | 26.29 > 25.62 | 26.51 > 25.81 | 20.85 > 20.61 | 20.13 > 19.82 | 22.51 |
| sacks a team game | 1.28 > 1.59 | 1.15 > 1.57 | 1.51 > 1.90 | 1.37 > 1.81 | 1.46 > 1.84 | 1.30 > 1.74 | 2.41 (FBS 2.00) |
| pressure rate | 17.6 > 20.3 | 17.3 > 20.2 | 21.3 > 24.2 | 21.2 > 24.2 | 26.1 > 30.1 | 24.4 > 28.4 | 22.2 |
| sacks a pressure | 0.194 > 0.211 | 0.176 > 0.208 | 0.195 > 0.221 | 0.182 > 0.214 | 0.233 > 0.256 | 0.207 > 0.240 | 0.283 |
| time to throw, s | 2.47 > 2.44 | 2.40 > 2.38 | 2.50 > 2.48 | 2.47 > 2.45 | 2.60 > 2.57 | 2.55 > 2.53 | 2.71 |
| completion rate | 66.3 > 66.5 | 67.8 > 67.9 | 62.3 > 62.1 | 62.8 > 63.0 | 59.3 > 59.2 | 60.3 > 60.8 | 64.5 |
| yards an attempt | 7.61 > 7.59 | 8.00 > 7.99 | 7.81 > 7.75 | 8.03 > 8.12 | 7.19 > 7.16 | 7.36 > 7.42 | 7.05 |
| interceptions a team game | 0.836 > 0.811 | 0.738 > 0.747 | 0.669 > 0.619 | 0.590 > 0.560 | 0.321 > 0.294 | 0.325 > 0.298 | 0.756 (FBS 0.804) |
| third down conversion | 45.8 > 45.2 | 46.9 > 46.1 | 43.2 > 42.1 | 43.4 > 42.5 | 41.3 > 40.2 | 41.5 > 40.9 | 39.8 |

Why:

- **Blitzing decides the blitz** (item 21). A Mike 15 points over the typical man (about two standard deviations of the test league's starting linebackers, 6.8 points) gets 41% and 61% more pressures a blitz at the NFL and 70% and 55% at FBS: 19% to 28% more a standard deviation of Blitzing alone at the NFL, inside real's 36% a standard deviation of everything that makes a blitzer (0.051 of 0.140). His start alone gives +0.021 a blitz and his moves alone +0.019 (the Mike in man pressure at the NFL), and the two together +0.067, more than apart. The Sam gains about as much a blitz at the NFL (+0.051 and +0.055) and the Will about half (+0.023 and +0.027), from lower bases; at FBS all three gain 0.024 to 0.044. A nickel's or a safety's +15 moves little (+0.004 to +0.014): three in four of their rushes are never blocked and still get home only 1.7% of the time on the NFL test league, which Blitzing doesn't reach (below). The typical man's moves and start are as before; the typical team's linebackers' own pressures a blitz rise 0.006 to 0.012 with item 22's changes, which reach every blocked rusher (a lineman picks up 77% of linebackers' blitzes on the NFL test league, 1,000 games).
- **A quick 3-technique's wins become pressure** (item 22). A tackle's finesse win now comes off at his blocker's hip or past it, 4.8 yards from the passer (main 5.6), and 52% of them become pressure (main 16%; a bull rush win 46%, main 48%); his stalled finesse move becomes pressure on 1.4% of reps (main 0.9%; his stalled bull rush 3.0%, main 3.2%; NFL test league, 300 games). So the first tackle's Finesse Rush +15 adds 3.5 pressures and 2.2 sacks a season at the NFL (main -4.0 and -0.65) and 0.84 sacks at FBS (main -0.43). His Power Rush +15 still adds more pressures (10.9 and 9.1), since the bull rush wins more reps (4.2 more a hundred against 3.2 at the NFL), and he now gets his pressures from the wins themselves, not from a better mix of moves (his share of wins that became pressure moves +0.3 and +1.5 points, main +10.9 and +5.6). An edge's speed and finesse wins become pressure 34% of the time (main 18% and 14%; his bull rush wins 23%), so his Finesse Rush +15 adds 6.3 and 5.2 pressures a season (main 2.4 and 3.0) and his Power Rush +15 less than on main (0.5 and 0.7, main 1.0 and 1.3): going to the bull rush no longer gets him wins that convert better.
- **The league's rush comes closer to real.** Pressure rises 2.7 to 4.0 points of dropbacks at every level and sacks 0.31 to 0.44 a team game: NFL sacks 1.59 and 1.57 (main 1.28 and 1.15, real 2.41), FBS 1.90 and 1.81 (real 2.00), NFL pressure 20.3% and 20.2% (real 22.2%), sacks a pressure 0.21 (real 0.283). Points fall 0.66 to 0.70 a team game at the NFL and FBS and 0.24 and 0.30 at high school, every total inside its test band. The passer throws 0.02 s sooner, third down falls 0.6 to 1.1 points toward real at the NFL and FBS, and picks fall 0.03 to 0.05 a team game at FBS and high school (FBS seed 1 2.02% of attempts to 1.94%; further below real at FBS, not looked into).

The coaches' call table (`call_values 2000` on main and after): the lane leaves its run cells as they were (the pass rush doesn't reach a run) and moves its straight pass cells 0.48 yards a cell (the mean of |change|; at most 11.7): against four rushers 0.02 to 0.31 yards a snap less, against Cover 0's double A gap blitz 0.41 less, against the linebacker blitz from 0.16 less to 0.24 more by coverage. The rows the table takes from a new run (the three play-action concepts, the fakes' changes and the run rows, `tools/call_values/compose.py`) move 0.35 yards a cell with the lane, less than main's own new run moves them off the committed table (0.84), so the committed table stands, as at lanes SF4, SF6 and SF7; the straight pass rows wait for minor bug 405.

Speed (`match_check`, 2,000 games at each level on seed 1, main 78f55db and the lane in turn on an otherwise quiet machine, two rounds with the order swapped; the tool's own time): the NFL -4.8% (17.70 s to 16.85 a run; pairs -9.6% and +0.6%, main's first run slow), FBS +0.3% (18.15 to 18.20; +1.1% and -0.6%), high school +0.8% (11.90 to 12.00; +0.8% both), inside a pair's noise.

Tried and dropped (the same seeds):

- **The rusher at the hip at the shed** (`HIP_TURN` 1): 39% of a tackle's finesse wins became pressure (bull rush wins 47%), and his Finesse Rush +15 still cost 1.4 pressures a season at the NFL.
- **Further round** (`HIP_TURN` 2 and 3): 44% of his finesse wins became pressure, but 43% to 47% of those were sacks (of the whole league's pressures about 21% are sacks, real 28%).
- **A faster exit off a finesse win** (1.0 and 1.5 × the speed win's pace) and a quicker turn (`HIP_EASE` 0.1 s): no more of his wins became pressure, as the passer let it go sooner.
- **The set's give at main's 0.25 on every move**: the bull rush gave 0.1 yards a second less ground, and the typical tackle's pressures fell to 12.5 and 12.8 a season at the NFL (main 14.6).

Still off (`GAPS.md`, Engine): linebackers' blitzes get home half as often as real (the typical team's 0.058 and 0.068 pressures a blitz, real 0.140), with a lineman on 77% of them; the Will's blitz gets home a quarter as often as the Mike's; a defensive back's rush almost never gets home when nobody blocks him; edges get fewer pressures than tackles; at FBS the first tackle's Finesse Rush +15 adds sacks but no pressures past the noise; NFL sacks are still a third short of real, now mostly the passer's quick clock (67% and 72% of throws inside 2.5 s, real 50%), lane EG7's. The Data section counts a blitz only when he was blocked or got pressure (`GAPS.md`, Logic).

## Short yardage and long runs (scheme-fit lane SF9, items 8 and 9, 2026-11-21)

Design: `docs/design/scheme-fit.md`, items 8 and 9. On third and fourth down with 2 or less to go, real NFL backs convert 66.9% of their designed runs and quarterbacks 82.7% (nflverse 2024; quarterbacks 78.9% over 2022-24, the spec's 79%) (item 8). Backs' designed runs of 20+ yards are 2.2 to 2.4% of real carries and runs of 10+ 10.6%, and a burner's speed should then be worth what it really is (item 9). Item 8 is built. Item 9 is traced and stopped: its fix is the open-field tackle, which reaches the catch and the returns too, past the lane's spec (below).

How it was measured: `match_check` with `AFM_RUNDIAG=1` and the lane's new SF9 read-out (backs' designed runs: the share gaining k yards or more, wide of the tackles and inside, who ends them by length, missed tackles, short yardage by distance, box and field, and quarterbacks' short yardage, each beside nflverse 2024, with FTN's 2024 charting for sneaks), 4,000 games at the NFL and FBS on seed 11, main 3a04dae and after, and every variant below on the same NFL games; `match_check` at the NFL, FBS and high school on seeds 1 and 2 (2,000 games, `AFM_GAPS=1`); `fit_check`'s typical arms in all 12 contexts on the audit's seeds, main and after.

What was wrong on main:

- **A linebacker filled past the sticks** (item 8). He fills to 2.6 yards past the line on a run and holds at 2.1 once he is nearly on the runner (minor bug 164, bug 305), whatever the distance. On third and 1 he met the back past the line to gain, so his stop there was a first down. NFL backs converted 74.7% of their designed runs on third and fourth down and 2 or less (real 66.9%): to go 1 78.1% (69.1%), to go 2 65.5% (60.8%), against a 6-, 7- and 8-man box 75.9%, 75.8% and 76.5% (68.2%, 69.4% and 62.0%). Linebackers ended 20.0% of those runs, 2.95 yards past the line (real 24.5% at 2.17), and on third and 1 9.6% of the backs' runs gained nothing (real 21.0%). FBS backs converted 76.5%.
- **Long runs half of real** (item 9). Backs' designed runs of 10+ are 8.85% of carries (real 10.4% in 2024) and of 20+ 1.02% (2.26%), while runs of 5 to 9 yards are 32.3% (24.5%): the middle is fat and the tail thin. Of the runs that reach 10, 11.5% go on to 20 (real 21.8%). Safeties end 49.2% of the 5-9 yard runs (real 23.3%) and 73.6% of the 10-19 (46.8%). Inside the tackles backs gain 3.92 a carry (real 4.26), 6.8% go 10+ (9.6%) and 0.81% 20+ (1.93%); wide of them 5.25 (4.72), 13.3% (13.5%) and 1.47% (3.69%).

What changed (`game/scrimmage.rs`, `lb_fill`; the size proposed, not confirmed):

- **A linebacker plays the sticks** (item 8; `lb_fill`, `LB_FILL` 2.6, `LB_HOLD` 2.1, `LB_STICKS` 0.25). On third and fourth down his fill, his hold and the depth of his flow over the top stop a quarter yard short of the line to gain when that is nearer: on third and 1 he fills to 0.75 yards past the line, on third and 2 to 1.75. Every other down is as before.

Short yardage, 4,000 games on seed 11 (main > after; real nflverse 2024):

| | NFL | FBS | real NFL |
|---|---|---|---|
| backs, 3rd and 4th and 2 or less | 74.7% > 66.9% | 76.5% > 70.9% | 66.9% |
| to go 1 | 78.1% > 69.9% | 81.9% > 75.0% | 69.1% |
| to go 2 | 65.5% > 58.7% | 65.0% > 62.2% | 60.8% |
| 6-, 7- and 8-man box | 75.9, 75.8, 76.5% > 69.7, 68.0, 68.2% | | 68.2, 69.4, 62.0% |
| linebackers' stops, share and yards | 20.0% at 2.95 > 29.3% at 1.85 | | 24.5% at 2.17 |
| to go 1, no gain | 9.6% > 15.6% | | 21.0% |
| quarterbacks | 85.1% > 84.7% | 84.1% > 83.0% | 82.7% (2022-24 78.9%) |
| sneaks | 88.8% > 88.2% | 89.1% > 88.7% | 85.2% (FTN) |
| fourth down conversion, all plays | 53.3% > 52.0% | 51.1% > 50.9% | 53.7% |
| third down conversion | 44.6% > 43.7% | 41.7% > 41.0% | 39.8% (FBS 40.3%) |

The league (2,000 games a seed, main > after):

| | NFL seed 1 | NFL seed 2 | FBS seed 1 | FBS seed 2 | high school seed 1 | high school seed 2 | real NFL |
|---|---|---|---|---|---|---|---|
| points a team game | 26.95 > 26.77 | 28.05 > 28.04 | 25.62 > 25.51 | 25.81 > 25.72 | 20.61 > 20.42 | 19.82 > 19.53 | 22.51 (FBS 26.97) |
| yards a carry | 4.40 > 4.36 | 4.25 > 4.24 | 4.80 > 4.78 | 4.66 > 4.63 | 5.60 > 5.60 | 5.23 > 5.25 | 4.50 (FBS 5.12) |
| third down conversion | 45.2 > 44.4 | 46.1 > 45.4 | 42.1 > 41.6 | 42.5 > 41.5 | 40.2 > 39.7 | 40.9 > 40.3 | 39.8 |
| fourth down conversion | 54.7 > 52.4 | 55.0 > 53.5 | 51.9 > 50.6 | 52.2 > 50.9 | 49.3 > 47.7 | 50.8 > 49.5 | 53.7 |
| fourth down tries a team game | 1.52 > 1.54 | 1.49 > 1.52 | 1.88 > 1.92 | 1.87 > 1.93 | 1.45 > 1.44 | 1.46 > 1.52 | 1.46 |

The typical team (`fit_check`'s typical arm in each of the 12 contexts, team 0 of the league; FBS seeds 151 and 171 at 3,000 games an arm, NFL 161 at 2,400; main > after, the range over the contexts): on third and fourth down and 2 or less its backs' and other runners' runs convert NFL 77.2-79.4% > 66.5-69.2% (real 67% for backs), FBS 81.3-83.6% and 81.3-82.9% > 74.7-76.1% and 72.5-74.8%; against its defense NFL 73.1-78.8% > 64.3-70.2%, FBS 75.4-80.7% and 76.7-81.4% > 69.8-72.2% and 69.6-73.0%. Its quarterbacks' runs stay (NFL 83.8-90.0% > 83.6-90.8%, FBS 81.5-89.7% and 85.3-92.4% > 81.3-89.6% and 82.8-92.4%; real 79%), and so do its backs' long runs (NFL 20+ 0.97-1.17% > 0.95-1.17%, 10+ 6.8-8.2% > 6.7-8.3%; FBS 20+ 1.29-1.65% > 1.25-1.68%).

Why:

- **Backs convert at the real rate** (item 8). A linebacker who meets the back short of the sticks makes a stop, so linebackers end 29.3% of short-yardage runs at 1.85 yards (real 24.5% at 2.17) and the line's share falls toward real (tackles 20.7% to 15.3%, real 16.8%). Each distance and box comes to its real rate, the 8-man box still 6 points high. The no-gain share on third and 1 rises from 9.6% to 15.6% (real 21.0%), and runs of 6+ on it fall from 28.0% to 24.3% (16.7%): the back still breaks past the sticks more often than real, item 9's open field.
- **The quarterback's sneak barely moves.** His sneak is over the line before a linebacker's fill reaches it: 88.2% (real 85.2%). Linebackers lined up at 3 yards on these downs (4.5 to 4.8 today) stopped sneaks at the real rate (84.9%), but backs then converted 75.8% (main 74.7%), so it was dropped; why a linebacker nearer the line stops fewer backs was not looked into (`GAPS.md`). Quarterbacks' other short-yardage runs are 6% of theirs (real 27%) and convert 33% (real 74-76%): designed quarterback runs, lane SF10.
- **The league moves a little toward real.** Third down falls 0.5 to 1.0 points at every level (NFL 44.4% and 45.4%, real 39.8%), fourth down 1.3 to 2.3 points (NFL 52.4% and 53.5%, real 53.7%), and points 0.0 to 0.3 a team game; every total stays inside its test band. Long runs don't move (backs' 10+ 8.81%, 20+ 1.03% on seed 11).

Item 9, traced and stopped. Each variant on the same 4,000 NFL games (seed 11; main 10+ 8.85%, 20+ 1.02%, 20+ of 10+ 11.5%, safeties end 16.5% of the 20+ runs, yards after the catch 5.24, points 26.27; real 10.4%, 2.26%, 21.8%, 39.8%, 5.20 and 22.51):

- **Why.** Two things hold the back inside 10 yards. Every deep back fits the run at 9.75 yards and attacks once the runner is 4.75 past the line (`defend`, the deep back's fit), whatever the coverage or where the run goes, so both safeties come down to the same depth on every run. And once a tackler is squared up 0.4 to 5 yards in front of the carrier, he shuffles to the carrier's line at 4 times the gap a second, up to his top speed (`game/sim.rs`, `pursue_cap`, the breakdown), so the carrier can't get round him: deep safeties' head-on tackles miss 6.7% of the time (33,029 on 191,770 runs), and safeties miss 4% of their tries on 5-9 yard runs.
- **The fit moved, the wall moved with it.** The deep fit at 12 yards: 10+ 15.4%, 20+ 1.07% (the runs reach 10 and stop there). Attacking at 7: 11.9% and 1.29%. A one-high safety as the last man and two-high safeties split by the run's side: 12.3% and 1.07%. The rotated-down safety his own fit (2.5 and 6.5 yards): 8.72% and 1.30%, at 4.26 yards a carry (main 4.44, real 4.50). The box count out to 9.5 yards: 9.04% and 1.33% at 4.28. The deep back coming down at full speed before the runner is through (0.6 of it today): 10.2% and 2.66%, but 42% of the 20+ runs went 40+ (real 25%) at 4.88 a carry and 130 rushing yards a team game (real 118).
- **A later read did nothing.** The tackler reading the carrier's cut 0.15 to 0.35 s late, or his spot 0.25 s late: 20+ 1.01% to 1.05%; the late read in the chase too, 1.30%. Head-on tackles missed more (their angle term -0.3 to +0.3): 1.25%.
- **Only the open field opens it.** A squared-up tackler shuffling at 55% of his top speed: 10+ 9.99%, 20+ 1.88%, 20+ of 10+ 18.8%. At 1.5 times the gap a second: 10.1%, 2.17% and 21.6%. But the same tackle meets receivers after the catch: yards after the catch rose to 5.75 and 5.73 a catch (real 5.20), pass yards 237 to 248 and 247 a team game, rushing yards to 127 and 131 (real 118), points to 28.62 and 29.04, and safeties ended 10% of the 20+ runs (real 40%).
- **What it needs** (Cain chose this as its own lane on 2026-11-21, lane SF9b, straight after SF10; its sizes proposed, not confirmed; built the same day, "The open-field tackle" below). The open-field tackle as one piece for runs, the catch and returns (the return breakaways are short too, `GAPS.md`): a squared-up tackler gives ground to the carrier's cut by his own and the carrier's ratings rather than a near-perfect shuffle, sized so the backs' tail, yards after the catch and returns land on real together, with the deep safeties' fit read by the coverage and the wide run's stuffs and sideline (below). The burner arms (`fit_check`'s `RB1-pg-fast` and the rest) were not run, since the tail did not move.

Also seen on the way (`GAPS.md`): wide runs are 32% of the backs' runs (real end-gap runs 20%) and are stuffed for no gain or a loss 11.6% of the time (real 23.2%), linebackers making 2.4% of those stuffs (27.7%); wide runs of 1-4 yards end out of bounds 0.1% of the time (real 12.8%) and of 5-9 3.7% (19.4%).

The coaches' call table (`call_values.csv`) is kept: its snaps are first and 10 (`flow::bare_game`), where nothing changed. The analysts' tables wait for SF10 with the other refits (`docs/design/scheme-fit.md`, every game lane).

Speed (`match_check`, 2,000 games on seeds 1 and 2 at each level, main and the lane in turn on an otherwise quiet machine, the order swapped between seeds; the tool's own time): the NFL +3.9% (pairs +4.1% and +3.7%), FBS -5.3% (-5.6% and -5.0%), high school -2.9% (+1.8% and -7.4%), inside a pair's noise. The change is one comparison a tick for each linebacker.

Still off (`GAPS.md`, Engine): long runs half of real (item 9, above); quarterbacks' sneaks 3 points high and the linebacker's alignment trade-off; short-yardage runs of 6+ still 7.6 points high on third and 1; wide runs short of real stuffs and the sideline.

## The roster shapes the call (scheme-fit lane SF10, items 10, 11 and 23, 2026-11-21)

Design: `docs/design/scheme-fit.md`, items 10, 11 and 23, and engine gaps item 10. Cain: "the roster should always determine playcall": every call, run or pass, leans to what the caller's own players win, as the engine pays them, a sharper caller harder, with the situation, the scheme, the looks and today's results still counting (item 10). AI staffs put on a short-yardage back and a third-down back where the roster has one (item 11). Personnel, the man and pressure lean, the blitz lean and the quarterback's runs read the roster, and a good free safety opens up the playbook (item 23, Cain's addition). Designed quarterback runs (engine gaps item 10: 0.54 a team game on the test rosters, real 1.69) are measured on the world's rosters. The lane also rebuilds the coaches' call table on typical teams (minor bug 405) and refits the analysts' four models now that the physics lanes are in.

How it was measured: `fit_check` on the audit's NFL seed 161 at 2,400 games an arm, main bbaa51c and after, on 26 of the audit's arms (one man's rating +15, the free safety's four coverage reads +15 together, a back's weight at 235 or 195 pounds, three corners built for man), with six new counts in the tool (the backs' gap runs and wide runs, the quarterback's designed runs, who is on the field on third and fourth down with 2 or less and with 4 or more to go, and the defense's single-high calls); `match_check` at the NFL, FBS and high school (2,000 games, seed 1, `AFM_GAPS=1`) and on world seed 1's 32 NFL clubs (`AFM_AB_WORLD`), main and after; and a variant of the branch with the roster's lean and the situational backs turned off, on the same games, to split the rebuilt call table's share from the lean's.

What was wrong on main:

- **The call read nothing of the roster but a burner and the passer's legs** (items 10 and 23). The run call read the scheme, the short-yardage cuts, the promises, the what-beats-what table and today's results; the pass call a burner (deep concepts × 1 + 0.3 × burner) and how much the passer runs; personnel the scheme's table; the coverage the head coach's background; the blitz the linebackers' and corners' weight. On `fit_check` a man's +15 moved the call's shape only by its noise: three corners built for man (Man Coverage +12, Zone Coverage -12) +0.06 points of man coverage, the passer's Arm Strength +15 +0.25 air yards a throw (his own reads), the second and third tight ends' Run Blocking +15 -0.15 and +0.03 points of 12 and 13 personnel.
- **No situational backs** (item 11). AI backs rotated on fatigue only (`flow.rs`, the back's 0.60 threshold); who the second back was didn't decide when he came on.
- **The call table's teams weren't typical** (minor bug 405). `call_values` played its cells with one fixed team whose first tight end sat 17.4 points under the typical starter's Catching, and the committed table's faked rows were each straight row plus a fake's change from another run (`tools/call_values/compose.py`).

What changed (`game/roster_fit.rs`, new; `game/calling.rs`; `game/flow.rs`; `game/synthetic.rs`; `data/roster_values.csv`, new; `data/call_values.csv`; `afm-tools`'s `call_values`; every size proposed, not confirmed):

- **What each call gains from his own men, measured** (item 10's "how fit is measured"). `call_values roster 100000` plays the call table's two typical NFL teams with every watched man at the typical man's numbers, then on each of 100,000 draws moves every watched rating of one side's men by U(-18, +18) points and each man's weight by U(-8%, +8%), and plays one snap of every cell: each pass concept straight and off the fake, each run, each personnel group's runs and throws, the back on third and short and on third and long (how often the snap converts), each look of the call table and base or nickel against three receivers. A weight is the least-squares slope of the cell's value (the call table's yards: capped at 40, less 45 for a turnover) on a rating's deviation at a slot; each kind of cell shares a pair's weight unless they differ past their noise (DerSimonian-Laird pooling, a chi-square test at 5%), and a weight is kept only past 2.5 times its noise, so a rating the engine doesn't pay reads nothing. 2,840 weights in 107 cells on 179 slot-and-rating pairs (`data/roster_values.csv`, about 13 minutes on 4 cores).
- **The typical man at each place on the depth chart** (`synthetic::typical_at`, `typical_fb`). A team's men are read about the level's typical man at their place (its first, second and third tight end about the typical first, second and third, the fullback about the typical fullback), the k-th man on each of the 64 synthetic teams' charts, so a team like its level's typical one leans nowhere (`tests::roster_fit`).
- **The caller leans by his creativity** (topic 6.2, "calls that fit his players"; `trust`, 0 at 1, 0.12 at 10, 0.55 at 20, the look table's curve). On every snap the men who can go and the back his staff puts on for the down (`Game::off_fit`, `def_fit`): a concept × exp(trust × what it gains with his men), a play-action concept by its faked play; a run × exp(trust × 1.5 × its gain), the look table's run size; a personnel group × exp(trust × its gain on the call's runs or throws); a coverage × exp(-trust × what it gives up with his men); the nickel and the linebacker blitz on their odds by what they save (`lean_odds`; Cover 0's blitz as before). What every option shares cancels, and the scheme, the situation, the looks and today's results weigh as before.
- **Situational backs** (item 11; `Game::situational_back`, `roster_fit::back_value`, `SIT_EDGE` 0.01). On third or fourth and 2 or less (or inside the 3) and on a passing down, an AI staff puts on the back with whom the snap converts most often by the table, among the backs fresh enough to go in, when he converts more often than the depth chart's back by a point of the chance; your own team plays your depth chart and packages.
- **The call table on typical teams** (minor bug 405; `call_values 2000`). Both teams of the tool are the NFL's typical team (`fit::base_team`, the West Coast and a 4-3 and 3-4 defense), every man at the typical man's numbers, and every row, straight, faked and run, comes from the same run, so `compose.py` is gone. Against main's table the cells move 2.66 yards on average (straight throws 2.46, faked 3.21, runs 1.10; 2.14 within a row once each row's own level is taken out). Spacing against Cover 1 robber, the cell bug 405 named, gives 0.59 yards (main's table 1.11).
- **The analysts' models refitted** on the branch (none feeds play): expected completions (`xcomp_fit 600`; the old weights read CPOE +1.5 to +3.0 on today's engine), expected rush and YAC (`xyards_fit 600`), expected pass rate (`xpass_fit 600 7`) and the fourth-down, punt, kick and win expectancy tables (`decide_fit 1500 1`).

The audit's arms (`fit_check`, NFL seed 161, 2,400 games an arm on the same games as its context's typical arm; each change on team 0 against its typical arm, main > after, the after side ± its standard error over the 40 blocks):

| arm | what the call should do | main | after |
|---|---|---|---|
| three corners built for man (Man Coverage +12, Zone Coverage -12; West Coast) | lean to man | man +0.06 points of calls, single-high +0.33, points against +0.71 a game | man +7.31 ±0.16, single-high +3.50 ±0.24, points against -0.13 ±0.30 |
| the nickel corner's Man Coverage +15 | lean to man | man -0.05 | +2.60 ±0.16 |
| the first corner's Man Coverage +15 | lean to man | man +0.23, points against -0.06 | +1.70 ±0.15, -0.75 ±0.23 |
| the passer's Arm Strength +15 (West Coast) | throw deeper | air yards a throw +0.25, the X's targets 20+ yards out +0.39 points, points +0.34 | +0.46 ±0.05, +0.92 ±0.40, +1.27 ±0.26 |
| the passer's Deep Accuracy +15 | throw deeper | air -0.02, points +0.60 | +0.10 ±0.05, +1.32 ±0.28 |
| the X receiver's Speed +15 | throw deep to him | his targets 20+ out +1.10 points, points -0.17 | +1.38 ±0.39, +0.39 ±0.27 |
| the left tackle's Pass Blocking +15 | throw more, deeper | points -0.14 | +0.41 ±0.26 |
| the second tight end's Run Blocking +15 (a 12 personnel scheme) | more 12 | 12 personnel -0.15 points of snaps | +0.22 ±0.23 |
| the third tight end's Run Blocking +15 (a 13 personnel scheme) | more 13 | 13 personnel +0.03 | +0.29 ±0.21 |
| the first back at 195 pounds (power gap) | the second back on in short yardage | the second back on -0.80 points of short-yardage snaps | +36.0 ±0.9 |
| the first back at 235 pounds | the first back stays on | -0.53 | +1.07 ±0.83 |
| the second back's Pass Blocking +15 | a third-down back | the second back on third and long +1.35 points | +0.56 ±0.59 |
| the first back's Leg Drive +15 (power gap) | more gap runs | gap runs -0.12 points of his carries, points +0.12 | +0.21 ±0.12, +0.53 ±0.21 |
| the passer's Speed +15 (West Coast; spread option) | more designed runs | +0.01 and +0.64 designed runs a team game | +0.01 ±0.02 and +0.62 ±0.05 |
| the free safety's Zone Coverage, Play Recognition, Football IQ and Anticipation +15 (West Coast; man pressure) | open the playbook (Cain's addition) | single-high +0.24 and +0.17, blitz -0.26 and +0.10, points against -1.34 and -0.56 | -0.34 ±0.27 and +0.25 ±0.29, -0.19 and -0.31, -1.21 ±0.21 and -1.02 ±0.23 |

The league (`match_check`, 2,000 games on seed 1; main > the variant with the lean off > after, high school main > after; real NFL 2024, nflverse or FTN):

| | NFL test rosters | FBS test rosters | high school | world seed 1's NFL clubs | real NFL |
|---|---|---|---|---|---|
| points a team game | 26.77 > 26.77 > 26.75 | 25.51 > 25.47 > 25.68 | 20.42 > 20.56 | 28.58 > 28.03 > 28.33 | 22.51 (FBS 26.97) |
| yards a carry | 4.36 > 4.41 > 4.37 | 4.78 > 4.76 > 4.76 | 5.60 > 5.59 | 4.52 > 4.48 > 4.51 | 4.50 (FBS 5.12) |
| third down conversion | 44.4 > 44.0 > 44.3% | 41.6 > 41.8 > 41.7% | 39.7 > 40.3% | 45.3 > 45.3 > 45.3% | 39.8% |
| play action, of dropbacks without screens | 21.1 > 23.2 > 23.2% | 22.1 > 24.1 > 24.0% | 21.8 > 22.2% | 18.3 > 21.1 > 20.9% | 21.2% |
| screens off a fake, of screens | 34.6 > 21.0 > 21.4% | 32.2 > 20.7 > 21.4% | 25.4 > 21.4% | 35.8 > 19.3 > 19.9% | 27.3% |
| man coverage, of the home side's calls | 32.6 > 35.4 > 35.3% | 32.4 > 34.6 > 34.7% | 33.2 > 34.2% | 28.0 > 31.4 > 31.4% | 31% of dropbacks (NGS 2025; teams 18-45%) |
| throws 20+ yards downfield | 11.0 > 11.4 > 11.1% | 9.9 > 10.2 > 10.0% | | 11.0 > 11.3 > 12.3% | 11.5% |
| designed quarterback runs a team game | 0.74 > 0.72 > 0.75 | 1.20 > 1.23 > 1.21 | 1.46 > 1.43 | 1.70 > 1.63 > 1.66 | 1.69 |

Why:

- **The call now uses what its men win** (items 10 and 23). With three corners built for man the call is in man 7.3 points more often and single-high 3.5 points more (main +0.06 and +0.33), and what those corners cost on main, where the call stayed in zone (+0.71 points against a game), is gone (-0.13). One man's +15 pays more where the call can use it: a man corner's points against -0.06 > -0.75 a game, the passer's Arm Strength +0.34 > +1.27 points, his Deep Accuracy +0.60 > +1.32, a fast X receiver -0.17 > +0.39, the left tackle's Pass Blocking -0.14 > +0.41, a power back's Leg Drive +0.12 > +0.53. The big arm throws deeper (+0.46 air yards a throw, main +0.25), and the fast X draws more of his targets 20+ yards out (+1.38 points, main +1.10).
- **What the engine pays little leans little.** The table pays the second tight end's Run Blocking 0.016 yards a point on 12 personnel's runs and the third's 0.013 on 13's, about 0.2 yards a run at +15, and their hands nothing past the noise on those groups' throws, so two blocking tight ends lean 12 and 13 by about a standard error (+0.22 and +0.29 points of snaps). A passer's Speed pays 0.017 yards a point on the speed option, 0.008 on QB power and nothing on the zone read, so a fast passer runs no more in the West Coast (+0.01 designed runs a team game), and in the spread option its reads already give him his runs (+0.62, main +0.64). A back's Leg Drive and weight pay on no run in particular, and his Speed 0.010 on the toss and 0.008 on duo, so a back's build barely changes which run is called (the fast back's arms move the gap and wide shares by their noise).
- **A smart free safety leans to quarters** (Cain's addition). The table pays a free safety's Zone Coverage, Play Recognition, Football IQ and Anticipation most in quarters with four rushing (0.28 yards a snap less for +15 on all four), then in Cover 1 and Cover 3 match with a linebacker's blitz (0.17 and 0.15) and the robber (0.14), and nothing in the two-high looks; with the lean only as strong as that, his single-high share doesn't move past its noise (-0.34 ±0.27 in the West Coast, +0.25 ±0.29 in man pressure). "A good FS should open up the playbook" needs the engine to pay him more as the single-high man (`GAPS.md`).
- **Situational backs where the table pays them** (item 11). On third and short the table pays a back's weight 0.04 points of the conversion chance a pound and his Balance 0.03 a point, so a 195-pound first back gives way to the typical second back on 36 points more of the short-yardage snaps, and a 235-pound one stays on. On third and long it pays a back's hands and protection nothing past the noise (only his Leg Drive, 0.004 points a point), so no third-down back comes on (the second back's Pass Blocking +15: +0.56 ±0.59 points of those snaps).
- **The league's call mix moves with the rebuilt call table, not the lean.** The variant with the lean and the situational backs off moves as much as after (main > the variant here): on the test rosters play action rises 2.1 points of the dropbacks without screens (21.1% > 23.2%, real 21.2%), screens off a fake fall 13.6 points of the screens (34.6% > 21.0%, real 27.3%) and man coverage rises 2.8 points (32.6% > 35.4%, real 31%); on world seed 1's clubs the same three go 18.3% > 21.1%, 35.8% > 19.3% and 28.0% > 31.4%. The rebuilt table's faked rows come from the same run on the same typical team as its straight rows, where main's were each straight row plus another run's change from the fake, so a fake off a run pays what the engine gives it and a screen off a fake less. On the world's clubs play action and man come to real and screens off a fake go from 8.5 points over real to 7.4 under (`GAPS.md`); on the test rosters play action and man go 2 and 4 points past real, which no test band reads. Points, yards a carry and third down don't move past a seed's noise at any level.
- **The lean adds about 0.3 points on the world's clubs** (28.03 > 28.33 a team game, the test rosters about nothing) and deep throws 11.3% > 12.3% of throws (real 11.5%): the world's starting passers carry 9.1 points more Deep Accuracy than the test rosters', read about the typical man, so every club leans to the deep crossers, smash and the faked fades (`GAPS.md`, the typical starter isn't the world's).
- **Designed quarterback runs are at real on the world's clubs** (engine gaps item 10): 1.66 a team game on world seed 1's 32 clubs (main 1.70, real 1.69), whose passers' legs run like real ones; the test rosters' typical passers run 0.75 (main 0.74), so the gap the audit saw was the test rosters', and nothing more is built for it. On third and fourth down and 2 or less (`AFM_RUNDIAG=1`, 4,000 NFL games on seed 11) quarterbacks convert 85.2% of their runs (main 84.7%, real 82.7%) and backs 67.7% (66.9%, real 66.9%); 6% of the quarterbacks' runs there are not sneaks (real 27%) and those convert 21% (main 33%, on 109 and 125 runs; real 76%), the engine's play of those keepers, not the call (`GAPS.md`).

Speed: `match_check`, 2,000 games on seeds 1 and 2 at each level, main and the lane in turn on an otherwise quiet machine, the order swapped between seeds; the tool's own time. Working the fit out on every snap cost the NFL +4.9% (pairs +8.8% and +1.1%), FBS +5.5% (+8.4% and +2.7%) and high school +2.5% (+5.0% and 0.0%), so each side keeps the fit and the men it read (`Side::off_memo`, `def_memo`) and works it out again only when its men change, since a game's players never change. With it every game plays the same (`match_check`'s output is the same line for line at every level on both seeds), and the lane against main is the NFL -2.7% (+1.1% and -6.5%), FBS +1.2% (-2.5% and +4.9%) and high school +2.4% (+2.5% and +2.2%), inside a pair's noise.

Still off (`GAPS.md`): no third-down back, since third and long pays no back's hands or protection; a back's build barely changes which run is called (Leg Drive and weight pay on no run in particular, the burner's edge waits for item 9's open-field tackle); a smart free safety leans to quarters, not to single-high man and pressure (Cain's addition); the second and third tight ends' hands pay nothing on 12 and 13 throws; fast receivers on deep routes (the table pays a receiver's Speed on the shallow cross, smash and the bubble, not four verticals); screens off a fake 6 points under real on the rebuilt call table; quarterbacks' other short-yardage runs (6% of theirs, converting 21%); the world's passers lean deep (the typical starter isn't the world's); one table, measured at the NFL, serves every level; the run or pass split doesn't read the roster (proposed, not confirmed: item 10's bullets name which run and which throw, not how often each).

## The open-field tackle (scheme-fit lane SF9b, item 9, 2026-11-21)

Design: `docs/design/scheme-fit.md`, item 9, and lane SF9's trace ("Short yardage and long runs", above). Backs' designed runs of 20+ yards are 2.26% of real NFL carries, of 10+ 10.4%, and 21.8% of the runs that reach 10 go on to 20 (nflverse 2024); then a burner's speed should be worth what it really is (item 9's second half). Cain chose the open-field tackle as its own lane on 2026-11-21 ("Open-field lane before SF10", then "okay" to SF10 merging first): one piece for runs, the catch and returns, with the deep safeties' fit and the wide run's stuffs and sideline, measured against all three, and SF10's tables rebuilt on its engine. Every size below is proposed, not confirmed.

How it was measured: `match_check` with `AFM_RUNDIAG=1 AFM_GAPS=1`, the SF9 read-out and the lane's new rows for yards after the catch by length (beside nflverse 2024), 4,000 NFL games on seed 11, main 371fb53 and after, every variant on the same games; the league at the NFL, FBS and high school on seeds 1 and 2 (2,000 games, `AFM_GAPS=1`) and on world seed 1's 32 NFL clubs (`AFM_AB_WORLD`), main and after; `fit_check` on the audit's NFL seed 161 (2,400 games an arm): the first back's Speed +15 (0.14 s in the forty, every other rating held) and the drawn fast and slow backs (four men a band, 4.42 against 4.64 s) in power gap and wide zone, and SF10's 17 arms, main and after; real backs' 20+ runs against the forty with the draft slot held (nflverse play-by-play 2018-25, players and the combine, as `data/real-world/scripts/ab_real.py`), with and without where they carry held.

What was wrong on main:

- **A squared-up tackler mirrored the carrier** (lane SF9's trace). Once a tackler was 0.4 to 5 yards in front of the carrier and within 6, he shuffled to where the carrier's line met his at 4 times the gap a second, up to his full top speed (`game/sim.rs`, `pursue_cap`), so the carrier could seldom get round him: deep safeties' head-on tackles on backs' runs missed 6.7% of the time.
- **He squared up to every runner in front.** Whichever way the runner was going, a man in front broke down and shuffled, so the meeting at the safeties' depth was a duel of moves and the runner's speed never decided it: speed got a back to 10 yards, never from 10 to 20.
- **Every deep back fitted the run the same way.** At 9.75 yards, attacking once the runner was 4.75 past the line, coming down at 0.6 of his speed, whatever the coverage, his job or the run's side (`game/scrimmage.rs`, `defend`), so both safeties met the runner at the same depth on every run.
- **The rotated-down safety backed up.** At 8.1 yards (`ss_depth`), just outside the box count, he took the deep fit and dropped toward 9.75 before he came forward.
- **The carrier ran at 95% of his top speed** (`run_carrier`). One arm on the ball costs a sprinter 1-2% (Barr et al. 2015, international rugby players over 40 m with the ball in one hand). At 95% linebackers, who run about 2.5% under the typical back without a ball, ran backs down from behind: they ended 16.5% of the 20+ runs (real 7.2%).

What changed (`game/sim.rs`, `pursue_cap`, `run_carrier`; `game/scrimmage.rs`, `defend`, `deep_fit`):

- **He squares up only to a runner coming at him** (`SQUARE_UP` 0.7): the cosine between the runner's line and the line from the runner to him over 0.7, about 45 degrees, or the runner nearly still. A runner crossing his face or going away from him he races on his angle at full speed (the pursuit intercept, with the last man's deeper angle), so a faster runner beats it and a slower one is run down or pushed to the sideline.
- **Squared up, he shuffles by his feet and the carrier's cut** (`SHUFFLE` 0.65, `SHUFFLE_FEET` 0.01, `SHUFFLE_CUT` 0.005): sideways at most 0.65 of his top speed for a tackler with feet 60 (half Agility, 0.3 Tackling, 0.2 Pursuit) against a carrier with a cut of 70 (half Agility, half Elusiveness), 0.01 more a point of his feet and 0.005 less a point of the carrier's cut, held to 0.4-1.0 (his full top speed before).
- **The deep backs fit by their job in the call** (`deep_fit`; each as the runner's depth when he attacks, his depth until then and the share of his top speed he comes down at). A safety with the deep zone in a single-high shell (Cover 1, Cover 3, Cover 1 robber, Cover 3 match) is the last line: 7, 13 and 0.3 (`FIT_LAST`). A safety on the far side of the run (the runner a yard or more past the ball the other way) plays the cutback from depth: 6.5, 12 and 0.4 (`FIT_AWAY`). The rest fill the alley: 4.75, 9.75 and 0.85 (`FIT_ALLEY`). A safety's share moves 0.1 a step of his Downhill lean (`FIT_DOWNHILL`), and a back rolled into the box keeps 0.6 (`BOX_SHARE`).
- **A deep back already nearer the line than his fit holds his depth** there, so the rotated-down safety plays the run from 7.2 to 9 yards.
- **The ball carrier runs at 98.5% of his top speed** (`CARRY_PACE`; backs, receivers after the catch, returners and quarterbacks alike).
- **The tables on the new engine:** the coaches' call table (`call_values 2000`), the roster table (`call_values roster 100000`) and the analysts' four models (`xcomp_fit 600`, `xyards_fit 600`, `xpass_fit 600 7`, `decide_fit 1500 1`).

Backs' designed runs, the catch and returns (`match_check`, 4,000 NFL games on seed 11, main > after; real nflverse 2024):

| | main | after | real |
|---|---|---|---|
| 10+ yards, of carries | 8.71% | 10.56% | 10.4% |
| 20+ | 0.98% | 2.26% | 2.26% |
| 40+ | 0.30% | 0.51% | 0.56% |
| of the runs that reach 10, on to 20 | 11.2% | 21.4% | 21.8% |
| of the runs past 20, on to 40 | 30.3% | 22.5% | 24.6% |
| 5-9 yards, of carries | 31.8% | 28.2% | 24.5% |
| safeties end the 5-9, 10-19 and 20+ | 49.3, 74.1, 17.1% | 43.3, 56.4, 34.7% | 23.3, 46.8, 39.8% |
| linebackers end the 20+ | 16.5% | 12.2% | 7.2% |
| corners end the 20+, touchdowns of them | 28.1, 24.4% | 20.7, 21.5% | 24.2, 22.3% |
| inside the tackles: a carry, 10+, 20+ | 3.84, 6.5%, 0.76% | 3.99, 8.4%, 1.72% | 4.26, 9.6%, 1.93% |
| wide of them | 5.24, 13.4%, 1.44% | 5.63, 15.1%, 3.40% | 4.72, 13.5%, 3.69% |
| rushing yards a team game | 116.3 | 121.9 | 118.0 |
| yards after the catch | 5.14 | 5.17 | 5.20 |
| catches going 1-4, 10-19 and 20+ after the catch | 46.6, 9.8, 4.1% | 45.8, 10.4, 3.9% | 34.3, 14.3, 3.9% |
| kickoff returns of 40+, punt returns of 20+ | 2.5, 4.2% | 2.7, 4.8% | 5.0, 8.0% |
| points a team game | 25.93 | 26.61 | 22.51 |

The burner (`fit_check`, NFL seed 161, 2,400 games an arm; points of carries for the first back; real backs from nflverse 2018-25 at the same draft slot, per the same gap in the forty):

| | main | after | real |
|---|---|---|---|
| Speed +15 (0.14 s), power gap and wide zone: 20+ | +0.3, +0.3 | +0.5, +0.4 | +0.67, with where they carry held +0.51 |
| the same: 10+ | +1.0, +0.9 | +0.8, +0.8 | +1.09 |
| the same: yards a carry | +0.25, +0.17 | +0.32, +0.30 | |
| the same: points a game | +0.41, +0.03 | +0.47, +0.58 | |
| drawn 0.22 s faster (4.42 against 4.64 s, 206 against 239 pounds), as recruited: 20+ | +0.1 ±0.3, +0.0 ±0.4 | +0.6 ±0.4, +0.8 ±0.3 | +1.06 (0.52 to 1.61), held +0.80 (0.22 to 1.39) |
| the same: 10+ | -0.3, -0.4 | +0.5, +1.1 | +1.71 |
| the same: of the runs that reach 10, on to 20 | +2.1, +1.1 | +4.1, +5.0 | +6.3 |
| the same: yards a carry, before contact | -0.06, -0.09; +0.26, +0.24 | +0.14, +0.32; +0.44, +0.61 | +0.28 over expected (the spec) |

SF10's arms on the new engine (`fit_check`, NFL seed 161, 2,400 games an arm, each change on team 0 against its typical arm on the same games; main 371fb53 > after, the after side ± its standard error over the 40 blocks; main's side is SF10's own "after" column within a few hundredths):

| arm | measure | main | after |
|---|---|---|---|
| three corners built for man (West Coast) | man, single-high (points of calls), points against a game | +7.32, +3.51, -0.13 | +7.39 ±0.19, +3.33 ±0.30, -0.52 ±0.21 |
| the nickel corner's Man Coverage +15 | man | +2.59 | +2.57 ±0.18 |
| the first corner's Man Coverage +15 | man, points against | +1.70, -0.75 | +1.92 ±0.17, -0.96 ±0.22 |
| the passer's Arm Strength +15 | air yards a throw, the X's targets 20+ out (points of his targets), points | +0.46, +0.88, +1.27 | +0.42 ±0.05, +0.89 ±0.46, +0.87 ±0.28 |
| the passer's Deep Accuracy +15 | air yards, points | +0.10, +1.32 | +0.18 ±0.05, +0.66 ±0.25 |
| the X receiver's Speed +15 | his targets 20+ out, points | +1.34, +0.39 | +1.24 ±0.43, -0.11 ±0.29 |
| the left tackle's Pass Blocking +15 | points | +0.41 | -0.28 ±0.22 |
| the second tight end's Run Blocking +15 (a 12 personnel scheme) | 12 personnel, points of snaps | +0.22 | +0.07 ±0.23 |
| the third tight end's Run Blocking +15 (a 13 personnel scheme) | 13 personnel | +0.29 | +0.21 ±0.28 |
| the first back at 195 pounds (power gap) | the second back on, points of the short-yardage snaps | +36.0 | +33.5 ±0.7 |
| the first back at 235 pounds | the same | +1.1 | +3.4 ±0.9 |
| the second back's Pass Blocking +15 | the second back on, points of the third and long snaps | +0.57 | +0.75 ±0.48 |
| the first back's Leg Drive +15 (power gap) | gap runs, points of the backs' runs; points | +0.27, +0.53 | +0.29 ±0.17, -0.31 ±0.19 |
| the passer's Speed +15 (West Coast; spread option) | designed runs a team game | +0.01 and +0.62 | -0.00 ±0.02 and +0.57 ±0.04 |
| the free safety's Zone Coverage, Play Recognition, Football IQ and Anticipation +15 (West Coast; man pressure) | single-high, blitz (points of snaps), points against | -0.34 and +0.24, -0.19 and -0.31, -1.21 and -1.02 | -0.13 ±0.26 and -0.18 ±0.26, +0.06 and +0.06, -1.09 ±0.21 and -0.93 ±0.22 |
| the first back's Speed +15 (power gap; wide zone) | wide runs, points of the backs' runs; his carries a game | -0.04 and +0.40, +0.28 and +0.32 | +0.03 ±0.08 and +0.56 ±0.24, +0.44 ±0.11 and +0.26 ±0.11 |

The league (`match_check`, 2,000 games a seed, main > after; real NFL 2024, nflverse):

| | NFL seed 1 | NFL seed 2 | FBS seed 1 | FBS seed 2 | high school seed 1 | high school seed 2 | real NFL |
|---|---|---|---|---|---|---|---|
| points a team game | 26.75 > 27.46 | 27.89 > 28.74 | 25.68 > 26.34 | 25.44 > 26.67 | 20.56 > 21.50 | 19.45 > 20.33 | 22.51 (FBS 26.97) |
| yards a carry | 4.37 > 4.60 | 4.29 > 4.38 | 4.76 > 4.96 | 4.63 > 4.83 | 5.59 > 5.89 | 5.24 > 5.48 | 4.50 (FBS 5.12) |
| runs of 10+, of carries | 7.2 > 9.6% | 6.9 > 8.7% | 11.7 > 12.2% | 11.3 > 11.5% | 18.9 > 17.6% | 16.6 > 15.1% | 11.6% (FBS 14.5%) |
| runs of 10-19 yards | 8.1 > 8.7% | 7.8 > 8.0% | 12.6 > 11.0% | 12.3 > 10.5% | 19.2 > 15.5% | 17.1 > 13.5% | 8.2% |
| runs of 20+ yards | 1.1 > 2.2% | 1.1 > 2.0% | 1.3 > 2.5% | 1.1 > 2.4% | 1.7 > 3.7% | 1.5 > 3.1% | 2.3% |
| yards after the catch | 5.03 > 5.02 | 5.30 > 5.30 | 6.55 > 6.61 | 6.81 > 6.95 | 7.03 > 7.20 | 7.06 > 7.22 | 5.20 |
| third down conversion | 44.3 > 44.0% | 45.5 > 45.2% | 41.7 > 41.4% | 42.0 > 41.4% | 40.3 > 39.1% | 40.8 > 40.1% | 39.8% |
| kickoff returns of 40+ | 2.7 > 2.9% | 2.5 > 2.6% | 0.8 > 0.9% | 1.0 > 1.0% | 0.5 > 0.7% | 0.6 > 1.0% | 5.0% |
| punt returns of 20+ | 4.5 > 5.1% | 4.0 > 4.9% | 4.6 > 5.4% | 4.9 > 5.6% | 4.8 > 6.3% | 4.6 > 5.3% | 8.0% |

World seed 1's 32 NFL clubs (`AFM_AB_WORLD`, 2,000 games on seed 1, main > after; real nflverse 2024): backs' designed runs of 10+ 9.49% > 10.82% (real 10.4%), of 20+ 1.12% > 2.49% (2.26%), of the runs that reach 10 on to 20 11.8% > 23.0% (21.8%), of those past 20 on to 40 27.2% > 22.5% (24.6%), safeties ending the 20+ 16.6% > 31.7% (39.8%); yards a carry 4.51 > 4.75 (4.50), inside the tackles 3.96 > 4.08 (4.26) and wide of them 5.84 > 6.35 (4.72); rushing yards 130.9 > 137.2 a team game (118.0), yards after the catch 5.59 > 5.60 (5.20), points 28.33 > 29.53 (22.51), third down 45.3% > 45.1% (39.8%); kickoff returns of 40+ 4.4% > 4.5% (5.0%), punt returns of 20+ 5.1% > 6.7% (8.0%); backs' yards a carry over 204-carry blocks spread 0.53 > 0.69 (real 0.57).

Why:

- **Speed now decides the meeting in the open.** A tackler squares up only to a runner coming at him (`SQUARE_UP`); one crossing his face or past him he races on his angle, so a faster back beats the angle and a slower one is caught. Of the runs that reach 10, 21.4% go on to 20 (main 11.2%, real 21.8%), and the drawn fast backs take 4.1 and 5.0 points more of theirs on to 20 than the slow (main 2.1 and 1.1, real 6.3). The first back's Speed +15 adds 0.5 and 0.4 points of 20+ runs (main 0.3 and 0.3; real 0.51 with where they carry held, 0.67 raw), and backs drawn 0.22 s faster as recruited +0.6 and +0.8 (main +0.1 and +0.0), inside the real band (+1.06, 0.52 to 1.61) and at its usage-held +0.80.
- **The tail at real with the right man at the end of it.** The deep middle safety stays deep and comes down slowly as the last line (`FIT_LAST`), so of the runs past 20 he ends 34.7% (main 17.1%, real 39.8%) and 22.5% go on to 40 (main 30.3%, real 24.6%); a deep back coming down at full speed made 42% of them go 40+ (lane SF9). The alley player still fills at 0.85 of his speed, so 5-9 yard runs fall from 31.8% to 28.2% of carries (real 24.5%) and safeties end 43.3% of them (main 49.3%, real 23.3%).
- **Yards after the catch stay at real**, since the same rule meets receivers: a defender coming at the catch from in front squares up, one chasing from the side or behind races his angle. 5.14 > 5.17 a catch (real 5.20); loosening the shuffle alone, as lane SF9 tried, gave 5.75.
- **The carrier's pace.** At 98.5% of his top speed linebackers end 12.2% of the 20+ runs (main 16.5%, real 7.2%) and corners 20.7% (28.1%, 24.2%).
- **Returns move little** (kickoff returns of 40+ 2.5% > 2.7%, real 5.0%; punt returns of 20+ 4.2% > 4.8%, real 8.0%): every lever in this lane moved them by a few tenths, so the return's own wall (the cover men at the catch on punts, the kicker and two safeties on kickoffs, lane EG3's trace) holds them, not the open-field shuffle (`GAPS.md`).
- **The roster table pays the back's speed.** Rebuilt on the new engine, it pays the first back's Speed 0.091 yards a point summed over its 12 runs (main 0.055): outside zone 0.0103 a point (0.0028), the toss 0.0163 (0.0100), inside zone 0.0097 (0.0051), counter 0.0090 (0.0020). Every run pays it more, so a burner leans the call to runs (his carries +0.44 and +0.26 a game, main +0.28 and +0.32) but barely to the wide ones (+0.56 ±0.24 points of the backs' runs in wide zone, +0.03 in power gap).
- **Inside the tackles the tail is real; wide of them it runs past real.** On world seed 1's clubs 1.92% of the inside runs go 20+ (real 1.93%) and 4.27% of the wide ones (3.69%), at 6.35 yards a carry wide (main 5.84, real 4.72), since only 8.0% of the wide runs are stuffed (real 23.2%): the wide run's stuffs and sideline are what the open field doesn't reach (below, `GAPS.md`). Yards a carry go 4.51 > 4.75 there (real 4.50), and on the test rosters 4.37 > 4.60 and 4.29 > 4.38.
- **The league: the runs past 20 double at every level and points rise 0.7 to 1.2 a team game.** The NFL test rosters 26.75 > 27.46 and 27.89 > 28.74 (real 22.51), FBS 25.68 > 26.34 and 25.44 > 26.67 (real 26.97), high school 20.56 > 21.50 and 19.45 > 20.33, the world's clubs 28.33 > 29.53. At FBS and high school the 10-19 yard runs fall 1.6 to 3.7 points, since the alley player comes down sooner and the last man stays deep. Every total stays inside its test band.
- **SF10's arms keep their shape.** Every call's lean moves within its noise: three corners built for man still lean 7.4 points to man and 3.3 to single-high, the big arm still throws 0.42 yards deeper, a 195-pound first back still gives way on 33.5 points more of the short-yardage snaps. Points move by up to 0.8 on single arms (the passer's Deep Accuracy +1.32 > +0.66, the first back's Leg Drive +0.53 > -0.31), but the typical arm's own games move a context's arms together: in the West Coast the six arms that change nothing on its offense (the corners, the free safety, the second back's Pass Blocking, the passer's Speed) score 0.27 a game under it here and 0.12 over on main, and in power gap the eight drawn backs 0.37 under (main 0.18 over). With that taken out the passer's Arm Strength, his Deep Accuracy, the X's Speed, the left tackle's Pass Blocking and the back's Leg Drive move -0.01, -0.27, -0.11, -0.30 and -0.29 against main, inside the noise.

Tried and dropped, each on the same 4,000 NFL games: the carrier's pace alone at 0.97, 0.985 and full speed, before the square-up rule (the 40+ share of the runs past 20 38%, 41% and 42.5%, real 24.6%; points up to 27.8); the square-up cone at 0.5 and 0.85 (20+ 1.81% and 1.71%, against 1.77% at 0.7 in the same build: the cone's width barely matters); the last man's lead at 1.2 and 1.6 (`LAST_MAN_LEAD`, 1.4 kept: 20+ 1.59%, and 2.04% with yards after the catch 5.30); the alley filling at 0.7 of his speed (20+ 1.38%); the alley later, at 5.5 and 10.5 yards (10+ 11.7%); the last line and the far safety deeper, at 14 and 13 yards (20+ 1.78%, the tail unchanged); the last line's and the far safety's shares at 0.4 and 0.4 (20+ 2.10%) and at 0.35 and 0.35 (2.29%, within the noise of the 0.3 and 0.4 kept, 2.25% on the old tables).

Speed (`match_check`, 2,000 games on seeds 1 and 2 at each level, main and the lane in turn on an otherwise quiet machine, the order swapped between seeds; the tool's own time): the NFL +0.1% (pairs +3.3% and -3.1%), FBS -2.6% (-1.5% and -3.6%), high school -1.9% (0.0% and -3.7%), inside a pair's noise.

Still off (`GAPS.md`): the wide run's stuffs and sideline (wide runs stuffed 11.6% of the time, real 23.2%, linebackers making 2.3% of those stuffs, real 27.7%; wide runs of 1-4 yards out of bounds 0.1%, real 12.8%): the open-field tackle doesn't reach them, since those runs end inside the tackle box before the open field, so they need the wide run's path and the defense's force and flow; returns' breakaways; safeties still end 43% of the 5-9 yard runs (real 23%) and 56% of the 10-19 (47%), linebackers 12% of the 20+ (7%); catches going 1-4 yards after the catch 45.8% (real 34.3%) and 10-19 10.4% (14.3%); backs' yards a carry spread over 204-carry blocks 0.84 on the test rosters (main 0.69, real 0.57) and 0.69 on world seed 1's clubs (main 0.53): the luck in it is at real (0.43 within one back, real about 0.42) and the lasting part wider (0.52 on the world's clubs, real about 0.39), so who the back is decides too much of his yards a carry; points.

## Fit scores from what wins in the engine (scheme-fit lane SF11, item 24, 2026-11-21)

Design: `docs/design/scheme-fit.md`, item 24: refit the world's fit scores from measured value per scheme and job, so installs and recruiting chase what wins, with the free and strong safety, the Mike, Will and Sam and the outside and slot corner apart (Cain: yes, job by job). The lane changes no game code, so `match_check` plays the same games in the same time; it changes what the world's staffs and recruits read (how it works in `docs/reference/how-it-works/staff-and-coaches.md`, systems and fit).

How it was measured: `fit_check values fbs 200000 151` (`afm-tools`, `fit/values.rs`; 39 minutes on 4 cores). Team 0 of the FBS test league is set to the level's typical team and plays the other 31 in turn under the typical staff; game k runs offense k mod 5, front (k/5) mod 2 and coverage family (k/10) mod 4. In each game 24 of its men (the passer, the first back, the fullback, the three receivers, the first tight end, the five linemen, both edges and inside men, the three linebackers, the three corners and both safeties) have each watched rating moved by a uniform draw of up to 18 points either way and their weight by up to 8%: 157 ratings and weights on offense, 159 on defense (each job's ratings the engine reads for his work there, the ones the old mixes weighed, Football IQ, and Communication for the passer, the center, the linebackers and the safeties). Each game is played twice on the same seed with every move turned the other way, so the opponent, the field and the game's own luck drop out of the pair (its two halves' points correlate 0.32). A rating's worth in a system is the least-squares slope of half the pair's difference in points scored (offense) or kept off the board (defense, read by the front and by the coverage family) on half the moves' difference, over 20,000 pairs in each offense, 50,000 in each front and 25,000 in each coverage family (a slope's noise about 0.005 points a game a rating point). A rating's systems share one weight unless they differ past the noise (chi-square at 5%, then each pulled toward the shared one as far as the noise says, DerSimonian-Laird); a shared weight is kept only past 2.5 of its noise. A job reads its men's weights: the three receivers' as one, the two guards', the two edges', the two inside men's and the two outside corners'.

What it found, in points a game for a rating point on the typical FBS team (`crates/afm-engine/data/fit_values.csv`):

- **Corners.** Outside: Man Coverage 0.037 in man pressure and 0.010 in Cover 2, Zone Coverage 0.045 in Cover 2 and 0.020 in man pressure, Speed 0.024 in man pressure and 0.012 in Cover 2. The slot: Man Coverage 0.051 in man pressure and 0.018 in Cover 2, Zone Coverage 0.044 in quarters, and his weight costs most in man pressure (0.013 a pound).
- **Linebackers.** The Mike's Zone Coverage 0.032 in Cover 2 and 0.017 in man pressure; the Will's Tackling 0.032 in Cover 2 and 0.022 in Cover 3. The Sam is worth 0.010 a point in the front and almost nothing in the coverage, since the typical team's nickel takes his snaps.
- **Safeties.** The free safety's Tackling 0.037 in Cover 3 and 0.027 in man pressure, his Zone Coverage 0.021 and Anticipation 0.014 in every family (his Speed and Ball Tracking pay nothing past the noise); the strong safety's Zone Coverage 0.038 in Cover 3 and 0.024 in man pressure.
- **The passer.** Arm Strength 0.103 in power gap and 0.072 in spread option, Short Accuracy 0.055 in the air raid and 0.039 in power gap, Eye Manipulation 0.046 in the West Coast and 0.029 in wide zone, Pocket Presence 0.033 in power gap and 0.010 in spread option; his Speed costs points in every offense (`GAPS.md`).
- **Backs, receivers and tight ends.** The back's Elusiveness 0.023 in the West Coast and 0.001 in power gap; receivers' Speed 0.021 in spread option and 0.019 in the air raid, 0.013 in the West Coast; the tight end's Run Blocking 0.055 in wide zone and 0.043 in the West Coast, 0.007 in spread option, his Route Running 0.013 in the air raid.
- **The lines.** Linemen read the same in every offense but a guard's Run Blocking (0.026 in wide zone to 0.031 in spread option) and the left tackle's Flexibility (0.011 in spread option) (`GAPS.md`); the 4-3 pays an edge's Anchor 0.011 (the 3-4 0.005) and an inside man's Strength 0.009 (0.005).
- **The systems themselves.** The typical team scores 31.9 a game in the air raid and the West Coast, 31.0 in wide zone and spread option and 30.6 in power gap, and allows 19.7 in man pressure, 20.8 in Cover 3, 21.1 in Cover 2 and 21.4 in quarters. Fit is read against each kind's average, so this doesn't enter it (`GAPS.md`).

Of the old hand-set mixes' weight, 17% sat on ratings the table now pays nothing at the position's jobs (tight ends 47%, backs 29%, edges 26%; the audit's 74% was measured before lanes SF1-SF10 made many of those ratings count, on a +15 test at 3,000 games an arm).

## The red zone (engine gaps lane EG6, item 8, 2026-11-22)

Design: `docs/design/engine-gaps.md`, item 8: the red zone plays tight like real, measured again after SF7's coverage jobs and fixed at the root where it was still loose: the short field, the end line, tight windows and the defense's red zone calls. The spec's targets of 53.0% completion and 19.0% touchdowns a throw were an older reading: nflverse 2021-25 gives 56.7% and 23.2% on every red zone throw, and 60.4% and 24.6% with FTN's throwaways taken out (2022-25), which is what the engine's throws compare with, since it throws few away (lane EG7). Cain was away, so every choice below is the most realistic option under his proxy, and every size is proposed, not confirmed.

How it was measured: `match_check` with `AFM_GAPS=1`, which now prints the red zone by yards to the goal (1-5, 6-10, 11-15, 16-20 and 21+) beside real: throws a team game, completion, touchdowns and picks a throw, air yards and the share caught at or past the goal line; throws under a yard of separation (against FTN's contested share), their catch rate and breakups; sacks, scrambles, throwaways, man, Cover 0, rushers, time to throw and pressure on dropbacks; designed runs' yards and touchdowns, dropbacks of snaps, on first down, and Cover 0's rushers and sacks; man and zone completion and touchdowns inside the 10, from 11 to 20 and outside; targets by route inside the 10 and from 11 to 20; and the goal line: third and fourth and goal from the 1 and from the 2 and two-point tries, each with its scores, dropbacks and runs, Cover 0 and man, and the runs against Cover 0 and against man. Real: nflverse play-by-play 2021-25 (regular seasons, no two-point tries or spikes on downs), FTN charting 2022-25 (throwaways, contested balls) and nflverse participation data 2023-25 (man or zone, the coverage, rushers, time to throw, pressure, routes). The NFL and FBS on seeds 1 and 2 at 2,000 games and high school at 1,000, main 64f0f00 and after; `fit_check` on the coverage jobs, NFL seed 161 at 2,400 games an arm and FBS seed 151 at 3,000.

What was wrong on main (the NFL test league, both seeds; real in brackets):

- **A man defender played the goal line like midfield.** He kept his cushion and stayed over the top of his man for the deep ball (`cover_inner`), though with the end line a few yards behind the receiver there is little deep ball left. Man coverage was completed 68.0% of the time inside the 10 (54.0%) and 70.8% from 11 to 20 (60.2%), against 71.4% outside the 20 (61.9%): the red zone took away only what it took away everywhere.
- **The passer waited for open grass.** The window he needed was the same at the goal line as at midfield, so he threw under a yard of separation on 12.4% of throws inside the 5 (FTN's contested share 22.7%) and 12.5% from 6 to 10 (20.0%), against 12.1% outside the 20 (13.4%).
- **The defense seldom called man near its goal.** Man on 42.5% of dropbacks inside the 5 (75.1%), 30.4% from 6 to 10 (55.4%) and 28.2% from 11 to 15 (47.3%); Cover 0 on 10.2% inside the 5 (40.2%). Its only red zone call was one step: Cover 0 two and a half times as often inside the 10 (`call_defense_memo`).
- **Cover 0 sent seven.** The double-A pair, the Mike again and the free safety (`assign_defense`): 7.05 rushers on Cover 0 inside the 5 (5.26), with man on every receiver and nobody left, and a sack on 16.4% of its dropbacks there (4.2%).
- **Offenses threw on first down near the goal as at midfield.** The pass table reads down, distance, score and clock, not the field (`tables::real_pass_rate`): first downs inside the 5 were thrown 43.0% of the time (31.1%), from 6 to 10 54.4% (38.4%) and from 11 to 15 56.5% (43.6%).
- **A try was called as first and goal from the 2.** The pass table has no row for a try, so it read one as a first down: tries were dropbacks about 30% of the time (29.5% measured on this lane's defense before the fix; real 71.6%), and the box score had them made 45.0% of the time (real 47.5%) only while runs at the goal line were easy.
- So red zone completion was 65.5% (60.4%), a touchdown came on 28.9% of red zone throws (24.6%) and on 68.5% of trips (59.8%), points a drive were 2.75 (2.00) and pass touchdowns 2.24 a team game (1.46).

What changed (`game/scrimmage.rs`, `cover_inner`, `quarterback`, `assign_defense`; `game/calling.rs`, `call_offense_memo`, `call_defense_memo`, `cover0_rush`, `Situation::trying`; `game/sheet.rs`, `def_call`):

- **The end line takes the deep ball away** (`END_LINE_ROOM` 20, `END_LINE_UNDER` 1.5, `END_LINE_INSIDE` 2.0, `END_LINE_SIT` 0.4). With shut = how much of the deep ball the end line has taken from his man (0 with 21 or more yards to the end line, 0.55 at the goal line, 1 at the end line), a man defender's cushion is × (1 - shut), he plays 1.5 × shut yards underneath his man, shades him inside by 0.6 × (1 + 2 × shut) yards and reads his break 0.4 × shut seconds sooner. Over the top has nowhere to land, so he takes away the quick throws inside that the end line leaves.
- **Tighter windows near the goal** (`END_ZONE_TRUST` 0.8, `RED_ZONE_TRUST` 0.8). The open space the passer needs falls 0.8 yards for a catch point in the end zone and 0.8 × clamp((20 - yards to the goal) / 20, 0, 1) for any throw: a catch there is a touchdown and a miss only a down, and the short field leaves no open grass to wait for.
- **The defense's red zone calls** (`RED_ZONE_MAN` 1.15, `RED_ZONE_ZERO` 3.0). Man coverages (Cover 0, Cover 1, Cover 1 robber and 2-man) lean in on the log-odds from nothing at the 20 to 1.15 at the goal line, and Cover 0 another 3.0, in place of the step at the 10. Cover 0's step on 2 or less to go fades as the goal nears (`COVER0_SHORT` 2.5, × 2.5^(1 - near)): stacked on the red zone lean it put Cover 0 on 62% to 75% of third and fourth and goal's dropbacks from the 1 or 2, where real is 50.0%, the same as every snap inside the 3 (50.5%).
- **Cover 0 brings its own rush** (`cover0_pressure`, `COVER0_SIX` 0.45, `COVER0_FIVE` 0.30): six (the double-A pair) on 45% of its snaps, five on 30% (a linebacker two times in three, else a defensive back) and four on the rest (real Cover 0 inside the 5 44%, 28% and 28%, 5.26 rushers; 5.2 to 5.6 at every distance). The free safety and the Mike no longer rush on top; a man left over takes a tight end or the back, or robs the middle. The call sheets' Cover 0 draws the same rush (`def_call`).
- **The short field** (`RED_ZONE_PASS`): inside the 20 the pass table's rate moves on the log-odds by yards to the goal (1-5, 6-10, 11-15, 16-20) and down (first, second, and third and fourth), read on nflverse 2021-25 against the same down, distance and score outside the 20, outside the last two minutes of a half and close late games: first and goal from the 8 is thrown 30% of the time and first and 10 from the 15 36%, against 46% at midfield. The same at every level.
- **Tries from the real tries' mix** (`TRY_PASS` 0.716, `Situation::trying`). A try from scrimmage is no down, so it calls from real two-point tries' own mix, a dropback 71.6% of the time (nflverse 2021-25, 663 tries; 44.0% of the dropbacks made and 56.4% of the runs), in place of first and goal from the 2, and the short field's lean by down passes it by. The same at every level, the call sheets' two-point box too.
- **The tables on the new engine.** The coaches' call table (`call_values 2000`) and the roster table (`call_values roster 100000`) measure Cover 0 on its own rush now (each cell's snaps walk the mix; the column keeps its name, `Cover0/DoubleA`), so what a coach knows of Cover 0 is what he calls; on main they measured the seven-man Cover 0. The other columns don't move. The analysts' four models are refitted on the lane: completion (`xcomp_fit 600`), rush and catch yards (`xyards_fit 600`) and the pass rate (`xpass_fit 600 7`) before the try's mix and Cover 0's fading step, which change only third and fourth and goal and tries, and the fourth-down, try and win expectancy tables (`decide_fit 1500 1`) on the final engine.

The red zone at the NFL (`match_check`, the test league, 2,000 games on each of seeds 1 and 2, their mean, main > after; real nflverse 2021-25, FTN charting 2022-25 and nflverse participation data 2023-25 as above):

| | main | after | real |
|---|---|---|---|
| red zone completion | 65.5% | 62.4% | 60.4% |
| a touchdown a red zone throw | 28.9% | 27.5% | 24.6% |
| touchdowns a trip | 68.5% | 63.9% | 59.8% |
| trips a team game | 3.63 | 3.56 | 3.98 |
| points a drive | 2.75 | 2.64 | 2.00 |
| pass and rush touchdowns a team game | 2.24, 0.97 | 2.12, 0.85 | 1.46, 0.91 |
| field goal tries a team game | 1.85 | 1.94 | 1.85 |
| points a team game | 28.10 | 26.61 | 22.51 |
| completion 1-5, 6-10, 11-15, 16-20 yards out | 62.9, 65.9, 68.0, 69.1% | 57.5, 61.5, 65.9, 68.2% | 56.1, 56.4, 61.4, 65.9% |
| the same outside the 20 | 68.6% | 68.7% | 68.6% |
| a touchdown a throw 1-5, 6-10 | 56.6, 41.0% | 51.6, 38.1% | 48.0, 29.5% |
| an interception a throw 1-5, 6-10 | 2.1, 2.1% | 3.5, 2.6% | 2.6, 2.4% |
| man coverage completed inside the 10, 11-20, outside | 68.0, 70.8, 71.4% | 60.2, 66.7, 71.5% | 54.0, 60.2, 61.9% |
| zone the same | 62.9, 67.8, 67.6% | 59.1, 67.3, 67.7% | 61.3, 68.2, 72.9% |
| man, of dropbacks 1-5, 6-10, 11-15, 16-20, outside | 42.5, 30.4, 28.2, 27.9, 28.5% | 71.8, 52.5, 40.4, 32.6, 29.0% | 75.1, 55.4, 47.3, 43.5, 37.9% |
| Cover 0 the same | 10.2, 6.8, 3.0, 2.9, 2.9% | 44.0, 22.5, 10.3, 5.7, 3.5% | 40.2, 14.2, 7.3, 4.1, 1.9% |
| Cover 0 inside the 5: rushers, a sack of its dropbacks | 7.05, 16.4% | 5.81, 10.3% | 5.26, 4.2% |
| throws under a yard of separation 1-5, 6-10, 11-15 (real: FTN's contested balls) | 12.4, 12.5, 11.5% | 15.2, 14.2, 13.1% | 22.7, 20.0, 17.1% |
| a sack of dropbacks 1-5, 6-10 | 6.0, 3.9% | 6.4, 3.9% | 5.2, 6.6% |
| first downs thrown 1-5, 6-10, 11-15, 16-20 | 43.0, 54.4, 56.5, 56.1% | 35.2, 47.5, 49.7, 51.8% | 31.1, 38.4, 43.6, 45.5% |
| designed runs inside the 5: yards a carry, a touchdown a carry | 1.04, 47.9% | 0.70, 39.6% | 1.06, 41.0% |
| the same from 6 to 10 | 3.24, 20.9% | 2.80, 17.4% | 2.85, 11.7% |
| throws a team game 1-5, 6-10, 11-15, 16-20 | 1.02, 1.34, 1.46, 1.54 | 1.02, 1.35, 1.41, 1.46 | 0.87, 0.96, 1.11, 1.17 |
| inside the 10: slants caught, a touchdown | 74.5, 66.5% | 73.0, 66.9% | 58.3, 45.2% |
| fades (real: go routes), corners caught | 18.8, 34.8% | 21.1, 28.7% | 31.5, 41.5% |
| third and fourth and goal from the 1, from the 2: scored | | 51.8, 45.9% | 55.6, 44.5% |
| the same, Cover 0 of the dropbacks | | 59.7, 59.2% | 50.0% |
| two-point tries: dropbacks, made (the box score's way) | | 73.0%, 44.6% (main 45.0%) | 71.6%, 47.5% |

Every level (`match_check`, 2,000 games at the NFL and FBS and 1,000 at high school on seeds 1 and 2, main > after; real NFL as above, FBS cfbfastR as `match_check` has it):

| | NFL seed 1 | NFL seed 2 | FBS seed 1 | FBS seed 2 | high school seed 1 | high school seed 2 | real NFL |
|---|---|---|---|---|---|---|---|
| points a team game | 27.45 > 26.03 | 28.74 > 27.20 | 26.34 > 25.27 | 26.67 > 25.45 | 21.39 > 20.65 | 20.18 > 19.50 | 22.51 (FBS 26.97) |
| touchdowns a red zone trip | 67.3 > 62.3% | 69.7 > 65.4% | 68.7 > 63.8% | 68.7 > 65.7% | 74.9 > 71.3% | 71.3 > 67.6% | 59.8% |
| red zone completion | 64.7 > 61.4% | 66.2 > 63.4% | 62.2 > 59.8% | 62.5 > 60.5% | 59.6 > 58.5% | 60.3 > 59.1% | 60.4% |
| a touchdown a red zone throw | 28.2 > 26.6% | 29.7 > 28.5% | 29.8 > 27.3% | 29.8 > 28.6% | 28.0 > 27.9% | 28.8 > 27.4% | 24.6% |
| points a drive | 2.68 > 2.56 | 2.83 > 2.71 | 2.39 > 2.33 | 2.44 > 2.36 | 2.42 > 2.37 | 2.32 > 2.25 | 2.00 |
| pass touchdowns a team game | 2.13 > 2.00 | 2.35 > 2.24 | 1.97 > 1.84 | 2.04 > 1.95 | 1.13 > 1.13 | 1.30 > 1.21 | 1.46 |
| rush touchdowns | 0.98 > 0.86 | 0.97 > 0.84 | 1.24 > 1.16 | 1.23 > 1.12 | 1.61 > 1.47 | 1.26 > 1.22 | 0.91 |
| yards an attempt | 7.56 > 7.51 | 8.02 > 7.95 | 7.77 > 7.72 | 8.14 > 8.02 | 7.19 > 7.22 | 7.39 > 7.41 | 7.05 (FBS 7.29) |
| yards a carry | 4.60 > 4.48 | 4.38 > 4.26 | 4.96 > 4.84 | 4.83 > 4.71 | 5.88 > 5.78 | 5.49 > 5.39 | 4.50 (FBS 5.12) |
| third down conversion | 44.0 > 43.3% | 45.2 > 45.0% | 41.4 > 41.1% | 41.4 > 41.7% | 39.0 > 39.6% | 39.8 > 39.6% | 39.8% |
| two-point tries made (the box score's way) | 43.2 > 43.0% | 46.8 > 46.2% | 35.2 > 40.6% | 35.3 > 38.3% | 40.8 > 43.3% | 42.6 > 41.2% | 47.5% |

At FBS man coverage inside the 10 goes from 64.5% to 58.9% (both seeds), man inside the 5 from 41.3% to 71.8% of dropbacks and Cover 0 from 9.5% to 43.9%, with 5.72 rushers (7.04), and tries are dropbacks 69.6% of the time; at high school man inside the 10 63.3% > 58.5%, man inside the 5 42.5% > 74.4% and Cover 0 9.5% > 48.1%, tries dropbacks 69.1%.

Why:

- **Man coverage near the goal is tighter, and only there.** Man inside the 10 is completed 60.2% of the time (main 68.0%, real 54.0%) and from 11 to 20 66.7% (70.8%, 60.2%), while outside the 20 it doesn't move (71.4% > 71.5%, real 61.9%): the end line rule acts only as the end line nears. What is left inside the 10 is the gap man coverage has everywhere (about 10 points over real outside the 20, `GAPS.md`); zone sits near real inside the 20 (59.1% and 67.3%, real 61.3% and 68.2%) and short of it outside (67.7%, real 72.9%).
- **Throws into tight windows.** Inside the 5 the passer throws under a yard of separation on 15.2% of throws (main 12.4%, real contested 22.7%), so more of his throws are broken up or picked: an interception on 3.5% of throws inside the 5 (main 2.1%, real 2.6%), and touchdowns a throw there 51.6% (56.6%, real 48.0%).
- **The defense's calls near the goal near real.** Man on 71.8% of dropbacks inside the 5 (real 75.1%) and 52.5% from 6 to 10 (55.4%), Cover 0 on 44.0% and 22.5% (40.2% and 14.2%), with Cover 0's rush at 5.81 men (real 5.26). The red zone's lean over the mix outside it is about real's: inside the 5 Cover 0's odds are 22 times those outside the 20 (real 35), from 6 to 10 8.0 times (8.5). Cover 0 runs over real at every distance because the mix outside the 20 does (3.5% of dropbacks, real 1.9%, main 2.9%), and man runs short outside the red zone (29.0%, real 37.9%, main 28.5%): the base mix (`shell_mix`) was held in place on main by a call table that read Cover 0 as seven rushers and badly exploitable (`GAPS.md`).
- **Cover 0's sacks fall but stay high.** A sack on 10.3% of its dropbacks inside the 5 (main 16.4%, real 4.2%): the passer's hot read against the free man and the protection's pick-up of the fifth and sixth rusher are the rest (`GAPS.md`).
- **Offenses run more near the goal.** First downs inside the 5 are thrown 35.2% of the time (main 43.0%, real 31.1%) and from 6 to 10 47.5% (54.4%, 38.4%). The table's lean is real's, but the engine passes more on first down everywhere (54.2% outside the 20, real 51.9%) and the call's other leans stack on it in the red zone.
- **The goal line and tries.** On third and fourth and goal from the 1 the offense scores 51.8% of the time (real 55.6%; the analysts' fit on the synthetic leagues reads 47.7%, main's 62.0%) and from the 2 45.9% (44.5%). Cover 0 meets 59% to 60% of those dropbacks (real 50.0%; 62% to 75% before the step faded). Tries are dropbacks 73.0% of the time (real 71.6%) and are made 46.8% with a try played again after a flag left out (real 47.5%; the box score's way 44.6%, main 45.0%, and 39.8% on the lane before the try's own mix).
- **Goal-line runs score at real inside the 5 and fail too often at the goal line.** Inside the 5 a designed run scores 39.6% of the time (main 47.9%, real 41.0%) but gains 0.70 a carry (1.04, real 1.06). On third and fourth and goal runs score 50.4% from the 1 (real 56.7%) and 37.5% from the 2 (52.5%), and on tries 32.1% (56.4%), least against Cover 0 (22% to 52%), which meets 53% to 70% of them: man defenders playing closer and Cover 0's extra rushers and free man stop more runs behind the line (on seed 1, before the tables were rebuilt, the calls' lean to man and Cover 0 alone took the runs inside the 5 from 1.03 a carry to 0.89, and the end line's cushion with it to 0.68). The throws there are at or over real (54.5% from the 1, real 53.3%; 49.6% from the 2, 41.8%; 52.3% on tries, 44.0%), so the goal line as a whole sits near real. From 6 to 10 runs gain 2.80 a carry (real 2.85) and score 17.4% (20.9%, real 11.7%).
- **The quick throws inside are the old gap that is left.** Inside the 10 slants are still caught 73.0% of the time with a touchdown on 66.9% (real 58.3% and 45.2%), and fades and corners too seldom (21.1% and 28.7%, real 31.5% and 41.5%): the underneath and inside leverage takes some of the quick game, but the throws to the end line don't come off as often as real (`GAPS.md`).
- **The league: points fall 1.4 to 1.5 a team game at the NFL, 1.1 to 1.2 at FBS and 0.7 at high school,** touchdowns a red zone trip 3.0 to 5.0 points, and field goal tries rise 0.09 a team game at the NFL. Every total stays inside its test band; FBS points move further under cfbfastR's 26.97 (25.27 and 25.45), since FBS rush touchdowns were already short (`GAPS.md`). Tries are made more often at FBS (35.2% and 35.3% > 40.6% and 38.3%) now that they pass like real.

The coverage jobs (`fit_check run`, the audit's own measure: NFL seed 161 at 2,400 games an arm and FBS seed 151 at 3,000, the arms of lane SF7's coverage jobs and SF6's safety, main > after, measured before the try's mix and Cover 0's fading step; * past the noise of the paired blocks):

| | NFL main | NFL after | FBS main | FBS after |
|---|---|---|---|---|
| the typical team's points allowed, the base (West Coast against the Cover 3 4-3, `wc`) | 22.91 | 21.33 | 22.17 | 21.22 |
| the same against man press (`mp`) | 21.51 | 20.10 | 20.77 | 19.27 |
| CB1 Man Coverage +15 in man press: catch rate on him, yards a target, breakups a season | -6.2*, -1.02*, +2.2* | -5.9*, -1.20*, +2.7* | | |
| CB2 Man Coverage +15 in man press: catch rate on him | -7.7* | -7.4* | | |
| SS Tackling +15 in the base: run stops a season, points allowed | +2.8*, -0.56* | +3.1*, -0.55* | -0.66* points | -0.31 points |
| Mike Football IQ -15 in the base: busts a season | +2.4* | +2.8* | +8.6* | +8.5* |
| points allowed: CB1 Man Coverage +15, base and man press | -0.96*, -0.90* | -0.39, -0.70* | -0.53*, -0.68* | -0.26, -0.67* |
| CB2 Man Coverage +15 | -0.54*, -1.02* | -0.23, -0.56* | -0.56*, -0.66* | -0.41, -0.54* |
| SS Man Coverage +15 | -0.24, -0.61* | +0.48*, -0.16 | -0.33, -0.56* | +0.06, -0.26 |
| CB1 Man Coverage -15 | +0.16, +0.47* | +0.61*, +0.85* | +0.15, +0.34 | -0.13, +0.57* |

What a cover man does on his own keeps its size: the catch rate, yards and breakups on him move as much as on main, and so do the safety's run stops and the Mike's busts. The red zone takes 1.0 to 1.6 points a game off every typical team. In the NFL's base context all four coverage arms' points move up by 0.4 to 0.7 against its typical arm, the better men saving less and the worse man costing more alike, so the shift belongs to that context's typical arm, not to the jobs: the same arms in the repeat set's 21-personnel context (`wc21`, typical 22.57 > 21.55) move CB1 +15 -0.62 > -0.61, CB2 +15 -0.20 > -0.45, SS Man +15 +0.10 > +0.26, CB1 -15 +0.50 > +0.39 and SS Tackling +15 -0.22 > -0.77, inside the noise. In man press, where the arms' men play the coverage on every snap, the better corners still save 0.56 to 0.70 points a game at the NFL (main 0.90 to 1.02) and 0.54 to 0.67 at FBS (0.66 to 0.68).

Tried and dropped, each on NFL seed 1 before the tables were rebuilt: the end line's reach at 30 yards (`END_LINE_ROOM`; 20 kept), with the inside shade at 1 (2 kept) or the step underneath at 1.0 (1.5 kept): red zone completion within a point of the kept sizes and touchdowns a throw up to 0.7 points higher, with goal-line runs at 0.65 a carry (0.74 kept); the deep safeties and the deep zones held shallower near the goal (nothing past the noise); the passer giving up on a covered play sooner near the goal (the path seldom fires); the deep backs fitting the run sooner at the goal line (goal-line runs fell further under real); Cover 0 without the free safety's rush, six rushers on every snap (the real mix kept). On the final tables, the try with only the short field's lean taken off (the table's first and goal kept): tries stayed dropbacks 29.5% of the time and were made 39.8%.

Speed: `match_check` against main, the two run in turn on the same seeds and the order swapped between the seeds: the NFL 2.2% faster (the two pairs 0.5% and 3.8%), FBS 0.7% slower (1.4% and 0.0%) and high school 0.6% faster (5.3% faster and 4.2% slower), all inside the noise of one run. The try's mix and the fading step add a branch a call.

Still off (`GAPS.md`): man coverage completed too often at every distance (inside the 10 60.2%, real 54.0%; outside the 20 71.5%, real 61.9%) and zone too seldom outside the 20; the quick throws inside the 10 too easy and the end zone throws too hard (slants 73.0% caught, real 58.3%; fades 21.1%, real 31.5%; corners 28.7%, real 41.5%); too few contested throws (15.2% inside the 5, real 22.7%) and few throwaways (1.6%, real 7.8%; lane EG7); Cover 0 over real at every distance, on the goal line's downs and on tries, and man short outside the red zone (the base mix); Cover 0's sacks (10.3% inside the 5, real 4.2%); goal-line runs that fail too often, most against Cover 0 (from the 2 37.5%, real 52.5%; on tries 32.1%, real 56.4%), and runs from 6 to 10 that score too often (17.4%, real 11.7%); first downs thrown more than real near the goal; more snaps a trip than real; FBS rush touchdowns (1.16 and 1.12, real 1.58); points.

## The quarterback off script (engine gaps lane EG7, item 9, 2026-11-22)

The target (`docs/design/engine-gaps.md`, item 9): 10% of throws after 3.94 s (engine 2.5%), time to throw 2.71 s (2.48), scrambles 1.97 a team game (1.60), sacks a pressure 0.283 (0.24), sack rate 6.8% (5.5%), and 18% of dropbacks out of the pocket (FTN charting). Built over main 278ae30; every size is proposed, not confirmed, each choice the most realistic option under Cain's proxy.

What was wrong:

- **He felt every rusher he couldn't see.** A free man outside his eyes within 7 yards was felt at 0.15 + 0.005 × Pocket Presence a tenth of a second, about 0.5 for the NFL's typical starter, so he felt nearly every blind-side rusher within two tenths of a second and got rid of it (`urgent`) or slid away. NFL passers were sacked on 3.7% of their dropbacks in the pocket (real 6.1%, FTN charting) and 0.209 times a pressure (PFR 0.283): the "quick clock" of `GAPS.md` was this sense, not the clock itself, which already let in-pocket throws go at about real times (2.31 s, real 2.40).
- **He had no hot.** The same sense was all he had for the man his protection left free, the blitzer real passers count before the snap; at 0.5 a tenth it hid that.
- **The drill waited for the run.** Receivers broke off their routes only once he took off or at 4.6 s, and only those who may freelance, so a passer who broke out to throw threw to men still running the play's routes.
- **He broke out only when nothing at all was there**, the checkdown and the forced ball included, and out of the pocket the checkdown, the forced ball and the throwaway came on the pocket's clock.
- **Zone defenders kept to their zones** while the receivers worked the drill.

What's built (`game/scrimmage.rs`):

- **The feel** (`quarterback`, `FEEL_UNSEEN`, `FEEL_SLOPE`): 0.18 a tenth of a second for his level's typical starter and 10% more a 10 points of Pocket Presence over his (read against the typical starter, the lane's rule; it read the raw rating). He feels a blind-side rusher within 0.4 s about half the time, within a second 86% of the time.
- **His hot** (`hot_men`, `KNOW_HOT`, `KNOW_READ`, `KNOW_LOOK`): a rusher the protection leaves free at the snap is the man the protection call gives the passer, and he knows he is coming, whether he is in his eyes or not: 85% of the time for his level's typical starter, 10 points more a 10 of his Football IQ and Play Recognition over his (0.6 and 0.4, as `BLITZ_ID` weighs them), 8 less a 10 of the blitzer's Blitzing over his position's typical starter's (his timing and disguise; a man on the line is read on the passer's side alone), between 40% and 98%, the sizes of the back's own read of a blitzer (`picks_up`). A disguise that fooled him or a busted blocker leaves him knowing none. With the feel alone at its new size he knew his hot no better than any blind-side man: 44% of the sacks against Cover 0 and 31% of the rest came from the man the protection left free, and Cover 0's sacks went back to 15.8% of its dropbacks (lane EG6 had brought them to about 10%; real 7.6% outside the 20).
- **The call at the end of his reads** (`BAIL`, `Pass::bail`): the first time his reads or his clock run out with nobody open, after 1.6 s and with a way out, he breaks out of the pocket with chance clamp(0.8 × (1 + lean(Scramble)), 0, 1), or stays for the checkdown. Once a play: a passer who stays has made his call.
- **The drill from the break-out** (`run_route`, `DRILL_DRIFT`): every receiver works it from the moment he breaks out to throw, since scramble rules are coached, not a freelance, each 3 yards toward the side he broke to; past 4.6 s or once he runs, the freelancers break off as before.
- **The drill's own clock** (`DRILL_CLOCK`, `DRILL_AWAY`): out of the pocket the checkdown and the forced ball wait until 1.5 s after he broke out (or his clock, if later), and he can throw it away from 1.0 s after it.
- **Plaster** (`defend`, `PLASTER_READ`): once he breaks out, a zone defender takes the nearest receiver nobody has in man, 0.4 s after (0.1 s sooner a 10 points of Play Recognition over his level's typical starter's), as zone defenders are coached to when the play breaks down.
- **The read-out.** Each play record carries FTN's out-of-the-pocket flag (`PlayRecord::out_of_pocket`: outside the tackle box at the throw or the sack, on the drill, or scrambling; `game/stats.rs`), and `match_check` with `AFM_GAPS=1` prints the passer off script against FTN charting and nflverse participation data 2023-25: dropbacks in and out of the pocket, pressured or not, each with its sacks, scrambles, throws, throwaways, completion, yards an attempt, air yards, yards and EPA a dropback and time to throw, and when pressure and sacks come.

The league (`match_check` with `AFM_GAPS=1`, seed 1, 2,000 games at the NFL and FBS and 1,000 in high school, main > after, on the rebuilt tables; real nflverse 2021-25, PFR's pressures, FTN charting and participation data 2023-25 at the NFL, cfbfastR 2022-24 at FBS):

| | NFL | real | FBS | real | high school |
|---|---|---|---|---|---|
| sacks a team game | 1.60 > 1.89 | 2.41 | 1.82 > 1.92 | 2.00 | 1.77 > 1.78 |
| sack rate | 4.4% > 5.2% | 6.8% | 5.2% > 5.5% | 6.05% | 7.7% > 7.7% |
| sacks a pressure | 0.209 > 0.257 | 0.283 | 0.212 > 0.236 | | 0.253 > 0.269 |
| pressure on dropbacks | 20.4% > 19.7% | 22.2% | 24.1% > 22.8% | | 29.7% > 28.0% |
| sacked in the pocket, of its dropbacks | 3.7% > 4.6% | 6.1% | 4.3% > 4.8% | | 6.7% > 6.8% |
| out of the pocket, of dropbacks | 6.3% > 5.3% | 18.8% | 6.7% > 5.7% | | 6.9% > 6.2% |
| out of it: thrown, scrambled, sacked | 31%, 55%, 14% > 40%, 45%, 14% | 66%, 26%, 9% | 30%, 53%, 18% > 38%, 45%, 17% | | 36%, 43%, 21% > 43%, 35%, 22% |
| out of it: time to throw, EPA a dropback | 3.56 s, -0.049 > 3.78 s, -0.013 | 4.18 s, +0.016 | 3.87 s, -0.278 > 4.07 s, -0.248 | | 4.06 s, -0.389 > 4.27 s, -0.335 |
| time to throw | 2.47 > 2.49 | 2.71 | 2.51 > 2.53 | | 2.58 > 2.57 |
| thrown after 3.94 s | 2.5% > 2.7% | 10.0% | 4.1% > 4.2% | | 6.2% > 5.9% |
| scrambles a team game | 1.31 > 0.91 | 1.97 | 1.31 > 0.95 | | 0.71 > 0.53 |
| throwaways, of throws | 1.2% > 0.7% | 4.1% | 1.6% > 1.4% | | 1.7% > 1.4% |
| points a team game | 26.03 > 26.15 | 22.51 | 25.27 > 25.20 | 26.97 | 20.65 > 21.04 |
| yards an attempt | 7.51 > 7.58 | 7.05 | 7.72 > 7.68 | 7.29 | 7.22 > 7.26 |
| EPA a play | 0.069 > 0.061 | -0.004 | 0.073 > 0.068 | | 0.076 > 0.086 |
| fumbles lost a team game | 0.357 > 0.384 | 0.447 | 0.429 > 0.401 | | 0.279 > 0.307 |

Why it moved: blind-side rushers who beat their blocks get home, so NFL sacks rise by a fifth and sacks a pressure come near real; the strip sacks that come with them lift lost fumbles toward real at the NFL (engine gaps item 6). With the feel alone (before his hot) NFL sacks were at real, 2.41 a team game, but 31% of them came from the man the protection left free and Cover 0's went back to 15.8% of its dropbacks; knowing his hot he throws hot or slides away from him, as real passers do, and Cover 0's sacks fall back to about lane EG6's (outside the 20 11.0%, main 8.9%, real 7.6%; inside the 5 11.5%, main 10.2%, real 4.2%), a little above main with the rest of the sacks. FBS sacks come to 1.92 a team game against cfbfastR's 2.00. Scrambles fall at every level: on main a third of them came from passers fleeing a free man they found late (with his hot known the NFL's fell from 1.24 to 0.91), and real scrambles come from coverage that holds and pockets that cave, which the engine has too few of (below). A passer who breaks out now has his receivers on the drill and the drill's time, so out of the pocket he throws more and later and runs less, with EPA a dropback out of it near real at the NFL, but fewer passers get out than before, so the targets that ride on it stay short: the share out of the pocket, throws after 3.94 s and time to throw. Every total stays inside its test band. Throwaways fall further under real: a passer who doesn't feel the man coming doesn't throw it away to beat him either.

The coaches' call table (`call_values 2000`): its pass cells fall 0.10 yards a snap against four rushers (the mean of |change| 0.17) and move 0.42 a cell against blitzes (Cover 1's linebacker blitz +0.20 on average, as the passer reads his hot, Cover 3's -0.20); its run cells don't move. The roster table (`call_values roster 100000`): pass blocking pays more, about twice main's at tackle and guard (the first tackle's Pass Blocking 0.32 a point summed over the cells, main 0.13), as a beaten blocker's man now gets home; 3,275 weights (main 3,044). The analysts' four models are refitted on the rebuilt tables: completion (`xcomp_fit 600`), rush and catch yards (`xyards_fit 600`), the pass rate (`xpass_fit 600 7`) and the fourth-down, try and win expectancy tables (`decide_fit 1500 1`).

`fit_check` on the quarterback's reads and the pass rush (`run nfl 2400 161` and `run fbs 3000 151`, 14 arms, the audit's typical teams; main > after on the final engine, * past the noise):

- **The typical team.** NFL sack rate 4.72% > 5.51%, scrambles 0.91 > 0.56 a game; FBS 4.11% > 4.46% and 1.10 > 0.76.
- **The passer's mind reads his hot.** Football IQ +15 takes 0.72* > 0.87* points off the NFL sack rate and 0.58* > 0.90* at FBS (pressure -1.4* > -1.5* and -1.5* > -2.3*); Decisions +15 0.53* > 0.70* and 0.36* > 0.51*. Anticipation and Eye Manipulation keep their sizes (Eye Manipulation's picks -0.00 > -0.13* at the NFL, its FBS points +0.76* > +0.72*).
- **Pocket Presence pays less.** +15 takes 0.53* > 0.36* points off the NFL sack rate (FBS 0.51* > 0.38*), and its points fall from +0.34 to +0.06 a game: it reads the feel for a rusher he can't see, and on main it also found his hot, which his Football IQ and Play Recognition read now (`GAPS.md`).
- **Throw on the Move** pays nothing past the noise either way (NFL points -0.04 > +0.04), with few passers out of the pocket. **Speed** +15 adds scrambles (+0.28* > +0.34* a game at the NFL, +0.85* > +0.51* at FBS) and costs points as on main (NFL +0.15 > -0.24, FBS -0.36 > -0.20, both inside the noise).
- **The rush gets home.** The first edge's sacks a season 3.21 > 4.24 on fewer pressures (12.0 > 10.5), the first tackle's 4.21 > 5.21; the edge's Finesse Rush +15 adds 1.26* > 1.77* sacks (FBS 1.57* > 1.90*), the tackle's 2.25* > 2.32* (FBS 0.74* > 1.04*), Power Rush still nothing.
- **Blitzing pays.** The Mike's Blitzing +15 takes 0.005 > 0.022* EPA a dropback off his defense at the NFL (FBS 0.002 > 0.002), as his timing and disguise beat the passer's read of his hot; the strong safety's from +0.013* (worse) to -0.016*.

Tried and left out, each on NFL seed 1:

- **The escape from a caving pocket.** Three triggers (a winning rusher's pressure over 0.02 to 0.5, a free rusher he knows of within 0.8 to 1.2 s of him, any pressure over 0.3), at 0.15 to 0.6 a tenth of a second, away from the rush and at full speed with a rusher within 3 yards. Up to 4.3 escapes a team game took the share out of the pocket to 14%, but the passers who escaped were sacked on 20% to 34% of those dropbacks (real 15% of pressured passers out of the pocket), and in-pocket sacks fell to 3.4%. A rusher bearing down catches a passer who starts from a standstill; real escapes come from a step-up or a spin against the first man, which the sack check reads only at the tackle (`GAPS.md`).
- **Breaking out after his first read.** 15% of dropbacks out of the pocket, but the throws out there went short and easy: 73.5% complete at 6.3 air yards (real 61% at 9.5).
- **Eyes downfield on the move** (no throw under 5 or 8 yards for 0.6 to 1.0 s, and a wider window on the move): throws on the move went deeper than real, 13.6 to 14.9 air yards against 9.5, and no later.
- **The run held back on the move**: scrambles fell to 0.9 a team game.

Speed: the same inside the noise. `match_check` against main, the two run in turn on seeds 1 and 2 and the order swapped between the seeds, the tool's own time on an otherwise quiet machine (4 cores, the same build settings): the NFL 2.1% faster (the two pairs 6.6% faster and 2.8% slower; 2,000 games 21.2 and 21.8 s against 22.7 and 21.2 s on main), FBS 3.7% slower (8.3% and 0.4% faster) and high school 2.0% slower (4.0% and 0.0%). His hot is read once a dropback; the feel, the drill and plaster add a check a tick.

What is left is in `GAPS.md`: few passers out of the pocket (the escape, the boot's share) and few scrambles, with throws after 3.94 s and time to throw short (a passer still out inside 2.5 s on 68% of throws, real 50%, as receivers come open early, which also keeps NFL sacks a fifth short), throwaways, Cover 0's sacks, and the passer's throws on the move still easier than real.

## Debug tools
- `cargo run --release --bin tendency_check -- nfl|fbs|fcs|small|hs [games per arm] [seed] [spread|ends|fit|orders|levels|all|drift|centers] [keys]`: the tendency proofs; `drift` shows which tendency moves the league average (a comma list of keys runs one arm with those at the spread).
- `cargo run --release --bin match_check -- nfl|fbs|hs|cfl|arena|nfle|xfl2001|xfl|aaf|usfl1983|usfl|ufl2009|ufl [games] [seed] [pbp]`: averages against real data (the other leagues' books against their own real averages; `usfl` is the 2022 USFL); `pbp` prints the first game play by play with its whys. `AFM_ROSTERS=file.json` plays a trial's exported rosters (`python3 tools/rosters/export_rosters.py <raw dir> <out.json> [division]`, a college division (FBS by default) or NFL; the raw files must be unzipped), `AFM_DUMP=file.json` writes the synthetic teams, `AFM_AB_WORLD=<world file>` plays a world's 32 NFL clubs (NFL only), `AFM_RUNDIAG=1` adds the run diagnostic (lane SF3: who starts the tackle by his block at the hit, the line's edge against yards before contact, the backs' lasting spread), `AFM_GAPS=1` the engine gaps audit's counts (lane EG1: injuries by position, cause and body part against nflverse, the sideline and concussion checks; lane EG2: fumbles by how the ball came out, by nflverse's kinds with who came up with them, and each kind's rate by position) and the rule books and flags (lane EG5: each foul kind flagged, accepted, declined, its yards and first downs against real; safeties on the offense's foul; pass interference by air yards; ineligible downfield by call and where the lineman stood; kickoffs; free plays; throwaways and grounding; pre-snap fouls a snap by down; delays on fourth down; fouls before a kick's snap; the hard count; the defense's hit fouls by where), and `AFM_RECDIAG=1` the receivers' (lane SF4: wide receivers', tight ends' and backs' contested share, contested catch rate and drops against real, their true spreads between men, an inch of height against contested balls, separation's spread and its slope on Speed, drops' slope on Catching with the spread Catching alone gives them, and the forty's slopes of aDOT and of targets 20+ yards downfield).
- `cargo run --release --bin dynasty_check -- value [games] [seed]`: what each position and the play callers are worth (`AFM_LEVEL`, `AFM_ONLY`). `cargo run --release --bin call_values -- 400`: rebuilds the coaches' what-beats-what table (`AFM_CELL=concept:Coverage` shows one cell's outcomes).
- `cargo run --release --bin fit_check -- run <fbs|nfl|hs> [games] [seed] [out] [arms]`: the scheme-fit check (`docs/design/scheme-fit.md`, lane SF0): does a man's skill pay inside his team's scheme? Team 0 is the level's typical team; each arm changes one thing on it (a rating, a body number, a drawn man, the scheme) and replays the same games as its context's typical arm. `fit_check read <dir>` writes the tables (each job's measures, the items' measures against real, as recruited, personnel, where men lined up, ratings nothing reads, the world's fit weights against what +15 moved); `fit_check repeat` compares runs on the claims behind the items.
- `cargo run --release --bin fire_check -- hits <nfl|fbs|fcs|hs> [games] [seed]`, `talks <level> [seed]`, `run [games] [seed] [stakes]`: coach fire ("Coach fire" above): the big hit's line and the fire counts a team game; the AI coaches' pregame picks against main's; each prototype room's landing on a Branch-type safety and a calm pocket passer, then the two men under each room, A/B on the same seeds against their coach's own read (stakes 0.8 for playoff games).
- `cargo run --release --bin garbage_check -- fbs|nfl|hs [games] [quality sd] [seed]`: garbage time against real play-by-play (who plays the 4th quarter by lead, run-out, 4th downs, sample why lines).
- `cargo run --release --bin leagues_check -- <start year> <history years> [scale] [seed]`: the other leagues in a small world (college on the stand-in): every season's table, scoring against the real league, champions, rosters and Canadians, the CFL draft, the best passing lines and the coaches' seats.
- `cargo run --release --bin play_trace -- <run|pass|N> [seed] [sweep]`: one snap traced tick by tick, or a sweep of seeds. `AFM_LEVEL=fbs|hs`, `AFM_COV=0-9`, `AFM_LOS=yards` and `AFM_BASE=1` set the level, coverage, spot and personnel; `AFM_PRESSURE=0-5` the pressure, `AFM_BF=uc|pistol|gun` the backfield and `AFM_FAKE=1` a run fake before the throw (scheme-fit item 3). The one-snap mode prints every agent; the sweep prints one line a seed (`AFM_TRACE=1` is `match_check`'s switch for the agent trace).
