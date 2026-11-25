# Multi-season realism audit: the seed 321 world (2026-11-24)

Read-only. Nothing was fixed and no world was built. Thread "Audit multi-season realism".

Each root cause is the likely one from the docs and a grep of the code. A patch thread measures it before it changes anything.

## Cain's answer (2026-11-24)

"agreed", to the list as proposed:

- **Yes to items 1 to 22, 24 and 25.** No to item 23 (eras in history: history only).
- **Build order:** FBS rushing (1) and freak defenders (3) first, measured on the world's own rosters, since awards (5) and top receivers (8) move with them. Frozen programs (2) as its own design thread. NIL spread (14) after research on the real split by sport.
- **The future run (25) now:** this world played on 10 seasons with the log, about 4 to 5 runner hours plus 40 minutes for its book.
- Each item is one lane in its own fresh thread. The causes and any sizes stay proposed, not confirmed, until its thread has measured them.

### Decisions taken for Cain while he slept (item 3, 2026-11-25)

Cain, 2026-11-24: "i'm giving you permission to make decisions for me als long as you note them down." Each of these is proposed, not confirmed, and his to change.

1. **The block's bend at 20** (`BLOCK_BEND`), read by the pass rush, the run block and the shed's pull alike. It is one block, so it gets one reading; at 20 the FBS sack ladder sits at real from the 2nd best season a year to the 200th.
2. **The shed's pull back to 1.0** (`SHED_RUNNER`; 0.9 since minor bug 253), to hold yards a carry where item 1 left them (5.36 at FBS, 4.49 on the NFL clubs).
3. **Two men to a tackle at most**, and a shared tackle is an assist for both with no solo, as the NCAA's and the NFL's scorers keep it.
4. **A tackle for loss is a loss of half a yard or more**, since the book keeps whole yards.
5. **Kickoffs covered by the bench**: picked on 0.6 Speed, 0.3 Tackling and 0.1 Block Shedding, a starter 30 points down on both kickoff units.
6. **The two-gap read stops at the typical edge's weight**, the lightest men its fit saw.
7. **An athlete from another sport goes on his card at his sport's spot**, and a move takes whenever a man's card and position disagree. A save's men like him move at their staff's next look.
8. **No rusher was lifted to bring the team totals back.** Sacks a team game fell to 1.78 at FBS (real 2.09) and 1.82 on the NFL clubs (2.40), and NFL points rose 0.6. The missing sacks are linebackers' and backs', the next lane; lifting the line again would have put the stars back over real.
9. **Left for later, in `GAPS.md`**: stuffs still reading any loss, a man far lighter than a tackle at tackle, the third linebacker and the nickel on kick units, and the roster, fit, call and analysts' tables (rebuilt once after the defenders' lanes).
10. **The order after it, in the same thread**: linebackers and backs behind the line, who makes the tackle, the play caller's run lean (the coordinator took his yes on it), then items 6, 7, 8, 16 and 19.
11. **No world run.** Everything was measured with `match_check` and `freak_check` on the seed 321 world file.

### Decisions taken for Cain while he slept (item 3, the pass rush lane, 2026-11-25)

The same permission. Each is proposed, not confirmed, and his to change.

