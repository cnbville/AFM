# Engine gaps beyond scheme fit (spec)

On 2026-11-17 Cain asked what other parts of the match engine need work beyond the update already running (scheme fit): "what other engine parts might need to get improved other then the update we already have running?" The engine gaps audit measured the engine against real NFL play-by-play and the repo's college targets and found 30 gaps. Its report is `afm/engine-gaps/report.md` in the project folder, with the tools in `tools/` and the raw numbers in `raw/` beside it.

Cain agreed to all 30 and to the lanes the same evening (22:09 UTC, "agreed"). The question was: "Should all 30 go on the engine fix list as lanes EG1-EG7? Say 'yes to all', or the numbers to leave out. My pick: yes to all, with EG1-EG5 beside the scheme-fit chain and EG6 and EG7 after their SF lanes." This file says what gets built and in which lane. None of it is built.

Cain's engine rule (2026-11-14: nothing in the engine before the next world builds are done, `docs/design/engine-fix-list.md`) is met: the seed 181 world was built on 2026-11-17. So EG1-EG5 can start now.

## How it was measured

- **Engine.** `match_check` on main bf8c865: NFL and FBS 2,000 games each, high school 1,000, and the NFL again in rain (`AFM_RAIN=1`), in a 17 mph wind and with no wind (`AFM_WIND`; the default is 5 mph). The audit's own counts are `AFM_GAPS=1 match_check` (`gaps_counts`, committed with this spec): points by quarter and late in each half, margins, injuries by position, field goals by distance with blocks, punts and kickoffs with returns, tries, safeties, fumbles by kind, the RPO share, quarterback runs, pass interference yards, penalties by kind and the call on a half's last snap. It counts exactly as the audit's scratch block did. The audit ran before scheme-fit lane SF3 merged, so each lane takes its before numbers from its own main.
- **Real.** nflverse play-by-play, 2021-25 regular seasons (1,359 games, 2,718 team games), read with the scripts in the report's `tools/`. College numbers come from the repo's `data/real-world/college_game_targets.json` (cfbfastR). There is no high school play-by-play.
- **Code.** Each code claim was checked twice against the code, and the how-it-works pages were corrected where they said less (PR #541).
- **One limit.** The test leagues are synthetic, with typical players and typical staffs. Where a gap could come from the test league's rosters, the item says so.

All numbers are per team game unless named. "Engine" is the NFL test league unless named.

## The items, as agreed

Cain agreed to all 30 at once. Each item's target is the real number. How the engine gets there is the lane's job, from how real football works: root causes, no caps, no flat boosts, no fudge on a total. Any mechanic or size below that isn't a real number is Claude's, proposed, not confirmed. A lane that meets a real choice (not a realism question) brings Cain a concrete proposal before it builds that part.

### EG1: Injuries

**1. Injuries for every collision, not just the ball carrier.** (high)
- **Target.** Injuries by position like real. Offensive linemen get hurt about as often per man as backs (nflverse injury reports, episodes a player-season: OL 0.43, RB 0.44, WR 0.47, CB 0.51). On a 53-man roster that puts linemen near 18% of injuries and backs near 8%. The NFL's 1996-2005 surveillance put each line near 20%. The total stays near today's 1.10 week-plus injuries a team game, which is fair.
- **Today.** An injury is rolled only in a tackle (the ball carrier and each tackler, a man joining at half the force; `game/sim.rs`, `hit_effects` → `injury_check`), on a sacked quarterback (never the sacker; `game/scrimmage.rs`, the sack) and on a flagged late hit on a slide (`late_on_slide`). It is never rolled in a block, at the catch point (targeting included), on a quarterback as he throws (roughing included), on the late hitter, on a hit that forces a fumble or on an arm tackle the runner breaks (`first_hit`). So backs take 35% of game injuries, receivers 16% and offensive linemen 0.2%.
- **The fix.** Every collision can hurt both men, by its force: the block (pile-ups, a man rolled up from behind), the hit at the catch point, the hit on a throwing quarterback, the tackler, the sacker and the late hitter.
- **Example.** In a 17-game season an engine team's backs miss time from about 6.6 game injuries (real about 1.8), and its offensive line from almost none (real about 4).

**2. Non-contact injuries.** (high)
- **Target.** About 70% of in-game ACL tears in the NFL came without contact to the knee (2014-19, 140 tears on video); offensive linemen are the exception, at 70% with contact. Hamstrings, groins and calves are sprinting injuries of receivers, backs and corners. The Achilles is its own injury.
- **Today.** Every game injury is drawn at a hit (`GAME_INJURIES`), and the Achilles is not a game injury kind. Practice injuries are built (`training/week.rs`); this item is the game's own.
- **The fix.** A man can get hurt with nobody touching him when he cuts, plants or sprints, more with tired legs (fatigue is already tracked), sized from real injury studies.
- **Example.** A receiver running a go route late in a hot game can pull a hamstring; today he never does.

**3. Back in the same game.** (medium)
- **Target.** Most in-game injuries are short: an ankle taped, a stinger, cramps. The player misses a few snaps or a series and comes back. The NFL ran 538 in-game concussion checks in 2025, and most of those players went back in. The lane sizes the return rate from real injury reports.
- **Today.** A hurt man plays on hurt or is out for the rest of the game (`game/flow.rs`, `apply_injuries`). A bruise (0-1 week, 23% of game injuries) sends about half its men out for good at Toughness 10 (`game/sim.rs`, `injury_check`).
- **The fix.** A hurt man goes to the sideline, gets checked and comes back after a few snaps, a series, or not at all, by what the injury is and how bad. A concussion follows his level's protocol by era. His backup plays meanwhile.
- **Example.** A guard rolls an ankle, misses three snaps and returns.

### EG2: Ball security

Fumbles are 31% short overall (0.79 a team game, real 1.14), and fumbles lost 17% short (0.37, real 0.447). The mix is wrong:

| Fumbles a team game | engine | real |
|---|---|---|
| on sacks | 0.085 (4.1% of sacks) | 0.298 (12.4%) |
| after a catch | 0.294 | 0.244 |
| on designed runs | 0.374 (1.4% of carries) | 0.197 (0.75%) |
| fumbled snaps | none | 0.185 (lost 25%) |
| QB hit as he throws | 0.011 | 0.036 |
| scrambles | 0.024 | 0.024 |

**4. Fumbles that stay live.** (high)
- **Target.** Fumble return touchdowns 0.032 a team game (87 in five seasons, 7% of lost fumbles, a third of the defense's touchdowns from scrimmage beside pick-sixes at 0.067). 28% of defensive recoveries are advanced, 14% for 10 yards or more. 8.5% of fumbles go out of bounds, which keeps the ball with the offense.
- **Today.** A fumble is dead where it is fallen on (`game/sim.rs`, `fumble` sets `Ball::Dead`; `Ball::Loose` is never made). Nobody advances one. A defense that falls on it in the offense's end zone gets first and goal at the 1, not a touchdown. An offense that falls on it in its own end zone gives up no safety, and the ball never goes out of bounds.
- **The fix.** A loose ball that bounces and is scooped or fallen on by whoever gets there, run back like any carry, out of bounds at the sideline, with the end zone's results (touchdown, touchback, safety) and each level's rules on who may advance it, by era.
- **Example.** An edge strips the quarterback, the tackle scoops it at the 30 and runs it in.

**5. Fumbled snaps.** (high)
- **Target.** 0.185 a team game, 16% of all fumbles; the offense keeps 75% of them.
- **Today.** The code says it: "The fumbled snap is not modelled" (`game/scrimmage.rs`, `run_snap`). A bad shotgun snap only costs time, and the check runs only on called passes from the shotgun, so under-center exchanges and shotgun runs carry no snap risk.
- **The fix.** Every snap can go wrong, under center and in the shotgun, from the center's snap and the quarterback's hands, more with a wet ball. A fumbled snap is a loose ball (item 4).
- **Example.** A center with a poor Shotgun Snap rating loses a ball every few games in real life; today never.

**6. Strip sacks at the real rate.** (high)
- **Target.** 12.4% of sacks fumble (engine 4.1%). Fumbles on a quarterback hit as he throws 0.036 a team game (engine 0.011).
- **The fix.** The strip comes from how the sack happens: the hit the passer doesn't see coming, the rusher going for the ball (his strip habit, which coach fire already raises) and the quarterback's own ball security.
- **Example.** An edge who gets home ten times a year forces about one fumble today; real about three.

**7. Backs' fumbles at the real rate.** (medium)
- **Target.** 0.75% of designed runs fumble (engine 1.4%). Fumbles after a catch come down too (0.244 a team game, engine 0.294).
- **The fix.** The carrier's ball security against the hit and the strip, read against the level's typical starter, so a sure-handed back fumbles less and a loose one more.
- **Example.** A 300-carry back fumbles about 4 times today; real about 2.

### EG3: The kicking game

**11. Long field goals at the real make rate.** (medium)
- **Target.** NFL: 55+ yards made 59.7% and tried 0.15 a team game (engine 70.9% and 0.30); 50-54 73.4% (77.4%); 40-49 79.8% (84.7%). FBS: 30-39 80.7% (88.0%), 40-49 65.2% (75.2%), 50+ 50.6% (65.9%).
- **Today.** A field goal is a roll of the kick's angle against the posts and the leg against the distance (`game/kicking.rs`, `field_goal`). The coach tries one when his make estimate clears 0.3 inside range plus 3 (`game/flow.rs`, `scrimmage`, `kicker_range`).
- **The fix.** Makes fall with distance like real at each level, and the tries follow them down through the coach's own estimate.
- **Example.** A college kicker from 45 makes three in four today; real two in three.

**12. Blocked kicks from a tracked battle, and live.** (medium)
- **Target.** Blocked: field goals 1.89% (from 0.4% under 30 yards to 3.4% from 55+), punts 0.48%, extra points 0.70% (engine 0.58%, 0.18% and 0.26%). A team is part of a blocked kick about once in 7 games (engine once in 20). Returned: in 2021-25, 17 of 51 blocked punts and 8 of 101 blocked field goals went back for touchdowns, and 3 blocked punts were safeties.
- **Today.** A block is a formula on the units' ratings (`game/kicking.rs`, `field_goal`; punts the same on the operation time). A blocked field goal is handled as a miss, and a blocked punt lands at a random spot from 12 yards behind the line to 3 past it (`game/flow.rs`, `punt`). Neither is ever returned. A punting team that falls on its own blocked punt gets a first down (minor bug 408).
- **The fix.** The kick's protection against the rush plays out (the low kick, the operation time, the edge rusher), and a blocked kick is a live ball by the rules (item 4): either side can pick it up, and a kicking team that recovers behind the line keeps the ball only if it reaches the line to gain.

**13. Return breakaways.** (medium)
- **Target.** Kickoff returns of 40+ 5.0% (engine 1.8%), punt returns of 20+ 8.0% (2.8%), return touchdowns 0.024 a team game (0.014), punt return average 9.45 (8.3). The average kickoff return is already right (25.7, real 25.9 in 2025) and stays.
- **Today.** Returns are tracked plays with return blocking and the same tackling as a carry (`game/kicking.rs`, `run_return`).
- **The fix.** Find why the long ones close (the cover men's pursuit, the blocks set on the return, the returner's vision and cuts) and fix that at the root, under each era's kickoff rules.
- **Example.** An elite returner breaks one about a third as often as real.

**14. The kicking range reads the weather.** (low-medium)
- **Target.** In outdoor games, real teams try 13% fewer kicks of 50+ in winds of 15 mph or more (0.335 a team game against 0.386 in calm, dry games) and 26% fewer at 32 F or colder (0.286), and make 78% of the ones they pick in the wind. The engine tries 8% more in a 17 mph wind (0.62 against 0.58 at 5 mph) and makes 68% (74%).
- **Today.** The coach's range and make estimate read only the kicker's ratings (`game/flow.rs`, `kicker_range`, `fg_make`), while the kick itself reads wind, cold, rain and altitude (`game/kicking.rs`, `leg`).
- **The fix.** The coach's estimate reads the same weather the kick does.
- **Example.** Into a 20 mph wind a real coach punts from the 36; the engine's coach tries the 53-yarder.

**Bug 403** (fouls on kickoff and punt returns are never enforced) sits in the same code. It needs Cain's own yes (asked in its own thread); if he says yes, EG3 takes it. Kicks drew 0.86 accepted fouls a team game in 2025, 14% of all accepted fouls, holding first.

### EG4: The end of halves and game management

**15. The call on a half's last snap.** (medium)
- **Target.** The last snap with 5 s or less in the second quarter, real against engine: a normal play 11% (38%), a field goal 38% (25%), a kneel 13% (1.7%), a spike 5.2% (0.1%), a punt about none (3.5%), a deep shot of 35+ air yards 3.7% (1.5%). Trailing by 1-8 at the end of the game: a field goal 11.1% (2.7%), a deep shot 2.7% (0.6%), a spike 3.3% (0.6%). Touchdowns on that last snap 2.5% (0.8%). Tied teams kneel too: 96 times in five seasons (38 to end the first half, 51 to end regulation, 7 in overtime).
- **Today.** The last-snap branch kicks a field goal in range at 6 s or less (in the second quarter, or the fourth when tied or down 1-3), kneels (in the second quarter only when ahead, `should_kneel`) or spikes. Its comment says "else take a shot", but no code does (`game/flow.rs`, `scrimmage`). Out of range the last snap is an ordinary two-minute call. There is no Hail Mary concept. The win chance on screen credits a last-snap shot at the end zone that no engine play throws (`game/win_chance.rs`, `drive`).
- **The fix.** A Hail Mary (receivers flood the end zone, the quarterback buys time and throws it up) and a last-snap call by score, spot and time, kneels when tied included. The win chance then credits the play that is really thrown.
- **Example.** Down 4 at midfield with 3 s left, a real team throws to the end zone; the engine runs a normal concept.

**16. Laterals on the last play.** (low)
- **Target.** A lateral on 22% of last-snap passes when trailing by 1-8 from 40+ yards out (74 plays in five seasons).
- **Today.** None. The option pitch is the engine's only lateral, a run play behind the line.
- **The fix.** The last play's laterals, with their loose-ball risk (item 4).

**17. The clock before halftime.** (low-medium)
- **Target.** Real NFL defenses call 1.35 timeouts a game in the last 4:00 of the second quarter (in 75% of games), and real offenses kneel on 13% of the half's last snaps (item 15).
- **Today.** The defense calls a timeout only when behind late in the fourth quarter (`game/flow.rs`, `after_snap_clock`). Every second-quarter offense hurries under 2:00 whatever its spot or score (`game/calling.rs`, `Situation::hurry`), so a team at its own 10 never runs out the half (part of why kneels are 1.7% of last snaps, real 13%). Points in the half's last 2:00 are already real (3.23, real 3.05), so this is about behaviour.
- **The fix.** A defense saves time for its own possession with its timeouts, and an offense hurries, plays normally or runs out the clock by its spot, the score and both sides' timeouts.
- **Minor bug 407** (your own live timeouts give back no clock) is in the same code; EG4 fixes it if it is still open.

**18. Protecting a late lead.** (medium)
- **Target.** Designed runs by the team ahead in the last 5:00 end out of bounds 2.3% of the time (5.8% in the first three quarters), catches 14.7% (21.3%).
- **Today.** The ball carrier's sideline choice reads his Contact habit and the closing tackler, never the score or the clock (`game/sim.rs`, `run_carrier`). The two-minute drill is the only clock mode a runner has (`game/scrimmage.rs`, `hurry_run`). No runner goes down short of the goal line to let his team kneel out the game.
- **The fix.** The carrier reads the score and the clock: inside when ahead late, out when behind late, down short of the goal line when the first down wins the game.
- **With S6.2.** Today's runners rarely reach the sideline at all: 6.9% of plays end out of bounds (real 10.7%), designed runs 1.3% (5.5%). So item 18 only shows once S6.2 is tuned. S6.2 (Cain: "yes tune", `docs/design/fix-list.md`) has no lane of its own, so EG4 tunes it with item 18.
- **Example.** Up 4 with 3:00 left, a real back cuts back inside to keep the clock running; the engine's back steps out as often as in the first quarter.

**19. Two-point tries at the real rate.** (low-medium)
- **Target.** 0.244 a team game (engine 0.141). Success is already right (47.7%, real 47.5%).
- **Today.** A fixed chart (`game/calling.rs`, `go_for_two`).
- **The fix.** A real habit table by score and time, moved by the coach the way fourth downs already are.

**20. Fourth-down sums by level.** (low-medium)
- **Target.** Each level's own numbers: punts average 46.5 yards gross in the NFL, 42.3 at FBS and 40.0 at FCS (the repo's college targets), and a college miss comes back to the line of scrimmage (item 25).
- **Today.** The staff's fourth-down values use the NFL's numbers at every level and read neither team: the conversion chance from yards to go only, a punt always 41 yards net, a miss at the NFL's spot of the kick (`game/calling.rs`, `fourth_down`, `fourth_ev`). They pick field goal or punt for every coach, lean the sharper coaches' go calls and fill your fourth-down chart.
- **The fix.** The values come from each level's numbers and read both teams (the offense's short yardage, the punter, the kicker).
- **Example.** An FCS team's punt is valued as a 41-yard net punt, whatever its punter.

