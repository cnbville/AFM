# The line bank run

About 100,000 new versions for the game's talks (Cain 2026-11-07, "yes on all" to `afm/line-bank-api/proposal.md` in the project folder): one-on-ones 52,000, game day 23,000, staff 7,000, recruiting 6,000, the AD's office 4,000, agents 4,000, the room 3,000, boosters 1,000. Fans, press and story quotes get none. Claude Sonnet 5.5 writes them through the Message Batches API (half price, no prompt caching), code and one model pass check them, and they land in `crates/afm-engine/data/lines/<family>_run.txt`.

## Files

- `parts.py`: every part the run fills (who says it, its facts and when-checks, what it is, its length) and the split: each family's total and, inside it, the groups the proposal names, each filled thinnest first to an even count. `python3 parts.py` prints the split.
- `check.py`: the checker, every rule from lane 13's tools in one place and strict (a line that breaks one is dropped and its part asks again): the format, tags and facts each part's speaker can have, when-checks, tags that never meet, level words against the levels a line is said at, family, spouses, days, swearing, talking back without `brash` (and never in high school), era tags, names that read wrong at a level ("the {them}" off the NFL, "a {team}"), exact and near repeats (75% shared words) against the bank and the run, and each two-word opening at most 1 in 20 of a part. `python3 check.py --bank` runs the rules over today's bank (no repeat checks) to show how strict they are: 1-8% of today's kept lines would be dropped, most of them real (tags that never fit, days, family, a level's words on an untagged line).
- Held lines (talk facts lane L2): a one-on-one line that denies or admits something about his own conduct ("I've done nothing wrong", "I know I messed up") carries `blameless=yes` or `at_fault=yes`, facts nothing sets yet, so the game does not say it until the talks' fact sheet backs it. `check.py` drops a new `talk.` line that says one of the blatant forms (`CLAIM`) and rejects either fact on a writer's line; `python3 check.py --bank` allows them on the bank's own.
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
python3 tools/linebank/run.py repool       # read the saved checker replies again (after a parser fix) and pool what now passes; no call
python3 tools/linebank/run.py merge        # write the eight <family>_run.txt files
```

`--writer` and `--checker` pick each step's model (`claude-sonnet-5-5`, the default for both, or `claude-haiku-5-5`). The top-up after the run (Cain 2026-11-08) wrote with Haiku and checked with Sonnet: `run.py run --writer claude-haiku-5-5 --cap <dollars>`. Haiku can't be the checker: on 4,429 Haiku lines Sonnet's checker flagged it caught 9% of Sonnet's flags (33% with thinking on), and what it missed were real problems (`afm/line-bank-api/haiku50/` in the project folder).

Each round sends one write batch (one part per request, up to 120 versions, at most 12 requests a part, each with its own angle, its own sample of up to 600 existing versions and the openings the part has nearly used up), runs every reply line through `check.py`, then sends one checker batch (Sonnet 5.5 reads each kept line once, 250 to a request, may think aloud, and ends with `FLAGS:` and the lines that break a rule; only that list counts, and a flag it takes back in its last words, "fine; no flag" or "(ignore)", doesn't). Lines that pass both join the pool; the next round asks only for what's still short. The thinnest parts (by tenths of their target) go first, then the parts the proposal fills first, so a round the cap trims leaves no part with nothing; a trimmed round is the last one. Thinking is at its lowest setting (`between_tools`; Sonnet 5.5 refuses `disabled`), but the model may still think first, so a reply cut at its token limit loses its last line. The Batches API refuses the server-side `fallbacks` parameter, so a refused or failed request is just counted and refilled next round.

**Money.** Every batch's usage is added up at its model's batch price (Sonnet 5.5 $1 in and $5 out per million tokens, Haiku 5.5 $0.05 and $0.25; `MODELS` in `run.py`). Before a batch goes out, its worst case (every request writing to its token limit) must fit under the cap (`--cap`, $40 by default, $3 for the pilot), or the batch is trimmed; the run keeps $2 for the last checker pass. The key itself is capped at $45 in the Console.

**State** lives in `/mnt/project-files/afm/line-bank-api/run/` (`--state`): `state.json` (batch ids, spend, counts, and which step each batch has reached), `pool/<part>.txt` (kept lines), `replies/<batch>.jsonl` (each batch's replies), `kept/<batch>.json` (a write batch's lines that passed the code check), `dropped/<batch>-code.tsv` and `-checker.tsv` (every dropped line and why), `log.txt` and `report.md` (counts, keep rate, cost and sample lines per part). Every step is saved before the next, so a run that stops anywhere (a machine restart) picks up where it was: `run.py run` first finishes any batch in flight and any lines not yet checked or pooled, and adds nothing twice.

## The recruiting talks job (2026-11-12)

A second job on the same machinery (Cain: 20,000 versions for the recruiting one-on-ones): `recruit20k.py` holds its 96 parts and targets, `briefs/recruittalks.md` its rules, and `run.py --job recruit20k` runs it into its own state folder (`/mnt/project-files/afm/line-bank-api/recruit20k/`) and its own file, `crates/afm-engine/data/lines/recruit_talks_run.txt` (`merge` writes only that file for a job). The family is `recruittalks`; its speaker is the union `recruit` (`parts.py`: both sides' tags; a line tagged for one side is said only by that side, an untagged line by both). Differences from the first run:

- Parts carry facets (`FACETS` in `recruit20k.py`): each request covers three of them, 60 lines at most, so a narrow part does not come back as near copies.
- A `--parts key:count,...` run fills only those parts (their pool counts against the given count): use it to aim a round at parts that are still empty, since a cap-trimmed round otherwise takes the biggest shortfalls first.
- `check.py` adds: no untagged we, us or our, no coach names, no tag word written into a line, and one key phrase never used as a refrain (a phrase shared by too many lines of a part).
- A level never reaches this family: the tag `college` means the transfer side, and the level-word rules are off for it.
- The worst case of a round has to fit under `--cap` minus $2, so a round trimmed by the cap is the last; start `run` again to continue. The cost came to $10.92 for 12,253 versions (`docs/design/recruit-talk-lines.md`).
- `python3 recruit20k.py --doc <path>` writes the part list (who says each key, what it takes as true, its facts).

## After the run

1. `run.py merge`, then empty `TODAY` in `crates/afm-engine/src/tests/line_seasons.rs` so every family meets the 2% bar.
2. `cargo test -p afm-engine --lib -- tests::lines tests::line_seasons tests::talks_matter tests::portal_talks tests::fans`, and the one-on-ones' 0.5% (`a_season_of_talks_counts_its_repeats`).
3. A data-only PR with the eight files, the emptied list and the counts in `docs/reference/how-it-works/people.md`.
4. Remind Cain to delete `AFM_LINEBANK_KEY` from the environment.
