# First play: design notes

Cain's notes from playing the first build (section 8, PR #75, the Mac app with the 30-year world `afm/worlds/world-2026-seed2026/`).
Thread "First play design notes". Started 2026-10-21.

Each note gets a stable number. A note with several aspects is split into separate items. Each item says:
- the date and the screen it is about,
- its status: already decided, planned but not built (with the doc that covers it), plain bug (sent to the section 8 build thread), or open (waiting on Cain's yes or no).

Not built on purpose in the first build (`docs/game.md`): choice screens where the AI still decides, most of 6b training, HS JV squads, faces, the 2D view.
Screen-by-screen list of what's designed but not there yet (Cain asked 2026-10-21 15:23, checked against main after PR #81): the doc "AFM first build: what's not there yet", https://claude.ai/artifact/9Z9kNGBk3EkhbAF1CQ9tbq

## Notes

### Note A (2026-10-21 07:09, opening screen)
Cain: "Opening screen feels so cheep, i need it to feel like a proper opening screen like a actual game, the color is cheap feels like 1980. The opening screen should be similar to FM27, The showtime aspect is not needed there."

1. **The opening screen should feel like a real game's title screen.** Today it is a working form: a headline, the world library list, a path box and the saves list on one page (`app/ui/app.js`, `screens.start`). Open: no design doc covers a title screen; `docs/design/screens.md` has no topic for it. Proposal posted in the thread.
2. **Its colours read cheap and dated ("1980").** Open, part of the same redesign.
3. **Model it on FM27's opening screen.** Open: full-screen atmosphere, the logo, a short menu (Continue, New career, Load, Settings, Quit), the last save up front.
4. **No Showtime on the opening screen.** A change to a decision: section 10 choice 5 (2026-10-20 19:07, `docs/handoff/decisions.md` line 348) says section 8 builds its screens on Showtime from day one. This would make the opening screen the one exception. Waits for Cain's yes. Updated 09:21: the question now covers the whole pre-game flow (opening, new career, load) in one FM-style look, with Showtime starting when you take the job (see 5f).

Picture for items 1-3 (09:21): Cain asked for a Nano Banana prompt, so the opening screen's stadium art is AI-generated and Cain makes it. Prompt below, under "Prompts".
First picture (12:06): Cain's Nano Banana result, saved as `art/opening-stadium-1.jpg` (2752 by 1536). Fits the layout: the left third is dark for the logo and menu. Small things a build would handle: faint yard numbers, a made-up midfield logo and sideline boards, all unreadable at screen size; a 16:10 Mac screen crops a thin strip top and bottom.

### Note B (2026-10-21 09:21, new career setup, "Who are you, coach?")
Cain, with a screenshot of the You and Road into coaching panels: "I'm not happy with those screen neither, the screenshot, It looks cheap and childish. Only a part of the screen is used, the sides are empty which is weird for a page where i need to scroll and it's a sign of part design."

5. **The new-career screen looks cheap and childish.** Open. Why it reads that way: Showtime's TV-app parts (the team-colour room glow, big rounded glass panels, pill chips, a 54 px quiz-style headline) used for a long data form; 15 identical sliders at 9.5 that never say what a number means; native dropdowns. The setup's content is built (first-playable.md topic 2: you, background, ratings pool, style, job); only the look and layout are new. Proposal in the thread, each a yes or no:
   - 5a. FM's create-a-manager flow: five steps across the top (You, Background, Ratings, Style, Job), one full-window page each, no long scroll, Back and Next bottom right.
   - 5b. Ratings as a dense FM table: grouped, a coloured 1-20 number cell, minus and plus buttons, points left at the top.
   - 5c. A live side panel: your reputation in words, how boosters, media and recruits at a big program would greet you (people topic 15), and three coaches in the world with a profile like yours.
   - 5d. Backgrounds as a list that says what each gives (points, reputation, who already knows you), not pills.
   - 5e. The job step as FM's club picker: a full-width table (team, level, conference, last season, budget, expectations, the boss) with filters, the chosen job's file beside it.
   - 5f. (Item 4's scope) the opening, new-career and load screens share one FM-style look: dark graphite, one accent, no team glow. Showtime starts when you take the job.
6. **Only the middle of the window is used on a page that scrolls.** Built short of the spec, not a new design: `docs/design/screens.md` topic 1 says the screen fills the window (laid out for 1440 by 900 points). In-game screens do (`.view > .pad`); the start and setup screens were built as a 1100 px centred column (`app/ui/app.css`, `.setup`). Doesn't block play; fixed with item 5's redesign.

### Mock-ups (2026-10-21 12:33, Cain: "give me mock ups")
Canvas: https://claude.ai/artifact/FrYwnUMKnBs1L31EEYPude (8 screens at 1440 by 900: opening, load game, new career steps 1-6). Clickable: the menu, steps, Back and Next move between screens; backgrounds, ratings minus and plus, style scales, jobs and saves respond. One FM-style look (graphite, one floodlight amber accent, Archivo, the stadium picture behind each page title). Job step and world story use trial 10's real data (Big 12 records, stadiums, NIL budgets, coaches, athletic directors, Iowa State's program grades).
- The flow has six steps, not five: `first-playable.md` topic 2 puts the world and the name toggle first.
- **Confirmed 2026-10-21 14:58** (Cain: "I like this way more, go ahead and implement it"): items 1-6 and 5a-5f, item 4 included (the opening, load and new-career screens leave Showtime; Showtime starts when you take the job). Built by the "Section 8 build: first playable game" thread, handed over with the mock, the art and items 7-8.

7. **The setup's ratings miss four designed ratings.** Found while drawing the mock: people topic 15 lists play calling, in-game adaptation, evaluation and negotiating among your ratings; the built setup (`crates/afm-game/src/setup.rs`) has only the other 15. Planned, not built. The mock shows all 19 with a "not built yet" tag and a pool of 220 (proposed, not confirmed). No question: the design already covers it.
8. **UCF's stadium holds 10,000 in the world** (trial 10 raw `programs`, real about 45,000). A data bug, doesn't block play; logged for the next fix list.

### Note C (2026-10-21 15:21, Home, South Carolina, Mon 9 Feb 2026; screenshot `shots/09-home.png`)
Cain: "new issues, please list these. issue 1: Home page. it feels empty, i need something to fill this up, didn't we discuss a live slides that changes ?"

9. **Home feels empty.** Already designed, not built: `docs/design/screens.md`, Home (Cain's comments on the mock, 2026-10-20). Section 8 built plain screens on purpose; section 10 brings the finish (screens.md, build order). Today Home has a small hero, an empty Inbox, an empty My players and Around football. What the design puts there:
   - 9a. The hero turns through slides, about 8 seconds each: this week (record, standing, next game and countdown), a short opponent report, the last game with the plays that decided it, a player of the week, the recruiting story of the week. Each lit by its subject's colours; dots and arrows; holds under the pointer.
   - 9b. Tracked players take the inbox's place: each player's latest result with the stat line, who on your staff saw it and what comes next.
   - 9c. Storylines as posters with a drawn scene per kind of story.
   - 9d. The next seven days, your boss's latest word, and the team in words (form, injuries, mood).
   - 9e. Spotted in the screenshot: Around football is mostly small-college position coach hires. The design says Home shows the news that concerns you; that filter isn't built.

### Note D (2026-10-21 15:52, Inbox, South Carolina; screenshot `shots/10-inbox.png`)
Cain: "next list of things - don't build these immeidately jsut list them so i can execute later. I do want a mock screen though. 1. Messages should look more... official not just like text in a box. Messages. They come in unread and highlighted/bolded and turn regular when i open them. When i open them, a 3d mockmail screen should show up and display the entire message. Messages from the last 30 days should stay stored. On top there should be a filter, ALL -> READ -> Unread. With an button for SAVED MAILS & one for Notes."
Not to be built until Cain says so. Mock: https://claude.ai/artifact/GVwr6ozYJjK7LMT7EotUC8 (one clickable screen, 13 mails from 9 Feb to 10 Mar 2026, South Carolina's real players and assistants from the 30-year world; the off-field staff, agent and reporter names are made up for the mock).

10. **The inbox should feel like real mail.** Today it is one list of one-line items (`app/ui/app.js`, `screens.inbox`, `itemRow`), with no read state, no opening and no answers. `docs/design/screens.md` topic 8 designs a two-pane inbox; the points below go further. Open, waiting on Cain's yes or no, except where marked.
   - 10a. **An official look.** Each mail is a document from a real sender: a letterhead (the university, the SEC office, an NFL club, an agency, a newspaper masthead, a player's plain email), From, To, Date and Subject, the full text, tables where the content is a list (injuries, the spring plan, the cap), attachments and a signature. New.
   - 10b. **Unread mail is bold and highlighted and turns regular once opened.** Already decided, not built: screens.md topic 8 ("unread bold"). The mock adds a dot, a team-colour bar and an unread count on the sidebar's Inbox.
   - 10c. **Opening a mail shows a 3D letter with the whole message.** A change to a decision: screens.md topic 8 puts the message in the right pane of a two-pane list. Cain's version: a full-screen letter that rises out of a small stack of paper, over a blurred inbox. Beside it, the answers and actions. Newer and Older move through the mails without closing it.
   - 10d. **Keep mail for 30 days.** New. Today the screen asks for 120 days. Mock proposal: a mail clears on the morning it turns 31 days old, the row shows "Clears in 10 d" for the last ten days, and saved mails stay until you delete them.
   - 10e. **A filter on top: All, Read, Unread.** New. The design's filter by type stays as a row of sender chips below it (Your boss, Staff, Players, Compliance, League and NFL, Media, Agents), with counts.
   - 10f. **A Saved mails button.** New. It replaces the design's "pinned on top": a star on each row and in the letter saves a mail, and the button shows only the saved ones.
   - 10g. **A Notes button.** Partly built: notes exist on each player's page today (write, delete, shown on Home under My players). New: all your notes in one panel from the inbox, including notes that aren't about one player.
   - 10h. **Spotted in the screenshot: the inbox is full of conference realignment news.** Same gap as 9e: the design says the inbox holds what concerns you, and that filter isn't built. Builds on Cain's answer in the Game bug check thread (2026-10-21 15:47, bug 1): FBS conference moves stay in the inbox; FCS, lower-division and high school moves show only when you coach at that level. Mock proposal for the rest: other league news (hires at other schools and the like) goes to News, and news about your team and your rivals also comes once a morning as clips from your communications staff. Reworded 16:10 after that answer.
   - 10i. **Answers are picked moves, with an answer-by date.** Already decided: talks are picked moves and tones with a huge line bank that covers all text (`docs/design/people.md` topic 5; `docs/handoff/decisions.md` line 306). Not built in the inbox: the mock shows two or three answers per mail (no tones yet) and what each one did, and "Answer by Fri 13 Mar" in the row until you answer.

Cain's answers (2026-10-21 16:13), not to be built until he says go:
- 10a yes. 10c yes. 10d yes. 10f yes.
- 10e yes, with a change: the filter under All / Read / Unread is by keyword, not by sender. Mock (updated 16:30): keyword chips (Your job, Your players, Recruiting, Portal, Injuries, Money, Spring practice, Draft and NFL, Staff, Your people, Media, League) and a "Filter by keyword" field that searches the subject, the sender and the whole letter. Each row shows its first two keywords. Reading taken: both chips and typing; say if only one was meant.
- 10g yes, and each note logs the page it was written on ("Inbox · <the mail's subject>", "Scout report · <player>", "Player page · <player>", "Depth chart"). Mock: "Add a note" in an open letter opens the Notes panel with that mail as the source.
- 10h yes, but don't strip everything outside your school. Cain: "major news league FCS stand outs from scout for the portal sdhould still show up so that we don't miss a trinidad chambliss, also people i'm linked to (coaches that go off to NFL, players that get drafted. So that i can keep in touch, talk to them and eventually request their help for scouting trips etc.)". With his Game bug check answer (FBS conference moves in the inbox; FCS, lower-division and HS moves only at your level), the inbox keeps:
  - 10h1. FBS conference moves, and the major league news.
  - 10h2. Your scouts' lower-division standouts as portal watch lists (FCS, D2 and below), so a small-school star isn't missed. Scouting.md covers portal evaluation (film, the old staff, the player's whispers, tampering contacts); the watch-list mail is new.
  - 10h3. News about people you're linked to: former players drafted or at the combine, former assistants hired (NFL or college), your old coaches. Mock: a weekly "Your people" memo from your chief of staff, with congratulations or a call as picked answers.
  - 10h4. Keeping in touch and asking favours: a linked person can offer or be asked for help (introductions on a recruiting or scouting trip, an NFL club's read on your draft prospects). The network itself is designed (`docs/design/people.md` topic 2, "You are in it too", and topic 15, "Your network"); news about it in the inbox, keeping in touch and asking favours are new. Mock: a personal email from your old defensive coordinator, now with the Falcons, offering intros to high school coaches around Charlotte for the May evaluation trip.
  The mock's people, apart from South Carolina's players and assistants and the world's lower-division players, are made up.

Cain (2026-10-21 16:41, screenshot of the mock's list): "i like the 3d mail aspect but this main screen looks very confusing and chaotic, nothing like an actual clean mail adress". Two causes: a layout bug in the mock (each row's columns stacked onto four lines) and too much per row (sender badge, role, keyword pills, coloured bars). Redrawn 16:55 as a clean mail client: one line per mail (sender, subject, a grey preview, an answer-by date only when one is due, the time), unread in bold white with a small dot, the star only on hover or when saved, plain day headings. Search sits top right; the keywords moved from 12 chips into one "Keyword" dropdown; Saved mails and Notes are quiet buttons. The letter is unchanged. Proposal for item 10e's look, waiting on Cain.

**Confirmed 2026-10-21 17:08** (Cain: "i'm happy, lock it in for the next build"): item 10 as the mock stands (version 6): 10a-10i with Cain's 16:13 answers (10e keywords with a search box, 10g notes log their page, 10h1-10h4), the clean one-line list from 16:55, and the 3D letter (fixed 16:55: it had opened blank because the paper stack covered it). Queued with the "Section 8 build: first playable game" thread for its next build, after the opening and new-career pages. Updated 17:11 (Cain: "don't build after the start screen"): parked. The build thread does not start it after the opening pages; it waits for Cain's go.

11. **A D3 defensive tackle with 34 sacks** (trial 10 raw `player_seasons`, 2025: Luc Weekly, North Park, a 5-star, 34 sacks and 124 tackles in 13 games). Far above the FBS record (24, Terrell Suggs, 2002), and a 5-star at D3 is rare. A data oddity seen while pulling the portal list; doesn't block play; logged for the next fix list.

## Prompts

### Opening screen stadium (Nano Banana, 2026-10-21)
Cinematic wide photograph of a packed American football stadium at night, seen from high in the upper deck behind one end zone, a few minutes before kickoff. Floodlights blaze down through a light haze, so the beams and the grain of the air are visible. The field is deep green with crisp white yard lines; tiny players warm up near midfield, too far away to make out. The crowd is a dark, textured mass with scattered phone lights and camera flashes. The sky is blue-black with the last trace of dusk at the horizon. Composition: the stadium and field fill the right two-thirds of the frame; the left third falls away into deep shadow (dark stands and night sky), calm and empty enough for a logo and a menu to sit on it. Mood: big, quiet anticipation, like the opening shot of a TV broadcast. Look: full-frame camera, 35 mm lens, f/2.8, natural floodlight colour (cool white with a warm edge), filmic and restrained, slightly desaturated, deep blacks, soft glow around the lights, no heavy HDR, no oversaturation. A fully fictional stadium: no team logos, wordmarks, sponsor boards, scoreboard graphics, or readable text or numbers anywhere, and not a copy of any real stadium. Nobody close to the camera. Landscape 16:9, highest resolution.

Rotation swaps for the first sentence:
- High school: a small-town Texas high school stadium on a Friday night, aluminium bleachers, one press box, four light towers, the band in the stands, pickup trucks beyond the fence and flat land to the horizon.
- College: a huge college bowl of 100,000 on a Saturday night, the student section one solid block of an unbranded colour, the band forming on the field.
- NFL: a modern NFL stadium with its roof open to the night, giant video boards glowing blank, smoke drifting from the tunnel at one end.
