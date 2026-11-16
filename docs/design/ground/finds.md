# Ground round 6: finding and paying off (college, 2026-11-15)

College only (Cain, 2026-11-15 16:16 UTC). Round 6 of the offseason game (`offseason-game.md`, section 14): leads, blanks, finds, blooms, setbacks and the ground report. It builds on Ground's rules 1 and 2, the fog (GR2), the first find (GR31), the hunt goal (GR33), your eye in scenes (GR52), the May list (GR57), one memory per person (GR62) and the race (GR65). It also builds on what the game already has: tips and each tipper's record (`folk/tips.rs`, SK35, C14), the scouting file's looks (`world/scouting/file.rs`), the film room and kids who pop off another kid's tape, your own read and hints, look-backs and your eye (SK14), coverage (SK66), the hunt (`world/hunt/`) and the services (`recruiting.rs`). Cain agreed to the round 6 reply and its question on 2026-11-15 at 19:07 UTC (GR66, GR67). Built facts were read on main (cdfc3b1) on 2026-11-15.

## What it does

Every kid you find starts as a lead: somebody's word, or something your own eye caught. A lead has blanks, and you fill them by working: his tape, a live look, your camp, the transcript, his coach on his character. A kid becomes your find when you act first, with an offer while the world still rates him low. When the world catches up (stars, bigger offers, a starting job, all-conference, the draft), the find blooms, and it pays off. The coach who told you remembers you believed him, your man gets the credit, and your name as a finder grows among coaches and athletic directors. Things go wrong too, and you feel them: the kid who's smaller than his coach said, the find a rival flips, the miss from your own ground. Each August the ground report tells your year.

## Two of them

**Georgia State: the Columbus end.**
- May 2026 (round 2's week): your D-line coach hears from the Columbus coach about a sophomore end who grew three inches. It's a lead: the coach's word (Known, so his real list, and no record with you yet) and five blanks (measured size, tape, a live look, grades, character).
- June 13, your camp: he measures 6'4" and 245 and runs a 4.78. Two blanks fill. That night you watch his sophomore tape and your eye catches his first step.
- You offer him that week, a 2-star with no other offer. Now he's your find. He commits in the spring.
- A year later Auburn offers and flips him (round 5). Two years after that he makes all-SEC. He's still your find. The Columbus coach tells people you believed first, your D-line coach's record gets the credit, and your name as a finder grows.

**LSU: the August report.**
- Three finds signed, one already blooming: a Lake Charles tackle you offered as a 3-star with two offers is a 4-star now, with Alabama in.
- One setback: a Baton Rouge trainer's client measured 5'10" at your camp, not the 6'0" he'd said. The trainer's record with you drops to 1 of 5.
- One miss: a Monroe receiver nobody of yours saw signed with Louisiana Tech and caught 1,100 yards as a freshman. His coach is a Stranger to you, because nobody on your staff has been to his school since 2025. The report names the fix: north Louisiana needs a man.

## Leads

- **Where they come from, all from work:** the coach's list (by your standing, GR19), the camp walk-up, the track meet, the move-in, the other sport (the basketball forward who never played football), the trainer's client, the 7v7 circuit, a kid who pops off another kid's tape (built), and the paper (box scores, all-state lists, other staffs' offers), which reaches everyone at once and is crowded.
- **The lead card:** the kid; who told you and what he said, in his own voice; that person's record ("4 of his 6 kids held up"); the blanks; who else knows; and the order the news went (rule 2).
- **Filling the blanks (rule 1: each step costs hours and always fills its blank):** call his coach (character and grades, as true as the coach is straight), watch his tape (your film hours), see him live (an evaluation day or your camp), measure and time him (your camp), get his transcript (your academic staff or his counselor, GR62). The blanks are the built looks, so a lead becomes a scouting file.
- **No dice decide whether you hear.** Leads are the world's real kids and the people around them.

## Finds

- A lead becomes your find when you act first: an offer while the world still rates him low (his stars and his other offers). Hearing first gives you the head start, and acting is what counts (the question below).
- The find is credited to whoever's ground it was: the man who heard first, and you as the one who decided.
- Your first find (GR31): the junior safety on the south side coach's list becomes your find the week you offer him.
- A find you lose to a rival is still your find, and your class misses him. A find who never blooms goes in your eye's record as a miss (built look-backs).

