# World trial 6: what to fix and what to decide

Written 2026-10-16 from world trial 6 (`world-trial6.md`; Debug Lab Verdict tab: https://claude.ai/artifact/3SAzca1V9NBFKkW5pAPTSt). Each item is one aspect, a line on what was seen and a line on what changes. Everything here is proposed, not confirmed.

## Part A: being fixed now, no answer needed (17)

Clear bugs, export gaps and fixes from the first round that fell short. Cain asked for section 4 finished without approvals, so these are built straight away, picking the most realistic option, and measured with one world run. Say stop on any you disagree with.

F1. **A camp redshirt who plays is still developed and shown as a redshirt** (suggestion 2 in the Debug Lab). When a camp redshirt plays, the yearly step treats that season as a backup season (games, performance, form, injuries), and his row says backup.
F2. **An early enrollee's college spring step lands on his high school row**. An early enrollee's college spring step opens his first college row instead of ending his high school row.
F3. **Draft notes say 'a EDGE' and 'a OT'**. The note writes 'an' before EDGE, OT, IOL, LB, S, RB and LS.
F4. **Signing-day news is filed under the next football year in most years**. File a signing stage's news under the class's season too, as fix A6 did for the counters.
F5. **Specialist signing notes repeat the same star count**. Add the camps' stars to the note only when they differ from the services'.
F6. **Long snappers almost never log a game, so their seasons go uncounted**. Put the long snapper (and the holder) on the field for kicks and punts, so their games count.
F7. **The Shrine Bowl and the Senior Bowl get the wrong talent tiers** (suggestion 4 in the Debug Lab). Let the Senior Bowl name its rosters first every year, and the Shrine Bowl take the next tier after it, whatever the game dates.
F8. **The awards show crowns the same players year after year** (suggestion 5 in the Debug Lab). Judge the awards show from the season's production against the position's best, with team success, the way the Heisman now works.
F9. **The Commander-in-Chief's Trophy has two holders at once**. Treat it as one three-way trophy, settled after the Army-Navy game.
F10. **A team's own huge game can still be posted as Medium**. Post a game's result at least Big when it is huge for one of its teams.
F11. **Momentum hits on interception returns are skipped**. Look the player up on the team he plays for, as the stat fix does. This changes game results a little, so it needs a world run to measure.
F12. **Player pages' season rows show no events**. The player page rows carry the season events, games missed, the dip and the spring gain.
F13. **The dashboard's day-one share mixes windows**. In build_data.py, merge rows by date and split them by window (coach-change, winter, spring) before computing the first-day share.
F14. **Give the redshirt role only to freshmen the staff means to sit** (suggestion 1 in the Debug Lab). world/season.rs depth_chart gives the redshirt role only to freshmen below the travel squad. Freshmen in the two-deep play a normal season in every era, and the four-game stop and the pre-2018 burns then touch only the players a staff meant to sit. Check the redshirt trick count again after it.
F15. **One age rule for every note in a season and its offseason (D5)** (suggestion 3 in the Debug Lab). Signing, flip, early-enrollment, redshirt, award, NIL and portal notes use the age on that season's row, the rule D5 already uses for the high school notes.
F16. **Place the Toronto game before the London games** (suggestion 14 in the Debug Lab). formula_slots chooses the Toronto game among Bills home games whose opponent has not gone abroad, or places it before the London games.
F17. **Let the signing-day fallback use any walk-on place** (suggestion 16 in the Debug Lab). In the fallback, a dropped recruit may take a walk-on place at any listed program with roster room at his position, even when the plan's walk-on places are used.

F11 (momentum on interception returns) changes game results a little; the world run after the fixes measures it.

## Part B: yes or no (38)

Numbers match the Debug Lab, where each item has yes / no / later buttons and a button that copies the answers. Fixes first, then creative ideas.

### Realism fixes (12)

6. **Let ratings drift between passes instead of being drawn again** (Recruiting)
   - Seen: Only 25% / 28% / 27% of recruits called 5-stars at commitment end as 5-stars, and 38% / 36% / 35% end at 3 stars or fewer. The same new draw at every pass likely feeds the 4-stars who never reach FBS and the high 5-star draft rate.
   - Change: Each rating pass moves a recruit's services' projection only part of the way (for example halfway) toward the new read. Then re-measure the 4-star landing and 5-star draft items before touching them.
7. **Rate the class with the staffs' weights and draw** (Recruiting)
   - Seen: 15.6% / 15.7% / 14.8% of final 4-stars in classes 1997-2017 never played college (43.8 / 44.3 / 41.0 a class), because the services weigh projection 0.75 and the staffs 0.4, on different draws.
   - Change: The services rank the class with the same film and projection weights the staffs use, from the same projection draw the signing stage reads. Consider it only if suggestion 6 leaves the gap.
8. **Give starters a small base portal lean** (Portal, eligibility and the draft)
   - Seen: Starters are 4.9% / 4.7% / 4.9% of entrants, and 4-5 star starters entered only 49 / 40 / 40 times since 1991, because a starter's base lean is 0 and events only multiply it.
   - Change: college::lean_checkpoint gives a starter a base lean of about a third of a rotation player's, so a coaching change, a missed bowl or a blue-chip signing at his spot can move him.
9. **Let a program take an entrant as its third man** (Portal, eligibility and the draft)
   - Seen: Only 13% / 13% / 13% of FBS entrants land at FBS (real at least 60%), because a program wants an entrant only for its two-deep or an open starting spot.
   - Change: recruiting::portal_day lets a program with roster room also take an entrant who would be its third man at the position.
10. **Calibrate the portal size by era** (Portal, eligibility and the draft)
   - Seen: Entries are about a tenth of the real portal (FBS about 360-400 in the 2025 cycle against 2,918 in 2022 and 4,201 in 2024-25), and the fixes cut them a further 8-14%.
   - Change: After suggestions 8 and 9, find which recruiting fix cut the steered signings, then tune the portal-era entry rate to the NCAA 2022 counts and the 2024-25 record (rates.json, trial3-baselines.md).
11. **Weigh NIL money in the declare choice** (Portal, eligibility and the draft)
   - Seen: Underclassmen declaring peak in the NIL era (72 / 75 / 72 a year in 2021-2025), the reverse of the real drop to about 58 in 2024 (from memory, to verify).
   - Change: From 2021 the declare chance weighs the NIL money a player would give up against the value of his projected round (recruiting::draft_reads).
12. **High school playoffs: one round a week with the real field size** (NFL, high school and weather)
   - Seen: Every classification sends 8 teams to 3 rounds spread over 4-7 weeks; 33 of 51 states have 2+ idle weeks between rounds. The 8-team field is open realism question 2.
   - Change: Each state plays one playoff round a week from its real start to its final, with the field size that number of rounds needs (docs/research/hs-playoff-formats.md).
13. **Set hurricane chances per state from the real record** (NFL, high school and weather)
   - Seen: College games moved or cancelled in 6 / 8 / 6 of 30 seasons, Florida only in world 2027, and almost no October storms; all 8 hurricane states share one small chance.
   - Change: calendar/weather.rs gives each state its own yearly storm chance by month from the real record of storms that hit football weekends, with Florida highest and October included.
15. **A higher bar for a former Heisman winner** (Stakes, news and postseason)
   - Seen: 2 / 1 / 2 players still win the Heisman twice (real: only Archie Griffin, from memory).
   - Change: The former-winner factor in awards() goes from 0.85 to 0.7 (proposed, not confirmed).
17. **A smaller slump skill loss for kickers, punters and snappers (Cain's call)** (World and players)
   - Seen: Specialists are 3.9-4.0% of season rows but 17% / 26% / 15% of the 2+ point drops, and a slump alone costs them 2.1-2.6 points, because their rating is nearly all skill.
   - Change: Halve the slump's skill loss (0.5 to 3.0 per skill) for K, P and LS, so a slump costs them about what it costs other players.
18. **Build volatile development, starting with major injuries** (World and players)
   - Seen: Seasons losing more than 1 point: 0.10% / 0.11% / 0.11%. The biggest drop in 90 world-years is 2.9 points, and major-injury seasons rise a median +0.6.
   - Change: Build fix list item 4 (volatile development). Its detail round settles which causes take ability away and how much, starting with major injuries.
45. **A coach fired or retiring in December coaches through his team's last game** (World and players)
   - Seen: When a champion's head coach retired or was fired in December, the successor got the national title and the playoff wins without coaching that season: 3 / 2 / 0 seasons (world 2026: 2002 Georgia, 2020 Miami after a 12-2 title season, 2021 Ohio State; world 2027: 2011 and 2024 Georgia). Coaches who left for another job kept their credit.
   - Change: The carousel still decides in December, but a fired or retiring coach whose team is still playing coaches through its last game, so the playoff wins and the title go to the coach on the sideline. Coaches leaving for another job keep today's rule.

### Creative ideas and export additions (26)

19. **Export each portal entrant's record** (Portal, eligibility and the draft)
   - Seen: The raw export has only daily entry counts, so landing days, waits, held playoff entrants and destinations could not be checked.
   - Change: Add portal.jsonl to the AFM_RAW export: entered, held until, decided, from, to, role, reason, and whether the first choice fell through.
20. **Export draft withdrawals with their day** (Portal, eligibility and the draft)
   - Seen: Withdrawals are Small events, so the title-game withdrawal day (B8) cannot be seen.
   - Change: Add a withdrawals-by-day counter, and the late declarations, to calendar_years in the raw export.
21. **Export pro seasons so the slump list covers whole careers** (World and players)
   - Seen: The season lists stop at college (0 rows at 30 or older), so no age decline or pro slump can be listed.
   - Change: Write each pro season (start, end, role, events, games missed) into the alumni and card season lists.
22. **Say why a game is loud** (Stakes, news and postseason)
   - Seen: Tags decide every game's loudness, but no event text names a tag and the export cannot show them.
   - Change: Every Big or Huge college game text names the stake reasons, for example 'Huge: both ranked, a playoff place at stake'. That also puts them in the raw export.
23. **Flag the spin-up years in the calendar export** (NFL, high school and weather)
   - Seen: The export starts in 1990 and the history in 1996. The spin-up years show the 1990 and 1993 NFL seasons a week early, the tag window in 1990-1992 before the tag existed, no combine line before 1994, few all-star invites and 'All-America teams are named: 0 players' in 1990.
   - Change: Mark the six spin-up years in the export, or leave them out, so trial checks and the dashboard read only 1996-2025.
24. **Profile the phases of one world** (World and players)
   - Seen: The A/B gives the total cost of section 4 (+6%) but not where it goes.
   - Change: Run one 5-year world on the fixed build with AFM_PROFILE=1 so the clock prints the time of each step.
25. **Warn on any season without an FBS champion** (World and players)
   - Seen: The BCS bug passed through the report and dashboard without a warning before B1.
   - Change: world_trial's report prints a warning line for every season with no FBS national champion.
26. **A sixth-win game is huge for that team** (Stakes, news and postseason)
   - Seen: 21.5 / 21.0 / 21.7 FBS teams a year go into their last regular-season game with 5 wins.
   - Change: A last regular-season game at 5 wins is huge for that side on its own, as in T8.1's 'must win' example.
27. **The coach's job as an NFL stake** (Stakes, news and postseason)
   - Seen: T8.1 names 'your job on the line', and college has Tag::HotSeat, but NFL stakes read only the playoff line and division races.
   - Change: A late-season NFL game is a stake for a team whose head coach is on the hot seat.
28. **Play rivalries only in their real years** (Stakes, news and postseason)
   - Seen: Every listed rivalry is played all 30 years: Texas-Texas A&M in its 2012-2023 pause, Iowa-Nebraska before 2011, Bedlam after it ended (from memory, to verify).
   - Change: Each listed rivalry gets its real years (start, pauses, end) and is scheduled only in those years.
29. **Real all-time series records** (Stakes, news and postseason)
   - Seen: Series lines count only simulated years ('since 1996').
   - Change: Load each listed rivalry's real record up to 1995 and show the all-time record with the simulated games added.
30. **Real trophy holders when the history starts** (Stakes, news and postseason)
   - Seen: 53 trophy games a world say the 1996 winner 'wins' the trophy, as if new.
   - Change: Load who held each trophy after the 1995 season, so the 1996 game keeps or claims it.
31. **College games on weeknights and holidays** (Stakes, news and postseason)
   - Seen: All regular-season college results fall on a Saturday (41,513 / 41,252 / 41,269).
   - Change: Schedule a real share of college games on Thursday and Friday nights, November weeknights, Labor Day Monday and Black Friday.
32. **Signing-day class rankings in the news** (Recruiting)
   - Seen: No item names the best recruiting classes in any world.
   - Change: Post one Big item on the early-period day and on February signing day naming the top three classes by the services' stars.
33. **Decommitments when a head coach is fired** (Recruiting)
   - Seen: Flips and decommitments happen only at the signing stages; none from May to November.
   - Change: On the day a head coach is fired, each of his commitments reconsiders, and some decommit before the early period.
34. **Offers that build from the sophomore year** (Recruiting)
   - Seen: First offers come on April 30 of the junior year, so junior days have no offers or visits behind them.
   - Change: Top prospects get dated offers from their sophomore year on, building up through junior days.
35. **All-American Bowl invitations announced in the fall** (Recruiting)
   - Seen: The January rosters are picked on game day, so no page shows an invitation before the game.
   - Change: Pick and post the All-American and Polynesian Bowl rosters as Small news in the senior fall, and keep the game-day note.
36. **A May graduation for graduate transfers** (Portal, eligibility and the draft)
   - Seen: Only a December graduation exists; 81% of alumni notes carry one, and real graduate transfers mostly moved after a May graduation.
   - Change: Add a May graduation date per school; a graduate with a season left then gets the graduate-transfer option for the coming season.
37. **The advisory grade before the decision** (Portal, eligibility and the draft)
   - Seen: The advisory read comes after declaring; the real committee returns grades in mid-December (from memory, to verify).
   - Change: Move the advisory grade to mid-December, so it feeds the declare plan before the plan is drawn.
38. **In-season firings with an interim coach** (Portal, eligibility and the draft)
   - Seen: No college head coach is fired before the Sunday after rivalry week.
   - Change: A program can fire its coach in October or November, with an interim coach to the end of the season.
39. **Era-true NFL team names and cities in the hidden history** (NFL, high school and weather)
   - Seen: History events name teams before they existed or had that name, for example the Washington Commanders 138 / 148 / 134 times before 2022 and the Las Vegas Raiders 133 / 170 / 116 times before 2020.
   - Change: Each history year uses the team names and home cities that year really had. Names only; the league's size by year is a separate question.
   - One yes or no for the bundle; its parts: Houston Oilers to 1996, Tennessee Oilers 1997-1998, Titans from 1999; Raiders: Los Angeles to 1994, Oakland 1995-2019, Las Vegas from 2020; Chargers: San Diego to 2016; Rams: St. Louis 1995-2015; Washington's names by year.
40. **District schedules for high school regular seasons** (NFL, high school and weather)
   - Seen: Schedules have no rematches now but pair any two schools of a class across the whole state; small states play 11-45% of games outside their class.
   - Change: Build each regular season from districts of nearby schools in one class: a round robin plus non-district games.
41. **Hurricanes hit the coast, not the whole state** (NFL, high school and weather)
   - Seen: A Gulf storm moves games in Lubbock and El Paso, because Dallas's climate stands for all of Texas.
   - Change: Apply a hurricane only to schools within a set distance of the Gulf or Atlantic coast.
42. **Let Division II HBCUs into the HBCU Legacy Bowl** (NFL, high school and weather)
   - Seen: The pool takes only SWAC and MEAC players; the 24 SIAC and CIAA programs are left out.
   - Change: Seniors from SIAC and CIAA programs join the HBCU Legacy Bowl pool.
43. **One spring step per player (Cain's call)** (World and players)
   - Seen: 3,369 / 3,294 / 3,231 of the 11,438 / 11,266 / 11,221 alumni with a spring step made two or more, and 17 / 26 / 22 made four.
   - Change: A player who already made a spring step is not a candidate again.
44. **Size high school spring steps from normal growth (Cain's call)** (World and players)
   - Seen: High school steps are a median +6.2 / +6.1 / +6.2 and 72% are 5+ points, while a whole high school season row at 15 changes a median +4.1.
   - Change: Size the spring step from his normal yearly growth (for example one extra year of growth) instead of from the gap to his hidden ceiling (development.rs spring_jump).