**21. AI fake punts and fake field goals.** (low)
- **Target.** 0.036 fake punts a team game (98 in five seasons, 55% converted). Fake field goals exist too.
- **Today.** AI staffs never fake (`game/flow.rs`, `fake_punt` needs `prep.teams.fake`, set only from your week's plan), and no fake field goal exists anywhere. The AI's only special-teams trick is the surprise onside kick (a chance of 0.002 × aggressiveness / 10).
- **The fix.** AI staffs fake at the real rate by situation and boldness, and a fake field goal play for every staff.

**22. The rules at 0:00.** (low)
- **Target.** Untimed snaps at 0:00 about once in 68 NFL games.
- **Today.** No untimed down after a defensive foul on a half's last play, and no 10-second runoff.
- **The fix.** Both, by each level's rule book and era.

### EG5: Each level's rule book and the flags

**23. Penalty yards and first downs by level.** (medium)
- **Rules.** College defensive holding is 10 yards (the engine gives 5), with a first down on every call since 2022. In high school (NFHS) only roughing the passer, kicker, holder or snapper gives an automatic first down; pass interference is 15 yards from the previous spot with no first down (since 2013), and holding is 10.
- **Today.** Penalty yards and first downs come from one table with no level (`game/sim.rs`, `FoulKind::yards`). When a foul is marked, only pass interference's spot changes by level (`game/flow.rs`, `resolve`). Celebrations, college taunting and bench flags already have rules by level.
- **The fix.** Yards, spots and first downs by level and era, through the rule books.
- **Example.** A high school offense on 3rd and 20 draws pass interference: real 3rd and 5; the engine a first down.

**24. Penalties near the goal lines.** (low-medium)
- **Rules.** Every level halves the distance whenever a foul's yards would cover more than half the way to the goal, from the enforcement spot. In the NFL, 12 of the 65 safeties of 2021-25 came on a foul in the end zone (7 for grounding, 3 for holding).
- **Today.** A defensive foul is halved only when its full yards would reach the goal line, and then from the line of scrimmage, even a personal foul marked from the end of a run (`game/flow.rs`, `resolve`). An accepted offensive foul in the offense's own end zone puts the ball at the 1 instead of scoring a safety. Pre-snap fouls halve correctly (`no_play`).
- **Example.** Roughing the passer on an incompletion from the 20 puts the ball at the 5 today; the rule gives the 10. Defensive holding from the 6: today the 1, the rule the 3.

**25. Kick rules by level.** (low)
- **Rules.** In college a missed field goal comes back to the line of scrimmage (or the 20), about 7 yards better for the kicking team. In high school a miss that reaches the end zone is a touchback at the 20, up to about 17 yards better on a long try. College and high school kickoffs out of bounds go to the receiving team's 35.
- **Today.** A miss goes over at the NFL's spot of the kick at every level (`game/flow.rs`, `field_goal`), and a kickoff out of bounds goes to the 40 (`game/books.rs`, `kick_out_spot`).

**26. Ineligible man downfield (a bug, minor bug 406).** (medium)
- **Target.** 0.106 a team game on every kind of play. The engine calls 0.383 at the NFL (FBS 0.535, high school 0.744), all on RPOs, with RPOs at only 3.3% of plays (real 6.2%, FTN 2025).
- **Today.** A lineman past the line's limit when an RPO is thrown is flagged with 60% (`game/scrimmage.rs`, `run_snap`).
- **The fix.** Find why so many linemen are past the limit on RPO throws (not measured yet), fix that at the root, and flag by each level's limit.
- **Example.** An RPO team loses 5 yards on about one RPO in five.

**27. Pass interference at the real rate and size.** (medium)
- **Target.** 0.507 a team game at 15.8 yards (engine 0.352 at 12.5): 8.0 yards a team game instead of 4.4.
- **With SF5.** SF5 (where picks and breakups come from) works the same catch-point contact. Whichever of the two merges second measures pass interference again and keeps it at real.

**28. The fouls the engine lacks.** (low)
- **Missing.** Too many men on the field (real 0.090 a team game); a kickoff out of bounds or short of the landing zone (0.043); illegal contact (0.105) and defensive illegal use of hands (0.125), the NFL's 5-yard automatic first downs beside holding (the repo's 2024 counts); live-ball offside and its free play (today offside always kills the snap, `game/flow.rs`, `pre_snap_foul`).
- **Off.** Intentional grounding 0.015 (real 0.080); false starts 0.88 (1.14) and delay of game 0.22 (0.30) short; the 5-yard formation group long (illegal formation alone 0.41, against real formation, shift, motion and offensive offside together 0.26).

