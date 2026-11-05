# The clock: how it works today

Checked against the six-part day lane (player immersion I1) on 2026-11-05, before it merged to main.

Paths are relative to `crates/afm-engine/src/` unless they start with `crates/` or `app/`. The spec is `docs/design/day-sections.md`; what each section holds at each level is `docs/design/day-sections-map.md`. Hours the code sets that Cain hasn't answered are marked in code as proposed, not confirmed.

## Quick answers

- How is a day played? As steps in clock order: 23 fixed hours plus every game's final whistle, each draft night's opening and each practice's end (`world/clock.rs`, `play_through`, `next_step`, `timed_steps`, `hour::FIXED`).
- When does a world day start and end? At 5:00 AM Eastern; it runs to 4:59 AM the next morning. A "day minute" counts minutes after 5:00 AM Eastern, 0 to 1439 (`calendar/zone.rs`, `DAY`, `et`, `moment`, `world_day`).
- Where does Continue stop? At your next section's first minute (5 AM, 8 AM, 1 PM, 4 PM, 7 PM, 10 PM on your local clock), or at a live moment inside a section (`world/sections.rs`, `next_stop`; `world/clock.rs`, `continue_until_needed`).
- What is a live moment? Your kickoff and final whistle, each draft night's start, the negotiating window, the new league year, the trade deadline, cutdown day, signing day's 7 AM local and your meetings at their hours (`world/sections.rs`, `live_moments`).
- Does a decision at a stop reach the hours after it? Yes. Standing at a stop at minute m means every step up to m has run and none after it, so a claim made at the 1 PM stop is in when the 4 PM claims run (`world/clock.rs`, `play_through`; test `a_stop_comes_before_the_hours_after_it`).
- Does stopping change the world? No. Stopping at every section with no choices plays the same steps as a straight day (test `sections_play_the_same_world`).
- What time zone is my seat in? Your team's town's: by state, with split states by town and longitude; Arizona keeps standard time all year; daylight saving by the US rules of each era (`calendar/zone.rs`, `Zone::of_place`, `daylight`; `world/sections.rs`, `team_zone`, `seat_zone`).
- Are the world's times Eastern? Yes. Every moment is stored Eastern; screens turn it into your local time (`world/sections.rs`, `local`, `clock_of`, `when_text`).
- When does a result show? At its final whistle: college kickoff plus 3 hours 15, NFL plus 3:05, high school 7 PM local plus 2:30 (`world/clock.rs`, `hour::COLLEGE_GAME`, `NFL_GAME`, `HS_GAME`; `world/college.rs`, `final_whistle`; `world/nfl_year.rs`, `nfl_final`; `world/hs_year.rs`, `state_final`).
- What does a line with no hour get? The moment of the step that posts it, or "all day" when it is posted for another day; the old 9:00 AM default is gone (`calendar/events.rs`, `Event::new`, `Calendar::post`).
- What hides a line until its hour? Screens show a line once its moment has come; a scheduled one (a date, a window, a deadline, your day's slots and practice) shows ahead (`world/sections.rs`, `has_come`; `crates/afm-game/src/views.rs`, `shown`).
- How does holiday mode work? It passes the sections without stopping and comes back at the first section of the day you picked, or at the section after a huge moment (`world/clock.rs`, `holiday_to`).
- What does the top bar say? Your local date and section, "Fri 30 Oct, 7 PM" (`world/sections.rs`, `clock_text`; `crates/afm-game/src/views.rs`, `status`).
- What does a stop show? A card under the top bar: what came since the last stop, what you're doing now and what's due before the next stop (`crates/afm-game/src/views.rs`, `stopped`; `app/ui/screens/daystop.js`).

## The steps of a day

`hour` in `world/clock.rs` names the fixed hours (Eastern): 5:00 dawn (the day's bookkeeping, all-day lines), 6:00 paper, 7:00 plan (Monday's training week), 8:00 early (Black Monday), 10:00 office, 11:00 wire, noon, 1:00 PM afternoon, 2:00 polls, 3:00 later, 4:00 deadline (claims, cutdown, tags, the league year), 5:00 market, 6:00 awards, 6:30 practice (bowl practices), 7:30 CFP rankings, 8:00 evening (recruiting, schedule release), 8:45 Heisman, 9:00 show, 10:30 night (portal decisions), 11:30 late night, 1:00 AM overnight (the college season's end, arrests, court cases), 4:00 AM late (stories, legacy, scouting, plans) and 4:30 AM heal.

`run_step` runs one step: the games whose final whistle is now first (`college::play_at`, `nfl_year::games_at`, `hs_year::games_at`), then the draft night and the practices ending now, then the hour's own work, then the NFL's (`nfl_year::step`, `pro::step`) and the people's (`folk::step`). News written in the step gets the step's moment (`NewsItem.at`).

`advance_day` is the rest of today's steps, then the next morning (`end_day`). `run_to(d, m)` plays whole days to `d`, then `d` through minute `m`.

`World.played` is how far today has run: 0 before the first step, m + 1 when standing at minute m.

## The weekly batches, by day

- NFL practice injuries: each club practices on its own days (camp: every day but a preseason game's and the day after; season: the three days before a game that aren't the day after one) and its injuries come when its practice ends, a sixth of the week's risk a camp day and a third a season day (`training/days.rs`, `nfl_practice`, `practice_at`; `pro/games.rs`, `practice_injuries`).
- College and high school practice injuries: rolled with Monday's week, they wait for their practice day's end (`training/seat.rs`, `practice_injuries`, `SeatTraining.pending`; `training/week.rs`, `monday`, `others_pending`; `training/days.rs`, `practice_at`).
- The NFL's Tuesday: holdouts, heal week and the in-season trouble countdown stay weekly; arrests are daily overnight, rulings daily in office hours (`pro/mod.rs`, `step`; `people.rs`, `pro_arrests_day`, `pro_rulings_day`).
- Court cases open overnight every day, not on Wednesday (`folk/courts.rs`, `opens`).

## Practice hours

A team's practice starts at its staff's hour on its own clock: NFL 11:15 AM in season and 9:15 AM in camp, 2 hours 15 on the field; college 3:45 PM, or 8:45 AM for one program in five by custom, 2:45; high school 3:30 PM, or 7:30 AM before August 15, 3 hours (`training/days.rs`, `staff_start`, `practice_length`, `morning_program`, `summer`). At your seat you set the start yourself inside your level's rules (high school after a 3 PM bell in the school year, college 6 AM to a 9 PM finish, the NFL 8 AM to 3 PM) on the training week (`training/days.rs`, `practice_start`, `start_limits`; `crates/afm-game/src/lane_training.rs`, `practice_hour`).

## The draft's three days

Round 1 is the first night, rounds 2 and 3 the second, 4 to 7 the last day (`world/recruiting.rs`, `night_of`). Each night opens at its minute and makes its own picks; they join their clubs that night and each comes out at its own minute (`draft_night`, `commit_picks`, `pick_stamps`). The draft between nights is kept on the world (`World.draft_run`). The undrafted rush follows the last pick (`draft_end`). Your war room makes a night's picks with you and then waits for the next night while the clock runs on (`world/scouting/live.rs`, `holds`, `open_night`, `LiveDraft.waiting`).

## Your game day

Your kickoff and your final whistle are live moments; the sections between them pass while you coach (`world/sections.rs`, `next_stop`, `seat_game_length`). The sections before kickoff are game morning; the game day screen kicks off only at the kickoff stop (`app/ui/screens/gameday.js`; `crates/afm-game/src/lane_gameday.rs`, `now`, `at_kick`).

## Meetings and answers

A meeting keeps the hour its place line names ("the team room, 7:30 p.m."); otherwise the boss meets at 10 AM, parents and boosters at 7 PM, recruits by phone at 8 PM, a player or staff talk 15 minutes after practice, the weekly press at the program's own day (Monday, Tuesday or Wednesday) and hour (11 AM, noon or 1 PM), the postgame podium 15 minutes after the final whistle (`world/folk/scene.rs`, `add`, `hour_in`, `default_hour`; `world/folk/press.rs`, `press_day`, `press_hour`). A player's first answer to your NIL money comes at your next section (`world/front_office/college.rs`, `remember`, `Reaction.due`). Mail is written through the stop and filed once its stamp, on your local clock, has come (`crates/afm-game/src/lane_core/mail.rs`, `after_days`).

## Where the knobs are

- `world/clock.rs`, `hour`: the fixed hours and the game lengths.
- `calendar/zone.rs`, `SECTION_HOURS`: the six sections.
- `training/days.rs`: practice starts, lengths, the morning share, the hours you may set.
- `world/folk/scene.rs`, `default_hour`; `world/folk/press.rs`, `press_day`, `press_hour`: meeting hours.
- `world/recruiting.rs`, `night_of`, `pick_stamps`: the draft's nights and minutes.

## Related pages

- [training.md](training.md): the practice week.
- [injuries.md](injuries.md): practice and game injuries.
- [draft.md](draft.md): the draft and the war room.
- [nfl.md](nfl.md): the NFL's year.
