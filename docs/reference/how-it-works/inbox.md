# The inbox: how it works today
Checked over main 604b9cc on 2026-11-13 (the inbox lane: the portal's messages, the two-pane inbox and the college seat's other writers), before it merged.

Paths are relative to `crates/afm-game/src/lane_core/` unless they start with `crates/`, `app/` or `docs/`. "The pen" is `mail.rs`, `Pen`: one writer per stretch of days, with your name, your team and the mails it has made.

## Quick answers

- Where does a message come from? `mail::after_days` runs after each stretch of days (and after each new day in holiday mode), asks `compose` for the mails of the days since the last one, and files each once its local stamp has come (`mail.rs`, `after_days`, `compose`; `World::holiday_to_with`, `lib.rs`, the `holiday` command).
- Can the same message be written twice? No: a mail's id is a hash of its tag and its day, so the same tag on the same day is one mail; a later, longer body or table replaces the older and marks it unread (`mail.rs`, `letter`, `fnv`, `after_days`).
- How long does a message stay? 30 days (`KEEP_DAYS`, `afm_engine::world::desk`).
- What makes a row "Needs you"? An unanswered mail with an answer that does something (a call, a note, a vote) or with a day to answer by that has not passed, or a letter about a meeting whose scene is still open and either stops the clock or asks for your call (a player who asked for time, a rule broken, a second opinion) (`desk.rs`, `needs_you`, `scene_waits`).
- Which lines does a college seat's Continue card leave out? The portal's lines and the services' class ranking, because a message carries them (`mail.rs`, `card_covered`; `views.rs`, `stopped`). The morning clips leave out the portal's too.
- Does any message raise a world event? No. Every writer reads what the world already holds (the calendar's lines, the news store, the people's dated records, the training desk, the honors book).
- Is the Monday recruiting digest still one mail? Yes, with an "Around the trail" part (`mail.rs`, the digest block; `mail/trail.rs`).

## The portal's messages

See [recruiting.md](recruiting.md), "The portal in your inbox".

## College seat writers (the inbox lane, part B)

- **Around the trail** (Monday digest, `trail.rs`): what happened to the recruits on your board and every 5-star at other schools: official visits, commitments (a later flip leaves only the last one) and signings ("flipped" shows on the signing), each read from the recruit's own dated record (`PersonCal::visits`, `commit`, `signed`). At most 20 board rows and 10 other 5-stars; the rest are a count (`steps`, `part`, `BOARD_ROWS`, `STAR_ROWS`).
- **Signing days** (`signing.rs`): a notice from the personnel director when the early signing period opens and on national signing day, with how many places of your class are committed or signed to us; once the services have ranked the classes, where yours stands among the FBS classes, by the engine's own order (score, then program number), while the line is fresh (`notice`, `rank`, `class_scores`). A high school seat gets the signing day notice with its seniors' offers and decisions. The ranking line leaves the card (`covered`).
- **NIL answers** (`nil.rs`): one memo from the collective for each time a batch of answers reaches you (a recruit's promise, a transfer's offer, a roster player's deal), worst first, with the number you set and why; the card no longer carries a row each (`answers`; `front_office::college::Reaction`).
- **Trainers' memo** (`trainer.rs`): who is out, for how long, the morning after a game that hurt someone and on Wednesdays in the season (off the season only a man out 14 days or more), read from the training desk's record (`SeatTraining::hurt`); a starter out four weeks or more makes it loud. After a game it offers to check in on each of them. An NFL seat has its own injury report (`mail.rs`, `injuries`).
- **Coaching carousel** (Mondays, `carousel.rs`): the head-coaching firings, departures and hires of the past week in your division, read from the news store (`NewsKind::CoachFired`, `CoachLeft`, `CoachHired`; extensions and assistant hires are counted in a line), your own program left out. The scan goes back by date, not by count, because the carousel step writes hundreds in a day (`week`, `monday`, `ROWS`).
- **Where we stand** (Mondays, `polls.rs`): this past week's polls (the Coaches Poll, the AP, the BCS or the playoff committee's rankings) with your rank and points, your record counted from the games dated before the Monday, and the next game with the other side's rank and record (`where_we_stand`, `World::poll_weeks`, `college_fixtures_in`).
- **Selection day** (`selection.rs`): a letter from your athletic director at 1 PM Eastern, after the committee's noon step: your bowl or playoff place and game, your seed if you are one of the top four named in the day's line (a top four team without a game yet has the first-round bye), or a ban that stands (`era.ban_until` against the coming season), or a memo that there is no postseason for you. A lower division's miss cannot be told from a bye, so it sends nothing (`notice`, `is_day`, `seed_of`).
- **Honors** (`honors.rs`): the Heisman finalists, the winner, the awards show and the NFL's awards each come as their own memo, stamped a quarter hour after the line came out, with only your players' part of the line (the finalists and the show are split item by item). The All-America line names nobody, so the memo is read from the honors book (`w.stats.honors`) for your first-team men that year (`memos`, `all_america`).
- **NCAA penalties** (`compliance.rs`): a letter from your athletic director at 2 PM Eastern when the NCAA penalizes your program, with the terms read from the program's record (`era.cut`, `cut_until`, `ban_until`) against the coming season, and the show-cause coach linked (`ncaa`; `NewsKind::NcaaPenalty`). The news scan goes back by date (`mail.rs`, `news_mail`).
- **Practice rules** (`compliance.rs`): a notice from the compliance office (the league office at a club, the state association at a high school) at 1 PM Eastern when your practice week was found out, with how and what it cost, from `SeatTraining::cases` (`practice_cases`).
- **A player asks about his future** (the engine's letter, `world/folk/letters.rs`): files under Portal as well as Players and, being an open meeting that stops the clock, shows as Needs you until the meeting is held.

## Where the knobs are

- `crates/afm-game/src/lane_core/mail/carousel.rs`: `ROWS`
- `crates/afm-game/src/lane_core/mail/trail.rs`: `BOARD_ROWS`, `STAR_ROWS`
- `crates/afm-game/src/lane_core/mail/nil.rs`: `ROWS`
- `crates/afm-game/src/lane_core/mail/trainer.rs`: `ROWS`
- `crates/afm-game/src/lane_core/mail/polls.rs`: `POLLS`

## Related pages

[recruiting.md](recruiting.md), [clock.md](clock.md), [injuries.md](injuries.md), [staff-and-coaches.md](staff-and-coaches.md), [people.md](people.md)
