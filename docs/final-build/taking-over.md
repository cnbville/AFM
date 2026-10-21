# Taking over a job: your first two weeks

Thread "Career and world improvements". Cain's idea (2026-10-21 19:22): "When you start a job, there should be like a team walk through, analysing key players, the behaviour of the AD, biggest boosters and what they need to keep happy. It should feel like entering a new job and not just starting a spreadsheet game like it does now."
Items 1-23 and the bug posted 19:40; **Cain agreed to all on 2026-10-21 20:48 ("agree on all").**

**Now:** `start_career` (`crates/afm-engine/src/world/seat.rs`) sets the seat and you land on Home on 9 February with an empty inbox. Nobody meets you, and nothing tells you who matters.
**In real life** a new head coach spends his first two weeks in meetings. Each one teaches him something and leaves a first impression.

## Already decided, so it comes with this (not new)
- Your boss's first goals meeting, and keeping or letting go the old staff (`docs/design/first-playable.md` topic 2).
- Bosses and big boosters as people, each with trust in you (`docs/design/people.md` topic 12).
- Talks: team meetings, one-on-ones, parents (people topic 5). Lines from the big line bank, never stale (topic 5).
- Press conferences and reporters as people (people topic 9).
- The hire reveal (`docs/design/screens.md` topic 9).
- Your read of your own players sharpens with what your staff has seen (`docs/design/scouting.md` topic 1, Q31).
- Any task can be handed to your staff (`docs/design/calendar.md` topic 10).

## What is built that this reads
- The athletic director: name, judgement, patience, since (`world/program.rs`, `AthleticDirector`; reviews in `world/staff.rs`, `college_review`).
- A big booster only as an amount of money that can walk away (`world/program.rs`, `Collective.big_booster`). No booster is a person yet.
- The NFL owner's traits: meddling, a pet player, guaranteed cash, patience, the pull to make a splash, overriding the draft room, trust in his GM (`world/pro/office.rs`).
- Captains and the room, players' personality and ties (people topics 1-4, build 9).
- Portal leans with dated reasons, "his head coach is gone" (`world/college.rs`, `LeanCheck::CoachingChanges`); AI head coaches meet the players they want to keep before the winter window, swayed by their relationships rating (`world/recruiting.rs`, around "A head coach meets the players he wants to keep").
- Commits lean away after a coaching change (`world/recruiting.rs`, `new_coach`).
- Academics, off-field trouble, injuries, facilities, the collective and revenue share, program seasons and coaches' jobs.

## The agreed items

### How it plays
1. **The first two weeks are dated meetings.** Continue stops on each one. Each is a scene: the person, where you are, what he says, and your answers picked as moves (people topic 5). You can hand any meeting to your staff (calendar topic 10).
2. **A takeover file you can reopen.** It keeps everything the meetings told you on one page, with who said it and when.
3. **Every briefing has an angle.** A holdover position coach oversells his own players to keep his job, and an AD plays down the budget. Each line names its source and leans by that person's interests, so you learn whom to trust.
4. **First impressions set trust.** Each person's trust starts from your reputation against the job and from his ties to the old coach. These meetings are its first move (people topics 2, 3 and 12).

### The players
5. **The state of the team.** In real life a new staff grades the whole roster on film in its first weeks; in the NFL it does this with the GM's pro scouts, before the combine. You get the key players at every position as your staff reads them, plus the holes after the seniors leave. The reads are wide on day one (film and the holdovers only) and sharper after spring ball (scouting topic 1).
6. **The leaders and how they took the change.** The captains and the room's voices, each with his first reaction to the change, in words.
7. **Who might leave.** Since 2025 a head coaching change gives players 15 days in the portal, starting five days after the hire (calendar topic 3). Caleb Williams followed Lincoln Riley from Oklahoma to USC in 2022. Your staff lists the flight risks: close to the old coach, unhappy with his role, or courted by others. Needs the bug below fixed.
8. **One-on-ones before the window.** You hold the talks the AI head coach holds today, yourself, in week one, with the players you pick. What you say, such as a role promise or an honest depth chart, moves each player's lean.
9. **Commits who wobble.** The recruiting coordinator's list of next year's commits and top targets, who's wavering and why, and your first calls.
10. **The academic and conduct report.** Who's at risk of being ineligible and the conduct cases still open, from the built academics and trouble.
11. **The trainer's report.** Who's hurt, who's rehabbing, when each is back, and which surgeries are still to come.

