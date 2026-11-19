# Scheme fit: schemes and skills drive results (spec)

On 2026-11-15 Cain asked whether the engine rewards a player's skills inside his team's scheme. His examples were a ballhawk free safety behind a blitz defense (Kerby Joseph), a strong safety who plays the run and man coverage (Brian Branch), 13 personnel that shouldn't feed the wide receivers, a power back against a burner on the draw, a Vikings-like defense, a Lions-like offense, and AI staffs that build and recruit to fit.

The audit measured all of it. The full pass ran on 2026-11-16, and its report is `afm/scheme-fit/full-pass.md` in the project folder, with the tools in `tools/` and the raw numbers in `raw/` beside it. Cain answered its 25 items one by one on 2026-11-16 and 17 and agreed to all of them. This file says what gets built and in which lane. SF0 (the measuring tool), SF1 and SF3 are built (2026-11-17), and SF2 (2026-11-18).

The engine lanes start after the world rebuild. Cain's rule (2026-11-14): nothing in the engine is touched before the next world builds are done (`docs/design/engine-fix-list.md`).

## How it was measured

- **Paired A/B tests on the match engine.** Team 0 of a 32-team synthetic league is set to the level's typical team (`change_one`). Every arm plays the same games on the same seeds, in 40 blocks. An arm adds +15 to one rating of one man, or changes his body, in one context: the offense's scheme or the defense's shell, some with forced personnel (`s.guide.personnel`). A star (\*) means the change is past the noise (|z| at least 1.96).
- **As recruited.** The generator's own men at the same overall, four drawn per band, stand in for the realism test against real slopes. Those real slopes are held for draft slot.
- **Repeats.** FBS seeds 151 and 171, a full NFL sweep on seed 161, and world seed 1's NFL rosters.
- **One limit.** At the NFL, +15 is clipped at 99 on 51 of the typical starter's ratings, so those arms only move +11 to +14.

## The items, as agreed

Where Cain's answer is marked "by the floor", the item only matches real numbers, fixes a bug or follows one of his own rules. Under his rule that realism is the floor and only real choices are asked, it was marked yes without a question. He got the list on 2026-11-17 (00:28 UTC) to veto any of them. If he vetoes one, this file changes before its lane starts.

### Passing game

**1. A personnel preference fits the play.** Cain: yes, after his note: "personnel is a preference but scheme should still decide the play". Nothing is forced either way: on 3rd and 1 in their territory, a 13 personnel group must not force a pass.
- **The rule.** The situation and the scheme decide run, pass or play action first, as today. Then the group is picked to fit that play, with the plan's preference as a lean.
- **Today.** A firm preference is applied without looking at the call (`calling.rs` `pick_personnel`: the guide's want replaces the group with chance 0.55 × its firmness, 47% for a firm want, on snaps that aren't hurry-up or short yardage). So a power team set to 13 throws 45% of the time from 13 (real 36%) and 34% from 12 (real 46%).
- **After.** 13 comes out mostly on runs and play action, and a straight dropback more often from 12 or 11. A creative caller still throws from heavy sets to break his tendencies (the tell).
- **Checks.** The real pass rates by group (11: 65%, 12: 46%, 13: 36%, 21: 40%) are only the season check, never a rule on a snap. Short yardage and the hurry-up keep their own picks.

**2. Tight ends past TE1 run routes.** Cain: yes, "a gamebreaking miss", because backup tight ends get no receiving stats to grow on.
- **Target.** TE2 and TE3 get the routes real offenses give them. 13 personnel throws about half its targets to tight ends (real 56%, engine 23%), and 12 personnel a third (real 33%, engine 15%).
- **Today.** A tight end past TE1 blocks unless the concept names him (`playbook.rs` ~895, `route_for`). TE2 is named in 23 concepts and TE3 in none.

**3. Play action that works.** Cain: yes, "result breaking", because it hurts every pocket passer.
- **The fix.** The line shows run action, the fake reads as a run to the defense, and the boot goes away from the flow with protection built for it. Once it pays, coaches call it at the real rate.
- **Targets.**
  - Value: +0.12 EPA a dropback over a straight drop (engine -0.25).
  - Sacks: 6.6% of play action dropbacks (engine 24%), and 5.7% on boots and rollouts (engine 48%).
  - Call rate: 21.2% of dropbacks (engine 6.0%).
- **Today.** The boot keeps the standard five-man protection. The QB fakes straight back and then sets 7 yards toward the strength side, where his tackle is already blocking (`scrimmage.rs` ~809-833, ~4031).
- **After it.** Ball Handling and Throw on Move should come alive, so re-measure them.

