# Section 8: The first playable

The design for section 8 (roadmap: new-game setup with the team choice, the name toggle, your background and editable starting stats; save and load; plain text screens; Raichel starts playing and debugging here). Thread "let's get started planning section 8", started 2026-10-20. Design only, nothing here is built yet.

How this doc works:
- It starts from `docs/design/section8-readiness.md` (what the game stands on, and the 7 things it still needs).
- Each topic says what exists: **Built** is in the code, **Decided** is confirmed design not built yet, **New** is what this section adds.
- Real life is the default (Cain, 2026-10-16). Numbers no source publishes are Claude's picks, marked **proposed, not confirmed**.
- Sections 5, 6 and 7 are settled designs (`docs/design/team-management.md`, `docs/design/people.md`, `docs/design/scouting.md`). This section adds the screens and choices on top of them and re-asks nothing in them.
- Four choices were Cain's (listed at the end); 1 and 4 are confirmed, 2 and 3 wait for him. The doc is written on Claude's picks until he answers.

Topic order: 1 the world you start in, 2 starting your career, 3 save and load, 4 the clock on screen, 5 the first playable's screens, 6 your choices against the AI's, 7 the high school seat, 8 memory and speed on the M1, 9 the app (UI technology), 10 debugging in play, 11 what section 8 builds and what stays later.

---

## Topic 1. The world you start in

### What exists
- **Built:** `World::new` builds the country, runs 6 spin-up years and 30 years of hidden history, and stops at February 2026. The same seed gives the same world on any machine and core count (build 9 test). Trial 9 timed it: the history took 61 minutes on 4 cloud cores with every college and NFL game on the engine and high school on the stand-in; each later season took 102-110 s.
- **Decided:** 30 years of hidden history (2026-10-11). College history on the engine (Raichel, 2026-10-14). Every high school plays full games (Q13). A save starts the day after the Super Bowl (Q41, calendar topic 1). The real/fictional name toggle before a save starts (Q11).
- **New:** how a new game gets its history, and whether the high school history runs on the engine (waiting on Cain, choices 1 and 2).

### The design (on Claude's picks)
**A world library.** The game keeps ready-built worlds, each one a save on the morning after the 2026 Super Bowl with its history done. Starting a career picks a world from the library and loads it, as fast as a save loads. "Build a new world" runs in the background, or overnight, and adds a world to the library when it's done. Any number of careers can start from one world, so a world can be replayed with a different team or background.
- The library shows each world's headline story in words, never its hidden numbers: the reigning champions, the dynasties of its history, the hottest coaching names, the top recruits of the 2026 and 2027 classes. You pick a world the way you'd pick a starting database in FM.
- The first worlds are built before the first playable ships and come with it (a build step, timed).
- A world is built from a seed. Two worlds from the same seed are the same world; different seeds give different worlds, every game result included.
- **Each career gets fresh randomness from its first day** (Cain asked 2026-10-20 whether every world would have the same results). Two careers from the same library world share its history up to February 2026, like two FM saves from one database, and then play out differently. The career's own seed is stored in its save, so it still replays exactly (topic 3).
- A world build on the M1 is a guess of 45-60 minutes for the history as built today, not measured. A fanless MacBook Air slows under long load; the Intel Mac is likely slower. The build times it on Cain's machines before the library ships.

**High school history on the engine for the last 12 years** (choice 2). Every player in college or the NFL in 2026 then has real high school stat lines, which scouting reads (Q9: "stats in highschool should translate to NFL"). Earlier high school years stay on the stand-in, so state record books before 2014 keep team records only. Cost: about 10-12 minutes a season on 4 cores (2,000 high school engine games take 16 s), so about 2.4 more hours per world. All 36 years would add about 7 hours.

**The name toggle** comes before the world loads, because names are fixed with the world: real names for the real NFL and colleges and their people (Q11), or fictional ones for all of them (`NameMode`, built). High school names are generated in both (towns are real, schools are generated, Q12). The toggle also covers JUCO programs (world design, topic 8).

---

## Topic 2. Starting your career

