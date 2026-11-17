//! Lane C5, the facilities race and the press (CP29-CP31).
//!
//! - CP29: a rival's new room can be a reason on a lost recruit's why-line.
//!   The engine says what the rooms gave in his decision (`rooms_part`) and
//!   which room to name (`room_to_name`); the game writes the line
//!   (`crates/afm-game/src/lane_help.rs`, `lost_why`).
//! - CP30: a rival's feasibility study and quiet phase are never in the
//!   news; you hear of them only from your own donors, as rumors
//!   (`rumors`, each month on the first).
//! - CP31: each July the press ranks every conference's facilities
//!   (`pecking`, on the money year's day): kept for the Pecking order
//!   screen, in the news for your conference and the Power 4.
//!
//! News and text only: nothing here moves a result. Rumors come only to a
//! college seat; the pecking order is a store and news lines. Every draw is
//! keyed (`super::draw`). Sizes are Claude's (proposed, not confirmed).

use super::drive::{self, Stage};
use super::rooms::Room;
use crate::calendar::events::{Event, Kind};
use crate::calendar::{Date, Level, Loudness};
use crate::positions::Position;
use crate::rng::clamp;
use crate::world::World;
use crate::world::data::Division;
use crate::world::program::GradeCat;

// ---------------------------------------------------------------------------
// The stores
// ---------------------------------------------------------------------------

/// A rumor one of your donors passed on about a rival's drive (CP30).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Rumor {
    pub date: Date,
    /// The rival's program.
    pub about: u32,
    /// The drive it is about (its id) and the stage it was in.
    pub drive: u32,
    pub stage: Stage,
    /// The donor who told you.
    pub from: u32,
    /// He knows it first hand (a donor of both schools whom the rival
    /// sounded out or asked), not through the grapevine.
    pub direct: bool,
    /// The building as he named it ("a new football building", or only "a
    /// new building" when he didn't know).
    pub building: String,
    /// What he said of their progress, as a share of their goal (None in
    /// the study: nothing is raised yet). The truth was `truth`.
    pub heard: Option<f32>,
    pub truth: Option<f32>,
    /// A lead gift was in when he told you, and he knew it.
    pub lead_in: bool,
    /// The line as he said it.
    pub text: String,
}

/// One conference's facilities pecking order in one July (CP31).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Pecking {
    pub year: i32,
    pub division: Division,
    pub conference: String,
    /// Best first.
    pub order: Vec<Placed>,
}

/// A program's place in its conference's pecking order.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Placed {
    pub program: u32,
    /// The press's score, 0-100.
    pub score: f32,
    /// The rooms built or renovated in the last two seasons, a bit per
    /// `Room` (the press loves the new).
    pub new_rooms: u16,
    /// Its newest building, when it opened in the last two seasons.
    pub new_hall: Option<String>,
}

impl Default for Pecking {
    fn default() -> Pecking {
        Pecking { year: 0, division: Division::FBS, conference: String::new(), order: Vec::new() }
    }
}

impl Pecking {
    /// A program's place, 1 first.
    pub fn rank(&self, p: usize) -> Option<usize> {
        self.order.iter().position(|x| x.program as usize == p).map(|k| k + 1)
    }
}

// ---------------------------------------------------------------------------
// CP29: the rooms on a lost recruit's why-line
// ---------------------------------------------------------------------------

/// A room counts as new for three seasons after it opens or is renovated.
pub const NEW_SEASONS: i32 = 3;

