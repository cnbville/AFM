//! Capital's scenes (lane C4: CP2, CP13, CP25): a lunch or dinner with a
//! donor, a room of donors (the suite before a home game, the caravan's
//! booster dinner, the golf outing), and the call on a named donor's name
//! when scandal hits him. One folk scene kind, `scene::Kind::Capital`, with
//! Capital's own kind in `Scene.b`.
//!
//! - **A donor:** two exchanges over the meal. Ask him for the drive's
//!   building or for NIL money (`gifts::ask`: you are in the room, so the
//!   lead gift can come), push him past his lean when he'd rather give to
//!   the other side, hear him out, thank him for what he has given, or show
//!   him the plans for the room he wants. Being there warms him to you;
//!   each pick that gives him what he likes warms him more, by his ego.
//!   Your eye can catch what the office's file misses: a man stretched
//!   thin, a man who will want something back.
//! - **A room of donors:** work the room (a word with each), or sit with one
//!   (one exchange, the same picks).
//! - **His name** (CP25): take it down or keep it; left a week, the
//!   university decides (`names`).
//!
//! Handed on, the development office's man has the meal: no lead gift, no
//! warmth (he wanted the coach). Sizes are Claude's (proposed, not
//! confirmed).

use super::gifts::{self, Answer, Side};
use super::week::{self, meal};
use super::{Stage, donors};
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::scene::{self as fscene, Kind as SceneKind, PickKind, Scene, Who as SWho};
use crate::world::ground::week::{Slot, SlotKind};

/// Capital's scene kinds (`Scene.b`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Donor,
    Room,
    Name,
}

impl Kind {
    pub fn code(self) -> u64 {
        self as u64
    }
    pub fn from_code(c: u64) -> Option<Kind> {
        [Kind::Donor, Kind::Room, Kind::Name].get(c as usize).copied()
    }
}

/// A room of donors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RoomKind {
    Suite,
    Caravan,
    Golf,
}

/// Exchanges over a meal, and once you sit with one donor in a room.
pub const MEAL: u8 = 2;
pub const SEAT: u8 = 1;
/// Warmth toward you (each a share of the way to full warmth): being there,
/// hearing him out, thanking him and showing him his room's plans (more for
/// a man who wants his name on a building), times 0.5 + ego / 20 for the
/// last three; a word in a room. A meeting within three weeks of the last
/// counts half.
pub const MEET: f32 = 0.06;
pub const HEAR: f32 = 0.08;
pub const THANK: f32 = 0.10;
pub const PLANS: f32 = 0.06;
pub const PLANS_BUILDING: f32 = 0.10;
pub const WORD: f32 = 0.03;
pub const RECENT: i32 = 21;
/// How hard it is to see across the table that he's stretched thin, or
/// that he'll want something back (the eye's roll, `ground::scenes`).
pub const SEE_STRETCHED: f64 = 0.6;
pub const SEE_COLLECTS: f64 = 0.5;

// Picks.
const ASK_BUILDING: u32 = 1;
const ASK_NIL: u32 = 2;
const HEAR_HIM: u32 = 3;
const THANK_HIM: u32 = 4;
const SHOW_PLANS: u32 = 5;
const PUSH: u32 = 6;
const OTHER_SIDE: u32 = 7;
const WORK_ROOM: u32 = 10;
const SIT: u32 = 20;
const TAKE_DOWN: u32 = 1;
const KEEP_UP: u32 = 2;
const LEAVE: u32 = 900;

/// The scene's running state (a table the screens skip).
pub const STATE_TITLE: &str = "capital:state";

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct State {
    /// The donor across the table (after you sit with him, in a room).
    pub donor: Option<u32>,
    /// A room's donors.
    pub present: Vec<u32>,
    pub room: Option<RoomKind>,
    /// Exchanges used and allowed.
    pub used: u8,
    pub allowed: u8,
    /// What was done: "building", "nil", "heard", "thanked", "plans",
    /// "room".
    pub done: Vec<String>,
    /// A "rather the other side" answer waiting: the side he'd rather.
    pub rather: Option<Side>,
    /// An ask was made (it counted the meeting already).
    pub asked: bool,
    /// His warmth toward you when it began.
    pub warmth: f32,
    /// What it came to, a part a step.
    pub lines: Vec<String>,
}

pub fn state(s: &Scene) -> State {
    s.docs.iter().find(|d| d.title == STATE_TITLE).and_then(|d| serde_json::from_str(&d.note).ok()).unwrap_or_default()
}

