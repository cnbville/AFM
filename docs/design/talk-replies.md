# Talk replies that match the game, and talks as a shortcut

Thread "Diagnose why talks feel off", 2026-11-19. Cain shared five talks (Justis Sanders Jr., Jeremiah Lampkins, Donte Jones twice, Joshua Merez II) and asked what feels off "without me saying". Diagnosis on main 349eefb; every item below was agreed by Cain the same day. Sizes and wording beyond what is written here are proposed, not confirmed.

## What was wrong (the causes, in the code)

- The game works out how your line landed ("Heard as", `Turn.landing` and the hits), but the voice card that writes his reply never gets it (`afm-game/.../persona/cards.rs`, `voice`). "Reassured on a starting job; it landed" sat above "I've heard promises before, show me on paper".
- A promise to a player releases nothing to the voice (`talk/mod.rs`, `Act::Promise`), so the voice fell back on stock lines: "promises before", "on paper", "in writing" in 4 of 17 replies.
- Answering his ask or counter is read by the ear and dropped by the game (`Act::Answer(_) => {}`): "Consider it done" did nothing.
- A counter never carries his number: `Give::Money(_)` is worded "more money", and a coach's counter is "he'd need more money to take the job" though `hiring::would_take` has the figure (`office/deals.rs`, `weigh`).
- A promise made in this talk shows on the next line in his history as made "today" (`talk/between.rs`), read as an earlier meeting ("you told me this morning").
- A claim or decision that contradicts an open promise of yours isn't treated as breaking it (Justis: his place wouldn't change, then "you'll be last").
- A man's wants follow his job, not the talk: a high school coach interviewing for your staff keeps a feeder coach's wants (`wanting.rs`, `Kind::HsCoach`: his kids' futures, `Give::Favour` "clinic and camp invites"), and wouldn't name his players.
- Everyone talks alike: 16 of 17 replies say "Coach,", 4 say "I appreciate", 4 open by echoing your words as a question. Nothing gives a person his own way of speaking; the voice runs with thinking off.
- "Heard as" lists every hit, both ways and some with no object ("stung on his pride; doubted; reassured; it cost you a little").
- The ear and the voice see only the last 6 lines (`card::SO_FAR`).
- Ending a talk that holds a decision keeps the page open with its picks (`wrap::close`).
- Most asks run nothing: "move to slot", "make weight" and others reply "He agreed to..." and change nothing (`asks.rs`, the catch-all); no act starts a focus or a project.
- Coaches' money is one fixed bar: below it "more money", above it yes, nothing moves between rounds.
- Ideas show only for your own players and never name a want; coaches, candidates, parents and agents get none.

## Agreed (Cain, 2026-11-19)

Cain, 13:07 UTC: "agree on all but i'm missing some." Then 13:26 UTC on the five additions: "1. Yes, not just for this, but for all possible actions. Live talks needs to devlop into a sophisticated short cut / yes to the rest".

### Lane A: replies that match the game

1. **Blind voice.** His card says how your line landed, in his terms ("you believe him on the starting job; it eases you"), and his reply shows it.
2. **Promise reaction.** A promise releases his take on it from his belief in your word: he takes it, takes it warily, or doubts it out loud. He asks for it in writing only when his trust is low and he's that kind of man.
3. **Answers count.** Accepting his ask or counter makes it your offer on those terms (the confirm card as for any deal); refusing closes it; a counter of yours is weighed like an offer.
4. **He names his number.** His counter carries his figure the way people say it ("two-eight, three years"). Asked for a ballpark he gives his ask; a cagey man gives a floor or a range, by his makeup.
5. **This talk stays this talk.** Promises and claims from the talk you're in show on his card as "earlier in this talk", not in his history with you.
6. **Going back on your word.** A claim or decision that contradicts an open promise of yours breaks it on the spot, with the promise layer's cost; he's told and raises it.
7. **Wants follow the talk.** In an interview every candidate has a candidate's wants (role, money, path, scheme); a high school coach adds his players recruited as part of the hire, and asked about his players he names them and talks them up by his ego.
8. **His own way of talking.** Speech habits drawn once from his makeup, age and background and kept with him: how he addresses you (Coach, Coach and your name, sir, nothing), how long he talks, a phrase or two of his own. His own last lines in the talk go on the card with a rule not to reuse an opener. Never fixed by pasting lines.
9. **Voice thinks a little.** A short think before each reply, as the ear has since PR #575 (about double a reply's cost, about a second slower).
10. **Shorter "Heard as".** What your line was, at most one hit each way with what it touched, then how it landed; hits with no object dropped.
11. **Whole talk remembered.** The ear and the voice see the whole talk, plus a short "settled so far" list (agreed, promised, refused, still open).
12. **End means over.** End always closes the talk. A decision it leaves open (hire him or not) moves to its screen (the staff screen's candidates) with a line in your inbox, and his patience runs from that day.

Lane A starts after PR #575 (the ear audit) merges, and is measured by replaying Cain's five talks on the real model (`replay_tests.rs`).

### Lane B: talks as a shortcut

13. **Act from the talk, for every action.** Cain: "not just for this, but for all possible actions. Live talks needs to devlop into a sophisticated short cut". Anything you say you'll do, or ask someone to do, that a screen in the game can do runs from the talk after one confirm card: a focus, a skill or body project, a position move, a depth chart spot, a staff assignment, a recruiting target, a training plan, a contract step, and every other action the screens offer. If it can't run (no room left, wrong time of year, not yours to do), he hears why and the card says so. Example: "I'm putting you on a release project this winter" brings up "Start a release project for Lampkins? Both WR project spots are taken: this replaces Hall's." Do it, and it is on his Development tab and the promise counts as kept. The ask catch-all ("He agreed to...") goes: every ask either runs or says why not.
14. **Hints at what he wants.** A "What he's after" read in the left panel for every person, from your staff's read of him and what he has told you, vague at first and sharper as he opens up; Ideas for every kind of person, not only your players. A staff that reads him badly can be wrong; never his exact number. Example: Merez at the start "After: something for his players, a bigger name"; once he mentions camps "After: clinic and camp invites for his kids".
15. **Real haggling.** He opens at his ask and gives part of the gap each round, less each time, down to his walk-away number, at a pace set by his greed, patience and how much he wants it. He trades across terms (years or a buyout for salary, a title for money), can make the first number, can ask for time or walk away once his patience is spent. Coaches, agents and your own players on the same rules. Example: Donte asks 3.1; you offer 2.3, he says 2.9; you say 2.6, he says 2.8 over 4 years; you say 2.7 over 3 with his California buyout covered, and he takes it.

Lane B starts after lane A merges (it builds on items 3, 4 and 11).

## Already built elsewhere (not re-proposed)

The ear's reading of your line (PR #575, ear audit items 1 to 7), the landing words (#547), personal talk (#560), group talks (#561) and the recruit's visit answer (#563) fix how your line is read, not how his reply is written.
