/* Capital's Donors screen (lane CS2; docs/design/capital.md CP37, CP39, board 2).
 * The people who could fund your next building, and what you can do about it
 * this week:
 *
 *   capitaldonors   the fever (how hot donors are for your program and which
 *                   moments set it), your donors as a list (the office's range
 *                   in words, his loyalty to you, when you last met, old
 *                   guard or new money, buildings or NIL once you've met
 *                   him), and the picked donor's page: what drives him, what
 *                   he remembers of you, his circle, the development
 *                   director's read and a lunch or dinner you can book.
 *
 * It reads `capital_donors` and `capital_week` and writes `capital_book` and
 * `capital_unbook` (lane C4). No dollar figure is on it (CP39): money reads in
 * words. Press Box look (CP39): dark, ivory, one gold accent, amber for what
 * slips. Every class is cpd- so the app's global names never restyle it. */
"use strict";

if (!NAV_SECS.some((s) => s[0] === "capital")) NAV_SECS.splice(NAV_SECS.findIndex((s) => s[0] === "team"), 0, ["capital", "Capital"]);
I.cpdonor = '<circle cx="9" cy="8" r="3.5"/><path d="M2.5 20c0-3.6 2.9-6 6.5-6s6.5 2.4 6.5 6M16 5a3.5 3.5 0 0 1 0 7M18 14.5c2.2.6 3.5 2.4 3.5 5.5"/>';
navAdd({ go: "capitaldonors", label: "Donors", icon: "cpdonor", sec: "capital", seats: ["college"] });
CRUMBS.capitaldonors = "Donors";

const CPD = { id: null, sort: "time", meal: "dinner", day: null };
const CPD_MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const CPD_DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const cpdDate = (iso) => new Date(iso + "T12:00:00");
const cpdShort = (iso) => { const d = cpdDate(iso); return `${CPD_MONTHS[d.getMonth()]} ${d.getDate()}`; };
const cpdMonthYear = (iso) => { const d = cpdDate(iso); return `${CPD_MONTHS[d.getMonth()]} ${d.getFullYear()}`; };
const cpdLong = (iso) => cpdDate(iso).toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric", year: "numeric" });
const cpdIso = (d) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
const cpdIni = (name, size = 44) => `<span class="cpd-ini" style="--s:${size}px">${esc((name || "?").split(/[\s,]+/).filter(Boolean).map((x) => x[0]).slice(0, 2).join("").toUpperCase())}</span>`;
const cpdFirst = (n) => (n || "").split(" ")[0];
const cpdQuote = (t) => `“${String(t || "").replace(/^["“]+|["”"]+$/g, "")}”`;
const cpdCap = (s) => (s ? s[0].toUpperCase() + s.slice(1) : "");

// "2 weeks", "9 days", "Never": how long since you sat down with him.
function cpdSeen(d) {
  if (d.seenDays == null) return { text: "Never", slip: true };
  const n = d.seenDays;
  const text = n < 1 ? "Today" : n === 1 ? "Yesterday" : n < 14 ? `${n} days` : n < 60 ? `${Math.round(n / 7)} weeks` : `${Math.round(n / 30)} months`;
  return { text, slip: n >= 120 };
}
// His loyalty to you as a word (the bar is his warmth, 0 cold to 1 he'd follow you anywhere).
function cpdLoyalty(d) {
  if (d.met === 0 && d.last == null) return "Not met";
  const w = d.warmth;
  return w >= 0.8 ? "Devoted" : w >= 0.65 ? "Close" : w >= 0.45 ? "Friendly" : w >= 0.3 ? "Cool" : "Cold";
}
const cpdHe = (d) => (d.she ? "she" : "he");
const cpdHim = (d) => (d.she ? "her" : "him");

screens.capitaldonors = async (args) => {
  let v;
  try { v = await api("capital_donors", {}); } catch (e) {
    title("Donors", "Capital");
    return `<div class="pad sl cpd"><section class="cpd-pb cpd-none"><div class="cpd-lab">Donors</div><b class="cpd-h">Donors are the head coach's to court.</b>
      <p>${esc(e.message || "You're not the head coach of a college program.")} At a program where you are, your donors appear here, with the fever after your big moments.</p></section></div>`;
  }
  if (args && args.id) { CPD.id = +args.id; delete args.id; }
  const all = v.donors;
  title("Donors", v.program);
  if (!all.length) {
    return `<div class="pad sl cpd">${cpdTop(v, all)}${cpdFever(v.fever)}<section class="cpd-pb cpd-none"><div class="cpd-lab">No donors yet</div><b class="cpd-h">The development office has no names for you yet.</b>
      <p>Programs below the Power 4 keep fewer. Winning brings more, and your boosters in the talks are on this list when it fills.</p></section></div>`;
  }
  const list = cpdSort(all);
  if (!all.some((d) => d.id === CPD.id)) CPD.id = list[0].id;
  const sel = all.find((d) => d.id === CPD.id);
  let wk = null;
  try { wk = await api("capital_week", {}); } catch (e) { wk = null; }
  setTimeout(cpdFaces, 30);
  return `<div class="pad sl cpd">${cpdTop(v, all)}${cpdFever(v.fever)}
    <div class="cpd-grid"><div class="cpd-col">${cpdList(list)}</div>
      <div class="cpd-col side">${cpdPage(sel, v)}${cpdBook(sel, v, wk)}</div></div></div>`;
};

