# Section 10: Screens and the 2D view

The design for section 10 (roadmap: dense FM-style screens with a modern finish, player cards, the news inbox, history, the 2D Next Gen Stats-style replays, screens for each seat, your own photos). Thread "Design the UI", started 2026-10-20. Design only, nothing here is built yet.

How this doc works:
- Section 8 (`docs/design/first-playable.md`) already settled the app (Tauri, HTML screens drawn by the Mac's WebKit, the engine on its own thread) and lists every screen a full year needs at each level (its topic 5). This section designs how those screens look and work. It re-asks neither.
- Each topic says what exists: **Built** is in the code, **Decided** is confirmed design, **New** is what this section adds.
- Real life is the default. For a UI, "real life" means two things: how real football staffs see their information (depth boards, call sheets, the All-22, Next Gen Stats, a draft war room), and the conventions of the management games Cain plays (Football Manager). Where those settle it, it's written in. Taste choices nobody can settle for Cain are listed at the end with Claude's picks; the doc is written on the picks until he answers.
- The mock-up of the key screens is the "AFM screens mock" canvas (https://claude.ai/artifact/7CdxyHAryWF24XyA7pvdqF): home, roster, player page, game page and the 2D replay, all drawn on the picks.

Topic order: 1 the window, 2 the look, 3 tables, 4 showing what your staff knows, 5 the player page, 6 people and your own profile, 7 your team's screens by seat, 8 inbox, news, talks and press, 9 the big-moment screens, 10 stats, game pages and history, 11 charts, 12 the 2D view, 13 faces and photos on screen, 14 speed and memory, 15 comfort and accessibility, 16 what section 10 builds and when.

---

## Topic 1. The window

### What exists
- **Decided:** one Continue button always in the same place, with the date and what's next; a progress line while days run; it says why it stopped (section 8 topic 4). Every name is a link to its page (section 8 topic 5). A "Report a bug" button on every screen (section 8 topic 10).
- **Background:** FM24 and earlier keep a left sidebar (Home, Inbox, Squad, Tactics, Staff, Scouting, Finances...) so any area is one click away. FM26 (November 2025, rebuilt on Unity) replaced the sidebar with cards, tiles and pop-ups; reviewers called it a "labyrinth" where one click became several, and Miles Jacobson said SI knew it wasn't popular with everyone (fmscout, Galaxus, GosuGamers, 2025).
- **New:** the frame every screen sits in.

### The design
- **The frame, on the pick (choice 1):** a left sidebar with every area of your job, one click each; a top bar with back and forward, the screen's breadcrumb, a search box, the date and the Continue button at the top right; the screen itself fills the rest. The sidebar shows only what your seat has: the NFL seat shows Cap and Trades, the college seat Recruiting and NIL, the high school seat Your school and Feeder area.
- **Back and forward behave like a browser,** with the Mac trackpad swipe and the mouse's back button. Every screen keeps its sort, filters, tab and scroll when you come back to it. (FM26's reset selections were one of its main complaints.)
- **Search everything** (⌘K): any player, coach, team, school, town, game, record or screen by name, with level and position shown so the 40 people named Jalen Williams can be told apart.
- **Right-click on any name** gives the actions that fit: open, add to "my players", compare, scout, offer, talk, trade for, make an offer, add to the shortlist.
- **Pinned items** sit under Continue: things that must be answered (section 8 topic 4) with their deadlines.
- **Size.** Laid out for Cain's 13-inch MacBook (1440 by 900 points, the M1 Air's default), working down to 1280 by 800 and up to a full external screen. Any page can open in its own window (⌘-click), so a player page can sit beside the depth chart on a second screen.

---

## Topic 2. The look

### What exists
- **Decided:** dense tables like FM, with a modern finish (Q45). Visuals never decide anything (2026-10-10). Attributes are 1-99 like Madden (2026-10-10, option A), with the five hidden traits 1-20 never shown.
- **Built:** the Debug Lab (`tools/dashboard/`) and the player pages are today's HTML screens: dense tables, tabs, dark background.
- **New:** type, colour, density.