**4. Eye Manipulation moves the safety off the throw.** Cain: yes, with the safety's counter.
- **The duel.** The QB's Eye Manipulation goes against the safety's read of the routes (Play Recognition, and Football IQ with item 12) and his read of the QB's commit (Anticipation). The eyes-on-the-QB flaw (`EyesOnQb`) makes him bite more.
- **Today.** The look-off on the first read pulls the deep defender toward where the QB is looking (`scrimmage.rs` ~4143 sets it, ~6000-6011 moves him), and the first read is often the throw. The bite already has the right shape (`clamp(1 - 0.01 * (Anticipation + stuck - 60), 0.3, 1.5)`), but it pulls toward the throw.
- **Result today.** Eye Manipulation +15 scores 0.37 fewer points a game at FBS and 0.09 more at the NFL (not past the noise).
- **After.** A real look-off holds the safety, or moves him off the throw.

**5. Decisions makes a better passer, not just a careful one.** Cain: yes, keeping the split.
- **The split.** Football IQ stays the pre-snap and protection read: blitz ID, the hot read and assignments. Decisions becomes picking the right throw.
- **Today.** Decisions +15 cuts picks by about 9% at both levels, but he throws 0.4 to 0.5 yards shorter, holds the ball a little longer and scores no more (FBS -0.29, NFL -0.08, neither past the noise). The note on `DECISIONS_TRUST` (`scrimmage.rs` ~3776, minor bug 283) describes this exact trade as the problem it fixed, and the trade is still there.
- **Target.** Fewer picks without losing depth, and more points.

**6. Route running and hands drive a receiver's results.** Cain: yes, "absolutely, that diversity is pivotal". The item was widened to his ask in the progression thread that teams value catching and route running more ("okay then both"). The draft side and the supply of very fast men belong to that thread's lanes.
- **After.**
  - Route running beats zone too: leverage, timing, finding the window.
  - Hands decide drops and contested balls at their real size.
  - Speed pays where real speed does (deep balls, after the catch), not in separation.
  - Height pays in contested catches and the red zone.
- **Today.**
  - With air yards and the defender's coverage held, Route Running +15 adds 0.47 yards of separation against man and takes 0.33 away against zone. Zone is 80% of targets.
  - Against man it works through the defender's trail at the break (`scrimmage.rs` ~2768). Against zone its only reads are the room he drifts to when he sits down (~3269) and the speed he keeps through the cut (~3405).
  - So X receiver Route Running +15 adds 105 yards a season (NFL 155) and no points. Catching +15 adds 0.4 to 1.1 points of catch rate on throws to him and no points.
  - Speed +15 adds 0.15 yards of separation and 0.65 points a game at FBS, and Acceleration +15 adds 0.67 at the NFL.
- **Real.** Receivers' separation has a true spread of 0.42 yards between players and no link to the forty (Next Gen Stats 2018-25).

**25. The quarterback feeds his receivers by their receiving.** By the floor.
- **Today.** The first look goes to the man rated best at his own spot in the progression: 10% plus 2% a point of edge, up to 55% (`scrimmage.rs` ~888-916; `FEED_BASE`, `FEED_PER_POINT` and `FEED_MAX` ~5020-5033). A back's spot rating is his overall with a lean on Vision, Elusiveness, Speed and Balance (`spots.rs`, `lean`).
- **What that does.**
  - Any of RB1's running ratings at +15 raises his targets per route by about half (FBS 11 to 17 per 100 routes, NFL 15 to 21).
  - At the NFL a better runner scores 0.3 to 0.5 points a game less, because a back's target is worth 4.6 to 5.6 yards and a wide receiver's 8.3 to 8.6.
  - A tight end's run blocking feeds him as much as his hands do.
- **Target.** Hands, routes, getting open and the concept decide who is fed, each man read against his own position's typical starter. A running or blocking rating moves a man's targets little.

### Run game

**7. A blocked man is out of the play.** Cain: yes, in the first lane.
- **The rule.** A defender in a block he isn't winning can't tackle the runner, and he has to shed first. One who is beating his blocker can still get an arm out.
- **Today.** A blocked man can tackle whenever the block is anything short of clearly the blocker's (`sim.rs` ~2992-3001). He is skipped only when the block's `w` is below -0.2; otherwise he gets a 0.25 + 0.3 * `w` chance within reach - 0.55. The runner also counts him as 0.15 to 0.5 of a free man when he picks his lane (~2611).
- **What that does.**
  - The first tackle on 34% of designed runs is made by a man who never shed his block.
  - Run Blocking +15 on any lineman wins 7 to 12 more blocks per 100 but moves yards a carry by only 0.00 to 0.05.
  - Block Shedding +15 on a defensive tackle sheds 10.9 more per 100 and moves nothing in the run game.
