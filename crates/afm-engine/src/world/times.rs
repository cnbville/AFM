//! Recorded times (speed update S6-S6f; minor bugs 22, 23 and 26): every
//! sprint a high schooler runs where someone holds a watch or a laser,
//! stored on the day he runs it with the event, how it was timed and
//! where. Spring track meets and the state meet (MileSplit and
//! athletic.net print every result), the colleges' June camps, the
//! recruiting sites' regional showcases and their national finals, and the
//! forty his high school lists for him. Stars and offers read his speed
//! and burst from these times (`seen`), never from his hidden future; a kid
//! nobody timed is judged by how fast he looks on film.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::season::{key_normal, key_uniform};
use super::{Person, Stage, World};
use crate::attributes::Attr;
use crate::calendar::{Date, Weekday};
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::clamp;

/// What he ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// The 40-yard dash.
    Forty,
    /// The forty's first 10 yards (his burst).
    Split,
    /// The 100 meters.
    M100,
    /// The 200 meters.
    M200,
}

/// How it was timed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Timed {
    /// Laser gates at a camp or showcase.
    Laser,
    /// Fully automatic timing at a track meet.
    Fat,
    /// A stopwatch: 0.2 to 0.3 seconds fast (S6b).
    Hand,
}

/// Where he ran it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum At {
    /// A spring track meet (his season's best is kept).
    Meet,
    /// His state's track meet.
    StateMeet,
    /// A college's one-day camp, by program.
    Camp(u16),
    /// A recruiting site's regional showcase.
    Regional,
    /// The sites' national showcase finals.
    National,
    /// The forty his high school lists for him (S6c).
    School,
}

/// One recorded time.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Time {
    pub event: Event,
    pub timed: Timed,
    /// Seconds, as printed.
    pub secs: f32,
    pub date: Date,
    pub at: At,
    /// His weight that day, pounds (a camp weighs him; a meet goes by the roster).
    pub lb: u16,
}

/// Every recorded time, by player id, and this spring's runners.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Times {
    pub by_id: BTreeMap<u32, Vec<Time>>,
    /// This spring's track runners (people indexes) and the year.
    #[serde(skip)]
    runners: (i32, Vec<usize>),
}

impl Times {
    /// His times, oldest first.
    pub fn of(&self, id: u32) -> &[Time] {
        self.by_id.get(&id).map_or(&[], |v| v.as_slice())
    }
}

/// The time an event takes him with no wind, nerves or watch error: the
/// combine's forty fit (`links::forty`), the split from a burst-heavy mix,
/// and the sprints from a speed-heavy one (a 4.40 forty runs about a 10.7
/// 100m, a 4.60 about 11.2; the 200m is twice the 100m and a quarter second).
fn base(event: Event, speed: f64, accel: f64, lb: f64) -> f64 {
    let run = |a: f64| 5.56 - 0.0124 * (a * speed + (1.0 - a) * accel) + crate::body::forty_weight(lb);
    match event {
        Event::Forty => run(0.75),
        Event::Split => 0.348 * run(0.25),
        Event::M100 => 2.5 * run(0.9) - 0.3,
        Event::M200 => 5.0 * run(0.9) - 0.35,
    }
}

/// How the event reads speed and burst (its share of top speed) and how
/// far one time moves on a rating point.
fn shape(event: Event) -> (f64, f64) {
    match event {
        Event::Forty => (0.75, 0.0124),
        Event::Split => (0.25, 0.348 * 0.0124),
        Event::M100 => (0.9, 2.5 * 0.0124),
        Event::M200 => (0.9, 5.0 * 0.0124),
    }
}

/// A run's spread, seconds: a laser forty about 0.05, its split 0.03, a
/// fully timed 100m 0.08 (wind and form), a 200m 0.15.
fn spread(event: Event) -> f64 {
    match event {
        Event::Forty => 0.05,
        Event::Split => 0.03,
        Event::M100 => 0.08,
        Event::M200 => 0.15,
    }
}

