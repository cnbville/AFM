//! Find recruits: the college staff's search (FR1-FR13, FS1-FS21, minor
//! bugs 12-16).
//!
//! Who your staff knows (F2): every kid a service or the kicking camps
//! rank, every kid your staff has a file on, every kid with an offer, the
//! all-state teams, your board and your pipeline schools' players; every
//! junior college mover and every portal entrant. The search reads only
//! that: the services' public word, the kids' public numbers and your
//! staff's files. Sorted by the composite rank by default (F3).

use std::collections::HashSet;

use afm_engine::calendar::Date;
use afm_engine::world::data::Division;
use afm_engine::world::front_office::college as desk;
use afm_engine::world::scouting::readers::Staff;
use afm_engine::world::scouting::{file, public::DeskKind, services};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::fields::{self, Ctx, Ranks, Seat, Sure, Val};
use super::query::{Query, thousands};
use super::{page_of, sort_matches, staff};
use crate::views::{self, TeamRef};
use crate::{Reply, colors};

/// A search as the screen sends it.
pub struct Spec {
    pub class: Option<i32>,
    pub level: String,
    pub pos: Option<String>,
    pub state: Option<String>,
    pub q: String,
    pub filters: Option<Value>,
    pub sure: Sure,
    pub unseen: bool,
}

impl Spec {
    pub fn parse(a: &Value) -> Spec {
        let s = |k: &str| a.get(k).and_then(Value::as_str).filter(|s| !s.is_empty() && *s != "ALL").map(str::to_string);
        Spec {
            class: a.get("class").and_then(Value::as_i64).map(|c| c as i32),
            level: s("level").unwrap_or_else(|| "hs".into()),
            pos: s("pos"),
            state: s("state"),
            q: s("q").unwrap_or_default().trim().to_lowercase(),
            filters: a.get("filters").filter(|v| v.is_array()).cloned(),
            sure: Sure::parse(a.get("sure").and_then(Value::as_str)),
            unseen: a.get("unseen").and_then(Value::as_bool).unwrap_or(false),
        }
    }

    pub fn json(&self) -> Value {
        json!({ "find": "college", "class": self.class, "level": self.level, "pos": self.pos, "state": self.state, "q": self.q, "filters": self.filters, "sure": self.sure.key(), "unseen": self.unseen })
    }
}

/// The default search (as agreed): 3 stars and up on the composite, or
/// from your own state.
fn default_filters(w: &World, j: usize) -> Value {
    let code = w.states.get(w.programs[j].state).map_or(String::new(), |s| s.code.clone());
    json!([{ "k": "cstars", "op": "ge", "v": 3 }, { "k": "state", "op": "is", "v": code, "or": true }])
}

/// What a search found.
pub struct Found<'a> {
    pub ctx: Ctx<'a>,
    pub query: Query,
    pub catalog: Vec<fields::Group>,
    pub matches: Vec<usize>,
    /// Kids your staff knows in the class and level.
    pub known: usize,
    /// Kids who pass the filters but whom your staff hasn't seen (left
    /// out, or shown, by the "not seen" switch).
    pub unseen: usize,
    pub class: i32,
}

fn is_known(w: &World, j: usize, p: &afm_engine::world::Person, board: &HashSet<u32>) -> bool {
    p.stars > 0
        || p.camp_stars > 0
        || staff::has_file(w, Staff::Program(j), p.player.id)
        || !p.cal.offers.is_empty()
        || !p.cal.early_offers.is_empty()
        || p.cal.commit.is_some()
        || p.cal.all_state > 0
        || board.contains(&p.player.id)
        || w.programs[j].hs_pipeline.contains_key(&p.hs)
}

