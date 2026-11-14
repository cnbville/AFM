# Live talks, rebuilt: the AI plays the person

Proposed, not confirmed. Written 2026-11-14 for Cain, after his messages that day:

- "my intention was never to add an in game ai just so it can direct me to the right options that are alraedy on my screen ? my intention was always to have the player. I never intended it to be linked to the linebank"
- "it feels like i'm promising him the keys to the kingdome and the linebank makes it feel like i'm saying 'You'd be good addition"
- "Why isn't it making custom promises ? Add a promise where if i punt in 40 yard distance it breaks my promise. Same as if i talked to a Edge Rusher and promise him we'll run blitzes or stunts to stop him getting chipped. I'm missing a real layer of football knowledge right now"
- "I want a architectural overhaul"

Nothing here is built. Every size is Claude's, proposed, not confirmed. The questions are at the end.

## 1. What is wrong today

Read on main 81f5b14 (lanes 1-6, 8, 9 and 10 merged).

- A typed line is read onto one of the scene's own options: the reader returns an option id (o1 to o12) or a turn (t1 to t48 in a one-on-one), and the game runs that chip's code (`live_talk/say/mod.rs`, `land`; `say/one_on_one.rs`).
- His answer is the line bank's line for that chip. The writer then rewrites it, so he answers the chip, not your words (`writer_step`).
- "Heard as" shows the chip. Your "you'll kick from the 50" becomes "You'd be a good addition".
- A promise can only be one of the fixed promise kinds (start, snaps, role, place, contract, early chance, no redshirt). Nothing covers how you'll use him.
- This was the agreed rule LT1 ("a typed line becomes one of today's choices and scores with the chip's formula"). Cain says now it was never his intent.

## 2. The new shape

With a key, a talk is a real conversation with the person. Three parts, in this order, for every line you type:

1. **The ear** (one call) reads what you did: every act in your line, the facts you stated, the promises you made with their exact terms. It knows nothing about chips.
2. **The game** decides what your line did to him: trust, mood, belief in each promise, whether he opens up, whether he walks out, what he tells his friends. It uses his attributes, his traits, his state and the world's facts. The AI decides none of it.
3. **The voice** (one call) is him. It gets who he is today in words, the talk so far, your words, and how your line landed. It answers as him, in his own words, about what you actually said.

His words never change the world. Only your words do, and only through the game.

Example, a kicker in March:

- You: "You're kicking from the 50 this year. No more punting from the 35."
- Heard as: "A promise: every 4th down inside the opponent's 50 is a field goal try for him, not a punt (a 67-yard try at most), all season."
- The game: his longest make is 51, his staff calls his leg good to about 55. He's sharp (high awareness) and remembers you punted from the 37 three times last fall. He believes it a little. Trust +0.03, mood +0.08.
- Him: "The 50? Coach, I'll take every one you give me. But you sent the punt team out at the 37 against Iowa. I'll believe it when I see it."
- Week 3, 4th and 6 at their 41: the plan's field goal line keeps it, or your staff warns you before the game that it won't.

Example, an EDGE rusher:

- You: "We're going to stunt you with the 3-tech and send the nickel off your side, so the back can't chip you."
- Heard as: "A promise: he's in a stunt on at least 1 in 4 of his rushes, per game, this season. A plan for the nickel to blitz off his side (a talking point, nothing the log can hold you to)."
- Him, a fourth-year who's been chipped all year: "That's all I've wanted since October. The tackle's had help every snap."

## 3. The parts

### 3.1 The conversation (engine, new)

A new talk object in the engine, `folk/convo`, beside today's scenes. It holds who, where, the talk's time, the lines on both sides, the acts read so far and what they did. It replaces the scene with options for every talk with a person when a key is set. Without a key, today's scenes, chips and line bank play exactly as now.

It keeps from today: the talk hours, the fact sheet (`claims`), the people a line may name, the picks that ask first (`finals.rs`), the lies store and its daily check, P1's belief and `settle_word`, and the fact checks lanes 9 and 10 built (`claims/check/roster.rs`, `recruit.rs`, `money.rs`).

### 3.2 The ear: acts, not chips

The ear returns a list of acts. Each act is:

- a move (the nine today: Ask, Tell him straight, Praise, Challenge, Criticize, Promise, Warn, Hand him something, Listen),
- a tone (Fiery, Warm, Firm, Calm),
- what it's about: a point on his sheet, a teammate, a subject, or a free topic in a few words,
- for a fact: its kind and what you said, checked by the game as today,
- for a promise: its terms (3.5).

