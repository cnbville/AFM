/* The Handbook's pages (update 3, lane 8: H8, H9; board 38). One plain page
 * per system, written from docs/reference/how-it-works/: what it is, how it
 * works in four points, what each level does differently, where you see it
 * and the related pages. The live parts (your own players, your staff's
 * reads, the dates coming up) come from the game: `l8_page` in
 * crates/afm-game/src/lane_handbook.rs.
 *
 * Adding a page (lanes 7, 9, 10 and 11): call hbAdd({...}) from your own
 * screen file with the fields below, point your screens at it with
 * hbScreen(screen, id, on), and add its live example as one arm of `live`
 * in lane_handbook.rs (it returns the same shape as the others). A page
 * with no live arm still shows; its "From your save" box just stays out.
 *
 *   id      short key ("film")         group   one of HB.groups, or a new one
 *   title   "Film room"                lede    two sentences at most
 *   how     four [title, text] points  lv      {hs, college, nfl}: one line each
 *   where   [[label, screen, args]]    rel     related page ids
 */
"use strict";

const HB = { groups: ["The players", "Building the team", "Running the team"], pages: [], screens: {} };
function hbAdd(p) {
  const i = HB.pages.findIndex((x) => x.id === p.id);
  if (i >= 0) HB.pages[i] = { ...HB.pages[i], ...p }; else HB.pages.push(p);
  if (p.group && !HB.groups.includes(p.group)) HB.groups.push(p.group);
}
// A screen's "?" opens this page; on lists what the screen itself shows:
// [title, text, pin] where pin is a column or label on the screen that the
// panel marks with its number (found by its text).
function hbScreen(screen, id, on) {
  HB.screens[screen] = { page: id, on: on || null };
}
const hbPage = (id) => HB.pages.find((p) => p.id === id);

