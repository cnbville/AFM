# Gaps

Gaps found while fixing bugs or building, so they carry over between threads (Cain, 2026-11-17). Each: what's missing, why it matters, impact. Remove an entry once it's built. Nothing here is agreed until Cain says yes.

## Engine

- **The engine's instructions read a freelance trust, not his trust in the coach** (medium). `GamePlayer.trust` is 0.5 for every player the world sends to a game, so `tendencies::obey_chance` never sees how much he believes his own coach; only the new bond part (`GamePlayer.bond`, `obey_trust`) reaches it. The built `Person::trust` could feed it directly. Found 2026-11-17 (thread "Build coach and player bonds").

## Logic

- **A coach of yours can't see last season if it was played before you took the seat** (high). His unit's numbers and form (`claims::rooms::unit`, `points_of`) read only `w.seat_games`, the games kept since you arrived. In a new career every holdover's unit has no numbers and no good or poor read, so the voice gets nothing to analyse, the holdover meeting can't narrate his unit's year, and the keep or let go talk can't let you blame his unit (`takeover.rs`, `staffing.rs`). The world already keeps every team's season in the squads, games and players tallies. Found 2026-11-17 (thread "Give coaches real depth when asked why"); fix proposed there.
- **A coach hired after last season reads as "one season" here** (medium). `rooms::unit` sets `years` to `(w.year - first_season + 1).max(1)`, so a man who hasn't coached a game here gets 1, and his card says he's been the coordinator "one season". Robert Leonard, hired after a 2-10 year, took the blame for it. Same thread.
- **A position coach's year is only a growth read** (medium). His room's own numbers on the field (the depth counts of his players) never reach his talk. Same thread.

- **Brothers are not people to each other** (medium). `Family::older_brother_at` names a program, not a person, so a younger brother can't commit to the coach his brother was close to, and the bond's "his little brother commits to you" (`docs/design/coach-bonds.md`, section 7) isn't built. Same thread.
- **No bonds with strength coaches or NFL position coaches** (medium). Strength coaches see a player more than anyone in the off-season, and NFL clubs store no position coaches. They wait for support departments and stored NFL staffs. Same thread.
- **Your own effort in a bond is talks only** (low). `bonds::you_effort` is 1.0: what you do counts only as one-on-ones (a week of contact each) and your choices at the moments, not time you give him in your week (the design's funeral "a day of your week"). Same thread.
- **The returns are not built** (low). A former player's summer visits, his hall of fame asking you to present him, naming a son after you, and a hometown discount in NFL free agency beyond the built "guys follow you" (design section 7). Same thread.
- **No "I'm not going anywhere" promise** (low). A close player hurt by a coach leaving is the price of the bond, but you can't promise to stay. The promises lane has no kind for it. Same thread.

## UI

- **The holdover meeting shows no numbers for his unit** (medium). Without last season's points, yards and third downs on the narration you can't judge whether to keep him. Same cause as the first Logic gap.
