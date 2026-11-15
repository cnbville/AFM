// "What pulls him": the tracker on a recruit's race page and on the recruit's
// and the transfer's files (talk facts design, section 7; lane L13). The
// game sends `pulls` (lane_frontoffice/pulls.rs, `section`): the seven
// groups in the order his file reads them, each with a word, whether it is
// firm, whether his words and your read part, and the newest evidence with
// where and when it came from. Nothing here is a number of his: the panel
// shows what you have seen, and fills as you talk to him.
"use strict";

// How each word reads on the dot beside it.
function plTone(word, conflict) {
  if (conflict) return "conf";
  if (word === "Matters most" || word === "Matters a lot") return "hi";
  if (word === "Seems to matter") return "mid";
  if (word === "Not his thing") return "lo";
  return "none";
}

// One line of evidence: what he did or said, then where and when.
function plEvidence(r) {
  return `<span class="pl-e${r.guess ? " guess" : ""}"><span class="pl-t">${esc(r.text)}</span><span class="pl-f">${esc(r.from)}</span></span>`;
}

// Your school's side of a group: your best fact for it, and how it stands
// next to his other top schools. It is what answers "why isn't he drawn?".
function plYours(y) {
  if (!y) return "";
  return `<span class="pl-y"><span class="pl-yh"><small>Your school</small>${y.tier_word ? `<b class="pl-tier ${esc(y.tier)}" title="How your fact stands next to his other top schools, as your staff reads it">${esc(y.tier_word)}</b>` : ""}</span><span class="pl-t">${esc(y.text)}</span>${y.also ? `<span class="pl-t pl-also">${esc(y.also)}</span>` : ""}</span>`;
}

// The panel. `x` is the file's `pulls`; `o.wrap` puts it in the race page's
// tokens when it sits on a file page, which has none of its own.
function plPanel(x, o = {}) {
  if (!x) return "";
  const forWhom = x.for && x.for !== "him" ? `<span class="fr-k pl-for">what ${esc(x.for)} want</span>` : "";
  const head = `<div class="fr-ph"><span><b class="frr-h3">What pulls him</b> <span class="fr-mute">what you have seen from him, not his numbers</span></span>${forWhom}</div>`;
  const flags = (x.flags || []).map((f) => `<div class="pl-flag">${esc(f)}</div>`).join("");
  let body;
  const anyYours = x.groups.some((g) => g.yours);
  const intro = `<p class="pl-intro fr-mute">Nothing yet from him. What he brings up, how he takes each point and what he tells you when you ask all land here.</p>`;
  if (x.empty && !anyYours) {
    body = `<div class="pl-none"><p class="fr-mute">Nothing yet. What he brings up, how he takes each point and what he tells you when you ask all land here.</p>
      <div class="pl-names">${x.groups.map((g) => `<span class="frr-tag">${esc(g.name)}</span>`).join("")}</div></div>`;
  } else {
    body = (x.empty ? intro : "") + x.groups.map((g) => {
      const tone = plTone(g.word, g.conflict);
      const first = g.rows.slice(0, 2).map(plEvidence).join("");
      const more = g.rows.length > 2
        ? `<details class="pl-more"><summary>${g.rows.length - 2} earlier</summary>${g.rows.slice(2).map(plEvidence).join("")}</details>`
        : "";
      return `<div class="pl-row ${tone}${g.rows.length ? "" : " empty"}">
        <span class="pl-name">${esc(g.name)}</span>
        <span class="pl-word"><i class="pl-dot"></i>${esc(g.word)}${g.firm ? '<small class="pl-firm">firm</small>' : ""}</span>
        <span class="pl-ev">${first}${more}${plYours(g.yours)}</span></div>`;
    }).join("");
  }
  const panel = `<div class="fr-panel pl">${head}${flags}${body}</div>`;
  return o.wrap ? `<div class="fr pl-wrap">${panel}</div>` : panel;
}