A line with no act ("Hi", gibberish, an order to the AI) does nothing. The voice may still answer it as a person would ("Coach?").

### 3.3 The game decides

Each act is scored with today's formulas for its move, tone and subject (`talks.rs`, `landing`; `talk_points::rest`): trust, mood, his stance, opening up, praise by how true it rings, criticism by his ego. Nothing new is invented for the size of a line. What's new:

- several acts in one line, with diminishing weight (question 4),
- free topics off his sheet, scored as the move on the nearest subject (question 3),
- belief in a promise from its terms and the real numbers behind it (3.6),
- what he passes on to others (3.8),
- whether he ends the talk (3.9).

The reading is saved with the talk, so a reload replays the same world.

### 3.4 The voice: who he is today

The voice's card is built by one function from a whitelist type (as `live_talk/card.rs` does today). It holds, in words:

- His traits as behaviour, in three bands ("proud, needs to hear he matters"; "hot-headed"; "a pro, does the work"). Never numbers.
- His mood and its reasons. His trust in you, and why (kept and broken promises, how your last talks went).
- His sheet: what's on his mind, today's points with their real values.
- What pulls him and his other options as he knows them: a recruit's other schools and what they told him, a pro's market as his agent tells it.
- His own view of his role: his practice group, his snaps and starts, what you told him. Never the depth chart.
- His football world (3.7).
- Open promises, with your exact words and their terms.
- What he wants to hear from you today (his asks), from his wants and his mood's reason.
- How your line landed: good, flat or bad; how far he believes each promise; his stance; whether he opens up and what the game lets him tell.
- His voice: age, where he's from, his background, how long he's known you.

Never on the card: potential, ratings, hidden numbers, the depth chart, anything he couldn't know. The reply filter stays (ratings words, numbers not on the card).

The voice is free in what he says. It may ask back, push, bargain, joke, or go quiet. It must answer in the spirit the game gave it (a bad landing is never a warm reply).

### 3.5 Promises from your words

A promise is no longer one of seven kinds. It is built from parts:

- **who**: him, or a group he's in,
- **a measure**: something the game records,
- **a number**: from your words ("the 50" is a 67-yard try; "a lot of targets" has no number, so the ear asks),
- **a scope**: per game, per season, or by a date,
- **a horizon**: when it starts and ends,
- **as long as**: a condition on him ("as long as you get 3 tackles a game", as Cain agreed 2026-11-13).

A measure is anything a game or the world records. The first set:

- Playing time: starts, snaps, snap share, by package (3rd down, red zone, base, nickel).
- Usage: carries, targets, air yards of 20 or more, red zone touches, returns, his alignment (slot or outside, wide 9 or 5-tech).
- Defense: his pass rushes, rushes in a stunt, blitzes, his coverage (man or zone, on the slot).
- Kicking: punts and field goal tries by field position and distance, kickoffs.
- Position: where he lines up in games.
- Roster: no signing or draft pick at his spot, no player ahead of him added.
- Staff and scheme: a hire (a position coach), a scheme (a 3-4 front, an air raid offense).
- Money and life: NIL, a contract, time away, playing near home.

The other thread's usage terms (thread "Make big promises land in live talks", items 1-9) are the first measures with a game plan setting each. A new measure is one reader of the game log or the world. A promise on something nothing records stays a talking point, and "Heard as" says the game can't hold you to it.

Each promise is judged by P1's rules (`docs/design/promises.md`): kept or broken on its games or its date, costs scaled by his belief.

### 3.6 Football knowledge decides belief

Belief stays P1's model (likely and shape). A usage promise adds two parts:

- **Is it real?** The game checks your real numbers: your coordinator's stunt rate, where you punted last season, how many targets your slot got. A sharp player (awareness, years in the game) believes a promise your numbers back, and doubts one they contradict. A raw freshman believes either.
- **Is it possible?** A 67-yard line for a kicker whose longest make is 51 is a stretch. Ego believes it more, a sharp kid less.

A specific plan is believed more only when it's real. Wording never beats talent.

### 3.7 His football world

The voice knows his game the way a player does:

- How he's been used, from your games: his snaps, how often he was chipped (once the engine saves chips), his stunts, his alignment, the kicks he got and the distances.
- The scheme in words: "a 4-3 under, the DC stunts a lot on passing downs."
- His own numbers he would know: his longest make, his sacks, his yards a carry.
- His football IQ (awareness, years) sets how technical he talks. A 10-year vet talks protection slides. A freshman says "I just want to get after the QB."

### 3.8 What he passes on

What you say about someone else travels. Tell a receiver his friend is soft, or promise two kickers the 50, and it reaches the other man through close friends (tie 35 and up), by the listener's loyalty, temper and professionalism. It costs that player's trust as talk behind his back does. Your staff flags a clash with an open promise as you say it.

### 3.9 How long a talk runs

A talk runs until you end it, he ends it, or its time is up. Typed lines cost time as today (`dialogue.rs`, `TYPED_CHARS_A_MINUTE`). He ends it early when the game says (a hot temper and a bad line, an insult, a sulk). The voice never ends a talk on its own.

### 3.10 Memory

He remembers in game facts, not AI text: what you talked through (`Folk.talked`), promises with your exact words, kept and broken ones, how the last talks landed. Nothing the AI writes is ever read back by the game.

### 3.11 The screen

With a key, the talk page is the conversation: his reply, your box, "Heard as" under your line with each act and each promise's terms, and the trust and mood bars as today. The chips fold under "Ideas". A clicked idea is said as your line (no ear call: its reading is known) and answered by the voice. Without a key, the page is today's.

### 3.12 Who gets it

Every talk with a person: one-on-ones, recruits and their families, parents, discipline meetings, staff and job interviews, prospect interviews, sources, coordinators, agents, GMs and your boss. Money talks keep their numbers in the game's negotiation model; the voice only speaks for the agent.

Not this shape: the pregame and halftime rooms (a speech, no back and forth: lane 5's reading stays, a captain's answer is voiced), and the desk asks (analyst, scouts, find me a player: lane 4 stays, it was never on chips).

AI teams never call the API. Their coaches keep today's picks.

## 4. Cost and time

Prices: Claude Haiku 5.5, $0.10 in, $0.01 a cache hit, $0.50 out, per million tokens (`proposal.md`, "Model and cost"). Writing the cache costs about 1.25x the input price. Token counts are Claude's estimates; lane 1 measured the reader's rules and schema at 1,616 tokens.

| Call | Cached | New in | Out | Cost |
|---|---|---|---|---|
| Ear, each line | 2,500 (rules, schema, measures list) | about 1,000 (what he can be about, the talk so far, your line) | about 250 | about $0.00025 |
| Voice, first line | 3,500 written (rules and his card) | about 550 | about 120 | about $0.00055 |
| Voice, later lines | 3,500 read | about 700 | about 120 | about $0.00017 |

- A talk of 6 lines: about $0.003.
- A season, heavy use (about 300 talks of 6 lines, plus the desk as today): about $1.5 to $2. Typical: under $1. The $50 cap stays.
- A Sonnet 5.5 voice ($2 in, $0.10 a hit, $10 out): about $0.026 a talk, about $8 a heavy season. It also needs thinking on (lane 1 found Sonnet refuses `disabled`), so it is slower.
- Time per line: Haiku answered in 0.8 s on a small call. Ear then voice is about 2 to 2.5 s. "Heard as" shows as soon as the ear answers (about 1 s), then his reply.

## 5. What happens to each part built so far

| Part | Fate |
|---|---|
| Lane 1, the pipe (key, client, spend, cap, tickets) | Kept. Adds a second cache mark for the voice's card. |
| Lane 2, the boxes | Changed: the talk page becomes the conversation (3.11). The settings card and ask box stay. |
| Lane 3, the core | Split. Kept: tickets, the card whitelist, the filter, ask-first picks, the lies store, the fact checks, `Heard`. Replaced: `scene::say` landing on one option for person talks, the writer rewriting a bank line, LT1. |
| Lane 4, the desk | Kept as is. |
| Lane 5, rooms | Pregame and halftime reading kept. Parent calls and discipline move to the conversation. |
| Lane 6, rooms that gain | The engine side kept (worries, sources, plan settings). Interviews, sources and the staff meeting move to the conversation. |
| Lane 7, NFL desk and boss (not built) | Starts on the new shape. |
| Lane 8, the off-field price | Kept. |
| Lane 9, one-on-ones (merged) | Kept: the fact checks (`claims/check/roster.rs`), a false fact's cost on the spot (`typed_against`), typed promise logging. Replaced: the card of turns t1 to t48 and its writer (`say/one_on_one.rs`). Its 100 labelled lines are relabelled as acts. |
| Lane 10, college desk (merged) | Kept: recruit fact checks, lies found at signing and arrival, NIL figure checks. Replaced: the mapping onto pitch points and promise picks (`say/recruit.rs`). |
| Lanes 11 and 12 (AI staff, dev tools) | Untouched. |
| Line bank (all 100k+ lines, your own lines L12) | Kept for play with no key. Never used with a key, not even as examples for the voice. |
| Talk facts lanes L1 to L13 (the fact sheet) | Kept. The sheet is what the voice knows. |
| Promises P1 and P2 | Kept. Promises with terms (3.5) judge through P1. |

