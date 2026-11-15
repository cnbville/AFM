// Live talks v3, lane N6: fixtures for the six conversation boards (the shapes
// convo.js draws from `people_scene`, `people_player` and the scene's `talk`
// block). Used by convo-shots.js to draw each board in Chromium with no game
// running. Text follows the agreed mock-ups (afm/live-talks/v3-mockups/).
"use strict";
const NAVY = ["#1f3a6e", "#0b1426"], GARNET = ["#73000a", "#1a0407"], BLUE = ["#0085ca", "#101820"];
const line = (by, text, extra = {}) => ({ by, name: by === 2 ? extra.name || "Desmond Okoro" : by === 1 ? "You" : "", text, tell: null, ...extra });

const desmond = {
  colors: GARNET,
  scene: {
    id: 11, kind: "OneOnOne", kindName: "One-on-one", title: "No clean look all Saturday", dateText: "Tue 13 Oct", place: "Your office", today: true, opened: true, done: false, future: false,
    who: [{ kind: "player", id: 44, link: "[[p:44|Desmond Okoro]]", pos: "EDGE" }], hours: 6.5, outcome: [], docs: [], picks: [], subjects: [], next: null,
    bars: { trust: { from: 0.5, now: 0.6, words: "solid" }, mood: { from: -0.4, now: 0.35, words: "hopeful" } },
    say: { cmd: "people_say", args: { id: 11 }, limit: 400 },
    lines: [
      line(1, "Be straight with me. What's going on with you?", { heard: "a question about him · he opened up" }),
      line(2, "Every pass they put the back on me or slide the line my way. Thirty-something rushes Saturday and not one clean look. I'm not complaining about the work. I just want a shot at the guy.", { tell: "leans in, talks with his hands" }),
      line(1, "The chips are their call. Getting you free is ours. From Saturday we run twists with you in them, one rush in four, every game through the bowl.", { heard: "a promise about his rushes · hope, and some doubt" }),
      line(2, "Coach Kessler barely calls twists. If that changes, I'm all in. But I'll be counting.", { tell: "half a smile: he's heard promises before" }),
    ],
    talk: { persona: true,
      channel: "In person", minutes: { left: 11, of: 25 }, week: 10,
      left: [
        { k: "tiles", label: "His last 3 games", note: "from the play log", items: [{ n: 94, l: "pass rushes" }, { n: 11, l: "pressures" }, { n: 2, l: "sacks" }], text: "Slid to on 61%, chipped on 31%, doubled on 18%. Their calls." },
        { k: "chips", label: "On his mind", items: [{ lead: "Getting home", rest: "on the quarterback", hot: false }, { lead: "April", rest: "and the draft" }, { rest: "His brother **Isaiah**, DE, class of 2028" }] },
        { k: "word", items: ["Your word to him: **starts every game.** Kept 6 of 6.", "New today: **stunts on 1 in 4 rushes, through the bowl.**"] },
      ],
      after: { 2: [{ k: "logged", title: "In a stunt on at least 1 in 4 of his pass rushes, every game, through the bowl.", counts: "Counts from Sat 17 Oct", chart: { label: "1 in 4, promised", line: 0.25, bars: [{ l: "5 Sep", v: 0.12 }, { l: "12 Sep", v: 0.07 }, { l: "19 Sep", v: 0.11 }, { l: "26 Sep", v: 0.08 }, { l: "3 Oct", v: 0.1 }, { l: "10 Oct", v: 0.09 }, { l: "Sat", now: true }] }, facts: [{ l: "Judged on", t: "The line chart's stunt flag, game by game" }, { l: "Plan setting", t: "Free him up: on, simmed or live" }, { l: "He believes it", t: "Mostly. He's counting." }], staff: { name: "Ron Kessler", role: "DC", ini: "RK", text: "\"Our four-man fronts can carry a lot more twists. I'll script T-E twists to his side.\"" } }] },
      right: [
        { k: "stand", label: "Where this talk stands", mood: "Hopeful", was: "Frustrated", feel: [{ l: "Trust", t: "Solid" }, { l: "Patience", t: "Plenty" }, { l: "Heat", t: "Calm" }], read: "Your read: he'll count every rush from Saturday." },
        { k: "asks", label: "His asks", sub: "What he wants most that you haven't given", items: [{ text: "Play the right side, over their left tackle", pct: 0.48, sub: "of his snaps on the right now" }, { text: "Stay on the field on 3rd and long", pct: 0.61, sub: "of 3rd and 7+ he's on for now" }] },
        { k: "learned", label: "What you learned", items: [{ ini: "TW", text: "Tre Wallace has something going on at home", sub: "From Desmond, only part of it. In your notes on Tre.", act: { text: "Call Tre in", cmd: "people_open", args: { who: 51 } } }] },
      ],
      ideas: ["Ask what he sees from their left tackle", "Offer him more 3rd-down snaps", "Ask him to take the freshman Kobe Haynes under his wing", "Ask where his head is on April"],
    },
  },
  pp: { id: 44, mine: true, mood: { value: 0.35, words: "hopeful" }, reasons: [{ date: "Sat 10 Oct", why: "34 rushes and no clean look. The back on him or the slide his way.", up: false }, { date: "Since September", why: "we've barely run a twist for him.", up: false }, { date: "April", why: "on the fence about the draft.", up: false }], trust: { value: 0.6, words: "solid" }, ties: [], promises: [{ what: "starts every game", due: "Sat 17 Oct", kept: null }], ear: [{ who: "His brother Isaiah", what: "Very close", sure: "Sure" }], staffSays: "He'll count every rush from Saturday.", life: null, out: null, case: null, line: null, mentor: null },
  peek: { name: "Desmond Okoro", pos: "EDGE", jersey: 44, class: "Junior", ht: "6-4", wt: 252, where: "Sumter, South Carolina" },
};

