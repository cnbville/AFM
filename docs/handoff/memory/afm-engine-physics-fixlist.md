---
name: afm-engine-physics-fixlist
description: Dynasty cause found 2026-10-14 (engine position weights wrong, coaching no effect) and Raichel's coaching rule for the physics fix list
metadata:
  type: project
  modified: 2026-10-14T11:31:56.037Z
---

World trial 3 dynasty check (2026-10-14, `dynasty_check` bin on branch claude/project-thread-ata5eh): equal FBS teams, same +0.5 lineup boost per group: QB +10 adds ~0.2 pts, OL ~0.2, DL ~0.9, secondary 1-2, LB ~2, WR/TE 2-3, RB +10 swings -7.5/+6.8 (unstable, likely bug); play caller 18 vs 4 ~0. All starters +1 = ~4 pts. Tennessee won 36 of 43 close games vs 25 expected. Raichel: trace Tennessee first, then fix both.

Physics fix list (proposed): QB play drives passing, line play decides runs/pressure, RB bug, coaching that shows in games.
Raichel's coaching rule (2026-10-14): the play call itself must matter through the battles (running into a +2 box loses because of the matchup, whoever called it). HC quality drives calls when the user isn't picking, plus adjustments, prep and scheme install.

Tactics thread note (2026-10-14): good plans/calls rewarded only through battles (box count, leverage, matchups), never a flat bonus, for coordinator and user calls alike. Proposed dev check: good vs bad plan point swing within a band from real play-caller data; caller 18 vs 4 must move clearly off ~0 (proposed, not confirmed).
Status: Raichel said yes; spec merged to main 2026-10-14 (4f83f03) at docs/design/engine-physics-fixes.md, the NEXT NEEDED FIX; Raichel 2026-10-14: do not build until told.

**How to apply:** fold this into the physics fix list item 4. Related: [[afm-match-engine-build]]
