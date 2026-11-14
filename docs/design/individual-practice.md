# Individual practice: flaws and tools

Design started 2026-11-14 in the thread "Plan the training patch". Plan and sizes agreed by Cain the same day. Not built.

## Cain's words

- 2026-11-14 12:02: "There is a lack of payoff to setting training, it feels unfulfilling, acts like busywork and feels like a dead feature."
- 12:04: "i was mostly talking about individual practice for now." The team-week ideas from the same thread (testing-day changes, a staff development report, scout-team looks, split first-team reps) are parked.
- 12:14: "that's an symptomatic approach when i prefer an Etiological approach that boosts players immersiveness."
- 12:15, on flaws and tools as a new engine layer that replaces the "focus one rating" plan: "well... i meant to fix both, but yeah go ahead." Read as: yes to the root (flaws and tools) and yes to dropping the rating focus. The symptoms (charting, notes, a habit chart) are fixed on top of it.

## Why it is dead today (the cause)

Measured from the code, not from a world run (`docs/reference/how-it-works/training.md`):

- A plan is "grow this rating faster". The focus multiplies one rating's growth rate by 1 + 0.5 × take, which is 1.3 to 1.5 (`development.rs`, `GrowthCtx::rate`). That's about +1 point a year on a young player with room to grow, and under half a point on an older one.
- Your staff's read of your own players is a range of ±2 to ±4.3 (`crates/afm-game/src/reads.rs`). The gain stays hidden inside it.
- The focus ignores time. He gets the same lift with no individual work as with an hour a day. Extra work only feeds the team week's 15% move (`training/work.rs`, `PLAN_REACH`).
- Nothing reports back. The player page says "growing steadily" (`lane_training.rs`, `growth_words`).
- The design's veteran who refuses a focus (`docs/design/training.md`, topic 5) was never built.

The root: real coaches never work on a number. They see a specific problem on tape, drill it with a specific coach, and watch whether it holds up on Saturday. The game has no such problems in it, so nothing you do can show up on the field.

## What it does

- **High school.** Your 16-year-old quarterback throws off his back foot when pressure comes. You see it on his tape and work it through summer 7-on-7s. In September he still slips back on third down. By his senior year it is gone, and college coaches' notes say "cleaned up his base". The offers follow.
- **College.** Your redshirt freshman left tackle oversets against speed, so the inside counter beats him. The line coach drills his kick-slide in spring. He still loses that rep in camp one-on-ones. By October the tape tags show it happening less each week, and in November he stones the conference's best edge.
- **NFL.** Your veteran quarterback rebuilds his footwork with a private coach in the offseason, as Josh Allen did with Jordan Palmer. In week 1 he slips back under pressure. By midseason his throws under pressure come good, and the press notices.

## Topic 1. What a flaw and a tool are

1. **A flaw is one specific way of losing one battle.** It is how scouts write weaknesses: "oversets", "rounds his breaks", "locks onto his first read", "peeks into the backfield". It costs him only in its own situation.
2. **A tool is something added to his game.** A counter move, a release package against press, a pump fake that moves safeties. It helps only in its own situation.
3. **They sit inside an existing engine moment and never own one.** Each attribute owns its moment (`docs/design/attributes.md`, "One main owner per engine moment"), and each tendency owns how often (`docs/design/tendencies.md`). A flaw or tool moves the owner's read in one narrow slice, the way Dynamic Links do (`docs/design/dynamic-links.md`). An overset tackle's pass set reads lower only against speed and the inside counter. Against power he plays to his rating.
4. **Not talent.** Fixing an overset doesn't make a slow tackle fast, and a tool doesn't add a ceiling. Ratings keep growing by the yearly step as they do now.
5. **No cliffs.** Each flaw has a severity from 0 to 1 that grows and shrinks smoothly.
6. **Flaws against links.** A flaw and a link in the same situation both count. A Burner receiver who rounds his breaks still runs past people, but he doesn't separate on the out.

## Topic 2. The list by position

Each entry gives the engine moment it sits in, what it costs (or adds), and the tell on tape. Tight ends use the receiver and line lists. Every player can carry the ball-carrier and tackler entries.