const nokey = JSON.parse(JSON.stringify(desmond));
delete nokey.scene.say;
nokey.scene.lines = [desmond.scene.lines[0], desmond.scene.lines[1]];
nokey.scene.picks = [];
nokey.scene.lines[0].picked = "Ask · how he is";
nokey.scene.lines.push(line(1, "From Saturday you're in our twists. One rush in four, every game, through the bowl.", { picked: "Promise · in a stunt · 1 in 4 of his rushes · every game · through the bowl" }));
nokey.scene.talk = {
  channel: "In person", minutes: { left: 9, of: 25 }, week: 10, lede: "The same talk with live talks off. You pick; the game plays him.", left: desmond.scene.talk.left.slice(0, 1),
  after: { 2: [{ k: "logged", title: "In a stunt on at least 1 in 4 of his pass rushes, every game, through the bowl.", counts: "Counts from Sat 17 Oct", facts: [{ l: "Judged on", t: "The line chart's stunt flag" }, { l: "Plan setting", t: "Free him up: on" }, { l: "He believes it", t: "Mostly. He's counting." }] }] },
  moves: { tabs: [
    { k: "ask", name: "Ask", label: "Ask about", rows: [{ text: "How he is", tag: "Asked", cmd: "x" }, { text: "The locker room", tag: "Asked", cmd: "x" }, { text: "Their left tackle", tag: "Football", cmd: "x" }, { text: "April and the draft", tag: "His future", cmd: "x" }, { text: "His brother Isaiah", tag: "Family", cmd: "x" }] },
    { k: "tell", name: "Tell", rows: [{ text: "Where he stands", cmd: "x" }] },
    { k: "offer", name: "Offer or promise", builder: { title: "Build the promise", fields: [
      { k: "what", l: "What", opts: [{ v: "stunt", t: "In a stunt on his rushes" }, { v: "starts", t: "Starts every game" }] },
      { k: "much", l: "How much", half: true, opts: [{ v: "4", t: "1 in 4" }, { v: "3", t: "1 in 3" }] },
      { k: "when", l: "When", half: true, opts: [{ v: "every", t: "Every game" }, { v: "most", t: "Most games" }] },
      { k: "until", l: "Until", opts: [{ v: "bowl", t: "Through the bowl" }, { v: "season", t: "End of the season" }] }],
      say: "\"From Saturday you're in our twists. One rush in four, every game, through the bowl.\"", judged: "The stunt flag, game by game. Free him up goes on for every game.", go: { text: "Promise it", cmd: "x" } } },
    { k: "ask_to", name: "Ask him to", rows: [{ text: "Take the freshman under his wing", cmd: "x" }] }] },
};

