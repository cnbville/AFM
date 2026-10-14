# 17 World Trial 3

Thread "World Trial 3" from the AFM project on the old account (thread id cmsg_015NJfb6NRUve4Wa2buMtZKsKq4hQ8PrYACjiAt4Cefp8B). Every message, oldest first, copied word for word on 2026-10-14. 36 messages.

### 2026-10-14 08:33:08 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsKq4hQ8PrYACjiAt4Cefp8B · attachments: AFM World Mockup <https://claude.ai/code/artifact/86418cce-1674-4647-b952-d45103e32ad7>; AFM Player Pages <https://claude.ai/code/artifact/11553b36-ec51-4ac1-9cc6-528bfd83c53c>; world-trial3.md</sub>

World Trial 3 is starting in its own thread, as you asked.

### 2026-10-14 08:33:19 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsST9gxtvKTtkmVHV8zD9ikL</sub>

Starting World Trial 3 by building 15 years of history with every college game on the match engine.

### 2026-10-14 08:33:23 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs5H8xQqNtE6jqgt3vHfkdoW</sub>

*[Internal note between Claude sessions: Received a message from the coordinator]*

> The previous thread had already started a dashboard export. It's saving that work to main now. Pull main before you build the dashboard, and build on that export if it has landed.

### 2026-10-14 08:33:31 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsYJ7rvqVGp65iXHkoG35m6B</sub>

*[Progress checklist update]*

```text
World Trial 3

✱ Reading the trial tools and fix-list notes
○ Run the world with 15 years of history on the engine
○ Check the flagged items and match engine gaps
○ Write the trial report with a why on each result
○ Refresh the mock GUI with a bigger debug dashboard
○ Refresh the player pages
```

### 2026-10-14 08:39:15 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsFbkVob3nzUic2hT2c3cVA5</sub>

*[Progress checklist update]*

```text
World Trial 3

✓ Read the trial tools and fix-list notes
✱ Adding a game log and dashboard export to the trial tool
○ Run the world with 15 years of history on the engine
○ Check the flagged items and match engine gaps
○ Write the trial report with a why on each result
○ Refresh the mock GUI with a bigger debug dashboard
○ Refresh the player pages
```

### 2026-10-14 08:39:51 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsLsdxpDFERavRUGQ5jFEC5H</sub>

*[Internal note between Claude sessions: Received a message from the coordinator]*