### The design
- **Dense first.** Table rows 26 px high, 13 px text, numbers in a tabular font so columns line up. A roster of 90 shows about 28 players on a 13-inch screen without scrolling. A compact setting goes to 22 px rows.
- **Type:** one clean sans for everything (Inter), with tabular figures; no decorative display font.
- **Theme:** dark by default with a light theme in settings (choice 2). Dark is FM's default and easier for long evenings; both are built from the same colour tokens.
- **Your team's colours** (choice 3, pick yes): the top bar strip, your team's rows in standings and your side in the 2D view use your team's real colours (the colleges' and NFL clubs' real colours exist with the world; a high school gets its generated colours). The rest stays neutral so ratings and charts read the same for every team. Other teams' pages carry their own colours in their header.
- **Rating colours.** A 1-99 rating is coloured in five bands that also differ in lightness, so they read without colour vision: under 50, 50-64, 65-74, 75-84, 85 and up. The bands are relative to the level where it matters (the level rating, built: "89 among seniors"). A range is drawn as a bar from its low to its high end (topic 4).
- **No meters for people.** Morale, trust and chemistry never get a bar or a number (people topic 3, decided): they show as your staff's words and the tells.
- **Era look in the news only.** The screens stay modern in every year; the news screen follows the era's media (people topic 9: a newspaper page in 1998, a feed of posts today).

---

## Topic 3. Tables

### What exists
- **Decided:** every stat (Q47), leaderboards filtered by level, state, conference, season, position, age and class; custom searches ("running backs, 2027, Texas 4A-6A, at least 150 carries, yards after contact per carry above 4"); side-by-side comparisons (match engine topic 8.8).
- **New:** how every table works.

### The design
Every list in the game is one table component, so it works the same everywhere:
- **Sort** by any column (click; shift-click for a second sort).
- **Columns:** pick any column from a grouped list (bio, body, attributes, stats by season or career, contract, scouting, recruiting). Save the set as a named **view** ("Pass rushers", "Cap sheet"), like FM's custom views; each table remembers its last view.
- **Filters:** quick chips at the top (position, level, class, injured, on my list) and a filter builder for anything else, with AND and OR, that also builds the custom searches. A search can be saved and turned into a watch: the inbox tells you when a new player matches.
- **Select rows** to compare (up to five side by side), add to a list, or act on them.
- **Stat columns** show per game, per snap, totals or ranks; any stat cell opens its leaderboard.
- **Big tables only draw the rows on screen** (topic 14), so a leaderboard of every high school back in the country scrolls as fast as a roster.
- **Export** any table to CSV, so a stat nerd can take it into a spreadsheet.

---

## Topic 4. Showing what your staff knows

### What exists
- **Decided:** ranges that sharpen with better scouts, plus hints (Q18); never an exact ceiling (Q46, "pivotal"). Each staff keeps its own file on each player: a range per attribute, "unknown" where nobody has seen it, measured against listed measurables, staleness and the evidence behind each read (scouting topic 1). Potential as words: role tier, floor and ceiling in words, how sure, hints, comparisons (scouting topic 7). Tendencies as a range plus a word (`tendencies.rs`, `TendRead`, built).
- **Found in the audit:** `docs/design/scouting.md` topic 1 writes an attribute range as "12-15" and topic 4 says ranges end "two to three points wide on the 1-20 scale". Attributes are 1-99 (2026-10-10, option A), so the on-screen range is on 1-99, for example "62-70", and a well-scouted senior's range is about 6-8 points wide on 1-99 (the same share of the scale). Corrected here; the scouting doc is pointed to this note.
- **New:** how it looks.

### The design
- **A range is drawn as a bar** spanning low to high in the rating band colours, with the numbers beside it ("62-70"). A narrow bar is a sure read; a wide one says your staff isn't sure. Unknown shows as a grey dash.
- **Your own players** are read by your coaches daily (decided), so their ranges are narrow, but still ranges.
- **Stale reads are marked:** a small clock and "last seen 14 Mar" on a file older than the season's phase; "needs a new look" when your staff noticed a change.
- **Every read opens its evidence:** hover a range to see who read it, from which looks, and each person's grade, the way a real club's file is a stack of dated reports.
- **Measurables** show exact with where they were measured ("4.48, combine"), or "listed 6-3, 215" in grey.
- **Potential in words only**, at the top of the player page beside the role tier ("Power Four starter, boom or bust, staff fairly sure") with the hints under it. No stars, no bar, no number.
- **The level rating** ("89 among seniors") shows as a number, as built and decided: it is your staff's read of where he ranks now, not his ceiling.

