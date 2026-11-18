# Engine gaps beyond scheme fit (spec)

On 2026-11-17 Cain asked what other parts of the match engine need work beyond the update already running (scheme fit): "what other engine parts might need to get improved other then the update we already have running?" The engine gaps audit measured the engine against real NFL play-by-play and the repo's college targets and found 30 gaps. Its report is `afm/engine-gaps/report.md` in the project folder, with the tools in `tools/` and the raw numbers in `raw/` beside it. Where the report and this file differ, this file is right (it was checked once more before it merged).

Cain agreed to all 30 and to the lanes the same evening (22:09 UTC, "agreed"). The question was: "Should all 30 go on the engine fix list as lanes EG1-EG7? Say 'yes to all', or the numbers to leave out. My pick: yes to all, with EG1-EG5 beside the scheme-fit chain and EG6 and EG7 after their SF lanes." This file says what gets built and in which lane. EG5 is built (2026-11-18; its section below says how); the rest is not.

Cain's engine rule (2026-11-14: nothing in the engine before the next world builds are done, `docs/design/engine-fix-list.md`) is met: the seed 181 world was built on 2026-11-17. So EG1-EG5 can start now.

The choices left open after Cain's answer were made overnight under his proxy (2026-11-18, "you have full proxy and are expected to push forward, when given a chose make the most realistic choice"). Each is marked "Claude's call under Cain's proxy" where it appears, and all of them are listed at the end ("Decided under Cain's proxy").

## How it was measured

