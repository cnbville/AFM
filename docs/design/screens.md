# Section 10: Screens and the 2D view

The design for section 10 (roadmap: dense FM-style screens with a modern finish, player cards, the news inbox, history, the 2D Next Gen Stats-style replays, screens for each seat, your own photos). Thread "Design the UI", started 2026-10-20. Design only, nothing here is built yet.

How this doc works:
- Section 8 (`docs/design/first-playable.md`) already settled the app (Tauri, HTML screens drawn by the Mac's WebKit, the engine on its own thread) and lists every screen a full year needs at each level (its topic 5). This section designs how those screens look and work. It re-asks neither.
- Each topic says what exists: **Built** is in the code, **Decided** is confirmed design, **New** is what this section adds.
- Real life is the default. For a UI, "real life" means two things: how real football staffs see their information (depth boards, call sheets, the All-22, Next Gen Stats, a draft war room), and the conventions of the management games Cain plays (Football Manager). Where those settle it, it's written in. Taste choices nobody can settle for Cain are listed at the end with Claude's picks; the doc is written on the picks until he answers.
- The mock-up of the key screens is the "AFM screens mock" canvas (https://claude.ai/artifact/7CdxyHAryWF24XyA7pvdqF). Round 2 (2026-10-20) draws eight screens on Showtime: home, roster, player page, Game Center, the 2D replay, the recruiting board, early signing day with the reveal, and a talk with a player. Round 3 (the same afternoon) works in Cain's comments on the canvas and his ask for a pass-over ("do a pass-over, i want ultra depth in certain aspects, especially scouting"): every screen got deeper, and three scouting screens were added: a recruit's full scouting file, the scouting department, and the NFL seat's draft board.

Topic order: 1 the window, 2 the look (Showtime), 3 tables, 4 showing what your staff knows, 5 the player page, 6 people and your own profile, 7 your team's screens by seat, 8 inbox, news, talks and press, 9 the big-moment screens, 10 stats, game pages and history, 11 charts, 12 the 2D view, 13 faces and photos on screen, 14 speed and memory, 15 comfort and accessibility, 16 what section 10 builds and when.

---

## Topic 1. The window

### What exists
- **Decided:** one Continue button always in the same place, with the date and what's next; a progress line while days run; it says why it stopped (section 8 topic 4). Every name is a link to its page (section 8 topic 5). A "Report a bug" button on every screen (section 8 topic 10).
- **Background:** FM24 and earlier keep a left sidebar (Home, Inbox, Squad, Tactics, Staff, Scouting, Finances...) so any area is one click away. FM26 (November 2025, rebuilt on Unity) replaced the sidebar with cards, tiles and pop-ups; reviewers called it a "labyrinth" where one click became several, and Miles Jacobson said SI knew it wasn't popular with everyone (fmscout, Galaxus, GosuGamers, 2025).
- **New:** the frame every screen sits in.

### The design
- **The frame, on the pick (choice 1):** a left sidebar in Showtime's frosted `chrome` with your team's mark on top and every area of your job, one click each; Showtime's blurred top bar with back and forward, the screen's breadcrumb, a search box, the date and the Continue button at the top right; the screen itself fills the rest. The sidebar shows only what your seat has: the NFL seat shows Cap and Trades, the college seat Recruiting and NIL, the high school seat Your school and Feeder area.
- **Back and forward behave like a browser,** with the Mac trackpad swipe and the mouse's back button. Every screen keeps its sort, filters, tab and scroll when you come back to it. (FM26's reset selections were one of its main complaints.)
- **Search everything** (⌘K): any player, coach, team, school, town, game, record or screen by name, with level and position shown so the 40 people named Jalen Williams can be told apart. It also searches the text of your own notes (topic 5).
- **Right-click on any name** gives the actions that fit: open, add to "my players", compare, scout, offer, talk, trade for, make an offer, add to the shortlist.
- **Pinned items** sit under Continue: things that must be answered (section 8 topic 4) with their deadlines.
- **Size.** Laid out for Cain's 13-inch MacBook (1440 by 900 points, the M1 Air's default), working down to 1280 by 800 and up to a full external screen. Any page can open in its own window (⌘-click), so a player page can sit beside the depth chart on a second screen.