### EG6: The red zone (after SF7)

**8. The red zone plays tight like real.** (high)
- **Target.** NFL red zone completion 53.0% (engine 65.9%), a touchdown on 19.0% of red zone throws (26.5%), touchdowns a trip 59.8% (65.9%), pass touchdowns 1.46 a team game (1.80), points a drive 2.00 (2.37), points 22.5 (24.8). FBS engine: completion 63.1%, a touchdown on 26.8% of throws (the repo has no college red zone target).
- **Today.** The old known gap is still there: "slants and smash near the goal line score too often against man coverage" (`docs/match-engine.md`, 2026-10-14). No scheme-fit item covers it.
- **Why after SF7.** Red zone coverage rides on SF7's coverage jobs, so EG6 measures again after SF7 merges and fixes what is left at the root: the short field, the end line, tight windows and the defense's red zone calls.
- **Example.** Second and goal from the 8, a real NFL pass completes about half the time; the engine's two in three.

### EG7: The quarterback off script (after SF4 and SF8)

**9. Quarterbacks extend plays.** (high)
- **Target.** 10% of throws come after 3.94 s (engine 2.5%), time to throw 2.71 s (2.48), scrambles 1.97 a team game (1.60), sacks a pressure 0.283 (0.24), sack rate 6.8% (5.5%). 18% of real dropbacks leave the pocket (FTN charting).
- **Today.** Quarterbacks are rid of the ball or down. SF8 (the pass rush) is about the rushers' wins, not the passer's escape.
- **Why after SF4 and SF8.** Extending a play sits on the quarterback's reads (SF4) and the rush (SF8).
- **Example.** A Josh Allen type escapes the edge, runs right and throws 30 yards downfield.

