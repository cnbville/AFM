# Section 3: the match engine and stats database (design done)

Status: design finished 2026-10-13. All ten topics are answered. Next, a fresh thread builds it (see the section loop in `docs/roadmap.md`). The first build step is athlete-first player creation (topic 5), before the match engine itself.

This spec is written so anyone outside the project chat can follow it. Raichel's own words are in quotes.

Labels:
- **Confirmed**: Raichel answered it.
- **Proposed, not confirmed**: Claude's draft. Ask Raichel before building on it.

See `docs/roadmap.md` for the whole plan and the working rules.

---

## 0. Ground rules for this section

**Ultra realism, unless it breaks the game (confirmed).** Raichel on topic 1: "it's so extremely important that we make it realistic, unless it breaks the game or the possibility of the game you'll always match ultra realism".

**One engine for every level (confirmed).** Every game in the world (high school, college, NFL) runs on the same full engine, including every high school game. There is no shortcut model for games the user doesn't watch. High school players get the same stat lines as college and NFL players.

**Log as much as possible (confirmed).** Raichel: "basically as much logging as possible". See topic 8.

**Outliers must be possible (confirmed).** Raichel: "i'm very insistant of having outliers be present, a derrick henry type of situation where he scored 153 touchdowns in higschool should be a ble to happen". Nothing caps a season or a career. See topic 10.

**Leave room for better visuals (confirmed).** Raichel: "I am potentially intrested in better visuals so leave room for htat". The engine outputs full player positions several times a second, so a nicer 2D view, or 3D later, can draw the same data without changing the engine.

## Build order (confirmed)

1. **Athlete-first player creation** (topic 5). It changes the player model and the engine reads positions, so it comes first.
2. **The match engine and the stats database** (topics 1-9).
3. **The development-only checks against real football** (topic 10), run as tests on every engine change.

## Topics

1. How a single play is decided: confirmed
2. How players move and collide: confirmed
3. Play calls and schemes: confirmed
4. Penalties, momentum and game feel: confirmed
5. High school, college and NFL in one engine: confirmed
6. Injuries and substitutions during a game: confirmed
7. Special teams: confirmed
8. The stats database, including high school: confirmed
9. Watching and calling your games: confirmed
10. Checking results against real football: confirmed

---

## Topic 1. How a single play is decided (confirmed 2026-10-13)

Claude's proposal (accepted):
- **Each play is a set of one-on-one battles happening at the same time.** Every lineman fights his blocker, every receiver works against his defender, the QB's read goes against the coverage, and the runner tries to beat his tacklers. Attributes, body, fatigue, scheme fit and the play call decide each battle.
- **The battles chain together.** A missed block gives the rusher a clean lane. That pressures the QB, who throws early into tight coverage. The result comes out of that chain, never from a single roll like "pass complete: 64%".
- **Every player is tracked on the field.** The 2D view replays exactly what the sim decided, and every stat (pressures, yards after contact, separation) comes from something that really happened. Separation is the gap between a receiver and his nearest defender.
- **Each notable play keeps its why.** For example: "Sack: the RT lost to a speed rush in 2.1 seconds while the QB held the ball because his first read was covered."

Raichel's answer: "I agree wiht your proposal and both questions".

Confirmed rules:
- **Every player's position is tracked several times a second**, like the NFL's Next Gen Stats, not only the key matchups. That gives honest separation, pursuit angles (the paths tacklers take to the ball) and an honest 2D replay.
- **Randomness sits inside each battle**, so a great corner still loses sometimes. There is no final luck roll on the whole play, so every upset has a football reason.

---

## Topic 2. How players move and collide (confirmed 2026-10-13)

