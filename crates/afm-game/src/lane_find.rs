//! Lane commands: finding players as a real scouting search (update 2,
//! lane 7). `call` answers the commands this lane owns and returns None
//! for any other.
//!
//! - `find_recruits` (FR1-FR13, FS1-FS21, minor bugs 12-16): a college
//!   staff's search over every kid it knows, on the services' public word,
//!   the kids' public numbers and your staff's own files.
//! - `find_prospects` (FR14): the NFL club's search over the coming
//!   drafts, on its board, its scouts' files and the public desks.
//! - `colleges_calling` (FR15): the high school coach's side, where the
//!   colleges come to you.
//! - `find_offers` and `find_insider`: the drawers a row opens (a kid's
//!   offers, an insider's picks and record).
//! - `find_save` and `find_forget` (FS19): saved searches, told through
//!   the inbox after a services update, a camp or a big game.
//! - `recruit_race` (board 8): a recruit's race month by month and the
//!   tug of war of his reasons.
//!
//! Every filter and column reads what your staff knows, never a true
//! rating: a kid your staff hasn't seen is "not seen", and his ceiling is
//! only ever a word.

mod college;
mod fields;
mod nfl;
mod query;
mod race;
mod saved;
mod school;
pub(crate) mod staff;

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::front_office::college as desk;
use afm_engine::world::scouting::services;
use serde_json::{Value, json};

use crate::views::{self, TeamRef};
use crate::{Game, Reply};
use fields::{Ctx, FKind, Group, Val};
use query::fmt_num;

pub use saved::after_days;

const COMMANDS: [&str; 8] = [
    "find_recruits",
    "find_prospects",
    "colleges_calling",
    "find_offers",
    "find_insider",
    "find_save",
    "find_forget",
    "recruit_race",
];

/// Rows on a page of results.
pub const PAGE: usize = 50;

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    Some(run(game, cmd, args))
}

fn run(game: &mut Game, cmd: &str, args: &Value) -> Reply {
    // A page never changes the world: the services rate only on their own
    // days (an older file caught up once, when it loaded).
    let w = game.world.as_mut().ok_or("No world is loaded.")?;
    match cmd {
        "find_save" => return saved::save(w, args),
        "find_forget" => return saved::forget(w, args),
        "find_recruits" => {
            desk::ready(w);
        }
        _ => {}
    }
    // Opening a saved search: what its last mail named is no longer new.
    if let Some(k) = args.get("saved").and_then(Value::as_u64) {
        saved::opened(w, k as usize);
    }
    let w: &World = w;
    match cmd {
        "find_recruits" => college::find(w, college_seat(w)?, args),
        "find_prospects" => nfl::find(w, nfl_seat(w)?, args),
        "colleges_calling" => school::calling(w, args),
        "find_offers" => offers(w, args),
        "recruit_race" => race::page(w, args),
        _ => insider(w, args),
    }
}

fn college_seat(w: &World) -> Result<usize, String> {
    match views::mine(w) {
        Some(TeamRef::College(j)) => Ok(j),
        _ => Err("Find recruits is for a college head coach.".into()),
    }
}

fn nfl_seat(w: &World) -> Result<usize, String> {
    match views::mine(w) {
        Some(TeamRef::Nfl(t)) => Ok(t),
        _ => Err("Find prospects is for an NFL club.".into()),
    }
}

/// Runs the filters over a pool (FS1, FS2): every player who passes with
/// the not-seen let through, then, unless the switch includes them, the
/// ones a filter on your staff's reads can't read. Returns the matches and
/// how many were left out for not being seen.
pub fn apply(ctx: &mut Ctx, query: &query::Query, pool: Vec<usize>, include: bool) -> (Vec<usize>, usize) {
    ctx.unseen = true;
    let query = &query.settled(ctx, &pool);
    let passed: Vec<usize> = pool.into_iter().filter(|&i| query.pass(ctx, i)).collect();
    ctx.unseen = false;
    if !query.reads_staff() {
        ctx.unseen = include;
        return (passed, 0);
    }
    let seen: Vec<usize> = passed.iter().copied().filter(|&i| query.pass(ctx, i)).collect();
    let left_out = passed.len() - seen.len();
    ctx.unseen = include;
    (if include { passed } else { seen }, left_out)
}

