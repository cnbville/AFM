# Ground: how it works today

Checked against main at e677a8d on 2026-11-16 (lane G11a, the high school chain reaction). Ground is being built in lanes (`docs/design/ground.md`); each lane adds its section here.

## Quick answers

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

## Where the knobs are

- `world/high_school.rs`: `CLIMB_SWING` (0.12), `CLIMB_CAP` (2.5), `FORM_UP` (0.3), `FORM_DOWN` (0.15), `CLIMB_MIN_SCHOOLS` (8), `alumni_value` (the old players' worth; all proposed, not confirmed).
- `world/recruiting.rs`: `SIT_WEIGHT` (0.6), `move_in_chance` (0.1 + 0.5 × money, at most 0.6; proposed, not confirmed).

## Related pages

- [recruiting.md](recruiting.md): high schools and being seen, visibility, transfers.
- [people.md](people.md): high school coaches.