Claude's proposal (accepted):
- **Movement comes from real physics.** Each player has a top speed, an acceleration and a deceleration, all from his attributes and body. A 330 lb guard takes longer to reach full speed than a 190 lb corner. Turning costs speed, and how much depends on agility and hip fluidity, so a stiff-hipped corner loses ground when a receiver breaks hard. Real tracking data (top speeds of about 20-23 mph for the fastest players) sets the scale.
- **Players see and react.** Each player only reacts to what he can notice. Reaction time depends on awareness, play recognition and where he is looking. A safety staring at the QB is late to a receiver behind him.
- **Contact is mass times speed plus technique.** A block or tackle compares momentum (weight times speed), leverage (who is lower), strength and technique. A 245 lb back at full speed can run through a 180 lb corner who arrives flat-footed. Strength and technique still beat raw size in a stalemate.
- **Body state changes during the game.** Fatigue lowers speed, strength and reaction time, depending on stamina and snaps played. Weather, turf and a heavy pass rush late in the game all show up here.

### 2.1 Vision

Raichel's answer: "Yes and it should partially be linked to play recognizatiion skills, someone with hight play recoconition or FILM-IQ could potentially "see" more. But offensively and defensly. Still an element of luck/physics. If someone falls he shouldn't be able to then tackle (just an example) i just want to reward smart players over "dumb,lazy" ones".

Confirmed rules:
- **Vision is modelled.** A player only reacts to what is in his line of sight, so a defender can be "blind" to a crossing route. That is what makes play-action, pick plays and blindside sacks work.
- **Smart players see more,** on offense and on defense. Play recognition and film IQ widen what a player picks up and how early. A sharp linebacker spots the play-action fake, and a smart QB reads the blitz before the snap.
- **Lazy players pay too.** Work rate decides whether a player keeps chasing the play or gives up, so a lazy safety misses tackles he could have made.
- **Physics still wins.** A player on the ground can't tackle until he's up, and a blocker in his way can block his view. There is always a small element of luck.

### 2.2 Tackles play out step by step

Raichel's answer: "yes".

Confirmed rule: **a tackle is a fight, not one check.** First contact, the runner keeps his legs churning, help arrives. Yards after contact and broken tackles come out of that fight, and a gang tackle can stop a strong back.

---

## Topic 3. Play calls and schemes (confirmed 2026-10-13)

Claude's proposal (accepted):
- **A play is a full set of assignments.** Each play has a formation, and every player has a job: a route, a blocking rule (such as "block the man in front of you, or help inside"), a coverage, a gap to fill or a rush move. The battles from topic 1 come out of these jobs.
- **Schemes are real systems.** For example West Coast, Air Raid or wide zone on offense, and 4-3, 3-4, Cover 3 or two-high shells on defense. A scheme decides which plays a team has and which players fit it. A slow, powerful guard fits a power-gap scheme better than a wide-zone one.
- **Players need to learn the playbook.** Knowledge grows in practice and depends on football IQ. A rookie who hasn't learned the system busts assignments: he runs the wrong route or blows a coverage. High school teams carry small playbooks, NFL teams big ones.
- **Coordinators call plays like real coaches.** The AI reads down, distance, score, clock, field position, what's been working and the opponent's tendencies. How good the calls are comes from the coordinator's hidden ability, so a sharp coordinator spots a weakness and keeps hitting it.
- **Pre-snap adjustments.** A smart QB can check to a different play, and a smart defense can change its coverage or show a fake blitz. Both depend on IQ and experience.

### 3.1 Freelancing and coach strictness

Raichel's answer: "Yes but also link it to a coach stat- Strictness, certain coaches are just dicks that don't trust their players, make it dynamic, same as if a play runs long and the original routes are covered, wr's can run to an open field to bail their QB out, this also encouraged or discouraged based on coach strcitness. MAke sure you again do this on defense as well, DB's brekaing assignments, that's what partially seperates the good players from system players. Otherwise good players in bad teams might not show up in the stats as much as they deserve."

Follow-up: "about topic 3 no need to be made hidden, if anything coaches build up a reputation for it."

