# World trial 6: section 4 on three 30-year worlds

Three worlds (seeds 2026, 2027, 2028), each with 30 years of history (1996-2025), every college game on the match engine (about 83,100 games and 5.9 million team plays a world) and, new in this trial, the section 4 daily clock: every event on its real day, by era. Each world took about 41 minutes on 4 cores. Run 2026-10-16 on `main` after the section 4 build (PR #30) and its first fix round (PR #31).

Where everything is:
- The AFM Debug Lab, Verdict tab: positives, negatives and bugs, suggestions with yes / no / later buttons, what was fixed, spot checks and doubts: https://claude.ai/artifact/3SAzca1V9NBFKkW5pAPTSt
- The full player pages (309 players, 103 per world): https://claude.ai/artifact/QPTC1ZCGUmP9hfLDU2J7ZY (copy in `artifacts/world-trial6-player-pages.html`).
- Raw output in the project folder at `afm/trials-section4/seed-<n>/` (report, dashboard data, cards, gzipped raw export); the dashboard page and data at `afm/trials-section4/dashboard/`.
- The yes / no list: `world-trial6-proposals.md`. Every point of section 4 with its code and Claude's own picks: `docs/design/calendar-checklist.md`.

How it ran. The three worlds ran once on the build. Five checkers (world and players, recruiting, portal and eligibility, NFL and high school with weather, stakes and news) read the raw output against the spec and real football. Their bugs were fixed (PR #31), the three worlds ran again, and the checkers measured the new runs against the old ones. A skeptic pass then re-measured the 12 numbers the decisions rest on; 3 needed a correction, applied below. After the runs, two more bugs were fixed: stat lines booked on the wrong team and a dashboard page that did not load.

Each result says why. "Traced" means read in the code; "inferred" means read from the numbers. Everything proposed here is proposed, not confirmed.

## Headline

- The calendar works: signing days, the early period, transfer windows, the draft, the Super Bowl and the NFL dated steps land on their real days by era, and each recruit commits on his own day.
- The game shape held: FBS teams score 28.9-29.1 a game (real 27.8), 22.5-22.9% of games are decided by 28+ (real 21.1%), home teams win 56% (real 58%).
- 32 bugs from the first runs are fixed and measured: stars are exactly 33 / 350 / 2,000 / 4,000 a class, 4-stars are drafted 24-25% of the time (real 24.4%), every season has a champion, about 1,380 true freshmen play a year (was 69), and committed recruits who sign nowhere fell from 46% to 1.1%.
- The fixes caused new problems. The biggest: from 2018 nearly every freshman is held to four games (5+ games fell from about 870 a year to about 135), and a camp redshirt who plays is still developed as a redshirt. 17 bugs (proposals part A) were then fixed and merged (PR #33, Cain's yes on 2026-10-16). They are not measured here: the next trial measures them.
- 38 suggestions wait for a yes or no (proposals part B), 26 of them creative ideas.
- Big slumps are rare and every one has a cause. Specialists slump too hard and major injuries cost too little (see Slumps).

## Fixed in this build (34)

Bugs the first three worlds showed, fixed and measured again: before the fix, then after it in each world (2026 / 2027 / 2028).

### A1. Star ratings no longer pile up over the rating passes
- Before: Stars per class (median, world 2026): 5-star 35, 4-star 619, 3-star 5,594, 2-star 14,140 (targets 33 / 350 / 2,000 / 4,000). Final 4-stars were drafted 14.3% of the time and 3-stars 1.8%. 206 4-stars a class at regular positions never played college.
- After: Exactly 33 / 350 / 2,000 / 4,000 in every class of all three worlds. Draft rate by final stars, classes 1997-2019: 4-stars 25.0% / 24.8% / 24.4% (real 24.4%), first round 4.2% / 4.4% / 4.0% (real 3.4%); 3-stars 4.5% / 4.5% / 4.5% (real 6.0%); 2-stars 0.9% (real 2.1%). The dashboard's own count gives 25.1% / 25.0% / 24.6% for 4-stars. 4-stars who never played college fell to 43.8 / 44.3 / 41.0 a class (see negatives). Real: tools dashboard real.json draft_by_star (ESPN-rated signees, classes 2010-2021).
- Why: Traced: run_signing_stage gives every senior in the pool new stars at each pass (service_stars), so anyone outside the quotas drops to 0 and a kid holds only his latest rating. The model's denominators count every rated recruit, signee or not, so its 2- and 3-star rates read a little low.

### A2. The snapping camps convert one batch a class
- Before: Each rating pass converted a new batch. World 2026 alumni notes said 'the snapping camps ranked him a 5-star long snapper' 3,857 times and a 4-star 11,592 times, and 113 5-star and 336 4-star converted snappers a class never played college.
- After: One batch a class: 25 5-star and 75 4-star camp snappers, by rule. World 2026 alumni notes: 138 5-star and 260 4-star snapper notes. In college (world 2026) there are 5 / 4 / 11 / 16 5-star camp snappers for classes 2022-2025, never more than 25. How many of them sign is a negative.
- Why: Traced: camp_rankings converts snappers only when find_snappers is set, at SignStage::Summer(0), and snappers keep their camp stars after that.

### A3. Camp-rated kickers, punters and snappers no longer count as blue-chips
- Before: Per class 175 5-star and 527 4-star K, P or LS. They made 1,194 of 1,781 5-star commitment items, 1,584 of 2,613 5-star signing items, and 34 of 75 / 55 of 90 / 51 of 105 4-5 star starter portal-entry items. 1,899 of the 1,999 All-American Bowl alumni of classes 2001-2020 were long snappers, and about 5 a class were drafted (world 2026). All 8 bust cards were specialists.
- After: No K, P or LS holds 4 or 5 services' stars in any world: 0 / 0 / 0 5-star commitment or signing items, and 0 of the 49 / 40 / 40 4-5 star starter portal entries. About 22 of the 33 5-stars a class play in the All-American Bowl. Of its 100 players a class (2001-2020), about 42 / 44 / 43 are archived, 28.5 / 29.4 / 28.0 are drafted and 13.0 / 12.3 / 11.9 go in round 1. 0 of the 8 bust cards are specialists. Signing notes read 'as a 3-star P (a 5-star by the specialist camps)' (1,444 / 1,453 / 1,400 notes).
- Why: Traced: camp stars sit in their own field (camp_stars), and the services cap specialists at 3 stars without using a quota place (SERVICE_SPECIALIST_MAX). The All-American Bowl, the news, the cards and the portal news read the services' stars.

### A4. Verbal commitments hold through signing day
- Before: Committed recruits who signed nowhere: 46.5% / 46.4% / 46.2% of verbal commitments (logs 1996-2016) and 45.8% / 46.2% / 45.6% (2018-2025). Flips: 21.6% / 21.6% / 21.8%. 20.6% of blue-chip flips went down a level, there were 4,139 blue-chip flip news items, and 1,224 committed blue-chips never played college (world 2026).
- After: Signed nowhere: 1.1% / 1.1% / 1.1% (1996-2016) and 1.2% / 1.3% / 1.2% (2018-2025). Flips: 6.9% / 7.1% / 7.0% and 9.1% / 9.3% / 8.9%. Blue-chip flips down a level: 0.2% / 0.3% / 0.5%. Blue-chip flip news over 30 years: 1,381 / 1,452 / 1,437, and only 5.1% / 2.8% / 2.9% go from FBS to a lower-prestige FBS school (pre-fix 14.2%). Committed blue-chips who never played college: 17 / 18 / 20 (classes 1997-2021). The fallback part is under not fixed.
- Why: Traced: run_signing_stage gives a staff's own commitments 5 film SD at every stage and keeps committed kids in view. A commitment now falls through only when his position or the class runs out of room, and a flip happens when the recruit himself prefers another offer.

### A5. An unsigned committed recruit's note no longer says no program offered him
- Before: 1,199 of the 1,224 committed blue-chips who never reached college had the note 'No program offered him...' right after 'Committed to <school>' (world 2026).
- After: 0 / 0 / 0. Archived players now read 'His commitment to <school> fell through, and no other program had room for him' (3 / 2 / 3) or the prep-school version (6 / 9 / 6).
- Why: Traced: the None branch of run_signing_stage picks the fell-through text when the recruit was committed. 'No program offered him' now also needs an empty offer list at the stage before (cal.offers).

### A6. Signing counters land in the class's own year
- Before: Log years 1990, 2006, 2012 and 2017 held no February signing day (0 decommits, 0 late signees, and a 100% FBS early share on the dashboard). Years 2004, 2009, 2015 and 2020 held two (5,903-6,592 decommits).
- After: Every log year holds exactly one February signing, and no year has 0 late signees. Decommits run 32-140 / 31-150 / 39-132 a year. FBS signees average 2,941 / 2,944 / 2,946 a year (1996-2025). The FBS class-size swing from the counters fell from 12.0% / 11.9% / 11.2% to 7.2% / 7.2% / 7.6%. Only the class of 2025 stays about double, and that is real signing (see negatives).
- Why: Traced: signing_stage swaps in the log of the class's season before it runs the stage, then swaps back. The news items are not swapped (see negatives).

### A7. Official visits follow the NCAA calendar by era
- Before: Official visits: 19,614 / 19,622 / 19,684 a year before 2018 and 17,518 / 17,466 / 17,674 from 2018, so the 2018 junior-visit rule added nothing. Juniors took May and June official visits in every era, and no official visit could happen from Sept 1 to Nov 29.
- After: 19,526 / 19,616 / 19,480 before 2018 and 22,366 / 22,296 / 22,405 from 2018 (+15%). Per FBS+FCS signee: 3.55 / 3.54 / 3.55 before and 3.82 / 3.81 / 3.80 after. The pre-2018 total held without the junior visits, so senior-fall visits replaced them. Unofficial visits (medians): 17,498 / 17,618 / 17,354 and 14,362 / 14,839 / 14,536 a year.
- Why: Traced: official_visit_allowed lets only the next class visit, from Sept 1 of the senior year in every era, and from 2018 also from April 1 to June 30 of the junior year. recruiting_day does nothing on a day whose period forbids the action. Visit dates are Small events and not exported, so the windows are traced, not measured.

### A8. The 2023 early signing period opens on the real day
- Before: Wed Dec 6, 2023.
- After: Wed Dec 20, 2023 in all three worlds. Every early period now opens on its real day: Dec 20 2017, Dec 19 2018, Dec 18 2019, Dec 16 2020, Dec 15 2021, Dec 21 2022, Dec 20 2023, Dec 4 2024, Dec 3 2025.
- Why: Traced: anchors.rs picks the early-signing rule by the season (third Wednesday through 2023, first Wednesday from 2024). The change was adopted in March 2024 (docs/research/rules-and-money-2026.md).

### A9. Commitment notes give the senior-season age
- Before: World 2026: 6,166 of 6,184 'during his senior season' notes and 500 of 5,776 'before his senior season' notes gave an age one year above the senior row. 32 of 103 cards read 'Committed to ... at 18, during his senior season.' then 'Named first-team all-state ... at 17.'
- After: 0 of 3,953 / 4,032 / 3,965 'during' notes and 0 of 2,962 / 2,898 / 2,930 'before' notes run a year high, and no card has the commitment-then-all-state pair. Other note ages are under not fixed (D5).
- Why: Traced: announce_commitments uses age - 1 once the state's high school season has developed.

### A10. College all-star games fill every year, and high school all-stars can play in them
- Before: College all-star invites dipped in many years (for example 211 in 2002, and 220 of 410 in 2022 and 2023). In world 2026 the Shrine Bowl averaged 1 drafted-or-signed player in the 12 years whose invite day fell before Jan 15. 0 of 6,150 high school all-stars ever played in a Senior, Shrine, NFLPA or HBCU game.
- After: The invite counter steps up by era with no dips: 220 to 1999, 320 to 2010, 420 to 2015, 520 to 2020 and 610 from 2021, every game full. Drafted-or-signed Shrine Bowl players: 73 / 74 / 70 a year in the early-invite years. 716 of 1,005 / 729 of 1,031 / 681 of 995 archived high school all-stars later played a college all-star game, and 297 / 276 / 260 of 960 first-round picks were high school all-stars.
- Why: Traced: recruiting::all_star_invites takes seniors whose college season is over and who are out of eligibility, without waiting for the Jan 15 deadline. The high school game sets its own flag (PersonCal.hs_all_star), so it no longer blocks the college invite.

### B1. Every season has a national champion, the BCS years included
- Before: 16 / 16 / 16 of 30 seasons (1998-2013) had no FBS champion. The BCS title game was played, but nobody got the title.
- After: A champion in 30 of 30 seasons in each world, with a title news line every season. A head coach is credited every season, but in 3 / 2 / 0 seasons the credit goes to the successor hired in December, not the coach who won it, because that coach retired or was fired before the January title game. The coach who led the champion is credited in 27 / 28 / 30 seasons. Example: world 2026, 2020 Miami: MJ Wood was fired after a 12-2 title season and Charles Allen got the title. No season where a title game's winner differs from the champion. The dashboard's champions list is complete. How few programs win is a negative.
- Why: Traced: world/college.rs crown_title_game runs after every stored game. A title game outside any bracket now sets the champion and the program's title flag. finish() then credits the program, the coach and the news as before. Inferred from the spot check: the coach credited is whoever holds the job in January, so a December change hands the title and the playoff wins to the successor.

### B2. The champion is first in the final poll
- Before: The final poll's No. 1 was not the champion in 5 / 4 / 2 playoff-era seasons (world 2026: 2014, 2015, 2016, 2023 and 2024).
- After: 0 / 0 / 0 seasons out of 28 title-game seasons. No team shows up twice in any poll's top 4.
- Why: Traced: release_poll moves the champion to first in the final poll when a title game decided it (not in the 1996-97 polls-only years). Only first place is set; see the negative on the title-game loser.

### B3. Playoff teams' portal entrants wait until their season is over
- Before: In the 2023-2025 cycles, entries dated after the title game were 0 / 0 / 0. Title teams' entrants entered after the semifinal and played the title game while in the portal. In the 2023 cycle, 2 / 6 / 2 entries after the window closed did not line up with any playoff team's last game. In 2024 the four bye teams' entrants sat out the playoff (traced).
- After: Entries dated after the title game: 2 / 1 / 6 (2023 cycle), 4 / 3 / 2 (2024) and 3 / 8 / 1 (2025). In all 12 world-cycles (2022-2025 in three worlds), every entry after a window closed came 1-5 days after a playoff team's last game. No 4-5 star starter's entry news came before a playoff game his team then played (0 of 49 / 40 / 40). Script: held_check.py.
- Why: Traced: recruiting::portal_open now holds the entrants of any team still alive. college::playoff_alive reads unplayed playoff and title games and the undecided bracket slots, so bye teams count too. release_held then enters each held player 1-5 days after his team's last postseason game, or on his window day if that is later.

### B5. The fall-term grade check runs
- Before: fall_ineligible was 0 every year. No player was ruled out of a bowl or spring ball after the fall term.
- After: 57-125 players a year are ruled out after the fall term (yearly average 73 / 72 / 71 for 1991-2005 and 104 / 102 / 111 for 2022-2025). Each note names the term and the GPA, for example 'Academically ineligible for the postseason and spring practice at 20 after the fall term (GPA 1.84).' The spring check is unchanged: 15-86 a year.
- Why: Traced: college::fall_term_check runs at the start of december_graduation. It puts players on teams still playing on the out list (cs.out), and hs_year::college_spring_candidates skips them for spring ball. spring_term_check clears the flag.

### B6. The December graduation note matches his eligibility
- Before: In 4,693 / 4,775 / 4,764 cases the note said 'with a year of eligibility to use anywhere', and then his college career ended that same January.
- After: 0 / 0 / 0. The 'year to use anywhere' wording now appears on 5,760 / 5,694 / 5,866 of the 21,478 / 21,312 / 21,337 December graduates in the notes. The others read 'Graduated in December at 22.' Script: note_checks2.py.
- Why: Traced: college::december_graduation now checks college::last_season, which counts the season in play, the four- or five-season limit and the five-year clock, before it writes the longer wording.

### B7. Only players headed to the draft opt out
- Before: 46 / 45 / 47 bowl opt-outs 'to prepare for the draft' did not enter that draft. That is 19% of the 237 opt-outs in world 2026. Example: Rodney Bloomfield (Nebraska).
- After: By the same check: 0 / 1 / 0. The one case is Vincent Kohake (Florida, world 2027): he opted out at 22 and entered the draft pool at 23 the same winter, because his birthday fell in between. 102 / 96 / 120 of the 168 / 166 / 186 opt-out notes are underclassmen who 'will enter the NFL draft'; each has a declaration at the same age or one year older, and none withdrew.
- Why: Traced: college::opt_outs now allows only a senior in his last season (last_season) or a player whose declare_plan is yes. recruiting::declarations makes an opted-out underclassman declare, even while his team is still playing, and skips him on withdrawal day.

### B9. True freshmen play again, under each era's redshirt rule
- Before: FBS true freshmen who got into a game: 69 / 66 / 69 a year in 1997-2017 and 74 in 2018-2025 (trial 5: 678 and 658). 3.3% of FBS players first played as true freshmen (trial 5: 32.7%). No note said a redshirt year was burned or kept, and the four-game counter was 409-681 a year from 2019.
- After: 1,384 / 1,379 / 1,389 a year in 1997-2017 and 1,342.5 / 1,373.5 / 1,338 in 2018-2025 (medians), about 46-47% of FBS signees. 59.3% / 59.4% / 59.4% of FBS players first play as true freshmen. Burned-redshirt notes come only before 2018 (0-5 a season after), and about 2,400 players a world carry 'kept his redshirt year under the four-game rule', all from 2018. The four-game counter is 0 before 2018, 10,992 / 11,173 / 11,175 in 2018 (a backlog) and 5,045-6,632 a year after.
- Why: Traced: world/college.rs start no longer keeps camp redshirts out of games. settle_eligibility (January 2) keeps the year for 4 games or fewer from 2018 and burns it otherwise, and the review fix (bda2c42) takes a camp redshirt off the game roster at four games from 2018. Three side effects are negatives: the redshirt row, the four-game stop and the pre-2018 burns.

### B10. Quarterbacks now lead the Heisman race
- Before: Winners: RB 28 / 30 / 28, QB 2 / 0 / 2. Running backs were 101 / 107 / 105 of 120 finalists.
- After: Winners: QB 23 / 18 / 21, RB 5 / 3 / 7, WR 1 / 8 / 2, TE 1 / 1 / 0. QBs are 88 / 74 / 89 of 120 finalists. Real 1996-2025: QB 22, RB 5, WR 2, DB 1 (from memory, to verify). World 2027's 8 receiver winners are high.
- Why: Traced: heisman_score was replaced by production() and heisman_kind() with priors QB 1.0, RB 0.84, WR/TE 0.78, defense 0.75 (Claude's picks, proposed, not confirmed). Repeat winners are under not fixed, and same-school finalists rose (see negatives).

### B11. Rivalry results carry the series record
- Before: 0 rivalry texts carried a series record (77 rivalry results a year).
- After: 2,259 / 2,251 / 2,258 results between listed rivals carry it, which is every meeting after the first. In 2,182 / 2,174 / 2,181 meeting-to-meeting steps, the record always adds exactly the winner's win (0 errors). Every record reads 'since 1996'. Example: 'Iowa beat Iowa State 35-28 in the Cy-Hawk game to keep the Cy-Hawk Trophy. Iowa leads the series 15-12 since 1996.'
- Why: Traced: record_series counts every meeting while the history records, and result_event adds the sentence from the second meeting. Conference title and playoff meetings count too, as in real all-time records. Real records before 1996 are not loaded (see suggestions).

### B12. Rivalry and playoff wording
- Before: 'in the Rivalry' for Oregon-Oregon State 30 times. 'to claim' was written for the team that already held the trophy in 961 of 1,590 trophy games (world 2026). 8 CFP first-round and 482 lower-division playoff results did not say which round. 'Selection day: 0 bowls filled.' in the spin-up years.
- After: 'in the Rivalry': 0 / 0 / 0. The game is 'the Civil War' 24 times through 2019 and 'the Oregon-Oregon State game' 6 times from 2020 in each world. The verb matches who held the trophy in 1,590 of 1,590 trophy games in each world ('to keep' 937 / 954 / 993, 'to claim' 600 / 583 / 544, 'to win' at the first meeting 53). Results with no round named: 0 / 0 / 0 (for example 'in the Rose Bowl, a CFP semifinal', 'in the FCS quarterfinals', 'for the D2 title'). No 'Selection day: 0 bowls' line.
- Why: Traced: Rivalry::name_in in calendar/data.rs, the holder check and playoff_round in world/college.rs result_event, and selection_day posting only when there are bowls or brackets. The Civil War cut-over year is from memory, to verify.

### C1. A Sunday Christmas moves the slate to Christmas Eve
- Before: World 2026 exported 8 (1994), 9 (2005), 6 (2011), 6 (2016) and 6 (2022) stakes games on Christmas Day afternoons, and 0 or 1 on Saturday Dec 24.
- After: Christmas Day afternoons now hold 1 / 0 / 1 (1994), 1 / 1 / 1 (2005), 0 / 0 / 0 (2011), 1 / 1 / 0 (2016) and 1 / 2 / 2 (2022) exported games; Dec 24 afternoons hold 6 / 9 / 10, 10 / 6 / 9, 9 / 11 / 9, 5 / 8 / 7 and 8 / 8 / 8. The rule keeps 2, 2, 1, 2 and 3 games on Christmas Day, and no world exports more than that. Real 2022: 11 games on Dec 24 and 3 on Dec 25 (from memory, to verify).
- Why: Traced: nfl_year::formula_slots keeps only the Christmas windows on Dec 25 and moves the other Sunday games to Saturday Dec 24. The exported games are only the Big and Huge stakes games, so the counts are a sample.

### C2. NFL dated steps on their real days
- Before: Trade deadline in the real week in 24 of 36 seasons (a week late in 2012-2023). 'The NFL season kicks off.' dated the day before the opener in 32 of 36. On the 2017 Monday Christmas two games kicked off at 8:15pm. Traced: Pro Bowl Games on the Tuesday before the Super Bowl every year, Shrine Bowl always on a Tuesday.
- After: Trade deadline in the real week in 36 of 36 seasons in all three worlds. The kickoff text is on the opener's own day (8:20pm Thursday from 2002, 1pm Sunday before) in 32 of 36, and the day before in 0; the 4 others are the opener-date misses listed in the positives. Night kickoff clashes: 0 / 0 / 0. Rebuilt from the exported Super Bowl dates with the new formulas (these are Small events the export leaves out): the Pro Bowl is on the Sunday after the Super Bowl to the 2008 season, the Sunday before it in 2009-2024, none for 2020 and Tuesday Feb 3, 2026; the Shrine Bowl is on a Saturday (Super Bowl minus 15 days) to January 2021 and a Thursday (minus 10) from February 2022. All 8 Pro Bowl and 8 Shrine Bowl dates checked match the real ones (from memory, to verify).
- Why: Traced: anchors.rs trade_deadline_week (6 to 2011, 8 in 2012-2023, 9 from 2024), pro_bowl and shrine_bowl; nfl_year::day posts the kickoff on the opener's day; formula_slots skips the separate Monday night game on a Monday Christmas.

### C3. NFL era details: negotiating window, cap, practice squads, games abroad
- Before: The two-day negotiating window was posted 23 times before 2013. Practice squads had 8 spots in 1990-2003. The 1990-1993 spin-up years showed a $26.4-32.3 million cap. Games abroad matched the real count in 29 of 36 seasons (Toronto 2008-2013 and the second 2021 London game missing).
- After: In all three worlds the window first opens Mar 10, 2013 ('three-day' to 2015, 'two-day' from 2016). Practice squads: 5 spots in 1990-2003, 8 in 2004-2013, 10 in 2014-2019, 16 in 2020-2024 and 17 in 2025 (2025 left unverified by the fixer). No cap text before 1994, and 'no salary cap' in 2010. Games abroad match real.json's abroad_by_year in 36 / 36 / 34 of 36 seasons, with 2 in 2021 everywhere. The Toronto part fails twice in world 2028 (under not fixed).
- Why: Traced: anchors.rs negotiating_window, rules.rs RuleBook::era (no cap before 1994) and ChangeState::rules (floors only after a vote), nfl_year::international_sites (2 London games in 2021, one Bills home game in Toronto in 2008-2013).

### C4. Early-season flexes capped at two
- Before: Up to 6 flexes in weeks 5-10 in a season (2015, 2018); 8.8 flexes a season in 2014-2025 (world 2026).
- After: At most 2 flexes in weeks 5-10 in every season 2014-2025 in all three worlds. Total flexes 2014-2025: 6.8 / 6.5 / 7.2 a season. Later weeks are still too busy (see negatives).
- Why: Traced: nfl_year::flex counts early_flexes and refuses a third in weeks 5-10.

### C5. Continue stops on Big moments
- Before: Continue stopped only on Huge moments: 25.4 / 26.8 / 26.5 stop days a year. The longest run without a stop was 115-131 days (median 124), from the draft to cutdown day. It skipped free agency opening, the tag deadline, Black Monday, NFL kickoff and NFL Honors.
- After: 50.3 / 50.1 / 50.1 stop days a year (NFL 26.3 / 26.4 / 26.4). The longest run is 78-84 days (median 80), from the draft to the mid-July deadline for long-term deals with tagged players. Free agency opening now stops it in 29 of 30 years (none in the 2011 lockout); the tag deadline, Black Monday, NFL kickoff and NFL Honors in 30 of 30. Holiday mode still stops on Huge only: 26.8 / 26.9 / 27.1 days a year.
- Why: Traced: world/clock.rs continue_until_needed now stops on an event that needs a decision or is Big or louder; holiday_to still checks Huge only. Measured as the days with a Big or Huge event, because the export cannot replay Continue. Side effect: see the negative on other teams' Big moments.

### C6. No design codes or zero combine lines in texts
- Before: '...holding in (A4, A5).' in the veterans-report text in 36 of 36 years; ', 0 abroad' in schedule releases; 'The NFL invites 0 prospects to the scouting combine.' in 1990-1993.
- After: 0 / 0 / 0 exported texts with a design code, no ', 0 abroad', and no zero combine line. The exported rule labels carry no codes either (Small events, and their labels, are not exported). One zero line is left (under not fixed).
- Why: Traced: codes removed from the texts in world/nfl_year.rs, build_schedule leaves out ', 0 abroad', and recruiting.rs combine_invites posts only when the count is above 0.

### D1. High school teams never meet the same opponent twice
- Before: About 56% of teams met some opponent twice in a regular season (replay of the old weekly reshuffle), 99-100% in SD, WY, DC and VT.
- After: Traced: hs_year::state_schedule never pairs two schools that have met. A Python replay of the new rule on the real program counts gives 0 repeated pairs in 51 of 51 states and 0.15% bye weeks (most in AK 5.6% and DC 4.8%). Games outside the classification are rare except in small states (DC 45%, WY 16%, SD 11%). The world's high school game counter rose from 67,458 / 67,465 / 67,462 to 68,119 / 68,123 / 68,122 a year, about the +660 the replay predicts from fewer byes and Florida's extra classes.
- Why: Traced and replayed (hs_schedule_check.py). High school games are not exported, so no game list could be checked directly.

### D2. Florida plays in 7 classifications
- Before: 4 classes of about 139 schools, 4 state champions a year.
- After: 7 classes of about 80 schools and 7 state champions a year (one 8-team bracket per class). The game counter rise above includes it.
- Why: Traced: world/data.rs leading_number reads '7 (plus Independent League and FIT)' as 7. DC keeps the default 4, because the data has no DC value.

### D3. Every spring step now comes after a notice
- Before: Notices equalled steps (173,979 and 173,953 in world 2026); only 44.2% of steps had a notice first; 7,447 of 18,106 alumni steps read 'out of nowhere'.
- After: Notices 350,032 / 350,049 / 349,755; steps 175,003 / 175,020 / 174,862; fizzles 175,029 / 175,029 / 174,893, a fizzle share of exactly 0.500 (design: about half). Every step had a notice first; no 'out of nowhere' notes.
- Why: Traced: hs_year.rs jumps considers only players with a spring notice; the better half by the landing read step and the rest fizzle. Notices go to about 6%, so about 3% still step.

### D4. Spring steps land on the season row
- Before: After a step, the next season row started a median +2.9 above the previous row's end (90th percentile +7.8).
- After: After a step: median +0.00 and 90th percentile +0.00 (12,289 / 12,302 / 12,240 row pairs). Every row with a spring gain carries the springBreakout event (17,460 / 17,383 / 17,294 rows, 0 without).
- Why: Traced: development.rs record_spring_step adds the gain to the latest row's end with the event; raw.rs exports it as 'spring'. Two side effects are in the negatives.

### D6. The high school January all-star games start in their real years
- Before: The All-American Bowl was played 4 times before 2001 and the Polynesian Bowl 20 times before 2017 (both every January from 1997).
- After: The first All-American Bowl is Jan 13, 2001 (class of 2001) and the first Polynesian Bowl Jan 21, 2017 (class of 2017), with no earlier players in any world. The invite counter adds 100 from the 2000 season and 100 more from 2016. Real first games: Dec 30, 2000 and January 2017 (from memory, to verify).
- Why: Traced: hs_year::all_star_game returns early before all_star_first_season (the 2000 and 2016 seasons).

### D7. The slump list names the first-year dip
- Before: 43% of the 200 biggest form slumps fell in a first year at a new level, and each read only 'a form slump'.
- After: The firstYear event sits on 34 / 31 / 29% of the 200 biggest form slumps and on 922 / 947 / 945 of the form slumps of 5+. 0 rows have a dip without the event. The dashboard's slump rows read 'first year at this level (dip 0.64)'.
- Why: Traced: development.rs develop_season adds FirstYear whenever the dip (ctx.adjustment) is above 0, without a draw; build_data.py names it.

### After the runs. Season rows no longer name the opponent
- Before: 1,633 season rows (2%) in world 2026 named the other team's program, and that game counted twice.
- After: Fixed after these runs (not in these worlds): on interception returns the defense blocks and the offense tackles, and those lines were booked on the wrong team. A new test plays 120 games and checks every line sits on its own team. Game results do not change.
- Why: Traced: game/stats.rs GameStats::book put every blocker on the offense and every forced fumble on the defense.

### After the runs. The debug dashboard loads, and two of its counts are right
- Before: A missing bracket in the NFL calendar chart stopped the whole page from loading. Years with no games abroad had no point. Continue stop days counted only Huge moments.
- After: Fixed in the page and the data builder before this publish: the page loads, 2006 and 2020 show 0 games abroad, and stop days count Big and Huge moments, as built.
- Why: Traced: tools/dashboard/index.html and build_data.py.

## Not fixed or only partly (7)

### D5. Note ages still disagree, now on most player pages
- Seen: The part D5 changed works: state all-star game notes no longer run a year high (pre-fix 10,105, now 0), and 998 of 1,005 January bowl notes give 17, the senior row's age (world 2026). But cards with a note age going backwards rose from 45 (pre-fix, world 2026) to 90 / 95 / 89 of 103. The main new pair: 11,594 / 11,530 / 11,685 alumni pages (classes 1997-2024) read 'Signed with Texas (FBS) at 18' and then 'Played in his state's high school all-star game at 17' (12,133 / 12,084 / 12,199 with the 'Walked on at ... at 18' pages); after a December signing the January bowls do the same (All-American 124 / 112 / 104, Polynesian 53 / 51 / 58). Other pairs on cards: early enrollment at 18 then a spring notice at 17 (21 / 23 / 26), a redshirt note at 19 then a spring notice at 18 (31 / 22 / 26). Old pairs left: signing at 18 then all-state at 17 (10 / 8 / 11), NIL loss at 19 then portal entry at 18 (5 / 12 / 4).
- Why: Traced: D5 moved the spring, state all-star and January bowl notes to the age on the season row (hs_year.rs row_age, age - 1). The signing and early-enrollment notes (world/recruiting.rs, left on the raised age on purpose by A9), the redshirt notes (college.rs settle_eligibility on January 2), the Heisman note and the NIL-loss note still print the age raised by the yearly step. Two age rules now sit side by side.
- Fix: Every note about a season and its offseason uses the age on that season's row: signing, flip, early enrollment, redshirt, award, NIL and portal notes too.

### A4 (fallback). The signing-day fallback for dropped commitments places almost no one
- Seen: 0 notes reading 'fell through on signing day' and 0 news items reading '<name> signs with X after his commitment to Y fell through' in any export of the three worlds. Meanwhile 1,571 / 1,553 / 1,541 committed recruits still signed nowhere in logs 1996-2016 (about 75 a year). The fix's main part, the staff bonus, did the work (see A4 under fixed).
- Why: Inferred: the fallback runs after the matching, when the programs that list the recruit have used their class slots and their walk-on places at his position. If it had placed even 100 recruits a year, about 2% of them would later turn pro and show up in alumni; none do. The remaining number is small (1% of verbal commitments).
- Fix: Proposed, not confirmed: in the fallback, let a dropped recruit take a walk-on place at any listed program with roster room at his position, even when the plan's walk-on places are used.

### B4. The redshirt trick still almost never happens
- Seen: Redshirt tricks in 2018-2025: 18 / 13 / 18 in total. Of these, 11 / 7 / 13 came in 2018, and 0-4 a year after that. Before the fix the count was 0. The fixer estimated about 35-45 a year in FBS.
- Why: Traced: college::redshirt_trick needs a veteran with two or more counted seasons who has not used his redshirt. season::depth_chart makes 94% of first-year players camp redshirts (9,781 / 9,822 / 9,847 in the final rosters), so almost nobody has a year left by year three. Only 14 / 8 / 5 of about 5,500 third-year backups and rotation players still have one, and no fourth-year does. 2018 had a backlog from the years before the rule. Script: redshirt_stock.py.
- Fix: Fix the camp redshirt share first (suggestion 1), so a freshman who makes the travel squad plays and does not redshirt. The pool of veterans with a year to save then grows by itself. The 0.3 bar can stay.

### B10 (repeat winners). A few players still win the Heisman twice
- Seen: 2 / 1 / 2 players won it twice and hold 4 / 2 / 4 of the 30 trophies (before: 5 / 3 / 5 players with 12 / 6 / 10). Examples: Donald Sutton (QB, Ohio State) 2005-06 and Steven Edwards (QB, Georgia) 2012-13 in world 2026. Real: only Archie Griffin (1974-75) ever won it twice (from memory, to verify).
- Why: Traced: awards() multiplies a former winner's score by 0.85 (Claude's pick). A quarterback on a top team who leads his group by a wide margin still clears that bar.
- Fix: Raise the bar for a former winner, for example from 0.85 to 0.7 (proposed, not confirmed).

### C3 (Toronto). The Bills' Toronto games (one part of C3)
- Seen: Worlds 2026 and 2027 have all 6 Toronto seasons right. World 2028 has 1 game abroad in 2011 and 2 in 2013 (real 2 and 3), so its Toronto game is missing in 2 of 6 seasons, 2 of 18 across the worlds.
- Why: Traced: nfl_year::formula_slots picks the Toronto week first (the Bills home week nearest week 13), then needs a Bills home game whose opponent has not gone abroad. London games come earlier (from week 4) and can already have used that opponent, and then the slot is silently skipped. Inferred: about 1 chance in 15 with one London game and 1 in 8 with two, which fits 2 misses in 18.
- Fix: Choose the Toronto game among Bills home games whose opponent has not gone abroad, or place it before the London games.

### C6 (All-America zero line). One zero-count line is left in a spin-up year
- Seen: 'All-America teams are named: 0 players honored nationally.' is posted in 1990 in all three worlds. The selection-day and combine zero lines are gone.
- Why: Traced: world/recruiting.rs posts the All-America count with no check for zero. B12 only guarded the selection-day line and C6 the combine line.
- Fix: Post the All-America line only when at least one player is honored.

### B8. The title-game withdrawal day cannot be seen in the data (traced as working)
- Seen: Withdrawals run 0-9 a year. Late declarers from the title teams: 3 / 6 / 5 (2024 season) and 7 / 3 / 2 (2025). No late declarer withdrew in the notes.
- Why: Traced: clock::advance_day runs a second withdrawal step on title game + 7 when the late declare day falls after January 15. recruiting::declarations keys its draws by player, so earlier choices come out the same. The data cannot confirm it: withdrawals are Small events, which the export leaves out, and the counter is yearly. Late declarers are mostly first-rounders, whose chance to withdraw is near zero (an advisory read of round + 1.5 x noise must reach 5, then 30%), so no case is expected in two seasons. Script: late_withdraw.py.
- Fix: No code change. Export the withdrawal step with its day (see suggestions) to check it.

## Positives (31)

### World and players

**Game shape stays close to real football.** FBS per team game, 2026 / 2027 / 2028: points 28.9 / 29.0 / 29.1 (pre-fix 28.9; trial 5 36.1 to 36.2; real 27.8), games decided by 28+ 22.5 / 22.9 / 22.8% (pre-fix 21.7%; real 21.1%), field goal tries 1.54 / 1.53 / 1.52 (1.56; real 1.56), home wins 56.4 / 56.8 / 56.1% (56.1%; real 58.4%), plays 70.6 / 70.5 / 70.8 (70.8; real 66.6). Real figures: tools dashboard real.json, vitals.FBS.
Why: Inferred: the fixes did not touch the engine; B9 put redshirts back on game rosters, which moved these numbers by less than a point. Real figures: tools dashboard real.json, vitals.FBS.

**Runaway margins and hopeless programs stay gone.** Best FBS team's average margin, mean of 30 seasons: 32.4 / 32.9 / 32.0 (pre-fix 30.6 / 29.5 / 29.2; trial 5 52.0 to 57.3; real +22 to +39, docs/trials/dynasty-proof.md). FBS programs with no winning season in 2011-2025: 2 / 6 / 5 (pre-fix 2 / 4 / 3; trial 5 10 to 13; real 0, real.json), a little above real. Win share carried into the next season: 0.526 / 0.538 / 0.561 (pre-fix 0.511 / 0.489 / 0.505; real 0.536). Unbeaten FBS teams a year: 0.80 / 1.00 / 1.03 (real 0.6).
Why: Inferred: these come from PR #28 (built before section 4), and the fixes left them within noise. Real: docs/trials/dynasty-proof.md and real.json.

**Every FBS roster has a kicker.** Going into 2026: FBS teams with no kicker 1 / 0 / 0 of 138; with no punter 12 / 9 / 5 (8.7 / 6.5 / 3.6%; pre-fix 5.1%; about 5% real per the pre-fix item, source not checked). No team lacks both. Kickers 2.7 to 2.8 and punters 1.5 to 1.6 a roster (real table scaled to 105: 3.1 and 1.8).
Why: Traced in the report section "Kickers, punters and roster needs" (fix list topic 1, PR #28). The camp-star fix (A3) did not cost rosters their specialists.

**FCS share of draft picks holds at the real level.** Draft picks by final division: FCS 5.4 / 5.2 / 5.2% (pre-fix 5.7%; trial 5 6.0 to 6.8%; real about 6%, CLAUDE.md calibration note). FCS share by year runs 2.3 to 9.3 / 2.7 to 11.7 / 3.1 to 9.7%; trial 5 had years of 18.7 to 21.8%.
Why: Inferred: the draft pipeline was not touched by the fixes.

**Coaching churn is unchanged.** FBS head coaches a year, 1997-2025: hires median 27 / 25 / 27, fired median 19 / 18 / 20 (mean 18.7 / 18.3 / 19.1), left for another job 4 / 3 / 4, tenure median 4 years (pre-fix 25 hires, 17 fired, mean 18.3; trial 5 fired 18 to 19, hired 26 to 28). From memory, to verify: real FBS has about 15 to 30 head coach changes a year.
Why: Inferred: crowning the BCS champions (B1) now credits those coaches with titles, which feeds job security, but the yearly volume did not move.

**Every big rating drop names its cause.** All 24 / 23 / 20 seasons with a 2+ point drop carry recorded events (form slump 22 / 23 / 18, major injury 20 / 17 / 18, with games missed). 0 seasons with 4+ games missed lack an injury event. Only 180 / 187 / 184 of about 12,700 form slumps of 5+ (1.4%) have no recorded cause.
Why: Traced: the raw export writes each season's events, games missed, coaching and now the dip and spring gain (raw.rs). The few uncaused form slumps come from the yearly form draw itself (development.rs: a normal draw with spread 2, plus the team situation for young players), which needs no event.

**Spring breakout news keeps its dates and texts.** 5,539 / 5,504 / 5,527 'breaks out in spring practice' events, all on April Saturdays (the spring game), college only; high school steps are never posted as news. High school steps by players from states without spring ball: 2 / 0 / 0.
Why: Traced: world/hs_year.rs jumps posts a medium event only for college steps, dated on the spring game; high school notices and steps run only in the 10 spring-ball states.

**Player pages are structurally sound.** 103 cards in each world: no drafted player without a college season, no season ages out of order, no level going backwards. One card overall differs from the last row by 2 points (Leon Jackson, world 2028), because the pros moved him from IOL to OT.
Why: Traced: cards.py checks every card; the position change is in his notes.

### Recruiting

**Each recruit announces on his own day.** There are 589 / 571 / 558 5-star commitment announcements at regular positions over 30 years. They spread over every weekday (76-90 / 67-100 / 73-89 each), and no day has more than 3. July 4 is the only cluster: 24 of 186 / 18 of 194 / 19 of 193 July-August announcements (12.9% / 9.3% / 9.8%).
Why: Traced: each decision gets its own announcement day after its wave, and 8% of the June wave pick July 4 (Claude's pick, proposed, not confirmed). Real: recruits pick their own day, and July 4 is a known favorite (from memory).

**The early period changes signing and early enrollment the way it did in real life.** February's share of FBS signees for classes 1997-2017 is 94.0% / 94.2% / 94.0% (the rest enroll in January). January enrollees are 1.3-2.6% of FBS+FCS signees in the 1990s and 7.4% / 7.9% / 7.9% for class 2017, then 22.7-33.5% from class 2018. Blue-chips who enrolled early: 5-stars 13.5% / 12.6% / 12.3% before 2018 and 51.7% / 47.5% / 54.4% after; 4-stars 10.9% / 10.0% / 9.6% and 39.2% / 39.7% / 39.6%.
Why: Traced: early_enrollee_chance ramps up through the 2000s and jumps once recruits sign in December (signing_stage Early). Real: early enrollment grew through the 2010s and jumped with the December period (from memory, to verify against 247Sports early-enrollee counts).

**Signing days and the early period fall on the real dates.** All 36 national signing days are on the first Wednesday of February. The early period opens on Wed Dec 20 2017, Dec 19 2018, Dec 18 2019, Dec 16 2020, Dec 15 2021, Dec 21 2022, Dec 20 2023, Dec 4 2024 and Dec 3 2025. Since fix A8, every one of them is a real date. No log year before 2017 has a December commitment or signing, in all three worlds.
Why: Traced: calendar/anchors.rs YearDates sets the first Wednesday of February and the era's December Wednesday. calendar/rules.rs turns the early period on from the class of 2018. Real: the first early period was Dec 20-22, 2017, and it moved to the first week of December from 2024 (docs/research/rules-and-money-2026.md).

### Portal, eligibility and the draft

**Title-game players declare after their last game.** 2024 season: the title teams' projected first-rounders declared on January 24 (title game January 20), 3 / 6 / 5 of them. 2025 season: January 23 (title game January 19), 7 / 3 / 2. Everyone else declared by January 15. In the BCS and four-team years, the title game came before January 15.
Why: Traced: recruiting::declarations skips teams with an unplayed game, and clock::advance_day runs the deadline again on title day + 4. Real 2025 rule: title-game players had until about January 24 (NFL.com, in calendar.md).

**The 2022-2025 transfer windows sit on their real days.** 2022: Dec 5-Jan 18 plus Apr 15-30. 2023: Dec 4-Jan 2 plus Apr 15-30. 2024: Dec 9-28 plus Apr 16-25. 2025: Jan 2-16 with no spring window. In all three worlds no entry falls outside its window except playoff players' entries in the 5 days after their last game (before the fixes, 2 entries in world 2026 fell outside).
Why: Traced: clock::portal_windows reads the era's transfer rule from calendar/rules.rs. The 2025-26 window and the CFP 5 days come from docs/design/calendar.md (ESPN source); the 2022-2024 dates are from memory, to verify. The pre-2022 shape is a negative (one winter stretch).

**Portal outcomes are close to real.** In 2018-2025, 74-85% / 74-84% / 73-86% of entrants landed somewhere. 13-26% neither landed nor went back. 2-23 a year returned to their old school.
Why: Inferred: the landing share comes from portal_day's matching. Real: Hudl's 2025 report has 737 unsigned of about 4,460 tracked entrants (16.5%); the NCAA's 2022 count has 1,833 of 2,918 FBS entrants on aid at a new school, which leaves out walk-ons (both in data/real-world/rates.json).

**The sit-out rule follows each era.** Moves into FBS from a four-year school (players' notes): 1991-2005, 158 of 158 / 155 of 155 / 168 of 168 sat out (December graduates sat too). 2006-2019, 131 of 143 / 131 of 136 / 180 of 189 sat, and every non-sitter was a December graduate (12 / 5 / 9). From 2020, 0 of 59 / 52 / 32 sat. The yearly counter is 15-53 to 2019 and 0 after.
Why: Traced: recruiting::portal_day sets the sit from the era's rule, the destination and the graduate flag. Real (from memory, to verify): a four-year transfer into FBS sat a year until the one-time exception, and graduate transfers were exempt from 2006.

**Deals can fall through while a player waits (T9).** Among landed transfers in the notes, the first choice fell through for 7% / 7% / 6% before 2018, 5% / 6% / 5% in 2018-21 and 3% / 5% / 4% from 2022.
Why: Traced: recruiting::portal_day gives about a 1% chance a day of waiting that the first choice goes elsewhere. No real count is published.

**Opt-outs skip bowls and almost never the playoff.** In the notes, 2 / 5 / 1 of the 168 / 166 / 186 opt-outs skipped a playoff game (before the fixes, 8 of 237 in world 2026). Bowl opt-outs are 0-4 a year before 2016. The 2016-2025 trend is a negative.
Why: Traced: college::opt_outs gives a first-rounder 0.45-0.75 for a bowl and 0.03 for a playoff game, scaled by era. Real (from memory, to verify): the trend started with Fournette and McCaffrey in 2016, and playoff opt-outs are rare.

**Coach-change windows take only players whose head coach left.** Entries in the coach-change windows: 392 / 317 / 345 (2022), 358 / 281 / 378 (2023), 441 / 394 / 400 (2024) and 342 / 392 / 482 (2025), all from programs with a new head coach. The windows last 30 days for 2022-2024 and 15 days from 2025.
Why: Traced: recruiting::portal_open lets in only players at a program with a new head coach who carry the 'his head coach is gone' lean event. The window lengths are Claude's pick, proposed, not confirmed. That every window opens on one day is a negative.

### NFL, high school and weather

**Super Bowl and season dates land on the real days.** Super Bowl on the real date in 36 of 36 seasons in all three worlds. First game on the real date in 32 of 36; the misses are 1990 and 1993 (spin-up years, a week early), 2001 (a week late, the 9/11 week is not modelled) and 2012 (Thursday instead of the real Wednesday). Wild card weekend starts on a Saturday, Black Monday is a Monday, the Hall of Fame Game a Thursday, enshrinement a Saturday and NFL Honors a Thursday in 36 of 36 seasons in all three. Real dates are from memory, to verify.
Why: Traced: calendar/anchors.rs builds every date from a table of real Super Bowls for 1990-2003 and from Labor Day after that.

**Era rule books switch in the right years.** In all three worlds: NFL games a season 267 to 2019, 269 in 2020, 285 from 2021 (real 272 + 13 from 2021, real.json). The draft opens on a Saturday to 2009 and a Thursday from 2010. The cap text runs $34.6 million (1994) to $279.2 million (2025), the same values as before the fixes, with 'no salary cap' in 2010 and a 136-day lockout in 2011. BCS standings 1998-2013, the 4-team CFP 2014-2023, 12 teams from 2024. The early signing period opens Dec 20, 2017. Spring transfer windows 2023-2025, one January window from Jan 2, 2026. Four-game redshirts from 2018.
Why: Traced: calendar/rules.rs RuleBook::era and anchors.rs switch each rule in a hard year from the real history.

**Formula schedules follow the real TV and international eras.** Prime-time games a season: 35 (1996), 37 (2005), 44 (2006), 52 (2012), 53 (2016) and 55-58 (2020-2025) in all three worlds. Games abroad match real.json's abroad_by_year in 36 / 36 / 34 of 36 seasons (Mexico City 2005, London from 2007, Toronto 2008-2013, none in 2020, 7 in 2025; the two misses are Toronto, under not fixed). Saturday games come only after Army-Navy, and week 18's Saturday games are division games.
Why: Traced: nfl_year::international_sites holds the real counts by year; formula_slots adds Thursday nights late in the season from 2006 and every week from 2012, Christmas games, Black Friday from 2023. The real list is abroad_by_year in /home/claude/runs/test-dash/data/real.json, itself from memory, to check (its own note).

**Snow and cold fall where and when they should.** Snow in 169 / 195 / 151 exported college results, 166 / 195 / 149 of them in November or December, almost all in northern and mountain states. The Saturday 1pm replay (world 2026) gives no snow in TX, LA, FL or SC, 15% of December Saturdays in Minneapolis and 19.5% in Syracuse. Rain in 13.4% / 13.3% / 14.0% of college games; 48 / 53 / 54 lightning delays a year.
Why: Traced: calendar/weather.rs draws each day from the state's monthly normals and snows only when it rains below 34F. One climate town per state is the known limit (A34).

**When a hurricane hits, the game moves like a real one.** All 50 / 47 / 29 moved college games since 1996 go to a later open Saturday; 13 / 26 / 9 are cancelled, only when no Saturday is open. High school no contests jump in storm years: 309 in 2023 (world 2026), 255 in 2022 (world 2027), 215 in 2011 (world 2028), against about 145 in a normal year.
Why: Traced: college::play_day looks for the next Saturday both teams have open; hs_year::play_week turns 30% of a storm Friday's games into no contests.

**Heat rules bite in the Deep South and barely in the Mid-Atlantic.** The Python replay of weather_at matches the world's heat-day counter in 36 of 36 years in all three worlds. Share of preseason practice days at a heat index of 104F or more at 3pm (world 2026): LA 35%, FL 34%, TX 31%, MS 29%, AR 25%, SC 17%, GA 9%, NC 6%, VA 3%, MD 0.9%, NJ 0.7%. About 44 / 42 / 45 limited days a year.
Why: Traced: hs_year::heat_days counts practice days before the opener in the 15 states with a heat rule.

**Each state's high school season sits on its own real dates.** In 2025 every state opens on a Friday except Michigan and Minnesota (Thursday), as they really do. The 2025 finals land in the real windows from the research files (TN, SC, MA, MO Dec 6; LA, VA Dec 13). Every school gets one homecoming and one senior night: 14,018 / 14,021 / 14,022 and 14,023 / 14,021 / 14,021 a year for 14,078 programs. About 68,100 high school games a year.
Why: Traced: hs_year::state_dates shifts each state's 2026 dates by Labor Day and Thanksgiving, so the weekdays hold every year.

### Stakes, news and postseason

**The postseason format follows the real eras.** In all three worlds: 1996-97 crowns the final poll's No. 1. 1998-2005 play the BCS title game in the real host bowl (8 of 8). 2006-2013 have a separate BCS title game. 2014-2023 play the semifinals in the real bowl rotation (10 of 10). 2024-25 have 12 teams with 4 first-round games. The 2024 byes went to conference champions in all three worlds. In 2025 at least one top-4 seed in world 2028 was not a conference champion, which the real 2025 straight seeding allows. The 2024 and 2025 title games are on Jan 20 and Jan 19, the real dates.
Why: Traced: world/college.rs selection_day and semifinal_bowls, the eras in calendar/rules.rs and the dates in calendar/anchors.rs. Real rotations and dates are from memory, to verify.

**The stakes days fall on the right weekdays and in order.** Each world: conference title games on Saturday (214 of 214), selection day on Sunday (30 of 30), Army-Navy on a Saturday no earlier than the second Saturday of December (30 of 30), Heisman finalists on Tuesday and the ceremony on Saturday (30 of 30), the separate BCS and CFP title games on Monday (20 of 20; two real ones were Thursdays, see the dates item). No order problems. NFL: wild card games Saturday to Monday with Monday games only from 2020 (6), divisional games Saturday and Sunday, conference games Sunday, the Super Bowl on Sunday (36 of 36).
Why: Traced: calendar/anchors.rs builds each date from real anchors (Thanksgiving, Labor Day, the nth weekday of a month). Same as before the fixes.

**Polls come out on the real rhythm.** Each world: the preseason AP poll on a Monday in mid-August (30 of 30), BCS standings on Sundays (132 releases in 16 seasons), CFP rankings on Tuesdays (57) plus the last one on selection Sunday (12), about 6 a year.
Why: Traced: world/clock.rs advance_day and world/college.rs release_poll. Real rhythm from memory, to verify. The first CFP ranking date is in the dates negative.

**The year keeps its agreed shape.** The draft, cutdown day, the trade deadline, the playoffs and the Super Bowl are Huge in 30 of 30 years in each world. No Huge moment and no Continue stop falls in May or June. The only July stop is the mid-July deadline for tagged players.
Why: Traced: world/clock.rs year_start and world/nfl_year.rs. T8 asks for these huge moments every year and a quieter stretch from May to July.

**NFL late-season stakes are read per team, and the counts add up.** 17.6 / 18.6 / 18.3 of the 32 teams a year play for their season in the last four weeks (before: 17.6). 10.7 / 12.2 / 12.1 games a year matter to both sides and are Huge; 29.2 / 29.1 / 29.1 matter to one side and are Big. The per-team counts match the stake events in 36 of 36 years in each world.
Why: Traced: world/nfl_year.rs stakes reads each side on its own. From memory, to verify: about 18 to 22 teams are still alive going into week 15 in recent seasons.

**The fixes did not move any stakes loudness.** Comparing each fixed world with its pre-fix world, event counts by level, kind and loudness moved by more than 10% only for recruiting commitments at Medium (191 / 192 / 193 a year down to 66 / 67 / 67), signings at Medium (87 / 87 / 88 down to 33 / 33 / 34), and a few draft-choice and portal notes. Game results, polls, awards, stakes and NFL dates kept their loudness.
Why: Inferred from kinds_compare.py. The recruiting drop comes from the star-rating fix (A1), outside this area.

## Negatives and bugs (65)

Bugs and export gaps first, then realism. "New" marks problems the fixes caused or uncovered.

### Bugs (11)

**A camp redshirt who plays is still developed and shown as a redshirt** (World and players, new, serious)
- Seen: When a camp redshirt plays, his season row keeps the role 'redshirt' with no performance. World 2026 alumni: all 13,407 'cost him his redshirt year' notes that match a row sit on a 'redshirt' row without a performance; 11,303 of them played 5 to 16 games (8,655 played 5 to 12, 2,648 played 13 to 16). Example: DL Garrett (EDGE, Alabama, world 2027): 'Played in 12 games at 19, which cost him his redshirt year.'; his age-18 row reads redshirt, no performance, 71 to 80. Major injuries on such rows show 0 games missed (560 / 569 / 569 redshirt rows).
- Why: Traced: development.rs develop_season treats PlayingTime::Redshirt as a year without games (no_games): no performance, no slump or breakout draw, half the injury chance, games missed set to 0, and physical growth x1.35. B9 put camp redshirts back into games (world/college.rs start) but the yearly step still reads their camp role; settle_eligibility only changes eligibility and adds a note.
- Fix (proposed, not confirmed): When a camp redshirt plays, the yearly step treats that season as a backup season (games, performance, form, injuries), and his row says backup.

**An early enrollee's college spring step lands on his high school row** (World and players, new)
- Seen: On alumni pages, 13% of college spring steps (518 / 513 / 499 of 3,992 / 3,936 / 3,970) land on the age-17 high school row. Example: Leon Jackson (world 2028), an early enrollee at Texas: 'Broke out in spring practice at 17: +10.3', and his high school senior row runs 50 to 67.
- Why: Traced: record_spring_step adds the gain to the latest row, and an early enrollee has no college row yet in his first spring (the fixer flagged this).
- Fix (proposed, not confirmed): An early enrollee's college spring step opens his first college row instead of ending his high school row.

**Draft notes say 'a EDGE' and 'a OT'** (World and players)
- Seen: 27 / 45 / 35 draft notes on the 103 cards, for example 'a OT is a premium position' (pre-fix 41 in world 2026; trial 5 42).
- Why: Traced: world/recruiting.rs line 890 writes 'a {position} is a premium position' with a fixed 'a'.
- Fix (proposed, not confirmed): The note writes 'an' before EDGE, OT, IOL, LB, S, RB and LS.

**Signing-day news is filed under the next football year in most years** (Recruiting, new)
- Seen: 22 of 36 signing days fall after the Super Bowl (e.g. Wed Feb 6, 1991). On those days the commitment, flip and signing news carry the next football year (1991: 36 commitment and 30 signing items), while the 'National signing day' date line and, since fix A6, the counters carry the class's season (1990).
- Why: Traced: fix A6 swaps in the season's log for the counters only. The news items take the year the clock is in, and the clock has already started the new football year after the Super Bowl.
- Fix (proposed, not confirmed): File a signing stage's news under the class's season too, as A6 does for the counters.

**Specialist signing notes repeat the same star count** (Recruiting, new)
- Seen: 258 / 262 / 268 archived signing notes read like 'as a 3-star K (a 3-star by the specialist camps)'.
- Why: Traced: since fix A3 the signing note always adds the camps' stars for a kicker, punter or snapper, even when they equal the services' stars.
- Fix (proposed, not confirmed): Add the camps' stars to the note only when they differ from the services'.

**Long snappers almost never log a game, so their seasons go uncounted** (Portal, eligibility and the draft, new)
- Seen: College season rows for 2018-2025: long snappers 927 / 976 / 897 (97% with 1-4 games), against kickers 5,552 / 5,517 / 5,515 and punters 3,951 / 4,006 / 3,957. 4 / 7 / 5 long snappers from the class of 2015 'declared early' in 2018, their fourth season, right after a December note saying it was their last.
- Why: Traced: in game/flow.rs, the field-goal unit's on_field list is the protectors and the kicker, and the punt unit is the punter plus coverage. Neither includes the snapper. world/college.rs play_day counts a game only for a player with a snap. So college::settle_eligibility sees 0 games, and from 2018 a starting snapper's season becomes a four-game redshirt once. In 2018 that gave fourth-year snappers a fifth year, and they 'declared early'.
- Fix (proposed, not confirmed): Put the long snapper (and the holder) on the field for kicks and punts, so their games count.

**The Shrine Bowl and the Senior Bowl get the wrong talent tiers** (NFL, high school and weather, new, serious)
- Seen: To 2021 the Shrine Bowl names its rosters first, skipping the 60 best, so it takes the next tier: 572 / 558 / 529 of its players went in picks 33-100, and of its players who reached the NFL only 14% / 14% / 16% went undrafted, against 23% / 23% / 23% at the Senior Bowl. In 2022-2025 the Senior Bowl names its rosters first, so the Shrine Bowl gets what is left: 4 / 8 / 14 of its players went in picks 33-100 over 2022-2025, and 63% / 65% / 64% of those who reached the NFL went undrafted (world 2026: 230 of 364; counting all its players, 232 of 366).
- Why: Traced: recruiting::all_star_invites runs 7 days before each game; the Shrine and NFLPA games skip the 60 best and take the next best left, the Senior Bowl takes the best left. So the game dates decide the order. The Shrine Bowl moved into Senior Bowl week from 2022 (C2), and the Senior Bowl date falls a week early in 2022-2025, so the order flipped. Real Senior Bowl rosters are picked first, in the fall, from the best seniors, and the Shrine Bowl takes the next tier (from memory, to verify).
- Fix (proposed, not confirmed): Let the Senior Bowl name its rosters first every year, and the Shrine Bowl take the next tier after it, whatever the game dates.

**The awards show crowns the same players year after year** (Stakes, news and postseason, serious)
- Seen: The same player wins the same award in back-to-back years 42 / 42 / 41 times in 30 years over the 8 awards (for example the Doak Walker 9 times in world 2026, the Davey O'Brien 10 times in world 2028). Real back-to-back winners are rare, for example Michael Crabtree (2007-08) and Justin Blackmon (2010-11) in the Biletnikoff (from memory, to verify).
- Why: Traced: world/college.rs awards('show') ranks starters by last_performance plus 0.01 times their season production. last_performance is the rating from the fall-camp development step, and 0.01 times a season's production adds only about 2 points. So the best-rated player wins again and again, although the code's comment says skill players are judged by stats. It was in the pre-fix notes as out of that analyst's area.
- Fix (proposed, not confirmed): Judge the awards show from the season's production against the position's best, with team success, the way the Heisman now works.

**The Commander-in-Chief's Trophy has two holders at once** (Stakes, news and postseason, new)
- Seen: World 2026, 2010: Navy 'claims' the Commander-in-Chief's Trophy by beating Air Force on Oct 9, then Army 'keeps' it by beating Air Force on Nov 6. In 2011 Air Force 'claims' it twice. After both games, two different academies hold it in 24 / 26 / 20 of 30 years.
- Why: Traced: calendar/data.rs lists the trophy on two separate rivalry rows (Navy-Air Force and Army-Air Force), and result_event keeps a holder per row. In real life it is one three-way trophy: it goes to the academy with the best record in the three service games, Army-Navy included, and the holder keeps it on a tie (from memory, to verify).
- Fix (proposed, not confirmed): Treat it as one three-way trophy, settled after the Army-Navy game.

**A team's own huge game can still be posted as Medium** (Stakes, news and postseason)
- Seen: Every rivalry-week game is huge for both sides, but of 990 such results 346 / 324 / 345 were posted Medium and 438 / 446 / 427 Big. Now that Continue stops on Big, the Medium ones (about 11 a year) are the ones it still skips for the teams they matter to.
- Why: Traced: stakes_for_day stores who the game is huge for on the Game, but result_event posts the game's own loudness (g.loud).
- Fix (proposed, not confirmed): Post a game's result at least Big when it is huge for one of its teams.

**Momentum hits on interception returns are skipped** (Match engine, new)
- Seen: Found while fixing the stat rows, not measured: on an interception return, missed tackles and fumbles look for the player on the wrong team, find nobody, and give no momentum hit.
- Why: Traced: game/flow.rs resolve, the edge_hit calls use fixed sides (missed tackles on the defense, fumbles on the offense).
- Fix (proposed, not confirmed): Look the player up on the team he plays for, as the stat fix does. This changes game results a little, so it needs a world run to measure.

### Export gaps (2)

**Player pages' season rows show no events** (World and players, new)
- Seen: Card season rows carry only age, level, role, position, start, end, performance and level ratings. They have no events, games missed, first-year dip or spring gain, so a page shows a 17-point senior year or a 16-point form slump with no reason beside it (same before the fixes).
- Why: Traced: afm-tools/src/dashboard.rs card writes the short row; only the raw export (raw.rs) and the dashboard slump list carry the events.
- Fix (proposed, not confirmed): The player page rows carry the season events, games missed, the dip and the spring gain.

**The dashboard's day-one share mixes windows** (Portal, eligibility and the draft)
- Seen: The dashboard shows 37.1% / 36.6% / 36.7% / 39.8% as the first-day share for 2022-2025 (world 2026), while the winter window's own first day took 56-60%. Its portal_by_day lists 27 / 27 / 19 repeated dates in 2022-2024, and each football year mixes the previous spring window with the next winter's windows.
- Why: Traced: tools/dashboard/build_data.py takes the busiest single row over all of the year's rows. Coach-change and winter windows log separate rows for the same dates, and a log year holds April of one cycle and December-January of the next.
- Fix (proposed, not confirmed): In build_data.py, merge rows by date and split them by window (coach-change, winter, spring) before computing the first-day share.

### Realism (52)

**From 2018 the four-game stop holds nearly every freshman to four games** (World and players, new, serious)
- Seen: FBS true freshmen with 5+ games (median a year): 871 / 865 / 877 in 1997-2017 and 132.5 / 135.5 / 147.5 in 2018-2025 (world 2026: 821 in 2017, 153 in 2018). 5-stars with 5+ games as true freshmen: about 27 of 33 a year before 2018, about 7 from 2018. From 2018 only 5 / 4 / 5 redshirts were burned in all, and 85-87% of the kept-redshirt notes stopped at exactly 4 games. The redshirt role goes to 91-94% of first-year players in both eras (9,781 / 9,822 / 9,847 camp redshirts on the final rosters).
- Why: Traced: world/season.rs depth_chart gives the redshirt role to every college player below the starters and rotation who still has his redshirt, and the review fix (bda2c42, world/college.rs play_day) takes every such player off the game roster at four games from 2018, whatever his level or the team's need. Before 2018 the same players simply kept playing. Real (from memory, to verify): the 2018 rule let more freshmen play a few games; it did not bench freshmen who start or rotate.
- Fix (proposed, not confirmed): The redshirt role goes only to freshmen the staff means to sit (below the travel squad), so the four-game stop never touches the two-deep. A narrower option: let the staff burn the redshirt of a freshman who becomes one of the best two at his position.

**Before 2018, planned redshirts play anyway, many for a handful of games** (World and players, new)
- Seen: 510 / 512 / 518 FBS true freshmen a year played only 1 to 4 games in 1997-2017 and lost their redshirt year for it. Exported players carry 22,518 / 22,275 / 22,390 'cost him his redshirt year' notes before 2018, 3,865 / 3,788 / 3,684 of them for 1 to 4 games, and 6,036 / 5,951 / 5,954 players carry two. 75-80% of each alumni class from 1993 to 2016 carries one (world 2026: 666 of 851 in 1993), two thirds of them after 11-13 games. Example: Bryan Comeau (world 2026, Colorado): 2 games at 19, then 7 games at 20, each 'cost him his redshirt year'.
- Why: Traced: depth_chart hands the redshirt role to 91-94% of freshmen, and season::game_team gives the match engine every college player on the roster, so the engine plays 'redshirts' by rating. settle_eligibility then burns the year and sets redshirt_used = false, so next fall he gets the role again. The full-season cases are the role problem in the four-game item; this item is the few-snap burns. The portal checker rated it bad. Real (from memory, to verify): before 2018 coaches rarely played a planned redshirt for a few snaps.
- Fix (proposed, not confirmed): Once the redshirt role goes only to freshmen the staff means to sit, keep those players out of games before 2018 unless injuries at their position force one in.

**A spring step now makes a slump more likely the very next fall** (World and players, new)
- Seen: Slump rate in the season right after a spring step: 8.8 / 9.3 / 8.4% (college 12.8 / 12.4 / 11.9%) against 5.1 / 5.0 / 5.1% (college 6.3 / 6.3 / 6.4%) with no step in the two seasons before. Pre-fix world 2026: 7.1% in the next season and 11.7% in the season after.
- Why: Traced: development.rs develop_season raises the slump chance from 6% to 16% after a 'hot' year (a rise of 6+ between the last two rows' ends). Since D4 the step sits on the earlier row's end, so the check sees it one season sooner: the breakout fall itself.
- Fix (proposed, not confirmed): The hot-year check leaves the spring gain out, so a spring step does not raise the slump chance of the fall it leads into.

**Titles concentrate on too few programs** (World and players)
- Seen: Distinct FBS champions in 30 years: 8 / 7 / 9, and the top program has 10 / 13 / 8 titles (Georgia, Georgia, Texas A&M). One program reaches 16 / 15 / 15 of 28 title games. Most titles in any 15 years: 6 / 9 / 5; fewest different champions in a 15-year window: 5 / 4 / 5 (real 4-6 and 7-9, real.json balance). World 2027: Georgia won 9 titles under one coach, Robert Grimes (1993-2011), whose roster was the strongest in FBS in 10 of 16 seasons. Series records lean the same way: 19 / 18 / 22 of 77 long rivalries have a leader with 80% or more of the wins (Florida-Georgia 30-0 in world 2027).
- Why: Traced in world 2027: Grimes' hidden ratings are recruiting 18.7, network 20.0 and scheme 18.4 (raw coaches.jsonl), and Georgia's roster ranked first or second in nearly every year of his tenure (programs.jsonl). In the BCS years only the top two play for the title, so the clearly best team gets there most years. Crowning the BCS champions (B1) made this visible; it was hidden before. Real (from memory, to verify): about 15 distinct champions in 1996-2025, at most 6 titles and about 9 title games for one program.
- Fix (proposed, not confirmed): Answer the dynasty re-trace's yes/no list (docs/trials/world-trial4-proposals.md, last section), which proposes limits on how long one staff can keep the strongest roster.

**Ratings almost never fall, even after a major injury** (World and players)
- Seen: Seasons losing more than 1 point: 0.10 / 0.11 / 0.11% (pre-fix 0.15%). The biggest drop in three worlds is 2.9 points (Eugenio Mascorro, CB, 21, FBS, major injury, 12 games missed). Major-injury seasons: median change +0.6 in every world (pre-fix +0.5); only 25 / 25 / 24% lose any ability; at 22 or older the median is +0.1.
- Why: Traced (this corrects the pre-fix item): a major injury does take 1.5 to 5.0 points off speed, acceleration, agility and jumping, and 0.5 to 4.0 off their ceilings, scaled up from 24 and 28 (development.rs develop_season, the 'Torn ACLs' branch). But the same season's growth still runs at half rate, and for players under 22 it outweighs the loss. The slump branch takes 0.5 to 3.0 off skills only. Fix list item 4 (volatile development) is agreed but not built.
- Fix (proposed, not confirmed): Build fix list item 4 (volatile development): settle in its detail round which causes take ability away and how much, starting with major injuries.

**One-score games are too rare** (World and players)
- Seen: FBS games decided by 8 or less: 28.9 / 28.9 / 28.5% (pre-fix 29.9%; real 36.7%, real.json vitals.FBS.one_score).
- Why: Inferred: a game's margin strays from what the two lineups' strength predicts by 16.7 / 16.8 / 16.75 points (median of the yearly spreads, dashboard 'luck sd'); real.json gives 13.0 for real games, though the two may not be measured the same way. Results spread too wide around the expected margin, so fewer games end close.
- Fix (proposed, not confirmed): Find which part of the engine widens single-game margins (for example drive outcomes in lopsided score states) before tuning it.

**A recruit called a 5-star when he commits often ends far lower** (Recruiting, new, serious)
- Seen: Classes 1997-2021 had 489 / 463 / 468 recruits whose summer or fall commitment news called them 5-stars. Their final stars: 5-star 25% / 28% / 27%, 4-star 37% / 36% / 38%, 3 stars or fewer 38% / 36% / 35%, and 7 / 10 / 7 ended unranked. The same swings feed the flips: 62% / 64% / 64% of blue-chip flip news is a move up from an FCS, D2 or JUCO commitment to FBS. Before 2018, 26% / 27% / 28% of final blue-chips' first commitments went to a school below FBS.
- Why: Traced: run_signing_stage draws a fresh services' projection for every recruit at every stage (scout_player, accuracy 12). Each rating pass (four summer waves, then December or January) ranks the class on 0.25 x film + 0.75 x projection - visibility and replaces the last stars (fix A1). Nothing carries a recruit's last rating forward, so each pass is a new draw around his true upside. Before the fixes this was hidden, because stars only piled up. Real: rankings move, but a summer 5-star rarely ends below 4 stars and never unranked (from memory, to verify against 247Sports composite histories).
- Fix (proposed, not confirmed): Let each rating pass move a recruit's projection only part of the way toward the new read (for example halfway), so ratings drift between passes instead of being drawn again.

**One in three 4-stars before 2018 does not start at an FBS school** (Recruiting, serious)
- Seen: Final 4-stars of classes 1997-2017 first played at: FBS 64.8% / 65.0% / 65.0%, FCS 12.0% / 12.4% / 12.9%, D2 2.6% / 2.0% / 2.4%, JUCO 5.0% / 4.8% / 4.9%, and never in college 15.6% / 15.7% / 14.8%. That is 43.8 / 44.3 / 41.0 4-stars a class who never played college (pre-fix 206 with stacked labels; trial 5: 1.8). Most of their notes end 'None of his offers worked out, and he stopped playing after high school.' (957 / 980 / 911). These are weaker players: their last high school row ranks them 73 among seniors, against 85 for 4-stars who signed FBS. Classes 2018-2021 do better (FBS 82.8% / 84.6% / 84.2%), though that sample lacks players still in college.
- Why: Traced: staffs judge recruits on 0.6 x film + 0.4 x their own projection read, while the services' stars weigh projection at 0.75. Before 2018 the last rating is the January pass, and the February signing stage draws a new projection, so the label and the staffs' read come from different draws. From 2018 the December stage rates and signs on one draw. D2 staffs skip any 4-star not already committed to them, so a 4-star the FBS staffs pass on has few places left. Inferred: the label is too loosely tied to who the staffs want, most of all before 2018. Real: nearly every 4-star signs with an FBS school, and a few go to junior college or prep school for grades (from memory, to verify). The fix for the item above may cover part of this one.
- Fix (proposed, not confirmed): Rate the class with the same film and projection weights the staffs use, from the same projection draw the signing stage reads.

**5-stars reach the draft more often than real ones** (Recruiting, new)
- Seen: Final 5-stars of classes 1997-2019 were drafted 70.5% / 71.9% / 74.0% of the time (real 62.2%), in the first round 31.2% / 31.2% / 32.4% (real 21.0%). 4-stars match (see positives).
- Why: Inferred: the services' read of the very top players is too exact at the final pass. Any change to how ratings drift between passes will move this number too, so check it after that change. Real: real.json draft_by_star (ESPN-rated FBS and FCS signees, classes 2010-2021, drafted 2013-2025).
- Fix (proposed, not confirmed): Widen the services' projection noise for the top of the class until the 5-star draft rate sits near the real 62%.

**The All-American Bowl roster is mostly 3-stars** (Recruiting, new)
- Seen: The game takes 100 players a year, but only 36.0 / 36.6 / 36.6 a class end as blue-chips (5-stars 22.0 / 21.8 / 23.4, 4-stars 14.0 / 14.8 / 13.2; classes 2001-2017). Only 5% of final 4-stars who did not enroll early play in a January bowl (313 / 336 / 295 of about 6,100).
- Why: Traced: hs_year.rs all_star_game ranks players with 3+ services' stars by stars + 0.05 x overall + noise. So one star is worth 20 points of current overall, and a strong 3-star beats most 4-stars. Inferred: every final 4- and 5-star is archived, so the rest of the roster ended at 3 stars or fewer. Real: the game invites the top national recruits, mostly 4- and 5-stars (from memory, to verify).
- Fix (proposed, not confirmed): Invite by the services' final ranking (stars first, then rank inside the tier), filling the position places in the next item, and keep the Polynesian Bowl's heritage bonus.

**Early enrollees miss the January high school all-star games** (Recruiting, new)
- Seen: Final 5-stars (classes 2001-2021) who enrolled early played in the All-American or Polynesian Bowl 18% / 20% / 25% of the time. Those who did not enroll early played 78% / 77% / 81% of the time. Since 2018 about half of the 5-stars enroll early.
- Why: Traced: early enrollees enroll on January 10 (anchors.rs early_enrollment) and become college players. The All-American Bowl is on the second Saturday of January (Jan 8-14) and the Polynesian Bowl on the third, and hs_year.rs all_star_game only invites players still in high school. Real: early enrollees play the All-American Bowl and then report to campus, and the real game has been in the first week of January in recent years (from memory, to verify).
- Fix (proposed, not confirmed): Let the class's signed early enrollees play in the January all-star games.

**About half of the camps' 5-star long snappers sign nowhere** (Recruiting, new)
- Seen: Class 2026 is the only class whose unsigned specialists the export shows. Of its 25 5-star camp snappers, 11 / 15 / 10 signed anywhere, and of the 75 4-star snappers, 39 / 42 / 29. Kickers do better: 20 / 19 / 24 of 25 and 49 / 32 / 38 of 75. Punters: 20 / 15 / 17 and 31 / 29 / 30.
- Why: Traced: camp_rankings converts each high school's best snapper to long snapper at the first rating. The top 25 get 5 camp stars and the next 75 get 4 (CAMP_TIERS is cumulative). Inferred: 100 rated snappers a class is more than the programs' plans take. Real: the top-ranked camp snappers nearly all land FBS places, often as preferred walk-ons (from memory, to verify).
- Fix (proposed, not confirmed): Shrink the long snappers' camp tiers to about 10 5-stars and 30 4-stars a class.

**Recruits give verbal commitments to junior colleges before their senior season** (Recruiting, new)
- Seen: Among final blue-chips, 233 / 287 / 249 (classes 1997-2017) and 32 / 36 / 42 (2018-2021) made their first verbal commitment to a junior college. About half of them (116 / 139 / 137 before 2018) did so before the senior season. Example: 'Committed to MN Junior College 6 at 17, before his senior season.' then 'Flipped his commitment from MN Junior College 6 to Clemson at 18.'
- Why: Traced: at a summer wave, run_signing_stage commits each recruit whose time has come to whatever program holds him (held[k] can be any division), junior colleges included. Real: junior colleges sign players late, mostly after the four-year schools' signing days, and don't take verbal commitments from rising seniors (from memory, to verify).
- Fix (proposed, not confirmed): Junior colleges take part only in the February signing stage, not in the summer waves.

**The House roster limit nearly doubles the FBS class of 2025** (Recruiting)
- Seen: Class 2025 signed 5,388 / 5,443 / 5,416 FBS players, against a median of 2,923 / 2,872 / 2,950 and 2,240-3,330 in every other year; FCS signed 4,036 / 4,075 / 4,072. Class 2026 is back to 2,923 / 2,856 / 2,971 FBS. Active FBS players of the class of 2025 in world 2026: 4,619, against 2,681 (class of 2024), about 33 a team. Pre-fix world 2026: 5,341.
- Why: Traced: for sign year 2025, world/ncaa.rs scholarship_rules moves FBS (and opted-in FCS) from 85 with a yearly cap of 25 to a 105-man roster with only the staff cap of 39. In the world the 85 counted walk-ons too, so about 20 places open at once and one class fills them. Real (from memory, to verify): 2025 high school classes stayed near their usual size, and programs filled the bigger rosters mostly with transfers.
- Fix (proposed, not confirmed): From 2025 keep a yearly FBS high school signing cap near a normal class (about 28), so the new places fill over several classes and through the portal.

**Too few recruits commit before their senior season** (Recruiting)
- Seen: Announced before the senior season: 26.9% / 26.7% / 27.0% of signees in classes 1997-2017 (pre-fix 29.9%) and 27.2% / 27.1% / 26.9% in 2018-2026 (pre-fix 27.1%). With no verbal commitment before signing (before 2018): 4-stars 51.3% / 51.7% / 51.2%, 5-stars 45.9% / 43.5% / 43.1%. February holds 43.2% / 43.5% / 43.1% of all commitments before 2018 (pre-fix 49.7%). From 2018, December holds 18.6% / 18.7% / 18.8% and February 26.8% / 27.2% / 26.9%.
- Why: Traced: commit_timing (Claude's pick, proposed, not confirmed) puts about 40% of recruits at the early period or later and moves 4- and 5-stars later. Real (from memory, to verify against 247Sports and On3): in the 2020s about 60-70% of a top 300 are committed when the senior season starts, and only about 10-15% reach the early period uncommitted.
- Fix (proposed, not confirmed): Shift the commit_timing split so about two thirds of recruits, blue-chips included, commit by the end of the summer and about 15% reach the early period uncommitted.

**No commitments before May of the junior year** (Recruiting)
- Seen: No log year in any of the three worlds has a commitment in March or April. The first verbal commitments come from May 1 of the junior year.
- Why: Traced: clock.rs commit waves fall on Apr 30, Jun 30, Aug 31 and Oct 31. Recruits get offers only at a signing stage of their own class, and the first is April 30 of the junior year. The confirmed T5.2 text says some kids commit as early as sophomores. Real: junior days (January-March) bring an early wave, and some top recruits commit as sophomores (from memory, to verify).
- Fix (proposed, not confirmed): Add a commitment wave at the end of March of the junior year, after junior days.

**June is a quiet month for commitments** (Recruiting)
- Seen: June holds 2.5% / 2.5% / 2.5% of all commitments before 2018 and 3.2% / 3.3% / 3.2% after. July holds 12.2% / 12.1% / 12.2% and 11.5% / 11.2% / 11.3%. 5-star announcements: June 5.4% / 5.3% / 7.5%, July 17.3% / 18.6% / 21.0%.
- Why: Traced: the June wave decides on June 30 and announces from July 1. The April wave's announcements end by late June. Real (from memory, to verify against On3 and 247Sports): since the 2018 official-visit change, June has been the busiest commitment month, right after the spring official visits.
- Fix (proposed, not confirmed): Move the June wave's decision day from June 30 to June 15, so its announcements start in mid-June.

**FBS and FCS sign almost everyone in December** (Recruiting)
- Seen: Early share for classes 2018-2026: FBS 97.6% / 97.7% / 97.5% (pre-fix 96.4% / 96.0% / 95.9%), FCS 94.7% / 94.3% / 94.5% (pre-fix 89.2% / 88.6% / 89.3%). The first class, 2018: FBS 97.2% / 97.2% / 96.5%.
- Why: Traced: the Early stage signs every recruit whose timing is the early period or sooner and who holds an FBS or FCS offer. By Claude's pick (proposed, not confirmed) that is about 88% of recruits. Inferred: fix A4 raised it a little, because commitments now hold in December too and fewer committed recruits wait. Real: 80-90% for FBS since 2018 (real.json calendar.fbs_signed_early_pct, from memory, to check), lower in the first year and lower at FCS (from memory).
- Fix (proposed, not confirmed): FBS staffs hold back about 10% of their class slots at the early period for February.

**Class sizes still echo the empty start** (Recruiting)
- Seen: First college seasons (players who appear in games) for FBS class 1999: 1,571 / 1,551 / 1,553, and class 1998: 1,655 / 1,686 / 1,694, against medians of 1,914 / 1,888 / 1,898. The dips fade after 2003. Median year-to-year swing: FBS 6.4% / 7.7% / 6.7% (pre-fix 8.8%, trial 5 2.5%), FCS 5.4% / 8.2% / 7.4% (pre-fix 10.9%), D2 10.4% / 11.0% / 10.7% (pre-fix 13.9%).
- Why: Traced: World::new starts with high school classes and empty college rosters and runs 6 spin-up years. The first classes fill empty rosters together and leave together. Inferred: the echo is weaker than before because commitments now hold and fewer recruits fall out. Real: FBS signee totals change a few percent a year (from memory).
- Fix (proposed, not confirmed): Lengthen the spin-up before recorded history from 6 to 12 years.

**The portal is about a tenth of its real size** (Portal, eligibility and the draft, serious)
- Seen: Entries across all divisions in the 2022-2025 cycles: 1,092-1,384 / 1,018-1,377 / 1,092-1,486 a year. FBS entrants average 169 / 172 / 176 a year over 31 years; about 26% of entrants are FBS players, so roughly 360 / 366 / 401 in the 2025 cycle (inferred). The final FBS rosters hold 364 / 347 / 400 four-year transfers: 2.7% / 2.6% / 3.0% of players, a median of 2 a team. Blue-chip or drafted alumni with an FBS-to-FBS move: 1.3% / 1.1% / 1.3% of classes 1997-2016 (trial 5: 9.9%).
- Why: Inferred: low base leans (PIPELINE.portal_entry backup 0.18, rotation 0.07), a starter lean of 0 and a narrow landing rule (the next two items) all cut the volume; which part cuts most is not traced. Real: 2,918 FBS and 2,085 FCS entrants in 2022 (data/real-world/rates.json); a record 4,201 FBS entrants in 2024-25, then 23% fewer in January 2026 (docs/research/trial3-baselines.md); Hudl counts 2,983 transfers landing at FBS in the 2025 cycle, about 22 a team (rates.json).
- Fix (proposed, not confirmed): Calibrate the portal-era entry rate to the NCAA 2022 counts and the 2024-25 record, after the starter and landing fixes.

**Starters almost never enter the portal** (Portal, eligibility and the draft, serious)
- Seen: Starters are 4.9% / 4.7% / 4.9% of entrants (before: 4.9% / 4.4% / 4.8%). Since 1991, 4-5 star starters entered only 49 / 40 / 40 times, 16 / 13 / 14 of them from FBS.
- Why: Traced: college::lean_checkpoint gives a starter a base lean of 0. A coaching change, a missed bowl or a blue-chip signing at his spot only multiply the lean, so none of them can move a starter. Real: starters chasing a bigger stage, NIL or a coach are a large share of entrants. Hudl 2025 has 22% of moves going one level up (rates.json).
- Fix (proposed, not confirmed): Give starters a small base lean so the existing events can move them (proposed: a third of a rotation player's).

**FBS entrants almost never land at another FBS school** (Portal, eligibility and the draft, serious)
- Seen: 13% / 13% / 13% of FBS entrants landed at FBS (out of 5,251 / 5,337 / 5,468). Before the fixes: 13% / 14% / 14%.
- Why: Traced: recruiting::portal_day lets a program want an entrant only if he would make its two-deep or beat its backups at an open starting spot. 78-79% of entrants are backups, so they drop to FCS, D2 or JUCO. Real: Hudl 2025 tracked 3,681 FBS-origin entrants and 2,983 landings at FBS, so at least 60% of FBS entrants stayed in FBS (rates.json).
- Fix (proposed, not confirmed): Let a program with roster room also take an entrant who would be its third man at the position.

**Early entries have the wrong shape across eras** (Portal, eligibility and the draft, serious)
- Seen: Underclassmen declaring, yearly average for the 1993-2003 / 2004-2013 / 2014-2020 / 2021-2025 drafts: 59 / 64 / 60 / 72, then 61 / 58 / 61 / 75, then 62 / 60 / 64 / 72. Before the fixes: 62 / 70 / 74 / 67 in world 2026. The NIL era is now the highest.
- Why: Traced: recruiting::draft_reads sets the chance only from the projected round and family money. T3.1 says the choice also weighs NIL, portal offers, playing time and the degree. Real (from memory, to verify): about 40-55 a year before 2011, 98 in 2014, about 95-115 a year in 2016-2021, then 58 in 2024 as NIL money kept players in school; wf/real.json gives 58-98 for 2021-2024, also from memory.
- Fix (proposed, not confirmed): Make the declare chance weigh the NIL money he would give up (from 2021) against the value of his projected round.

**The fixes made the portal 8-14% smaller still** (Portal, eligibility and the draft, new)
- Seen: Yearly average entries in 2022-2025: 1,234 / 1,158 / 1,232, against 1,345 / 1,346 / 1,378 before the fixes. In 2018-2021: 926 / 924 / 929, against 1,020 / 1,017 / 1,072. Before 2018 the change is under 3%. Most of the drop is first-year players a family steered to their school: 1,368 / 1,292 / 1,309 such entries in 31 years, against 2,246 / 2,323 / 2,267.
- Why: Inferred: the recruiting fixes (A1-A4) changed who signs where, and fewer signings now carry the steered mark (recruiting.rs line 3065). Steered entries scale with the era factor (0.5 before 2018, 0.8 in 2018-19, 1.0 from 2020; recruiting::era_factor), which is why the drop shows from 2018. The alumni file shrank from about 54,600 to 26,400 players after A1, so the notes cannot show the steered share directly.
- Fix (proposed, not confirmed): Find which recruiting fix cut the steered signings (not A4's fallback, which places nobody), then calibrate the portal size.

**More FBS transfers come from D2 than from FCS** (Portal, eligibility and the draft, new)
- Seen: Four-year transfers on the final FBS rosters, by where they came from (FBS / FCS / D2): 183 / 81 / 100, then 200 / 65 / 82, then 207 / 67 / 126. It was the same before the fixes (210 / 110 / 194 in world 2026).
- Why: Inferred: the world's D2 rosters are bigger than FCS (14,203 against 11,110 players in world 2026), and portal_day lets a D2 standout shop at FBS directly, even though his move-up reason targets FCS. Real: Hudl 2025 tracked 665 FCS-origin entrants against 112 from D2; both are floors, but FCS outnumbers D2 about 6 to 1 (rates.json), and FCS to FBS is the usual step up.
- Fix (proposed, not confirmed): Have FBS programs discount a D2 entrant's level the way they discount FCS play, so most D2 standouts land at FCS.

**Every opt-out is announced on selection day** (Portal, eligibility and the draft, new)
- Seen: Each season's opt-out news items all share one date: Dec 2, 2018; Dec 8, 2019; ... Dec 7, 2025 (for example, 9 items on Dec 8, 2024 in world 2026). The other worlds show the same.
- Why: Traced: college::selection_day calls opt_outs at its end, and opt_outs dates every announcement today. Real (from memory, to verify): players announce opt-outs across December, from bowl selection up to days before the game.
- Fix (proposed, not confirmed): Give each opt-out his own announcement day between selection day and his bowl.

**Every declaration lands on January 6 or January 15** (Portal, eligibility and the draft)
- Seen: In all 33 seasons in all three worlds, projected first-rounders declared only on January 6 or January 15. The exception is title-team players on January 23-24.
- Why: Traced: recruiting::declarations runs only on the soft date (January 6) and the deadline (January 15), in every era. The soft deadline exists only from the 2025 rules (calendar.md). Real: underclassmen announce from late November to January 15, most within days of their team's last game.
- Fix (proposed, not confirmed): Each underclassman announces on his own day after his team's last game, before the deadline.

**The college carousel fires and hires in one day** (Portal, eligibility and the draft)
- Seen: Every coach-change window still opens on the same day for every program: Dec 2, 2022; Dec 1, 2023; Dec 6, 2024; Dec 5, 2025, in all three worlds.
- Why: Traced: staff::carousel runs once, the Sunday after rivalry week, and makes every firing and every hire that day. Real (from memory, to verify): searches run one to three weeks, so each school's window starts on its own day.
- Fix (proposed, not confirmed): Give each opening its own search length, so each hire, and that program's window five days later, falls on its own day.

**Opt-outs jump at 2018 and then stay flat or fall** (Portal, eligibility and the draft)
- Seen: Opt-outs: 9 / 5 / 9 in 2016, 8 / 9 / 6 in 2017, then 14-30 / 14-29 / 17-29 a year in 2018-2025. The 2022-2025 averages are 16 / 22 / 22, against 26 / 28 / 28 before the fixes. Fix B7 rightly removed underclassmen with no plan to declare.
- Why: Traced: college::opt_outs multiplies by 0.05 up to 2015, 0.4 in 2016-2017 and 1.0 from 2018. Real (from memory, to verify): a handful in 2016, rising every year to several dozen draft opt-outs a bowl season by 2021-2023.
- Fix (proposed, not confirmed): Replace the two steps with a yearly ramp from 2016 that keeps rising into the 2020s.

**The spring window draws almost nobody** (Portal, eligibility and the draft)
- Seen: Spring windows (2023 / 2024 / 2025 springs): 60 / 53 / 65, then 58 / 51 / 48, then 54 / 59 / 49 entries, about 4-5% of each cycle.
- Why: Traced: the spring branch of recruiting::portal_open takes only backups who lost the spring battle, at a third of the winter rate (Claude's pick, not confirmed). Real (from memory, to verify): the April windows of 2023-2025 drew roughly a quarter of each cycle's entrants.
- Fix (proposed, not confirmed): Raise the spring rate to the real share of a cycle.

**Before 2018, most transfers happen on one day** (Portal, eligibility and the draft)
- Seen: In the release era (to 2017), 70-85% of each winter's transfers enter on the first day of the stretch (the first Monday in December). In 2018-2021 the first day still takes 47-63%.
- Why: Traced: recruiting::portal_open spreads the entry day by len x (1 - q)^2 x a draw, so better players cluster on day 1 in every era. Real (from memory, to verify): release-era and open-portal transfers were spread over the offseason, with waves after the regular season and after the bowls.
- Fix (proposed, not confirmed): Before the 2022 windows, spread entries over the whole offseason, with waves after each team's last game.

**No spring entries in the open-portal years (2018-2021)** (Portal, eligibility and the draft)
- Seen: No portal entries fall in March to May for the 2018-2021 seasons in any world. Spring entries start only with the 2023 spring window.
- Why: Traced: clock::portal_windows gives the OpenPortal and Permission eras one December-January stretch only. Real (from memory, to verify): the portal was open all year from October 2018, and many players entered after spring practice, before the May 1 date for playing that fall.
- Fix (proposed, not confirmed): Add a spring entry wave for the 2018-2021 seasons, taken from players who lost the spring depth battle.

**High school playoffs are 3 rounds stretched over 4 to 7 weeks** (NFL, high school and weather, serious)
- Seen: Unchanged: every classification sends 8 teams to a 3-round playoff spread over the state's real window. 33 of 51 states have 2 or more idle weeks between rounds (Texas plays Nov 14, Nov 28 and Dec 19; New York has 4 idle weeks). About 19% of teams reach the playoffs (Florida 10% with its 7 classes, Texas 8%). Real: Illinois takes 384 of 529 programs and North Carolina 48 per class, one round a week (docs/research/hs-playoff-formats.md).
- Why: Traced: hs_year::seed_playoffs cuts the field to 8 and state_dates puts the three rounds at the start, middle and end of the window. The 8-team field is a standing proposed, not confirmed pick (realism question 2).
- Fix (proposed, not confirmed): Play one round each week from the state's real playoff start to its final, with the field size that number of rounds needs.

**Hurricanes are too rare and miss Florida and October** (NFL, high school and weather, serious)
- Seen: Seasons with college games moved or cancelled, 1996-2025: 6 / 8 / 6 of 30. Florida games moved or cancelled: 0 / 8 / 0. October storm events: 0 / 0 / 13 (all Georgia, world 2028); of all storm events since 1990, September holds 85 of 87 / 61 of 73 / 23 of 38. Real: roughly 15 of these 30 seasons, Florida hit most, and October storms such as Matthew 2016, Michael 2018 and Milton 2024 (from memory, to verify).
- Why: Traced: calendar/weather.rs gives all 8 hurricane states the same small chance per 4-day block (0.48% in September, 0.24% in August and October), about 7% per state per season. No fix touched it.
- Fix (proposed, not confirmed): Set each state's yearly storm chance from the real record of storms that hit football weekends, with Florida highest and October included.

**The Senior Bowl takes underclassmen** (NFL, high school and weather, new)
- Seen: 38% / 37% / 39% of Senior Bowl players to 2021, and 46% / 48% / 50% in 2022-2025, had declared early. The Senior Bowl holds 831 / 830 / 834 of the 960 first-round picks, about 28 of 32 a year. Real Senior Bowl rosters are seniors, plus a few graduated underclassmen in recent years, and many top picks play in no all-star game (from memory, to verify).
- Why: Traced: recruiting::all_star_invites lets a declared underclassman into the Senior Bowl pool when his draft read is round 3 or better.
- Fix (proposed, not confirmed): Invite to the Senior Bowl only players whose college eligibility is over.

**Long snappers still take about five All-American Bowl places a class** (NFL, high school and weather, new)
- Seen: Long snappers hold 103 / 122 / 97 All-American Bowl places over classes 2001-2019, about 5 a class of 100, all rated 3 stars by the services. They are 140 / 154 / 120 of the 977 / 1,006 / 965 archived All-American and Polynesian Bowl players of classes 2001-2021. Before the fixes they held 3,714 of 4,121 (world 2026). Real All-American Bowl rosters carry about one snapper a team (from memory, to verify).
- Why: Traced: hs_year::all_star_game ranks players with 3+ services' stars by stars + 0.05 x overall + noise, with no places by position. A camp-converted snapper rated 80-87 gains about 4 points from his overall, so he outranks most 3-stars and some 4-stars. Inferred: the archived file keeps every camp-rated snapper but not every 3-star, so a share of archived players overstates the snappers; the per-class count is the fair measure.
- Fix (proposed, not confirmed): Give the high school all-star rosters set places by position, with one or two places each for kickers, punters and snappers.

**Christmas games on a weekday follow one number per era** (NFL, high school and weather, new)
- Seen: From 2016 every Christmas from Wednesday to Monday gets 2 games (to 2019) or 3 (from 2020); a Tuesday gets 0. Before 2016 only a Monday Christmas has a game (Monday night). So 2019 has 2 (real 0), 2020 has 3 (real 1), 2021 has 3 (real 2) and 2024 has 3 (real 2), while 2006 has 1 (real 2) and 2009 and 2010 have 0 (real 1 each). Real counts are from memory, to verify; real.json gives 2-3 a season for 2020-2025. No world exports more Christmas games than the rule gives.
- Why: Traced: nfl_year::formula_slots weekday_xmas sets the same windows (4:30pm and night, plus 1pm from 2020) for any weekday Christmas from 2016.
- Fix (proposed, not confirmed): Give each season up to 2025 its real number of Christmas Day games, from a table by year.

**Cutdown day says 90 to 53 in every era** (NFL, high school and weather, new)
- Seen: 'Cutdown day: rosters go from 90 to 53.' in 36 of 36 years in all three worlds. Real: the offseason limit was 80 until the 2011 labor deal, and teams cut to 75 before cutting to 53 until 2017 (from memory, to verify).
- Why: Traced: nfl_year.rs writes 90 as a fixed number; only the 53 comes from the rule book.
- Fix (proposed, not confirmed): Take the offseason limit and the cutdown steps from the rule book by era.

**Some NFL and all-star dates are still a week off (bundle, no design choice)** (NFL, high school and weather, new)
- Seen: Senior Bowl: Jan 29, 2022, Jan 28, 2023, Jan 27, 2024 and Jan 25, 2025, a week before the real Feb 5, Feb 4, Feb 3 and Feb 1; 6 other dates checked match. League year: Mar 11, 2020 to Mar 8, 2023, a week before the real Mar 18, 2020 to Mar 15, 2023, so the negotiating window and free agency open a week early; a Wednesday in 2013-2015 (real Tuesday) and Mar 8, 2017 (real Mar 9). HBCU Legacy Bowl: mid-January (Jan 22, 2022 to Jan 25, 2025); the real game is in late February, after the Super Bowl. All-American Bowl: the second Saturday of January (Jan 11, 2025), real the first Saturday in most years (Jan 4, 2025); first game Jan 13, 2001, real Dec 30, 2000. All real dates from memory, to verify.
- Why: Traced: anchors.rs senior_bowl = min(last Saturday of January, Super Bowl - 8), league_year = second Wednesday of March, hbcu_bowl = max(last Saturday of January - 7, title game + 5), all_american_bowl = second Saturday of January. Moving the Senior Bowl alone would put the Shrine Bowl's invites first again (see the tier item).
- Fix (proposed, not confirmed): Put each on its real day. The parts: the Senior Bowl on the Saturday 8 days before the Super Bowl every year; the league year on its real date by year; the HBCU Legacy Bowl in late February from 2022; the All-American Bowl on its real January date, with Dec 30, 2000 for the first game.

**Events that real football cancelled still happen (bundle, no design choice)** (NFL, high school and weather, new)
- Seen: The Hall of Fame Game is played and enshrinement held in 36 of 36 years, including 2011 (lockout) and 2020. The combine invites 320 in 2021. The All-American Bowl and the Shrine Bowl are played in January 2021. Real: the 2011 and 2020 Hall of Fame Games were cancelled, the 2020 enshrinement moved to August 2021, the January 2021 All-American Bowl and Shrine Bowl were cancelled and the 2021 combine had no in-person workouts (from memory, to verify). The model already drops the 2020 season's Pro Bowl and games abroad.
- Why: Traced: nfl_year::dated_events and the all-star schedules post every year; only anchors::pro_bowl and international_sites have a 2020 exception.
- Fix (proposed, not confirmed): Skip each cancelled event in its real year. The parts: the 2011 Hall of Fame Game; the 2020 Hall of Fame Game and enshrinement; the January 2021 All-American Bowl and Shrine Bowl; the 2021 combine workouts.

**Late-season flexes still fire about 4 to 5 times a season** (NFL, high school and weather)
- Seen: Total flexes 2014-2025: 6.8 / 6.5 / 7.2 a season (8.8 before the fixes), of which 2 are in weeks 5-10, so about 4.8 / 4.5 / 5.2 in weeks 11-18. In 2006-2013: 4.9 / 4.8 / 5.6 a season (5.2 before). Real seasons see about 1 to 4 flexes (from memory, to verify).
- Why: Traced: nfl_year::flex flexes in any Sunday afternoon game whose win value beats the night game's by 2, in every eligible week from week 11, with no stakes test.
- Fix (proposed, not confirmed): Flex only when the scheduled night game has lost its stakes (a team in it is out of the playoff race).

**One team can still own Sunday night** (NFL, high school and weather)
- Seen: The most flexes for one team in one season: 5 / 5 / 4 (7 before the fixes). Denver 2021 in world 2026 (weeks 5, 11, 12, 14, 17), Tennessee 2021 in world 2027 (weeks 7 and 14-17), Buffalo 2017 in world 2027 (weeks 13-16 in a row).
- Why: Traced: neither nfl_year::flex nor formula_slots counts a team's prime-time games. The real league caps a team's standalone prime-time games at about 6 or 7 a season (from memory, to verify).
- Fix (proposed, not confirmed): Cap each team's standalone prime-time games per season, in both the formula and flex.

**Combine invites are the same every year** (NFL, high school and weather)
- Seen: 320 combine invites in every year from 1995 in all three worlds (98 / 94 / 98 in 1994, none before). Real invites ran about 319-338 in 2015-2025 (from memory, to verify).
- Why: Traced: recruiting::combine_invites takes round(320 x scale).
- Fix (proposed, not confirmed): Invite every prospect above a draft-read bar, so the list moves with the class.

**The undrafted rush signs the same number every year** (NFL, high school and weather)
- Seen: 'The undrafted free agent rush: 411 players sign' in 36 of 36 years in all three worlds, and 411 really sign each year after exactly 257 picks. Real undrafted signings move with roster room and class strength (from memory, to verify).
- Why: Traced: recruiting::draft sets udfa = round(257 x 1.6) from PIPELINE, and the pool always has more than 411 left.
- Fix (proposed, not confirmed): Sign undrafted players above a lower draft-read bar, so the count moves with the class.

**Heisman finalists from one school are now common** (Stakes, news and postseason, new)
- Seen: Two or more finalists came from one school in 9 / 16 / 7 of 30 years (before: 5 / 2 / 1). World 2027 examples: 2020 Texas Tech (WR and RB), 2023 Clemson (WR and QB). Real: about 5 of the 30 seasons 1996-2025 (2004, 2005, 2016, 2019, 2020; from memory, to verify).
- Why: Traced: the new Heisman score (B10) multiplies every candidate by his team's rank factor and judges him against his own position group's top 10. So a top team's quarterback and its best back or receiver both clear the bar. Nothing splits the vote between teammates, as real voters do. The B10 fix caused this.
- Fix (proposed, not confirmed): Lower a candidate's score when a teammate ranks above him in the race.

**The title-game loser often drops below No. 2 in the final poll** (Stakes, news and postseason, new)
- Seen: Where the title-game loser lands in the final poll, of 28 title games: No. 2 in 18 / 12 / 19, No. 3 in 7 / 8 / 6, No. 4 in 3 / 4 / 3, outside the top 4 in 0 / 4 / 0. Real: the runner-up was No. 2 in the final AP poll in nearly every season from 2014 to 2024 (from memory, to verify).
- Why: Traced: release_poll puts the champion first (B2) and orders the rest by ranking(). That score uses win share, margin, schedule, the preseason view and a conference title, but not how far a team went in the postseason. This was B2's own doubt.
- Fix (proposed, not confirmed): Put the title-game loser second in the final poll.

**Some postseason dates differ from the real ones (bundle, no design choice)** (Stakes, news and postseason, new)
- Seen: Same in all three worlds. (1) Four-team semifinals: the sim always splits them over Dec 31 and Jan 1 (10 of 10 years). Real: both on one day every year (Dec 31 or Jan 1; Dec 29 in 2018 and Dec 28 in 2019). (2) 12-team quarterfinals: sim two on Dec 30 and two on Jan 1. Real 2024: one on Dec 31 and three on Jan 1 (the Sugar Bowl moved to Jan 2). (3) The 2008 and 2009 seasons' BCS title games: sim Monday Jan 12, 2009 and Monday Jan 11, 2010. Real: Thursday Jan 8, 2009 and Thursday Jan 7, 2010. (4) The 2000-2002 seasons' title bowls: sim Jan 4 every year, real Jan 3. (5) First CFP ranking: sim always the first Tuesday of November. Real: Oct 28, 2014, Oct 31, 2017, Oct 30, 2018, Oct 31, 2023, and Nov 24, 2020. All real dates from memory, to verify.
- Why: Traced: calendar/anchors.rs sets the four-team semifinals to Dec 31 and Jan 1, the quarterfinals to Dec 30 and Jan 1, the BCS title game to Jan 4 (before 2006) or the first Monday on or after Jan 6, and the first CFP ranking to the first Tuesday of November.
- Fix (proposed, not confirmed): One dates bundle with no design choice: (1) both semifinals on one real day each year; (2) the real quarterfinal days; (3) Thursday title games for the 2008 and 2009 seasons; (4) Jan 3 title bowls for 2000-2002; (5) the real first CFP ranking dates.

**The rivalry list decides who gets a huge game, not the season** (Stakes, news and postseason)
- Seen: FBS teams with a listed rivalry-week rival have at least one huge game in 99 / 99 / 100% of seasons. Teams without one have none in 83 / 83 / 83%. Rivalry week and Army-Navy make up 55 / 54 / 55% of all per-team huge games (before: 55%). 22 / 23 / 24% of those sides (14.5 / 15.4 / 16.0 a year; before: 14) belong to teams that finished with 4 wins or fewer, so no bowl was at stake.
- Why: Traced: world/college.rs stakes_for_day makes a rivalry-week (or Army-Navy) game huge for both sides with no other stake. This is Claude's pick, proposed, not confirmed. T8.1 says a game is huge only when it really matters. Unchanged by the fixes.
- Fix (proposed, not confirmed): A rivalry-week game is huge for a side only when it has one more real stake, the rule other rivalries already use.

**NFL stakes come all or nothing** (Stakes, news and postseason)
- Seen: Huge games per NFL team-season: none 44.9 / 41.8 / 42.7%, four (the most the window allows) 19.8 / 22.4 / 21.5%. Only 35 / 36 / 36% land in the agreed 1 to 3 (before: 35%). No NFL regular-season game is huge before the last four weeks.
- Why: Traced: world/nfl_year.rs stakes returns early unless the week is one of the last four. From then on, a team within one win of the playoff line is huge every week. T8.1 says most seasons have 1 to 3 and some have none.
- Fix (proposed, not confirmed): Count an NFL game as huge for a team only when its result can clinch or end that team's playoff hopes.

**Most game results land in the inbox** (Stakes, news and postseason)
- Seen: T8 makes Medium a message in your inbox. 62 / 62 / 61% of results with an FBS team reach Medium or louder, 60 / 59 / 59% of FCS results and 49 / 49 / 49% of D2 results. College results are 80 / 80 / 80% of all Medium events (1,333 / 1,324 / 1,326 a year; before: 72% of 1,851, the share grew only because the recruiting fix cut other Medium notes). D2 games with an out-of-state visitor reach it 61 / 60 / 60% of the time, in-state games 21 / 23 / 23%. FCS: 64 / 64 / 64% against 27 / 28 / 28%.
- Why: Traced: stakes_for_day makes any game with any tag Medium. Inferred: the hometown tag drives most of it, since it can only fire for an out-of-state visitor. Tags are not exported, so this cannot be checked directly.
- Fix (proposed, not confirmed): Story tags alone (hometown, revenge, coach's return, milestone) never lift a game result above Small.

**Continue stops on every team's Big moments** (Stakes, news and postseason)
- Seen: Now that Big stops Continue, 23.9 / 23.7 / 23.7 stop days a year have only college or high school Big moments (before, with Huge only: 11.4 in world 2026). In world 2026, about 89 Big college results a year fall on about 23 days, mostly fall Saturdays, so an NFL manager would stop on most fall Saturdays for other teams' games.
- Why: Traced: world/clock.rs continue_until_needed reads only an event's loudness. Events do not carry whose moment they are (there is no human team yet). The C5 fix doubled this.
- Fix (proposed, not confirmed): Each event carries the teams and level it matters to, and Continue stops on Big moments for the manager's own team or level and on Huge moments anywhere (proposed, not confirmed).

**Bowl loudness ignores which bowl it is** (Stakes, news and postseason)
- Seen: Major bowls outside the playoff (the BCS four before 2014, the New Year's Six after) were only Medium 15 / 13 / 16 times. Lesser bowls were Huge 26 / 28 / 28 times (before: 14 and 24).
- Why: Traced: stakes_for_day scores a bowl only from a ranked meeting and story tags. Nothing in it stands for the bowl's stature.
- Fix (proposed, not confirmed): Let a bowl's tier set its loudness range: major bowls Big or Huge, other bowls Medium or Big.

## Slumps

From every season row on active players and alumni (374,716 / 374,668 / 374,278 rows with a start and end rating; 311,002 / 311,292 / 310,763 played). Rating drops of 2+ points: 24 / 23 / 20 (pre-fix 46 / 46 / 44 of about 530,000 rows); per 100,000 rows 6.4 / 6.1 / 5.3 against 8.7 / 8.7 / 8.3. Form slumps of 5+ below the season-start rating: 12,756 / 12,688 / 12,763, which is 4.10 / 4.08 / 4.11% of played seasons (pre-fix 20,511 / 20,246 / 20,454, 4.43 / 4.37 / 4.42%). Seasons losing more than 1 point: 0.10 / 0.11 / 0.11% (pre-fix 0.15 / 0.15 / 0.16%). The export shrank because the star fix cut the 4- and 5-star alumni (specialists fell from 12.2% of rows to 3.9 to 4.0%, and they had been 15 / 18 / 10 of the pre-fix drops), so compare rates, not counts. Every 2+ drop has a recorded cause; 180 / 187 / 184 form slumps of 5+ (1.4%) have none. No row is at 30 or older (no pro seasons in the export). Active players' years are exact; alumni years are class + age - 18 (about). Recent seasons are over-represented, because every active player is exported but only drafted, blue-chip and camp-rated alumni.

**Judgement.** Mostly believable. Every big rating drop is a major injury (5 to 12 games missed) on top of a form slump, or a slump alone, and every big form slump is a slump (4 to 9 points off his play), often with a first-year dip (up to 7 points) at 18. Too harsh: kickers and punters. A slump alone costs them 2.1 to 2.6 points (Isaiah Brown, Exodus Jackson, Brian Trawinski, George Vallillo, Michael Wade, Manuel Garcia, Joseph Torres), because the slump takes 0.5 to 3.0 off every skill and their rating is nearly all skill (traced, development.rs). Specialists are 4% of rows but 15 to 26% of the 2+ drops. Too soft: major injuries. The worst drop in 90 world-years is 2.9 points (Eugenio Mascorro, CB, 21, 12 games missed); major-injury seasons rise a median +0.6, because the half-rate growth of a young player outweighs the 1.5 to 5.0 points taken off his speed, acceleration, agility and jumping. Odd but explained: camp-rated long snappers rated 80 to 87 at 18 fill 4 of the 10 biggest form slumps in worlds 2026 and 2028; they still played 64 to 70, so the size of the slump comes from how high their rating starts. New since the fixes: a spring step raises the slump chance of the very next fall (8.4 to 9.3% against 5.0 to 5.1%; college 12%), and freshmen who play a full season as camp redshirts can never slump or break out, because their row still reads redshirt (see the negatives). De'Shawn Johnson (world 2028) missed 11 of 12 games yet shows a normal season of play: the form figure ignores games missed, so read form slumps only for players who played.

### World 2026: the 10 biggest rating drops

| Player | Pos | Age | Level | Role | Year | Rating | Why |
|---|---|---|---|---|---|---|---|
| Chase Marshall | WR | 19 | SmallCollege | backup | 2025 | 45.8 to 43.2 (-2.6) | form slump; major injury (7 games missed) |
| Logan Miller | WR | 21 | SmallCollege | backup | 2025 | 43.7 to 41.2 (-2.5) | form slump; major injury (9 games missed) |
| Micah Green | WR | 20 | FCS | backup | 2025 | 51.6 to 49.1 (-2.5) | form slump; major injury (9 games missed) |
| Isaiah Brown | K | 16 | HS | starter | 2021 | 42.8 to 40.3 (-2.5) | form slump |
| Joniel Williams | CB | 15 | HS | starter | 2020 | 35.2 to 32.7 (-2.5) | form slump; major injury (7 games missed) |
| Justin Cage | RB | 19 | SmallCollege | backup | 2025 | 44.9 to 42.4 (-2.5) | form slump; major injury (10 games missed) |
| Exodus Jackson | K | 20 | FBS | rotation | 2025 | 56.2 to 53.9 (-2.3) | form slump |
| Andrew Campbell | WR | 20 | FBS | backup | 2024 | 56.4 to 54.1 (-2.3) | form slump; major injury (10 games missed) |
| Isiah Jones | CB | 19 | SmallCollege | backup | 2025 | 45.3 to 43.0 (-2.3) | form slump; major injury (8 games missed) |
| Dominick Rogers | CB | 17 | HS | starter | 2022 | 33.1 to 30.9 (-2.2) | stagnation; major injury (8 games missed) |

### World 2026: the 10 biggest form slumps

| Player | Pos | Age | Level | Role | Year | Rated, played | Why |
|---|---|---|---|---|---|---|---|
| Stefan Ewing Jr. | EDGE | 18 | FBS | starter | 1992 | 52.1, played 35.1 (-17.0) | form slump; first year at this level (dip 0.83, about 5.8 off his play) |
| Phoenix Sturgis-Jones | OT | 18 | FBS | starter | 2023 | 55.2, played 38.9 (-16.3) | form slump; moderate injury (4 games missed); first year at this level (dip 0.88, about 6.2 off his play) |
| Brady Zollinger | LS | 18 | FCS | rotation | 2024 | 81.3, played 65.3 (-16.0) | form slump; first year at this level (dip 0.61, about 4.3 off his play) |
| Jonathan Porterfield | S | 18 | FBS | starter | 1991 | 48.0, played 32.0 (-16.0) | form slump; major injury (8 games missed); first year at this level (dip 0.78, about 5.5 off his play) |
| Jeffrey Chung | LS | 18 | FBS | rotation | 2022 | 81.6, played 65.8 (-15.8) | form slump; first year at this level (dip 0.76, about 5.3 off his play) |
| Kaden Foster | LS | 18 | SmallCollege | rotation | 2022 | 80.8, played 65.1 (-15.7) | form slump; first year at this level (dip 0.71, about 5.0 off his play) |
| Samuel Dillon Jr. | LS | 18 | SmallCollege | rotation | 2024 | 81.8, played 66.1 (-15.7) | form slump; first year at this level (dip 0.52, about 3.6 off his play) |
| Kaleb Matthews | EDGE | 20 | SmallCollege | backup | 2024 | 47.8, played 32.6 (-15.2) | form slump; first year at this level (dip 0.23, about 1.6 off his play) |
| Stephen Chrisp | S | 14 | HS | backup | 2019 | 23.2, played 8.3 (-14.9) | form slump; spring step (+6.7) |
| Luke Stephenson | CB | 17 | HS | starter | 2022 | 44.4, played 29.6 (-14.8) | form slump |

### World 2027: the 10 biggest rating drops

| Player | Pos | Age | Level | Role | Year | Rating | Why |
|---|---|---|---|---|---|---|---|
| Brian Trawinski | K | 15 | HS | starter | 2021 | 37.1 to 34.5 (-2.6) | form slump; poor coaching (4.8) |
| Jack McGrail | WR | 17 | HS | starter | 2024 | 41.6 to 39.0 (-2.6) | form slump; major injury (5 games missed) |
| Adam Riggans III | RB | 19 | FBS | backup | 2024 | 59.8 to 57.3 (-2.5) | form slump; major injury (9 games missed) |
| Chase Coleman | CB | 17 | HS | starter | 2013 | 48.6 to 46.3 (-2.3) | form slump; major injury (10 games missed) |
| Marquay Hudson | EDGE | 16 | HS | starter | 2009 | 56.8 to 54.6 (-2.2) | form slump; major injury (7 games missed) |
| George Vallillo | P | 19 | SmallCollege | rotation | 2023 | 49.1 to 47.0 (-2.1) | form slump |
| Zane Greaves | RB | 20 | FBS | backup | 2025 | 61.5 to 59.4 (-2.1) | body stopped growing; form slump; major injury (9 games missed) |
| Jeremiah Robinson | CB | 19 | SmallCollege | backup | 2024 | 50.1 to 48.0 (-2.1) | form slump; major injury (11 games missed) |
| Michael Wade | P | 15 | HS | rotation | 2021 | 38.7 to 36.6 (-2.1) | form slump |
| Manuel Garcia | K | 16 | HS | starter | 2023 | 39.5 to 37.4 (-2.1) | form slump |

### World 2027: the 10 biggest form slumps

| Player | Pos | Age | Level | Role | Year | Rated, played | Why |
|---|---|---|---|---|---|---|---|
| Robert Carter | WR | 18 | FBS | rotation | 1991 | 46.8, played 27.0 (-19.8) | form slump; moderate injury (2 games missed); first year at this level (dip 1.00, about 7.0 off his play) |
| Robert Kaper | S | 18 | FBS | rotation | 1992 | 47.5, played 31.4 (-16.1) | form slump; minor injury (2 games missed); first year at this level (dip 0.74, about 5.2 off his play) |
| Miguel Navarette | CB | 18 | SmallCollege | rotation | 2014 | 48.0, played 32.1 (-15.9) | form slump; first year at this level (dip 0.65, about 4.5 off his play) |
| Peter Sprecher | S | 15 | HS | starter | 2019 | 32.3, played 16.5 (-15.8) | form slump |
| MC Lopez | CB | 19 | SmallCollege | backup | 2024 | 39.6, played 24.6 (-15.0) | form slump |
| Herman Thomas | IOL | 18 | FBS | rotation | 2000 | 61.7, played 46.7 (-15.0) | form slump; minor injury (2 games missed); first year at this level (dip 0.66, about 4.6 off his play) |
| Jorge Agosto | K | 18 | FCS | rotation | 2023 | 41.4, played 26.5 (-14.9) | form slump; spring step (+1.8) |
| Corde Connell Jr. | P | 18 | SmallCollege | rotation | 2025 | 41.2, played 26.4 (-14.8) | form slump; first year at this level (dip 0.42, about 2.9 off his play) |
| Daniel Ulibarri | S | 20 | FCS | starter | 2024 | 50.8, played 36.0 (-14.8) | form slump |
| Finn Michaels | LB | 16 | HS | starter | 2021 | 47.8, played 33.0 (-14.8) | form slump |

### World 2028: the 10 biggest rating drops

| Player | Pos | Age | Level | Role | Year | Rating | Why |
|---|---|---|---|---|---|---|---|
| Eugenio Mascorro | CB | 21 | FBS | backup | 2025 | 60.9 to 58.0 (-2.9) | form slump; major injury (12 games missed) |
| Isaiah Wilson III | WR | 19 | FBS | backup | 2019 | 63.6 to 60.9 (-2.7) | form slump; major injury (7 games missed) |
| Joseph Torres | P | 18 | SmallCollege | rotation | 2022 | 34.8 to 32.5 (-2.3) | form slump; first year at this level (dip 0.21, about 1.5 off his play) |
| Sebastian Terry | CB | 15 | HS | starter | 2019 | 41.0 to 38.8 (-2.2) | form slump; major injury (8 games missed) |
| Zachary Imrie | CB | 20 | SmallCollege | backup | 2025 | 52.0 to 49.8 (-2.2) | form slump; major injury (7 games missed) |
| Cosme Espinal | S | 20 | FCS | backup | 2025 | 46.5 to 44.3 (-2.2) | form slump; major injury (7 games missed) |
| De'Shawn Johnson | RB | 22 | FBS | starter | 2012 | 70.9 to 68.7 (-2.2) | major injury (11 games missed) |
| William Thompson | RB | 20 | FBS | starter | 2013 | 71.4 to 69.2 (-2.2) | form slump; major injury (10 games missed) |
| Jaden Malloy | RB | 20 | FCS | backup | 2025 | 50.8 to 48.6 (-2.2) | form slump; major injury (10 games missed) |
| Harry Powell | EDGE | 17 | HS | starter | 2024 | 43.3 to 41.1 (-2.2) | form slump; major injury (7 games missed) |

### World 2028: the 10 biggest form slumps

| Player | Pos | Age | Level | Role | Year | Rated, played | Why |
|---|---|---|---|---|---|---|---|
| Ryan Robrahn | LS | 18 | FCS | rotation | 2022 | 82.3, played 64.6 (-17.7) | form slump; first year at this level (dip 0.83, about 5.8 off his play); spring step (+0.8) |
| Thomas Bowman | LS | 18 | FBS | rotation | 2023 | 86.9, played 70.1 (-16.8) | form slump; first year at this level (dip 0.77, about 5.4 off his play) |
| Darien Humber | LS | 18 | SmallCollege | rotation | 2024 | 83.0, played 66.6 (-16.4) | form slump; first year at this level (dip 0.72, about 5.0 off his play) |
| Roman Staton | EDGE | 18 | SmallCollege | starter | 2024 | 44.7, played 28.7 (-16.0) | form slump; first year at this level (dip 0.67, about 4.7 off his play) |
| Logan Doehring | LS | 18 | SmallCollege | starter | 2025 | 80.4, played 64.5 (-15.9) | form slump; first year at this level (dip 0.63, about 4.4 off his play) |
| Israel Jackson | RB | 21 | FCS | rotation | 2025 | 57.1, played 41.3 (-15.8) | form slump |
| Jacob Moore | IOL | 18 | SmallCollege | rotation | 2025 | 41.0, played 25.2 (-15.8) | form slump; first year at this level (dip 0.66, about 4.6 off his play) |
| Willie Beckett | S | 19 | SmallCollege | backup | 2023 | 45.1, played 29.4 (-15.7) | form slump |
| Seth Moses | CB | 21 | SmallCollege | backup | 2025 | 50.9, played 35.4 (-15.5) | form slump |
| Joseph Mondragon | S | 19 | FCS | rotation | 2023 | 47.6, played 32.3 (-15.3) | form slump |

## Spot checks (12)

A skeptic re-measured each key number from the raw output. 9 held; 3 needed a correction, applied everywhere above.

- **Held.** Every signing class has exactly 33 / 350 / 2,000 / 4,000 5/4/3/2-stars; final 4-stars of classes 1997-2019 drafted 25.0 / 24.8 / 24.4% (real 24.4%, pre-fix 14.3% in world 2026); 3-stars 4.5 / 4.5 / 4.5% (real 6.0%); the dashboard gives 25.1 / 25.0 / 24.6% for 4-stars.
  Result: meta.json signing_classes holds 30 classes (1997-2026) in each world, and every one is exactly 33 / 350 / 2000 / 4000. The engine counts only non-specialists (recruiting.rs line 2183: `!is_specialist()`). Rerunning rc_draft_by_star.py gives 4-stars 25.0 / 24.8 / 24.4% (8107 / 8129 / 8111 recruits), 3-stars 4.5 / 4.5 / 4.5%, and pre-fix 2026 4-stars 14.3%. The dashboard figures in regress output are 25.14 / 24.99 / 24.62%. Side note: alumni counts by class_of run 350-354 4-stars a class, because reclassified players shift class, so the 4-star denominator is about 8,107 rather than 23 x 350. The dashboard's 3-star rate (4.69 / 4.66 / 4.67%) includes kickers' camp 3-stars in the numerator; the script excludes them, which matches meta. Neither point changes the claim.
- **Held.** In logs 1996-2016, committed recruits who signed nowhere fell from 46.5% / 46.4% / 46.2% of verbal commitments to 1.1% / 1.1% / 1.1%. Flips fell from 21.6% / 21.6% / 21.8% to 6.9% / 7.1% / 7.0%. The world 2026 flip rate is corrected here from the checker's 7.0%.
  Result: Re-ran wf2/recruiting/rc_commit_timing.py on all six worlds (pre-fix with RC_PREFIX=1). The 'logs 1996-2016' lines match exactly. Fixed: flips 10,098 (6.9%) / 10,269 (7.1%) / 10,273 (7.0%); signed nowhere 1,571 / 1,553 / 1,541 (1.1% each). Pre-fix: flips 32,433 (21.6%) / 32,445 (21.6%) / 32,598 (21.8%); signed nowhere 69,592 (46.5%) / 69,773 (46.4%) / 69,098 (46.2%). The 'decommits' counter (recruiting.rs, signing stage) counts a recruit who was committed but signed nowhere at that stage, so the wording fits. World 2026 is 6.9%, as the claim says.
- **Corrected.** An FBS national champion is crowned in 30 of 30 seasons in each world, with head-coach credit every season; pre-fix 16 / 16 / 16 seasons (1998-2013) had none.
  Result: Rerunning champions.py gives the same output: 30/30 champions and 30 'wins the national title' lines per world. Pre-fix has 16 seasons with no champion in all three worlds (1998-2013 each), and the 1996-97 flags are the polls-only years. But the script's credit check is loose: it accepts any stint with titles >= 1, and coach.titles also counts conference titles (college.rs line 1972, best_title != None). A strict check compares each program season's 'coach' id with stint titles against the nat/conf seasons he coached. When the season's coach retired or was fired in December, before the January title game, the title (and the playoff wins) went to the successor, who had not coached that season. World 2026: 2002 Georgia (Hazard to Robson), 2020 Miami (MJ Wood fired after a 12-2 title season, credit to Charles Allen), 2021 Ohio State (Fabrizio to Garrett Moore, whose stint shows 5 titles for 4 own title seasons and 56 wins for 53). World 2027: 2011 Georgia (Grimes to Koons) and 2024 Georgia (Koons to Echols III). World 2028: none. Coaches who 'left for another job' kept their credit. Correction applied: Champion crowned in 30/30 seasons per world (pre-fix 16/16/16, 1998-2013) holds. Head-coach credit lands every season, but in 3 / 2 / 0 seasons it goes to the successor hired in December, not the coach who won it (the coach retired or was fired before the January title game). The coach who led the champion is credited in 27 / 28 / 30 seasons.
- **Held.** FBS true freshmen who play: a median of 1,384 / 1,379 / 1,389 a year in 1997-2017 (pre-fix 69), and 1,342.5 / 1,373.5 / 1,338 in 2018-2025.
  Result: Recomputed from raw player_seasons and programs, using rows with year == class_of at an FBS program. Fixed medians: 1997-2017 are 1,384 / 1,379 / 1,389, and 2018-2025 are 1,342.5 / 1,373.5 / 1,338. No row has g == 0, so every counted row is a player who played. Pre-fix medians for 1997-2017 are 69 / 66 / 69: the claim's single 'pre-fix 69' fits worlds 2026 and 2028, and world 2027 is 66. true_freshmen2 output: 1,381 / 1,377 / 1,395. Precision applied: Optional precision: pre-fix 69 / 66 / 69 (one figure per world).
- **Held.** Heisman winners over 30 years: QB 23 / 18 / 21, RB 5 / 3 / 7, WR 1 / 8 / 2 (pre-fix RB 28 / 30 / 28); 2 / 1 / 2 players win it twice.
  Result: Rerunning awards.py gives output identical to the saved files: 2026 QB 23, RB 5, TE 1, WR 1; 2027 QB 18, WR 8, RB 3, TE 1; 2028 QB 21, RB 7, WR 2. Pre-fix runs give RB 28 (+2 QB), RB 30, RB 28 (+2 QB). I cross-checked positions against alumni records that carry a 'Won the Heisman Trophy' note: they agree, and the 1-2 winners per world still active with no notes are labelled by their finalist line. The repeat winners are each a single player whose alumni notes show two wins: Donald Sutton and Steven Edwards (2026), Jessie Whichard (2027), Maurice Shields and Shawn Evans (2028).
- **Corrected.** In world 2026 alumni, 11,303 notes reading 'Played in N games at A, which cost him his redshirt year' with N of 5-12 sit on the age A-1 college row. Each of those rows has role 'redshirt' and no performance (perf null).
  Result: redshirt_rows.py reproduces notes 21,622, matched 12,864 (all redshirt with perf null), and 11,303 with N >= 5. With 'game' singular also allowed, the counts are 22,628 notes and 13,407 matched, which is the world checker's figure; the 543 extra matches are 1-game notes. All unmatched notes belong to alumni with no detail seasons. The age A-1 row is the right season: player_seasons for year class_of + A - 19 has g equal to N in 6,856 of 10,278 cases and within 2 in 9,770, better than the neighbouring years. But N is not 5-12. The 11,303 notes run from N=5 to N=16: 5:385, 6:301, 7:318, 8:254, 9:461, 10:339, 11:3,422, 12:3,175, 13:2,182, 14:452, 15:11, 16:3. Only 8,655 have N of 5-12. Correction applied: 11,303 notes with N of 5-16 (8,655 with N 5-12; 2,648 with N 13-16) sit on the age A-1 college row, and every one is role 'redshirt' with perf null.
- **Held.** FBS true freshmen with 5+ games, median a year: 871 / 865 / 877 in 1997-2017 against 132.5 / 135.5 / 147.5 in 2018-2025; world 2026 had 821 in 2017 and 153 in 2018.
  Result: Rerunning true_freshmen2.py on all three worlds (full player_seasons stream) gives output identical to the saved files: medians 871 / 865 / 877 against 132.5 / 135.5 / 147.5, and world 2026 has 821 in 2017 and 153 in 2018. The games spread confirms the cause in world 2026. In 2017, 389 freshmen played 13 games and 210 played 12. In 2018, 961 played exactly 4 games and only 153 played 5+ (2019: 940 at exactly 4). About 9-10% of FBS true freshmen with any games play 5+ from 2018 on.
- **Held.** Up to 2021 the Shrine Bowl took 572 / 558 / 529 players later picked 33-100. In 2022-2025 it had only 4 / 8 / 14 such players, and 63% / 66% / 64% of its NFL-bound players went undrafted (world 2026: 232 undrafted of 366).
  Result: Re-counted from alumni.jsonl, where pick is the overall pick (max 257). Up to 2021, picks 33-100 were 572 / 558 / 529. For 2022-25 they were 4 / 8 / 14, and undrafted was 232/366 (63.4%), 230/351 (65.5%) and 221/347 (63.7%). Wording nuance: these shares are of all Shrine players. Of the undrafted, 230 / 228 / 220 signed as undrafted free agents and 2 / 2 / 1 went unsigned. On a strict NFL-bound basis the shares are 230/364 = 63%, 228/349 = 65% and 220/346 = 64%. Precision applied: Optional wording: '63% / 66% / 64% of its players went undrafted (232 of 366 in world 2026; all but 2 / 2 / 1 of them signed as undrafted free agents)'. A strict NFL-bound share is 63% / 65% / 64%.
- **Held.** 489 / 463 / 468 recruits were called 5-stars in summer or fall commitment news (classes 1997-2021); only 25 / 28 / 27% ended as 5-stars, and 38 / 36 / 35% ended at 3 stars or fewer (7 / 10 / 7 unranked).
  Result: The script reproduces: 489 / 463 / 468 news 5-stars. Final 5-star share is 122 (24.9%) / 128 (27.6%) / 127 (27.1%). Three stars or fewer is 187 (38.2%) / 169 (36.5%) / 162 (34.6%), with final 0 at 7 / 10 / 7. No alumni page has two 'Committed to' notes, so matching only the first note loses nothing, and none of the unmatched names appear in alumni or people within 2 classes. Caveat: class 2021 is barely archived (alumni hold 22 5-stars and 78 4-stars for that class, people.jsonl only 3 players), so 9 / 11 / 11 of its roughly 20 news 5-stars count as '3 stars or fewer' by default. Restricted to classes 1997-2020: 468 / 443 / 450 recruits, 24.6 / 27.8 / 27.6% ended as 5-stars, 37.6 / 35.7 / 33.6% at 3 stars or fewer. The world 2027 share is 36.5% on the script's own numbers (the 36% comes from adding rounded buckets). The finding stands within about 1 point.
- **Held.** The same player wins the same award in back-to-back years 42 / 42 / 41 times over the 8 awards in 30 years. The Doak Walker alone repeats 9 times in world 2026.
  Result: Recomputed from calendar_events 'College football awards:' texts, years 1996-2025 (meta first = 1996): one show a year, all 8 awards parsed every year. Back-to-back repeats are 42 / 42 / 41, the same whether matched by name or by name plus school. World 2026 by award: Doak Walker 9, Mackey 7, Groza 6, Outland 5, Butkus 4, Thorpe 4, Biletnikoff 4, O'Brien 3. Note: if 1991-1995 are included the totals are 52 / 48 / 46, but the claim's 30-year window is correct.
- **Corrected.** 90 / 95 / 89 of 103 cards have a note age that goes backwards (pre-fix 45 in world 2026); 12,087 / 12,067 / 12,171 alumni pages read 'Signed with ... at 18' and then 'Played in his state's high school all-star game at 17'.
  Result: Rerunning cards_ages.py confirms the cards: 90 / 95 / 89 of 103, and pre-fix 2026 has 45. Rerunning rc_note_ages.py reproduces 12,087 / 12,067 / 12,171 on line 22. But those are counts of adjacent recruiting-note pairs in which a sign note at any age is followed by a state all-star note, and the sign notes include 'Walked on at' notes. Counting pages directly (classes 1997-2024): 'Signed with ... at 18' before 'Played in his state's high school all-star game at 17.' is 11,594 / 11,530 / 11,685. Adding walk-on pages gives 12,133 / 12,084 / 12,199, with 539 / 554 / 514 walk-on-only pages. Any signing or walk-on note followed by a younger state all-star note gives 12,223 / 12,186 / 12,294. Correction applied: Cards part holds (90 / 95 / 89 of 103; pre-fix 45). Pages: 11,594 / 11,530 / 11,685 alumni pages read 'Signed with ... at 18' and then the state all-star game at 17. With walk-on pages ('Walked on at ... at 18') it is about 12,100-12,200 per world (12,133 / 12,084 / 12,199). The 12,087 / 12,067 / 12,171 figures are the script's pair counts, not 'Signed with' pages.
- **Held.** There are 24 / 23 / 20 rating drops of 2+ points. The biggest is Eugenio Mascorro (CB, 21, FBS backup, world 2028, about 2025): 60.9 to 58.0, from a form slump plus a major injury (12 games missed). The biggest form slump is Robert Carter (WR, 18, FBS rotation, world 2027, about 1991): rated 46.8, played 27.0, from a form slump, a first-year dip of 1.00 and a moderate injury.
  Result: Recounted from the people and alumni detail.seasons rows, both with and without the script's name+class dedup (about 560 duplicate name+class records exist, and none of them holds a 2+ drop). Drops of 2+: 24 / 23 / 20. Biggest drop across the worlds: Mascorro in world 2028, -2.9 (60.9 to 58.0), events slump and majorInjury, 12 games missed; the other worlds peak at -2.6. Biggest form slump: Robert Carter in world 2027, class 1991, age 18, FBS rotation, start 46.8, perf 27.0 (form -19.8), events slump, moderateInjury and firstYear, dip 1.00, 2 games missed; the other worlds peak at -17.0 and -17.7. The final.json slumps block equals world/result.json. Caveat: Carter's 1991 season falls in the warm-up years before the recorded window (meta first = 1996).

## Speed

Each 30-year world took 2,492 / 2,478 / 2,493 s wall (about 41 minutes), against 2,287 / 2,404 / 2,449 s before the fixes and about 1,960 s in trial 5, with the same engine work (about 83,100 games and 5.9 million team plays), so time per million plays rose from 383 to 415-417 s. The clean like-for-like 5-year A/B puts section 4 with its fixes at +6% (961 s to 1,018 s); the rest of the gap to trial 5 is mostly code merged before section 4 and machine load, and the likely costs inside section 4 (the daily clock's dated events, redshirts back on game rosters) are inferred, not profiled.

## Doubts (25)

- Reconciled here from the output files: the world 2026 flip rate is 6.9%, not 7.0% (out_rc_commit_timing_2026.txt).
- Reconciled here: the long-snapper share of high school all-stars is of archived players, not NFL-bound ones; the alumni file keeps every camp-rated snapper, so the fair figure is about 5 places a class.
- Reconciled here: A4's fallback cannot explain the smaller portal, because it places nobody; the steered-signing drop is not traced.
- Small events are not exported (visits, 4-star commitment and signing news, weekly polls, tags, withdrawals, Pro Bowl, Shrine Bowl, single NFL slots), so their timing is traced in code, not measured.
- High school games and state champions are not exported; D1 and D2 rest on code, a Python replay and the game counter.
- The alumni file holds only drafted, blue-chip and camp-rated players and shrank from about 54,600 to 26,400 after A1, so note counts compare rates, not totals, with pre-fix runs.
- Unsigned specialists can be checked only for class 2026, and blue-chip destinations for classes 2018-2021 miss players still in college.
- The export has no pro seasons, so the slump list stops at college and shows no age decline.
- Continue cannot be replayed from the export; stop days are the days with a Big or Huge event.
- B8 (the title-game withdrawal day) and B3's bye teams sitting out no playoff game are traced only.
- Traced, no case seen: the coach-change window opens Dec 1-6, before selection day, so a playoff team that changed coaches would not have its coach-change entrants held.
- The runs predate the stat-record fix (1d1a4a3): 1,633 mis-sided season rows (2%) in world 2026; movement counts were redone without them, the Heisman stat lines were not.
- Trial 5 numbers come from the pre-fix findings; the trial 5 output was not re-read.
- Pre-fix dashboard data exist only for world 2026, so some pre-fix 2027 and 2028 numbers come from the pre-fix findings.
- Many real figures are from memory, to verify: recruiting timing, redshirt use, transfers, opt-outs, early entries, 2025 class sizes, postseason and NFL dates, Christmas games, flexes, hurricanes, cancelled events, roster limits.
- tools dashboard real.json is itself partly from memory (abroad_by_year, early entrants); draft_by_star and vitals.FBS are sourced.
- The histories end in 2025, so the real 2026 schedule, nine games abroad and the 12 off-field changes (T11.2) did not run.
- Seen in passing, not measured: Missouri State is listed as FBS in 2015 (it moved up in 2025).
- Seen in passing, not measured: some college notes are out of age order (an opt-out at 22 before an 'At 21' note).
- Seen in passing, not traced: portal whispers fell from 29-63 to 17-46 a year (world 2026).
- Seen in passing, not traced: high school all-state namings fell from 16,166 to 16,057 a year although Florida gained 3 classes.
- Not traced: 3 / 6 / 6 four- and five-star cards have no offers note, and cards from classes before about 2002 have no college stats.
- bowl_line.py reads games in file order (games.jsonl has no dates), so its 2014 count of 5 teams in world 2026 is likely an artifact.
- The A/B speed run did not set AFM_PROFILE, so where the extra time goes is inferred.
- Seen while writing the notes: 4 of the 30 biggest form slumps (Stefan Ewing Jr., Jonathan Porterfield, Robert Carter, Robert Kaper, 1991-1992) fall in the spin-up years before recorded history, which the alumni pages still keep; their slumps are real engine seasons, but the years are not part of the 30-year record.

## Next

- Part A of `world-trial6-proposals.md` (17 bugs and fixes that fell short) is being built now, then one world run to measure it.
- Part B (38 items) waits for Cain's yes or no, item by item; the Debug Lab has buttons for each.
- Claude's own picks during the build are listed as "proposed, not confirmed" at the end of `docs/design/calendar-checklist.md`.
