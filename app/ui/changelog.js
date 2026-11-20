// What's new (Cain, 2026-11-13): the changelog on the opening screen, written
// for players. A few short lines per version, in plain words: what you will
// notice, with no PR numbers, code names or build talk. Newest first. A
// version that changed nothing you can see is left out.
//
// Versions from 0.2.492 on write themselves: .github/scripts/patch_notes.py puts each
// shipped build's panel between the two auto markers below, from the Patch note section
// of its merged pull requests (see docs/patch-notes.md). A build only gets a panel when a
// pull request in it has that section in plain words. Entries under the markers are
// hand-written and frozen; do not add new ones by hand.
//
// `v` is the app number (0.2.<build>), `date` the day it came out, `title` a
// few words on the version, `notes` three or four short lines. The patch notes
// (afm/patch-notes.md) keep the full story for the team.
"use strict";

const CHANGELOG = [
  // auto:start (written by .github/scripts/patch_notes.py, do not edit by hand)
  {
    v: "0.2.661", date: "20 Nov", title: "Continue stops when a call rings, so you can answer it and 2 more",
    notes: [
      "The kicking game now plays like real football: long field goals are as hard as they really are, kicks get blocked and run back, and returners break long ones more often.",
      "Long field goals are made at the real rate, and coaches try fewer of them, especially into a strong wind or on a freezing day.",
      "Field goals, extra points and punts can be blocked by a real rush, and the loose ball is live: it can be scooped up and returned for a touchdown.",
      "Kick and punt returns break into the open more often.",
      "Flags on returns now count, with the yards, ejections and fines that come with them.",
      "Talks now get things done. Say you're putting a player on a project or a focus and it happens after one confirm. Every talk shows what the other person seems to be after, and coaches and agents now haggle with you round by round.",
    ],
  },
  {
    v: "0.2.658", date: "20 Nov", title: "Engine gaps lane EG5: each level's rule book and the flags",
    notes: [
      "Flags now follow each level's own rule book, so college and high school games mark penalties the way their officials do, and NFL flags come at their real rate and size.",
      "College defensive holding costs 10 yards, and high school pass interference and holding carry no automatic first down (Texas plays college rules).",
      "Fouls near the goal line are marked half the distance, and a hold in your own end zone is a safety.",
      "New calls: illegal contact, illegal hands, neutral zone infractions, twelve men on the field, kickoffs out of bounds or short, and free plays after a defender jumps.",
      "Linemen no longer drift downfield on pass plays, so ineligible man downfield and blocks in the back are rare, as in real games.",
      "Pass interference changes the catch, false starts climb on third and fourth down with the snap count, and offenses take a delay to back up a punt or try a hard count on fourth and short.",
    ],
  },
  {
    v: "0.2.657", date: "20 Nov", title: "Ground lane S6: the rival screen",
    notes: [
      "You can now see the rival staff's men on your ground, the race they run for your commits in dated signs, the claims they make about you and what you can do about it.",
      "A new Rivals page lists the men working your areas, your commits in play and the claims out there.",
      "A rival's page shows his style, his ground on the map, and his history with you.",
      "The race for a kid reads as numbered, dated signs, each with where you heard it.",
      "Answer a wavering commit with a call to the family, or a claim about his program (a risk if it isn't true).",
    ],
  },
  {
    v: "0.2.656", date: "19 Nov", title: "Talk replies that match the game (lane A, items 1-12)",
    notes: [
      "The people you talk to now answer what you actually said: they react to how your words landed, take your promises the way their trust in you says, name their price and hold you to your word.",
      "Promises get a real reaction, and saying the opposite later breaks the promise and he calls you on it.",
      "Coaches you interview name their number when you ask for a ballpark, and saying yes to their counter makes it your offer.",
      "A high school coach you interview names his best players and wants them recruited as part of the hire.",
      "Everyone has their own way of talking, remembers the whole conversation, and \"Heard as\" is shorter.",
      "Ending a talk always closes it; an open decision moves to its screen with a note in your inbox.",
    ],
  },
  {
    v: "0.2.655", date: "19 Nov", title: "EG2: ball security (live fumbles by each level's rules, fumbled snaps, strip sacks, backs' fumbles at the real rate) and 3 more",
    notes: [
      "Talking to your team now feels like talking to a room full of people, not one captain. Each player takes what you say his own way, and the right man speaks up.",
      "A line to the defense, the seniors or the backups lands on them, and the room shows who's with you and who's trading looks.",
      "Captains and leaders pull their friends with them, and calling a man out in front of everyone stings him and his friends harder.",
      "A promise to the team is a promise to every man in it.",
      "Ask a full room if anyone has a problem and it goes quiet, but the man who held back may ask to see you the next day.",
      "Meet each position room once a week with its coach.",
    ],
  },
  {
    v: "0.2.650", date: "19 Nov", title: "Fix minor engine bugs 409-412: the play action leftovers and 7 more",
    notes: [
      "Promising a player he'll be a captain now means something. Keep your word when the team votes, or pay for it.",
      "\"You'll be a captain\" in a talk is now a real promise that shows on your promise list.",
      "It's judged when your team names its captains at camp.",
      "If the vote leaves him out, you name him anyway, and the locker room notices who he jumped.",
      "Your player believes it more when he's already one of the room's leaders.",
      "The game now needs about a gigabyte less memory in season, so clicks wait less on a Mac with 8 GB.",
    ],
  },
  {
    v: "0.2.638", date: "19 Nov", title: "A rotating defensive line and real weather",
    notes: [
      "Defensive tackles now rotate like a real NFL line, and rain and strong wind hurt the passing game as much as they really do.",
      "Staffs rotate their tackles in waves. A star still plays most snaps, while a room of equals shares them.",
      "Rain makes long throws much harder to complete than short ones.",
      "A light breeze barely matters, but a 15-20 mph wind cuts passing the way it does in real games.",
    ],
  },
  {
    v: "0.2.636", date: "18 Nov", title: "Coaches answer with real specifics in live talks",
    notes: [
      "The coaches you talk to now give real answers about their scheme and their recruiting. Ask a coordinator what he'll run, or a recruiter who he's working, and you get specifics.",
      "Coordinators and candidates name their offense or defense, say how well your roster fits it, and say what it would take to switch.",
      "Assistants name the recruits they're working and the players who signed through them.",
      "In the holdover staff meeting, each coach answers for himself, and the right man speaks up.",
      "Coaches no longer act offended by a fair question.",
      "Holdover coaches only boast about what's actually true.",
    ],
  },
  {
    v: "0.2.635", date: "18 Nov", title: "Ground lane G9: the race and the AI on Ground and 2 more",
    notes: [
      "Play action finally works: the run fake freezes linebackers and pass rushers, the line blocks for it, and coaches use it about as often as real NFL teams do.",
      "Any pass play can now come off a run fake, so you'll see play-action crossers, floods and even screens, not just three special plays.",
      "Defenders who read the backfield bite for a moment, longer for an undisciplined one and against a quarterback who hides the ball well.",
      "Play-action passes are no longer sacked one time in five: the back and the line protect the fake, and bootlegs get a pulling guard while the quarterback rolls away from the rush.",
      "Play callers fake most on early downs, in run-first systems like wide zone, and against defenses that keep biting.",
      "Arguments now work at the table: true numbers talk a player down to his real market, recruits hear the truth about your depth chart, and a veteran will take less if you promise to bring in help.",
    ],
  },
  {
    v: "0.2.632", date: "18 Nov", title: "Talks: \"Heard as\" says how a line really went (the landing read) and 5 more",
    notes: [
      "Development projects now belong to the staff that set them, so a player who arrives on your team brings no one else's project and your rooms keep their real number of slots.",
      "A player who transfers, signs or is drafted starts fresh with his new staff.",
      "His last staff's project shows as a note on his Development tab and takes no slot.",
      "Every staff in the world follows the same rule, so rooms never run past their slots.",
      "The people you talk to now know what time of year it is. In January a player talks about winter workouts, spring practice and the portal window, not the next game.",
      "Every conversation knows where your team is in its year and what's coming next.",
    ],
  },
  {
    v: "0.2.625", date: "18 Nov", title: "Coaches walk you through the season from the world's records",
    notes: [
      "Your coaches now really break down a season when you ask them, with the numbers, the games that slipped away and the players who struggled.",
      "Ask a coordinator how the year went and he walks you through it: points and yards against the conference, third downs, red zone, big plays, takeaways.",
      "He names the games that got away and the starters who struggled or stood out at their spot.",
      "A coach hired after last season tells you it wasn't his season, then gives what he saw on the tape.",
      "Push for more and he gives you the rest; sharper coaches see more.",
      "Ask a coach about one of his players and he knows how the player's numbers rank against the conference's starters.",
    ],
  },
  {
    v: "0.2.624", date: "18 Nov", title: "Make the search bar fast and 4 more",
    notes: [
      "A player's card now says which season its stats are from, and shows the season being played so far, so you are not looking at last year's numbers by mistake.",
      "The hover card and compare name the season (\"2024 season\") instead of \"Last season\".",
      "Until a college season's title game is played, its numbers so far show on their own line.",
      "NFL players show this year's numbers so far until the regular season ends.",
      "A pro's card shows his pro seasons, not his old college year.",
      "You can now start development projects for your own players: a skill project with his position coach, or a body project with the strength staff, with your staff's honest read before you commit.",
    ],
  },
  {
    v: "0.2.619", date: "18 Nov", title: "Scheme-fit lane SF3: a blocked man is out of the play and 2 more",
    notes: [
      "Capital now has a Pecking order page: see where the press ranks your conference's facilities each July, how you have moved, and what your donors whisper about rivals' building plans.",
      "Every program in your conference ranked on facilities, with its move since last July, its newest building and why the press rates it there",
      "The drive you know of for each rival: public with how far along it is, a building going up, or a rumor",
      "Your own rank over the years as a line",
      "A list of what your donors have heard about rivals' quiet campaigns",
      "Players now build a real bond with each coach who works with them, from a stranger up to someone who sees you like a big brother, a father or a grandfather. How you handle the hard moments counts most.",
    ],
  },
  {
    v: "0.2.616", date: "18 Nov", title: "Opening a save closes the open world before reading the file",
    notes: [
      "Loading a save is lighter on memory now: the game lets go of the world you had open before it reads the next one, so big worlds load on an 8 GB Mac.",
      "Loading a save while a career or world is open no longer holds two worlds in memory.",
      "Big worlds' saves and autosaves load again on 8 GB Macs.",
      "If a file can't be opened, you land back on the start page instead of a broken career.",
    ],
  },
  {
    v: "0.2.615", date: "17 Nov", title: "Capital lane CS4: the project screen and 3 more",
    notes: [
      "Your capital campaign now has its own page: see how close the goal is, who gave and what moved them, and make the calls that are yours.",
      "The drive page shows the goal, how much is paid in, pledged and still to raise, and the four stages of the campaign.",
      "Every pledge is listed with the moment that brought it, like a top-10 pick or a conference title.",
      "The lead donors who are waiting for you are named, with what they'd give to.",
      "Your AD says where he stands, and you can set the goal, go public early, work the lead donors yourself or push him to borrow.",
      "Capital's Donors screen shows who could fund your next building, how hot donors are for your program, and lets you book a lunch or dinner with one.",
    ],
  },
  {
    v: "0.2.611", date: "17 Nov", title: "Capital lane CS1: the Campus screen and 3 more",
    notes: [
      "The facilities race now has a voice. Rivals' new buildings can cost you recruits and you're told why, your donors pass on what rivals are quietly raising for, and every July the press ranks your conference's facilities.",
      "When a recruit picks a rival, the reasons can name the rival's new weight room, locker room or other room over yours.",
      "Your donors tell you, over dinner or in the suite before kickoff, when a rival is quietly raising money for a new building, and roughly how far along it is.",
      "Rumors from a donor who was in the room are close to the truth. Word heard through friends is vaguer.",
      "Each July the press publishes a facilities pecking order for every conference, with where you stand and how far you moved.",
      "A school visit is now a scene you play: walk the stops, sit down with the head coach, and watch a new name's blanks fill as you go.",
    ],
  },
  {
    v: "0.2.607", date: "17 Nov", title: "Capital lane C4: your week and scenes",
    notes: [
      "Your donors now live in your week. Dinners, the suite before kickoff and the spring caravan sit beside your other scenes, and you choose the rooms a recruit tours on his official visit.",
      "Your development office puts donors worth your time on your week, each with a reason, and you can book your own dinners.",
      "In season your only donor hour is the suite before a home game.",
      "On an official visit you pick the three rooms he sees, and he notices what you skip.",
      "When a donor whose name is on a building lands in scandal, you decide whether the name comes down.",
      "Groundbreakings, reveal videos and ribbon cuttings keep who was on stage, how the recruits took it and the next day's headline.",
    ],
  },
  {
    v: "0.2.606", date: "17 Nov", title: "Capital lane C3: the money and the drive and 2 more",
    notes: [
      "Burners are as rare as in real football now: a 4.2 forty is a once-a-draft event, not a handful every spring.",
      "Very fast prospects at the combine come up about as often as in the real NFL draft.",
      "A player's top speed comes from his body. Hard work in the weight room still trims a little, but no longer turns a 4.5 man into a 4.4 man.",
      "Freaks still exist: the fastest men still run 4.2s, and big men who fly are as common as real ones.",
      "How you talk now matters. Your players, staff, boss and recruits hear your heat, your language and who you aim it at, and they react the way real people would.",
      "Pick how you talk in your coach profile, then let your real words reshape your reputation over a season.",
    ],
  },
  {
    v: "0.2.603", date: "17 Nov", title: "Capital lane C2: the rooms and the campus",
    notes: [
      "Your campus is now made of real rooms, from the locker room to the NFL wall, and each one does something. Build, renovate and watch rivals answer.",
      "Thirteen rooms per program, each with a level, starting from every school's real buildings.",
      "Rooms age and are read against the best in your conference, so a rival's new weight room makes yours look older.",
      "Each room has its own effect: the weight room grows bodies, the recovery center gets hurt players back sooner, the content studio helps your players' deals, the locker room makes visits count.",
      "In the year before a new building opens, your team works in temporary space.",
      "Recruits now care about campus life, some of them a lot.",
    ],
  },
  {
    v: "0.2.602", date: "17 Nov", title: "Talks that know where they stand and what is between you and 1 more",
    notes: [
      "The game no longer freezes for minutes when you press Continue while it autosaves.",
      "Pressing Continue during an autosave now waits a few seconds for the save to finish, then plays.",
      "The button says \"Finishing the autosave…\" while it waits.",
      "Macs with 8 GB of memory no longer run out of memory during an autosave.",
      "The people you talk to now know where they stand and what's between you: a senior talks about the draft, a player you met last week knows you're new, and one you've coached for years remembers your promises.",
      "Players know their eligibility, the draft and their place, and a man in his last season never talks about transferring.",
    ],
  },
  {
    v: "0.2.599", date: "17 Nov", title: "Ground lane S7: your men and your desk (the staff carousel and assistant desk screens)",
    notes: [
      "Your staff now has a home on the Ground menu: see who another school is after, counter before he walks, and see every man's ground, mood and record in one place. Assistants get their own desk.",
      "A warning when another school wants one of your men, with his face, what pulls him each way and your read of him",
      "Three counters (a raise, a title, a promise), each showing how far it reaches and what it costs",
      "Who could take his area, and what your standing loses with him",
      "Every man's areas, mood and record, and the areas nobody works",
      "An assistant's desk: your list, his answers, his memory of you and your goals",
    ],
  },
  {
    v: "0.2.598", date: "17 Nov", title: "Fix the \"Ask in your words\" 400: nullable enums as anyOf, and a check on every schema and 3 more",
    notes: [
      "Ground has a Leads screen: open a kid someone told you about and see who told you, which of the five blanks are filled and by whom, who else is on him, and what to do next.",
      "Your leads sit in a strip; pick one to open his card.",
      "The five blanks show who filled each and when, with a button on the open ones.",
      "A timeline shows who heard about him and who offered, against the day the services update.",
      "Offer him, fill the open blank first, or let him go; each says what it risks.",
      "Once he's your find, the card follows how he blooms.",
    ],
  },
  {
    v: "0.2.593", date: "17 Nov", title: "Ground lane G5: the week's scenes played",
    notes: [
      "Your recruiting week now plays out as real scenes: walk into a high school coach's office, run your own clinic, sit with families on junior day and take the call the morning a kid decides.",
      "A school visit goes stop by stop: the sign-in sheet shows who was in before you, the coach's office gives you his list, and your eye can catch a kid in the weight room nobody listed.",
      "Your clinic, open practices and the state coaches' convention warm many coaches at once, fastest with the ones who run your scheme, and the hallway is where the names come out.",
      "Junior days, visit weekends and the spring game let you sit with families, hear from player hosts and offer on the spot; from 2027 the spring game is yours to hold or skip.",
      "Every scene ends on one line of what you walked away with, and any scene you hand to your coordinator still comes back with his line.",
    ],
  },
  {
    v: "0.2.592", date: "17 Nov", title: "Progression step 4c: no 99 wall, a thin top past 99, the bench by body weight",
    notes: [
      "The best players can now go past 99 under the hood, so true freaks stand out from merely great players. Big men's bench numbers now follow their weight, as they do at the real combine.",
      "Ratings no longer stop at 99. Past the mid-90s each extra point gets harder, so a 100+ arm is about as rare as a 4.25 forty.",
      "Elite prospects are as uneven as anyone else. A top talent can have one standout tool and a weaker spot.",
      "Bench press reps follow body weight, and the strongest men in the weight room can reach the 40-rep club.",
      "Vertical jumps top out near the real record instead of running far past it.",
    ],
  },
  {
    v: "0.2.591", date: "17 Nov", title: "Let updates skip a pending older version and 1 more",
    notes: [
      "Your athletic director now writes down what you commit to. Promise to keep the state's best player home or to win eight games, and it goes on his goals sheet, or he tells you why not.",
      "Commitments to your boss become goals he judges you on, or he says they're already on his list, below his bar or not something he can measure.",
      "Promises to a parent, a coach on your staff or your boss now count, not only promises to players.",
      "\"Heard as\" now says how a line went with each person, on their own scale, and when you didn't answer their question.",
      "People stop asking the same question over and over once you've answered.",
      "Asked why he should keep his job, a coordinator makes his case.",
    ],
  },
  {
    v: "0.2.588", date: "16 Nov", title: "College followers retuned so the biggest stars reach 500,000 and 2 more",
    notes: [
      "At the end of your first spring, Ground writes you a report: your goals as verdicts, the kids you found, where your ground grew and who outworked whom.",
      "Your goals read as Met, Mostly, Partly or Missed, each with why",
      "Your finds show with their faces and how far they have bloomed",
      "Your ground by area, February to August, with lost ground marked",
      "Who outworked whom in each area, counted in signs",
      "The miss, a setback, your AD's word and the paper's",
    ],
  },
  {
    v: "0.2.585", date: "16 Nov", title: "Ground lane G8: a following and famous names (GR42-GR46) and 1 more",
    notes: [
      "The people you talk to now know what the talk means to them, and they answer what you actually said.",
      "A coordinator you inherit fights for his job: he makes his case from his record, asks where he stands and takes bad news his own way.",
      "Everyone knows who you are in the room, so nobody mixes you up with the coach you replaced.",
      "Small talk stays small: ask about the family and he tells you about his family.",
      "An offer gets a real answer before he brings up his own worries.",
      "The talk page now shows what's at stake before you start.",
    ],
  },
  {
    v: "0.2.582", date: "16 Nov", title: "Ground lane G6: the offseason campaign and 2 more",
    notes: [
      "Combine numbers now look like a real combine: real jump averages by position, the odd explosive leaper, and agility records that are hard to touch.",
      "The vertical and broad jump spread like real ones, and strong legs help.",
      "Quarterbacks and backs no longer broad jump like receivers.",
      "The very fastest 3-cones and shuttles are rare, as they are in real life.",
      "Drill results are no longer capped, so a record-breaking day prints as it happened.",
      "Every coach, trainer and family on your ground now has a page: his face, what he keeps of you in his own words, and what your standing gets you.",
    ],
  },
  {
    v: "0.2.579", date: "16 Nov", title: "Ground lane G4: news in order, leads, finds and your board from your ground and 3 more",
    notes: [
      "The people on your recruiting ground now feel like people: coaches, families, counselors, trainers and 7v7 men each remember you, talk to each other and take what you built with them to their next job.",
      "A recruit's family remembers you for his younger brother, and never forgets a lie.",
      "Word gets around: what you do for one coach's kid reaches his staff and his area.",
      "When a coach moves on, your standing goes with him, and his successor may already know you.",
      "Each person tells you a new level in their own words, and their page shows why you stand where you do.",
      "Players now come in the real range of body shapes for their position, so you'll find short-armed tackles, long-armed corners, light receivers and the occasional huge guard.",
    ],
  },
  {
    v: "0.2.575", date: "16 Nov", title: "Ground lane S2: the meeting at the map",
    notes: [
      "Monday's meeting is a screen: your coordinator's letter, a map of where he wants to send each man, and the week day by day, which you can change by dragging.",
      "A new Ground screen, The meeting, with the road days and your free hours for the week.",
      "Each place that wants a man is a numbered stop on the map, with who goes and where you stand there.",
      "Your coordinator puts one call to you each week; make it or leave it to him.",
      "Drag a man to another day to change his road, or hand the whole week to your coordinator.",
    ],
  },
  {
    v: "0.2.574", date: "16 Nov", title: "Ground lane S1: the map screen and the school page and 4 more",
    notes: [
      "The people around the kids now remember you. Every high school coach and counselor you deal with has a standing with your staff, from Stranger to Home, and it moves with the work you put in.",
      "Your evaluation days, calls, camps and campus visits build standing with each kid's coach, his coordinators and his counselor.",
      "When a coach moves up a level he tells you himself, in his own words, after lunch in his planning period.",
      "Ground you stop working fades, and your staff warns you before a level slips, with who was there instead.",
      "Pulling an offer or breaking a promise costs you with his coach for years. His kids starting, graduating and getting drafted with you pays.",
      "Your own high school and hometown know you from day one.",
    ],
  },
  {
    v: "0.2.569", date: "16 Nov", title: "Playing time grade reads next fall's depth chart and 5 more",
    notes: [
      "You can now sign recruits, transfers and your own players to NIL deals of up to four years. Locking in a star costs a buyout for anyone who wants to poach him.",
      "A Years box sits beside every NIL promise and portal offer, and a Term column is on the NIL desk.",
      "Some players value a guaranteed deal and take less a year for it. Others would rather sign year to year and sell themselves again.",
      "A player who transfers out of a multi-year deal owes half of what's left. The school that takes him pays it to you.",
      "Rival schools now lock up their own blue-chips and top transfers, and renegotiate to keep them.",
      "If a new build is too buggy to play, Settings can now put an older one back, and your saves are copied aside first.",
    ],
  },
  {
    v: "0.2.561", date: "16 Nov", title: "Progression step 3: upper and lower body strength, the bench at the real slope, the weight room by body knack and 2 more",
    notes: [
      "You can now end any meeting just by saying goodbye. Nothing is held against you for leaving, and anything you didn't argue stands as it was.",
      "Say goodbye, goodnight or \"that's all\" and the meeting wraps up, or press Wrap it up.",
      "Your boss's goals stand as he set them when you don't argue them.",
      "When a real call is still yours (keeping a holdover, a punishment, a release), the talk ends and that choice waits for you on the page.",
      "The meeting's old answer buttons stay out of the way while you're talking.",
      "Every NFL club now grades draft prospects for its own scheme, so boards split the way real ones do.",
    ],
  },
  {
    v: "0.2.558", date: "16 Nov", title: "Progression update steps 1-2: the split ceiling, knacks, skill projects, AI bets, no yearly cap and 2 more",
    notes: [
      "When you tell a player he's your guy and mean it, he weighs it like a real commitment. Players who trust you take it to heart, and empty praise stops piling up.",
      "A line where you stake yourself on a player now counts for far more than a pile of compliments.",
      "How much he buys it depends on his trust in you, the promises you've kept or broken with him, and where he sees himself on the team.",
      "A player who doesn't trust you shrugs off the big line and trusts you a little less.",
      "Saying it again right away adds nothing, and saying it again soon adds little.",
      "Generic praise counts a little each time and stops adding up after three or four lines.",
    ],
  },
  {
    v: "0.2.555", date: "16 Nov", title: "A world from a newer game says \"update the game\", not a reader error and 2 more",
    notes: [
      "A high school receiver's film now shows the routes he really ran, so a starting tight end's hands and route running read from his tape even when he caught few balls.",
      "Routes on a high school record come from his snaps on pass plays, not his catches.",
      "Tight ends who mostly block but start get their hands and routes read on film.",
      "Backs and receivers are counted by how often each really runs a route.",
      "A pass-heavy school's receivers show more routes than a run-heavy school's.",
      "Small colleges now sign classes the way real ones do: as many as their roster has room and need for.",
    ],
  },
  {
    v: "0.2.552", date: "15 Nov", title: "Player page: when he can sign and his first day with you and 9 more",
    notes: [
      "College programs now spend on their support staff from what they really earn, and strength coaches, trainers and other support heads move between jobs like real people.",
      "The richest athletic departments spend the most on staff and recruiting, with no ceiling per level.",
      "Support staff heads have an age and a salary, and the best get hired away by programs and clubs that pay more.",
      "A new head coach usually brings his own strength coach with him.",
      "An interim head coach leaves the support staff as it is.",
      "The star on a player's page now fills in once he's on your players, so you can tell at a glance.",
    ],
  },
  {
    v: "0.2.541", date: "15 Nov", title: "NFL pay: records can jump, the elite out-earn the top, real markets by era (realism audit 1, 3, 4)",
    notes: [
      "The NFL's money now works like the real market: the very best at a position can set records far above everyone else, and clubs bid against each other for the top free agents.",
      "An elite player is paid for how far he stands above the rest, so the best back costs much more than a good starter.",
      "Free agents with several suitors see clubs top each other's offers, as far as each club's price and cap room go.",
      "Each position's market follows its era: backs were paid far more in the late 90s, quarterbacks and receivers rose in the 2020s.",
      "After today the markets move with the deals your league signs.",
    ],
  },
  {
    v: "0.2.540", date: "15 Nov", title: "Live talks v3: the older meetings play on the conversation, not on chips and 44 more",
    notes: [
      "AI schools now offer and take commitments week by week through spring, summer and fall, not on five fixed dates. Each recruit commits on his own day, or sooner when one school clearly leads.",
      "You can now type straight to the man across the money desks: an agent on a contract, another club's GM on a trade, or a recruit about NIL money. He answers in his own words on a real call.",
      "Typing on a contract page calls his agent, and your words go into that call.",
      "Typing on the trade desk calls their GM about the man you're after.",
      "Typing on a recruit's NIL promise calls him, and the usual contact rules apply.",
      "The old \"bring it up\" fact buttons are gone, so nothing tells you what to say.",
      "Depth chart: a man who is out keeps the place you give him",
      "Film reads what your tape showed: a skill his record barely tested, blocking in space",
      "Plus 37 smaller changes under the hood.",
    ],
  },
  // auto:end
  {
    v: "0.2.492", date: "15 Nov", title: "Talks hear more of what you type",
    notes: [
      "When you type a line in a talk, the game now tells you what it heard beyond the closest option, and the person answers what you actually said.",
      "A pitch like \"you'll get the kicks\" counts as a promise, the way the chip would.",
      "A short \"Heard as\" line under your words shows the extra it caught.",
    ],
  },
  {
    v: "0.2.491", date: "15 Nov", title: "Type your own line in every one-on-one",
    notes: [
      "Every one-on-one now has a box to type your own line. It scores the way a chip does, so wording alone never beats the best chip.",
      "A fact he can check is checked as you say it: injuries, a teammate's status, his case, his place and your record. A false one costs trust.",
      "A line on another point turns the talk to it, just as its chip would.",
    ],
  },
  {
    v: "0.2.490", date: "15 Nov", title: "The game saves itself",
    notes: [
      "About every 10 minutes of play the game saves in the background, without freezing the screen.",
      "A quiet line over the Save button says when it last saved. The autosave sits on the Load page, one per coach.",
    ],
  },
  {
    v: "0.2.489", date: "15 Nov", title: "NFL trade finder",
    notes: [
      "Shop a player around the league over several days, quietly or openly. Offers arrive with their own expiry dates.",
      "Counter, ask for more, or name a rival team. A bluff that gets caught costs a year of trust.",
      "Closing a deal covers owner sign-off, the new contract and the physical, with a draft night step.",
      "It opens from the trades screen, or from \"shop\" on a player.",
    ],
  },
  {
    v: "0.2.488", date: "15 Nov", title: "Typed facts are checked, and a smarter skill hover",
    notes: [
      "Typed facts to a recruit or transfer are checked against what he can see. True facts work like the chip, and a false one costs you.",
      "Typed promises (start, role, position, early chance, no redshirt) ask first, then count as the chip's promise.",
      "Hovering a skill name on a player's page now explains what it does at his position.",
    ],
  },
  {
    v: "0.2.486", date: "15 Nov", title: "Rival coaches make promises, and a focus counts",
    notes: [
      "Rival coaches now promise recruits and transfers, then keep or break the promise. A broken one costs the player's trust and can send him to the portal.",
      "The training focus you set for a player grows that skill: about half again at 30 minutes a week, double at 60, up to his ceiling.",
      "A veteran's focus slows that skill's decline.",
    ],
  },
  {
    v: "0.2.483", date: "14 Nov", title: "Big-moment reveals and one team page",
    notes: [
      "Commitments, signings, hires, firings, titles, records and the Hall of Fame class play as full-screen reveals. Replay any of them on Your career, Big moments.",
      "Every team page now has the same top: four tiles, the three key players, strength by unit and the next four games with win chances.",
      "NFL pages add cap shares and deals ending. College pages add class needs, the portal and the recruiting pipeline by state.",
    ],
  },
  {
    v: "0.2.481", date: "14 Nov", title: "What pulls him, a class filter and a real recruiting budget",
    notes: [
      "Recruit and transfer files show what pulls a player, with a line for your school.",
      "The depth chart has a class filter, and a failed live-talk call no longer loses what you typed.",
      "When your athletic director says yes to a bigger recruiting budget, your recruiting grows and coaches on the road reach a second school.",
      "Promises at the press conference and in first team meetings now count in your reviews, for three seasons in full.",
    ],
  },
  {
    v: "0.2.474", date: "14 Nov", title: "The Attributes tab lines up and zooms",
    notes: [
      "Your read and your staff's read line up digit over digit, red for yours and grey for theirs.",
      "Hover a bar to open a close-up of that skill's marks, four points either side.",
    ],
  },
  {
    v: "0.2.471", date: "14 Nov", title: "December signees count once",
    notes: [
      "A December signee counts once, so February and the late signing days show the real room in each class.",
      "Under the House roster limit, AI programs take January transfers up to their open places.",
      "Recruiting results change, so worlds play differently from this build on.",
    ],
  },
  {
    v: "0.2.470", date: "14 Nov", title: "Call and text recruits, portal exits read the whole player",
    notes: [
      "You can call or text a recruit. A call counts as half a sit-down and a text as a quarter, under each era's real contact rules.",
      "His family calls you when something he cares about changes, like a coach leaving or a better school's offer.",
      "You can promise a recruit a start, a real role, his position or no redshirt. Once he signs, the promise is checked against his first season.",
      "A college player's choice to leave now follows his own view of his future, your promises, his trust in the head coach and his pay.",
      "Hovering your own outlined bar on the rating card opens your read, next to the staff's range.",
    ],
  },
  {
    v: "0.2.467", date: "14 Nov", title: "The Attributes screen, and your own lines in talks",
    notes: [
      "The Attributes tab has a new screen: the eight families with a word each, a reason for each skill in play, and a compare with one of your players.",
      "Your moves and recruit pitches in talks draw on a bank of written lines.",
      "Interview rooms offer your scouts' worst worries as picks, and you can call his coach, a teacher or his trainer as a source.",
      "In a staff meeting you can argue your own case, which can move the plan one step.",
      "A transfer's file has a \u201cWhere you stand\u201d panel, and the depth chart has pills for the next three falls.",
    ],
  },
  {
    v: "0.2.462", date: "14 Nov", title: "Your team's rooms read the facts",
    notes: [
      "The team talk opens on the real day, result, streak and next game. Parent calls name the real complaint, and exit meetings talk about why he is leaving.",
      "A staff read is never narrower than 2 wide, so a very weak skill no longer shows a falsely certain \u201c1\u201d.",
      "The Testing day on the Body tab can show his numbers as of Senior year, the combine or next year in the NFL, and the plot shows his future weight.",
      "The race page's squares narrow as your talks and his words give reasons. A letter and its pop-up come at the same stop.",
      "The desk takes typed questions on the stat finder, squad bar, Find and opponent pages, and the scout box can write a report.",
    ],
  },
  {
    v: "0.2.454", date: "14 Nov", title: "Charted data for every seat, typed lines in the pregame",
    notes: [
      "NFL, FBS and FCS games are charted snap by snap the morning after, for every seat.",
      "Type a line in the pregame and halftime rooms. At halftime it lands only when you confirm.",
      "The Needs map has an \u201cIn the portal\u201d card. The depth chart keeps its familiar plates, and each backup is its own row you can drop a man into.",
      "Once you have looked at a player, your own read of his potential judges him, with the staff's read beside it.",
      "One-on-ones open on what weighs on the player, and an arrest always comes up.",
    ],
  },
  {
    v: "0.2.447", date: "13 Nov", title: "Recruit talks built on the facts, and this page",
    notes: [
      "A home visit, a call or a sit-down opens on what changed, asks what the recruit weighs, then reacts to your answer. His file shows what your talks showed.",
      "AI schools build the same case, so who signs where can change.",
      "After a loss, praise only counts a game from the last 7 days.",
      "An agent whose stated want fits your club can ask for a little less.",
      "The opening screen has this What's new page.",
    ],
  },
  {
    v: "0.2.444", date: "13 Nov", title: "Type your own lines in talks",
    notes: [
      "In a live talk you can type a line and it lands on the closest option, the same as clicking it.",
      "Staff interviews now work from the first question to the last.",
      "Anything you can't take back now asks before it goes through, including “Let the staff coach it”.",
      "Game day links open the tab they name, and a few game page tabs that wouldn't respond work again.",
      "Pros with a recent off-field case cost clubs a little less. This shows in new worlds.",
    ],
  },
  {
    v: "0.2.441", date: "13 Nov", title: "Easier depth chart, sharper boss talks",
    notes: [
      "Drag a player on the depth chart with a plain mouse drag. The spot lights up, and Esc cancels.",
      "In a season review, owning a miss earns a little trust from your boss. Pointing to a goal you hit earns more.",
      "Reporters ask about an arrest or a court date the week before the next step.",
      "The talk boxes are in, and the staff and scouting rooms now speak from your team's real record.",
    ],
  },
  {
    v: "0.2.437", date: "13 Nov", title: "People talk about your season",
    notes: [
      "Bosses, boosters, your old coach and the GM now talk about the season you are really having.",
      "The press asks about a player's actual case, at the step it has reached.",
      "A tackle you put at left tackle stays at left tackle on every snap.",
    ],
  },
  {
    v: "0.2.434", date: "13 Nov", title: "No more talk about old games",
    notes: [
      "Talks and the press only bring up a game from the last week.",
      "Each career keeps its own notebook.",
      "A school with a clearly better offer pulls a money-first recruit harder.",
    ],
  },
];

// The page body: one panel a version, the one you are on marked. `e` escapes.
function changelogBody(list, current, e) {
  return `<div class="pg-body"><div class="pg-col pg-log">${list.map((x, i) => `<section class="pg-panel pg-rel${i === 0 ? " new" : ""}">
      <div class="pg-rh"><span class="v">${e(x.v)}</span><span class="d">${e(x.date)}</span>${x.v === current ? '<span class="pg-tag pro">You are on this one</span>' : ""}</div>
      <h3>${e(x.title)}</h3>
      <ul>${x.notes.map((n) => `<li>${e(n)}</li>`).join("")}</ul>
    </section>`).join("")}</div></div>`;
}