/// Runs a search for your program.
pub fn search<'a>(w: &'a World, j: usize, spec: &Spec) -> Found<'a> {
    let sign = desk::next_class(w);
    let class = spec.class.filter(|c| (sign..=sign + 2).contains(c)).unwrap_or(sign);
    let staff_of = Staff::Program(j);
    let board: HashSet<u32> = w.front_office.college.board.iter().map(|t| t.id).collect();
    let pos = spec.pos.as_deref().and_then(fields::position);
    let mut pool: Vec<usize> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        let ok = match spec.level.as_str() {
            "juco" => {
                p.stage == Stage::College
                    && p.juco
                    && p.cal.signed.is_none()
                    && p.college_seasons >= 1
                    && p.college.is_some_and(|c| w.programs[c].division == Division::JUCO)
            }
            "portal" => p.stage == Stage::College && p.cal.portal.is_some() && p.college != Some(j),
            _ => {
                matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
                    && p.class_of == class
                    && p.cal.signed.is_none_or(|s| s.program == j)
                    && is_known(w, j, p, &board)
            }
        };
        if ok {
            pool.push(i);
        }
    }
    let known = pool.len();
    let code = |i: usize| w.states.get(w.people[i].home_state).map_or("", |s| s.code.as_str());
    pool.retain(|&i| {
        let p = &w.people[i];
        pos.is_none_or(|x| p.player.position == x)
            && spec.state.as_deref().is_none_or(|s| s == code(i))
            && (spec.q.is_empty()
                || p.name.to_lowercase().contains(&spec.q)
                || w.school_name(p.hs).to_lowercase().contains(&spec.q))
    });
    let ids: Vec<u32> = pool.iter().map(|&i| w.people[i].player.id).collect();
    let mut ctx = Ctx::new(w, Seat::College(j), staff_of, spec.sure, true);
    ctx.reads = staff::reads(w, staff_of, &ids);
    if spec.level == "hs" {
        ctx.ranks = Ranks::of_class(w, class);
    }
    let catalog = fields::catalog(Seat::College(j), w);
    let filters = spec.filters.clone().unwrap_or_else(|| default_filters(w, j));
    let query = Query::parse(&filters, &catalog);
    // Every kid who passes with the not-seen counted in, then the switch:
    // left out, a filter on your staff's reads fails for a kid it hasn't
    // seen (FS2).
    let (matches, unseen) = super::apply(&mut ctx, &query, pool, spec.unseen);
    Found { ctx, query, catalog, matches, known, unseen, class }
}

/// The ids a saved search finds today.
pub fn ids(w: &World, j: usize, spec: &Spec) -> Vec<u32> {
    let f = search(w, j, spec);
    f.matches.iter().map(|&i| w.people[i].player.id).collect()
}

/// A class in this school year's words.
pub fn class_words(w: &World, class_of: i32) -> &'static str {
    let seniors = if w.today < w.dates.hs_school_start { w.year } else { w.year + 1 };
    match class_of - seniors {
        ..=0 => "seniors",
        1 => "juniors",
        2 => "sophomores",
        _ => "freshmen",
    }
}

/// The services' last word on the class, in words (F1).
fn update_words(f: &Found) -> String {
    match f.ctx.ranks.date {
        Some(d) => {
            let word = services::update_days(d.year()).iter().find(|u| u.0 == d).map(|u| u.1);
            match word {
                Some(s) => format!("the services' {s} update was {}", short(d)),
                None => format!("the services last rated the class {}", short(d)),
            }
        }
        None => "the services haven't ranked this class yet".into(),
    }
}

/// A date the way the search writes it: "21 Jan".
pub fn short(d: Date) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (_, m, day) = d.parts();
    format!("{day} {}", M[(m - 1) as usize])
}

