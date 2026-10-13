# AFM roadmap

This is the full plan for AFM, written so that anyone who never saw the project chat can understand it. It records Raichel's answers to the 50 roadmap questions (2026-10-11) in their own words, the platform choice, the build order and every section with its status.

Labels used in this file:
- **Confirmed**: Raichel answered it.
- **Proposed, not confirmed**: Claude suggested it and Raichel has not said yes. Treat it as a draft and ask before building on it.

Related docs:
- `docs/decisions.md`: the decision log, mostly about the player model.
- `docs/development-model.md`: how the player model works today.
- `docs/design/world.md`: section 2 (the world), the full design spec.
- `docs/design/season-calendar-and-pacing.md`: draft calendar (week by week) and pacing.

---

## 1. The vision

AFM (American Football Manager) is Football Manager for American football. Raichel's original brief: an equivalent of Football Manager 27, focused on American football. Realism, stats and dynamic potential are extremely important. The flow is patient and doesn't rush climaxes. It has player cards, and high school players who improve realistically all the way to the NFL.

In one sentence: you are an NFL general manager and head coach in one, in a living world where every high school, college and NFL game is simulated, players grow from 14-year-olds into pros with hidden and changing potential, and your career can run 50 years or more.

What makes it AFM:
- **Ultra realism.** Everything follows how real football works. If there is a realistic option and a simplified one, the realistic one is always the answer.
- **Dynamic, not static.** Outcomes come from attributes, personality and circumstances, never flat rules. Some rookies are ready at once (Jaxon Smith-Njigba, Malik Nabers); some UDFAs thrive for specific reasons.
- **Hidden potential.** You never see a player's exact ceiling. You see a vague outlook plus hints, sharper the better your scouts are.
- **Stats everywhere.** Raichel: "i want every possible stat, i want to be an huge stat nerd."
- **A patient season.** The year builds slowly. Big moments (the draft, cutdown day, the trade deadline, the playoffs) are earned, not constant.

## 2. How Raichel wants the work done

These rules apply to every Claude session and every contributor.

1. **Raichel owns design decisions.** Run real mechanic choices past Raichel with a concrete proposal, then wait before building. Raichel (2026-10-10): "calm down with the freestyling... run aspects through me."
2. **Default to ultra realism (2026-10-11).** Never ask "realistic or simplified?" questions. The answer is always the most realistic option. Raichel: "i'm noticing you ask me a lot of questions that you already know the answer too, i've told you multiple times i want ultra realism so your questions are arbitrary." Ask only about deep mechanics: how things work and how they interact. Raichel's example of a good question: how the ranking of coaches works, and how a school in a crime or low-income zone differs from a Mater Dei. Each question comes with a concrete proposal for Raichel to agree with or adjust.
3. **Go topic by topic, in order, and don't skip topics.** Raichel caught a skipped topic in section 2 ("Why are you skipping topic 2?").
4. **Influences are noteworthy, rarely driving.** Raichel repeatedly warns against any single factor becoming overpowered. Background, family, school and similar factors nudge outcomes; they don't decide them.
5. **Real football reasoning, never flat rules.** Derive outcomes from the attributes that matter.
6. **Every trial result explains why.** Each notable outcome gets a reason so Raichel can judge it. Sanity-check results for contradictions first.
7. **Keep token use low.** One message can cost Raichel about 6% of a session limit. So: one checker pass per trial, no agent fan-outs, no small follow-up messages. Build threads send one final reply. Write plainly and explain jargon.
8. **Push and merge only with Raichel's OK.** Raichel gave that OK for build threads: they merge to `main` so any new session sees the latest work (trial 6, 2026-10-11, answer B: "merge into main").
9. **Gameplay finds the bugs now.** The player model is done. Raichel finds further calls and bugs while playing, not in more trials.

### The section loop (confirmed 2026-10-11)

Every section goes through the same four steps:
1. **Design rounds with Raichel**, as many batches as it takes, until the details are nailed down.
2. **A spec in the repo**: a thread writes the complete design from the chat into `docs/design/`, keeping Raichel's own wording and examples.
3. **A fresh thread builds it**, merges it and sends one message.
4. **A short trial**, with a "why" on every result.

Raichel's comment on depth (2026-10-11): "i understand i ask you for 50 questions so you can't get much detail in just 50 but obviously i want waaaaay more control of the details for example in the training aspect i see sooooo much realism, potential in there." So the 50 answers below are the outline. Each section's own design rounds add the detail.

---

## 3. The 50 roadmap answers

Asked in five batches of ten on 2026-10-11. Each item gives the question, the options offered, and Raichel's answer. Raichel's own words are in quotes. All answers are **confirmed** unless marked.

### 3.1 You and your career

