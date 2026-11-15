# Progression update: the split ceiling in all growth

Design done with Cain on 2026-11-14 and 2026-11-15 (thread "Design split ceilings for all progression"). Not built. It is the sibling of `docs/design/progression.md` (projects, the body limit and the knack) and is built with it, after the individual practice build.

Words used here: the **everyday ceiling** is today's per-skill ceiling (`Player.ceilings`). The **limit** is what his body and makeup allow (progression.md, items 1-6). **Room** is the limit minus the everyday ceiling. **Knack pace** is Φ(1.4 × (z − 0.3)) from progression.md item 5.

## Why (Cain's words)

- 2026-11-14 18:27 UTC, asking whether the split is for projects only: "okay because i was hoping to have it be for everything, but also i want a dedicated thread for that so we can focus on it's complimentary project/focus aspect".
- 21:37, before deciding: "first run a bigger test on other positions with crictical attributes, like agility + speed + finesse rush. For elite edge rushers with bad technique. One test per position group at least".
- 2026-11-15 00:02, on the 12 items below: "yes to all".

## The agreed design (12 answers, all Cain's yes on 2026-11-15 00:02 UTC)

1. **Everyday coaching closes room too.** Every player, every year, not only projects: the everyday ceiling rises by room × 0.10 × knack pace × coaching (0.5 + coaching / 20) × snaps (starter 1, rotation 0.8, redshirt 0.6, other 0.5). That is about a third of a project year. In school there is no age rule, as for projects (progression.md item 11). In the pros it follows the pro-season curve of progression.md item 16 (proposed, not confirmed: the item was asked with an age scale before the no-age rule was agreed).
2. **The limit never holds back today's natural rises.** Drift from work ethic and coaching, breakout lifts and veteran learning work as today. A natural rise past the limit lifts the limit with it. Why: about a third of college DBs already outgrow the limit set at 14 by a little (3 points at the 90th percentile), and those rises make today's stars. Holding them back cut 80+ cover men from 49 to 14 over three classes.
3. **Neglect lowers the everyday ceiling, never the limit.** A poor coach, being buried, low drive or trouble lower it as today's drift does. A better coach or a project later can win it back (the portal kid who blooms somewhere else).
4. **A falling limit never takes ratings away.** When his speed fades with age the limits that read it fall too. That only stops new gains; it never pulls a rating or an everyday ceiling down. The engine already punishes slow legs in the battles.
5. **A breakout year closes room.** Besides today's form and lift, a breakout closes room at the size of a project click (+6 to +10 to the ceiling, never past the limit), so a raw kid's breakout year is bigger than a polished kid's.
6. **The normal weight room works inside the leashes.** Every college and pro player's strength program moves his physical ceilings inside the agreed leashes (progression.md item 2), with his body knack deciding how much: a non-responder gains little, a big responder a lot. A body project adds on top. This is also what lets the strength staff read the body knack for everyone after his first offseason (progression.md item 3).
7. **Minor bug 385 is fixed in the same build.** Test days show real college strength gains (Jacobson et al. 2013: bench max +34% for receivers and DBs and +18% for linemen over four years, mostly in years one and two, the forty flat). Today a receiver's bench rises 2.4 reps from 18 to 22 against roughly +8 to +13 real.
8. **Skills with no limit, working as today:** catching (hands), ball tracking, arm strength, kick and punt power and accuracy, kickoff technique, trajectory, get-off, snapping, elusiveness, body control, touch, deep accuracy, ball security, ball handling, and every mental read (football IQ, play recognition, vision, anticipation, decisions, composure, concentration, communication, pocket presence, eye manipulation, disguise). Gifts and experience, not body.
9. **Four more skills get a limit**, besides progression.md item 6's table:

   | Skill | Body part of the read | Makeup part |
   |---|---|---|
   | Leg drive | strength, balance | toughness |
   | Pursuit | speed, burst | motor |
   | Blitzing | burst, agility | football IQ |
   | Throw on the move | agility, balance | composure |

