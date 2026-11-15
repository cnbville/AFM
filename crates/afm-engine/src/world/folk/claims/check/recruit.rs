//! Facts to a recruit or a transfer in a typed line (live talks lane 10,
//! LT2 and LT3; the talk facts design, sections 6 and 14; the promise
//! design, sections 9 and 11). Each is looked up where he would look: the
//! public record (your titles, your last season, your draft picks), the
//! public roster at his spot (who comes back, who leaves) and, on its day,
//! the money (your offer, at signing or on his decision day).
//!
//! - **True**: it counts what the option it landed on counts, and nothing
//!   more.
//! - **False, and he can check it as you say it** (your picks, your titles,
//!   your record, the room at his spot): never saved. He knows, so the
//!   point you made adds nothing to your case, the oversell costs its 0.006
//!   (`case::OVERSELL`) and your name on the trail drops a little
//!   (`CAUGHT_TRAIL`): `landed`.
//! - **False, and he can only check it on a day** (a NIL number at
//!   signing): a lie, saved with that day. Found out, it costs what a
//!   broken promise costs, scaled by his belief, once he is yours: his
//!   trust and his mood, which raise his portal lean (`due`, then
//!   `truth::judge`). If he goes elsewhere he never sees it.
//! - **Where he will stand** ("you'll start day one"): a promise about
//!   what he'll see, never a fact. It marks "later" and lands on the
//!   promise it means (`pitch::typed_promise`).
//! - Anything the game can't place reads `Unknown` and counts for nothing.
//!
//! A typed fact only ever takes away: the conversation's math (what a
//! point adds, the case's cap, the AI staffs' one step) is never touched.
//! A claim in your favour that understates the truth ("we've had a few
//! guys drafted" with twelve) is true.

use crate::calendar::Date;
use crate::world::folk::case::{self, Case};
use crate::world::folk::claims::truth::{STANDING, Stated, Truth};
use crate::world::folk::claims::{self, Ear, Kind};
use crate::world::folk::scene::{Kind as Scenes, Scene};
use crate::world::folk::{Name, facts, person};
use crate::world::front_office::portal;
use crate::world::{Stage, World};

/// Your name on the trail drops this much when a recruit catches a false
/// thing as you say it (word gets around). A broken promise costs 0.08 of
/// a name; this is a quarter of that. Proposed, not confirmed.
pub const CAUGHT_TRAIL: f32 = 0.02;

/// A money figure counts as true up to this much over your real offer
/// (people round). Proposed, not confirmed.
pub const MONEY_SLACK: f64 = 0.10;

/// A saved lie to a recruit who has signed with you but not arrived is
/// looked at again this many days on. Proposed, not confirmed.
pub const ARRIVAL_CHECK_DAYS: i32 = 7;

/// What checking a fact finds, and the day he can see it when that is
/// not today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Found {
    pub truth: Truth,
    pub sees: Option<Date>,
}

impl Found {
    fn now(truth: Truth) -> Found {
        Found { truth, sees: None }
    }
    const UNKNOWN: Found = Found { truth: Truth::Unknown, sees: None };
}

/// The recruit or transfer a talk is with, and your program: (his place
/// in the people, his id, the program, a transfer).
pub fn who(w: &World, s: &Scene) -> Option<(usize, u32, usize, bool)> {
    let transfer = match s.kind {
        Scenes::PortalCall | Scenes::PortalVisit => true,
        Scenes::HomeVisit
        | Scenes::RecruitCall
        | Scenes::RecruitMessage
        | Scenes::FamilyCall
        | Scenes::OfficialTalk => false,
        _ => return None,
    };
    let id = s.a as u32;
    let i = person(w, id)?;
    let j = s.b as usize;
    (j < w.programs.len()).then_some((i, id, j, transfer))
}

pub fn check(w: &World, s: &Scene, stated: &Stated) -> Truth {
    read(w, s, stated).truth
}