Confirmed rules:
- **Players adjust within a play by their own reading,** as well as following their assignment. A receiver turns his route into a hitch against soft coverage, and a QB scrambles when the pocket breaks. Only smart, experienced players do it well.
- **Strictness is a coach trait, and it is visible.** Coaches build a reputation for it over time, so players and agents can weigh it too. A free-roaming DB might steer away from a strict coach.
- **A strict coach punishes freelancing,** so his players stick to their assignments. A loose coach lets them make plays.
- **Trust is per player and it moves.** A veteran who keeps making smart plays earns freedom. One who gets burned loses it.
- **Freelancing works on both sides of the ball.** On offense, when a play runs long and the routes are covered, receivers break off to open field to bail out the QB. On defense, DBs jump routes and leave their zone. It is a gamble: a smart player wins it more often, and a reckless one gives up big plays.
- This is part of what separates real playmakers from system players, and lets good players on bad teams still show up in the stats.

### 3.2 Coordinators adjust during the game

Raichel's answer: "Obviously and i expect you to research substantial information and data base".

Confirmed rules:
- **Coordinators learn from the game as it goes** (the opponent keeps blitzing, so they call more screens) and adjust at halftime. How fast and how well comes from their hidden ability.
- **Built on real data.** The build uses real play-by-play data, such as the public nflverse and cfbfastR datasets covering every NFL and college play for years, plus research on how real teams change their calls by situation.

---

## Topic 4. Penalties, momentum and game feel (confirmed 2026-10-13)

Claude's proposal (accepted):
- **Penalties come out of the battles.** A blocker who gets beaten may hold, and how often depends on his discipline and on how badly he lost. Jumping early depends on focus, crowd noise and fatigue. A DB who is beat deep may interfere. Totals get checked against real rates per level: high school throws more flags than the NFL.
- **Officials differ.** Each officiating crew has its own tendencies, such as one that calls holding tightly and one that lets them play, like real NFL crews.
- **The setting matters.** Weather (wind on deep passes, rain on ball security, cold on kicking), the field surface, altitude and crowd noise (a road team struggling with the snap count) all feed into the battles.

### 4.1 Momentum

Claude proposed momentum as a small hidden mental state per player, moved by big plays and mistakes, with personality setting how much it bites. Raichel asked: "it's defitenely an intangible but how do you presume it can be measured ? like how do you expect an real life scout to rank this. I'm not shutting it down, i'm asking".

Claude's answer: real scouts don't rank momentum itself. They rank how a player handles swings ("composure" or "short memory"), from film of the next snap after a mistake, stat splits (passer rating on the drive after a turnover, numbers when trailing in the 4th quarter) and character work (interviews, combine interviews, tests such as the S2 cognitive test).

Raichel's answer: "Okay sounds good".

Confirmed rules:
- **Momentum exists, kept small,** so it adds texture without deciding a game on its own.
- **No new rating.** The swing comes from the personality and mental traits that already exist, such as composure and confidence, plus the hidden mental health part (section 2, 1.7).
- **Scouts estimate it with noise.** Their notes read like real ones ("bounces back well after mistakes", "can press after a bad play"). They get sharper with more film, more interviews and a better scout.
- **The stats show it.** The database keeps "after a turnover" and "when trailing" splits, so the user can spot it.

### 4.2 Clock management is a coaching skill

Raichel's answer: "Oviously".

Confirmed rule: **timeouts, the two-minute drill, kneel-downs and 4th-down choices are a coaching skill**, built on real 4th-down and win-probability data. Weaker coaches make real mistakes and stay too cautious, the way real ones do.

---

## Topic 5. High school, college and NFL in one engine (confirmed 2026-10-13)

Claude's proposal (accepted):
- **Each level has its real rules.** College needs one foot in bounds for a catch and the NFL two. Overtime differs: the NFL plays a 10-minute period, college gives each team a possession from the 25, and high school starts from the 10 in many states. Hash marks are wider in high school and college. High school plays 12-minute quarters and has a mercy rule (a running clock in blowouts). Each state's own rules apply, since Texas and Massachusetts play by college rules.
- **The gaps between levels come from the players, not from level settings.** High school games look sloppier because the players are slower, less skilled and newer to the game. Fumbled snaps, missed tackles and blown coverages happen more often for that reason, not because of a "high school penalty". A 4.4-second recruit dominates for the same reason.
- **Rosters work the way they really do.** A small high school has 25 players, so many play both ways and on special teams, and they tire. Big programs have depth. College has scholarship limits and the NFL its 53-man rules.

