# Section 5: team management (design in progress)

Design thread "Section 5 plan", started 2026-10-16 with Cain. Real life is the default: anything real exists and works the way it really does. Only choices real life doesn't settle are asked.

Already decided before this section (build work, no questions): roster rules (Q27 and the 2026 research numbers), depth charts and who plays (trial 5 calls, the depth and rotation physics fix, calendar topic 10), cap rules (Q23), rookie contracts (Q25; the UDFA rush is Q34, section 7), eras (calendar topic 11), the 2026 start (it falls out of the 30-year history), honors voting (real rules and real voter habits).

Topics: 1 the living NFL; 2 contract value; 3 agents; 4 negotiations (its own topic, Cain 2026-10-16); 5 free agency and the street pool; 6 trades; 7 AI front offices; 8 scheme fit and value.

## Topic 1: the living NFL (confirmed 2026-10-16)

Real facts to build:
- Every NFL player is the same person who came up through high school and college. Development, ageing, cuts and retirement run on the model in `docs/development-model.md`. No background players.
- Rosters follow each era's real counts. Today: 90 in the offseason, 53 in season, 48 active on game day, 16 on the practice squad plus 1 International Player Pathway spot (from 2017). Raw international athletes (rugby, track, Aussie rules) enter each year.
- At the start of history the league is filled with full generated veterans, spread like a real 1990 roster. World classes replace them.
- Every NFL game runs on the engine, history included: preseason by era (four, none in 2020, three from 2021), the regular season and the playoffs. Preseason playing time follows each staff's habits.
- The league follows history: 28 teams to 1994, 30 from 1995, 31 from 1999, 32 and the realignment from 2002, every move and name change on its real date. (For the calendar build: its schedule uses today's 32-team alignment for every year.)
- Cut and undrafted players stay in a real free agent pool; teams sign from it all year.
- The other pro leagues exist on their real dates and play on the engine under their own rules (CFL, Arena, NFL Europe, XFL, AAF, USFL, UFL). Players choose between them and waiting for an NFL call.

Cain's answers:
1. **Snaps drive pro development.** Cain: "Absolutely, i'm shocked it hasn't, the amount of snaps and the quality of snaps matter just as much as the EPA". Development reads real snaps and how hard they were (the opponents he faced), plus practice reps by role. A practice squad player grows mostly on technique and the playbook.
2. **Rust.** Cain: "Agreed, but 3 game max". A hidden game sharpness drops while a player is out of football (faster with low work ethic), costs a little timing and reading of the game, and is fully back within at most 3 games.
3. **Waiting or retiring while unsigned.** Cain: "As dynamic as possible". Each week an unsigned veteran weighs his chance of an NFL call (injuries around the league at his position, his own level) against his age, career earnings, family, love of the game and offers from other leagues. Most give up by midseason; a few hold out for a September injury call.

## Topic 2: contract value (in discussion)

Notes from Cain (2026-10-16, not answers to the questions yet):
- The cap must grow realistically.
- Non-QB franchise players get paid by what they are to their team. Cain: "a Bijan or Jahmyr Gibbs getting record setting, it should up the floor off the RB's but not by a set amount since everyone just knows they're much more then a regular RB", unlike "a Kenneth Walker who is good but not franchise". Claude's reading (proposed, not confirmed): the price comes from the player's real share of his team's offense (snaps, touches and targets, explosive plays, how much the plan runs through him, how far he is above the replacement at hand), not the position label. A record deal pulls the comparables up only for players the market sees as that kind of player; a good starter's comparables stay the good starters.
- The real case, looked up 2026-10-16 (Wikipedia player pages, Pro Football Reference 2026 rushing). 2025 season: Robinson 287 carries, 1,478 yards, 5.1 a carry, 79 catches for 820 yards; Gibbs 243 carries, 1,223 yards, 5.0, 77 catches for 616; Walker 221 carries, 1,027 yards, 4.6, 31 catches for 282 (2024: 3.7 a carry in 11 games). 2026 through week 5: Robinson 580 yards at 5.7, Gibbs 504 at 4.8 plus 243 receiving and 9 TDs, Walker 537 at 6.2 in 4 games. Deals in 2026: Robinson 3 years up to $75M, $51M guaranteed (August 4, a record); Gibbs 3 years $67.5M (up to $75.75M), $51.5M guaranteed (August 6); Walker, Super Bowl LX MVP, 3 years $45M with Kansas City in free agency (March 12). The split is not rushing (Walker matches them in 2026). It is the receiving role, playing every game for three years, steady efficiency every year, top-12 draft pedigree and age at the second deal.
- Cain (2026-10-16): stat padding shows in yards per play, so pricing reads efficiency per play over totals. Claude's addition (proposed, not confirmed): per-play numbers are judged with the line in mind (yards before and after contact, success rate), and a small sample counts less.
- Depends on the engine's position values being right first (dynasty item 1, approved and in progress in "Fixing the dynasty, part 2": RB 0.79 of QB value, target 2.4 points per lineup point).
- **Question 1 answered (Cain, 2026-10-16):** a hidden stats-or-film lean per GM, and "teams should still reach". Cain: "a player on a bad team getting 1400+ yards isn't a 'bad player'. I'm just trying to make sure a derrick henry back on the titans doesn't go unnoticed in the game". So efficiency is judged in context: stacked boxes, a defense keyed on him, a weak line and a weak passing game around him make his numbers worth more, not less. Volume carried against that is itself evidence. Only empty volume (low efficiency with no hard context) is marked down, and old-school GMs still overpay for totals.
- **Question 2 answered (Cain, 2026-10-16):** "for non-decision years 8% and for decision years research the average of the last few similar years". Real cap (NFL.com): 1994 $34.6M ... 1997 $41.5M, 1998 $52.4M, 2005 $85.5M, 2006 $102.0M, 2009 $128.0M, 2010 none, 2011 $120.0M, 2013 $123.0M, 2014 $133.0M, 2020 $198.2M, 2021 $182.5M (COVID), 2022 $208.2M, 2026 $301.2M. Media-deal years: 1998 +26.2%, 2006 +19.3% (also a CBA extension), 2014 +8.1%, 2022 +14.1%. Rule: normal years +8%; a year a new media deal starts gets the average of the last three media-deal jumps (13.8% from 2006, 2014, 2022). A lockout year follows 2011 (no cap the year before, then 6.3% below the last capped year). Claude's addition (proposed, not confirmed): the average rolls forward with the world's own media deals after 2026.
- **Question 3 answered (Cain, 2026-10-16):** "make it personality based yes". No flat contract-year boost; a driven or money-hungry player plays harder and through knocks (more injury risk), a cautious one protects his body, most don't change.

Topic 2 is confirmed.

## Topic 3: agents (confirmed 2026-10-16)

Real facts to build: every player has an NFLPA-certified agent (fee capped at 3%); a few negotiate alone (Lamar Jackson). Agencies range from one-man shops to giants; size brings leverage and information. The player sets the goals (money, guarantees, years, contender, role, home) from his hidden personality, and can overrule his agent. Deadlines make deals (the tag deadline in mid-July, camp report day, week 1). Since the 2020 CBA camp fines are mandatory and can't be forgiven for players on rookie deals, so stars often hold in (Brandon Aiyuk 2024) or hold out (CeeDee Lamb 2024). The combine is the agents' meeting place (calendar A11).

Cain's answers:
1. **Agents bluff.** Cain: "Definitely and bad bluffs should hurt the players causing realistic disputes and firing". An agent's hidden honesty sets how often he claims interest that isn't there; a GM reads it by judgment and history with that agent. A bluff that is called hurts the player (an offer cut or pulled, a worse relationship with the team), can start a public dispute, and can lead the player to fire his agent.
2. **Agent-GM relationships carry across clients.** Cain: "Yes, absolutely doesn't need to be said, it's ultra realism game".
3. **How talks run.** Cain: "You're downplaying the importance of negotiations. i'm expecting a ultra-realistic but gamified aspect in the game", and "I want a separate topic for the negotiations". Moved to topic 4.
4. **Holdouts, hold-ins and trade requests.** Agreed as proposed: each day the player weighs how underpaid he is against the market, years left, leverage (age, tag risk, team need), fines and personality (greed, loyalty, leadership, temper), with a readable reason.

## Topic 4: negotiations (in discussion)

Real facts to build: your side is you, your cap manager and, for big money, the owner (big guarantees need his sign-off; a meddling owner can take over, as Jerry Jones did with Micah Parsons before the 2025 trade to Green Bay). Every real term is negotiable (average per year, years, signing and option bonuses, roster and workout bonuses, guarantees at signing, injury guarantees and vesting dates, escalators, likely and unlikely incentives, void years, no-trade clauses, rookie offset language; Shemar Stewart's 2025 holdout was over guarantee wording). Agents want the headline average, teams want low practical guarantees and a cap shape that fits. Every real move exists for both sides (deadlines, best and final, pulling an offer, tag threat, trade shopping, going to the player; bluffs, silence, leaks, social posts, holdout threats, trade requests). Leaks shape the public story and burn the relationship. Free agency is a race from the legal tampering window. AI teams negotiate under the same rules, history included.

Question 1 (mood as a bar or words) was rejected by Cain: "Neither it feels lackluster... especially in this day and age where the financial aspects are damn near predatory. It needs to be more substantial". Replaced by 1a-1e, all agreed (Cain, 2026-10-16):
- **1a. A term-by-term board:** every term with your offer, his last ask and his stated stance ("firm", "open", "won't discuss"); both sides bring comparable deals and can dispute them.
- **1b. Cap staff analysis of every draft deal:** real cash in years 1-3, practical guarantee, cap hit and dead money if cut in each year, rank at the position, and an estimate of the agent's real floor with a confidence range. Accuracy follows the cap manager's rating; a weak staff can be confidently wrong.
- **1c. Tells instead of a meter:** reply speed, counter or silence, leaks, the player's posts, skipped voluntary OTAs, reported interest from other teams (possibly a bluff).
- **1d. Traps in the fine print:** injury guarantees vesting just after a likely cut date, back-loaded "funny money", void years, offset and guarantee-voiding language; per-game roster bonuses, escalators, early vesting from the agent's side. Spotting one depends on staff and agent skill; a burned player, and his agent's other clients, remember.
- **1e. Sit-downs with the player apart from his agent:** his personality shows and can move him off the agent's line; going around the agent risks the agent's goodwill.
- **2. Promises (agreed):** logged with a date, shown on the term board; a broken promise costs trust with the player, his agent's other clients and the locker room, by how public it was; a wary player can ask for it in writing as a real clause.
- **3. Your negotiating rating (agreed):** sets how well you read tells and bluffs, sense his floor and spot his traps; works alongside the cap manager's rating (1b); grows with deals; your background sets the start.
- **4. Pace (agreed):** each offer waits a day or more; busy agents and calm or content players are slower; deadlines compress talks (most deals in the last 48 hours); a stalled talk cools and he starts reading the market.

Topic 4 is confirmed.

## Topic 5: free agency and the street pool (in discussion)

Real facts to build: UFA at 4+ accrued seasons, RFA at 3, ERFA below (an accrued season is 6 games on full pay status). The legal tampering window opens two days before the new league year (4pm ET); stars go in hours, then the second wave and bargains, post-draft signings, June 1 cuts, summer veterans, camp injuries and the cutdown flood. Cap room isn't spending: owners' budgets and the 90% cash spending floor. The player weighs money after state income tax, guarantees, a contender, role, scheme, coach, home, climate, market size, friends on the roster. Early money is highest; waiters can be squeezed or cash in; one-year prove-it deals. The street pool works all season. Every signing runs through topic 4's negotiation with several bidders.

Cain's answers (2026-10-16):
1. **AI free agency boards (agreed):** each AI team ranks targets by its own grades and needs with a walk-away price; it moves down the board as targets sign elsewhere; a desperate team overpays.
2. **Weighing offers (agreed, plus):** "Agree climate and party city aspects too. New york and dolphins will have a leg up despite being non-competitive". Hidden priorities set the weights; take-home after state tax counts; climate and nightlife cities (New York, Miami) pull players even when the team is losing.
3. **When he signs: the perfect offer (Cain's mechanic):** "Give players a perfect offer possibility, if that is reached contract is signed on the dot (doesn't have to be financial but does need to be balanced) if a 85% is attained, he'll likely accept but a 7 day timer starts in which he can be swayed. Be careful with this dynamic". Each player has a hidden ideal package built from his own weights (money, guarantees, role, contender, city, promises). An offer that reaches it is signed at once. One at 85% or more is likely accepted, but a 7-day window opens in which other teams can sway him. Claude's safeguards (proposed, not confirmed): "balanced" means no single term can be stretched to cover the rest (a huge role promise can't stand in for money he cares about); the ideal is hidden and read only through tells, the agent and sit-downs; AI teams work under the same rule; during the tampering window an agreement waits for the new league year like real verbal deals.
4. **Comp picks:** Cain: "If a coach has been active longer than 2 off-seasons, it's a third round. If shorter it's relative to ranking. The lower the ranking the higher the pick". (Being clarified: see the thread.)

## Topic 6: trades (confirmed 2026-10-16)

Real facts to build: trades from the new league year to the deadline (Nov 10 in 2026); deals after the Super Bowl wait for the new league year. Players, picks up to three drafts ahead (comp picks since 2017), conditional picks and pick swaps. No salary retention; teams eat money by converting salary to bonus before a trade; remaining signing bonus accelerates onto the trading team's cap. No veto without a rare no-trade clause; physicals can void a trade. No fairness veto (the 1989 Herschel Walker trade). AI to AI trades (Q26); draft-day trades in section 7.

Cain's answers:
1. **Each GM's own pick value chart (agreed):** his lean picks the chart (classic Jimmy Johnson style or a flatter surplus-value chart); future picks discounted by his urgency.
2. **Trade value is worth minus contract (agreed, "but delicate"):** the team's own grade over his remaining years minus his cost, adjusted for need; old-school GMs weigh name and stats more. Handle with care in the build and check it in the trial.
3. **GM to GM talks on the topic 4 table (agreed):** term board, tells, leaks; hidden GM-GM relationships; a GM who fleeced another gets fewer calls back.
4. **Shopping rumors hurt the player (yes):** a leaked shopping effort costs morale and trust by personality; a loyal veteran takes it worst; some play angry.

## Topic 7: AI front offices (in discussion)

Real facts to build: each GM carries his own style (draft-and-develop like the Packers and Steelers; the Saints' yearly restructures; the Eagles' void years and early extensions; some won't pay RBs, some always pay the lines), and it moves with him. Teams load up in a rookie-deal QB window (the 2021 Rams: Stafford, Ramsey, Von Miller, Super Bowl LVI). Resets: one big dead-money year, or stripping to youth and picks. A hot-seat GM trades the future for now; owners demand splashes, block rebuilds or take over talks. Your own staff decide the same way (calendar topic 10).

Cain's answers (2026-10-16):
1. **Contend or rebuild:** agreed as proposed (the GM's own read of roster, QB window, cap 2-3 years out, core age, division, owner patience, job security, with a readable reason), plus Cain: "Add a fan dynamic and general opinion aspect to it": fan mood (results, hype, star signings, season ticket renewals A17, market size) and the media story feed the call, felt most by the owner.
2. **Where styles come from:** agreed (learned from the front office he came up in, analytics spreading by era, winners copied).
3. **Human biases:** "Yes, look at Deshaun Watson who despite all got picked over Shedeur Sanders, while JJ McCarthy got traded after 2 seasons because they have no trust. So expand on these strengths weaknesses and tie them into owner involvement." Expanded list 3.1-3.17 posted in the thread, waiting on Cain.
