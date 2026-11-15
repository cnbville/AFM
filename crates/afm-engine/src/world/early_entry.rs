//! Early entry (the early entrants lane, Cain 2026-11-14): whether an
//! underclassman leaves for the draft. He weighs where he hears he will go
//! against what keeps him in school, the way real draft advice frames it:
//! the slot's money and the clock to a second contract, his age and his
//! degree, his family's need, the NIL pay he would give up, how long his
//! position lasts in the league, a new head coach, and his own make-up.
//!
//! Each part is a term on the log-odds of going, so no single one decides
//! him: a projected first-rounder with big NIL pay still mostly goes, and
//! a fringe prospect goes only when several parts pull together. Real
//! targets: underclassmen entering (special eligibility plus degree
//! finishers) 128 in 2021, 100 in 2022, 82 in 2023, 70 in 2025 and 63 in
//! 2026; undrafted underclassmen fell from 28 in the 2021 draft to 5 in
//! 2024 (NFL announcements; CBS Sports, April 2025). Every size here is
//! proposed, not confirmed.

use super::background::hardship;
use super::{Person, World};
use crate::player::Trait;
use crate::positions::Position;
use crate::rng::clamp;

/// Log-odds of going by the round he hears, at the top of each round
/// (round 8: just past the last pick), for a third-year junior from an
/// average family with no NIL pay and no degree, before the rookie scale.
/// Past the draft the odds fall 1.5 a round more. Fitted with the other
/// parts to the real counts (see the module note; proposed, not confirmed).
const BY_ROUND: [f64; 8] = [3.0, 0.05, -0.85, -1.45, -2.05, -2.45, -2.85, -3.45];
const PAST_DRAFT: f64 = 1.5;

/// The share of a slot's rookie money a pick sees on average, by round
/// (undrafted last): first-rounders play out their deals, while about half
/// of day-three picks are gone by their third year (proposed, not
/// confirmed).
const KEEPS: [f64; 8] = [0.95, 0.85, 0.75, 0.65, 0.55, 0.5, 0.45, 0.3];

/// Why he went, or why a good prospect stayed: the biggest part of his
/// choice, for his note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Why {
    Slot,
    Age,
    Degree,
    Family,
    Coach,
    Position,
    Drive,
    Nil,
    Young,
    Loyal,
}

impl Why {
    pub(crate) fn code(self) -> u8 {
        self as u8 + 1
    }
    pub(crate) fn from_code(c: u8) -> Option<Why> {
        use Why::*;
        [Slot, Age, Degree, Family, Coach, Position, Drive, Nil, Young, Loyal].get(c.checked_sub(1)? as usize).copied()
    }
    /// The reason as the end of a sentence about him.
    pub(crate) fn text(self) -> &'static str {
        match self {
            Why::Slot => "where he was told he'd be picked",
            Why::Age => "his age and the clock to a second contract",
            Why::Degree => "his degree done",
            Why::Family => "his family's need for the money",
            Why::Coach => "the head coaching change",
            Why::Position => "how short careers at his position run",
            Why::Drive => "his own drive to get to the league",
            Why::Nil => "the NIL money he would give up",
            Why::Young => "how much better another year could make him",
            Why::Loyal => "his ties to the program",
        }
    }
}

/// His lean: the chance he goes, and the part that weighed most.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Lean {
    pub chance: f64,
    pub why: Why,
}

/// What going is worth to him, the way an agent puts it: the era's rookie
/// deal in the middle of the round he hears, its guaranteed money (bonus
/// and guaranteed base) over the whole deal plus a year of the rest at the
/// share such picks see ([`KEEPS`]). A first-rounder's four guaranteed
/// years dwarf a year of NIL pay; a day-three pick's small bonus doesn't.
/// Past the draft, a year of an undrafted deal at the same odds.
fn slot_value(draft_year: i32, round: f64) -> f64 {
    use super::pro::money;
    if round >= 7.5 {
        return money::undrafted_terms(draft_year, 0.5).apy * KEEPS[7];
    }
    let r = round.round().clamp(1.0, 7.0) as u32;
    let pick = ((r as f64 - 0.5) * 32.0).round() as u32;
    let t = money::rookie_terms(draft_year, pick, r);
    let sure = clamp(t.bonus_share + t.base_guarantee, 0.0, 1.0);
    t.apy * (t.years as f64 * sure + (1.0 - sure) * KEEPS[r as usize - 1])
}

