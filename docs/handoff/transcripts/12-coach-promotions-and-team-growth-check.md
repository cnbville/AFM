# 12 Coach promotions and team growth check

Thread "Coach promotions and team growth check" from the AFM project on the old account (thread id cmsg_015NJfb6NRUve4Wa2buMtZKsQwTWC8dnTgBhsQw5YwgLPZ). Every message, oldest first, copied word for word on 2026-10-14. 8 messages.

### 2026-10-13 16:30:04 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsQwTWC8dnTgBhsQw5YwgLPZ · attachments: coach-promotion-and-team-growth-check.md</sub>

answers these questions  in a fresh thread while the other one runs/.

1. THe example of Michael snow going from HS to ->  DB coach, what was the reason for DB coach ? i want to make sure you're not just promoting on prestige in general but speciliaztion, did his team perform well defenesively in HS ?
2. Do a double pass to see if the team growth wasn't naive, ofcourse teams can get btter but i don't want the bar to rise, teams that rise should have reasoning so that players who play the game can pick a strategy and not just every school gets better. I don't have any data to say this is the casei t's just an worry

### 2026-10-13 16:30:16 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsKHGk5wfLHerFufb53wjReh</sub>

I'm starting by tracing Michael Snow's path from HS coach to DB coach in the trial 2 world.

### 2026-10-13 16:30:29 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsD8qykHuvixYMhNE7dcCm2b</sub>

*[Progress checklist update]*

```text
Coach promotion and team growth check

✱ Tracing Michael Snow's move to DB coach
○ Checking how program strength changes over time
○ Writing up findings and proposed fixes
```

### 2026-10-13 16:35:42 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsTkQEJmP3r7PAnZ9KfTdq71</sub>

*[Progress checklist update]*

```text
Coach promotion and team growth check

✓ Traced Michael Snow's move to DB coach
✱ Checking what drives programs that rise (second pass)
○ Writing up findings and proposed fixes
```

### 2026-10-13 16:39:31 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsY17yApH9TSksXXwTC3fsv5</sub>

*[Progress checklist update]*

```text
Coach promotion and team growth check

✓ Traced Michael Snow's move to DB coach
✓ Checked what drives programs that rise (two passes)
**✓ Wrote up findings and proposed fixes**
```

### 2026-10-13 16:39:39 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsJRTZNC71MHLRCmThjgZ3gD</sub>

Can you agree to or adjust fixes A, B and C below? I'll add the ones you agree to the trial 3 list. I didn't change any code.

**1. Michael Snow.** Yes, he was hired as a specialist, not on general prestige. But the reason is a hidden number, not how his team played.
- His own teams were average: 5-5, then 6-4. The 2024 state title at Long Lake Hills was won by the coach before him.
- He got the job because his hidden DB teaching skill is 13.6 out of 20. He's a defense coach, and his offense skills are around 9.
- Across the whole world, 86% of high school coaches who became college position coaches went to a group they teach better than their own average.
- What's naive: the hiring school reads that hidden skill exactly, which no real school could do. High school games don't track defensive stats yet (that comes with section 3).
- I also found a bug: the "hot high school coach" check counts the school's last two seasons, not the coach's own. So a new coach can get credit for his predecessor's title.

**2. Team growth.** The bar doesn't rise. Wins come from games between teams, and the grades recruits see are ranked within each level every year. Average FBS prestige has stayed flat since 1999. But the reasons teams rise are weak:
- A program's rise links to its win rate, but almost not at all to how good its head coach is.
- Kentucky had excellent hidden coaches and still lost for 30 years.
- Bowling Green rose over a losing decade, mostly by drifting toward the middle.
- The cause: recruits never look at the head coach's recruiting skill. Facilities also go up by themselves instead of through someone's decision.

**Proposed fixes**
- **A.** The "hot high school coach" check uses his own seasons only.
- **B.** Schools hire on what they can see: a rough read of his skill, how his players at that position improved and moved up, and later his unit's stats against others at his level. Each coach also gets a "came up as" specialty, like DB coach or OC.
- **C.** Every rise needs a cause a player could copy. A great recruiter wins some recruits over bigger names. Facilities improve in steps from a booster gift or a budget decision. Drift toward the middle is smaller. Every big riser or faller in the trial gets a "why" line.

The full numbers are in [coach-promotion-and-team-growth-check.md](/mnt/project-files/design/coach-promotion-and-team-growth-check.md).

### 2026-10-13 16:55:29 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs1xiQQRbYt8kXXf8RRo2yaw</sub>

agree on all three, at them to the list. Probably run the fix after section 3

### 2026-10-13 16:55:52 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsGEqHa22e9tC5tAPLhcy3xu</sub>

Done. A, B and C are now item 15 on the [trial 3 list](/mnt/project-files/design/world-trial3-candidates.md), marked to build after section 3.