// ---------- The players ----------
hbAdd({
  id: "ratings", group: "The players", title: "Ratings",
  lede: "Every player has 67 ratings from 1 to 99, one for each thing he does, on the same scale from high school to the NFL. A position rating weighs the ones that position needs.",
  how: [
    ["One scale for every level", "A 60 is a 60 anywhere. A high school star and an NFL starter sit on the same line, so most high schoolers rate far below the pros."],
    ["Position ratings weigh what the spot needs", "His rating at a position mixes the ratings it uses: a quarterback's arm, accuracy and decisions, a corner's speed, coverage and ball skills. Height and weight don't count in it; they count in how well he fits the spot."],
    ["You see your staff's read, never the number", "Every rating is a range from your staff. Your own players read narrow; other teams' players read wider, and skills nobody has watched show as not seen."],
    ["The level rating ranks him", "\"81 among 3A juniors\" places him against players his age at his level. It is a label for comparing and changes nothing on the field."],
  ],
  lv: {
    hs: "Ratings grow fastest from 14 to 18. Level ratings compare him with his class in his state and classification. You see your own players every day; other schools' only on film.",
    college: "Level ratings compare him with his division: FBS, FCS or small college. Recruits arrive with wide ranges that narrow once your coaches work with them.",
    nfl: "No level rating: a pro is his rating, on the same 1-99 scale. Bodies are mostly done; reads and technique still grow into the late twenties.",
  },
  where: [["Squad", "roster"], ["Player page, Ratings tab", "player"], ["Depth chart", "depth"], ["Compare players", "compare"]],
  rel: ["body", "scouting", "development"],
});
hbAdd({
  id: "body", group: "The players", title: "Body and speed",
  lede: "A player's body sets what his ratings can do on the field. Speed is his own gift, not a sign of football skill: a burner can be raw and a slow player polished. The forty is what the stopwatch says, from his speed, his acceleration and his weight.",
  how: [
    ["Speed is top speed for his size", "Acceleration is how quickly he gets there. How fast he plays mixes the two by position: linemen lean on burst, receivers and the secondary on top speed. Of two players with the same speed rating, the stopwatch favors the lighter one."],
    ["The forty comes from both and his weight", "Speed counts three times as much as acceleration, and every pound over 220 adds a little time. Lighter than about 175 lb, less weight stops helping."],
    ["Measured or listed", "A forty is exact once someone times him electronically: a camp, spring testing, the combine. Until then your staff shows a range, and school listings run tall and heavy."],
    ["Bodies grow by the date", "Height follows the real growth curve and weight fills out by position, so the oldest in a class are the biggest. Speed grows with the body: a senior has about 96% of his top speed. The weight room adds pounds toward his position's weight, never past it."],
  ],
  lv: {
    hs: "Most forties are hand-timed and run 0.2 to 0.3 seconds fast, so your staff and college coaches discount them. Bodies are still growing, nearly full height by 17 and heavier through senior year: a receiver adds about 20 lb from 14 to 17, a lineman 30 or more.",
    college: "Spring testing and pro days give the times on file; pro day forties run fast, and NFL clubs discount them. Receivers and backs arrive near their grown weight; linemen fill out until about 22, and a late-growing tackle can add 60 lb. More weight at the same speed rating means a slower forty.",
    nfl: "The combine forty is the number everyone quotes. After about 28, speed fades a little each year, and backs and corners feel it first.",
  },
  where: [["Player page, Body tab", "player"], ["The speed hover on any player", "roster"], ["Recruiting board, measurables", "recruit_board"], ["Draft board, combine numbers", "draftboard"]],
  rel: ["ratings", "scouting", "development"],
});
hbAdd({
  id: "scouting", group: "The players", title: "Scouting ranges",
  lede: "Your staff never knows a player's true ratings. Every rating you see is a range from the people who watched him: narrow when they know him well, wide when they don't.",
  how: [
    ["Who watched, and how well", "Each coach and scout has his own eye. A sharp eye narrows the range and puts it nearer the truth; a weak one misses both ways."],
    ["More looks help, up to a point", "Each look cuts the random part of a read, but part of every misread is the reader's own lean, and more looks from him won't fix it. A second person's look helps more than a tenth look by the same one."],
    ["Tests measure the test", "Lasers and scales are exact for what they measure, so speed is nearly exact once he's timed. Football skills come only from film, practice and games."],
    ["Potential is words, never a number", "Your staff says how far he can go (\"could start in the Big 12\") and how sure they are. Small schools are harder to read: fewer people see them and the competition tells you less."],
  ],
  lv: {
    hs: "You see your own players every day, so your ranges are narrow. College coaches see them on visits, at camps and on film, so their reads are wider than yours.",
    college: "Your position coaches and recruiting staff read recruits from film, camps, visits and their high school coaches. Your own players read narrow once your coaches work with them.",
    nfl: "Your scouts and the combine read the draft class; pro scouts read other clubs' players from film. Every club gets the same combine numbers and reads them its own way.",
  },
  where: [["Player page, Ratings tab", "player"], ["Scouting department", "scouting"], ["Recruiting board", "recruit_board"], ["Draft board", "draftboard"]],
  rel: ["ratings", "development", "recruiting"],
});
hbAdd({
  id: "development", group: "The players", title: "Development and hidden potential",
  lede: "Players grow toward ceilings nobody can see. Your staff judges how far a player can go in words, never a number, and those judgements can be wrong.",
  how: [
    ["The year decides, the weeks deliver", "Once a year decides how much a player grows; training decides when it lands, week by week, in the skills his position uses."],
    ["Potential moves", "Every rating has its own ceiling, and ceilings move: work ethic, coaching, coachability and playing time push them, injuries pull them down. Young players' ceilings move most; from about 27 they stop rising."],
    ["Coaching, playing time and mood", "Good teaching speeds growth and poor teaching can cost him. Coachable players take more from both. Players who play grow more than players who sit, and a happy player a little faster."],
    ["Breakouts, slumps and age", "Some years a player jumps and some years he slips. Speed and quickness fade from about 27, strength in the early 30s, reading the game last."],
  ],
  lv: {
    hs: "Bodies and ratings change fastest from 14 to 18. Technique and the mental game grow more slowly in high school than in college.",
    college: "College coaching speeds technique and the mental game. A redshirt year still grows him, but a starter grows more. Small-college players learn game skills a little slower.",
    nfl: "Most players are near their ceilings. Technique and reads can still grow into the late twenties while speed starts to fade.",
  },
  where: [["Player page, Development tab", "player"], ["Training", "training"], ["Squad", "roster"]],
  rel: ["ratings", "training", "people"],
});
hbAdd({
  id: "tendencies", group: "The players", title: "Tendencies",
  lede: "A tendency is how often a player tries something, never how well it works. His ratings decide how well; his habits decide how often.",
  how: [
    ["15 habits in 8 groups", "From a quarterback's scrambles to a back's bounce outside, each habit sits on a 1 to 20 scale, with about 10 the typical NFL player."],
    ["Where they come from", "Every player starts with his own habits at 14, nudged a little by his body and his mind."],
    ["They set young", "Results, your staff's style, the scheme, age and big injuries move them each year: fully until about 22, barely after 26."],
    ["Instructions", "You can ask a player to do something more or less often. Disciplined, coachable players who trust you obey more; under pressure, old habits creep back."],
  ],
  lv: {
    hs: "Habits are still forming. What your staff stresses now can stick for a career.",
    college: "Habits still move, but less each year. A new staff's style can reshape a young player.",
    nfl: "Habits are mostly set, so instructions are your main tool. How often he obeys rests on his discipline, his coachability and his trust in you.",
  },
  where: [["Player page, Overview", "player"], ["Plans, instructions", "plans"]],
  rel: ["gameplan", "people", "development"],
});
hbAdd({
  id: "links", group: "The players", title: "Dynamic Links",
  lede: "Dynamic Links are physical gifts that work together: size, reach and leap make a Highpointer. Each is a small edge in its own situations, not a boost to everything.",
  how: [
    ["Two to four body traits", "A link needs every part near the top at his position, on real combine numbers. The weakest part sets its strength, so one elite trait alone makes none."],
    ["It grows smoothly", "A link starts once every part is in about the top 30% at his position and is full near the top 5%. There is no cliff at a number."],
    ["An edge, not a boost", "A full link is worth about five rating points, only in its own situations. His skills still decide how he uses it: a Highpointer with poor hands still drops balls."],
    ["Links stack", "Links that act in different situations each count in full. Two in the same situation: the stronger counts in full, the other at half."],
  ],
  lv: {
    hs: "Rare in high school: most boys are still growing into their frames, so your staff says what a body projects to.",
    college: "Staffs prize them, and spot them before the recruiting sites do: a physical gift can earn a lightly rated player a big offer.",
    nfl: "Common at the top of the NFL. Once he has tested, clubs grade the gift into his draft value.",
  },
  where: [["Player page, Body tab", "player"], ["Recruiting board", "recruit_board"], ["Draft board", "draftboard"]],
  rel: ["body", "scouting", "draft"],
});

