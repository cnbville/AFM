# Fix list for the section 6 build

Cain's answers to the open items of the trial 8 fixes, 2026-10-19 (thread "Full build and trial 7"). Cain: "btw don't tune them immediately, add them to a fix list i'll include with the building of section 6". Nothing here is built. Each item says what Cain said, what the game does today (traced in the code where it says so, numbers from `match_check` and trial 8's raw export at `afm/trials-8/`), and what the fix changes. The details in each fix are Claude's proposals, not confirmed.

Not on this list, by Cain's answer the same day: NFL passing on world-like rosters (7.9 yards an attempt, real 7.05; "keep it like this for now") and FBS scoring (24.5 points a team game, real 27.0; "Keep it for now, we'l tune later").

## S6.1 Run fits: combination blocks and linebacker fits
Cain: "yes tune them" (to "Build run fits (double teams, linebacker fits)?").

Today (traced, `docs/match-engine.md`, "Trial 8 fixes"): runs have the wrong shape. On the world's FBS rosters 31% of runs gain nothing (real 18%), 19% gain 1-3 yards (real 37%) and 37% gain 4-9 (real 34%). Linebackers make 18% of first tackles (real 37%), defensive backs 42% (real 30%). Blocks are one on one, with no double team that holds a tackle and then climbs to a linebacker, so a lineman either sheds early (a stuff) or the back is through to the safeties. Moving the block start only trades one for the other. Synthetic NFL teams run 3.39 a carry with 43% stuffs (real 4.50 and 18%).

The fix (proposed, not confirmed):
- Combination blocks: two linemen take a down lineman together, and one climbs to the linebacker once the down lineman is controlled, by the run concept (zone, power, counter) and the front.
- Linebacker fits: each linebacker owns a gap from the call and the front, reads his key, fills it and meets the back near the line; a linebacker who misreads or is late opens the cutback.
- Check after, on the world's FBS and NFL rosters (`AFM_ROSTERS`): gains by bucket, first tackles by group, yards a carry (FBS 5.12, NFL 4.50) and yards after contact.

## S6.2 Plays ending out of bounds
Cain: "yes tune" (to "Fix how few NFL plays end out of bounds?").

Today (`match_check`, 600 NFL games): 1.4% of NFL plays end out of bounds in normal time. After a play that stops the clock, the next snap comes 9.2 s of game time later (real 15.2, nflverse 2021-25), likely because few stops are the kind where the clock starts again once the ball is spotted (inferred). Teams run 64.0 plays a game (real 61.9). Why so few plays reach the sideline is not traced yet.

The fix (proposed, not confirmed):
- Trace where runs, catches and scrambles end against the sideline: cutbacks inside, tackles in bounds, routes toward the sideline, and players getting out on purpose late in a half.
- Read the real share of plays ending out of bounds from nflverse's play by play (its out-of-bounds flag), by play type and time left, and tune toward it.
- Check after: that share, the snap-to-snap times in `match_check` and plays a game.

## S6.3 Bonus money paid twice (the $251M contract)
Cain: "251 m a year seems broken right ?"

Today (traced in the code, numbers from trial 8's raw export):
- An extension adds the old deal's whole signing bonus to the new deal (`market.rs`, `extend`), and a season's pay counts the deal's whole signing bonus in its first year (`pro/mod.rs`, the season pay). So every extension pays out every earlier bonus again.
- A restructure turns salary into signing bonus (`money.rs`, `restructure`), but that bonus is only paid in a deal's first year. So restructured money isn't paid in the year it's restructured; it lands at the next extension, together with the old bonuses.
- The headline average and the guarantee count the old bonuses again. Makai Tarry's deal reads $251M a year and $1.96B guaranteed because of this.
- His pay was 1-2% of the cap in most seasons (the restructured money went unpaid), then 46% of it in 2033, the first year of his extension. From 2026 to 2045, 144 player seasons paid one player more than 30% of the cap; the worst was Hunter Baker (QB, 2038) at 139%. Not all 144 are this bug, since a new deal's bonus can be large too. The same money goes into each team's cash spending.
- The prices themselves look right: Tarry's deals were 20-25% of the cap of the year they started (the real top five quarterbacks average 23.4%).
- Also seen: tags may be priced too high. His transition tag was $117M, 23% of that year's cap, and the quarterback tag has a floor at 85% of the top five's average (19.9% of the cap). The real 2025 quarterback franchise tag was $40.2M, 14.4% of the cap (from memory, to verify).
- Also seen: a new deal can pay a lot up front. His 2037 Miami deal (a free agent signing, not an extension) paid 43% of that year's cap in its first season. Real first-year cash on the top quarterback deals is about a third of the cap or less (from memory, to verify).

The fix (proposed, not confirmed):
- A signing bonus is paid once, in the year it is signed. A restructure's bonus is paid in the year of the restructure.
- An extension's bonus is only its new money. The old deal's unpaid proration still counts on the cap, as it does now.
- The headline average and guarantee of an extension count only the new money (OverTheCap's "new money" average).
- Check the tag prices against the real tags by year, and the first-year cash of new deals against real deals.
- Check after: each season's pay and cap hit against the cap for the top players, and each team's cash a year.

# Earlier: the trial 5 fix list (2026-10-15)

Cain's answers to world trial 5 (`docs/trials/world-trial5.md`), kept as they were written on 2026-10-15. Topics 2-5 were built in build 7 (`docs/design/build7-checklist.md`). Each item says what Cain wanted (his words), what the game did then (traced in the code), and the questions settled in the detail round.

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
