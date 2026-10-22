//! High school football money (later extras, topic 1): every school's
//! football budget on a real school's lines (the district's allocation and
//! the coaches' stipends, the gate, the state association's playoff share,
//! pay-to-play fees where a state allows them, the booster club), kept for
//! this year and the last. At your own school the booster club's account
//! is yours: fundraisers fill it, purchases spend it, the equipment budget
//! comes from the district and helmets follow the real rules (reconditioned
//! and recertified every two years, retired at ten).

use super::data::{FUNDRAISERS, PURCHASES, hs_index, pay_to_play_share};
use super::{Book, Line, Spread, dollars, draw, month_share};
use crate::calendar::{Date, Event, Kind, Level, Loudness};
use crate::world::World;

pub const N: usize = 12;
pub const INCOME: usize = 5;

pub const DISTRICT: usize = 0;
pub const GATE: usize = 1;
pub const PLAYOFF: usize = 2;
pub const PAY_TO_PLAY: usize = 3;
pub const BOOSTER: usize = 4;
pub const STIPENDS: usize = 5;
pub const EQUIPMENT: usize = 6;
pub const TRAVEL: usize = 7;
pub const GAME_DAY: usize = 8;
pub const STADIUM: usize = 9;
pub const PURCHASES_LINE: usize = 10;
pub const MEALS: usize = 11;

pub const LINES: [Line; N] = [
    Line { name: "District allocation", income: true, spread: Spread::Even },
    Line { name: "Gate receipts", income: true, spread: Spread::Games },
    Line { name: "Playoff gate share", income: true, spread: Spread::Day },
    Line { name: "Pay-to-play fees", income: true, spread: Spread::Day },
    Line { name: "Booster club", income: true, spread: Spread::Season },
    Line { name: "Coaching stipends", income: false, spread: Spread::Season },
    Line { name: "Equipment and reconditioning", income: false, spread: Spread::Season },
    Line { name: "Buses and travel", income: false, spread: Spread::Season },
    Line { name: "Officials and game day", income: false, spread: Spread::Games },
    Line { name: "Field, lights and upkeep", income: false, spread: Spread::Even },
    Line { name: "Booster purchases", income: false, spread: Spread::Day },
    Line { name: "Team meals", income: false, spread: Spread::Season },
];

/// One school's football books: this money year (from 1 July) and the last.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct School {
    pub budget: Book<N>,
    pub now: Book<N>,
    pub last: Book<N>,
    /// The fee each player pays to play, in dollars (0 where none).
    pub fee: u16,
    /// A home crowd on a normal Friday.
    pub crowd: u32,
}

/// One line of your booster club's account.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub date: Date,
    pub what: String,
    /// Dollars in (+) or out (-).
    pub amount: f32,
}

/// A fundraiser under way: which, and the day it is counted.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Running {
    pub key: String,
    pub ends: Date,
}

/// Your high school's own money: the booster account, the equipment budget
/// and the helmets (the seat only).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Seat {
    pub school: usize,
    pub booster: f32,
    pub ledger: Vec<Entry>,
    pub running: Vec<Running>,
    /// Fundraisers and purchases already done this money year.
    pub done: Vec<String>,
    /// The money year you last asked the district for more.
    pub asked: i32,
    /// The equipment budget the district gives you this year, and spent.
    pub equipment: f32,
    pub equipment_spent: f32,
    /// The year the oldest helmets were bought, and last recertified.
    pub helmets: i32,
    pub recert: i32,
}

/// A school's assistants and what they cost (your own staff at your school).
fn staff(w: &World, h: usize) -> (usize, f64) {
    if w.seat_school() == Some(h)
        && let Some(c) = w.career.as_ref()
        && !c.hs_staff.is_empty()
    {
        return (c.hs_staff.len(), c.hs_staff.iter().map(|a| a.stipend as f64).sum());
    }
    let s = &w.high_schools[h];
    let n = (2 + s.enrollment / 400).clamp(3, 15) as usize;
    let each = (2500.0 + 5500.0 * s.resources + 400.0) * hs_index(w.year) / hs_index(2024).max(1e-9);
    // A head coach's own stipend on top (often double an assistant's).
    (n, n as f64 * each + 2.0 * each)
}

