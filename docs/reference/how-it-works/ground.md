# Ground: how it works today

Checked against branch commit 28f13b8 on 2026-11-16 (lane G1a, areas and map data), main bf92e7d plus lane G1b on 2026-11-16 (standing and one memory), and main d291de5 plus lane G7 on 2026-11-16 (people in full).

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/`, `app/`, `data/` or `tools/`. Ground's spec is `docs/design/ground.md`; each lane adds its section here.

## Quick answers

- How many areas are there? 208, every state and DC split the way staffs split them, from 1 (small states) to 20 (Texas) (`data/real-world/ground_areas.json`, `world/ground/areas.rs`, `table`).
- Where do the names come from? Hand-written seeds: name, nickname, area codes and one to three core counties per area; Georgia's 12 carry every county, as the mock-up (`data/real-world/ground_area_seeds.csv`).
- Which area is a school in? Its county's; a county the table doesn't know (or none) goes to the state's area whose center is nearest the town (`world/ground/areas.rs`, `area_for`).
- Is a school's area saved? No. `HighSchool::ground_area` is filled from the table when a world is built and when a file is read, so older worlds get it too (`world/ground/areas.rs`, `place_schools`; `world/mod.rs`, `World::empty`; `save.rs`, `read_world`).
- Can an area cross a state line? Yes, one does: Georgia's Columbus and the Valley takes Russell, Lee and Chambers in Alabama (`ground_area_seeds.csv`).
- How long is a drive? Straight-line miles × 1.25 road miles at 55 an hour (`world/ground/areas.rs`, `trip`).
- When does a man fly? When the drive is over 5 hours; a flight takes 2.5 hours on the ground plus the air at 450 an hour (`trip`).
- How many schools a day? 1 to 5 by how close the area's schools sit (`schools_a_day`).
- What are an area's words? Up to three, from the world's own data, each against the nation: talent per school, how many programs sign there, speed or linemen, school size, money and the state's football culture (`characters`).
- What are the map's pins? Your program's signees from the area, its legends (round 1-3 picks, All-Americans, the Hall) and the six strongest rivals' pipelines into it (`pins`). Your finds come with lane G4.
- Does any of this change the world? No. Nothing in the world reads it yet; every seed plays the same.
- Who is on my ground? At college: high school head coaches and coordinators, junior college coaches and other college staffs' men (world coaches), private trainers and 7v7 directors with their kids (former players), each school's counselor by name, each recruit's family and the prep schools' coaches (`world/ground/people.rs`, `Who`, `Kind`).
- Does a younger brother start from scratch? No. A family is one memory, keyed by the oldest brother the world knows; a cousin's family starts a level lower (`people::family_of`, `cousin_of`; `standing::from_cousin`).
- Does word get around? Yes: what happens with one person reaches his close ties in 2-5 days at half its weight and his area in 5-12 days at a quarter (`world/ground/word.rs`).
- What happens when a coach changes jobs? Your standing goes with him (the memory is keyed by the man), you get a memo, and the new head coach at his old school starts Known if he sat in on your visits or the man before him was Trusted with you (`world/ground/jobs.rs`).
- Does my old Contacts list still count? For people on your ground at a college seat, a contact's warmth, favors and tip record live in the memory; the contact shows a copy refreshed each day (`world/ground/stores.rs`, `sync`).
- What does the person screen read? `ground::card::card` (the `ground_person`, `ground_family` and `ground_school_people` commands): words only, never a hidden number.
- What are the levels? Stranger 0-24, Known 25-49, Trusted 50-74, Home 75-100 points (`world/ground/standing.rs`, `Level::of`).
- Who holds standing? A program (its ties stay) or a coach (his own ties, which go with him). A staff's standing with a person is the higher of its program's and its best current man's (`standing.rs`, `staff`, `holders`).
- What builds it today? Your staff's looks with recruits: an evaluation day at his school is a visit, a call is a call, your June camp is an event, a visit to campus is the host's call (`standing.rs`, `on_look`, hooked in `world/scouting/dept.rs`, `add`). Lane G3 replaces how the days are spent.
- How big is a visit? 6 points × the man's people skill (0.6 + 0.04 × relationships, 1-20) × the person's warmth (0.7-1.3) × the program's pull (0.9 + 0.2 × prestige) (`contact`).
- And a call? An assistant's call is a third of a visit. Your head coach's call or visit is a visit × his name, 0.8 + 1.3 × prestige² (about 1.85 at prestige 0.9, about 0.85 at 0.2) (`contact`).
- A second visit the same week? Half the first, then half again, down to a sixteenth (`contact`, `week`).
- How fast can it climb? Known needs two contacts; Trusted a year since Known with no broken word in that year; Home three years since you met and a kid of his who did well with you, or five years. Points above the gate are lost (`cap`).
- How does it fade? From 60 days without contact, 1 point a month, plus 0.5 for each rival staff that was with his kids more often than you in the last 30 days, checked on the first of each month (`month`, `rivals`).
- Do I get warned? Yes, on the first of the month when the month ahead would drop a level: a memo at his hour ("Columbus is slipping: no visit since March, Georgia was there twice ... A visit this month holds it."), the five schools with the most talent first (`month`, `slipping`, `WARN_MAX`).
- Does moving up show? Each new level comes as his letter at his hour, in his words, and he keeps it as a moment (`announce`, `up_words`).
- What does a pulled offer cost? 20 points with the kid's head coach (your program and your head coach), and a big moment held 3-5 years by his nature; while he holds it, being there buys a quarter (`word`, `pay`, `HELD`).
- Does the AI have ground (G1b)? Yes, read from the world's 30 years whenever asked: nothing is stored, so the world's recruiting is unchanged until lane G9 (`derived`).
- Do my roots count? Your own high school's coach starts Trusted, your hometown's coaches and the high schools you coached at Known, and a kid from your hometown knows your name (`seed`, `roots`, `knows_your_name`).
- What does an area's level read? Each school's head coach's standing, weighted by the talent the school sent out over the last four classes (`area_points`, `talents`).


## The areas (GR1)

- **The seeds** (`data/real-world/ground_area_seeds.csv`): one row per area: state, key, name, nickname, area codes and its core counties. A county from another state carries its state ("AL:Russell").
- **The tool** (`tools/ground/areas.py`): reads the seeds and the Census county shapes (us-atlas 3.0.1 `counties-10m.json`, downloaded from the npm registry into `tools/ground/.cache/` or read from a folder given as its argument). A county no seed names joins the area of the nearest seeded county in its own state, by centroid (great-circle miles). Independent cities in Virginia, Maryland, Missouri and Nevada are written "X city", as the town table writes them.
- **What it writes** (`data/real-world/ground_areas.json`): per area its counties (atlas spelling), center (area-weighted centroid), label point (the centroid of the member county nearest the center), square miles, real high schools (the town table's `slots` summed) and `near` (each school's own county's schools per 100 square miles, averaged over the area's schools). Also the town table's county spellings the atlas writes differently (nine: Alaska's census areas and boroughs, "La Salle", "Dona Ana", Shannon SD and one Arizona row), each with its area, matched by the town's place to the nearest county.
- **The engine's table** (`world/ground/areas.rs`, `TABLE`): read once from the JSON (built in with `include_str!`), shared, never saved. `area(id)`, `code(id)` ("GA-404"), `find(code)`, `in_state(state)`.
- **Each school's area** (`HighSchool::ground_area`, a `u16` index into the table, `#[serde(skip)]`). The built `HighSchool::area` (one per county, for area pipelines) stays as it was.

