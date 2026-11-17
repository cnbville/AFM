# Gaps

Gaps found while fixing bugs or building, so they carry over between threads (Cain, 2026-11-17). Each: what's missing, why it matters, impact. Remove an entry once it's built. Nothing here is agreed until Cain says yes.

## Engine

From the engine gaps audit (2026-11-17, thread "Find engine gaps beyond scheme fit"): real NFL play-by-play (nflverse 2021-25) and the repo's college targets against `match_check` on main, each code claim checked twice. Numbers, sources and code lines are in the project folder at `afm/engine-gaps/report.md` (items 1-30, proposed lanes EG1-EG7). None of it overlaps the scheme-fit items.

- **Injuries only in tackles and sacks** (high). Blocks, the hit at the catch point and the hit on a throwing quarterback never roll one, so backs take 35% of game injuries (real about 8%) and offensive linemen 0.2% (real about 18%). Item 1.
- **No non-contact injuries** (high). About 70% of NFL ACL tears come without contact, and hamstrings are sprinting injuries; the engine draws every injury at a hit. Item 2.
- **Nobody comes back in the same game** (medium). A hurt man plays on hurt or is out; most real in-game injuries cost a few snaps, and a bruise sends about half its men out for good. Item 3.
- **Fumbles are dead where they're fallen on** (high). No fumble is ever returned (real 0.032 return touchdowns a team game, a third of the defense's touchdowns from scrimmage), a defensive recovery in the end zone is first and goal at the 1 instead of a touchdown, and none goes out of bounds (real 8.5%). Item 4.
- **No fumbled snaps** (high). Real 0.185 a team game, 16% of all fumbles; a bad shotgun snap only costs time (`run_snap`). Item 5.
- **Strip sacks at a third of real** (high). 4.1% of sacks fumble, real 12.4%. Item 6.
- **Backs fumble about twice as often as real** (medium). 1.4% of designed runs, real 0.75%. Item 7.
- **The red zone plays loose** (high). NFL red zone completion 65.9% (real 53.0%), a touchdown on 26.5% of red zone throws (19.0%), 2.37 points a drive (2.00). Item 8.
- **Quarterbacks don't extend plays** (high). 2.5% of throws come after 3.94 s (real 10%), sacks 5.5% of dropbacks (6.8%), scrambles 1.60 a team game (1.97). Item 9.
- **Few designed quarterback runs** (medium). 0.54 a team game (real 1.69); the test league's passers are typical, so SF10 measures it on world rosters. Item 10.
- **Long field goals made too often** (medium). NFL 55+ made 70.9% (real 59.7%) and tried twice as often; FBS 40-49 75.2% (65.2%), 50+ 65.9% (50.6%). Item 11.
- **Blocked kicks at a third of real, and never returned** (medium). Field goals 0.58% (real 1.89%), punts 0.18% (0.48%), extra points 0.26% (0.70%); a kick's block is a formula, not a tracked play, and real blocked punts go back for a touchdown a third of the time. Item 12.
- **Few long returns** (medium). Kickoff returns of 40+ 1.8% (real 5.0%), punt returns of 20+ 2.8% (8.0%). Item 13.
- **The kicking range ignores the weather** (low-medium). The engine tries 8% more kicks of 50+ in a 17 mph wind; real teams try 13% fewer in 15+ mph wind and 26% fewer at 32 F or colder. Item 14.
- **A half's last snap is an ordinary call** (medium). No Hail Mary; second-quarter last snaps are normal plays 38% (real 11%), kneels 1.7% (13%), spikes 0.1% (5.2%); a tied team never kneels. The screens' win chance credits a shot no play throws. Item 15.
- **No laterals on the last play** (low). Item 16.
- **The clock before halftime** (low-medium). Real defenses call 1.35 timeouts a game in the last 4:00 of the second quarter; the staff's timeout needs the fourth quarter (`after_snap_clock`), and every offense hurries in the half's last 2:00 whatever its spot or score. Item 17.
- **Nobody protects a late lead** (medium). The carrier's sideline choice never reads the score or clock; real runs by the team ahead in the last 5:00 end out of bounds 2.3% of the time (5.8% earlier). Goes with S6.2. Item 18.
- **Two-point tries short** (low-medium). 0.141 a team game (real 0.244), made at the real rate. Item 19.
- **Fourth-down sums use the NFL's numbers at every level** (low-medium). Conversion from yards to go only, a punt always 41 net, a miss at the NFL's spot (`fourth_down`, `fourth_ev`). Item 20.
- **AI staffs never fake a kick** (low). Real 0.036 fake punts a team game; no fake field goal exists. Item 21.
- **No untimed down or 10-second runoff** (low). Item 22.
- **Penalties mark the NFL's yards at every level** (medium). College defensive holding is 10 yards; in high school, pass interference, holding, face masks and roughness carry no automatic first down (only roughing the passer, kicker, holder or snapper do). Item 23.
- **Penalties near the goal lines** (low-medium). Defensive fouls halve only when they'd reach the goal line, from the line of scrimmage; a foul in the offense's own end zone is never a safety (real 12 of 65 safeties in 2021-25). Item 24.
- **Kick rules by level** (low). A missed field goal goes over at the NFL's spot and a kickoff out of bounds at the 40 at every level (college: the line of scrimmage and the 35). Item 25.
- **Ineligible man downfield called about four times too often** (medium, a bug). 0.383 a team game (real 0.106), all on RPOs; minor bug 406. Item 26.
- **Pass interference short** (medium). 0.352 a team game at 12.5 yards (real 0.507 at 15.8). Item 27.
- **Fouls missing or off** (low). Too many men on the field, kickoffs out of bounds, illegal contact, defensive illegal use of hands and live-ball offside don't exist; grounding 0.015 a team game (real 0.080); false starts and delay of game short, illegal formation long. Item 28.
- **Interior linemen rotate too little** (low-medium). The top two DTs play 78% of snaps (real 68% and 50%). Item 29.
- **Rain and wind a little small** (low). Rain costs 1.9 points a team (real 3.4), 17 mph wind 2.0 (2.8). Item 30.

## Logic

- **A coach of yours can't see last season if it was played before you took the seat** (high). His unit's numbers and form (`claims::rooms::unit`, `points_of`) read only `w.seat_games`, the games kept since you arrived. In a new career every holdover's unit has no numbers and no good or poor read, so the voice gets nothing to analyse, the holdover meeting can't narrate his unit's year, and the keep or let go talk can't let you blame his unit (`takeover.rs`, `staffing.rs`). The world already keeps every team's season in the squads, games and players tallies. Found 2026-11-17 (thread "Give coaches real depth when asked why"); fix proposed there.
- **A coach hired after last season reads as "one season" here** (medium). `rooms::unit` sets `years` to `(w.year - first_season + 1).max(1)`, so a man who hasn't coached a game here gets 1, and his card says he's been the coordinator "one season". Robert Leonard, hired after a 2-10 year, took the blame for it. Same thread.
- **A position coach's year is only a growth read** (medium). His room's own numbers on the field (the depth counts of his players) never reach his talk. Same thread.

## UI

- **The holdover meeting shows no numbers for his unit** (medium). Without last season's points, yards and third downs on the narration you can't judge whether to keep him. Same cause as the first Logic gap.