/// What the rooms gave program `j` in recruit `i`'s decision, in appeal
/// points, as the decision adds them up today (`recruiting::appeal`): the
/// facilities grade's own term (part of "the name") and what the locker
/// room and the recruiting lounge added to his visits there (part of the
/// rest). (grade part, visits part).
pub fn rooms_part(w: &World, i: usize, j: usize) -> (f64, f64) {
    let p = &w.people[i];
    let prog = &w.programs[j];
    let grade = crate::world::recruiting::FACILITIES_GRADE * prog.grades[GradeCat::Facilities as usize] / 100.0;
    let contact = p.cal.visits.iter().filter(|v| v.0 == j).count().min(5) as f64;
    let unofficial =
        p.cal.visits.iter().filter(|v| v.0 == j && v.2 == crate::world::Visit::Unofficial).count().min(5) as f64;
    let official = p.cal.visits.iter().any(|v| v.0 == j && v.2 == crate::world::Visit::Official);
    let visits = crate::world::recruiting::GAME_DAY_PULL * unofficial.min(contact) * super::rooms::game_day_lift(prog)
        + if official { crate::world::recruiting::OFFICIAL_PULL * super::rooms::visit_lift(prog) } else { 0.0 };
    (grade, visits)
}

/// How much a recruit cares about a room, from his position and his
/// family's wants (proposed, not confirmed): only to choose which room the
/// why-line names, never how much the rooms counted.
pub fn cares(w: &World, i: usize, r: Room) -> f64 {
    let p = &w.people[i];
    let x = &p.family.decider_wants;
    let big = matches!(p.player.position, Position::OT | Position::IOL | Position::DT | Position::EDGE);
    match r {
        Room::Weight => 0.6 + x.development + if big { 0.6 } else { 0.2 },
        Room::Nutrition => 0.2 + 0.4 * x.development + if big { 0.4 } else { 0.0 },
        Room::SportsScience | Room::Recovery => 0.2 + 0.6 * x.development,
        Room::Film => 0.3 + 0.6 * x.development + if p.player.position == Position::QB { 0.4 } else { 0.0 },
        Room::IndoorField => 0.3 + 0.4 * x.development,
        Room::Locker | Room::Stadium => 0.5 + x.prestige,
        Room::TrophyHall => 0.3 + 0.5 * x.prestige + 0.5 * x.development,
        Room::RecruitingLounge => 0.2 + 0.4 * x.prestige,
        Room::PlayersLounge => 0.2 + 3.0 * super::rooms::campus_life_want(p),
        Room::Academic => 0.2 + 1.2 * x.academics,
        Room::ContentStudio => 0.1 + 0.8 * x.money,
    }
}

/// The room the why-line names when `chosen`'s rooms beat `yours` in his
/// decision: a room it opened or renovated in the last three seasons that
/// reads at least half a level over yours, by how much he cares, its share
/// and the gap (true: a new room); else its room furthest over yours that
/// he cares about (false). None when no room of theirs reads over yours.
pub fn room_to_name(w: &World, i: usize, chosen: usize, yours: usize) -> Option<(Room, bool)> {
    let (a, b) = (&w.programs[chosen].campus, &w.programs[yours].campus);
    let year = w.year;
    let gap =
        |r: Room| a.read[r as usize] as f64 - if b.level[r as usize] > 0.0 { b.read[r as usize] as f64 } else { 0.0 };
    let score = |r: Room| cares(w, i, r) * r.weight() * gap(r);
    let fresh =
        |r: Room| a.level[r as usize] > 0.0 && a.since[r as usize] > 0 && a.since[r as usize] + NEW_SEASONS > year;
    let best = |only_new: bool| {
        Room::ALL
            .into_iter()
            .filter(|&r| gap(r) >= 0.5 && (!only_new || fresh(r)))
            .max_by(|&x, &y| score(x).total_cmp(&score(y)))
    };
    best(true).map(|r| (r, true)).or_else(|| best(false).map(|r| (r, false)))
}

// ---------------------------------------------------------------------------
// CP30: rumors of rivals' quiet phases
// ---------------------------------------------------------------------------