/// How fast a hand time runs (Mann et al., JSCR 2015: hand 4.90 against
/// electronic 5.12 on 81 Division I players), and the extra spread.
const HAND_FAST: f64 = 0.24;
const HAND_SD: f64 = 0.10;
/// A high school's own listed forty runs faster still: a coach rounds his
/// players down (proposed, not confirmed: another 0.05 to 0.15 seconds).
const SCHOOL_FAST: (f64, f64) = (0.05, 0.10);

/// His age on a day, in years with the part since his birthday.
fn age_at(p: &Person, d: Date) -> f64 {
    if p.born == crate::calendar::born::UNKNOWN {
        p.player.age as f64 + f64::from(p.player.grow)
    } else {
        (d.0 - p.born.0) as f64 / 365.25
    }
}

/// The share of a grown man's speed a typical boy has at an age (the speed
/// curve's average bloomer, `development::speed_with_body`).
fn typical_share(age: f64) -> f64 {
    crate::development::speed_with_body(age, 0.0)
}

/// One run, timed and stored (`key` keeps the draw off every stream).
fn run(w: &mut World, i: usize, event: Event, timed: Timed, at: At, d: Date, key: u64) -> f32 {
    let p = &w.people[i];
    // He runs on his real weight; a meet or his school only knows the
    // listed one, so that is the weight the time is read against.
    let real = crate::body::body_now(&p.player).weight;
    let lb = match at {
        At::Meet | At::StateMeet | At::School => super::scouting::file::listed(&p.player).1,
        _ => real,
    };
    let a = &p.player.attributes;
    let id = p.player.id as u64;
    let mut t = base(event, a.get(Attr::Speed), a.get(Attr::Acceleration), real) + spread(event) * key_normal(key, id);
    let step = match timed {
        Timed::Hand => {
            t -= HAND_FAST + HAND_SD * key_normal(key ^ 0x4A2D, id);
            if at == At::School {
                t -= SCHOOL_FAST.0 + SCHOOL_FAST.1 * key_uniform(key ^ 0x5C01, id);
            }
            if matches!(event, Event::M100 | Event::M200) { 0.1 } else { 0.01 }
        }
        _ => 0.01,
    };
    let secs = ((t / step).round() * step) as f32;
    let time = Time { event, timed, secs, date: d, at, lb: lb.round() as u16 };
    let v = w.times.by_id.entry(p.player.id).or_default();
    // A meet keeps his season's best by event and watch, as a meet result
    // page lists his best mark; the school lists one forty a year.
    if matches!(at, At::Meet | At::School)
        && let Some(old) =
            v.iter_mut().find(|x| x.at == at && x.event == event && x.timed == timed && x.date.year() == d.year())
    {
        if at == At::School || secs < old.secs {
            *old = time;
        }
        return secs;
    }
    v.push(time);
    secs
}

// --- The year ----------------------------------------------------------------------------

/// The spring track season's dates (S6a): meets every Saturday from the
/// second Saturday of March to the second of May, the state meets on the
/// third Saturday of May (proposed, not confirmed: one date for every state).
pub fn track_dates(year: i32) -> (Date, Date, Date) {
    let first = Date::nth_weekday(year, 3, Weekday::Sat, 2);
    let last = Date::nth_weekday(year, 5, Weekday::Sat, 2);
    (first, last, Date::nth_weekday(year, 5, Weekday::Sat, 3))
}

