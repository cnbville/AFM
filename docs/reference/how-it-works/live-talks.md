# Live talks: how it works today

Checked against main at 7691db7 on 2026-11-13 (lane 2, the boxes, before it merged to main; lane 1 is on main since #314), lane 3, the core, over main c04c172 on 2026-11-13, before it merged, and lane 4, the desk, over main 65b1d0a on 2026-11-13, before it merged. Lanes 5 to 10 are not built yet: their sections below are empty until each lane fills its own.

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
- Does a reply change? Only with a usable key: a scene that takes a typed line gets `say`, and a poll's nested reply gets it too (`live_talk/mod.rs`, `after`; `say/mod.rs`, `flag`).
- Can a typed line do something a chip can't? No. It lands on one of the scene's own options through the chip's code, and the bank picks its line with the same seeds (`world/folk/scene.rs`, `say`, `land`).
- Can a typed line fire a coach without asking? No. A pick that can't be undone comes back as one reading to confirm, and `scene::say` refuses it unconfirmed (`world/folk/finals.rs`; `scene.rs`, `asks_first`).
- What if the AI fails mid-talk? The box hides and the chips go on; if the writer fails, the bank's line shows (`live_talk/say/mod.rs`, `reader_step`, `writer_step`).
- Is "you're my starter" typed to your player checked against the depth chart? No: players never see the chart. It is heard as a promise about what he'll see and handed to the seam lane P1 fills (`world/folk/claims/truth.rs`, `standing_promise`; `talks.rs`, `typed_promise`).
- Is a lie judged? Not yet: the lies store and its daily check exist, and judging costs nothing until lane P1 fills it (`world/folk/claims/truth.rs`, `judge`).

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

The text field grows up to 140 px; the poll asks every 450 ms (a poll stops when its box is no longer the one kept for its key); the ask box's send button says "Ask" (an `ask` entry with a `button` is a one-button box that says it, "Write the report", then "Write it again"); the demo bar floats at the bottom right and a page keeps 96 px under its last row while it is on; the confirm card's default line, the "off" note's tail ("The chips work as before." and the filters', bar's and file's versions) and every box's own words are lane 0's where the frame has them and mine where it has not.

## Lane 3, the core

Built 2026-11-13. A typed line lands on one of a scene's own options through the same code as the chip, so the line bank still picks its line with the same seeds and the world after is the chip's. Your words go on your line for that pick, never on a second line.

### A typed line, step by step

- `people_say {id, text}` checks the meeting (open, not over, a kind that takes a typed line, at most 400 characters), builds the reader's card from the scene's own options and returns a ticket. No call runs in the command (`crates/afm-game/src/live_talk/say/mod.rs`, `people_say`).
- The reader (label `scene_reader`, Haiku 5.5, structured output, at most 400 tokens) answers with an option id (o1 to o12, or none), a close second when unsure, a tone, a promise as read, the facts stated, the people named and its fit: clear, unsure or not a line. The rules and the schema are fixed for every scene, so the cache holds (`say/mod.rs`, `READER_RULES`, `reader_schema`).
- Its answer is applied inside `ai_poll`, under the game lock. Not a line, or no option: `{"state": "none", "note": "He doesn't follow."}` and nothing changes. A chip clicked while the line was read: `off`, "The talk moved on before that landed." Unsure, or a pick that can't be undone: `{"state": "unsure", "readings": [{"pick", "heard", "final"}]}`, and the reading waits (`say/mod.rs`, `reader_step`, `Waiting`).
- Otherwise it lands: `folk::scene::say` runs the scene's own answer with `Scene.heard` set for that one answer, then the same ticket carries the writer's call (label `scene_writer`, at most 200 tokens) for the line the answer wrote for him (`say/mod.rs`, `land`, `go_on`).
- The writer's words take the bank line's place only if they pass the filter and nothing moved meanwhile; the scene keeps what was shown. If the writer fails or is dropped, the bank's line stays (`say/mod.rs`, `writer_step`).
- The scene's log shows a typed line as you typed it: `live_talk::after` puts your words (markup taken out) in the line's `text` and keeps the chip's words as `chip`, with a key or none, so lane 2's box finds your line by its text (`say/mod.rs`, `typed_lines`).
- The poll answers `{"state": "done", "heard": "ask about his scheme and how he'd use your players, calm", "marks": [...], "reply": <the people_scene reply>}`. `live_talk::after` puts the box on the nested reply too (`say/mod.rs`, `done`; `live_talk/mod.rs`, `after`).
- Confirming: the box sends `{id, ticket, pick}` with one of the readings; the answer is a new ticket. A reading waits 10 minutes and is taken once (`say/mod.rs`, `confirm`, `WAIT_TTL`).