/// A donor who knows first hand tells you, each month, with this chance at
/// full warmth (proposed, not confirmed).
pub const TELL_DIRECT: f64 = 0.3;
/// The grapevine's monthly chance for each ask a rival's drive has made,
/// before your ear and the distance (proposed, not confirmed).
pub const GRAPEVINE: f64 = 0.012;
/// The grapevine at most a month for one drive.
pub const GRAPEVINE_MAX: f64 = 0.25;
/// Out of state, the grapevine carries this share.
pub const OUT_OF_STATE: f64 = 0.35;
/// Warm donors (warmth toward you 0.6 or more) give you your ear: six of
/// them hear all the grapevine carries.
pub const EAR_DONORS: f64 = 6.0;
/// A first-hand read of a rival's progress errs by this much (sd, share of
/// the goal); the grapevine's by `ERROR_GRAPEVINE`.
pub const ERROR_DIRECT: f64 = 0.08;
pub const ERROR_GRAPEVINE: f64 = 0.2;
/// The grapevine names the building this often; else "a new building".
pub const NAMES_BUILDING: f64 = 0.7;
/// Rumors are kept this many years.
pub const KEEP_YEARS: i32 = 4;

/// Your college program, if you coach one.
fn seat_college(w: &World) -> Option<usize> {
    w.seat.filter(|s| s.level == Level::College).and_then(|s| s.program)
}

/// A rival in the race: a program in your conference, or one in your
/// state at your level.
pub fn rival(w: &World, me: usize, p: usize) -> bool {
    if p == me {
        return false;
    }
    let (a, b) = (&w.programs[me], &w.programs[p]);
    a.division == b.division && (a.conference == b.conference || a.state == b.state)
}

