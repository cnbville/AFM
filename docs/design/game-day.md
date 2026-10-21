# Section 9: game day control (design in progress)

Status: design started 2026-10-21 in the thread "Section 9 design: game day control". Not built. Topics go one at a time with Cain; this file records each one as it is settled.

Labels:
- **Confirmed**: Cain answered it.
- **Proposed, not confirmed**: Claude's design, posted in the thread, not answered yet.
- **Decided earlier**: settled by an earlier section; the source is named.

---

## What already exists

### Decided earlier (built on, never re-asked)
- **Who calls (match engine 9.1):** the head coach can take over any snap; otherwise the OC or DC shows his call drawn up and the user accepts it or sends it back. Call quality comes from the coach's ability and a real play-and-counter library.
- **The reward rule (plays and tactics topic 0 and 6):** calls and plans pay off only through the battles, never a flat bonus, the same for the user and the AI, big enough to matter and never so big that calls beat talent. The researched band (`docs/research/engine-and-tactics.md`, section 7): best against worst play caller, same players, about 3-7 points a game; the whole coach effect at most about 7 on offense and 5 on defense (10th to 90th percentile, an upper bound); a great QB about 12. The engine measures callers 18 against 4 at +4.4 points (`docs/match-engine.md`).
- **The weekly plan (plays and tactics topics 4-7):** film breakdown, the real practice week, the call sheet by situation, matchup plans, the scout team, plans that can be wrong, weather and venue, tendency reports with accuracy from staff and sample, self-scouting, opponents adapting.
- **Clock and decisions (match engine 4.2, Q37):** timeouts, the two-minute drill, kneel-downs and 4th downs are a coaching skill on real 4th-down and win-probability data; in-game decisions come with optional advice from the analytics staff.
- **Game length (Q40, match engine topic 9):** a watched game takes about 15 minutes; play by play when calling live; key plays, more plays or every snap; between plays the user can change the plan, call a timeout, swap players or tell the coordinator to attack a weakness; staff notes at halftime. No other games live; any game can be watched back in 2D from its seed (9.2).
- **People (section 6):** pregame and halftime talks are designed (people topic 5); the doctors decide on concussions (people topic 11); your time is a budget (people topic 5).
- **Scouting (section 7):** advance scouting of the next opponent is section 9's (scouting topics 3 and 13).
- **Screens (section 10, topic 12):** the 2D view keeps a panel to the right of the field for the call and the between-play actions.
- **Tendencies:** player instructions and moves against the other team's habits (`docs/design/tendencies.md`, topics 3 and 4).

### Built (2026-10-21)
- `game/calling.rs`: `Film` per team per game; game prep from film (`Memory`, studied as if the staff had seen 20 × skill/20 snaps); `read_habit` with an error that shrinks with film study; the matchup `Plan` (target a corner, bracket a receiver, spy the QB, moves against habits); look tells that defenders key (`opp_look`); 4th downs, two-point tries.
- `game/flow.rs`: `play_game` plays a whole game in one loop. Onside kicks, timeouts, kneels and spikes are decided by the AI. The coin toss is a flip; the winner never chooses.
- `game/roster.rs`, `game/scrimmage.rs`: the weekly install (75% of the playbook in a normal week, trimmed on short weeks) and the mental errors when the load is over what a player absorbs.
- Not built: pausing a game between snaps, replay challenges, any way for the user to step in. Only the seat's games keep their play-by-play.
- AFM 0.2.23 (section 8 build, 2026-10-21) added a "Who calls the plays" choice on the new-career Style step. Cain did not ask for it; topic 4 builds on it, not decided here.

---

## The topics