- **Target.** The league's run game stays where it is (first tacklers by position are about real today). The line then explains real-sized gaps: backs' yards before contact have a true spread of 0.36 a carry from one season to the next (PFR 2018-25).
- **Next to it.** Fix list E1 (leg drive for linemen) sits next to this item, and it stays Cain's call.

**8. Short yardage at the real rate.** Cain: yes.
- **Target.** On 3rd and 4th down with 2 or less to go, designed runs convert 67% for backs and 79% for QBs (2024 NFL). The engine converts 80% and 89% at the NFL, and 83% and 92% at FBS.
- **Cause not traced.** The fix comes from the cause, never a cap on conversions. One lead, not proven: outside its own 3 the defense has no short-yardage package (`calling.rs` ~1688: goal-line personnel only inside the 3 with two or fewer receivers out).
- **After item 7.** Measure this again after item 7, which changes how blocks decide runs.

**9. Breakaway runs to real.** By the floor.
- **Target.** Backs' designed runs of 20+ yards are 2.2 to 2.4% of real carries (engine NFL 1.1%, FBS 1.3%, a typical RB1 0.9 to 1.2%). Runs of 10+ yards are 10.6% (a typical RB1 7.8 to 8.9%).
- **Then a burner's speed is worth what it really is.** As recruited, NFL backs 0.22 s faster have 0.5 to 0.6 points more of their carries go 20+. Real backs that much faster gain +1.06 points (band 0.52 to 1.61, nflverse 2018-25). Per carry the gap is already real: +0.31 yards, against a real +0.28 over expected.
- **Cause not traced.** Never a boost on long runs. The line in `docs/match-engine.md` (10+ runs 3.6%) is out of date.

**10. The roster shapes every call.** Cain: yes, widened in his words: "the roster should always determine playcall". Every call, run or pass, leans to what his own players win.
- **Runs** lean to the back, the line and the tight ends: power, counter and iso for a power back, toss and outside zone for a burner.
- **Passes** lean to the arm, the receivers and the protection: more shots with a big arm and burners, more quick game behind a shaky line, more of the concepts that get his best receiver open.
- **How fit is measured.** How well each call fits comes from what the engine rewards for it, measured after the physics lanes.
- **What still counts.** The situation, the scheme, the defense's looks and today's results. A sharper caller leans harder, and he still mixes in the rest so his tendencies don't give him away.
- **Today.**
  - `pick_run` (`calling.rs` ~1001, `run_weights`) reads the scheme, the short-yardage cuts, the promises, the what-beats-what table against the fronts seen and today's results, never his own players.
  - The pass call (`concept_weights` ~1257) reads only a burner (deep concepts weigh 1 + 0.3 * burner, ~1384) and how much the QB runs.
  - The engine already rewards the back's fit a little: a burner gains 0.5 to 0.7 yards more on a toss at the NFL, and a heavy back 0.3 to 0.55 more on counter, iso and power at FBS.

**11. AI teams use situational backs.** By the floor.
- **Target.** A short-yardage back and a third-down back, where the roster has one.
- **Today.** AI backs rotate on fatigue only (`flow.rs`, back threshold 0.60). Only your own packages change who's in.

### Coverage and the ball

**12 to 14. A free safety's Football IQ, Anticipation and Ball Tracking make him a ballhawk.** Cain: yes ("a good FS should open up the playbook for a defense"; the rest of his note is in item 23). The three were asked as one question, with this split:
- **12. Football IQ reads the concept.** He knows where the routes are going, passes them off and carries the seam, so he is in the right spot before the throw. Today Football IQ is read for a defender only on busts (`scrimmage.rs` ~673) and in a rusher's choice of move (`sim.rs` ~1851).
- **13. Anticipation reads the QB.** He breaks when the QB commits, before the ball is out. This is his side of item 4's duel, so breaking early on a fake burns him. Today route jumping is only for underneath zone and man defenders (`!deep`, `scrimmage.rs` ~6013), and a deep man's Anticipation leans him just 0.004 a point toward the QB's eyes (~5994).
- **14. Ball Tracking reads the ball.** From depth he judges the ball's flight and gets to the catch point. Today Ball Tracking is never read for a defender, only for receivers (~4939) and returners. This item takes in the fix list's "Ball tracking for corners and safeties".
- **What stays.** Zone Coverage and Play Recognition keep their jobs: holding his area, and how fast he reacts to the throw (~4910).
- **Today.** +15 in any of the three adds no picks at either level (-0.1 to +0.04 a season, all inside the noise). Zone Coverage +15 adds 0.5 to 0.8 picks.
- **Real** (2024 NFL play-by-play with FTN coverage charting, 387 picks). Safeties make 39% of all picks, 43% behind Cover 1, 44% behind 2-man and 40% against five or more rushers. The engine gives safeties 23% behind Cover 1, 21% behind 2-man and 22% in man overall.