/// The recruiting sites' regional showcases (S6d): eight Sundays from March
/// to June, each in a football state, open to the rated kids within a long
/// drive; and their national finals on the fourth Sunday of June (as The
/// Opening and the Elite Series finals; proposed, not confirmed).
pub fn showcase_dates(year: i32) -> ([(Date, &'static str); 8], Date) {
    let sun = |m: u32, n: u32| Date::nth_weekday(year, m, Weekday::Sun, n);
    (
        [
            (sun(3, 1), "TX"),
            (sun(3, 3), "FL"),
            (sun(4, 1), "GA"),
            (sun(4, 3), "CA"),
            (sun(5, 1), "OH"),
            (sun(5, 3), "NC"),
            (sun(6, 1), "LA"),
            (sun(6, 3), "PA"),
        ],
        sun(6, 4),
    )
}

/// The day the high schools send in their players' questionnaires (S6c).
pub fn school_forty_day(year: i32) -> Date {
    Date::ymd(year, 8, 20)
}

/// The day's timing (called by the clock every day).
pub(crate) fn day(w: &mut World, d: Date) {
    let (y, m, dd) = d.parts();
    let (first, last, state) = track_dates(y);
    if d.weekday() == Weekday::Sat && d >= first && d <= last {
        super::clock::timed("track", || meet(w, d));
    }
    if d == state {
        super::clock::timed("track", || state_meet(w, d));
    }
    let (regionals, finals) = showcase_dates(y);
    if let Some(&(_, host)) = regionals.iter().find(|r| r.0 == d) {
        super::clock::timed("showcase", || regional(w, d, host));
    }
    if d == finals {
        super::clock::timed("showcase", || national(w, d));
    }
    if m == 6 {
        super::clock::timed("camps", || camps(w, d));
    }
    if d == school_forty_day(y) {
        super::clock::timed("school forties", || school_forties(w, d));
    }
    // A kid's times go with him when he leaves high school.
    if (m, dd) == (7, 1) {
        let people = &w.people;
        w.times.by_id.retain(|&id, _| {
            people
                .binary_search_by_key(&id, |p| p.player.id)
                .is_ok_and(|i| matches!(people[i].stage, Stage::HighSchool | Stage::PrepSchool))
        });
    }
}

/// How likely a boy is to run spring track (S6a): skill players most, the
/// big men rarely, and the faster he is the likelier (proposed, not
/// confirmed: about 45% of receivers, backs and defensive backs at an
/// average speed, 18% of quarterbacks, tight ends and linebackers, 4% of
/// linemen).
fn runs_track(p: &Person) -> f64 {
    let base = match p.player.position {
        Position::WR | Position::CB | Position::RB | Position::S => 0.45,
        Position::QB | Position::TE | Position::LB | Position::EDGE => 0.18,
        Position::DT | Position::OT | Position::IOL => 0.04,
        Position::K | Position::P | Position::LS => 0.05,
    };
    let a = &p.player.attributes;
    let mix = 0.9 * a.get(Attr::Speed) + 0.1 * a.get(Attr::Acceleration);
    base * clamp(0.5 + 0.035 * (mix - 62.0), 0.2, 1.6)
}

/// This spring's runners, drawn once a year (and again after a load).
fn runners(w: &mut World, y: i32) {
    if w.times.runners.0 == y && !w.times.runners.1.is_empty() {
        return;
    }
    let seed = w.options.seed as u64 ^ 0x7AC0_0000 ^ (y as u64) << 8;
    let list: Vec<usize> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage == Stage::HighSchool && key_uniform(seed, p.player.id as u64) < runs_track(p))
        .map(|(i, _)| i)
        .collect();
    w.times.runners = (y, list);
}

fn meet(w: &mut World, d: Date) {
    let y = d.year();
    runners(w, y);
    let key = w.options.seed as u64 ^ 0x7AC0_0000 ^ (y as u64) << 8 ^ (d.0 as u64) << 20;
    let runners = std::mem::take(&mut w.times.runners.1);
    for &i in &runners {
        let p = &w.people[i];
        if p.stage != Stage::HighSchool {
            continue;
        }
        let id = p.player.id as u64;
        // He runs most Saturdays; a meet has the electric timing three times in four.
        if key_uniform(key ^ 1, id) >= 0.7 {
            continue;
        }
        let hs = p.hs as u64;
        let timed = if key_uniform(key ^ 2, hs) < 0.75 { Timed::Fat } else { Timed::Hand };
        run(w, i, Event::M100, timed, At::Meet, d, key ^ 3);
        if key_uniform(key ^ 4, id) < 0.6 {
            run(w, i, Event::M200, timed, At::Meet, d, key ^ 5);
        }
    }
    w.times.runners.1 = runners;
}

/// A runner's best fully timed 100m this spring (a hand time counts at its
/// known head start).
fn best_100(w: &World, id: u32, y: i32) -> Option<f64> {
    w.times
        .of(id)
        .iter()
        .filter(|t| t.event == Event::M100 && t.date.year() == y && t.at == At::Meet)
        .map(|t| t.secs as f64 + if t.timed == Timed::Hand { HAND_FAST } else { 0.0 })
        .min_by(f64::total_cmp)
}

/// The state meets (S6a): each state's 16 fastest in the 100m run it, fully timed.
fn state_meet(w: &mut World, d: Date) {
    let y = d.year();
    let seed = w.options.seed as u64 ^ 0x57A7_0000 ^ (y as u64) << 8;
    runners(w, y);
    let mut by_state: Vec<Vec<(f64, usize)>> = vec![Vec::new(); w.states.len()];
    for &i in &w.times.runners.1 {
        let p = &w.people[i];
        if p.stage != Stage::HighSchool {
            continue;
        }
        if let Some(t) = best_100(w, p.player.id, y) {
            by_state[w.high_schools[p.hs].state].push((t, i));
        }
    }
    let mut fastest: Option<(f32, usize)> = None;
    for list in &mut by_state {
        list.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        list.truncate(16);
        for &(_, i) in list.iter() {
            let t = run(w, i, Event::M100, Timed::Fat, At::StateMeet, d, seed);
            if key_uniform(seed ^ 9, w.people[i].player.id as u64) < 0.7 {
                run(w, i, Event::M200, Timed::Fat, At::StateMeet, d, seed ^ 0x200);
            }
            if fastest.is_none_or(|f| t < f.0) {
                fastest = Some((t, i));
            }
        }
    }
    if w.recording
        && let Some((t, i)) = fastest
    {
        use crate::calendar::{Event as CalEvent, Kind, Level, Loudness};
        let p = &w.people[i];
        let school = &w.high_schools[p.hs].name;
        let text = format!(
            "State track meets: the fastest 100m in the country is {t:.2} by {}, a {} at {school}.",
            p.name, p.player.position
        );
        let e = CalEvent::new(d, Level::HighSchool, Kind::Date, Loudness::Small, text)
            .person(p.player.id, &p.name)
            .hs_link(p.hs, school);
        w.calendar.post(e);
    }
}

/// Who a showcase or a camp may draw: a high schooler in the next three
/// classes, not a kicker, punter or snapper (they have their own camps).
fn camp_age(p: &Person, y: i32) -> Option<f64> {
    if p.stage != Stage::HighSchool || p.player.position.is_specialist() {
        return None;
    }
    match p.class_of - y {
        1 => Some(1.0),
        2 => Some(0.8),
        3 => Some(0.3),
        _ => None,
    }
}

/// A laser forty with its split, and your staff's look when he is on your board.
fn test(w: &mut World, i: usize, at: At, d: Date, key: u64, mine: &[usize]) {
    run(w, i, Event::Forty, Timed::Laser, at, d, key);
    run(w, i, Event::Split, Timed::Laser, at, d, key ^ 0x5B17);
    if matches!(at, At::Regional | At::National) && mine.binary_search(&i).is_ok() {
        let id = w.people[i].player.id;
        super::scouting::dept::add(w, id, super::scouting::readers::PERSONNEL, super::scouting::Kind::Showcase, d);
    }
}

/// Your board's recruits, when you hold a college seat (a showcase is open
/// to every staff, so yours sees the ones it follows).
fn your_board(w: &World) -> Vec<usize> {
    let Some(super::scouting::readers::Staff::Program(prog)) = super::scouting::readers::seat_staff(w) else {
        return Vec::new();
    };
    let mut v = super::scouting::dept::watch(w, prog);
    v.sort_unstable();
    v
}

/// A regional showcase (S6d, minor bug 26): rated kids and the ones with
/// offers or a fast track time, within about 500 miles, whose families can
/// make the trip.
fn regional(w: &mut World, d: Date, host: &str) {
    let y = d.year();
    let Some(hs) = w.states.iter().position(|s| s.code == host) else { return };
    let key = w.options.seed as u64 ^ 0x5E61_0000 ^ (d.0 as u64) << 20;
    let mine = your_board(w);
    let goes: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            let Some(age) = camp_age(p, y) else { return false };
            if super::geo::miles(hs, p.home_state) > 500.0 {
                return false;
            }
            let fast = best_100(w, p.player.id, y).is_some_and(|t| t <= 11.0);
            let invited = p.stars >= 2 || !p.cal.offers.is_empty() || !p.cal.early_offers.is_empty() || fast;
            invited && key_uniform(key, p.player.id as u64) < 0.45 * age * (0.3 + 0.7 * p.bg.money)
        })
        .collect();
    for i in goes {
        test(w, i, At::Regional, d, key ^ 1, &mine);
    }
}

