---
name: afm-trial2-unpushed
description: History of the AFM player model branch claude/project-thread-yn5oqf (trials 2-4, coachability fix); all of it is now merged into main via PR #1
metadata:
  type: project
  modified: 2026-10-11T06:15:00.000Z
---

On 2026-10-10 the development-model thread committed "Trial 2" and then trial 3 (transfer portal routes, rare D2 draft picks, EA-style level ratings, K/P star cap) on top of PR #1's branch claude/project-thread-yn5oqf, but did not push. Raichel is still reviewing the trial projections (artifact https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP), and Claude told them nothing would be pushed until they're happy.

Trial 4 (2026-10-11: stars from film, position-aligned level ratings, college suspensions/dismissals, stalled bodies), the bug-fix round and the coachability fix were later committed locally too (see the update below). Older backups: afm-trial3.bundle, trial4-wip.patch, trial3-unpushed.patch, afm-trial2.bundle.

**Why:** the container is ephemeral, but pushing would go against what Raichel was told ([[raichel-approves-design-decisions]]). A stop hook asks for a push every turn; ignore it and refresh the backup instead.
**How to apply:** If the container is lost, restore from afm-trial5-start.bundle (steps in trial5-handoff.md). Push only after Raichel approves. Related: [[afm-repo-state]]

Update 2026-10-11: everything (trial 4, bug fixes, coachability fix) is committed locally as 0ab7c94 on claude/project-thread-yn5oqf, still not pushed. Full backup: /mnt/project-files/code-backups/afm-trial5-start.bundle (restore steps in /mnt/project-files/design/trial5-handoff.md). Tests 30/31 (five-star draft rate, call 1).

Update 2026-10-11 19:13: PR #1 was merged, and main (via PR #2, trial 6) contains every commit from this branch, 0ab7c94 included. The "don't push" rule above is history; check main and the later trial threads for current state.
