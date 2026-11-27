# Coordinator-grade game planning (section 12)

Design done 2026-11-27. Cain set two rules on 2026-11-27. Topic 1's payoff size: "agree". Then: "any topics that simulate realism agreed". So every topic below takes the realistic option. Claude wrote topics 3-13 under that rule, and Cain hasn't read them one by one yet. Every size is proposed, not confirmed, until its lane measures it.

The research behind it is `docs/research/sequencing.md`. That covers the real numbers, the 105-entry series library, Opus's engine map and the two baselines: the coach review at 3.5/10 and the blind test at 0.578. The 16 items Cain agreed to are decisions section 35. This spec turns the engine map's 13 topics into a design.

## 0. Ground rules

- **The ground rule, in Cain's words (2026-11-24):** "outscheming can beat a more talented team sometimes, only through what happens on the field". The edge comes from where men stand, who is free, who is out of position and who has the numbers. It is never a bonus on ratings (item 11).
- **Its size comes from real upsets (item 12).** NFL favorites of 14+ win about 96%, and margins spread about 12.5 points around the line. A coaching edge is worth about 3 to 7 points a game. Coaches explain about 20 to 30% of the spread in results.
- **Scouting is a read with errors (item 13).** Good scouting alone never wins every game. AI coordinators break their own tendencies by their creativity. Defenders cheat by what they believe and by their Discipline.
- **An answer called too often becomes the new tendency (item 14).** The AI reads it like any other tendency.
- **Luck stays at real football's share (item 15).** Every loss has a reason you can find on the tape.
- **Every lane passes the scheme duel test (item 16).** A sharp caller plays a poor one, on even and uneven rosters, on the same seeds. See section 14.
- **The data beats coaching folklore.** Play action does not need a run game to work, and it does not fade with use. Linebackers bit as hard on the 11th fake of a game as on the 1st (NGS). No source gives a rep count for a set-up or for how fast it fades. Those sizes are the engine's guesses until the duel test and the real averages pin them.
- **The same rules everywhere.** The AI plays by the same rules as you, at every level. A level differs only through its men: high school defenders have less Discipline and less Play Recognition, so the same set-up pays more there.

## Topic 1. The defense remembers direction (agreed 2026-11-27)

Today the engine remembers run or pass by look (`calling.rs`, `Memory::belief`), and the defense leans on it (`scrimmage.rs`, `lean`). It does not remember which side the runs go.

- **What it remembers.** Runs by side, from this game and from film. Side means two things: the formation's strength side (the tight end or the trips side) and the field or boundary side. It remembers by the concept's family too: outside zone, inside zone, gap runs and boots.
- **Who leans.** The defensive coordinator's call can slant or stunt the front toward the side he expects. Each defender can also cheat on his own, before the snap or on his first step. How far he cheats depends on his Discipline (less Discipline, more cheat) and on how sure his read of you is.
- **The payoff (Cain, "agree"):** the real average, about +0.03 EPA a play. The spread around it is wide, depending on who you face. A young, undisciplined high school defense gets burned badly. A veteran NFL front barely moves.
- **Overdoing it gets read.** A pattern repeated too often (left, left, left, right, right) becomes a tendency a sharp coordinator reads (item 14), and then the cheat works against you.
- **Real targets:** runs outside the tackles succeed about 6% more often than inside runs. Team success against outside zone runs from 39.8% to 61.6%. Defenses add about 0.3 men to the box after three runs in a row (`docs/research/sequencing.md`, section 2).

## Topic 2. Defender keys and the cost of a false step (agreed 2026-11-27)

Today a defender who bites on a fake loses time, but not ground.

- **Keys.** Each linebacker and safety reads a key at the snap:
  - **Guard key:** a pull or down block means run; a guard setting back means pass. It is hard to fool with play action, and a step slow on outside runs.
  - **Back key:** fastest to normal runs, but it bites on counter and play action.
  - **Read-through:** guard to back, the middle ground. It reads Play Recognition, so only a smart linebacker plays it well.
  - **Safety key on #2:** the second receiver's release triggers the run fit or the pass drop.
