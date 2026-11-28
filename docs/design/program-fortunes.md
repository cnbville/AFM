# The rise and fall of programs (realism audit item 2)

Design, 2026-11-24. Thread "Design rise and fall of programs". Lanes P0, P1 and P2 built 2026-11-24. The measuring pass ran 2026-11-25 ("The measuring pass" below): it sized the bond's curve, two marks are still not met, and question 11 is open.

## Cain's answer (2026-11-24)

"agree on all": yes to questions 1 to 10 as proposed. The sizes (half the name from the last ~20 seasons, an 18 recruiter's bond about 3 times an 8's, the pass marks) are targets to tune on the measuring pass, still proposed, not confirmed until measured. Next: lane P0 (the measure), then P1 and P2 side by side, each in its own thread; the measuring pass states its cost and waits for Cain's yes.

## What it does in the game

- A coach makes an era. A great hire at Michigan State or Kansas State can turn a 4-8 program into a 10-win one in four or five years, the way Dantonio and Snyder did, and the program sinks again when he leaves or the next hire misses.
- A giant can fall. Texas with a bad hire signs worse classes and goes 7-6 for a few years, as real Texas did from 2010 to 2020. Nebraska's 1990s titles fade from what a 2026 recruit knows.
- A riser builds a name without a title, the way Oregon, Boise State and Cincinnati did, and big programs hire its coach away.
- Your own seat feels it: your record, your recruiting and your staff move a program's classes, so building a program is possible and leaving one hurts it.
- Long records look real: the best 30-year record near .850, not .896; nobody at .300 for 28 years; no Big Ten program 30 years without a winning season.

## What the world does today, against real

Real: every FBS team season with 8 or more games, 2001-2025, from the CollegeFootballData schedules (`data/real-world/scripts/program_persistence.py`). World: the seed 321 world, 1996-2025 (`docs/trials/realism-audit-seed321.md`, item 2).

| Measure | World | Real |
|---|---|---|
| Win share, correlation 1 year on | 0.60 | 0.54 |
| 5 years on | not read | 0.28 |
| 10 years on | 0.37 | 0.21 |
| 15 to 20 years on | not read | 0.18 to 0.20 |
| 25 years on (real: 24, only 116 pairs) | 0.30 | 0.10 |
| Strength, 1 year on (world: roster strength; real: a simple SRS from the scores) | 0.90 | 0.74 |
| Strength, 10 years on | 0.79 | 0.49 |
| Strength, 24-25 years on | 0.76 | 0.51 |
| Best long record | Texas .896 (358-41, 30 seasons) | Ohio State .847 (277-50, 25 seasons) |
| Long-time programs at .300 or under | 10 (28+ seasons) | 0 (worst New Mexico State .302) |
| No winning season in 22+ seasons | Michigan State (30), Kansas (28), Kansas State (25) | none (UL Monroe the only one with two or fewer) |
| Longest run without a winning season | 30 | 15 (Eastern Michigan) |
| Unbeaten seasons (10+ wins, no loss) a year | 1.13 | 0.84 |
| Longest winning streak | 65 | 34 (USC 2003-05) |
| Programs both in the bottom and the top quarter of FBS in some 5-year spans | not read | 27 of 118 (Baylor, Stanford, Nebraska, Houston, Memphis, SMU, Minnesota, Texas Tech and others) |
| 2001-05 top 10 by SRS still top 10 in 2021-25 | not read | 4 (9 still in the top 25) |

What the real numbers say: about half of a real program's strength is permanent (its state, money, name and conference) and half comes in eras that fade within five to ten years. In the world about three quarters is permanent. The world gets the one-year number nearly right and the long ones far too high, so the missing part is the eras, not single-season luck.

## The root cause

A recruit picks a school mostly on things that don't move (home ties, closeness, the name), and the name itself hardly moves either, so a program's talent depends on what it is and not on who coaches it. The head coach, who makes the eras in real football, explains 1 to 7% of the swings in a program's classes in the world against 29% in real life, so no coach can lift Michigan State and no bad hire can sink Texas; the loop from talent to wins to name then holds every program near its seeded place.

