# Ground: how it works today

Checked against branch commit 28f13b8 on 2026-11-16 (lane G1a, areas and map data), main bf92e7d plus lane G1b on 2026-11-16 (standing and one memory), main d291de54 plus lane G3 on 2026-11-16 (your week and road days), branch lane S2 on 2026-11-16 (the meeting at the map), main fd85a4f9 plus lane G7 on 2026-11-16 (people in full), main plus lane G11a on 2026-11-16 (the high school chain reaction), main 6aed06fd plus lane G4 on 2026-11-16 (news in order, leads and finds), branch lane S5 on 2026-11-16 (the person screen) and main 256934ee plus lane G6 on 2026-11-16 (the campaign), main plus lane G8 on 2026-11-16 (a following and famous names), main 80ce85c plus lane G11b on 2026-11-16 (favors, switched off), branch lane S8 on 2026-11-16 (the ground report screen), main d64b44f plus the college followings retune on 2026-11-16 (storms reach college), and main plus lane G5 on 2026-11-16 (the scenes), and main plus lane S4 on 2026-11-17 (the lead card), and main plus lane G10 on 2026-11-16 (your staff's ground and the assistant seat), and main plus lane S7 on 2026-11-17 (your men and your desk, the screens), and main a5d9e29 plus Capital lane C4 on 2026-11-17 (donor slots in the week), and branch lane S3 on 2026-11-17 (the school visit screen), and main bfbec55b plus lane G9 on 2026-11-17 (the race and the AI on Ground). Your word's quiet promises, the program a promise was made at, your own word and his family's, the day a word is dated, a family's trust and a cousin's word (code oddity 29) checked over main 349eefb on 2026-11-19, before it merged.

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
- How do the week's scenes play? As folk scenes of their own kind at their hour, your eye rolled the way a film catch is, chips or live talks v3 with a key, one line each, your coordinator's when you hand one on (lane G5, below; `world/ground/scenes/`).
- Do the areas change the world? Yes, since lane G9: every other staff's men work them, and its board reads that work ("Lane G9" below). The chain reaction (lane G11a) changes it too.
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
- Does the AI have ground? Yes. Since lane G9 every other staff works it by your rules (areas, road days, five calls a week, fade, his kids), on top of day one from the world's 30 years (`derived`), and its board reads it. See "Lane G9" below.
- Do my roots count? Your own high school's coach starts Trusted, your hometown's coaches and the high schools you coached at Known, and a kid from your hometown knows your name (`seed`, `roots`, `knows_your_name`).
- What does an area's level read? Each school's head coach's standing, weighted by the talent the school sent out over the last four classes (`area_points`, `talents`).
- Are favors on? No. The favor economy (lane G11b) ships switched off in every world (`WorldOptions.favors`, false); with it off nothing in `world/ground/favors.rs` runs and placing a kid works as before (`folk/backchannel.rs`, `place_player`). Lane G12's trial turns it on. See "Lane G11b" below.

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
- Can a recruit have a famous father? Yes, from lane G8: a son or nephew of a world player with a Pro Bowl or more, or a Heisman, carries his name (`world/following.rs`, `names`, `man_fame`).
- How often? Per famous man, 45% have one son who plays and 13% two, each 14 when the man is 36 to 50; 30% have a nephew, 14 when the man is 30 to 55 (`kin`).
- Does it change how many boys have an NFL father? No. A son is one of the class's boys whose father already played in the NFL (1 in 200); now the father has a name (`names`; `world/background.rs`, `generate_family`).
- How much does the name add? To his fame, 0.35 × the man's fame for a son, 0.18 × for a nephew, in high school and college, half of it as a pro (`HYPE_SON`, `HYPE_NEPHEW`; `world/fame.rs`, `fame`).
- When is a commitment big news? When his following is one of his class's 10 biggest (and at least 10,000), huge for the class's biggest; otherwise his stars decide as before (`commit_news`; `world/recruiting.rs`, `commit_now`).
- What does the hype kid's commitment do for the program? When his following is one of his class's 10 biggest, it lifts the program's brand exposure grade for a year by 3 × his share of the class's biggest following; a program holds its biggest lift, not the sum (`committed`, `day`, `LIFT_TOP`).
- Does it stay if he flips or decommits? No. The lift is his while he is committed to, signed with or playing for that program; a flip moves it (`holds`, `committed_in`).
- Does a recruit's following change his own pick? Yes: brand exposure counts 1 + his share of the class's biggest following in the prestige part of a school's appeal, so up to twice (`world/recruiting.rs`, `stage_weight`, `utility_fit`, `six_parts`).
- Can a college player have a social media storm? Yes, a college player or recruit with 500,000 followers, the same weekly chance as a pro (`world/storylines/detect/weekly.rs`, `chances`; `STORM_FOLLOWERS`).

## Lane S2: the meeting at the map (the screen)

Screen: `app/ui/screens/groundweek.js` and `.css` ("The meeting" under Ground in the sidebar, a college seat only; board 2b, GR71). It reads `ground_week` and writes `ground_road_set`, `ground_road_clear`, `ground_hand` and `ground_call` (lane G3). Every size is the build's pick, proposed, not confirmed.

- **What `ground_week` adds for it** (`crates/afm-game/src/lane_groundweek.rs`): `today`; `home` (your state's code, your campus as latitude and longitude, your state's areas); `coordinator` (name and job); `road.closes` (the last day of the window's run); `fronts` (one per area the plan sends a man to, in the plan's order: its number, area, centre and label point, your men there, their days, the coordinator's reason, how many of the area's head coaches you are Known with, the trip, and a kind: "your call" when you set the row, "new door" when you know none of its head coaches, else "the plan"); `one_call` (the first call on your list still to make, with its reason, your level with him, its 15 minutes and how many more wait).
- **The board** draws your state from `art/ground/<ST>.json` with the front areas lifted, a route from the campus to each front, numbered pins and a pill with the men and days. A front past the frame (a flight to another state) runs to the frame's edge and is listed under the map with its trip. Rivals' dotted lines and each front's sign-in sheet against its rival come with the race (lane G9); until then a front shows who you know there.
- **The week by day:** each man on his days (a flight striped), the scenes at their hour (the ones that come to you in gold), the men still at home with the rule that keeps one home. Drag a man's chip to another day to move that day, or a man at home to a day to send him to the area picked beside his name; the × on a chip takes him off that day (his last day clears the road).
- **The letter** shows its first line as the quote and what moved; the plan and the calls it repeats stay under "The whole letter".
- **What it does not do yet:** the screen shows each scene's line; lane G5 plays them (each slot names its scene, and a scene of yours opens from its slot; the school visit has its own page, lane S3 below); the coordinator's one call is a call on your list (his goals are lane G6's, the race lane G9's).
- Test: `crates/afm-game/tests/ground_week_screen.rs` (builds a small world).

## Lane S4: the lead card (the screen)

Screen: `app/ui/screens/groundleads.js` and `.css` ("Leads" under Ground in the sidebar, a college seat only; board 4b, GR73, GR79). It reads `ground_leads` and `ground_lead` and writes `ground_fill`, `ground_lead_drop` (lane G4) and `recruit_offer` (the recruiting desk). Every size is the build's pick, proposed, not confirmed.

- **The screen:** a strip of your open leads (newest first, each with its blanks filled out of five; the first with all five filled opens first), and the picked lead's card in three columns. Left: his drawn face and name, his year in school ("rising junior": June and July read a year ahead), the teller's and your man's names, the services (a rating once they've ranked his class, else the next update's date) and his offers; how he's growing (a year ago against now: weight, height, the forty), only once your staff has measured him, else his word and what fills it; the teller's record as a bar of kids who held up and your man's finds. Middle: the ring of five blanks, each filled one with what it found, who filled it and when, the open ones with what fills them and a button (a blank you've asked for says so); who's in his corner (the teller's word with your level with him, and your staff's read of the file). Right: the race in signs on a timeline, your word, and the bloom steps with what each pays.
- **The race** (`race` in `crates/afm-game/src/lane_leads.rs`): up to three programs, those that offered first and then those that heard earliest, each with the days it heard (the order the news went: `leads::news_of`) or offered (`PersonCal.early_offers`; an offer with no date shows as a ring on the line's end), and your own days (when you heard, an early offer of yours, each look that filled a blank). The window is today to the services' next update (`services::next_update`; proposed, not confirmed: the update stands in for "the day the services rate his class", which the world doesn't date); a kid they've ranked has no window.
- **Your word:** offer now (two clicks: the first asks, the second sends `recruit_offer`; it is gone once you've offered), fill the open blank first, let him go (two clicks), each with what it risks. "Wait for his junior tape" is a card with no button: waiting is doing nothing. Once he's your find, the box shows when you offered, what the world thought of him then, each bloom, a flip or a setback.
- **What the card carries beyond G4** (`card` in `lane_leads.rs`): `services` (rated, stars once rated, the next update), `growth.then` (his body a year back on the growth curve that the spring weigh-in's news reads, only once measured) and `growth.height_in`, `today`, `offers_all` (every offer on him, an early one your old staff made included), `race`, ISO dates on the lead, the news, each filled blank (with the look's kind, e.g. "Your camp") and each hearing, `held` and `of` on a person, `word.offered`, and `ready` (all five filled) on each row of `ground_leads`.
- **What it does not show:** the teller's record year by year (the store keeps held and total, not the year), hours spent on the lead (not kept), each blank's finding in words beyond size, speed and the read (the files keep only who looked and when), and rivals' own colors (the world's programs carry no card colors here).
- Test: the lead test in `lane_leads.rs` (the card's new keys, and the offer through the desk's command: once it's out the word drops the option).

## Lane S5: the person (the screen)

Screen: `app/ui/screens/groundperson.js` and `.css` ("People" under Ground in the sidebar, a college seat only; board 5, GR74, GR79). It reads `ground_person` and `ground_people` and writes `ground_call` (lane G3) and `face_drawn_store`. Every size and every word below is the build's pick, proposed, not confirmed.

- **Where it opens from:** a coach's row on the map's "Who matters here" and on the school page, and any tie on a person page, open the person (`data-go="groundperson"` with `kind` and `id`). The school is one click from his role line. "People" in the sidebar lists everyone your staff has dealt with, the last in touch first (`ground_people`, at most 60, with the total).
- **What `ground_person` adds to the card** (`crates/afm-game/src/lane_ground_people.rs`): `age` (a coach's, a former player's from his birth date, a prep coach's 48; none for a counselor or a family), `school`, `lv` (his level as ticks), `staff_read`, `moments` (dated, five at most, newest first: `new` for the newest if it is under a week old, `fade` for a small one over a year old, a note for a big one: "He keeps it at full weight for years" or "He holds it against you until 2029"), `next` and `gets` (see below), `staffs`, `talked`, `last`, `met`, `call` (the key `ground_call` takes), `today` and `face`.
- **Your staff's read** (`staff_read`): words only. His eye ("Good. 4 of his 6 kids held up.") and how straight he is read his record with you once three of his tips have settled, else "Too early to say" at Known or better and nothing at Stranger. His nature ("Loyal. Holds a debt a long time, a grudge longer.") shows from Known: loyal at 0.65 or more, self-serving at 0.35 or less; a long grudge at 4.5 years or more, a short one at 3.5 or less.
- **What a level gets you** (`gets`): for his kind and each level, the words in the build's own wording from people.md's "What he can tell you". `next` says when the next level can come from the gates (GR28): Known after two contacts; Trusted not before a year after Known with no broken word in it; Home not before three years (a kid who did well) or five since you met.
- **His standing with every staff** (`staffs`): the programs that are Known or better with him by day one's read (`standing::points`), the five best, then yours last with the date you first dealt with him. A note says "took N of his kids" (his school's kids whose first college it is) or "his old school" (a trainer's or 7v7 man's college). Not shown for a counselor or a family.
- **What you've talked through** (`talked`): the number of live talks with him and the last one's day (`Folk.live.persona.talks`); for a family also the groups of points made to the kid (`Folk.talked`). A coach's talks are not recorded by topic, so no topic chips show for him.
- **His head:** drawn from his recipe by the page, the way the faces are (`face`: `spec`, `colors`, `drawn_on`, the saved picture). A coach is `World::coach_face`; a trainer, a 7v7 director and a prep coach are `World::adult_face` (the coach's own method, from his looks number, heritage and age: a former player's age from his birth date, a prep coach's 48). The drawn head has the place's colors on the jersey (his school's or program's). A counselor and a family keep their initials on the place's colors (the heads are men's). The picture is kept in the save's faces folder under an id in a range of its own (2,147,483,648 and up, by kind and id), so it never takes a player's head.
- **No hidden number:** the page carries the card's words and the words above; the test checks the read is words.
- **Test:** `crates/afm-game/tests/ground_person_screen.rs` (builds a small world).

## Lane S8: the ground report (the screen)

Screen: `app/ui/screens/groundreport.js` and `.css` ("Ground report" under Ground in the sidebar, after People, a college seat only; board 8b, GR77, GR79). It reads `ground_report` (lane G6) and writes nothing; heads come from `face` and are kept with `face_drawn_store`. Every size and word below is the build's pick, proposed, not confirmed.

- **When there is one:** the report is written at the verdict (the Monday before your first game). Before it the page says when it comes and what it will read, with links to the meeting and the map. Every earlier spring stays (the last 6, newest first) and a Spring picker shows when there are two or more.
- **The top:** the date, your goals as a count ("3 met, 1 mostly"), your finds ("2, and 1 blooming"), your misses (0 or 1), and a button to the verdict memo in your inbox.
- **The hero:** the title with its last word "spring" outlined ("Your first spring"; a later spring adds "at Georgia State" under it), one line of what the spring came to, and one tile per goal (`goals`): the slot (Your needs, Your AD's, Your goal, The hunt, a bold goal marked), the verdict (Done reads "Met" with a check; Mostly and Partly a part-filled amber ring; Missed an amber cross; a dropped goal reads "Dropped"), the goal's words and why from the world.
- **Finds** (`finds`): his head (drawn from his recipe, the photo you set, else his initials), his state (signed, committed, lost to a rival, still open), "BLOOMING" or "BLOOMED" and "A NAME" by `steps` (1, 2, 3) with three bars and their words (World catches up, He plays, A name), his stars then and now and the latest bloom's words. Under them "Who found them": each of your men with finds, blooms and big blooms (`men`).
- **Your ground, February to August** (`areas`): one row per area read at the February meeting, sorted by how far it moved. The line is your standing 0 to 100 with the four levels' bands (Stranger, Known, Trusted, Home at 25, 50 and 75): an open dot for February, a filled dot for August, a bar between, amber where a level was lost (`lost`). The headline area (`headline`, the one that moved most past 0.5 points) is called out and its row lit gold. `feb_pts` and `pts` are new on `AreaLine` (`#[serde(default)]`); a report saved before them reads the level's floor.
- **Who outworked whom** (`outworked`): up to 8 areas, your staff's dated contacts against the busiest other staff's, as a split bar and a sentence, in signs ("your staff was in 4 times; Auburn's name was on 9 of the sheets"), never their numbers or boards. "Close" when within one, the rival's name in amber when he is ahead by two or more, "You" when you are.
- **The miss, a setback, the words:** `miss` and `setback` are split at their first colon into a headline and the rest (`miss_parts`, `setback_parts`); the miss's fix is a link to the map. `ad_parts` and `paper_parts` split your AD's line and the paper's into who and the quoted line.
- **Not yet:** the setback's trainer-against-camp scale (the data is one sentence), faces for the rival's recruiters (the race is lane G9) and the people counts under an area ("Known with 6 of 13"): the chart reads standing points.
- **Test:** `crates/afm-game/src/lane_campaign.rs` (`a_first_spring_runs_from_the_meeting_to_the_report` checks every field the screen draws; `the_reports_words_split_into_who_and_what`).

## Lane S7: your men and your desk (the screens)

Screens: `app/ui/screens/groundstaff.js` ("Your men" under Ground, the head coach only; board 7b, GR76) and `grounddesk.js` ("Your desk" near the top of the sidebar, the college assistant only; board 9, GR78), with `groundstaff.css` (classes `gs-` and `gk-`). They read lane G10's commands (`crates/afm-game/src/lane_groundstaff.rs`) and add no rules. Every size and word below is the build's pick, proposed, not confirmed.

- **Your men, the warning** (`ground_carousel`): when another school's search has reached one of your men and he is waiting on your counter, the page opens on him: his drawn face, "<name> may go", who wants him as what and why, a track of the offer, your counter's last day and the early signing day, what pulls him each way in words on a beam with his mood word, and your read of his nature. His office: his line in the hallway and three counters (a raise, the assistant head coach title, the next coordinator job), each with how far it reaches in words and four pips (a long shot to likely to keep him), what it costs you, and a button that is off once you have countered or when it can't apply ("he's your assistant head coach already"). "Let him go now" sends him at once. No counter by the last day and the search hires him. Beside it: who takes his area (up to five candidates: three on ground, the best teacher, one the rules bar, each with his ground level and signs, never a rating), the days to the early signing day, what leaves with him by area (his area's level today against if he goes) and the commits who would waver, each with "Call him" (`ground_waver_call`, a one-line result). His answer to a counter comes at once and shows on the next page.
- **Your men, a man's ground** (`ground_staff_man`): with nobody courted, or on any man you pick in the rail, his face, mood word with what lifts and weighs on him, style and why, his record with you (kids he wanted, passed on, was right on, overruled on), road days, the kids credited to him, his areas (each with his level there, why, who works it with him, head coaches known) with "Take off", the areas nobody works as buttons that give him one (up to 4 a man; it takes hold the next Monday: `ground_staff_areas` takes the whole list, so the screen reads his list first), the people around him (each opens the People page), what people say of him, the coordinator's letter, the men who left, the kids from areas nobody of yours works. A ride-along shows with a button to end it.
- **The rail:** every man along the bottom with his face, job, best area and level, mood word (amber "May go" if courted) and calls right. A click opens him. A man courted opens on his warning.
- **The top:** the date, your men, warnings (amber when any), areas with nobody, and the staff room.
- **Your desk** (`ground_desk`): your list this week (up to two kids; each with your case in your words, the film's word and your head coach's verdict: waiting on the film, signed, wait with his reason, or gone elsewhere; "Push back in his office" and "Let it ride" when he said wait; a kid he signs is marked your find); a form to bring him a kid from your leads with your case in a line (`ground_push`; the engine refuses a kid outside your area and group, or past two a week, and says why); his Monday plan for you (road days and area; "Plan your week" opens the week); his memory of you in his words with the calls judged, arguments and signed counts; your goals with how far along each is, the ground goals you can pick (`ground_desk_goal`) and his August verdict; what brings offers (your record, your group, your ground, who is courting you, your finds). His trust in your eye is words ("he listens when you push"), never a number.
- **What the commands add for the screens:** `face` (a drawn head from the recipe or the one kept in the save's folder, with initials and the key to keep it: `lane_ground_people::face_of`), `today`, ISO dates on the desk's memory, plan and pushes (the engine's day count never reaches the screen), `key` (kind and id) on each person on a man's ground, `counter` as `raise`, `title` or `promise` once made. A man's mood number is removed from his page.
- **Not yet:** the staff room's own page isn't redrawn (the "Open the staff room" button goes to the old one); a map in the "what leaves" panel (board 7b draws a map: the page reads the areas' levels as words and bars); the assistant's own area, group and week as separate pages in the sidebar (they stay the meeting, the map and the leads, which an assistant already reaches); the played scenes for a commit you call and for pushing back (lane G5's scenes don't cover them yet, so each is one line).
- **Test:** `crates/afm-game/tests/ground_staff_screen.rs` (the rail, a man's page with no mood number, faces, the desk's ISO dates).

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
- Your word (`word`): a promise to a kid judged kept +5 ("You did right by him, like you said."; one that lapsed or closed when you left the job is neither, `Promise::quiet`); judged broken −20, or an offer pulled −20, each a big moment held 3-5 years, and a broken word resets the Trusted clock. A promise made at your program now pays it, its head coach and you (an assistant's own word too); one made at another program you coached (judged when you left it, or a promise that you'd stay broken when you started here) pays only your own word, which goes with you, and spreads no further; one made at a job outside college football is no word to a high school coach, though his family still remembers it, on your own word. Each word is dated the day it was judged (one from a job away from college waits for your next college seat; its grudge runs from that day). A family holds a broken word against you when any of your staff's holders keeps it, yours included (`stores::family_points`), and a cousin family's good word doesn't lift a family you broke your word with (`from_cousin`) (`Promise::team`, `pay_to`; code oddity 29, proposed, not confirmed).
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
- **Not built here.** Your men's routes and faces, road days and the week's hours (lane G3), the overnight news in order of standing (lane G4), Friday nights on the map (GR5, needs G3's calendar), a stadium (the world has none). A coach's row now opens his person page (lane S5).

## What it does not do yet

- The AI working its ground: lane G9. Favors: lane G11b, switched off (below).
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

## Lane G11b: favors, switched off (GR22, GR60, GR63)

Every size here is the build's pick, proposed, not confirmed, to be tuned in GR61's trial (lane G12). The design is `docs/design/ground/placement.md`.

### The switch (GR61)

- `WorldOptions.favors` (`world/mod.rs`), false in every world and in old world files (`#[serde(default)]`). `favors::on` needs it and a college seat. With it off: `favors::day` returns at once, `place`, `ask` and `set_rule` refuse, the board says `on: false`, and every hook below takes its old path, with no new draw. So every seed plays the same.
- Your seat only. The AI's own placements and its spending of favors come with the AI working its ground (lane G9, GR65); the rules here take any holder, so it uses the same ones.

### A look costs a favor (`place`)

- You (or your coordinator) call a man on another college staff about a kid: a senior you passed on (not yet signed), a player of yours in the portal, or another school's player in the portal. A Division I staff's call comes only after he enters the portal. Any direction, up or down.
- Guards: a real hole on the taker's roster (`hole`: he would make its two-deep at his spot, or the spot is short of twice a 53-man roster's numbers scaled to the level, for next fall); one call per staff; three calls a kid a year (`MAX_CALLS`); one home a kid a year (one favor chain).
- His staff looks: a senior gets a call on his file (`scouting/hs.rs`, `Promo`), as hard as your word with that man is good (`word_read`: 0.5 until a claim settles, then toward your record, levelling off near six to eight claims, `WORD_SETTLE` 2). A portal kid's look lets a small school outside its region take him (`recruiting.rs`, `looked`).
- You owe the man one, or two in a scarce week (signing day and the day after, the early signing period's first three days, the portal window's last three: `scarce`); what he owed you is spent first. The call is 15 minutes on your week's ledger.
- A claim from the tips' words (worth a look, can play for you, a sure thing) and a worry you name (his grades, his temper, his health, his work). A worry reaches the kid's family that evening as a moment. A real worry you didn't name is noted at the call (temper 15+ or an incident, grades under 2.5, injury-prone 15+, work ethic under 6).

### A home found (`sign`, `hardness`)

- When he signs there (a portal kid: lands there), his coach owes you by how hard the home was: 2 for a scholarship where he had none at that level or above; 1 for a step up at the same level (a loyal coach counts a starting chance, a self-serving one the bigger name); 0 for a place any staff would give (junior college, Division III, an NAIA walk-on), a step down, or a kid whose offer you pulled. His best standing counts every offer he held, yours included.
- One comes at signing and the rest at the settle. A home in your coaching tree counts half; a second home from the same coach in a year counts one less. His coach's standing gains 4 a favor (`HOME_POINTS`), with a moment ("You found Simmons a home at Elon.").
- The no you placed: a pulled offer's 20 points come back to a quarter of the cost (15 back, outside the gates, and the pull is no longer held for years) when he has a home by signing day, unless he was committed to you or it is your second pull at his school in three years (`restore`).
- Your other calls for him lapse; a look lapses after 400 days or when he goes elsewhere.

### Your word settles (`settle`)

- After a year for a transfer and two for a high school signee, on his last season's role at the level you claimed: worth a look needs him to play (150 snaps or a starter or rotation role), can play for you a starter or rotation role at that level or above, a sure thing a starter. A pro holds every claim.
- Everyone who heard it records it: the man you called and the kid's coach, in your own and your program's memory (`your_word`, "your word held up 4 of 5 times").
- Held: the taker's man owes you one, two if he starts; the kid's coach pays the rest of the home and his standing gains 5. Not held: the rest never comes. A worry you hid that came true costs 12 with both ("You never mentioned Reyes's grades."). A quit or a move in year one takes back what his coach paid.

### A favor buys one answer one level up (`ask`, GR60)

- At Stranger his real list (his kids in his own order, by his eye), at Known the truth about one kid (his work, his grades, his temper, his health, in words), at Trusted his first call on one kid. Home buys nothing more.
- Never past the gates (GR28): no answer from a man you never met, no Trusted answer inside a year of Known or while he holds a broken word, no Home answer before a kid of his did well with you (or five years).
- The lift is stored (`Lift`) for the news on that kid. `news::make` reads `favors::lifted` and puts you at that level, behind the staffs already there (`Why::Favor`, "he owed you one"). `news::keep` spends it once the answer comes (`spend_lift`). With the switch off `lifted` is always None, so the order is G4's own.
- A self-serving man dodges 60% × (1 − 2 × his loyalty) of the time when a bigger program already holds that level with him: nothing is spent, and his memory keeps "he dodged your ask when Georgia wanted the same". Asking for more than he owes: he answers, it costs 3 points, and you owe him.
- It never buys a commitment, a signature or a transfer (rule 3).

### What you owe, and what he owes (`calls_in`, `refused`, `fade`)

- A man you owe calls it in through his weekly ask (`folk/network.rs`, `asks_week`): 6% a week, and always once the debt is 330 days old. His line opens "You owe me one, Coach." A yes pays it back; a no, or never calling back, is a broken promise: 20 points and a big moment he holds ("I called in what you owed me, and you said no."). A college colleague's tie cools by 0.15 instead.
- What he owes you fades by one each January after two years without a favor between you. What you owe never fades.

### Rule 3 in the portal

- With the switch on, a placement adds nothing to his choice and skips no depth check (`backchannel::bonds` leaves placements out); placing a kid goes through `favors::place` (`backchannel::place_player`).

### Your coordinator's rule (GR63, `coordinator`, `set_rule`)

- Each Monday your recruiting coordinator places the routine no's (the kids whose offers you pulled in the last 120 days, your players in the portal) with the rule's calls (default 1, up to 3), as "worth a look", to the programs with a hole (`takers`: his home state first, near his best standing, then the bigger name). The favors are his own.
- A kid from a school where your staff is Trusted or Home comes to you instead, as a memo ("Yours to call: ...").

### Your leavers (`leavers`, `judge_leavers`)

- Your players who enter the portal are noted on Mondays. On September 1 his high school coach judges: helped to land (placed and landed) +3 ("You helped Reyes land at Troy."), drifted out of football −4 ("Reyes drifted after he left you."), landed on his own nothing.

### The board and the commands

- `favors::board`: your haves (pulled offers without a home, your players in the portal), your calls and where each stands, who owes you and whom you owe in words ("He owes you a big one, for a look at Simmons"; your coordinator's marked), and the rule.
- Game commands (`crates/afm-game/src/lane_favors.rs`): `ground_favors`, `ground_favor_takers` (`kid`), `ground_favor_place` (`kid`, `coach`, `claim` 0-2, `worry`), `ground_favor_ask` (`kind`, `id`, `kid`), `ground_favor_rule` (`calls`). Off, every call but the board refuses. No screen yet.

### What G11b does not do

- The AI's books, placements and spending (lane G9), access asks (his juniors at your camp, a word in a job search), stashes at a junior college or prep school, the eras' rules (a release before October 15, 2018, grayshirts under the 25 cap, NIL as a step up from February 2024), the era's warnings on asks for a prospect's relatives, brokers placing kids, and the NFL and high school seats.

### Where the knobs are

- `world/ground/favors.rs`: `HOME_POINTS` 4, `MAX_CALLS` 3, `LOOK_DAYS` 400, `SETTLE_TRANSFER` 365, `SETTLE_SIGNEE` 730, `DODGE` 0.6, `OVERASK` −3, `PLACED_NO_SHARE` 0.25, `SECOND_PULL_YEARS` 3, `WORD_SETTLE` 2, `CALL_IN_BY` 330, `CALL_IN_WEEK` 0.06, `OWED_FADE` 730, `LEAVER_HELD` 3, `LEAVER_DRIFT` −4, `DEFAULT_CALLS` 1; the worries' lines in `real_worry`; the hole in `hole`; a scarce week in `scarce`.

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
- **The scenes** (GR51): the meeting, an official weekend, a junior day, your camp, unofficial visits, your calls and your own road days, and (lane G5) your clinic, the spring game, the convention and open practices, ranked (the meeting 9, officials 8, junior day, camp, the clinic and the spring game 7, a slipping coach's call and your road 6, the convention 5, unofficials 4, a call 3, an open practice 2); the top five are yours, the rest your coordinator's. Lane G5 plays them (`plan::draft_slots`, `scenes::coach_slots`). Capital lane C4 adds your donor slots to the same ranking (a booking 8, the suite before a home game 7, a lead donor 6, the caravan and the golf outing 5, another donor 4; `capital::week::slots`), played all year by Capital (`capital.md`, "Lane C4").

### Road days (GR48) and trips (GR49)

- **The windows:** the spring (contact, or evaluation before 2024, in April to June), the fall evaluation period (September to November) and the winter contact periods (`road::window`). The days come off `Dept.eval_days` as the built ones did, so the department screen's squares still count them.
- **A man's day:** his focus schools first, then the schools with the most board need, then the area's schools by talent where he stands lowest with the coach; a school he saw this week goes last (GR54: a second visit buys half anyway, in `standing::contact`). Each board recruit there gets a look (practice; a fall Friday's game), and the coach's time counts through `on_look`; at a school with none on the board the coach and his coordinators get the visit directly.
- **The money:** see the quick answers. With no books yet the tier's usual line stands in (Power $5M, Group of 5 $0.8M, FCS $0.3M, below $0.05M), and older years scale by `hs_index`. Your AD's raise grows the line, so it buys flights (`tests/recruit_budget.rs`).

### One ledger (GR50)

- `Week.ledger`: each entry's day, what, hours, and whether you chose it. Chosen hours (calls) come out of the week's free hours film uses (`film::time::used` adds them); booked hours (your road day, what you host) don't, but every hour counts for the day's tiredness (`film::time::today`) and booked hours past 12 a week count for the week's wear (`film::time::week`, `BOOKED_FREE`).

### The beat (GR56), the letter, the May list (GR57), the week off (GR58)

- **The beat:** week 1 holds January 1. Each week lists what is on the calendar (the portal, the coaching changes, the services' updates, signing day, junior days, the combine, spring practice, officials for juniors, the spring road's first and last day, the spring game, the draft, term's end, Memorial Day, the May list, your signees, your camps, the transfers, the dead period, NIL payday, the Fourth, media days, the polls, fall camp, high schools, Week 0), read from `YearDates` and `recruiting_period`, and picks the beat and the stake by what is in it (`beat::beat`). The 2026 reading matches `ground/months.md`'s sheet; weeks 3 and 7 (the coordinator at your door, the February meeting) come from lane G6 (`campaign`), as do the AD's talk and the verdict.
- **The Monday letter:** the beat line first ("Week 16: the road opens Wednesday, the spring game is Saturday."), then what last week bought (new names on the board, levels reached, school visits and recruits seen, your calls), the road days left, the week's flights, the plan, your scenes and your calls, and the stake. The recruiting desk's Monday digest opens with it and carries its subject (`crates/afm-game/src/lane_core/mail.rs`, `people_mail`).
- **The May list:** the day after Memorial Day, each man's rising seniors he saw in person since April 1 (not signed, not committed elsewhere), ranked by your offer, then stars, then how many of your men saw him; your open official spots are the weekends from Friday to the dead period × 10 at FBS (6 FCS, 4 below), less those set. A memo from your coordinator; `may_invite` books the top on each one's first open weekend through the built visit rules (`plan::may_list`, `may_invite`).
- **The week off:** one or two weeks inside June's and July's dead period (`week::take_off`): your eye's wear goes to zero the first day, no film, your calls go to your coordinator and the calls back he takes count as his (`phone::lapse`).

### What it does not do yet

- Lane G5 plays the scenes (below). The goals are lane G6's (the May list ranks by them); your men's own ground and areas on the map came with lane G10 (below), the AI's weeks with lane G9.
- The 2027 recruiting model (one pool of 300 from January to May) isn't in the game's rule book; `road::pool` is where it goes.
- FCS runs the FBS recruiting calendar (`recruiting_period` has one).

### Where the knobs are

- `week/road.rs`: `ROAD_SHARE` 0.12, `ROAD_WEEKS` 24, `DAY_COST` 180, `MILE_COST` 0.70, `FLIGHT_COST` 700, the stars' `WEIGHT`.
- `week/plan.rs`: `DAYS_PER_MAN` (3, 2, 3), `CALLS` 5, `YOURS` (3, 5), `PER_WEEKEND` (10, 6, 4); the second man past 12 of need.
- `week/phone.rs`: `CALL_HOURS` 0.25, `WAITS` 60 minutes, who picks up (his hour's 50 minutes, a coach's evening 7 to 9:30 PM).
- `week/mod.rs`: `BOOKED_FREE` 12; the hosted hours in `hosted`.

## Lane G5: the scenes

Every size here is the build's pick unless the spec gave it, proposed, not confirmed. Code: `crates/afm-engine/src/world/ground/scenes/` (`mod.rs` the store, the eye's roll, the life of a scene and the room for live talks; `visit.rs` the school visit; `calls.rs` the meeting, a coach's call and the night calls; `campus.rs` junior day, the visits, the spring game, your camp and the announcement; `coaches.rs` your clinic, open practice and the convention); the line bank `data/lines/ground.txt`; game commands `crates/afm-game/src/lane_groundweek.rs`. Only your seat plays them, so the world plays the same on every seed.

### How a scene plays (GR32, GR52)

- **One folk scene kind,** `scene::Kind::Ground`, with Ground's own kind in `Scene.b` (`scenes::Kind`, the 15 of GR55). Its running state (the person, the school, the kids, the stops, standing moved, the people gathered) is a table titled "ground:state" on the scene's docs that the screens skip (`state`, `is_state`).
- **Where they come from:** at 5 AM, each of today's slots that is yours becomes a scene at its hour (Continue stops for it; at the next office hour it goes to your staff if you haven't held it, the built rule in `folk::scene::day`). Your coordinator's slots of yesterday play on his skill and get their line (`dawn`, `staff_plays`; his road days are the road's own). Only in Ground's months: January after the takeover's weeks to August, and not in a week off (`in_season`). A night call that matters becomes a scene after the road's day (`after_road`, `calls::night`).
- **What you notice** rolls on your eye (your evaluation, less the day's tiredness) the way a film catch does: 0.30 + 0.035 × (eye − 10) − 0.40 × how hard it is to see, 0.04 to 0.95, × (1 − 0.5 × tiredness). A 16 eye catches a plain thing about half the time, a 9 eye about a quarter (`notice_chance`; keyed, so a scene rolls the same). Your man's eye when he plays it.
- **The talk:** the scene's picks (chips), with the line bank's greetings when there's no key. With a usable key a typed line plays on live talks v3 with the person across (`folk::persona::meetings`, `Kind::Ground` in `KINDS`; who is across in `scenes::room`): the coach in his office or on the phone, your coordinator at the map, your man on the night line, the families on campus or the coaches at a clinic, sideline, camp or convention as a room, the kid and his family the morning he decides. What a line does to each person's trust lands as standing at 50 points a unit of trust, inside the gates (`after_talk`, `TALK_POINTS`).
- **A pick that gives him what he wants** (people.md's wants: asked about his kids, the truth, a clinic talk; a family's questions answered by the head coach himself) is 2 points × his warmth with your program and your head coach, inside the gates, with his words kept as a moment (`wanted`, `WANT`, `standing::grant`).
- **What you walk away with** (rule 1): every scene ends on one line, kept in `Ground.scenes.played` (the last 300) with its kids, new names, standing moved and stops, and on its slot (`line_for`).
- **Handed on,** your coordinator plays the default stops on his skill with people and his eye: one line (GR32). The hand-off button names him (`hand_label`, `folk::handoff`).

### The scenes

- **The meeting at the map** (Monday 8 AM): his draft on the map; go with it, change it at the map, or hand him the week (`plan::hand`).
- **A school visit** (GR72, board 3; your road day's first school, `road::first_school`): four hours. The sign-in sheet first (every college at this school's kids in the last 60 days, by their offers and their kids' campus visits, newest first: `visit::sheet`). Then the stops in any order while the hours last: his office (60 minutes, two exchanges: ask about his kids for his list by where you stand, tell him straight where his best kid stands, ask him to your clinic, sell your program), the weight room (30: your eye on the six best kids nobody of yours has, each a lead you saw yourself when it catches: `leads::seen`), the counselor (30: transcripts), practice in April, May and the fall (60: a live look). Leaving is the visit with him (and his coordinators sitting in), his list if you never asked, and a visit on the week's log so the road doesn't send a man back the same day.
- **A coach's call** (at his hour; GR53): he picks up or the call waits, the greeting by your level, his newest news, then ask, straight, the clinic invite.
- **The night calls** (GR13, GR14): your man's night call that mattered; send him back later this week (one more road day, inside the rules), get his tape, ask what he saw himself (his eye's words), or let him sleep.
- **Junior day, an unofficial, an official weekend (Friday to Sunday), the spring game** (your campus): the kids on campus, your offer first. Sit with three families (your name with them and the question their top want asks), hear a kid's player host, offer on the spot (`front_office::college::offer`), and at the spring game an hour in the coaches' suite (the coaches within 400 miles who know you). From 2027 the spring game is yours to hold or skip (`set_spring_game`); a skipped one makes no slot and its Saturday's visits stay unofficial.
- **Your camp** (June's camp days): the campers (`dept::campers`, the department's own pick), walk the stations (your eye catches the walk-up nobody listed), coach a group, sit with the coach who brought the most, offer on the spot; the coaches who brought kids get the event.
- **The announcement** (7:30 AM, the morning a kid decides): yours flipping first, else the biggest on your board who committed since yesterday, one call a morning. Welcome him, be gracious, or keep the door open until he signs.
- **Your clinic** (the Friday of spring practice's third week, week 12 in 2026): 80 coaches at the smallest program to 300 at the biggest, by prestige, from within 400 miles, the ones who know you first and every coach you asked. Your hour on the stage is an event with every coach in the room (+3 × his warmth, the spec's table) × the scheme fit: 1.0 + 0.5 × how well he knows your offense, or your front and coverage, so the ones who run it warm fastest; a recruiting talk is 1.15 with everyone. Then the hallway: the coaches who know you (up to five) give their lists, and your eye catches who sells a name too hard (straightness under 9) or talks his kids down (15 and up).
- **Open practice** (Tuesday 4 PM of the other spring practice weeks): 8 to 30 coaches on your sideline from within 150 miles, half an event each; a quarter of them bring their best junior, and your eye can catch him at the fence (a lead you saw yourself). The walk to the lot is the hallway.
- **The convention** (the Tuesday after the third Monday of July, week 30): your state's head coaches, about 55% come and your session draws 25% to 75% of those by prestige; the talk and the hallway as at the clinic, with no scheme multiplier.
- **A recruit's call and film** are the built ones (a recruit's page, the film room).

### The commands (`crates/afm-game/src/lane_groundweek.rs`)

- `ground_week`: each slot carries `scene` (the folk scene to play) and `sceneKind`; its line is what the scene came to.
- `ground_scenes`: the 15 scenes, the last 40 played (line, kids, new names, standing moved, stops, where he stands now), the spring game's date and whether it's your call, the clinic and the convention dates, the coaches you asked to the clinic.
- `ground_scene` {id}: one scene's ground part for the school visit screen (lane S3): its kind, time, the person and his level, the school, your man, the minutes left, the stops so far, the kids with the new names marked, standing moved, the tables (the sign-in sheet, his list, who came) and the hand-off. Lane S3 added for the visit's page: `at` (the stop the talk is in, "office" or empty), `start` (the minute it began), `minutesTotal` (240 for a visit), `eye` (`words`: sharp, good, fair or weak, and `tired`) and `manIsYou`.
- `ground_spring_game` {hold}. A scene plays on the people commands (`people_scene`, `people_answer`, `people_hand`, a typed line).

### What it does not do yet

- The school visit has its page (lane S3, below); the other scenes play on the scene pages the game already has.
- The 2027 practice periods aren't in the calendar: the clinic and open practices use spring practice's dates every year.
- The campaign's goals (lane G6) don't weigh which scenes come to you yet; the ranking is G3's.

### Where the knobs are (G5)

- `scenes/mod.rs`: `WANT` 2.0, `TALK_POINTS` 50, `KEEP` 300, `SPRING_GAME_CHOICE` 2027, the notice formula.
- `scenes/visit.rs`: `MINUTES` 240, the stops' minutes (60, 30, 30, 60), two exchanges, `LOOK_AT` 6.
- `scenes/campus.rs`: three families a day.
- `scenes/coaches.rs`: `CLINIC` (80, 300), `CLINIC_MILES` 400, `SIDELINE` (8, 30), `SIDELINE_MILES` 150, `SIDELINE_SCALE` 0.5, `CONVENTION_COMES` 0.55, `SESSION` (0.25, 0.75), `RECRUITING_FIT` 1.15, `HALLWAY` 5, `BROUGHT` 0.25.

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
- **Not yet:** your man's mood (no staff mood store yet: lane G10), your name as a finder read by athletic directors and coaches (lane G7), pipelines that grow from blooms (the built pipelines grow from signees), the race's signs (lane G9), the ground report's screen (lane S8; its data is lane G6's `report`).

### The commands (`crates/afm-game/src/lane_leads.rs`)

- `ground_leads`: your open leads, newest first: the kid (your staff's read, potential in words), how, who told you, his claim, blanks filled, whether he's a find, his word, how much news.
- `ground_lead` (`id`): the lead card: the kid, his offers and stars, who told you and what he said, the teller's standing, why and record, your man's finder record, the measured size once measured and his recorded times, the five blanks (who filled each and when, asked, what fills it and its time), each piece of news with its order and why, who else knows, his find, and your word (offer now, wait for his junior tape, fill the next blank first, each with what it risks; the bloom steps with what each pays, in words).
- `ground_fill` (`id`, `blank`: `size`, `speed`, `tape`, `live`, `character`), `ground_lead_drop` (`id`), `ground_finds` (finds with blooms and setbacks, misses, your men, your name as a finder).

### Where the knobs are (G4)

- `world/ground/news.rs`: `ORDER_KEPT` 8, `KEPT` 1500, `CALLS_A_DAY` 3, `SMALL_CALLS` 8, `BUSY` 0.5, `LEVEL_DAYS` [0, 1, 3], `public_lag`, `fast`, `FAST_100` 10.75, `GREW` 1.5, `big_line`, `region`.
- `world/ground/leads.rs`: `KEPT` 3,000, `LIST_EVERY` 30, `LIST_MAX` 6, `teller` (agendas), `numbers` (the shading), `truth`, the board's weights in `Board::of` and `Board::near`, the oversold margins in `measured` (1 inch, 0.10 s).
- `world/ground/finds.rs`: `TELLER` [2, 4, 8], `FINDER` [0.5, 1, 2], `low`, `LOW_OFFERS` 5, `pay`, `MISSES` 12, `MISSES_KEPT` 60.
## Lane G6: the campaign (part H, GR30-GR41, GR59; the report's data, GR77)

Every size here is the build's pick unless the spec gave it, proposed, not confirmed. Code: `world/ground/campaign/` (`mod.rs` the store, the dates, the day, the meeting, Monday changes, the verdict and what it pays; `goals.rs` the 63 goals and their readers; `motives.rs` your AD's motives; `talk.rs` his talk; `report.rs` the ground report); game commands `crates/afm-game/src/lane_campaign.rs`. Only a college head coach's seat has a campaign (`campaign::seat`); nothing in the world reads it, so the world plays the same on every seed.

### Quick answers

- **When does a spring run?** From the AD's talk the day after signing day (`Spring::talk`) and the February meeting the Monday after it (`Spring::meeting`) to the verdict, the Monday of the week before your first game (`verdict_day`; the season's Week 1 when you have no game yet). The class is next year's (`class_now`: this year's until signing day, then the next).
- **What is a goal?** An `Aim` (goal 1 to 63, a number `n`, what it's about `of`, the first class), read every day against the world (`goals::read`): a share of the way there, its words ("2 of 3 blue-chips committed") and whether it can only be judged later. The list of 63 by group, who raises each, its classes (one, or two for the two-class goals) and whether it's judged later (a starter, an All-American) is `goals::GOALS`.
- **The scale:** Done at the whole way, Mostly from two thirds, Partly from a third, else Missed (`Verdict::of`); a later goal shows the pace and settles when the class has played.
- **The slots** (GR33): Needs (your coordinator's, from the plan's biggest hole: `needs_goal`), your AD's (one, two if he meddles 12+ or has patience 8 or less: `motives::held`), Yours (your coordinator's ground goal until you pick: `ground_goal`, hold an area you lead, take one you're close to, open one where nobody knows you) and the Hunt (goal 44, on or off).
- **Your AD's talk** (GR35, GR36): a boss meeting the day after signing day (`talk::schedule`, `Scene.c == 4`). He reads his motives (the conference bar, the national rank at a power program, the generational kid, the state's best and the fence at a flagship, the hype kid, a famous name, the donor's old school, the rival, clean, qualifies, the city's best, a reach: `motives::motives`) and puts each one he holds you to on the table: take it (+0.02 trust), argue it down (he gives by his patience and his trust in you, else −0.01), take it bigger (+0.04 and a word for recruiting money: granted at trust 0.35+), swap it for one of his wishes (when he has patience 8+ or trust 0.55+, else −0.01), or ask why (his reason, the goal stays on the table). Skipping it costs −0.04 and his goals stand. With a key it plays as a live office talk. The goals become his boss goals for the season (`GoalKind::Campaign`), and his review reads them; the class rank goal he used to draft isn't drafted that year (`boss::draft_goals`, `ad_set`).
- **The February meeting** (GR30): your coordinator's memo "The February meeting: our goals for the class of {class}": the class you signed and its rank, the Needs goal and why, your AD's word, a ground goal and why, the hunt, and that Mondays change them. Your ground's area levels are kept for the report.
- **Changing them** (GR37, GR38): only at a Monday meeting. `pick` any goal on the list for Yours (or the hunt): the one you held counts as a miss with whoever you told (`drop_at`). Your AD's can't be dropped. A two-class goal holds its slot until its last class signs.
- **Who knows** (GR39): your staff always; tell your AD (`tell_ad`) or name it at media days (`name_at_media_days`, July's media days only), and it's public.
- **The race** (GR40): every Monday each goal's reading is kept, and a memo "A goal is slipping" when one drops a step from Partly or better. The Monday letter carries one line per goal (`letter_lines`). The May list ranks a kid who counts toward an open goal ahead of the rest at the same offer (`serves`). Flips in and commitments lost are tracked every day for goals 8, 39, 40 and 42 (`track`).
- **The verdict** (GR30, GR41): the memo "The verdict at the map": each goal on the scale (the pace for later ones), the wishes you met, your AD's line and the paper's. A goal pays by how likely it looked when you picked it (`worth`: a sure thing pays 0.4 and costs 1.2, a long shot pays 1.3 and costs 0.4); a bold goal (0.15 or more less likely than your coordinator's ground goal) pays × (1 + how much bolder, up to 0.6), and your AD reads a told bold goal by your level (`salt`). What moves: your AD's trust (his goals 0.04 a step, yours he knows 0.025, public ones 0.015 more, a wish met 0.015), your staff's warmth 0.02 a step (0.02 more lost on a bold miss), and 0.8 standing points with the coaches who know your program (Known or better). A little each, never a firing on its own. Classes settle the day after their signing day and later goals at each signing day and verdict after (`settle`, "Your goals" note).
- **Your first offseason** (GR31, GR59): with a January takeover, Ground waits until the January window closes (`waits`; the week doesn't turn). Then your coordinator is at your door (the memo "Our ground"): where you stand, three schools in one area (one where the coach knows you, one where he knows the program, one with a kid he's heard about: `guide_schools`), the road day he'd put you on (`guide_go`) and that you can send him off (`guide_off`). The tools open as you need them: the map that day, the leads board with your first lead, the rivals page with your first flip or loss, the week planner at the February meeting (`tool_open`); everything after the verdict.
- **The ground report** (GR77, for screen S8): built at the verdict (`report::build`, `Campaign.reports`, the last 6): its title ("Your first spring"), each goal's line and why, your finds, your areas February to August, the headline, who outworked you in your areas (visits by program from the meeting to the verdict), your men, the miss, the setback, your AD's line and the paper's.

### Stand-ins until their lanes

- No staff mood store yet (lane G10): your staff's warmth goes through the network's warmth.
- G8's loud news isn't built: a famous name reads the NFL bloodline, a loud commitment (goal 27) a top-10 rank or top-10 followers.

### The commands (`crates/afm-game/src/lane_campaign.rs`)

- `ground_campaign`: this spring's goals by slot (reading, verdict so far, how likely it looked, bold, who knows, parts by class), past goals, your AD's wishes, the guide (his three schools, the tools open), the dates, whether today is a Monday meeting or media days, the reports.
- `ground_goals`: the 63 by group, who raises each, its classes and how likely it looks for your class today.
- `ground_goal_pick` (`goal` as number or key, `n`, `of`), `ground_goal_drop` (`slot`: `yours`, `hunt`), `ground_hunt` (`on`), `ground_goal_tell` (`slot`), `ground_goal_name` (`slot`), `ground_guide_go`, `ground_guide_off`, `ground_report` (`year`, the newest by default).

### Where the knobs are (G6)

- `campaign/mod.rs`: `AD_TRUST` 0.04, `TOLD_TRUST` 0.025, `PRESS_TRUST` 0.015, `WISH` 0.015, `STAFF_WARMTH` 0.02, `BOLD_MISS` 0.02, `NAME_POINTS` 0.8, `worth`, `salt`, the bold margin 0.15 and cap 0.6 in `hold`, `chance_words`, the ground goal's scores in `ground_goal`.
- `campaign/talk.rs`: the talk's trust moves and `yields`. `campaign/motives.rs`: each motive's bar and weight, `held`, `conference_bar`. `campaign/goals.rs`: each goal's reader, the starter bars in `fits` (68, 61, 54, 48).

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

## Lane G8: a following and famous names (GR42-GR46)

**Famous men.** A world player is famous from his career and books: a Hall of Famer 1.0; an MVP, Offensive or Defensive Player of the Year, Super Bowl MVP or three first-team All-Pros 0.85; a first-team All-Pro, four Pro Bowls or a Heisman 0.65; a Pro Bowl 0.4; anyone else 0 (`world/following.rs`, `man_fame`). It reads his alumni record's pro career when he has retired, his live career when he still plays (`World::alumni`, `Alumnus::pro`), the Hall (`nfl_book.hall`) and the Heisman (`stats.honors`). Only men in the world's books count: the recorded years from 1996 on in a 2026 world, so the first famous sons arrive around 2012.

**His kin.** Keyed on the man (his player id): 45% one son who plays, 13% two, 42% none; each son 14 when the man is 36 to 50 (even odds each year). A nephew for 30%, 14 when the man is 30 to 55 (`kin`). All proposed, not confirmed.

**The class day.** After the new class is in (`World::add_class`), each famous man's kin due this year joins it (`names`). A son is a boy of the new class whose father played in the NFL (`Bloodline::NflPlayer`), of the man's ethnicity, in the man's home state if one fits there, any state otherwise; a nephew a boy whose father didn't play (`Bloodline::None`). A boy already your former player's son is skipped, and your guys' sons skip a famous name (`world/legacy/guys.rs`, `sons`). The pick is keyed, so a seed always makes the same family. A son takes the man's surname; about one in eight is named for him ("Darnell Hayes Jr."). A nephew takes the surname two times in three (a brother's son), keeps his own otherwise (a sister's). Stored in `World::following.names` (`Name`: the boy, the man, son or nephew, the man in words, his fame, the hype).

**What the name does.**
- Fame: the hype adds to his fame like an honor, 0.35 × the man's fame for a son, 0.18 × for a nephew, in high school and college; half of it as a pro (`world/fame.rs`, `fame`). A Hall of Famer's son adds 0.35, about what a fifth star adds in high school (0.55 for 5 stars against 0.32 for 4). Followers, NIL brand deals, NIL Go's range and a storyline's weight all read fame, so all of them follow.
- Staffs notice him: his recruiting visibility takes 0.15 × the man's fame more (half for a nephew), beside the NFL bloodline's 0.15 (`world/recruiting.rs`, `recruit_visibility`).
- His file: "His father is Darnell Hayes, a four-time Pro Bowl receiver." leads the family notes, in place of "His father played in the NFL." (`world/scouting/lines.rs`, `family_notes`). The news says "son of four-time Pro Bowl receiver Darnell Hayes" (`news_name`, `Name::tag`).
- What it does not do: his ratings, ceilings and stars are the boy's own (a son keeps the NFL bloodline's small edge he already had).

**The class's biggest followings.** At February's book, after followers move, each high school class's biggest and tenth-biggest following is kept (`World::following.classes`, `class_tops`), and every high schooler's spotlight is set: his following ÷ his class's biggest, 0 to 1 (`Person::spotlight`; 0 out of high school). A world file from before this lane fills them on load (`after_load`).

**News (GR43).** A commitment, a flip and a signing-day flip post their news when he has 4 or 5 stars, as before, or a following in his class's 10 biggest (and at least 10,000, `NEWS_FLOOR`). Loudness is the louder of his stars' (5 stars medium, else small; a signing-day flip medium) and his following's: big for the 10 biggest, huge for the biggest. The news then adds "He has 412,000 followers." (`commit_news`, `followers_words`; `world/recruiting.rs`, `commit_now`, `run_signing_stage`). Big news gets its own screen and stops Continue; huge is the full show (`calendar/events.rs`, `Loudness`).

**Brand exposure (GR44).** A commitment (or a signature without one) from one of the class's 10 biggest followings (and at least 10,000) lifts the program's brand exposure grade by 3 × his spotlight (`committed`, `committed_in`). Each morning (5 AM, after the class day) a lift ends after 365 days or once he no longer holds it (committed to, signed with, or in college at the program), and each program's grade carries its biggest lift, at most 3 points and never past 99 (`day`, `set_lift`, `LIFT_TOP`, `LIFT_DAYS`; `Program::brand_lift` is what is in the grade now). At the year's grades the lift comes off first, so the grade moves from its own read, and goes back on after (`world/staff.rs`; `strip`). The top-10 rule and the biggest-not-sum rule are proposed, not confirmed: with every commitment lifting and the lifts summed, a seed 152 test world (scale 0.05) had 515 programs lifted, most at the 3-point cap.

**His pick (GR45).** In a school's appeal, the prestige part is (title chances + stadium + tradition + w × brand exposure) / 4 with w = 1 + his spotlight: 1 for a boy nobody follows, 2 for the class's biggest following (`world/recruiting.rs`, `stage_weight`, `utility_fit`, `six_parts`). A transfer's spotlight is 0, so the portal weighs it once, as before.

**Storms (GR46).** Each week a college player, high schooler or prep player with 500,000 followers or more rolls the pro's storm: 0.15% × (1 + his temper's z above 0), about 1 in 670 a week (`world/storylines/detect/weekly.rs`, `chances`; `STORM_FOLLOWERS`). The story ("A post that blew up") is open at every level. High school followers top out at 400,000 × reach, so a recruit reaches it only in a big-reach year or as a famous name with Aura. College followers were retuned on 2026-11-16 so the biggest college names get there: about 7 to 18 college players over 500,000 on the seed 1 world, the biggest near a million (`world/fame.rs`, `COLLEGE_TOP`, `COLLEGE_POWER`, `college_honors`; people.md, "Fame, Aura, followers and brand money"). At 1 in 670 a week, ten such players give about one college storm a year. Seed 1 played on 600 days to mid-2028 with games on the stand-in (`follow_check`): 8 college players over 500,000, the biggest 1.02 million (a five-star receiver at Texas A&M), the 10th 463,000, 462 over 100,000; no college storm came in that time (under one was due at 1 in 670 a week, so none is within chance), 13 NFL storms kept.

**Measured (2026-11-16).** On the seed 1 world (history on the engine) played on 600 days with games on the stand-in (`crates/afm-tools/src/bin/follow_check.rs`): 31 famous names joined the 2026 class and 28 the 2027 class (43 sons, 16 nephews; by the man's fame 24 at 0.4, 17 at 0.65, 12 at 0.85, 6 at 1.0); a Hall of Famer's son led his freshman class's followings (11,410); 9 programs held a brand lift in mid-2027, the biggest Florida's +2.56; no college player or recruit had 500,000 followers, so no storm came at those levels (14 NFL storms kept). Seed 151 at a tenth of the size, this lane against main with history on the stand-in: the two worlds are the same until 2008 (no followers before social media's reach), and recruiting after stays within the noise of a world that small (5-stars drafted 70% against 77% of 91, 2-stars 1.3% against 1.4%, FBS players from the school's state median 48% in both). The stand-in history gives no Pro Bowls, so it has no famous names.

### Where the knobs are (lane G8)

- `world/following.rs`: `HYPE_SON` 0.35, `HYPE_NEPHEW` 0.18, `FAMOUS` 0.4, the tiers in `man_fame`, the odds in `kin`, the Jr. 12% and the nephew's name 65% in `names`, `LIFT_TOP` 3, `LIFT_DAYS` 365, `BIG_NEWS` 10, `NEWS_FLOOR` 10,000, `STORM_FOLLOWERS` 500,000.
- `world/recruiting.rs`: `stage_weight`; the visibility's 0.15 in `recruit_visibility`.

## Lane G10: your staff's ground and the assistant seat

Every size here is the build's pick unless the spec gave it, proposed, not confirmed. Code: `crates/afm-engine/src/world/ground/staff/` (`mod.rs` the store `Ground.staff` and the day, `beats.rs` areas and ride-alongs, `men.rs` each man's ground, mood and a candidate's signs, `carousel.rs` a man courted and a man gone, `desk.rs` the assistant seat); game commands `crates/afm-game/src/lane_groundstaff.rs`. Only your own college seat has it: every hook returns nothing without a career, so a careerless world plays the same on every seed. In a seat world what moves is your own file (which of your men makes a look: the man on the area goes) and, with you acting, what you do; the world outside your seat doesn't (`tests/ground.rs`, `the_world_plays_the_same_with_the_ground`).

### Areas for each man (GR68)

- **Who works areas:** your on-field assistants, coordinators and position coaches, not the head coach (`staff::men`). Each keeps his position group too: the board's group, the film and the role pitch are the built ones.
- **The coordinator's proposal** on the first day of a job and at the February meeting (the Monday after signing day, `february_meeting`): the areas worth most to your program (the board's need there, then the talent its schools control times how far you reach: your name in the state, the drive), each to the man with the best claim on it: his own standing with its head coaches, weighted by talent, plus 12 points on his home state, less 8 for each area he already has. Up to 3 areas a man; a man left with none goes as a second man on the area with the fewest men, biggest first. The rest stay empty and the letter names them ("Nobody on Gwinnett: what comes out of there, we'll hear about late.") (`beats::propose`, `apply`).
- **Your changes, any Monday:** `set_areas` (the head coach's seat only, up to 4 areas a man), planned from the next Monday. The department's state command now moves every area of the state (`dept::set_area`, `beats::set_state`); its state screen reads the man who holds most of the state's areas (`dept::area_coach`, `beats::state_man`).
- **What reads it:** the week's plan sends each man to his own areas, the neediest first (`week::plan`, `draft_roads`), and the man on a school's area who stands best with its head coach recruits its kids (`dept::recruiter_for`, `beats::recruiter_for_school`). Without areas set, both fall back on the built state split.
- **A man who leaves:** his areas wait for Monday, when the coordinator gives each to the man with the fewest until you or the February meeting say otherwise (`beats::orphans`).
- **The ride-along:** a younger man rides with an older one for up to 8 weeks. Each day the veteran is out, the head coaches and coordinators at his schools meet the young man too, as his own memory (a sat-in contact), so he keeps them when the veteran goes. It costs both men's road day (`beats::ride`, `ride_day`; `week/road.rs`).
- **Credits and misses:** each Monday and signing day, each of your signees is credited to the man on his area; a 3-star or better from an area nobody of yours works who signs elsewhere is a miss, for the ground report (`beats::signees`).

### Each man's ground (GR68, GR76)

- **His page** (`men::ground`): his areas with his level there (from his own memories with the area's head coaches), his people at Known or better, his finds, his road days this season, his style and his mood.
- **His style** from his makeup: a salesman (straight 8 or less, relationships 11 or more), a straight shooter (straight 14 or more), a closer (recruiting 14 or more, ambition 12 or more) or a grinder (`men::style`).
- **His mood**, from 0.6: overruled more than twice in two years (−0.07 each past two, up to four), an area he didn't want (all new ground while another man works his home state, −0.08), a long spring (more than 30 days on the road, −0.08), passed over for coordinator in the last two years if he's ambitious (12 or more, −0.12), a promise you broke (−0.15); finds that bloomed (a 3-star or less of his who started or was drafted, +0.05 each, up to four), a title (+0.04), a raise this year (+0.04). The page shows the words and the reasons, never the number (`men::mood`).
- **His mood in the carousel:** an offer to one of your men weighs 0.06 × (0.5 − mood) more or less, so a happy man stays and an unhappy one listens (`staff::weigh_offer_at`, `ground::staff::offer_pull`).
- **What leaves with him:** each of his areas' level for your staff today and with him gone (the program's own ties and your other men stay) (`men::what_leaves`).
- **Hiring for ground, in signs** (`men::signs`): a candidate's old jobs (public); where his ties are, named by area only where someone on your staff is Known there, else just "can't place" in the state; what the people you're Trusted with say about him; and for a high school coach, what his school's trust is worth and the hiring rule. Never his numbers.

### The carousel for your men (GR76)

- **The warning:** when another school's search reaches one of your men and he means to go, the job waits 3 days for your counter and your inbox has the warning with his line in the hallway (`carousel::courting`, read by `staff::fill_opening`). Only your own staff's men, never a move inside your own program.
- **Your counter, one per offer:** a raise (what the new job pays over his, from 5% to 30%, out of your staff money; the room hears it), the assistant head coach title (the next hire wants one too), or a promise of the next coordinator job (tracked like hiring promises: break it and his trust goes). Each counter's reach is 0.5 + 4 × (what it's worth to him − what the job pulls) + 0.4 × (mood − 0.5) + 0.02 × (loyalty − 10), from 5% to 95%, shown in words ("a coin flip"). The title and the promise count for more with his ambition; all count half against a head job. The roll is fixed per man and day, so it doesn't move with the order of the day's work (`carousel::reach`, `counter`). `let_go` sends him at once; no answer by the third day and the search hires him.
- **What keeps him:** what lifts his mood, his loyalty (13 or more), being home in your state, 4 years or more on your staff (`carousel::keeps_him`).
- **The coordinator job:** when a coordinator job on your staff goes to someone else, a man you promised it to has it kept or broken, and an ambitious position coach left out is passed over (`carousel::roster`).

### When a man goes (GR68)

- **His ties go with him:** the memories are his, so your staff's level in his areas falls to what the program and your other men hold.
- **The terms:** good (his mood 0.45 or more), grateful (0.7 or more, or a promise you kept), angry (a promise you broke, let go while unhappy, or his mood under 0.3) or neither (retired, or let go). Good or grateful, he's your coaching tree: Trusted (55) or Home (78) with you and the program wherever he lands. Angry, the people who know him well hear him run you down (−4 each, 4 to 14 days later) (`carousel::departed`).
- **His commits waver:** each kid committed to you from his areas has a memo and a few days. Each holds unless a roll under 35% × his loyalty (from his love of the game and coachability) comes up, halved if you called him (`call_commit`, a one-line result, not yet a played scene), cut by 30% if your program is Known or better with his family. One who reopens follows the man 45% of the time when his new school is near your level and could reach him (`carousel::settle_waver`).
- **A new man:** his page reads his ground the day he arrives; a high school coach hired for his area brings his old school's people at Trusted (`carousel::arrived`).

### The assistant seat (GR69)

- **Your area and group are yours:** your head coach's Monday plan gives you your road days; you set only your own row, on your own areas, inside his days (`desk::check_road`), and plan your route, the schools you go to first (`desk::route`).
- **Up to 2 kids a week** (`desk::push`): from your area or your group, with your case. When the film is in, he decides: no room at the position (offers out at the cap), a kid out of the program's reach, or his read: (his staff's read − the position's line) / its spread, plus 2 × (his trust in your eye − 0.4). At zero or better he signs it and the offer goes out through the built offer chain; otherwise wait ("Wait for his junior tape", "Bring me more on him") (`desk::decide`). A kid he signs at 3 stars or less is your find.
- **Push back or let it ride:** pushing back is his office in one line (not yet a played scene); his answer comes now, by his trust and the read (`push_back`, `let_ride`).
- **His trust in your eye** starts at 0.35 (0.5 if he knows you), moves 0.04 with each step the world takes on a kid you pushed (a level-up offer, his stars rising) and 0.06 when the call settles three years on. The screen shows words, never the number (`desk::steps`, `trust_words`).
- **The coach you told:** a high school coach whose kid you pushed and who heard "wait" remembers when the world catches up, and half of it reaches his ties (`desk::told_coach`, `word::spread_to`).
- **Goals at the February meeting:** his two (commits from your areas, from your last three classes' pace, 1 to 4; and one for your group) and a ground goal you pick (`desk::set_goals`, `pick_goal`).
- **The August verdict:** each goal met (+1) or short (−1), his word on your spring, and his staff review: a score of 2 or more makes letting you go 0.6 times as likely, 1 makes it 0.8, below zero 1.3, −2 or worse 1.6 (`desk::august`, `review_factor`, read by `staff::staff_review`).
- **When you're courted:** if he trusts your eye (0.45 or more) or your verdict was good, he counters: assistant head coach, or first in line for the next coordinator job, kept if you stay (`desk::courted`).

### The game's commands

`ground_staff` (the staff rail: each man, his areas, mood, style), `ground_staff_man`, `ground_staff_areas`, `ground_staff_ride`, `ground_staff_ride_end`, `ground_carousel` (who's courted, the counters with their reach and side effect, what leaves with him, wavering commits), `ground_counter`, `ground_let_go`, `ground_waver_call`, `ground_signs`, `ground_desk` (the assistant's desk), `ground_push`, `ground_push_back`, `ground_let_ride`, `ground_desk_goal`, `ground_route` (`crates/afm-game/src/lane_groundstaff.rs`). The screens are lane S7 (above).

### What G10 does not do

- AI staffs work areas by the same rules since lane G9 ("Lane G9" below); their men have no moods or carousel counters.
- The moments (a commit you call, the office when you push back, the staff room) are one-line results: lane G5's played scenes don't cover them yet. The hunt and the guided first offseason are lane G6's.

### Where the knobs are

- `staff/beats.rs`: `PER_MAN` 3, `HOME_STATE` 12, `RIDE_WEEKS` 8; the claim's −8 a held area; 4 areas a man for your changes.
- `staff/men.rs`: `LONG_SPRING` 30 days; the mood sizes in `mood`; the style cut points in `style`.
- `staff/carousel.rs`: `ANSWER_DAYS` 3, `BOND` 0.35, `FOLLOW` 0.45, `RUN_DOWN` 4; the counter's worth in `worth`, its reach in `reach`.
- `staff/desk.rs`: `PER_WEEK` 2, `STEP` 0.04, `SETTLED` 0.06; trust's start 0.35 (0.5); the review factors (0.6, 0.8, 1.3, 1.6).
- `staff/mod.rs`: the mood's pull in `offer_pull` (0.06).

## Lane G9: the race and the AI on Ground (GR65, GR64 for the AI)

Every size here is the spec's (the seat's sizes and gates) or the build's own pick, proposed, not confirmed. Code: `world/ground/ai.rs` (the AI's ground, saved as `Ground.ai`) and `world/ground/race.rs` (your side, `Ground.race`); game commands `crates/afm-game/src/lane_race.rs`. This lane changes how the whole world recruits.

### The AI works its ground (`ai.rs`)

- **Who:** every program that plays this year except your college seat's (`ai_program`, `seat_college`: your staff works by your plan, ground on or off).
- **Areas** (`plan`, at the February meeting and on a world's first day): each area is worth the talent its schools sent out in the last four classes at the staff's level (`TALENT`: FBS weighs a 5-star 20 and a 2-star 1; Division III a 2-star 1.5 and a 4-star 0.3) × its footprint in the state, only where the footprint is 0.15 or more (`REACH_FLOOR`) and the area is worth a tenth of its best (`AREA_FLOOR`). A program at prestige 0.8 or more (`NATIONAL_PRESTIGE`) always works the five richest areas at its level first (`NATIONAL_AREAS`), wherever its name draws: the G9 world check (seed 171) found big programs too local and the title race too wide with the footprint squared and no national tier (proposed, not confirmed). Each man (the on-field staff, the head coach last) takes up to three, the area he has the most ties in first; an area worth three times the last one picked gets a second man (`SECOND_MAN`). Each area's schools are kept in talent order for the season (`Ai.order`).
- **The road** (`road_day`, each evening of a road window: `week::road::window`, `day_kind`): a man works three of five school days in the spring and the winter contact periods, the fall's Thursday and Friday; FBS and FCS staffs inside the window's day limit (`dept::eval_limits`: 140 spring and 33 fall days from 2023), the FBS head coach off the spring road from 2008 (`road::may_go`); staffs below Division I out on a share of their days (`ROAD_SHARE`: Division II 0.5, NAIA, Division III and junior colleges 0.35). A day goes to one of his areas, to the schools with the most talent his staff hasn't seen in 28 days (`REVISIT_DAYS`), as many as the area allows (`areas::schools_a_day`).
- **Calls** (`calls`, Mondays): five a staff (the seat's `CALLS`), to a coach one contact from Known first, then Known coaches in its areas it hasn't talked to in 90 days (`CALL_GAP`), by the area's man.
- **A contact** (`contact`): the seat's sizes. 6 points × the man's people skill × the coach's warmth × the program's pull; the head coach's name (0.8 + 1.3 × prestige²) on his own; a call a third; a second contact the same week half; one a day. The man builds his own tie in full, the program a third. The gates are the seat's (`cap`: Known after two contacts, Trusted a year after Known, Home three years after meeting with a kid who did well, or five).
- **Fade** (`month`, the first): from 60 days without contact, a point a month and half a point more for each other staff with him in the last 30 days. A tie under a point with no contact in two years is dropped.
- **His kids** (`kids`): a starter last season (February 1, +6), a graduate and a draft pick (May 20, +4 and +12), to the program and its head coach, with Home's gate opened.
- **Day one under the work:** a tie nobody worked reads day one (`standing::derived`: pipelines, footprint, the men's own `Coach.ties`), and a worked one never reads below it (`standing::base`: the build's pick, since the world itself moves day one as pipelines and men change). A man hired onto your staff brings his worked ties (your memory starts from them).
- **The store:** one compact tie per coach and holder, saved as tuples (`Ties`).

### What the AI's board reads

See [recruiting.md](recruiting.md), "How a staff builds its board": his coach tells it by its standing (`told`), the services' lists by the states its men work (`work_states`), the admissions read, the track lists, and the tape a kid sends (`SENT_TAPE`). Early offers lean on how well its staff knows his coach (`knows`) where they leaned on the footprint, and a rogue booster's targets are the kids whose coaches its staff knows (0.3 or more, `ncaa::before_matching`).

### Everything that read the AI's ground now reads its work

`standing::points` (news in order, lane G4; the map's owners, `leader`; your men's and rivals' pages) reads the AI's worked ties. Your fade counts the rival staffs' own visits and calls with his head coach (`standing::rivals`).

### Your side of the race (`race.rs`)

- **The sign-in sheet and the polo** (`sheet`, `at_school`): on your man's visit, the day's log adds who signed in the last six weeks (`SHEET_DAYS`) and a rival there the same day ("Danner was there too, in his Auburn polo.").
- **What his coach tells you** (`told`, `ground_told`): a Stranger nothing; a Known coach who came through this year; a Trusted one how often ("Auburn's been in twice."); a Home one who is about to offer ("Get in first.") and who has offered.
- **Rival recruiters as people** (`rivals`, `rival`): the rival men whose areas are yours (your men's, else your state's): program, role, style (`staff::men::style`), where he stands in each shared area from his own ties, his Home schools, years on that staff and titles, staffs you shared, claims he made and your commits he worked, and his visits on sheets at schools where you're Known.
- **A commit who wavers** (`watch`, Mondays): signs from the last two weeks, a rival's man at his school with a staff his coach knows (1), a rival's offer (1), a visit he took (2). Two make a memo "... is wavering" with the signs, once a month a kid. Your one call (`fight`, `ground_fight`, a one-line result until a played scene takes it): your head coach calls his family and his coach (contacts), the family answers by your standing, and his hold on you grows by `FIGHT_HOLD` 0.06 × how it landed (Home 1, Trusted 0.75, Known 0.45, Stranger 0.2) for 60 days (`fight_hold`, read beside `recruiting::stickiness`, which tops at 0.18).
- **Claims about the other side** (`claim`, `run_down`, `settle`): his head coach's job ("gone in a year", settles December 15: held if that coach is gone), the depth chart ("they'll bury him": two underclass starters at his spot, one at QB, RB, TE, K and P) and the scheme (his fit to their system under −0.3), the last two settling in 60 days. Yours (`ground_claim`): a false one costs 12 with the family and 6 with his coach, with a moment ("You lied to us about Auburn."). A rival salesman in a kid's top three, with a staff his coach knows and a visit there this year, runs you down once a season (the claim that's true, else your job): −3 with the family while it stands; you hear it when his coach knows you; a false one settles +5 for you with the family and −12 for that staff with his coach.
- **Rumors** (`rumors`, after the boosters pick on signing day): a rival booster's target on your board, with a Trusted family or coach, is "somebody's taking care of the family"; Home names the school. Only a case proves it (`ncaa::enforcement`). Boosters pay before NIL only (the built model).

### The game's commands

`ground_rivals`, `ground_rival` {man}, `ground_sheet` {school}, `ground_told` {id}, `ground_race` (wavers and claims), `ground_fight` {id}, `ground_claim` {id, about, topic} (`crates/afm-game/src/lane_race.rs`). The screen is lane S6.

### What G9 does not do

- The AI doesn't plan areas by its class plan's needs, hire a local high school coach for his ground, or play scenes; its men's style shows only in run-downs.
- Your call to a wavering commit and the claims are one-line results, not played scenes. The services' predictions are the built insiders (`scouting/services.rs`). Money outside the rules in the NIL era goes through the collectives (built).

### The world check (2026-11-17)

Measured on stand-in games against main bfbec55b; the reports and the table are in the project folder at `afm/ground-g9/` (`check.md`).
- Seed 171 at full scale: in-state share of FBS classes median 36%, p10 18%, p90 74% (main 33/14/65, real 37/15/65); five-star spread, star draft rates and the FCS share of picks as on main; a season about 7% slower (seed 171, 2008-2025: 356 s against 333 s), memory after the history 0.1-0.2 GB more.
- Titles: one seed's count swings by chance (seed 171 at full scale: main 8 champions in 15 years, G9 13-14). Averaged over seeds 171, 231, 241 and 251 at tenth size, G9 has 10.25 champions in 15 years and main 11.0, with 2.75 unbeaten each. The target is about 9 to 10 (Cain's call, NIL era, 2026-11-17: a bit more spread than real history's 8, from healthy competition, not from top programs collapsing).
- Class strength by prestige tier (`world_trial`, "class strength by prestige tier"; four seeds, tenth size): the top 10 produce 0.65 draft picks per program a year (main 0.71) and the 11-25 tier 0.43 (main 0.50); 26-65 0.22 (main 0.20). The top 10 sign 45% of their class out of state (main 55%). A mild squeeze at the top, with the top 10 still the strongest classes; neither main's early offers nor main's board alone undoes it.
- The first build weighed footprint squared with no national tier; seed 171 at full scale had in-state 38% (19-74), so the plan now weighs footprint once and gives programs at prestige 0.8 or more the five richest areas (`NATIONAL_PRESTIGE`, `NATIONAL_AREAS`, proposed, not confirmed).

### Where the knobs are (G9)

- `ai.rs`: `LIST_DAYS` 400, `REVISIT_DAYS` 28, `CALL_GAP` 90, `ROAD_SHARE`, `AREA_FLOOR` 0.1, `REACH_FLOOR` 0.15, `NATIONAL_PRESTIGE` 0.8, `NATIONAL_AREAS` 5, `SECOND_MAN` 3, `SENT_TAPE` [0.3, 0.6, 1.2, 1.6], `TALENT`, `PRUNE` 1.
- `race.rs`: `SHEET_DAYS` 42, `WAVER_DAYS` 14, `WAVER_GAP` 30, `FIGHT_HOLD` 0.06, `FIGHT_DAYS` 60, `RUN_DOWN_TOP` 3.

## Lane S3: the school visit screen

Board 3 (GR72, GR79) on lane G5's scene. Code: `app/ui/screens/groundvisit.js` and `.css` (every class `gv-`, the glass and rows of the school page's `gd-`); `people.js` hands a school visit scene to `gvScene` instead of the conversation page; `groundweek.js` opens a scene of yours from its slot.

- **What it reads.** `people_scene` (the lines, the picks, the sign-in sheet, and its `ground` part), `ground_school` (its two colors, where its kids went, the head coach's first year), `ground_person` (the coach's face, his level, his standing with every staff, his record with you) and `ground_lead` (the new lead's blanks and the order the news went). It writes `people_answer` (a stop, a pick in his office), `people_hand` (the top bar's skip, with its result) and a typed line on live talks when a key is usable (`people_say`, only while the talk is in his office).
- **The left column.** The stops in the order you play them, then the ones still open with the hour each would start (the sign-in sheet at the visit's first minute, then 60, 30 and 30 minutes, practice's 60 only on a day it is offered). A stop with no hours left reads "No time left for it". Under it, where the school's kids went (its first colleges, yours marked), from the school page's data.
- **The middle.** The head coach with his drawn head (initials when he has none), his level with you and the highest other staff he is close to, then the talk: what you noticed in italics, your lines, his with the tell your read caught, how a typed line was heard under yours. Under the talk your picks as chips: the four exchanges in his office, the stops between them and heading out. When it ends, the one line you walk away with and the next scene waiting.
- **The right column.** The newest new name of the visit (a tab for each when there are several) with what the public thinks and what the coach said, five blanks that fill as stops go (the live look is marked while the weight room or practice is still open), who heard first with the level each program holds at this school, and your ground with the coach: his level, what this visit moved, his words from the newest moment and what the next level would get you.
- **Never a hidden number.** Your eye is a word, the coach's nature a word from his record, the public's view stars and offers as everywhere else.
- **Links.** The new name opens his lead card (lane S4's `groundleads`); the other 14 scenes keep the conversation page.
- **Test.** `tests/ground_scenes.rs` (engine) plays a visit stop by stop and checks what the page reads: the hours, where the talk is, the eye's words. The page itself was checked on mock data in both states (between stops, finished).

## Related pages

[recruiting.md](recruiting.md), [scouting.md](scouting.md), [people.md](people.md), [new-teams.md](new-teams.md), [staff-and-coaches.md](staff-and-coaches.md).