### 5.1 Two-way players

Raichel's answer: "Yes".

Confirmed rule: **the head coach decides who plays both ways**, by talent and depth. His best athletes play both ways, and fatigue and injury risk go up with snap count. A strict coach rides his starters harder.

### 5.2 Athlete first, position second

Raichel asked: "how do you foresee the creaitng of players? are you thinking hey i need this many players per squad , so i'll build 3 QB's using this specifc randomizer with likely stats. Or are you building palyers with randomized/controlled stats and THEN assisgnming a position ?"

Today the code is position-first: it picks a position weighted by how common it is, then builds the attributes around it (`crates/afm-engine/src/player.rs`). Real football is the other way round: at 14 a kid is just an athlete, his coach puts him where the team needs him, and colleges move "athletes" around (a high school QB becomes a college safety, a tight end grows into a tackle).

Raichel asked for a SWOT analysis ("do a swot analyse of that potential fix, because it sounds better when i think about it"). Claude's summary:
- **Strengths:** matches real football; every player is already rated at every position; coaching matters more (a good coach spots a kid's real position, a bad one wastes him).
- **Weaknesses:** calibration has to be redone (talent model, body sizes per position, draft rates per position; the five-star 63% and FCS 6% targets need re-checking); a new "athlete talent" model is needed.
- **Opportunities:** hidden gems (a future NFL receiver stuck at QB for a small school has odd stats and gets overlooked); scouts can project conversions ("plays QB, NFL safety body"); coaches earn a reputation for developing converted players.
- **Threats:** poor coach logic gives silly players such as a 5'9" tackle (fix: place by real body and skill fit); real QB arm talent is rare, so skill leanings must be generated like real ones (fix: a calibration test on real position shares); section 3 gets bigger.

Raichel's answer: "yes definitely it'll definitely help with creating athletic specimens. A wide receiver who has high play recognization but bad hands could be a 4.2 40-yard safety".

Confirmed rules:
- **Build the athlete first.** Generate the body, the athletic traits (speed, size, strength, agility) and the skill leanings (arm, hands, toughness) from real distributions, with no position yet.
- **The position comes from the coach.** Each coach places players by fit and team need, using the "rated at every position" system. A small school's best athlete plays QB and safety. A big program might move him to wide receiver.
- **Positions change over a career.** Growth, college coaches and NFL needs move players, as often as in real life. The player card shows his position history.
- **Real totals still hold.** Real position shares are checked across the whole class, like the five-star draft rate, not forced at birth.
- Raichel's example: **a wide receiver with high play recognition but bad hands could be a 4.2 40-yard safety.**
- **It is its own build step, before the match engine,** since the engine reads positions.

### 5.3 Schemes follow the coach and the players he has

Raichel's answer: "yes but also link it to coach level and available players. I don't want a highschool to run an 3-4 okie if the edge sucks but they have great DT's."

Confirmed rules:
- **Most high schools run simpler schemes** (wing-T, spread option, heavy run) because of small playbooks and young players. Only elite programs run college-like systems.
- **A coach picks his scheme from his background, then fits it to the players he actually has.** It is never forced by level.
- **Better coaches adapt well; weaker ones adapt less.** But none will run a 3-4 Okie with bad edges and great defensive tackles.

---

## Topic 6. Injuries and substitutions during a game (confirmed 2026-10-13)

Claude's proposal (accepted):
- **Injuries come from what happened on the play.** The force and angle of a hit, a cut on bad turf, fatigue, the body part involved and the player's injury proneness decide the risk. A knee hit from the side while the foot is planted is how ACLs really tear. Injury types, how often each happens and recovery times come from real injury data per position.
- **Hurt players can play on with lower ratings.** A sprained ankle cuts speed and cutting, and the stats show it.
- **Concussion protocol per level.** The NFL has independent spotters and the protocol. College and high school follow their own rules, and some high schools have no trainer on the sideline.
- **Substitutions work like real rotations.** Defensive linemen rotate to stay fresh, backfields can be a committee, and packages come on by situation (nickel against three receivers, a goal-line set). The coach sets the plan, and fatigue triggers the swaps.

Raichel's answer: "yes on both".

Confirmed rules:
- **Playing through pain is a choice,** made by the player's toughness, the stakes (playoffs, a contract year) and pressure from the coach, with a real risk of making the injury worse. That creates "he played on a torn ligament" stories and bad coaches who rush players back.
- **Serious injuries leave lasting marks,** such as lost speed after an ACL tear and a higher chance of the same injury again, based on real return-to-play research. It varies by player: some come back fully and some never do.

---

## Topic 7. Special teams (confirmed 2026-10-13)

Claude's proposal (accepted):
- **Kicks follow real ball flight.** Leg strength sets distance and accuracy sets the miss. Wind, rain, cold and altitude bend the ball, so a kick in Denver flies further. Every kick depends on the snap, the hold and the protection, each its own battle: a bad snap or a missed block on the edge can ruin it.
- **Punts have hang time and direction.** Gunners fight their blockers to get downfield. Returners decide whether to field, fair catch or let it bounce, by their judgment.
- **Returns are full plays,** with blocks, lanes and tackles like topic 2.
- **Each level uses its real rules,** including the NFL's kickoff format from 2024, college and high school kickoffs, and touchback lines. Fake punts and onside kicks are play calls, chosen by the coach's aggressiveness and the situation.

Raichel's answer: "yes" (both questions), then a correction on pressure kicks: "i don't agree on a small part, it's what seperates the best kickers from a okay kicker. Anyone can kick a ball 50 yards. the mental is the rest. So make those intangibles way bigger for kickers/punters, also give them specilaiton stats, one kicker can be lethal within 30 yards but struggle with power and one could blast a ball from 55 but be inconsistent from 20-40".

Confirmed rules:
- **Long snappers and holders are real players with their own ratings,** so a bad snap comes from a weak snapper, not bad luck.
- **For kickers and punters, the mental side is a big factor, not a small one.** "Anyone can kick a ball 50 yards. the mental is the rest."
- **Kicker specialties come from how each kick happens:**
  - **Accuracy by distance:** separate short-range and long-range accuracy, plus leg power. One kicker is lethal inside 30 but runs out of leg at 48. Another can blast it from 55 but is inconsistent from 20-40.
  - **Consistency:** how much his kick varies from try to try. A steady kicker has tight misses, a streaky one wild ones.
  - **Composure, separate from both:** how much pressure, an iced kick or a playoff game pulls his accuracy down. It is the biggest factor between a good kicker and a great one.
- **Punters get the same split:** distance, hang time, placement inside the 20, and composure in tight games.
- The stats show all of it: field goal percentage by distance band and in clutch moments.

---

## Topic 8. The stats database, including high school (confirmed 2026-10-13)

Raichel: "since i'm a stats fetiscist it's imporatn that i'm fully informede of your intentinos". This topic is about what gets logged, computed, stored and compared. How it looks on screen is section 10; the "how you use it" list below only says what the data must make possible.

The short version Raichel agreed to:
1. **One source.** Every play of every game is logged with who did what and why. Every stat is worked out from that log, so nothing is made up after the fact.
2. **Everything is logged.** The classic box score for every position, plus advanced stats (EPA, over-expected stats, separation, pressure and win rates), a grade for every player on every play, and splits. Coaches get stats too.
3. **Fair comparisons.** One career record from high school to the NFL. Every stat is compared to the player's own level, state and schedule, and adjusted for era.
4. **High school stats are noisy.** The user sees them as schools report them. Scouts' film shows the truth.
5. **What it's for.** Coach buzz, hidden-gem flags, scouting, awards, contracts, records and the Hall of Fame.

### 8.1 Where every stat comes from
- **The engine records every play as it happens.** One play log entry per play: who lined up where, the play call, every battle and who won it, where the ball went, the result, and the why.
- **Every stat is computed from that log.** If the box score says a sack, there is a beaten blocker and a rusher behind it.
- **Tracking stats are measured during the play.** Separation, top speed, time to throw and pursuit distance are worked out while the play runs and saved as numbers, so they exist for every game even where the raw tracking is not kept.

### 8.2 What gets stored

| Layer | What it holds | Kept for |
|---|---|---|
| Play log | Every play of every game at every level: down, distance, formation, personnel, call, result, every battle outcome, tracking numbers, the why | Forever (see 8.6 for washouts) |
| Player game lines | Every stat below, per player per game | Forever (see 8.6) |
| Season and career totals | Built from the game lines, tagged by level, team and competition strength | Forever |
| Full tracking | Every player's position several times a second, for the 2D replay | The user's games and notable plays |

### 8.3 The stats themselves

Every group has the classic box score plus advanced stats. "Over expected" means compared to what an average player would do in the same situation, worked out from the play log.

- **Passing.** Classic: completions, attempts, yards, TDs, interceptions, sacks taken, passer rating, yards per attempt, adjusted net yards per attempt. Advanced: EPA per dropback, success rate, completion % over expected, air yards, average depth of target, time to throw, pressure-to-sack rate, turnover-worthy plays, big-time throws, accuracy by depth and area, results against the blitz and each coverage, play-action and scramble splits.
- **Rushing.** Classic: carries, yards, TDs, yards per carry, fumbles. Advanced: rush yards over expected, yards before and after contact, broken and missed tackles forced, explosive run rate, stuffed rate, success rate, results by gap and run scheme.
- **Receiving.** Classic: targets, catches, yards, TDs, drops. Advanced: separation at the catch, yards per route run, target share, air yards share, YAC over expected, contested catch rate, route-by-route results, results against man and zone.
- **Blocking (OL, TE, RB, FB).** Pressures and sacks allowed, pass block win rate (holding the block for 2.5 seconds), time held; run block win rate, blocks that sprung a run, penalties.
- **Pass rush.** Sacks, pressures, QB hits, hurries, pass rush win rate, time to pressure, double-team rate, win rate by move (speed, power, spin).
- **Run defense.** Tackles, tackles for loss, run stops, run stop win rate, missed tackles.
- **Coverage.** Targets, completions, yards and passer rating allowed, separation allowed, passes defended, interceptions, yards per coverage snap, results in man and zone. Freelancing (topic 3): times a DB left his assignment, and how often it won or gave up a big play.
- **Tackling.** Solo and assisted tackles, missed tackle rate, yards after contact allowed, forced fumbles.
- **Special teams (topic 7).** Field goals by distance band (under 30, 30-39, 40-49, 50+), clutch kicks (pressure-weighted), extra points, kickoff distance and hang time. Punts: gross, net, hang time, inside the 20, touchbacks. Returns: yards and return yards over expected. Snapper errors. Coverage units: tackles, gunner wins.
- **Team and unit.** Points, yards, EPA per play on offense and defense, success rate, red zone and third down rates, explosive play rate, pressure rate, penalties, turnovers, starting field position, time of possession. Opponent-adjusted unit ratings, the way DVOA measures each play against the league average and the opponent.
- **Coaching and play calling.** Pass rate over expected, 4th-down choices against the win-probability model, timeout and clock use, play-action and blitz rates, halftime adjustment (second-half against first-half EPA), and player development under each coach. These feed coach buzz.
- **Player grades.** Every player gets a grade on every play from the battle results, PFF style. Grades roll up into game, season and career grades.
- **Splits for everything.** By quarter, half and score margin; trailing or leading; after a turnover (topic 4); red zone, third down, two-minute drill; weather (cold, wind, rain); home or away; against ranked opponents.

### 8.4 Fair comparisons across levels and eras
- **One career record** covers high school, college and the NFL. Every season is tagged with level, state, class, conference and competition strength.
- **Percentiles among peers.** Every stat shows a percentile against the right peers, such as "98th percentile yards after contact among Kansas 1A running backs". This matches the level-relative ratings from section 1.
- **Opponent-adjusted versions.** A 2,000-yard season against weak schedules is shown both raw and adjusted.
- **Era adjustment.** Records keep raw numbers, with an era-adjusted view next to them (passing yards rose a lot over the last 30 years at every level).

### 8.5 High school stats as reported

Raichel's answer: "yes but don't make school data so distorted that scouts consitently show up to "lies"".

Confirmed rules:
- **The engine knows the true stats of every high school game.**
- **The user sees what a MaxPreps-style site shows.** Nobody keeps advanced stats for small schools.
- **Most schools report accurately.** A minority report late or not at all. Padding is rare and small. So when scouts look, the numbers usually match, and a real "lie" stands out.
- **Scouts' film fills the gaps.** Good scouts log the true numbers and advanced stats for players they watch.
- **Debug and trial tools always show the true stats.**

### 8.6 What is kept

Raichel's answer: "Yes, keep playerstats forever (i assume you delete the ones that won't go to NCAA or don't go pro and don't make it through UDFA, camps etc)", then "basically as much logging as possible".

