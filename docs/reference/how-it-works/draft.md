# The NFL draft: how it works today

Checked against main at dfb8b02 with update 2 lane 8 on 2026-10-26.

## Quick answers

- Who picks whom? Each club takes the player with the best value on its own board: board grade + position value + need - a top-of-draft damp (`world/recruiting.rs`, `draft_value`, `draft_step`).
- Where does a club's board grade come from? Its own file read (scouts, GM, coaches, combine) blended with the public grade by how sure it is; a GM who trusts himself leans on the public less (`world/pro/scouts.rs`, `read`, `board_from`).
- What is the "public grade"? The December read, aligned across positions, plus a 1-point keyed draw. It orders the pool (`world/recruiting.rs`, `draft_reads`, `draft_begin`).
- How much is a quarterback worth over an average position? About 12 board points at grade 85, if the club needs one; an edge rusher graded 90 about 5 (the `BOARD_PER_CAP` comment says 7) (`world/recruiting.rs`, `draft_position_value`, `BOARD_PER_CAP`).
- Does need matter? Yes: need = clamp((78 - weakest starter)/2.5, -1.5, 3) + 1 if he is 31+, halved in rounds 1-2, × the GM's own style 0.5-1.5 (`career.rs`, `need`; `recruiting.rs`, `draft_value`).
- Does a team with a young franchise QB draft another early? Rarely: need is -8 when its best QB is 82+ and 30 or younger, and the QB position value is gated by need (`career.rs`, `need`; `draft_value`, `qb_gate`).
- Does the combine change a prospect's grade? Not by a flat shift. It adds shared measurements to every club's file, so a club that barely saw him learns a lot (`world/pro/scouts.rs`, `combine`; `world/recruiting.rs`, `combine`).
- Do freak athletes go early? Yes: +2.0 grade points per full Dynamic Link once he has tested (1.0 on film) (`world/pro/scouts.rs`, `read`, `DRAFT_GIFT`).
- Do small-school players get marked down? Yes: FCS -1, D2/JUCO -5, NAIA -6, D3 -7 grade points, and film reads 1.2× or 1.5× noisier (`world/pro/scouts.rs`, `competition_discount`, `read`).
- Do red flags cost picks? -4 per suspension, -8 per dismissal in every grade; character worry costs up to a few more; very bad reads go on the do-not-draft list (`career.rs`, `PIPELINE.red_flag_discount`; `scouts.rs`, `board_from`).
- Who declares early? From the December round read: rounds 1-2 85%, 3-4 35% + family money, 5-7 8%, undrafted 0.05%; less with NIL income, more with ambition; the mid-December advisory grade redraws it (`world/recruiting.rs`, `declare_chance`, `advisory_grades`).
- Are there draft-day trades? Yes, priced on each GM's chart (classic Jimmy Johnson or a flatter surplus chart), with a later pick this year or next year's (`world/pro/scouts.rs`, `trade_on_clock`; `world/pro/trades.rs`, `chart`).
- How does the undrafted rush work? Each club calls the best left on its board (grade 50+) with a bonus pool; each player picks the best path to the 53 (`world/pro/scouts.rs`, `rush`).
- What does a first pick earn? From 2011, four years at about 5.1% of the cap a year (1.5% at the end of round one, 0.4% in round seven); before 2011 up to twice that at the top, with longer deals (`world/pro/money.rs`, `rookie_terms`, `rookie_scale_share`).
- Are there compensatory picks? From 1994: up to 32 a year at the end of rounds 3-7 for net free agents lost, at most four per club, topped up to 32 with supplemental 7ths; diversity picks from 2021 (`world/pro/trades.rs`, `award_comp_picks`).
- Can an owner overrule the draft room? In round one, with chance 0.15 × his `draft_override`; he takes the first 5-star, or in-state 4-star, in the top 40 left (`world/recruiting.rs`, `draft_step`).
- Can a prospect refuse a club? Rarely: a top QB (score 85+, ego z > 1, a parent or handler deciding) with the first pick held by a weak team, 12%; that club puts him on its do-not-draft list (`world/scouting/predraft.rs`, `choose`, `on_boards`).

