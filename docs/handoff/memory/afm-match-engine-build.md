---
name: afm-match-engine-build
description: Section 3 match engine build status (merged to main 2026-10-14), known calibration gaps and what is still unbuilt
metadata:
  type: project
---

Match engine merged to main 2026-10-14 (merge a710f20, branch claude/section3-match-engine-7n0c6b). Code in crates/afm-engine/src/game/, docs/match-engine.md.
- Dev checks: `match_check nfl|fbs|hs`, `play_trace` (AFM_LEVEL/AFM_COV/AFM_LOS/AFM_TRACE), tests/match_engine.rs (wide bands).
- NFL now: completion 66%, YPA 7.4, 3rd down 39.7% (all near real). Known gaps logged as trial candidates: points/plays high (pass rate 66% vs 58%; college calls from NFL tables so passes 64% vs 50%), sacks and INTs low, NFL YPC 3.9 vs 4.5, red zone TD 65% vs 56%, kick returns short, HS 83 plays/team, FBS completion ~4 pts low.
- Not built: world games through the engine, season/career record with percentiles and era adjustment, coach buzz from stats, gem flags.
- Lesson: synthetic lower-level teams have absolute ratings far below NFL (FBS ~61, HS ~37), so any linear attribute formula is very steep there; the fix that worked was curves that narrow below 60 and the QB leading his receiver's real cut.

**Why:** next threads (world trial 3 list, then trial 3) build on this.
**How to apply:** read docs/match-engine.md first; don't re-tune known gaps without Raichel (they are trial candidates). Related: [[afm-section3-design]]
