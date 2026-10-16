# Section 4: the build checklist

Status: built 2026-10-16 by the thread "Section 4 calendar build". Cain: "fully complete section 4, without any manual approval from me" and "don't pause any builds to ask for permission, instead choose the most realistic option then tell me in the morning what you pondered about".

Every agreed point (T1 to T11 in `calendar.md`) and every addition (A1 to A46 in `calendar-additions.md`) is listed with where it lives in code. Marks:
- **Built**: works in the world now.
- **Dated slot**: the date, the window and the rule are on the calendar; the mechanic inside belongs to a later section, as the additions file says.
- **Proposed, not confirmed**: Claude's own pick, made without asking under the overnight rule.

Code paths are under `crates/afm-engine/src/`.

## The 11 agreed topics

| # | Point | Status | Where |
|---|---|---|---|
| T1.1 | One day at a time; Continue skips quiet days and stops when something needs you | Built | `world/clock.rs` `advance_day`, `continue_until_needed` |
| T1.2 | One real clock for every level: high school Friday, college Saturday, NFL Sunday, Thursday and Monday nights | Built | `world/hs_year.rs`, `world/college.rs`, `world/nfl_year.rs` |
| T1.3 | Dates from real anchors each year | Built | `calendar/anchors.rs` `YearDates` |
| T1.4 | A save starts the day after the Super Bowl | Built | `YearDates::start` |
| T1.5 | The biggest days run hour by hour | Built as the time model (every event has a clock time); the hour-by-hour screens are section 10 | `calendar/events.rs` `Moment` |
| T2 | The NFL year, February to February, every dated step | Built | `world/nfl_year.rs` `day` |
| T2.1 | CBA offseason limits bind training | Dated slot: phases 1-3, OTAs, minicamp, camp and the in-season padded-practice count are dated; what training does is section 6b | `world/nfl_year.rs`, `calendar/rules.rs` |
| T2.2 | Year one plays the real 2026 schedule; later years by formula | Built: all 272 real games with real days, kickoffs and the nine games abroad; later years by the opponent formula with prime time, holidays, games abroad and flex | `data/real-world/nfl_schedule_2026.csv`, `world/nfl_year.rs` |
| T3 | The college year and the portal | Built: real weeks, CFP and bowls, the January window (Jan 2-16) with 5 extra days for CFP teams, a 15-day window after a head coach change, draft deadlines (soft Jan 6, hard Jan 15, withdraw 3 days later) | `world/college.rs`, `world/recruiting.rs`, `world/clock.rs` |
| T3 lean | The all-season lean: events move it, the redshirt trick, portal regret, whispers, coaches sway | Built | `college::lean_checkpoint`, `recruiting::portal_*` |
| T3.1 | One combined January choice: draft, portal or stay | Built; CFP teams' players decide after their last game | `recruiting::draft_reads`, `declarations`, `portal_open` |
| T3.2 | Bowl opt-outs from draft grade, personality and stakes | Built | `world/college.rs` |
| T4 | Each state's own season, playoffs and transfer season | Built (games still on the stand-in until section 3 moves them) | `world/hs_year.rs`, `calendar/data.rs` |
| T4.1 | Spring ball breakouts for (RS) freshmen and sophomores, with a notice first | Built: about 3% of eligible players break out; about half of notices fizzle | `hs_year::notices`, `jumps` |
| T4.2 | National independents like IMG Academy, kept in check | Built | `world/high_school.rs`, `recruiting.rs` |
| T5 | Recruiting periods, visits and signing | Built: contact, evaluation, quiet and dead periods by date; official visits on Fridays, unofficial game-day visits, drop-ins in evaluation weeks | `calendar/mod.rs` `recruiting_period`, `recruiting::recruiting_day` |
| T5.1 | Commits, decommits and flips move like the portal lean | Built | `recruiting::signing_stage` |
| T5.2 | When a kid commits comes from his personality and family | Built: four commitment waves (after spring evaluation, June official visits, summer camps, senior-season visits), then each recruit announces on his own day | `recruiting::commit_timing`, `announce_commitments` |
| T5.3 | Committed and local kids allow more | Built: drop-ins favor committed kids, local unofficial visits, commits pull friends | `recruiting::recruiting_day`, `signing_stage` |
| T5.4 | Recruiting as real actions a human coach could use later | Built: offers, visits and drop-ins are stored per recruit | `PersonCal::offers`, `PersonCal::visits` |
| T6 | The in-season week, day by day, at every level | Built as dated rhythm events (NFL, college, high school weeks) | `clock.rs` `rhythm` |
| T6.1 | Each day's decisions as hooks | Dated slot: the days are marked; the decisions are section 5, 6b and the play-calling build | `clock.rs` `rhythm` |
| T6.2 | The install limit | Dated slot: needs the plays and tactics build | |
| T7 | Where training sits in the year | Dated slot: every block is dated; what training does is section 6b | `world/nfl_year.rs` |
| T7.1, T7.2 | Each block favors a kind of growth; load cuts both ways | Dated slot for section 6b | |
| T8 | Small, medium, big, huge; Continue never skips huge | Built | `calendar/events.rs` `Loudness`, `clock.rs` |
| T8.1 | A regular-season game is huge only when it really matters | Built for both the NFL and college, per team (see the stakes numbers below) | `nfl_year::stakes`, `college::stakes_for_day` |
| T8.2 | No news held back after a huge moment | Built: nothing is held | |
| T9 | Waits come from the people involved; deals can fall apart | Built for the portal (each entrant decides on his own day; a first choice can fall through while he waits) and coach hiring (no interviews of coaches still in the playoffs). Contract and trade waits are section 5 | `recruiting::portal_day`, `staff::weigh_offer` |
| T10 | Continue and holiday mode | Built | `clock.rs` `continue_until_needed`, `holiday_to` |
| T10.1, T10.2 | Staff decide like real people and grow with responsibility | The staff ratings and carousel exist from section 2; the hand-off screens are section 10 | `world/staff.rs` |
| T11 | Era rule books for the 30 hidden years | Built: 16/17 NFL games, BCS, 4-team and 12-team playoffs, transfer eras, NIL, early signing from 2017, four-game redshirts from 2018 and the rest | `calendar/rules.rs` |
| T11.1 | On-field rules never change during a career | Built | |
| T11.2 | Only the 12 agreed off-field changes, each through its real process | Built: owners' votes need 24 of 32, the CBA ends after 2030, the 18-game season moves the Super Bowl a week | `calendar/rules.rs` `Changes` |

