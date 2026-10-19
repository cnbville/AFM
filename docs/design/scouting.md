# Section 7: Scouting, recruiting and the draft

The design for section 7 (roadmap: your scouting staff and assignments, reports with hidden potential, vague outlooks and hints, character reports with "insane detail", the combine, pro days and private workouts, the live draft and the undrafted rush; for the college seat, recruiting as the college draft). Thread "start new thread to design section 7", started 2026-10-19. Design only, nothing here is built yet.

How this doc works:
- Each topic starts with what already exists, in three groups. **Built** is in the code. **Decided** is confirmed design that isn't built yet. **New** is what this section adds. The audit with file and line references is in the project folder at `afm/section7/existing.md`.
- Real life is the default (Cain, 2026-10-16). Everything that exists and works a certain way in real football goes in as it really works, without a question (Cain, 2026-10-19: "you know i want ultra realism and these all just feel like stuff you should know").
- Numbers that no real source publishes are Claude's picks, marked **proposed, not confirmed**. They get checked in the next trial, not tuned before it.
- One evaluation model for every level. A high school coach, a college recruiting staff and an NFL personnel department judge players the same way, from what they have seen, each with its own people, rules and calendar. A career can start at any level (`docs/handoff/decisions.md`, section 12).
- Every written line (scout reports, character notes, draft coverage, phone calls) comes from section 6's huge line bank (`docs/design/people.md`, topic 5), so nothing goes stale (Cain, 2026-10-19: "make sure conversations-messages has a uge database so it doesn't get stale").
- Potential stays hidden (Q18, Q46). Nothing in this doc ever shows the player an exact ceiling. True values are for debug and trial tools only.

Topic order:
- **Part A, how anyone judges a player:** 1 what a staff knows, 2 scouting staff at each level, 3 assignments and the scouting year, 4 where information comes from, 5 character reports, 6 medical evaluation, 7 potential, hints and comparisons, 8 learning from your own reads.
- **Part B, the NFL draft:** 9 the draft board, 10 the pre-draft season, 11 draft day, 12 the undrafted rush, 13 pro scouting.
- **Part C, college recruiting:** 14 evaluating recruits, 15 your recruiting board and moves, 16 portal, JUCO, prep and international players.
- **Part D, the high school seat:** 17 the high school coach's side.
- **Part E, following players:** 18 the "my players" list and draft coverage.

---

## Topic 1. What a staff knows about a player

### What exists
- **Built:** one scout read of the overall and potential with random error by the scout's accuracy (1-20) and the level (high school and small schools harder to read, the NFL easiest; `scouting.rs`, `scout_player`). Coaches read each attribute with an eye whose insight and misreads depend on their skill and shrink with film (`athlete.rs`, `Eye`), but only to place players at positions. Each NFL team reads pros with an error that shrinks with seasons of film (`world/pro/mod.rs`, `grade`). Tendencies show as a range plus a word (`tendencies.rs`, `TendRead`). There is no scouting noise per attribute (`docs/design/attributes.md`).
- **Decided:** you see rating ranges that sharpen with better scouts, plus hints (Q18). Every report has error that shrinks with the scout's skill and with how much he has seen: film, live games, workouts, interviews (Q31). Your board is built only from your own reports, never from true ratings, and AI teams build theirs the same way (Q31). Your own players are read through your coaches (game view, settled by real life).
- **New:** one file per staff per player, built up from what that staff has actually seen, attribute by attribute.

### How real football works
- A club's view of a prospect is a stack of written reports by different people, each dated, each from a different look: the area scout's fall report, the cross-checker's, the position coach's film study in spring, the combine, the pro day, the visit. Clubs grade on their own scales (the Patriots and Seahawks famously use their own letters and numbers), and the grade means a projected role: "starter in year one", "starter by year two", "backup", "camp body".
- Scouts see different things in different places. Film shows technique and decisions. Live games show size, speed against real opponents, effort and body language on the sideline. Practice shows work habits and how coaches handle him. Workouts measure athletic traits. Interviews show how he learns and how he talks about the game.
- Two looks by the same scout don't cancel his bias: a scout who loves a body type sees it every time. That's why clubs send a second and third man (cross-checkers, the director, the GM) to the top prospects.
- Clubs know their own players far better than anyone else's: they see every practice and every meeting.

### The design
**Every staff keeps a file on each player it looks at.** An NFL club, a college staff and a high school coach each hold their own files. A file is the staff's whole knowledge of the player:
- A read of each of the 67 visible attributes, shown as a range (Q18), like "12-15". An attribute nobody has seen yet shows as unknown.
- Measurables: exact where the staff measured them (an all-star weigh-in, the combine, a camp with lasers), "listed" where it only has the school's word (school listings run tall and heavy, a real habit).
- The position he fits now and the one he could play (built: `athlete::better_position`), with the reason.
- A grade in the staff's own words, tied to a role at its level ("a year-one starter", "a backup with starter traits", "a camp body"; for a recruit "a Power Four starter", "an FCS player"), and a level-relative read (built: "89 among seniors").
- The potential read of topic 7, the tendencies' ranges, the character read of topic 5 and the medical read of topic 6.
- The evidence: every look, by whom, when and of what kind, so you can open any report and see who wrote it.

**The ranges come from the evidence.** Each look narrows the attributes that kind of look can see (topic 4 lists which). Each person who looks has his own misread of each attribute, kept for his career (the built `Eye` keyed by person), so his second look narrows his range but keeps his lean. A second person's look brings his own lean, and the file blends them weighted by each one's skill. That's what cross-checking is for. Proposed, not confirmed: a scout's misread is about 60% his lasting lean and 40% the noise of the look.

**A file goes stale.** The ranges are what the staff knew at its last look. If a player grows over the summer, the file shows last spring's player until someone looks again, and the file says when the last look was. A big change that someone notices without a full look (a weight gain the trainer mentions, a breakout game on TV) moves the file a little and marks it "needs a new look".

**Your own players are read by your coaches.** For your own roster the file fills every day from practice, through each position coach's eye (built), so your read of your own team is sharp but not perfect, and a poor coach misjudges his own room. Section 6b's training shows its own read of effort and progress.

**AI staffs work the same way.** Every AI program and club builds its files from the same kinds of looks, with its own people's errors. That's what makes one club's sleeper another club's bust.

**Cost.** Your own staff's files hold the full ranges (about 150 bytes each), for every player your staff has looked at, a few tens of thousands: a few megabytes. AI files store only who looked, what kind of look and when (about 24 bytes) and rebuild the ranges from the player's record when a decision needs them, so the hundreds of AI staffs fit in about 100-200 megabytes. Proposed, not confirmed: those sizes. The build measures them on Cain's 8 GB M1.

---

## Topic 2. Scouting staff at each level

### What exists
- **Built:** each NFL team has one `draft_skill` number (`league.rs`) that sets its whole draft board's error. College boards' error comes from the head coach's recruiting rating (`world/recruiting.rs`). There are no scouts as people.
- **Decided:** a scouting staff of a director, area scouts and cross-checkers (Q31). Staff are rated and affect scouting (Q29). The owner budgets for scouting (Q28). Section 6 (topic 10) put the football operations department (personnel and recruiting staff, the college GM, the NFL front office and scouts) under one table with an evaluation rating, left the details to this section, and made every staff member a person with the coach set of traits. Scouting can be handed to the scouting director with instructions (calendar topic 10). The staff market is section 6's (topic 11).
- **New:** the real personnel jobs at each level and era, their ratings, and the scouting services.

