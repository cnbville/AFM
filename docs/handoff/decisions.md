# Every decision Raichel made

Written 2026-10-14 from the word-for-word transcripts in `transcripts/`, for the move to the new account. It lists every decision Raichel made in the old project, by section and topic, with Raichel's own words, the date and where to find it.

How to read it:
- **Quotes are copied exactly**, typos included. Times are UTC.
- **Pointers** like ([T13] 2026-10-13 17:08) mean: open that transcript and find the message at that time. [RM] is `raichel-messages.md`, which lists every message Raichel wrote, oldest first.
- **Superseded** marks a decision that a later one replaced. The table at the end lists them all.
- A plain "yes" from Raichel accepts Claude's proposal just above it. Those proposals are summed up here, and the full wording is in the spec named in each section.
- Claude's own picks that Raichel never answered are **not** in this file. They are in `open-questions.md`.

The specs in the repo hold the full detail and Raichel's wording. This file is the index and the record of who decided what, and when.

| Section | Full spec in the repo |
|---|---|
| Vision, the 50 roadmap answers, build order | `docs/roadmap.md` |
| 1. Player model | `docs/decisions.md`, `docs/design/player-model-decisions.md`, `docs/design/trial6-fix-list.md` |
| 2. The world | `docs/design/world.md`, `docs/world-model.md` |
| 3. Match engine and stats | `docs/design/match-engine.md`, `docs/match-engine.md` |
| Engine physics fixes | `docs/design/engine-physics-fixes.md` |
| 4. Calendar | `docs/design/calendar.md` |
| Plays and tactics | `docs/design/plays-and-tactics.md` |

---

## 1. Vision, platform and how the game looks

- **The brief (2026-10-10, the project description).** "Create an equivalent game to football manager 27 but then focused on American Football. Realism, stats and dynamic potential is extremely important. It should have an patient flow that doesn't rush climaxes. It should have everything from player cards, High school players that have realistic improvements upto the NFL."
- **Visuals don't decide anything (2026-10-10).** Raichel asked what visuals do to realism ("So for me ultra realism is important but i like the ida of having visuals. What effect will thais have on my realism ?"). Answer: the sim decides, visuals only replay it. Raichel picked "A mix between A & C": text and stats as the main view, plus a 2D view. "Then the highlight aspects inspired by next-gen visulaizations like this one" (a Next Gen Stats pass chart, saved in `uploads/c2625333-...png`). ([T02] 2026-10-10 20:15)
- **Stack, first round (2026-10-10).** Claude's default was a TypeScript web app ([T01] 19:43). Raichel asked about Godot and Unity ("expand on what you mean about "real game" with godot/unityy", [T01] 19:46) but never picked a stack then. **Superseded** by the Rust desktop app below.
- **Platform (Q49, 2026-10-11).** "Whatever allows me to run the most realistic dense game engine (discuss your answer with me first)", then "Okay you can rewrite into rust, do this in a new thread and immeidately push to git" ([T00] 2026-10-11 19:19, 19:26). A desktop app with a Rust engine. The UI technology is not chosen yet.
- **Build order (Q50, 2026-10-11).** Playable early with plain screens, then add depth: "Ahhhh okay, damn bruh you had me confused.. hahhahah yeah A. Smart" ([T00] 19:28).
- **Better visuals later (2026-10-13).** "I am potentially intrested in better visuals so leave room for htat ." The engine outputs full player positions so a nicer 2D or a 3D view can come later. ([T13] 17:50)
- **Personal use only (Q6).** "FOr now it's just meant for me personally on my computer, no itention of releasing it as an steam game" ([T00] 2026-10-11 18:42).

## 2. The 50 roadmap answers (2026-10-11)

All in `docs/roadmap.md` section 3, with Raichel's words. Asked in five batches of ten in the project chat ([T00] 2026-10-11 18:42 to 19:31). The ones with Raichel's own wording:

- **Q1 role:** "C", both GM and head coach, like FM.
- **Q2 career:** "Absolutely, the game starts in 2026 then could go on for 50 years if needed, switching teams".
- **Q3 teams:** "Any teams, again it's based on real life and a comprehensive simulation game."
- **Q4 levels you coach:** "NFL for now, leave room for updates". **Superseded** on 2026-10-18 by the coaching career at every level (section 12 below).
- **Q5 names:** "A, realistic names". **Superseded** by Q11.
- **Q6:** personal use (above). **Q7:** "One realistic mode". **Q8:** "Yes, changeble starting stats".
- **Q9 levels simulated:** "All 3 with extreme detail, i want every part to matter, stats in highschool should translate to NFL (i mean this nuanced)".
- **Q10 league setup:** "A, -> same goes for FBS, FCS, D2 they should all have real life aspects."
- Raichel added: "make sure the questions in batch 2 take these answers into account" ([T00] 18:42).
- **Q11 names:** "I changed my mind, i want NFL to have real names too, but add an toggle when starting new game that allows you to choose before you start a save" ([T00] 18:50).
- **Q12-Q17:** A (real towns with generated high schools; every high school plays full games; one engine adjusted for opponent strength; real rules per level; real calendar per level; real recruiting geography).
- **Q18 what you see:** "A -> add an small hint/tip when you have high scouting skills. For example if a smaller edge in HS has a High QB pressure rate and the GM's scouting skills are high, he gets a small hint "this player is probably gonna grow in size or a hint that says the opposite"".
- **Q19 personality:** "Absolutely 10000% yes, ultra-realism". **Q20** manager ratings grow: "Yes."
- **Q21-Q30:** "yes to all" ([T00] 18:51): off-field events, morale and chemistry, the full cap, agents, rookie scale and fifth-year option, trades including AI to AI, roster limits, owner budgets (full finances later), rated staff, schemes.
- **Q31 scouting:** "A yes but i'm curious what you think scouting as an NFL GM/HC is right now ? how do you currently treat it." ([T00] 19:07)
- **Q32 combine:** "A) Yes but with insane detail, did the player show up early or  late, are his parents loudmouths that yell at every game or are they social media happy ?  how did he seem to the scouts ?"
- **Q33-Q39:** A (live draft with trades, UDFA rush, game plan plus live calling, real schemes with adjustable tendencies, in-game decisions with staff advice, weather and travel modestly, in-game injuries).
  - Q36 said "not a play-drawing editor". **Superseded** on 2026-10-14 by plays and tactics topic 8, a full play designer.
