# Section 6: People

The design for section 6 (roadmap: personality, morale and chemistry, off-field events, rated staff and hiring, owner goals and job security, your coach profile). Thread "make new thread for designing section 6", started 2026-10-18. Design only, nothing here is built yet.

How this doc works:
- Each topic starts with what already exists, in three groups. **Built** is in the code. **Decided** is confirmed design that isn't built yet. **New** is what this section adds. The audit with file and line references is in the project folder at `afm/section6/existing.md`.
- Real life is the default (Cain, 2026-10-16). Everything that exists and works a certain way in real football goes in as it really works, without a question. Cain on 2026-10-19: "you know i want ultra realism and these all just feel like stuff you should know".
- Numbers that no real source publishes are Claude's picks, marked **proposed, not confirmed**. They get checked in the next trial, not tuned before it.
- One person model for every level. A player is the same person at 14, in college and in the NFL, so every trait is read the same way at high school, college and the pros. A career can start at any level (`docs/handoff/decisions.md`, section 12).

Topic order (Cain, 2026-10-18; the list was checked against the code on 2026-10-18):
- **Part A, players as people:** 1 personality, 2 relationships, 3 morale, 4 chemistry and the locker room, 5 talking to players, 6 life away from football, 7 off-field trouble and outside discipline, 8 your team rules and discipline, 9 media, social media and fans (built on `docs/research/media-and-social.md`; press conferences come in here).
- **Part B, staff:** 10 staff at each level, 11 the staff market.
- **Part C, your job:** 12 bosses and their goals, 13 coaches' own lives, 14 job security and moving jobs, 15 your coach profile.

---

## Topic 1. Personality

### What exists
- **Built:** work ethic, coachability, competitiveness and love of the game (`player.rs`, drawn independently, 1-20). Ambition (`world/background.rs`, `Family.ambition`), but only one recruiting choice reads it. What a player wants (money, close to home, playing time, academics, development, prestige), first his family's decider's wants, then his own once he can choose. Hidden mental stability, with family life events and recovery. Pros' contract goals, a separate draw at signing: money, security, winning, role, city, home, loyalty, greed and temper (`world/pro/market.rs`, `Goals`). Parent styles. On the field: 12 mental ratings and the hidden consistency, temperament (after the whistle), recovery, injury proneness and longevity. Coaches: ambition, loyalty, temper and sportsmanship. GMs and owners: their section 5 traits and biases.
- **Decided:** personality is "Absolutely 10000% yes, ultra-realism" (Q19). Hardship works through personality, and a perfect upbringing with low ambition can hurt a gifted kid (section 2, topic 1.8). Character reports with "insane detail" (Q32, section 7). Contract-year effort is personality-based (section 5, question 3). Leadership's off-field half and adaptability were left to this section when the attribute list was cut (`docs/design/attributes.md`).
- **New:** one personality per person read at every level, four missing traits, real correlations between traits, and how traits change with age and events.

### How real football reads it
- NFL clubs grade character as its own line on a prospect: interviews, background checks, the people around him, and the league's Player Assessment Tool at the combine. Clubs take players off their boards over it. Scouts talk about "football character" (does he love it, work, take coaching) separately from "off-field character" (judgment, maturity, trouble).
- Personality in adults is real and measurable (the Big Five). Traits are correlated, not independent: careful, steady people also tend to be calmer and more agreeable (van der Linden and others, 2010, a meta-analysis of 212 samples).
- People mature. Conscientiousness, emotional stability and social dominance rise mostly between 20 and 40 (Roberts, Walton and Viechtbauer, 2006, 92 samples). A wild 19-year-old often settles, but not always, and life events can move a person either way.
- Self-confidence that runs into narcissism helps people get picked as leaders, but doesn't make them better leaders once they're in charge (Grijalva and others, 2015, a meta-analysis). That's the line between a loud, confident player and a real captain.

### The design
**The traits.** Every person carries these hidden 1-20 traits from age 14:

| Trait | What it is | Where it comes from |
|---|---|---|
| Work ethic | Effort in training and study | Built |
| Coachability | How well instruction sticks | Built |
| Competitiveness | Drive to beat the man across from him | Built |
| Love of the game | Keeps playing through the grind, or walks away | Built |
| Ambition | How hard he pushes for more (a bigger stage, a starting job, the big contract) or settles | Built in `Family`, moves into personality |
| Loyalty | Stays with his team, coach, school and town | Moves from pros' goals into personality |
| Greed | How much money counts against everything else | Moves from pros' goals into personality |
| Temper | Off-field anger: rows with coaches, public fights, firing his agent | Moves from pros' goals into personality; on-field temperament stays its own rating |
| Ego | Needs the ball, the spotlight and the credit; brings swagger and confidence too | New |
| Leadership | Others follow him; he steadies a room | New (its on-field half is the communication rating) |
| Professionalism | Judgment off the field: curfews, partying, his weight, rehab, being on time, study at home | New |
| Adaptability | Handles a new city, scheme, coach, level or culture | New |
| Stability | Mental health | Built (`Background.stability`) |

What each one moves, at every level (sizes in the next part). No trait decides anything alone; each one leans outcomes beside the rest (Raichel's rule: influences are noteworthy, rarely driving).
- **Ambition:** declaring early (it joins his round, family money and NIL pay in `declare_chance`), entering the portal for playing time, a pro pushing for a bigger role or a trade, whether he keeps pushing after a big payday (it joins work ethic and competitiveness in section 2's complacency risk), and how a backup takes being a backup.
- **Loyalty:** high school transfers, decommitments and flips, staying in the portal or not, hometown and re-signing discounts, staying with a coach who stuck with him, and how hard a trade or a shopping rumor hits him (section 5 already uses it there).
- **Greed:** choosing NIL money over fit, holdouts and hold-ins (section 5), and asking prices.
- **Temper:** public spats, sideline blow-ups with coaches, firing his agent, how a benching or a broken promise turns into a scene instead of a quiet word, and the trouble roll in topic 7 (fights).
- **Ego:** wanting the ball (targets, carries, the starting job), complaining in public when his role shrinks, jealousy over teammates' money and touches, getting picked as a leader early but leading less well than his pick suggests, and bouncing back quickly from his own mistakes. A big ego with big talent is a star's swagger. A big ego with less talent is a problem.
- **Leadership:** captain votes, how a team's mood holds after losses (topic 4), passing the coach's message on, mentoring young players at his position (topic 2), and calling out teammates.
- **Professionalism:** keeping his weight and conditioning in the offseason, rehab (how fast and how fully he comes back), fines and curfews (topic 8), study at home (part of how fast he learns a playbook, which already reads football IQ, work ethic and the position coach), and the trouble roll with stability (topic 7). Work ethic is effort at the facility; professionalism is what he does when nobody is watching. A hard worker can still be out on Thursday night.
- **Adaptability:** how fast he settles after a move (a transfer, a trade, a new coach, a new scheme, a first year far from home), how much a new playbook's language slows him, homesickness (it scales the existing "wants to be close to home" pull), and adjusting from high school to college and from college to the pros.
- **Stability, work ethic, coachability, competitiveness, love of the game:** as built, and morale (topic 3) now reads them too.

**Wants stay separate.** What he values (money, playing time, development, prestige, home, academics; for pros also winning, role and city) stays as built. Personality says how hard he pushes for it and how he reacts when he doesn't get it. Pros' contract goals stop being a separate random draw: loyalty, greed and temper come from his personality, and his wants carry the weights.

**Correlations.** The traits are drawn at 14 with real correlations. Professionalism goes with work ethic and stability, temper against stability and loyalty, ego against coachability and loyalty and slightly with leadership and competitiveness, and adaptability with stability. Background leans stay as section 2 built them (hardship fuel, the comfortable kid's complacency, parent styles) and stay small. Proposed, not confirmed: the correlation sizes, set from the Big Five meta-analysis (around 0.2 to 0.4).

**How traits change.**
- Growing up. Professionalism, stability and temper settle as he ages, mostly from 18 into his early thirties. Proposed, not confirmed: on average 2 points across those years, ranging from 0 to 5. Ego tends to grow with fame and money and shrinks after humbling years.
- Events move a trait by a point or two, either way. Proposed, not confirmed: a mentor stepping in or a strong veteran room (professionalism up), a death or a serious injury (stability down; for some, love of the game or work ethic up), a big payday (ambition or work ethic down for some, already built at 8%; ego up), stardom (ego up), a release or being benched (ego down, temper up for some), a strict, respected head coach (professionalism up a little each year).
- Traits never jump overnight, and a person never becomes someone else.

**Coaches, GMs and owners are people too.** Coaches keep their ambition, loyalty, temper and sportsmanship, and gain ego (taking credit, clashing with a GM or AD, refusing to change), work ethic, professionalism and adaptability (a new level, a new scheme, a new city) on the same scale. Leadership for a coach is his motivation rating, which already exists. GMs and owners keep their section 5 traits. Proposed, not confirmed: which of these each role reads is set when topics 10 to 14 are written.

**What you see.** You never see the numbers. You read a person through what he does, your staff's notes and scouts' character reports, written in words ("a pro's pro", "needs the ball", "a quiet leader", "has to be managed", "homesick last fall"), with how sure your staff is. That depends on how long they've known him and on their relationship rating. Section 7 designs the scouting side.

**Cost.** Four new traits on every person is a few megabytes across the whole world, which is nothing on an 8 GB M1.

---

## Topic 2. Relationships between people

### What exists
- **Built:** coaching trees (an assistant joins his head coach's tree, head coaches hire from their own tree, a mentor's ability rubs off), coaches' ties to high schools that travel with them, staff relationships with recruits, an older brother at a college and whether the kid follows him, a football bloodline (dad played in college or the NFL, never a real person in the world), agents' history with GMs, and news tags for a revenge game and a coach's return. Every college player keeps the staff he played under each year (`college_staff`).
- **Decided:** the portal lean moves when his position coach leaves and when friends on the team leave, not only when the head coach goes (calendar topic 3; only the head coach part is built).
- **New:** friendships, mentors, grudges and family ties as real links between people in the world, and what they do.

### How real football works
- Players follow coaches. Caleb Williams followed Lincoln Riley from Oklahoma to USC in 2022; Shedeur Sanders and Travis Hunter followed Deion Sanders from Jackson State to Colorado in 2023. In the NFL, free agents sign where a coach who knows them now works, because they already know his system.
- Head coaches bring their people. A new head coach hires assistants he has worked with, and many follow him from job to job.
- Friends move together. High school teammates commit to the same college as a package, and a friend already on a roster helps a recruiting pitch. NFL teams draft a quarterback's college receiver: Joe Burrow and Ja'Marr Chase, LSU to Cincinnati in 2021.
- Veterans mentor young players at their position, and a room's habits rub off on the rookies in it.
- Grudges are real. A player remembers the team that cut him or passed on him, and a coach remembers the school that fired him. It shows in where he signs and in what he says. There's no published evidence that a "revenge game" makes a player better on the field (inferred).
- Family runs through the sport: sons of former players are legacies at dad's school, high school coaches coach their own sons, and coaches' sons join their fathers' staffs.

### The design
**Who knows whom.** A tie links two people, with a kind and a strength from 0 to 1.
- **Teammates and staff.** Who played with whom and who coached whom comes from history (the program and season records the world already keeps), so it costs no extra memory. A close tie is stored only when two people became close.
- **Friends.** Ties grow with time together, more so in the same position room, the same recruiting class, the same high school or hometown, freshman roommates and hard seasons survived together. Personality shapes it: leaders and loyal players make more and deeper ties, and a big ego or temper makes fewer. Ties fade slowly once people are apart. Proposed, not confirmed: up to 6 close friends per player, and a tie falls by a third each year apart.
- **Mentors.** A veteran at the same position with high leadership and professionalism takes a young player under his wing. The young player's professionalism rises (topic 1, events), and he gets a small push in practice (the size is set with section 6b). A bad veteran teaches bad habits the same way.
- **Coach and player.** The coach who recruited him, his position coach and his head coach each hold a tie with him that grows with time and trust (topic 3).
- **Grudges.** A grudge is born from an event: cut, traded, benched for someone he thinks is worse, passed over in the draft, fired, a broken promise, a public insult. Its strength comes from how much it hurt and from his ego and temper, and it fades slowly while loyalty and stability soften it. It changes his choices (he won't sign there, or takes less to sign with a rival) and his words (media, topic 9). It doesn't change his ratings on game day.
- **Family.** The bloodline links to a real person in the world when one fits. A kid born around 2012 can be the son of a player from the 1996-2010 history, and later sons can be the children of players you coached. Some coaches' sons play on rosters their fathers coach, at high school, as college walk-ons and as recruits, and some join their fathers' staffs.

**What ties do.**
- **Recruiting:** package deals between high school teammates, a friend or a relative already on the roster, dad's school as a legacy pull, and the bond with the coach who recruited him. Each is a small weight next to his wants.
- **Portal and transfers:** following his recruiter, position coach or head coach to a new school (scaled by the tie and his loyalty), and close friends leaving (the decided lean events). High school transfers too, where state rules allow.
- **NFL:** in free agency, a pull toward a coach or coordinator he played well for (he knows the system, so he learns the playbook faster) and toward close friends. The pull is about as big as the city weight.
- **Staff hiring:** head coaches hire people they have worked with and trust, and assistants follow a head coach to his new job. The tree stays as built.
- **Morale and chemistry:** friends on the roster lift a player's mood, losing one (cut, traded, transferred) dips it, and grudges sit inside a locker room (topics 3 and 4).
- **News:** revenge games, reunions, legacy commitments and mentor stories come from real ties instead of tags alone.

**You are in it too.** Your former players and assistants remember you, so a tie can bring a recruit's younger brother, an assistant who wants to work for you again, or a player who won't play for you. Your coaching tree grows as your assistants get head jobs.

**What you see.** A player's page lists his notable ties ("close with Marcus Reed since high school", "played for you at Toledo", "released by Dallas in 2028"). How much you know depends on your staff's relationship ratings and how long they've been around him.

**Cost.** History gives teammates and staff for free, and only close ties and grudges are stored, a few per person, so it stays small on an 8 GB M1.

---

## Topic 3. Morale

### What exists
- **Built in pieces:** the college portal lean with dated reasons (no snaps by midseason, his head coach gone, homesick, a blue-chip signed at his spot, no bowl, the units his position leans on). Pros' trust in their team (hit by broken promises and shopping rumors), holdouts, hold-ins and trade requests on greed, loyalty, trust and temper. The NIL jealousy hit to stability. How often a player follows an instruction (it reads discipline, coachability, his trust in the coach and the coach's strictness; `tendencies.rs`, `obey_chance`).
- **Decided:** promises logged and their cost, shopping rumors hurting by personality (section 5); contract-year effort by personality, with no flat boost (section 5, question 3); what an ignored instruction does to trust and morale (tendencies, topic 4).
- **New:** one mood per player at every level that all of those read, what moves it, and what it changes.

### How real football works
- No coach sees a number. He reads body language, practice effort, who a player sits with and what he says to his position coach, and he hears it through captains, agents and family.
- What players care about: their role and snaps, being used (targets, carries), their money against the market, winning, being treated fairly and told the truth by their coaches, their health, friends, life at home, and what's said about them.
- Unhappy players show it in ways that matter: practice effort and focus, attitude in meetings, public complaints, asking to be traded, holdouts, the portal, refusing to re-sign. One unhappy star can split a room (Antonio Brown in Pittsburgh in 2018).
- Players overrate themselves. A backup with a big ego thinks he should start, so his mood comes from the gap between his role and what he believes he's worth, not his true rating.
- The effect on game-day play is small and hard to measure. Effort, preparation and focus move more than talent does (inferred; nobody has published a measure for football, `docs/research/media-and-social.md`).

### The design
**One mood, built from parts.** Each player has a satisfaction with each part of his football life, weighted by his wants and personality:
- **Role:** snaps and his place on the depth chart, against what he believes he deserves (his self-view: his real level plus an overrating that grows with ego).
- **Usage:** touches, targets and carries, mostly for ego players at skill positions.
- **Money:** NIL or salary against what he thinks the market pays him (greed weights it). It doesn't apply in high school except in states with high school NIL.
- **Winning:** results against hopes, weighted by competitiveness and ambition.
- **Coaches:** his trust in his head coach and position coach (below).
- **Team:** friends on the roster, grudges inside it and the locker room's mood (topic 4).
- **Life:** family, homesickness, health and injuries (topic 6).
- **Noise:** praise and criticism from the media, fans and social media (topic 9), weighted by ego and stability.

The parts add up to his **mood**. Every change carries a dated reason, the way the portal lean already does. Mood drifts back toward his own baseline: stability sets how far it swings and how fast it settles. Proposed, not confirmed: a bad week moves it a little, a lost starting job a lot, and it heals by about a quarter a month without new events.

**Trust in his coaches.** Separate from mood, each player holds trust (0 to 1) in his head coach and his position coach. It rises with honesty, promises kept, fair playing time, being heard (topic 5) and fair discipline (topic 8), and it falls on the opposite. It also falls when he's told one thing and given another, like being called "the guy" and then benched. Pros' trust in their team becomes this same trust, aimed at the coach and the GM. Instructions are followed more often when trust is high (built).

**What mood changes.** Each effect is small alone and adds to the others. Proposed, not confirmed, sizes in brackets.
- **Practice:** effort and focus. Development already reads work ethic, so mood scales it (up to 10% more or less growth at the extremes).
- **Game day:** focus and effort through his form. It works with the hidden consistency (at most about one rating point either way). The contract-year rule stays personality-based.
- **Decisions:** the portal lean, decommitments, high school transfers, quitting or choosing another sport (high schoolers play basketball, baseball and track too), declaring early, holdouts, trade requests, re-signing discounts, opt-outs and retirement. The existing systems read mood instead of their own scattered parts.
- **Behavior:** public complaints and social media posts (ego and temper; topic 9), and the locker room (a low-mood leader drags others down; topic 4).
- **Instructions:** the obey chance reads his trust (built).

**One system for every level.** High school mood moves transfers, quitting and other sports. College mood moves the portal, NIL talks and declaring. NFL mood moves holdouts, trade requests and re-signing. They are the same mood and the same trust, read by the decisions that exist at each level.

**What you see.** No meter. Cain turned down a mood bar or word scale for negotiations as lackluster (section 5, question 1) and asked for tells instead, so morale works the same way. You read a player through signs: practice effort, body language in meetings, who he sits with, skipped voluntary workouts, his posts, his agent or parents calling, and whispers from captains (decided, calendar topic 3). Your staff gives its read in words with its top reasons ("thinks he should be starting over the freshman"). How sure and how right that read is depends on your position coaches' and your own relationship ratings and on how long you've known him. A weak staff can be confidently wrong. Talking to him is topic 5.

**Cost.** Eight parts, trust and a short list of dated reasons per player on a roster. People outside rosters keep only their mood.

---

## Topic 4. Chemistry and the locker room

### What exists
- **Built:** line chemistry, from how many of the five linemen started together last season (350+ snaps), which decides how well they pass off stunts. "The locker room" is an incident count that slowly dents a coach's reputation. The communication rating sets pre-snap calls and busted assignments in the engine. Recruits weigh the units their position leans on. The NIL jealousy hit.
- **Decided:** units learn together, so swapping two linemen drops their chemistry for a while (plays and tactics, topic 4.9). A bad locker room chips at a coach's reputation, visibly but slowly (section 2). Whispers about the locker room vibe (calendar topic 3, Raichel: "yes definitely get whispers, it should be part of the game just like lockerroom vibe").
- **New:** the locker room as people (leaders, captains, groups, a team mood), and chemistry on the field beyond the offensive line.

### How real football works
- Captains are mostly voted by teammates, in the NFL and in college, and some head coaches pick their own. Many college programs also run a leadership council.
- A room has a pecking order: stars and veterans on top, rookies doing the chores. Position rooms, offense against defense and class years form groups. Money changes it: a rookie or a transfer paid more than proven veterans causes friction (NIL and rookie contracts both).
- One player can split a room, and strong leaders hold one together through a losing streak.
- Cohesion and winning go together, and winning builds cohesion at least as much as cohesion builds winning (Carron and others, 2002, a meta-analysis in sport).
- On the field, chemistry is reps together. Offensive linemen pass off stunts, a secondary that has played together blows fewer coverages, and a quarterback and receiver who have thrown together for years trust timing throws (Burrow and Chase reunited in Cincinnati).

### The design
**The room is the people in it.** It's built from topics 1 to 3: each player's mood, his ties and grudges, and his standing. Standing comes from his role, years on the team, contract or NIL, star status, his position (the quarterback stands highest) and his leadership.
- **Groups.** Players form groups through their ties (position rooms, units, class years, hometowns and backgrounds). A group with a strong, unhappy member can turn sour, and two hostile groups split a room.
- **Captains.** Before the season the team votes, weighted by leadership, standing, performance and ties. Ego wins some early votes (the narcissism finding in topic 1). Each head coach either uses the vote or picks his own, which is part of his style. Captains carry the coach's message, lift a slumping room, and their own mood weighs most.
- **Team mood.** Every player's mood, weighted by standing, spreads through ties. A bad mood spreads faster from a star or a captain, and leaders with good moods damp it. It moves with results, fairness (playing time, money, discipline), the coach's trust and his motivation rating.
- **Money and status.** Big money paid to a young player or a newcomer causes jealousy among the veterans at his position and among the stars. The built NIL jealousy becomes one case of this, at every level (NIL, rookie deals, a veteran's extension denied while a free agent is paid). Ego and greed sharpen it, and loyalty and leadership soften it.
- **Rookies and veterans.** Real rookie traditions (carrying pads, paying for a dinner) build ties when they're light and grudges when a veteran goes too far. Proposed, not confirmed: rare, and only as small events.

**What the room changes.** Proposed, not confirmed, sizes in brackets.
- **Practice:** team mood scales effort across the roster (up to 5% growth either way at the extremes), on top of each player's own mood.
- **After a loss:** a strong room bounces back. In a weak room, form dips together the next week, through consistency (small).
- **Decisions:** a bad room pushes portal leans, trade requests and refusals to re-sign. A good room keeps players who could leave.
- **The coach:** the room joins incidents in the reputation hit that's already decided, and a bad room is part of how bosses judge you (topic 12).

**Chemistry on the field.** Real reps together:
- **Offensive line:** as built (seasons started together), and it also grows inside a season with snaps together.
- **Secondary:** snaps the corners and safeties have played together lower busted coverages, through the same path the communication rating uses now.
- **Quarterback and receiver:** snaps together raise how confidently the quarterback throws on timing to that receiver, and how often they're on the same page on option routes. It carries across teams (a reunion keeps it) and fades over seasons apart.
- Everything stays inside the battles. Chemistry trims errors and hesitation, and never adds a flat bonus. Proposed, not confirmed: going from strangers to years together is worth about as much as 5 points of the communication rating (1-99) for that pair or unit.

**What you see.** The room in words: the captains and leaders, the groups, who's unhappy and why, and the vibe through whispers. How clear the picture is depends on your staff's relationship ratings and on your captains.

---

## Topic 5. Talking to players

### What exists
- **Decided:** in negotiations, sit-downs with a player apart from his agent, logged promises with their trust cost (by how public they were), and a wary player asking for a promise in writing (section 5, items 1e and 2). Exit meetings in January, where players learn their future (section 5, "The NFL year"). Cain moved player talks into this section on 2026-10-18 (Q43 had parked them for later). Halftime decisions belong to game day (section 9), and the talk itself is designed here.
- **Built:** nothing yet.
- **New:** the talks themselves, at every level.

### How real football works
- A head coach's week is full. He can't sit down with all 53 (or 85, or 120) players, so most daily talking is done by position coaches, and the head coach picks his moments.
- Talks come in kinds: a one-on-one in the office, a position room, a team meeting (Monday after a loss, the night before a rivalry), the pregame and halftime locker room, the leadership council, exit meetings, and, in high school and college, the parents. In the NFL, the agent is often in the loop.
- Old coaching rules hold because they work: praise in public, criticize in private; don't promise what you can't keep; players compare notes, so what you tell one gets around; a fiery speech every week stops working.
- Players hide things. A slump can be a sick parent, a breakup or an injury he's playing through, and he only says so to someone he trusts.

### The design
**Your time is a budget.** Each week (each day in the season) has a few slots for one-on-ones, next to film and the game plan. Your position coaches handle the rest every day, and you can give them a line for a player ("keep him level", "push him", "tell me if he's slipping"). How well they carry it out depends on their relationship rating.

**A one-on-one.** You pick the subject: his role, his usage, his money, his behavior, his life, the team. Then you make moves, each with a tone (calm, warm, firm or fiery):
- **Ask:** find out what's going on. Whether he opens up depends on his trust in you and his personality, and that's how you learn the reason you couldn't see (topic 3's hidden parts and topic 6's events).
- **Tell him straight:** his real role and why. Honesty costs mood now, but it builds trust.
- **Praise, challenge or criticize.** A competitive player rises to a challenge, a fragile one sinks. Criticism lands on a big ego as an insult.
- **Promise:** a role, snaps, a contract talk or a spot. It's logged and works like section 5's promises at every level. A broken one costs trust with him, his friends and the room, by how public it was.
- **Warn:** what happens if it goes on (topic 8).
- **Hand him something:** a mentor role for a rookie, a voice on the leadership council, a captaincy talk.
- **Listen:** nothing promised. It helps with players who need to be heard, and frustrates a player who wants answers.

He answers in words, with tells your relationship rating reads (eye contact, short answers, bringing up his agent, his parents or a teammate, asking for it in writing). A good read lets you change course in the same talk. A move that fits his personality and timing moves trust and mood the right way, and a wrong one backfires.

**Where you say it matters.** Private talks stay private, mostly: what you tell one player can reach others through his ties. Saying different things to two players at the same position gets found out. Public words go through the media (topic 9) and count double, good or bad.

**Talking to a group.** Team and unit talks, pregame and halftime: you pick a message (focus, calm, fire, a challenge, a reassurance) and a tone. Each player takes it by his personality and mood, so the same speech lifts some and loses others. A tone used too often wears out. Halftime talks change mood and focus for the second half (small). How they appear during a live game is section 9.

**The people around him.** In high school and college you talk to parents too (the parent styles are built: loud at games, busy on social media, calm, absent), and in the NFL to agents. Same moves, with their own personalities, and a parent or agent can carry your words back to him, good or bad.

**Exit meetings.** In January you meet each player: his future (cut, a tag or extension talk, a starting chance, a position change, the portal, the draft). For a college player it's often where he decides to stay or go. What you say can keep or lose him.

**AI coaches talk too,** with the same moves chosen by their personality and relationship ratings, so other programs' rooms rise and fall for real reasons.

### 1 question

**How does a talk play on screen?**
Real life doesn't settle this, because it's about how the game presents a conversation.
- Option A: you pick moves and tones from the list above, he answers in written lines with tells, and a talk runs 2 to 5 exchanges. It's fast, the same choice always works for the same reasons, and it's easy to prove in tests.
- Option B: you type freely, and a small language model on your Mac reads what you meant and writes his answer. It feels the most natural, but it needs about 1 to 2 GB of memory on an 8 GB M1 next to the world, it's slower, and it's harder to keep fair and testable.

My pick: A. It runs on any Mac, and your skill shows in which move you pick for which player, at what moment.