### How real football works
- **An NFL personnel department** (2020s): the GM, an assistant GM, a director or vice president of player personnel, a director of college scouting with an assistant director, two or three national scouts or cross-checkers who see the top prospects across the country, six to nine area scouts who each own a region (Northeast, Southeast, Midwest, Southwest, West and so on), a director of pro personnel with three to six pro scouts, and entry-level scouting assistants who cut film, run the database and do the grunt work. A data and analytics group sits beside them. In the 1990s departments were about half that size.
- **The scouting services.** BLESTO (founded 1963) and National Football Scouting (which started the combine in 1982 and moved it to Indianapolis in 1987) are shared services: member clubs get their spring lists of rising seniors and early grades, and some area scouts work for the service, not the club. Most clubs belong to one.
- **Who rises.** Scouts start as assistants or come from coaching and playing, become area scouts, then national scouts, directors and GMs (Ozzie Newsome, John Schneider and Chris Ballard all came up through scouting).
- **Contracts run through the draft.** Most scouts' deals end in May, so personnel staff move between clubs from May into June, and clubs reshape their departments after the draft.
- **College.** Only the head coach and his on-field assistants (9 until January 2018, then 10) may recruit off campus, so all in-person evaluation of high school players is theirs. Everyone else works from film and on campus: a director of player personnel, recruiting directors, analysts, on-campus recruiting staff, graphic designers for recruiting edits, and from about 2023 a general manager who runs the roster and the money. Rich programs carry dozens; small ones have assistants doing it all.
- **High school.** The head coach is his own scout: he judges his freshmen and move-ins and is the first person college coaches call about a kid. Big programs have an assistant who works as recruiting coordinator.
- **The public.** Recruiting services (Rivals from 1998, Scout.com until 2017, ESPN's rankings from 2006, 247Sports from 2010 with its composite from 2012, On3 from 2021) and draft analysts (Mel Kiper Jr. since 1978, later Mike Mayock, Dane Brugler) employ evaluators of their own. They aren't on any club's staff, but clubs and fans read them.

### The design
**Personnel staff are people,** with section 6's coach set of traits and the ratings this job needs, on the same hidden 1-20 scale:

| Rating | What it does |
|---|---|
| Judging ability | How small his misread of what a player is now |
| Judging potential | How small his misread of where a player is headed (the built `Eye` insight) |
| Position knowledge, by group | Like coaches' teaching by group: a scout who knows linemen reads them better than corners |
| Athletic read | How well he turns tests and movement into attributes |
| Character read | How much people open up to him, and how well he sees through what they say |
| Network | His sources at schools and in his region: trainers, coaches, equipment men who tell him things early |
| Work rate | How many schools and games he gets through, and how thorough each report is |
| Conviction | Whether he stands on his grade in the draft room or folds to the room |

**Each scout has hidden biases,** like section 5's GMs: a body type he loves (height at quarterback, long arms at corner), a type he distrusts (small-school players, players from one conference), testing over tape or tape over testing, overreacting to all-star week, a soft spot for his own region's players, one trait he chases ("speed kills"). A bias leans his reads by a little, never decides them. Proposed, not confirmed: up to about a third of a typical misread.

**The jobs by level and era:**
- **NFL:** the jobs above, with real sizes by era (about a dozen in the 1990s, about two dozen in the 2020s), set below the cap by the owner's scouting budget (Q28). The GM reads the top prospects himself with his built `draft_skill`, which becomes his judging ratings. The head coach and position coaches study prospects' film in spring and run drills at pro days, through their built eye. Your club belongs to BLESTO or National (or neither, as some clubs did), and gets its spring list and shared grades.
- **College:** the head coach and the on-field assistants evaluate in person (their recruiting rating stays as built for the relationship side; evaluation is a new rating each coach has). Personnel staff evaluate film and run the board. Sizes follow the budget below the NCAA's limits (section 6, topic 10).
- **High school:** the head coach evaluates, with an assistant as recruiting coordinator at big programs (topic 17).
- **Public evaluators:** each recruiting service and each draft analyst is a person or a desk with his own reads, biases and track record, which the media (section 6, topic 9) quotes. This replaces the built single shared service read.

**Where scouts come from and where they go:** retired players and failed coaches from the world's own history, and young people who start as scouting assistants. They climb the real ladder (assistant, area scout, national scout, director, assistant GM, GM), and a good one gets hired away (section 6, topic 11) in the post-draft window when his contract ends.

**What you see:** their ratings are hidden, like every staff member's. You learn them from their work (topic 8).

---

## Topic 3. Assignments and the scouting year

### What exists
- **Built:** the NFL dates: all-star games, the combine, pro days by program, the draft (`world/nfl_year.rs`, `calendar/anchors.rs`). College recruiting periods by era (`calendar/`).
- **Decided:** you assign scouts to regions, conferences or single players (Q31). A report takes 1 to 3 weeks, faster with good scouts, and combine numbers come out at once (calendar topic 9). Your scouts' week in season: college games on Saturday, school visits midweek, advance scouting of the next opponent (calendar topic 6; advance scouting is section 9's). Scouting can be handed to the director with instructions like "focus on edge rushers" (calendar topic 10). NFL scouts don't visit high school practices (calendar topic 5).
- **New:** the real scouting year, how much one scout can cover, and how assignments work.

### How real football works
- **May:** the services' spring meetings put out the list of rising seniors with early grades. Clubs add their own names.
- **Summer:** area scouts watch film of their region's list from home and write first reports.
- **August to November:** school calls. An area scout visits two or three schools a week: film in the morning, practice in the afternoon, and talks with the position coaches, the strength coach, the trainer, the academic staff and the equipment manager. He sees a game on Saturday. He visits each big program in his region once or twice a fall. Cross-checkers see the top prospects across regions; the director and GM see the very top themselves.
- **December:** final fall reports and the club's December meetings, where the first board goes up by position.
- **January to April:** all-star games, the combine, pro days, private workouts, top-30 visits, the medical recheck, and the final draft meetings where the board is set.
- **Underclassmen** are on nobody's spring list. Scouts write them up only once they declare in January, from film and the spring events, so clubs know juniors less well than seniors.

### The design
**The year runs on the real calendar above,** on the world's daily clock, for every club and for you.

**Assignments.** You give each scout a job:
- **A region** (the default for area scouts): he covers its schools on the real rhythm, with his work rate setting how many schools and games he gets to.
- **A conference or a group** (all small schools in the Southeast, every edge rusher in the class).
- **A single player** ("see him again", "go to his pro day"), which takes days from his region.
- **An event:** an all-star week, the combine, a pro day.
- **A cross-check:** a second look at the top of another scout's list.
An assignment costs time. A scout pulled onto single players leaves holes in his region, and the gap shows in the files.