### What exists
- **Built:** `Seat` (level, NFL team, college program) decides which news stops the clock for you; no high school seat yet. Every AI coach's ratings, record and reputation.
- **Decided:** you are head coach, and GM too in the NFL (Q1). Any team (Q3) at any level, one career moving between them (decisions section 12). One realistic mode (Q7). Your background sets your reputation and gives a pool of points to spread across your ratings, like FM (Q8; people topic 15, Cain 2026-10-19). Your ratings grow (Q20). Your profile: name, age, hometown, alma mater, family, looks, and your own photo if you want one (people topics 13 and 15, faces design). Your seven coaching style settings (tendencies design, "Your coaching style"). Your health never ends your career (people topic 13).
- **New:** the order of the setup screens, how you take over a job on day one, and starting without a job.

### The design
**The setup, in order:**
1. The world (library) and the name toggle.
2. You: name, age, hometown, alma mater, family (spouse, children and their ages, where your roots are), looks or your own photo.
3. Your background (people topic 15's roads: former NFL player at four levels of career, former college player, never played past high school, high school coach, college position coach or coordinator, NFL assistant or coordinator, scout or personnel, analytics). It places you in the world's own history: where you played or coached and with whom, so those people know you (people topic 2).
4. Your ratings: the background's pool spread across them, with the background's lean as the starting spread. Proposed, not confirmed: each background's pool size.
5. Your coaching style settings.
6. Your job (below).

**Your job.** Every head coaching job in the world is open to pick: every NFL club, every college program at every division, every high school. You walk in the way a real new hire does in February:
- The coach you replace leaves into the world: fired or retired by the team's own situation, he joins the coaching market like any fired coach (people topic 14) and can be hired elsewhere.
- His staff is under contract and stays until you decide. Keeping or letting people go follows the real rules of your level (people topics 10 and 11): in the NFL you choose within your power structure; in college the doctors, trainers and academic staff are the athletic department's; in high school you recommend and the principal decides, usually tied to a teaching job.
- Your contract is the one a real hire at that job gets, sized like the built pay ranges for your reputation and the job (proposed, not confirmed: the exact sizing). Your boss sets your first goals in a meeting (people topic 12).
- The calendar is real for every level on that February morning. A new college coach arrives after the early and February signing periods with spring ball ahead; a high school coach arrives in the offseason lifting season with spring ball by his state's rule; an NFL head coach arrives with his staff to finish, the combine ahead and free agency in March.
- Your reputation from your background meets the job: a never-played-past-high-school coach walking into a blue blood starts with skeptical boosters, media and recruits (people topic 15). Nothing stops you taking the job.

**Starting without a job.** You can also start unemployed and go through the real market: openings come from the built carousel, teams judge you as they judge every candidate, and interviews use the talk system from the other side (people topic 14). In February most head jobs are filled, so this start usually means an assistant job, a high school job or a wait until the next carousel.

**Assistant jobs.** People topic 14 already lets a fired coach take a coordinator, position or analyst job. In those you control what that real job controls: a coordinator builds his side's game plan and calls its plays, runs his side's practice plan, proposes his side's depth chart to the head coach and recruits his area; a position coach coaches his group, proposes its depth and recruits his area. The head coach (AI) decides everything else and can overrule you. Your aim is the real one: results that put you in line for the next job.

---

## Topic 3. Save and load

### What exists
- **Built:** nothing is serialisable yet (no `serde` in the engine). Each player and each game has its own random stream, so the world is reproducible from a seed.
- **New:** everything here.

### The design
- **The save holds the whole world,** every random stream included, so a loaded save plays on exactly as if it had never been saved. That is what makes a bug reproducible (topic 10).
- **Format:** the world derives `serde`; the save is a compressed binary file (`bincode` and `zstd`), with a version number. When the game changes, an upgrade step turns an older save into the new shape, so a long career survives game updates. A save that can't be upgraded says so plainly and is never silently broken.
- **Slots and autosave,** like FM: save as many named saves as you like; autosave on a schedule you set (every day, week or month of game time, proposed default every week) into its own rotating slots; the game also saves itself before every huge moment (the draft, cutdown day, the trade deadline, signing days, the playoffs). Proposed, not confirmed: three rotating autosaves.
- **Size and speed:** measured in the build. Targets, proposed, not confirmed: a save under 500 MB on disk and under 30 seconds to load or save on the M1, without stopping the screens (saving runs in the background from a copy taken between days).
- **The library worlds (topic 1) are saves too,** marked read-only so a career never overwrites the world it started from.

---

## Topic 4. The clock on screen

### What exists
- **Built:** the daily clock (`advance_day`), Continue (`continue_until_needed`: stops on a decision, a huge moment anywhere, or a big one that concerns your seat), holiday mode (`holiday_to`: stops for every huge moment), loudness for every event (small, medium, big, huge), news, and hour-by-hour running of the biggest days.
- **Decided:** calendar topics 1 and 8 to 10: one day at a time; small events are a news line, medium an inbox message, big a screen of their own, huge the full show; nothing happens instantly (agents answer in days, scouting reports take weeks); handing any task to staff with instructions, and staff deciding like real people with a reason you can read.
- **New:** how it looks and feels while you play.

### The design
- **One Continue button,** always in the same place, with the date and what's next ("Wed 4 Mar: free agency negotiating window opens in 5 days"). It shows a progress line while days run and stops on the day something needs you, saying why it stopped.
- **The inbox** carries medium and big events, each with what it is about and what you can do (open the player, answer the agent, read the report). Small events are news lines you can filter by your team, level, the players you follow and the world.
- **Big moments open their own screen;** huge ones get the full show (the war room on draft night, cutdown day hour by hour, the deadline's last hours, signing day hour by hour for a college coach, the playoff bracket).
- **Things that must be answered** (an offer with a deadline, a player asking to talk, a staff member asking for a yes) stay pinned until you answer or hand them off. If the deadline passes, what happens is what real life does: the offer lapses, the agent goes elsewhere, the staff member decides alone.
- **Holiday mode** with a date, and the instructions your staff follow while you're away.
- **Your own game day** in section 8: the engine plays it from your game plan with your coordinator calling the plays (section 9 brings live calling and in-game decisions). You see the full box score, the drive chart and the play-by-play with the engine's reasons, then the week's next days.

---

## Topic 5. The first playable's screens

### What exists
- **Built:** the trial's player pages (cards) and the Debug Lab show what text screens need, from the world's own data; the game hides true potential (`debug` feature off).
- **Decided:** a dense FM look, plain screens first (Q45, Q50). Every written line comes from section 6's huge line bank (people topic 5).
- **New:** which screens the first playable needs to play a whole year at each level.

### The design
Plain, dense text screens: tables, lists and words, no art yet (section 10 brings the finish and the 2D view). Every name is a link to its page. Each screen shows only what your staff could know (scouting topic 1): ranges and words, never hidden numbers.

**At every level:**
- Home: the inbox, the next days, your boss's latest word, your team's state in words.
- The calendar: your level's year, with your own dates on it.
- Roster and player pages: the cards' layout, with your staff's reads, the player's history, mood and trust in words, his ties (people topics 1 to 3).
- Depth chart, with your position coaches' proposals and their reasons.
- Game plan and tactics: the plays-and-tactics design's game plan, playbook and installs, your style settings.
- Results: schedule, standings, box scores, play-by-play, stats with every stat (Q47), the stat book and record books.
- Staff: who you have, what each does, their work in words, hiring and firing (people topics 10 and 11).
- Talks: one-to-one, unit and team talks as picked moves and tones (people topic 5).
- Team rules and discipline (people topic 8); the media: press conferences (people topic 9).
- Your boss: meetings, goals, what you can ask for (people topic 12); your contract and the job market (people topic 14).
- Your profile: ratings, reputation in words, record, coaching tree (people topic 15).
- Scouting: your staff, assignments, reports and files (scouting topics 1 to 8).
- News and history: the world's news, awards, the Hall of Fame, every season of every team.
- The "my players" list (scouting topic 18).

**NFL seat (GM and head coach):** the cap and contracts, negotiations on the term-by-term board (team management topic 4), free agency, trades, waivers and the practice squad, the draft board, the pre-draft season and the war room, the undrafted rush (scouting topics 9 to 13), your owner.

**College seat:** the recruiting board and your moves (scouting topics 14 to 16), visits and signing days, scholarships and roster limits by era, the portal, NIL and revenue sharing, academics and eligibility, your athletic director, president and boosters.

**High school seat:** topic 7.

**Where the game view's waiting items would sit** (brainstorm items 2-20, still waiting on Cain in the Bird's-eye game view thread; not re-asked here): the career story page under your profile; a player's past shown when you meet him again on his player page and in the talk screen; earned nicknames on player pages and in the news.

---

## Topic 6. Your choices against the AI's

### What exists
- **Built:** every decision your seat needs has an AI side already: depth charts, game plans and play calling, contracts, tags, trades, waivers, free agency, the draft board and draft-day trades, the undrafted rush, recruiting boards and offers, the portal, NIL, staff hiring and firing, captains and the room.
- **Decided:** any task can be handed to a staff member with instructions, and staff decide like real people (calendar topic 10).
- **New:** how your choices plug in.

### The design
- **Where you choose, the AI's choice for your team is skipped.** Your team runs on exactly the same rules, calendar and engine as every AI team. Nothing is easier or harder for you.
- **Where you don't choose, the person whose job it is decides,** by his ratings and habits, and leaves his reason (the built AI decision becomes that staff member's). Each task has a default owner by level (the cap manager runs the cap, the defensive coordinator the defensive game plan, the recruiting coordinator the board), and you can take any task back or hand it on, with instructions ("never trade a first-round pick", "ask me before cutting a starter").
- **Bosses still have their say over you,** as people topic 12 decided: a meddling owner, a booster pushing a player, a principal overruling your discipline.
- **Training until section 6b is designed:** your strength and position coaches run the weekly training as the AI does now (calendar topic 7's slots, the built development step). Section 6b adds your own control.

---

## Topic 7. The high school seat

### What exists
- **Built:** generated high schools in real towns, each state's calendar, classification, playoffs, grade checks and transfer rule, one head coach per school with ability and network, college staffs' practice drop-ins, high school pay ($50-160k), the climb to college on buzz, the principal-and-boosters review (`world/high_school.rs`, `world/hs_year.rs`, `world/staff.rs`). The engine plays high school games under the NFHS rules (12-minute quarters, the Kansas-plan overtime, a running clock in blowouts) and Texas and Massachusetts's NCAA-rules variant (`game/rules.rs`). After trial 8's clock fix a high school team runs 51.3 plays a game in `match_check` (the old 83 is gone), and scores 14.0 points.
- **Decided:** every high school game on the engine (Q13). A high school staff that you hire, many of them teachers (people topic 10). The school's side of the job: principal, athletic director, superintendent, school board, booster club, goals, reviews and non-renewal (people topics 12 and 14). Your side of recruiting as a high school coach: promoting your kids, your credibility with college staffs, hosting, camps, eligibility (scouting topic 17).
- **New:** the high school seat itself, and checking high school game length and scoring against real numbers.

### The design
- **Every high school game on the engine,** world-wide, each season (about 10-12 minutes a season on 4 cores; the build times it on the M1). High school players' stat lines then feed the stat book, the state record books and college scouting.
- **Real game length and scoring.** No public high school play-by-play exists, so the check is real team totals: MaxPreps and state association records for points, plays and yards a game, by state and classification. The build checks the engine against them and logs the gap for the next trial (no tuning before Cain's yes).
- **Your program:** the roster from the school's own enrollment, all four grades, with varsity, JV and freshman teams on big campuses (you decide who plays where), players going both ways, transfers and move-ins by your state's rule, and the freshmen coming up from your feeder middle schools and youth teams.
- **Your staff,** built from people topic 10: assistants who teach at the school for a stipend, outside walk-on coaches where your state allows them, and an athletic trainer only if the school has one.
- **The school's side,** built from people topics 12 and 14: your principal and athletic director as people with goals (wins, grades, numbers coming out, behavior), the booster club's money for equipment, meals and travel, parents who go over your head, and non-renewal of your coaching stipend while you keep teaching.
- **Your teaching job.** Most high school head coaches teach (or, in Texas, run the campus athletics). Your teaching load is part of your hours and your health and burnout (people topic 13), and part of your contract and pay.
- **Getting your kids recruited** is scouting topic 17, built as designed.

---

## Topic 8. Memory and speed on the M1

### What exists
- **Measured:** trial 9 used 5.93 GB after the history and grew about 60 MB a season (peak 7.62 GB), with the trial's game log on, which the game doesn't keep. Each future season took 102-110 s on 4 cores: college games 38 s, the NFL 17 s, summer signing 14 s, NFL people 10 s, high school 9 s.
- **Decided:** Cain's MacBook M1 has 8 GB and he also plays on an Intel Mac (project memory, 2026-10-16 and 17). A 4 GB memory budget for the world (trial 9 finding 12, being built in the thread "Full build, bug check and trial 9"; this section uses what that build sets).
- **New:** the app's share and the speed of a played day.

### The design
- **The world stays inside the 4 GB budget,** for all 50+ years of a career, with high school stat lines added. Old seasons move to compact history (what the record books, player pages and history screens need) so memory doesn't grow without end. The build measures it on the M1.
- **The app's own screens take about 150-300 MB** (topic 9), so the whole game stays under about 4.5 GB and leaves the M1 room for the system.
- **A played day must feel quick.** Proposed, not confirmed, measured on the M1 in the build: a quiet day under half a second; a Friday night with every high school game, the heaviest day of the week, under about a minute with a progress line; Saturday's college games under 15 seconds.

---

## Topic 9. The app (UI technology)

### What exists
- **Decided:** a desktop app over the Rust engine (Q49), for Cain's own computer (Q6). Better visuals later, so the engine keeps full player positions for a 2D or 3D view (2026-10-13).
- **New:** the technology (waiting on Cain, choice 3).

### The design (on Claude's pick)
**Tauri.** The engine runs inside the app in Rust, with no server and no copy of the world. The screens are HTML drawn by the Mac's own WebKit, so the app is about 15 MB and the screens take about 150-300 MB. One build makes a universal app for Apple Silicon and Intel.
- The Debug Lab and the player pages are HTML already, so the first plain screens reuse their tables and layouts.
- Dense sortable tables, filters and right-click menus are what HTML does best. The 2D replays (section 10) fit a canvas; the faces show as images.
- The engine works in a background thread, so the screens never freeze while days run.
- Weighed and passed over: Electron (ships its own Chrome, 300-500 MB more memory, too heavy for 8 GB); egui or Slint (pure Rust and light, but FM-style tables and polish take far more work); SwiftUI (Mac only); Godot or Unity (made for 3D scenes, weak at dense tables, the engine behind a bridge).
- Proposed, not confirmed: a light screen framework (Svelte) over plain HTML, picked in the build.

---

## Topic 10. Debugging in play

### What exists
- **Decided:** Raichel finds bugs in gameplay from here (2026-10-11: "Don't find/search calls/bugs, that's up to me to find out when eventually in gameplay"). Potential stays hidden in the game; exact numbers only in debug tools (Q46).

### The design
- **"Report a bug"** on every screen: you write what looks wrong in a line, and the game saves a bug pack: the save from the start of that day, the screen you were on, the day's events and your note, into a folder you can drop into a thread.
- **Claude replays it exactly.** The world is reproducible (topic 3), so the debug tools load the pack, replay the day and read the hidden numbers behind it, in the Debug Lab, without the game ever showing them to you.
- **The game itself has no debug view.** True potential and hidden ratings never appear on your screens.

---

## Topic 11. What section 8 builds and what stays later

### What exists
- Build 9 split sections 6 and 7 into what it built, what comes with section 8, and later builds (`docs/design/build9-plan.md`).

### The design (confirmed, choice 4)
Section 8 builds everything your own seat touches in a first season at each level, so the first career is the real job:
- Everything build 9 marked "with section 8".
- From build 9's later builds, the parts your seat touches: every assistant coach as a person and high school staffs (people topic 10); assistants' contracts, coaches' agents and permission rules (topic 11); bosses as people and coaches' families and health (topics 12 and 13); reporters and stories for press conferences (topic 9); friends, mentors and family ties (topic 2); team rules for AI teams (topic 8); area recruiters and camps as looks, character and medical reads at full detail, potential words, look-backs, pro scouting, the portal and JUCO, the high school coach's side and the "my players" list (scouting).
- Still later: other leagues' games on the engine (the CFL's own rules), the supplemental draft, prospects' own pre-draft choices, smokescreens and mocks, quarterback-receiver chemistry in the engine, and social media and NIL from followers.

---

## Choices waiting on Cain

Posted in the thread on 2026-10-20 with Claude's picks. The doc is written on the picks until Cain answers.

1. **How a new game gets its history.** A: a world library with background builds; B: build on your Mac every time (about 45-60 minutes, a guess); C: the quick stand-in for the history. **Confirmed: A** (Cain, 2026-10-20 08:51).
2. **How many years of high school history run on the engine.** All 36 (about 7 more hours per world), the last 12 (about 2.4 hours; every player in college or the NFL in 2026 has real high school lines), or none. Claude's pick: the last 12.
3. **The UI technology.** Claude's pick: Tauri.
4. **What section 8 builds.** Everything your seat touches in a first season, as in topic 11. **Confirmed: yes** (Cain, 2026-10-20 08:51).