/// The national finals (S6d): the next class's best 150 on the services'
/// read and its 40 fastest timed kids are invited.
fn national(w: &mut World, d: Date) {
    let y = d.year();
    let key = w.options.seed as u64 ^ 0x0FE2_0000 ^ (d.0 as u64) << 20;
    let mut best: Vec<(f64, usize)> = Vec::new();
    let mut fast: Vec<(f64, usize)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if camp_age(p, y) != Some(1.0) {
            continue;
        }
        if p.stars >= 3
            && let Some(r) = p.cal.service_read
        {
            best.push((r, i));
        }
        let s = seen(w, p);
        if s.timed
            && let Some(mix) = crate::body::play_speed(p.player.position, s.grown.0, s.grown.1)
        {
            fast.push((mix, i));
        }
    }
    best.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    fast.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut invited: Vec<(usize, f64)> = best.iter().take(150).map(|x| (x.1, 0.85)).collect();
    invited.extend(fast.iter().take(40).map(|x| (x.1, 0.6)));
    invited.sort_by_key(|x| x.0);
    invited.dedup_by_key(|x| x.0);
    let mine = your_board(w);
    for (i, odds) in invited {
        if key_uniform(key, w.people[i].player.id as u64) < odds {
            test(w, i, At::National, d, key ^ 1, &mine);
        }
    }
}

