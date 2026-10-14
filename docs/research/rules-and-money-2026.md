# Research area 2: 2026 rules and money

Researched 2026-10-14 by Claude for Raichel. Research only. Anything that points to a design change is under "Proposals for Raichel" and is **proposed, not confirmed**.

It feeds: section 5 (team management: the cap, contracts, rosters), the calendar spec (`docs/design/calendar.md`, topics 2-5 and its fixed list of future changes), and the world's college money and eligibility (`docs/design/world.md`). The NFL figures were also added to `data/real-world/nfl_rules.json`.

## College after the House settlement

**The settlement** (House v. NCAA, final approval June 6, 2025, CBS: https://www.cbssports.com/college-football/news/house-v-ncaa-settlement-approved-landmark-decision-opens-door-for-revenue-sharing-in-college-athletics/)
- Runs 10 years. $2.8 billion in back pay to athletes from 2016-2024.
- Schools that opt in may pay players directly ("revenue sharing"). Cap: **$20.5M per school in 2025-26**, rising **at least 4% a year**. 2026-27: **about $21.3M** (MultiState, 2026-09-29: https://www.multistate.us/insider/2026/9/29/the-new-economics-of-college-athletics-an-update-on-name-image-and-likeness-laws).
- The **College Sports Commission (CSC)** enforces it. A part called "CAP" watches schools' payouts.
- **NIL Go** (built by Deloitte): every outside NIL deal over **$600** is reviewed for fair market value. Schools pay Deloitte $5,000-$500,000.
- Rules took effect July 1, 2025.

**Rosters and scholarships** (CBS, 2025-06-23: https://www.cbssports.com/college-football/news/ncaa-removes-scholarship-limits-aligns-with-house-settlement-as-roster-sizes-evolve-in-new-college-sports-era/)
- Football roster cap **105** at opt-in schools. The 85-scholarship limit is gone: any number of the 105 may be on scholarship.
- Players already on a roster when the rule started are grandfathered ("designated student-athletes"). Their school may go over 105 until their eligibility runs out.

**NIL Go so far** (CSC NIL Data Report, data to 2026-07-01: https://assets.tina.io/29b83311-e587-42b1-861e-87ebde9aa253/NIL%20Data%20Report%20-%20July%208,%202026.pdf)
- Since launch on June 11, 2025: 34,195 deals worth $355.2M and 1,812 deals worth $89.9M (the report doesn't label which total is cleared; the second is almost certainly "not cleared").
- May 1 to June 30, 2026 alone: 7,639 deals cleared ($112.9M) and 659 not cleared ($33.7M). So about **8% of deals and 23% of the money** fail review: big booster-style deals fail far more often than small ones.
- No split by sport or school is published.

**Law**
- 35 states have NIL laws (MultiState, 2026-09-29). The Senate passed the Protect College Sports Act 77-22; it now goes to the House.

**Eligibility: five seasons in five years (new in 2026)**
- The NCAA Division I Cabinet adopted **"5-in-5"** on June 23, 2026 (FootballScoop: https://www.footballscoop.com/2026/06/23/ncaa-approves-age-based-5-for-5-eligibility-rule). Players get **five seasons of competition in a five-year window**. The clock starts at enrollment or the 19th birthday, whichever comes first (the NCAA's April 2026 board concept said the academic year after turning 19 or finishing high school: https://www.ncaa.org/media-center-di-board-of-directors-directs-cabinet-to-advance-age-based-eligibility-rules/).
- It **ends the redshirt** and the hardship/injury waivers. It applies from the high school class of 2023 on, not to anyone whose eligibility ended by spring 2026.
- Court fights (Morgan Lewis, 2026-08-25: https://www.morganlewis.com/pubs/2026/08/ncaa-5-in-5-rule-faces-legal-challenges-as-college-football-season-nears): class-of-2022 athletes won a preliminary injunction in Colorado (July 31, 2026), but the NCAA got it **stayed on appeal (Aug 21)**, so the rule stands for now. A Louisiana court let 33 athletes (16 football players, some with past NFL contracts) return in August 2026.

**Transfer portal** (2026 dates; ESPN and Eleven Warriors: https://www.elevenwarriors.com/college-football/2025/10/158528/college-football-transfer-portal-window-officially-moved-to-jan-2-through-jan-16)
- One window, **January 2-16**, for FBS and FCS. The April window is gone.
- Players in the national title game get 5 extra days after their season (2026: Jan 20-24).
- After a head coach change: 15 days, starting when the new coach is hired.
- No deadline to pick a new school. Transfers are eligible at once if academically eligible.
- These match the calendar spec already.

**Recruiting calendar 2025-26, FBS** (Texas Football: https://www.texasfootball.com/article/2026/03/09/the-2025-26-division-i-fbs-recruiting-calendar, and a full date list: https://www.gunslingerbuzz.com/p/2026-college-football-key-dates)
- Dead: Jan 1-4, Jan 12-14, Feb 2 to Mar 1, May 24-27, Jun 22 to Jul 31. Quiet: Mar 2 to Apr 14, May 28 to Jun 21. Contact: Jan 5-10, Jan 15-31, Apr 15 to May 23.
- Official visits for juniors: April 1 to June 21. Verbal offers to sophomores from June 15. Calls once a week to juniors from Sept 1.
- National Signing Day: Feb 4, 2026. The early signing period is three days starting the Wednesday after the last FBS regular-season game, before the conference title games (adopted March 7, 2024: https://www.nationalfootballpost.com/early-football-signing-period-moving-to-early-december). A June signing period was tabled, not adopted.
- Official visits per school: a 2024 report quotes a GM saying recruits may take unlimited official visits but each school may host 70 a year (https://footballscoop.com/news/college-football-moving-closer-to-potential-june-signing-period-earlier-december-dates). Not confirmed on an NCAA page.
- D2 and D3 differ (D2 contact and visits for 2028 recruits from June 15; D3 official visits from Jan 1).

**NFL draft: declaring and coming back**
- 2026 dates: soft deadline Jan 5, declaration deadline Jan 14 (tentative), approved list Jan 15; title-game players by Jan 23 (gunslingerbuzz list above).
- To keep college eligibility, a player must withdraw **within 72 hours after the NFL's declaration deadline**, must not be drafted, and must not sign with an agent (UGA compliance: https://compliance.sports.uga.edu/amateurism-reminders/; Sportskeeda: https://sportskeeda.com/college-football/can-go-back-college-declare-nfl-draft-ncaaf-nfl-rules-explored). For 2026 that is about January 17. No 2026 NCAA page with the exact date was found, so the date is inferred from the rule. This answers the open question in `docs/handoff/open-questions.md`.

**Already in the dataset:** high school NIL by state and transfer rules (`hs_states.csv`), NIL position ranges and the portal counts (`rates.json`).

## NFL rules and money, 2026

Already in `data/real-world/nfl_rules.json` before: the cap ($301.2M), minimums, tags, fifth-year option tiers, free agency types and dates, practice squad pay, the 90-to-53 cutdown, the rookie scale. Added now:

- **RFA tenders 2026**: right of first refusal only $3.52M; original round $3.674M; second round $5.767M; first round $8.046M (or $8.546M without a limiting offer-sheet term). Each is at least 110% of last year's salary (NFL.com: https://www.nfl.com/news/2026-nfl-free-agency-questions-answers).
- **Unsigned free agents**: a team that tenders 110% to its own unsigned UFA by late April gets his rights back after July 22, until Nov 17. Unsigned tagged or RFA players by Nov 17 can't play that season.
- **Game day**: 48 active of 53. Up to 2 practice squad players elevated per game, each player at most 3 times, then he reverts. An emergency third QB can be listed when only two QBs are active; he may play only if both are injured or ejected.
- **Practice squad**: 16, plus 1 international player. Up to 6 veterans with no limit on seasons; veterans and young players with two or fewer seasons together at most 10.
- **Injured reserve**: at least 4 games out; up to 8 players may return per season (10 for playoff teams) plus 2 named at the cutdown; a 21-day practice window before they must be activated. Reserve/PUP and reserve/NFI also sit at least 4 games (Fox Sports, 2025-07-22: https://www.foxsports.com/articles/nfl/from-ir-and-pup-to-practice-squads-what-do-nfl-roster-designations-mean).
- **Cap mechanics**: signing bonuses spread over at most 5 years; void years are placeholder years that hold bonus money and accelerate when the deal voids (SumerSports: https://sumersports.com/the-zone/void-years-explained/). Releasing or trading a player before June 1 puts all his remaining bonus on this year's cap; up to two "post-June 1" designations a year split it over this year and next (PFR: https://profootballrumors.com/2022/04/the-nfls-post-june-1-cuts-explained-6).
- **Spending floor**: teams must pay out 90% of the cap in cash over 2024-26 and 2027-30; any shortfall goes to the players (CBA Art. 12 §9: https://Overthecap.com/collective-bargaining-agreement/article/12/section/9).
- **Compensatory picks**: for a net loss of free agents, valued by salary, playing time and honors; rounds 3-7; at most 4 per team; 32 formula picks in 2026. A team that loses a minority head coach or GM to another team gets a 3rd-round pick in each of the next two drafts (NFL Football Operations: https://edge-operations.nfl.com/updates/football-ops/33-compensatory-draft-choices-announced-for-2026-nfl-draft). The exact formula is not public.
- **2026-27 dates** (NFL.com: https://www.nfl.com/news/2026-27-national-football-league-important-dates): trade deadline **Nov 10, 2026**; playoffs from Jan 16, 2027; Super Bowl LXI Feb 14 at SoFi; 2027 tags Feb 17 to Mar 3; combine Mar 1-8; league year Mar 11; draft Apr 29 to May 1 in Washington, DC.
- **The CBA runs through the 2030 season.** An 18-game season needs a new CBA; the earliest start would be 2027. The proposal: two byes, one international game per team, a two-game preseason, the Super Bowl on Presidents' Day weekend (CBS, 2025-04-02: https://www.cbssports.com/nfl/news/nfl-moving-to-18-game-schedule-when-can-we-expect-league-to-expand-including-changes-to-international-slate).

Gaps, said plainly: the 2027 salary cap (no official projection found), 2027 fifth-year option amounts by position, the full comp-pick formula, school-by-school revenue-sharing payouts and the football share of them, which schools opted in.

## Proposals for Raichel (proposed, not confirmed)

1. **5-in-5 is already real in 2026.** The calendar spec (topic 3) says "a player can play up to 4 games and still redshirt: 5 years to play 4", and its list of future changes has "5-to-play-5" as something that might happen later. Real life moved first: since June 2026 players get 5 seasons in 5 years, with no redshirt, from the class of 2023 on, while lawsuits continue. Proposal: start the game with 5-in-5 in force, with older players under the old rule, and keep the court fight as a live risk (a ruling could change it during a career). That also changes the spring-ball breakout group ("redshirt freshmen") and the portal "redshirt trick" in the calendar spec.
2. **NIL Go as a real filter.** About 8% of outside deals (23% of the money) failed review in mid-2026. Proposal: booster-style offers far above a player's market value can be blocked or cut back, so collectives can't simply outbid everyone.
3. **Revenue sharing grows 4% a year** from $20.5M, alongside the agreed future "revenue cap" change in the calendar's fixed list.
4. **The draft-withdrawal deadline** is 72 hours after the NFL's declaration deadline (about Jan 17 in 2026). This closes the open calendar question.
5. **NFL roster rules for section 5** as listed above (48 active, 2 elevations a game, 3 per player, emergency third QB, 8 IR returns, the spending floor, comp picks).
