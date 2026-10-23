//! Look-backs (topic 9): three years on, your staff's grades against how
//! the players turned out. Each draft class your war room graded and each
//! recruiting class your program signed is kept with your rank of each
//! player; on September 1 three years later every player gets an outcome
//! rank in his class (by how good he is now, or how far he got), and the
//! file's verdict: the read held up, we had him too high, too low, or we
//! never had a fair read (fewer than three looks). The scout who saw him
//! most carries the verdict on his record, which is how a staff learns
//! whose eye to trust.

use std::collections::HashMap;

use crate::calendar::Date;
use crate::positions::Position;
use crate::world::{Stage, World};

/// One graded player.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Graded {
    pub year: i32,
    pub id: u32,
    pub name: String,
    pub pos: Position,
    /// Your rank of him in the class (1 is your best).
    pub ours: u32,
    /// The public's rank (the consensus board or the services' rank).
    #[serde(default)]
    pub public: Option<u32>,
    /// The pick he went at, or the stars he signed with.
    #[serde(default)]
    pub pick: Option<u32>,
    /// The club or program that took him.
    #[serde(default)]
    pub team: Option<u16>,
    /// Your staffer with the most looks at him.
    #[serde(default)]
    pub lead: Option<u32>,
    /// How many looks your staff had.
    #[serde(default)]
    pub looks: u16,
    /// His rank in the class three years on.
    #[serde(default)]
    pub outcome: Option<u32>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Records {
    /// Draft classes your war room graded.
    #[serde(default)]
    pub drafts: Vec<Graded>,
    /// Recruiting classes your program signed (your rank in your class).
    #[serde(default)]
    pub classes: Vec<Graded>,
}

/// The four verdicts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Held,
    TooHigh,
    TooLow,
    NoFairRead,
    /// Not three years yet.
    Open,
}

impl Verdict {
    pub fn words(self) -> &'static str {
        match self {
            Verdict::Held => "The read held up",
            Verdict::TooHigh => "We had him too high",
            Verdict::TooLow => "We had him too low",
            Verdict::NoFairRead => "We never had a fair read",
            Verdict::Open => "Too early to say",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Verdict::Held => "held",
            Verdict::TooHigh => "high",
            Verdict::TooLow => "low",
            Verdict::NoFairRead => "unfair",
            Verdict::Open => "open",
        }
    }
}

/// A graded player's verdict, by the same out-on-a-limb distance the board
/// uses (12 places or a third of the rank).
pub fn verdict(g: &Graded) -> Verdict {
    let Some(out) = g.outcome else { return Verdict::Open };
    if g.looks < 3 {
        return Verdict::NoFairRead;
    }
    match super::board::limb(g.ours as usize - 1, Some(out as usize - 1)) {
        1 => Verdict::TooLow,
        -1 => Verdict::TooHigh,
        _ => Verdict::Held,
    }
}

/// Your staffer who saw him most, and how many looks your staff had.
fn lead_of(w: &World, id: u32) -> (Option<u32>, u16) {
    let mut by: HashMap<u32, u16> = HashMap::new();
    let mut n = 0u16;
    if let Some(v) = w.scouting.dept.looks.get(&id) {
        for l in v {
            *by.entry(l.who).or_default() += 1;
            n += 1;
        }
    }
    if let Some(t) = w.seat.and_then(|s| s.nfl_team)
        && let Some(club) = w.clubs.get(t)
        && let Some(v) = club.files.get(&id)
    {
        for l in v {
            *by.entry(l.who as u32).or_default() += 1;
            n += 1;
        }
    }
    let lead = by.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map(|x| x.0);
    (lead, n)
}

/// Draft day: your board as it stood when the room opened (your order over
/// the director's), the top 300.
pub(crate) fn snapshot(w: &mut World, grades: &HashMap<u32, f64>, public: &HashMap<u32, usize>) {
    let year = w.year;
    if w.scouting.records.drafts.iter().any(|g| g.year == year) {
        return;
    }
    let order = super::board::ordered(w, grades);
    let places = super::places(w);
    let mut v = Vec::new();
    for (k, id) in order.into_iter().take(300).enumerate() {
        let Some(&i) = places.get(&id) else { continue };
        let p = &w.people[i];
        let (lead, looks) = lead_of(w, id);
        v.push(Graded {
            year,
            id,
            name: p.name.clone(),
            pos: p.player.position,
            ours: k as u32 + 1,
            public: public.get(&id).map(|&r| r as u32 + 1),
            pick: None,
            team: None,
            lead,
            looks,
            outcome: None,
        });
    }
    w.scouting.records.drafts.extend(v);
}

