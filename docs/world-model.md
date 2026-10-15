# How the world works (section 2)

The build of `docs/design/world.md`. Code: `crates/afm-engine/src/world/`. Trial report: `cargo run --release --bin world_trial -- <seed> <years>` (in `afm-tools`, debug only). The latest trial is `docs/trials/world-trial2.md`.

Everything below that Raichel has not answered is marked **proposed, not confirmed**.

## The shape of it

- `World::new` builds the whole country, then runs 30 years of hidden history before the 2026 start (decision: 30 years). Six extra spin-up years run first so rosters, coaches and prestige settle; nothing from them is kept.
- Every year runs in this order: new 14-year-old class, high school transfers, rosters, games, coach reputations, the coaching carousel, players leaving college (draft and graduation), the portal, signing day, program prestige and grades, life events.
- Each person has his own random stream, so a seed gives the same world on any machine. A 30-year world takes about 100 seconds in release mode.
- Game results are a stand-in until section 3: each team's strength comes from its starters, with home field and a win chance from the strength gap.

## Programs (topics 2, 6, 7)

- Real colleges from `data/real-world/colleges.csv`: 138 FBS, 127 FCS (Chicago State, with no team, is left out), 160 D2. JUCO programs are placeholders ("TX Junior College 1") until the real list is researched (waits on realism question 1).
- Prestige comes from history, recent results, the head coach, facilities, NIL, fans and TV, and conference (topic 7). Recruits read it relative to their division, so a strong D2 school does not out-pull an FBS one.
- 12 EA-style grades. Steady ones move about 15% of the way to their target each year (tradition, academics, facilities, atmosphere, campus life, brand exposure). Fast ones move about 80% (title chances, playing time, coach prestige, NIL, pro pipeline). Conference strength moves only on realignment (Raichel confirmed the split). Two straight titles take a C- program to about B+.
- NIL collectives have budgets that follow results and booster mood, with booster departures. A falling collective sends warnings through the inbox, the news and a falling arrow on the NIL grade, getting louder each year (Raichel's follow-up answer 4).
- Recruiting footprints grow with prestige; pipelines to a high school grow only from players who made an impact, and fade slowly (topic 4).
- FCS playoff: the SWAC and MEAC stay out, as in real life (Celebration Bowl). The Ivy League is in (it joined from 2025).

## High schools (topics 1, 2)

- Each state gets as many schools as the data says. The state's real top programs keep their names; the rest are generated in real towns from the data. Classifications go by enrollment.
- Powerhouses recruit transfers, the national ones out of state too. Transfers follow each state's real sit-out rule from the data.
- High school NIL follows each state's real rule (`hs_nil_allowed` in `hs_states.csv`; 47 of 51 allow it).
- A rare pull lets a kid from a bad home transfer to a powerhouse (Raichel confirmed: rare).

## Players' hidden background and family (topics 1, 5)

- Six background parts: money, safety, family (home stability), school resources, exposure network, academics (GPA). Plus a hidden mental-health stability level (confirmed).
- Hardship adds a little grit, a YAC tendency and mental toughness. It is never a driver.
- Risk: high potential with a bad home, or a perfect setup with low ambition, raises trouble and quitting a little.
- Families have a decider (parent, grandparent, handler, the kid himself) with their own wants. A handler can steer a kid to the wrong school. Siblings and bloodlines are a small influence.
- Scouts see a character report whose accuracy depends on the scout.

## Coaches (topic 3)

- Hidden ability (teaching by position group, scheme, game management, motivation, discipline, recruiting, network, relationships) and a public reputation that lags it.
- Reputation catches up about 30% a year. A title gives a massive boost, bigger the lower the coach's starting reputation (the Cignetti case). Bad results chip it away slowly. Assistants move at half speed.
- Hiring runs on recency bias: the last two seasons count most.
- Coaches age, improve and decline, and pass a little of their teaching to assistants (coaching trees, an influence only).
- Patience when firing grows with tenure (proposed, not confirmed: year-one coaches are almost never fired).

## Recruiting and movement (topics 4, 6, 8)

- Visibility decides who gets seen. Stars come from film and projection, with the lowest-visibility kids underrated.
- Signing day is a matching: programs offer down their boards, and each recruit (or his decider) picks by what he wants: level, brand, playing time, distance from home, academics, development, NIL.
- JUCO: kids go there for grades, late growth or no offers, and move up after 1-2 years or stop. JUCO grades improve while there.
- The portal: playing time, a coaching change, a handler's steer, moving up, dismissal or money. Tampering is quiet and never punished; it makes moving up more likely at schools with small collectives. NIL can backfire on a player.
- The draft: 257 picks plus undrafted signings, scored from the scouted rating and projection. Kickers and punters go late.

## Trial 1 additions (built 2026-10-13)

Raichel's running list from the trial 1 walk-through (`/mnt/project-files/design/world-trial1-feedback.md`).

1. **Prove-it extension.** A head coach with 5+ years, loyalty (no job-hopping, or one move with high loyalty) and real success (a title in his last 8 seasons, or 70% wins over 50+ games) who has two bad years in a row usually gets one more year, more often when there's an excuse (many players lost to the draft, a young lineup). The news reports it. Fail the prove-it year (a losing record, or well below expectation without a strong record) and he's fired.
2. **Ethnicity and hiring bias.** Players and coaches have an ethnicity. It never changes a rating. Players match who really plays at each level (state child population, how often each group plays, and the real FBS mix). AI hiring carries a hidden bias against coaches of color, strongest in the history, bigger at big jobs, fading slowly after 2026, calibrated toward TIDES counts. The player's own hires never carry it. A yearly diversity report appears in the news.
3. **Names** follow ethnicity and birth decade (`data/names/`). Rebuilt 2026-10-14 (world trial 3 finding 9): every SSA boy's name since 1940 (38,669) and every Census surname with 100+ people (167,407), plus suffixes (Jr., II, III), hyphenated surnames, players who go by initials and inner-capital spellings (DeAndre). Shares are proposed, not confirmed; see `data/names/README.md`.
4. **Pipelines.** Impact is judged inside a player's own conference (all-conference, awards, draft round). Pipelines now exist to a high school and to its area, and staff build personal ties to the schools their impact players came from; ties move with the coach. States with more FBS-level talent than their own big programs can sign spill over to nearby Group of 5 and FCS schools.
5. **Coach ladder.** Every college has 8 position coaches under the coordinators. Coaches climb from high school to position coach to coordinator to head coach, or skip steps on buzz (a high school coach with recent state titles and a dominant record gets college looks). A new head coach keeps some of the old staff. The stat-record part of buzz waits for section 3.
6. **Academics.** NCAA tiers decide who can play right away (full qualifier), who must sit year one (academic redshirt) and who must go JUCO. Strict schools have admissions bars and a few special admits a year. A recruit with risky grades is weighed by the school's academic support, and some players grow out of it. College GPA drifts, and a low GPA can cost a season.
7. **Draft value.** NFL teams draft one by one with their own needs and a position value calibrated to 2000-2025 drafts (QBs go first, kickers late). Every pick weighs the player's grade, the team's board, position value and team need. A QB's premium depends on the team needing one, except at the very top. A team with a young franchise QB holding a top-5 pick trades down to a QB-needy team when a first-round QB is there. At the top, great players at lesser positions count for less than their grade (a generational WR rarely goes first). The numbers are proposed, not confirmed.
8. **Player pages.** Height, weight, arm, hand and wingspan from the real combine per position, full combine tests, level ratings each season, the coaches he had, his offers and why he was drafted.

Fixes from trial 1's flags: foster homes no longer give money (flag 3), the NIL life event happens once (flag 4), and unsigned 3+ star recruits get a reason (flag 5).

## Trial 3 fix list (built 2026-10-14)

Raichel's list in `/mnt/project-files/design/world-trial3-candidates.md`. Numbers marked (proposed) are Claude's picks inside the agreed design.

- **Names (1, 2).** The 1,307 names only the mortgage source has are reweighted from that source's own mix to national shares (Trey is now about 15% Black). DeShawn is added as an estimate.
- **Coaching gap in the 2000s (3).** The hiring bias eases from 4.2 points in 2005 by 0.08 a year (was 5.0 and 0.12), to bring 2009 nearer the real 7 of 120 (proposed).
- **Coach hiring on visible evidence (5, 15A-B).** A high school coach's buzz counts only his own seasons. Every coach keeps a record per position group: how much his players grew against others of the same level and age, and how many moved up (signed with a bigger school, made the draft). Schools read his teaching through noise (sd 1.5-5 points, sharper for a better-connected hiring coach) plus that record. Each coach has a "came up as" group; a coordinator from the other side of the ball reads 1 point worse. Head-job hires mark down coaches past 58, and past 54 if they were never a head coach. An FBS head job takes coordinators only from FBS (proposed numbers).
- **Every rise has a cause (15C).** A head coach's recruiting and relationships, and his position coach's recruiting, add a small bond with recruits. Facilities age a little each year and move in steps (a booster gift or a budget decision), more likely the further behind a school is. Tradition moves slowly with recent results and titles. Each program keeps a story line: new head coach, facility project, title (proposed numbers).
- **NIL offers per player (13).** An offer is the real position value (QB about $650k at the top of FBS down to K/P $25k) times his talent and rank, times the collective's size. Schools bid against a better offer for their top targets. A program with a hot head coach and a collective bigger than its prestige can "reach" with a 1.6x offer some years. Money weighs by family need (wealth, a crisis) and ambition shifts weight from money to the best path (playing time, development). Player pages say what his deal was (proposed numbers).
- **Bench (6).** The bench formula is refit so position means land on the real combine means; WR and CB still run about 2 high.
- **Kickers and punters (7).** Leg strength now leans on a specialist body, so the strongest legs are not on lineman-sized bodies. College kickers average 70.7 in and 188 lb, punters 72.2 in and 199 lb.
- **Height format (8).** Pages show 6'0 3/4", to the eighth, like the combine.
- **College games on the match engine (12).** Every college game is played by the match engine, the whole history included, so the history comes from the same football as the save (Raichel, 2026-10-14: the final game builds 30 years this way; trial 3 uses 15). At about 45 ms a game that takes about 2 minutes a season on 4 cores, so the engine needs speeding up. `AFM_ENGINE_FROM=<year>` in `world_trial` uses the old stand-in before that year, for quick checks only. Team schemes are picked from the coach for now (proposed). High school games stay on the stand-in (about 70,000 a year).
- **Match engine fix.** Injuries on kick returns were booked to the wrong team.
- **Dynasties (4, Raichel 2026-10-14).** A trace showed one program fielding the best lineup in 28 of 30 years, because recruits judged playing time from one school-wide grade. Now each recruit and transfer also weighs his position room: who at his position will still be there and ahead of him by his second year. He also weighs the units his position leans on, against his division's norm: a QB his receivers, line and tight ends, a WR his QB, a corner his pass rush and safeties, and so on. Buried backups who would start elsewhere enter the portal more often. In the check run the most titles in 30 years fell from 16 to 6 (Georgia and Tennessee), and the five-star draft rate fell to 64% (real 63%). The weights and the "linked units" table are proposed, not confirmed.
- **Head-job searches.** A search now calls up to 20 names, then promotes the school's own coordinator. Before, an SEC school could end up with a brand-new coach.
- **Hiring bias after 2016.** Real progress stalled after the mid-2010s, so the ease stops in 2016. A check run gave 7 Black FBS head coaches in 2009 (real 7) and 17-20 in 2025 (real 16).
- **Trial report.** Each program shows why it moved (hires, facility projects), and the report lists the longest Power 4 runs without a winning season, the most titles, and the top program's lineup strength against the rest of FBS every year.
- Items 9, 10, 11 and 14 are checks, measured in world trial 3. Last check run: pick 1 a QB 58-62% of the time (real 73%), three-stars drafted 4.8% (real 6.0%), two-stars 1.0% (real 2.1%), longest Power 4 drought 21 seasons (real high about 16).

## Coaching market (dynasty item 11.11-11.15, 2026-10-15)

Raichel approved the five items on 2026-10-15. Why: in 30-year trials one coach held a top program for 12-14 years and nobody hired him away. Every number below is proposed, not confirmed. Code: `world/staff.rs` (market, NFL staffs), `world/coach.rs` (age), `league.rs` (NFL staff slots).

- **Short lists (11.11).** A search for a college head or coordinator job ranks every coach who fits the job by how hot he is, and calls the top names first: 15 for a head job, 10 for a coordinator job. The random draw of 40 names stays, so lesser-known names still appear. Hotness is what a committee can see: reputation with recency bias, +3 for the biggest stage (scaled by the job's prestige), +2 for each conference title and +4 for each national title in his last three seasons there, and up to +3 for a program rising under him (+1 per 0.01 of prestige gained in three years, down to -1). An FBS head job also calls the 3 hottest NFL coordinators. A college search marks down a man who never coached in college by 2 points (no recruiting ties). Position coach searches still draw at random and judge unit records. Prestige still decides who can lure whom (11.13).
- **NFL staffs and hiring (11.12).** Each of the 32 NFL teams has a named head coach and two coordinators (`Team::head_coach`, `oc`, `dc`). The staff sets the team's coaching rating (head coach half, each coordinator a quarter on his side of the ball); the old random staff changes in `advance_league` now apply only to the class sim. Each season a team's record comes from its roster (0.13 win share per roster sd), its staff (0.06 per staff sd) and luck (sd 0.12), so win shares spread about like the real 0.19. The 12 (14 from 2020) best records make the playoffs; the champion and runner-up are drawn by record. Owners fire on wins: 1.6 × (0.5 − last win share) + 0.6 × the year before, less half the coach's result against his roster, past 0.1, times 3.2, times owner patience; 30% in year one, 70% in year two. A new head coach keeps each coordinator 30% of the time; a coach who stays fires a coordinator after a losing year (up to 35%). Candidates for a head job: every NFL coordinator, out-of-work former NFL head coaches and coordinators, and the 8 hottest FBS head coaches. Owners mark down a man with no NFL experience by 4 points (2 for a coordinator job). If nobody says yes, the team promotes its own coordinator.
- **How a coach weighs a move (11.13).** Before, a sitting coach moved only for a prestige gain over 0.03. Now the gain is one part of his "pull", all in prestige points. The money: 0.05 × ln(new pay / old pay), so doubling his pay is worth 0.035. Pay rises with prestige inside each level (FBS head coach $0.6M-12M, FCS $0.15-0.7M, D2 $70-200k, JUCO $50-120k, high school $50-160k; coordinators and position coaches have their own ranges), a winner has had 10% raises per title (to three), NFL head coaches earn $5-14M by owner and coordinators $1.2-4M, and the NFL paid up to twice as much in the 1990s, easing to college levels by 2024 (Claude's reading of 2024-25 pay, to check). A first head job adds 0.12 (0.18 for the most ambitious). The NFL adds up to ±0.04 by ambition (the hidden 1-20 trait the coach already had), and a gifted recruiter gives up his edge there (−0.02); an NFL coach who recruits well is drawn back to college (+0.02). A young core of starters coming back holds a head coach (−0.03). Age past 52 costs 0.003 a year and each season settled 0.0015 (to 12). Each coach now has a hidden home state (half the time where his first job was, else drawn by where players grow up): going home adds 0.03, leaving home costs 0.02. A seat getting hot adds up to 0.03. The move must clear 0.03 (the old bar), then ambition and loyalty set the chance as before. Story lines name the two biggest pulls ("a bigger program and a big raise", "the step up to the NFL", "a chance to go home to TX"), on both programs' stories and in the news.
- **Age (11.14).** Before, a coach at his learning ceiling never declined (only scheme did). Each coach now has a hidden age his sharpness starts to fade (about 59, sd 3.5, 52-68) and a pace (0.08-0.35 points a year, 15% faster each year). Recruiting fades fastest (1.2×), then motivation (1.0×), game management (0.8×), relationships (0.6×), discipline (0.5×) and teaching (0.4×). Network holds. Learning stops at 58. A coach at pace 0.2 loses about 1.6 points of head-coach value from 58 to 65.
- **Early exits (11.15).** Coaches under 60 can retire for their health (0.08% a year at 40, 0.55% at 59) or step away burned out (0.8% a year after ten seasons in the biggest head jobs, scaled by the job's prestige squared and tenure to 15 seasons, ±40% by temper; a coordinator carries 40% of the pressure). After 60 the old retirement shape stays (4% at 60, +3 points a year). Both leave a story line with the reason.
- **Check run** (seed 2026, 30 years, stand-in history, `AFM_ENGINE_FROM=2026`; the report's "Coaching market" section): FBS head coach changes 27.1 a year (real about 20-30); FBS head coaches fired 18.7; hired away by another college 4.3 and by the NFL 0.6 (real about 5-7 and 0.5, Claude's reading, not checked); NFL head coach hires 6.3 a year (real 2015-25 about 7), 4.4 from NFL coordinators and 0.6 straight from college; retirements before 60: 43.7 a year at all levels, 0.4 of them FBS head coaches (real about 0.5, Claude's reading). FBS national-title coaches stayed 15.6 seasons on average at that school (median 14; real title coaches' tenures run about 10-15, Claude's reading); the most titles by one coach was 4 (three coaches). One college head coach went to the NFL about every two years, spread over the 30 years.
- The 3-year world test's FCS draft share bound moved from 12% to 14%: that short world runs 11.5-12.7% on seeds 7-9 before this build and 12.5-13.0% after; the 30-year world gives 6.2% (real about 6%).

## Era limits, NIL and prestige (dynasty items 11.4-11.10, 2026-10-15)

Raichel approved items 11.4-11.10 in the "Dynasty fix" thread (`/mnt/project-files/afm/dynasty-fix/round1-kickers-and-item11.md`). Code: `world/ncaa.rs` (rules by era, NIL budgets, rogue boosters and the NCAA), with small hooks in `program.rs`, `staff.rs`, `season.rs` and `recruiting.rs`. **Status: built and compiling; the 30-year world check run and the trial numbers against real are not done yet** (the build moved to a fresh thread).

How it works:
- **11.4 Roster limits by era.** `ncaa::roster_limits(program, season)` is the one function every roster rule reads: (roster limit, most new signees a year), after any NCAA scholarship cut. FBS: 95 scholarships until 1991, 85 from 1992 (the 1991 NCAA convention; Sea of Blue, "A brief history of NCAA football scholarships"), 25 initial counters a year (NCAA Bylaw 15.5.6.1), until 2024; from 2025 the House settlement's 105-man roster with every player eligible for a scholarship and no yearly cap (CBS Sports, 2025). FCS: 63 equivalencies spread over at most 85 players on aid, 30 initial counters a year (Bylaw 15.5.6.2, quoted in College Sports Journal; 63 since 1994, Wikipedia FCS); from 2025 the 90 FCS programs that opted into the House settlement follow the FBS rule and the 36 that opted out keep the old one (SI, July 15, 2025; the list is in `ncaa.rs`). D2: 36 equivalencies and no cap on players on aid (scholarshipstats.com). JUCO: the NJCAA's 85 scholarships (scholarshipstats.com). The world has no walk-ons yet, so its rosters are the scholarship players.
- **11.5 No NIL before July 2021.** `Collective.budget` is now the NIL money a program can pay players in an offseason (`ncaa::nil_budget`): zero through the 2020 offseason, so the class of 2021 signed without NIL and the class of 2022 was the first with it. The booster base (`base`, `big_booster`) gives in every era: before NIL its money counts toward facilities and prestige only. NIL offers, roster pay, the "bought away" portal reason, the rising-program reach and the collective warnings all start in 2021. Before 2021 the NIL grade ranks the booster base.
- **11.6 Illegal booster money before 2021.** A few FBS programs have a hidden rogue booster with a yearly pot. On signing day he adds cash to his program's offers to a few of the best recruits in its footprint; it works through the recruit's existing money weight, so it sways needy families most. A target is paid only if he signs. Payments surface with a chance a year of 1 - exp(-paid in the last four years / $8M) (four years is the NCAA statute of limitations). A case brings real penalty types through the world's own systems: scholarships and signees cut for three seasons (`roster_limits`), no conference title and no playoff for one or two seasons (`season.rs`), and a show-cause order that ends the named coach's college job and keeps colleges from hiring him (`Coach::show_cause`). The program disassociates the booster. From July 2021 rogue boosters stop (the money goes through the collective), but payments from before can still surface for four years (Tennessee's 2023 case covered 2018-20). Real anchors: SMU (death penalty, 1987), Alabama 2002 (booster Logan Young paid about $150,000 for one recruit; 21 scholarships over three years, a two-year bowl ban), USC 2010 (30 scholarships over three years, two-year bowl ban, one-year show-cause for assistant Todd McNair), Ohio State 2011 (85 to 82 for three years, a one-year bowl ban, a five-year show-cause for Jim Tressel), Miami 2013 (9 scholarships over three years, two-year show-causes for three assistants), Ole Miss 2017 (two-year bowl ban, 13 scholarships, show-causes of eight and five years for two staff members).
- **11.7 A ceiling on NIL budgets.** No booster base gives more than $20M a year (`ncaa::COLLECTIVE_CEILING`): Ohio State's players got "around $20 million" in 2024 (AD Ross Bjork, CBS Sports, July 24, 2024), Notre Dame's collective took in $20.5M in 2023 and the Texas One Fund spent $13.3M (Sportico tax filings via CBS Sports, Dec. 30, 2024), and Opendorse's Blake Lawrence: "very few collectives actually had $20 million dollars to spend a year". Growth slows as the boosters near the ceiling and never passes it. The NIL market grows into its 2024 size (Opendorse Power 4 totals: 2021 $393M, 2022 $667M, 2023 $840M, 2024 $1.0B). From 2025 revenue sharing adds 75% of a $20.5M cap growing 4% a year for Power 4 schools (Front Office Sports: Texas and Texas Tech give football about 75%, $15.37M; CBS Sports: 4% a year), and the American's floor ($10M over three years) for the Group of 5.
- **11.8 NIL offers within real ranges.** No offer goes above the top of the real range for a starter at a major program (`rates.json`, CBS via SI 2024): QB $800K, OL $500K, DL $600K, RB, WR and LB $300K, DB $225K.
- **11.9 A fixed yardstick for prestige.** `prestige_rel` is prestige divided by the division leader's prestige when recorded history began, not by the current leader's. The spin-up years still measure against the leader. A program can now read above 1.0.
- **11.10 Recent results build slowly.** `recent = 0.75 recent + 0.25 season score`, and the season score is no longer capped at 1.0, so a title season counts more than a plain 12-1 season.

Proposed, not confirmed (Claude's picks inside the approved wording):
- D2 rosters stay at 110 (no cap on players on aid in D2); JUCO moves from 80 to the NJCAA's 85 for every JUCO, the California schools too (they give no athletic aid).
- Where no NCAA yearly cap exists (FBS and opted-in FCS from 2025, D2, JUCO), staffs still sign at most about 1.4 normal classes (39, 33, 42, 56), the world's earlier pick.
- The 25-signee limit runs through 2024 as approved; the real NCAA waived it for 2022-23 and 2023-24 (CBS Sports, May 18, 2022), keeping the 85 cap. Raichel may want the waiver.
- The NIL market ramp (each collective follows the Power 4 total), football's 75% of revenue sharing for every Power 4 school, the Group of 5 at the American's floor, and no revenue sharing in FCS and below.
- Tight end offers top out at $300K and specialists at $50K (not in the data).
- Rogue boosters: a yearly chance of 0.02 times the fan base (1.5x with a big booster) to gain one, 10% a year to stop, a pot of $50K-$300K, prices of $150K, $50K and $15K for a 5-, 4- and 3-star, at most four targets a year in the program's footprint. Detection scale $8M. Penalties: one scholarship per $15K found (3 to 30) over three seasons; a one-year postseason ban from $100K and two from $300K; the head coach named in one case in four, otherwise the recruit's position coach, for one year plus one per $100K (up to 8). FBS only.
- The yardstick (the leader's prestige when recorded history began) and the 0.25 weight on the latest season.

## Kickers, punters and roster needs (fix list topic 1, 2026-10-15)

Raichel's yes on items 1.1-1.11 (`/mnt/project-files/afm/dynasty-fix/round1-kickers-and-item11.md`). Code: `world/recruiting.rs` (class plans, boards, camps, walk-ons, portal), `positions.rs` (the long snapper), `game/roster.rs` and `game/flow.rs` (who kicks), `league.rs` (NFL long snappers).

How it works:
- **A class plan per position (1.1, 1.2).** Each staff wants the real FBS roster count at each spot, scaled to its roster limit (`real_roster`, `roster_limits`). It signs this year's hole, or half the hole two years out, whichever is bigger. Two years out, seniors, likely early NFL entrants (read against the last draft) and likely transfers count as gone. Each staff ranks recruits on a board per position.
- **Talent beats need (1.3).** At a full position a staff still offers a recruit who is clearly better than the best one left at its open spots: a quarter of a film standard deviation better at the top program, a full one at the bottom, at most two over the plan. Not for specialists.
- **Specialists (1.4-1.6).** Kickers and punters are graded on leg power, accuracy and the mental side, against their own spot. Their stars come from the kicking camps (top 25 at a spot five stars, 100 four, 250 three, 500 two), not the general ranking. A staff keeps about one kicker and one punter on scholarship; the other kickers, the second punter and the long snappers walk on. A walk-on gets no NIL and pays his way, as at Pioneer League schools.
- **The long snapper (1.7)** is a position. His rating is his snap (the hidden snapping trait). Nobody is placed there in high school; each high school's best snapper who isn't a ranked prospect can be ranked by the snapping camps, and then colleges recruit him as a snapper.
- **Who kicks (1.8, 1.9)**: see `docs/match-engine.md`. A kicker kicks and a punter punts, a big-legged kicker takes the kickoffs, and with only one of them he does both.
- **Portal (1.10).** A program whose starter left and who has no returning starter at that spot reaches out to every entrant there who beats its remaining backups, best entrants first. The 40 random programs stay (item 11.16 is not built).
- **NFL (1.11).** Every NFL team carries one long snapper. Teams draft a kicker, punter or snapper only when theirs is below par or aging, with a flat discount on top of the old position value.

Results against real (seed 2026, 8 years of history):

| FBS, after signing day | Game | Real |
|---|---|---|
| Teams with no kicker / no punter / neither | 0 / 0 / 0 of 138 | 0% / 4-6% / 0% |
| Kickers / punters / long snappers per roster | 3.1 / 1.7 / 2.3 | 3.1 / 1.8 / 2.2 (scaled to 105) |
| Kickers and punters on scholarship, per team | 1.0 and 0.9 | (no data) |
| Walk-ons: K / P / LS | 68% / 45% / 100% | no recruiting record 69% / 65% / 82% |

Other FBS positions per 105-man roster (real in brackets): QB 5.2 (5.1), RB 7.6 (7.5), WR 14.3 (14.2), TE 6.0 (6.5), OT 8.2 (8.2), IOL 9.4 (8.9), EDGE 8.3 (8.0), DT 7.4 (7.4), LB 12.1 (12.8), CB 9.6 (8.9), S 9.7 (10.5). FCS: 7 of 127 teams have no punter (real about a quarter list none).

On the engine (last season, FBS, per team game): field goal tries 1.73 (real 1.56), made 82% (real 76%), two-point tries 0.20 (real 0.24), punt gross 44.4 yards (real 42.3). Field goals were kicked by kickers 70% and punters 30% in that run; since then a punter kicks only when his team has no kicker. Points run high on the engine at every level (a known gap), which lifts field goal tries. The kick accuracy that gives 82% was the same before this build.

Still off:
- Specialists drafted: 4.0 kickers, 3.3 punters and 1.3 long snappers a year (real 2.15, 1.96, 0.35, nflverse 2000-2025). One year with a thin class (the class-size waves, fix list 11.4) took 39 specialists; the other years took 2-9 in all (real about 4.5).
- Junior colleges fill about 53 of 80 places, and 28 of 69 have no punter.
- A walk-on never earns a scholarship (walk-ons, fix list topic 5).

The class sim in `career.rs` is unchanged (the calibrate output for seed 2026 matches the earlier code exactly).

Merged with the era limits build: class plans read `ncaa::roster_limits` (era limits, NCAA penalty cuts, FCS opt-outs), and the plan's yearly signing cap is that function's second number. The numbers above were measured before the merge, with 2025 limits in every year.

## Proposed, not confirmed (Claude's own picks in the build)

- Spin-up of 6 extra years before the 30-year history.
- Roster caps: FBS 105, FCS 100, D2 110, JUCO 80. Class sizes: FBS 28, FCS 24, D2 30, JUCO 40.
- Fix list topic 1 (2026-10-15): the long snapper rating (75% the snap at five points a step, 25% blocking and coverage); the splits inside the real OL, DL and DB counts (48% tackles, 52% edges, 46% corners); each staff's own habit for 3 or 4 kickers and 1 or 2 punters; the plan rule (this year's hole, or half the two-year hole rounded down); the draft bars for early entrants (the 64th and 138th best drafted overall, with the declare odds the draft uses); the talent-beats-need margin and cap; camp tiers 25/100/250/500, camp attendance 0.3 + 0.7 x family money, camp noise 1.5 points; the snapping camps rank each high school's best snapper who has no stars; the same specialist scholarship rule at every level; walk-ons get no NIL and pay tuition; the portal's open-spot rule (a starter left, nobody who started there is back, the entrant must beat the backups left); long snapper NIL, ageing and injury as kickers and punters; two at K, P and LS on the game-day chart; the holder (punter first, then the backup QB); NFL teams draft specialists only when their own is below par or aging, plus a flat discount (2 points for K and P, 5 for LS); the long snapper left out of NFL team strength.
- Playoff fields: FBS 12, FCS 16, D2 16, JUCO 4. High school: 8 teams per state classification.
- Firing patience by tenure: 10% in year one, 40% in year two, 75% in year three, full after that.
- Head coaches from FBS or FCS don't drop to high school jobs.
- 4% of recruits follow an older brother to his school.
- Placeholder JUCO programs in 12 states.
- A junior college makes at most 15 out-of-state offers a year (Mississippi and Kansas JUCO leagues cap out-of-state players; the exact number is a guess).
- No athletic scholarships: Pioneer League schools cost the family tuition, the Ivy League gives need-based aid only. Both lower a recruit's interest.
- After three seasons, a head coach under .400 over two years is much more likely to be fired, whatever the roster he inherited.
- Game stand-in: a college team 5 rating points better wins about 4 games in 5.
- Trial 1 list: each state's child population mix (no Census download was reachable); how often each group plays high school football; the hiring-bias size and fade; the strict admissions list (26 schools, 3 special admits each, Ivy 2); position draft values; arm and hand formulas; the Polynesian name lists; the overflow spill; the impact weights; prove-it numbers (60% chance plus excuse); position-coach pay-off weights in coaching (head coach 20%, coordinator 35%, position coach 30%, facilities 15%); staffs skip offering a recruit who already holds a clearly better offer; the hiring bias is flat to 2005, then eases, and also shapes which long-time assistant gets promoted when no candidate says yes.

## Open

- Realism questions 2-7 from the spec's open questions (asked one at a time in the build thread): classifications and playoffs, every college level (D3, NAIA), realignment, the coaching carousel, NFL franchise traits, shifting talent geography. Question 1 (all real programs) was asked first.
- Real towns for generated high schools would need a census town list; the data has each state's top hometowns only.
