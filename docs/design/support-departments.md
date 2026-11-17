# Support staff as full departments

World additions item 4 (thread "Find what the world is still missing", 2026-11-17). Proposal for Cain, 2026-11-17. **Agreed in full 2026-11-17** (Cain, 16:53 UTC: "agree on all", all seven questions below). Not built: one lane after the world rebuild.

## What it does in the game

Today each team has five support heads, one man each. After this, each head runs a real department of people: assistants, a dietitian, sports scientists, assistant trainers, doctors, a behavioral health clinician, tutors, analysts and recruiting staff. Each person has the ratings his job needs. The department's head sets how good the work is. His people set how many of your players get it.

What you feel:

- A rich FBS program with five trainers finds the backup safety's hamstring on Tuesday. A poor one with one trainer finds it when he pulls up lame on Saturday.
- A great dietitian gets your 285-pound freshman tackle to 305 a semester sooner. She never raises his ceiling.
- A sports scientist flags your tailback's load before the strain, and the trainer's return dates hold more often.
- A behavioral health clinician brings a player back to himself sooner after a death in the family or a public benching.
- An assistant trainer you raised becomes someone else's head trainer, and you lose him.
- Small schools feel thin. A Division III program has one trainer for every sport, and a third of public high schools have none.

## What is already decided (cite, not asked again)

