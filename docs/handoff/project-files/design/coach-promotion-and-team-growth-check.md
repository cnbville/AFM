# Coach promotion and team growth check (2026-10-13)

Same world as trial 2 (seed 2026, 30 years of hidden history). Nothing in the code was changed.
The proposals at the bottom are proposed, not confirmed.

## 1. Michael Snow: high school head coach to DB coach

**His record.** Pierz South 2024: 5-5. Long Lake Hills 2025: 6-4. Neither team stood out.
Long Lake Hills did win the 2024 state title (12-1), but that was the coach before him.

**Why he got the job.** It was specialization, not general prestige, but from a hidden number:
- An opening for a position coach looks at high school coaches in the same state whose
  hidden teaching skill for that group is 13 or higher (out of 20). His DB teaching is 13.6.
- The hiring score then adds a big bonus for that hidden DB teaching skill (`staff.rs:507`).
  He is a defense-side coach: DB 13.6, DL 13.2, LB 11.8; offense mostly around 9.
- He did not get the "hot high school coach" boost. That needs a state title plus 90% wins
  over his last two seasons. His school's two seasons were 18-5 (78%).

**Across the whole world:** 3,379 moves from high school to a college position coach job.
- 86% went to a group he teaches better than his own average.
- 72% went to his stronger side of the ball.
- 39% went to his single best group.

**What's naive.**
- The hiring school reads his hidden teaching skill exactly, with no noise and no visible
  evidence. A real school can't see that. It sees his unit's results and his players' growth.
- High school seasons only store wins, losses and titles. There are no defensive stats yet
  (those come with section 3, the match engine).
- Bug: the "hot high school coach" check uses the school's last two seasons, not the coach's
  own. A new coach can get credit for his predecessor's title (`staff.rs:316`).

## 2. Does every program just get better?

**The bar does not rise.**
- Wins come from games between teams (one wins, one loses), so they can't all rise.
- The letter grades recruits see are ranked within each level every year. Someone's gain
  is someone's loss.
- Average FBS prestige: 0.597 in 1996, 0.607-0.611 every year from 1999 to 2023. Flat.
  The spread between top and bottom is unchanged (0.136 to 0.132).
- Lower levels drift up in the first 10-15 years and then flatten (FCS 0.209 to 0.246, D2
  0.118 to 0.139). The start values were slightly out of balance. This happens inside the
  hidden history, before the game starts.
- Slow leftover: tradition still creeps toward recent results (it moves about 1.5% a year).
  Over a 50-year career that adds about +0.01 prestige. Harmless, but it should be fixed.

**But rises are only weakly reasoned.** How much each FBS program's prestige changed over
30 years, compared with possible causes (a correlation from -1 to 1; 0 means no link):
- winning percentage: 0.36
- head coach's hidden quality: 0.05
- head coach's recruiting skill: 0.05 (FCS: -0.16)
- a big booster: 0.20
- starting prestige: -0.29 (low teams drift up, high teams drift down)

Examples:
- Kentucky had coaches with hidden quality 16.4 and a recruiter rated 20/20. Even so, it lost
  for 30 years (0.69 to 0.58).
- Bowling Green rose from 0.42 to 0.58 over a losing decade. It is mostly drift toward the middle.

Why: a recruit's choice never looks at the head coach's recruiting skill. That skill only
sharpens how well his staff rates recruits (`recruiting.rs:907`). Recruits look at the coach's
reputation grade, not his skill at building relationships. Game management adds less than
1 rating point. Facilities and fans follow results and NIL money automatically, so nobody
chooses to invest in them.

## Proposed fixes (proposed, not confirmed)

**A. Hot high school coach uses his own seasons.** A bug fix.

**B. Coaches get hired on visible evidence, not hidden skill.** Each school reads his hidden
teaching with noise, and a better-connected staff gets a sharper read. On top of that, it adds
what can be seen:
- how his players at that position grew under him, against what was expected
- how many of them moved up: college offers, picks
- once section 3 is built, his unit's stats compared with his level (pass defense, takeaways)

Each coach also gets a "came up as" specialty, like DB coach or OC. A high school head coach
is only seen as a DB candidate if he came up on defense or his secondary stood out.

**C. Rises need a reason.** Every rising program should have a cause the player can copy:
- The head coach's recruiting skill wins recruits directly, through relationships. A great
  closer beats a bigger name for some recruits.
- Facilities move in steps (a booster gift, a budget decision), not on their own.
- Drift toward the middle shrinks to the realistic amount, and starting values begin balanced.
- Every big riser or faller in the trial report gets a "why" line: coach, booster, pipeline,
  facilities, conference.