/// Find recruits: the page.
pub fn find(w: &World, j: usize, args: &Value) -> Reply {
    let spec = Spec::parse(args);
    let mut f = search(w, j, &spec);
    // The players ticked for Compare are read even when today's filters
    // leave them out.
    f.ctx.reads = staff::reads(w, Staff::Program(j), &super::compare_ids(args));
    let ctx = &f.ctx;
    let sort = args.get("sort").and_then(Value::as_str).unwrap_or(if spec.level == "hs" { "nat" } else { "now" });
    let flip = args.get("dir").and_then(Value::as_i64) == Some(-1);
    let sorted = sort_matches(ctx, &f.catalog, &f.matches, sort, flip);
    let page = args.get("page").and_then(Value::as_u64).unwrap_or(0) as usize;
    let rows: Vec<Value> = page_of(&sorted, page).iter().map(|&i| row(ctx, i)).collect();
    // By the composite (F1): stars among the matches, specialists apart.
    let mut dist = [0usize; 5];
    let mut sums = [0usize; 3];
    for &i in &f.matches {
        let p = ctx.p(i);
        let k = if p.player.position.is_specialist() {
            4
        } else {
            match p.stars {
                5 => 0,
                4 => 1,
                3 => 2,
                _ => 3,
            }
        };
        dist[k] += 1;
        if fields::value(ctx, i, "new") == Val::N(1.0) {
            sums[0] += 1;
        }
        if ctx.board.contains(&p.player.id) {
            sums[1] += 1;
        }
        if p.cal.commit.is_some_and(|c| c.0 != j) {
            sums[2] += 1;
        }
    }
    let chart = super::chart(ctx, &f.catalog, &sorted, args, ("t40", "wt_v"));
    let compare = super::compare(ctx, &f.catalog, args, &COMPARE);
    let team = TeamRef::College(j);
    let pr = &w.programs[j];
    let sign = desk::next_class(w);
    let update = update_words(&f);
    let since = ctx.ranks.prev_date.or(ctx.ranks.date).map(short);
    Ok(json!({
        "seat": "college",
        "team": views::team_brief(w, team),
        "class": f.class,
        "classes": [sign, sign + 1, sign + 2],
        "class_words": class_words(w, f.class),
        "level": spec.level,
        "home": w.states.get(pr.state).map(|s| s.code.clone()),
        "states": w.states.iter().map(|s| s.code.clone()).collect::<Vec<_>>(),
        "update": update,
        "since": ctx.ranks.date.map(short),
        "prev_since": since,
        "count": f.matches.len(),
        "count_text": thousands(f.matches.len() as i64),
        "known": f.known,
        "known_text": thousands(f.known as i64),
        "unseen": f.unseen,
        "unseen_text": thousands(f.unseen as i64),
        "unseen_on": spec.unseen,
        "sure": spec.sure.key(),
        "filters": f.query.json(),
        "filter_words": f.query.words(),
        "default": spec.filters.is_none(),
        "catalog": fields::catalog_json(&f.catalog),
        "sort": sort,
        "sort_label": fields::field_of(&f.catalog, sort).map_or_else(|| "Composite rank".to_string(), |x| x.label),
        "dir": if flip { -1 } else { 1 },
        "page": page,
        "pages": sorted.len().div_ceil(super::PAGE).max(1),
        "rows": rows,
        "dist": [["5★", dist[0]], ["4★", dist[1]], ["3★", dist[2]], ["2★ or less", dist[3]], ["Specialists", dist[4]]],
        "sums": { "new": sums[0], "board": sums[1], "elsewhere": sums[2] },
        "chart": chart,
        "compare": compare,
        "services": services_of(w),
        "saved": super::saved::list(w, "college"),
        "recruiters": recruiters(w, j),
        "insiders": insiders(w),
        "next_update": short(services::next_update(w.today).0),
        "next_update_word": services::next_update(w.today).1,
        "spec": spec.json(),
    }))
}

/// The active services by name (F4), in the order of the stars columns.
pub fn services_of(w: &World) -> Vec<Value> {
    w.scouting
        .desks
        .iter()
        .enumerate()
        .filter(|(_, d)| d.kind == DeskKind::Service && d.active(w.year))
        .map(|(k, d)| json!({ "k": k, "name": d.name }))
        .collect()
}

