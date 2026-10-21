# Final build list (collected 2026-10-21 17:30 UTC)

Every agreed fix from the threads, in one place, for tonight's final build. Collected by reading the threads and files only; nothing was built here.
Sources: "First play design notes" (`afm/first-play/notes.md`), "Game bug check" (`afm/play-bugs/bugs.md`), "Career and world improvements", "Section 8 build: first playable game" (`docs/game.md`, PR #82), "World for first saves" (`docs/trials/world-trial10.md`), "Bird's-eye game view" (`afm/game-view/game-view.md`), "Section 9 design", "GitHub world download and app updates", the doc "AFM first build: what's not there yet" (https://claude.ai/artifact/9Z9kNGBk3EkhbAF1CQ9tbq).

## 0. Build rules (Cain 2026-10-21 21:36, "Final build brainstorm" thread)
- B7, the rule over everything: the game must be fully polished from tomorrow, with every designed feature in and working. Cain: "now every time you give me the execuse (oh this was meant to be build later) i don't want that ... i need to be able to mention those bugs without you saying, oh yeah we planned to do this later." Major bugs that are easy to fix get fixed at once. Small bugs may remain, but when Cain reports one, the answer is a fix, never "planned for later".
- B1. Build in about five lanes, each merged to main when green, so one failure never blocks the others.
- B2. Lane order: foundations first (new save fields with serde defaults, birth dates, face and appearance recipe, Aura, events storing ids for L2, memory compaction); then the engine (G1 pausing between snaps, officials, D1-D3, the links, HS tuning); then the world (X1 finances, X7 other leagues, takeover T1-T23 and T-B, conference moves, levelers); then the screens; the gated visuals last.
- B3. Memory gate: compaction goes first, and no lane merges if in-season `memory_check` grows (5.1 GB now, 4 GB budget).
- B4. A fresh world after the build: the world library is built on GitHub in parallel, as agreed, with the batch fired once the final build is merged (worlds built earlier would lack the new fields). Confirmed by Cain 21:37.
- B5. The game drawing heads (F14) and the 3D view (X2) go last, behind a switch, and wait for Cain's ok on the picture or the clip.
- B6. After every lane, one scripted full season is clicked through on each seat (HS head coach, college, NFL) with save and load; the multi-agent sweep (A17) fixes whatever breaks. Each season clicks every screen, plays through game day, and saves and reloads. Confirmed by Cain 21:37 ("agreed on both").

## 1. Agreed, not built yet

### Inbox (notes item 10, confirmed 17:08; IN TONIGHT'S BUILD, Cain 20:54 item 16 agreed)
Mock v6: https://claude.ai/artifact/GVwr6ozYJjK7LMT7EotUC8. Code today: `app/ui/app.js` `screens.inbox`, `itemRow`; `crates/afm-game/src/views.rs` `inbox`.
- 10a. Official look: each mail is a document with a letterhead (university, conference office, NFL club, agency, newspaper masthead, a player's plain email), From, To, Date, Subject, full text, tables, attachments, signature.
- 10b. Unread mail bold with a small dot, regular once opened; unread count on the sidebar's Inbox (screens.md topic 8).
- 10c. Opening a mail shows a full-screen 3D letter rising from a paper stack over a blurred inbox; answers and actions beside it; Newer and Older without closing.
- 10d. Mail kept 30 days: it clears the morning it turns 31 days old; "Clears in N d" for the last ten days; saved mails stay until deleted.
- 10e. Filter on top: All, Read, Unread. Below it one "Keyword" dropdown (Your job, Your players, Recruiting, Portal, Injuries, Money, Spring practice, Draft and NFL, Staff, Your people, Media, League) and a search box top right that searches subject, sender and the whole letter.
- 10f. Saved mails: a star on the row (shown on hover or when saved) and in the letter; a quiet Saved mails button shows only those.
- 10g. Notes button: all your notes in one panel, including notes not about one player; each note logs the page it was written on ("Inbox · subject", "Scout report · player", "Player page · player", "Depth chart"); "Add a note" in an open letter.
- 10h1. Inbox keeps FBS conference moves and the major league news.
- 10h2. Your scouts' lower-division standouts as portal watch lists (FCS, D2 and below) ("don't miss a Trinidad Chambliss").
- 10h3. News about people you're linked to: former players drafted or at the combine, former assistants hired (NFL or college), your old coaches; a weekly "Your people" memo from your chief of staff with picked answers (congratulate, call).
- 10h4. Keeping in touch and asking favours: a linked person offers or is asked for help (intros on a recruiting or scouting trip, an NFL club's read on your prospects). Needs engine work on the network (people.md topics 2 and 15).
- 10h5. Other league news (hires at other schools) goes to News, not the inbox; news about your team and rivals also comes as morning clips from your communications staff.
- 10i. Answers are picked moves (people.md topic 5): two or three answers per mail with what each did, and "Answer by <date>" in the row until answered.
- List look (16:55): one line per mail (sender, subject, grey preview, answer-by date only when due, time), plain day headings.

### Conference moves (Game bug check, bug 1; Cain "1. yes", "2. No i'm fine with FBS conference changes, just not FCS, lower divisions or HS unless i'm coaching those")
- 1a. Post each move on 1 July, the day it takes effect, worded "officially leaves the Mountain West for the Pac-12 today". Schedule membership still switches at the start of the football year. Old world files: re-date the 9 Feb lines on load (`world/program.rs` `apply_history`, `World::after_load` in `world/seat.rs`).
- 1b. Inbox: FBS moves for every college coach; FCS and lower moves only to a coach at that level; HS moves only at HS (`views.rs` `inbox`).

### Home (notes item 9; designed in `docs/design/screens.md`, Home, agreed with section 10 on 2026-10-20)
- 9a. The hero turns through slides about 8 s each: this week (record, standing, next game, countdown), a short opponent report, the last game with its deciding plays, player of the week, recruiting story of the week; subject's colours, dots and arrows, holds under the pointer.
- 9b. Tracked players replace the inbox block: latest result with the stat line, who on your staff saw it, what comes next.
- 9c. Storylines as posters with a drawn scene per kind of story.
- 9d. The next seven days, your boss's latest word, the team in words (form, injuries, mood).
- 9e. Around football filtered to news that concerns you (today it is mostly small-college position coach hires).

### Setup leftovers (first-playable.md topic 2, agreed; not in PR #82)
- Start without a job, and assistant jobs.
- "People you know" on the You step.
- A fictional-names world.
- Your family and looks read by the world after setup (kept on your profile today, nothing reads them).

### Section 9, game day control (designed and merged 2026-10-21, PR #83, `docs/design/game-day.md`; nothing built; IN TONIGHT'S BUILD, Cain 20:41 "yes")
- G1. The engine pauses between snaps for your call (today `play_game` runs a whole game in one loop); Continue stops at kickoff.
- G2. Officials make real mistakes, so replay challenges matter.
- G3. A real coin toss choice.
- G4. The tape's hints.
- G5. The realism and learning settings.
- The rest of the 12 topics (opponent report, game plan, game morning, between plays, in-game decisions, injuries, sideline, halftime, delegated games, after the game, your play-calling rating) as written in game-day.md.

### Taking over a job: your first two weeks (Cain 2026-10-21 19:22 idea, 20:48 "agree on all"; "Career and world improvements" thread; full items, decided parts and code pointers in `afm/improvements/taking-over.md`)
- T1. The first two weeks run as dated meetings where Continue stops, each a scene with picked answers.
- T2. A takeover file.
- T3. Every briefing has an angle.
- T4. First impressions set trust.
- T5. The staff's state-of-the-team report.
- T6. Leaders' reactions.
- T7. Flight risks (needs bug T-B below).
- T8. Your one-on-ones before the window.
- T9. Commits who wobble.
- T10. The academic and conduct report.
- T11. The trainer's report.
- T12. The boss's file: his track record with coaches from world history.
- T13. Named top boosters met in week one.
- T14. Each booster's wants.
- T15. The old coach's loyalists.
- T16. The first team meeting.
- T17. The introductory press conference.
- T18. How the world took your hire.
- T19. The beat reporters.
- T20. The program tour.
- T21. The first-weeks plan.
- T22. The high school parents' meeting.
- T23. The high school feeder tour.
- T-B (real-life bug): your own hire opens no coaching-change portal window and no "his head coach is gone" lean; both fire only at the late-November carousel (`world/clock.rs` `coach_window`, `college::lean_checkpoint` `CoachingChanges`). Real rule: a 15-day window starting five days after the new hire.

### Scope (Cain 2026-10-21 20:57): "i want the next build to be final (afterwards we'll tune and fix but i want all features applied as we designed)". The final build applies every designed feature; tuning and fixing come after.

### Dynamic Links: genetic gifts (designed, `docs/design/dynamic-links.md`, PR #85; IN THE FINAL BUILD)
- D1. Physics first: the contested catch counts both men's size (reach = height + arm + vertical at the catch point; today `game/scrimmage.rs` `fight` ignores size and the defender's reach skips arms).
- D2. Physics first: the QB sees a size mismatch, so a covered big receiver can be "open" for a jump ball or back shoulder (today `window()` reads separation only).
- D3. Physics first: staffs value uncoachable traits (today `positions.rs` `overall_for` is a linear sum and size counts only as a scout bias; recruiting stars are 0.25 film + 0.75 projection).
- D4. Then the links themselves as written in dynamic-links.md: several smaller stackable body links (Cain 20:40), a real, modest edge in their own situations, never a big bonus, no new limits.

### Section 11, later extras (designed, `docs/design/later-extras.md`, PR #86; copy `afm/section11/later-extras.md`; IN THE FINAL BUILD)
- X1. Full club finances at every level (topic 1): NFL, college and HS books on real lines, budgets set from the books, an attendance market, the money side of A17 renewals, the blackout rule 1973-2014, facility projects, stadiums and relocation, club values, realignment by media money, guarantee games, and what each seat sees.
- X2. A 3D game view, the coaches' view (mock-up B, https://claude.ai/artifact/CteVV5nFKW8TmDigAqe3hR; Cain 21:29 "i love B & D"): the engine adds facing, action and ball height to its frames as output only, so every seed gives the same results. The broadcast camera stays one of the 3D cameras. One switch between 3D and the 2D dots (a button or V, even mid-play); the screen opens in whichever was used last. Ships only after Cain okays a clip.
- X3. The 2D view (mock-up D): dots by default; top-down player figures as a setting.
- X4. Photo folder import (topic 4): matching by id, name plus team or year, or name alone; a review screen; undo for the whole batch; "Export AI prompts".
- X5. Build 9's "still later" items from their existing designs: prospects' pre-draft choices, smokescreens, mocks, the supplemental draft (through 2019), QB-receiver and secondary chemistry, social media with NIL from followers.
- X6. Aura in place of looks for follower growth (with F15).
- X7. The other leagues on the engine with their real rule books (CFL, Arena, NFL Europe, XFL, AAF, USFL, UFL) as full leagues, a Canadian national pool for the CFL ratio, and their coaching jobs in the carousel.
- No NewGAN-style photo pool for now (Cain 21:08).
- X8. Four film-room tools in the coaches' view: drawing on a paused frame, cut-ups by tag filter, an isolate-one-player camera, the position coaches' snap grades on the film. Agreed (Cain 21:31 "agreed on all, ship that to the final build list"). Section 11 X1-X8: nothing waiting.

### Unfinished thoughts sweep (Cain 2026-10-21 21:52 "D+E to final build"; thread "Unfinished thoughts sweep", bundle message cmsg_013Ukratj7v15wm7eeTHSDRz7sM3Gg27GPhnbDkcea4PMh)
- U-d. Real high school playoff formats and private-school rules per state, the 10 states' gaps filled from each association's handbook (answers open question Q2). Today each classification takes its best 2^rounds teams (`world/hs_year.rs`, the playoff field near line 1009). Research: `docs/research/hs-playoff-formats.md` (the table and its "Gaps").
- U-e. Weather from each town's real climate normals (NOAA), not one main town per state (`calendar/weather.rs`; calendar-checklist build plan Q5).
- The rest of that bundle (a-c, f-h) and its questions 1-2 still wait on Cain in that thread.

### Creative additions (Cain 2026-10-21 22:05 "agree on all"; thread "Creative additions brainstorm")
- C1. Rivalry trophies (real by name, generated for HS), kept for a year.
- C2. Fans storming the field after big upsets, with conference fines (SEC up to $500k from 2024).
- C3. A travelling national pregame show (GameDay-like, its own name) picks the game of the week; recruits book visits that weekend.
- C4. Uniform sets (home, away, alternates, throwbacks, summer reveals) you choose each week; recruits react.
- C5. Retired numbers, rings of honor and statues, with ceremonies for your former stars.
- C6. Your office fills with mementos (game balls, rings, covers, trophy photos) and moves with you.
- C7. Preseason magazines with predictions, all-conference picks and cover athletes.
- C8. Betting lines (spread, over/under); upsets judged by the line; coaches' ATS records; information only.
- C9. A documentary crew in your building (Hard Knocks since 2001, the NFL can force a club since 2013; college and juco docs): exposure and Aura, distraction by personality, your words on TV.
- C10. The yearly video game ratings release each August: public perception, not truth; players react.
- C11. Fantasy football points, rankings and pickup headlines.
- C12. A yearly jersey sales list (NFLPA; college since 2021 group licensing).
- C13. The title parade.
- C14. Championship rings and the ring ceremony (conference and bowl too; broke ex-players sell them).
- C15. Retirement farewells (the announcement, farewell-tour gifts from rivals).
- C16. The one-day contract to retire with his first team.
- C17. Signature celebrations by personality under each era's rules (the NFL loosened them in 2017; in college a taunt before the goal line takes the points off).
- Network (Cain 22:18 "agree on all"; his dialogue-screen idea, built out):
- N1. A Contacts screen listing everyone you have a tie with (coaches, GMs, owners, ADs, agents, reporters, scouts, HS coaches, former players): how you know him, warmth in words, last contact, what's owed either way.
- N2. Reaching out: call, text or lunch with anyone, using the talk system's moves (congratulate, condole, ask advice, share film, check in), costing time from your week; too often reads needy, and only reaching out when you need something gets noticed.
- N3. A favours ledger: help given and taken (a good word, hiring someone's son or assistant, a read on a prospect, a fair trade) logged and callable; an unrepaid favour cools the tie.
- N4. Networking trips on the calendar (AFCA convention, Senior Bowl week, the Combine, clinics, spring visits to other staffs): meetings with whoever is there, costing days away.
- N5. Warm ties open doors: a head coach or GM who likes you lists you, calls before a job is public, trusts your interview more; results and reputation still decide (it only gets you in the room).
- N6. Coaching an all-star game (Senior Bowl, Shrine Bowl staffs from NFL assistants), picked or asking.
- N7. Speaking at and hosting clinics: ties with HS coaches and reputation.
- NFL side (Cain 22:30 "agreed, push all to list"):
- P1. NFLPA team report cards (players grade owner, facilities, food, family room, travel, staff; public 2023-24, internal from 2025 after an arbitrator's ruling), read by free agents and agents; bad grades push owners to spend (ties to X1).
- P2. League discipline for clubs and coaches on the real scale (Bountygate 2012: Payton a full season; Dolphins tampering 2022: lost a 1st and a 3rd; Dallas $10M and Washington $36M cap penalties for the 2010 uncapped year), for you and AI clubs.
- P3. Medical second opinions and disputes: players pick their own surgeons and can fight team doctors (Trent Williams 2019), with grudges, trade requests and agents.
- P4. Star player power: a franchise QB wants a say in signings, the OC and the game plan (Rodgers 2021, Brady in Tampa); your answer shapes the tie and the room.
- P5. Your in-game decisions graded publicly the next morning (4th downs, timeouts; NYT 4th Down Bot 2014, EdjSports), read by critics and your boss.
- P6. Schedule release day in May; prime-time count from national appeal; flex scheduling (SNF from 2006, MNF/TNF from 2023) and short weeks.
- P7. International games (International Series since 2007: London, Mexico, Germany, Brazil), a home game given up, clubs' international marketing rights from 2022 growing fans and the pathway pool; travel costs body clock and prep.
- P8. Rookie rituals (veterans' dinner bills, e.g. Dez Bryant $54,896 in 2010; carrying pads) and where they cross the line (2013 Dolphins, the Wells report) as a chemistry and discipline test.

### Rule 0 and linking (Cain 2026-10-21 20:18 and 20:20, section 10 thread; screens.md topic 17, PR #84)
- Rule 0 for the whole build: calm never means less. Cain: "i still want the screens filled, it just has to be intuitive, aeshetical and luxury. Don't take this as an excuse to add less feature or data into screens". Screens stay full and FM-dense; links go on top, never in place of data.
- L1. Every noun and every number with a story behind it is a link, including the text in the inbox, news, Home and the calendar.
- L2. Events store the ids of the people, games and records they name (today they store only a team id, so that text is plain).
- L3. Each screen gets a "Go to" line, from the per-screen map in topic 17.
- L4. Links carry what you were looking at, so Back returns you to the same view.
- L5. A quick card on hover.
- L6. Shift-click opens a page beside the current one.
- L7. The breadcrumb is a path you can click.
- L8. A new Development tab on the player page (training topic 13's content).
- Proof Cain named: depth chart to Development and back to the same board.
- All screens.md picks marked "proposed, not confirmed" are confirmed (Cain, 20:18 "yes").

### Faces (Cain 2026-10-21 20:03: "let's implement faces into the final build"; agreed-to-build from `docs/design/faces.md`, memory faces-track and faces-3d-thread, renders under `afm/faces-3d/`)
- F1. 3D heads as the face baseline: the v7 ten (Style E MakeHuman heads, Google GNM face shapes, own-code strand hair and MakeHuman hair, free art only, no MetaHuman), accepted 2026-10-19 "for now". Reference sheet `afm/faces-3d/face-variety/gnm/v7/cmp-v7-approved-light.png`.
- F2. Every render in the approved light: render_v7 `LIGHT=appr SSS=.35 SPEC=.35 VT=AgX LOOK="AgX - Medium High Contrast" EXPO=-.35`. Compare each new sheet with the approved renders before showing it.
- F3. The face recipe is added at player creation on its own random stream (seeds unchanged); the face's heritage matches the name's, mixed ancestry allowed.
- F4. `names.rs` and `Ethnicity` split East, Southeast and South Asian, and Polynesian as its own group, so heads match names.
- F5. A new team headshot every 4 years on media day.
- F6. Lineups follow who really plays.
- F7. Your own photos: a "Change photo" button on any player's page (PNG, JPG, WebP, HEIC, crop box), copied into the save's folder, replaces the face everywhere, "Back to generated face", stays with him when he leaves.
- F8. Media day keeps your photo; a small "new team photo" note switches in one click.
- F9. "Copy AI prompt" beside it, built from heritage, age, height, weight, position, team colours, hair, beard and appearance, with random looks ("bleached dreads, scar above temple, fat").
- F10. Appearance per player, drawn at creation on its own stream: hair style and colour with dye at real rates, facial hair, scars and a broken nose, neck and forearm tattoos, earrings, tooth gap, freckles; build from weight, frame and position; looks change over the years (haircuts, greying, hairline). Rates counted from real roster headshots.
- F11. Silhouette with initials wherever no face exists yet (also section 10 item S4 below).
- Folder import of many photos: section 11 item X4.
- F12. A new headshot on any team change too (transfer, trade, signing), as well as every 4 years (2b). Cain 20:13: "yes if it doesn't cause too much delay". Faces are only drawn when a screen shows the player, then saved (screens.md topic 13, proposed, not confirmed).
- F13. ~~A hidden appeal score from facial symmetry (6a).~~ Replaced (Cain 20:43): "replace goodlooks with AURA. It's less controversial ... hard to put in numbers. So Aura is also determining if a player get's Ads for NIL deals and stuff". Aura decides who gets ads, NIL deals and endorsements (6b, yes through Aura). Looks/symmetry play no part in the sim. How Aura is made and whether it touches NFL pay: questions 18-19 below.
- F15. Aura (Cain 20:45 "agreed on both"): a hidden 1-20 trait of its own (charisma, camera presence, style), drawn at 14 with a small link to ego and leadership, growing a little with age and media training. Ads, NIL deals and endorsements come from Aura times fame (performance, stage, position, market), so Aura alone never makes a backup rich.
- F16. No NFL QB pay premium for looks or Aura (dropped, Cain 20:45).
- F14. The game draws the heads itself (same GNM shapes and strand hair, no Blender, under a second a face). Cain 20:29: "fine we'll test it". Test first: the build posts one comparison picture in the project chat (the same ten players drawn by the game's code next to the approved v7 ten), and faces ship only after Cain's ok.

### Section 10 points missing above (section 10 thread, 2026-10-21 20:03, checked against screens.md)
- S1. Level rating ("89 among seniors", `level_rating.rs`) on the player page and squad (topic 4).
- S2. Measurables with where they were measured ("4.48, combine"), listed ones in grey (topic 4).
- S3. Player hero: jersey number as the big outlined number (today it shows the position), position history on hover, the attribute group dropdown, 5 key mental attributes by default (topic 5).
- S4. Faces placeholder: silhouette with initials on the player page and as small round crops in squad, depth chart and recruiting lists; "Change photo" and "Copy AI prompt" on every player (topic 13).
- S5. "My players" as its own list with an add/remove button on any player (scouting part E); Home lists only noted players today.
- S6. Search also finds games, records, schools, towns, screens and your notes' text (topic 1).
- S7. Maps: the recruiting map (areas, territories, recruits), the poster shelf, the high school feeder-area map (topic 7).
- S8. Tables: "on my list" and "has my note" chips; a stat cell opens its leaderboard; long tables draw only the rows in view, sorted and filtered by the engine (topics 3, 14).
- S9. Speed: room drift pauses while days run and in the background; no glass on dense tables; frame times measured on the M1 and an Intel Mac (topic 14).
- S10. Back and forward by trackpad swipe and the mouse's back button (topic 1).
- S11. 5-in-5 eligibility: on the agreed off-field change list (section 4 topic 11, `calendar/rules.rs` `FiveInFive`); the section 10 thread reports it real since June 2026 while the world still uses redshirts. Real life is the default.

### Memory (budget 4 GB agreed in build 10)
- In-season memory is 5.1 GB on the 8 GB M1 (opening 3.3 GB). Compact the season's stores (game logs, the year's records, 8-byte season records) to get under 4 GB (`docs/game.md`, Doubts).

### Designed and agreed, not built (sections 6b, 8 and 10; detail per screen in https://claude.ai/artifact/9Z9kNGBk3EkhbAF1CQ9tbq)
- Your choices where the AI still decides: recruiting board, offers and visits, the portal and NIL.
- NFL: negotiating, tags, free agency, trades, waivers, the cap sheet by year, the draft board and war room.
- Game plan, call sheet, playbook, installs, the play designer.
- People: talks, press conferences, team rules, meetings with your boss and his goals.
- Staff: hiring and firing, coordinators' and position coaches' proposals.
- Scouting: the scouting file (7 tabs) and the department (areas, assignments, evaluation days, look-backs).
- Training (6b): the practice week by period, the time budget, habits, body plans, position moves, practice injuries, weekly growth, practice snaps on the engine.
- High school seat: varsity, JV and freshman squads; principal, AD and boosters as people; your teaching job; your school and feeder area page; staff people at AI high schools.
- Squad: six views and a Columns menu. Depth chart: the formation board you drag, position coaches' proposals, nickel/dime/goal line/special teams boards.
- Tables everywhere: second sort, Columns menu, saved views, filter builder, saved searches as watches, compare up to five, per-game/per-snap, CSV, arrow keys.
- Moving around: right-click actions on names, pinned must-answer items under Continue, a page in its own window, ⌘[ and ⌘].
- Staff reads: hover a range for who read it and from what; "last seen" marks.
- News: feeds by beat, national, recruits and fans; how sure each report is; a story's chain.
- Game page: scoreboard hero with win probability; Key plays, Advanced, Drive chart, Snaps, Charts, Watch tabs.
- Player page: Reports, Stats (seasons, game logs, splits), History with you; position fit list; actions; editing a note; pass charts, route maps, run paths.
- Coach page tabs (ratings as your read, tree, contract, reports); your profile's tree, contract and job market.
- Standings: playoff brackets. History: leaderboards, record books, Hall of Fame, award winners, the coaching carousel, the draft by year.
- Big moments: the reveals (hat ceremony, your pick, your hire, a title, a record falling) and the live rooms for signing day, the draft and cutdown day.
- Holiday mode on screen; Settings (text size, reduced motion); charts; the 2D view.
- Section 9 is now designed (see below); faces are now in (see Faces).

## 2. Answered by Cain tonight (all agreed; nothing left waiting)
Cain 20:54: "Trace non of these, just think of a plan as to how to fix it and will try it in the final build". So the build plans each fix from the code and real football, builds it, and measures it in the final build; no separate trace step.
- A1. Bug 2a: a birth date for every person; a class runs from September to August birthdays. Agreed.
- A2. Bug 2b: kids held back or reclassified (about 1 in 10), so some seniors are 19. Agreed.
- A3. Bug 2c: the long college roads the world lacks (2020 COVID year, medical hardship, juco, D2/FCS start then moving up, missions, military, older walk-ons); add only the missing ones. Agreed.
- A4. UCF's stadium: fix the data (real about 45,000). Agreed.
- A5. The D3 5-star DT with 34 sacks: KEEP it as a possibility. Cain: "if anything i love that as long as his stats match up to him being elite, it allows me to take a comfortable risk on him in the portal, but i repeat the ratings/ have to back it up." Build check: a lower-division player's huge numbers must come from ratings that are truly elite for his level (and visible to scouts), never from an engine quirk; if Luc Weekly's ratings don't back his line, fix the cause.
- A6. Dynasties (Georgia 18, Texas 10 of 31): add the real leveler. Agreed.
- A7. HS scoring 15.5 a team game: tune HS offense toward real points and yards a play. Agreed.
- A8. HS pass rushers (86.5 sacks): offenses slide protection, chip and double the best rusher. Agreed.
- A9. HS receivers (top 1,134 yards): targets lean toward the best receiver. Agreed.
- A10. HS interceptions (top 7): measure after A7 and A9, then tune. Agreed.
- A11. Mater Dei 248 straight: add the real leveler. Agreed.
- A12. 5-stars drafted 87.6% (real 62%): tune the part that runs above real. Agreed.
- A13. FBS rosters median 86 (real ~105): more local walk-ons stay. Agreed.
- A14. Game view item 2 (career story page) and items 4-20 (earned nicknames: triggers, who coins them, units, plays, effects, giving one, writing, merch, broadcast use, mocking names, redemption, reactions, hype names, coaches' names, the spread, rival fans' names, heir names). Agreed.
- A15. Keep the build's own picks in `docs/game.md` "Proposed, not confirmed" as they are. Agreed.
- A16. The new inbox (10a-10i) goes into tonight's build. Agreed.
- A17. The multi-agent bug sweep is in the final build. Cain: "Yes this build is allowed to use max help as long as bugs get found and immediately fixed."
- A18-19. Aura and no QB pay premium (F15, F16). A20. Section 9 in tonight's build (20:41 "yes").

## 3. Built or in flight (don't build again)
- PR #75: first playable (game layer, Showtime screens, Tauri Mac app, HS seat, training focus, recruiting and contracts screens).
- PR #78: "Get a world" download and "Build a world on this Mac".
- PR #79: start/setup pages scroll; the auto-updater.
- PR #80: seat audit, 70 UI bugs across HS, college and NFL (save in the Mac app, records 0-0, inbox, morning stops, NFL scores, depth chart order, setup pool, job list).
- PR #81: old free agents' price falls past their prime; another level's games don't stop Continue; 7 second-pass bugs.
- PR #82 (merged, live in AFM 0.2.23): new opening, Load game and six-step new career (notes items 1-6, 5a-5f, item 4 off Showtime), the four new ratings (item 7), world and save cards, rename and delete saves, your profile, 14 review bugs fixed. It also added a "Who calls the plays" choice on the Style step, which nobody asked for (the review found your play calling never reached college and NFL games without it).
- In flight: the GitHub world build (seed 2026, 30 years, started 16:10, about 3 h) and the first signed app update, in the "GitHub world download and app updates" thread. It is a different world from the one you imported.
- Earlier builds: fix list S6.1-S6.3 (build 9), trial 9 findings (build 10).

## Parked by Cain (not tonight unless he says)
- Trial 7 parked items (dynasties, NFL scoring, college stat lines, awards, trades, round 1 positions) until their systems exist.
- Trial 8: NFL passing 7.9 yards an attempt and FBS scoring "we'll tune later".