1. **Big on big at the snap.** The linemen take the men on the line in order across the formation and the widest is the tackle's; a spare lineman takes a blitzer through his reach. It is how every staff teaches it, and the old nearest-man pairing left the edge with no blocker on 13.8% of his rushes on the NFL clubs.
2. **Every blitzer creeps through the cadence**: a linebacker to 3 yards off the ball (the double-A too), a nickel or a corner to 2.5, a safety to 3, and a back wider than the end comes round him. Sized on PFR's pressures a blitz (linebackers 0.168, real 0.172; corners 0.188, 0.181; safeties 0.155, 0.159).
3. **No timed run-up on top of the creep.** A 0.25 s run-up gave linebackers 7.4 sacks a 100 rushes (real 4.99) and six-man rushes 23.6% sacked (real 9.0%).
4. **The check-release back checks at the snap**, and a back with a route in the call runs it while the passer throws hot. Checking first moved sacks less than 0.01 a team game; it is how backs are taught.
5. **The hot throw before the set from 0.85 s** (0.2 s later under center), when a man the passer knows is free will be on him first. 1.0 s barely differed.
6. **A pressure on a man's line is any hurry, hit or sack**, in the box score and the Data section, as PFR counts them. The play's own pressure flag and the team's count are unchanged. A rush snap is once a dropback he rushed and a rush win once a dropback; a man who rushed has no coverage snap.
7. **The call mix is unchanged.** Linebackers rush a fifth more than PFR's blitzes and safeties a third as often, but there are no college counts and the mix is each staff's; in `GAPS.md`.
8. **No rusher was lifted to bring sacks back.** The NFL clubs have 1.74 a team game (real 2.41) and the FBS teams 1.93 (2.09). The missing sacks are the four-man rush's and the sack a pressure makes, not the blitzers'.
9. **The NFL clubs' best sack seasons were left over real, not capped.** Big on big leaves a team's second-best tackle single-blocked by a guard on every four-man rush, and the best three seasons a year are 24.7, 21.0 and 19.7 (real 19.4, 16.5, 15.8). Rushers are never capped; the cause belongs to the four-man rush, the next lane.
10. **The order after it, in the same thread**: the four-man rush first (it holds most of the missing sacks and the tackles' single blocks), then who makes the tackle, the play caller's run lean, then items 6, 7, 8, 16 and 19.
11. **No world run.** Everything was measured with `match_check` and `freak_check` on the seed 321 world file.

### Decisions taken for Cain while he slept (the play caller's run lean, 2026-11-25)

The coordinator took Cain's yes on the fix itself ("give the engine each league's own typical men, carried in saves and replays"); the rest was decided under the same permission. Each is proposed, not confirmed, and his to change.

1. **The run lean before the four-man rush.** The pass rush lane's list put the four-man rush next; the coordinator asked for the run lean first.
2. **A league is a college division, one state's high schools, the NFL or one other league.** A conference's men would sit closer still, but conferences play each other every September and recruit from the same pool; a division is the field a staff measures its men against.
3. **The typical man is the man the engine seats at the place at kickoff with everyone dressed**, averaged over the league's teams: the same reading of a team's men the lean makes on each snap, so a team like its league's typical one leans nowhere.
4. **Worked out once a season**, at each league's first game day (college at fall camp, a state the day before its season, the NFL and each other league the day before its first game), not weekly: a staff's sense of its league's men doesn't move with one week's injuries.
5. **Carried with every game team**, so a save, a live game and a replay read the same men. A save holds one table a league, a replay of one of your games its two teams' (about 9 KB each before packing).
6. **A world saved before the fix works out its tables at its next dawn**, so an old save's games read their league's men from the next day on.
7. **The defense's two body reads go with it.** The blitz lean by the named linebackers' and corners' weight and the shell lean by the safeties' hitter read had the same fault and now read the league's men, measured as the level's are (every man on the rosters at those positions, the men listed at each safety job). They move little: the FBS blitz rate fell 1.1 points of dropbacks.
8. **Nothing was retuned and no table was rebuilt.** The roster lean's weights are slopes, now read about the league's middle; the test leagues play the same games.
9. **`AFM_LEAGUE=off`** in `match_check` and `ab_check` reads a world's teams about the level's men, for before and after runs.
10. **The order after it, in the same thread**: the four-man rush, then who makes the tackle, then items 6, 7, 8, 16 and 19.
11. **No world run.** Everything was measured with `match_check` on the seed 321 world file.

### Decisions taken for Cain while he slept (the four-man rush, 2026-11-25)

Under the same permission. Each is proposed, not confirmed, and his to change.

1. **The fix is the blocker's set, not the rushers.** The ends were met at the line and won in front of the passer's eyes; real tackles kick back against a wide rusher. No rusher's rating or move was changed.
2. **The set point is 0.3 of the way from the line to the passer's launch point**, on the line from the rusher to it, at 0.4 of the blocker's top speed. Deeper sets gave the edges more pressure, but the world's long-armed power end ran further past the best real seasons: 22.7 sacks a season at 0.3, 23.6 at 0.35 and 29.7 at 0.4 (19.2 before). At 0.3 the best season a year stays where it was (23.9, main 23.8; real 19.4).
3. **Only against a man coming round the edge** (a rusher with an outside contain lane). A man over the blocker and a blitzer through his gap are mirrored as before.
4. **The spare man passes over a man a teammate is on his way to**, unless that man has beaten his blocker or is the plan's rusher. Without it the spare guard chased the end his tackle was setting for; without the plan's exception the slide to the star broke and the best edge ran to 31 to 39 sacks a season.
5. **The spare man helps where the rusher is winning first** (w over 0.2), then the nearest block, from 3.5 yards (2.5 before); a man doubling anyone but the plan's rusher comes off for a block within 2.5 yards whose rusher is winning by half a point more than his own man. Tackles' sacks fell from 0.83 to 0.64 a team game (real 0.52).
6. **Three ideas were tried and dropped.** The end running an arc to a point beside the passer took the best season a year to 50 to 82. A penalty on an inside move with a blocker beside the rusher (no room inside) did little at 3 points and took sacks to 1.57 a team game at 6. A speed win coming off at full speed changed nothing.
7. **Nothing was lifted to bring sacks back.** Sacks went from 1.72 to 1.76 a team game on the NFL clubs (real 2.41). The line's pressures are near real now; the missing sacks are in what turns a pressure into a sack, in `GAPS.md` for its own lane.
8. **The order after it, in the same thread**: who makes the tackle, then items 6, 7, 8, 16 and 19. The sack a pressure makes stays in `GAPS.md` and isn't taken up ahead of them.
9. **No world run.** Everything was measured with `match_check` on the seed 321 world file.

## What was read

- The world: seed 321, 30 seasons of history (1996-2025), stopped on 1 January 2026, built 2026-11-22 on main `094635f` (after every scheme-fit and engine-gaps lane). Release `world-2026-seed321-jan2026-094635f` on the worlds page.
- Its book (the World Reader's SQLite file, 7.8 GB): 330 checks against real (122 near, 53 off, 25 far off, 107 with no data because the world has no game log, 23 with no real figure), plus my own queries over every program season, every player season in the stat book, every retired pro's year-by-year history, the last two seasons' games, the money books and the recruiting log.
- Real figures: `data/real-world/` (named with each number). Where I had no sourced figure it says "from memory".

## What this world can't answer

- **Anything after 2026.** The world stops on 1 January 2026. So NIL growth under the House settlement, revenue sharing, the 105 roster limit and rating drift over future seasons are not in it. Collectives carry one year of House-era money and a flat trend.
- **Year-by-year player ratings.** The world log was off. Rating drift is read from program strength by season, NFL starters by season and the careers of kept players, not from every player's yearly row.
- **Game-by-game checks before 2024.** Only the last two seasons' games are stored. Earlier seasons are team and player totals.
- **What a run would cost:** playing this world on 10 seasons on the match engine with the world log on is about 4 to 5 hours of one GitHub runner, plus about 40 minutes for its book (my estimate from the rebuild's 17 minutes a season). It needs Cain's yes first. It would answer NIL and collective growth, the House cap, prestige drift and future rating drift.

## What looks right

These need no patch.

- **FBS games.** 28.4 points a team game (real 27.0 to 27.6), completion 59.5% (60.6%), 7.13 yards an attempt (7.29), 68.2 plays (66.6). Points sd 13.5 (14.0), one-score games 33.5% (36.7%), wins by 28+ 19.7% (21.1%). Win shares miss the Pythagorean line by 0.086 (0.080).
- **No rating drift in college.** FBS program strength averages 61.6 in 1996-99 and 61.9 in 2025, with the same spread (sd 4.2 then, 4.0 now). FCS, Division II and III are as flat.
- **No rating drift in the NFL after the warm-up.** NFL starters (8+ starts) average 80.3 to 80.9 overall in every three-year span from 2004 to 2025. The first years of history start lower (77.6 in 1996) and settle by 2004; that is the hidden history warming up.
- **Development curves.** Retired pros with four or more NFL seasons peak at 28.0 (QB), 27.2 (OL), 26.7 (LB), 26.5 (DL), 26.0 (WR, TE, DB) and 24.6 (RB). Backs fall 1.4 to 2.8 points a year from 28, linemen 0.2 to 1.5 from 30. Growth per year is 5.0 at 18, 2.4 at 21, 1.0 at 23. Starters' ages by position sit within a few points of real (CBs 30 or older 14.5%, real 12.5%).
- **The title count.** 12 different FBS champions in 29 seasons, the most by one school 7 (Georgia), 6 in any 15 years (real 5). No champion from outside the power conferences (real 0).
- **Coaches.** FBS head coach tenure median 4 seasons (real 4), 17.1% of FBS programs with a new head coach a year (17.6%), NFL 7.5 changes a year (6.5).
- **NFL money.** The cap matches every year, league revenue $23.2B in 2024 (real $23.0B), club value $5.84B ($5.9B), top QB deal 21.9% of the cap (21.5%), dead money 8.3% (about 7.5%).
- **College money.** SEC and Big Ten revenue $167M a school (about $200M), Group of 5 $48M ($55M), FBS crowds 38,400 (41,000).
- **Recruiting and the portal.** 2,944 FBS signees a year (real 2,913), 275 four-stars (301) and 41 five-stars (46) a class, FBS portal entrants 2,656 in 2021-22 (2,724) with 62.4% finding a school (62.8%), in-state share 38% (37%).

## The ranked list

Impact is for the player's seat: high (he sees it every season), medium (he sees it in the record books or the market), low (history or a small number).

### High

**1. FBS backs gain 0.7 more a carry in the world than real.**
- World: 205.8 rushing yards a team game and 5.79 a carry (real 171.8 and 5.12, cfbfastR 2022-24). Regular backs (100+ carries, 2021-24): 5.7 a carry at the median, 4.6 at the 10th percentile (real 4.9 and 3.5). 90 thousand-yard backs a year (real 45 to 51). The rushing leader averages 2,272 yards (real 1,809) and 1.9 backs a year pass 2,000 (real about one every two years).
- Likely cause: the engine is tuned on the synthetic FBS test league, where a carry gains 4.76 to 4.82 (`docs/match-engine.md`; `game/synthetic.rs`, `typical_starter`). The world's rosters play 1.0 above that. `GAPS.md` already says "The typical starter isn't the world's" for the NFL; the same holds at FBS. `match_check` can play a world's college rosters (`AFM_ROSTERS`, `tools/rosters/export_rosters.py`), but the engine lanes measured on the test league and on a world's NFL clubs.
- Touches: the match engine, the stat book and records, awards, draft stock for backs, play calling's values (`data/call_values.csv` is built on the typical team).
- Status: new at this size. Trial 7's finding 2 (college stat lines) is parked.
- Built 2026-11-24 (thread "Fix FBS rushing yards per carry"; `docs/match-engine.md`, "The even run block"): measured on this world's own FBS teams and schedule (`match_check fbs`, `AFM_AB_WORLD`), 5.71 a carry before. The cause was a run block that read the blocker's body and not the defender's, so a world's athletes-first college rosters tilted to the blockers; with the defender's body counted the same way they gain 5.36, and this world's NFL clubs 4.47 (4.62 before, real 4.50). Thousand-yard backs in seasons of 13 games with nobody hurt: 106 a year before, 85 after. The 0.24 left sits in the play callers' run lean and in receivers' blocks on wide runs (`GAPS.md`), both past this item. The book shows none of it until the next world rebuild.

**2. Program fortunes are frozen for 30 years.**
- World: an FBS program's strength correlates 0.90 with its own strength a year later, 0.79 ten years later and 0.76 twenty-five years later. Win share: 0.60 at one year (real 0.536), 0.37 at ten, 0.30 at twenty-five. Texas went 358-41 (.896) and Georgia 350-56 (.859) over 30 seasons (real best about .83, Ohio State, from memory). Ten programs with 28+ FBS seasons sit at .300 or under (real: one or two, from memory). Michigan State had no winning season in all 30 years, Kansas none in 28, Kansas State none in 25. 1.13 unbeaten FBS teams a year (real 0.6), 69% of champions unbeaten (52%), a 65-game winning streak (real 47).
- Likely cause: the one the dynasty diagnosis named on 2026-10-28 (`afm/dynasty-layers/diagnosis.md`): talent follows the program, not the coach. A recruit's fixed pulls (home, brand, `FIVE_STAR_PULL`, `utility_fit` in `world/recruiting.rs`) outweigh the staff (`FEEL_SD` 0.07 against a coach's 0.06). Prestige is 39% parts that never move (tradition, fans, facilities in `world/program.rs`, `prestige_of`). Titles themselves are spread, so the count hid it.
- Touches: recruiting, the portal, prestige, coach hiring and firing, schedules and odds, the campaign's goals.
- Status: known (trial 7 finding 3 parked, trial 10 finding 1 waiting). The home fade (`HOME_FADE_FLOOR`) and lane 5 were built since; this is the first read of the whole 30 years on the finished engine.

**3. A few defenders a year post impossible seasons.**
- World (FBS, 2010-24): the sack leader averages 29.9 (real leaders 14 to 17; the record is 24). 3.4 players a year reach 20 sacks and one every three years reaches 30. One defensive tackle had 84.5 sacks in 12 games, after 67.5 as a freshman. The tackles-for-loss leader averages 52.7 (record 32). The tackle leader averages 196 (real 150 to 168), and 15.9 players a year pass 150. The 99th percentile is fine for sacks (10, real 10); it is the top handful.
- Likely cause: the best linemen's edge over ordinary FBS blockers turns into wins too steeply at the very top (the top 1% get 5.2 sacks per 100 rushes and the top three 9.1; the best real seasons are about 3.5 to 4, my estimate from 17 sacks on about 450 rushes). The help rule exists but doesn't bring him back (`game/calling.rs`, `help_on`, `HELP_GAP`; `game/scrimmage.rs`, `slide_to`, `CHIP_W`): one slide or one chip, and the passer doesn't throw sooner. Edges also don't rotate by plan (`GAPS.md`, "Edges' third and fourth men still play only on fatigue"). Tackles not traced.
- Touches: the match engine, records, awards (a defender's line is worth `6 × sacks` to voters, `world/college.rs`, `award_stats`), draft stock.
- Status: trial 10 finding 3 (high school, waiting) is the same at FBS.
- Built 2026-11-25 (thread "Fix freak defender stat lines"; `docs/match-engine.md`, "Defenders' season lines"): measured on this world's own FBS teams and schedule and on its NFL clubs (`match_check`, `AFM_AB_WORLD`, `AFM_DEFENDERS`), against real defenders' lines that are new in `data/real-world/defenders.json`. Six causes, none of them the help on the best rusher. (1) A block read a rating gap in a straight line, so an edge 10 to 20 points over the typical blocker won 52% to 65% of his rushes; it now reads the gap on a log curve that never goes flat (`BLOCK_BEND`), and he wins 38% to 48%. (2) A tackle's get-off read his weight with no floor, and (3) an athlete from another sport stayed at his sport's spot for life, so a 5-9, 167 lb thrower at tackle had 42 sacks a season. (4) Every man in a pile got the tackle, the first of them a solo. (5) Starters and linemen covered kickoffs. (6) A loss of an inch was a tackle for loss. In seasons of 13 games with nobody hurt, the best sack season a year went from 42.2 to 19.2 and the 2nd best from 25.6 to 15.5 (real 15.8 and 15.5), men with 20 sacks from 3.9 a year to 0.4 (real none), the best tackles-for-loss season from 49.2 to 27.8 (real 22.2) and the best tackle season from 205.7 to 185.0 (real 159.5). A 6-0, 240 lb edge with top tools still rushes with the league's best (`freak_check`): nothing is capped. Rushing held (5.36 a carry at FBS, 4.49 on the NFL clubs), and the Heisman voters' read no longer puts a defender on top (item 5). Left, all in `GAPS.md`: sacks and tackles for loss a team game are now short (FBS 1.78 and 4.35 against 2.09 and 5.64; NFL clubs 1.82 sacks against 2.40), because linebackers and backs rarely get home and the stars' extra sacks hid it; the tackle leaders are still 15% to 25% over real, because a team's tackles go to too few men. Both are the next lanes in the same thread. The book and its records show none of it until the next world rebuild.
- Second lane built 2026-11-25 (same thread; `docs/match-engine.md`, "The blitz gets home"): the pass rush, against real blitz counts that are new in `data/real-world/blitz.json` (PFR 2018-24). Four causes. (1) The protection took the nearest man from one sideline to the other, so on an over front a guard was sent out to the end and the edge had no blocker on 13.8% of his rushes; the linemen now take the men on the line in order, big on big. (2) A blitzer started flat-footed from his coverage spot and got there after the ball was gone; every blitzer now creeps through the cadence. (3) The check-release back waited to read a man near him; he now checks at the snap. (4) The passer never threw hot before his set; he now does from 0.85 s when a man he knows is free will be on him first. The book now counts any hurry, hit or sack as a pressure on a man's line, and a rush snap once a dropback. On the NFL clubs linebackers went from 0.086 pressures a rush to 0.168 (real 0.172 a blitz), corners 0.094 to 0.188 (0.181) and safeties 0.026 to 0.155 (0.159). Sacks a team game: FBS 1.78 to 1.93 (real 2.09), NFL clubs 1.82 to 1.74 (2.41), the free edges' sacks gone. Left, all in `GAPS.md`: the four-man rush gets half the real pressure, a pressure becomes a sack two thirds as often as real, a team's second-best tackle is single-blocked by a guard so the NFL clubs' best sack seasons rose to 24.7, 21.0 and 19.7 a year (real 19.4, 16.5, 15.8), and six or more rushers are sacked too often. The four-man rush is the next lane.
- Third lane built 2026-11-25 (same thread; `docs/match-engine.md`, "The four-man rush"): the tackle's set and the spare man's eyes. Two causes. (1) A pass blocker mirrored an end from the snap at the line, as he mirrors a man over him, so the two met 0.8 s after the snap with the end 0.65 yards past the line and 6.8 from the passer, and an end who won came off in front of the passer's eyes; against a man coming round the edge the tackle now kicks back to a set point 0.3 of the way to the passer's launch point, and the two meet at 1.2 s, 2.0 yards past the line. (2) The lineman left over against a four-man rush helped the block nearest him all play, so the other tackle had a guard alone; he now passes over a man a teammate is on his way to and helps where the rusher is winning. On the NFL clubs the edges went from 0.037 pressures a rush to 0.052 (real 0.066) and 0.53 sacks a team game to 0.70 (1.27), the tackles from 0.064 to 0.052 (0.031) and 0.83 to 0.64 (0.52); four-man rushes pressured 13.6% to 15.5% and sacked 3.5% to 3.4% (real 27.2% and 6.0%); sacks 1.72 to 1.76 a team game (2.41). Left, all in `GAPS.md`: a pressure becomes a sack two thirds as often as real, which now holds most of the missing sacks; an inside rusher wins his block as often as an edge; the best sack season a year is still 23.9 (real 19.4), now the long-armed power edge's.

**4. Quarterbacks go first overall less than half as often as real.**
- World: 31% of first picks are quarterbacks (real 73.1%), 11.3% of top-10 picks (19.1%). Several drafts have no first-round quarterback.
- Likely cause: a quarterback is 2 to 3 points behind other positions at draft age and grows longest (among pros with six or more seasons: 68.7 at 21 against 70.6 to 71.8 for linemen and backs, then 78 at 31). The board prices a position by today's grade (`world/recruiting.rs`, `draft_value`, `draft_position_value`: market value rises with `(grade - 66) / 24`), so the quarterback premium arrives after the draft.
- Touches: the draft, mocks (the best mock hits 1 first-round pick, real about 10), rookie deals, quarterback careers.
- Status: trial 7 finding 10, parked.

- Built 2026-11-25 (thread "Fix QBs going first in the draft"), decided by proxy while Cain slept: "the one franchise chance". Measured on the seed 321 world rebuilt on the stand-in (`draft_qb_check 321 30 1.0`, same seed before and after). The likely cause above didn't hold: pros grow nearly alike over a rookie deal (QBs 4.6 over their first four NFL seasons, the field 3.3 to 4.9), so pricing picks on the rookie deal moved nothing. What held: round one already had 3.2 QBs a draft (real 3.0), but the best one lost the first pick to a freak non-QB graded 89 to 106 while his own premium at grades 80 to 86 was 8 to 13 points, and in 12 of 30 drafts the club picking first already had a young QB rated 82+. The fix: until the first QB is taken, in rounds 1-2, a club without a young franchise QB adds the drop from the best QB on its board to the next one to the best one (a starting QB almost never reaches free agency). First picks QBs 33% to 60%, top 5 18% to 25% (real 28.5%), top 10 12% to 17% (19.1%), round one 3.2 to 3.5 (3.0). Size proposed, not confirmed. What's left is in `GAPS.md`: half the clubs have a young 82+ QB, and some classes' best QB sits 40th to 100th on talent.

**5. The Heisman and the MVP go to quarterbacks half as often as real.**
- World: 38% of Heisman winners are quarterbacks (real 80%, from memory) and 24% of NFL MVPs (84%). 46% of Defensive Players of the Year come from the front seven (80%). 0.45 quarterbacks in the jersey top 10 (4).
- Likely cause: the voters' weights are placeholders (`world/college.rs`, `heisman_kind`: QB 1.0, RB 0.84, defenders 0.75, "proposed, not confirmed"), and they read box scores that items 1 and 3 inflate for backs and defenders.
- Touches: awards, fame and NIL, the Hall of Fame, storylines.
- Status: trial 7 finding 9, parked. Measure again after 1 and 3.

### Medium

**6. The lower divisions score 4 to 6 points too many and have too few blowouts.**
- World: FCS 29.2 points a team game (real 26.3), Division II 31.2 (25.9), Division III 31.5 (27.2), junior college 32.5 on 6.8 yards a carry. Wins by 28 or more: FCS 12% (real 21%), Division II 11% (28%), Division III 10% (37%). Average margin in Division III 13.4 (real 23.9).
- Likely cause: two things. The rushing excess of item 1 grows as the level falls. And teams inside a lower division are too alike: Division III strength runs 46 to 57, where real Division III holds everything from Mount Union to winless programs.
- Touches: the match engine, the small-college world, late-bloomer stat footprints the draft reads.
- Status: new.

**7. High school scoring is still 5.5 points short and its record seasons are off.**
- World: 20.5 points a team game (real about 26, from memory; 15.5 at trial 10). Best sack season 82 (real about 40), best receiving season 1,601 yards (real about 2,500), most interceptions 9 (12).
- Likely cause: not traced. Trial 10 findings 2 to 5 are still waiting for a yes or no.
- Touches: high school games, recruits' stat lines, the high school seat.
- Status: known, waiting.

**8. The best FBS receivers draw too little.**
- World: 17 thousand-yard receivers a year (real 28 to 33). Among receivers with 25+ catches the 90th percentile is 834 yards (real 922) and the 99th 1,183 (1,353).
- Likely cause: the passer spreads the ball (`GAPS.md`: "The passer throws to his second read more than his first", "Fast receivers draw too few deep targets").
- Touches: the match engine, receivers' draft stock and awards.
- Status: the two gaps are listed; the world count is new.

**9. Group of 5 head coaches are paid 2.5 to 4 times real.**
- World: MAC head coaches average $3.86M, Conference USA $3.73M, the Sun Belt $3.55M, the Mountain West $4.33M, the American $5.10M (real about $0.8M, $0.9M, $1.0M, $1.6M and $2.3M, USA Today's database from memory). The SEC ($7.15M) and Big Ten ($6.65M) are near real.
- Likely cause: the job's line is `lo × (hi / lo) ^ prestige` with $0.6M to $12M at FBS (`world/staff.rs`, `salary`), so a prestige of 0.5 already pays $2.7M, and item 11 pushes every prestige up.
- Touches: coach contracts and buyouts, college budgets (a Group of 5 school has $48M in all), the carousel (why leave for a small raise).
- Status: built 2026-11-24 (thread "Fix Group of Five coach pay"). The cause was the line's place by prestige and a cap of 7% of a budget also read by prestige, which the market's renewals reached. The line now reads what the department earns on its books and the cap is the line stretched (`money_place`, `PAY_MONEY`, `pay::COLLEGE_STRETCH`). Seed 331 (12 years, stand-in games) in 2025, head coaches: MAC $1.23M, Conference USA $1.13M, the Sun Belt $1.19M, the Mountain West $1.34M, the American $1.49M, the Big 12 $5.04M, the ACC $5.76M, the Big Ten $7.45M, the SEC $8.03M (before: $3-5M across the Group of 5). The Group of 5 average $1.28M (real about $1.3M); the American is low and the MAC high because the books' Group of 5 money is bunched (`GAPS.md`). Sizes proposed, not confirmed.

**10. Prestige drifts up and bunches.**
- World: FBS prestige averages 0.51 in 1995 and 0.60 in 2024. The spread shrinks from 0.19 to 0.14 and the lowest rises from 0.1 to 0.3.
- Likely cause: prestige reads booster money in plain dollars, `sqrt(boosters / $25M)` (`world/program.rs`, `prestige_of`), and money grows with every era.
- Touches: recruiting reach, coach pay and contract length, searches, the footprint. Another 20 seasons of the same drift would crowd the top.
- Status: new.

**11. NFL champions repeat too often.**
- World: 6 back-to-back champions in 28 seasons with two three-peats (real 3 and none). Win share carried to the next season 0.39 (real 0.329).
- Likely cause: not traced. Clubs keep too few of their own free agents and good veterans go unsigned (both in `GAPS.md`), so the same strong rosters stay strong by other means; the draft and the cap don't pull them back.
- Touches: the NFL seat, the cap, free agency, the draft order's worth.
- Status: new as a pattern.

**12. General managers are almost never replaced.**
- World: the average GM has held his job 13.9 years (real 4 to 6, from memory).
- Likely cause: few callers of `replace_gm` (`world/pro/office.rs`); owners fire coaches, rarely the GM.
- Touches: front offices, the NFL seat's job security, item 11.
- Status: new.

**13. Too few undrafted players in the NFL.**
- World: 27.9% of 53-man rosters went undrafted (real 40.8%, nflverse), 351 undrafted signings a year (about 550), 312 unrestricted free agents a year (about 550).
- Likely cause: not traced. Drafted players last too long (round-one careers 11 years at the median, real 9), so fewer places open.
- Touches: the undrafted rush, free agency, pro scouting, careers.
- Status: new.

**14. NIL is spread too evenly across a power roster.**
- World: 24% of Power 4 players earn under $10,000 a year and 44% of SEC and Big Ten players earn $100,000 or more. A power program pays $27M to $31M in all; the Power 4 total of $1.94B sits beside Opendorse's $1.9B for 2025.
- Corrected 2026-11-24 (`docs/research/nil-split-by-sport.md`, Cain's "sure" to its item 1): the "real 66.5% and about 10%" are Opendorse's Power 4 football figures for July 2024 to August 2025, before revenue sharing began, not the 2025-26 year the world's payday is in. Against the College Sports Commission's 2025-26 pay, real is about 25-30% under $10,000 and about 35-45% at $100,000 or more (Claude's arithmetic, not published figures), so the world's 24% (seed 321, a 2025 payday) is at real; the 44% was one world and one year: the seed 1 world's July 2026 payday reads 66-68% at $100,000 or more in the SEC, Big Ten and ACC, well over real. See the research doc's "Measured" section. Opendorse's $1.9B is its projection for college football players, not every sport, so the totals compare football with football. The gap that stays is the low end of the paid set (real 10th percentile among the paid $17,500-$25,000; the world's $82K-$202K).
- Likely cause: not traced; the paid set and its floors (`world/ncaa.rs`, `nil_market`; `world/program.rs`, `third_party`).
- Touches: NIL, recruiting pulls, the portal, your budget.
- Status: built 2026-11-25 (thread "Spread NIL pay by player value"; researched 2026-11-24, `docs/research/nil-split-by-sport.md`). The cause was the revenue share's three role weights (a starter 1, the rotation 0.45, everyone else 0.25), with a recruit's floor read from the depth at his spot. A paid man's weight is now his own standing × his position's worth (`share_standing`, `share_weight`, `nil_bid::ShareBars`). Seed 1's July 2026 payday, played on the engine before and after: the paid 10th percentile is $21-25K at every position (before $81-200K; real $17,500-$25,000), 44% of a Power 4 roster makes $100,000 or more (66%; real 35-45%), offensive linemen's 90th percentile is $498K (real $512K) and quarterbacks' $1.65M (real $1.611M), and the medians are within 12% of the Commission's but for receivers (−21%) and the defensive line (−22%). The class of 2027's Power 4 three-stars sign for a median $30K (before $122K) and two-stars for $10K ($111K). Open: the medians on a played second season (the stand-in's July 2027 can't read them), deals paid in the program's ratio of its pots, and the third party's quarterback market, each in `GAPS.md`; the research doc's "Item 4, built and measured" has every number. Sizes proposed, not confirmed.

**15. NFL games missed read at a quarter of real.**
- World: 0.28 to 0.34 games missed a player-season (real 1.2).
- Likely cause: a season row leaves out games on injured reserve, where the long absences are (`docs/reference/how-it-works/injuries.md`; `world/pro/games.rs`, `record`). So part of this is the count, not the injuries. Measure both before changing either.
- Touches: injuries, careers, depth, the medical read.
- Status: known as a count.

**16. NFL passing and scoring are still hot.**
- World: 24.9 points a team game in 2025 (real 22.5), 7.8 yards an attempt (7.05), QB EPA 0.147 a dropback (0.070), 11.8 passers a year over 4,000 yards (9.4), time to throw 2.28 s (2.71), pressure rate 21.4% (29.5%).
- Likely cause: in `GAPS.md` ("NFL scoring runs 4 to 7 points over real"). Cain chose on 2026-10-19 to leave NFL passing as it was for now.
- Touches: the match engine, NFL stat lines, contracts by production.
- Status: known. Listed only because every engine lane is now built.

### Low

**17. Stars against the draft.** 5.5% of draft picks were unrated recruits (real 25.2%), 47% were three-stars (35.9%), 72% of five-stars are drafted (63%) and 16% of four-stars (22.1%). Five-stars go to 22.8 programs a class (real 16). Likely cause: the services rate nearly everyone who signs, and the real data has holes for junior college and late players (`world/recruiting.rs`, `star_score`). Status: trial 10 finding 7, waiting.

**18. Kickers play too long.** Drafted kickers last 14 years at the median (real 6). They rise to 84 and stay; nobody loses his job to a bad year. Not traced. Status: new.

**19. The FBS home edge is short, and so are finals by 3 and 7.** Home teams win 54.9% (real 58.5%). 6.7% of games end by 3 (real 9.9%) and 6.2% by 7 (8.6%). Not traced. Status: new.

**20. FBS portal entrants lag the latest real rise.** 3,097 in 2024 (real about 3,700 in 2025, Hudl). Near real for 2021-22. Status: new.

**21. Draft picks come from too many colleges.** 118 colleges with a pick a year (real 97), 8.2% of picks from FCS (real 6%). Status: new.

**22. Too few in-season NFL trades.** 9.2 a year (real 17.6). Trades in all are near (85, real 91). Status: trial 7 finding 8, parked. Measured 2026-11-24 (thread "Fix in-season NFL trades"): the world was at real and the book read the wrong count. Its check read `deadline_trades`, the trades of deadline week alone (the two calls four days before the deadline and on the day), against the real count from September to November. The world's own count of every trade from kickoff to the deadline (`season_trades`) is 16.4 a year in 2021-2025 (12, 15, 18, 17, 20; real 15, 19, 15, 18, 21, mean 17.6), and 11.5 in 2002-2020 (real 11.1). The check and the trades chart now read `season_trades`, and the chart's "Trades" line counts draft-day trades too, as the real figure does (`crates/afm-reader/src/defs_nfl.rs`, `N9.season_trades`). No game change. Status: fixed (the measure).

**23. History plays one era.** FBS teams throw 29.6 passes a game in 1996-99 and 31.2 in 2020-24, and complete 58.8% then and 60.3% now. Real football changed more. History only. Status: new.

**24. Reader housekeeping (tools).** The known-issues list still shows odd-28 as open, but the code reads the game team now (`world/pro/games.rs`, `record`). "Different FBS champions in 15 years" reads 11.5 because it counts titles of programs that were FCS then. "Median peak age of quarterbacks" reads 24 because it counts every quarterback who ever reached a camp; established ones peak at 28.0. "FBS roster size" reads 1 January, before the trim to 105. The stat book's Division II and III scoring has data in program seasons, but the checks wait for the game log.

**25. The future run.** Play this world on 10 seasons with the log, to read what it can't answer (above). About 4 to 5 runner hours. Needs Cain's yes.

## Systems each high item touches, in short

| Item | Engine | Recruiting and portal | Money | Draft and NFL | Records and awards |
|---|---|---|---|---|---|
| 1 FBS rushing | yes | stat footprints | | backs' stock | yes |
| 2 Frozen programs | | yes | prestige, pay | | streaks |
| 3 Freak defenders | yes | | | linemen's stock | yes |
| 4 Quarterbacks in the draft | | | rookie deals | yes | |
| 5 Awards | | fame, NIL | | | yes |

## Method notes

- Program strength, wins and totals: `programs__seasons` (25,946 rows). Player seasons: `stats__seasons` (1.79 million rows) with `engine_stats` for the columns. Careers: `alumni__player__history` and `people__player__history`. Games: `college__games` and `college_prev__games` (2025 and 2024). Money: `programs` (collectives), `coaches` (contracts), `finance__league`, `market__years`. Flows: `log_history`.
- Real season lines: `college_players.json` (ESPN box scores, 2021-24 means). Real scores: `college_scores.json`, `college_history.json`, `nfl_game_targets.json`, `nfl_history.json`. Real careers: `nfl_careers.json`, `development_injuries.json`. Real NIL and portal: `rates.json`.
- One world is one draw. Title counts and single records swing between seeds; the rates over 3,500 program seasons and 1.79 million player seasons do not.
