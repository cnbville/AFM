# Ground: the expansion (draft, 2026-11-05)

Cain, 2026-11-05 19:31 UTC: "I like the entire concept, it just feels barebones, this needs to become a DLC, not a small feature. It has a lot of potential." Earlier, 18:19 UTC, on owning your ground: "very strong, feels actually fun and gives the game an edge where time spent is worth more than pure stats. It feels like a fair grind method."

Everything below is proposed, not confirmed. Nothing goes on any list until Cain says yes to the part and names the list. How we got here: `afm/scouting-review/review-2026-11-05.md` (rounds 1-6).

## The promise

In real football the people who find players aren't the ones who watch the most film. They're the ones who know their ground: every school in their area, every coach, trainer and counselor, and which of them tells the truth. That knowledge is built one visit at a time, over years, and someone who works harder can take it away. Ground makes that the game you play every week from February: a map you work, people you win over, news that reaches you first because you earned it, and rivals who want the same ground.

Three rules hold it together:
1. Every hour always buys something. A school visit always gets the coach's list; a Friday night always shows you both rosters; a week in an area always raises your standing there. Luck decides only how good the kids turn out and when things happen to them.
2. News goes in order. When something happens to a kid, the people around him tell programs in order of their standing, and you always see the order and why you were where you were.
3. Ground buys knowledge and trust, never a signature. The kid still chooses (the built recruiting model), talent still beats ground, and potential stays hidden.

## A year in it

### College: head coach at Georgia State, year one
- **February 2.** You take over. Most of your map of Georgia is grey. The program's old ground is Known on Atlanta's south side; your own is Trusted in Birmingham, where you coached before. Georgia is Home in all of metro Atlanta, and Georgia Tech, Auburn and Alabama are Trusted in pieces of it.
- **February.** Two assistant jobs are open. One candidate is a high school head coach in Columbus, Georgia: Columbus is Home for him. The other is a better position coach with no ties. You choose.
- **February to March.** Junior days and your spring clinic. You invite coaches from the south side and Columbus, and 80 come. Each one you sit with warms to you, and your talk lands best with the coaches who run your scheme.
- **April 15 to May 31.** Your men are on the road; you can't go (FBS head coaches are barred from the spring road since 2008). Each evening your map changes: schools visited, the names on each coach's list, new leads. A Columbus coach your man has now visited twice tells him about a sophomore end who grew three inches. Nobody else has heard. At 10 PM the lead reaches you with six snaps and three blanks: his measured size, a laser time, a live look.
- **June.** Camps. The coaches who trust you bring their kids. The sophomore end measures 6'5" and runs a 4.68. You offer the next morning. Georgia hasn't heard: its area man is Home in Atlanta but has never worked Columbus.
- **July.** The dead period is your film month on the sweep's names. You speak at the state coaches' convention, and your standing ticks up in four areas.
- **August.** Preseason all-state lists come out, and everyone sees them. Two of your finds are on them, so now the race is public.
- **September to November.** Friday nights. Your men are at games, seeing both rosters. In October Georgia's area man turns up at the sophomore end's game: the news has reached them. You've been his first offer for four months, and his coach trusts you.
- **December.** Home visits and early signing.
- **Three years on.** The end is all-conference, and the services ranked him late. The Columbus coach sends you his next one. Columbus is Trusted for you now, and Georgia sends its defensive line coach there twice a month.

### NFL: head coach (with or without the final say)
- **February.** Combine interviews (built). On your scouts' map, your Southeast man is Home at three SEC schools and a stranger at the Black colleges in his region. Three of your old assistants are college head coaches, so your coaching tree gives you Trusted ground at their programs.
- **March to April.** Pro days. Your coordinators go to the big ones. Your area man goes to a Division II pro day in his region because a strength coach he's known for years told him about a 6'4" corner nobody has tested.
- **April.** The draft. Your old assistants' word on their own players is true; they don't oversell to you.
- **May.** Rookie camp tryouts, then the spring meeting. Each area scout presents his list of rising seniors. Your Southeast man's list has 14 names the scouting service doesn't have; your new Southwest man's has two.
- **June to July.** Summer film on the list.
- **August.** Joint practices with another club show you its bubble players up close. At cutdown, your pro scout's contacts at that club told you who'd go.
- **September to December.** School calls. The more standing your man has at a school, the more he's let in: practice, the staff's honest word, the trainer's word on injuries.
- **Two years on.** The Division II corner went in round 6 and starts in year two. It's on your area man's record. A rival club poaches him the next spring, and his contacts go with him.

