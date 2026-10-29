// Lane screens: help on every screen (update 3, lane 6).
"use strict";
/* H1, H3 and H7, in Press Box, on the rating card's look (ratings.css):
 *
 *   H1  a rating's name explains itself (help_attr): any element whose
 *       whole text is a rating's label ("Arm strength") gets data-h1.
 *   H3  a level badge ("FBS level 73") explains itself
 *       (help_level): the screens mark it with data-h3="<player id>".
 *   H7  football terms in any text explain themselves (the glossary below):
 *       the first time a term shows in a page it gets a dotted line.
 *
 * H2 lives on the range's own card (ratings.js), H4 on the depth chart
 * (squadboards.js), H6 in letters and the news (core.js). The cards follow
 * the hover setting and its delay; the keyboard reaches them too.
 */

// ---------- H7: the glossary ----------
// [term as shown, the words that find it, what it means]. Real football,
// plain words; the level that uses it most is named where it matters.
const H7_TERMS = [
  ["Nickel", ["nickel"], "A fifth defensive back in place of a linebacker, against three receivers. The nickel back usually covers the slot."],
  ["Dime", ["dime"], "A sixth defensive back, for obvious passing downs: two linebackers or fewer stay on the field."],
  ["Cover 0", ["cover 0", "cover zero"], "Man coverage with no deep safety: everyone else blitzes. A miss is a touchdown."],
  ["Cover 1", ["cover 1", "cover one"], "Man coverage with one deep safety in the middle, often with a robber underneath."],
  ["Cover 2", ["cover 2", "cover two"], "Two deep safeties split the field in halves; the corners sit in the flats."],
  ["Cover 3", ["cover 3", "cover three"], "Three deep defenders split the field in thirds (two corners and a safety), four underneath."],
  ["Cover 4", ["cover 4", "cover four", "quarters"], "Four deep defenders split the field in quarters; the safeties read the slot receivers and fit the run."],
  ["Cover 6", ["cover 6", "cover six"], "Quarters to one side and Cover 2 to the other, set by the formation."],
  ["Man coverage", ["man coverage", "man-to-man"], "Each defender covers one receiver wherever he goes."],
  ["Zone coverage", ["zone coverage"], "Each defender covers an area and the receivers in it, eyes on the quarterback."],
  ["Blitz", ["blitz", "blitzes", "blitzed"], "More than four rushers, or a rusher from off the line: a linebacker or a defensive back."],
  ["Stunt", ["stunt", "stunts", "twist", "twists"], "Two linemen trade gaps after the snap to confuse the blockers."],
  ["RPO", ["rpo", "rpos", "run-pass option"], "Run-pass option: the quarterback reads one defender after the snap and hands off or throws."],
  ["Play-action", ["play-action", "play action"], "A fake handoff before the throw, to pull linebackers and safeties up."],
  ["Read option", ["read option", "zone read"], "The quarterback reads an unblocked end: hand off if he stays wide, keep it if he crashes."],
  ["Shotgun", ["shotgun"], "The quarterback lines up about five yards behind the center and takes a long snap."],
  ["Pistol", ["pistol"], "A short shotgun, about four yards back, with the running back behind the quarterback."],
  ["Personnel", ["11 personnel", "12 personnel", "21 personnel", "10 personnel", "13 personnel", "22 personnel"], "The offense's grouping: the first digit is the running backs, the second the tight ends. 11 personnel is one back, one tight end and three receivers."],
  ["A-gap", ["a-gap", "a gap"], "The gap on either side of the center; B is outside the guard, C outside the tackle."],
  ["3-technique", ["3-technique", "three-technique"], "A defensive tackle on the outside shoulder of a guard: the penetrating tackle in a 4-3."],
  ["4-3", ["4-3"], "Four down linemen and three linebackers."],
  ["3-4", ["3-4"], "Three down linemen and four linebackers, the two outside ones usually rushing."],
  ["Red zone", ["red zone"], "Inside the opponent's 20-yard line."],
  ["Two-minute drill", ["two-minute drill", "two-minute offense"], "The hurry-up offense at the end of a half, working the clock and the sideline."],
  ["Onside kick", ["onside kick", "onside kicks", "onside"], "A short kickoff the kicking team tries to recover after it goes 10 yards."],
  ["Touchback", ["touchback", "touchbacks"], "A kick downed in or through the end zone: the ball comes out to a fixed yard line."],
  ["Gunner", ["gunner", "gunners"], "The outside man on punt coverage, first down the field to the returner."],
  ["Long snapper", ["long snapper", "long snappers"], "The specialist who snaps on punts, field goals and extra points."],
  ["Pick six", ["pick six", "pick-six"], "An interception returned for a touchdown."],
  ["Depth chart", ["depth chart"], "The order of players at each position: the starter, then the next men in."],
  ["Two-way player", ["two-way", "two-way player", "two-way players"], "A player on both offense and defense, common in high school where rosters are small."],
  ["Redshirt", ["redshirt", "redshirted", "redshirting"], "A college season without using a year of eligibility: he practises, and since 2018 can play up to four games. He keeps four seasons to play in five years."],
  ["Academic redshirt", ["academic redshirt"], "A freshman who missed the full qualifying grades: he can practise but not play his first year."],
  ["Grayshirt", ["grayshirt", "grayshirted"], "A signee who delays enrolling until January, so his scholarship counts toward the next class."],
  ["Preferred walk-on", ["preferred walk-on", "preferred walk-ons"], "A recruit promised a roster place without a scholarship; some earn one later."],
  ["Walk-on", ["walk-on", "walk-ons"], "A player on the roster without an athletic scholarship."],
  ["NIL", ["nil"], "Name, image and likeness: since 1 July 2021 college players (and high schoolers in states that allow it) can be paid for endorsements and appearances."],
  ["Collective", ["collective", "collectives"], "A booster-funded group that pools money for a school's players' NIL deals."],
  ["House settlement", ["house settlement", "house limit"], "The 2025 court settlement: schools may pay players directly up to a yearly cap, and roster limits replace scholarship limits."],
  ["Transfer portal", ["transfer portal", "the portal"], "The NCAA's database a college player enters to transfer; other staffs can contact him from then on."],
  ["Verbal commitment", ["verbal commitment", "verbal", "verbally committed"], "A recruit's public pledge to a school. Nothing binds either side until he signs."],
  ["Decommit", ["decommit", "decommits", "decommitted"], "A recruit backing out of his verbal commitment."],
  ["Official visit", ["official visit", "official visits"], "A campus visit the school pays for, limited in number and timing by the rules."],
  ["Unofficial visit", ["unofficial visit", "unofficial visits"], "A campus visit the family pays for."],
  ["Signing day", ["signing day", "national signing day", "early signing period"], "When recruits sign binding letters with a school: the early period in December, the traditional day in February."],
  ["Contact period", ["contact period", "dead period", "quiet period", "evaluation period"], "The recruiting calendar's windows: contact allows visits and meetings off campus, quiet only on campus, dead none, evaluation watching without talking."],
  ["Scholarship limit", ["scholarship limit", "85 scholarships"], "The cap on players on athletic aid: 85 at FBS until the House settlement's roster limit, 63 equivalencies at FCS."],
  ["Hot seat", ["hot seat"], "A coach whose job is in danger."],
  ["Buyout", ["buyout", "buyouts"], "What a school or club owes to end a coach's contract early, or what a coach owes to leave one."],
  ["Combine", ["combine", "the combine"], "The NFL's invitation-only testing week in February: the forty, the bench, the jumps, the agility drills, the medicals and interviews."],
  ["Pro day", ["pro day", "pro days"], "A school's own testing day for its draft prospects, with NFL scouts watching."],
  ["Franchise tag", ["franchise tag", "franchise-tagged", "tagged"], "A one-year deal a club can force on its expiring player, at the average of the top salaries at his position."],
  ["Transition tag", ["transition tag"], "A cheaper one-year tag that gives the club the right to match any offer, without draft picks back."],
  ["Comp pick", ["comp pick", "comp picks", "compensatory pick", "compensatory picks"], "An extra draft pick from the league for losing more and better free agents than a club signed."],
  ["Restricted free agent", ["restricted free agent", "restricted free agents", "rfa"], "A player with three accrued seasons: his club can tender him and match any offer, or take draft picks back."],
  ["Tender", ["tender", "tenders", "tendered"], "A one-year offer at a set level to a restricted or exclusive-rights free agent."],
  ["Practice squad", ["practice squad"], "Up to 16 players who practise with an NFL club but are not on its 53; any club can sign them away."],
  ["Waivers", ["waivers", "waived", "waiver claim", "waiver wire"], "A released player goes on waivers: clubs can claim his contract, worst record first."],
  ["Dead money", ["dead money"], "Cap charges for a player no longer on the club: the signing bonus not yet counted, and guarantees."],
  ["Salary cap", ["salary cap", "cap space", "the cap"], "The league's limit on what a club can spend on player salaries in a league year."],
  ["Rookie contract", ["rookie contract", "rookie deal"], "A drafted player's first deal, set by the rookie wage scale: four years, plus a fifth-year option in the first round."],
  ["Fifth-year option", ["fifth-year option"], "A first-round pick's club can add a fifth season to his rookie deal, at a set price."],
  ["53-man roster", ["53-man roster", "cutdown", "cut-down"], "NFL clubs cut to 53 players before the season; 48 dress on game day."],
  ["OTAs", ["otas", "organized team activities", "minicamp", "minicamps"], "The NFL's spring practices: voluntary OTAs, then one mandatory minicamp, with no live contact."],
  ["Spring practice", ["spring practice", "spring ball"], "College teams' 15 spring practices, ending in a spring game; high school spring ball where the state allows it."],
];
// One case-insensitive pattern for every word that finds a term, longest
// first, on whole words. Acronyms only match in capitals (no "nil" in a sentence).
const H7_ALIAS = new Map();
for (const [term, finds, text] of H7_TERMS) for (const f of finds) H7_ALIAS.set(f, { term, text });
const H7_CAPS = new Set(["nil", "rpo", "rpos", "rfa", "otas"]);
const H7_RE = new RegExp(`(^|[^A-Za-z0-9-])(${[...H7_ALIAS.keys()].sort((a, b) => b.length - a.length).map((s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})(?![A-Za-z0-9-])`, "i");

