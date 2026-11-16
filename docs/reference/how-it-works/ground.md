# Ground: how it works today

Checked against branch commit 28f13b8 on 2026-11-16 (lane G1a, areas and map data), main bf92e7d plus lane G1b on 2026-11-16 (standing and one memory) and main d291de54 plus lane G3 on 2026-11-16 (your week and road days).

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
- Who is on my ground? At college: high school head coaches and coordinators (world coaches), private trainers and 7v7 directors (former players, keyed only so far) and each school's counselor (`world/ground/people.rs`, `Who`).
- What are the levels? Stranger 0-24, Known 25-49, Trusted 50-74, Home 75-100 points (`world/ground/standing.rs`, `Level::of`).
- Who holds standing? A program (its ties stay) or a coach (his own ties, which go with him). A staff's standing with a person is the higher of its program's and its best current man's (`standing.rs`, `staff`, `holders`).
- What builds it today? Your staff's looks with recruits: an evaluation day at his school is a visit, a call is a call, your June camp is an event, a visit to campus is the host's call (`standing.rs`, `on_look`, hooked in `world/scouting/dept.rs`, `add`). Since lane G3 the days are the week's plan (below).
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
- Not on it yet: your claims about kids, favors, promises' trust and what you've talked through. Lane G7 moves `folk/network.rs`, `scouting/hs.rs` and `folk/promise.rs` onto it.

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

- Families, word travelling to a man's ties, jobs carrying standing in full and talks reading the memory: lane G7.
- News in order of standing and the lead card: lane G4. The AI working its ground: lane G9. Favors: lane G11b.
- An area's level on lane G1a's areas: `area_level` (any list of schools: `area_points`, `leader`).

## Where the knobs are

- `world/ground/areas.rs`: `ROAD_FACTOR` 1.25, `ROAD_MPH` 55, `FLY_OVER_HOURS` 5, `FLIGHT_GROUND_HOURS` 2.5, `FLIGHT_MPH` 450, `LEGENDS` 12, `PIPELINES` 6; the cut points in `schools_a_day`; the word thresholds in `characters`.
- `tools/ground/areas.py`: `W` 800, `PAD` 10, `TOL` 0.6 (the map art).
- `data/real-world/ground_area_seeds.csv`: the areas themselves.
- `world/ground/standing.rs`: `VISIT`, `CALL_SHARE`, `EVENT`, `WORD_HELD`, `WORD_BROKEN` (unused until claims), `PROMISE_BROKEN`, `STARTED`, `GRADUATED`, `DRAFTED`, `FADE_AFTER`, `FADE`, `FADE_RIVAL`, `PROGRAM_SHARE`, `SAT_IN`, `HELD`, `WARN_MAX`, `TIE_POINTS`, `STAR` (talent weights 0.2, 0.3, 1, 3, 8, 20 by stars).
- `world/ground/memory.rs`: `MOMENTS` (5).
- `world/ground/people.rs`: `makeup`.


## Lane G3: your week and road days

Every size here is the build's pick unless the spec gave it, proposed, not confirmed. Code: `crates/afm-engine/src/world/ground/week/` (`mod.rs` the store and the day, `beat.rs`, `plan.rs`, `road.rs`, `phone.rs`); game commands `crates/afm-game/src/lane_groundweek.rs`. Only a college seat has a week; no other program reads it, so the world plays the same on every seed.

### The day (on the six-part day)

