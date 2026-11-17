/* Capital's Campus screen (lane CS1; docs/design/capital.md CP21, CP37, CP39,
 * board 1). The football side of your campus as a stylized map (not real
 * geography): each building with its rooms and their levels read against your
 * conference's best, a building under way with what it adds and moves (gold
 * squares), the open plots, every room against the conference with a button to
 * put a renovation to your athletic director, the program beside yours (its
 * rivalry game, else the conference neighbour) and what is next.
 *
 *   capitalcampus   no arguments.
 *
 * It reads `capital_campus` and asks with `capital_renovate` {room}. No dollar
 * figures (CP39): money reads in words. Every class is cc- so the app's global
 * names never restyle it. */
"use strict";

I.cccampus = '<rect x="3.5" y="9" width="7" height="11" rx="1"/><rect x="13.5" y="4" width="7" height="16" rx="1"/><path d="M6 12.500h2M6 16h2M16.500 8h2M16.500 12h2M16.500 16h2"/>';
navAdd({ go: "capitalcampus", label: "Campus", icon: "cccampus", sec: "capital", seats: ["college"] });
CRUMBS.capitalcampus = "Campus";

const CC = { said: "", compare: true };

screens.capitalcampus = async () => {
  let x;
  try { x = await api("capital_campus", {}); } catch (e) {
    title("Campus", "");
    return `<div class="pad sl cc"><section class="cc-pb"><div class="cc-lab">Capital</div><p class="cc-empty">${esc(e.message || String(e))}</p></section></div>`;
  }
  title("Campus", x.program);
  const said = CC.said ? `<div class="cc-said"><b>${esc(CC.said)}</b></div>` : "";
  CC.said = "";
  const top = `<div class="cc-top"><div><span class="cc-lab">Capital · ${esc(x.program)}</span><h1>Campus</h1></div>${capTabs("capitalcampus")}<span class="cc-date">${esc(capDate(x.today, { weekday: "short", month: "long", day: "numeric", year: "numeric" }))}</span></div>`;
  return `<div class="pad sl cc">${said}${top}<div class="cc-grid"><div class="cc-col">${ccMap(x)}</div><div class="cc-col">${ccRooms(x)}${ccRival(x)}${ccNext(x)}</div></div></div>`;
};

// ---- the map ----
function ccRoomRow(r) {
  return `<div class="cc-rm${r.temporary ? " temp" : ""}"><span>${esc(capCap(r.name))}${r.note || r.temporary ? `<small>${esc(capCap(r.temporary && !r.note ? "the team works without it this year" : r.note))}</small>` : ""}</span>${capPips(r.level, r.target)}</div>`;
}

function ccMap(x) {
  const halls = x.halls.map((h) => `<div class="cc-bld"><div class="cc-bh"><b>${esc(h.name)}</b>${h.opened ? `<small class="${h.dated ? "amb" : ""}">${h.opened}${h.dated ? " · dated" : ""}</small>` : ""}</div>${h.rooms.map(ccRoomRow).join("")}</div>`).join("");
  const plots = x.buildings.map((b) => {
    const moves = b.rooms.filter((r) => !r.new);
    const fresh = b.rooms.filter((r) => r.new);
    return `<div class="cc-plot"><div class="cc-bh"><b>${esc(capCap(b.name))}</b><small>under way · opens for ${b.opens}</small></div>
      <p class="cc-note">${esc(b.status)}${moves.length ? ` ${esc(capCap(moves.map((r) => r.name).join(", ")))} move${moves.length === 1 ? "s" : ""} here.` : ""}${b.temporary ? " The team works without those rooms until it opens." : ""}</p>
      ${fresh.map((r) => `<div class="cc-rm"><span>${esc(capCap(r.name))} <small>new</small></span>${capPips(0, r.target)}</div>`).join("")}</div>`;
  }).join("");
  const free = x.free_plots > 0 ? Array.from({ length: x.free_plots }, () => '<div class="cc-free"><b class="cc-dim" style="font-size:13px">Open plot</b><span class="cc-note cc-dim">Room for a building the drive could pay for.</span></div>').join("") : "";
  const compare = x.rival ? `<button type="button" class="cc-btn big" data-act="cctoggle">${CC.compare ? "Hide" : "Compare with"} ${esc(x.rival.name)}</button>` : "";
  const warn = x.temporary ? '<p class="cc-note amb">The team works in temporary space this year: the rooms a new building takes read a level lower for their tools.</p>' : "";
  return `<section class="cc-pb"><div class="cc-head"><div><h2>The football side of campus</h2><span class="cc-note">Ivory squares are the level today, read against the ${esc(x.conference)}'s best. Gold squares are what a building or a renovation adds.</span></div>${compare}</div>${warn}
    <div class="cc-map" aria-label="Your campus"><div class="cc-halls">${halls}${plots}${free}</div></div></section>`;
}

// ---- every room against the conference ----
function ccRooms(x) {
  const rows = x.rooms.map((r) => {
    let act = "";
    if (r.renovating) act = '<span class="cc-chip">This summer</span>';
    else if (r.can) act = `<button type="button" class="cc-btn" data-act="ccrenovate" data-room="${r.room}" title="Put a renovation of the ${esc(r.name)} to your athletic director: ${esc(r.odds)}">Renovate</button><span class="cc-odds">${esc(capCap(r.odds))}</span>`;
    return `<div class="cc-room"><b>${esc(capCap(r.name))}</b><div class="cc-rr"><span>${capPips(r.level, 0)} <span class="cc-dim" style="font-size:12px">${r.level} of 5</span></span><span class="cc-rd ${r.tone}">${esc(r.read)}</span>${act}</div><small>${esc(r.unlocks)}</small></div>`;
  }).join("");
  const head = x.head ? "" : '<p class="cc-note cc-dim">Only the head coach puts a renovation to the athletic director.</p>';
  return `<section class="cc-pb"><div class="cc-lab">Against the ${esc(x.conference)}'s best</div>${head}${rows}</section>`;
}

// ---- the program beside yours ----
function ccRival(x) {
  const r = x.rival;
  if (!r || !CC.compare) return "";
  const rows = r.rooms.map((o) => `<span>${esc(capCap(o.name))}</span><span class="n">${o.you}</span><span class="n${o.them > o.you ? " amb" : ""}">${o.them}</span>`).join("");
  return `<section class="cc-pb warn"><div class="cc-lab amb">${esc(r.name)}, beside yours${r.game ? ` · ${esc(capCap(r.game))}` : ""}</div><p class="cc-note">${esc(r.line)}</p>
    <div class="cc-vs"><span class="h">Room</span><span class="h n">You</span><span class="h n">Them</span>${rows}</div></section>`;
}

// ---- what is next ----
function ccNext(x) {
  const drive = typeof screens.capitaldrive === "function" ? '<button type="button" class="cc-btn" data-go="capitaldrive">Open the drive</button>' : "";
  return `<section class="cc-pb cc-next"><div class="cc-lab">What's next</div><p>${esc(x.next.text)}</p>${drive}</section>`;
}

// ---- the buttons ----
Object.assign(CORE_MORE, {
  cctoggle: (el) => { CC.compare = !CC.compare; refresh(el); },
  ccrenovate: async (el) => {
    try {
      const r = await api("capital_renovate", { room: Number(el.dataset.room) });
      if (r && r.said) CC.said = r.said;
    } catch (e) { toast(e.message || String(e)); }
    refresh(el);
  },
});
