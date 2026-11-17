# Gaps

Gaps found while fixing bugs or building, so they carry over between threads (Cain, 2026-11-17). Each: what's missing, why it matters, impact. Remove an entry once it's built. Nothing here is agreed until Cain says yes.

## Engine

- **The press ranks only finished rooms** (low). `race::press_score` reads `Campus.level` and `since`, so a building that has broken ground adds nothing until it opens, and the Pecking order screen can't say "read as 10th since the groundbreaking" as board 5 draws it. Recruits' groundbreaking pull (facilities lane #281) is separate. Found 2026-11-17 (thread "Build Capital screen CS5"); needs Cain's call on whether a building under way counts for the press.

## Logic

- **A coach of yours can't see last season if it was played before you took the seat** (high). His unit's numbers and form (`claims::rooms::unit`, `points_of`) read only `w.seat_games`, the games kept since you arrived. In a new career every holdover's unit has no numbers and no good or poor read, so the voice gets nothing to analyse, the holdover meeting can't narrate his unit's year, and the keep or let go talk can't let you blame his unit (`takeover.rs`, `staffing.rs`). The world already keeps every team's season in the squads, games and players tallies. Found 2026-11-17 (thread "Give coaches real depth when asked why"); fix proposed there.
- **A coach hired after last season reads as "one season" here** (medium). `rooms::unit` sets `years` to `(w.year - first_season + 1).max(1)`, so a man who hasn't coached a game here gets 1, and his card says he's been the coordinator "one season". Robert Leonard, hired after a 2-10 year, took the blame for it. Same thread.
- **A position coach's year is only a growth read** (medium). His room's own numbers on the field (the depth counts of his players) never reach his talk. Same thread.

## UI

- **The holdover meeting shows no numbers for his unit** (medium). Without last season's points, yards and third downs on the narration you can't judge whether to keep him. Same cause as the first Logic gap.
- **The Pecking order shows your conference only** (low). A state rival outside the conference has its rumors in "What your donors hear" but no row, and the table never ranks you against your state. Found in the same thread (CS5); the press order is built per conference (CP31).
- **No pecking order for high school or NFL seats** (medium). The tab row reads "Your seat" for them (CS6), but nothing ranks a district's or an owner's rooms. Same thread.
