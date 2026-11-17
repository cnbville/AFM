# AFM decisions log

Everything Raichel (GitHub `cnbville`, the owner) has decided, with dates and reasons, plus what is still open. Detailed design notes are in `docs/design/`. If this file and a design note disagree, this file is newer.

## Goal
An American football management sim modelled on Football Manager. Priorities: realism, deep stats, dynamic potential, a patient season flow that doesn't rush climaxes, player cards, and high school prospects who develop realistically all the way to the NFL.

## How Raichel wants the work done
- **Ask before deciding (2026-10-10).** Every design or technical decision (stack, mechanics, rules, defaults) goes to Raichel as a short question with options and a recommendation, then wait. Raichel: "calm down with the freestyling... run aspects through me".
- **Realism first (2026-10-10).** Mechanics follow real football reasoning, not flat rules. Example: a CB moved to FS keeps value through speed, but an FS with poor run support is weaker at SS. Derive outcomes from the attributes that matter.
- **Dynamic, never static (2026-10-11).** Example: some rookies handle the NFL at once (Jaxon Smith-Njigba, Malik Nabers); some UDFAs thrive for specific reasons.
- **Explain every trial result.** Each notable outcome in a trial needs a "why", so Raichel can judge whether it's within reason. Sanity-check results for realism and contradictions before showing them (trial 3 had many).
- **Tuning by trials.** Raichel reviews sample careers and iterates.
- **Write plainly, keep token use low.** Short, simple sentences, split into labelled parts. One checker pass per trial, no big multi-agent fan-outs.

## Confirmed decisions
### Presentation (2026-10-10)
- Game day: text and stats as the main view, plus a 2D play view. Highlights styled like NFL Next Gen Stats (pass charts, route maps, run paths). The stat simulation decides plays; visuals only replay them, so the engine must output play-level spatial data.

### Ratings and attributes (2026-10-10)
- 1-99 absolute scale: 99 = an all-time great at his peak (prime Tom Brady). High school players dominate on lower absolute ratings.
- Cards also show a level-relative rating (e.g. "99 among HS juniors"). A level-relative 99 in HS or college must be rare. Benchmark: EA College Football has 0-1 players at 99, about 10 at 95+, about 100 at 90+ of ~11,000.
- The level rating ranks against the last five seasons together, not the current season alone, so a strong year shows more 90+ players and a weak year fewer (Cain, 2026-10-24: "yes"). Whether its rarity should follow real honours (5-stars, All-America teams) instead of EA's counts is asked, not answered yet.
- About 35-40 FM-style attributes (2026-10-10). Raichel found the 38 built in section 3 too thin and agreed on 2026-10-18 (thread "Full build and trial 7") to 67 visible attributes and 5 hidden ones, then an engine retune on them; tendencies become their own layer after, with a short design round. The list: `docs/design/attributes.md`. The tendencies round ran the same evening: `docs/design/tendencies.md` (topics 1-4 and six coaching styles, all confirmed).
- About 20 real depth-chart positions. Every player is rated at every position he could play, with fit derived from the attributes each position needs.

### Potential and development (2026-10-10)
- A hidden ceiling per attribute, but not a hard cap (Raichel hated FM strikers scoring 35 goals with finishing stuck at 11). Proven breakout seasons can lift a ceiling by up to ~10 points a year, at any age (less when older). Normal years move it up to ~5. Original vs boosted potential is shown on trial and debug pages only, never in the game (2026-10-11, roadmap Q46).
- Development is volatile, not linear: sophomore slumps, true freshmen jumping, redshirts, single big-jump seasons. Slumps lower production and, slightly, rating.
- Reaching potential: roughly a third within 1-2 points, most at 90-95%, busts well short.

### Stars and busts
- 2026-10-10/11: a five-star must have dominated high school.
- Five-star busts usually start after signing: injury, work ethic, off-field trouble.
- Early bloomers who stop growing are an allowed bust cause (2026-10-11, card 2).
- **Revised 2026-10-11:** scouts misjudging a player is also a bust cause, but a small one. Raichel: "plenty of hard-working QBs don't make it to the NFL". FBS staffs make errors too.

