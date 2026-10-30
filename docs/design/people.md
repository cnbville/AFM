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

### How a talk plays on screen (Cain, 2026-10-19)

Asked: picked moves and tones (A) or free typing read by a small language model on the Mac (B). Cain: "A, but make sure conversations-messages has a uge database so it doesn't get stale, which is one fo my hatrets of FM, been seing the same shit for 5 years".

- **A, confirmed.** You pick moves and tones, and he answers in written lines with tells, over 2 to 5 exchanges.
- **The lines must never go stale.** FM's problem is a small pool of fixed sentences seen for years. AFM's answer has five parts, and they apply to every written line in the game: talks, press conferences, interviews, posts, news and fan voices (topic 9).
  1. **A huge line bank.** Tens of thousands of lines, written and reviewed during development (not generated on your Mac), stored as data files that can keep growing after release. Text compresses well, so even 100,000 lines are a few megabytes.
  2. **Lines are built from parts, not picked whole.** An answer is put together from an opening, the point, a detail and a closing, each with many versions, so the same move almost never produces the same sentence twice.
  3. **Every person has a voice.** Word choice comes from his personality (a big ego talks about himself, a quiet leader keeps it short), his age, where he grew up, his background and the era (2026 slang isn't 1998 slang). Two players saying the same thing sound different.
  4. **Lines carry real facts.** Names, stats, dates and shared history get filled in from the world: last week's fumble, the sick mother he told you about in March, the coach who cut him in 2027, the friend who just transferred. Because the facts change, the line is new even when its pattern repeats.
  5. **The game remembers what it said.** Each person tracks the lines he has used with you, and nobody repeats a pattern to the same person for a long time. The bank is weighted toward lines you haven't seen yet.
- Proposed, not confirmed: the bank's starting size (about 50,000 lines across talks, media and fans) and how long a pattern rests before it can come back to the same person (a season). The build proves it by reading a few hundred generated talks and counting repeats, and the count is timed.

---

## Topic 6. Life away from football

### What exists
- **Built:** family life events for high school, prep and college players: a family move, a parent losing a job, a death, a new baby, a mentor stepping in, NIL changing the family's life. They move stability, money and home. GPA in high school, academic redshirts and spring ineligibility in college, and graduating (it frees a player to transfer and to declare). Section 2's unsafe neighborhood raises the risk of incidents, injuries outside football and losing people close to him.
- **Decided:** births, funerals, weddings, quarterback-led throwing sessions, youth camps and charity events get real dates and real absences (calendar A43, dated slots only). A parent falling ill pulls a college player toward home, and a breakup or academic trouble moves his portal lean (calendar topic 3).
- **New:** what those events are and do for everyone, pros included, and mental health support.

### How real football works
- Players miss games for the birth of a child and for funerals, and teams allow it. Offseason weddings and vacations, and quarterbacks gather their receivers for throwing sessions before camp.
- Families lean on the money. A young pro or a paid college player often supports parents and siblings, and the pressure is real. Many pros struggle with money after their careers.
- Mental health support is required now. Since 2019 every NFL club must retain a behavioral health team clinician (the NFL-NFLPA Behavioral Health Agreement, CBA Article 39). Colleges run counseling and sports psychology, more at big programs.
- College players are students: classes, majors, eligibility and a degree, which matters most to players whose pro chances are thin.
- High school athletes often play other sports (basketball, baseball, track), and some choose one over football.

### The design
**Events.** Each person's life runs on the daily clock, with real rates by age. Proposed, not confirmed, where no source gives one.
- **Family:** births (his own children, more common as he ages), weddings (offseason), a parent's or relative's illness, deaths, a family moving, money trouble at home, a parent losing or finding work. The built events stay and extend to pros.
- **Relationships:** a girlfriend or wife, a breakup or divorce, a long-distance relationship pulling him toward a city.
- **School:** for high school and college players, classes and grades through the season (the GPA trend is built), a major, academic trouble, graduating, and the degree as a reason to stay in college.
- **Money:** supporting family, a bad investment or a dishonest adviser (pros), big NIL or contract money changing family life (built for NIL).
- **Health away from football:** illness and injuries outside football (section 2's unsafe area raises the risk).
- **Community:** charity work, his own youth camp, appearances. These build fans and the Walter Payton Man of the Year case, cost a little offseason time and raise his name (topic 9).
- **Offseason habits:** throwing sessions led by a quarterback (they build quarterback and receiver chemistry, topic 4), private trainers, or a long vacation. Professionalism and ambition decide which.
- **Other sports:** high school athletes play a second or third sport by season. It splits training time (section 6b) and can win him over to another sport.

**What events do.**
- **Absences:** a birth costs a game or practices when it falls in season, and a funeral a few days, on the calendar's real dates. You can't stop it, and refusing leave is a talk (topic 5) that costs trust with the whole room.
- **Mood and stability:** each event moves his life part of morale (topic 3) and his stability, scaled by personality. A strong family, a girlfriend or wife and a good mentor steady him, and so does mental health support.
- **Focus:** a heavy week at home lowers his practice focus and his form a little, unless he opens up and gets support.
- **Decisions:** moving closer to an ill parent (decided for the portal, and now also free agency and trade requests), retiring early for family, choosing school or another sport, taking guaranteed money over a bet on himself when family depends on him.

**Mental health support.** Each team has the support real teams of its level and era have: NFL clinicians (required from 2019), counseling and sports psychologists at colleges (more at richer programs), and little in most high schools. Support speeds stability's recovery (built: support lifts the target) and lowers the risk of a crisis. Players with low stability and pride (ego, temper) use it less.

**What you see.** Only what he or his family tell you, what your staff picks up and what's public. A player who trusts you tells you about the sick parent. One who doesn't just slumps.

---

## Topic 7. Off-field trouble and outside discipline

### What exists
- **Built:** a yearly trouble roll, 1.2% a year in high school, 3% in college and 1% at prep schools (`world/season.rs`). It reads work ethic, coachability and background, and ends in a season-long suspension or dismissal (about 30% are dismissals). Draft boards take off 4 points for a suspension and 8 for a dismissal. A player on notice gets dismissed the next time. An incident count feeds a coach's reputation. Pros have no trouble at all.
- **Decided:** a suspension appeal takes weeks, and news isn't held back (calendar topics 8 and 9). Guarantee-voiding language in contracts (section 5, 1d). The rule books change by era (calendar `rules.rs`).
- **New:** the kinds of trouble, with each level's real process and penalties by era, at every level including the NFL.

### How real football works
- **The NFL.** The USA Today arrest database counted about 56 arrests and citations a year from 2007 (71 in 2006), roughly 2.5% of players a year, with DUI and domestic cases falling after 2014. NFL players are arrested less often than other men their age (a 2015 study, 2000-2013).
  - **Personal conduct policy:** a 6-game unpaid baseline for a first domestic violence or sexual assault case since December 2014, and no conviction needed. Since the 2020 CBA a jointly appointed disciplinary officer rules first, and the commissioner hears the appeal and can raise it (Deshaun Watson, 2022: the officer gave 6 games; during the league's appeal it was settled at 11 games and a $5 million fine). A player charged with a violent crime can go on the commissioner exempt list: paid leave, no practice and no games, sometimes for months.
  - **Substances of abuse:** since the 2020 CBA, no suspensions for marijuana, testing only at the start of camp, and treatment first. DUI is a 3-game suspension.
  - **Performance-enhancing drugs:** a first anabolic steroid positive is 6 games under the 2020 CBA (4 under the 2014 policy), and stimulants and masking agents 2.
  - **Gambling (2023 policy):** betting on NFL games is at least a year, on his own team at least two, and non-NFL betting at a team facility or on team travel 2 games, then 6, then a year. Fixing a game is banishment.
- **College.** Team rules first (topic 8), then the school (conduct code, Title IX), then the NCAA. A failed PED test costs a year of eligibility. Cannabis has been off the Division I banned list since June 2024. Betting on his own school's games costs eligibility for good. Academic standing decides who plays.
- **High school.** The school and the state association. Ejection rules are set by each state: the next game in most, more for fighting (California: three games and a sportsmanship course). Academic eligibility varies by state, and Texas's "no pass, no play" means a grade below 70 sits him out of games (not practice) for three school weeks.
- Charges take time. An arrest becomes a charge, court dates, a plea, a dismissal or an acquittal, over weeks or months, and the league and team decide alongside (Jabrill Peppers missed seven games on paid leave in 2024, then was acquitted).

### The design
**Trouble is an event with a kind and a process.** The yearly roll is replaced. Each person carries a daily risk that ends in a real kind of trouble, then runs the real process of his level and era.
- **Risk:** a base rate by level and age, matched to the NFL's real 2.5% a year. College and high school rates have no national data, so they're proposed, not confirmed (the current 3% and 1.2% stay as the starting point). It's scaled by professionalism and stability (most), temper (fights, domestic cases), greed (gambling, benefits), background (built), money and fame (nightlife, more chances), the room and the coach (a strong room and a strict, respected coach lower it), and age (it falls through the twenties).
- **Kinds,** weighted by the real mix: DUI and traffic, fights and assault, domestic violence, drugs and possession, weapons, theft and fraud, failed drug tests (PEDs, and recreational drugs where banned), gambling, academic misconduct, and NCAA benefits cases (mainly before NIL, like Ohio State's 2010 memorabilia case). The mix shifts by era with the real policies (for example, fewer DUIs after ridesharing and the 2014 policy).
- **The process at each level** follows the rule book of the year: the police and courts (charge, court dates, outcome), then the league or association (NFL personal conduct, exempt list, drug and gambling policies; NCAA eligibility; state association rules), then the school or club (your call, topic 8). Every step has its date on the calendar, and appeals take weeks.
- **On-field discipline:** ejections, fines and suspensions for dangerous hits follow each level's rules (the rules by era live in the match engine and calendar rule books).

**What trouble does.**
- Games and practices missed, and paid or unpaid time, by the real rule.
- Money: forfeited game checks, guarantees voided where the contract says so (section 5), NIL deals pulled, endorsement loss.
- His name: draft boards mark it (built red flags, now by kind and outcome, so an acquittal weighs far less than a conviction), and scouts' character reports carry it (section 7).
- His people: his mood, stability and trust in you, by how you handled it (topic 8), and the room's mood (topic 4).
- Your job: incidents feed the bad-locker-room hit on your reputation (built) and your bosses' view (topic 12). The media covers it (topic 9).
- Second chances are real: some programs and clubs take players others dismissed, by the head coach's style and how desperate the team is.

**What you see.** The arrest or the failed test reaches you when it happens, from police, the league or the news. Sometimes you hear it first through the media, before the player tells you, and that costs trust in both directions.

---

## Topic 8. Your team rules and discipline

### What exists
- **Built:** a head coach's strictness, read from his discipline rating and visible, so coaches build a reputation for it. In games it limits freelancing, and a stricter coach's instructions are followed more often (`tendencies.rs`).
- **Decided:** strictness is visible and players and agents weigh it ("a free-roaming DB might steer away from a strict coach", match engine 3.1). Structure or freedom is one of your seven style settings (tendencies). The trust cost of strict instructions and of discipline belongs to this section (tendencies, topic 4).
- **New:** the rules you set, what breaking them looks like, the punishments each level allows, and how the room takes it.

### How real football works
- **NFL club discipline is set by the CBA (Article 42).** A schedule of maximum fines rises each year (2025: up to $3,089 for being late to a meeting, curfew or a team activity; $822 a pound a day for being overweight), with warning letters first for small offenses. Unexcused training camp absence costs $50,000 a day ($40,000 for rookie contracts). For conduct detrimental to the club, the maximum is one week's pay and up to four weeks' unpaid suspension, and a deactivation for conduct counts as discipline (written in after Terrell Owens's 2005 case). The club tells the union, and the player can file a grievance to an arbitrator.
- **College coaches can't fine scholarship players.** They use running and early-morning workouts, lost privileges, benching, suspensions ("violating team rules") and dismissal. Programs run their own drug testing and class checks.
- **High school coaches** set team rules within the school's code: attendance, grades, behavior. Parents push back, and some go to the AD or the principal.
- What decides whether discipline works is fairness. If the star is held to the same rule as the backup, the room accepts it. Double standards are the oldest locker-room complaint, and some coaches let the leadership council set punishments so the players own it.

### The design
**Your rulebook.** You set your team's rules, each with how strictly you enforce it: curfews (the night before games, in camp), being on time, meetings and class attendance (college and high school), weight targets, dress and travel, phones and social media (a ban costs players NIL money from 2021; topic 9), your own drug testing (college), and conduct away from the facility. AI coaches set theirs from their structure-or-freedom style, their discipline rating and their personality.

**Breaking rules.** Violations come from personality (professionalism most, then temper and ego), mood (unhappy players test limits), age and the room. A long list of strict rules catches more players and wears on veterans, and a loose list lets small things grow into the off-field trouble of topic 7. You learn about a violation through your staff, and only as well as they see it (their discipline and relationship ratings), so some never reach you.

**Your responses,** within each level's real limits by era:
- A talk or a warning (topic 5).
- A fine: NFL only, on the CBA schedule of the year, with warning letters first for small offenses.
- Extra conditioning or lost privileges (college and high school).
- Benching: a series, a start or a game.
- A team suspension: in the NFL, unpaid and up to four weeks for conduct detrimental; in college and high school, games.
- A deactivation, a release or a dismissal (section 5's cap rules apply to releases; a dismissed college player can transfer).
- Handing it to the leadership council.

You can go past the NFL limit, as real clubs have, but the player files a grievance and an arbitrator can overturn it and award back pay. The union and agents remember.

**What it does.**
- **On him:** trust and mood move by how fair he thinks it was and by his personality. A proud, quick-tempered star takes a public benching as war, and a coachable young player can take it as a lesson. Firm, fair discipline from a respected coach raises professionalism over time (topic 1). Unfair discipline breeds grudges (topic 2).
- **On the room:** consistency is tracked. Punishing the backup and excusing the star for the same thing lowers team mood and trust in you across the roster. Fair, consistent rules raise them and lower the trouble risk in topic 7.
- **On games:** suspended and benched players are missing, and the depth chart and the engine see it.
- **On your name:** how you run a team becomes your reputation as a disciplinarian or a players' coach. Recruits, parents, free agents and agents weigh it (decided), and the media covers it (topic 9).

---

## Topic 9. Media, social media and fans

Built on the research Cain asked for on 2026-10-16: `docs/research/media-and-social.md` (PR #48). Its sources and labels (published, known, inferred) carry over. Facts below are from it unless marked.

### What exists
- **Built:** a plain news feed (`world/news.rs`: a date, a kind, a channel, a program and one line of text) with loudness from the calendar (small news line, medium inbox, big screen, huge full show). NFL fans' mood from -1 to 1 feeds the owner and the contend-or-rebuild call. College programs have a fan base number. The polls and the Heisman have real voters (built in the final build: `world/voters.rs`, `world/polls.rs`; the AP panel, the Coaches Poll panel, the Harris panel, the BCS computers, the playoff committee, the Heisman electorate; how they work in `docs/world-model.md`, "Poll and award voters"). Viral clips (2% a year, 6% with a social-media parent) get a kid a first look. Coach reputation is "what schools and the media think of him", and buzz comes from his units' numbers. Calendar dates exist for media days, state media polls, polls, awards, signing days and portal windows.
- **Decided:** press conferences come into this section (Cain, 2026-10-18), and the Monday press conference is on the calendar. News isn't held back after a big moment (calendar topic 8). Trade rumors before the deadline and draft smokescreens get their windows (calendar additions). Fans' general opinion feeds the contend-or-rebuild call (section 5, topic 7).
- **New:** everything else. The people who report, how stories start and spread, press conferences, players' and recruits' social media, followers and NIL, fans as a force, and what all of it feeds back into.

### The design
**The media world changes by era.** The world runs from 1996, so it follows the real timeline: newspapers and phone-line recruiting services in the 1990s, Rivals (1998-2000), MaxPreps (2002), Twitter as the place news breaks (about 2008-2012), Instagram (2010) and 247Sports (2010), "click, don't type" for coaches and recruits (August 2016), The Athletic (2016), TikTok (2018), NIL (July 2021), On3 (2021), On3 buying Rivals (2025). News speed rises with it: the next morning's paper, then the same day online, then minutes on X. Recruits get a public voice from about 2008-2010.

**Reporters and outlets are people.** Each level has the real kinds:
- **NFL:** national insiders, team beat writers, team-employed media, local talk radio and TV, national shows and ex-player analysts, athlete-owned podcasts and draft media (mock drafts, anonymous scout quotes like Bob McGinn's poll of NFL personnel).
- **College:** beat writers, the school's own content studio, recruiting services with team sites and insiders (247Sports Crystal Ball predictions, right 80-92% of the time for the top analysts), portal reporters, hot-seat media, and the poll voters (about 60 AP voters and 65 coaches, plus the 13-person committee).
- **High school:** a shrinking local paper, MaxPreps rankings, Hudl film, local TV highlights, and recruits as their own outlets.

Each reporter has sources (who he knows: agents, executives, coaches, parents, boosters), accuracy, a reach, a leaning (toward a team or a market) and a temperament. Reporters move between outlets and retire, and new ones come up. Real names follow the name toggle (Q11).

**Stories start somewhere and spread.** A story starts with a source who has a reason to talk: an agent creating a market, a team floating a smokescreen or softening fans before a cut, a player's camp, a parent, a booster, a rival staff, a scout. Then it runs the real chain: the first report, aggregation ("per [reporter]"), reaction, then a response or a correction. Reports climb a real ladder of certainty ("exploring", "expected to", "per sources", "has agreed"), and some are wrong. How far a story goes depends on the name (attention follows fame more than performance: the 2015 study of 53 NFL quarterbacks), the market and fan base (the most intense fan bases are Buffalo, Kansas City and Green Bay, not the biggest cities), and the moment (a March story competes with free agency).

**Press conferences and interviews.** Your availabilities follow each level's real rules: the NFL's media policy (postgame within 10-15 minutes, the head coach and quarterback at the podium, fines for not talking, injury reports three times a week with fines for wrong ones), each college's own habits (and the SEC, Big Ten and ACC availability reports from 2024), and local reporters in high school. You answer with the same moves and tones as talks (topic 5): deflect, praise, take the blame, criticize a player (it counts double, topic 5), fire back at a critic, give away little or a lot about injuries (within the rules), or hand the opponent bulletin-board material. Reporters you treat well and feed get scoops and treat you softer, and ones you freeze out dig. Players give their own interviews, and their quotes come from their personality (ego and temper make headlines).

**Social media.** Players, recruits, coaches (you too) and parents have accounts, from the era they existed.
- **Followers** grow with fame, performance, big moments (draft night: Ashton Jeanty +35,000 in a day in 2025), market, looks (the faces design has a small hook for it) and how much he posts. Some are fake.
- **Posts** come from personality and mood: offers and commitments ("Blessed to receive an offer from..."), hype, cryptic complaints, a public trade request, late-night posting (NBA players who tweeted late played worse the next day, a sleep proxy, so it costs a little focus), and fights with fans. The NFL rule stops posting from 90 minutes before kickoff until after interviews (2009 on), and your own team rule can ban it (topic 8).
- **Recruits' signals:** posting a school's hashtag made a recruit about 3 times as likely to sign there, and each coach on a staff he follows made it 47% more likely (the University of Iowa study). These are tells for every staff, yours included, and section 7 uses them.
- **NIL is a social media market:** a player's NIL value starts from followers per platform and engagement, then adds performance, TV exposure and deals (the On3 model). Most deals are small posts, and most money comes from collectives (about 82%). The world's NIL code reads this value.

**Fans.** Each team has a fan base with a size, a passion and a mood. NFL fans' mood is built, and it extends to every college and high school community (and to boosters, topic 12).
- **Mood** moves with wins against hopes, rivalries, hires and departures, star signings and scandals.
- **Fan voices** (callers, posts, comments, signs) are drawn when a screen opens, from the facts and the mood, so they cost no memory. *Built 2026-10-30 in the fan lane (`world/folk/fans.rs`): the town after a game, the call-in show, the message board and the crowd with its signs, at all three levels; its sizes are proposed, not confirmed; how it works in `docs/reference/how-it-works/people.md`, "Fan voices". Lane 13f writes the full lines.*
- **Money follows:** attendance rises with this season's and last season's wins, bowl and playoff history and rivalries (last season matters most early on), and donations jump after surprise wins (about +28% after a five-win jump, Anderson, NBER). Both feed budgets (topic 12).
- **Pressure:** social media lets fans push a school or club to act fast (Rutgers took a day to go from suspending to firing a coach), so fan mood is one input to firing decisions (topic 14).
- **Abuse is real:** the NCAA's 2024 pilot study confirmed 5,000+ abusive or threatening messages aimed at 3,164 athletes, and 70% of surveyed NFL players had been messaged about betting or fantasy losses (peaking in fantasy playoff weeks). Harassment went with depression in one study of D1 athletes, so it hits stability for players who read it (low stability, high ego).

**Polls and award voters are media people** (built, final build). The built polls get real voters with their published habits: they favor their own state and conference (less than before 2007) and anchor on the preseason rank. Award voting follows the same idea for each award's real electorate.

**What coverage feeds back into.** Proposed, not confirmed, sizes small, because nobody has measured them:
- **Morale:** praise and criticism are the "noise" part of mood (topic 3), weighted by ego and stability. Players who say they don't read it mostly don't.
- **Reputation:** coach and player reputation move with coverage (attention follows fame).
- **Recruiting and draft stock:** rankings, predictions and anonymous scout quotes shape perception, and boards weigh them a little.
- **Negotiations:** leaks create or kill markets (section 5's reported interest and bluffs).
- **Job security:** fan and media pressure is one input next to results and money (topics 12 and 14).

**What you see.** A news screen by level and era: your beat, the national stories, your recruits' feeds and the fan reaction, all marked with how sure each report is. You can follow reporters and players, and your staff flags what matters.

**Cost.** The world stores events, not text: about 32-64 bytes for who, what, when, source, how sure and outlet, and the lines are written when a screen opens (from the line bank of topic 5: parts, voices, real facts and memory, so news and fan voices don't go stale either). At 2-5 million events a year, past seasons keep only the loud ones and summarize the rest, and fan voices are never stored. The research's sizes get timed in a proof before the build commits to them.

---

## Topic 10. Staff at each level

### What exists
- **Built:** college staffs of 11: a head coach, two coordinators and 8 position coaches (specialists included). NFL staffs of a head coach and two coordinators, plus one overall staff number per club for development. One head coach per high school, with no staff. Each coach has hidden ability (teaching for each position group, scheme, game management, motivation, discipline, recruiting, network, relationships; `world/coach.rs`), calling ratings for the play caller, a learning ceiling, a mentor (his coaching tree) and a public reputation. The game shows the reputation, never the ability. GMs and owners are section 5's.
- **Built, as stand-ins:** a college's facilities number (0-1) does the work of the support staff. It is 15% of the development a player gets, and it sets how much his development swings, how much academic help he gets (grades, eligibility), and the quality-control part of film study. NFL film study uses the owner's budget the same way (`world/season.rs`, `world/college.rs`, `world/pro/games.rs`).
- **Decided:** staff are rated and affect development, injuries and scouting (Q29). Owners budget for staff, scouting and facilities (Q28). Your medical staff estimates return dates (Q39). Your staff gives analytics advice in games (Q37). A scouting staff of a director, area scouts and cross-checkers (section 7). The cap manager (section 5). Any task can be handed to staff, staff decide like real people and leave a reason, and staff with real responsibility grow and get hired away (calendar topic 10). A high school staff that you hire, many of them teachers (roadmap section 6 and "Your career at every level").
- **New:** the real staffs of every level and era, the jobs and ratings of the people who aren't coaches, who hires whom, where new staff come from, and what you see of them. Practice itself (how position coaches and the strength staff spend the week) is section 6b's.

### How real football works
- **The NFL has no limit on staff size, and staffs have grown for decades.** Vince Lombardi's staffs had seven or eight coaches, with no special teams, strength or quality-control coaches (Packers.com). In 2006 the Patriots had the league's smallest staff with 11 position coaches, and the Jaguars and Ravens had 17 (Boston.com). Most staffs passed two dozen coaches in the 2020s (the 2022 Vikings had 29, Star Tribune). A modern staff: the head coach, an assistant head coach, offensive, defensive and special teams coordinators, often a passing and a running game coordinator, position coaches with assistants, quality-control coaches (the entry job), coaching fellows, a head strength coach with assistants, a head athletic trainer with assistants, team physicians, a dietitian, sports science, analytics, a player engagement director, and video and equipment staff. Since 2019 every club must keep a behavioral health clinician and a pain management specialist (the NFL-NFLPA agreement). Concussions are judged with an independent neurological consultant on every sideline (since 2013) and independent spotters in the booth who can stop play. Analytics staffs went from a few teams in the 2000s to nearly all by the early 2020s.
- **College is capped by NCAA rules, and the caps change by era.** FBS programs had a head coach and 9 full-time assistants until January 2018, then 10. Up to four graduate assistants and up to five strength coaches. Staff outside those 11 could break down film, scout and help in the office but not coach on the field, until June 2024, when the NCAA let any staff member coach (only the head coach and 10 assistants may still recruit off campus). Off-field staffs grew fast in the 2010s at rich programs: personnel and recruiting directors, analysts (Alabama hired fired head coaches Steve Sarkisian and Mike Locksley as analysts in 2016), creative and social media staff, and from about 2023 general managers who run the roster and the NIL and revenue-sharing money. Strength coaches must be certified (NCAA, 2015), and top ones earn more than many head coaches (Iowa's Chris Doyle, about $800,000 in 2019). Dietitians spread after the NCAA allowed unlimited meals in 2014. Sports medicine and academic support belong to the athletic department, not the coach, and in the Power Five the team doctors' decisions can't be overruled by coaches (independent medical care, 2017). FCS and lower divisions have smaller staffs and budgets.
- **High school staffs are teachers.** Most assistants teach at the school and coach for a stipend of a few thousand dollars, and some states allow outside "walk-on" coaches with a coaching certificate. Small schools have a head coach and two to four assistants, and big programs field varsity, JV and freshman staffs (large Texas schools carry 15 or more coaches, and the head coach is often the campus athletic coordinator). Medical care is uneven: 37% of public high schools have a full-time athletic trainer, 32% part-time and 31% none (private schools 27%, 28% and 45%; the CATCH ON study, 2015-2018 data). Every state has had a youth concussion law since 2014 (Washington's Lystedt law was the first, in 2009): a player with a suspected concussion sits until a medical professional clears him.
- **Who hires whom.** An NFL head coach picks his coaches within the owner's budget, with the GM or owner having a say depending on the club's power structure. A college head coach hires his assistants, strength staff and support staff within the athletic director's pool, but not the doctors and trainers. A high school head coach recommends, and the principal and the district hire, usually as a teacher first.
- **The ladder.** Most coaches are former players. They start as graduate assistants or quality-control coaches (Bill Belichick started in 1975 as a Colts assistant for $25 a week), become position coaches, then coordinators, then head coaches. High school head coaches jump to college (Gus Malzahn, Springdale High School to Arkansas offensive coordinator in 2006; Art Briles, Stephenville High School to Texas Tech in 2000). Fired head coaches rebuild as analysts or coordinators.

### The design
**Staff are people.** Every staff member is a person with topic 1's traits (the coach set) and the ratings his job needs, on the same hidden 1-20 scale. Coaches keep the built ability and calling ratings. The other jobs get their own ratings, and replace the stand-ins above:

| Department | Jobs (by level and era) | Ratings | What they feed |
|---|---|---|---|
| Coaching | Head coach, assistant head coach, coordinators (offense, defense, special teams), position coaches and their assistants, quality control, graduate assistants, fellows, on-field analysts (college from June 2024) | Built: teaching by group, scheme, game management, motivation, discipline, recruiting, network, relationships, calling | As built: development, game plans, play calling, recruiting. Special teams gets its own coordinator where the era and budget have one |
| Performance | Head strength coach, assistants, dietitian, sports science | Strength development, speed and conditioning, injury prevention, nutrition (weight and body) | The physical side of development (it takes the facilities number's place; facilities stay as the building and equipment), weight targets, soft-tissue injury risk, fatigue in games and late-season wear |
| Medical | Head athletic trainer, assistant trainers, team physicians, behavioral health clinician | Diagnosis, rehab, return-to-play judgment; the clinician's counseling | The return estimate's accuracy (Q39) and catching injuries early, how fast a player recovers, re-injury risk, how fast stability recovers after a hard event (topic 6), spotting a player in trouble |
| Football operations | Analysts and quality control, video, personnel and recruiting staff (college), college GM (from about 2023), and section 5 and 7's front office (GM, cap manager, scouts) | Opponent study, self-scouting, analytics (in-game advice), evaluation, organization; the GM's evaluation and negotiating (section 5) | The film study a staff does each week (it takes the budget's place), spotting your own tendencies before opponents do, fourth-down and two-point advice (Q37), recruiting boards and visits (section 7), the roster and money talks |
| Player support | Academic advisers and tutors (college), player engagement director | Academic support, life skills | Grades and eligibility (it takes the facilities number's place), trouble risk (topic 7), how fast a young player settles (adaptability), money and NIL help |

Video and equipment staff are named people with no ratings: they matter to a real team, but not to a decision you make.

**The doctors are independent where the rules say so.** On concussions nobody can overrule them: in the NFL (independent consultants), the Power Five (since 2017) and every state's high schools (by state law, from 2009 to 2014 by state). Before those rules, and for other injuries, a head coach can lean on his medical staff and a player can play through pain (match engine topic 6): it raises re-injury risk, and a trainer with good judgment pushes back. Where a high school has no athletic trainer, nobody is checking, and injuries are found late.

**Sizes follow the rule book and the money.** Each level and era uses its real limits (the NCAA's 9 then 10 assistants, four graduate assistants, five strength coaches, on-field analysts from June 2024, smaller lower divisions). Below the limits, the budget decides: the NFL owner's staff budget (Q28), the athletic director's salary pool and support budget (topic 12), and a high school's district stipends, teaching slots and booster club. A rich program carries dozens of analysts, a poor one has graduate assistants doing everything, and a small high school has three coaches who also teach history. Proposed, not confirmed: the counts the rules don't set, by level, school size and budget, starting from the real examples above. Within a department the head counts most, and his assistants decide how many players get real attention: one trainer for 100 players can't see them all (proposed, not confirmed: the head is about 60% of a department's effect).

**Who hires whom follows the level.** You hire the people who report to you, and department heads propose their own assistants for your yes. In the NFL your say depends on the club's power structure (section 5). As a college head coach you hire your coaches, strength and support staff, but the doctors, trainers and academic staff are the athletic department's. As a high school head coach you recommend, and the principal hires, usually into a teaching job, so a good coach can be lost because the school has no opening in his subject. Staff hiring and firing themselves are topic 11.

**Staff at work.** Every staff member does his job every day, and you hand off what you choose (calendar topic 10, decided): a coordinator calls plays and builds game plans, position coaches set their part of the depth chart, the strength coach runs the offseason, the trainers run rehab, the analysts write the opponent report. Each leaves his work in words with a reason, and his read is only as good as his ratings and how long he has known the player: a trainer's "four to six weeks, maybe sooner", a strength coach's "he didn't come in this summer", a position coach's "start the veteran, the rookie still blows his checks". Staff disagree with each other and with you, from their ratings and personalities: the old-school coordinator trusts veterans, the young analyst wants to go for it on fourth down.

**Staff and players.** The strength coach is the person players see most in the offseason, and a respected one sets the culture. Staff build ties with players (topic 2): a trusted position coach lifts his players' trust in the program (topic 3), players follow a coach who leaves (topic 2), and recruits commit to the coach who recruited them (built).

**Where staff come from.** Real coaches are mostly former players, so new coaches come from the world's own retired players at every level: a college player who washed out becomes a graduate assistant or a high school coach, an NFL veteran becomes a position coach. Who goes into coaching follows the person (love of the game, leadership, football IQ, how his playing career ended) and his ties (his old coaches hire him). Coaches' sons follow their fathers (topic 2). Trainers, doctors, analysts and the rest come in as new people. Everyone climbs the real ladder: graduate assistant or quality control, position coach, coordinator, head coach; high school head coaches to college; fired head coaches to analyst jobs. Growth and poaching are decided (calendar topic 10), and the built mentor effect stays.

**What you see.** Staff ratings are hidden, like the built coach ability. You judge your staff by their work: how often a trainer's estimate held, how a position coach's players grew, how right the analysts' report was, what other players and coaches say about them, and their reputation. Your read sharpens the longer they work for you. Other teams' staff you know only by reputation and results.

**Cost.** About 115,000 staff people across the world (about 2,000 in the NFL, about 22,000 in college, about 90,000 in high school), each a few hundred bytes: tens of megabytes, fine on an 8 GB M1. Most of their work is a few numbers a week, far cheaper than games.

---

## Topic 11. The staff market

### What exists
- **Built, for the AI:** the coaching carousel at every level: Black Monday, college searches that run day by day, interim coaches, in-season firings, NFL hires after the conference title games with a playoff team's coaches held until it is out. College head coaches sign contracts with buyouts. Pay by level, role, prestige and era. A coach weighs an offer on money, prestige, a first head job, the NFL's pull, his recruiting edge, the roster he'd leave, his age, home and his seat, and his ambition and loyalty decide (`world/staff.rs`, `weigh_offer`, which writes his reason in plain words). Head coaches hire from their own tree. A hidden hiring bias against coaches of color by era, calibrated to the real counts; your own hires never carry it (`world/ethnicity.rs`). The Rooney Rule's compensatory picks (`world/pro/trades.rs`).
- **Decided:** hiring follows the real interview rules (calendar topic 9). Staff with real responsibility grow and get hired away (calendar topic 10).
- **New:** your side of the market: finding candidates, interviews, offers and counters, contracts for every staff job, blocking and allowing moves, firing your own staff, and the Rooney Rule's interview rules, for you and the AI alike.

### How real football works
- **NFL permission rules.** A club can't stop an assistant from interviewing for a head coaching job. Since May 2020 it also can't stop him interviewing for a "bona fide" coordinator job elsewhere (one with a real part in the game plan, play calling not required), and can't write a right to match into his contract; the commissioner settles disputes. Lateral moves can be blocked while he is under contract. Timing since 2022: other clubs' coaches can't be interviewed until the third day after their season ends, and not in person until the wild-card games are over; a playoff team's coaches can't be hired until it is out.
- **The Rooney Rule.** Since 2003 a club hiring a head coach must interview a minority candidate (the Lions were fined $200,000 in 2003 for skipping it when they hired Steve Mariucci). It grew to general managers (2009), then in 2020 to two external minority candidates for head coach, one for each coordinator job and one for senior front-office jobs, with women counted from then on. In 2022 one of the head coach interviews had to be in person, and every club had to carry a woman or minority offensive assistant. Clubs that develop a minority head coach or GM who is hired elsewhere get compensatory picks (2020).
- **NFL contracts.** Assistants usually sign for two or three years. A new head coach brings most of his own staff, and the club pays off the rest of the old staff's contracts.
- **College.** Assistants sign contracts, and top coordinators get multi-year deals with buyouts both ways. The hiring school often pays a departing coach's buyout. The early signing period (December 2017) pulled the head coaching carousel into late November and early December, so many coaches leave before their bowl game and an interim coaches it. Athletic directors use search firms. A new head coach brings his staff but often keeps one or two who know the roster and the recruits, and players leave after a coaching change (built, the portal lean).
- **High school.** The job is posted, a principal or AD and a committee interview, and the school board approves, usually after the season and before teaching contracts are signed in the spring. A coach needs a teaching opening in his subject unless the state allows walk-on coaches. Stipends follow the district's schedule, so coaches move to richer districts.
- **Coaches have agents.** A few agents represent many of the top coaches (Jimmy Sexton's college clients, Nick Saban among them). They shop interest to drive up pay, and an offer elsewhere is the usual way to a raise at home.
- **What a coach wants besides money:** play calling, a title (assistant head coach, a passing game coordinator), the staff he can bring, years and security, the head coach's scheme and fit, and a path to a head job ("head coach in waiting", like Will Muschamp at Texas in 2008).

### The design
**Finding candidates.** Your list starts with who you know: your coaching tree, coaches and former players you worked with (topic 2 ties), names your staff recommends, and coaches' agents pitching their clients. Anyone else you know only by reputation and results, and your read of a stranger is rough. You can search by role, level and the things you can see (scheme, history, reputation, where he recruits), and the AI searches the same way. College athletic directors can hire a search firm, which brings names you don't know at a price.

**Interviews** use the talk system (topic 5): you ask about his scheme and how he'd use your players, a player you're worried about, who he'd bring and where he recruits, and he asks about play calling, money, title, years and your job security. You read him through his answers, tells and references from people who worked with him (sharper when they're in your network). A confident talker interviews better than he coaches (ego, topic 1), so a great interview isn't proof. The rules of the level and era apply: permission, timing and the Rooney Rule's interviews in the NFL (the league fines a club that skips them, as it did the Lions), the NCAA's staff limits (topic 10), and the principal, a teaching opening and the board in high school.

**Offers and counters.** An offer is money within your budget (topic 12), years, a title, play calling, the staff he can bring and a buyout. His team can counter with a raise, a better title, play calling or a promise of the head job, and his agent shops your offer. He decides through the built offer logic, which now reads the whole person (topic 1) and his ties (topic 2), and gives his reason in words either way.

**Contracts by job and level.** NFL assistants sign for two or three years. College coordinators and top assistants sign multi-year deals with buyouts at programs that pay them, and the rest year to year. Support staff are year to year. A high school coaching job is a yearly stipend on top of a teaching contract. Proposed, not confirmed: pay for jobs the built pay table doesn't cover (strength coaches, trainers, analysts, GAs), from the real salary databases by level and era.

**Losing staff.** Your staff draw interest the way the AI's do. Where the rules let you block a move (a lateral one in the NFL, a contract with a buyout in college), you choose: block it (he stays, unhappy, and his agent remembers), counter, or let him go with your blessing (he remembers that too, topic 2). When a coordinator leaves for a head job he hires from his tree, so he can take your best assistants, the recruits he was close to can follow him (built relationships), and his players feel it (topics 2 and 3). A staff that wins gets raided, as real ones do.

**Firing your own staff.** You can fire anyone, at any time, within the level's rules. You owe the rest of his contract (paid from the budget, topic 12). The room reacts by what the players thought of him (topic 3): firing a coordinator they loved costs trust, and firing one they blamed can lift it. A midseason firing is big news (topic 9), and blaming an assistant for your own failures is seen through. Bosses can push you to fire someone, or block you (topic 12). A fired coach can hold a grudge (topic 2).

**The AI uses the same market.** AI head coaches find, interview, offer, counter, block and fire under the same rules, from their personality and ratings. The built carousel gains assistants' contracts, counters, permission rules and coaches' agents.

---

## Topic 12. Bosses and their goals

### What exists
- **Built:** the NFL owner's hidden traits (meddling, a pet player, will to guarantee cash, patience, the pull to make a splash, overriding the draft room, trust in his GM), the fans' mood, and each club's power structure and contend-or-rebuild call (section 5, `world/pro/office.rs`). The college athletic director (judgment, patience, since when), who reviews his head coach against the roster he was given, with a honeymoon, the buyout's weight against his budget, prove-it years and extensions, and who moves on himself (`world/staff.rs`, `college_review`, `ad_turnover`). Athletic budgets by division and prestige. Boosters and NIL collectives with a big booster who can walk away, and rogue boosters before 2021 (`world/program.rs`). A high school review by "the principal and boosters" from the last two seasons (`world/staff.rs`, `hs_review`).
- **Decided:** owners budget for staff, scouting and facilities now, with full club finances later (Q28). Owner meetings: preseason goals, a bye-week check-in and an end-of-season review (calendar A44, dated slots). Owner goals and job security were always in section 6 (roadmap).
- **New:** every boss as a person, the goals each one sets and how he judges them, what you can ask him for, the college president and board, what boosters want, the high school principal, AD, school board and booster club, and how bosses change over time.

### How real football works
- **The NFL owner hires and fires** the head coach, directly or through his GM, and sets the football budget. Owners differ hugely: the Rooneys have had three head coaches since 1969 (Noll, Cowher, Tomlin), while impatient owners churn coaches every two or three years. A few clubs have a team president between the owner and the football side (the Packers are owned by shareholders, with a board and a president). Owners sell or pass clubs to heirs, and a new owner often brings a new coach (the Pegulas bought the Bills in 2014; the Walton-Penner group bought the Broncos in 2022).
- **The college athletic director** hires and fires, with the president's sign-off, and public schools' boards approve big contracts. Contracts carry real goals as bonuses: bowl games, conference titles, the playoff, rankings, and academic marks (the NCAA's Academic Progress Rate, since 2004, which can cost a team the postseason). Boosters fund buildings, buyouts and, since 2021, NIL collectives, and big ones get access and a voice (T. Boone Pickens gave Oklahoma State $165 million in 2005). They can stop a hire: Tennessee's deal with Greg Schiano collapsed in a day in 2017 after fans, boosters and politicians objected. Since the House settlement (2025), schools pay players directly (about $20.5 million a school in 2025-26), so the AD and the program's GM manage that money too.
- **The high school head coach** answers to the principal and the AD (in Texas he is often the campus athletic coordinator himself), and above them the superintendent and the school board. A booster club raises money for equipment, meals and travel. Parents push, and some go to the AD, the principal or the board. Schools judge their coach on more than wins: grades and eligibility, behavior, how many kids come out for the team, and the community.
- **Goals get negotiated.** Coaches manage expectations in public and with their bosses, and they ask for things: a bigger staff pool, an indoor facility, NIL money, a non-conference schedule they can win, an extension to show recruits they're staying.

### The design
**Every boss is a person** with topic 1's traits, the built ones kept: the NFL owner (his section 5 traits), the team president where a club has one, the college athletic director (judgment and patience, built) and president, the high school principal, AD and superintendent, and the big boosters and booster club leaders. Each one has trust in you, which moves with results against his goals, how you handle trouble (topic 7), money, the fans and the media (topic 9), and how you treat him (going around him, leaking to the press, or keeping him informed).

**Goals are set in meetings,** on each level's real calendar (the NFL's A44 meetings; at a college the AD before the season and after it; at a high school the principal at the start of the school year):
- **What a boss wants** comes from the team's situation and his own personality: a playoff run or a patient rebuild (the built contend-or-rebuild call), beating the rival, a bowl or a conference title, recruiting rankings, filling the stadium, a clean program (no NCAA cases, no arrests), graduation and the Academic Progress Rate, staying under budget, and at a high school grades, numbers and behavior.
- **You negotiate them.** You can argue for a lower bar (a young roster, injuries) or take on a higher one for more money and time, and your record of hitting what you promised builds his trust.
- **College contracts** write some of these goals in as bonuses (bowls, titles, the playoff, academic marks), and they're part of your contract talks (topic 14).
- He tells you in words, never as a score (section 5's rule: no meters). How he feels shows in what he says, what he grants and what the media hears from his side (topic 14).

**What you can ask for.** More staff money (topic 10), facility projects (the built facilities number now moves with them), recruiting and travel budget, NIL and revenue-sharing money (college), schedule changes (college non-conference games are the AD's, with your input), roster spots and practice-squad money (NFL), an extension or a raise. His answer reads his money (the built athletic budgets and boosters, the owner's cash and budget), his trust in you, and his personality.

**Bosses meddle.** The owner's built meddling (taking over talks, protecting a pet player, overriding the draft room) applies to you. A booster can push for a player, a coach or a firing, and pull his money if ignored (the built big booster who walks away). A president can step into a scandal. A high school principal can overrule your discipline when a parent complains (topic 8). The school board can block a hire you want. How hard each pushes comes from his personality and power.

**Bosses change.** Owners sell or die and heirs take over, at real rates. College ADs move on (built) and presidents too. High school principals turn over often. A new boss didn't hire you, reads you fresh, and is quicker to make his own hire (the built new-AD effect, now at every level).

**AI teams** have the same bosses with the same goals, so the AI's carousel comes from the same meetings and reviews.

---

## Topic 13. Coaches' own lives

### What exists
- **Built:** a coach's age, retirement from 60 (4% a year at 60, 3% more each year after), and stepping away before 60 for his health (likelier with age) or burnout (likelier the longer he has carried a big head job, and with a hot temper; `world/staff.rs`, `early_exit`). Going home pulls him and leaving home pushes back when he weighs an offer.
- **Decided:** dated slots for births, funerals and weddings (calendar A43). One career of 50 years or more (Q2).
- **New:** coaches' families, health beyond one roll, workload, money, coaches' own trouble, and what happens after they retire. The same life for you.

### How real football works
- **Health.** The job is long hours under pressure, and it shows: Urban Meyer resigned at Florida in December 2009 after chest pains, turned it into a leave the next day, and left for good a year later; in 2013 Denver's John Fox had heart surgery during the season and Houston's Gary Kubiak collapsed at halftime with a mini-stroke, and both missed weeks while an interim coached. Coaches have slept in their offices in season (Joe Gibbs, Dick Vermeil).
- **Burnout and comebacks.** Dick Vermeil left the Eagles in 1983 citing burnout, came back with the Rams in 1997 and won the Super Bowl in his third season. Joe Gibbs retired in 1993 and returned to Washington in 2004.
- **Family.** Coaches move their families every few years. A child's last year of high school, a spouse's career and roots make them turn jobs down or wait. Coaches' sons follow them into coaching (the Bowdens, the Harbaughs) and onto their staffs and rosters, and that can cost trust: Iowa wrote scoring and win targets into offensive coordinator Brian Ferentz's 2023 contract, his father's staff.
- **Trouble.** The rules of topic 7 apply to coaches and owners too. Arkansas fired Bobby Petrino in 2012 after a motorcycle crash exposed an affair with a staff member he had hired. Jim Tressel resigned at Ohio State in 2011 over a cover-up. The NFL suspended Colts owner Jim Irsay six games in 2014 after a DUI arrest.
- **Age.** Coaches work late in life: Nick Saban retired at 72, Bobby Bowden was pushed out at 80, Joe Paterno was fired at 84 and Pete Carroll took the Raiders job at 73. After coaching, many go into television, consult, or come back.

### The design
**Every coach has a life,** AI coaches and you alike:
- **A family** drawn with him (or set in your profile, topic 15): a spouse or not, children and their ages, and where the family's roots are. It moves him when he weighs a job (the built home pull now reads it): a child in his last year of high school, a spouse with a career, aging parents. Family events land on the calendar's slots (births, weddings, deaths), and a coach can miss a game for one, with an interim calling it. Sons can grow up into the world's players and coaches (topic 2), and hiring them carries the room's view of fairness (topic 8).
- **Health** replaces the single roll with a state that reads age, stress (the pressure of the job, losing, the hot seat), workload and his own habits (professionalism). Events follow real kinds and rates by age (heart trouble, strokes, cancer, exhaustion): a few days off, a medical leave of weeks with an interim (Fox, Kubiak), stepping away (Meyer), or retiring. Proposed, not confirmed: rates from men's health data by age, raised by stress, starting from the built numbers.
- **Workload and burnout** keep the built rule and add his hours: a work ethic that never stops and a temper wear him down faster, and a staff he trusts with real work spares him (topic 10). A worn-out coach decides a little worse (proposed, not confirmed: a small hit to game management and motivation while it lasts). Some who step away come back years later, at real rates.
- **Money.** Career earnings build up. A rich coach with little ambition retires sooner, and one who needs money takes jobs he otherwise wouldn't.
- **Trouble** runs on topic 7 for coaches, owners and staff, by their level's rules: arrests, affairs with staff, NCAA cases and cover-ups (show-cause orders are built), the NFL's personal conduct policy (it covers coaches and owners), and the same media and boss fallout.
- **Retirement** reads health, age, family, money, love of the game, results and his boss's push, instead of a flat rate from 60. Retired coaches stay in the world: TV analysts, consultants, references, mentors to their tree, and some come back.

**Your own life.** You get the same: a family from your profile, events you decide on (miss the game for your child's birth, or not), health and burnout from your hours, which your staff and the calendar's holiday mode take off you (calendar topic 10). Your health never ends your career for you (Cain, 2026-10-19: "imaging spending months on a save and it gets ripped from you"). It can put you on leave for weeks with an interim coaching, and your doctor and family can push you to step down, but only you end the career.

---

## Topic 14. Job security and moving jobs

### What exists
- **Built, for the AI:** the athletic director's review with a honeymoon, buyouts, prove-it years, extensions and non-renewals; high school reviews; NFL head coaches fired by the owner's patience; the GM's hot seat; in-season firings with interim coaches; the carousel at every level, with offers weighed in plain words; the hiring bias by era; coach buzz from his units' numbers that gets high school coaches college jobs (`world/staff.rs`, `world/pro/office.rs`).
- **Decided:** you can be fired and hired elsewhere, at any team, for 50 years or more (Q2, Q3). One career across high school, college and the NFL (decisions section 12). Section 5's negotiation system and your negotiating rating.
- **New:** your side: how you read your standing, your contract talks, being fired, the offers that come to you, interviews for your next job, and moving between levels.

### How real football works
- **The hot seat is public.** Media hot-seat lists, the boss's "vote of confidence", boosters talking, recruits and their parents asking whether you'll be there, and decommitments when the answer looks like no.
- **Firings follow a calendar.** The NFL's Black Monday after the regular season, college firings after the last regular-season game (and more often during the season lately: Texas A&M fired Jimbo Fisher in November 2023 and owed him about $76 million, with no offset), and high schools after the season, often by not renewing the coaching stipend while the coach keeps his teaching job.
- **A fired coach is paid.** College contracts are mostly guaranteed. An offset clause cuts what the old school owes when he takes a new job, and its absence (Fisher) makes the old school pay it all. NFL clubs pay out the contract, usually with offsets.
- **What fired coaches do:** wait it out on the payout, go into television (Jon Gruden at ESPN, Urban Meyer at ESPN and Fox), rebuild as an analyst (Alabama's "home for wayward coaches"), take a coordinator job, or step down a level.
- **Moving between levels is common.** High school to college: Gerry Faust went from Cincinnati's Moeller High School straight to head coach at Notre Dame in 1981, Gus Malzahn to Arkansas's offensive coordinator in 2006. College to the NFL and back: Jimmy Johnson (Miami to the Cowboys, 1989), Nick Saban (Dolphins 2005, then Alabama 2007), Pete Carroll (USC in 2001 after the NFL, back to Seattle in 2010), Chip Kelly (Oregon to the Eagles in 2013, then UCLA in 2018), Jim Harbaugh (Stanford, the 49ers, Michigan, the Chargers). A player can go straight to a head job on his name: Deion Sanders took Jackson State in 2020 after coaching high school.
- **Leaving.** A college coach can leave any time and owes his buyout, which the new school often pays. An NFL head coach under contract can't take another NFL job without his club's permission, so clubs trade coaches for picks (the Buccaneers gave the Raiders two firsts, two seconds and $8 million for Jon Gruden in 2002; the Broncos gave the Saints a first and a second for Sean Payton in 2023). A high school coach leaves at the end of the school year, around his teaching contract.
- **Contracts are signals.** College coaches push for extensions because recruits avoid a coach in the last years of his deal. Contracts carry pay, years, buyouts both ways, offset language, bonuses (topic 12), the staff pool and facility promises.

### The design
**Your standing, read like a real coach reads it.** No meter (section 5's rule). You see what a real coach sees: your boss's words and what he grants or refuses (topic 12), media hot-seat lists and reports from your boss's side (topic 9), boosters and fans, recruits and parents asking about your future and some decommitting, players' agents hedging, and your staff's read, as good as their network. The built reviews decide, now with each boss's goals and trust (topic 12) in them.

**Your contract** is talked out with the section 5 negotiation system, against your boss (and his money people) or through your agent if you hire one (topic 11): pay, years, buyouts both ways, offset language, bonuses tied to the goals, the staff pool, facility commitments and play-calling or roster power (your say in the NFL power structure). An extension is also a message: recruits, players and staff read the years left.

**Being fired.** It comes on the real calendar and in the real way for your level: called into the office on Black Monday, after the last regular-season game, during the season with an interim named, or a high school's non-renewal. You're paid what the contract says, offsets included. Then the world goes on and you choose what's next: wait for offers on the payout, take a coordinator, analyst or position job, step to another level, or take time off (TV and consulting keep your name in the news). Every one of these is a real path back.

**Offers and interviews for you.** Openings come from the built carousel. Interest reaches you through your agent, your network and the news, and teams judge you as they judge every candidate: reputation, results against expectation, the buzz from your units, your network, references from people who worked with you, and the same hidden hiring bias by era that every AI search carries. Interviews use the talk system (topic 5) from the other side: the owner, AD or principal asks about your plan, your staff, your record and your scandals, and your answers count against what he wants (topic 12). You can be in several searches at once, use an offer to get a raise at home, and lose a job you wanted to a better interview.

**Moving between levels** follows the real paths in both directions, with the real gaps: a high school coach's first college job is usually a position, analyst or coordinator job unless his name is big; an NFL coach can walk into a big college job; a college coach entering the NFL faces the bigger league's skepticism. Each move carries the real costs: buyouts, permission and trades in the NFL, and the timing of each level's calendar (no high school job mid-school-year, college searches in late November and December, NFL hires after the playoffs).

**Leaving on your own.** You can resign, retire or take another job while employed, under the same rules: your buyout in college, permission or a trade for picks in the NFL, the end of the school year in high school. Your players and recruits feel it (topics 2 and 3), and leaving in the middle of a bowl run or a playoff push costs your name.

---

## Topic 15. Your coach profile

### What exists
- **Built:** your seat, which decides which news stops the clock for you (`world/mod.rs`, `Seat`; a college or NFL seat, no high school seat yet). Every AI coach's ratings, calling ratings, coaching tree, record and reputation (topic 10).
- **Decided:** you are the head coach and, in the NFL, the GM too, like a Football Manager manager (Q1). You can start at any team (Q3), at any level (decisions section 12). One realistic mode, no difficulty levels (Q7). Your background (ex-player, scout, coordinator) sets your starting reputation, and you can change your starting stats (Q8: "Yes, changeable starting stats"). Your ratings grow over your career, like FM (Q20). Your negotiating rating reads tells and bluffs, senses floors and spots traps, grows with deals, and starts from your background (section 5, topic 1). Your seven style settings: risk, structure or freedom, physical or protective, ball hunting or sound, mould or adapt, and the schemes and coordinators' pull (tendencies, "Your coaching style").
- **New:** your list of ratings, the backgrounds, how the world reads you, and how each rating grows.

### How real football works
- Head coaches come from every road: former stars (Deion Sanders, Mike Vrabel), college players who became career assistants (Kirby Smart, Sean McVay), men who never played college football (Mike Leach), high school coaches (Gerry Faust, Gus Malzahn), and personnel and analytics people on the general manager's side (most GMs started as scouts, and the Browns hired baseball's Paul DePodesta as chief strategy officer in 2016). The road shapes what a coach is good at, who he knows and how others see him at the start.
- Coaches keep learning: from the head coaches they work under (coaching trees), their own staffs, clinics and offseason visits to other programs, and doing the job. Learning slows with age, and a skill a coach stops using fades (an NFL coach's recruiting, a college coach's cap work).
- Reputation is what others believe, built from results, the road you took and what people say. It isn't your real ability, and the two can be far apart for years.

### The design
**Your ratings** are the same ones every AI coach has, so you sit on the same scale as the world, plus the two that come with your jobs:
- Teaching for each position group (quarterbacks, running backs, receivers, offensive line, defensive line, linebackers, defensive backs, specialists).
- Scheme, game management, motivation, discipline, recruiting, network, relationships.
- Play calling: skill and in-game adaptation (aggressiveness is your risk setting, not a rating).
- Evaluation: judging what a player is now and what he can become (section 7 builds the scouting on it; Q20 named "judging talent").
- Negotiating (section 5).
You see your own ratings (you set them at the start, Q8). Other people see only your reputation.

**Backgrounds** follow the real roads: former NFL player (a journeyman, a starter, a Pro Bowler or a star), former college player, never played past high school, high school coach, college position coach or coordinator, NFL assistant or coordinator, scout or personnel, and analytics. Your background sets:
- Your starting reputation (Q8), and with it which bosses take you seriously, how players and recruits receive you, and how skeptical boosters and the media are if you start somewhere big.
- Where your ratings start: your background gives you a pool of points to spread across your ratings, like FM, so an ex-star starts stronger on motivation and relationships and a career assistant on teaching and scheme (Cain, 2026-10-19, answering Q8's "changeable starting stats"). Proposed, not confirmed: the size of each background's pool.
- Your network: your background places you in the world's own history, where you played or coached and with whom, so those people (now coaches, GMs, agents and scouts) know you (topic 2).
- Your negotiating start (section 5).

**You.** Name, age, hometown, alma mater, family (topic 13) and looks (the faces design: your own photo if you want it). You have no hidden traits: you make the choices, and the world reads you from what you do. Your discipline record becomes a disciplinarian's or a players' coach's name (topic 8), your talks and press conferences a voice the media and players expect (topics 5 and 9), and your results a reputation as a winner, a recruiter, a play caller or a developer, in words ("a quarterback guru", "can't win the big one").

**How ratings grow** (Q20). Proposed, not confirmed, in its sizes:
- **Doing the job:** calling plays builds play calling, recruiting builds recruiting, deals build negotiating, coaching a position group builds teaching for it, scouting builds evaluation.
- **People around you:** a strong coordinator or position coach on your staff lifts you in his area, as the built mentor effect does for AI coaches.
- **Study:** clinics and offseason visits to other staffs, time you spend in the offseason calendar.
- **Age and use:** learning slows from your fifties, and a rating you stop using fades slowly (your recruiting after years in the NFL).
- Ratings stay on the 1-20 scale, and each point costs more the higher it gets.

**Your record** is kept like every coach's: wins and losses by level, titles, your tree (coaches who worked under you and where they went), players you developed and sent on, and the history of every job, hire and firing.

---

## Cain's answers on the two open choices (2026-10-19)

Both picks were confirmed, and they are written into topics 13 and 15.

1. **Your health never ends your career for you.** Health can put you on leave for weeks with an interim, and your doctor and family push you to step down, but only you end the career. Cain: "nah, imaging spending months on a save and it gets ripped from you".
2. **Your background gives you a pool of points** to spread across your starting ratings, like FM. Cain: "A".
