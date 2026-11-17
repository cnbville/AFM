//! Gifts (Capital lane C3; `docs/design/capital.md`, CP9, CP10, CP14).
//!
//! A donor asked for the building says yes by his bond with the school,
//! his warmth toward the coach in the room, the last two seasons and the
//! fever (`fever`), more when the building holds a room he cares about and
//! less the more he leans to NIL (`Donor::nil_lean`). What he gives is a
//! share of what he could truly give over a drive (`Donor::capacity`), read
//! the same way, and better with a better development office.
//!
//! The NIL-or-buildings split is decided donor by donor (CP9): a donor who
//! leans to NIL and gives to the building takes part of it out of his NIL
//! money (the collective's base is thinner that year), all of it when you
//! push him past his lean. Some gifts carry a catch he keeps quiet (CP10):
//! a call a year or three on about his nephew's walk-on spot; the staff's
//! read can warn you (`screen::catch_read`). Lead gifts need the head coach
//! in the room (CP14): an AI head coach meets his lead donors through the
//! drive, you meet yours in your week (lane C4's slots, or `ask`); a drive
//! left to the development office gets only a courtesy gift from them.
//!
//! Every program asks on the same odds (CP27). Sizes are Claude's
//! (proposed, not confirmed).

use super::Capital;
use super::donors::{Donor, STRANGER};
use super::drive::Stage;
use crate::calendar::{Date, Level, Loudness};
use crate::rng::clamp;
use crate::world::World;
use crate::world::folk::Team;
use crate::world::folk::cast::Want;

/// A yes's base chance, and what affinity, warmth and winning add.
pub const BASE_YES: f64 = 0.45;
pub const YES_AFFINITY: f64 = 0.6;
pub const YES_WARMTH: f64 = 0.8;
pub const YES_WINS: f64 = 0.4;
/// The fever lifts the chance and the size (CP7, CP8).
pub const FEVER_CHANCE: f64 = 0.5;
pub const FEVER_SIZE: f64 = 0.4;
/// The share of his capacity a yes gives, before the rest.
pub const BASE_SHARE: f64 = 0.6;
/// A building without a room he cares about reads at this much.
pub const OFF_ROOM: f64 = 0.6;
/// A donor's lean taken out of his NIL money when he gives to the
/// building (CP9): half of a year's payment per unit of NIL lean, all of
/// it when pushed.
pub const NIL_DENT: f64 = 0.5;
pub const NIL_DENT_PUSHED: f64 = 1.0;
/// A pushed ask's chance, times (0.5 + his building side), and what it
/// costs his warmth.
pub const PUSH_WARMTH: f32 = -0.04;
/// Below this building side he says he'd rather give to NIL (and above
/// one minus it, to the building) unless pushed.
pub const RATHER: f64 = 0.4;
/// What a lead donor gives the development office without the head coach
/// in the room: a courtesy gift, this share of his gift.
pub const COURTESY: f64 = 0.25;
/// A catch's chance: his `collects` times this (CP10).
pub const CATCH: f64 = 0.6;
/// The AD grants a catch this often when it falls to him.
pub const AD_GRANTS: f64 = 0.6;
/// What a catch does to his warmth: granted, refused.
pub const CATCH_KEPT: f32 = 0.05;
pub const CATCH_REFUSED: f32 = -0.2;
/// Days you have to answer a catch before the AD does.
pub const CATCH_DAYS: i32 = 30;

/// A donor's odds when asked for the building.
#[derive(Clone, Copy, Debug)]
pub struct Odds {
    pub chance: f64,
    /// The share of his capacity a yes gives.
    pub share: f64,
}

/// His building side, 0 (only NIL) to 1 (only buildings).
pub fn building_side(d: &Donor) -> f64 {
    ((1.0 + d.nil_lean as f64) / 2.0).clamp(0.0, 1.0)
}

/// The share of his giving that school `p` pulls (CP28).
fn pull(d: &Donor, p: usize) -> f64 {
    d.schools().into_iter().find(|s| s.0 == p as u32).map_or(0.0, |s| s.1 as f64)
}

