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
| under 250 lb, top 10% / all | 10.6% / 7.7% | 12.6% / 7.8% | 17.0% / 6.4% | 11.4% / 6.1% | 18% / 19% |
| 290 lb or more, top 10% / all | 5.8% / 10.6% | 10.7% / 10.7% | 0% / 17.0% | 1.0% / 17.2% | |
| arms under 33 in, top 10% / all | 67.3% / 67.4% | 68.0% / 66.9% | 81.1% / 68.1% | 81.9% / 68.4% | 32% / 24% |
| 6-5 or taller, top 10% / all | 10.6% / 18.7% | 10.7% / 18.8% | 12.3% / 20.6% | 5.7% / 20.9% | 21% / 29% |
| best sack season | 21 | 27 | 20 | 31 | 22.5 and 23 |
| sacks a season, top 1% / 5% | 16 / 11 | 22 / 16 | 17 / 12 | 21 / 15 | |
| the freak's median rank in pressures (best) | top 6.5% (0.2%) | top 3.0% (0.2%) | top 22% (5.6%) | top 33% (1.6%) | with the best |
| the freak's sacks a season | 6-18 | 12-23 | 6-15 | 5-20 | |
| sacks a pressure, all edges | 22.2% | 27.6% | 22.2% | 27.8% | 27.3% |

Light and short-armed edges hold at least their share of the top seasons, and nothing caps the freak: on seed 82 he is a top 3% rusher most years with 12-23 sacks, more than on main. The seed 83 freak has lower Acceleration, Flexibility and Speed (88, 79, 80); his pressures rank lower than on main, his sacks higher. One man's rank swings a lot from season to season (top 2% to top 60% for the same ratings), so a median over ten seasons moves about 10 points from noise. The synthetic league's bodies come from the combine, so its shares of light (7%) and short-armed (67%) edges differ from real regulars'; the check is the top share against the league's own.

2,000 `match_check` games a seed, NFL averaged over seeds 82-85 (main dd9ac18 and this lane merged with it; real NFL from nflverse 2021-25, PFR for pressure; FBS from cfbfastR 2022-24):

| | main | after | real |
|---|---|---|---|
| NFL sacks a team game | 1.90 | 2.39 | 2.41 |
| NFL sack rate a dropback | 5.2% | 6.6% | 6.8% |
| NFL pressure rate (PFR's count) | 23.3% | 23.8% | 22.2% |
| NFL sacks a pressure | 0.216 | 0.265 | 0.283 |
| NFL time to throw | 2.48 s | 2.46 s | 2.71 s |
| NFL points a team game / sd | 23.6 / 10.71 | 23.1 / 10.83 | 22.5 / 9.9 |
| NFL completion % / yards an attempt | 66.4% / 7.32 | 66.4% / 7.34 | 64.5% / 7.05 |
| NFL interceptions a team game | 0.73 | 0.72 | 0.76 |
| NFL scramble rate | 4.6% | 4.3% | 4.9% |
| FBS sacks a team game (seed 82) | 2.11 | 2.20 | 2.00 |
| FBS points a team game | 22.9 | 22.8 | 27.0 |
| High school sacks / sack rate / points (seed 82) | 2.39 / 10.2% / 17.5 | 2.48 / 10.7% / 17.4 | |

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
- **The best sack seasons reach 27-31** (real 22.5 and 23; the top 1% of seasons 21-22 sacks), most of them heavy power rushers. Sacks a team game at the NFL sit at real, so the tail, not the level, is long. The synthetic league plays fixed depth charts, with no rotation by body.
- **Tall edges are under their share of the top seasons** (6-11% of the top, 19-21% of all; real 21% of 29%; main 11-12%).
- **As recruited, a long-armed edge's pressures end in a sack too often** (+2.2 pts of sacks a pressure an inch, real raw +0.66), though pure arms are at real (+0.68, real +0.65).
- **FBS sacks rose to 2.20 a team game** (main 2.11, real 2.00): the finish reads FBS edges' Tackling and Pursuit, about 54, a little over 60 for the best.

## Debug tools
- `cargo run --release --bin tendency_check -- nfl|fbs|fcs|small|hs [games per arm] [seed] [spread|ends|fit|orders|levels|all|drift|centers] [keys]`: the tendency proofs; `drift` shows which tendency moves the league average (a comma list of keys runs one arm with those at the spread).
- `cargo run --release --bin match_check -- nfl|fbs|hs|cfl|arena|nfle|xfl2001|xfl|aaf|usfl1983|usfl2022|ufl2009|ufl [games] [seed] [pbp]`: averages against real data (the other leagues' books against their own real averages); `pbp` prints the first game play by play with its whys. `AFM_ROSTERS=file.json` plays a trial's exported rosters (`python3 tools/rosters/export_rosters.py <raw dir> <out.json> [division]`, college only; the raw files must be unzipped), `AFM_DUMP=file.json` writes the synthetic teams.
- `cargo run --release --bin dynasty_check -- value [games] [seed]`: what each position and the play callers are worth (`AFM_LEVEL`, `AFM_ONLY`). `cargo run --release --bin call_values -- 400`: rebuilds the coaches' what-beats-what table (`AFM_CELL=concept:Coverage` shows one cell's outcomes).
- `cargo run --release --bin garbage_check -- fbs|nfl|hs [games] [quality sd] [seed]`: garbage time against real play-by-play (who plays the 4th quarter by lead, run-out, 4th downs, sample why lines).
- `cargo run --release --bin leagues_check -- <start year> <history years> [scale] [seed]`: the other leagues in a small world (college on the stand-in): every season's table, scoring against the real league, champions, rosters and Canadians, the CFL draft, the best passing lines and the coaches' seats.
- `cargo run --release --bin play_trace -- <run|pass|N> [seed] [sweep]`: one snap traced tick by tick, or a sweep of seeds. `AFM_LEVEL=fbs|hs`, `AFM_COV=0-9`, `AFM_LOS=yards` and `AFM_BASE=1` set the level, coverage, spot and personnel. `AFM_TRACE=1` prints every agent.
