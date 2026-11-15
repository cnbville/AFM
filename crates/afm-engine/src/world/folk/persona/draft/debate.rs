//! The draft room debate (question 10: "your scouts argue on the clock").
//! On the clock, the top men left on your board each get a case: the
//! scout who reads him highest argues for him, from his own looks (what he
//! likes, how hard he pushes: a man of conviction "pounds the table"), and
//! the one who reads him lowest says why (what worries him). The file's
//! flags (a character worry, a medical concern) are said out loud. Every
//! word comes from the readers' own reads in your file, never the truth.
//! You hear them out, ask, and make the pick yourself (a decision you
//! confirm), or hand it to your staff.
//!
//! Sizes are Claude's picks, proposed, not confirmed.

use crate::world::World;
use crate::world::scouting::file::Link;
use crate::world::scouting::lines;
use crate::world::scouting::readers::{self, Staff};

/// How many men get a case. Proposed, not confirmed.
pub const MEN: usize = 4;
/// How far apart the high and low men must read him for the low man to
/// speak up, in overall points. Proposed, not confirmed.
pub const SPLIT: f64 = 2.5;

/// One man's case.
#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    pub id: u32,
    pub name: String,
    /// The high man (reader code, name) and his case in words.
    pub by: (u32, String),
    pub case: String,
    /// The low man, when he reads him well under the high man.
    pub against: Option<(u32, String, String)>,
    /// The file's flags, in words.
    pub flags: Vec<String>,
}

/// The men left at the top of your board, while you're on the clock.
pub fn candidates(w: &World) -> Vec<u32> {
    let Some(l) = w.scouting.live.as_ref() else { return Vec::new() };
    if !crate::world::scouting::live::on_clock(l) {
        return Vec::new();
    }
    let grades = crate::world::scouting::live::grades(w, l);
    let pick = l.run.next;
    crate::world::scouting::board::ordered(w, &grades)
        .into_iter()
        .filter(|id| !w.scouting.board.is_dnd(*id))
        .filter(|id| {
            l.run
                .pool
                .iter()
                .find(|x| w.people[x.0].player.id == *id)
                .is_some_and(|x| crate::world::recruiting::draft_value(w, &l.run, l.team, x.0, pick).is_some())
        })
        .take(MEN)
        .collect()
}

/// The cases for the men at the top of your board.
pub fn debate(w: &World) -> Vec<Case> {
    let Some(t) = super::club(w) else { return Vec::new() };
    candidates(w).into_iter().filter_map(|id| case(w, t, id)).collect()
}

/// One man's case from your file: the high and low men among the readers
/// who've seen him (not you), and the file's flags.
pub fn case(w: &World, t: usize, id: u32) -> Option<Case> {
    let staff = Staff::Club(t);
    let i = crate::world::scouting::place(w, id)?;
    let (rds, looks, f) = readers::read(w, staff, id)?;
    let (_, _, codes) = readers::evidence(w, staff, id);
    let mut reads: Vec<(f64, usize, crate::attributes::Attr, crate::attributes::Attr)> = Vec::new();
    for (r, rd) in rds.iter().enumerate() {
        if matches!(rd.link, Link::You | Link::Event) || !looks.iter().any(|l| l.reader == r) {
            continue;
        }
        if let Some((c, best, worst)) = super::scouts::own_centre(w, i, &f, r) {
            reads.push((c, r, best, worst));
        }
    }
    if reads.is_empty() {
        return None;
    }
    reads.sort_by(|a, b| b.0.total_cmp(&a.0));
    let p = &w.people[i];
    let (hc, hr, best, _) = reads[0];
    let high = &rds[hr];
    let conviction = match high.link {
        Link::Scout(k) => w.clubs.get(t).and_then(|c| c.scouts.get(k as usize)).map_or(10, |s| s.conviction),
        _ => 10,
    };
    let push = if conviction >= 15 {
        "pounds the table for"
    } else if conviction <= 6 {
        "hedges, but leans toward"
    } else {
        "likes"
    };
    let (lo, hi) = crate::world::scouting::yours::range((hc, (f.now.1 * 1.4).max(2.0)));
    let case = format!(
        "{} {push} {}: he has him at {lo}-{hi} as a {}. \"{}.\"",
        high.name,
        p.name,
        p.player.position,
        cap(lines::attr_phrase(best, true))
    );
    let against = reads.last().filter(|x| reads.len() > 1 && hc - x.0 >= SPLIT).map(|&(lc, lr, _, worst)| {
        let (lo, hi) = crate::world::scouting::yours::range((lc, (f.now.1 * 1.4).max(2.0)));
        let low = &rds[lr];
        (
            codes.get(lr).copied().unwrap_or(0),
            low.name.clone(),
            format!("{} has him lower, at {lo}-{hi}: \"{}.\"", low.name, cap(lines::attr_phrase(worst, false))),
        )
    });
    let mut flags = Vec::new();
    let worry = |k: usize, bad_high: bool| {
        f.traits.get(k).filter(|t| t.seen).is_some_and(|t| if bad_high { t.centre >= 13.0 } else { t.centre <= 7.0 })
    };
    if worry(7, true) || worry(10, false) || worry(0, false) {
        flags.push("Security and the scouts both flag his character.".into());
    }
    match f.medical.grade {
        0 => {}
        1 => flags.push("The doctors have a minor concern.".into()),
        2 => flags.push("The doctors have a major concern.".into()),
        _ => flags.push("He failed our physical.".into()),
    }
    Some(Case {
        id,
        name: p.name.clone(),
        by: (codes.get(hr).copied().unwrap_or(0), high.name.clone()),
        case,
        against,
        flags,
    })
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}