/// Checks one typed fact in a talk with a recruit or a transfer.
pub fn read(w: &World, s: &Scene, f: &Stated) -> Found {
    let Some((i, id, j, transfer)) = who(w, s) else { return Found::UNKNOWN };
    let v = f.value.to_lowercase();
    // Where he will stand is a promise about what he'll see.
    if STANDING.contains(&f.kind.as_str()) && (f.about.is_none() || f.about == Some(id)) {
        return Found::now(Truth::Later);
    }
    match f.kind.as_str() {
        "record" => record(w, j, &v).map_or(Found::UNKNOWN, Found::now),
        "nil_amount" | "money" if f.about.is_none() || f.about == Some(id) => money(w, i, j, transfer, &v),
        "status" => f.about.and_then(|a| status(w, i, j, a, &v)).map_or(Found::UNKNOWN, Found::now),
        // Who comes back at his spot, said as a number: the public roster.
        "depth_rank" | "role" | "snaps_share" if f.about.is_none() => {
            room(w, i, j, &v).map_or(Found::UNKNOWN, Found::now)
        }
        _ => Found::UNKNOWN,
    }
}

/// The public record: national and conference titles, last season's
/// record, draft picks (every pick on the program's record, the most a
/// coach could mean), first-rounders.
fn record(w: &World, j: usize, v: &str) -> Option<Truth> {
    let p = &w.programs[j];
    if let Some((won, lost)) = win_loss(v) {
        let last = p.seasons.last()?;
        // A better record than the real one is false; a worse one is
        // modesty.
        return Some(if won > last.wins || (won == last.wins && lost < last.losses) {
            Truth::False
        } else {
            Truth::True
        });
    }
    let n = number(v)?;
    let has = |x: &[&str]| x.iter().any(|k| v.contains(k));
    let truth = if has(&["first round", "first-round", "1st round", "first rounder", "first-rounder"]) {
        p.draft_picks.iter().filter(|x| x.1 == 1).count() as f64
    } else if has(&["draft", "pick", "nfl", "pros", "the league"]) {
        p.draft_picks.len() as f64
    } else if has(&["conference"]) {
        p.conference_titles as f64
    } else if has(&["national", "title", "championship", "natty", "ring"]) {
        p.titles as f64
    } else {
        return None;
    };
    Some(if n > truth { Truth::False } else { Truth::True })
}

/// A money figure: your offer to him, checked on the day the money is
/// real (signing; a transfer's decision day). Under it is true; over it
/// by more than `MONEY_SLACK` is a lie he sees that day.
fn money(w: &World, i: usize, j: usize, transfer: bool, v: &str) -> Found {
    let Some(n) = dollars(v) else { return Found::UNKNOWN };
    let offer = if transfer { portal::place(w, i, j).1 } else { crate::world::recruiting::offer_money(w, j, i) };
    if n <= offer * (1.0 + MONEY_SLACK) + 1000.0 {
        Found::now(Truth::True)
    } else {
        Found { truth: Truth::False, sees: Some(money_day(w, i, transfer)) }
    }
}

/// The day the money is real for him: a transfer's decision day, else the
/// next signing date (the early period, then signing day).
pub fn money_day(w: &World, i: usize, transfer: bool) -> Date {
    if transfer && let Some(e) = w.people[i].cal.portal.filter(|e| !e.on_hold() && e.decide_by >= w.today) {
        return e.decide_by;
    }
    let y = &w.dates;
    [y.early_signing, Some(y.signing_day)]
        .into_iter()
        .flatten()
        .find(|&d| d >= w.today)
        .unwrap_or(y.signing_day.add(364))
}

