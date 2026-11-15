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
