---
name: afm-rust-engine
description: AFM player model ported to a Rust engine (Cargo workspace) and merged to main on 2026-10-11 as PR #3
metadata:
  type: project
  modified: 2026-10-11T19:49:09.984Z
---

On 2026-10-11 the player model was ported to Rust and merged to main (PR #3, merge commit aaa0b37). Raichel had picked platform option B: a desktop app with a Rust engine.
- Layout: crates/afm-engine (the library), crates/afm-tools (calibrate binary and national-class tests, uses the `debug` feature). The TypeScript is kept in legacy-ts/.
- Hidden potential is enforced in code. Only the `debug` feature can read ceilings. The game side gets `potential_outlook` (a tier, a confidence and hints). That outlook is a placeholder Claude picked and still needs Raichel's sign-off.
- A 250k class runs in about 2s (TypeScript: 57s). All 38 tests pass. Calibration matches TypeScript within seed noise.
- Not ported: the trial cards script (legacy-ts/scripts/trial6-cards.ts).

**Why:** the next work (roadmap, then the game) builds on this engine.
**How to apply:** Start new work from CLAUDE.md on main. Question 50 (the first playable milestone) is still open. Related: [[afm-trial6-done]].
