//! The Attributes page's data (`attr_view`): one player's 67 skills the way
//! the page draws them (`docs/reference/how-it-works/ratings.md`, "The
//! Attributes page's data").
//!
//! - Which skills are in play at his position: the eight skill families
//!   plus "Also in play" with a reason each (`lists.rs`).
//! - Each skill's two reads, your own and your staff's. Your own read judges
//!   wherever there is one; your staff's stands in only where there is none
//!   (`judge`). Colour, band words, the strip and the arrows all use the
//!   judging read, so they compare like with like.
//! - The marks for the skill at his level and position (backup, starter,
//!   elite), asked here and cached for the season, never in `player()`.
//! - Where each skill counts toward his overall (`positions::weight`).
//! - How the judging read moved since the last season it was kept
//!   (`Career::kept_attrs_of`, compared with its own source).
//! - With a second player (`with`): his rows the same way and the gaps.
//!
//! Potential stays words: the staff's line, and "your read" is empty until
//! the game forms one (`strip.potential.mine` is null).

mod lists;
pub(crate) use lists::list;

use std::collections::HashSet;

use afm_engine::attributes::{ATTRIBUTE_COUNT, Attr, Category};
use afm_engine::positions::{self, Position};
use afm_engine::world::norms::{self, Marks, Pool, Rated};
use afm_engine::world::seat::KeptAttrs;
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::{band, marks_cached, pos_words, who_phrase};
use crate::reads;
use crate::{Reply, views};

/// How far a kept middle must move to count as moved (as on the range
/// card's "Up a little" / "Down a little", `lane_help.rs`, `last_year`).
const MOVE: f64 = 1.5;

/// "Counts about 4% of an edge rusher's overall rating."
pub(crate) fn weight_line(pos: Position, a: Attr) -> String {
    let one = pos_words(pos).0;
    let art = if one.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
    let w = positions::weight(pos, a) * 100.0;
    if w < 0.05 {
        format!("Doesn't count toward {art} {one}'s overall rating.")
    } else if w < 1.0 {
        format!("Counts for less than 1% of {art} {one}'s overall rating.")
    } else {
        let n = if w >= 10.0 { format!("{w:.0}") } else { format!("{:.1}", w).trim_end_matches(".0").to_string() };
        format!("Counts about {n}% of {art} {one}'s overall rating.")
    }
}

fn group_name(c: Category) -> &'static str {
    match c {
        Category::Physical => "Physical",
        Category::Passing => "Passing",
        Category::Running => "Running",
        Category::Receiving => "Receiving",
        Category::LinePlay => "Line play",
        Category::Defense => "Defense",
        Category::KickingAndSnapping => "Kicking and snapping",
        Category::Mental => "Mental",
    }
}

fn mid_of((lo, hi): (u8, u8)) -> f64 {
    (lo as f64 + hi as f64) / 2.0
}

fn range_json(r: Option<(u8, u8)>) -> Value {
    r.map_or(Value::Null, |(lo, hi)| json!([lo, hi]))
}

/// The skills in play at a position: the families' skills, then the
/// "also in play" ones.
fn in_play(pos: Position) -> HashSet<Attr> {
    let l = list(pos);
    l.core.iter().copied().chain(l.also.iter().map(|x| x.0)).collect()
}

/// Which read judges a skill: your own where you have one, your staff's
/// only where you have none ("mine", "staff" or "none").
pub(crate) fn judge_of(mine: Option<(u8, u8)>, staff: Option<(u8, u8)>) -> (&'static str, Option<(u8, u8)>) {
    match (mine, staff) {
        (Some(r), _) => ("mine", Some(r)),
        (None, Some(r)) => ("staff", Some(r)),
        (None, None) => ("none", None),
    }
}

