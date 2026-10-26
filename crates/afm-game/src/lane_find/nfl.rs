//! Find prospects: the NFL club's search (FR14 with FS1-FS21).
//!
//! The same search as the college staff's, over the coming two drafts: the
//! prospects of this draft (declared, the seniors, and the juniors your
//! scouts have files on before the deadline), and next year's eligible
//! players your scouts have seen or who started last season. It reads your
//! board and grades, your scouts' files and the public desks: their big
//! boards and mocks.

use std::collections::{HashMap, HashSet};

use afm_engine::player::PlayingTime;
use afm_engine::world::data::Division;
use afm_engine::world::scouting::predraft::{LIST_INTERVIEW, LIST_VISIT};
use afm_engine::world::scouting::readers::Staff;
use afm_engine::world::scouting::{board, file, public};
use afm_engine::world::{Stage, World, franchise};
use serde_json::{Value, json};

use super::fields::{self, Ctx, Draft, Seat, Sure, Val};
use super::query::{Query, thousands};
use super::{page_of, short, sort_matches, staff};
use crate::Reply;
use crate::views::{self, TeamRef};

/// A search as the screen sends it.
pub struct Spec {
    pub class: Option<i32>,
    pub level: String,
    pub pos: Option<String>,
    pub conf: Option<String>,
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
            level: s("level").unwrap_or_else(|| "all".into()),
            pos: s("pos"),
            conf: s("conf"),
            q: s("q").unwrap_or_default().trim().to_lowercase(),
            filters: a.get("filters").filter(|v| v.is_array()).cloned(),
            sure: Sure::parse(a.get("sure").and_then(Value::as_str)),
            unseen: a.get("unseen").and_then(Value::as_bool).unwrap_or(false),
        }
    }

    pub fn json(&self) -> Value {
        json!({ "find": "nfl", "class": self.class, "level": self.level, "pos": self.pos, "conf": self.conf, "q": self.q, "filters": self.filters, "sure": self.sure.key(), "unseen": self.unseen })
    }
}

/// The default search: prospects your scouts have looked at, and in this
/// draft every one with a grade on your board.
fn default_filters(this_draft: bool) -> Value {
    if this_draft {
        json!([{ "k": "looks", "op": "ge", "v": 1 }, { "k": "grade", "op": "le", "v": 8 }])
    } else {
        json!([{ "k": "looks", "op": "ge", "v": 1 }])
    }
}

pub struct Found<'a> {
    pub ctx: Ctx<'a>,
    pub query: Query,
    pub catalog: Vec<fields::Group>,
    pub matches: Vec<usize>,
    pub known: usize,
    pub unseen: usize,
    pub class: i32,
}

/// Whether he started for his college last season.
fn started(p: &afm_engine::world::Person) -> bool {
    p.player.history.last().is_some_and(|h| h.level.is_college() && h.playing_time == PlayingTime::Starter)
}

fn level_ok(w: &World, p: &afm_engine::world::Person, level: &str) -> bool {
    let d = p.college.map(|c| w.programs[c].division);
    match level {
        "fbs" => d == Some(Division::FBS),
        "fcs" => d == Some(Division::FCS),
        "small" => !matches!(d, Some(Division::FBS) | Some(Division::FCS)),
        _ => true,
    }
}

/// The draft's side for the search: your board, its grades and the desks.
fn draft_side(w: &World, year: i32) -> Draft {
    let mut d = Draft { year, ..Default::default() };
    if year != public::draft_year(w) {
        return d;
    }
    let grades = board::seat_grades(w);
    let order = board::ordered(w, &grades);
    let teams = franchise::active_teams(w.year).len().max(1);
    let b = &w.scouting.board;
    d.board = order
        .iter()
        .enumerate()
        .map(|(k, &id)| (id, (k, b.tier_of(id).unwrap_or_else(|| board::tier_of_rank(k, teams)))))
        .collect();
    d.public = board::public_ranks(w);
    let s = &w.scouting.public;
    if s.year == year {
        for (desk, _, ids) in &s.boards {
            let m: HashMap<u32, u32> = ids.iter().enumerate().map(|(k, &id)| (id, k as u32 + 1)).collect();
            d.desks.push((*desk as usize, m));
        }
        for (_, _, ids) in &s.mocks {
            for (k, &id) in ids.iter().enumerate() {
                let pick = k as u16 + 1;
                let e = d.mocks.entry(id).or_insert((pick, pick));
                e.0 = e.0.min(pick);
                e.1 = e.1.max(pick);
            }
        }
    }
    for &(id, list) in &w.scouting.predraft.lists {
        let bit = match list {
            LIST_VISIT => 1,
            LIST_INTERVIEW => 2,
            _ => 0,
        };
        *d.lists.entry(id).or_insert(0) |= bit;
    }
    d
}

