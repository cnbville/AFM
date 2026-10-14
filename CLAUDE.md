# AFM: American Football Manager

A deep, realistic American football management sim modelled on Football Manager: high school prospects develop through college to the NFL, with dynamic potential and a patient season flow. Owner: Raichel (GitHub `cnbville`).

## Read first
1. `docs/roadmap.md`: the vision, Raichel's working rules, all 50 roadmap answers, the platform and every build section with its status.
2. `docs/design/world.md`: section 2 (the world), the finished design spec.
3. `docs/decisions.md`: every player-model decision so far, what's assumed and what's open.
4. `docs/development-model.md`: how the current player model works.
5. `docs/design/match-engine.md`: section 3 (the match engine and stats database), the finished design spec. `docs/match-engine.md` says how the built engine works.
6. `docs/design/season-calendar-and-pacing.md`: draft calendar for section 4 (older; the roadmap answers override its defaults).

## Working rules (from Raichel)
- **Ask before every design decision.** Bring a concrete proposal, then wait for the answer. Never pick defaults yourself.
- **Default to ultra realism.** Never ask "realistic or simplified?": the answer is always the most realistic option. Ask only about deep mechanics (how things work and interact), each with a concrete proposal Raichel can agree with or adjust. Go topic by topic in order and don't skip topics.
- **Influences are noteworthy, rarely driving.** No single factor (background, family, school) should become overpowered.
- **Everything realistic and dynamic.** Reason the way real football works, and derive outcomes from attributes, not flat rules.
- **Every trial result explains why.** Each notable outcome gets a reason, so Raichel can judge it. Sanity-check results for contradictions before showing them.
- **Keep token use low and write plainly.** Use short sentences and simple structure. One checker pass per trial, no big agent fan-outs.
- **Don't push or merge without Raichel's OK.** Build threads have that OK: they merge to `main` and send one final reply.
- **Mark what's unconfirmed.** In docs, label Claude's proposals that Raichel hasn't answered as "proposed, not confirmed". Don't invent decisions.
- **Potential is hidden in the game.** The player-facing side never sees a true ceiling, only scouted estimates and hints. Exact potential is for trial and debug tooling only.

## The section loop
Each roadmap section goes: (1) design rounds with Raichel until the details are nailed down, (2) a spec in `docs/design/` with Raichel's own wording and examples, (3) a fresh thread builds it, merges it and sends one message, (4) a short trial with a "why" on every result.

## Next job
- Section 1 (the engine, the player model in Rust) is done. Raichel finds further calls and bugs in gameplay, not in more trials.
- Section 2 (the world): built 2026-10-13 (`crates/afm-engine/src/world/`, how it works in `docs/world-model.md`, trial in `docs/trials/world-trial1.md`). Raichel's trial 1 list (items 1-8: prove-it extensions, ethnicity and hiring bias, names, pipelines, the coaching ladder, academics, draft value, measurables) built 2026-10-13, trial in `docs/trials/world-trial2.md`; item 9 (the hidden-gem stat footprint) waits for section 3. Open: realism questions 2-7 and trial 2's flags.
- Section 3 (match engine and stats database): design done 2026-10-13, spec in `docs/design/match-engine.md`. Athlete-first player creation built 2026-10-13 (`athlete.rs`). The match engine, game stats and the real-data checks built 2026-10-14 (`game/`, how it works and known gaps in `docs/match-engine.md`). Still open in section 3: running the world's games through the engine and the career stats record. Next: build the world trial 3 fix list (`/mnt/project-files/design/world-trial3-candidates.md`), then run world trial 3 with a bigger debug dashboard (Raichel's agreed order).
- Confirmed by Raichel (2026-10-11): the section order and the section loop above, the scouting-staff design, the college program grade categories, 30 years of hidden history before 2026, and the topic 1 leftovers (personality and the powerhouse flip side).

## Code
- Rust (edition 2024), Cargo workspace. Pure logic, no UI yet.
- `crates/afm-engine`: the engine library a desktop UI will call.
  - `attributes.rs`, `positions.rs`, `player.rs`: attributes, position fit, player generation.
  - `development.rs`: the yearly development step (growth, slumps, breakouts, coaching × coachability).
  - `scouting.rs`: noisy scouting reports, and `potential_outlook`, the vague potential read the game may show.
  - `level_rating.rs`: level-relative ratings ("89 among seniors").
  - `league.rs`: the 32 NFL teams, front offices, depth charts and background players.
  - `career.rs`: simulates a whole national class (250,000) from age 14 through high school, recruiting, college, the draft and pro careers. The tuning knobs are in `PIPELINE`.
  - `world/`: section 2, the living world (real colleges, generated high schools, coaches, backgrounds, recruiting, portal, NIL, 30-year history). See `docs/world-model.md`.
  - `body.rs`: height, weight, arms, hands and combine results, from the real combine.
  - `game/`: section 3, the match engine (tracked plays, calling, kicking, rules per level, the play log and stats). See `docs/match-engine.md`.
  - `athlete.rs`: athlete-first creation (section 3, topic 5.2): body, athletic traits and skill leanings first, then coaches place and move players.
  - `debug.rs`: true potential and ceilings, only with the `debug` feature. The game must never enable it.
  - Unit tests live in `src/tests/`.
- `crates/afm-tools`: tooling with the `debug` feature. `stats.rs` (class summary), `bin/calibrate.rs`, and `tests/national_class.rs` (real-world calibration checks), `bin/world_trial.rs` (the section 2 trial report; `AFM_CARDS=path.json` writes full player pages), `tests/world.rs`, `bin/match_check.rs` and `bin/play_trace.rs` (match engine checks), and `tests/match_engine.rs`.
- Speed: each player has his own random stream, so per-player work runs on all cores (rayon) and a seed gives the same class on any machine and core count.
- `data/real-world/` and `data/names/`: researched data with READMEs naming sources and gaps.
- `legacy-ts/`: the original TypeScript model and scripts, for reference (`cd legacy-ts && npm install`). The trial 6 card script lives there.
- `docs/trials/trial6-cards.json`: the trial 6 cards. `docs/trials/players30-final.json`: the earlier 30-player trial.

## Commands
- `cargo test --workspace --release`: all 50 tests, including a full 250,000-player class, a small 3-year world and 640 match engine games (about 60 seconds once built).
- `cargo run --release --bin world_trial -- 2026 30`: the full world with 30 years of history and the trial report (about 100 seconds).
- `cargo run --release --bin calibrate -- 250000 seed`: headline stats for one class (about 2 seconds).
- `cargo run --release --bin positions`: position shares (high school and draft against real data), drafted bodies against the combine, and position changes.
- `cargo run --release --bin match_check -- nfl 400`: match engine averages against real 2021-25 data (`fbs`, `hs` too; add a seed and `pbp` for a play-by-play). `play_trace` traces one snap.
- `cargo clippy --workspace --all-targets` and `cargo fmt --check`: CI runs both.
- Calibration: in the world, FCS players are about 6% of draft picks (real about 6%). The standalone class sim in `career.rs` still gives 2-5%.