- **People topic 10** (`docs/design/people.md`, "Topic 10. Staff at each level"): staff are people with hidden 1-20 ratings; the departments table (performance, medical, football operations, player support) with each job, its ratings and what it feeds; video and equipment staff are named people with no ratings; doctors are independent where the rules say so; sizes follow the rule book and the money; who hires whom by level; staff at work leave their read in words; where staff come from and the ladder; you judge staff only by their work; about 115,000 staff people is fine on an 8 GB M1.
- **Q29**: staff are rated and affect development, injuries and scouting. **Q28**: owners budget for staff. **Q37**: your staff gives analytics advice in games. **Q39**: your medical staff estimates return dates.
- **People topic 11**: support staff sign year to year; the staff market.
- **Capital CP17 and CP19** (`docs/design/capital.md`): the weight room, sports science lab, recovery center, nutrition center and academic center are rooms with levels 1-5, and the rooms share out today's facilities effect.
- **Training** (`docs/design/training.md`): the strength staff writes each player's body plan with the dietitian; the trainers' load flags; no athletic trainer at a high school means injuries are found later.
- **World topic (background money)**: "No nutrition effect" on a kid's physical growth from his family's money (`docs/design/world.md`, Raichel's answer). That is about home, not a team dietitian, but question 4 keeps to its spirit.

## What is built today

- Five support heads per college and NFL club (strength, medical, analytics, academic, player engagement): one stored person each, a hidden 1-20 rating, pay, a yearly market, and a strength head who follows his head coach (`world/people.rs`, `Staffer`, `Support`; `docs/reference/how-it-works/staff-and-coaches.md`, "Support heads").
- What reads them: the strength head drives physical growth, the weight room's speed and the strain guard; the medical head the yearly injury roll, game injuries with the recovery center, the doctor talks and the scouts' medical reads; the analytics head NFL film study and a GM's film lean; the academic head grades beside the academic center; engagement cuts trouble by up to 12%.
- The staff screen already lists an assistant strength coach, a dietitian, a team physician, assistant trainers, a director of player personnel, analysts and graduate assistants. They are drawn from a key with a rating around their head, never stored, and **nothing reads them**. The screen already splits each head's one rating into his job's facets, which average to it (`docs/world-model.md`, "Your read of your staff") (`world/staff_read.rs`, `support_base`).
- High schools have no support staff. A player there reads "a trainer part time or none" (`world/people.rs`, `support_ctx`).

## How real football staffs it

- **NFL.** The CBA requires a head athletic trainer and at least two assistants (Article 39, since 2021 camps); most clubs carry about four (Harvard Football Players Health Study, 2016). Every club must keep a behavioral health clinician and a pain management specialist (NFL-NFLPA, 2019). Dietitians: six full-time in the whole league in 2014 (Sports Illustrated, the Dolphins' hire), 28 of 32 clubs with a full-time one later, and 44 full-time dietitians across the league now (Professional Football Registered Dietitian Society). Analytics: every club had at least two staffers and 21 had three or more in 2023; the most had seven (ESPN, Seth Walder's staff list). Sports science (GPS and load tracking) spread through the late 2010s; the league's own player tracking chips came in 2014.
- **FBS.** On-field caps by era: 9 assistants until January 2018, then 10, four graduate assistants, and a cap on strength coaches; off-field staff has no limit. Alabama's 2023 media guide listed 69 off-field staffers, among them 5 in recruiting, 4 in player personnel, 3 in player development and 2 chefs. Georgia led a CBS Sports survey of support staff and spent about $4.5 million on it. Dietitians spread after the NCAA allowed unlimited meals in 2014. Sports medicine and academics belong to the athletic department; in the Power Five the doctors can't be overruled (2017) and mental health services must be there (2019 autonomy rules). Revenue sharing from 2025 is pushing some schools to trim the analyst army (CBS Sports).
- **FCS and below.** A handful of support people, often shared across sports. A Division III trainer may cover every team on campus.
- **High school.** 37% of public high schools have a full-time athletic trainer, 32% part-time and 31% none; private schools 27%, 28% and 45% (CATCH ON). Teachers coach; a coach runs the weight room; a big Texas school may have its own strength coach. A local doctor often volunteers on Friday nights.

## The proposal

### 1. The departments and their jobs

Each department has a head (the built support head) and its people. Every person is stored, with topic 1's traits, a birth year, pay, a year he came and the ratings his job needs.

| Department | Head | His people | Ratings (hidden 1-20) |
|---|---|---|---|
| Performance | Head strength coach | Assistant strength coaches, strength interns (college), dietitians, sports scientists | Strength coach: strength development, speed and conditioning, injury prevention. Dietitian: nutrition. Sports scientist: load reading |
| Medical | Head athletic trainer | Assistant trainers, team physicians (contracted), behavioral health clinician, pain management specialist (NFL) | Trainer: diagnosis, rehab, return-to-play judgment. Physician: diagnosis, surgery. Clinician: counseling |
| Player support | Academic head (college) and player engagement head | Academic advisers, learning specialists, tutors (college); player development staff | Academic support; life skills |
| Football operations | Head of analytics | Analysts and quality control, recruiting and personnel staff (college) | Opponent study, self-scouting, analytics, evaluation, organization |

The college general manager and his roster office are item 5, not here. Video and equipment staff stay named people with no ratings (decided). NFL position coaches are item 2.

The built single rating per head becomes the center of his job's ratings, so a 16 strength head stays about a 16 strength head.

### 2. Sizes by level (2026; history follows the eras)

| Level | Performance | Medical | Player support | Football ops |
|---|---|---|---|---|
| NFL | Head + 2-4 assistants, 1-2 dietitians, 1-2 sports scientists | Head + 3 trainers, 2 physicians, clinician, pain specialist | Engagement head + 1 | Analytics 2-7 |
| Power 4 | Head + 4 assistants and interns, 2-3 dietitians, 1-3 sports scientists | Head + 3-5 trainers and GA trainers, physicians from the university, clinician at most | Academics 2-4 and tutors, development 1-3 | Analysts 10-30, recruiting and personnel 5-15 |
| Group of 5 | Head + 2-3, 1 dietitian, 0-1 sports scientist | Head + 2 | Academics 1-2, development 0-1 | Analysts 3-8, recruiting 2-4 |
| FCS | Head + 1-2, a shared dietitian or none | Head + 1-2 | Academics 1 | Analysts 0-3, recruiting 0-2 |
| Division II, NAIA, JUCO | One strength coach for the department | One or two trainers for the department | A shared adviser | None |
| Division III | A shared strength coach | One trainer for every sport | A shared adviser | None |
| High school | None paid; a coach runs the weight room (big Texas schools may have one) | Full-time, part-time or none by the CATCH ON shares | None (teachers) | None |

Within each range the money decides: the college's football support line (and the athletic budget for the department's own people), the NFL owner's spending level, a school's district and booster club. Eras follow the real growth: dietitians rare before 2014, sports science from about 2015, NFL clinicians from 2019, analysts swelling through the 2010s and trimmed a little after revenue sharing.