/// The log-odds of going from the round he hears (1.0 = the top of round
/// one, 8.0 = just past the last pick; continuous).
fn by_round(round: f64) -> f64 {
    if round >= 8.0 {
        return BY_ROUND[7] - PAST_DRAFT * (round - 8.0);
    }
    let r = round.max(1.0);
    let k = (r.floor() as usize).clamp(1, 7);
    let f = r - k as f64;
    BY_ROUND[k - 1] + (BY_ROUND[k] - BY_ROUND[k - 1]) * f
}

/// What he weighs, before the weights: kept apart so a trial can dump
/// them and the weights can be checked against real counts.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Facts {
    /// Where he hears he will go (1.0 the top pick, 8.0 just past the last).
    pub round: f64,
    /// Years since high school.
    pub years: f64,
    pub graduated: bool,
    /// His family's need, 0-1.
    pub need: f64,
    /// NIL pay he would give up against what going is worth to him
    /// ([`slot_value`]; 0 before the NIL era).
    pub nil: f64,
    /// How far the rookie scale (2011 CBA) has set in: none before the 2012
    /// draft, a third in 2012, two thirds in 2013, all from 2014.
    pub scale: f64,
    pub pos: Position,
    pub new_coach: bool,
    pub ambition: f64,
    pub ego: f64,
    pub greed: f64,
    pub loyalty: f64,
}

/// What he weighs (see the module note).
pub(crate) fn facts(w: &World, p: &Person, round: f64, draft_year: i32) -> Facts {
    let t = &p.player.personality.traits;
    let nil = if super::ncaa::nil_era(draft_year - 1) && p.nil_income > 0.0 {
        p.nil_income / slot_value(draft_year, round).max(1.0)
    } else {
        0.0
    };
    Facts {
        round,
        years: (draft_year - p.class_of) as f64,
        // A December graduate knows his degree is coming when he weighs it.
        graduated: p.cal.graduated || super::college::graduates_in_december(p, w.year, w.options.seed),
        need: 0.6 * hardship(&p.bg) + 0.4 * clamp(p.family.decider_wants.money * 3.0, 0.0, 1.0),
        nil,
        scale: clamp((draft_year - 2011) as f64 / 3.0, 0.0, 1.0),
        pos: p.cal.pro_position.unwrap_or(p.player.position),
        new_coach: p.college.is_some_and(|c| w.programs[c].hc_changed == w.year),
        ambition: t.z(Trait::Ambition),
        ego: t.z(Trait::Ego),
        greed: t.z(Trait::Greed),
        loyalty: t.z(Trait::Loyalty),
    }
}

/// The parts of his choice, each a term on the log-odds of going.
fn parts(f: &Facts) -> Vec<(f64, Why)> {
    let mut parts: Vec<(f64, Why)> = Vec::with_capacity(10);
    // The slot: first-rounders' money is life-changing; a fringe read
    // promises little.
    parts.push((by_round(f.round) - by_round(3.5), Why::Slot));
    // His age: a fourth-year player has less to gain from a year more and
    // more to lose on the clock to a second contract; a true junior can
    // still climb.
    parts.push((0.55 * clamp(f.years - 3.0, 0.0, 1.0) + 0.35 * clamp(f.years - 4.0, 0.0, 1.0), Why::Age));
    if f.years <= 3.0 && f.round > 2.5 {
        parts.push((-0.25, Why::Young));
    }
    // His degree: the promise to his family kept, nothing left to stay for
    // in school (a third of 2026's list were degree finishers).
    if f.graduated {
        parts.push((0.6, Why::Degree));
    }
    // His family's need: a hard-up home, a decider who wants the money.
    parts.push((1.6 * (f.need - 0.35), Why::Family));
    // The NIL pay he would give up (from 2021; part B 11), against what
    // going is worth to him: even some pay changes the math for a mid-round
    // prospect, and more counts for less (4.5 at most).
    if f.nil > 0.0 {
        parts.push((-4.5 * f.nil / (f.nil + 0.3), Why::Nil));
    }
    // The rookie scale (2011 CBA): a slotted four-year deal makes the
    // second contract the prize, and its clock starts on draft day. Real
    // underclass entrants went from about 40-55 a year before it to about
    // 95-115 in 2016-2021. A first-rounder went anyway.
    if f.round >= 1.5 {
        parts.push((1.5 * f.scale, Why::Age));
    }
    // His position: backs wear out young; a quarterback needs reps;
    // specialists are rarely drafted.
    let pos = match f.pos {
        Position::RB => 0.45,
        Position::QB => -0.3,
        Position::K | Position::P | Position::LS => -0.9,
        _ => 0.0,
    };
    parts.push((pos, Why::Position));
    // A new head coach: the staff that recruited him is gone.
    if f.new_coach {
        parts.push((0.4, Why::Coach));
    }
    // His make-up (section 6 topic 1): ambition and ego push him to go,
    // greed wants the money now, loyalty keeps him.
    parts.push((0.7 * f.ambition + 0.3 * f.ego + 0.25 * f.greed, Why::Drive));
    parts.push((-0.4 * f.loyalty, Why::Loyal));
    parts
}

