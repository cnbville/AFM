# Ground: the offseason game (spec)

Spec written 2026-11-16 from the Ground list (`ground/list.md`, GR1-GR79), every item agreed by Cain between 2026-11-06 and 2026-11-15. Cain, 2026-11-15 23:47 UTC: "Okay start the build". The round files in `ground/` hold the full design and its examples (copied from the project folder `afm/ground/`). The nine agreed mock-ups are on the Design canvas https://claude.ai/artifact/1r3RLoGyRzyA46i6PjX7is; the redrawn boards 2b, 4b, 7b and 8b replace boards 2, 4, 7 and 8 there (files in the project folder `afm/ground/round8-boards/` and `afm/ground/round8-boards-redraw/`).

This file says what gets built, where it lives in the code and in which lane. It doesn't redo the design. Where a round file and this spec differ, the list wins, then the round file.

## What Ground is

Cain, 2026-11-13 22:56 UTC: "Right now, the game is in a wasteland from february to september. This scouting rework is supposed to act like an individual game. It needs to be incredibly immersive and hold new players attention through the dead-area."

From signing day to the first game you work your ground: the schools, the coaches and the people around the kids, the roads between them, and the rivals who want the same kids. You set goals in February and get a verdict in August. Every week has a beat, a plan, scenes and a result. People remember you. Rivals work the same ground by the same rules. Discovery comes from work, never from dice.

Three rules hold it together (`ground/ground-design.md`, "The promise"):
1. **Every hour buys something.** A school visit always gets the coach's list; a week in an area always moves your standing there. Luck decides only how good the kids turn out and when things happen to them.
2. **News goes in order.** When something happens to a kid, the people around him tell programs in order of their standing, and you always see the order and why you were where you were.
3. **Ground buys knowledge and trust, never a signature.** The kid still chooses (the built recruiting model), talent still beats ground, and potential stays hidden.

**Scope** (Cain 2026-11-15 16:16 UTC: "Let's purposely keep it to NCAA right now, im planning on doing a seperate NFL update."): college seats only, head coach and assistant (SK76). The NFL and high school seats wait for their own updates; their parts in the round files are notes. Every college program in the world works Ground by the same rules (GR65), so the AI's side is in scope.

## The parts

Each part names its items, what it is in one or two lines, and where it lives. "New" is a new store or module; everything else extends what's built.

### A. Areas and the map (GR1-GR8, GR11, GR70)

- **Areas the way staffs split states** (GR1): each state splits into named areas with their real nicknames and area codes ("the 757", "the Valley", "the DMV"). An area is a list of counties; every high school's area is its county's (`HighSchool.county`). The data is `data/real-world/ground_areas.json`, one row per area: state, key, name, nickname, area codes, counties, and its drawn outline for the map. It is built once by `tools/ground/areas.py` from the Census county shapes (us-atlas, public domain); a county not named in a row joins the nearest area in its state by centroid. Your staff use the names in letters and calls.
- **Fog** (GR2): the public view (rosters, stars, box scores) shows everywhere. What your ground knows (which kids are prospects, the coach's word) shows only where your standing has opened it. An area is fogged until you are Known with at least one person there.
- **Character** (GR3): each area's words come from the world's own data (the state's football culture, school sizes, money, how many staffs recruit it), learned as you work there. Until you are Known in an area it shows its public line only.
- **Travel** (GR4): drive time from campus to each school from the towns' real positions (`hs_towns.csv` latitude and longitude; road miles 1.25 × straight-line, 55 mph); a flight when the drive is over 5 hours. Schools a day: two to five in a city area and one or two in the country, by the area's schools per square mile.
- **The map moves** (GR5): the spring sweep's trails, June's camps, Friday nights' games, scores and breakout pings, from the world's own events.
- **History** (GR6): pins for your signees, your finds and the area's legends (the world's 30 years of draft picks and honors by high school), and rivals' pipelines drawn to their campuses (`Program.hs_pipeline`, `area_pipeline`).
- **Your roots** (GR7): your hometown and your old jobs are ground you hold on day one (your own ties, part B), and a kid from your hometown knows your name.
- **Owners' colors** (GR8): each area in its leader's colors, contested areas marked, your standing beside each. An area's level is its people together, weighted by the talent each controls (GR27).
- **Every school is a place** (GR11): its page shows its colors, stadium and coach, where its kids went, and how far back its tape goes (Hudl from 2006).
- **Code:** new `crates/afm-engine/src/world/ground/areas.rs` (the areas, the school-to-area map, travel), read by the map screen (part N).

### B. People and one memory (GR17, GR62, GR63)

- **Everyone on your ground is a person** (GR62): a face, a voice, an agenda, a nature and ties. High school head coaches and coordinators are built people (`world/coach.rs`). Trainers and 7v7 directors are built former players (`world/former.rs`, `Life::Trainer`, `Life::Camp`). Counselors, families, junior college and prep school coaches and other college staffs join as ground people (GR63). People are made when they matter (a kid on someone's board, a school in someone's ground), from the world's seed, so they're the same every time.
- **Each person has** an eye (how good his word is), straightness (how much he oversells), an agenda, a nature (loyal or self-serving, and how long he holds a grudge or a debt), what he wants from you, his ties and his life (`ground/people.md`, "What each person has"). You never see eye, straightness or nature as numbers: you learn them from his record ("4 of his 6 kids held up") and your staff's read.
- **One memory per person** (GR62), with every staff he deals with: standing and its why, your word (claims and how they held), favors either way in words, promises to him and his kids, how his kids fared, what you've talked through, and up to five moments in his words with their dates. Standing, talks, the lead card, the favor board and the AI staffs all read it. It takes over from the four stores that track this today (a contact's warmth, favors and record in `folk/network.rs`; the high school coach's credit in `scouting/hs.rs`; promises' trust in `folk/promise.rs`; standing). Your trail name stays what a stranger reads before he knows you.
- **Yours and the program's** (GR29): a memory is held by a coach (his own ties, which go with him) or by a program (its ties, which stay). A staff's standing with a person is the higher of its program's and its best current man's own.
- **People change** (GR62): high school coaching changes on January 20, the college carousel and the coach ladder are built; standing with a man goes with him; the new man at his old school starts from his own memory of you (Stranger, or Known if he sat in on your visits). Word travels to a man's close ties at about half its weight and to his area at about a quarter (proposed, not confirmed). A family keeps its memory for the younger brother; a cousin starts a level lower.
- **Code:** new `world/ground/people.rs` (who is a ground person and their makeup) and `world/ground/memory.rs` (the one memory), in a new `World.ground` store.