### NFL entry and the draft
- Rating tiers are only guidance (about 65 roster, 75 starter, 85 Pro Bowl, 92+ elite) and are checked against test data.
- Teams draft imperfectly: each team's drafting skill is tied to its GM and owner, so bad teams draft players they shouldn't.
- D2 and below are rarely or never drafted (mostly UDFA), and at most on day 3.
- Benchmarks: non-D1 picks about 4.4 a year (2010-25), 83% on day 3. FCS direct picks about 5.8% of picks. On3 five-star draft rate about 63%.
- Real draft round ends (2025): 32, 64, 102, 138, 176, 216, 257.

### Trial 5 and 6 decisions (2026-10-11)
- Calls 1-7 from trial 4 (`docs/design/player-model-decisions.md`): all answered A, with Raichel's extra context.
- Coachability bug ("Fix both"): fixed and in the code.
- League sim: generated players with real team names. Rosters are filled by a warm-up of about 15 seasons, so every player has a full sim-grown history.
- Trial 6 fixes: see `docs/design/trial6-fix-list.md`. Raichel said these are the **final fixes before building the game**; later issues get debugged in the game.
- **Film line: option C, confirmed by Raichel.** Three-stars need a top 2,700 senior film rank, two-stars top 6,500, FBS offers below three stars top 4,500 (`PIPELINE.starFilmRank`, `PIPELINE.fbsOfferMaxFilmRank`).
- Trial 6 was built and run (20 random but realistic careers, each with a "why" box). Raichel finds calls and bugs from here on in gameplay, not in more trials.

### Trial 6 details Claude picked (Raichel said: pick the most realistic option)
- First-year dip (0-1) on every step up, from step size, position, technique polish, football IQ and make-up, physical readiness, scheme fit and luck. A full dip costs about 7 points of on-field play that year.
- High school growth lags on skills (technique 0.65, mental 0.7, strength 0.6 of full speed); college teaching catches it up (non-physical growth x1.3). Freshmen rarely beat upperclassmen: a blocking penalty of 10 points in year one, 5 in year two, scaled by program prestige.
- Portal: backups enter about 18% a year, rotation players 7%, more after a coaching change. Outcomes are up, sideways, down or no school.
- Dismissals: 35% are serious (drop a level or quit, 45% quit). Lesser causes move a good player sideways to a lower-profile FBS school (10% quit).
- Declaring early: projection plus urgency (NIL at small schools, RB shelf life, fear of falling, age, injury, coaching change, lost starting job) against staying (big NIL money, room to rise, a bad final year, a bad likely drafting team).
- Draft value by position (QB +3.5, EDGE and OT +1.5, CB +1, WR and DT +0.5, IOL and S -0.8, LB and TE -1, RB -3.5) plus team need; a team with a young franchise QB won't take another early.
- Kickers and punters: a separate board from round 5 (round 4 at 30%), 15% chance a team takes one, and only if clearly better than its current one.
- League: the 32 real team names with random GM, owner and coaching (1-20). Other classes are background players with a rating and age (starters about 78, rotation 73, backups 69, better on well-run teams).
- Camp: real depth chart slots (53 plus a 16-man practice squad), camp bodies from other classes, rookie protection for high picks (round 1 +9 in the first seasons, fading), incumbency, and a lean towards youth (not for kickers and punters). Years of NFL film reduce misjudgment.
- Practice squad: players with at most two roster seasons, under 27, at most three squad years. Released players under 30 can be claimed on waivers (60%); blocked starters (78+) can be traded; injured teams sign street free agents mid-season (15%).
- Landing spot: a "situation" score from the roster around him and how well the team is run, doubled for players 25 and under. It moves performance and potential growth.
- Retirement by position (QB 35, RB 28, WR, LB, CB, S, EDGE 30, TE and DT 31, OT and IOL 32, K and P 37), moved by longevity, love of the game (new trait) and wear. Running QBs (speed, acceleration, agility average 78+) retire 4 years earlier. Being good keeps a player going only a few years past his usual age.
- Known gap: FCS players are about 2-3% of draft picks (real about 6%).

