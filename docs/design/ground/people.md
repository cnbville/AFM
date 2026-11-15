# Ground round 4: the people (college, 2026-11-15)

College only (Cain, 2026-11-15 16:16 UTC). Round 4 of the offseason game (`offseason-game.md`, section 14). It builds on standing (GR17-GR29), the favor economy (`placement.md`, whose six questions have been open since November 7) and rounds 1 to 3 (GR30-GR59). It also builds on what the game already has: the world's coaches and former players, contacts and favors (`folk/network.rs`, `folk/backchannel.rs`), straightness and agendas in tips (`folk/tips.rs`), promises (lanes P1 and P2), talk facts and live talks v3, and the coach ladder and the coaching changes (`world/staff.rs`). Cain agreed to the round 4 reply and both questions on 2026-11-15 at 18:50 UTC (GR60-GR63). The NFL's side waits for the NFL update. Built facts were read on main (cdfc3b1) on 2026-11-15.

## What it does

Everyone on your ground is someone. Each person has a name, a face, a voice, what he wants from you, how straight he is and who he's tied to. And he remembers you. One memory per person holds everything between you, and the whole game reads it: his standing and its why, what he says in a talk, the lead card, the favor board and the AI staffs' race. People change jobs, and what you built goes with the man. A family keeps its memory for the younger brother.

## Two of them

