# The six-part day (player immersion I1)

Cain, 2026-10-30 00:38 UTC: "i want days split up in time, 5 AM, 8 AM, 1 PM, 4 PM , 7 PM, 10 PM. So one day turns into 6 sections". Design done 2026-11-05. Not built yet. On 2026-11-04 21:27 UTC Cain chose to build it in the next build, as its own lane, ahead of the world rebuild (thread "Add day sections to the next build").

What each section holds at each level, with sources, is in `day-sections-map.md` beside this file: real game weeks, 26 key days, hard times, time zones, and what the code does today with each event's function. Its code notes were written at `ca23d9b` (2026-10-30), so check each one on main before changing it. Most real times there are marked "unsure", because the 2026-10-30 check could open few sources. Use them as the defaults, and read the source where one opens.

How settled each part is:
- Topics 1-5: Cain's own answers.
- Topic 9: settled without a question (it was already built).
- Topics 6-8: Claude's picks. Cain's rule of 2026-11-05 08:11 UTC applies: "if there is no reason not, to include it in builds and don't mention the superficial questions". So they get built as written.

## The six sections

| | Hours (your local time) | Length |
|---|---|---|
| S1 | 5-8 AM | 3 h |
| S2 | 8 AM-1 PM | 5 h |
| S3 | 1-4 PM | 3 h |
| S4 | 4-7 PM | 3 h |
| S5 | 7-10 PM | 3 h |
| S6 | 10 PM-5 AM | 7 h (8 on the fall clock-change night, 6 in the spring) |

A time on a boundary opens the next section (4:00 PM is S4). The day runs 5 AM to 5 AM, so S6 crosses midnight. Sections are not spendable: they describe the day and spend no hours (the "own hours" budget was dropped).

## Topic 1. Continue stops at every section (Cain)

Cain, 2026-10-30 12:27 UTC: "Every slot, mainly because right now it's very annoying when i for example offer an NIL to a player, and i want to see their response, i expect an idea of their response preetty soon, now it just skips an entire day".

- Continue stops at every section, six times a day. This replaces the confirmed rule "Continue skips quiet days" (`calendar.md` topic 1 item 1 and topic 10; also `first-playable.md` and `game.md`).

## Topic 2. Where you stand when Continue stops (Cain, yes to all four, 2026-11-04 21:38 UTC)

1. Continue stops at the first minute of each section (5:00, 8:00, 1:00, 4:00, 7:00, 10:00). What you decide there happens at that hour. Continue then plays the section's hours in clock order and stops at the next section. Example: a 4 PM ET waiver claim goes in at the 1 PM stop, and the 4 PM stop opens with who you got.
2. Live moments open at their real minute inside a section, and the rest of the section plays after them. That covers your game, the draft, free agency's opening, the trade deadline, cutdown day, the undrafted rush and signing day. Today the only live hold is your draft war room (`world/scouting/live.rs`).
3. The day runs from 5 AM to 5 AM, so the 10 PM section covers the night.
4. The top bar shows the date and the section, like "Fri 30 Oct, 7 PM".

## Topic 3. Local time (Cain, "local time for both", 2026-11-04 21:39 UTC)

1. The six sections run on your seat's local time. A Seattle coach's 5 AM stop is 5 AM Pacific, which is 8 AM Eastern. Central is one hour behind Eastern. Arizona keeps standard time all year: 3 hours behind Eastern in summer, 2 in winter.
2. Every time on screen shows in your local time only, with no ET beside it. A league deadline stays the same real moment for every club: 4 PM ET is 1:00 PM for a Pacific seat. The world's events keep their Eastern moment (`calendar/date.rs` `Moment` is ET) and land in the local section they fall in. High school times are local already.
- Following from 1 and 2: for a Pacific seat, 4 PM ET falls right on the 1 PM stop. So the 8 AM stop is the last chance, and the 1 PM stop opens with the results.

## Topic 4. Everything at its real hour (Cain, yes to all three, 2026-11-04 21:40 UTC)

1. Every event, result, letter and news item lands in the section of its real hour. Results post at the final whistle, and mail arrives at its stamp. Nothing shows before its hour. This removes the 9:00 AM default (`calendar/events.rs` `Event::new`; news has a date and no time, `world/news.rs` `NewsItem`) and mail written after the day ran (`lane_core/mail.rs` `after_days`). The map's table "At a default or wrong time today" lists each one and the section it belongs in.
2. The weekly batches split into the day and section they happen in. A practice injury shows after that day's practice, not in the NFL's Tuesday batch (`pro/games.rs`, `pro/mod.rs` `day`) or the college and high school Monday plan (`training/seat.rs` `practice_injuries`, `training/week.rs` `monday`). The same goes for new court cases (Wednesday today, `folk/courts.rs`).
3. Where real life sets no hour, the hour comes from the sender's own day. A high school recruit answers after the last bell, a GM calls during office hours, and a booster calls in the evening.

## Topic 5. What a section shows when it opens (Cain, yes to all three, 2026-11-05 08:09 UTC)

1. Each stop opens with a short "since 1 PM" list: your mail, results, news, other teams' moves and deadlines that passed.
2. Under it, one line says what you're doing now, from your seat's real day: "4:00 practice, then the staff meeting." It describes the day only.
3. It lists what's due before the next stop. Example: at a Pacific seat's 8 AM stop, "Waiver claims due 1:00 PM".
- Its look gets a mock-up for Cain before the lane builds it (rule: mock-ups first for new screens), on Showtime like every screen.