### C. Standing (GR17-GR29, GR53, GR54, GR60)

- **The ladder** (GR17): Stranger, Known, Trusted, Home, with each person. At college: high school coaches, trainers, counselors, families, junior college and prep school coaches and other college staffs.
- **A why on every level** (GR18), in words, from the memory ("Trusted: four visits this year, his Reyes started for you, you helped place Simmons at Elon").
- **What each level gets you** (GR19, GR16: every coach answers): Stranger, the public answer a day or two later; Known, his real list the same day; Trusted, the truth about a kid (his work, his family, the injury he plays through) and tape before you ask; Home, he calls you first, before the kid is public.
- **What builds it** (GR23): being there (visits, calls, his clinic, your camp), your word coming true, help, how his kids fared with you and your program's level, weighed by your man's skill with people and the person's own nature. A call from you counts by your name (GR53: at LSU about twice your assistant's visit, at Georgia State about one). A second visit to the same person in one week buys about half of the first (GR54).
- **How long it takes** (GR28): Known in a few visits, Trusted in a year or two of keeping your word, Home in three to five years, usually after a kid of his did well with you. These are the gates no amount of visits or favors skips (GR60).
- **How it fades** (GR24): slowly with time away and rivals' visits; sharply with a broken promise (an offer pulled, a kid buried after you said he'd play); your man leaving takes his own ties with him.
- **Warnings** (GR25): before a level drops you hear it, with the reason and the fix ("Columbus is slipping: no visit since March, Georgia was there twice").
- **Moving up is a moment** (GR26): each new level comes in that person's own words, at his hour ("From now on you hear about my guys first"), and the morning map shades the area.
- **Standing lives with people** (GR27): an area's level is its people together, weighted by how much talent each controls (his school's recruits by stars over the last four classes).
- **On day one** every program has ground from the world's 30 years: its pipelines (`Program.hs_pipeline`, `area_pipeline`, `pipeline`), its footprint and its coaches' ties (`Coach.ties`), so the map already has owners.
- **Sizes** (proposed, not confirmed; tuned in GR61's trial worlds and in play): see "Starting sizes" below.
- **Code:** `world/ground/standing.rs`.

### D. News in order (rule 2, GR16, GR19, GR20, GR64)

- **What starts news:** something real happening to a kid in the world: he grows (measured at school or a camp), has a big game (the engine's box score), runs a camp time, gets an offer from another program, moves in or transfers (June 15), qualifies academically, or gets a new coach.
- **Who knows first:** the people closest to him (his coach, his trainer, his family).
- **Who they tell, in order** (GR19, GR20, GR64): by standing level first. Within a level, a busy powerhouse coach answers by standing, then by nature: a loyal coach calls the staff he's closest to, a self-serving one the biggest program (GR64 refines GR20). A small-school coach answers everyone fast and pushes his kids. The public hears last (the services, then social media, faster from the 2010s).
- **Every lead shows the order and why:** "Auburn: Monday. You: Tuesday." The order is fixed from standing and nature; only the exact day carries a little noise.
- **Code:** `world/ground/news.rs`.

### E. Your week (GR12-GR15, GR47-GR54, GR56-GR58)

- **The meeting at the map** (GR47, GR71): Monday, 8 AM, your one weekly planner. Your recruiting coordinator drafts the week (each man's days and places drawn as routes, the calls, what you host, the film queue); you change any of it. Today's scouting assignments (`dept.rs`, `Dept.assignments`) and the film queue (`film/queue.rs`) become rows in it.
- **Road days** (GR48): you spend your staff's recruiting-person days yourself, by each year's rule book: 140 in the 2026 spring contact period (April 15 to May 23), up to 11 men off campus at FBS; one pool of 300 from January 1 to May 31 if the 2027 model passed. This replaces the built pace of 14 every Thursday for your seat (`dept.rs`, `college_day`). The fall's evaluation days (33) work the same way.
- **Trips** (GR49): every trip comes out of your recruiting money (`finance/college.rs`), shown as trips, not dollars: driving is cheap, a flight reaches a far area in a day, and your AD's raise buys more flights (`folk/staffing.rs`, `recruit_bonus`).
- **One ledger** (GR50): everything you do goes on the weekly ledger at its real time (a call 15 minutes, a school visit half a day, a junior day your Saturday), the same ledger film uses, with its tiredness (`film/time.rs`).
- **Three to five scenes a week come to you** (GR51), picked by your coordinator with his reason; he plays the rest. A played week takes about 10 minutes, a handed-off one about a minute.
- **The day** (GR12-GR15, on the built six-part day): 5 AM the morning map (what changed overnight); 8 AM Monday's meeting, other days the in-flight list (everything you're waiting on, with when it lands); 7 to 10 PM the night calls, each man in his own voice; a call that matters stops the day at its minute (a live moment), and a coach who barely knows you may not wait.
- **Every week has its beat** (GR56): the 35-week sheet in `ground/months.md` from each year's real calendar, and Monday's letter opens with it ("Week 16: the road opens Wednesday, the spring game is Saturday"). The recruiting desk's Monday digest becomes this letter.
- **The May list** (GR57): the day after Memorial Day, each man brings his list from the spring road; your coordinator ranks them against your goals, and the top gets your open official visit spots.
- **July's week off** (GR58): your tiredness goes to zero and your family gets the time; your coordinator holds the phones, and a call he takes counts as his.
- **Code:** new `world/ground/week.rs` (the plan, the road days, trips, the night calls, the letter) on `world/sections.rs` and `world/clock.rs`.

### F. Scenes (GR32, GR51, GR52, GR55, GR72)

- **The 15 scenes** in `ground/week.md` are the starting set (GR55): the meeting at the map, a school visit, a coach's call, a recruit's call, the night calls, junior day, an unofficial visit, your clinic, open practice, the spring game (hold it or skip it from 2027), an official visit weekend, your camp, the announcement, your state's coaches' convention, and film.
- **Every scene plays the same way:** the place (its time, the room, who's there); what you notice, rolled on your eye, or your man's on his visits (GR52); the talk, on live talks v3 with a key and chips without one, with talk facts keeping every line true; and what you walk away with (rule 1).
- **Skipped,** it runs on your man's skill with people and his eye and gives one line (GR32).
- **A school visit** (GR72, board 3): its stops in order (the sign-in sheet with who came before you, the coach's office, the weight room, the counselor), where the school's kids went, the talk with the coach, the new lead filling its blanks as the stops go, who heard first, and your ground with him.
- **Code:** `world/ground/scenes.rs` on the built scenes (`folk/scene.rs`) and live talks (`folk/persona/`, `crates/afm-game/src/live_talk/persona/`).

### G. Leads, finds and the ground report (GR66, GR67)

- **Leads come only from work** and the crowded public paper (GR67): the coach's list (by standing), the camp walk-up, the track meet, the move-in, the other sport, the trainer's client, the 7v7 circuit, a kid who pops off another kid's tape (built), and the paper (box scores, all-state lists, other staffs' offers), which reaches everyone at once.
- **The lead card** (GR73, board 4b): the kid, who told you and what he said, that person's record, the five blanks (measured size, timed speed, tape, a live look, grades and character), who else knows, and the order the news went. Filling a blank costs hours and always fills it; the blanks are the built looks (`scouting/file.rs`), so a lead becomes a scouting file.
- **A find** (GR66): a kid becomes your find when you act first, with an offer while the world still rates him low (his stars and his other offers), not when you hear first. A find you lose to a rival stays your find. It's credited to the man who heard first and to you as the one who decided.
- **Blooms** (GR67), in steps when the world catches up: a small bloom (a fourth star, or an offer from a level up), a bloom (a starting job by year two, or all-conference), a big bloom (drafted, or All-American). The lower the world rated him when you acted, the more it pays. A bloom pays the person who told you (his memory and his word), your man (his record, mood and name), your name as a finder (noteworthy, never driving), the news when it's big, and your eye.
- **Setbacks are felt:** the oversold kid, the bust, the flip, the grades, the miss from your own ground (built coverage, SK66), the coach who stops calling.
- **Your staff's board fills from your ground plus the public view,** replacing stars and footprint (`dept.rs`, `watch`). The public view still shows everyone (GR2).
- **The ground report** each August with the verdict (GR77): finds made, signed and blooming, misses, standing gained and lost by area, who outworked whom (in signs), your men's records, a setback, your AD's word and the paper.
- **Code:** `world/ground/leads.rs` and `world/ground/finds.rs`, on tips (`folk/tips.rs`), the file, film, look-backs and the hunt.

### H. The campaign (GR30, GR31, GR33-GR41, GR59)

- **Goals in February, a verdict in August** (GR30): at the February meeting (the Monday after signing day) your coordinator sets your needs, your AD's goals come from his talk, and you pick your own; you race them all spring; the verdict comes the week before the first game.
- **Your AD's motives** (GR35): read from his makeup, his president, his donors, the program, this year's class in his state and the fans. He sets one or two goals he holds you to in a talk the day after signing day (live talks v3 on the AI with a key, chips without one); the rest stay wishes that only count when you meet them.
- **The goal list** (GR36): the 63 goals in `ground/goals.md` in eight groups, a starting set. Your own goal can be any of them (GR37). The hunt is an optional fourth (GR33). A goal that runs two or three classes holds your slot for each (GR38).
- **The scale** (GR39): every goal ends as a share in words: Done, Mostly (two-thirds or more), Partly (a third or more), Missed. A near miss counts half on a goal with one target.
- **Changing goals:** drop one of your own at any Monday meeting, and it counts as a miss with whoever you told (GR40); pick a new one at any Monday meeting, and it pays by how likely it looked when you picked it (GR41).
- **Bold goals** (GR34): a bolder ground goal than your coordinator suggests pays more if you make it and costs a little of your staff's mood if you miss, and your AD reads it by your program's level (a low-tier AD takes it with salt).
- **Media days** (GR59): a goal you name there is public and judged by the press and the fans too, on the same scale.
- **What the verdict moves:** a little each, never a firing on its own: your AD's trust, your staff's mood, your name with the coaches on your ground.
- **Your first offseason** (GR31): guided by a person on your staff to a first find before signing day. In your first season Ground waits until the January portal window has closed. Tools open as you first need them.
- **Code:** `world/ground/campaign.rs` on the bosses (`folk/boss.rs`), the Needs map, the measures catalogue (`folk/terms.rs`, `folk/usage.rs`) and the AD's office talk (`folk/persona/office/`).

### I. A following and famous names (GR42-GR46)

- **Famous names** (GR42): the world's famous players pass their name and its hype to a son or nephew coming through high school, like Arch Manning.
- **News** (GR43): a commitment's news reads his following as well as his stars; the class's 10 biggest followings make big news and the very biggest huge news.
- **Brand exposure** (GR44): a commitment lifts your program's brand exposure grade for a year, up to 3 points for the class's biggest following.
- **His pick** (GR45): a recruit weighs a program's brand exposure by his following, up to twice as much for the class's biggest followings.
- **Storms** (GR46): a college player or recruit with 500,000 followers gets a pro's chance of a social media storm (about 1 in 670 a week, more with a temper).
- GR44 and GR45 change how the whole world recruits, so their lane checks them on a world (cost asked first).
- **Code:** `world/fame.rs`, `world/background.rs`, `calendar/events.rs`, `world/recruiting.rs`, `storylines/detect/weekly.rs`.

### J. The race (GR64, GR65)

- **Every rival staff works the same ground by the same rules** (GR65): the same road days, money, people, memory and favors. Its recruiters are people with a style (grinder, closer, salesman, straight shooter, from their makeup), their own ground, a history with you and a career.
- **You see the race in signs, never numbers:** sign-in sheets, the polo in the stands, a kid's visits and posts, the order the news went, what people tell you by their standing, the services' predictions, the map's owners and pipelines.
- **Moves both ways:** invade, defend, poach a man for his ground, outwork, flip a commit (a warning and one scene to fight for him), run the other down (a claim that settles on the facts; a false one costs whoever made it). Money outside the rules is rumors and a case if found. Ground's own work never buys a signature.
- **The AI plays Ground:** it plans its week by its needs, its ground and the threats it sees through the same signs; it knows only what its people know, so its board comes from its men's tips and looks and the services, not stars and footprint. That replaces today's board (`recruiting.rs`) and doesn't sit beside it.
- It changes how the whole world recruits, so its lane checks the world's recruiting against real (where blue chips sign, flagships holding their state, how often low-star kids are drafted, the FCS share of draft picks), on GR61's trial worlds.
- **Code:** `world/ground/race.rs` and `world/ground/ai.rs`, wired into `world/recruiting.rs`.

### K. Your staff's ground (GR68, GR69)

- **Each on-field assistant works areas on the map plus his position group** (GR68): the area man finds kids and builds the people; the position coach judges them and sells the role. Areas are set at the February meeting from your coordinator's proposal and can move any Monday. Home ground is fast and new ground slow; a big area can take two men; an empty one shows as misses in the ground report; a young coach who rides along with a veteran builds ground of his own. Today an area is a whole state (`dept.rs`, `Dept.areas`); Ground's areas are the map's.
- **Each man's ground** is his people and standing, his ties, his record and finds, his style, his week (road days, hours, tiredness) and his mood. Off-field staff stay on campus by each year's rule book.
- **Hiring trades teaching, scheme and ground:** a candidate's ground shows only in signs fogged by your network; his own ties come with him and his old program keeps its own; a high school coach can be hired for his area within the built limits (C17).
- **Losing a man** (GR76, board 7b): signs and a warning first, then a counter scene (a raise, a title or a promise, tracked like hiring promises). When he goes, his ties go with him and his commits waver (a warning and one scene each), and some follow him. A man who leaves happy joins your coaching tree; one who leaves angry runs you down.
- **The college assistant seat** (GR69, GR78, board 9): your area and your position group are yours. Your head coach's Monday plan gives you road days and film; you plan your own route. You bring him up to two kids a week on the built offer chain, he signs or says wait by his own read, the open spots and his trust in your eye, and a disagreement is a scene in his office. A kid he signs is your find; one he says wait on isn't, but the people who saw you push remember. He sets your area and position goals in February; you pick your own ground goal and may try the hunt; his August verdict counts in his staff review.
- **Code:** `world/ground/staff.rs` on `dept.rs`, `world/staff.rs`, `front_office/chain.rs`, `folk/staffing.rs` and `folk/hiring.rs`.

### L. Favors (GR22, GR60, GR61, GR63)

- **Placing kids builds standing** (GR22), grown into its own layer, an internal economy useful to a champion, a playoff team and a bottom-tier D1 team alike (`ground/placement.md`).
- **A favor buys one answer one level above your standing** (GR60), behind the staffs who already hold that level, and never faster than standing's minimum times.
- **Favors and your word show in words, never numbers** (GR63); other college staffs, junior college and prep school coaches and brokers get standing like everyone; your coordinator places the routine no's by a rule you set, and you take the Trusted and Home schools.
- **It ships switched off** (GR61). It turns on after a 30-year trial on three new seeds, each run with and without it, shows every tier gains on its own measure (LSU keeps its Home schools, a 10-2 team signs more transfers who start, Georgia State signs more kids who outplay their stars) while the top 10's share of the top 300 recruits doesn't rise. The trial also tunes the favor sizes. Until then, placing a kid works as it does today.
- **Code:** `world/ground/favors.rs`, behind a world switch that is off.

### M. The chain reaction (GR21)

- A high school whose kids do well in college and the NFL climbs: more staffs visit, more exposure, better kids come (move-ins, and transfers where state rules allow). A school that stops producing slowly falls back. New in the world: today a school's prestige is fixed and kids change schools only on your pitch.
- It changes the world, so its lane checks it on a world (cost asked first).
- **Code:** `world/high_school.rs` (prestige by year), `world/hs_year.rs` (move-ins and transfers on June 15).

### N. The screens (GR70-GR79)

A new **Ground** section in the sidebar (boards 1 and 2b): The map, This week, Leads, People, Rivals, Ground report.

| Screen | Board | Items |
|---|---|---|
| The map, Monday 5 AM | 1 | GR70 (with GR1-GR8): the state in its leaders' colors, your standing beside each area, your men's faces and routes, the overnight list, road days and your week's hours on top, and the area you pick: who leads it and how, your standing with its people, its people by name, your men there this week, the kids it sent you and the ones you missed |
| The meeting at the map | 2b | GR71: the coordinator's Monday letter, the board as a map of this week's fronts (numbered, with routes and rivals' dotted lines), each front's stake with the sign-in sheets against its rival, his one call as a gold card with what each answer costs, the road days as ticks, your week's hours, and the week day by day (drag a man to a day) with the scenes at their hour |
| A school visit | 3 | GR72 (part F) |
| The lead card | 4b | GR73: his face and growth (weight, height, the forty, fall to now), the services and his offers, the five blanks as a ring (who filled each and when; the open one lit with what fills it and its time), who's in his corner, the race in signs on a timeline against the window that closes when the services rate his class, your word (offer now, wait for his junior tape, fill the last blank first) with what each risks, the bloom steps with what each pays, and your man's record |
| A person | 5 | GR74: his face and life, your staff's read of him, his memory of you in his words (dated, the newest marked, the old fading), your word kept, favors owed, his kids with you, what you've talked through, what your level gets you and what the next would, his standing with every staff, and his ties |
| A rival | 6 | GR75: his face, program, areas, style (read from what he does), ground, ties and career; the race for a kid in signs, dated and numbered, each with where it came from; his claim and when it settles; your answers with their costs; where he works on the map; his history with you; your men against him |
| Your staff at the carousel | 7b | GR76 (part K) |
| The ground report | 8b | GR77: your offseason by name ("Your first spring") with the four goals as verdicts and why, your finds with faces and their bloom steps, your ground February to August by area (the headline area called out, lost ground marked), who outworked whom per area head to head with faces, in signs, the miss and its fix, a setback, your AD's word and the paper |
| The assistant's desk | 9 | GR78 (part K) |

**The look** (GR79): data screens (the week, the lead, your staff, the report, your desk) use the Press Box palette: dark blocks, ivory text, one gold accent for what needs your call, amber where something slips or falls short, a rival's color only on his own marks, and no saturated fills, glows or tinted backdrops. People and scenes (a visit, a person, a rival) use the st-room glass look with the place's or program's colors and drawn heads. Every screen is built on a person and what's at stake. Cain on the first redraw (2026-11-15 21:50 UTC): "colors feel childish, like i'm looking at a childrens book, it's chaotic"; the calmed redraws are the agreed look.

**Not confirmed, so not built:** the mock-ups' invented seniors' names (Cortez Banks, Tyree Mosley), the "an offer, any day?" marker on a rival's lane, the sign-in counts per front, the "how far it reaches" bars on the counters, and Wade's and Pryor's ratings. The sample world's people and numbers stand in for the world's own.

## Starting sizes (proposed, not confirmed)

The list says the sizes are proposals, tuned in GR61's trial worlds and in play (GR61, GR63, GR67, GR68). These are the starting values the build uses.

| What | Starting size |
|---|---|
| Standing points per person and holder | 0-100: Stranger 0-24, Known 25-49, Trusted 50-74, Home 75-100 (`ground-design.md`, part 2) |
| A school visit | +6 points, × his people skill (0.6 + 0.04 × relationships, 1-20) × the person's warmth to visitors (0.7-1.3 by nature) × the program's pull (0.9-1.1 by prestige: real, but less than the work) |
| A second visit the same week | half of the first (GR54) |
| A call from your head coach | a visit × his name (0.8 + 1.3 × prestige²: about 2 at LSU, about 1 at Georgia State; GR53); an assistant's call a third of a visit |
| Your clinic, your camp, the convention | +3 with each coach there, × scheme fit at the clinic (1.0-1.5) |
| Your word held / broken | +5 / -12, with a moment either way |
| A promise broken (an offer pulled, a kid buried) | -20 and a big moment held at full weight for about four years |
| His kid did well with you (started, graduated, drafted) | +6, +4, +12 |
| The gates (GR28) | Known needs 2 contacts; Trusted a year since Known with no broken word in it; Home three years since you met and a kid of his who did well with you (or five years) |
| Fade | from 60 days without contact, -1 a month, and -0.5 more a month for each rival staff that visited him more than you did |
| Warning | when the current rate drops a level within 30 days |
| Word travels | half its weight to his close ties, a quarter to his area |
| A favor | buys one answer one level up (GR60); its size is the trial's |
| Bloom pay | × 1.0 for a 2-star with no offers down to × 0.4 for a 3-star with five |

## The lanes

Cain, 2026-11-15 23:55 UTC: "make sure you plan individual threads for this build so we don't dump everything in one big thread with huge memory, use appropiate models and effort". So every lane below is its own fresh thread, started by the coordinator in dependency order, and each merges to main on green and sends one final reply (an agreed sequence). Game logic runs on Opus at high effort; screens from the agreed boards run on Sonnet at high effort.

Every lane:
- reads this spec, its part above and the round files it names, and nothing else "for context";
- adds or extends `docs/reference/how-it-works/ground.md` in the same PR (one page for Ground, a section per lane) and bumps its commit line;
- carries a `## Patch note` (or `Players: no`);
- keeps every result of the world the same per seed unless its row says it changes the world; a lane that changes the world runs no world before Cain's yes on its cost;
- adds to `World.ground` with `#[serde(default)]` fields, so old saves load; two lanes running side by side both add fields there, and the second to merge rebases;
- builds engine tests the way the box allows (4 cores, about 15 GB: `cargo test -j 2 -p afm-engine --lib --config 'profile.test.package.afm-engine.opt-level=0' --config 'profile.test.package.afm-engine.debug=0' -- <filter>`; never two engine compiles at once).

### The lane table

| Lane | Thread title | Items | Starts after | Model, effort | Changes the world? |
|---|---|---|---|---|---|
| G1a | Build Ground's areas and map data | GR1, GR3, GR4 (data and travel), GR6 pins' data | now | Opus, high | No |
| G1b | Build Ground's standing and one memory | GR7, GR17-GR29, GR62 (the store) | now | Opus, high | No |
| G8 | Build a following and famous names | GR42-GR46 | now | Opus, high | Yes: world check, cost asked first |
| G11a | Build the high school chain reaction | GR21 | now | Opus, high | Yes: world check, cost asked first |
| S1 | Build the Ground map screen | GR2, GR5, GR6, GR8, GR11, GR70, GR79 | G1a, G1b | Sonnet, high | No |
| G3 | Build Ground's week and road days | GR12-GR15, GR47-GR51, GR53, GR54, GR56-GR58 | G1a, G1b | Opus, high | No (your seat only) |
| G4 | Build news order, leads and finds | Rule 2, GR16, GR19, GR20, GR64, GR66, GR67 | G1b | Opus, high | No (your seat only) |
| G7 | Build Ground's people in full | GR18, GR26, GR62, GR63 | G1b | Opus, high | No |
| S2 | Build the meeting at the map screen | GR71 | G3 | Sonnet, high | No |
| S4 | Build the lead card screen | GR73 | G4 | Sonnet, high | No |
| S5 | Build the person screen | GR74 | G7 | Sonnet, high | No |
| G5 | Build Ground's scenes | GR32, GR52, GR55 | G3, G4 | Opus, high | No |
| G6 | Build the offseason campaign | GR30, GR31, GR33-GR41, GR59 | G3, G4 | Opus, high | No |
| G10 | Build your staff's ground and the assistant seat | GR68, GR69 | G3, G7 | Opus, high | No (your seat only) |
| S3 | Build the school visit screen | GR72 | G5 | Sonnet, high | No |
| S8 | Build the ground report screen | GR77 | G6 | Sonnet, high | No |
| S7 | Build the staff carousel and assistant desk screens | GR76, GR78 | G10 | Sonnet, high | No |
| G9 | Build the race and the AI on Ground | GR65 (with GR64 for the AI) | G3, G4, G7, G10 | Opus, high | Yes: the world's recruiting against real, cost asked first |
| G11b | Build favors, switched off | GR22, GR60, GR63 | G7 | Opus, high | No (off) |
| S6 | Build the rival screen | GR75 | G9 | Sonnet, high | No |
| G12 | Run the favors trial | GR61 | G9, G11b | Sonnet, high | Yes: six 30-year worlds, cost asked first |

Waves: G1a, G1b, G8 and G11a start at once. When G1a and G1b are in, S1, G3, G4 and G7 start. Then S2, S4, S5, G5, G6 and G10 as their rows allow, then S3, S8, S7, G9 and G11b, then S6 and G12.

### What each lane needs to know

**G1a, areas and map data (Opus, high).** Part A.
- Write `data/real-world/ground_area_seeds.csv` by hand: per state, the areas real staffs use, each with its name, nickname, area codes and one to three core counties (about 200 areas: Texas about 19, California about 11, Florida about 10, small states one). Georgia is the mock-up's 12 areas, with full county lists in the project folder's `afm/ground/round8-boards/tools/map.py` (`AREAS`, plus Russell, Lee and Chambers in Alabama for the Valley).
- `tools/ground/areas.py` builds `data/real-world/ground_areas.json` (each area's counties, center and label point; a county not seeded joins the nearest seed by centroid) and the drawn outlines for the map (`app/ui/art/ground/<ST>.json`: each area's fill path, its border arcs, and the projection, as in `map.py`: an arc shared by counties of two areas is a border).
- County shapes: us-atlas 3.0.1 (ISC; Census cartographic boundaries) from `https://registry.npmjs.org/us-atlas/-/us-atlas-3.0.1.tgz`. The proxy blocks jsdelivr and unpkg.
- 40 town rows in `hs_towns.csv` have county names the atlas spells differently (Virginia's independent cities "X city", Alaska's census areas, "La Salle", "Dona Ana", Shannon SD): match by name, then by the town's latitude and longitude to the nearest county centroid.
- New `world/ground/areas.rs`: the area table (one shared table from the data, not saved), each school's ground area (`HighSchool` gets a ground area field filled at build and on load), drive and flight times from campus, schools a day, and each area's character words from the world's data (GR3). The built `World.areas` (one per county, for area pipelines) stays as it is.
- Checks: unit tests (every school in an area, Georgia's split as the mock-up, travel times sane), the art files' size reported.

**G1b, standing and one memory (Opus, high).** Parts B and C, and roots (GR7).
- New `World.ground` store with `world/ground/people.rs`, `memory.rs` and `standing.rs`. A memory is keyed by person (a high school coach or coordinator, a trainer or 7v7 director from `world/former.rs`, a counselor made per school) and holder (a program, or a coach for his own ties: GR29). Sparse: only people someone has dealt with.
- Day one for every program from the world's 30 years: `Program.hs_pipeline`, `area_pipeline`, `pipeline`, `footprint` (`world/program.rs`) and each coach's `Coach.ties` (a tie to a school becomes a bond with its head coach today). Until G9, the AI's ground is re-read from those each year, so the world's recruiting is unchanged.
- Your seat: your roots (your hometown's coaches Known, your own high school's coach Trusted, your old jobs' schools Known), the points, gates, fade, warnings and moving up from "Starting sizes", the why in words, an area's level from its people weighted by talent (GR27; needs G1a's areas: if G1a isn't in yet, keep the weighting by school and let S1 roll it up by area).
- Your seat's visits today are the evaluation days in `scouting/dept.rs` (`college_day`), camps and visits to campus: count them as contacts now; G3 replaces how they're spent.
- Warnings and moving up post to your inbox at the person's hour (the six-part day, `world/sections.rs`).
- Checks: unit tests on the tiny seat world (`crate::tests::legacy::seat("college")`), `engine_hash` or a seed comparison showing the world plays the same, the store's size on `memory_check`.

**G8, a following and famous names (Opus, high).** Part I. Independent of the rest. Read `ground/goals.md`, "A following".
- Built today: `world/fame.rs` (fame times Aura), `nil_go_top` in `world/recruiting.rs`, the brand-deals point in `folk/claims/program.rs`, the pros' storm in `storylines/detect/weekly.rs`, news loudness in `calendar/events.rs`, bloodlines in `world/background.rs` and your guys' sons in `world/legacy/guys.rs`.
- GR44 and GR45 change how the world recruits: build, test, then ask Cain for a world check's cost (a new seed, not 51, 52, 74-96, 101, 111, 112, 121, 131 or 141) before running it.

**G11a, the chain reaction (Opus, high).** Part M. Independent of the rest.
- Today a school's prestige is fixed (`world/high_school.rs`) and kids change schools only on your pitch. Move-ins and transfers go on June 15 (`hs_year.rs`, `transfer_day`) by each state's transfer rule (`transfer_rule`).
- It changes the world: build, test, then ask Cain for a world check's cost before running it.

**S1, the map screen (Sonnet, high).** Part A's screen and GR70. Board 1 (render `afm/ground/round8-boards/renders/Main.png`, file `canvas/project/Main.dc.html`) and the school page (GR11). Adds the sidebar's Ground section. Data screens use the Press Box palette and people use the st-room glass look (GR79); the boards' `ground.css` is in `afm/ground/round8-boards/assets/`. See a screen in a browser with `cargo build --profile test -p afm-game --bin afm_play` and Playwright. A screens-only PR skips the engine in CI; its game commands go in `crates/afm-game` with a test.

**G3, the week and road days (Opus, high).** Part E.
- The built pieces it replaces or wraps for your seat: `scouting/dept.rs` `college_day` (evaluation days spent 14 every spring Thursday and the fall's over 12 weeks, assignments in turn, `second_schools` on the AD's raise, your June camps, visits as looks), `eval_limits`, `off_campus_recruiters`, the film queue (`film/queue.rs`), the ledger and tiredness (`film/time.rs`), the recruiting money (`finance/college.rs`), the recruiting desk's Monday digest (`crates/afm-game/src/lane_core/mail.rs`, `DIGEST_SPAN`), the six-part day and live moments (`world/sections.rs`), meetings that stop Continue (`folk/scene.rs`, `waiting`).
- The week's beats are `ground/months.md`'s 35-week sheet, read from each year's calendar (`calendar/anchors.rs`, `calendar/mod.rs` `recruiting_period`), never hard-coded dates.
- Scenes come as slots with the coordinator's reason; G5 makes them playable. Until then a scene is its one-line result.

**G4, news, leads and finds (Opus, high).** Parts D and G (not the report screen).
- Your board fills from your ground plus the public view, replacing stars and footprint for your seat (`scouting/dept.rs`, `watch`). Leads build on tips and each tipper's record (`folk/tips.rs`), the file's looks (`scouting/file.rs`), finds off another kid's tape (`film/finds.rs`), look-backs (`scouting/lookback.rs`), coverage (SK66) and the hunt (`world/hunt/`).
- Finds and blooms are recorded for the ground report (G6, S8) and pay the memory (G1b).

**G7, people in full (Opus, high).** Part B in full: GR18, GR26, GR62, GR63.
- The four stores that track what a person thinks of you read through the one memory: `folk/network.rs` (contacts, warmth, favors, record), `scouting/hs.rs` (`HsSide.credibility`), `folk/promise.rs` (trust, your names). Keep their saves loading.
- Families (`folk/ties.rs`: the decider, brothers), word travels, jobs that carry standing (the coaching changes on January 20, `world/clock.rs` `hs_carousel_day`; the college carousel, `world/staff.rs`), live talks reading the memory (talk facts, `folk/claims/`).

**S2, the meeting at the map (Sonnet, high).** Board 2b (`afm/ground/round8-boards-redraw/WeekB.dc.html`, `weekb.png`) on G3's commands.

**S4, the lead card (Sonnet, high).** Board 4b (`LeadB.dc.html`, `leadb.png`) on G4's commands.

**S5, the person (Sonnet, high).** Board 5 (`Person.dc.html`, `Person.png`) on G7's commands. Drawn heads are `app/ui/screens/heads/`.

**G5, the scenes (Opus, high).** Part F. The 15 scenes on the built scenes (`folk/scene.rs`) and live talks v3 (`folk/persona/`, `crates/afm-game/src/live_talk/persona/`), with chips and the line bank when there's no key. Your eye in a scene rolls the way film catches do (`film/`).

**G6, the campaign (Opus, high).** Part H and the ground report's data.
- Built pieces: the bosses (`folk/boss.rs`, the recruiting goal by class score), the Needs map (`crates/afm-game/src/lane_squadboards/needs.rs`), season terms and `terms::likely` (`folk/terms.rs`), the AD's office talk (`folk/persona/office/`, lane N5c), the state's best (`world/legacy/college.rs`, `state_top`) and the fence (L6), the takeover (`folk/takeover.rs`), the January portal window (`calendar/anchors.rs`, `january_window`).
- Each of the 63 goals becomes a measure in live talks v3's catalogue (`docs/design/live-talks-persona.md`, section 8.4).

**G10, your staff's ground and the assistant seat (Opus, high).** Part K.
- Built pieces: areas by state (`scouting/dept.rs`: `Dept.areas`, `set_area`, `area_coach`, `default_area_coach`) become the map's areas; the offer chain and overruling (`front_office/chain.rs`, `heart`); hiring and its counters, titles and promises (`folk/staffing.rs`, `folk/hiring.rs`); the carousel (`world/staff.rs`); the assistant's job (`world/seat.rs`; `foIsHead` on the screens).

**S3, the school visit (Sonnet, high).** Board 3 (`Visit.dc.html`) on G5's commands, in the st-room glass look.

**S8, the ground report (Sonnet, high).** Board 8b (`ReportB.dc.html`, `reportb.png`) on G4's and G6's commands.

**S7, the carousel and the desk (Sonnet, high).** Boards 7b (`StaffB.dc.html`, `staffb.png`) and 9 (`Assistant.dc.html`) on G10's commands.

**G9, the race and the AI (Opus, high).** Part J. The AI's board from its men's ground replaces today's (`world/recruiting.rs`: boards from reach, pipelines, footprint and stars); rival recruiters as people with styles; the signs; flips with a warning and a scene; claims about rivals on the built lie check; money outside the rules on `folk/finals.rs`. It changes how the world recruits: build, test on small worlds, then ask Cain for the check's cost (the world's recruiting against real: where blue chips sign, flagships holding their state, how often low-star kids are drafted, the FCS share of draft picks).

**G11b, favors, switched off (Opus, high).** Part L, on `ground/placement.md`, the built favors (`folk/network.rs`: `Favour`, `owe`) and `folk/backchannel.rs`, behind a world switch that is off.

**S6, the rival (Sonnet, high).** Board 6 (`Rival.dc.html`) on G9's commands, in the st-room glass look.

**G12, the favors trial (Sonnet, high).** GR61: three new seeds, each run 30 years with favors and without (six world builds, about an hour each), measured on GR61's tests (LSU keeps its Home schools, a 10-2 team signs more transfers who start, Georgia State signs more kids who outplay their stars, the top 10's share of the top 300 doesn't rise), and the favor sizes tuned. Cost asked first; the switch turns on only with Cain's yes on the result.

## Memory and speed

The 8 GB Mac is the target. The ground store keeps standing only for people someone has dealt with (sparse, keyed by person and holder), moments only for your seat's people and the few each AI staff needs, and the area data as one shared table. Each lane that adds to the store reports its size on `memory_check`, and engine speed-ups keep identical results per seed.