/// The last two seasons' win share.
pub fn wins(w: &World, p: usize) -> f64 {
    let s = &w.programs[p].seasons;
    let v: Vec<f64> = s.iter().rev().take(2).map(|s| s.wins as f64 / (s.wins + s.losses).max(1) as f64).collect();
    if v.is_empty() { 0.5 } else { v.iter().sum::<f64>() / v.len() as f64 }
}

/// A donor's odds for a building holding `rooms` at program `p`, with
/// `coach` in the room (None: the office alone).
pub fn odds(w: &World, p: usize, d: &Donor, rooms: u16, coach: Option<u32>, fever: f64) -> Odds {
    let fit = if d.rooms.iter().any(|&r| rooms & (1u16 << r) != 0) { 1.0 } else { OFF_ROOM };
    odds_fit(w, p, d, fit, coach, fever)
}

/// The odds with the room fit given: 1.0 where no building is in it (an
/// NIL ask), else how well the drive's rooms match his own.
fn odds_fit(w: &World, p: usize, d: &Donor, fit: f64, coach: Option<u32>, fever: f64) -> Odds {
    let a = d.affinity as f64;
    let warm = coach.map_or(STRANGER, |c| d.warmth_to(c)) as f64;
    let wp = wins(w, p);
    let side = building_side(d);
    let chance =
        clamp(BASE_YES + YES_AFFINITY * (a - 0.5) + YES_WARMTH * (warm - 0.5) + YES_WINS * (wp - 0.5), 0.03, 0.9)
            * (1.0 + FEVER_CHANCE * fever)
            * (0.6 + 0.4 * fit)
            * (0.4 + 0.6 * side);
    let office = super::screen::office(w, p);
    let share = clamp(
        BASE_SHARE
            * (0.5 + a)
            * (1.0 + FEVER_SIZE * fever)
            * (0.3 + 0.7 * side)
            * fit
            * (0.8 + 0.4 * office)
            * (0.7 + 0.6 * warm),
        0.02,
        1.0,
    ) * pull(d, p);
    Odds { chance: chance.min(0.95), share }
}

/// The expected gift (the study's read, CP15).
pub fn expected(w: &World, p: usize, d: &Donor, rooms: u16, coach: Option<u32>, fever: f64) -> f64 {
    let o = odds(w, p, d, rooms, coach, fever);
    o.chance * o.share * d.capacity
}

/// Years a pledge is paid over: three to five, more for a bigger gift.
fn years_for(amount: f64, capacity: f64) -> u8 {
    if amount >= 0.5 * capacity {
        5
    } else if amount >= 0.2 * capacity {
        4
    } else {
        3
    }
}

/// A yes to the drive: the pledge, the NIL money it takes (CP9) and a catch
/// he may keep quiet (CP10). Returns the pledge id.
#[allow(clippy::too_many_arguments)]
pub(crate) fn give(
    w: &mut World,
    drive: u32,
    p: usize,
    donor: u32,
    amount: f64,
    closer: Option<u32>,
    pushed: bool,
    room: Option<u8>,
) -> u32 {
    let day = w.today;
    let Some(d) = w.capital.donor(donor).cloned() else { return 0 };
    let years = years_for(amount, d.capacity);
    let id = super::pledges::make(w, donor, p as u32, amount, years, closer, room, day);
    if let Some(x) = w.capital.pledges.iter_mut().rev().find(|x| x.id == id) {
        x.drive = drive;
    }
    if let Some(x) = w.capital.drives.iter_mut().find(|x| x.id == drive) {
        x.asked.push((donor, super::drive::GAVE));
    }
    // The NIL money it takes (CP9): only in the NIL era, where donors lean.
    let lean = (1.0 - building_side(&d)) * if pushed { NIL_DENT_PUSHED } else { NIL_DENT };
    if lean > 0.0 && crate::world::ncaa::nil_era(w.year) {
        let c = &mut w.programs[p].collective;
        c.base = (c.base - lean * amount / years as f64).max(5_000.0);
    }
    // The catch (CP10).
    let key = (id as u64) << 8;
    if super::draw(w, 20, key) < CATCH * d.collects as f64 {
        let kind = catch_kind(&d, super::draw(w, 21, key));
        let due = day.add(365 + (super::draw(w, 22, key) * 730.0) as i32);
        let cid = w.capital.id();
        w.capital.catches.push(Catch {
            id: cid,
            pledge: id,
            donor,
            program: p as u32,
            kind,
            due,
            called: None,
            answer: None,
        });
    }
    id
}

