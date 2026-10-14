# Section 3, topic 8: the stats database (proposed, not confirmed)

This is Claude's full proposal for how stats work. Nothing here is decided until Raichel answers.

## 1. Where every stat comes from

- **The engine records every play as it happens.** Every player's position is tracked several times a second (topic 1). From that, the engine writes one play log entry per play: who lined up where, the play call, every battle and who won it, where the ball went, the result, and the why.
- **Every stat is computed from that log.** Nothing is invented after the fact. If the box score says a sack, there is a beaten blocker and a rusher behind it.
- **Tracking stats are measured during the play.** Separation, top speed, time to throw and pursuit distance are worked out while the play runs and saved as numbers. So they exist for every game, even where the raw tracking is not kept (see question 1).

## 2. What gets stored

| Layer | What it holds | Kept for |
|---|---|---|
| Play log | Every play of every game at every level: down, distance, formation, personnel, call, result, every battle outcome, tracking numbers, the why | Forever |
| Player game lines | Every stat below, per player per game | Forever |
| Season and career totals | Built from the game lines, tagged by level, team and competition strength | Forever |
| Full tracking | Every player's position several times a second, for the 2D replay | Your games and notable plays (question 1) |

## 3. The stats themselves

Every group has the classic box score stats plus the advanced ones. "Over expected" means compared to what an average player would do in the same situation, worked out from the play log.

**Passing**
- Classic: completions, attempts, yards, TDs, interceptions, sacks taken, passer rating, yards per attempt, adjusted net yards per attempt.
- Advanced: EPA per dropback (how much each play changed the expected points), success rate, completion % over expected, air yards, average depth of target, time to throw, pressure-to-sack rate, turnover-worthy plays, big-time throws, accuracy by depth and by area of the field, results against blitz and against each coverage, play-action and scramble splits.

**Rushing**
- Classic: carries, yards, TDs, yards per carry, fumbles.
- Advanced: rush yards over expected, yards before and after contact, broken and missed tackles forced, explosive run rate, stuffed rate, success rate, results by gap and by run scheme.

**Receiving**
- Classic: targets, catches, yards, TDs, drops.
- Advanced: separation at the catch, yards per route run, target share, air yards share, yards after catch over expected, contested catch rate, route-by-route results, results against man and against zone.

**Blocking (OL, TE, RB, FB)**
- Pass blocking: pressures and sacks allowed, pass block win rate (holding the block for 2.5 seconds), time held.
- Run blocking: run block win rate, blocks that sprung a run, penalties.

**Pass rush**
- Sacks, pressures, QB hits, hurries, pass rush win rate, time to pressure, double-team rate, win rate by move (speed, power, spin).

**Run defense**
- Tackles, tackles for loss, run stops, run stop win rate, missed tackles.

**Coverage**
- Targets allowed, completions, yards and passer rating allowed, separation allowed, passes defended, interceptions, yards allowed per coverage snap, results in man and in zone.
- Freelancing (topic 3): times a DB left his assignment, and how often it won or gave up a big play.

**Tackling**
- Solo and assisted tackles, missed tackle rate, yards after contact allowed, forced fumbles.

**Special teams (as agreed in topic 7)**
- Field goals by distance band (under 30, 30-39, 40-49, 50+), clutch kicks (pressure-weighted), extra points, kickoff distance and hang time.
- Punts: gross, net, hang time, inside the 20, touchbacks.
- Returns: yards and return yards over expected.
- Snaps: snapper errors.
- Coverage units: tackles, gunner wins.

**Team and unit**
- Points, yards, EPA per play on offense and defense, success rate, red zone and third down rates, explosive play rate, pressure rate, penalties, turnovers, starting field position, time of possession.
- Opponent-adjusted unit ratings, the way DVOA measures each play against the league average and the opponent's strength.

**Coaching and play calling**
- Pass rate over expected, 4th-down choices against the win-probability model, timeout and clock use, play-action and blitz rates, halftime adjustment (second-half against first-half EPA), and player development under each coach.
- These feed coach buzz.

**Player grades**
- Every player gets a grade on every play from the battle results, in PFF style (topic 1 already decides who won each battle).
- Grades roll up into game, season and career grades.

**Splits for everything**
- By quarter, half and score margin, and when trailing or leading.
- After a turnover (topic 4).
- In the red zone, on third down and in the two-minute drill.
- In weather (cold, wind, rain), at home or away, and against top-25 or ranked opponents.

## 4. Fair comparisons across levels and eras

- **One career record.** One player record covers high school, college and the NFL. Every season is tagged with level, state, class, conference and competition strength.
- **Percentiles among peers.** Every stat shows a percentile against the right peers: "98th percentile yards after contact among Kansas 1A running backs". This matches the level-relative ratings from section 1.
- **Opponent-adjusted versions.** A 2,000-yard season against weak schedules is shown both raw and adjusted.
- **Era adjustment.** Football changes. Passing yards rose a lot over the last 30 years in high school, college and the NFL. Records keep raw numbers, with an era-adjusted view next to them.

## 5. High school stats as reported (question 2)

- **The engine knows the true stats of every high school game.**
- **What you see is what a MaxPreps-style site shows.** Some schools report late or never, some pad numbers, small schools often don't report at all, and nobody keeps advanced stats for them.
- **Your scouts' film fills the gaps.** Good scouts log the true numbers and advanced stats for players they watch. That's another reason scouting staff matter.
- **Debug and trial tools always show the true stats.**

## 6. What the stats feed

- **Coach buzz (world item 5).** Unit stats relative to level, such as a high school coach whose team breaks the state points-per-game record, or an OC whose EPA is top 1% in his conference. This fades over 2-3 years.
- **Hidden-gem footprints (world item 9).** These are searchable flags for outliers relative to level, such as top 1% in yards after contact, top 1% in separation, or a testing outlier. Stats can lie: a weak schedule or a scheme that pads numbers can fake a footprint, and a bad team can hide one.
- **Scouting, recruiting, awards (all-conference, Heisman, All-Pro, MVP), contracts and agents.** Agents quote stats in talks.
- **Record books.** Per state, conference, level and all time, plus streaks. These feed the Hall of Fame.

## 7. How you use it in the game

- Leaderboards for every stat, with filters for level, state, conference, season, position, age and class.
- A custom search, such as "running backs, 2027, Texas 4A-6A, at least 150 carries, yards after contact per carry above 4".
- Side-by-side player comparisons.
- Career pages with every season at every level.
- Game pages with the box score, a drive chart, the win-probability graph and the play-by-play with its why.

## 8. Questions for Raichel

1. **What to keep.** Keep the full play log and every stat for every game forever, but full position tracking only for your own games and notable plays? Claude's pick: yes. All tracking numbers (separation, speed, time to throw) are still saved for every game; only the raw replay data is dropped.
2. **High school stats.** Show them as reported, like MaxPreps, with the truth only from your scouts' film? Claude's pick: yes.