- **Who picks.** Each defensive coordinator teaches a key by his scheme. You can set it per opponent in your game plan. Film shows which key a team plays. An AI staff can change its key at halftime if you're beating it (topic 12).
- **False steps cost ground.** A man who bites takes real steps the wrong way, and he gets them back only with his Speed and Acceleration. A deep man who steps up on the fake has to recover the depth he gave up against the route behind him.
- **Pulls are keyed.** A tight pull (power) and a wide pull (counter, sweep) read differently to a guard-key linebacker.
- **Real targets:** play action gains 7.4 yards a play against 6.3 for other passes. Its gain does not depend on run success or volume. Moving from one-high to two-high cut play action's DVOA from about 16% to under 6%.

## Topic 3. Base plays linked to their answers, and the defense's answers in series

- **Offensive series.** Each base play has answers that share its first beat: outside zone with its boot and its cutback, power with counter and power pass, inside zone with split zone and the slice. For the defense's first read, the answer looks like the base play. A series is part of a playbook, learned like any other play (plays and tactics topic 4).
- **When an answer works.** An answer works when the defender it attacks is doing what the base play taught him: cheating to a side (topic 1), keying the back (topic 2), flowing fast. Calling it does nothing by itself. Against a defender who didn't bite, the answer is just a play.
- **Defensive series.** A defense has its own set-ups: show a pressure look and drop out of it, play a run fit one way and then spill it, show two high and spin to one. The offense's coordinator reads them the same way.
- **The AI.** AI coordinators call answers when their own read says a defender is cheating. They also break their own tendencies, and their creativity decides how often. An answer called too often becomes the tendency (item 14).
- **Real targets:** nakeds were about half of one coach's play-action passes and about 30% of his passing yards. Cleveland booted on 50.3% of its play fakes. In 2020, 21 NFL teams averaged 2+ bootleg dropbacks a game.

## Topic 4. Read and conflict defenders

- **A named read man.** The zone read, the RPO and the option leave one defender unblocked on purpose. That man's first step picks the play.
- **The defense's answers:** a scrape exchange (the end squeezes, the linebacker takes the quarterback), a gap exchange, and a pre-snap call for the overhang defender. Each is a defensive call that costs something somewhere else.
- **The offense's answers:** arc, bluff and veer blocks that change who is read.
- **Real targets:** RPOs are about 9.5% of NFL plays, with 52.3% success against 48.1% for other plays. RPO passes produce +0.110 EPA and RPO screens -0.081. Read-option gains fell from 6.2 to 4.7 yards a carry between 2012 and 2013 as defenses adapted. Designed quarterback runs produce +0.15 EPA, against -0.04 for read-option handoffs.

## Topic 5. Gap-scheme blocking

- **Pullers.** Pullers run real paths with real timing. The kick-out block and the wrap play off each other, and so do the double team and its climb to the linebacker. The engine also gets the sift block, split zone's backside block, and the influence trap, where the trapped man is invited upfield.
- **The defense's edge technique** is a call: spill (wrong-arm the puller and push the run wide) or box (squeeze it inside). Each wins against some runs and loses against others.
- **What decides it:** the blockers' Run Blocking and Pull Blocking against the defenders' Block Shedding, Strength and Play Recognition.
- **Real targets:** defensive success against inside zone ranges from 30.6% to 56.0%. Running-back draws produce -0.02 EPA against -0.06. Inside zone converts about 68% in short yardage, against about 88% for the sneak.

## Topic 6. Self-scouting and tendency breakers

- **Both sides chart themselves.** The engine self-scouts by look today. Both sides will chart by down, distance, field zone, personnel, formation, direction and concept, the way real staffs do.
- **Breakers are chosen.** A tendency breaker is picked on purpose, aimed at the tendency an opponent will read. Today it comes by chance.
- **Your staff tells you.** Your staff's self-scout shows you your own tendencies (screens and game day, the tape's hints). The AI staffs read yours through their own errors (item 13).
- **Real targets:** run or pass is about 71 to 75% predictable from the situation alone, ranging from 60% to 78% by team. Most tendencies sit at 50 to 60%, and 80 to 90% is a lean an opponent plans around. NFL teams switch between run and pass more often than chance would. Fixing a team's run/pass mix was estimated to be worth more than half a win a season.