/// How many June camps a kid goes to, on average (S6d; proposed, not
/// confirmed): a few for a recruit colleges already want, about one for a
/// kid a little known, and less when the family can't pay. Rising seniors
/// go most.
fn camps_wanted(p: &Person, age: f64) -> f64 {
    let offered = !p.cal.offers.is_empty() || !p.cal.early_offers.is_empty();
    let want = 0.2 + 2.0 * p.visibility.clamp(0.0, 1.0) + if offered { 1.0 } else { 0.0 } + 0.3 * p.stars as f64;
    want * age * (0.3 + 0.7 * p.bg.money)
}

/// The colleges' June camps (S6d): every June day some kids go to a camp,
/// at a school that offered them or one in their state; a school far away
/// takes a family that can afford the trip. Your own camps are your staff's
/// (`scouting::dept`), so they are left out here.
fn camps(w: &mut World, d: Date) {
    let y = d.year();
    let key = w.options.seed as u64 ^ 0xCA3B_0000 ^ (d.0 as u64) << 20;
    let yours = match super::scouting::readers::seat_staff(w) {
        Some(super::scouting::readers::Staff::Program(p)) => Some(p),
        _ => None,
    };
    let mut in_state: Vec<Vec<usize>> = vec![Vec::new(); w.states.len()];
    for (j, g) in w.programs.iter().enumerate() {
        if matches!(g.division, super::data::Division::FBS | super::data::Division::FCS) && Some(j) != yours {
            in_state[g.state].push(j);
        }
    }
    let mut goes: Vec<(usize, usize)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        let Some(age) = camp_age(p, y) else { continue };
        let id = p.player.id as u64;
        if key_uniform(key, id) >= camps_wanted(p, age) / 30.0 {
            continue;
        }
        let offers: Vec<usize> = p
            .cal
            .offers
            .iter()
            .copied()
            .chain(p.cal.early_offers.iter().map(|o| o.0))
            .filter(|&j| Some(j) != yours)
            .collect();
        let host = if !offers.is_empty() && key_uniform(key ^ 1, id) < 0.6 {
            offers[(key_uniform(key ^ 2, id) * offers.len() as f64) as usize % offers.len()]
        } else {
            let list = &in_state[p.home_state];
            if list.is_empty() {
                continue;
            }
            list[(key_uniform(key ^ 3, id) * list.len() as f64) as usize % list.len()]
        };
        if super::geo::miles(w.programs[host].state, p.home_state) > 400.0 && p.bg.money < 0.5 {
            continue;
        }
        goes.push((i, host));
    }
    for (i, host) in goes {
        test(w, i, At::Camp(host as u16), d, key ^ 4, &[]);
    }
}