### In other lanes and small fixes

**10. Designed quarterback runs.** (medium) Engine 0.54 a team game (2.1% of designed runs), real 1.69 (6.9%: sneaks, keepers, draws and power). The test league's passers are typical (Speed 61), so part of this can be the roster. It goes into SF10 (the roster shapes every call), which measures it on world rosters.

**29. Interior DL rotation.** (low-medium) The top two DTs play 78% of snaps (real 68% and 50%). Edges were fixed with bug 358; interior linemen were not. A small fix.

**30. Rain and wind at the real size.** (low) Rain costs the engine 1.9 points a team (yards an attempt -0.46); real rain games 3.4 (-0.75), on 60 games (a standard error of about 1 point). A 17 mph wind costs 2.0 points; real 15-19 mph 2.8 (69 games). Completion and yards a carry move about as real. Weather reads throws, catches, fumbles, kicks and both sides' calls; footing (speed, cuts) and heat (fatigue) are not read. A small fix, kept inside the real numbers' error.

### Not lanes (yet)

- **Close games.** NFL test-league games decided by 8 or less 44.8% (real 53.5%), overtime 4.4% (6.2%), margins of 21+ 20.6% (16.9%). This may be the test league's spread of team quality, not the engine. A check on a world's NFL rosters settles it before it is called a gap.
- **FBS scoring.** 23.5 a team game (real 27.0). Explosive runs are 20% short at FBS (SF9 covers breakaway runs) and interceptions 22% short (SF5).