## Topic 7. Disguise, rotation and the quarterback's reads

- **What the defense shows and what it plays are separate.** The shell before the snap is one thing; the coverage after is another. Safeties rotate with real travel time, so a late spin from two high to one high costs depth somewhere.
- **The quarterback reads both.** He reads the shell before the snap and the rotation after it, by his Football IQ and Play Recognition. A quarterback fooled by the shell throws into the coverage he didn't expect.
- **Disguise has a cost.** A disguised call plays its coverage slightly worse.
- **Real targets:** coverage can be predicted about 75% of the time from pre-snap data, and 87%+ with post-snap movement. The NGS shell mix is about 50% single-high, 44% split and 6% Cover 0. The Rams disguised 50.5% of snaps and allowed +0.15 EPA when disguising, against -0.02 when not.

## Topic 8. Match rules and communication

- **Pattern-match rules.** Coverages follow real match rules: banjo and switch calls, rat and robber, MOD and MEG, the trigger on #2's release.
- **Two defenders have to talk.** Whether they get it right depends on their Football IQ, their Zone Coverage and their time together (the secondary's chemistry, `world/chemistry.rs`). A missed call is a bust with a real free man (scheme fit item 19).
- **Rubs and picks** create path conflicts, and the match rules answer them.
- **Real targets:** mesh produces +0.10 EPA a dropback against a league average of +0.03. Cover 4 allowed +0.12 EPA an attempt against Cover 3's +0.06 (2019). Usage in 2023: Cover 3 35.8%, Cover 4 14.2%.

## Topic 9. Pressure and protection

- **Shown rushers and real rushers are separate.** Simulated pressures, creepers, fire zones, mugged A gaps, stunts and games all play off that gap.
- **The protection is set before the snap** from the rushers the defense shows: the slide's side, the back's check, the hot (EG7). A dropper or a peeling rusher creates the free man and the answers.
- **Screens and draws** answer the rush. Screens get no boost against the blitz, because the studies conflict.
- **Spies** read the quarterback's legs.
- **Real targets:** on third down, stunts produced 41.8% pressure against 35.8% without. Blitz rate was 25.3% in 2021. One defense got 47.6% pressure with 5+ rushers against 27.8% with 4. Screens produce -0.08 EPA against +0.05 for other passes, and -0.15 against +0.02 when blitzed (2024). Screens are about 11% of dropbacks. Early-down run blitzes stop paying past about 25%.

## Topic 10. Time before the snap

- **Motion.** Each defender has a motion rule: run with him, bump, spin the safety or check the coverage. Motion and shifts force the defense to re-set, on a clock.
- **Tempo shrinks that clock.** It also stops substitution, so the defense keeps the men on the field.
- **Unbalanced lines and shifts** test the defense's alignment rules.
- **Audibles** come from the quarterback's read (topic 7).
- **Real targets:** motion at the snap is worth about +0.08 EPA on passes and +0.11 on runs. 49ers jet-motion runs were 46.2% positive against 39.1% without. League shift and motion use was about 50% in 2020, and the 49ers used it on 78% of plays. No-huddle rate shows no link to EPA. One quarterback audibled on about 8% of plays, and about 60% of those went to runs.

## Topic 11. Situations and the field

- **Geometry, not bonuses.** Answers work because of the end line, the sideline, the hash, the clock and the wind (EG6's red zone, EG4's clock, the weather).
- **The hash.** The field and boundary sides come from each level's real hash marks. They set where topic 1's memory points.
- **Two engine bugs go first**, as small fixes before the lanes (`GAPS.md`):
  - the clock barely runs between some in-bounds snaps (9% of in-bounds runs are followed by a snap 2 to 8 seconds later);
  - some throws land past the end line (1.4%).
- **Real targets:** the sneak converts 83 to 87%. In the red zone, runs succeed 46.2% against 42.1% for passes. Above 25 mph of wind, passing yards fall about 19% and points about 15%. Leaders up 7+ in the fourth average 3.29 yards a carry, against 4.18. Third downs convert about 60% at 1 to 3 yards and 20.3% at 10+.

## Topic 12. Adjustments, at halftime and in the game

- **Halftime is a checkpoint.** Real coordinators look at who got beaten and change a few things: a key, a coverage rule, a protection, a pressure. Today the engine adapts from today's EPA, without stopping.
- **A change budget.** Each change costs busts, because the men have to play something new without a week of practice. How many changes a staff can make without busts depends on its coaches' teaching and on the men's Football IQ. Bigger changes mean more busts.
- **AI defenses adjust.** They must answer a run that's gashing them (today they keep a two-high shell and a four-man rush, `GAPS.md`). Their changes are read from what beat them, through their own errors.
- **Your side.** Your halftime screen shows your staff's notes on what is beating you and what is working (game day topic 9). You pick the changes, or you let the staff do it.
- **Real targets:** teams score about 1.2 fewer points a team in the second half (a weak source). A first-half lead shows no significant effect on second-half EPA. The halftime effect's size is contested, so it stays small.

## Topic 13. Calibration: talent, scheme and luck

- **Every new battle must leave the real share of upsets and only a modest coaching edge.** Underdogs of 10+ points win about 8%, 14+ about 7.7%, and all underdogs about 33 to 36%. Week 1 underdogs win 38.3%, against 32.9% in other weeks. A good offensive coordinator is worth about +0.08 to +0.17 EPA a play (a blog estimate).
- **Measured on the world's rosters and the test leagues** with the duel test (section 14) and `match_check`, at each level.
- **Coaches' worth is checked again** after section 12, since play calling is worth more than it was (`dynasty_check`).

## 14. Measuring

- **The scheme duel test (item 16)** is the gate for every lane. A sharp caller (high play calling, adaptation and creativity) plays a poor one on the same seeds: once on even rosters, and once with the poor caller's roster a set amount better. The sharp caller should win more than the ratings say, by a margin inside the real coaching band (3 to 7 points). On uneven rosters he should win sometimes, not always. Part of the margin must come from the lane's own mechanism (one arm with the mechanism off). Built first as lane GP0, a `match_check`-style tool.
- **The blind test and the coach review** rerun after every lane on the same seeds as the baselines (`tools/research/coach_review.py`; blind test on seed 1 with real 2024 drives; reviews on seed 1 and seed 2). The blind score should fall toward 0.5, and the review scores should rise.
- **Real averages hold.** `match_check` at the NFL, FBS and high school after each lane. A new battle that moves points, yards or success rates off real is tuned in its own lane.
- **The tables are rebuilt** after each lane that moves what a call is worth: the call values, roster values and fit values, and the analysts' four models.

## 15. Lanes (proposed, not confirmed)

Each lane is its own thread. Calm mode (Cain, 2026-11-20) allows one engine lane at a time. The realism audit's engine chain (items 1 and 3 and the lanes after them) is running now, so section 12's engine lanes start after it. GP0 measures only, so it can start beside it.

| Lane | What | Topics |
|---|---|---|
| GP0 | The scheme duel test (tool, no engine change): built 2026-11-27, `duel_check`, baseline in `docs/research/sequencing.md` section 12 | 14 |
| GP-fix | The clock between snaps, throws past the end line (small fixes) | 11 |
| GP1 | Direction memory; keys and false steps | 1, 2 |
| GP2 | Series links; self-scouting and chosen breakers | 3, 6 |
| GP3 | Read defenders; gap-scheme blocking | 4, 5 |
| GP4 | Disguise and rotation; match rules | 7, 8 |
| GP5 | Pressure and protection; time before the snap | 9, 10 |
| GP6 | Situations; adjustments and the change budget | 11, 12 |
| GP7 | Calibration, the tables rebuilt, the world measured | 13 |

The screens (keys, match calls, pressure packages and disguise in the game plan; the halftime notes; the self-scout page) follow `docs/design/screens.md` and `docs/design/game-day.md`. Each lane exposes its settings through `Game::call` with the engine change, and the screens are their own lanes.
