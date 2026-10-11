# AFM: American Football Manager

A deep, realistic American football management sim modelled on Football Manager: high school prospects develop through college to the NFL, with dynamic potential and a patient season flow. Owner: Raichel (GitHub `cnbville`).

## Read first
1. `docs/decisions.md`: every decision so far, what's assumed and what's open.
2. `docs/design/trial6-fix-list.md`: **the next job.**
3. `docs/development-model.md`: how the current player model works.

## Working rules (from Raichel)
- **Ask before every design decision.** Send a short question with options and your recommendation, then wait for the answer. Never pick defaults yourself.
- **Everything realistic and dynamic.** Reason the way real football works, and derive outcomes from attributes, not flat rules.
- **Every trial result explains why.** Each notable outcome gets a reason, so Raichel can judge it. Sanity-check results for contradictions before showing them.
- **Keep token use low and write plainly.** Use short sentences and simple structure. One checker pass per trial, no big agent fan-outs.
- **Don't push or merge without Raichel's OK.**

## Next job: trial 6
- Build the seven fixes in `docs/design/trial6-fix-list.md`. Raichel says these are the last fixes before building the game itself.
- **Walk through the list with Raichel before building.** Several items need details agreed first (league structure, first-year adjustment drivers, NIL pull).
- The film line is assumed to be **option C** (2700 / 6500 / 4500, see `docs/decisions.md`). Raichel hasn't confirmed it explicitly, so confirm before switching it on.
- The trial needs a league sim with generated players on real team names (decided). Today the sim runs one class on its own, with no teams.

## Code
- TypeScript, Node 22, Vitest, no runtime dependencies. Pure logic, no UI.
- `src/attributes.ts`, `positions.ts`, `player.ts`: attributes, position fit, player generation.
- `src/development.ts`: the yearly development step (growth, slumps, breakouts, coaching × coachability).
- `src/scouting.ts`: noisy scouting reports.
- `src/levelRating.ts`: level-relative ratings ("89 among seniors").
- `src/career.ts`: simulates a whole national class (250,000) from age 14 through high school, recruiting, college, the draft and pro careers. The tuning knobs are in `PIPELINE`.
- `scripts/calibrate.ts`: prints headline stats for one class.
- `docs/trials/players30-final.json`: the 30-player trial cards.

## Commands
- `npm install`, then `npm test` (about 1 minute), `npm run typecheck`, `npm run calibrate -- 250000 seed`.
- Fast one-off sims: `npx esbuild x.ts --bundle --platform=node --format=esm --outfile=x.mjs && node x.mjs`. A class of 250,000 takes about a minute.
- Known test failure: 30 of 31 tests pass. The five-star draft rate is 0.88 against a bound of 0.85 (real is about 0.63). It's expected until call 1 (rawer arrivals) is built.
