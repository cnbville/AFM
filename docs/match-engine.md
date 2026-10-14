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

## Calibration (development only)

The checks compare synthetic-league games to real 2021-25 play-by-play. They never feed back into the game. Run `cargo run --release --bin match_check -- nfl 400` (or `fbs`, `hs`). The automatic version is `crates/afm-tools/tests/match_engine.rs`, with wide bands.

Latest numbers (400 games each, 2026-10-14):

| Per team game | NFL engine | NFL real | FBS engine | FBS real | HS engine |
|---|---|---|---|---|---|
| Points | 27.6 | 22.5 | 31.5 | 27.0 | 32.0 |
| Completion % | 66.2 | 64.5 | 56.7 | 60.6 | 47.2 |
| Yards per attempt | 7.4 | 7.05 | 7.3 | 7.3 | 6.1 |
| Yards per carry | 3.9 | 4.5 | 5.1 | 5.1 | 6.0 |
| Third-down conversion | 39.7% | 39.8% | 37.1% | 40.3% | 33.3% |
| Sack rate | 4.1% | 6.8% | 2.3% | 6.1% | 2.8% |
| Interception rate | 1.5% | 2.3% | 1.4% | 2.6% | 1.3% |
| Plays | 70 | 62 | 76 | 67 | 83 |

There is no high school play-by-play dataset yet, so the HS column has no real figure next to it.

### Known gaps (trial candidates, not fixed on purpose)
- **Points and plays run high** at every level. NFL teams pass too often (66% against 58%), and college teams pass like the NFL (64% against 50%) because calling starts from NFL tables at every level.
- **Sacks and interceptions are low** at every level.
- **NFL runs are short** (3.9 against 4.5): too many runs for no gain, too few of 10-19 yards, a few too many of 40+.
- **Red zone**: 65% touchdowns against 56%. Slants and smash near the goal line score too often against man coverage.
- **Deep throws** are a bit rare at the NFL level (9% of throws travel 20+ yards against 11.5%).
- **Kickoffs**: NFL returns average 18 against 26.5. College and high school returns average about 10, because the synthetic kickers there are weak and their kicks land short. The real kickoff data predates the NFL's 2025 rule.
- **High school plays** (83 a team) are far above real high school games (12-minute quarters).
- **FBS completion** is about 4 points under the real figure.

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

## Not built yet
- Running every game in the world through the engine (the world still uses its own season results), and the season and career record across levels with percentiles and era adjustment.
- Coach buzz from stats, hidden-gem stat flags, awards and records. These come with the world trial 3 list and later sections.
- The 2D replay view (section 10).

## Debug tools
- `cargo run --release --bin match_check -- nfl|fbs|hs [games] [seed] [pbp]`: averages against real data; `pbp` prints the first game play by play with its whys.
- `cargo run --release --bin play_trace -- <run|pass|N> [seed] [sweep]`: one snap traced tick by tick, or a sweep of seeds. `AFM_LEVEL=fbs|hs`, `AFM_COV=0-7`, `AFM_LOS=yards` and `AFM_BASE=1` set the level, coverage, spot and personnel. `AFM_TRACE=1` prints every agent.
