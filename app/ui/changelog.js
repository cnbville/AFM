// What's new (Cain, 2026-11-13): the changelog on the opening screen, written
// for players. A few short lines per version, in plain words: what you will
// notice, with no PR numbers, code names or build talk. Newest first. A
// version that changed nothing you can see is left out.
//
// `v` is the app number (0.2.<build>), `date` the day it came out, `title` a
// few words on the version, `notes` three or four short lines. The patch notes
// (afm/patch-notes.md) keep the full story for the team.
"use strict";

const CHANGELOG = [
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
