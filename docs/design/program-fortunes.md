# The rise and fall of programs (realism audit item 2)

<<<<<<< HEAD
Design, 2026-11-24. Thread "Design rise and fall of programs". Lane P1 built 2026-11-24; P0 and P2 not yet.
=======
Design, 2026-11-24. Thread "Design rise and fall of programs". Lane P2 built 2026-11-24; the rest not built yet.
>>>>>>> origin/main

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

- **P0, the measure.** A `program_check` page in the World Reader's book (or a tool on it) that prints the table above for any world from its season totals, with the real line beside it, plus the head coach's share of class swings and of margin at the 20 biggest programs (real 29% and 23%). Runs on the seed 321 book with no world build. It can start at once.
<<<<<<< HEAD
- **P1, what a recruit reads** (items 1 to 4): the name from lived results, the coach's record in the pitch, the bond by recruiting skill, transfers and the staff. Built 2026-11-24 (thread "Let recent results drive recruiting"): `world/program.rs`, `lived` (item 1), `world/fortunes.rs` (`pipeline` and `head_record`, item 2; `bond_rate`, item 3; `transfer_bond`, item 4), unit tests in `src/tests/fortunes.rs`; how it works in `docs/reference/how-it-works/recruiting.md` ("Program grades, prestige and pipelines", "Relationships and visits", "The transfer portal"). Sizes it chose where the answer gave none, proposed, not confirmed: a coach's own picks count half in a draft from before he came; his record above his rosters reads at 2 (a season one win-share above reads like a win share one higher) over his last three head-coaching seasons; a head coach's bond 0.06 and a position coach's 0.03 a year at a skill of 8; a transfer's calls 0.35 of a year of the head coach and 0.75 of the position coach; an old recruiting bond 0.6 of a year of the head coach, a year of the position coach, fading 15% a year. The measuring pass sizes them.
- **P2, staffs and hiring** (items 5 to 8): the new coach's staff, the hot lists, title reputation, coordinator searches.
=======
- **P1, what a recruit reads** (items 1 to 4): the name from lived results, the coach's record in the pitch, the bond by recruiting skill, transfers and the staff.
- **P2, staffs and hiring** (items 5 to 8): the new coach's staff, the hot lists, title reputation, coordinator searches. Built 2026-11-24 (thread "Make staff hiring move programs"): a holdover is kept only on the new head coach's read above the best man a program like his can draw (`world/staff.rs`, `staff_worth`, `could_bring`); hot lists read the last three seasons against the roster (`hotness`, `HOT_ABOVE`; `world/coach.rs`, `above_three`); a title raises standing by the season against the roster (`title_reputation`, `title_above`); coordinator searches read play calling through the head coach's eye (`fill_opening`, `CALL_WEIGHT`). How it works: `docs/reference/how-it-works/staff-and-coaches.md`. Sizes proposed, not confirmed, for the measuring pass.
>>>>>>> origin/main
- P1 and P2 touch different code and can be built side by side with unit tests. Then one measuring pass sizes them together on four seeds against the real table, before and after. Its cost is stated and waits for Cain's yes before any world runs.

Pass marks, set before: win share 10 years on under 0.27 and 20 years on under 0.25 on every seed; strength 10 years on under 0.60; the best 25-year record under .870; at most two long-time programs under .330; no 20-year run without a winning season; unbeaten seasons 0.6 to 1.0 a year; 15 or more programs in both the bottom and the top quarter in some 5-year span; head coach's share of class swings 20 to 35%. Cain's NIL-era title call stays: 9 to 10 champions in 15 years.

Not questions yet: if the top still goes unbeaten too often after P1 and P2 (Texas-like records, the 65-game streak), the upset rate at the top is an engine question for its own thread. The athletic director's own skill at reading candidates waits for the AD layer design.

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
