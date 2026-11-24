# Your coach, earned: the coaching career (version 3, 2026-11-24)

Cain, 2026-11-24: "i want you overhaul/propasal a ratings overhaul, right now ratings i as a coach have feel useless"; "i want more of earn to receive type of system, similar to EA College footballs"; on version 1: "read through the code, give me a better overhaul"; on version 2: "not gamified enough".

Version 3 keeps version 2's reading of the code (main `d5a75d5`) and turns the overhaul into a game loop: XP, a coach level, skill trees, goals and trophies. It replaces version 2's five bars. Everything below is **proposed, not confirmed**. Nothing is built until Cain answers the questions at the end.

Mock-up of the Career screen: https://claude.ai/artifact/FyZoAghjYaBqiQR49RRgRN (click the six trees).

Decided already, so not asked again: Q20 "your ratings grow" (`docs/handoff/decisions.md`); `docs/design/people.md`, topic 15 (ratings grow from doing the job, the people around you, study, age and use; your negotiating "reads tells and bluffs, senses floors and spots traps"); roads C3-C7 (locked strengths, the rest at the road's last job level); flaws and tools for players (`docs/design/individual-practice.md`); the game-day rule that your own ratings call the snaps you don't take (`docs/design/game-day.md`).

## What it does

- **You earn XP for everything you do as a coach, and you see it land.** After every game a card lists it: "Beat the line by 10 points, +240. Touchdowns on 4 of 5 red zone trips you called, +90."
- **XP raises your coach level, 1 to 100.** Every level pays a skill point. Levels 10, 25, 50, 75 and 100 pay three and a new name ("Up-and-comer", "Established").
- **Points go into six skill trees:** Tactician, Recruiter, Developer, Motivator, Scout, Program Builder. Your road opens the first. You open the others by a feat, such as signing 10 players you pitched yourself.
- **A tree holds rating steps and abilities.** An ability works in one situation: Red Zone Caller, Living Room, Halftime Adjuster, Flaw Fixer. Each has bronze, silver and gold.
- **You can't buy what you haven't done.** A tree's higher tiers open by XP earned in that tree. Silver and gold need the work itself: gold Red Zone Caller needs 400 red zone snaps you called.
- **A career fills about a sixth of everything.** About 110 points against about 660 to buy it all. Each tree ends in a capstone, and you may hold two.
- **Goals every week.** Three weekly goals drawn from your real week, season goals from your boss and your own ambitions, and about 120 trophies. A trophy shows how many of the world's coaches hold it.
- **You start with two blind spots from your road** and work them off.
- **Every coach in the world is on the same rules.** You hire a coordinator because he is a level 41 Tactician with gold Third-Down Caller.
- **It stays football.** An ability moves one rating's read in one situation by up to 3 points. The battle on the field still decides, and a good call never beats talent.

Example. Season 4, week 9, head coach at Toledo, calling your own offense. You are level 23 with 2 points to spend. You win 31-27 at Miami (Ohio) as a 6-point underdog: +910 XP, which takes you to 1,540 of the 2,380 for level 24. Your Red Zone Caller is silver and has the 400 snaps for gold, but gold costs 3 points, so you wait a level or put 2 into Halftime Adjuster's silver now. Recruiter is at tier 2 from your own visits. Scout is still closed: 6 of the 10 proved reads it needs.

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

## The loop

Every number in this part is a starting value for the measure (item 14).

### 1. XP

XP comes from events the world already records. Each event pays your level. Most also pay one tree.

| Event | XP | Tree |
|---|---|---|
| Coaching a game | 100 | Level only |
| A win | 150 | Level only |
| Each point better than the line, up to 20 | 24 | Level only |
| A call or decision of yours that beat the table: a red zone trip, a fourth down, a two-minute drive, a second half won after trailing | 40 to 110 | Tactician |
| A game plan you built yourself | 60 | Tactician |
| A visit you hosted, a pitch of yours that landed | 20 to 60 | Recruiter |
| A signing you pitched yourself, by his rank against your program's usual class | 40 to 300 | Recruiter |
| A practice you ran with a group | 10 | Developer |
| A flaw fixed or a tool built on your focus, a breakout, an all-conference player, a draft pick | 50 to 400 | Developer |
| A talk that landed, a one-on-one, a promise kept, a story handled well (today's lesson points) | 15 to 250 | Motivator |
| A read of yours that proved right at a look-back | 80 | Scout |
| A hire who works out, a deal under his market, a drive closed, an assistant hired as a head coach | 60 to 500 | Program Builder |
| A weekly goal | 80 to 200 | Its tree |
| A season goal or an ambition's step | 400 to 1,500 | Its tree, or level only |
| A trophy | 200 to 2,000 | Level only |

- Game XP reads the line, so an upset pays most. A win by 10 as a 10-point favorite pays 250. A win by 3 as a 14-point underdog pays 658. A loss as a 14-point favorite pays 100.
- Work your staff does pays your staff, not you. A head coach who hands recruiting to his assistants earns no Recruiter XP.
- An average FBS season where you do most jobs yourself pays about 10,000 XP: about 4,300 from 13 games, 2,500 from weekly goals, 1,300 from recruiting, 1,000 from development, 600 from people and 1,000 from season goals. A title season pays about 16,000.
- The yearly age growth (`coach.rs`, `age_one_year`) stops for your coach. Decline from about 59 stays as built.
- The lesson-point letter (+1.0 in any rating) goes. A story handled well pays XP in the tree it tested.

### 2. Coach level

- The next level costs 1,000 + 60 × your level. Level 2 costs 1,060 XP, level 50 costs 3,940, level 100 costs 6,940.
- Pace: about 8 levels in your first season, about 3 a season by year ten (about level 43), about level 85 after 30 average seasons. Level 100 takes 25 to 30 strong seasons.
- Each level pays one skill point. Levels 10, 25, 50, 75 and 100 pay three and a name: Up-and-comer, Established, Proven, Elite, Legend. About 110 points in a full career.
- A level-up is a moment on the Big moments screen, with the point to spend.

### 3. What points buy

| Buy | Cost | Needs |
|---|---|---|
| A rating step to 11-14 | 1 | The tree open |
| A rating step to 15-17 | 2 | Tier 2 of its tree |
| A rating step to 18 or 19 | 3 | Tier 3 of its tree |
| A rating step to 20 | 4 | Tier 3 of its tree |
| An ability at bronze | 1 | Its tier open |
| Bronze to silver | 2 | The work done (its counter) |
| Silver to gold | 3 | More of the work done |
| A capstone | 6 | Its tree XP and its feat |

Everything in the game costs about 660 points: about 290 for the 48 abilities, 36 for six capstones and about 340 to take all 19 ratings to 20. No refunds.

### 4. The six trees

| Tree | Its ratings | Its XP comes from | Opens by (one of) |
|---|---|---|---|
| Tactician | Play calling, scheme, adaptation, game management | Snaps you call, game plans, fourth downs, the clock, halftime | Calling a side for a season; 3 wins as an underdog of 7 or more |
| Recruiter | Recruiting | Your own calls, visits, pitches and signings | 10 signings you pitched yourself |
| Developer | The 8 teaching groups | Practice time with a group, focus points, players who grow | 3 of your players gaining 4 or more overall in one year; a player of yours drafted |
| Motivator | Motivation, discipline, relationships | Talks that land, one-on-ones, rulings, promises kept | 25 one-on-ones that landed; a win after trailing by 14 |
| Scout | Evaluation | Film you watch yourself, reads that prove right | 10 reads proved right at a look-back |
| Program Builder | Network, negotiating | Hires, deals, boosters, your boss, assistants who move up | Two seasons as a head coach; 5 hires |

- Your road opens one tree on day one and gives two of its tier 1 abilities at bronze.
- Tier 2 opens at 2,000 XP earned in that tree, tier 3 at 6,000, the capstone at 15,000 with its feat. A coach who works one tree hard reaches tier 2 in his first season, tier 3 in his third and the capstone in about his sixth.

### 5. The abilities

Each ability moves one rating's read in one situation: +1 at bronze, +2 at silver, +3 at gold, on the 1-20 scale. It is the coach's version of a player's tool (`craft.rs`). Each sits on a moment the code already has.

| Tree | Tier 1 | Tier 2 | Tier 3 |
|---|---|---|---|
| Tactician | Opening Script (your first 15 plays); Red Zone Caller (inside the 20); Clock Manager (the last two minutes of a half) | Third-Down Caller; Halftime Adjuster (the second half); Film Room (the week's film on the opponent) | Two-Minute Drill (trailing, under two minutes); Fourth-Down Nerve (fourth-down decisions) |
| Recruiter | Living Room (in-home visits); Official Visit (his weekend on campus); Home State (the state you have signed the most from) | Closer (signing week); Flipper (a recruit committed elsewhere); Portal Pitch (transfers) | Pipeline (high schools you have signed from); Family Man (his parents in the room) |
| Developer | Position Room (the group you run yourself); Flaw Fixer (a flaw you set as a focus); Freshman Ready (first-year players learning the playbook) | Tool Builder (a tool you set as a focus); Position Change (a player learning a new spot); Practice Planner (practice load against growth) | Late Bloomer (players of 21 and older with room left); Pro Ready (seniors in their last year) |
| Motivator | Bounce Back (the week after a loss); Open Door (one-on-ones); Underdog Talk (pregame as an underdog) | Halftime Speech; Big Game (rivals, title games, bowls); Standards (rules for your stars) | Veterans' Coach (juniors and seniors); Keeper (a player thinking about the portal) |
| Scout | Position Eye (one group you pick); Tape Grinder (hours of film in a week); Character Read (his makeup) | Projection (frames and late growth); Medical Read (his injury file); Second Eye (a second group) | Gem Finder (players with few looks on them); Transfer Eye (tape from another level) |
| Program Builder | Staff Builder (your read of a coach you interview); Booster Room (asking a donor); Press Room (press conferences) | Dealmaker (reading his lowest number); Coaching Tree (assistants learning from you); Boss's Ear (an ask to your boss) | Bluff Reader ("another school offered more"); Coaches' Coach (high school coaches and NFL scouts) |

- Silver and gold need a counter of that exact work. Red Zone Caller: silver at 120 red zone snaps you called, gold at 400. Living Room: silver at 15 in-home visits, gold at 60. The counters are set so gold takes about four seasons of doing that job yourself.
- Tactician abilities that read play calling work only on the side you call. For the side you don't call, see the capstone.
- A defensive coach's Tactician tree carries the defense's versions (third-down pressure, red zone defense, disguise) in the same slots.

### 6. Capstones

One a tree, 6 points, at most two in a career.

| Tree | Capstone | What it does | Its feat |
|---|---|---|---|
| Tactician | The System | Your coordinators call with your Tactician abilities at half strength | A conference title won while calling a side |
| Recruiter | Blue-Chip Magnet | Your proof is graded one step stronger against his other schools (`folk/claims/`) | A class in the top tenth of your division |
| Developer | Master Teacher | Pick one position group; every Developer ability reads one rank higher there | 5 drafted players who signed as three-stars or lower |
| Motivator | Culture | Your team leaders carry a talk to the players it missed | 5 wins in one season after trailing at the half |
| Scout | The Eye | Your first look at a player is as sharp as a third look | 3 all-conference starters you signed as two-stars or walk-ons |
| Program Builder | The Program | When an assistant leaves, his room keeps half of what it learned under him | 3 of your assistants hired as head coaches |

### 7. Blind spots

A blind spot is a red node: it costs 2 points of a rating in one situation. Your road starts you with two. You work one off by doing that work (a counter) and paying 2 points; it pays 300 XP when it goes. Proposed by road:

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

### 8. Goals

- **Weekly goals.** Three a week, drawn from your real week: the opponent's strengths, your roster's problems, your recruiting board. "Score touchdowns on 3 of your first 4 red zone trips" against the league's 3rd best red zone defense. "Sit down with the three seniors who lost their jobs." 80 to 200 XP each, harder ones more. One swap a week. Goals are about a quarter of a season's XP.
- **Season goals.** Your boss's goals and your own ambitions (`legacy/ambitions.rs`) pay XP in place of today's lesson points.
- **Career milestones.** 50 wins, a first draft pick, a first conference title.

### 9. Trophies

About 120, from the world's own records. Each pays XP once. Rarity is the share of the world's head coaches at your level who hold it, counted from the world, so it is true for this save. Examples: Giant Killer (beat a ranked team as an underdog of 10 or more), First Sunday (a player you coached is drafted), Sunday Pipeline (ten drafted), The Rebuild (3 wins or fewer to a bowl inside two seasons), Diamond (a two-star you signed goes in the first three rounds), Tree (three assistants hired as head coaches).

### 10. Rust

An ability unused for two seasons reads at half strength until you do that work again: 10 in-home visits bring Living Room back. Nothing is lost for good. Ratings don't fade.

### 11. Every coach in the world

Every coach gets a level, trees, abilities and blind spots. At world build they are drawn from his own career record (games, results against the line, signings, draft picks, jobs held). Each year his job pays XP by the same table from his season's totals, and his road and job decide where his points go. His page shows them as your read of him does: vague for a stranger, clear for your own staff. Hiring reads them. AI coaches' ratings keep today's yearly growth (`Coach::pace`) until the measure says whether XP can carry them too.

### 12. Where you start

Level 1, whatever your road. Your road's head start is its ratings (as today, `setup.rs`, `BACKGROUNDS`), one open tree, two bronze abilities and two blind spots. The other choice is a starting level by road (a former NFL coordinator at about 25); question 11.

### 13. What you see

- **The Career tab** on your page (the mock-up): level, XP bar, points, the six trees with every node's rank and counter, this week's goals, last game's XP, season goals, trophies, blind spots.
- **The post-game XP card**: every line of XP and which of your abilities read on how many snaps.
- **The level-up moment** on Big moments.
- **A season page**: what each ability did, in counts. "Red Zone Caller: 31 trips, 21 touchdowns; the table expected 18."
- **A coach's page** anywhere in the world: his level, trees and abilities, as far as you know him.

### 14. Kept from version 2

- **One store for your ratings** (the bug above). Every reader takes the coach record; `career.you` stays as the setup record. Ships first, on its own.
- **`coach_check`.** A tool that plays your coach at 5 and at 18 in one rating, and with and without each ability, on the same seeds, per seat: wins, points, signings, growth. It sets every size before anything ships.
- **The dead ratings get work.** As a head coach who doesn't call: your scheme shares the week's film study and the spring install with the coordinator on the side you came up on, half each; your adaptation is half of the second half's adjustments. Negotiating reads below the NFL the way it does there (`front_office/nfl.rs`, `market_read`): how wide your read of his lowest number is, and how often you read a bluff right.
- **The mentor rule.** A coordinator or position coach better than you still lifts you: his work on your staff pays you a share of XP in his tree.

## Where the game loop bends real life

- A real coach doesn't choose where his skill grows. Here you place points. The gates (tree XP, counters, feats) keep the choice inside the work you really did.
- Weekly goals and trophies are a game layer. They pay XP and change nothing on the field.
- Levels are a readout of XP. No rule reads your level except the points it pays.

## Lanes (proposed, not confirmed)

| Lane | What | Files | Waits for |
|---|---|---|---|
| RE-bug | One store for your ratings | `folk/talks.rs`, `pitch.rs`, `takeover.rs`, `handoff.rs`, `front_office/preview.rs`, `legacy/guys.rs` | Nothing |
| RE0 | `coach_check` | `afm-tools` | RE-bug |
| RE1 | XP, the level, points, the XP book; age growth and lesson points out (items 1-3) | `world/coach.rs`, a new `world/folk/career_xp.rs`, `storylines/credit.rs`, `legacy/` | P2 staff hiring (`coach.rs`) |
| RE2 | Trees, tiers, counters, the abilities' store, blind spots, rust, the start (items 4-7, 10, 12) | a new `world/coach_craft.rs`, `setup.rs` | RE1 |
| RE3 | Tactician abilities, the head coach's plan and halftime (an engine lane) | `game/calling.rs`, `game/flow.rs`, `world/season.rs` | The FBS rushing lane, RE0 |
| RE4 | Recruiter, Developer, Motivator and Scout abilities | `folk/case.rs`, `game/talk.rs`, `scouting/film/`, `craft_work.rs` | P1 recruiting (`recruiting.rs`) |
| RE5 | Program Builder abilities, negotiating below the NFL | `folk/persona/negotiate/`, `haggle.rs` | G5 coach pay |
| RE6 | Every coach in the world: levels and abilities from his record, hiring (item 11) | `world/staff.rs`, `folk/hiring.rs`, `coach_look.rs` | P2 staff hiring |
| RE7 | Goals and trophies with rarity (items 8, 9) | `legacy/ambitions.rs`, `storylines/`, a new `world/folk/trophies.rs` | RE1 |
| RE8 | Screens (item 13), from the mock-up | `app/ui` | RE1, RE2 |

## Not measured

- How many lesson points a season pays today.
- What any rating but play calling is worth in wins, signings or growth. RE0 measures it.
- The XP a real season of play pays. The table's sizes and the 10,000 a season are estimates; RE1 counts them on a small world before the level curve is fixed.

## 12 questions for Cain

This list replaces version 2's 12. All proposed, not confirmed. Each has a name, since typed lists get renumbered.

1. **Six trees: your road opens one, the others open by a feat?** Example: a college offensive coordinator starts with Tactician; Recruiter opens once he has signed 10 players he pitched himself. In games: nothing by itself; it sets what you can build first. My pick: yes.
2. **Levels: 1 to 100, one point a level, three at 10, 25, 50, 75 and 100?** Example: about 8 levels in your first season, about 3 a season by year ten, level 100 after 25 to 30 strong seasons. About 110 points against 660, so a career fills about a sixth. In games: two coaches with 20 seasons each are built differently. My pick: yes.
3. **Earn before you buy: tiers open by XP in that tree, silver and gold by the work itself?** Example: ten wins don't open Recruiter tier 2, 2,000 XP from your own visits and pitches does; gold Red Zone Caller needs 400 red zone snaps you called (about four seasons) and 3 points. In games: your build follows what you really did. My pick: yes.
4. **Upset XP: game XP reads the line?** Example: a win by 10 as a 10-point favorite pays 250; a win by 3 as a 14-point underdog pays 658; a loss as a 14-point favorite pays 100. In games: nothing on the field; a small school's coach levels as fast as a blue blood's. My pick: yes.
5. **Ability size: up to 3 points of its rating in its own situation?** Example: gold Red Zone Caller reads your play calling 14 as 17 inside the 20 and as 14 everywhere else. Players' tools go to 5. In games: a few more red zone touchdowns a season on trips you call; `coach_check` measures it before it ships. My pick: 3, raised only if the measure shows you can't feel it.
6. **Capstones: one a tree, two in a career?** Example: The System: your coordinators call with your Tactician abilities at half strength; it needs 15,000 Tactician XP and a conference title won while calling a side. In games: your red zone edge reaches the side you don't call. My pick: yes.
7. **Weekly goals: three from your real week, one swap?** Example: "score touchdowns on 3 of your first 4 red zone trips, 180 XP", against the league's 3rd best red zone defense. In games: nothing directly; one part of each game matters more to you. My pick: yes.
8. **Trophies: about 120, rarity counted from this world's coaches?** Example: "Giant Killer: beat a ranked team as an underdog of 10 or more. 11% of FBS head coaches hold it." In games: nothing; XP and something to chase. My pick: yes.
9. **The whole world: every coach has a level, trees and abilities?** Example: two coordinator candidates read "level 41 Tactician, gold Third-Down Caller" and "level 33, silver Film Room, silver Living Room". In games: your hire changes third downs or recruiting, not everything at once. My pick: yes.
10. **Rust: an unused ability reads at half strength after two seasons, and nothing is ever lost or refunded?** Example: six NFL seasons leave your gold Living Room reading +1.5; your first 10 in-home visits back bring it to +3. In games: your first class back lags a little. My pick: yes.
11. **The start: level 1 for every road?** Example: a former NFL coordinator starts at level 1 with his road's ratings, Tactician open, two bronze abilities and two blind spots. The other way he starts at about level 25 with points already placed, and his first season pays 3 levels and not 8. In games: the same ratings either way. My pick: level 1.
12. **The dead ratings get work, the same rules at every level (kept from version 2)?** Example: a defensive head coach with scheme 17 and a coordinator at 13 plans the week at 15; your adaptation 16 with a coordinator at 10 plays the second half at 13; in a portal deal negotiating 18 reads a bluff right 69% of the time against 45% at 10. In games: your defense reads film better, your side adjusts faster after the half, you overpay less. My pick: yes.
