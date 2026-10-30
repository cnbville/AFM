# The living NFL: how it works today

Checked against main at b9acd15 with update 3 lanes 5 and 3 on 2026-10-28, your IR and staff duties (minor bug 73, SC6) at 6200524 on 2026-10-29, and the expansion line with next update lane 10 the same day; agents' own voice with the next update's lane 14 at 7487fc5 on 2026-10-30; update 3 lane 4 (speed in recruiting and on the field) merged with main at b2b4213 on 2026-10-30.

Paths are relative to `crates/afm-engine/src/`. "Proposed, not confirmed" sizes are Claude's; the code says which.

## Quick answers

- Do agents sound different? Yes: an agent's words come from his honesty, his temper and his agency's size, and a bluff comes out vague; the numbers and the market's scoring are unchanged (`world/folk/dialogue.rs`, `agent_says`; `world/front_office/nfl.rs`, `propose`, `agent_claim`).
- How is a free agent's asking price set? His league-wide consensus read is ranked against the league's veterans at his position, read off the real pay ladder, then cut for age past his prime (`world/pro/market.rs`, `ideal`, `market_grade`, `ladder_share`, `market_price`, `decline`).
- Does a team see a player's true rating? No. Each club reads `overall()` plus a keyed misread that shrinks with GM judgment and seasons on film (`world/pro/mod.rs`, `grade`).
- What signs a deal on the spot? An offer scoring 1.0 against his hidden ideal. At 0.85 or more he agrees with a 7-day window; below that his ideal falls (`world/pro/market.rs`, `market_day`, `score`, `decay`).
- Can an offer win on role or city alone? No. If money falls under 0.82 (and money matters to him) or guarantees under 0.75 (and security matters), the score is capped at 0.84 (`world/pro/market.rs`, `score`).
- Does state income tax matter? Yes. The money part uses pay after the state's top rate (`world/pro/market.rs`, `score`, `state_tax`).
- What is a franchise tag's price? The five-year average of the top-5 cap hits as a share of the cap (from 2011), or 120% of his own last cap hit if higher. A third tag costs 144% or the QB price (`world/pro/market.rs`, `tags_and_tenders`, `tag_bases`; `world/pro/money.rs`, `tag_repeat`).
- How many tags can a team use? One a year from 2011 (franchise or transition). Before that, one of each (`world/pro/market.rs`, `tags_and_tenders`).
- Who goes on waivers? Under 4 accrued seasons, always. Veterans only after the trade deadline (`world/pro/market.rs`, `waivable`).
- Waiver order? Last season's draft order until week 3, then worst record first (`world/pro/market.rs`, `waiver_order`).
- When do trades happen? Four offseason windows, every Tuesday of the season, and twice in deadline week. Rebuilding clubs never buy (`world/pro/mod.rs`, `day`; `world/pro/trades.rs`, `trade_window`).
- Does a trade put dead money on the seller? Yes. All remaining proration lands on the seller this year, or split over two years after June 1 (`world/pro/trades.rs`, `kept_bonus`, `execute`).
- When do players retire? At the Super Bowl, by a logistic curve around a position's usual age, moved by longevity, love of the game and wear. Unsigned players can also retire week by week (`world/pro/mod.rs`, `season_end`, `unsigned_week`).
- Does mood make a pro retire? No. `season_end` does not read mood. Mood does feed holdouts (`world/pro/market.rs`, `holdouts`).
- How is the MVP picked? The best stats-and-film read times a position weight (QB 1.0) times a win factor of 0.4 + 1.2 x win share (`world/pro/honors.rs`, `season_honors`).
- Who gets fired as GM? A GM is replaced at Black Monday when the owner's trust is under 0.3 and he has 2+ years in the job. There is also a 3% yearly chance he "moved on" (`world/pro/mod.rs`, `black_monday`).
- Do agents bluff? Yes, when there is only one offer under 0.85. The odds are (1 - honesty) x 0.6. The GM calls the bluff by his skill and his history with the agent (`world/pro/market.rs`, `bluff`).
- Does an NFL team carry over cap room? Yes, from 2011: the room left when the regular season ended (`world/pro/mod.rs`, `set_carryover`, `space`).
- Do players hold grudges against teams? Yes, from some cuts, trades and broken promises. A grudge lowers the ties part of the team's offer score (one of ten weighted parts, weight 0.4) to as low as 0.7, a few percent off the whole score (`world/people.rs`, `grudge`, `ties_q`; `world/pro/market.rs`, `score`). See the People page.

## Rosters and depth

