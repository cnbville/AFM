# Fix list for the next build

Cain's answers to world trial 5 (`docs/trials/world-trial5.md`), 2026-10-15. Nothing here is built. Cain will ask for the list to be executed; that is when we go into extreme detail on how, topic by topic. Until then, each item says what Cain wants (his words), what the game does today (traced in the code), and the questions to settle in the detail round (proposed, not confirmed).

Also waiting, not part of this list yet: the 10-item yes/no list from the dynasty re-trace (`docs/trials/world-trial4-proposals.md`, last section). When Cain answers it, the agreed items join this list.

## 1. Kickers, punters and roster needs (before the next trial run)
Cain: "That should alreaedy be in there, if it's not, add it an fix list to do before  the next trial run."

Today (traced): it is not in. High school rosters fill positions by need (`athlete.rs`, `place_class`), but college recruiting ranks recruits by rating only, with 8 points off for specialists (`recruiting.rs`), no position needs and no walk-ons. In the last season of trial 5, 91-103 of 138 FBS teams had neither a kicker nor a punter. With no kicker the engine's field goal estimate is 0.2 (`flow.rs`, `fg_make_estimate`), so teams try 0.5 field goals a game (real 1.56), go for two 1.8 times a game, punt on 4th-and-goal, and whoever is listed first kicks off and punts.

To settle in the detail round:
- Position needs for every program and level: how many at each spot (real FBS rosters carry about two kickers, two punters and a long snapper), and how a need changes the value of a recruit or a transfer.
- Specialists judged against specialists (by leg, accuracy and the mental side, per the special teams decision of 2026-10-13), not against the whole class.
- The long snapper: not a position in the game today.
- Check after: no FBS team without a kicker and a punter; field goal tries, two-point tries and punt distance against real.

## 2. Pipelines and the home state
Cain: "Pipelines should already be in the game, if it's not. Add it to the fix list."

Today (traced): pipelines are in. They grow only from impact players (all-conference, awards, day 1-2 draft picks), to a high school and to its area, and staff carry personal ties to those schools (`docs/world-model.md`, trial 1 list item 4). But rosters still come out national: LSU 1-6% from Louisiana, Alabama 4-8%, Ohio State 3-14%, median FBS roster 7-8% home-state. A recruit counts closeness only by miles on a 1,500-mile scale (`recruiting.rs`, `utility`), and there is no pull toward his home state or its flagship.

To settle in the detail round:
- Why the built pipelines don't hold a state: how strong they are, how fast they grow, and whether a program starts 1996 with none.
- A home-state pull: family, growing up a fan, the state flagship, local media; how big for each recruit (influences are noteworthy, rarely driving).
- Staffs recruiting their own state first.
- A real check: home-state shares of real FBS rosters (CFBD rosters). Claude's memory says flagships keep about 40-70% in-state; to verify before setting a target.

## 3. Position changes are real, and nobody switches groups for a game
Cain: "Agreed, but more extreme. You don't switch from QB to MLB or MLB to QB temporarily. If you get moved, your position changes majoryl and it'd damn near impossible for you to get back. The only exceptions are Tight Ends playing Fullback too, Cornerbacks/Wide receiver. Logic is that MLB is an excruciatingly difficult position just like QB. No person has the bandwidth to switch temporarily. BUUUUUUT within position groups it can be changed. A RT can play LT if the starting LT is injured but make sure their overall adjusts accordingly."

Today (traced):
- The game-day depth chart lists a player at any spot on his side of the ball when he rates 4 points better there (`roster.rs`, `CROSS_LIST`), so a guard can start at quarterback for a season while still listed as a guard (3-4% of FBS seasons with 100+ passes in trial 5; one threw 74 touchdowns).
- Emergency cover fills an empty spot with anyone on that side of the ball.
- Two-way play: a player starts on the other side when he beats its starters by a margin per level (0 at small high schools, large in the NFL).
- Between seasons a staff moves a player to a clearly better spot (`athlete.rs`, `reconsider`); kickers are never moved in or out.
- The engine has 13 positions. There is no LT/RT, guard/center, slot corner, fullback, MLB/OLB or free/strong safety split, so "within the group" swaps can't be told apart from position changes yet. The ~20 real depth-chart positions are decided (`docs/decisions.md`) but were never drafted for sign-off.

To settle in the detail round:
- Position groups and which moves are within a group (LT/RT, guard/center, and so on), with the overall at the new spot.
- The two exceptions that may play both: tight end and fullback, corner and receiver.
- A real move: the listed position changes, his development follows the new spot, and moving back is close to impossible.
- What happens to cross-listing, emergency cover and two-way play at each level (high school two-way play is real).
- Depends on the ~20 positions.

## 4. Volatile development
Cain: "Agreed, make it more volatile, when giving me the debugs always mention what players have suffered those volatile slumps so i can judge it."

Today: fewer than 0.3% of player seasons lose more than a rating point, at any age (trial 5, dashboard Player paths). The yearly step's slumps slow growth but rarely take ability away (inferred).

To settle in the detail round:
- Which real causes take ability away (a major injury, weight gain, lost drive, a bad coaching fit, off-field trouble) and how big each can be.
- Standing rule for every trial and debug view from now on: name the players who suffered big slumps, with how much they lost and why, so Cain can judge each one.

## 5. Walk-ons
Cain: "Agreed."

Today: there are no walk-ons. Unrated players are 7-8% of draft picks (real 25%, inflated by walk-ons, JUCO players and name mismatches in the real match). Ties into item 1 (specialists, preferred walk-ons) and item 2 (walk-ons are mostly local).

To settle in the detail round:
- Preferred walk-ons and open tryouts, how many per program and level, who they are, and how one earns a scholarship.