/// What he will call about later (CP10), from what he wants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CatchKind {
    /// His nephew on the team as a walk-on.
    WalkOn,
    /// A say in the next coordinator's hire.
    SayInHire,
    /// The sideline, the team plane, the locker room after games.
    Access,
    /// A kid from his old high school gets a scholarship look.
    LocalKid,
    /// His name bigger on the building, on the field's end zone.
    Name,
}

impl CatchKind {
    pub fn words(self) -> &'static str {
        match self {
            CatchKind::WalkOn => "his nephew's walk-on spot",
            CatchKind::SayInHire => "a say in the next coordinator hire",
            CatchKind::Access => "a sideline pass and a seat on the team plane",
            CatchKind::LocalKid => "a scholarship look for a kid from his old high school",
            CatchKind::Name => "his name on the end zone too",
        }
    }
}

fn catch_kind(d: &Donor, u: f64) -> CatchKind {
    let mut v: Vec<CatchKind> = d
        .wants
        .iter()
        .filter_map(|w| match w {
            Want::SayInHires => Some(CatchKind::SayInHire),
            Want::SayOnPlayer => Some(CatchKind::WalkOn),
            Want::Access => Some(CatchKind::Access),
            Want::Building => Some(CatchKind::Name),
            Want::OldSchool => Some(CatchKind::LocalKid),
            _ => None,
        })
        .collect();
    if v.is_empty() {
        v = vec![CatchKind::WalkOn, CatchKind::Access];
    }
    v[((u * v.len() as f64) as usize).min(v.len() - 1)]
}

/// A catch a gift carried (CP10).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Catch {
    pub id: u32,
    pub pledge: u32,
    pub donor: u32,
    pub program: u32,
    pub kind: CatchKind,
    /// The day he calls about it.
    pub due: Date,
    pub called: Option<Date>,
    /// Granted or refused, once answered.
    pub answer: Option<bool>,
}

/// The catches' day: a call that comes due rings (yours waits for you
/// `CATCH_DAYS`, then the AD answers); every other program's AD answers it
/// at once, on the same odds.
pub(crate) fn catches(w: &mut World, d: Date) {
    let due: Vec<usize> = (0..w.capital.catches.len())
        .filter(|&k| {
            let c = &w.capital.catches[k];
            c.answer.is_none()
                && (c.called.is_none() && c.due <= d || c.called.is_some_and(|x| x.days_to(d) >= CATCH_DAYS))
        })
        .collect();
    for k in due {
        let (id, p, called, donor, kind, pledge) = {
            let c = &w.capital.catches[k];
            (c.id, c.program as usize, c.called, c.donor, c.kind, c.pledge)
        };
        // The pledge was pulled or he is gone: nothing to call about.
        let live = w.capital.pledges.iter().any(|x| x.id == pledge && x.pulled.is_none())
            && w.capital.donor(donor).is_some_and(|x| x.alive());
        if !live {
            w.capital.catches[k].answer = Some(false);
            w.capital.catches[k].called = Some(d);
            continue;
        }
        if called.is_none() && w.seat_program(p) {
            w.capital.catches[k].called = Some(d);
            let name = w.capital.donor(donor).map_or_else(String::new, |x| x.name.clone());
            let text = format!("{name} calls about his gift: he'd like {}.", kind.words());
            let e = crate::world::finance::story(w, Level::College, Loudness::Medium, text).program(p);
            crate::world::finance::post(w, e);
            continue;
        }
        let yes = super::draw(w, 23, id as u64) < AD_GRANTS;
        answer(w, k, yes);
    }
}

/// You answer a catch (CP10): granted warms him a little; refused cools him,
/// and the more he likes to collect the likelier he pulls the rest.
pub fn answer_catch(w: &mut World, id: u32, yes: bool) -> Result<String, String> {
    let Some(k) = w.capital.catches.iter().position(|c| c.id == id) else { return Err("No such call.".into()) };
    if w.capital.catches[k].answer.is_some() {
        return Err("That's been answered.".into());
    }
    if !w.seat_program(w.capital.catches[k].program as usize) {
        return Err("Not your program's donor.".into());
    }
    Ok(answer(w, k, yes))
}