---

## Topic 5. The player page

### What exists
- **Built:** the trial's player pages (cards): bio, body, every season at every level with "What happened", offers, draft, pro career.
- **Decided:** what the card holds (Q46 without true potential); character reads in "insane detail" (scouting topic 5); medical reads (topic 6); mood and trust in words, his ties (people topics 1-3); history with you on the "my players" list (scouting part E); the face or your photo with "Change photo" and "Copy AI prompt" (faces design).
- **New:** the layout.

### The design
**The header** stays on every tab: the face (or your photo), name, position (with his position history on hover), number, team, class or years in the league, age, hometown, height and weight, the level rating, your staff's role tier and potential words, his status (injured, suspended, on the portal, a free agent), and the actions (talk, scout, offer, add to my players, compare). Under the face: "Change photo", "Copy AI prompt", and "New team photo" when media day shot one.

**Tabs:**
1. **Overview:** your staff's summary in words, the attribute groups at a glance, this season's stats, recent form, his mood in your staff's words with its top reason, the latest news about him.
2. **Attributes:** all 67 in their 8 groups as ranges, and the position fit list (built: every player is rated at every position) with your staff's best position and the reason.
3. **Reports:** every report on him, dated, by whom, with each grade, the character read and the medical read.
4. **Stats:** every season at every level, every stat, with game logs, splits and advanced stats.
5. **Career:** the timeline from age 14: schools, recruiting, offers and visits, transfers, the draft, contracts, injuries, awards, with the "What happened" line for each season.
6. **Tendencies:** the 15 tendencies as ranges with their words, and your instructions to him.
7. **Contract** (pros) or **Recruiting** (recruits) or **School** (high school): what applies to his level.
8. **History with you** (when he's on "my players"): when you first saw him, your grades then, what the public said, every talk.

Where the game view's waiting items would sit (not asked here, still with Cain in the Bird's-eye game view thread): an earned nickname in the header under his name and in the news and play-by-play (brainstorm items 4-20).

---

## Topic 6. People and your own profile

### What exists
- **Decided:** coaches and staff as people with ratings in words from your own read, records and coaching trees (people topics 10, 11, 15); your profile with ratings, reputation in words, record and coaching tree (section 8 topic 5).
- **New:** the layout.

