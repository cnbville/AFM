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
- **Home field (trial 4 topic 3).** Crowd noise from the stadium, night games, rivalries and stakes; the road offense works on a silent count, misses checks and false-starts at the real road ratio; travel, time zones, the body clock and altitude (`data/real-world/college_altitude.csv`) tire the visitors; warm-climate teams handle cold worse. FBS home margin 2.5 points (real 2.4); NFL 0.9 (real 2.1, finding).
- **College calling (trial 4 topic 5).** Pass rate and fourth-down tables from real FBS play-by-play (cfbfastR 2023-24, `data/real-world/scripts/college_situations.py`).
- **S8-S10.** Each DC's own nickel habit; one-gap and two-gap fronts fit runs and the rush differently; no-huddle snaps stop the defense substituting, and breath comes back with time between snaps.
- **Position spots (fix list topic 3).** The engine plays each man at his listed spot: tackles by side, guards and center, nose and three-technique, middle and outside linebackers, nickel, free and strong safety, fullback (`flow.rs` `lineup`, `spots.rs`). A man at a spot he has never been listed at plays below his rating there; a man from another position group is an emergency (`PlayRecord.emergency`). High school players still go both ways.

Measured after steps 2-4 (`match_check`, 600 games): FBS 28.8 points (real FBS 27.0), completion 61.1% (real 60.6%), 7.0 yards an attempt (real 7.3), games within 8 points 38% (NFL real 54%); NFL 32.0 points (real 22.5), completion 69% (real 64.5%).

- **Plays and tactics (step 5).** Sets, motion, fronts with techniques, stunts, fire zones, robber and match coverages, RPOs, the option pitch, missed run fits; motion and disguise before the snap; knowledge per concept family and line chemistry; film, the matchup plan, tendencies and self-scouting; four calling skills; the play designer (`game/designer.rs`). The checklist is `docs/design/build7-checklist.md`, step 5.
- **The install (calendar T6.2).** `GameTeam::plan()`: the share of the book installed this week, from the coordinator's install (what his players absorb, `roster::absorb`) times the week (`roster::prep_week`: a Thursday game 0.65, a normal week 1.0, a bye 1.15). A smaller plan calls fewer concepts; calls past the install add busts (`Tactics.load`).
- **Calibration (step 5j, each proposed, not confirmed).** Kickoffs: the kicker kicks from between the hashes and trails, the safeties stay behind the wave (return touchdowns 0.11 → 0.02 a team game, real 0.012). Tackling in space: one open-field lunge of 2.5 yards (`sim::OPEN_REACH`). Drops: quick 1.25 s, reads a tenth after the top of the drop (throws under 1.6 s 21% → 6%, real 10%). Quick game is one picture: the QB keys the flat defender and throws the open one of the first two routes (stick against Cover 3: 48% → 87% complete). Man coverage lags 0.12 s at the break.

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
- **Officials who make mistakes (G2).** Each play can be a close call (a catch at the sideline, a fumble or down by contact, the ball across the goal line, the spot against the line to gain, an interception or a trapped ball). A crew of average eye gets 16% of close calls wrong (`WRONG_SHARE`); each crew's eye is drawn per game (0.7 sharp to 1.4 poor) on the officials' own stream, so no other draw moves. A wrong ruling stands unless replay sees it and the video is clear (78%, `EVIDENCE`). Replay by level and era (`Replay::of`): NFL coach's challenges from 1999 (two, a third from 2004 when both win; a lost one costs a timeout), the booth on scoring plays (2011) and turnovers (2012) and inside two minutes, replay assist from 2023; college booth reviews of any play from 2006 and one coach's challenge; high school none. AI coaches challenge by their replay assistant's read and the stakes. The replay assistant reads a wrong ruling right 55% to 92% of the time by the head coach's game management, and says "throw it" on a right one about a third as often as he misses. Measured over 1,000 NFL games (`match_check nfl 1000 7`): 0.89 wrong rulings a game (target about 0.8), 0.38 coach's challenges a game won 38% (real 2024: 0.47 a game, 50 of 133 won), 0.85 booth reviews a game with 56% reversed (real about 0.7, a little over half), 0.28 wrong rulings left standing; most close calls are catches, then goal-line rulings, spots, fumbles and interceptions. The challenges run a little rare and the booth a little busy: proposed, not confirmed. Each close call is kept on its play record (`Officiating`) for the game page.
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

