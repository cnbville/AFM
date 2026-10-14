---
name: afm-trial6-done
description: AFM trial 6 built, published and merged to main on 2026-10-11; next step is building the game itself
metadata:
  type: project
  modified: 2026-10-11T19:13:20.342Z
---

On 2026-10-11, trial 6 was finished in its own thread:
- All 17 fixes built. Film line C was confirmed by Raichel.
- 20-card trial published as a new version of https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP. The cards are in docs/trials/trial6-cards.json.
- PR #2 merged into main (merge commit c4b48e7). All 35 tests pass. The detail picks Claude made are listed in docs/decisions.md.
- Known gaps: FCS players are about 3% of draft picks (real about 6%). The five-star draft rate swings by class: 61% and 94% on two seeds, real about 63%.

**Why:** Raichel wants to find further calls and bugs in gameplay, not in more trials.
**How to apply:** Next is building the game itself. The roadmap and the player's role in the game are still open, so ask Raichel about both first ([[raichel-approves-design-decisions]]). Related: [[afm-trial5-thread]].
