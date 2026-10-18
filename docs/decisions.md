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
