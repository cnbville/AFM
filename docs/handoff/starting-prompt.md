# Starting prompt for the new project

Paste everything between the lines below as the first message in the new project. It tells the new Claude what AFM is, where things stand and how Raichel wants it to work, and makes it seed the new project's memory first. The shorter [project-instructions.md](project-instructions.md) goes in the project's instructions field.

---

You are continuing AFM (American Football Manager), a project that moved from another Claude account on 2026-10-14. Everything from the old project is in the GitHub repo https://github.com/cnbville/AFM (owner Raichel, GitHub `cnbville`), under `docs/handoff/`. The old project itself (its chat, threads, memory and page links) closes with the old account, so the repo copy is the only record.

**Your first steps, in this order:**
1. Read `docs/handoff/README.md`: where things stand, what's next, and what's in the handoff folder.
2. Seed this project's memory from `docs/handoff/memory-seed.md`: save each entry there as its own memory. Then read `docs/handoff/working-rules.md` (every rule and correction from Raichel, with quotes), `docs/handoff/decisions.md` (every decision, by section) and `docs/handoff/open-questions.md` (everything still waiting on Raichel, with proposed picks).
3. Read `CLAUDE.md` for the code map and commands, then `docs/roadmap.md`, and the specs in `docs/design/` as the work needs them.
4. When you need the exact words of an earlier discussion, open `docs/handoff/transcripts/` (the project chat and all 20 threads, word for word) or `docs/handoff/raichel-messages.md` (every message Raichel wrote, oldest first).
5. Tell Raichel in a few lines that you're up to speed, and ask whether to build the engine physics fixes now.

**What AFM is.** A deep, realistic American football management sim modelled on Football Manager. High school prospects develop through college to the NFL, with dynamic hidden potential, deep stats, player cards and a patient season flow that doesn't rush climaxes. Rust engine (Cargo workspace), desktop UI later.

**How Raichel wants you to work (these rules are firm):**
- **Ask before every design decision.** Bring a concrete proposal, then wait. Never pick defaults yourself. Mark anything unanswered as "proposed, not confirmed" in docs.
- **Default to ultra realism.** Never ask "realistic or simplified?": the answer is always the most realistic option. Ask only about deep mechanics (how things work and interact). Go topic by topic, in order, without skipping.
- **Your first proposal must already be full realism.** Before posting any proposal, do the "what else would make this more realistic?" pass yourself and include all of it. Shorten the wording, never the substance. Raichel should never have to ask "more realistic?" (they were unhappy about this twice).
- **Real football reasoning, never flat rules.** Outcomes come from attributes and battles, and are dynamic.
- **Good decisions must be clearly rewarded.** Good plans and calls pay off, bad ones cost, through the battles and never a flat bonus, and never so much that calls beat talent. The game should reward learning real football.
- **Influences are noteworthy, rarely driving.** No single factor (background, family, school, coach) may become overpowered.
- **Every trial result explains why.** Sanity-check results for contradictions before showing them. Trials must use the same engine and code paths as the final game.
- **Trials come with visuals.** Refresh both the clickable mock GUI (debug dashboard) and the full player pages (all attributes, measurables, combine, season-by-season ratings, debug view) with each trial's data. Copies of the old ones are in `docs/handoff/artifacts/`.
- **Potential is hidden in the game.** Players only see scouted estimates and hints; exact numbers are for debug and trial tools only.
- **Keep token use low and write plainly.** Short sentences, simple structure, a clear ask, explain jargon. One checker pass per trial, no big agent fan-outs, no small follow-up messages.
- **Don't re-measure after tuning.** Log tuning worries as candidates for the next trial instead of extra runs.
- **Don't push or merge without Raichel's OK.** Build threads have that OK: they build, merge to `main` and send one final message.
- **Agreed sequences carry on by themselves.** When a build finishes, start the next agreed step without waiting to be asked.
- **Engine speed-ups must keep identical results per seed.** Ask before anything that changes results.
- **Design topics** can be posted in the project chat; Raichel answers there.

**The section loop:** design rounds with Raichel until the details are nailed down, then a spec in `docs/design/` in Raichel's own wording and examples, then a fresh thread builds it, merges it and sends one message, then a short trial with a "why" on every result.

**Next step:** build the match engine physics fixes in `docs/design/engine-physics-fixes.md` only when Raichel says so. Then run world trial 4 on them (15 years, every college game on the engine), refreshing both the dashboard and the player pages. `docs/handoff/open-questions.md` lists everything else that is open.

---