// ---------- Building the team ----------
hbAdd({
  id: "recruiting", group: "Building the team", title: "Recruiting",
  lede: "You find high school players, judge them, win them over and sign them, inside the NCAA's calendar. Stars are the recruiting sites' opinion; your board runs on your staff's grades.",
  how: [
    ["Stars are an opinion", "The sites give stars from what they've seen and where they think he's headed. They see late bloomers and small-school players late, and their misreads last."],
    ["Interest is earned", "A recruit weighs the level, your program's name, playing time, distance from home, academics, development, money and the people he knows. Often a parent's voice decides."],
    ["The calendar decides contact", "Contact, evaluation, quiet and dead periods decide when you can see him and talk to him. Official visits and offers have their own dates."],
    ["Commits and signing", "Most players commit in waves through spring, summer and fall; the early signing period in December and signing day in February make it binding. A commit can flip until he signs."],
  ],
  lv: {
    college: "You recruit on the NCAA calendar, with official visits, offers and two signing periods.",
    hs: "You don't recruit; you help. College coaches visit your practices, and your calls and film help your players get offers.",
    nfl: "No recruiting. The draft and free agency take its place, and your scouts know these players from college.",
  },
  where: [["Recruiting", "recruiting"], ["Recruit board", "recruit_board"], ["Visits", "visits_desk"], ["Calendar", "calendar"]],
  rel: ["scouting", "portal", "calendar"],
});
hbAdd({
  id: "portal", group: "Building the team", title: "Portal and NIL",
  lede: "College players can leave through the transfer portal and earn money from their name, image and likeness. Both decide who stays on your roster.",
  how: [
    ["Why players enter", "Playing time, being buried on the depth chart, mood and a coaching change all push a player toward the portal. Happy players who play rarely leave."],
    ["The windows", "Players enter in the portal windows after the season and in the spring, or when their head coach leaves."],
    ["Recruiting the portal", "A transfer picks a school by his own wants, like a recruit: playing time, money, the level and the people he knows."],
    ["NIL", "From July 2021 players earn from brand deals and the school's collective. Stars with a big following earn the most."],
  ],
  lv: {
    college: "Your roster changes every winter and spring. Keep players playing and happy, and watch the NIL budget.",
    hs: "No portal at high school. Your players' next step comes through recruiting.",
    nfl: "No portal in the NFL. Free agency, trades and waivers move players, and contracts take NIL's place.",
  },
  where: [["Portal", "portal_desk"], ["NIL", "nil_desk"], ["People", "people"]],
  rel: ["recruiting", "people", "contracts"],
});
hbAdd({
  id: "draft", group: "Building the team", title: "The draft",
  lede: "Each spring the NFL's 32 clubs pick the year's best college players over seven rounds, each from its own board.",
  how: [
    ["Every club has its own board", "A club ranks prospects from its scouts, coaches, the combine and the public's grades. A GM who trusts his staff leans on the public less."],
    ["Need and position", "Quarterbacks and pass rushers are worth more; clubs draft for need, more so after the first two rounds."],
    ["Testing and red flags", "The combine adds the same measurements to every club's file. Physical gifts lift a grade; suspensions and character worries cut it, and small schools are marked down."],
    ["Declaring, and after", "Underclassmen declare by mid-January, mostly the ones graded for the first rounds. Players nobody drafts sign where they see the best path to the 53."],
  ],
  lv: {
    nfl: "You run the board and make the picks on the clock. Draft-day trades are priced on each GM's own value chart.",
    college: "Your players declare or come back. Who gets picked tells recruits what your program does for them.",
    hs: "Years away for your players, but every college coach recruits with it in mind.",
  },
  where: [["Draft board", "draftboard"], ["Pre-draft", "predraft"], ["Scouting department", "scouting"]],
  rel: ["scouting", "contracts", "body"],
});
hbAdd({
  id: "contracts", group: "Building the team", title: "Contracts and the cap",
  lede: "An NFL contract is salary plus bonuses, under a hard cap. A signing bonus is paid at once but counts in equal shares over up to five years.",
  how: [
    ["The cap hit", "What counts this year: his salary plus this year's share of his signing bonus."],
    ["Dead money", "Cut or trade him and every bonus share still to come counts at once, plus any guaranteed salary. After June 1 it splits over two years."],
    ["Tags and tenders", "One franchise or transition tag a year keeps a star, at a price set by the top salaries at his position."],
    ["Free agency", "A free agent's price comes from where he ranks at his position, cut for age. He weighs money after state tax, guarantees, role and the city."],
  ],
  lv: {
    nfl: "Real contracts under the real cap: bonuses, guarantees, tags, extensions and dead money.",
    hs: "No contracts or salaries. Your budget comes from the school district and the booster club.",
    college: "Scholarships and the program's NIL money instead of contracts; players can leave through the portal.",
  },
  where: [["Contracts", "contracts"], ["Salary cap", "fo_cap"], ["Finances", "money"]],
  rel: ["draft", "people"],
});

