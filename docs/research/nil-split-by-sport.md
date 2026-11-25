# Research area 7: the real NIL and revenue-share split (by sport, position, star level and the House settlement)

Researched 2026-11-24 by Claude for Cain, thread "Research real NIL split by sport", for realism audit item 14 (`docs/trials/realism-audit-seed321.md`). Research only: nothing here changes the engine. Everything under "Proposals" is **proposed, not confirmed**. Proposal 4 was built on 2026-11-25 (thread "Spread NIL pay by player value"); its numbers are under "Item 4, built and measured" near the end.

Labels as in `README.md`: **[published]** (someone else's figure, linked), **[computed]** (Claude's arithmetic on published figures, shown), **[estimated]** (Claude's judgment).

**Limits of this research.** The primary reports (Opendorse's PDFs, NIL Wire, Yahoo) could not be opened from the research container: the hosts are outside its network allowance, and I did not try to get round that. Every Opendorse figure below is therefore read from press coverage and search summaries of the reports, with the outlet named. The College Sports Commission (CSC) medians come from press coverage of its October 2026 release and from the table already used by the world's fit (`docs/reference/how-it-works/recruiting.md`, "NIL money"). The AFM side is read from the seed 1 fit runs in the project folder (`afm/nil-gaps/paidset_v3_run.txt`) and from the audit's seed 321 numbers. **I did not run a world.**

## Bottom line

1. The audit's two "real" figures (66.5% of Power 4 football players under $10,000; about 10% at $100,000 or more) are **Opendorse's 2024-25 picture, before revenue sharing**. They are not the 2025-26 picture. The College Sports Commission's (CSC) 2025-26 payments are the better yardstick.
2. The audit's remark that Opendorse's $1.9B "is every sport" looks wrong: Opendorse's $1.9B is its projection for **college football players** in 2025.
3. **Measured 2026-11-24 (item 3, below): the world is too even, and by more than the audit's seed 321 number showed.** On the seed 1 world's July 2026 payday, 22% of a Power 4 roster is under $10,000 (at real), but **66% is at $100,000 or more** against about 35-45% real. The cause is traced: every paid man at a position and role gets the same number, so most of the paid sit on one plateau just above $100,000, where the CSC shows a long tail down to the 10th percentile at $17,500-$25,000.

## What the audit compared against

| Figure | What it really is | Source |
|---|---|---|
| 66.5% under $10,000 a year; 0.3% over $1M; $100K and up about 10% (9.1% + 0.6% + 0.3% in `data/real-world/rates.json`, `power4_earnings_distribution`) | Power 4 **football** players, from NIL compensation "facilitated or disclosed through Opendorse" between **July 1, 2024 and August 10, 2025**. That is the last season before revenue sharing began on July 1, 2025, and only the deals Opendorse can see. | Opendorse, *College Football Kickoff* (Aug 2025), as read in [Joe Pompliano's post](https://x.com/JoePompliano/status/1960740025307476233), [Yahoo Sports](https://sports.yahoo.com/article/opendorse-report-details-power-four-203651154.html) and [NIL Wire](https://www.nil-wire.com/p/inside-the-opendorse-report-breaking-down-college-football-earnings-e) **[published]** |
| $1.9B for 2025 | Opendorse's projection of what **college football players** earn in 2025: collegiate (school revenue share) $1.4B, commercial $290M, collective $213M. Opendorse has direct insight into about half of Power 4 and Group of 6 schools, so it is an estimate. It also projects $2.4B in 2026 and $2.6B in 2027. | [Front Office Sports](https://frontofficesports.com/college-football-players-projected-to-earn-1-9b-this-year-nearly-double-2024/), [Forbes via search](https://www.forbes.com/sites/kristidosh/2025/08/27/college-football-players-this-season-projected-to-bank-19b/) **[published]** |

So the world's Power 4 total of $1.94B is a football number set against a football number. Opendorse's $1.9B may also include some Group of 6 money, so the match is close, not exact.

## The real split by sport

AFM has football only, so the split matters in two places: how much of a school's revenue-share cap goes to football, and how much of a school's third-party money (boosters, collectives, marketing partners) is football's.

**Revenue share (the school's own money, from July 1, 2025)** [published]
- The commonly cited back-damages split: 75% football, 15% men's basketball, 5% women's basketball, 5% everything else ([CBS Sports](https://www.cbssports.com/college-football/news/how-historic-house-v-ncaa-settlement-will-impact-college-athletics-on-and-off-the-field-for-years-to-come)). LSU said 75% football and 15% men's basketball. Texas Tech reported 74% football, 17-18% men's basketball, 2% women's basketball, 1.8% baseball.
- Opendorse's average Power 4 budget: **65.6% football, 20.3% men's basketball, 6.6% women's basketball, 7.5% everything else**; a football room's average is about $13.4M of the $20.5M cap. CBS adds that many athletic directors expect football to be 75% or more.
- First-year actuals, Opendorse via [Front Office Sports](https://frontofficesports.com/college-hoops-accounts-for-nearly-30-of-revenue-sharing-payments/): men's and women's basketball together took nearly 29% of revenue-share dollars, football the largest share.
- Ohio State pays revenue share only to football, both basketball teams and volleyball. Schools choose which sports to pay.

**Third-party NIL (collectives, commercial)** [published]
- Collective dollars by sport (Opendorse, [Front Office Sports](https://frontofficesports.com/as-nil-turns-three-collectives-and-football-still-control-the-industry/)): football 72.2%, men's basketball 21.2% (another table in the same coverage: 67.8% and 22.6%).
- Commercial dollars: football 76.6%.
- Whole NIL market, a later Front Office Sports piece: football 79%, men's basketball 9.6%, volleyball 5.5%. I could not confirm it is the same measure as the $4B market report.
- Football players' earnings are 13.1% of Power 4 school revenue and 13.3% of Group of 6's (Opendorse, already in `rates.json`).

**Where the money moved with the House settlement** [published, Opendorse's table via Front Office Sports]: collective money was $853M in 2024 and is projected at $213M in 2025, while school revenue share (collegiate) is $1.4B of the $1.9B. [computed] That is about 74% school, 15% commercial, 11% collective. The CSC counted $1.77B paid through its payment system from June 2025 to June 2026 (some outlets say $1.98B).

**Against AFM** [computed]: a Power 4 school's revenue share is 0.75 of the cap (`revenue_cap`, `revenue_share`; not to be confused with `NIL_PAID_P4`, the share of the roster that is paid). That sits between Opendorse's 65.6% and the athletic directors' 75% and above, and equals the benchmark and LSU's. The third-party pot was fitted to **football roster budgets** (The Athletic's SEC $33-38M and so on; the NIL scheme spec agreed 2026-11-15, item 1), so it is already football's part. **Nothing in the split by sport needs a change.**

## By position (CSC, 2025-26)

Revenue share among Power 4 players paid anything [published: [Fox Sports](https://foxsports.com/stories/college-football/nil-players-salary-money-position-quarterback), [On3](https://www.on3.com/nil/news/how-much-do-football-players-make-nil-revenue-sharing-college-sports-commission-full-data/), [RG](https://rg.org/news/football/college-sports-commission-football-pay-data-quarterbacks)]:

| Position | Median | Other points found |
|---|---|---|
| QB | $230,000 | 75th percentile $675,000; 90th $1,611,000 |
| DL | $150,000 | |
| OL | $133,000 | 90th percentile $512,000 |
| DB | $115,000 | |
| WR, LB | $112,500 | |
| RB | $108,500 | |
| TE | $93,000 | |

- Non-Power 4, among those paid: QB $31,000 (90th percentile $260,000), OL $17,500 (90th $112,000), other spots $14,000-$19,000.
- Third-party (cleared) NIL, Power 4 medians: QB $65,550, LB $21,500 (90th percentile $228,100), DB $25,000, RB $26,050.
- Paid share of a roster: 75% of Power 4 rostered players and 29% outside, in the table the world's item 13 already uses (Business of College Sports); YSBR's own calculation from CSC counts: 70-83% for skill and line positions at Power 4, about 30% at most non-Power 4 positions (kickers 16%).
- **The 10th percentile among paid Power 4 players is $17,500 to $25,000 at every position with enough players** (YSBR's reading of the CSC table, [Youth Sports Business Report](https://youthsportsbusinessreport.com/what-the-first-college-athlete-compensation-data-says-about-who-actually-gets-paid-1-6-million-quarterbacks-and-500-rowers/)). This is the figure the world does not meet.
- Opendorse's budget share by position (2025): QB 14-23% of a conference's football budget, WR 11-17%, OL 14-17%, DB 13-14%, DL about 15% (`rates.json`).

**Against AFM** (seed 1 fit run, July 2026 payday) [computed from `afm/nil-gaps/paidset_v3_run.txt`]: Power 4 revenue share among the paid, QB $231K, DL $151K, OL $135K, DB $112K, WR $110K, LB $112K, RB $106K, TE $93K against the CSC's $230K, $150K, $133K, $115K, $112.5K, $112.5K, $108.5K, $93K. The medians are at real. The Power 4 starters' market (QB $2.02M, EDGE $928K, OT $776K, WR $659K, CB $510K, RB $579K, IOL $622K) is within 20% of The NIL Standard's medians at every spot it names (QB and EDGE within 1%, tackle 14% under, interior line 18% over); safeties, linebackers and tight ends have no real figure.

## By recruit star level

What is published is thin, and nothing gives a number for each star level:
- Five-stars: $750,000 to $1M a year (The Athletic). Single deals: Underwood about $10M over 4 years, Curtis about $2M, Cantwell about $1.5M, Ojo $5.1M over 3 years (already in the NIL scheme spec).
- Class averages per recruit (all stars together, On3): $96,000 to $253,000 at the top classes; top classes $8M-$9M (USC); Big Ten classes' average per recruit: USC $143,000, Michigan $134,000, Washington $95,000 ([SI, On3 estimates](https://www.si.com/college/oregon/football/nil-recruiting-class-oregon-ducks-ohio-state-buckeyes-rankings-deals-money-maryland-dan-lanning-ryan-day-name-image-likeness)). ESPN reported upfront payments up to $25,000 a month to committed 2026 recruits.
- On3's valuation moved from an algorithm to deals on July 1, 2026, so its 2026 numbers do not compare with 2025's.
- **No source gives a four-star, three-star or two-star median.** I did not find one and will not invent one.

**Against AFM** (seed 1, class of 2027, Power 4 signees with a deal, from `paidset_v3_run.txt`): 5★ median $600K (real $750K-$1M); 4★ $177K; 3★ $124K (10th percentile $98K); 2★ $113K (10th percentile $95K). A Power 4 school pays 28 of its 28.5 signees. The top classes run $8.3M-$10.8M against real $6M-$9M+. So the class totals are near real, but the bottom of a class arrives at the revenue share of a rotation player.

## What the House settlement changed in the picture

[published unless marked]
- Revenue sharing began July 1, 2025. Cap $20.5M in 2025-26, up at least 4% a year ($21.3M for 2026-27). The football roster limit is 105 at schools that opted in. NIL Go reviews outside deals over $600; May-June 2026 it cleared 7,639 deals ($112.9M) and did not clear 659 ($33.7M).
- Collective money fell as the school's own money rose (Opendorse's table above), so **2024-25 and 2025-26 are different markets.** A Power 4 player who earned a few thousand dollars from a collective in 2024-25 could be on a school's revenue share in 2025-26.
- Football's share of the school's pool is the largest (65-75%). Basketball takes about a fifth to a quarter; every other sport shares a tenth or less.

## The 2025-26 yardstick for the audit's two figures

[computed, Claude's arithmetic, not a published figure]
- **Under $10,000:** about a quarter of a Power 4 roster is not paid any revenue share (the CSC's 75%), and the 10th percentile of the paid is $17,500-$25,000, so almost nobody paid is under $10,000. Third-party deals are small at most spots (medians $21,500-$65,550 among those who have one). Real 2025-26 is roughly **25-30% under $10,000**.
- **$100,000 or more:** the paid medians are above $100,000 at six of seven positions (tight ends $93,000), so about half of the 75% paid, **about 35-45% of a Power 4 roster**, is at or over $100,000 from revenue share alone. The SEC and Big Ten sit somewhat above the Power 4 average (The Athletic's rosters $27-38M against CBS's $22.5M average).
- **Correction (2026-11-24, after item 3 ran).** An earlier version of this section, and PR #632's note in the audit, called the world's 44% (SEC and Big Ten, seed 321, a 2025 payday) "at real". That 44% is one world and one year; July 2026 on seed 1 reads **66-68%** (below). The 44% is inside the real range, the 2026 figure is not. Read the two together, not either alone.

## Measured: the seed 1 world's July 2026 payday (item 3, 2026-11-24)

The seed 321 world file is not reachable from the research container (its release is not on the repo's releases list), so this ran on **seed 1** (`world-2026-seed1`, the shipped world, joined from its parts and checked against its hash), played from 1 January 2026 to 2 July 2026 on the match engine with `nil_scheme market` on main (`bbd4a30`), the tool extended with percentiles (`crates/afm-tools/src/bin/nil_scheme.rs`). Raw output: project folder `afm/nil-split/market_seed1_2026-07.txt`. The seed 1 world predates signees carrying deals, so on this payday nobody holds a deal and everyone is on the July split (`GAPS.md`); the split's own spread is exactly what is read. Seed 1 is older than the seed 321 rebuild, so the figures need one repeat on seed 321 before a lane sizes anything.

| Whole roster, everything he is paid | Under $10K | Paid anything | $100K or more |
|---|---|---|---|
| SEC | 21% | 82% | 68% |
| Big Ten | 22% | 82% | 67% |
| ACC | 23% | 80% | 67% |
| Big 12 | 24% | 80% | 60% |
| Power 4 together | 22% | 81% | 66% |
| Group of 5 | 73% | 49% | 4% |
| FCS | 87% | 46% | 0% |
| Real Power 4 (2025-26, from the CSC, computed above) | 25-30% | about 75% | 35-45% |

Revenue share among the Power 4 players paid any, 10th / 25th / 50th / 75th / 90th percentile, against the CSC:

| Position | World | CSC median | CSC other points |
|---|---|---|---|
| QB | $202K / $209K / $231K / $369K / $803K | $230K | 75th $675K, 90th $1,611K |
| OL | $116K / $121K / $135K / $270K / $508K | $133K | 90th $512K |
| DL | $130K / $136K / $151K / $269K / $551K | $150K | |
| WR | $97K / $101K / $110K / $141K / $405K | $112.5K | |
| LB | $97K / $102K / $112K / $180K / $406K | $112.5K | |
| DB | $98K / $102K / $112K / $184K / $409K | $115K | |
| RB | $94K / $98K / $106K / $122K / $382K | $108.5K | |
| TE | $82K / $85K / $93K / $150K / $324K | $93K | |
| Every spot | 10th percentile $82K-$202K | | 10th percentile $17.5K-$25K |

What it shows:
- **Medians at real** at every position (within 6%), and the offensive line's 90th percentile at real ($508K against $512K).
- **The low half is a plateau.** The 10th, 25th and median sit within about 10-15% of each other at every spot. The CSC's 10th percentile is a seventh to a tenth of its median.
- **The top is too low at quarterback**: the 75th percentile $369K against $675K and the 90th $803K against $1,611K.
- The paid share is 81% (real 75%, YSBR's range 70-83%).

## Where the world is too even, and the likely cause

The audit's symptom ("spread too evenly") is real, and item 3 puts a size on it:
- Real: a long low tail. 10th percentile among paid $17,500-$25,000; median $93,000-$230,000; 90th percentile 3.8 times the median on the offensive line and 7 times on the quarterback.
- World: one revenue-share number per position and role. `share_weight` and `share_value` give a starter 1, a rotation player 0.45 and a backup or newcomer 0.25, times a position weight (`recruiting.rs`, "the school pays its whole revenue share to its paid players", `SHARE_ROTATION`, `SHARE_DEPTH`). Within a role, two players at the same position get the same money whatever their stars, age or play. **Most paid men are backups or newcomers, so the 0.25 weight is the plateau the median sits on**: the three role weights were fitted so the median of the paid matched the CSC's median, which put every backup at about the real median and left nothing below it. The tail above is only the starters.
- The diagnostic (a deal-and-role table in the same tool) also shows `last_role`, the field `share_value` reads when a man has no `nil_role`, marks about 2,450 Power 4 men a starter and about 570 a rotation player against about 1,120 starters by this season's play (`nil_read`), so even the role weights do not separate cleanly. I did not trace that further; it is for the lane. **Traced in the lane (2026-11-25):** a newcomer's `last_role` and `last_performance` are still his high school's until his first college season is counted, and most recruits started in high school. On the July 2 rows 60% of the Power 4 men marked a starter (64% in the Group of 5, 58% in FCS) have no college season; on February 9, 37%. The new split reads only men with a college season (`recruiting::college_season`). The third party's market still counts them (below, "Left open").
- Recruits: `PaidLine::pays` (`recruiting.rs`) sets the floor of any offer to the recruit's place's July revenue share (Cain's choice on a decision card, 2026-11-19). With a flat role weight, that floor is the same for a two-star backup and a four-star who will rotate, so P4 two-stars and three-stars land at $95,000-$125,000 at the 10th percentile.
- Likely cause: [estimated] the July split has no per-man spread inside a role. Fixing it where it starts means giving each paid man a weight from his own value (his play against his level's starters, his stars while he has no season, his class year), keeping each position's paid median at the CSC's, and fitting the tail to the CSC's tail. Cain's floor then follows by itself: a backup's floor falls to his own small share.
- Systems it touches: recruiting offers and the portal's usual money (`bid_for`, `portal_usual`), the stay read (a man's money now), keep-him offers (`keep_offers`), NIL Go's book, your NIL desk's budget, the world log, the development step's payday flag (a deal over $250,000), and the saves' stored deals (`PersonCal::nil_school`).

## Proposals (all proposed, not confirmed)

Cain answered "sure" on 2026-11-24 to the first five picks (items 1, 2, 3, 4 and 5 as posted, with item 4 conditional on item 3). Item 3 has run (above) and confirms the low tail. Status:

1. **Replace the audit's yardstick.** Done in PR #632, and corrected again above: the world's under-$10,000 share is at real, its $100,000-and-up share on July 2026 is not.
2. **Keep football's share of the revenue-share cap at 0.75 and the third-party pot as football's part.** Nothing found changes it.
3. **Measure the whole paid roster before changing anything.** Done (above), on seed 1. One repeat on seed 321 is still open because its file is not reachable here; Cain's call whether to wait for it or size the lane on seed 1 and check it on the rebuild.
4. **Spread the revenue share inside each role by the man's own value** (play against his level, stars while he has no season, class year), medians kept at the CSC's, tail fitted to the CSC's (10th percentile $17,500-$25,000; offensive line 90th percentile 3.8 times the median; quarterback 7 times, 75th percentile $675K). Item 3 confirms the cause, so this is ready for a lane. My pick: yes, one lane, Opus, measured on the paid roster with this tool. **Built 2026-11-25** (below, "Item 4, built and measured"); every size in it is proposed, not confirmed.
5. **Leave the star levels' offers alone until item 4 is in, then measure again.** Five-star medians sit at $600K against $750K-$1M and the floor will move them; no source gives the other star levels. **After item 4** (seed 1's class of 2027): five-stars sign for a median $622K (before $600K). The floor did not move them, because the third party's top bid sets a five-star's offer. Four-stars take $122K (before $172K), three-stars $30K ($122K) and two-stars $10K ($111K). The five-star gap is the third party's to close: for Cain, with its refit. **The refit** (below, "The third party's refit") read NIL Go's range on the bidding market; the class of 2027's five-stars are read on the played second season there.
6. **The paid share (81% against 75%)**: decide after item 4 lands, since deal holders past the set are paid on top of it by design (`recruiting.rs`, `DEAL_FIRST`). No change now. **After item 4:** the school pays 75% of a Power 4 roster by construction. With third parties' money 79% are paid anything on July 2026 and 85% on July 2027 (85% on main too): a deal holder past the set is paid by the collective alone. My pick: leave it.

## Item 4, built and measured (2026-11-25)

Built in the thread "Spread NIL pay by player value". How it works, with every function, is in `docs/reference/how-it-works/recruiting.md`, "The revenue share by each man's own value". Every size is **proposed, not confirmed**.

**The cause.** `share_weight` knew three role weights (a starter 1, the rotation 0.45, everyone else 0.25) times the position ladder, so every role sat on one step. A recruit's floor read "the role he would have on arrival" from the depth at his spot, so a two-star at a thin spot signed for a starter's share.

**The fix.** A paid man's weight is his standing × his position's worth:
- A starter's standing is his play against his level's median starter at his spot (3% a point, the third party's own step).
- Anyone else with a college season: 1.5 × the share of his level's starters at his spot he outplays, his play read ahead for the year he grows (4.5 points going into his first or second year, 0.5 after: what seed 1's players gained).
- A newcomer, or a man who sat the season out: his stars, as his school's level reads them.
- His position's worth counts in full from the level's median paid man up and fades to nothing under him, so the bottom of every room is paid alike, as the CSC's 10th percentile is.
- One quarterback plays: only the school's first quarterback keeps growing past the median paid man.
- A recruit's floor is his stars' standing in next July's paid set, not his place by depth.

One size was fitted (the 1.5). The growth steps were measured on seed 1. The rest is read from the game each July 1 (each level's starters at each spot) or was already in the market (the play step, the star ladder, the position ladder, the paid shares).

**Measured** on the payday item 3 read (seed 1, July 2026), played again in this lane on the engine from the shipped world, once on main and once with the fix (`nil_scheme market`; raw output, the rows and the scripts in the project folder, `afm/nil-split/item4/`). Revenue share among the Power 4 players paid any, 10th / 25th / 50th / 75th / 90th percentile:

| Position | Before | After | CSC |
|---|---|---|---|
| QB | $200K / $209K / $231K / $366K / $803K | $25K / $32K / $216K / $558K / $1.65M | median $230K, 75th $675K, 90th $1,611K |
| OL | $115K / $121K / $135K / $269K / $510K | $22K / $30K / $149K / $379K / $498K | median $133K, 90th $512K |
| DL | $130K / $136K / $151K / $269K / $551K | $22K / $30K / $117K / $353K / $520K | median $150K |
| WR | $96K / $101K / $110K / $142K / $404K | $21K / $29K / $89K / $254K / $398K | median $112.5K |
| LB | $97K / $102K / $112K / $179K / $404K | $24K / $31K / $108K / $251K / $397K | median $112.5K |
| DB | $98K / $102K / $113K / $183K / $410K | $23K / $31K / $121K / $295K / $404K | median $115K |
| RB | $93K / $98K / $106K / $121K / $378K | $23K / $31K / $102K / $233K / $371K | median $108.5K |
| TE | $81K / $85K / $94K / $150K / $321K | $22K / $30K / $103K / $238K / $334K | median $93K |
| Every spot | 10th percentile $81K-$200K | 10th percentile $21K-$25K | 10th percentile $17.5K-$25K |

| Whole Power 4 roster, everything he is paid | Under $10K | Paid anything | $100K or more |
|---|---|---|---|
| Before | 22% | 81% | 66% |
| After | 25% | 79% | 44% (41-50% by conference) |
| Real (2025-26, from the CSC, computed above) | 25-30% | about 75% | 35-45% |

- **The low tail is at real**, at every position, and so is the share of a roster at $100,000 or more.
- **The top is at real** on the offensive line ($498K against $512K) and at quarterback ($1.65M against $1.611M).
- **Quarterbacks.** A school's first quarterback takes a median $1.62M from the school (before $763K; the top $2.59M), its second $380K (before $232K; the top $818K). The quarterbacks' part of a school's share is a median 14.3% (before 8.2%; the 90th percentile 19.1%), inside Opendorse's 14-23% of a budget.
- **Starters' whole pay** (third party and brand deals included) stays within 5% of before at every spot The NIL Standard names but one. With The NIL Standard's median in brackets: EDGE $927K ($929K), OT $807K ($907K), WR $693K ($726K), CB $530K ($572K), RB $580K ($527K), IOL $587K ($525K). Quarterbacks rise from $2.03M to $2.73M against The NIL Standard's $2M.
- **The Group of 5 and FCS** stay where they were: Group of 5 medians among the paid QB $51K and $29-43K elsewhere (before $66K and $21-45K), FCS $18K and $7-12K.
- **Newcomers** take a median $34K (before $112K).

**The class of 2027 and the July 2027 payday.** Seed 1 played on from the June 2026 save to July 2027, once on main and once with the fix (`nil_scheme run`, games on the stand-in; raw output in `afm/nil-split/item4/`). The class of 2027's signed offers, the share with a deal and the median:

| Signed | Before | After | Real |
|---|---|---|---|
| Power 4 five-star | 96%, $600K | 100%, $622K | $750K-$1M |
| Power 4 four-star | 97%, $172K | 98%, $122K | a four-star receiver at a top program $100K-$250K |
| Power 4 three-star | 98%, $122K | 98%, $30K | no figure |
| Power 4 two-star | 98%, $111K | 99%, $10K | no figure |
| Group of 5 four-star | 35%, $42K | 86%, $22K | no figure |
| Group of 5 three-star | 20%, $28K | 2%, $12K | no figure |
| Group of 5 two-star | 25%, $27K | 0% | no figure |
| FCS four-star | 36%, $10K | 77%, $8K | no figure |
| FCS three-star | 20%, $6K | 13%, $2K | no figure |
| FCS two-star | 34%, $6K | 3%, $1K | no figure |

- **A recruit's floor follows his stars.** Why: the floor is his stars' standing in next July's paid set. Before, it was the role the depth at his spot gave him, so a two-star at a thin spot signed for a starter's share.
- **Five-stars did not move** ($622K against $600K): the third party's top bid sets a five-star's offer, not the floor. Five-star quarterbacks sign for $1.58M-$2.42M (real: about $2.4M at a $50M school). Four-star receivers at the 15 richest schools take a median $108K (before $112K; real $100K-$250K).
- **The top classes cost less**: Georgia $5.92M, Miami $5.80M, Clemson $5.52M (before Georgia $10.84M, Auburn $9.63M, Clemson $9.52M; real $6-9M). Before, 28 signees at $111K or more cost $3M before the first blue-chip.
- **Outside the Power 4 almost no signee is paid**: 0.7 a Group of 5 program and 0.8 an FCS program at the mean (before 5.6 and 6.3). Why: those schools pay 29% of a roster, about 30 men, and a three-star freshman's standing there (about 0.13 of a median starter) is under every returning man who played, so only four-stars make the set. No source gives a real figure.
- **The portal moved a little, and the cause is not traced.** A Power 4 school lands 9.3 transfers (before 11.4; across the Power 4, 93 fewer land as backups and 45 fewer in the rotation) and signs 28.2 recruits against 27.0; a Group of 5 school lands 11.2 (10.5) and an FCS school 12.0 (11.2). The lane changes no portal code. Landing deals: Power 4 starters a median $526K (before $613K), the rotation $88K ($111K). Winter entries are unchanged (backups 21.4% against 21.2%, starters 4.5% against 4.3%), and so are keep-him offers (Power 4 median $29K against $28K) and multi-year deals (3.6% of Power 4 signees against 3.9%).

July 2027's payday follows a 2026 season played on the stand-in. Power 4 revenue share among the paid, 10th / 50th / 90th percentile:

| Position | Before | After | CSC |
|---|---|---|---|
| QB | $111K / $179K / $987K | $10K / $203K / $1.68M | median $230K, 90th $1,611K |
| OL | $64K / $114K / $442K | $10K / $74K / $559K | median $133K, 90th $512K |
| DL | $78K / $173K / $493K | $11K / $106K / $611K | median $150K |
| WR | $55K / $92K / $398K | $10K / $47K / $452K | median $112.5K |
| LB | $52K / $91K / $321K | $9K / $62K / $426K | median $112.5K |
| DB | $53K / $94K / $365K | $9K / $65K / $449K | median $115K |
| RB | $52K / $87K / $306K | $9K / $42K / $386K | median $108.5K |
| TE | $48K / $78K / $297K | $9K / $70K / $366K | median $93K |

| Whole Power 4 roster, July 2027 | Under $10K | Paid anything | $100K or more |
|---|---|---|---|
| Before | 17% | 85% | 56% |
| After | 22% | 85% | 38% (34-43% by conference) |
| Real | 25-30% | about 75% | 35-45% |

- **The tails and the shares hold in the second year.** The share at $100,000 or more and under $10,000, the quarterbacks' 90th percentile and the line's are at real or near it. A school's first quarterback takes a median $1.59M, its second $322K, and the quarterbacks' part of a school's share is a median 15.2% (the 90th percentile 20.5%).
- **The medians read low there, and the stand-in season is why.** The stand-in keeps every man's camp role and reads his play its own way: a Power 4 school has 23.2 returning starters against 17.6 after the engine-played 2025 season, their play is tighter (10th percentile 54.7 against 51.4), and 47.7 men a school have no play to read against 38.0. So a man who didn't start outplays fewer of his level's starters: the rotation's median standing is 0.42 against 0.63 and a backup's 0.13 against 0.26, and 44 scholarship men a school stand at a four-star's 0.25 or better against 49. With nobody holding a deal the same rosters split to medians of RB $43K, WR $50K, LB $72K, TE $80K, DB $88K, OL $105K, DL $132K and QB $221K. Giving each returning man the standing a man at his rank had after the engine-played season (same position group and year in school) brings them to RB $90K, WR $104K, TE $119K, LB $127K, DB $139K, DL $150K, OL $175K and QB $273K [Claude's estimate, not a played season]. A second season on the engine would read the medians; the stand-in can't.
- **Deals move the bottom.** On that payday 6,579 of 7,879 Power 4 men hold a deal (4,200 a keep-him offer, 1,882 the deal they signed, 497 a transfer's); on July 2026 43 of 7,583 did. A deal is paid in the program's ratio of its two pots (`nil_payouts`), so a three-star's $30K floor is $19K of revenue share and a two-star's $10K is $8K: the paid 10th percentile reads $9-11K (real $17.5-25K; $21-25K on the 2026 payday). Deal holders also fill the paid set first, so the 14 second-year men a school who redshirted, hold no deal and have no play for a rival to price are paid by the school 6% of the time where their standing would pay 85% of them a median $27K. Both rules are older than this lane.
- **Starters' whole pay** in 2027: QB $2.59M, OT $1.04M, EDGE $919K, WR $771K, RB $667K, IOL $663K, CB $621K (before $1.89M, $826K, $752K, $631K, $523K, $513K, $476K; The NIL Standard's 2026 medians $2M, $907K, $929K, $726K, $527K, $525K, $572K). The stand-in's 23 starters a school take 73% of the money (62% before), so this too waits for a played season.
- **The top runs past real.** The best-paid quarterbacks make $10.94M, $10.12M, $9.16M and $9.08M in 2027 (before $7.88M, $6.65M, $5.66M, $5.54M) and $9.66M, $8.86M and $8.29M on the 2026 payday (before $8.27M, $8.01M, $7.83M); real stars make about $4-7.5M. The school now pays a first quarterback what the CSC shows, on top of a third-party market fitted when it paid him half of that.

**Left open** (each is in `GAPS.md`):
- **The medians are unread on a played second season.** The stand-in's July 2027 can't judge them (above). One `nil_scheme run <world> 2027 8` with its games on the engine would, at about an hour of machine time.
- **A deal holder's revenue share is his deal in the program's ratio**, not his standing's share, so the paid 10th percentile falls to $9-11K once a class holds deals, and a transfer who lands in the rotation makes his deal, a median $92K, where a kept man in the rotation makes $178K.
- **A keep-him offer takes a place first**: the second-year men without a deal are shut out of the paid set (above).
- **Refitted 2026-11-25 (below): $2.21-2.29M on the depth chart's first line, the top $6.2M.** A starting quarterback's whole pay is $2.73M against The NIL Standard's $2M, and the top is $9.7M against a real $4-7.5M. The third party's quarterback market (`nil_fit` 3.2) was fitted while the school paid a starting quarterback about half of what it pays him now. The CSC's own figures put a first quarterback at about $1.5M from the school and about $1M from third parties [estimated from its 90th percentiles: a first quarterback is about one paid quarterback in five], so the two real sources disagree at the median; the top agrees with neither. The lane left the third party alone.
- **Fixed 2026-11-25 (the third party's refit, below).** The third party's bar still counts newcomers' high school roles (the trace above): its starters' medians read 7-13 points low in July (Power 4 quarterbacks 49.6 against 61.1 on men with a college season) and 1.5-7 points low in February. Fixing it needs a refit of the third party's weights, so it is its own lane.
- **Five-stars** sign for $622K against a real $750K-$1M (item 5).
- **Two medians run a fifth under on the 2026 payday**: receivers $89K (real $112.5K) and the defensive line $117K ($150K). Each room's median reads its own mix of starters, backups and newcomers in the paid set; a refit of the position ladder was tried and dropped, because it moves the tails off real.
- **The quarterbacks' 75th percentile** is $558K against $675K.
- **Specialists** have no real figure; at 0.4 of a receiver they take a paid median $125K.
- **The portal's small move** (2.1 fewer transfers a Power 4 school) is not traced.
- **Seed 321** is still unread: its file is not reachable from a thread container.

## The third party's refit, built and measured (2026-11-25)

Built in the thread "Refit third-party NIL market" on Cain's proxy (every pick is in `docs/decisions.md`, "The third party's NIL market refit"). How it works, with every function, is in `docs/reference/how-it-works/recruiting.md`, "The third party's market". Every size is **proposed, not confirmed**. No constant was fitted: the lane removed the third party's fitted position factors and reads the bidding market's own spot shares, which are The NIL Standard's ratios.

**The causes.** Three, each traced on seed 1's July 2026 payday.

1. **The starters' bar counted high school roles.** `nil_medians`, `bid_lifts` and the star lifts read anyone on a college roster whose last role was a starter's, and a signee carries his high school role until his first fall camp. In July 60% of the Power 4's "starters" had no college season and the bar sat 7-13 points low (quarterbacks 49.6 against 61.1), so 43% of last season's real starters read as 10 or more points over a median starter (13% on the real bar).
2. **The third party had a second market.** It split its pot by a payroll weight with a fitted factor a position (quarterbacks 3.2), whatever the school paid the same man. It was fitted when the school paid a first quarterback about half of what item 4 pays him.
3. **July read last season's roles.** A Power 4 school had 12.9 of last season's starters on its roster and 8.4 of them on the first line of the chart fall camp would set, of 24. Most starting jobs had nobody priced as a starter, and the pot went to the few who were.

**The fix.** One market for every third party's dollar: a man's market price at his school is its year of money × his spot's share × his talent, the school's top bid for him. On July 1 the school pays his revenue share, and the third party fills every man from his revenue share up to the same multiple of his price, the multiple that spends its pot. His role is today's depth chart's. The bars read only men who started a season at their school. A man with no college season and no deal gets no third-party money. NIL Go's range reads the same market.

**Measured** on seed 1 played on the engine from the shipped world to July 2, 2026 (`nil_scheme market`; main b7da97c against the lane; brand deals included).

Power 4 starters' whole pay, medians:

| | QB | EDGE | OT | WR | CB | RB | IOL |
|---|---|---|---|---|---|---|---|
| The NIL Standard, 2026 | $2.0M | $929K | $907K | $726K | $572K | $527K | $525K |
| The chart's first line, before | $2.07M | $643K | $614K | $467K | $428K | $474K | $427K |
| The chart's first line, after | $2.21M | $890K | $925K | $727K | $596K | $551K | $514K |
| Started last season, before | $2.73M | $926K | $809K | $693K | $530K | $580K | $592K |
| Started last season, after | $1.80M | $722K | $630K | $613K | $476K | $445K | $513K |

The first-line rows are the same men on both paydays, joined by id (the tool's own read of the whole first line after: QB $2.29M, RB $557K, the rest as above). The earlier lanes read "started last season"; a third of those men are no longer on the first line in July, so the first line is the like-for-like read of The NIL Standard's starters.

- **Every spot but quarterback is within 5% of real** on the first line (10-36% under before). Quarterbacks are 11-14% over.
- **Inside the first line the pot is spread, not piled.** Returning starters on it took a median $5.13M at quarterback before and the other men $1.19M; now $3.27M and $1.94M. At receiver $859K and $280K, now $839K and $589K.
- **The top.** The biggest checks were $9.49M, $8.85M and $8.45M; now $6.20M, $5.94M and $5.31M (real stars about $4-7.5M). The 90th percentile among quarterbacks who started last season fell from $7.89M to $5.13M. Texas's returning quarterback (his play 62.1, a point over the Power 4's median starter): $8.45M to $4.98M.
- **The revenue share and the whole roster are where they were.** Paid medians QB $223K, OL $149K, DL $117K, WR $90K. A whole Power 4 roster: 26% under $10K (before 25%, real 25-30%), 76% paid anything (79%, real about 75%), 43% at $100K or more (44%, real 35-45%).
- **Who the third party pays.** A median 25.5 men of a Power 4 roster, 28% of its money. By last season's role, starters take 43% of all the money (52% before), the rotation 16% (12%) and everyone else 42% (36%); by the chart, starters 69%, the rotation 11%, everyone else 20%.
- **Below the Power 4.** The Group of 5 pays 36% of its men anything (52% before; the school pays 29%). The revenue share is unchanged (QB $52K, $29-43K elsewhere). The first line makes $88K at quarterback and $35-55K at the other spots. FCS: 37% paid anything (50%).
- **NIL Go** on the same unit had denied 7.6% of 2026's reviewed deals by July 2 and 18% of their dollars (real 7.9% and 23%).
- **A rival's bid reads lower on the real bar.** The market check's price for the men who started last season (the best rival's top bid, the day before): QB $1.41M (was $1.99M), EDGE $746K ($987K), OT $677K ($830K), WR $579K ($745K). July pays a starter a median 1.23 times it.
- **Seed 1's class of 2026 holds no signing deals** (the world file is older than they are), so its Power 4 five-stars take the school's share alone that July: a median $182K (before $299K).

## Deals in later years, built and measured (2026-11-25)

The second PR of the thread "Refit third-party NIL market", on Cain's proxy (picks in `docs/decisions.md`, "Deals in later years"). How it works: `docs/reference/how-it-works/recruiting.md`, "Deals in later years". Every size proposed, not confirmed.

Seed 1, played on the engine from the same June 29, 2026 save to the July 2027 payday, once on the refit (before) and once on this lane (after); `nil_scheme run <save> 2027 8`, then `nil_scheme market`. About 32 minutes a side. Raw output in the project folder at `afm/nil-market/`.

Power 4 revenue share among the paid:

| Spot | 10th pct before | after | real | Median before | after | real |
|---|---|---|---|---|---|---|
| QB | $10K | $25K | $17.5-25K | $202K | $223K | $230K |
| RB | $10K | $19K | | $49K | $60K | $108.5K |
| WR | $10K | $23K | | $68K | $71K | $112.5K |
| TE | $9K | $22K | | $75K | $78K | $93K |
| OL | $10K | $22K | | $85K | $111K | $133K |
| DL | $10K | $22K | | $98K | $119K | $150K |
| LB | $10K | $22K | | $71K | $80K | $112.5K |
| DB | $9K | $21K | | $63K | $76K | $115K |

Who the school pays (Power 4): last season's redshirts 10% before and 82% after (a median $31K); men with no deal 5% and 50%; two-star newcomers 87% and 29%; three-stars 94% and 73% (a median $21K and $30K); four-stars 90% and 91% ($60K and $97K); walk-ons paid anything 590 of 766 and 218.

The whole Power 4 roster: 21% under $10K before and 18% after (real 25-30%); 86% and 87% paid anything (real 75%); 40% and 41% at $100K or more (real 35-45%). Of the 87%, 897 men are paid by the third party alone: 580 signees with a deal outside the school's set, 241 keep-him holders, 52 transfers.

The first line with a college season, whole pay: QB $2.32M before and $2.37M after (real $2M), OT $971K and $896K ($907K), EDGE $910K and $884K ($929K), WR $714K and $676K ($726K), CB $603K and $589K ($572K), RB $553K and $551K ($527K), IOL $591K and $596K ($525K). The biggest checks: $7.32M, $5.77M, $4.95M before; $5.67M, $5.62M, $4.82M after.

Keep-him offers before the 2027 window: Power 4 4,740 before and 3,740 after (a median $28K and $47K), Group of 5 4,893 and 2,759 ($2K and $11K), FCS 8,463 and 4,936. FBS portal entries by February 1: starters 4.2%, the rotation 7.5%, backups 21.4% on both.

The Group of 5: 62% of a roster paid anything before and 44% after (the school pays 29% and 33%); FCS 62% and 46%.

Why the medians at four spots still read low: a Power 4 school has 19.1 of last season's starters back in July 2027 against 12.9 in the July 2026 file, so starters take more of the school's pot. On the July 2026 payday the same split read RB $100K, WR $90K, LB $109K, DB $122K. Not traced (`GAPS.md`).

## Sources

- Opendorse, College Football Kickoff (Aug 2025): https://biz.opendorse.com/wp-content/uploads/2025/08/College-Football-Kickoff-How-NIL-is-Reshaping-the-Game-in-2025-August-2025.pdf (not opened; via the coverage below)
- Opendorse, NIL at Four (Jul 2025): https://biz.opendorse.com/wp-content/uploads/2025/07/NIL-at-Four-Monetizing-the-New-Reality_Final.pdf (not opened)
- Yahoo Sports / Sporting News on the Opendorse report: https://sports.yahoo.com/article/opendorse-report-details-power-four-203651154.html
- Front Office Sports: https://frontofficesports.com/college-football-players-projected-to-earn-1-9b-this-year-nearly-double-2024/ , https://frontofficesports.com/college-hoops-accounts-for-nearly-30-of-revenue-sharing-payments/ , https://frontofficesports.com/as-nil-turns-three-collectives-and-football-still-control-the-industry/
- CBS Sports on the revenue-share split: https://www.cbssports.com/college-football/news/how-historic-house-v-ncaa-settlement-will-impact-college-athletics-on-and-off-the-field-for-years-to-come
- CSC pay data by position: https://foxsports.com/stories/college-football/nil-players-salary-money-position-quarterback , https://www.on3.com/nil/news/how-much-do-football-players-make-nil-revenue-sharing-college-sports-commission-full-data/ , https://rg.org/news/football/college-sports-commission-football-pay-data-quarterbacks
- Youth Sports Business Report on who gets paid: https://youthsportsbusinessreport.com/what-the-first-college-athlete-compensation-data-says-about-who-actually-gets-paid-1-6-million-quarterbacks-and-500-rowers/
- Business of College Sports on the 75% and 29%: https://businessofcollegesports.com/name-image-likeness/what-college-football-players-really-earn-from-nil-and-revenue-sharing/
- SI on class averages: https://www.si.com/college/oregon/football/nil-recruiting-class-oregon-ducks-ohio-state-buckeyes-rankings-deals-money-maryland-dan-lanning-ryan-day-name-image-likeness
- AFM: `docs/trials/realism-audit-seed321.md` item 14; `docs/reference/how-it-works/recruiting.md` "NIL money"; `data/real-world/rates.json` `nil`; `docs/research/rules-and-money-2026.md`; `afm/nil-gaps/paidset_v3_run.txt`.
