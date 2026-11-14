# Live talks, rebuilt: everyone you talk to is a real person

Version 2, 2026-11-14. It replaces version 1 (the same file, PR #387), whose questions 3 to 24 are withdrawn.

## Where it stands

Cain's words, 2026-11-14 (UTC):

- 20:22: "my intention was never to add an in game ai just so it can direct me to the right options that are alraedy on my screen ? my intention was always to have the player. I never intended it to be linked to the linebank"
- 20:19, his own ideas: custom football promises ("if i punt in 40 yard distance it breaks my promise"; an EDGE promised "blitzes or stunts to stop him getting chipped") and "a real layer of football knowledge".
- 20:24: "I want a architectural overhaul"
- 22:02: yes to version 1's question 1 (a typed line is never mapped onto a chip) and question 2 (two calls a line: the ear, then the game, then the voice).
- 22:05: mapping your words onto 9 moves and the nearest topic makes it "just as usefull as a linebank".
- 22:12, yes to the three things only an AI playing the person can give: asking gets real information from what he knows ("This has always been my fucking intent with this live feature"), deals you build from real game actions ("good"), and asking people to do things ("good"). Then: "This update was meant to implement it in every facet, recruiting, player relationship, negotiating. Everything, so keep this momentum going and re-design it."

Agreed: those five yeses. Everything else here is proposed, not confirmed. The questions are in section 13.

## 1. What it does

Every person you can talk to in the game is played by the AI as himself, from his real state in the world. You talk to him in your own words. What you learn, what you agree and what you ask him to do become real things in the world. The game decides all of it.

### Your players

- **Ask what he knows.** "Who's checked out in the room?" "Tre. His dad's been sick since October and he keeps talking about going home." That's true in the world (a parent ill, homesick, Tre's portal lean). He knows it because they're close friends. A loyal friend won't tell you. A captain sees more of the room than a backup.
- **Make a deal.** "Stay out of the portal. You get the 3rd-down job, I'll offer your brother, and you meet our collective." He weighs it against what he wants and his real options. He might counter: "The job and my brother. I don't need the collective."
- **Promise in football terms.** "You'll be in a stunt on 1 in 4 of your rushes." The game judges it from your games' snaps.
- **Ask him to do something.** "Take Malik under your wing." It becomes a mentor tie if he agrees. "Talk to Tre for me." He has his own talk with Tre, and it can go badly.

### Recruiting

- **Ask the recruit.** "What did Georgia tell you?" He answers from Georgia's real case with him, and he may oversell it. "Who has the final say?" "My grandma. She wants me close to home." That's his real decider and what she weighs.
- **Ask the people around him.** His high school coach: "What's he like when nobody's watching?" The answer comes from the coach's real eye for players and his warmth toward the kid.
- **Build the offer yourself.** A start with your terms, a NIL figure, his high school coach as an analyst, his brother's offer, his jersey number. He and his decider weigh the package against every other school's.
- **Catch a lie.** He says LSU offered $400,000. His coach says it was about $150,000. You only find out by checking.
- **Ask your commits to help.** "Get your 7-on-7 teammate on a visit." The friend's case for your school gains a real point if your commit goes to bat.

### Negotiating

- **The agent.** "Who else is in on him?" He answers from the real market, and bluffs by his own honesty, as agents already do (`world/pro/market.rs`, `bluff`). "What does he really want?" He tells you what helps his client.
- **Contracts with your own terms.** "$750,000 if he plays 65% of the snaps." Incentives already exist. Now they can be built from anything the game records.
- **GMs.** "Why are you shopping him?" He answers from his real reasons, as far as he wants you to know them. You offer a pick that becomes a 2nd if the player plays 60% of their snaps. He weighs it with his front office's real valuation.
- **NIL, with a player, his family or his agent,** against what his other schools offer.

### Your staff and your bosses

- **Coordinators.** "Why do we keep punting at the 38?" He answers from his real 4th-down habits. "Go for it on 4th and 3 or less past midfield" becomes a game plan setting.
- **Scouts and the analyst.** "What do you really think of the Tulane kid?" The scout answers from his file, with his bias. "Go see him again Friday" becomes a real scouting trip.
- **Your boss and the money.** The AD: "What do I need to keep my job?" He answers from his real goals for you. "Give me $400,000 more for the staff pool" is weighed against his budget and how he rates you. Boosters too, once the AD layer (its own design thread) is in.