**What a scout can cover.** About two or three school calls a week plus a game, so about 35-45 school calls a fall. Proposed, not confirmed: those counts, scaled by work rate and travel (a West region with long flights covers fewer).

**Instructions** go to your director or a scout in plain choices ("focus on players who fit our 3-4", "small schools too", "ask me before taking anyone off the board"), and he follows them by his personality and conviction (calendar topic 10). A director handed the whole department runs the year himself.

**Reports take time.** A look becomes a written report after 1 to 3 weeks, faster with good scouts (decided). Combine and pro day numbers come at once.

**AI clubs and college staffs** run the same year with their own budgets and people. A club that skimps on scouts has thinner files on small-school players, so it misses more of them.

---

## Topic 4. Where information comes from

### What exists
- **Built:** film reads (`scout_player`), all-star practice weeks that pull a grade toward the truth (`world/recruiting.rs`, `all_star_game`), combine invites and tests from the real combine (`body.rs`), the hidden-gem stat footprint that earns a small-school player a second look (`draft_reads`), visibility to recruiters (`world/recruiting.rs`), the services' stars, kicking camps.
- **Decided:** Q31's film, live games, workouts and interviews. High school stats are noisy as schools report them, padding is rare and small, and scouts' film shows the truth (match engine topics; Raichel: not "so distorted that scouts consitently show up to lies"). Composure read from film and splits (match engine). Combine with "insane detail" (Q32).
- **New:** every real source, what each one shows, what it costs and who may use it.

### How real football works
- **Film.** Game film is the base of every evaluation. College coaches' film moves between schools and to the NFL; high school film became universal with Hudl (founded 2006), before which many small-school kids had little tape at all. Highlight reels show only the best plays, so evaluators ask for full games.
- **Live games and practice.** Size and speed in person, effort when the play goes away from him, how he reacts to coaching and to a bad play.
- **The people around him.** The position coach sells his player. The strength coach, trainer and equipment manager tell scouts the truth about work and toughness. Academic staff, campus police and local reporters fill in the rest.
- **Stats services.** Pro Football Focus has graded every college snap since the mid-2010s; tracking data spread through college in the 2020s.
- **All-star games.** The Senior Bowl's weigh-in gives the first official height, weight, hand and arm measurements, and its practice week gives one-on-ones against the best players in the class.
- **The combine.** Measurements; the 40 (with 10 and 20-yard splits), bench press reps at 225 pounds, vertical, broad jump, three-cone and short shuttle; position drills; medicals (topic 6); drug tests; interviews (since 2020 each club gets 45 formal interviews of 18 minutes each, down from 60 of 15, plus any number of informal talks); the NFL's own cognitive test (the Wonderlic was given at the combine until 2022; clubs use their own tests, like the S2).
- **Pro days.** Players re-test or test for the first time at their school, often hand-timed on a fast surface, so pro day 40s run faster than combine ones and clubs discount them.
- **Pre-draft training.** Prospects spend the winter at testing camps that teach the 40 and the shuttle. Numbers rise without the football player changing.
- **Private workouts and visits.** Clubs work prospects out at their schools, and may host up to 30 out-of-town prospects at their facility before the draft for interviews, physicals and written tests, but no drills; players from the club's local area don't count against the 30 and may work out.
- **Background checks.** Club security staff and NFL Security talk to people around each prospect. Social media gets combed: Josh Allen's old tweets surfaced the day before the 2018 draft.
- **The public.** Recruiting rankings, big boards and mock drafts. Clubs read them, and the media reads anonymous scouts' quotes.
- **High school extras:** transcripts, camps (college camps in June, national showcases like the Nike camps and The Opening, the Under Armour and All-American Bowl games), 7v7 tournaments, verified measurements at the few camps that use lasers, and the high school coach's word.

### The design
**Every source is a kind of look** with four things: which attributes it reveals and how much, what else it shows (character, medical, tendencies), what it costs (scout days, travel money, a top-30 slot, an interview slot) and who may use it by the rules of the level and era.

| Look | Reveals most | Also shows |
|---|---|---|
| Game film | Technique and football skills, decisions, tendencies | Less from weak opponents (built level visibility); full games more than highlights |
| Live game | Size, speed, strength against real opponents, effort | Body language, how he takes coaching |
| School call | Practice habits | Character and medical bits from the staff (topics 5, 6) |
| Stats services | Production, splits | Flags for a second look (built footprint) |
| All-star practice | Skills against top competition | Official measurements, interview time |
| Combine and pro day tests | Athletic traits | Exact measurables |
| Position drills | Footwork, hands, change of direction | |
| Interviews | Football IQ, how he learns, communication | Character (topic 5) |
| Cognitive test | Football IQ and processing, roughly | |
| Background check | | Character, trouble, the people around him |
| Medicals | | Topic 6 |

Proposed, not confirmed: how much each look narrows each attribute group. The build sets them so a well-scouted senior's ranges end about two to three points wide on the 1-20 scale, a lightly seen small-school player's five or more.

**Tests measure the test.** A 40 time comes from speed, acceleration, body and the day, plus pre-draft training (a few hundredths of a second, more for a hard worker at a good testing camp), plus the timing: electronic at the combine, hand-timed and faster at pro days. Staffs that read tests well (the athletic read rating) discount the right amounts. Proposed, not confirmed: pre-draft training worth up to 0.05 seconds in the 40, pro day times about 0.05 faster than combine times.

**People don't always tell the truth.** A college position coach talks up his player, a high school coach protects his kid, an agent leaks good numbers, a player coached for interviews comes across polished. A scout with a good character read and network sees through more of it.

**The rules decide who may look:** NFL scouts never at high school practices, college coaches only in the NCAA's periods, top-30 limits, interview limits, no workouts on visits, the eras of film (thin small-school tape before Hudl) and of tests (the Wonderlic at the combine until 2022).

**Public evaluators read players too** (topic 2): services and analysts publish rankings, big boards and mocks from their own reads. Staffs weigh them a little (section 6, topic 9: "boards weigh them a little"), more for players they haven't seen themselves.

---

## Topic 5. Character reports in "insane detail"

### What exists
- **Built:** a character report from the background (`world/background.rs`, `character_report`): home, money, neighbourhood, parent style, a handler, an NFL father, how he handles setbacks, risk words, a mentor, his story lines, each line seen by chance from the scout's accuracy. Nothing reads it yet. Red flags for suspensions and dismissals cost draft points (`career.rs`).
- **Decided:** "insane detail: did the player show up early or late, are his parents loudmouths that yell at every game or are they social media happy? how did he seem to the scouts?" (Q32). Character reports feed each player's hidden personality (Q31). Section 6 gave every person 13 hidden traits (topic 1), relationships (topic 2), life events (topic 6), off-field trouble as events with a process, with draft boards marking it by kind and outcome (topic 7), and media (topic 9).
- **New:** what scouts find, from whom, how sure they are, and how it lands on a board.

