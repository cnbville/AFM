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
- About 35-40 FM-style attributes (the full list still needs Raichel's sign-off).
- About 20 real depth-chart positions. Every player is rated at every position he could play, with fit derived from the attributes each position needs.

### Potential and development (2026-10-10)
- A hidden ceiling per attribute, but not a hard cap (Raichel hated FM strikers scoring 35 goals with finishing stuck at 11). Proven breakout seasons can lift a ceiling by up to ~10 points a year, at any age (less when older). Normal years move it up to ~5. The UI shows original vs boosted potential.
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

## Assumed, not yet confirmed
- **Stack: TypeScript** (Node, Vitest, no runtime dependencies). It's the working default but was never formally confirmed. Raichel weighed Godot or Unity early on, so the model stays pure logic with no UI coupling.
- **Season calendar defaults** in `docs/design/season-calendar-and-pacing.md`: player runs one pro team, save starts mid-February, real league structure with fictional names. All unconfirmed. Note the league sim decision above uses real team names.

## Still open, to talk through with Raichel
- **Next step: build the game itself.** The roadmap and the player's role in the game (GM, coach, owner) are still open.
- The league structure in detail (how many college programs, conferences, schedules).
- The first-year adjustment drivers (step size, position, polish, football IQ and make-up, physical readiness, fit and luck): proposed, not signed off.
- How strong the NIL pull back to school is, and whether a player knows roughly which team would draft him.
- The full attribute list.

## Trial history
- Trials 1-4: see `docs/design/player-model-decisions.md` and `docs/design/open-bugs-after-trial4.md`.
- Trial 6 (final, 20 players, 2026-10-11): same link as below, new version. Card data: `docs/trials/trial6-cards.json`, made by `scripts/trial6-cards.ts`.
- The 30-player trial (on the trial 4 code, before the coachability fix): https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP. The card data is `docs/trials/players30-final.json`. Raichel's review of it produced `docs/design/trial6-fix-list.md`.