/// After the draft: where each graded player went.
pub(crate) fn after_draft(w: &mut World) {
    let year = w.year;
    let places = super::places(w);
    let mut found: Vec<(usize, u32, u16)> = Vec::new();
    for (k, g) in w.scouting.records.drafts.iter().enumerate() {
        if g.year != year {
            continue;
        }
        if let Some(&i) = places.get(&g.id)
            && let Some(c) = w.people[i].pro.as_ref()
            && let Some((y, _, pick, team)) = c.draft
            && y == year
        {
            found.push((k, pick, team as u16));
        }
    }
    for (k, pick, team) in found {
        let g = &mut w.scouting.records.drafts[k];
        g.pick = Some(pick);
        g.team = Some(team);
    }
}

/// Signing day: your program's new class, ranked by your staff's read of
/// how good each will be.
fn record_class(w: &mut World, prog: usize) {
    let year = w.today.year();
    if w.scouting.records.classes.iter().any(|g| g.year == year) {
        return;
    }
    let mut v: Vec<(f64, usize)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        let signed = p.cal.signed.as_ref().is_some_and(|s| s.program == prog) && p.class_of == year;
        if !signed {
            continue;
        }
        let read = super::readers::read(w, super::readers::Staff::Program(prog), p.player.id)
            .map_or(p.player.overall(), |r| r.2.ceiling.0);
        v.push((read, i));
    }
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut out = Vec::new();
    for (k, &(_, i)) in v.iter().enumerate() {
        let p = &w.people[i];
        let (lead, looks) = lead_of(w, p.player.id);
        out.push(Graded {
            year,
            id: p.player.id,
            name: p.name.clone(),
            pos: p.player.position,
            ours: k as u32 + 1,
            public: None,
            pick: Some(p.stars as u32),
            team: Some(prog as u16),
            lead,
            looks,
            outcome: None,
        });
    }
    // The services' order of the same class (by stars, then the national rank).
    let mut by_public: Vec<(u8, usize)> = v.iter().map(|&(_, i)| (w.people[i].stars, i)).collect();
    by_public.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (k, (_, i)) in by_public.into_iter().enumerate() {
        let id = w.people[i].player.id;
        if let Some(g) = out.iter_mut().find(|g| g.id == id) {
            g.public = Some(k as u32 + 1);
        }
    }
    w.scouting.records.classes.extend(out);
}

/// How far a player got, for the outcome rank: how good he is now, with a
/// step for each level he reached and a fall for one who left the game.
fn outcome_score(w: &World, i: Option<usize>) -> f64 {
    let Some(i) = i else { return 0.0 };
    let p = &w.people[i];
    let level = match p.stage {
        Stage::Pro => 12.0,
        Stage::College | Stage::Declared => 4.0,
        Stage::Done => -8.0,
        _ => 0.0,
    };
    p.player.overall() + level + 0.02 * p.college_snaps.min(2000) as f64
}

/// September 1: classes three years old get their outcome.
pub(crate) fn day(w: &mut World, d: Date) {
    let (y, m, day) = d.parts();
    if w.seat.is_some_and(|s| s.level == crate::calendar::Level::College)
        && let Some(prog) = w.seat.and_then(|s| s.program)
        && (d == w.dates.signing_day.add(1) || w.prev_dates.as_ref().is_some_and(|p| d == p.signing_day.add(1)))
    {
        record_class(w, prog);
    }
    if !(m == 9 && day == 1) {
        return;
    }
    let places = super::places(w);
    for list in [0, 1] {
        let recs = if list == 0 { &w.scouting.records.drafts } else { &w.scouting.records.classes };
        let years: Vec<i32> = {
            let mut v: Vec<i32> =
                recs.iter().filter(|g| g.outcome.is_none() && g.year + 3 <= y).map(|g| g.year).collect();
            v.sort();
            v.dedup();
            v
        };
        for yr in years {
            let recs = if list == 0 { &w.scouting.records.drafts } else { &w.scouting.records.classes };
            let mut scored: Vec<(f64, usize)> = recs
                .iter()
                .enumerate()
                .filter(|(_, g)| g.year == yr)
                .map(|(k, g)| (outcome_score(w, places.get(&g.id).copied()), k))
                .collect();
            scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            let recs = if list == 0 { &mut w.scouting.records.drafts } else { &mut w.scouting.records.classes };
            for (r, (_, k)) in scored.into_iter().enumerate() {
                recs[k].outcome = Some(r as u32 + 1);
            }
        }
    }
    // Twelve years of look-backs are plenty.
    w.scouting.records.drafts.retain(|g| g.year + 12 > y);
    w.scouting.records.classes.retain(|g| g.year + 12 > y);
}

/// A staffer's record: (held, too high, too low) over his settled reads.
pub fn record_of(r: &Records, who: u32) -> (u32, u32, u32) {
    let mut out = (0, 0, 0);
    for g in r.drafts.iter().chain(r.classes.iter()).filter(|g| g.lead == Some(who)) {
        match verdict(g) {
            Verdict::Held => out.0 += 1,
            Verdict::TooHigh => out.1 += 1,
            Verdict::TooLow => out.2 += 1,
            _ => {}
        }
    }
    out
}