## The 46 additions

| # | Addition | Status |
|---|---|---|
| A1 | Roster bonus due dates | Dated slot (contracts are section 5) |
| A2 | The June 1 release date | Dated slot |
| A3 | Workout bonus attendance on OTA days | Dated slot |
| A4 | Camp report day as the holdout line | Dated slot |
| A5 | Hold-in days | Dated slot |
| A6 | The RFA offer sheet deadline | Dated slot |
| A7 | Per-game roster bonus game days | Dated slot |
| A8 | Incentive cut-off at week 18 | Dated slot |
| A9 | College all-star games (Senior Bowl, Shrine Bowl, HBCU Legacy Bowl, NFLPA Collegiate Bowl) | Built: invite lists from the world |
| A10 | Combine invite list day | Built (about 320 invites) |
| A11 | Combine week as the agents' meeting place | Built as a whisper window |
| A12 | Pro day dates per school | Built |
| A13 | Top-30 visit window | Built |
| A14 | The rookie program in June | Built |
| A15 | NFL awards and Hall of Fame dates | Built, placeholder picks (proposed, not confirmed) |
| A16 | Ring ceremony | Built |
| A17 | Season ticket renewal deadline per team | Built |
| A18 | Poll and ranking releases (AP, Coaches, CFP Tuesdays, BCS era) | Built, placeholder rankings (proposed, not confirmed) |
| A19 | CFP selection day | Built |
| A20 | Bowl selection day under the tie-ins | Built |
| A21 | Bowl practices | Built |
| A22 | Finals week | Built |
| A23 | Term-end academic eligibility checks | Built |
| A24 | December graduation and graduate transfers | Built |
| A25 | College awards dates | Built, placeholder picks |
| A26 | Conference media days | Built |
| A27 | Marked college games (senior day, homecoming) | Built |
| A28 | Rivalry week with trophies and all-time records | Built |
| A29 | Grade checks per state | Built |
| A30 | SAT and ACT dates | Built |
| A31 | Marked high school games (homecoming, senior night) | Built |
| A32 | All-state teams per state | Built |
| A33 | State media polls | Built |
| A34 | Daily weather per town from real climate | Built per state (one main football town each), not yet per town |
| A35 | Heat index on practice days | Built |
| A36 | State heat rules for high school practice | Built |
| A37 | Lightning delays | Built |
| A38 | Weather postponements and relocations | Built |
| A39 | High school forfeits and no contests | Built |
| A40 | Storyline tags (revenge, coach's return, hometown, rivalry, milestone, school records) | Built |
| A41 | Poll stakes | Built |
| A42 | "On this day" anniversaries | Built: title games, Heismans and first picks from 5, 10, 15... years ago |
| A43 | Personal events (births, funerals, weddings, throwing sessions, youth camps, charity events) | Dated slot (section 6) |
| A44 | Owner meetings (preseason goals, bye-week check-in, end-of-season review) | Dated slot (section 6) |
| A45 | Betting market releases (win totals, weekly lines) | Built: win totals after the schedule release, lines after each week's last game |
| A46 | Whisper windows (combine week, draft smokescreens, trade rumors) | Built as windows (deception is section 7) |

## The build plan's open questions (taken at Claude's proposals)

1. Build order: built now, after the dynasty merge. Proposed, not confirmed.
2. Chances of the 12 off-field changes: Claude's yearly odds in `calendar/rules.rs`. Proposed, not confirmed.
3. Latest draft withdrawal date: 72 hours after the January 15 deadline, with a later date (February 7) for players on title-game teams, from the 2025 rules. Still worth a source check.
4. Speed budget: at most 25% slower. Measured in the 30-year runs.
5. Climate data: built from published normals for one main football town per state (NOAA's files could not be downloaded). Still to check against NOAA, and per town later.
6. Placeholder rankings and awards: simple picks from results and stats, marked. Built.

## Claude's picks made in this build (proposed, not confirmed)

- Commitment waves on Apr 30, Jun 30, Aug 31 and Oct 31; each recruit then announces on his own day within the next two months, with 8% of June's deciders picking July 4.
- Visit chances: officials on Fridays in contact and quiet periods (12% a day per offered prospect), game-day unofficial visits within 250 miles (25%), drop-ins on evaluation Fridays (20%, 40% for a committed kid).
- Spring breakouts: the top 3% of eligible players by the mix land the step; notices go to as many and about half fizzle.
- The head-coach-change portal window opens five days after the carousel: 30 days in the windows era (2022-2024), 15 days from 2025.
- Late NFL stakes: in the last four weeks, a team within a win of the playoff line, or two division contenders meeting. News is huge only when both sides play for their season; one-sided stakes are big news and still huge for that team. In the quick checks about 55% of teams get 1 to 4 huge games and the rest none.
- College per-team stakes: the rivalry-week (or Army-Navy) game is huge for both sides; another rivalry needs one more stake; otherwise two of a ranked meeting, a playoff place, a conference race between leaders, a sixth win for a bowl in the last game, a coach on the hot seat late in the year. In the quick checks 53-63% of FBS teams get 1 to 3 and the rest none.
- Below the FBS, game news is big at most nationally; the game stays huge for its two teams.
- NFL formula years: international games by era (nine a year from 2026), Christmas games from 2016 (on a Thursday Christmas they are the week's Thursday games), Saturday games after the college season, flex for Sunday and Monday nights late in the year.
- Bowl season starts the Saturday after the conference title games.
- School records for the milestone tag: passing, rushing, receiving yards and sacks, from the program's own history.
- Coaches Poll: a noisy copy of the placeholder ranking.
