---
name: afm-trial5-answers
description: Raichel's 2026-10-11 answers to AFM calls 1-7 (all A, with added context) and the request for a 30-player trial; nothing to be built until talked through
metadata:
  type: project
---

On 2026-10-11 Raichel answered the seven trial-5 calls ([[afm-trial4-calls]]) with A on all, plus:
1. A, but first explain what's missing: "a highly ranked 5 star can come out to the NCAA but they rarely are on par"; the HS-to-NCAA gap feels too small (example: the late-blooming FCS edge vs the true-freshman safety, which looked realistic). Explain before adjusting anything.
2. A, plus a possible new layer: declare decisions should weigh a bad final year, which team would draft him (Dante Moore reportedly stayed rather than go to the Jets; Sam Darnold to the Jets as a cautionary tale) and NIL money that makes staying worth millions. "We'll talk before we implement."
3. A, plus a free-agent market that tracks released players so they can re-sign like the real NFL, and the same tracking for the NCAA portal.
4. A. 5. A. 7. A.
6. A, plus situational weights: a team with a great defense missing a generational CB or OLB reaches for that position.
8. New: the 10-player sample hides bugs; build a 30-player trial before the big decisions.
Raichel closed with "let's talk first". When Claude began checking how freshmen compare with college veterans, Raichel said "you're missing the point on 1". Raichel clarified: the late-bloomer FCS edge went from college level 81 to a 72 rookie, which looked too close to a pro starter. Raichel cited Giovanni Manu and Miles Frazier, college O-line stars who barely played. Raichel is happy with the FBS safety going from 95 to 76 (compared him to Caleb Downs).

Claude's explanation:
- The 81 was his start-of-year level. He then jumped 65 to 72 in his last college year, which only about 4% of draftees do, ending at 85 on the end-of-year scale (the safety ended at 94).
- FBS draftees at levels 84-89 start as rookies at about 69.
- What's really missing is a first-year adjustment at each step up.

Claude promised to show the end-of-season level on the cards. Decision card posted: adjustment year (recommended) / depth charts only / keep.

Measured on 2026-10-11 (seeds trial-4 and a, scripts in /tmp/claude-0/proj/t6/gap.ts):
- Five-stars arrive at FBS rated about 65-66 overall, against about 59 for the median upperclass FBS starter. 91% of them start as true freshmen.
- Four-stars arrive at about 58 and 41% start.
- Every star tier grows only about +7 over four college years, against +14 to +25 from 14 to 17.
- Draft-day median overall is about 70.

Later on 2026-10-11 Raichel decided the first-year adjustment: add it, but make it dynamic, not static. Some players handle the step up at once (Jaxon Smith-Njigba, Malik Nabers), and some UDFAs thrive "due to x or y". Raichel also said: "when we're running these trial and errors, you need to explain to me why so i can judge if it's within reason". So every trial card must explain why.

Claude proposed these drivers and asked whether the list looks right: step size, position, polish vs rawness, football IQ and make-up, physical readiness, fit and luck (including scouts underrating a player).

Proposals for 2, 3 and 6 are in /mnt/project-files/design/trial5-talk.md. Raichel chose the league sim with real teams and rosters (2026-10-11); talk through how it works before building. Claude is building the 30-player trial on the current code first.

**Why:** [[raichel-approves-design-decisions]]: answers are A, but Raichel wants to discuss details (1, 2, 3, 6) before anything is implemented.
**How to apply:** Discuss first; build nothing for calls 1-7 until Raichel confirms the details. Related: [[afm-realism-principle]]