### How real football works
- Clubs split **football character** (does he love it, work, take coaching, lead, prepare) from **off-field character** (judgment, maturity, trouble, the people around him).
- The questions are concrete: is he on time, does he sit in the front of meetings, how does he treat the equipment staff, what does the trainer say about his rehab, who are his friends at home, who handles his money, how did he react when he lost his job, did he party through a suspension, what do teammates say when the coaches aren't around.
- A failed drug test at the combine costs real picks (Randy Gregory fell to the 60th pick in 2015). A draft-night video sent Laremy Tunsil from a top-five projection to 13th in 2016. Many clubs keep players off their boards entirely for character, and other clubs take them.
- People shade their words: college coaches want their players drafted, parents defend their kids, rivals plant stories.

### The design
**A character file per player, built like topic 1's ranges.** Every scout who looks writes notes on the traits he can see (section 6's 13: work ethic, coachability, competitiveness, love of the game, ambition, loyalty, greed, temper, ego, leadership, professionalism, adaptability, stability). Each trait shows as a word with how sure the staff is ("a pro's pro, fairly sure"), never a number. The notes come from things that really happened to the player and the people around him, which is the "insane detail":
- **Habits** from professionalism and work ethic: early or late to meetings, his weight in the summer, how he rehabbed, a strength coach's "first one in".
- **His family** from the built parent styles and decider: "his father yells at officials every game", "his mother runs his Instagram", "a cousin handles everything".
- **How he seemed** in interviews and visits, from ego, leadership, football IQ and stability: "talked about himself for 18 minutes", "drew up our blitz on the board from memory", "quiet, a little guarded".
- **His people** from section 6's relationships: teammates' word, his circle at home, a handler or an agent already around him.
- **His record** from section 6's trouble events: charges by kind and outcome, team suspensions, failed tests, what the campus police know. Sealed juvenile records stay unseen.
- **His story** (built story lines) and life events (section 6, topic 6): a death in the family, a child, money trouble at home.

**Sources shade it.** Every note says who it came from. The position coach's word runs warm, the trainer's and equipment manager's run true, a parent defends, a rival school's staff runs cold. A scout with a strong character read and network gets more notes and sees through the shading.

**Some things only surface late.** Old social media posts, a rumor from an ex-girlfriend, a teammate's leak to a reporter: the media side (section 6, topic 9) can drop them in the last week, as real drafts do.

**On the board** (topic 9) the club's character grade sits next to the football grade: clean, minor concern, major concern, or off the board. Each club's line comes from its own people: its GM's and owner's tolerance (section 5's biases), the head coach's style and how desperate the team is (section 6: some clubs take players others won't).

**At every level.** College staffs read recruits the same way, from high school coaches, teachers, counselors, visits and parents (topic 14). A high school coach knows his own players best of anyone (topic 17).

---

## Topic 6. Medical evaluation

### What exists
- **Built:** hidden injury proneness and longevity (`player.rs`), injuries with real types and recovery (match engine topic 6), each player's injury history.
- **Decided:** your medical staff estimates return dates (Q39); medical staff rate diagnosis and return-to-play judgment (section 6, topic 10).
- **New:** medical reads of prospects and recruits.

### How real football works
- At the combine, every club's doctors examine every invited prospect: history, scans, the joints that were hurt. Many clubs call the medicals the most important part of the week. Prospects with open questions come back for the medical recheck in Indianapolis in April.
- Clubs give medical grades and disagree. A player one club fails, another drafts: Willis McGahee went 23rd in 2003 after a severe knee injury in his last college game; Jaylon Smith's nerve damage dropped him to the 34th pick in 2016; Nick Chubb's earlier knee reconstruction didn't stop a long career after the 35th pick in 2018.
- Top-30 visits allow physicals. Trades and signings depend on physicals (built for trades).
- In recruiting, staffs know injuries from high school trainers (37% of public high schools have a full-time athletic trainer) and from the kid's own film; offers get pulled after a senior-year injury.

### The design
**A medical read per file.** Doctors and trainers write what they find: each past injury and how it healed, a hint of hidden injury proneness ("his knees will need managing"), how long he might last, and a medical grade (cleared, minor concern, major concern, failed). The read comes from the true injury history, injury proneness and longevity, through the medical staff's diagnosis rating and what they were allowed to examine (the combine and recheck for invitees, a top-30 or local visit, otherwise only records and film).

**Clubs disagree for real reasons.** Each club's doctors read the same body with their own error, and each GM and owner weighs risk his own way (section 5's risk bias). A player with a big medical question slides on most boards and goes early on one.

**Recruits:** college staffs see injuries mostly through film and the high school's trainer, if there is one, and a staff may pull an offer after a bad injury, by the coach's style and how full the class is.

---

## Topic 7. Potential, hints and comparisons

### What exists
- **Built:** `potential_outlook` (`scouting.rs`): a tier in words (unlikely beyond high school, college, NFL roster, NFL starter, Pro Bowl, elite), how sure the staff is (unsure, fairly sure, sure) and six hints from scout accuracy 13 up. It is a placeholder until this section. Level-relative ratings (`level_rating.rs`).
- **Decided:** "the game never shows an exact ceiling. You get a small, vague idea of potential (for example 'could become a starter') plus small, important hints. Both get sharper and more reliable the better your scouting" (Q46). The hint example: "if a smaller edge in HS has a high QB pressure rate and the GM's scouting skills are high, he gets a small hint 'this player is probably gonna grow in size' or a hint that says the opposite" (Q18). Potential is dynamic: it rises and falls with development (section 1).
- **New:** the full vocabulary, comparisons, and what makes a hint.

### How real football works
- Scouts talk about potential in a handful of real ways: a ceiling and a floor ("high floor, limited ceiling"), "raw", "pro-ready", "a traits guy", "boom or bust", "a year away", "he hasn't scratched the surface".
- They project the body: frame, how much weight he can carry, whether he's done growing (growth plates, the family's size, the age he hit puberty), "a basketball body".
- They compare players: "reminds me of a young..." Comparisons describe a style and body, not a promise.
- Age matters: a 21-year-old with the same tape as a 23-year-old projects higher, and analytics staffs weigh it more than old-school scouts.