/// The first of each month, at a college seat: each rival drive in its
/// study or quiet phase may reach you through one of your donors, once a
/// stage. A donor of both schools whom the rival sounded out (the study
/// interviews the big donors) or asked knows first hand and tells you by
/// his warmth toward you; otherwise the grapevine carries it by how many
/// donors the rival has asked, in state or not, and how many of your
/// donors are warm to you.
pub(crate) fn rumors(w: &mut World, d: Date) {
    let Some(me) = seat_college(w) else { return };
    let Some(coach) = w.career.as_ref().map(|c| c.coach as u32) else { return };
    let mine: Vec<usize> = super::donors::of(w, me);
    if mine.is_empty() {
        return;
    }
    let ear = clamp(
        mine.iter().filter(|&&k| w.capital.donors[k].warmth_to(coach) >= 0.6).count() as f64 / EAR_DONORS,
        0.15,
        1.0,
    );
    let mut told = Vec::new();
    for x in w.capital.drives.iter() {
        let p = x.program as usize;
        if !matches!(x.stage, Stage::Study | Stage::Quiet) || p >= w.programs.len() || !rival(w, me, p) {
            continue;
        }
        if w.capital.rumors.iter().any(|r| r.drive == x.id && r.stage == x.stage) {
            continue;
        }
        let key = (x.id as u64) << 24 ^ (x.stage as u64) << 20 ^ (d.year() as u64) << 4 ^ d.month() as u64;
        // First hand: your donors who are the rival's too and were sounded
        // out or asked.
        let knows = |k: usize| {
            let dn = &w.capital.donors[k];
            let theirs = dn.home as usize == p || dn.also == Some(p as u32);
            theirs
                && match x.stage {
                    Stage::Study => {
                        x.lead.contains(&dn.id) || super::draw(w, 60, (dn.id as u64) << 20 ^ x.id as u64) < 0.5
                    }
                    _ => x.lead.contains(&dn.id) || x.asked.iter().any(|a| a.0 == dn.id),
                }
        };
        let mut teller = None;
        for &k in mine.iter().filter(|&&k| knows(k)) {
            let dn = &w.capital.donors[k];
            let warm = clamp((dn.warmth_to(coach) as f64 - 0.3) / 0.5, 0.1, 1.0);
            let chatty = 0.6 + 0.4 * dn.meddling as f64 / 20.0;
            if super::draw(w, 61, key ^ (dn.id as u64) << 40) < TELL_DIRECT * warm * chatty {
                teller = Some((k, true));
                break;
            }
        }
        if teller.is_none() {
            let asked = match x.stage {
                Stage::Study => x.lead.len().max(3) as f64,
                _ => (x.asked.len() + x.lead.len()) as f64,
            };
            let near = if w.programs[me].state == w.programs[p].state { 1.0 } else { OUT_OF_STATE };
            let chance = (GRAPEVINE * asked * near * ear).min(GRAPEVINE_MAX);
            if super::draw(w, 62, key) < chance {
                // The warmest of your donors passes it on (a keyed pick
                // weighted by warmth).
                let total: f64 = mine.iter().map(|&k| w.capital.donors[k].warmth_to(coach) as f64).sum();
                let mut u = super::draw(w, 63, key) * total;
                for &k in &mine {
                    u -= w.capital.donors[k].warmth_to(coach) as f64;
                    if u <= 0.0 {
                        teller = Some((k, false));
                        break;
                    }
                }
                if teller.is_none() {
                    teller = mine.last().map(|&k| (k, false));
                }
            }
        }
        if let Some((k, direct)) = teller {
            told.push((x.id, k, direct, key));
        }
    }
    for (id, k, direct, key) in told {
        let Some(x) = w.capital.drives.iter().find(|x| x.id == id) else { continue };
        let p = x.program as usize;
        let truth = (x.stage != Stage::Study && x.goal > 0.0).then(|| (drive::raised(&w.capital, x) / x.goal) as f32);
        let err = if direct { ERROR_DIRECT } else { ERROR_GRAPEVINE };
        let heard = truth.map(|t| {
            clamp(
                t as f64 + err * crate::world::season::key_normal(w.options.seed as u64 ^ 0xC5_0000, key ^ 64),
                0.0,
                1.2,
            ) as f32
        });
        let named = direct || super::draw(w, 65, key) < NAMES_BUILDING;
        let building = if named { format!("a new {}", x.kind) } else { "a new building".to_string() };
        let lead_in =
            direct && x.lead.iter().any(|l| w.capital.pledges.iter().any(|q| q.drive == x.id && q.donor == *l));
        let dn = &w.capital.donors[k];
        let them = w.program_name(p).to_string();
        let opener = match (d.month(), direct) {
            (9..=12, _) => "In the suite before kickoff",
            (_, true) => "Over dinner",
            _ => "On the golf course",
        };
        let how = if direct { "He was in the room for it" } else { "He heard it from friends" };
        let what = match x.stage {
            Stage::Study => format!("{them} has consultants sounding out donors for {building}"),
            _ => format!("{them} is quietly raising money for {building}"),
        };
        let progress = heard.map(|h| match h {
            h if h < 0.15 => "They are just getting started.".to_string(),
            h if h < 0.35 => "They have a few big gifts in.".to_string(),
            h if h < 0.6 => "They are near halfway.".to_string(),
            h if h < 0.85 => "They are well past halfway.".to_string(),
            _ => "They are close to going public.".to_string(),
        });
        let lead = if lead_in { " A lead gift is in." } else { "" };
        let text = format!(
            "{opener}, {} tells you {what}. {how}.{}{lead}",
            dn.name,
            progress.map(|s| format!(" {s}")).unwrap_or_default()
        );
        let rumor = Rumor {
            date: d,
            about: p as u32,
            drive: id,
            stage: x.stage,
            from: dn.id,
            direct,
            building,
            heard,
            truth,
            lead_in,
            text: text.clone(),
        };
        w.capital.rumors.push(rumor);
        let e = Event::new(d, Level::College, Kind::Whisper, Loudness::Small, text)
            .rule("money")
            .program(me)
            .school_link(p, &them);
        w.calendar.post(e);
    }
    let year = w.year;
    w.capital.rumors.retain(|r| r.date.year() + KEEP_YEARS > year);
}