## The lanes

Each lane is its own fresh thread. The coordinator starts them, and each one merges to main on green and sends one final reply (an agreed sequence). Every lane runs on Opus at max effort, like the scheme-fit lanes (Cain's rule for engine work).

| Lane | Items | Starts after | Model | Changes |
|---|---|---|---|---|
| EG1 Injuries | 1, 2, 3 | now, beside the SF chain | Opus max | game results, the world's injuries |
| EG2 Ball security | 4, 5, 6, 7 | now, beside the SF chain | Opus max | game results |
| EG3 The kicking game | 11, 12, 13, 14 (bug 403 if Cain says yes) | now, beside the SF chain | Opus max | game results |
| EG4 The end of halves and game management | 15-22, with S6.2 | now, beside the SF chain | Opus max | game results, the staffs' calls |
| EG5 Rule books and flags | 23-28 | now, beside the SF chain | Opus max | game results |
| EG6 The red zone | 8 | SF7 | Opus max | game results |
| EG7 The quarterback off script | 9 | SF4 and SF8 | Opus max | game results |
| SF10 (its own lane) | 10 | SF4 to SF9 | Opus max | the calls |
| Small fixes | 29, 30 | now, one short thread | Opus max | game results |

- **Order.** If they can't all start at once: EG2, EG1, EG5, EG3, EG4 (Claude's order, by impact: the four high items in EG2 and EG1 first, then the bug in EG5).
- **Where they meet the SF lanes.** Item 27 shares the catch point with SF5, and item 13 the open-field running with SF9's long runs. Whichever lane merges second measures the other's numbers again.
- **Other lists.** Minor bug 406 is item 26 (EG5) and minor bug 408 is part of item 12 (EG3). Each lane removes its entries from `GAPS.md` (Engine, "The engine gaps audit") and marks its minor bugs fixed in the project folder's `afm/minor-bugs/list.md`.
- **The memory diet.** Its M3 (item 13) waits for these lanes too. Cain's words were "wait until all the engine changes are done" (`docs/design/memory-diet.md`).

