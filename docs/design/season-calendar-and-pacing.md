# AFM Design: Season Calendar and Pacing

> SUPERSEDED (2026-10-14): the finished section 4 spec is `docs/design/calendar.md`. Where the two differ, `calendar.md` wins.

> DRAFT for roadmap section 4. The roadmap answers (`docs/roadmap.md`, 2026-10-11) settle this doc's defaults: you are GM and head coach of one NFL team, a save starts mid-February, and names are real (with a fictional toggle at new game, not fictional by default). The week-by-week calendar, tension tiers and climax budget are still unconfirmed.

Status: draft v1 · 2026-10-10
Scope: the full football year across high school, college and the pro league, what the player decides each week, and how the game builds tension without rushing its big moments.

League, team and event names use real NFL/NCAA terms as placeholders. A shipped game would need fictional or licensed equivalents.

---

## 1. Assumptions and defaults

These are the defaults this doc picks where the brief left a choice open. Each one is listed again under Open questions (section 9).

| Topic | Default | Why |
|---|---|---|
| Player role | Head of football operations for one pro team (GM and head coach in one, like FM's manager) | Matches FM, and lets the player touch roster, contracts, game plans and scouting. |
| HS and college | A fully simulated living world the player scouts but does not run | High school players can then grow realistically into draft prospects without asking the player to manage three levels at once. A college or HS career mode can come later on the same calendar. |
| Save start | Week 1 of the AFM year, the Monday after the championship game (mid-February) | The first hours are spent learning the roster, scouting and building toward the draft, which is the slow start the brief asks for. FM's summer start would throw the player straight into camp. |
| Time step | One day per tick, with the game pausing whenever a decision is due | Same feel as FM's Continue button. |
| Rules | Modern pro rules (17 games, 18 weeks, 14-team playoffs, 53-man roster, 16-man practice squad, hard cap) and a 12-team college playoff, stored as editable data per league year | Rules change over time, so they must not be hard-coded. |

---

## 2. The AFM year at a glance

The year is 52 calendar weeks, **W1 to W52**. W1 starts the Monday after the championship game. Dates are approximate and shift by a few days each year with the real calendar.

| Phase | Weeks | Months | Pro league | College | High school | Tension |
|---|---|---|---|---|---|---|
| A. Review and evaluation | W1–W3 | mid Feb – early Mar | Season review, staff, tags, combine | Spring practice opens | Winter lifting, junior days | Low, rising |
| B. Free agency | W4–W7 | Mar | Legal tampering, league year, FA waves | Spring practice | Track season starts | Major spike then settle |
| C. Draft build-up | W8–W10 | Apr | Visits, pro days, board, **Draft** | Spring games | Spring evaluation period | Rising to **climax** |
| D. Development quiet | W11–W21 | May – mid Jul | Rookie camp, OTAs, minicamp, dead period | Summer workouts | Spring ball, camps, 7-on-7, official visits, commitments | Deliberately low |
| E. Training camp | W22–W29 | mid Jul – early Sep | Camp, preseason, **cutdown day** | Fall camp, Week 0 | Practice opens, season starts | Rising to **climax** |
| F. Early season | W30–W38 | Sep – early Nov | Weeks 1–9 | Weeks 1–10 | Regular season | Slow ramp |
| G. Deadline and race | W39–W47 | Nov – early Jan | **Trade deadline**, Weeks 10–18 | Rivalry week, title games, early signing, bowls, portal | Playoffs, state titles, all-star games | Building, earned climaxes |
| H. Postseason | W48–W52 | Jan – early Feb | Wild card to championship | CFP title, all-star games, signing day | National signing day | Sustained **climaxes** |

---

## 3. Week-by-week calendar

Each row lists what happens in the world and what the player is asked to decide. Tier is the highest tension tier that week can reach (section 5): T0 ambient, T1 notable, T2 major, T3 climax.

### Phase A. Review and evaluation (W1–W3)

| Week | Pro league | College | High school | Player decides | Tier |
|---|---|---|---|---|---|
| W1 | Season ends. Coaching carousel finishes. Franchise and transition tag window opens. Reserve/future contracts signed. | Spring practice opens at early schools. Coaching staff changes settle. | Winter strength programs. Junior days at colleges. Rankings refresh after senior season. | End-of-season review of every player (keep, extend, release, shop). Fill staff vacancies. Set offseason priorities with the owner. Sign reserve/future players. | T1 |
| W2 | **Scouting Combine** (measurements, drills, medicals, interviews). Agents meet teams informally. | Spring practice continues. | Track and field begins, which feeds speed data into scouting. | Choose which prospects get your 45 formal combine interviews. Assign medical staff to flagged players. Hold early talks with agents of your own free agents. | T1 |
| W3 | Tag deadline. Teams make cap cuts. Pro days begin. | Pro days at campuses. | Spring evaluation planning. | Apply the franchise tag or not. Cut or restructure contracts to clear cap. Send scouts to pro days. | T2 |

### Phase B. Free agency (W4–W7)

| Week | Pro league | College | High school | Player decides | Tier |
|---|---|---|---|---|---|
| W4 | Legal tampering (Mon–Wed). **League year opens** Wednesday. Free agency wave 1. Trades agreed earlier become official. | Spring practice. | Track season. | Make offers to top free agents. Agents shop offers between teams over several days, so nothing resolves instantly. Decide on RFA tenders. | T2 |
| W5 | FA wave 2 (second tier, value deals). | Spring practice. | Track season. | Fill depth. React to players you lost. Revisit the draft board in light of signings. | T1 |
| W6 | Compensatory picks announced. FA wave 3 (bargains, one-year deals). | Spring practice. | Track season. | Final veteran depth signings. Note the new draft capital. | T1 |
| W7 | Annual league meeting votes on rule changes. | Spring practice. | Track season. | Vote on rule proposals (AI teams vote too, and rule changes alter sim balance). Trade talks for draft picks open. | T0 |

### Phase C. Draft build-up (W8–W10)

| Week | Pro league | College | High school | Player decides | Tier |
|---|---|---|---|---|---|
| W8 | Top-30 prospect visits. Teams with new head coaches start offseason programs early. | Spring games. Transfer decisions for players unhappy with spring depth charts. | Spring evaluation period opens: college coaches watch HS players in person. | Pick your 30 visit slots. Order medical rechecks. Build the big board by position and overall. | T1 |
| W9 | Offseason program **Phase 1** (strength and conditioning). Pro days end. | Spring games end. | Spring evaluation continues. | Lock the board. Set your draft-day strategy (trade up, trade down, best available, need). Field incoming trade calls. | T1 |
| W10 | **The Draft.** Thursday Round 1, Friday Rounds 2–3, Saturday Rounds 4–7. Undrafted free agent signing starts the moment the draft ends. | Drafted players leave campus. | Spring ball in states that hold it. | Every pick, every trade offer while on the clock, the UDFA scramble. | **T3** |

### Phase D. Development quiet (W11–W21)

This is the longest calm stretch in the year, and it is calm on purpose. The player works on development, contracts and the next generation of prospects. It mirrors FM's summer before pre-season friendlies.

| Week | Pro league | College | High school | Player decides | Tier |
|---|---|---|---|---|---|
| W11 | UDFA signings settle. Fifth-year option deadline approaching. | Finals. | Spring ball. | Exercise or decline fifth-year options. Assign rookies a development plan and a veteran mentor. | T1 |
| W12 | **Rookie minicamp.** | Summer session begins. | Spring ball ends. | First look at rookies on the field. Scouting confidence on them rises. Try out a few camp invitees. | T1 |
| W13 | **Schedule release.** Offseason program Phase 2 (on-field individual work). | Summer workouts. | Camp season begins. | Read the schedule (rest days, travel, short weeks). Plan the season's workload. Sign post-draft veterans. | T0 |
| W14 | Phase 3: **OTAs** begin (team drills, no live contact). | Summer workouts. | College camps and showcases. | Install playbook. Set install pace (fast install = more early mistakes from rookies). Watch for holdouts. | T0 |
| W15 | OTAs. | Summer workouts. | Showcases. Official visits begin. | Position moves (e.g. a safety tried at linebacker). Contract extension talks open. | T0 |
| W16 | OTAs. **June 1** release designations take effect. | Summer workouts. | Official visit season peaks. | Use June 1 releases for cap savings. Send scouts to HS camps (combine-style testing for underclassmen). | T1 |
| W17 | **Mandatory minicamp.** | Summer workouts. | Official visits. 7-on-7 circuit. | Respond to absences and holdouts (fine, negotiate, trade). Set a provisional depth chart. | T1 |
| W18 | **Dead period** begins (players off until camp). | Summer workouts. | Elite camps. Commitment wave starts. | Extension negotiations. Long-term scouting assignments for college and HS. | T0 |
| W19 | Dead period. | Summer workouts. | Commitments. | Extensions. Staff development (send coaches to clinics, which raise coaching attributes slowly). | T0 |
| W20 | Dead period. | Summer workouts. | Commitment wave peaks around early July. | Extensions. Read scouting reports that have now matured (reports take 1–3 weeks to arrive). | T0 |
| W21 | Dead period ends. | Fall camp prep. | Summer workouts. | Final pre-camp contract decisions. Choose camp structure (number of padded days, joint practice partners). | T0 |

### Phase E. Training camp (W22–W29)

| Week | Pro league | College | High school | Player decides | Tier |
|---|---|---|---|---|---|
| W22 | **Rookies report.** Deadline for long-term deals with tagged players. | Fall camp prep. | Summer workouts. | Last chance to extend tagged players or play the year on the tag. | T2 |
| W23 | **Veterans report. Training camp opens.** Position battles begin. | Fall camp opens. | Summer workouts end. | Set the battles you want to watch. Choose a practice intensity (higher means faster development and more injuries). | T1 |
| W24 | First padded practices. Camp reports start to show who is rising. | Fall camp. | Fall practice opens. | Read daily camp reports. Rotate first-team reps among battling players. | T1 |
| W25 | Hall of Fame game for two teams. Joint practices. | Fall camp. | Two-a-days. | Joint practice choices. Who plays in the first preseason game and for how long. | T1 |
| W26 | **Preseason game 1.** | Fall camp. | Scrimmages. | Snap counts. First real game tape on rookies. | T1 |
| W27 | **Preseason game 2.** | Fall camp ends. | Scrimmages. | Starters play more. Make trade calls for players you'll likely cut. | T1 |
| W28 | **Preseason game 3.** | **College Week 0.** | **HS Week 1.** | Final evaluations. Rank every player on the bubble. | T2 |
| W29 | **Cutdown to 53** (Tuesday). Waiver claims (Wednesday). **Practice squad** formed. Captains named. | College Week 1 (Labor Day weekend). | HS Week 2. | Who stays, who goes, who you claim off waivers, who you hope to sneak onto the practice squad. Set the Week 1 depth chart. | **T3** |

### Phase F. Early season (W30–W38)

The in-season weekly loop (section 4) runs every week from here to the end of the regular season.

| Week | Pro league | College | High school | Player decides | Tier |
|---|---|---|---|---|---|
| W30 | **Week 1** (Thursday opener, Sunday, Monday). | Week 2. | Week 3. | Weekly loop. Season opener presentation is T2: special, but early. | T2 |
| W31 | Week 2. | Week 3. | Week 4. | Weekly loop. First injury replacements. | T1 |
| W32 | Week 3. | Week 4. | Week 5. | Weekly loop. First quarter-season scouting assignments (college conference play starts). | T1 |
| W33 | Week 4. | Week 5. | Week 6. | Weekly loop. Players returning from injured reserve can be designated. | T1 |
| W34 | Week 5. **Bye weeks begin** (run to Week 14). | Week 6. | Week 7. | On your bye: self-scout report, extra rest, an injury recovery boost, an in-season scheme review. | T1 |
| W35 | Week 6. | Week 7. | Week 8. | Weekly loop. Early trade market forms around losing teams. | T1 |
| W36 | Week 7. | Week 8. | Week 9. | Weekly loop. | T1 |
| W37 | Week 8. | Week 9. | Week 10 (end of most HS regular seasons). | Weekly loop. Decide if you are a buyer or seller. | T1 |
| W38 | Week 9. | Week 10. | **HS playoffs begin.** | Weekly loop. Trade talks intensify, negotiated over days, not minutes. | T2 |

### Phase G. Deadline and playoff race (W39–W47)

| Week | Pro league | College | High school | Player decides | Tier |
|---|---|---|---|---|---|
| W39 | **Trade deadline** (Tuesday, 4pm). Week 10. | Week 11. | HS playoffs round 2. | Final offers. Every incoming call while the clock runs down. Then the week's game plan. | **T3** |
| W40 | Week 11. | Week 12. | HS playoffs. | Weekly loop. Live with the deadline's consequences. | T1 |
| W41 | Week 12 (Thanksgiving games). | **Rivalry week.** | HS regional finals. | Weekly loop. Short week for Thursday teams. | T2 |
| W42 | Week 13. | **Conference championships. Early signing period.** | State semifinals. | Weekly loop. Watch where your followed HS prospects sign. | T1 |
| W43 | Week 14. Last byes. | CFP field announced. Army–Navy. Award season. | **State championships.** | Weekly loop. Playoff picture appears in the news feed. | T2 |
| W44 | Week 15. | CFP first round. Bowl season begins. | Season over. Senior HS players get final evaluations. | Weekly loop. Start of "earned climax" eligibility (section 5.3). | T2 |
| W45 | Week 16. | Bowls. | All-star game selections. | Weekly loop. Rest starters or chase seeding. | T2 |
| W46 | Week 17. | CFP quarterfinals (New Year's). **Transfer portal opens.** | All-star practices. | Weekly loop. | T2 or **T3 (earned)** |
| W47 | **Week 18** (final regular-season week). "Black Monday" coaching firings follow. | CFP semifinals. **Underclassman draft declaration deadline.** Portal closes. | **HS All-American games.** | Weekly loop. Final seeding. Note which underclassmen declared. | T2 or **T3 (earned)** |

### Phase H. Postseason (W48–W52)

| Week | Pro league | College | High school | Player decides | Tier |
|---|---|---|---|---|---|
| W48 | **Wild card round.** Eliminated teams begin the offseason. | CFP title game (some years W49). | Recruits take final visits. | Win-or-go-home game plan. If eliminated: the review in W1 starts early for you, with scouting time at all-star games. | **T3** |
| W49 | **Divisional round.** | National title. All-star game rosters set. | Final visits. | Game plan. | **T3** |
| W50 | **Conference championships.** | **Senior Bowl** and Shrine Bowl practices and games. | Dead period before signing day. | Game plan. Scouts attend all-star week (the first time prospects face a real pro coaching staff). | **T3** |
| W51 | Bye week before the title game. Pro Bowl events. Awards. | Spring semester. | **National signing day.** | Prepare. If out, run all-star week interviews and finalize staff. | T1 |
| W52 | **Championship game.** | Spring semester. | Signing day aftermath. | The game. | **T3** |

The next year starts at W1 the following Monday. Aging, retirements and the yearly potential review (section 6) are processed in the background on that day.

---

## 4. The in-season weekly loop

From W30 to the end of the season every week has the same shape. The player is asked a few small questions every day and a few big ones on fixed days. Delegating any of these to staff is always allowed, with staff quality deciding how well they do it.

| Day | What happens | Player decides |
|---|---|---|
| Monday | Game review. Film grades per player. Injury diagnoses (some take days to confirm). Media on the last game. | Accept or adjust grades for development purposes. Talk to players (praise, criticism) which moves morale. |
| Tuesday | Players' day off. Waiver claims process. Practice squad changes. Free agent workouts. | Roster moves: IR, waivers, signings, practice squad. Before the deadline, trade talk. |
| Wednesday | Practice 1 (base install). First injury report. | Game plan: weekly emphasis (e.g. run-heavy, blitz early), personnel packages, practice intensity. |
| Thursday | Practice 2 (third down, red zone). | Adjust plan on practice reports. Decide on players listed as limited. |
| Friday | Practice 3 (walkthrough). Final injury designations (out, doubtful, questionable). | Final depth chart. Game-day inactives shortlist. |
| Saturday | Travel. College games around the country. Practice squad elevations deadline. | Elevate up to two practice squad players. Scouting reports from college games arrive over the following days. |
| Sunday (or Thu/Mon) | **Game day.** | Choose how to watch: full play-by-play coaching, key-moments mode, or result only. During the game: play calls or tendencies, timeouts, fourth downs, challenges, halftime adjustments. |

Scouting runs alongside the loop. The player assigns scouts to college games, HS games and pro opponents on Monday, and reports come back after a delay. That delay is a core pacing tool: the player knows more next month than this week.

Thursday and Monday night games compress or stretch this loop. Short weeks cut a practice day (more mistakes, less install). After a long week the team gets an extra practice day.

---

## 5. Pacing: how tension builds without rushing

### 5.1 Principles

1. **Time is the resource.** Almost nothing important resolves in the same tick it starts. Contract talks, trades, injuries, scouting and development all play out over days or weeks.
2. **Climaxes are scarce.** A handful of moments per year get the full treatment. Everything else is quieter by design, so the big moments land.
3. **Quiet weeks still matter.** A low-tension week asks small questions with long consequences: a scouting assignment, a mentor pairing, a camp invite. These are the "patient" decisions that pay off years later.
4. **Information arrives in layers.** Nobody, including the player, ever knows a prospect's ceiling for certain. Each event on the calendar adds a layer of evidence (section 6).
5. **Consequences echo forward.** A 15-year-old you scouted at a summer camp in year 1 may be your first-round pick in year 5. The calendar is built so those threads run in the background and surface at their natural moments.

### 5.2 Tension tiers

Every event in the game has a tier. The tier controls how it is presented and whether it may stop time.

| Tier | Name | Presentation | Stops time? | Examples |
|---|---|---|---|---|
| T0 | Ambient | Inbox line or news ticker only | No | Schedule release, staff clinic, HS commitment of an unfollowed player, rule vote |
| T1 | Notable | Inbox item with a decision or a report | Only if a decision is due | Combine results, camp reports, a regular-season game, compensatory picks, a followed prospect committing |
| T2 | Major | Its own screen, news headlines, media reaction | Yes | Free agency opening, tag deadline, final preseason evaluation, season opener, rivalry week, a star's contract holdout |
| T3 | Climax | Full presentation: dedicated screen, slowed time, crowd/war-room audio, live ticker of other teams | Yes, and cannot be skipped through without a confirmation | See 5.3 |

### 5.3 Which events may be climaxes

**Fixed climaxes (every year):**

| Event | Week | Why it earns T3 |
|---|---|---|
| The Draft (rounds 1–3 in full, rounds 4–7 at T2) | W10 | Payoff for 10 weeks of scouting and a decade of HS-to-college development in the sim. |
| Cutdown day | W29 | Payoff for all of camp. Every bubble player's story resolves on one day. |
| Trade deadline | W39 | The season's fork: commit or rebuild. |
| Playoff games | W48–W50 | Win or go home. |
| Championship game | W52 | The end of the year. |

**Earned climaxes (only when the season creates them):** a regular-season game is promoted to T3 only if it meets at least one test:

- It decides a division title or a playoff berth for the player's team (from W44 on).
- It is the first start of a player the player drafted in round 1 *and* who was scouted by the player's staff in high school (the long thread paying off).
- A franchise record or historic milestone is genuinely on the line in that game.

At most **two** earned climaxes can occur in a regular season. If more qualify, only the highest stakes ones get T3; the rest stay T2.

**Things that are never climaxes:** free agency (it is a T2 spike, and offers resolve over days so it never becomes a frenzy), the combine (it is information, not a result), preseason games, schedule release, awards, and anything in the HS or college world on its own. College and HS events reach the player as T0–T1 unless they involve a prospect the player follows, which caps them at T2.

### 5.4 Cooldowns

- After any T3, the following 5 game days contain only T0 and T1 events for the player's team, except injuries and fixed calendar events that cannot move.
- No two fixed climaxes are less than 3 weeks apart (the calendar above already meets this from W10 to W29 to W39; playoffs are the deliberate exception, as a run of climaxes is the point of the postseason).
- The 11-week development quiet (W11–W21) is protected. The only T2 events allowed there are a star holdout or a major injury.

### 5.5 The tension curve over a year

| Stretch | Weeks | Shape | Peak |
|---|---|---|---|
| Rise 1 | W1–W10 | Review (T1), combine (T1), tag and FA spike (T2), then a steady build of visits and board work | Draft (T3) |
| Trough | W11–W21 | Protected quiet: development, contracts, the HS recruiting world simmering | None |
| Rise 2 | W22–W29 | Camp reports (T1), preseason (T1), final evaluations (T2) | Cutdown day (T3) |
| Rise 3a | W30–W39 | Opener (T2), weekly loop (T1), trade market forming (T2) | Trade deadline (T3) |
| Rise 3b | W40–W47 | Playoff race, rivalry week, seeding (T2), up to two earned climaxes | Earned climaxes (T3) |
| Finale | W48–W52 | Back-to-back climaxes, with a T1 bye week before the final | Championship game (T3) |

Three long rises, each ending in a climax, with one long protected trough between the draft and camp.

### 5.6 Anti-rush rules

These stop the sim from resolving drama faster than real football does.

- **Negotiations take days.** A contract offer gets a response in 1–4 days. Counter-offers add more days. Agents talk to other teams in between.
- **Trades take days.** A trade proposal sent on Monday may get an answer by Wednesday. On deadline day only, responses come within hours, which is part of why it is a climax.
- **Scouting reports take 1–3 weeks** depending on scout quality and distance. The combine and pro days are the exception, because the data is public.
- **Injuries unfold.** A player leaves a game "questionable to return", gets an MRI Monday, a diagnosis Tuesday, and a return timeline that can change.
- **Development is shown in trends, not jumps.** Attribute changes are applied weekly in small steps but the player sees them in monthly snapshots with trend arrows.
- **Breakouts are telegraphed.** A player cannot become a star overnight. Practice reports, coach comments and snap-count growth precede any big rating jump by at least 3 weeks.
- **Continue always stops before a T3.** "Advance to next event" can skip quiet days but never jumps past a climax without showing a build-up screen.

---

## 6. How development and potential follow the calendar

Dynamic potential is a range, not a number. It narrows as evidence arrives and can move up or down. The calendar decides when evidence arrives.

| Calendar moment | Weeks | What it reveals |
|---|---|---|
| HS spring evaluation, summer camps, 7-on-7 | W8–W22 | First athletic testing for underclassmen (40 time, shuttle, vertical). Wide potential range. |
| Track season | W2–W12 | Real speed data for HS athletes. |
| HS season | W28–W43 | Game tape. Growth spurts can happen in-season for 15–17-year-olds. |
| HS all-star games | W46–W47 | Elite-vs-elite tape for top seniors. |
| National signing day | W51 | Where the player will develop (college coaching quality now affects growth). |
| College spring practice and spring game | W1–W9 | First reports on early enrollees and redshirts. |
| College season | W28–W49 | Production and tape. Each season narrows the range further. |
| Transfer portal | W46–W47 | Players move to new systems. Fit changes development. |
| Draft declaration deadline | W47 | Underclassmen decide whether to enter. Early entrants carry wider ranges. |
| Senior Bowl and Shrine Bowl | W50 | How a prospect handles pro coaching. |
| Combine and pro days | W2–W9 | Verified measurements and medicals. Interview personality data. |
| Rookie minicamp, OTAs, camp | W12–W29 | Pro-level evidence. The range narrows sharply in year one. |
| Yearly potential review | W1 | Ages everyone a year. Applies the big age-based shifts (late bloomers, early peakers, physical decline from late 20s). |

Rules of thumb for realism:

- A potential range can shift by at most one band per checkpoint, so "suddenly a five-star" cannot happen. **Exception (decided by Raichel, 2026-10-10):** a proven breakout season, meaning production well above what the player's ratings predict over a real sample of snaps, can lift the ceilings behind that production by up to about 10 points in a year. This can happen at any age, with smaller lifts as players get older.
- Most potential movement happens between ages 15 and 23, slows from 24 to 26, and the ceiling only falls after that.
- Coaching quality, playing time, injuries, personality (work rate, coachability) and scheme fit drive which way a range moves. Luck exists but is small.
- The player never sees a true number, only their staff's estimate, which is as good as their scouts.

---

## 7. Time controls and delegation

- **Continue** advances one day, or to the next day with a decision, a T2 or a T3.
- **Holiday mode** (from FM) lets the player hand everything to staff until a chosen date. It always stops for T3 events unless the player opts out per event type.
- Every recurring decision has a "delegate to" setting: GM, coordinator, director of scouting, cap manager. Staff with poor attributes make worse calls, which is how delegation stays a trade-off rather than a shortcut.
- In the development quiet (W11–W21) the game suggests "advance to camp" with a summary of what staff will handle, so players who want speed can have it without the design losing its slow default.

---

## 8. Implementation notes

- Store the calendar as data: a list of events with `week`, `day`, `level` (pro/college/hs), `tier`, `blocking` (whether it waits on a decision), and the rule set that created it. Real dates are derived from anchors (championship game date, Labor Day, Thanksgiving) so the calendar shifts naturally year to year.
- Rule sets (roster sizes, playoff format, signing periods, portal windows, cap rules) are versioned by league year so history stays correct and rule changes voted on in W7 can take effect the next season.
- Tiers are computed at runtime for earned climaxes, using the tests in 5.3, and capped by the cooldown rules in 5.4.
- High school seasons vary by state (start dates, number of games, spring football or not). Model states as regions with their own sub-calendars; the table above uses a typical large football state.

---

## 9. Open questions for Raichel

1. **Player role:** pro team only (default), or should a college or high school coaching career be playable from launch?
2. **Save start:** mid-February before the combine (default), or mid-July at training camp like FM?
3. **Climax budget:** is two earned regular-season climaxes per year the right ceiling, or should it be one?
4. **Real vs fictional:** keep real league structure and rules with fictional names (default), or a fully fictional league?
