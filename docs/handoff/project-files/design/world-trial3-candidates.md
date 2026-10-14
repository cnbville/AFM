# Candidates for world trial 3

Flags from world trial 2 (2026-10-13), parked at Raichel's request: "list them up for potential trial 3".

**Status 2026-10-14: built and merged to main (merge 0c4c536).** What changed for each item is in `docs/world-model.md`, under "Trial 3 fix list". Items 9, 10, 11 and 14 are checks for world trial 3. Last check run: pick 1 a QB 58-62% (real 73%), three-stars drafted 4.8% (real 6.0%), two-stars 1.0% (real 2.1%), longest Power 4 drought 21 seasons. Raichel's decisions on 2026-10-14: every college game runs on the engine, history included (30 years in the game, 15 for trial 3); for dynasties, position room plus portal plus linked units ("A QB won't like having horrible receivers. A corner won't like playing with bad D-line").

1. **Name data: Trey reads as mostly white** (371 of 407 players). Only one source (mortgage records) has the name.
2. **Name data: DeShawn is missing.** It's in neither source.
3. **The coaching gap is too wide in the 2000s.** In 2009 there were 3 Black FBS head coaches of 138 (real: 7 of 120). 2025 is right (17 vs real 16).
4. **Dynasties are still strong.** Alabama won 9 titles in 30 years (real high: 6). This carries over from trial 1's flag 1.
5. **An unrealistic hire.** A 66-year-old FCS defensive coordinator with no head-coaching record got the Florida State head job.
6. **Bench reps run about 3 high for skill players** (WR 17.8 vs 14.7, CB 17.6 vs 14.5).
7. **Kickers and punters run heavy** (kickers about 8 lb).
8. **Height format on player pages.** It shows 6'0.8" instead of 6'0 3/4".
9. **Draft mix in round 1.** EDGE runs a little low (10% vs 17%) and IOL a little high (10% vs 5%). A QB goes first 58% of the time (real: 73%).
10. **Star draft rates.** Five-stars run a little high (66% vs 63%); three-stars (4.8% vs 6.0%) and two-stars (1.1% vs 2.1%) run low.
11. **A long-term SEC doormat.** Mississippi State had no winning season in 30 years. Check whether a Power 4 program can stay that bad that long.
12. **The stand-in game model.** It gives odd champions, such as Pittsburgh with 3 titles. Section 3 replaces it.
13. **NIL offers per player** (added 2026-10-13, after Matthew Simmons picked Pittsburgh over Texas). Today each offer is just the collective budget split over the class and scaled by his board rank, and money is scored on a log scale that flattens big offers ($3.5M scored barely above $2.0M).
    - **Position value (confirmed by Raichel):** "Position value is a huge aspect, use realistic data." Build offers from the real NIL ranges by position in `data/real-world/`.
    - **Bidding (confirmed):** schools raise offers to beat rivals.
    - **The "new era" reach (confirmed, Raichel's own idea):** "add a dynamic where a lower tier school with a good NIL can reach in order to try to bring a new era to the school. Think Colorado Buffaloes or Texas Tech or Indiana Hoosiers. They had a good coach and were on the brink of becoming better so they reached with an offer. This is situational though, not permanent."
    - **Money weighed by family money and ambition (confirmed):** "it should differ based on family money... a 5* that is poor will still care a lot, 5* that's suburban will care but is not scared to take less for a percentage. Someone who is wealthy and 5* will take less, but also tie it to ambition. If someone is ambitious he cares more about the best situation. A delicate dynamic." Reading: poor = money matters a lot; middle class = will take a smaller share for a better fit; wealthy = takes less; high ambition shifts weight from money to the best situation (playing time, development, winning). Each is a lean, not a rule.

### Item 13 in full: how money weighs (proposed, not confirmed beyond Raichel's quotes above)
- **Money need, not one number.** Each recruit gets a hidden money need from family money (poor high, suburban middle, wealthy low), shifted by: a family crisis (job loss, a death), a handler or parent who wants the payday, siblings to support, and his own personality.
- **Ambition pulls the other way.** High ambition moves weight from money to the best path to the NFL: playing time, development, the coach, the pro pipeline, winning. So an ambitious poor kid still cares about money, but less than a poor kid with low ambition.
- **"Taking less" has a limit.** Every recruit has a floor below the top offer he will accept, set by money need. Poor and low ambition: he takes the top offer unless the gap is small. Suburban: he'll give up a share (say 20-40%) for a clearly better fit. Wealthy and ambitious: money is close to a tiebreaker.
- **Real money scale.** Offers are compared as share of the best offer and as what they mean to his family, not on a flattening log scale.
- **The decider matters.** If a parent or handler decides, their own money want counts too (the Tennessee mom example from topic 5).
- **It can change.** A family event in senior year can raise the need; NIL backfire and a later portal move can shift it again.

14. **Draft position mix, whole first round** (tuned 2026-10-13 at Raichel's request, not re-measured after the last change to save tokens; check in trial 3). Every pick weighs grade, the team's board, position value and team need. Changes: QB premium tied to team need (except at the very top), top-5 trade-downs by teams with a franchise QB, lesser positions count for less at the very top, EDGE and CB up, IOL down. Last measured run (three seeds, before the final pick-1 fix), sim vs real:
    - Pick 1: QB 39 / 73, EDGE 38 / 19, OT 14 / 8. The final fix restores the pick-1 QB rule; an earlier run had QB 63, EDGE 22, OT 13.
    - Picks 1-5: QB 29 / 28, EDGE 23 / 21, OT 17 / 15, WR 10 / 11, CB 9 / 7, RB 3 / 8.
    - Round 1: QB 11 / 9, WR 15 / 13, EDGE 13 / 17, OT 12 / 12, CB 11 / 12, DT 8 / 10, IOL 5 / 5.
    - Round 2: EDGE 9 / 14 and IOL 11 / 8 still off.
    - Worry: EDGE runs low after pick 5 and RBs low in the top 5.
15. **Coach hiring and program rises (agreed by Raichel 2026-10-13; build after section 3).** Findings in `design/coach-promotion-and-team-growth-check.md`.
    - **A. "Hot high school coach" uses his own seasons.** Bug: the check reads the school's last two seasons, so a new coach can get credit for his predecessor's title (`staff.rs` `hs_buzz`).
    - **B. Hire on visible evidence, not hidden skill.** Schools read a coach's teaching with noise (a better-connected staff gets a sharper read). On top they add what they can see: how his players at that position grew and moved up under him, and (with section 3) his unit's stats relative to his level. Each coach gets a "came up as" specialty (DB coach, OC, and so on).
    - **C. Every rise has a cause a player could copy.** The head coach's recruiting skill wins recruits through relationships. Facilities move in steps (a booster gift, a budget decision). Drift toward the middle is smaller, and starting values begin balanced. Every big riser or faller in the trial gets a "why" line.