/// How the judging read moved since the last kept season, comparing each
/// source with its own last season (None: nothing kept, or that skill was
/// not seen then).
pub(crate) fn moved(now_mid: f64, last: Option<&KeptAttrs>, mine: bool, a: Attr) -> Option<Value> {
    let last = last?;
    // Kept in half points: read it through `mid`, not the raw byte.
    let then = last.mid(mine, a.index())?;
    let d = now_mid - then;
    let dir = if d >= MOVE {
        "up"
    } else if d <= -MOVE {
        "down"
    } else {
        "same"
    };
    Some(json!({ "dir": dir, "by": d.round() as i32, "from_season": last.season }))
}

/// One player's rows, and what the strip and the gaps need from them.
struct Rows {
    rows: Vec<Value>,
    /// Per skill: the judging read's middle (None: not seen).
    mids: Vec<Option<f64>>,
    /// Per skill: how far the judging middle is above his position's
    /// starter mark (None without a read or marks).
    gaps: Vec<Option<f64>>,
    /// Per skill: whether it is in play at his position.
    play: Vec<bool>,
    /// Per skill: the move, as (up, size).
    moves: Vec<Option<(bool, i32)>>,
    pool: Option<Pool>,
}

fn rows_of(w: &World, k: usize) -> Rows {
    let p = &w.people[k];
    let id = p.player.id;
    let pos = p.player.position;
    let play = in_play(pos);
    let staff = reads::attributes(w, k);
    let mine = reads::mine_attributes(w, k);
    let pool = norms::pool_of(w, k);
    let last = w.career.as_ref().and_then(|c| c.kept_attrs_of(id).last());
    let mut out = Rows {
        rows: Vec::with_capacity(ATTRIBUTE_COUNT),
        mids: vec![None; ATTRIBUTE_COUNT],
        gaps: vec![None; ATTRIBUTE_COUNT],
        play: vec![false; ATTRIBUTE_COUNT],
        moves: vec![None; ATTRIBUTE_COUNT],
        pool,
    };
    for (i, &a) in Attr::ALL.iter().enumerate() {
        let s = match staff[i].1 {
            reads::Read::Range(lo, hi) => Some((lo, hi)),
            reads::Read::NotSeen => None,
        };
        let m = match mine[i].1 {
            reads::Read::Range(lo, hi) => Some((lo, hi)),
            reads::Read::NotSeen => None,
        };
        let (judge, jr) = judge_of(m, s);
        let marks: Option<Marks> = pool.and_then(|pl| marks_cached(w, pl, pos, Rated::Attr(a)));
        let mid = jr.map(mid_of);
        let band_words = match (&marks, jr) {
            (Some(mk), Some((lo, hi))) => {
                let who = who_phrase(w, mk, k, pos);
                Some(band(mk, lo, hi, &who))
            }
            _ => None,
        };
        let mv = mid.and_then(|x| moved(x, last, judge == "mine", a));
        let wp = positions::weight(pos, a) * 100.0;
        out.mids[i] = mid;
        out.gaps[i] = mid.zip(marks.as_ref()).map(|(x, mk)| x - mk.starter);
        out.play[i] = play.contains(&a);
        out.moves[i] = mv.as_ref().and_then(|v| {
            let by = v["by"].as_i64()? as i32;
            match v["dir"].as_str()? {
                "up" => Some((true, by)),
                "down" => Some((false, by)),
                _ => None,
            }
        });
        out.rows.push(json!({
            "key": a.key(),
            "label": a.label(),
            "group": group_name(a.category()),
            "in_play": out.play[i],
            "weight_pct": (wp * 10.0).round() / 10.0,
            "weight_line": weight_line(pos, a),
            "staff": range_json(s),
            "mine": range_json(m),
            "judge": judge,
            "mid": mid,
            "starter": marks.as_ref().map(|x| x.starter.round()),
            "elite": marks.as_ref().map(|x| x.elite.round()),
            "backup": marks.as_ref().map(|x| x.backup.round()),
            "band": band_words.as_ref().map(|b| b.0.clone()),
            "band_line": band_words.as_ref().map(|b| b.1.clone()),
            "moved": mv,
        }));
    }
    out
}