## 6. Build plan (proposed)

| # | Lane | Holds | Model |
|---|---|---|---|
| O1 | The conversation | `folk/convo`, the ear's acts, scoring acts, the voice and its card, memory, ending a talk, saved readings, one-on-ones on it first | Opus |
| O2 | Promises with terms | The measure readers, the promise parts, belief from real numbers, plan settings that keep them, the staff's warning before a game (builds on the usage terms thread) | Opus |
| O3 | The screen | The conversation page, "Heard as" with acts and terms, Ideas | Sonnet |
| O4 | Every other talk | Recruits and families, parents, discipline, interviews, sources, coordinators; then lane 7's agents, GMs and boss | Opus |
| O5 | Checks | 100 lines labelled as acts, 40 exploits, the voice against the game's landing, cost and time on Cain's Mac | Sonnet |

O1 and O2 side by side (O2 owns the measures, O1 calls them). O3 after O1's contract. O4 after O1. O5 with each app build.

## 7. Questions

1. Drop LT1. With a key, your line is read into what you did, never onto a chip. My pick: yes.
2. Two calls a line: the ear, then the game, then the voice. My pick: yes (one call would let the AI decide how he takes it before the game does).
3. A free topic off his sheet is scored as the move on the nearest subject. My pick: yes.
4. A line counts up to 3 acts: the strongest in full, the second at half, the third at a quarter, inside today's caps. My pick: yes.
5. The voice gets his whole state in words (3.4), never potential, ratings or the chart. My pick: yes.
6. His traits reach the voice as behaviour in three bands, so talking to him tells you his character the way real talks do. My pick: yes.
7. The voice must answer in the spirit the game gave it, and is free in everything else. My pick: yes.
8. His words never change the world. A threat to transfer is said only when the game's lean says so. My pick: yes.
9. A promise is built from parts (who, measure, number, scope, horizon, as long as) over anything the game records. My pick: yes.
10. Belief in a usage promise checks your real numbers: a sharp player believes it more when they back it and less when they don't. My pick: yes.
11. Belief also checks what's possible for him (a 67-yard line for a 51-yard leg): ego believes more, a sharp kid less. My pick: yes.
12. The voice knows his football world (3.7), and his awareness and years set how technical he talks. My pick: yes.
13. Before each game, your staff warns you when the plan won't keep a promise. My pick: yes.
14. A promise lands as heard, with its terms shown. Only an unsure reading asks first, and the picks that can't be undone still ask. My pick: yes.
15. What you say about another player reaches him through close friends, by the listener's loyalty, temper and professionalism. My pick: yes.
16. A clash with an open promise (two starts at one spot) is flagged by your staff as you say it, and lowers his belief. My pick: yes.
17. A talk runs until you end it, he ends it or its time is up, and he walks out only when the game says. My pick: yes.
18. He remembers in game facts and your exact words, never in text the AI wrote. My pick: yes.
19. With a key, the chips fold under "Ideas", and a clicked idea is said as your line and answered by the voice. My pick: yes.
20. Without a key, today's chips and line bank play unchanged. With a key, the bank is never used, not even as examples. My pick: yes.
21. Haiku 5.5 stays for the ear and the voice. My pick: yes.
22. Before 21 is final, a 10-talk comparison with a Sonnet voice runs on your Mac (about $0.30). My pick: yes.
23. The new shape covers every talk with a person (3.12). The rooms and the desk keep theirs. My pick: yes.
24. The build runs in lanes O1 to O5 (section 6). My pick: yes.