// ---------- the scan: marking names and terms in a page ----------
const HP = { cache: new Map(), timer: 0, hideTimer: 0, watch: 0, el: null, on: null, labels: null, seen: new WeakMap(), queued: new Set(), scanning: false };
const HP_SKIP = "a,button,input,textarea,select,option,script,style,svg,code,[data-rr],[data-h1],[data-h3],[data-h7],.rr-card,.hp-card,[contenteditable],.st-label,.h1,h1,h2,.sb-last,.sb-first";
async function hpLabels() {
  if (HP.labels) return HP.labels;
  try {
    const v = await api("help_attrs", {});
    HP.labels = new Map(v.map((x) => [x.label, x.key]));
  } catch (e) {
    HP.labels = new Map();
  }
  return HP.labels;
}
// The roots help reads: the screen, the page beside it and an open letter.
const HP_ROOTS = ".view, .beside-body, .lt-paper, .letter-room";
function hpScan(root) {
  if (!root || !root.isConnected || prefs.hover === "off") return;
  const labels = HP.labels;
  let seen = HP.seen.get(root);
  if (!seen) HP.seen.set(root, (seen = new Set()));
  const walk = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (n) => (n.nodeValue.trim().length < 2 || !n.parentElement || n.parentElement.closest(HP_SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
  });
  const nodes = [];
  for (let n = walk.nextNode(); n && nodes.length < 6000; n = walk.nextNode()) nodes.push(n);
  for (const n of nodes) {
    const el = n.parentElement;
    const text = n.nodeValue.trim();
    // H1: an element whose whole text is a rating's name.
    if (labels && el.childNodes.length === 1 && labels.has(text)) {
      el.dataset.h1 = labels.get(text);
      el.classList.add("hp-name");
      if (!el.hasAttribute("tabindex")) el.tabIndex = 0;
      continue;
    }
    // H7: the first time each term shows on this page.
    if (seen.size >= 40 || n.nodeValue.length > 2000) continue;
    const m = n.nodeValue.match(H7_RE);
    if (!m) continue;
    const word = m[2];
    const key = word.toLowerCase();
    const hit = H7_ALIAS.get(key);
    if (!hit || seen.has(hit.term)) continue;
    if (H7_CAPS.has(key) && word !== word.toUpperCase()) continue;
    seen.add(hit.term);
    const at = m.index + m[1].length;
    const rest = n.splitText(at);
    rest.splitText(word.length);
    const span = document.createElement("span");
    span.className = "hp-term";
    span.dataset.h7 = hit.term;
    span.tabIndex = 0;
    span.textContent = word;
    rest.replaceWith(span);
  }
}
function hpQueue(root) {
  HP.queued.add(root);
  if (HP.scanning) return;
  HP.scanning = true;
  setTimeout(async () => {
    await hpLabels();
    const roots = [...HP.queued];
    HP.queued.clear();
    HP.scanning = false;
    for (const r of roots) hpScan(r);
  }, 60);
}
new MutationObserver((list) => {
  for (const m of list) {
    const t = m.target.nodeType === 1 ? m.target : m.target.parentElement;
    if (!t || (t.closest && t.closest(".rr-card,.hp-card"))) continue;
    const root = t.closest && t.closest(HP_ROOTS);
    if (root) { hpQueue(root); continue; }
    for (const a of m.addedNodes) {
      if (a.nodeType !== 1) continue;
      const r = a.matches(HP_ROOTS) ? a : a.querySelector(HP_ROOTS);
      if (r) hpQueue(r);
    }
  }
}).observe(document.body, { childList: true, subtree: true });