- **Statuses**: Roster, PracticeSquad, Reserve (IR or PUP), FreeAgent, Other(league) (`world/pro/mod.rs`, `Status`).
- **Depth slots** (starters, rotation, roster): QB 1/0/3, RB 1/1/4, WR 3/1/6, TE 1/1/3, OT 2/0/4, IOL 3/0/5, EDGE 2/1/5, DT 2/1/5, LB 2/1/5, CB 2/1/6, S 2/0/4, K/P/LS 1/0/1 (`league.rs`, `depth_slots`).
- **Depth order**: the club's `grade`. Non-specialists lose 0.6 a year past 30. Last year's 8-game starter here, if under 32, gets +3 (QB) or +1.5. Injured or holding out: -50. The owner's pet: +4 (`world/pro/mod.rs`, `order_depth`).
- **Team strength**: the average rating of the starters, long snapper left out (`league.rs`, `Team::strength`).
- **Roster limit**: the offseason limit from the new league year to cutdown, then the regular-season limit (`world/pro/mod.rs`, `roster_limit`; the sizes are in `w.rules`).
- **Game-day actives**: healthy, not suspended, not sitting out by your discipline, and for your own club not on the non-football injury list from report day (`training/seat.rs`, `SeatTraining::hurt_out`; preseason sides the same, `preseason_side`). Starters and rotation first, then backups in a fixed order (QB, IOL, OT, DT, EDGE, LB, CB, S, WR, RB, TE) up to the actives limit. Then practice-squad elevations (`world/pro/games.rs`, `actives`; `world/pro/squad.rs`, `elevate`).
- **Elevations**: 2 a game from 2020. Each player can be elevated 2 times a season in 2020-21 and 3 times from 2022 (`world/pro/squad.rs`, `elevation_limits`). Protected squad players: 4 a week from 2020 (`protected_count`).
- **Practice squad eligibility**: at most 2 seasons played, fewer than 3 seasons on a squad, age 26 or under. Veteran places: 2 (2014-15), 4 (2016-19), 6 with no accrual limit (2020 on) (`world/pro/squad.rs`, `squad_eligible`, `veteran_places`).
- **Cutdown**: each player is ranked on the club's grade, plus a camp term (first year -4 x 0.2, new team -1 x 0.2, competitiveness), plus preseason tape (clamped ±3), minus 3 x (1 - sharpness). Then add rookie protection by round for 3 years (`PIPELINE.rookie_protection`), +3 for last year's starter and +0.3 a year under 27. A 4+ season veteran below starter level pays a cost (up to 3). GM biases also count: sunk cost +2x, impatience -2x, "not my guy" -2x. The owner's pet gets +10, and a player out 6+ weeks -2. Each position keeps its share of the limit, and the best of the rest fill it. Young players within 7 points of the cut line go to the squad (`world/pro/mod.rs`, `cut_scores`, `cut_split`, `cutdown`).
- **Short rosters**: at cutdown and each Tuesday in season, a team below 53 signs the best street player at its thinnest spot. The engine needs 30 players to play a team (`world/pro/mod.rs`, `fill_roster`; `world/pro/games.rs`, `build_side`).
- **IR**: minimum stay 6 weeks, 4 from 2020. Returns per season: 0 to 2011, 1 (2012-16), 2 (2017-19), unlimited (2020-21), 8 from 2022 (`world/pro/games.rs`, `ir_minimum`, `ir_returns`). The reserve list ends at the new league year. A player still hurt moves to PUP and comes back without using a return (`world/pro/mod.rs`, `league_year`, `heal_week`).
- **Start of history**: each club gets generated veterans for its 53 slots plus 45% more as camp bodies. About 5 a team wait on the street, rated 66 ± 4 (`world/pro/mod.rs`, `fill_league`).

## Contracts and the cap