- **Engine.** `match_check` on main bf8c865: NFL and FBS 2,000 games each, high school 1,000, and the NFL again in rain (`AFM_RAIN=1`), in a 17 mph wind and with no wind (`AFM_WIND`; the default is 5 mph). The audit's own counts are `AFM_GAPS=1 match_check` (`gaps_counts`, committed with this spec): points by quarter and late in each half, margins, injuries by position, field goals by distance with blocks, punts and kickoffs with returns, tries, safeties, fumbles by kind, the RPO share, quarterback runs, pass interference flagged and accepted, penalties by kind and the calls on a half's last snaps. It counts exactly as the audit's scratch block did. Its fumbles are scrimmage plays only: the engine plays muffs and fumbled returns but never logs a kick play's fumble. The audit ran before scheme-fit lane SF3 merged, so each lane takes its before numbers from its own main.
- **Real.** nflverse play-by-play, 2021-25 regular seasons (1,359 games, 2,718 team games), read with the scripts in the report's `tools/`. College numbers come from the repo's `data/real-world/college_game_targets.json` (cfbfastR). There is no high school play-by-play; the rule books by level come from the NFL, NCAA and NFHS rules and their yearly changes.
- **Code.** Each code claim was checked twice against the code, then once more with the numbers and the rule books before this file merged. The how-it-works pages were corrected where they said less (PR #541).
- **One limit.** The test leagues are synthetic, with typical players and typical staffs. Where a gap could come from the test league's rosters, the item says so.

All numbers are per team game unless named. "Engine" is the NFL test league unless named.

## The items, as agreed

Cain agreed to all 30 at once. Each item's target is the real number. How the engine gets there is the lane's job, from how real football works: root causes, no caps, no flat boosts, no fudge on a total. Any mechanic or size below that isn't a real number or a rule is Claude's, proposed, not confirmed. A lane that meets a real choice (not a realism question) brings Cain a concrete proposal; while he is asleep it takes the most realistic option itself, builds it and lists it in its final reply (Cain, 2026-11-18).

### EG1: Injuries

**1. Injuries for every collision, not just the ball carrier.** (high)
- **Target.** Injuries by position like real. Offensive linemen get hurt about as often per man as backs (nflverse injury reports, episodes a player-season: OL 0.43, RB 0.44, WR 0.47, CB 0.51). On a 53-man roster that puts linemen near 18% of injuries and backs near 8%. The NFL's 1996-2005 surveillance put each line near 20%.
- **The total.** Contact and non-contact together (items 1 and 2 share it), the total stays near today's 1.10 week-plus injuries a team game, which is fair (real about 1.0, `game/sim.rs`, `injury_check`). The mix by body part (`GAME_INJURIES`) stays real too: the strains item 2 adds take their share of it, not a share on top.
- **Today.** An injury is rolled only in a tackle (the ball carrier and each tackler; a man joining the tackle hits with both men's full momentum, and only his big-hit lean and his part in slowing the runner are halved; `game/sim.rs`, `hit_effects` → `injury_check`), on a sacked quarterback (never the sacker; `game/scrimmage.rs`, `sack_check`) and on a flagged late hit on a slide (`late_on_slide`). It is never rolled in a block, at the catch point (targeting included), on a quarterback as he throws (roughing included), on the late hitter, on a tackle that forces a fumble (`hit_effects` returns first; a strip sack still rolls the quarterback's) or on an arm tackle the runner breaks (`first_hit`). So backs take 35% of game injuries, receivers 16% and offensive linemen 0.2%.
- **The fix.** Every collision can hurt both men, by its force: the block (pile-ups, a man rolled up from behind), the hit at the catch point, the hit on a throwing quarterback, the tackler, the sacker and the late hitter.
- **Example.** In a 17-game season an engine team's backs miss time from about 6.6 game injuries and its offensive line from almost none (0.05). At the real shares (backs 8%, linemen 18%) of today's total (18.6 a season), the backs would have about 1.5 and the line about 3.4.

**2. Non-contact injuries.** (high)
- **Target.** About 70% of in-game ACL tears in the NFL came without contact to the knee (2014-19, 140 tears on video); offensive linemen are the exception, at 70% with contact. Hamstrings, groins and calves are sprinting injuries of receivers, backs and corners. The Achilles is its own injury.
- **Today.** Every game injury is drawn at a hit (`GAME_INJURIES`: hamstrings 7.4, groins 3.4, calves 3.0 and quads 1.5 of 100 come from hits today), and the Achilles is not a game injury kind. Practice injuries are built (`training/week.rs`); this item is the game's own.
- **The fix.** A man can get hurt with nobody touching him when he cuts, plants or sprints, more with tired legs (fatigue is already tracked), sized from real injury studies, inside item 1's total.
- **Example.** A receiver running a go route late in a hot game can pull a hamstring; today he never does.

**3. Back in the same game.** (medium)
- **Target.** Most in-game injuries are short: an ankle taped, a stinger, cramps. The player misses a few snaps or a series and comes back. The NFL ran 538 in-game concussion checks in 2025, and most of those players went back in: those are the men the checks cleared. The lane sizes the return rate from real injury reports.
- **Today.** A hurt man plays on hurt or is out for the rest of the game (`game/flow.rs`, `apply_injuries`). A bruise (0-1 week, 23% of game injuries) sends about half its men out for good at Toughness 10 (`game/sim.rs`, `injury_check`). A concussion is already caught by his level's protocol, 92% of the time in the NFL, 80% in college and 60% in high school, the same in every era: a caught one is out for the game and a missed one plays on hurt (`apply_injuries`).
- **The fix.** A hurt man goes to the sideline, gets checked and comes back after a few snaps, a series, or not at all, by what the injury is and how bad. His backup plays meanwhile. A concussion follows his level's protocol by era: once one is diagnosed he is out for the day (the NFL from 2009, college from 2010, high school from 2010-11, where some states also bar a same-day return after any removal for signs); before those years he could go back in the same game. Today's catch rates are the starting point.
- **With section 9.** Game day's topic 7 (`docs/design/game-day.md`, proposed, not confirmed) sketches this moment for your team. He sits at least one play, and the medical read comes in steps (field, tent, locker room), each with a status and a return estimate. Concussions are the doctors' call by the era's rules (`docs/design/people.md`, "The doctors are independent where the rules say so"), and a player may hide symptoms. Other returns are your call, with the trainer's advice and the player's wish. EG1 builds the engine side so that screen can sit on it; until section 9 is built, the staff's own call decides for every team.
- **Example.** A guard rolls an ankle, misses three snaps and returns.

### EG2: Ball security

Fumbles from scrimmage are 20% short (0.79 a team game, real 0.98), and fumbles lost 17% short (0.37, real 0.447, also from scrimmage). Counting kick plays, real fumbles are 1.14 a team game. The mix is wrong:

| Fumbles a team game | engine | real |
|---|---|---|
| on sacks | 0.085 (4.1% of sacks) | 0.298 (12.4%) |
| after a catch | 0.294 | 0.244 |
| on designed runs | 0.374 (1.4% of carries) | 0.197 (0.75%) |
| fumbled snaps | none | 0.185 (lost 25%) |
| on passes without a catch (a hit as he throws, interception returns) | 0.011, all interception returns | 0.036 (8% lost) |
| scrambles | 0.024 | 0.024 |

**4. Fumbles that stay live.** (high)
- **Target.** Fumble return touchdowns 0.032 a team game (87 in five seasons, 7% of lost fumbles, a third of the defense's touchdowns from scrimmage beside pick-sixes at 0.067). 28% of defensive recoveries are advanced, 14% for 10 yards or more. 8.5% of fumbles go out of bounds, which keeps the ball with the offense.
- **Today.** A fumble is dead where it is fallen on (`game/sim.rs`, `fumble` sets `Ball::Dead`; `Ball::Loose` is never made). Nobody advances one. A defense that falls on it in the offense's end zone gets first and goal at the 1, not a touchdown. An offense that falls on it in its own end zone gives up no safety, and the ball never goes out of bounds.
- **The fix.** A loose ball that bounces and is scooped or fallen on by whoever gets there, run back like any carry, out of bounds at the sideline, with the end zone's results (touchdown, touchback, safety).
- **Rules.** The defense may always advance it. In the NFL (since 1979) only the fumbler may advance his own team's fumble on fourth down and after the two-minute warning of either half; a teammate's recovery comes back to the spot of the fumble (or stays where he got it, if that is behind the fumble), and is a safety in his own end zone. College has the same rule on fourth down only. High school (NFHS) lets anyone advance it. These hold in every era the world plays.
- **Example.** An edge strips the quarterback, the tackle scoops it at the 30 and runs it in.

**5. Fumbled snaps.** (high)
- **Target.** 0.185 a team game, 16% of all fumbles (19% of those from scrimmage); the offense keeps 75% of them.
- **Today.** The code says it: "The fumbled snap is not modelled" (`game/scrimmage.rs`, `run_snap`). A bad shotgun snap only costs time, and the check runs only on called passes from the shotgun, so under-center exchanges and shotgun runs carry no snap risk.
- **The fix.** Every snap can go wrong, under center and in the shotgun, from the center's snap and the quarterback's hands, more with a wet ball. A fumbled snap is a loose ball (item 4).
- **Example.** A real offense puts a snap on the ground about once in five games (0.185 a team game) and loses one about once in 22, more often with a poor snapper; today never.

**6. Strip sacks at the real rate.** (high)
- **Target.** 12.4% of sacks fumble (engine 4.1%). Fumbles on passes without a catch 0.036 a team game, interception returns included (the real count holds every pass attempt without a catch or a sack, and only 8% of these are lost). The engine's 0.011 are all interceptors fumbling on the return.
- **Today.** A sack fumbles at 0.06 when the passer saw the rusher and 0.16 when he didn't, × (1.3 − his Ball Security / 120, read raw), × 1.2 from his blind side or 0.8 from the other (`game/scrimmage.rs`, `sack_check`). It reads neither the rusher's strip habit nor the hit's force, which only a tackle's fumble reads (`game/sim.rs`, `hit_effects`). The hit on a quarterback as he throws rolls no fumble (`throw`).
- **The fix.** The strip comes from how the sack happens: the hit the passer doesn't see coming, its force, the rusher going for the ball (his strip habit, which coach fire already raises) and the quarterback's own ball security, read against the level's typical starter. The ball can come out as he throws too.
- **Example.** An edge with ten sacks a year gets about 0.4 fumbles out of them today (4.1%), one every two or three years; real about 1.2 (12.4%), one a year.

**7. Backs' fumbles at the real rate.** (medium)
- **Target.** 0.75% of designed runs fumble (engine 1.4%). Fumbles after a catch come down too (0.244 a team game, engine 0.294).
- **Today.** A carrier fumbles at a tackle by its force, the tackler's strength and tackling (read against his level's typical man at his position), his own Ball Security (read raw), rain or snow and the tackler's strip habit (`game/sim.rs`, `hit_effects`); when he reaches the ball out for the line (`reach`); and on a bad handoff or a pitch on the ground (`game/scrimmage.rs`, `run_snap`), both counted as designed runs' fumbles. So a sure-handed back already fumbles less.
- **The fix.** Find why the rate runs twice real (the force, the tackler's side, the handoffs) and fix it at the root, with the carrier's ball security read against the level's typical starter, so a sure-handed back fumbles less and a loose one more.
- **Example.** A 300-carry back fumbles about 4 times today; real about 2.

### EG3: The kicking game

**11. Long field goals at the real make rate.** (medium)
- **Target.** NFL: 55+ yards made 59.7% and tried 0.15 a team game (engine 70.9% and 0.30); 50-54 73.4% (77.4%); 40-49 79.8% (84.7%). FBS: under 30 92.1% (97.8%), 30-39 80.7% (88.0%), 40-49 65.2% (75.2%), 50+ 50.6% (65.9%). `match_check`'s field goal rows read the NFL's numbers at every level, so this list is the FBS target.
- **Today.** A field goal is a roll of the kick's angle against the posts and the leg against the distance (`game/kicking.rs`, `field_goal`). The coach kicks when the kick's expected points beat the punt's and his make estimate clears 0.3 (`game/calling.rs`, `fourth_down`), unless he goes for it. `game/flow.rs`, `scrimmage` turns a kick past `kicker_range` + 3 into a go inside the 40 or a punt, and the half's last kick reads `kicker_range` alone. The make estimate is `fg_make` (`game/flow.rs`), a model with its own range and spread, not the kick's roll.
- **The fix.** Makes fall with distance like real at each level, and the tries follow them down: the roll and the coach's estimate move together.
- **Example.** A college kicker from 45 makes three in four today; real two in three.

**12. Blocked kicks from a tracked battle, and live.** (medium)
- **Target.** Blocked: field goals 1.89% (from 0.4% under 30 yards to 3.4% from 55+), punts 0.48%, extra points 0.70% (engine 0.58%, 0.18% and 0.26%). A team is part of a blocked kick about once in 7 games (engine once in 20). Returned: in 2021-25, 17 of 51 blocked punts and 8 of 101 blocked field goals went back for touchdowns, and 3 blocked punts were safeties.
- **Today.** A field goal block is a formula on the rush unit against the protection, a low kick and the kicker's get-off (`game/kicking.rs`, `field_goal`). A punt block reads the operation time alone (the snapper's Long Snapping, the punter's Get Off and a bad snap), with no rusher or shield (`punt`). A blocked field goal is handled as a miss (`game/flow.rs`, `field_goal`), and a blocked punt lands at a random spot from 12 yards behind the line to 3 past it, where the kicking team falls on it 40% of the time (`game/kicking.rs`, `punt`). Neither is ever returned. A punting team that falls on its own blocked punt gets a first down (`game/flow.rs`, `punt`; minor bug 408).
- **The fix.** The kick's protection against the rush plays out (the low kick, the operation time, the edge rusher), and a blocked kick is a live ball (item 4).
- **Rules.** Behind the line either side can pick it up and advance it, and a kicking team that recovers a fourth-down kick there keeps the ball only if it reaches the line to gain (on an earlier down it has the next down). A blocked kick that still crosses the line is a kick again: only the receiving team may advance it, and the kicking team may only down it, or keep it after the receiving team touched it past the line.

**13. Return breakaways.** (medium)
- **Target.** Kickoff returns of 40+ 5.0% (engine 1.8%), punt returns of 20+ 8.0% (2.8%), return touchdowns 0.024 a team game (0.014), punt return average 9.45 (8.3). The average kickoff return is already right (25.7, real 25.9 in 2025) and stays.
- **Today.** Returns are tracked plays with return blocking and the same tackling as a carry (`game/kicking.rs`, `run_return`).
- **The fix.** Find why the long ones close (the cover men's pursuit, the blocks set on the return, the returner's vision and cuts) and fix that at the root, under each era's kickoff rules.
- **Example.** An elite returner breaks one about a third as often as real.

**14. The kicking range reads the weather.** (low-medium)
- **Target.** In outdoor games, real teams try 13% fewer kicks of 50+ in winds of 15 mph or more (0.335 a team game against 0.386 in calm, dry games) and 26% fewer at 32 F or colder (0.286), and make 78% of the ones they pick in the wind. The engine tries 8% more in a 17 mph wind (0.62 against 0.58 at 5 mph) and makes 68% of them (74% at 5 mph).
- **Today.** The coach's range and make estimate read only the kicker's ratings (`game/flow.rs`, `kicker_range`, `fg_make`), while the kick itself reads the weather: the wind along its line, cold, rain and altitude for its distance (`game/kicking.rs`, `leg`), and the wind's speed for its aim, 1.2% wider a mph (`field_goal`). In the default runs the 5 mph wind blows straight across (`wind_along` 0); only the `AFM_WIND` runs give each game a wind along the field.
- **The fix.** The coach's estimate reads the same weather the kick does.
- **Example.** Into a 20 mph wind a real coach punts from the 36; the engine's coach tries the 53-yarder.

**Bug 403** (fouls on kickoff and punt returns are never enforced) goes into EG3 (Claude's call under Cain's proxy: real returns draw them, so the realistic answer is yes). Kicks drew 0.86 accepted fouls a team game in 2025, 14% of all accepted fouls, holding first. A return runs on the same tackles and blocks as a scrimmage play (`escort`), whose foul draws were sized for scrimmage plays, so EG3 sizes the return fouls to real kicks by kind (holding, blocks in the back, face masks, personal fouls) and enforces them with their ejections, fines and sparks.

### EG4: The end of halves and game management

**15. The call on a half's last snaps.** (medium)
- **Target.** Counted the way `AFM_GAPS` and the audit's real script both count it: every snap with 5 s or less left, in the second quarter at any score and in the fourth when tied or down 1-8, in one pool (real 1,011 snaps in five seasons, 74% of them in the second quarter; engine 1,723 in 2,000 games, 81%). Each share is of the whole pool, real against engine. Second quarter: a pass under 35 air yards 11% (38%), a run 2.9% (7.4%), a field goal 38% (25%), a kneel 13% (1.7%), a spike 5.2% (0.1%), a punt about none (3.5%), a deep shot of 35+ air yards 3.7% (1.5%). Fourth quarter, tied or down 1-8: a field goal 11.1% (2.7%), a deep shot 2.7% (0.6%), a spike 3.3% (0.6%). Read within each quarter: second-quarter field goals 51% (31%) and kneels 18% (2.1%), fourth-quarter field goals 42% (14%). Touchdowns on these snaps 2.5% (0.8%). A spike and the snap after it both count, and nflverse counts sacks inside passes and scrambles inside runs, where the engine gives each its own row (second quarter 1.8% and 1.3%). Tied teams kneel too: 96 times in five seasons (38 to end the first half, 51 to end regulation, 7 in overtime).
- **Today.** The last-snap branch kicks a field goal in range at 6 s or less (in the second quarter, or the fourth when tied or down 1-3), kneels (`should_kneel`: only with a lead, in either half, so a tied team never kneels; in the second quarter only under 30 s with the clock running and the ball short of the other side's 40) or spikes (in the hurry-up with the clock running, 3 to 30 s left, no timeouts left, before the last down). Its comment says "else take a shot", but no code does (`game/flow.rs`, `scrimmage`). Out of range the last snap is an ordinary two-minute call. There is no Hail Mary concept. The win chance on screen credits a last-snap shot at the end zone that no engine play throws (`game/win_chance.rs`, `drive`).
- **The fix.** A Hail Mary (receivers flood the end zone, the quarterback buys time and throws it up) and a last-snap call by score, spot and time, kneels when tied included. The win chance then credits the play that is really thrown.
- **Example.** Down 4 at midfield with 3 s left, a real team throws to the end zone; the engine runs a normal concept.

**16. Laterals on the last play.** (low)
- **Target.** A lateral on 22% of passes thrown with 5 s or less left in the fourth quarter from 40+ yards out, when tied or down 1-8 (16 of the 74 such passes in five seasons).
- **Today.** None. The option pitch is the engine's only lateral, a run play behind the line.
- **The fix.** The last play's laterals, with their loose-ball risk (item 4).

**17. The clock before halftime.** (low-medium)
- **Target.** Real NFL defenses call 1.35 timeouts a game in the last 4:00 of the second quarter (in 75% of games), and real offenses kneel on 18% of the second quarter's last snaps (13% of item 15's pool).
- **Today.** The defense calls a timeout only in the fourth quarter, behind and under 3:00 (`game/flow.rs`, `after_snap_clock`; its second-quarter clause is switched off with `&& false`). Every second-quarter offense hurries under 2:00 whatever its spot or score (`game/calling.rs`, `Situation::hurry`), so a team at its own 10 hurries from 2:00 and kneels out only the last 30 s, and only when ahead with the clock running (`should_kneel`). `after_snap_clock` already holds a second-quarter run-out pace (own half, not behind, 38 s between snaps) that is never reached, because the hurry is checked first. Points in the half's last 2:00 are already real (3.23, real 3.05), so this is about behaviour.
- **The fix.** A defense saves time for its own possession with its timeouts, and an offense hurries, plays normally or runs out the clock by its spot, the score and both sides' timeouts.
- **Minor bug 407** (your own live timeouts give back no clock) is in the same code, so EG4 fixes it if it is still open (Claude's call under Cain's proxy; it was not in the audit).

**18. Protecting a late lead.** (medium)
- **Target.** Designed runs by the team ahead in the last 5:00 end out of bounds 2.3% of the time (5.8% in the first three quarters), catches 14.7% (21.3%).
- **Today.** The ball carrier's sideline choice reads his Contact habit and the closing tackler, never the score or the clock (`game/sim.rs`, `run_carrier`). The two-minute drill is the only clock mode a runner has (`game/scrimmage.rs`, `hurry_run`). No runner goes down short of the goal line to let his team kneel out the game.
- **The fix.** The carrier reads the score and the clock: inside when ahead late, out when behind late, down short of the goal line when the first down wins the game.
- **With S6.2.** Today's runners rarely reach the sideline at all: 6.9% of plays end out of bounds (real 10.7%), designed runs 1.3% (5.5%). So item 18 only shows once S6.2 is tuned. S6.2 (Cain: "yes tune", `docs/design/fix-list.md`) is partly built: build 9 had a carrier near the sideline take it and a tackle drive him out, and minor bugs 137, 138 and 141 push men out from 8 yards. Runners stepping out more readily was tried and dropped, because it cost yards a carry and explosive runs. What is left is route and run-path geometry (short outs 5.0% of targets, real 17.4%; a wide run's path turns upfield and ends 15+ yards from the sideline about two times in three), which waited for Raichel (`docs/match-engine.md`, "Fix list S6.1 and S6.2" and "Passing, the sideline and the pocket"). S6.2 has no lane of its own, so EG4 tunes the rest of it with item 18: the wide run's path toward the sideline and the short outs, the way real teams run them (Claude's call under Cain's proxy). Bending the run path reworks the run game tuned in minor bugs 164 and 136, so EG4 keeps yards a carry, the run shape and explosive runs at real while it does, and it meets SF4 (the receivers' routes) and SF9 (long runs) under the rule in "Where they meet".
- **Example.** Up 4 with 3:00 left, a real back cuts back inside to keep the clock running; the engine's back steps out as often as in the first quarter.