---

## Topic 2. The look: Showtime

### What exists
- **Decided:** dense tables like FM, with a modern finish (Q45). Visuals never decide anything (2026-10-10). Attributes are 1-99 like Madden (2026-10-10, option A), with the five hidden traits 1-20 never shown.
- **Decided (Cain, 2026-10-20 11:08):** round 1 of this design (a plain dark-grey FM look) left Cain "severly underwhelmed". He handed over his own design system, **Showtime** ("the look of the All-22 mock draft room, made reusable"), now in the repo at `ui/showtime/` (`README.md`, `tokens.json`, `showtime.css`, 18 components with examples). AFM's screens are built on it. That settles round 1's choices 2 (dark is Showtime's main theme, light is opt-in) and 3 (the subject's colours light the screen).
- **Built:** the Debug Lab (`tools/dashboard/`) and the player pages; they stay trial tools and don't move to Showtime.
- **New:** how Showtime's rules map onto a football management game.

### The design
Showtime's five principles, applied to AFM:
1. **The room is dark, the subject is the light.** `accent` and `accent-2` are always the real colours of whatever the screen is about: your team on your screens, a player's team on his page, a recruit's high school on his poster, the two teams on a game page (home and away blended in the room), the opponent on its scouting report. Moving from your page to a rival's re-lights the screen over one second.
2. **One hero, one moment.** Overview screens (home, a team, a player, a coach, a game) open with a hero or a room. Data screens (roster tables, the cap sheet, leaderboards, the depth chart's list view, record books) keep the hero small and the room off, and use the dense table and quiet `fill-1` rows, as Showtime's README says for heavy programs. FM-level density lives there.
3. **Glass over colour, never over grey.** Frosted panels sit only over a room, a hero or a photo; panels on the plain ground are `raised`.
4. **Numbers are shapes.** Jersey numbers, ranks on a board, draft picks, the week of the season, scores and a recruit's high school number use the giant outlined numeral. Stats use Archivo condensed with tabular figures, so columns line up.
5. **Calm, then motion.** Things rise in once. Only real events pulse: the pick clock, a player announcing, a trade call, a live game. The `live`, `good` and `gold` colours mean states only, and gold appears at most once a screen.

How the pieces map:

| Showtime piece | In AFM |
|---|---|
| Hero | Your team's home, a player's page, a coach's page, a game's scoreboard |
| Poster shelf | Recruiting targets, draft prospects, the week's storylines, a player's comparisons |
| Ranked rows | The draft board, the recruiting board, the depth chart's order, leaderboards' top ten |
| NowCard and timer | Kickoff countdown, the pick clock, a recruit announcing, the trade deadline, cutdown day |
| Feed | The live draft, signing day, the news, the inbox, a game's key plays |
| Sheet with ledger | Trade offers, contract offers, the scholarship and NIL budget, a portal offer |
| Meters | Your staff's attribute ranges (the filled part spans low to high), measurables against the position |
| Moment | The huge moments only (topic 9) |
| Banner | Things that must be answered (section 8 topic 4) |
| Table (dense) | Every data table (topic 3) |

- **Type:** Archivo at three widths (wide for titles, normal for names, condensed for numbers). The app ships the font inside it (SIL Open Font Licence), so it works offline.
- **Ranges** are Showtime meters whose filled part spans the staff's low to high read; an attribute nobody has seen is the dashed "estimate" meter. Morale, trust and chemistry never get a meter (people topic 3, decided): they stay words and tells.
- **Light theme** is Showtime's opt-in `data-theme="light"`, in settings.
- **Data screens are a warm room** ("make it look like it belongs in a warm room, not a cheap mobile game", Cain, 2026-10-20, on the roster). How the mock reads it, proposed, not confirmed: the room's light is the subject's colours dimmed and warmed, like low lamps in a film room. Tables sit on warm, nearly opaque glass with warm hairlines and faint zebra rows. Tabs are underlined, filters are segmented with hairline borders, and a status is a small dot with words. No pill on every cell, no neon, no big rounded mobile cards.
- **The news follows the era** (people topic 9: a newspaper page in 1998, a feed of posts today); the screens stay Showtime in every year.

## Topic 3. Tables

### What exists
- **Decided:** every stat (Q47), leaderboards filtered by level, state, conference, season, position, age and class; custom searches ("running backs, 2027, Texas 4A-6A, at least 150 carries, yards after contact per carry above 4"); side-by-side comparisons (match engine topic 8.8).
- **New:** how every table works.

### The design
Every list in the game is one table component, so it works the same everywhere:
- **Sort** by any column (click; shift-click for a second sort).
- **Columns:** pick any column from a grouped list (bio, body, attributes, stats by season or career, contract, scouting, recruiting). Save the set as a named **view** ("Pass rushers", "Cap sheet"), like FM's custom views; each table remembers its last view.
- **Filters:** quick chips at the top (position, level, class, injured, on my list, has my note) and a filter builder for anything else, with AND and OR, that also builds the custom searches. A search can be saved and turned into a watch: the inbox tells you when a new player matches.
- **Every column says where it belongs** (Cain on the roster, 2026-10-20: "Spacing between columns feels off, and it's unclear what column fits in where"). A group row above the headers names each block of columns and whose opinion it is ("Athletic · Opinion: Dane Kowalski, Strength Coach"). A hairline divides the groups. Numbers are right-aligned and words left-aligned, each header lines up exactly with its cells, and each header has a tooltip saying what it is.
- **Quick filters for the common cuts** ("i'm seeing filter system so i can quickly check all freshhman or seniors"). A roster has Unit (all, offense, defense, special teams) and Class (Fr, So, Jr, Sr, each with its redshirt form) as segmented buttons with counts, plus toggles (injured, in a redshirt year, unsettled). They combine, so "defensive seniors" is two clicks.
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
- **A range is drawn as a bar** spanning low to high, with the numbers beside it ("62-70"). A narrow bar is a sure read; a wide one says your staff isn't sure. An attribute nobody on your staff has seen shows "Not seen" on a dashed track.
- **The bar's colour follows the value:** "the lower, the whiter it is, the higher the redder it is" (Cain, 2026-10-20, on the mock). The colour is set by where the bar sits on the 1-99 track, so a 50 is nearly white and a 90 is deep red. Claude's reading, proposed, not confirmed: it is one fixed rating red on every page, not the team's colour, so a 90 looks the same on any team's page and stays readable for teams whose colours are dark or pale.
- **Every group of reads says whose opinion it is,** on its own line under the group's title: "Opinion: Andre Lyle, WR Coach" (Cain, 2026-10-20). A group read by nobody says "Not seen by your staff".
- **Your own players** are read by your coaches daily (decided), so their ranges are narrow, but still ranges.
- **Stale reads are marked:** a small clock and "last seen 14 Mar" on a file older than the season's phase; "needs a new look" when your staff noticed a change.
- **Every read opens its evidence:** hover a range to see who read it, from which looks, and each person's grade, the way a real club's file is a stack of dated reports. In a scouting file (topic 7) a click on any attribute shows how its range narrowed look by look, and each reader's own range beside the blend (proposed, not confirmed).
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
**The hero** opens his page, lit by his team's colours (a recruit's by his high school's), with his jersey number as the giant outlined numeral behind his name. A smaller version of it stays on every tab: the face (or your photo, on a portrait card in the hero), name, position (with his position history on hover), number, team, class or years in the league, age, hometown, height and weight, the level rating, your staff's role tier and potential words, his status (injured, suspended, on the portal, a free agent), and the actions (talk, scout, offer, add to my players, notes, Dynamic Links, compare). Cain asked for the Dynamic Links button on 2026-10-20 and will explain what it does later; until then the mock has it as a placeholder. Under the face: "Change photo", "Copy AI prompt", and "New team photo" when media day shot one.

**Tabs:**
1. **Overview:** your staff's summary in words, the attribute groups at a glance, this season's stats, recent form, his mood in your staff's words with its top reason, the latest news about him.
2. **Attributes:** all 67 in their 8 groups as ranges, and the position fit list (built: every player is rated at every position) with your staff's best position and the reason. Physical and his position's own group show in full. A group panel shows the rest, and its dropdown picks the group: the 5 mental attributes that matter most at his position (the default), all 12 mental, running, passing, line play, defense, or kicking and snapping (Cain, 2026-10-20: "make a dropdown menu so that the other stats show too"). A group your staff hasn't seen shows each attribute as "Not seen". An old read says where it came from, for example "from his high school film, fall 2022".
3. **Reports:** every report on him, dated, by whom, with each grade, the character read and the medical read.
4. **Stats:** every season at every level, every stat, with game logs, splits and advanced stats.
5. **Career:** the timeline from age 14: schools, recruiting, offers and visits, transfers, the draft, contracts, injuries, awards, with the "What happened" line for each season.
6. **Tendencies:** the 15 tendencies as ranges with their words, and your instructions to him.
7. **Contract** (pros) or **Recruiting** (recruits) or **School** (high school): what applies to his level.
8. **History with you** (when he's on "my players"): when you first saw him, your grades then, what the public said, every talk.

**Your own notes** (Cain, 2026-10-20, on the mock: "Need a notes button so i can manually write notes"). A Notes button sits in the hero's actions on every player's page, with the number of notes you have on him. It opens a panel at the side, so his attributes stay readable while you write. You type plain text and save it; the note takes the game's date and week, and the list runs newest first. You can edit or delete any note. Notes are private and only for you: your staff and the AI never read them, and they change nothing in the world. They belong to your save and follow the person, so a note on a high school junior is still on his page when he is an NFL veteran or a coach. Also proposed, not confirmed: the same Notes button on coaches, staff, teams, schools and games; a small note mark by his name in every table, on boards and on the depth chart; the "has my note" filter chip (topic 3) and the search through notes (topic 1); and his notes shown on the "History with you" timeline.

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
- **Roster** (Cain's two comments, topics 2 and 3): a data screen in the warm room, the class and unit quick filters, a Columns menu kept per view, and six views of the same players: Overview (body, level, role and outlook in your staff's words, games, snaps, season line, this week's health), Ratings (athletic ranges, football IQ, the position's key skill, best fit), Season (games, snaps, the season line and each of the last three games), Eligibility and money (eligibility, redshirt, academics, revenue share, NIL, hometown, high school, public stars), Health and mood (availability, injury, practice, load, and mood with its reason in words, never a meter) and Draft (what NFL scouts say about your own players, the advisory committee and all-star invites). Proposed, not confirmed: the six views and their columns.
- **Depth chart:** drawn the way a real staff's depth board is, as the formation on the field: each spot with its first, second and third player stacked, ranges and status beside each name, dragged to reorder. A list view sits beside it. Each position coach's proposal shows as a ghost name with his reason, which you accept or ignore. Package depth (nickel, dime, goal line, special teams units) are their own boards.
- **Game plan:** laid out like a real call sheet: situations down the side (opening script, 1st and 10, 3rd and short/medium/long, red zone, two-minute, coming out), plays in each box with their install level. Scouting of the opponent fills a panel beside it (section 9 designs the opponent report).
- **Play designer:** a top-down field with the formation, routes drawn as paths with their branches, blocking arrows and rush paths, in the coaching-diagram style (circles for offense, Xs or Vs for defense), with legality flags (plays and tactics topic 8).
- **Practice:** a calendar of the week by day and period, each period a block coloured by type (individual, group, team, special teams, walkthrough) with its contact level; click a period to set it or watch it (topic 12).
- **Cap (NFL):** the cap sheet as a table by year (cap hit, dead money, cash, guarantees, void years), with the term-by-term negotiation board as its own screen.
- **Recruiting (college):** a poster shelf of your top targets, each poster lit by his high school's own colours with your rank as the outlined number behind it, the selected recruit's file beside it, and the full board as a dense table with each recruit's status, interest in your program in your staff's words, visits and offers; a map of the country by state and town, with your recruiting areas, your coaches' territories and the recruits on it. A switch turns the shelf into the **board** (scouting topic 15): the class plan by position (spots, commits, open spots) over a warm table of every target and commit in four groups: the recruit (home, size measured and listed), the read (your staff's grade and how sure, the public's stars), the file (his evaluator, his area recruiter, the last look and who took it, a stale flag) and the race (interest, offers and who leads, status, next step). The hero counts the staff's fall evaluation days. Proposed, not confirmed: a file is stale when nobody has seen him in person since 1 September.
- **A scouting file (every seat).** Every player your staff has looked at has a file screen, the deepest page in the game (scouting topics 1 and 4-8). The hero sits in his team's colours (a recruit's high school's) with measured against listed size, the public's view (each service's stars and rank, and the composite) set apart from your staff's read in words, his offers and who leads, the actions, and when the file was last updated and by whom. Seven tabs:
  - **Summary:** the grade in role words, floor and ceiling, how sure and exactly why (what has and hasn't been seen), the hints with who found each, a comparison from the world's own history, where your read and the public's differ, who on your staff disagrees, and what is stale.
  - **Ratings:** every attribute that matters at his position as a range, with the kinds of look behind it (film, live, camp, interview) and an arrow where it moved; a click shows how it narrowed (topic 4).
  - **Looks:** the evidence log, every look in date order: who, what kind, where, what it showed, what it narrowed, how long the report took. The rules that limit the looks sit on top. For a recruit in 2026: 33 fall evaluation days for the whole staff (one coach out for one day is one day), only the head coach and 10 assistants off campus, personnel staff on film, calls and campus only.
  - **Tendencies:** how often he tries things (`tendencies.md`), each as a range plus a word with whose opinion it is, the charted number behind it, how it fits your offense, and how the reads moved between his junior and senior film.
  - **Character:** the 13 traits as words with how sure; the notes in detail, each with its source and how that source leans (his coach runs warm, the trainer runs true, a parent defends); who decides in the family; his record and social media.
  - **Medical and academics:** the medical read and what wasn't examined; core courses and core GPA through the NCAA Eligibility Center, with your academic staff's read.
  - **History:** his path since 14, his season stats, the public's grades over time next to your staff's, and his offers in the order they came.
  Proposed, not confirmed: the chips for the kinds of look, the arrow's rule, the reader-by-reader panel, the stale strip, and an "Ask for a new look" button that lists only the looks the rules allow right now.
- **Scouting (college seat):** the department at work, in five tabs (scouting topics 2, 3, 8 and 14):
  - **Staff and areas:** a tile map of the country with each coach's recruiting area and the targets per state, and a table of the 11 off-campus evaluators and the personnel staff: evaluation days used, live looks, film, files written, the current assignment, and what his record says in words from the program's look-backs (a coach new to the program has no record yet).
  - **Assignments and calendar:** the fall evaluation days as 33 squares (used, planned, free); who goes where to see whom on which day, and why; the recruiting coordinator's ranked asks for the free days, which you give out; the coverage gaps ("Southeast Texas: 3 targets, no live look this fall"); and your standing instructions with each person's answer in words ("follows it", or "pushes back" with his reason).
  - **The year:** the real recruiting calendar from June camps to National Signing Day, today marked, rule changes that are pending shown as news.
  - **Look-back:** the class signed four years ago, signee by signee and by evaluator: what each was graded then, what he became, the reason when a miss had one, and the times your staff was on a player first.
  - **Public services:** each service as its own desk with its lean and track record in words, and where your board is out on a limb against them.
  Proposed, not confirmed: made-up names for the services (the real ones are trademarks), giving out the free days one by one, and the look-back's verdict words (held up, too high, too low, no fair read).
- **Draft board (NFL seat):** the war room's board as real clubs keep it (scouting topics 9 and 10), in three views. The vertical board is your order in tiers: each player's grade as a role word with a round, the character and medical lines, how sure, who on your staff disagrees, the public rank with an out-on-a-limb mark, a top-30 visit mark and scheme fit, and a line where each of your picks falls saying how many players above it are likely gone. The horizontal board shows cards by position, coloured by round grade, so you see where the class is deep or thin. The lists hold the do-not-draft list (each reason in the words of whoever took him off), the priority free agents (who calls each, his path to the 53, the bonus and guarantee), the top-30 visits, local day, private workouts and interviews used, and the tests that moved a grade. Beside the board is the selected prospect's file in five tabs: every report by its author and job with his track record, the ranges, combine and pro day side by side, character and medical with his interview, and the meeting, where each person argues his tier and your call reorders the board. Proposed, not confirmed: the tier names and the out-on-a-limb rule.

- **Your school and feeder area (high school):** your school's page (enrollment, classification, principal and athletic director, boosters, facilities) and a map of the town with the feeder middle schools and youth teams, and the kids coming up.

---

## Topic 8. Inbox, news, talks and press

### What exists
- **Decided:** medium events in the inbox, big ones their own screen, huge ones the full show; small ones news lines with filters (calendar topic 8, section 8 topic 4). Talks as picked moves and tones with written answers and tells over 2-5 exchanges (people topic 5); every line from the huge line bank (people topic 5). News by era with how sure each report is (people topic 9).
- **New:** the layout.

### The design
- **Home** is a dashboard of your job: the next seven days, your boss's latest word, the team in words (form, injuries, mood in your staff's words), and the news that concerns you. Cain reshaped three parts of it on the mock (2026-10-20):
  - **The hero turns through slides** ("make this box a revolving slide, it slides into different aspects, like a shortened scoutreport or anything. Recent aspects"). The slides are this week (record, standing, the next game with its countdown), a short opponent report, the last game with the plays that decided it, a player of the week, and the recruiting story of the week. Each slide is lit by its subject's colours. Each one stays about 8 seconds, the slides hold while the pointer is on them, and the dots and arrows move them by hand. With reduced motion they stay put. Proposed, not confirmed: the slides come from what happened since you last looked, so a quiet week shows fewer.
  - **Tracked players** take the inbox's place ("try to Make this tracked player results section"). This is the "my players" list (scouting part E) with each player's latest result: a recruit's Friday game, your own players' Saturday, an injured player's status, and later your former players in the NFL. Each result shows the stat line, who on your staff saw it with their opinion, and what comes next. It can be filtered to recruits or your roster. The inbox keeps its own screen, one click away in the sidebar.
  - **Storylines** keep their posters ("Keep the structure but changes the Graphics into somethimg more immerse"). In place of a big number, each story gets a drawn scene: an X-ray for an injury, Friday night lights for a recruit, a badge for an honor, a snaps chart for a player who wants more time. Proposed, not confirmed: there is one drawing per kind of story, filled with that story's numbers and colours.
- **The inbox** is a two-pane list: messages on the left (unread bold, pinned on top, filter by type), the message on the right with its actions as buttons (open the report, answer the agent, hand it to staff).
- **A talk** opens as its own screen in a calm room in your team's colours: him on the left (face, mood in words, what your staff thinks he wants), the conversation in the middle, your moves and tones as a list on the right. His tells show inside his lines ("looks at the floor", "brings up his agent"), never as a meter. From the round 3 mock, proposed, not confirmed: his side has four tabs (Now: his mood in words with dated reasons, his tells, what he wants; File; With you: the promises ledger and your history; In his ear: family, teammates and outside voices, each with how sure), and a move takes two steps: pick it with a tone, read what your staff expects and the risk (a weak staff can be confidently wrong, people topic 5), then say it. The tones are people topic 5's.
- **Press conferences** use the same screen with the reporters as the other side.
- **News** is a feed by level and era: your beat, national, your recruits' feeds, fan reaction, each report marked with its certainty ("per sources", "has agreed"). A story opens its chain: first report, who repeated it, the response.

---

## Topic 9. The big-moment screens

### What exists
- **Decided:** big moments open their own screen, huge ones the full show: the war room on draft night, cutdown day hour by hour, the deadline's last hours, signing day hour by hour, the playoff bracket (section 8 topic 4); the live draft and the undrafted rush (scouting topics 12 and 13).
- **New:** how they look.

### The design
**Two kinds of screen.** A big moment is a live studio: a room in the colours of whoever is up, a NowCard with the clock, a live feed and glass panels. A huge one ends in Showtime's Moment, the full-screen reveal: a kicker that tightens, the outlined number, the name wiping in, then the verdict. Everything else in the game stays calm so these land.

**The reveals,** each one only when it really happens:
- A recruit's commitment or signing: the hat ceremony (his finalists' caps on the table), then "He's a Cyclone", his high school number and your staff's verdict in words. A miss gets its own quiet reveal ("He's a Cornhusker").
- Your draft pick: "The pick is in", the pick number, the name, your staff's verdict. Other clubs' picks run in the feed, not as reveals.
- Your hire and your firing; a title; a record falling (the old holder named); a Hall of Fame induction; a player you scouted at 15 making his first NFL start.
- The verdict is always words or a short token (a role tier, "QB1", "Steal"), never a potential number.

**The live studios:**
- **The war room:** your board on the left (your order, tiers, who's gone greyed out), the pick clock as a NowCard and the live pick feed in the middle, trade calls on the right as Sheets with the two-sided ledger, with your staff's voices ("our guy is still there", "Philadelphia is calling about 14"). The room takes the colours of the club on the clock. The clock runs in real minutes you can speed up or pause. After the last pick the undrafted rush opens in the same screen: a list of who's left and your offers.
- **Signing day:** the hour-by-hour feed of signings with your board's targets lit, the announcements you're waiting on as NowCards with their start times, your class's progress, and your calls. In the round 3 mock: the day from the first e-signed agreements at 7 am (the National Letter of Intent ended in October 2024; recruits now sign an athletics aid agreement) to the night, flips and losses on your board, the class ranks moving by service and in the conference, the class plan by position filling, every signee's file a click away, and the 105-man roster math with the walk-ons.
- **Cutdown and the deadline:** the clock to the deadline as a NowCard, your roster count against the limit, the waiver wire and trade offers as they come.
- **Playoffs:** the bracket for every level (state playoffs by classification, the College Football Playoff, the NFL), every game a link.

---

## Topic 10. Stats, game pages and history

### What exists
- **Decided:** every stat at every level, kept forever (Q47); game pages with the box score, a drive chart, the win-probability graph and the play-by-play with its why; leaderboards, custom searches, comparisons, career pages (match engine topic 8.8); Hall of Fame, awards and record books (Q48, built PR #42).
- **New:** the layout.

### The design
- **A game page:** a scoreboard hero in a room lit by both teams' colours, the score by quarter, the win-probability line across the game with the big swings marked, then tabs. In the round 3 mock, all built from one play log so every tab agrees: Key plays (the plays that decided it, each with its call, its why and its numbers), Box score (offense with clean and pressured splits, targets, air yards, yards after the catch and separation; defense with pressures; special teams with the wind; team), Advanced (success rate, EPA a play, explosive plays and pressure rate by half, an EPA ledger, every fourth down weighed against the model, and the player groups in each coach's words), Drive chart, Play-by-play (every snap with both calls, the pre-snap look and why: "Cover 3, the deep third left open by the corner's jump"), Snaps (by personnel and package, and the rotations), Charts (a pass chart by depth and side, runs by gap, EPA by drive) and Watch (topic 12). Proposed, not confirmed: the win probability and expected points models, the success rule (40% of the distance on 1st down, 60% on 2nd, all of it on 3rd and 4th) and which plays count as key plays.
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
- **Watch mode is full screen:** the sidebar goes away, the field sits in a room lit by both teams' colours, and the situation bar, playback bar, play-by-play line and side panels are glass over it.
- **The field from above,** drawn to scale with real markings by level (high school, college and NFL hash marks differ: NFL hashes are 18 ft 6 in apart, college 40 ft, high school 53 ft 4 in), the end zones in the home team's colours and name, the line of scrimmage and the line to gain.
- **Players are numbered dots** in team colours (choice 4), the way FM's 2D and Next Gen Stats draw them. The ball carrier gets a ring, the ball its own small marker with its height shown by size while in the air.
- **The camera** follows the ball at a sideline height that shows all 22 by default (the coaches' All-22 view), with a tight view on the ball and an end-zone view, and free zoom.
- **Playback:** play, pause, step a tenth of a second at a time, slow motion, replay the play, next and previous play. The engine's ten frames a second are smoothed to the screen's 60.
- **Next Gen tiles** beside the field update live while a play runs: the target's separation, the ball carrier's speed, time to throw.
- **Overlays** you turn on: route and run paths as trails, each receiver's separation from his nearest defender, speed of the ball carrier, time to throw and the pocket clock, the pass rush's time to pressure, the coverage shell before the snap. Names on hover. The round 3 mock adds the coverage shown before the snap against the one played after it ("Shown: two-high", "Played: Cover 3 sky"), the quarterback's eyes and his target as the read moves, and each defender's job. A breakdown panel steps through the read second by second, with a tracking table and both calls (proposed, not confirmed: the steps and their times).
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
- **Glass and rooms cost the graphics chip.** Frosted glass (`backdrop-filter`) and the room's large blurred lights are drawn by the GPU on every frame they change. On the M1 that is cheap; on the Intel Mac's integrated graphics many glass layers over a moving room can drop frames. So the rooms' drift pauses while days are running and when the window is in the background, glass stays on the screens Showtime gives it to (overview and moment screens, not dense tables), and the build measures frame times on both Macs.
- **Targets, proposed, not confirmed, measured on the M1 in the build:** a screen opens in under 100 ms; sorting a 90-man roster is instant; a leaderboard of every high school player sorts in under half a second; the screens stay under 300 MB.

---

## Topic 15. Comfort and accessibility

### The design
- **Text size** scales from 90% to 130% in settings without breaking the tables.
- **Colour** never carries meaning alone: a range is read by its numbers as well as its bar, results in charts also differ in shape (a filled dot for a completion, a hollow one for an incompletion, a star for a touchdown).
- **Keyboard:** every screen works by keyboard; Space is Continue (FM's habit); ⌘K searches; ⌘[ and ⌘] go back and forward; arrow keys move through tables.
- **Reduced motion** follows the Mac's setting, as Showtime already does: no rise-ins, no drifting rooms, no reveal animation (the reveal shows at once); the 2D view still plays.
- **Light theme** in settings (Showtime's opt-in), for daytime play. Heroes, rooms and moments stay dark in both, as Showtime defines.

---

## Topic 16. What section 10 builds and when

### What exists
- **Decided:** playable early with plain screens, then depth and polish while playing (Q50, option A). Section 8 builds plain, dense text screens with no art; section 10 brings the finish and the 2D view (section 8 topic 5).

### The design
- **Showtime, the frame, the table component and the range meters** (topics 1-4) are the base every screen uses. On the pick (choice 5), section 8's screens are built on them from the start, so the first playable already looks like this and nothing is restyled later. Section 8's "plain screens, no art" then means no faces, no 2D view and no charts yet, not a grey look.
- **Section 10's own build** then adds what the first playable leaves out: the charts (topic 11), the 2D view (topic 12), faces in the screens (topic 13), the reveals and the full shows' finish (topic 9), the maps (topic 7), and the play designer's drawing (topic 7).
- **Section 9** designs and builds game day control into the panel the 2D view keeps for it.

---

## Choices for Cain

Round 1 was posted in the thread on 2026-10-20 with Claude's picks. Cain answered with his Showtime design system (11:08), which settles two of them. The doc is written on the picks for the rest until Cain answers.

1. **How you move around the game.** A left sidebar in Showtime's chrome with everything one click away, like FM24 (pick), or Showtime's top bar with section tabs and no sidebar. Proposed, not confirmed.
2. ~~The default theme.~~ **Settled by Showtime:** dark is the main theme, light is opt-in (Cain, 2026-10-20 11:08).
3. ~~Your team's colours on the screens.~~ **Settled by Showtime:** the subject's colours light every screen (Cain, 2026-10-20 11:08).
4. **Players in the 2D view.** Numbered dots like FM's 2D and Next Gen Stats (pick), or small top-down figures with helmets. Proposed, not confirmed.
5. **Section 8 builds its screens on Showtime from the start** (pick), or plain screens first and Showtime in section 10. Proposed, not confirmed.
