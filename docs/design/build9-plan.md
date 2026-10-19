# Build 9: section 5 leftovers, sections 6 and 7, bug check, trial 9

Thread "Full build and trial 9", started 2026-10-19 21:21 from Cain's message: "start building leftovers from section 5, then design a plan to build 6 & 7 ... full build, then a bug check -> 50 year world trial -> fix found bugs -> build the fixes -> then get ready for building the game." Cain was asleep, so every fork was picked by Claude and is listed here (proposed, not confirmed).

Specs: `docs/design/team-management.md` (section 5), `docs/design/people.md` (section 6), `docs/design/scouting.md` (section 7), the fix list at the top of `docs/design/fix-list.md` (S6.1-S6.3, built with section 6 by Cain's ask), decisions in `docs/handoff/decisions.md` sections 13 and 14.

## Status

Steps 1-3 built 2026-10-19 (PR #59). The linebacker gap fits planned for S6.1 were tried and dropped (they lowered linebackers' tackles); combination blocks alone are built. S6.2 is partly fixed: NFL plays out of bounds 1.6% to 5.3% (real 10.7%); the rest is route and run geometry, listed for Raichel (`docs/match-engine.md`, "Fix list S6.1 and S6.2").

## The split

Sections 6 and 7 are mostly about people the world runs on, and partly about your own screens. As with section 5, the build splits three ways:

1. **Built now, the world and AI side:** every AI team, program and person runs on it, history included, and the trial measures it.
2. **With section 8, your own screens and choices:** everything that only exists because you click it (a talk, a press conference, your rulebook, your war room, your board, your assignments, your hires, your profile). The world side underneath is built now so section 8 only adds the screens.
3. **Later builds, not tonight:** parts that are big on their own and that the AI world can run without for now. Each is named below so nothing is lost.

## Section 5 leftovers (built first)

The section 5 build (PR #43) marked every topic built. Checked against the spec line by line, these real rules were still missing:

| Item | What is built | Where |
|---|---|---|
| Practice squad elevations | From 2020 a team short at a spot dresses up to two practice squad players a game; each player at most twice a season in 2020-21, three times from 2022 | `pro/squad.rs`, `elevate` |
| Protected practice squad and poaching | From 2020 each team protects its best four each week; the rest can be signed to another team's 53, with three weeks' pay guaranteed | `pro/squad.rs`, `poach` |
| Injury settlements | A player cut while hurt at cutdown is paid for the weeks he will miss, as dead money | `pro/squad.rs`, `settle_injury` |
| Power structure (topic 8) | Each club's final say: GM (most), head coach (about a fifth) or owner (the most meddling owners); a coach with the final say weighs scheme fit 1.4 times as much | `pro/office.rs`, `Power` |
| Other leagues | Players back from the CFL, a spring league or the UFL bring a season of real snaps into their development. Their games still don't run on the engine (later build: the CFL's own rules need engine work) | `pro/leagues.rs` |
| Fix list S6.3: bonus paid twice | A signing bonus is paid once, in the year signed; a restructure's bonus in its year; an extension's headline and guarantee count only new money (`Contract::new_from`) | `pro/money.rs`, `pro/market.rs` |
| Fix list S6.3: tag prices | Tags follow the real formula: from 2011 the five-year average of the top cap numbers as a share of the cap; before 2011 last year's top five. The old floor at 85% of the top five's market price is gone | `pro/market.rs`, `tags_and_tenders` |

## Section 6, world side (built now)

| Topic | Built now | With section 8 | Later build |
|---|---|---|---|
| 1 Personality | The 8 new traits on every person (ambition, loyalty, greed, temper, ego, leadership, professionalism, adaptability) with real correlations, drawn at 14; maturing from 18 into the early thirties; pros' contract goals read them; ambition joins declaring early; professionalism joins development and trouble | Your read in words | |
| 2 Relationships | Grudges (released or traded pros won't go back cheaply) and the pull of a coach he played for, in free agency | Your ties on player pages | Friends, mentors, families and sons as stored ties |
| 3 Morale | One mood and one trust in his coaches per player, built from role against his own view of himself (ego), winning, money, the room and life, with a dated reason; it moves development (up to 10%), the college portal lean, holdouts and trade requests, and retirement | Tells and your staff's read | Usage, noise and life parts at full detail |
| 4 Chemistry | Captains voted each year at every NFL team and college program; team mood from players' moods weighted by standing, damped by leaders; it moves development up to 5% | The room in words | Secondary and quarterback-receiver chemistry in the engine |
| 6 Life away from football | | | Pros' life events and absences, other sports |
| 7 Off-field trouble | The NFL gets trouble for the first time: real kinds and rates (about 2.5% a year), the personal conduct, substance, PED and gambling policies by era, suspensions with their dates, the commissioner exempt list, voided guarantees; college and high school trouble gets kinds | Your discipline call | Courts and appeals at full detail |
| 8 Your team rules | | Your rulebook and responses | AI team rules |
| 9 Media | | Press conferences | Reporters, stories, social media, NIL from followers |
| 10 Staff | Each NFL club and college program gets its named heads of strength, medical, analytics and (college) academics, rated, sized by budget and era; they take over the stand-ins: medical sets recovery, strength the physical side of growth, analytics film study, academics eligibility | Your staff screens | Every assistant as a person, high school staffs |
| 11 Staff market | Support heads leave, retire and are replaced each year | Hiring, interviews, blocking moves | Assistants' contracts, coaches' agents, permission rules |
| 12-15 Bosses, coaches' lives, job security, your profile | | Your side of all four | Bosses as people, coaches' families and health |
| Fix list S6.1, S6.2 | Run fits (combination blocks, linebacker fits) and plays ending out of bounds in the engine | | |

## Section 7, world side (built now)

| Topic | Built now | With section 8 | Later build |
|---|---|---|---|
| 1 Files per staff | Every NFL club keeps a file on each draft prospect: the looks it took (who, what kind, when). Its read of every attribute is rebuilt from the looks: each scout's lasting lean plus the noise of each look, narrowing with more looks (about 60% lean, 40% noise) | Ranges on your screens | |
| 2 Scouting staffs | Each club's department: a director, national scouts (cross-checkers) and area scouts, sized by era and the owner's budget, each with judging, potential, position, athletic and character reads, work rate, conviction and hidden biases (a body type, small schools, testing over tape, his own region) | | College personnel staffs, public evaluators |
| 3 The scouting year | Fall school calls by region at each scout's work rate, cross-checks of the top names, the director and GM on the very top, all-star weeks, the combine, pro days, top-30 visits and combine interviews | Your assignments | |
| 9 Boards | Each club's board comes from its own files (the shared consensus grade is gone from AI picks); tiers; a do-not-draft line for character | Your board and meetings | |
| 10 Pre-draft | The combine and pro days narrow each club's ranges instead of a flat shift | Your visits | Prospects' own choices, smokescreens, mocks |
| 11 Draft day | AI-to-AI trades on the clock from each GM's chart, his tier and rivals' needs, replacing the scripted quarterback trade | Your war room | Supplemental draft |
| 12 Undrafted rush | The bonus pool per club by era, offers from clubs that marked him, his choice by path to the 53, money and the people who called | Your calls | Minicamp tryouts |
| 14 Recruits | Every coach gets an evaluation rating; a college board's error comes from its staff's evaluators | Your recruiting board | Area recruiters, camps as looks |
| 5, 6, 7, 8, 13, 15-18 | | | Character and medical reads at full detail, potential words, look-backs, pro scouting, the portal and JUCO, the high school coach's side, the "my players" list |

## Memory

Cain plays on an 8 GB M1 and trial 8 peaked at 7.8 GB. New per-person data is kept small: the 8 traits are one byte each, mood and trust are 4-byte floats, a pro's grudges a short list, scouting files only the looks (a few bytes per club per prospect, kept for the draft class only). Trial 9 records peak memory.

## Order

1. Section 5 leftovers and S6.3, merge.
2. Section 6 world side and S6.1-S6.2, merge.
3. Section 7 world side, merge.
4. Bug check (a code read for bugs, not a world run), fixes merged.
5. Trial 9: one 50-year world (30 years of history, 20 into the future), timed, peak memory, Debug Lab and player pages at the same links.
6. Bugs the trial finds fixed and merged, checked by tests only. Tuning and realism findings go to Cain as a yes or no list.
7. Main green, and a note of what section 8 needs from this build.