**19. Two-point tries at the real rate.** (low-medium)
- **Target.** 9.8% of tries (0.244 a team game; engine 5.0%, 0.141). The engine has more tries (2.80 a team game, real 2.49) because it scores more touchdowns (item 8), so the share is the target. Success is already right (47.7%, real 47.5%).
- **Today.** A fourth-quarter chart by the score after the touchdown, followed 80% of the time; with a kicker under 0.75 on extra points the team goes for two 55% of the time; and every try carries a lean of 1% at aggressiveness 10, 0.4 points more for each point bolder (`game/calling.rs`, `go_for_two`).
- **The fix.** A real habit table by score and time, moved by the coach the way fourth downs already are.

**20. Fourth-down sums by level.** (low-medium)
- **Target.** Each level's own numbers: punts average 46.5 yards gross in the NFL, 42.3 at FBS and 40.0 at FCS (the repo's college targets), and a college miss comes back to the line of scrimmage (item 25).
- **Today.** The staff's fourth-down values take their expected points and the go habit from the level's table (the NFL's, or college's below it, high school included: `game/tables.rs`, `expected_points_in`, `real_fourth_go_habit`). The rest is the NFL's at every level, and the only player they read is the offense's kicker, through his make estimate: the conversion chance from yards to go only, a punt always 41 yards net, a miss at the NFL's spot of the kick (`game/calling.rs`, `fourth_down`, `fourth_ev`). They pick field goal or punt for every coach, lean the sharper coaches' go calls and fill your fourth-down chart.
- **The fix.** The values come from each level's numbers and read both teams (the offense's short yardage, the punter, the kicker).
- **Example.** An FCS team's punt is valued as a 41-yard net punt, whatever its punter.