fn answer(w: &mut World, k: usize, yes: bool) -> String {
    let (id, p, donor, kind, pledge) = {
        let c = &mut w.capital.catches[k];
        c.answer = Some(yes);
        c.called.get_or_insert(w.today);
        (c.id, c.program as usize, c.donor, c.kind, c.pledge)
    };
    let coach = w.programs[p].head_coach.map(|c| c as u32);
    let collects = w.capital.donor(donor).map_or(0.0, |d| d.collects as f64);
    if let (Some(c), Some(d)) = (coach, w.capital.donor_mut(donor)) {
        let b = d.bond_mut(c);
        b.warmth = (b.warmth + if yes { CATCH_KEPT } else { CATCH_REFUSED }).clamp(0.0, 1.0);
    }
    if yes {
        // A donor's say in a hire sits badly with your AD.
        if kind == CatchKind::SayInHire && w.seat_program(p) {
            let b = crate::world::folk::boss::boss(w, Team::College(p));
            w.folk.mine.nudge(b.key(), -0.02);
        }
        return format!("Granted: {}.", kind.words());
    }
    if super::draw(w, 24, id as u64) < collects {
        let today = w.today;
        if let Some(x) = w.capital.pledges.iter_mut().find(|x| x.id == pledge && x.open()) {
            x.pulled = Some(today);
            return "Refused. He pulls the rest of his pledge.".into();
        }
    }
    "Refused. He's cooler toward the staff now.".into()
}

/// The side of an ask (CP9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Side {
    Building,
    Nil,
}

/// A donor's answer to your ask.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum Answer {
    /// He pledges to the building: the pledge id.
    Pledged(u32),
    /// He gives to the collective: the money a year it adds.
    Nil(f64),
    /// He'd rather give to the other side (ask again with a push, or for
    /// that side).
    Rather(Side),
    No,
}

