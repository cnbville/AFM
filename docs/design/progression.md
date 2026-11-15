# The progression update: projects, the body limit and the knack

Design done with Cain on 2026-11-14 (thread "Design the progression update"). Not built. The build comes after the individual practice build (`docs/design/individual-practice.md`), because both touch `development.rs` and `athlete.rs`. The sibling design, `docs/design/progression-everyday.md` (thread "Design split ceilings for all progression", 12 items, all Cain's yes on 2026-11-15), covers how the split drives everyday growth, the trims and the build checks; the two are built together.

Sizes come from a class-sim prototype (`/mnt/project-files/afm/progression/proposal.md`, `prog-probe.patch`, `prog_probe.rs`, `an.py`, `body_check.rs`; man coverage, seeds lbprobe, lbB, lbC). They are the agreed working values. The build checks them on a real world.

## Why (Cain's words)

- "previous proposed additions of +2.0 over 3 years in no way shape form give me the intrinsic need to take a risk on a player... It's taking a leap on a player and it either [helping] me or hurting me... How are we gonna make players feel the intrinsic need to take risks on players while maintaining a healthy ratio... i'm talking about adding that to the development. Making it worth taking risks." (15:22 UTC)
- "okay so we basically override ceilings? ... i mean technique ceilings" (15:47)
- On the body: "physical rebuilds happen all the time... speed is trainable but has limits. I do see the appeal of a lanky burner, trading minimal speed and then gaining a huge strength boost. Malleable raw athletes turned into kinda lab-grown diamonds. Understand the tight rope i want to walk with physical ceiling?" (17:37)

## What it does

- A four-year project on a raw corner (senior film man coverage under 45, tools well ahead of technique) pays off or burns you. With the knack (about 1 in 5) his best college season is +22 against the same kid without it: from 36 on film to about 70 instead of 47. Half reach 70+ and 1 in 5 reach 80+. Middling (about half): +15, 1 in 4 reach 70+. No knack (about 1 in 3): +6, the focus alone.
- Your lanky burner (6'3, 190, 4.42 at 18) on a body project: a big responder's strength ceiling rises about 14 into the top tenth of receivers. He carries about 15 lb more and runs about 4.47, and his contested catch room grows with the strength. A non-responder gains almost nothing.
- The world keeps its stars: AI staffs run projects too, and the one shared trim holds the count of 80+ men (sized in the sibling design).

## The agreed design (17 answers, all Cain's yes on 2026-11-14)

1. **Physical ceilings move too, each on its own leash** (17:41 UTC). A hidden body knack decides who responds (FAMuSS: 585 people on the same 12-week program gained 0 to 250% strength, muscle size -2% to +59%). The technique limits read his physical ceilings as they are now, not frozen at 14, so a body project opens room on contested catch, press and blocking.
2. **The leashes** (17:45). The most a body project adds to the ceiling:

   | Rating | Leash | Based on |
   |---|---|---|
   | Strength | about 7 for a typical responder (one spread of his position's strength: 7.1 for FBS-level receivers at 22), about 14 for a big one, close to 0 for a non-responder; his frame (height, arms, frame weight) decides how far | strength is the most trainable (Jacobson et al. 2013: bench max +34% for receivers and DBs, +18% for linemen over four years) |
   | Weight | up to 25 lb past where the weight room already takes him | college rebuilds |
   | Speed | 1 point at most; each pound costs time at the combine's rate (`body::forty_weight`, 0.0033 s a lb: about 0.05 s for 15 lb) | the forty is flat over college, skill players a bit slower by year four |
   | Burst | 1.5 at most | vertical and power gains only in years one and two |
   | Agility and hips | 2 at most (mobility work) | |
   | Balance | 3 at most | strength and base |

3. **Clues only, as vague staff words** (17:54). Neither knack is ever a number or a grade.

   | Clue | Hints at |
   |---|---|
   | How much he filled out in high school against his frame ("puts weight on easily", "hard gainer") | body knack, a fair hint |
   | The strength staff's read after his first offseason, from testing days ("took to the program", "hasn't moved much"); a better strength coach reads it sooner | body knack, sharpens with each testing day |
   | Coachability ("takes coaching", "set in his ways") | technique knack, a small part |
   | How fast his past focuses took | technique knack, once you've tried something |
   | The position coach's read a few weeks into a project ("it's starting to click", "still fighting it"); a better coach reads it sooner | technique knack, sharpens over weeks |

4. **Technique room** (17:58). The limit is what players with his tools reach (his tools' rank among the class at his position group mapped onto the class's ceilings), plus 4, plus the whole gap when that level is above his own ceiling. It never sits under his ceiling, so it only adds room and never caps a freak. Measured among college DBs: median room 9, p90 32, p99 51. A tighter limit (rank map plus 4 alone) gave a median of 6 and made a project on a raw corner worth +1 to +10, not a bet.
5. **The technique knack** (18:05): hidden, one per kind of skill (coverage, routes, blocking, rushing, passing mechanics, tackling), about a third from coachability (0.35 × coachability z + the rest his own). Has it about 1 in 5, middling about half, none about 1 in 3. Pace Φ(1.4 × (z − 0.3)).
6. **Positional logic: each limit reads his makeup too, about a third of the read** (18:09; Cain: "I don't mind a corner back with tackling trait every now and then but only if his toughness are up to par. Same goes for a lanky 6'3 WR. It won't make sense to do if his contested catch isn't up to par").

   | Skill | Body part of the read | Makeup part |
   |---|---|---|
   | Man and zone coverage, press | speed, burst, agility, hips | play recognition |
   | Tackling, shedding | strength, speed, balance | toughness, physicality |
   | Contested catch | strength, body control, jumping | catching (hands), competitiveness |
   | Routes, release | agility, burst, balance | football IQ |
   | Pass rush | burst, strength, hips, balance, arms | motor, competitiveness |
   | Blocking | strength, balance, agility, arms | toughness |
   | Passing mechanics | arm, balance, agility | composure |

   Catching itself stays a gift, as today. The sibling design proposes the full no-limit list and four more limited skills.
7. **A body project and a skill project can run in the same year** (18:13). The body side is the strength staff's time and the skill side is the position room's. The price is extra wear: more injury risk and less in the tank on game day.
8. **Skill projects per position room** (Cain's numbers, 18:15 and 18:17): QB 1, RB 1, WR 2, TE 2 ("a high conversion position. Big body WR & QBs often convert to TE"), O-line 2, D-line 2, LB 1, CB and nickel 2, safeties 1; kickers, punters and snappers none. A project takes the position coach's one-on-one minutes at the 60-minute focus, so the rest of the room gets a little less. Body projects are limited by the strength staff's size and quality, not the room.
9. **Size** (18:27): with the knack and a good coach a project year closes about a quarter of the room: room × 0.30 × knack pace × coaching (0.5 + coaching / 20) × U(0.7, 1.3). His rating grows toward the new ceiling at the 60-minute focus's 2x. Without the 2x the ceiling rose 20 but the rating only 11, so the bet never showed on the field.
10. **The click** (21:38): chance 0.25 × knack pace a project year (about 1 in 5 for a kid with the knack). His ceiling jumps 6 to 10, and half of it shows on the field within weeks.
11. **No age rule in school** (21:40; Cain: "I don't want it age related, make the rule Senior/RS Senior. To make sure a JUCO player or someone with bad luck/bad behaviour but then improved still has a shot"). Full strength through high school and every college season to his senior or redshirt senior year, whatever his age. Body projects follow the body's own clock (strength builds to about 24-25).
12. **The cost** (21:40): the focus's cost, a quarter of the year's growth in his other key technique skills, plus the focus's short learning dip in the skill being rebuilt.
13. **AI staffs run projects** on the same rules and room counts, picking from their own staff's read of his tools and the clues, never true values (21:45). How often depends on style: developer staffs use their slots most years and bet bolder, balanced about half, win-now rarely and only on kids already close. Cain first asked about a sandbox switch to turn AI projects off, then dropped it (21:47): "AI coaches can do it, i just want a teeny tiny edge". On the class sim AI projects at about 1.5 a 20-man room a year added about a fifth more 80+ cover men (60 against 49 over three classes) before the trim.
14. **High school coaches run projects** on the same rules, their coaching level and staff size setting what they get out of it (21:48). Class sim: about 1 in 20 DBs a year, 48 80+ cover men against 49.
15. **Your small edge** (21:49): AI staffs read the knack clues a bit less sharply (they pick more on tools) and keep a dead-end project going about a year longer on average. Everything else is the same as yours.
16. **Pro projects by pro season** (22:19): half strength in his first two pro seasons, a fifth in seasons three and four, none after, with the same room counts. Class sim (63 DBs who reached the NFL with 10+ room left, three classes): +7 median at his best pro season, 1 in 10 +25 or more; middling-knack kids gained most, since kids with the knack had mostly used their room in college. Small sample: measure again on a world.
17. **Converts keep their limits** (22:26). The limit comes from body and makeup, not position, so a receiver moved to corner, or a receiver or quarterback moved to tight end, keeps his room. His new position's skills start growing from the day he moves, from where they are, and his first season there is a learning year with the focus's dip. Today converts produce nothing (the late-bloom test: no LB, QB or WR moved to CB or S reached 70). Not measured: the build checks converts make it at a real rate, not a flood.

## Left to the sibling design or the build

Settled in `docs/design/progression-everyday.md` (Cain's yes to all 12 on 2026-11-15): everyday growth closing room, the limit lifting with natural rises, neglect, a falling limit, breakouts closing room at the click's size, the normal weight room inside the leashes, minor bug 385 in the same build, the no-limit list, four more limited skills, makeup that grows, one trim per skill family (2 to 5 points; this design alone needed 1.5 off man coverage ceilings of 65+), and a check that a hit project moves draft stock.

Still for the build:

- The screens: a project is chosen and followed on the player page's Development tab (the individual practice design's Focus and Plan sub-tabs), with the clues in Reports.
- World checks at the build: the star count per skill family, the bet outcomes by knack, AI projects by style, converts, pro projects (the full list with the sibling's in `progression-everyday.md`, "Checks at the build").
