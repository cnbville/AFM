/* The calendar (update 3, lane 2: CAL1-CAL7, board 41) in Press Box: the
 * year strip with its two lanes, the month with a week column, readable day
 * cells in four kinds, the period line under each day, the picked day hour
 * by hour, and Ahead. It replaces the first playable's calendar; the old
 * "calendar_month" command stays for anything else that reads it. Every
 * class is cb- so the app's global names never restyle it. */

const CB_GLYPH = (k) => `<i class="cb-gl ${k}" aria-hidden="true"></i>`;

screens.calendar = async (a) => {
  const C = keep("calmonth", {});
  if (a.day) { C.day = a.day; C.month = null; }
  if (a.month) { C.month = a.month; C.day = null; }
  const x = await api("calendar_board", C.day ? { day: C.day } : C.month ? { month: C.month } : {});
  title("Calendar", x.title);
  const view = C.view === "List" ? "List" : "Month";
  const kicker = x.head ? `${mark(x.head.team, 40)}` : "";
  const tabBtn = (n, attrs) => `<button class="cb-tab" ${attrs} aria-selected="${n === view}">${n}</button>`;
  const head = `<div class="cb-head">
    <div class="cb-title">${kicker}<div><div class="st-label">${esc(x.head ? x.head.label : "")}</div><h1>${esc(x.title)}</h1></div>
      <nav class="cb-tabs" role="tablist">${tabBtn("Month", 'data-act="cbview" data-v="Month"')}${x.level === "other" ? "" : tabBtn("Week", 'data-go="training"')}${tabBtn("The year", 'data-go="year"')}${tabBtn("List", 'data-act="cbview" data-v="List"')}</nav></div>
    <div class="cb-nav"><button class="cb-btn" data-act="cbmonth" data-v="${x.prev}" aria-label="Previous month">‹</button><button class="cb-btn" data-act="cbmonth" data-v="">Today</button><button class="cb-btn" data-act="cbmonth" data-v="${x.next}" aria-label="Next month">›</button><b>${esc(x.today_text)}</b></div>
  </div>`;
  const body = view === "List" ? cbList(x) : cbMonth(x);
  return `<div class="pad sl cb cb-${x.level}">${head}${cbStrip(x)}<div class="cb-main">${body}<div class="cb-side">${cbDay(x, C)}${cbAhead(x)}</div></div></div>`;
};
GOTO.calendar = () => [["Schedule", "schedule"], ["Training", "training"], ["Inbox", "inbox"], ["News", "news"]];

// CAL1: the year strip, the football phases on one lane and the recruiting
// periods (or the draft and scouting year) on the other.
function cbStrip(x) {
  const s = x.strip;
  if (!s) return "";
  const months = s.months.map((m) => `<button class="cb-mo ${m.on ? "on" : ""}" style="left:${m.x}%" data-act="cbmonth" data-v="${m.month}">${esc(m.n)}</button>`).join("");
  const phases = s.phases.map((p) => `<button class="cb-ph l${p.l}" style="left:${p.x}%;width:${p.w}%" data-act="cbmonth" data-v="${p.month}" title="${esc(`${p.n}: ${p.from} to ${p.to}`)}">${esc(p.n)}</button>`).join("");
  const marks = s.marks.map((k) => `<button class="cb-mk" style="left:${k.x}%" data-act="cbday" data-iso="${k.iso}" title="${esc(`${k.n} · ${k.date}`)}" aria-label="${esc(`${k.n}, ${k.date}`)}"></button>`).join("");
  const pers = s.pers.map((p) => `<i class="cb-per ${p.k}" style="left:${p.x}%;width:${p.w}%"></i>`).join("");
  const lb = s.lanes[1];
  return `<section class="cb-panel cb-strip ${lb ? "" : "one"}">
    <div class="cb-sl"><b>${esc(s.label)}</b><span>${esc(s.lanes[0])}</span>${lb ? `<span class="b">${esc(lb)}</span>` : ""}</div>
    <div class="cb-sr"><div class="cb-mos">${months}</div><div class="cb-lane">${phases}${marks}</div>${lb ? `<div class="cb-pers">${pers}</div>` : ""}
      <span class="cb-box" style="left:${s.box.x}%;width:${s.box.w}%"></span>${s.today != null ? `<span class="cb-now" style="left:${s.today}%"></span>` : ""}</div>
  </section>`;
}