### Game day and scouting

- **Pregame and halftime.** Your speech hits each player on his own things. Call out the O-line and the left tackle takes it personally. Captains answer.
- **Prospect interviews and sources.** "Tell me about that night in Tempe." He answers from the real case, in the stance the game drew for him. A teacher or trainer tells you only what they know.

### They come to you

People raise things themselves, from their own state. A player wants to talk about his snaps. An agent calls with the Jets' offer. A recruit's dad calls about playing time. Your OC tells you Miami called him about their head job. You answer in your own words.

## 2. The one rule

The AI reads your words and voices the other person. The game decides everything that matters: what is true, what he knows, what he is willing to say, whether he accepts, what it costs, and what happens next. His words never change the world. Your words change it only through the game.

## 3. How every person works

Every person the game holds is built from four things the world already knows. Where a kind of person lacks one, the build adds it.

### 3.1 What he knows

What he knows is the world's own facts, filtered by where he stands:

- **About himself:** his life events, his mood and its reason, his wants as he'd say them, his offers, his injury as he feels it, what people promised him and whether they kept it.
- **About people close to him** (a close friend, tie 35 and up, family, his mentor): their life events, their moods and reasons, their portal thinking, their trouble.
- **About his unit and his room:** who works and who coasts as practice shows it, who's unhappy, who broke a rule. A captain (leadership) sees more than a backup.
- **From his job:** an agent knows the market and his client. A GM knows his roster, his cap and his staff's grades. A scout knows his file. A coordinator knows his tape. A high school coach knows his kids, their families and every school that came through his door. A boss knows the budget, his goals and the board.

How sure he is follows his eye for it, as the scouting sources already do: the truth, plus his misread, plus his shade (`world/scouting/file.rs`, `source_says`). Lane 6 built that for sources. It becomes everyone's rule.

What he tells you depends on his trust in you, his professionalism, his loyalty to the person it's about, and his own interest in the answer. He can answer straight, hold back, deflect or lie. A lie goes in a store of his lies, like yours today (`claims/truth.rs`, `Live`), so you can catch it by checking another source or the facts.

What you learn goes into your file on that person: your reads, what pulls him, your scout file, your notes on the room. That's the payoff. What you learn changes what you know, and what you know changes your decisions.

### 3.2 What he wants

