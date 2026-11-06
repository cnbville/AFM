# What each section of the day holds

The spec is `day-sections.md` beside this file (design done 2026-11-05); it wins wherever the two differ. This map was written 2026-10-30 in the project folder (`afm/player-immersion/sections/`); its code notes are at `ca23d9b`.

This maps a real coach's day onto the game's six sections, at all three levels. It is built from real coaching days (sources below) and from how the game's clock works today (code at `ca23d9b`). Cain, 2026-10-30: "i want days split up in time, 5 AM, 8 AM, 1 PM, 4 PM , 7 PM, 10 PM. So one day turns into 6 sections". He also chose that Continue stops at every section. **Every placement here is proposed, not confirmed.** The main cases share Eastern time: Dutch Fork High (Irmo, SC), South Carolina (SEC) and the Carolina Panthers, 2026-27. NIL's quick reactions are a separate update.

Legend: [n] = source. "rule" = set by a rule. "unsure" = not confirmed: a known doubt, or sources that disagree. No tag = what programs usually do. HC = head coach, AD = athletic director.

How sure: a check on 2026-10-30 could open only three sources: the nflverse schedule [60], the time zone database [61] and a calendar. They confirm every weekday named here, the Panthers' 2026 schedule, the NFL's 2026 kickoff times and the time zone gaps. Most other source sites were blocked and the search budget was spent, so every other real time and rule here is unsure until its source is read. "unsure" in the text marks the ones with a known doubt.

## The six sections

| | Hours | Length | That part of a real football day |
|---|---|---|---|
| S1 | 5-8 AM | 3 h | Coaches in before dawn: film, staff meeting, early lifts. Players mostly asleep in season. Overnight news waits. |
| S2 | 8 AM-1 PM | 5 h | Office hours. HS: school. College: class, practice for morning programs. NFL: team meeting, installs, pressers. Noon kickoffs. |
| S3 | 1-4 PM | 3 h | HS: last classes. College: class, press conference, 3:30 kickoffs. NFL: practice, Sunday's 1 PM games. The hours before 4 PM ET deadlines. |
| S4 | 4-7 PM | 3 h | Practice at HS and afternoon colleges. NFL: the 4 PM ET results, late Sunday games. Pregame for night games. |
| S5 | 7-10 PM | 3 h | Friday HS kickoffs, college night games, NFL prime time, draft rounds 1-3. Staff meetings, film, recruiting calls. |
| S6 | 10 PM-5 AM | 7 h (8 or 6 when the clocks change) | Night games end, film and scores go out, staffs work late, insiders break hires. College players' 8 free hours (rule). |

A time on a boundary opens the next section (4:00 PM is S4). The day runs 5 AM to 5 AM, so S6 crosses midnight. The clocks change inside S6. They fall back on the night into Sun Nov 1, 2026, so that S6 runs 8 hours. They spring forward on the night into Sun Mar 14, 2027, so that one runs 6 [61]. Arizona and Hawaii don't change.

### What changes from the first table (sent 2026-10-30)

| Was | Now | Why |
|---|---|---|
| HS 1 PM: athletics period | Class to 3:40 | A Texas rule [18]; SC has none, and a weight class is the school's choice. |
| HS 5 AM: zero-hour lifting | Staff up 5:30-5:45 [9]; early lifts where offered | A zero period is a school choice, common in California [20]. |
| College 4 PM: practice | S2 or S4, by program | SC practiced 9:30 AM in spring 2026 [24]; Purdue practiced mornings under Ryan Walters (2023-24) [25], and its hour now is unsure; many SEC teams practice afternoons. |
| College 10 PM: bed check | Players' 8 free hours inside 9 PM-6 AM | That is the rule [27]; bed checks are custom the night before games. |
| NFL 10 PM: game plan into the night | Monday and Tuesday nights; other nights home 10-11 PM | Tuesday is the plan day [50]. |

## High school (Dutch Fork, Irmo, SC)

School 8:20 AM-3:40 PM (2025-26) [1], so practice starts about 4:00 (a 3:00 bell would put it in S3). The HC, Tom Knotts, is also the AD [67]: his S2 and S3 go to other sports, officials, buses and visiting college coaches. Where the HC teaches, S2 and S3 are class. Assistants are nearly all teachers. 2026: first practice Thu July 30, first games Aug 21, last regular Friday Oct 30, playoffs from Nov 6, finals Dec 4-5 (from High School Football America [66], not SCHSL).

### Game week (home, Friday 7:30 PM). The week starts the morning after the game.

| Day | S1 5-8 AM | S2 8 AM-1 PM | S3 1-4 PM | S4 4-7 PM | S5 7-10 PM | S6 10 PM-5 AM |
|---|---|---|---|---|---|---|
| Sat | Asleep (film ran to 1 AM) | Players in: key plays on film, lift, injury checks. Staff grade film [8] | Staff tag the next opponent's film [8] | Coordinators cut film. HC with family | HC on opponent film at home | Sleep |
| Sun | Off: church, family | HC drafts the week [8] | Staff meeting (2 PM in one program) [10]: plan, depth chart | Coordinators script Monday | Players' film at 6 PM in some programs [10] | HC finishes the scouting report |
| Mon | Staff up 5:30-5:45 [9] | Bell 8:20 [1]. Staff teach; HC on AD work | Class to 3:40. Film in planning periods | Practice about 4:00-6:30: install, scout film | Staff meeting, opponent film [9] | Assistants home about 11 [9] |
| Tue | In early | School. HC: parents, media, AD calls | School | Heavy day, full pads. Past 3 hours counts as two practices (rule) [2] | Staff film, adjust the plan | Sleep |
| Wed | In early | School. Previews for TV and papers | School | Shorter: situations, shells | Call sheet done | Sleep |
| Thu | Light | School. HC as AD: officials, gate, security | School | Walk-through, about an hour | JV game 6:00 or 7:30 PM (one SC school, 2019) [6]; varsity staff watch | Players rest |
| Fri | HC in early | School. No college contact with a prospect before his game (rule) [17] | Bell 3:40. Team meal, taping | Warm-up. No game starts at WBGT 92.1 or more (rule) [4] | Kickoff 7:30 PM [5]. College assistants may watch, Sept 1-Nov 29 (rule) [13] | Ends about 10. Film to Hudl, swapped with next week's opponent; stats to the paper [8]. TV highlights 11 PM (2019) [11]. Film to about 1 AM |