- **Q40 game length:** "Game time on paper should always be 15 minutes, when live play calling it's play by play".
- **Q41-Q45:** A ([T00] 19:19): save starts mid-February; weekly training; news inbox now, press conferences later; owner goals; dense FM look.
- **Q46 potential:** "Potential should pretty much be hidden, like the suggestion i gave earlier the better your scouting you'll receive small important hints and i agree there should be a small idea of potential but i absolutely don't want the "this is the players ceiling exactly" that defeats the entire point of the game and is also what i mentioned in the beginning. This is extremely pivotal to understand". Then "46. agreed." ([T00] 19:26)
- **Q47 stats:** "Yes, i want every possible stat, i want to be an huge stat nerd". **Q48** history and Hall of Fame: "yes".
- **Depth comes later, per section (2026-10-11):** "obviously i want waaaaay more control of the details for example in the training aspect i see sooooo much realism, potential in there." ([T00] 19:31). Training became its own section, 6b.

## 3. Project process decisions

- **Ask first (2026-10-10).** "I need you to calm down with the freestyling. I need you to run aspect through me because you're making a lot of decisions without me right now" ([T00] 20:01).
- **Ultra realism by default (2026-10-11).** "so i'm noticing you ask me a lot of questions that you already know the answer too, i've told you multiple times i want yltra realism so your questions are arbitrary." Plus the example of good questions: "lets talk about the ranking of coaches, if a school is in a crime/low income zone how does that effect as opposed to a matter dei. That's the real shit i want to. get into." ([T00] 19:35)
- **Section order, the section loop, and four other items: "ok to all" (2026-10-11 20:03).** The six were: the section order (world, match engine, calendar, team management, people, training, scouting and draft, first playable, game day, screens); the loop per section; the scouting-staff design; the college grade categories; 30 years of hidden history; the topic 1 leftovers (personality and the powerhouse flip side). ([T00] 20:03)
- **Everything in git (2026-10-11).** "push to an full spec in repo - i want everything we roadmapped and build so far uploaded to github in extensive detail. I work outside of this chat too so anywone not using this chat should be able to read it too" ([T00] 19:56).
- **Trial 6 setup (2026-10-11 17:53-17:56).** A fresh thread, 20 players, no hunting for bugs, read GitHub, push at once. Q1 "B random but realisitc scenarios that show the full range of possible outcomes", Q2 "A", Q3 "B" (merge into main). ([T00])
- **Gameplay finds the bugs from here (2026-10-11).** "These will be the final fixes until we're in the game, we'll debug from there." ([T04] 17:21) and "Don't find/search calls/bugs, that's up to me to find out when eventually in gameplay." ([T00] 17:53)
- **Build order after section 3 design (2026-10-13).** "yes to both, but first we also had fixes for world-trial 3, i propose we first build section 3 into the engine, then we fix the the list into the engine and then we run a World Trial 3 with an increased dashboard so i can deepdive/debug." ([T00] 18:22)
- **Parallel work (2026-10-13/14).** Section 4 design: "Let's work section 4 in the mean time" ([T00] 18:27). Plays and tactics: "let's do that in the mean time or will that distort the progress ?" ([T00] 2026-10-14 11:08).
- **Design topics answered in the project chat (2026-10-13).** "i'm answeringing topics here to save tokens ->" ([T00] 05:49).
- **PRs merge (2026-10-13).** "do PR and merge ofcourse" ([T11] 16:39).
- **Physics fix: spec only (2026-10-14).** "don't build push spec to  to git as next needed fix," ([T17] 12:20).
- **The account move (2026-10-14).** "I have two claude accounts, this one which has this project is going to close and i want to continue on a different one." Then "wait till i finish the other threads so there is full context and only one pass", and "can't harp enough that this handoff needs to include all the possible context so that nothing we spoke about gets left behind...nothing". ([T00] 11:43-12:22)

## 4. Section 1: the player model (2026-10-10 to 2026-10-11)

Thread [T02] unless noted. Details: `docs/decisions.md`, `docs/design/player-model-decisions.md`.

### Ratings, attributes, positions
- **1-99 scale:** "A." ([T02] 2026-10-10 20:16). "I want the scale to be to 99, so an elite player like an Prime tom brady is 99" (20:52).
- **Deeper attributes, about 35-40 FM-style:** "B" (20:17). The exact list was promised for sign-off and never sent (see open questions). The code still has 25.
- **Positions: about 20, every player rated at every spot:** "C, but use realistiac and dynamic reasoning as to why. Speed for an CB playing as an FS is more useful  then run support skills and FS with bad run support isn't as good at SS. Returning opinion is that every thing needs to be realistic" (20:21). The code still has 13.
- **Level-relative rating on cards:** "Q1: Yes" (20:59).

### Potential and breakouts
- **Hidden ceiling per attribute, never a hard wall:** "A. but i have a very important addition. I absolutely hated in in football manager when i had an Striker score me 35 goals but his potential limited him to 11 finishing. I understand that there is an ceiling but certain break out seasons should be able to break it open." (20:23)
- **Breakouts lift a ceiling up to about 10 a year, at any age, less when older:** "B on both" (20:25).
- **Original vs boosted potential:** "Q2: both" (20:59). **Superseded** by Q46: shown on trial and debug pages only, never in the game.
- **Potential hidden in the game:** Q46 above, "extremely pivotal".

### Development and volatility
- **More volatility (2026-10-10 20:52):** "The improvements are way too linear, i'm not seeing enough volatiltity, Sophomore slumps in the Pro League, True freshman instantly rising, or redshirting. It's too perfect. One single season can make a bigger jump then what i'm seeing."
- **Reaching potential:** "I'm not seeing anyone actually hitting their full hidden potential." Then "Q4: Try that, we'll do more trials to see ooutcome" (20:59): about a third reach it.
- **99s too easy:** "it's too easy to be 99 rated HS and NCAA" (21:25).
- **First-year adjustment, dynamic (2026-10-11 12:53):** "add an dynamic aspect, Players like an Jax Smith-Njigba or Malik Nabers immediately proved to be able to handle, certain UDFA thrive due to x or y. Just don't make it stact, make it dynamic". This came after Raichel questioned the FCS edge rusher (81 in college to 72 as a rookie) against the FBS safety (95 to 76): "let me clarify, i'm happy with the safety, suspicious of the edge" (12:31-12:38). The drivers Claude listed are still unconfirmed.

