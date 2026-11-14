//! Plan ahead on the depth chart (Cain, 2026-11-13): the stadium's base
//! boards for each of the next three falls, a private reminder of who you
//! have starting when you recruit. The plan is a file beside the save
//! (`depth_plan.rs`), never part of the world: nothing in the game reads it.
//!
//! A planned fall starts as your chart now. Every man who is still here that
//! fall keeps his place in the order and the men behind him move up; a man
//! whose eligibility (college), class (high school) or contract (NFL) is up
//! by then is out, and a spot nobody fills is open: a place to recruit. At a
//! college your commits arrive in their class's fall, and are offered next
//! to the men who could play the spot, never placed for you. Once you move a
//! man at a position in a fall, that position is yours for that fall; the
//! positions you haven't touched keep following your chart.

use std::collections::{HashMap, HashSet};

use afm_engine::positions::Position;
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::depth::{self, Spot};
use super::needs::{class_in, is_commit, portal_entry};
use super::{mid, my_team, own_read};
use crate::depth_plan::Plan;
use crate::views::{self, TeamRef};

/// How many falls ahead a plan runs.
pub const AHEAD: i32 = 3;

/// The formations a plan covers: the stadium's base boards.
pub(crate) fn planned_form(form: &str) -> bool {
    matches!(form, "offense" | "defense" | "specialists")
}