### Quarterbacks
- Flaw, **drops his eyes to the rush**: pocket presence's vision cone shrinks once a rusher is close. Tape: he stares at the rush and misses the open man.
- Flaw, **throws off his back foot**: accuracy under pressure falls further than his rating says. Tape: balls sail high when the pocket closes.
- Flaw, **locks onto his first read**: each read past the first comes later. Tape: the second read was open and he never got there.
- Flaw, **stares down his target**: defenders' anticipation gains on his short throws. Tape: the linebacker undercuts the slant.
- Flaw, **long motion in traffic**: release speed counts for less with a rusher in reach (strip-sacks, batted balls). Tape: the ball comes out low and slow.
- Tool, **pump fake that moves safeties**: eye manipulation gains on double moves and the second read.
- Tool, **hard count that draws flags**: the rushers' guess on the count (the "times the snap" tendency) misses more.
- Tool, **back-shoulder throw**: a share of contested throws to the sideline where only his receiver can catch it (with that receiver's chemistry).

### Running backs
- Flaw, **ball away from his body in traffic**: ball security reads lower in a crowd. Tape: the ball swings loose on contact.
- Flaw, **ducks his head in pickup**: blitz pickup misses more against a rusher who moves late. Tape: the linebacker sidesteps him.
- Flaw, **runs too upright**: leg drive counts for less at first contact. Tape: he goes down at the first hit.
- Flaw, **slow through the mesh**: the press to the line starts later. Tape: the hole closes before he gets there.
- Tool, **jump cut**: elusiveness gains in the backfield against a filling linebacker.
- Tool, **stiff arm**: balance and leg drive gain against arm tackles in the open field.

### Receivers (and tight ends in routes)
- Flaw, **rounds his breaks**: route running separates less on out-breaking and comeback routes. Tape: the corner stays on his hip.
- Flaw, **tips his breaks**: he gears down before the cut, and defenders' anticipation gains. Tape: the corner jumps the route.
- Flaw, **body catcher**: catching reads lower on balls off his frame and under contact. Tape: it bounces off his pads.
- Flaw, **one release against press**: release reads lower against a pressing corner who has seen it. Tape: jammed at the line on third down.
- Tool, **release package**: release gains against press.
- Tool, **late hands**: contested catching gains (the corner can't time his hands).
- Tool, **sudden stop**: route running gains on comebacks and hitches.

### Offensive line (and tight ends blocking)
- Flaw, **oversets**: the pass set reads lower against speed and the inside counter. Tape: beaten inside by the counter.
- Flaw, **catches instead of punching**: anchor reads lower against power. Tape: walked back into the quarterback.
- Flaw, **over his toes**: run blocking falls off more against finesse moves. Tape: the swim leaves him on the ground.
- Flaw, **high pad level**: run blocking loses on short yardage and double teams. Tape: no push at the goal line.
- Flaw, **heavy feet on the climb**: blocking in space misses more on the way to the linebacker. Tape: the linebacker scrapes free.
- Tool, **re-fits his hands**: he recovers more often once a rush starts to win.

### Defensive line and edges
- Flaw, **no plan**: the counter move comes less often and fits the set worse (the engine's counter, `update_blocks`). Tape: a stalled rush dies.
- Flaw, **high pad level**: he gets washed out on double teams. Tape: driven off the ball.
- Flaw, **lets the blocker into his chest**: block shedding reads lower against a punch. Tape: locked up all game.
- Flaw, **eyes in the backfield**: he loses his gap on traps and misdirection. Tape: trapped.
- Tool, **real counter**: the counter fires more often and is better fitted.
- Tool, **long arm**: the power rush gains against a short set.
- Tool, **spin**: the finesse rush gains against an overset tackle.

### Linebackers
- Flaw, **catches blocks**: block shedding reads lower against climbing guards. Tape: glued to the guard.
- Flaw, **overruns the ball**: pursuit angles lose to cutbacks. Tape: the back cuts behind him.
- Flaw, **no depth in his drop**: zone coverage leaves the window behind him. Tape: the dig fits in behind him.
- Flaw, **loses the back on the wheel**: man coverage on backs reads lower on the wheel. Tape: the back runs free up the sideline.
- Tool, **shock and shed**: block shedding gains with his hands.
- Tool, **A-gap rip**: blitzing gains against the guard.

### Corners
- Flaw, **peeks into the backfield**: man coverage loses to double moves and play action. Tape: beaten deep on the sluggo.
- Flaw, **opens his hips early**: man coverage loses to inside breaks. Tape: the slant was there all day.
- Flaw, **lunges at the jam**: press whiffs more, and he is beaten clean when it does. Tape: on the ground at the line.
- Flaw, **doesn't play the hands**: contested defending reads lower at the catch point. Tape: in position and the catch still made.
- Tool, **mirror press**: patient feet, so press gains without the whiff risk.
- Tool, **bait coverage**: he shows open and closes, so anticipation gains on throws into his zone.

### Safeties
- Flaw, **poor angles**: pursuit and tackling in the open field lose to cutbacks. Tape: a 12-yard run becomes 60.
- Flaw, **eyes stuck on the quarterback**: the eye manipulation against him works better. Tape: the post behind him.
- Flaw, **late rotation**: his disguise turns to the true coverage later. Tape: the seam is open after the snap.
- Tool, **robber bait**: anticipation gains on in-breaking routes in front of him.

### Specialists
- Flaw, **plant foot drift**: long kicks miss to one side more. Tape: wide right from 48.
- Flaw, **drop inconsistency** (punter): punt power has more shanks. Tape: a 28-yard punt.
- Flaw, **high snaps in the wet** (long snapper): bad snaps in rain and snow.
- Tool, **pooch and coffin corner** (punter): punt placement gains inside the 50.
- Tool, **onside touch** (kicker): kickoff technique gains on onside kicks.

### Ball carriers and tacklers (anyone)
- Flaw, **dives at ankles**: tackling misses more in the open field.
- Flaw, **carries it loose**: ball security reads lower in a crowd (backs above, any carrier).

## Topic 3. Where they come from

1. **At 14, from who he is.** Each player draws flaws from his ratings, body and habits, with chance for the rest. A stiff tackle (low flexibility) leans toward oversetting. A tall back leans toward running upright. A gunslinger leans toward staring down targets. A low-IQ quarterback leans toward locking onto his first read. These are influences, never certainties (the working rules).
2. **Raw players carry more.** A typical freshman starts with 3 to 5 flaws and almost no tools (proposed, not confirmed).
3. **Coaching shapes the count from there.** Good staffs fix flaws and add tools. Poor ones leave them, and bad habits can grow a flaw. No level holds a fixed count. The NFL has fewer flaws because players who never fix theirs don't make it.
4. **The check against real life.** Draft profiles usually list a few technique weaknesses for each prospect. A drafted player should carry 1 to 3 flaws and 1 to 2 tools (proposed, not confirmed; measured in the trial).
5. **They change by themselves too.** Snaps that keep losing to a flaw can make it worse (the tendencies' "results teach", `docs/design/tendencies.md`, topic 3.3). A tool can grow from game reps that keep winning with it.

## Topic 4. Finding them: the staff's eye

1. **Flaws are the world's truth.** Your staff sees them through their eye, as with every other read. They are not potential, so they can be named once they're seen.
2. **Found on tape and in practice.** Each week your coaches' practice and game looks can find a flaw. The better the eye and the more reps of that situation he has, the sooner they find it. Your own looks count too (own reads, SK23).
3. **Misdiagnosis.** A weak eye can name the wrong flaw, e.g. "his hands" when it's his feet. Working on a wrong flaw fixes nothing, but the minutes still count toward his general growth.
4. **Tape tags.** Each snap where a flaw (or tool) decided the rep gets a tag in his film and game book, e.g. "beat inside by the counter: overset". The tags are what you see, so a flaw nobody has named yet shows up as losses with no tag.
5. **Other staffs read them too.** Scouting reports, draft boards and recruiting files name flaws ("raw technique, fixable") through each evaluator's eye.

## Topic 5. Working on them: points of emphasis

1. **One or two points of emphasis per player at a time**, as real position coaches give them (`docs/design/training.md`, topic 5). Each one is a flaw to fix or a tool to add.
2. **The staff sets them unless you do.** His position coach picks from what the staff has seen, and you can change any of them. This keeps a 100-man roster from becoming busywork: you step in for the players you care about.
3. **It replaces the rating focus** (Cain's yes, 2026-11-14). Body plans, habits, position moves, extra work and rest stay as they are.
4. **Minutes decide the pace.** The work comes from the minutes he really spends on that situation: individual periods, one-on-ones, kicking stations, his own extra work, a private trainer's sessions, and for mental flaws film and installs. A team week with more individual time fixes more flaws but runs fewer team reps (prep). That is the real trade every staff makes.
5. **The room shares its individual time.** Individual periods are set per position group in the week, so a room's minutes are one budget: more time for one man comes out of another's, unless the week gives the group more (and the team fewer team reps). Extra work after practice and a private trainer add on top (proposed, not confirmed).
6. **Set on the player page.** You set his points on his Development tab (topic 9b), never on a new Training screen.

## Topic 6. How a fix lands: three stages

1. **Learn.** He works it in drills. While he's thinking about it, he is often a little worse in team periods and games in that situation: the real motor-learning dip.
2. **Carry.** It shows in one-on-ones and team periods first. Games lag behind practice.
3. **Hold.** It holds in games. Under stress (a big game, the fourth quarter when he's tired, low composure) the old habit comes back up to a third of the way, as coached-down tendencies do (`docs/design/tendencies.md`, topic 3.7). That fades over the next season.
4. **What sets the pace.** The minutes; the teacher's teaching in that group; his coachability, work ethic and football IQ (for mental flaws); and age. Young players re-groove fastest. By about 26 a habit is set, so a 30-year-old fixes slowly, using the same settle curve habits use (`training/seat.rs`, `habit_rate`).
5. **It can stall.** If he stops buying in or the minutes dry up, it stops. Dropping it partway keeps part of the gain.
6. **Tools go the same way.** Learn, then carry, then hold, with the same dip.

## Topic 7. Who teaches

1. **The position coach**, by his teaching in that group (`world/season.rs`, `room_teaching`). A coach who played the spot teaches its technique flaws better. That is the "came up in that group" link the room already reads.
2. **A veteran mentor** (people, built: `world/folk/ties.rs`). A mentor who has a tool can pass it on.
3. **Private trainers in the offseason.** NFL players, and college players with NIL money, hire them. They are real named people in the world, each with a quality and a speciality (quarterback mechanics, pass-rush moves, receiver releases). Your players choose them, and you can suggest one. Real: Jordan Palmer and Josh Allen, Chuck Smith with pass rushers.
4. **Camps for high schoolers.** Summer camps and private coaches for those who can get to them (Elite 11, college camps). Better access for richer families and bigger schools, as in real life, and noteworthy, never driving.

## Topic 8. The player's voice

1. **Buy-in.** How much of the work sticks comes from his coachability, his trust in you and his coach, his ego and his age.
2. **He can refuse.** A veteran star with ego may refuse a flaw he doesn't believe he has, and his trust in you moves with it (agreed in `docs/design/training.md`, topic 5, never built).
3. **He can ask.** Players come to you, through the talks, to ask to add a tool ("I want a spin move") or to work with a trainer. Saying yes uses a point of emphasis and builds trust. Saying no costs a little.
4. **He feels it.** When a fix holds, he says so in his one-on-ones, and his confidence and mood move with his results as they already do.

## Topic 9. Seeing the payoff (the symptoms, fixed on top)

1. **Working on** on his page: each point of emphasis with its stage, who teaches it, the minutes this week against usual, and the tape tags per game since it started, as a short trend.
2. **Practice charting.** Your staff charts the flaw's situation from your practice snaps on the engine: a receiver's drops per 100 off-frame balls, a tackle's one-on-ones against speed, a quarterback's 7-on-7 completion rate under pressure. You see it week by week.
3. **The coach's note every two weeks** on each point, written through his eye, so a weak eye can misread it. E.g. "his kick-slide is cleaner, he held up against the counter twice today".
4. **The habit chart.** A habit plan is charted from his games, e.g. "bounced outside on 18% of runs, 11% over the last four".
5. **The world notices.** Scouts, draft notes, recruiters and the press read fixes ("cleaned up his footwork"), each through their own eye.

## Topic 9b. Where it lives on screen (mock-up)

Cain 2026-11-14 12:40: "The back-end could be amazing, but how do we retain focus on the results." 13:07: "i want the graphical changes in the player page -> development, and not necessarily replacing the visual training side bar route... don't replace the current training screens, i quite like them."

Mock-up: https://claude.ai/artifact/55SqK4iz9STFrrojYn9iCq (in the app's own look: team colour wash, gold kickers, the staff-read ring, the Broadcast field; proposed, not confirmed).

1. **Everything lives on the player page's Development tab.** The Training screens in the sidebar stay as they are. Their plan columns (Plans, the squad's Development view) show his points of emphasis where they showed a rating focus.
2. **Four sub-tabs, one screen each, no scrolling** (Cain 13:10: "just stacking them beneath each other is not proper game design, perhaps tabs in development"). The tiles it has today (growth, the year, practice, load, role) become one slim strip above the sub-tabs, the same on all four.
   - **Progress** (opens first): each point with a stage ring, one hero number from games with a before and now (e.g. "beaten by the counter: 1 in 41 pass sets"), one chart of practice (dashed) and games (solid) since the flaw was seen with the dip and slips marked, the same rep then and now on the Broadcast field, the coach's latest note and what others write now. A gold dot on the sub-tab when something new happened (a point held in games, a slip).
   - **Points:** the flaws and tools the staff has seen, each with what it costs, how sure they are and the reps as proof; for the picked point who teaches, how much time (and what it costs in team reps), the coach's estimate as a range and his buy-in. The sub-tab shows how many points are set (2/2).
   - **Plan:** the rest of his plan as today (habit, body, position, extra work, rest), who teaches him, and his share of the room's individual time with a link to the room in Training.
   - **Reports:** his key ratings as ranges, staff notes and the practice log, as today.
3. **Plain words** (Cain 13:30: "Keep the sports jargon but make sure the rest are consumer friendly"). Football terms stay (kick-slide, pass block, man coverage, play action, high pad level). Everything else is everyday English: a point of emphasis shows as "Focus 1" and "Focus 2", the stages as "Learning", "Works in practice" and "Fixed in games", "flaws his coaches have seen" and "new skills he could learn" rather than tape and tool words, "Is he on board?" rather than buy-in, and the staff's numbers as guesses ("Hale's guess", "past guesses: 7 of 9 right").
4. **One palette.** The team's colour for the wash, gold for progress and the picked point, white and greys for the rest, amber only when something went wrong.

## Topic 9c. The projection before you pick (proposed, not confirmed)

Cain 2026-11-14 13:18: "i want a bit more transparency about the projection of gain. As an NCAA coach you only get certain players for a few years or less... if i have a brilliant athlete with one sore spot... very good zone coverage. But i'm predominantly man coverage team. I have to know if developing x player is gonna give me some sort of hope. It shouldn't be a perfect projection but i need some transparency."

Mock-up: the "Points, a man-team corner" board in the canvas above. Every number is the staff's read through its eye, never the hidden truth.

1. **The gain on every candidate.** Each flaw or tool in the list shows its gain in rating points in its own situation, the middle of the staff's range (a full one is about 5, topic sizes). A tool your scheme rarely uses shows its gain "for you", weighted by how often you meet its situation.
2. **What it's worth.** For the picked one: the skill in that situation now and fixed, as two ranges, and how often your scheme meets the situation (from your own calls, e.g. "you play man on 61% of snaps").
3. **In your defense.** His role in your scheme now and once it holds, from the staff's depth read, with the coach's line on the gap.
4. **Against his clock.** When it holds, as a range drawn on his remaining seasons, and the seasons of payoff left after it. A senior is told plainly when a fix likely won't land in time. More minutes move the range, and the line says by how much.
5. **The whole skill.** Where the skill itself could get by his last season, as a wider range named as a guess. It reads the hidden ceiling through the coach's eye like the potential words, so it can be wrong; the exact ceiling stays hidden.
6. **How sure.** A word for the fix and one for the whole skill, from the coach's eye, how many snaps of that situation he has seen, and how many of his past calls held. Every projection is kept, and the look-back later shows whether it came true, so you learn which coach to trust.

## Topic 10. The AI

AI staffs do the same with their own eyes, teachers and styles. A moulding staff works its system's flaws first, and an adapting one works the player's biggest. The world and your team run on the same rules.

## What the build must prove

1. **Each flaw and tool moves its own situation by its size and nothing else.** An A/B test per entry, like the links (`ab_check`).
2. **Counts match real life.** Drafted players carry 1 to 3 flaws and 1 to 2 tools. A high school freshman carries 3 to 5.
3. **Fix speeds match the agreed pace** (sizes, item 2).
4. **No level drifts.** NFL, FBS and high school averages stay where the match checks put them.
5. **The payoff shows.** On a test season, a fixed flaw's tags per game fall, and the chart and notes report it.
6. **Speed and memory.** A few bytes per player, and a small check in each engine moment. Measured on the 8 GB Mac budget.

## Sizes (confirmed by Cain, 2026-11-14 12:40: "yes, okay i like the plan")

1. **Size.** A full flaw costs about what 5 rating points of its owning attribute would, and only in its situation. A full tool adds the same. That's the same size as a full link.
2. **Pace.** With usual individual time, a willing, coachable 20-year-old with a strong teacher (teaching 15+) gets a flaw to hold in about one offseason (12-16 weeks). A poor teacher or low coachability takes a year or more. A 30-year-old takes about twice as long.
3. **The dip.** While he's learning, the flaw costs up to a quarter more in team periods and games, for 2 to 4 weeks.

## Proposed, not confirmed

- The whole list in topic 2 (entries and their engine moments).
- The starting counts (3 to 5 for a freshman) and the draft target (1 to 3 flaws, 1 to 2 tools).
- How private trainers and camps are drawn (quality, speciality, cost).
- The stress return (up to a third), borrowed from tendencies.
