# Gaps

Gaps found while fixing bugs or building, so they carry over between threads (Cain, 2026-11-17). Each: what's missing, why it matters, impact. Remove an entry once it's built. Nothing here is agreed until Cain says yes.

## Engine

(none yet)

## Logic

- **Writing a blank value makes a person's kept part** (medium). Since memory diet lane M1, a person's college part and calendar rest exist only once written (`world/stored.rs`). A new loop that resets a field for everyone (`p.cal.x = None` over all people) quietly makes the part for a million high schoolers and gives back about 0.4 GB, with no failing test unless the tiny world's share check trips. `memory_season` prints the kept counts; a quick-CI check on a small world's high schoolers after a season would catch it. Lane M1 thread.
- **World files still write every blank field of every person** (low). About 120 field names per person, 986,000 of them high schoolers with nothing in most: skipping blank parts would make files and loads smaller, but it changes the shape the World Reader reads. Lane M1 thread.
- **Resident memory sits about 0.4 GB above the live heap in season** (medium). After the allocator hands back free pages, the seed 181 world holds 5.19 GB of live data but 5.6 GB resident at the first game (main: 6.44 and 6.43). Freed space is scattered across pages still in use. Worth measuring on the Mac, whose allocator differs, before any fix. Lane M1 thread.
- **A person's school, state and college indexes are 8-byte numbers** (low). About 2,100 places read them as `usize`; a typed index would let them be 2-4 bytes (about 0.03 GB). Lane M1 thread.

- **A coach of yours can't see last season if it was played before you took the seat** (high). His unit's numbers and form (`claims::rooms::unit`, `points_of`) read only `w.seat_games`, the games kept since you arrived. In a new career every holdover's unit has no numbers and no good or poor read, so the voice gets nothing to analyse, the holdover meeting can't narrate his unit's year, and the keep or let go talk can't let you blame his unit (`takeover.rs`, `staffing.rs`). The world already keeps every team's season in the squads, games and players tallies. Found 2026-11-17 (thread "Give coaches real depth when asked why"); fix proposed there.
- **A coach hired after last season reads as "one season" here** (medium). `rooms::unit` sets `years` to `(w.year - first_season + 1).max(1)`, so a man who hasn't coached a game here gets 1, and his card says he's been the coordinator "one season". Robert Leonard, hired after a 2-10 year, took the blame for it. Same thread.
- **A position coach's year is only a growth read** (medium). His room's own numbers on the field (the depth counts of his players) never reach his talk. Same thread.

## UI

- **The holdover meeting shows no numbers for his unit** (medium). Without last season's points, yards and third downs on the narration you can't judge whether to keep him. Same cause as the first Logic gap.