1. Advance scouting: the opponent report
2. The game plan you build
3. Game morning: inactives, game-time decisions, warmups, the coin toss
4. Calling plays live: which snaps stop for you (the game view thread's item 4), your call, the coordinator's call
5. Between plays: timeouts, substitutions, packages, tempo, a word to a player or a coach
6. In-game decisions: 4th downs, two-point tries, onside kicks, penalties and replay challenges, with the analytics staff's advice
7. Injuries during the game
8. The sideline: benching a player, changing QBs, tempers, momentum
9. Halftime
10. Watching when you hand the game off
11. After the game: why calls worked or failed, the film it leaves
12. Your own play-calling rating, and how a game you called is replayed from its seed

---

## Topic 1. Advance scouting and the opponent report

### The report (proposed, not confirmed; posted 2026-10-21, no question on it)
- **Built from film, the way real staffs build it.** NFL quality control coaches tag every snap of the opponent's last four games (down, distance, hash, field zone, personnel, formation, motion, play, result). A coordinator in his first year at a new team is scouted from his old team's film. The staff's size and skill decide how many snaps get tagged and how many tags are wrong.
- **In person by the level's real rules.** NFL clubs may send a pro scout to the opponent's game the week before. The NCAA bans in-person scouting of future opponents (Bylaw 11.6; Michigan's sign-stealing scheme broke it in 2023 and cost Jim Harbaugh three games). High schools swap game film and send an assistant when the schedule allows.
- **Contents:** run-pass split by down, distance, formation and personnel; favourite plays by situation (3rd down, red zone, two-minute); blitzes and coverages by down; the head coach's 4th-down and timeout habits; tells; each player's tendency read (tendencies topic 4); who is hurt and who is new; special teams (a returner who brings everything out, a slow punt snapper); a page of player matchups ("their left tackle struggles with speed").
- **Every line says how sure the staff is, with its sample** ("runs 78% from 12 personnel, 41 snaps"). Early in the season, and against a new coach, the report is thin.
- **Your own film time counts:** any opponent game can be watched back in 2D and judged with your own eyes.
- **Sign stealing as it really is.** Decoding signals from what can be seen is legal in the NFL; filming them is not (Spygate, 2007: $750,000 in fines and a 1st-round pick). Helmet radios (NFL; college since 2024) leave high school's hand signals (plays topic 3.8) the ones still exposed. A staff that cracks an opponent's signals reads some calls a beat earlier, through the same defender reads as any tell. Who breaks the rules follows people topic 7.

### Other teams' games show only what film shows (confirmed 2026-10-21)
Question 1: "When you watch another team's game back, should it show only what game film shows?" Cain: "Yes and since this is still a a game despite the ultra-realism the tape should provide some hints/tips despite scouting ranking."
- Other teams' games show the dots, the results and your staff's tags, labelled as your staff's and with their mistakes. The true calls and the engine's reasons show only on your own games and in debug tools.
- The tape gives its own hints and tips, whatever the scouting staff's rating (below).

### The tape's own hints and tips (Cain's ask, confirmed; the details proposed, not confirmed)
Claude read "despite scouting ranking" as: no rating decides the hints, neither the staff's nor the user's own.
- **Hints come from your film time.** "Study the tape" on any of the opponent's games uses film time from your week (people topic 5's time budget) and gives that tape's hints at once, each linked to the snaps that show it. Watching in 2D is for seeing the evidence.
- **A hint is what the tape really shows, with the count** ("on 3rd and 7 or longer they showed two deep safeties on 9 of 11 snaps, and rotated one down at the snap on 7"). Always true to that tape, but one game is a small sample: a habit can be noise or opponent-specific.
- **Kinds:** formation and personnel tells; player tells (plays topic 7.4: a back's depth, a lineman leaning on his hand, a safety creeping down, a QB staring at his first read); habits by situation; weaknesses in the battles ("lost 6 of 8 reps to speed rushes"); special teams; the head coach's habits.
- **The tip is the textbook answer** ("outside leverage on 3rd down gives up the slant"). It never promises the play works: the battles decide, and a good opposing coach breaks his habits (plays topic 5.6) or uses one as bait.
- **Your finds are yours:** they sit in the opponent report under your name, apart from your staff's lines, and one click adds a hint to the game plan.
- **AI coaches don't get hints:** their staffs read film through their ratings, as built. The hints stand in for a human's trained eye on a film of moving dots.
- Waiting on Cain: question 2 (how many hints per tape; pick: the three strongest, a habit seen on several tapes merging into one stronger hint, so up to about 12 a week) and question 3 (which first; pick: what the staff's report doesn't already have).

---

## Topic 2. The game plan you build (proposed, not confirmed; posted 2026-10-21, no question)

- **Your coordinators draft it, you change what you want.** The draft lands on plan day (NFL Tuesday, college Sunday or Monday, high school the weekend), each part with its reason. You edit any part, approve it, or send it back with a note ("more 12 personnel runs"). A play-calling head coach (Sean McVay, many high school head coaches) builds his side himself. A draft is as good as the coordinator who wrote it (plays topic 5.8).
- **Seven parts, as in a real plan:**
  1. Offense: the opening script and the situation boxes, each box's plays in order of preference and split by hash.
  2. Defense: fronts, coverages and pressures for the same boxes; a plan per opponent personnel group; their best receiver (bracket, or a shadow corner as Darrelle Revis played it); their QB (a spy, or rush lanes that keep him in the pocket).
  3. Personnel: who plays in each package (nickel, dime, goal line, jumbo), the defensive line rotation by series, a snap limit for a player back from injury.
  4. Special teams: kick and punt direction (away from a dangerous returner, squib, coffin corner), the return scheme and returners, fakes and onside kicks that are on only if the look from film shows up.
  5. Your game rules: a 4th-down chart and a two-point chart, a timeout rule ("keep two for the end"), when to go up-tempo, the four-minute plan to protect a lead.
  6. Player instructions for this opponent (tendencies topic 3).
  7. The QB's checks this week (plays topic 3.3).
- **The plan is the week's practice:** its plays and looks become the install and the practice script (training topic 2); the scout team runs the looks from the opponent report. A play off the sheet can still be called, less practiced.
- **It pays off only through the battles:** a practiced play has fewer busts; a bracket puts two men on one receiver and leaves someone short elsewhere; a tell from the report lets defenders key a play a beat sooner. Best plan against worst, same players, stays inside the researched 3-7 point band (plays topic 6.3); plan and calls together stay under a great QB's 12.
- **Plans leak:** a player you cut or trade knows your calls; a club that signs him before playing you learns some of them, and its defenders read those calls like a tell. The real answer: change signals and call names, at a little extra learning that week.

---

## Topic 3. Game morning (proposed, not confirmed; posted 2026-10-21, no question)

- **The day runs in the real order:** morning meeting, arrival, warmups, inactives, pregame talk, coin toss, kickoff. It stops only where a decision is yours and not handed off.
- **Game-time decisions:** a questionable player tests his injury in warmups; the trainer reports what he saw and the player says how he feels, as honestly as his personality lets him. You decide whether he plays and how much, except on concussions, where the doctors decide (people topic 11).
- **Inactives by level:** NFL 48 of 53 plus up to two practice-squad call-ups, named 90 minutes before kickoff (team management); college the travel squad for road games within the conference's limit; high school the whole varsity dresses, JV players moved up when needed.
- **Their inactives reach you:** the NFL list is public 90 minutes before kickoff; your staff proposes a plan line when a key opponent is out ("attack the backup, No. 31").
- **Warmups set the kicking line:** the kicker's range toward each end in the day's wind ("the 35 going south, the 28 going north") feeds the 4th-down chart.
- **The coin toss is a real choice:** receive, kick, defer, or take the wind. Since the NFL allowed deferring in 2008 most winners defer; overtime differs (Super Bowl LVIII: the 49ers chose to receive and the Chiefs won on the second possession). AI coaches choose by their own habits. Replaces the engine's flip.
- **The pregame talk** (people topic 5): optional, by you or a captain.