/// The state's code for a school.
fn state_code(w: &World, h: usize) -> &str {
    &w.states[w.high_schools[h].state].code
}

/// A normal home crowd: the school's size, the town's football and the
/// program's name (Texas and the Deep South fill bigger stadiums).
fn crowd(w: &World, h: usize) -> u32 {
    let s = &w.high_schools[h];
    let culture = w.states[s.state].culture;
    let base = 350.0 + 0.9 * s.enrollment as f64;
    let pull = (0.45 + 1.3 * s.prestige) * (1.0 + 0.35 * culture.clamp(-1.0, 1.5));
    let noise = 0.85 + 0.3 * draw(w, 0x45C0, h as u64, 0);
    (base * pull * noise).clamp(150.0, 14_000.0) as u32
}

/// The fee where the school charges one (a keyed draw against the state's
/// share; private schools fold it into tuition).
fn fee(w: &World, h: usize) -> u16 {
    let s = &w.high_schools[h];
    if s.private || draw(w, 0x45FE, h as u64, 0) >= pay_to_play_share(state_code(w, h)) {
        return 0;
    }
    let idx = hs_index(w.year);
    let raw = (60.0 + 140.0 * draw(w, 0x45FF, h as u64, 0) + 60.0 * s.area_money) * idx;
    ((raw / 5.0).round() * 5.0) as u16
}

/// This money year's budget (all lines, dollars of the year).
fn budget(w: &World, h: usize) -> Book<N> {
    let s = &w.high_schools[h];
    let idx = hs_index(w.year);
    let mut b = Book::new(w.year);
    let (_, stipends) = staff(w, h);
    let crowd = crowd(w, h) as f64;
    let ticket = (3.0 + 4.0 * s.area_money + 2.0 * s.prestige) * idx;
    let roster = (35.0 + s.enrollment as f64 / 28.0).clamp(28.0, 140.0);
    let homes = 5.0;
    b.l[DISTRICT] = ((10_000.0 + 13.0 * s.enrollment as f64 * (0.5 + s.area_money)) * (0.6 + 0.8 * s.resources) * idx
        + stipends) as f32;
    b.l[GATE] = (homes * crowd * ticket * 0.85) as f32;
    let last = s.seasons.last();
    let extra = last.map(|x| (x.wins + x.losses).saturating_sub(10) as f64).unwrap_or(0.0);
    b.l[PLAYOFF] = (extra * crowd * 1.4 * ticket * 0.5) as f32;
    let fee = fee(w, h) as f64;
    b.l[PAY_TO_PLAY] = (fee * roster * 0.85) as f32;
    b.l[BOOSTER] = ((2_500.0 + 45_000.0 * s.area_money * (0.3 + s.prestige)) * idx) as f32;
    b.l[STIPENDS] = stipends as f32;
    b.l[EQUIPMENT] = ((roster * 95.0 + 1_500.0) * idx * (0.7 + 0.6 * s.resources)) as f32;
    b.l[TRAVEL] = ((homes + extra) * (380.0 + 300.0 * s.area_money) * idx) as f32;
    b.l[GAME_DAY] = (homes * (700.0 + crowd * 0.35) * idx) as f32;
    b.l[STADIUM] = ((4_000.0 + 9_000.0 * s.resources + crowd * 2.0) * idx) as f32;
    b.l[MEALS] = ((10.0 + extra) * roster * 6.0 * idx * s.area_money.max(0.2)) as f32;
    b
}

/// Opens every school's books (the first day or an older file).
pub(crate) fn open(w: &mut World) {
    let n = w.high_schools.len();
    let schools: Vec<School> = (0..n)
        .map(|h| {
            let b = budget(w, h);
            School { now: Book::new(w.year), budget: b, last: Book::default(), fee: fee(w, h), crowd: crowd(w, h) }
        })
        .collect();
    w.finance.hs = schools;
    open_seat(w);
}

