# Lane 4: Money and leagues

Read first: `CLAUDE.md`, `../plan.md` (your files: `crates/afm-game/src/lane_money.rs`, `app/ui/screens/money.js/.css`, plus engine modules), `../list.md` (X1, X7, P1, C8, A4, conference moves 1a/1b), `docs/design/later-extras.md` topics 1 and 5 (main spec), `docs/design/team-management.md`, `docs/world-model.md` (section 5), `docs/research/rules-and-money-2026.md`, `docs/design/screens.md`. Work in progress: `claude/final-wip-lane4` (little).

Two PRs are fine: finances first, other leagues second.
- X1: full club finances at every level as later-extras.md topic 1 says: NFL, college athletic department and HS books on real lines, budgets set from the books, an attendance market, the money side of season-ticket renewals (calendar A17), the blackout rule 1973-2014, facility projects, stadiums and relocation, club values, realignment by media money, guarantee games, and what each seat sees (a finances screen per seat: dense, charted, linked).
- P1: NFLPA team report cards (owner, facilities, food, family room, travel, staff; public 2023-24, internal from 2025), read by free agents and agents; bad grades push owners to spend.
- C8: betting lines (spread, over/under) for every game, upsets judged by the line, coaches' ATS records; information only, on schedules and game pages.
- A4: UCF's stadium capacity (real about 45,000); check other stadium data for similar errors.
- Conference moves 1a: post each move on 1 July, the day it takes effect ("officially leaves the Mountain West for the Pac-12 today"); schedule membership still switches at the start of the football year; old world files re-date the 9 Feb lines on load (`world/program.rs` `apply_history`, `World::after_load` in `world/seat.rs`). 1b: the inbox shows FBS moves to every college coach, FCS and lower only to a coach at that level, HS moves only at HS (`views.rs` `inbox`).
- X7: the other leagues on the engine with their real rule books (CFL, Arena, NFL Europe, XFL, AAF, USFL, UFL) as full leagues in their real years, a Canadian national pool for the CFL ratio, their coaching jobs in the carousel and the job list.

Books compact (yearly lines, not daily); in-season memory must not grow noticeably; say what you added. Check every seat (HS, college, NFL) in the browser. Update `docs/world-model.md` and `docs/game.md`.

Found by other lanes and the build thread (2026-10-22), yours to fix:
- Free agents ask too much: quarterbacks at $64-80M a year, a 40-year-old quarterback at $46M (front office, PR #98). Real top QB deals are about $55-60M a year (2025-26), and a 40-year-old gets a short, cheap deal.
- The trade chart rates veterans above picks (front office): check against how real clubs value picks and players.
- Every portal player gets the same usual NIL figure (front office): NIL should follow his market (position, stats, fame, the school).