- **Cap**: the real cap from 1994. 2010 is priced as 2009 and has no hard cap. Before 1994 a notional cap of $34.6M x 1.065^(year-1994). After 2026, `future_cap` (`world/pro/money.rs`, `price_cap`, `capped`, `cap_books`).
- **Minimum**: shares of the cap by accrued seasons: 0.301% (0), 0.344% (1), 0.369% (2), 0.394% (3), 0.419% (4-6), 0.45% (7+) (`world/pro/money.rs`, `minimum`). A practice squad week pays the rookie minimum / 17 x 0.28 (`practice_squad_week`).
- **Free agency**: unrestricted at 4 accrued seasons, 6 in 2010 (`world/pro/money.rs`, `ufa_seasons`). A season accrues with 6+ games on full pay (`world/pro/mod.rs`, `season_end`).
- **Contract kinds**: Rookie, Undrafted, Veteran, Minimum, Franchise, Transition, Tender, Future, PracticeSquad, Extension (`world/pro/money.rs`, `Kind`).
- **Dead money**: a release accelerates every later year's proration and owes the guaranteed money left. After June 1, or with a June 1 designation (from 2006, up to 2 a year), this year's proration stays and the rest moves to next year (`world/pro/money.rs`, `dead_if_cut`; `world/pro/market.rs`, `clear_room`). Each charge is kept with its player and cause (`world/pro/mod.rs`, `DeadWhy`, `add_dead`).
- **Top 51**: until the first game, only the 51 biggest cap hits count. Clubs also hold back 5% of the cap for the season (`world/pro/mod.rs`, `payroll`, `space`, `IN_SEASON_RESERVE`). Void-year bonus lands the March after a deal's last season (`payroll`, `market::open_league_year`).
- **Uncapped 2010**: a team's room is last year's cap x (1 + 0.02 x (owner rating - 10)) (`world/pro/mod.rs`, `space`).
- **Deal layout**: for big deals (over 3x the 4-year minimum), the signing bonus is min(0.55 x guarantee, 0.35) of the money and the rest of the guarantee is guaranteed base. Deals of 3+ years add an 8% injury guarantee, a 6% ramp and a 5% roster bonus. There is a 1% workout bonus, a 4% per-game bonus at age 30+ (from 2006), and 10% incentives on a big 1-year deal. A GM with early_extensions over 0.6 adds 2 void years (from 2016). An elite QB (consensus 88+) gets a no-trade clause 30% of the time (`world/pro/market.rs`, `terms_for`).
- **Front-loading**: if more than 10% of the cap would still be free, the bonus becomes guaranteed first-year salary and pay falls 10% a year (`world/pro/market.rs`, `terms_for`). No deal pays more than 41% of the cap in year one (`world/pro/money.rs`, `FIRST_YEAR_MAX`).
- **Restructures**: base above the minimum becomes bonus spread over up to 5 years. Void years only fill out to 5 (`world/pro/money.rs`, `restructure`). Before the league year, no later cap hit can pass 26% of that year's cap (`CAP_HIT_MAX`). A restructure is also skipped if it would push next year's books over next year's cap (`world/pro/market.rs`, `clear_room`).
- **Cap moves** (`world/pro/market.rs`, `cap_moves`, `clear_room`): these run at the tag window, the day before the league year and at kickoff. Before the league year the target is 7.5% of the cap free, 10.5% for a contender. (That is about 4.5% for the draft class, plus 3% for the market, or 6% for a contender.) Restructures come first, biggest core base salaries first. A GM with the restructure habit over 0.7 does at least 2. Then cap casualties: the most cap saved for the least surplus. Before the league year these are only players whose surplus is under 20% of their hit, unless the club is over. On opening day any saving counts. A deal signed this league year is never cut before the league year opens.
- **Rookie scale** (from 2011): 4 years. The yearly value as a share of the cap falls in a line from 5.13% at pick 1 to 1.5% at pick 32, 1.2% to 0.58% across round 2, then down to a 0.4% floor. Before 2011 the top of round one costs up to 2x the scale, with 6-year deals for picks 1-16, 5 for the rest of round one, 4 for rounds 2-3 and 3 after. Bonus/guarantee shares by round: picks 1-20 0.65/0.35, rest of R1 0.55/0.2, R2 0.35/0.1, R3 0.2/0, later 0.08/0 (`world/pro/money.rs`, `rookie_terms`, `rookie_scale_share`). Undrafted: 3 years at the minimum plus a bonus (`undrafted_terms`, `undrafted_offer`).
- **Fifth-year option** (decisions from 2014, round 1 only): from 2021 the price is 50% / 72% / 85% of the next year's top-5 average for 0 / 1 / 2+ Pro Bowls. Before that, 72% for top-10 picks and 50% for the rest. The option is picked up if the club's walk-away price is at least 95% of it. It is fully guaranteed from 2021, injury-only before (`world/pro/market.rs`, `fifth_year_options`, `option_price`, `set_option`).
- **Cash floor**: from 2016, every 4 years, a club pays out 90% of the four years' caps in cash or pays the shortfall (`world/pro/market.rs`, `cash_floor`).
- **Incentives**: settled at Black Monday as a cap credit or charge for next year. Missed per-game bonuses come back. An unlikely incentive is earned with 12+ starts or a Pro Bowl (`world/pro/market.rs`, `settle_incentives`).

## How players are valued (prices)

- **Consensus read** (what the league thinks): overall + 0.3 x clamp(½ efficiency grade + ½ totals rank - overall, ±6) + 0.5 x min(honors, 4). Honors count over the last 3 seasons: Pro Bowl 1, All-Pro +2 (`world/pro/market.rs`, `consensus`, `production`, `honors`). The efficiency grade is the engine's per-play grade and needs 150+ graded snaps. The totals rank runs 90 at the top of the position, 66 at the median and about 45 at the bottom (`rank_totals`).
- **Ladder**: each February the veterans' consensus reads are listed by position. A veteran here means 4+ accrued seasons or a veteran-type deal (`world/pro/market.rs`, `set_ladders`). His rank over the league's starting places gives a share of the top-5 pay. General positions: 1.0 at the top 6%, 0.58 at 25%, 0.36 at 50% (16th of 32), 0.11 at 100%, 0 at 200%. QBs: 1.0 at 10%, 0.9 at 37.5%, 0.6 at 50%, then a cliff (`ladder_share`). Below every veteran he is ranked among everyone at the position, young players included (`market_grade`).
- **Price**: the cap x `market_share` (the minimum at grade 66 rising to the top-5 share at 90, with a 1.6 power curve), capped at 1.08 x the top 5. Then x `decline` (`world/pro/market.rs`, `market_price`; `world/pro/money.rs`, `market_share`, `top5_share`, `record_apy`).
- **Top-5 shares of the cap**: QB 0.20, EDGE 0.158, WR 0.136, DT 0.117, CB 0.108, OT 0.0985, S 0.081, IOL 0.077, LB 0.075, RB 0.074, TE 0.063, K 0.023, P 0.015, LS 0.007 (`world/pro/money.rs`, `top5_share`).
- **Age**: the prime ends at RB 27, OT/IOL 30, QB/K/P/LS 33, the rest 29. Past it the price falls 4% a year, plus 5% a year more after 3 years past, with a floor of 40% (`world/pro/market.rs`, `decline`, `prime_end`). (`docs/world-model.md` says "4% a year to 60%"; the code wins.)
- **A team's own price** (`world/pro/market.rs`, `worth`). View = consensus + (own grade - consensus) x (0.4 + 0.6 x overconfidence), moved toward film or stats by the GM's film lean and recency. Then the GM's biases: name value, "not my guy", loyalty, sunk cost, risk on players 25 or under. Scheme fit adds 2 x the power structure's fit weight. Past his prime: -1 a year. Price changes:
  - Need: x(1 + 0.05 x need) if he would start.
  - Capped at 2x the minimum if he would be below the rotation.
  - Mode: x1.08 for a contender (if he starts); a rebuilder pays x0.8 at 28+ and x1.05 at 25 or under.
  - A GM who won't pay backs caps RBs at 3.5% of the cap.
  - A pet position: x1.1.
  - After a bad year (fans under -0.3) an owner adds a splash for an 82+ player (+10% x his PR trait).