## Topic 6. Who sets when your team practices (Claude's pick, under Cain's rule)

1. Other teams practice at their level's real hours. High school teams practice after the last bell. In summer and preseason they go at the hours the state's heat rules allow (Florida's Zachary Martin Act; in Georgia the first five preseason days are 2 hours in helmets only). College programs keep their own hour by custom: some practice in the morning (Purdue under Ryan Walters; South Carolina at 9:30 in spring 2026), and many go about 4 PM. Every program stays inside the NCAA's 4 hours a day, 20 a week in season, and the players' 8 free hours between 9 PM and 6 AM. NFL clubs set their own times, mostly late morning to midafternoon.
2. At your seat you set your own practice hour inside your level's rules. A high school coach sets it after the bell, a college coach inside the 20-hour week and the 8 free night hours, and an NFL coach as each club does. Today the training day's clock is a proposed fixed one (`lane_training/week.rs` `day_clock`: high school lift 6:30 and practice 3:15, college lift 6:35).

## Topic 7. Your game day across the sections (Claude's pick, under Cain's rule)

1. Your game plays in its kickoff's section, not at the morning stop (today `world/live_seat.rs`, `app/ui/screens/gameday.js`). The sections before it hold game morning in real order, as `game-day.md` topic 3 has it: meetings, the walk or arrival, warmups, inactives 90 minutes out, college's final availability report, the coin toss. The sections after it hold the press, recruits and film. A 7:30 PM college game: the walk at 5:30 in S4, kickoff in S5, the press and recruits in S6. A 1 PM ET road game for a Pacific NFL seat kicks off at 10 AM local, in S2.

## Topic 8. When meetings, talks and visits happen (Claude's pick, under Cain's rule)

1. Meetings the world sets keep their real hour. The boss meets in office hours. Takeover meetings keep the hours already in their text (the principal at 7:15 AM, the college team meeting at 7:30 PM, `folk/takeover.rs` `schedule`). Press conferences are held on each program's or club's own day and hour: South Carolina's weekly one is Tuesday at 1 PM (2024). Today the game holds every college and NFL weekly press on Monday (`folk/press.rs` `day`). Postgame press comes 10 to 15 minutes after the game (unsure).
2. The meetings you call go into the next section your real day has room for, with no budget of hours: a player after practice, parents in the evening, a recruit by phone at night.

## Topic 9. Quiet stretches (settled, no question)

- Holiday mode stays as built. You pick a week, two weeks, a month or three months. It passes the sections without stopping, your staff makes every call, and it comes back early only for a huge moment (`app/ui/screens/core.js` `holiday`). The dialog's old promise to stop "for anything only you can decide" is already gone from main.
- Continue stops at every section (topic 1). A quiet stretch like South Carolina's high school dead week (June 28 to July 4, 2026) is 42 stops. Holiday mode is the way past it.

## What the build has to do

- The world runs a section at a time, in clock order. Today `world/clock.rs` `advance_day` runs a whole day in one call, in code order, and the clock is a date with no minutes. A decision made at the 1 PM stop has to reach the 4 PM processing of that same day. So the day can't be worked out whole at 5 AM and only shown later.
- Continue (`continue_until_needed`, `world/seat.rs` `World::morning_stop`) stops at the next section boundary, or at a live moment's minute inside the section.
- The seat has a time zone. Its sections and every time shown are local. The world's moments stay Eastern.
- Results, mail, news, transactions, draft picks and draft-day trades get their real minute and show in their section, never before it. The draft's three days run on their own days.
- The NFL's Tuesday batch, the Monday training plan's injuries and Wednesday's court cases split into the days they happen.
- Each stop gets its opening card (topic 5), after Cain's mock-up yes.
- Your practice hour, your game's sections, the world's meeting hours and the meetings you call (topics 6-8).
- The quick NIL reactions (lane NB9, built on today's daily clock) land in the next section, not the next day.
- Speed: a quiet day's six sections together stay within today's quiet-day time on the M1 (`first-playable.md`: under half a second).
- World results change, because batches move to their days. That's agreed (topic 4), so this lane lands before the world rebuild (NB13), which builds on new seeds.
- Docs: `calendar.md` (topic 1 item 1, topic 10), `first-playable.md` and `game.md` get the new Continue rule. There is no how-it-works page for the clock yet, so the lane adds one (`docs/reference/how-it-works/clock.md`), with its commit line.

## What the build must prove

- Continue stops six times on a quiet day, at 5:00, 8:00, 1:00, 4:00, 7:00 and 10:00 local, at all three levels and in a Pacific and an Arizona seat.
- A waiver claim made at the 1 PM stop (Eastern seat) or the 8 AM stop (Pacific seat) is processed at 4 PM ET that day, and the next stop shows the result.
- Nothing posts before its hour. A Saturday 3:30 PM college game's result first shows at the 7 PM stop. No draft pick shows before its minute.
- No event keeps the 9:00 AM default unless 9:00 is its real hour.
- Your 7:30 PM game plays in S5, with game morning in the sections before it.
- Stopping at every section with no choices made gives the same world, seed for seed, as running the same days straight through.