### Your bosses
12. **The boss's file: how he treats coaches.** Coaches ask around about an AD before taking a job. Read his record from the world's own history: whom he hired and fired, after how many seasons and on what record, and who has his ear. All in words, never numbers. In the NFL: the owner, with his built traits, and the GM. At a high school: the principal, the AD and the superintendent.

### The boosters
13. **Meet the top boosters in week one.** In real life the AD takes a new coach to meet the biggest donors and the collective's leaders. Each is a named person with what he has given (a building, the collective, a buyout), his business, and his ties to the AD, the president and the old coach. Counts: 3-5 named at a Power 4 program, 1-3 lower down; at a high school the booster club president plus one or two local business owners. The NFL has none; the owner's family and partners fill that role.
14. **What each booster wants.** One or two wants each, drawn from the real kinds: beating the rival, access (the sideline, the team plane, the coaches' box), a say in hires or on a player, NIL deals for his company, a clean program, his name on a building, his old high school's players recruited. Ignore them and he pulls his money (the built walk-away) and speaks against you when the AD reviews you. Real: Tennessee's deal with Greg Schiano collapsed in a day in 2017 after fans, boosters and politicians objected.
15. **The old coach's loyalists.** Boosters, staff and players tied to the man you replaced start colder and judge you against him. Real: Penn State's Paterno loyalists fought the trustees for years after 2011. At a high school the old coach often still teaches down the hall (his stipend not renewed, his teaching job kept; people topic 14).

### The room and the public
16. **Your first team meeting sets the tone.** Real: on his first day at Colorado in December 2022, Deion Sanders told the players to find new schools, and dozens left. Your opening speech is a team talk (topic 5); each player takes it by his personality and his ties to the old coach.
17. **The introductory press conference.** Real: Charlie Weis promised Notre Dame "a decided schematic advantage" in December 2004, and it followed him out. Your answers set what fans and the media expect (topic 9). At a high school, the local paper.
18. **How the world took your hire.** Hire grades and how fans and recruits reacted, in words, on day one, from your reputation against the job (people topic 15).
19. **The beat reporters.** The communications director tells you who covers the team, who's fair and who leaks: topic 9's reporters, introduced with their outlet, leaning and sources.

### The place
20. **The program tour.** The last five seasons and why your predecessor left, the rival and your record against him, facilities in words, the money (budget, collective, revenue share) and the schedule already set for years ahead.
21. **The first-weeks plan.** Your operations director (college), GM (NFL) or AD (high school) lists what's due and when: the portal window closing, spring ball, the combine, free agency and the tag deadline, the state's spring practice.

### High school only
22. **The parents' meeting** in your first weeks (topic 5's parents' talk).
23. **The feeder tour:** the middle school and youth league coaches, and who's coming up (scouting topic 17's feeder look-ahead).

## Bug (real-life rule, agreed with the list)
Your own hire opens no coaching-change portal window, and your players get no "his head coach is gone" lean. Both only fire at the late-November carousel: `world/clock.rs` sets `coach_window` only on the Sunday after rivalry week, and `college::lean_checkpoint(LeanCheck::CoachingChanges)` runs only there. `staff::take_seat` calls `leave(old, "fired")`, which sets `hc_changed = cycle_year(w)` (the previous year in February) and nothing else. Real rule (calendar topic 3): 15 days, opening five days after the new coach is hired. Fix: your hire (and any off-cycle head coach change) opens the window and pushes the lean for that program's players.

## Builder's notes (proposed, not confirmed)
- A real order of the first days, by level (college): day 1 the team meeting in the evening; day 2 the introductory press conference, then the AD; days 2-5 the holdover staff one by one and the staff's state-of-the-team report; week 1 the one-on-ones, the trainer and the academic advisor, the recruiting coordinator's calls; week 1-2 the boosters and the collective; week 2 the communications director and the first-weeks plan. NFL: the owner and GM, the press conference, the coordinators, the roster review with the pro scouts before the combine, the cap manager. High school: the principal and AD, the school board's welcome, the athletics period with the players, the booster club, the parents' meeting, the feeder tour.
- The same arrival runs for every later job in a career, not only the first.