/// Your area recruiters and the states each has.
fn recruiters(w: &World, j: usize) -> Vec<Value> {
    let mut by: Vec<(usize, Vec<String>)> = Vec::new();
    for (s, st) in w.states.iter().enumerate() {
        let Some(c) = afm_engine::world::scouting::dept::area_coach(w, j, s as u8) else { continue };
        match by.iter_mut().find(|x| x.0 == c) {
            Some(x) => x.1.push(st.code.clone()),
            None => by.push((c, vec![st.code.clone()])),
        }
    }
    let home = w.programs[j].state;
    by.sort_by_key(|x| {
        (!x.1.iter().any(|c| Some(c) == w.states.get(home).map(|s| &s.code)), std::cmp::Reverse(x.1.len()))
    });
    by.into_iter().take(8).map(|(c, states)| json!({ "id": c, "name": w.coaches[c].name, "states": states })).collect()
}

/// The insiders and their records (FR5).
pub fn insiders(w: &World) -> Vec<Value> {
    let year = w.year;
    w.scouting
        .desks
        .iter()
        .enumerate()
        .filter(|(_, d)| d.kind == DeskKind::Service && d.active(year) && d.from.max(services::INSIDERS_FROM) <= year)
        .map(|(k, d)| {
            let (n, right) = services::insider_record(w, k, year - 1);
            let (n2, right2) = services::insider_record(w, k, year);
            json!({
                "k": k, "name": services::insider_name(d), "desk": d.name,
                "last": [n, right], "this": [n2, right2],
                "pct": if n > 0 { Some((right as f64 * 100.0 / n as f64).round()) } else { None },
                "open": w.scouting.rankings.picks.iter().filter(|p| p.desk as usize == k).count(),
            })
        })
        .collect()
}

/// The fields Compare lines up (FS20): label, field key.
const COMPARE: [(&str, &str); 16] = [
    ("Composite", "comp"),
    ("National rank", "nat"),
    ("Your read", "now"),
    ("Could play", "proj"),
    ("Height (listed)", "ht"),
    ("Weight (listed)", "wt"),
    ("40", "t40"),
    ("Vertical", "tvj"),
    ("Offers", "offers"),
    ("Officials used", "officials"),
    ("From campus (mi)", "miles"),
    ("GPA", "gpa"),
    ("Work ethic", "chr:0"),
    ("Coachability", "chr:1"),
    ("Room to grow (lb)", "room"),
    ("Games missed hurt", "missed"),
];

/// Schools by the name the page prints (an announced list's neutral order).
pub fn by_name(w: &World, schools: &[u16]) -> Vec<usize> {
    let mut v: Vec<usize> = schools.iter().map(|&s| s as usize).collect();
    v.sort_by(|&a, &b| w.program_name(a).cmp(w.program_name(b)));
    v
}

fn opt_str(v: Val, dp: usize) -> Value {
    match v {
        Val::N(x) => json!(format!("{x:.dp$}")),
        _ => Value::Null,
    }
}