/// The strip's three skill cells: up to three each.
fn strip(w: &World, k: usize, r: &Rows) -> Value {
    let a = |i: usize| Attr::ALL[i];
    let cell = |i: usize, extra: Value| {
        let mut v = json!({ "key": a(i).key(), "label": a(i).label() });
        if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
            o.extend(e.clone());
        }
        v
    };
    let mut above: Vec<(usize, f64)> =
        (0..ATTRIBUTE_COUNT).filter(|&i| r.play[i]).filter_map(|i| r.gaps[i].map(|g| (i, g))).collect();
    above.sort_by(|x, y| y.1.total_cmp(&x.1));
    let strongest: Vec<Value> = above
        .iter()
        .filter(|x| x.1 > 0.0)
        .take(3)
        .map(|&(i, g)| cell(i, json!({ "gap": (g * 10.0).round() / 10.0 })))
        .collect();
    let short: Vec<Value> = above
        .iter()
        .rev()
        .filter(|x| x.1 < 0.0)
        .take(3)
        .map(|&(i, g)| cell(i, json!({ "gap": (g * 10.0).round() / 10.0 })))
        .collect();
    let mut mv: Vec<(usize, bool, i32)> =
        (0..ATTRIBUTE_COUNT).filter(|&i| r.play[i]).filter_map(|i| r.moves[i].map(|(up, by)| (i, up, by))).collect();
    mv.sort_by_key(|x| std::cmp::Reverse(x.2.abs()));
    let moved: Vec<Value> = mv
        .iter()
        .take(3)
        .map(|&(i, up, by)| cell(i, json!({ "dir": if up { "up" } else { "down" }, "by": by })))
        .collect();
    json!({
        "strongest": strongest,
        "short": short,
        "moved": moved,
        // Your own potential read is a system the game does not have yet.
        "potential": { "mine": Value::Null, "coaches": reads::potential_words(w, k) },
    })
}

