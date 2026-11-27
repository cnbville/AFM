# Scheme sequencing: real targets

Research for section 12, coordinator-grade game planning (decisions section 35). Research only, never decisions: every size here is a target to propose, not an agreed number.

Measured 2026-11-24 on the NFL's 2022-2024 seasons (nflverse play-by-play, participation and FTN charting; `tools/research/real_sequencing.py`; the numbers in `data/real-world/sequencing.json`). Downs with the win chance between 5% and 95%. EPA a play; its noise is about 1.1 / sqrt(plays).

## 1. Direction: does running one way set up the other way?

Designed runs, by where the offense's last runs in the same game went (`run_location`).

| After | This run goes | Runs | EPA | Success |
|---|---|---|---|---|
| any | (all runs) | 35,130 | -0.027 | 41.8% |
| two the same way | the same way again | 3,843 | -0.024 | 41.4% |
| two the same way | the other way | 3,509 | -0.007 | 42.6% |
| three the same way | the same way again | 1,556 | -0.033 | 40.4% |
| three the same way | the other way | 1,349 | +0.001 | 42.4% |

- Going the other way after a streak gains a little: +0.018 EPA after two (noise about 0.026), +0.035 after three (noise about 0.041). The sign is the coaches' one; the size is inside the noise.
- Outside runs alone (`run_gap` end): no difference either way (+0.010 against +0.023 after two; +0.034 against +0.025 after three, noise about 0.07).
- The defense does react: in 11 personnel on 1st and 2nd and 5-10, the box holds 5.96 men after no runs in the offense's last 3 snaps, 6.12 after one, 6.17 after two and 6.25 after three.
- Proposed reading: real NFL defenses lean toward what they've seen, but a whole league's average set-up is worth hundredths of a point a run. A set-up pays when one defender really cheats, and the engine should get its edge from that defender's belief and Discipline (item 13), not from the streak itself. The league average is the check: the engine's "other way after three" gain should sit near +0.03 EPA, not +0.3.

## 2. Play action and the run game

- Play action on 1st and 2nd down, 10+ snaps into the game, by the offense's run share so far: under 35% run, play action +0.116 EPA against straight drops' +0.046; 35-45%, +0.081 against +0.055; 45-55%, +0.113 against +0.050; over 55%, +0.065 against +0.095 (mostly teams protecting a lead).
- Play action after the snap before it: after a successful run +0.099, after a failed run +0.062, after a pass +0.125.
- So running more doesn't make play action work better, and a run just before it doesn't either. This matches Baldwin (2018) and later public work. The fake's hold should keep reading the defender (Play Recognition, Discipline, Downhill) and his look at the backfield, not the run count.

## 3. Predictability (serial correlation)

Run rate on the next snap of the same drive, after a run and after a pass:

| Down and distance | After a run | After a pass |
|---|---|---|
| 2nd and 7-10 | 25.7% | 36.0% |
| 2nd and 4-6 | 46.1% | 48.7% |
| 2nd and 1-3 | 67.6% | 59.0% |
| 3rd and 1-3 | 51.9% | 44.9% |
| 4th and 1-3 | 61.2% | 48.2% |

- On 2nd and long real teams alternate: they run less after a run (Kovash and Levitt 2009 found the same over 2001-2005 and called it a mistake a defense can read). In short yardage they keep doing what worked. The engine's callers should show both habits, and a sharp defensive caller should read them.

## 4. Upsets: how often talent loses

Closing spread against the result, 854 games:

| Favorite by | Games | Favorite wins |
|---|---|---|
| 0.5-3 | 355 | 58% |
| 3.5-6.5 | 291 | 72% |
| 7-9.5 | 121 | 73% |
| 10-13.5 | 59 | 83% |
| 14+ | 28 | 96% |

