# Memory seed for the new project

The new project starts with empty memory. Save the entries below as its first memories, one file each, then adjust them as work moves on. They are up to date on 2026-10-14. The old memory in [memory/](memory/) is a little behind: it doesn't say that the calendar and plays specs were written, that the physics spec is on main, or that world trial 3 finished.

Each entry gives a name, a one-line description, a type, and the body to save.

---

### afm-project
- **Description:** What AFM is, who owns it, where everything lives
- **Type:** project

AFM (American Football Manager): an ultra-realistic American football management sim modelled on Football Manager, for Raichel's personal use. Owner Raichel (GitHub `cnbville`), repo https://github.com/cnbville/AFM. Rust engine (Cargo workspace), desktop app later, UI tech not chosen. The project moved from another Claude account on 2026-10-14; everything from the old project is in `docs/handoff/` (word-for-word transcripts, decisions, open questions, rules, old memory, shared files, pages). Old claude.ai artifact links are dead; the pages are saved in `docs/handoff/artifacts/`.

### afm-state-2026-10-14
- **Description:** Where each roadmap section stood at the account move
- **Type:** project

- 1 player model: done (Rust).
- 2 world: built (`world/`, `docs/world-model.md`), trials 1-3 in `docs/trials/`.
- 3 match engine and stats: engine built; every college game runs on it, history included. Not built: the career stats record and high school games on the engine.
- Engine physics fixes: spec agreed and on main (`docs/design/engine-physics-fixes.md`), **not built: wait for Raichel's word**.
- 4 calendar: design done, spec `docs/design/calendar.md`, not built.
- Plays and tactics: design done, spec `docs/design/plays-and-tactics.md`, not built.
- 5-11: not designed.
- All branches merged; no open PRs.

### afm-next-steps
- **Description:** The agreed next steps, in order
- **Type:** project

1. Build the engine physics fixes only when Raichel says so (QB play, line play, depth and rotation, the RB bug, coaching through the battles).
2. Then world trial 4: 15 years with every college game on the engine; refresh both the debug dashboard and the player pages.
3. Raichel's calls on world trial 3 findings 2-9 and the other items in `docs/handoff/open-questions.md`.
The checks before trial 4 were proposed and never confirmed.

### raichel-working-rules
- **Description:** How Raichel wants Claude to work; full list with quotes in docs/handoff/working-rules.md
- **Type:** feedback

Ask before every design decision, with a concrete proposal, and wait. Ultra realism always; never ask "realistic or simplified?". Topic by topic, no skipping. Influences noteworthy, never driving. Dynamic, attribute-driven outcomes, never flat rules. Every trial result explains why; sanity-check first. Short, plain sentences and a clear ask. Low token use: one checker per trial, no fan-outs, no small follow-ups. Build threads merge to main and send one final reply. Mark Claude's unanswered picks "proposed, not confirmed". Potential is hidden in the game.
**Why:** Raichel corrected each of these at least once (see `docs/handoff/working-rules.md`).
**How to apply:** check this list before every proposal, trial and reply.

### afm-full-realism-first-proposal
- **Description:** Every design proposal must already include the "what else would make this more realistic" pass
- **Type:** feedback

Raichel had to ask "more realistic?" twice in a row on 2026-10-14 and was unhappy: "yes but i'm once again not happy that i had to give you an extra prompt before you considered those changes".
**Why:** Claude trimmed realism to keep messages short.
**How to apply:** before posting, add leverage, information and deception, situation and weather, level differences, real research and per-player nuance. Shorten wording, never substance.

### afm-trials-and-visuals
- **Description:** What every trial must deliver
- **Type:** feedback

Every trial refreshes both the clickable mock GUI (debug dashboard) and the full player pages (all attributes, measurables, combine, season-by-season ratings, debug view). Trials use the same engine and code paths as the final game. Don't re-measure after tuning; log worries for the next trial. Old pages: `docs/handoff/artifacts/`. Rebuild data with `AFM_DASH=dash.json AFM_CARDS=cards.json cargo run --release --bin world_trial -- 2026 15`.
**Why:** "why did you only give me a players page trial 2 ?" and "As simillar to the final game as possible otherwise i'll find compounding bugs later on".

### afm-agreed-sequences
- **Description:** When a build finishes, the next agreed step starts without being asked
- **Type:** feedback

Raichel (2026-10-14): "no the agreement was to do the world trial 3 fix after the engine ? is there something blocking you from doing this ?" Briefs name the full agreed sequence. Finished work merges ("why are the latest two threads not merged ?").

### afm-coaching-rule
- **Description:** How coaching and play calls must matter in the engine
- **Type:** project

Raichel 2026-10-14: a call wins or loses through the battles (running into a +2 box loses whoever called it). Coach quality drives the calls when the user doesn't pick, plus adjustments, prep and scheme install. No flat bonus. Good plans must be clearly rewarded without being broken, so learning real football pays. Applies to the physics fix and the plays design.

### afm-engine-rules
- **Description:** Rules for changing the match engine
- **Type:** feedback

One engine for every game at every level (history included). Engine speed-ups must give exactly the same results per seed; ask before anything that changes results. Real-data checks are development safeguards only, never in-game nudges. Outliers (Derrick Henry's 153 HS touchdowns) must stay possible. On-field rules stay at 2026 for the whole career; off-field changes only from the agreed list of 12.

### afm-open-items
- **Description:** Where the open questions live and the most urgent ones
- **Type:** project

Full list: `docs/handoff/open-questions.md`. Most urgent: physics build go-ahead; the trial 4 checks; world trial 3 findings 2-9; the Trey name question; world realism Q2-7; HS games on the engine; plays research and build timing; calendar leftovers; the 35-40 attribute list and ~20 positions (decided, never drafted for sign-off).

### afm-parked
- **Description:** Features Raichel parked for later
- **Type:** project

Watching players' offseason work (likely section 6b); a playable NCAA coach career (recruiting built as real actions so it's possible); better visuals or 3D; press conferences and player talks; full club finances; deep training (6b).

### afm-data
- **Description:** The researched real-world data and its gaps
- **Type:** reference

`data/real-world/` and `data/names/`, each with a README naming sources and gaps. Five-star draft target 63% (Raichel). No public source: NIL per school, NIL below Power 4, JUCO and prep-to-FBS counts, 7 small D2 stadiums. HS NIL rules per state are in `hs_states.csv` (added 2026-10-13). Placeholder JUCO programs still in `world/recruiting.rs`.