fn put_state(s: &mut Scene, st: &State) {
    let note = serde_json::to_string(st).unwrap_or_default();
    match s.docs.iter_mut().find(|d| d.title == STATE_TITLE) {
        Some(d) => d.note = note,
        None => s.docs.push(fscene::Doc { title: STATE_TITLE.into(), note, cols: Vec::new(), rows: Vec::new() }),
    }
}

/// Capital's kind of a scene.
pub fn kind_of(s: &Scene) -> Option<Kind> {
    (s.kind == SceneKind::Capital).then(|| Kind::from_code(s.b)).flatten()
}

fn you(w: &World) -> Option<u32> {
    crate::world::scouting::readers::you(w).map(|c| c as u32)
}

/// A donor's place in the scene: his booster slot's link when the talks
/// know him, else his name.
fn who_of(w: &World, prog: usize, id: u32) -> SWho {
    let d = w.capital.donor(id);
    match d.and_then(|d| d.cast) {
        Some((slot, _)) => SWho::Cast(
            crate::world::folk::cast::CastRef {
                team: crate::world::folk::Team::College(prog),
                role: crate::world::folk::cast::Role::Booster,
                slot,
            }
            .token(),
        ),
        None => SWho::Group(d.map_or_else(String::new, |d| d.name.clone())),
    }
}

/// Moves his warmth toward you `x` of the way to full.
pub(crate) fn warm(w: &mut World, donor: u32, coach: u32, x: f32) {
    if let Some(d) = w.capital.donor_mut(donor) {
        let b = d.bond_mut(coach);
        b.warmth = (b.warmth + x * (1.0 - b.warmth)).clamp(0.0, 1.0);
    }
}

/// The ego's share of a pick he likes: 0.5 + ego / 20.
fn ego(w: &World, donor: u32) -> f32 {
    w.capital.donor(donor).map_or(1.0, |d| 0.5 + d.ego / 20.0)
}

/// Warmth in words.
pub fn warmth_words(x: f32) -> &'static str {
    match x {
        x if x >= 0.8 => "close to you",
        x if x >= 0.65 => "warm to you",
        x if x >= 0.45 => "polite",
        x if x >= 0.3 => "cool toward you",
        _ => "cold toward you",
    }
}

// --- Making them ------------------------------------------------------------------------------

/// Makes the scene for a slot of yours.
pub(crate) fn start(w: &mut World, prog: usize, slot: &Slot) -> Option<u32> {
    let mut s = Scene::new(SceneKind::Capital, slot.date, slot.title.clone());
    s.minute = Some(slot.minute);
    s.stops = true;
    let ok = match slot.kind {
        SlotKind::Donor => setup_donor(w, prog, &mut s, slot.donor?),
        SlotKind::DonorRoom => setup_room(w, prog, &mut s, week::room_kind(slot)),
        _ => false,
    };
    ok.then(|| fscene::add(w, s))
}

fn setup_donor(w: &mut World, prog: usize, s: &mut Scene, donor: u32) -> bool {
    let Some(d) = w.capital.donor(donor).filter(|d| d.alive()).cloned() else { return false };
    let me = you(w).unwrap_or(u32::MAX);
    s.b = Kind::Donor.code();
    s.a = donor as u64;
    s.who = vec![who_of(w, prog, donor)];
    let m = meal(s.minute.unwrap_or(week::DINNER));
    s.place = match d.money {
        donors::Money::Old => format!("{m} at his club"),
        donors::Money::New => format!("{m} near his office"),
    };
    let st = State { donor: Some(donor), allowed: MEAL, warmth: d.warmth_to(me), ..State::default() };
    put_state(s, &st);
    true
}

/// The donors in a room: the drive's lead donors first, then the richest
/// the office reads, the warmest among equals.
pub fn gathered(w: &World, prog: usize, n: usize) -> Vec<u32> {
    let me = you(w).unwrap_or(u32::MAX);
    let open = super::drive::open_drive(&w.capital, prog).map(|k| &w.capital.drives[k]);
    let mut v: Vec<(f64, u32)> = super::donors::of(w, prog)
        .into_iter()
        .map(|k| {
            let d = &w.capital.donors[k];
            let r = super::screen::read(w, prog, d, d.bond(me).map_or(0, |b| b.met));
            let lead = open.is_some_and(|x| x.lead.contains(&d.id));
            (if lead { 100.0 } else { 0.0 } + 10.0 * (r.lo + r.hi) as f64 / 2.0 + d.warmth_to(me) as f64, d.id)
        })
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().take(n).map(|x| x.1).collect()
}

