# Your coach, earned: the coaching career (version 4, 2026-11-24)

Cain, 2026-11-24: "i want you overhaul/propasal a ratings overhaul, right now ratings i as a coach have feel useless"; "i want more of earn to receive type of system, similar to EA College footballs"; on version 1: "read through the code, give me a better overhaul"; on version 2: "not gamified enough"; on version 3: "i meant not gamified as a system it doesn't feel like grind/reward"; on the first pace: "too slow for a game of this realism and needing of time"; then: "use logic, certain stuff like clock manager is only used when simulating, the ratings need to make sense for both manual play and simmign games".

Version 4 keeps the reading of the code (main `d5a75d5`) and replaces version 3's XP, points and trees with one loop: **use it to level it.** Everything below is **proposed, not confirmed**, except the pace, which Cain chose on 2026-11-24: one skill reaches level 10 in one season of doing it. Nothing is built until Cain answers the questions at the end.

Mock-up of the Career screen: https://claude.ai/artifact/FyZoAghjYaBqiQR49RRgRN (click the six groups).

Decided already, so not asked again: Q20 "your ratings grow" (`docs/handoff/decisions.md`); `docs/design/people.md`, topic 15 (ratings grow from doing the job, the people around you, study, age and use); roads C3-C7 (locked strengths, the rest at the road's last job level); flaws and tools for players (`docs/design/individual-practice.md`); the game-day rule that your own ratings call the snaps you don't take (`docs/design/game-day.md`).

## What it does

- **Every coaching skill has its own level, 0 to 10, and levels only by doing that exact work.** 45 skills: Red Zone, Third Down, Adjustments, Living Room, Official Visits, Flaw Fixing, One-on-Ones, Film, and so on. Every rep fills that skill's bar.
- **Every level pays the moment you reach it.** No points to save or place.
- **Every skill pays three ways, so it works however you play.** Your team is better prepared in that situation, always. Your screen shows you more there when you play it yourself. Your coach decides better there when you sim.
- **The pace is weeks, not seasons.** Something levels almost every week. One skill gets its first perk inside two weeks, its second by midseason and level 10 after one season of doing it.
- **The long game is breadth.** About a dozen skills fit in your week and there are 45. What you keep doing is who you are; what you drop goes rusty.
- **Your ratings are the sum of it.** A rating rises because its skills did: Play Calling goes 15 to 17 because you called a season of red zones and third downs.
- **Weekly goals pay bonus reps** in a named skill.
- **Every coach in the world is on the same rules.**

Example. Season 1, week 9, head coach at Toledo, calling your own offense. This week Third Down reached level 7, Living Room level 5 and Calls and Texts level 6, which opened a perk: you now see when each rival school last reached a recruit. Your Play Calling reads 17, 15 from your road and 2 earned. End of Half sits at level 2: it is one of your road's two cold skills and levels at half speed until level 3. You handed the defense's calls to your coordinator, so those reps are his.

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

You earn XP from games, goals and recruiting; levels pay coach points; points buy abilities in archetype trees; your backstory opens one tree first; abilities have tiers and costs; nobody gets everything (the level cap went from 50 to 100 in College Football 26). What AFM takes: levels, points, a tree you can't finish, your road as the head start. What it doesn't: EA's abilities are flat perks. Here a tool moves one rating's read in one situation, and the battle still decides.

Sources: [Destructoid, CFB 26 coach abilities](https://www.destructoid.com/best-college-football-26-coach-upgrade-abilities-in-dynasty-mode/); [CBS Sports, CFB 26 dynasty deep dive](https://www.cbssports.com/college-football/news/ea-sports-college-football-26-dynasty-mode-deep-dive-exploring-the-biggest-changes-and-updates); [EA, CFB 26 dynasty](https://www.ea.com/games/ea-sports-college-football/college-football-26/features/cfb26-dynasty).


## Playing a game yourself and simming it (the code today)

Cain's point, checked in the code. A live game stops for you and records what the staff would do (`game/live.rs`, `LiveGame`; `docs/match-engine.md`, "Live games, officials and the toss"). "A call you pick replaces the staff's." So:

- When you call a snap yourself, your play calling, adaptation and game management read nothing on that snap. Your pick is the call.
- When a snap doesn't stop for you, when you watch, or when you let the staff coach it, the side you call is called by your own ratings (`season.rs`, `coach_caller`, `your_calls`) and the other side by its coordinator.
- Your fourth-down and two-point charts decide the snaps that don't stop for you.

So a hands-on player's game-day ratings do nothing, and a player who sims never sees them work. A skill that pays in only one way of playing is wrong. Section 2 below is the fix.

## The loop

Every number in this part is a starting value for the measure (section 12), except the pace.

### 1. Skills level by doing the work

- 45 skills in six groups (section 5). Each has a level from 0 to 10 and a bar.
- A rep is one piece of that exact work: a red zone snap, an in-home visit, a one-on-one, a game of film.
- A rep done well fills more. Your own call is graded against the numbers the game already has (the call table, the fourth-down model, his file, his market): a good one counts up to double, a poor one half. Reps against a better opponent count more.
- There are no points, no trees to buy and no refunds. The only way to Red Zone 10 is red zone work.

### 2. Every skill pays three ways

| | When | What your level does |
|---|---|---|
| Your team (or the person in front of you) | Always | How well your players are prepared for that situation, or how well your work lands on that person. The same whether you play, watch or sim |
| Your screen | When you do it yourself | What your coach knows and shows you there: tendencies, odds, his file, more choices, less time spent |
| Your coach | When you sim or hand the snap off | How well he decides there without you |

Game-day examples:

| Skill | Your team (always) | Your screen (you play) | Your coach (you sim) |
|---|---|---|---|
| Red Zone | Fewer busted assignments inside the 20; more red zone plays known cold | Their red zone tendencies; a sharper suggested call | Calls the red zone better |
| End of Half | Faster to the line; fewer procedure flags in hurry-up | How many plays the clock still holds; when to spend a timeout | Manages the clock better |
| Short Yardage and Fourth Down | Your short-yardage package runs cleaner | The odds of each choice, tighter the higher your level | Decides better |
| Adjustments | More of an adjustment reaches your players | A clearer report of what hurt you; a second adjustment to pick | Picks the right adjustment |

Off the field the first two columns do the work. A pitch lands by your Living Room level whether you type the lines or pick from your staff's options, and the screen shows you more of his file. When you hand a job to an assistant, his own levels do it and he earns the reps.

A skill that can't fill all its columns is cut or merged. Version 3's Clock Manager and Two-Minute Drill are one skill now (End of Half). Replay challenges are no skill: the replay assistant does that work.

### 3. What a level pays

- **Power, every level.** Half a point of the skill's rating in its own situation, so 5 points at level 10, the top size of a player's tool (`craft.rs`, `FULL`). It reads on your team's preparation always and on your coach's choices when you sim.
- **A screen perk at levels 3, 6 and 10.** Something new to see or do when you do that work yourself (section 5 has one example per skill; the full list of 135 is its own round with Cain).
- **Your rating.** Each of the 19 ratings is your road's start plus what its skills earned. All of a rating's skills at level 10 lift it by 6, to 20 at most. A season doing about half of a rating's work lifts it 2 to 3.

### 4. The pace (decided: one season)

Cain chose on 2026-11-24: one skill reaches level 10 in one season of doing it every week.

- Each skill has a season's worth of reps: about 150 red zone snaps for a play caller, about 25 in-home visits, about 120 one-on-ones, 13 halves.
- Level 3 comes at a tenth of that (inside two weeks), level 6 at four tenths (midseason), level 10 at all of it.
- Good work is up to twice as fast, so a sharp caller reaches Red Zone 10 by November. A sim levels at the normal pace.
- About a dozen skills fit in your week, so in your first season 3 to 5 skills level in a normal week.
- Everything at 10 would take about four seasons of rotating your work, and rust (section 7) means you won't hold it all.

### 5. The 45 skills

Each row: what counts as a rep, the rating it lifts, and one example of a screen perk.

**Game day (9).** On the side of the ball you call. Reps come from game snaps in both ways of playing, and from that situation's period in practice and that part of the week's plan, so a head coach whose coordinator calls still earns them at a slower pace.

| Skill | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Opening Script | A scripted play run in a game | Play calling | 3: their tendencies on the first two series |
| Red Zone | A snap inside the 20 | Play calling | 3: their red zone tendencies on the call screen |
| Third Down | A third-down snap | Play calling | 6: a plan for each of their third-down fronts on your sheet |
| Pressure | A snap against their blitz (offense) or your own blitz call (defense) | Play calling | 6: tag a hot route on any pass call |
| End of Half | A snap inside two minutes of a half | Game management | 3: how many plays the clock still holds |
| Short Yardage and Fourth Down | A snap with 2 or fewer to go, or a fourth-down choice | Game management | 3: the odds of each choice (a high school has no analyst today) |
| Adjustments | A drive after one that hurt you; a halftime | Adaptation | 6: pick two adjustments at the half |
| Game Planning | A game plan you built | Scheme | 10: their likely answer to each of your top five calls |
| Install | A week of install you ran | Scheme | 6: one more concept a week in camp |

**Recruiting (8).** Reps only from contacts you make yourself.

| Skill | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Living Room | An in-home visit | Recruiting | 6: a visit takes less of your week |
| Official Visits | A weekend you hosted | Recruiting | 6: one more stop on his weekend |
| Calls and Texts | A call or message | Recruiting | 6: when each rival school last reached him |
| Closing | A talk in his last two weeks before signing | Recruiting | 3: which school he leans to, as a range |
| Flipping | A talk with a recruit committed elsewhere | Recruiting | 3: how firm his commitment is |
| The Portal | A talk with a transfer | Recruiting | 3: why he is leaving |
| High School Coaches | A visit to a coach's office | Network | 6: he calls you first about a late riser |
| Parents | A talk with his parents in the room | Relationships | 6: what each parent wants, apart from him |

**Teaching (8).** Reps from practice periods you run and focus points you set.

| Skill | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Your Room | A practice period with the group you run | That group's teaching | 6: run a second group one day a week |
| Flaw Fixing | A week on a flaw you set as a focus | That group's teaching | 6: two flaws in one man at once |
| Tool Building | A week on a tool you set as a focus | That group's teaching | 3: how many weeks a tool still needs |
| Freshmen | A week of a first-year player's install | That group's teaching | 6: a freshman carries one more package |
| Position Changes | A week of a player at a new spot | That group's teaching | 3: his fit at the new spot before you move him |
| Practice Planning | A week's plan you set | All teaching, a little | 6: one more full-speed period at the same load |
| Late Bloomers | A week with a player of 21 or older who still grew | That group's teaching | 3: who still has room, in words |
| Pro Prep | A week with a draft-eligible player | That group's teaching | 3: what NFL scouts mark against him |

**People (8).**

| Skill | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Pregame Talks | A pregame talk | Motivation | 6: aim a talk at one position group |
| Halftime Talks | A halftime talk | Motivation | 6: a word to one player on top of the room |
| After a Loss | A week after a loss | Motivation | 3: who took the loss hardest |
| One-on-Ones | A one-on-one in your office | Relationships | 10: players come to you before a problem grows |
| Standards | A ruling on a team rule | Discipline | 6: your captains report what the staff missed |
| Veterans | A talk with a junior or senior | Relationships | 3: which veterans the room follows |
| Keeping Players | A talk with a player thinking of leaving | Relationships | 3: who is thinking of leaving, a week sooner |
| Staff Room | A staff meeting | Relationships | 6: one coach takes a second job without dropping his first |

**Scouting (6).**

| Skill | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Film | A game of film you watched | Evaluation | 6: a game of film in two thirds of the time |
| Character | A read of a player's makeup | Evaluation | 3: one makeup trait as a word on every file |
| Medical | A read of an injury file | Evaluation | 3: which injuries tend to come back |
| Projection | A read of a player 19 or younger | Evaluation | 6: a tighter range on his body at 21 |
| Gems | A look at a player no other staff has watched | Evaluation | 3: a flag on players nobody has watched |
| Transfer Tape | A read of tape from another level | Evaluation | 3: his numbers set to your level |

**Program (6).**

| Skill | A rep | Rating | Screen perk (example) |
|---|---|---|---|
| Hiring | An interview | Network | 6: one more of a candidate's skills shown |
| Boosters | An ask to a donor | Network | 3: what a donor wants his name on |
| Press | A press conference | Relationships | 6: steer one question a week |
| Deals | An offer made in a negotiation | Negotiating | 3: his ask as a tighter range |
| Bluffs | A claim by his side that you judged | Negotiating | 3: a flag when his story doesn't match the market |
| Your Boss | A meeting with your boss | Network | 3: what your boss is judged on this year |

### 6. Coach level and names

Your coach level is your skill levels added up and shown out of 100 (450 skill levels = level 100). It buys nothing. It is the one number for how much coach you are, on your page and on every coach's page. Names come at 20, 40, 60 and 80: Up-and-comer, Established, Proven, Elite.

### 7. Rust

A skill you don't use for a full season drops 2 levels a season, never below 5, and never loses a perk it reached at 3. It comes back at double speed. Your ratings move with it. Since only about a dozen skills fit in your week, most coaches hold a dozen sharp skills and the rest somewhere above 5.

### 8. Where you start: your road

- Your road's ratings are your start, as today (`setup.rs`, `BACKGROUNDS`).
- Your road gives starting levels in its own skills. A college offensive coordinator starts with Opening Script, Red Zone, Third Down and Game Planning at 4 or 5. Those levels are already in his road's ratings, so they lift nothing; only levels earned in play do.
- Your road gives two cold skills (version 2's blind spots). A cold skill reads 2 points low in its situation and levels at half speed until level 3. Proposed by road:

| Road | Cold skills |
|---|---|
| Former NFL star, Pro Bowler | Install; Flaw Fixing |
| Former NFL starter, journeyman | End of Half; Boosters |
| Former college player | Pro Prep; Short Yardage and Fourth Down |
| Never played past high school | Veterans; Living Room |
| High school coach | The Portal; Deals |
| College position coach or coordinator | End of Half; Press |
| NFL assistant or coordinator | Living Room; High School Coaches |
| Scout or personnel man | Adjustments; Pregame Talks |
| Analytics | Living Room; Veterans |

### 9. Weekly goals

Three a week, drawn from your real week: the opponent's strengths, your roster's problems, your board. Each pays bonus reps in a named skill, about a week's worth. "Score touchdowns on 3 of your first 4 red zone trips: +10 Red Zone reps", against the league's 3rd best red zone defense. One swap a week. Season goals from your boss and your own ambitions (`legacy/ambitions.rs`) pay bonus reps in place of today's lesson points.

### 10. Every coach in the world

Every coach has the same 45 skills. At world build his levels are drawn from his jobs and years, so the world starts in balance: a 50-year-old coordinator already holds his dozen sharp skills. Each year his job gives him reps by the same curve and rust. His page shows his skills as your read of him does: vague for a stranger, clear for your own staff. Hiring reads them: you hire a coordinator for Third Down 9. When a coordinator calls a side, his levels call it and he earns the reps. AI coaches' ratings keep today's yearly growth (`Coach::pace`) until the measure says whether skills can carry them too.

### 11. What goes

- The yearly age growth for your coach (`coach.rs`, `age_one_year`). Decline from about 59 stays as built.
- The lesson-point letter (+1.0 in any rating). A story handled well pays bonus reps in the skill it tested.
- Version 3's XP, coach points, six trees, capstones and trophies.

### 12. Kept from version 2

- **One store for your ratings** (the bug above). Ships first, on its own.
- **`coach_check`.** Plays your coach with a skill at 0 and at 10, everything else the same, on the same seeds, per seat and in both ways of playing: wins, points, signings, growth. It sets every size before anything ships.
- **Negotiating below the NFL**, the way it reads there (`front_office/nfl.rs`, `market_read`): Deals is how wide your read of his lowest number is, Bluffs how often you read a bluff right.
- The head coach whose coordinator calls is no longer dead on game day: his game-day skills still prepare the team (section 2, the first column), and he earns their reps from practice periods and the plan.

### 13. What you see

- **The Career tab** (the mock-up): coach level, this week's level-ups with what each paid, the 45 skills by group with level, bar, reps to the next level and the next perk, your ratings as road plus earned, this week's goals, your week's hours, cold and rusty skills.
- **A level-up line the moment it happens**, in the game feed or the week's card: "Red Zone reached level 6. New: a sharper suggested call inside the 20."
- **A season page**: what each skill did, in counts. "Red Zone: 31 trips, 21 touchdowns; the table expected 18."
- **A coach's page** anywhere in the world: his skills, as far as you know him.

## Where the loop bends real life

- A skill mastered in one season is faster than a real career. Cain chose the pace for a game where a season takes many real hours. Your road carries your past years, and AI coaches start already levelled, so the world stays in balance.
- Screen perks give you information a real coach would have in his head. Hidden numbers stay hidden: a perk shows ranges, words and tendencies, never a true rating or potential.

## Lanes (proposed, not confirmed)

| Lane | What | Files | Waits for |
|---|---|---|---|
| RE-bug | One store for your ratings | `folk/talks.rs`, `pitch.rs`, `takeover.rs`, `handoff.rs`, `front_office/preview.rs`, `legacy/guys.rs` | Nothing |
| RE0 | `coach_check`, both ways of playing | `afm-tools` | RE-bug |
| RE1 | The skills' store, reps, levels, the pace, rust, the road's start, ratings as road plus earned; age growth and lesson points out | `world/coach.rs`, a new `world/coach_skills.rs`, `setup.rs`, `storylines/credit.rs`, `legacy/` | P2 staff hiring (`coach.rs`) |
| RE2 | The full perk list with Cain (135), then the perk gates on the screens | design round, `afm-game` | RE1 |
| RE3 | Game-day skills in the engine, all three columns (an engine lane) | `game/calling.rs`, `game/flow.rs`, `game/live.rs`, `world/season.rs` | The FBS rushing lane, RE0 |
| RE4 | Recruiting, Teaching, People and Scouting skills | `folk/case.rs`, `game/talk.rs`, `scouting/film/`, `craft_work.rs` | P1 recruiting (`recruiting.rs`) |
| RE5 | Program skills, negotiating below the NFL | `folk/persona/negotiate/`, `haggle.rs` | G5 coach pay |
| RE6 | Every coach in the world: skills from his record, hiring | `world/staff.rs`, `folk/hiring.rs`, `coach_look.rs` | P2 staff hiring |
| RE7 | Weekly and season goals paying reps | `legacy/ambitions.rs`, `storylines/` | RE1 |
| RE8 | Screens, from the mock-up | `app/ui` | RE1 |

## Not measured

- How many lesson points a season pays today.
- What any rating but play calling is worth in wins, signings or growth. RE0 measures it.
- A season's worth of reps for each skill. The counts above are estimates; RE1 counts them on a small world before the curve is fixed.
- What "better prepared in a situation" is worth on the field. RE0 measures it before RE3 sets its size.

## Decided

- **The pace**: one skill reaches level 10 in one season of doing it (Cain's card, 2026-11-24).
- **Both ways of playing**: every rating must make sense when you play a game yourself and when you sim it (Cain, 2026-11-24). Section 2 is the proposed way to do it.

## 11 questions for Cain

This list replaces version 3's 12. All proposed, not confirmed. Each has a name, since typed lists get renumbered.

1. **Use it to level it: skills level only by reps, with no points to spend?** Example: about 150 red zone snaps make Red Zone 10; nothing else can. In games: your coach is built by how you spend your weeks. My pick: yes.
2. **Three ways: your team always, your screen when you play, your coach when you sim?** Example: Red Zone 8 means fewer busted assignments inside the 20 in every game, their red zone tendencies on your call screen when you call, and better calls when you sim. In games: a hands-on player and a simmer both feel the same skill. My pick: yes.
3. **Reps both ways: a sim levels at the normal pace, your own calls up to twice or down to half?** Example: a sharp caller reaches Red Zone 10 by November, a simmer by the bowl game, a careless caller next season. In games: nothing directly. My pick: yes.
4. **Power: half a point a level, 5 at level 10, in its own situation?** Example: Red Zone 10 on Play Calling 14 reads 19 inside the 20 and 14 everywhere else. In games: clearly more red zone touchdowns; the measure gives the number before it ships. My pick: yes.
5. **Ratings: road plus earned, up to 6 when all of a rating's skills are at 10?** Example: your Play Calling 14 reaches 20 with Opening Script, Red Zone, Third Down and Pressure all at 10; a season of half that work lifts it 2 to 3. In games: your whole offense is called better on snaps you don't take. My pick: yes.
6. **Screen perks at levels 3, 6 and 10?** Example: Living Room 3 would show one more thing his family cares about; Calls and Texts 6 shows when each rival last reached him. In games: nothing on the field; you decide with more in front of you. My pick: yes, with the full list as its own round.
7. **Rust: a skill unused for a season drops 2 levels a season, never below 5, and comes back at double speed?** Example: three NFL seasons take Living Room from 10 to 5; half a season of visits brings it back. In games: your first class back lags a little. My pick: yes.
8. **Your road: starting levels in its skills and two cold skills?** Example: a college coordinator starts with Red Zone and Third Down at 4 or 5 and with End of Half and Press cold: 2 points low, half speed until level 3. In games: your first weeks have a real weakness. My pick: yes.
9. **Weekly goals: three from your real week, each paying about a week of bonus reps in one skill?** Example: "score touchdowns on 3 of your first 4 red zone trips: +10 Red Zone reps". In games: nothing directly. My pick: yes.
10. **The whole world: every coach has the 45 skills?** Example: two coordinator candidates read "Third Down 9, Pressure 8" and "Game Planning 8, Living Room 7". In games: your hire changes third downs or recruiting, not everything at once. My pick: yes.
11. **Order: the bug first, then the measure, then the skills, with the engine side after the FBS rushing lane?** Example: the bug fix ships this week on its own; nothing else changes your game until the measure has set the sizes. My pick: yes.
