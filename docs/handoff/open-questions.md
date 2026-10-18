# Open questions, unbuilt decisions and parked features

Written 2026-10-14 from the transcripts, for the move to the new account. It lists everything still waiting on Raichel, from every thread, with Claude's proposed pick where one was made.

How to read it:
- **Claude's pick** is what a Claude session proposed in the old project. Raichel has not answered it. Treat it as "proposed, not confirmed".
- **No pick proposed** means nobody suggested an answer. Where this file adds one, it is marked **handoff suggestion**: it was never discussed with Raichel.
- Pointers like ([T17] 2026-10-14 12:16) name the transcript and the message time (UTC). The link list is at the bottom.
- Nothing here is decided. Per Raichel's rule, ask before building any of it.

---

## A. Waiting on Raichel now, most urgent first

### 1. Build the engine physics fixes
- **Status:** the spec is on main (`docs/design/engine-physics-fixes.md`). Raichel: "don't build push spec to  to git as next needed fix," ([T17] 2026-10-14 12:20).
- **Claude's pick:** build it in a fresh thread once Raichel says so, then run world trial 4. This is the next needed fix.

### 2. The checks before world trial 4
- **Asked:** "Do these checks look right? If they do, a new thread can build it." ([T17] 2026-10-14 12:16). Raichel answered with "don't build", so the checks themselves were never confirmed.
- **The checks (proposed, not confirmed):**
  1. Measure every position's points per rating point against real football (WAR studies, EPA by position, contracts, draft value). The QB comes first by a clear gap, and RB and TE sit near the bottom.
  2. Rotation players are worth clearly less than starters, with snap shares inside real bands.
  3. A play caller rated 18 against one rated 4 clearly moves the margin, inside a real play-caller band. A good plan against a bad plan does the same.
  4. Runs into a +2 box gain clearly fewer yards than runs into a light box, inside real splits.
  5. `match_check` (NFL, FBS, HS) and `cargo test --workspace --release` still pass.
  6. Then world trial 4: 15 years with every college game on the engine, checking dynasties and droughts.
- **Claude's pick:** yes, as written.

### 3. World Trial 3 findings ([T17] and `docs/trials/world-trial3.md`)
Raichel took finding 1 first ("Let's start with the first dynasties", 09:16), and it became the physics fix. Findings 2-9 never got a call. The physics spec lists them as "not in this spec".

| # | Finding | Claude's pick or note | Status |
|---|---|---|---|
| 1 | Tennessee won 9 of 15 titles | Cause traced: depth counts more than starters, QB and line barely matter, RB bug, no coaching effect | Physics fix agreed, not built |
| 2 | Boston College, Iowa State and Wake Forest had no winning season in 15 years | Candidate: blowout share and margin. Likely the same engine cause as 1 (inferred) | **Handoff suggestion:** re-measure in trial 4 after the physics fix |
| 3 | 6 Black FBS head coaches in 2025 (real 16) | "Needs a trace before any fix" (the cause was not found) | Pick: trace first |
| 4 | Pick 1 is a QB 45% of the time (real 73%); round 1 CB and EDGE off | No pick proposed. Raichel's standing rule: position value on every first-round pick, intertwined with need | Waiting |
| 5 | Three-stars 4.6% drafted (real 6.0%), two-stars 1.1% (real 2.1%) | No pick proposed. Inferred cause: star ratings track true ability too closely | Waiting |
| 6 | Too much offense in college (35 points, 62% passing), low sacks and INTs, short kick returns, no home edge | Known engine gaps. Cause: college calls from NFL tables; `quick_game` has a neutral crowd | Waiting |
| 7 | Unreal stat lines: lead back 92% of carries, a kicker with 94 sacks | Backs' rotation is part of physics fix 3. Kickers playing pass rusher is not in the spec | Partly in the fix |
| 8 | Kickers, punters and QBs too athletic; broad jumps long | No pick proposed. Cause: the same athletic spread for every position | Waiting |
| 9 | 46.5% of players share a full name | Candidate: bigger name pools | Waiting |
| 10 | Career stats record and high school games on the engine | Still unbuilt (see B) | Waiting |

### 4. The name question from world trial 2
- **Asked:** "I need one call from you: the name data says Trey is mostly white (371 of 407). Is that wrong?" ([T11] 2026-10-13 13:50). Also noted: DeShawn is in neither data source, so no player has that name.
- Raichel answered other points and never this one. The trial 3 fix list later reweighted names.
- **No pick proposed.** **Handoff suggestion:** ask together with finding 9 (repeated names).

### 5. World realism questions 2-7
Raichel answered question 1 ("Agree buddy", [T11] 2026-10-13 08:03): all real programs, plus D3, NAIA and the real JUCO programs in a data pass. The real JUCO swap is still not done: `world/recruiting.rs` has placeholder JUCOs.
- **Q2, high school classifications and playoffs** ([T11] 2026-10-13 09:20). Claude's pick: "use each state's real playoff format for each classification. That means the real field size (Texas 6A takes 4 teams per district, Florida uses rankings, and so on), the real number of rounds, and the real private-school rules... It needs a research pass on 51 rows, with one source each." Unanswered.
- **Q3-Q7, never asked:** every college level (D3, NAIA), conference realignment, the coaching carousel, NFL franchise traits, shifting talent geography. By Raichel's ultra-realism rule the answer is the realistic version, but the details still need design (world.md section 0 and follow-up answer 7).