- The margin lands 12.5 points either side of the line (standard deviation; Stern 1991 had 13.9). That is the luck item 15 holds to: the same game replayed on many seeds should spread about this much.
- The researched coaching band (`docs/research/engine-and-tactics.md`, section 7): best against worst play caller, same players, 3-7 points a game. A 7-point scheme edge turns a 14-point favorite (about 13% upsets on this spread) into a 7-point one (about 29%). Proposed reading for item 12: a complete outscheming is worth about the real band's top, 7 points, so a 2-star that outschemes a 5-star wins roughly two to three times as often as with even calls. A larger edge needs a real source first.

## 5. Coverages

EPA a dropback by the coverage called (participation, 2022-2024): Cover 3 +0.102, Cover 6 +0.083, Cover 1 +0.062, Cover 4 +0.057, Cover 2 +0.053, 2-man +0.043, Cover 0 +0.039, Cover 9 -0.027. By the targeted man's route and coverage (92 cells with 120+ targets): the best for the offense are the post against Cover 6 (+0.62) and the corner against Cover 3 (+0.55); the worst are the swing against Cover 4 (-0.21) and screens against Cover 2 and man (-0.07 to -0.08). Every cell is in the data file. The engine's EPA by coverage (`match_check`, "passing by the coverage called") is checked against it in the report.

## 6. Tracking data (Big Data Bowl)

- The competition's data is CC BY-NC 4.0: "You may access and use the Competition Data for non-commercial purposes only" (2025 rules page). Using only averages from it in a free, non-commercial game looks allowed; selling the game would not be. Kaggle is blocked from the thread machines, and downloading needs a Kaggle account that has accepted the rules.
- Slices by year (not all checked): 2023 the pass rush and pass blocking, 2024 tackling, 2025 pre-snap and motion, 2026 the ball in the air.

## 7. The series and counters library

105 entries (52 offense series, 35 defensive answers, 18 situations and evidence; $29.38 on Sonnet 5.5 with web search), each with the base play, the stressed defender, his tells, the answers, what decides them in the battles and their ratings, the defense's answer back, timing, levels, evidence, doubts and sources: `/mnt/project-files/afm/scheme-sequencing/library/` (one JSON a topic; `tools/research/series_library.py`).

## 8. The engine against real: coverages

EPA a dropback by the coverage called, the engine's NFL test league (400 games, seed 1, main 2930ba6) against real 2022-2024:

| Coverage | Engine | Real |
|---|---|---|
| Cover 0 | +0.081 | +0.039 |
| Cover 1 | +0.093 | +0.062 |
| 2-man | +0.119 | +0.043 |
| Cover 2 | +0.071 | +0.053 |
| Cover 3 | +0.214 | +0.102 |
| Cover 4 | +0.138 | +0.057 |
| Cover 6 | +0.208 | +0.083 |

- The order is close to real: Cover 3 and Cover 6 give up the most in both, Cover 0, Cover 2 and Cover 1 the least. The engine's dropbacks are worth about twice real's in the zone shells (its passing runs hot, a known gap), so the spread between coverages is wider than real too.

## 9. Opus as a coach on the engine's games (before section 12)

40 engine games (30 NFL, 10 FBS; `match_check` play-by-play, seed 1), one Opus 5.5 review each through the Batches API ($4.34). Raw results: `/mnt/project-files/afm/scheme-sequencing/review_before/`.
- Average scores: sequencing 3.6 of 10 (how much the calling showed set-ups, answers and adjustments), realism 4.0 of 10.
- 540 findings: 246 unreal (58 serious), 132 failed adjustments (56 serious), 84 tendencies, 78 missed answers. The run of them: defenses that keep a two-high shell and a four-man rush while a run gashes them; offenses that keep calling a run that fails; inside runs into a Cover 0 double-A blitz at the goal line, three times in a row; the clock barely running between some in-bounds snaps; a few throws landing past the end line; odd end-of-game kneels. The engine bugs are in `GAPS.md`.
- A second run, 60 games on seed 2 (20 NFL, 20 FBS, 20 high school; $6.40; `/mnt/project-files/afm/scheme-sequencing/review_before_2/`), gives the same picture at every level. Sequencing scores were 3.55 for the NFL, 3.4 for FBS and 3.25 for high school. Realism scores were 4.0, 4.05 and 3.7. It made 791 findings: 371 unreal (89 serious), 186 failed adjustments (84 serious), 119 missed answers and 115 tendencies. The most common serious one is an offense that keeps throwing into the same defender's zone after picks, or a defense that never changes its shell while a run gashes it.
- The same reviews on the same seeds after section 12 are the "after".