**21. AI fake punts and fake field goals.** (low)
- **Target.** 0.036 fake punts a team game (98 in five seasons, 55% converted). Fake field goals exist too.
- **Today.** AI staffs never fake (`game/flow.rs`, `fake_punt` needs `prep.teams.fake`, set only from your week's plan), and no fake field goal exists anywhere. The AI's only special-teams trick is the surprise onside kick (a chance of 0.002 × aggressiveness / 10).
- **The fix.** AI staffs fake at the real rate by situation and boldness, and a fake field goal play for every staff.

**22. The rules at 0:00.** (low)
- **Target.** Untimed snaps at 0:00 about once in 68 NFL games.
- **Rules.** NFL: an accepted defensive foul, a double foul, or the kicking team's illegal touching or fair-catch interference on a half's last play gives an untimed down; the offense's other fouls don't. A foul that stops the clock late in a half costs the offense 10 seconds (the runoff, which the other side may decline). College: an accepted live-ball foul by either team extends the period (the offended team may decline that when the offense's foul carries loss of down), at the end of every quarter to 2022 and of the second and fourth from 2023 (a foul at the end of the first or third is marked off to start the next); from 2011, a 10-second runoff in each half's last minute for a foul that stops the clock, at the offended team's choice. High school (NFHS): an accepted live-ball foul by either team extends every quarter, except a loss-of-down, unsportsmanlike or nonplayer foul; no runoff, but from 2017 the offended team may start the clock on the snap after an accepted foul in a half's last 2:00.
- **Today.** No untimed down after any foul on a period's last play (the try is the only untimed down), and no 10-second runoff.
- **The fix.** Each level's extensions and runoff above, by era.

### EG5: Each level's rule book and the flags

**Built 2026-11-18** (thread "Build engine lane EG5 rule books"; how it works in `docs/match-engine.md`, "Each level's rule book and the flags", and `docs/reference/how-it-works/match-engine.md`, "Penalties"). Every size is proposed, not confirmed. Claude's calls under Cain's proxy, each the most realistic option found:
- Texas high schools play the NCAA's book every season and Massachusetts through 2018, both on college hash marks (the fields they were laid out for); the other leagues keep the NFL's book.
- A lineman past the limit is flagged by how plainly he is past it: the crew sees where he is, not what he knows, so his Football IQ keeps him back instead.
- The defender's interference hinders the catch and is flagged when the ball falls incomplete and on 12% of the catches made through it, so the offense declines few flags (6%, real 11%; main 44%).
- Illegal contact is drawn in the NFL only, as the shove past 5 yards in the hand fight; below the NFL that contact is legal.
- The NFL's men, who are fined, draw 30% fewer late hits and face masks than college and high school players would on the same tackles; roughing the passer is widest in the NFL (college and high school flag 0.62 of the same hits).
- Unsportsmanlike conduct after the play is drawn by the man who made its big moment, by his ego and temper.
- College's cadence on third and fourth down pushes false starts 0.66 as hard as the NFL's, since more college offenses snap on a clap or a sideline signal (real FBS third down 1.65 times first down's false starts, the NFL 1.76).
- The delay to back up a punt reads the distance (half the time from the 33 in the NFL), 0.8 as often under the NCAA's and the NFHS's books, whose whole punt team may leave at the snap.
- Fouls before a kick's snap come from the unit (the punt team's split men, the field goal wall, the rush timing the snap), and a coach set to kick on fourth and 3 or less first tries a hard count.
- The tackle's helmet-first share is refitted to 0.09 (0.07), so FBS targeting stays in its real band once EG5 brought the late hits to FBS's real count.
- The call table's run rows stay (EG5 moves them only by noise) and its pass rows, straight and SF1's faked ones, wait for SF5 or SF10, as minor bug 405 says; the analysts' tables are refitted on the engine with SF1 merged.

**23. Penalty yards and first downs by level.** (medium)
- **Rules.** College defensive holding is 10 yards (the engine gives 5), with a first down on every call since 2022; before 2022 the first down came only when the defense held an eligible receiver beyond the line on a forward pass that crossed it. In high school (NFHS) only roughing the passer, kicker, holder or snapper gives an automatic first down; pass interference is 15 yards from the previous spot with no first down (since 2013), and holding is 10.
- **Also by level.** Offensive pass interference: NFL 10 yards, college 15, NFHS 15; the engine gives 10 everywhere. Intentional grounding: NFL loss of down and 10 yards from the previous spot (or the spot of the pass, if that is farther back), college loss of down at the spot of the pass, NFHS loss of down and 5 yards from the spot of the pass; the engine uses the NFL's everywhere. A defensive foul during a run that ends beyond the line is marked from the end of the run at every level, not only a personal foul.
- **Each state's book.** These are the NFHS states' rules. Texas high schools play college rules (with the UIL's own changes) in every year, and Massachusetts did through 2018 (NFHS from 2019), so their games take the college rules in items 22-28. The engine already sends Texas and Massachusetts games to `Rules::high_school_on_college_rules` (`world/hs_year.rs`, `world/front_office/school.rs`), but that sets only the hashes, keeps the high school level and does it for Massachusetts in every year. EG5 keys these rules on the state's book and year, not on the high school level.
- **Today.** Penalty yards and first downs come from one table with no level (`game/sim.rs`, `FoulKind::yards`). When a foul is marked, only pass interference's spot changes by level, and every defensive foul but a personal foul is marked from the line (`game/flow.rs`, `resolve`). Celebrations, college taunting and bench flags already have rules by level.
- **The fix.** Yards, spots and first downs by level, state and era, through the rule books.
- **Example.** A high school offense in an NFHS state on 3rd and 20 draws pass interference: real 3rd and 5; the engine a first down.

**24. Penalties near the goal lines.** (low-medium)
- **Rules.** Every level halves a distance penalty that would move the ball more than half the distance from the enforcement spot to the offending team's goal line. Pass interference keeps its own spots near the goal: the NFL at the spot of the foul, the 1 when it is in the end zone (half the distance when the ball was already that close; the exact line is in the rule book's table); college at the spot of the foul, 15 yards from the previous spot or the 2, whichever comes first (half the distance from a previous spot on or inside the 2); NFHS 15 yards from the previous spot, halved like any distance penalty. In the NFL, 12 of the 65 safeties of 2021-25 came on a foul in the end zone (7 for grounding, 3 for holding).
- **Today.** A defensive foul is halved only when its full yards would reach the goal line, and then from the line of scrimmage, even a personal foul marked from the end of a run (`game/flow.rs`, `resolve`); college and high school interference go to the 1. An accepted offensive foul is never a safety: intentional grounding in the end zone puts the ball at the 1, and every other offensive foul, holding included, is marked from the line of scrimmage, half the distance at most (a hold in the end zone with the line at the 6 puts it at the 3), never from where it happened (the foul's own spot is recorded but read only for grounding). Pre-snap fouls halve correctly (`no_play`).
- **Example.** Roughing the passer on an incompletion from the 20 puts the ball at the 5 today; the rule gives the 10. Defensive holding from the 6: today the 1, the rule the 3.

**25. Kick rules by level.** (low)
- **Rules.** In college a miss the defense doesn't touch past the line comes back to the line of scrimmage (or to the 20, when the line is inside it), about 7 yards better for the kicking team; a miss the defense fields is returned like a punt. In high school (NFHS) a try is a scrimmage kick like a punt: a miss that reaches the end zone is a touchback at the 20, up to about 17 yards better on a long try, and a short one is the defense's ball where it stops or is downed, or where its return ends. A college kickoff out of bounds goes to the receiving team's 35 through 2007, its 40 in 2008-2011 (kickoffs from the 30), and since 2012 30 yards beyond the kickoff spot (the 35 from the 35). A high school one goes 25 yards beyond the kickoff spot (the 35 from the 40). A spot measured from the kick moves with a free kick from elsewhere (after a safety or a penalty). The NFL's 40 stays for its usual kickoff. Texas plays the college rules.
- **Today.** A miss goes over at the NFL's spot of the kick at every level (`game/flow.rs`, `field_goal`), and a kickoff out of bounds goes to one fixed spot, the 40 (`game/books.rs`, `kick_out_spot`).

**26. Ineligible man downfield (a bug, minor bug 406).** (medium)
- **Target.** 0.106 a team game on every kind of play. The engine calls 0.383 at the NFL (FBS 0.535, high school 0.744), all on RPOs, with RPOs at only 3.3% of plays (real 6.2%, FTN 2025).
- **Today.** A lineman past his level's limit (NFL 1 yard, college 3, high school 2: the snap's `downfield`, `game/flow.rs`) when an RPO is thrown past the line is flagged at 60%, × 0.4 to 1.4 by his Football IQ (`game/scrimmage.rs`, `run_snap`).
- **The fix.** Find why so many linemen are past the limit on RPO throws (not measured yet) and fix that at the root. The limits by level are built and stay.
- **Example.** An RPO team loses 5 yards on about one RPO in five.

**27. Pass interference at the real rate and size.** (medium)
- **Target.** 0.507 accepted a team game at 15.8 yards (engine 0.352 at 12.5): 8.0 yards a team game instead of 4.4. The engine already flags 0.651 a team game, and the offense declines the other 46%, which it does when the ball is caught anyway (`game/flow.rs`, `resolve`; `AFM_GAPS` prints flagged and accepted). So the gap is in which contact draws the flag and where it is spotted, not in how many flags fly.
- **With SF5.** SF5 (where picks and breakups come from) works the same catch-point contact. Whichever of the two merges second measures pass interference again and keeps it at real.

**28. The fouls the engine lacks or calls at the wrong rate.** (low)
- **Missing.** The defense's too many men on the field (real 0.070 a team game; the offense's 0.020 already sits inside illegal formation, which stands for every 5-yard offensive alignment foul, `game/flow.rs`, `pre_snap_foul`). A kickoff out of bounds or short of the landing zone booked as a foul: the engine already plays both (the ball at the 40, `game/kicking.rs`, `kickoff`) but never flags either; under the 2025 rule book the test league plays, the real rate is 0.151 a team game (short of the zone 0.079, out of bounds 0.072, the repo's 2025 counts), not the 2021-25 mix of 0.043 (no landing zone before 2024). Illegal contact (0.105) and illegal use of hands (0.125, both sides together; the defense's is a 5-yard automatic first down in the NFL, the offense's 10 yards) as calls of their own (the repo's 2024 counts). Today defensive holding stands for illegal contact and the defense's illegal hands, and offensive holding for the offense's (`match_check`, the penalty note): defensive holding is 0.504 accepted a team game against real holding alone 0.329, so the new calls take their share out of holding, not on top of it. Live-ball offside and its free play (today offside always kills the snap, `pre_snap_foul`).
- **By level.** These counts are the NFL's. Illegal contact is the NFL's alone: college and high school have no 5-yard contact rule, and their defensive illegal use of hands is holding's 10 yards (college's with a first down from 2022). The landing zone is the NFL's from 2024. Live-ball offside and its free play are the NFL's and college's; in NFHS high school a defender in the neutral zone is encroachment and the play is dead. Too many men is 5 yards in the NFL and college; in NFHS a snap taken with 12 on the field is illegal participation, 15 yards.
- **Off.** Intentional grounding 0.015 (real 0.080); false starts 0.88 (1.14) and delay of game 0.22 (0.30) short; the 5-yard formation group long (illegal formation alone 0.41, against real formation, shift, motion, offensive offside and the offense's too many men together 0.28).
- **Illegal blocks in the back** (found in the last review; it goes into EG5, Claude's call under Cain's proxy). The engine calls 0.289 accepted a team game, all on scrimmage plays (returns draw them too but bug 403 keeps them off the books), against real 0.10 on all plays, about 0.07 of it on kicks. So on scrimmage plays it runs about ten times real. It was tuned up to `match_check`'s old real column: the flag on a block from behind went from 3% to 7.5% to chase 0.28, when 0.1 a team game was already near real (`game/scrimmage.rs`, `escort`). EG5 sizes it again on the real kinds and checks why blocks from behind happen as often as they do, and EG3 sizes the return side with bug 403.
- **In `match_check`.** Its penalty rows read against Claude's 2023 reading of nflpenalties.com, for diagnosis only, the same at every level, with kinds folded together (pass interference 0.42; defensive holding 0.65 with illegal contact and illegal hands; illegal formation 0.60 with shifts, motion, ineligible downfield and too many men; illegal block in the back 0.28). Read that way, illegal formation shows 32% short where it is long. EG5 follows this file's targets (nflverse 2021-25, the 2025 numbers where the rule book changed) and puts them in those rows, each kind on its own, so the tool and this file agree.

### EG6: The red zone (after SF7)

**8. The red zone plays tight like real.** (high)
- **Target.** NFL red zone completion 53.0% (engine 65.9%), a touchdown on 19.0% of red zone throws (26.5%), touchdowns a trip 59.8% (65.9%), pass touchdowns 1.46 a team game (1.80), points a drive 2.00 (2.37), points 22.5 (24.8). FBS engine: completion 63.1%, a touchdown on 26.8% of throws (the repo has no college red zone target).
- **Today.** The old known gap is still there: "slants and smash near the goal line score too often against man coverage" (`docs/match-engine.md`, 2026-10-14). No scheme-fit item targets it, but two will move it: SF4 (item 6: height pays in contested catches and the red zone) and SF5 (item 17: breakups from 4.2 to 8.2 per 100 throws).
- **Why after SF7.** Red zone coverage rides on SF7's coverage jobs, so EG6 measures again after SF7 merges and fixes what is left at the root: the short field, the end line, tight windows and the defense's red zone calls.
- **Example.** Second and goal from the 8, a real NFL pass completes about half the time; the engine's two in three.

### EG7: The quarterback off script (after SF4 and SF8)

**9. Quarterbacks extend plays.** (high)
- **Target.** 10% of throws come after 3.94 s (engine 2.5%), time to throw 2.71 s (2.48), scrambles 1.97 a team game (1.60), sacks a pressure 0.283 (0.24), sack rate 6.8% (5.5%). 18% of real dropbacks leave the pocket (FTN charting).
- **Today.** A passer with a lane can already break out of the pocket with his eyes downfield, then throw on the move, throw it away or run (the scramble drill, minor bug 138: with nobody open or pressure over 0.5 after 1.6 s, he breaks toward the side with no free rusher; `game/scrimmage.rs`, `quarterback`, `EXTEND`, `EXTEND_RUN`). But few get out before the play is dead (`docs/match-engine.md`, "Passing, the sideline and the pocket"). His receivers' own drill (`run_route`) starts only once he takes off to run or at 4.6 s, not when he breaks out to throw. SF8 (the pass rush) is about the rushers' wins, not the passer's escape. EG7 starts from the drill that is there.
- **Why after SF4 and SF8.** Extending a play sits on the quarterback's reads (SF4) and the rush (SF8).
- **Example.** A Josh Allen type escapes the edge, runs right and throws 30 yards downfield.

### In other lanes and small fixes

**10. Designed quarterback runs.** (medium) Engine 0.54 a team game (2.1% of designed runs), real 1.69 (6.9%: sneaks, keepers, draws and power). The test league's passers are typical (Speed 61), so part of this can be the roster. It goes into SF10 (the roster shapes every call), which measures it on world rosters.

**29. Interior DL rotation.** (low-medium) The top two DTs play 78% of snaps (real 68% and 50%). Edges were fixed with bug 358; interior linemen were not. A small fix.

**30. Rain and wind at the real size.** (low) Rain costs the engine 1.9 points a team against the default dry games at 5 mph (yards an attempt -0.46); real rain games 3.4 (-0.75), on 60 games (a standard error of about 1 point). A 17 mph wind costs 2.0 points against the default 5 mph (2.5 against no wind); real 15-19 mph 2.8 against dry games under 10 mph (69 games). Completion and yards a carry move about as real. Weather reads throws, catches, fumbles, kicks and both sides' calls; footing (speed, cuts) and heat (fatigue) are not read. A small fix, kept inside the real numbers' error.

### Not lanes

- **Close games.** NFL test-league games decided by 8 or less 44.8% (real 53.5%), overtime 4.4% (6.2%), margins of 21+ 20.6% (16.9%). On world seed 1's NFL clubs SF3 read games within 8 at 56.6% before it and 52.9% after (real 53.5%), and the points spread at 9.6 and 9.9 (real 9.9) (`docs/design/scheme-fit.md`, SF3, "Measured"). So close games come from the test league's spread of team quality, not the engine. Overtime and margins of 21+ were not read on world rosters.
- **FBS scoring.** 23.5 a team game (real 27.0). Cain, 2026-10-19: "Keep it for now, we'l tune later" (`docs/design/fix-list.md`), so it stays as it is. Explosive runs are 20% short at FBS (SF9 covers breakaway runs). Interceptions are 22% short, and no lane covers that: SF5 moves where picks come from but keeps the league's total (scheme-fit item 15).

## The lanes

Each lane is its own fresh thread. The coordinator starts them, and each one merges to main on green and sends one final reply (an agreed sequence). Every lane runs on Opus at max effort, as the scheme-fit lanes do (Cain, 2026-11-17).

| Lane | Items | Starts after | Model | Changes |
|---|---|---|---|---|
| EG1 Injuries | 1, 2, 3 | now, beside the SF chain | Opus max | game results, the world's injuries |
| EG2 Ball security | 4, 5, 6, 7 | now, beside the SF chain | Opus max | game results |
| EG3 The kicking game | 11, 12, 13, 14, bug 403 | now, beside the SF chain | Opus max | game results |
| EG4 The end of halves and game management | 15-22, the rest of S6.2, bug 407 | now, beside the SF chain | Opus max | game results, the staffs' calls |
| EG5 Rule books and flags | 23-28, illegal blocks in the back | now, beside the SF chain | Opus max | game results |
| EG6 The red zone | 8 | SF7 | Opus max | game results |
| EG7 The quarterback off script | 9 | SF4 and SF8 | Opus max | game results |
| SF10 (its own lane) | 10 | SF4 to SF9 | Opus max | the calls |
| Small fixes | 29, 30 | now, one short thread | Opus max | game results |

- **All at once.** EG1-EG5 and the small fixes start together, beside the SF chain (Claude's call under Cain's proxy: Cain wants the lanes running side by side, and the few places they build on each other are named below). If they can't all start at once, the order is EG2, EG1, EG5, EG3, EG4 (by impact: the five high items in EG2 and EG1 first, then the bug in EG5).
- **Between the EG lanes.** Item 12's live blocked kicks (EG3) and item 16's laterals (EG4) use item 4's loose ball (EG2). Those lanes build their other items first; if EG2 has not merged when they reach that part, they merge the rest and add it, in the same thread, once EG2 is on main. Item 20's college miss spot (EG4) is item 25's rule (EG5), and item 22's fouls at 0:00 (EG4) are marked by EG5's enforcement: whichever of the two merges second reads the other's. Items 11 and 14 (EG3) and item 20 (EG4) all move who tries a field goal, so whichever merges second measures the other's tries again. Bug 403's return fouls (EG3) and the illegal blocks (EG5) share the same foul draw (`escort`): whichever merges second measures both again. EG1 and EG2 both change `hit_effects` (injuries and fumbles at a tackle).
- **Where they meet the SF lanes.** Item 27 shares the catch point with SF5, and item 13 the open-field running with SF9's long runs. Item 8 meets SF4 and SF5 the same way. The rest of S6.2 (EG4) meets SF4 (routes) and SF9 (run paths). Whichever lane merges second measures the other's numbers again: an SF lane with `AFM_GAPS=1 match_check`, an EG lane with `fit_check`. `docs/design/scheme-fit.md` says the same in its "Engine gaps" note and its last lane rule.
- **Other lists.** Minor bug 406 is item 26 (EG5), minor bug 408 is part of item 12 (EG3), minor bug 407 goes with item 17 (EG4), and bug 403 with EG3. Each lane removes its entries from `GAPS.md` (Engine, "The engine gaps audit (agreed)") and marks its minor bugs fixed in the project folder's `afm/minor-bugs/list.md`.
- **The memory diet.** Its M3 (item 13) changes results, so it waits for every SF and EG lane to merge (Cain: "wait until all the engine changes are done", `docs/design/memory-diet.md`). M1 and M2 keep every result the same, so they go on beside these lanes.

Every EG lane:
- reads this file, the report's section for its items (`afm/engine-gaps/report.md`) and follows its targets; where the two differ, this file is right;
- fixes root causes only: no caps, no flat boosts, no fudge on a total, and never a cap on freaks; each rating is read against the level's typical starter;
- follows each level's rules by era where a rule differs (`game/rules.rs`, `Rules::for_year`; the other leagues in `game/books.rs`), and in high school each state's book (Texas plays college rules every year, Massachusetts through 2018);
- measures its items before and after with `AFM_GAPS=1 match_check` at the NFL and FBS (2,000 games each) and high school (1,000), on the same seeds as main, so the two runs are an A/B. EG3 adds the wind runs for item 14 and a cold run, through a temperature setting it adds to `match_check` beside `AFM_WIND` and `AFM_RAIN` (nothing sets `Conditions.temperature` there today); the small fixes the rain and wind runs for item 30;
- where `AFM_GAPS` has no count for a target, adds the count to `gaps_counts` first and measures main with it: fumble returns, advances and fumbles out of bounds; fumbled snaps (today an unthrown pass becomes a sack, so they would land in the sack or run bucket); blocked kicks returned; men back in the same game; second-quarter defensive timeouts; out of bounds by the team ahead in the last 5:00 (today's out-of-bounds count skips the last 2:00 of each half and the fourth quarter's last 5:00); laterals; untimed downs;
- checks the league's totals against real with `match_check`; a total that leaves its real band is fixed in the lane, not left for later (in EG5, against this file's penalty numbers, not `match_check`'s old penalty column, until EG5 replaces it);
- reruns `call_values` when its change moves what a call gains (the coaches' table, `crates/afm-engine/data/call_values.csv`), and `decide_fit` when it moves conversions, punts, kicks or scoring, as that tool's header says (nearly every EG lane does; EG3's field goals by distance are one of its tables). An EG lane that merges after SF10 also reruns whichever of `xcomp_fit`, `xyards_fit` and `xpass_fit` its change moves, since SF10 refits them "once the physics lanes are in" (`docs/design/scheme-fit.md`);
- in EG6 and EG7, measures again with `fit_check` the scheme-fit items its change reaches (coverage jobs for EG6; the quarterback's reads and the pass rush for EG7);
- in EG1, also measures games missed by position on a small world, because the world reads every game's injuries;
- updates `docs/match-engine.md` and the how-it-works pages it touches (`docs/reference/how-it-works/`) in the same PR, and bumps their commit lines;
- carries a `## Patch note`, because players see the change;
- reports `match_check`'s time against main, so a slower engine is never a surprise;
- rebases and measures again before it merges, if another engine lane (SF or EG) merged first.

## Decided under Cain's proxy (2026-11-18)

Cain was asleep and gave full proxy ("when given a chose make the most realistic choice"). These calls were Claude's, for his morning look:

1. **All five lanes start at once** (EG1-EG5 and the small fixes), with the few hand-offs between them named in "Between the EG lanes".
2. **Bug 403 goes into EG3.** Return fouls are enforced, sized to real kicks by kind.
3. **EG4 tunes the rest of S6.2** with item 18: the wide run's path toward the sideline and the short outs, keeping the run game at real.
4. **Bug 407 goes into EG4** (your own live timeouts give back the clock).
5. **Illegal blocks in the back go into EG5** (about ten times real on scrimmage plays), and EG5 moves `match_check`'s penalty column to the real kinds.
6. **Lanes don't wait for Cain at night**: a real choice gets the most realistic option, built and listed in the lane's final reply.
