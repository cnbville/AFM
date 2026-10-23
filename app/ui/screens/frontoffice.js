// Front office screens (lane 10): the sidebar's front office entries for
// your seat, and helpers the desks share. Each desk registers its own
// screens in fo-nfl.js, fo-college.js and fo-school.js.
"use strict";

// The desks get their own section in the sidebar (NAV_SECS in app.js),
// after "Your team", named for the seat.
const FO_SEC = { nfl: "Front office", college: "Recruiting office", hs: "Your school" };
NAV_SECS.splice(NAV_SECS.findIndex((x) => x[0] === "team") + 1, 0, ["fo", { toString: () => FO_SEC[seatKind()] || "Front office" }]);

// Out of work, a career has no team pages: only its market and the world.
for (const n of NAV) {
  if (n.sec !== "team" && n.go !== "team") continue;
  const before = n.show;
  n.show = () => !!(S.status && S.status.mine) && (!before || before());
}

// Each desk adds its entries with foNavAdd(kind, entries), entries being
// [screen, label, icon]; "all" is every career's (the job market, under
// World). A desk is the head coach's: an assistant sees only "all". A
// desk's entry with the same screen or label as a plain one takes its
// place for that seat (the plain screen stays reachable by its links).
function foIsHead() {
  const seat = S.status && S.status.seat;
  return !!(S.status && S.status.mine) && (!seat || seat.head !== false);
}
function foNavAdd(kind, entries) {
  for (const [go, label, icon] of entries) {
    if (kind === "all") {
      const at = NAV.findIndex((n) => n.go === "you");
      NAV.splice(at < 0 ? NAV.length : at, 0, { go, label, icon, sec: "world", fo: true, show: () => !!(S.status && S.status.you) });
      continue;
    }
    const same = NAV.find((o) => !o.fo && (o.go === go || o.label === label));
    if (same) foNavHide(same.go, kind);
    NAV.push({ go, label, icon, sec: "fo", seats: [kind], fo: true, show: foIsHead });
  }
}
// Takes a plain entry out of one seat's sidebar: the NFL desk's cap sheet
// holds every contract by year, so it stands in for Contracts.
function foNavHide(go, kind) {
  const n = NAV.find((o) => o.go === go && !o.fo);
  if (n) n.seats = (n.seats || ["nfl", "college", "hs"]).filter((k) => k !== kind);
}

// The desks' sidebar icons (24-grid strokes, like the rest in app.js).
Object.assign(I, {
  office: '<rect x="3" y="7" width="18" height="13" rx="2"/><path d="M8 7V5a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M3 13h18"/>',
  cap: '<path d="M12 3a9 9 0 1 0 9 9h-9z"/><path d="M15 3.4A9 9 0 0 1 20.6 9H15z"/>',
  tag: '<path d="M3 12V4a1 1 0 0 1 1-1h8l9 9-9 9z"/><circle cx="7.5" cy="7.5" r="1.5"/>',
  fa: '<path d="M4 20h4L19 9l-4-4L4 16z"/><path d="M13.5 6.5l4 4M14 20h6"/>',
  trade: '<path d="M4 8h14l-3.5-3.5M20 16H6l3.5 3.5"/>',
  cut: '<circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M8.2 8.2L20 20M8.2 15.8L20 4"/>',
  board: '<rect x="5" y="4" width="14" height="17" rx="2"/><path d="M9 4V3h6v1M9 10h6M9 14h6M9 18h4"/>',
  find: '<circle cx="10.5" cy="10.5" r="6.5"/><path d="M20 20l-4.8-4.8M8 10.5h5M10.5 8v5"/>',
  visit: '<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 10h18M8 3v4M16 3v4M9 15.5l2 2 4-4"/>',
  portal: '<path d="M14 3h5a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1h-5"/><path d="M3 12h12M11 8l4 4-4 4"/>',
  nil: '<circle cx="12" cy="12" r="9"/><path d="M15 9.5c-.5-1-1.6-1.5-3-1.5-1.7 0-3 .8-3 2s1.3 1.7 3 2 3 .8 3 2-1.3 2-3 2c-1.4 0-2.5-.5-3-1.5M12 6v2M12 16v2"/>',
  school: '<path d="M3 21h18M5 21V10l7-4 7 4v11M9 21v-5h6v5M12 6V2l4 1.5L12 5"/>',
  map: '<path d="M9 4L3 6v14l6-2 6 2 6-2V4l-6 2z"/><path d="M9 4v14M15 6v14"/>',
  market: '<path d="M12 3v18M5 6h11l3 2.5L16 11H5zM19 13H8l-3 2.5L8 18h11z"/>',
});

// Your family at home and the people in football who know you (the
// profile's Profile tab).
function youLife(y) {
  const f = y.family;
  if (!f) return "";
  const known = (y.known || []).map((k) => `<div class="item"><span class="when">${esc(k.close)}</span><span>${coachLink(k.id, k.name)} <span class="sub">· ${esc(k.now)}</span><br><small class="sub">${esc(k.how)}, ${esc(k.years)}</small></span></div>`).join("");
  const log = f.log.map((l) => `<div class="item"><span class="when">${esc(l.date)}</span><span>${rich(l.text)}</span></div>`).join("");
  const home = f.home ? `<dt>Home</dt><dd>${esc(f.home)}${f.since ? ` since ${f.since}` : ""}${f.moves ? ` · moved ${f.moves} time${f.moves === 1 ? "" : "s"} for football` : ""}</dd>` : "";
  const kids = f.children.length ? `<dt>Children</dt><dd>${f.children.map((c) => `${esc(c.name)}, ${c.age} <span class="sub">(${esc(c.school.toLowerCase())})</span>`).join("; ")}</dd>` : "";
  const fam = `<div class="panel"><h2>At home <small>${esc(f.mood)}</small></h2><dl class="kv">${home}${kids}</dl>${log ? `<div class="items">${log}</div>` : ""}</div>`;
  return `<div class="grid2">${fam}<div class="panel"><h2>People who know you <small>a search calls them about you</small></h2><div class="items">${known || '<div class="empty">Nobody in the world\'s history yet: they come with every staff you join.</div>'}</div></div></div>`;
}