### Stars, busts, the draft
- **NFL tiers are guidance only; teams draft badly sometimes:** "make the tiers a suggestion. Teams have stats like how good they draft (linked to GM stats and Owner stats)" (20:59).
- **D2 and below:** "D2 rarely or never gets drafter (UDFA) and if they do it's day 3 max." (21:25)
- **Five-stars must dominate high school:** "Q1: Agree with fix 1, can't be 5 star without being dominant, in those cases the drop of can't begin until FBS" (23:07).
- **Misjudging is a small bust cause (2026-10-11).** Raichel first read the rule as busts only after signing, then: "okay then i misunderstood originally, obviously misjuding should be an aspect, albe it small. Plenty off hard working QB's don't make it to the nfl, so it'd be dumb to ignore that." ([T04] 14:42). **Supersedes** the earlier rule that busts only start after signing.
- **Calls 1-7 after trial 4: all A, with context (2026-10-11 12:27):**
  1. Freshmen and the HS-to-college gap. Raichel: "Feel like we're missing an dynamic layer... the gap between HS and NCAA you have is too little... tell me what before you adjust anything".
  2. Declaring early. Arch Manning, Dante Moore and the Jets, Sam Darnold: "(we'll talk before we implement.)"
  3. Careers. "make sure there is an free agent market that tracks those players so they'll be able to resigned just like the current NFL and also the NCAA portal."
  4. Injuries cost games: A.
  5. Depth charts: A.
  6. Position value. "also add an dynamic where an team that has a veeeeery good defense but is missing an generational CB or OLB reaches for the player for that position. So positional and situational weights."
  7. Sideways and down transfers: A.
  - Plus: "let's maybe build 30 player trial before we go make big decisions." and "let's talk first".
- **Fix the coachability bug and stars on upside:** "Fix both" (card, 13:57).
- **Trial 5 as a league sim with real teams:** "League sim" (card, 13:48). Filled with generated players: "Generated" (card, [T04] 14:02). Trial 5 in a new thread: "Do trial 5 on a new thread" ([T00] 13:58).
- **Trial 5 review ([T04], 2026-10-11):**
  - Retirement by position: "I agree that retirement should start to take into account position but leave room for outliers like Derrick Henry" (14:37).
  - Dismissal severity: "no i meant the 83... dismissed at 19, doesn't quanitfy to dropping to fcs does it ? he can transfer to a smaller FBS school depeniding on how sever the cause of dismasall was", then "A" (14:47-14:50).
  - Career length: "Certian players play longer, certain players can play shorter. Mental stuff, also players that rely on their legs lika an Jayden Daniels can opt out way faster than a true pocket passer" (14:55).
  - Cuts: "It's indeed a cut too early, so let's look at lowering the odds, as long as there is an free agency/ practice squad itmakes sense." (15:00)
  - Urgency to declare: "someone that's unranked might be scared that he'll drop off next year, so let's add an dynamic that calculates urgency to declare, either financial (small schools barely have NIL) or opportunity (RB's have a short shelf live...)" (15:00).
  - In-season changes: "add the dynamic of in-season upgrades and downgrades, players can make a good impressioni n pre-season. Outperform statwise, that combined with bad judgment i can see a 62 rated pro perhaps make the roster at the worst team in the NFL" (17:21).
  - "I also agree with all the other calls and bugs." All of it became `docs/design/trial6-fix-list.md`, built in trial 6.
- **Film line for stars and FBS offers:** "C" ([T00] 17:48), the strict line.
- **Drafting team matters (Darnold, Dante Moore):** in call 2's context above, built as the landing-spot score in trial 6.

### Season calendar draft (thread [T03], 2026-10-10/11)
- **What you control:** "NFL team only" (card, [T03] 2026-10-11 14:00).
- The rest of that draft (`docs/design/season-calendar-and-pacing.md`) is **superseded** by `docs/design/calendar.md` wherever they differ. Its "one step per checkpoint" rule was already replaced by the breakout rule on 2026-10-10.

## 5. Real-world data

