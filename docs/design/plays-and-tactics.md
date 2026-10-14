# Plays and tactics database (design done)

Status: design finished 2026-10-14. All nine topics are confirmed by Raichel. Nothing here is built yet. It extends section 3 topic 3 (play calls and schemes) and 9.1 (live play calling) in `docs/design/match-engine.md`, and feeds section 5 (schemes) and section 9 (game day) on the roadmap.

This spec is written so anyone outside the project chat can follow it. Raichel's own words are in quotes.

Labels:
- **Confirmed**: Raichel answered it.
- **Proposed, not confirmed**: Claude's draft. Ask Raichel before building on it.

What exists today (2026-10-14): `crates/afm-engine/src/game/playbook.rs` has 16 pass concepts, 9 run concepts, 8 coverages, 5 pressures and 4 defensive personnel groups. `calling.rs` calls plays from real NFL situational rates, bent by scheme and coordinator skill. This spec replaces that small library with a real one.

---

## 0. Ground rules for this design

**Where it started.** Raichel: "Hopefully this will be a game where if you chose a play that objectively is wrong (running my RB against +2 defense) it punishes that and not solely if the HC. So the HC qualitity should be incombant on that." And on coaching: "in the long scheme it should only heavily influence the outcome of a play when the user-player doesn't pick."

**Good plans and calls must pay off (confirmed, a rule for the whole design).** Raichel: "it's important to me that the weights of this are rewarding good plans. Like earlier you had the issue with the HC, that it didn't have a lot off effect on the outcome. When building this topic it's imperative that it effects get rewarded without being broken so that it incentivises learning objective knowledge of the game."

So:
- A good call wins and a bad call loses **through the battles** (box numbers, leverage, matchups), never through a flat bonus.
- The same rules apply to the user's calls and the coordinator's calls.
- The size of the reward is checked against real data (topic 6), big enough to matter, never so big that calls beat talent.

**Ultra realism first.** Raichel asked twice for more realism on the first proposals. Every rule below is the full-realism version.

---

## Topic 1. What a play is made of (confirmed)

- **Plays are built from real parts,** the way NFL playbooks name them. Offense: personnel, formation, motion or shift, the concept, protection and tags (for example "11 personnel, trips right, jet motion, inside zone, slant tag"). Defense: personnel, front, the technique each lineman plays, coverage, pressure, stunts and disguise.
- **The library is sized from real playbooks, not a fixed count.** NFL teams carry several hundred plays plus variations and pick about 150 for a week's game plan. High school teams carry far fewer. The library grows to match, era by era. Starting estimate for the parts: about 40 formations, 25 run schemes, 60 pass concepts plus play-action, screens, RPOs and trick plays; on defense about 15 fronts, 20 coverages (including pattern-matching and rotations) and 40 pressures and stunts. Each part is researched: how it is blocked, its reads, and what it is built to beat.
- **Every job is a rule that reacts, not a fixed path.** Linemen block by real rules (zone steps, double-teams that climb to a linebacker, pulling guards). Routes have branches: a receiver bends his route against zone, breaks it off against man, or runs a sight adjustment against a blitz. Defenders play pattern-matching rules (a corner passes a receiver off or carries him, by the route). A play plays out differently against each look.
- **Real defensive fits.** Each lineman lines up in a real technique (where he sits on the blocker) with a gap to own. Every run has someone forcing it back inside, someone spilling it outside, and someone with the cutback. A missed fit is what turns a run into a big gain.
- **Real play-calling languages.** West Coast, Erhardt-Perkins and Air Raid name plays differently, and some are faster to learn. A rookie from an Air Raid college needs longer to learn a West Coast playbook (see topic 4).
- **Each level has its own plays and rules.** High school keeps the wing-T, flexbone triple option and single wing. College linemen may go 3 yards downfield on a pass (the NFL allows 1), which is why college runs so many RPOs (the QB hands off or throws depending on one defender).

## Topic 2. How plays beat each other (confirmed)

