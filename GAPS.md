# Gaps

Gaps found while fixing bugs or building, so they carry over between threads (Cain, 2026-11-17). Each: what's missing, why it matters, impact. Remove an entry once it's built. Nothing here is agreed until Cain says yes.

## Engine

- **The engine's instructions read a freelance trust, not his trust in the coach** (medium). `GamePlayer.trust` is 0.5 for every player the world sends to a game, so `tendencies::obey_chance` never sees how much he believes his own coach; only the new bond part (`GamePlayer.bond`, `obey_trust`) reaches it. The built `Person::trust` could feed it directly. Found 2026-11-17 (thread "Build coach and player bonds").
- **The press ranks only finished rooms** (low). `race::press_score` reads `Campus.level` and `since`, so a building that has broken ground adds nothing until it opens, and the Pecking order screen can't say "read as 10th since the groundbreaking" as board 5 draws it. Recruits' groundbreaking pull (facilities lane #281) is separate. Found 2026-11-17 (thread "Build Capital screen CS5"); needs Cain's call on whether a building under way counts for the press.

## Logic

- **"You'll be a captain" is only a pride hit in a talk** (medium). The ear has no captaincy term (`usage`, `terms`), so the promise binds you to nothing, he can't judge it later, and the room never hears it; real captains are usually voted by the team, so a coach promising it is promising something he may not control. `takeover::captains_of` already names captains. Found 2026-11-18 (thread "Make talks know the time of year").
- **A coaching-change transfer window keeps no dates once it opens** (low). `World.coach_window` is cleared the day it opens, so a talk can name the window while it waits but not when it closes once it's open; a late window keeps its dates (`late_windows`). Found 2026-11-18 (thread "Make talks know the time of year").

- **A coach of yours can't see last season if it was played before you took the seat** (high). His unit's numbers and form (`claims::rooms::unit`, `points_of`) read only `w.seat_games`, the games kept since you arrived. In a new career every holdover's unit has no numbers and no good or poor read, so the voice gets nothing to analyse, the holdover meeting can't narrate his unit's year, and the keep or let go talk can't let you blame his unit (`takeover.rs`, `staffing.rs`). The world already keeps every team's season in the squads, games and players tallies. Found 2026-11-17 (thread "Give coaches real depth when asked why"); fix proposed there.
- **A coach hired after last season reads as "one season" here** (medium). `rooms::unit` sets `years` to `(w.year - first_season + 1).max(1)`, so a man who hasn't coached a game here gets 1, and his card says he's been the coordinator "one season". Robert Leonard, hired after a 2-10 year, took the blame for it. Same thread.
- **A position coach's year is only a growth read** (medium). His room's own numbers on the field (the depth counts of his players) never reach his talk. Same thread.

- **Brothers are not people to each other** (medium). `Family::older_brother_at` names a program, not a person, so a younger brother can't commit to the coach his brother was close to, and the bond's "his little brother commits to you" (`docs/design/coach-bonds.md`, section 7) isn't built. Same thread.
- **No bonds with strength coaches or NFL position coaches** (medium). Strength coaches see a player more than anyone in the off-season, and NFL clubs store no position coaches. They wait for support departments and stored NFL staffs. Same thread.
- **Your own effort in a bond is talks only** (low). `bonds::you_effort` is 1.0: what you do counts only as one-on-ones (a week of contact each) and your choices at the moments, not time you give him in your week (the design's funeral "a day of your week"). Same thread.
- **The returns are not built** (low). A former player's summer visits, his hall of fame asking you to present him, naming a son after you, and a hometown discount in NFL free agency beyond the built "guys follow you" (design section 7). Same thread.
- **A coach leaving doesn't shake a high-need kid's home stability** (low). The design's "loses stability (-0.5 x closeness)" isn't built: only his mood, a Left moment, a year guarded and a year of trouble risk. The stability field is his background, so it needs a call on whether a coach's leaving may change it. Same thread.
- **No "I'm not going anywhere" promise** (low). A close player hurt by a coach leaving is the price of the bond, but you can't promise to stay. The promises lane has no kind for it. Same thread.

## UI

- **The holdover meeting shows no numbers for his unit** (medium). Without last season's points, yards and third downs on the narration you can't judge whether to keep him. Same cause as the first Logic gap.
- **The Pecking order shows your conference only** (low). A state rival outside the conference has its rumors in "What your donors hear" but no row, and the table never ranks you against your state. Found in the same thread (CS5); the press order is built per conference (CP31).
- **No pecking order for high school or NFL seats** (medium). The tab row reads "Your seat" for them (CS6), but nothing ranks a district's or an owner's rooms. Same thread.