function cpdTop(v, all) {
  const lead = all.filter((d) => d.lead).length;
  const unseen = all.filter((d) => d.seenDays == null || d.seenDays >= 120).length;
  return `<div class="cpd-top"><div><h1>${esc(cpdLong(v.today))}</h1><small>${esc(v.program)}, the development office's list.</small></div><div class="cpd-sp"></div>
    <div class="cpd-stat"><span>Donors</span><b>${all.length}</b></div>
    ${lead ? `<div class="cpd-stat"><span>Lead donors for the drive</span><b>${lead}</b></div>` : ""}
    <div class="cpd-stat"><span>Not seen in four months</span><b>${unseen}</b></div></div>`;
}

// The fever: how hot donors are for you, and the moments that set it.
function cpdFever(f) {
  const segs = f.moments.map((m, i) => `<i class="s${Math.min(i, 3)}" style="width:${Math.max(1, Math.round(m.share * 100))}%"></i>`).join("");
  const why = f.moments.map((m, i) => `<span><b class="${i === 0 ? "gold" : ""}">${esc(cpdCap(m.what))}${m.who ? `: ${esc(m.who)}` : ""}, ${esc(cpdMonthYear(m.date))}</b>${i === 0 ? " the biggest part" : ""}</span>`).join("");
  const fade = f.value < 0.05 ? "Nothing is carrying donors right now." : f.fades ? `Fades by ${esc(cpdMonthYear(f.fades))} without another run.` : "Still climbing with the run.";
  return `<section class="cpd-pb cpd-fever" aria-label="The fever"><div class="cpd-fv"><div class="cpd-lab">The fever</div><div class="cpd-word ${f.value >= 0.4 ? "hot" : ""}">${esc(f.word)}</div></div>
    <div class="cpd-fb"><div class="cpd-track" role="img" aria-label="Fever ${Math.round(f.value * 100)} out of 100">${segs}</div>
    <div class="cpd-why">${why}<span>${fade}</span></div></div></section>`;
}

function cpdSort(all) {
  const a = [...all];
  if (CPD.sort === "loyal") a.sort((x, y) => y.warmth - x.warmth);
  else if (CPD.sort === "unseen") a.sort((x, y) => (y.seenDays == null ? 1e9 : y.seenDays) - (x.seenDays == null ? 1e9 : x.seenDays));
  return a; // "time": the office's order, as the engine sent it
}

function cpdList(list) {
  const seg = [["time", "Worth your time"], ["loyal", "Closest to you"], ["unseen", "Longest unseen"]]
    .map(([k, t]) => `<button class="cpd-seg${CPD.sort === k ? " on" : ""}" role="tab" aria-selected="${CPD.sort === k}" data-act="cpdsort" data-k="${k}">${t}</button>`).join("");
  const rows = list.map((d) => {
    const s = cpdSeen(d);
    const lo = d.warmth < 0.35 && d.met > 0;
    const face = d.pro ? `<span class="cpd-face" data-face="${d.pro}">${cpdIni(d.name)}</span>` : cpdIni(d.name);
    const tags = [d.lead ? `<span class="cpd-tag gold">Lead donor</span>` : "", d.pledging ? `<span class="cpd-tag">Pledging</span>` : "", `<span class="cpd-tag">${d.money}</span>`, d.side ? `<span class="cpd-tag">${esc(d.side)}</span>` : ""].join("");
    return `<button class="cpd-row${d.id === CPD.id ? " sel" : ""}" data-act="cpdpick" data-id="${d.id}" aria-pressed="${d.id === CPD.id}">${face}
      <span class="cpd-who"><b>${esc(d.name)}</b><small>${esc(cpdCap(d.business))}</small><span class="cpd-tags">${tags}</span></span>
      <span class="cpd-cap">${esc(d.read)}</span>
      <span class="cpd-loy"><span class="cpd-bar${lo ? " amb" : ""}"><i style="width:${d.met === 0 && d.last == null ? 0 : Math.round(d.warmth * 100)}%"></i></span><small>${esc(cpdLoyalty(d))}</small></span>
      <span class="cpd-seen${s.slip ? " amb" : ""}">${esc(s.text)}</span></button>`;
  }).join("");
  return `<div class="cpd-bar2" role="tablist" aria-label="Sort donors">${seg}</div>
    <div class="cpd-head"><span></span><span>Donor</span><span>Could give</span><span>Loyalty to you</span><span>Last seen</span></div>${rows}
    <p class="cpd-foot">The range is the development office's screening of what a donor's assets show. It misses debt, and it narrows each time you meet him. His lean to buildings or NIL shows once you have met.</p>`;
}