### In the engine

- `scene::say(w, id, heard, confirmed)` lands a typed line on one of `scene::options` (the picks and `reach`, never a nav pick) through `land`, the same function `answer` now calls. It refuses a meeting that is over, a kind with no box, an empty line, one over 400 characters, an option not on the table, and a pick that can't be undone unless `confirmed`, with "That can't be undone, so say which you meant first." (`world/folk/scene.rs`, `say`, `land`, `ASK_FIRST`).
- `Heard` (the pick, tone, words, length, facts as stated, a promise as read (or a claim about where he stands, heard as one), a game plan ask and the people named) rides on `Scene.heard` for that one answer and is cleared before and after. It is never saved (`scene.rs`, `Heard`).
- `Said.typed` holds your words on your line for the pick. An answer that ends the meeting with no line of yours (an interview's "Thank him and wrap it up") gets one with the chip's words; a meeting that goes on gets no extra line, since scenes count your lines (`scene.rs`, `put_typed`).
- No box: press conferences, media, story meetings, and one-on-ones until lane 9 fills its hook (`scene.rs`, `NO_BOX`, `takes_typed`; `world/folk/talks.rs`, `ONE_ON_ONE_TYPED`, `one_on_one_say`).
- The picks that ask first: a pick marked `PickKind::Final`, or one of 28 rows by scene kind and code: a December signee's release (70-72), a holdover's job (22, 33, 34), letting a coach go (70-72), every discipline ruling above a warning (501-508), an all-star invitation or a search firm's call (95-98), hiring a high school coach's package (87), and the deals outside the rules (151-153, 181, 191, 192). Codes repeat across kinds, so the kind always decides (`world/folk/finals.rs`, `FINAL`, `is_final`).

### Facts and lies (seams for the later lanes)

- The reader may label facts of 13 kinds: depth_rank, ahead_of_him, role, snaps_share, touches, nil_amount, money, injury, status, stat, off_field, interest, record (`world/folk/claims/truth.rs`, `CLAIM_KINDS`). Each is marked by `claims::check::fact`, which sends a recruit's scene to `check/recruit.rs`, a GM's or boss's to `check/money.rs` and the rest to `check/roster.rs`. All three answer "unknown" today, which shows no mark (`claims/check/mod.rs`, `fact`).
- `Folk.live` keeps the lies (who, what, the day he can see it) and each agent's and GM's trust in your word (0.5 to start). It is saved only when not empty (`claims/truth.rs`, `Live`, `KEEP_LIES`, `WORD_START`).
- A claim about where your own player stands (his place on the chart, who is ahead of him, his role, his snaps, his touches) is a promise about what he'll see, as the promise design agreed on 2026-11-13 says (`afm/promises/proposal.md`, section 11). `scene::say` hears the first such claim as a promise when the line made none of its own: a start for a first place, else his place; ahead of a named teammate; a role; touches; with the claim kept on it. It marks "later", never checked against the staff's chart, and goes to `talks::typed_promise`, the seam lane P1 fills (his belief, judged on his next 2 practice weeks and 2 games). It logs nothing yet (`claims/truth.rs`, `STANDING`, `your_player`, `standing_promise`; `claims/check/mod.rs`, `fact`; `talks.rs`, `typed_promise`).
- The daily check marks a lie seen on its day and then judges it. Judging costs nothing until lane P1 fills it (a broken promise's cost scaled by his belief) (`claims/truth.rs`, `check_lies`, `judge`).
- The cost of a kept or broken promise is now one function, `talks::settle_word`, with the same numbers as before: a player's trust +0.1 kept, -0.22 broken (twice in public), his mood by 1.4 times that, close friends' trust -0.04 each, a grudge, your name for keeping your word +0.04 or -0.08; a cast person or a coach +0.1 or -0.2 (`world/folk/talks.rs`, `check_promises`, `settle_word`).
- New mood reasons, with codes 40 to 42 so a reason added to the list above them never takes their numbers in a save: a lie found out, what his agent heard you say in his contract talks, a recruit's lie (`talks.rs`, `Reason`).

### Cards and the filter

- Every card is built by one function from a whitelist type with words only: the meeting, you, who is across (name, role, class year), the people he may name, the options, the last 6 lines, his line as the game wrote it, the tell your read caught, what the game released in words, facts and example lines. It has no field for potential, true ratings, true wants or hidden trait numbers. Typed words lose `<` and `>` so they can't close their tag (`crates/afm-game/src/live_talk/card.rs`, `Card`, `scene`, `writer`, `quoted`).
- The filter drops a writer's reply that is empty, longer than 600 characters, carries markup, names one of 16 words (rating, overall, potential, ceiling, attribute and their forms, ovr, stat-line, engine, simulation, algorithm, prompt) or holds a number its card doesn't. Numbers count as digits ("1,200", "4.5") or words (two to million, "dozen", "twenty-two"); "one" and ordinals are words, not counts (`live_talk/filter.rs`, `check`, `BANNED`, `numbers`).

### Staff interviews typed (LT18, the proof)

- A typed question lands on one of the interview's four questions (60 scheme, 61 a difficult player, 62 where he recruits, 63 what he wants) or on 69, wrapping up. His answer is written from today's answer and the reads that question puts on his page, in words: a range 9 or wider is "still hard to read"; otherwise its middle 16 and up is "a real strength", 13 "good", 9 "about average", 6 "below average", lower "a weak spot"; ambition 14 "wants more, and soon", 9 "some ambition", lower "content where he is" (`live_talk/say/hiring.rs`, `facets`, `read_words`, `writer`).

### The later lanes' places

- Commands the core registers: `people_say`, `talk_typed`, `fo_say {desk}`, `ai_ask {kind}`, `ai_checks`, and question 5's `fo_fact` and `scout_source`. Until a lane fills its stub, each answers "That box comes with a later update." and no flag shows it (`live_talk/mod.rs`, `call`, `NOT_YET`).
- Each lane's file holds its flag and hooks: `say/{rooms,interview,staff,money,boss,one_on_one,recruit}.rs` (reader, heard, writer, flag; `rules` for its own reader in `staff` and `one_on_one`) and `ask/{analyst,squad,find,opponent,scout,report,leads,source}.rs` (ask, flag; `ask::push` adds an entry to a reply's `ask` list).
- `ai_checks` runs every labelled line of `checks/staff.json` (30 lines: five for each interview question and for wrapping up, five that are not a line), `deals.json` and `one_on_one.json` (empty, lanes 7 and 9) through the reader on one ticket, and answers `{rows: [{name, score, of, bar}], cost, seconds}` (`live_talk/checks/mod.rs`).
- The browser build's fake (`AFM_AI_FAKE=1`) reads a typed line by plain word match against the options and says the game's own line back, so a typed line lands where its words point with no key (`live_talk/say/stand_in.rs`).

### Tests

- Engine (`crates/afm-engine/src/tests/live_core.rs`): a typed question is the clicked one (same world bytes, same count of your lines); the same reading on the same save gives the same world; a line that ends the interview shows your words; nothing off the table or nav is landed; a final pick asks first and no caller can skip it; no box on press, media, stories and one-on-ones; `heard` never kept; the lies store waits; a promise costs what it cost before the move; a claim about where your player stands is heard as a promise and marked "later".
- Game (`live_talk/say/tests.rs`, `live_talk/filter.rs`): LT18 end to end on the fake; typed equals clicked through the commands; not a line changes nothing; unsure and the confirm; a final pick asks first; every failure kind falls back mid-scene; no key, no box; the reads match the interview; the filter on 200 replies; no hidden number reaches 1,000 cards.
- Full tests (`crates/afm-game/tests/live_core.rs`): the same save and reading replay the same, typed equals clicked from a save, and a save made with live talks plays without a key; every reply walked for hidden keys (`tests/common/mod.rs`, `nothing_hidden`, shared by later lanes).

## Lane 4, the desk

Built 2026-11-13 (live talks lane 4, the desk: LT13, LT14, LT15, LT16, LT20, LT21). Typed questions on the pages that read your numbers. The AI picks the question's terms, never a result: every ask is a form the page already has, filled by the reader, checked by Rust against the page's own lists, then run by the page (or the game). No JavaScript changed; lane 2's boxes take the answers. The one engine edit is `Mail.leads` (`crates/afm-engine/src/world/desk.rs`, a byte with a serde default).

- Where it lives: `crates/afm-game/src/live_talk/ask/{analyst,squad,find,opponent,scout,report,leads}.rs`, the Monday mail in `lane_core/mail.rs` (`leads_mail`), and `lane_data/mod.rs` (`analytics` is `pub(crate)` so the mail reads its tables). `analyst.rs` also holds the helpers the other ask files share (`question`, `start`, `reading`, `none`, `line_call`).
- Every ask is `ai_ask {kind, text, ...}` and answers a ticket. The reader's schema and rules are fixed for the page (the page's lists are in the rules, so the cache holds). A refused form answers `{"state": "none", "note"}` and nothing runs; a failed call hides the box (`off`).
- **The analyst (LT15, the stat finder).** Flag: a `stat_finder` reply. The reader fills the finder's form: find (seasons, careers, games, streaks), level, division, from, to, position, class, redshirt, age, streak length and up to 8 conditions on the finder's own numbers (`stat_finder_stats`). Rust refuses an unknown stat or level, a division the game doesn't have, a class that doesn't fit the level, a years range backwards, a streak with no condition, games or streaks at high school and more than 8 conditions (`ask/analyst.rs`, `check`). The game runs the search itself once (`stat_finder`, with the same arguments the page will use) and the writer gives one line from what it found; no writer call when nothing matched. The answer is `{query, line}`; the page runs the search (`ask.js`, `aiQueryStat`).
- **The squad bar (LT15).** Flag: any reply that carries `qf` (your squad's views and other teams' squads), with `prod` saying whether production is offered. The reader fills up to 4 rules from the bar's catalog (`quick_filter::catalog`: your staff's reads 0 to 99, age, height in inches, weight, stars and, where your games are kept, this season's production), positions and a name search. A field not in that catalog (potential, a future number) is refused. The game writes the line ("Showing players at WR or CB, speed under 60.") (`ask/squad.rs`).
- **Find (LT20).** Flag: `find_recruits` or `find_prospects`. The reader fills position, level (college: hs, juco or portal; NFL: all, fbs, fcs or small), class (college), how sure, the not-seen switch, a name and up to 8 filters from the seat's catalog (`lane_find/fields.rs`, `catalog`, minus position, which has its own place). Every key and every option is checked first: a filter on a text or yes-or-no field can only be "is", an option must be one of the field's own words (it goes to the page as its place in the list), a number must parse (`ask/find.rs`, `check`).
- **The opponent (LT16).** Flag: a `next_opponent` reply with no error, carrying the opponent's `ref`. The reader picks the side and one or two spots from the situation table's own list (20 down and distances, 7 field zones, the red zone, goal to go, the last two minutes, quarters, the score and garbage time, `opponent::spots`). The game reads their row and the level's from `situations` (so your charting's limits apply: no charting, no answer, with the page's own source line as the note), builds the table, and the writer gives one line (`ask/opponent.rs`).
- **A scout (LT13).** Flag: a `scout_file` reply with `seen`: one box for each scout with a voice in the file (up to 3), else one for the reader of the newest look. One call: the writer speaks his own read (`voices[].says`, the file's summary, character reads, habits, medical, testing, academics) in the first person, in ranges and words. His bias is never named; it is already in his read. Words that fail the filter fall back to his own voice line, or no answer (`ask/scout.rs`, `sheet`).
- **The report (LT14).** Flag: the same file, a button ("Write the report"). The writer gets the file's sections and writes a paragraph under each heading. A heading the file never gave, a number it doesn't hold, a banned word or markup drops the whole report for today's set lines: the file's own lines under the same headings (`ask/report.rs`, `checked`).
- **The weekly leads (LT21).** Every Monday from September to January (proposed, not confirmed) the game computes the outliers (`ask/leads.rs`, `outliers`): in each position group's analytics table (the Data section's percentiles among that season's qualifiers at the level), the players whose best number is at the 90th percentile or higher, scored by the mean of their two best percentiles plus an age credit (1.5 points a year younger than 21 in college, 25 in the NFL, 16 at high school, capped at 4 either way), at most 2 a group and 8 in all, minus your squad, your "my players" list, your recruiting board and your draft board. `mail.rs` files them as a memo from your analytics head with the table as linked text and the numbers in words. That is the whole mail with no key (`Mail.leads` 1). With a key, opening it makes the mail's one call (`auto`): the writer picks up to 5 and says why; each reason passes the filter or the game's numbers stand in its place; the picks go first, the table's last column becomes "Why he stands out" and `leads` becomes 2, stored under the lock. A second ask of the same mail reloads it with no call.
- **No call while the clock runs.** The mail is filed with no call; the one call waits for you to open it (`tests/live_desk.rs`).
- **Fallback words** (for the line bank's writing): the analyst's "N found. Name is first." / "Nothing matches that search." / "Your analyst set the search. Run it below."; the opponent's "Spot: N snaps charted. They pass X% of the time (the level Y%), Z yards a play, W% success."; the scout's own voice line; the report's set lines; the leads' numbers text.
- **Sizes (proposed, not confirmed):** the reader's 500 tokens, the analyst's and opponent's line 150 tokens, a scout's answer 220, a report 800 and at most 30 lines, 8 conditions and filters, 4 squad rules, 2 spots, 8 leads, 5 picks.
- **Not built:** other seats' leads at another league's clubs (none); the browser build's stand-in for the asks (`AFM_AI_FAKE=1` answers an ask with the fake's plain reply, which reads as "off"; lane 2's demo switch shows every state).

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
- `crates/afm-game/src/live_talk/say/mod.rs`: `READER_TOKENS`, `WRITER_TOKENS`, `WAIT_TTL`, `READER_RULES`, `WRITER_RULES`, `PROMISE_KINDS`, `PROMISE_BY`, `CONDITIONS`.
- `crates/afm-game/src/live_talk/card.rs`: `MAX_OPTIONS`, `SO_FAR`.
- `crates/afm-game/src/live_talk/filter.rs`: `BANNED`, `MAX_CHARS`, `NUMBER_WORDS`.
- `crates/afm-engine/src/world/folk/finals.rs`: `FINAL`. `world/folk/scene.rs`: `LIMIT`, `NO_BOX`. `world/folk/claims/truth.rs`: `CLAIM_KINDS`, `KEEP_LIES`, `WORD_START`.

## Related pages

- [README.md](README.md)
- [saves.md](saves.md)
- [people.md](people.md)
