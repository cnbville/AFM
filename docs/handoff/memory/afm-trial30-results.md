---
name: afm-trial30-results
description: AFM 30-player trial (2026-10-11) on the trial-4 code: two new bugs found (stars on upside, backwards coachability), Raichel chose Fix both; coachability fixed, film line options pending
metadata:
  type: project
---

On 2026-10-11 Claude published the 30-player trial Raichel asked for ([[afm-trial5-answers]]): artifact version 6 at https://claude.ai/artifact/HjXeV3d1q74zH9AmpVPiSP. It has 15 cards picked by route and 15 random cards from seed trial-4, each with a story, "why" bullets and flags naming a call. The code is the uncommitted trial-4 code, with no design changes; [[afm-trial2-unpushed]] covers the code state.

Two new bugs were found. Both break rules Raichel already approved. A decision card asks to fix both (recommended), coaching only, or neither yet:
1. **Stars on upside.** Three-stars, two-stars and FBS offers have no film floor: the star score is 25% film + 75% projection, and the recruit score is 40/60. About half of three-stars are outside the top ~5,700 seniors, and FBS signs about 850 a class from outside the top ~13,800. Claude promised to bring Raichel a film line before changing it.
2. **Coachability backwards.** src/development.ts coachingBoost = coachingMult × (mult ≥ 1 ? lean : 1/lean). Below coaching 10, low coachability boosts growth (coachability 1 at coaching 9.9 gives ×1.55, at 10 gives ×0.64). Under HS coaching below 10, coachability 1-6 gains 11.4 points in HS vs 9.3 for 14-20, and those players are 2× as likely to be five-stars. Intended fix: 1 + (mult − 1) × lean. The bug dates from the first model commit.

Flags across the 30 cards: call 3 on 11 cards, new bugs on 6, call 1 on 5, call 2 on 4, calls 4 and 5 on 2 each. Card 9 is the FCS edge Raichel questioned.

The other R1-3 early-cut finding (4-7 of 64 R1-2 picks and 6-14 of 38 R3 picks cut within 2 seasons) belongs to call 3.

Card data with stories: /mnt/project-files/design/players30-final.json (page build scripts were in ephemeral /tmp/claude-0/proj/t7).

**Why:** Raichel wants to judge every trial result, and the fixes change results ([[raichel-approves-design-decisions]]).
**How to apply:** Fix nothing until Raichel answers the card. Show the stars film line as options first.

Update 2026-10-11: Raichel chose "Fix both". The coachability fix is done and committed locally (growth is now 1 + (mult - 1) x lean). The film line options for stars and FBS offers moved to a new trial-5 thread (Raichel wanted to save tokens), along with the league sim talk. Handoff: /mnt/project-files/design/trial5-handoff.md.