// The picked donor's page.
function cpdPage(d, v) {
  const he = cpdHe(d);
  const face = d.pro ? `<span class="cpd-face big" data-face="${d.pro}">${cpdIni(d.name, 56)}</span>` : cpdIni(d.name, 56);
  const sub = [`${d.age}`, d.alum ? "an alum" : "", d.coached ? "you coached him" : ""].filter(Boolean).join(" · ");
  const wants = d.wantsLong.length ? d.wantsLong.map((w) => cpdCap(w)).join(". ") + "." : "Nothing in particular yet.";
  const rooms = d.rooms.length ? `Would put money into the ${d.rooms.join(" or the ")}.` : "";
  const rec = [];
  if (d.met === 0 && d.last == null) rec.push("You haven't met.");
  else {
    rec.push(`You've sat down ${d.met === 1 ? "once" : `${d.met} times`}${d.last ? `, last on ${cpdShort(d.last)}` : ""}. ${cpdCap(cpdHe(d))} is ${d.withYou}.`);
    if (d.kept || d.broken) rec.push(`${d.kept ? `You've kept ${d.kept} ${d.kept === 1 ? "promise" : "promises"} to ${cpdHim(d)}` : ""}${d.kept && d.broken ? " and " : ""}${d.broken ? `${d.kept ? "broken" : "You've broken"} ${d.broken}` : ""}.`);
    if (d.heard) rec.push(`${cpdCap(he)} has heard about ${d.heard === 1 ? "a promise" : `${d.heard} promises`} you broke to people in the same circle.`);
  }
  const circle = d.circle ? `<div><div class="cpd-lab">Who ${he} talks to</div><div class="cpd-p">${esc(cpdCap(d.circle.name))}${d.circle.mates.length ? `, with ${esc(d.circle.mates.slice(0, 4).join(", "))}` : ""}. ${d.circle.old ? "Old money: what one hears, they all hear." : "New money: they keep their own counsel."}</div></div>` : "";
  const also = d.also ? `<p class="cpd-p amb">${esc(d.also)} can court ${cpdHim(d)} too.</p>` : "";
  const catchLine = d.met > 0 || d.last ? `<p class="cpd-p">${esc(d.catch)}</p>` : "";
  return `<section class="cpd-pb cpd-page"><div class="cpd-hd">${face}<div class="cpd-hn"><b>${esc(d.name)}</b><small>${esc(sub)}${sub ? " · " : ""}${esc(cpdCap(d.business))}</small></div></div>
    <div><div class="cpd-lab">What drives ${cpdHim(d)}</div><div class="cpd-p">${esc(wants)} ${esc(rooms)}</div></div>
    <div><div class="cpd-lab">What ${he} remembers of you</div><div class="cpd-p">${rec.map(esc).join(" ")}</div></div>
    ${circle}
    <div class="cpd-read"><div class="cpd-lab">Your development director's read</div><p class="cpd-p">${esc(d.read)}${d.met === 0 ? " The range is wide until you've met." : ""}</p>${catchLine}${also}</div></section>`;
}

// Book a lunch or a dinner: any weekday from tomorrow to eight weeks out,
// January to August; the engine says why a day won't do.
function cpdBook(d, v, wk) {
  const today = cpdDate(v.today);
  const taken = new Map((v.booked || []).map((b) => [b.date, b]));
  const days = [];
  for (let k = 1; k <= 56 && days.length < 10; k++) {
    const x = new Date(today); x.setDate(x.getDate() + k);
    const g = x.getDay();
    if (g === 0 || g === 6 || x.getMonth() >= 8) continue;
    days.push(cpdIso(x));
  }
  if (!days.length) {
    return `<section class="cpd-pb"><div class="cpd-lab">This week</div><p class="cpd-note">From September to December your only donor time is the hour in a suite before a home game. Lunches and dinners open again in January.</p>${cpdWeek(wk)}</section>`;
  }
  if (!days.includes(CPD.day)) CPD.day = days.find((x) => !taken.has(x)) || days[0];
  const chips = days.map((x) => {
    const b = taken.get(x);
    const dt = cpdDate(x);
    return `<button class="cpd-day${x === CPD.day ? " on" : ""}${b ? " off" : ""}" data-act="cpdday" data-d="${x}" ${b ? "disabled" : ""} aria-pressed="${x === CPD.day}"><b>${CPD_DAYS[dt.getDay()]}</b><span>${CPD_MONTHS[dt.getMonth()]} ${dt.getDate()}</span>${b ? `<small>Booked</small>` : ""}</button>`;
  }).join("");
  const mine = taken.get(CPD.day);
  const meal = `<div class="cpd-bar2" role="tablist" aria-label="Lunch or dinner">${["lunch", "dinner"].map((m) => `<button class="cpd-seg${CPD.meal === m ? " on" : ""}" role="tab" aria-selected="${CPD.meal === m}" data-act="cpdmeal" data-m="${m}">${cpdCap(m)}</button>`).join("")}</div>`;
  const label = `${cpdCap(CPD.meal)} with ${esc(cpdFirst(d.name))} on ${esc(CPD_DAYS[cpdDate(CPD.day).getDay()])}, ${esc(cpdShort(CPD.day))}`;
  const booked = (v.booked || []).filter((b) => b.date >= v.today).map((b) => {
    const who = (cpdAllDonors(v, b.donor) || {}).name || "A donor";
    return `<div class="cpd-bk"><span><b>${esc(who)}</b> · ${CPD_DAYS[cpdDate(b.date).getDay()]}, ${esc(cpdShort(b.date))}, ${b.minute < 15 * 60 ? "lunch" : "dinner"}</span><button class="cpd-lnk" data-act="cpdunbook" data-id="${b.donor}" data-d="${b.date}">Call off</button></div>`;
  }).join("");
  return `<section class="cpd-pb"><div class="cpd-lab">This week</div>${cpdWeek(wk)}
    <div class="cpd-lab sp">Book a meal</div>${meal}<div class="cpd-days">${chips}</div>
    <p class="cpd-note">${mine ? "That day has a donor already." : "A day you are on the road or on your week off won't take it. Booking a day in this week bumps the least important thing on it to your coordinator."}</p>
    <button class="cpd-cta" data-act="cpdbook" data-id="${d.id}" ${mine ? "disabled" : ""}>${label}</button>${booked ? `<div class="cpd-bks">${booked}</div>` : ""}</section>`;
}
const cpdAllDonors = (v, id) => (v.donors || []).find((x) => x.id === id);

// This week's donor slots (the meetings the map keeps for donors).
function cpdWeek(wk) {
  const slots = wk ? wk.slots : [];
  if (!slots.length) return `<p class="cpd-note">No donor on your week yet. The development office has the lunches nobody asked you to take.</p>`;
  return `<div class="cpd-wk">${slots.map((s) => `<div class="cpd-sl${s.yours ? " mine" : ""}"><b>${esc(CPD_DAYS[cpdDate(s.date).getDay()])}</b><span>${esc(s.title)}</span><small>${s.yours ? "Yours" : "The office's"}${s.donor ? ` · ${esc(s.donor.withYou)}` : ""}</small></div>`).join("")}</div>`;
}

function cpdFaces() {
  const slots = [...document.querySelectorAll(".cpd-face[data-face]")];
  if (!slots.length || !window.AFMFaces || !AFMFaces.pics) return;
  const ids = [...new Set(slots.map((s) => +s.dataset.face))];
  AFMFaces.pics(ids, (id, src) => document.querySelectorAll(`.cpd-face[data-face="${id}"]`).forEach((s) => {
    if (!s.querySelector("img")) s.insertAdjacentHTML("afterbegin", `<img src="${src}" alt="">`);
    s.classList.add("has");
  }));
}

// ---- actions ----
Object.assign(CORE_MORE, {
  cpdpick: (el) => { CPD.id = +el.dataset.id; refresh(el); },
  cpdsort: (el) => { CPD.sort = el.dataset.k; refresh(el); },
  cpdmeal: (el) => { CPD.meal = el.dataset.m; refresh(el); },
  cpdday: (el) => { CPD.day = el.dataset.d; refresh(el); },
  cpdbook: async (el) => {
    try {
      const r = await api("capital_book", { donor: +el.dataset.id, date: CPD.day, dinner: CPD.meal === "dinner" });
      if (r && r.said) toast(r.said);
    } catch (e) { toast(e.message); }
    refresh(el);
  },
  cpdunbook: async (el) => {
    try {
      const r = await api("capital_unbook", { donor: +el.dataset.id, date: el.dataset.d });
      if (r && r.said) toast(r.said);
    } catch (e) { toast(e.message); }
    refresh(el);
  },
});