**15. A deep defender can play the ball from depth.** By the floor.
- **Today.** A pick at the catch point needs the defender closer to the ball than the receiver (`scrimmage.rs` ~6259), which a man coming from depth rarely is.
- **Target.** 30+ yard throws picked 6.7% of the time (engine 3.5%). Safeties make 39% of all picks and about 43% behind man (engine 22%). The league's total stays where it is.

**16. A rusher in a block doesn't pick passes through the lane.** By the floor.
- **Today.** Linemen make 16% of the engine's picks (real 4%, plus 5% by 3-4 outside linebackers), and 175 of their 195 came while they were engaged in a block.
- **Cause.** The lane check (`scrimmage.rs` ~6086-6124) lets any defender within 1.1 yards of the ball's path touch it, and a rusher faces the QB, so he counts as reading him. That double-counts the batted-ball model, which is about right on its own (1.7% of throws, real 2.0%).

**17. Breakups by linebackers and defensive backs to real.** By the floor.
- **Target.** 8.2 per 100 throws, picks left out (engine 4.2).
- **Cause not traced.** The levers are the catch point's gate (within reach and reacted, `scrimmage.rs` ~6239) and its 0.39 base (~6280).

**18. Man Coverage keeps paying at the top.** By the floor, and by Cain's rule never to cap freaks.
- **Today.** The trail a man defender plays behind his receiver's break falls 0.03 s a point and stops at zero (`scrimmage.rs` ~2766). Against a typical starting receiver it reaches zero 3 to 5 points above a typical starting corner, and past that a point only counts at the catch point.
- **Real.** Corners' completion rate allowed has a true spread of 4.7 points (PFR 2018-25, 30+ targets).

**19. A blown coverage leaves a man free.** By the floor.
- **Today.** A bust only reshapes the defender's job: his zone moves 5 yards shallower and 40% narrower, or a man defender plays a hook zone (`scrimmage.rs` ~2488). A Mike with Football IQ -15 busts 16.2 times a season instead of 7.4, and his defense allows no more points.
- **Real.** A blown coverage is a receiver running free.

**20. The strong safety's depth and fits are the coordinator's call.** By Cain's rule that the coach picks the job and the engine reflects the player (2026-11-07).
- **Today.** The engine judges how much of a hitter a safety is from his ratings (`scrimmage.rs` `hit_read`: Tackling, Strength and Play Recognition against Speed, Zone and Man Coverage). So a better cover man plays the run deeper and later: strong safety Man Coverage +15 costs 0.7 to 0.9 run stops a season at both levels.
- **After.** The call sets where he plays, and his skills decide how well he plays it.

### Front and pressure

**21. Blitzing decides the blitz.** By the floor.
- **Target.** Blitzing decides how fast a blitzer gets home and whether he beats the back's pickup. Real off-ball linebackers get 0.140 pressures a blitz, with a true spread of 0.051 (PFR 2018-24), so a good blitzer gets about a third more.
- **Today.** It is read only for who goes (`lb_blitzer`) and for the pickup (`picks_up`: 85% at 60, -0.8 points a rating point). Mike Blitzing +15 behind man pressure changes pressures by -0.1 a season (not past the noise). A blitz that gets home rides on Speed.

**22. A quick 3-technique's wins become pressure.** By the floor.
- **Today.** Finesse Rush +15 on a defensive tackle wins more reps (+2.3 per 100 rushes) but gets fewer pressures: -24% at FBS and -30% at the NFL.
- **Cause.** An engaged rusher drives the pair toward the QB at 1.4 times the speed on a power move and 0.7 otherwise (`sim.rs` ~2445). A pressure is the nearest rusher inside 3.2 yards after 0.6 s (`scrimmage.rs` ~4007). So interior pressure comes almost only from the bull rush: Power Rush +15 adds 12.8 pressures a season at FBS and 20.1 at the NFL.

### AI staffs and fit

**23. AI staffs lean personnel and identity toward their roster.** Cain: yes, through his answer on item 10.
- **The rule.** Personnel, the man and pressure lean, the blitz lean and the QB's run calls all read the roster. Three good tight ends mean more 12 and 13, a room of man corners more man, a fast QB more designed runs.
- **Cain's addition** (2026-11-17, on items 12 to 14): "a good FS should open up the playbook for a defense". A ballhawk free safety who carries the deep middle lets the coordinator call more single-high man and pressure, and fewer three-safety or three-deep looks. Without one, the defense plays more zone and nickel: "the Lions defense was mainly man coverage purely because they had Kerby to back them up, without kerby they started incorperating way more zone and nickle".
- **Today.** Personnel comes from the scheme's table, the man and pressure lean from the head coach's background, and the blitz lean from linebacker and corner weight only. A spread option QB keeps the ball 3.6 times a game whatever his speed.