Every EG lane:
- reads this file, the report's section for its items (`afm/engine-gaps/report.md`) and follows its targets;
- fixes root causes only: no caps, no flat boosts, no fudge on a total, and never a cap on freaks; each rating is read against the level's typical starter;
- follows each level's rules by era where a rule differs (`game/rules.rs`, `Rules::for_year`; the other leagues in `game/books.rs`);
- measures its items before and after with `AFM_GAPS=1 match_check` at the NFL and FBS (2,000 games each) and high school (1,000), on the same seeds as main, so the two runs are an A/B (EG3 adds the wind runs for item 14, and the small fixes the rain and wind runs for item 30);
- checks the league's totals against real with `match_check`; a total that leaves its real band is fixed in the lane, not left for later;
- reruns `call_values` when its change moves what a call gains (the coaches' table, `crates/afm-engine/data/call_values.csv`), and `decide_fit` when it moves fourth downs, tries or the clock (EG4 does);
- in EG6 and EG7, measures again with `fit_check` the scheme-fit items its change reaches (coverage jobs for EG6; the quarterback's reads and the pass rush for EG7);
- in EG1, also measures games missed by position on a small world, because the world reads every game's injuries;
- updates `docs/match-engine.md` and the how-it-works pages it touches (`docs/reference/how-it-works/`) in the same PR, and bumps their commit lines;
- carries a `## Patch note`, because players see the change;
- reports `match_check`'s time against main, so a slower engine is never a surprise;
- rebases and measures again before it merges, if another engine lane (SF or EG) merged first.