fn setup_room(w: &mut World, prog: usize, s: &mut Scene, kind: RoomKind) -> bool {
    let n = match kind {
        RoomKind::Suite => week::SUITE,
        RoomKind::Caravan => week::CARAVAN,
        RoomKind::Golf => week::GOLF,
    };
    let present = gathered(w, prog, n);
    if present.is_empty() {
        return false;
    }
    s.b = Kind::Room.code();
    s.who = present.iter().take(4).map(|&id| who_of(w, prog, id)).collect();
    s.place = match kind {
        RoomKind::Suite => "The athletic director's suite, before kickoff".into(),
        RoomKind::Caravan => "The booster club's dinner on the spring caravan".into(),
        RoomKind::Golf => "The booster golf outing, then dinner at the clubhouse".into(),
    };
    let st = State { present, room: Some(kind), allowed: SEAT, ..State::default() };
    put_state(s, &st);
    true
}

/// The line for a slot the development office had.
pub(crate) fn office_line(w: &World, s: &Slot) -> String {
    match s.kind {
        SlotKind::Donor => {
            let name = s.donor.and_then(|d| w.capital.donor(d)).map_or_else(|| "a donor".into(), |d| d.name.clone());
            format!("{}: the development office's man had it with {name}, who had hoped for you.", meal(s.minute))
        }
        _ => format!("{}: the athletic director hosted.", s.title),
    }
}

// --- The scene's life (called from `folk::scene`) ---------------------------------------------

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    let Some(prog) = week::your_program(w) else {
        s.narrate("Donors are the head coach's to court.");
        s.end();
        return;
    };
    match kind_of(s) {
        Some(Kind::Donor) => {
            open_donor(w, prog, s);
        }
        Some(Kind::Room) => open_room(w, prog, s),
        Some(Kind::Name) => super::names::open(w, prog, s),
        None => s.end(),
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32) {
    let Some(prog) = week::your_program(w) else {
        s.end();
        return;
    };
    match kind_of(s) {
        Some(Kind::Donor) => answer_donor(w, prog, s, code),
        Some(Kind::Room) => answer_room(w, prog, s, code),
        Some(Kind::Name) => super::names::answer(w, prog, s, code),
        None => s.end(),
    }
    if s.done {
        finish(w, s);
    }
}

pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    match kind_of(s) {
        Some(Kind::Name) => super::names::hand(w, s),
        Some(Kind::Donor) => {
            let st = state(s);
            let name = st.donor.and_then(|d| w.capital.donor(d)).map_or_else(String::new, |d| d.name.clone());
            s.outcome = vec![if st.used > 0 {
                format!("{}: you left before dessert; he noticed.", s.title)
            } else {
                format!("{}: the development office's man had it with {name}, who had hoped for you.", s.title)
            }];
        }
        Some(Kind::Room) => {
            s.outcome = vec![format!("{}: the athletic director hosted.", s.title)];
        }
        None => {}
    }
    finish(w, s);
}

/// The label of the hand-off.
pub fn hand_label(s: &Scene) -> &'static str {
    match kind_of(s) {
        Some(Kind::Name) => "Leave it to the university",
        Some(Kind::Room) => "Let the athletic director host",
        _ => "Send the development office",
    }
}

fn finish(w: &mut World, s: &Scene) {
    let line = s.outcome.join(" ");
    let slot = w.ground.week.slots.iter().find(|x| crate::world::ground::scenes::made(w, x) == Some(s.id)).cloned();
    if let Some(slot) = slot {
        week::set_line(w, &slot, &line);
    }
}

// --- A donor across the table -------------------------------------------------------------------

