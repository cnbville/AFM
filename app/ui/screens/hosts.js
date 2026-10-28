// One sidebar entry per job (post-release 3, SC1 and SC2): screens that
// draw their sibling pages as tabs. Loaded last, over every lane's screens.
"use strict";
/* Several jobs had two or three sidebar entries: Cap and Cap sheet, News
 * and Media, Standings and the race, five pages about you. A host keeps one
 * entry and draws the others as tabs over the page. Nothing is cut: every
 * page draws as it did, with its own head, tabs and "Go to" line.
 *
 *   hostTabs(key, parts)  screens[key] becomes a host. parts, in tab order:
 *                         {t: the tab's name (or a function, for a name by
 *                         seat), go: the page's screen, seats, show(args)};
 *                         one of them is key itself, the host's own page. A
 *                         part a seat doesn't have shows no tab, and a host
 *                         left with one page draws it bare.
 *   Old keys stay         go("media", args) opens News on its Media tab: the
 *                         route becomes the host's, so the sidebar lights its
 *                         entry and back returns to the tab. The page's own
 *                         sidebar line goes wherever its host holds it.
 *   args.on               the tab shown, by its page's screen key, kept in
 *                         the route; a page's own tabs keep args.tab. With no
 *                         args, a sidebar click or a redraw keeps the last
 *                         tab; a link with args of its own opens the host's
 *                         own page.
 */

const HOSTS = {};
// Each page's own screen function, from before any host took its key.
const HOST_PAGE = {};

function hostTabs(key, parts) {
  for (const p of parts) HOST_PAGE[p.go] ??= screens[p.go];
  HOSTS[key] = parts;
  screens[key] = (a) => hostDraw(key, a || {});
  for (const p of parts) if (p.go !== key) screens[p.go] = (a) => hostAlias(p.go, a || {});
  for (const n of NAV) {
    if (n.go === key || !parts.some((p) => p.go === n.go)) continue;
    const was = n.show;
    n.show = () => !hostOf(n.go) && (!was || was());
  }
}
// The parts this seat and these args have.
function hostParts(key, a) {
  const seat = seatKind();
  return HOSTS[key].filter((p) => (!p.seats || !seat || p.seats.includes(seat)) && (!p.show || p.show(a)));
}
// The host that draws a page as one of its tabs, or null.
function hostOf(go, a = {}) {
  for (const k in HOSTS) {
    const parts = hostParts(k, a);
    if (parts.length > 1 && parts.some((p) => p.go === go)) return k;
  }
  return null;
}
const hostLabel = (p) => (typeof p.t === "function" ? p.t() : p.t);

async function hostDraw(key, a) {
  const parts = hostParts(key, a);
  if (parts.length < 2) return HOST_PAGE[key](a);
  const own = parts.find((p) => p.go === key) || parts[0];
  const k = keep("host:" + key, { on: parts[0].go, route: null });
  const bare = Object.keys(a).every((x) => x === "on" || a[x] == null);
  let p = parts.find((q) => q.go === a.on);
  if (!p && bare && !S.besideDrawing && (S.route.root || S.route === k.route)) p = parts.find((q) => q.go === k.on);
  if (!p) p = own;
  if (!S.besideDrawing) { k.on = p.go; k.route = S.route; }
  const t0 = S.title;
  const html = await HOST_PAGE[p.go](a);
  // A page that names itself keeps its name; one that doesn't takes the host's.
  if (S.title === t0 && p !== own) title(CRUMBS[key] || hostLabel(own), hostLabel(p));
  const bar = parts.map((q) => `<button role="tab" data-act="hosttab" data-host="${key}" data-on="${q.go}" aria-selected="${q === p}">${esc(hostLabel(q))}</button>`).join("");
  return `<div class="pad host-bar"><div class="tabsline host-tabs" role="tablist">${bar}</div></div>${html}`;
}
function hostAlias(go, a) {
  const key = hostOf(go, a);
  return key ? hostMove(go, key, { ...a, on: go }) : HOST_PAGE[go](a);
}
// The route moves to the screen that draws the page now, unless the page
// draws beside or inside another screen.
function hostMove(from, to, a) {
  if (!S.besideDrawing && S.route.screen === from) { S.route.screen = to; S.route.args = a; }
  return screens[to](a);
}
CORE_MORE.hosttab = (el) => {
  const d = el.dataset;
  keep("host:" + d.host, {}).on = d.on;
  if (el.closest(".beside")) {
    if (S.beside) { S.beside = { screen: d.host, args: { on: d.on } }; drawBeside(true); }
    return;
  }
  S.route.screen = d.host;
  S.route.args = { on: d.on };
  render();
};

// ---------- SC1: one entry where two did one job ----------
const hostMine = () => !!(S.status && S.status.mine);
const HOST_SEATS = ["college", "nfl", "hs"];
// An assistant has no desk: he keeps the plain page and its own line.
hostTabs("fo_cap", [
  { t: "Cap board", go: "capboard", seats: ["nfl"] },
  { t: "Cap sheet", go: "fo_cap", seats: ["nfl"], show: foIsHead },
]);
hostTabs("nil_desk", [
  { t: "NIL", go: "nil_desk", seats: ["college"], show: foIsHead },
  { t: "Budget map", go: "capboard", seats: ["college"] },
]);
hostTabs("roster", [
  { t: "Squad", go: "roster" },
  { t: "Varsity and JV", go: "squads", seats: ["hs"], show: (a) => foIsHead() && trMine(a) },
]);
hostTabs("area", [
  { t: "School and feeders", go: "area", seats: ["hs"], show: foIsHead },
  { t: "Former players", go: "draftoutlook", seats: ["hs"] },
]);
// High school Recruiting is Colleges calling: the same players, their
// offers and where they stand, with every college that calls.
{
  const page = screens.recruiting;
  screens.recruiting = (a) => (seatKind() === "hs" ? hostMove("recruiting", "hsside", a || {}) : page(a));
}

// ---------- SC2: related pages under one entry ----------
hostTabs("news", [{ t: "News", go: "news" }, { t: "Media", go: "media" }]);
hostTabs("standings", [
  { t: "Standings", go: "standings", seats: HOST_SEATS },
  { t: () => ({ nfl: "Playoff picture", hs: "State rankings" })[seatKind()] || "Polls", go: "polls", seats: HOST_SEATS, show: hostMine },
  { t: "Other leagues", go: "leagues" },
]);
hostTabs("history", [
  { t: "History", go: "history" },
  // Another team's page stays its own page.
  { t: "Team history", go: "team", show: (a) => hostMine() && !a.kind },
]);
hostTabs("you", [
  { t: "Profile", go: "you" },
  { t: "Career story", go: "story" },
  { t: "Season chapters", go: "chapters" },
  { t: "Your legacy", go: "legacy" },
  { t: "Office", go: "office" },
]);
hostTabs("people", [
  { t: "People", go: "people" },
  { t: "Locker room", go: "locker", seats: HOST_SEATS, show: hostMine },
  { t: "Traditions", go: "traditions", seats: HOST_SEATS, show: hostMine },
]);
// A club in another league: its league's page is its Standings already.
navAdd({ go: "leagues", seats: HOST_SEATS });
{
  const alias = screens.leagues;
  screens.leagues = (a) => {
    if (seatKind() !== "other") return alias(a);
    if (!S.besideDrawing && S.route.screen === "leagues") S.route.screen = "leagues_mine";
    return HOST_PAGE.leagues(a || {});
  };
}