- A recruit: his decider's wants (`family.decider_wants`).
- A college player: his own wants (`background.rs`, `own_wants`), his role as he sees it (P1's `self_view`) and his mood's reason.
- A pro: money, winning, role and ties, as the market already weighs them.
- An agent: his client's money, his fee and his name.
- A GM: his team's value, his cap and his job.
- A coach: his career, his scheme and his pay.
- A boss: his goals for you, the budget and the program's name.
- A booster: winning and a say.

### 3.3 What he can do

Each kind of person has a list of real actions. A player can mentor, talk to a teammate, recruit a friend, host a recruit, keep something quiet, take a new role or position, play special teams, enter or stay out of the portal, sign, or restructure. A GM can trade. An agent can take an offer to his client. A scout can make a trip. A coordinator can change the plan. A boss can move budget, extend you or hire. A high school coach can steer a kid or open his door.

Whether he does it depends on his trust, loyalty, leadership and what's in it for him. The game then runs the action with its own rules. When you ask your captain to talk to Tre, that talk is played by the game between the two of them, from their traits, not by the AI. He reports back to you, honestly or not.

### 3.4 What you can put on the table

Anything your seat can really do: a scholarship offer (his brother's too), NIL money, contract money, years, guarantees and incentives, a roster spot, a role, a position, a jersey number, a captaincy, time away, a staff hire, a trade piece, a plan setting, an introduction, and promises (3.5). Deals outside the rules already exist (`world/folk/finals.rs`, codes 151-153, 181, 191 and 192), and they keep their real risk.

### 3.5 Terms: promises, incentives and conditions

Cain's idea, made general. A term is built from:

- **a measure:** anything the game records,
- **a number:** taken from your words ("the 50" is a 67-yard try),
- **a scope:** per game, per season, or by a date,
- **a horizon:** when it starts and ends,
- **as long as:** a condition on him.

The same terms serve a promise to a player or a recruit, an incentive in an NFL contract, a clause in a NIL deal, a conditional pick in a trade, and a promise to a coach ("you call the plays next year").

The measures grow as readers of the game log are added. The usage terms from the thread "Make big promises land in live talks" come first: the kicker's field goal line, stunts, blitzes, touches and deep shots. Then playing time by package, alignment, roster moves at his spot, staff hires and the scheme. A promise on something nothing records stays a talking point, and "Heard as" says the game can't hold you to it.

Each term is judged by P1's rules (`docs/design/promises.md`): on its games or its date, with costs scaled by his belief. A term that has a game plan setting gets kept by your staff in simmed games too. Before each game your staff warns you when the plan won't keep a promise.

### 3.6 How your words land

The ear reads your line with his card in front of it. It says which of his real things your words touched (a worry, a want, his pride, a promise you broke, a fact about him) and how: reassured, respected, stung, insulted, doubted. It must name a real item on his card each time.

The game sizes each hit from how much that item weighs for him and from his traits, inside today's caps. A sharp line about exactly his problem moves him. "Great job, keep working" touches nothing. Repeats fade, as they do today (`Folk.talked`).

"Heard as" names a hit only when you already know that thing about him. Otherwise it says whether it landed, as far as your read catches it (`went_words`).

### 3.7 Belief and football knowledge

Cain's idea, for every person at his level. A player talks football the way he knows it. A 10-year vet talks protection slides. A freshman talks reps. A coordinator talks his tape, a scout talks traits, a GM talks the cap.

Belief in a term checks the real numbers. A sharp player believes a plan your numbers back (your coordinator's stunt rate) and doubts one they contradict (you punted at the 37). He also checks what's possible for him: a 67-yard line on a 51-yard leg. Ego believes more, a sharp kid less. All of it goes through P1's belief parts.

### 3.8 The grapevine and your word

What you say travels through ties. Promise two kickers the 50, or call a teammate soft, and it reaches the other man through close friends, by the listener's loyalty, temper and professionalism. Your staff flags a clash with an open promise as you say it, and the clash lowers belief. How you keep your word already reaches recruits through your players and your trail name (P1). Now what you said reaches them too.

### 3.9 Memory and colour

The game remembers in its own facts: what you talked through, every term with your exact words, kept and broken promises, what he told you and whether it was true. The game never reads anything the AI wrote.

The voice may add small colour no system reads, like his dog's name or the spot back home where he trains. That goes into a short notebook that only the voice reads, so he stays the same person next time.

### 3.10 What the voice may say

The voice's card is built by one function from a whitelist type (`live_talk/card.rs`). It holds who he is in words (traits as behaviour, in three bands), what the game lets him say this time, the deal on the table and his answer to it, and how your line landed. It never holds potential, ratings, the depth chart, or a secret he won't tell. When he lies, the game hands the voice the lie, not the truth. When he hides something, the voice gets "he's holding something back" and the tell your read caught. The reply filter stays.

## 4. A line, step by step

1. **You type.** "Stay out of the portal. You're my 3rd-down rusher, and we'll stunt you on 1 in 4 rushes."
2. **The ear** (one call) returns what you asked, what you offered, what you asked him to do, the terms of each promise, the facts you stated, and the hits.
3. **The game** checks the facts, values the offer against his best real option, decides belief in each term, sizes the hits, decides what he'll say and whether he counters, and logs each promise with your words.
4. **The voice** (one call) answers as him: "1 in 4? I've been chipped on half my rushes since October. If that's real, I'm not going anywhere."
5. **On screen:** "Heard as" under your line, with the deal on the table, his reaction to each part as your read sees it, and the trust and mood bars.
6. **Anything that does something real** (an offer, money, a hire, a trade) shows a confirm card before it happens. A promise counts the moment you say it.

The reading is saved with the talk, so a reload plays the same world.

## 5. The deal engine

- A deal is a set of terms (3.4, 3.5). He values each one by his wants. A promise counts by his belief in it. Money is valued by the game's existing models (the NFL market, `worth`, the NIL pull).
- He compares the deal with his best real option: another school's case, another club's offer, staying put, the portal, free agency.
- He accepts, counters, refuses, or takes time ("let me talk to my mom"). Taking time is real: his decider and his agent get their say.
- His counter is built by the game from what he wants most that you haven't given, sized to close the gap. The AI only says it.
- Agents' bluffs and the GMs' calls on them stay as they are (`market.rs`, `bluff`).

## 6. Requests

- A request names one of his actions (3.3). He weighs it by trust, loyalty, leadership and what it costs or gains him.
- If he agrees, the game runs the action by its own rules (a mentor tie, his own talk with a teammate, a recruit's visit, a plan setting, a scouting trip). Each has a result, and some can backfire.
- He reports back in his own words, honestly or not.

## 7. New things the world stores

People can only know what the world keeps. This adds what they'd really know:

- Rule breaks your staff didn't see. Today they're rolled and dropped (`folk/rules.rs`, `your_week`). Teammates who were there now remember them.
- Chips in your games (the usage promises thread).
- What you said to whom (the grapevine).
- Terms on contracts, NIL deals and trades: custom incentives and conditional picks.
- What each person told you, and whether it was true (his lies store).
- Each person's colour notebook (voice only).
- Your learned notes, in your existing files.

## 8. Without a key

Proposed: the same asks, deals and requests work through pick lists. You pick a person, then a question, terms or a request from lists. The game logic is the same, so a save made with the AI plays without it. The old chips and line bank keep the words. With a key there are no answer buttons. An "Ideas" list shows what's on his mind and what you could offer, and clicking one puts words in your box that you can change.

## 9. Guards

- Outcomes come from attributes, traits and the world's facts. The AI judges which of his real things your words touched. The game decides how much each one counts, inside today's caps (morale moves growth at most ±15%, a home visit a school's pull at most +0.03, and so on).
- The AI can't invent facts or reasons. Every hit names a card item, and every answer is what the game released.
- Hidden potential never reaches a card. Nobody in the world knows a player's true ceiling. People have opinions, and those are reads with their own eye and bias.
- Same save, same reading, same world.
- Prompt tricks: your words go in tagged as data. A line that isn't a person talking does nothing. A tricked voice only changes words.

## 10. Cost and time

Claude Haiku 5.5: $0.10 in, $0.01 a cache hit, $0.50 out, per million tokens (`proposal.md`, "Model and cost"). Writing the cache costs about 1.25x the input price. Token counts are Claude's estimates. Lane 1 measured 1,616 cached tokens for the old reader.

| Call | Cached | New in | Out | Cost a line |
|---|---|---|---|---|
| Ear | about 3,000 (rules, schema, measures) | about 1,200 (his card items, the talk so far, your line) | about 300 | about $0.0003 |
| Voice, first line | about 4,000 written (rules, his card) | about 800 | about 150 | about $0.0007 |
| Voice, later lines | about 4,000 read | about 800 | about 150 | about $0.0002 |

- A 6-line talk: about $0.0035.
- A heavy season, about 1,100 talks with every facet in: about $4. Typical: about $2. The $50 monthly cap stays.
- A Sonnet 5.5 ear costs about $0.005 a line, so a heavy season would run about $33. That's why Haiku is the pick, with the checks lane testing it.
- Time: Haiku answered a small call in 0.8 s. Ear then voice is about 2 to 2.5 s a line. "Heard as" shows after about 1 s.

## 11. What's kept, changed and replaced

| Part | Fate |
|---|---|
| Lane 1, the pipe (key, client, spend, cap, tickets) | Kept. Adds a cache mark for the voice's card. |
| Lane 2, the boxes | The talk box is replaced by the conversation screen. The settings card stays. |
| Lane 3, the core | Kept: tickets, the whitelist card, the filter, the ask-first picks, the lies store, the fact checks. Replaced: landing a line on one option (`scene::say`), the writer that rewrote a bank line, rule LT1. |
| Lane 4, the desk | Kept as what your analyst and scouts know. They become people you talk to and give jobs. |
| Lane 5, rooms | The pregame and halftime reading is replaced by hits for each listener (fire, poise and focus in `game/talk.rs` stay as the sizes). Parent calls and discipline move to conversations. |
| Lane 6, rooms that gain | `source_says` is kept and becomes everyone's rule for what they know. Interviews, sources and the staff meeting move to conversations. |
| Lane 7, NFL desk and boss (not built) | Not built on the old brief. It becomes part of the negotiating lane. |
| Lane 8, the off-field price | Kept. |
| Lane 9, one-on-ones (merged #365) | Kept: the roster fact checks, a false fact's cost on the spot, promise logging. Replaced: the turn card and its writer (`say/one_on_one.rs`). |
| Lane 10, college desk (merged #381) | Kept: recruit fact checks, lies found at signing and arrival, NIL checks. Replaced: the mapping onto pitch points (`say/recruit.rs`). |
| Lanes 11 and 12 (AI staff, dev tools; not built) | Not touched. Lane 11's AI staff overlaps with "staff you give jobs to" (section 6), so check before it opens. |
| Talk facts L1-L13 (the fact sheet) | Kept. The sheet is what he knows about himself. |
| Promises P1 and P2 | Kept. Every term is judged through P1. |
| The usage promises thread | Its terms are the first measures. |
| Line bank, chips | Kept for play without a key. |

## 12. Build plan (proposed)

| # | Lane | Holds | Model |
|---|---|---|---|
| N1 | People | What he knows and tells (section 3.1), wants and actions for every kind, the new stored facts (section 7), the grapevine, people coming to you | Opus |
| N2 | Terms | Measure readers, terms on promises, incentives, NIL clauses and picks, plan settings that keep them, the staff's warning before a game | Opus |
| N3 | Deals and requests | Bundles, valuing against his best option, counters, taking time, requests run as actions, confirm cards | Opus |
| N4 | The conversation | The ear and the voice, cards, hits and their sizes, "Heard as", memory and colour, ending a talk, saved readings; your players first | Opus |
| N5 | Every facet | Recruiting (recruits, families, high school coaches, transfers), negotiating (agents, GMs, NIL, the boss), staff and scouts, team talks, prospects and sources | Opus, in 2 or 3 lanes |
| N6 | Screens | Mock-ups first for Cain, then the conversation page, the deal panel and the pick lists | Sonnet |
| N7 | Checks | 200 labelled lines across facets, 40 exploits, the leak test, cost and time on Cain's Mac | Sonnet |

N1 and N2 run side by side. N3 and N4 come after N1, and N6's mock-ups run now. N5 comes after N3 and N4. N7 runs with each app build. N1 to N7 change only your seat, so no world result changes and the world rebuild isn't touched. AI staffs using the same deals and requests (question 9) is its own lane afterwards, and it changes every world.

## 13. Questions

1. **How your words land.** The AI, reading as him, says which of his real things your words touched, and must point at a real one. The game sizes each one from how much it weighs for him and his traits, inside today's caps. My pick: yes.
2. **What you see.** "Heard as" names a hit only when you already know that thing about him. Otherwise it says only whether it landed, as far as your read catches it. My pick: yes.
3. **Counters.** The game builds his counter from what he wants most that you haven't given. The AI only says it. My pick: yes.
4. **Confirm.** Anything in a deal or request that does something real (an offer, money, a hire, a trade) shows a confirm card first. A promise counts the moment you say it. My pick: yes.
5. **Colour.** The voice may add small details no system reads (his dog's name), kept in a notebook only the voice reads, so he stays the same person. My pick: yes.
6. **Without a key.** The same asks, deals and requests work through pick lists. My pick: yes.
7. **With a key.** No answer buttons. An "Ideas" list puts words in your box that you can change. My pick: yes.
8. **Press and media** stay out, as you said on 2026-11-07. My pick: yes.
9. **AI coaches, GMs and agents** use the same deals, requests and terms through the game's own logic (never the API), built after your side works. This changes every world. My pick: yes.
10. **Haiku 5.5** for both calls. The checks lane tests it against a bigger model and shows you both. My pick: yes.
11. **The build** runs in lanes N1 to N7 (section 12), with screen mock-ups first. My pick: yes.