/// An underclassman's chance to declare (T3.1): see the module note.
/// `round` is where he hears he will go: the December read, then the
/// advisory committee's grade or his agent's word.
pub(crate) fn lean(w: &World, p: &Person, round: f64, draft_year: i32) -> Lean {
    let f = facts(w, p, round, draft_year);
    #[cfg(feature = "debug")]
    dump(p, &f, draft_year);
    weigh(&f)
}

/// His lean from what he weighs.
fn weigh(f: &Facts) -> Lean {
    let parts = parts(f);
    let logit = by_round(3.5) + parts.iter().map(|x| x.0).sum::<f64>();
    let chance = clamp(1.0 / (1.0 + (-logit).exp()), 0.0, 0.97);
    let going = chance >= 0.5;
    let why = parts
        .iter()
        .filter(|x| if going { x.0 > 0.0 } else { x.0 < 0.0 })
        .max_by(|a, b| a.0.abs().total_cmp(&b.0.abs()))
        .map_or(Why::Slot, |x| x.1);
    Lean { chance, why }
}

/// Trial tooling: `AFM_ENTRY_DUMP=file` appends every lean's facts, one
/// line each, so the weights can be fitted against real counts offline.
#[cfg(feature = "debug")]
fn dump(p: &Person, f: &Facts, draft_year: i32) {
    use std::io::Write;
    let Ok(path) = std::env::var("AFM_ENTRY_DUMP") else { return };
    let Ok(mut out) = std::fs::OpenOptions::new().create(true).append(true).open(path) else { return };
    let _ = writeln!(
        out,
        "{draft_year},{},{:.3},{},{},{:.3},{:.3},{},{:?},{},{:.2},{:.2},{:.2},{:.2},{}",
        p.player.id,
        f.round,
        f.years,
        f.graduated as u8,
        f.need,
        f.nil,
        (f.scale >= 1.0) as u8,
        f.pos,
        f.new_coach as u8,
        f.ambition,
        f.ego,
        f.greed,
        f.loyalty,
        p.cal.draft_read.map_or(0, |r| r.round),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slot_falls_smoothly() {
        let mut last = f64::INFINITY;
        for k in 0..=40 {
            let r = 1.0 + k as f64 * 0.25;
            let v = by_round(r);
            assert!(v < last, "{r}: {v}");
            last = v;
        }
        assert!((by_round(1.0) - BY_ROUND[0]).abs() < 1e-9);
        assert!((by_round(8.0) - BY_ROUND[7]).abs() < 1e-9);
    }

    fn junior(round: f64) -> Facts {
        Facts {
            round,
            years: 3.0,
            graduated: false,
            need: 0.35,
            nil: 0.0,
            scale: 1.0,
            pos: Position::WR,
            new_coach: false,
            ambition: 0.0,
            ego: 0.0,
            greed: 0.0,
            loyalty: 0.0,
        }
    }

    #[test]
    fn the_parts_pull_the_right_way() {
        // A fourth-year projected first-rounder goes even with $2M of NIL
        // pay against his slot's guaranteed $20M; a fringe true junior paid
        // to stay stays.
        let star = Facts { years: 4.0, nil: 0.1, ..junior(1.3) };
        assert!(weigh(&star).chance > 0.75, "{:?}", weigh(&star));
        let fringe = Facts { nil: 0.5, ..junior(6.5) };
        assert!(weigh(&fringe).chance < 0.1, "{:?}", weigh(&fringe));
        assert_eq!(weigh(&fringe).why, Why::Nil);
        // Each pull raises his chance on its own.
        let mid = junior(4.5);
        let base = weigh(&mid).chance;
        for more in [
            Facts { years: 4.0, ..mid },
            Facts { graduated: true, ..mid },
            Facts { need: 0.8, ..mid },
            Facts { pos: Position::RB, ..mid },
            Facts { new_coach: true, ..mid },
            Facts { ambition: 0.6, ..mid },
        ] {
            assert!(weigh(&more).chance > base, "{more:?}");
        }
        for less in [Facts { nil: 0.3, ..mid }, Facts { loyalty: 0.6, ..mid }, Facts { scale: 0.0, ..mid }] {
            assert!(weigh(&less).chance < base, "{less:?}");
        }
    }

    #[test]
    fn reasons_round_trip() {
        for c in 1..=10u8 {
            assert_eq!(Why::from_code(c).unwrap().code(), c);
        }
        assert!(Why::from_code(0).is_none());
    }
}
