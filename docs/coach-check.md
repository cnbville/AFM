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
- **The one cell that stands out** is game management for the NFL head coach who calls his offense in a sim: +0.93 points for, -0.88 against, +9.6 percent of wins (0.027). Game management is the rating with the widest reach (the clock, fourth downs and the caller's situational read). The other five seats don't repeat it, so it stays a maybe until the bigger run. (The bigger run: it did not repeat, see "Game day on the published world".)
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

The Coach check workflow runs `world`, `year` and `game` at 2,400 games an arm on the published seed 321 world (`world-2026-seed321-jan2026`). The first run (2026-11-25) finished the played year. Its other two parts stopped on two bugs, both fixed since, and ran again the same morning; all three parts are below.

What the first run found in the code:

- **A high school career ended the game in its first practice week at most schools.** The practice week read your seat's support departments, and a high school seat keeps its school where a college seat keeps its program. So the game read a college's dietitians for a school with a low number and stopped for a school numbered past the last college (1,181 colleges, most schools). The small world never showed it: its middle school's number is low. Fixed: a high school seat has no departments (`world/departments.rs`, `seat_who`).
- **The feats part stopped on a tied game** (the tool read a tie as a third team's win). Fixed in the tool: a tie is nobody's win.

### A played year on the published world: 400 days from 1 January 2026

Two runs, 44 minutes. The lifted teams' change against themselves in the plain run, then the untouched teams' change the same way (the noise).

| Level | Measure | Rating lifted | Change | Untouched |
|---|---|---|---|---|
| High school (3,520 schools an arm) | Growth, overall points a player (base 2.18) | Teaching | +0.076 (0.003) | -0.011 (0.002) |
| High school | Growth | Network | -0.012 (0.002) | |
| High school | Growth | Relationships | -0.009 (0.002) | |
| High school | Players signed by colleges (base 2.67) | Network | -0.070 (0.014) | -0.028 (0.015) |
| High school | Players signed by colleges | Relationships | -0.049 (0.015) | |
| High school | Players signed by colleges | Teaching | -0.040 (0.015) | |
| High school | Win share (base 0.49) | Teaching | +0.007 (0.001) | -0.002 (0.001) |
| High school | Win share | Network, relationships | -0.004, -0.001 (0.001) | |
| College (34 FBS programs an arm) | Growth (base 2.67) | Teaching | +0.028 (0.017) | -0.004 (0.020) |
| College | Growth | Recruiting, relationships | +0.019 (0.019), -0.034 (0.021) | |
| College | High school signees (base 30.6) | Recruiting | -0.15 (0.85) | +0.64 (1.01) |
| College | High school signees | Relationships | -2.68 (0.86) | |
| College | Signees' stars, summed (base 72) | Recruiting | +0.3 (2.2) | +1.6 (2.9) |
| College | Signees' stars, summed | Relationships | -6.6 (2.3) | |
| College | Players lost to the portal (base 20) | Relationships | +2.3 (1.1) | -0.4 (1.2) |
| College | Players lost to the portal | Recruiting, teaching | +1.1 (1.6), +0.9 (1.1) | |
| College | Win share, points margin a game | each | inside +/-0.03 (0.03), +/-0.9 (0.7) | +0.002 (0.032), +0.16 (0.61) |
| NFL (8 clubs an arm) | Growth (base 0.32) | Teaching, motivation, discipline | -0.036, +0.019, -0.037 (0.02) | -0.020 (0.028) |
| NFL | Win share | Teaching, motivation, discipline | +0.051 (0.046), -0.007 (0.048), -0.044 (0.018) | +0.000 (0.022) |

What it says, with the why:

- **A high school head coach's teaching is real, and the full world agrees with the small one**: 5 points is +0.087 overall points a player a year over the untouched schools (the small world: +0.087), 4 percent more growth, at about 25 errors. It reaches the field in the same year: +0.9 points of win share, six errors. Why: a high school has the smallest staff, so the head coach is the largest share of who teaches, and the stand-in reads the roster's overall.
- **A high school head coach's network and relationships do nothing in a year.** Growth sits on the untouched schools' own change, and no arm gets more players signed (network is -0.04 against the untouched schools, two errors the wrong way). Why: not traced yet. It is a gap: the one rating a high school coach has for getting his players signed shows no worth (in `GAPS.md`).
- **A college head coach's teaching is still not visible** (+0.03 against the untouched programs, one error) and the recruiting rating shows nothing at all (signees -0.8, stars -1.3 against the untouched programs, inside one error). Why for teaching: position coaches and coordinators do the teaching there. Why for recruiting: not known. With 34 programs an arm, a rating worth under about 2 signees or 6 stars a class can't be seen in one year; a second run would cut that by a third.
- **A college head coach's relationships went the wrong way**: 3.3 fewer high school signees (2.5 errors), 8 fewer stars (2.2 errors) and 2.7 more players lost to the portal (1.7 errors) than the untouched programs. The small world said the opposite (+0.9 stars at two errors). Fifteen college cells make one or two cells at two errors likely by chance, but three cells that move together are worth a second run from another morning before any lane sizes this rating. No why yet: it is in `GAPS.md` to trace.
- **Nothing an NFL head coach's rating does shows in a year.** Eight clubs an arm can only see a change of about 0.1 of win share. Discipline's -0.044 is 1.5 errors against the untouched clubs.
- **Untouched teams move too** (high school growth -0.011 at five errors): lifting a quarter of the coaches changes who signs where and who a player is read against, so the untouched column is the baseline and not zero.

### Game day on the published world: 2,400 games an arm

Each cell is the 5-point lift's worth a game, with its standard error in brackets. Wins are a share of games. Six times the small world's games, so a cell's error is about 0.2 of a point and 1.2 percent of wins. About 170 live cells were read, so eight or so at two errors are expected by chance; what counts is three errors, or the same cell at several seats. Dead in both ways at every seat: relationships, teaching, recruiting, evaluation, network, negotiating.

**High school head coach (he calls both sides).** At the base: sim 18.2-20.3, 45% wins; by hand 18.0-20.3, 45% wins.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| play calling | +1.42 (0.21) | -0.16 (0.29) | +0.045 (0.012) | +0.67 (0.23) | -0.48 (0.30) | +0.033 (0.012) |
| adaptation | +0.20 (0.20) | -0.04 (0.24) | +0.005 (0.012) | dead |  |  |
| game management | +0.19 (0.23) | +0.20 (0.23) | -0.002 (0.014) | +0.11 (0.22) | +0.03 (0.27) | +0.005 (0.011) |
| scheme | +0.81 (0.22) | -0.35 (0.27) | +0.030 (0.012) | -0.09 (0.22) | -0.05 (0.27) | -0.010 (0.012) |
| motivation | +0.20 (0.23) | -0.22 (0.24) | +0.013 (0.012) | +0.19 (0.22) | -0.10 (0.24) | +0.003 (0.011) |
| discipline | +0.59 (0.18) | +0.01 (0.25) | +0.017 (0.013) | +0.14 (0.21) | -0.05 (0.29) | +0.005 (0.011) |

**College head coach, his coordinators call.** At the base: sim 30.1-30.8, 49% wins; by hand 30.3-31.4, 48% wins. Dead here too: play calling, adaptation, scheme.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| game management | +0.25 (0.16) | +0.04 (0.18) | +0.005 (0.009) | +0.22 (0.16) | -0.15 (0.19) | +0.002 (0.007) |
| motivation | +0.17 (0.22) | +0.00 (0.24) | +0.002 (0.013) | +0.13 (0.25) | -0.22 (0.25) | -0.004 (0.011) |
| discipline | +0.28 (0.33) | -0.09 (0.30) | +0.000 (0.013) | -0.03 (0.26) | -0.27 (0.36) | -0.004 (0.013) |

**College head coach who calls his offense.** At the base: sim 28.8-31.6, 42% wins; by hand 29.3-31.1, 46% wins.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| play calling | +0.73 (0.29) | -0.22 (0.22) | +0.031 (0.013) | +0.27 (0.31) | +0.50 (0.27) | -0.015 (0.014) |
| adaptation | -0.01 (0.30) | -0.21 (0.27) | +0.029 (0.016) | -0.06 (0.31) | -0.53 (0.28) | +0.028 (0.013) |
| game management | +0.03 (0.27) | -0.48 (0.25) | +0.035 (0.012) | +0.33 (0.25) | -0.01 (0.27) | +0.011 (0.012) |
| scheme | +0.33 (0.30) | -0.31 (0.28) | +0.026 (0.012) | +0.28 (0.26) | -0.06 (0.34) | +0.015 (0.011) |
| motivation | -0.26 (0.25) | -0.42 (0.20) | +0.015 (0.010) | +0.11 (0.25) | -0.00 (0.25) | +0.004 (0.013) |
| discipline | -0.27 (0.29) | -0.36 (0.32) | +0.015 (0.013) | +0.17 (0.29) | -0.41 (0.31) | +0.015 (0.013) |

**College head coach who calls his defense.** At the base: sim 30.2-31.2, 47% wins; by hand 30.4-31.4, 48% wins.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| play calling | +0.31 (0.27) | +0.37 (0.28) | +0.018 (0.014) | +0.25 (0.29) | +0.13 (0.33) | +0.023 (0.013) |
| adaptation | +0.25 (0.30) | -0.32 (0.25) | +0.013 (0.012) | -0.02 (0.20) | -0.25 (0.20) | +0.004 (0.011) |
| game management | +0.15 (0.28) | -0.29 (0.25) | +0.015 (0.013) | +0.01 (0.21) | -0.19 (0.27) | +0.006 (0.012) |
| scheme | -0.10 (0.20) | +0.08 (0.23) | -0.011 (0.009) | -0.02 (0.16) | +0.04 (0.14) | -0.005 (0.007) |
| motivation | +0.02 (0.27) | -0.08 (0.21) | -0.001 (0.014) | -0.09 (0.23) | +0.06 (0.21) | -0.000 (0.009) |
| discipline | -0.00 (0.30) | +0.20 (0.24) | +0.005 (0.010) | -0.10 (0.27) | +0.08 (0.28) | -0.013 (0.012) |

**NFL head coach, his coordinators call.** At the base: sim 25.4-23.3, 57% wins; by hand 25.9-23.0, 61% wins. Dead here too: play calling, adaptation, scheme.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| game management | +0.28 (0.14) | +0.10 (0.14) | +0.003 (0.008) | +0.10 (0.13) | +0.17 (0.16) | -0.002 (0.008) |
| motivation | -0.13 (0.18) | -0.28 (0.16) | +0.011 (0.009) | +0.40 (0.22) | +0.03 (0.17) | +0.021 (0.010) |
| discipline | -0.09 (0.26) | -0.03 (0.22) | -0.001 (0.013) | +0.27 (0.27) | +0.50 (0.24) | -0.007 (0.011) |

**NFL head coach who calls his offense.** At the base: sim 25.8-23.5, 57% wins; by hand 26.3-23.4, 61% wins.

| Rating | Sim: points for | against | wins | By hand: points for | against | wins |
|---|---|---|---|---|---|---|
| play calling | +0.23 (0.20) | -0.19 (0.18) | +0.033 (0.013) | -0.11 (0.18) | -0.08 (0.20) | -0.007 (0.011) |
| adaptation | +0.12 (0.19) | -0.11 (0.20) | +0.023 (0.011) | dead |  |  |
| game management | -0.23 (0.22) | -0.28 (0.20) | +0.019 (0.014) | -0.02 (0.16) | -0.32 (0.21) | +0.005 (0.010) |
| scheme | -0.15 (0.19) | -0.33 (0.18) | +0.011 (0.011) | -0.03 (0.13) | +0.06 (0.12) | -0.005 (0.008) |
| motivation | +0.05 (0.18) | -0.07 (0.16) | +0.003 (0.008) | +0.13 (0.19) | -0.06 (0.17) | +0.006 (0.009) |
| discipline | -0.32 (0.22) | +0.01 (0.27) | +0.000 (0.017) | -0.12 (0.27) | -0.03 (0.23) | -0.009 (0.011) |

What it says, with the why:

- **Play calling is real when you call your offense and sim.** Five points are worth +4.5 percent of wins at the high school seat (3.8 errors), +3.1 in college (2.4) and +3.3 in the NFL (2.5): three seats, the same size, about five errors together. That is about four tenths of a win in a college season and half a win in an NFL one. Why: the caller's skill, reading and creativity all read it, on every offensive snap.
- **The high school head coach is the one seat where a rating moves the score by more than a point.** Play calling is +1.4 points for a game (seven errors), scheme +0.8 (3.7 errors), discipline +0.6 (3.3 errors). Why: he calls both sides, so his ratings fill both callers and the whole call sheet, and nobody on his staff covers for him.
- **When you call your college offense, game management, scheme and adaptation each show about +3 percent of wins in a sim** (2.9, 2.2 and 1.8 errors). One cell alone would be chance; three beside play calling at the same seat is a pattern. They are smaller in the NFL (+1 to +2 percent; adaptation at 2.1 errors, the other two inside two).
- **Calling your defense pays less than calling your offense.** No cell at that seat reaches two errors. Why: not traced. The defensive caller's ratings reach fewer parts of the dressed team (the tables above list the defense's sheet only).
- **With coordinators calling, no rating a head coach has clearly pays**, in college or the NFL. Game management, motivation and discipline reach the team; most cells sit at 0.1 to 0.3 of a point a game and none reaches two errors on wins in a sim (by hand, NFL motivation is +2.1 percent at 2.1 errors, one cell of twelve). This is the head coach most players will be.
- **By hand, almost everything is gone.** Play calling keeps +3.3 percent of wins at the high school seat (2.8 errors) and adaptation +2.8 percent at the college offense seat (2.2 errors), both through the call sheet your staff prepares, which the hand still plays from. Every other by-hand cell on wins is inside two errors but NFL motivation, above. Why: your call replaces the caller's, so only what the rating does before the snap is left.
- **The small world's standout did not repeat.** Game management for the NFL head coach who calls his offense was +9.6 percent of wins at 400 games; at 2,400 games on the real world it is +1.9 (0.014). It was chance, as the small world's note warned.
- **The design's size is still not there.** A tier 4 ability is 5 points of its rating. The best case today is +3 to +4.5 percent of wins, only for play calling, only when you call your offense, only in a sim. So lane RE3 needs a path of its own for each game-day ability, and above all for the by-hand column (proposed, not confirmed).

### The feats on the published world's history

Thirty seasons, the last 2024. Read from 146,913 engine games with a postseason flag and a win chance, 15,647 classes and 102 bonds at the top stage. A share is the head coaches at that level today who have met the feat anywhere in their career. Tiers are not built, so a share is the most an archetype could be held at. Head coaches have about nine seasons as a head coach at every level.

| Feat | High school (14,078) | FBS (136) | FCS (126) | NFL (32) | Section 10 wants |
|---|---|---|---|---|---|
| Mastermind (a top-tenth unit, or two in the top quarter) | 80% | 92% | 96% | 88% | 1 in 10 |
| Trail Boss (two classes first or second in the conference) | 0.4% | 29% | 18% | 22% | 1 in 10 |
| Standard Setter (five players at the bond's top stage) | 0% | 0% | 0% | 0% | 1 in 10 |
| Sideline General (five wins under 35% in two seasons) | 0% | 2.2% | 2.4% | 9.4% | 1 in 25 |
| Pro Factory (three picks signed at three stars or lower) | 0% | 35% | 2.4% | 12.5% | 1 in 25 |
| Builder (five postseason wins as head coach) | 40% | 37% | 48% | 34% | 1 head coach in 30 |
| Institution (two titles as head coach) | 3.8% | 29% | 34% | 22% | a handful a level |

Coordinators with Mastermind's feat: 59% in high school, 80% at FBS, 69% at FCS, 53% in the NFL. The median head coach meets Mastermind in his first season, Trail Boss in his 7th, Pro Factory in his 8th, Builder in his 6th and Institution in his 7th (FBS). At FBS, 58 head coaches meet two or more of the first five feats, 30 of them Builder's too and 21 Institution's as well.

What it says, with the why:

- **Only Sideline General is the size the design wants** (2 percent in college; 9 percent, three coaches, in the NFL). The high school zero is the tool's: high school games have no win chance to read. The NFL column's Trail Boss and Pro Factory are college feats met earlier in a career; their NFL and high school forms are not read.
- **Mastermind's feat is met by nearly everyone, in the first season.** Why: one top-tenth unit in any one season is the bar, and a head coach has two units a season for nine seasons. By chance alone that is 85 percent. It needs a bar that luck can't clear, such as top-tenth units in three seasons of five (proposed, not confirmed).
- **Builder and Institution are ten times too common.** Why: a bowl win and a conference title game count as postseason wins, and a conference title counts as a title, so nine ordinary seasons at a good program clear both. The top two archetypes need national or playoff marks, or a count that grows with the level (proposed, not confirmed).
- **Pro Factory is nine times too common at FBS** and right at FCS. Why: most draft picks were signed at three stars or lower, as in real life, so three of them is a matter of years at an FBS program. A rate per class would read development; a count reads tenure (proposed, not confirmed).
- **Trail Boss is two to three times too common in college.** Why: first or second of about a dozen programs, twice in a career of nine classes, is within reach of any top-third program.
- **Standard Setter can't be met.** The whole world holds 102 bonds at the top stage, and no coach has five. Why: the top stage is rare by design (`docs/design/coach-bonds.md`). The feat needs the second stage, or fewer players.
- **The stack is too easy**: 21 of 136 FBS head coaches meet every gate up to Institution. Section 10 wants a handful a level.
- **So every feat but one needs a new size before lane RE1 awards it.** These are Cain's to set; the sizes above are a starting point for that round, and once a feat's wording changes in the tool, `coach_check world` reads this world again in under a minute.