**24. Fit scores from what wins in the engine, job by job.** Cain: yes. It goes last, after the rest.
- **The fix.** Refit the world's fit scores (`world/systems.rs`, `score_by`) from measured value per scheme and job, so installs and recruiting chase what wins.
- **Job by job.** Free and strong safety apart, Mike, Will and Sam apart, outside and slot corner apart. Today there is one score per position (S, LB, CB).
- **Today.** 74% of the weight sits on ratings whose +15 didn't move points in that same scheme (NFL 77%). For example, a safety's fit for man and pressure is Man Coverage 0.5, Speed 0.3 and Tackling 0.2, with nothing for the deep man's range or ball skills.
- **Why last.** Measured on today's engine, the scores would chase today's bugs.

## The lanes

Each lane is its own fresh thread. The coordinator starts them in the order below, and each one merges to main on green and sends one final reply (an agreed sequence).
- **Order.** The order is Claude's, from the report's pick. Cain called items 2 and 3 game and result breaking, so they go first with 7, and he agreed that 24 goes last.
- **Models.** Every lane runs on Opus at max effort (Cain, 2026-11-17).

| Lane | Items | Starts after | Model | Changes |
|---|---|---|---|---|
| SF0 The measuring tool | none | now | Opus max | no results (tooling) |
| SF1 Play action | 3 | world rebuild, SF0 | Opus max | game results |
| SF2 Personnel and tight ends | 1, 2 | world rebuild, SF0 | Opus max | game results |
| SF3 Blocks decide runs | 7 | world rebuild, SF0 | Opus max | game results |
| SF4 The QB's reads and the receivers | 5, 6, 25 | SF2 | Opus max | game results |
| SF5 Where picks and breakups come from | 15, 16, 17 | SF1, SF2, SF3 | Opus max | game results |
| SF6 The ballhawk safety | 4, 12, 13, 14 | SF5 | Opus max | game results |
| SF7 Coverage jobs | 18, 19, 20 | SF1, SF2, SF3 | Opus max | game results |
| SF8 The pass rush | 21, 22 | SF1, SF2, SF3 | Opus max | game results |
| SF9 Short yardage and long runs | 8, 9 | SF3 | Opus max | game results |
| SF10 The roster shapes the call | 10, 11, 23 | SF4 to SF9 | Opus max | the calls, AI identity |
| SF11 Fit scores job by job | 24 | SF10 | Opus max | the world (installs, recruiting) |

**Engine gaps.** SF10 also takes item 10 of the engine gaps (designed quarterback runs). Lane EG6 (the red zone) starts after SF7, and EG7 (the quarterback off script) after SF4 and SF8 (`docs/design/engine-gaps.md`). Some SF lanes share code with an EG lane running beside them: SF5 shares the catch point with engine gaps item 27 (pass interference, EG5), SF9 the open field with item 13 (return breakaways, EG3) and the run paths with the rest of S6.2 (EG4), SF4 the routes with S6.2's short outs (EG4), and SF4 and SF5 the red zone with item 8 (EG6). Whichever lane merges second measures the other's numbers again (an EG lane with `fit_check`, an SF lane with `AFM_GAPS=1 match_check`).

