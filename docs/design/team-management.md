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
