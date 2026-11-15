# Ground round 1b: your AD's motives and the goal list (2026-11-15)

College only (Cain, 2026-11-15 16:16 UTC). This redoes round 1's questions 1 and 3 after Cain's answers of 2026-11-15 16:23 UTC:

> On question 1: "too limited, an AD would realistically have tons of motives. THat one 5 star talent a state might call generational, that one talent with an insane following/hype i.e a Bryce Underwood or Arch manning and make sure you coordinate with the new live talk changes and give it both chip but also the Live AI where the AD is an intellgent AI."
>
> On question 3: "not persé i'd prefer to have over 50 different recrutiing goals, some can be muli-year. (Get the state #1 2-3 years in a row) with a scaling judgement if i had 2 out of the 3)."

Agreed the same minute and on the Ground list: GR33, the hunt; GR34, bolder goals, read by the program's level ("a AD of an low-tier college should take it with a bit more salt then an LSU"). Cain agreed to all eight questions on 2026-11-15 16:45 UTC (GR35-GR42), with a note on the 8th that's answered in "A following" at the end. What's still proposed, not confirmed, is marked. Every "built" below was checked on main on 2026-11-15.

## What it does

**LSU, the day after signing day.** Your AD's office, 9 AM. He's pleased with the class and gets straight to it. "Everybody in this state knows the kid from Destrehan. Best player Louisiana has had in fifteen years. If he goes to Texas, I hear about it at every Rotary lunch until I retire." Two more things are on his mind: Coleman, the donor behind the new football building, wants the quarterback with 900,000 followers, and the president wants the state's best kept at home. You type: "Destrehan's leaning Texas. I can't promise him. Get Coleman and the collective behind him and I'll have him on campus twice before June." He thinks it over. "Coleman will put in for him. But if Texas gets him, I want the state's number one the next two years." You've just agreed a two-year goal in one sentence. With no key, the same talk runs on chips: take it as it is, argue it down, take it bigger for help, swap it for another, or ask him why.

**Georgia State, the same morning.** A different man with different worries. "A top-6 class in the Sun Belt. Kids who stay eligible; the president asked about the last two. And get me somebody from Atlanta the paper writes about." Nobody here expects the state's #1. If you promise it anyway, he smiles and takes it with salt (GR34).

**The Monday after, at the February meeting,** your coordinator adds the needs, and you pick your own goal from the list. At LSU you take the president's wish as your own: keep 8 of the state's top 10 home.

**Late August, the verdict.** The class signs in December, so it reads as a pace. Coleman's quarterback is committed to you: Done. Seven of the top 10 are staying home: Mostly. The kid from Destrehan is committed to Texas: a miss for now, and if he signs there, your AD's two-year goal starts in February.

## Where his motives come from

The AD raises his motives in his own words, read from the real world, never a menu.

| What he reads | What it makes him want | Built or new |
|---|---|---|
| His makeup | An ambitious AD wants names and rankings. A careful one wants a clean class that graduates. One with an ego wants the hype kid. A proud one wants the rival beaten. | Built: his traits (ego, patience, judgement, meddling and the rest, `folk/cast.rs`). The AD layer design (draft PR #386, still open) adds ambition, football first and media touch |
| His president | Brand first: reach and the big names. Academics first: grades and graduation. | New: the president is a person in the AD layer design |
| His donors | Each big donor's own want: the in-state star, the rival beaten, the collective's man, a kid from his old high school | Built: boosters' wants (`folk/cast.rs`: beat the rival, his old high school's players recruited, a clean program, win now). The AD layer design names 6-10 donors |
| The program | Its level, last class's rank, the rival's class, recent seasons, his trust in you | Built |
| This year's class in his state | A generational prospect, the state's #1, a kid with a huge following, a famous name, how deep the state is | Built: ranks, the state's top prospects (`state_top`), followers (`fame.rs`), a father who played (bloodlines). New: a famous father's name (question 8) |
| The fans | A class that fills the stadium | Built: his Crowds goal |

Real ADs and donors push this way. Michigan's donors put together the deal that flipped Bryce Underwood, the country's top recruit, from LSU in November 2024. Arch Manning's name alone made his 2022 commitment national news.

## His talk after signing day