**SF0, the measuring tool.** It commits the audit's tool to `afm-tools` as one bin, `fit_check`, so every lane measures the same way.
- **Source.** Copies of the scratch tools are in the project folder, `afm/scheme-fit/tools/`: `fit_pass.rs.txt` for the arms and contexts, `fit_draws.rs.txt` for the as-recruited draws, `fit_wroster.rs.txt` for the world's NFL rosters, and `fit_pass.py.txt` with its helpers for the read-out.
- **What it keeps.** The contexts (`wc`, `ar`, `wz`, `pg`, `so`, `pg12`, `pg13`, `wc21`, `q`, `mp`, `c34`, `c2`), the typical team through `change_one`, paired seeds, 40 blocks, four drawn men a band for as recruited, and world rosters through `AFM_AB_WORLD`, like `ab_check`. Its read-out prints the same tables as the report.
- **Checks.** A quick test that a tiny run works. It adds a line to the commands in `CLAUDE.md`. It changes no results.
- **Built** 2026-11-17 (`crates/afm-tools/src/fit/`, `bin/fit_check.rs`). `cargo run --release --bin fit_check -- run fbs` plays every arm on FBS seed 151 at 3,000 games an arm, `run nfl` the NFL on seed 161 at 2,400, and `run fbs 3000 171 <out> repeat` the audit's repeat set on seed 171 (`AFM_AB_WORLD=<world file>` with `nfl` plays a world's clubs). `fit_check read <out>` writes the report's tables to `<out>/report/`: one file per job, `effects.csv`, `_items.md` (each item's own measures beside its real target), `_recruited.md`, `_personnel.md`, `_alignment.md`, `_unread.md`, `_weights.md` and `_compact.md`. `fit_check repeat fbs151=<out> fbs171=<out> nfl161=<out>` prints the repeat table and each run against the first on points. Beyond the audit's slots it keeps each item's own measures (play action and the boot, targets in each personnel group, the catch against man and zone, the first tackle on runs and whether it came from a block, short yardage, the backs' long runs, picks and breakups by depth and coverage), so a lane reads its items in one place.
- **Checked** 2026-11-17. On the same games it writes the same block sums as the audit's scratch tools, slot for slot (44 arms in all 12 contexts on FBS seed 151 and 11 on world seed 1's NFL rosters), and its read-out gives the audit's effects to the last digit. Built on the audit's own commit, it gives the report's raw numbers again (FS Zone Coverage +15 in Cover 3: +0.52 picks a season at FBS seed 151). One fix on the way, in the shared as-recruited draw (`ab::recruit`, which `ab_check` uses too): a drawn man's band is checked on the body he plays with. The game knows him by the typical man's id, and a college body's fill offset is drawn from the id, so 22 of the 48 FBS draws had played outside their bands, up to 18 lb (a light defensive tackle drawn at 270-274 lb played at 292). Now none do. The NFL's draws are the same as before, since a grown man's offset has faded. At FBS three light bands now find no man (the lightest edges and tackles), and the read-out names them.

**SF1, play action (item 3).**
- **Built** 2026-11-17 (`game/scrimmage.rs`, `sim.rs`, `calling.rs`, `playbook.rs`, `sheet.rs`, `tables.rs`; how it works in `docs/reference/how-it-works/match-engine.md` and `play-calling.md`, "Play action"; the measures in `docs/match-engine.md`, "Play action that works"). Cain's pick (2026-11-17): a fake on any dropback play. Any dropback concept but the RPO halves can carry the run fake, screens too, at the real NFL habit by down, distance, score, clock and field (FTN charting 2022-2025), leaned by the scheme, your word to a player and the coaches' table. The back meets the quarterback at the mesh and carries the fake into the line; every defender who keys the backfield plays the run until his own read time; the line half-slides with the back in it; on the boot a guard pulls to the end man and the quarterback keeps rolling; a passer can throw hot off the fake once his eyes are back. The coaches' table has a faked row for every concept that can carry the fake; its straight pass rows wait for minor bug 405. Sizes proposed, not confirmed.
- **Measured.** `fit_check`, main with SF3 merged and after (the typical team, West Coast; FBS seeds 151 and 171, NFL 161): play action 3.3-4.2% of dropbacks to 20.7-21.7% (real 21.2%), sacked 19.1-21.9% to 5.0-5.2% (real 6.6%), the boot sacked 39.7-51.3% to 1.8-2.4% (real 5.7%), play action over a straight drop -0.41, -0.37 and -0.20 to -0.05, -0.04 and +0.04 EPA a dropback (real +0.12). The league (`match_check`, 2,000 games a level, every team on its own staff): play action 7.9% to 19.6% of NFL dropbacks and sacked 22.9% to 4.7% (FBS and high school alike: 7.1% and 8.7% to 19.9%, sacked 20.7% and 27.2% to 5.6% and 5.7%), worth -0.09 to +0.19 EPA a dropback, +0.08 over a straight drop (FBS -0.28 to +0.07 and +0.01, high school -0.42 to +0.07 and +0.17). Seed 181's 32 clubs: 6.8% to 16.2%, the gap -0.18 to +0.10. Ball Handling and Throw on the Move (+15) move points about as chance would: one cell of fifteen is past the noise (Throw on the Move, West Coast, FBS seed 151, +0.22 ± 0.10). Sacks fall to 1.64 a team game on the NFL test league (real 2.41), points rise 0.9 (24.7 to 25.6, real 22.5), and every total stays inside its test band. match_check is about 5% slower. The tables and the whys: `docs/match-engine.md`, "Play action that works".
- **Left open.** Straight drops are sacked short of real and are worth too much, most of all on 3rd down, so the gap is +0.08 at the NFL and +0.01 at FBS (real +0.12): the pass rush and the coverage, lanes SF8, SF5 and SF7. Minor bugs 409-412, the play action leftovers, fixed 2026-11-18 (`docs/match-engine.md`, "Play action that works", left open): the boot, the shot and the dig keep their own edge in the table, straight drops line up in the gun, the scramble drill paces a passer by his speed, and a faked throw is play action on the film and for busts.

