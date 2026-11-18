# Personal talk

Design, 2026-11-17, thread "Let talks get personal". Cain: "let's add a little bit of personal talk to convo's when i ask about families i keep getting 'They're good, but let's talk about football'. Multiple personal questions just get the same few lines, there needs to be more interpersonal guidelines." Cain agreed to all nine items the same evening ("yes to all"). Every size below is Claude's, proposed, not confirmed.

## Why it happened

Every personal question reached the game as one topic, Life. The game only looked for a life event on his fact sheet (a birth, a death, a sick parent, a divorce, money trouble, homesick). With none, it handed the voice one fixed sentence, "Nothing new at home to tell: his family is doing fine" (`talk::own_nothing`). The voice's card carried his age, state, high school, money growing up and class, and none of the family facts the world stores. The voice may add no fact the card doesn't give, and its colour notes may not touch family, so it said "fine" and went on to what it wants from the talk: football.

## How real people do it

People open up in layers (social penetration theory, Altman and Taylor 1973): the surface (where you're from, your family, what you do on a day off) with nearly anyone, how you feel about home with people you know, and your troubles only with people you trust. Disclosure is returned: a man who hears something personal from you tends to give something back. Asking too deep too soon closes people. Good college staffs know the mothers' names and ask after the little brother's game, and players remember the coach who asked.

## The nine items (agreed 2026-11-17)

1. **His people on the card.** The voice gets what the world already stores about his home: who raised him, who makes his calls, an older brother playing college football, a father who played college or pro football, how his folks show up at games, a mentor back home, his own wife and children from his life events, his high school and hometown, and what has happened at home before (his story).
2. **Small details drawn, not stored.** What the world doesn't store (his mother's or father's first name, a parent's job, younger brothers and sisters, his major in college, a girlfriend, what he does on a day off) is drawn from his background, keyed to him, so it is the same every time and never contradicts what is stored (a boy raised in foster care gets no dad). Coaches' families already work this way (`life::family`). These details are flavour only: they drive nothing.
3. **Layers, not a secret.** A new question topic, Home, for small talk about his life away from football (home, family, hometown, school, his day off, his kids). It has no trust gate and asking about a new part of his life is never "asking again". Life stays for troubles at home (a sick parent, money at home, a loss, a divorce), behind its trust gate as before. What he says about home comes in three layers: the surface, how he feels about home, and his troubles.
4. **How open he is comes from his makeup and your bond.** His openness is the bond's own (loyalty and coachability open him up, ego and temper close him, a man burned by a coach in the last year is guarded) with a little more for a man who warms to new people (adaptability). How deep he goes is your bond's stage, moved one step by his openness, and for anyone without a bond (a recruit, a parent, a coach, a boss), his trust in you times his openness.
5. **He answers and stays on it.** A personal question gets a personal answer. He turns to football, his role or his own ask in the same breath only when something presses on him (a want that weighs at least `PRESSING`).
6. **You opening up counts.** When you share something about your own life, it lands as a hit on his life, more when the two of you share something real (home state, how you grew up, money growing up, your high school: `home::common`). The voice never makes up facts about you: his card has only your profile's public facts and what you told him.
7. **He remembers, both ways.** What he told you about his life and what you told him about yours are kept (`memory::Told`). The ear sees what he told you, so asking after it later ("how was Kayla's graduation?") lands as a remembered hit. Asking again what he already told you reads as not listening: a small trust cost, and he says he told you.
8. **A good personal talk feeds the bond.** A talk where something personal landed (something new told at his layer, something you remembered, you opening up) counts as one more week of contact with you (`bonds::personal`), on top of the sit-down's own week, once a week at most. The bond's caps stay (four extra weeks owed, time alone stops at 0.70 without a hard moment). Asking about his troubles before he'd open up that far costs a little trust and closes him for the rest of the talk.
9. **The same for coaches, staff, bosses and parents.** Your coordinator talks about his own wife and children (the coaches' drawn families), a parent about raising him and the rest of the family, anyone else from his own drawn family, all on the same layers and openness.

## First meetings (2026-11-17)

Cain, the same night, with a screenshot of a first meeting where the player opened "Ready. Tell me what I'm getting paid and how it's figured." and the coach's "First time meeting, nice to meet you" was heard as not answering him: "also this ruins all personal feleing you get to players, there needs to be more personal conversations". Built with the nine items:

- The first time he sits down with you (no one-on-one with you yet, no bond past Just met), he opens with a hello in his own way (by his mood and makeup), not the subject, unless he asked for the meeting, it's an exit meeting or something on his sheet weighs on him.
- Your hello or a little small talk is its own act. Early in a talk it lands a little on his life, by how open he is, and he says hello back his own way. A hello on your first line is never a dodge.

## Sizes (proposed, not confirmed)

- Openness: a player's is `bonds::openness` × (1 + 0.1 × adaptability z), 0.6-1.4. Anyone else: 1 + 0.04 × (warmth - 10) + 0.02 × (loyalty - 10) - 0.02 × (ego - 10) - 0.015 × (temper - 10), 0.6-1.4.
- Depth with a bond: Just met the surface, Knows you how he feels, Trusts you and up his troubles; openness 1.15 or more moves him one layer deeper before Trusts you, 0.8 or less one layer shallower. Without a bond: trust × openness under 0.5 the surface, under 0.65 how he feels, else his troubles.
- What he says to one question: up to 1 + (openness ≥ 0.95) + (openness ≥ 1.2) things, those the question names first, then what he hasn't told you yet.
- Pressing: a want weighing 0.5 or more still comes up after a personal answer (his usual bar is 0.22).
- Hits: a personal answer at his layer reassures his life (weight 0.7); you opening up respects it (weight 0.6, × (1 + 0.5 × shared parts), at most 3 parts); remembering respects it (weight 1.1). Asking again what he told you costs 0.01 trust. Asking about his troubles too soon costs 0.015 × (1.6 - openness). All inside the line's caps.
- Drawn details: younger brothers and sisters 0 (30%), 1 (35%), 2 (20%), 3 (15%), half brothers and half sisters; a girlfriend for 35% of high schoolers and 40% of college players; a pro 25 or older is married 55% of the time when his life events say nothing; majors from the majors college football players most often take (general studies, business, communications, kinesiology and exercise science, sports management, criminal justice, sociology, psychology), leaned on his grades.
- Memory: 12 things told kept a person, both ways together.
- A hello on the first two lines: respected on his life, weight 0.3 x his openness.