### The design
- **A coach or staff page** uses the same header and tabs as a player page: Overview, Ratings (as your read, a range, for anyone not on your staff; your own staff you know better), Record (every season, every job), Tree (who he worked under and who worked under him, as a tree you can click through), Contract, Reports.
- **Your profile** has the same shape: your ratings as exact numbers (they're yours), your reputation in the words the media and the market use, your record, your tree, your contract and job market.
- Where the career story page would sit (brainstorm item 2, waiting on Cain): a Story tab on your profile.

---

## Topic 7. Your team's screens by seat

### What exists
- **Decided:** the screen list per seat (section 8 topic 5); depth charts with your position coaches' proposals and reasons; game plans, playbooks, installs and the play designer (plays and tactics topics 4, 5, 8); the practice schedule editable to the period, each player's plan, install progress, block reports (training topic 13); the cap, contracts and the term-by-term negotiation board (team management topic 4); recruiting boards, visits and signing days (scouting topics 14-16); the high school seat (section 8 topic 7).
- **New:** how the key ones look.

### The design
- **Depth chart:** drawn the way a real staff's depth board is, as the formation on the field: each spot with its first, second and third player stacked, ranges and status beside each name, dragged to reorder. A list view sits beside it. Each position coach's proposal shows as a ghost name with his reason, which you accept or ignore. Package depth (nickel, dime, goal line, special teams units) are their own boards.
- **Game plan:** laid out like a real call sheet: situations down the side (opening script, 1st and 10, 3rd and short/medium/long, red zone, two-minute, coming out), plays in each box with their install level. Scouting of the opponent fills a panel beside it (section 9 designs the opponent report).
- **Play designer:** a top-down field with the formation, routes drawn as paths with their branches, blocking arrows and rush paths, in the coaching-diagram style (circles for offense, Xs or Vs for defense), with legality flags (plays and tactics topic 8).
- **Practice:** a calendar of the week by day and period, each period a block coloured by type (individual, group, team, special teams, walkthrough) with its contact level; click a period to set it or watch it (topic 12).
- **Cap (NFL):** the cap sheet as a table by year (cap hit, dead money, cash, guarantees, void years), with the term-by-term negotiation board as its own screen.
- **Recruiting (college):** the board as a table with each recruit's status, interest in your program in your staff's words, visits and offers; a map of the country by state and town, with your recruiting areas, your coaches' territories and the recruits on it.
- **Your school and feeder area (high school):** your school's page (enrollment, classification, principal and athletic director, boosters, facilities) and a map of the town with the feeder middle schools and youth teams, and the kids coming up.

---

## Topic 8. Inbox, news, talks and press

### What exists
- **Decided:** medium events in the inbox, big ones their own screen, huge ones the full show; small ones news lines with filters (calendar topic 8, section 8 topic 4). Talks as picked moves and tones with written answers and tells over 2-5 exchanges (people topic 5); every line from the huge line bank (people topic 5). News by era with how sure each report is (people topic 9).
- **New:** the layout.

### The design
- **Home** is a dashboard of your job: the inbox, the next seven days, your boss's latest word, the team in words (form, injuries, mood in your staff's words), and the news that concerns you.
- **The inbox** is a two-pane list: messages on the left (unread bold, pinned on top, filter by type), the message on the right with its actions as buttons (open the report, answer the agent, hand it to staff).
- **A talk** opens as its own screen: him on the left (face, mood in words, what your staff thinks he wants), the conversation in the middle, your moves and tones as a list on the right. His tells show inside his lines ("looks at the floor", "brings up his agent"), never as a meter.
- **Press conferences** use the same screen with the reporters as the other side.
- **News** is a feed by level and era: your beat, national, your recruits' feeds, fan reaction, each report marked with its certainty ("per sources", "has agreed"). A story opens its chain: first report, who repeated it, the response.

---

## Topic 9. The big-moment screens

### What exists
- **Decided:** big moments open their own screen, huge ones the full show: the war room on draft night, cutdown day hour by hour, the deadline's last hours, signing day hour by hour, the playoff bracket (section 8 topic 4); the live draft and the undrafted rush (scouting topics 12 and 13).
- **New:** how they look.