**Q1. Your role.** Options: GM only, head coach only, both like FM, college coach.
Answer: **C, both GM and head coach, like a Football Manager manager.** You run the roster, contracts and draft, and the game plans and play calls.

**Q2. Can you be fired and join another team?**
Answer: "Absolutely, the game starts in 2026 then could go on for 50 years if needed, switching teams."
- The career starts in 2026.
- It can run 50+ years.
- You can be fired, and you can move to other teams.

**Q3. Which teams can you start with?**
Answer: "Any teams, again it's based on real life and a comprehensive simulation game."

**Q4. What can you coach in the first version?**
Answer: "NFL for now, leave room for updates." College coaching may come later, so the code must not block it.

**Q6. Single player or online?**
Answer: "For now it's just meant for me personally on my computer, no intention of releasing it as a steam game." A single-player game for personal use.

**Q7. Difficulty levels?**
Answer: one realistic mode. No difficulty levels.

**Q8. Your background (ex-player, scout, coordinator) sets your starting reputation?**
Answer: "Yes, changeable starting stats." Your background sets your starting reputation, and you can edit your starting stats.

**Q20. Your own manager ratings (motivating, tactics, judging talent) grow over your career, like FM?**
Answer: yes.

### 3.2 The world

**Q5 and Q11. Names.**
- Q5 (team names: real cities with made-up nicknames, real names, or fully fictional): "A, realistic names."
- Q11 (colleges with real names): "I changed my mind, i want NFL to have real names too, but add a toggle when starting new game that allows you to choose before you start a save."
- **Result: real names everywhere (NFL, colleges, conferences), with a toggle at new-game setup to switch to fictional names.** The game is personal, so licensing is not a concern.

**Q9. Which levels are simulated, and how deep?**
Answer: "All 3 with extreme detail, i want every part to matter, stats in highschool should translate to NFL (i mean this nuanced)."
- High school, college and the NFL are all simulated in extreme detail.
- High school stats carry forward to the NFL, with nuance (adjusted for competition, not taken at face value).

**Q10. NFL setup (real 32 teams, 17 games, 14-team playoff)?**
Answer: "A, -> same goes for FBS, FCS, D2 they should all have real life aspects." The real NFL structure, and real structure for FBS, FCS and D2 as well.

**Q12. High schools.** Answer: **real towns with generated high schools.**

**Q13. High school games.** Options: every school plays full simulated games, or full games only for strong programs.
Answer: **every school plays full simulated games.**

**Q14. One match engine for every level, adjusted for opponent strength, so high school numbers really carry forward?**
Answer: yes.

**Q15. Real rules at each level** (NCAA eligibility, the four-game redshirt rule, scholarship and roster limits, D2 partial scholarships, NIL and portal windows)?
Answer: yes.

**Q16. Real calendar at each level** (high school season, signing days, portal windows, bowls, combine, draft)?
Answer: yes.

**Q17. Recruiting geography with real talent hotbeds** (Texas, Florida, Georgia, California)?
Answer: yes.

### 3.3 Players and people

**Q18. What you see of players.** Options: rating ranges that sharpen with better scouts (FM-style), or exact numbers.
Answer: ranges that sharpen, plus hints: "add a small hint/tip when you have high scouting skills. For example if a smaller edge in HS has a high QB pressure rate and the GM's scouting skills are high, he gets a small hint 'this player is probably gonna grow in size' or a hint that says the opposite."

**Q19. Personality (leadership, greed, loyalty, temper) affects contracts, the locker room and development?**
Answer: "Absolutely 10000% yes, ultra-realism."

**Q21. Off-field events** (holdouts, arrests, media drama), driven by personality, and rare? Answer: yes.

**Q22. Morale and chemistry** (playing time, contract unhappiness, team leaders)? Answer: yes.

**Q46. The player card.** Claude listed: ratings, level rating, original vs boosted potential, career stats, scout hints and character report.
Answer: "Potential should pretty much be hidden, like the suggestion i gave earlier the better your scouting you'll receive small important hints and i agree there should be a small idea of potential but i absolutely don't want the 'this is the players ceiling exactly' that defeats the entire point of the game and is also what i mentioned in the beginning. This is extremely pivotal to understand."
- **Pivotal rule:** the game never shows an exact ceiling. You get a small, vague idea of potential (for example "could become a starter") plus small, important hints. Both get sharper and more reliable the better your scouting.
- The earlier "original vs boosted potential" display is for trial and debug pages only, never the game. Raichel agreed to this.
- The engine enforces it: true potential is only readable with the `debug` cargo feature (see `docs/decisions.md`).

### 3.4 Roster, money and staff (batch 3, Raichel: "yes to all")

