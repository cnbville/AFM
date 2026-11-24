# Your coach, earned: the coaching career (version 5, 2026-11-24)

Cain, 2026-11-24: "i want you overhaul/propasal a ratings overhaul, right now ratings i as a coach have feel useless"; "i want more of earn to receive type of system, similar to EA College footballs"; on version 1: "read through the code, give me a better overhaul"; on version 2: "not gamified enough"; on version 3: "i meant not gamified as a system it doesn't feel like grind/reward"; on the first pace: "too slow for a game of this realism and needing of time"; then: "use logic, certain stuff like clock manager is only used when simulating, the ratings need to make sense for both manual play and simmign games"; on version 4: "i want it to lean into archetypes more"; "i want it similar but rebranded to mine".

Version 5 keeps version 4's loop (**use it to level it**) and its reading of the code (main `d5a75d5`), and puts the loop on an **archetype ladder** in the shape of EA College Football's, under AFM's own names.

What is decided, and by whom:

- **By Cain** (2026-11-24): the pace (one ability maxes in one season of doing it), and that every rating must make sense both when you play a game yourself and when you sim it.
- **By proxy** (the night of 2026-11-24/25): everything else. Cain went to sleep with the questions open and wrote: "i'm giving you permission to make decisions for me als long as you note them down. At no point will you wait for my answer or my input until i tell you otherwise". Each pick is listed at the end under "Decided by proxy 11-24/25" with its reason, so he can change any of them.
- **Sizes** stay proposed, not confirmed, until `coach_check` measures them (section 12).

Mock-up of the Career screen: https://claude.ai/artifact/FyZoAghjYaBqiQR49RRgRN (click an archetype, then an ability).