// ---------- Running the team ----------
hbAdd({
  id: "gameplan", group: "Running the team", title: "The game plan",
  lede: "Your plan sets what your team runs: the offense and defense, how firmly you want them followed, and the opponent's habits you go after. A good plan wins through the players, never instead of them.",
  how: [
    ["Your guidelines, their calls", "You set the system and how firmly you want it. Your coordinators call the plays and lean toward what you asked; down, distance, score and clock still rule."],
    ["A caller's quality", "A better play caller picks better plays for the moment. He gets no bonus on the field: the play still has to win its battles."],
    ["Film study", "Your staff breaks down the opponent's snaps. More film and a sharper staff find more habits to go after."],
    ["Installs take time", "A system your coordinators know well is called better. Switching costs until the staff and the players learn it."],
  ],
  lv: {
    hs: "As head coach you call both sides unless your coordinators do. Many players go both ways, so plans lean on your best athletes.",
    college: "Coordinators call each side, and NCAA hour limits mean a big install competes with everything else in the week.",
    nfl: "Every snap is scouted. Opponents key your formations, so a habit you never break gets punished.",
  },
  where: [["Plans", "plans"], ["Game day", "gameday"], ["Play designer", "designer"]],
  rel: ["training", "tendencies", "calendar"],
});
hbAdd({
  id: "training", group: "Running the team", title: "Training",
  lede: "Training decides when a player's growth lands and how fresh he is on game day. It doesn't decide how much he grows in a year.",
  how: [
    ["The week", "Each Monday the week's practice runs: strength, speed, conditioning, technique, contact, kicking and the mental side. Recovery and team work grow nothing but keep players fresh."],
    ["The rules", "Every level has its limits: NCAA hours, the NFL's CBA, your state's contact rules. AI staffs keep to them; you can push past on purpose and take the risk."],
    ["Individual plans", "Set a focus for a player and his growth lands there first. A body plan can add or cut weight."],
    ["Practice is a battle too", "Good practice reps move your staff's depth chart a little, and hard contact risks injuries."],
  ],
  lv: {
    hs: "Your state sets contact limits: most allow about 90 minutes of full contact a week in season.",
    college: "In season, 20 countable hours a week, at most 4 a day, with a day off; out of season 8 hours, up to 2 of them film.",
    nfl: "The CBA sets each phase: strength only in phase one, no live contact in OTAs, 14 padded practices across the season.",
  },
  where: [["Training", "training"], ["Plans", "plans"], ["Player page, Development tab", "player"]],
  rel: ["development", "gameplan", "calendar"],
});
hbAdd({
  id: "people", group: "Running the team", title: "People and morale",
  lede: "Every player and coach is a person, with a personality, friends, a family and a mood. Mood changes how a player grows and whether he stays.",
  how: [
    ["Personality", "Work ethic, coachability, competitiveness and love of the game show on his page; ambition, loyalty, temper and the rest your staff learns over time."],
    ["Mood and morale", "Mood is set at camp each year, then talks, his life and the media move it. A happy player grows a little faster, an unhappy one slower."],
    ["What mood moves", "Unhappy college players lean toward the portal; unhappy pros hold out. A friend leaving hurts."],
    ["Chemistry", "A quarterback and his receivers time throws better after seasons together, and a secondary hands off coverage better."],
  ],
  lv: {
    hs: "Families, grades and jobs after school weigh on your players. Most of them won't play in college, and they know it.",
    college: "Playing time and the depth chart weigh most on mood, and NIL money adds its own pressure.",
    nfl: "Contracts and role weigh most. A grudge from a cut or a trade can last.",
  },
  where: [["People", "people"], ["Staff", "people_staff"], ["Inbox", "inbox"]],
  rel: ["development", "portal", "contracts"],
});
hbAdd({
  id: "calendar", group: "Running the team", title: "The calendar",
  lede: "The world runs on a daily clock with each level's real dates: games, signing days, the portal, the combine, the draft. Continue stops when something needs you.",
  how: [
    ["One day at a time", "Continue plays days until something needs your answer. Nothing pops up: it lands in your inbox and under Continue."],
    ["Each level's year", "High school runs from summer workouts to the state final; college adds recruiting periods, signing days and the portal; the NFL runs from the combine to the Super Bowl."],
    ["Periods decide contact", "Recruiting has contact, evaluation, quiet and dead periods; the NFL has its own scouting calendar."],
    ["What's coming", "The year strip and Ahead show the next dates that matter, with what each one asks of you."],
  ],
  lv: {
    hs: "Your state association sets the season: first practice, the playoffs and the state final.",
    college: "Recruiting never stops, and the contact calendar decides when you can talk to recruits.",
    nfl: "The league year starts in March with free agency; the draft is in April, camp in July, and the cut to 53 comes before week 1.",
  },
  where: [["Calendar", "calendar"], ["Schedule", "schedule"], ["Inbox", "inbox"]],
  rel: ["recruiting", "training", "draft"],
});