### 6. High school games and the history
- **Confirmed rule:** one engine for every game at every level, including every high school game (section 3, ground rules).
- **Open:** Claude proposed "High school is the one exception, at about 70,000 games a year. It stays on the quick model until the engine is much faster." ([T15] 2026-10-14 08:05). Raichel's answer covered college and the 30-year history but did not address high school.
- **Claude's pick:** high school stays on the stand-in until the engine is fast enough, then moves to the engine.

### 7. Engine speed
- Raichel asked what speeding up the engine means ([T15] 2026-10-14 08:10, again 08:12 in [T00]). The answer: only the code gets faster; the same seed must give exactly the same game, play by play. "If a speed-up would change any result, I won't do it without asking you first." ([T15] 08:12)
- **Open:** when to do it. A 30-year world takes about an hour on 4 cores. Claude's pick: before the first playable version.

### 8. Plays and tactics: research and build timing
From `docs/design/plays-and-tactics.md`, open items (proposed, not confirmed):
- **Research still to do:** the real gap between the best and worst play-callers in points per game; play-action and box-count splits; concept-against-coverage results; real playbook sizes per level; practice-hour rules per state; the 2026 scheme mix per level.
- **Where the build goes:** the coordinator placed it with schemes (section 5) and game day (section 9). Ask Raichel.

### 9. Calendar leftovers (`docs/design/calendar.md`, open items)
- The latest date a player can withdraw from the NFL draft and keep college eligibility. Unverified; check with a source.
- The chance of each of the 12 agreed off-field rule changes. Not set (proposed, not confirmed).
- Where the calendar build goes in the order. Ask Raichel.