pub(super) fn view(w: &World, args: &Value) -> Reply {
    let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
    let k = views::person_index(w, id).ok_or("That player is no longer in football.")?;
    let p = &w.people[k];
    let pos = p.player.position;
    let r = rows_of(w, k);
    let l = list(pos);
    let fams = crate::lane_squadboards::families(pos);
    let school = matches!(p.stage, Stage::HighSchool | Stage::PrepSchool).then_some(p.hs);
    // The pool as the range cards word it, read against his overall.
    let pool_line =
        r.pool.and_then(|pl| marks_cached(w, pl, pos, Rated::Overall)).map(|m| super::pool_words(w, m.pool, school));
    let with = match args.get("with").and_then(Value::as_u64) {
        Some(o) => {
            let ok = views::person_index(w, o as u32).ok_or("The other player is no longer in football.")?;
            let rb = rows_of(w, ok);
            let gaps: Vec<Value> = (0..ATTRIBUTE_COUNT)
                .filter_map(|i| {
                    let g = r.mids[i]? - rb.mids[i]?;
                    Some(json!({ "key": Attr::ALL[i].key(), "gap": (g * 10.0).round() / 10.0 }))
                })
                .collect();
            Some(json!({
                "id": o,
                "name": w.people[ok].name,
                "pos": w.people[ok].player.position.name(),
                "skills": rb.rows,
                "gaps": gaps,
            }))
        }
        None => None,
    };
    Ok(json!({
        "id": id,
        "name": p.name,
        "pos": pos.name(),
        "pos_words": pos_words(pos).0,
        "pool_words": pool_line,
        "default_view": {
            "families": fams.iter().map(|f| json!({
                "name": f.0,
                "skills": f.1.iter().map(|a| a.key()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "also": l.also.iter().map(|(a, why)| json!({ "key": a.key(), "label": a.label(), "why": why })).collect::<Vec<_>>(),
        },
        "strip": strip(w, k, &r),
        "skills": r.rows,
        "with": with,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(pos: Position) -> HashSet<Attr> {
        in_play(pos)
    }

    #[test]
    fn list_sizes_match_the_agreed_lists() {
        let want = [
            (Position::QB, 26),
            (Position::RB, 20),
            (Position::WR, 27),
            (Position::TE, 28),
            (Position::OT, 23),
            (Position::IOL, 24),
            (Position::EDGE, 25),
            (Position::DT, 24),
            (Position::LB, 30),
            (Position::CB, 27),
            (Position::S, 30),
            (Position::K, 10),
            (Position::P, 12),
            (Position::LS, 14),
        ];
        for (pos, n) in want {
            let l = list(pos);
            assert_eq!(l.core.len() + l.also.len(), n, "{}", pos.name());
            assert_eq!(keys(pos).len(), n, "{}: no skill twice", pos.name());
        }
    }

    #[test]
    fn the_core_is_the_eight_families() {
        for pos in Position::ALL {
            let fams: HashSet<Attr> =
                crate::lane_squadboards::families(pos).iter().flat_map(|f| f.1.iter().copied()).collect();
            let core: HashSet<Attr> = list(pos).core.iter().copied().collect();
            assert_eq!(core, fams, "{}: the list's core is the position's eight families", pos.name());
        }
    }

    #[test]
    fn every_also_has_a_reason_and_kickers_show_families_only() {
        for pos in Position::ALL {
            for (a, why) in list(pos).also {
                assert!(why.len() > 10, "{} {:?}", pos.name(), a);
            }
        }
        for pos in [Position::K, Position::P, Position::LS] {
            assert!(list(pos).also.is_empty(), "{}", pos.name());
        }
    }

    #[test]
    fn the_owners_hard_cases() {
        let edge = keys(Position::EDGE);
        for a in [Attr::RunBlocking, Attr::PassBlocking, Attr::KickPower, Attr::Catching, Attr::PuntPower] {
            assert!(!edge.contains(&a), "an edge rusher shows no {a:?}");
        }
        for pos in [Position::LB, Position::S, Position::CB] {
            assert!(keys(pos).contains(&Attr::Catching), "{} shows catching", pos.name());
        }
    }

    #[test]
    fn his_own_read_judges_and_the_staff_fills_in() {
        assert_eq!(judge_of(Some((70, 76)), Some((60, 68))), ("mine", Some((70, 76))));
        assert_eq!(judge_of(Some((70, 76)), None), ("mine", Some((70, 76))));
        assert_eq!(judge_of(None, Some((60, 68))), ("staff", Some((60, 68))));
        assert_eq!(judge_of(None, None), ("none", None));
    }

    #[test]
    fn a_move_compares_a_read_with_its_own_last_season() {
        let mut last =
            KeptAttrs { season: 2026, pos: 0, staff: vec![0; ATTRIBUTE_COUNT], mine: vec![0; ATTRIBUTE_COUNT] };
        let i = Attr::Speed.index();
        // Half points: 140 is a middle of 70, 120 a middle of 60.
        last.staff[i] = 140;
        last.mine[i] = 120;
        // Your own read is compared with your own, not with the staff's.
        let m = moved(63.0, Some(&last), true, Attr::Speed).unwrap();
        assert_eq!((m["dir"].as_str(), m["by"].as_i64(), m["from_season"].as_i64()), (Some("up"), Some(3), Some(2026)));
        let m = moved(63.0, Some(&last), false, Attr::Speed).unwrap();
        assert_eq!((m["dir"].as_str(), m["by"].as_i64()), (Some("down"), Some(-7)));
        let m = moved(70.9, Some(&last), false, Attr::Speed).unwrap();
        assert_eq!(m["dir"], "same");
        // Not seen then, or nothing kept: no arrow.
        assert!(moved(63.0, Some(&last), true, Attr::Agility).is_none());
        assert!(moved(63.0, None, true, Attr::Speed).is_none());
    }

    #[test]
    fn the_weight_line_says_how_much_it_counts() {
        let l = weight_line(Position::EDGE, Attr::PowerRush);
        assert!(l.starts_with("Counts about ") && l.contains("an edge rusher's overall"), "{l}");
        assert!(weight_line(Position::QB, Attr::PuntPower).starts_with("Doesn't count"));
        assert!(weight_line(Position::QB, Attr::ArmStrength).contains("a quarterback's"));
    }
}
