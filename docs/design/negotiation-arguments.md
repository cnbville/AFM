# Negotiation that responds to arguments

Agreed by Cain on 2026-11-17 (thread "Check how negotiation responds to arguments", then "go" and "both"): items 1, 2, 3a, 3b, 3c, 4, 5, 6 and 7, college first. Every size below is Claude's, **proposed, not confirmed**. Built in lanes NG1 to NG6 (the table at the end), each its own PR from the thread "Make negotiation respond to arguments".

His question: "if I make good points do I actually lessen their wants? And do I use their trust and love for me to take less? Like an NFL QB taking less so he can get better supporting staff?" And: "you threw 20 INTs, earn my trust back before market rate", for college NIL too, not just the NFL.

## What happens today (on main 2ff4e5f)

- **NFL.** Your words never reach the agent's number. His price is the market's score (`front_office::nfl::judge` on `pro::market::score`). Good lines only raise the agent's trust in your word, which only moves favors.
- **Two trusts.** A good one-on-one raises his trust in you (`persona::stores::nudge_trust`, `Person.trust`). The contract score reads his trust in the club (`ProCareer.trust`, `market.rs` `score`, `loyal_q`), which starts full and only drops.
- **One flat floor.** `score` caps any offer at 0.84 when `money_q < 0.82` and his money goal is over 0.3. So most players re-sign at about 78% of their ideal plus state tax, whether you went 13-4 or 6-11. A man whose money goal is 0.3 or less has no cap and can sign at 40% or lower: a cliff and an exploit.
- **Winning, loyalty and a bond barely count.** They are parts of a ten-part average that all sit near 1.0. The want the agent names moves his opening ask by at most 6% (`WANT_ASK`). Nothing values the cap room a discount leaves.
- **A stat line counts a little.** `market::consensus` adds 0.3 x his production over his rating (at most 6 points either way): a bad year costs at most about 2 grade points.
- **College, your own player.** His number is the recruit price (`deals.rs`, `money_bar`, `Who::Own`: `usual_nil_offer`, which reads stars, or last year's pay). Neither reads his season. A player who isn't thinking of leaving says yes to any amount (`Deals::own`), even a tenth of his market, and only gets upset on July 1.
- **College, the portal.** A player's market price reads `last_performance`: his rating through the season plus development's drawn form (`college.rs`, `development.rs`). His engine games don't enter it, so a season of interceptions doesn't lower what other schools would pay.
- **The money facts check.** `claims::check::money` reads every fact as unknown, so a stat you state in a money talk counts for nothing.
- **One loyalty discount never runs.** `deals::HOME_DISCOUNT` is in `Deals::pro`, but your contract offers go to `judge`, not to it.

Root cause: the contract score averages parts that sit near 1.0, so money hardly moves it, and the only real floor is a flat money cap. Loyalty and a bond can't earn a discount because they don't set that floor, talks feed a trust the score doesn't read, and in college the price reads neither his games nor your words.

## The three numbers

Every money talk (an NFL contract, your college player's NIL and revenue share, a recruit's NIL) works on three numbers. Each is an engine number, never shown.

- **His market (M).** What the market would really pay him. NFL: his ideal from `market::ideal`, on `consensus`. College: his market price (the best rival's top bid, capped at your own top bid for his role; `nil_bid::Market::bid` from the lane "Pay inherited players their market price"). A recruit: the money that beats his best other school's appeal (`Deals::recruit`), never past the best rival's top bid.
- **His ask (A).** Where his side opens: his market moved up by greed and ego (agents open high), your first number (anchors), the want his agent names (`want_factor`). Today's asks stay as built.
- **His lowest number (F).** The least he will take from you: his market less his own discount (item 1). F = M x (1 - d).

Your offer is weighed against his **bar** (B). He signs or agrees at B or above. B starts at his ask and comes down two ways:

- **True points** (item 3a) move B from A toward M, never below M: B = M + (A - M) x (1 - pull).
- **What he feels about you** (item 1) lowers M itself to F for you alone: B = F + (A - F) x (1 - pull) once his discount applies.

So arguments alone can talk him down to his real market and no further. Only the relationship takes him below it, as real hometown discounts do.

## Item 6: the college market reads his season and tape

Agreed: "In college, a player's portal value reads his season and tape, so a bad year really lowers what other schools would pay. A lowball raises his portal lean the day you offer it."

- **His season read.** The portal market reads his season as the NFL's consensus reads a pro: `season_read = last_performance + TAPE_W x clamp(tape, -TAPE_CAP, TAPE_CAP)`. `tape` is what his engine games showed over his ratings this season, from the results ledger (`results::Ledger`, each read's `shown`, weighted by its trials and shrunk by credibility as growth shrinks it, `Fit::weight`). Proposed, not confirmed: `TAPE_W` 0.3 and `TAPE_CAP` 6, the NFL's own sizes, so one bad year costs at most 1.8 points of play over what his ratings already lost, about 5% of his price at `BID_PLAY` 3% a point.
- **Where it lands.** One function (`nil_bid::season_read`) read everywhere the market reads `last_performance` for a price: the December keep offers and the market price (`nil_bid::Market::bid`), the portal's usual money (`recruiting::portal_usual`, `portal_nil`). The stars, the draft and development keep reading `last_performance`.
- **No engine games.** A season on the stand-in has no ledger: the read is `last_performance`, as today.
- **AI schools.** Every school prices every player this way. This changes the world: portal prices and December keep offers.
- **The lowball's lean** is in item 5.

## Item 5: a lowball upsets a staying player the day you offer it

Agreed: "College NIL: a lowball upsets a staying player the day you offer it, not months later."

- **He has his number.** A player who isn't thinking of leaving no longer takes any amount. His ask is his market price (the inherited lane's `market_price`, or last July's pay when higher), moved by greed and ego as a pro's is (`ASK_GREED`, `ASK_EGO`). His bar is B above, and his lowest number F is item 1's.
- **His answer.** An offer at B or above is a yes. Between B and half his lowest number: a counter at B, in his family's or his agent's words. Under half of F (`OWN_LOWBALL` 0.5, proposed): a lowball.
- **The day you offer it.** A lowball, or any offer under F, upsets him at once: his trust in you falls by `deals::cost` (0.02 to 0.06, his ego and temper), his mood falls by a mood reason (pay), and his portal lean rises by `LOWBALL_LEAN` x (1 - offer / F) x his money care (`LOWBALL_LEAN` 0.15, proposed: an offer of half his number at full money care adds 0.075, about a third of the way to a leaver's 0.25).
- **No second hit in July.** A deal he agreed to in a talk is never a July 1 upset (the NIL desk's reaction reads the agreed deal as his own yes). A lowball he refused still counts in July if you never paid him.
- **A player already leaning out** keeps today's rule (`Deals::own`'s lean cut), with his money read on his number instead of the recruit price.

## Item 1: each player's lowest number

Agreed: "Each player gets his own lowest number, built from your bond with him, trust, whether you're contending, home ties and whether he's already made his money. Most get 0-8% off, and a rich, loyal winner up to about 25%. That's like real hometown discounts."

Real hometown discounts mostly run 5 to 15% under market (Tom Brady's later deals and Patrick Mahomes's 2023 restructure were the rare quarter to half); they come from players who already earned their big money, love the place and expect to win.

- **His discount** d for a club or school (proposed, not confirmed). His own club or school gets every part; a new one only its winning and his home (the ring chaser and the hometown man):
  - Bond: 0.06 x his closeness to you (`folk::bonds::close`, from the bonds build, PR #542).
  - Trust: 0.03 x his trust in you over half (`(trust - 0.5) / 0.5`, at least 0), one trust (item 2).
  - Contending: 0.04 x how much your team wins, read as his want reads it (`want_fit`'s Winning: +1 at a .800 season, 0 at .500, nothing below; college: your last season and your final rank) x his winning want over half.
  - Home: 0.03 when you play in his home state (NFL: the club's state; college: his home state's school or within 250 miles), x his home want.
  - Sum, times his money factor: 1.6 - 1.2 x his money goal (0.4 for a man who lives for money, 1.54 for one who barely cares), times 1 + 0.6 x made, where made is 1 once he has signed a second contract or earned three times the league's minimum over his career (NFL), or earned a seven-figure NIL year (college), else 0.
  - At most 0.25 (`FLOOR_MAX`).
- **Examples.** An average starter (no bond, trust 0.5, a .500 club, away from home, money 0.5) gets 0%. A loyal starter who trusts you (trust 0.8) on a 12-5 club: about 4%. A veteran past his second deal, close to you (0.8), trust 0.9, a contender, home state, money 0.3: 25%, the cap.
- **No cliff.** The flat 0.84 cap on `money_q < 0.82` in `market::score` is replaced by his own floor: an offer under his lowest number for that club scores at most 0.84, and one at it or above is scored as today. A man who barely cares about money gets a bigger discount, never a free one.
- **AI clubs.** Their own players get the same discount from the same parts (their head coach's bond and trust where the world keeps them, their winning, his home). This changes the world: re-signings across the league. Before and after are measured on the test world, then on a world if Cain says yes (see "Measuring").
- **College.** The same parts and the same cap price your own player's lowest number (item 5). AI schools' December keep offers use it too.

## Item 2: one trust

Agreed: "One trust: what a player thinks of you counts in his contract."

- **For your own pro**, the contract score's loyalty part reads one trust: the average of his trust in the club (`ProCareer.trust`: trade rumors, broken promises) and his trust in you (`persona::makeup::trust`, which your talks and kept promises move).
- **For an AI club's own player**, his trust in the club, as today (the world keeps no trust in an AI head coach for pros yet).
- **Item 1's trust part** reads the same one trust.

## Item 3a: true points talk him down to his market, never below

Agreed: "A true point that his stats back up can talk him down to what the market would really pay, never below it. His agent or family can argue back from the play log ('eight were tipped'). A cherry-picked point gets caught and costs you trust."

- **What a point is.** A fact you state in a money talk (the ear's `stat`, `injury`, `status`, `off_field`, `interest` or `record`) about his season or his place. The money facts check is filled (`claims::check::money` and a new `check::money::season`): his stat lines from his season (a pro's `ProSeason.stats`; your college player's lines in your games kept whole, `w.seat_games`), his grade against his level's standard at his spot, his games missed hurt, a case on his record, and other clubs' or schools' real interest (the market's pending offers; the portal's offers).
- **Its strength.** A true point counts by how far the fact sits from the standard of a player paid his market (his stat or grade against the median starter at his spot and level, in standard deviations, at most 1), times how much of it is on him (item 3a's push-back below). Praise, a fact that isn't about money, or one already in his price counts nothing (`Truth::Priced`).
- **What it moves.** All true points in a talk add up to `pull` (at most 1): B = M + (A - M) x (1 - pull). Sized so one strong true point (one standard deviation, all on him) takes half the way from his ask to his market (`POINT_PULL` 0.5, proposed), two take it all.
- **Never below his market.** A point never lowers M. A bad season already lowered M through his read (item 6 in college, `consensus` at the NFL). Saying it again only gets you to the number the market already set.
- **His side argues back.** His agent (NFL), his agent or his family's decider (college) reads the same season for context: on interceptions, how many came under pressure or on a receiver's drop; on sacks allowed, how often the line beside him lost; on missed games, whether he played hurt. The share of the stat on him is what's left (at least 0.3). How hard they argue back: the agent's skill, the family's read (a decider who follows football closely). Their line names the reason ("eight of them came with a man in his face"), in the voice.
- **A cherry-picked point.** A true stat whose whole season says otherwise (his grade at or above his market's standard though the stat is bad) is caught by his side with chance 0.3 + 0.5 x skill (proposed). Caught: the point counts nothing, and the agent's or decider's trust in your word falls 0.05 (`truth::Live`). Not caught: it counts at half.
- **A false point** is caught on the spot when his side can check it (a stat line, a missed game) and costs what a false fact already costs (`talks::typed_against`): nothing gained, his trust in you down.
- **Where it runs.** Your seat only. The AI's own talks keep today's market.

## Item 3b: "earn it back" ends as a prove-it deal

Agreed: "'Earn it back' usually ends as a prove-it deal: shorter, a smaller guarantee, bonuses on the stat you named. That's how Mayfield, Geno Smith and Darnold signed." (Baker Mayfield, Tampa Bay, one year in 2023; Geno Smith, Seattle, one year in 2022; Sam Darnold, Minnesota, one year in 2024.)

- **When.** A true performance point has landed (pull 0.3 or more from a performance fact) and his bar still sits above your offer.
- **The counter's shape.** His side counters with a prove-it deal instead of only a lower number: NFL one year (two for a man under 27), the guarantee at most half his ideal's share, and incentives on the stat you named worth the gap between his ask and your offer, on lane N2's season terms (`terms::read`, `Contract::add_incentive`; met last season makes it likely to be earned and it counts on the cap now, as built). College: a one-year NIL deal (`nil_bid::NilDeal` with one year), a smaller base, and the gap as a clause on the stat (`terms::Clause`, `On::Nil`).
- **His makeup picks it.** A man who wants security (security goal over 0.6) asks for the guarantee back and takes fewer incentives; a confident man (ego high) wants the incentives.
- **You can still pay him.** An offer at his bar is a yes on any shape.

## Item 3c: his makeup decides how it lands

Agreed: "His makeup decides how it lands. A proud player loses trust and leans toward free agency or the portal. A professional one takes it."

- **Proud** (ego z over 0.5, proposed): each performance point that lands costs his trust in you 0.02 x (1 + ego z), more when the push-back found the blame wasn't his (x1.5 when less than half of it was on him). His lean to leave rises: NFL, his bar to re-sign before the market rises by 0.03 x ego z (he wants to see what others think, `own_bar`'s test); college, his portal lean rises by 0.03 x ego z.
- **Professional** (professionalism z over 0.5): no trust cost when the point is true and the blame was his; a small trust gain (0.01) when you were straight with him, as the one-on-ones' "be straight" already reads it.
- **Everyone else**: half the proud man's cost.
- **Sizes are on lane N1's** trust and patience scales and inside lane N4's caps.

## Item 4: "take less and I'll sign you help"

Agreed: "A new promise, 'take less and I'll sign you help.' Keep it and his trust rises. Break it and his next deal costs more."

- **The promise.** A new give and promise kind, `Help(pos)`: you will add a starter at a position (or "help", any position) before next season's opener. It counts only for a man whose winning want is over half.
- **What it's worth.** Believed as P1 believes a promise (`promise::belief_own`), it adds to his discount: 0.05 x belief x his winning want over half (proposed), inside item 1's cap.
- **Kept.** By the opener, your club signed, traded for or drafted in the first two rounds a man at that position who starts the opener. Kept: his trust in you +0.08 and in the club +0.05.
- **Broken.** His trust in you and the club falls as a broken promise does (`market.rs`, 0.3), and his next deal with you costs more: his ask carries a premium of 5% x his winning want (`ProCareer.help_owed`, cleared when he signs again), proposed.
- **NFL only** for now (no college ask came with it).

## Item 7: recruits

Agreed: "Recruits: points come from his tape and your depth chart. They only lower his money if his other offers see him the same way."

- **What a point is.** His film (his last high school season's read, `film_raw`) against the stars he's priced at, and where he'd stand on your depth chart (the men ahead of him at his spot within two years, `recruit_top`'s count).
- **What it lowers.** His money ask is capped today at the best top bid of the schools that want him, yours included. A true point takes your own bid out of that cap: his ask can't sit above what his other offers bid, because their read of him is the market's. When his best other offer already bids your number, the point lowers nothing.
- **A false point** about his place is heard as a promise (as built); about his film it is checked against his film read now.
- **How he takes it.** Most recruits take it badly: his trust in you falls 0.02 to 0.04 by ego, and your case loses a playing time point. A humble, coachable recruit (ego z under -0.3 and coachability over 12) takes it straight: no trust cost, and his family believes your other promises more ("be straight", as built).

## Measuring

- Each lane's tests: a player with a bad season priced lower in the portal, a staying player's lowball upsetting him on the day, a true point moving the counter and stopping at his market, a cherry-picked point caught, a prove-it counter, a proud and a professional man, the help promise kept and broken, a recruit's point capped at his other offers.
- **A world check** (lanes NG1 and NG4 change the world): re-signings before free agency, re-sign prices against market, portal prices by season, against main on one seed. It needs Cain's yes with the cost stated first.
- **NG4 measured** (2026-11-18, Cain's yes on "Measure first"; seed 1's shipped world played one season with `play_on`, stand-in games): re-signed before free agency main 91, NG4 76; unrestricted free agents 304 and 317; players rated 78+ unsigned at cutdown 71 and 85; median dead money $37.6M and $34.6M; trades 39 and 39. The first build gave 69 re-signs and 100 unsigned: in free agency his price stopped 2% above the best offer and his market read his ideal, so a man who barely cared about money could never take the best offer. His market now takes out the room the score already allows and is capped at the best offer once he is on the market (`MARKET_SLACK`, `Ideal::best`).
- **NG1 measured** (2026-11-18, the ignored `negotiation_college::season_read_measure`): seed 1's shipped world played on the engine to January 5, 2027 (`play_on` with `engine`, 330 days, about 18 minutes), then read. 47,659 of 70,095 college players with a season read carry tape. The tape moves the read very little: p5 -0.13 points, p50 0, p95 +0.12; 10 players lose more than half a point and 6 gain more. At `BID_PLAY` (3% a point) nine in ten prices move under half a percent. The ledger's shrink for credibility (`Fit::weight`) keeps one season's tape small, so a season of interceptions still barely lowers what other schools would pay. The shipped world file itself has no ledger for 2025, so before its first season the read is `last_performance` alone.
- **NG4's re-sign fix** (2026-11-18, a proxy call for Cain while he slept: give clubs a real edge with their own players before free agency): before the market his agent reads what the other clubs would really pay (`market::agent_read`: the best `make_offer` from any other club, less the sure thing `SURE_THING` 0.05 for a deal now over the market's risks; the minimum when nobody else would bid), and his market at his own club is never above it. Proposed, not confirmed. Same seed: re-signed 83 (main 91), unrestricted free agents 311 (304), rated 78+ unsigned at cutdown 80 (71), median dead money $37.1M ($37.6M), trades 41 (39). The re-signs still short of main are players another club values more by over 5%; they go to the market.

## The lanes

| Lane | What | Items | Changes the world | Waits for |
|---|---|---|---|---|
| NG1 | The college market reads his season and tape | 6 | Yes (portal prices, keep offers) | The inherited-pay lane's `nil_bid::Market` |
| NG2 | Your college player's number, lowest number and points | 5, 1 (college), 3a-c (college) | No (your seat) | NG1 |
| NG3 | Recruits' points | 7 | No | NG2 |
| NG4 | The NFL's lowest number and one trust | 2, 1 | Yes (re-signings) | Bonds PR #542 for the bond part (0 until then) |
| NG5 | NFL points, prove-it deals, makeup | 3a-c | No | NG4 |
| NG6 | "Take less and I'll sign you help" | 4 | No | NG5 |

Each lane updates `docs/reference/how-it-works/live-talks.md` (and `recruiting.md`, `nfl.md` where it touches them) in the same PR.