### Platform (2026-10-11)
- **A desktop app with an engine in Rust.** Raichel picked option B: the game must run the densest, most realistic simulation (every high school, college and NFL game, 50+ year careers), so the engine is a fast native library on all processor cores. The player model was ported from TypeScript to Rust right away; the TypeScript stays in `legacy-ts/` for reference.
- **Potential stays hidden in the game (2026-10-11).** The player-facing side never sees an exact ceiling: only a vague tier ("could become a starter") plus small hints, both sharper and more reliable with better scouts. Exact potential is for trial and debug pages only.

### Rust port details Claude picked (2026-10-11, the coordinator's brief said to pick and list them)
- Layout: a Cargo workspace. `crates/afm-engine` is the engine library a desktop UI will call later; `crates/afm-tools` holds calibration and trial tooling. No UI or app shell yet.
- Hidden potential is enforced in code: ceilings and true potential are private to the engine. Only the `debug` cargo feature (used by afm-tools) can read them. The game must never turn it on.
- New player-facing read: `potential_outlook` gives a tier in words (high school only, college, NFL roster, NFL starter, Pro Bowl, elite), how sure the staff is, and hints (late or early bloomer, body done growing, work ethic, coachability). Hints only come from scouts rated 13+, more and truer the better the scout. A placeholder until the scouting design is signed off.
- Speed: each player has his own random stream, so per-player work runs on all cores and a seed gives the same class on any machine. Class-wide decisions use the class's own stream. A 250,000-player class takes about 2 seconds (TypeScript: about 57).
- Same random generator as TypeScript (sfc32, same seeding), but the per-player streams mean a seed gives a different class than in TypeScript. Results match statistically (see the PR for the side-by-side).
- One test band widened: veterans' leftover upside (median potential minus rating at 33-35) must stay under 1.5 instead of 1. TypeScript itself gave 0.61-1.06 over six seeds; Rust gives 0.63-1.27 over nine.
- Dependencies: rayon (multi-core) in the engine; serde and serde_json only in the tooling.
- Not ported: the trial 6 card script (`legacy-ts/scripts/trial6-cards.ts` still runs). Port it when the next trial needs it.

### Roadmap (2026-10-11)
- All 50 roadmap answers, the build order and the sections are in `docs/roadmap.md`. Section 2 design is in `docs/design/world.md`.
- They settle the season calendar doc's old defaults: you are GM and head coach of one NFL team (college coaching later; since 2026-10-18 a career can start at a high school, a college or an NFL team, see `docs/handoff/decisions.md` section 12), a save starts mid-February, and names are real with a fictional toggle at new game.

### Section 3 build: athlete-first details Claude picked (2026-10-13, proposed, not confirmed)
The confirmed rule (topic 5.2) is "build the athlete first, the coach places him, real totals still hold". These numbers are Claude's calibration picks for it:
- Body: 29% lineman frames; skill leanings mapped so a two-standard-deviation gift reaches the mid 90s; the kicking leg shifted 0.4 standard deviations rarer.
- Natural spot: chosen with per-position comparison factors set once from 40,000 sample athletes so natural spots match real high school roster shares; the natural spot rates at least 5% above any other.
- Placement: best athletes first; an open roster spot adds 8% of fit, each player beyond the need costs 12%; within a family the harder spot adds 1.5% (tackle, edge) or 1% (corner). Kickers are never converted to or from late.
- Coach eye: insight 25-75% and misread 7 to 2 points per attribute, by coach skill 0-20; misreads shrink 10% a year of film (to half). NFL scouts: 65% insight, 2.5 points.
- Re-placement margins: high school 3% better fit, college at signing and each spring 1.5%, pro scouts 0.5%.
- Talent tail refit from 2.6 to 2.75: misplacement costs top players a little, and the top of the NFL needed it back (90+ seasons averaged 0.8% over four classes before the refit, 1.7% before athlete-first).
- Two single-class test bands widened, with the seed spread as evidence: five-star floor 86 to 85 among seniors (the rule is top 100 on film), and the 90+ pro share floor 0.5% to 0.2% (0.3% to 2.6% across eight classes, old and new model).
- Measured on the reference class: 14% of players change position at least once, 6% of draftees play a different position than as freshmen. Real rates are not in the dataset yet; a trial should check them.