### 3. Head and depth

A player gets his department's work as a blend of the head's skill and how much of the staff's time reaches him. People topic 10 proposed the head at about 60% of the effect (question 1). Each kind of person covers a real number of players (question 2). Starters and the injured come first, so a thin staff shortchanges the backups and the healthy.

### 4. Rooms and people

The rooms keep their decided share of today's facilities effect (CP19). The people do the work in them. Where a room gives a tool, the person reads it: the sports science lab's load read comes through the sports scientist, so a level 5 lab with nobody in it shows no load read, and a great sports scientist in a level 1 lab reads half as much. This follows how the built code already adds the recovery center to the medical head and the academic center to the academic head.

### 5. What each department feeds

Built reads move from the single head to the department. New reads, each sized in a question where life doesn't set it:

- **Strength staff**: as built (physical growth, the weight room, the strain guard), with depth deciding how many players get a real program. New: the report-day conditioning test and in-game fatigue (listed as not read in `staff-and-coaches.md`).
- **Dietitian**: how fast a body plan moves weight and how well a player holds it in season (question 4).
- **Sports scientist**: the load read on each player and fewer soft-tissue injuries (question 5).
- **Trainers and physicians**: as built (return estimates, recovery, the yearly roll, game injuries, medical reads), with depth deciding how early injuries are found.
- **Behavioral health clinician**: how fast stability comes back after a hard event (question 6). Spotting trouble stays engagement's, as built.
- **Academics**: as built (grades and eligibility), with depth deciding how many players get a tutor.
- **Analysts**: as built in the NFL (film study), and new in college: film study's budget part and self-scouting your own tendencies (topic 10).
- **Recruiting and personnel staff**: more film looks for your board each week and the visit weekends run smoothly (section 7 says they work from film and on campus).

### 6. Hiring, the market and the AI

As decided in topic 10 and 11. Heads propose their own people for your yes; at a college the trainers, doctors and academic staff are the athletic department's. Assistants climb: an assistant trainer is the first man up for a head trainer job. A new strength head brings one or two of his assistants. The AI uses the same rules.

### 7. What you see

Hidden ratings, read from their work (decided). New: the staff screen's drawn support people become these stored people, with their real track record.

### 8. Cost

About 13,000 stored people in college and the NFL and several thousand high school trainers (one school per 35,000 people, about two in three with a trainer): a few megabytes. Their work is a few numbers a week per player. The yearly market grows from 5 posts a team to about 15-60.

## Questions for Cain (all agreed 2026-11-17)

1. Head at 60% of a department's effect, his people's reach at 40%.
2. Reach per person: a trainer about 30 players, a strength assistant about 25, a dietitian about 60, a sports scientist about 60, an academic adviser about 40 (tutors 10 each).
3. A typical department gives exactly today's results, so the world's averages don't move; only the spread between rich and poor staffs grows.
4. The dietitian moves weight plans faster and holds weight in season, and never touches ceilings or natural growth.
5. A top sports scientist cuts soft-tissue injuries by up to 8% against a typical one, on top of the strength guard, with the two capped at 20% together.
6. A top behavioral health clinician brings stability back up to 30% faster after a hard event than a typical one.
7. Build it as one lane after the world rebuild, measured on a fresh seed world.

Cain answered "agree on all" (2026-11-17, 16:53 UTC): every question above is yes, and the whole proposal stands as written. The build lane measures sizes 1, 2, 5 and 6 on a fresh seed world against question 3 (a typical department gives today's results).

Sources: NFL-NFLPA CBA Article 39; Harvard Football Players Health Study, ch. 3 (2016); Sports Illustrated, 2014-12-05; Professional Football Registered Dietitian Society (sportsrd.org/pfrds); ESPN NFL analytics staff list (2023); Alabama 2023 media guide via CBS Sports; CBS Sports support staff survey; Saturday Down South support staff spending; CATCH ON study (2015-2018).
