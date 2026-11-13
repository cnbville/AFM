# Live talks: how it works today

Checked against main at 7691db7 on 2026-11-13 (lane 2, the boxes, before it merged to main; lane 1 is on main since #314). Lanes 3 to 10 are not built yet: their sections below are empty until each lane fills its own.

Live talks are typed lines to your players, recruits and staff, answered in character by Claude Haiku 5.5, on top of the chips and the line bank, which always still work. The rule under all of it: the AI only reads your words and writes theirs; the game decides every result. The design is `afm/live-talks/proposal.md` in the project folder; this page says what the code does.

## Quick answers

- Which model do live talks use? Claude Haiku 5.5, `claude-haiku-5-5`, thinking off, structured output when a schema is given, the fixed rules marked for the cache (`crates/afm-ai/src/client.rs`, `MODEL`, `Request::body`).
- Where is the API key kept? In memory on `Game.ai`, and on a Mac also in the login Keychain (service "AFM"). Never in the world, a save, a world file, a log, the game folder or a screen (`crates/afm-game/src/live_talk/key.rs`, `Ai`; `app/src-tauri/src/keychain.rs`).
- Who reads the key from the Keychain? The app, once at start, and it hands the key to the game. The app saves it when `ai_key_set` works and removes it on `ai_key_clear`, with the game unlocked, since macOS may stop to ask the person; `kept` reads "keychain" only after a save worked (`app/src-tauri/src/main.rs`, `main`, `follow_keychain`, `keychain_status`).
- Does a key survive an app update? Not checked yet. The app is ad hoc signed, so macOS may ask for the Mac password again after an update. It is the first check on Cain's Mac.
- What does the browser build use? `AFM_AI_KEY` for a key, a name of the game's own. `AFM_AI_FAKE=1` plays every call on the fake instead, with no key and no cost (`crates/afm-game/src/bin/afm_play.rs`, `live_talk_from_env`). The desktop app never reads either.
- How long can a call take? 6 seconds in all, connecting and reading the answer included. A redirect is never followed, so the key goes only to the address the client was built with (`crates/afm-ai/src/client.rs`, `TIMEOUT`, `Real::build`).
- What counts as a failure? No key, no network, a timeout, an API error, a refusal, the credit or the key's own limit gone, the game's cap reached, 32 questions already open, or an answer that can't be used (cut off, empty, not the JSON asked for). All of them mean "play on the chips" (`crates/afm-ai/src/client.rs`, `Failure`).
- What is the cap? $50 a month, by calendar month in UTC, counted from each reply's `usage` at list price. At the cap every call fails before it is sent (`crates/afm-ai/src/spend.rs`, `CAP_NANO`; `crates/afm-ai/src/lib.rs`, `Client::call`).
- Where is the spend kept? `ai-spend.json` in the game folder, written to `.partial` and renamed. The fake's spend goes to `ai-spend-fake.json` so a test run never counts against the real month (`spend.rs`, `Spend`; `live_talk/key.rs`, `use_fake`).
- Is a refusal paid for? Yes: any reply with `usage` is counted, a refusal and a cut-off reply included, before it is turned into a failure (`crates/afm-ai/src/lib.rs`, `Client::call`).
- Can the game call a model with no price in the table? No. It fails before anything is sent, so the cap can't miss a cost (`spend.rs`, `Price::of`).
- Does a call run inside a command? No. A command returns a ticket at once, a worker thread makes the call, and `ai_poll {ticket}` hands the result back under the game lock (`live_talk/tickets.rs`, `start`, `poll`).
- What happens after a failure that will repeat (the API refuses the key, the credit is gone)? The boxes stay hidden (`usable` is false) until a new key or a good Test (`live_talk/key.rs`, `note_failure`, `why_not`).
- Does a reply change today? No. `live_talk::after` only tidies old tickets in lane 1; the core (lane 3) fills it (`live_talk/mod.rs`, `after`).

## The contract with the screens

`live_talk::after` adds these to a reply, only when a key is set (or the fake is on) and the cap is not reached. Lane 1 defines them and `ai_demo` plays them; lane 3 fills `after`. A lane that needs a new field adds an optional one and writes it in its own section.

```
live_talk::after adds these to a reply, only when a key is set and the cap is not reached:
  "say": {"cmd": "people_say", "args": {"id": 12}, "limit": 400}
      the talk box
  "ask": [{"cmd": "ai_ask", "args": {"kind": "analyst"}, "limit": 400},
          {"cmd": "ai_ask", "args": {"kind": "report", "player": 88}, "button": "Write the report"},
          {"cmd": "ai_ask", "args": {"kind": "source", "player": 88}, "limit": 400,
           "choose": {"key": "source", "options": [{"value": 5, "label": "His high school coach"}]}},
          {"cmd": "ai_ask", "args": {"kind": "leads", "mail": 31}, "auto": true}]
      a list: an entry is a question box (limit), a button with no text (button),
      or a call the page makes once as it opens (auto); choose adds a short pick list
No flag, no box. A box sends cmd with args plus {"text": "..."} and gets {"ticket": 7}.
ai_poll {"ticket": 7} answers one of:
  {"state": "wait"}
  {"state": "unsure", "readings": [{"pick": 3, "heard": "a promise of more snaps, warm"}, ...]}
      the box shows them, then sends cmd with {"ticket": 7, "pick": 3}
      a pick that can't be undone always comes back this way, with its one reading
  {"state": "done", "heard": "...", "marks": [{"text": "...", "mark": "true"}], "reply": {...}}
      reply is the screen's normal reply; marks are "true", "false", "priced", "later"
  {"state": "none", "note": "He doesn't follow."}
  {"state": "off", "note": "..."}
      the box hides for this talk and the chips stay
For an ask, done carries one of:
  {"table": ..., "line": "...", "query": [...]}   a search that ran (the squad bar gets the query and the line)
  {"text": "..."}                                 a scout's answer, a report, a source's answer
  {"reload": true}                                the page asks for itself again (the leads mail)
ai_checks {} gives a ticket; done carries {"rows": [{"name", "score", "of", "bar"}], "cost", "seconds"}.
On a yes to question 5, the same step adds two lists that need no key:
  "facts": {"cmd": "fo_fact", "args": {"id": 12}, "list": [{"fact": 2, "label": "..."}]}
      chips on a contract talk, a trade call or a NIL form; a chip sends cmd with args plus {"fact": 2}
  "sources": {"cmd": "scout_source", "args": {"player": 88}, "list": [{"source": 5, "label": "..."}], "topics": [...]}
      a row on the scout file; it sends cmd with args plus {"source": 5, "topic": 1}
  both answer with the page's normal reply
```

## Lane 1, the pipe

### The commands

- `ai_status`: what the settings card shows. `key_set`, `key_ending` ("...abcd", never more), `kept` ("keychain" or "memory", when a key is set), `fake`, `usable` (a box may show), `month` ("2026-11"), `spent` (dollars), `cap` (50), `cap_reached`, `calls` (this month), and `note` (one plain line when something is wrong: a lasting failure, the cap, the last failure, or an unreadable spend file) (`live_talk/key.rs`, `status`).
- `ai_key_set {key}`: checks the shape (`sk-ant-`, letters, digits, `-` and `_`, 20 to 300 characters, trimmed first) and keeps the key in memory. It answers with the status. In the app the reply may also carry `keychain_note`, a plain line, when the Keychain would not take the key (`crates/afm-ai/src/client.rs`, `Key::parse`; `app/src-tauri/src/main.rs`, `follow_keychain`).
- `ai_key_clear`: forgets the key, and the app removes it from the Keychain.
- `ai_test`: one tiny call ("Reply with the single word ok."), to see that the key is accepted. It answers `{"ticket": n}`; the poll's `done` carries `text` ("The key works."), `seconds`, `cost` and a fresh `status`. It goes through even after a lasting failure, since a good test is how that is cleared (`live_talk/mod.rs`, `test`).
- `ai_poll {ticket}`: "wait" until the call is back, then the ticket's answer in the contract's shapes. A ticket is answered once; an unknown or old one answers `off` with "That answer is gone." (`live_talk/tickets.rs`, `poll`).
- `ai_demo {state?}`: on the fake only. With no state it lists the states and the flags a reply carries (the contract's `say`, `ask`, `facts`, `sources`). With a state (`wait`, `unsure`, `done`, `none`, `off`, `cap`, `table`, `text`, `reload`, `checks`, `ask_first`) it gives a ticket that answers "wait" twice and then that state's answer (`live_talk/demo.rs`).

### Tickets

- A ticket is a worker thread, a slot for its result and a next step. `start` refuses (before anything is sent) when there is no client, the cap is reached, a lasting failure stands (unless forced), or 32 tickets are open (`tickets.rs`, `start`, `MAX_OPEN`).
- The next step runs under the game lock when `ai_poll` finds the result. It returns `Step::Done(answer)` or `Step::Call(request, next)`, which starts a second call on the same ticket. A typed line is the reader, the game's step, then the writer; the core fills those steps (`tickets.rs`, `Step`, `Next`).
- A panic in a call becomes an API failure, so a ticket never waits for ever. A ticket is dropped 3 minutes after its call started (a second call in the chain starts the count again); a call times out in 6 seconds, so only a ticket nobody came back for is ever dropped (`tickets.rs`, `spawn`, `TTL`, `prune`).
- Tickets live on `Game.ai` and are many at once; `Game.job` holds one job and refuses a second, which is why this is its own map.

### The client

- `Model` has one call. The real client is on `ureq` and sends one POST to `https://api.anthropic.com/v1/messages` with the headers `x-api-key` and `anthropic-version: 2023-06-01`. The base URL is written out because thread machines set `ANTHROPIC_BASE_URL` (`crates/afm-ai/src/client.rs`, `Real`, `BASE_URL`, `VERSION`).
- The body: model, `max_tokens`, `thinking: {"type": "disabled"}` (Sonnet 5.5, which answers that with a 400, gets `{"type": "between_tools"}`; no lane uses Sonnet yet), the rules as the system text (marked `cache_control: ephemeral`), the card and line as the user turn, and `output_config.format` with the schema when there is one (`client.rs`, `Request::body`).
- HTTP errors become failures: 401 and 403 "The key was not accepted"; 429 and 5xx "busy"; a message that names the credit balance or a usage limit is "credit gone"; the key is taken out of any message (`client.rs`, `status_failure`, `Key::scrub`). A call that never got an answer is a timeout if it timed out, else no network.
- A reply that came back but can't be used (a refusal, cut off at the token limit, empty, or not the JSON asked for) is a failure after it is counted (`client.rs`, `Reply::classify`).
- The fake plays recorded replies by the request's label ("test", "reader", "reader_first", "writer"), queued replies and failures first. Its recordings `test`, `reader` and `reader_first` are real Haiku 5.5 replies, kept from lane 1's real test on 2026-11-13; `writer` is written by hand from the proposal's card until a lane records a real one (`crates/afm-ai/recordings/`). It keeps every request it was asked. `AFM_AI_FAKE=1` gives it 0.7 s of latency so a screen can show its waiting state (`crates/afm-ai/src/fake.rs`).
- With the fake in use the boxes are on without a key (`usable` is true). Without it a key is needed.

### Spend

Prices are per million tokens. Money is counted in billionths of a dollar.

| Model | Input | Cache read | Write, 5 min | Write, 1 hour | Output |
|---|---|---|---|---|---|
| Claude Haiku 5.5, prompt up to 100,000 tokens | $0.10 | $0.01 | $0.125 | $0.20 | $0.50 |
| Claude Haiku 5.5, prompt over 100,000 tokens | $0.50 | $0.05 | $0.625 | $1.00 | $2.50 |
| Claude Sonnet 5.5 (build-side tools only) | $2 | $0.10 | $2.50 | $4 | $10 |

(`crates/afm-ai/src/spend.rs`, `Price::of`, `Price::cost`, `LONG_PROMPT`.) Source: the claude-api skill's model table and its Haiku 5.5 notes, read 2026-11-13. The cache writes are 1.25 times and 2 times the input price. A prompt is the fresh input plus the cache read plus the cache writes.

- The book keeps the last 24 months. A file that can't be read is set aside as `ai-spend.bad` and counting starts again from zero, with a note (`spend.rs`, `Spend::open`, `KEEP_MONTHS`).
- A call that fails with no reply (a timeout, no network) is not counted: nothing says what it cost.

### Tests

- Quick check (`cargo test -p afm-ai`, `cargo test -p afm-game --lib live_talk::`): the key's shape and its Debug; the request body; the usage and the cache split; the price table; the months; the cap and its reset; the book on disk; the fake; the real client against a local server (what it sends, each HTTP error, a timeout, nothing listening); a game with no key builds no client; the marker key reaches no file of the game folder; the ticket flow and a chained second call; every failure kind; the cap from a file; the demo.
- Full tests (`crates/afm-game/tests/ai_key_bug_pack.rs`): the same marker after a bug pack on a small world.
- One ignored real test (`crates/afm-ai/tests/real_calls.rs`, `real_three_calls`): three calls on the real API, under a cent. It prints "skipped" with no key.

## Lane 2, the boxes

Built on 2026-11-13 (lane 2, screens only: `app/ui/screens/ai.js`, `say.js`, `ask.js`, `ai.css`, and one call at each place). The frames are lane 0's, built as drawn (pick 19). No Rust changed, no API call is made by these files: a box only sends the command its reply's flag names, then asks `ai_poll`.

### What a place does

- One call puts a box on a screen: `aiSayBox(reply, key, options)` for the talk box, `aiAskBox(reply.ask, kinds, key, options)` for the ask box, `aiFacts(reply, key, options)` for the fact chips, `aiScoutPanel(file)` for the scout file, `aiSettingsPanel()` for the settings card. A box shows only when the reply carries its flag (`say`, an `ask` entry of the place's kind, `facts`, `sources`), or the demo switch is on and the place's own preset stands in. No flag, no box and no gap (`ai.js`, `aiFlag`; `ask.js`, `aiAskItems`).
- The box's state is kept by a key per talk or page (`ai.js`, `aiBox`), so a screen that draws again finds it as it was. It is not saved, and a typed line is never kept anywhere but the game's own log.
- The places: the scene (`people.js`, `screens.people_scene`, key `scene:<id>`) and the face to face one-on-one (`talk.js`, `tkFaceToFace`, same key); the pregame and halftime room (`talk.js`, `tkChoose`, key `talk:<moment>`); the contract talk (`fo-nfl.js`, `fo_talk`, key `fo:<id>`) and the trade call (`fo_trades`, key `fo-trade`); the NIL promise on a recruit's file (`fo-college.js`, `fcActions`, key `nil:<id>`); the stat finder (`trends.js`, key `statfind`); Find recruits and Find prospects (`find.js`, keys `find:college`, `find:nfl`); the squad bar (`quickfilter.js`, `qfBar`, now `qfBar(scope, groups, rows, shown, ask)`, called with `x.ask` from `app.js`, `training.js` and `squadboards.js`); the next opponent (`desk.js`, `screens.nextopp`); the scout file's Summary tab (`scouting.js`, `scFileSummary` plus `aiScoutPanel`); the leads mail (`core.js`, `showLetter`, `letterHtml`); the settings card (`core.js`, `screens.settings`, before the Logos panel, and `pregame.js`, the settings dialog).

### The states of a box

Ready and typing (a counter out of the flag's `limit`, default 400; Enter sends, Shift+Enter is a new line; the button wakes with text), wait (the words stay, the button waits, one line says who is reading), unsure (two or more readings to pick from, "Say it this way" or "Say it another way"; one reading, which a step that can't be undone always has, is a confirm card with "Do it" and "Not that"), none (the note in bold, "Your words are still in the box"), off (one note in place of the strip, once; after that no box), done. A pick sends the box's `cmd` again with `{ticket, pick}`; the answer may be a new ticket, an answer in the poll's shapes, or the page's normal reply, and the box takes any of them (`say.js`, `aiPicked`; `ai.js`, `aiSettle`).

Done, for a talk box: the box clears; "Heard as" and the marks go under your line in the talk's log (matched by the text you typed, or read from `heard` and `marks` on the log's own line when the core puts them there: `ai.js`, `aiYours`); the page draws again from its own reply, or runs the place's `apply` (a contract talk takes `reply.answer` as the agent's answer, as an offer's reply does: `fo-nfl.js`, `foAiApply`). On a page with no log of its own, or in the demo, the box shows your line, how it was heard and `reply.text` under it.

Done, for an ask box: a `query` is handed to the page's own filters and the page runs the search (the stat finder, Find and the squad bar take it, in `ask.js`, `aiQueryStat`, `aiQueryFind`, `aiQuerySquad`), with the set fields lit gold and a note under the strip; without such a page (the opponent card) the chips, the `line` and the `table` show in the box; `text` shows as an answer card by the scout, the source or "Scouting report"; `reload` draws the page again.

### Added fields (proposed, not confirmed; all optional, so lanes 3 to 10 may ignore them)

- A `query` entry is `{field, value}` with optional `op`, `v2`, `label` and `words` (what the chip says). The stat finder takes `field` as one of `find, level, division, state, from, to, pos, class, rs, age_min, age_max, min_len` and a condition as `{field: "cond", stat, op, value}`; Find takes `pos, class, level, place, q, sure, unseen` and any field of its catalog (`op` one of `ge, le, between, is`); the squad bar takes `pos`, `q` and any field of its catalog (`op` one of `<, ≤, =, ≥, >`).
- A mark may carry `when` ("Saturday": "Checked Saturday"). A reading may carry `why` (the line on the confirm card, "It goes on his record, and the room sees it"; else "It can't be taken back once it is done.").
- An `ask` entry may carry `label`, `placeholder`, `caption`, `name`, `role`, `by`, `byNote`, `wait` (the wait line; the leads mail's default is "<first name> is picking the best few and writing why.") and `rerun {cmd, args}`: without `rerun` the opponent card's chips are read-only (the frame draws them with a ×), since only the AI can read a new question.
- A report's `text` may hold headings as lines that start with `## `.
- The leads mail's `letter` reply carries `ask` with the `auto` entry; the page makes the call once as the mail opens and reads the mail again on `reload`.

### The demo switch

Off by default, and only while `ai_status.fake` is true (`AFM_AI_FAKE=1`): the browser build on a real key never meets it. Settings has a Demo row (or add `?aidemo` to the address). While it is on, every place shows its box against its preset flag, every call goes to `ai_demo` with the state the bar at the foot of the window names, a fact chip or the sources row only marks itself, and the leads preset runs on any memo (`ai.js`, `aiDevOn`, `aiDevPaint`, `aiCall`).

### The settings card

`ai_status`, `ai_key_set` (the key field is a password field; Enter saves, also inside the pregame dialog), `ai_key_clear`, `ai_test` (the card says "Works. One small call, 0.6 s." or the refusal's line) and `ai_checks` (rows with a bar and a tick at the bar to clear; amber only under the tick; cost and seconds) (`ai.js`, `aiCardHtml`, `aiCardAct`). With no key the card also says the game plays on the chips. With a key and the cap reached it says so with the day the cap resets.

### Sizes (proposed, not confirmed)

The text field grows up to 140 px; the poll asks every 450 ms; the confirm card's default line, the "off" note's tail ("The chips work as before." and the filters', bar's and file's versions) and every box's own words are lane 0's where the frame has them and mine where it has not.

## Lane 3, the core

Not built yet.

## Lane 4, the desk

Not built yet.

## Lane 5, rooms on today's choices

Not built yet.

## Lane 6, rooms that gain something

Not built yet.

## Lane 7, NFL desk and your boss

Not built yet.

## Lane 8, the off-field price

Not built yet. It will update [nfl.md](nfl.md).

## Lane 9, one-on-ones typed

Not built yet.

## Lane 10, college desk

Not built yet.

## Where the knobs are

- `crates/afm-ai/src/client.rs`: `MODEL`, `VERSION`, `TIMEOUT`.
- `crates/afm-ai/src/spend.rs`: `CAP_NANO`, `CAP_USD`, `LONG_PROMPT`, `KEEP_MONTHS`.
- `crates/afm-game/src/live_talk/tickets.rs`: `TTL`, `MAX_OPEN`.

## Related pages

- [README.md](README.md)
- [saves.md](saves.md)
- [people.md](people.md)