## Blooms

When the world catches up, a find blooms, in steps (proposed):
- **A small bloom:** a fourth star, or an offer from a program a level up.
- **A bloom:** a starting job by his second year, or all-conference.
- **A big bloom:** drafted, or All-American.

The lower the world rated him when you acted, the more it pays: a 2-star with no offers counts more than a 3-star with five.

## What a bloom pays

- **The person who told you** keeps the moment ("You were the first to believe in my kid", GR62), and his word with you grows.
- **Your man** gets the credit: his record, his mood and his name. Rivals notice him too (round 7).
- **Your name as a finder** grows among coaches and athletic directors. Small-school coaches send you their late bloomers, and an AD weighs it when he hires, as one thing among many.
- **The news** names the program that believed first when the bloom is big ("the only FBS school that offered him": Buffalo and Khalil Mack, `placement.md`).
- **Your eye** grows from look-backs (built, SK14), your coach profile keeps the count, and the hunt goal (GR33) counts at the verdict. Today only a hunted kid pays like this, three years after you land him (`world/hunt/`, `look_backs`).

## Things go wrong

- **The oversold kid:** smaller or slower than his coach said. Your camp or the stopwatch shows it, and the tipper's record drops (built).
- **The find who busts:** a scholarship spent, and a miss in your eye's record.
- **The flip:** a rival takes your find (round 5). He stays your find, and your class misses him.
- **The grades:** a find who doesn't qualify goes to a prep school or a junior college (built paths), and you keep first call for when he's ready (`placement.md`).
- **The miss:** a kid from your ground nobody of yours saw blooms somewhere else (built coverage, SK66). The ground report names who knew (his coach) and why you didn't hear (your standing there).
- **The coach who stops calling** after you pull his kid's offer (GR24), and **your man poached** with his ties (round 5, round 7).

## The ground report

Each August, with the verdict (GR30): finds made, signed and blooming; misses; standing gained and lost by area; who outworked whom (in signs, GR65); and your men's records. Earlier finds keep blooming in later reports, through the draft.

## The long game

Finds that bloom grow pipelines (built: school, area and state pipelines in `world/program.rs`), your name as a finder follows you to your next job (GR29), and the storyline engine gets ground stories (the find who bloomed, the pipeline that broke).

## Under the hood

Today your staff's board fills from stars and footprint, and a kid popping off another kid's tape is the strongest finder (`offseason-game.md`, item 14). Ground's build replaces that for your staff and the AI (GR65), and doesn't sit beside it. What isn't public reaches you only through your ground, and the public view still shows everyone (GR2).

## What's built and what's new

| Built | New |
|---|---|
| Tips with each tipper's record (`folk/tips.rs`, SK35, C14) | The lead card: who, his word, his record, the blanks, who else knows, the order |
| The scouting file's looks (`world/scouting/file.rs`) | Blanks as the steps that fill those looks |
| The film room and kids who pop off another kid's tape (`film/`) | One finder among many |
| Your own read, hints, look-backs and your eye (SK14) | Finds, blooms and their payoff |
| Coverage of the kids nobody of yours saw (SK66) | The miss in the ground report, with who knew and why |
| The hunt and its look-back (`world/hunt/`) | Every find pays off, not only a hunted kid |
| Pipelines (`world/program.rs`) | Pipelines that grow from finds that bloom |
| Your staff's board from stars and footprint | Your board from your ground plus the public view |

The rest follows real life and the agreed rules, so it's taken as design: the lead sources, the blanks, the bloom steps, the setbacks, the ground report in August, and the long game. The bloom steps and payoff sizes are proposals, tuned in GR61's trial worlds.

## Question (agreed, Cain 2026-11-15 19:07 UTC: GR66)

**1. A kid becomes your find when you act first, with an offer while the world still rates him low, not when you hear first?**
Example: Georgia State hears about the Columbus end on a Tuesday in May and offers him after your June camp, a 2-star with no other offer. A year later Auburn flips him. When he makes all-SEC, he's still your find: the Columbus coach tells people you believed first, your D-line coach gets the credit and your name as a finder grows. If you'd heard first and never offered, the find would be Auburn's.
My pick: yes.
