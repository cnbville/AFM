# Section 4: the season calendar and pacing (built)

Status: design finished 2026-10-14. All eleven topics are answered. On 2026-10-15 Cain confirmed 46 additions (`docs/design/calendar-additions.md`) and asked for one build plan covering both (`docs/design/calendar-build-plan.md`). **Built 2026-10-16**: each point with its code is in `docs/design/calendar-checklist.md`, and how it works is in `docs/world-model.md` ("Section 4: the calendar").

This spec is written from the "Section 4 calendar design" thread in the project chat. Raichel's own words are in quotes. It replaces the old draft `docs/design/season-calendar-and-pacing.md` wherever the two differ.

Labels:
- **Confirmed**: Raichel answered it.
- **Proposed, not confirmed**: Claude's draft. Ask Raichel before building on it.
- **Parked**: Raichel wants it, and the design comes later.

Dates marked "source-checked" were verified against real sources during the design (Raichel asked for a source check on topic 3). Re-check every real date against current sources when building.

---

## 0. Ground rules for this section

- **Every level runs its real calendar and rule book.** Your team and every AI team, college and high school use the same calendar and the same rules.
- **Real events drive decisions, not fixed dates.** Players lean toward a choice all season, and a window is only when they act on it (topic 3, the portal).
- **Nothing happens instantly** when it doesn't in real life (topic 9).
- **The world is built for a possible NCAA coach career later (parked).** Recruiting is built as real actions that a human could use too (topic 5).

### Parked for later design
1. **Watching players' offseason work.** Raichel: "i'm interested in building an creative way wherre we can see and montiors player work in the off-season. We'll come to that later though. I just wanted to mention it so you leave room for that." The calendar leaves room: the NFL summer break and the weeks outside team practice are a window where players train on their own, with their own trainers, and that work shows up later. How you watch it gets designed later (probably section 6b).
2. **A possible NCAA coach career.** Raichel: "i want to build the dynamics into the game for realism and potentially a feature where i become an NCAA coach." So recruiting actions (visits, practice drop-ins, calls, offers) are real actions that AI college staffs use now and a human coach could use later.

---

## Topic 1. The clock (confirmed)

1. **One day at a time.** Continue skips quiet days and stops when something needs you.
2. **One real clock for every level.** High school plays Friday night, college Saturday, the NFL Sunday, plus Thursday and Monday night, Thanksgiving, Christmas and late-season Saturday games.
3. **Dates are worked out each year from real anchors,** so the year shifts the way it really does. Examples: the NFL opener is the Thursday after Labor Day, the Super Bowl is the second Sunday in February, the draft is the last Thursday to Saturday of April. 2026 uses the real 2026 dates.
4. **A save starts the day after the Super Bowl** (mid-February, from the roadmap answers).
5. **The biggest days run hour by hour** (Raichel: "yes"): free agency opening, the trade deadline, cutdown day, the draft and the UDFA rush (signing undrafted players right after the draft). On those days, who acts first matters. Example: an agent fields three offers within an hour.

## Topic 2. The NFL year (confirmed)

