# Suggested project instructions

The old project had no project instructions set. Paste the block below into the new project's instructions so every Claude there starts with the same rules. It is short on purpose; the detail lives in the repo.

---

AFM is Raichel's ultra-realistic American football management sim, modelled on Football Manager. Repo: https://github.com/cnbville/AFM (owner Raichel, GitHub `cnbville`). It moved from another Claude account on 2026-10-14.

Before any work, read `docs/handoff/README.md`, then `CLAUDE.md`. The old project's full chat and threads are word for word in `docs/handoff/transcripts/`; decisions are in `docs/handoff/decisions.md`, open questions in `docs/handoff/open-questions.md`, and every working rule in `docs/handoff/working-rules.md`.

How to work:
- Ask before every design decision. Bring a concrete proposal, then wait. Never pick defaults.
- Ultra realism always. Never ask "realistic or simplified?". The first proposal must already be the full-realism version: do the "what else would make this more realistic?" pass yourself. Shorten wording, never substance.
- Go topic by topic, in order, without skipping.
- Real football reasoning: outcomes come from attributes and battles, dynamic, never flat rules. Influences are noteworthy, never driving.
- Good calls and plans are rewarded through the battles, never a flat bonus, never so much that calls beat talent.
- Potential is hidden in the game; exact numbers only in debug and trial tools.
- Every trial result explains why, sanity-checked first. Trials use the same engine and code paths as the final game, and refresh both the mock GUI dashboard and the full player pages.
- Write plainly: short sentences, a clear ask, explain jargon. Keep token use low: one checker pass per trial, no agent fan-outs, no small follow-up messages.
- Mark Claude's unanswered picks "proposed, not confirmed" in docs.
- Build threads build, merge to `main` and send one final reply. Agreed sequences carry on without being asked. Don't re-measure after tuning; log it for the next trial.
- Engine speed-ups must keep identical results per seed; ask before anything that changes results.
- Design topics can be posted in the project chat; Raichel answers there.

Next step: build the engine physics fixes (`docs/design/engine-physics-fixes.md`) only when Raichel says so, then world trial 4.
