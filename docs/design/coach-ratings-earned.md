# Your coach, earned: the coaching career (version 2, 2026-11-24)

Cain, 2026-11-24: "i want you overhaul/propasal a ratings overhaul, right now ratings i as a coach have feel useless"; "i want more of earn to receive type of system, similar to EA College footballs"; then, on version 1: "read through the code, give me a better overhaul".

Version 2 is written from the code (main `d5a75d5`), not from the reference pages. It replaces version 1 (bars on the 19 ratings). Everything below is **proposed, not confirmed**. Nothing is built until Cain answers the questions at the end.

Decided already, so not asked again: Q20 "your ratings grow" (`docs/handoff/decisions.md`); `docs/design/people.md`, topic 15 (ratings grow from doing the job, the people around you, study, age and use; your negotiating "reads tells and bluffs, senses floors and spots traps"); roads C3-C7 (locked strengths, the rest at the road's last job level); flaws and tools for players (`docs/design/individual-practice.md`); the game-day rule that your own ratings call the snaps you don't take (`docs/design/game-day.md`).

## What it does

- **You get a coach level and five experience bars** (game day, teaching, people, roster, front office). A bar fills only from that kind of work, done well. A full bar pays a coach point for that area.
- **Coach points buy two things: rating steps and tools.** A tool is a named piece of coaching know-how that works in one situation: "red zone caller", "closer in the living room", "fixes footwork", "halftime adjuster". It is the coach's version of the players' flaws and tools.
- **You open a tool by doing that exact work.** Call 120 red zone snaps and "red zone caller" opens; a point then buys it. No coach can buy everything.
- **You start with blind spots from your road** and can work them off. An analytics man starts "cold in living rooms"; a former star "slow to trust a scheme he never ran".
- **Your staff has tools too.** You hire a coordinator because he is a third-down caller, and a line coach because he fixes pass sets.
- **The ratings that do nothing today get work to do.** As a head coach who lets coordinators call, your scheme, play calling and adaptation are switched off today.
- **You see it.** A weekly line of what you earned, your tools on your page, and a season page of what each rating and tool did.

Example season. You call your own offense at an FBS school and do your own quarterback recruiting. By November your game-day bar has filled once and your roster bar once. You have called 131 red zone snaps and made 9 in-home visits, so two tools are open. You spend one point on "red zone caller" (your play calling reads 1 point higher inside the 20) and one on "closer in the living room" (your pitch lands harder on an in-home visit). Your teaching bar is a third full, because your position coaches ran the rooms.

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

## The overhaul

### 1. One store for your ratings (the bug)

Every reader takes your ratings from the coach record. `career.you` stays as the record of who you were on day one.

### 2. Five areas, five bars, one coach level

| Area | Ratings | Work that fills its bar |
|---|---|---|
| Game day | Scheme, play calling, adaptation, game management | Snaps you call, plans and call sheets you build, fourth downs, timeouts and challenges you decide, halftime adjustments |
| Teaching | The 8 groups | Practice minutes you spend with a group, focus points you set, flaws your players fix |
| People | Motivation, discipline, relationships | Talks, rulings, one-on-ones, parents, staff meetings |
| Roster | Recruiting, evaluation | Contacts, visits, pitches, film you watch, looks you settle |
| Front office | Network, negotiating | Deals, hires, clinics, favors, calls to other coaches |

Work is graded on how well it was done, from numbers the game already has: a call against what the call table expected for that look, a pitch against his file, a read against what he turned out to be, a deal against his market. Results don't fill a bar. A full bar pays one coach point for that area. Your coach level is the points you have earned. A head coach who does every job himself fills about five bars a season; one who hands recruiting to his staff fills none in Roster.

### 3. Rating steps cost more as they rise

One point a step below 14, two from 14, three from 17, four for 19 to 20. A point bought in an area goes to a rating in that area.

### 4. Coach craft: tools

A tool moves one rating's read in one situation, the way a player's tool does (`craft.rs`). Three tiers, worth 1, 2 and 3 points of that rating on the 1-20 scale, costing 1, 2 and 3 coach points. A tool opens when you have done that work enough; then a point buys it.

First set (proposed; each sits on a moment the code already has):

| Rating | Tools | Where it reads |
|---|---|---|
| Play calling, offense | Red zone; third down; opening script; two-minute; against the blitz; play action | The caller's reading and situational skill in that situation (`game/calling.rs`) |
| Play calling, defense | Third-down pressure; red zone; disguise; against tempo | The same, on defense |
| Adaptation | Halftime; mid-drive; bad weather | The run-pass and coverage shifts after what hurt you |
| Game management | Fourth downs; the clock before the half; late timeouts; challenges | `game/flow.rs` |
| Scheme | Installs a new playbook fast; self-scout (your own tells); opponent film | `systems.know`, `film_tells`, film study |
| Teaching, each group | Fixes flaws; builds tools; freshmen; position changes | The yearly craft step and growth in that group (`craft::year`, the coaching number) |
| Motivation | The underdog talk; after a loss; halftime; big games; one man alone | How much of a talk reaches him (`game/talk.rs`, `land`) |
| Discipline | Sees more on the road; standards with stars | What your staff sees (`folk/rules.rs`) |
| Relationships | Parents; veterans; quarterbacks; your staff; boosters; the press | Your read and your trust gains with that kind of person |
| Recruiting | The living room; the official visit; signing week; the portal; flipping a commit; one state | How a point lands in that setting (`folk/case.rs`, `lands`) |
| Evaluation | One position group's eye (8); late bloomers; character; medical | Your eye on that kind of read (`scouting/film/`) |
| Network | High school coaches; NFL scouts; your tree | Credibility and sources with them |
| Negotiating | Agents; NIL; staff contracts; trades | Your read of his lowest number |

About 75 tools with the per-group ones. At three tiers each that is far more than a career's points.

### 5. Coach craft: blind spots

A blind spot is the mirror: it costs 1 to 3 points of a rating in one situation. Your road starts you with two. You work one off by doing that work (the same count that opens a tool) and paying two points. Proposed by road:

| Road | Blind spots |
|---|---|
| Former NFL star, Pro Bowler | Slow to trust a scheme he never ran; impatient with slow learners |
| Former NFL starter, journeyman | The clock before the half; the booster room |
| Former college player | NFL scouts don't know him; fourth downs |
| Never played past high school | Veterans test him; the living room of a star |
| High school coach | The portal; agents |
| College position coach or coordinator | Game management as the man in charge; the press |
| NFL assistant or coordinator | The living room; high school coaches don't know him |
| Scout or personnel man | Halftime; the team talk |
| Analytics | Cold in living rooms; veterans test him |

### 6. Age stops paying you

The yearly age growth goes for your coach. Age only slows the bars: full pace to 45, 80% to 55, 60% after. Decline from about 59 stays as built.

### 7. Lesson points become coach points

A story handled well or an ambition's step adds experience to the area it tested. The +1.0 letter goes; the choice moves to the points.

### 8. What you don't use goes dull

A tool unused for three seasons drops a tier; a rating whose area earned nothing for two seasons loses 0.3 a year, never below your road's start.

### 9. The head coach's scheme in the week's plan

On the side of the ball you came up on, your scheme shares film study and the spring install with that coordinator, half each. This is the rule the code already uses for a coordinator and the room he came up in (C60).

### 10. The head coach's adaptation at halftime

In the second half the caller's adaptation is half his and half yours. A head coach who calls nothing still leads the adjustments.

### 11. Negotiating below the NFL

In NIL, portal, staff and your own contract talks, your negotiating sets how wide your read of his lowest number is and how often you read a bluff right. The NFL already does both (`front_office/nfl.rs`, `market_read`: a range 22% wide at 10, 1 point narrower a rating point; line 1124: right 45% of the time at 10, 3 points more a rating point). The same rules everywhere.

### 12. Your staff has tools and blind spots

Every coach in the world gets them from his own career: drawn from his jobs and years at world build, grown each year by his job. They show on his page as your read of him does, vague for a stranger and clear for your own staff. Hiring reads them: a coordinator's tools are his calling in those situations.

### 13. Names you earn

From results the world can see: quarterbacks drafted, top classes, underdog wins, fourth downs. On your page and in the press. Recruits, agents and bosses read them through the proof and reputation the game already has (`world/folk/claims/`).

### 14. You see it

- The week's line in your inbox: "+0.08 game day: 71 snaps called against Iowa, 9 better than the table".
- Your profile's Career tab: level, five bars, your tools and blind spots, what is close to opening.
- A season page: what each rating and tool did, in counts. "Red zone caller: 31 trips, 21 touchdowns; the table expected 18."

### 15. Measure first

A tool (`coach_check`) plays your coach at 5 and at 18 in one rating, everything else the same, on the same seeds, per seat: wins, points, signings, growth, money. It sets the tools' sizes and shows any rating that still does too little.

## Built with it, no question

- A coordinator or position coach better than you still lifts you in his area (the mentor rule, `age_one_year`), paid as experience in that area.
- AI coaches keep their yearly growth by job (`Coach::pace`); only tools and blind spots are added for them.
- Your staff-development effect (`legacy/tree.rs`) is shown on the staff screen.
- Ratings stay 1-20 on the world's scale.

## Lanes (proposed, not confirmed)

| Lane | What | Files | Waits for |
|---|---|---|---|
| RE-bug | One store for your ratings (item 1) | `folk/talks.rs`, `pitch.rs`, `takeover.rs`, `handoff.rs`, `front_office/preview.rs`, `legacy/guys.rs` | Nothing |
| RE0 | `coach_check` (item 15) | `afm-tools` | RE-bug |
| RE1 | Areas, bars, points, costs, age, lesson points, fading (items 2, 3, 6, 7, 8) | `world/coach.rs`, a new `world/folk/your_work.rs`, `storylines/credit.rs`, `legacy/` | P2 staff hiring (`coach.rs`) |
| RE2 | Coach craft: the store, your tools and blind spots, how they open (items 4, 5) | a new `world/coach_craft.rs`, `setup.rs` | RE1 |
| RE3 | Game-day tools, the plan and halftime (items 9, 10; an engine lane) | `game/calling.rs`, `game/flow.rs`, `world/season.rs` | The FBS rushing lane, RE0 |
| RE4 | World-side tools: talks, pitches, film, teaching | `folk/case.rs`, `game/talk.rs`, `scouting/film/`, `craft_work.rs` | P1 recruiting (`recruiting.rs`) |
| RE5 | Negotiating below the NFL (item 11) | `folk/persona/negotiate/`, `haggle.rs` | G5 coach pay for staff and your own contract |
| RE6 | The world's coaches' tools and hiring (item 12) | `world/staff.rs`, `folk/hiring.rs`, `coach_look.rs` | P2 staff hiring |
| RE7 | Names (item 13) | `storylines/`, `folk/claims/` | RE1 |
| RE8 | Screens (item 14), after a mock-up | `app/ui` | RE1, RE2 |

## Not measured

- How many lesson points a season pays today.
- What any rating but play calling is worth in wins, signings or growth. RE0 measures it.
- Every size above is a starting value for RE0.

## 12 questions for Cain

One aspect each. Items 1 (the bug), 6, 7, 14 and 15 are not asked: 1 is a plain bug, 6 and 7 follow from the bars, 14 and 15 are how the rest is shown and sized.

1. **Tools as the abilities you earn (section 4)?** Example: "red zone caller" at tier 2 reads your play calling 14 as 16 inside the 20 and as 14 everywhere else. In games: your offense scores a little more often on red zone trips you call; the field between the 20s is unchanged. My pick: yes.
2. **A tool opens by doing that exact work, then costs a point (section 4)?** Example: after 120 red zone snaps called the tool opens; you choose it over a rating step. With no work, no tool, however many points you hold. In games: your build follows what you really did. My pick: yes.
3. **Two blind spots from your road at the start (section 5)?** Example: an analytics coach's pitch reads 2 points lower on an in-home visit until he has made about 15 of them and paid 2 points. In games: your first seasons have a real weakness to manage or hand to staff. My pick: yes.
4. **Five areas with one point per full bar, spent inside that area (section 2)?** Example: a season of calling plays and recruiting yourself pays about one game-day point and one roster point; neither can go into teaching. In games: nothing directly; it sets what you can build. My pick: yes.
5. **Rating steps at 1, 2, 3 and 4 points by height (section 3)?** Example: recruiting 13 to 14 costs one point, 17 to 18 three. In games: a 20 is a career's work, as it is for the world's best. My pick: yes.
6. **Unused tools drop a tier after three seasons, unused ratings fade 0.3 a year (section 8)?** Example: six NFL seasons take a college recruiter from 16 to about 14.8 and his "living room" tool from tier 3 to 1. In games: going back to college, your first classes lag your name. My pick: yes.
7. **Your scheme shares the week's plan on the side you came up on (section 9)?** Example: a defensive head coach with scheme 17 and a coordinator at 13 plans the week at 15, where today it is 13. In games: your defense reads the other team's film better on Saturday. My pick: yes.
8. **Your adaptation is half of the second half's adjustments (section 10)?** Example: your adaptation 16 with a coordinator at 10 plays the second half at 13. In games: after halftime your side leans faster to what worked and away from what hurt. My pick: yes.
9. **Negotiating read in college and high school deals (section 11)?** Example: a transfer's side says another school offered more. At 10 you read it right 45% of the time, at 18 69%; and your read of his number is "$445K to $555K" at 10 and "$465K to $535K" at 18. In games: nothing on the field; you overpay less. My pick: yes.
10. **Every coach in the world gets tools and blind spots (section 12)?** Example: two coordinator candidates both read "scheme 14 to 16"; one is a third-down caller, the other installs fast. In games: your hire changes third downs or your first spring, not everything at once. My pick: yes.
11. **Names from results, read by recruits, agents and bosses (section 13)?** Example: three quarterbacks drafted in five years make you "a quarterback developer", and a quarterback recruit weighs it as proof in your pitch. In games: nothing directly; it pulls players at that spot. My pick: yes.
12. **Order: the bug first, then the measure, then the bars and tools, with screens after a mock-up (the lane table)?** Example: the bug fix ships this week on its own; nothing else changes your game until the measure has set the sizes. My pick: yes.