fn open_donor(w: &mut World, prog: usize, s: &mut Scene) {
    let st = state(s);
    let Some(id) = st.donor else {
        s.end();
        return;
    };
    let Some(d) = w.capital.donor(id).cloned() else {
        s.end();
        return;
    };
    let me = you(w).unwrap_or(u32::MAX);
    let met = d.bond(me).map_or(0, |b| b.met);
    let read = super::screen::read(w, prog, &d, met);
    let catch = super::screen::catch_read(w, prog, &d, met);
    let last = d
        .bond(me)
        .and_then(|b| b.last)
        .map_or_else(|| "You've never met.".to_string(), |l| format!("You last saw him on {}.", l.pretty()));
    s.narrate(format!(
        "{} with {}, {}. {last} The office's file: {} {catch} He's {}.",
        meal(s.minute.unwrap_or(week::DINNER)),
        d.name,
        d.business,
        read.words,
        warmth_words(d.warmth_to(me))
    ));
    // What your eye catches across the table.
    let (eye, tired) = crate::world::ground::scenes::your_eye(w);
    let k = [0xCA4D_0001, id as u64, s.date.0 as u64];
    if d.stretched() && crate::world::ground::scenes::notice(w, eye, SEE_STRETCHED, tired, &k) {
        s.narrate("Something is off: he talks about his bankers twice, and the club's waiter has to ask about the card. The office's read may be generous.");
    }
    let k = [0xCA4D_0002, id as u64, s.date.0 as u64];
    if d.collects >= 0.45 && crate::world::ground::scenes::notice(w, eye, SEE_COLLECTS, tired, &k) {
        s.narrate(
            "He brings up, unprompted, what the last coach did for a friend's son. He'll want something back one day.",
        );
    }
    donor_picks(w, prog, s, &st);
}

/// The picks across the table for the donor in `st`.
fn donor_picks(w: &World, prog: usize, s: &mut Scene, st: &State) {
    s.picks.clear();
    let Some(id) = st.donor else { return };
    let Some(d) = w.capital.donor(id) else { return };
    let first = d.name.split_whitespace().next().unwrap_or(&d.name).to_string();
    if let Some(side) = st.rather {
        match side {
            Side::Nil => {
                s.pick(
                    PUSH,
                    "Push him: the building first",
                    "Push",
                    "firm",
                    "Less likely, and it costs you a little with him",
                );
                s.pick(OTHER_SIDE, "Take the NIL money instead", "Ask", "warm", "He'd rather pay your players");
            }
            Side::Building => {
                s.pick(
                    PUSH,
                    "Push him: the players need it now",
                    "Push",
                    "firm",
                    "Less likely, and it costs you a little with him",
                );
                s.pick(
                    OTHER_SIDE,
                    "Ask for the building instead",
                    "Ask",
                    "warm",
                    "He'd rather his money went into a building",
                );
            }
        }
        s.pick(LEAVE, format!("Thank {first} for his time"), "Close", "", "");
        return;
    }
    let open = super::drive::open_drive(&w.capital, prog).map(|k| &w.capital.drives[k]);
    let asking = open.filter(|x| matches!(x.stage, Stage::Quiet | Stage::Public));
    let done = |k: &str| st.done.iter().any(|x| x == k);
    if let Some(x) = asking
        && !done("building")
        && !x.asked.iter().any(|a| a.0 == id && (a.1 == x.stage as u8 || a.1 == super::drive::GAVE))
    {
        let lead = if x.lead.contains(&id) { "A lead gift needs you in the room" } else { "The drive asks everyone" };
        s.pick(ASK_BUILDING, format!("Ask him for the new {}", x.kind), "Ask", "firm", lead);
    }
    if !done("nil") && !w.capital.nil_asks.contains(&(id, w.year)) && crate::world::ncaa::nil_era(w.year) {
        s.pick(ASK_NIL, "Ask him for NIL money for the roster", "Ask", "firm", "Money a year into the collective");
    }
    if !done("heard") {
        s.pick(HEAR_HIM, "Hear him out: his business, his family", "Listen", "warm", "He likes being heard");
    }
    let gave = w.capital.pledges.iter().any(|p| p.donor == id && p.program == prog as u32 && p.paid > 0.0);
    if gave && !done("thanked") {
        s.pick(THANK_HIM, "Thank him for what he's given", "Thank", "warm", "He likes being thanked");
    }
    if let Some(x) = open
        && !done("plans")
        && let Some(r) = d.rooms.iter().copied().find(|&r| x.rooms & (1u16 << r) != 0)
    {
        let room = super::rooms::Room::from_index(r).map_or("room", |r| r.words());
        s.pick(SHOW_PLANS, format!("Show him the plans for the {room}"), "Show", "warm", "The room he'd give to");
    }
    s.pick(LEAVE, format!("Thank {first} for his time"), "Close", "", "");
}