// ---------- which page each screen's "?" opens (H11) ----------
for (const [page, list] of Object.entries({
  ratings: ["roster", "player", "compare", "squads", "depth"],
  body: ["faces"],
  scouting: ["scouting", "scout", "proscout", "myplayers", "finder", "find_insider"],
  recruiting: ["recruiting", "recruit_board", "recruitboard", "recruit_file", "recruit_find", "recruit_race", "visits_desk", "hsside", "colleges_calling", "find_prospects", "find_offers"],
  portal: ["portal_desk", "nil_desk"],
  draft: ["draftboard", "predraft", "warroom", "draftrecord", "draftoutlook"],
  contracts: ["contracts", "fo_cap", "capboard", "fo_expiring", "fo_market", "fo_talk", "fo_trades", "fo_moves", "fo_office", "money", "money_league", "school", "market", "area"],
  gameplan: ["plans", "designer", "gameday", "live", "game", "games", "evaluate", "needs"],
  training: ["training"],
  people: ["people", "people_scene", "people_cast", "people_staff", "staff", "staff_person", "locker", "media", "office", "you", "coach"],
  calendar: ["home", "inbox", "calendar", "schedule", "year", "news"],
})) for (const s of list) hbScreen(s, page);

// What the main screens themselves show (the panel's "On this screen").
hbScreen("roster", "ratings", [
  ["Rating", "Your staff's read of how he plays now at his listed spot, as a range: narrow for your own players.", "Rating"],
  ["Class and age", "High school and college players by class, pros by years in the league.", "Class"],
  ["Mood and status", "How he feels about his place, and whether he's hurt, suspended or ineligible.", "Mood"],
]);
hbScreen("depth", "ratings", [
  ["The order", "Who starts and who's next at each spot. Yours where you set it, else your staff's."],
  ["Reads beside each name", "Your staff's range at that spot, so you can see the gap between a starter and his backup."],
  ["Readiness", "Fresh, sore or hurt: a starter who isn't ready plays worse than his read."],
]);
hbScreen("player", "ratings", [
  ["Your staff's read", "A range until your staff knows him well; narrow means sure. Speed is exact once he is timed.", "Ratings"],
  ["The level rating", "\"81 among 3A juniors\" ranks him against his class and level over the last five seasons."],
  ["Since last year", "The change since last season, with the reasons on the Development tab.", "Development"],
]);
hbScreen("training", "training", [
  ["The week", "Each day's work by family, and the hours the rules leave you."],
  ["Load", "How hard the week is. Push past the limits and the risk of a practice injury climbs."],
  ["Focus", "A player's own plan: where his growth lands first."],
]);
hbScreen("plans", "gameplan", [
  ["Your system", "The offense and defense you want; your coordinators build the playbook from it."],
  ["How firm", "Firm, a lean or his call: how strongly the coordinators follow your guidelines."],
  ["Instructions", "Ask a player to do something more or less often. He obeys most of the time, not always."],
]);
hbScreen("recruit_board", "recruiting", [
  ["Your grade", "Your staff's grade from film, camps and visits. The board sorts by it; the stars stay for comparison.", "Your grade"],
  ["Interest", "How much he likes you, from contact, visits, playing time and fit. Hover for his top reasons.", "Interest"],
  ["The contact calendar", "Contact, evaluation, quiet and dead periods decide when you can see him and talk to him."],
]);
hbScreen("recruiting", "recruiting", [
  ["Your class", "Who has committed, by position, against the scholarships you have."],
  ["Offers", "Who you've offered and where each one stands."],
  ["The calendar", "The period you're in today decides who you can see and talk to."],
]);
hbScreen("portal_desk", "portal", [
  ["Your players in the portal", "Who entered, why your staff thinks he did, and whether he can still be kept."],
  ["Transfers you can chase", "Players in the portal who fit your needs, with your staff's read of each."],
  ["The window", "When players can enter, and how many days are left."],
]);
hbScreen("nil_desk", "portal", [
  ["The budget", "What the collective can spend this year, and what is already promised."],
  ["Deals", "Who gets what. Money can keep a player, but it can also stir the room."],
  ["Brand deals", "Money players earn on their own, from their following."],
]);
hbScreen("contracts", "contracts", [
  ["Cap hit", "What counts against this year's cap: his salary plus this year's share of his signing bonus.", "Cap hit"],
  ["Through", "The last season of his deal.", "Through"],
  ["Room", "The cap, less every contract already signed for that year."],
]);
hbScreen("fo_cap", "contracts", [
  ["Cap hit", "What counts against this year's cap: his salary plus this year's share of his signing bonus."],
  ["Dead if cut", "What would still count if you cut him in March: every bonus share still to come, plus guaranteed salary."],
  ["Room", "The cap, less every contract already signed for that year."],
]);
hbScreen("draftboard", "draft", [
  ["Your board", "Your staff's order of the class, from your scouts, coaches and the combine."],
  ["Grades as ranges", "How sure your staff is: a wide range means few looks or a hard player to read."],
  ["Need", "Where your roster is thin. Clubs draft for need more after the first two rounds."],
]);
hbScreen("scouting", "scouting", [
  ["Your scouts", "Who reads what, how sharp each one's eye is, and the tastes that tilt his reads."],
  ["Assignments", "Where they go and whom they watch. More eyes on one player narrow the range faster than more looks by one."],
  ["The year", "Fall film, all-star games, the combine and pro days: each adds to your files."],
]);
hbScreen("calendar", "calendar", [
  ["The year strip", "Your season's phases on one lane and recruiting periods (or the draft year) on the other; the gold line is today."],
  ["Day cells", "Games, things that need you, recruiting or scouting, and the world, at most three a day."],
  ["Ahead", "The next dates that matter, with a countdown."],
]);
hbScreen("home", "calendar", [
  ["What's next", "Your next game and the next dates that matter."],
  ["Inbox", "Mail from your staff, your boss and the world. Nothing pops up; it all lands here."],
  ["Continue", "Plays days until something needs you."],
]);
hbScreen("inbox", "calendar", [
  ["Letters", "Every mail with who sent it and why. Some ask for an answer by a date."],
  ["Keywords", "Filter by job, players, recruiting, the portal, injuries, money and more."],
  ["Kept for 30 days", "Mail clears after 30 days unless you save it."],
]);
hbScreen("people", "people", [
  ["Mood", "How each player feels, and the biggest reasons your staff can see."],
  ["Talks", "What you said to whom, and how it landed."],
  ["Ties", "Friends, family and the people around the program."],
]);
hbScreen("gameday", "gameplan", [
  ["The opponent", "Your staff's scouting: their habits and the matchups they want."],
  ["Your plan", "What you'll run at them, and the habits you go after."],
  ["Kickoff", "Watch it, call it yourself, or let your staff call it."],
]);
hbScreen("schedule", "calendar", [
  ["Results", "Every game with its score and a link to its page."],
  ["What's next", "The next opponent and how your staff reads them."],
  ["The year", "The whole season's dates, including the ones that aren't games."],
]);
