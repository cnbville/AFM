// What's new (Cain, 2026-11-13): the changelog on the opening screen, written
// for players. A few short lines per version, in plain words: what you will
// notice, with no PR numbers, code names or build talk. Newest first. A
// version that changed nothing you can see is left out.
//
// Add a panel here with every version that ships (the patch notes thread's job,
// from afm/patch-notes.md); the page shows only what is in this list.
//
// `v` is the app number (0.2.<build>), `date` the day it came out, `title` a
// few words on the version, `notes` three or four short lines. The patch notes
// (afm/patch-notes.md) keep the full story for the team.
"use strict";

const CHANGELOG = [
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