/// Your booster account when you take a high school job: the club's
/// balance carried from before you, the district's equipment budget and
/// the helmets' ages.
pub(crate) fn open_seat(w: &mut World) {
    let Some(h) = w.seat_school() else {
        w.finance.seat_hs = None;
        return;
    };
    if w.finance.seat_hs.as_ref().is_some_and(|s| s.school == h) {
        return;
    }
    let s = &w.high_schools[h];
    let idx = hs_index(w.year);
    let booster = ((1_500.0 + 25_000.0 * s.area_money * (0.3 + s.prestige))
        * idx
        * (0.4 + draw(w, 0x45B0, h as u64, w.year))) as f32;
    let equipment = w.finance.hs.get(h).map(|x| x.budget.l[EQUIPMENT]).unwrap_or(0.0);
    let helmets = w.year - (draw(w, 0x45E1, h as u64, w.year) * 9.0) as i32;
    let recert = w.year - 1 - (draw(w, 0x45E2, h as u64, w.year) * 2.0) as i32;
    w.finance.seat_hs = Some(Seat {
        school: h,
        booster,
        ledger: vec![Entry { date: w.today, what: "Balance when you took over".into(), amount: booster }],
        running: Vec::new(),
        done: Vec::new(),
        asked: 0,
        equipment,
        equipment_spent: 0.0,
        helmets,
        recert,
    });
}

/// The month's accruals for every school.
pub(crate) fn month(w: &mut World, m: u32) {
    let homes = |m: u32| {
        if matches!(m, 9 | 10) {
            2.0
        } else if m == 11 {
            1.0
        } else {
            0.0
        }
    };
    for s in &mut w.finance.hs {
        for (k, line) in LINES.iter().enumerate() {
            let share = match line.spread {
                Spread::Games => homes(m) / 5.0,
                // The school year: booster money and costs land August to November.
                Spread::Season => {
                    if matches!(m, 8..=11) {
                        0.2
                    } else {
                        0.2 / 8.0
                    }
                }
                sp => month_share(sp, m),
            };
            s.now.l[k] += s.budget.l[k] * share as f32;
        }
    }
    if let Some(seat) = w.finance.seat_hs.as_mut() {
        let part = if matches!(m, 8..=11) { 0.2 } else { 0.2 / 8.0 };
        seat.equipment_spent += seat.equipment * part * 0.85;
    }
}

/// 1 July: the year closes, the playoffs and fees are booked, and a new
/// budget opens.
pub(crate) fn money_year(w: &mut World) {
    let n = w.finance.hs.len().min(w.high_schools.len());
    for h in 0..n {
        let b = budget(w, h);
        let fee = fee(w, h);
        let crowd = crowd(w, h);
        let s = &mut w.finance.hs[h];
        // Fees and the playoff share land when the season is settled.
        s.now.l[PLAYOFF] += s.budget.l[PLAYOFF];
        s.now.l[PAY_TO_PLAY] += s.budget.l[PAY_TO_PLAY];
        s.last = std::mem::replace(&mut s.now, Book::new(w.year));
        s.budget = b;
        s.fee = fee;
        s.crowd = crowd;
    }
    if w.seat_school().is_some() && w.finance.seat_hs.as_ref().is_none_or(|s| Some(s.school) != w.seat_school()) {
        open_seat(w);
    }
    let year = w.year;
    let equipment = w.seat_school().and_then(|h| w.finance.hs.get(h)).map(|x| x.budget.l[EQUIPMENT]);
    if let (Some(seat), Some(eq)) = (w.finance.seat_hs.as_mut(), equipment) {
        seat.done.clear();
        seat.equipment = eq;
        seat.equipment_spent = 0.0;
        // The booster club's yearly giving lands in the account too.
        let h = seat.school;
        let gift = w.finance.hs.get(h).map(|x| x.budget.l[BOOSTER]).unwrap_or(0.0) * 0.6;
        seat.booster += gift;
        seat.ledger.push(Entry { date: w.today, what: format!("Booster club dues and pledges, {year}"), amount: gift });
        trim(seat);
    }
}

fn trim(seat: &mut Seat) {
    if seat.ledger.len() > 200 {
        let cut = seat.ledger.len() - 200;
        seat.ledger.drain(..cut);
    }
}

