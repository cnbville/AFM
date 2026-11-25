# coach_check: what your coach's ratings are worth today

Coach ratings lane RE0 (`docs/design/coach-ratings-earned.md`, section 12), built 2026-11-25. The tool is `crates/afm-tools/src/bin/coach_check.rs`. It changes nothing in the game. It measures what each of your ratings does today, so the design's sizes can be set on numbers, and it counts how many coaches meet each archetype's feat.

Every number below is from a small world (seed 331, scale 0.02, one year of history, games on the stand-in up to the day measured). It is a thin world: use it for what is live or dead and for rough sizes. The published world's run is the last section.

## The four parts

- `coach_check game [games] [seed] [world]`: game day. Your career starts at a middle team of each level and your staff runs the team to 10 October. Then your coach is set to 12 in every rating, and one rating at a time to 17 (a tier 4 ability is 5 points). The same engine games are played in both arms against the sixteen teams nearest yours. Six seats. Two ways of playing: "sim" (your coach calls and decides) and "by hand" (a caller of the level's typical skill calls both sides, the same in both arms, and the rest is your coach's). A rating that leaves your dressed team unchanged prints "dead". Every other row lists the parts of the team the rating reaches. `AFM_SEATS=nfl` plays only one level.
- `coach_check hand [games] [seed] [world]`: live games that stop on every snap. The same answers are fed to two coaches who differ in one rating, and the tool prints the play where the two games part.
- `coach_check year <world> [days=400]`: away from game day. The world is played on twice from the same morning, once as it is and once with one rating 5 points higher for every fourth head coach at each level (college: teaching, recruiting, relationships; high school: teaching, network, relationships; NFL: teaching, motivation, discipline). Each team is read against itself in the plain run. The teams nobody touched give the noise.
- `coach_check world <world>`: the feats of section 5, read on the world's own history: how many head coaches and coordinators meet each, and in which head coach season.

`<world>` is a world file or `new [scale] [history_years] [seed]`. The Coach check workflow (`.github/workflows/coach-check.yml`, by hand) runs the last three on a published world.

## What each rating reaches on game day

Read from the dressed team (`world/season.rs`, `coach_caller`, `your_calls`, `play_caller`; `world/hs_year.rs`, `hs_team`).

| Rating | When you call a side | When your coordinators call | By hand |
|---|---|---|---|
| Play calling | That side's caller: skill, reading, situational, creativity; how closely the call sheet is kept; at the NFL the install too | Nothing | Only the call sheet's stick (and the NFL install) |
| Adaptation | That side's caller's adaptation | Nothing | Nothing |
| Game management | The head coach's clock and fourth downs; the caller's reading, situational and creativity; the sheet's stick | The clock and fourth downs only | The clock and fourth downs, the stick |
| Scheme | The caller's reading, situational and creativity; the stick; at high school film study and two players' orders | Nothing | The stick (high school: film study, orders) |
| Motivation | The staff's motivation (talks) | The same | The same |
| Discipline | The staff's strictness | The same | The same |
| Relationships, teaching, recruiting, evaluation, network, negotiating | Nothing on game day | Nothing | Nothing |

Why: the engine takes a play caller per side. When a coordinator calls, your play calling, adaptation and scheme are not in the team at all. At your own high school you are both callers whatever you set, so the "who calls" setting changes nothing there (in `GAPS.md`).

## Sizes on game day: 400 games an arm

Each cell is the 5-point lift's worth a game, with its standard error in brackets. Wins are a share of games.