/// Runs a search for your club.
pub fn search<'a>(w: &'a World, t: usize, spec: &Spec) -> Found<'a> {
    let dy = public::draft_year(w);
    let class = spec.class.filter(|c| (dy..=dy + 1).contains(c)).unwrap_or(dy);
    let staff_of = Staff::Club(t);
    let this: HashSet<usize> = public::prospects(w).into_iter().collect();
    // Juniors are this draft's until the declaration deadline passes.
    let juniors_open = w.today <= w.dates.declare_deadline || w.today.month() >= 8;
    let mut pool: Vec<usize> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        let ok = if class == dy {
            this.contains(&i)
                || (juniors_open
                    && p.stage == Stage::College
                    && dy - p.class_of >= 3
                    && (p.cal.draft_read.is_some() || staff::has_file(w, staff_of, p.player.id)))
        } else {
            p.stage == Stage::College
                && !this.contains(&i)
                && class - p.class_of >= 3
                && (started(p) || staff::has_file(w, staff_of, p.player.id))
        };
        if ok {
            pool.push(i);
        }
    }
    let known = pool.len();
    let pos = spec.pos.as_deref().and_then(fields::position);
    pool.retain(|&i| {
        let p = &w.people[i];
        pos.is_none_or(|x| p.player.position == x)
            && level_ok(w, p, &spec.level)
            && spec.conf.as_deref().is_none_or(|c| p.college.is_some_and(|k| w.programs[k].conference == c))
            && (spec.q.is_empty()
                || p.name.to_lowercase().contains(&spec.q)
                || p.college.is_some_and(|k| w.program_name(k).to_lowercase().contains(&spec.q)))
    });
    let ids: Vec<u32> = pool.iter().map(|&i| w.people[i].player.id).collect();
    let mut ctx = Ctx::new(w, Seat::Nfl(t), staff_of, spec.sure, true);
    ctx.reads = staff::reads(w, staff_of, &ids);
    ctx.draft = draft_side(w, class);
    if class != dy {
        ctx.dd = afm_engine::calendar::YearDates::new(class);
    }
    let catalog = fields::catalog(Seat::Nfl(t), w);
    let filters = spec.filters.clone().unwrap_or_else(|| default_filters(class == dy));
    let query = Query::parse(&filters, &catalog);
    let (matches, unseen) = super::apply(&mut ctx, &query, pool, spec.unseen);
    Found { ctx, query, catalog, matches, known, unseen, class }
}

/// The ids a saved search finds today.
pub fn ids(w: &World, t: usize, spec: &Spec) -> Vec<u32> {
    let f = search(w, t, spec);
    f.matches.iter().map(|&i| w.people[i].player.id).collect()
}

/// The fields Compare lines up (FS20).
const COMPARE: [(&str, &str); 16] = [
    ("Your board", "board"),
    ("Your grade (tier)", "grade"),
    ("Your read", "now"),
    ("Could be", "proj"),
    ("Consensus", "cons"),
    ("Earliest mock pick", "mock_hi"),
    ("Height, verified", "ht_v"),
    ("Weight, verified", "wt_v"),
    ("Arm length", "arm"),
    ("40", "t40"),
    ("Vertical", "tvj"),
    ("Age at the draft", "age_draft"),
    ("Seasons as a starter", "starts"),
    ("Games missed hurt", "missed"),
    ("Work ethic", "chr:0"),
    ("Medical read", "medical"),
];

