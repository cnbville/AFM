# AFM as a game: the bird's-eye view

Owned by the thread "Bird's-eye game view". It judges AFM as a game for the person playing it, next to how real the simulation is. It never trades realism for fun: real life stays the default, and the questions here are only about what real life does not settle (what the player sees, how often, how fast, and how they learn). It judges and proposes; the build threads own the code and the specs.

First pass: 2026-10-17, on `main` at cc519b1 (full build steps 2-5 in progress on their own branch).

## The lenses

Every section is judged on six things:
1. **Decisions:** what the player decides, and whether it changes outcomes enough to feel (but never beats talent).
2. **Feedback:** after a decision, can the player find out why it worked or failed?
3. **Pacing and tedium:** does it still work in season 30, not just season 1?
4. **Hidden information:** how potential, attributes and other teams' intentions reach the player.
5. **Challenge and progression:** with one realistic mode (Q7), where the challenge and the long-term goals come from.
6. **Speed on the 8 GB M1:** how long Continue takes, and memory.

## The map

| Section | State | Where the player touches it | Game judgement |
|---|---|---|---|
| 1 Player model | Built | Player growth, slumps, breakouts | Strong. Hidden, dynamic potential is the FM core loop. Gap: no in-game "why he grew" yet (6b). |
| 2 World | Built | Recruits, college careers, coaches, history | Strong as a backdrop. The player meets it mostly through scouting (7) and news. |
| 3 Match engine | Built | Watching games, live calls, stats | Strong base: every play keeps its why. Risk: the coach-call reward band (plays topic 6.3) is still unresearched, and it decides whether game day feels worth playing. Full build step 5 is calibrating calling now. |
| 4 Calendar | Built | Continue, holiday mode, daily decisions, loud and quiet moments | Strong. The Small/Medium/Big/Huge ladder is the right shape. Risks: no speed target for Continue, and daily decisions times 17 weeks times 50 years needs a light report-back path. |
| Plays and tactics | Designed, building | Game plans, install limit, tendencies, play designer | Very strong as a game: the install trade-off, tendency tells and opponents adapting are real decisions with real feedback. |
| 5 Team management | Designed | Negotiations, free agency, trades, AI GMs | Very strong: term board, tells, fine-print traps and the perfect-offer rule are playable mechanics, not a mood meter. Risk: a lot of screens per deal; staff summaries matter. |
| 6 People | Not designed | Morale, locker room, staff, owner, your profile | The biggest open game area. Owner goals and job security are where the challenge comes from in one realistic mode. |
| 6b Training | Designed 2026-10-20 (`docs/design/training.md`) | Weekly focus, individual plans | Needs a feedback path: staff progress reports with an explained why, accurate by staff skill. |
| 7 Scouting and draft | Not designed | Reports, hints, draft board, the draft | The heart of the hidden-information game. Needs a way to learn from your own past reads. |
| 8 First playable | Built 2026-10-21 (PR #75, `docs/game.md`) | New game, saves, plain screens | Comes after 5, 6, 6b and 7, so Cain feels nothing as a player until then. |
| 9 Game day | Partly designed (section 3 topic 9, plays topic 6) | Live calling, in-game decisions | Risk: accept or reject on every snap the coordinator calls is about 130 prompts a game against a 15-minute game. |
| 10 Screens and 2D | Designed 2026-10-20 (`docs/design/screens.md`, Showtime design system) | Every screen, cards, history | Depends on the desktop UI tech (still open), which must run on both Macs. |
| 11 Later | Parked | Press, player talks, finances, college career | Fine to wait. |

## Speed measurement (2026-10-17)

Measured on 4 cloud cores, 16 GB, `main` at cc519b1:
- `world_trial 2026 2` (the world plus 2 history seasons, college games on the engine, high school on the stand-in): 252 s wall, 673 s CPU, **peak memory 4.0 GB**. A 30-year history holds more people, so its peak is likely higher (inferred, not measured).
- `match_check hs 400`: 2.6 s for 400 games, so about 6.5 ms a game on 4 cores. `fbs 400`: 2.4 s.
- The world has about 8,300 high school programs (8,011 small plus 255 powerhouses in the coach line of the report), so about 4,100 high school games each Friday. On the engine that is about 27 s per Friday on these 4 cores (inferred from the two numbers above), against about 1.5 s for a college Saturday.
- What it means for play: once high school games move onto the engine (full build step 4), Continue through an autumn week costs about half a minute on the high school games alone, and the 8 GB M1 has about 4 GB left for macOS and the game's screens.

## Proposals sent to Cain (2026-10-17)

The six questions in the thread's first reply. Answers are recorded here when they come.

1. Draft look-back by scout: settled by real life (front offices review old drafts), withdrawn as a question. Goes into section 7.
2. Own players read through your coaches: settled by real life (teams judge their own players through their coaches), withdrawn as a question. Goes into sections 6 and 7; how much coach skill and time with the player set the accuracy is a detail for those designs.
3. Weekly staff digest. Proposed, not confirmed.
4. Play-calling situations filter. Answered in section 9 (Cain 2026-10-21 19:55, `docs/design/game-day.md` topic 4, PR #83): a choice between your situations (3rd down, 4th down, red zone, goal line, two-minute, plus a take-over key) and every snap, "so that players feel incentivised to try to clall plays and learn". Also a choice for call time: 25 seconds for realism, or unlimited for learning.
5. A speed target for Continue (pick: an in-season week under 10 s on the M1, other games simulated in the background while the user works the week). Proposed, not confirmed.
6. A memory target (pick: the whole game under 3 GB on the M1). Proposed, not confirmed.
7. A feel-test console after section 5. Proposed, not confirmed.

## Notes for later sections (no question needed yet)

- Section 6: the user's own career record, coach awards and the Hall of Fame as a coach or contributor follow real life (Q48), so they give the long-term goal without a difficulty setting.
- Section 6b: every training change should come back as a staff report saying why, so the trial "why" rule also exists in the game.
- Section 7: the potential read (`potential_outlook`) is still a placeholder; its words and tiers are the main thing the player will see of hidden potential.
- Section 9: halftime notes and the post-game "why calls worked" breakdown (plays topic 6.6) are the feedback loop for game day; they need to be short enough to read every week.

## Focus set by Cain (2026-10-17 13:03): the emotional connection

Cain: "i want o focus on the emotional connection to the game. why it's gonna be fun including the realism." From now on this view judges first whether the player will care: about players, programs, rivals and their own career. Speed, memory and tedium stay as background notes only.

### In place now (built or designed)
- Players you can follow from age 14: hometown, family, background, high school, recruiting, college, draft, pros (sections 1-2, built).
- Hidden potential with hints, so a find is your own read and can be proven right or wrong (Q18, Q46).
- Slumps and breakouts that have a reason (development model, built).
- Families as actors: loud parents, the people around a recruit (world topic 5, built).
- A real 30-year history, so programs, coaches and rivalries carry a past on day one (built).
- News with reasons: a coach's move names his two biggest pulls (world-model 11.13, built).
- Loud and quiet moments: Small/Medium/Big/Huge, and a regular game becomes Huge when it really matters, e.g. the first start of a QB you scouted since high school (calendar topic 8, built).
- People who remember: broken promises, burned players and their agent's other clients, fleeced GMs, the owner's pet player (section 5, designed).
- Any past game can be watched back from its seed (section 3 topic 9.2).
- A player followed from high school now goes on to a full pro career: an NFL made of the world's own players, contracts on real 2026 pay ladders, an agent, free agency, tags, trades, AI front offices, awards and the Hall of Fame (PR #43, 2026-10-17 evening). Cain's own negotiation and roster screens come with section 8.

### Missing for the emotional side
- ~~A face~~: settled 2026-10-19 (PR #53): 3D heads, Cain's own AI photos on any player (kept through media day), a "Copy AI prompt" button built from the player's data, and a looks list for every player (hair dye, scars, a broken nose, tattoos, earrings, a build that follows his weight, looks that change over the years).
- ~~Players' own voice~~ and ~~media and social media~~: section 6 designed 2026-10-19 (`docs/design/people.md`, PRs #55-#57): 13 hidden personality traits, relationships, morale read through tells (no meter), chemistry, talks with players, life away from football, off-field trouble under real rules, team rules, media and fans by era, staff market, your bosses and job security.
- ~~The career stats record, awards, records and Hall of Fame~~: built and merged 2026-10-17 in PR #42 (career stats record with coach buzz, records, Hall of Fame; high school games on the engine; plays and tactics with the install limit).
- A way to collect "your" players and your own career story (questions sent 2026-10-17, below).

## Log
- 2026-10-17 15:45: PR #42 merged (full build steps 2-5). Legacy pieces now built. Step 6 building: section 5 run by AI teams, an NFL of the world's own players, pro careers on player pages. Faces: Blender hair approved on the 3D heads, beards next. Emotional-side gaps left: face (in progress), players' voice and media (section 6), the three follow questions.

## The lens for brainstorming (Cain 2026-10-17 16:26)
"think like a football fan, not just a robot". Every idea is judged by what a fan feels: hype, debate, being early on a guy, pride, rivalry, heartbreak, the story you tell a friend. The system behind it stays ultra-realistic.

## Brainstorm list (started 2026-10-17 16:22, Cain: "start a list as we brainstorm")

Every item needs Cain's yes or no and is also posted in the thread. Status after each item. All proposed, not confirmed.

Following your players
1. "My players" list kept by the game, with news when one of them does something. Designed in section 7 (`docs/design/scouting.md` part E, PR #58, 2026-10-19). Was parked for section 7 (Cain 16:26: "that's a scouting aspect. we'll touch on later, it's about what it gonna make that scouting screen relate to fans"). Fan angles to bring then: being on a guy before anyone else, your board against the public big boards and mock drafts, player comparisons ("a young Jalen Hurts"), the combine freak moment (DK Metcalf, 4.33 at 228 lb), the hometown story, the online debate about a prospect.
2. Your career story page, a timeline of every job and big call and how it turned out. Waiting.
3. A player's past with you shown on his card when you meet him again. Covered by section 7 (`docs/design/scouting.md` part E, PR #58): every player on the "my players" list carries the record of your history with him (first look, grades then, what the public said). Withdrawn as a question 2026-10-19.

Nicknames (Cain: "Add a dynamic where playersr can earn nicknames in very specific situation")
Settled by real life, no question: childhood and family nicknames carried up from high school (Sauce Gardner); unit nicknames (Steel Curtain, Legion of Boom, the Hogs); named plays (The Catch, the Immaculate Reception, Beast Quake); history players have them too; a nickname shows wherever the player does (card, news, play-by-play, history).
4. Triggers: a specific, rare moment from the play log that matches the player's traits. Waiting.
5. Who coins it and whether it sticks. Waiting.
6. When a unit earns a nickname. Waiting.
7. When a play earns a name. Waiting.
8. What a nickname changes. Waiting.
9. Giving a nickname yourself. Waiting.
10. How names are written, and never reusing a legendary real one. Waiting.
Nicknames, round 2 (Cain 16:43: "brainstorm more about the nikcname")
11. Merch: a nickname that sticks shows up on shirts and jersey sales (Beast Mode became Lynch's own brand). Waiting.
12. The broadcast call uses it on big plays. Waiting.
13. Mocking nicknames from failure (Butt Fumble 2012, Captain Checkdown), hurting by personality. Waiting.
14. Redemption: a mocking name replaced, or owned by the player. Waiting.
15. The player's reaction by personality: embraces, trademarks (Lynch, Gronkowski), hates it. Waiting.
16. Hype names before the proof (Johnny Football, 2012), with pressure attached. Waiting.
17. Coaches and GMs get them too, the user included (The Hoodie, Coach Prime). Waiting.
18. The spread is visible: beat writer, local radio, national, with dates in the news. Waiting.
19. Rival fans' own mocking names. Waiting.
20. Heir nicknames: "the new ___" for a younger player with the same style. Waiting.
- 2026-10-17 20:22: PR #43 merged (full build step 6, section 5 for AI teams). Trial 7 (one 30-year world) started 19:55. Nothing on the open list changes.
- 2026-10-18 00:50: full build finished; trial 7 merged (PR #44, `docs/trials/world-trial7.md`). How its findings touch whether a fan cares (all 18 already with Cain in the build thread, not asked again here):
  - Records lose their meaning: 8-16 FBS backs pass 2,000 rushing yards most years (real 0-2), sack leaders reach 35 (record 24), NFL passing leaders 5,800-10,200 yards, NFL teams 37.8 points a game (real 22.5). A fan cares about a record only when it is rare; nickname items 4, 7 and 16 lean on rare moments, so they depend on this fix.
  - Little variety at the top: 8 champions in 30 years (real 15), Georgia 14 titles. Rivalry and hope need a chance for the underdog.
  - Comeback Player went to long snappers 5 times: an award a fan would laugh at breaks the spell.
  - Cards for classes before 2006 lose their college stats: items 2 and 3 (career story, a player's past) need the full record kept.
  - A 30-year world now takes 94 minutes on 4 cloud cores (trial 6: 41).
- 2026-10-18 13:15: PR #45 merged trial 7's 12 plain bugs (items 4-7 and 11-18), no trial after. Effect on what a fan sees: player cards now keep every recorded college year and the offers note (item 15), so the dependency of brainstorm items 2 and 3 on old college stats is gone; every NFL game now runs on the engine with a box score (item 18), so a followed player's pro career has no missing games; blue-chip recruits sign scholarships again (item 4), so a 5-star's recruitment reads like the real thing; the 2010 uncapped year has a market again (item 5); class rankings list FBS programs only (item 13); trade and stat text fixed (item 17). Still off until the next trial: comp picks (9-22 a year, real 32) and tags (14-23 a year in 2021-25, real 6-9). Still waiting on Cain (trial 7 items 1-3, 8-10): inflated records, two programs winning most titles, award positions. These are the ones that touch whether a fan cares most.
- 2026-10-19 01:22: merged since PR #45: PR #46 deeper attribute list (67 visible, 5 hidden) with an engine retune; PR #47 Cain's career at every level (one career can start at any high school, college or NFL team and move between them; the first playable opens all three); PR #48 social media and news research (`docs/research/media-and-social.md`, base for section 6 media); PR #49-#50 player tendencies designed and built (six coaching styles, 10 instructions players obey or ignore; results and 5 engine bugs for Cain in `afm/tendencies-build/results.md`).
  - Fan side: players who obey or ignore your instructions give them a character on the field (the stubborn veteran, the coachable rookie). A career that starts at a high school means the "kid you noticed at 15" can be your own player.
  - Effect on open items: item 2 (career story page) now spans high school, college and the NFL. The media research has no nickname section, so items 5, 13 and 18 (who coins a nickname, mocking names, how it spreads) are still new ground for section 6. Items 2-20 still wait on Cain.
- 2026-10-19 17:07: PR #52 merged trial 8's 16 fixes and the names fix (injured reserve ends at the new league year; tendencies in a 3-point style band; local seniors fill FBS and FCS walk-on places; NFL pace 64 plays a game; FBS runs 5.08 a carry; comp picks count qualifying free agents; one snapper on scholarship; five engine bugs; realistic race shares for rare first names). PR #53 (faces, own photos, looks list) merging.
  - Fan side: a face and changing looks are the biggest emotional gain so far; you recognise your guy, see the scar from the 2031 injury and the tattoos he added after the payday. Local seniors as walk-ons give every program a hometown kid to root for. Realistic paces and run averages help records mean something again (trial 7 concern).
  - Effect on open items: none changes. The looks list fits items 4 and 10 (a body-based nickname can draw on a broken nose or a build), noted for whoever builds nicknames.
- 2026-10-19 19:59: section 6 (people) design landed, `docs/design/people.md`, all 15 topics. Two of Cain's answers there are pure game feel:
  - Talks are picked moves backed by a huge line bank, so they never go stale. Cain: "make sure conversations-messages has a uge database so it doesn't get stale, which is one fo my hatrets of FM, been seing the same shit for 5 years". Lesson for this view: every repeated text (news, talks, nicknames, broadcast calls) needs a large, varied bank.
  - Your own health never ends your career. Cain: "nah, imaging spending months on a save and it gets ripped from you". Lesson: nothing outside the player's control should end a long save.
  - Fan side: morale read through tells instead of a meter, players with lives away from football and era-true media (newspapers in 1996, X by about 2010) are what make players feel like people.
  - Effect on open items: people.md has no nicknames and no career story page, so items 2-20 stay open and unchanged. Item 18 (watching a nickname spread) should follow the era media: the next morning's paper in the 1990s, minutes on X today. Items 12 and 18 also fall under the large-text-bank lesson above.
- 2026-10-19 20:48: section 7 (scouting and draft) design landed, `docs/design/scouting.md` (PR #58), no open questions. Each staff keeps its own file per player with ranges that narrow with every look; scouts are people with biases, track records and conviction; NFL pre-draft season, a live draft with AI clubs trading, the undrafted race; college evaluation; the high school coach's seat; following players.
  - Fan side: all the scouting fan moments from 2026-10-17 are designed in: being on a guy first, your board against public mocks, "plays like a young..." comparisons from the world's own history, combine moments with clips, fan reactions and debate, hometown stories, all from section 6's line bank. The scout who found a star talks about it for years.
  - Effect on open items: item 1 is designed; item 3 is covered (the history record per player) and withdrawn. Still waiting on Cain: item 2 and items 4-20. No nickname design anywhere yet.
- 2026-10-20 00:35: PR #59 merged section 5's leftovers and the AI-world side of sections 6 and 7: personalities, moods with reasons, the locker room, grudges, staffs; scouting departments, files that sharpen with each look, each club's own board, draft-day trades between AI clubs, the undrafted rush, recruit evaluators. Fixes: run fits, signing bonus paid once, out-of-bounds plays only partly fixed (5.3% of NFL plays, real 10.7%). Cain's own side of 6 and 7 comes with section 8. Trial 9 (one 50-year world) running.
  - Fan side: the AI world now has the human texture a fan reads in the news: grudges, locker-room stories, a club that fell in love with a prospect and traded up for him. Fewer plays ending out of bounds than real makes the two-minute drill feel less real; trial 9 will show it.
  - Effect on open items: none. Item 2 and items 4-20 still wait on Cain.
- 2026-10-20 05:05: trial 9 (one 50-year world) done; its 11 plain bugs fixed in PR #60. Report `docs/trials/world-trial9.md`; `docs/design/section8-readiness.md` lists what the first playable stands on and 7 things it still needs. Section 8 is next.
  - Fixes a fan feels: NFL moods read pay against the right market; the undrafted rush keeps calling after round 1; no extensions weeks after signing; news grammar ("the Jaguars's camp", "Alabama fire head coach") fixed. Sloppy news text breaks the spell as fast as a wrong number.
  - Findings waiting on Cain in the build thread (not asked here): Texas won 12 of 20 future titles (dynasty again, after Georgia in trial 7; a fan needs hope for the underdog); NFL players almost never happy with their role (10 of 2,141, so every locker room reads unhappy and moods lose meaning); trades few (3-18 a year, so deadline day, one of the Huge moments, has little drama); NFL injuries too rare (no next-man-up stories); memory reaches 7.1 GB by 2045 with the game log on, 4 GB budget proposed (this view measured 4.0 GB for a 2-year world on 2026-10-17; on the 8 GB M1 this decides whether a 50-year save is playable at all).
  - Effect on open items: none changes. Item 2 (career story page) is a natural part of the first playable; worth raising when section 8 design starts. Items 2 and 4-20 still wait on Cain.
- 2026-10-20 09:02: section 8 (first playable) designed, `docs/design/first-playable.md` (PR #61), 11 topics. Cain's calls 2026-10-20: a library of ready-built worlds (a career starts in seconds, fresh randomness from day one); the last 12 years of high school history on the engine for now; a Tauri app with HTML screens; section 8 builds everything the seat touches in season one (every assistant as a person, high school staffs, bosses, reporters, area recruiters, look-backs, the "my players" list). Staff run training like the AI until 6b is designed. Also: your profile and a background points pool, any head job in February or starting unemployed, exact-replay saves with autosaves before big moments, memory and speed targets for the 8 GB M1, a "Report a bug" button.
  - Fan side: starting in seconds means you get to your first season while the excitement is fresh; autosaves before big moments protect the long save (Cain's "months on a save" rule); starting unemployed gives an underdog career story from day one.
  - Effect on open items: first-playable.md places them (line 130): the career story page under your profile, earned nicknames on player pages and in the news. Still waiting on Cain: item 2 and items 4-20. Speed and memory targets (my earlier questions 5 and 6) are now settled there.
- 2026-10-20 10:33: section 6b (training) designed, `docs/design/training.md` (PRs #65, #67), 15 topics. Practices are periods by the minute (gear, contact, reps for ones, twos and scout team), each growing only what it works; real rule books per level and era; a plan per player (main focus, a habit to fix, a position move, a body target, extra work, rehab); practice injuries from contact and load; a summer feed of signs ending in the report-day weigh-in; the AI uses the same tools. Cain's calls: growth lands every block and every week of the season; a plan moves a young player's year by up to about 15% either way, a main focus growing up to 1.5 times faster while other skills slow; practice snaps run on the match engine.
  - Fan side: weekly growth means you watch your project player get better through the season, not once a year. The summer feed and the weigh-in give a real "did he put in the work?" moment (the player who showed up 15 pounds heavier). A habit to fix gives each young player a story arc you own. This also answers the old "staff report on why he grew" note: training now carries its own reasons.
  - Effect on open items: none. Item 2 and items 4-20 still wait on Cain.
- 2026-10-20 19:12: build 10 (trial 9's 14 findings) merged (PR #66). Section 10 designed on Cain's Showtime design system (`docs/design/screens.md`, PRs #68, #70-#72; mock canvas https://claude.ai/artifact/7CdxyHAryWF24XyA7pvdqF): dense, warm film-room screens; a player's whole file, the department and an NFL draft board; an 8-tab Game Center; a 2D replay with the coverage shell and the QB's eyes; signing day hour by hour; talks with move previews. Cain chose a frosted sidebar, numbered dots in the 2D view and Showtime from day one. Design is done for now; the section 8 build (first playable with 6b inside) starts in its own thread.
  - Fan side: signing day hour by hour and the QB's eyes in the replay are pure fan moments; the warm film-room look makes it feel like a football building, not a spreadsheet.
  - Effect on open items: the build is now passing the spots where items 2 and 4-20 would sit (first-playable.md line 130). Asked Cain in the thread to answer before the build reaches them.
- 2026-10-21 00:27: section 8 merged (PR #75, `docs/game.md`): a Tauri Mac app, every screen on Showtime, careers at any high school, college or NFL head job. In: squad, depth chart, a training focus per player, recruiting and contracts screens, game pages, player pages with staff reads as ranges, notes, saves. Not yet: the choice screens where the AI still decides, most of training, the high school JV squad, faces, the 2D view. Trial 10's 30-year world imports as the first save.
  - Fan side: Cain can finally play. From now on this view judges what Cain feels while playing, not designs on paper. The biggest emotional gaps in the build are the missing faces and 2D view (you can't see your guy yet) and the AI still making choices that should be yours.
  - Effect on open items: none of items 2 and 4-20 is built (no answer from Cain yet). Future proposals must fit `docs/game.md`.
- 2026-10-21 19:50: the section 9 (game day) design thread took over first-pass question 4 (which snaps show the coordinator's call) and covers the halftime notes and post-game "why calls worked" breakdown (its topics 9 and 11). Note: this is the first-pass list's item 4, not brainstorm item 4 (nicknames). Brainstorm items 2 and 4-20 still wait on Cain.
- 2026-10-21 19:57: first-pass item 4 answered in section 9 (see the proposals list). Game-feel lesson from Cain: where realism and learning pull apart, he wants a player's choice (a realism mode and a learning mode), never a flat difficulty setting.
