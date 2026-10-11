# AFM: American Football Manager

A deep, realistic American football management sim modelled on Football Manager: high school prospects develop through college to the NFL, with dynamic potential and a patient season flow. Owner: Raichel (GitHub `cnbville`).

## Read first
1. `docs/decisions.md`: every decision so far, what's assumed and what's open.
2. `docs/development-model.md`: how the current player model works.
3. `docs/design/trial6-fix-list.md`: the last trial's fixes (all built).

## Working rules (from Raichel)
- **Ask before every design decision.** Send a short question with options and your recommendation, then wait for the answer. Never pick defaults yourself.
- **Everything realistic and dynamic.** Reason the way real football works, and derive outcomes from attributes, not flat rules.
- **Every trial result explains why.** Each notable outcome gets a reason, so Raichel can judge it. Sanity-check results for contradictions before showing them.
- **Keep token use low and write plainly.** Use short sentences and simple structure. One checker pass per trial, no big agent fan-outs.
- **Don't push or merge without Raichel's OK.**
- **Potential is hidden in the game.** The player-facing side never sees a true ceiling, only scouted estimates and hints. Exact potential is for trial and debug tooling only.

## Next job: the roadmap and the game itself
- The player model is done and ported to Rust (2026-10-11). Raichel finds further calls and bugs in gameplay, not in more trials.
- Raichel answered the 50 roadmap questions (GM+HC role, Rust desktop app, and more; see project memory). Still open: the first playable milestone. Then write the roadmap into the repo, in sections, and build section 1. Ask before deciding.

## Code
- Rust (edition 2024), Cargo workspace. Pure logic, no UI yet.
- `crates/afm-engine`: the engine library a desktop UI will call.
  - `attributes.rs`, `positions.rs`, `player.rs`: attributes, position fit, player generation.
  - `development.rs`: the yearly development step (growth, slumps, breakouts, coaching × coachability).
  - `scouting.rs`: noisy scouting reports, and `potential_outlook`, the vague potential read the game may show.
  - `level_rating.rs`: level-relative ratings ("89 among seniors").
  - `league.rs`: the 32 NFL teams, front offices, depth charts and background players.
  - `career.rs`: simulates a whole national class (250,000) from age 14 through high school, recruiting, college, the draft and pro careers. The tuning knobs are in `PIPELINE`.
  - `debug.rs`: true potential and ceilings, only with the `debug` feature. The game must never enable it.
  - Unit tests live in `src/tests/`.
- `crates/afm-tools`: tooling with the `debug` feature. `stats.rs` (class summary), `bin/calibrate.rs`, and `tests/national_class.rs` (real-world calibration checks).
- Speed: each player has his own random stream, so per-player work runs on all cores (rayon) and a seed gives the same class on any machine and core count.
- `legacy-ts/`: the original TypeScript model and scripts, for reference (`cd legacy-ts && npm install`). The trial 6 card script lives there.
- `docs/trials/trial6-cards.json`: the trial 6 cards. `docs/trials/players30-final.json`: the earlier 30-player trial.

## Commands
- `cargo test --workspace`: all 38 tests, including a full 250,000-player class (a few seconds once built).
- `cargo run --release --bin calibrate -- 250000 seed`: headline stats for one class (about 2 seconds).
- `cargo clippy --workspace --all-targets` and `cargo fmt --check`: CI runs both.
- Calibration gap: FCS players are about 2-5% of draft picks depending on the seed (real about 6%).