> Handoff from the thread that built the fix list:
> - main is at 3fa4f09 and has the fix list plus the dashboard data export.
> - Run: AFM_DASH=dash.json AFM_CARDS=cards.json cargo run --release --bin world_trial -- 2026 15. Every college game runs on the engine, history included. Expect about 45 minutes on 4 cores. Do NOT set AFM_ENGINE_FROM, which switches to the quick stand-in model and is meant for checks only.
> - dash.json has: every program season with engine team totals (points for/against, yards, turnovers, sacks), prestige and grade history, story lines, coaches with hidden ratings and unit records, every draft pick, and FBS head coaches by ethnicity per year. Build the bigger debug dashboard from it.
> - Checks to report with whys: items 9, 10, 11 and 14 in world-trial3-candidates.md (the #1 pick is a QB 58-62% of the time vs 73% real; three-stars drafted 4.8% vs 6.0%; two-stars 1.0% vs 2.1%; longest Power 4 drought 21 seasons); the dynasty spread (the stand-in check capped at 6 titles); diversity (real figures: 7 in 2009, 16 in 2025); and engine scoring per division against docs/match-engine.md. Drafted kickers and punters still run heavy (209 and 230 lb vs 197 and 214).
> - Not built yet: per-player season stats from world games (the career stats record) and high school games on the engine.

### 2026-10-14 08:49:20 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsC8G4WnGNDe4GHom5PyAyeo</sub>

*[Progress checklist update]*

```text
World Trial 3

✓ Read the trial tools and fix-list notes
**✓ Added a game log with player stat lines to the dashboard export**
✱ Running the world: 15 years of history, every college game on the engine
○ Check the flagged items and match engine gaps
○ Write the trial report with a why on each result
○ Refresh the mock GUI with a bigger debug dashboard
○ Refresh the player pages
```

### 2026-10-14 09:09:36 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsRtQGSVHL8917NwCZRdbizC</sub>

*[Progress checklist update]*

```text
World Trial 3

✓ Read the trial tools and fix-list notes
✓ Added a game log with player stat lines to the dashboard export
✓ Ran the world: 15 years, every college game on the engine
✓ Checked the flagged items and match engine gaps
✓ Wrote the trial report with a why on each result
✓ Refreshed the mock GUI with a bigger debug dashboard
**✓ Refreshed the player pages and merged to main**
```

### 2026-10-14 09:09:44 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs1pdtAXV2fmLHpxBJQySgMx</sub>

World Trial 3 is done. I need your calls on the findings below. I only logged them as candidates and didn't fix anything.

I ran 15 years of history (2011-2025) with every college game on the match engine. That's 42,427 games, and the world built in 14 minutes.

**What's off (each one has its why in the report and the dashboard):**
1. **Dynasties are back.** Tennessee won 9 of 15 titles and went 198-13 under one coach. The engine turns a small lineup edge into wins far more reliably than the old stand-in did. Tennessee went 16-0 in years when its lineup only ranked #3 or #4. The earlier "max 6 titles" result was measured on the stand-in.
2. **Boston College, Iowa State and Wake Forest never had a winning season** in 15 years. 25% of FBS games end 28+ points apart, so weak teams almost never win.
3. **Only 6 Black FBS head coaches in 2025** (real 16). Just 11% of coordinators are coaches of color. I haven't traced why the engine history shrinks that pool.
4. **The #1 pick is a QB only 45% of the time** (real 73%).
5. **Three-stars are drafted at 4.6% (real 6.0%) and two-stars at 1.1% (real 2.1%).** Five-stars land on 62.5% (real 63%).
6. **Too much offense in college games:** 35 points and 76 plays a team, 62% passing, few sacks and interceptions, 10-yard kick returns, and no home-field edge.
7. **Stat lines are unreal.** One back gets 92% of his team's carries and runs for 4,267 yards. A kicker had 94 sacks: depth charts pick by rating, and kickers are built too athletic (4.54 forty, real 4.91).
8. **Kickers, punters and QBs are too fast and too heavy** at the combine.
9. **Names repeat too much:** 46.5% of players share a full name with another active player.

**New in the dashboard:** sortable pages for every program (each season with engine team totals and its story), every coach (hidden ratings and the gap between reputation and true value), every draft pick, champions, engine averages against real FBS, season stat leaders against the real records, and coach diversity by year. Debug view is on by default. Player pages now include college stat lines.

The trial code, report and raw output are merged to main.

### 2026-10-14 09:16:18 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs642eCACQQyZxqpdnxaQA8K</sub>

Let's start with the first dynasties. I'm worried what causes the dynasty. It's too much but isn't the issue more the physics of the simulated games ?

### 2026-10-14 09:16:29 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs4LN5we2HCs1GnoJ5J42R2N</sub>

Good question. I'll test it: how often the stronger lineup wins by size of the gap, and how much the coach adds.

### 2026-10-14 09:26:46 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsSgkmFcq7M5qbR2oU9ePPHt</sub>

*[Progress checklist update]*

```text
Dynasty cause check

✱ Re-running the 15-year world with per-game lineup and coach data
○ Measure win chance by lineup gap against real football
○ Measure how much the head coach adds on top
○ Answer: physics, roster, or coach
```

### 2026-10-14 09:27:10 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsCS1whtgMBrH32aAWzy8jJ7</sub>

3 years is fine for this specific test

### 2026-10-14 09:27:15 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsF8PWmcsL45NvKm3RHURAdB</sub>

Thanks. The 15-year run is almost done (it takes about 14 minutes), so I'll let it finish. More games also gives a sharper answer.

### 2026-10-14 09:37:59 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs8HXFS8s88wppgAAnTwmQnp</sub>

*[Progress checklist update]*

```text
Dynasty cause check

✓ Re-ran the 15-year world with per-game lineup and coach data
✓ Measured win chance by lineup gap (12,583 FBS games)
✓ Measured what the head coach's calling adds
**✓ Checked Tennessee's wins against what its lineup predicts**
```

### 2026-10-14 09:38:05 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsFJ6kEUJzUp14oiuAuUHbc6</sub>

Partly, but the physics aren't the main cause. I measured 12,583 FBS games.

**Upsets are fine.** Once you know the lineup gap, game results still swing by about 19 points either way. Real college games swing about 16 points around the betting line. So the engine isn't too predictable game to game.

**The coach's play calling adds almost nothing.** Scheme and game management have about zero effect on the score. So a great coach isn't winning games on Saturdays.

**Lineup gaps are big, and each rating point is worth a lot.** One point of lineup strength is worth about 4 points on the scoreboard. Tennessee's lineup averaged 7.8 points better than its opponents, which predicts about 87% wins.
- **Inferred:** gaps turn into blowouts faster than real lines suggest (about 22 points of spread between teams here against about 15 for real). It could be the engine's scale or rosters that are too far apart. I can't tell which yet.

**Something extra is going on at Tennessee.** In close games (lineup gap under 3), it won 36 of 43 when its lineup predicts about 25. That's far beyond luck. Georgia, Alabama and Ohio State won about what their lineups predicted. The edge isn't coaching, so it has to be in the roster, and the average rating hides it.
- **Inferred:** the engine may weigh some positions, like QB or the lines, much more than the average does. That would actually be realistic.

**My recommendation:** trace Tennessee's close games position by position against its opponents, then run a controlled test. Two teams with the same average lineup, one with a better QB or line. The test changes nothing in the game. Should I run it?

### 2026-10-14 10:50:30 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs2mDb7SheKadGw25N4MZNhu</sub>

okay

### 2026-10-14 10:50:42 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsPHkUL1PiL6quUGJwpk48di</sub>

On it. First I'll trace Tennessee's lineup position by position, then run the controlled test.

### 2026-10-14 10:56:06 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs2aSEwHEbN95RGpybRD48yn</sub>

You're right, it's the physics. The engine values positions wrongly.

**The test:** two equal FBS teams played 1,000 games per line. Then one team got the same total upgrade (its lineup average +0.5), put in one place each time. I ran it on two different team pairs.

| Upgrade | Extra points per game |
|---|---|
| QB +10 | +0.1 to +0.3 |
| Offensive line +2 each | +0.2 |
| Defensive line +2.6 each | +0.9 |
| Secondary +2.6 each | +0.9 to +2.0 |
| Linebackers +5 each | +2.0 to +2.2 |
| Receivers and TE +2.6 each | +2.1 to +3.1 |
| RB +10 | −7.5 in one pair, +6.8 in the other |
| Play callers 18 or 4 (out of 20) | about 0 |

For scale, every starter +1 is worth about +4 points.

**What's wrong:**
- **A much better QB changes almost nothing.** In real football the QB is the most valuable player by far.
- **The offensive line barely counts.** Real lines decide a lot of games.
- **Receivers and linebackers count the most.** That's backwards from real draft value and real win impact.
- **One running back can swing a game by 7 points either way.** That's unstable, and it points to a bug in how runs resolve.
- **Coaching has no effect in games.** A great play caller and a poor one get the same results.

**How this makes dynasties (inferred):** the world rates a team by its plain average, but the engine rewards receivers and linebackers. A team stacked there wins far more than its average predicts. That would explain Tennessee winning 36 of 43 close games. Tracing Tennessee position by position would confirm it.

**Proposal:** fix the physics before tuning dynasties.
1. QB play: his accuracy, decisions and pocket play should drive passing results.
2. Line play: blocks should decide runs and pressure the way they do in real games.
3. The running back bug.
4. Coaching that shows in games (play calls, adjustments, fourth downs).

Then re-measure positions against real win values before running another trial.

Should I write this up as the fix list, or trace Tennessee first?

### 2026-10-14 10:56:06 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsVaAq1xY8ZbvUtUgUKKcU68</sub>

*[Progress checklist update]*

```text
Dynasty cause check

✓ Re-ran the 15-year world with per-game lineup and coach data
✓ Measured win chance by lineup gap and coach calling
✓ Built a controlled test: same boost put at different positions
**✓ Ran it twice (two team pairs, 1,000 games per line)**
○ Trace Tennessee's lineup position by position
```

### 2026-10-14 10:57:27 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs3YFirnktA93unBaCa3TvLS</sub>

trace tenessee first so we can fix borth

### 2026-10-14 10:57:32 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs1FnjMAAqz7ndjSfwrg7M5E</sub>

Got it. I'll record each team's starters by position for every season and trace Tennessee's close games.

### 2026-10-14 10:58:46 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsFSqQpgAur9XRzg8m4t1dL4</sub>

*[Progress checklist update]*

```text
Tennessee trace

✓ Added each FBS team's starters by position to the trial export
✱ Re-running the 15-year world (about 14 minutes)
○ Compare Tennessee's lineup with its opponents, position by position
○ Measure each position's weight across all FBS games
```

### 2026-10-14 11:03:56 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsCzm1do7cydE19gzhdZsm8J</sub>

*[Internal note between Claude sessions: Received a message from the coordinator]*

> Raichel added design input on coaching for the physics fix list (attached). In the chat I told them their view is right. The play call itself must matter through the battles: running into a +2 box loses because of the matchup, whoever called it. Coach quality then drives the calls whenever the user isn't picking, plus the things the user never controls: adjustments, prep and how well the scheme is installed. Fold that into item 4 of the fix list and keep tracing Tennessee first, as they asked.

*(attached project messages: cmsg_015NJfb6NRUve4Wa2buMtZKsHbB4xnxW3vi6G1XM8QBi53)*

### 2026-10-14 11:14:16 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsWs27qhYfZ8YiCWmpf1hpZQ</sub>

*[Progress checklist update]*

```text
Tennessee trace

✓ Added each FBS team's starters by position to the trial export
✓ Measured each position's weight across 10,644 FBS games
✓ Compared Tennessee's lineup with its rivals by position
✱ Re-running with depth, experience, kicker and coordinators (about 14 minutes)
○ Find what explains the rest of Tennessee's edge
```

### 2026-10-14 11:19:08 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsSs3fb96Gpj3aAW328U167W</sub>

are you still running it ? looks stuck

### 2026-10-14 11:19:16 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsNtTsXvydpQqPNHvygnQMEp</sub>

It's still running and not stuck. It's about 5 minutes into a 14-minute run, so roughly 9 to go.

This rerun adds depth, experience, the kicker and the coordinators. I need it because Tennessee's position mix explains only part of its edge.

### 2026-10-14 11:19:49 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsHdDx54E1jvJcJasVa45J6u</sub>

why is it saying this ?

*[Attached file: a screenshot of the app: "Pontificating 27s" and a branch bar with a "Create PR" button. Saved as [uploads/6a354a20-ae59-440e-8fb1-162f8890577b.png](../uploads/6a354a20-ae59-440e-8fb1-162f8890577b.png), original at [project-files/uploads/hearth/6a354a20-ae59-440e-8fb1-162f8890577b](../project-files/uploads/hearth/6a354a20-ae59-440e-8fb1-162f8890577b). Matched to this message by its upload time, so the match is inferred.]*

![a screenshot of the app: "Pontificating 27s" and a branch bar with a "Create PR" button](../uploads/6a354a20-ae59-440e-8fb1-162f8890577b.png)

### 2026-10-14 11:19:52 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsC28nY1KKsW3cL3KXfd839m</sub>

That bar means this thread's branch has changes that aren't on main yet. They're the trace tools: the per-game export and the dynasty test (+152 lines). I committed them so they aren't lost. You don't need to create a PR, so you can ignore the button.

"Pontificating" is just the app's word for "working". The world run is still going.

### 2026-10-14 11:28:32 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsA1TZ5gbgATE3wL1eAo4msb</sub>

*[Progress checklist update]*

```text
Tennessee trace

✓ Added each FBS team's lineup by position to the trial export
✓ Measured each position's weight across 10,644 FBS games
✓ Compared Tennessee's lineup with its rivals by position
✓ Added depth, experience, kicker and coordinators
**✓ Found what explains Tennessee's edge**
```

### 2026-10-14 11:28:39 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsY7z43MccHF3NCr5DmyqMRC</sub>

Found it. The biggest cause is depth, not the starters.

**What decides games in the world (points per rating point, all 10,644 FBS games):**
- Rotation players (the backups who get snaps): **1.33**. That's more than any group of starters.
- Receivers 0.67, defensive line 0.56, secondary 0.56, offensive line 0.48, linebackers 0.32, RB 0.27, **QB 0.20**, TE 0.11.
- Coordinators' play calling and the kicker: about 0.

**Tennessee:**
- Its starters are 7 to 13 points above the FBS average at every position. Its weakest spot is QB.
- Its rotation players are 9.9 points above average. That alone is worth about 13 points a game against an average team. The team rating ignores depth completely, which is why Tennessee beat its rating so often.
- Its "playing time" grade is F. It stacks talent on the bench, and the engine rewards that heavily.
- With depth counted, Tennessee's leftover edge drops from about 6 points a game to 3. That's about 2 standard deviations, so partly luck. I can't pin down the rest with the data I have.

**Why Georgia didn't do the same:** its depth was just as good, but its starters at WR and DL were a bit weaker, and the engine weighs those highly.

**The physics fix list:**
1. **QB play:** accuracy, decisions and pocket play should drive passing. Today a QB counts for almost nothing.
2. **Line play:** blocks should decide runs and pressure. Today the offensive line counts less than the receivers.
3. **Depth and rotation:** backups play too many snaps or tire starters too fast. Depth should matter (more on the lines, where real teams rotate), but not more than the starters.
4. **The RB bug:** one running back swings a game by 7 points either way.
5. **Coaching, by your rule:** the call itself wins or loses through the matchups. Running into a stacked box loses whoever called it. Coach quality decides the calls when you don't pick, plus adjustments, game prep and how well the scheme is installed.

Then measure every position's value against real football before the next trial.

Should I write this up as the spec for a build thread?

### 2026-10-14 12:14:24 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs4HjD3BRyYA7KwmWxtJpEvF</sub>

yes

### 2026-10-14 12:14:35 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKs16wWDXK8XSzA6q6trsPSPg</sub>

Got it. I'll write the engine physics fix list as a build spec in the repo now.

### 2026-10-14 12:16:28 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsQXY57KzSqpYEqsEnJpvovD</sub>

The spec is written and merged to main: [engine-physics-fixes.md](/mnt/project-files/design/engine-physics-fixes.md).

It covers the five fixes you agreed to: QB play, line play, depth and rotation, the RB bug, and coaching by your rule (calls win or lose through the matchups, with no flat bonus). It also lists checks to run before world trial 4. Those checks are my proposal and still need your OK:
- Measure every position's value against real football. The QB should be clearly first, and RB and TE near the bottom.
- Backups should count less than starters, and snap shares should fall inside real ranges.
- A play caller rated 18 against one rated 4 should clearly change the score.
- Runs into a stacked box should gain clearly fewer yards.

Do these checks look right? If they do, a new thread can build it.

### 2026-10-14 12:20:49 UTC · Raichel (member)
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsJ5rrg3pZ8JMFC4pou2af26</sub>

don't build push spec to  to git as next needed fix,

### 2026-10-14 12:21:05 UTC · Claude
<sub>id cmsg_015NJfb6NRUve4Wa2buMtZKsYQHBrHPCTsnZicrsMu1JYo</sub>

Done. The spec is in git on main, marked as the next needed fix, not built. Nothing gets built until you say so.