fn answer_donor(w: &mut World, prog: usize, s: &mut Scene, code: u32) {
    let mut st = state(s);
    let Some(id) = st.donor else {
        s.end();
        return;
    };
    let Some(me) = you(w) else {
        s.end();
        return;
    };
    if code == LEAVE {
        end_donor(w, s, &mut st, me);
        put_state(s, &st);
        return;
    }
    let line = exchange(w, prog, s, &mut st, me, id, code);
    if !line.is_empty() {
        s.narrate(line.clone());
        st.lines.push(line);
    }
    if st.used >= st.allowed && st.rather.is_none() {
        end_donor(w, s, &mut st, me);
    } else {
        donor_picks(w, prog, s, &st);
    }
    put_state(s, &st);
}

/// One exchange with the donor across the table.
fn exchange(w: &mut World, prog: usize, s: &mut Scene, st: &mut State, me: u32, id: u32, code: u32) -> String {
    let name = w.capital.donor(id).map_or_else(String::new, |d| d.name.clone());
    let ask = |w: &mut World, st: &mut State, side: Side, push: bool| -> String {
        st.asked = true;
        match gifts::ask(w, prog, id, side, push) {
            Ok((Answer::Rather(other), words)) => {
                st.rather = Some(other);
                words
            }
            Ok((_, words)) => {
                st.rather = None;
                st.used += 1;
                words
            }
            Err(e) => {
                st.rather = None;
                e
            }
        }
    };
    match code {
        ASK_BUILDING => {
            st.done.push("building".into());
            ask(w, st, Side::Building, false)
        }
        ASK_NIL => {
            st.done.push("nil".into());
            ask(w, st, Side::Nil, false)
        }
        PUSH => {
            // He'd rather the other side: push for the one you asked.
            let side = match st.rather {
                Some(Side::Nil) => Side::Building,
                _ => Side::Nil,
            };
            ask(w, st, side, true)
        }
        OTHER_SIDE => {
            let side = st.rather.unwrap_or(Side::Nil);
            st.done.push(if side == Side::Nil { "nil" } else { "building" }.into());
            ask(w, st, side, false)
        }
        HEAR_HIM => {
            st.done.push("heard".into());
            st.used += 1;
            let x = HEAR * ego(w, id);
            warm(w, id, me, x);
            let wants: Vec<&str> =
                w.capital.donor(id).map_or_else(Vec::new, |d| d.wants.iter().map(|x| x.words()).collect());
            let rooms: Vec<&str> = w.capital.donor(id).map_or_else(Vec::new, |d| {
                d.rooms.iter().filter_map(|&r| super::rooms::Room::from_index(r)).map(|r| r.words()).collect()
            });
            let mut line = format!("{name} talks for an hour, and you let him.");
            if !wants.is_empty() {
                line.push_str(&format!(" What he wants from the program: {}.", wants.join("; ")));
            }
            if !rooms.is_empty() {
                line.push_str(&format!(" If he gave to a room it would be the {}.", rooms.join(" or the ")));
            }
            line
        }
        THANK_HIM => {
            st.done.push("thanked".into());
            st.used += 1;
            warm(w, id, me, THANK * ego(w, id));
            format!("You thank {name} for what he's given. He waves it off and keeps smiling.")
        }
        SHOW_PLANS => {
            st.done.push("plans".into());
            st.used += 1;
            let building =
                w.capital.donor(id).is_some_and(|d| d.wants.contains(&crate::world::folk::cast::Want::Building));
            warm(w, id, me, if building { PLANS_BUILDING } else { PLANS } * ego(w, id));
            if building {
                format!("{name} leans over the renderings and asks where the names go.")
            } else {
                format!("{name} studies the renderings and asks good questions.")
            }
        }
        _ => {
            let _ = s;
            String::new()
        }
    }
}

/// The meal ends: being there warms him (half for a meeting within three
/// weeks of the last), and it counts as a meeting.
fn end_donor(w: &mut World, s: &mut Scene, st: &mut State, me: u32) {
    let Some(id) = st.donor else {
        s.end();
        return;
    };
    let today = w.today;
    let recent = w
        .capital
        .donor(id)
        .and_then(|d| d.bond(me))
        .and_then(|b| b.last)
        .is_some_and(|l| l < today && l.days_to(today) < RECENT);
    if !st.asked {
        donors::met(w, id, me, today);
    }
    warm(w, id, me, if recent { 0.5 * MEET } else { MEET });
    let d = w.capital.donor(id).cloned();
    let name = d.as_ref().map_or_else(String::new, |d| d.name.clone());
    let now = d.map_or(st.warmth, |d| d.warmth_to(me));
    let mut line = st.lines.join(" ");
    if !line.is_empty() {
        line.push(' ');
    }
    let how = if now > st.warmth + 0.05 {
        format!("{name} leaves {} than he came.", if now >= 0.65 { "warmer" } else { "a little warmer" })
    } else {
        format!("{name} is {}.", warmth_words(now))
    };
    s.outcome = vec![format!("{}: {line}{how}", s.title)];
    s.end();
}