// CAL2-CAL5: the month, a week column and the days.
function cbMonth(x) {
  const C = keep("calmonth", {});
  const dows = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map((d) => `<span>${d}</span>`).join("");
  let cells = "";
  x.weeks.forEach((w, i) => {
    // Another league's club runs its own staff's plan: no training screen.
    const wk = x.level === "other" ? "disabled" : w.now ? 'data-go="training" title="This week\'s practice plan"' : 'data-go="training" data-tab="The year" title="Your training year"';
    cells += `<button class="cb-wk ${w.now ? "now" : ""}" ${wk}><b>${esc(w.big)}</b>${w.a ? `<span class="a">${esc(w.a)}</span>` : ""}${w.b ? `<span class="b">${esc(w.b)}</span>` : ""}</button>`;
    cells += x.days.slice(i * 7, i * 7 + 7).map((d) => cbCell(d, C.day ? C.day === d.iso : x.day.iso === d.iso)).join("");
  });
  const kinds = [["g", "Game"], ["y", "Needs you"], ["r", x.legend.r], ["w", "The world"]].map(([k, n]) => `<span>${CB_GLYPH(k)}${esc(n)}</span>`).join("");
  const pers = x.legend.periods.map((p) => `<span><i class="cb-pl ${p.k}"></i>${esc(p.n)}</span>`).join("");
  return `<section class="cb-panel cb-month">
    <div class="cb-grid cb-dows"><span>Week</span>${dows}</div>
    <div class="cb-grid cb-days">${cells}</div>
    <div class="cb-legend">${kinds}${pers ? `<span class="cb-sep"></span>${pers}` : ""}</div>
  </section>`;
}

function cbCell(d, picked) {
  const lines = d.lines.slice(0, 3).map((l) => `<span class="cb-ln ${l.k}">${CB_GLYPH(l.k)}<span>${esc(l.t)}</span></span>`).join("");
  const more = d.lines.length > 3 ? `<span class="cb-more">+${d.lines.length - 3} more</span>` : "";
  const cls = ["cb-d", d.game ? "g" : "", d.in_month ? "" : "out", d.past ? "past" : "", d.today ? "today" : "", picked ? "pick" : ""].join(" ");
  return `<button class="${cls}" data-act="cbday" data-iso="${d.iso}" aria-pressed="${picked}"><span class="cb-dt"><b>${d.day}</b>${d.res ? `<em class="${d.rk ? d.rk.toLowerCase() : "ha"}">${esc(d.res)}</em>` : ""}</span>${lines}${more}${d.per ? `<i class="cb-ps ${d.per}"></i>` : ""}</button>`;
}

// The month as a list: every day with something on it.
function cbList(x) {
  const C = keep("calmonth", {});
  const days = x.days.filter((d) => d.in_month && d.lines.length);
  const rows = days.map((d) => {
    const dt = new Date(d.iso + "T12:00:00");
    const wd = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"][dt.getDay()];
    const picked = C.day ? C.day === d.iso : x.day.iso === d.iso;
    return `<button class="cb-lrow ${d.past ? "past" : ""} ${picked ? "pick" : ""} ${d.today ? "today" : ""}" data-act="cbday" data-iso="${d.iso}"><span class="cb-ld"><b>${d.day}</b><small>${wd}</small></span><span class="cb-lls">${d.lines.map((l) => `<span class="cb-ln ${l.k}">${CB_GLYPH(l.k)}<span>${esc(l.t)}</span></span>`).join("")}</span>${d.res ? `<em class="cb-lres ${d.rk ? d.rk.toLowerCase() : "ha"}">${esc(d.res)}</em>` : "<span></span>"}</button>`;
  }).join("");
  return `<section class="cb-panel cb-list"><div class="cb-ph2"><b>${esc(x.title)}</b><span>${days.length} days with something on them</span></div>${rows || '<div class="cb-empty">Nothing on the calendar this month.</div>'}</section>`;
}

