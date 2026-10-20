# Section 6b: training

Design written 2026-10-20 (thread "Section 6b design: training"), the way sections 6, 7 and 8 were: what is built and decided first, then each topic as the finished real-life design. Real life is the default (project ground rule). Numbers no source publishes are marked "proposed, not confirmed". Three choices real life can't settle are at the end; Cain agreed to all three on 2026-10-20. Design only: nothing here is built yet.

Topic order: 0 what exists, 1 the practice day, 2 the week, 3 the year and its rules, 4 the time budget, 5 the individual plan, 6 who teaches, 7 load, fatigue and practice injuries, 8 practice as something you watch, 9 the offseason on their own (calendar parked item 1), 10 high school, 11 the AI's training, 12 how growth lands, 13 what you see, 14 what the build must prove, 15 the three choices (confirmed).

## Topic 0. What exists

**Built** (code):
- The yearly development step (`development.rs`, `develop_season`): growth toward hidden ceilings, scaled by coaching (0.78 at 1 to 1.25 at 20, `coaching_mult`), coachability, work ethic, playing time, level of competition, form, injuries and age windows. Poor teaching and a scheme that doesn't fit cost ability (fix list 4.6). Redshirts and practice squad players grow from practice only (`PlayingTime::Redshirt`, `PracticeSquad`). A skill only an old position used fades.
- College's yearly step lands at fall camp (`world/college.rs`, `start`: "one checkpoint a year until training, section 6b, splits it").
- Spring breakouts in college spring practice and high school spring ball, read off the position coach's teaching and open reps (`world/hs_year.rs`, `spring_mix`). Early enrollees' spring step (`open_row_with_spring_step`).
- Bowl practices: up to 15, about five a week, finals costing two, a small step for young players (`college::bowl_practices`, `development::extra_practices`).
- The calendar's training blocks at every level, posted as events (`nfl_year::training_blocks`): NFL phases 1-3, OTAs, minicamps, camp with acclimation, in-season Wednesday to Friday with 14 padded practices (`calendar/rules.rs`, `nfl_padded_practices`); college winter, 15 spring practices, summer, camp, 20 hours in season; high school openings by state. The weekly rhythm lines (`clock.rs`). Heat days on August high school practices logged, "the step on players waits for training" (`hs_year.rs`).
- Playbook knowledge per concept family from reps and football IQ, the install from days since the last game, line chemistry, film (`world/systems.rs`, `game/roster.rs` `knowledge`; `docs/world-model.md`, "Plays and tactics in the world").
- Practice reads of a designed play against the scout team (`game/designer.rs`, `practice`).
- Tendencies with yearly change, six coaching styles and coaching pull (`tendencies.rs`).
- Coaches' hidden teaching by position group (`world/coach.rs`), teaching credit schools can see (`staff::credit_teaching`).
- Fatigue and injuries from the play in games (`game/flow.rs`).

**Decided** (docs):
- Q42: weekly training focus per unit plus an individual plan for each player, with far more depth (roadmap; decisions section 1).
- Calendar topic 6: each day has its decisions (game plan Tuesday, practice plan and reps Wednesday to Friday); a game plan has an install limit; short weeks install less.
- Calendar topic 7: the blocks above; each block favours a different kind of growth (body in the offseason, technique and scheme in OTAs and spring, reps and battles in camp, maintenance in season); training load cuts both ways (more growth, more soft-tissue injuries and tired legs); veterans' rest days are a real call. Calendar topic 2.1: the CBA's offseason limits bind your training.
- Plays and tactics topic 4: players learn parts, three depths of knowing, learning from reps, meetings, film, walkthroughs and study; real practice limits cap the install; topic 5: the real practice week, the scout team matters; topic 8: new plays cost practice time and give a noisy practice read.
- Tendencies: "work on a habit" in each player's individual plan; practice contact (physical or protective style) comes with 6b.
- People: work ethic is effort in training; mood scales practice effort (up to 10% of growth), team mood up to 5%; mentors give a small push in practice; other sports split a high schooler's training time; birth and funeral absences cost practices; the performance staff (strength, speed and conditioning, injury prevention, nutrition) replace the facilities stand-in for the physical side, and the medical staff run rehab; your rulebook includes weight targets; staff do their job daily and leave reasons; your read of your staff comes from their work.
- Scouting topic 1: your own players are read daily through practice by each position coach's eye.
- Section 8 topic 6: until 6b is built your staff run training as the AI does; 6b adds your control.
- Development model: potential is hidden; ceilings drift with coaching, work ethic and playing time.