### High school: head coach at a big school in Texas
- **February.** Signing day in your gym (SK38). Then your building: the 6 AM weight room shows who's committed. You know the basketball coach a little (Known) and the track coach not at all.
- **March to May.** Track season. After you've helped at two meets, the track coach (now Known) tells you his 400 runner, a junior who has never played football, wants to try. The basketball coach's 6'5" forward is bored. A family moving in from Louisiana calls about their son.
- **Spring.** Spring practice, where the state allows it. College coaches come through your school; the staffs that trust your word ask about your juniors, and you sell your late bloomer to the ones you've been straight with.
- **June.** You take your kids to the camps of staffs you know. The district's 7v7 director wants your quarterback on his travel team; he's a ground person with his own agenda.
- **July.** Dead period and summer workouts.
- **August.** Tryouts. The 400 runner, the forward and the move-in all show up, because you worked your building.
- **Fall.** Friday nights. The college coaches in your stands are your outward ground at work.
- **The year after.** The 400 runner signs with an FCS school. That staff trusts your word more, and next year it comes to you first.

## The parts

### 1. The map
- **College:** recruiting areas grouped the way real staffs split states (Houston, Dallas-Fort Worth, east Texas, the Rio Grande Valley; south Florida, Tampa, Orlando, Jacksonville), built from the world's own recruiting areas (`HighSchool.area`: state and county). Each area holds its high schools, its people, its events and its kids by class, plus your standing and every rival's.
- **NFL:** the scouting regions (built: eight, `world/pro/scouts.rs`, `covers`) with every college in them, down to the NAIA and junior colleges. Your coaching tree shows as dots wherever your people coach.
- **High school:** your building (the other sports, the weight room, the counselors), your district (the schools you play and their coaches), your town (families moving in), and outward the college staffs that recruit your area.
- **What you see:** a heat map of your standing against the leader in each area. Click into any area.