Real life against the docs and the code: `calendar.md:158` (confirmed) has "JV and film Saturday", and so does the game's own Monday line for the high school week (`world/clock.rs` `rhythm`). The game plays its JV and freshman games on Thursday (`world/front_office/school.rs` `sub_games`), as one SC school did in 2019 [6]. The Saturday line goes to the minor bugs list, since the code disagrees with itself.

### The rest of the year

| Key day | S1 | S2 | S3 | S4 | S5 | S6 |
|---|---|---|---|---|---|---|
| Playoff week (from Nov 6; brackets out Sunday [21]) | As a game week | School | School | Shorter, less contact | Film | Sleep |
| Early signing, Wed Dec 2 (Dec 2-4) | Opening hour unsure (the national letter ended in 2024) | Ceremonies, some at 10 AM [12]. December dead period (2026-27 dates unsure): no college coaches in person [14] | More ceremonies, some at 1 PM [12] | Practice (finals week) | Film | Sleep |
| State finals, Fri-Sat Dec 4-5 | Rest | Finals from noon Friday (2025) [7] | TV finals; bus | 4 PM final; warm-up | Night final. Dutch Fork's 2025 final: Sat 7:30 PM [7] | Bus home |
| January school day | Early lift (unsure) | College coaches visit with the principal's OK, never in class (rule, unsure) [16] | Same | Voluntary lifting, never mandatory (rule) [2] | HC as AD at winter games | Recruits post offers |
| Spring school day | Lift | Quiet period: no college visits (spring 2027's dates unsure; spring 2026's were Mar 2-Apr 14 [73]). Contact (Apr 15-May 29, 2027): assistants visit; head coaches stay on campus (rule) [15]. A pending FBS change could rewrite spring 2027 (unsure) [62] | Same | Track, baseball or lifting. May: 10 spring practices, at most 8 in full pads (rule) [2] | Spring game allowed, no admission (rule) [2] | Sleep |
| June weekday | Early workouts, before the heat | Lift, speed, 7v7 or team camps: 10 days, June 1-July 23 (rule) [3] | WBGT 90-92: 1 hour, no gear. 92.1 up: nothing (rule) [4] | Off, or evening work | Family | Sleep |
| Dead week (June 28-July 4, 2026) | No practice, lifting, meetings, film or camps (rule) [3] | Same | Same | Same | Same | Same |
| Preseason (from Thu July 30) | Arrive | Helmets only days 1-2, then shoulder pads, full gear Aug 5 (rule) [2]. No school yet, so practice is in the morning | Rest, meals, film | A second session is unsure. One SC school went 2 hours in the morning and 2 more from 6 PM in 2018 [22]. The usual heat ramp allows one practice a day in the first five days; SCHSL's rule is unchecked | Off | Sleep |

### Hard times

| Time | What | Section |
|---|---|---|
| 8:20 AM, 3:40 PM | Dutch Fork's bells [1] | S2, S3 |
| Thu 6:00 or 7:30 PM; Fri 7:30 PM; Fri 11 PM | JV (2019) [6]; varsity [5]; TV highlights (2019) [11] | S4-S5; S5; S6 |
| 3 hours | Longest session before it counts as two (rule) [2] | S4 |
| Any hour | WBGT: 87-89.9 shells only; 90-92 one hour, no gear; 92.1 up, practice and games stop (rule) [4]. Unsure: these bands match Georgia's table [68], where 87-89.9 also caps practice at 2 hours | any |
| Game day; school hours | No college contact with a prospect before his game [17]; visits only with the principal's OK, never in class [16] (rules; the visit rule is unsure, its source is a 2017-18 basketball guide) | S1-S5; S2-S3 |
| Dec 2-4, Feb 3 | Signing ceremonies, 9 AM-3:45 PM [12][40] | S2-S3 |
| Whole days | NCAA: evaluation Sept 1-Nov 29; dead from Nov 30 into January (end unsure); spring contact Apr 15-May 29 [13][14], which a pending change could rewrite (unsure) [62]. SCHSL: spring practice in May; summer June 1-July 23; dead week June 28-July 4 [2][3] | all |

### What differs by state and time zone
- Texas (UIL, mostly Central): a 60-minute athletics class in the school day plus 8 hours a week outside it [18]. Shared stadiums: varsity Thursday or Friday 7 PM, JV and freshmen Wednesday or Thursday 5 PM [19].
- Georgia (GHSA): 10 spring days, the last can be a game against another school [69]. First 5 preseason days: 2 hours, helmets only [68].
- Florida (FHSAA; the panhandle west of the Apalachicola River is Central, Tallahassee is Eastern): 20 spring sessions ending in a spring game. Heat rules by law since 2020, the Zachary Martin Act [70].
- California (CIF, Pacific): at most 2 full-contact practices a week, 90 minutes each [71]. School can't start before 8:30, but a zero period can [20]. TV games at 7 PM, 10 PM Eastern [72].

## College (South Carolina, SEC)

Rules (changes since the 2025 House settlement unchecked): players get at most 4 hours a day and 20 a week in season, a game day counts 3, one day off (SC: Monday, in 2022) [26][35]. They get 8 straight free hours inside 9 PM-6 AM, and 8 hours after a home game that ends past 9 PM (that clause unsure) [27]. Coaches have no limit. 2026: opener Sept 5, bye Oct 17, at Oklahoma Oct 31 [74].

### Game week (home, Saturday 7:30 PM). The week starts Sunday.

