# Page format for the how-it-works reference

Every page in this folder follows the same shape, so a thread can find a fact in seconds.

- Title: `# <System>: how it works today`.
- Line 2: `Checked against main at <short commit> on <date>.` Update it whenever you refresh the page.
- `## Quick answers`: 8-20 one-line answers to the questions Cain is most likely to ask, each with its source, e.g. "Does the elusiveness check read weight? Yes, mass in the tackle-break roll (`game/scrimmage.rs`, `break_tackle`)."
- Then one `##` section per sub-system. Plain words, short sentences. Give the real numbers: scales (1-20, 0-100), formulas, weights, caps, ages, dice. Every fact ends with its source in backticks: `path/file.rs`, `function_name` (path relative to `crates/afm-engine/src/` unless it starts with `crates/` or `app/`).
- Say "what it reads" and "what it does not read" when a check could plausibly read something it doesn't.
- `## Where the knobs are`: the constants and tables that tune this system.
- `## Related pages`: links to other pages here.
- No design opinions, no proposals, no "should". Only what the code does today. If code and a design doc disagree, the code wins and the page says so in one line.