// CAL6: the picked day, hour by hour.
function cbDay(x, C) {
  const d = x.day;
  const all = C.all === d.iso;
  const shown = all ? d.rows : d.rows.slice(0, 4);
  const row = (r) => {
    const go = r.go ? (() => { const { screen, label, ...rest } = r.go; return { screen, label, args: rest }; })() : null;
    const goAttrs = go ? `data-go="${esc(go.screen)}" data-args='${esc(JSON.stringify(go.args))}'` : "";
    const act = r.act ? (r.act.mail != null ? `<button class="cb-chip" data-act="openmail" data-id="${esc(r.act.mail)}">${esc(r.act.label)}</button>` : r.act.scene != null ? `<button class="cb-chip" data-go="people_scene" data-id="${r.act.scene}">${esc(r.act.label)}</button>` : "") : "";
    return `<div class="cb-row ${r.k === "y" ? "you" : ""} ${go ? "click" : ""}" ${goAttrs}><span class="cb-time">${esc(r.time)}</span><div class="cb-rc"><div class="cb-rh"><span class="cb-k ${r.k}">${esc(r.label)}</span>${go ? `<span class="cb-go">${esc(go.label)} ›</span>` : ""}</div><div class="cb-rt">${rich(r.t)}</div>${r.s ? `<div class="cb-rs">${rich(r.s)}</div>` : ""}${act ? `<div>${act}</div>` : ""}</div></div>`;
  };
  const rest = d.rows.slice(4);
  const more = !all && rest.length ? `<button class="cb-dmore" data-act="cball" data-iso="${d.iso}">+${rest.length} more on this day: ${esc(rest.map((r) => plainText(r.t)).join(" · "))}</button>` : "";
  const empty = d.rows.length ? "" : `<div class="cb-empty">Nothing on the calendar. The week's plan runs as set.</div>`;
  return `<section class="cb-panel cb-day"><div class="cb-dh"><h2>${esc(d.title)}</h2><span class="cb-rel ${d.today ? "now" : ""}">${esc(d.rel)}</span></div>${d.ctx ? `<div class="cb-ctx">${esc(d.ctx)}</div>` : ""}${shown.map(row).join("")}${more}${empty}</section>`;
}

// CAL7: the next four dates that matter, with the one number that counts.
function cbAhead(x) {
  if (!x.ahead || !x.ahead.length) return "";
  const rows = x.ahead.map((a) => {
    const bar = a.bar ? `<span class="cb-bar ${a.bar.warn ? "warn" : ""}"><i><b style="width:${a.bar.pct}%"></b></i><em>${esc(a.bar.label)}</em></span>` : "";
    return `<button class="cb-arow" data-act="cbday" data-iso="${a.iso}"><span class="cb-ad"><b>${a.d}</b><small>${esc(a.m)}</small></span><span class="cb-ac"><b>${esc(a.t)}</b>${a.s ? `<span>${esc(a.s)}</span>` : ""}${bar}</span><span class="cb-in">${esc(a.in)}</span></button>`;
  }).join("");
  return `<section class="cb-panel cb-ahead"><div class="cb-ph2"><b>Ahead</b><span>the dates that move your year</span></div>${rows}</section>`;
}

Object.assign(CORE_MORE, {
  cbday: (el) => { const C = keep("calmonth", {}); C.day = el.dataset.iso; C.month = null; C.all = null; S.route.args = {}; refresh(el); },
  cbmonth: (el) => { const C = keep("calmonth", {}); C.day = null; C.month = el.dataset.v || null; C.all = null; S.route.args = {}; refresh(el); },
  cbview: (el) => { keep("calmonth", {}).view = el.dataset.v; refresh(el); },
  cball: (el) => { keep("calmonth", {}).all = el.dataset.iso; refresh(el); },
});