/// Your camp's laser times (minor bug 23: stored the day he runs them).
pub(crate) fn your_camp(w: &mut World, i: usize, prog: usize, d: Date) {
    let key = w.options.seed as u64 ^ 0xCA3C_0000 ^ (d.0 as u64) << 20;
    test(w, i, At::Camp(prog as u16), d, key, &[]);
}

/// The forties the high schools list (S6c): one a year for each player the
/// colleges ask about (rated, offered or getting looks), hand-timed and fast.
fn school_forties(w: &mut World, d: Date) {
    let y = d.year();
    let key = w.options.seed as u64 ^ 0x5C40_0000 ^ (y as u64) << 8;
    let list: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            camp_age(p, y).is_some_and(|a| a >= 0.8)
                && (p.stars >= 2 || p.visibility >= 0.35 || !p.cal.offers.is_empty() || !p.cal.early_offers.is_empty())
        })
        .collect();
    for i in list {
        run(w, i, Event::Forty, Timed::Hand, At::School, d, key);
    }
}

/// How many camps and showcases he has tested at (visibility's camps).
pub(crate) fn tested_at(w: &World, id: u32) -> usize {
    w.times.of(id).iter().filter(|t| t.event == Event::Forty && t.timed == Timed::Laser).count()
}

// --- What the times say ------------------------------------------------------------------

/// What a staff or a service reads of his speed and burst (S6f), in rating
/// points: today's, and grown on the typical boy's curve, as recruiters
/// project a kid from his age; whether anyone timed him electronically;
/// and how sure the read of his play speed is (its spread).
#[derive(Clone, Copy, Debug)]
pub struct Seen {
    pub now: (f64, f64),
    pub grown: (f64, f64),
    pub timed: bool,
    pub sd: f64,
}

/// How rough the film read of a kid's speed and burst is, rating points
/// (proposed, not confirmed).
const FILM_SD: f64 = 6.0;
/// What a hand time and a school's own forty count for against an
/// electronic one (S6b, S6c; proposed, not confirmed).
const HAND_WEIGHT: f64 = 0.4;
const SCHOOL_WEIGHT: f64 = 0.1;

/// The read: his film's look at his speed and burst, then every recorded
/// time in a weighted least squares fit of both, each time moved to his
/// age today on the typical curve. One shared read: every staff and service
/// sees the same tape and the same public times.
pub fn seen(w: &World, p: &Person) -> Seen {
    let id = p.player.id as u64;
    let a = &p.player.attributes;
    let key = w.options.seed as u64 ^ 0xF11A_0000 ^ (w.year as u64) << 24;
    let film = (
        a.get(Attr::Speed) + FILM_SD * key_normal(key, id),
        a.get(Attr::Acceleration) + FILM_SD * key_normal(key ^ 1, id),
    );
    let prior = 1.0 / (FILM_SD * FILM_SD);
    let (mut m11, mut m12, mut m22) = (prior, 0.0, prior);
    let (mut r1, mut r2) = (prior * film.0, prior * film.1);
    let today = w.today;
    let age = age_at(p, today);
    let share_now = typical_share(age);
    let mut timed = false;
    for t in w.times.of(p.player.id) {
        let (mix, per) = shape(t.event);
        let (off, sd, wt) = match (t.timed, t.at) {
            (Timed::Hand, At::School) => (HAND_FAST + SCHOOL_FAST.0 + 0.5 * SCHOOL_FAST.1, 0.15, SCHOOL_WEIGHT),
            (Timed::Hand, _) => (HAND_FAST, (spread(t.event).powi(2) + HAND_SD * HAND_SD).sqrt(), HAND_WEIGHT),
            _ => (0.0, spread(t.event), 1.0),
        };
        timed |= t.timed != Timed::Hand;
        // The time back to the mix of speed and burst it measures.
        let at_zero = base(t.event, 0.0, 0.0, t.lb as f64);
        let then = (at_zero - (t.secs as f64 + off)) / per;
        let u = then * share_now / typical_share(age_at(p, t.date)).max(0.5);
        let k = wt / (sd / per).powi(2);
        m11 += k * mix * mix;
        m12 += k * mix * (1.0 - mix);
        m22 += k * (1.0 - mix) * (1.0 - mix);
        r1 += k * mix * u;
        r2 += k * (1.0 - mix) * u;
    }
    let det = m11 * m22 - m12 * m12;
    let s = (m22 * r1 - m12 * r2) / det;
    let acc = (m11 * r2 - m12 * r1) / det;
    // The spread of his play speed's read (a mid mix of the two).
    let sd = (0.25 * (m22 + m11 - 2.0 * m12) / det).max(0.0).sqrt();
    let grow = 0.96 / share_now.max(0.5);
    let fit = |v: f64| clamp(v, 1.0, 99.0);
    Seen { now: (fit(s), fit(acc)), grown: (fit(s * grow), fit(acc * grow)), timed, sd }
}