### 2. Standing
- One number per program per area (0-100), shown as four levels you can feel: Stranger, Known, Trusted, Home (proposed: 0-24, 25-49, 50-74, 75+).
- **Built from:** time there (visits, games, events, with a weekly cap so a flood can't buy it), how warm your people there are, how the area's kids fared with you (played, developed, graduated, treated right: the built pipeline signals), your program's pull (prestige, winning, NIL: real, but it counts for less than the work), and your staff's own ties (a man from Houston brings his).
- **Fades:** without presence it drifts down month by month, faster where rivals are working.
- **What each level gives you:**
  - Stranger: public information only (stars, box scores).
  - Known: coaches return your calls and give you their seniors' names. Their kids show in your search with a first read.
  - Trusted: you get the juniors' and sophomores' names, honest word on grades, character and injuries, and news of a breakout within days. Coaches bring their kids to your camp.
  - Home: you hear first. A coach's word leans his kid your way, families know your name, and rivals have to pay more to get in.
- **Real:** Ohio State in Ohio, LSU in Louisiana, the old line about building a fence around your state. A small program can own its backyard against a giant by outworking it.

### 3. People
- The ground is people, each a person in the world with a name, a face and a history with you: high school head coaches (in the world), their coordinators and strength coaches, private trainers and 7v7 directors (contact types today), counselors, athletic directors, local reporters, your former players living there, boosters.
- **Each one has:** an eye (how good his word is), straightness (how much he oversells; built for tips), an agenda (a trainer pushes his paying client, a coach protects his kid, a 7v7 director wants his team seen), what he wants from you (to be asked, honesty about his kids, a clinic invite, help placing his lesser kids, someday a job), and ties to other programs (his old school, his old boss).
- **What moves your relationship:** showing up; honesty (telling him his kid isn't for you, and helping place him at a friend's program); keeping your promises to his kids; how his kids are treated on your roster (playing time, development, a degree); a kid leaving you on bad terms; and your man who knew him leaving your staff.
- **People move jobs, and their ties go with them.** Real: Arkansas hired Springdale High's Gus Malzahn as offensive coordinator in December 2005, and four of the "Springdale Five", led by quarterback Mitch Mustain, committed that February. Malzahn left after one season, and Mustain left too.
- **Built on:** `world/folk/network.rs` (contacts and warmth), `world/folk/tips.rs` (straightness and agendas), the world's high school coaches.

### 4. News: how leads happen
- **What starts it:** something real happening to a kid in the world. He grows (measured at school or a camp), has a big game (the engine's box score), runs a camp, showcase or track time, posts a weight-room number (high school), changes position, gets an offer from another program, moves in or transfers, qualifies academically, or gets a new coach at his school.
- **How it travels:** the people closest to him know first (his coach, his trainer). Each one passes it to the programs he's tied to, warmest first. A program with no tie hears in order of its standing in the area. The public hears last (the services, then social media, faster from the 2010s).
- **The lead card:** the kid; who told you and what he said, in his own voice; that person's record ("4 of his 6 kids held up"); what's still blank (measured size, a timed speed, tape, a live look, grades, character); who else knows; and the order the news went ("Georgia: not yet. Auburn: Tuesday. You: Monday.").
- **Filling blanks:** each step costs hours and always fills its blank. Call his coach (character and grades, as true as the coach is straight), get his tape, see him live (an evaluation day), bring him to camp (measured and timed), get his transcript (your academic staff). The blanks are the built looks (`world/scouting/file.rs`), so a lead becomes a scouting file.
- **The paper trail:** box scores, all-state teams, camp results and other staffs' offers reach everyone at once, so leads from paper are crowded.
- **No dice decide whether you hear.** There's a little noise only in the exact day and in how true someone's word is.

### 5. Your week
- **Monday, 8 AM (the six-part day): the staff meeting at the map.** You set where each man works this week (areas, schools, events) within the era's rules: evaluation-day limits (built: 42 fall days before 2021, 33 from 2023), contact, quiet and dead periods, the FBS head coach's spring ban (2008), travel (driving or flying; far areas cost days), and the recruiting budget (travel, camps, gear; SEC programs spend millions, the Group of Five far less).
- **What a day always buys:** a school visit gets the coach's list, his tie and standing. A Friday game shows you both rosters and both coaches. A clinic warms many ties a little. Your camp measures the kids and warms the coaches who brought them. A 7v7 event or track meet, where the rules allow it.
- **What only you can do as head coach:** host (clinics, junior days, camps, official visits, your spring game), show up (fall Fridays, December home visits and school visits), call (your call to a high school coach counts for more), decide (offers, pulls, where the men go) and hire.
- **Your men's ratings matter:** work rate (visits a day), recruiting (how much each visit builds), relationships (how fast people warm to him), evaluation (how true what he brings back is), personality (a man who's coasting shows it: SK29).
- **Results come at their real hour:** road calls in the evening section, the map's changes the next morning.

### 6. Rivals
- Every program and club has standing everywhere, built from the 30 years of history: today's footprints, state pipelines, school and area pipelines and coaches' ties become their ground. The AI plans its weeks under the same rules and budgets.
- **You see who owns each area and which men hold it:** "Georgia's defensive line coach has worked south Atlanta for nine years: Home."
- **Your moves:** invade (pour time into a rival's area, as Michigan's 2015 camps did in the South), defend your home, poach (hire the man who holds an area: his ties come with him and his old program loses them), and outwork (a small program owning its backyard).
- **The AI answers:** a program losing an area sends more men, sends its head coach, or hires a local high school coach.

### 7. The ground's year
- **College:** signing day and junior days (February), clinics (spring on campus, state conventions in summer), spring practice with high school coaches watching (March to April), the spring evaluation (April to May by era; a contact period from 2024), camps and official visits (June; juniors' official visits from 2018), the July dead period, preseason lists (August), Friday nights (September to November), home visits and early signing (December, from 2017).
- **NFL:** all-star games (January), the combine (February to March), pro days including the small schools (March to April), the draft and undrafted rush (April), rookie camp tryouts and the spring meetings (May), summer film, joint practices and cutdown (August), school calls and games (September to December).
- **High school:** the offseason program and the other sports (winter), spring practice where the state allows it, college coaches through your school (spring), camps and 7v7 (June), a dead period (July, by state), two-a-days and tryouts (August), Friday nights and playoffs (fall), signing ceremonies (December and February).
- Each event shows who goes, what it costs, what it does to standing and what news it makes.

### 8. Pipelines
- When a school's kids keep coming to you and doing well, the school becomes a pipeline: its coach sends you his best first, and its kids grow up wanting your helmet (built: school, area and state pipelines in `world/program.rs`).
- **Pipelines break:** a new coach at the school (with his own ties), a broken promise, his kid buried on your depth chart, or your man leaving (Springdale and Arkansas after Malzahn left).
- You see each pipeline's health and what's straining it.

### 9. Finds
- A kid you heard about before the public and before every rival is a find, credited to whoever's ground it was: your man or you.
- **When the world catches up** (stars, offers, all-conference, the draft), the find blooms. The news names you, your staff respects you more, the coach who told you trusts you more, and your name as a finder spreads among coaches and athletic directors.
- **A yearly ground report:** finds, finds you signed, blooms, the good players nobody of yours saw (SK66), standing gained and lost by area, and who outworked whom.
- Your coach profile keeps the career count.
- **Real:** Buffalo was Khalil Mack's only FBS offer, and he went fifth in 2014. J.J. Watt walked on at Wisconsin. Bill Nunn's ties at Black colleges gave the Steelers John Stallworth (round 4, 1974; Nunn stayed behind to re-time him) and Donnie Shell (undrafted, 1974).

### 10. Your staff's ground
- Each man carries his own ground: his home area, his old schools, his years there. Hiring him buys it; losing him takes most of it away (the program keeps what the schools tie to the program itself).
- **Careers run on ground:** a man who owns Houston gets hired away. Your recruiting coordinator and personnel staff work the ground on campus (visits, camp lists).
- **You choose:** give a man his home area (fast) or a new one (slow), and one man per area or overlap.
- **The college assistant seat (SK76, agreed) is the pure grind seat.** You are the area man: your area is your career, your head coach signs off on your kids, and your finds get you the coordinator job.

### 11. NFL ground in depth
- **School calls:** your area scouts' standing at each college (built: SK9 contacts; SK19 openness also gates it) decides what they get on a call: practice, the staff's honest word, the trainer and strength coach (character and medical), early word on underclassmen.
- **The spring meeting (May):** each area scout presents his list of rising seniors. The names he had that the scouting service didn't are his finds.
- **Your coaching tree:** colleges where your old assistants and friends coach are Trusted ground, and their word on their players is true. Real: Saban was Belichick's defensive coordinator in Cleveland, and New England took Hightower (2012) and Barmore and Mac Jones (2021) from Alabama. This builds on G2, the coaching tree you build.
- **Pro ground:** other clubs' staffs that your pro scouts know (who's about to be cut), and agents (built, with their agendas).
- **Your own field:** rookie camp tryouts (May) and joint practices (August).
- **The GM seat:** the department's map is yours to set.

### 12. High school ground in depth
- **Your building:** the other coaches (basketball, track, baseball, wrestling) and the PE teachers, each a person with a tie to you. How much they share their athletes is a real fight at many schools. The weight room shows commitment. You only learn about the kids in the halls as the building comes to trust you.
- **Your district:** rival high school coaches (transfers, with SK42's risks), the 7v7 directors and trainers who steer kids (built: C18), and families moving in.
- **Your outward ground:** the college staffs that trust your word (built: credibility), the camps you send kids to, and the all-star selectors. It decides whether your kid gets looked at.
- **Feeder middle schools (L14),** where the world has them.

## Also part of it (no question needed)
- **Screens (mock-ups first, Cain's rule):** the ground map; an area page; a person page; the leads board and lead card; the week planner; the ground report, finds ledger, pipelines and rivals pages.
- **The world:** the same ground for every program and club from 30 years of history, so on day one the map already has owners. The AI's recruiting reads its ground where it now reads footprint and pipelines, and today's checks against real data (where FBS signees come from) still hold. Memory and speed are measured in the build against the 8 GB M1 target.
- **Eras:** before Hudl (2006), tape was mailed and ground mattered even more. Hudl put film everywhere. In the Twitter era kids post their offers (2010s), so news reaches the public faster. The 2017 early signing period and the 2018 junior official visits move the year earlier. NIL (2021) adds collectives and handlers as ground people. The portal (2018 on) adds ground among college players (lower-division coaches: C15, built). The 2023-24 calendar changes the spring.
- **Balance:** returns shrink for repeated work in one area within a week, so spreading out against focusing is a real choice. Talent beats ground: being Home doesn't sign a kid who wants to play elsewhere. A rival who outworks you passes you. Every result says why.
- **Stories:** the storyline engine (agreed, update 2's list) gets ground stories: the turf war for Houston, the coach who stopped calling, the pipeline that broke, the find who bloomed, the area man who got poached.
- **Round 1's 15 items** are the wiring under it (a day out covers the place, the spring sweep, the coach's list, your own database, first grades, everyone at your camp, showcase leads, NFL write-ups, coverage, the weekly report, misses, ties, tips when a kid breaks out, the film queue).

## What it builds on in the code
`HighSchool.area` (state and county); `Program.area_pipeline`, `hs_pipeline`, `pipeline`, `footprint` (`world/program.rs`); `world/scouting/dept.rs` `area_coach` (your areas); `world/folk/network.rs` (contacts and warmth); `world/folk/tips.rs` (straightness, agendas, why the public missed him); `world/scouting/file.rs` (a lead's blanks as looks); SK9 area contacts; SK19 openness; the six-part day; the storyline engine.