### The design
- **The war room:** your board on the left (your order, tiers, who's gone greyed out), the pick clock and the live pick feed in the middle, trade calls coming in on the right, with your staff's voices ("our guy is still there", "Philadelphia is calling about 14"). The clock runs in real minutes you can speed up or pause. After the last pick the undrafted rush opens in the same screen: a list of who's left and your offers.
- **Signing day:** the hour-by-hour feed of commitments with your board's targets highlighted, hat ceremonies in the news, and your calls.
- **Cutdown and the deadline:** the clock to the deadline at the top, your roster count against the limit, the waiver wire and trade offers as they come.
- **Playoffs:** the bracket for every level (state playoffs by classification, the College Football Playoff, the NFL), every game a link.

---

## Topic 10. Stats, game pages and history

### What exists
- **Decided:** every stat at every level, kept forever (Q47); game pages with the box score, a drive chart, the win-probability graph and the play-by-play with its why; leaderboards, custom searches, comparisons, career pages (match engine topic 8.8); Hall of Fame, awards and record books (Q48, built PR #42).
- **New:** the layout.

### The design
- **A game page:** the score by quarter, the win-probability line across the game with the big swings marked, then tabs: Box score (team and every player), Drive chart, Play-by-play (every play with its why: "Cover 3, the deep third left open by the corner's jump"), Advanced (EPA, success rate, pressure, separation by player), and Watch (topic 12).
- **Leaderboards** are tables (topic 3) with level, state, conference, season, position, age and class filters.
- **Record books** per state, conference, level and all time, with streaks, each record linking to the game and the player.
- **History:** every season of every team, award winners by year, the Hall of Fame with each member's page, the coaching carousel by year, the draft by year.

---

## Topic 11. Charts

### What exists
- **Decided:** highlights in the style of Next Gen Stats: pass charts, route maps, run paths (2026-10-10, Cain's pass chart example).
- **New:** which charts and where.

### The design
- **Pass chart:** every throw of a game or season drawn from the line of scrimmage to where it was caught or fell, coloured by result (complete, incomplete, touchdown, interception), on the player page and the game page.
- **Route map:** a receiver's routes from the tracking, his separation at the catch point on each.
- **Run paths:** a back's carries drawn from the handoff, with yards before and after contact.
- **Pressure map:** where a pass rusher won and how fast.
- **Win probability** and **EPA by drive** on game pages; **season trends** as small lines in tables (form over the last games).
- All charts read the stored tracking numbers for every game; the full paths come from replaying the game from its seed (topic 12).

---

## Topic 12. The 2D view

### What exists
- **Built:** the engine tracks all 22 players and the ball ten times a second (`game/sim.rs`, `Frame`: time, ball, each player's id and position) and keeps full tracking when a game is recorded (`game/flow.rs`, `record`).
- **Decided:** text and stats are the main view, plus an FM-style 2D view (2026-10-10). The engine's positions are the contract with any view, so a better 2D or 3D view can come later without engine changes (match engine 9.3). Your game takes about 15 minutes to watch, with key plays, more plays or every snap, and Next Gen Stats overlays (separation, speed, time to throw); between plays you can step in (match engine topic 9). Any past game can be watched back in 2D afterwards by replaying it from its seed, at no storage cost (9.2). Any practice period can be watched (training topic 8).
- **Not designed yet:** section 9, game day control. The view leaves room for it (below).
- **New:** how it looks and plays.

### The design
- **The field from above,** drawn to scale with real markings by level (high school, college and NFL hash marks differ: NFL hashes are 18 ft 6 in apart, college 40 ft, high school 53 ft 4 in), the end zones in the home team's colours and name, the line of scrimmage and the line to gain.
- **Players are numbered dots** in team colours (choice 4), the way FM's 2D and Next Gen Stats draw them. The ball carrier gets a ring, the ball its own small marker with its height shown by size while in the air.
- **The camera** follows the ball at a sideline height that shows all 22 by default (the coaches' All-22 view), with a tight view on the ball and an end-zone view, and free zoom.
- **Playback:** play, pause, step a tenth of a second at a time, slow motion, replay the play, next and previous play. The engine's ten frames a second are smoothed to the screen's 60.
- **Overlays** you turn on: route and run paths as trails, each receiver's separation from his nearest defender, speed of the ball carrier, time to throw and the pocket clock, the pass rush's time to pressure, the coverage shell before the snap. Names on hover.
- **Under the field:** the play-by-play line with its why, the down, distance, clock, score and win probability.
- **How much you watch:** key plays, more plays, or every snap (decided), and a speed for the plays you don't watch.
- **Room for section 9:** a panel to the right of the field is kept for the call: your coordinator's call drawn up, accept or send back, your own call from the call sheet, and the between-play actions (timeout, substitutions, a note to the coordinator). Section 9 designs what goes in it.
- **Replays cost nothing to store:** a past game is replayed from its seed, in the background, in about a second for one game (match_check plays 2,000 high school games in 16 s on 4 cores); the view waits only for that.
- **Practice periods** play in the same view with the period's rules (thud ends the play at contact).
- **3D later:** the view reads only the frames, so a 3D view can replace it without engine changes (decided).

---

## Topic 13. Faces and photos on screen

### What exists
- **Decided:** a face for every player (faces decisions 1-5), a new headshot every 4 years on media day, a silhouette with initials until faces are drawn; your own photo on any player, kept through media day, with "Change photo", "Copy AI prompt" and the looks list (faces design, PR #53). The face baseline: the v7 3D heads in the approved light, accepted for now on 2026-10-19; face work paused.
- **New:** where faces show and how they load.

### The design
- **Faces show** in the player page header (large), the talk screen, the war room, news items, the inbox and lists as a small round crop (roster, depth chart, recruiting board), so you know your players by face.
- **Your photo** replaces the face everywhere he shows up; the generated face is the default.
- **A face is drawn once and kept** as a small image in the save (about 30-60 KB at the sizes shown), only for players a screen has shown (faces design: "only faces you look at are drawn"). Lists show the silhouette with initials until a face is ready, so a screen never waits.
- **How the shipped game draws a face is open** until face work restarts (the 3D heads are rendered with Blender today, which the game can't ship). Until then the game shows the silhouette with initials and your photos. Not a question now; it comes back with the faces track.

---

## Topic 14. Speed and memory

### What exists
- **Decided:** the screens take about 150-300 MB beside a 4 GB world; the engine works on its own thread so screens never freeze; a screen asks only for the rows it shows (section 8 topics 8 and 9).
- **New:** how the screens keep to it.

### The design
- **Screens ask the engine for what they show:** a table asks for the rows in view plus a margin, sorted and filtered on the engine's side, so a leaderboard of 300,000 high school players costs the same as a roster.
- **Text is written when a screen opens** (people topic 9, decided) and dropped when it closes.
- **Charts and the 2D view draw on a canvas** (no element per dot), which runs at 60 frames a second on the M1.
- **Faces** load as small images, a few hundred at most at once.
- **Targets, proposed, not confirmed, measured on the M1 in the build:** a screen opens in under 100 ms; sorting a 90-man roster is instant; a leaderboard of every high school player sorts in under half a second; the screens stay under 300 MB.

---

## Topic 15. Comfort and accessibility

### The design
- **Text size** scales from 90% to 130% in settings without breaking the tables.
- **Colour** never carries meaning alone: rating bands differ in lightness, results in charts also differ in shape (a filled dot for a completion, a hollow one for an incompletion, a star for a touchdown).
- **Keyboard:** every screen works by keyboard; Space is Continue (FM's habit); ⌘K searches; ⌘[ and ⌘] go back and forward; arrow keys move through tables.
- **Reduced motion** follows the Mac's setting: no animated transitions; the 2D view still plays.

---

## Topic 16. What section 10 builds and when

### What exists
- **Decided:** playable early with plain screens, then depth and polish while playing (Q50, option A). Section 8 builds plain, dense text screens with no art; section 10 brings the finish and the 2D view (section 8 topic 5).

### The design
- **The frame, the look, the table component and the range bars** (topics 1-4) are the base every screen uses. On the pick (choice 5), section 8's plain screens are built on them from the start, so the first playable already has this look and nothing is built twice.
- **Section 10's own build** then adds what plain screens leave out: the charts (topic 11), the 2D view (topic 12), faces in the screens (topic 13), the war room and the other full shows' finish (topic 9), the maps (topic 7), and the play designer's drawing (topic 7).
- **Section 9** designs and builds game day control into the panel the 2D view keeps for it.

---

## Choices for Cain

Posted in the thread on 2026-10-20 with Claude's picks. The doc is written on the picks until Cain answers.

1. **How you move around the game.** A left sidebar with everything one click away, like FM24 (pick), or FM26's cards and tiles. Proposed, not confirmed.
2. **The default theme.** Dark (pick), with light in settings, or light by default. Proposed, not confirmed.
3. **Your team's colours on the screens.** The top bar strip, your rows and your side in the 2D view in your team's colours (pick), or one neutral AFM look for every team. Proposed, not confirmed.
4. **Players in the 2D view.** Numbered dots like FM's 2D and Next Gen Stats (pick), or small top-down figures with helmets. Proposed, not confirmed.
5. **Section 8 builds its plain screens on this frame and table component from the start** (pick), or plain screens first and a restyle in section 10. Proposed, not confirmed.