### The design
**The potential read is words, never a number.** Each file's potential read has:
- **The role tier,** at the player's level: for a recruit (an FBS starter, a Power Four starter, an FCS player, a small-college player) and for a prospect or pro (as built, from a camp body to one of the best in the league).
- **A floor and ceiling in words,** from where the staff's estimate sits and how wide it is: "high floor, limited ceiling", "boom or bust", "raw but the tools are there", "what you see is what you get".
- **How sure the staff is,** from how much it has seen and who saw it.
- **Hints,** each one a real observation tied to a true cause in the player's model, picked up only by good evaluators and sometimes wrong. They come from the body (still growing, frame to add 20 pounds, done growing, early bloomer: the built maturation), from personality (a worker, soaks up coaching, questions about his love of the game), from tendencies (gambles too much in coverage), from age (young for his class), from stats against tape (Q18's pressure rate at a small size) and from the hidden-gem footprint (a quarterback with a safety's body).
- **Comparisons:** "plays like a young..." from players in the world's own history, past and present, matched by position, body and style (attributes and tendencies), never by potential. Good evaluators make better comparisons. A comp you can look up is part of the fun.

**The read moves,** because potential is dynamic: a breakout spring, an injury or a coach change can move it, and the staff sees the move only when it looks again.

**Hints have a floor of skill.** Proposed, not confirmed: hints start at judging potential 13, as built, with the hit rate rising from about 60% at 13 to about 90% at 20. A wrong hint is a real misread the scout believes.

---

## Topic 8. Learning from your own reads

### What exists
- **Decided:** the draft look-back by scout (game view, settled by real life: front offices review old drafts). You judge staff by their work, never by numbers, and your read sharpens the longer they work for you (section 6, topic 10).
- **New:** the look-back itself and each scout's track record.

### How real football works
- Front offices review their drafts years later: who graded whom, who pounded the table for a bust, who missed a star in his region. Scouts are kept, promoted or let go on it, and GMs are fired over drafts.
- Hindsight is unfair in real ways: injuries, a bad coaching fit, a player who never got a chance.

### The design
**Every grade is kept.** Each report stays in the file forever with its author and date.

**The look-back.** Each summer, and any time you ask, your director (or you) can compare past grades with what players became, three or more years on: by scout, by position, by region, by round. It reads like a real audit: "Daniels had our second-highest grade on 14 players who became starters; he was high on small-school receivers and missed on four of five." It also says when a miss had a reason (an injury, a coach who never played him).

**That's how you learn a scout's hidden ratings and biases.** Never as numbers: as a track record that grows more telling with each year. AI clubs do the same, and a GM keeps, promotes or fires scouts on it, so departments improve or decay for real reasons.

**Reputation.** A scout's public reputation follows his club's drafts and lags his real skill, like coaches' (built), and shapes who wants to hire him (section 6, topic 11).

---

## Topic 9. Your draft board and the AI's

### What exists
- **Built:** each AI team picks from a board made of the consensus December grade (aligned across positions), its own error from the GM's `draft_skill` (quarterbacks 1.3 times noisier), what the position is worth on the market, need weighted by the GM's style (need against best player available), a damp on lesser positions at the very top, kickers only when needed, and owners who overrule round 1 picks for a name (`world/recruiting.rs`, `draft`). Red flags cost 4 or 8 points (`career.rs`). Each pick writes its reason.
- **Decided:** your board comes only from your own reports, and AI boards are built the same way with their own errors (Q31). The power structure decides who has the final say (section 5). Draft boards mark trouble by kind and outcome (section 6, topic 7). Recruiting and draft stock: rankings and anonymous scout quotes shape perception, and boards weigh them a little (section 6, topic 9).
- **Waiting for this section, measured, not tuned here:** trial 7's round 1 positions (trial 8: receivers 20.6% of first-rounders, real 13.4%; tackles 6.5%, real 12.1%) and the 5-star draft rate (88.4% drafted, real 62%; 42.9% in round 1, real 21%).
- **New:** the board as real clubs build it, and the AI moving off the shared consensus onto its own files.

### How real football works
- Clubs build a **horizontal board** (players by position, in grade order) and a **vertical board** (the whole class in the order they'd take them), with players grouped in **tiers**: within a tier they're about equal, so need breaks the tie.
- The board is argued over in meetings: in December after the fall, in February after the combine, and in April for the final board, after the medical recheck. Area scouts present their players, cross-checkers and coaches push back, the analytics staff brings its models (age, production, testing), and the GM or whoever holds the final say settles it.
- Every board has a **do-not-draft list** for character and medicals, and **priority free agents** marked for after the draft.
- Clubs' boards really do differ: a player one club grades in round 1 is a day-3 grade on another, and scheme fit splits them (a 3-4 club and a 4-3 club see the same edge rusher differently).
- Coaches and scouts disagree: coaches want what fits now, scouts the best long-term player.

### The design
**Each club's board comes from its own files.** The built consensus grade goes away. Every club grades each prospect from its own scouts' reports (topics 1 to 7), so a club with better scouts, or the right scout in the right region, sees what others miss. Position value, need, the GM's style, scheme fit (built), character and medical lines and the owner's meddling stay as built, now on top of each club's own grades. The public consensus (media boards, topic 4) still exists, and clubs weigh it a little for players they barely saw.

**Your board:**
- Your director builds a first board from the files, with his reasons. You move players, set tiers, set the do-not-draft list and mark priority free agents. Or you hand it to him (calendar topic 10).
- Each player on it shows your grade in words, the character and medical lines, how sure your staff is and who disagrees.
- **The meetings happen on their dates** (December, after the combine, the final board in April). Your scouts, coordinators, position coaches and analytics staff argue for their players in their own words: the scout who pounds the table (conviction), the coach who wants a fit, the analyst who flags an old prospect. Who wins depends on who holds the final say: in the NFL that is you as GM and coach, but an owner may meddle (section 5).
- **Scheme fit is real on your board:** the built fit scores tell you how a player fits what you run, and your coaches read it through their own eyes.

**What the AI gains:** each AI club's errors now come from its real people and its real looks, not one number. A club whose West region scout is poor drafts worse out West, a club that skimps on scouting misses small-school players, and a club whose GM loves size reaches for it.

---

## Topic 10. The pre-draft season

### What exists
- **Built:** December reads, the advisory committee's grades, declarations and withdrawals by the real dates, the four all-star games with invites and practice weeks, combine invites (about 320) and tests that move a grade up to 1.5 points, pro days as dated events with no effect (`world/recruiting.rs`, `world/nfl_year.rs`). A player can turn an all-star invite down.
- **Decided:** combine, pro days and private workouts with real measurables and insane detail (Q32). The combine week is also agents' and teams' week for free agency whispers (calendar). Media covers mock drafts and anonymous scout quotes (section 6, topic 9).
- **New:** pro days, private workouts and visits that matter, players' and agents' own moves, smokescreens, and the media season.

### How real football works
- **Prospects choose** what to do: skip combine workouts and test at their pro day (common for top players), skip the Senior Bowl, work out only for some teams, train at a testing camp all winter.
- **Pro days** run from early March to mid April. Every club can send scouts and coaches; head coaches and GMs go to the big ones. Players not invited to the combine get their only test there.
- **Private workouts** at the school with a position coach, often for quarterbacks; **top-30 visits** (no drills); **local days** for players from the club's area (testing allowed).
- **Agents steer.** A top prospect's camp can tell a club he won't play there (John Elway in 1983 and Eli Manning in 2004 forced trades from the teams picking first), or skip a club's visit to signal it.
- **Smokescreens.** Clubs plant false interest and say little in public: a visit can be real interest or cover.
- **The media season:** mock drafts every week, big boards, risers and fallers, anonymous scouts' quotes, last-week rumours about medicals or character.

### The design
**Each event is a kind of look** from topic 4, on its real date, open to every club:
- **Pro days** now matter: tests for players not invited, re-tests for those who want to improve (hand-timed, discounted by clubs that read tests well), drills run by the clubs' position coaches. You choose which pro days your people attend; your head coach at a pro day is a look by his eye.
- **Private workouts** at the school: one club, one player, a coach's full look. They cost your coaches' and scouts' days.
- **Top-30 visits:** you have 30 slots for out-of-town prospects (interviews, a physical, written tests, no drills) and unlimited local players (with testing), using the real definition of local by club. Each visit narrows the character and medical reads and builds a first relationship with the player.
- **Interviews** at the combine (45 formal, 18 minutes, from 2020; 60 of 15 before) and the all-star games: each one a character and football IQ look. You pick who gets your slots.

**Prospects and their camps make their own choices** from their personality, agent and projection: skip the workouts if they're sure to go high, test everywhere if they need it, refuse a visit, or (rarely, a top quarterback with a strong family and agent) say they won't play for the club picking first.

**Smokescreens and leaks.** Your interest leaks through visits and the media, so other clubs read it. You can feed a false lead through the press (section 6, topic 9), and AI clubs do too, by their GMs' personalities. Clubs that read the market well see through more of it.

**Mock drafts and the public board** come from the public evaluators (topic 2) and move a little with the news; they're quoted by the media and players read them. A player who falls far below his mock projection carries it (section 6, topic 3 morale; a chip on his shoulder for the competitive ones).

**The combine's effect moves into the files.** The built flat 1.5-point shift goes away: tests and drills narrow each club's ranges, so a club that already knew a player learns little, and one that didn't learns a lot.

---

## Topic 11. Draft day, live

### What exists
- **Built:** the draft in three days in April, in last season's order with traded and compensatory picks (`world/pro/trades.rs`), rounds by era (`world/franchise.rs`), each pick with its reason, one scripted trade up for a quarterback at the top five, the rookie scale (section 5). Pick value charts for trades (classic and surplus, by GM).
- **Decided:** you pick on the clock, trade up or down, and AI teams trade too (Q33). The draft is a huge moment and holiday mode always stops for it (calendar topics 8 and 10).
- **Waiting for this section, measured, not tuned here:** trial 7's trade volume (trial 8: 5-18 trades a year).
- **New:** the live clock, trades on the clock, slides and draft-night news.

### How real football works
- **The clock:** 15 minutes in round 1 and 10 in round 2 until 2008; then 10, 7, 5 for rounds 3 to 6 and 4 in round 7; 8 minutes in round 1 from 2026. A club that lets its time run out can still pick, but the next club may go first (the Vikings in 2003).
- **The days:** two days through 2009, three since 2010 (round 1 on Thursday night, 2 and 3 on Friday, 4 to 7 on Saturday). The draft moved between cities from 2015.
- **Trades on the clock** are phone calls between GMs, priced on their charts, with picks from future drafts. Dozens of trades happen across a draft weekend.
- **Slides:** Aaron Rodgers was a candidate for the first pick in 2005 and went 24th; Brady Quinn fell to 22nd in 2007. A slide hurts and sometimes fuels a career.
- **News lands on the night:** Tunsil's video in 2016, old posts, a medical leak.
- **The supplemental draft** in summer, for players who lost their eligibility, until 2019: clubs bid a pick from the next draft (Josh Gordon cost Cleveland a second-round pick in 2012).

### The design
**The draft plays live** in your war room on the real days. AI picks come at the pace you choose (each pick, to your next pick, or to the end of the round), and the game always stops for your pick and for any call to you.

**On the clock:**
- The real clock for the era. Your board shows who's left, by tier, and your staff speak up in their own words ("take the tackle, the tier ends here", "the edge is the better player").
- The phone rings: AI clubs offer trades on their own charts and needs, and you can call any club. Offers come in the real shapes (a swap with a later-round pick back, a future first), and you see each club's offer, never its chart.
- Run out of time and the next club may pick before you.

**Trades between AI clubs happen live too,** from each GM's chart, his traits (impatience, overconfidence, a pet position), need, and how many players he has left in a tier. They replace the built scripted quarterback swap. A GM trades up when his tier is about to empty and a rival needs the same position, and down when his tier is deep.

**The player's side:** a prospect watches from home or the green room (the league invites the top projected players), and a slide shows on him: the green room camera, his mood after (section 6, topic 3), a chip on his shoulder for a competitive player. His first talk with you starts on the night.

**Draft-night news:** a leak, an old post or a medical rumour can land in the hours before or during the draft from the media side (section 6, topic 9), and clubs react on their boards in real time.

**The supplemental draft** runs in summer through 2019, with its real bidding rules.

---

## Topic 12. The undrafted rush

### What exists
- **Built:** each undrafted player goes to the club with the clearest path to the 53 by its own grade, with some luck (`world/pro/mod.rs`, `undrafted_team`), on a three-year minimum deal with a signing bonus (`money.rs`, `undrafted_terms`). Unsigned players join the free agent pool and other leagues (section 5).
- **Decided:** a rush to sign undrafted free agents right after the draft (Q34). Clubs mark priority free agents on the board (topic 9).
- **New:** the real race, money and the player's choice.

### How real football works
- Calls start during round 7 and the best undrafted players sign within minutes of the last pick. Scouts and position coaches who know the player make the calls; the agent fields offers.
- **Money:** each club has a capped pool for undrafted signing bonuses (about $75,000 per club under the 2011 agreement, rising with the rookie pool: about $92,000 in 2016). There's no limit on guaranteed base salary, so clubs compete with guarantees too.
- **The player picks the path, not just the money:** depth at his position, the coaches who recruited him, how many undrafted players the club has kept (some clubs have long streaks of keeping one).
- Players nobody signs get rookie minicamp tryouts on a trial basis, and some earn a contract that weekend.

### The design
**The rush is live,** in the minutes after the draft. Your scouts and coaches call your priority free agents, and AI clubs call theirs at the same time.

**Each player weighs every offer** like section 5's market, quickly: the bonus and guarantees, his path to the 53 at each club (depth against his own view of himself), the people who called (a scout he knows from his college, the position coach), the club's record of keeping undrafted players, home. His agent's skill and honesty (built) shape the answers. You see answers come in fast and offers taken by others.

**The money follows the real rules by era:** the bonus pool per club as the CBA sets it, guarantees outside it, the minimum salaries by year.

**After the rush:** rookie minicamp tryouts for the rest, a few signed off them, and the free agent pool and other leagues for everyone else (built).

---

## Topic 13. Pro scouting

### What exists
- **Built:** each club's read of a pro from its GM's skill, sharper with his seasons of film (`world/pro/mod.rs`, `grade`), used for depth charts, free agency, trades and waivers.
- **Decided:** pro scouting works the same way as college scouting, for free agents and trades (Q31). Advance scouting of the next opponent is section 9's.
- **New:** the pro side of the personnel department and its year.

### How real football works
- Pro scouts grade every player in the league each offseason, follow a set of clubs through the season, and track the CFL, the UFL and the street free agents.
- Every club gets every NFL game's coaches' film, so pro evaluation is mostly film, plus what scouts hear: practice reports from contacts, agents' pitches.
- **Cutdown day** puts hundreds of players on waivers at once, with 24 hours to claim. Pro scouts' grades decide the claims.
- **Tryouts:** clubs work out street free agents on Tuesdays in the season to keep a list ready for injuries.
- **Trades and free agency** lean on pro grades, and a trade or a signing ends with the physical (built for trades).

### The design
**Pro scouts are the same kind of people** (topic 2) with files built the same way (topic 1). Their looks are film of every game, practice reports through their network, workouts and tryouts, and the medical at a physical.
- **The yearly league grade:** each spring the pro department grades every player in the league, with each scout covering his assigned clubs. Your own players come through your coaches' daily read (topic 1).
- **The season:** each pro scout follows his clubs, the waiver wire and the street list; the director keeps the emergency list by position.
- **Cutdown and waivers:** claims come from the pro department's grades of the players just cut, under the 24-hour clock.
- **Tuesday tryouts:** you can work out street free agents, a look that costs a day of a coach's time.
- **Other leagues:** a scout assigned to the CFL or the UFL brings players back, and the International Player Pathway (built) brings in players from other sports.
- **The market and trades** read your files (section 5's price through your eyes, not true ratings).

---

## Topic 14. Evaluating recruits

### What exists
- **Built:** visibility to recruiters from school exposure, the high school coach's network, camps (family money), tape, state football culture, viral clips, bloodline and all-state honours. One shared services read (accuracy 12) that drifts halfway each look; stars from film, projection and visibility on real quotas; kicking and snapping camps with their own stars. Each college staff ranks recruits per position on 60% film and 40% the services' projection, with error from the head coach's recruiting rating, scheme fit and an academic risk cut; who a staff even sees depends on its footprint, pipelines and its coaches' ties (`world/recruiting.rs`). Recruiting periods, visits and signing days by era (`calendar/`).
- **Decided:** recruiting is the college draft: your own board, evaluations, offers, visits, relationships and signing days on the same rules and calendar the AI staffs use (roadmap section 7). Local kids: drop-ins at high school practice in evaluation weeks and easy unofficial visits (calendar topic 5). FBS staffs scout the top JUCO programs (world topic 6). High school stats are noisy, film shows the truth (match engine).
- **New:** college staffs' evaluations as files (topic 1), the real looks of recruiting, and each service as its own evaluator.

### How real football works
- **Who may look and when.** Only the head coach and his on-field assistants may evaluate off campus, and only in evaluation and contact periods, which have limits by era. In the 2020s FBS staffs get 42 evaluation days in the fall and 168 in the spring window, counted per coach per day (NCAA Bylaw 13.17).
- **Film first.** Hudl put every high school's film online from the late 2000s. Before that, small-school kids depended on a coach mailing tapes.
- **Camps.** College one-day camps in June let staffs measure and coach a kid themselves: real height and weight, a laser-timed 40, how he takes coaching. Satellite camps (Michigan's 2015 tour) took camps into other states; the NCAA banned them in April 2016 and lifted the ban weeks later. National showcases (Nike's camps and The Opening, the Elite 11 for quarterbacks) and all-star games (the U.S. Army All-American Bowl from 2000, the Under Armour game from 2008, the Polynesian Bowl from 2017) put the best against each other.
- **Junior days and visits.** Staffs read the kid and his family on campus: how he acts, who makes the decisions, what they ask.
- **The people around him.** High school coaches, teachers, counselors, the trainer. Programs pull offers over social media posts and trouble.
- **Grades.** Transcripts go through the NCAA Eligibility Center (core courses and the sliding scale; test scores dropped from the requirement in 2023). Academic staff judge whether a kid will qualify and stay eligible.
- **Offers spread.** Staffs watch who else has offered: a big offer brings more, and an offer list is part of a kid's stock. Many offers aren't committable; the kid finds out when he tries to commit.
- **Services.** 247Sports, Rivals, On3 and ESPN each rate kids with their own analysts, and the composite averages them. Staffs read them, fans live by them, and the stars shape perception and NIL, but staffs offer on their own evaluations.

### The design
**College staffs keep files on recruits like any staff** (topic 1). Their looks:
- **Film** of games (from Hudl's era, full games for almost every kid; before it, thin tape for small schools, so their visibility stays as built).
- **In-person evaluations** in the NCAA's periods, by the on-field coaches only, against the era's evaluation-day limits. Each day a coach spends watching a kid's practice or game is a look by his evaluation rating and eye.
- **Your camps:** verified measurables and a day of coaching him (his coachability and competitiveness, partly). Satellite camps in their real years.
- **Showcases and all-star games:** the best against each other, open to every staff and every service.
- **Junior days, official and unofficial visits:** character and family reads, and the relationship (built).
- **Calls to his coach, teachers and counselor:** character, academics, and the high school coach's own credibility (topic 17).
- **Transcripts:** the academic read, by the era's rules, read by your academic staff (section 6, topic 10).
- **Social media and trouble:** character notes, and a reason to pull an offer.

**Each recruiting service is its own evaluator** (topic 2): its analysts' looks are film and the showcases, with their own biases (bigger programs' camps, players who go viral). The composite exists from 2012. The built shared read and the star quotas stay as how each service turns its read into stars.

**Staffs read the offers market.** A kid's offer list feeds a staff's interest a little (herding is real), so a kid with a big offer gets looked at more and graded a bit higher by staffs that have barely seen him. Proposed, not confirmed: it counts for up to a tenth of a staff's grade, less the more the staff has seen him itself.

**The AI's boards move onto files.** The built board (film plus projection with the head coach's error) becomes each staff's own grades from its own looks, and the coaches' new evaluation rating replaces the head coach's recruiting rating as the source of error. Recruiting and relationships stay as built.

---

## Topic 15. Your recruiting board and moves

### What exists
- **Built, for the AI:** class plans per position, boards per position, three targets per plan spot plus three, the position coach calling and visiting every target, the head coach's own time for the very top, five official visits per recruit, unofficial visits within 250 miles, relationships from time, visits and staff ties, offers down the board, commitments and flips, early and late signing, walk-ons and preferred walk-ons, scholarships and roster limits by era, NIL and revenue sharing (`world/recruiting.rs`, `world/college.rs`).
- **Decided:** your side of all of it, on the same rules (roadmap). Recruiting actions are real actions (calendar topic 5). Talks are picked moves with tones, with parents too (section 6, topic 5). Your NIL and revenue-sharing budget is section 5's college seat.
- **New:** how your staff divides the work, and what you decide.

### How real football works
- **Every assistant recruits an area and coaches a position.** The area recruiter finds and builds the relationship ("he recruits Atlanta"), the position coach evaluates and sells the role, and the head coach closes on visits and in the living room. A recruiting coordinator runs the board and the calendar.
- **Boards are by position,** with priorities and backups, and offers go out in waves: a first wave in the spring, more after camps.
- **The commitment is only the start:** staffs keep "babysitting" commits, and rivals keep trying to flip them up to signing day.
- **The rules by era** set what contact is allowed, when visits can be taken and how many, and when kids can sign.

### The design
**You run your program's recruiting like the AI does, with your hands on it.**
- **Areas and positions:** you give each on-field assistant his recruiting area (the default from his ties and history, as real staffs do) on top of his position. His area is where he finds kids and builds trust, his position where he evaluates. Who recruits a kid matters for the relationship (built: the position coach's recruiting, ties to the high school coach).
- **The board:** by position, with your class plan (built), your staff's grades and how sure they are, the services' stars beside them, the offer list and who's leading.
- **Moves, each costing staff time inside the rules:** evaluate (send a coach in an evaluation period), call or message (by the era's rules), offer (committable or not), invite to camp, junior day or an official visit, a home visit by the head coach, and pull an offer. Talks with the kid and his parents are section 6's picked moves.
- **Commits:** you keep each one warm or lose him to a flip (built), and decide what to do when a better player wants his spot.
- **The money:** your NIL and revenue-sharing budget (section 5's college seat) and the athletic department's recruiting budget for travel and visits (section 6, topic 12).
- **Hand it off:** your recruiting coordinator or college GM can run any part of it with instructions (calendar topic 10).

---

## Topic 16. Portal, JUCO, prep and international players

### What exists
- **Built, for the AI:** the transfer portal with its windows, leans, whispers and tampering, and who wants an entrant (his two-deep fit); JUCO movers re-evaluated as new players; prep school years; kicking camps; the International Player Pathway for the NFL from 2017 (`world/recruiting.rs`, `world/pro/leagues.rs`). Whispers reach better-connected scouts earlier (calendar).
- **Decided:** portal leans and whispers (calendar topic 3).
- **New:** how staffs evaluate each kind of player.

### How real football works
- **Portal players** come with college film and stats against college opponents, so they're evaluated like pros: faster, surer, with medical and character records from a college program, and with tampering before they enter.
- **JUCO players** are scouted at the top JUCO programs, for one or two years of college ball and grades to check.
- **Prep schools** give a kid a year to grow or qualify.
- **International players:** Australian punters trained by ProKick Australia (from 2007) have filled college punting jobs; Europe's NFL academies and camps produce a few prospects.

### The design
**Same files, different looks.**
- **Portal:** college film and the stat book, the old program's staff (who may run cold on a player who left), the player's own whispers before he enters (built), and the tampering contacts (built, never punished).
- **JUCO and prep:** film and visits at the programs, the transcript check for the move up.
- **International:** players found through their own pipelines (ProKick for punters, the NFL's academies and the International Pathway), with thin tape and a single look.
- **Walk-on tryouts** (built): a look in person, open to any student, by the program's rules.

---

## Topic 17. The high school coach's side

### What exists
- **Built:** one head coach per high school with ability and network (`world/high_school.rs`, `world/coach.rs`); his network is part of each kid's visibility to recruiters (`world/recruiting.rs`); local college staffs drop in on practices in evaluation weeks (calendar topic 5); transfers between schools by each state's rule.
- **Decided:** a high school staff that you hire, many of them teachers (section 6, topic 10). The high school seat (roadmap, "Your career at every level").
- **New:** judging your own kids and getting them recruited.

### How real football works
- **The coach knows his kids best.** He sees them every day, and middle school and youth coaches tell him who's coming. He decides who plays varsity, who moves position, who needs a year on JV.
- **He's the kid's first recruiter.** He sends film to college staffs, takes their calls, hosts them in evaluation periods, pushes kids to camps, and keeps them eligible (core courses, grades).
- **His word is his currency.** College staffs trust coaches whose kids turned out as described and stop listening to those who oversell. Big programs have an assistant who works as recruiting coordinator.
- **Others are in the kid's ear too:** 7v7 coaches, private trainers, handlers and the services.

### The design
**You read your own players through your eyes and your staff's** (topic 1): daily practice, the weight room, and a look ahead at the freshmen coming up from your feeder middle schools and youth programs. Move-ins and transfers come with their old school's word.

**Getting your players recruited is part of the job:**
- **Promote:** send film to programs and call staffs you know. Each call uses your network (built) and your credibility with that staff.
- **Your credibility** is a relationship with each college staff (section 6, topic 2): it grows when your kids turn out as you said and falls when you oversell. You choose what to say about each kid, and a strong word for a borderline kid is a real trade-off.
- **Host:** college coaches visit in evaluation periods (built drop-ins), and you choose whom to show.
- **Camps and showcases:** you push kids toward the right ones, which costs the family money (built: camps from family money).
- **Eligibility:** you and the counselor keep their core courses and grades on track (section 6's academic support).
- **The others in his ear** (7v7 coaches, trainers, handlers) are people with their own aims and can pull a kid away from your advice.

**The payoff is real:** a coach who places kids well builds the buzz that moves him up (built: the climb to college on buzz) and the pipelines that bring college staffs back (built).

---

## Topic 18. The "my players" list and draft coverage

### What exists
- **Built:** players followed from 14 to retirement with their whole history; a combine riser's news line ("lights up the combine"); news and events by loudness (`world/news.rs`, `calendar/events.rs`).
- **Waiting since the game view (brainstorm item 1, parked for section 7):** a "my players" list kept by the game, with news when one of them does something. Cain: "it's about what it gonna make that scouting screen relate to fans". Its fan angles: being on a guy before anyone else, your board against the public big boards and mock drafts, player comparisons, the combine freak moment, the hometown story, the online debate about a prospect.
- **Decided:** every line comes from the huge line bank (section 6, topic 5). Media and fan voices by era (section 6, topic 9).
- **New:** the list and the draft's coverage.

### How real football works
- Scouts and fans both keep their guys: the area scout who found a star talks about it for years, and fans follow a recruit from his first offer to the pros.
- Draft coverage is a genre: TV picks with analysts' takes, grades for each club's class the next morning, winners and losers, the best players still available, mock drafts scored after the fact.
- Combine moments become legend: DK Metcalf's 4.33 at 228 pounds in 2019.

### The design
**The list fills itself and you add to it.** Everyone you scouted, recruited, coached, drafted or graded highly is on it, and you can add anyone. Each one carries the record of your history with him: when your staff first looked, what they graded him then, and what the public said at the time.

**Being on a guy first is recorded.** When a player you had high early does something big, the news and your staff say so with the facts: "your area scout had him as a starter-level talent in 2029, when the services had him at two stars". When you missed one, that's recorded too (topic 8).

**Your board against the world:** your grades next to the public boards and mock drafts all draft season, so you see where you're out on a limb.

**The moments:** a test far beyond what his body should do becomes a combine moment, with the clip in the news, fan reactions and the online debate (section 6's fan voices). A hometown story comes from the built story lines.

**Draft coverage:** each pick announced with analysts' takes (the public evaluators of topic 2, in their own voices, from the line bank), next-morning grades for every club's class, winners and losers, risers and fallers, and the mock drafts scored. Years later, the look-back of topic 8 has a public side too: re-drafts in the media.

---

## What the build must prove

- **Memory and time on Cain's 8 GB M1:** the files of every staff in the world (topic 1's sizes), and the AI's daily evaluation work, timed per season. Proposed, not confirmed: the whole of section 7 adds under 300 MB and under a fifth to a season's simulation time.
- **Clubs really differ:** the same prospect's grade spread across the 32 boards, and the number of first-rounders from outside the public top 50 (real: one to three a year, `career.rs`).
- **Scouts matter:** a club with better scouts drafts better over 20 years, by about what the real spread between the best and worst drafting clubs is, and no more (influences are noteworthy, never driving).
- **The measured items from trial 7,** reported in the next trial, not tuned before it: round 1 positions, the 5-star draft rate, draft and trade volume.
- **Text never repeats:** a few hundred generated scout reports, character notes and draft-night lines read for repeats, timed (section 6, topic 5).

## Open choices (waiting on Cain)

None. Real life settles every topic here; the numbers no source publishes are marked proposed, not confirmed, for the next trial to check.