## 10. The blind drive test

500 pairs: one real NFL drive (2024) and one engine drive (NFL test league, 400 games, seed 1), each in the same plain format (clock, down and distance, spot, personnel and backfield, the play's kind, side and air yards, play action, the result), in a random order. Opus 5.5 picks the real one and says why (Batches API, $4.17; `coach_review.py blind`). 0.5 means it can't tell.
- **Realism score: 0.578** (289 of 500 right, noise about 0.022). Opus was sure (0.8+) and right on 26.
- A first run scored 0.61, but two tells were the converter's own (a recovered fumble shown as lost; "and inches" shown as 0 to go). Both are fixed; 0.578 is the baseline.
- Its tells on the drives it got right: play choice for the situation (239 mentions), the clock between snaps (199: in-bounds runs followed by a snap 3-5 seconds later), spots and distances that don't add up from fractional yards (62), throws past the end line (29), turnovers (19).
- Run it again after every engine lane on the same seed and real year: the score should fall toward 0.5.

## 11. What the engine needs (Opus's engine map)

One Opus 5.5 pass over the library mapped each entry to the engine's plays and ratings, what the engine already has, what's missing and an A/B test that would show it works (`/mnt/project-files/afm/scheme-sequencing/library/engine_map.json`). It grouped the missing pieces into 13 topics, a proposed order for section 12's design rounds (proposed, not confirmed):

1. Run-direction memory and defenders who lean toward a side (16 entries). Most run series depend on a defender learning a direction and then leaning that way before the snap or on his first step. The engine remembers run versus pass by look, but not by side. Its defenders never cheat toward a side.

2. Defender keys and the cost of a false step (13 entries). Play action, counter, cutbacks and eye violations all come down to what a defender reads (the guard, the triangle, the back, #2) and how far he goes the wrong way. The engine has play-action hold time. It does not have key types or a depth cost that must be undone.

3. Series links: base plays tied to their answers, and the defense's answers in series (29 entries). Coaches call the counter when a defender shows a tell, and the counter shares the base play's first beat. The engine has no links between base plays and their answers, and no defensive counter-series.

4. Read and conflict defenders: zone read, RPO, option, overhang, scrape exchange (11 entries). The offense leaves one defender unblocked, and that defender's first step picks the play. The engine has RPOs. It lacks a named read man with a step state, scrape or gap exchange as a defensive call, and arc, bluff or veer counters.

5. Gap-scheme blocking: pullers, kick-out vs wrap, spill vs box, doubles, split zone, trap (13 entries). Power, counter, trap, split zone and sweeps work through specific blocker-on-defender paths and edge techniques. The engine tracks blocks and sheds. Spill/box choices, puller timing, sift blocks and influence traps are not listed.

6. Self-scout by direction, down and concept, plus tendency breakers chosen on purpose (13 entries). Real staffs chart themselves and their opponents by down, distance, zone, personnel and direction, then break their own patterns on purpose. The engine self-scouts by look only, picks breakers by chance, and has no defensive self-scout.

7. Disguise and rotation timing; the QB's pre-snap and post-snap reads (12 entries). The shell the defense shows and the coverage it plays must be separate things, and safeties need real travel time. The engine leans its shells but does not model rotation timing or a QB being fooled by a shell.

8. Match rules and communication: banjo, switch, rat/robber, MOD/MEG, rubs (13 entries). Mesh, bunch, dagger and four verticals win or lose on pattern-match triggers and on whether two defenders communicate. The engine tracks busts but does not list trigger rules or rub path conflicts.

9. Pressure and protection: sims, creepers, fire zones, mugs, stunts, screens/draws, spies (12 entries). The real rush is hidden until the snap, protection is set from the shown rushers, and droppers or peeling rushers create the answers. The engine tracks the rush. It lacks shown-vs-real rusher counts, slide sides, a read of fake blocks, and rush lanes.

10. Pre-snap time: motion rules, shifts, unbalanced lines, tempo, substitution, audibles (8 entries). Motion and shifts force the defense to re-set, and tempo shrinks the time it has to do it. The engine has jet motion misfits and audibles. It lacks per-defender motion rules, a re-alignment clock and a substitution window.

11. Situations and field geometry: clock, red zone, goal line, backed up, hash, weather (12 entries). Many answers only work because of the end line, the sideline, the clock or the wind. These need geometry and clock state, not bonuses.

12. Adjustment budget: halftime and in-game changes (4 entries). Real coordinators change a few things after reviewing who got beaten, and more changes mean more busts. The engine adapts from today's EPA continuously. It has no halftime checkpoint and no change budget.

13. Calibration: talent vs scheme, upsets and coaching effect (4 entries). Every new battle must leave the right share of upsets and only a modest coaching edge.

Its warnings on the library, kept with the data: no source gives a rep count or a decay time for a set-up (every "2-4 reps then the answer" is a guess); a few entries still repeat the "run to set up play action" claim their own evidence contradicts; some numbers come from game sites, forums or AI-written pages and are not evidence; studies on screens against the blitz disagree.

## 12. The scheme duel test: the baseline on main (lane GP0, 2026-11-27)

The gate for every section 12 lane (`docs/design/game-planning.md`, item 16 and section 14). The tool is `duel_check` (`crates/afm-tools/src/bin/duel_check.rs`); it changes nothing in the engine.

**How it measures.** A sharp caller plays a poor one on the same games. Both coordinators (offense and defense) of a team carry the rating: every call skill (play calling, reading, situational, adaptation, creativity) at 17 for the sharp team and 5 for the poor one, style at the middle; the head coach stays average. The rosters are the level's synthetic teams, the sharp caller's at quality 0 and the poor caller's `delta` rating points a player better (0, 0.5, 1, 1.5, 2, 3, 4, 6). Every delta plays 2,000 games in two arms on the same seeds: both callers average (the ratings alone, the control) and the duel. The sharp team is at home on even games, schemes come from the level's synthetic league. The coaching edge is the duel's margin less the control's on the same games. A game replays to the same margin (checked on 40 games every run). Run: `duel_check <nfl|fbs|hs> [games] [seed] [out.md]` (seed 701 here; `AFM_DUEL_SHARP`, `AFM_DUEL_POOR`, `AFM_DUEL_DELTAS` and `AFM_DUEL_HEAD=1` change the arms). On 4 cores: 394 s at the NFL, 409 s at FBS, 285 s at high school.

**The switches for the lanes.** `AFM_DUEL_OFF=each` (or one name or a comma list) plays one more arm per mechanism with `AFM_GP_OFF` set to its name; the lane's own share is the edge with it on less the edge with it off. The names, in lane order: `direction`, `keys` (GP1), `series`, `selfscout` (GP2), `read_defenders`, `gap_blocking` (GP3), `disguise`, `match_rules` (GP4), `pressure`, `tempo` (GP5), `situations`, `adjustments` (GP6), `calibration` (GP7). A lane's engine code reads `AFM_GP_OFF` once at the start of each game (never cached across games) and skips its mechanism when its name is listed. An arm that plays exactly like the full duel is marked "not read": that is every one of them today (checked with `direction`).

**Gate for a lane (proposed, not confirmed):** the NFL's mean edge for the 17 against 5 gap inside 3 to 7 points once all lanes are in; each lane raises it, with its own share shown by its switch; the favorite's win rate stays near the real column below and the margin spread stays at 12.5; no delta's sharp team wins every game.

### What main gives today

| Level | Mean edge, 17 against 5 | Per point of gap | Range over the deltas | Inside 3 to 7? |
|---|---|---|---|---|
| NFL | +2.07 | +0.17 | +1.32 to +2.68 (each ± 0.34) | no, in every delta |
| FBS | +1.62 | +0.14 | +1.22 to +2.09 (± 0.40) | no, in every delta |
| High school | +0.73 | +0.06 | +0.36 to +1.16 (± 0.39) | no, in every delta |

The engine pays a 12-point gap in callers 2.1 points a game at the NFL, 1.6 at FBS and 0.7 in high school, under the real band of 3 to 7 for the whole range of coaches. The sharp team's win rate rises by 1 to 9 points over the control at the NFL, most where the rosters are close. The edge does not grow with the roster gap and sits inside the noise of each row, so read a lane on the mean over the eight deltas (about ± 0.12). High school pays least.

**The upsets, NFL (the control is the ratings alone):**

| Roster spread (points) | Real bin | Real favorite wins | Control | Duel (17 against 5) |
|---|---|---|---|---|
| 2.2 | 0 to 3.1 | 58.3% | 56.9% | 48.0% |
| 4.0 | 3.1 to 6.6 | 71.8% | 63.1% | 57.4% |
| 5.5 | 3.1 to 6.6 | 71.8% | 68.4% | 61.7% |
| 7.1 | 6.6 to 9.6 | 72.7% | 72.8% | 68.4% |
| 11.0 | 9.6 to 13.6 | 83.1% | 82.8% | 75.7% |
| 15.0 | 13.6 and up | 96.4% | 89.7% | 85.7% |
| 21.7 | 13.6 and up | 96.4% | 95.9% | 94.6% |

- **The margin spread is at real.** With average callers the margin sits 12.55 points around the roster's mean at the NFL (12.73 on even rosters), against the real 12.48 around the line. FBS reads 14.7 and high school 14.0; the real file has no figure for them.
- **The control tracks real in the middle and runs low at the ends.** At 4 points a favorite wins 63.1% (real 71.8%, 291 real games, about 2.4 standard errors off) and at 15 points 89.7% (real 96.4%, but only 28 real games, and a normal curve of 12.5 points gives 88 to 91% there). So the engine's upsets are not too many by the spread's own size; the real small-spread bins are the odd ones, and the lanes should not chase them.
- **The duel takes 4 to 9 points off the favorite's win rate in the bins up to 11 points**, so a sharp coach beats a better roster sometimes and never always (the sharp team wins 5.4% of the games where the poor caller's roster is 6 points better a player, 21.7 points on the field). That is the shape the spec asks for; its size follows the edge, so it will grow with the lanes.
- FBS and high school (same table, `fbs` and `hs` runs): the control's favorites win 56.5, 61.0, 68.0, 73.2, 83.0, 89.5 and 96.5% at FBS roster spreads of 2.3, 4.3, 6.9, 8.9, 13.6, 17.6 and 24.9 points; at high school 54.9, 59.6, 63.4, 69.7, 77.5, 84.5 and 93.5% at 1.6, 3.3, 4.8, 7.0, 10.3, 13.8 and 20.0 points. The duel takes 0 to 5 points off at FBS and 0 to 4 at high school. Each level's own real margins are not in the repo, so these are measures only.

**Why it is this size (not measured, for the lanes).** The callers act through the call table and the leans, the sharp team calls more of what beats the shell it sees, and nothing on the field answers a set-up (no direction memory, no keys, no series, no disguise): the rest of the ladder is the lanes' work, as the engine map's topics 1 to 12 say. This run sets the number they have to lift.