Confirmed rules:
- **Keep every stat forever, and as much logging as possible.**
- **Full position tracking (the raw replay data) is kept only for the user's own games and notable plays.** All tracking numbers (separation, speed, time to throw) are still saved for every game.

Confirmed 2026-10-13 (Raichel: "yes to both"), Claude's proposal for washouts:
- Trimming happens only where save size or speed truly needs it (about 250,000 high school players enter each year, and most never play college).
- **Players who make college or the pros:** everything kept forever.
- **Players who wash out:** after they leave, a small record stays: season totals, awards and any record they set. Their game-by-game lines and play logs are deleted. A state record from 2031 still shows the kid who set it.
- **High school play logs:** deleted once nobody in that game is still playing. Team totals and box scores stay.

### 8.7 What the stats feed
- **Coach buzz (section 2, world item 5).** Unit stats relative to level, such as a high school coach whose team breaks the state points-per-game record, or an OC whose EPA is top 1% in his conference. It fades over 2-3 years.
- **Hidden-gem footprints (section 2, world item 9).** Searchable flags for outliers relative to level, such as top 1% in yards after contact, top 1% in separation, or a testing outlier. Stats can lie: a weak schedule or a padding scheme can fake a footprint, and a bad team can hide one.
- **Scouting, recruiting, awards (all-conference, Heisman, All-Pro, MVP), contracts and agents.** Agents quote stats in talks.
- **Record books** per state, conference, level and all time, plus streaks. These feed the Hall of Fame.
- Confirmed 2026-10-13 (Raichel: "yes to both"): the "over expected" stats are what lift a player's ceilings when he keeps beating his ratings (the breakout rule from section 1).