/// His gifts as a staff projects them (D3, S6f): his frame and athletic
/// traits grown, with speed and burst from the times and the film, not
/// from his hidden ceilings.
pub fn seen_links(p: &Person, s: &Seen) -> crate::links::Links {
    let mut grown = p.player.ceilings;
    for a in Attr::ALL {
        grown.set(a, 0.96 * p.player.ceilings.get(a));
    }
    grown.set(Attr::Speed, s.grown.0);
    grown.set(Attr::Acceleration, s.grown.1);
    crate::links::measure(p.player.position, &p.player.body, &grown, p.player.game.recovery)
}

/// The stars a class's fastest timed kids are owed (S5): the fastest 1 in
/// 100 at his position on the recorded burst-and-speed mix (S2b) at least
/// 3 stars, 1 in 1,000 at least 4. `class` counts the class at each
/// position; the result is each kid's floor, by people index.
pub(crate) fn speed_floors(
    w: &World,
    kids: &[usize],
    class: &[f64; POSITION_COUNT],
) -> std::collections::HashMap<usize, u8> {
    let mut by_pos: Vec<Vec<(f64, usize)>> = vec![Vec::new(); POSITION_COUNT];
    for &i in kids {
        let p = &w.people[i];
        let s = seen(w, p);
        if !s.timed {
            continue;
        }
        if let Some(mix) = crate::body::play_speed(p.player.position, s.grown.0, s.grown.1) {
            by_pos[p.player.position.index()].push((mix, i));
        }
    }
    let mut out = std::collections::HashMap::new();
    for (k, list) in by_pos.iter_mut().enumerate() {
        list.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let n = class[k];
        let (four, three) = ((n / 1000.0).round() as usize, (n / 100.0).round() as usize);
        for (r, &(_, i)) in list.iter().enumerate().take(three) {
            out.insert(i, if r < four { 4 } else { 3 });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A time read back gives the mix of speed and burst it was run from
    /// (S6f), and the times land where real ones do (S6).
    #[test]
    fn times_read_back_their_ratings() {
        for event in [Event::Forty, Event::Split, Event::M100, Event::M200] {
            let (mix, per) = shape(event);
            let t = base(event, 88.0, 80.0, 190.0);
            let back = (base(event, 0.0, 0.0, 190.0) - t) / per;
            assert!((back - (mix * 88.0 + (1.0 - mix) * 80.0)).abs() < 1e-9, "{event:?} {back}");
        }
        // A fast receiver and a guard.
        let forty = base(Event::Forty, 90.0, 88.0, 185.0);
        let m100 = base(Event::M100, 90.0, 88.0, 185.0);
        let guard = base(Event::Forty, 55.0, 60.0, 300.0);
        assert!((4.25..4.45).contains(&forty), "{forty}");
        assert!((10.3..10.8).contains(&m100), "{m100}");
        assert!((5.0..5.35).contains(&guard), "{guard}");
        // Hand times run fast, and a school's own listing faster still.
        assert!(HAND_FAST > 0.2 && SCHOOL_FAST.0 > 0.0);
    }
}