**High school head coach (he calls both sides).** At the base: sim 11.4-28.0, 20% wins; by hand 11.2-27.5, 17% wins.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| play calling | -0.17 (0.42) | +0.01 (0.65) | -0.020 (0.024) | -0.21 (0.43) | +0.09 (0.41) | -0.005 (0.017) |
| adaptation | -0.24 (0.39) | -0.28 (0.61) | -0.035 (0.022) | dead |  |  |
| game management | -0.32 (0.45) | +0.26 (0.66) | +0.000 (0.024) | -0.15 (0.39) | +0.60 (0.42) | +0.010 (0.014) |
| scheme | +0.78 (0.51) | -0.07 (0.52) | +0.005 (0.024) | +0.31 (0.52) | +0.75 (0.52) | +0.015 (0.018) |
| motivation | -1.11 (0.45) | -0.08 (0.52) | -0.033 (0.021) | -0.01 (0.46) | +0.50 (0.53) | +0.015 (0.019) |
| discipline | -0.47 (0.45) | -0.27 (0.63) | -0.020 (0.019) | -0.49 (0.48) | -0.21 (0.47) | +0.035 (0.019) |

Dead in both ways: relationships, teaching, recruiting, evaluation, network, negotiating.

**College head coach, his coordinators call.** At the base: sim 25.7-21.0, 60% wins; by hand 24.7-21.7, 54% wins.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| game management | +0.33 (0.37) | -0.22 (0.47) | +0.030 (0.022) | -0.22 (0.42) | +0.34 (0.37) | -0.003 (0.019) |
| motivation | +0.22 (0.59) | +0.31 (0.53) | +0.010 (0.024) | +0.43 (0.61) | -0.39 (0.66) | +0.022 (0.035) |
| discipline | -0.05 (0.61) | +0.81 (0.70) | -0.015 (0.027) | +1.22 (0.69) | +0.51 (0.68) | +0.050 (0.031) |

Dead in both ways: play calling, adaptation, scheme, relationships, teaching, recruiting, evaluation, network, negotiating.

**College head coach who calls his offense.** At the base: sim 26.1-21.6, 62% wins; by hand 25.2-22.1, 57% wins.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| play calling | -0.02 (0.58) | -0.50 (0.57) | -0.020 (0.031) | -0.03 (0.54) | -0.07 (0.59) | -0.020 (0.029) |
| adaptation | -0.24 (0.49) | +0.64 (0.38) | -0.018 (0.023) | dead |  |  |
| game management | +0.28 (0.60) | -0.29 (0.57) | -0.032 (0.030) | +0.30 (0.55) | -0.18 (0.46) | +0.003 (0.024) |
| scheme | -0.50 (0.64) | +0.16 (0.50) | -0.062 (0.027) | -0.39 (0.46) | -0.02 (0.34) | -0.022 (0.017) |
| motivation | -0.59 (0.47) | -0.11 (0.54) | -0.010 (0.023) | +0.60 (0.51) | -0.71 (0.58) | +0.000 (0.028) |
| discipline | -0.81 (0.63) | +1.19 (0.72) | -0.092 (0.035) | +1.04 (0.75) | +0.30 (0.54) | +0.008 (0.023) |

Dead in both ways: relationships, teaching, recruiting, evaluation, network, negotiating.

**College head coach who calls his defense.** At the base: sim 25.0-22.1, 57% wins; by hand 24.9-21.3, 56% wins.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| play calling | +0.05 (0.67) | +0.35 (0.51) | -0.033 (0.028) | +0.13 (0.45) | +0.18 (0.57) | -0.007 (0.021) |
| adaptation | +1.04 (0.53) | -0.75 (0.48) | +0.047 (0.025) | dead |  |  |
| game management | +0.91 (0.65) | -0.53 (0.46) | +0.032 (0.030) | +0.41 (0.46) | -0.28 (0.43) | +0.020 (0.023) |
| scheme | -0.45 (0.46) | +0.21 (0.39) | -0.030 (0.025) | -0.05 (0.27) | +0.14 (0.33) | -0.010 (0.012) |
| motivation | +1.01 (0.70) | +0.23 (0.61) | -0.005 (0.029) | +1.33 (0.56) | +0.29 (0.59) | +0.030 (0.026) |
| discipline | -0.19 (0.64) | -0.52 (0.59) | -0.003 (0.034) | +0.28 (0.60) | +1.07 (0.54) | -0.007 (0.031) |