### 10. The attribute list and positions
- **Decided 2026-10-10:** about 35-40 FM-style attributes ("B") and about 20 positions ("C"). Claude promised: "Before I build anything I'll send you the full list of about 35–40 attributes to sign off." ([T02] 2026-10-10 20:17)
- **Never sent.** The code still has 25 attributes (`attributes.rs`) and 13 positions (`positions.rs`).
- **Answered 2026-10-18 for the attributes:** Raichel agreed to 67 visible and 5 hidden attributes (`docs/design/attributes.md`), built the same day. The positions are still 14 (LS added); the ~20 positions are still open.
- **No pick proposed** for the positions. **Handoff suggestion:** draft the full list (release, hands, ball skills, leadership, football IQ and the like, per section 3's needs) and the ~20 positions (C/G, DE/DT, MLB/OLB, FS/SS, slot CB, FB, long snapper) for Raichel's sign-off before section 5.

### 11. The player-facing potential read
- `potential_outlook` (tier words, confidence, hints) is a placeholder until section 7's scouting design. The hint rule from Q18 is confirmed; the wording and tiers are not.

### 12. Player-model items Claude picked that Raichel never signed off
From `docs/decisions.md` "Still open":
- **The first-year adjustment drivers:** step size, position, technique polish, football IQ and make-up, physical readiness, scheme fit, luck. Raichel agreed to a dynamic adjustment, not to this list.
- **How strong the NIL pull back to school is,** and **whether a player knows roughly which team would draft him** (Raichel raised Dante Moore and the Jets).
- **Trial 6 details** (first-year dip sizes, portal rates, dismissal splits, declare weights, draft value by position, kicker board, league background players, camp, practice squad, landing spot, retirement ages): Raichel said to pick the most realistic option. Listed in `docs/decisions.md`.

### 13. The decision list from 2026-10-10 that was never fully answered
`docs/design/player-model-decisions.md` items 6-13 were never answered directly. What settled each one later:
- **6. How much potential can move:** settled by the breakout rule (5b, 5c) and the volatility feedback.
- **7. What drives development:** partly settled (coaching, coachability, background, playing time). The full list is open.
- **8. How often development runs:** not settled. The calendar (topic 7) puts training blocks in the year and spring breakouts in spring; the detail is section 6b.
- **9. Ageing:** partly settled by retirement by position, running QBs fading early and Derrick Henry outliers. The base curve is Claude's.
- **10. Injuries:** settled by Q39 and section 3 topic 6.
- **11. The pipeline:** settled by Q9 and Q13 (a full simulated world).
- **12. Real-number tests:** settled by section 3 topic 10 (development-only checks).
- **13. The old draft PR #1:** it was later merged (A: keep it and change it to match the answers).

### 14. Flags from earlier world trials never called
- **World trial 1, flag 2:** the "title chances" grade looks generous. Never called.
- **World trial 2 flags:** all 12 parked as trial 3 candidates ("no list them up for potential trial 3", [T11] 2026-10-13 15:16). Items 1-15 of that list were built (merge 0c4c536). The list is in `project-files/design/world-trial3-candidates.md`.

### 15. Claude's own calibration picks (proposed, not confirmed)
Raichel has not signed these off. Each list names its numbers:
- `docs/world-model.md`, "Proposed, not confirmed": spin-up of 6 years, roster caps, playoff fields, firing patience, the 4% brother rule, placeholder JUCOs, 15 JUCO out-of-state offers, Pioneer and Ivy aid, the .400 firing rule, the stand-in's 4-in-5, and the trial 1 list picks. Also the "(proposed)" numbers in its trial 3 fix list section, and the linked-units weights in the dynasty fix.
- `docs/match-engine.md`, "Choices Claude made": sprint model, gaps narrowing below 60, throw error, the QB leading receivers, carriers at 95% speed, block whiffs (~22%), tempo 37.5 s (33 for Air Raid), the 4th-down curve, screen timing, kick contain, goal-line compression.
- `docs/decisions.md`: the athlete-first calibration picks and the Rust port details.

### 16. Not chosen yet
- **The desktop UI technology** (Q49 chose a Rust engine and a desktop app; the UI stack is open).

### 17. Tendencies: the six coaching styles (asked 2026-10-18 21:5x)
- **Asked:** Raichel said yes to "your style is yours to set" and added "find more individual styles to pick". Claude proposed six more settings besides risk: structure or freedom, physical or protective, ball hunting or sound, mould or adapt, schemes pull too, coordinators bring their own (`docs/design/tendencies.md`, "Your coaching style").
- **Claude's pick:** yes to all six.

---

## B. Decided but not built

| Decision | Where it was decided | State |
|---|---|---|
| About 35-40 attributes, about 20 positions | [T02] 2026-10-10 | Attributes built 2026-10-18 (67 + 5 hidden); positions 14 |
| Engine physics fixes (5 items) | [T17] 2026-10-14 | Spec only, waiting on "build" |
| Career stats record across levels (percentiles, era adjustment) | Section 3 topic 8 | Not built |
| High school games on the engine | Section 3 ground rules | Stand-in for now |
| Coach buzz from stats (trial 1 item 5, stat part) | [T11] 2026-10-13 11:58 | Waits for the stats record |
| Hidden-gem "oh damn" stat footprint (trial 1 item 9) | [T11] 2026-10-13 12:35 | Waits for the stats record |
| Awards, records, Hall of Fame | Q48; section 3 topic 8 | Not built |
| Real JUCO programs and D3/NAIA in the data | [T11] 2026-10-13 08:03 | Placeholders |
| Section 4 calendar | `docs/design/calendar.md` | Spec only |
| Plays and tactics database | `docs/design/plays-and-tactics.md` | Spec only |
| Engine speed-up (same results per seed) | [T15] 2026-10-14 08:12 | Not started |
| Sections 5 to 11 | `docs/roadmap.md` | Not designed |
| Player tendencies (15 in 8 groups, topics 1-4) | `docs/design/tendencies.md`, 2026-10-18 | Spec only |

Done since the specs were written (the specs still say "not done"):
- **High school NIL rules per state** were added to `data/real-world/hs_states.csv` on 2026-10-13. `docs/design/world.md` follow-up 5 still says "Not done yet".
- **Every college game runs on the engine**, history included. The "Not built yet" line in `docs/match-engine.md` is out of date on this.

---

## C. Parked for later

| Feature | Raichel's words | Where |
|---|---|---|
| Watching players' offseason work | "i'm interested in building an creative way wherre we can see and montiors player work in the off-season. We'll come to that later though." | [T16] 2026-10-14 08:40; likely section 6b |
| A playable NCAA coach career | "i want to build the dynamics into the game for realism and potentially a feature where i become an NCAA coach." | [T16] 2026-10-14 11:57; Q4 "NFL for now, leave room for updates" |
| Better visuals, maybe 3D | "I am potentially intrested in better visuals so leave room for htat ." | [T13] 2026-10-13 17:50 |
| Press conferences and player talks | Q43: a news inbox now, these later | Roadmap section 11 |
| Full club finances | Q28: owner budgets now, full finances later | Roadmap section 11 |
| Deeper training design | "in the training aspect i see sooooo much realism, potential in there." | [T00] 2026-10-11 19:31; section 6b |
| Possible release beyond personal use | "no itention of releasing it as an steam game" (for now) | [T00] 2026-10-11 18:42 |

---

## D. Data gaps (from `data/real-world/README.md`)
- No public source: NIL dollars per school (all levels), NIL ranges below Power 4, yearly JUCO and prep-to-FBS counts, 7 small D2 stadiums.
- Real towns for generated high schools need a census town list; the data has each state's top hometowns only.
- Real position-change rates are not in the dataset (athlete-first measures 14% of players changing position).

[T00]: transcripts/00-project-chat.md
[T02]: transcripts/02-build-the-player-potential-and-development-model.md
[T11]: transcripts/11-build-the-world.md
[T13]: transcripts/13-section-3-match-engine-design.md
[T15]: transcripts/15-build-the-match-engine.md
[T16]: transcripts/16-section-4-calendar-design.md
[T17]: transcripts/17-world-trial-3.md