- **The drive first, then the rest of the game**, the way the public models do (Pro Football Reference's, nflfastR's). The drive's touchdown and field goal chances come from the spot, scaled so the drive is worth the down, distance and spot's real expected points (read between the table's 5-yard rows), and shrink when the half's clock is too short to get there: about six yards and six seconds a snap in a hurry, plus about sixteen seconds of running clock unless a timeout stops it. Late, a side short of time kicks when it reaches range, but not on fourth down outside it. A drive that stalls near the edge of range is part punt, part missed kick. Each ending takes its own time: a scoring drive about eight yards a snap, a stall about four and a half snaps; a side in a hurry scores as soon as the short clock lets it, and a side level or ahead late in the fourth runs its four downs first, at about 36 seconds a snap less 30 for each timeout the other side spends. The drive's split has small kinks where kicks give way to punts, so a spot is never read as worth less than one up to 25 yards further back. A field goal's make comes from the distance by level (the NFL's and FBS's from their real 2021-25 kicks). After the drive, the final margin is about normal around the lead plus the pregame line spread over the time left, with a spread that shrinks with the square root of the time left (Stern 1991). A leader with the ball who can kneel out the clock against the other side's timeouts has won. Overtime plays by its level's rule (the NFL's timed period, college's possessions from the 25, high school's from the 10).
- **Per level**: the quarter's length, the two-minute warning, the play clock, field goal makes, the try's worth, where a kickoff starts and the margin's spread (13.5 points over a game in the NFL, 16 FBS, 16.5 FCS and small colleges, 18 high school), fitted to each level's games on the engine.
- **The line.** The pregame spread is the closing line where the books hung one, else the home field (1.7 points in the NFL, 2.5 elsewhere).
- **Checked** (`tests/win_chance.rs`): sane at known states at every level (the opening snap near even, a 7-point favourite well ahead, a kneel-out won, down 14 with two minutes left nearly lost, up 10 at the half well ahead), never falling as the lead grows and never by more than one percentage point as the ball moves closer (the expected points table has small kinks), and calibrated on 30 engine games each in the NFL, FBS and high school: a Brier score of 0.110 in the NFL over 5,107 snaps, 0.178 in FBS over 5,314 and 0.102 in high school over 4,084, and in each fifth of the range the side won within 0.15 of the chance said.

## Debug tools
- `cargo run --release --bin tendency_check -- nfl|fbs|fcs|small|hs [games per arm] [seed] [spread|ends|fit|orders|levels|all|drift|centers] [keys]`: the tendency proofs; `drift` shows which tendency moves the league average (a comma list of keys runs one arm with those at the spread).
- `cargo run --release --bin match_check -- nfl|fbs|hs|cfl|arena|nfle|xfl2001|xfl|aaf|usfl1983|usfl2022|ufl2009|ufl [games] [seed] [pbp]`: averages against real data (the other leagues' books against their own real averages); `pbp` prints the first game play by play with its whys. `AFM_ROSTERS=file.json` plays a trial's exported rosters (`python3 tools/rosters/export_rosters.py <raw dir> <out.json> [division]`, college only; the raw files must be unzipped), `AFM_DUMP=file.json` writes the synthetic teams.
- `cargo run --release --bin dynasty_check -- value [games] [seed]`: what each position and the play callers are worth (`AFM_LEVEL`, `AFM_ONLY`). `cargo run --release --bin call_values -- 400`: rebuilds the coaches' what-beats-what table (`AFM_CELL=concept:Coverage` shows one cell's outcomes).
- `cargo run --release --bin garbage_check -- fbs|nfl|hs [games] [quality sd] [seed]`: garbage time against real play-by-play (who plays the 4th quarter by lead, run-out, 4th downs, sample why lines).
- `cargo run --release --bin leagues_check -- <start year> <history years> [scale] [seed]`: the other leagues in a small world (college on the stand-in): every season's table, scoring against the real league, champions, rosters and Canadians, the CFL draft, the best passing lines and the coaches' seats.
- `cargo run --release --bin play_trace -- <run|pass|N> [seed] [sweep]`: one snap traced tick by tick, or a sweep of seeds. `AFM_LEVEL=fbs|hs`, `AFM_COV=0-9`, `AFM_LOS=yards` and `AFM_BASE=1` set the level, coverage, spot and personnel. `AFM_TRACE=1` prints every agent.