fn note(w: &mut World, h: usize, loud: Loudness, text: String) {
    let e = Event::new(w.today, Level::HighSchool, Kind::News, loud, text).rule("money").program(h);
    w.calendar.post(e);
}

/// Dated steps at your school: fundraisers finishing, the helmet check on
/// 1 August and the athletic director's fix on 15 August.
pub(crate) fn dated(w: &mut World, d: Date) {
    if w.finance.seat_hs.as_ref().is_some_and(|s| Some(s.school) != w.seat_school()) {
        open_seat(w);
    }
    let Some(seat) = w.finance.seat_hs.as_ref() else { return };
    let h = seat.school;
    let due: Vec<String> = seat.running.iter().filter(|r| r.ends <= d).map(|r| r.key.clone()).collect();
    for key in due {
        finish_fundraiser(w, h, &key);
    }
    let (_, m, day) = d.parts();
    let Some((helmets, recerted)) = w.finance.seat_hs.as_ref().map(|s| (s.helmets, s.recert)) else { return };
    let year = w.year;
    let old = year - helmets >= 10;
    let recert = year - recerted >= 2;
    if m == 8 && day == 1 {
        if old {
            note(
                w,
                h,
                Loudness::Big,
                format!(
                    "Equipment: your oldest helmets were bought in {}. Helmets past ten years can't be reconditioned or recertified (NOCSAE), so they must be replaced before the first game. A full set costs about {}.",
                    helmets,
                    dollars(price("helmets", year))
                ),
            );
        } else if recert {
            note(
                w,
                h,
                Loudness::Medium,
                format!(
                    "Equipment: the helmets are due for reconditioning and recertification before the first game (about {}).",
                    dollars(price("recert", year))
                ),
            );
        }
    }
    if m == 8 && day == 15 {
        if old || recert {
            let key = if old { "helmets" } else { "recert" };
            let cost = price(key, year) as f32;
            if let Some(seat) = w.finance.seat_hs.as_mut() {
                seat.equipment_spent += cost;
                if old {
                    seat.helmets = year;
                }
                seat.recert = year;
            }
            let what = if old { "replaced the expired helmets" } else { "had the helmets recertified" };
            note(
                w,
                h,
                Loudness::Medium,
                format!(
                    "Your athletic director {what} from the equipment budget ({}). It leaves less for the rest of the year.",
                    dollars(cost as f64)
                ),
            );
        }
    }
}

fn price(key: &str, year: i32) -> f64 {
    let p = PURCHASES.iter().find(|p| p.key == key).map(|p| p.price).unwrap_or(0.0);
    p * hs_index(year)
}

/// What a fundraiser brings in at a school this year: the town's money,
/// the program's pull, and a little luck.
pub fn fundraiser_take(w: &World, h: usize, key: &str) -> Option<(f64, f64)> {
    let f = FUNDRAISERS.iter().find(|f| f.key == key)?;
    let s = &w.high_schools[h];
    let mean = f.base * hs_index(w.year) * (0.45 + 0.9 * s.area_money) * (0.7 + 0.6 * s.prestige);
    Some((mean * 0.75, mean * 1.25))
}

fn finish_fundraiser(w: &mut World, h: usize, key: &str) {
    let Some((lo, hi)) = fundraiser_take(w, h, key) else { return };
    let k = FUNDRAISERS.iter().position(|f| f.key == key).unwrap_or(0) as u64;
    let take = lo + (hi - lo) * draw(w, 0x45F0 + k, h as u64, w.year);
    let name = FUNDRAISERS.iter().find(|f| f.key == key).map(|f| f.name).unwrap_or("Fundraiser");
    let today = w.today;
    if let Some(seat) = w.finance.seat_hs.as_mut() {
        seat.running.retain(|r| r.key != key);
        seat.booster += take as f32;
        seat.ledger.push(Entry { date: today, what: name.to_string(), amount: take as f32 });
        trim(seat);
    }
    if let Some(s) = w.finance.hs.get_mut(h) {
        s.now.l[BOOSTER] += take as f32;
    }
    note(w, h, Loudness::Medium, format!("{name} is in: {} for the booster club.", dollars(take)));
}

