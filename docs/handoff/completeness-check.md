# Completeness check

Done 2026-10-14 before merging the handoff. For each conversation in the old project: how many messages were copied, how many Raichel wrote, and where its decisions and open questions now sit in the readable layer. Every one of Raichel's 230 messages and decision-card picks was read for this (they are listed in [raichel-messages.md](raichel-messages.md)).

How the copies were made: `fetch_project_timeline` and `fetch_thread` with full bodies, oldest first, paging until no cursor was left. Bodies cut short on a first fetch were fetched again in full; no copied message is truncated. The project chat's one post with replies but no thread (the topic 3 post, 2026-10-13 05:40) has its replies at the end of `00-project-chat.md`. Notes from the coordinator to a thread, and decision cards with their picks, are kept as they appeared.

"D" means `decisions.md`, "O" means `open-questions.md`, "R" means `working-rules.md`.

| # | Conversation | Messages | By Raichel | Decisions | Open questions | Checked |
|---|---|---|---|---|---|---|
| 00 | Project chat (and replies to the topic 3 post) | 212 | 80 | D §1-3 (vision, all 50 roadmap answers, process), §5 data, §6 world topics 1-8, §8 the coaching rule; film line C in §4 | O A1-A2 (physics), A16 (UI tech), C (parked) | Yes |
| 01 | Map AFM repo toward the football sim goal | 7 | 1 | D §1 (stack question; TypeScript later superseded) | None | Yes |
| 02 | Build the player potential and development model | 123 | 20 | D §4 (scale, attributes, positions, potential, breakouts, volatility, tiers, five-stars, D2, calls 1-7, first-year adjustment, cards "League sim" and "Fix both") | O A10 (attribute list, positions), A12 (first-year drivers), A13 (items 6-13) | Yes |
| 03 | Design the season calendar and pacing | 11 | 1 | D §4 ("NFL team only"); the draft is superseded by section 4 | None | Yes |
| 04 | Trial 5 league sim | 48 | 13 | D §4 (trial 5 review: retirement, misjudging, dismissals, career length, cuts, urgency, in-season changes; card "Generated") | O A12 (NIL pull, drafting-team knowledge) | Yes |
| 05 | Trial 6 final build | 18 | 1 | D §3 (trial 6 setup) | None (known gaps went into the calibration notes) | Yes |
| 06 | Rewrite engine in Rust | 8 | 1 | D §1 (Q49 Rust), §2 (Q46 agreed, Q50) | O A16 (UI tech) | Yes |
| 07 | Full spec to GitHub | 8 | 1 | D §3 (everything in git; six items confirmed) | None (its one open line, escaping to a powerhouse, was answered later: world follow-up 9) | Yes |
| 08 | Real-world dataset check | 14 | 2 | D §5 (63%, research freely) | O D (data gaps) | Yes |
| 09 | Real-world data research | 11 | 1 | D §5 | O D (data gaps) | Yes |
| 10 | World spec to GitHub | 16 | 5 | D §6 (topic 8 and follow-up answers 1-9) | O A5 (realism defaults, follow-up 7) | Yes |
| 11 | Build the world | 111 | 32 | D §6 (realism Q1, trial 1 list items 1-9, trial 2 parking, NIL item 13, pick-1 QB, whole first round) | O A4 (Trey), A5 (realism Q2-7), A14 (trial 1 flag 2, trial 2 flags), B (gem footprint, coach buzz stats, real JUCOs) | Yes |
| 12 | Coach promotions and team growth check | 8 | 2 | D §6 (fixes A-C) | None (built in the trial 3 fix list) | Yes |
| 13 | Section 3 match engine design | 62 | 24 | D §7 (topics 1-10, athlete-first, build order) | O A6 (HS on the engine), B (career stats record) | Yes |
| 14 | Match engine spec to GitHub | 5 | 0 | D §7 ("yes to both" on its two unconfirmed items, answered in 00) | None | Yes |
| 15 | Build the match engine | 98 | 9 | D §6 (trial 3 fix list: engine history, dynasty fix B plus linked units, fresh thread) | O A6 (HS history), A7 (engine speed), A15 (Claude's picks) | Yes |
| 16 | Section 4 calendar design | 59 | 24 | D §9 (topics 1-11, the 12 off-field changes) | O A9 (withdrawal date, chances, build order), C (offseason work, NCAA coach career) | Yes |
| 17 | World Trial 3 | 36 | 8 | D §8 (physics question, trace first, five fixes, don't build) | O A1-A3 (build go-ahead, the checks, findings 2-9) | Yes |
| 18 | Plays and tactics design | 31 | 12 | D §10 (topics 1-9, the reward rule) | O A8 (research, build timing) | Yes |
| 19 | Calendar spec to GitHub | 9 | 1 | D §9 ("merg") | O A9 | Yes |
| 20 | Account move handoff (plus its brief in `20b`) | 27 | 2 | D §3 (the account move) | None | Yes |

Working rules (R) were taken from every conversation; most come from 00, 02, 04, 11, 13, 15, 16, 17 and 18.

## Other things checked

| Item | Result |
|---|---|
| Memory: `MEMORY.md` and 22 memory files | Copied to [memory/](memory/), identical to the live memory (checked with `cmp` just before merging). The new memory to seed is [memory-seed.md](memory-seed.md). |
| Shared folder `/mnt/project-files` | Every file copied byte for byte to [project-files/](project-files/), uploads included (checked with `cmp`). |
| Published pages | All 3 saved as HTML after their last update ([artifacts/](artifacts/)). |
| Branches | All 15 `claude/*` branches are fully merged into `main` ([project-settings.md](project-settings.md)). |
| Pull requests | All 20 merged, none open. |
| Code backups | Everything in them is on `main`, except `trial5-film-measure.ts`, which is kept in the handoff copy. |
| Email addresses and credentials | Searched the whole handoff folder (transcripts, raw JSON, uploads); none found. |

## Not carried over

See the README's "Not carried over" section. In short: the coordinator's start-up briefs for threads 01-19 and every session's internal tool log; earlier versions of the three pages; memory's two sync bookkeeping files; project settings beyond what the threads could read; and the live project itself.