### 8.8 What the data must make possible (screens are section 10)
- Leaderboards for every stat, filtered by level, state, conference, season, position, age and class.
- Custom searches, such as "running backs, 2027, Texas 4A-6A, at least 150 carries, yards after contact per carry above 4".
- Side-by-side player comparisons.
- Career pages with every season at every level.
- Game pages with the box score, a drive chart, the win-probability graph and the play-by-play with its why.

---

## Topic 9. Watching and calling your games (confirmed 2026-10-13)

Claude's proposal (accepted):
- **Every game in the world runs on the same full engine,** including every high school game, in parallel on all CPU cores, with no shortcut model.
- **The user's game takes about 15 minutes to watch.** The 2D view shows highlights, like Football Manager. The user chooses how much to see: key plays only, more plays, or every snap. Next Gen Stats-style overlays show separation, speed and time to throw. Every play, shown or not, keeps its why in the play-by-play.
- **Between plays the user can step in:** change the game plan, call a timeout, swap players, or tell the coordinator to attack a weakness. Staff give notes at halftime.

### 9.1 Live play calling

Raichel's answer: "As an HC i can always chose to take over a play or i can allow the OC/DC to choose. When i don't choose to pick, the game should show me the play/visualazed and then i can accept or deny. THe quality of the playcal is depend on the quality of the coach and how well you researched plays and what counters what".