Decided already, so not asked again: Q20 "your ratings grow" (`docs/handoff/decisions.md`); `docs/design/people.md`, topic 15 (ratings grow from doing the job, the people around you, study, age and use); roads C3-C7 (locked strengths, the rest at the road's last job level); flaws and tools for players (`docs/design/individual-practice.md`); the game-day rule that your own ratings call the snaps you don't take (`docs/design/game-day.md`).

## What it does

- **You are an archetype, and you climb a ladder of eleven.** Three base archetypes (Play Caller, Closer, Players' Coach), three elite ones (Mastermind, Trail Boss, Standard Setter), three hybrids (System Builder, Sideline General, Pro Factory) and two at the top (Builder, Institution).
- **An archetype holds abilities in four tiers.** 53 abilities: Red Zone, Third Down, Living Room, Flaw Fixing, One-on-Ones, Film, and so on.
- **A tier comes only from doing that exact work.** No points, nothing to buy. Every rep fills that ability's bar, and every tier pays the moment you reach it.
- **An advanced archetype unlocks by a feat plus tiers you earned.** Mastermind needs 16 of Play Caller's 24 tiers and a unit you call in the top tenth of your level. Unlocking it opens tier 4 of its abilities, its signature ability and its name on your page and across the world.
- **Work is never wasted.** A locked archetype's abilities work up to tier 3 from day one, and reps past that are banked for the day it unlocks.
- **Every ability pays three ways, so it works however you play.** Your team is better prepared in that situation, always. Your screen shows you more there when you play it yourself. Your coach decides better there when you sim.
- **The pace is weeks, not seasons.** About one tier a week. An ability's first tier comes inside two weeks and its fourth after one season of doing it. The ladder is the long chase.
- **Your ratings are the sum of it.** Play Calling goes 15 to 17 because you called a season of red zones and third downs.
- **Every coach in the world is on the same ladder.** You hire a coordinator because he is a Mastermind. A recruit's father has heard you are a Pro Factory.

Example. Season 2, week 9, head coach at Toledo, calling your own offense. You are a Play Caller and a Closer. This week Red Zone reached tier 4, which opened its last perk, and Living Room reached tier 3. Mastermind is locked: you have 15 of the 16 Play Caller tiers it needs, and your offense is 11th of 134 in the country, on pace for the top-13 finish the feat asks. Its abilities already work: Game Planning sits at tier 3 with 40 reps banked past it, so tier 4 lands the day Mastermind unlocks. Your Play Calling reads 17, 15 from your road and 2 earned. End of Half is one of your road's two cold abilities and only reached tier 1 last month.

## What the code does today

### Your ratings are stored twice, and your talks read the frozen copy (a bug)

Your ratings live in two places: the coach record (`world.coaches[c].ability`, `yours`) and a copy taken on setup day (`career.you`, `world/seat.rs`, `You`). The yearly growth and decline write only the coach record (`world/coach.rs`, `age_one_year`). Only a placed lesson point writes both (`storylines/credit.rs`, `place`).

These read the setup-day copy, so they never see your growth or your decline:

- Your read of people in every one-on-one: `folk/talks.rs`, `sharpness` (10 callers) and the expected-reaction read at line 2551.
- Whether you catch a recruit's words parting from his reaction: `folk/pitch.rs`, `catches`.
- The hand-off screen's "your pitch" and "your catch rate": `folk/pitch.rs`, `hand_pitch`; `folk/handoff.rs`.
- The first calls to commits after you take a job, and a signee's choice to stay: `folk/takeover.rs`, `recruiting_calls` and line 3118.
- The job preview's recruiting line: `front_office/preview.rs`.

Your profile shows the coach record. So the number you see grows, and the number your talks use doesn't. Root cause: two stores with no single owner. The fix is one store (the coach record) and `career.you` kept only as the setup record.

### What each rating reads, seat by seat

"Dead" means no code reads it for you in that seat.

| Rating | Head coach with coordinators | When you call a side | High school head coach | NFL |
|---|---|---|---|---|
| Play calling | **Dead** (`season.rs`, `play_caller` picks the coordinator) | The caller's skill, reading, situational and creativity (`coach_caller`, `your_calls`); 18 against 4 is +4.4 points a game | As college | As college |
| Adaptation | **Dead** | In-game shifts (`calling.rs`) | As college | As college |
| Scheme | **Dead on game day**: film study and the spring install read the coordinators (`season.rs` line 1810; `systems.rs` line 786). Read once at setup for which systems you know (`staff.rs`, `systems::draw`) | Almost not read: your play calling replaces the scheme-and-game-management base | 0.7 of film study (`hs_year.rs`) | As college |
| Game management | Late timeouts, delay of game, crowd noise; fourth downs only where your own chart doesn't decide (`flow.rs`) | Same | Same | Same |
| Teaching | 0.2 of each player's coaching number (`season.rs` line 1116); 10 points is about 5% faster growth | Same | **0.6** of it (`season.rs` line 1061) | Through the staff rating (`staff.rs`, `set_nfl_coaching`) |
| Motivation | Trust per read +0.003 a point (`people.rs`); how much of a talk reaches a player (`game/talk.rs`: 0.78 + 0.022 a point) | Same | Same | Same |
| Discipline | How much rule-breaking your staff sees (0.45 + 0.025 a point, `folk/rules.rs`); how often players obey an instruction and may freelance (`game/roster.rs`, `scrimmage.rs`) | Same | Same | Same |
| Relationships | Read in about 50 places: bonds, promises, the portal sit-downs, your read of people, 40% of your recruiting pull | Same | Same | Same |
| Recruiting | Your pitch: each point lands 0.04 on a base of 0.6, so 15 against 10 is a third more (`folk/case.rs`, `skill_of`); 60% of the head coach's pull (`recruiting.rs` line 6615) | Same | Not a recruiter's seat | Nearly dead: only your work rate as a reader of files (`scouting/readers.rs`) |
| Evaluation | Film, files, walk-ons, the staff's eye for flaws, the draft board (`Coach::evaluation`, 15 callers) | Same | Same | Same |
| Network | Credibility with colleges, sources, contacts | Same | Exposure for your players | Same |
| Negotiating | **Dead**: all three readers are in the NFL market (`pro/market.rs` line 1504; `front_office/nfl.rs` lines 1124, 1788). NIL, the portal, staff contracts and your own contract don't read it | **Dead** | **Dead** | Whether you read an agent's bluff right (0.45 + 0.03 a point); the width of your read of a price (22% of it at 10, 1 point less a rating point) |

### How you earn today

- **Age.** Each year every rating rises by learn × min(room to 20 / 6, 1): learn 0.45 under 35, 0.2 to 47, 0.05 to 57. A head coach's shared ratings take two thirds of that (`Coach::pace`). At 40 that is about 0.13 a year in each of 19 ratings, special teams teaching included, whatever you did. The page shows one decimal.
- **Lesson points** (`storylines/credit.rs`; `legacy/mod.rs`, `pay`). A story handled well or an ambition's step banks 0.25 to 1.00. A full point is a letter: you place +1.0 in a rating. It costs the same at 19 as at 10, has no season cap, and can be drawn from any bank into any rating. Of the story catalogue's 173 rating tests (`storylines/catalogue.rs`), 81 are relationships or motivation, 4 scheme, 1 adaptation and none play calling. Teaching is paid only by a player of yours who grew the most (`legacy/guys.rs`). How many points a season brings is not measured.
- **Your eye** gains up to 3 points from draft look-backs (`scouting/own.rs`, `credit_eye`). This one is real earn-to-receive already.
- **Where you start.** Roads lock two or three strengths at 15 to 19 and set the rest at 11 to 14 (`afm-game/src/setup.rs`, `BACKGROUNDS`).

### What your ratings do that you never see

- Each year your assistants move 5% × (your mean teaching / 20) of the gap toward your scheme, game management, motivation, network, recruiting and their group's teaching (`legacy/tree.rs`). You develop your staff, and no screen says so.
- Your reputation moves 30% a year toward 0.3 teaching + 0.2 scheme + 0.15 game management + 0.15 motivation + 0.1 discipline + 0.1 relationships + 12 × results against expectation (`coach.rs`, `season_reputation`). The trade reads part of your hidden ratings directly.

## Why they feel useless (root causes)

1. **A bug freezes them** where you use them most: your own talks and pitches.
2. **Growth is an age term**, too small to see, the same for every rating, blind to what you did.
3. **The one earned system is flat**: +1 anywhere, no rising cost, and it mostly pays two ratings.
4. **A head coach's game-day ratings are switched off** unless he calls plays himself.
5. **Negotiating does nothing below the NFL.**
6. **A rating is one flat number.** Nothing in it is yours: no specialty, no weakness, nothing to build toward.
7. **Nothing reports back.** No screen shows a rating earning anything.

## How real coaches' skills show up

- **Reps make the coach.** Sean McVay called plays as Washington's coordinator for three seasons before the Rams. Kirby Smart ran Nick Saban's defense for eight.
- **Coaches are known for specialties, not one number.** Andy Reid's screens and plays off a bye week. Bill Belichick taking away an offense's best player. Kyle Shanahan's run game off the same looks. Ed Orgeron in a living room. Offensive line coaches known for one technique.
- **And for blind spots.** Reid's clock management. Coaches who can't close a recruit, or who lose the locker room's veterans.
- **A head coach who doesn't call plays still runs the week.** He sets the plan on the side he came up on, signs off on the call sheet and leads the halftime adjustments.
- **Head coaches hire specialties.** A staff is built from a red zone man, a third-down man, a recruiter for one state.

## How EA's College Football does it

- You earn XP from games, goals and recruiting. Levels pay coach points, and points buy abilities.
- You pick one of three base archetypes on day one: Recruiter, Motivator or Tactician. The other two are bought later.
- There are eleven archetypes. Three elite ones sit above the bases (Elite Recruiter, Master Motivator, Scheme Guru), three hybrids join two bases (Architect, Strategist, Talent Developer), and two sit at the top (Program Builder, CEO).
- An advanced archetype unlocks by a feat plus points spent in its parent: two top-five classes and 50 points in Recruiter for Elite Recruiter, five playoff wins for Program Builder, two national titles for CEO.
- Each archetype holds its own abilities, more than 50 in all, in four tiers. Nobody maxes everything.

What AFM takes: the ladder's shape (three base, three elite, three hybrid, two top), unlocks by a feat plus work in the parent, abilities in four tiers, and a career that can't hold everything. What it changes: there is no XP and no points. A tier comes only from doing that ability's work, an ability pays in its own situation through the battles the engine already plays, and every name is AFM's own.

Sources: [EA, College Football 26 dynasty deep dive](https://www.ea.com/games/ea-sports-college-football/college-football-26/news/cfb26-campus-huddle-dynasty-deep-dive); [Operation Sports, best coach archetypes in College Football 26](https://www.operationsports.com/best-coach-archetypes-in-college-football-26); [TheGameHaus, all College Football 26 coach tree perks](https://thegamehaus.com/college-football/all-college-football-26-coach-tree-perks/2025/06/26/); [Sportskeeda, College Football 26 archetypes guide](https://www.sportskeeda.com/college-football/coach-archetypes-ea-sports-college-football-26-ultimate-guide); [EA, College Football 25 coaching abilities tips](https://www.ea.com/games/ea-sports-college-football/college-football-25/tips-and-tricks/college-football-25-dynasty-mode-coaching-abilities-tips); [Destructoid, College Football 26 coach abilities](https://www.destructoid.com/best-college-football-26-coach-upgrade-abilities-in-dynasty-mode/).

## Playing a game yourself and simming it (the code today)

Cain's point, checked in the code. A live game stops for you and records what the staff would do (`game/live.rs`, `LiveGame`; `docs/match-engine.md`, "Live games, officials and the toss"). "A call you pick replaces the staff's." So:

- When you call a snap yourself, your play calling, adaptation and game management read nothing on that snap. Your pick is the call.
- When a snap doesn't stop for you, when you watch, or when you let the staff coach it, the side you call is called by your own ratings (`season.rs`, `coach_caller`, `your_calls`) and the other side by its coordinator.
- Your fourth-down and two-point charts decide the snaps that don't stop for you.

So a hands-on player's game-day ratings do nothing, and a player who sims never sees them work. A skill that pays in only one way of playing is wrong. Section 2 below is the fix.

## The loop

Every number in this part is a starting value for the measure (section 12), except the pace.

### 1. Abilities level by doing the work

- 53 abilities in eleven archetypes (sections 5 and 6). Each has four tiers and a bar.
- A rep is one piece of that exact work: a red zone snap, an in-home visit, a one-on-one, a game of film.
- A rep done well fills more. Your own call is graded against the numbers the game already has (the call table, the fourth-down model, his file, his market): a good one counts up to double, a poor one half. Reps against a better opponent count more.
- There are no points, no trees to buy and no refunds. The only way to Red Zone tier 4 is red zone work.

### 2. Every ability pays three ways

| | When | What your tier does |
|---|---|---|
| Your team (or the person in front of you) | Always | How well your players are prepared for that situation, or how well your work lands on that person. The same whether you play, watch or sim |
| Your screen | When you do it yourself | What your coach knows and shows you there: tendencies, odds, his file, more choices, less time spent |
| Your coach | When you sim or hand the snap off | How well he decides there without you |

Game-day examples:

| Ability | Your team (always) | Your screen (you play) | Your coach (you sim) |
|---|---|---|---|
| Red Zone | Fewer busted assignments inside the 20; more red zone plays known cold | Their red zone tendencies; a sharper suggested call | Calls the red zone better |
| End of Half | Faster to the line; fewer procedure flags in hurry-up | How many plays the clock still holds; when to spend a timeout | Manages the clock better |
| Short Yardage and Fourth Down | Your short-yardage package runs cleaner | The odds of each choice, tighter the higher your tier | Decides better |
| Adjustments | More of an adjustment reaches your players | A clearer report of what hurt you; a second adjustment to pick | Picks the right adjustment |

Off the field the first two columns do the work. A pitch lands by your Living Room tier whether you type the lines or pick from your staff's options, and the screen shows you more of his file. When you hand a job to an assistant, his own tiers do it and he earns the reps.

An ability that can't fill all its columns is cut or merged. Version 3's Clock Manager and Two-Minute Drill are one ability (End of Half). Replay challenges are no ability: the replay assistant does that work.

### 3. What a tier pays

| Tier | Reached at | Power in its own situation | Screen |
|---|---|---|---|
| 1 | A tenth of a season's reps | 1.5 points of its rating | First perk |
| 2 | Four tenths | 3 points | Second perk |
| 3 | Seven tenths | 4 points | |
| 4 | A full season | 5 points, the top size of a player's tool (`craft.rs`, `FULL`) | Third perk |

- **Power** reads on your team's preparation always and on your coach's choices when you sim.
- **Screen perks** are something new to see or do when you do that work yourself. Section 6 has one example per ability. The full list of 159 is lane RE2's first job; it is written into this doc before its screens are built.
- **Your rating.** Each of the 19 ratings is your road's start plus what its abilities earned. All of a rating's abilities at tier 4 lift it by 6, to 20 at most. A season doing about half of a rating's work lifts it 2 to 3.
- **Tier 4 of an advanced archetype's ability needs that archetype** (section 5). Tiers 1 to 3 are open to everyone from day one.

### 4. The pace (decided: one season)

Cain chose on 2026-11-24: one skill maxes in one season of doing it every week. With tiers, that is tier 4.

- Each ability has a season's worth of reps: about 150 red zone snaps for a play caller, about 25 in-home visits, about 120 one-on-ones, 13 halves.
- Tier 1 comes inside two weeks, tier 2 by midseason, tier 3 in November, tier 4 at a full season.
- Good work is up to twice as fast, so a sharp caller reaches Red Zone tier 4 by November. A sim levels at the normal pace.
- About a dozen abilities fit in your week, so in your first season you earn about one tier a week, and more in the first month.
- The ladder is slower on purpose (section 5): the tiers pay you every week, the archetypes are what you chase across seasons.

### 5. The eleven archetypes

| Step | Archetype | Who he is | Its abilities |
|---|---|---|---|
| Base | Play Caller | Calls a side of the ball and installs it | 6 |
| Base | Closer | Recruits and evaluates | 6 |
| Base | Players' Coach | Teaches and leads the room | 6 |
| Elite | Mastermind | The game plan beats you before kickoff | 4 and a signature |
| Elite | Trail Boss | Owns the recruiting trail | 4 and a signature |
| Elite | Standard Setter | The room runs itself by his rules | 3 and a signature |
| Hybrid | System Builder | Signs players for his system and makes them fit | 3 and a signature |
| Hybrid | Sideline General | His team plays above itself on Saturday | 3 and a signature |
| Hybrid | Pro Factory | Finds them raw and sends them to the league | 4 and a signature |
| Top | Builder | Builds the whole program: staff, press, money | 4 and a signature |
| Top | Institution | The program is his | 2 and a signature |

**The base archetypes.** All three are open to every coach from day one: work is work. You pick your first archetype when you start your career, from the ones your road allows (section 8). That pick sets your starting tiers and what the world calls you. You hold a second or third base archetype once you have earned 8 of its 24 tiers.

**Unlocking an advanced archetype** takes tiers you earned in its parents and one feat. The feat is read at your level, so a high school coach, a college coach and an NFL coach climb the same ladder.

| Archetype | Tiers | Feat in college | High school | NFL |
|---|---|---|---|---|
| Mastermind | 16 of Play Caller's 24 | A unit you call or run finishes in the top tenth of your division by the Data section's rating, or twice in the top quarter | The same, in your state's class | The same: top 3 of 32, or twice in the top 8 |
| Trail Boss | 16 of Closer's 24 | Two classes ranked first or second in your conference | Two years leading your district in players signed by colleges | Two draft or free agent classes the look-backs grade in the league's top quarter |
| Standard Setter | 16 of Players' Coach's 24 | Five players, past or present, at the top stage of the bond with you ("like a big brother", "like a father", "like a grandfather") | The same | The same |
| System Builder | 12 in Play Caller and 12 in Closer | Eight starters you signed, on a unit in the top quarter of your division | Eight starters you have coached since their first year, on a top-quarter unit | Eight starters you drafted or signed, on a top-quarter unit |
| Sideline General | 12 in Play Caller and 12 in Players' Coach | Five wins in two seasons in games where the Data section gave you under 35% | The same | The same |
| Pro Factory | 12 in Closer and 12 in Players' Coach | Three of your players drafted who signed with you as three-stars or lower | Three of your players signed by FBS programs who had no offer as sophomores | Three starters you picked in round 4 or later, or signed undrafted |
| Builder | Any two advanced archetypes | Five postseason wins as a head coach | Five playoff wins | Five playoff wins |
| Institution | Builder | Two titles as a head coach: conference or national | Two state titles | Two conference titles, or one Super Bowl |

**What an unlock pays, at once:**

- Tier 4 of its abilities opens. Reps you banked past tier 3 count, so some reach tier 4 that day.
- Its signature ability opens (below).
- Its name goes on your page, in the news and into how the world reads you (section 10).
- An archetype is never lost. Its abilities can still rust (section 7).

**The eight signatures.** One per advanced archetype. A signature is an ability like any other: four tiers, reps from its own work, the same pace, three ways to pay. It starts at zero on the day of the unlock, because it needs the standing the feat gave you.

| Archetype | Signature | A rep | What it does |
|---|---|---|---|
| Mastermind | Take Him Away | A game plan that names one of their players | Your players' assignments against that man are cleaner. On your screen: where he lines up and what they call for him. In a sim your coach picks the man and plans for him |
| Trail Boss | Home Turf | A contact with a recruit from an area where you stand at Trusted or Home (`docs/design/ground.md`) | Recruits from those areas start warmer with you. On your screen: who else is working the area. Your assistants borrow part of your standing there |
| Standard Setter | The Standard | A week in which your captains handled a problem before the staff did | Newcomers pick up the room's habits sooner and fewer rule breaks go unseen. On your screen: the room's mood by position group, in words. Rulings your staff makes follow your past rulings |
| System Builder | The System | A week of install with a starter you signed as a fit | Players you signed as fits install your system faster (`world/systems.rs`, `fit_at`). On your screen: a recruit's fit job by job before you offer. Assistants who leave carry your system with them (`world/trends.rs`) |
| Sideline General | Us Against the World | A game as an underdog, or a half you trailed | Your room holds its best pitch of fire when behind or doubted (`game/talk.rs`). On your screen at the half: who is flat and who is too hot. In a sim your coach's talks find that pitch |
| Pro Factory | The Pipeline | A draft pick you signed and coached | Your draft picks at a position count for more as proof in a pitch to a recruit at that position (`claims::Alumni`). On your screen: which recruits care about it most. Your assistants use the proof too |
| Builder | The Tree | A season one of your former assistants spends as a coordinator or head coach elsewhere | Your tree takes your calls: leads on players and on staff. Candidates from your tree arrive knowing your system. On your screen: your tree, and who in it is ready for a bigger job |
| Institution | The Name | A season at the same program after you reached it | Part of the program's name moves with you (`world/program.rs`, `lived`), and your word weighs more with boosters and your boss. On your screen: what your leaving would cost the program, in words |

**The climb, for a head coach who does the work:** a second base archetype in season one, a first elite archetype at the end of season one or two, a hybrid in seasons two to four, Builder in seasons four to six. Institution is a career's work, and most coaches in the world never reach it.

**Your coach level** is your tiers added up and shown out of 100 (212 tiers = level 100). It buys nothing. It is the one number for how much coach you are, on your page and on every coach's page. Your title is your archetype, so version 4's names (Up-and-comer and the rest) are gone.

### 6. The 53 abilities

Each row: what counts as a rep, the rating it lifts, and one example of a screen perk. The eight signatures are in section 5.

Where reps come from:

- **Game-day abilities** level on the side of the ball you call. Reps come from game snaps in both ways of playing, and from that situation's period in practice and that part of the week's plan, so a head coach whose coordinator calls still earns them at a slower pace.
- **Recruiting abilities** level only from contacts you make yourself.
- **Teaching abilities** level from practice periods you run and focus points you set.

**Play Caller (base, 6).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Opening Script | A scripted play run in a game | Play calling | Tier 1: their tendencies on the first two series |
| Red Zone | A snap inside the 20 | Play calling | Tier 1: their red zone tendencies on the call screen |
| Third Down | A third-down snap | Play calling | Tier 2: a plan for each of their third-down fronts on your sheet |
| End of Half | A snap inside two minutes of a half | Game management | Tier 1: how many plays the clock still holds |
| Short Yardage and Fourth Down | A snap with 2 or fewer to go, or a fourth-down choice | Game management | Tier 1: the odds of each choice (a high school has no analyst today) |
| Install | A week of install you ran | Scheme | Tier 2: one more concept a week in camp |

**Closer (base, 6).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Living Room | An in-home visit | Recruiting | Tier 2: a visit takes less of your week |
| Official Visits | A weekend you hosted | Recruiting | Tier 2: one more stop on his weekend |
| Calls and Texts | A call or message | Recruiting | Tier 2: when each rival school last reached him |
| Closing | A talk in his last two weeks before signing | Recruiting | Tier 1: which school he leans to, as a range |
| Film | A game of film you watched | Evaluation | Tier 2: a game of film in two thirds of the time |
| High School Coaches | A visit to a coach's office | Network | Tier 2: he calls you first about a late riser |

**Players' Coach (base, 6).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Your Room | A practice period with the group you run | That group's teaching | Tier 2: run a second group one day a week |
| Flaw Fixing | A week on a flaw you set as a focus | That group's teaching | Tier 2: two flaws in one man at once |
| Freshmen | A week of a first-year player's install | That group's teaching | Tier 2: a freshman carries one more package |
| One-on-Ones | A one-on-one in your office | Relationships | Tier 4: players come to you before a problem grows |
| Standards | A ruling on a team rule | Discipline | Tier 2: your captains report what the staff missed |
| Veterans | A talk with a junior or senior | Relationships | Tier 1: which veterans the room follows |

**Mastermind (elite, from Play Caller, 4).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Game Planning | A game plan you built | Scheme | Tier 4: their likely answer to each of your top five calls |
| Adjustments | A drive after one that hurt you; a halftime | Adaptation | Tier 2: pick two adjustments at the half |
| Pressure | A snap against their blitz (offense) or your own blitz call (defense) | Play calling | Tier 2: tag a hot route on any pass call |
| Practice Planning | A week's plan you set | All teaching, a little | Tier 2: one more full-speed period at the same load |

**Trail Boss (elite, from Closer, 4).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Flipping | A talk with a recruit committed elsewhere | Recruiting | Tier 1: how firm his commitment is |
| The Portal | A talk with a transfer | Recruiting | Tier 1: why he is leaving |
| Parents | A talk with his parents in the room | Relationships | Tier 2: what each parent wants, apart from him |
| Gems | A look at a player no other staff has watched | Evaluation | Tier 1: a flag on players nobody has watched |

**Standard Setter (elite, from Players' Coach, 3).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Keeping Players | A talk with a player thinking of leaving | Relationships | Tier 1: who is thinking of leaving, a week sooner |
| Staff Room | A staff meeting | Relationships | Tier 2: one coach takes a second job without dropping his first |
| Character | A read of a player's makeup | Evaluation | Tier 1: one makeup trait as a word on every file |

**System Builder (hybrid, Play Caller and Closer, 3).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Transfer Tape | A read of tape from another level | Evaluation | Tier 1: his numbers set to your level |
| Position Changes | A week of a player at a new spot | That group's teaching | Tier 1: his fit at the new spot before you move him |
| Projection | A read of a player 19 or younger | Evaluation | Tier 2: a tighter range on his body at 21 |

**Sideline General (hybrid, Play Caller and Players' Coach, 3).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Pregame Talks | A pregame talk | Motivation | Tier 2: aim a talk at one position group |
| Halftime Talks | A halftime talk | Motivation | Tier 2: a word to one player on top of the room |
| After a Loss | A week after a loss | Motivation | Tier 1: who took the loss hardest |

**Pro Factory (hybrid, Closer and Players' Coach, 4).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Tool Building | A week on a tool you set as a focus | That group's teaching | Tier 1: how many weeks a tool still needs |
| Late Bloomers | A week with a player of 21 or older who still grew | That group's teaching | Tier 1: who still has room, in words |
| Pro Prep | A week with a draft-eligible player | That group's teaching | Tier 1: what NFL scouts mark against him |
| Medical | A read of an injury file | Evaluation | Tier 1: which injuries tend to come back |

**Builder (top, 4).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Hiring | An interview | Network | Tier 2: one more of a candidate's skills shown |
| Press | A press conference | Relationships | Tier 2: steer one question a week |
| Deals | An offer made in a negotiation | Negotiating | Tier 1: his ask as a tighter range |
| Bluffs | A claim by his side that you judged | Negotiating | Tier 1: a flag when his story doesn't match the market |

**Institution (top, 2).**

| Ability | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Boosters | An ask to a donor | Network | Tier 1: what a donor wants his name on |
| Your Boss | A meeting with your boss | Network | Tier 1: what your boss is judged on this year |

### 7. Rust

An ability you don't use for a full season drops one tier a season, never below tier 2, and keeps the perks it reached. It comes back at double speed. Your ratings move with it. An archetype you unlocked stays yours. Since only about a dozen abilities fit in your week, most coaches hold a dozen sharp abilities and the rest at tier 2 or 3.

### 8. Where you start: your road and your first archetype

- Your road's ratings are your start, as today (`setup.rs`, `BACKGROUNDS`).
- You pick your first archetype from the ones your road allows. Its six abilities start at tier 1 or 2. Those tiers are already in your road's ratings, so they lift nothing; only tiers earned in play do.
- Your road gives two cold abilities (version 2's blind spots). A cold ability reads 2 points low in its situation and levels at half speed until tier 1.
- Nobody starts with an advanced archetype. Your road's past counts toward the tiers, never toward a feat.

| Road | First archetype | Cold abilities |
|---|---|---|
| Former NFL star, Pro Bowler | Players' Coach | Install; Flaw Fixing |
| Former NFL starter, journeyman | Players' Coach or Play Caller | End of Half; Boosters |
| Former college player | Players' Coach or Closer | Pro Prep; Short Yardage and Fourth Down |
| Never played past high school | Play Caller or Closer | Veterans; Living Room |
| High school coach | Players' Coach or Play Caller | The Portal; Deals |
| College position coach or coordinator | Play Caller, Closer or Players' Coach | End of Half; Press |
| NFL assistant or coordinator | Play Caller or Players' Coach | Living Room; High School Coaches |
| Scout or personnel man | Closer | Adjustments; Pregame Talks |
| Analytics | Play Caller | Living Room; Veterans |

### 9. Weekly goals

Three a week, drawn from your real week: the opponent's strengths, your roster's problems, your board. Each pays bonus reps in a named ability, about a week's worth. "Score touchdowns on 3 of your first 4 red zone trips: +10 Red Zone reps", against the league's 3rd best red zone defense. One swap a week. Season goals from your boss and your own ambitions (`legacy/ambitions.rs`) pay bonus reps in place of today's lesson points. When an archetype is close, one of the three goals aims at it.

### 10. Every coach in the world, and the world reading your archetype

- Every coach has the same 53 abilities and the same ladder. At world build his tiers are drawn from his jobs and years, and his feats are read from his record in the world's history (his units, classes, draft picks, bonds, postseason wins and titles), so the world starts in balance: a 50-year-old coordinator already holds his dozen sharp abilities, and a few are Masterminds.
- Each year his job gives him reps by the same curve and rust, and he unlocks archetypes by the same feats.
- His page shows his archetypes and abilities as your read of him does: vague for a stranger, clear for your own staff. An archetype is public, like a reputation. Tiers are your read.
- Hiring reads it: you hire a coordinator because he is a Mastermind with Third Down at tier 4. When a coordinator calls a side, his tiers call it and he earns the reps.
- The world reads yours, a little. A boss with an opening looks for the archetype his program lacks (`world/staff.rs`, `fill_opening`). A recruit hears it before your first call, and a Pro Factory's pitch on development starts as proof (`folk/case.rs`). A staff candidate weighs it. Noteworthy, never driving: it opens a door, and the pitch, the money and the results still decide.
- Shares, for the measure to hit (proposed): each elite archetype held by about 1 head coach or coordinator in 10 at a level, each hybrid by about 1 in 25, Builder by about 1 head coach in 30, Institution by a handful at each level.
- AI coaches' ratings keep today's yearly growth (`Coach::pace`) until the measure says whether abilities can carry them too.

### 11. What goes

- The yearly age growth for your coach (`coach.rs`, `age_one_year`). Decline from about 59 stays as built.
- The lesson-point letter (+1.0 in any rating). A story handled well pays bonus reps in the ability it tested.
- Version 3's XP, coach points, trees bought with points, capstones and trophies. Version 4's levels 0 to 10 and its coach-level names.

### 12. Kept from version 2

- **One store for your ratings** (the bug above). Ships first, on its own.
- **`coach_check`.** Plays your coach with an ability at zero and at tier 4, everything else the same, on the same seeds, per seat and in both ways of playing: wins, points, signings, growth. On a small world it also prints how many coaches hold each archetype and when a typical head coach unlocks each. It sets every size before anything ships.
- **Negotiating below the NFL**, the way it reads there (`front_office/nfl.rs`, `market_read`): Deals is how wide your read of his lowest number is, Bluffs how often you read a bluff right.
- The head coach whose coordinator calls is no longer dead on game day: his game-day abilities still prepare the team (section 2, the first column), and he earns their reps from practice periods and the plan.

### 13. What you see

- **The Career tab** (the mock-up): your archetype and coach level, this week's tiers with what each paid, the ladder of eleven with each locked archetype's tiers and feat as bars, the selected archetype's abilities with tier, bar, reps to the next tier and the next perk, your ratings as road plus earned, this week's goals, cold and rusty abilities.
- **A line the moment a tier lands**, in the game feed or the week's card: "Red Zone reached tier 2. New: a sharper suggested call inside the 20."
- **An unlock is a big moment**: its own card, a news line, and the abilities that jumped to tier 4 from banked reps listed on it.
- **A season page**: what each ability did, in counts. "Red Zone: 31 trips, 21 touchdowns; the table expected 18."
- **A coach's page** anywhere in the world: his archetypes, and his abilities as far as you know him.

## Where the loop bends real life

- An ability mastered in one season is faster than a real career. Cain chose the pace for a game where a season takes many real hours. Your road carries your past years, and AI coaches start already levelled, so the world stays in balance.
- Screen perks give you information a real coach would have in his head. Hidden numbers stay hidden: a perk shows ranges, words and tendencies, never a true rating or potential.
- A real reputation grows by degrees. An archetype switches on at a feat. The switch is the game's reward; the work under it (tiers 1 to 3 and the banked reps) grows by degrees as in life.
- Tier 4 behind an archetype is a game rule. The football reason: the last step of a craft comes with standing. Players, recruits and staff give more to a coach whose record backs him.

## Lanes

Each lane is its own thread. Order by proxy: the bug, then the measure, then the store. Calm mode holds: one engine lane at a time.

| Lane | What | Files | Waits for |
|---|---|---|---|
| RE-bug | One store for your ratings | `folk/talks.rs`, `pitch.rs`, `takeover.rs`, `handoff.rs`, `front_office/preview.rs`, `legacy/guys.rs` | Nothing |
| RE0 | `coach_check`, both ways of playing, and its world read (archetype shares, the climb) | `afm-tools` | RE-bug |
| RE1 | The store: abilities, reps, tiers, the pace, rust, the road's start and first archetype, ratings as road plus earned; the ladder, feats, unlocks and banked reps; age growth and lesson points out | `world/coach.rs`, a new `world/coach_ladder.rs`, `setup.rs`, `storylines/credit.rs`, `legacy/` | RE-bug |
| RE2 | The full perk list (159) written into this doc, then the perk gates on the screens | this doc, `afm-game` | RE1 |
| RE3 | Game-day abilities in the engine, all three columns (an engine lane) | `game/calling.rs`, `game/flow.rs`, `game/live.rs`, `world/season.rs` | RE0, RE1, and no other engine lane running |
| RE4 | The Closer's and the Players' Coach's side: recruiting, scouting, teaching and people abilities | `folk/case.rs`, `game/talk.rs`, `scouting/film/`, `craft_work.rs` | RE1 |
| RE5 | Builder's and Institution's abilities, negotiating below the NFL | `folk/persona/negotiate/`, `haggle.rs` | RE1, G5 coach pay |
| RE6 | Every coach in the world: tiers and archetypes from his record, hiring and the world's read of an archetype | `world/staff.rs`, `folk/hiring.rs`, `coach_look.rs`, `folk/case.rs` | RE1 |
| RE7 | Weekly and season goals paying reps | `legacy/ambitions.rs`, `storylines/` | RE1 |
| RE8 | Screens, from the mock-up | `app/ui` | RE1 |
| RE9 | The eight signatures | by signature: `game/`, `folk/`, `world/systems.rs`, `world/trends.rs`, `world/program.rs`, `legacy/tree.rs` | RE3, RE4, RE5 |

## Not measured

- How many lesson points a season pays today.
- What any rating but play calling is worth in wins, signings or growth. RE0 measures it.
- A season's worth of reps for each ability. The counts above are estimates; RE1 counts them on a small world before the curve is fixed.
- What "better prepared in a situation" is worth on the field. RE0 measures it before RE3 sets its size.
- How many coaches in a world meet each feat, and in which season a typical head coach does. RE0's world read measures it; the feats' sizes move until the shares in section 10 hold.

## Decided by Cain

- **The pace**: one skill maxes in one season of doing it (his card, 2026-11-24).
- **Both ways of playing**: every rating must make sense when you play a game yourself and when you sim it (2026-11-24). Section 2 is how.
- **Archetypes**: "lean into archetypes more"; "similar but rebranded to mine" (2026-11-24). Sections 5 and 6 are how.

## Decided by proxy 11-24/25

Cain was asleep and gave the proxy ("make decisions for me als long as you note them down"). Each pick is the one closest to real football that still keeps his three decisions above. He can change any of them; nothing here is his own word. Each has a name, since typed lists get renumbered.

1. **The ladder**: eleven archetypes in EA's shape, three base, three elite, three hybrid, two top. Why: it is the shape he asked for.
2. **Names**: Play Caller, Closer, Players' Coach; Mastermind, Trail Boss, Standard Setter; System Builder, Sideline General, Pro Factory; Builder, Institution. No name is EA's. Why: he asked for it rebranded; these are words coaches are called in real life. The abilities keep version 4's plain football names.
3. **Unlocks**: a feat plus tiers earned by work, no points. Why: it keeps "use it to level it", which he did not reject, inside EA's unlock rule.
4. **Tier 3 open, tier 4 gated** (this changes the pitch he saw, which banked all work until the unlock): a locked archetype's abilities work up to tier 3 from day one; the archetype opens tier 4 and its signature, and reps past tier 3 are banked. Why: in real life work pays when you do it, and a head coach's hiring, halftime talks and deals can't wait four seasons for Builder.
5. **Tiers**: four per ability in place of levels 0 to 10, at a tenth, four tenths, seven tenths and a full season of reps, paying 1.5, 3, 4 and 5 points, with screen perks at tiers 1, 2 and 4. Why: EA's shape, at the pace he chose; about one tier a week.
6. **Feats at your level**: each feat has a college, a high school and an NFL form (section 5), read against your own division, conference, district or league. Why: a Toledo coach can't sign a national top-five class, and a real reputation is made against your peers.
7. **The feats themselves**: the table in section 5. Why: each is a thing coaches are really known for, and each can be read from what the world already stores.
8. **Signatures**: one per advanced archetype, eight in all, each a full ability that starts at zero on the unlock. Why: an unlock needs something new to grind, and each signature needs the standing the feat gives.
9. **Your first archetype**: you pick it at the start from the ones your road allows. Why: EA lets you pick; your road is your real past, so it limits the pick.
10. **The base archetypes are open to all**: you hold a second or third at 8 of its 24 tiers. Why: nothing stops a real play caller from recruiting.
11. **Three ways** (version 4's question): your team always, your screen when you play, your coach when you sim. Why: it is the only way found to keep his rule on manual play and sims.
12. **Reps both ways** (version 4's question): a sim levels at the normal pace, your own calls at up to double or down to half. Why: playing well should pay, and simming should never be punished.
13. **Ratings** (version 4's question): road plus earned, up to 6 when all of a rating's abilities are at tier 4. Why: the ratings he called useless now move because of what he did.
14. **Rust**: one tier a season unused, never below tier 2, back at double speed, an archetype never lost. Why: coaches do go rusty at work they stop doing, and a title earned is not taken back.
15. **Your road**: starting tiers in your first archetype and two cold abilities; no advanced archetype at the start. Why: the ladder is the game, so nobody starts up it.
16. **Weekly goals**: three from your real week, each paying about a week of bonus reps in one ability, one aimed at an archetype that is close. Why: an aimed grind.
17. **The whole world**: every coach on the same abilities, ladder and feats, with his feats read from the world's history. Why: the AI on the same rules is a standing rule of this game.
18. **The world reads your archetype**: hiring, recruits and staff candidates, as a small factor. Why: reputations are real, and "influences are noteworthy, rarely driving".
19. **The climb**: second base archetype in season one, first elite at the end of season one or two, a hybrid in seasons two to four, Builder in four to six, Institution a career. Why: weekly rewards come from tiers; the ladder has to last a career.
20. **Coach level**: tiers out of 100, no names. Why: the archetype is the title now.
21. **Order**: the bug first, then the measure, then the store, each lane in its own thread, with no size final until `coach_check` has measured it. Why: the bug is real whatever the design, and the measure protects realism.
22. **The perk list**: lane RE2 writes all 159 into this doc and builds them without a separate round. Why: he asked not to be waited on; each perk shows only ranges, words and tendencies.
