# Gaps

Gaps found while fixing bugs or building, so they carry over between threads (Cain, 2026-11-17). Each: what's missing, why it matters, impact. Remove an entry once it's built. Nothing here is agreed until Cain says yes.

## Engine

(none yet)

## Logic

- **A coach of yours can't see last season if it was played before you took the seat** (high). His unit's numbers and form (`claims::rooms::unit`, `points_of`) read only `w.seat_games`, the games kept since you arrived. In a new career every holdover's unit has no numbers and no good or poor read, so the voice gets nothing to analyse, the holdover meeting can't narrate his unit's year, and the keep or let go talk can't let you blame his unit (`takeover.rs`, `staffing.rs`). The world already keeps every team's season in the squads, games and players tallies. Found 2026-11-17 (thread "Give coaches real depth when asked why"); fix proposed there.
- **A coach hired after last season reads as "one season" here** (medium). `rooms::unit` sets `years` to `(w.year - first_season + 1).max(1)`, so a man who hasn't coached a game here gets 1, and his card says he's been the coordinator "one season". Robert Leonard, hired after a 2-10 year, took the blame for it. Same thread.
- **Your going rate is 0.6 of what an AI school pays the same player** (high). `front_office::college::splits` hands out 0.6 of the school's revenue share and of the collective's money (`NIL_ROSTER`, kept for your offers), while an AI school's July 1 pays its roster all of both (`recruiting::nil_payouts`, `NIL_ROSTER_TWO`). So the desk's "roster split" for a starter reads well under what the same man earns at a rival, and "under half the going rate" warnings fire late. Found 2026-11-17 (thread "Pay inherited players their market price"); not changed there.
- **A player's stay read doesn't see the deal you set for next year** (medium). `stay_read` weighs his NIL income (last July's, brand money included) and December's keep-him offer, not your desk deal, so a raise you give in the spring doesn't hold him in the spring window until July 1 pays it. It also counts his own brand money as pay. Same thread.
- **AI schools pay rotation players and backups the split, never their market price** (medium). Only starters get December's keep-him offer (`nil_bid::keep_offers`); everyone else shares what the fixed deals leave. A breakout rotation player a rival would pay more is underpaid on every AI roster. The world check proposed in the same thread measures it.
- **A position coach's year is only a growth read** (medium). His room's own numbers on the field (the depth counts of his players) never reach his talk. Same thread.

## UI

- **The holdover meeting shows no numbers for his unit** (medium). Without last season's points, yards and third downs on the narration you can't judge whether to keep him. Same cause as the first Logic gap.