/// Find prospects: the page.
pub fn find(w: &World, t: usize, args: &Value) -> Reply {
    let spec = Spec::parse(args);
    let f = search(w, t, &spec);
    let ctx = &f.ctx;
    let dy = public::draft_year(w);
    let this = f.class == dy;
    let sort = args.get("sort").and_then(Value::as_str).unwrap_or(if this { "board" } else { "now" });
    let flip = args.get("dir").and_then(Value::as_i64) == Some(-1);
    let sorted = sort_matches(ctx, &f.catalog, &f.matches, sort, flip);
    let page = args.get("page").and_then(Value::as_u64).unwrap_or(0) as usize;
    let rows: Vec<Value> = page_of(&sorted, page).iter().map(|&i| row(ctx, i)).collect();
    // By the round your board has them in.
    let teams = franchise::active_teams(w.year).len().max(1);
    let picks = if this { board::coming_slots(w).len().max(teams * 7) } else { teams * 7 };
    let mut dist = [0usize; 6];
    let (mut senior_bowl, mut combine, mut seen_week) = (0usize, 0usize, 0usize);
    for &i in &f.matches {
        let p = ctx.p(i);
        let k = match ctx.draft.board.get(&p.player.id) {
            Some(&(r, _)) if r < teams => 0,
            Some(&(r, _)) if r < 2 * teams => 1,
            Some(&(r, _)) if r < 3 * teams => 2,
            Some(&(r, _)) if r < picks => 3,
            Some(_) => 4,
            None => 5,
        };
        dist[k] += 1;
        if let Some(a) = p.cal.all_star.as_ref() {
            seen_week += 1;
            if *a == "Senior Bowl" {
                senior_bowl += 1;
            }
        }
        combine += usize::from(p.cal.combine_invite);
    }
    let visits = w.scouting.predraft.lists.iter().filter(|x| x.1 == LIST_VISIT).count();
    let draft = &ctx.dd.draft[0];
    let team = TeamRef::Nfl(t);
    let chart = super::chart(ctx, &f.catalog, &sorted, args, if this { ("board", "cons") } else { ("t40", "wt_v") });
    let compare = super::compare(ctx, &f.catalog, args, &COMPARE);
    let confs: Vec<String> = {
        let mut v: Vec<String> = w
            .programs
            .iter()
            .filter(|p| matches!(p.division, Division::FBS | Division::FCS))
            .map(|p| p.conference.clone())
            .collect();
        v.sort();
        v.dedup();
        v
    };
    let mut dist_v = vec![
        json!(["Round 1", dist[0]]),
        json!(["Round 2", dist[1]]),
        json!(["Round 3", dist[2]]),
        json!(["Rounds 4-7", dist[3]]),
        json!(["Free agents", dist[4]]),
    ];
    if dist[5] > 0 {
        dist_v.push(json!(["Not graded yet", dist[5]]));
    }
    Ok(json!({
        "seat": "nfl",
        "team": views::team_brief(w, team),
        "class": f.class,
        "classes": [dy, dy + 1],
        "this_draft": this,
        "draft_date": short(draft.date),
        "combine": [short(ctx.dd.combine_start), short(ctx.dd.combine_end)],
        "combine_done": w.today > ctx.dd.combine_end && this,
        "senior_bowl": short(w.dates.senior_bowl),
        "senior_bowl_done": w.today > w.dates.senior_bowl && this,
        "level": spec.level,
        "confs": confs,
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
        "sort_label": fields::field_of(&f.catalog, sort).map_or_else(|| "Your board".to_string(), |x| x.label),
        "dir": if flip { -1 } else { 1 },
        "page": page,
        "pages": sorted.len().div_ceil(super::PAGE).max(1),
        "rows": rows,
        "dist": dist_v,
        "sums": { "senior_bowl": senior_bowl, "all_star": seen_week, "combine": combine, "visits": visits },
        "chart": chart,
        "compare": compare,
        "desks": desks_of(w),
        "saved": super::saved::list(w, "nfl"),
        "spec": spec.json(),
    }))
}

/// The analyst desks working this year, in the order of their columns.
fn desks_of(w: &World) -> Vec<Value> {
    w.scouting
        .desks
        .iter()
        .enumerate()
        .filter(|(_, d)| d.kind == public::DeskKind::Analyst && d.active(w.year))
        .map(|(k, d)| json!({ "k": k, "name": d.outlet }))
        .collect()
}