What I checked (the 2026-10-28 dynasty diagnosis, `afm/dynasty-layers/diagnosis.md`, measured on 11 worlds, and today's code on main `2930ba6`):

- **Static appeal decides 74 to 82% of blue-chip choices** before any visit or bond (diagnosis, round 2). Home ties are the biggest term, then the name. Item 8 (the name bends above 75% of the old leader) and the home fade were built since; they cut the top-3 lock-in from about 81% to about 61% (real 51%) but left the rest.
- **The name barely moves.** Tradition keeps 99.5% of itself each year and gains only 0.5% of recent form, plus titles (`world/staff.rs`, the yearly program update: `0.995 * p.tradition + 0.005 * p.recent`). Tradition, fans and facilities are 39% of prestige (`world/program.rs`, `prestige_of`), and fans and facilities follow tradition. So Nebraska's 1990s still sell in 2026, and a 15-year Oregon run adds almost nothing.
- **The coach doesn't change the school's appeal.** The pro pipeline grade counts the program's picks, not the coach's (`staff.rs`, the grades block). Coach prestige is his reputation only, and the prestige grades together measured near zero in choices.
- **The coach's bond is smaller than a recruit's own random feel.** 0.006 per point of the head coach's recruiting skill (`world/recruiting.rs`, the relationship block), so an 18 against an 8 is about 0.06; each recruit's feel for a school varies by 0.07 (`FEEL_SD`). Four times the bond alone moved nothing measurable (diagnosis, `coach4` worlds), because the bond only acts on the 20 to 25% of choices static appeal leaves open.
- **Transfers have no staff term** (`portal_day`).
- **Assistants follow the program.** A new head coach keeps each coordinator 40% and each position coach 50% of the time whoever they are.
- **Hiring follows the program too.** Head-job heat is 3 times the employer's prestige plus 4 a title in three years (`staff.rs`, `hotness`); a title adds a flat reputation scaled by program size (`coach.rs`, `title_reputation`). The 3-year rise in prestige counts (up to +3), but wins above what the roster should win don't. Coordinator searches read network, teaching and recruiting, never play calling (`coach.rs`, `candidate_score`).

What already exists and is not the cause: booster arrivals and exits (`program.rs`, the collective's year), NCAA cases with scholarship cuts and bowl bans (`ncaa.rs`), realignment (`finance/college.rs`, `realign`), firing on wins against the roster's expectation. These shocks are real; they just can't make an era on their own while talent ignores the coach.

### The same cause elsewhere

- **The portal:** transfers pick schools, never staffs (item 4).
- **The coach market and pay** (the G5 coach pay lane is running): a riser's coach should be priced by what he did above his roster. Items 6 and 7 feed it.
- **Ground and Capital:** a campaign's goals and the donors' fever read results; they will see bigger swings and need no change.
- **The power rating and odds** (`world/power.rs`): the preseason projection is fitted on the world's past, so it is refitted after the build.
- **Your seat:** the same rules make your own recruiting and record count, which is the point.
- **Saves:** item 1 keeps a short results history per program; it must be saved and loaded.

### Structural notes

- The world seeds tradition from real titles before 1996 and then has no way for an era to end. Any future fix that adds a fixed part (a bigger home pool, a bigger brand) makes this worse; the fixed parts are about right in size, the moving parts are too small.
- The world keeps no past final polls or year-by-year SRS, so persistence can only be read from season totals. P0 below reads it the same way on both sides.

## Lanes (agreed 2026-11-24)

- **P0, the measure.** A `program_check` page in the World Reader's book (or a tool on it) that prints the table above for any world from its season totals, with the real line beside it, plus the head coach's share of class swings and of margin at the 20 biggest programs (real 29% and 23%). Runs on the seed 321 book with no world build. It can start at once. **Built 2026-11-24** (`afm_read <book> program_check`, `crates/afm-reader/src/program_check.rs`, `docs/reference/how-it-works/book.md`); the real line is `data/real-world/program_persistence.json`.
- **P1, what a recruit reads** (items 1 to 4): the name from lived results, the coach's record in the pitch, the bond by recruiting skill, transfers and the staff. Built 2026-11-24 (thread "Let recent results drive recruiting"): `world/program.rs`, `lived` (item 1), `world/fortunes.rs` (`pipeline` and `head_record`, item 2; `bond_rate`, item 3; `transfer_bond`, item 4), unit tests in `src/tests/fortunes.rs`; how it works in `docs/reference/how-it-works/recruiting.md` ("Program grades, prestige and pipelines", "Relationships and visits", "The transfer portal"). Sizes it chose where the answer gave none, proposed, not confirmed: a coach's own picks count half in a draft from before he came; his record above his rosters reads at 2 (a season one win-share above reads like a win share one higher) over his last three head-coaching seasons; a head coach's bond 0.06 and a position coach's 0.03 a year at a skill of 8; a transfer's calls 0.35 of a year of the head coach and 0.75 of the position coach; an old recruiting bond 0.6 of a year of the head coach, a year of the position coach, fading 15% a year. The measuring pass sizes them.
- **P2, staffs and hiring** (items 5 to 8): the new coach's staff, the hot lists, title reputation, coordinator searches. Built 2026-11-24 (thread "Make staff hiring move programs"): a holdover is kept only on the new head coach's read above the best man a program like his can draw (`world/staff.rs`, `staff_worth`, `could_bring`); hot lists read the last three seasons against the roster (`hotness`, `HOT_ABOVE`; `world/coach.rs`, `above_three`); a title raises standing by the season against the roster (`title_reputation`, `title_above`); coordinator searches read play calling through the head coach's eye (`fill_opening`, `CALL_WEIGHT`). How it works: `docs/reference/how-it-works/staff-and-coaches.md`. Sizes proposed, not confirmed, for the measuring pass.
- P1 and P2 touch different code and can be built side by side with unit tests. Then one measuring pass sizes them together on four seeds against the real table, before and after. Its cost is stated and waits for Cain's yes before any world runs.

Pass marks, set before: win share 10 years on under 0.27 and 20 years on under 0.25 on every seed; strength 10 years on under 0.60; the best 25-year record under .870; at most two long-time programs under .330; no 20-year run without a winning season; unbeaten seasons 0.6 to 1.0 a year; 15 or more programs in both the bottom and the top quarter in some 5-year span; head coach's share of class swings 20 to 35%. Cain's NIL-era title call stays: 9 to 10 champions in 15 years.

Not questions yet: if the top still goes unbeaten too often after P1 and P2 (Texas-like records, the 65-game streak), the upset rate at the top is an engine question for its own thread. The athletic director's own skill at reading candidates waits for the AD layer design.

## The measuring pass (2026-11-25, seeds 341, 351, 361 and 371)

Thread "Measure program fortunes on four seeds", under Cain's sleep proxy. Every choice it made for him is under "Decided by proxy" below.

How it was read: full-size worlds with their games on the stand-in (`star_mix build`), one machine a world on GitHub (`.github/workflows/program-check.yml`), each read with `afm_read <book> program_check`. A stand-in world can't judge the win marks: its win share falls off far faster than an engine world's (1 year on 0.35 to 0.41, the seed 321 engine world 0.60, real 0.54), so unbeaten seasons, the best record and the win-share marks wait for an engine world. Roster strength and class hauls read the same on both (strength 1, 10 and 24 or 25 years on: 0.88, 0.78 and 0.77 here before the lanes, 0.90, 0.79 and 0.76 on seed 321).

### Before and after lanes P1 and P2, at the sizes they proposed

30 years of history, read over 2001-2025. Before is main before both lanes (`8f0ba00`).

| Seed | Strength 10 years on, before | after | Head coach's share of class swings, before | after |
|---|---|---|---|---|
| 341 | 0.78 | 0.78 | -1% | 4% |
| 351 | 0.78 | 0.77 | 1% | 6% |
| 361 | 0.79 | 0.76 | 5% | 3% |
| 371 | 0.77 | 0.77 | 4% | 2% |
| Real | 0.49 | | 29% | |

At those sizes the two lanes moved nothing that can be measured. Seven quarter-size arms on seed 341 (the bond at 4 times, heavier title and pipeline grades, a faster name, a faster recent grade, and all together) left the coach's share between -4% and 6%.

### Why the size of the bond was not the lever

Replays of one recruiting year from a saved world (`fortunes_ab`), the same recruits under each arm:

- The bond is the coach's rate times his time on the recruit. The rate was 3^((skill - 8) / 10), and FBS head coaches' skill is 15.5 with a spread (SD) of 2.1, so a coach one spread better built 26% more bond. His time on the same recruit is 1.4 in his own state against 0.85 outside it, 1.0 visited against 0.4 not, 1.0 an own target against 0.35. So where the recruit lives and whether he visits move the bond three times over, the coach's skill by a quarter.
- A bigger bond therefore made home count more, not the coach. At 16 times, 92% of signees changed schools, in-state signings went from 52% to 63%, the link between a program's name and its class fell from 0.78 to 0.24, and the coach's skill still read 0.05 to 0.18 against the change in his class.
- A steeper curve at the same average bond did what item 3 asks. With each point of skill worth 40% more bond (base 30) and the average bond unchanged, the coach's skill read 0.30 against the change in his class's best ten (0.09 before), with in-state signings and the name's link unchanged.

### Fitting the curve (seed 341, full size, 20 years, read over 2006-2025)

| Arm | Strength 1, 5, 10 years on | Win share 1, 5, 10 years on | Coach's share of class swings | Programs in both the bottom and top quarter |
|---|---|---|---|---|
| The lanes' sizes (base 3) | 0.88, 0.80, 0.76 | 0.35, 0.28, 0.27 | 6% | 11 |
| Base 10, same average bond | 0.89, 0.80, 0.77 | 0.39, 0.31, 0.30 | 4% | 14 |
| Base 30, same average bond | 0.88, 0.77, 0.70 | 0.39, 0.30, 0.19 | 14% | 15 |
| Base 30, twice the average bond | 0.87, 0.72, 0.64 | 0.45, 0.24, 0.17 | 11% | 21 |
| Base 30, four times | 0.86, 0.65, 0.54 | 0.48, 0.28, 0.18 | 23% | 19 |
| Base 100, same average bond | 0.87, 0.74, 0.65 | 0.41, 0.25, 0.16 | 18% | 16 |
| Base 100, twice | 0.87, 0.72, 0.63 | 0.46, 0.32, 0.19 | 17% | 26 |
| Real | 0.74, 0.55, 0.49 | 0.54, 0.28, 0.21 | 29% | 27 |

Base 30 at four times the average bond met every mark a stand-in world can read, but its Group of 5 won 5 of 20 titles (FIU three). Base 100 at the same average bond moved strength half as far, with no bigger bond for an average coach.

### The fitted sizes on the four seeds

28 years of history (30 ran to 335 of a job's 355 minutes), read over 2001-2025 like the worlds above. Fitted is base 100 at the same average bond: `BOND_BASE` 100, `HC_BOND` 0.0043, `PC_BOND` 0.0022 (`world/fortunes.rs`).

| Seed | Strength 10 years on: before | lanes' sizes | fitted | Coach's share of class swings: before | lanes' sizes | fitted |
|---|---|---|---|---|---|---|
| 341 | 0.78 | 0.78 | 0.70 | -1% | 4% | 13% |
| 351 | 0.78 | 0.77 | 0.71 | 1% | 6% | 15% |
| 361 | 0.79 | 0.76 | 0.67 | 5% | 3% | 9% |
| 371 | 0.77 | 0.77 | 0.69 | 4% | 2% | 16% |
| Real | 0.49 | | | 29% | | |

With the fitted sizes on the four seeds:

- Strength 5 years on 0.74 to 0.76 (before 0.79 to 0.82, real 0.55) and 20 years on 0.65 to 0.69 (before 0.75 to 0.79, real 0.49).
- Programs in both the bottom and the top quarter in some 5-year span: 21 to 26 (the lanes' sizes 14 and 15 on the two seeds read, real 27, the mark 15 or more). 5-year blocks that moved .150 or more: 21 to 23% (before 16 to 18%, real 36%).
- Win share 1 year on 0.42 to 0.45 (before 0.38 to 0.41, real 0.54): eras now last through a coach's years. The other win marks wait for an engine world.
- No Group of 5 champion in 112 seasons. The average coach's bond is unchanged, so in-state signings and the name's link to a class stay where they were (the replays above).
- A famous former player's name adds up to 3 points to his skill in his first season as head coach, which is now up to 4 times the bond (1.4 times before).

Two marks are still not met: strength 10 years on (0.69, the mark under 0.60) and the head coach's share of class swings (13%, the mark 20 to 35%). The fit moved strength a third of the way to real and the coach's share two fifths.

The stronger arm (base 30 at four times the average bond) on the same four seeds: strength 10 years on 0.53 to 0.61 and the coach's share 11 to 22%, but the Group of 5 won 3 to 5 titles in 28 seasons on every seed, and the best long record in FBS was Middle Tennessee's on seed 351 (.775) and Troy's on seed 361 (.752). A bond that large is worth more than the whole step between a Group of 5 school and a national name, so it was not taken.

### What is left (proposed, not confirmed)

The rest is not a size. A top recruiter's bond can only grow until it outweighs the step between levels, and that step is small in a recruit's read: about 0.5 between a Group of 5 school and a national name (from the terms of `utility_fit`, not measured), where base 30 at four times gives a head coach at 19 a bond of 1.8 against a typical 0.55.

**11. Should a coach's bond win battles between schools of the same standing before it can lift a school over a level?** A recruit weighs the bond in full against schools at or below the level of the best school recruiting him, and less against a school a level above.
- Example: a Group of 5 head coach at 19 fights Alabama for a 4-star. Today, with a bond strong enough to make eras at the top, he wins him, and his school wins titles. After, he wins nearly every fight with other Group of 5 schools and the weaker Power 4 ones, and builds the best Group of 5 class, as real risers do.
- In games: your recruiting rating decides the battles at your level; moving up a level takes wins, a name and a bigger job.
- My pick: yes, as its own lane with a measuring pass on these four seeds. It would let the stronger bond be used (strength 10 years on 0.58, the coach's share 16%).

Also still open: lanes P1's other sizes and P2's sizes stay as proposed. The quarter-size arms that moved the grades, the name and the recent grade moved no mark; P2's sizes were not tested on their own.

### Decided by proxy (2026-11-25, Cain asleep: "i'm giving you permission to make decisions for me als long as you note them down")

1. **Seeds**: 341, 351, 361 and 371 (331 went to the GM tenure lane).
2. **Stand-in games, on GitHub**: the pass ran on full-size stand-in worlds, one machine a world. An engine-game world of this length needs two jobs of about five hours each. The win marks wait for an engine world.
3. **Before**: main before both lanes (`8f0ba00`).
4. **Two workflows, run by hand**: `program-check.yml` builds the worlds and reads them, `program-collect.yml` prints their pages in one log.
5. **Test knobs**: sizes were read from an `AFM_TUNE` setting during the pass and taken out before the merge. The game reads no such setting.
6. **Smaller rounds for test arms**: quarter-size worlds and 20-year full-size worlds on seed 341; the last round at 28 years, not 30.
7. **The curve, not the bond's size**: an 18 recruiter's bond is 100 times an 8's (1.58 times a point), not "about 3 times". The typical FBS head coach builds what he built before.
8. **The careful arm**: base 100 at the same average bond, not base 30 at four times, which met the strength mark but gave the Group of 5 title runs.
9. **Stopped with two marks unmet** and wrote question 11, instead of pushing the size further.
10. **The other sizes stay as proposed**: no arm on them moved a mark.
11. **Merged on green**: sizing these lanes is what the pass was agreed for.

## Lane P3: a coach's bond wins at his level first (2026-11-26, question 11, Cain "yes")

Built 2026-11-26 (thread "Make recruiter pull win at its level"). How it works: `docs/reference/how-it-works/recruiting.md`, "A school's level holds a great recruiter's bond in check".

- **Cause**: a recruit's utility adds each school's bond with no look at who else is in the field, so a bond big enough to make eras at the top (base 30 at 4 times the typical bond: strength 10 years on 0.53 to 0.61, the coach's share 11 to 22%) also lets a Group of 5 head coach at 19 beat Alabama for a 4-star, and the Group of 5 won 3 to 5 titles in 28 seasons on every seed.
- **Rule**: the bond a coach builds up to a typical FBS head coach's rate (skill 15.5) is the same against every school. The part above it counts in full against schools at his own level and below and less against a school above. A school's standing is its level (Power 4 1.0, Group of 5 0.85, FCS 0.55) times 0.9 plus 0.1 of its name (capped where the name stops growing, `NAME_TIE`; the first arm read 0.6 plus 0.4, see below): 0.92 to 0.98 for any Power 4 school, 0.80 for the best Group of 5 name. A recruit's field is the five schools he likes best before any bond; a school keeps 1 minus the gap to the best standing in that field over 0.25 of its coach's bond above the typical one (from all of it to none). The gap closes as the school's name grows with its wins, so moving up a level takes years of winning.
- **Sizes taken** (proposed, not confirmed until measured): `BOND_BASE` 30, `HC_BOND` 0.0425, `PC_BOND` 0.0217 (the stronger arm, 4 times the typical bond), `LEVEL_STEP` 0.25, the field of five (`BOND_FIELD`), the standing 0.9 level + 0.1 name (`BOND_NAME`; 0.6 + 0.4 in the first arm).
- **A transfer's bond is unchanged** (the measuring pass's base 100 and sizes, `PORTAL_BASE`): the portal has no field of schools at the time a bond is built; a level rule there needs the suitors known first (`GAPS.md`).
- **Marks to read** (the pass's): strength 10 years on under 0.60 (real 0.49), the head coach's share of class swings 20 to 35% (real 29%), programs in both quarters 15 or more (real 27), and no Group of 5 champion run (the stronger arm: 3 to 5 titles in 28 seasons).

### First arm, measured 2026-11-27 (standing 0.6 level + 0.4 name; one arm, seeds 341, 351, 361 and 371, 28 years, stand-in games, read over 2006-2025)

`LEVEL_STEP` 0.25, standing 0.6 level + 0.4 name, field of five. Run 46000622749.

| Seed | Strength 1, 5, 10 years on | Win share 10 years on | Coach's share of class swings | Programs in both quarters | Group of 5 titles in 28 seasons | Marks met |
|---|---|---|---|---|---|---|
| 341 | 0.88, 0.75, 0.69 | 0.24 | 10% | 23 | 1 | 6 of 9 |
| 351 | 0.87, 0.75, 0.70 | 0.26 | 10% | 20 | 1 | 5 of 9 |
| 361 | 0.88, 0.77, 0.72 | 0.27 | 8% | 26 | 0 | 4 of 9 |
| 371 | 0.88, 0.76, 0.72 | 0.24 | 18% | 26 | 0 | 6 of 9 |
| Real | 0.74, 0.55, 0.49 | 0.21 | 29% | 27 | | |

- **The Group of 5 title runs are gone** (0 or 1 title in 28 seasons on every seed, against 3 to 5 on the stronger arm without the rule).
- **The strength and share marks did not move.** Strength 10 years on 0.69 to 0.72 (the fitted base-100 arm: 0.67 to 0.71; the stronger arm without the rule: 0.53 to 0.61; the mark under 0.60) and the coach's share 8 to 18%, mean 11.5% (fitted arm: 9 to 16%, mean 13%; the mark 20 to 35%).
- **Cause, from the rule's own terms**: a recruit's five favourites hold a national name (standing 0.94) for almost every 4-star and 5-star, so a great recruiter at a middling Power 4 school (standing about 0.80, a gap of 0.14 of the 0.25) keeps under half of his bond above the typical. The rule held the Group of 5, and it also held the Power 4 risers, who are the schools that make eras in real life (Utah, TCU, Baylor under Briles). The name term (0.4 of the standing) makes that gap large inside the Power 4.
- Not merged on these numbers: the rule added a mechanism and no gain on the fitted arm. Cain said yes 2026-11-28 to a second arm: standing 0.9 level + 0.1 name, so Power 4 schools fight each other with nearly the whole bond and only the Group of 5 and FCS are held against the level above. Its results follow.

## Questions

**1. Should the name a recruit reads come from the results he has lived through?** Tradition becomes half a slow all-time part (the titles and history it holds today) and half the program's results over about the last 20 seasons, recent ones weighing more (a season's weight halves in about seven years). Today tradition keeps 99.5% of itself a year and only titles add to it. This keeps the confirmed rule that tradition is a steady grade.
- Example: Nebraska, three titles in the 1990s and middling since. Today a 2026 recruit reads nearly all of that 1990s name. After, he reads about half of it, and Nebraska's name sits mid-Big Ten. Oregon's 2010-2014 run (two title games, no title) builds a national name; today it adds almost nothing.
- In games: recruits' "Tradition" and "Brand" grades drift over a decade; big names that stop winning sign worse classes ten years on.
- My pick: yes, sized so the real 10-year correlation (about 0.5 on strength) comes out.

**2. Should a coach's own record count in his school's pitch?** The pro pipeline grade reads the picks his staffs developed at every school he coached, not only this school's; title chances read his record above his rosters. His past counts most in his first seasons and fades as his record here builds.
- Example: Saban arrives at Alabama in 2007. Today the pitch shows Alabama's thin picks under Shula and his name adds a small bond. After, recruits read his LSU title and picks, and Alabama's classes go from the teens (Shula's 2003-06 averaged 13th to 41st) to the top 5, as they really did.
- In games: hiring a proven coach lifts your next classes; hiring a coordinator with no record doesn't, until he wins.
- My pick: yes.

**3. Should each coach's recruiting skill set how fast his bond with a recruit grows?** An 18 recruiter builds a bond about 3 times as fast as an 8 (the head coach on his targets, the position coach on his group), tuned until the head coach explains the real 29% of class swings at the 20 biggest programs. Today 18 against 8 is worth less than the recruit's own random feel.
- Example: two schools with the same name and the same home pull fight for a 4-star. Today the better recruiter wins about as often as a coin flip. After, he wins most of them.
- In games: your staff's recruiting ratings decide close races; a weak recruiter at a big school loses battles he'd win today.
- My pick: yes.

**4. Should transfers weigh their bond with the staff pitching them?** Built the same way as a recruit's, from contact and the coaches' recruiting skill. Today a transfer reads the school, its money and his room only.
- Example: a coach who recruited a player out of high school and lost him gets him back in the portal two years later more often than a stranger would.
- In games: the portal follows coaches, and a coach who moves jobs pulls some of his old players and targets with him.
- My pick: yes.

**5. Should a new head coach keep an assistant only when he rates him above the best man he could bring?** Today 40% of coordinators and 50% of position coaches stay whoever they are, so a staff the program paid for outlasts every head coach.
- Example: a new head coach at Texas finds a 9-rated defensive coordinator. Today he keeps him 40% of the time. After, he brings his own 14 unless the 9 is the best he can get.
- In games: a coaching change changes the whole staff, for better or worse, as real ones do.
- My pick: yes.

**6. Should head-job hot lists rank coaches by how far their last three seasons beat what their rosters should win?** Today they rank by the employer's prestige and titles, so the dynasty's own staff leads every big search.
- Example: a Group of 5 coach going 11-2 with a roster expected to go 7-5 tops the list. Today a coordinator at the champion with an average record of his own does.
- In games: risers lose their coaches to big jobs, and big jobs get the coaches who actually win above their talent (with the noise of a three-year read, so some hires still miss).
- My pick: yes.

**7. Should a national title raise a coach's reputation by how far his team beat its expected results?** Today it is a flat bonus that grows with the program's size.
- Example: a coach who wins a title with the most talented roster in the country gains less than one who wins it with the 12th.
- In games: reputation means skill more than address; coach prestige and pay follow it.
- My pick: yes.

**8. Should coordinator searches score candidates on play calling (scheme and game management), read through the hiring head coach's eye?** Today they read network, teaching and recruiting only.
- Example: a head coach with a good eye hires the 16 play caller over the 11 with the bigger network; one with a poor eye still misses sometimes.
- In games: a good coordinator lifts a program's margin, and good ones get hired.
- My pick: yes.

**9. Should the world's history run free, so its 2026 order is its own?** With real-sized eras, about 4 of the 1996 top 10 would still be top 10 by 2026 (real: 4, with 9 in the top 25). The other choice pulls the history toward the real 2021-2025 standings so 2026 opens close to real life (Ohio State, Georgia, Texas on top).
- Example: free, a world could open 2026 with Michigan State in a Dantonio-like run and Texas rebuilding. Pulled, it opens like the real 2026 preseason, but the eras in the last ten years of history are partly scripted.
- In games: free gives each world its own story; pulled gives the familiar real start.
- My pick: free. The big programs' fixed parts (their states, money and stadiums) still keep most of them near the top, as in real life.

**10. Should P0 start now and P1 and P2 be built side by side, then measured together?** P0 needs no world. P1 and P2 touch different code. The measuring pass costs world runs and waits for your yes with its cost.
- My pick: yes.
