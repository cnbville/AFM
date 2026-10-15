# Section 4: the calendar build plan

Status: written 2026-10-15 at Cain's request: "add the 46 and the 11 agreed topics that are already decided into one big build plan". Nothing is built yet.

What it builds:
- **The 11 agreed topics** in `docs/design/calendar.md` (confirmed 2026-10-14). Written below as T1 to T11.
- **The 46 additions** in `docs/design/calendar-additions.md` (confirmed 2026-10-15). Written below as A1 to A46.

The order of the phases, the code layout and the checks are Claude's picks: **proposed, not confirmed.** The open questions at the end need Cain's answer before the build starts.

---

## Where the code stands

- The world runs one **whole year per step** (`World::advance_year` in `crates/afm-engine/src/world/mod.rs`): new class, high school transfers, rosters, all games, the coaching carousel, college exits and the draft, the portal, signing day, programs, life events.
- There are no dates. Everything in a year happens in that fixed order, at the same moment.
- College games run on the match engine. High school games still use the stand-in. The NFL has front offices and depth charts for the draft, but **no NFL season is played** and there are no contracts (those are section 5).
- The 30 hidden years run through the same yearly step.

The calendar build replaces the yearly step with a **daily clock**. The world then does each thing on its real day, under that day's rules. This changes results per seed, so every world number has to be measured again in the trial after the build.

## How it is built (applies to every phase)