| Day | S1 5-8 AM | S2 8 AM-1 PM | S3 1-4 PM | S4 4-7 PM | S5 7-10 PM | S6 10 PM-5 AM |
|---|---|---|---|---|---|---|
| Sun | Players asleep: 8 hours after a late game (rule, unsure) [27]. Staff grade film | Staff meeting; injury list. Players: treatment | Players lift, fix mistakes on film. World: firings land | Practice (hour unsure) | Staff start the next plan. HC calls Saturday's visitors | HC's TV show 10:30 PM (2023) [75] |
| Mon | Staff in early | Players: class, no football. Staff: game plan | World: SEC sets kickoff times about 12 days out (hour unsure) [30] | Staff: install, recruiting meeting | Players: study hall. Staff call and text recruits | Staff finish install |
| Tue | Morning teams meet about 7 (Purdue under Walters, 2023-24) [25] | Practice for morning teams; class | HC press conference 1-1:30 PM (2024) [23] | Practice for afternoon teams | Nov 3-Dec 1: CFP rankings 7-9:30 PM [31]. Calls | Activity ends in time for the 8 free hours (rule) |
| Wed | Staff meeting | Practice (morning teams) | Class; position meetings | Practice (afternoon teams) | SEC availability report by 8 PM (rule) [28] | Staff: third down, red zone |
| Thu | Staff meeting | Lighter practice | Class | Report update after practice (rule) [28] | World: Thursday games on TV. Calls | Call sheet done |
| Fri | Players: class | Walkthrough. Away: charter flight midday | Away: walk the stadium | Report update (rule) [28]. Team hotel, home games too (Alabama [46]; SC unsure) | Dinner, meetings, snack. World: high school games | Lights out (hour unsure) |
| Sat | Players sleep; staff up | Breakfast, meetings, chapel. World: noon games [29] | Lunch, rest. World: 3:30 games | Pregame meal. Recruits arrive 2-3 hours early (2010) [47]. Walk 5:30, gates 6 [33]. Final report 6 PM (rule) [28] | Kickoff 7:30 | Ends near 11. Press, then the HC meets recruits [47]. Players' 8 hours start (rule, unsure) |

### How the week shifts

| Case | What moves |
|---|---|
| Noon kickoff | Wake about 6 (S1). Pregame meal about 8, walk about 10, kickoff (S2). Ends about 3:30, press (S3). HC with recruits (S4). |
| 3:30 kickoff | Pregame meal about 11 (S2). Walk 1:30, kickoff (S3). Game (S4). Ends about 7: press, recruits (S5). |
| Thursday 7:30 PM game | No Sunday practice, Monday off as usual [35]. First availability report Monday 8 PM (rule) [28]. Main practice Tuesday; walkthrough and travel Wednesday. Friday: players off, staff fly to recruits. |
| Bye (Oct 17) | Sunday-Monday self-scout [34]. Practice only Tuesday and Wednesday. Thursday-Friday staff on the road: school visits (S2-S3), high school practice (S4), Friday games (S5); watch only, no talking (rule) [13]. |

### The rest of the year

| Key day | S1 | S2 | S3 | S4 | S5 | S6 |
|---|---|---|---|---|---|---|
| Carousel Sunday (Nov 29, after rivalry week) | AD and president decide | The fired HC meets the AD | Moves go public. Kiffin met his Ole Miss team at 1 PM CT, then LSU named him [36] | Interim named. Players get a 15-day portal window (rule). When it opens is unsure: 5 days after a new HC is named [37], or at the hire [63] | Search firms call | Hires leak (unsure) |
| Early signing, Wed Dec 2 (Dec 2-4) [40] | Agreements arrive (hour unsure) | Signees posted one by one; HS ceremonies | HC's signing-day press conference | | Calls with signees | |
| Title Saturday (Dec 5), Selection Sunday (Dec 6) | | Sun: CFP bracket, noon-3 PM [31] | Sun: bowl bids follow | Sat: SEC title game 4 PM [29] | Sat: night title games | |
| CFP first round (Dec 18-19) | | Sat noon game [32] | Sat 3:30 game | | Fri 8 PM, Sat 7:30 games | |
| Portal day (Jan 4-13, 2027) [38] | Staff check the database | Compliance enters names within 2 business days, so they show in office hours (rule) [39] | Staff call new entries | Transfers visit | HC closes by phone | Commitments break online |
| Offseason day (winter, spring) | 6 AM workouts are common; staff meeting | Class. Spring: practice about 9:30 (SC 2026) [24] | Class, meetings. Spring Tuesdays: HC press 1 PM [24] | Out of season: 8 hours a week, 2 days off (rule) [26]. Contact period: assistants at high schools [13] | Study hall | 8 free hours (rule) |
| Official visit weekend (late May-June) | | Sat: breakfast; position coach, HC, academics | Fri: the recruit flies in. Sat: tours, photos | Sat: roster and money talk | Dinner with staff | Out with a player host |
| Fall camp, padded day | Breakfast | Practice (a 3-hour cap is unsure); one practice a day since 2017 (rule) [43] | Treatment, rest | Walkthrough, no pads or contact (rule) [43] | Install meetings | Curfew |

From 2027 (FBS committee approved; the final Cabinet vote is unsure): two out-of-season periods with 21 practices in all (at most 7 weeks together, 5 each) and fall camp of 21 practices in 27 days [41]; a 14-week season (approved) [42].

### Hard times (ET)

| Time | What | Section |
|---|---|---|
| 9 PM-6 AM | Players' 8 free hours; after a home game past 9 PM, 8 hours before the next activity (rule; that clause unsure) [27] | S5-S1, S6 |
| Tue 1-1:30 PM; Sun 10:30 PM | SC HC: press conference (2024) [23]; TV show (2023) [75]. His weekly radio show's day and hour are unsure: the only source found is Clemson's (Thu 6-7 PM) | S3; S6 |
| Wed 8 PM; Thu, Fri after practice; 90 min before kickoff | SEC availability reports (rule) [28] | S5, S4, game |
| Mondays, about 12 days out | SEC kickoff times, at least 6 days ahead [30][29] | S2-S3 (hour unsure) |
| Sat 12-1, 3:30-4:30, 6-8 PM; about 10:30 PM | SEC TV windows [29]; West Coast late window [44] | S2; S3-S4; S4-S5; S6 |
| Kickoff minus 2 h | Gamecock Walk; gates 30 minutes later [33] | game |
| Tue 7-9:30 PM, Nov 3-Dec 1; Jan 14, 15, 25 at 7:30 PM | CFP rankings [31]; CFP semifinals and title game [32] | S5 |
| Dec 2-4; Feb 3 | Signing; no confirmed start hour [40] | S1-S3 |
| Jan 4-13, 2027 | Portal window [38] | S2-S4 |
| Whole days | Fall evaluation Sept 1-Nov 29 (33 days a school) and spring contact Apr 15-May 29 [13]. A July 2026 FBS proposal would change the model from Jan 1, 2027: 300 days for winter and spring, 50 for fall evaluation. The outcome of its October vote is unsure [62]. Dead Nov 30 into January (end unsure) and June 28-July 31 [14] | S2-S5; all |

