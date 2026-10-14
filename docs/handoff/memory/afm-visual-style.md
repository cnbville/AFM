---
name: afm-visual-style
description: Raichel's chosen AFM visual style: stats/text first, 2D highlights styled like NFL Next Gen Stats (decided 2026-10-10)
metadata:
  type: project
---

Raichel decided (2026-10-10) that game day is mostly text and stats, with highlights shown in 2D, and that the highlights should look like the NFL's Next Gen Stats graphics (they shared a pass chart: throw locations relative to the line of scrimmage, colored by complete, incomplete, touchdown or interception, plus a stat line).

**Why:** ultra realism is the priority, and visuals should only show what the simulation decides.
**How to apply:** The match engine has to output play-level spatial data (pass target location, air yards, ball carrier path), not just box-score totals. Next Gen Stats metrics such as CPOE need an expected model behind them. Related: [[afm-repo-state]] [[raichel-approves-design-decisions]]