Confirmed rules:
- **The user, as head coach, can take over any snap.**
- **Otherwise the OC or DC picks the play, shows it drawn up, and the user accepts it or sends it back.**
- **The quality of the call depends on the coach's ability and on a real play-and-counter library** built from research, for example which coverages beat four verticals.

### 9.2 Watching other games

Raichel's answer: "No but i should be able to wathc back and visualisation if i chose to, so no live sim but afterwards".

Confirmed rules:
- **No watching other games live.**
- **Any other game can be watched back in 2D afterwards.** This costs no storage: every game has its own random seed, so the engine re-runs any past game exactly as it happened.

### 9.3 Better visuals later

Confirmed (section 0): the engine's output (every player's position several times a second) is the contract with any view, so a better 2D or a 3D view can come later without engine changes.

---

## Topic 10. Checking results against real football (confirmed 2026-10-13)

Claude's proposal: before the match engine counts as done, its output gets checked against real numbers at each level, the way the draft and five-star rates were checked.
- **NFL:** points per game, completion %, sack rate, yards per carry, penalties per game, field goal % by distance, how often 4th downs are attempted and converted. From nflverse play-by-play data.
- **College:** the same checks per division, from cfbfastR data.
- **High school:** scoring and blowout rates per state and class, from MaxPreps.
- **Player ranges:** the best season each year should look real, such as top NFL passers around 4,500-5,000 yards.