/// What you know of a rival's drive (CP30): the public phase and after
/// from the news, the study and the quiet phase only through a rumor you
/// heard. For the Pecking order screen (lane CS5).
#[derive(Clone, Debug)]
pub enum Seen<'a> {
    Public(&'a drive::Drive),
    Rumored(&'a Rumor),
}

/// The rivals' drives you know of, at your college seat (newest first).
pub fn rivals_drives(w: &World) -> Vec<Seen<'_>> {
    let Some(me) = seat_college(w) else { return Vec::new() };
    let mut v = Vec::new();
    for x in w.capital.drives.iter().rev() {
        let p = x.program as usize;
        if p >= w.programs.len() || !rival(w, me, p) || x.shelved && x.public_on.is_none() {
            continue;
        }
        if x.public_on.is_some() {
            v.push(Seen::Public(x));
        } else if let Some(r) = w.capital.rumors.iter().rev().find(|r| r.drive == x.id) {
            v.push(Seen::Rumored(r));
        }
    }
    v
}

// ---------------------------------------------------------------------------
// CP31: the press's pecking order each July
// ---------------------------------------------------------------------------

/// What the press weighs in a room (proposed, not confirmed): the rooms a
/// writer tours and photographs (the stadium, the locker room, the weight
/// room, the indoor field) count most; the labs and the offices least.
/// They sum to 1.
pub fn press_weight(r: Room) -> f64 {
    match r {
        Room::Stadium | Room::Locker => 0.14,
        Room::Weight | Room::IndoorField => 0.12,
        Room::PlayersLounge | Room::RecruitingLounge | Room::Film => 0.07,
        Room::TrophyHall | Room::Recovery => 0.06,
        Room::SportsScience | Room::Nutrition | Room::Academic => 0.04,
        Room::ContentStudio => 0.03,
    }
}

/// A room built or renovated in the last two seasons reads this many
/// levels higher to the press (proposed, not confirmed).
pub const SHINE: f64 = 0.5;
/// Pecking orders are kept this many years (your rank over the years).
pub const KEEP_PECKING: i32 = 30;

/// The press's score for one program, 0-100, and its new rooms.
pub fn press_score(w: &World, p: usize) -> (f64, u16) {
    let c = &w.programs[p].campus;
    let mut s = 0.0;
    let mut new = 0u16;
    for r in Room::ALL {
        let k = r as usize;
        let l = c.level[k] as f64;
        if l <= 0.0 {
            continue;
        }
        let fresh = c.since[k] > 0 && c.since[k] + 2 > w.year;
        if fresh {
            new |= r.bit();
        }
        s += press_weight(r) * (l + if fresh { SHINE } else { 0.0 }).min(5.0);
    }
    (s * 20.0, new)
}

/// On the money year's day (1 July), after the rooms are read: the press
/// ranks every conference's facilities (four-year levels, conferences of
/// two or more). Kept every year; once the rooms live, a news line for
/// your conference and each Power 4 conference.
pub(crate) fn pecking(w: &mut World) {
    let year = w.year;
    if w.capital.pecking.iter().any(|x| x.year == year) {
        return;
    }
    let mut by: std::collections::BTreeMap<(Division, String), Vec<Placed>> = Default::default();
    for (p, prog) in w.programs.iter().enumerate() {
        if !prog.plays(year) || prog.division == Division::JUCO || !prog.campus.set || prog.conference.is_empty() {
            continue;
        }
        let (score, new_rooms) = press_score(w, p);
        let new_hall =
            prog.campus.halls.iter().filter(|h| h.opened + 2 > year).max_by_key(|h| h.opened).map(|h| h.name.clone());
        by.entry((prog.division, prog.conference.clone())).or_default().push(Placed {
            program: p as u32,
            score: score as f32,
            new_rooms,
            new_hall,
        });
    }
    let mut fresh = Vec::new();
    for ((division, conference), mut order) in by {
        if order.len() < 2 {
            continue;
        }
        order.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.program.cmp(&b.program)));
        fresh.push(Pecking { year, division, conference, order });
    }
    if super::live(w) {
        for k in 0..fresh.len() {
            news(w, &fresh[k]);
        }
    }
    w.capital.pecking.extend(fresh);
    w.capital.pecking.retain(|x| x.year + KEEP_PECKING > year);
}