/// Adds the fields of `more` to the object `v` (a row too big for one
/// `json!`).
pub fn merge(v: &mut Value, more: Value) {
    if let (Value::Object(a), Value::Object(b)) = (v, more) {
        a.extend(b);
    }
}

/// One page of the sorted results.
pub fn page_of(sorted: &[usize], page: usize) -> &[usize] {
    let from = (page * PAGE).min(sorted.len());
    &sorted[from..(from + PAGE).min(sorted.len())]
}

/// A value's place in a sort: what is known first, best first.
fn sort_key(v: &Val, lower: bool) -> (u8, f64, String) {
    match v {
        Val::N(x) => (0, if lower { *x } else { -*x }, String::new()),
        Val::T(s) => (0, 0.0, s.to_lowercase()),
        Val::None => (1, 0.0, String::new()),
        Val::Unseen => (2, 0.0, String::new()),
    }
}

/// The matches sorted by a field (FS1, minor bug 14): best first, or worst
/// first with `flip`; players with nothing known for it go last either
/// way, then the services' stars and the player's id settle ties.
pub fn sort_matches(ctx: &Ctx, catalog: &[Group], matches: &[usize], key: &str, flip: bool) -> Vec<usize> {
    let lower = fields::field_of(catalog, key).is_some_and(|f| f.lower) || key == "name";
    let mut keyed: Vec<((u8, f64, String), usize)> = matches
        .iter()
        .map(|&i| {
            let v = if key == "name" { Val::T(ctx.p(i).name.clone()) } else { fields::value(ctx, i, key) };
            (sort_key(&v, lower), i)
        })
        .collect();
    keyed.sort_by(|a, b| {
        let (ka, kb) = (&a.0, &b.0);
        let known = ka.0.cmp(&kb.0);
        let mut by = ka.1.total_cmp(&kb.1).then_with(|| ka.2.cmp(&kb.2));
        if flip {
            by = by.reverse();
        }
        let (pa, pb) = (ctx.p(a.1), ctx.p(b.1));
        known.then(by).then(pb.stars.cmp(&pa.stars)).then(pa.player.id.cmp(&pb.player.id))
    });
    keyed.into_iter().map(|x| x.1).collect()
}

/// A value as the page writes it.
pub fn shown(f: &fields::Field, v: &Val) -> String {
    match v {
        Val::N(x) => match f.kind {
            FKind::Word => f.opts.get(*x as usize).map_or_else(|| fmt_num(*x, 0), |s| s.to_string()),
            FKind::Flag => if *x >= 0.5 { "Yes" } else { "No" }.to_string(),
            _ => fmt_num(*x, f.dp),
        },
        Val::T(s) if s.is_empty() => "—".into(),
        Val::T(s) => s.replace('\n', ", "),
        Val::None => "—".into(),
        Val::Unseen => "Not seen".into(),
    }
}

/// The chart (FS21): any two numeric fields against each other for the
/// best of the matches by the sort, your picked players in gold.
pub fn chart(ctx: &Ctx, catalog: &[Group], sorted: &[usize], args: &Value, defaults: (&str, &str)) -> Value {
    const MOST: usize = 600;
    let pick = |k: &str, d: &str| -> fields::Field {
        args.get(k)
            .and_then(Value::as_str)
            .and_then(|s| fields::field_of(catalog, s))
            .filter(|f| f.kind == FKind::Num)
            .or_else(|| fields::field_of(catalog, d))
            .expect("a default chart field is in the catalog")
    };
    let (fx, fy) = (pick("x", defaults.0), pick("y", defaults.1));
    let picked: Vec<u64> = args
        .get("picked")
        .and_then(Value::as_array)
        .map_or(Vec::new(), |a| a.iter().filter_map(Value::as_u64).collect());
    let mut points = Vec::new();
    let mut missing = 0usize;
    for &i in sorted.iter().take(MOST) {
        match (fields::value(ctx, i, &fx.k), fields::value(ctx, i, &fy.k)) {
            (Val::N(x), Val::N(y)) => {
                let p = ctx.p(i);
                points.push(json!({
                    "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "x": x, "y": y,
                    "picked": picked.contains(&(p.player.id as u64)),
                    "xs": shown(&fx, &Val::N(x)), "ys": shown(&fy, &Val::N(y)),
                }));
            }
            _ => missing += 1,
        }
    }
    json!({
        "x": fx.json(), "y": fy.json(), "points": points, "missing": missing,
        "of": sorted.len().min(MOST), "total": sorted.len(),
    })
}