const amari = {
  colors: GARNET,
  scene: {
    id: 12, kind: "HomeVisit", kindName: "Home visit", title: "Close to home, and a coach who stays", dateText: "Mon 30 Nov", place: "Their living room · Greenville", today: true, opened: true, done: false, future: false,
    who: [{ kind: "player", id: 90, link: "[[p:90|Amari Vance]]", pos: "CB" }, { kind: "cast", tok: "mom", link: "Loretta Vance", role: "Mother" }, { kind: "cast", tok: "hs", link: "Dale Pruitt", role: "His high school coach" }], hours: 6.5, outcome: [], docs: [], picks: [], subjects: [], next: null,
    say: { cmd: "people_say", args: { id: 12 }, limit: 400 },
    lines: [
      line(1, "Mrs. Vance, Columbia is a hundred miles from this porch. You can make every home game, and he's home for Sunday dinner whenever he wants.", { heard: "Reassured · Flat · No change" }),
      line(2, "Distance matters. So does who's coaching him two years from now. People are saying you might not be at South Carolina long.", { name: "Loretta Vance", tell: "hands folded, watching your face" }),
      line(1, "I'm signed through 2030 and my AD is behind me. I'm not going anywhere."),
      line(2, "Georgia says I start as a freshman.", { name: "Amari Vance", tell: "glances at his grandmother before he says it" }),
    ],
    talk: {
      channel: "Their living room", minutes: { left: 22, of: 45 }, week: 10,
      room: [{ lead: true, name: "Amari Vance", tag: "Interested", stars: "Four stars", line: "Tanner Creek HS · Greenville, SC · 6-0, 182. Weighs, as far as you know: playing early, the SEC, NIL" }, { name: "Loretta Vance", ini: "LV", tag: "Guarded", tagKind: "", line: "His grandmother. She has the final say. Weighs: close to home, a coach who stays, his grades", tone: "warm" }, { name: "Dale Pruitt", ini: "DP", tag: "Open", tagKind: "good", line: "His high school coach. Sent you two players. Weighs: being straight with the kid", tone: "cool" }],
      roomActs: [{ text: "Bring in your DB coach, on the phone", cmd: "x", args: {} }, { text: "Bring in a former player from Greenville", cmd: "x", args: {} }, { text: "Ask Coach Pruitt to step out", cmd: "x", args: {} }],
      after: { 2: [{ k: "logged", badge: "Logged · a promise to Amari and his grandmother", title: "You stay South Carolina's head coach through his senior year.", facts: [{ l: "Judged on", t: "Your job. A move or a firing breaks it." }, { l: "She believes it", t: "Not yet. She's heard it before." }] }], 3: [{ k: "read", text: "Your read: he's using it. Your staff heard Georgia told him he'd compete for a start, with both their starting corners leaving." }] },
      right: [
        { k: "rank", label: "What pulls him", note: "From your file · words only, no numbers", items: [{ name: "Georgia", sub: "Playing time", tag: "Leads" }, { name: "South Carolina", sub: "Home", tag: "Close behind", on: true }, { name: "Clemson", sub: "Fading" }], text: "His grandmother decides. Her pull is closeness and a coach who stays." },
        { k: "asks", label: "Their asks", sub: "What they want most that you haven't answered", items: [{ text: "Loretta: a plan for his grades", sub: "Her first want you haven't touched" }, { text: "Amari: hear it from the DB coach", sub: "Through Coach Pruitt, just now" }] },
        { k: "facts", label: "The rules right now", items: ["Contact period: home visits, calls and texts allowed.", "This is your one head coach home visit. A dead period starts the day before signing."] },
        { k: "ideas", label: "Ideas", note: "A click puts the words in your box to change", items: ["Tell Mrs. Vance about the academic staff and a tutor", "Ask Coach Pruitt what Amari needs most", "Show the film of your corners in press"] },
      ],
    },
  },
  pp: null, peek: { name: "Amari Vance", pos: "CB", jersey: 3, class: "Class of 2027", ht: "", wt: "", where: "" },
};