**Georgia State: two years with the Columbus coach.**
- April 2026 (round 2's week): you're straight with him that his senior corner isn't for you, and you get Mercer to look. Mercer signs him in May, his only scholarship. The coach owes you a big one.
- April 2027: a year of your word holding and a dozen calls later, he's Trusted. He says it his way, at 1 PM in his planning period: "I'll tell you what I really think of my kids." He brings up the corner every time you talk ("your boy at Mercer started six games").
- December 2027: he joins Valdosta State's staff (the pennant in his office said where his ties run). Your tie goes with him, and now he's a college staff that can take the kids you pass on. His old defensive coordinator gets the Columbus job. He sat in on your visits, so he starts at Known instead of Stranger.

**LSU: the Destrehan family.**
- Spring 2026: the state's best player leans Texas, and his mother decides (the built decider). You never oversell her. In May you tell her he'd start at corner for you only if he beats your senior, and that Texas sees him at safety.
- July 2026: he picks Texas. That fall Texas moves him to safety. She remembers who told her the truth, and by spring 2027 she's Trusted.
- February 2028: his younger brother, a sophomore, has letters from Texas and Alabama. You can't call a sophomore yet, but she calls you. She tells you what the other staffs don't know: he wants to stay close to home and play early.

## Who's on your ground

| Who | Built today | New | What he can tell you | What he wants from you |
|---|---|---|---|---|
| High school head coach | A full person: face, jobs, recruiting ties that travel with him, eye, straightness, warmth and favors | Standing, his nature, his moments, his life | His list, the truth about a kid, the tape | To be asked, the truth about his kids, help placing his lesser kids, a clinic talk, someday a job |
| His coordinators and strength coach | Staff counts; some are former players coaching | Full people | Who works, who's hurt, the weight room numbers | Notice, a step up the ladder (a graduate assistant spot, a staff job) |
| Private trainers | Former players in their lives now (`Life::Trainer`), with eye and straightness; a kid's trainer as a source (eye 11) | Clients, an agenda, standing | His clients' work and real speed | His clients seen, his business known |
| 7v7 directors and camp men | Former players (`Life::Camp`); camp tips (C14) | A team, an agenda, standing | Who's rising on the circuit | His team seen, offers for his players |
| Counselors and teachers | A teacher as a source (true, eye 9) | Full people | Grades, the transcript, the core courses | A kid placed where he'll graduate |
| The family | The decider and his wants; brothers as ties | The family as people with one memory | What the kid really wants | The truth, a degree, playing time, home close by |
| Junior college and prep school coaches | Junior college coaches in the world; a prep school year, with no coach | Prep school coaches as people; standing and favors | Late bloomers, academic cases | Homes for his kids moving up, your no's to develop |
| Other college staffs | Coaches in the world, contacts | Standing and favors (the favor economy) | Their leavers, their no's | Looks, homes for their kids |
| Your coaching tree | Your mentor, the coaches who learned under you | Standing and moments | What they hear | A job, a word |
| Your former players there | Old teammates and players you coached (C13) | Standing and moments | The area and its schools | To be remembered |

## What each person has

- **Who he is:** a name, an age, where he's from and his jobs. A face: built for coaches (`Looks::coach`), and the same keyed recipe for everyone else.
- **His eye:** how good his word on a kid is (built: a coach's recruiting rating, a former player's `eye`).
- **His straightness:** how much he oversells (built: tips, a former player's `straight`).
- **His agenda:** what he pushes. A coach protects his kid, a trainer his paying client, a 7v7 director his team's exposure, a counselor a kid who'll graduate, a mother what the family wants.
- **His nature:** loyal or self-serving (GR23), and how long he holds a grudge or a debt.
- **What he wants from you:** to be asked, the truth about his kids, a clinic talk, help placing his lesser kids, a job someday.
- **His ties:** to programs (his old school, his old boss) and to the other people around his kids (the trainer who works them, the 7v7 team they play for).
- **His life:** his job and how safe it is, his season, and what he's after next (a college job, a state title, retirement).
- **His voice:** his own words in live talks with a key, and the line bank's lines for his kind and nature without one.

You never see his eye, straightness or nature as numbers. You learn them from his record ("4 of his 6 kids held up") and your staff's read, the way potential stays hidden.

## One memory per person

What it holds:
1. His standing with you and with every other staff (GR17), with its why (GR18).
2. Your word: your claims about kids and how they held up ("4 of 5 held up", `placement.md`).
3. Favors either way, in words (the built balance).
4. Your promises to him and to his kids, kept or broken (built: lanes P1 and P2).
5. How his kids fared with you: played, graduated, or buried and gone (GR23).
6. What you've talked through (built: `Folk.talked`).
7. The moments he keeps: about five, each in his words with its date ("You came when we were 2-8", "You pulled my tackle's offer in January"). A big one holds at full weight for about four years, then fades by his nature. DeSoto kept Washington State off its campus for nearly four years after a pulled offer (`placement.md`). What you owe never fades.

Who reads it: standing and its warnings (GR25); live talks, where he brings up his moments and the AI only words what's there (the one rule); talk facts; the lead card's order (rule 2); the favor board; the takeover file at a new job; and the AI staffs (round 5).

What it replaces: today four things each keep their own track of what a person thinks of you. They are a contact's warmth, favors and record (`folk/network.rs`), the high school coach's settled view of each college (`scouting/hs.rs`), promises' trust and your trail name (`folk/promise.rs`, `Mine::names`), and standing (agreed, not built). They become one record per person. Your trail name stays what a stranger reads before he knows you (C10).

## How people change

- **Jobs.** High school coaching changes come on January 20, college jobs move in the carousel after rivalry week, and a high school coach with buzz climbs to a college staff (all built: the coach ladder). Coaches' recruiting ties already travel with them. New: your standing with him goes with him too. The new coach at his old school starts from his own memory of you: a Stranger, or Known if he sat in on your visits or heard about you from the man before him.
- **New people.** Former players open gyms and start 7v7 teams (built: their lives now), counselors change schools, and kids transfer (June 15, built).
- **Word travels.** People talk. What you did with one person reaches his close ties within days at about half its weight, and his area at about a quarter (proposed; the bad placement in `placement.md` has the shape).
- **Their own years.** A coach's title run, his hot seat, his wish for a college job and his retirement reach you as news through your ties, and they change what he needs from you.

## Families

- In many homes the family decides more than the kid does (built: the decider). Standing with a family follows the recruiting calendar's contact rules, like the kid's.
- The memory stays after the kid signs, wherever he signs. The younger brother (built: brothers) starts where the family left you, and a cousin starts a level lower. A son who played for you and did well makes his family Home.
- Lose a kid honestly and the family can still be Trusted. Lie to them and the whole family remembers.

## Your coaching tree and old players

- Your mentor and the coaches who learned under you (built: `Coach.mentor`) start at Trusted, or Home if they left you on good terms. They trade favors with you, take your no's, and recruit against you (round 5).
- Former players living on your ground (built: C13) start at Known, and one coaching high school starts at Trusted.
- Kids you placed become people in your book (`placement.md`).

## The favor economy at college

`placement.md` holds the full design. Its college parts: champions place their no's and leavers, playoff programs buy the truth, and the bottom tier trades looks, places and early snaps. You earn a favor by giving something scarce (a look, a home found, a claim that holds, help). A favor buys one answer one level up. What you owe never fades, and nothing buys a kid's choice.

Its six questions:
- Its questions 1 and 6 are asked below, as questions 1 and 2.
- Its question 2 (the ledger in words, never numbers) follows GR18 and your plain-words rule, so it's taken.
- Its question 3 (the sizes) stays proposals, tuned in the trial of question 2.
- Its question 4 (more people on the ladder) follows one memory per person. Every person keeps the same memory, so other college staffs, junior college and prep school coaches and brokers (trainers, 7v7 directors, NIL handlers) get standing like everyone.
- Its question 5 (your coordinator places the routine no's by a rule you set, and you take the Trusted and Home schools) follows GR47 and GR51.
- The NFL parts wait for the NFL update.

The money side of brokers (a handler who wants paid for his kid) belongs to round 5, the race, with the NIL lanes.

## What's built and what's new

| Built | New |
|---|---|
| Coaches as full people: faces, jobs, recruiting ties that travel (`world/coach.rs`, `looks.rs`, `staff.rs`) | Standing and one memory per person |
| Former players as trainers, 7v7 men, high school assistants and reporters, with eye and straightness (`world/former.rs`) | Clients, teams and agendas; faces from the same recipe |
| A teacher as a source (`scouting/own.rs`) | Counselors and teachers as people |
| The decider, and brothers as ties (`folk/ties.rs`) | The family as people with one memory |
| Contacts, favors, the record and asks (`folk/network.rs`, `folk/backchannel.rs`) | Favors in words, the favor board, earning and spending by `placement.md` |
| Promises and your three names (`folk/promise.rs`) | Promises to coaches about their kids, read from the same memory |
| The coaching changes and the coach ladder (`world/staff.rs`, `world/clock.rs`) | Your standing travels with the man; the new man starts from his own memory |

People are made when they matter (a kid on someone's board, a school in someone's ground), from the world's seed, so they're the same every time. The build measures them against the 8 GB Mac.

## Questions (both agreed, Cain 2026-11-15 18:50 UTC: GR60, GR61)

**1. A favor buys one answer one level above your standing, behind the staffs who already hold that level, and never faster than standing's minimum times?**
Example: Georgia State is Known with the Columbus coach, and he owes you a big one for the corner Mercer signed. You spend one on the sophomore end. He tells you what he'd tell a Trusted staff (the kid's work, his family, the shoulder he plays through), but Auburn is already Trusted there, so Auburn heard it Tuesday and you hear it Wednesday. You still can't be Trusted with him before a year of keeping your word. Today a favor buys a name for your staff, the grapevine, or a word for one of your players.
My pick: yes.

**2. The favor layer ships switched off, and turns on after a 30-year trial on three new seeds shows it helps every tier without making the top richer?**
Example: each seed runs 30 years twice, with and without favors (six world builds, about an hour each). It turns on if LSU keeps its Home schools, a 10-2 team signs more transfers who start, and Georgia State signs more kids who outplay their stars, while the top 10's share of the top 300 recruits doesn't rise. Until then, placing a kid works as it does today.
My pick: yes.