/// The players ticked for Compare (at most five).
pub fn compare_ids(args: &Value) -> Vec<u32> {
    args.get("compare")
        .and_then(Value::as_array)
        .map_or(Vec::new(), |a| a.iter().filter_map(Value::as_u64).map(|x| x as u32).take(5).collect())
}

/// Compare (FS20): two to five players side by side on the fields given,
/// the best of each line marked. The caller reads the ticked players' files
/// into the context first (they may sit outside today's filters).
pub fn compare(ctx: &Ctx, catalog: &[Group], args: &Value, lines: &[(&str, &str)]) -> Value {
    let at: Vec<usize> = compare_ids(args).iter().filter_map(|&id| views::person_index(ctx.w, id)).collect();
    if at.len() < 2 {
        return json!({ "players": at.iter().map(|&i| who(ctx, i)).collect::<Vec<_>>(), "rows": [] });
    }
    let rows: Vec<Value> = lines
        .iter()
        .filter_map(|&(label, k)| {
            let f = fields::field_of(catalog, k)?;
            let vals: Vec<Val> = at.iter().map(|&i| fields::value(ctx, i, k)).collect();
            let nums: Vec<Option<f64>> = vals.iter().map(Val::num).collect();
            let best = if matches!(f.kind, FKind::Num | FKind::Word) && nums.iter().filter(|x| x.is_some()).count() >= 2 {
                let key = |x: f64| if f.lower { -x } else { x };
                let top = nums.iter().flatten().map(|&x| key(x)).fold(f64::MIN, f64::max);
                let n = nums.iter().filter(|x| x.is_some_and(|v| (key(v) - top).abs() < 1e-9)).count();
                if n == 1 { nums.iter().position(|x| x.is_some_and(|v| (key(v) - top).abs() < 1e-9)) } else { None }
            } else {
                None
            };
            let text: Vec<String> = at
                .iter()
                .zip(&vals)
                .map(|(&i, v)| if k == "now" { now_range(ctx, i).unwrap_or_else(|| shown(&f, v)) } else { shown(&f, v) })
                .collect();
            Some(json!({ "label": label, "k": k, "vals": text, "best": best, "unseen": vals.iter().map(|v| *v == Val::Unseen).collect::<Vec<_>>() }))
        })
        .collect();
    json!({ "players": at.iter().map(|&i| who(ctx, i)).collect::<Vec<_>>(), "rows": rows })
}

fn who(ctx: &Ctx, i: usize) -> Value {
    let w = ctx.w;
    let p = ctx.p(i);
    let team = views::team_of(w, p).map(|t| views::team_brief(w, t));
    json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "team": team, "cls": views::class_text(w, p) })
}

/// Your staff's overall read as a range ("62-70"), when it has one.
pub fn now_range(ctx: &Ctx, i: usize) -> Option<String> {
    let r = ctx.read(i).filter(|r| r.now.0 > 0.0)?;
    let half = (r.now.1 as f64).max(1.0);
    let lo = (r.now.0 as f64 - half).floor().clamp(1.0, 99.0);
    let hi = (r.now.0 as f64 + half).ceil().clamp(1.0, 99.0);
    Some(format!("{lo:.0}-{hi:.0}"))
}

/// A date the way the search writes it: "21 Jan".
pub fn short(d: Date) -> String {
    college::short(d)
}

