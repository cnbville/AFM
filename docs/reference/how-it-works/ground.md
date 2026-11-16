# Ground: how it works today

Checked against main at e677a8d on 2026-11-16 (lane G11a, the high school chain reaction). Ground is being built in lanes (`docs/design/ground.md`); each lane adds its section here.

## Quick answers

- Does a high school's prestige move? Yes, each June 15, on its old players in college and the NFL (`world/high_school.rs`, `climb`, called from `world/clock.rs` on `hs_transfer_day`, before the transfers).
- What counts as an old player? Anyone whose last high school it was (`Person::hs`) and who is in college, declared for the draft or a pro now. Players out of football count nothing (`alumni_value`).
- What is each worth? In college 1.0 at a power program, 0.6 elsewhere in FBS, 0.3 in FCS, 0.1 lower, ×1.5 if he started last season; in the NFL 2.0 on a roster or injured reserve, 3.0 with 9 or more starts last season, 0.8 on a practice squad; 0.3 in another league (`alumni_value`).
- How far does it move? 15% of the gap a year when it climbs, 8% when it falls (`CLIMB_UP`, `CLIMB_DOWN`).
- Can a state end up with more powerhouses? Not for long: each state's schools are ranked by their old players and the school at rank k moves toward the k-th highest starting prestige in its state, so the state keeps its ladder and only who holds each rung moves (`climb`, `Chain::start`).
- What happens when nobody's old players are playing? Ties keep today's order, so nothing moves (`climb`).
- Does it draw dice? No: it reads the world only, so it never moves the world's stream (`climb`).
- What does prestige change? A recruit's school exposure in his visibility (`world/recruiting.rs`, `recruit_visibility`), who is a powerhouse (prestige 0.75 or more, `HighSchool::is_powerhouse`), what draws standouts' transfers and move-ins (`high_school_transfers`), and everything else that already read a school's prestige (staff pay, crowds, money, schedules).
- Where is its history? `HighSchool::chain.years`, its prestige each June after the step; `chain.college`, `chain.power`, `chain.nfl` count this June's old players and `chain.made` is their score (`Chain`). `prestige_in` reads a past June, `chain_why` says it in words.
- Is it in the news? When a school becomes a powerhouse or slips out, on June 15, a small high school news line with the counts (`climb`).
- A stadium your town paid for (`legacy/hs.rs`) stays on top as `chain.lift` (+0.03).
- An old save gets each school's start from its prestige on load (`chain_after_load`, from `World::after_load` in `world/seat.rs`).

## The chain reaction (lane G11a, GR21)

Every June 15, before the transfers, `climb` counts each school's old players on all cores (`alumni_value`), ranks each state's schools by the sum (ties by today's prestige, then by id), and moves each school toward the prestige its rank earns: the state's starting prestiges sorted best first, at that rank, plus the school's lift (at most 1.0). It moves 15% of the gap up or 8% down (`CLIMB_UP`, `CLIMB_DOWN`), clamped to 0.02-1.0. Each school keeps its starting prestige (`Chain::start`, set at the build in `generate_state_schools` and for a new school in `new_teams/school.rs`).

What it reads: where old players are now (stage, college division and power conference that season, last season's role, NFL status and starts). What it does not read: wins or titles, the current roster, star ratings, or anyone's hidden potential.

The loop closes through the kids: prestige is a recruit's school exposure (0.3 of his visibility, `recruit_visibility`), so a climbing school's kids are seen more, sign higher and keep it climbing; a school at 0.75 or more is a powerhouse and draws the state's standouts (`high_school_transfers`).

## Move-ins and transfers by the state's rule (lane G11a)

On June 15 the standouts (the top 3% of their state's age 15-17) weigh a move to a powerhouse as before (`world/recruiting.rs`, `high_school_transfers`; the pulls are in `recruiting.md`, "High schools and being seen"). The state's rule (`world/high_school.rs`, `transfer_rule`) now decides how many go:
- What he would sit: the rule's sit-out (a full year 1.0, half a season 0.5), or nothing on a state's free first transfer (`TransferRule::free_once`).
- A move-in: where a bona fide family move makes him eligible at once (`TransferRule::move_exempt`, every state but Arizona and Tennessee, whose rules sit transfers even with a residence change), a family moves into the new school's district with a chance of 0.1 + 0.5 × family money, at most 0.6 (`move_in_chance`), drawn on his own key.
- The pull is cut by 60% of what he would still sit: × (1 − 0.6 × sit-out × (1 − move-in chance)) (`SIT_WEIGHT`). A move out of state already moves the family; it sits only where the rule doesn't free a move.
- A family that moved in-state frees him (where the rule allows) and his note says so: "his family moving into its district so he could play at once".

## Where the knobs are

- `world/high_school.rs`: `CLIMB_UP` (0.15), `CLIMB_DOWN` (0.08), `alumni_value` (the old players' worth; proposed, not confirmed).
- `world/recruiting.rs`: `SIT_WEIGHT` (0.6), `move_in_chance` (0.1 + 0.5 × money, at most 0.6; proposed, not confirmed).

## Related pages

- [recruiting.md](recruiting.md): high schools and being seen, visibility, transfers.
- [people.md](people.md): high school coaches.
