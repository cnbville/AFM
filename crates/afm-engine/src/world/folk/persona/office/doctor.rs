//! The team doctor (question 10, brought back 2026-11-15): asking him about
//! a man gives his chart as the medical staff reads it, and asking him to
//! clear a hurt man is the pressure to clear him, which the medical staff
//! answers by its own judgment and honesty.
//!
//! Who is he? The head athletic trainer, who speaks for the team's doctors
//! in the built second-opinion meeting (`life.rs`, P3): the cast person in
//! the `Trainer` job, his eye the medical staff's rating
//! (`Support.medical`). A separate team physician is not drawn (proposed,
//! not confirmed).
//!
//! What runs is the built P3 code: a second opinion (`life::second_opinion`)
//! and rushing him back (`life::rush_back`, the re-injury risk of
//! `rushed_week`), the same functions the meeting's chips run.

use super::super::Them;
use super::super::knows::{Known, Source, Topic};
use crate::calendar::Date;
use crate::game::sim::InjuryKind;
use crate::world::World;
use crate::world::folk::cast::{self, CastRef, Role as CastRole};
use crate::world::folk::{self as folk, Team};

/// Whether he is your team's doctor (the head athletic trainer).
pub fn is_doctor(w: &World, him: Them) -> bool {
    matches!(him, Them::Cast(r, _) if r.role == CastRole::Trainer && Some(r.team) == Team::seat(w))
}

/// The medical staff's rating, 1-20: his eye for a chart.
pub fn eye(w: &World, team: Team) -> f64 {
    let r = match team {
        Team::College(p) => w.programs[p].support.medical.rating,
        Team::Nfl(t) => w.clubs.get(t).map_or(10, |c| c.support.medical.rating),
        // A high school's trainer, where it has one, works alone.
        _ => 8,
    };
    if r == 0 { 10.0 } else { f64::from(r) }
}

/// A man's chart today: what he has, how many days he has left out (as
/// the world holds it), whether he was rushed back lately, and his major
/// injuries over his career.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chart {
    pub kind: Option<InjuryKind>,
    pub days: u32,
    pub rushed: Option<Date>,
    pub majors: u32,
}

impl Chart {
    pub fn hurt(&self) -> bool {
        self.days > 0
    }
}

/// His chart, from the stores that hold it: a pro's days out
/// (`ProCareer.out_days`), your school's training room (`training.hurt`,
/// `others_hurt`), the rushed-back list (`folk.rushed`) and his history.
pub fn chart(w: &World, i: usize) -> Chart {
    let p = &w.people[i];
    let id = p.player.id;
    let today = w.today;
    let mut c = Chart::default();
    if let Some(pro) = &p.pro {
        c.days = if pro.out_days > 0 { pro.out_days } else { pro.out_weeks * 7 };
    }
    if let Some(h) = w.training.hurt_on(id, today) {
        c.days = c.days.max(today.days_to(h.until).max(0) as u32);
        c.kind = h.kind;
    }
    if let Some((back, kind)) = w.training.other_hurt(id, today) {
        c.days = c.days.max(today.days_to(back).max(0) as u32);
        c.kind = c.kind.or(kind);
    }
    c.rushed = w.folk.rushed.iter().filter(|x| x.0 == id && x.1.days_to(today) <= 56).map(|x| x.1).max();
    c.majors = p
        .player
        .history
        .iter()
        .flat_map(|h| h.events.iter())
        .filter(|e| matches!(e, crate::player::DevelopmentEvent::MajorInjury))
        .count() as u32;
    c
}

/// His estimate of the days left: the true count off by a keyed share
/// that shrinks with the staff's eye (an elite staff within a few percent,
/// a poor one up to about a third). Proposed, not confirmed.
pub fn estimate(w: &World, team: Team, id: u32, days: u32) -> u32 {
    let e = eye(w, team);
    let u = folk::roll_n(w, 0xD0C7 ^ id as u64, w.today.0 as u64 / 7);
    let off = u * (20.0 - e) / 20.0 * 0.35;
    ((days as f64) * (1.0 + off).max(0.4)).round().max(1.0) as u32
}

/// The re-injury risk of a man rushed back, in words: `rushed_week`'s 8% a
/// week for eight weeks.
pub const RUSHED_WORDS: &str =
    "bring him back early and the chance he goes down again is about one in twelve every week for two months";

/// What the doctor knows when you ask about a man's injury (`Topic::Injury`
/// about your player): his chart, in words.
pub fn knows(w: &World, him: Them, topic: Topic, about: Option<Them>) -> Vec<Known> {
    let (true, Topic::Injury, Some(Them::Player(id))) = (is_doctor(w, him), topic, about) else {
        return Vec::new();
    };
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let Some(i) = folk::person(w, id) else { return Vec::new() };
    if !team.roster(w).contains(&i) {
        return Vec::new();
    }
    let c = chart(w, i);
    let e = eye(w, team);
    let name = &w.people[i].name;
    let mut words: Vec<String> = Vec::new();
    if c.hurt() {
        let est = estimate(w, team, id, c.days);
        let what = c.kind.map_or("his injury".to_string(), |k| format!("a {}", k.name()));
        let when = if est < 10 {
            format!("{est} days or so")
        } else {
            let wk = est.div_ceil(7);
            format!("about {wk} {}", folk::noun(wk, "week"))
        };
        words.push(format!("{name} has {what}; the doctors have him out {when}"));
        words.push(RUSHED_WORDS.into());
    } else if let Some(d) = c.rushed {
        words.push(format!(
            "{name} came back early; we're watching it closely until {}",
            crate::calendar::Date(d.0 + 56).pretty()
        ));
    } else {
        words.push(format!("{name} has nothing on his chart: he's cleared"));
    }
    if c.majors >= 2 {
        words.push(format!("his body has been through {} major injuries", c.majors));
    }
    vec![Known {
        topic: Topic::Injury,
        about,
        from: Source::Job,
        eye: e,
        point: None,
        says: Vec::new(),
        words: words.join("; "),
        sure: (0.35 + 0.65 * e / 20.0) as f32,
        day: (c.days > 0).then(|| w.today.add(c.days as i32)),
    }]
}

/// How willing the doctor is to clear a hurt man early, 0-1 (the pressure
/// to clear him): a staff with good judgment (14+) never does (P3's rule,
/// the chip's own); below it, his trust in you against his honesty and
/// judgment: 0.5 + 0.5 x (trust - 0.5) - 0.025 x (honesty - 10) - 0.02 x
/// (judgment - 10). He clears at 0.45 or more, so words never clear a man
/// the chip wouldn't. Proposed, not confirmed.
pub fn clear_score(w: &World, him: Them, team: Team) -> f64 {
    if !crate::world::folk::life::would_rush(w, team) {
        return 0.0;
    }
    let d = cast::get(w, CastRef::new(team, CastRole::Trainer, 0));
    let t = super::trust(w, him) as f64;
    0.5 + 0.5 * (t - 0.5) - 0.025 * (d.honesty - 10.0) - 0.02 * (d.judgement - 10.0)
}

/// The score he clears a man at. Proposed, not confirmed.
pub const CLEARS: f64 = 0.45;

/// What a doctor who wouldn't clear him costs: the chip's own (-0.05) when
/// his judgment refused it, less (-0.02) when he only didn't like being
/// pushed. Proposed, not confirmed.
pub const REFUSED_JUDGMENT: f32 = -0.05;
pub const REFUSED_PUSHED: f32 = -0.02;