/// One recruit's row: everything the column views show (FR1-FR13).
pub fn row(ctx: &Ctx, i: usize) -> Value {
    let w = ctx.w;
    let p = ctx.p(i);
    let id = p.player.id;
    let j = ctx.program().unwrap_or(0);
    let read = ctx.read(i);
    let now = read.filter(|r| r.now.0 > 0.0).map(|r| {
        let half = (r.now.1 as f64).max(1.0);
        [
            ((r.now.0 as f64) - half).floor().clamp(1.0, 99.0) as u8,
            ((r.now.0 as f64) + half).ceil().clamp(1.0, 99.0) as u8,
        ]
    });
    let proj = read.filter(|r| r.ceiling.0 > 0.0).map(|r| fields::PROJ[fields::tier(ctx, r.ceiling.0 as f64)]);
    let sure = read.filter(|r| r.ceiling.0 > 0.0).map(|r| file::sure_words(r.ceiling.1 as f64));
    let vs = match fields::value(ctx, i, "vs") {
        Val::N(x) => Some(fields::VS[x as usize]),
        Val::Unseen => Some("unseen"),
        _ => None,
    };
    let it = crate::lane_frontoffice::college::interest(w, j, i, super::offered_here(w, id));
    let offers = views::offers_of(p).len();
    let comp = ctx.ranks.comp.get(&id).copied();
    let ps = ctx.ranks.pos_state.get(&id).copied();
    let svc: Vec<Value> = ctx
        .ranks
        .boards
        .iter()
        .map(|(k, m)| json!({ "k": k, "stars": m.get(&id).map(|x| x.0), "rank": m.get(&id).map(|x| x.1) }))
        .collect();
    let tests = fields::testing(ctx, i);
    let tops = w.scouting.rankings.tops.iter().find(|t| t.id == id);
    let ins = fields::insider(ctx, i).map(|x| {
        let d = &w.scouting.desks[x.desk as usize];
        let (n, right) = services::insider_record(w, x.desk as usize, w.year - 1);
        json!({
            "team": views::team_brief(w, TeamRef::College(x.program as usize)), "conf": x.conf, "desk": d.name,
            "name": services::insider_name(d), "k": x.desk, "date": short(x.date),
            "pct": if n > 0 { Some((right as f64 * 100.0 / n as f64).round()) } else { None },
        })
    });
    let area = fields::area_coach(ctx, i).map(|c| json!({ "id": c, "name": w.coaches[c].name }));
    let hs_team = if p.stage == Stage::College {
        p.college.map(|c| views::team_brief(w, TeamRef::College(c)))
    } else {
        Some(views::team_brief(w, TeamRef::HighSchool(p.hs)))
    };
    let class_name = if p.stage == Stage::College {
        p.college.map(|c| w.conference_name(&w.programs[c].conference).to_string())
    } else {
        fields::hs_of(ctx, i).map(|h| views::hs_class(w, h))
    };
    let chr = |k: usize| match fields::value(ctx, i, &format!("chr:{k}")) {
        Val::N(c) => Some(file::trait_word(k, c)),
        _ => None,
    };
    let habit = read.filter(|r| r.film > 0).and_then(|r| {
        afm_engine::tendencies::Tend::ALL
            .iter()
            .filter(|t| t.owned_by(p.player.position))
            .map(|&t| (t, fields::tend_read(ctx, i, t, r)))
            .max_by(|a, b| {
                let d = |x: &afm_engine::tendencies::TendRead| ((x.low as f64 + x.high as f64) / 2.0 - 10.5).abs();
                d(&a.1).total_cmp(&d(&b.1))
            })
            .map(|(t, r)| format!("{}: {}", t.label(), r.word))
    });
    let injuries = {
        let missed: u32 = p.player.history.iter().map(|h| h.games_missed).sum();
        let major = fields::value(ctx, i, "major").num().unwrap_or(0.0) as u32;
        match (missed, major) {
            (0, 0) => "None".to_string(),
            (m, 0) => format!("Missed {m} game{}", if m == 1 { "" } else { "s" }),
            (m, k) => format!("{k} major · missed {m}"),
        }
    };
    let sport = match fields::value(ctx, i, "sport") {
        Val::N(x) if x > 0.0 => {
            let name = fields::SPORTS[x as usize];
            let mark = |k: &str, f: &dyn Fn(f64) -> String| match fields::value(ctx, i, k) {
                Val::N(v) => Some(f(v)),
                _ => None,
            };
            let detail = match name {
                "Track" => mark("m100", &|v| format!("100 m {v:.2}"))
                    .or_else(|| mark("shot", &|v| format!("shot put {v:.1} ft")))
                    .map(|a| match mark("m200", &|v| format!("200 m {v:.2}")) {
                        Some(b) => format!("{a} · {b}"),
                        None => a,
                    }),
                "Basketball" => mark("hoops", &|v| format!("{v:.1} points a game")),
                "Wrestling" => mark("wrestle", &|v| format!("{v:.0} wins")),
                _ => None,
            };
            Some(match detail {
                Some(d) => format!("{name}: {d}"),
                None => name.to_string(),
            })
        }
        _ => None,
    };
    let (ht, wt) = file::listed(&p.player);
    let verified = fields::verified(ctx, i).map(|k| k.name());
    let mut v = json!({
        "id": id,
        "name": p.name,
        "pos": p.player.position.name(),
        "spec": p.player.position.is_specialist(),
        "cls": views::class_text(w, p),
        "class_of": p.class_of,
        "school": hs_team,
        "hs_colors": colors::high_school(p.hs),
        "town": views::hometown(w, p),
        "state": w.states.get(p.home_state).map(|s| s.code.clone()),
        "stars": p.stars,
        "camp_stars": if p.player.position.is_specialist() && p.camp_stars > 0 { Some(p.camp_stars) } else { None },
        "comp": comp.map(|c| format!("{:.4}", c.1)),
        "nat": comp.map(|c| c.0),
        "posrank": ps.map(|x| x.0),
        "staterank": ps.map(|x| x.1),
        "move": fields::movement(ctx, i),
        "new": fields::value(ctx, i, "new") == Val::N(1.0),
        "svc": svc,
        "ht": afm_engine::body::height_text(ht),
        "wt": wt,
        "verified": verified,
        "ht_v": opt_str(fields::value(ctx, i, "ht_v"), 1),
        "wt_v": opt_str(fields::value(ctx, i, "wt_v"), 0),
        "arm": opt_str(fields::value(ctx, i, "arm"), 2),
        "hand": opt_str(fields::value(ctx, i, "hand"), 2),
        "wing": opt_str(fields::value(ctx, i, "wing"), 2),
    });
    let more = json!({
        "tests": tests.as_ref().map(fields::tests_json),
        "read": now,
        "proj": proj,
        "sure": sure,
        "looks": read.map_or(0, |r| r.looks),
        "vs": vs,
        "interest": it.words,
        "tone": it.tone,
        "reason": it.reason,
        "leader": it.leader.map(|x| views::team_brief(w, TeamRef::College(x))),
        "offers": offers,
        "on_board": ctx.board.contains(&id),
    });
    super::merge(&mut v, more);
    let more = json!({
        "committed": p.cal.commit.map(|c| views::team_brief(w, TeamRef::College(c.0))),
        "top": tops.map(|t| json!({ "date": short(t.date), "schools": by_name(w, &t.schools).into_iter().map(|s| views::team_brief(w, TeamRef::College(s))).collect::<Vec<_>>() })),
        "officials": fields::officials_used(ctx, i),
        "visits_you": fields::visits_to_you(ctx, i),
        "camp": fields::CAMP[fields::camp(ctx, i)],
        "followers": p.followers,
        "nil": fields::nil_value(ctx, i),
        "miles": ctx.miles(i).round(),
        "instate": p.home_state == ctx.home,
        "ties": fields::ties(ctx, i),
        "area": area,
        "bond": fields::bond(ctx, i),
        "gpa": format!("{:.1}", fields::gpa_shown(ctx, i)),
        "transcript": read.is_some_and(|r| r.transcript),
        "test": p.cal.test_score,
        "risk": read.map(|_| fields::RISK[fields::risk(ctx, i)]),
        "core": match fields::core_behind(ctx, i) {
            None => "No transcript yet".to_string(),
            Some(0) => "On track".to_string(),
            Some(n) => format!("Behind: {n} short"),
        },
        "work": chr(0),
        "coach": chr(1),
        "habit": habit,
        "age_class": fields::age_for_class(p),
        "young": fields::age_for_class(p).is_some_and(|a| a < 17.25),
        "reclass": fields::reclassified(p),
        "injuries": injuries,
        "classification": class_name,
        "sos": fields::value(ctx, i, "sos").num(),
        "line": fields::stat_line(ctx, i),
        "sport": sport,
        "frame": fields::frame_read(ctx, i).map(|r| json!({ "room": r, "word": fields::FRAME[fields::frame_word(r)] })),
        "insider": ins,
        "kind_text": if p.stage == Stage::College { "College" } else { "High school" },
    });
    super::merge(&mut v, more);
    v
}