/// Starts a fundraiser at your school (one of each a year, two at once).
pub fn start_fundraiser(w: &mut World, key: &str) -> Result<String, String> {
    let f = FUNDRAISERS.iter().find(|f| f.key == key).ok_or("No such fundraiser.")?;
    let today = w.today;
    let seat = w.finance.seat_hs.as_mut().ok_or("Only a high school coach runs the booster club.")?;
    if seat.done.iter().any(|k| k == key) {
        return Err(format!("You already ran the {} this year.", f.name.to_lowercase()));
    }
    if seat.running.len() >= 2 {
        return Err("Two fundraisers are already running; the town has only so many weekends.".into());
    }
    seat.done.push(key.to_string());
    let ends = today.add(7 * f.weeks as i32);
    seat.running.push(Running { key: key.to_string(), ends });
    Ok(format!(
        "{} under way. The money is counted in {} week{}.",
        f.name,
        f.weeks,
        if f.weeks == 1 { "" } else { "s" }
    ))
}

/// Buys something from the booster account.
pub fn buy(w: &mut World, key: &str) -> Result<String, String> {
    let p = PURCHASES.iter().find(|p| p.key == key).ok_or("No such purchase.")?;
    let year = w.year;
    let cost = (p.price * hs_index(year)) as f32;
    let today = w.today;
    let seat = w.finance.seat_hs.as_mut().ok_or("Only a high school coach runs the booster club.")?;
    if seat.done.iter().any(|k| k == key) {
        return Err(format!("{} already bought this year.", p.name));
    }
    if seat.booster < cost {
        return Err(format!(
            "The booster account has {}; this costs {}.",
            dollars(seat.booster as f64),
            dollars(cost as f64)
        ));
    }
    seat.booster -= cost;
    seat.done.push(key.to_string());
    seat.ledger.push(Entry { date: today, what: p.name.to_string(), amount: -cost });
    trim(seat);
    match key {
        "helmets" => {
            seat.helmets = year;
            seat.recert = year;
        }
        "recert" => seat.recert = year,
        _ => {}
    }
    let h = seat.school;
    if let Some(s) = w.finance.hs.get_mut(h) {
        s.now.l[PURCHASES_LINE] += cost;
    }
    // Each step is worth less the better equipped the school already is.
    let school = &mut w.high_schools[h];
    let step = p.resources * (1.0 - school.resources).max(0.0) * 1.6;
    school.resources = (school.resources + step).min(1.0);
    Ok(format!("Bought: {} for {}.", p.name, dollars(cost as f64)))
}

/// Asks the district for more equipment money (once a year): a richer
/// district, a winning program and a big school make a yes likelier.
pub fn ask_district(w: &mut World) -> Result<String, String> {
    let year = w.year;
    let seat = w.finance.seat_hs.as_ref().ok_or("Only a high school coach asks the district.")?;
    if seat.asked == year {
        return Err("You already asked the district this year; the answer stands.".into());
    }
    let h = seat.school;
    let s = &w.high_schools[h];
    let last = s.seasons.last();
    let win = last.map(|x| x.wins as f64 / (x.wins + x.losses).max(1) as f64).unwrap_or(0.5);
    let chance = (0.15 + 0.35 * s.area_money + 0.25 * win + 0.15 * s.prestige).clamp(0.05, 0.85);
    let yes = draw(w, 0x45A5, h as u64, year) < chance;
    let more = (seat.equipment as f64 * (0.15 + 0.2 * draw(w, 0x45A6, h as u64, year))) as f32;
    let seat = w.finance.seat_hs.as_mut().ok_or("No seat.")?;
    seat.asked = year;
    if yes {
        seat.equipment += more;
        if let Some(b) = w.finance.hs.get_mut(h) {
            b.budget.l[EQUIPMENT] += more;
            b.budget.l[DISTRICT] += more;
        }
        Ok(format!("The district said yes: {} more for equipment this year.", dollars(more as f64)))
    } else {
        Err("The district said no: the budget is set, and other programs are asking too. Try the booster club.".into())
    }
}