The real league calendar and its rule book (the CBA, the deal between the league and the players' union):
- **February:** the coaching carousel finishes, reserve/future deals (cheap contracts for next season) are signed, the salary cap is set.
- **Late Feb to early March:** the combine. The franchise tag window opens (a one-year deal that keeps a star from free agency).
- **Mid-March:** two days of legal tampering (teams may talk to other teams' players but not sign them). Then the new league year opens at 4pm ET: free agency, trades, expiring deals. Restricted free agents (3 years in the league) can get offer sheets until late April.
- **Late March:** the league meeting (rules votes, see topic 11) and compensatory picks (extra picks for losing free agents).
- **March to April:** pro days, the 30 official visits each team may host, the draft in late April, then the UDFA rush.
- **May to June:** rookie minicamp, OTAs (voluntary practices), the fifth-year option deadline (an extra year on first-round deals), schedule release in mid-May, mandatory minicamp in June, then the summer break.
- **July 15:** the deadline for long-term deals with tagged players.
- **Late July to August:** training camp, three preseason games (four for the Hall of Fame Game teams), one cut from 90 to 53, then waivers and practice squads.
- **September to January:** 18 weeks, 17 games, one bye. Trade deadline the Tuesday after week 9. Flex scheduling and international games included.
- **January to February:** Black Monday (coach firings the day after the season), 14 playoff teams, the Pro Bowl Games, the Super Bowl.

Decisions:
1. **The CBA offseason limits bind your training** (which weeks you may practice, no pads until camp, contact limits). Raichel: "yes", with room left for the offseason work feature (parked item 1).
2. **Year one uses the real 2026 schedule** (real dates and prime-time games). Later years are built by the real opponent formula. Raichel: "yes".

## Topic 3. The college year and the portal (confirmed)

The real NCAA calendar, on the 2026 rules:
- **January:** the transfer portal, players' draft decisions, early enrollees arrive for the spring semester.
- **February to April:** National Signing Day, then spring practice (15 practices, ending in the spring game).
- **April to June:** official visit season for recruits.
- **Summer:** limited team workouts (8 hours a week for 8 weeks). The school money year starts July 1. Revenue sharing (schools paying players directly) is capped at about $20.5M per school.
- **August:** fall camp (29 practices), then Week 0 in late August.
- **Season:** 12 games. In season, team activities are capped at 20 hours a week and 4 a day. A player can play up to 4 games and still redshirt: 5 years to play 4. Rosters are capped at 105.
- **November to December:** rivalry week, coach firings, conference title games, Army-Navy, the early signing period (first Wednesday of December), CFP selection (12 teams).
- **Mid-December to January:** CFP first-round games on campus, bowls, the quarterfinals and semifinals at the big bowls, the title game around Jan 19.
- **Other levels** run their own real calendars: FCS (24-team playoff), D2 (28), D3 (32), NAIA and JUCO.

**January deadlines (source-checked 2026-10-14):**
- **Portal:** one window, January 2 to 16. The spring window is gone. Players on CFP teams get 5 extra days after their season ends. When a head coach leaves, players get 15 days, and the clock starts when the new coach is hired, not when the old one is fired.
- **NFL draft** (the 2025 rules, the latest published): a soft deadline on Jan 6 for players whose season ended early, a hard deadline Jan 15. Players in the CFP title game get until about Jan 24, then 72 hours to take it back. A later withdrawal (opt-out) deadline is Feb 7.
- So the portal locks in mid-January, but the draft choice can still change until early February.
- **Still to check:** how late a player can withdraw from the draft and still keep his college eligibility.
- Sources: [ESPN](https://www.espn.com/college-football/story/_/id/46524004/ncaa-adopts-jan-2-16-transfer-portal-window-fbs-fcs-26), [Buckeye Sports](https://www.buckeyesports.com/ncaa-approves-single-transfer-portal-window-from-january-2-16-for-2026), [NFL.com](https://nfl.com/news/league-implements-new-deadlines-for-2025-nfl-draft-special-eligibility).

**The portal, driven by real events (confirmed).** Raichel: "i'm trying to find an dynamic edge to it where they choose based on real life events".
- A player doesn't decide on January 2. All season, an **inner lean** (stay or go) builds from what happens to him. The window is only when he acts on it.
- **Events that move the lean:**
  - Playing time: benched, losing the depth chart battle, a starter returning from injury.
  - His school signs a five-star at his position in the December early signing period.
  - Coaches: his head coach is fired, his position coach leaves, or a new scheme doesn't fit him. Each is felt on its own.
  - Money: his NIL deal shrinks, the collective dries up, or another school's offer reaches him through a handler (tampering: illegal, but real).
  - Life: a parent falls ill (move closer to home), homesickness, a breakup, academic trouble, friends on the team leaving.
  - Draft read: a weak NFL grade can push him toward a bigger stage to prove himself.
  - The season: no bowl game, or a breakout season that makes him wanted.
- **How it plays out:**
  - The redshirt trick: some players who know they're leaving stop playing after 4 games to keep the year.
  - Timing: stars often have a deal lined up and enter on day 1. Fringe players wait and see. Some enter and never find a school ("portal regret").
  - Once he enters, his school can cut his NIL. Coming back is the school's choice.
  - Personality sets how much each event weighs: a loyal player shrugs off a benching, an ambitious one leaves over a lost snap share. No single event decides it.
- **Whispers.** You hear about a player's lean before he enters; better-connected scouts hear earlier and more accurately. Example: "his family is unhappy with his snaps." Raichel: "yes definitely get whispers, it should be part of the game just like lockerroom vibe."
- **Coaches sway it.** Raichel: "Coaches should also have a recruiting skill that influences outcomes or can sway players." Coaches already have a recruiting skill and a player-relationships skill (section 2). A strong recruiter can pull a transfer to his school even against a better offer. A coach with strong relationships can talk a wavering player into staying (for example, sitting down with the family after a benching); a poor one makes the lean worse. Neither skill decides it alone.

Decisions:
1. **One combined January choice per player:** declare for the draft, enter the portal, or stay. He weighs his draft grade, NIL and portal offers, his playing time, his degree and his personality, all inside the same short window. Raichel: "Yes".
2. **Bowl opt-outs:** players with high draft grades skip minor bowls, rarely CFP games, driven by draft grade, personality and what's at stake. Raichel: "Yes".

## Topic 4. The high school year, per state (confirmed)

Every state runs its own real calendar. The data already has each state's classes (1A to 6A and so on), playoff format, title-game dates and transfer rules. Examples: Texas finals are in mid-December at AT&T Stadium; California runs section playoffs, then state regionals.

Added:
- **Season start and length per state.** Alabama and Georgia start in mid-August, many northern states in late August. Most play 9 to 10 regular-season games.
- **Spring football,** allowed in states like Texas, Florida, Georgia and Alabama, banned in many others.
- **Summer:** state dead periods (coaches may not work with players), camps, 7-on-7 tournaments, college camps where kids get seen.
- **The year around it:** winter lifting and track, junior days at colleges (January to March), early graduation in December for early enrollees, January all-star games (the All-American Bowl, the Polynesian Bowl), state all-star games in summer.
- **Playoffs** by each state's format, and the "transfer season" each state's rules allow.

Decisions:
1. **Spring ball is a breakout stage.** Raichel: "make spring ball a place where if a (RS) freshman or sophomore (at max) has all the right combinations he can get like an instant ability boost. In real life sometimes the players really seperate themselves unexpenctantly." And on who qualifies: "use common sense, redshirt, true freshman AND sophomore."
   - Eligible: true freshmen, redshirt freshmen and sophomores. No older players.
   - It applies in both college spring practice and high school spring ball (in the states that have it). Raichel: "both".
   - The jump needs the right mix: work ethic, coachability, a good position coach, open reps on the depth chart, a body that's filling out, and room under his hidden ceiling. When the mix is there, he can make a sudden, real step that lands by the spring game.
   - **A notice comes first,** early in spring. Raichel: "if player so and so keeps mainainting x and y. He'll likely make huge steps". Example: "If Marcus keeps his weight up and keeps winning his footwork reps, he'll likely make a big step." Notices are sharper with better staff, and some don't pan out.
   - Spring football also gives players in those states extra reps and an extra chance to be seen in the spring evaluation period.
2. **National independents like IMG Academy exist, kept in check.** Raichel: "be careful with the pulling since we already had a lot of trouble calibrating dynasties in NCAA".
   - Only a handful exist.
   - Tuition (about $90k a year) means family wealth matters.
   - Boarding away from home costs homesick kids.
   - A roster has only so many starting spots, so they can't hoard talent.
   - They give exposure only, with no extra pull on which college a kid picks.

## Topic 5. Recruiting periods, visits and signing (confirmed)

The real FBS recruiting calendar (source-checked). The NCAA sets fixed periods:
- **Contact period** (coaches may meet recruits in person), **evaluation period** (watch but not talk), **quiet period** (contact on campus only), **dead period** (no contact).
- The 2025-26 calendar as an example: quiet Mar 2 to Apr 14, contact Apr 15 to 30, quiet in May, dead May 24 to 27, quiet May 28 to Jun 21 (camp season and the main window for official visits), dead Jun 22 to Jul 31.
- **Official visits** (paid by the school) from April 1 of the junior year. **Unofficial visits** (paid by the family) any time outside dead periods.
- **Signing:** the early period on the first Wednesday of December, then February.
- **During the season:** game-day visits, coaches watching Friday nights in evaluation weeks, offers that build up over the years.
- Sources: [Texas Football: 2025-26 FBS recruiting calendar](https://www.texasfootball.com/article/2026/03/09/the-2025-26-division-i-fbs-recruiting-calendar), [MGoBlog: NCAA visit rule change](https://mgoblog.com/mgoboard/ncaa-passes-major-re-vamp-recruiting-rules).

Decisions:
1. **Commits, decommits and flips work like the portal lean.** Events move them: a coach fired, a five-star signed at his position, NIL money, a handler steering him, a visit that goes badly or well. Coaches' recruiting skill sways it too.
2. **When a kid commits comes from his personality and family.** Some commit early as sophomores, some take every visit, some wait for signing day and the TV cap ceremony.
3. **Committed and local kids allow more.** Raichel: "add aspects, like if a player is already commited and from a local school. I'll be able to do more then in the offical week days. visit HS practice etc"
   - **Local kids:** the college staff can drop by high school practice in evaluation weeks, and the kid comes to games on easy unofficial visits. His family can watch home games, so the bond builds faster and costs less.
   - **Committed kids:** coaches keep "babysitting" the commit with regular contact to hold him. Once he signs, contact is unlimited.
   - **Committed kids recruit too:** a committed local star pulls teammates and friends toward the same school. Real, but small.
4. **Built as real actions.** Visits, practice drop-ins, calls and offers are actions AI college staffs use now, the same actions a human NCAA coach could use later (parked item 2). NFL scouts don't visit high school practices, as in real life.

## Topic 6. The in-season week (confirmed)

**NFL (a Sunday game):**
- **Monday:** film review, lifting, injury exams (MRIs), your press conference.
- **Tuesday:** players' day off. Coaches build the game plan. Waiver claims process at 4pm ET.
- **Wednesday:** install the base plan (1st and 2nd down). The injury report starts (did not practice, limited, full).
- **Thursday:** third downs and the red zone.
- **Friday:** situations and the two-minute drill. Final game statuses (out, doubtful, questionable).
- **Saturday:** walkthrough, travel, team hotel. Practice squad call-ups due by 4pm ET.
- **Sunday:** the inactives list 90 minutes before kickoff, then the game.

**What shifts the rhythm:** Thursday and Monday night games make short or long weeks. The bye gives players 4 days off by the CBA, and teams self-scout during it. Long trips and time zones matter (a West Coast team at a 1pm East Coast kickoff). The CBA caps padded practices at 14 per season. Other teams can sign your practice squad players. IR players get a 21-day practice window before they must be activated.

**Your scouts' week:** college games on Saturday, visits to college practices midweek, and advance scouting of the next opponent.

**College:** a 20-hour week with one required day off. Practice Tuesday to Thursday around classes, travel Friday, game Saturday. Coaches recruit at high school games on Friday nights.

**High school:** film Monday, practice Tuesday to Thursday, game Friday, JV and film Saturday.

Decisions (Raichel: "yes yes"):
1. **Each day has its own decisions:** the game plan on Tuesday, the practice plan and reps Wednesday to Friday, injury statuses Friday, inactives Sunday. Staff can handle any of them.
2. **A game plan has an install limit.** The more plays and checks you put in, the more mental errors players with low football IQ make. A short week shrinks how much they can learn. "How much to install" is a real trade-off.

## Topic 7. Where training sits in the calendar (confirmed)

Only when each kind of training can happen. How training works is section 6b.

**NFL blocks (from the CBA):**
- **Feb to Mar:** players on their own; rehab can happen at the team facility.
- **Phase 1 (2 weeks, April):** strength and conditioning only. Teams with a new head coach start earlier and get an extra minicamp.
- **Phase 2 (3 weeks):** individual drills, no offense against defense.
- **Phase 3, OTAs (10 days) and mandatory minicamp (3 days):** 7-on-7 and 11-on-11 without live contact. The best time to learn the scheme and switch positions.
- **Summer break (about 5 to 6 weeks):** players train on their own. The offseason monitoring feature lives here (parked item 1).
- **Camp:** 5 days without pads to get used to the heat, then pads. Veterans get rest days.
- **Season:** practice Wednesday to Friday, 14 padded practices in total, the bye.

**College:** winter conditioning (Jan to Feb), 15 spring practices, 8 hours a week in summer, 29 practices in fall camp, 20 hours a week in season.
**High school:** offseason lifting, spring ball where allowed, summer 7-on-7 and dead periods, then the season.

Decisions (Raichel: "yes yes"):
1. **Each block favors a different kind of growth:** body and strength in the offseason, technique and scheme in OTAs, reps and the depth chart battle in camp, mostly maintenance in season. Development speed follows the calendar.
2. **Training load cuts both ways:** more padded, high-tempo reps mean more growth, but also more soft-tissue injuries and tired legs on game day. Rest days for veterans are a real call.

## Topic 8. Pacing: how the year builds without rushing (confirmed)

Raichel found the first wording confusing ("you're wording is confusing"), so this is the simple version that was answered.

**The idea:** some moments in the year are big and some are small. The game shows big moments loudly and small ones quietly, so the big ones feel special.
- **Small:** a line in the news. Example: a recruit you don't follow commits.
- **Medium:** a message in your inbox. Example: combine results.
- **Big:** the game stops and shows its own screen. Example: free agency opens.
- **Huge:** the full show, with the war room and live updates. Continue never skips past one. These happen every year: the draft, cutdown day, the trade deadline, the playoffs and the Super Bowl.

**The shape of the year:** three long rises, each ending in a huge moment: combine to draft, camp to cutdowns, the season to the deadline and playoffs. A quieter stretch from May to July sits in between. Quiet weeks still ask small questions with long tails, like a scouting assignment or a mentor pairing.

Decisions (Raichel: "yes and no"):
1. **A regular-season game can become huge, but only when it really matters.** Example: you must win to make the playoffs, or it's the first start of a QB you scouted since high school. Judged from real stakes (playoff odds swing, a division title, a rivalry, your job on the line), with no fixed cap. Most seasons have 1 to 3; some have none. This replaces the old draft's cap of 2.
2. **No holding back news after a huge moment.** Real life doesn't wait: if a player gets arrested the day after the draft, it happens. The game just shows it a little quieter. This replaces the old draft's 5-day hold.

## Topic 9. How long things take (confirmed)

In real football almost nothing happens instantly. The game copies the real waiting:
- **Contract offer:** the agent answers in a few days and talks to other teams in between. On day 1 of free agency, top players sign within hours.
- **Trade offer:** an answer in a day or two; on deadline day, within hours. A trade only counts after the player passes his physical.
- **Scouting report:** 1 to 3 weeks, faster with good scouts. Combine numbers come out instantly.
- **Injury:** "questionable to return" during the game, a scan Monday, the diagnosis Tuesday. The return date can still change.
- **Hiring a coach:** weeks of interviews, under the real rules (you can't interview a coach whose team is still in the playoffs until it's out).
- **Waivers:** 24 hours; any team can claim the player in that time.
- **Suspension:** an appeal takes weeks.

Decisions (Raichel: "yes and yes"):
1. **Waits come from the people involved, not fixed numbers.** A tough agent with a hot player waits for more offers; a player nobody wants answers fast.
2. **Deals can fall apart while you wait.** Another team signs your free agent first, or a trade dies on a failed physical.

## Topic 10. Time controls and handing work to staff (confirmed)

- **Continue** moves the game forward, skips quiet days, and stops when something needs you.
- **Holiday mode** (like FM) lets staff run everything until a date you pick. It always stops for the huge moments, like the draft.
- **Handing off tasks:** any task can go to a staff member (scouting to the scouting director, the cap to the cap manager, the game plan to a coordinator), with instructions like "never trade a first-round pick" or "ask me before cutting a starter".

Decisions (Raichel: "yes yes"):
1. **Staff decide like real people,** from their own skill, personality and habits, and leave a reason you can read. Example: an old-school coordinator keeps playing a veteran over a better rookie, and his note says why.
2. **Staff given real responsibility grow** and gain a reputation, so other teams try to hire them away (coordinators becoming head coaches).

## Topic 11. Rule changes over the years (confirmed)

- **A rule book for every year.**
- **The 30 hidden years before 2026 use each era's real rules.** Examples: 16 NFL games until 2021, then 17; the BCS, then a 4-team playoff, then 12 teams; no portal before 2018; no NIL before 2021.
- **The salary cap** grows with league money each year.

Decisions:
1. **On-field rules never change during a career.** Raichel: "no because it may cause engine breaking aspects that i can't fix in game". The rules on the field stay at the 2026 rules for the whole career. Nothing like the kickoff or passing rules changes during play, and the world does not react to events by changing playing rules.
2. **Off-field changes only from a fixed, agreed list.** Raichel: "yes but make fixed possibilities that we agree upon before". Only these can ever happen, each with a realistic chance and through the real process (NFL owner votes in March need 24 of 32; the CBA ends after the 2030 season; college changes come through NCAA votes, courts and conferences). The list, confirmed 2026-10-14 (Raichel: "sure"):

   **NFL**
   1. 18-game season (with 2 byes)
   2. New CBA after 2030: a deal, or a lockout that freezes the offseason (like 2011)
   3. Playoffs grow to 16 teams
   4. Roster or practice squad size changes
   5. A team relocates to a new city

   **College**
   6. A bigger playoff (14 or 16 teams)
   7. Schools switch conferences
   8. The SEC and Big Ten break away from the rest
   9. 5 years to play 5 (no more redshirt rule)
   10. The revenue-sharing cap goes up or down

   **High school**
   11. More states allow high school NIL
   12. States change their transfer rules

   Anything not on this list stays fixed. Adding to it needs Raichel's OK.

---

## Open items

- **Still to check:** the latest date a player can withdraw from the NFL draft and keep his college eligibility (topic 3).
- **Chances for each off-field change** (topic 11) are not set yet: proposed, not confirmed. Ask Raichel when building.
- **Parked:** offseason work monitoring (designed in section 6b, `docs/design/training.md` topic 9) and an NCAA coach career (now decided: your career at every level, decisions section 12).
- **Where the build goes in the order:** ask Raichel.
