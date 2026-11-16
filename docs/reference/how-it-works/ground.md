# Ground: how it works today

Checked against branch commit 28f13b8 on 2026-11-16 (lane G1a, areas and map data), main bf92e7d plus lane G1b on 2026-11-16 (standing and one memory), main d291de54 plus lane G3 on 2026-11-16 (your week and road days), branch lane S2 on 2026-11-16 (the meeting at the map), main fd85a4f9 plus lane G7 on 2026-11-16 (people in full), main plus lane G11a on 2026-11-16 (the high school chain reaction) and main 6aed06fd plus lane G4 on 2026-11-16 (news in order, leads and finds) and main dd7735f4 plus lane S4 on 2026-11-16 (the lead card).

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
- What are the map's pins? Your program's signees from the area, its legends (round 1-3 picks, All-Americans, the Hall) and the six strongest rivals' pipelines into it (`pins`). Your finds are in `Ground.finds` (lane G4).
- Do the areas change the world? No. Nothing in the world reads them yet; every seed plays the same. (The chain reaction, lane G11a, does change it: below.)
- Who is on my ground? At college: high school head coaches and coordinators, junior college coaches and other college staffs' men (world coaches), private trainers and 7v7 directors with their kids (former players), each school's counselor by name, each recruit's family and the prep schools' coaches (`world/ground/people.rs`, `Who`, `Kind`).
- Does a younger brother start from scratch? No. A family is one memory, keyed by the oldest brother the world knows; a cousin's family starts a level lower (`people::family_of`, `cousin_of`; `standing::from_cousin`).
- Does word get around? Yes: what happens with one person reaches his close ties in 2-5 days at half its weight and his area in 5-12 days at a quarter (`world/ground/word.rs`).
- What happens when a coach changes jobs? Your standing goes with him (the memory is keyed by the man), you get a memo, and the new head coach at his old school starts Known if he sat in on your visits or the man before him was Trusted with you (`world/ground/jobs.rs`).
- Does my old Contacts list still count? For people on your ground at a college seat, a contact's warmth, favors and tip record live in the memory; the contact shows a copy refreshed each day (`world/ground/stores.rs`, `sync`).
- What does the person screen read? `ground::card::card` (the `ground_person`, `ground_family` and `ground_school_people` commands): words only, never a hidden number.
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