- **No counter table decides anything.** A call wins or loses through geometry and the battles. Run into a +2 box and there are more defenders than blockers, so someone goes unblocked. Throw four verticals into Cover 2 and one safety must cover two receivers.
- **Conflict defenders.** Many plays attack one defender with two jobs, like the linebacker on play-action (stop the run or drop into his zone). His IQ and discipline decide whether the play works.
- **Players can still beat the call.** A wrong call usually loses, but an elite back can break a tackle against a stacked box, and a great corner can save a good call. A great call with bad players can still fail. Neither side is overpowered.
- **The researched counter knowledge steers the coaches, not the results.** Coaches use it to pick calls, and their quality decides how well. Real splits (yards per carry by box count, results by concept against each coverage) are only development checks, like the other real-data checks.
- **Leverage and alignment.** Where a defender lines up decides which routes beat him. A corner with outside leverage gives up the slant. The offense reads this before the snap.
- **Plays set up other plays.** Run inside zone five times and the linebackers start cheating forward, which opens the bootleg or the play-action shot. Each defender remembers what he has seen this game, by his own IQ, not only the coordinator.
- **Play-action follows real research.** Studies show play-action works about as well whether or not the run game is working. So the fake fools defenders through their keys and discipline, not through the rushing stats.
- **Matchup hunting through personnel.** Two tight ends force the defense to choose: stay heavy and a linebacker covers the tight end, go light and the run gets easier. No-huddle tempo stops the defense from substituting, and tired defenders lose battles.
- **The field itself.** Hash marks differ per level (high school widest, NFL narrowest), so the short side and wide side change which plays work. Wind, rain and the red zone squeeze some plays and open others.
- **Communication and the crowd.** A loud crowd hurts the visiting offense: checks get missed and false starts go up, so it uses a silent count and keeps its calls simpler.

## Topic 3. Before the snap (confirmed)

1. **Motion gathers information.** If a defender follows the motion man, that hints at man coverage. Good defenses disguise it by passing the motion off or rotating. Motion also shifts leverage, creates stacks against man, and jet motion freezes linebackers for a beat.
2. **Shifts and formations force the defense to declare.** Moving the strength late makes the defense re-align, and slow or confused defenders get caught out of place.
3. **QB checks depend on the coach's trust.** The coach sets how much freedom each QB gets: none, two plays called in the huddle with the QB picking one at the line, or full audibles like Peyton Manning. Trust grows with experience and good checks, and strict coaches give less (strictness is the visible coach trait from section 3, 3.1).
4. **Protection calls.** The center and QB name the middle linebacker and slide the line toward the threat. A wrong call leaves a rusher free, and smart linemen fix calls on their own.
5. **Defensive disguise.** Show two deep safeties and rotate to one at the snap, show a blitz and drop out. Late rotation needs fast, smart defenders, and the QB sees through it in proportion to his IQ against theirs.
6. **Defensive checks.** Defenses check their coverage against trips, bunches and motion, called by the middle linebacker or safety. A rookie who misses the check busts the coverage.
7. **Tells.** A lineman leaning on his hand, a safety creeping down. Smart players read them, and film study (topic 5) teaches a team the opponent's tells.
8. **Communication by level.** NFL and, since 2024, college teams have one helmet radio per side. High school uses hand signals, wristbands and boards. Complex checks take time, so the real play clock per level limits them, and tempo leaves the defense less time to disguise.
9. **Hard counts.** The QB varies his snap count to draw jumps. Discipline decides who jumps, and a loud crowd takes the hard count away from the visiting offense.

## Topic 4. Playbooks and learning them (confirmed)

1. **Players learn parts, not plays.** A right tackle who knows inside zone knows it from every formation. Knowledge carries across plays that share parts, and across systems that share concepts under other names.
2. **Three depths of knowing.** He knows his job, he understands why (so he adjusts correctly), or he has mastered it (he plays fast and his freelancing is smart). A player still thinking plays a step slow, which is why rookies look slow.
3. **The load differs by position.** The QB, center, middle linebacker and safeties must know everyone's job. A receiver needs his route tree and its branches, a lineman the line calls.
4. **Where learning comes from.** Practice reps (starters get the most, scout team players far fewer), meetings, film, walkthroughs and study at home. Football IQ, work ethic and the position coach's teaching set the speed.
5. **Real practice limits.** NFL CBA limits, NCAA hour limits and state high school rules cap how much can be installed. Teams install in stages through OTAs, spring ball and camp.
6. **The coach picks the playbook size.** Too big for his players means more busts, too small means opponents read it. Sizing it right is a coaching skill. High school books stay small, NFL books large.
7. **Weekly wrinkles carry risk.** A new play this week can surprise the opponent, but it is barely learned.
8. **Knowledge fades and resets.** Unused plays fade over the offseason. A new coach brings a new system, and only shared concepts carry over. Players signed mid-season get a small package first.
9. **Units learn together.** A line that has played together makes cleaner calls and passes stunts better. Swap two linemen and that chemistry drops for a while.