- **Five-star draft target 63%, and research freely (2026-10-13):** "1. 63 2. YOU RESEARCH WHATEVER YOU NEED TO" ([T08] 06:01).
- Raichel supplied two data files (2026-10-11): `uploads/907dfe5e-...json` and `uploads/128d811e-...zip`. Then: "it seems you're still not happy with the data, you go ahead and do the search in a fresh thread then push it to git" ([T00] 22:42). Result: `data/real-world/` (PRs #7-#12).

## 6. Section 2: the world

Spec `docs/design/world.md`. Design in the project chat [T00]; the spec thread is [T10]; the build is [T11].

### Topics
1. **Hidden background (2026-10-11 19:52):**
   1. "I don't like the nutrition aspect but i do like the rest, also make it incidentally effect certain aspects like grittyness, tendency to YAC... do not make this a driving factor".
   2. "high potential +  bad surroundings + bad family structure + is a recipe for disaster but i repeat don't make this overly powerful... Perfect family/surrounding and low ambition can hurt someone that is gifted."
   3. "yes but don't make it overly important... noteworthy influence on potential, not driving factor."
   4. "someone from mater dei should have less untapped potential then a gritty kid... I repeat, don't make this a driving factor... players like Amon-Ra St. Brown exceeded excpetions".
   5. "make the exposure network extremely dynamic and long lasting, so that schools in lower divisions feel incentivized to hire coaches with network over ability... a mater dei doesn't need a coach with a lot of exposure since the school itself is exposure."
   6. "Definitely an aspect, should also impact scholarships to ivy like schools, use a similar rating system as EA CFB27 use for colleges".
   - Follow-ups (19:54): "1. Yes, a bit more then rarely 2.  Yes and Yes 3. Mental health a big part,  religion not."
2. **Powerhouses (2026-10-11 23:30):** "1. Yes, it happens a lot so it's more then realistic 2. no, it's gonna be too hard to regulate" (local coaches keep stars; no penalties for powerhouses).
3. **Coaches rated (2026-10-13 05:41):** "1. Yes 2. Yes": hidden ability and lagging reputation; ability grows and declines; coaching trees.
4. **Exposure (05:45):** "1. Yes 2. Yes but only if those player signed leave an impact, so leaders in stat for,s, winning awards, being drafted in Day 1/2. Give that pipeline a  reason in stead of just signing 20 players to force a pipeline to exist".
5. **Family (05:49):** "1. Yes 2. Yes, this recently hhappened with that kid who was forced to sign to Tenesse by his mom then immeidately transfered when he was 18."
6. **JUCO (05:51):** "1. Yes since everything is realistic those should be too, do remember the toggle we have in the start set-up 2. Yes".
7. **Prestige (06:02):** "1.  it's delicate since it'll be hard to consistently push nfl talent while having subpar coaching hires. Sure one or two drafts could happen that way but not over 10-2o years 2. Yes, if anything this is what pisses me offin the EA CFB game... Certain aspects should upgrade lineaery but certain aspect should volatily".
8. **NIL (07:34):** "1. follow state rules 2. Yes, but give clear warnings in game".

### Follow-up answers ([T10] 2026-10-13 07:37-07:56)
- 1 mental health "Yes"; 2 coach reputation: "Yes, big results like state titles or conference wins should have a maaaaassive boost. Think of Kirk Cignetti with JMU. But then signing bad players, having bad locker room should visibly but slowly chip away at that reputation realisticy.  A lot of bad coaches get hired and fired because of recency bias"; 3-6 "Yes"; "7. yes, 8 yes"; 9 "yes".
- "where do yu want me to answer those questions /" was answered: right in the thread.

### Build questions ([T11])
- **Realism Q1, all real programs:** "Agree buddy" (2026-10-13 08:03). Claude's proposal: all 426 real FBS, FCS and D2 schools, plus D3, NAIA and the real JUCO programs in a data pass. Questions 2-7 were never asked (see open questions).
- **A mock GUI:** "can you visiualize the trial for me so i get an idea, a mock gui" (10:15).

### World trial 1 list ([T11] 2026-10-13 11:35-12:39), all built 2026-10-13
Kept in `project-files/design/world-trial1-feedback.md`.
1. **Prove-it extension:** "IF a succesfull coach with 6 titles that has been there a long while has two bad seasons... there should be something inbetween where the sucess gives him an extension... Only in case of loyal coaches and sucessful coaches" (11:35), then "Agree." (11:41).
2. **Race disparity in coach hiring:** "There is an race bias in coaching, it's a senstive but realistic aspect... especially pre-2026 there was a disparity of caucasion coaches. I want that realized in the game, it's senstive but neseccary for immersion" (11:41).
3. **Names by ethnicity:** "yes on all , ad it to list but make sure there is an link to crating names. There is no white guy that is called Jalen or Trey or Kobe  like in this trial run" (11:43).
4. **Pipelines:** "despite the prestige increasing pretty heavily, there is no new pipeline. There should be realistic pipelines links" (11:51), "agree" (11:55).
5. **Coach buzz from stats:** "i like the overrating of the East Michigan coach, it's realistic... make more stuff reason for hiring. Like breaking an all-time record of highest PPG, or an absurd amount of EPA % make it relative to level" (11:58). Then "agree, also add the same dynamic to HS coach to and NCAA OC/DC, not just jumping from HC to HC, even HS to QB coach." (11:59)
6. **Academics:** the Kendrick Lewis GPA case (12:02). Then "why i mentioned party school is that a RICE/UCLA won't take that risk unless they fully believe the player can outrgrow it" (12:04), "agree" (12:06).
7. **Draft value:** "a cornerback is rarely drafted top 5, let alone top 3, let alone first pick" (12:18), "agree" (12:21). Then "don't just check #1 check the entire first round, the logic needs to be bound to every draft pick but still be intertwined with team need" (16:23).
8. **Measurables and fuller pages:** "i want more player details being shown, combine stats" (12:18), "where is the fuller page we discussed ?" (12:22).
9. **Hidden-gem stat footprint:** "the something in the stats should really show up that make me go " oh damn"  so you can scout it." (12:35). Then "Agree, okay i've debugged enough for trial one." (12:39). Waits for the career stats record.

### After world trial 2 ([T11] 2026-10-13)
- **Refresh both pages each trial:** "why did you only give me a players page trial 2 ? and not the mock up you gave me the first time as well ?" (14:49).
- **Park trial 2's flags:** "no list them up for potential trial 3. I wanna get moving on the rest" (15:16). They went to `project-files/design/world-trial3-candidates.md`.
- **NIL per player (item 13):** "but he ranked it on the NIL rating of tthe school and not his individual offer ? that's a major let down" (15:54). Then "1. Position value is a huge aspect, use realistic data 2. Yes and add an dynamic where a lower tier school with a good NIL can reach in order to try to bring a new era to the school. Think Colorado Buffaloes or Texas Tech or Indinaa hoosiers... This is situational though not permant" (15:58), and "it should differ based family money... A delicate dynamic" (16:00).
- **Pick 1 QB share:** "I'm still noticing the #1 QB is below real life percentage... Even this year with Jeremiah Smith, which is a sure fire generational WR he's probably not going first due to teams needing a QB. Same goes for edge rusher. so do a doublep pass on this" (16:10).
- **No re-measuring after tuning:** "no need to re-messure after tuning. Note it as an worry/potential fix in the list so we can trial it after words" (16:38).

### Coach promotions and team growth ([T12] 2026-10-13)
- Raichel's ask ([T00] 16:30): "what was the reason for DB coach ? i want to make sure you're not just promoting on prestige in general but speciliaztion", and "i don't want the bar to rise, teams that rise should have reasoning so that players who play the game can pick a strategy".
- Fixes A, B and C: "agree on all three, at them to the list. Probably run the fix after section 3" ([T12] 16:55). Built as trial 3 item 15.

### World trial 3 fix list ([T15] 2026-10-14)
- **Build it right after the engine:** "no the agreement was to do the world trial 3 fix after the engine ? is there something blocking you from doing this ?" (06:46).
- **All college games on the engine, history included:** "but how can we trust the data output if the engine is not kept the same" (08:05), then "Since i'm just playing it myself i want the final game version to include the last 30 years so that the current players are as realistic as possible. Just for this specific test, run 15 years instead... As simillar to the final game as possible otherwise i'll find compounding bugs later on" (08:07). **Supersedes** Claude's proposal of the quick model for history (08:03-08:04).
- **Dynasty fix:** "B- but check linked positions too. A QB won't like having horrible receivers. A corner won't like playing with back D-line. The nuances are important and just B is too plain" (08:14). B was "room plus portal".
- **World Trial 3 in a fresh thread:** "start it in a cresh thread please" (08:33).

## 7. Section 3: the match engine and stats database

Spec `docs/design/match-engine.md`. Design in [T13], 2026-10-13.

- **HS stats belong here:** Raichel asked "in a different thread you mentioned we'll track hs stats in this section, is that coming ?" ([T00] 16:57). Yes.
- **Topic 1, how a play is decided:** "I agree wiht your proposal and both questions, it's so extremely important that we make it realistic, unless it breaks the game or the possibility of the game you'll always match ultra realism" (17:00).
- **Topic 2, movement:** vision tied to recognition: "Yes and it should partially be linked to play recognizatiion skills, someone with hight play recoconition or FILM-IQ could  potentially "see" more. But offensively and defensly. Still an element of luck/physics. If someone falls he shouldn't be able to then tackle... i just want to reward smart players over "dumb,lazy" ones" (17:07). Tackles step by step: "2. yes" (17:08).
- **Topic 3, calls and schemes:** "Yes but also link it to a coach stat- Strictness, certain coaches are just dicks that don't trust their players, make it dynamic... wr's can run to an open field to bail their QB out... DB's brekaing assignments, that's what partially seperates the good players from system players." and "2. Obviously and i expect you to research substantial information and data base" (17:12). Then "about topic 3 no need to be made hidden, if anything coaches build up a reputation for it." (17:18). **Supersedes** strictness as hidden.
- **Topic 4, momentum:** "it's defitenely an intangible but how do you presume it can be measured ?" (17:20), then "Okay sounds good 2. Oviously" (17:21).
- **Topic 5, levels in one engine:** two-way players "1. Yes". Raichel raised player creation: "are you building palyers with randomized/controlled stats and THEN assisgnming a position ?" (17:28), "do a swot analyse of that potential fix" (17:29), "yes definitely it'll definitely help with creating athletic specimens. A wide receiver who has high play recognization but bad hands could be a 4.2 40-yard safety" (17:31). Schemes: "yes but also link it to coach level and available players. I don't want a highschool to run an 3-4 okie if the edge sucks but they have great DT's." (17:32)
- **Build order:** athlete-first creation first, then the engine, then the dev checks.
- **Topic 6, injuries and subs:** "yes on both" (17:33).
- **Topic 7, special teams:** "yes" (17:36). Then: "i don't agree on a small part, it's what seperates the best kickers from a okay kicker. Anyone can kick a ball 50 yards. the mental is the rest. So make those intangibles way bigger for kickers/punters, also give them specilaiton stats, one kicker can be lethal within 30 yards but struggle with power and one could blast a ball from 55 but be inconsistent from 20-40" (17:37). **Supersedes** Claude's proposal that the mental side is a small factor for kickers.
- **Topic 8, stats:**
  - "expand your proposal on section 8, since i'm a stats fetiscist" (17:39); the full proposal is `project-files/design/section3-topic8-stats-proposal.md`.
  - "So this is more about the logging of stats, not how the GUI displays it right ?" (17:41); "please compress it a bit" (17:42).
  - "1. Yes, keep playerstats forever (i assume you delete the ones that won't go to NCAA or don't go pro...) 2. yes but don't make school data so distorted that scouts consitently show up to "lies"" (17:44).
  - "basically as much logging as possible" (17:44).
- **Topic 9, watching and calling:** "1. As an HC i can always chose to take over a play or i can allow the OC/DC to choose. When i don't choose to pick, the game should show me the play/visualazed and then i can accept or deny. THe quality of the playcal is depend on the quality of the coach and how well you researched plays and what counters what 2. No but i should be able to wathc back and visualisation if i chose to, so no live sim but afterwards" (17:50).
- **Topic 10, real-data checks:** "i understand the idea.... but are you treating it as an safe guard or as an in-game fix ?" (18:11); "okay becuase i'm very insistant of having outliers be present, a derrick henry type of situation where he scored 153 touchdowns in higschool should be a ble to happen" (18:12); "as long as it's in development, yes" (18:13).
- **Next step is the build:** "i assume next step is building section 3 into the engine ? or not" ([T00] 18:20).
- **The spec's two unconfirmed items: "yes to both"** ([T00] 18:22). (1) For high school players who never make college, keep season totals, awards and records, and trim only their game-by-game detail. (2) A player who keeps beating expectations in the stats gets higher hidden ceilings; this is how the breakout rule triggers.

## 8. The engine physics fixes (World Trial 3, [T17], 2026-10-14)

- **The question:** "Let's start with the first dynasties. I'm worried what causes the dynasty. It's too much but isn't the issue more the physics of the simulated games ?" (09:16). "3 years is fine for this specific test" (09:27).
- In the project chat: "World trial 3 is making me nervous about the engine, earlier we discussed how we wanted the outcome of plays to manifest right ? remember ?" ([T00] 10:55).
- **Trace first:** "trace tenessee first so we can fix borth" (10:57).
- **The coaching rule ([T00] 11:03):** "i can partially understand the coaching part, it still needs to have better affect but in the long scheme it should only heavily influence the outcome of a play when the user-player doesn't pick. Hopefully this will be a game where if you chose a play that objectively is wrong (running my RB against +2 defense) it punishes that and not solely if the HC. So the HC qualitity should be incombant on that."
- **The five fixes as a spec:** "yes" (12:14). They are QB play, line play, depth and rotation, the RB bug, and coaching by the rule above.
- **Don't build yet:** "don't build push spec to  to git as next needed fix," (12:20). The spec is `docs/design/engine-physics-fixes.md`.

## 9. Section 4: the calendar

Spec `docs/design/calendar.md`. Design in [T16], 2026-10-13/14. Topic order: "yes" (2026-10-13 21:17).

1. **The clock:** confirmed (one-day steps; one real clock; dates from real anchors; save starts the day after the Super Bowl; big days hour by hour).
2. **The NFL year (08:40):** "1. yes but i'm interested in building an creative way wherre we can see and montiors player work in the off-season. We'll come to that later though. I just wanted to mention it so you leave room for that". Then "yes" (08:40): year one uses the real 2026 schedule.
3. **The college year and the portal:**
   - "When does the official cutt-off date happen ? after January right ?" (08:44). "how sure are you of that ?i  believe i received  different info. Source check please" (08:45).
   - "No i trust your second research, what about transferportal ? i'm trying to find an dynamic edge to it where they choose based on real life events" (08:48). **Supersedes** Claude's first, partly wrong dates.
   - "yes definitely get whispers, it should be part of the game just like lockerroom vibe. Coaches should also have a recruiting skill that influences outcomes or can sway players" (08:50).
   - "1. Yes, 2. Yes" (08:51): one January choice per player; bowl opt-outs.
4. **The high school year (08:56):**
   - "1. yes, make spring ball a place where if a (RS) freshman or sophomore (at max) has all the right combinations he can get like an instant ability boost... "GIve players a notice ->" if player so and so keeps mainainting x and y. He'll likely make huge steps"".
   - "2. but be careful with the pulling since we already had a lot of trouble calibrating dynasties in NCAA," (about IMG-type independents).
   - Corrections: "redshirt freshman or true freshman ofcourse." (09:03), "botj" (09:03), "use common sense, redshirt, true freshman AND sophomore" (09:04). The rule: true freshmen, redshirt freshmen and sophomores. **Supersedes** Claude's two wrong restatements (redshirt freshmen and sophomores only; true and redshirt freshmen only).
5. **Recruiting (11:55):** "yes but add aspects, like if a player is already commited and from a local school. I'll be able to do more then in the offical week days. visit HS practice etc". Then "you're instinction is right but i want to build the dynamics into the game for realism and potentially a feature where i become an NCAA coach." (11:57)
6. **The in-season week:** "yes yes" (11:58).
7. **Training slots:** "yes yes" (11:59).
8. **Pacing:** "you're wording is confusing" (12:03), then "yes and no" (12:03): regular-season climaxes only with real stakes; no holding events back. **Supersedes** the old draft's climax cap of 2 and its 5-day news hold.
9. **How long things take:** "yes and yes" (12:05).
10. **Time controls and delegation:** "yes yes" (12:07).
11. **Rule changes:** "1. no because it may cause engine breaking aspects that i can't fix in game" (12:08). "yes but make fixed possibilities that we agree upon before" (12:09). The fixed list of 12 off-field changes: "sure" (12:14).
- **Merge the spec:** "merg" ([T19] 12:21).
- **World trial 6 (2026-10-16, thread "Section 4 calendar build"):** list in `docs/trials/world-trial6-proposals.md`.
  - Part A, 17 fixes: "yes for all, continue with the situation" (11:33). Built and merged (PR #33); the next trial measures them.
  - Part B, 38 items, and 9 findings from building part A: re-checked under the two rules of 2026-10-16 (real life is the default; never ask what is already decided). 45 are settled by an earlier decision or by real life and go to the build list; the proposals file names what settles each.
  - Spring steps per player (part B 43): "2" (15:04). A player makes at most two spring steps.
  - Spring step size (part B 44): "unless i encounter unusual steps in the future, keep it as is" (15:04). Steps stay sized from the gap to the hidden ceiling.
  - Part D, 44 Debug Lab findings that had no item (2026-10-16, thread on trial 6 leftovers): 41 settled by real life or an earlier decision, listed in the proposals file. Cain's answers (23:21), and "add those to the plan as well" (23:21):
    - Spin-up before history: "agreed": 12 years instead of 6.
    - Story hooks alone lifting another team's result into the inbox: "No.". They only colour the text.
    - Continue: "Yes, anything that'd be of importance to me". It stops on Big moments that matter to the manager and on Huge moments anywhere.

## 10. Plays and tactics database

Spec `docs/design/plays-and-tactics.md`. Design in [T18], 2026-10-14.

- **Start it now:** "i assume later in the process you'll need an extremely extensive database for plays and tactics, is that right ?" ([T00] 11:08). "let's do that in the mean time or will that distort the progress ?" (11:08).
- **Topic 1, what a play is made of:** "anyway to make it more realistic ?" (11:10). Then "I do but i'm wondering why you didn't propose this initially ?" (11:12). All five additions confirmed: size from real playbooks, rule-based reactive jobs, real techniques and gap fits, play-calling languages, plays and rules per level.
- **Topic 2, counters:** "any ways to make it more realitisct >" (11:18), then "yes but i'm once again not happy that i had to give you an extra prompt before you considered those changes" (11:25). Geometry and battles decide, never a counter table.
- **Topic 3, pre-snap:** "yes" (11:25). **Topic 4, playbooks and learning:** "yes" (11:27).
- **Topic 5, the game plan (11:30):** "Yes, and it's important to me that the weights of this are rewarding good plans. Like earlier you had the issue with the HC, that it didn't have a lot off effect on the outcome. Twhen buulding this topic it's imperal that it effects get rewarded without being broken so that it incentivises learning objective knowledge of the game". A rule for the whole design.
- **Topic 6, who calls:** "Agree" (11:47). **Topics 7, 8 and 9:** "yes" (11:49, 11:49, 11:50).

## 11. Player faces (section 10, thread "Regen faces", 2026-10-16)

Spec `docs/design/faces.md`. Cain asked how AFM will make faces for fictional players (20:39). Answers at 22:04:

- **Style: GAN photo faces with a stored recipe.** "GAN-> make sure it's somehow ethnetically accurate with the names". A face's dominant heritage matches the ethnicity his name comes from.
- **New headshot every 4 years on media day:** "Once per 4 years on media day". (Asked: also on a team change; proposed, not confirmed.)
- **Mixed ancestry for faces, names keep the main group:** "yes".
- **Face recipe added at player creation on its own random stream (identical results per seed), pictures in section 10:** "yes".
- **Silhouette with initials on plain screens before section 10:** "yes".
- **Looks in the sim:** "elaborate please". Proposal 6a-6c open (appeal measured from the face; small input to social following, NIL and endorsements; small QB pay premium; never ratings or the draft).
- **Proof of concept:** "yes immediately".
- Cain's computers: a MacBook M1 and an Intel Mac (22:00). The face model must run on both.
- **Proof sample size:** "current amount is enough, just need to see a baseline" (22:43): the random pool stopped at 900 faces. Results in `docs/design/faces.md`, "Proof".
- **3D baseline for now (2026-10-19 14:12, thread "3D face model"):** "still too far from realistic but for now i'm to tired of fixing this that i'm accepting this": the ten v7 3D heads in the approved light. Face work paused.
- **Your own photos (2026-10-19, same thread):** an upload button on the player page. Any player ("Any"); media day keeps your photo ("Keep yours" on the card); a "Copy AI prompt" button ("yes but include random aspects, like bleached dreads, scar above temple, fat."); a folder import later ("agreed"). Spec `docs/design/faces.md`, "Your own photos".
- **Appearance (2026-10-19 17:00):** "yes" to a per-player list (hair and dye, facial hair, scars and a broken nose, neck and forearm tattoos, earrings, a gap in the teeth, freckles, build from weight, frame and position, changes over a career, rates from real roster headshots). Spec `docs/design/faces.md`, "Appearance".

## 12. Your career: coach at high school, college and the NFL (2026-10-18)

Cain asked in the project chat (20:55): "is it to late to pivot to an game where i can play as a CFB and HS coach ?" The coordinator answered that it is not too late (the world already simulates every level, and none of the player's own career screens exist yet) and asked on a decision card whether the coaching modes replace the NFL career or come alongside it. Cain tapped **"Alongside"** (20:56).

- **One career at every level.** A career can start at a high school, a college or an NFL team, and climb (or fall) between them the way real coaches do. The NFL mode stays as it was agreed (Q1: GM and head coach in one).
- **The college coaching mode moves out of section 11 ("Later")** into the main build. High school coaching is new.
- What it builds on, so none of it is asked again: Q2 (fired, hired elsewhere, 50+ years), Q3 (any team), Q8 (your background sets your starting reputation), Q20 (your ratings grow), Q41 (a save starts mid-February), and the coach ladder of section 2 (high school, position coach, coordinator, head coach, with buzz from titles and records). Your own moves between levels follow the same real market the AI coaches use.
- **The first playable opens all three levels at once** (21:07). Claude asked whether to open NFL and college first and add high school once its games run on the engine; Cain asked how much longer all three would take (about one more build step and two short design rounds, Claude's reading from past builds), then: "okay then we'll do all at once, update the existing roadmap".
- What it touches: sections 5 to 10 of `docs/roadmap.md` (each says how), and the gaps listed there under "What a coach career at high school and college still needs".

---

## 13. Section 6: people (thread "make new thread for designing section 6", 2026-10-18 and 19)

The design is in `docs/design/people.md`, all 15 topics.

- **The topic list** (2026-10-18 22:17): Cain asked to "redo the topic list with your new find logical thinking which should've been happening from the start", so every topic was checked against the code and the docs first (Built, Decided, New; audit in the project folder at `afm/section6/existing.md`).
- **Real life goes straight into the spec** (2026-10-19): "are you just asking me stuff to show me you did a good job ? because you know i want ultra realism and these all just feel like stuff you should know". Only choices real life can't settle are asked; unpublished numbers are marked "proposed, not confirmed".
- **Talks are picked moves and tones (A), with a huge line bank** (2026-10-19): "A, but make sure conversations-messages has a uge database so it doesn't get stale, which is one fo my hatrets of FM, been seing the same shit for 5 years". The bank covers all text: talks, media, news and fans (topic 5).
- **Your health never ends your career for you** (2026-10-19): "nah, imaging spending months on a save and it gets ripped from you". Health can force a leave, and your doctor and family push you to step down, but only you end the career (topic 13).
- **Your background gives a pool of points for your starting ratings, like FM** (2026-10-19): "A" (topic 15, settling how Q8's "changeable starting stats" works).

## 14. Section 7: scouting, recruiting and the draft (thread "start new thread to design section 7", 2026-10-19)

The design is in `docs/design/scouting.md`, 18 topics in five parts (how anyone judges a player; the NFL draft; college recruiting; the high school seat; following players). Audit in the project folder at `afm/section7/existing.md`. Cain asked for the thread on 2026-10-19 20:05: "start new thread to design section 7".

- **Built on earlier decisions, not re-asked:** Q18 and Q46 (ranges that sharpen, hints, never an exact ceiling), Q31 (director, area scouts and cross-checkers; boards only from your own reports; AI boards the same way; pro scouting the same), Q32 (insane character detail), Q33 (live draft with trades), Q34 (the undrafted rush), Q28 and Q29 (budgets, rated staff), calendar topics 5, 9 and 10, section 6's traits, trouble, media and line bank, and the career at every level (section 12).
- **Real life went straight in,** per section 13's rule. No open choices: unpublished numbers are marked "proposed, not confirmed" for the next trial.
- **Parked trial 7 items** (round 1 positions, the 5-star draft rate, trade volume) are measured after section 7 is built, not tuned in the design.

## 15. Section 8: the first playable (thread "let's get started planning section 8", 2026-10-20)

The design is in `docs/design/first-playable.md`.

- **A world library** (choice 1, 08:51: "A"): the game keeps ready-built worlds saved at February 2026; a career loads one, and new worlds build in the background or overnight. Asked whether every world would then have the same results ("it sounds to me like you're expecting every world to have the same individual game results ?"), the answer: no, each seed is a different world, and each career gets fresh randomness from its first day.
- **Section 8 builds everything your seat touches in a first season** (choice 4, 08:51: "Yes"), including build 9's later-build parts listed in topic 11.
- **High school history on the engine for the last 12 years** (choice 2, 09:00: "12 (for neow)"), after asking "7 HOURS ?" about all 36 (a one-time build per library world). Cain asked whether coaching ratings still develop in the earlier years: yes, only high school box scores and stat lines are missing there.
- **The app on Tauri** (choice 3, 09:00: "If that's truly the best option and won't lag due to rust engine"), after asking what Football Manager runs on (Sports Interactive's own C++; FM26's front end on Unity).
- Cain asked for a fresh thread to run a world with high school on the engine for 12 years once the trial 9 fixes thread is done.

## 16. Section 6b: training (thread "Section 6b design: training", 2026-10-20)

The design is in `docs/design/training.md`. Cain said "okay let's go" (09:04, in the section 8 thread) to designing 6b next. Written the section 6-8 way: real life is the default, built on Q42, calendar topics 2, 6 and 7, plays and tactics topics 4, 5 and 8, the tendencies' "work on a habit" and practice contact, people topics 1-3, 6, 8, 10 and 15, scouting topic 1 and section 8 topic 6. Cain agreed to all three choices of topic 15 (10:31: "agree to all"): growth lands every block and every week of the season for every player; a plan moves a young player's year by at most about 15% either way, and a main focus grows up to 1.5 times its share; practice snaps of your team and every college and NFL team run on the match engine, other high schools on a fitted model. Design done. Calendar parked item 1 (watching offseason work) is designed in topic 9. A fact to correct in the build: college camp is up to 25 practices in 29 days (2021 model), not 29 practices.

## 17. Section 10: screens and the 2D view (thread "Design the UI", 2026-10-20)

The design is in `docs/design/screens.md`, with a mock-up canvas of the key screens. Cain asked (10:41): "wanna design the UI while we wait for trial 9 fixes are complete ?". Written on top of section 8 (Tauri, the screen list) without re-asking it. Real life is read as how football staffs see their information (depth boards, call sheets, the All-22, Next Gen Stats, the war room) plus FM's conventions. Found in the audit: the scouting doc wrote attribute ranges on a 1-20 scale; attributes are 1-99 (2026-10-10), so ranges are on 1-99, corrected there. Five taste choices went to Cain (navigation, default theme, team colours, dots in the 2D view, section 8 building on this frame).
- **The look is Cain's own Showtime design system** (11:08, round 1 "i feel severly underwhelmed", with the system attached): "a streaming TV app: a dark room, the subject's own colors as light, frosted glass, huge outlined numbers and one big moment at a time". It lives in `ui/showtime/`. It settles choice 2 (dark main, light opt-in) and choice 3 (the subject's colours light the screen). Round 2 of the mock redraws eight screens on it.
- **Your own notes on players** (Cain, 12:55, a comment on the mock's player page: "Need a notes button so i can manually write notes"): a Notes button on every player's page opens a side panel where you write plain-text notes, saved with the game's date, private to you and kept for the whole save. Built into the mock's player page. Claude's extensions in `screens.md` topic 5 (notes on coaches, teams and games, a note mark in tables, search and a filter) are proposed, not confirmed.
- **Four more comments on the mock's player page** (Cain, 12:56-13:00):
  - Range bars: "the lower, the whiter it is, the higher the redder it is". Claude's reading (proposed, not confirmed): one fixed rating red on every page, not the team's colour.
  - "make a dropdown menu so that the other stats show to": a group panel with a dropdown for every other attribute group.
  - The reader line becomes "Opinion: Andre Lyle, WR Coach". Cain wrote "Andrew"; the mock keeps Andre, his name on the other screens.
  - "add a Dynamic Links button, i'll explain it's intentions later": the button is in the mock as a placeholder, and its purpose waits for Cain. Asked again at 19:04, he said "keep it aplaceholder now".
- **Three comments on the mock's Home** (Cain, 13:04-13:05). They are written into `screens.md` topic 8, and the parts marked there are Claude's proposals, not confirmed.
  - The hero becomes a revolving slide of recent things: this week, a short opponent report, the last game, a player of the week, recruiting.
  - The inbox box becomes tracked player results, the "my players" list with each player's latest game.
  - The storyline posters keep their structure but get immersive drawn scenes in place of the big numbers.
- **Two comments on the mock's roster** (Cain, 13:09-13:11): "This page feels like it's lacking so much information for a game. Spacing between columns feels off, and it's unclear what column fits in where, i'm seeing filter system so i can quickly check all freshhman or seniors." and "make it look like it belongs in a warm room, not a cheap mobile game". Written into `screens.md` topics 2, 3 and 7: a group row over the headers saying what each block is and whose opinion, class and unit quick filters, six views, a warm film-room look. The look's details and the six views are Claude's reading, proposed, not confirmed.
- **A pass-over with depth, especially scouting** (Cain, 13:32: "do a pass-over, i want ultra depth in certain aspects, especially scouting"). Every screen of the mock got deeper (Game Center with eight tabs from one play log, the 2D replay's read step by step, signing day hour by hour, the talk's move previews and promises), and three scouting screens were added: a recruit's full scouting file, the scouting department, and the NFL seat's draft board. The design behind them is section 7's (`scouting.md`); how they lay it out is in `screens.md` topics 7, 8, 9, 10 and 12, with Claude's picks marked proposed, not confirmed. The real 2026 NCAA rules the screens use were checked on the web: 33 fall evaluation days for a whole FBS staff, only the head coach and 10 assistants off campus, the July 2026 plan for 17 recruiters and 50 days still pending.
- **Choices 1, 4 and 5** (19:07, "agree on all", answering the three picks as listed in the thread at 19:05): a left sidebar in Showtime's frosted chrome, not Showtime's top bar with tabs; numbered dots in the 2D view, not top-down figures; and section 8 builds its screens on Showtime from day one. All five of round 1's choices are now decided.

---

## Superseded decisions

| Old | Replaced by | When |
|---|---|---|
| TypeScript web app (Claude's default, never confirmed) | Rust engine in a desktop app (Q49) | 2026-10-11 |
| Season calendar draft `season-calendar-and-pacing.md` ([T03]) | `docs/design/calendar.md` wherever they differ | 2026-10-14 |
| Q4: you coach the NFL only, college coaching later | One career that can start at a high school, a college or an NFL team | 2026-10-18 |
| Calendar draft: one potential step per checkpoint | Breakouts can lift a ceiling up to ~10 a year | 2026-10-10 |
| Calendar draft: at most 2 climax games, news held back 5 days | Climaxes only with real stakes, nothing held back (topic 8) | 2026-10-14 |
| Original vs boosted potential on cards ("Q2: both") | Shown on trial and debug pages only (Q46) | 2026-10-11 |
| Five-star busts only start after signing | Scouts misjudging is also a small cause | 2026-10-11 |
| Q5 "A, realistic names" | Q11: real names everywhere, fictional toggle at new game | 2026-10-11 |
| League sim warm-up of about 15 seasons | 30 years of hidden history | 2026-10-11 |
| Q36 "not a play-drawing editor" | Plays topic 8: a full play designer | 2026-10-14 |
| Coach strictness hidden | Visible, coaches build a reputation for it | 2026-10-13 |
| Kicker and punter mental side small | Big, plus specialization by distance | 2026-10-13 |
| History on the quick model, engine from 2026 (Claude's proposal) | Every college game on the engine, history included | 2026-10-14 |
| Trial 3 fix list result "at most 6 titles" (measured on the stand-in) | 9 of 15 titles on the engine; physics fix needed | 2026-10-14 |
| Dynasty fix option B alone | B plus linked units | 2026-10-14 |
| Calendar topic 3, Claude's first deadline dates | The second, source-checked research | 2026-10-14 |
| Spring breakouts for two class years only | True freshmen, redshirt freshmen and sophomores | 2026-10-14 |
| Decided, not yet built: 13 positions, 25 attributes in code | About 20 positions and 35-40 attributes (decided 2026-10-10) | open |

[T00]: transcripts/00-project-chat.md
[T01]: transcripts/01-map-afm-repo-toward-the-football-sim-goal.md
[T02]: transcripts/02-build-the-player-potential-and-development-model.md
[T03]: transcripts/03-design-the-season-calendar-and-pacing.md
[T04]: transcripts/04-trial-5-league-sim.md
[T05]: transcripts/05-trial-6-final-build.md
[T06]: transcripts/06-rewrite-engine-in-rust.md
[T07]: transcripts/07-full-spec-to-github.md
[T08]: transcripts/08-real-world-dataset-check.md
[T09]: transcripts/09-real-world-data-research.md
[T10]: transcripts/10-world-spec-to-github.md
[T11]: transcripts/11-build-the-world.md
[T12]: transcripts/12-coach-promotions-and-team-growth-check.md
[T13]: transcripts/13-section-3-match-engine-design.md
[T14]: transcripts/14-match-engine-spec-to-github.md
[T15]: transcripts/15-build-the-match-engine.md
[T16]: transcripts/16-section-4-calendar-design.md
[T17]: transcripts/17-world-trial-3.md
[T18]: transcripts/18-plays-and-tactics-design.md
[T19]: transcripts/19-calendar-spec-to-github.md
[T20]: transcripts/20-account-move-handoff.md
[RM]: raichel-messages.md

## 17b. Section 10 follow-up (thread "Section 10: UI and screens", 2026-10-21)

- **Claude's picks in `screens.md` confirmed** (Cain, 20:18, "yes"): every item marked "proposed, not confirmed" there is now confirmed (one rating red, notes beyond players, six squad views, stale after 1 September, made-up service names, tier names, the talk screen's tabs, the success rule, the speed targets and the rest).
- **Linking** (Cain, 20:18): "What seperates a game from a excel sheet into a a game is intuitiveness, it should be extremely calming and soothing navigating through the UI. Calming doesn't mean barebones... i want intuitive linking, i should be able to somehow go from depth chart to a progress screen back to the chart. I want you to make sure that every page has relevant hyperlinks, without becoming chaotic." Written up as `screens.md` topic 17 (rules and a link map per screen) and a Development tab on the player page (training topic 13's content).
- **Full screens** (Cain, 20:20): "i still want the screens filled, it just has to be intuitive, aeshetical and luxury. Don't take this as an excuse to add less feature or data into screens." Topic 17's rule 0.