- Who hears news first (G4)? The people closest to the kid (his coach, his trainer, the camp that timed him) tell programs by standing: Home the day it happens, Trusted a day later, Known on day 3; within a level a busy coach (school prestige 0.5 or more) calls by his nature, a loyal one the staff he's closest to, a self-serving one the biggest program, three calls a day; a small-school coach calls everyone he knows at once, eight a day (`world/ground/news.rs`, `order`).
- When does the public hear? By the news: an offer the same day from 2018, a big game the next day, a college camp's laser time two days on, a move 3 days, a test score 30, a growth spurt 45 (more before the 2010s, 3 more at a small school); track results, all-state and a new coach are the paper, everyone at once (`public_lag`, `What::paper`).
- Does a coach who holds a grudge still call? No: while he holds a big bad moment against you, you hear after the public (`order`, `Why::Stopped`).
- What makes news? Laser fortys at camps and showcases and fully timed 100s from the built times, other programs' dated offers from your level or one below, a kid who moved in, the spring weigh-in (1.5 inches or more in a year, first week of April), the academic line for seniors (October to May), first-team all-state, a new head coach at a school whose old one knew you, and a big game on the engine's box score (`week`, `on_hs_game`, `big_line`).
- Whose news does my seat keep? Kids whose coach is Known or better with your staff, and three-stars or better in the states you recruit (your staff's areas, else your state, your footprint at 0.3+ and 400 miles) (`follows`, `region`).
- Where do leads come from? His coach's list on a visit or call (once a month a school), his coach or trainer calling with news, tips with tape, the camp walk-up, a kid off another kid's tape, and the paper (`world/ground/leads.rs`, `How`).
- What does each level get from his coach's list? Stranger: the names everyone has (three stars or an offer) a day or two later; Known: his real list the same day, with his size, speed and claim, shaded by his agenda and blurred by his eye; Trusted: the truth (his work, his family, games missed hurt) as a call look in your file, and his tape asked for before you ask (`coach_list`).
- What are a lead's blanks? Measured size (your camp, a showcase, his school's spring numbers on a spring visit), timed speed (your camp, a showcase), tape, a live look (practice, a game, your camp), grades and character (a call, the transcript, a home visit): the looks in your file (`Blank::by`).
- How do I fill one? `fill` asks for its look: film within the week, calls within three days, the next evaluation day, or an invitation to your June camp, where he always comes (`fill`; `scouting/dept.rs`, camps).
- What if the coach oversold him? When your camp or a showcase measures him an inch shorter or a tenth slower than he said, his word settles as not held, his record drops ("1 of his 5 kids held up") and your staff writes you (`measured`, `record_words`).
- When is a kid my find? When you offer while the world rates him low: three stars or fewer at FBS (two below), five or fewer other offers. A find you lose stays yours (`world/ground/finds.rs`, `on_offer`).
- What does a find pay? 1.0 for a kid under the star line with no offers, 0.7 at the line, less 0.06 an offer, never under 0.4 (`pay`). A small bloom (a fourth star, or an offer from a level above yours and his offers then), a bloom (a starter in his first two college seasons, or all-conference), a big bloom (drafted, or All-American) pay the teller 2, 4 and 8 standing points × the pay with a moment and a call, your name as a finder 0.5, 1 and 2 × the pay, your man's record, and a big bloom makes news (`reached`, `bloom`).
- What's a miss? Each September 1, up to 12 kids from areas where your staff has people, one or two college seasons in, at your division or higher, who started or made all-conference somewhere else and whom nobody of yours ever looked at, with his coach and why you didn't hear (`misses`).
- Does G4 change the world? No. Every other program's place in the order is read from its ground when the news happens; only your seat keeps news, leads and finds, and your seat's board and file read them.
- Does a high school's prestige move? Yes, each June 15, on its old players in college and the NFL (`world/high_school.rs`, `climb`, called from `world/clock.rs` on `hs_transfer_day`, before the transfers).
- What counts as an old player? Anyone whose last high school it was (`Person::hs`) and who is in college, declared for the draft or a pro now. Players out of football count nothing (`alumni_value`).
- What is each worth? In college 1.0 at a power program, 0.6 elsewhere in FBS, 0.3 in FCS, 0.1 lower, ×1.5 if he started last season; in the NFL 2.0 on a roster or injured reserve, 3.0 with 9 or more starts last season, 0.8 on a practice squad; 0.3 in another league (`alumni_value`).
- How far does it move? Its score is read against what its prestige predicts in its state (a straight-line fit of score on prestige over the state's schools), in standard deviations, at most 2.5 either way (`state_form`). Its form moves 30% of the way to that reading when it rises and 15% when it falls (`FORM_UP`, `FORM_DOWN`), and its prestige is its name plus 0.12 per deviation of form (`CLIMB_SWING`): never more than 0.3 from its name.
- What is its name? `Chain::start`: its prestige when the world began (tradition, money, pull), plus a stadium its town builds. Prestige swings around it, so a name built over decades doesn't vanish with a few thin classes. A state with fewer than 8 schools holds still (`CLIMB_MIN_SCHOOLS`).
- What happens when every school scores alike (nobody playing yet, early in the history)? Nothing moves (`state_form`).
- Does it draw dice? No: it reads the world only, so it never moves the world's stream (`climb`).
- What does prestige change? A recruit's school exposure in his visibility (`world/recruiting.rs`, `recruit_visibility`), who is a powerhouse (prestige 0.75 or more, `HighSchool::is_powerhouse`), what draws standouts' transfers and move-ins (`high_school_transfers`), and everything else that already read a school's prestige (staff pay, crowds, money, schedules).
- Where is its history? `HighSchool::chain.years`, its prestige each June after the step; `chain.college`, `chain.power`, `chain.nfl` count this June's old players and `chain.made` is their score (`Chain`). `prestige_in` reads a past June, `chain_why` says it in words.
- Is it in the news? When a school becomes a powerhouse or slips out, on June 15, a small high school news line with the counts (`climb`).
- A stadium your town paid for (`legacy/hs.rs`) adds 0.03 to its prestige and its name.
- Where is its form? `Chain::form`, in standard deviations.
- An old save gets each school's start from its prestige on load (`chain_after_load`, from `World::after_load` in `world/seat.rs`).

## Lane S2: the meeting at the map (the screen)

Screen: `app/ui/screens/groundweek.js` and `.css` ("The meeting" under Ground in the sidebar, a college seat only; board 2b, GR71). It reads `ground_week` and writes `ground_road_set`, `ground_road_clear`, `ground_hand` and `ground_call` (lane G3). Every size is the build's pick, proposed, not confirmed.

- **What `ground_week` adds for it** (`crates/afm-game/src/lane_groundweek.rs`): `today`; `home` (your state's code, your campus as latitude and longitude, your state's areas); `coordinator` (name and job); `road.closes` (the last day of the window's run); `fronts` (one per area the plan sends a man to, in the plan's order: its number, area, centre and label point, your men there, their days, the coordinator's reason, how many of the area's head coaches you are Known with, the trip, and a kind: "your call" when you set the row, "new door" when you know none of its head coaches, else "the plan"); `one_call` (the first call on your list still to make, with its reason, your level with him, its 15 minutes and how many more wait).
- **The board** draws your state from `art/ground/<ST>.json` with the front areas lifted, a route from the campus to each front, numbered pins and a pill with the men and days. A front past the frame (a flight to another state) runs to the frame's edge and is listed under the map with its trip. Rivals' dotted lines and each front's sign-in sheet against its rival come with the race (lane G9); until then a front shows who you know there.
- **The week by day:** each man on his days (a flight striped), the scenes at their hour (the ones that come to you in gold), the men still at home with the rule that keeps one home. Drag a man's chip to another day to move that day, or a man at home to a day to send him to the area picked beside his name; the × on a chip takes him off that day (his last day clears the road).
- **The letter** shows its first line as the quote and what moved; the plan and the calls it repeats stay under "The whole letter".
- **What it does not do yet:** the scenes are one-line results until lane G5; the coordinator's one call is a call on your list (his goals and the race come with G6 and G9).
- Test: `crates/afm-game/tests/ground_week_screen.rs` (builds a small world).

## Lane S4: the lead card (the screen)

Screen: `app/ui/screens/groundleads.js` and `.css` ("Leads" under Ground in the sidebar, a college seat only; board 4b, GR73, GR79). It reads `ground_leads` and `ground_lead` and writes `ground_fill`, `ground_lead_drop` (lane G4) and `recruit_offer` (the recruiting desk). Every size is the build's pick, proposed, not confirmed.

- **The screen:** a strip of your open leads (newest first, each with its blanks filled out of five; the first with all five filled opens first), and the picked lead's card in three columns. Left: his drawn face and name, his year in school ("rising junior": June and July read a year ahead), the teller's and your man's names, the services (a rating once they've ranked his class, else the next update's date) and his offers; how he's growing (a year ago against now: weight, height, the forty), only once your staff has measured him, else his word and what fills it; the teller's record as a bar of kids who held up and your man's finds. Middle: the ring of five blanks, each filled one with what it found, who filled it and when, the open ones with what fills them and a button (a blank you've asked for says so); who's in his corner (the teller's word with your level with him, and your staff's read of the file). Right: the race in signs on a timeline, your word, and the bloom steps with what each pays.
- **The race** (`race` in `crates/afm-game/src/lane_leads.rs`): up to three programs, those that offered first and then those that heard earliest, each with the days it heard (the order the news went: `leads::news_of`) or offered (`PersonCal.early_offers`; an offer with no date shows as a ring on the line's end), and your own days (when you heard, an early offer of yours, each look that filled a blank). The window is today to the services' next update (`services::next_update`; proposed, not confirmed: the update stands in for "the day the services rate his class", which the world doesn't date); a kid they've ranked has no window.
- **Your word:** offer now (two clicks: the first asks, the second sends `recruit_offer`; it is gone once you've offered), fill the open blank first, let him go (two clicks), each with what it risks. "Wait for his junior tape" is a card with no button: waiting is doing nothing. Once he's your find, the box shows when you offered, what the world thought of him then, each bloom, a flip or a setback.
- **What the card carries beyond G4** (`card` in `lane_leads.rs`): `services` (rated, stars once rated, the next update), `growth.then` (his body a year back on the growth curve that the spring weigh-in's news reads, only once measured) and `growth.height_in`, `today`, `offers_all` (every offer on him, an early one your old staff made included), `race`, ISO dates on the lead, the news, each filled blank (with the look's kind, e.g. "Your camp") and each hearing, `held` and `of` on a person, `word.offered`, and `ready` (all five filled) on each row of `ground_leads`.
- **What it does not show:** the teller's record year by year (the store keeps held and total, not the year), hours spent on the lead (not kept), each blank's finding in words beyond size, speed and the read (the files keep only who looked and when), and rivals' own colors (the world's programs carry no card colors here).
- Test: the lead test in `lane_leads.rs` (the card's new keys, and the offer through the desk's command: once it's out the word drops the option).

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
- Your finds are in `Ground.finds` (lane G4) for the map's pins.

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

## Lane S1: the map screen and the school page

Screens in `app/ui/screens/ground.js` and `ground.css` (Press Box for the map, the glass look with the school's own colors for the school page: GR79); the data in `crates/afm-game/src/lane_ground.rs`, commands `ground_map` and `ground_school`; test `crates/afm-game/tests/ground_map.rs`. The game's lane table in `lib.rs` is now a slice, so a lane adds one line and no count. A new **Ground** section in the sidebar for a college seat; it holds "The map" now, and each later screen adds its own entry.

- **What opens.** The state your campus is in; a picker lists every state. The area you hold best is picked first; click an area, its label or its row in the list to pick another.
- **The fill.** Each area is shaded in the color of the program that leads it: the program whose head coaches, weighted by each school's talent, stand highest with it (`ground_map`, `top_two`, built on `standing::area_points`; the same weighting as GR27). Yours shows as yours. Contested means the second program is within 5 points of the leader, a fifth of a level (`CONTESTED`, the build's pick, proposed, not confirmed). Every program counts, so a state is one pass over its schools and the world's programs (about a second for Texas on the shipped world; not measured on a full world).
- **Fog (GR2).** An area is fogged until you are Known (25 points) with at least one head coach there. A fogged area is hatched, shows only its public line, the biggest schools by talent and who leads it; its words and its people stay hidden. A school visit opens it (`known` in `ground_map`).
- **Your standing beside each area.** A word and three ticks (Stranger none, Known one, Trusted two, Home three) from `standing::area_points`; "With 4 of its 13 head coaches, since April 14" counts head coaches at Known or better and the earliest day you met one of them.
- **Pins (GR6).** Your signees of this class and the three before it (blue; `SIGNED_BACK`) and the area's legends (white ring) at their schools' towns, from `areas::pins`. Your finds come with lane G4.
- **Pipelines (GR6).** Each area's strongest rival pipeline as a dotted line in that rival's color to its campus, cut at the frame with the program's name where the campus is off the map; the picked area shows up to four. Lines weaker than 0.12 are left out.
- **Lately.** What people kept of you in the last seven days, in their own words, newest first, from the moments on your program's and your own memories (`Memory::moments`). Only moments on your own memories show; the news order comes with lane G4.
- **The area picked.** Who leads it and how many of its head coaches stand at Home, Trusted and Known with them and their head coach's years there; where you stand; the area's words once you're Known; its biggest schools' head coaches by name with your level with each (the biggest first); the drive or flight from your campus, schools a day, your signees and its schools.
- **A school (GR11).** Its colors (`colors::high_school`), town, enrollment, public or private, whether it plays in the state's biggest class, titles, the drive or flight; its people (head coach, coordinators, counselor) with your level with each and, once Known, the why in words and the newest moment he keeps; where its kids went (first college of every alum and every kid past high school, with how many were drafted); its last six seasons; your signees and its legends from there (the area's top 12 legends only, `areas::LEGENDS`). Its season records go back as far as the school's record does. Online film (Hudl) from 2008, as the film rules read (`scouting/dept.rs`, `hudl`); the spec says 2006, so one of them is wrong (open question).
- **Not built here.** Your men's routes and faces, road days and the week's hours (lane G3), the overnight news in order of standing (lane G4), Friday nights on the map (GR5, needs G3's calendar), a stadium (the world has none), the person screen for a coach (lane S5; his row opens his school).

## What it does not do yet

- The AI working its ground: lane G9. Favors: lane G11b.
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
- `world/high_school.rs`: `CLIMB_SWING` (0.12), `CLIMB_CAP` (2.5), `FORM_UP` (0.3), `FORM_DOWN` (0.15), `CLIMB_MIN_SCHOOLS` (8), `alumni_value` (the old players' worth; all proposed, not confirmed).
- `world/recruiting.rs`: `SIT_WEIGHT` (0.6), `move_in_chance` (0.1 + 0.5 × money, at most 0.6; proposed, not confirmed).

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

## Lane G4: news in order, leads and finds

Every size below is the build's pick (proposed, not confirmed, tuned in GR61's trial worlds and in play) unless the spec's "Starting sizes" names it.

### News in order (rule 2, GR16, GR19, GR20, GR64)

- **What starts it** (`world/ground/news.rs`, `week`, Mondays, the week before): a laser forty at a college's camp or a showcase at 4.45 or better for backs, receivers and DBs, 4.65 for backers, ends, tight ends and quarterbacks, 5.00 for the big men (`fast`); a fully timed 100 meters at 10.75 or better at a meet (`FAST_100`); another program's dated offer from your division's rank or one below (`PersonCal.early_offers`, `folk::tips::rank`; an FBS seat hears FBS and FCS offers, not Division II's); a kid whose last season was at another school (moved in); the spring weigh-in, the first Monday from April 1 to 7, when he grew 1.5 inches or more in the year (his height on his growth curve a year apart, `body::height_share`; `GREW`); a senior crossing the academic line, October to May (`recruiting::ncaa_tier` on his eligibility GPA, `Ground.notq`); first-team all-state from December; a new head coach at a school whose old one was Known or better with your staff, for kids you have leads on (`Ground.heads`). A big game is read off each high school game on the engine: 300 passing yards or 4 touchdown passes, 200 rushing or 3 rushing touchdowns, 150 receiving or 3 receiving touchdowns, 3 sacks, 4 tackles for loss, 2 interceptions (`on_hs_game`, `big_line`, hooked in `world/hs_year.rs` beside the prospects' tally).
- **Whose** (`follows`): sophomores to seniors in high school or prep school, unsigned (`recruit`), whose coach is Known or better with your staff, or three-stars or better in your states (`region`: your staff's areas, else your state, footprint 0.3+ and 400 miles).
- **Who knows first:** his head coach; for a laser time his trainer if he has one (lane G7's `people::trainer_of`: two stars or more, one in his state, the same one every time), and the program whose camp timed him first of all (`Why::There`). The paper (track, all-state, a new coach, a showcase's times) has no teller.
- **The order** (`order`): every program at Known or better with the teller (its program's or best current man's standing, `standing::staff`), and yours. By level first; within a level a busy coach (his school's prestige 0.5+ or national, `BUSY`) orders a loyal man's calls by standing points (loyalty 0.5+ on his makeup: `Why::Closest`) and a self-serving man's by the program's prestige (`Why::Biggest`); Home hears on day 0, Trusted day 1, Known day 3 (`LEVEL_DAYS`), three calls a day within a level (`CALLS_A_DAY`). A small-school coach calls everyone he knows from day 0, eight a day, whatever the level (`SMALL_CALLS`, `Why::Pushes`). Each day may slip one (a 30% chance from the world's seed and the news), never ahead of the call before it. Anyone whose day reaches the public's hears with it. A staff he holds a big bad moment against isn't called (`Why::Stopped`). A Stranger hears a day or two after the public (`Why::Public`); the paper reaches everyone on its day (`Why::Paper`).
- **The public's day** (`public_lag`): an offer 0 days from 2018 (1 from 2010, 7 before); a big game 1; a college camp's laser time 2 from 2010 (14 before); a move 3 from 2018 (7 before); a test score 30; a growth spurt 45 from 2010 (60 before); the paper 0. A small school (prestige under 0.35, not national) adds 3, except for the paper.
- **What your seat keeps** (`News`, `Ground.news`, up to 1,500, the oldest delivered first out): the line in words, the day, the teller, the first eight programs in the order and yours wherever it falls, how many heard before the public, the public's day, your day, place and why. Each is kept once by its key (the kid, what, and the year, the offer's program, the school he left, the day). Nothing in the world reads it.
- **In words:** "Auburn: Monday. You: Tuesday." (`order_words`, the first three and you; weekdays within the week, dates beyond) and why you were where you were ("You're Trusted with Coach Smith, and he's closer to Georgia State: he called them first.", `why_words`).
- **Reaching you** (`deliver`, each day): on your day the news joins his lead, or makes one (`leads::from_news`).

### Leads (GR19, GR67)

- **A lead** (`world/ground/leads.rs`, `Lead`, `Ground.leads`, sorted by kid, up to 3,000, the oldest dropped or settled out first): the kid, the day you heard, how (`How`: his coach's list, a call with news, told when you called, a tip, the camp walk-up, off another kid's tape, the paper), who told you (a ground person, or a tipper's name), your man who heard (your area man for his state, else his position coach), your level with the teller then, what he said in his own voice, the height and forty he gave, his claim (worth a look, can play for you, a sure thing), the news that reached you, whether his word held, and dropped. A kid's first lead stays: later news joins it.
- **His coach's list** (`coach_list`, when your staff's practice, game or call look is at his school, once every 30 days a coach, `LIST_EVERY`, at most 6 names, `LIST_MAX`): a Stranger gives the names everyone has (three stars or an offer) one or two days later; Known gives his real list the same day: each kid his read places at or above your level's floor (the tips' bar and floor, `folk/tips.rs`; his read is the kid's film plus noise of 1.5 + 0.45 × (20 − his eye), placed in the kid's class), with his claim (a level up when he leans) and the kid's size and speed as he gives them: height + 0.6 × lean + up to half an inch of blur, forty − 0.08 × lean + up to 0.05 of blur (`numbers`; lean is his agenda × (1 − straightness/20) × (1.2 − your standing/100): his eye and straightness are his makeup's (lane G7, `people::makeup`), his agenda's push a family's 0.9, a trainer's 0.8, a high school, junior college or prep coach's 0.6, a 7v7 man's 0.5, another college staff's 0.4, a counselor's 0.2). Trusted adds the truth (his work ethic, his family, games missed hurt last fall) to his words, a call look by your man in your file (the grades and character blank) and his tape asked for (`truth`).
- **From tips** (`from_tip`, hooked in `folk/tips.rs`, `send`): a high school coach, a trainer or a 7v7 man becomes the lead's teller; an alum or a youth coach is a tipper by name. A camp man's laser time and height are the lead's size and speed.
- **From your looks** (`on_look`, hooked in `scouting/dept.rs`, `add`): a kid at your camp with two stars or fewer, no lead and no offer from you walked up; a kid off another kid's tape (`Kind::SideFilm`) with no lead is one.
- **From news** (`from_news`): a call (Trusted or Home with the teller, before the public) or told (Known); otherwise the paper. A new lead only from news that names a prospect (not a new coach or a test score) and only while the public doesn't rate him: two stars or fewer from the paper, three from a person; an offer makes one only from your division's rank or above. Only a Home coach with a kid you didn't have writes you at his hour, with the order; a Trusted one's call lands on the board without a mail (build's pick, proposed, not confirmed: a spring with every Trusted call as mail sent 136 in five months).
- **The five blanks** (`blanks`, `Blank::by`): the earliest look in your file that fills each, and whether you asked. Measured size: your camp, a showcase, a spring (April-May) practice look; timed speed: your camp, a showcase; tape: high school, college, office or side film; a live look: practice, a game, your camp, a showcase; grades and character: a call, the transcript, a home visit.
- **Filling one** (`fill`): asks for its look in your file's asks: his film within the week, the calls within three days, the next evaluation day, or an invitation to your June camp (2nd and 3rd Saturdays), where an invited kid always comes (`scouting/dept.rs`, camps). A filled blank or one already asked can't be asked again.
- **His word** (`measured`, `settle`, `Ground.records`): when your camp or a showcase measures him more than an inch shorter or (not a kicker) a tenth slower than he said, the lead's word settles as not held, the teller's record is 1 kid more and none held, and your staff writes "was oversold". A find's first bloom settles it as held, a bust as not. The record in words: "4 of his 6 kids held up" (`record_words`).
- **Who else knows** (`who_else`): the programs that heard his news before or with you, and the programs that offered him.
- **Your board** (`scouting/dept.rs`, `watch`; `Board`): for your college seat, your offers, visits and commits (100), your leads (4 + 2 × claim + 1 a filled blank, so a lead you heard about sits with the two-stars and a filled, claimed one with the four-stars) and the public's three-stars or better, each + 2 × stars + 3 × your staff's standing with his coach (Stranger 0, Known 0.4, Trusted 0.7, Home 1.0) in place of footprint and pipelines; portal players + 4 as before. Without a college seat, or with the test switch off, the built board.

### Finds, blooms and setbacks (GR66, GR67)

- **A find** (`world/ground/finds.rs`, `on_offer`, hooked in `front_office/college.rs`, `offer_at`, for your seat): your offer while he has three stars or fewer at FBS (two below, `low`) and five or fewer other offers (`LOW_OFFERS`). Kept with his stars, other offers and the highest level among them then, the teller and your man from his lead (else your area man), you, and what it pays (`pay`: 1.0 under the star line, 0.7 at it, less 0.06 an offer, 0.4 to 1.0; a two-star with no offers 1.0, a three-star with five 0.4).
- **Blooms** (`day`, `reached`, each day): a small bloom, a fourth star above his stars then, or an offer from a program a level above yours and above his offers then (`tier`: division, then a power conference above the rest of FBS); a bloom, a starter in his first two college seasons, or all-conference; a big bloom, drafted, or a first-team All-American. Each new step: your man's record (blooms, big blooms; `Man`, `Ground.men`), your name as a finder + 0.5, 1 or 2 × the pay (`FINDER`, `Ground.finder`, in words by `finder_words`), the teller + 2, 4 or 8 points × the pay inside the gates with your program and your head coach, the moment in his words and his call at his hour (`TELLER`, `standing::bloom`), his kid held up on his record; a big bloom posts news naming the program that believed first.
- **Setbacks:** the flip (he signs elsewhere: still your find, `lost`), the grades (a prep school or a junior college after his class signed), the bust (September 1, four years past his class with no bloom: the teller's word didn't hold).
- **Misses** (`misses`, September 1): kids one or two college seasons in, at your division or higher, from the areas where your staff has people, whom nobody of yours ever looked at or had as a lead, who made all-conference or started there; the 12 strongest (`MISSES`, all-conference first, the lower his stars the higher), each with his school's coach, your level with him and why you didn't hear ("His coach at Monroe is a Stranger to you: nobody on your staff has been to his school since 2025."). Up to 60 kept (`MISSES_KEPT`).
- **Not yet:** your man's mood (no staff mood store yet: lane G10), your name as a finder read by athletic directors and coaches (lanes G6 and G7), pipelines that grow from blooms (the built pipelines grow from signees), the race's signs (lane G9), the ground report (lanes G6 and S8, which read `Ground.finds`, `misses` and `men`).

### The commands (`crates/afm-game/src/lane_leads.rs`)

- `ground_leads`: your open leads, newest first: the kid (your staff's read, potential in words), how, who told you, his claim, blanks filled, whether he's a find, his word, how much news.
- `ground_lead` (`id`): the lead card: the kid, his offers and stars, who told you and what he said, the teller's standing, why and record, your man's finder record, the measured size once measured and his recorded times, the five blanks (who filled each and when, asked, what fills it and its time), each piece of news with its order and why, who else knows, his find, and your word (offer now, wait for his junior tape, fill the next blank first, each with what it risks; the bloom steps with what each pays, in words).
- `ground_fill` (`id`, `blank`: `size`, `speed`, `tape`, `live`, `character`), `ground_lead_drop` (`id`), `ground_finds` (finds with blooms and setbacks, misses, your men, your name as a finder).

### Where the knobs are (G4)

- `world/ground/news.rs`: `ORDER_KEPT` 8, `KEPT` 1500, `CALLS_A_DAY` 3, `SMALL_CALLS` 8, `BUSY` 0.5, `LEVEL_DAYS` [0, 1, 3], `public_lag`, `fast`, `FAST_100` 10.75, `GREW` 1.5, `big_line`, `region`.
- `world/ground/leads.rs`: `KEPT` 3,000, `LIST_EVERY` 30, `LIST_MAX` 6, `teller` (agendas), `numbers` (the shading), `truth`, the board's weights in `Board::of` and `Board::near`, the oversold margins in `measured` (1 inch, 0.10 s).
- `world/ground/finds.rs`: `TELLER` [2, 4, 8], `FINDER` [0.5, 1, 2], `low`, `LOW_OFFERS` 5, `pay`, `MISSES` 12, `MISSES_KEPT` 60.
## The chain reaction (lane G11a, GR21)

Every June 15, before the transfers, `climb` counts each school's old players on all cores (`alumni_value`). In each state with 8 or more schools, `state_form` fits score on prestige over the state's schools and reads each school's gap to the line in standard deviations (at most 2.5). Its form (`Chain::form`) moves 30% of the way to the reading when the reading is higher and 15% when lower, and its prestige becomes its name (`Chain::start`, set at the build in `generate_state_schools` and for a new school in `new_teams/school.rs`) plus 0.12 × form, within 0.02-1.0.

What it reads: where old players are now (stage, college division and power conference that season, last season's role, NFL status and starts). What it does not read: wins or titles, the current roster, star ratings, or anyone's hidden potential.

The loop closes through the kids: prestige is a recruit's school exposure (0.3 of his visibility, `recruit_visibility`), so a climbing school's kids are seen more, sign higher and keep it climbing; a school at 0.75 or more is a powerhouse and draws the state's standouts (`high_school_transfers`).

## Move-ins and transfers by the state's rule (lane G11a)

On June 15 the standouts (the top 3% of their state's age 15-17) weigh a move to a powerhouse as before (`world/recruiting.rs`, `high_school_transfers`; the pulls are in `recruiting.md`, "High schools and being seen"). The state's rule (`world/high_school.rs`, `transfer_rule`) now decides how many go:
- What he would sit: the rule's sit-out (a full year 1.0, half a season 0.5), or nothing on a state's free first transfer (`TransferRule::free_once`).
- A move-in: where a bona fide family move makes him eligible at once (`TransferRule::move_exempt`, every state but Arizona and Tennessee, whose rules sit transfers even with a residence change), a family moves into the new school's district with a chance of 0.1 + 0.5 × family money, at most 0.6 (`move_in_chance`), drawn on his own key.
- The pull is cut by 60% of what he would still sit: × (1 − 0.6 × sit-out × (1 − move-in chance)) (`SIT_WEIGHT`). A move out of state already moves the family; it sits only where the rule doesn't free a move.
- A family that moved in-state frees him (where the rule allows) and his note says so: "his family moving into its district so he could play at once".

## The world check (seed 161, 2026-11-16)

`chain_check <world file>` (crates/afm-tools) prints the chain's history, moves by class and where blue chips played. Seed 161 at a tenth of the size, history on the stand-in (1984-2025), against main the same way:
- Powerhouses: 246 at the start, 234-247 every five years, 240 at the end; 3-7 schools that didn't start as one hold the status at a time.
- The states' real top-five schools: 234 of 246 still powerhouses, mean prestige 0.923 at the start and 0.888 at the end.
- The biggest climbers gained 0.24-0.29 (Louisville High, KY, 0.50 to 0.75); the biggest fallers lost 0.15-0.28 (Mater Dei 0.95 to 0.67 with 29 old players in college, 5 at power programs, and 1 in the NFL).
- June moves, classes 2021-2026: 26-36 per 1,000 against main's 32-36; transfers to a powerhouse 105-149 a class against main's 123-189, a fifth to a quarter of them move-ins.
- Blue chips, classes 2020-2026, at today's powerhouses: 36.2% against main's 39.7%. FBS signees by home state match main to 0.3 points.
- A world's talent per school is flat (the top tenth of schools hold 20% of the old players' score), so a free walk on production alone took the real powerhouses down (two earlier rules: real top-five schools to 0.79 and 0.75, powerhouses to 219 and 177). Prestige swings around the name for that reason.

## Related pages

[recruiting.md](recruiting.md), [scouting.md](scouting.md), [people.md](people.md), [new-teams.md](new-teams.md), [staff-and-coaches.md](staff-and-coaches.md).