## The draft order and the slots

- Order: last season's teams sorted by the playoff round they went out in, then record, worst first (`world/nfl_year.rs`, `order_from`, `draft_order`).
- Rounds: 12 through 1992, 8 in 1993, 7 from 1994 (`world/franchise.rs`, `draft_rounds`). Expansion teams' places in 1995, 1999 and 2002 by the real rules (`franchise.rs`, `round_order`).
- Comp slots per round from 1994: rounds 3-7 get 6, 4, 6, 8, 9 (scaled by league size). Until free agency awards them they go to keyed teams (`franchise.rs`, `comp_picks`, `draft_board`). Once awarded they sit at the end of their round, the best value first, diversity picks last (`world/pro/trades.rs`, `board_owners`).
- Traded picks go to their owners; a pick forfeited in a supplemental draft is removed (`trades.rs`, `board_owners`).
- A world smaller than full size keeps every k-th pick (`franchise.rs`, `draft_board`).
- Pick times: round 1 Thursday at 10 minutes a pick, rounds 2-3 Friday at 7, rounds 4-7 Saturday at 5 (`recruiting.rs`, `draft_end`, `pick_time`).

## The December read and declaring

`world/recruiting.rs`, `draft_reads` (after the regular season):

- Who: college players 3+ years out of high school, not at a junior college.
- Pro position: pro scouts project him at his best NFL position; he moves there on draft morning (`athlete::better_position`; `draft_begin`).
- Read: `scout_player` at accuracy 13 (8 for a small-school player whose stats footprint flagged him) on his projected self (`scouting.rs`, `scout_player`). The code comment calls the flagged read "truer", but a lower accuracy in `scout_player` gives a wider error.
- Grade = overall + (0.25 + 0.2 × youth) × (potential - overall) - competition discount - red flags; youth = clamp((23 - age)/3, 0, 1).
- Score (the public's ranking number) = grade + position value × (1.0 if grade 78+, 0.6 if 72+, else 0.3) - 2 for kickers and punters, 5 for long snappers (`specialist_draft_discount`).
- Round = where his score ranks against the draft's slots; beyond the slots he is "round 8", projected undrafted (`career.rs`, `draft_round`).
- Each underclassman draws a first plan from `declare_chance` (`recruiting.rs`).
- `declare_chance`: rounds 1-2 0.85; 3-4 0.35 + 0.02 × (family money want × 10); 5-7 0.08; else 0.0005. In the NIL era × clamp(1 - 0.6 × NIL income / a year of his round's rookie pay, 0.25, 1). × (1 + 0.25 × ambition z). Capped 0.97. Rookie pay used: $4M round 1, $1.6M round 2, $1.2M rounds 3-4, $1M later.
- December 15, the advisory committee (`advisory_grades`; date in `world/clock.rs`; update 2 lane 8, SK21): who asks is every underclassman projected in rounds 1-7 or planning to go. From 1994 the committee answers; from 2014 five names a school (an AI staff sends its five best scores, more for a projected first-rounder; your program sends the names on your list, `own::request_advisory`) with three answers (1, 2, 3 = return to school), before 2014 a round 1-8. Advisory = round(December round + 1.5 × a keyed draw). A junior not sent decides on round(December round + 2.4 × a keyed draw) with no grade. The plan is redrawn from `declare_chance` × your stay-or-go talk's factor (`own::talk_factor`) for your own players.
- Announcements: each on his own day between his last game (or the advisory date) and the deadline, sooner with competitiveness; seniors who have used their eligibility enter at the deadline; playoff teams' players after their last game (`declarations`, `Declare::Soft`, `Deadline`).
- Withdrawals: a declared underclassman with advisory round 5+ withdraws 30% of the time, unless he opted out of his bowl (`declarations`, `Declare::Withdraw`).
- The COVID year: a senior with only that year left decides at the deadline (`eligibility::covid_return_chance`).

## All-star games, combine and pro days

- All-star invites by December score, in order Senior Bowl (110), Shrine (110), HBCU Legacy (90, from 2022, HBCU conferences only), NFLPA (100, from 2012); at most two of each specialist. Skips: the top 15 of the pool in round 1 60%, the rest of round 1 35%, round 2 10%, others 2% (`recruiting.rs`, `all_star_invites`).
- All-star week: every club's area scout (and a national at the Senior Bowl) files a look, and the public grade and score move 0.3 × (an accuracy-16 read - grade), capped ±2 (`recruiting.rs`, `all_star_game`; `world/pro/scouts.rs`, `all_star`).
- Combine invites: every declared prospect projected in rounds 1-7 plus up to 60 (× world scale) of the rest by score (`recruiting.rs`, `combine_invites`).
- Combine: shared measurements into every file (none in a year its workouts were cancelled; none for a prospect who chose to skip them), the clubs' interviews (60, 45 from 2020), news for anyone testing 1.8 sd above his position's invitees, and one "combine moment" for the 40 furthest under what his weight predicts (4.25 + 0.0042 × (weight - 180)), if 0.12 s or more (`recruiting.rs`, `combine`; `scouts.rs`, `combine`; `world/scouting/predraft.rs`, `combine_moments`).
- Pro days: March to early April, FBS and FCS programs only; untested prospects measured (hand times, 0.05 s fast); clubs grading him 66+ send a coach (`world/nfl_year.rs`, `new_season`; `scouts.rs`, `pro_day`; `world/scouting/file.rs`, `tests`). Each is counted (SK20): clubs whose best read there is 60+ came, 74+ sent a head man; kept three years in `Scouting::pro_days` and read by recruiting ([scouting.md](scouting.md), "The fresh look").
- Pre-draft training: every declared prospect's speed, acceleration and agility measurements read up to +2.0 points for a hard worker with money (2 × clamp((work ethic - 6)/12) × (0.5 + 0.5 × money)), less the staff's test-reading discount (`world/scouting/file.rs`, `read`, `training`). The "testing camp" choice flag is shown in words only; nothing reads it.

## The prospect's own choices

`world/scouting/predraft.rs`, `choose` (January 20) and the spring:

- Skip the combine workouts (test at the pro day): round 1, 0.25 + 0.1 × ego z (positive ego only).
- Turn down the Senior Bowl: rounds 1-2, 0.2 + 0.1 × ego z (positive ego only).
- Testing camp: 0.35 + 0.15 × work ethic z + 0.3 × money. Words only.
- A round-1 QB throws for only some clubs: 40%. Words only.
- Turn down a club's top-30 visit: round 1 and score 80+, 0.08 + 0.05 × ego z (positive ego only) per club; that club's file gets no visit (`declines`).
- Leaks: Wednesdays in March-April, each club 35% a week; 0.15 + 0.2 × GM overconfidence of them are smokescreens (`leaks`). On draft morning a club picking within 10 places ahead of a leaking club adds up to 1.5 × (1 - GM skill/20, max 0.8) per leak (0.9 × for a fake), capped 2 points (`leak_pull`, `on_boards`).
- Your war room can plant a leak (`plant`); other clubs read it the same way.

## A club's board

`world/pro/scouts.rs`, `read` and `board_from` (draft morning, `world/recruiting.rs`, `draft_begin`):

- Its read of each prospect it filed: grade (as on [scouting.md](scouting.md)), its variance, its character risk.
- Aligned onto the draft scale with every position (`level_rating.rs`, `PositionAligner::for_draft`). Specialists climb a ladder against the class instead, capped at the grade found at 18% of the draft (kickers), 27% (punters), 70% (long snappers).
- Blend: weight on its own read 1/max(var, 0.25); on the public grade (1.5 - GM overconfidence)/16.
- Character: minus 1.0 × max(risk - 0.5, 0). Do-not-draft if risk ≥ 2.5, or a dismissal with risk ≥ 1.0.
- Hard minimums (SK17): do-not-draft under one of the GM's own lines (`own::ai_minimums`, keyed by the GM; about a third hold one), or at your club under one of yours set to drop.
- The read itself carries the shared tape tilt, the analytics look, a member club's service list in the fall, area contacts and your word as a college coach (`own::word_lift`) ([scouting.md](scouting.md), "The fresh look").
- A prospect it never filed: the public grade + 3 × a keyed draw.
- What it does not read: the GM's position fixation (only the market reads it, `world/pro/market.rs`), the GM's film lean, or the Power's fit weight.

## Picking

`world/recruiting.rs`, `draft_value` and `draft_step`. Each pick looks at the top 300 left by the public order.

- Round factor rf: rounds 1-2 1.0, 3-4 0.6, 5-7 0.3.
- Position value = 140 × (market value at his position - at an average position), market value = 0.004 + (top-five share - 0.004) × clamp((grade - 66)/24, 0, 1)^1.6, on draft-day top-five shares: QB 0.234, EDGE 0.148, OT 0.135, CB 0.131, DT 0.112, WR 0.110, TE 0.090, RB 0.088, LB 0.080, S and IOL 0.075, K 0.023, P 0.015, LS 0.007; average 0.111 (`draft_top5`, `market_value`, `VETERAN_MEAN`).
- QB gate: position value × max(clamp((need + 2)/4, 0.2, 1), 1 - (pick - 1)/8), so at the first picks any team takes the best QB.
- Damp: a great player at a position other than QB, EDGE or OT loses 0.35 × (grade - 85) × (1 - (pick - 1)/10) near the top.
- Need: `career::need` × 0.5 in rounds 1-2 × the GM's style (0.5 + a keyed uniform, fixed for the team).
- Specialists only with need above 0, and never two at one specialist spot in a draft.
- Value = grade + position value + need - damp; the highest wins. The pick's reason names need (need ≥ 1), premium position (value ≥ 2, with the rookie-deal saving), a trade up, or best player left.
- A pick joins the club's depth chart at once as a background member, so the next pick's need sees him (`draft_step`).

## Trades on the clock

`world/pro/scouts.rs`, `trade_on_clock` (AI to AI; never your club unless you act):

- Seller willing if 3+ of its top 4 values are within 1.5 of its best, or 2+ with a surplus-chart GM, or it is rebuilding.
- A call happens 55% of the time in round 1, 40% in rounds 2-3, 20% later.
- Buyer: a club in the next 12 slots whose best value beats what it expects at its own slot; eagerness = gap × (0.6 + 0.5 × impatience + 0.4 × risk) ≥ 3. The most eager wins.
- Price on the seller's chart: the slot swap plus a later pick of the buyer's worth 90% of the gap, else next year's cheapest covering round (or one dearer if that one is gone or promised). A swap with no extra goes through only if the gap is ≤5% of the seller's slot value.
- Charts (`world/pro/trades.rs`, `CLASSIC`, `SURPLUS`): classic pick 1 3,000, pick 32 590, pick 64 270, pick 224 2; surplus pick 1 3,000, pick 32 1,400, pick 64 980, pick 224 200. A new GM uses the classic chart with chance 0.85 - 0.5 × era (era 0 in 1995 to 1 in 2025) (`world/pro/office.rs`, `new_gm`).

## The live draft (your war room)

`world/scouting/live.rs`:

- Opens on draft day when your seat is at an NFL club that holds a pick; AI picks run to your turn (`start`, `advance`).
- Pick clock (minutes): 15 in round 1 and 10 after until 2007; then 10, 7, 5 for rounds 3-6, 4 in round 7; round 1 is 8 from 2026 (`clock_minutes`). If time runs out the next club in the round jumps in and you pick after it (`timeout`).
- Your staff's pick: the top of your board order (your moves first, then your director's grades) that the club would take and isn't on your do-not-draft list (`staff_choice`; `world/scouting/board.rs`, `ordered`).
- Offers while you're on the clock: clubs in the next 12 whose eagerness is 2.5+, each calling with chance 0.55 + 0.1 × min(eagerness, 3), so 80-85% (the code comment says about half), at most three, priced on their chart (`offers`).
- Trading up: the club above asks 1.05 × the chart gap, free if the gap is ≤3% of its slot (`quote`, `trade_up`).
- Tiers on your board by rank per team: top-ten grades, round 1, late 1-2, round 2, 2-3, round 3, rounds 4-5, rounds 6-7 (`board.rs`, `tier_of_rank`). "Out on a limb" = 12 places or 35% of your rank from the public (`board.rs`, `limb`).

## The undrafted rush

`world/pro/scouts.rs`, `rush`, called from `world/recruiting.rs`, `draft_end`:

- Room per club = the offseason roster limit - roster - this year's picks.
- Bonus pool per club: $75,000 × (cap that year / 2011 cap) (`udfa_pool`).
- Each club calls its best players left with grade 50+ not on its do-not-draft list, room + 4 at a time; the pool is split by (n - k)²; its top three also get 50%, 30% and 15% of the minimum guaranteed. Clubs keep calling down their boards until full or out of names.
- Each player, best first, takes the offer with the highest path + 0.6 × cash + 0.8 if a coach knew him + 0.3 if home + luck. Path = the position's roster slots - teammates rated at least his self-view (overall + 3 × ego z). Cash in units of 0.04% of the cap. Luck sd = 1.6 - agent skill/20.
- Your club calls only your priority free agent list, once, with its terms cut to your pool; with no list your scouts pick the best left, room clamped 4-20 (`live.rs`, `rush`, `default_pfa`).
- Nobody signed him: rookie minicamp tryouts at up to 12 per club (board grade 46+); each club signs one with chance 0.08 + 0.01 × clamp(overall - 50, 0, 10). The rest leave football (`world/scouting/pro.rs`, `minicamp_tryouts`).
- A drafted or undrafted player credits his college's pipelines: round 1 1.0, rounds 2-3 0.8, later 0.5, undrafted 0.25 (`recruiting.rs`, `draft_end`, `credit_impact`).

## Rookie contracts

`world/pro/money.rs`:

- From 2011: four years, a year's value = `rookie_scale_share` of the cap: pick 1 5.13%, pick 32 1.50%, pick 33 1.20%, pick 64 0.58%, falling to 0.40% (`rookie_terms`, `rookie_scale_share`).
- Before 2011: round 1 × (1 + (1 - (pick - 1)/31)), so pick 1 about twice the scale; years 6 for picks 1-16, 5 rest of round 1, 4 in rounds 2-3, 3 later (4 at most before 1994).
- Bonus share and guarantee: round 1 picks 1-20 65% bonus + 35% base guarantee; rest of round 1 55% + 20%; round 2 35% + 10%; round 3 20%; later 8%. Offsets in round 1. Never below the minimum.
- Undrafted: three years at the minimum plus the rush bonus, with any guaranteed base on top (`undrafted_offer`).
- Fifth-year option on round-one picks, decided from 2014: taken when the club's worth of him is at least 95% of the option price; your club decides its own (`world/pro/market.rs`, `fifth_year_options`).
- A meddling owner (0.6+) with no pet makes his first-round QB his pet (`world/pro/mod.rs`, `join_drafted`).

## Compensatory picks

`world/pro/trades.rs`, `award_comp_picks` (on the comp-pick date each spring, from 1994):

- Each qualifying free agent from last year (still with his new team at season's end) is valued: his pay's rank among all contracts + snap share (if 25%+) + 20 for All-Pro or 5 for a Pro Bowl.
- Round by percentile of that value: top 5% round 3, 10% round 4, 15% round 5, 25% round 6, 35% round 7; below, nothing.
- Each gain cancels a loss of the same round (else a loss in a later round, else the latest loss in an earlier round). A club gets a pick for each loss left, at most four; the league keeps the best 32; short of 32, supplemental 7ths go in draft order.
- Diversity picks (from 2021 drafts): a minority assistant hired away as a head coach (from 2020) earns his old club a pick in each of the next two drafts: round 3 if he was there more than two offseasons, else round 3, 4 or 5 by the club's place in the draft order (first third, middle, last) (`award_comp_picks`, `note_diversity_hire`).

## The supplemental draft

`world/scouting/supp.rs` (through 2019, the second Wednesday of July):

- Eligible: college players three years out who lost eligibility this year (`eligible`).
- Each club bids from overall + 4 × a draw × (1.4 - 0.04 × GM skill): round 2 at 74+, 3 at 70+, 5 at 66+, 7 at 63+. The earliest round wins; ties by a lottery weighted to worse teams. The winner forfeits that round next year and signs him on an undrafted deal (`hold`).

## After the draft

- What it took to go in rounds 1-2 and 3-4 is kept as overall bars for staffs to read their underclassmen against (`recruiting.rs`, `draft_end`, `w.draft_bars`).
- A public top-32 player who slides 40+ picks gets a note (`scouts.rs`, `note_slide`).
- Mocks are scored, your class kept for the look-back, files cleared and scouting departments turned over (`world/scouting/mod.rs`, `after_draft`; `scouts.rs`, `after_rush`).

## Where the knobs are

- `world/recruiting.rs`: `draft_top5`, `veteran_top5`, `VETERAN_MEAN`, `VETERAN_MIN`, `BOARD_PER_CAP` 140, `market_value` (66/24/1.6), `specialist_draft_discount`, `competition_discount`, round factors in `draft_value` and `draft_reads`, `declare_chance`, the advisory 1.5 sd, the 30% withdrawal, all-star sizes and skip odds, the combine extra 60, the owner-override 0.15.
- `career.rs`: `need`, `PIPELINE.draft_picks` 257, `PIPELINE.udfa_ratio`, `PIPELINE.red_flag_discount` (4, 8), `draft_round`.
- `world/pro/scouts.rs`: `DRAFT_GIFT`, the board blend (0.25 floor, /16, 1.5 - overconfidence), do-not-draft lines 2.5 and 1.0, `trade_on_clock` thresholds (1.5 tier, 3.0 eager, 12 slots, call odds), `udfa_pool`, the rush's weights.
- `world/pro/trades.rs`: `CLASSIC`, `SURPLUS`, `award_comp_picks` percentiles. `world/franchise.rs`: `draft_rounds`, `comp_picks`.
- `world/pro/money.rs`: `rookie_terms`, `rookie_scale_share`, `undrafted_offer`.
- `world/scouting/predraft.rs`: choice odds, `declines`, leak odds, `leak_pull` 1.5 and cap 2.
- `world/scouting/live.rs`: `clock_minutes`, offer eagerness 2.5, the 1.05 trade-up ask.
- `world/scouting/pro.rs`: minicamp 12 per club, grade 46, sign odds.
- `level_rating.rs`: `PositionAligner::for_draft` specialist caps.

## Related pages

- [scouting.md](scouting.md): the files and reads the boards are built from.
- [nfl.md](nfl.md): contracts, free agency and trades outside the draft.
- [recruiting.md](recruiting.md): the college pipelines a pick credits.
- [development.md](development.md): potential and youth that the grade projects.
- [people.md](people.md): ego, ambition and family that drive declaring and the rush.