- **What it does not read**: looks or Aura. Contracts read football only (`world/fame.rs`, module notes).

## Free agency and the market

- **Dates**: the tag window opens (ladders set, cap moves, early re-sign talks). Then the tag deadline (last talks at a lower bar, tags and tenders). The tampering window opens from 2013, then the new league year. The market runs daily for 14 days after the league year, then on Tuesdays and Fridays until camp (`world/pro/mod.rs`, `day`).
- **His goals** (hidden, fixed): money, security, winning, role, city, home, loyalty, greed, temper, each 0.05-1. They come from his id, love of the game, competitiveness, family money and his loyalty/greed/temper traits. 0.3% negotiate alone (`world/pro/market.rs`, `goals`).
- **His ideal**: the market price x (1 + 0.2 x (money - 0.5)) x (1 + 0.004 x (agent skill - 10)), never past the record deal for his age. The guarantee share is 0.55 / 0.40 / 0.25 / 0.05 by grade 85 / 78 / 72 / below, plus 0.2 x (security - 0.5), capped at 0.8. Years are the usual length + 1.5 x (security - 0.5). He wants to start if role > 0.45 and grade ≥ 70 (`world/pro/market.rs`, `ideal`; `world/pro/money.rs`, `usual_years`).
- **Offer score weights**: money x1.6, security, years 0.4, winning x0.7, role x0.7, city x0.4, home x0.4, loyalty x0.6, ties 0.4 and the NFLPA report card 0.35. The parts:
  - win_q = 0.75 + (strength - 80)/12, clamped 0.4-1.15.
  - city_q = 0.85 + 0.3 x city pull (warm state, nightlife).
  - home 1.1.
  - loyalty with his old team 0.9 + 0.25 x trust.
  - (`world/pro/market.rs`, `score`, `city_pull`; `world/finance/nfl.rs`, `card_pull`, 0.94-1.06)
- **Offers**: a club offers only if its walk-away price is at least 72% of his ideal. It opens at 88% of the ideal and climbs 1.5% a day to 103%. A needy club (need ≥ 2.5, or a contender with need ≥ 1) goes straight to its number. It needs 75% of the money free in cap space. The guarantee is capped at 0.25 + 0.5 x the owner's cash will (`world/pro/market.rs`, `make_offer`).
- **Inside the 7-day window** a clearly better offer (+0.03) sways him. Unsigned, his ideal falls toward the best offer (x0.97, but not below 1.02 x that offer) or 1% a market day with none. His guarantee wish falls 2%, and after 6 weeks he drops the starting-job wish (`world/pro/market.rs`, `market_day`, `decay`).
- **Bluffs**: believed, the club moves up to its number (+8% at most). Called (odds 0.3 + 0.03 x (GM skill - 10), moved by the agent relationship), the offer is pulled (25%) or cut 6%. The agent loses 0.2 relationship with that club. The player fires him with odds 0.15 + 0.3 x temper (`world/pro/market.rs`, `bluff`).
- **Agents**: 180 x world scale (at least 40). 30 giants, 50 mid-size, the rest one-man shops. Skill is about 9 + 2.5 x size, honesty about 0.7. Round 1 picks go to giants 75% of the time (`world/pro/market.rs`, `staff_agents`, `sign_agent`).
- **Re-signing**: a club talks early only with players at market grade 68+. He re-signs at a score of 0.85, or 0.82 at the deadline, plus 0.12 x money x (1 - loyalty) for a money-minded player who wants to test the market. Before 1993 a team keeps anyone it wants (`world/pro/market.rs`, `resign_talks`).
- **Extensions**: offered in the last year, or with a year left for an early-extension GM (> 0.55) or a rookie after 3 seasons (2011 on). Market grade 76+ only. The ask is cut 4% per year left on the deal. It adds at least 2 years (age 32 or under). It signs at 0.87, or 0.82 near a deadline (`world/pro/market.rs`, `extension_talks`, `extend`).
- **Street signings** (players under consensus 66 before camp, everyone after): price = market share x decline x (1 - 4% a week unsigned). The club where he would gain the most playing time wins. Grade 58+ only (`world/pro/mod.rs`, `ask`, `street_signings`).
- **Reserve/future deals**: at Black Monday for non-playoff teams and at the Super Bowl for playoff teams. A club keeps squad players graded 58+ who are 28 or younger, and adds up to 4 street players (27 or under, grade 60+) (`world/pro/market.rs`, `futures`).
- **Holdouts**: only a star (market grade 85+) paid under half his market, with 3+ seasons. Push = greed x min(under/3, 1) x leverage (1, or 0.6 at 30+) - 0.3 x loyalty x trust + 0.1 x temper - 0.15 x mood. At push ≥ 0.3 he holds in. He holds out if push > 0.4, or > 0.55 on a rookie deal from 2020. Each week he gives in with odds 0.08 + 0.05 x weeks, +0.15 in season. A holdout at a rebuilding club, or with trust under 0.5, requests a trade (`world/pro/market.rs`, `holdouts`). A holdout misses camp: +0.3 on that year's development dip (see [development.md](development.md)) (`world/pro/mod.rs`, `camp`).
- **Voluntary weeks** (offseason phases 1-2 and OTAs): each pro comes with chance clamp(0.9 + 0.015 x (work ethic - 10) - 0.04 x max(age - 29, 0), 0.45, 0.99), drawn each week; one who skips works at his own effort and reads Unprepared for prep that week. In OTA weeks the three best absent starters at 82+ overall make small news (`training/week.rs`, `attends`, `monday`, `absent_news`; see [training.md](training.md)).
- **Promises**: a starting job promised in a deal is checked in week 1. If broken: his trust -0.3 and the agent's relationship with the club -0.1 (`world/pro/market.rs`, `check_promises`).