/// You ask a donor yourself (CP9; lane C4's donor slots call it): for the
/// drive's building or for NIL, pushing him past his lean or not. You are in
/// the room, so it counts as a meeting, and his lead gift can come. One ask
/// a donor in each of the drive's stages; an answer of "rather the other
/// side" leaves the ask open.
pub fn ask(w: &mut World, p: usize, donor: u32, side: Side, push: bool) -> Result<(Answer, String), String> {
    let Some(you) = w.career.as_ref().map(|c| c.coach as u32) else { return Err("No career.".into()) };
    if !w.seat_program(p) || w.programs[p].head_coach != Some(you as usize) {
        return Err("Only the head coach closes the big gifts.".into());
    }
    let Some(d) = w.capital.donor(donor).cloned().filter(|d| d.alive()) else { return Err("No such donor.".into()) };
    if pull(&d, p) <= 0.0 {
        return Err("He gives to another school.".into());
    }
    let open = super::drive::open_drive(&w.capital, p);
    let (drive, stage, rooms) = match open {
        Some(k) => {
            let x = &w.capital.drives[k];
            (x.id, x.stage, x.rooms)
        }
        None if side == Side::Building => return Err("There's no drive to give to.".into()),
        None => (0, Stage::Closed, 0),
    };
    if side == Side::Building && !matches!(stage, Stage::Quiet | Stage::Public) {
        return Err("The drive isn't asking for gifts yet.".into());
    }
    let mark = stage as u8;
    if side == Side::Building {
        let x = w.capital.drives.iter().find(|x| x.id == drive);
        if x.is_some_and(|x| x.asked.contains(&(donor, super::drive::GAVE))) {
            return Err("He has given to this drive already.".into());
        }
        if x.is_some_and(|x| x.asked.contains(&(donor, mark))) {
            return Err("You've asked him in this stage of the drive already.".into());
        }
    } else if w.capital.nil_asks.contains(&(donor, w.year)) {
        return Err("You've asked him for NIL money this year already.".into());
    }
    let today = w.today;
    super::donors::met(w, donor, you, today);
    let bside = building_side(&d);
    let fever = super::fever::fever(w, p);
    // An NIL ask has no building in it, so no room fit.
    let mut o = match side {
        Side::Building => odds(w, p, &d, rooms, Some(you), fever),
        Side::Nil => odds_fit(w, p, &d, 1.0, Some(you), fever),
    };
    // The NIL ask reads his NIL side the same way.
    if side == Side::Nil {
        let flip = (0.4 + 0.6 * (1.0 - bside)) / (0.4 + 0.6 * bside);
        o.chance = (o.chance * flip).min(0.95);
        o.share *= (0.3 + 0.7 * (1.0 - bside)) / (0.3 + 0.7 * bside);
    }
    let leans_away = match side {
        Side::Building => bside < RATHER,
        Side::Nil => bside > 1.0 - RATHER,
    };
    if leans_away && !push {
        let other = if side == Side::Building { Side::Nil } else { Side::Building };
        let words = match other {
            Side::Nil => format!("{} would rather pay your players than pour concrete.", d.name),
            Side::Building => format!("{} would rather his money went into a building.", d.name),
        };
        return Ok((Answer::Rather(other), words));
    }
    if push && leans_away {
        o.chance *= 0.5 + if side == Side::Building { bside } else { 1.0 - bside };
        if let Some(x) = w.capital.donor_mut(donor) {
            let b = x.bond_mut(you);
            b.warmth = (b.warmth + PUSH_WARMTH).max(0.0);
        }
    }
    if side == Side::Building {
        if let Some(x) = w.capital.drives.iter_mut().find(|x| x.id == drive) {
            x.asked.push((donor, mark));
        }
    } else {
        let y = w.year;
        w.capital.nil_asks.retain(|a| a.1 >= y);
        w.capital.nil_asks.push((donor, y));
    }
    let key = (donor as u64) << 24 ^ (drive as u64) << 4 ^ mark as u64 ^ (side as u64) << 3;
    if super::draw(w, 25, key) >= o.chance {
        return Ok((Answer::No, format!("{} says not now.", d.name)));
    }
    let amount = o.share * d.capacity;
    match side {
        Side::Building => {
            let room = d.rooms.iter().copied().find(|&r| rooms & (1u16 << r) != 0);
            let id = give(w, drive, p, donor, amount, Some(you), push && leans_away, room);
            let words = format!("{} pledges {} to the drive.", d.name, crate::world::finance::dollars(amount));
            Ok((Answer::Pledged(id), words))
        }
        Side::Nil => {
            let yearly = amount / 3.0;
            let c = &mut w.programs[p].collective;
            c.base += yearly;
            let words =
                format!("{} puts {} a year into the collective.", d.name, crate::world::finance::dollars(yearly));
            Ok((Answer::Nil(yearly), words))
        }
    }
}

/// Estates (lane C1's) go to the program's drive: one restricted to a room
/// waits for a drive holding it, up to three years (CP17's rooms).
pub(crate) fn estates(w: &mut World) {
    let year = w.year;
    for k in 0..w.capital.estates.len() {
        let e = w.capital.estates[k];
        if e.paid {
            continue;
        }
        let Some(dk) = super::drive::open_drive(&w.capital, e.program as usize) else { continue };
        let (drive, rooms) = (w.capital.drives[dk].id, w.capital.drives[dk].rooms);
        let fits = e.room.is_none_or(|r| rooms & (1u16 << r) != 0) || year - e.year >= 3;
        if !fits {
            continue;
        }
        let id = w.capital.id();
        w.capital.pledges.push(super::Pledge {
            id,
            donor: e.donor,
            program: e.program,
            total: e.amount,
            paid: e.amount,
            made: Some(w.today),
            years: 1,
            tied: None,
            pulled: None,
            room: e.room,
            drive,
        });
        w.capital.estates[k].paid = true;
    }
}

/// Each July a pledge pays its year (a pulled one keeps what it paid).
pub(crate) fn pay(cap: &mut Capital) {
    for x in cap.pledges.iter_mut().filter(|x| x.open()) {
        x.paid = (x.paid + x.total / x.years.max(1) as f64).min(x.total);
    }
}
