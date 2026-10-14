# AFM project
- Owner: Raichel (GitHub cnbville). Repo: https://github.com/cnbville/AFM
- Goal: an American football management sim modeled on Football Manager. Priorities: realism, deep stats, dynamic player potential, a slow-building season flow, player cards, and high school prospects who develop realistically through to the NFL.
- No project instructions set yet.

## How Raichel wants Claude to work
- Raichel owns design decisions: run real mechanic choices past them with a proposal, and wait before building.
- DEFAULT TO ULTRA REALISM (2026-10-11): never ask "realistic or simplified?" questions; the answer is always the most realistic option. Ask only about deep mechanics (how things work and interact), each with a concrete proposal for Raichel to agree with or adjust. Go topic by topic in order and don't skip topics.
- FIRST PROPOSAL MUST ALREADY BE FULL REALISM (2026-10-14, Raichel unhappy, repeated twice): before posting any design proposal, do the "what else would make this more realistic?" pass yourself and include all of it. Shorten wording, never substance. Raichel should never have to ask "more realistic?".
- Mechanics follow real football reasoning, never flat rules, and are dynamic. Trials explain WHY each notable outcome happened.
- Influences should be noteworthy but rarely driving factors: Raichel repeatedly warns against overpowered single factors.
- Keep token use low: ~6% of their session limit per message. One checker per trial, no agent fan-outs, no small follow-ups. Build threads send one final reply and merge to main. Write simply and explain jargon.
- Don't re-measure after tuning (2026-10-13): log tuning worries as trial candidates and test them in the next trial instead of extra runs.
- Design topics: post them as top-level messages in the project chat; Raichel answers there to save tokens (2026-10-13).
- Section loop: design rounds with Raichel, then a spec in the repo, then a fresh thread builds and merges, then a short trial with whys.
- Saving design details: project chat is the full record, and this memory holds the summary. At the end of each section's design, a thread writes the complete spec into the repo from the chat, keeping Raichel's own wording and examples.
- Trials: Raichel likes clickable mock GUIs and full player pages (all attributes, measurables, combine, season-by-season ratings, debug view) to judge whether results are realistic. Always refresh BOTH the mock GUI and player pages with each trial's data.
- Agreed sequences carry through: when a build finishes, the next agreed step starts in the same flow without waiting to be asked (Raichel had to push for the trial 3 fixes after the engine merged, 2026-10-14). Briefs should name the full agreed sequence.
- Trials must match the final game as closely as possible (same engine, same code paths), so bugs don't compound later (2026-10-14).