// ---------- the cards ----------
function hpHide() {
  clearTimeout(HP.timer);
  clearInterval(HP.watch);
  if (HP.el) HP.el.remove();
  HP.el = null;
  HP.on = null;
}
function hpAttrHtml(x) {
  return `<div class="rr-top"><span class="st-label">Rating</span><b class="hp-big">${esc(x.label)}</b></div>
    <p class="hp-what">${esc(x.what)}</p>
    <p>${esc(x.field)}</p>
    <p class="rr-comb"><span class="st-label">How it grows</span>${esc(x.grows)}</p>
    <p class="rr-pool">${esc(x.matters)} Hover a range for where he stands.</p>`;
}
function hpLevelHtml(x) {
  return `<div class="rr-top"><span class="st-label">Level rating</span><b>${x.rating}</b></div>
    <div class="rr-word">${esc(x.title)}</div>
    <p>${esc(x.what)}</p>
    <p>${esc(x.pool)}</p>
    <p class="rr-comb"><span class="st-label">The scale</span>${esc(x.scale)}</p>
    ${x.above ? `<p class="hp-above">${esc(x.above)}</p>` : ""}
    <p class="rr-pool">${esc(x.public)}</p>`;
}
function hpTermHtml(term) {
  const t = H7_TERMS.find((x) => x[0] === term);
  return `<div class="rr-top"><span class="st-label">Football term</span><b class="hp-big">${esc(term)}</b></div><p>${esc(t ? t[2] : "")}</p>`;
}
function hpPlace(t, el) {
  let r = t.getBoundingClientRect();
  const w = el.offsetWidth, h = el.offsetHeight;
  let left = r.left - 12, top = r.bottom + 8;
  if (left + w > window.innerWidth - 12) left = window.innerWidth - w - 12;
  if (top + h > window.innerHeight - 12) top = r.top - h - 8;
  el.style.left = Math.max(12, left) + "px";
  el.style.top = Math.max(12, top) + "px";
}
function hpShow(t, html) {
  if (HP.on !== t || !t.isConnected) return;
  if (HP.el) HP.el.remove();
  if (typeof rrHide === "function") rrHide();
  const el = document.createElement("div");
  el.className = "rr-card sl hp-card";
  el.setAttribute("role", "tooltip");
  el.innerHTML = html;
  document.body.appendChild(el);
  hpPlace(t, el);
  el.addEventListener("mouseenter", () => clearTimeout(HP.hideTimer));
  el.addEventListener("mouseleave", () => { HP.hideTimer = setTimeout(hpHide, 160); });
  HP.el = el;
  clearInterval(HP.watch);
  HP.watch = setInterval(() => { if (!HP.on || !HP.on.isConnected) hpHide(); }, 250);
}
function hpOpen(t, delay) {
  if (prefs.hover === "off") return;
  clearTimeout(HP.timer);
  clearTimeout(HP.hideTimer);
  if (HP.el && HP.on !== t) { HP.el.remove(); HP.el = null; }
  HP.on = t;
  HP.timer = setTimeout(async () => {
    const d = t.dataset;
    if (d.h7) return hpShow(t, hpTermHtml(d.h7));
    const ck = d.h1 ? `a|${d.h1}` : `l|${S.status ? S.status.today : ""}|${d.h3}`;
    let x = HP.cache.get(ck);
    if (!x) {
      try {
        x = d.h1 ? await api("help_attr", { k: d.h1 }) : await api("help_level", { p: Number(d.h3) });
      } catch (e) {
        x = { error: e.message || String(e) };
      }
      if (HP.cache.size > 300) HP.cache.clear();
      HP.cache.set(ck, x);
    }
    hpShow(t, x.error ? `<p>${esc(x.error)}</p>` : d.h1 ? hpAttrHtml(x) : hpLevelHtml(x));
  }, delay);
}
const HP_SEL = "[data-h1],[data-h3],[data-h7]";
document.addEventListener("mouseover", (ev) => {
  const t = ev.target.closest && ev.target.closest(HP_SEL);
  if (!t || t === HP.on) return;
  hpOpen(t, Math.min(450, Number(prefs.hoverDelay) || 350));
});
document.addEventListener("mouseout", (ev) => {
  const t = ev.target.closest && ev.target.closest(HP_SEL);
  if (!t || t !== HP.on) return;
  if (ev.relatedTarget && t.contains(ev.relatedTarget)) return;
  clearTimeout(HP.timer);
  HP.hideTimer = setTimeout(() => { if (HP.on === t) hpHide(); }, 160);
});
document.addEventListener("focusin", (ev) => {
  const t = ev.target.closest && ev.target.closest(HP_SEL);
  if (t) hpOpen(t, 120);
});
document.addEventListener("focusout", (ev) => {
  const t = ev.target.closest && ev.target.closest(HP_SEL);
  if (t && t === HP.on) hpHide();
});
document.addEventListener("scroll", () => { if (HP.el) hpHide(); }, true);
document.addEventListener("mousedown", (ev) => { if (HP.el && !HP.el.contains(ev.target)) hpHide(); });

// ---------- H6: why-lines ----------
// "Why, as your staff sees it": two or three reasons under a letter or a story.
function hpWhy(why, cls = "") {
  if (!why || !why.length) return "";
  return `<div class="hp-why ${cls}"><span class="st-label">Why, as your staff sees it</span><ul>${why.map((t) => `<li>${rich(t)}</li>`).join("")}</ul></div>`;
}
