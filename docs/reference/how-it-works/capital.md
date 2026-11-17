# Capital: how it works today

Checked against main 14d72d6c plus lanes C1 (the donors) and C2 (the rooms and the campus) on 2026-11-17.

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
- Does the store change the world? No. Nothing reads it yet (lanes C3 and C4 will), and its draws are keyed on the seed and the donor (`world/capital/mod.rs`, `draw`); `tests/capital.rs`, `the_world_plays_the_same_with_the_donors`.

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
- Where is the campus map? `Campus.halls` (each building with its rooms and opening) and `Campus.open_plots` (CP21, stylized): the real buildings that stood, else a stadium, the football offices and an indoor facility if it has one; each new building adds a hall and takes a plot.

### The rooms' tools

Each tool new with this lane reads its room against the division's typical room this July (`Campus.typical`, `typical_read`), so the level's totals stay where they were.

| Room | What it does in the game | Where |
|---|---|---|
| Locker room | Official visit pull × (1 + 0.1 × read over typical), ±25%; the room's mood + 0.12 × value over typical | `visit_lift`, `world/recruiting.rs` (`OFFICIAL_PULL`); `mood_lift`, `world/people.rs`, `settle_room` |
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

## Related pages

- [people.md](people.md): the boosters in the talks and their promises.
- [recruiting.md](recruiting.md): facilities in a recruit's choice today.
- [nfl.md](nfl.md): NFL contracts and extensions.
- [development.md](development.md): college coaching and the facilities' measured size.
- [injuries.md](injuries.md): the game's injury mix and the strength staff's guard.
