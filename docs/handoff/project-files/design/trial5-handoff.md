# Trial 5 handoff

Written 2026-10-11 by the player model thread. Raichel asked for trial 5 to run in a new thread to save tokens. Raichel also wants fewer agents: use one checker, not a fan-out.

## Code state
- Repo cnbville/AFM, branch `claude/project-thread-yn5oqf`. Commit `0ab7c94` holds all of trial 4, the bug fixes after it, and the coachability fix.
- The commit is **local only and not pushed**. Raichel was told nothing gets pushed until they are happy with the trials, so don't push without their word.
- Backup bundle: `/mnt/project-files/code-backups/afm-trial5-start.bundle` (all refs, verified). To restore it:
  `git fetch /mnt/project-files/code-backups/afm-trial5-start.bundle claude/project-thread-yn5oqf && git checkout -B claude/project-thread-yn5oqf FETCH_HEAD`
- Typecheck passes. Tests: 30 of 31 pass. The one failure is the five-star draft rate (0.879, bound 0.85). It belongs to call 1, so leave it until call 1 is built.
- Fast sims: `node_modules/.bin/esbuild x.ts --bundle --platform=node --format=esm --outfile=x.mjs --log-level=error && node x.mjs`. A class of 250,000 takes about a minute, so run at most 2 at once on 4 CPUs.
- The calibration table in `docs/development-model.md` was measured before the coachability fix. Re-measure it before quoting those numbers.

## Coachability fix (done, approved by Raichel: "Fix both")
- **Bug:** `src/development.ts` multiplied coaching by coachability above average and divided by it below average. So under coaching below 10 (most high schools), the least coachable players grew fastest.
- **Fix:** `coachingBoost = 1 + (coachingMult - 1) * coachabilityLean`. Coachability now only scales how far coaching moves growth from average, good or bad.
- **High school gain under coaching below 10** (seed trial-4):
  - before: 11.4 points for coachability 1–6, 9.3 for 14–20;
  - after: 10.2 for both.
- **High school gain under coaching of 10 or more:** after the fix, 11.5 for coachable players against 11.0 for uncoachable ones.
- **Four- and five-stars combined:** now even across coachability levels.
- **Five-stars alone:** still 8 from coachability 1–6 against 3 from 14–20 on this one seed. It may be noise, since there are only about 32 five-stars a class. Check it on more seeds before calling it fixed.
- **Not changed:** the coachability × coaching term in potential drift (around line 207) still makes bad coaching hurt uncoachable players less. On balance coaching still hurts them, so this fits the rule.

## What the 30-player trial found
- **Page:** https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP (version 6). It has 15 careers picked by route and 15 drawn at random from seed trial-4. Every card has a story, "why" bullets and flags naming a call.
- **Card data:** `/mnt/project-files/design/players30-final.json`.
- **Flags by call:**
  - call 3 (pro careers) on 11 cards;
  - the new bugs on 6 cards;
  - call 1 on 5, call 2 on 4, calls 4 and 5 on 2 each;
  - 6 cards had nothing to change.
- **Class-wide findings:**
  - 4–7 of 64 first- and second-round picks, and 6–14 of 38 third-rounders, are cut within two seasons (call 3).
  - Card 9 is the FCS edge Raichel was suspicious of (college level 81, 85 by the end of the year, then a 72 rookie). Call 1's dynamic first-year adjustment covers it.

## Open: the film line for stars and FBS offers (Raichel chose "Fix both")
- Raichel approved fixing it, but Claude promised to **bring film line options with numbers before changing anything**. The new thread owns this.
- **The bug:** three-stars, two-stars and FBS offers have no film requirement.
  - The star score is 25% film + 75% projection. FBS offers to two-stars and unranked seniors score 40% film + 60% projection.
  - About half of three-stars come from outside the top ~5,700 seniors, and about 790 from outside the top 10,000.
  - FBS signs about 850 players a class from outside the top ~13,800 seniors.
  - Card 28 got three stars and an FBS scholarship with 67,114 seniors ahead of him. Card 10, ranked 508th, got no FBS offer.
- **Current settings** in `src/career.ts` PIPELINE:
  - star shares per class of 250,000: about 32 five-stars, 350 four-stars, 2,000 three-stars and 4,000 two-stars;
  - film floors: `starFilmRank { maxFor5 100, maxFor4 1000, minFor4 150, minFor3 800, minFor2 3000 }`;
  - signings: FBS about 3,250, FCS about 2,500 and D2 about 11,250 a class.
- **What to measure** for each option (e.g. three-stars only from the top N on film, FBS offers only from the top M), on two seeds:
  - where three-stars and FBS signees rank among seniors;
  - the share of draft picks who were two-stars or unranked (sim 16%, real about 19%);
  - what happens to late bloomers (cards 7 and 9 went small school and then moved up or got drafted).
- Present the options to Raichel with a recommendation and wait for the pick.

## Open: the league sim (Raichel chose it for trial 5)
- Raichel picked "League sim" on the decision card: 32 NFL teams and the college programs with full rosters, and every class simulated together.
- Claude promised to **talk through how the league works before building any of it**.
- Proposals for calls 1, 2, 3 and 6 and the league are in `/mnt/project-files/design/trial5-talk.md`.
- Still unanswered:
  - Does the driver list for the first-year adjustment look right? (step size, position, polish against rawness, football IQ and make-up, physical readiness, fit and luck)
  - How strong should the NIL pull back to school be?
  - Should a player know roughly which team would draft him?

## How Raichel wants Claude to work
- Ask before every design decision, with short options and a ★ recommendation, and wait for the answer.
- Everything must be realistic, reasoned the way real football works.
- Every trial card explains why things happened, so Raichel can judge whether it's within reason.
- Sanity-check trial output for realism and contradictions before showing it.
- Memory: `afm-trial5-answers.md`, `afm-trial30-results.md`, `afm-trial2-unpushed.md` and `afm-trial4-calls.md` in the team memory.