- **When.** The day after signing day, in his office (round 1's day; Cain's answer was about what he wants, so the day stays). His reaction to the class you just signed opens it.
- **With a key.** A live talk on the new system (live talks v3, `docs/design/live-talks-persona.md`, section 6.9): the AI plays the AD as himself, and the game decides everything (v3's one rule). You can argue with facts, which the game checks ("the three schools above us in the rankings all spend more on recruiting"), trade one goal for another, ask for help (recruiting money, the collective, a donor's call), or ask why he wants it. He answers from his real motives and his trust in you (and from his real books once the AD layer is built). Boss meetings already play this way (lane N5c, `world/folk/persona/office/`). Bargaining the goals themselves is new (section 6.9 says so).
- **With no key.** The same talk on pick lists and chips (v3, section 13): take it as it is, argue it down, take it bigger for help (the built boss meeting's pattern), swap it for another of his motives, or ask why.
- **What comes out.** One or two goals he holds you to (question 1). His other motives stay wishes: meeting one lifts his trust a little, and missing it costs nothing.
- **One list for everything.** Each goal below becomes a measure in v3's catalogue (section 8.4), so the same goal can be a campaign goal, a promise to your AD, and a term in an assistant's deal (N5c already writes season goals into staff deals).

## Read by the program's level

- Every suggestion is sized to the program. Your coordinator and your AD pitch what the program can reach in a good year: a top-6 Sun Belt class at Georgia State, a top-5 national class at LSU.
- A bolder goal is read by level (GR34). At a low-tier program the AD takes it with salt: he doesn't count on it, so making it surprises him (a bigger gain) and missing it costs little. At LSU he takes it at its word: bold is the job there, so making it earns the normal gain and missing it costs more.

## The goal list (63)

Classes: how many recruiting classes it runs. "Later" means it's judged after the class plays (a freshman start, an honor, the draft): its slot comes back once the class signs, and the result comes back on your look-backs. Who raises it: who in the world pitches it to you. You can pick any of them yourself.

### The class

| # | Goal | Classes | Judged by | Who raises it |
|---|---|---|---|---|
| 1 | A top-N national class | 1 | The national class rank (built: class scores) | AD, president |
| 2 | A top-N class in the conference | 1 | The class rank in the conference (the built boss goal) | AD |
| 3 | Beat the rival's class | 1 | Your rank above his | AD, donors |
| 4 | The best class in the state | 1 | Above every program in your state | AD, president |
| 5 | N blue-chips in one class | 1 | Four- and five-stars signed | AD, you |
| 6 | A blue-chip roster | 3 | Over your last four classes, more four- and five-stars than two- and three-stars (Bud Elliott's blue-chip ratio) | You, AD at a contender |
| 7 | A top-N class three years running | 3 | Each year's rank | AD |
| 8 | Hold the class | 1 | No commitment lost before signing day | You |
| 9 | An early class | 1 | N commitments by the verdict | Coordinator |
| 10 | A class that fits | 1 | Every signee graded a fit at his spot by your staff (or by you, where you've looked) | You |

### Home and the state

| # | Goal | Classes | Judged by | Who raises it |
|---|---|---|---|---|
| 11 | The state's #1 | 1 | Sign your state's top-ranked prospect (built: `state_top`) | AD, president, donors |
| 12 | The state's #1, two years running | 2 | Each year's #1 | AD |
| 13 | The state's #1, three years running | 3 | Each year's #1 (Cain's example) | AD, you |
| 14 | Keep N of the state's top 10 home | 1 | Your fence, counted at the top 10 (built: L6 counts the top 25) | AD, president |
| 15 | The fence, three years running | 3 | N of the top 10 each year | AD |
| 16 | The all-state team | 1 | N first-team all-state signees (built honors) | AD |
| 17 | The city's best | 1 | N of your metro's top 5 | President, local donors |
| 18 | The hometown kid | 1 | The best prospect from your school's own county | Fans, president |
| 19 | Win back the state | 1 | More in-state signees than the program that led your state last year | AD, donors |
| 20 | Half the class from home | 1 | Your in-state share | President |

### The big names

| # | Goal | Classes | Judged by | Who raises it |
|---|---|---|---|---|
| 21 | A five-star | 1 | Sign one | AD, donors |
| 22 | The generational one | 1 | The best prospect your state has had in ten years, by rank (Cain's example) | AD, donors, president |
| 23 | The hype kid | 1 | The recruit with the class's biggest following (built: followers) (Cain's example: Bryce Underwood) | AD, president, collective |
| 24 | A famous name | 1 | A recruit from a famous football family (question 8) (Cain's example: Arch Manning) | AD, fans |
| 25 | The national #1 | 1 | The country's top-ranked prospect | AD at a blue blood |
| 26 | Two five-stars in one class | 1 | Sign two | AD at a contender |
| 27 | A commitment the country hears | 1 | His commitment is big or huge news (built: news loudness, but today a commitment is at most medium news; see "A following", question 1) | AD, president |

### Positions

| # | Goal | Classes | Judged by | Who raises it |
|---|---|---|---|---|
| 28 | A quarterback of the future | 1 | A top-10 quarterback in the class | AD, you |
| 29 | A quarterback every class | 3 | One each year your staff grades a future starter | You |
| 30 | An elite offensive tackle | 1 | A top-10 offensive tackle | You |
| 31 | Rebuild the line | 2 | A tackle, a guard and a center your staff grades future starters | Coordinator |
| 32 | A secondary class | 1 | Two corners and a safety graded starters | Coordinator |
| 33 | The best at your biggest need | 1 | The best player at your weakest spot on your staff's board | Coordinator |
| 34 | A freshman specialist | 1, later | A kicker or punter who starts as a freshman | Coordinator |
| 35 | Own a position in the state | 1 | Your state's top 3 at one position | You |
| 36 | An athlete who plays now | 1, later | A two-way athlete who plays as a freshman | You |
| 37 | Size inside | 1 | N defensive tackles past a measured size bar (built: bodies) | Coordinator |

### Battles

| # | Goal | Classes | Judged by | Who raises it |
|---|---|---|---|---|
| 38 | Head to head with the rival | 1 | Win N of the recruits you both offered | AD, donors |
| 39 | Flip one | 1 | A recruit committed elsewhere signs with you | You |
| 40 | Flip a blue blood | 1 | A blue-chip committed to a top-10 program signs with you | AD, donors |
| 41 | Into his backyard | 1 | N signees from the rival's state | You |
| 42 | Nobody flips | 1 | No commitment lost to a rival | AD |
| 43 | Out-recruit the champion | 1 | A class ranked above the conference champion's | AD |

### Your eye

| # | Goal | Classes | Judged by | Who raises it |
|---|---|---|---|---|
| 44 | The hunt (GR33, its own slot) | 1 | A find of yours committed to you or rising with the services | You |
| 45 | A find who starts | 1, later | An unranked signee who starts as a freshman | You |
| 46 | A two-star all-conference | 1, later | A two-star signee named all-conference in his first three seasons | You |
| 47 | An unranked kid drafted | 1, later | Drafted | You |
| 48 | A class that outplays its stars | 1, later | N of the class starting by their third year | You |
| 49 | Ahead of the services | 1 | N kids you offered before the services rated them who end four-stars | You |

### The program

| # | Goal | Classes | Judged by | Who raises it |
|---|---|---|---|---|
| 50 | A class with reach | 1 | The class's combined following (built) | President, collective |
| 51 | Every signee qualifies | 1 | No academic casualty (built: transcripts and the Grades goal) | President, AD |
| 52 | A class that graduates | 1, later | The class's graduation rate | President |
| 53 | A clean class | 1 | No signee with a character flag on your staff's read (built: the Clean goal) | AD, president |
| 54 | Inside the collective's pot | 1 | No deal past what the collective can pay | AD, collective |
| 55 | The donor's kid | 1 | The recruit a big donor asks for (built: a booster's old high school) | A donor, through the AD |
| 56 | The collective's man | 1 | The recruit the collective puts its money on | The collective's head |

### Ground and pipelines

| # | Goal | Classes | Judged by | Who raises it |
|---|---|---|---|---|
| 57 | Build a pipeline | 3 | N signees from one high school | You |
| 58 | Open a new state | 2 | N signees from a state you haven't signed from in five years | You |
| 59 | Take an area | 1 | Become its leader (GR30's ground goal) | You |
| 60 | Hold an area | 1 | Stay its leader | You |
| 61 | Open an area | 1 | From Stranger to Known | You |
| 62 | A junior college pipeline | 2, later | N junior college signees who start | You |
| 63 | Every coach signs one | 1 | Each assistant signs a recruit from his own area | Coordinator |

## Several years, and the scale

- **A goal that runs two or three classes holds your slot for each of them** (question 4). Each class's part is read as a pace at its August verdict and settles when that class signs.
- **The scale** (Cain's "a scaling judgement if i had 2 out of the 3"): every goal ends as a share, said in words. Done (all of it), Mostly (two-thirds or more), Partly (a third or more), Missed. Two of three state #1s is Mostly. Trust, your staff's mood and your name move with the share.
- **A near miss counts half on a goal with one target** (question 5): the state's #2 when the goal was the #1. A goal that counts already scales by its share (7 of 8 is Mostly).

## Your slots, and changing them

- **Needs:** your coordinator's pitch from the Needs map, from the position goals (28-37).
- **Your AD's:** one or two of his motives, from his talk. They change only by talking to him again, by his patience and his trust in you.
- **Yours:** any goal on the list, ground goals included (question 3). GR30 said one ground goal.
- **The hunt:** optional, its own slot (GR33).
- That makes three or four goals, plus the hunt (GR30 said three).
- **Dropping one of yours** (question 6): at any Monday meeting. It counts as a miss with whoever you told (your staff, and your AD or the press if you told them).
- **Picking a new one** (question 7): at any Monday meeting. It pays by how likely it looked when you picked it (the same idea as v3's season terms, `terms::likely`), so a July pick you've nearly met pays little. This replaces round 1's lock at the end of March.

## How it fits what's built

- **Live talks v3** (on main): the AD's talk is a v3 talk on N5c's office parts, with pick lists and chips when there's no key. The goals become measures in section 8.4's catalogue.
- **The AD layer design** (draft PR #386, open): it adds his ambition, his president and his donors. Until it's agreed, his motives read his built traits, the boosters' wants and the program.
- **The state's best** (built, `world/legacy/college.rs`): `state_top` ranks a state's prospects, and your fence (L6) counts how many of the top 25 you sign. Goals 11-15 read them.
- **Bloodlines** (built, `world/background.rs`): 1 kid in 25 has a father who played in college and 1 in 200 one who played in the NFL. It lifts his ceilings a little and makes staffs notice him, and his scouting report says so, but the father has no name. Your guys' sons (G4, built, `world/legacy/guys.rs`): a former player of yours may have a son coming through in his home state, with his name, leaning toward you. Question 8 takes this to the world's famous players.
- **Followers** (built, `fame.rs`): a recruit's fame comes from his stars and honors, times his Aura.
- **News loudness** (built, `calendar/events.rs`): small, medium, big and huge. Goal 27 reads big and huge.
- **Your ambitions** (G1, built) stay your career goals, up to three. The campaign's goals are recruiting goals for one to three classes.
- **The boss's goals** (`boss.rs`): his recruiting goal joins this list as goal 2, and the summer goals meeting keeps the rest of his goals.

## Questions (all agreed, Cain 2026-11-15 16:45 UTC: GR35-GR42)

1. Your AD holds you to one or two of his motives, and the rest stay wishes that only count for you when you meet them?
2. Take this list of 63 as the starting set, with rows added or cut any time?
3. Your own goal can be any goal on the list, ground goals included (GR30 said one ground goal)?
4. A goal that runs two or three classes holds your slot for each of them?
5. On the scale, a near miss counts half on a goal with one target (the state's #2 when the goal was the #1)?
6. You can drop one of your own goals at any Monday meeting, and it counts as a miss with whoever you told?
7. You can pick a new one at any Monday meeting, and it pays by how likely it looked when you picked it?
8. The world's famous players pass their name and its hype to a son or nephew coming through high school, like Arch Manning (new: today the fathers who played have no name, except your own former players)?

My pick: yes to all eight.

## A following (round 1c, after Cain's note on question 8; all four agreed, Cain 2026-11-15 16:58 UTC: GR43-GR46)

Cain, 2026-11-15 16:45 UTC: "Agree on all, but on 8th, players right now do have following but if memory serves me correct we don't really use it right now".

**What a following does today** (checked on main, 2026-11-15). Every player has one: his fame (his stars, his play, his honors, his market) times his Aura (`fame.rs`), and the screens show it. It feeds four small things:

- what NIL Go clears for a collective's deal with him (each follower raises the top a little, `nil_go_top` in `recruiting.rs`);
- a "brand deals" talking point in your pitch once he passes 25,000 (`folk/claims/program.rs`);
- a pro's chance of a social media storm, from 500,000 (`storylines/detect/weekly.rs`);
- how much a storyline about him weighs (`storylines/mod.rs`).

Nothing big reads it. Not who recruits him or how hard, not his own pick (every recruit weighs a program's brand exposure the same, as a quarter of its prestige), not his commitment's news (read from his stars only: a five-star's is medium news, never big), and not your program, its donors or its crowds. So goal 27 can't be met today, and the hype kid is only a number on his page.

**What it should do,** so the hype kid is worth chasing (agreed). Questions 2 and 3 change how the whole world recruits, so a build checks them on a world.

1. His commitment's news reads his following as well as his stars: the class's 10 biggest followings make big news and the very biggest huge news, the way Bryce Underwood's flip and Arch Manning's commitment led the national news?
2. His commitment lifts your program for a year, up to 3 points on its brand exposure grade for the class's biggest following, so the hype kid helps you recruit the next class? Travis Hunter's flip to Jackson State in December 2021 made an FCS program national news.
3. His own pick weighs a program's brand exposure by his following, up to twice as much for the class's biggest followings, since the stage is part of his business?
4. The spotlight cuts both ways: a college player or recruit with 500,000 followers gets a pro's chance of a social media storm (about 1 in 670 a week today, more with a temper)?

My pick: yes to all four.
