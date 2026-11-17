# Capital: how it works today

Checked against main at 14d72d6 on 2026-11-17 (lane C2, the rooms and the campus, before it merged).

The design is `docs/design/capital.md` (CP1-CP40). Every size below is proposed, not confirmed, unless the spec gives it.

## Quick answers

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

## The rooms' tools

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

## The year

- 1 July, `finance::college::money_year`: `rooms::age` first (the handover season sets the rooms live; after that every room ages), then projects open (`rooms::open` once live, else the old step), the ADs decide new buildings (once live their step is what the rooms add, `rise`), then `rooms::settle`: renovations, the rooms kept with the number, and every room read (`read_all`).
- February's book (`staff::update_programs`): the number's old drift runs only while the rooms don't live; the rooms follow it (`sync`).
- A number moved by hand once the rooms live (a tool, a test, the boss's yes) moves every room in step (`sync_program`).
- An old save fits its rooms from its number on load (`after_load`).

## Where the knobs are

`world/capital/rooms.rs`: `Room::weight` (each room's share), `AGE_A_YEAR` 0.1, `built_level` ($4M a room reads 3, each doubling 0.83 more, 1.5-5), `optional_share`, `plots`, `renovation_cost` (the tier's building × the room's share / 0.85 × 0.4), `renovation_chance` (0.02 + 0.265 × levels behind, at most 0.7, 0.4 times with the department in the red, none under half a level), `TEMPORARY_MOOD` 0.04, each tool's slope and caps above, `campus_life_want` (0.02 + 0.14 × u³ − 0.02 × professionalism z, 0.01-0.22).

## Related pages

- [recruiting.md](recruiting.md): the facilities grade and the groundbreaking pull.
- [development.md](development.md): college coaching and the facilities' measured size.
- [injuries.md](injuries.md): the game's injury mix and the strength staff's guard.
