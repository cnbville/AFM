# AFM: American Football Manager

A deep, realistic American football management sim modelled on Football Manager: high school prospects develop through college to the NFL, with dynamic potential and a patient season flow. Owner: Raichel (GitHub `cnbville`).

## Read first
0. `docs/handoff/README.md`: the account-move handoff (2026-10-14): current state, next steps, every decision (`decisions.md`), open questions (`open-questions.md`), working rules (`working-rules.md`), and word-for-word copies of the old project's chat and threads (`transcripts/`).
1. `docs/roadmap.md`: the vision, Raichel's working rules, all 50 roadmap answers, the platform and every build section with its status.
2. `docs/design/world.md`: section 2 (the world), the finished design spec.
3. `docs/decisions.md`: every player-model decision so far, what's assumed and what's open.
4. `docs/development-model.md`: how the current player model works.
5. `docs/design/match-engine.md`: section 3 (the match engine and stats database), the finished design spec. `docs/match-engine.md` says how the built engine works.
6. `docs/design/plays-and-tactics.md`: the plays and tactics database (play library, counters, pre-snap, learning playbooks, game plans, play calling and coach quality, tendencies, a play designer, ideas spreading). Design done 2026-10-14, built 2026-10-17 in build 7 step 5 (`docs/design/build7-checklist.md`; `game/designer.rs`, `world/trends.rs`).
8. `docs/research/README.md`: realism research (real targets for the engine, rules, baselines, development), with sources and proposals for Raichel. Research only, never decisions.
7. `docs/design/calendar.md`: section 4 (the season calendar and pacing), the finished design spec. Design done 2026-10-14, built 2026-10-16 (every point with its code in `docs/design/calendar-checklist.md`; how it works in `docs/world-model.md`, "Section 4: the calendar"). It replaces the older draft `docs/design/season-calendar-and-pacing.md` wherever they differ.
9. `docs/design/faces.md`: player faces (section 10): GAN photo faces with a stored recipe that ages, matched to the name's heritage. Design started 2026-10-16, proof of concept in `tools/faces/`, not built into the game.

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
- Section 3 (match engine and stats database): design done 2026-10-13, spec in `docs/design/match-engine.md`. Athlete-first player creation built 2026-10-13 (`athlete.rs`). The match engine, game stats and the real-data checks built 2026-10-14 (`game/`, how it works and known gaps in `docs/match-engine.md`). Still open in section 3: the career stats record (college games now run on the engine; high school games still use the stand-in). The world trial 3 fix list was built on 2026-10-14 (`docs/world-model.md`, "Trial 3 fix list"). Every college game now runs on the engine, history included. World trial 3 ran 2026-10-14 (`docs/trials/world-trial3.md`: 15 years on the engine, 9 findings logged as candidates; the debug dashboard is the mock GUI artifact). Next: Raichel's calls on the trial 3 findings. The engine physics fixes (QB drives passing, line play decides runs and pressure, depth and rotation, the RB bug, coaching through the battles) were built 2026-10-14 (spec `docs/design/engine-physics-fixes.md`, how it works in `docs/match-engine.md`, "Physics fixes"). World trial 4 ran on them (`docs/trials/world-trial4.md`). Next: Raichel's calls on the trial 4 proposals (`docs/trials/world-trial4-proposals.md`, topics 1-3 first) and the trial 3 findings still open. World trial 5 ran 2026-10-15 on the same engine: three 30-year worlds (seeds 2026-2028), the new debug dashboard (`tools/dashboard/`), raw output in the project folder at `afm/trials-30yr/`; findings and Claude's doubts in `docs/trials/world-trial5.md` (new: most FBS teams have no kicker or punter, flagships don't hold their state, out-of-position play). Raichel answered all five on 2026-10-15; they are in the fix list for the next build, `docs/design/fix-list.md`, to execute when Raichel asks (details settled then). Every trial from now on names the players who suffered big slumps, with how much and why.
- Section 4 (the calendar): built 2026-10-16 from the 11 topics, the 46 additions and the build plan; the world runs on a daily clock. Claude's own picks are listed as "proposed, not confirmed" at the end of `docs/design/calendar-checklist.md`. World trial 6 (the phase 11 trial) ran 2026-10-16 on three 30-year worlds: `docs/trials/world-trial6.md` (positives, negatives, 34 fixes measured, slumps, spot checks), raw output in the project folder at `afm/trials-section4/`, the verdict on the Debug Lab's Verdict tab. Part A of `docs/trials/world-trial6-proposals.md` (17 bugs and fixes that fell short) built and merged 2026-10-16 (PR #33), not measured yet: the next trial measures it. Part B, C and D, the trial 6 leftovers, the section 3 leftovers (stat book, record books, Hall of Fame) and plays and tactics were built 2026-10-17 in build 7 (thread "Build everything agreed, then trial 7", checklist `docs/design/build7-checklist.md`). Next in that thread: section 5 in the world (step 6), then trial 7.
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
  - `calendar/`: section 4: dates and real anchors per year (`anchors.rs`), rule books by era and the 12 agreed off-field changes (`rules.rs`), events and loudness (`events.rs`), weather (`weather.rs`), recruiting periods. The daily clock is `world/clock.rs`; the NFL year `world/nfl_year.rs`; the high school year `world/hs_year.rs`.
  - `debug.rs`: true potential and ceilings, only with the `debug` feature. The game must never enable it.
  - Unit tests live in `src/tests/`.
- `crates/afm-tools`: tooling with the `debug` feature. `stats.rs` (class summary), `bin/calibrate.rs`, and `tests/national_class.rs` (real-world calibration checks), `bin/world_trial.rs` (the section 2 trial report; `AFM_CARDS=path.json` writes full player pages, `AFM_DASH=path.json` the debug dashboard data, `AFM_RAW=dir` the full raw export as JSON lines (all three turn on the world's game log: box score totals, college stat lines and play-level situation totals, trial tooling only): every program season with engine team totals, coaches with hidden ratings, every draft pick, diversity by year), `tests/world.rs`, `bin/match_check.rs`, `bin/play_trace.rs`, `bin/dynasty_check.rs` (what each position and the play callers are worth) and `bin/call_values.rs` (rebuilds the coaches' what-beats-what table, `crates/afm-engine/data/call_values.csv`) (match engine checks), and `tests/match_engine.rs`.
- Speed: each player has his own random stream, so per-player work runs on all cores (rayon) and a seed gives the same class on any machine and core count.
- `data/real-world/` and `data/names/`: researched data with READMEs naming sources and gaps. Names (rebuilt 2026-10-14): every SSA boy's name since 1940 and every Census surname with 100+ people.
- `tools/dashboard/`: the debug dashboard v2 (AFM Debug Lab): `build_data.py` turns world trial raw exports into the page's data; see its README.
- `legacy-ts/`: the original TypeScript model and scripts, for reference (`cd legacy-ts && npm install`). The trial 6 card script lives there.
- `docs/trials/trial6-cards.json`: the trial 6 cards. `docs/trials/players30-final.json`: the earlier 30-player trial.

## Commands
- `cargo test --workspace --release`: all 52 tests, including a full 250,000-player class, a small 3-year world with its college games on the match engine, and 640 match engine games (about 3 minutes once built).
- `cargo run --release --bin world_trial -- 2026 30`: the full world with 30 years of history and the trial report. With every college game on the engine this takes about 2 minutes per season on 4 cores; `AFM_ENGINE_FROM=2026` uses the old stand-in for the history (quick checks only, about 6 minutes).
- `cargo run --release --bin calibrate -- 250000 seed`: headline stats for one class (about 2 seconds).
- `cargo run --release --bin calendar_check -- 1.0 3 2026 2100`: section 4 counters per year (commitments by month, signings, visits, portal days, declarations, stakes) and the loud events of the last year. Arguments: world scale, history years, seed, first engine year (2100 keeps every game on the stand-in, for speed).
- `cargo run --release --bin positions`: position shares (high school and draft against real data), drafted bodies against the combine, and position changes.
- `cargo run --release --bin match_check -- nfl 400`: match engine averages against real 2021-25 data (`fbs`, `hs` too; add a seed and `pbp` for a play-by-play). `play_trace` traces one snap.
- `cargo clippy --workspace --all-targets` and `cargo fmt --check`: CI runs both.
- Calibration: in the world, FCS players are about 6% of draft picks (real about 6%). The standalone class sim in `career.rs` still gives 2-5%.
