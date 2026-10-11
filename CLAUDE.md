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

## Next job: the game itself
- Trial 6 is done: all 17 fixes in `docs/design/trial6-fix-list.md` are built, and the film line is option C (confirmed).
- Raichel finds further calls and bugs in gameplay, not in more trials.
- Still open: the roadmap for building the game and the player's role in it. Ask before deciding either.

## Code
- TypeScript, Node 22, Vitest, no runtime dependencies. Pure logic, no UI.
- `src/attributes.ts`, `positions.ts`, `player.ts`: attributes, position fit, player generation.
- `src/development.ts`: the yearly development step (growth, slumps, breakouts, coaching × coachability).
- `src/scouting.ts`: noisy scouting reports.
- `src/levelRating.ts`: level-relative ratings ("89 among seniors").
- `src/league.ts`: the 32 NFL teams, front offices, depth charts and background players.
- `src/career.ts`: simulates a whole national class (250,000) from age 14 through high school, recruiting, college, the draft and pro careers. The tuning knobs are in `PIPELINE`.
- `scripts/calibrate.ts`: prints headline stats for one class.
- `docs/trials/trial6-cards.json`: the trial 6 cards (`scripts/trial6-cards.ts`). `docs/trials/players30-final.json`: the earlier 30-player trial.

## Commands
- `npm install`, then `npm test` (about 1 minute), `npm run typecheck`, `npm run calibrate -- 250000 seed`.
- Fast one-off sims: `npx esbuild x.ts --bundle --platform=node --format=esm --outfile=x.mjs && node x.mjs`. A class of 250,000 takes about a minute.
- All 35 tests pass (trial 6). Calibration gap: FCS players are about 3% of draft picks (real about 6%).
