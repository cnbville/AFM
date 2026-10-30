// Game day (section 9, docs/design/game-day.md): the game day screen
// (your staff's report on the opponent, the film room and its hints, the
// plan and your game rules, which snaps stop for you), your game called
// live snap by snap, halftime, the final whistle and every game's page.
"use strict";
(() => {
  // ---------- small helpers ----------
  const pct = (v, d = 0) => (v == null || Number.isNaN(v) ? "-" : (100 * v).toFixed(d) + "%");
  const sign = (v, d = 2) => (v == null ? "-" : (v > 0 ? "+" : v < 0 ? "−" : "") + Math.abs(v).toFixed(d));
  const one = (v) => (v == null ? "-" : Number(v).toFixed(1));
  const colorsOf = (t) => (t && t.colors ? t.colors : ["#444", "#222"]);
  // A game's room is lit by both teams (screens topic 2).
  const lightBoth = (a, b) => setColors([colorsOf(a)[0], colorsOf(b)[0]]);
  const teamGo = (t) => (t ? `data-go="team" data-kind="${t.ref.kind}" data-id="${t.ref.id}"` : "");
  const teamArgs = (t, screen) => (t ? `data-go="${screen}" data-kind="${t.ref.kind}" data-id="${t.ref.id}"` : "");
  const goLine = (links) => `<nav class="gd-goto"><span class="st-label">Go to</span>${links.filter(Boolean).slice(0, 5).map(([t, attrs]) => `<a class="link" ${attrs}>${esc(t)}</a>`).join("")}</nav>`;
  // Names in a line of play text as links: each play carries who was in it.
  function linkNames(text, who) {
    let out = esc(text);
    const seen = new Set();
    for (const [id, name] of (who || []).slice().sort((a, b) => b[1].length - a[1].length)) {
      if (!name || seen.has(name)) continue;
      seen.add(name);
      const e = esc(name);
      const i = out.indexOf(e);
      if (i < 0) continue;
      // Only the first mention links (screens topic 17, rule 7).
      out = out.slice(0, i) + playerLink(id, name) + out.slice(i + e.length);
    }
    return out;
  }
  // A hint's names as links (the hint carries who it names).
  const linkNamesIn = (text, who) => (who ? linkNames(text, who) : esc(text));
  const sure = (s) => `<span class="gd-sure ${s === "sure" ? "good" : s === "fairly sure" ? "warn" : ""}">${esc(s)}</span>`;
  const upper = (t) => {
    const x = t == null ? "" : String(t);
    return x ? x[0].toUpperCase() + x.slice(1) : "";
  };
  const qClock = (p) => `<span class="gd-qc">${esc(p.q)} ${esc(p.clock)}</span>`;
  // A spot from your side of the ball: the engine's "OWN 25" is the
  // offense's own, so on defense it is their 25.
  const rel = (s, ours) => String(s || "").replace(/\b(OWN|OPP) (\d+)/g, (_, w, n) => `${(w === "OWN") === !!ours ? "your" : "their"} ${n}`);
  // How many plays the live chart is drawn across: the plays so far over
  // the share of the game played, so the line grows left to right.
  function liveSpan(st) {
    const v = st.view;
    const last = st.wp && st.wp.length ? st.wp[st.wp.length - 1][0] : 0;
    if (st.status !== "live" || v.overtime || v.quarter > 4) return last;
    const ql = v.quarter_len || 900;
    const played = Math.min(1, Math.max(0.06, (v.quarter - 1 + (ql - v.secs) / ql) / 4));
    return Math.max(last, Math.round(last / played));
  }

  // A small win-probability chart: the series (play, home chance) drawn
  // as your team's line, with the big swings as dots that link to the play.
  function wpChart(series, mineHome, opts = {}) {
    if (!series || series.length < 2) return "";
    const W = opts.w || 900, H = opts.h || 120, pad = 6;
    const n = Math.max(series[series.length - 1][0], opts.span || 0) || 1;
    const xs = (k) => pad + ((W - 2 * pad) * k) / n;
    const ys = (v) => pad + (H - 2 * pad) * (1 - (mineHome ? v : 1 - v));
    const pts = series.map(([k, v]) => `${xs(k).toFixed(1)},${ys(v).toFixed(1)}`).join(" ");
    const area = `${xs(series[0][0]).toFixed(1)},${(H / 2).toFixed(1)} ${pts} ${xs(series[series.length - 1][0]).toFixed(1)},${(H / 2).toFixed(1)}`;
    const marks = (opts.marks || [])
      .map((m) => `<circle cx="${xs(m.n).toFixed(1)}" cy="${ys(m.wp).toFixed(1)}" r="4.5" class="gd-wp-dot ${m.good ? "good" : "bad"}" data-act="gd-play" data-n="${m.n}"><title>${esc(m.title)}</title></circle>`)
      .join("");
    const halves = (opts.halves || []).map((k) => `<line x1="${xs(k)}" x2="${xs(k)}" y1="0" y2="${H}" class="gd-wp-half"/>`).join("");
    return `<svg class="gd-wp" viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" role="img" aria-label="Win probability">
      <line x1="0" x2="${W}" y1="${H / 2}" y2="${H / 2}" class="gd-wp-mid"/>${halves}
      <polygon points="${area}" class="gd-wp-area"/><polyline points="${pts}" class="gd-wp-line"/>${marks}</svg>`;
  }

  // ======================================================================
  // The game day screen: the week before your game and its morning.
  // ======================================================================
  const TOSS_WORDS = { receive: "Receive", defer: "Defer to the second half", kick: "Kick off", goal: "Take the wind", defense_first: "Defense first", offense_first: "Offense first" };
  const SIT = [["third_down", "3rd down"], ["fourth_down", "4th down"], ["red_zone", "Red zone"], ["goal_line", "Goal line"], ["two_minute", "Two-minute drill"], ["decisions", "Toss, tries, kicks, challenges"]];

  screens.gameday = async () => {
    const h = await api("gameday");
    const tab = keep("gdtab", { t: "Report" });
    if (!h.has_game) {
      if (h.me) setColors(h.me.colors);
      return `<div class="film"><div class="pad"><div class="h1">Game day</div><p class="sub">${esc(h.why)}</p>
        ${h.calling ? callingPanel(h.calling) : ""}${h.settings ? settingsPanel(h.settings) : ""}</div></div>`;
    }
    lightBoth(h.me, h.opp);
    const at = h.neutral ? "vs" : h.home ? "vs" : "at";
    const where = h.neutral ? "Neutral site" : h.home ? "At home" : "On the road";
    const when = h.today ? `Today${h.kick ? " · kickoff " + esc(h.kick) : ""}` : `${esc(h.date)}${h.kick ? " · " + esc(h.kick) : ""} · in ${h.in} day${h.in === 1 ? "" : "s"}`;
    const live = h.live;
    let action = "";
    if (live && !live.over) action = `<button class="st-btn primary" data-go="live">Back to the game · ${esc(live.quarter)} ${esc(live.clock)}</button>`;
    else if (live && live.over) action = `<button class="st-btn primary" data-go="live">The final whistle</button>`;
    else if (h.can_kick_off) action = `<button class="st-btn primary" data-go="talk" title="The locker room before kickoff: your talk, or a captain's">Pregame talk</button><button class="st-btn glass" data-act="gd-kick" title="Kick off now; without your talk, your staff reads the room for you">Kick off</button><button class="st-btn glass" data-act="gd-delegate" title="Your staff coaches the whole game with your plan and rules; you see the result">Let the staff coach it</button>`;
    const wx = h.weather;
    const why = [h.name, ...(h.why || [])].filter(Boolean);
    const hero = `<div class="st-room gd-room"><i></i><i></i><i></i>
      <div class="gd-hero">
        <div class="st-label">Game day · ${esc(h.level)} · ${when}</div>
        <div class="gd-match">
          <a class="gd-side" ${teamGo(h.me)}>${mark(h.me, 72)}<span><b>${esc(h.me.name)}</b><small>${esc(h.records[0])}</small></span></a>
          <span class="gd-at">${at}</span>
          <a class="gd-side" ${teamGo(h.opp)}>${mark(h.opp, 72)}<span><b>${esc(h.opp.name)}</b><small>${esc(h.records[1])}</small></span></a>
        </div>
        <div class="gd-meta">${why.map((w) => `<span class="st-pill">${esc(w)}</span>`).join("")}<span>${where}</span>
          ${wx ? `<span>${esc(wx.text)}${wx.crowd ? ` · crowd ${wx.crowd}%` : ""}</span>` : `<span class="muted">Weather and the kicking line come on game morning</span>`}</div>
        ${wx ? `<div class="gd-kicking">${wx.kicking.map((k) => `<span>${esc(k)}</span>`).join("")}</div>` : ""}
        ${action ? `<div class="st-actions">${action}</div>` : h.today ? "" : `<p class="gd-note">The film room and the plan are open all week. Continue stops on the morning of the game.</p>`}
      </div></div>`;
    const tabs = ["Report", "Film room", "Game plan", "Calling"];
    if (!tabs.includes(tab.t)) tab.t = "Report";
    let body = "";
    // The deeper game plan (update 2, lane 5, screens/gameplan.js) draws
    // the week's plan and the teams like us; without it the older tabs.
    if (tab.t === "Report") body = reportTab(h) + (typeof window.dpLikeUs === "function" ? await window.dpLikeUs(h) : "");
    else if (tab.t === "Film room") body = filmTab(h);
    else if (tab.t === "Game plan") body = typeof window.dpWeekTab === "function" ? await window.dpWeekTab(h, { matchups: matchupsForm(h), fallback: planTab(h) }) : planTab(h);
    else body = `${settingsPanel(h.settings)}${callingPanel(h.calling)}`;
    const go = goLine([
      ["Their squad", teamArgs(h.opp, "roster")],
      ["Their schedule", teamArgs(h.opp, "schedule")],
      ["Your depth chart", 'data-go="depth"'],
      ["Your schedule", 'data-go="schedule"'],
      ["Standings", 'data-go="standings"'],
    ]);
    return `${hero}<div class="pad gd-pad">${go}
      <div class="tabsline">${tabs.map((t) => `<button data-act="gd-tab" data-t="${t}" aria-selected="${t === tab.t}">${t}${t === "Film room" && h.finds.length ? ` <small class="gd-count">${h.finds.length}</small>` : ""}</button>`).join("")}</div>
      ${body}</div>`;
  };

  function reportTab(h) {
    const r = h.report;
    const finds = h.finds.length
      ? `<div class="panel warm gd-finds"><h2>Your finds <small>from the tapes you studied: what the film really shows, with the count</small></h2>${h.finds.map((f) => hintRow(f, f.sources[0])).join("")}</div>`
      : "";
    const line = (l) => `<div class="gd-line"><span>${l.id != null ? linkFirst(l.text, l.id) : esc(l.text)}</span>${l.sure ? `<small>${l.n} snap${l.n === 1 ? "" : "s"} · ${sure(l.sure)}</small>` : ""}</div>`;
    const sections = (r.sections || [])
      .map((s) => `<div class="panel warm"><h2>${esc(s.title)} <small>${esc(s.unit)}</small></h2><div class="gd-lines">${s.lines.map(line).join("")}</div></div>`)
      .join("");
    const mine = h.out_mine.length ? `<div class="panel warm"><h2>Your injury report <small>${h.out_mine.length}</small></h2><div class="gd-lines">${h.out_mine.map((l) => `<div class="gd-line"><span>${linkFirst(l.text, l.id)}</span></div>`).join("")}</div></div>` : "";
    // Your two-deep's prep this week (update 2, TR43): the words, and the
    // ones your staff worries about.
    const pm = h.prep_mine;
    const prep = pm && typeof twWord === "function"
      ? `<div class="panel warm gd-prep"><h2>Your players' prep <small>the two-deep this week · <a class="link" data-go="training">Training</a></small></h2>
        <div class="gd-prepwords">${[5, 4, 3, 2, 1].map((w) => `<span>${twWord(w)} <b class="${w <= 2 && pm.words[w] ? "amber" : ""}">${pm.words[w]}</b></span>`).join("")}</div>
        ${pm.rows.length ? `<div class="gd-lines">${pm.rows.map((r) => `<div class="gd-line gd-prepline"><span>${playerLink(r.id, r.name)} <span class="muted">${esc(r.pos)} · ${esc(r.role)}</span> ${twWord(r.word)} ${twTrend(r.five)}${r.rust || r.says ? `<br><small class="${r.word <= 2 || r.rust ? "amber" : "muted"}">${r.rust ? `Rust ${esc(r.rust)}. ` : ""}${esc(r.says || "")}</small>` : ""}</span></div>`).join("")}</div>` : '<div class="sub">Everyone in the two-deep is prepared or sharp.</div>'}
        <div class="sub gd-prepnote">Poor prep shows only through the battles today: more busts and late reads, not a worse player.</div></div>`
      : "";
    return `<div class="gd-staff"><div><div class="st-label">Your staff's report on ${esc(h.opp.name)}</div>
        <div class="sub">${r.games ? `From ${r.games} game${r.games === 1 ? "" : "s"} of film, ${r.snaps} snaps tagged by ${esc(r.staff)}.${r.thin ? " Early in the season the report is thin: read every line as a small sample." : ""}` : `No film of ${esc(h.opp.name)} this season yet: they haven't played. The report fills in from their games, tagged by ${esc(r.staff)}.`}</div></div>
        <a class="link gd-side-link" data-act="gd-tab" data-t="Film room">Study their tapes yourself →</a></div>
      ${finds}
      <div class="gd-report">${sections || '<div class="panel warm"><div class="empty">No film of them yet this season. The report fills in once they play.</div></div>'}${mine}${prep}</div>`;
  }
  // A line whose subject is a player: his name (the text's start) links.
  function linkFirst(text, id) {
    const m = String(text).match(/^([^(:]+?)(\s*\(|:)/);
    if (!m || id == null) return esc(text);
    return playerLink(id, m[1]) + esc(String(text).slice(m[1].length));
  }

  function hintRow(f, src) {
    const key = `data-date="${src.date}" data-home="${src.home}" data-away="${src.away}" data-key="${esc(f.key)}"`;
    const btn = !f.plan
      ? `<span class="gd-eyes">For your eyes</span>`
      : f.added
        ? `<button class="st-btn small glass gd-in" data-act="gd-hint" data-add="0" ${key} title="Take it out of the plan">In the plan ✓</button>`
        : `<button class="st-btn small glass" data-act="gd-hint" data-add="1" ${key}>Add to plan</button>`;
    return `<div class="gd-hint ${f.added ? "added" : ""}"><div class="gd-hint-k"><span class="gd-kind">${esc(f.kind)}</span><small>${esc(f.unit)}</small></div>
      <div><b>${linkNamesIn(f.text, f.who)}</b><div class="gd-tip">${esc(f.tip)}</div><div class="gd-effect">${f.plan ? "In the plan: " : ""}${esc(f.effect)}</div></div>
      <div class="gd-hint-n"><b>${f.hit}/${f.of}</b><small>${f.tapes > 1 ? f.tapes + " tapes" : f.unit === "their kicking game" ? "kicks" : "snaps"}</small></div>${btn}</div>`;
  }

  function filmTab(h) {
    const f = h.film;
    const left = Math.max(0, f.budget - f.used);
    const hrs = (x) => one(x).replace(/\.0$/, "");
    const slots = Math.max(1, Math.floor(f.budget / f.per_tape + 1e-6));
    const done = Math.min(slots, Math.round(f.used / f.per_tape));
    const meter = `<div class="panel warm gd-time"><div><div class="st-label">Your film time this week</div><div class="gd-hours"><b>${hrs(left)}</b> hour${left === 1 ? "" : "s"} left of ${hrs(f.budget)}</div>
      <div class="sub">A tape takes about ${f.per_tape} hours to study properly. Each one studied gives its strongest few hints at once, the same whatever your staff's rating; a habit seen on several tapes becomes one stronger hint.</div></div>
      <div class="gd-slots" title="${done} of ${slots} tapes' worth studied">${Array.from({ length: slots }, (_, i) => `<i class="${i < done ? "on" : ""}"><span>${i < done ? "Studied" : `${f.per_tape} h`}</span></i>`).join("")}</div></div>`;
    const tapes = h.tapes.length
      ? h.tapes
          .map((t) => {
            const key = `data-date="${t.key.date}" data-home="${t.key.home}" data-away="${t.key.away}"`;
            const action = t.studied
              ? `<span class="gd-studied">Studied</span>`
              : left >= f.per_tape
                ? `<button class="st-btn small primary" data-act="gd-study" ${key}>Study · ${f.per_tape} h</button>`
                : `<button class="st-btn small glass" disabled title="Your film time this week is spent">No film time left</button>`;
            return `<div class="panel warm gd-tape ${t.studied ? "done" : ""}"><div class="gd-tape-head">
              <span class="gd-tape-date">${esc(t.date)}</span>
              <span class="gd-tape-vs"><span class="muted">${esc(t.home ? "vs" : "at")}</span>${mark(t.vs_ref, 22)}<a class="link name" ${teamGo(t.vs_ref)}>${esc(t.vs)}</a></span>
              <span class="gd-tape-score">${esc(t.score)}</span><span class="muted">${t.snaps} snaps</span>${action}</div>
              ${t.studied ? (t.hints.length ? t.hints.map((x) => hintRow(x, t.key)).join("") : '<div class="empty">Nothing on this tape stood out: no habit showed often enough to trust.</div>') : ""}</div>`;
          })
          .join("")
      : `<div class="panel warm"><div class="empty">No tape of ${esc(h.opp.name)} yet this season. Their games reach your film room as they play them.</div></div>`;
    return `${meter}${tapes}`;
  }

  function pickSelect(name, cur, options, staffWords) {
    const opt = (v, t) => `<option value="${v}" ${String(cur) === String(v) ? "selected" : ""}>${esc(t)}</option>`;
    return `<select class="gd-select" data-act="gd-plan" data-k="${name}">${opt("staff", staffWords)}${opt("none", "Nobody: play it straight")}${options.map((o) => opt(o.id, `${o.name} (${o.pos})${o.seen ? ` · on tape ${o.seen} snap${o.seen === 1 ? "" : "s"}` : ""}`)).join("")}</select>`;
  }
  function triSeg(name, cur, words) {
    const v = cur == null ? "staff" : cur ? "yes" : "no";
    return `<div class="seg">${[["staff", "Staff decides"], ["yes", words[0]], ["no", words[1]]].map(([k, t]) => `<button data-act="gd-plan" data-k="${name}" data-v="${k}" aria-pressed="${v === k}">${t}</button>`).join("")}</div>`;
  }
  // The week's matchups against this opponent: whose corner to throw at,
  // the bracket, the spy, the kicks and the draws.
  function matchupsForm(h) {
    const p = h.plan;
    const o = p.options;
    return `<div class="gd-form">
          <label>Throw at their corner<small>your play caller looks his way first</small></label>${pickSelect("target_cb", p.target_cb, o.corners, "Staff decides")}
          <label>Bracket their receiver<small>two men on him, someone short elsewhere</small></label>${pickSelect("bracket", p.bracket, o.receivers, "Staff decides")}
          <label>Spy their quarterback<small>a linebacker mirrors him instead of rushing or dropping</small></label>${triSeg("spy", p.spy, ["Spy him", "No spy"])}
          <label>Kick away from their returner<small>shorter, safer kicks: field position for the return</small></label>${triSeg("kick_away", p.kick_away, ["Kick away", "Kick deep to him"])}
          <label>Draws and screens<small>more of them against edge rushers who win upfield</small></label>${triSeg("draws", p.draws, ["More of them", "As usual"])}
        </div>
        <div class="sub">${p.hints ? `${p.hints} hint${p.hints === 1 ? "" : "s"} from your film study in the plan: your players key those habits from the snaps that show them. ` : ""}The plan holds whether you call the game or your staff does.</div>`;
  }
  function planTab(h) {
    const r = h.rules;
    const seg = (k, cur, opts) => `<div class="seg">${opts.map(([v, t]) => `<button data-act="gd-rule" data-k="${k}" data-v="${v}" aria-pressed="${cur === v}">${t}</button>`).join("")}</div>`;
    const fourth = { staff: "Your staff decides each one by its own lean and the game.", numbers: "Go for it whenever the model says going wins more often than punting or kicking.", aggressive: "Go for it unless kicking is clearly better (two points of win chance).", conservative: "Kick unless going is clearly better (four points of win chance)." };
    const two = { staff: "Your staff decides by its chart and its trust in the kicker.", chart: "Dick Vermeil's chart in the 4th quarter: go for two when the margin after the touchdown says so (down 2, 5, 9, 10, 13, 17 or 18; up 1, 4, 5, 12, 15 or 19).", kick: "Always kick the extra point." };
    return `<div class="grid2"><div class="panel warm"><h2>Matchups <small>the week's plan against ${esc(h.opp.name)}</small></h2>${matchupsForm(h)}</div>
      <div class="panel warm"><h2>Your game rules <small>they decide when no snap stops for you</small></h2>
        <div class="gd-form stack">
          <label>4th downs<small>${esc(fourth[r.fourth])}</small></label>${seg("fourth", r.fourth, [["staff", "Staff"], ["numbers", "The numbers"], ["aggressive", "Aggressive"], ["conservative", "Conservative"]])}
          <label>Two-point tries<small>${esc(two[r.two])}</small></label>${seg("two", r.two, [["staff", "Staff"], ["chart", "The chart"], ["kick", "Always kick"]])}
          <label>Tempo<small>${r.tempo === "hurry" ? "No-huddle: more snaps, less time for their defense to substitute and disguise." : r.tempo === "milk" ? "Use the play clock: fewer snaps, the game shorter." : "Huddle and snap as the play caller likes."}</small></label>${seg("tempo", r.tempo, [["normal", "Normal"], ["hurry", "No-huddle"], ["milk", "Milk the clock"]])}
          <label>Timeouts<small>${r.timeouts === "keep_two" ? "Keep two for the end of each half." : r.timeouts === "keep_one" ? "Keep one for the end of each half." : r.timeouts === "free" ? "Use them when the snap needs it." : "Your staff spends them as the game goes."}</small></label>${seg("timeouts", r.timeouts, [["staff", "Staff"], ["keep_two", "Keep two"], ["keep_one", "Keep one"], ["free", "Freely"]])}
          <label>Four-minute plan<small>${r.four === "run" ? "Ahead by one score late: run it out." : r.four === "attack" ? "Keep attacking with the lead." : "Your staff decides with the lead and the clock."}</small></label>${seg("four", r.four, [["staff", "Staff"], ["run", "Run it out"], ["attack", "Keep attacking"]])}
        </div>${r.changed ? `<div class="sub">Changed for this game only: the standing rules on the Playbook come back after it. <a class="link" data-act="gd-rulereset">Back to the standing rules</a></div>` : ""}</div></div>`;
  }

  function settingsPanel(s) {
    const seg = (k, cur, opts) => `<div class="seg">${opts.map(([v, t]) => `<button data-act="gd-set" data-k="${k}" data-v="${v}" aria-pressed="${cur === v}">${t}</button>`).join("")}</div>`;
    const sits = SIT.map(([k, t]) => `<label class="gd-check"><input type="checkbox" data-act="gd-sit" data-k="${k}" ${s.situations[k] ? "checked" : ""} ${s.stop !== "situations" ? "disabled" : ""}> ${t}</label>`).join("");
    return `<div class="panel warm"><h2>How you call it <small>change it between any two snaps</small></h2>
      <div class="gd-form">
        <label>Which snaps stop for you<small>${s.stop === "every" ? "Every snap stops: calling plays yourself is how you play and learn. After each one you see your call beside your play caller's." : s.stop === "off" ? "None: your staff calls the game with your plan and rules, and you watch." : "The situations you pick stop; any other snap you can take over with N."}</small></label>
        ${seg("stop", s.stop, [["situations", "Your situations"], ["every", "Every snap"], ["off", "None"]])}
        <label>Situations<small>with "Your situations"</small></label><div class="gd-checks">${sits}</div>
        <label>Time for a call<small>${s.clock === "realism" ? "Realism: 25 seconds, the time between the play clock starting and the radio cutting out; then your play caller's call goes in." : "Learning: the game waits as long as you want."}</small></label>
        ${seg("clock", s.clock, [["realism", "Realism · 25 s"], ["learning", "Learning · no clock"]])}
      </div></div>`;
  }

  function callingPanel(c) {
    if (!c) return "";
    const row = (t, x) => `<tr><td class="l">${t}</td><td>${x.snaps}</td><td>${x.snaps ? sign(x.epa) : "–"}</td><td>${x.snaps ? `${x.success.toFixed(0)}%` : "–"}</td></tr>`;
    return `<div class="panel warm"><h2>Your play calling this season <small>${c.games} game${c.games === 1 ? "" : "s"} with your own calls</small></h2>
      <div class="tbl"><table><thead><tr><th class="l">Snaps called by</th><th>Snaps</th><th>EPA a play</th><th>Success</th></tr></thead><tbody>${row("You", c.yours)}${row("Your staff", c.staff)}</tbody></table></div>
      <div class="sub">EPA counts for your team on both sides of the ball. A success is 40% of the distance on 1st down, 60% on 2nd and all of it on 3rd and 4th.</div></div>`;
  }

  // ---------- the game day screen's actions ----------
  async function gdDo(cmd, args) {
    try {
      await api(cmd, args);
      render($(".view") && $(".view").scrollTop);
    } catch (e) {
      toast(e.message);
    }
  }
  const tapeKey = (d) => ({ date: Number(d.date), home: Number(d.home), away: Number(d.away) });
  document.addEventListener("click", async (ev) => {
    const el = ev.target.closest("[data-act^='gd-'],[data-act^='lv-'],[data-act^='gp-']");
    if (!el) return;
    const d = el.dataset;
    switch (d.act) {
      case "gd-tab": S.keep.gdtab = { t: d.t }; render(); break;
      case "gd-study": gdDo("gameday_study", tapeKey(d)); break;
      case "gd-hint": gdDo("gameday_hint", { ...tapeKey(d), key: d.key, add: d.add === "1" }); break;
      case "gd-rule": gdDo("gameday_rules", { [d.k]: d.v }); break;
      case "gd-rulereset": gdDo("gameday_rules", { reset: true }); break;
      case "gd-set": gdDo("gameday_settings", { [d.k]: d.v }); break;
      case "gd-plan": if (d.v) gdDo("gameday_plan", { [d.k]: d.v === "yes" ? true : d.v === "no" ? false : "staff" }); break;
      case "gd-kick": kickoff(); break;
      case "gd-delegate": {
        if (!(await sure("Hand the whole game to your staff?", "They coach it with your plan, your film study and your game rules. You see the result and the game page.", "Hand it over"))) break;
        cont();
        break;
      }
      default: if (live[d.act]) live[d.act](d, el, ev);
    }
  });
  document.addEventListener("change", (ev) => {
    const d = ev.target.dataset;
    if (d.act === "gd-plan" && ev.target.tagName === "SELECT") {
      const v = ev.target.value;
      gdDo("gameday_plan", { [d.k]: v === "staff" || v === "none" ? v : Number(v) });
    }
    if (d.act === "gd-sit") {
      const all = {};
      document.querySelectorAll("[data-act=gd-sit]").forEach((x) => (all[x.dataset.k] = x.checked));
      gdDo("gameday_settings", { situations: all });
    }
    if (d.act === "lv-set") live.setStop(ev.target.value);
  });

  async function kickoff() {
    showBusy("Kickoff…");
    try {
      LV.state = await api("gameday_kickoff");
      LV.seen = 0;
      hideBusy();
      go("live");
    } catch (e) {
      hideBusy();
      toast(e.message);
    }
  }

  // ======================================================================
  // The live game.
  // ======================================================================
  // The state of the screen: the last answer from the game, how many plays
  // the feed has shown, the sheet picker, the realism clock and watching.
  const LV = { state: null, seen: 0, sheet: null, timer: null, deadline: 0, watch: null, busy: false, fourth: null, two: null };

  screens.live = async () => {
    const st = LV.state && LV.state.status === "live" && LV.fresh ? LV.state : await api("live");
    LV.fresh = false;
    LV.state = st;
    if (st.status === "none") return `<div class="film"><div class="pad"><div class="h1">No game is being played</div><p class="sub">Your games are called from the game day screen on their morning.</p><div><button class="st-btn glass" data-go="gameday">Game day</button></div></div></div>`;
    lightBoth(st.home, st.away);
    const html = st.status === "final" ? finalScreen(st) : liveScreen(st);
    setTimeout(() => live.after(st), 0);
    return html;
  };

  function scoreboard(st) {
    const v = st.view;
    const p = st.pending;
    const side = (k) => {
      const t = k === 0 ? st.home : st.away;
      const to = v.timeouts[k];
      const poss = v.poss === k && st.status === "live" && !(p && (p.ask === "toss" || p.ask === "halftime"));
      return `<div class="lv-team ${k === st.mine ? "mine" : ""}">
        <a ${teamGo(t)} class="lv-name">${mark(t, 54)}<span><b>${esc(st.names[k])}</b><small>${[0, 1, 2].map((i) => `<i class="lv-to ${i < to ? "on" : ""}"></i>`).join("")}${v.challenges && (v.challenges[0] || v.challenges[1]) ? ` <span class="lv-chal" title="Challenges left">⚑${v.challenges[k]}</span>` : ""}</small></span></a>
        <span class="lv-pts">${v.score[k]}</span><span class="lv-ball ${poss ? "on" : ""}" title="${poss ? "Ball" : ""}">●</span></div>`;
    };
    const wpMine = v.wp_mine;
    const ask = p && p.ask;
    const where = rel(v.spot, v.ours).toUpperCase();
    const dd =
      st.status === "final" ? "Final" + (v.overtime ? " (OT)" : "")
      : ask === "toss" ? "Coin toss"
      : ask === "onside" ? "Kickoff"
      : ask === "halftime" ? "Halftime"
      : ask === "pat" ? `Try · ${esc(where)}`
      : `${esc(v.dd)} · ${esc(where)}`;
    return `<div class="lv-board">
      ${side(1)}
      <div class="lv-mid"><div class="lv-clock"><b>${esc(v.q)}</b> <span>${esc(v.clock)}</span></div><div class="lv-dd">${dd}</div>
        <div class="lv-wp" title="Your win chance: the model's, from the score, the ball and the time left"><span>${esc(st.names[st.mine])}</span><div class="lv-wpbar"><i style="width:${(100 * wpMine).toFixed(1)}%"></i></div><b>${pct(wpMine)}</b></div>
      </div>
      ${side(0)}</div>
      <div class="lv-wpline">${wpChart(st.wp, st.mine === 0, { h: 56, halves: [], span: liveSpan(st) })}</div>`;
  }

  // The field: the home team's end zone on the left all game, the ball,
  // the line to gain and the drive so far.
  function fieldStrip(st) {
    const v = st.view;
    const ask = st.pending && st.pending.ask;
    const showBall = st.status === "live" && !["toss", "onside", "halftime"].includes(ask);
    const W = 1000, H = 64, ez = 70, fy = (W - 2 * ez) / 100;
    const fx = (x) => ez + fy * (v.poss === 0 ? x : 100 - x);
    const ball = fx(v.x);
    const gain = v.to_goal <= v.togo + 0.01 ? null : fx(Math.min(100, v.x + v.togo));
    // The drive: from its first snap to the ball.
    const plays = st.plays || [];
    let start = null;
    for (let i = plays.length - 1; i >= 0; i--) {
      const q = plays[i];
      if (q.off !== st.names[v.poss] || q.kind === "Kickoff" || q.kind === "ExtraPoint" || q.kind === "TwoPoint" || q.kind === "Punt") break;
      if (q.dd) start = q.x;
    }
    const lines = [];
    for (let y = 10; y < 100; y += 10) lines.push(`<line x1="${ez + fy * y}" x2="${ez + fy * y}" y1="6" y2="${H - 6}" class="lv-yl"/><text x="${ez + fy * y}" y="${H - 9}" class="lv-num">${y <= 50 ? y : 100 - y}</text>`);
    const [hc] = colorsOf(st.home), [ac] = colorsOf(st.away);
    const drive = start != null && showBall ? `<rect x="${Math.min(fx(start), ball)}" y="14" width="${Math.abs(ball - fx(start))}" height="${H - 28}" class="lv-drive"/>` : "";
    return `<svg class="lv-field" viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" aria-label="The field">
      <rect x="0" y="0" width="${ez}" height="${H}" fill="${hc}" opacity=".55"/><rect x="${W - ez}" y="0" width="${ez}" height="${H}" fill="${ac}" opacity=".55"/>
      <text x="${ez / 2}" y="${H / 2 + 4}" class="lv-ez">${esc(initials(st.names[0]))}</text><text x="${W - ez / 2}" y="${H / 2 + 4}" class="lv-ez">${esc(initials(st.names[1]))}</text>
      ${lines.join("")}${drive}
      ${showBall ? `<line x1="${ball}" x2="${ball}" y1="0" y2="${H}" class="lv-los"/>${gain != null && ask !== "pat" ? `<line x1="${gain}" x2="${gain}" y1="0" y2="${H}" class="lv-gain"/>` : ""}<circle cx="${ball}" cy="${H / 2}" r="6" class="lv-pball"/>` : ""}
    </svg>`;
  }

  function feedRow(p, st, fresh, delay) {
    const yours = p.yours && ((p.ours && p.yours & 1) || (!p.ours && p.yours & 2));
    const cls = [p.key ? "key" : "", p.td ? "td" : "", p.turnover ? "to" : "", p.ours ? "ours" : "theirs", fresh ? "fresh" : ""].join(" ");
    const extra = [
      p.officiating ? `<div class="lv-ref">${esc(p.officiating)}</div>` : "",
      ...(p.injuries || []).map((i) => `<div class="lv-hurt">${esc(i)}</div>`),
      yours && p.staff_call ? `<div class="lv-cmp">Your call: ${esc(p.ours ? p.call : p.defense)}${p.off_sheet ? " (off the sheet)" : ""} · your staff's: ${esc(p.staff_call)}</div>` : "",
      !yours && st.view && p.ours && p.call ? `<div class="lv-cmp">${esc(p.call)}${p.why ? " · " + esc(p.why) : ""}</div>` : "",
    ].join("");
    return `<li class="${cls}" style="${delay ? `animation-delay:${delay}ms` : ""}"><span class="lv-fq">${esc(p.q)} ${esc(p.clock)}</span>
      <div><div class="lv-ftext">${p.dd ? `<span class="lv-fdd">${esc(rel(p.dd, p.ours))}</span> ` : ""}${linkNames(p.text, p.who)}${yours ? ' <span class="lv-yours">Your call</span>' : ""}</div>${extra}</div>
      <span class="lv-fe ${p.epa > 0 ? "up" : p.epa < 0 ? "down" : ""}" title="Expected points added for ${esc(p.off)}">${p.kind === "Kickoff" || p.kind === "ExtraPoint" ? "" : sign(p.epa, 1)}</span></li>`;
  }

  function feed(st) {
    const plays = st.plays || [];
    const fresh = Math.max(0, plays.length - LV.seen);
    const logs = {};
    for (const l of st.log || []) (logs[l.n] ??= []).push(l);
    const rows = [];
    // Newest first; the snaps since your last look rise in one by one.
    for (let i = plays.length - 1; i >= Math.max(0, plays.length - 60); i--) {
      const p = plays[i];
      const isNew = i >= plays.length - fresh;
      const order = isNew ? i - (plays.length - fresh) : 0;
      for (const l of logs[p.n + 1] || []) rows.push(`<li class="log"><span></span><div>${esc(l.text)}</div><span></span></li>`);
      rows.push(feedRow(p, st, isNew, isNew ? Math.min(order, 12) * 110 : 0));
    }
    return `<ul class="lv-feed">${rows.join("")}</ul>`;
  }

  function teamLine(st) {
    const t = st.totals;
    const r = (k, f) => `<tr><td class="l">${k}</td><td>${f(t[1])}</td><td>${f(t[0])}</td></tr>`;
    return `<div class="tbl lv-totals"><table><thead><tr><th class="l"></th><th>${esc(initials(st.names[1]))}</th><th>${esc(initials(st.names[0]))}</th></tr></thead><tbody>
      ${r("Yards", (x) => x.yards)}${r("Pass", (x) => x.pass)}${r("Rush", (x) => x.rush)}${r("1st downs", (x) => x.first_downs)}${r("3rd downs", (x) => x.third)}${r("Turnovers", (x) => x.turnovers)}${r("Sacks", (x) => x.sacks)}${r("Penalties", (x) => x.penalties)}${r("Possession", (x) => x.top)}</tbody></table></div>`;
  }

  // ---------- the decision panel ----------
  function callCard(c, i, kind) {
    const today = c.today ? `<small class="lv-today">${c.today.n} today, ${c.today.yds.toFixed(1)} yds a play</small>` : "";
    const key = i < 9 ? `<kbd>${i + 1}</kbd>` : "";
    if (kind === "off") {
      return `<button class="lv-call ${c.staff ? "staff" : ""}" data-act="lv-call" data-i="${i}">${key}<span><b>${esc(c.play)}</b><small>${esc(c.look)} · ${esc(c.family)}</small>${c.staff && c.why ? `<em>${esc(c.why)}</em>` : ""}${today}</span>${c.staff ? '<span class="lv-tag">Staff\'s call</span>' : ""}</button>`;
    }
    return `<button class="lv-call ${c.staff ? "staff" : ""}" data-act="lv-def" data-i="${i}">${key}<span><b>${esc(c.coverage)}, ${esc(c.pressure)}</b><small>${esc(c.personnel)} · ${esc(c.front)} front${c.stunt && c.stunt !== "none" ? " · " + esc(c.stunt) : ""}${c.spy ? " · QB spy" : ""}${c.disguise ? " · disguised" : ""}</small>${today}</span>${c.staff ? '<span class="lv-tag">Staff\'s call</span>' : ""}</button>`;
  }

  function decision(st) {
    const p = st.pending;
    const v = p.view;
    const mine = st.mine;
    const advice = (p.advice || []).map((a) => `<div class="lv-advice">${esc(a)}</div>`).join("");
    const ruling = p.ruling ? `<div class="lv-ruling">${esc(p.ruling)}</div>` : "";
    const clock = st.settings.clock === "realism" && !["halftime", "toss"].includes(p.ask) && !LV.watch ? `<div class="lv-timer"><b id="lv-secs">25</b><div class="st-bar"><i id="lv-tbar" style="width:100%"></i></div></div>` : "";
    let head = "", body = "", foot = "";
    const sideline = () => {
      const them = st.their_players || [];
      const opts = (pos) => them.filter((x) => pos.includes(x.pos)).map((x) => `<option value="${x.id}">${esc(x.name)} (${esc(x.pos)})${x.today ? ` · ${x.today} today` : ""}</option>`).join("");
      const ours = v.poss === mine;
      return `<details class="lv-side"><summary>Sideline <small>timeouts ${v.timeouts[mine]} · tempo ${esc(v.tempo)} · a word to a coordinator</small></summary><div class="lv-side-body">
        <div class="lv-row"><button class="st-btn small glass" data-act="lv-timeout" ${v.timeouts[mine] ? "" : "disabled"}><kbd>T</kbd> Timeout</button>
        ${ours ? `<div class="seg">${[["normal", "Normal"], ["hurry", "No-huddle"], ["milk", "Milk the clock"]].map(([k, t]) => `<button data-act="lv-tempo" data-v="${k}" aria-pressed="${v.tempo === k}">${t}</button>`).join("")}</div>` : ""}</div>
        <div class="gd-form small">
          <label>Throw at</label><select data-act="lv-word" data-k="target_cb"><option value="">Your coordinator's choice</option><option value="none">Nobody in particular</option>${opts(["CB"])}</select>
          <label>Bracket</label><select data-act="lv-word" data-k="bracket"><option value="">Your coordinator's choice</option><option value="none">Nobody</option>${opts(["WR", "TE"])}</select>
          <label>Their QB</label><div class="seg"><button data-act="lv-wordb" data-k="spy" data-v="1">Spy him</button><button data-act="lv-wordb" data-k="spy" data-v="0">No spy</button></div>
          <label>Kicking</label><div class="seg"><button data-act="lv-wordb" data-k="kick_away" data-v="1">Away from the returner</button><button data-act="lv-wordb" data-k="kick_away" data-v="0">Kick deep</button></div>
          <label>Their rush</label><div class="seg"><button data-act="lv-wordb" data-k="draws" data-v="1">Draws and screens</button><button data-act="lv-wordb" data-k="draws" data-v="0">As usual</button></div>
        </div></div></details>`;
    };
    if (p.ask === "toss") {
      head = `<div class="lv-ask"><span class="st-label">The coin toss</span><h2>${v.overtime ? "Overtime: you won the toss" : "You won the toss"}</h2></div>`;
      body = `<div class="lv-choices">${p.toss.map((k, i) => `<button class="lv-choice ${p.staff_toss === k ? "staff" : ""}" data-act="lv-toss" data-v="${k}"><kbd>${i + 1}</kbd><b>${TOSS_WORDS[k] || k}</b>${p.staff_toss === k ? "<small>Your staff's choice</small>" : ""}</button>`).join("")}</div>
        <div class="lv-advice">${esc(st.weather.text)}. ${esc(st.weather.kicking.join(" · "))}.${(p.advice || []).map((x) => " " + esc(x)).join("")}</div>`;
    } else if (p.ask === "halftime") {
      return halftimePanel(st);
    } else if (p.ask === "challenge") {
      head = `<div class="lv-ask"><span class="st-label">Replay</span><h2>Challenge the call?</h2></div>`;
      body = `${ruling}<div class="lv-advice">${p.staff_challenge ? "Your replay assistant upstairs: “Throw it.”" : "Your replay assistant upstairs: “Don't. It stands.”"} A lost challenge costs a timeout. You have ${v.challenges ? v.challenges[mine] : 0} left.</div>
        <div class="lv-choices"><button class="lv-choice ${p.staff_challenge ? "staff" : ""}" data-act="lv-chal" data-v="1"><kbd>1</kbd><b>Throw the flag</b></button><button class="lv-choice ${!p.staff_challenge ? "staff" : ""}" data-act="lv-chal" data-v="0"><kbd>2</kbd><b>Let it go</b></button></div>`;
    } else if (p.ask === "onside") {
      head = `<div class="lv-ask"><span class="st-label">Kickoff</span><h2>Onside or deep?</h2></div>`;
      body = `${advice}<div class="lv-choices"><button class="lv-choice ${!p.staff_onside ? "staff" : ""}" data-act="lv-onside" data-v="0"><kbd>1</kbd><b>Kick deep</b>${!p.staff_onside ? "<small>Your staff's choice</small>" : ""}</button><button class="lv-choice ${p.staff_onside ? "staff" : ""}" data-act="lv-onside" data-v="1"><kbd>2</kbd><b>Onside kick</b>${p.staff_onside ? "<small>Your staff's choice</small>" : ""}</button></div>`;
    } else if (p.ask === "pat") {
      const two = LV.two;
      head = `<div class="lv-ask"><span class="st-label">After the touchdown · ${v.score[mine]}-${v.score[1 - mine]}</span><h2>Kick or go for two?</h2></div>`;
      body = `${advice}<div class="lv-choices"><button class="lv-choice ${!p.staff_two ? "staff" : ""}" data-act="lv-pat" data-v="0"><kbd>K</kbd><b>Kick the extra point</b>${!p.staff_two ? "<small>Your staff's choice</small>" : ""}</button><button class="lv-choice ${p.staff_two ? "staff" : ""} ${two ? "on" : ""}" data-act="lv-pat" data-v="1"><kbd>G</kbd><b>Go for two</b>${p.staff_two ? "<small>Your staff's choice</small>" : ""}</button></div>
        ${two ? `<div class="lv-calls">${p.offense.map((c, i) => callCard(c, i, "off")).join("")}<button class="lv-call sheet" data-act="lv-sheet"><kbd>F</kbd><span><b>The full sheet…</b><small>any formation and play</small></span></button></div>` : ""}`;
    } else if (p.ask === "defense") {
      head = `<div class="lv-ask"><span class="st-label">${esc(v.q)} ${esc(v.clock)} · your defense</span><h2>${esc(v.dd)} <small>at ${esc(rel(v.spot, false))}</small></h2>${p.their_personnel ? `<div class="lv-pers">They sent on <b>${esc(p.their_personnel)}</b> personnel</div>` : ""}</div>`;
      body = `${advice}<div class="lv-calls">${p.defense.map((c, i) => callCard(c, i, "def")).join("")}<button class="lv-call sheet" data-act="lv-sheet"><kbd>F</kbd><span><b>The full sheet…</b><small>any package, coverage, pressure and front</small></span></button></div>`;
      foot = sideline();
    } else {
      // Your offense's snap, and 4th downs.
      const fourth = v.down === 4 || p.staff_fourth;
      const staffF = p.staff_fourth || "go";
      const choose = LV.fourth || (p.staff_fourth ? null : "go");
      head = `<div class="lv-ask"><span class="st-label">${esc(v.q)} ${esc(v.clock)} · your offense</span><h2>${esc(v.dd)} <small>at ${esc(rel(v.spot, true))}</small></h2></div>`;
      let fourthRow = "";
      if (fourth) {
        const w = p.fourth_wp;
        const opt = (k, key, t, extra) => `<button class="lv-choice ${staffF === k ? "staff" : ""} ${choose === k ? "on" : ""}" data-act="lv-fourth" data-v="${k}"><kbd>${key}</kbd><b>${t}</b>${extra}${staffF === k ? "<small>Your staff's call</small>" : ""}</button>`;
        fourthRow = `<div class="lv-choices">${opt("go", "G", "Go for it", w ? `<span class="lv-wpn">${pct(w[0])}</span>` : "")}${opt("punt", "P", "Punt", w ? `<span class="lv-wpn">${pct(w[1])}</span>` : "")}${p.fg_make != null ? opt("fg", "K", "Field goal", `<span class="lv-wpn">${w && w[2] ? pct(w[2]) : ""}</span><small>make about ${pct(p.fg_make)}</small>`) : ""}</div>`;
      }
      const showCalls = !fourth || choose === "go";
      body = `${advice}${fourthRow}${showCalls && p.offense.length ? `<div class="lv-calls">${p.offense.map((c, i) => callCard(c, i, "off")).join("")}<button class="lv-call sheet" data-act="lv-sheet"><kbd>F</kbd><span><b>The full sheet…</b><small>any formation and play; off the week's sheet it's less practiced</small></span></button></div>` : ""}
        ${fourth && choose && choose !== "go" ? `<div class="lv-row"><button class="st-btn primary" data-act="lv-fourth-go">${choose === "punt" ? "Send the punt team" : "Send the kicking team"} <kbd>↵</kbd></button></div>` : ""}`;
      foot = sideline();
    }
    const word = typeof tkWordCard === "function" ? tkWordCard(st) : "";
    return `<div class="panel warm lv-decide">${clock}${word}${head}${ruling && p.ask !== "challenge" ? ruling : ""}${body}
      <div class="lv-row lv-keys"><button class="st-btn small glass" data-act="lv-staff" title="Your staff's call on this snap"><kbd>↵</kbd> Go with the staff</button>
      <button class="st-btn small glass" data-act="lv-watch" title="Your staff calls the snaps and you watch, stepping in at any one"><kbd>W</kbd> Watch</button>
      <button class="st-btn small glass" data-act="lv-sim" title="Your staff coaches the rest of the game at once"><kbd>E</kbd> Staff to the end</button></div>${foot}</div>`;
  }

  function halftimePanel(st) {
    const h = st.halftime;
    const unit = (u, who) => `<div class="lv-half-u"><div class="st-label">${who}</div><div class="lv-half-n"><b>${u.yards}</b><small>${u.yards === 1 ? "yard" : "yards"} on ${u.plays} play${u.plays === 1 ? "" : "s"}</small></div>
      <div class="sub">Runs ${u.run.n} at ${u.run.ypc.toFixed(1)} a carry (${sign(u.run.epa)} EPA) · dropbacks ${u.pass.n} (${sign(u.pass.epa)} EPA, ${u.pass.sacks} sacked) · success ${u.success.toFixed(0)}%</div></div>`;
    const adj = (h.adjust || [])
      .map((a, i) => `<label class="lv-adj"><input type="checkbox" class="lv-adj-on" data-i="${i}" checked><span><b>${esc(a.who)}</b>${esc(a.text)}</span></label>`)
      .join("");
    return `<div class="panel warm lv-decide lv-half"><div class="lv-ask"><span class="st-label">Halftime · ${esc(st.names[1])} ${h.score[1]}, ${esc(st.names[0])} ${h.score[0]}</span><h2>The locker room</h2></div>
      <div class="grid2">${unit(h.us, "Your offense")}${unit(h.them, "Their offense")}</div>
      ${h.works.length ? `<div><div class="st-label">What works</div>${h.works.map((x) => `<div class="lv-advice">${esc(upper(x))}</div>`).join("")}</div>` : ""}
      ${h.fails.length ? `<div><div class="st-label">What doesn't</div>${h.fails.map((x) => `<div class="lv-advice">${esc(upper(x))}</div>`).join("")}</div>` : ""}
      ${adj ? `<div><div class="st-label">Your coordinators' adjustments</div>${adj}</div>` : `<div class="lv-advice">Your coordinators want no change: the plan holds.</div>`}
      ${h.hurt.length ? `<div><div class="st-label">The trainers</div>${h.hurt.map((x) => `<div class="lv-hurt">${esc(x)}</div>`).join("")}</div>` : ""}
      ${typeof tkHalfBlock === "function" ? tkHalfBlock(st) : ""}
      <div class="lv-row"><button class="st-btn primary" data-act="lv-half">Start the second half <kbd>↵</kbd></button></div></div>`;
  }

  function sheetPanel(st) {
    const sh = st.sheet;
    const p = st.pending;
    const s = LV.sheet;
    const sel = (k, list, label) => `<label>${label}</label><select data-act="lv-sheetv" data-k="${k}">${list.map((x) => { const v = typeof x === "string" ? x : x.name; return `<option value="${esc(v)}" ${s[k] === v ? "selected" : ""}>${esc(upper(v))}</option>`; }).join("")}</select>`;
    if (p.ask === "defense") {
      return `<div class="overlay lv-sheet-o"><div class="box lv-sheet"><b>Your defensive call</b>
        <div class="gd-form small">${sel("personnel", sh.def_personnel, "Package")}${sel("coverage", sh.coverages, "Coverage")}${sel("pressure", sh.pressures, "Pressure")}${sel("front", sh.fronts, "Front")}${sel("stunt", sh.stunts, "Stunt")}
        <label>Disguise</label><select data-act="lv-sheetv" data-k="disguise"><option value="">Show it</option><option value="1" ${s.disguise ? "selected" : ""}>Disguise it</option></select>
        <label>QB spy</label><select data-act="lv-sheetv" data-k="spy"><option value="">No</option><option value="1" ${s.spy ? "selected" : ""}>Yes</option></select></div>
        <div class="ask-actions"><button class="st-btn glass" data-act="lv-sheet-x">Cancel</button><button class="st-btn primary" data-act="lv-sheet-go">Call it</button></div></div></div>`;
    }
    const plays = s.kind === "run" ? sh.runs : s.kind === "rpo" ? sh.rpos : sh.passes;
    return `<div class="overlay lv-sheet-o"><div class="box lv-sheet"><b>Your call from the full sheet</b><small class="sub">A play off the week's sheet is less practiced: a few more busts.</small>
      <div class="gd-form small">${sel("personnel", sh.personnel, "Personnel")}${sel("backfield", sh.backfield, "Backfield")}${sel("set", sh.set, "Set")}${sel("motion", sh.motion, "Motion")}
      <label>Strength</label><select data-act="lv-sheetv" data-k="strength"><option value="left" ${s.strength === "left" ? "selected" : ""}>Left</option><option value="right" ${s.strength === "right" ? "selected" : ""}>Right</option></select>
      <label>Kind</label><select data-act="lv-sheetv" data-k="kind">${[["pass", "Pass"], ["run", "Run"], ["rpo", "RPO"]].map(([k, t]) => `<option value="${k}" ${s.kind === k ? "selected" : ""}>${t}</option>`).join("")}</select>
      ${sel("play", plays, "Play")}
      ${s.kind === "rpo" ? sel("run", sh.runs, "The run") : ""}
      ${s.kind !== "pass" ? `<label>Direction</label><select data-act="lv-sheetv" data-k="dir"><option value="left" ${s.dir === "left" ? "selected" : ""}>Left</option><option value="right" ${s.dir === "right" ? "selected" : ""}>Right</option></select>` : ""}</div>
      <div class="ask-actions"><button class="st-btn glass" data-act="lv-sheet-x">Cancel</button><button class="st-btn primary" data-act="lv-sheet-go">Call it</button></div></div></div>`;
  }

  function liveScreen(st) {
    const p = st.pending;
    const watching = LV.watch ? `<div class="lv-watching"><span class="st-pill live">Watching</span> Your staff calls it at ${LV.watch.pace === "every" ? "every-snap" : "key-play"} pace. <a class="link" data-act="lv-pace">${LV.watch.pace === "every" ? "Key plays only" : "Every snap"}</a> · <button class="st-btn small primary" data-act="lv-stepin">Step in <kbd>Esc</kbd></button></div>` : "";
    const stop = st.settings.stop;
    const opts = `<select class="gd-select small" data-act="lv-set">${[["situations", "Your situations stop"], ["every", "Every snap stops"], ["off", "No snap stops"]].map(([k, t]) => `<option value="${k}" ${stop === k ? "selected" : ""}>${t}</option>`).join("")}</select>`;
    return `<div class="st-room lv-room"><i></i><i></i><i></i>${scoreboard(st)}${fieldStrip(st)}</div>
      <div class="pad lv-pad">
        <div class="lv-bar"><span class="st-label">${esc(st.date)} · ${esc(st.weather.text)}</span><span class="lv-bar-r">${opts}
          <button class="st-btn small glass" data-act="lv-take" title="The next snap stops for you whatever the situation" aria-pressed="${st.take_next}"><kbd>N</kbd> ${st.take_next ? "Next snap is yours" : "Take the next snap"}</button></span></div>
        ${watching}
        <div class="lv-grid">${p ? decision(st) : ""}
          <div class="lv-right"><div class="panel warm"><h2>The game <small>newest first · names link</small></h2>${feed(st)}</div>${teamLine(st)}</div></div>
      </div>${LV.sheet ? sheetPanel(st) : ""}`;
  }

  function finalScreen(st) {
    const a = st.after;
    const mine = st.mine;
    const v = st.view;
    const word = a.tied ? "A tie" : a.won ? "A win" : "A loss";
    const row = (t, x) => `<tr><td class="l">${t}</td><td>${x.snaps}</td><td>${x.snaps ? sign(x.epa) : "–"}</td><td>${x.snaps ? `${x.success.toFixed(0)}%` : "–"}</td></tr>`;
    const kp = a.key_plays.map((p) => `<li class="${p.td ? "td" : ""} ${p.turnover ? "to" : ""}"><span class="lv-fq">${esc(p.q)} ${esc(p.clock)}</span><div><div class="lv-ftext">${linkNames(p.text, p.who)}</div><div class="lv-cmp">${esc(p.call || "")}${p.why ? " · " + esc(p.why) : ""}</div></div><span class="lv-fe ${(p.team === mine) === (p.swing * (mine === 0 ? 1 : -1) > 0) ? "up" : "down"}">${sign(100 * Math.abs(p.swing) * ((mine === 0 ? 1 : -1) * Math.sign(p.swing)), 0)}%</span></li>`).join("");
    const balls = a.stars.map((s) => `<div class="lv-star ${s.side === mine ? "mine" : ""}">${playerLink(s.id, s.name)} <small>${esc(s.pos || "")} · ${esc(s.team)}</small><div class="sub">${esc(s.line)}</div></div>`).join("");
    const off = a.officials;
    return `<div class="st-room lv-room"><i></i><i></i><i></i>${scoreboard(st)}</div>
      <div class="pad lv-pad">
        <div class="lv-final"><div><div class="st-label">Final · ${esc(st.date)}</div><div class="h1">${word}, ${v.score[mine]}-${v.score[1 - mine]}</div></div>
          <button class="st-btn primary" data-act="lv-finish">Finish the day and open the game page <kbd>↵</kbd></button></div>
        <div class="grid2"><div class="panel warm"><h2>The plays that decided it <small>by the swing in your win chance</small></h2><ul class="lv-feed">${kp}</ul></div>
          <div style="display:grid;gap:18px;align-content:start">
            <div class="panel warm"><h2>Your calls and your staff's <small>this game</small></h2><div class="tbl"><table><thead><tr><th class="l">Called by</th><th>Snaps</th><th>EPA a play</th><th>Success</th></tr></thead><tbody>${row("You", a.calls.yours)}${row("Your staff", a.calls.staff)}</tbody></table></div>${a.calls.off_sheet ? `<div class="sub">${a.calls.off_sheet} of your calls ${a.calls.off_sheet === 1 ? "was" : "were"} off the week's sheet.</div>` : ""}</div>
            <div class="panel warm"><h2>Game balls</h2>${balls || '<div class="empty">Nobody stood out.</div>'}</div>
            ${a.hurt.length ? `<div class="panel warm"><h2>The trainers <small>scans tomorrow</small></h2>${a.hurt.map((x) => `<div class="lv-hurt">${esc(x)}</div>`).join("")}</div>` : ""}
            <div class="panel warm"><h2>The officials</h2><div class="sub">${off.close ? `${off.close} close call${off.close === 1 ? "" : "s"}: ${off.wrong} wrong on the field, ${off.fixed} fixed by replay.` : "No close calls."}</div></div>
          </div></div></div>`;
  }

  // ---------- the live screen's actions ----------
  async function send(input, opts = {}) {
    if (LV.busy) return;
    LV.busy = true;
    clearTimer();
    try {
      LV.seen = (LV.state && LV.state.plays ? LV.state.plays.length : 0);
      const st = await api("live_answer", input);
      LV.state = st;
      LV.fresh = true;
      LV.fourth = null;
      LV.two = null;
      if (!opts.keepSheet) LV.sheet = null;
      LV.busy = false;
      if (S.route.screen === "live") render();
      else go("live");
    } catch (e) {
      LV.busy = false;
      toast(e.message);
    }
  }
  function clearTimer() {
    if (LV.timer) clearInterval(LV.timer);
    LV.timer = null;
    LV.hold = (LV.hold || 0) + 1;
  }
  const live = {
    // After the screen is drawn: the realism clock, watching, focus.
    after(st) {
      clearTimer();
      if (S.route.screen !== "live") return;
      LV.seen = st.plays ? st.plays.length : 0;
      if (st.status !== "live" || !st.pending) return;
      if (LV.watch) {
        const last = st.plays[st.plays.length - 1];
        const wait = LV.watch.pace === "every" ? 900 : last && last.key ? 1700 : 160;
        const step = () => LV.watch && S.route.screen === "live" && send({ kind: "staff", stop: "every" });
        // The 2D view (lane 13) shows the snap first, then the next one goes.
        const hold = window.AFMViews && window.AFMViews.liveHold ? window.AFMViews.liveHold(LV.watch.pace, !!(last && last.key)) : null;
        if (hold) {
          const tok = (LV.hold = (LV.hold || 0) + 1);
          hold.then(() => tok === LV.hold && (LV.timer = setTimeout(step, 350)));
          return;
        }
        LV.timer = setTimeout(step, wait);
        return;
      }
      if (st.settings.clock === "realism" && !["halftime", "toss"].includes(st.pending.ask)) {
        LV.deadline = Date.now() + 25000;
        LV.timer = setInterval(() => {
          const left = Math.max(0, LV.deadline - Date.now());
          const s = $("#lv-secs"), b = $("#lv-tbar");
          if (s) s.textContent = Math.ceil(left / 1000);
          if (b) b.style.width = `${(100 * left) / 25000}%`;
          if (left <= 0) {
            clearTimer();
            toast("The radio cut out: your play caller's call went in.");
            send({ kind: "staff" });
          }
        }, 200);
      }
    },
    "lv-call": (d) => {
      const p = LV.state.pending;
      if (p.ask === "pat") return send({ kind: "pat", two: true, option: Number(d.i) });
      send({ kind: "call", option: Number(d.i), ...(p.view.down === 4 || p.staff_fourth ? { fourth: "go" } : {}) });
    },
    "lv-def": (d) => send({ kind: "defense", option: Number(d.i) }),
    "lv-staff": () => {
      const p = LV.state && LV.state.pending;
      if (!p) return;
      if (p.ask === "halftime") return live["lv-half"]();
      // Going for it against your staff's kick: his play call stands.
      if (LV.fourth === "go" && p.staff_fourth) return send({ kind: "call", fourth: "go" });
      if (LV.fourth) return send({ kind: "call", fourth: LV.fourth });
      send({ kind: "staff" });
    },
    "lv-fourth": (d) => {
      const p = LV.state.pending;
      if (d.v !== "go" && d.v === LV.fourth) return send({ kind: "call", fourth: d.v });
      LV.fourth = d.v;
      // Going when your staff planned to: his call stands as the first card.
      if (d.v === "go" && !p.offense.length) return send({ kind: "call", fourth: "go" });
      render($(".view").scrollTop);
    },
    "lv-fourth-go": () => LV.fourth && send({ kind: "call", fourth: LV.fourth }),
    "lv-pat": (d) => {
      if (d.v === "0") return send({ kind: "pat", two: false });
      if (LV.two) return send({ kind: "pat", two: true, option: 0 });
      LV.two = true;
      render($(".view").scrollTop);
    },
    "lv-onside": (d) => send({ kind: "onside", onside: d.v === "1" }),
    "lv-toss": (d) => send({ kind: "toss", choice: d.v }),
    "lv-chal": (d) => send({ kind: "challenge", throw: d.v === "1" }),
    "lv-timeout": () => send({ kind: "timeout" }),
    "lv-tempo": (d) => send({ kind: "tempo", tempo: d.v }),
    "lv-wordb": (d) => send({ kind: "plan", [d.k]: d.v === "1" }),
    // A word on the sideline to a player after his mistake (lane 14, D6).
    "tk-word": (d) => {
      const w = (LV.state.words || []).find((x) => x.id === Number(d.id));
      const o = w && w.options[Number(d.i)];
      if (o) send({ kind: "word", id: w.id, move: o.pick.move, tone: o.pick.tone });
    },
    "lv-half": () => {
      const st = LV.state;
      const input = { kind: "halftime" };
      document.querySelectorAll(".lv-adj-on").forEach((x) => {
        if (!x.checked) return;
        const a = st.halftime.adjust[Number(x.dataset.i)];
        Object.assign(input, a.change || {});
      });
      send(input);
    },
    "lv-take": async () => {
      try { LV.state = await api("live_take"); LV.fresh = true; render($(".view").scrollTop); toast("The next snap stops for you."); } catch (e) { toast(e.message); }
    },
    "lv-watch": () => {
      LV.watch = { pace: keep("lvpace", { p: "key" }).p, stop: LV.state.settings.stop };
      send({ kind: "staff", stop: "every" });
    },
    "lv-pace": () => {
      const k = keep("lvpace", { p: "key" });
      k.p = k.p === "every" ? "key" : "every";
      if (LV.watch) LV.watch.pace = k.p;
      render($(".view").scrollTop);
    },
    "lv-stepin": () => live.stepIn(),
    async stepIn() {
      if (!LV.watch) return;
      const stop = LV.watch.stop;
      LV.watch = null;
      clearTimer();
      try { await api("gameday_settings", { stop }); LV.state = await api("live"); LV.fresh = true; render($(".view").scrollTop); } catch (e) { toast(e.message); }
    },
    "lv-sim": async () => {
      if (!(await sure("Hand the rest of the game to your staff?", "They call every snap from here with your plan and rules. You'll see the final.", "Hand it over"))) return;
      LV.watch = null;
      clearTimer();
      showBusy("Your staff coaches the rest of the game…");
      try { LV.seen = LV.state.plays.length; LV.state = await api("live_staff"); LV.fresh = true; hideBusy(); render(); } catch (e) { hideBusy(); toast(e.message); }
    },
    async setStop(v) {
      try { await api("gameday_settings", { stop: v }); LV.state = await api("live"); LV.fresh = true; render($(".view").scrollTop); } catch (e) { toast(e.message); }
    },
    "lv-sheet": () => {
      const p = LV.state.pending;
      const sh = LV.state.sheet;
      LV.sheet = p.ask === "defense"
        ? { personnel: "Nickel", coverage: sh.coverages[4] || sh.coverages[0], pressure: sh.pressures[0], front: sh.fronts[0], stunt: sh.stunts[0], disguise: false, spy: false }
        : { personnel: "11", backfield: "shotgun", set: "doubles", motion: "none", strength: "left", kind: "pass", play: sh.passes[0].name, run: sh.runs[0], dir: "left" };
      render($(".view").scrollTop);
    },
    "lv-sheet-x": () => { LV.sheet = null; render($(".view").scrollTop); },
    "lv-sheet-go": () => {
      const p = LV.state.pending;
      const s = { ...LV.sheet };
      if (p.ask === "defense") return send({ kind: "defense", custom: { ...s, disguise: !!s.disguise, spy: !!s.spy } });
      if (p.ask === "pat") return send({ kind: "pat", two: true, custom: s });
      send({ kind: "call", custom: s, ...(p.view.down === 4 || p.staff_fourth ? { fourth: "go" } : {}) });
    },
    "lv-finish": async () => {
      showBusy("The rest of the day plays…");
      try {
        const r = await api("day");
        S.status = r.status;
        hideBusy();
        LV.state = null;
        if (r.game) go("game", { index: r.game.index });
        else go("home");
      } catch (e) { hideBusy(); toast(e.message); }
    },
  };
  document.addEventListener("change", (ev) => {
    const d = ev.target.dataset;
    if (d.act === "lv-sheetv" && LV.sheet) {
      LV.sheet[d.k] = ev.target.value === "1" ? true : ev.target.value === "" && (d.k === "disguise" || d.k === "spy") ? false : ev.target.value;
      if (d.k === "kind") {
        const sh = LV.state.sheet;
        LV.sheet.play = ev.target.value === "run" ? sh.runs[0] : ev.target.value === "rpo" ? sh.rpos[0].name : sh.passes[0].name;
        render($(".view").scrollTop);
      }
    }
    if (d.act === "lv-word" && ev.target.value) {
      const v = ev.target.value;
      send({ kind: "plan", [d.k]: v === "none" ? "none" : Number(v) });
    }
  });

  // Keys on the live screen: they never reach the global Continue.
  document.addEventListener(
    "keydown",
    (ev) => {
      if (S.route.screen !== "live" || !LV.state) return;
      const typing = ev.target.closest && ev.target.closest("input, textarea, select, [contenteditable]");
      if (typing || ev.metaKey || ev.ctrlKey || ev.altKey || document.querySelector(".overlay:not(.lv-sheet-o)")) return;
      const st = LV.state;
      const p = st.pending;
      const k = ev.key.toLowerCase();
      const act = (fn) => { ev.preventDefault(); ev.stopPropagation(); fn(); };
      if (k === " ") return act(() => {});
      if (k === "escape") return act(() => (LV.sheet ? live["lv-sheet-x"]() : live.stepIn()));
      if (st.status === "final") { if (k === "enter") act(() => live["lv-finish"]()); return; }
      if (!p || LV.busy) return;
      if (LV.sheet) { if (k === "enter") act(() => live["lv-sheet-go"]()); return; }
      if (k === "enter") {
        if (p.ask === "halftime") return act(() => live["lv-half"]());
        return act(() => live["lv-staff"]());
      }
      if (k === "w") return act(() => live["lv-watch"]());
      if (k === "e") return act(() => live["lv-sim"]());
      if (k === "n") return act(() => live["lv-take"]());
      if (k === "t" && ["snap", "defense"].includes(p.ask)) return act(() => live["lv-timeout"]());
      if (k === "f" && ["snap", "defense", "pat"].includes(p.ask)) return act(() => live["lv-sheet"]());
      if (p.ask === "snap" && (p.view.down === 4 || p.staff_fourth)) {
        const m = { g: "go", p: "punt", k: "fg" }[k];
        if (m && (m !== "fg" || p.fg_make != null)) return act(() => live["lv-fourth"]({ v: m }));
      }
      if (p.ask === "pat" && (k === "k" || k === "g")) return act(() => live["lv-pat"]({ v: k === "g" ? "1" : "0" }));
      const n = Number(k);
      if (n >= 1 && n <= 9) {
        const i = n - 1;
        if (p.ask === "toss" && p.toss[i]) return act(() => send({ kind: "toss", choice: p.toss[i] }));
        if (p.ask === "challenge" && i < 2) return act(() => send({ kind: "challenge", throw: i === 0 }));
        if (p.ask === "onside" && i < 2) return act(() => send({ kind: "onside", onside: i === 1 }));
        if (p.ask === "defense" && p.defense[i]) return act(() => live["lv-def"]({ i }));
        if ((p.ask === "snap" || (p.ask === "pat" && LV.two)) && p.offense[i]) return act(() => live["lv-call"]({ i }));
      }
    },
    true,
  );

  // ======================================================================
  // The game page (screens topic 10).
  // ======================================================================
  // Tabs other lanes add (the Watch tab): name -> async (game, args) => html.
  window.gamePageTabs = window.gamePageTabs || {};
  const GP_TABS = ["Key plays", "Box score", "Advanced", "Drive chart", "Play by play", "Snaps", "Charts", "Officials"];

  screens.game = async (a) => {
    const g = await api("game_page", { index: a.index });
    lightBoth(g.home, g.away);
    // Lanes may put their tabs first (window.gamePageFirst); the first tab
    // is the default.
    const first = (window.gamePageFirst || []).filter((t) => window.gamePageTabs[t]);
    const extra = Object.keys(window.gamePageTabs).filter((t) => !first.includes(t));
    const tabs = [...first, ...GP_TABS, ...extra];
    const tab = keep("gptab", { t: tabs[0] });
    if (a.tab) tab.t = a.tab;
    if (!tabs.includes(tab.t)) tab.t = tabs[0];
    const [h, w] = g.sides;
    const mineHome = g.mine !== 1;
    const marks = g.key_plays.filter((p) => Math.abs(p.swing) >= 0.08).map((p) => ({ n: p.n, wp: p.wp_home + p.swing, good: (p.swing > 0) === mineHome, title: `${p.q} ${p.clock}: ${p.text}` }));
    const qs = (g.quarters || []).map((q, i) => `<th>${i < 4 ? "Q" + (i + 1) : "OT"}</th>`).join("");
    const qline = (k) => (g.quarters || []).map((q) => `<td>${q[k]}</td>`).join("");
    const won = (k) => g.sides[k].score > g.sides[1 - k].score;
    const half = (() => { const p = g.snaps.find((x) => x.q === "Q3"); return p ? [p.n] : []; })();
    const sideTeam = (k) => (k === 0 ? g.home : g.away);
    const hero = `<div class="st-room gp-room"><i></i><i></i><i></i><div class="gp-hero">
      <div class="st-label">${esc(g.date)} · ${esc(g.level)}${g.overtime ? " · overtime" : ""}${g.mine != null ? " · your game" : ""}</div>
      <div class="gp-score">
        <a class="gp-team ${won(1) ? "won" : ""}" ${teamGo(g.away)}>${mark(g.away, 60)}<b>${esc(w.name)}</b></a><span class="gp-pts ${won(1) ? "won" : ""}">${w.score}</span>
        <span class="gp-at">at</span>
        <span class="gp-pts ${won(0) ? "won" : ""}">${h.score}</span><a class="gp-team ${won(0) ? "won" : ""}" ${teamGo(g.home)}><b>${esc(h.name)}</b>${mark(g.home, 60)}</a>
      </div>
      ${g.line && (g.line.words || g.line.crowd) ? `<div class="gp-line">${g.line.words ? `<span><span class="st-label">The line</span> ${esc(g.line.words)}${g.line.against ? ` · <b>${esc(g.line.against)}</b>` : ""}</span>` : ""}${g.line.crowd ? `<span><span class="st-label">Crowd</span> ${Number(g.line.crowd).toLocaleString()}</span>` : ""}</div>` : ""}
      <div class="gp-under"><div class="tbl gp-qs"><table><thead><tr><th class="l"></th>${qs}<th>T</th></tr></thead><tbody><tr><td class="l">${esc(w.name)}</td>${qline(1)}<td><b>${w.score}</b></td></tr><tr><td class="l">${esc(h.name)}</td>${qline(0)}<td><b>${h.score}</b></td></tr></tbody></table></div>
        ${(window.gamePageOwnWp || []).includes(tab.t) ? "" : `<div class="gp-wpbox"><div class="st-label">${esc(g.mine != null ? sideTeam(g.mine).name : h.name)} win chance · the big swings marked</div>${wpChart(g.wp, mineHome, { h: 110, marks, halves: half })}</div>`}</div>
      ${g.stars.length ? `<div class="gp-stars"><span class="st-label">Game balls</span>${g.stars.slice(0, 4).map((s) => `<span>${playerLink(s.id, s.name)} <small>${esc(s.pos || "")} · ${esc(s.line)}</small></span>`).join("")}</div>` : ""}
    </div></div>`;
    let body;
    if (window.gamePageTabs[tab.t]) body = await window.gamePageTabs[tab.t](g, a);
    else body = gp[tab.t](g, a);
    const go = goLine([
      ["Your schedule", 'data-go="schedule"'],
      ["Standings", 'data-go="standings"'],
      ["Game pages", 'data-go="games"'],
      ["Next game", 'data-go="gameday"'],
      [g.mine === 0 ? g.away.name : g.home.name, teamGo(g.mine === 0 ? g.away : g.home)],
    ]);
    return `${hero}<div class="pad gp-pad">${go}<div class="tabsline">${tabs.map((t) => `<button data-act="gp-tab" data-t="${t}" aria-selected="${t === tab.t}">${t}</button>`).join("")}</div>${body}</div>`;
  };

  const yoursOn = (p, mine) => mine != null && p.yours && ((p.team === mine && p.yours & 1) || (p.team !== mine && p.yours & 2));
  function playRows(list, g, opts = {}) {
    let lastQ = null;
    return list
      .map((p) => {
        const sep = opts.quarters && p.q !== lastQ ? `<tr class="grouprow"><td class="l" colspan="7">${p.q === "OT" ? "Overtime" : "Quarter " + p.q.slice(1)} · ${esc(g.sides[1].name)} ${p.score[1]}, ${esc(g.sides[0].name)} ${p.score[0]}</td></tr>` : "";
        lastQ = p.q;
        const you = yoursOn(p, g.mine);
        const ofs = p.officiating ? `<div class="gp-ref">${esc(p.officiating.note)}${p.officiating.reviewed ? (p.officiating.overturned ? " · overturned on review" : " · stood after review") : ""}</div>` : "";
        return `${sep}<tr id="gp-p${p.n}" class="${p.td ? "td" : ""} ${p.turnover ? "to" : ""}"><td class="l q"><a class="vw-pbp" data-vwgo="${p.n}" title="Watch this snap" aria-label="Watch this snap">▶</a>${esc(p.q)} ${esc(p.clock)}</td><td class="l q"><span class="gp-off" style="--c:${colorsOf(p.team === 0 ? g.home : g.away)[0]}"></span>${esc(initials(p.off))}</td><td class="l q">${esc(p.dd)}</td>
          <td class="l gp-text">${linkNames(p.text, p.who)}${you ? ' <span class="lv-yours">Your call</span>' : ""}${ofs}</td>
          <td class="l gp-why">${p.call ? `<b>${esc(p.call)}</b>` : ""}${p.defense ? `<span>vs ${esc(p.defense)}</span>` : ""}${p.look ? `<span>${esc(p.look)}</span>` : ""}${p.why ? `<em>${esc(p.why)}</em>` : ""}${you && p.staff_call ? `<span>Your staff's call: ${esc(p.staff_call)}</span>` : ""}</td>
          <td class="${p.epa > 0 ? "up" : p.epa < 0 ? "down" : ""}">${p.kind === "Kickoff" || p.kind === "ExtraPoint" ? "" : sign(p.epa)}</td><td>${pct(p.wp_home + (p.swing || 0))}</td></tr>`;
      })
      .join("");
  }
  const pbpHead = (g) => `<thead><tr><th class="l">Q</th><th class="l">Ball</th><th class="l">Down</th><th class="l">Play</th><th class="l">The calls and why</th><th title="Expected points added for the offense">EPA</th><th title="${esc(g.home.name)} win chance after the play">${esc(initials(g.home.name))} win</th></tr></thead>`;

  const gp = {
    "Key plays": (g) => {
      const rows = g.key_plays
        .map((p) => {
          const swing = (g.mine === 1 ? -1 : 1) * p.swing;
          return `<div class="gp-kp ${p.td ? "td" : ""} ${p.turnover ? "to" : ""}"><span class="gp-kp-q">${esc(p.q)}<small>${esc(p.clock)}</small></span>
            <span class="gp-off big" style="--c:${colorsOf(p.team === 0 ? g.home : g.away)[0]}"></span>
            <div><div class="gp-kp-text">${linkNames(p.text, p.who)}</div><div class="gp-kp-why">${p.dd ? esc(p.dd) + " · " : ""}${esc(p.call || p.kind)}${p.defense ? " vs " + esc(p.defense) : ""}${p.why ? " · " + esc(p.why) : ""}</div></div>
            <div class="gp-kp-n"><b class="${swing > 0 ? "up" : "down"}">${sign(100 * swing, 0)}%</b><small>${esc(g.mine != null ? "your" : g.home.name)} win chance</small></div>
            <span class="gp-kp-links"><a class="link gp-kp-go" data-vwgo="${p.n}" title="Watch this snap">Watch ›</a><a class="link gp-kp-go" data-act="gp-play" data-n="${p.n}" title="This snap in the play by play">Snap ›</a></span></div>`;
        })
        .join("");
      const calls = g.calls ? `<div class="panel warm"><h2>Your calls and your staff's <small>EPA for your team, success by the 40/60/100 rule</small></h2><div class="tbl"><table><thead><tr><th class="l">Called by</th><th>Snaps</th><th>EPA a play</th><th>Success</th></tr></thead><tbody>${[["You", g.calls.yours], ["Your staff", g.calls.staff]].map(([t, x]) => `<tr><td class="l">${t}</td><td>${x.snaps}</td><td>${sign(x.epa)}</td><td>${x.success.toFixed(0)}%</td></tr>`).join("")}</tbody></table></div></div>` : "";
      const stars = g.stars.map((s) => `<div class="lv-star ${s.side === g.mine ? "mine" : ""}">${playerLink(s.id, s.name)} <small>${esc(s.pos || "")} · ${esc(s.team)}</small><div class="sub">${esc(s.line)}</div></div>`).join("");
      return `<div class="grid2"><div class="panel warm"><h2>The plays that decided it <small>the biggest swings in win chance, scores and turnovers</small></h2>${rows || '<div class="empty">No play swung this one.</div>'}</div>
        <div style="display:grid;gap:18px;align-content:start">${calls}<div class="panel warm"><h2>Game balls</h2>${stars || '<div class="empty">Nobody stood out.</div>'}</div></div></div>`;
    },
    "Box score": (g) => {
      const [h, w] = g.sides;
      const T = g.totals;
      const stat = (t, f) => `<tr><td class="l">${t}</td><td>${f(T[1])}</td><td>${f(T[0])}</td></tr>`;
      const bx = g.box || [{}, {}];
      const side = (s, k) => {
        const b = bx[k] || {};
        return `<div class="panel warm"><h2>${esc(s.name)}</h2>
        ${tbl(["Passing", "C/A", "Yds", "TD", "Int", "Sk", "Clean", "Pressured", "aDOT"], (b.passing || s.passing.map((r) => ({ ...r }))).map((r) => [playerLink(r.id, r.name), `${r.cmp}/${r.att}`, r.yds, r.td, r.int, r.sk, r.clean || "", r.pressured || "", r.adot != null ? one(r.adot) : ""]))}
        ${tbl(["Rushing", "Car", "Yds", "TD", "Long", "Before contact"], (b.rushing || s.rushing).map((r) => [playerLink(r.id, r.name), r.car, r.yds, r.td, r.long, r.bc != null ? one(r.bc) : ""]))}
        ${tbl(["Receiving", "Rec", "Tgt", "Yds", "TD", "Air yds", "YAC", "Sep"], (b.receiving || s.receiving).map((r) => [playerLink(r.id, r.name), r.rec, r.tgt, r.yds, r.td, r.air != null ? Math.round(r.air) : "", r.yac != null ? Math.round(r.yac) : "", r.sep != null ? one(r.sep) : ""]))}
        ${tbl(["Defense", "Tkl", "TFL", "Sk", "Pres", "Int", "PD"], (b.defense || s.defense).map((r) => [playerLink(r.id, r.name), r.tkl, r.tfl, r.sk, r.pres ?? "", r.int, r.pd]))}
        ${b.kicking && b.kicking.length ? tbl(["Kicking", "FG", "Long", "XP", "Punts", "Avg", "In 20"], b.kicking.map((r) => [playerLink(r.id, r.name), r.fg, r.long || "", r.xp, r.punts || "", r.avg ? one(r.avg) : "", r.in20 || ""])) : ""}
        ${b.returns && b.returns.length ? tbl(["Returns", "KR", "Yds", "PR", "Yds", "Long"], b.returns.map((r) => [playerLink(r.id, r.name), r.kr, r.kr_yds, r.pr, r.pr_yds, r.long])) : ""}</div>`;
      };
      const team = (k) => (k === 0 ? g.home : g.away);
      const scores = (g.scoring || [])
        .map((x) => `<div class="gp-sum"><span class="gp-sum-t">${mark(team(x.side), 24)}</span>
          <div><div class="gp-sum-h"><b>${esc(x.what)}</b><span class="muted">${esc(x.q)} ${esc(x.clock)}</span></div>
          <div>${linkNames(x.text + (x.try ? ` (${x.try})` : ""), x.who)}</div>${x.drive ? `<small class="muted">${esc(x.drive)}</small>` : ""}</div>
          <span class="gp-sum-s"><span class="${x.side === 1 ? "lead" : ""}">${x.score[1]}</span>–<span class="${x.side === 0 ? "lead" : ""}">${x.score[0]}</span></span></div>`)
        .join("");
      return `<div class="grid2 gp-box-top"><div class="panel warm"><h2>Team stats</h2><div class="tbl"><table><thead><tr><th class="l"></th><th>${esc(w.name)}</th><th>${esc(h.name)}</th></tr></thead><tbody>
        ${stat("Total yards", (t) => t.yards)}${stat("Passing", (t) => t.pass)}${stat("Rushing", (t) => t.rush)}${stat("First downs", (t) => t.first_downs)}${stat("Third downs", (t) => t.third)}${stat("Fourth downs", (t) => t.fourth)}${stat("Sacks by", (t) => t.sacks)}${stat("Turnovers", (t) => t.turnovers)}${stat("Penalties", (t) => t.penalties)}${stat("Possession", (t) => t.top)}${stat("Plays", (t) => t.plays)}
        </tbody></table></div></div>
        <div class="panel warm"><h2>Scoring <small>${esc(w.name)} first</small></h2>${scores || '<div class="empty">No points in this game.</div>'}</div></div>
        <div class="grid2 even">${side(w, 1)}${side(h, 0)}</div>
        <div class="sub">Clean and pressured: completions and attempts with the pocket clean and under pressure. Sep: yards of separation at the catch point. Pres: pressures. Before contact: yards a carry before the first defender touched him.</div>`;
    },
    Advanced: (g) => {
      const A = g.advanced;
      const r = (t, f, tip) => `<tr><td class="l" title="${esc(tip || "")}">${t}</td><td>${f(A[1])}</td><td>${f(A[0])}</td></tr>`;
      const halves = g.halves || [];
      const hrow = (t, f) => `<tr><td class="l">${t}</td>${halves.map((x) => `<td>${f(x[1])}</td><td>${f(x[0])}</td>`).join("")}</tr>`;
      const fourth = (g.fourths || [])
        .map((x) => `<tr><td class="l">${esc(x.q)} ${esc(x.clock)}</td><td class="l">${esc(x.team)}</td><td class="l">${esc(x.dd)}</td><td class="l"><b>${esc(x.did)}</b></td><td>${pct(x.go)}</td><td>${pct(x.punt)}</td><td>${x.fg ? pct(x.fg) : "-"}</td><td class="l ${x.cost > 0.015 ? "down" : ""}">${esc(x.verdict)}</td></tr>`)
        .join("");
      return `<div class="panel warm"><h2>Team numbers <small>from every snap: success is 40% of the distance on 1st down, 60% on 2nd, all of it on 3rd and 4th</small></h2><div class="tbl"><table><thead><tr><th class="l"></th><th>${esc(A[1].name)}</th><th>${esc(A[0].name)}</th></tr></thead><tbody>
        ${r("Plays", (x) => x.plays)}${r("EPA a play", (x) => sign(x.epa_play))}${r("Total EPA", (x) => sign(x.epa, 1))}${r("Success rate", (x) => x.success.toFixed(0) + "%")}${r("Explosive plays", (x) => x.explosive, "Runs of 10+ and passes of 20+")}
        ${r("Run EPA a play", (x) => sign(x.run_epa))}${r("Dropback EPA a play", (x) => sign(x.pass_epa))}${r("Early-down pass rate", (x) => x.early_pass.toFixed(0) + "%")}${r("Red zone TDs / trips", (x) => x.rz)}${r("Pressure rate allowed", (x) => x.pressure.toFixed(0) + "%")}
        ${r("Average depth of target", (x) => one(x.adot))}${r("Yards after the catch", (x) => one(x.yac))}${r("Time to throw", (x) => x.ttt.toFixed(2) + " s")}${r("Completion rate", (x) => x.cmp.toFixed(0) + "%")}${r("Yards a carry", (x) => one(x.ypc))}${r("Yards before contact", (x) => one(x.before_contact))}${r("Average start (own yard line)", (x) => Math.round(x.start))}
        </tbody></table></div></div>
        ${halves.length ? `<div class="panel warm"><h2>By half</h2><div class="tbl"><table><thead><tr><th class="l"></th>${halves.map((x, i) => `<th>${i === 0 ? "1st" : i === 1 ? "2nd" : "OT"} · ${esc(initials(A[1].name))}</th><th>${esc(initials(A[0].name))}</th>`).join("")}</tr></thead><tbody>
          ${hrow("EPA a play", (x) => sign(x.epa_play))}${hrow("Success rate", (x) => x.success.toFixed(0) + "%")}${hrow("Explosive plays", (x) => x.explosive)}${hrow("Pressure rate allowed", (x) => x.pressure.toFixed(0) + "%")}</tbody></table></div></div>` : ""}
        <div class="panel warm"><h2>Every 4th down against the model <small>the true win chance of going, punting and kicking at the snap</small></h2>${fourth ? `<div class="tbl"><table><thead><tr><th class="l">When</th><th class="l">Team</th><th class="l">Situation</th><th class="l">Did</th><th>Go</th><th>Punt</th><th>Kick</th><th class="l">The model</th></tr></thead><tbody>${fourth}</tbody></table></div>` : '<div class="empty">No 4th-down decisions in this game.</div>'}</div>`;
    },
    "Drive chart": (g) => {
      const bars = g.drives
        .map((d, i) => {
          const homeSide = d.team === 0;
          // The home team drives left to right, as on the live field.
          const a = homeSide ? d.start_x : 100 - d.start_x, b = homeSide ? d.end_x : 100 - d.end_x;
          const lo = Math.min(a, b), hi = Math.max(a, b);
          const c = colorsOf(homeSide ? g.home : g.away)[0];
          return `<div class="gp-drive" data-act="gp-drive" data-i="${i}"><span class="gp-dq">${esc(d.q)} ${esc(d.clock)}</span><span class="gp-dteam">${esc(initials(d.name))}</span>
            <span class="gp-dtrack"><i style="left:${lo}%;width:${Math.max(0.8, hi - lo)}%;background:${c}"></i><b style="left:${homeSide ? hi : lo}%">${homeSide ? "▶" : "◀"}</b></span>
            <span class="gp-dres ${d.points > 0 ? "score" : d.points < 0 ? "bad" : ""}">${esc(d.result)}</span><span class="gp-dn">${d.plays} pl · ${d.yards} ${d.yards === 1 ? "yd" : "yds"} · ${esc(d.time)}</span></div>`;
        })
        .join("");
      return `<div class="panel warm"><h2>Every drive <small>${esc(g.home.name)} drives left to right · click a drive for its plays</small></h2><div class="gp-drives">${bars}</div></div>`;
    },
    "Play by play": (g, a) => {
      const f = keep("gppbp", { team: "all", only: "all", drive: null });
      let list = g.snaps;
      if (f.drive != null && g.drives[f.drive]) { const d = g.drives[f.drive]; list = list.filter((p) => p.n >= d.first && p.n <= d.last); }
      if (f.team !== "all") list = list.filter((p) => String(p.team) === f.team);
      if (f.only === "key") { const ks = new Set(g.key_plays.map((p) => p.n)); list = list.filter((p) => ks.has(p.n)); }
      if (f.only === "yours") list = list.filter((p) => yoursOn(p, g.mine));
      if (f.only === "scoring") list = list.filter((p) => p.td || p.kind === "FieldGoal");
      const seg = (k, opts) => `<div class="seg">${opts.map(([v, t]) => `<button data-act="gp-f" data-k="${k}" data-v="${v}" aria-pressed="${String(f[k]) === v}">${t}</button>`).join("")}</div>`;
      return `<div class="filters">${seg("team", [["all", "Both teams"], ["1", esc(g.away.name)], ["0", esc(g.home.name)]])}${seg("only", [["all", "Every snap"], ["key", "Key plays"], ["scoring", "Scores"], ...(g.mine != null ? [["yours", "Your calls"]] : [])])}${f.drive != null ? `<a class="link" data-act="gp-f" data-k="drive" data-v="">Drive ${f.drive + 1} only · show all ×</a>` : ""}</div>
        <div class="panel warm"><div class="tbl pbp gp-pbp"><table>${pbpHead(g)}<tbody>${playRows(list, g, { quarters: f.drive == null })}</tbody></table></div></div>`;
    },
    Snaps: (g) => {
      const sn = g.snap_counts;
      if (!sn) return '<div class="empty">No snap counts for this game.</div>';
      const pers = (k) => tbl(["Personnel", "Snaps", "Share", "Run", "Pass", "Yds a play", "EPA a play"], sn.personnel[k].map((r) => [`<b>${esc(r.name)}</b>`, r.n, r.share + "%", r.run + "%", r.pass + "%", one(r.ypp), sign(r.epa)]));
      const pack = (k) => tbl(["Their package", "Snaps", "Share", "Yds a play", "EPA a play"], sn.packages[k].map((r) => [`<b>${esc(r.name)}</b>`, r.n, r.share + "%", one(r.ypp), sign(r.epa)]));
      const rot = (k) => {
        const rows = sn.players[k];
        const unit = keep("gprot", { u: "OFF" }).u;
        const list = rows.filter((r) => r.unit === unit);
        return table(`gprot${k}${unit}`, [
          { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) },
          { k: "pos", t: "Pos", l: true },
          { k: "snaps", t: "Snaps" },
          { k: "share", t: "Share", f: (r) => r.share + "%" },
        ], list, { sort: "snaps", empty: "Nobody played here." });
      };
      const u = keep("gprot", { u: "OFF" }).u;
      return `<div class="grid2 even"><div class="panel warm"><h2>${esc(g.away.name)} on offense <small>by personnel</small></h2>${pers(1)}<h2>Against ${esc(g.home.name)}'s packages</h2>${pack(1)}</div>
        <div class="panel warm"><h2>${esc(g.home.name)} on offense <small>by personnel</small></h2>${pers(0)}<h2>Against ${esc(g.away.name)}'s packages</h2>${pack(0)}</div></div>
        <div class="filters"><div class="seg">${[["OFF", "Offense"], ["DEF", "Defense"], ["ST", "Special teams"]].map(([k, t]) => `<button data-act="gp-rot" data-v="${k}" aria-pressed="${u === k}">${t}</button>`).join("")}</div><span class="sub">The rotations: every player's snaps</span></div>
        <div class="grid2 even"><div><div class="st-label gp-sl">${esc(g.away.name)}</div>${rot(1)}</div><div><div class="st-label gp-sl">${esc(g.home.name)}</div>${rot(0)}</div></div>`;
    },
    Charts: (g) => {
      const c = keep("gpchart", { team: g.mine != null ? String(g.mine) : "0" });
      const k = Number(c.team);
      const seg = `<div class="seg">${[["1", esc(g.away.name)], ["0", esc(g.home.name)]].map(([v, t]) => `<button data-act="gp-ct" data-v="${v}" aria-pressed="${c.team === v}">${t}</button>`).join("")}</div>`;
      return `<div class="filters">${seg}<span class="sub">The offense's charts; their defense's are the other team's</span></div>
        <div class="grid2 even"><div class="panel warm"><h2>Pass chart <small>every throw from the line to where it was caught or fell</small></h2>${passChart(g, k)}</div>
          <div class="panel warm"><h2>Runs by gap <small>carries and yards a carry</small></h2>${gapChart(g, k)}</div></div>
        <div class="grid2 even"><div class="panel warm"><h2>EPA by drive</h2>${driveEpa(g)}</div><div class="panel warm"><h2>EPA by quarter</h2>${quarterEpa(g)}</div></div>
        <div class="panel warm"><h2>Win chance <small>${esc(g.mine != null ? (g.mine === 0 ? g.home : g.away).name : g.home.name)}</small></h2>${wpChart(g.wp, g.mine !== 1, { h: 180, marks: g.key_plays.filter((p) => Math.abs(p.swing) >= 0.08).map((p) => ({ n: p.n, wp: p.wp_home + p.swing, good: (p.swing > 0) === (g.mine !== 1), title: p.text })) })}</div>`;
    },
    Officials: (g) => {
      const o = g.officials;
      const rows = o.rows.map((r) => `<tr><td class="l">${esc(r.q)} ${esc(r.clock)}</td><td class="l">${esc(r.call)}</td><td class="l wrap">${esc(r.note)}</td><td class="l">${r.wrong ? '<span class="dot bad"></span>Wrong on the field' : '<span class="dot good"></span>Right'}</td><td class="l">${esc(r.who)}</td><td class="l">${r.overturned ? "Overturned" : r.who === "nobody" ? "" : "Stands"}</td><td class="l">${esc(r.against)}</td></tr>`).join("");
      const flags = o.flags || [];
      const team = (k) => (k === 0 ? g.home : g.away);
      const tally = [0, 1].map((k) => {
        const on = flags.filter((f) => f.side === k && f.accepted);
        return `<div class="gp-flagteam"><span>${mark(team(k), 26)}<a class="link name" ${teamGo(team(k))}>${esc(team(k).name)}</a></span><b>${on.length}</b><small>for ${on.reduce((a, f) => a + f.yards, 0)} yards${flags.some((f) => f.side === k && !f.accepted) ? ` · ${flags.filter((f) => f.side === k && !f.accepted).length} declined` : ""}</small></div>`;
      }).join("");
      const frows = flags
        .map((f) => `<tr class="${f.accepted ? "" : "muted"}"><td class="l q"><a class="link" data-act="gp-play" data-n="${f.n}">${esc(f.q)} ${esc(f.clock)}</a></td><td class="l"><span class="gp-off" style="--c:${colorsOf(team(f.side))[0]}"></span>${esc(team(f.side).name)}</td><td class="l"><b>${esc(upper(f.foul))}</b>${f.pre_snap ? '<small class="muted"> · before the snap</small>' : ""}</td><td class="l">${f.name ? `${playerLink(f.id, f.name)} <small class="muted">${esc(f.pos)}</small>` : ""}</td><td>${f.yards}</td><td class="l">${f.accepted ? "Accepted" : "Declined"}</td></tr>`)
        .join("");
      return `<div class="gp-stack"><div class="panel warm"><h2>Flags <small>${flags.length} thrown · every one on the play-by-play</small></h2>
          <div class="gp-flags">${tally}</div>
          ${frows ? `<div class="tbl"><table><thead><tr><th class="l">When</th><th class="l">Against</th><th class="l">Foul</th><th class="l">On</th><th>Yards</th><th class="l"></th></tr></thead><tbody>${frows}</tbody></table></div>` : '<div class="empty">A clean game: no flags.</div>'}</div>
        <div class="panel warm"><h2>Close calls <small>${o.close} · ${o.wrong} wrong on the field · ${o.fixed} fixed by replay</small></h2>
          ${rows ? `<div class="tbl"><table><thead><tr><th class="l">When</th><th class="l">The call</th><th class="l">What happened</th><th class="l">On the field</th><th class="l">Review</th><th class="l">Result</th><th class="l">Against</th></tr></thead><tbody>${rows}</tbody></table></div>` : `<div class="empty">No close calls: nothing in this game was a matter of inches or angles.</div>`}
          <div class="sub">A close call is a catch at the sideline, a ball across the goal line, a spot against the line to gain, a fumble or a trapped ball. ${g.level === "High school" ? "High school games have no replay." : g.level === "NFL" ? "Coaches challenge with a red flag; the booth reviews scoring plays, turnovers and the last two minutes of each half." : "The booth can review any play."}</div></div></div>`;
    },
  };

  // ---------- the charts ----------
  function passChart(g, k) {
    const throws = (g.passes || []).filter((p) => p.team === k);
    if (!throws.length) return '<div class="empty">No throws charted.</div>';
    const W = 420, H = 380, x0 = W / 2, y0 = H - 40, sy = 6.2, sx = 7.4;
    const grid = [-10, 0, 10, 20, 30, 40, 50].map((d) => `<line x1="10" x2="${W - 10}" y1="${y0 - d * sy}" y2="${y0 - d * sy}" class="${d === 0 ? "gp-los" : "gp-gl"}"/><text x="4" y="${y0 - d * sy - 3}" class="gp-gt">${d}</text>`).join("");
    const cls = (p) => (p.int ? "int" : p.td ? "td" : p.complete ? "c" : "i");
    const lines = throws
      .map((p) => {
        const x = x0 + Math.max(-27, Math.min(27, p.y)) * sx, y = y0 - Math.max(-10, Math.min(55, p.air)) * sy;
        return `<g class="gp-throw ${cls(p)}"><line x1="${x0 + (p.from || 0) * sx}" y1="${y0}" x2="${x}" y2="${y}"/><circle cx="${x}" cy="${y}" r="4.2"><title>${esc(p.text)}</title></circle></g>`;
      })
      .join("");
    const n = (f) => throws.filter(f).length;
    return `<svg class="gp-chart" viewBox="0 0 ${W} ${H}">${grid}${lines}</svg>
      <div class="gp-legend"><span class="c">Complete ${n((p) => p.complete && !p.td)}</span><span class="td">Touchdown ${n((p) => p.td)}</span><span class="i">Incomplete ${n((p) => !p.complete && !p.int)}</span><span class="int">Intercepted ${n((p) => p.int)}</span><span class="muted">Left of the QB is left on the chart</span></div>`;
  }
  function gapChart(g, k) {
    const gaps = (g.gaps || [])[k];
    if (!gaps || !gaps.some((x) => x.n)) return '<div class="empty">No designed runs.</div>';
    const max = Math.max(...gaps.map((x) => x.n));
    return `<div class="gp-gaps">${gaps.map((x) => `<div class="gp-gap"><div class="gp-gapbar"><i style="height:${(100 * x.n) / Math.max(1, max)}%"></i></div><b>${x.n ? one(x.ypc) : "–"}</b><small>${x.n} ${x.n === 1 ? "carry" : "carries"}</small><span>${esc(x.name)}</span></div>`).join("")}</div>`;
  }
  function bars(values, labels, colors) {
    const max = Math.max(0.5, ...values.map((v) => Math.abs(v)));
    return `<div class="gp-bars">${values.map((v, i) => `<div class="gp-bar" title="${esc(labels[i])}: ${sign(v, 1)}"><div class="gp-bar-up">${v > 0 ? `<i style="height:${(50 * v) / max}%;background:${colors[i]}"></i>` : ""}</div><div class="gp-bar-dn">${v < 0 ? `<i style="height:${(50 * -v) / max}%;background:${colors[i]}"></i>` : ""}</div><small>${esc(labels[i].split(" ")[0])}</small></div>`).join("")}</div>`;
  }
  function driveEpa(g) {
    const vals = [], labels = [], cols = [];
    g.drives.forEach((d, i) => {
      const e = g.snaps.filter((p) => p.n >= d.first && p.n <= d.last && p.team === d.team && !["Kickoff", "ExtraPoint", "TwoPoint", "Punt", "FieldGoal"].includes(p.kind)).reduce((s, p) => s + p.epa, 0);
      vals.push(e);
      labels.push(`${i + 1} ${d.name}: ${d.result}`);
      cols.push(colorsOf(d.team === 0 ? g.home : g.away)[0]);
    });
    return bars(vals, labels, cols);
  }
  function quarterEpa(g) {
    const rows = g.epa_q.map((q, i) => `<tr><td class="l">${i < 4 ? "Q" + (i + 1) : "OT"}</td><td class="${q[1] > 0 ? "up" : "down"}">${sign(q[1], 1)}</td><td class="${q[0] > 0 ? "up" : "down"}">${sign(q[0], 1)}</td></tr>`).join("");
    return `<div class="tbl"><table><thead><tr><th class="l"></th><th>${esc(g.away.name)}</th><th>${esc(g.home.name)}</th></tr></thead><tbody>${rows}</tbody></table></div>`;
  }

  // ---------- the game page's actions ----------
  Object.assign(live, {
    "gp-tab": (d) => { S.keep.gptab = { t: d.t }; render(); },
    "gp-play": (d) => {
      S.keep.gptab = { t: "Play by play" };
      S.keep.gppbp = { team: "all", only: "all", drive: null };
      render().then(() => { const r = document.getElementById("gp-p" + d.n); if (r) { r.scrollIntoView({ block: "center" }); r.classList.add("flash"); } });
    },
    "gp-drive": (d) => { S.keep.gptab = { t: "Play by play" }; S.keep.gppbp = { team: "all", only: "all", drive: Number(d.i) }; render(); },
    "gp-f": (d) => { const f = keep("gppbp", { team: "all", only: "all", drive: null }); f[d.k] = d.k === "drive" ? null : d.v; render($(".view").scrollTop); },
    "gp-rot": (d) => { keep("gprot", { u: "OFF" }).u = d.v; render($(".view").scrollTop); },
    "gp-ct": (d) => { keep("gpchart", {}).team = d.v; render($(".view").scrollTop); },
  });

  // ======================================================================
  // Continue on game day: the morning of your game opens the game day
  // screen; a game under way is handed to your staff only if you say so.
  // ======================================================================
  CRUMBS.gameday = "Game day";
  CRUMBS.live = "Live";
  // The week before a game is when the film and the plan get done, so the
  // game day screen has its own sidebar line, not only the morning's stop.
  I.gameday = '<circle cx="6.5" cy="17" r="2.5"/><path d="M15 4l5 5M20 4l-5 5M8.5 15c2.5-4.5 5.5-6.5 9.5-6.5M15.5 6.5l2.5 2-2 2.5"/>';
  navAdd({ go: "gameday", label: "Game day", icon: "gameday", sec: "team", after: "schedule" });
  const baseCont = cont;
  cont = async function () {
    if (S.busy) return;
    if (S.status && S.status.mine) {
      let now = null;
      try { now = await api("gameday_now"); } catch (e) { now = null; }
      // Game day: Continue goes to kickoff, unless no snap stops for you.
      if (now && now.today && now.live === "none" && now.stop !== "off") return kickoff();
      if (now && now.live === "live") {
        if (S.route.screen !== "live") return go("live");
        if (!(await sure("Hand the rest of your game to your staff?", "Continue plays the day: your staff calls every snap left with your plan and rules.", "Hand it over"))) return;
      }
    }
    await baseCont();
    // A morning stop on game day: the game day screen opens.
    const why = S.lastWhy || "";
    if (/^Game day/.test(why) && S.route.screen === "home") go("gameday");
  };
  // The morning's reason, kept from the last Continue.
  const baseApi = api;
  api = async function (cmd, args) {
    const r = await baseApi(cmd, args);
    if (cmd === "continue" && r) S.lastWhy = r.morning && r.why && r.why[0] ? r.why[0].text : "";
    return r;
  };
})();