**Q23. Salary cap:** full real rules: dead money, restructures, franchise tag, rollover.
**Q24. Contracts** are negotiated through agents with their own personalities.
**Q25. Rookie wage scale** by pick, plus the fifth-year option.
**Q26. Trades** with current and future picks, and AI teams trade with each other.
**Q27. Roster rules:** 90 in the offseason, 53 active, practice squad, IR.
**Q28. Money:** owner budgets for staff, scouting and facilities now; full club finances later.
**Q29. Staff** (coordinators, position coaches, scouts, trainers, medical), each rated, with an effect on development, injuries and scouting.
**Q30. Schemes** (3-4 vs 4-3, West Coast vs Air Raid) change which players fit and what they're worth to you.

### 3.5 Scouting and the draft

**Q31. Scouts assigned to regions, conferences or single players, with reports that improve over time.**
Answer: yes. Raichel also asked how scouting works today.
- **How it works today (in the engine):** each high school senior gets three film reports with random error, averaged into his stars. Upside is one shared guess, which misses more often. Each NFL team grades its draft board once, and its accuracy depends on the GM's and owner's drafting skill. The user can't scout yet.
- **The design for the game (confirmed 2026-10-11, details in section 7):** a scouting staff (director, area scouts, cross-checkers). You assign them to regions, conferences or single players. Every report has error that shrinks with the scout's skill and with how much he has seen (film, live games, workouts, interviews). Your draft board is built only from your own reports, never from true ratings, and AI teams build theirs the same way with their own errors. Pro scouting works the same way for free agents and trades. Character reports feed each player's hidden personality.

**Q32. Combine, pro days and private workouts with real measurables** (40-yard dash, bench, vertical, shuttle)?
Answer: "Yes but with insane detail, did the player show up early or late, are his parents loudmouths that yell at every game or are they social media happy? how did he seem to the scouts?" Character detail is part of scouting.

**Q33. Live draft day:** you pick on the clock, trade up or down, and AI teams trade too. Answer: yes.

**Q34. A rush to sign undrafted free agents** right after the draft. Answer: yes.

### 3.6 Game day

**Q35. Your control:** set the game plan, and call plays live whenever you want. Answer: yes.
**Q36. Playbook:** real schemes with tendencies you can adjust (run-pass mix, blitz rate), not a play-drawing editor. Answer: yes.
**Q37. In-game decisions** (halftime adjustments, timeouts, fourth downs, two-point tries), with optional analytics advice from your staff. Answer: yes.
**Q38. Weather, home field, altitude, travel and short weeks** affect play, modestly. Answer: yes.
**Q39. In-game injuries,** with your medical staff estimating return dates. Answer: yes.
**Q40. Game length.** Answer: "Game time on paper should always be 15 minutes, when live play calling it's play by play." Watching a game takes about 15 minutes; when you call plays live, it goes play by play.

Earlier confirmed (2026-10-10): game day is text and stats as the main view, plus an FM-style 2D play view. Highlights look like NFL Next Gen Stats (pass charts, route maps, run paths). The sim decides each play; the visuals only replay it, so the engine must output play-level positions.

### 3.7 Season flow

**Q41. A new save starts mid-February,** right after the Super Bowl. Answer: yes.
**Q42. Weekly training focus per unit, plus an individual plan for each player.** Answer: yes. Raichel wants far more depth here (see section 2 above), so training is its own section (6b).
**Q43. Media:** a news inbox now; press conferences and player talks later. Answer: yes.
**Q44. Owner goals and job security** (expectations, patience). Answer: yes.

### 3.8 Screens, stats and history

**Q45. Look:** dense tables like FM, with a modern finish. Answer: yes.
**Q47. Stats:** a full play-by-play stats database with advanced stats (EPA, CPOE, pressure rate) at every level. Answer: "Yes, i want every possible stat, i want to be an huge stat nerd."
**Q48. History:** Hall of Fame, awards (MVP, Heisman, All-American) and record books. Answer: yes.

### 3.9 Platform and build order

**Q49. Platform.** Raichel: "Whatever allows me to run the most realistic dense game engine (discuss your answer with me first)." Options discussed:
- A. Runs in a browser: easiest, but limited memory and speed. Fifty years with every high school game simulated would choke it.
- **B. A desktop app with an engine in Rust (chosen).** Maximum speed on all processor cores and large local save files. Football Manager itself runs on a fast native engine.
- C. A desktop app keeping the TypeScript code: no rewrite, but several times slower once the world gets big.