## Tags and tenders

- **Tags** (from 1993): best expiring players first. Franchise tag if market grade ≥ 80 and 1.5 x walk-away ≥ the price. Transition tag if ≥ 78, 1.2 x walk-away ≥ the price, and a 15% roll. A club short of room clears it first (`world/pro/market.rs`, `tags_and_tenders`, `clear_room`). The price base floor is 45% of the top-5 share (`tag_bases`).
- **RFA tenders** (3 accrued, grade 66+): First round if grade 80+, Second if 74+, else the original round (Right of First Refusal for the undrafted). Cap shares: First 2.67%, Second 1.91%, Original 1.17%. The tender is at least 110% of his own cap hit (`world/pro/money.rs`, `tender_price`). Exclusive rights (under 3 accrued, grade 62+): a minimum tender.
- **Offer sheets**: the old team gets 5 days to match. It matches if its walk-away price is at least 95% of the sheet and it has room. If not, it receives the tender's pick (`world/pro/market.rs`, `offer_sheets`).
- **Pre-1993**: before 1989 a club keeps any veteran with market grade 62+. In Plan B (1989-92) only its 37 protected players are kept (`world/pro/market.rs`, `open_league_year`).

### An agent's first word (update 3, lane 5)

- In the ten days before the market opens (the tampering window from 2013, else the new league year), an agent at 0.3 warmth or more with you may call (a fifth of weekdays, once a year) about his best client headed for free agency (`world/folk/backchannel.rs`, `c19_day`). Hearing him out before the window is tampering: you learn his ask (his ideal, shaded up by 0.25 × (1 - the agent's honesty)), and it leaks one time in ten; on June 1 the league takes your next fifth-round pick and fines the club $250,000 and the GM $50,000 (`league_rulings`; the Falcons, 2024). Either answer puts your club first: his offer score × 1.02 for your club (`world/pro/market.rs`, `score`).

## Trades and picks

- **Pick charts**: Classic (Jimmy Johnson: 3,000 at pick 1, 590 at 32) or a flatter Surplus chart (1,400 at 32). Future picks are discounted per year by urgency: contend 0.3, middle 0.18, rebuild 0.08 (`world/pro/trades.rs`, `CLASSIC`, `SURPLUS`, `pick_value`). The classic chart is drawn with odds 0.85 - 0.5 x era (`world/pro/office.rs`, `new_gm`).
- **A player in points**: Σ over his deal's years of 10,000 x (the club's yearly worth x 0.88^years once he is 30+ - his cash cost) / cap. A rebuilder counts 0.35 of the surplus for years at 27+. A contender counts 1.15 for this year (`world/pro/trades.rs`, `player_points`).
- **A call**: a non-rebuilding buyer with a hole (need ≥ 1.2) calls. The odds per window: offseason 0.45, season week 0.1, deadline 0.45, x1.5 for a contender. The target must grade 2+ above the buyer's worst starter and be "surplus" to the seller:
  - the seller is rebuilding and he is 26+,
  - he is not a starter there,
  - he asked out, or
  - he is on your block.
  - The ask is the seller's points (x0.85 if he wants out, at least 30) x 1.05-1.35 by past fleecing. The buyer pays if its own points ≥ the ask (`world/pro/trades.rs`, `trade_window`).
- **What blocks or kills a trade**:
  - a no-trade clause,
  - the owner's pet,
  - injury, or a player just moved,
  - a free agent signed this year,
  - the buyer's cap,
  - the seller staying under the cap after the bonus it keeps.
  - Talks fail 35% of the time, and a failed talk can leak as a shopping rumor (30%: trust -0.1 - 0.2 x loyalty). A failed physical voids 2%. The package is the cheapest single pick or pair over 3 drafts (`world/pro/trades.rs`, `trade_window`, `package`, `shopping_rumor`).
- **Comp picks**: OverTheCap's method. Value from APY rank, snap share and honors (All-Pro 20, Pro Bowl 5). The top 5% get a round 3 value, then 10/15/25/35% for rounds 4-7. Losses are cancelled by signings, at most 4 a team and 32 for the league. The rest are supplemental 7ths. Only players whose deal expired count, signed by the Monday after the draft at 1.5x+ the 4-year minimum (`world/pro/trades.rs`, `note_cfa`, `award_comp_picks`). Board slots: `world/franchise.rs`, `comp_picks`.
- **Draft rounds**: 12 to 1992, 8 in 1993, 7 after (`world/franchise.rs`, `draft_rounds`). Draft-day trades and the boards are on the draft page.

## Front offices

- **GM** (`world/pro/office.rs`, `Gm`, `new_gm`): judgment 1-20 (= `draft_skill`). Each bias is 0-1, drawn as normal(mean, 0.22):
  - film_lean 0.35 + 0.3 x era
  - sunk_cost 0.45, impatience 0.35, not_my_guy 0.4, loyalty 0.4, recency 0.45, copycat 0.4, overconfidence 0.45, risk 0.5
  - name_value 0.5 - 0.2 x era
  - a pet position 30% of the time
  - pays_backs with odds 0.8 - 0.35 x era
  - restructures 0.25 + 0.2 x era, early_extensions 0.2 + 0.3 x era.
  - era = (year - 1995)/30, clamped 0-1. A new GM copies the last champion's film lean, by copycat x 0.5 (`replace_gm`).
- **Owner**: meddling, a pet player, cash will, patience, PR, draft override and trust in the GM (starts 0.7). A meddler adopts any signing over 8% of the cap as his pet (`world/pro/office.rs`, `Owner`, `new_owner`; `world/pro/market.rs`, `sign_offer`).
- **Power**: an owner with meddling > 0.8 keeps the final say 70% of the time. Otherwise the coach has it 20% of the time, else the GM. Scheme fit weight: GM 1.0, coach 1.4, owner 0.9 (`world/pro/office.rs`, `draw_power`, `Power::fit_weight`).
- **Fans**: after the season, fans = 0.5 x old + 0.3 x (wins over expectation)/3 + (win% - 0.5) x 1.2 + playoffs (+0.15 / -0.05) + 0.1 per round past the first, clamped ±1. The owner's trust moves by 0.1 x over + 0.08 x fans (`world/pro/office.rs`, `fan_year`).
- **Contend or rebuild** (each new league year), x =
  - (strength - league mean)/1.5
  - +0.6 for a good QB (80+), else -0.3
  - +0.3 x cap space share
  - -0.15 x (starters' average age - 26.5)
  - +0.5 if on the hot seat (owner trust < 0.35)
  - +0.3 x fans x (1 - patience).
  - Contend if x > 0.6. Rebuild if x < -0.6, not on the hot seat and patience > 0.3. Otherwise middle. A reason string is kept (`world/pro/office.rs`, `choose_modes`).
- **Your club**: every duty (Contracts, Tags, FreeAgency, Trades, Cap, Cutdown, Waivers, PracticeSquad, InjuredReserve, and college Board/Visits/Portal/Nil) runs the same code, with `yours` checked first (`world/front_office/mod.rs`, `Duty`, `yours`).
- **A move you make with the duty at your staff stands (SC6)**: it no longer takes the whole duty back. A deal, trade, tag, tender or option you make is done; your staff leaves alone a player you're in talks with (open, standing or agreed) when it re-signs, extends, tags or bids in free agency (`world/front_office/nfl.rs`, `talking`; `world/pro/market.rs`, `resign_talks`, the extensions, the tags, `free_agency`). Your cut plan, protections and waiver claims stand and the staff fills the rest: `seat_cutdown` runs when the plan isn't empty, `protected` returns your list and the staff protects its best for the places left, and your claims come first in `claims` (`world/front_office/roster.rs`; `world/pro/squad.rs`; `world/front_office/mod.rs`, `seat_head_club`). A tag you give yourself uses your club's tag for the staff too.
- **Injured reserve at your club (minor bug 73)**: with the duty yours (the default), a player hurt past the minimum isn't placed by the engine: your medical staff sends a decision (`world/front_office/reserve.rs`, `recommend`) at the three injury sites (`world/pro/games.rs`: preseason, games, practice). You place him (`place`: he sits at least `ir_minimum`) and bring him back once healed (`activate`: a return used unless off PUP, roster room needed); a healed player waits for your word (`healed`, from `heal_week`). With the duty at your staff it runs as every club's. Between the league year and the final cutdown (cutdown morning included, since your moves come before that day's cut) the list ends the season and a player with fewer than four accrued seasons reaches it only through waivers (`place_words`, `before_cut`). Every club's pre-cutdown placement is season-ending (`ProCareer::ir_out`, set in `games.rs` `record_preseason` and in camp in `practice_injuries`; `heal_week` skips him); a release clears it (`release_with`) and so does the next league year.

## Waivers, cuts and releases

- **A release** sends him to the street, books the dead money and sets `former`. If he is waivable he goes on the wire, and his guaranteed money moves to the claiming club. If he clears, his old club owes it (`world/pro/mod.rs`, `release_with`; `world/pro/market.rs`, `claims`).
- **Claims** in waiver order. A club claims if he grades 2+ above its worst healthy player at the spot (or 60+ when it is short), it has room or a player to drop, and it can afford his cap hit (`world/pro/market.rs`, `claims`).
- **Injury settlement**: a player cut while hurt gets his base x min(weeks out, games)/games on this year's cap (`world/pro/squad.rs`, `settle_injury`).
- **Poaching**: another club can sign an unprotected squad player to its 53 (`world/pro/squad.rs`, `poach`).

## Retirement

- **At the Super Bowl** (`world/pro/mod.rs`, `season_end`): typical age = `career::retire_age` (QB 35, RB 28, WR/EDGE/LB/CB/S 30, TE/DT 31, OT/IOL 32, K/P/LS 37) + 0.8 x longevity + 0.2 x (love - 10) - 0.3 x (wear - 6, if positive).
  - Chance = (0.008 + 0.5 / (1 + e^(-(age - typical)/1.3))) x standing, plus 0.08 for each year past typical + 4.
  - Standing: 0.35 if overall ≥ 85, 0.6 if ≥ 78, else 1. It rises toward 1 once he is 2+ years past typical.
  - A star's announced farewell season always ends the career (`stories::careers`).
  - Unsigned free agents are not rolled here.
- **Unsigned, weekly in season**: chance = 0.03 + 0.05 x (1 - call) + 0.01 a year past 28 + 0.02 x rich - 0.004 x (love - 10) - 0.01 x (1 - family money), clamped 0.005-0.4. Sharpness falls 0.02 + 0.002 x (10 - work ethic) a week, floor 0.4 (`world/pro/mod.rs`, `unsigned_week`).
- **What it reads**: age, position, longevity, love of the game, wear, overall, earnings (unsigned only). What it does not read: mood, team, contract money left (for rostered players).
- A retiring player under contract leaves his proration as dead money. His Hall of Fame case is filed (`world/pro/mod.rs`, `retire`).

## Other leagues

- **Signing at the street level** (leagues not run in full that year): on a league's opening day, free agents aged 31 or under with consensus 54-68 sign with odds 0.3 (age 26 or under) or 0.15. They return at season's end with sharpness 1 and spring snaps of 0.11 x months, clamped 0.3-0.8 (`world/pro/leagues.rs`, `day`, `sign_players`).
- **Full leagues** from 1996 (`world/pro/other.rs`, `FULL_FROM`, `full`): the CFL, both Arena leagues, NFL Europe, the XFLs, AAF, UFLs and the 2022 USFL, with real teams by era, games on the engine under each league's rule book, and coaches. Rosters come from the street. Talent bands (consensus): CFL 50-70, arena 42-60, spring leagues 50-68. About 2 in 3 street players are willing (`world/pro/other.rs`, `level`, `fill`). Pay, for example: CFL $70K x era (`pay`).
- **NFL Europe allocations**: each club sends 3 young players (25 or under, ≤ 2 seasons played) ranked just below its top 10 young players. Spring snaps 0.35 (`world/pro/leagues.rs`, `allocate`).
- **International Player Pathway** (2017 on): 4 players a year to 2021, 8 in 2022, 13 in 2023, 16 after. Generated, rated 46-56, put on practice squads (`world/pro/leagues.rs`, `pathway`).

## Honors

- **Candidates**: 8+ games. The read is (1 - film weight) x stats/top-10 average + film weight x film/top-10 average. Film weight: QB/RB/WR/TE 0.2, K/P 0, OL/LS 1.0, defense 0.5 (`world/pro/honors.rs`, `candidates`, `reads`, `film_weight`, `stats_value`).
- **Pro Bowl**: per conference. Slots: QB/RB/OT/EDGE/DT/S 3, WR/CB/LB 4, TE 2, IOL 5, K/P/LS 1. Score = read x (1 + 0.06 x past Pro Bowls, up to 5) x (0.9 + 0.2 x win share) (`world/pro/honors.rs`, `season_honors`, `pro_bowl_slots`).
- **All-Pro**: first and second teams. Slots: QB/RB/TE/K/P 1, WR/IOL 3, LS 0, the rest 2. Score = read x (0.95 + 0.1 x win share) (`all_pro_slots`).
- **Awards**: MVP, OPOY (QB x0.9), DPOY (EDGE/DT x1.1), the two Rookies of the Year, and Comeback (last season ≤ 6 games or 6+ IR weeks). Super Bowl MVP: the winners' best stats_value (non-QB x1.3) + 2 x grade sum (`world/pro/honors.rs`, `season_honors`, `super_bowl_mvp`).
- **Records**: seeded with the real 1983 marks and updated each season (`world/pro/honors.rs`, `REAL_1983`, `season_records`).
- **Hall of Fame**: case = 4 x first-team All-Pro + 1.5 x second-team + Pro Bowls + 5 x MVPs + 1.5 x other awards + 0.7 x rings + 0.4 x seasons as a starter. It is filed only if 12+. Eligible 6-25 years after retiring. Up to 5 a year get in if case + 0.6 x years waited ≥ 20. The vote is the Saturday before the Super Bowl (`world/pro/honors.rs`, `hall_case`, `hall_vote`).
- **Honors feed back** into price (consensus), fame (`world/fame.rs`) and ego (`world/people.rs`, `year_book`: 40% chance of +1 ego after a Pro Bowl or All-Pro season).

## Off-field trouble and suspensions

- About 3% of rostered pros a year before 2014 and 2.2% after, scaled by professionalism, stability, temper, age and team mood. Real kinds by era with suspension rules by year. Rulings come weeks later, the exempt list from 2015, voided guarantees, and fringe players cut after violent cases (`world/people.rs`, `pro_trouble_week`, `rule`). Full detail on the People page.

## Franchises and books

- **Franchises**: 28 teams to 1994, Carolina and Jacksonville in 1995, the Browns' move to Baltimore in 1996 (dark until 1999), Houston in 2002 with 8 divisions. Each keeps its `World::nfl` index (`world/franchise.rs`, `ERAS`, `active`, `round_order`). From 2027 the owners' meeting can vote a move (24 of 32 needed) (`world/nfl_year.rs`, `league_meeting`, `relocate`). From 2027 it can also vote two expansion clubs in, at the real rate; they get indices from 32 on, an expansion draft and the first and last picks of their first draft ([new-teams.md](new-teams.md); `world/new_teams/nfl.rs`, `vote`; every lookup past index 31 reads `World::expansion` through `franchise::*_in`).
- **Stand-in games** (when the engine is off): win odds are a logit of 0.52 x strength gap in sd + 0.24 x staff gap + 0.2 home. Ties 0.3% (`world/nfl_year.rs`, `PER_SD`, `win_prob`).
- **Club finance**: revenue and cost lines, gates, stadiums and NFLPA report cards (`world/finance/nfl.rs`). The cards feed free agents' scores (`card_pull`). `spend_level` sets support-staff budgets (`world/people.rs`, `staff_support`).

### Schedules, numbers and news (update 3, lane 4)

- Christmas Day games: 1999-2025 from the real schedules (1 in 1999, 2 in 2004; minor bug 45), three a year after 2025 (proposed, not confirmed) (`world/nfl_year.rs`, `christmas_games`).
- College holiday games keep the real 2001-2025 Thanksgiving and Black Friday days per pair, each pair keeping its 2025 day from 2026 (minor bug 46; `world/schedule.rs`, `HOLIDAY_GAMES`).
- NFL players may wear 0 from 2023 (minor bug 47; `world/stories/numbers.rs`).
- A flex story pairs each game with the stored game of the same meeting, so a division rival's second game no longer reads as a flex every day (B4, minor bug 57; `world/stories/nfl.rs`, `season_day`).
- The trial reads the world's trades and comp picks year by year against the real counts: 40-102 trades a year in 2002-2025 (86-96 since 2021) and 29-38 comp picks (minor bug 48; `afm-tools` `world_trial.rs`, `real_trades_and_comp`).

### Placing a player at cutdown (update 3, lane 5)

- From ten days before cutdown day to cutdown day you can send a player you'd release to a friend's club (a coach there at 0.2 warmth or more): he and your latest pick in next year's draft for theirs a round higher (round 5 on), else the same round, through the trade code (`world/folk/backchannel.rs`, `place_player`, `swap`; `world/front_office/deals.rs`, `execute`). The friend owes you one.

## Where the knobs are

- `world/pro/money.rs`: `minimum` shares, `top5_share`, `record_apy` (1.08), `market_share` (66-90, power 1.6), `usual_years`, `rookie_scale_share`, `rookie_terms` round shares, `tender_price`, `FIRST_YEAR_MAX` 0.41, `CAP_HIT_MAX` 0.26.
- `world/pro/market.rs`: `ladder_share` tables (GENERAL, QB), `decline` (0.04, 0.05, floor 0.4), `score` weights and caps (0.82/0.75 → 0.84), signing bars (1.0, 0.85, 7 days), `make_offer` climb (0.88 + 0.015/day to 1.03, cutoff 0.72), `decay` (0.97, 0.99), `bluff` (0.6, 0.3, 0.25, 0.94), holdout thresholds (0.3/0.4/0.55), `clear_room` targets (0.075, +0.03).
- `world/pro/mod.rs`: `TOP_51`, `IN_SEASON_RESERVE` 0.05, `order_depth` boosts, `cut_scores` terms, `unsigned_week` and `season_end` retirement formulas.
- `world/pro/trades.rs`: `CLASSIC`, `SURPLUS`, urgency 0.3/0.18/0.08, call odds 0.45/0.1/0.45, talks fail 0.35, physical 0.02, `player_points` 0.88 and 0.35/1.15.
- `world/pro/office.rs`: `new_gm` means, `draw_power` (0.8, 0.7, 0.2), `fan_year`, `choose_modes` (±0.6).
- `world/pro/squad.rs`: elevation limits, `veteran_places`. `world/pro/games.rs`: `ir_minimum`, `ir_returns`.
- `world/pro/honors.rs`: slots, `film_weight`, `stats_value`, MVP weights, Hall thresholds (12, 20, 0.6).
- `league.rs`: `depth_slots`. `career.rs`: `retire_age`, `PIPELINE.rookie_protection`.

## Related pages

- [draft.md](draft.md): draft boards, draft-day trades, the undrafted rush.
- [people.md](people.md): mood, trust, grudges, trouble, fame.
- [staff-and-coaches.md](staff-and-coaches.md): head coach firings at Black Monday and the carousel.
- [injuries.md](injuries.md): practice and game injuries, recovery.
- [development.md](development.md): how mood, the room and support staff feed growth.
- [match-engine.md](match-engine.md): NFL games on the engine.