### Section 3 build: match engine picks Claude made (2026-10-14, proposed, not confirmed)
The full list with the numbers is in `docs/match-engine.md` ("Choices Claude made"). In short: the sprint model from the 40 time; acceleration and agility gaps narrow below 60; throw error about 10% wider per 20 accuracy points; the QB leads a receiver by how he really runs; carriers at 95% speed while reading blocks; blocks in space can whiff; tempo 37.5 seconds; a fourth-down success curve; screen timing and the screen "smell"; contain on kick coverage; goal-line coverage compression. Known gaps (points, pass rate, sacks, NFL run length, red zone, returns, high school play count) are listed there as trial candidates.

### World trial 3 fix list picks (2026-10-14, proposed, not confirmed)
Listed in `docs/world-model.md` ("Trial 3 fix list"): the NIL position values and offer formula, the recruiting bond, facility steps, hiring read noise and age mark-downs, the bias fade, the bench refit, the leg-body lean for kickers, the name reweighting, and team schemes picked from the coach. Confirmed by Raichel 2026-10-14: every college game runs on the engine, history included (30 years in the game, 15 for trial 3).

### Progression update steps 1-2 (built 2026-11-15)
Built from `docs/design/progression.md` and `progression-everyday.md`, both agreed by Cain on 2026-11-14/15; how it works in `docs/reference/how-it-works/progression.md`. Cain's own picks in the build: AI staffs' project lean is the coach's own, moved by his seat (decision card, 2026-11-15), and the 12-point yearly growth cap is dropped (physical study item 16, 2026-11-15). The limit's read takes off the class's drift by age, so only a player's own growth past his class lifts it (the agreed rank, item 4, worked out with the physical study thread). Claude's picks, proposed, not confirmed:
- The passing mechanics family is footwork only.
- The makeup part of the read uses makeup ceilings, not today's ratings, so a boy's young reads don't hold his limit down.
- The leans' shares: 3 in 10 developers, 4 balanced, 3 win-now (the prototype's).
- High school staffs' time is 0.3 + 0.5 × resources.
- NFL clubs run projects with a balanced lean.
- A suspended or dismissed year closes no room.
- The class's drift is fitted with a coach of 10 and rotation snaps.
- The trims are the design's class-sim values until step 5 sizes them after the physical items. With them, today's 80+ counts run about 2-3 times higher on the class sim (corners 11 to 29 a class).

### Progression update step 3 (built 2026-11-15)
Cain's picks (18:29 UTC placement, physical study answers): Strength split into upper body and Lower Body, tied like real (item 4b); the bench at the real slope plus a fixed lifting knack, which fixes minor bug 385 through the slope rather than faster strength (item 4); freaks from the body knack with no wider leash (item 5); the normal weight room inside the leashes by the body knack (everyday item 6). How it works: `docs/reference/how-it-works/body.md` and `match-engine.md`, "Upper and lower body". Claude's picks, proposed, not confirmed:
- The tie: the legs are 0.75 times the upper body's draw plus his own; an old save's legs equal his Strength.
- Which engine reads take which half (the sort on `match-engine.md`), and each position's split of the old strength weight in the overall (0.4 to 0.6 legs).
- The within-position bench slope 0.40 reps a point, set so the drafted spread matches the real one on average; each position's offset so its drafted mean is the real one; the lifting knack sd 1.5 reps; the squat and clean without the knack.
- The weight room's response 0.8 × body knack z, bounded at a whole leash under the typical path and the leash's most over it, front-loaded 0.30/0.25/0.20/0.15/0.10 at 18-22 (burst only at 18-19).
- Measured with `bench_check`: the college bench gain is +23-29% at the skill spots (real +34%) and +27-31% on the lines (real +18%), and 35+ reps come 0.8 a draft (real 3.4) with the 99 wall still on strength. The wall comes off in step 4 (item 1), and the counts get measured again then.

