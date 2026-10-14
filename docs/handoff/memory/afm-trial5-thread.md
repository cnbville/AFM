---
name: afm-trial5-thread
description: AFM trial 5 moved to its own thread on 2026-10-11 to save tokens; league uses generated players; film-line options for stars/FBS offers measured and awaiting Raichel
metadata:
  type: project
---

On 2026-10-11 Raichel moved trial 5 into a new thread ("Trial 5 league sim") to cut token use: keep it lean, no agent fan-outs, at most one checker per trial.

Code: restored from /mnt/project-files/code-backups/afm-trial5-start.bundle (commit 0ab7c94, local only, never pushed) onto local branch claude/project-thread-aqjblw. Uncommitted film-line knobs (PIPELINE.starFilmRank.maxFor3/maxFor2, fbsOfferMaxFilmRank, record.seniorFilmRank; defaults Infinity = old behaviour) committed locally as 79221e9 (not pushed), backup bundle code-backups/afm-trial5-thread.bundle; measuring script trial5-film-measure.ts.

Decisions:
- League players: **Generated** (real team names, sim-grown players, ~15-season warm-up). Raichel chose it 2026-10-11.
- Film line: options A loose (3★ top 5,000 / 2★ 12,000 / FBS below 3★ 10,000), B middle (3,500 / 8,000 / 6,500), C strict (2,700 / 6,500 / 4,500), recommended C. Measured two seeds: low-star draft share now 12%, A 7%, B 8%, C 12% (real ~19%); FCS draft share now 7%, options 3-3.5% (real ~8%). Awaiting Raichel's pick.

Still to talk before building the league: rest of league structure, first-year adjustment drivers, NIL pull, whether players know their likely drafting team.

**Why:** [[raichel-approves-design-decisions]].
**How to apply:** Ask the remaining league questions one at a time. Related: [[afm-trial5-answers]], [[afm-trial30-results]]

Update 2026-10-11 14:06: Raichel said "wait, I haven't judged trial 5 yet" (read as the 30-player trial). Paused all trial 5 work until they give feedback on it; the film-line card stays open.

Raichel's 30-card feedback (2026-10-11): card 1: retirement and ageing should depend on position (backs earlier) but leave room for outliers like Derrick Henry (via the longevity trait), for call 3. Card 2: five-star bust from scouts over-rating upside is "realistic enough", as long as FBS staffs also have room for error (they do).
Claude told Raichel the early-bloomer five-star bust (card 2) will count as an allowed bust reason unless Raichel objects.

Update 2026-10-11 14:42: Raichel revised the five-star bust rule: scouts misjudging a player SHOULD be a cause too, but a small one ("plenty of hard-working QBs don't make it to the NFL"). Early-bloomer busts (card 2) are fine. Don't build anything yet; Raichel is still reading the other 28 cards.

Card 3 feedback (2026-10-11): a good FBS player (level rating 83) dismissed at 19 dropped to FCS. Raichel: dismissal shouldn't automatically mean a drop to FCS; he can move to a smaller FBS school depending on severity. Current code (career.ts, dismissed players) always drops exactly one level. Proposed fix: dismissal severity (serious = drop/leave, lesser = sideways move to a lower-profile FBS program, better players get more offers). Option posed, awaiting answer. Part of call 7 (sideways/down transfers).

2026-10-11 14:50: Raichel chose A (dismissal severity). The three fixes (position ageing, small scout-misjudgment bust cause, dismissal severity) are kept in /mnt/project-files/design/trial6-fix-list.md, to walk through after she finishes the cards. Goal: trial 6 is the last trial needed. She asked for them to be kept in a separate thread; thread creation is the coordinator's job.
Coordinator keeps the live "Trial 6 fix list" post in the project chat. New agreed fixes: send the coordinator one line via send_message (session_01SxBd4aGfJ7xRphfZpsT2ZH) and keep /mnt/project-files/design/trial6-fix-list.md as the only file copy.
Card 5 note (2026-10-11): added to fix 1: career length varies by mental makeup, and legs-reliant QBs (Jayden Daniels) fade/retire earlier than pocket passers.
Cards 6-7 (2026-10-11): added fix 4 (fewer early cuts + practice squad/FA) and fix 5 (urgency to declare: money/NIL, position shelf life, fear of falling, age, injury, family, coaching change) to trial6-fix-list.md.
Final (2026-10-11 17:21): Raichel agreed to all calls and bugs; fixes 6 (camp/in-season roster form, poor judgment lets a 62 make a bad team) and 7 (calls 1-5 + film line C) added. These are the last fixes before building the game; debug in-game after. Not built yet.
2026-10-11 17:30: With Raichel OK, everything pushed to branch claude/project-thread-aqjblw, draft PR #2 (https://github.com/cnbville/AFM/pull/2). Repo now has CLAUDE.md, docs/decisions.md, docs/design/*, docs/trials/. Trial 6 build moves to a plain Claude Code session outside the project.
2026-10-11 19:13: Raichel merged PR #2 into main. main now holds all code + docs (CLAUDE.md, docs/decisions.md, trial6-fix-list with 8 fixes). Trial 6 build continues outside the project from main.