## Current plan (agreed by Raichel 2026-10-13)
1. Section 3 build: athlete-first (PR #20) and the match engine + game stats + dev checks merged to main 2026-10-14 (docs/match-engine.md lists known gaps, logged as trial 3 candidates, and Claude's unconfirmed calibration picks). Still open: world games through the engine, career stats record. See [[afm-match-engine-build]].
2. World trial 3 fix list: BUILT and merged 2026-10-14 (merge 0c4c536; docs/world-model.md "Trial 3 fix list"). Raichel 2026-10-14: ALL college games on the engine incl. history (game builds 30 yrs; trial 3 uses 15 yrs, as close to the final game as possible); dynasty fix = position room + buried-backup portal + linked units (QB wants receivers/line, CB wants pass rush). Engine speed-ups must keep identical results per seed; ask before anything that changes results.
3. World Trial 3 RAN 2026-10-14 (merge 56c9964, docs/trials/world-trial3.md; 15 yrs on engine, builds in 14 min). 9 findings, all logged as candidates, waiting on Raichel: Tennessee 9/15 titles (engine turns small lineup edges into wins), 3 P4 programs with no winning season in 15 yrs, only 6 Black FBS HCs in 2025 (real 16; cause not traced), pick-1 QB 45%, 3-star 4.6%/2-star 1.1%, engine too much offense + no home edge, absurd stat lines (lead RB ~92% of carries; a kicker with 94 sacks because depth charts pick by rating and kickers are too athletic), K/P/QB too athletic, 46.5% repeated full names. World can keep a game log (WorldOptions::keep_game_log, trial only). Mock GUI now data-driven debug dashboard.
   - 2026-10-14 follow-up in the World Trial 3 thread: engine position values are wrong (QB/OL barely matter, WR/LB too much, RB bug, coaching no effect). Raichel: trace Tennessee first, then fix both. See [[afm-engine-physics-fixlist]].
- Section 4 calendar design runs in its own thread in parallel.
- Plays and tactics database design runs in its own thread (started 2026-10-14, design only). See [[afm-playbook-design]].

## Decisions (confirmed by Raichel)
- Visuals: text/stats main view plus FM-style 2D play view with Next Gen Stats-style highlights; the sim decides, visuals replay.
- Ratings 1-99 absolute (99 = prime Brady), plus a level-relative rating (99 is rare in HS/NCAA). About 35-40 FM-style attributes (list needs Raichel's sign-off). ~20 positions, each player rated at every position.
- Potential: hidden per-attribute ceilings, not hard caps; breakouts lift a ceiling ~10/yr. HIDDEN IN GAME (pivotal): vague outlook plus hints, sharper with better scouts; exact numbers only in debug/trial tools.
- Volatile development; five-star busts after signing plus a small scout-misjudge share; NFL tiers are guidance only; imperfect drafting via GM/Owner skill; D2 and below rarely drafted, day 3 max; generated players with a hidden warm-up history.
- Draft: positional value applies to EVERY first-round pick (QB/EDGE premium, e.g. a generational WR like Jeremiah Smith may not go #1 if teams need a QB), always intertwined with team need.
- World history (2026-10-14): the final game builds 30 years of history on the real match engine (one engine for history and play, so data can be trusted; long new-save build time accepted, engine speed-up planned). HS-on-engine for history was proposed as an exception pending speed; check thread for Raichel's final word.
- Coaching in games (2026-10-14): a bad play call loses through the battles whoever called it; HC quality drives calls when the user isn't calling, plus adjustments, prep and scheme install.

## Real-world data: DONE (2026-10-13)
- data/real-world/ re-researched and merged (PRs #7-#12); gaps in its README. Five-star draft target: 63%, confirmed by Raichel 2026-10-13 (247Sports 70% kept as alternate). PR #12 added HS transfer rules for all 50 states + DC and 9 of 16 D2 stadium sizes. Still no public source: NIL per school and below Power 4, yearly JUCO/prep-to-FBS counts, 7 small D2 stadiums. HS NIL rules per state not in dataset yet. See [[afm-real-world-data]].

## Engine: DONE in Rust (2026-10-11)
- Merged to main (PR #3); TS in legacy-ts/. 38/38 tests; a 250k class in ~2s.

## Roadmap answers (2026-10-11)
- GM+HC like FM; career from 2026, 50+ years, can be fired or switch teams; any team; NFL coaching only for now; personal use; one realistic mode; background sets reputation; editable starting stats.
- HS, college and NFL in extreme detail with one match engine; HS stats carry to the NFL with nuance. Real names with a fictional toggle; real towns, generated high schools; every HS plays full games; real rules and calendar per level; recruiting geography; personality drives contracts, locker room and development; manager ratings grow.
- Full cap rules, agents, rookie scale, trades incl. AI-AI, roster limits, owner budgets, rated staff, schemes. Scouting staff with own boards; combine with deep character detail; live draft, UDFA rush; game plan plus optional live play calling; injuries. A game takes ~15 min, or play by play when calling live. Save starts mid-Feb; weekly training; inbox; owner goals; dense FM look; every possible stat; Hall of Fame.
- Desktop app, Rust engine. Build order: playable early with plain screens, then add depth.
- Sections: 1 engine (done), 2 world (built), 3 match engine + stats DB (design DONE 2026-10-13, all 10 topics; answers in [[afm-section3-design]]; spec merged PR #19 at docs/design/match-engine.md; both "proposed" items confirmed by Raichel: trim washouts' game-by-game detail, over-expected stats lift ceilings; match engine merged 2026-10-14), 4 calendar (design DONE 2026-10-14, all 11 topics; answers in [[afm-section4-design]]; spec not yet written), 5 team management, 6 people, 6b training (deep), 7 scouting/draft, 8 first playable, 9 game day, 10 screens + 2D, 11 later.

## Section 2 world: BUILT 2026-10-13 (PR #16, #17, #18), spec at docs/design/world.md
- Code in crates/afm-engine/src/world/, docs/world-model.md, docs/trials/world-trial1.md, world-trial2.md. PR #18 tuned first-round draft positional value.
- Coach promotions/team growth check DONE 2026-10-13: fixes A-C agreed as trial 3 item 15, build after section 3.
- Realism Q1 answered: all real programs (swap placeholder JUCOs for real ones). Q2 (HS classifications/playoffs, real per-state formats) proposed, unanswered; Q3-7 still to ask.
- Items 1-8 BUILT 2026-10-13 (trial docs/trials/world-trial2.md). Trial 2 flags: Trey mostly white in name data, coach gap too wide in 2000s, dynasties strong (Alabama 9 titles/30y; a later run had Tennessee 16/30y), skill-player bench high, bad coach hire (66yo FCS coordinator to FSU), height format. Item 9 waits for section 3. All parked as trial 3 candidates. Item 13: NIL offers per player (position value from real data, bidding, situational "new era" reach by rising schools like Colorado/Texas Tech/Indiana; money weight by family wealth and ambition; all confirmed).
- Trial 1 walk-through DONE 2026-10-13. All 9 items agreed, list at /mnt/project-files/design/world-trial1-feedback.md:
  1. Successful+loyal coach gets a 'prove it' extension after 2 bad seasons; 3rd ends it.
  2. Realistic race disparity in coach HIRING only (AI bias via networks, fading after 2025; Rooney Rule; TIDES-style report; no effect on ability). Player ethnicity shares real per level/state.
  3. Names drawn by ethnicity, state and birth year from SSA/Census data; bigger pools.
  4. Pipelines from impact relative to conference (all-conference, any draft pick/UDFA), to schools/areas; coaches bring ties; G5/FCS overflow regions.
  5. Coach buzz from standout stats (PPG record, EPA) relative to level, fades 2-3 yrs; applies to every rung. Add college position coaches. Stat part with section 3.
  6. Real NCAA academic tiers (2.3 qualifier, 2.0-2.299 academic redshirt, <2.0 JUCO); per-school admissions bars.
  7. Real positional draft value curves plus team needs.
  8. Height/weight/measurables from real combine distributions; combine results from attributes+body; fuller player pages.
  9. Hidden gems leave "oh damn" stat footprints. Built with section 3.
- Mock GUI: https://claude.ai/artifact/HaZ7FBAwE1FGCNmCWyC4Fg ; Player pages: https://claude.ai/artifact/3997w39vMdTQvdS6UJKzHR
- Earlier flags: dynasties a bit strong, generous title-chances grade, small story bugs (repeat life event, bust with no reason, repeating names).
Design summary (topics 1-8, 2026-10-11/13): hidden background (6 parts, hardship small grit, mental health big hidden factor); powerhouses pull transfers, no consequences; coach hidden ability + lagging rep, careers grow/decline, trees as influence; exposure/visibility, pipelines only from impact; family deciders, handlers can steer wrong school; JUCO real programs, small lasting marks; prestige audiences converge over 10-20y, grades steady vs fast; NIL collectives with budgets, HS NIL per state rules, clear warnings when collectives dry up.
