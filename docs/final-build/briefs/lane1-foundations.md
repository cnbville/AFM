# Lane 1: Foundations

Read first: `CLAUDE.md`, `../plan.md`, `../list.md`, `docs/game.md`, `docs/design/people.md` (Aura, fame, NIL), `docs/design/screens.md` topic 17 (L2), `docs/design/later-extras.md` (X6). Work in progress: `claude/final-wip-lane1`.

Other lanes wait on your fields, so open a first PR with items 1-4 and 6 as soon as they pass, then memory compaction as a second PR.

1. A1: a birth date for every person (players, coaches, you); a school class runs from September to August birthdays. Every age shown comes from the date.
2. A2: kids held back or reclassified (about 1 in 10, real rates), so some seniors are 19.
3. Aura (F15, F16, X6): a hidden 1-20 trait (charisma, camera presence, style), drawn at 14 with a small link to ego and leadership, growing a little with age and media training. Ads, NIL deals, endorsements and social follower growth come from Aura x fame (performance, stage, position, market), so Aura alone never makes a backup rich. It replaces any "looks" input. No NFL QB pay premium for looks or Aura. Hidden: the game crate never shows the number (a scouted word at most). Expose a clear engine function other lanes call (e.g. `aura_appeal(person, fame)`), documented in `docs/world-model.md`.
4. L2: events store the ids of the people, games and teams they name (today `calendar/events.rs` `Event` holds only a team id), so the screens can link every name in inbox, news, Home and calendar text. Give the text a way to mark spans with their ref, fill it everywhere events naming a person, game or team are written, and expose it through `views.rs`. Make the existing inbox/news/home renderers in `app/ui/app.js` turn refs into links (minimal; Lane 9 restyles the screens). Document the format at the top of the event module so other lanes write refs too.
5. Memory compaction: in-season resident memory is 5.1 GB vs the agreed 4 GB budget on an 8 GB M1 (`docs/game.md`, Memory and Doubts). Compact the season's stores (game logs, the year's records, 8-byte season records, anything that grows from day one) and get in-season memory under 4 GB. Measure with `crates/afm-tools/src/bin/memory_check.rs` before and after (a smaller world is fine for the ratio if a full one is too slow; say what you measured). Compaction must keep identical results per seed.
6. Saves: all new fields `#[serde(default)]`; `World::after_load` fills birth dates, Aura and anything else for an older world file, deterministically.

New randomness on its own streams, so other results per seed don't move (birth dates and reclassification are agreed fixes). Unit tests in `crates/afm-engine/src/tests/`. Update `docs/game.md` and `docs/world-model.md`.