Raichel asked: "i understand the idea.... but are you treating it as an safe guard or as an in-game fix ?" Claude: only a safeguard during development, never an in-game fix. Raichel: "okay becuase i'm very insistant of having outliers be present, a derrick henry type of situation where he scored 153 touchdowns in higschool should be a ble to happen". Final answer: "as long as it's in development, yes".

Confirmed rules:
- **The real-data checks are development-only safeguards, never in-game fixes.** The game never pushes results toward a target. If passing numbers come out too high, the cause gets fixed in the engine (coverage reactions, how accuracy works), and the tests prove the fix worked. They never touch a save.
- **Targets are recent seasons (about 2021-2025).**
- **The era changes through the world, not a hidden dial.** Coaches adopt what works (more passing, more 4th-down attempts), and it spreads through coaching trees, the way it really happened. Real trend data only checks that the spread looks believable.
- **Automatic tests run on every engine change,** like the national class checks, so a tweak to tackling can't quietly break passing numbers.
- **Outliers must happen.** In a save, a season can be an outlier: a record-breaking passer, or a year full of upsets. Nothing caps a season or a career. Derrick Henry's 153 high school touchdowns came from a huge, fast back at a small school (Yulee), playing weak defenses, with a coach who gave him the ball 40 times a night for four years. When that mix comes together, the numbers go wild.
- **The tests check the extremes as well as the averages.** Across the 30 years of history, the best seasons and careers should land near the real records, sometimes beating them, and never get squashed toward normal.

---

## Open questions

None. Both earlier questions (washout trimming in 8.6, over-expected stats lifting ceilings in 8.7) were confirmed by Raichel on 2026-10-13.

## Build status

1. **Athlete-first player creation: built 2026-10-13** (`crates/afm-engine/src/athlete.rs`; how it works in `docs/development-model.md`; Claude's calibration picks in `docs/decisions.md`).
2. **The match engine and game stats: built 2026-10-14** (`crates/afm-engine/src/game/`; how it works, calibration and known gaps in `docs/match-engine.md`). Still to come: running every world game through it, and the season and career record across levels.
3. **The development-only checks: built 2026-10-14** (`match_check`, `crates/afm-tools/tests/match_engine.rs`).
