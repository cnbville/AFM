# Trial 6 fix list (collected from the 30-player trial review)

Nothing here is built. Raichel is still reading the cards; we walk through this list together when she finishes. Goal: trial 6 is the last trial needed.

## 1. Position-based ageing and retirement (card 1, call 3)
- Retirement and decline should depend on position (running backs fade and retire earlier than quarterbacks, kickers, linemen).
- Keep room for outliers like Derrick Henry, using the existing longevity trait.
- Career length also varies by player: mental makeup (love of the game, durability of mind) lets some play longer and others quit sooner. Playing style matters too: a runner-quarterback who relies on his legs (like Jayden Daniels) can fade or opt out much earlier than a pure pocket passer. (Raichel, card 5 note.)
- Card 1: a running back still rated 88 at 30 and starting at 34 was flagged as unrealistic.

## 2. Scouts misjudging as a small bust cause (card 2)
- Five-star busts still start after signing (injury, work ethic, off-field).
- Early bloomers who stop growing are an allowed bust reason (card 2).
- Scout misjudgment is also a cause, but a small one (Raichel: plenty of hard-working QBs never make the NFL).
- FBS staffs have room for error too, so their offers can be wrong in the same way.

## 3. Dismissal severity (card 3, call 7)
- Today every dismissal that does not end a career drops the player exactly one level (FBS to FCS), whatever his quality or the cause.
- Fix (option A, chosen): give each dismissal a severity. Serious causes drop a level or end the career. Lesser causes (academics, failed drug test, team rules) move him sideways to a lower-profile FBS program. Better players get more offers. Needs lower-profile FBS programs to exist in the sim.

## Also open (not part of the three fixes)
- Film line for stars and FBS offers: card posted, recommended option C (strict). Measured numbers are in the thread and in memory afm-trial5-thread.
- League sim details, first-year adjustment drivers, NIL pull, drafting-team knowledge.

## 4. Fewer early cuts, with a practice squad and free agency (card 6, call 3)
- Lower the odds that a young draftee is cut too early.
- A cut player goes to the practice squad or the free-agent pool, where he can be signed again, instead of his career ending.

## 5. Urgency to declare for the draft (card 7, call 2)
- Each draft-eligible player weighs how urgently he needs to leave, not only his projected round. Urgency rises with:
  - money: small schools pay little NIL, so a pro contract is worth more to him;
  - shelf life: running backs and other short-career positions leave early to get paid while they can;
  - fear of falling: an unranked or fringe prospect may declare in case his stock drops next year;
  - also: age (older players can't wait), injury history (get paid before the next one), family need, and a coaching change or lost starting job at his school.
- Staying still pulls the other way: big NIL money, room to raise his stock, and a bad landing spot.

## 6. Roster decisions from camp and in-season form (card 8, calls 3 and 5)
- A player below the roster mark is usually cut in camp or goes to the practice squad, not kept a full season.
- Preseason and in-season form move players up and down the depth chart and on or off the roster: outperforming on the field matters, not only rating.
- Teams judge imperfectly: a 62-rated player with a strong camp can make the roster of a badly run team (weak GM or coaching).

## 7. All remaining calls and bugs from the 30 cards (Raichel agreed to all, 2026-10-11)
- **Call 1, rawer arrivals and a dynamic first-year adjustment** (cards 1, 9, 11, 16): players arrive at college rawer, linemen most of all; every step up (HS to college, college to pro, transfer up) has a first-year dip that depends on step size, position, polish, football IQ and make-up, physical readiness, fit and luck.
- **Call 2, early entry by projection** (cards 12, 15, 18): players weigh where they project, not just a grade of 70; see fix 5 for urgency.
- **Call 3, pro careers** (cards 10, 11, 14, 17, 20): no career-ending cut for young or roster-level players; free agency and practice squad; safeties and backs decline earlier (fix 1).
- **Call 4, injuries cost games** (cards 22, 26): moderate and major injuries cost games; a major senior-year injury hurts recruiting.
- **Call 5, depth chart slots** (cards 13, 24): one punter and one kicker per team; roles come from real slots.
- **Film line for stars and FBS offers, option C** (cards 9, 10, 21, 23, 24, 28): three-stars from the top 2,700 on film, two-stars from the top 6,500, FBS offers below three stars from the top 4,500. Good-film seniors (cards 10, 23) get offers and stars before weak-film ones.
- **Coachability bug** (card 28): already fixed in the code.

## 8. Raichel's own notes on calls 2, 3, 6 and 7 (first round, 2026-10-11)
- **Call 2, the declare decision also weighs:** a bad final year (stay to rebuild stock); which team would likely draft him (players avoid a bad franchise: Dante Moore reportedly stayed rather than go to the Jets); and NIL money that can make staying worth millions. Fix 5 adds urgency on top.
- **Landing spot matters after the draft:** a bad organisation can stall a top pick (Sam Darnold and the Jets), and a better situation later can revive him. Today pro coaching is a random roll unrelated to the drafting team.
- **Call 3, tracked markets:** released players enter a free-agent pool tracked season to season; teams sign by need, rating, age and cost; players can re-sign with their own team; unsigned players can retire from the pool. The NCAA transfer portal works the same way: enter, get offers or not, some end up without a school. Cards show every move.
- **Call 6, position value with situational reaches:** teams weigh positions by general value (QB, edge, tackle up; running back down) and by their own roster. A team with a great defense missing only a generational CB or OLB reaches for him; a team with a young franchise QB won't take another early.
- **Call 7, sideways and down transfers:** players can transfer across or down a level, not only up (for playing time, a coaching change, or a dismissal; see fix 3).

## 9. Last three items (2026-10-11)
- **15. Veterans start too long.** Staffs expect an ageing veteran to fade and lean towards the younger man; starters past their usual age keep the job only if clearly better.
- **16. Kickers and punters get their own draft board.** Few are drafted, and only on day 3.
- **17. Some top-500 seniors still end up two-stars.** Checked in trial 6.

These are the final fixes before the game itself; later problems get debugged in the game.

**Status: all 17 items built in trial 6 (2026-10-11).** Cards: `docs/trials/trial6-cards.json`, page https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP.
