//! What a week's practice works, per player (topics 1, 4 and 7): each
//! period's work on the attribute families, by the reps a player really
//! gets (the ones, the twos, the scout team, the young players' period,
//! the specialists at their own station), its gear, contact and tempo;
//! how much a player can absorb in a day; the load his body takes, and
//! what the week leaves for the game (prep and freshness).

use super::FAMILIES;
use super::plan::{Contact, Day, Kind, Period, Tempo, Week, Who};
use crate::player::{Player, PlayingTime};
use crate::positions::Position;
use crate::rng::clamp;

/// A player's place in the week's reps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Role {
    Starter,
    /// The twos (rotation players and the first backups).
    Backup,
    /// Outside the two-deep: the scout team and the developmental period.
    Young,
    /// Kickers, punters and snappers at their own station.
    Specialist,
}

pub const ROLES: usize = 4;

impl Role {
    pub const ALL: [Role; ROLES] = [Role::Starter, Role::Backup, Role::Young, Role::Specialist];
    pub fn of(pos: Position, last: Option<PlayingTime>) -> Role {
        if pos.is_specialist() {
            return Role::Specialist;
        }
        match last {
            Some(PlayingTime::Starter) => Role::Starter,
            Some(PlayingTime::Rotation) => Role::Backup,
            _ => Role::Young,
        }
    }
    pub fn label(self) -> &'static str {
        ["Starter", "Two-deep", "Scout team and young", "Specialist"][self as usize]
    }
}

/// Work a minute on each family (strength, speed, conditioning,
/// technique, contact, kicking, mental) at normal tempo, for a player in
/// the period (proposed, not confirmed: the shape of topic 1's "each
/// period grows what it works, and only that"). The contact column is
/// scaled by the period's contact level.
fn per_minute(k: Kind, specialist: bool) -> [f64; FAMILIES] {
    if specialist {
        return match k {
            Kind::Individual | Kind::SpecialTeams => [0.0, 0.05, 0.1, 0.2, 0.0, 1.0, 0.05],
            Kind::Team | Kind::Situations | Kind::Scrimmage | Kind::SevenOnSeven | Kind::InsideRun => {
                [0.0, 0.0, 0.05, 0.05, 0.0, 0.3, 0.1]
            }
            Kind::Young | Kind::OneOnOnes => [0.0, 0.0, 0.05, 0.05, 0.0, 0.4, 0.0],
            _ => per_minute(k, false),
        };
    }
    match k {
        Kind::WarmUp => [0.0, 0.15, 0.2, 0.0, 0.0, 0.0, 0.0],
        Kind::Individual => [0.0, 0.1, 0.2, 1.0, 0.35, 0.0, 0.03],
        Kind::InsideRun => [0.0, 0.0, 0.3, 0.45, 1.0, 0.0, 0.4],
        Kind::SevenOnSeven => [0.0, 0.1, 0.35, 0.65, 0.2, 0.0, 0.6],
        Kind::OneOnOnes => [0.0, 0.1, 0.3, 0.8, 0.6, 0.0, 0.15],
        Kind::Team => [0.0, 0.0, 0.3, 0.35, 0.6, 0.0, 0.8],
        Kind::Scrimmage => [0.0, 0.05, 0.45, 0.35, 1.0, 0.0, 0.8],
        Kind::Situations => [0.0, 0.0, 0.25, 0.25, 0.35, 0.0, 1.0],
        Kind::SpecialTeams => [0.0, 0.05, 0.2, 0.15, 0.25, 0.0, 0.2],
        Kind::Walkthrough => [0.0, 0.0, 0.0, 0.15, 0.0, 0.0, 0.7],
        Kind::Conditioning => [0.0, 0.2, 1.2, 0.0, 0.0, 0.0, 0.0],
        Kind::Speed => [0.0, 1.0, 0.3, 0.0, 0.0, 0.0, 0.0],
        Kind::WeightRoom => [1.0, 0.15, 0.1, 0.0, 0.0, 0.0, 0.0],
        Kind::Meeting => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.6],
        Kind::Film => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.9],
        Kind::Young => [0.0, 0.05, 0.25, 0.8, 0.5, 0.0, 0.5],
        Kind::Recovery => [0.0; FAMILIES],
    }
}

/// How much of the contact column a contact level teaches.
fn contact_factor(c: Contact) -> f64 {
    match c {
        Contact::Air => 0.0,
        Contact::Bags => 0.35,
        Contact::Control => 0.65,
        Contact::Thud => 0.9,
        Contact::Live => 1.0,
    }
}