/// Someone on your roster leaving (graduating, gone to the draft) or
/// staying, said of him by name: the public roster.
fn status(w: &World, i: usize, j: usize, about: u32, v: &str) -> Option<Truth> {
    let k = person(w, about)?;
    let p = &w.people[k];
    if p.stage != Stage::College || p.college != Some(j) || k == i {
        return None;
    }
    let going = ["leav", "graduat", "gone", "declar", "moving on", "last year", "his last", "out of eligibility"];
    let staying = ["coming back", "returning", "staying", "back next"];
    let says_going = going.iter().any(|x| v.contains(x));
    let says_staying = staying.iter().any(|x| v.contains(x));
    if says_going == says_staying {
        return None;
    }
    Some(if says_going == facts::leaving(w, k) { Truth::True } else { Truth::False })
}

/// How many come back at his spot, said as a number ("only one guy coming
/// back at corner"): the fact sheet's room. Fewer than really stay is
/// false; that many or more is true.
fn room(w: &World, i: usize, j: usize, v: &str) -> Option<Truth> {
    if !["back", "return", "staying", "stay"].iter().any(|x| v.contains(x)) {
        return None;
    }
    let n = number(v)?;
    let sheet = claims::recruit(w, i, j, Ear::Himself);
    let staying: f64 = sheet.get(Kind::Room)?.get("staying")?.parse().ok()?;
    Some(if n < staying { Truth::False } else { Truth::True })
}

/// A record said as wins and losses ("11-2", "eleven and two").
fn win_loss(v: &str) -> Option<(u32, u32)> {
    let t: Vec<&str> = v.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).filter(|x| !x.is_empty()).collect();
    for x in &t {
        if let Some((a, b)) = x.split_once('-')
            && let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>())
            && a + b <= 20
        {
            return Some((a, b));
        }
    }
    for k in 0..t.len().saturating_sub(2) {
        if t[k + 1] == "and"
            && let (Some(a), Some(b)) = (word_number(t[k]), word_number(t[k + 2]))
            && a + b <= 20.0
        {
            return Some((a as u32, b as u32));
        }
    }
    None
}

const WORDS: [&str; 21] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
];

fn word_number(x: &str) -> Option<f64> {
    if let Some(k) = WORDS.iter().position(|&n| n == x) {
        return Some(k as f64);
    }
    match x {
        "no" | "none" | "nobody" | "nothing" => Some(0.0),
        "a" | "an" | "single" | "lone" => Some(1.0),
        "couple" | "pair" => Some(2.0),
        "dozen" => Some(12.0),
        "thirty" => Some(30.0),
        "forty" => Some(40.0),
        "fifty" => Some(50.0),
        _ => x.trim_end_matches(['.', ',']).replace(',', "").parse::<f64>().ok(),
    }
}

/// The first count in the words: digits or a number word.
pub fn number(v: &str) -> Option<f64> {
    v.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == ','))
        .filter(|x| !x.is_empty())
        .find_map(|x| word_number(x.trim_end_matches(['.', ','])))
}

/// A money figure in the words: "$250,000", "250k", "a quarter million",
/// "1.2 million", "half a million".
pub fn dollars(v: &str) -> Option<f64> {
    let v = v.to_lowercase().replace(',', "");
    let t: Vec<&str> = v.split(|c: char| c.is_whitespace() || c == '$').filter(|x| !x.is_empty()).collect();
    let scale = |s: &str| match s.trim_end_matches(['.', ',']) {
        "k" | "thousand" | "grand" => Some(1e3),
        "m" | "mil" | "million" | "millions" => Some(1e6),
        _ => None,
    };
    for (k, x) in t.iter().enumerate() {
        let x = x.trim_end_matches(['.', ',']);
        let (digits, tail) = x.split_at(x.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(x.len()));
        let base = match digits.parse::<f64>() {
            Ok(n) => n,
            Err(_) => match x {
                "half" => 0.5,
                "quarter" => 0.25,
                _ => match word_number(x) {
                    Some(n) if n > 1.0 => n,
                    _ => continue,
                },
            },
        };
        let mult = scale(tail).or_else(|| {
            t.get(k + 1).and_then(|n| scale(n)).or_else(|| match t.get(k + 1) {
                Some(&"a") => t.get(k + 2).and_then(|n| scale(n)),
                _ => None,
            })
        });
        match mult {
            Some(m) => return Some(base * m),
            None if !digits.is_empty() && tail.is_empty() && base >= 1000.0 => return Some(base),
            None => {}
        }
    }
    None
}