### What differs by conference and time zone
- West Coast schools run three hours behind: a 6 AM staff meeting is 9 AM ET, evening recruiting calls land in Eastern S6, and a 7:30 PM home kickoff is 10:30 PM ET, ending near 2 AM ET [44]. In 2025 USC kicked off at 11 PM ET one week and at noon ET (9 AM PT) the next [64].
- Group of 5: more weeknight TV (the MAC on Tuesday and Wednesday nights in November, mostly 7 PM) [45]. Smaller staffs do more themselves.
- FCS: the same January portal window; playoffs run to the title game, Mon Jan 11, 2027, 7:30 PM [65].

## NFL (Carolina Panthers)

Rules: players get at least one day off, almost always Tuesday [50]. 14 padded practices in the regular season: 11 in the first 11 weeks (one of those weeks may have two) and 3 after, 3 hours at most; playoff clubs one a week [48]. The HC meets the media at least three days a practice week, plus the day after games [53]. Coaches have no limit. 2026: twelve Sunday 1 PM games, Sunday night at home Oct 4, bye Oct 11, Thursday at Green Bay Oct 29, Monday at Tampa Bay Nov 30 [60].

### Game week (home, Sunday 1 PM). The week starts Monday.

| Day | S1 5-8 AM | S2 8 AM-1 PM | S3 1-4 PM | S4 4-7 PM | S5 7-10 PM | S6 10 PM-5 AM |
|---|---|---|---|---|---|---|
| Mon | Staff in by 5-6, grading every snap | Players: treatment, lift, corrections film; scans | HC press conference. Pro staff set Tuesday's tryouts | Players gone. Staff break down the opponent | First- and second-down plan. World: Monday night game 8:15 | Staff leave near midnight |
| Tue | Staff in before 6: the plan day | Players' day off [50]. Free agents work out | Plan: base downs, then third down and red zone. HC and GM settle moves | 4 PM: waiver claims on Monday's cuts, since claims are due the day after a player is waived (rule). Nov 10: trade deadline 4 PM [57] | Practice scripts, call sheet draft | Past midnight; some sleep in the office |
| Wed | HC's staff meeting 6:30-7 | Team meeting: the HC presents the plan. Installs, walkthrough. HC presser | First full practice, about 2 hours | Practice report by 4 PM ET, or after practice if it ends later (rule, unsure) [51]. Players lift; staff grade practice | Staff tune third down. Players study tablets | Home 10-11 PM |
| Thu | Staff in early | Third-down install | Practice: third down, red zone | Practice report, timed as Wednesday's (rule) [51] | World: Thursday night game 8:15 | Home earlier |
| Fri | Staff in early | Red zone and two-minute meetings. HC's last presser | Short practice, about 1.5 hours | Final report, now with game status, timed as Wednesday's (rule) [51][52]. Call sheet finished | Often a family night | Quiet |
| Sat | Quiet | Walkthrough, about an hour: a mock game | Players free. Last roster call. World: college games | 4 PM: last moves, up to two practice-squad elevations (rule) [54]. Team hotel (clubs differ) | Hotel: meetings, chapel, snack | Curfew, bed check near 11 |
| Sun | Players asleep. HC with the call sheet | Pregame meal, buses. Inactives 11:30 AM, 90 minutes out (rule). Warmups from about noon | Kickoff 1:00 [60]; ends about 4:05 | Locker room opens 10-15 minutes after the players come in; HC, QB and one player at the podium (unsure: from PFWA's policy page, not the 2026 text) [76]. World: 4:05 and 4:25 games | World: Sunday night 8:20. Staff start grading | The night game ends near 11:30 |

### How the week shifts

| Case | What moves |
|---|---|
| Thursday night (at Green Bay, Oct 29, 8:15 PM) | Four days after a Sunday game: walkthroughs only, game status due Tuesday. Wednesday: flight (S3-S4), hotel (S5). Thursday: inactives 6:45 PM (S4), kickoff (S5), home about 2-3 AM (S6). Then a long weekend. |
| Monday night (at Tampa Bay, Nov 30, 8:15 PM) | Everything a day later: Tuesday off, practice Thursday to Saturday, game status Saturday, Sunday flight. Home near 2 AM. Then a short week to Sun Dec 6 at Minnesota, 4:25 PM. |
| 4:25 PM and 8:20 PM Sundays | 4:25: inactives 2:55 (S3), kickoff (S4), home near midnight. 8:20: late meal (S3), inactives 6:50 (S4), kickoff (S5), ends near 11:30 (S6). |
| Bye (Oct 11) | At least four days off in a row (rule) [48]; that a Saturday is among them is unsure. Staff self-scout and work ahead. |

### The rest of the year

| Key day | S1 | S2 | S3 | S4 | S5 | S6 |
|---|---|---|---|---|---|---|
| Trade deadline, Tue Nov 10 | The GM's phone starts early | Calls; insiders report talks | Deals agreed, physicals | 4:00 PM deadline [57]; late deals announced | Traded players fly in | In the building by morning |
| Camp, padded day (late July-August) | Wake about 6, treatment | Meetings from 7:30; padded practice 2-2.5 hours. Camp: at most 16 padded, never more than 3 days in a row (rule) [49] | Lunch, rest, meetings | Walkthrough, no pads; dinner | Install and film meetings | Curfew about 11 |
| Cutdown day | The GM and HC meet at dawn | HC tells each cut player | Releases, trades for depth | Deadline: Tuesday 4 PM in 2023-25; 2026 reported Sun Aug 30, 6 PM (unsure) [59] | The 53 is out; agents call | Claims prepared; due next day (2026: 1 PM, unsure) [59] |
| Black Monday (Jan 11, 2027, after Week 18) | Firings land early (some owners fire Sunday night) | Firings out; exit meetings; HC's season-end presser | GM and owner pressers; interview requests | Candidate lists leak | Agents push their coaches | |
| Combine (2027: Mar 1-8) [57] | Medicals (hour unsure) | Measurements; HC and GM podiums | On-field testing from 3 PM Thu-Fri, 1 PM Sat-Sun (2024-25; no source, unsure) | On TV | Team interviews in hotel suites | Interviews run late |
| Tag deadline (a Tuesday, 4 PM; 2026: Mar 3) [58] | | Last talks with agents | Long-term deals done or not | 4 PM deadline | Tag news | |
| Negotiating window (a Monday, noon; 2026: Mar 9) [58] | | Noon: clubs may talk to other clubs' free agents; no signing | Deals agreed in principle leak | Flood of agreed deals | More deals | Leaks all night |
| League year (a Wednesday, 4 PM; 2026: Mar 11) [58] | | | Paperwork | 4 PM: deals and trades become official | Free agents fly in | Visits set for morning |
| Draft Thursday and Friday (2027: Apr 29-30) | | Thu: board set; trade calls | Calls | War room | Thu round 1 at 8 PM, 8 minutes a pick (new in 2026). Fri rounds 2-3 at 7 PM [55][56] | Ends near 11-11:30 PM; staff rework the board |
| Draft Saturday (2027: May 1) | | Rounds 4-7 at noon [55] | Rounds go on | Ends about 6-7 PM; the undrafted rush starts at once | Most undrafted deals done within hours | |

Unsure: the 2027 dates found for the tags (Feb 17-Mar 3) and the league year (Mar 11), from a search summary of [57], are exactly 2026's dates, so they fall on a Wednesday and a Thursday. That looks like a copying slip in the summary. The game itself puts the 2027 league year on the second Wednesday of March, Mar 10 (`calendar/anchors.rs` `league_year_day`). Read [57] before using any of them.

### Hard times (ET)

| Time | What | Section |
|---|---|---|
| 9:30 AM Sun | Six of 2026's nine games abroad: London three times, Paris, Madrid, Munich [60] | S2 |
| Thu Sept 10, 8:35 PM; Sun Sept 27, 4:25 PM; Sun Nov 22, 8:20 PM | The other three games abroad: Melbourne, Rio, Mexico City [60] | S5; S4; S5 |
| 90 minutes before kickoff | Inactives: 11:30 AM; 2:35 or 2:55 PM; 6:45 or 6:50 PM (rule) | S2; S3; S4 |
| Noon | Negotiating window; draft Saturday [55][58] | S2 |
| 1:00 PM Sun | Early games, 132 in Weeks 1-17 [60] | S3 |
| 4:00 PM | Elevations and last moves (the day before), waiver claims, trade deadline, tag deadline, league year [54][57][58]. Practice reports and game status (two days out) by 4 PM or after practice (unsure) [51][52] | S4 opens with them |
| 4:05, 4:25 PM Sun; about 6:30 PM | Late games [60]; Super Bowl (Feb 14, 2027) | S4 |
| 7 PM Fri; 8 PM Thu | Draft rounds 2-3; round 1 [55] | S5 |
| 8:15 PM Thu and Mon; 8:20 PM Sun | Prime time [60] | S5 into S6 |

### What differs by time zone
- West Coast clubs live three hours behind: the 4 PM ET deadlines (claims, elevations, trades) hit at 1 PM, the start of their S3, though practice reports may go in after practice instead (unsure) [51]. 13 of their 19 games at Central or Eastern hosts kick at 1 PM ET, 10 AM on their body clock [60]. An Eastern club's flight home from the West lands about 3-4 AM ET.
- Central clubs (Chicago, Dallas, Green Bay, Houston, Kansas City, Minnesota, New Orleans, Tennessee) run an hour behind: 1 PM ET is noon.
- Rules and deadlines are the same for all 32. Each club sets its practice times and media format.

## The world's day

What the rest of the world does in each section. Each stop opens with what it did since the last one (topic 5).

| Section | High school world | College world | NFL world |
|---|---|---|---|
| S1 5-8 AM | Quiet. Overnight offers and commitments seen | Overnight news: West Coast scores, late hires, flips | Quiet. Night-game news. Black Monday firings start |
| S2 8 AM-1 PM | School. College coaches visit in open periods. Signing ceremonies. Friday finals from noon | Office hours: portal names entered, league news. Noon kickoffs. Selection Day | Insiders' scoops before inactives. Pressers, tryouts. Sunday's 9:30 games abroad, the noon window, draft day 3 |
| S3 1-4 PM | Bell. Afternoon signings, polls. Sunday brackets | Press conferences. Sunday firings and hires. 3:30 kickoffs | Practice league-wide. Sunday 1 PM games. Combine testing |
| S4 4-7 PM | Practice everywhere. Thursday JV | Afternoon practices. Availability updates. Title games at 4 | The 4 PM deadlines and the transaction wire. Late Sunday games, Rio's among them |
| S5 7-10 PM | Friday kickoffs statewide; live scores | Night kickoffs. Wednesday reports, Tuesday CFP rankings. Peak recruiting calls | Prime time, including the Melbourne and Mexico City games. Draft rounds 1-3. Preseason games. Combine interviews |
| S6 10 PM-5 AM | Final scores, 11 PM highlights, Hudl uploads, recruits' posts | Late-window games. Flights home. Insiders break hires and flips | Night games end. Free agent deals leak. Flights land |

## What the game does today

Code at `ca23d9b`; engine paths under `crates/afm-engine/src/`, game paths under `crates/afm-game/src/`. `world/clock.rs` `advance_day` runs a whole day in one call, in code order. `continue_until_needed` stops only on a morning (`world/seat.rs` `World::morning_stop`) or after a day, and returns at once while your draft war room is open (`world/scouting/live.rs`). "Today" is a date, and the clock never runs on minutes. A `calendar/date.rs` `Moment` (date and minute) still feeds a few things: weather is drawn at a kickoff's local hour, an NFL crowd is louder from 6 PM local, the travelling team's body clock reads the kickoff hour (`world/college.rs`, `world/pro/games.rs`), and the calendar sorts a day's lines by minute (`calendar/events.rs`). Short paths below: `pro/`, `folk/`, `front_office/`, `scouting/`, `clock.rs`, `college.rs`, `recruiting.rs`, `hs_year.rs` and `nfl_year.rs` are under the engine's `world/`, and `anchors.rs` under its `calendar/`; `lane_*` paths are the game's.

### At a real time that fits its section

| What | Time (ET) | Section | Where |
|---|---|---|---|
| NFL deadlines: tag, league year, trade deadline, cutdown, tagged players' deals | 4:00 PM | S4 | `calendar/anchors.rs` `YearDates::with_weeks`; `world/nfl_year.rs` `dated_events` |
| Negotiating window; draft nights; undrafted rush; claims after cutdown | Noon; Thu 8, Fri 7, Sat noon; Sat 6 PM; noon | S2; S5, S5, S2; S4; S2 | `anchors.rs` `negotiating_window`, `YearDates::with_weeks` |
| NFL kickoffs: the real 2026 schedule, abroad, playoffs, Super Bowl, preseason | 9:30 AM-8:35 PM from 2026 (Melbourne is the latest); playoffs 1:00-8:15 PM, Super Bowl 6:30 PM, preseason 7 PM. 10:20 PM was only week 1's second Monday game, 2006-2021 | S2-S5, none in S6 | `nfl_year.rs` `formula_slots`, `real_schedule`, `international_sites`, `start_round`; `world/pro/mod.rs` `preseason_week` |
| College kickoffs, bowls, brackets | Noon, 3:30, 7:30, West 10:30 PM; bowls 4 and 8 PM | S2, S3, S5, S6; S4, S5 | `world/schedule.rs` `kickoff`, `weeknights`; `world/college.rs` `selection_day`, `advance_brackets` |
| HS kickoff; JV and freshmen | Fri 7:00 PM local (SC custom 7:30); Thu 6:00, 4:30 PM | S5; S4 | `world/hs_year.rs` `seat_setup`; `lane_schedule.rs` `jv_rows` |
| NFL week lines | Mon 9 AM, Tue 4 PM, Sat 4 PM, Sun 11 AM | S2, S4, S4, S2 | `world/clock.rs` `rhythm` |
| Weather | Local hour | any | `calendar/weather.rs` `weather_at` |
| Training day clock (proposed, not confirmed); takeover meetings (hour in the place text) | HS lift 6:30, practice 3:15; college lift 6:35; principal 7:15 AM; college team meeting 7:30 PM | S1-S5 | `lane_training/week.rs` `day_clock`; `world/folk/takeover.rs` `schedule` |

### At a default or wrong time today, and where each belongs

`calendar/events.rs` `Event::new` stamps 9:00 AM, and the calendar shows 9:00 as "All day" (`lane_calendar.rs` `lines_on`), except slot, deadline and practice lines, which show "09:00". News has a date and no time (`world/news.rs` `NewsItem`).

| What | Today | Where set | Belongs in |
|---|---|---|---|
| Game results | College: 9:00 on game day, before kickoff (notable games only). NFL prime time, abroad, playoffs: at kickoff. The Super Bowl: 9:00 on its day, before its 6:30 PM kickoff. Other NFL games: no line. HS varsity: no line. Your school's JV and freshman results: 9:00 Thursday, before their 4:30 and 6:00 PM games. HS all-star results and polls at 9:00 (weekly polls on Monday) | `college.rs` `result_event`; `nfl_year.rs` `play_day`, `play_round_games`, `super_bowl_book`; `front_office/school.rs` `sub_games`; `hs_year.rs` `state_all_stars`, `poll` | The final whistle, about 3 hours after kickoff: noon game S3, 3:30 game S4-S5, 7:30 game S6, Super Bowl about 10 PM (S6), JV and freshmen S4-S5, Friday HS about 10 PM (S6). HS polls Monday or Tuesday |
| College polls; CFP rankings | Sunday 9:00; Tuesday 9:00 | `college.rs` `release_poll`; `clock.rs` `advance_day` | Sunday afternoon, S3 (unsure); CFP Tuesday 7-9:30 PM, S5 [31] |
| Recruiting: offers, visits, commitments, decommits, portal entries, signings, declarations | 9:00. Mail: commitments 9 PM, signings 6 PM, portal entries 7 PM; at a high school, your players' commitments 8:15 PM and signings 5 PM. Offers and official visits come in a Monday 8:00 AM digest, and a college's offer to your high school player is emailed between 11:00 AM and 3:59 PM | `recruiting.rs` `early_offers`, `recruiting_day`, `announce_commitments`, `decommit_after_firing`, `enter_portal`, `run_signing_stage`; `clock.rs` `advance_day`; `lane_core/mail.rs` `people_mail` | Offers, commitments: evening posts, S5 (unsure). Signings: S2-S3 [12]. Portal names: office hours, S2-S3 [39]. Visits: Friday S3 to Sunday S2; game days, 2-3 hours before kickoff [47] |
| NFL trades, waivers, cuts, signings | 9:00 | `pro/market.rs` `post`, `claims`; `pro/mod.rs` `day` | Office hours S2-S4; claims and deadlines at 4 PM, S4; free agents from 4 PM on day one |
| NFL Wednesday, Thursday and Friday lines; NFL deadlines without a time (offer sheets, fifth-year option) | 3 PM (Wednesday's and Friday's name the injury report); "9:00 am" | `clock.rs` `rhythm`; `nfl_year.rs` `dated_events` | Reports by 4 PM ET or after practice, S4 or later (unsure) [51]; those deadlines 4 PM ET, S4 (unsure) |
| Draft picks and draft-day trades | Picks stamped 10 minutes apart in round 1, 7 in rounds 2-3 and 5 after (the war room's own clock is 8, 7, 5, 5, 5, 5 and 4 minutes from 2026): round 1 ends about 1:10 AM, Saturday near midnight, after the 6 PM rush. All three days run inside Thursday, and every draft-day trade posts Thursday at 9:00 | `recruiting.rs` `draft`, `draft_end`; `scouting/live.rs` | Each pick and trade at its minute: round 1 ends about 11:30 PM, day 3 about 6-7 PM; clocks 8, 7, 5, 4 minutes [56] |
| Mail | Stamped by kind, 6:00 AM to 11:10 PM, but written after the days ran and shown at the next stop | `lane_core/mail.rs` `after_days` | The section of its stamp |
| Your game | Played at the morning stop | `world/live_seat.rs`; `app/ui/screens/gameday.js` | Its kickoff's section (topic 7) |
| Practice injuries, the training week, the NFL's Tuesday batch, court cases | College and HS seats: drawn or planned on Monday (your injuries dated ahead). NFL: every club's practice injuries, yours included, with healing and trouble, on Tuesday. Courts: new cases open on Wednesday. Each case then moves on its own due date, checked daily (charges 30-90 days later, a ruling 30-120 days after that), so rulings land on any weekday. Your players' lines at 9:00 | `training/seat.rs` `practice_injuries`; `training/week.rs` `monday`; `pro/games.rs`; `pro/mod.rs` `day`; `folk/courts.rs` `day`, `step`; `folk/mod.rs` | Each injury in that day's practice section; arrests overnight (S6, unsure); rulings in office hours, S2-S3 (unsure) |
| Hires and firings | News, no time | `world/staff.rs` `college_news`; `nfl_year.rs`; `clock.rs` `advance_day` | College Sunday S3 [36]; Black Monday S1-S2; hires S3-S5; leaks S6 |
| Meetings; boss season meetings; postgame and weekly press | Meetings set ahead that stop Continue: a date, opened at the morning stop. Boss season meetings: the goals and review meetings are made while their own day runs, and your staff takes them over at the end of that day, so you never hold one. The handover clears the mark that stops Continue (looks like a bug). The midseason check-in never stops Continue. Postgame press: made during the game day's run, never stops Continue, waits up to 7 days, no time. Weekly press: Monday at college and the NFL; at a high school, the local paper calls Thursday | `folk/scene.rs` `due`, `day`; `folk/boss.rs` `day`; `folk/mod.rs` `day`; `folk/press.rs` `day`; `world/seat.rs` | Their real section (topic 8); postgame press 10-15 minutes after the game (unsure) [76]; weekly press at the school's or club's own hour (SC: Tuesday 1 PM, 2024 [23]) |

## Time zones

The world's hard times are Eastern: every `Moment` is ET (`calendar/date.rs`). High school keeps 7 PM local with no Eastern time; weather runs on local hours.

Proposed, not confirmed:
- A seat lives on its own local time: its sections start at 5 AM, 8 AM, 1 PM, 4 PM, 7 PM and 10 PM where it is.
- League times stay Eastern and show in local time with ET beside: "Waiver claims due 1:00 PM (4:00 PM ET)". The world's events keep their Eastern hour and land in the local section they fall in.
- Pacific NFL seat: the 4 PM ET deadlines open S3 at 1 PM (practice reports may go in after practice instead, unsure); a 1 PM ET road game kicks off at 10 AM (S2); Sunday night is 5:20 PM (S4). Pacific college seat: a noon ET road game is 9 AM (S2).
- Central is one hour behind. Arizona keeps standard time all year: 3 hours behind Eastern in summer, 2 in winter. Hawaii: 6 and 5.
- On the two clock-change nights a seat's S6 keeps its clock hours, 10 PM to 5 AM, so it runs 8 hours in the fall and 6 in the spring.
- High school is all local. A Friday 7 PM game in California ends near 1 AM Eastern, in an Eastern seat's S6.

## Sources

Most pages were blocked from this session, so facts come from search summaries of them; 57-60 come from the project's earlier research. 62-76 come from the research behind this map, for facts that had no source here.

1. Dutch Fork handbook: https://docs.google.com/document/d/1vTp8845L7-MkLe1smSp2YC3dFzWGGmRg/export?format=pdf
2. SCHSL rules 2026-27: https://schsl.org/wp-content/uploads/2026/07/2026-27-Rules-Section.docx-2.pdf
3. SCHSL summer: https://schsl.org/archives/18548
4. SCHSL heat: https://schsl.org/wp-content/uploads/2024/07/Heat-Guidelines-Updated_-April-2024-3.pdf
5. The State: https://www.aol.com/high-school-football-season-week-120000280.html
6. Spring Valley JV: https://media.hometeamsonline.com/photos/football/SVVIKINGSFOOTBALL/2019_JV-B-Squad_Football_Schedule_(1).pdf
7. ABC News 4, finals: https://www.abcnews4.com/news/local/schsl-football-championships-to-broadcast-live-on-my-tv-charleston-dec-5-and-6
8. Outside the Huddle: https://outsidethehuddle.net/2020/10/05/coachs-corner-being-a-head-coach-the-weekend/
9. Teacher-coach: https://ebulling.medium.com/day-in-the-life-of-a-pm-high-school-coach-1e02a5388ecf
10. Natchez Democrat: https://www.natchezdemocrat.com/?p=448018
11. WIS: https://www.wistv.com/2019/08/31/football-friday-get-latest-high-school-football-scores
12. Rivals: https://southcarolina.rivals.com/news/south-carolina-football-recruiting-early-signing-period-announcement-times
13. NCAA calendar 2026-27: https://ncaaorg.s3.amazonaws.com/compliance/recruiting/calendar/2026-27/2026-27D1Rec_FBSRecruitingCalendar.pdf
14. Oriel (secondary): https://orieladmissions.com/ncaa-recruiting-calendar/
15. NCAA 2023 model: https://www.ncaa.org/news/2023/4/7/media-center-comprehensive-di-football-recruiting-model-recommended.aspx
16. ASU compliance: https://sundevilcompliance.asu.edu/sites/default/files/mbb_recruiting_guide_2017-18.pdf
17. NCAA game day: https://wbca.org/072320-ncaa1-er-2020-12-pdf
18. UIL limits: https://www.uiltexas.org/policy/tea-uil-side-by-side/limits-on-practice-and-performance
19. Galena Park ISD: https://www.galenaparkisd.com/cms/lib/TX02217083/Centricity/Domain/839/2024%20Composite%20Football%20Schedule.pdf
20. California SB 328: https://aalrr.com/newsroom-alerts-3580
21. WACH, brackets: https://wach.com/sports/high-school-sports/high-school-football-state-playoff-brackets-released
22. WSPA: https://digital-release.wspa.com/?p=26793
23. Gamecocks, preseason: https://gamecocksonline.com/news/2024/07/25/fb-south-carolina-preseason-football-information/
24. Gamecocks, spring: https://gamecocksonline.com/news/2026/02/27/gamecocks-announce-spring-practice-dates-3/
25. SI, Purdue: https://www.si.com/college/purdue/football/ryan-walters-explains-benefits-purdue-boilermakers-football-practicing-morning-big-ten-football
26. Hour limits: https://static.goredfoxes.com/custompages/pdf/hour-limitations.pdf
27. WBCA: https://wbca.org/2017-ncaa-d1-autonomy-proposals-qa-pdf
28. SEC reports: https://www.secsports.com/news/2024/08/sec-to-provide-public-availability-reports-for-football-basketball-and-baseball
29. SEC TV windows: https://hailstate.com/news/2026/6/10/sec-announces-2026-football-television-windows
30. Fox Carolina: https://www.foxcarolina.com/2026/10/12/kickoff-time-set-south-carolina-tennessee-game/
31. CFP rankings: https://fbschedules.com/college-football-playoff-sets-2026-rankings-release-schedule/
32. CFP schedule: https://fbschedules.com/college-football-playoff-sets-2026-27-schedule-kickoff-times-tv-designations/
33. WIS, the Walk: https://www.wistv.com/2025/09/05/heading-williams-brice-gamecocks-home-opener-heres-what-know
34. Post and Courier: https://www.postandcourier.com/sports/carolina/gamecocks-football-bye-week-plans-self-scout-shane-beamer/article_231b875b-ab5a-4d4f-b51d-80b77b7f4ea8.html
35. WACH, Thursday: https://wach.com/sports/usc-gamecocks/shane-beamer-details-unique-week-ahead-of-thursday-night-game
36. The Advocate: https://www.theadvocate.com/baton_rouge/sports/lsu/lane-kiffin-latest-lsu-officials-confident/article_c2a2e962-0e6c-5ffc-b535-6b40205feb48.html
37. CBS: https://www.cbssports.com/college-football/news/ncaa-officially-adopts-new-transfer-portal-rules-including-major-adjustment-for-players-after-coaching-change
38. Athletic Business: https://www.athleticbusiness.com/operations/governing-bodies/article/15837188/ncaa-di-cabinet-approves-new-transfer-windows-for-several-sports
39. LSU compliance: https://www.compliance.lsu.edu/transfer-regulations
40. NextCommit: https://www.nextcommit.ai/blog/national-signing-day-2027
41. NCAA 2027 calendar: https://www.ncaa.org/news/fbs-oversight-committee-approves-legislation-to-modify-football-calendar/
42. NBC Sports: https://www.nbcsports.com/college-football/news/ncaa-cabinet-tweaks-college-football-calendar-with-14-week-season-and-bowl-rule-change
43. NCAA 2017: https://www.ncaa.org/news/2017/4/14/di-council-votes-to-eliminate-football-two-a-days.aspx
44. Fox Sports: https://www.foxsports.com/stories/presspass/fox-college-football-starts-saturdays-strong
45. MAC: https://ohiobobcats.com/news/2026/5/27/mac-announces-select-2026-football-game-times-and-tv-selections
46. Business Alabama: https://businessalabama.com/taking-care-of-the-tide/
47. Arkansas Online: https://www.arkansasonline.com/news/2010/nov/21/recruiting-guy-official-visits-include-so-20101121/
48. CBA Art. 24: https://overthecap.com/collective-bargaining-agreement/article/24/section/1
49. CBA Art. 23: https://overthecap.com/collective-bargaining-agreement/article/23/section/7
50. NFL timeline: https://operations.nfl.com/learn-the-game/nfl-coaching-timeline
51. NFL injury reports: https://operations.nfl.com/media/1818/2015-injury-report-policy.pdf
52. SI, game status: https://www.si.com/nfl/2016/08/22/nfl-team-injury-report-changes
53. NFL media policy: https://www.profootballwriters.org/wp-content/uploads/2026/08/26NFLMediaAccessPolicy.pdf
54. ESPN, elevations: https://www.espn.com/nfl/story/_/id/31869693
55. NFL.com, draft: https://www.nfl.com/news/2026-nfl-draft-order-for-all-seven-rounds
56. Steelers Now: https://steelersnow.com/nfl-announces-significant-change-for-2026-nfl-draft-in-pittsburgh/
57. NFL.com, dates: https://www.nfl.com/news/2026-27-national-football-league-important-dates
58. Bengals, free agency: https://www.bengals.com/news/2026-nfl-free-agency-essential-information
59. Bengals, cutdown: https://www.bengals.com/news/2026-nfl-53-man-roster-cutdown-deadline-essential-information
60. nflverse schedule: https://github.com/nflverse/nfldata/blob/master/data/games.csv
61. IANA time zone database: https://www.iana.org/time-zones
62. NCAA, recruiting model proposal (July 2026): https://www.ncaa.org/fbs-oversight-committee-recommends-changes-to-the-off-campus-recruiting-model/
63. Eleven Warriors, portal windows: https://www.elevenwarriors.com/college-football/2025/10/158528/college-football-transfer-portal-window-officially-moved-to-jan-2-through-jan-16
64. USC kickoffs, 2025: https://www.si.com/college/usc/football/late-night-usc-trojans-michigan-state-kickoff-time-big-ten-fans-slam-lincoln-riley-jonathan-smith-conference-realignment-spartans ; https://www.aol.com/sports/uscs-lincoln-riley-trojans-noon-155353382.html
65. FCS playoffs 2026: https://fbschedules.com/ncaa-reveals-broadcast-schedule-for-2026-division-i-fcs-playoffs/
66. High School Football America, SC dates: https://highschoolfootballamerica.com/?p=136790
67. Tom Knotts as AD: https://www.maxpreps.com/sc/irmo/dutch-fork-silver-foxes ; https://lexingtonchronicle.com/stories/tom-knotts-named-national-hs-football-coach-of-the-year,11537
68. GHSA heat table: https://www.ghsa.net/sites/default/files/documents/sports-medicine/HeatHumidity.pdf
69. UGA Grady, GHSA spring game: https://gradynewsource.uga.edu/ghsa-approves-spring-football-game-scrimmage
70. Florida spring and heat law: https://www.orangeobserver.com/news/2024/apr/26/florida-fhsaa-spring-high-school-football-kicks-off-april-29/ ; https://www.legiscan.com/FL/text/S1696/2020
71. California AB 2127: https://www.leginfo.ca.gov/pub/13-14/bill/asm/ab_2101-2150/ab_2127_bill_20140702_enrolled.htm
72. SI, CIF-SS TV games: https://www.si.com/high-school/california/cif-southern-section-sets-tv-games-for-high-school-football-first-three-weeks-01j3dj5adgrf
73. NCAA calendar 2025-26: https://ncaaorg.s3.amazonaws.com/compliance/recruiting/calendar/2025-26/2025-26D1Rec_FBSMFBRecruitingCalendar.pdf
74. WIS, SC 2026 schedule: https://www.wistv.com/2025/12/12/south-carolina-football-unveils-2026-schedule/
75. Gamecocks, TV show (2023): https://gamecocksonline.com/news/2023/08/23/gamecock-football-with-shane-beamer-show-debuts-sunday/
76. PFWA, media access policies 2014-2026: https://www.profootballwriters.org/nfl-media-access-policy/