const keon = {
  colors: BLUE,
  scene: {
    id: 13, kind: "Gm", kindName: "Phone call · Mark Ruiz, his agent", title: "Re-signing Keon Sterling", dateText: "Fri 5 Mar", place: "", today: true, opened: true, done: false, future: false,
    who: [{ kind: "player", id: 11, link: "[[p:11|Keon Sterling]]", pos: "WR" }], hours: 6.5, outcome: [], docs: [], picks: [], subjects: [], next: null,
    say: { cmd: "people_say", args: { id: 13 }, limit: 400 },
    lines: [
      line(2, "Keon loves Charlotte, I'll say that. But the Jets are at nineteen a year, and they'll be on the phone at noon Monday.", { name: "Mark Ruiz", tell: "quick, like a line he has used before" }),
      line(1, "Nineteen with what guaranteed? Their cap is tight after that quarterback deal.", { heard: "you doubt the offer · he's annoyed you checked; the number comes down" }),
      line(2, "Call it real interest at eighteen. He wants to stay, but he wants top-ten money and real guarantees.", { name: "Mark Ruiz", tell: "slower now" }),
      line(1, "Four years, seventy million, thirty-six guaranteed at signing, twenty of it up front. A million a year more if he catches 90. And he stays our X.", { heard: "an offer, and a promise about his role · the offer goes on the table to confirm" }),
    ],
    talk: {
      channel: "Phone call", minutes: { left: 3, of: 15 }, week: 10, lede: "Only you can talk to him until the window opens.",
      after: { 2: [{ k: "read", text: "Your read: the Jets number was air. Your cap people have the Jets at about $11M of room." }], 3: [{ k: "logged", badge: "Logged · your word, not in the contract", title: "He stays your X: lined up at X on most of his snaps, every game." }] },
      right: [{
        k: "deal", title: "The deal on the table", note: "Built from your words", cols: ["Your offer", "Ruiz's counter"],
        rows: [{ l: "Years", a: "4", b: "4" }, { l: "New money", a: "$70.0M", b: "$74.0M" }, { l: "A year", a: "$17.5M", b: "$18.5M" }, { l: "Guaranteed at signing", a: "$36.0M", b: "$42.0M", hot: true }, { l: "Signing bonus", a: "$20.0M", b: "$22.0M" }, { l: "Incentive: 90 catches", a: "$1.0M a year", b: "none" }, { l: "Year 4, guaranteed for injury", a: "none", b: "$8.0M" }],
        text: "The game builds his counter from what Keon wants most that you haven't given: guarantees. Ruiz opened at $20M a year on 24 Feb.",
        chart: { type: "stack", label: "Your offer against the cap", legend: ["Bonus, spread", "Salary"], bars: [{ l: "2027", a: 9.8 * 0.4, b: 9.8 * 0.6, top: "$9.8M" }, { l: "2028", a: 7.5, b: 11.1, top: "$18.6M" }, { l: "2029", a: 7.5, b: 12.6, top: "$20.1M" }, { l: "2030", a: 7.5, b: 14, top: "$21.5M" }], note: "Cap room this year: $24.6M. The 90-catch incentive stays off this year's cap: he caught 81, so it's not likely to be earned." },
        send: { text: "This sends a real offer to Mark Ruiz: **4 years, $70M, $36M guaranteed at signing.** He takes it to Keon, and you hear back by Sunday.", change: "Change the guarantee to $40M", cmd: "x", args: {} },
      }],
    },
  },
  pp: { id: 11, mine: true, mood: { value: 0.1, words: "level" }, reasons: [], trust: { value: 0.5, words: "steady" }, ties: [], promises: [], ear: [], staffSays: "Money and guarantees first, then a contender.", life: null, out: null, case: null, line: null, mentor: null },
  peek: { name: "Keon Sterling", pos: "WR", jersey: 11, class: "Age 27", ht: "6-1", wt: 198, where: "five seasons with you" },
};