// --- A room of donors -------------------------------------------------------------------------

fn open_room(w: &mut World, prog: usize, s: &mut Scene) {
    let st = state(s);
    let me = you(w).unwrap_or(u32::MAX);
    let n = st.present.len();
    let what = match st.room {
        Some(RoomKind::Suite) => "The suite fills two hours before kickoff",
        Some(RoomKind::Caravan) => "The caravan's dinner: the booster club, a buffet and a microphone",
        _ => "Eighteen holes, then dinner at the clubhouse",
    };
    s.narrate(format!("{what}. {n} of your donors are here."));
    let rows: Vec<(Option<u32>, Vec<String>)> = st
        .present
        .iter()
        .filter_map(|&id| {
            let d = w.capital.donor(id)?;
            let met = d.bond(me).map_or(0, |b| b.met);
            Some((
                None,
                vec![
                    d.name.clone(),
                    d.business.clone(),
                    super::screen::read(w, prog, d, met).words,
                    warmth_words(d.warmth_to(me)).to_string(),
                ],
            ))
        })
        .collect();
    s.docs.retain(|d| d.title != "In the room");
    s.docs.push(fscene::Doc {
        title: "In the room".into(),
        note: String::new(),
        cols: ["Name", "Business", "The office's read", "With you"].iter().map(|c| c.to_string()).collect(),
        rows: rows.into_iter().map(|(pid, cells)| fscene::DocRow { pid, cells }).collect(),
    });
    room_picks(w, s, &st);
}

fn room_picks(w: &World, s: &mut Scene, st: &State) {
    s.picks.clear();
    s.pick(WORK_ROOM, "Work the room: a word with each", "Work the room", "warm", "A little with everyone");
    for (k, &id) in st.present.iter().take(4).enumerate() {
        let Some(d) = w.capital.donor(id) else { continue };
        s.pick(SIT + k as u32, format!("Sit with {}", d.name), "Sit down", "warm", "A real conversation with one");
    }
    s.pick(LEAVE, "Let the athletic director host", "Close", "", "");
}

fn answer_room(w: &mut World, prog: usize, s: &mut Scene, code: u32) {
    let mut st = state(s);
    if st.donor.is_some() {
        // Seated with one: the donor's picks.
        answer_donor(w, prog, s, code);
        return;
    }
    let Some(me) = you(w) else {
        s.end();
        return;
    };
    match code {
        WORK_ROOM => {
            for &id in &st.present {
                warm(w, id, me, WORD);
            }
            let n = st.present.len();
            s.outcome = vec![format!("{}: you worked the room, a handshake and a word with all {n}.", s.title)];
            s.end();
        }
        c if (SIT..SIT + 4).contains(&c) => {
            let Some(&id) = st.present.get((c - SIT) as usize) else { return };
            st.donor = Some(id);
            st.allowed = SEAT;
            st.warmth = w.capital.donor(id).map_or(0.5, |d| d.warmth_to(me));
            let name = w.capital.donor(id).map_or_else(String::new, |d| d.name.clone());
            s.narrate(format!("You take the seat next to {name}."));
            s.who = vec![who_of(w, prog, id)];
            donor_picks(w, prog, s, &st);
        }
        _ => {
            s.outcome = vec![format!("{}: you shook a few hands and left it to the athletic director.", s.title)];
            s.end();
        }
    }
    put_state(s, &st);
}

/// Today's date's scenes waiting (tests).
pub fn waiting_on(w: &World, d: Date) -> Vec<u32> {
    w.folk.scenes.iter().filter(|s| s.kind == SceneKind::Capital && s.date == d && !s.done).map(|s| s.id).collect()
}

/// A final pick (no typed line lands on it unasked).
pub(crate) fn final_pick(s: &mut Scene, code: u32, label: &str, hint: &str) {
    s.pick(code, label, "Decide", "firm", hint);
    if let Some(p) = s.picks.last_mut() {
        p.kind = PickKind::Final;
    }
}

pub(crate) const NAME_DOWN: u32 = TAKE_DOWN;
pub(crate) const NAME_KEEP: u32 = KEEP_UP;
pub(crate) const NAME_LEAVE: u32 = LEAVE;
