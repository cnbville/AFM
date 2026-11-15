# Live talks, rebuilt: everyone you talk to is a real person

Version 3, 2026-11-14. It replaces version 2 (PR #390) and version 1 (PR #387). Version 2's 11 questions are withdrawn and asked again in section 17, together with the new ones.

Everything not marked agreed is proposed, not confirmed. "Built" means the game has it on main today (main a46299d), with its file. "Designed" means another agreed design holds it but it isn't built. "New" means this design adds it.

## Where it stands

Cain's words, 2026-11-14 (UTC):

- 20:19: custom football promises ("if i punt in 40 yard distance it breaks my promise"; an EDGE promised "blitzes or stunts to stop him getting chipped") and "a real layer of football knowledge". These are his ideas.
- 20:22: "my intention was never to add an in game ai just so it can direct me to the right options that are alraedy on my screen ? my intention was always to have the player. I never intended it to be linked to the linebank"
- 20:24: "I want a architectural overhaul"
- 22:02: yes to no chip mapping (rule LT1 dropped) and to two calls a line (the ear reads, the game decides, the voice answers as him).
- 22:05: mapping your words onto moves and the nearest topic is "just as usefull as a linebank".
- 22:12: yes to asking getting real information from what he knows ("This has always been my fucking intent with this live feature"), yes to deals you build from real game actions ("good"), yes to asking people to do things ("good"), and "implement it in every facet, recruiting, player relationship, negotiating. Everything".
- 22:32: "You're proposal doesn't feel exhaustive enough". This version is the answer.

The decision log is `docs/handoff/decisions.md`, section 22.

Built so far: the shared contract (PR #402) and lane N1, people (2026-11-15): who each of the 17 kinds is and his makeup, what he knows and how he tells it (football from his snaps too), his wants, heat and patience, the stores and people coming to you (`world/folk/persona/`, how it works in `docs/reference/how-it-works/live-talks.md`, "Live talks v3, lane N1: people"). Every size N1 set is proposed, not confirmed.

## Contents

1. What it does
2. The one rule
3. A person: what he knows, tells, wants, can do, feels and remembers
4. You: your name, your record, your style, your time
5. The conversation: acts, hits, turns, channels, who's in the room
6. Every person you can talk to (17 kinds)
7. Every moment of the year
8. The catalogues: questions, terms, requests, measures
9. Football talk (Cain's idea): what each man knows and saw on the field, football with your staff, coaching in words, your football checked
10. Information: truth, lies, secrets, leaks, rumours
11. Leverage and negotiation
12. Rules on talking: contact periods, tampering, benefits, compliance
13. Without a key
14. Guards
15. Cost and time
16. What's kept, changed and replaced; the build
17. Questions

## 1. What it does

Every person you can talk to is played by the AI as himself, from his real state in the world. You talk in your own words. What you learn, what you agree and what you ask him to do become real in the world. The game decides all of it. Some of what that means:

- **Your players.** "Who's checked out in the room?" "Tre. His dad's been sick since October and he keeps talking about going home." That's true in the world, and he knows because they're close. A loyal friend won't tell you. A captain sees more of the room than a backup.
- **A transfer from your rival.** "What do they check to on 3rd and long?" He tells you what he knew there, as far as his spot and his years let him, and your staff checks it against the tape.
- **Recruits and their people.** "What did Georgia tell you?" comes from Georgia's real pitch, oversold if he wants leverage. "Who has the final say?" "My grandma. She wants me close to home." His high school coach knows every school that came through his door.
- **Deals you build.** "Stay out of the portal: the 3rd-down job, an offer to your brother, and you meet our collective." He weighs it against his real options and counters with his own asks.
- **Asks that do things.** "Take Malik under your wing" makes a mentor tie. "Talk to Tre for me" runs a real talk between them, and it can go badly.
- **Agents and GMs.** The agent answers from the real market and bluffs by his honesty. The GM tells you why he's shopping a guy, as far as he wants you to know. You offer a pick that becomes a 2nd if the player hits 60% of snaps.
- **Football (Cain's idea).** Talk football with anyone, at his level. "Why aren't you getting home?" "They slide to me every pass and keep the tight end in." That's what the play log shows. Promise him twists to free him up, and the game judges it on the stunt flag (section 9).
- **Your staff.** "Why do we keep punting at the 38?" gets your coordinator's real 4th-down habit. "Go for it on 4th and 3 past midfield" becomes a plan setting.
- **Your boss and the money.** The AD says what keeps your job. You can bargain your goals ("judge me on a bowl, not nine wins") and ask for staff money.
- **Game day.** Your speech hits each man on his own things. On the sideline you calm a hothead or chew out the official, and the official can flag you for it.
- **They come to you.** A player wants to talk snaps. An agent calls with the Jets' offer. A recruit's dad calls about playing time. Your OC tells you Miami called him.

## 2. The one rule

The AI reads your words and voices the other person. The game decides everything that matters: what's true, what he knows, what he's willing to say, whether he accepts, what it costs and what happens next. His words never change the world. Yours change it only through the game. (Cain's rule of 2026-11-07, kept.)

## 3. A person

Every person the world holds is built from six things. Each section says what's built and what's new.

### 3.1 What he knows

He knows the world's own facts, filtered by where he stands.

| What | Who knows it | Built store |
|---|---|---|
| His life events: a birth, a wedding, a parent ill or dying, a divorce, money trouble, homesick | Himself; his family; close friends (tie 35+); you when his trust is 0.55+ | Built: `world/folk/life.rs`, `players_week` |
| His mood and why | Himself; close friends see the mood, and the why by tie; a captain sees the room | Built: `world/folk/talks.rs`, `Reason` |
| His wants | Himself, as he'd say them; his decider; his agent | Built: `world/background.rs`, `own_wants`, `decider_wants`; the NFL's goals, `world/pro/market.rs`, `goals` |
| His portal thinking | Himself; close friends; his family | Built: `world/college.rs`, `lean_checkpoint` |
| His case and whether he did it | Himself (the truth); close friends (a rumour); everyone (the public step) | Built: `courts`, `Point::hid` |
| A rule break | Your staff if they saw it; teammates who were there | Built for the seen ones: `world/folk/rules.rs`. New: the unseen ones are kept, so teammates know |
| Effort and practice | His unit, his position coach, the strength coach | Built: practice battles and grades (`training.md`) |
| An injury as he feels it | Himself; the trainers know the diagnosis | Built: injuries; the medical read in steps is designed (`game-day.md`) |
| What schools told a recruit | The recruit, his decider, his high school coach | Built: `world/folk/case.rs` |
| A recruit's NIL offers | Himself, his decider, his agent | Built: `offer_for`, `portal_nil` |
| His grades | Himself, his teachers, his high school coach | Built: `ncaa_tier` |
| His character | His high school coach, teachers, trainer, teammates, each with his own eye | Built: `world/scouting/own.rs`, `sources`; `file.rs`, `source_says` |
| An opponent's habits | Its players and staff; your QC from the tape; a transfer or a cut player from there | Built: opponent film, play calling's tendencies. New: what a man who came from there knows |
| A pro's market | His agent; the GMs who made offers | Built: `world/pro/market.rs` |
| A club's needs, cap and grades | Its GM, its staff and scouts | Built: `world/pro/office.rs`, the cap |
| A scout's file | That scout | Built: `world/scouting/file.rs` |
| Jobs opening and who's leaving | Coaches in the network, agents, ADs | Built: the carousel (`staff-and-coaches.md`) |
| Your boss's goals and trust in you | Your boss | Built: `world/folk/boss.rs` |
| The budget and boosters' moods | The AD, the big boosters | Built: `world/finance/college.rs`; `takeover.rs`, `booster_push` |
| Rules and open cases | The compliance office, the league office | Built: `compliance.rs`; the rule books by era (`calendar/rules.rs`) |
| Who told whom what | Everyone who was in the talk | New: the grapevine log (10.5) |
| What he told you before, and whether it was true | Himself | New: his lies store (10.2) |

How sure he is follows his eye for it, the way sources already work: the truth, plus his misread, plus his shade (`world/scouting/file.rs`, `source_says`). Lane 6 built that for sources. It becomes everyone's rule. A teammate's eye is his Football IQ and his years with the man. A high school coach's eye is his recruiting rating. A GM's is his judgment.

Nobody in the world knows a player's true potential. People have opinions, and those are reads with their own eye and bias.

### 3.2 What he tells you

What he tells you depends on:

- his trust in you,
- his professionalism,
- his loyalty to the person it's about (a close friend's secret is safe with a loyal man),
- his own interest in the answer (a recruit oversells offers for leverage, an agent sells his client),
- who else is in the room (5.6).

He answers one of four ways: **straight**, **partly** (the safe half), **deflects** ("ask him yourself"), or **lies**. A lie goes in his lies store (10.2), so you can catch it later. When he holds something back, the voice gets "he's holding something back" and the tell your read caught (`went_words`, the talk's tells). It never gets the secret.

Asking costs something. Each question takes time. Grinding him on a teammate costs trust ("why do you keep asking me about Tre?"), more with loyalty.

### 3.3 What he wants

| Person | Wants | Built |
|---|---|---|
| A recruit | His decider's wants: playing time, NIL, distance, development, academics, the school | Built: `decider_wants` |
| A college player | His own wants, his role as he sees it, his mood's reason | Built: `own_wants`, P1's `self_view`, `Reason` |
| A pro | Money, security, winning, role, city, home, loyalty, greed, temper | Built: `market.rs`, `goals` |
| A parent or decider | What the family weighs (money, closeness, academics) | Built: `generate_family` |
| An agent | His client's money, his fee, his name | Partly built (honesty, agency size). New: fee and name as wants |
| A GM | His team's value, his cap, his job | Partly built (biases, judgment). New: job pressure |
| A coach on your staff | His career, his scheme, his pay, his family | Partly built (ambition, pay). New: play-calling, a title |
| A high school coach | His kids' futures, his program's name, a college job | New |
| Your boss | His goals for you, the budget, the program's name | Built: `boss.rs`, `draft_goals` |
| A booster | Winning and a say | Built: `takeover.rs`, `{want}` |
| A teammate | His own wants, plus his friends' good | New: friends' good |

### 3.4 What he can do

Every kind of person has a list of real actions (the full list is 8.3). Whether he does what you ask depends on his trust, loyalty, leadership, and what it costs or gains him. When he agrees, the game runs the action by its own rules, never the AI. When you ask your captain to talk to Tre, the game plays that talk between the two of them from their traits. He reports back in his own words, honestly or not.

### 3.5 What he feels during a talk

- His mood and trust move as the talk goes (built: the talk screen's bars).
- His stance on a hard topic: owns it, plays it down, denies, defiant, shaken, pushes back, sulks, fired up (built: `talk_points.rs`, `stance`).
- **Heat (new):** a talk can get hot. Insults, a lie he catches or a promise you broke raise it, by his temper. Past a point he goes quiet, raises his voice or walks out. The game decides, never the voice.
- **Patience (new for everyone):** an agent's patience is built. Everyone gets it. A long talk that goes nowhere wears it down.

### 3.6 What he remembers

- What you talked through and how it landed (built: `Folk.talked`).
- Every promise and term, with your exact words (built for promises in P1, new for terms).
- What he told you, what you told him, and what turned out true (new: his lies store, and yours, built in `claims/truth.rs`).
- How you treated him: a talk that went hot, a deal you lowballed, a request you made (new).
- **His colour notebook (new):** small details the voice adds that no system reads (his dog's name, the spot back home where he trains). Only the voice reads it, so he stays the same person. Question 5.

### 3.7 His voice

- His age, schooling and where he's from, his family's money, his personality as behaviour in three bands ("proud, needs to hear he matters", "hot-headed", "a pro"), his mood, his Football IQ and years in the game (section 9), how long he's known you.
- The setting changes how he talks: the living room with his grandmother there, the locker room, your office, a phone call.
- He swears when that man would, in that setting.
- Never caricature: his speech comes from his age, schooling, personality and football IQ, never from his race.
- The voice names only people and teams on its card. It never names a real person.
- Hard topics (a death in his family, an arrest) are voiced with care.

## 4. You

- **Your name with each group (built in parts):** the room, your recruiting trail name and the agents (P1 split `Mine.straight` three ways), your contacts' warmth (`Career.known`), your boss's trust (`boss.rs`), your public bar (`Mine.expectation`).
- **Your record speaks (built):** your wins, your draft picks by position (`claims::Alumni`), your players developed. People check what you claim against it, as the recruit talks already do (lane 10).
- **Your style (new):** what you do in talks builds how people see you. Blunt or soft, a screamer or calm, straight or a liar, a man who keeps his word or not. It fits some personalities and clashes with others: a coachable, competitive kid wants hard coaching, and a proud vet bristles at it. It reaches recruits through your players.
- **What you know (built):** your files, reads and notes. What people tell you goes into them.
- **Your time (built):** talk hours, 10 a week in season and 16 out of it (`talks.rs`, `week_budget`). Each talk costs its real length (`dialogue.rs`, `TYPED_CHARS_A_MINUTE`). Calls and texts follow the NCAA rules of the era (`calendar/contact.rs`).

## 5. The conversation

### 5.1 What the ear reads

The ear sorts your line into the things the game has to act on. That list is fixed because the game must do something with each one. What's inside each one is open.

| Act | What the game does with it |
|---|---|
| A question | Finds what he knows and what he'll say (3.1, 3.2) |
| A claim | Checks it against the world (built: the fact checks of lanes 3, 9 and 10) |
| An offer | Values the terms against his best option (section 11) |
| A promise | Logs it with its terms and decides his belief (P1, 8.4) |
| A request | Decides if he'll do it, then runs it (8.3) |
| Telling him a decision | Benched, starting Saturday, suspended, cut, traded, an offer pulled, a coach let go: the game runs the real action once you confirm (question 4), and he takes it by his traits |
| A threat or an ultimatum | Weighs it by your leverage and his temper ("start or sit, your call") |
| A deadline | Sets a real date on an offer ("this is good till Friday") |
| An apology | Repairs trust for a broken promise or a hot talk, by how real it is (you keep the next one) |
| Confiding | You tell him something private; he may keep it or pass it on (10.5) |
| Asking him to keep quiet | Lowers the odds he passes something on, by his loyalty |
| An invitation | A visit, dinner, a meeting with someone (calendar rules) |
| Bringing someone in | Your coordinator explains the scheme, a former player tells his story (5.6) |
| Showing something | Film, numbers, the facility, your alumni at his spot. Only real things can be shown |
| Answering his ask | Accept, counter or refuse his terms |
| Ending the talk | Ends it |

### 5.2 How your words land (the hits)

Separately, the ear reads your line with his card in front of it. It says which of his real things your words touched (a worry, a want, his pride, a promise you broke, a fact about him, a friend) and how: reassured, respected, stung, insulted, doubted, flattered, threatened. It must name a real item on his card each time.

The game sizes each hit from how much that item weighs for him and from his traits, inside today's caps. A sharp line about exactly his problem moves him. "Great job, keep working" touches nothing. Repeats fade (built: `Folk.talked`). Question 1.

### 5.3 The game's turn

The game resolves every act and hit. It decides what he says back (his answer, his counter, his own asks), whether the heat rises, and whether he ends the talk. Then the voice says it.

### 5.4 Turns and length

- He asks you things too, from his wants (agreed in LT2: "the family asks its own questions from what it weighs"). A recruit asks who's ahead of him at corner. The family asks about class sizes. A vet asks if you're staying.
- A question you dodge is a hit. He notices, and so does a sharp kid's mother.
- A talk runs until you end it, he ends it, or its time runs out.

### 5.5 Channels

| Channel | Who | What it allows | Built |
|---|---|---|---|
| In person: your office, his home, the facility, the field, the sideline | Anyone you can meet by the rules | Showing film, the facility, bringing people in | Built: sit-downs, home and official visits |
| A phone call | Anyone by the rules | About half a sit-down | Built: `calendar/contact.rs` (3 exchanges) |
| A text | Recruits and players by the era's rules; staff | Short | Built: messages (1 exchange) |
| A video call | Recruits in the era it's allowed; prospects | Film can be shown | New |
| Through someone else | His high school coach, his agent, his parent | You hear his answer secondhand, with their shade | New |
| A letter or an email | Your boss, staff, a compliance office | Formal | Built: inbox letters |
| A group | The team, a position room, the staff, the council, a family | Every listener hears every line | Partly built (rooms) |

### 5.6 Who's in the room

- Every person in the room hears every line and takes it his own way. Each gets his own hits.
- The decider matters most on a home visit. The kid can want one thing while his mother wants another.
- With his agent in the room, the player says less and the agent negotiates.
- You can ask someone to leave, or bring your coordinator, your position coach or a former player in. What they say comes from their own traits (a coordinator's teaching, a former player's story).
- A recruit with his high school coach in the room: the coach answers some questions for him.

## 6. Every person you can talk to

For each: what you talk about, what you can learn, deal, ask and promise, and what they come to you about. "Built" marks the hooks that exist today.

### 6.1 Your players (high school, college, NFL)

- **Moments:** camp, game weeks, after a game, after a benching or a lost job, an injury, trouble (an arrest, a rule break), grades and eligibility, a family crisis, the portal window, the draft decision (built: the December read), a contract year or extension, a holdout (built), a trade request (built: leverage x0.5), cut day, a release, retirement, a position change, a redshirt, a bowl opt-out (new), senior day, the exit meeting (built), captains (built: `settle_room`), the council (built), a players-only meeting (built: D13).
- **Learn:** his wants, his worries, how his body really feels (a tough or proud man hides a knock), his family, his plans (the portal, the draft), his teammates (moods, trouble, effort, who's leaving, who's been out past curfew), the room (who leads it), and his old team's habits if he came from an opponent.
- **Deal:** his role and usage terms, NIL and revenue share (built budget), his position, his jersey number, a council seat, a redshirt, time away (built), a tutor, a walk-on's scholarship (built), and at the NFL his contract (built: extensions, restructures, incentives), honouring a trade request, a release, a practice squad spot.
- **Ask him to:** mentor a young player, talk to a teammate, recruit a friend or a transfer, host a recruit, keep something quiet, change position, play special teams, make weight, go to class, tell you if something happens, report to voluntary workouts, take a pay cut, apologise to a teammate, speak to the team, play hurt (the medical system decides the risk, built: `opinion_answer`, `rushed_week`).
- **Promise:** anything in 8.4.
- **He comes to you about:** his snaps or role, a trade or his contract, his family, trouble he's in (a man may tell you before the police call does), a teammate, his position coach, another school's NIL whisper (tampering, section 12), the draft, the portal.

### 6.2 Their families

- Parents of your high school and college players, and the decider (built: `generate_family`). Parent calls are built (L9: a punishment, pay or position move).
- **Learn:** how he's really doing (homesick), home life, money, what other schools say to them.
- **Ask:** help with his grades, keep him close, talk him out of the portal.
- **They come to you about:** playing time, an injury, grades, NIL, "my son needs to transfer".

### 6.3 Recruits

- **Moments by the calendar (built: `calendar/mod.rs`, `recruiting_period`):** the first offer call, camps, junior days (new), unofficial visits (built: game days, drop-ins), official visits (built), home visits (built), calls and texts by era (built), the official visit's 45-minute sit-down (built: lane 6), the commitment call, decommit and flip talks (built: decommits), signing days, early enrolment (new), preferred walk-on offers (built: walk-ons).
- **Learn:** his top schools and why (built: `pulls.rs`), what each school told him (built: `case.rs`), his decider and what she weighs (built), his grades, his character (built: sources), his NIL offers (oversold), his visit plans, his friends who are also recruits (new: package deals are real).
- **Deal:** the offer, a NIL figure, role and usage terms, his position, a jersey number, early enrolment, his brother's offer, his high school coach on your staff (hiring is built), his visit.
- **Ask him to:** visit, commit, shut his recruitment down, bring a teammate on a visit, enrol early, go to summer school (built: it pushes eligibility).
- **Promise:** a start, a role, his position, an early chance, no redshirt (built: P1, lane 10), usage terms, NIL clauses, "I'll be here all four years" (judged by your job, new).
- **Lies both ways:** his inflated offers. Your claims are checked when he arrives (built: lane 10).
- **He comes to you about:** "Georgia offered, can you match?", "I'm taking an official to Bama", a decommit warning, "are you leaving?" (built: the hot seat claim).

### 6.4 The people around a recruit

- His high school coach (built: source, eye, warmth, credibility), his 7-on-7 coach (new), his trainer (built when the family pays for one), a teacher (built), his counselor (new), a handler (built as a decider: 6-12%), a street agent (new, and illegal benefits risk).
- **Learn:** his character, grades and family, which schools came through the door, who really decides, an injury he hides, his true times (built: recorded times).
- **Deal with the high school coach:** a clinic, camp invites for his other kids, a GA or analyst job, a look at his other players.
- **Ask:** steer the kid, open his door, tip you on kids (built: tips), set up a visit.
- **They come to you about:** a kid worth a look (built: the network's kid calls), a job for a nephew (built), a clinic talk (built).

### 6.5 Transfers and the portal

- **Rules:** you may only talk to him after he enters (real; section 12). Before that, contact through a third party is tampering.
- **Learn:** why he left (built: P1's history of broken promises, `burned`), his old team's habits (new), his NIL ask, his other schools.
- **Deal and promise:** as a recruit, plus a role from day one.
- **Ask:** bring a teammate from his old school.
- **Your own players:** retention talks (built: "Your word on his future", P1's stay-out-of-the-portal promise).

### 6.6 Agents

- Player agents at the NFL, NIL agents in college from 2021, coaches' agents (yours is built from your playing days; your staff's are new).
- **Learn:** what his client wants (built hidden goals; he reveals what helps), who else is interested (built: market offers; bluffs built), his client's mood. He hides health and off-field things.
- **Deal:** years, money, guarantees, signing bonus, incentives with terms, void years by era, the tag (built), an extension's timing, a restructure (built), working a trade.
- **Ask:** take an offer to his client, hold other teams off, get his client to report, tell you a number, steer another client your way.
- **He comes to you about:** another club's interest, a client who wants out, an extension, a holdout, a free agent in the tampering window (built: `backchannel.rs`, `c19_day`).
- Hardball has a price (agreed LT8): bring up his client's DUI and his client hears it.

### 6.7 Other clubs: GMs, coaches, scouts

- **GMs (built: trade talks, haggle, leverage, rumours, `office.rs`):** learn why he's buying or selling, what his staff thinks of a player (biases built), what others offered (a bluff). Deal: players, picks, conditional picks (new), cap room. Ask: a heads-up when someone comes free, "don't send him to our division". He comes to you with offers, "are you shopping him?", deadline calls and calls on the clock in the live draft (built: `world/scouting/live.rs`, `TradeOffer`; a talk is new).
- **Other head coaches:** the handshake after a game (new), job talk (built: the carousel), coaching-tree favours (built: the network), recommending or poaching a staff member (asking permission to interview an assistant, new).
- **Other clubs' scouts and pro scouts:** draft talk, smoke (11-07 decided against; question 10).

### 6.8 Your staff

- Coordinators (built: the staff meeting, LT17, plan settings), position coaches (built: "tell me if he's slipping", reads), the strength coach, trainers and doctors (built: second opinions), recruiting staff (built: the offer chain; overruling costs trust), scouts (built: trips, letters, your name on a player), the analyst (built: lane 4), QC (built: LT16), academic staff (new), GAs.
- **Learn:** their honest view, with their biases, what other jobs they hear about (built: the staff market), the mood of their room, the opponent.
- **Deal:** their contract (pay built; title, play-calling, a promotion promise, years and a buyout new), their staff budget, time off.
- **Ask:** scouting trips, a recruit to target or call, a plan setting, a player to work with (built: individual plans and who teaches), mentor a young coach, stay through the bowl.
- **They come to you about:** another job, being overruled, a player problem, a raise, a family matter (coaches' families are drawn, not stored; new if kept).
- **Hiring and firing:** interviews (built: LT18, C25), references (11-07 decided against; question 10), his contract (new), letting him go (built: asks first, `finals.rs` 70-72).

### 6.9 Your bosses

- High school: the principal and AD (built: interviews). College: the AD (built: boss meetings; the AD layer has its own design thread). NFL: the owner (built: job security) and your GM (built: every club's front office, `world/front_office/`): the draft board, free agents, who has the last word on the 53. As an assistant (built seats at college and the NFL): your head coach.
- **Learn:** your real job security (built: trust 0-1), the budget (built), what boosters are saying (built: `booster_push`), what he thinks of a coach, what's coming.
- **Deal:** your contract (an extension, a raise, a buyout), staff money (built: AD recruiting money), facilities (new), **the goals themselves** (built goals; bargaining them is new), roster control (new at the NFL).
- **Ask:** back you with the boosters, hire someone, give you time, keep the president off your back.
- **Promise:** goals as terms: a bowl, a graduation rate, no arrests, a class rank (built: `draft_goals`).
- **He comes to you about:** the goals meeting, the midseason check-in and the review (built), a warning letter after three losses (built), a compliance problem (built: NCAA letters), a booster complaint.

### 6.10 Boosters and the money

- Boosters (built: takeover talks, a big booster leaving), collectives (built: the NIL budget), donors (new).
- **Learn:** what he wants, what rival boosters are up to (built: rogue boosters before NIL).
- **Deal:** access (sideline passes, a seat in the room, new), a name on a building (new), a say in a hire (with its risk).
- **Ask:** fund the collective for a position group, fund a building, back off.
- **He comes to you about:** demands ("fire your OC"), complaints, offers, some of them outside the rules (built: `finals.rs`).

### 6.11 Prospects and the draft

- Combine interviews (built: LT11), top-30 visits, private workouts and pro days (built: your days out), all-star week (built), his agent, his college coaches (built: sources).
- **Learn:** his character, the night of his case (built: `Point::hid`), his medical as he tells it, his football IQ, where he'd like to play.
- **Deal:** the undrafted rush (built: guaranteed money). You sell your roster at his spot and the money.
- **The calls (new):** the draft-day call to the man you just picked, and the call to the one you passed on who thought you'd take him.
- **Your own players (college seat, new):** NFL scouts come through and ask about your players. You talk them up or down, and your word with each club moves the way a high school coach's credibility already does (built for the high school seat). Oversell a man who busts and that club trusts you less.
- **They come to you about:** "don't draft my client, he won't sign there" (new, real), declare or return for your own college players (built: the December read).
- 11-07's decided-against items here: the whiteboard, coached answers, a prospect remembering you, his family, the team doctor, security, smoke (question 10).

### 6.12 Game day

- Pregame and halftime speeches: hits for each listener (built sizes: fire, poise, focus in `game/talk.rs`), captains answer.
- The sideline (designed: `game-day.md`, topic 8): calm a hothead, a benched QB, an injured man who wants back in (the medical read in steps), the headset.
- Officials (designed: officials who make mistakes, tempers, flags for arguing): ask why, argue, and pay for it with a flag. You can't talk a call back, only challenge it (designed).
- The handshake with the other coach (new).
- The postgame locker room (built: the team talk reads the last game).

### 6.13 Compliance and the league

- The compliance office (built: letters), NCAA enforcement (built: penalties, show-cause), the league office (built: suspensions), the conference office (built: letters).
- **Learn:** what's allowed, in plain words.
- **Deal:** self-reporting a violation (real: it lowers the penalty, new).
- **Ask:** file a waiver, appeal a suspension (new).
- **An investigation (new):** an NCAA investigator interviews you and your staff. Lying to him is its own violation, as in real life.

### 6.14 The assistant seats (built at college and the NFL)

- Your head coach is your boss: what he wants, what he knows, what he'll give (play-calling, a title, a raise, a recommendation).
- Your area's recruits (built: assistants recruit areas), your position room, the other coaches.
- Head job interviews and search firms (built: job interviews D11, a search firm's call asks first).

### 6.15 The high school seat

- Players' parents (built), your principal and AD, the booster club (new).
- College recruiters calling about your kids: you talk them up or down, with your credibility on the line (built: credibility and "worth a look" calls; answering in words was decided against on 11-07, question 10).
- Kids you pitch to transfer in (built: `world/scouting/pitch.rs`, found out 18%+).
- The state association (built: transfer rules by state).

### 6.16 Your own career

- Your agent (built from your playing days): your next job and your contract.
- Search firms and job interviews (built).
- Your mentors and your coaching tree (built: contacts).
- Your own family (new, not stored today).

### 6.17 Media and fans

Out by Cain's rule of 2026-11-07 ("No press or media"). Press conferences are built (`folk/press.rs`) and already move real things (your public bar), so questions 8 and 9 ask about them and about social media and fans.

## 7. Every moment of the year

Built: the calendar's dates and contact rules (`calendar/anchors.rs`, `calendar/contact.rs`, `world/nfl_year.rs`, `world/hs_year.rs`). Talks follow it.

| When | High school | College | NFL |
|---|---|---|---|
| January | Recruiters visit your seniors | Portal window, the carousel, hiring, contact period, signees whose coach left | Black Monday, hiring, playoffs |
| February | Signing day for your seniors | National signing day, staff contracts | The combine and its interviews, tags |
| March | Transfers in (by state rules) | Spring practice, junior days | The tampering window, free agency, agents' calls |
| April | College coaches' spring looks | Spring portal window, spring game | Top-30 visits, the draft, the undrafted rush |
| May | Spring ball where allowed | Evaluation period | Rookie camp, OTAs (voluntary: who shows up) |
| June | 7-on-7, camps | Official visits, camps | Minicamp, extensions |
| July | Dead period talks with your own kids | Dead period, NIL deals, media days | Camp, holdouts |
| August | Camp, captains | Camp, captains, walk-ons onto scholarship | Cuts, the 53, practice squads, waivers |
| September-November | Friday nights, parents | Game weeks, evaluation, the midseason check-in, home visits on bye weeks | Game weeks, the trade deadline, IR |
| December | Playoffs, college calls | Early signing, the portal opens, bowls, opt-outs, draft decisions | The stretch run, contract talks |

## 8. The catalogues

### 8.1 Questions you can ask

Anything. What he answers is decided by 3.1 and 3.2. A question about something nobody in the world stores gets "I don't know", or his own opinion if he'd have one.

### 8.2 Terms you can offer

| Term | Who | Built today | How it's done or judged |
|---|---|---|---|
| A scholarship offer | Recruit, his brother, a walk-on | Built | Done at once |
| A NIL figure | Recruit, transfer, your player | Built figures; clauses new | The NIL model; clauses by 8.4 |
| Revenue share (from 2025) | Your player | Built budget (`revenue_cap`) | The NIL model |
| Role, a start, snaps, touches | Player, recruit | Built (P1, lane 10) | His games (P1) |
| Usage terms | Player, recruit | Proposed in the usage thread | The play log |
| His position | Player, recruit | Built promise kind | Where he lines up |
| No redshirt, an early chance | Recruit | Built | Games played |
| A jersey number | Player, recruit | New | Done at once |
| Early enrolment | Recruit | New | The calendar |
| Time away | Player | Built | Done |
| A tutor | Player | New | Academics |
| A staff job | A recruit's high school coach, a coach | Built hiring | Done when there's a spot |
| A council seat | Player | Built council | Done (captains stay voted) |
| An NFL contract: years, money, guarantees, bonus | Pro, agent | Built | The market model |
| Incentives with terms | Pro | Built incentives, counted as likely or not likely to be earned (`pro/money.rs`); terms new | 8.4 |
| An extension, a restructure, a pay cut | Pro | Built restructures | The cap rules |
| The tag | Pro | Built | The rules |
| A trade | Pro, GM | Built | The trade model |
| A release, an early release | Pro | Built releases | Done |
| A practice squad spot | Pro | Built | Done |
| Picks, conditional picks | GM | Built picks; conditions new | 8.4 |
| Cap room in a trade | GM | Built | The cap |
| A staff contract: pay, years, title, play-calling, a promotion | Coach | Built pay; the rest new | 8.4 (who calls the plays) |
| Your contract: an extension, a raise, a buyout | Boss | Built pay; bargaining new | His trust and the budget |
| Staff money | Boss | Built (AD recruiting money) | The budget |
| Facilities | Boss, booster | New | The budget |
| Your season's goals | Boss | Built goals; bargaining new | The season |
| Access and a say | Booster | New | Done |
| A clinic, camp invites, a look at his kids | High school coach | Built network favours | Done |
| Your promise to stay | Anyone | New | Your job |
| Anything outside the rules | Recruit, booster, handler | Built (`finals.rs`) | Compliance risk |

### 8.3 Requests

| Ask | Who | Built action | New |
|---|---|---|---|
| Mentor a young player | A veteran | Mentor ties (`ties.rs`) | Set by your ask |
| Talk to a teammate | A player, a captain | One-on-ones (`talks.rs`) | A talk the game runs between them |
| Recruit a friend, bring him on a visit | A player, a commit | A school's case (`case.rs`) | A teammate's point in the case |
| Host a recruit | A player | Visits | His effect on the visit |
| Keep something quiet | Anyone | | The odds he passes it on |
| Change position, play special teams | A player | Position moves, roles | |
| Make weight, go to class | A player | Weight rule, academics | |
| Tell you when something happens | A player, a coach | "Tell me if he's slipping" | Anyone |
| Report to workouts or camp | A pro | Holdouts | |
| Take a pay cut, restructure | A pro | Contracts | |
| Play hurt | A player | Second opinions, `rushed_week` | His trust and the risk |
| Commit, shut it down, enrol early, summer school | A recruit | Commit timing, summer school | |
| Steer a kid, open the door, send a kid | A high school coach | Credibility, tips | |
| Take an offer, hold off, give a number | An agent | The market | |
| A heads-up, not in our division | A GM | Trades | |
| A trip, a file, a question to find out | A scout | Days out, letters | Sending a scout with a question (question 10) |
| A plan setting | A coordinator | The game plan | |
| Target or call a recruit | An assistant | The offer chain | |
| Work with a player | A position coach | Individual plans | |
| A second opinion, clearing a player | A doctor | Second opinions | The team doctor (question 10) |
| Money, time, a hire | Your boss | AD money | |
| Back you with the boosters | Your boss | `booster_push` | |
| Fund the collective or a building | A booster | The NIL budget | Buildings |
| Self-report, a rules question, a waiver, an appeal | Compliance, the league | Letters | All new |
| Calm a teammate | A captain | Designed (game day tempers) | |

### 8.4 Measures for promises and terms

Cain's idea, made general. A term is a measure, a number from your words, a scope (a game, a season, by a date), a horizon and "as long as". The same terms serve promises, incentives, NIL clauses, conditional picks and promises to coaches and bosses.

**Agreed 2026-11-14 in the promises thread:** the 9 football promise items, "as an baseline for every position with their own promises, don't treat it like a kicker or edge fix" (Cain, 23:08 UTC). The agreed 9: the kicker's field goal line and its plan setting; stunts for a pass rusher (EDGE or DT, at least 1 in 4 of his rushes, "free him up"), judged on the stunts he's in and never on chips; a blitzer (LB, S, nickel) rushing on at least 1 in 4 dropbacks ("send him"); touches with your number; deep shots ("take shots to him"); belief in a number past what he thinks is real; and anything off the list stays a talking point. The rows below are the reading of that answer for every position: proposed, not confirmed.

#### The rules every term follows

- **Judged on the play log,** game by game, kept or broken by its date, with P1's costs scaled by how far he believed it.
- **Only what you control:** your calls, where you line him up, his assignment, his touches. Never the other team's calls (chips, doubles, their coverage) or luck (catches, drops, sacks). Those are saved and shown on his page, but nothing is judged on them.
- **A plan setting keeps it.** Each term has a setting on the game plan (`GameTeam.prep`, `game/roster.rs`, keyed by player). Your staff keeps it in every game your team plays, simmed or live. Your staff turns it on from your open promises, and you can turn one off. AI teams never set them, so AI games don't change. Before each game your staff warns you when the plan won't keep one.
- **Belief:** a number past what he thinks is real at his level is believed less by a sharp kid and more by a big ego (agreed item 8, through P1's likely and shape parts). For a kicker, what's real is his range from his Kick Power. Ego stretches it and Football IQ tightens it.
- **Recruits** get the same terms, checked from his first games once he signs.
- **High school** keeps box scores, not snaps (`world-log.md`), so high school terms are per-game counts: carries, targets, attempts, kicks and returns. New: `usage.rs` judges plays only, so a high school game says nothing today. N2 builds the box score judge.
- **Off the list** stays a talking point, and "Heard as" says the game can't hold you to it (agreed item 9).

#### What every position can be promised

All recorded on every college and NFL snap (`on_field`, `roles`, `down`, `yardline_100`, `clock`, `dpers` in `PlayRecord`):

- starts, snaps and snap share;
- snaps by situation: 3rd down, the red zone, the goal line, the two-minute drill;
- snaps by package: base, nickel, dime, goal line (`dpers`), or the offense's personnel (11, 12, 21 in the call);
- the spot he lines up at (`roles`: X, Z, slot, TE, LT to RT, and LE, RE, DT, NT, Sam, Mike, Will, the corners, nickel, dime, FS, SS);
- ahead of a named teammate (built: P1);
- not pulled after a mistake: his snaps after his turnover or bust (new reader);
- on or off special teams (`on_field` on kicks).

#### By position

"Every snap" means the data is kept on every college and NFL snap. "Your games" means only your own games keep it (the line chart, lane 9). "New" means nothing keeps it yet.

| Position | What you can promise | Judged on | Plan setting | Kept |
|---|---|---|---|---|
| QB | The job, and not pulled after a bad series | His snaps, and his snaps after a turnover | | Every snap |
| QB | Freedom at the line | The freedom you give him: none, a kill call or full audibles (built: plays and tactics 3.3), and his audibles in the call | Full audibles | Every snap |
| QB | Throws a game, play action, tempo, gun or under center | His attempts, `play_action`, `hurry`, the backfield in the call | Pass rate, play action, tempo, the backfield | Every snap |
| QB | Use his legs, or no designed runs | Runs with him the carrier (not scrambles) | Designed runs on or off | Every snap |
| QB | Protect him | Men kept in to block, a back in | Max protect, a back in | Your games |
| RB, every down | Carries a game, the goal-line carries | His carries, his carries inside the 5 | Feature him, the goal-line back | Every snap |
| RB, every down | The lead back, the run game built on him | His share of backfield snaps, the run family of his carries (inside zone, outside zone, gap, option) | Run family weights | Every snap |
| RB, every down | A fullback in front of him | Snaps with a fullback beside him (`roles`) | 21 personnel | Every snap |
| RB, 3rd down | The 3rd-down back | His 3rd-down snaps | 3rd-down back | Every snap |
| RB, 3rd down | Targets, routes not pass pro, screens | Targets; let out on pass plays (on the field, not a pass blocker in `blocks`); screens to him | Release him, screens to him | Every snap |
| X and Z | Outside, the X or the Z | His spot | Keep him outside | Every snap |
| X and Z | Targets, deep shots, red zone targets | Targets; targets 20+ air yards (agreed item 7); targets inside the 20 | Feature him, take shots to him, red zone target | Every snap |
| X and Z | Touches on the move (jet sweeps, reverses) | His carries | Gadget calls for him | Every snap |
| Slot | Our slot, inside or moved around | His spot (slot, second slot) | Keep him inside, move him | Every snap |
| Slot | Targets, 3rd-down targets, the quick game | Targets, targets on 3rd down, quick and RPO calls to him | 3rd-down target | Every snap |
| TE | Let out, not kept in | Pass plays he's on and not a pass blocker | Release him | Every snap |
| TE | Targets, red zone targets | Targets, targets inside the 20 | Feature him | Every snap |
| TE | Flexed out, the Y in 12 personnel | His spot (slot or wide as a TE), 12 and 13 personnel | Flex him, 12 personnel | Every snap |
| C | Our center, not moved to guard | His spot | | Every snap |
| C | Line calls | Not modeled: a talking point | | New |
| Guards | Left or right guard, not moved to tackle | His spot | | Every snap |
| Guards | A gap team (power, counter) | The run family you call | Run family weights | Every snap |
| Guards | Pulling | Not kept per man | | New |
| Guards and tackles | Help on their best man | Pass reps with a helper on his man (`DOUBLE`), the slide to his side, chips on his man (the promises build's new chipped flag) | Help him (slide, double, chip) | Your games |
| Tackles | The blind side, not moved inside | His spot (LT or RT) | Keep him at left tackle | Every snap |
| EDGE | Stunts to free him up (agreed item 3) | Stunts he's in, at least 1 in 4 of his rushes (`STUNT`) | Free him up (agreed) | Your games |
| EDGE | Rush, not drop | Pass plays he rushed against the ones he dropped | Rush him every down | Your games |
| EDGE | The right side, or both sides | His spot (LE or RE) | Keep him on one side | Every snap |
| EDGE | The 3rd-down rusher, every-down role | His snaps by down | Pass-rush package | Every snap |
| DT | The 3-technique, not the nose | His spot (DT or NT) | Keep him at the 3-technique | Every snap |
| DT | A one-gap front, get upfield | The front your system runs (built: `world/systems.rs`) | | Every snap |
| DT | Stunts, and staying on for passing downs | Stunts he's in (agreed item 3), his 3rd-down snaps | Free him up, pass-rush package | Your games |
| Off-ball LB | Blitzes (agreed item 5) | His rushes, at least 1 in 4 dropbacks | Send him (agreed) | Your games |
| Off-ball LB | Mike, Will or Sam | His spot | | Every snap |
| Off-ball LB | On the field in nickel and dime | His snaps in nickel and dime | Keep him on | Every snap |
| Off-ball LB | Spy the QB | QB spy calls (in the call); who spies isn't kept | Spy with him | New |
| Outside CB | Man coverage | Man snaps he's on (`coverage_man`) | More man | Every snap |
| Outside CB | Boundary or field side, no slot duty | His side with the hash, his spot | Keep him outside | Every snap |
| Outside CB | Help over the top | The coverage in the call (two-high, Cover 2, 4, 6) | Two-high to his side | Every snap |
| Outside CB | Press, or shadowing their best receiver | Not kept per snap | | New |
| Nickel | Our nickel, outside in base | His spot by package (nickel, dime, corner in base) | Nickel every pass down | Every snap |
| Nickel | Blitzes off the slot (agreed item 5) | His rushes on dropbacks | Send him (agreed) | Your games |
| FS and SS | The post safety, the robber, the box | His spot with the coverage in the call (single-high, Cover 1 robber) | Single-high with him deep, rob with him | Every snap |
| FS and SS | Blitzes (agreed item 5) | His rushes on dropbacks | Send him (agreed) | Your games |
| FS and SS | Covering tight ends | Who covers whom isn't kept, only the man on a target | | New |
| K | The field goal line (agreed items 1 and 2) | The try is the yard line plus 17. Any punt from inside the line breaks it. Going for it doesn't count | The field goal line | Every snap |
| K | The long tries, kickoffs, the only kicker | His tries by distance, kickoffs, other kickers used | | Every snap |
| P | Our punter, the kicker won't punt | Punts by him | | Every snap |
| P | Holding kicks | Not kept | | New |
| LS | Our snapper | On the field for every kick | | Every snap |
| Returners | Returns, the green light from deep, fair catches | Who returns; the kneel line and the fair catch rule (built: `Instruction`) | He returns, the kneel line | Every snap |
| Coverage units | The punt team, the kickoff team | On the field on kicks | | Every snap |
| Coverage units | Gunner | His spot on a kick isn't kept (`roles` is empty on kicks) | | New |

Outcomes stay off the list on purpose: catches, yards, sacks, picks, made kicks, the chips and doubles the other team calls. A man may still care about them, and he says so, but your word was never about them.

#### What the usage lane builds, and what's left for N2

The promises thread checked this table row by row against `world/folk/usage.rs` as it merges (2026-11-15). A term is `usage::Usage`: a measure with its flat key, a number, share or count, at least or at most, and slices (spot, down, inside a yard line, far, package, look). N2 and the other lanes use it as it is, with no second term type.

- **A measure already:** starts and snaps; snaps by down, the red zone, the goal line and the two-minute drill; by package; his spot; not pulled; special teams; the QB's throws with play action, tempo, gun or under center; designed runs; protection; carries and goal-line carries; routes; screens; targets, deep and red zone targets; a receiver's carries; help; rushes; stunts; blitzes; man; field goals by distance; the field goal line; punts; kickoffs; returns.
- **N2's readers, built 2026-11-15 (lane N2):** the QB's freedom at the line (`kill_call`, `audibles`; the snap keeps it in `PlayRecord::freedom`); the run family (`inside_zone`, `outside_zone`, `gap_scheme`, `option`; backs and linemen); quick and RPO calls to the slot (`quick_targets`); a corner's boundary or field side from the hash (`boundary`, `field_side`); help over the top (`help_over_top`, two-high), the post safety and the robber (`post_safety`, `robber`); other kickers used (`only_kicker`); and the spy (`spy`). Sizes proposed, not confirmed.
- **Not usage terms:** the returner's kneel line is an `Instruction`, and the DT's one-gap front is the system (`world/systems.rs`). Both are kept as promises through those, not through `usage.rs`.
- **Split (2026-11-15, the usage lane):** every role a snap keeps has its own spot key: `tailback`, `fullback`, `left_guard`, `right_guard`, `left_dt`, `right_dt`, `sam`, `mike`, `will`, `left_corner`, `right_corner`, `nickel` (the nickel alone now) and `dime`, beside the ones already split (`x`, `z`, `left_tackle`, `right_tackle`, `center`, `left_end`, `right_end`, `nose`, `free`, `strong`). The union keys stay for a promise that names a group: `wide`, `slot`, `inline`, `backfield`, `guard`, `end`, `tackle`, `outside_lb` (Sam or Will), `corner` (left or right) and `inside_corner` (the nickel or the dime). The slots and tight ends are numbered by order on the field, not by spot, so `slot` and `inline` stay one key each. The boundary or field side stays on N2's list.
- **High school (built, N2):** a game kept as a box score only judges the counts its stat line holds (carries, targets, touches, throws, kicks, returns, special teams snaps, and snaps as a share of the most any teammate played; `Usage::box_count`). A term that needs the plays (a spot, a down, the red zone, stunts) says nothing for that game.
- **Plan settings built:** the field goal line, free him up, send him, take shots, feature him, help him, rush him, more man (the usage lane); and in N2 his freedom at the line, the run family, designed QB runs (more or none), letting a back or tight end out, play action, pass lean, gun or under center, the package, screens, quick and RPO, the jet sweep, max protect, the corner's side, the post safety and the robber, the shell floors (two-high, single-high, robber), his snaps on every series and the returner first. Turning one off (`usage::set_off`, `people_promise_plan`) and the staff's warning the day before a game (`usage::warnings`) are built too. **Not built:** tempo, a flexed tight end, the spy on a named man and pulling a tackle stay talking points. Every N2 size is proposed, not confirmed.
- **Season terms (built, N2):** incentives, NIL clauses, conditional picks and staff and boss goals take the same `Usage` with `season` set, judged on the year's record by `world/folk/terms.rs` (stat lines, starts, games, a snap share, honors, the team's wins, playoffs, title and rank). N5b and N5c write deals on them. Proposed, not confirmed.

#### Off the field and the other terms

| Measure | Recorded where | Built |
|---|---|---|
| His practice group | P1's `seen.rs` | Built |
| Roster moves at his spot | Transactions | New reader |
| Staff hires, the scheme | The staff, the game plan | New reader |
| NIL paid | NIL | Built (lane 10 checks) |
| Contract terms | Contracts | Built |
| Time away | Talks | Built |
| Your job (you stay) | The carousel | New reader |
| Wins, a bowl, graduation, arrests | The world | Built (boss goals) |
| Individual work with a coach | `training/seat.rs`, `focus_minutes` | When individual practice is built |

## 9. Football talk (Cain's idea)

Cain, 20:19: "I'm missing a real layer of football knowledge right now". Any talk can be about football, and what's said is the football the game really plays. The engine already keeps it. Every college and NFL snap has the call and the defense's call, the personnel and the defense's package, the box, the blitz, man or zone, the route, every block (who on whom, won or lost, pressure given up), every rush win and its move, busts, separation, time to throw, air yards and the caller's reason (built: `game/stats.rs`, `PlayRecord`; kept in the world log, `wlog/games.rs`). Your own games' snaps also keep the line chart: the protection, how many stayed in to block, and who was in a stunt (built: lane 9, `game/chart.rs`). High school games keep box scores and drives, not snaps.

### 9.1 What he knows about football

- His Football IQ rating, his years at the level, and the systems he's played in: the offense, the front and the coverage family (built: `world/systems.rs`), and how much of the playbook he's learned (built: reps by family, `world/season.rs`).
- A freshman knows his own job. A 10-year vet knows the whole call and why it was called. A center knows where the slide is going. A QB knows his checks.
- The voice talks football at his level. A sophomore with a low Football IQ asks what "match quarters" means. A sharp vet notices when you get it wrong.

### 9.2 What he saw on the field

His complaints, asks and answers come from his own snaps:

| Position | What he talks about | In the play log (built) |
|---|---|---|
| QB | Protection, his reads and checks, the hits he takes, the calls | The protection and men kept in, time to throw, pressures, hits and sacks, the target, air yards, the call and its reason |
| RB, every down | Carries, zone or gap, the box | Carries, the box count, yards before contact, missed tackles, the call |
| RB, 3rd down | Pass pro, targets on 3rd down | His blocks on rushers (won, pressure given up), targets by down |
| X and Z | Targets, deep shots, press, his routes | Targets, the route, air yards, separation, man or zone, the man on him |
| Slot | Option routes, who's on him (a nickel or a safety) | The same, with the defender on him |
| TE | Blocking or routes, kept in or let out | His blocks, the route, targets, men kept in |
| Center | The protection, the nose, the A gaps | His blocks on the nose, the gap each rusher came through, busts |
| Guards | The 3-technique, twists coming at him, help | His blocks (won, how long, pressure given up), the stunt flag |
| Tackles | The EDGE across from him, help from the slide or a back | His blocks, the slide's side |
| EDGE | His wins by move (speed rush, bull rush, spin move), doubles, the slide toward him, twists and blitzes | His rush wins and their moves, pressures, sacks, the blocks on him, the slide, the stunt flag |
| DT | Doubles, his gap, pressure up the middle | The blocks on him (two blockers is a double), his gap, pressures, tackles |
| Off-ball LB | Blitzes, his drops, his run reads | The blitz, the target he covered, tackles, busts, freelancing |
| CB | Press or off, man or zone, help over the top | Man or zone, the target he covered, pass breakups, picks, separation on his man |
| S | One high or two, rotation, the box | The target he covered, the box count, tackles, missed tackles |
| K | Range, the weather, the hash, the snap and hold | Distance, made or missed, the spot and the hash, blocks, the rush on the kick; the game's weather (`calendar/weather.rs`) |
| P | Distance, inside the 20, the coverage | Distance, inside the 20, touchbacks, return yards |
| Returners | Returns, fair catches, the kneel line | Returns and yards, fair catches, touchbacks, the kneel instruction |

So "I keep getting doubled" comes only when he is. "Their right tackle couldn't handle my speed rush" comes when the log has him beating that man with it. "I'm open and he doesn't look at me" comes when his separation is high and his targets are low. A guard who gave up three pressures to the nose knows it, and a proud one won't say so. The protection, the men kept in, the rushers' gaps and the stunt flag come from the line chart, so they're there for your own games. For a man from another team, his blocks and rush wins are there and the line chart isn't. Chips are being added to the line chart by the promises build (a chipped flag, your games only).

### 9.3 Football with your staff

- "What beats their Cover 2?" comes from the coaches' what-beats-what table (built: `data/call_values.csv`), seen through your coordinator's reading and situational skill (built: `game/roster.rs`, `Caller`). A weak coordinator answers wrong and sounds sure.
- "Why do we keep punting at the 38?" comes from his aggressiveness (a style: deep shots, blitzes, 4th downs) and his situational skill. "Go for it on 4th and 3 past midfield" becomes a plan setting.
- "What do they do on 3rd and long?" comes from the opponent's tape and your QC's tags (built: `world/tape.rs`).
- Your analyst answers from the stats book: expected completions, rush yards and YAC over expected, pass rate over expected, the 4th down and win expectancy tables (built: `game::stats::XCOMP`, `world/tally/expect`, `world/tally/decide`).
- Your position coach tells you what a player is doing wrong, as far as his eye sees it (built: reads; flaws and tools when individual practice is built).

### 9.4 Coaching in words

- An instruction you give becomes the built one, and he keeps it by his discipline (built: `tendencies.rs`, `Instruction`): kneel deeper than so many yards, fair catch inside the 10, slide, take shots, protect the ball, high and tight, keep contain, no snap guessing, wrap up, stay home. "Don't chase their QB, keep contain" is the contain instruction.
- "Watch more film" becomes home study (built: `training/plan.rs`, TR35).
- A technique point ("get your hands inside") becomes a focus point when individual practice is built (`docs/design/individual-practice.md`).
- Anything the game doesn't model stays talk, and "Heard as" says the game can't hold you to it (agreed item 9).

### 9.5 Your football gets checked

- What you say about your scheme is a fact, checked against your play log and your systems (new readers on built data): "we throw to the slot more than anyone", "our nickel plays most snaps", "we're a 3-4 team".
- A recruit's high school coach, a vet with a high Football IQ and an agent each check it by his own eye. A claim found false costs what a lie costs (agreed 2026-11-07).
- "How would you use me?" is answered from your systems and what your play log did with his position last year. You can show him the film (5.1).

### 9.6 Football in promises and deals

- The ear knows the game's football: the playbook's families, the coverages, pressures, stunts and spies of a defensive call (built: `game/playbook.rs`, `DefCall`), protections (`game/chart.rs`), every spot a man lines up at (`roles`), routes, personnel and the three rush moves. So "we'll run twists to free you up" is a term on the stunt flag, and "you'll get help on their best rusher" is one on the slide, the doubles and the chips. Every position's terms are in 8.4.
- A sharp player believes a football promise only as far as your football can keep it (new, on P1's belief): twists for an EDGE when your front never runs them, deep shots for a receiver when your QB can't throw deep, carries for a feature back in a pass-first system.
- Your staff warns you before the game when the plan won't keep a term (8.4).
- Agents and GMs argue with the stats book, each by his own judgment: sacks and pressures, separation, yards after contact, the expected numbers. A greedy agent picks the numbers that sell.

### 9.7 What other teams know

- Your call words already travel with men who leave (built: `world/plans.rs`, `leak`: each one on the other side reads about one call in nine, at most three in ten).
- New: the same rule the other way. A man who came from another team reads their calls for you by the same sizes. In words he can tell you their checks and their 3rd-down habits, by his Football IQ, his years there and his loyalty to his old coaches. Your staff checks it against their tape.

## 10. Information

### 10.1 Truth

What a person says is the truth, plus his misread, plus his shade (3.1), filtered by what he's willing to tell (3.2).

### 10.2 Lies both ways

- **His:** a recruit's inflated offer, an agent's bluff (built), a player covering for a friend, a prospect hiding his case, a GM's smoke. Each goes in his lies store with the day you could find out.
- **Yours:** built (`claims/truth.rs`, `Live`). A lie found out costs what a broken promise costs (agreed 2026-11-07).
- **Catching them:** another source, the facts, the tape, your read of his tells. Some are never found, as in real life.

### 10.3 Secrets and off the record

- He can tell you something "between us". If you use it in a way he can see, it costs his trust.
- You can confide in him. He keeps it by his loyalty and his trust in you.

### 10.4 Your read of him

Your own read judges (Cain's rule: his own read always judges, the staff's read is the fallback). How sharp it is: built (`sharpness`: relationships and how long you've known him).

### 10.5 The grapevine

- What you say travels through ties: close friends, family, agents, a recruit's high school coach. It goes by the listener's loyalty, temper and professionalism.
- Promise two kickers the 50 and both find out. Call a teammate soft and he hears it.
- Your staff flags a clash with an open promise as you say it, and the clash lowers belief.
- Your word reaches recruits through your players (built: the trail name, P1).

### 10.6 Rumours

Built: a failed trade talk leaks as a shopping rumour (`trades`, 30%). New: talks leak too, by the same listener rule. A player who hears he's on the block, true or not, takes it as he would.

## 11. Leverage and negotiation

- **His best option:** another school's case, another club's offer, staying put, the portal, free agency. A deal is valued against it (built for the NFL market, trades and recruiting).
- **Your best option (agreed LT7):** a good backup makes walking away believable. The game checks it.
- **Deadlines:** real dates (signing day, the tag deadline, the trade deadline, cutdown) and your own ("good till Friday"). He weighs a deadline by his patience and competitiveness.
- **Anchors:** your first number moves what he expects, a little. Size proposed.
- **Walking away:** he can, you can. A walk-out isn't always the end.
- **Taking time:** "Let me talk to my mom." His decider and his agent get their say, and that takes days.
- **Many people:** the player, his agent and his family can want different things.
- **Verbal and signed:** a verbal commitment can be broken (built: decommits). A signature binds by the era's rules.
- **His asks and counters:** built by the game from what he wants most that you haven't given, sized to close the gap. The AI only says them. Question 3.
- **Lowballing:** an offer far under his worth costs warmth. Asking again after a no costs patience.

## 12. Rules on talking

- **NCAA contact rules by era (built):** contact, evaluation, quiet and dead periods; calls and texts by era; official visit limits.
- **Tampering (new as a talk):** talking to another school's player before he enters the portal, or another club's player or coach under contract. The other side can find out through the player, his coach or his friends, and the real penalty follows. The NFL's tampering window is built (`backchannel.rs`).
- **Benefits outside the rules (built):** the deals outside the rules in `finals.rs`, rogue boosters before NIL, NCAA penalties (`compliance.rs`). An offer typed in your own words that breaks the rules is read as one and carries the same risk.
- **High school undue influence (built):** pitching a kid to transfer is found out 18% and up (`pitch.rs`).
- **Lying to an investigator (new):** its own violation.
- **The CBA (built):** contracts, tags, restructures and practice rules by era.

## 13. Without a key

- Proposed: the same questions, deals and requests work through pick lists. Pick a person, then a question topic, terms or a request from lists. The game logic is the same, so a save made with the AI plays without it. Question 6.
- The old chips and line bank keep the words.
- With a key there are no answer buttons. An "Ideas" list shows what's on his mind and what you could offer, and clicking one puts words in your box to change. Question 7.

## 14. Guards

- **Outcomes:** the game decides from attributes, traits and the world's facts. The AI judges which of his real things your words touched. The game decides how much each counts, inside today's caps: morale moves growth at most ±15%, a portal lean x0.85 to x1.6, a home visit a school's pull at most +0.03, a pregame talk at most ±1.5 points for that game.
- **No invented facts:** every hit names a card item. Every answer is what the game released. When he lies, the voice gets the lie, not the truth.
- **Hidden data:** one whitelist card type (built: `live_talk/card.rs`), with no field for potential, ratings, hidden numbers or the depth chart. The reply filter stays (built: `filter.rs`).
- **Exploits:**
  - Your words go in tagged as data. A line that isn't a person talking does nothing.
  - Grinding for information costs time and trust.
  - Promise spam: belief falls with every open promise, and clashes are flagged.
  - Flattery spam fades.
  - Lie spam: the lies store and your name.
  - Deal spam: lowballing costs warmth.
  - "What's my potential?": he gives his own opinion of himself (ego and public reads). Nothing leaks.
- **Fairness:** AI teams never call the API. Question 11 gives them the same deals and asks through the game's own logic.
- **Determinism:** the reading is saved with the talk, so a reload plays the same world.
- **Failures:** no key, no network, a timeout or the cap: the talk goes on with pick lists or chips (built: `live_talk/say/mod.rs` fallbacks).
- **The cap:** $50 a month (built).

## 15. Cost and time

Claude Haiku 5.5: $0.10 in, $0.01 a cache hit, $0.50 out, per million tokens (`proposal.md`, "Model and cost"). Writing the cache costs about 1.25x the input price. Token counts are Claude's estimates.

| Call | Cached | New in | Out | Cost a line |
|---|---|---|---|---|
| Ear | about 3,000 (rules, schema, measures) | about 1,200 (his card items, the talk so far, your line) | about 300 | about $0.0003 |
| Voice, first line | about 4,000 written (rules, his card) | about 800 | about 150 | about $0.0007 |
| Voice, later lines | about 4,000 read | about 800 | about 150 | about $0.0002 |

A heavy season, every facet in:

| Facet | Talks | Lines | Cost |
|---|---|---|---|
| Your players and rooms | 300 | 1,800 | $1.10 |
| Recruiting (recruits, families, high school coaches, transfers) | 500 | 2,500 | $1.50 |
| Negotiating (agents, GMs, NIL) | 100 | 800 | $0.50 |
| Staff and scouts | 200 | 1,000 | $0.60 |
| Bosses, boosters, compliance | 30 | 180 | $0.10 |
| Game day | 100 | 200 | $0.10 |
| Prospects and sources | 150 | 900 | $0.55 |
| Press, if question 8 is yes | 40 | 200 | $0.10 |
| **All** | **1,420** | **7,580** | **about $4.50** |

A typical season is about half that. The $50 cap stays. A Sonnet 5.5 ear would cost about $0.005 a line, about $38 a heavy season, which is why Haiku is the pick. Time: Haiku answered a small call in 0.8 s. Ear then voice is about 2 to 2.5 s a line, and "Heard as" shows after about 1 s.

## 16. What's kept, changed and replaced; the build

### 16.1 Parts built so far

| Part | Fate |
|---|---|
| Lane 1, the pipe | Kept. Adds a cache mark for the voice's card. |
| Lane 2, the boxes | The talk box is replaced by the conversation screen. The settings card stays. |
| Lane 3, the core | Kept: tickets, the whitelist card, the filter, ask-first picks, the lies store, fact checks. Replaced: landing a line on one option (`scene::say`), the writer that rewrote a bank line, LT1. |
| Lane 4, the desk | Kept as what your analyst and scouts know. They become people you talk to and give jobs. |
| Lane 5, rooms | The speech reading becomes hits per listener (`game/talk.rs` sizes stay). Parent calls and discipline become conversations. |
| Lane 6, rooms that gain | `source_says` becomes everyone's rule for what they know. Interviews, sources and the staff meeting become conversations. |
| Lane 7 (not built) | Not built on the old brief. It becomes the negotiating lane. |
| Lane 8, the off-field price | Kept. |
| The "Also heard" fix (#388, merged 2026-11-14) | A patch on the old reading. The ear replaces it, since it reads every specific in your line. Kept: a promise counts even without the word "promise", and the reply filter checks the voice against your own words. |
| Lane 9 (#365) | Kept: roster fact checks, a false fact's cost, promise logging. Replaced: the turn card and its writer. |
| Lane 10 (#381) | Kept: recruit fact checks, lies found at signing and arrival, NIL checks. Replaced: the mapping onto pitch points. |
| Lanes 11 and 12 (not built) | Not touched. Lane 11's AI staff overlaps with staff you give jobs to, so check before it opens. |
| Talk facts L1-L13 | Kept. The sheet is what he knows about himself. |
| Promises P1 and P2 | Kept. Every term is judged through P1. |
| The usage promises thread | Its terms are the first measures. |
| The line bank and chips | Kept for play without a key. |

### 16.2 The agreed items LT1-LT21 (2026-11-07)

- LT1: replaced (agreed 2026-11-14).
- LT2-LT12, LT17-LT19: kept, and they become facets of this design (6.1-6.13).
- LT13-LT16, LT20, LT21 (the desk): kept. The analyst and scouts also become people.
- W1-W11 (taken overnight, never confirmed by Cain): untouched. Check against section 6.8 when lane 11 opens.

### 16.3 The build (proposed)

| # | Lane | Holds | Model |
|---|---|---|---|
| N0 | Mock-ups | The conversation page, who's in the room, the deal panel, texts, people coming to you, pick lists | Sonnet |
| N1 | People | What he knows and tells (football too, section 9), wants and actions for every kind, heat and patience, the new stores (unseen breaks, the grapevine, lies, notebooks), people coming to you | Opus |
| N2 | Terms | Measure readers and the football readers on the play log (section 9), every position's terms (8.4, which the promises thread is building now), terms on promises, incentives, NIL, picks, staff and boss goals, plan settings that keep them, the staff's warning | Opus |
| N3 | Deals and requests | Valuing against his best option, asks and counters, deadlines, taking time, many people, requests run as actions, confirm cards | Opus |
| N4 | The conversation | The ear and the voice, cards, hits, turns, channels, groups, memory, saved readings; your players first | Opus |
| N5a | Recruiting | Recruits, families, high school coaches and circles, transfers, visits, signing | Opus |
| N5b | Negotiating | Agents, GMs, NIL, contracts, trades, the undrafted rush | Opus |
| N5c | Staff, bosses, money | Coordinators, position coaches, scouts, the analyst, doctors, hiring and firing, the AD and owner, boosters, compliance | Opus |
| N5d | Game day and the room | Speeches, the sideline, officials, captains, the council, team meetings | Opus |
| N5e | Scouting and the draft | Prospects, sources, and whatever question 10 brings back | Opus |
| N6 | Screens | After Cain okays N0 and N4's contract is set | Sonnet |
| N7 | Checks | Labelled lines per facet, exploits, leaks, cost and time on Cain's Mac | Sonnet |
| N8 | AI staffs on the same logic | Question 11. It changes every world, so it goes on the rebuild gate | Opus |

Order: N0, N1 and N2 now, side by side. N3 and N4 after N1. N5a to N5e after N3 and N4, side by side (separate files). N6 after N0 and N4. N7 with each app build. N8 last. N1 to N7 change only your seat, so no world result changes. The build threads use the fake client; only Cain runs the real key.

## 17. Questions

1. **How your words land.** The AI, reading as him, says which of his real things your words touched, and must point at a real one. The game sizes each from how much it weighs for him and his traits, inside today's caps. My pick: yes.
2. **What "Heard as" shows.** It names what you touched only when you already know that thing about him. Otherwise it says only whether it landed. My pick: yes.
3. **His asks and counters.** The game builds them from what he wants most that you haven't given. The AI only says them. My pick: yes.
4. **Confirm.** Anything that does something real (an offer, money, a hire, a trade, a cut, a benching) asks you to confirm first. A promise counts the moment you say it. My pick: yes.
5. **Colour.** The voice may add details no system reads, kept in a notebook only the voice reads. My pick: yes.
6. **Without a key.** The same asks, deals and requests work through pick lists. My pick: yes.
7. **With a key.** No answer buttons. An "Ideas" list puts words in your box to change. My pick: yes.
8. **Press conferences come in.** You answer reporters in your own words, and it reaches your players, recruits and boss as press answers already do. My pick: yes, since they already move real things.
9. **Social media and fans stay out.** My pick: yes.
10. **Bring back what you turned down on 2026-11-07** ("too focused on dialogue"). In this shape each one gives real information or real actions. My pick by item:

| Item | Bring back? |
|---|---|
| The live whiteboard (a prospect explains a play) | Yes: reads his football IQ |
| Coached interview answers (agents prep prospects) | Yes: makes interviews honest work |
| A prospect remembering you | Yes: everyone remembers now |
| Asking a prospect's family | Yes |
| Asking the team doctor (and pressure to clear a man) | Yes |
| An NFL security director (background checks) | Yes |
| Sending a scout with a question | Yes |
| Asking pro scouts about other clubs' players | Yes |
| A coach's references before you hire | Yes |
| The draft room debate (your scouts argue on the clock) | Yes |
| Reading smoke on calls | Yes |
| Planting smoke | Yes |
| The high school coach answering college calls in words | Yes |
| Typing into Ground's calls | Yes, when Ground is built |
| Your typed notes graded by look-backs | No: not a person |

11. **AI coaches, GMs and agents** use the same deals, asks and terms through the game's own logic, never the API, after your side works. It changes every world. My pick: yes.
12. **People coming to you.** The urgent ones (a trade request, an agent's deadline, a family crisis, a boss meeting) stop Continue. The rest wait in the inbox, and they remember if you ignore them. My pick: yes.
13. **Haiku 5.5** for both calls everywhere. The checks compare it with a bigger model so you can see the difference. My pick: yes.
14. **The build** runs in lanes N0 to N8 (section 16.3), mock-ups first. My pick: yes.