const film = {
  colors: GARNET,
  scene: {
    id: 14, kind: "OneOnOne", kindName: "Film room", title: "Their left tackle sets short", dateText: "Wed 14 Oct", place: "Texas A&M week", today: true, opened: true, done: false, future: false,
    who: [{ kind: "player", id: 44, link: "[[p:44|Desmond Okoro]]", pos: "EDGE" }, { kind: "coach", id: 7, link: "Ron Kessler", role: "Defensive coordinator" }], hours: 5.9, outcome: [], docs: [], picks: [], subjects: [],
    say: { cmd: "people_say", args: { id: 14 }, limit: 400 },
    lines: [
      line(2, "Their left tackle sets short. Every time. Get me on an island with him and I win outside.", { tell: null }),
      line(1, "We've been running twists for you all year.", { heard: "a claim about your scheme · he knows it isn't so" }),
      line(1, "Against their quarterback, keep contain on 3rd and short. Don't rush past him.", { heard: "coaching for Saturday · he gets it; it costs him a few wins" }),
    ],
    talk: { persona: true,
      channel: "Film room", minutes: { left: 18, of: 30 }, week: 10, compact: true, lede: "Desmond with you, Ron Kessler sitting in. What's said is checked against the tape.",
      left: [
        { k: "tiles", label: "His rushes, last 3 games", note: "from the play log", items: [{ n: 94, l: "pass rushes" }, { n: 14, l: "rush wins" }, { n: "2.6s", l: "to win, on average" }] },
        { k: "bars", label: "How he wins", items: [{ l: "Speed rush", pct: 0.8, n: 8 }, { l: "Bull rush", pct: 0.4, n: 4 }, { l: "Spin move", pct: 0.2, n: 2 }] },
        { k: "split", label: "Where he lines up", pct: 0.52, a: "Left end", b: "Right end" },
        { k: "text", label: "Your staff on Texas A&M", paras: ["**Ron Kessler, DC, asked what beats their protection**", "\"They slide to the tight end on 64% of their passes. Twist away from the slide with Desmond looping inside. On our tape that's won about 1 rush in 3.\""], small: "From their last 4 games and our call values. Ron reads tape well." },
        { k: "clips", label: "On tape: their left tackle", note: "#71 Marcus Bell, Texas A&M · his last 2 games", items: [{ title: "vs Florida · 3rd and 7", sub: "Beaten outside by speed · sack in 2.9 s" }, { title: "vs Arkansas · 3rd and 9", sub: "Beaten outside by speed · pressure in 2.4 s" }, { title: "vs Arkansas · 2nd and 6", sub: "Held up a bull rush · clean pocket" }] },
      ],
      after: { 0: [{ k: "check", ok: true, text: "On their tape he gave up 4 pressures to speed rushes in 2 games. Desmond watched the same film." }], 1: [{ k: "check", ok: false, text: "The play log has him in a stunt on 9% of his rushes. He's in them, so he knows. His trust in you dips a little." }], 2: [{ k: "built", title: "Desmond keeps contain on 3rd and 4 or less against Texas A&M.", facts: [{ l: "In the game", t: "Keep contain, the built instruction, on his card" }, { l: "He'll do it", t: "Most of the time. He likes to rush." }] }] },
      to: { label: "Say something", list: ["Both", "Desmond", "Ron Kessler"] },
      right: [{
        k: "promises", label: "What you can promise", sub: "Every position has its own list, judged on the play log.", pick: "EDGE",
        tabs: ["QB", "RB", "3rd-down RB", "X and Z", "Slot", "TE", "Center", "Guards", "Tackles", "EDGE", "DT", "Off-ball LB", "Outside CB", "Nickel", "Safeties", "Kicker", "Punter", "Snapper", "Returners", "Coverage units"],
        cards: { EDGE: [
          { title: "Stunts to free him up", tag: "Promised Tue", facts: [{ l: "Judged on", t: "Stunts he's in, 1 in 4 rushes or more" }, { l: "Plan setting", t: "Free him up" }, { l: "Kept", t: "Your games" }] },
          { title: "Rush, not drop", facts: [{ l: "Judged on", t: "Pass plays he rushed, not dropped" }, { l: "Plan setting", t: "Rush him every down" }, { l: "Kept", t: "Your games" }] },
          { title: "His side, or both sides", facts: [{ l: "Judged on", t: "His spot: left or right end" }, { l: "Plan setting", t: "Keep him on one side" }, { l: "Kept", t: "Every snap" }] },
          { title: "The 3rd-down rusher", facts: [{ l: "Judged on", t: "His snaps by down" }, { l: "Plan setting", t: "Pass-rush package" }, { l: "Kept", t: "Every snap" }] }] },
        every: "Starts and snap share · snaps by situation and package · his spot · ahead of a named teammate · not pulled after a mistake · special teams.",
        talkonly: { badge: "Talk only", text: "**Chips, doubles, sacks.** Their calls and outcomes. He can care about them, but your word can't be about them." },
      }],
    },
  },
  pp: desmond.pp, peek: desmond.peek,
};

