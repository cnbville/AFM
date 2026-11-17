# Gaps

Gaps found while fixing bugs or building, so they carry over between threads (Cain, 2026-11-17). Each: what's missing, why it matters, impact. Remove an entry once it's built. Nothing here is agreed until Cain says yes.

## Engine

(none yet)

## Logic

- **A coach of yours can't see last season if it was played before you took the seat** (high). His unit's numbers and form (`claims::rooms::unit`, `points_of`) read only `w.seat_games`, the games kept since you arrived. In a new career every holdover's unit has no numbers and no good or poor read, so the voice gets nothing to analyse, the holdover meeting can't narrate his unit's year, and the keep or let go talk can't let you blame his unit (`takeover.rs`, `staffing.rs`). The world already keeps every team's season in the squads, games and players tallies. Found 2026-11-17 (thread "Give coaches real depth when asked why"); fix proposed there.
- **A coach hired after last season reads as "one season" here** (medium). `rooms::unit` sets `years` to `(w.year - first_season + 1).max(1)`, so a man who hasn't coached a game here gets 1, and his card says he's been the coordinator "one season". Robert Leonard, hired after a 2-10 year, took the blame for it. Same thread.
- **A position coach's year is only a growth read** (medium). His room's own numbers on the field (the depth counts of his players) never reach his talk. Same thread.

## UI

- **The holdover meeting shows no numbers for his unit** (medium). Without last season's points, yards and third downs on the narration you can't judge whether to keep him. Same cause as the first Logic gap.