/// The last pecking order of a program's conference (this July's once it
/// is out), and the one before it.
pub fn standing(w: &World, p: usize) -> (Option<&Pecking>, Option<&Pecking>) {
    let mut it = w.capital.pecking.iter().rev().filter(|x| x.order.iter().any(|y| y.program as usize == p));
    (it.next(), it.next())
}

/// A program's rank in its conference's pecking order, year by year
/// (oldest first): (year, rank, of how many).
pub fn ranks(w: &World, p: usize) -> Vec<(i32, usize, usize)> {
    w.capital.pecking.iter().filter_map(|x| x.rank(p).map(|r| (x.year, r, x.order.len()))).collect()
}

/// Why the press puts a program where it is, in words: its new building,
/// else its renovated room, else its best room.
pub fn because(w: &World, x: &Placed) -> String {
    if let Some(h) = &x.new_hall {
        return format!("its new {h}");
    }
    if let Some(r) = Room::ALL
        .into_iter()
        .filter(|r| x.new_rooms & r.bit() != 0)
        .max_by(|a, b| press_weight(*a).total_cmp(&press_weight(*b)))
    {
        return format!("its renovated {}", r.words());
    }
    let c = &w.programs[x.program as usize].campus;
    let best = Room::ALL
        .into_iter()
        .max_by(|a, b| (press_weight(*a) * c.level(*a)).total_cmp(&(press_weight(*b) * c.level(*b))))
        .unwrap_or(Room::Stadium);
    format!("its {}", best.words())
}

fn ordinal(n: usize) -> String {
    let s = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

/// The pecking order's news line: your conference's in full at your seat
/// (the top three, then you with your move), a Power 4 conference's top
/// one in a small line.
fn news(w: &mut World, x: &Pecking) {
    let me = seat_college(w).filter(|&m| x.order.iter().any(|y| y.program as usize == m));
    let p4 = x.division == Division::FBS
        && x.order.first().is_some_and(|y| w.programs[y.program as usize].power_conference(w.year));
    if me.is_none() && !p4 {
        return;
    }
    let names: Vec<String> = x.order.iter().map(|y| w.program_name(y.program as usize).to_string()).collect();
    let conf = &x.conference;
    let first = format!("{} first, on {}", names[0], because(w, &x.order[0]));
    let (text, loud) = match me {
        Some(m) => {
            let rank = x.rank(m).unwrap_or(0);
            let before = w
                .capital
                .pecking
                .iter()
                .rev()
                .find(|y| y.year < x.year && y.conference == x.conference && y.division == x.division)
                .and_then(|y| y.rank(m));
            let moved = match before {
                Some(b) if b > rank => format!(", up {}", b - rank),
                Some(b) if b < rank => format!(", down {}", rank - b),
                Some(_) => ", the same as last year".to_string(),
                None => String::new(),
            };
            let rest: Vec<String> =
                names.iter().enumerate().skip(1).take(2).map(|(k, n)| format!("{} {n}", k + 1)).collect();
            let you = format!(" {} is {} of {}{moved}.", names[rank.max(1) - 1], ordinal(rank), names.len());
            (
                format!("The press ranks the {conf}'s facilities: {first}; then {}.{you}", rest.join(", ")),
                Loudness::Medium,
            )
        }
        None => (format!("The press ranks the {conf}'s facilities: {first}."), Loudness::Small),
    };
    let mut e = crate::world::finance::story(w, Level::College, loud, text);
    if let Some(m) = me {
        e = e.program(m);
    } else {
        e = e.program(x.order[0].program as usize);
    }
    for (k, n) in names.iter().enumerate().take(3) {
        e = e.school_link(x.order[k].program as usize, n);
    }
    if let Some(m) = me
        && x.rank(m).is_some_and(|r| r > 3)
    {
        e = e.school_link(m, w.program_name(m));
    }
    crate::world::finance::post(w, e);
}
