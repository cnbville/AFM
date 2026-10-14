# Match engine physics fixes (after world trial 3)

Status: the next needed fix, not built yet. Raichel (2026-10-14): don't build it now, keep it in git as the next needed fix. The five fixes below are agreed by Raichel (2026-10-14, World Trial 3 thread). Targets and checks marked "proposed, not confirmed" are Claude's and still need Raichel's word. When Raichel says to build it, a fresh build thread builds this, merges to `main` and sends one reply. Then world trial 4 runs on it.

## Why
World trial 3 (`docs/trials/world-trial3.md`) gave Tennessee 9 titles in 15 years. Raichel asked whether the cause is the physics of the games. The trace says yes.

What decides FBS games in the world today (points of margin per rating point, all 10,644 FBS games of trial 3):

| Group | Points per rating point |
|---|---|
| Rotation players (backups who get snaps) | 1.33 |
| Receivers | 0.67 |
| Defensive line | 0.56 |
| Secondary | 0.56 |
| Offensive line | 0.48 |
| Linebackers | 0.32 |
| RB | 0.27 |
| QB | 0.20 |
| TE | 0.11 |
| Coordinators' play calling, kicker | about 0 |

The controlled test (`cargo run --release --bin dynasty_check -- 1000`, two equal teams, the same total boost put in one place) agrees: QB +10 adds about 0.2 points, the offensive line about 0.2, receivers 2-3, linebackers about 2. One RB +10 swings the game -7.5 or +6.8 depending on the run. Play callers 18 against 4 change nothing.

Tennessee's rotation players were 9.9 points above the FBS average, worth about 13 points a game. Team strength ignores depth, so Tennessee beat its rating again and again (36 of 43 close games won, 25 expected).

## The five fixes

### 1. QB play drives passing
Accuracy (short, medium, deep), decisions, reading the defense, pocket feel, release and arm should decide completions, interceptions, sacks and big plays. A great QB should be the single most valuable player on the field, as in real football.

### 2. Line play decides runs and pressure
Each block is a battle between a blocker and a defender (strength, technique, leverage, footwork, hands, awareness against the rusher's moves). Who wins those battles decides run lanes, yards before contact, pressure, sacks and how long the QB has. One weak lineman should be hunted and should hurt. Pressure should change the QB's decisions, not just end plays.

### 3. Depth and rotation
Depth matters, but never more than the starters. Rotation follows real football: defensive linemen rotate a lot (starters play about 60-75% of snaps), edge rushers somewhat, running backs share carries (a lead back takes about 55-65%), offensive linemen, QB, safeties and corners rarely rotate. Fatigue and the game state (blowouts, injuries) decide when backups come in. Find and fix why backups count more than starters today (too many snaps, starters tiring too fast, or both).

### 4. The RB bug
One running back changes a game by about 7 points either way. Trace it and fix it. A better back should help through vision, cutting, breaking tackles and catching, inside what his line gives him.

### 5. Coaching, by Raichel's rule
Raichel (2026-10-14): the play call itself wins or loses through the matchups. Running into a stacked box (+2 defenders) loses because of the numbers, whoever called it, the user or the coach. Coach quality decides the calls when the user doesn't pick, plus in-game adjustments, game prep and how well the scheme is installed.

So:
- No flat bonus for a good coach. A good call wins through the battles (box count, leverage, numbers to the side of the play, coverage against the route, a matchup hunted). A bad call loses the same way.
- The coordinator's skill decides how often he picks the right call for the situation and the defense he expects, how fast he adjusts during the game, and how well his game plan fits the opponent.
- Game prep and scheme install show up as players doing their job right (fewer busts, quicker reads), not as a bonus.
- This must fit the plays and tactics design (separate thread, `afm-playbook-design` in memory): geometry and battles decide, never a counter table.

## Checks before the next trial
Proposed, not confirmed:
- **Position values against real football.** Extend `dynasty_check` to measure every position's points per rating point, then compare the order with real-world value (research it: wins-above-replacement studies, EPA by position, NFL contracts and draft value). QB first by a clear gap; the lines and pass rush near the top; RB and TE near the bottom. Put the sources and the bands in `data/real-world/`.
- **Depth.** Rotation players worth clearly less than starters per rating point. Snap shares by position inside real bands (above), from the play log.
- **Play calling.** Caller 18 against 4 moves the margin clearly off zero, inside a band from real play-caller data (research it). A good plan against a bad plan with the same players does the same.
- **Box count.** Runs into a +2 box average clearly fewer yards than runs into a light box, inside real splits (research them).
- **Everything that already matches real data still does:** `match_check` for NFL, FBS and HS, and `cargo test --workspace --release`.
- Then world trial 4 runs on the full world (15 years, every college game on the engine) and checks that dynasties and droughts look real.

## Not in this spec
These trial 3 findings wait for Raichel's calls: college play calling tables, sack and interception rates, kick returns, home-field edge, kickers playing pass rusher, specialist and QB athleticism, the pick-1 QB share, three-star draft rates, the coach-of-color pipeline, and names. A build thread may fix a bug it meets on the way, but should list it in its reply.
