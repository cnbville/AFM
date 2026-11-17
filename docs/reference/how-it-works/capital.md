# Capital: how it works today

Checked against main a5d9e29 plus lane C6 (high school and NFL facilities) on 2026-11-17.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/`, `app/` or `data/`. Capital's spec is `docs/design/capital.md`; each lane adds its section here. Every size on this page is Claude's (proposed, not confirmed) unless the spec gives it.

## Quick answers

- How many donors does a program have? The Power 4 22-30, the rest of the FBS 15-22, FCS 8-14, Division II, NAIA and Division III 3-5, by prestige; junior colleges none. The boosters the talks know are on top of that, and ex-pros join besides (`world/capital/donors.rs`, `target`).
- Are the boosters in the talks the same people? Yes. Each booster slot the talks draw (`folk/cast.rs`, `Role::Booster`) is a donor with his name, age, traits, wants and business; a new man in a slot is a new donor and the old one stays on as a plain donor (`donors.rs`, `sync_boosters`, `from_booster`).
- Where is a donor's wealth? Hidden: `Donor::capacity` is what he can truly give over a drive, `Donor::paper` what his assets show. About 12% are stretched thin: paper 2-4 times capacity (`donors.rs`, `draw_donor`, `STRETCHED`).
- What does the player see of it? Only the development office's read, a range of words such as "Could name a room, maybe fund a whole building." It reads paper wealth, so a stretched man reads richer than he is (`world/capital/screen.rs`, `read`, `words`). No screen shows it yet (lane CS2).
- Old money or new? A new donor is new money 30% of the time in 1990, rising to 50% by 2026 (`donors.rs`, `new_money_share`); ex-pros are always new money.
- Does a broken promise spread? Yes, through his circle: his own warmth toward you -0.25, each old-money friend in the circle -0.12, each new-money friend -0.02, times 0.6 in a new-money circle (`donors.rs`, `word`). A promise to a college booster in the talks reaches his donor when it is judged (`folk/talks.rs`, `check_promises`, calls `booster_word`).
- Can a pledge be tied to me? Yes: a pledge the closing coach made while the donor's warmth toward him was 0.7 or more is tied to him (`world/capital/pledges.rs`, `make`, `TIE_WARMTH`). If he stops working at that school the rest is pulled that day (`pledges.rs`, `day`).
- Does the donor follow me? Maybe: 15% at the tie's warmth up to 60% at full warmth, when your new job is at another college; your new school becomes his second school with half his pull (`pledges.rs`, `day`, `FOLLOW_LO`, `FOLLOW_HI`, `FOLLOW_PULL`).
- When does an ex-pro join? At his first deal past the rookie one (a veteran deal, extension, franchise or transition tag) worth at least five times a first-year minimum a year; one look per pro, with a 30% chance plus 5% a season at the school (to 50%), less 15% for a transfer (`world/capital/expros.rs`, `year`, `ex_pro`).
- Who does an ex-pro remember? Every head coach and position coach he had at that school, warmth 0.63 and 0.08 more for each further season, to 0.9 (`expros.rs`, `ex_pro`).
- Can two schools court one donor? Yes: 7% of new donors have ties to a second four-year school in their state ("made in Austin"), which pulls 20-60% of his giving; `donors::of` lists him for both (`donors.rs`, `second_school`, `TWO_SCHOOLS`, `Donor::schools`).
- Do donors die? Yes, on a Gompertz curve near the SSA life table (0.3% a year at 50, 2% at 70, 5% at 80), times 0.8 for the wealthy; 3% of the rest drift off the list each year; the program fills back to its target (`donors.rs`, `year`).
- Does the store change the world? The donors alone don't: their draws are keyed on the seed and the donor (`world/capital/mod.rs`, `draw`; `tests/capital.rs`, `the_world_plays_the_same_with_the_donors`). From the handover season lane C3's drives read them and decide every program's buildings, so the world's future changes; its past stays the same per seed.

## The donors (lane C1)

- **The store** (`world/capital/mod.rs`, `Capital` on `World.capital`, saved; an older file builds it on its first day): donors, circles, pledges, estate gifts, the pros already looked at, and the year it was last brought up to date. In the tiny test world it holds about 7,400 donors and 2.7 MB.
- **When it runs** (`capital::day`, called from `world/clock.rs` after the books each morning): the first day builds every program's list; on the money year's day (1 July) donors age, die, drift and join, the booster slots sync and ex-pros join; every day tied pledges are checked.
- **A donor** (`donors.rs`, `Donor`): name, born, his school (`home`), a second school (`also`, `also_pull`), old or new money, his business in words, alumnus or not (75% of old money, 62% of new), capacity and paper wealth (hidden), affinity with the school (0.6 for an alumnus, 0.4 otherwise, sd 0.15), one or two of the nine wants (`folk/cast.rs`, `Want`; a building weighs 1.2, the others as a booster's), one or two rooms he'd give to in the spec's order (`ROOMS`), his circle, five traits 1-20, the booster slot he holds, the ex-pro's player id, the year he joined, when he left, and his bonds.
- **Two numbers for lane C3** (hidden): `nil_lean`, -1 gives only to NIL to 1 only to buildings (new money around -0.2, old around 0.3, sd 0.45; 1 before 2021), and `collects`, how much he likes to collect on a gift later (around 0.25, more for a meddler).
- **Wealth** (`draw_donor`, `median_capacity`): lognormal around a median by level (Power 4 $4M, other FBS $1.2M, FCS $0.35M, below $0.12M) times 0.6 + 0.8 x prestige and 3% a year from 2026 (`dollars`), sd 1.1 in logs (1.3 for new money), $25K to $3B. A booster's capacity is at least 30 years of his yearly gift (40 for the big giver).
- **Age**: old money joins around 58, new money around 48 (sd 9).
- **Circles** (`fill_circles`, `Circle`): one old-money circle per seven donors (at least one) and a new-money one past twelve (at most two), named ("the country club's Saturday foursome", "the tech founders' dinner"). Old money joins one 85% of the time, new money 40% (a quarter of those an old circle); ex-pros none.
- **His memory of a coach** (`Bond`): warmth 0-1 (0.5 a stranger), meetings and the last day (lane C4's slots call `met`), promises kept and broken, broken ones heard through the circle, and whether the coach coached him.
- **Estate gifts** (`Estate`): when a donor dies, a gift with chance 0.2 + 0.5 x affinity, sized 0.15 + 0.35 x affinity of his capacity, restricted to his first room; kept for lane C3 to pay. The dead stay three years.

## The wealth read (lane C1, CP3)

- **The office** (`screen.rs`, `office`): Power 4 0.75, other FBS 0.55, FCS 0.4, below 0.3, plus 0.2 x (prestige - 0.5).
- **The read** (`read`): the error in logs is 0.9 - 0.5 x the office - 0.08 a meeting (up to four), at least 0.1. The office's estimate for a donor is drawn once per school, so the read does not jump around; meetings narrow the range around it. It reads paper wealth in 2026 dollars.
- **The bands** (`BANDS`): gives every year, a major gift ($250K), name a room ($2M), fund a wing ($10M), fund a whole building ($40M), carry a whole campaign ($150M). In the test world, stretched donors read about 0.6 of a band above what they can really give.

## Pledges tied to a coach (lane C1, CP5)

- **A pledge** (`pledges.rs`, `Pledge`): donor, school, total and paid (dollars), the day, 1-5 years, the coach it is tied to, the day it was pulled, the room. Lane C3 makes and pays them through `make`.
- **Pulled** (`day`): an open tied pledge whose coach no longer holds a job at that school is pulled the same day; what was left is gone (`Pledge::left` 0). Lane C3 feels it in the building.
- **Following** (`day`): only to another college (not a junior college, not the NFL); the donor's affinity with his old school falls to 70%.

## Ex-pros (lane C1, CP6)

- **Who** (`expros.rs`, `year`): pros on an NFL deal of kind veteran, extension, franchise or transition worth five first-year minimums a year (`BAR`), not looked at before (`Capital.pros`). The rookie deal never counts.
- **His school**: the last college he played for (his alumni entry, else his college); none for a junior college.
- **The donor**: new money, an alumnus, "his contract with the ..." as the business, capacity 6% of the deal's new money (`SHARE`), affinity 0.5 + 0.06 a season (to 0.85), rooms weighted to the weight room and the locker room, a little more NIL-minded.

## Where the knobs are

- `world/capital/donors.rs`: `STRANGER` 0.5, `BROKEN_SELF` -0.25, `KEPT_SELF` 0.08, `HEARD_OLD` -0.12, `HEARD_NEW` -0.02, `LOOSE_CIRCLE` 0.6, `IN_CIRCLE_OLD` 0.85, `IN_CIRCLE_NEW` 0.4, `TWO_SCHOOLS` 0.07, `STRETCHED` 0.12, `DRIFT` 0.03, `GOMPERTZ_A` 2.78e-5, `GOMPERTZ_B` 0.094, `WEALTHY_MORTALITY` 0.8, `ESTATE_BASE` 0.2, `ESTATE_AFFINITY` 0.5, `KEEP_DEAD` 3; the counts in `target`, the medians in `median_capacity`, the share in `new_money_share`, the rooms' weights in `pick_rooms`.
- `world/capital/screen.rs`: `ERROR_BASE` 0.9, `ERROR_OFFICE` 0.5, `ERROR_MET` 0.08; `BANDS`; the offices in `office`.
- `world/capital/pledges.rs`: `TIE_WARMTH` 0.7, `FOLLOW_LO` 0.15, `FOLLOW_HI` 0.6, `FOLLOW_PULL` 0.5, `FOLLOW_AFFINITY` 0.7.
- `world/capital/expros.rs`: `BAR` 5, `JOIN_BASE` 0.3, `JOIN_SEASON` 0.05, `JOIN_MOVED` 0.15, `SHARE` 0.06, `COACHED_WARMTH` 0.55, `COACHED_SEASON` 0.08.

## Lane C2: the rooms and the campus

### Quick answers

- How many rooms does a program have? Thirteen, in the spec's CP17 order: locker room, weight room, sports science lab, recovery center, nutrition center, film theater and position rooms, indoor field, recruiting lounge, players' lounge, academic center, content studio, trophy hall and NFL wall, stadium (`world/capital/rooms.rs`, `Room`). `Room as u8` is the index lane C1's donors store.
- What is a room's level? 0 (none) to 5, kept as a decimal (`Campus.level`). The level shown is the read against the conference, rounded, 1-5 (`Campus::shown`).
- Where do the starting levels come from? Each FBS school's real buildings (`data/real-world/college_facilities.csv`): a building's kind and the data's note say which rooms it held, its money per room says the level it opened at, and it loses a tenth of a level a year since (`rooms_of`, `built_level`, `shape`). Then the levels are scaled so the rooms give the school's facilities number exactly (`fit`). Schools outside the data get the everyday rooms and a keyed share of the others.
- Does the facilities number still exist? Yes, as the trade's read: the rooms' weighted mean, level / 5 (`Campus::number`, CP19). In the world's past the number is the facilities lane's (real buildings, else the seat-based model) and the rooms follow it; from the season the world hands over the number is read from the rooms.
- Do rooms age? From the handover season, every room loses 0.1 of a level each July (CP22), never below the level's everyday rooms (5 × level scale × `buildings::BASE`) (`age`, `AGE_A_YEAR`). Before that, the old 0.006 a year on the number stands.
- Do rooms add to growth on top of facilities? No (CP19). The body's part of college coaching reads 0.55 weight room + 0.2 nutrition + 0.15 sports science + 0.1 recovery; the rest reads 0.5 film + 0.3 indoor field + 0.2 recovery. With every room at the number each part is the number, so the old +0.61 by 22 for the best complex stands (`physical`, `teaching`; `world/season.rs`, `develop`).
- Who sees what? A recruit's room tools read the room against the best in his conference (`Campus.read`, CP23): the best reads 5 and each level behind one less, never under 1. The body's tools read the room itself (`Campus::value`).
- Can I renovate? Yes, one room a year through your athletic director (`ask_renovation`; no screen calls it yet, lane CS1's Campus screen will): a level up, done over the summer, open next season. A new building lifts its rooms to 4 or 5 by its money (`open`).
- Do AI programs renovate? Yes, each July once the rooms live, on the same rules: the room furthest behind the department's target, by how far behind and the money (`renovations`, `most_behind`, `renovation_chance`).
- What is temporary space? In a new building's last year before it opens, the team works without the rooms it already has that the building moves: those rooms read a level lower for their tools and the room's mood pays 0.04 (`read_all`, `Campus.temporary`, `TEMPORARY_MOOD`, CP26). A summer renovation, the stadium and the indoor field don't count.
- Does campus life matter to a recruit now? Yes: each kid weighs it by his own keyed draw, most a little, some a lot, the less professional kid more (`campus_life_want`), times the campus life grade around 0.77 (`world/recruiting.rs`, the recruit's utility and its parts). It also adds a little to off-field trouble (`trouble_factor`).
- Does the number hold once the rooms live? In a measured world (seed 171, scale 0.05, games on the stand-in) the FBS mean went 0.633 at the handover, then 0.638, 0.625, 0.621 and 0.625, the spread 0.16 throughout: aging is met by the AI's renovations and new buildings (`facilities_check rooms`).
- Where is the campus map? `Campus.halls` (each building with its rooms and opening) and `Campus.open_plots` (CP21, stylized): the real buildings that stood, else a stadium, the football offices and an indoor facility if it has one; each new building adds a hall and takes a plot.

### The rooms' tools

Each tool new with this lane reads its room against the division's typical room this July (`Campus.typical`, `typical_read`), so the level's totals stay where they were.

| Room | What it does in the game | Where |
|---|---|---|
| Locker room | Official visit pull + 0.1 × read over typical of it (±25%), beside the AD's hosting raise; the room's mood + 0.12 × value over typical | `visit_lift`, `world/recruiting.rs` (`OFFICIAL_PULL`); `mood_lift`, `world/people.rs`, `settle_room` |
| Weight room | 0.55 of the body's coaching part | `physical` |
| Sports science lab | Soft-tissue share of game injuries × (1 − 0.3 × value over typical), 0.85-1.15, on top of the strength staff's guard; your staff sees load and risk at level 2.5 or more | `soft_tissue_guard`, `world/season.rs` (team build); `sees_load` |
| Recovery center | 0.1 of the body's part, 0.2 of the rest; game injury days back: 2 × value over typical in the medical staff's units (12% each), ±1 | `recovery_days`, `world/college.rs` → `training::seat::game_injuries` |
| Nutrition center | 0.2 of the body's part; your body plans' weekly rate × (1 + 0.4 × value over typical), 0.85-1.2 | `body_plan_rate`, `training/seat.rs` |
| Film theater and position rooms | 0.5 of the rest; film study 7 × value (in place of facilities); the install + 0.06 × value over typical, ±0.03 | `film`, `install_lift`, `world/season.rs`, `world/tape.rs` |
| Indoor field | 0.3 of the rest; your college practices indoors when the room exists | `has_indoor`, `training/seat.rs`, `indoor_field` |
| Recruiting lounge | Game-day (unofficial) visits count × (1 + 0.15 × read over typical), ±35% | `game_day_lift`, `world/recruiting.rs` |
| Players' lounge | The room's mood + 0.08 × value over typical; trouble × (1 − 0.2 × value over typical) | `mood_lift`, `trouble_factor` |
| Academic center | Academic support 0.3 + 0.7 × value (grades, eligibility, a recruit's support read) in place of facilities | `academic`; `world/recruiting.rs`, `academic_support`; `world/college.rs`; `world/season.rs`; `folk/claims/program.rs` |
| Content studio | A college player's brand money × (1 + 0.25 × value over typical), 0.85-1.25 | `nil_lift`, `world/fame.rs`, `brand_money` |
| Trophy hall and NFL wall | Pro pipeline and tradition reads behind the recruit's grades × (1 + 0.03 × read over typical), 0.9-1.1; the wall lists every first-round pick once the hall exists | `pitch_lift`, `world/staff.rs`, `update_programs`; `nfl_wall` |
| Stadium | Stadium atmosphere read × (1 + 0.03 × read over typical), 0.9-1.1 | `atmosphere_lift` |

What still reads the number, not a room: prestige (0.12 × facilities), the recruit's facilities grade (ranked within the division, the groundbreaking pull), the support staffs' budget, scouting money, traditions, the takeover's building words and the turf.

What isn't built yet: more official visitors a weekend (the game has no weekend host limit; lane C4's visit weekends), the tour of three rooms (CP20, lane C4), the gate and crowd noise from the stadium room (they follow seats and fans as before).

### The year

- 1 July, `finance::college::money_year`: `rooms::age` first (the handover season sets the rooms live; after that every room ages), then projects open (`rooms::open` once live, else the old step), the ADs decide new buildings (once live their step is what the rooms add, `rise`), then `rooms::settle`: renovations, the rooms kept with the number, and every room read (`read_all`).
- February's book (`staff::update_programs`): the number's old drift runs only while the rooms don't live; the rooms follow it (`sync`).
- A number moved by hand once the rooms live (a tool, a test, the boss's yes) moves every room in step (`sync_program`).
- An old save fits its rooms from its number on load (`after_load`).

### Where the knobs are

`world/capital/rooms.rs`: `Room::weight` (each room's share), `AGE_A_YEAR` 0.1, `built_level` ($4M a room reads 3, each doubling 0.83 more, 1.5-5), `optional_share`, `plots`, `renovation_cost` (the tier's building × the room's share / 0.85 × 0.4), `renovation_chance` (0.02 + 0.265 × levels behind, at most 0.7, 0.4 times with the department in the red, none under half a level), `TEMPORARY_MOOD` 0.04, each tool's slope and caps above, `campus_life_want` (0.02 + 0.14 × u³ − 0.02 × professionalism z, 0.01-0.22).

## Lane C3: the money and the drive

### Quick answers

- What starts the fever? Big moments (CP7): a national title 1.0, the Heisman 0.75, a conference title and a top-10 pick 0.5 each (the pick is half a national title), the playoff 0.25, a bowl win 0.12, a rivalry win 0.08 (`world/capital/fever.rs`, `What::strength`). They are read off the college season once it is over (titles, the bracket, bowls, rivalry games, the Heisman) and off the draft on 15 May (its first ten picks, by the college each came from) (`fever::day`).
- How does it fade and stack? Each moment counts (1 - months / 24)² of its strength, so it fades fast at first and is gone at two years; the fever is 1 - e^-sum, so a run stacks toward the top without passing it (`fade`, `fever_on`). A conference title alone reads 0.39 the day it is won and 0.056 by the spring after the next season; a title in 2027 and a title and the playoff in 2028 read 0.49 in March 2029 (`tests/capital_drive.rs`, `the_fever_starts_on_big_moments_stacks_and_fades`).
- What does the fever do? A donor's yes chance x (1 + 0.5 x fever) and his gift x (1 + 0.4 x fever) (`gifts.rs`, `FEVER_CHANCE`, `FEVER_SIZE`); a drive starts more often (`drive.rs`, `START_FEVER`); and the collective's yearly growth gains 0.05 x fever once the drives live (`program.rs`, `COLLECTIVE_FEVER`, CP9). Your program's every moment is a news line ("Donors answer the title"), and the big ones at the Power 4.
- What is the drive? Every program's capital campaign for one building, in its real stages: a five-month feasibility study, the quiet phase, the public phase and the close, the building going up on its own clock (`drive.rs`, `Drive`, `Stage`). On screen it is "the drive" (CP12). No screen shows it yet (lane CS3).
- Who starts one? Each July once the rooms live, every program's AD on the same rule (CP27): 4% + 0.7 x (the AD's target over the facilities number) + 0.2 x fever + up to 10% for money in the bank (less in the red) + 5% for a big booster, at most 45%; not while a drive or a project is under way, nor for three years after a shelved one (`drive::year`). It replaces the ADs' old July decisions (`finance/college.rs`, `facility_decisions`), which still build the world's past. Asking your AD for a facility (`ask_facility`, the boss's facility ask) opens a study once the rooms live.
- What building? The tier's: the Power 4 a football building ($140M in 2024 dollars), the rest of the FBS an operations center ($40M), FCS an indoor practice facility ($14M), below a weight room and locker room ($3M), each x 0.85-1.15 (`kind_for`). A size holds the full building's rooms, biggest first, to its share of their weight: the full complex, mid-size (0.75) or smaller (0.5) (`Size`, `rooms_for`).
- What is the goal? The share of the cost the drive raises from donors: Power 4 0.75, the rest of the FBS 0.4, FCS 0.12, below 0.06; the department's bonds and the university carry the rest, more the smaller the program (`FUND_SHARE`, `goal_for`; fitted on the world check below so each level builds about as before).
- What does the study say? What each donor would give (`gifts::expected`), plus what earlier pledges left and the small gifts, for each size; it advises the biggest size whose goal it covers nine tenths of, and names the lead donors (the three biggest gifts, each at least 8% of the goal) (`study`). An AI program shelves a drive the study can't advise; on a fever of 0.5 or more it goes a size bolder 30% of the time.
- How does a donor say yes? Chance 0.45 + 0.6 x (affinity - 0.5) + 0.8 x (warmth toward the coach in the room - 0.5) + 0.4 x (last two seasons' win share - 0.5), within 0.03-0.9, then x the fever's lift, x 0.6-1.0 for a building with or without a room he wants, x (0.4 + 0.6 x his building side); his gift a share of his capacity: 0.6 x (0.5 + affinity) x the fever's lift x (0.3 + 0.7 x building side) x 1 or 0.6 for his room x (0.8 + 0.4 x the office) x (0.7 + 0.6 x warmth), times his school's pull (CP28) (`gifts::odds`). His building side is (1 + `nil_lean`) / 2.
- Who asks whom? Each month the development office asks each donor of the stage's list with chance 0.25: the quiet phase the top half by capacity, the public phase everyone; a donor who gave is not asked again (`asks`, `OFFICE_PACE`). A lead gift needs the head coach in the room: an AI head coach meets each lead donor with chance 0.2 a month; your lead donors wait for you (lane C4's slots, or `gifts::ask`) when you work the drive yourself, and give the office only a courtesy gift (a quarter) when you leave it to the office (CP14, `COURTESY`). Your drive starts with the office running it.
- When does it go public? At half the goal (CP16: real campaigns wait for about that), or after two quiet years with 35% in; short of that it drops a size and tries a year more, and at the smallest is shelved (`stage_checks`). You can go public earlier (`go_public`); an AI program does on a fever of 0.5 with a quarter in, half the time. A launch before half is early: if the next season is a losing one, the drive stalls in public (gifts at half the chance for a year, a news line, your AD's trust -0.04) (`stall_check`).
- When does ground break? At four fifths of the goal, whatever the stage (`BREAK`, `break_ground`). The building goes in the books with what is pledged and opens the July after it is done (two years for a building, 15 months for an indoor field, 11 for a weight room and locker room).
- When does it close? At the goal or after two public years; the small gifts come half at the launch and half at the close (8% of the goal x (1 + fever) x (0.7 + 0.6 x win share)). Short of the line for its size, the biggest size the money builds breaks ground; if none, it is shelved and its pledges wait for the next drive (`close`, `shelve`).
- What does the goal cost me? At your program (CP15): a goal hit lifts the AD's trust 0.06 (0.1 for a goal bolder than the study's); a miss of 15% or more is in the papers and costs 0.05 (0.1 bold) (`TRUST_*`, `MISS`). You set the goal before it goes public (`set_goal`).
- Can I borrow? Yes (CP11, `push_borrow`): with half the goal in, ask the AD to break ground now, or to build a size bigger; he says yes at 0.35 + 0.8 x (trust - 0.5) + the department's money (±0.2) + 0.2 x fever, once a money year. Bonds past his line cost the books 7.5% a year for 22 years, and half of that comes out of football's staff pool, at most a quarter of it: a $60M push at a $24M pool is about 9% (`staff_cut`, in `finance/college.rs`, `open_year`). An AI drive a year in with half its goal, the fever at 0.3 or money in the bank, pushes 30% of Julys.
- What about the NIL pot? Each building gift from a donor who leans to NIL takes (1 - building side) x half a year's payment out of the collective's base, all of a year's when you pushed him past his lean (CP9, `NIL_DENT`, `NIL_DENT_PUSHED`). In your own ask (`gifts::ask`) a donor whose side is under 0.4 says he'd rather pay your players, and one over 0.6 he'd rather build, unless you push (his chance x (0.5 + his side), his warmth -0.04); a NIL yes adds a third of the gift a year to the collective. One NIL ask a donor a year.
- What are catches? A gift's hidden strings (CP10): chance 0.6 x the donor's `collects`, the kind from his wants (his nephew's walk-on spot, a say in a coordinator hire, the sideline and the team plane, a look for a kid from his old high school, his name on the end zone), called a year to three later (`gifts::give`, `Catch`). The staff's read is "Likes to collect", "Likes to be thanked" or "Gives and lets go" through the office's error (`screen::catch_read`). An AI program's AD grants 60%; yours rings as a news line and waits 30 days for `answer_catch`, then the AD answers. Granted: his warmth +0.05 (a say in a hire costs your AD's trust 0.02); refused: -0.2, and he pulls the rest with chance `collects`.
- What about overruns? 35% of buildings, 5-30% over, come up half a year after ground breaks (CP24). You choose (`fix_overrun`): cut rooms (the smallest go until their weight covers it), wait (it opens a year later and the drive reopens a year to raise it) or borrow (bonds with the staff pool cost). The AD waits 30 days, then waits; an AI program cuts rooms at 15% or more, borrows with money in the bank or a fever of 0.3, else waits (`building`, `apply_fix`).
- What happens to a pulled pledge? Before ground breaks it is money the drive lacks; after, the books carry it as bonds, and one that leaves the building under four fifths of what broke ground sets it back a year (`building`, CP5).
- What about estates? Lane C1's estate gifts go into the program's drive, one restricted to a room once a drive holds it, or after three years (`gifts::estates`). Pledges pay a year each July (`gifts::pay`).
- Does it change the world? Yes, from the handover season: the drives decide every program's buildings, and the fever lifts the collectives. The past is the same per seed (the drives and the collective's lift wait for the rooms to live, `capital::live`).

### Your calls (no screen yet; lane CS3)

`drive::set_goal`, `go_public`, `hands_on`, `push_borrow`, `fix_overrun`; `gifts::ask` (building or NIL, pushed or not; lane C4's donor slots call it) and `answer_catch`. Each is the engine's; no `Game::call` command wraps them yet.

### Where the knobs are

- `world/capital/fever.rs`: `What::strength`, `FADE_MONTHS` 24.
- `world/capital/gifts.rs`: `BASE_YES` 0.45, `YES_AFFINITY` 0.6, `YES_WARMTH` 0.8, `YES_WINS` 0.4, `FEVER_CHANCE` 0.5, `FEVER_SIZE` 0.4, `BASE_SHARE` 0.6, `OFF_ROOM` 0.6, `NIL_DENT` 0.5, `NIL_DENT_PUSHED` 1.0, `PUSH_WARMTH` -0.04, `RATHER` 0.4, `COURTESY` 0.25, `CATCH` 0.6, `AD_GRANTS` 0.6, `CATCH_KEPT` 0.05, `CATCH_REFUSED` -0.2, `CATCH_DAYS` 30; pledge years in `years_for`.
- `world/capital/drive.rs`: `STUDY_MONTHS` 5, `LAUNCH` 0.5, `BREAK` 0.8, `PUSH_BREAK` 0.5, `QUIET_MAX` and `PUBLIC_MAX` 24, `ADVISE` 0.9, `LEAD_SHARE` 0.08, `LEADS` 3, `OFFICE_PACE` 0.25, `AI_MEET` 0.2, `CROWD` 0.08, `STALL` 0.5, `FUND_SHARE`, `OVERRUN` 0.35, `BOND_RATE` 0.075, `BOND_YEARS` 22, `STAFF_SHARE` 0.5, `STAFF_CUT_MAX` 0.25, `ANSWER_DAYS` 30, the `TRUST_*`, `MISS` 0.15, `START_BASE` 0.04, `START_GAP` 0.7, `START_FEVER` 0.2, `SHELVED_WAIT` 3; the buildings in `kind_for`, build times in `build_days`.
- `world/program.rs`: `COLLECTIVE_FEVER` 0.05.

### The world check

Seed 181, 10 years of history, scale 0.05, every game on the stand-in, six seasons on (`facilities_check rooms 181 10 0.05 6`), against the same world on main before the lane (about 35 minutes each). The past is the same: both read the same at the handover (FBS mean 0.633). After six seasons:

| Level | Facilities, before | Facilities, with drives | New buildings in six Julys, before | With drives |
|---|---|---|---|---|
| Power 4 | 0.743 | 0.732 | 41 | 43 |
| Rest of the FBS | 0.501 | 0.493 | 28 | 27 |
| FCS | 0.172 | 0.165 | 61 | 34 |
| Division II | 0.083 | 0.084 | 37 | 28 |
| Division III | 0.074 | 0.061 | 72 | 34 |
| NAIA | 0.063 | 0.053 | 21 | 9 |

- The FBS holds where the old decisions kept it (0.618 against 0.629). Below the FBS fewer buildings go up: three to five donors rarely carry a drive, so about 30 studies a year are shelved there. Before the lane Division III rose from 0.057 to 0.074 in six years; with drives it holds near 0.06.
- In the sixth year: 57 studies opened, 35 buildings broke ground (20 full, 6 mid-size, 9 smaller), 44 campaigns closed and 34 hit their goal; raised against the goal 1.57 on average (lead gifts land in big pieces); 2 pushed to borrow, 10 overruns, 5 early launches; 136 catches waiting.
- The Power 4 collectives run about 4% thinner (13.2M against 13.8M a year): building gifts from NIL-minded donors (CP9), partly met by the fever's lift.
- The hottest programs each year are the champions (fever 0.7-0.85).

## Lane C6: high school and NFL facilities

### Quick answers

- What rooms does a high school have? Five (CP33's list and the stadium): weight room, film room, locker room, turf practice field, stadium, each 0 (none) to 5 (`world/capital/hs.rs`, `HsRoom`, `HsCampus` on `Capital.hs` by school). They are drawn the first live day around where the school's money keeps them (`target`: 0.8 + 3.4 x resources for the weight and film rooms, the locker room a little more for prestige, the stadium from the usual crowd), a keyed level either way; turf by `turf_chance` (0.02 + 0.4 x resources + 0.3 x enrollment / 2,500 + 0.1 x culture + 0.1 x the town's money - 0.1, within 0.03-0.95: about half of all schools).
- Do they age? A tenth of a level each 1 July (CP22, `hs::year`), never below 0.1.
- What does each one do? Each is read around the high schools' typical room this year (`Capital.hs_typical`, the mean value), so the level's totals stay where they were:

| Room | What it does | Where |
|---|---|---|
| Weight room | The body's coaching + 1.8 x value over typical (in coaching points) | `hs::physical_lift`, `world/season.rs` (the high school step's `physical_coaching`) |
| Film room | Film study + 3.5 x value over typical, in games and on your tape | `hs::film_lift`, `world/hs_year.rs` (`squad_team`), `world/tape.rs` |
| Locker room | The room's mood + 0.12 x value over typical (±0.08) in development | `hs::mood_lift`, `world/season.rs` |
| Turf field | Your practice on a wet grass field waits half an hour ("rain on grass") without one; the field's upkeep line 25% lower with one | `training/seat.rs`, `practice_weather`; `finance/hs.rs`, `budget` |
| Stadium | The home crowd x (1 + 0.3 x value over typical), 0.8-1.25: the gate and the crowds | `hs::crowd_factor`, `finance/hs.rs`, `crowd` |

- Who pays for the rooms? The booster club, from the shorter list (CP33): the racks and platforms lift the weight room a level, the locker room renovation the locker room, the meeting room with film screens the film room, the field house 0.6 each to the weight and locker rooms, the turf practice field builds turf to 5 (`hs::project_rooms`, the school desk's `PROJECTS`). The speed gear and the video tower stay equipment (the school's resources, as before). At your school your wish list decides (`front_office::school`, `booster_day`); each AI school's club takes the room furthest behind its target at the same prices, with chance 0.25 x levels behind (to 0.6) x twice the year's purchases over the price (to 1) (`hs::boosters`, CP27).
- What is a district? The public schools of one town in one state, a big town's split into groups of six by id (`hs::district`, `DISTRICT_MAX`). Private schools have no bond.
- When does a board call a stadium bond? In two windows: 1 February for the first Saturday in May, 1 August for November's general election day (the first Tuesday after the first Monday), at least 78 days before as Texas law asks (`election_days`, `next_election`, `ORDER_DAYS`). Each district with nothing under way weighs one with chance 0.002 + 0.25 x its need + 0.004 x culture, at most 0.2, where the need is how far its stadium trails the typical one and its own target (`call_chance`, `stadium_need`). Not after a passed bond for eight years, nor within a year of a failed one. Hawaii has no local votes.
- What does a bond build? A new stadium (need 0.25 or more and 1,500 students or a football state) at about $3,300 a seat in 2012 dollars (Allen ISD's 18,000 seats for $60M), seats 1.5 x the biggest usual crowd (2,000-18,000); else a renovation at a quarter of that (`package_for`). It opens 1 August two years after a new stadium's vote, one after a renovation's.
- How does the town vote? The expected yes share is 0.535 + 0.1 x (the district's win share - 0.5) + 0.03 for a state title + 0.02 x ln(crowds over the schools' size, ±0.03) + 0.04 x culture + 0.06 x (the town's money - 0.5) - 0.05 x (the tax bill against $25,000 a student - 1) + the coach's work; election night adds 0.06 x a keyed normal (`expected_yes`, `NIGHT_SD`). A November vote reads this season's record so far, a May vote last season's. It passes at the state's line: 50%, California 55%, Idaho two-thirds, Oklahoma, Iowa and Washington 60%, Missouri four-sevenths (`needs`).
- What is my work in town? Within ten weeks of election day, one slot a week (CP2's slot, from your hours): a lunch talk at the Rotary club (2 hours, +1 point, three times), a night at the booster club's phone bank (3 hours, +0.8, four times), the district's community forum (2 hours, +1.2, once) (`town_work`, `Work::size`). An AI head coach does his own on the same rule: up to 7 points by his relationships rating, a keyed draw (`coach_work`).
- Can I ask for a bond? Yes, once a money year (`ask_bond`): the board says yes at 0.08 + 1.2 x the need + 0.5 x (your principal's trust - 0.5) + 0.2 x (last season's win share - 0.5), within 0.03-0.85. The boss meeting's facility ask at a high school puts one on the ballot on his yes (`folk/boss.rs`, `grant`; `hs::call_for`). A bond you asked for that passes lifts his trust 0.04; one that fails costs 0.03.
- Who gets the stadium? Every school in the district (CP34): a new one sets each school's stadium to 5, a renovation lifts it 1.5; the field is turf, so each school's turf is at least 4.5; a new stadium's field house lifts the first school's locker room to 4.5 (`hs::open`). Your crosstown rival's crowds rise with yours.
- What do I see? News lines for your own district only: the board's call, election night with the share, the opening. `hs::seat_json` holds the rooms, the bond, its outlook in words ("It leans yes") and your work for lane CS6's screen.

### The NFL

- What rooms does a club have? Eight (`world/capital/nfl.rs`, `ROOMS_NFL`): locker room, weight room, sports science lab, training room, cafeteria and nutrition, film rooms, indoor field, family room and players' lounge, with shares 0.16, 0.18, 0.10, 0.14, 0.10, 0.14, 0.12, 0.06. The stadium stays the books' (`finance/nfl.rs`, `Home`). No donors: the owner pays (CP35).
- Where do they start? The first live league year (the day after the Super Bowl, `nfl::year` called from `finance::nfl::year_start`), each room at 5 x the club's facility number, a keyed 0.4 of a level either way. Then each ages 0.1 a year (CP22) and the club's facility number is their weighted mean (`ClubRooms::number`).
- What moves the report card (CP35)? Before the rooms live every line reads exactly as before (`nfl::card_lines`). Once they live: the locker room's line reads 0.7 x the locker room + 0.3 x the stadium's newness; the weight room's the weight room; the training room's half the training room and half the lab (with the medical staff as before); food and the nutritionist + 0.3 x the cafeteria over the league's typical; the treatment of families + 0.3 x the family room over typical (`card_line` names each room's line).
- What do the rooms do? Read around the league's typical room (`Capital.nfl_typical`): the body's coaching + 1.8 x (the body's rooms over typical, the college mix 0.55 weight, 0.2 nutrition, 0.15 lab, 0.1 training room), ±0.9 (`physical_lift`, `world/pro/mod.rs`); game injury days back in the medical staff's units + 2 x the training room over typical (`recovery_units`, `world/pro/games.rs`); soft-tissue injuries in games x (1 - 0.3 x the lab over typical), 0.85-1.15 (`soft_tissue_guard`); film study + 7 x the film rooms over typical, ±2 (`film_lift`, games and your tape); the room's mood + 0.12 x locker room + 0.08 x family room over typical, ±0.1 (`mood_lift`, `world/people.rs`); your body plans x (1 + 0.4 x the cafeteria over typical) (`body_plan_rate`, `training/seat.rs`); the indoor field (every club has one).
- Do owners renovate on their own? Yes, every club on one rule (CP27, `nfl::year`): the room furthest behind his target (5 x (0.55 + 0.45 x his spend level)), with the college ADs' chance (`rooms::renovation_chance`, 0.4 times after a year in the red). A renovation costs the room's share of a $160M (2024) building at half, grown with league revenue (`renovation_cost`), and opens 20 July (for camp if begun by June, else next year's) (`ClubBuild`, `nfl::day`).
- What about a poor report card? A shamed owner (as before) now redoes the rooms the card names: one or two of the locker room, training room and weight room lines at D+ or worse and he renovates those rooms; all three and he builds a new training facility as before, which sets every room to 5 (`card_push`, `opened_building`).
- How do I ask (CP35)? By room, once a league year (`ask_room`; the owner meeting offers the three rooms furthest behind, `rooms_to_ask`, `folk/boss.rs`, codes 60 + the room). His yes chance is his own facility chance (`folk::boss::chance`: his money, trust, the goals you met, his nature) x your leverage, + 0.06 a level the room trails his target, within 0.02-0.95. A typed facility ask that he grants redoes the room furthest behind (`grant_room`).
- What is my leverage (CP36)? 0.5 + last season's win share, x 0.7 for a losing season, + 0.15 for the playoffs, + 0.35 if another team offered you a job this past year or you turned one down (`leverage`, `wanted`). In words: "Hard to refuse" at 1.3 or more, "A fair hearing" at 0.9, else "A long shot"; a winless year reads 0.35, a champion with offers 2.0. Leverage moves only the room asks; your other asks of the owner read as before.
- What do I see? A news line when your club's renovation is ready for camp; `nfl::seat_json` holds each room's level, its card line and latest grade, your leverage in words, the build under way and the owner's words for lane CS6.

### Does it change the world?

Yes, from the season the world hands over (`capital::live`): the high school rooms' tools, the boosters' rooms, the bonds and the crowds; the clubs' rooms, renovations, report cards and their tools. Before that nothing reads them (an empty store reads neutral, and the clubs' card lines read the one number), so the past is the same per seed (`tests/capital.rs`, `the_world_plays_the_same_with_the_donors`). Tests: `tests/capital_seats.rs`.

### Where the knobs are

- `world/capital/hs.rs`: `AGE_A_YEAR` 0.1, `WEIGHT_COACHING` 1.8, `FILM_STUDY` 3.5, `LOCKER_MOOD` 0.12, `STADIUM_CROWD` 0.3, `TURF_UPKEEP` 0.25, `DISTRICT_MAX` 6, `WORK_DAYS` 70, `ORDER_DAYS` 78, `NIGHT_SD` 0.06; `target`, `turf_chance`, `call_chance`, `package_for`, `expected_yes`, `needs`, `Work::size`, `coach_work`, `ask_chance`.
- `world/capital/nfl.rs`: `weight`, `owner_target`, `renovation_cost`, `leverage`, `ask_chance`, the tools' slopes and caps above.

## Related pages

- [people.md](people.md): the boosters in the talks and their promises.
- [recruiting.md](recruiting.md): facilities in a recruit's choice today.
- [nfl.md](nfl.md): NFL contracts and extensions.
- [development.md](development.md): college coaching and the facilities' measured size.
- [injuries.md](injuries.md): the game's injury mix and the strength staff's guard.