**New here:** how a practice is built and what each part grows; the time budget; the individual plan; practice contact and practice injuries; the step on players from heat and contact rules; practice as something you watch; the offseason on their own; the AI's plans; development split across the year.

**A fact to correct in the build:** the calendar posts "NCAA: 29 camp practices". The real rule since 2021 is up to 25 practices inside the 29 days before the first game, with a 7-day acclimatization period (it was 5), at most 18 contact practices and 9 in full pads, two scrimmages, no full contact on more than two days in a row, 75 minutes of full contact a session at most, and the Oklahoma drill and other straight-line drills banned (NCAA proposal 2021-5, adopted; two-a-days ended in 2017).

## Topic 1. The practice day

### How real football works
- **A practice is a script of periods,** usually 5 minutes each, posted on a sheet by the minute (the head coach's "practice schedule"). A typical NFL or college practice runs 2 to 2.5 hours: warm-up and stretch, special teams, individual (each position group with its coach, technique drills), group (inside run against the front seven, 7-on-7 for the passing game, one-on-ones like receivers against corners and linemen against rushers), team (11-on-11, the starters against the scout team or ones against ones), situations (red zone, third down, two-minute, backed up), and conditioning at the end.
- **Gear sets what is allowed.** Helmets only, shells (helmets and shoulder pads), full pads. Some periods are walkthroughs at no speed.
- **Contact has real levels.** USA Football's and the NFL's words: air (no bodies), bags (pads and dummies), control (thud at a set speed on one player), thud (full speed, no taking to the ground), live (game speed, to the ground). Most practice snaps are thud. Tackling to the ground is rare at every level now.
- **Reps are split.** The "ones" (starters), "twos" and the scout team, which runs the opponent's plays from cards. In camp a coach gives a battle's two players equal reps; in season starters get most team reps.
- **Off the field:** meetings (team, unit, position), film, walkthroughs, the weight room, treatment, and study at home (tablets since about 2011).
- **Tempo:** some staffs practise fast with no huddle (Chip Kelly at Oregon and the Eagles ran plays every 12 to 15 seconds in practice), others slow and exact. Fast practice gives more reps, more fitness and more wear.
- **Innovations are real and spread:** Dartmouth's Buddy Teevens stopped players tackling each other in practice in 2010 and brought in a robotic tackling dummy (the MVP, 2015); the Ivy League ended full-contact hitting in in-season practices in 2016. Pete Carroll's Seahawks taught rugby-style shoulder tackling with little live tackling.

### The design
- **A practice is a list of periods,** each with a length in minutes, a kind (warm-up, special teams, individual, group, team, situations, walkthrough, conditioning, meeting, film, weight room, treatment), who is in it (units, groups or named players), gear, contact level (air, bags, control, thud, live), tempo and its content (which drills, which plays, which situations, which opponent looks for the scout team).
- **Each period grows what it works, and only that.** Individual periods grow technique attributes of that group (a lineman's hand use and footwork, a receiver's release and route running, a corner's press and backpedal). Group periods grow the battle skills and knowledge of the concepts run. Team periods grow knowledge, timing, reading, line chemistry and the game plan's install. Situations grow situational awareness and the call sheet's prep. Conditioning grows stamina and recovery work. The weight room grows strength, power and body. Film and meetings grow knowledge and reading with little wear. The attribute families are those of `docs/design/attributes.md`.
- **Contact level trades realism of the rep for wear.** A live rep teaches tackling, block shedding and ball security best and costs the most (concussions, ankle and knee injuries); thud teaches most of it at a fraction of the risk; air and bags teach steps and assignments only. This is the tendencies' "practice contact": a physical staff practises more live and pulls contact willingness and big hits up, a protective one tackles less and pulls toward sure, shoulder tackling.
- **Reps are counted per player,** and a player's growth from a period follows the reps he really got. A twos tackle gets fewer team reps than the starter, a scout team player's reps run the opponent's plays, so they grow his general skills and his effort but not his own playbook.
- **Tempo** adds reps per minute and wear per minute.
- **A practice day is limited by the rules of its level and date** (topic 3): a period the rules forbid can't be saved, like an illegal play in the designer.

## Topic 2. The week

### How real football works
Built and confirmed in calendar topic 6: the NFL's Monday film and lift, Tuesday off, Wednesday base install, Thursday third down and red zone, Friday situations and two-minute, Saturday walkthrough and travel; college's 20 countable hours with one day off, Tuesday to Thursday practice around classes; high school film Monday, practice Tuesday to Thursday, game Friday, JV and film Saturday. Plus:
- **NFL padded practices:** 14 in the regular season, 11 of them in the first 11 weeks and 3 in the last six, at most two in one week once (in the first 11 weeks), and three hours on the field on a padded day (CBA Article 24). Playoff teams may hold one a week. Most teams pad once a week early and rarely late. The bye gives players four days off (CBA).
- **College players who didn't travel or play** often get a short developmental practice on Sunday or Monday while the starters recover ("the young guys' practice"), inside the 20 hours.
- **Short weeks** (Thursday night) mean Monday is a walkthrough and there is almost no padded work; long weeks after Thursday add rest.
- **The scout team** gets the next opponent's cards on Wednesday; a good scout team quarterback who can imitate a mobile opponent is worth a lot that week.

### The design
- **A week is a set of days, each with its practice,** made from templates by day type (game minus 4, game minus 3 and so on), so the same template follows the game from Sunday to a Thursday night without being rewritten. Built on the calendar's daily clock.
- **The game plan and the practice plan are one schedule.** The install (plays and tactics topic 5) is the content of the team and situation periods: a play not repped this week is less prepared on game day, exactly as the built install limit says.
- **Young players' developmental periods** are real time inside the limits (college's Sunday or Monday practice, the NFL's young player periods during the week, high school's JV practices), growing players who don't play.
- **Short weeks, byes, travel and time zones** come from the calendar; the rules decide what each kind of week allows.

## Topic 3. The year and its rules

### How real football works
- **The NFL (CBA, 2011 and 2020).** A nine-week offseason program, four days a week. Phase 1 (two weeks): strength, conditioning and rehab only, with only strength coaches on the field. Phase 2 (three weeks): individual and group drills, no offense against defense, no helmets. Phase 3 (four weeks): up to 10 OTA days and the three-day mandatory minicamp, 7-on-7 and 11-on-11 without live contact, no pads. A club with a new head coach may start two weeks earlier and hold an extra voluntary minicamp. Breaking the rules costs OTA days and fines: the Seahawks lost OTA days in 2012 and again in 2014, when Pete Carroll was fined, and the Jaguars were fined $200,000 and Urban Meyer $100,000 in 2021 for contact in OTAs. Rookies report early to camp. Camp since 2020: a five-day acclimation without pads, then up to 16 padded practices, no more than three in a row, one padded practice a day, and walkthroughs. Veterans who skip the voluntary parts are allowed; skipping mandatory minicamp or camp is fined (built: holdouts and hold-ins).
- **College (NCAA Division I, FBS and FCS).** In season 20 countable hours a week, 4 a day, one day off. Out of season (winter, after spring) 8 hours a week, of which at most 2 may be film, the rest strength and conditioning. Summer: 8 hours a week for up to 8 weeks, up to 2 of them film and walkthrough (the summer access rules of the 2010s). Spring: 15 practices over 34 days, 12 with contact, none before the third, 8 with tackling, at most 3 mostly 11-on-11 scrimmages, no full contact on back-to-back days and at most 75 minutes of full contact in a session other than a scrimmage (2022 rule). Camp: the 2021 model in topic 0. Bowl practices: built. Divisions II and III have their own, smaller limits.
- **High school (each state).** Heat acclimatization (the NATA guidelines, adopted in some form by most states: the first days helmets only, then shells, pads from about day 6, limits on two-a-days and a three-hour cap; built as a heat rule per state). Full-contact limits by state (Texas's UIL limits game-speed tackling and blocking to the ground to 90 minutes a week from the week of the first game; California's 2014 law allows two full-contact practices a week of at most 90 minutes each in season and none in the offseason; about 20 states had a limit by 2014, more since). Spring ball where the state allows it (Texas, Florida, Georgia and others; built), summer 7-on-7 leagues and camps, dead periods, and the offseason weight program the school's coaches run.

### The design
- **Every level and era runs on its real rule book,** read from `calendar/rules.rs` by date: what kinds of periods are allowed, gear, contact, hours, days off and counts. Rules change with the era (no NCAA hour limits before 1991's 20-hour rule, two-a-days to 2016, the 2011 CBA's cut of OTAs and padded practices), so a 1998 camp has two-a-days in full pads.
- **Breaking a rule is possible where real teams break it** (contact in OTAs, extra "voluntary" workouts that aren't), and it is found out and punished as in real life: a league or NCAA case, lost practice days, fines, a coach's suspension, and the media story. A hard rule the software can't break doesn't exist in real football, so it doesn't here; detection follows the real ways (a player complains to the union, a video, an assistant who leaves).
- **Each block favours its growth** (calendar topic 7, confirmed): the offseason grows the body; spring and OTAs grow technique, the scheme and position moves; camp settles the depth chart through reps and battles; the season mostly keeps what is there and prepares each game. This comes from what the rules allow in each block, not from a flat multiplier: in Phase 1 only the weight room and conditioning exist, so only the body can grow.

## Topic 4. The time budget

### How real football works
Every coach says the same thing: there is never enough time. Each minute goes to the game plan, to fundamentals, to young players, to special teams, to conditioning, or to rest. More install means less teaching; more padded work means more wear; resting veterans means fewer reps for their unit. Staffs differ most in how they spend the minutes (Nick Saban's Alabama was known for exact, fast practice scripts; Bill Belichick for heavy situational work).

### The design
- **The rules give the minutes, and every minute goes somewhere.** No slider gives growth for free: a period added to a block takes the time from another, and the limits of topic 3 bind.
- **The trade-offs come from the battles and the body, never from a flat bonus:**
  - Install against fundamentals: more of the week on this opponent prepares the plan (knowledge and prep for Sunday) and grows less technique; more fundamentals grow the players and leave the plan thinner.
  - Starters against young players: reps to the twos grow them and leave the starters a little less sharp.
  - Contact against wear: live and padded work grows the contact skills and costs bodies (topic 7).
  - Tempo against freshness.
  - Conditioning against fatigue: a fit team fades less late in games (the built fatigue), but hard conditioning in season leaves tired legs on Sunday.
  - Rest against rust: a rested veteran is fresher and gets a little less sharp on this week's plan.
- **Players absorb only so much.** The learning in a day falls off after a point that depends on football IQ, work ethic and age (proposed, not confirmed: the curve's shape), so ten periods of the same drill do less than ten different ones, and an over-full install produces the built mental errors.

## Topic 5. The individual plan

### How real football works
Position coaches give each player one or two points of emphasis for a block ("finish your blocks", "eyes on your keys"), the strength staff gives each a body plan (gain 15 pounds, cut body fat, rehab a shoulder), and the head coach can move a player to a new position (Jason Peters went from college tight end to an undrafted tackle on the Bills' practice squad in 2004; Antonio Gates from basketball to tight end). Players also add their own work: extra film, extra throws after practice (Peyton Manning's routine), JUGS machine catches.

### The design
Each player's plan, set by his position coach (and the strength coach for the body part) unless you set it, holds:
- **A main focus:** one attribute or skill family (hand use, release, ball skills, reading the defense, tackling form, ball security, a kicker's directional punting). The focus gets the larger share of his individual periods, so it grows faster and the rest of his technique a little slower (up to about 1.5 times its share, choice 2, confirmed).
- **A habit to work on** (tendencies, decided): one tendency coached toward a target, at the built coaching pull times the extra time spent on it, with obedience and coachability deciding how much sticks. A slow back who bounces can be drilled to hit the hole.
- **A position move,** learning a new spot or a second one (the built position change, now learned over practice time, so a move in spring is ready for camp and one in October isn't).
- **A body plan:** a weight target (your rulebook's, decided in people topic 8) with the dietitian, strength or speed emphasis, and a real rate (proposed, not confirmed: about 1 to 2 pounds a week at most when gaining with a nutrition plan, most college freshmen linemen gaining 10 to 25 pounds in a year). Weight changes the body numbers as built (`body.rs`): heavier is stronger and slower.
- **Extra work** he does on his own: more film, extra throws or catches. Whether he really does it comes from his work ethic and professionalism (people topic 1), and you see it only through what your staff notice.
- **Rehab** when hurt, run by the trainers (topic 7).
- **A mentor** (people topic 2, decided) adds to his plan's growth through the built small push.

A player's plan can be refused or half-done: a veteran star with ego may ignore a focus he doesn't believe in, and his trust in you moves with it (people topics 3 and 5).

## Topic 6. Who teaches

### How real football works
Position coaches teach their group in individual periods; coordinators run group and team periods and the install; the head coach runs the schedule (and at a small high school, coaches a group himself). Strength coaches own the weight room and the summer. Graduate assistants and quality control coaches run the scout team and draw its cards. Specialists often have no full-time coach at small programs, and NFL teams hire kicking consultants.

### The design
- **Each period is taught by the coach in charge of it,** and his built teaching for that group scales what the players take from it, through each player's coachability (as the built step does). A strong position coach with a weak coordinator gives a group that grows its technique well and runs a poorly installed plan.
- **The scout team's quality** comes from the staff who run it and the backups' own imitation (plays and tactics topic 5, decided): it scales how well the starters are prepared for the opponent's looks.
- **Your own teaching counts where you coach.** If you, as head coach, run a group yourself (a high school head coach coaching the quarterbacks, or Sean McVay-style with the offense), your teaching rating (people topic 15) is the coach of that period, on the same scale as every coach. Your plan choices are rewarded through the battles, like your play calls (plays and tactics topic 6.4).
- **A group with no coach** (a high school without a kicking coach) learns from the head coach's teaching for it, usually low, or from a private kicking coach the family pays for (built money background).

## Topic 7. Load, fatigue and practice injuries

### How real football works
- **Most college injuries happen in practice by count, games by rate.** The NCAA Injury Surveillance Program measures a game injury rate many times the practice rate per player exposure, but there are so many more practices that a large share of all injuries happen in practice, the most in preseason camp.
- **Sudden load breaks bodies.** After the 2011 lockout cut the offseason program, about a dozen Achilles tendons tore in the first weeks of camp (as reported at the time). Hamstring and groin strains cluster in camp and early season.
- **Contact drives concussions.** Most practice concussions come from full contact, which is why the rules above cut it.
- **Teams monitor load.** GPS trackers (Catapult and others) on every NFL and most FBS players measure distance, high-speed running and accelerations; sports scientists flag a player whose load jumps, and staffs cut his reps. Veterans get days off (many teams give older players a midweek day off).
- **The injury report comes from practice.** NFL players are listed did not participate, limited or full each day from Wednesday; how a player practises decides his game status.
- **Coming back has steps:** rehab with the trainers, then individual drills, limited team work, full practice. PUP and NFI lists in camp, and the IR return window (21 days of practice before activation, built).

### The design
- **Every practice rep carries injury risk** from what the rep was: contact level, gear, tempo, his fatigue, his load against what his body is used to, the heat, the turf, and his injury proneness (proposed, not confirmed: risk per rep set so the practice share of all injuries and the camp peak land on the NCAA ISP's real split and the NFL's real camp counts).
- **Each player carries a running load,** the work of the last days against the last weeks. A sharp rise (a rookie in his first camp, a player back from injury, a team after a lockout) raises soft-tissue risk; steady build-ups lower it. The performance staff's injury prevention rating (people topic 10) is how early they see it and how well they shape the load.
- **Fatigue carries into games.** A heavy Thursday leaves less energy for Sunday's first quarter (the built game fatigue starts lower), and late-season wear adds up.
- **The injury report is what happened at practice.** A player limited by the trainers is listed limited; one who sat is listed out of practice; Friday's game status follows (calendar topic 6, decided).
- **Rehab follows the steps above,** each with the trainers' reads (the medical staff's return estimate, Q39), with a player who rushes back raising re-injury risk (built playing through pain).

## Topic 8. Practice as something you watch

### How real football works
Camp is when jobs are won: beat writers post daily "camp observations" of one-on-ones and team periods, a backup who keeps winning gets more reps with the ones, coaches grade every practice snap on film, and "practice players" who don't carry it to games are a known type. NFL camps are open to fans on some days and Hard Knocks shows one team's camp.

### The design
- **Team, group and one-on-one reps are real battles.** Where the practice snaps run on the match engine (choice 3, confirmed), each rep is a play with the period's gear and contact rules (thud: the play ends at contact, no tackle to the ground), so practice wins and losses come from the same attributes as games. Where they don't, the same numbers come from a model fitted to the engine.
- **The staff grade practice.** Each position coach grades his players' practice reps through his eye (scouting topic 1, built daily read), and the grades feed the depth chart battle in camp: a player who wins reps earns more reps with the ones. Practice grades carry noise from a few reps, as the play designer's practice read does.
- **Practice players and gamers.** Consistency and temperament (hidden traits) and his mood make some players better in practice than in games and some the other way, so the staff's practice grades can mislead.
- **You can watch any period** in the 2D view (section 10) or read its summary: the period's reps, who won the one-on-ones, the coach's notes, the injuries.
- **The media watch too.** In the NFL and big programs, camp reports and open practices produce news (the built news and loudness), and a starter losing reps becomes a story.

## Topic 9. The offseason on their own (calendar parked item 1)

Raichel, 2026-10-14: "i'm interested in building an creative way wherre we can see and montiors player work in the off-season."

### How real football works
- **The NFL summer break (about five to six weeks) and the February to April gap are the players' own.** Clubs can't require work, and most players train with private trainers: quarterback coaches (Tom House, Jordan Palmer's camps), pass-rush specialists, speed coaches, home-town gyms, or at the facility with the strength staff. Throwing sessions with receivers that the quarterback organises are common. Some players do nothing much.
- **Coaches learn about it in bits:** the strength coach's check-ins and the facility log, players' and trainers' social media workout videos, what agents and teammates say, and finally report day: the weigh-in and the conditioning test (failing it sends a player to the non-football injury list or a fine).
- **"The best shape of his life"** is a camp cliché because every player says it. The proof is the conditioning test and the first padded week.
- **College is different:** the 8 summer hours are the staff's, so the strength coach sees each player's lifting numbers and runs, and the rest of the summer the players spend in classes and on campus, with the staff's eyes.
- **Contract years and new deals change the summer.** A player in a contract year tends to work hardest; one who just got paid sometimes doesn't.

### The design
- **Every player's offseason is his own choice,** from his work ethic, professionalism, ambition, his contract situation, his mood, his life away from football (people topic 6) and his money (a private trainer costs money a minimum-salary player may not spend). He chooses where (home, the facility, a private trainer of real quality), how much and on what.
- **What you see depends on your level's rules and your staff:**
  - **NFL:** a feed of signs through the break: the facility log (who came in and when), the strength coach's check-ins, workout clips players post, a teammate's or agent's word, a private trainer's name. Then report day: the weigh-in against his target, the conditioning test, and the strength coach's first read. Each sign is only as true as the person giving it: a player's own clip shows his best day.
  - **College:** your strength staff's numbers from the 8 summer hours, attendance, and the summer's discipline problems (people topic 7).
  - **High school:** the school's weight program attendance, 7-on-7 and camp appearances, and what you hear in town.
- **What you can do:** set the offseason program the rules allow (the voluntary program, the summer hours), suggest a trainer, check in by talk (people topic 5: a call can motivate or annoy), put a weight clause in his contract (NFL weight clauses are real, built contract terms) and set rulebook targets. A summer of hard work shows as growth on report day; a lost one as weight, rust and a soft-tissue injury in camp.

## Topic 10. High school

### How real football works
A high school week is shorter and poorer: two or three hours a day after school, coaches who teach all day, often no athletic trainer (31% of public schools have none, the CATCH ON study), one varsity practice field shared with the JV and the band. Many athletes play other sports, and track helps speed. State rules set the heat days, contact minutes and spring ball. Big programs (Texas 6A, Florida, Georgia, California's top schools) look like small colleges.

### The design
- **The same practice model at its real size:** shorter practices, fewer coaches (the head coach and two to four teacher-assistants at a small school, built people topic 10), the JV and freshman teams sharing fields and coaches, the state's heat and contact rules, and spring ball where allowed.
- **Heat days have a step on players now:** a cut or moved practice is minutes lost from the budget; one that goes ahead in the heat raises fatigue and heat illness risk (the built heat rule's "waits for training" closes here).
- **Other sports split the year** (people topic 6, decided): a basketball season costs winter lifting; track grows speed and acceleration a little and costs strength work; baseball competes with spring ball.
- **No athletic trainer** means injuries are found later and load isn't watched (people topic 10, decided).

## Topic 11. The AI's training

### How real football works
Every staff has its own way: Saban's process, Belichick's situations, Carroll's competition and little tackling, Harbaugh's physical camps, Kelly's tempo, programs that redshirt and develop against ones that play the best now.

### The design
- **AI staffs build every plan with the same tools you have,** under the same rules: templates from the head coach's styles (physical or protective sets contact, structure or freedom sets the drill mix, mould or adapt sets how much each player's plan follows the system), the coordinators' install from their ratings (built), position coaches' focuses from their read of each player (built daily read), the strength coach's body plans, and the trainers' rest calls.
- **They decide like real people and leave a reason** (calendar topic 10, decided): "we went live twice this week, we tackled poorly Saturday", "rested Kelce, his load is up". Good staffs spend minutes well, poor ones waste them (too much install, too much contact, no young player periods).
- **Your staff's default plan is the same AI** (section 8 topic 6, decided): hand off any part and the staff run it by their ratings and styles.

## Topic 12. How growth lands

### How it works today
One yearly checkpoint at fall camp (college) and at each level's season start, plus spring breakouts and bowl practices.

### The design
- **Growth lands as the work happens** (choice 1, confirmed): each block's and each week's practice adds its growth to the attributes it worked, by the rules of the built step (ceilings, age windows, coaching, coachability, work ethic, form, injuries, playing time read as real reps and snaps).
- **The yearly calibration stays.** The staff's default plan of an average staff gives the same year's growth in expectation as today's yearly step, so the real-world checks (`national_class.rs`, the draft, the level ratings) still hold. Training choices move a player around that (choice 2, confirmed).
- **The yearly events keep their place:** ceiling drift and the body that stops growing at the year's turn, ageing in the offseason, form drawn for the season, breakouts showing in spring, camp or a hot season, slumps in season.
- **Game snaps still teach most** of the mental and technique skills, as built; practice adds what the reps and the teaching give.
- **Potential stays hidden.** Training moves current ratings toward ceilings; it moves ceilings only through the built drift (coaching, work ethic, playing time).

## Topic 13. What you see

- **Your staff's words and reads, never a growth number:** each position coach's weekly notes on his players ("his hands are quicker", "he's not getting it"), practice grades, the strength coach's numbers (lifts, speed tests, weight), the trainers' load flags and injury notes. Your read of a player's ratings is your staff's read (scouting topic 1), so a poor coach can tell you a player improved when he didn't.
- **The schedule** of every day, block and week, editable to the period, with each period's rules check.
- **Each player's plan** and how he's taking it (effort tells, people topic 3).
- **Install progress** per concept family (plays and tactics topic 4, decided).
- **At the end of a block** a short report from each department head: what was done, what grew, who's hurt, who's behind.
- Exact growth and true potential stay in debug and trial tools only.

## Topic 14. What the build must prove

Match and development checks only, no world runs (Cain's trial rule), each timed:
1. **The default plan keeps the calibration:** a national class on the default plans lands on today's yearly growth and the real checks in `national_class.rs`.
2. **Each period grows only what it works:** a block of individual line periods grows linemen's technique and not their reading; Phase 1 grows only the body.
3. **Choices move growth within the agreed size** (choice 2, confirmed) and never past ceilings.
4. **The trade-offs show through the battles:** more install raises the plan's prep and lowers technique growth; more live contact raises tackling growth and injuries; a heavy Thursday lowers Sunday energy.
5. **Practice injuries land on the real split:** the practice share of injuries and the camp peak against the NCAA ISP and the NFL's camp counts, and a load spike raises soft-tissue injuries.
6. **The rules hold:** no illegal period saves, and each level and era's counts (padded practices, contact sessions, hours) are never passed by the AI.
7. **The habit focus coaches a tendency down** at the expected speed with obedience.

## Topic 15. The three choices (confirmed)

Cain, 2026-10-20 10:31: "agree to all".

1. **How often growth lands:** every block and every week of the season, for every player in the world, so the AI and you train by the same step (a few seconds a season on 4 cores, to be timed in the build).
2. **How much a plan can move a year's growth:** the best realistic plan against a poor one moves a young player's year by at most about 15% either way, less than the gap between a great and a poor coach (built: about 25% either way). A main focus grows its skill up to about 1.5 times its share, and the rest of his skills pay for it.
3. **Which practice snaps run on the match engine:** your team's and every college and NFL team's team and group periods (about 6 more minutes a season on 4 cores, to be timed in the build). High school teams other than yours use a model fitted to the engine.