/// The fall asked for, 1 to `AHEAD` (0 is the chart now).
pub(crate) fn offset_of(args: &Value) -> i32 {
    args.get("offset").and_then(Value::as_i64).unwrap_or(0).clamp(0, AHEAD as i64) as i32
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Avail {
    /// On your roster that fall.
    Here,
    /// A commit whose class has arrived by then (college).
    Commit,
    Gone,
}

/// Whether a man is with you in a fall to come, by the same rules as the
/// needs map: college eligibility (a man in the portal is gone), a high
/// school class, a pro's contract.
pub(crate) fn avail(w: &World, t: TeamRef, i: usize, offset: i32, roster: &[usize]) -> Avail {
    let p = &w.people[i];
    let f0 = w.year;
    let fall = f0 + offset;
    if roster.contains(&i) {
        let here = match p.stage {
            Stage::College => {
                if portal_entry(w, p, true).is_some() {
                    false
                } else {
                    let div = match t {
                        TeamRef::College(j) => w.programs[j].division,
                        _ => afm_engine::world::eligibility::division_of(p, &w.programs),
                    };
                    afm_engine::world::eligibility::seasons_left(p, div, f0) as i32 > offset
                }
            }
            Stage::HighSchool | Stage::PrepSchool => p.class_of > fall,
            _ => {
                let through = p.pro.as_ref().and_then(|c| c.contract.as_ref()).map_or(f0 - 1, |k| k.last_year());
                through >= fall
            }
        };
        if here { Avail::Here } else { Avail::Gone }
    } else if matches!(t, TeamRef::College(_))
        && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
        && is_commit(w, i)
        && p.class_of <= fall
    {
        Avail::Commit
    } else {
        Avail::Gone
    }
}

fn takes(w: &World, t: TeamRef, id: u32, offset: i32, roster: &[usize]) -> bool {
    views::person_index(w, id).is_some_and(|i| avail(w, t, i, offset, roster) != Avail::Gone)
}

/// Fills the spots' men with the plan for a fall: a position you set is
/// your list (a man who is gone by then is a hole); a position you haven't
/// is your chart now with the men who are gone taken out.
pub(crate) fn apply(w: &World, t: TeamRef, offset: i32, plan: &Plan, spots: &mut [Spot]) {
    let roster = views::roster_people(w, t);
    let fall = w.year + offset;
    let order = w.seat_game_depth().unwrap_or_default();
    let mut seen: Vec<Position> = Vec::new();
    for s in spots.iter() {
        if !seen.contains(&s.pos) {
            seen.push(s.pos);
        }
    }
    let widths: HashMap<Position, usize> =
        seen.iter().map(|&p| (p, spots.iter().filter(|s| s.pos == p).count())).collect();
    let width = |p: Position| widths[&p];
    let mut cells: HashMap<Position, Vec<Option<u32>>> = HashMap::new();
    let mut used: HashSet<u32> = HashSet::new();
    for &pos in &seen {
        if let Some(v) = plan.get(fall, pos.name()).filter(|v| v.len() == 3 * width(pos)) {
            let c: Vec<Option<u32>> = v.iter().map(|id| id.filter(|&id| takes(w, t, id, offset, &roster))).collect();
            used.extend(c.iter().flatten());
            cells.insert(pos, c);
        }
    }
    // The rest follow your chart now: every spot's starter, then every
    // spot's second man, then every spot's third, as the chart is read.
    let mut lists: HashMap<Position, Vec<u32>> = HashMap::new();
    for &pos in &seen {
        if cells.contains_key(&pos) {
            continue;
        }
        let list = depth::staff_list(w, t, &roster, &order, &w.seat_depth, pos)
            .into_iter()
            .filter(|&id| views::person_index(w, id).is_some_and(|i| avail(w, t, i, offset, &roster) == Avail::Here))
            .collect();
        lists.insert(pos, list);
        cells.insert(pos, vec![None; 3 * width(pos)]);
    }
    for d in 0..3 {
        for pos in &seen {
            let Some(list) = lists.get(pos) else { continue };
            let wd = width(*pos);
            for r in 0..wd {
                let id = list.iter().copied().find(|id| !used.contains(id));
                if let Some(id) = id {
                    used.insert(id);
                }
                cells.get_mut(pos).expect("filled above")[d * wd + r] = id;
            }
        }
    }
    for &pos in &seen {
        let wd = width(pos);
        let c = &cells[&pos];
        for (r, s) in spots.iter_mut().filter(|s| s.pos == pos).enumerate() {
            for d in 0..3 {
                s.ids[d] = c[d * wd + r];
            }
        }
    }
}

/// Who a planned view needs beyond the spots: the roster, and the men who
/// are there that fall and not on this chart, with the commits (the
/// side panel's "who else could play it").
pub(crate) struct Ctx {
    pool: Vec<(usize, bool)>,
}

impl Ctx {
    pub(crate) fn new(w: &World, t: TeamRef, offset: i32, spots: &[Spot]) -> Ctx {
        let roster = views::roster_people(w, t);
        let placed: HashSet<u32> = spots.iter().flat_map(|s| s.ids.iter().flatten().copied()).collect();
        let mut pool: Vec<(usize, bool)> = roster
            .iter()
            .copied()
            .filter(|&i| !placed.contains(&w.people[i].player.id) && avail(w, t, i, offset, &roster) == Avail::Here)
            .map(|i| (i, false))
            .collect();
        if matches!(t, TeamRef::College(_)) {
            for (i, p) in w.people.iter().enumerate() {
                if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
                    && !placed.contains(&p.player.id)
                    && avail(w, t, i, offset, &roster) == Avail::Commit
                {
                    pool.push((i, true));
                }
            }
        }
        Ctx { pool }
    }
}

/// His class in words for a fall to come: "Junior", "Redshirt sophomore".
fn class_words(short: &str) -> String {
    let rs = short.starts_with("RS ");
    let base = match short.trim_start_matches("RS ") {
        "Fr" => "freshman",
        "So" => "sophomore",
        "Jr" => "junior",
        "Sr" => "senior",
        other => return other.to_string(),
    };
    if rs { format!("Redshirt {base}") } else { super::cap(base) }
}

/// One man on a planned spot: his class in that fall, never a health tag
/// (nobody knows who is hurt that far out), a commit marked.
pub(crate) fn man(w: &World, t: TeamRef, i: usize, num: Option<u8>, offset: i32) -> Value {
    let mut v = depth::man(w, t, i, num);
    let short = class_in(w, i, w.year + offset);
    let commit = !views::roster_people(w, t).contains(&i);
    v["class_full"] = json!(class_words(&short));
    v["class"] = json!(short);
    v["health"] = json!("full");
    v["health_why"] = json!("");
    v["ready"] = json!(if commit { "Commit" } else { "Still here" });
    v["commit"] = json!(commit);
    v
}

/// The side panel's "who else could play it" for a planned spot: the men
/// who are there that fall and not on the chart, the position's own first,
/// with your commits marked. No "why is he here": nobody put him there yet.
pub(crate) fn why(w: &World, t: TeamRef, ctx: &Ctx, pos: Position, spot: &str, offset: i32) -> Value {
    let (one, _) = crate::lane_ratings::pos_words(pos);
    let group = afm_engine::spots::group;
    let fall = w.year + offset;
    let mut alts: Vec<(bool, f64, usize, bool)> = ctx
        .pool
        .iter()
        .filter(|&&(i, _)| {
            let q = w.people[i].player.position;
            q == pos || (!q.is_specialist() && !pos.is_specialist() && group(q) == group(pos))
        })
        .map(|&(i, c)| {
            let q = w.people[i].player.position;
            (q != pos, -mid(own_read(w, t, i)).unwrap_or(0.0), i, c)
        })
        .collect();
    alts.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    alts.truncate(6);
    let rows: Vec<Value> = alts
        .iter()
        .map(|&(cross, _, i, commit)| {
            let q = &w.people[i];
            let note = if commit {
                format!("Commit, in your {} class. Arrives by fall {fall}.", q.class_of)
            } else if cross {
                format!("Plays {}: he would learn the spot.", q.player.position.name())
            } else {
                format!("Still here in fall {fall}.")
            };
            json!({
                "id": q.player.id, "name": q.name, "pos": q.player.position.name(),
                "class": class_in(w, i, fall), "read": own_read(w, t, i).json(), "note": note,
                "hurt": false, "commit": commit,
            })
        })
        .collect();
    json!({
        "plan": true, "title": spot, "sub": "", "read": Value::Null,
        "by": Value::Null, "by_name": "", "when": "",
        "why": [], "else_title": format!("Who else could play {one}"),
        "alts": rows, "practice": [],
    })
}

// ---------- changes ----------

/// The planned spots at a formation, as the screen shows them.
fn planned(w: &World, t: TeamRef, form: &str, offset: i32, plan: &Plan) -> Result<Vec<Spot>, String> {
    if !planned_form(form) {
        return Err("Plan ahead covers the base offense, the base defense and the specialists.".into());
    }
    if !(1..=AHEAD).contains(&offset) {
        return Err("Pick one of the next three falls.".into());
    }
    let (mut spots, _) = depth::spots(w, t, form)?;
    apply(w, t, offset, plan, &mut spots);
    Ok(spots)
}

/// Puts a man into one cell of a planned fall: spot `slot`, depth `n`.
/// The same swaps as the chart now (`depth::put`): two cells at one position
/// swap, a man from outside takes the cell, a man from another position's
/// cell (the same group) leaves it to the next man there. Only a man who is
/// with you that fall (a commit counts), only in his own position group, only
/// in a spot you set.
pub(crate) fn put(w: &World, plan: &mut Plan, args: &Value) -> Result<Option<String>, String> {
    let t = my_team(w)?;
    let form = depth::form_key(args);
    let offset = offset_of(args);
    let fall = w.year + offset;
    let spots = planned(w, t, form, offset, plan)?;
    let k = args.get("slot").and_then(Value::as_u64).ok_or("Missing spot.")? as usize;
    let n = args.get("n").and_then(Value::as_u64).ok_or("Missing depth.")? as usize;
    let pid = args.get("pid").and_then(Value::as_u64).ok_or("Missing player.")? as u32;
    if !(1..=3).contains(&n) {
        return Err("A spot is three deep: the starter and two behind him.".into());
    }
    let d = n - 1;
    let target = spots.get(k).ok_or("No such spot.")?;
    let roster = views::roster_people(w, t);
    let mi = views::person_index(w, pid).ok_or("He isn't on your roster.")?;
    let name = w.people[mi].name.clone();
    if avail(w, t, mi, offset, &roster) == Avail::Gone {
        return Err(format!("{name} won't be with you in fall {fall}, so he can't be planned there."));
    }
    let mpos = w.people[mi].player.position;
    let locked = |s: &Spot| {
        format!("{} is the head coach's call, so it isn't yours to set. Your job covers {}.", s.slot, w.your_controls())
    };
    if !target.controls {
        return Err(locked(target));
    }
    let named = args.get("from_slot").and_then(Value::as_u64).zip(args.get("from_n").and_then(Value::as_u64));
    let src = named
        .map(|(ks, f)| (ks as usize, (f as usize).wrapping_sub(1)))
        .filter(|&(ks, ds)| spots.get(ks).is_some_and(|s| s.ids.get(ds) == Some(&Some(pid))))
        .or_else(|| {
            spots.iter().enumerate().find_map(|(ks, s)| s.ids.iter().position(|x| *x == Some(pid)).map(|ds| (ks, ds)))
        });
    let group = afm_engine::spots::group;
    if group(mpos) != group(target.pos) && !src.is_some_and(|(ks, _)| group(spots[ks].pos) == group(target.pos)) {
        return Err(format!(
            "{name} plays {}, so he can't take {} ({}). A man moves only inside his own position group.",
            mpos.name(),
            target.slot,
            target.pos.name()
        ));
    }
    if src == Some((k, d)) {
        return Ok(Some(format!(
            "{name} is already {} at {} in fall {fall}, so nothing changed.",
            depth::depth_word(n),
            target.slot
        )));
    }
    if let Some((ks, _)) = src
        && !spots[ks].controls
    {
        return Err(locked(&spots[ks]));
    }
    let pos = target.pos;
    let mut writes: Vec<(Position, Vec<Option<u32>>)> = Vec::new();
    let mut cp = depth::pos_cells(&spots, pos);
    let b = depth::cell_at(&spots, k, d);
    match src {
        Some((ks, ds)) if spots[ks].pos == pos => {
            let a = depth::cell_at(&spots, ks, ds);
            cp.swap(a, b);
        }
        Some((ks, ds)) => {
            let q = spots[ks].pos;
            let mut cq = depth::pos_cells(&spots, q);
            cq[depth::cell_at(&spots, ks, ds)] = None;
            writes.push((q, cq));
            cp[b] = Some(pid);
        }
        None => cp[b] = Some(pid),
    }
    writes.push((pos, cp));
    for (p, _) in &writes {
        if !w.controls(*p) {
            return Err(format!("{} is not yours to set. Your job covers {}.", p.name(), w.your_controls()));
        }
    }
    for (p, c) in writes {
        plan.set(fall, p.name(), c);
    }
    plan.prune(w.year);
    Ok(None)
}

/// Gives a formation's positions back to your chart now, in one fall.
pub(crate) fn reset(w: &World, plan: &mut Plan, args: &Value) -> Result<(), String> {
    let t = my_team(w)?;
    let form = depth::form_key(args);
    let offset = offset_of(args);
    if !planned_form(form) || offset == 0 {
        return Err("Plan ahead covers the next three falls of the base boards.".into());
    }
    let (spots, _) = depth::spots(w, t, form)?;
    let mut names: Vec<&str> = spots.iter().map(|s| s.pos.name()).collect();
    names.dedup();
    plan.clear(w.year + offset, &names);
    Ok(())
}

/// Whether any position of a formation is yours in a fall (the reset button shows).
pub(crate) fn is_yours(w: &World, plan: &Plan, spots: &[Spot], offset: i32) -> bool {
    let names: Vec<&str> = spots.iter().map(|s| s.pos.name()).collect();
    plan.has_any(w.year + offset, &names)
}