1. **One clock, one code path.** The 30 hidden years, the 2026 start and every later year all run on the same daily clock. No shortcut path for history (Cain's trial rule: as similar to the final game as possible).
2. **Events are data.** Each event has a date (from real anchors per year), a level (NFL, college level, a state), the rule book that allows it, a loudness level (small, medium, big, huge) and whether it waits on a decision.
3. **Rule books per level and per year.** Era rules for the 30 hidden years (T11). From 2026 on, on-field rules stay fixed; off-field rules change only from the agreed list of 12.
4. **Headless first.** No screens yet (section 10). "Needs you" is a flag the trial tools can read. Continue, holiday mode and staff hand-offs run as functions the future screens will call.
5. **Slots for other sections.** Where a mechanic belongs to a later section (contracts, personality, training, scouting), the calendar builds the date, the window and the rule now, and a hook the later section fills. A slot does nothing until then.
6. **Speed and same results per seed.** Each person keeps his own random stream. Daily steps must not make a 30-year world much slower (see open question 4).

---

## Phase 1. The clock core

Delivers: T1 (all), the engine of T8, T9 and T10.

1. A date type and a daily clock. A save starts the day after the Super Bowl (T1.4).
2. Real anchors per year: Labor Day, Thanksgiving, the second Sunday of February, the last Thursday of April and the rest. 2026 uses the real 2026 dates (T1.3, T2.2).
3. An event queue that every system posts into and reads from.
4. Hour-by-hour days for free agency opening, the trade deadline, cutdown day, the draft and the UDFA rush (T1.5). Built now as the time model; they fill up as the NFL systems arrive.
5. Loudness levels (small, medium, big, huge) on every event, and the rule that Continue never skips a huge one (T8).
6. Continue: skip quiet days, stop when something needs you (T1.1, T10).
7. Holiday mode to a chosen date, which always stops for huge moments (T10).
8. Waits that come from the people involved, and deals that can fall apart while you wait (T9). Built as a general "pending answer" event with a due time.

Check: a 30-year world runs on the clock and gives the same result twice for one seed.

## Phase 2. Rule books by level and era

Delivers: T11, the rule side of T2, T3, T4 and T5.

1. The era rule books for the 30 hidden years: 16 NFL games until 2021, then 17; the BCS, then a 4-team playoff, then 12 teams; no portal before 2018; no NIL before 2021; and the rest of T11 (source-checked per era when building).
2. The 2026 rule book for every level: the NFL (CBA offseason limits, T2.1), FBS, FCS, D2, D3, NAIA and JUCO (each with its own playoff: FCS 24 teams, D2 28, D3 32), and every state.
3. The 12 agreed off-field changes, each through its real process (owner votes need 24 of 32, the CBA ends after 2030, NCAA votes and courts). Their chances are an open question.
4. The salary cap growing with league money each year (T11).

Check: a world started in 1996 shows each era's rules in the right years.

## Phase 3. The college year on the clock

Delivers: T3, A18 to A28, and the college parts of T6 and T7.

1. Games on their real weeks: Week 0 in late August, 12 games, rivalry week (A28), conference title games, Army-Navy, the 12-team CFP, bowls and the title game around January 19. The lower levels run their own seasons and playoffs.
2. **Marked games:** senior day and homecoming per school (A27). Rivalry games carry their trophy and all-time record (A28).
3. **Polls and rankings (A18):** AP preseason poll, weekly AP and Coaches Polls, CFP committee rankings on Tuesdays. Who votes how is a later mechanic; this phase builds the release days and a placeholder ranking from results, marked "proposed, not confirmed".
4. **CFP selection day (A19) and bowl selection day (A20)** under the real tie-ins.
5. **Bowl practices (A21):** a dated block for bowl teams only. Watched against the dynasty bar in the trial.
6. **Academic calendar:** finals week (A22), the term-end eligibility check (A23), December graduation and graduate transfers (A24), from each player's existing academics.
7. **Awards dates (A25):** Heisman finalists and ceremony, the awards show, All-America teams. Voting is a later mechanic; this phase posts the dates and a placeholder pick from stats.
8. **Conference media days (A26)** in July.
9. **The portal on its real window,** January 2 to 16, with the 5 extra days for CFP teams and the 15-day window after a head coach change (T3).
10. **The all-season lean:** the events that move a player toward staying or leaving, the redshirt trick, timing by star level, portal regret, NIL cut on entry, whispers, and coaches swaying players (T3). It runs every day from what happens, not once a year.
11. **One January choice per player:** draft, portal or stay, with the soft and hard draft deadlines and the later withdrawal date (T3.1).
12. **Bowl opt-outs** from draft grade, personality and the stakes (T3.2).
13. The college week: 20 hours, one day off, Tuesday to Thursday practice, Friday travel, Saturday games (T6). Coaches recruit at high school games on Fridays.
14. College training blocks: winter conditioning, 15 spring practices, 8 summer hours a week, 29 fall camp practices (T7). Slots now; what the training does is section 6b.
15. Early enrollees arrive in January.

Check: a season's dates match the real 2026 calendar; portal timing and draft declarations look real against the world trial numbers.

## Phase 4. The high school year on the clock

Delivers: T4, A29 to A33.

1. Each state's own season start, length, classes, playoff format and title-game dates (T4). High school games stay on the stand-in until section 3 moves them to the engine.
2. Spring football in the states that allow it.
3. **Spring ball breakouts (T4.1):** true freshmen, redshirt freshmen and sophomores only, in college spring practice and high school spring ball. The right mix (work ethic, coachability, a good position coach, open reps, a filling-out body, room under the ceiling) can give a sudden step by the spring game, with a notice first that doesn't always pan out.
4. Summer: state dead periods, camps, 7-on-7, college camps. Junior days, December early graduation, January all-star games, state all-star games.
5. Each state's transfer season.
6. **National independents like IMG Academy (T4.2):** a handful, tuition about $90k, homesickness from boarding, limited starting spots, exposure only.
7. **Grade checks per state (A29),** with a player able to miss playoff games.
8. **SAT and ACT dates (A30).**
9. **Marked games:** homecoming and senior night (A31).
10. **All-state teams (A32)** and **state media polls (A33).**

Check: state calendars match their real dates for a sample of states; breakouts land only on eligible players and stay rare.

## Phase 5. Recruiting as dated actions

Delivers: T5.

1. The NCAA periods: contact, evaluation, quiet, dead, on their real dates per year.
2. Official visits from April 1 of the junior year; unofficial visits outside dead periods.
3. The early signing period (first Wednesday of December) and February signing.
4. Commits, decommits and flips moved by events, like the portal lean (T5.1).
5. When a kid commits, from his personality and family (T5.2).
6. Local and committed kids: practice drop-ins in evaluation weeks, easy unofficial visits, babysitting commits, unlimited contact after signing, committed stars pulling friends (T5.3).
7. Everything as real actions an AI staff takes now and a human college coach could take later (T5.4). NFL scouts don't visit high school practices.

Check: commit timing spreads across the year like the real data; recruiting pull stays inside the dynasty bar.

## Phase 6. Weather from the date and the place

Delivers: A34 to A39. Feeds the match engine's existing weather inputs.

1. Daily weather per town from real climate norms (A34). Needs a researched climate data set (see open question 5).
2. Heat index on practice days (A35) and state heat rules for high school practice (A36).
3. Lightning delays by each level's rule (A37).
4. Rare postponements and relocations under each level's rules (A38).
5. High school forfeits and no contests by state rule, counted in playoff points (A39).

Check: game-day weather by city and month against real climate; disruptions stay rare.

## Phase 7. The NFL year on the clock

Delivers: T2, A1 to A17. **Most of this needs section 5** (contracts, rosters, free agency, trades) and an NFL season played on the engine. Phase 7 builds every date, window and rule now, and the parts that work without contracts.

Buildable now:
1. The full league calendar from T2: the carousel, reserve/future deals, the cap set, the combine, the tag window, legal tampering, the new league year at 4pm ET, RFA offer sheets, the league meeting, comp picks, pro days, top-30 visits, the draft, the UDFA rush, rookie minicamp, OTAs, the fifth-year option deadline, schedule release, mandatory minicamp, the summer break, the July 15 tag deadline, camp, three preseason games, the 90-to-53 cut, waivers and practice squads, 18 weeks with byes, the trade deadline, flex scheduling, international games, Black Monday, 14 playoff teams, the Pro Bowl Games and the Super Bowl.
2. Year one on the real 2026 schedule; later years by the real opponent formula (T2.2).
3. **The college all-star games (A9):** Senior Bowl, Shrine Bowl, HBCU Legacy Bowl, NFLPA Collegiate Bowl, each with an invite list from the world.
4. **Combine invite list day (A10)**, **pro day dates per school (A12)**, **the top-30 visit window (A13)**, **the rookie program (A14)**.
5. **Awards and Hall of Fame dates (A15):** Pro Bowl voting window, All-Pro, NFL Honors, Hall of Fame finalists, class and enshrinement. Placeholder picks until the voting mechanic exists.
6. **Ring ceremony (A16)** and **season ticket renewal day (A17)** as dated events.
7. The CBA offseason training limits that bind your team (T2.1), with room for the parked offseason-work feature.

Slots for section 5:
8. **Roster bonus due dates (A1)**, **the June 1 date and post-June 1 designations (A2)**, **workout bonus attendance on OTA days (A3)**, **camp report day as the holdout line (A4)**, **hold-in days (A5)**, **the RFA offer sheet deadline (A6)**, **per-game roster bonus game days (A7)**, **the week 18 incentive cut-off (A8)**.
9. **Combine week as the agents' meeting place (A11):** the whisper window opens now; the free agency it hints at needs section 5.

Check: every NFL date for 2026 matches the real calendar; later years shift the way the real anchors do.

## Phase 8. The weekly rhythm and training slots

Delivers: T6 and T7 (NFL and high school parts; college is in phase 3).

1. The NFL week, day by day: Monday film, exams and press conference, Tuesday off with waivers at 4pm ET, Wednesday install and the first injury report, Thursday third downs and red zone, Friday situations and final statuses, Saturday walkthrough, travel and practice squad call-ups by 4pm ET, Sunday inactives 90 minutes before kickoff (T6).
2. Short and long weeks, the bye with 4 days off, travel and time zones, 14 padded practices a season, practice squad poaching, the IR 21-day window (T6).
3. Your scouts' week: college games Saturday, practices midweek, advance scouting (T6).
4. The high school week: film Monday, practice Tuesday to Thursday, game Friday, JV and film Saturday (T6).
5. Each day's decisions as hooks: the game plan Tuesday, practice plan and reps Wednesday to Friday, injury statuses Friday, inactives Sunday, all hand-off-able to staff (T6.1).
6. The install limit: more plays mean more mental errors for low football IQ, and short weeks shrink it (T6.2). Needs the plays and tactics build; slot now.
7. The NFL training blocks: on their own in February and March, Phase 1, Phase 2, OTAs and minicamp, the summer break, camp, in season (T7). Each block favors a different kind of growth, and training load cuts both ways (T7.1, T7.2). Slots now; the training itself is section 6b.

Check: a sample week's days and deadlines match the real NFL week.

## Phase 9. Stakes, pacing and staff

Delivers: T8, T9, T10 in full, A40 to A42.

1. The stakes test for regular-season games: real stakes only (playoff odds swing, a division title, a rivalry, your job on the line), no fixed cap (T8.1).
2. **Storyline tags (A40):** revenge game, coach's return, hometown game, rivalry, milestone. Each tag counts toward the stakes test.
3. **Poll stakes (A41)** for college games.
4. **"On this day" anniversaries (A42)** from the 30 years of history, as news lines only.
5. No news held back after a huge moment, only shown quieter (T8.2).
6. Staff who decide like real people and leave a reason (T10.1), and who grow and get poached when given responsibility (T10.2). The staff ratings exist from section 2; the hand-off rules are built here.

Check: in a trial season, a typical team gets 1 to 3 huge regular-season games and some get none (T8.1).

## Phase 10. Timing slots for other sections

Delivers: A43 to A46. Dates and windows only; each mechanic comes with its section.

1. **Personal events (A43):** single-game absences for a birth or a funeral, offseason weddings, summer throwing sessions, youth camps and charity events (section 6, and 6b for throwing sessions).
2. **Owner meetings (A44):** preseason goals, the bye-week check-in, the end-of-season review the day before Black Monday (section 6).
3. **Betting market releases (A45):** preseason win totals and weekly lines (a later mechanic).
4. **Whisper windows (A46):** trade rumors before the deadline, draft smokescreens in April (section 7).

## Phase 11. The trial

1. Three 30-year worlds (seeds 2026 to 2028) on the clock, the same way as world trial 5.
2. Refresh the debug dashboard and the full player pages.
3. Check every date against the real 2026 calendar.
4. Re-measure everything the clock changes: titles and the dynasty bar, draft numbers, portal and recruiting timing, development.
5. Name the players with big slumps, how much and why.
6. Every result with a "why"; one checker pass.

---

## Depends on

| Needs | For | Status |
|---|---|---|
| Section 5 (team management) | A1 to A8, most of A11, the NFL season itself | Not designed |
| Section 6 (people) | A43, A44, the personality side of T3 and T5 | Not designed |
| Section 6b (training) | What T7's blocks do | Not designed |
| Section 7 (scouting) | A46, scout travel in T6 | Not designed |
| Plays and tactics build | T6.2, the install limit | Spec done, not built |
| Section 3 leftover | High school games on the engine | Open |
| The fix-list topics 2-5 and round-2 scheme items | Same world code (`world/`) | Open in "Fixing the dynasty" (part 2, PR #28, merged 2026-10-15) |

## Open questions for Cain before the build

1. **Where the build goes in the order.** Still open from the handoff. The dynasty fix (PR #28) is already on `main`, but fix-list topics 2-5 and the round-2 scheme items will touch the same world code. Proposed, not confirmed: start with phases 1 and 2 (the clock and rule books) now, and move the college and recruiting phases (3 and 5) after those fix-list items merge.
2. **The chance of each of the 12 off-field changes.** Still open from the handoff.
3. **The latest date to withdraw from the draft and keep college eligibility.** Still open; it gets source-checked when phase 3 is built.
4. **A speed budget.** A 30-year world takes about 33 minutes today. Proposed, not confirmed: the daily clock may add at most 25%.
5. **Climate data for weather.** Phase 6 needs real daily climate norms for every town. Proposed, not confirmed: research a public data set (for example NOAA climate normals) the same way as the other real-world data.
6. **Placeholder rankings and award picks.** Phases 3 and 7 need some ranking and award choice before the voter mechanics are designed. Proposed, not confirmed: a simple pick from results and stats, clearly marked, replaced later.