/// The share of a period's reps a role gets.
pub fn reps(who: Who, role: Role, camp: bool) -> f64 {
    match (who, role) {
        (Who::All, _) => 1.0,
        (Who::Ones, Role::Starter) => {
            if camp {
                0.6
            } else {
                0.75
            }
        }
        (Who::Ones, Role::Backup) => {
            if camp {
                0.5
            } else {
                0.4
            }
        }
        (Who::Ones, Role::Young) => 0.35,
        (Who::Ones, Role::Specialist) => 1.0,
        (Who::Young, Role::Young) => 1.0,
        (Who::Young, Role::Backup) => 0.5,
        (Who::Young, Role::Starter) => 0.0,
        (Who::Young, Role::Specialist) => 0.5,
    }
}

/// One period's raw work for a role.
pub fn period_work(p: &Period, role: Role, camp: bool) -> [f64; FAMILIES] {
    let spec = role == Role::Specialist;
    let mut w = per_minute(p.kind, spec);
    let r = reps(p.who, role, camp) * p.min as f64;
    // The scout team runs the opponent's plays: his general skills and
    // effort grow, not his own playbook.
    let scout = p.who == Who::Ones && role == Role::Young;
    let cf = contact_factor(p.contact);
    let (tempo_cond, tempo_skill) = match p.tempo {
        Tempo::Walk => (0.1, 0.3),
        Tempo::Normal => (1.0, 1.0),
        Tempo::Fast => (1.4, 1.15),
    };
    for (f, x) in w.iter_mut().enumerate() {
        let mut v = *x * r;
        match f {
            2 => v *= tempo_cond,
            3 | 5 => v *= tempo_skill,
            4 => v *= cf * if p.tempo == Tempo::Walk { 0.0 } else { tempo_skill },
            6 => {
                v *= if p.tempo == Tempo::Walk { 0.9 } else { tempo_skill };
                if scout {
                    v *= 0.4;
                }
            }
            1 if p.tempo == Tempo::Walk => v = 0.0,
            _ => {}
        }
        *x = v;
    }
    w
}

/// What a player can take in during one day, per family (topic 4,
/// "players absorb only so much"; proposed, not confirmed: the caps in
/// work-minutes, moved by football IQ, work ethic and age for the
/// learning families).
pub const DAY_CAP: [f64; FAMILIES] = [60.0, 30.0, 60.0, 60.0, 35.0, 50.0, 90.0];

/// A player's own absorb factor for technique, kicking and the mental
/// side: football IQ, work ethic and years in the game.
pub fn absorb_factor(p: &Player) -> f64 {
    let iq = p.attributes.get(crate::Attr::FootballIq);
    clamp(
        0.75 + 0.005 * (iq - 50.0) + 0.02 * (p.personality.work_ethic - 10.0) + 0.015 * (p.age as f64 - 17.0).min(8.0),
        0.6,
        1.3,
    )
}

/// One day's absorbed work: each family's raw work through its cap
/// (`cap (1 - e^(-raw / cap))`), so ten periods of one drill do less than
/// ten different ones.
pub fn day_work(d: &Day, role: Role, camp: bool, absorb: f64) -> [f64; FAMILIES] {
    let mut raw = [0.0; FAMILIES];
    for p in &d.periods {
        let w = period_work(p, role, camp);
        for f in 0..FAMILIES {
            raw[f] += w[f];
        }
    }
    let mut out = [0.0; FAMILIES];
    for f in 0..FAMILIES {
        let cap = DAY_CAP[f] * if matches!(f, 3 | 5 | 6) { absorb } else { 1.0 };
        out[f] = if raw[f] > 0.0 { cap * (1.0 - (-raw[f] / cap).exp()) } else { 0.0 };
    }
    out
}

pub fn week_work(w: &Week, role: Role, camp: bool, absorb: f64) -> [f64; FAMILIES] {
    let mut out = [0.0; FAMILIES];
    for d in &w.days {
        let x = day_work(d, role, camp, absorb);
        for f in 0..FAMILIES {
            out[f] += x[f];
        }
    }
    out
}

/// How much of each family's growth practice drives, against what a
/// player's year brings on its own (game snaps, age, the body growing):
/// the weight room drives strength, speed work speed far less (speed is
/// mostly given), conditioning stamina, the meetings only part of the
/// mental side (games teach most of it; topic 12). Proposed, not
/// confirmed.
pub const PRACTICE_DRIVEN: [f64; FAMILIES] = [0.55, 0.3, 0.7, 0.5, 0.6, 0.5, 0.35];

/// Each family's work against the neutral week's, near 1 for a neutral
/// plan.
pub fn ratio(work: &[f64; FAMILIES], neutral: &[f64; FAMILIES]) -> [f64; FAMILIES] {
    let mut r = [1.0; FAMILIES];
    for f in 0..FAMILIES {
        r[f] = (work[f] + 1.0) / (neutral[f] + 1.0);
    }
    r
}

