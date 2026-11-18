//! Word travels (GR62): people talk. What you did with one person reaches
//! his close ties within days at about half its weight, and his area at
//! about a quarter (the spec's sizes, proposed, not confirmed). The good
//! and the bad alike: a kid of his drafted from your program, a pulled
//! offer, your word held or broken.
//!
//! It reaches your program (its name in his area), not your men: nobody
//! called. It isn't a contact, so it never opens a gate on its own, and a
//! loss is never gated. A big moment reaches every head coach in his area
//! (the most talent first, `AREA_MAX`); a small one only the people who
//! already know you there.

use super::Holder;
use super::people::{self, Who};
use super::standing;
use crate::calendar::Date;
use crate::world::World;

/// The share that reaches his close ties, and his area.
pub const TIES_SHARE: f32 = 0.5;
pub const AREA_SHARE: f32 = 0.25;
/// The most head coaches in his area a big moment reaches (the build's
/// pick, proposed, not confirmed).
pub const AREA_MAX: usize = 40;

/// Word on its way.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Travel {
    /// The day it lands.
    pub due: Date,
    pub prog: u32,
    pub to: Who,
    pub by: f32,
    pub good: bool,
    /// What he heard, in words ("he heard what you did for Reyes").
    pub words: String,
    /// A close tie keeps it as a moment.
    pub close: bool,
    /// Whose memory it reaches (lane G10: a man's own, the assistant's
    /// push); none is the program's.
    #[serde(default)]
    pub holder: Option<Holder>,
}

/// Something happened between your program and a person: it travels.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spread(
    w: &mut World,
    prog: usize,
    from: Who,
    by: f32,
    good: bool,
    big: bool,
    about: Option<String>,
    d: Date,
) {
    if w.ground.off || by == 0.0 {
        return;
    }
    let who_name = people::name(w, from);
    let words = match (&about, good) {
        (Some(k), true) => format!("he heard what you did for {k}"),
        (Some(k), false) => format!("he heard how it went with {k}"),
        (None, true) => format!("he heard good things from {who_name}"),
        (None, false) => format!("he heard what happened with {who_name}"),
    };
    let mut ties = people::ties(w, from);
    // About one of his kids: that kid's family was paid with him already.
    if about.is_some() {
        ties.retain(|t| !matches!(t, Who::Family(_)));
    }
    let mut out: Vec<Travel> = Vec::new();
    for (k, t) in ties.iter().enumerate() {
        let lag = 2 + (people::u(w, *t, 0x3A7 + k as u64) * 4.0) as i32;
        out.push(Travel {
            due: d.add(lag),
            prog: prog as u32,
            to: *t,
            by: by * TIES_SHARE,
            good,
            words: words.clone(),
            close: true,
            holder: None,
        });
    }
    // His area: lane G1a's ground area of his school.
    if let Some(area) = people::school(w, from).and_then(|h| w.high_schools.get(h)).map(|s| s.ground_area) {
        let ph = Holder::Program(prog as u32);
        let talent = if big { standing::talents(w) } else { Vec::new() };
        let mut cands: Vec<(f32, Who)> = (0..w.high_schools.len())
            .filter(|&h| w.high_schools[h].ground_area == area)
            .filter_map(|h| people::head_coach_of(w, h).map(|c| (h, c)))
            .filter(|(_, c)| *c != from && !ties.contains(c))
            .filter(|(_, c)| big || w.ground.get(*c, ph).is_some())
            .map(|(h, c)| (talent.get(h).copied().unwrap_or(1.0), c))
            .collect();
        cands.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        for (_, c) in cands.into_iter().take(AREA_MAX) {
            let lag = 5 + (people::u(w, c, 0x3A8) * 8.0) as i32;
            out.push(Travel {
                due: d.add(lag),
                prog: prog as u32,
                to: c,
                by: by * AREA_SHARE,
                good,
                words: words.clone(),
                close: false,
                holder: None,
            });
        }
    }
    w.ground.travel.extend(out);
    w.ground.travel.sort_by_key(|t| t.due);
}

/// Word about you from one person to his close ties, at half, into a
/// holder's own memory (lane G10: the high school coach who saw you push
/// for a kid tells his ties; it goes with you, GR29).
pub(crate) fn spread_to(w: &mut World, holder: Holder, from: Who, by: f32, about: Option<String>, d: Date) {
    let Some(prog) = standing::seat_program(w) else { return };
    if w.ground.off || by == 0.0 {
        return;
    }
    let words = match &about {
        Some(k) => format!("he heard you pushed for {k} before anybody did"),
        None => format!("he heard good things from {}", people::name(w, from)),
    };
    let ties = people::ties(w, from);
    for (k, t) in ties.iter().enumerate() {
        if matches!(t, Who::Family(_)) {
            continue;
        }
        let lag = 2 + (people::u(w, *t, 0x3A9 + k as u64) * 4.0) as i32;
        w.ground.travel.push(Travel {
            due: d.add(lag),
            prog: prog as u32,
            to: *t,
            by: by * TIES_SHARE,
            good: true,
            words: words.clone(),
            close: true,
            holder: Some(holder),
        });
    }
    w.ground.travel.sort_by_key(|t| t.due);
}

/// Word that lands today reaches the program's memory with each man.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(prog) = standing::seat_program(w) else { return };
    let n = w.ground.travel.partition_point(|t| t.due <= d);
    if n == 0 {
        return;
    }
    let due: Vec<Travel> = w.ground.travel.drain(..n).collect();
    for t in due {
        if t.prog as usize != prog {
            continue;
        }
        let before = standing::level(w, prog, t.to);
        let holder = t.holder.unwrap_or(Holder::Program(prog as u32));
        let start = standing::base(w, t.to, holder);
        let m = standing::entry(&mut w.ground, start, t.to, holder, d);
        standing::gain(m, t.by, d);
        if m.contacts == 0 || m.heard.is_none() {
            m.heard = Some(t.words.clone());
        }
        if t.close {
            let line = format!("{}.", capital(&t.words)).replacen("He heard", "I heard", 1);
            m.keep(super::Moment { date: d, words: line, good: t.good, big: false, holds_to: None });
        }
        standing::announce(w, prog, t.to, before, d);
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
}
