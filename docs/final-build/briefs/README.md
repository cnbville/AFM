# Lane briefs (final build, 2026-10-21/22)

Each lane of `../plan.md` that runs in its own project thread reads its brief here. The build thread ("Final build") merges every lane.

## How a lane thread works

- Cain is asleep. No okays are possible: decide every fork yourself, picking the most realistic and most polished option, and log it in the docs under "Decided in the final build (Claude's pick)".
- Do not post to Cain. Work the whole lane through without stopping. Every turn must still end with a hearthbot call: use `no_reply_needed` while working.
- Your container is yours alone (4 cores), so builds, small worlds and match checks are fine.
- Start from the latest `origin/main`. If a `claude/final-wip-laneN` branch exists for your lane, an earlier attempt left work in progress there: read its diff (`git diff origin/main...origin/claude/final-wip-laneN`) and continue from it if it's sound.
- Follow the shared rules in `../plan.md` (code layout, saves, seeds, memory gate, checks, docs).
- Before each PR: rebase on the latest `origin/main` (other lanes merge during the night), fix conflicts, `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and the tests your change touches.
- Open PRs to `main` with the GitHub MCP tools, titled "Final build lane N: ...". Never merge them; the build thread merges when CI is green. Every PR body begins with exactly these two lines:

```
<!-- claude-projects-attribution: {"github_login":"cnbville"} -->
_Requested by **Cain** · [project thread](https://claude.ai/code/project/chan_013Ukratj7v15wm7eeTHSDRz?thread=cmsg_013Ukratj7v15wm7eeTHSDRz1PYbqD2iAmXaoXJwsS26pw)_
```

- Commit messages end with `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`.
- Big lanes open PRs in pieces as each passes, so the build thread can merge early.
- When the lane is done (or if you must stop), send the build thread (thread id `cmsg_013Ukratj7v15wm7eeTHSDRz1PYbqD2iAmXaoXJwsS26pw`) one `message_thread` report under 300 words: PR numbers, what's done, measurements, anything unfinished, honest doubts. Then resolve your thread.
- Quality bar (Cain, 2026-10-21): "this is the final build ... it has to be the finished outlay, product ... Previously getting 95% was acceptable but not anymore. You need to think intuitive, think about aesthetics. Make it a AAA game ... don't just focus on code, focus on making it a good game." No stubs, no placeholders, nothing "later".
- Driving the screens: `cargo run --release --bin afm_play`, then Playwright (Chromium installed, `PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers`; node's `playwright` package is global) against http://127.0.0.1:7700. A small world: `Game::call("build_world", {"seed": 7, "history_years": 2, "scale": 0.05})`. `tools/seat-ui/crawl.mjs` clicks through a seat. Look at screenshots critically and fix what's ugly or wrong.