/// Most a week's plan moves a family's growth either way (choice 2,
/// confirmed: the best realistic plan against a poor one moves a young
/// player's year by at most about 15%).
pub const PLAN_REACH: f64 = 0.15;

/// The growth share that lands this week: the calendar's reach moved by
/// how much the week really worked, times his attendance. The move
/// saturates toward `PLAN_REACH`, so a lopsided plan (all weight room)
/// costs the families it skips about 15% and no more: the year still has
/// its game snaps, its age and the body growing on its own.
pub fn share(base: &[f64; FAMILIES], r: &[f64; FAMILIES], attend: f64) -> [f64; FAMILIES] {
    let mut s = [0.0; FAMILIES];
    for f in 0..FAMILIES {
        let pf = PRACTICE_DRIVEN[f];
        let raw = pf * (clamp(r[f], 0.0, 1.8) - 1.0);
        s[f] = base[f] * (1.0 + PLAN_REACH * (raw / PLAN_REACH).tanh()) * attend;
    }
    s
}

/// The load a minute of a period puts on the body (the GPS units of
/// topic 7, proposed, not confirmed).
fn load_per_min(p: &Period) -> f64 {
    let base = match p.kind {
        Kind::WarmUp => 0.3,
        Kind::Individual => 0.6,
        Kind::InsideRun => 1.0,
        Kind::SevenOnSeven => 0.75,
        Kind::OneOnOnes => 0.85,
        Kind::Team => 0.8,
        Kind::Scrimmage => 1.1,
        Kind::Situations => 0.7,
        Kind::SpecialTeams => 0.6,
        Kind::Walkthrough => 0.1,
        Kind::Conditioning => 1.0,
        Kind::Speed => 0.85,
        Kind::WeightRoom => 0.5,
        Kind::Young => 0.75,
        Kind::Meeting | Kind::Film | Kind::Recovery => 0.0,
    };
    let c = match p.contact {
        Contact::Air => 0.85,
        Contact::Bags => 0.9,
        Contact::Control => 1.0,
        Contact::Thud => 1.15,
        Contact::Live => 1.35,
    };
    let t = match p.tempo {
        Tempo::Walk => 0.3,
        Tempo::Normal => 1.0,
        Tempo::Fast => 1.35,
    };
    base * c * t
}

/// A day's load for a role, and its full-contact minutes.
pub fn day_load(d: &Day, role: Role, camp: bool) -> (f64, f64) {
    let mut load = if d.game { if role == Role::Young { 20.0 } else { 90.0 } } else { 0.0 };
    let mut full = 0.0;
    for p in &d.periods {
        let r = reps(p.who, role, camp);
        load += load_per_min(p) * p.min as f64 * r;
        if p.contact.full() {
            full += p.min as f64 * r;
        }
    }
    (load, full)
}

/// A week's load, its full-contact minutes and its on-field minutes.
pub fn week_load(w: &Week, role: Role, camp: bool) -> (f64, f64, f64) {
    let mut out = (0.0, 0.0, 0.0);
    for d in &w.days {
        let (l, f) = day_load(d, role, camp);
        out.0 += l;
        out.1 += f;
        out.2 += d.field() as f64;
    }
    out
}

/// The game plan's preparation: minutes on this week's install (team
/// periods against the opponent's looks, situations, walkthroughs,
/// meetings and film).
pub fn prep_minutes(w: &Week) -> f64 {
    w.days
        .iter()
        .flat_map(|d| d.periods.iter())
        .map(|p| match p.kind {
            Kind::Team if p.what >= 1 => p.min as f64 * 1.2,
            Kind::Team => p.min as f64 * 0.5,
            Kind::Situations => p.min as f64,
            Kind::Walkthrough => p.min as f64 * 0.8,
            Kind::Meeting | Kind::Film => p.min as f64 * 0.5,
            Kind::SevenOnSeven => p.min as f64 * 0.4,
            _ => 0.0,
        })
        .sum()
}

/// The load of the days just before a game (the last three), for game-day
/// freshness: a heavy Thursday leaves less energy for Sunday's first
/// quarter (topic 4).
pub fn late_load(w: &Week, game_day: usize, role: Role) -> f64 {
    (game_day.saturating_sub(3)..game_day).map(|i| day_load(&w.days[i], role, false).0).sum()
}

/// Kickoff energy from the late load against the neutral week's
/// (proposed, not confirmed: 6% per extra neutral load, at most 15%).
pub fn freshness(late: f64, neutral_late: f64) -> f64 {
    if neutral_late <= 0.0 {
        return 1.0;
    }
    clamp(1.0 - 0.06 * (late / neutral_late - 1.0).max(0.0), 0.85, 1.0)
}