## The map art (GR1, for the map screen)

- `app/ui/art/ground/<ST>.json`, one per state and DC (51 files, about 540 KB together; Alaska 56 KB, Texas 37 KB, Georgia 21 KB): `areas` (each area's fill path, keyed "ST-key", with its home state; an area from another state drawn here too, as Georgia's Valley in Alabama's file), `borders` (arcs shared by two areas), `edge` (the outer edge) and `projection`.
- Projection: x = pad + (lon − lon0) × cos × k, y = pad + (lat1 − lat) × k, 800 wide (a state taller than 1,400 keeps that height), pad 10. Alaska's positive longitudes take 360 off (`wrap`).
- Paths are merged from the county shapes' shared arcs and each arc is simplified on its own (Douglas-Peucker, 0.6 px), so neighbors still meet. Islands under three points after simplifying draw only as edge.

## Travel (GR4)

- `trip(from, to)`: straight-line miles (`geo::miles_between`) × `ROAD_FACTOR` 1.25 at `ROAD_MPH` 55. Over `FLY_OVER_HOURS` 5 it is a flight: `FLIGHT_GROUND_HOURS` 2.5 + straight miles / `FLIGHT_MPH` 450. Atlanta to Columbus is a 2-hour drive; Atlanta to Miami a flight of about 4 hours door to door.
- `trip_to_school(w, program, school)` and `trip_to_area(w, program, area)` start from the program's campus (`geo::campus`); a school is at its town (`geo::place`).
- `Trip::words` says it plainly: "35 minutes away", "a 2-hour drive", "a flight, about 4 hours door to door".
- What it does not read: real roads, traffic, the time of day, the money a trip costs (lane G3).

## Schools a day (GR4)

`schools_a_day(area)`, by `near`: 6 or more, 5; 3, 4; 1.5, 3; 0.75, 2; under that, 1. In-town Atlanta and New York City are 5, Gwinnett 4, Southwest Georgia and Wyoming 1. The cut points are proposed, not confirmed.

## Character in words (GR3)

`characters(w)` gives each area a public line and up to three words. Facts come from `facts(w)`: the world's schools in the area (count, biggest class, private, enrollment, `area_money`), and the last four classes' three-star and better recruits, FBS signees and the programs that signed them (people past high school and alumni, by their high school).

- Public line: "53 schools, 31 in the biggest class. 22 FBS signees in the last four classes."
- Words, strongest first: "talent everywhere" (three-stars per school at 1.8× the nation or more) or "thin" (0.4× or less); "every school is recruited by everyone" (25 or more FBS programs signed there) or "a few programs have it to themselves" (3+ signees, 4 or fewer programs); "speed everywhere" or "big linemen" (RB, WR, CB and S, or OT, IOL, DT and EDGE, 8 points over the nation's share, with 5 or more recruits); "small towns and small schools, where the coach decides who gets seen" (mean enrollment under 500) or "big schools with big staffs and film on everyone" (over 1,800); "money and private schools" (mean money 0.62+ and a fifth private) or "working towns" (0.35 or less); "a football state" (the state in the top 10 by `StateInfo.culture`) or "football isn't the first sport here" (bottom 10).
- Showing the words only once you are Known there (fog, GR2) is the map screen's and standing's job (lanes G1b and S1).

## History pins (GR6)

`pins(w, program)` gives every area:
- `signees`: kids from its schools whose first college is the program (people past high school and alumni), newest class first.
- `legends`: alumni drafted in rounds 1-3 (round from the pick, 32 a round), All-Americans (`StatBook.honors`) and Hall of Famers, by fame (8 − round, +3 for a top-10 pick, +3 each All-America, +12 the Hall), up to `LEGENDS` 12.
- `pipelines`: each program's strongest `area_pipeline` (its county areas in this ground area) or `hs_pipeline` (its schools), 0.05 and up, the strongest `PIPELINES` 6, each with its campus for the line.
- Your finds join the pins with lane G4.

## Lane G1b: standing and one memory

Every size below is the spec's "Starting sizes" (proposed, not confirmed) unless it says it is the build's own pick, which is proposed, not confirmed too.

## People and their makeup

- `Who`: `Coach(id)` (a high school head coach or coordinator), `Trainer(id)` and `SevenOnSeven(id)` (former players), `Counselor(school)`. Families, junior college and prep school coaches and other college staffs come with lane G7 (`world/ground/people.rs`).
- Makeup, hidden and the same every run (from the world's seed, never its random stream): loyal 0-1 (a coach's built loyalty, 1-20, scaled; others from the seed), warmth 0.7 + 0.6 × (0.6 × a seeded draw + 0.4 × loyal), grudge 3-5 years, and his hour: a coach 1:00-1:50 PM (his planning period), a counselor 10-11 AM, trainers and 7v7 men 7-8:30 PM (`makeup`).
- A school's people: its head coach (who holds its kids), its coordinators (who sit in on visits) and its counselor (`at_school`, `head_coach_of`).

## One memory

- One record per person and holder, sorted, sparse: only your seat's holders keep memories (your program, you and your men), and only with people someone has dealt with (`world/ground/mod.rs`, `Ground`; `World.ground`, saved with `#[serde(default)]`).
- It holds: points, the day they met, the day it reached Known, the last contact and the last eight, contacts in all, this year's visits and calls, this week's contacts, the last broken word, whether a kid of his did well with you, his kids and how they fared (started, graduated, drafted, buried), where it started (day one from the program or a coach's tie, your hometown, your own school, an old job), up to five moments in his words, and the level last warned about (`world/ground/memory.rs`, `Memory`).
- Moments: the five he keeps are the big ones first, then the newest (`Memory::keep`). A big bad one holds 3-5 years (`holding`).
- Lane G7 adds: favors either way with their words, his tips' record with you and your word's with him, a staff's credit for a high school coach's word, what reached him through others (`heard`), where he worked when you last looked (`at`), and the network values last folded in (`seen`). See "Lane G7" below.

## Day one (GR7, part C)

- A program with a head coach: 100 × (1 − the product of (1 − part)) over its pipeline to the school × 0.8, to the area × 0.45, to the state × 0.25 and its footprint in the state × 0.2; capped at 74 unless the school itself is a pipeline (0.05 or more). A coordinator gets half. A counselor gets 60 × the program's pipeline to his school (`derived`, `program_part`).
- A coach's own tie to a school (`Coach.ties`, 0-1) is a bond with its head coach today: 85 × the tie (build's pick) (`derived`, `TIE_POINTS`).
- Read live: the AI's ground follows its pipelines and ties as the world changes them.
- Your seat, on the first day of a job: your program's and your own day-one people at Known or better are stored as old acquaintances (met five years ago; Known 180 days ago, or two years at Trusted), so they fade like any other from 60 days on. Your roots: your own high school's head coach at 55 points ("You're one of ours."), the other schools in your hometown at 30 ("You're a Columbus guy."), the high schools you coached at (your stops and your coach's history) at 35 (`seed`, `roots`). A new job drops the old program's memories and keeps your own (`seed`).

## Building standing

- Contacts: the man who made it builds his own tie in full and the program a third (build's pick, `PROGRAM_SHARE`); personnel staff with no man build the program's in full. One contact a day per holder, however many of his kids you saw. A coordinator who sat in gets half (`SAT_IN`). A call reaches the counselor too (`on_look`, `contact`).
- An event (your camp) is 3 points × the scheme fit (1.0 today) × his warmth, and doesn't halve in a week (`EVENT`).
- His kids with you (`kids`, `pay`): a starter last season +6 (February 1), a graduate +4 (May 20, his counselor too), a draft pick +12 (May 20, a big moment: "You got my Reyes drafted."). Paid to your program and your head coach, once each.
- Your word (`word`): a promise to a kid judged kept +5 ("You did right by him, like you said."); judged broken −20, or an offer pulled −20, each a big moment held 3-5 years, and a broken word resets the Trusted clock.
- The why in words: "Trusted: four visits this year, his Reyes started for you" from the memory, or the day-one source ("his kids have done well at LSU", "LSU's name in his area", "Coach Smith's own tie to his school") (`why`).

## What it does not do yet

- News in order of standing and the lead card: lane G4. The week's plan and road days: lane G3. The AI working its ground: lane G9. Favors: lane G11b.
- An area's level on lane G1a's areas: `area_level` (any list of schools: `area_points`, `leader`).

## Lane G7: people in full

Every size here is the spec's (word travels at half and a quarter) or the build's own pick, proposed, not confirmed.

### Who they are (GR62, GR63)

- `Who`: `Coach(id)` (any coach: a high school head coach or coordinator, a junior college coach, another college staff's man), `Trainer(id)`, `SevenOnSeven(id)`, `Counselor(school)`, `Family(oldest brother's player id)`, `Prep(index)`. New kinds are added at the end so stored lists stay sorted (`world/ground/people.rs`).
- `Kind`: high school head coach, coordinator, other high school coach, junior college coach, college coach (a four-year program), trainer, 7v7 director, counselor, family, prep school coach (`kind`).
- Makeup, hidden, the same every run: loyal, warmth, grudge and hour as lane G1b, plus his eye (a coach's recruiting rating; a former player's own `eye`; a counselor 8-11; a family 4-10; a prep coach 9-15), how straight he talks (a coach's keyed straightness, the same one the tips read, `folk/tips.rs`; a former player's own; a counselor 14-19; a family and a prep coach 6-16) and his agenda (his kids, his clients, his team seen, a kid who graduates, the family's wants, homes for his kids moving up, homes for his leavers). A family's loyalty leans on the kid's own. His hour: a college man 11 AM-noon, a prep coach 3-4 PM, families in the evening (`makeup`).
- Counselors: each school's counselors serve 4 to 14 years in turn; the name ("Ms. Alvarez") is made from the tenure's key with a surname at the job's national heritage shares (`counselor`, `made_name`).
- Families: a younger kid at the same high school with the same surname, one to six classes later, with an older brother in college (`older_brother_at`) or a family tie, reads his oldest brother's memory. A cousin (a family tie to a kid at another school) starts from the cousin family's points less 25. The decider speaks for it ("his mother", `family_voice`). What it wants: the truth, then its two strongest wants (`wants`).
- Prep schools: the real postgraduate programs (Fork Union, Hargrave, Milford, Bridgton, Avon Old Farms, Cheshire, Kimball Union, Tilton, Worcester, Valley Forge, IMG); a prep year's kid is at the one in his state, else one keyed to him (`PREPS`, `prep_of`).
- Trainers and 7v7 directors: a recruit with two stars or more has a trainer in his state, and a skill player (QB, RB, WR, TE, LB, CB, S) a 7v7 team, the same ones every time (`trainer_of`, `seven_of`); `clients` lists a man's kids.
- His ties (whom word reaches first): a head coach's coordinators and counselor and the trainers, 7v7 men and families of his best four kids; a coordinator's or counselor's head coach; a family's coach and trainer; a trainer's clients' coaches (`ties`).
- His life in words: years in the job, last season, the hot seat (three losing seasons), near retirement (62), wants a college job (ambition 13+ under 50); a former player's business; a counselor's years at the school (`life`).
- Faces: a coach's own recipe, a former player's own, everyone else a keyed recipe; heritage for the head (`looks`, `heritage`).

### Day one for the new people

- A family whose older son plays for the program: 40. A former player and his old school: 30; and a man who coached him: 40 (`derived`).
- A four-year college's man and you: your network's warmth × 100 (other college staffs get standing like everyone, read from the one tie) (`coach_part`).
- Your coaching tree (your mentor, the men who learned under you): 55, or 78 when he is loyal (14+); a former player of yours coaching high school 55, coaching elsewhere or training kids 40. Stored for you when a job starts (`seed`, `tree`).

### Contacts for the new people

- A call, a home visit or his visit to campus reaches the kid's family the way it reaches him; a junior college kid's look reaches his head coach there; a prep year's kid's look reaches his prep coach (a visit there skips his old high school coach) (`on_look`).

### His kids and your word reach the family

- A promise kept: +5 with his family ("You did right by our son, like you said."). An offer pulled or a promise broken: −20 and a big moment ("You lied to us."), held 3-5 years, so the younger brother hears it too.
- A son who started: +6. A son who graduated with you or was drafted from your program makes his family Home (`family_home`).
- A family keeps its memory for the younger brother: it doesn't fade with time and sends no warnings (`month`).

### Word travels (`word.rs`)

- Every payment with a moment (a kid who started, graduated or was drafted, your word held, a pulled offer, a broken promise) travels: to his close ties in 2-5 days at half (`TIES_SHARE`), to his area's head coaches in 5-12 days at a quarter (`AREA_SHARE`). A big moment reaches the area's 40 schools with the most talent (`AREA_MAX`), a small one only the people there who already know you.
- It travels from a coach (his family and counselor are paid with him, so they don't pass it on again), and word about one of his kids skips that kid's family, which was paid already.
- It reaches your program's memory, isn't a contact (it never opens a gate) and a loss is never gated. A close tie keeps it as a moment ("I heard what you did for Reyes."); anyone new to you gets it as the start of his why ("he heard what you did for Reyes").

### Jobs carry standing (`jobs.rs`)

- Each day the memory notes where each coach and counselor is (`at`, `people::post_key`). A coach who moved keeps every memory (it is keyed by the man); with your staff at Known or better you get a memo ("... has left Columbus for Valdosta State's staff. What you built with him goes with him: he can take the kids you pass on now.") and his why reads "followed him from Columbus".
- A high school head job he left waits for its next man (up to 150 days, `VACANT_DAYS`). The new man starts Known with each holder where he sat in on your visits there, or where the man before him was Trusted or better ("Coach Hill told him about you"); otherwise a Stranger.
- A counselor's tenure ends: the memory with that counselor goes, and the new one starts Known ("the school vouched for you") where you were Trusted with the last one.

### The older stores read through the memory (`stores.rs`)

- Your network (`folk/network.rs`): at a college seat, a contact who is a high school or junior college coach, a trainer or a 7v7 director has his warmth in your own memory (holder: you), his favor balance in `owes` and his tips' record in `his_word`. Each day `sync` folds in what the network's code changed since (a warmer tie: points × 100 inside the gates; a favor either way with its words from the ledger; a tip settled) and writes the memory back into the contact, so every older reader sees the memory. The first time, the contact's tie comes in as an old acquaintance. The network's own monthly cooling skips these people: they fade in the memory (`fades_in_memory`).
- The high school coach's credit with each college staff (`scouting/hs.rs`): kept as that staff's memory of you (`credit`, `set_credit`, `credits`), the same numbers; a file's old map moves in on load (`after_load`). These memories stay with you through job moves.
- Promises' trust (`folk/promise.rs`, `trust_part`): a recruit's family that knows you (Known or better) reads you from its memory, (points − 25) / 50 in the trail name's units, or at the bottom while it holds a broken word; your trail name until then.
- Live talks: a family's trust reads its memory (0.25 + 0.6 × points / 100, 0.15 while it holds a broken word), and a ground coach with no network tie reads your staff's standing (`persona/makeup.rs`, `trust`). His card in a talk lists where you stand and up to three moments he keeps, and what he knows on "his word" includes them (`persona/talk/items.rs`, `persona/knowing.rs`, `ground::card::talk_lines`).

### Moving up in his own words (GR26) and the why (GR18)

- Each kind says each new level its own way, and a self-serving head coach or trainer puts his price on it ("From now on you hear about my guys first. Don't make me regret it.") (`standing::up_words`). Each new level is kept 60 days in `Ground.ups` for the morning map.
- The why adds your tree, your old players, how he heard of you, favors in words ("he owes you a big one"), your word ("your word held up 4 of 5 times"), his ("4 of his 6 tips held up"), his biggest good moment, and for the new people their day one ("his brother plays for LSU", "he played at LSU", "you coached him") (`why`).

### A person's page (`card.rs`)

- `card(w, prog, who)`: name, role, kind, face recipe and heritage, level and why, who holds it, agenda, wants, life, ties with their levels, moments (newest first), his kids with you, favors in words with their list, your word, his word and your staff's read of it once three tips have settled ("his word holds up", "he oversells a little", "he oversells his kids"), his kids now, and his hour. No hidden number is on it.
- Game commands (`crates/afm-game/src/lane_ground_people.rs`): `ground_person` (`kind`: coach, trainer, seven, counselor, family, prep; `id`), `ground_family` (a recruit's player id), `ground_school_people` (a school id: its coaches and counselor, then the families, trainers and 7v7 men around its six best kids).

### What G7 does not do

- The favor board and spending favors (lane G11b, switched off), news in order and the lead card (G4), the week's plan (G3), the AI working its ground (G9). Counselors don't move to another school; they end a tenure.

## Where the knobs are

- `world/ground/areas.rs`: `ROAD_FACTOR` 1.25, `ROAD_MPH` 55, `FLY_OVER_HOURS` 5, `FLIGHT_GROUND_HOURS` 2.5, `FLIGHT_MPH` 450, `LEGENDS` 12, `PIPELINES` 6; the cut points in `schools_a_day`; the word thresholds in `characters`.
- `tools/ground/areas.py`: `W` 800, `PAD` 10, `TOL` 0.6 (the map art).
- `data/real-world/ground_area_seeds.csv`: the areas themselves.
- `world/ground/standing.rs`: `VISIT`, `CALL_SHARE`, `EVENT`, `WORD_HELD`, `WORD_BROKEN` (unused until claims), `PROMISE_BROKEN`, `STARTED`, `GRADUATED`, `DRAFTED`, `FADE_AFTER`, `FADE`, `FADE_RIVAL`, `PROGRAM_SHARE`, `SAT_IN`, `HELD`, `WARN_MAX`, `TIE_POINTS`, `STAR` (talent weights 0.2, 0.3, 1, 3, 8, 20 by stars).
- `world/ground/memory.rs`: `MOMENTS` (5), `FAVORS` (6).
- `world/ground/people.rs`: `makeup`, `counselor` (4-14 years), `PREPS`, the two-star line in `trainer_of` and `seven_of`, the brother rule in `older_brother`.
- `world/ground/standing.rs` (lane G7): `BROTHER` 40, `ALUM` 30, `OLD_PLAYER` 40, `TREE` 55, `TREE_HOME` 78, `COUSIN_DROP` 25, `up_words`.
- `world/ground/word.rs`: `TIES_SHARE` 0.5, `AREA_SHARE` 0.25, `AREA_MAX` 40, the lags in `spread`.
- `world/ground/jobs.rs`: `VACANT_DAYS` 150.


## Related pages

[recruiting.md](recruiting.md), [scouting.md](scouting.md), [people.md](people.md), [new-teams.md](new-teams.md), [staff-and-coaches.md](staff-and-coaches.md).