### Progression update step 3c, body projects (built 2026-11-16)
Design items 1, 2, 7, 8 and 11 (`docs/design/progression.md`): a strength or speed project on its own leash, the body knack deciding who responds, the frame deciding how far, running beside a skill project at the price of wear, limited by the strength staff, on the body's clock. How it works: `docs/reference/how-it-works/progression.md`, "Body projects". Claude's picks, proposed, not confirmed:
- The leash share Φ(1.2 × body knack z) (half the leash for a typical responder, nearly all for a big one) and the frame's room 0.6 to 1.05 from height, frame weight and arms against the position's combine sizes.
- A year closes 0.45 of what's left of the leash, by the strength staff and U(0.7, 1.3); 0.25 wear a year.
- Pounds follow the strength built: 1.2 lb a point on a 190 lb frame, scaled by frame weight, at most 25.
- AI staffs: one body project a room at a time in college and the NFL, on the man whose strength trails the typical FBS starter at his spot most (a read of 8+); none run by AI high school staffs.

### Progression update step 4a, bodies at creation (built 2026-11-16)
Physical study items 9-15 (Cain 2026-11-15: realism by default): no creation limits inside what people are, heavy linemen at the real rate, real arm lengths and the arm-height tie, real hands, receivers' heights, defensive backs' and receivers' weights. How it works: `docs/reference/how-it-works/body.md`, "How a body is created". Measured with `body_sizes` (eight 250,000 classes, 2,056 drafted men, against the combine 2000-2026 and MockDraftable 2005-2025). Claude's picks, proposed, not confirmed:
- Arms, hands and wingspans by each spot's real drafted mean and spread; arms 0.20 in an inch of height for linemen and edges, 0.37 for everyone else (the study's real 0.16-0.23 and 0.33-0.41).
- Each spot's size fit after his spot is chosen (`SIZE_FIT`): inches, an extra height spread, pounds, and a weight spread scale, fitted so drafted means and spreads land on real.
- A tackle's, guard's or defensive tackle's heavy side 1.6 times as spread as the combine's, applied after his spot is chosen.
- A high school coach's pull to an open spot grows with how open it is (0.08 a boy, at most 0.08) and the push starts at the first boy over (0.12 a boy): the old flat pull to half a boy open, with no push until half a boy over, rounded each spot to whole boys in a class of 18, which put tackles at two a class (11.8-12.4% of freshmen, real 8.5%) and guards and centers at two of 2.2.
- The weight room now pulls a frame 60% of the way to his position's weight (the share a position move already took), not all the way: before, the light half of every position was gone by the draft.
- Creation bounds 58-88 in and 120-480 lb.
- The bench's per-position offsets refitted with the real arms (longer arms bench fewer reps at the same strength).
Measured (eight classes, 2,056 drafted): every spot's mean height within 0.3 in and weight within 4 lb of the combine, arms, hands and wingspans on real; freshmen by position within the test's band (tackles 10.9%, real 8.5%); `match_check` NFL 25.0 points a team game (24.4 before; the synthetic rosters' longer arms), FBS 23.7; `freak_check 10 86-95` on its last measure (top 1% of edge seasons 19.5 sacks, 20+ sack seasons 9.8 a 1,000, before 9.4). Still off: guards and centers 0.75 in short (tall ones move to tackle as they grow), receivers 6'4 or taller 8% (real 10%) and 5'8 or shorter 0.7% (real 3.7%), FBS and FCS men listed 350+ 0.14% (real 0.41%) and drafted tackles' heaviest 1% 346 lb (real 365), and the class sim drafts tackles at 12% of picks (real 8%; it drafts by overall with no position value).

### Progression update step 4b, the drills (built 2026-11-16)
Physical study items 5-8 (Cain 2026-11-15: realism by default; the legs in the jumps). How it works: `docs/reference/how-it-works/body.md`, "Combine drills". Measured with the new `drill_tails` (eight 250,000 classes, 2,056 drafted men, one test day each, against the nflverse combine's drafted men 2000-2025). Claude's picks, proposed, not confirmed:
- The jumps read 0.8 his spring (Jumping, or his burst when more) and 0.2 his lower body.
- A jump knack of his own for life, one spring for both jumps (they go together 0.7), drawn with real's fat tail (a t of 5 degrees), sized by position to the part of real's spread the ratings leave.
- Each position's offsets on the vertical, broad, 3-cone and shuttle, so drafted averages are real's.
- The 3-cone and shuttle's fast end thins past 0.9 sd under the position's real mean and never reaches past 1.9 sd more.
- No printed drill limits, only bounds no man reaches.
Measured: every position's vertical, broad, 3-cone and shuttle mean within 0.3 in and 0.01 s of real, spreads within 0.9 in and 0.03 s; a draft brings 0.38 3-cones of 6.50 or faster (real 0.35 of the men who ran it; 4.1 before), 0.75 shuttles of 3.85 (real 0.58; 3.75), 2.6 verticals of 42 in (real 2.3; 0.6) and 0.5 of 44 (real 0.35; none), 0.38 broads of 140 in (real 0.15); the best 3-cone 6.36 and shuttle 3.78 (11 and 18 men had sat on the printed 6.40 and 3.80). Still open for step 4c: the forty's fast tail (3.1 a draft at 4.30 or faster, real 1.3) and the bench's fat top (35+ reps 0.6 a draft, real 3.5), both with the wall.

### Progression update step 4b part 2, the 10-yard split (built 2026-11-16)
Physical study item 2 (Cain 2026-11-15: "make sure you look at 10-y splits to, especially with big man that curve is imporant"). How it works: `docs/reference/how-it-works/body.md`, "Speed and size in the match engine". Measured with the new `split_check` (four 250,000 classes, 1,013 drafted men, their engine run and one test day each, against MockDraftable's electronic splits of drafted men 2005-2025).
- Before: the test split was 0.375 × forty − 0.15, apart from the engine's run. In the engine, men at the same forty and weight split 0.007-0.010 s apart (real 0.04), because speed and acceleration share one draw in the generator; the engine's splits ran 0.02-0.05 s quick.
- Now one sprint for both: the test day's split is his engine run at that day's forty plus 0.02 s of laser timing.
- A first-step gift of his own for life on his sprint's time constant (sd 0.08, about 0.036 s of split), and acceleration's pull on the start halved. Cain chose this, option A, on 2026-11-16. Option B (acceleration's own draw) reached the spread but its big-man curve ran 0.42-0.61 (real 0.28).
- Top speed reads 0.92 of the time constant (it was all of it plus 0.06 s), so a quick or slow starter runs his rating's forty in the engine. A man who runs a 4.30 can't start slowly: the constant is held so top speed stays under 11.8 yd/s.
- The edge's rush step leaves the gift out (minor bug 286). High school laser splits carry the gift, and a staff reads a split as 0.047 s wide.
Measured: every weight band's split within 0.01 s of real (1.55 under 200 lb to 1.81 over 315), the test split 0.343-0.345 of the forty (real 0.344-0.346), 0.037-0.042 s apart at the same forty (real 0.04), the split moving 0.29-0.34 s a second of forty (real big men 0.28). Aaron Donald's 1.59 with a 4.68 at 285 lb: the ratings alone give about 1.61 there, so it takes a first step 0.6 sd above the usual (before, 0.01 s of spread made it a 2 sd outlier).
What moved with it (match_check 400 games, freak_check seeds 86-95): NFL points 25.0 to 24.5 a team game (real 22.5), sacks taken 2.14 to 2.00, pressure rate 0.240 to 0.238; FBS points 23.7 to 23.6. Edges' pressures a snap 4.71% to 4.40% (real 3.44%), the top 1% of sack seasons 19.5 to 18.5 (real 17.5), 20+ sack seasons 9.8 to 6.5 a 1,000 (real 2.0). Why: the engine's starts were quicker than real before.
Left as it was: an interior rusher's heft counts 0.64 Acceleration points a pound (`game/sim.rs`, minor bug 285), set from the old split fit; the new split gives about 0.72. It was tuned on pressures, so it waits for the engine audit.

### Progression update step 4c, no 99 wall (built 2026-11-16)
Physical study items 1, 3, 17, 18 and 21 (Cain 2026-11-15: yes to 17 and 18, realism by default on 21; screens keep 1-99 and show "99+", item 19, step 6). How it works: `docs/reference/how-it-works/ratings.md`, "Past 99"; the bench and the weight room in `body.md`. Measured with the new `top_check` (eight 250,000 classes, 2,056 drafted men) and `drill_tails` (eight and 24 classes). Claude's picks, proposed, not confirmed:
- No 99 wall anywhere a rating or ceiling is written (bound 150, which no rating reaches).
- A thin top on every rating but the strength pair: past 95 a value counts for less on a log curve with no top (95 + 3 ln(1 + (v − 95)/3)), at creation and on every rise (growth, results, the weight room, body projects, backgrounds), and on the split ceiling's limit. With the wall off and nothing else the ceilings ran to 110-150 (power rush 150, through the limit's read of strength); a tanh top (a hard top at knee + room) only made a new wall at 102.
- Sized on item 17's anchor, "a 100+ arm is about as rare as a 4.25 forty" (real 0.19 a draft): a drafted man's key skill at 100+ now 0 to 0.9 a draft (arm 0, man coverage 0, pass blocking 0, route running 0.25, press 0.25, power rush 0.9), best 100-103.
- Item 18 measured: potential 99+ none in 8 drafts, best 97.5 (0.12 a draft and 99.2 before overalls read the strength pair on the thin top); overall at the draft best 91.3. The study's "about 0.6%" was the wall off with no thin top; item 17's thin top makes it rarer.
- The 94 rule is gone (item 21): an elite's key skills spread like anyone's (8.1 at potential 90+ against 9.4-9.9 below; 6.3 with the rule), and creation shifts his skills so his overall after the thin top is still his talent (potential 90+ 9.9 a draft, 9.75 with the rule; 8.75 once overalls read the strength pair on the thin top, since the weight room's strength past 95 counts for less).
- The bench reads body weight (item 3): a strength point is worth 0.35 × weight / 249 reps (Epley: reps follow the pounds on the bar), so big men's spread is wider and the skill spots' narrower, as real; centers and offsets refitted on sixteen classes.
- Upper-body strength's weight room response is right-skewed (r = 0.8 z + 0.3 (max(z, 0)² − 0.5)) with no most, so big responders make the 40-rep benchers (item 3: the fat top mostly from the body knack). Lower body keeps the plain response: skewed, it ran verticals to 56 inches.
- An overall reads strength and lower body past 95 on the thin top, though the ratings themselves keep their fat top: without it, linemen's overalls rose and one class drafted 19% tackles (real 8.1%); with it 13.0% over 8 classes, against 12.8% with the wall (the class sim's own lineman overdraft is bug 402; `national_class` gives tackles a band of 2.0 times real until it is fixed).
- The jump knack is a plain normal (it was a t of 5 degrees from step 4b): over 24 drafts the t gave a 52-inch vertical and 1.4 a draft at 44+ (real best 46.5 in 25 years, 0.35 a draft).
Measured: bench 35+ 4.7 a draft over 24 drafts (real 3.5), 40+ 0.67 (real 0.77), best 51 (real 51; before step 4c 0.8, none and 41), each position's spread 3.9-5.2 reps (real 4.0-5.5); vertical 44+ 0.5 a draft, best 47.5. NFL points 24.5 to 24.3 a team game, sacks taken 2.00 to 1.96, FBS 23.6 the same (match_check 400); edges' pressures a snap 4.40% to 4.41%, the top 1% of sack seasons 18.5 the same, 20+ sack seasons 6.5 to 7.2 a 1,000 (band 4.4-10.5; freak_check seeds 86-95). The best sack season was 32 before and after (real 22.5); that waits for the engine audit.
focus_check is the same as main (year one +4.13 with no focus, +1.75 at 30 minutes, +3.42 at 60). Chemistry's completion gap now depends more on the roster (8 leagues: 2.0 to 3.5 points with the wall, -0.3 to 4.1 without, mean 2.1), so its test reads four leagues.
Still open: the forty's fast tail (4.30 or faster 3.4-4.3 a draft, real 1.3; 4.25 0.5-1.3, real 0.19) is not the wall: the fast men are already in college. Cain 2026-11-16: "both", a separate speed lane for the cause and, with his engine update, route running and hands that pay off on the field.

### Progression update step 5, the star count (built 2026-11-17)
Everyday item 11 (one trim per family so the world keeps today's number of stars). Measured first: no trim could hold it. With every part on and the design's flat trims, tackles' 70+ pass blockers ran 87 to 176 over two classes (80+ 8 to 31), and corners' man coverage and backs' leg drive about doubled too; the strongest trims that kept the order of ceilings still left them about 1.7 times today. The cause: men close almost all their room in college (a typical 80+ tackle entered college at a 76 pass blocking ceiling with a 92 limit and left at 92), and everyday coaching, projects and breakouts each add stars on their own; body projects add none. Cain 2026-11-17 (the card): "Slow closing near the top". How it works: `docs/reference/how-it-works/progression.md`, "Everyday closing" and "The trims". Claude's picks, proposed, not confirmed:
- Closing slows past a ceiling of 55: a year closes 1 / (1 + (ceiling − 55) / 5) of what it would, so a 60 at half pace, a 70 at a quarter, an 80 at a sixth, never zero (`near_top`). Everyday closing, breakouts and project years (clicks too) take it; the weight room and natural rises don't. Starting at 75 did nothing for the 70+ count (most of the extra 70s close from below 75), and from 50 to 60 and spans of 5 to 10 all left the 80+ count about a sixth low with growing trims.
- Each family's trim has two slopes from natural ceilings of 60 and 75 (`trim_at`): the second most often eases the trim off, so the 65-80 band gives up the most and most families' natural 85s little or nothing. Sizes at a natural 70: tackles' pass blocking 5.9, backs' leg drive 5.3, receivers' routes 3.2, corners' man coverage 2.1, edges' finesse rush 0.9, linebackers' pursuit 0.1. A higher natural ceiling always keeps a higher trimmed one. The limit keeps the trimmed points.
- Backs' pass blocking keeps a trim though it isn't one of the backs' key skills (the design had one); the fit sizes every family that has a trim.
Measured on two fresh classes (seeds fitC and fitD, `prog_check`, every part on): collegians reaching 70 in a key family 2,805 today to 2,888 (+3%), 80 378 to 348 (−8%); single families move by about their noise (tackles' pass blocking 84 to 83 and 10 to 7, corners' man coverage 191 to 201 and 27 to 19). The elite-tools, bad-technique men still gain most where they have the knack (corners' man coverage 48.9 to 56.0 at their best college season, against 51.9 to 53.6 with no knack). focus_check: year one +4.02 with no focus, +1.71 extra at 30 minutes, +3.33 at 60 (main +4.13, +1.75, +3.42).

## Assumed, not yet confirmed
- The rest of `docs/design/season-calendar-and-pacing.md` (week-by-week calendar, tension tiers, climax budget) is a draft for section 4.

## Still open, to talk through with Raichel
- **Next step: section 2, the world** (`docs/design/world.md`). Question 50 is answered: playable early with plain screens, then depth.
- The league structure in detail (how many college programs, conferences, schedules): part of section 2.
- The first-year adjustment drivers (step size, position, polish, football IQ and make-up, physical readiness, fit and luck): proposed, not signed off.
- How strong the NIL pull back to school is, and whether a player knows roughly which team would draft him.

## Trial history
- Trials 1-4: see `docs/design/player-model-decisions.md` and `docs/design/open-bugs-after-trial4.md`.
- Trial 6 (final, 20 players, 2026-10-11): same link as below, new version. Card data: `docs/trials/trial6-cards.json`, made by `legacy-ts/scripts/trial6-cards.ts`.
- The 30-player trial (on the trial 4 code, before the coachability fix): https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP. The card data is `docs/trials/players30-final.json`. Raichel's review of it produced `docs/design/trial6-fix-list.md`.
