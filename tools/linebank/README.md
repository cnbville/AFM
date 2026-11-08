# The line bank run

About 100,000 new versions for the game's talks (Cain 2026-11-07, "yes on all" to `afm/line-bank-api/proposal.md` in the project folder): one-on-ones 52,000, game day 23,000, staff 7,000, recruiting 6,000, the AD's office 4,000, agents 4,000, the room 3,000, boosters 1,000. Fans, press and story quotes get none. Claude Sonnet 5.5 writes them through the Message Batches API (half price, no prompt caching), code and one model pass check them, and they land in `crates/afm-engine/data/lines/<family>_run.txt`.

## Files

- `parts.py`: every part the run fills (who says it, its facts and when-checks, what it is, its length) and the split: each family's total and, inside it, the groups the proposal names, each filled thinnest first to an even count. `python3 parts.py` prints the split.
- `check.py`: the checker, every rule from lane 13's tools in one place and strict (a line that breaks one is dropped and its part asks again): the format, tags and facts each part's speaker can have, when-checks, tags that never meet, level words against the levels a line is said at, family, spouses, days, swearing, talking back without `brash` (and never in high school), era tags, names that read wrong at a level ("the {them}" off the NFL, "a {team}"), exact and near repeats (75% shared words) against the bank and the run, and each two-word opening at most 1 in 20 of a part. `python3 check.py --bank` runs the rules over today's bank (no repeat checks) to show how strict they are: 1-8% of today's kept lines would be dropped, most of them real (tags that never fit, days, family, a level's words on an untagged line).
- `run.py`: the run. `briefs/`: the general rules (`rules.md`), one brief per family and the checker's (`checker.md`).
- `bank.py`: the bank's files, tag and fact words, and the parsing every tool shares.

## Running it

The key is the environment variable `AFM_LINEBANK_KEY` (never `ANTHROPIC_API_KEY`, so Claude Code sessions can't take it as their own login). A session sees a new environment variable only when it starts fresh. The calls go straight to `https://api.anthropic.com` (set explicitly, since thread machines set `ANTHROPIC_BASE_URL`). The script never prints the key.

```
python3 tools/linebank/run.py key          # the key is set and a free models call accepts it
python3 tools/linebank/run.py dry --pilot  # the pilot's requests and cost, no API call
python3 tools/linebank/run.py pilot        # about 2,000 lines over every family (about $0.75), then report.md
python3 tools/linebank/run.py run          # the full run: rounds until every part is full, stops before $40
python3 tools/linebank/run.py status       # batches, money spent, counts
python3 tools/linebank/run.py merge        # write the eight <family>_run.txt files
```

Each round sends one write batch (one part per request, up to 150 versions, at most 12 requests a part, each with its own angle and its own sample of up to 600 existing versions), runs every reply line through `check.py`, then sends one checker batch (Sonnet 5.5 reads each kept line once, 400 to a request, and flags what breaks a rule). Lines that pass both join the pool; the next round asks only for what's still short. Thinking is at its lowest setting (`between_tools`; Sonnet 5.5 refuses `disabled`). The Batches API refuses the server-side `fallbacks` parameter, so a refused request is just counted and refilled next round.

**Money.** Every batch's usage is added up at the batch price ($1 in, $5 out per million tokens). Before a batch goes out, its worst case (every request writing to its token limit) must fit under the cap (`--cap`, $40 by default, $3 for the pilot), or the batch is trimmed; the run keeps $2 for the last checker pass. The key itself is capped at $45 in the Console.

**State** lives in `/mnt/project-files/afm/line-bank-api/run/` (`--state`): `state.json` (batch ids, spend, counts), `pool/<part>.txt` (kept lines), `dropped/` (every dropped line and why), `replies/` (each batch's raw replies), `log.txt` and `report.md` (counts, keep rate, cost and sample lines per part). A run that stops resumes where it was: `run.py run` first finishes any batch still in flight.

## After the run

1. `run.py merge`, then empty `TODAY` in `crates/afm-engine/src/tests/line_seasons.rs` so every family meets the 2% bar.
2. `cargo test -p afm-engine --lib -- tests::lines tests::line_seasons tests::talks_matter tests::portal_talks tests::fans`, and the one-on-ones' 0.5% (`a_season_of_talks_counts_its_repeats`).
3. A data-only PR with the eight files, the emptied list and the counts in `docs/reference/how-it-works/people.md`.
4. Remind Cain to delete `AFM_LINEBANK_KEY` from the environment.
