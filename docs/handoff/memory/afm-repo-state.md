---
name: afm-repo-state
description: State of the cnbville/AFM repo: TypeScript stack, main branch, player development model PR #1 (as of 2026-10-10)
metadata:
  type: project
  modified: 2026-10-10T20:01:23.090Z
---

As of 2026-10-10: `main` holds only a one-file initial commit (README). PR #1 (draft, branch claude/project-thread-yn5oqf) adds a TypeScript scaffold (Node 22, Vitest, no runtime deps, GitHub Actions CI) and the core player model: src/attributes, positions, player, development (yearly checkpoint), scouting, career (HS→college→NFL cohort sim). Rules are in docs/development-model.md.

TypeScript is the default stack, but Raichel was still weighing Godot/Unity in the exploration thread, so the model is kept as pure logic with no UI coupling.

Design choices tied to the season doc (/mnt/project-files/design/season-calendar-and-pacing.md): one development checkpoint per year at W1, and potential moves at most one 5-point band per checkpoint.

**Why:** later threads build on this model (season flow, player cards, draft).
**How to apply:** Check PR #1's state before building on it. The full-cohort test (250k players) takes about 35s.