// ---------------------------------------------------------------------------
// After a typed line lands.

/// What a talk's case held before a typed line landed, so a point he
/// caught as false can add nothing.
#[derive(Clone, Copy, Debug)]
pub struct Before {
    case: Option<Case>,
}

pub(crate) fn before(w: &World, s: &Scene) -> Option<Before> {
    let (i, id, j, transfer) = who(w, s)?;
    let case = if transfer { portal::case_of(w, id).copied() } else { case::of(&w.people[i], j).copied() };
    Some(Before { case })
}

/// A typed line has landed in a talk with a recruit or a transfer: a false
/// thing he can check now costs what it costs, at most once a line; one he
/// can only check on a day is saved as a lie with that day.
pub(crate) fn landed(w: &mut World, id: u32, facts: &[Stated], before: Before) {
    let Some(s) = w.folk.scenes.iter().find(|s| s.id == id).cloned() else { return };
    let Some((i, him, j, transfer)) = who(w, &s) else { return };
    let mut caught = false;
    for f in facts {
        let found = read(w, &s, f);
        if found.truth != Truth::False {
            continue;
        }
        match found.sees {
            None => caught = true,
            Some(day) => {
                let lie = claims::Lie {
                    to: crate::world::folk::talks::PromiseTo::Player(him),
                    fact: f.clone(),
                    said: w.today,
                    sees: day,
                    seen: false,
                    who: w.people[i].name.clone(),
                    recruit: true,
                };
                claims::truth::file_lie(w, lie);
            }
        }
    }
    if !caught {
        return;
    }
    // The point adds nothing, and the oversell costs.
    let today = w.today;
    let c = if transfer { portal::case_mut(w, him) } else { Some(case::case_mut(w, i, j)) };
    let mut moved = 0.0;
    if let Some(c) = c {
        let was = c.total();
        let old = before.case.unwrap_or(Case::new(j));
        c.groups = old.groups;
        c.made = old.made;
        c.again = old.again - case::OVERSELL;
        c.on = c.on.max(today);
        moved = c.total() - was;
    }
    w.folk.mine.word(Name::Trail, -CAUGHT_TRAIL);
    if let Some(sc) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
        if !sc.done {
            sc.f[0] += moved;
        }
        sc.outcome.push("He knew that wasn't so. It did you no good with him, and word gets around.".into());
    }
}

/// What happens to a saved lie to a recruit on its day.
pub(crate) enum Due {
    /// He sees it: it costs, as he is yours.
    Judge,
    /// Not yet: look again on this day (he signed with you and hasn't
    /// arrived, or he hasn't decided).
    Later(Date),
    /// He never sees it: he went elsewhere.
    Lapse,
}

/// Whether a lie told to a recruit is seen on its day: only once he is
/// yours. A signee who hasn't arrived is looked at again a week on; a
/// recruit who hasn't signed anywhere, at the next signing date; one who
/// went elsewhere never sees it.
pub(crate) fn due(w: &World, to: u32, today: Date) -> Due {
    let Some(i) = person(w, to) else { return Due::Lapse };
    let Some(j) = w.seat.and_then(|s| s.program) else { return Due::Lapse };
    let p = &w.people[i];
    match p.stage {
        Stage::College if p.college == Some(j) => Due::Judge,
        Stage::College if p.cal.portal.is_some_and(|e| !e.on_hold() && e.decide_by >= today) => {
            Due::Later(money_day(w, i, true).max(today.add(1)))
        }
        Stage::HighSchool => match p.cal.signed {
            Some(s) if s.program == j => Due::Later(today.add(ARRIVAL_CHECK_DAYS)),
            Some(_) => Due::Lapse,
            None => Due::Later(money_day(w, i, false).max(today.add(1))),
        },
        _ => Due::Lapse,
    }
}