Dead in both ways: relationships, teaching, recruiting, evaluation, network, negotiating.

**NFL head coach, his coordinators call.** At the base: sim 30.8-27.6, 56% wins; by hand 31.3-27.7, 60% wins.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| game management | +0.46 (0.32) | -0.17 (0.29) | +0.022 (0.017) | -0.22 (0.39) | +0.18 (0.30) | -0.002 (0.014) |
| motivation | +0.10 (0.57) | -0.52 (0.36) | +0.030 (0.026) | -0.65 (0.46) | -0.67 (0.48) | -0.010 (0.025) |
| discipline | -0.36 (0.61) | -0.18 (0.65) | +0.039 (0.029) | -0.72 (0.61) | -0.43 (0.66) | +0.005 (0.028) |

Dead in both ways: play calling, adaptation, scheme, relationships, teaching, recruiting, evaluation, network, negotiating.

**NFL head coach who calls his offense.** At the base: sim 29.8-27.6, 54% wins; by hand 30.9-27.2, 62% wins.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| play calling | +0.39 (0.55) | -0.54 (0.61) | +0.031 (0.031) | -0.51 (0.49) | -0.29 (0.54) | -0.016 (0.025) |
| adaptation | +0.49 (0.44) | +0.49 (0.40) | +0.013 (0.027) | dead |  |  |
| game management | +0.93 (0.55) | -0.88 (0.60) | +0.096 (0.027) | +0.32 (0.43) | -0.19 (0.51) | +0.001 (0.023) |
| scheme | +0.67 (0.57) | -0.01 (0.40) | +0.031 (0.027) | +0.23 (0.35) | +0.41 (0.41) | +0.002 (0.018) |
| motivation | +0.94 (0.51) | -0.64 (0.49) | +0.036 (0.031) | -0.13 (0.41) | -0.55 (0.56) | -0.009 (0.030) |
| discipline | +0.62 (0.54) | -0.10 (0.54) | +0.051 (0.029) | -0.52 (0.61) | +0.34 (0.65) | -0.040 (0.032) |

Dead in both ways: relationships, teaching, recruiting, evaluation, network, negotiating.

What it says, with the why:

- **No rating clearly pays.** With 400 games a cell's standard error is about half a point and 2 to 3 percent of wins. About 170 live cells were read, so a few at 2 to 2.5 errors are expected by chance, and that is what is there. A 5-point lift in any game-day rating is worth less than about a point a game on this world. Why: a rating reaches the game through the caller's mix of calls and a handful of decisions, and the engine's results come mostly from the players.
- **The one cell that stands out** is game management for the NFL head coach who calls his offense in a sim: +0.93 points for, -0.88 against, +9.6 percent of wins (0.027). Game management is the rating with the widest reach (the clock, fourth downs and the caller's situational read). The other five seats don't repeat it, so it stays a maybe until the bigger run.
- **By hand, adaptation is dead** and play calling and scheme keep only the call sheet's stick. Why: your call replaces the caller's.
- **The size the design wants is not there.** Section 2 pays an ability 1.5 to 5 points of its rating. Five points of the rating itself is under a point a game today. So lane RE3 can't get a felt reward from rating points alone: each game-day ability needs its own path in the engine (proposed, not confirmed).

## Live games: what the hand proves

Eight live games a rating, the same answers fed to both coaches.

| Rating | College head coach calling his offense | High school head coach |
|---|---|---|
| Play calling | same in 0 of 8; part on play 14 of about 187 | 0 of 8; play 6 of about 148 |
| Adaptation | 1 of 8; play 74 | 0 of 8; play 34 |
| Game management | 0 of 8; play 55 | 0 of 8; play 18 |
| Scheme | 0 of 8; play 27 | 0 of 8; play 3 |
| Motivation | 0 of 8; play 31 | 0 of 8; play 19 |
| Discipline | 0 of 8; play 3 | 0 of 8; play 2 |
| Relationships | dead | dead |

Why the games part even for adaptation: the staff's own call is still worked out on every snap before the hand's replaces it (`game/flow.rs`), and it draws from the game's random stream. A different caller shifts every later draw. So a live game can show that a rating is read, never that it pays, and the sizes by hand come from the table above. The stream is in `GAPS.md`.

## A played year: 400 days from 1 January 2026

The lifted teams' change against themselves in the plain run, then the untouched teams' change (the noise).

| Level | Measure | Rating lifted | Change | Untouched |
|---|---|---|---|---|
| High school | Growth, overall points a player (base 2.16) | Teaching | +0.082 (0.013) | -0.005 (0.009) |
| High school | Growth | Network | -0.029 (0.014) | |
| High school | Growth | Relationships | +0.006 (0.010) | |
| High school | Players signed by colleges (base 13) | Network | +0.12 (0.10) | +0.25 (0.09) |
| High school | Players signed by colleges | Relationships | +0.14 (0.09) | |
| College | Growth (base 2.37) | Teaching | +0.034 (0.024) | -0.004 (0.025) |
| College | Signees' stars, summed (base 2.6) | Recruiting | -0.44 (0.46) | -0.21 (0.30) |
| College | Signees' stars, summed | Relationships | +0.88 (0.47) | |
| College | Players lost to the portal (base 24) | Relationships | -0.85 (0.94) | -1.18 (1.05) |
| College | Win share | each | inside +/-0.04 (0.03) | +0.011 (0.032) |
| NFL (8 clubs an arm) | Growth (base 0.3) | Teaching | -0.018 (0.020) | -0.031 (0.026) |
| NFL | Win share | Motivation, discipline, teaching | inside +/-0.02 (0.03) | +0.022 (0.019) |

What it says, with the why:

- **A high school head coach's teaching is real**: 5 points is +0.08 overall points a player a year, about 4 percent more growth. It is the only clear effect in the year. Why: a high school has the smallest staff, so the head coach is the largest share of who teaches.
- **A college head coach's teaching is not visible** (+0.03 with an error of 0.02), and an NFL head coach's is not either. Why: position coaches and coordinators do the teaching there, and the head coach is a small share.
- **The recruiting rating shows nothing; relationships might** (+0.9 stars a class in two runs out of two, at about two errors). This world's classes are thin (the stars of a whole class sum to about 3), so the published world decides.
- **Nothing reaches wins inside a year.** Why: growth and classes take seasons to reach the field.
- **Untouched teams move too.** Lifting other coaches changes who signs where, so the untouched column is the real noise, not zero.

## The feats on a world's history

The small world has one season of history and no engine games, so its shares only prove the tool reads. For the record: Trail Boss 20 percent of FBS head coaches (16 percent at FCS), Mastermind 0 to 3 percent, Builder 19 percent and Institution 6 percent of NFL head coaches (from the NFL's own book of seasons), no Standard Setter, Sideline General or Pro Factory. Section 10 wants each elite archetype at about 1 coach in 10, each hybrid at 1 in 25, Builder at 1 head coach in 30 and Institution a handful a level.

Not read, because the world does not keep it by season: System Builder (who signed each starter), and the high school and NFL forms of Trail Boss and Pro Factory. Lane RE1 needs those stores or other feats (`GAPS.md`).

## What the tool can't see

- A career can end before October. The world fires your coach as it fires anyone: on this world three careers in a row at the NFL seat were fired on 6 October at 1-3. The tool then plays the same morning as another career. The firing itself is in `GAPS.md`.
- The by-hand arm models the hand as a typical caller. A better or worse hand is the player, not a rating.
- The year plays its games on the stand-in, which reads team strength and not your game-day ratings, so its win column only carries growth and classes.

## The published world (seed 321)

Not run yet. The Coach check workflow runs `world`, `year` and `game` at 2,400 games an arm on the published seed 321 world; its results go here.