/// One prospect's row: what the column views show.
pub fn row(ctx: &Ctx, i: usize) -> Value {
    let w = ctx.w;
    let p = ctx.p(i);
    let id = p.player.id;
    let read = ctx.read(i);
    let now = read.filter(|r| r.now.0 > 0.0).map(|r| {
        let half = (r.now.1 as f64).max(1.0);
        [
            ((r.now.0 as f64) - half).floor().clamp(1.0, 99.0) as u8,
            ((r.now.0 as f64) + half).ceil().clamp(1.0, 99.0) as u8,
        ]
    });
    let proj = read.filter(|r| r.ceiling.0 > 0.0).map(|r| fields::PROJ_NFL[fields::tier(ctx, r.ceiling.0 as f64)]);
    let sure = read.filter(|r| r.ceiling.0 > 0.0).map(|r| file::sure_words(r.ceiling.1 as f64));
    let b = ctx.draft.board.get(&id).copied();
    let cons = ctx.draft.public.get(&id).map(|&r| r + 1);
    let vs = b.map(|(k, _)| match board::limb(k, ctx.draft.public.get(&id).copied()) {
        1 => "You're higher",
        -1 => "You're lower",
        _ => "Level",
    });
    let desks: Vec<Value> = ctx.draft.desks.iter().map(|(k, m)| json!({ "k": k, "rank": m.get(&id) })).collect();
    let tests = fields::testing(ctx, i);
    let (ht, wt) = file::listed(&p.player);
    let medical = match read {
        Some(r) if r.medical.1 => fields::MEDICAL[3 - r.medical.0.min(3) as usize],
        _ => "Not examined",
    };
    let character = match fields::value(ctx, i, "chr:0") {
        Val::N(c) => Some(file::trait_word(0, c)),
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
    let missed: u32 = p.player.history.iter().map(|h| h.games_missed).sum();
    let major = fields::value(ctx, i, "major").num().unwrap_or(0.0) as u32;
    let injuries = match (missed, major) {
        (0, 0) => "None".to_string(),
        (m, 0) => format!("Missed {m} game{}", if m == 1 { "" } else { "s" }),
        (m, k) => format!("{k} major · missed {m}"),
    };
    let starts =
        p.player.history.iter().filter(|h| h.level.is_college() && h.playing_time == PlayingTime::Starter).count();
    let seasons = p.player.history.iter().filter(|h| h.level.is_college()).count();
    let lists = ctx.draft.lists.get(&id).copied().unwrap_or(0);
    let mock = ctx
        .draft
        .mocks
        .get(&id)
        .map(|m| if m.0 == m.1 { format!("Pick {}", m.0) } else { format!("Picks {}-{}", m.0, m.1) });
    let conf = p.college.map(|c| {
        let pr = &w.programs[c];
        let name = w.conference_name(&pr.conference).to_string();
        if pr.division == Division::FBS { name } else { format!("{name} ({})", super::division_word(pr.division)) }
    });
    let mut v = json!({
        "id": id,
        "name": p.name,
        "pos": p.player.position.name(),
        "school": p.college.map(|c| views::team_brief(w, TeamRef::College(c))),
        "conf": conf,
        "cls": views::class_text(w, p),
        "age": fields::value(ctx, i, "age_draft").num().map(|a| format!("{a:.1}")),
        "early": fields::value(ctx, i, "early") == Val::N(1.0),
        "board": b.map(|x| x.0 + 1),
        "grade": b.map(|x| board::round_words(x.1)),
        "read": now,
        "proj": proj,
        "sure": sure,
        "looks": read.map_or(0, |r| r.looks),
        "cons": cons,
        "vs": vs,
        "desks": desks,
        "mock": mock,
        "ht": afm_engine::body::height_text(ht),
        "wt": wt,
        "verified": fields::verified(ctx, i).map(|k| k.name()),
        "ht_v": fields::value(ctx, i, "ht_v").num().map(afm_engine::body::height_text),
        "wt_v": fields::value(ctx, i, "wt_v").num(),
        "arm": fields::value(ctx, i, "arm").num().map(|x| format!("{x:.2}")),
        "hand": fields::value(ctx, i, "hand").num().map(|x| format!("{x:.2}")),
    });
    let more = json!({
        "tests": tests.as_ref().map(fields::tests_json),
        "combine": p.cal.combine_invite,
        "all_star": p.cal.all_star.as_ref().map(|a| a.to_string()),
        "starts": if starts > 0 { format!("{seasons} years, {starts} as a starter") } else { format!("{seasons} years") },
        "line": fields::stat_line(ctx, i),
        "habit": habit,
        "injuries": injuries,
        "medical": medical,
        "character": character,
        "trouble": fields::trouble(p),
        "top30": lists & 1 != 0,
        "interview": lists & 2 != 0,
        "level": if p.stage == Stage::Declared { "Declared" } else { "In college" },
    });
    super::merge(&mut v, more);
    v
}