- **5 AM, the morning map** (GR15): levels reached and lost since yesterday's morning with everyone your seat keeps a memory with, yesterday's road by man ("Davis in Gwinnett: 4 schools, 7 recruits seen") and calls you missed (`phone::morning`; `Week.overnight`).
- **Monday, the week turns** (GR47): the coordinator drafts the plan and writes the Monday letter (`plan::turn`). The plan is drafted the first time it is asked for in a week too (`plan::ensure`).
- **The in-flight list** (GR12): your men on the road (their night calls), coaches calling back with their hour, your staff's film queue with its date, your asks for a live look, what you host this week, the May list (`phone::in_flight`).
- **7 to 10 PM, the night calls** (GR13): each man out that day, 25 minutes apart from 7 PM, in his own words: his schools, who he saw, where he stands with the coaches (`phone::night_call`).
- **Live moments** (GR14): a coach calling back who matters (his school has a 3-star or better or your offer in the next two classes, or he's Trusted) and a night call that matters (a coach reached Known or better with him, or he saw a 4-star nobody of yours had seen in person) stop the day at their minute (`phone::moments`, read by `World::live_moments`).

### The plan (GR47)

- **Who goes:** your on-field staff (coordinators and position coaches); the head coach only in January's and December's contact periods and at FCS in the spring; never the FBS head coach in the spring window (`road::may_go`).
- **Where:** your assignments first (a player's school, a state's neediest area, the area and schools with a position group's kids, the cross-checks' schools), then your asks for a live look (to the kid's position coach), then each man to the area of his recruiting states (`Dept.areas`, else the staff's split) that needs him most: each board recruit by stars (0.5, 0.5, 1, 2, 4, 8), a quarter for one already seen in person this recruiting year. An area takes a second man past 12 of need.
- **How many days:** the window's pace: the days left × this week's open days / the window's open days ahead (school days in the spring and January, Thursday and Friday in the fall), up to 3 days a man (2 in the fall), Tuesday first (Monday is the meeting), never more men out a day than the rules allow. A flight goes only while the week's money covers one.
- **Your changes:** `set_road` (a man, an area, the days; checked against the window, the head coach's spring, the men at once and the flights), `clear_road`, `hand` (the week to your coordinator: only live moments stop you).
- **Your calls** (GR53): up to 5 a week: coaches slipping (a warning out), a Stranger one contact from Known with a kid on your board, a coach with a 3-star or better in an area nobody of yours reaches this week. The ones you don't make, your coordinator makes on Saturday as his own (`plan::coordinator_calls`).
- **The scenes** (GR51): the meeting, an official weekend, a junior day, your camp, unofficial visits, your calls and your own road days, ranked (the meeting, officials, junior day and camp, a slipping coach's call and your road, then the rest); the top five are yours, the rest your coordinator's. Until lane G5 each is its one-line result (`plan::draft_slots`; the game fills the line from the log).

### Road days (GR48) and trips (GR49)

- **The windows:** the spring (contact, or evaluation before 2024, in April to June), the fall evaluation period (September to November) and the winter contact periods (`road::window`). The days come off `Dept.eval_days` as the built ones did, so the department screen's squares still count them.
- **A man's day:** his focus schools first, then the schools with the most board need, then the area's schools by talent where he stands lowest with the coach; a school he saw this week goes last (GR54: a second visit buys half anyway, in `standing::contact`). Each board recruit there gets a look (practice; a fall Friday's game), and the coach's time counts through `on_look`; at a school with none on the board the coach and his coordinators get the visit directly.
- **The money:** see the quick answers. With no books yet the tier's usual line stands in (Power $5M, Group of 5 $0.8M, FCS $0.3M, below $0.05M), and older years scale by `hs_index`. Your AD's raise grows the line, so it buys flights (`tests/recruit_budget.rs`).

### One ledger (GR50)

- `Week.ledger`: each entry's day, what, hours, and whether you chose it. Chosen hours (calls) come out of the week's free hours film uses (`film::time::used` adds them); booked hours (your road day, what you host) don't, but every hour counts for the day's tiredness (`film::time::today`) and booked hours past 12 a week count for the week's wear (`film::time::week`, `BOOKED_FREE`).

### The beat (GR56), the letter, the May list (GR57), the week off (GR58)

- **The beat:** week 1 holds January 1. Each week lists what is on the calendar (the portal, the coaching changes, the services' updates, signing day, junior days, the combine, spring practice, officials for juniors, the spring road's first and last day, the spring game, the draft, term's end, Memorial Day, the May list, your signees, your camps, the transfers, the dead period, NIL payday, the Fourth, media days, the polls, fall camp, high schools, Week 0), read from `YearDates` and `recruiting_period`, and picks the beat and the stake by what is in it (`beat::beat`). The 2026 reading matches `ground/months.md`'s sheet; weeks 3 and 7 (the coordinator at your door, the February meeting) wait for lane G6.
- **The Monday letter:** the beat line first ("Week 16: the road opens Wednesday, the spring game is Saturday."), then what last week bought (new names on the board, levels reached, school visits and recruits seen, your calls), the road days left, the week's flights, the plan, your scenes and your calls, and the stake. The recruiting desk's Monday digest opens with it and carries its subject (`crates/afm-game/src/lane_core/mail.rs`, `people_mail`).
- **The May list:** the day after Memorial Day, each man's rising seniors he saw in person since April 1 (not signed, not committed elsewhere), ranked by your offer, then stars, then how many of your men saw him; your open official spots are the weekends from Friday to the dead period × 10 at FBS (6 FCS, 4 below), less those set. A memo from your coordinator; `may_invite` books the top on each one's first open weekend through the built visit rules (`plan::may_list`, `may_invite`).
- **The week off:** one or two weeks inside June's and July's dead period (`week::take_off`): your eye's wear goes to zero the first day, no film, your calls go to your coordinator and the calls back he takes count as his (`phone::lapse`).

### What it does not do yet

- The scenes are slots with their one-line result: lane G5 plays them. The goals the coordinator ranks against come with lane G6, your men's own ground and areas on the map with lane G10, the AI's weeks with lane G9.
- The 2027 recruiting model (one pool of 300 from January to May) isn't in the game's rule book; `road::pool` is where it goes.
- FCS runs the FBS recruiting calendar (`recruiting_period` has one).

### Where the knobs are

- `week/road.rs`: `ROAD_SHARE` 0.12, `ROAD_WEEKS` 24, `DAY_COST` 180, `MILE_COST` 0.70, `FLIGHT_COST` 700, the stars' `WEIGHT`.
- `week/plan.rs`: `DAYS_PER_MAN` (3, 2, 3), `CALLS` 5, `YOURS` (3, 5), `PER_WEEKEND` (10, 6, 4); the second man past 12 of need.
- `week/phone.rs`: `CALL_HOURS` 0.25, `WAITS` 60 minutes, who picks up (his hour's 50 minutes, a coach's evening 7 to 9:30 PM).
- `week/mod.rs`: `BOOKED_FREE` 12; the hosted hours in `hosted`.

## Related pages

[recruiting.md](recruiting.md), [scouting.md](scouting.md), [people.md](people.md), [new-teams.md](new-teams.md), [staff-and-coaches.md](staff-and-coaches.md).
