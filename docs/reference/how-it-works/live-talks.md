# Live talks: how it works today

Checked against main at 1f2b001 on 2026-11-13 (lane 1, the pipe, before it merged to main). Lanes 2 to 10 are not built yet: their sections below are empty until each lane fills its own.

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
- Is a lie judged? Not yet: the lies store and its daily check exist, and judging costs nothing until the promise design is decided (`world/folk/claims/truth.rs`, `judge`).

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

Not built yet.

## Lane 3, the core

Built 2026-11-13. A typed line lands on one of a scene's own options through the same code as the chip, so the line bank still picks its line with the same seeds and the world after is the chip's. Your words go on your line for that pick, never on a second line.

### A typed line, step by step

- `people_say {id, text}` checks the meeting (open, not over, a kind that takes a typed line, at most 400 characters), builds the reader's card from the scene's own options and returns a ticket. No call runs in the command (`crates/afm-game/src/live_talk/say/mod.rs`, `people_say`).
- The reader (label `scene_reader`, Haiku 5.5, structured output, at most 400 tokens) answers with an option id (o1 to o12, or none), a close second when unsure, a tone, a promise as read, the facts stated, the people named and its fit: clear, unsure or not a line. The rules and the schema are fixed for every scene, so the cache holds (`say/mod.rs`, `READER_RULES`, `reader_schema`).
- Its answer is applied inside `ai_poll`, under the game lock. Not a line, or no option: `{"state": "none", "note": "He doesn't follow."}` and nothing changes. A chip clicked while the line was read: `off`, "The talk moved on before that landed." Unsure, or a pick that can't be undone: `{"state": "unsure", "readings": [{"pick", "heard", "final"}]}`, and the reading waits (`say/mod.rs`, `reader_step`, `Waiting`).
- Otherwise it lands: `folk::scene::say` runs the scene's own answer with `Scene.heard` set for that one answer, then the same ticket carries the writer's call (label `scene_writer`, at most 200 tokens) for the line the answer wrote for him (`say/mod.rs`, `land`, `go_on`).
- The writer's words take the bank line's place only if they pass the filter and nothing moved meanwhile; the scene keeps what was shown. If the writer fails or is dropped, the bank's line stays (`say/mod.rs`, `writer_step`).
- The poll answers `{"state": "done", "heard": "ask about his scheme and how he'd use your players, calm", "marks": [...], "reply": <the people_scene reply>}`. `live_talk::after` puts the box on the nested reply too (`say/mod.rs`, `done`; `live_talk/mod.rs`, `after`).
- Confirming: the box sends `{id, ticket, pick}` with one of the readings; the answer is a new ticket. A reading waits 10 minutes and is taken once (`say/mod.rs`, `confirm`, `WAIT_TTL`).

### In the engine

- `scene::say(w, id, heard, confirmed)` lands a typed line on one of `scene::options` (the picks and `reach`, never a nav pick) through `land`, the same function `answer` now calls. It refuses a meeting that is over, a kind with no box, an empty line, one over 400 characters, an option not on the table, and a pick that can't be undone unless `confirmed`, with "That can't be undone, so say which you meant first." (`world/folk/scene.rs`, `say`, `land`, `ASK_FIRST`).
- `Heard` (the pick, tone, words, length, facts as stated, a promise as read, a game plan ask and the people named) rides on `Scene.heard` for that one answer and is cleared before and after. It is never saved (`scene.rs`, `Heard`).
- `Said.typed` holds your words on your line for the pick. An answer that ends the meeting with no line of yours (an interview's "Thank him and wrap it up") gets one with the chip's words; a meeting that goes on gets no extra line, since scenes count your lines (`scene.rs`, `put_typed`).
- No box: press conferences, media, story meetings, and one-on-ones until lane 9 fills its hook (`scene.rs`, `NO_BOX`, `takes_typed`; `world/folk/talks.rs`, `ONE_ON_ONE_TYPED`, `one_on_one_say`).
- The picks that ask first: a pick marked `PickKind::Final`, or one of 28 rows by scene kind and code: a December signee's release (70-72), a holdover's job (22, 33, 34), letting a coach go (70-72), every discipline ruling above a warning (501-508), an all-star invitation or a search firm's call (95-98), hiring a high school coach's package (87), and the deals outside the rules (151-153, 181, 191, 192). Codes repeat across kinds, so the kind always decides (`world/folk/finals.rs`, `FINAL`, `is_final`).

### Facts and lies (seams for the later lanes)

- The reader may label facts of 13 kinds: depth_rank, ahead_of_him, role, snaps_share, touches, nil_amount, money, injury, status, stat, off_field, interest, record (`world/folk/claims/truth.rs`, `CLAIM_KINDS`). Each is marked by `claims::check::fact`, which sends a recruit's scene to `check/recruit.rs`, a GM's or boss's to `check/money.rs` and the rest to `check/roster.rs`. All three answer "unknown" today, which shows no mark (`claims/check/mod.rs`, `fact`).
- `Folk.live` keeps the lies (who, what, the day he can see it) and each agent's and GM's trust in your word (0.5 to start). It is saved only when not empty (`claims/truth.rs`, `Live`, `KEEP_LIES`, `WORD_START`).
- The daily check marks a lie seen on its day and then judges it. Judging costs nothing yet: Cain rejected the first lie rule (2026-11-13), and the promise design decides it (`claims/truth.rs`, `check_lies`, `judge`).
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

- Engine (`crates/afm-engine/src/tests/live_core.rs`): a typed question is the clicked one (same world bytes, same count of your lines); the same reading on the same save gives the same world; a line that ends the interview shows your words; nothing off the table or nav is landed; a final pick asks first and no caller can skip it; no box on press, media, stories and one-on-ones; `heard` never kept; the lies store waits; a promise costs what it cost before the move.
- Game (`live_talk/say/tests.rs`, `live_talk/filter.rs`): LT18 end to end on the fake; typed equals clicked through the commands; not a line changes nothing; unsure and the confirm; a final pick asks first; every failure kind falls back mid-scene; no key, no box; the reads match the interview; the filter on 200 replies; no hidden number reaches 1,000 cards.
- Full tests (`crates/afm-game/tests/live_core.rs`): the same save and reading replay the same, typed equals clicked from a save, and a save made with live talks plays without a key; every reply walked for hidden keys (`tests/common/mod.rs`, `nothing_hidden`, shared by later lanes).

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
- `crates/afm-game/src/live_talk/say/mod.rs`: `READER_TOKENS`, `WRITER_TOKENS`, `WAIT_TTL`, `READER_RULES`, `WRITER_RULES`, `PROMISE_KINDS`, `PROMISE_BY`, `CONDITIONS`.
- `crates/afm-game/src/live_talk/card.rs`: `MAX_OPTIONS`, `SO_FAR`.
- `crates/afm-game/src/live_talk/filter.rs`: `BANNED`, `MAX_CHARS`, `NUMBER_WORDS`.
- `crates/afm-engine/src/world/folk/finals.rs`: `FINAL`. `world/folk/scene.rs`: `LIMIT`, `NO_BOX`. `world/folk/claims/truth.rs`: `CLAIM_KINDS`, `KEEP_LIES`, `WORD_START`.

## Related pages

- [README.md](README.md)
- [saves.md](saves.md)
- [people.md](people.md)