/// A kid's offers (minor bug 16): every school, its level and conference,
/// with the dates the early ones came. In a public order, never his own:
/// the schools on his announced list first, then by when they offered,
/// then by name.
fn offers(w: &World, args: &Value) -> Reply {
    let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
    let i = views::person_index(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let mut list: Vec<(usize, Option<Date>)> = Vec::new();
    for &(c, d) in &p.cal.early_offers {
        list.push((c, Some(d)));
    }
    for &c in &p.cal.offers {
        if !list.iter().any(|x| x.0 == c) {
            list.push((c, None));
        }
    }
    let mine = match views::mine(w) {
        Some(TeamRef::College(j)) => Some(j),
        _ => None,
    };
    if let Some(j) = mine
        && offered_here(w, id)
        && !list.iter().any(|x| x.0 == j)
    {
        let d = w.front_office.college.board.iter().find(|t| t.id == id).and_then(|t| t.offered);
        list.push((j, d));
    }
    let top = w.scouting.rankings.tops.iter().find(|t| t.id == id);
    let listed = |c: usize| top.is_some_and(|t| t.schools.contains(&(c as u16)));
    list.sort_by(|a, b| {
        listed(b.0)
            .cmp(&listed(a.0))
            .then(a.1.is_none().cmp(&b.1.is_none()))
            .then(a.1.cmp(&b.1))
            .then_with(|| w.program_name(a.0).cmp(w.program_name(b.0)))
    });
    let commit = p.cal.commit.map(|c| c.0).or(p.cal.signed.map(|s| s.program));
    let rows: Vec<Value> = list
        .iter()
        .map(|&(c, d)| {
            let pr = &w.programs[c];
            json!({
                "team": views::team_brief(w, TeamRef::College(c)),
                "level": division_word(pr.division),
                "conf": w.conference_name(&pr.conference),
                "date": d.map(short),
                "yours": Some(c) == mine,
                "committed": Some(c) == commit,
                "listed": listed(c),
                "state": w.states.get(pr.state).map(|s| s.code.clone()),
            })
        })
        .collect();
    let note = if top.is_some() {
        "His announced list first, then by when they offered. Dates are known for the offers made before his junior year's spring."
    } else {
        "By when they offered, then by name. Dates are known for the offers made before his junior year's spring."
    };
    Ok(json!({
        "id": id, "name": p.name, "pos": p.player.position.name(), "cls": views::class_text(w, p),
        "offers": rows, "count": rows.len(), "note": note,
    }))
}

/// Whether your program has a standing offer out to him.
pub fn offered_here(w: &World, id: u32) -> bool {
    w.front_office.college.board.iter().any(|t| t.id == id && t.offered.is_some() && t.pulled.is_none())
}

pub fn division_word(d: afm_engine::world::data::Division) -> &'static str {
    use afm_engine::world::data::Division as D;
    match d {
        D::FBS => "FBS",
        D::FCS => "FCS",
        D::JUCO => "JUCO",
        _ => "D2 and below",
    }
}

/// An insider's picks and record (FR5): every open pick of one desk's
/// insider, or every insider's pick on one kid.
fn insider(w: &World, args: &Value) -> Reply {
    let year = w.year;
    let pick_row = |x: &services::Pick| {
        let i = views::person_index(w, x.id);
        json!({
            "player": i.map(|i| { let p = &w.people[i]; json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "cls": p.class_of, "stars": p.stars }) }),
            "team": views::team_brief(w, TeamRef::College(x.program as usize)),
            "conf": x.conf,
            "date": short(x.date),
            "desk": w.scouting.desks.get(x.desk as usize).map(|d| d.name.clone()),
            "name": w.scouting.desks.get(x.desk as usize).map(services::insider_name),
        })
    };
    if let Some(id) = args.get("id").and_then(Value::as_u64) {
        let picks: Vec<Value> = w.scouting.rankings.picks.iter().filter(|x| x.id as u64 == id).map(pick_row).collect();
        return Ok(json!({ "id": id, "picks": picks }));
    }
    let k = args.get("desk").and_then(Value::as_u64).ok_or("Missing desk.")? as usize;
    let d = w.scouting.desks.get(k).ok_or("No such desk.")?;
    let (n, right) = services::insider_record(w, k, year - 1);
    let (n2, right2) = services::insider_record(w, k, year);
    let mut picks: Vec<&services::Pick> = w.scouting.rankings.picks.iter().filter(|x| x.desk as usize == k).collect();
    picks.sort_by_key(|p| std::cmp::Reverse(p.date));
    Ok(json!({
        "desk": d.name, "name": services::insider_name(d),
        "last": [n, right], "this": [n2, right2],
        "pct": if n > 0 { Some((right as f64 * 100.0 / n as f64).round()) } else { None },
        "picks": picks.into_iter().take(60).map(pick_row).collect::<Vec<_>>(),
        "note": "A pick counts as right when the kid signs where the insider said, at the pick's last word before he decided.",
    }))
}

#[cfg(test)]
mod tests;