**SF2, personnel and tight ends (items 1 and 2).**
- **Built** 2026-11-18 (`game/calling.rs`, `Groups::of`, `Personnel::heavy_fit`, `sticks_depth`, `rpo_chance`, `sheet_fake_chance`; `game/playbook.rs`, `stand_in`, `route_in`, `lands_on`, `reads_in`, `readable_library`, `off_line`; how it works in `docs/reference/how-it-works/play-calling.md`, the measures in `docs/match-engine.md`, "Personnel that fits the play and tight ends who run routes"). On SF1's call, which decides run, pass or play action first, the group fits the play: each wide receiver a group trades for a blocker leans it toward runs and throws off a run fake by the caller's tell, and away from straight dropbacks by the same, sized so the NFL's typical caller fits his groups about as real teams do on first and ten. The head coach's group, or the one your word needs, is a lean (its firmness × the scheme table's whole weight, before the fit), never a swap. The second and third tight ends take the places in the reads of the receivers their group lacks, in the concept's read order: in the slot's place the slot's route when it is a tight end's own (the seam, the corner, the drag), else the concept's own short job for him (or the slot's, where the concept draws him none); in the flanker's place the flanker's route when he lines up on the flanker's side, his own when he lines up across the ball; across the ball from the slot's place, the slot's route only when it works from either side; and his own job wherever the borrowed route would finish on top of another receiver's. The split end steps off the line beside an inline tight end, so the tight end is the end man and eligible. Third down's check of the routes that reach the sticks reads the group's own routes, an RPO throws only to a man its group puts on the field, and a call sheet's throw fakes as often as the calls that draw its look. Your call sheet in the CFL or arena football is drafted on that league's rule book.
- **Measured** with `fit_check` on the audit's seeds against SF1's head 73ef2b7: a power team set firmly to 13 throws on 37% to 40% of its 13 snaps (41% to 46% before, real 36%) and 38% to 43% of its 12 snaps (34% to 39%, real 46%), and its tight ends draw 57% to 63% of 13's targets (28% to 32%, real 56%); set to 12, 31% to 32% of 12's (18% to 19%, real 33%). On first and ten the NFL's typical caller brings 12 and 21 out on straight dropbacks 0.47 times as often as 11 for their share of the runs and fakes, 13 0.34 times and 22 0.19 (real 2024: 0.46, 0.47, 0.34 and 0.20). League-wide with every team set to 13, the tight ends draw 55% of 13 personnel's targets at the NFL and 59% at FBS (31% and 35% before, real 56%) and the backs 16% and 15% (30% before, real 17%); set to 12, 33% and 31% of 12's (21% before, real 33%), the second tight end about half of them (almost none before, real 43%). A backup tight end has a line of his own: set to 12 the second draws 2.50 to 2.67 targets a game (0.51 to 0.94 before), set to 13 the second 3.31 to 3.71 (0.86 to 1.79) and the third 1.29 to 1.94 (0.04 to 0.07), and Catching +15 on any of them adds 0.15 to 0.32 targets a game and 24 to 50 yards a season, past the noise on every seed (the third's is the repeat table's claim: +32, +27 and +37 yards a season, against +0.6, +0.6 and -0.2 before). The league's totals hold (`match_check`, 4 seeds at each level), over SF1's head and again over main 349eefb, which has engine gaps items 29 and 30 (there points +0.22, +0.20 and +0.03 a team game at the NFL, FBS and high school, and the tight ends' share of the targets up 2.1, 1.6 and 2.5 points, the same as over SF1's head), and the coaches' table doesn't move (`call_values` writes the same file as on SF1's head). `match_check`'s time against SF1's head, two rounds of 2,000 games on four seeds: +2.3% to +3.0% at the NFL and FBS, whose teams draft the kickoff sheet (at FBS the plays themselves +0.4% in instructions), and -1.5% to +0.8% in high school, which drafts none.
- **Proposed, not confirmed:** every size (the fit's 1 ± 2 × tell and the heavy powers, the lean's firmness × the table's weight, the split end's 1.2 yards off the ball), the routes a tight end runs in a receiver's place, and the sheet's fake read back from its look by Bayes' rule (decided under Cain's proxy on 2026-11-18, with the split end off the line, snag's sit and the RPO's throw on the field).
- **Left for other lanes:** a better tight end by any rating is fed more and scores no more, and in 13 personnel the second tight end out-draws the first because he takes the flanker's place, which many concepts read first (SF4, items 5, 6 and 25); tight ends' targets run 1.1 to 3.1 air yards deeper than real at every rank (the routes' own depths). Small bugs found on the way, each its own fix: the play designer counts a man a yard off the ball as on the line (minor bug 416), a group with no slot can call the bubble on the split end alone (417), and the call's groups don't read the down and distance (418).

**SF3, blocks decide runs (item 7).**
- **Built** 2026-11-17 (`game/sim.rs`: `SHED_EDGE`, `ARM_OUT` and the runner's read in `run_carrier`; how it works in `docs/reference/how-it-works/match-engine.md`, the measures in `docs/match-engine.md`, "A blocked man is out of the play"). A man on a block tries for the runner only while he is beating his blocker, 0.55 × w a try, so one in a block he isn't winning has to shed it first. As the runner comes, the shed off a block reads who is winning it: the pull is scaled by e^(edge / 9), so a man ahead of his blocker by 9 points comes off 2.7 times as hard as on an even block, and one behind by 9 at 0.37 of it. The runner counts a sealed man as no threat. The coaches' call table's run rows are rebuilt on it; its pass rows wait (minor bug 405). Sizes proposed, not confirmed.
- **What the 34% was.** The audit's count reads the play log's block records, which also hold a man put down earlier or picked up by a blocker after his hit (minor bug 404, left open: the fix changes the pile). Counted at the hit, 4.9% of the hits that start the tackle on NFL designed runs came from a man on a block and 0.6% from one he wasn't winning; now 2.2% and none. The lever was the shed as the runner comes, which read nobody: the line's edge moved backs' yards before contact 0.030 a carry a point on the NFL test rosters, now 0.065. `fit_check`'s item 7 row is labelled for what it counts.
- **Measured.** `fit_check`, main and after: Run Blocking +15 on a lineman moves yards a carry +0.02 to +0.14 at FBS seed 151 (-0.00 to +0.08 before), +0.02 to +0.15 at FBS seed 171 (-0.03 to +0.07) and -0.01 to +0.15 at NFL seed 161 (-0.02 to +0.04), past the noise on 8, 10 and 10 of 13 arms (5, 5 and 0 before); Block Shedding +15 on a defensive tackle -0.08 to -0.12, -0.05 to -0.11 and -0.06 to -0.13 allowed (-0.03 to +0.01 before), on all 5 arms in each. After coach fire merged, FBS seed 151 again: a lineman's +0.01 to +0.15 (11 of 13 past the noise; -0.02 to +0.07 and 3 on that main), a defensive tackle's -0.08 to -0.12 allowed (5 of 5). The back's Speed and Ball Carrier Vision keep what they had (Vision's wide zone arm moves about 0.04 between builds on its own). The league's run game stays: NFL test rosters 4.45 to 4.50 a carry (real 4.50), first tackles by the line 43.8% to 42.4% (real 43.1%). On world seed 1's NFL clubs the backs' yards before contact keep a lasting spread of 0.37 a carry over 204-carry blocks (0.32 before; real 0.36 between seasons), and the points spread and close games move to real (points sd 9.6 to 9.9, real 9.9; games within 8 56.6% to 52.9%, real 53.5%). The test rosters are wider apart than the world's clubs, so there the spread runs past real (backs' yards a carry 0.68 over 204-carry blocks, real 0.57). Short yardage (item 8) barely moves (NFL backs 77-78% on 3rd and 4th and 2 or less, real 67%).

Every game lane:
- reads this file, the report's sections for its items (`afm/scheme-fit/full-pass.md`) and Cain's answers above, and follows his wording;
- fixes root causes only: no caps, no flat boosts, no fudge on a total, and never a cap on freaks; each rating is read against the level's typical starter;
- measures its items before and after with `fit_check` on the audit's seeds (FBS 151 and 171, NFL 161);
- checks the league's totals against real with `match_check` at the NFL and FBS (and high school where its change reaches those games); a total that leaves its real band is fixed in the lane, not left for later;
- reruns `call_values` when its change moves what a call gains (the coaches' table, `crates/afm-engine/data/call_values.csv`), so coaches see the new values; SF10 reruns `xcomp_fit`, `xyards_fit`, `xpass_fit` and `decide_fit` once the physics lanes are in;
- updates `docs/match-engine.md` and the how-it-works pages it touches (`docs/reference/how-it-works/`) in the same PR, and bumps their commit lines;
- carries a `## Patch note`, because players see the change;
- reports `match_check`'s time against main, so a slower engine is never a surprise;
- rebases and measures again before it merges, if another engine lane (SF or EG) merged first.

**SF11** changes the world: programs install and recruit by fit. So it checks a world against real (installs, fit by school and class, recruiting), and its cost is asked first.