The user sees each player's knowledge per concept family and the install progress.

## Topic 5. The weekly game plan (confirmed)

1. **Film breakdown of the opponent.** Staff study the last few games and the coaches' history, broken down by down, distance, field zone, personnel and formation. The number and skill of the staff (including quality control coaches) limit how deep it goes, and the result has errors.
2. **A real practice week.** The NFL rhythm: review on Monday, plan on Tuesday, then 1st and 2nd down, 3rd down, red zone and two-minute on the following days, and a walkthrough before the game. College and high school use their own real versions. Short weeks (Thursday games) cut the install, and bye weeks add prep, with the small edge real data shows.
3. **The call sheet is organized by situation.** Opening script (like Bill Walsh's first 15 plays, which also tests the defense), 1st and 10, 2nd and long or short, 3rd down by distance, red zone, goal line, two-minute, four-minute, backed up, and shot plays. Calls off the sheet are possible but less practiced.
4. **Matchup plans.** Attack the weak corner, chip or double the great pass rusher, bracket their best receiver, spy a running QB. Injuries and inactive players reshape the plan.
5. **The scout team matters.** Backups imitate the opponent in practice. The better they imitate, the better the starters are prepared.
6. **Plans can be wrong.** A good opposing coach breaks his own tendencies, so the plan is worth only as much as the opponent is predictable.
7. **Weather and venue.** The forecast, turf or grass, a dome or altitude all shape the plan.
8. **The user's role.** The user builds the plan or hands it to the coordinators, and sets the head coach's philosophy (run and pass balance, 4th-down aggression, tempo). Their quality decides how good a delegated plan is.

Raichel added the reward rule here (see section 0).

## Topic 6. Who calls the plays, and how quality shows (confirmed)

1. **Who calls.** The user can take any snap. Otherwise the play-caller does: the coordinator, or the head coach himself if he calls plays (like Sean McVay). The user sees the proposed call and accepts or rejects it (section 3, 9.1).
2. **Coach quality is several skills.** Reading the defense, situational knowledge, game management, creativity (designing plays that fit his players) and in-game adjustment. Aggressiveness is a style, not a quality, and data shows most coaches are too cautious.
3. **The reward is checked against reality.** Research the real gap between the best and worst play-callers in points per game. A development check plays the same two teams with a good plan against a bad plan, and the swing must land in that real band: big enough to matter, never so big that calls beat talent. (The band itself is still to be researched.)
4. **The user and the coordinator play by the same rules.** A smart call by the user gets the same reward as a smart call by an elite coordinator. A bad call costs the same.
5. **Pressure changes coaches.** Some get conservative when behind or on the hot seat, others gamble. That comes from personality.
6. **The game teaches why.** The user can ask the coordinator why he proposes a call ("they're in a light box, so we run"). After the game, a breakdown shows why calls worked or failed: box count, coverage, the matchup. Learning real football is the way to get better.
7. **No exploit stays open.** If the user keeps calling the same winning play, the opposing coach adapts at his own skill level, and a weak coach adapts slowly.

## Topic 7. Tendencies and self-scouting (confirmed)

1. **Every team leaves a trail.** The play log records what each team calls by down, distance, personnel, formation, field zone, hash, score and motion. A formation that runs 80% of the time is a tell.
2. **Predictability is punished through the players.** A defender who knows the tell from film and from this game reads the play a beat sooner, so he wins more battles. How much depends on his IQ and film study. No flat penalty.
3. **Breaking tendencies costs too.** Calling a worse play just to be random loses yards. Some real offenses stay predictable and still win through execution (Kyle Shanahan's under-center runs and play-action look the same). Finding the right mix is coaching skill.
4. **Player tells.** A blocking tight end coming in signals run, a QB who stares at his first read, a back whose alignment shows the run side. These come from habits and IQ, and coaching can fix them.
5. **Defenses have tendencies too.** A coordinator who always blitzes on 3rd and long gets punished by screens and hot routes.
6. **Self-scouting.** The staff (quality control coaches) report the team's own tendencies, best during a bye week. Good staff spot them sooner, and good coaches add deliberate tendency breakers.
7. **Film builds over a season.** Early games have little film, and a new coach has none at his new team, so opponents study his old team. By the playoffs everyone is well scouted.
8. **What the user sees.** A tendency report for his team and each opponent, with its accuracy set by the staff's quality and the sample size.

## Topic 8. Building your own plays (confirmed)

1. **A play designer from the same parts.** Offense: formation, motion, each player's route with its branches, blocking rules and protection. Defense: alignments, techniques, coverage rules and rush paths. Existing plays can be edited with tags and variations.
2. **Legal by the real rules of each level.** Seven on the line, eligible receivers, motion rules, how far linemen may go downfield. The designer flags anything illegal before saving.
3. **The game never grades a design.** A play works only as well as its geometry. Two routes ending in the same spot, no answer to a blitz, or an unblocked defender fail on the field, the same way real bad designs do.
4. **The coordinator gives feedback,** good or poor by his skill. A sharp one says "nobody handles the Will linebacker if he blitzes", a weak one misses it.
5. **New plays must be installed and learned** like any other (topic 4), so they cost practice time.
6. **Practice gives an early, noisy read.** Run it against the scout team before using it in a game.
7. **Opponents learn it from film.** A new trick play works best the first time, then defenses prepare for it.
8. **The playbook travels with the user.** A new job means bringing it along, but the new players must learn it.
9. **AI coaches create plays too.** Creative coaches invent new concepts, which feeds topic 9.

## Topic 9. How ideas spread over the years (confirmed)

1. **The world starts with the real 2026 mix.** Each level's real schemes, researched: NFL wide zone and play-action, two-high defenses, college RPOs and tempo, regional high school styles like the wing-T.
2. **New ideas come from creative coaches, often at small schools.** The Air Raid grew at small colleges, the pistol at Nevada, the Wildcat at Arkansas. An idea needs the right players to work.
3. **Ideas spread through real channels.** Visible success (results, stats, buzz), coaching trees (assistants carry the playbook), offseason staff visits and copying from film. Open-minded coaches adopt fast, old-school ones slowly or never.
4. **Spread goes both ways between levels.** The NFL copies college (spread QBs forced it), and high schools copy what they see on TV.
5. **Defenses answer, so nothing stays on top.** Two-high shells answered the spread passing boom. Every trend gets a counter, and the cycle keeps going.
6. **Ideas die and come back.** The wishbone and run-and-shoot faded. An old idea can return when defenses forget how to stop it.
7. **Schemes shape the players.** If high schools pass more, more good passing QBs and fewer fullbacks come through over decades. The player pool shifts with the game.
8. **Analytics spread the same way.** On-field rules stay at the 2026 rules for the whole career (section 4, topic 11), so new rules never shift what works; only an agreed fixed list of off-field changes can happen. Data-driven habits like going for it on 4th down spread through the trees (section 3: the era changes through the world, not a hidden dial).
9. **The user can watch it.** The world shows trend lines per level by year (pass rate, scheme shares, 4th-down rate). Real history only checks that the pace looks believable.

---

## Open items (proposed, not confirmed)

- **Research still to do:** the real best-to-worst play-caller gap in points per game (topic 6), play-action and box-count splits, concept-against-coverage results, real playbook sizes per level, practice-hour rules per state, and the 2026 scheme mix per level.
- **When it is built:** the coordinator placed it with schemes (section 5) and game day (section 9). The World Trial 3 physics fix already works on making coaching show in games and was sent the reward rule. Ask Raichel where the build goes in the order.
