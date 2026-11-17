/* Capital's shared frame (lane CS1; docs/design/capital.md CP37, CP39): the
 * sidebar section the six Capital screens sit in and the tab row along the
 * top of each of them. A screen adds itself to the sidebar with navAdd
 * ({ sec: "capital" }) and to the tab row by listing its name in CAP_TABS
 * order; a tab shows only once its screen exists, so the lanes merge in any
 * order. Screens: capitalcampus, capitaldonors, capitaldrive, capitalproject,
 * capitalpecking, capitalseat. Every class is cap- (the shared frame) so the
 * app's global names never restyle it. No dollar figures anywhere (CP39). */
"use strict";

(() => {
  if (!NAV_SECS.some((s) => s[0] === "capital")) {
    const at = NAV_SECS.findIndex((s) => s[0] === "ground");
    NAV_SECS.splice(at >= 0 ? at + 1 : NAV_SECS.length, 0, ["capital", "Capital"]);
  }
})();

const CAP_TABS = [
  ["capitalcampus", "Campus"],
  ["capitaldonors", "Donors"],
  ["capitaldrive", "The drive"],
  ["capitalproject", "Project"],
  ["capitalpecking", "Pecking order"],
  ["capitalseat", "Your seat"],
];

// The tab row, with the screen you're on lit. A tab whose screen isn't built
// (or isn't on this seat) is left out.
function capTabs(active) {
  const tabs = CAP_TABS.filter(([go]) => screens[go] || go === active);
  if (tabs.length < 2) return "";
  return `<nav class="cap-tabs" aria-label="Capital screens">${tabs.map(([go, label]) => `<button type="button" class="cap-tab${go === active ? " on" : ""}" data-go="${go}"${go === active ? ' aria-current="page"' : ""}>${esc(label)}</button>`).join("")}</nav>`;
}

// Pips for a level out of 5: ivory for what stands, gold for what a building
// or a renovation adds.
function capPips(level, target) {
  const t = Math.max(level, target || 0);
  return `<span class="cap-pips" role="img" aria-label="${level} of 5${t > level ? `, ${t} when it opens` : ""}">${Array.from({ length: 5 }, (_, i) => `<i class="${i < level ? "on" : i < t ? "up" : ""}"></i>`).join("")}</span>`;
}

const capCap = (s) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : s);
const capDate = (iso, opt) => (iso ? new Date(iso + "T12:00:00").toLocaleDateString("en-US", opt) : "");