const inbox = {
  colors: GARNET,
  scene: null,
  asking: {
    items: [
      { id: 1, ini: "TW", name: "Tre Wallace", sub: "S · sophomore · at your office door", text: "Wants a word today. It's about home.", when: "7:40 AM", tag: "Now", urgent: true },
      { id: 2, ini: "IO", name: "Isaiah Okoro", sub: "DE · class of 2028 · a text", text: "Desmond says y'all are running twists for him now. That true?", when: "8:31 AM", tag: "Text" },
      { id: 3, ini: "RK", name: "Ron Kessler", sub: "Defensive coordinator", text: "Twenty minutes on Saturday's twists for Desmond, before he scripts them.", when: "Yesterday", tag: "By Fri" },
      { id: 4, ini: "RH", name: "Ray Hollins", sub: "Athletic director", text: "The collective's budget, before the A&M game.", when: "Mon", tag: "This week" },
      { id: 5, ini: "KH", name: "Kobe Haynes", sub: "DE · freshman", text: "Asked about his playing time on Monday. You haven't answered, and he's taken it as a no.", when: "Mon", tag: "Waited 3 days" },
    ],
    open: { id: 2, ini: "IO", name: "Isaiah Okoro", sub: "DE · class of 2028 · Sumter HS · Desmond's brother", quiet: { tag: "Quiet period", text: "Calls and texts only. No visits off campus." },
      days: [
        { day: "Tue 13 Oct", lines: [{ by: 2, text: "Desmond says y'all are running twists for him now. That true?", time: "9:12 PM" }, { kind: "grapevine", text: "Your promise to Desmond reached his brother the same night. Isaiah will judge it by what he sees on Saturday, and so will his coach." }] },
        { day: "Wed 14 Oct", lines: [{ by: 1, text: "It's true. One rush in four, every game. Watch Saturday.", time: "8:05 AM", heard: "a promise he can check on TV · he's watching A&M now" }, { by: 2, text: "Bet. My coach says y'all mostly play a four-man front. I'm more of a stand-up guy.", time: "8:31 AM", note: "Your read: he knows his game and he's asking if he fits. Your fronts this year: four linemen down on 71% of snaps; on the rest your ends stand up." }] },
      ],
      reply: { label: "Text Isaiah", ph: "Write it the way you'd text a recruit you want.", note: "He answers when he would: school's out until 3:30.", send: { cmd: "x", args: {} }, call: { cmd: "x", args: {} } } },
    stops: { tag: "Stops Continue", kind: "A family matter", name: "Tre Wallace", ini: "TW", sub: "S · sophomore · #2 · Rock Hill, SC", text: "He's outside your office. \"Coach, you got a minute? It's about home.\"",
      know: [{ text: "Something's going on at home. From Desmond on Tuesday, only part of it." }, { text: "Late to two meetings this week. His position coach, Wednesday." }],
      options: [{ text: "See him now", primary: true, cmd: "x" }, { text: "After practice, today", cmd: "x" }, { text: "Send his position coach", cmd: "x" }],
      warn: "If it waits past today, he takes it as you not caring, and so does Desmond, who sent him.",
      why: "A family crisis, a trade request, an agent's deadline, your boss asking for a meeting. Everything else waits in this list." },
  },
};

window.CV_FIXTURES = { desmond, nokey, amari, keon, film, inbox };