10. **The makeup part of the limit grows with him.** It reads his makeup ratings as they are now (toughness, hands, football IQ and the rest), the same way the body part reads his physical ceilings as they are now.
11. **One small trim per skill family** off natural ceilings, so the world keeps today's number of stars with everything on (everyday closing, your projects, AI and high school projects). Class sim values with everything on: DT power rush 2; edge finesse rush, RB pass blocking, LB zone coverage, CB man coverage and QB footwork 3.5; WR routes about 4; OL pass blocking, TE run blocking and S tackling 5, each off ceilings of 60+. The limit keeps the trimmed points, so investment can win them back. The build sizes every family again on a world with the agreed AI room counts by style.
12. **A project that hits moves his draft stock.** The build checks that a hit moves his draft grade the way real life does (a raw edge who learns to rush climbs boards). If it doesn't, scouts' grades lean more on the skills that matter most at his position.

## What was measured

Class sim (`career.rs`, `simulate_cohort`), three classes of 250,000 (seeds lbprobe, lbB, lbC), on a local probe that was never committed (`/mnt/project-files/afm/progression/prog-probe-all.patch`, `prog_probe_all.rs`, `run3.sh`, `an3.py`; raw tables in `positions-test.md` and `all-growth.md` beside them). The limit is the prototype's, fixed at 14; the agreed moving body read and arms aren't in it.

**Everyday closing alone (man coverage, DBs):** 80+ 48 against 49 today, 70+ 489 against 439. A raw corner with the knack under a coach of 12+ gains +7.3 at his best college season (ceiling +12), middling +3, none +1.

**Elite tools, bad technique, one test per group** (college players with top-quarter tools for the skill and bottom-third technique on senior film; best college season, median, today → with a 4-year project; share reaching 80+ with it). Today 0 to 2 of each whole group reach 70.

| Group, skill | Knack | Middling | No knack |
|---|---|---|---|
| Edge, finesse rush | 48 → 78, 46% | 46 → 71, 17% | 48 → 55, 0% |
| DT, power rush | 44 → 80, 49% | 44 → 66, 10% | 44 → 51, 0% |
| OL, pass blocking | 42 → 77, 37% | 44 → 66, 10% | 43 → 52, 0% |
| WR, routes | 47 → 77, 38% | 48 → 69, 15% | 47 → 55, 0% |
| TE, run blocking | 46 → 80, 47% | 47 → 69, 12% | 46 → 55, 0% |
| RB, pass blocking | 41 → 80, 51% | 41 → 66, 10% | 43 → 52, 0% |
| LB, zone coverage | 50 → 85, 63% | 49 → 72, 27% | 50 → 57, 0% |
| CB, man coverage | 47 → 81, 56% | 49 → 70, 19% | 49 → 57, 1% |
| S, tackling | 43 → 79, 50% | 39 → 62, 9% | 44 → 48, 0% |
| QB, footwork | 40 → 66, 27% | 40 → 64, 19% | 41 → 47, 0% |

- Why today none get there: a technique ceiling is drawn at 14 from skill instincts that don't read the body (`athlete.rs`, `draw_shape`), so these kids top out in the mid 50s to mid 60s on their own.
- Why QB is weakest: his balance and agility rarely run as far ahead of his footwork, so the limit is lower (median 81 against 83 to 92), and quarterbacks sit more, so the rating trails the ceiling by about 8.
- Everyday coaching alone, same groups: knack +5 to +10, middling +3 to +5, none about 0.
- Why item 12: overall and draft stock barely moved on a hit (edge +3 overall, OL +6, LB +0.4; OL with the knack 0 to 6 drafted of 164). One skill is one part of the position overall, the project costs a quarter of the other technique skills' growth, and these kids are behind in those too. The class sim's draft is rough, so the build checks it on a world.

## Checks at the build

- The star count per skill family against today's 80+ and 70+, with every mover on, on a world; then the trims.
- Every skill in progression.md item 6 and item 9 here gets its own elite-tools-bad-technique test, as above, on a world.
- Bug 385 against Jacobson's gains; the freak check (`no caps on freaks`: no body read takes value from an off-typical body).
- Draft stock after a hit project against real (item 12).
- Everyday closing in the pros against the pro-season curve (item 1, proposed part).
