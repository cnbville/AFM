# Research findings, in plain words

For each finding: what real football shows, what our game does now, and what it means for the game. Sources and full tables are in the repo under `docs/research/`.

**Is it real data?** Every real-football figure has a label:
- **[published]**: someone else's study or an official source (link in the research file).
- **[computed]**: Claude calculated it from raw real data (the dataset is named). The data is real; only the arithmetic is Claude's, and the script is in the repo so anyone can check it.
- **[estimated]**: Claude's own judgment or model on top of real data. Treat it as a reasoned guess.

"Our game now" figures come from our own trials and checks. Every "Means" line is Claude's reading of the data, and every "proposal" is Claude's idea, not data: it needs your yes before anyone builds it.

Area 1 is almost all **computed** from NFL tracking and play-by-play data (nflverse, which carries the NFL's own Next Gen Stats, Pro-Football-Reference snap counts and FTN charting) and college scores (CFBD). Only the PFF position values and the high school scoring are **published**, and only the play-caller figure and one cause are **estimated**. Area 2 is all **published**, except one date that is worked out from the rule.

## Area 1: how games should play

### 1. The quarterback should matter most, by far
- **Real:** [computed, NFL play-by-play 2021-25] a good starting QB (top 10%) is worth about **12 points a game** more than a poor one (bottom 10%). A running back is worth about 3 points, and a single receiver about 3.6. [computed, OverTheCap contracts] What teams pay agrees: top QBs earn 23% of the cap, edge rushers 16%, receivers 14%, and running backs 7%. [published, PFF's WAR study] PFF ranks the QB first by about six times the next position.
- **Our game now:** a QB who is 10 rating points better adds only about **0.2 points** a game.
- **Means:** physics fix 1 (QB play) has a clear target. A great QB should swing games by about 10 points; a running back should be one of the least important starters.

### 2. Backups should play the snaps real backups play
- **Real:** [computed, NFL snap counts 2021-25] starting linemen, the QB, corners and safeties play almost every snap. Defensive linemen and edge rushers rotate (the top one plays about two-thirds of the snaps). The lead running back gets about 70% of his team's back carries in a game, and about 59% over a season.
- **Our game now:** backups count for more than starters, and one college back took 92% of his team's carries.
- **Means:** these real snap shares become the target for physics fix 3 (depth and rotation).

### 3. Running into a crowded box costs yards, but not that many
- **Real:** [computed, NFL Next Gen Stats box counts 2021-25] on normal early downs, runs average 4.85 yards against 6 defenders in the box, 4.39 against 8, and 3.89 against 9. Losses or no gain go up from 15% to about 21%.
- **Our game now:** play calls barely change results.
- **Means:** your rule ("a run into a stacked box loses through the numbers") should cost about half a yard to a yard a carry. Harsher than that wouldn't be realistic.

### 4. Sacks come from pressure
- **Real:** [computed, NFL Next Gen Stats 2021-25] QBs are pressured on 28% of dropbacks. Almost all sacks happen on those plays (14% of pressured dropbacks end in a sack). Pressure turns a good passing play into a bad one.
- **Our game now:** too few sacks (4% of dropbacks against 6.8% in real games).
- **Means:** the fix is more pressure from line play (physics fix 2), not a separate sack boost.

### 5. Some routes beat some coverages
- **Real:** [computed, NFL Next Gen Stats routes and coverages 2021-25] the corner route works best against Cover 3, deep routes down the middle beat Cover 2, crossing routes beat man coverage, and curls die against man.
- **Our game now:** no check of this yet.
- **Means:** these become test checks. The engine should produce them through how players move, never through a lookup table.

### 6. Play-action works even without a good run game
- **Real:** [computed, FTN charting 2022-25] play-action adds about the same value for the worst and the best running teams.
- **Your tactics spec already says this.** The data confirms it.

### 7. A great play caller is worth points, but less than a great QB
- **Real:** [estimated] no public study has measured this, so Claude built its own model on 20 years of real NFL play-by-play, comparing coaches who had the same QB. A top coach (top 10%) against a poor one (bottom 10%) is worth about **7 points a game on offense and 5 on defense**. That's an upper limit, because it includes everything that comes with the coach.
- **Our game now:** a play caller rated 18 against one rated 4 changes about **0 points**.
- **Means:** coaching should clearly matter, by a few points a game, but never more than a great QB.

### 8. Home teams have a small edge
- **Real:** [computed, NFL results 1999-2025 and college scores] about **2 points** in the NFL today (it was 2.6 before 2010), and about 2 points in college conference games.
- **Our game now:** none.
- **Means (proposal):** add it through real causes like crowd noise and travel, not a flat bonus.

### 9. Scoring at each level
- **Real:** [computed, college scores 2014-25] college teams at every level score about 26-28 points a game, but blowouts grow lower down. 21% of FBS games are decided by 28 or more points, against 37% in D3. [published, MaxPreps' 5-million-game study] High school teams score about 26 a game, and the winner wins by about 26 on average.
- **Our game now:** too much offense (35 points a team in FBS in trial 3, and 32 in high school).
- **Means:** these become the scoring targets for each level.

## Area 2: real rules for 2026

Everything here is **[published]** (NCAA, NFL, court and news sources, linked in the research file), unless a line says otherwise.

### 1. College players now get 5 seasons in 5 years, and the redshirt is gone
- **Real:** the NCAA adopted this in June 2026, for the high school class of 2023 on. A court blocked it in July, but the block was paused on appeal in August, so the rule stands for now.
- **Our spec now:** the calendar still uses the redshirt (play 4 games and keep the year), and lists 5-in-5 as something that might happen later.
- **Means (proposal):** start the game with 5-in-5 in force, older players on the old rule, and keep the court case as a live risk.

### 2. NIL deals get checked
- **Real:** every outside NIL deal over $600 is reviewed. About 8% of deals, and 23% of the money, fail. Big booster deals fail most often.
- **Means (proposal):** a collective can't simply outbid everyone. Offers far above a player's market value can be blocked.

### 3. School pay to players grows each year
- **Real:** schools may pay players up to $20.5M in 2025-26, rising at least 4% a year ($21.3M in 2026-27). Football rosters are capped at 105, and the 85-scholarship limit is gone.
- **Means:** the yearly growth fits next to your agreed future changes.

### 4. The draft deadline question is answered
- **Real:** a player can pull out of the draft and keep playing college football if he does it within 72 hours of the NFL deadline (about January 17 in 2026 [estimated: worked out from the rule]), isn't drafted, and has no agent.
- **Means:** this closes the open question in the calendar spec.

### 5. NFL roster and cap rules for section 5
- **Real:** 48 of 53 players are active on game day. Teams can call up 2 practice squad players a game, and each player at most 3 times. Up to 8 players can come back from injured reserve (after at least 4 games out). Signing bonuses are spread over at most 5 years. Teams must spend 90% of the cap in cash over each multi-year period. The 2026 restricted free agent tenders run from $3.5M to $8.0M. The trade deadline is November 10, 2026.
- **Means:** section 5 can use these as they are.