Answer: "Okay you can rewrite into rust." The player model was ported to Rust and merged the same day (PR #3). A 250,000-player class now runs in about 2 seconds (TypeScript took about 57). The UI technology for the desktop app is not chosen yet.

**Q50. The first playable milestone.** This question confused Raichel at first; it is really about build order:
- **A (chosen): playable early with plain screens, then add depth and polish while playing and debugging.** Raichel: "Ahhhh okay... yeah A. Smart."
- B: build everything, including nice screens and the 2D view, before playing.

"First playable" means: you start a career in 2026, pick a team and play your first season (offseason, draft, games, playoffs) on plain text screens, with high school and college simulated around you. Before that, the world builds a hidden history so rosters, records and programs look real on day one.

---

## 4. Build sections

The order below is **confirmed** (Raichel, 2026-10-11: "ok to all"): world, match engine, calendar, team management, people, training, scouting and draft, first playable, game day, screens.

| # | Section | Status |
|---|---|---|
| 1 | Engine: the player model in Rust | **Done** (2026-10-11) |
| 2 | The world | **Design done**, ready to build (`docs/design/world.md`) |
| 3 | Match engine and stats database | Not started |
| 4 | Calendar | Not started (a draft doc exists) |
| 5 | Team management | Not started |
| 6 | People | Not started |
| 6b | Training (deep) | Not started |
| 7 | Scouting and draft | Not started |
| 8 | First playable | Not started |
| 9 | Game day control | Not started |
| 10 | Screens and 2D view | Not started |
| 11 | Later | Not started |

### Section 1. Engine (done)

The player model: attributes, positions, hidden potential, development from age 14 through high school, college and the NFL, stars and recruiting, the draft, pro careers and retirement. Tuned over six trials with Raichel. Ported to Rust and merged to `main` (PRs #2 and #3).
- Code: `crates/afm-engine`. How it works: `docs/development-model.md`. Every decision: `docs/decisions.md`. Trial history: `docs/design/`.
- Known gap: FCS players are about 2-5% of draft picks depending on the seed (real about 6%).
- The player-facing `potential_outlook` (tier words, confidence and hints) is a placeholder until section 7's scouting design.
- The full attribute list (about 35-40, FM-style) still needs Raichel's sign-off.

### Section 2. The world

Real NFL, colleges and conferences; real towns with generated high schools; teams, programs and staff; the real/fictional name toggle; each player's hidden background; the hidden history before 2026. Design finished 2026-10-13: all eight topics are in `docs/design/world.md`, with open questions at the end.
- Built 2026-10-13: `crates/afm-engine/src/world/`. How it works, and Claude's own picks marked "proposed, not confirmed": `docs/world-model.md`. Trial: `docs/trials/world-trial1.md`. Raichel's trial 1 list built the same day; trial: `docs/trials/world-trial2.md`.
- Open: realism questions 2-7 (classifications and playoffs, every college level, realignment, the carousel, franchise traits, talent geography) and the trial's flags (dynasties a bit strong, a generous title-chances grade).

### Section 3. Match engine and stats database

Every game at every level (high school, college, NFL) simulated play by play by one engine, adjusted for opponent strength. Play-level positions so the 2D view can replay plays. Injuries, weather, home field, travel. The full stats database with every possible stat, including advanced stats (EPA, CPOE, pressure rate).
- Design finished 2026-10-13: all ten topics are in `docs/design/match-engine.md`. The build starts with athlete-first player creation, then the engine. The real-data checks are development-only safeguards.

### Section 4. Calendar

The whole year for all three levels, with real rules and dates: games, signing days, portal windows, bowls, the combine, the draft, free agency, training camp. The patient pacing lives here. Starting point: `docs/design/season-calendar-and-pacing.md` (a draft from 2026-10-10; its defaults were later settled by the answers above: GM+HC role, mid-February start, real names with a toggle).

### Section 5. Team management

Roster limits (90/53, practice squad, IR), depth charts, contracts and the full cap, agents, the rookie scale and fifth-year option, trades (including AI to AI), free agency, the practice squad.

### Section 6. People

Personality, morale and chemistry, off-field events, rated staff and hiring (and staff being poached), owner goals and job security, and your own manager profile with growing ratings and an editable starting background.

### Section 6b. Training (deep)

Its own section because Raichel sees "sooooo much realism, potential" in it. The 50 answers only fix the outline: weekly focus per unit plus an individual plan per player, with staff quality affecting development. Everything else is for the design rounds.

### Section 7. Scouting and draft

Your scouting staff and assignments, reports with hidden potential, vague outlooks and hints, character reports with "insane detail", the combine, pro days and private workouts, the live draft and the UDFA rush.

### Section 8. First playable

New-game setup (team choice, name toggle, your background and editable starting stats), save and load, and plain text screens. Raichel starts playing and debugging here.

### Section 9. Game day control

Game plans, live play calling (play by play), in-game decisions (halftime, timeouts, fourth downs, two-point tries) and staff analytics advice. A watched game takes about 15 minutes.

### Section 10. Screens and 2D view

Dense FM-style screens with a modern finish, player cards, the news inbox, history (Hall of Fame, awards, record books), and the 2D Next Gen Stats-style replays.

### Section 11. Later

Press conferences and player talks, full club finances, a college coaching mode, and anything else Raichel adds.
