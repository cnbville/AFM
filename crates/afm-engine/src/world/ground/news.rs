//! News in order (rule 2, GR16, GR19, GR20, GR64; lane G4). Something
//! real happens to a kid in the world: he grows, has a big game, runs a
//! laser time, gets an offer from another program, moves in, qualifies
//! academically, gets a new coach or makes all-state. The people closest
//! to him know first (his coach, his trainer, the camp that timed him),
//! and they tell programs in order of their standing:
//!
//! - by level first: Home the day it happens, Trusted a day later, Known
//!   when he returns their calls, Strangers a day or two after the public;
//! - within a level a busy powerhouse coach answers by his nature (GR64):
//!   a loyal man calls the staff he's closest to, a self-serving one the
//!   biggest program; he makes a few calls a day;
//! - a small-school coach answers everyone he knows at once and pushes
//!   his kids (GR20);
//! - a coach who holds a broken word against you stops calling you;
//! - the public hears last: the services, then social media, faster from
//!   the 2010s; the paper (box scores, track results, all-state lists)
//!   reaches everyone at once.
//!
//! The order is fixed from standing and nature; only the exact day carries
//! a little noise (a day, from the world's seed). Only your seat keeps
//! news: every other program's place is read from its ground when the
//! news happens, and nothing in the world reads it, so the world plays the
//! same. The sizes are the build's picks (proposed, not confirmed).

use super::people::{self, Who};
use super::standing::{self, Level};
use super::{key, unit};
use std::collections::HashMap;

use crate::body::{body_now, height_share, height_text};
use crate::calendar::{Date, Weekday};
use crate::world::folk::tips::rank;
use crate::world::scouting::readers;
use crate::world::times::{At, Event as Run, Timed};
use crate::world::{Stage, World};

/// What happened to him.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum What {
    /// He grew, measured at his school's spring weigh-in: tenths of an inch
    /// in the last year.
    Grew(u16),
    /// A big game on the engine's box score.
    BigGame,
    /// A laser time at a camp or showcase (hundredths of a second), and
    /// the program whose camp timed him, if one did.
    Laser(u16, Option<u32>),
    /// A fully timed sprint at a track meet (the paper).
    Track(u16),
    /// Another program offered him.
    Offer(u32),
    /// He met the academic line.
    Qualified,
    /// He moved in from another school.
    MovedIn(u32),
    /// His school has a new head coach.
    NewCoach,
    /// First-team all-state (the paper).
    AllState,
}

impl What {
    /// Paper news reaches everyone at once: nobody hears it first.
    pub fn paper(self) -> bool {
        matches!(self, What::Track(_) | What::AllState | What::NewCoach | What::Laser(_, None))
    }

    /// What it was, in a few words ("his growth").
    pub fn words(self) -> &'static str {
        match self {
            What::Grew(_) => "his growth",
            What::BigGame => "his big game",
            What::Laser(..) => "his laser time",
            What::Track(_) => "his track time",
            What::Offer(_) => "his offer",
            What::Qualified => "his test score",
            What::MovedIn(_) => "his move",
            What::NewCoach => "his new coach",
            What::AllState => "all-state",
        }
    }

    /// A short name for the screens.
    pub fn key(self) -> &'static str {
        match self {
            What::Grew(_) => "grew",
            What::BigGame => "big_game",
            What::Laser(..) => "laser",
            What::Track(_) => "track",
            What::Offer(_) => "offer",
            What::Qualified => "qualified",
            What::MovedIn(_) => "moved_in",
            What::NewCoach => "new_coach",
            What::AllState => "all_state",
        }
    }
}

/// Why a program heard when it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Why {
    /// Its own camp timed him.
    There,
    /// His standing with the program, level by level.
    Standing,
    /// A loyal coach calls the staff he's closest to first (GR64).
    Closest,
    /// A self-serving coach calls the biggest program first (GR64).
    Biggest,
    /// A small-school coach calls everyone he knows at once (GR20).
    Pushes,
    /// He stopped calling after a broken word (GR24).
    Stopped,
    /// The services and social media.
    Public,
    /// The paper reaches everyone at once.
    Paper,
}

impl Why {
    pub fn key(self) -> &'static str {
        match self {
            Why::There => "there",
            Why::Standing => "standing",
            Why::Closest => "closest",
            Why::Biggest => "biggest",
            Why::Pushes => "pushes",
            Why::Stopped => "stopped",
            Why::Public => "public",
            Why::Paper => "paper",
        }
    }
}

/// One program's place in the order.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Heard {
    pub prog: u32,
    pub day: Date,
    /// Its level with the person who told it (0 Stranger to 3 Home).
    pub level: u8,
    pub why: Why,
}

/// One piece of news your seat keeps.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct News {
    pub key: u64,
    /// The kid, by player id.
    pub kid: u32,
    pub what: What,
    /// What happened, in words.
    pub line: String,
    /// The day it happened.
    pub date: Date,
    /// Who knew first and told (his coach, his trainer); None for the paper.
    pub source: Option<Who>,
    /// The programs that heard before the public, in order (the first
    /// `ORDER_KEPT`, and yours wherever it falls).
    pub order: Vec<Heard>,
    /// How many programs heard before the public in all.
    pub before_public: u16,
    /// The day the services and social media had it.
    pub public: Date,
    /// Your program's place: its day and why.
    pub you: Heard,
    /// Your place in the order (1 first); 0 when you heard with the public.
    pub place: u16,
    /// It has reached you (your lead has it).
    pub delivered: bool,
}

/// The programs a news item keeps in its order (yours always).
pub const ORDER_KEPT: usize = 8;
/// News your seat keeps (the oldest delivered go first).
pub const KEPT: usize = 1500;
/// Calls a busy coach makes a day (build's pick).
pub const CALLS_A_DAY: usize = 3;
/// A small-school coach's calls a day: he gets to everyone fast.
pub const SMALL_CALLS: usize = 8;
/// A school this strong (prestige) or a national one is a powerhouse whose
/// coach is too busy to call everyone at once (build's pick).
pub const BUSY: f64 = 0.5;
/// Days after it happens that each level hears, before his calls a day
/// spread it (Home, Trusted, Known; build's picks).
pub const LEVEL_DAYS: [i32; 3] = [0, 1, 3];

/// The days until the public has it, by what it is and the era (build's
/// picks): an offer the kid posts (from the 2010s the same day), a big
/// game in the next day's box scores, a laser time at a college's camp
/// when he posts it, a move in the local news, a test score when the
/// services hear it, a growth spurt only when a camp or the services
/// measure him. A small school has fewer services watching it: three more
/// days.
pub fn public_lag(what: What, year: i32, small: bool) -> i32 {
    let posts = year >= 2010;
    let base = match what {
        What::Offer(_) => {
            if year >= 2018 {
                0
            } else if posts {
                1
            } else {
                7
            }
        }
        What::BigGame => 1,
        What::Laser(..) => {
            if posts {
                2
            } else {
                14
            }
        }
        What::MovedIn(_) => {
            if year >= 2018 {
                3
            } else {
                7
            }
        }
        What::Qualified => 30,
        What::Grew(_) => {
            if posts {
                45
            } else {
                60
            }
        }
        What::Track(_) | What::NewCoach | What::AllState => 0,
    };
    base + if small { 3 } else { 0 }
}

/// A day name ("Monday").
pub fn weekday_name(d: Date) -> &'static str {
    match d.weekday() {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

/// When, against the day it happened: the weekday within the week, a
/// date beyond it.
pub fn when(d: Date, from: Date) -> String {
    if from.days_to(d) < 7 { weekday_name(d).to_string() } else { d.pretty() }
}

/// Every program's standing with a person (the higher of its program's
/// and its best current man's own), and whether he holds a broken word
/// against it; only programs at Known or better, and `always`.
fn standings(w: &World, who: Who, always: usize) -> Vec<(usize, f32, bool)> {
    let d = w.today;
    let mut v = Vec::new();
    for p in 0..w.programs.len() {
        let (pts, holder) = standing::staff(w, p, who);
        let stopped = w.ground.get(who, holder).is_some_and(|m| m.holding(d).is_some());
        if pts >= Level::Known.floor() || p == always {
            v.push((p, pts, stopped));
        }
    }
    v
}

/// The order a piece of news goes in: the programs that hear before the
/// public, each with its day and why, and the public's day. `host` is a
/// program whose own camp timed him: it knew first.
pub fn order(
    w: &World,
    what: What,
    source: Option<Who>,
    school: usize,
    date: Date,
    salt: u64,
    host: Option<usize>,
) -> Order {
    let you = readers::seat_staff(w)
        .and_then(|s| if let readers::Staff::Program(p) = s { Some(p) } else { None })
        .unwrap_or(usize::MAX);
    let h = w.high_schools.get(school);
    let busy = h.is_some_and(|h| h.prestige >= BUSY || h.national);
    let small = h.is_some_and(|h| h.prestige < 0.35 && !h.national) && !what.paper();
    let public = date.add(public_lag(what, date.year(), small));
    let jitter = |p: usize| (unit(w.options.seed, &[0x4E35, salt, p as u64]) < 0.3) as i32;
    let mut heard: Vec<Heard> = Vec::new();
    if let Some(j) = host {
        heard.push(Heard { prog: j as u32, day: date, level: 3, why: Why::There });
    }
    let mut stopped_you = false;
    if let Some(who) = source {
        let loyal = people::makeup(w, who).loyal >= 0.5;
        let mut list: Vec<(usize, f32, Level)> = Vec::new();
        for (p, pts, stopped) in standings(w, who, you) {
            if Some(p) == host {
                continue;
            }
            if stopped {
                if p == you {
                    stopped_you = true;
                }
                continue;
            }
            let lv = Level::of(pts);
            if lv >= Level::Known {
                list.push((p, pts, lv));
            }
        }
        // By level, then by his nature within it (GR64), then by id.
        list.sort_by(|a, b| {
            b.2.cmp(&a.2)
                .then_with(|| {
                    if loyal {
                        b.1.total_cmp(&a.1)
                    } else {
                        w.programs[b.0].prestige.total_cmp(&w.programs[a.0].prestige)
                    }
                })
                .then(a.0.cmp(&b.0))
        });
        let mut last = date;
        let mut k_in = 0usize;
        let mut lv_now: Option<Level> = None;
        for (k, &(p, _, lv)) in list.iter().enumerate() {
            if lv_now != Some(lv) {
                lv_now = Some(lv);
                k_in = 0;
            }
            let (base, per, why) = if busy {
                (LEVEL_DAYS[3 - lv as usize], CALLS_A_DAY, if loyal { Why::Closest } else { Why::Biggest })
            } else {
                // A small-school coach goes down his list fast, whatever the level.
                (0, SMALL_CALLS, Why::Pushes)
            };
            let spread = if busy { k_in / per } else { k / per } as i32;
            let day = date.add(base + spread + jitter(p)).max(last);
            k_in += 1;
            if day >= public {
                break;
            }
            last = day;
            heard.push(Heard { prog: p as u32, day, level: lv as u8, why });
        }
    }
    let your_level = match source {
        Some(who) if you < w.programs.len() => standing::level(w, you, who) as u8,
        _ => 0,
    };
    let paper = source.is_none();
    let you_heard = heard.iter().position(|x| x.prog as usize == you);
    let me = match you_heard {
        Some(k) => heard[k],
        None => Heard {
            prog: you as u32,
            // The paper reaches everyone at once; a stranger hears a day or
            // two after the public.
            day: if paper { public } else { public.add(1 + jitter(you ^ 0x55)) },
            level: your_level,
            why: if paper {
                Why::Paper
            } else if stopped_you {
                Why::Stopped
            } else {
                Why::Public
            },
        },
    };
    Order { heard, public, you: me, place: you_heard.map_or(0, |k| k as u16 + 1) }
}

/// A news item's order.
#[derive(Clone, Debug)]
pub struct Order {
    pub heard: Vec<Heard>,
    pub public: Date,
    pub you: Heard,
    pub place: u16,
}

/// The order in words: "Auburn: Monday. You: Tuesday." (rule 2).
pub fn order_words(w: &World, n: &News) -> String {
    let mut parts: Vec<String> = Vec::new();
    for h in n.order.iter().take(3) {
        let name = if n.place > 0 && h.prog == n.you.prog { "You" } else { w.program_name(h.prog as usize) };
        parts.push(format!("{name}: {}.", when(h.day, n.date)));
    }
    if n.place == 0 || n.place > 3 {
        parts.push(format!("You: {}.", when(n.you.day, n.date)));
    }
    parts.join(" ")
}

/// Why you were where you were, in words.
pub fn why_words(w: &World, n: &News) -> String {
    let who = n.source.map_or_else(|| "The paper".to_string(), |s| people::name(w, s));
    let lv = Level::ALL[(n.you.level as usize).min(3)].name();
    match n.you.why {
        Why::There => "Your own camp timed him.".into(),
        Why::Paper => "It was in the paper: everyone read it the same day.".into(),
        Why::Stopped => format!("{who} stopped calling you after your word broke with him."),
        Why::Public => format!("You're a {lv} to {who}, so you heard it with the public."),
        Why::Pushes => format!("{who} calls everyone he knows when a kid of his does something."),
        Why::Standing | Why::Closest | Why::Biggest => {
            let ahead = n.place.saturating_sub(1);
            let first = n.order.first().map(|h| w.program_name(h.prog as usize)).unwrap_or("nobody");
            match (ahead, n.you.why) {
                (0, _) => format!("You're {lv} with {who}: he called you first."),
                (_, Why::Closest) => {
                    format!("You're {lv} with {who}, and he's closer to {first}: he called them first.")
                }
                (_, Why::Biggest) => format!("You're {lv} with {who}, and he calls the biggest program first."),
                _ => format!("You're {lv} with {who}; {ahead} staff ahead of you heard first."),
            }
        }
    }
}

/// Keeps a news item for your seat, once by its key: false if it was kept.
pub(crate) fn keep(w: &mut World, n: News) -> bool {
    let g = &mut w.ground;
    if g.news.iter().any(|x| x.key == n.key) {
        return false;
    }
    g.news.push(n);
    if g.news.len() > KEPT {
        let drop = g.news.iter().position(|x| x.delivered).unwrap_or(0);
        g.news.remove(drop);
    }
    true
}

/// A news item for your seat, made from its order.
#[allow(clippy::too_many_arguments)]
pub(crate) fn make(
    w: &World,
    kid: u32,
    what: What,
    line: String,
    date: Date,
    source: Option<Who>,
    school: usize,
    host: Option<usize>,
    once: u64,
) -> News {
    let k = key(&[0x6E35, kid as u64, what_key(what), once]);
    let o = order(w, what, if what.paper() { None } else { source }, school, date, k, host);
    let before = o.heard.len() as u16;
    let mut kept: Vec<Heard> = o.heard.iter().take(ORDER_KEPT).copied().collect();
    if o.place as usize > ORDER_KEPT {
        kept.push(o.you);
    }
    News {
        key: k,
        kid,
        what,
        line,
        date,
        source: if what.paper() { None } else { source },
        order: kept,
        before_public: before,
        public: o.public,
        you: o.you,
        place: o.place,
        delivered: false,
    }
}

fn what_key(what: What) -> u64 {
    match what {
        What::Grew(x) => (1 << 32) | x as u64,
        What::BigGame => 2 << 32,
        What::Laser(x, j) => (3 << 32) | x as u64 | ((j.unwrap_or(0) as u64) << 16),
        What::Track(x) => (4 << 32) | x as u64,
        What::Offer(j) => (5 << 32) | j as u64,
        What::Qualified => 6 << 32,
        What::MovedIn(h) => (7 << 32) | h as u64,
        What::NewCoach => 8 << 32,
        What::AllState => 9 << 32,
    }
}

/// A recruit whose news your seat follows: in high school, sophomore to
/// senior, not signed.
pub(crate) fn recruit(p: &crate::world::Person, season: i32) -> bool {
    matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
        && p.class_of > season
        && p.class_of <= season + 3
        && p.player.age >= 15
        && p.cal.signed.is_none()
}

/// The states your program recruits, for the paper: your staff's areas,
/// else your own state, the states where your footprint is 0.3 or more and
/// those within 400 miles.
pub(crate) fn region(w: &World, prog: usize) -> [bool; 51] {
    let mut r = [false; 51];
    let mut any = false;
    for (_, states) in &w.scouting.dept.areas {
        for &s in states {
            r[(s as usize).min(50)] = true;
            any = true;
        }
    }
    if !any {
        let p = &w.programs[prog];
        let home = p.state.min(50);
        for (s, x) in r.iter_mut().enumerate() {
            *x = s == home || p.footprint[s] >= 0.3 || crate::world::geo::miles(home, s) <= 400.0;
        }
    }
    r
}

/// Your staff's level with each school's head coach, worked out once a pass.
pub(crate) struct Levels {
    prog: usize,
    memo: HashMap<usize, Level>,
}

impl Levels {
    pub(crate) fn new(prog: usize) -> Levels {
        Levels { prog, memo: HashMap::new() }
    }

    pub(crate) fn school(&mut self, w: &World, hs: usize) -> Level {
        let prog = self.prog;
        *self.memo.entry(hs).or_insert_with(|| {
            people::head_coach_of(w, hs).map_or(Level::Stranger, |who| standing::level(w, prog, who))
        })
    }
}

/// A kid your seat follows the news of: his coach is Known or better with
/// your staff (he'd tell you), or he's a three-star or better in the states
/// you recruit (you'd read it with the public).
fn follows(w: &World, i: usize, lv: &mut Levels, region: &[bool; 51]) -> bool {
    let p = &w.people[i];
    lv.school(w, p.hs) >= Level::Known || (region[p.home_state.min(50)] && p.stars >= 3)
}

/// His trainer (GR62): lane G7's, the same one every time.
pub(crate) fn trainer(w: &World, i: usize) -> Option<Who> {
    // One trainer per kid, the same one lane G7's people know.
    people::trainer_of(w, i)
}

/// A laser forty that makes news at his spot (build's picks): 4.45 for the
/// backs, receivers and DBs, 4.65 for the backers, ends, tight ends and
/// quarterbacks, 5.00 for the big men.
fn fast(pos: crate::positions::Position) -> f32 {
    use crate::positions::Position as P;
    match pos {
        P::RB | P::WR | P::CB | P::S => 4.45,
        P::LB | P::EDGE | P::TE | P::QB => 4.65,
        P::OT | P::IOL | P::DT => 5.00,
        _ => 0.0,
    }
}

/// A fully timed 100 meters that makes the paper: 10.75 (build's pick).
const FAST_100: f32 = 10.75;
/// He grew this much in a year (inches) for his coach to call about it.
const GREW: f64 = 1.5;

/// Mondays: what happened to the kids your seat follows in the last week
/// (times run, offers from other programs, a move to a new school, the
/// spring weigh-in in April, the academic line from October, first-team
/// all-state from December, a new coach at his school), each kept with
/// its order.
pub(crate) fn week(w: &mut World, prog: usize, d: Date) {
    let season = w.dates.season;
    let region = region(w, prog);
    let mut lv = Levels::new(prog);
    let from = d.add(-7);
    let weigh_in = d >= Date::ymd(d.year(), 4, 1) && d < Date::ymd(d.year(), 4, 8);
    // The academic line through the senior year, October to May.
    let fall = d.parts().1 >= 10 || d.parts().1 <= 5;
    let december = d.parts().1 == 12 || d.parts().1 <= 1;
    let mut out: Vec<News> = Vec::new();
    let mut notq: Vec<u32> = Vec::new();
    let mut heads: Vec<(u32, u32)> = Vec::new();
    let mine = rank(w.programs[prog].division);
    for i in 0..w.people.len() {
        if !recruit(&w.people[i], season) {
            continue;
        }
        let p = &w.people[i];
        let id = p.player.id;
        let hs = p.hs;
        // A new coach at his school: news to staffs that knew the old one.
        let old = w.ground.heads.binary_search_by_key(&(hs as u32), |x| x.0).ok().map(|k| w.ground.heads[k].1);
        let now_coach = w.high_schools.get(hs).and_then(|h| h.coach).map(|c| c as u32);
        if let (Some(o), Some(n)) = (old, now_coach)
            && o != n
            && super::leads::lead(w, id).is_some()
            && standing::level(w, prog, Who::Coach(o)) >= Level::Known
        {
            let line = format!("{} has a new head coach: {}.", w.school_name(hs), people::name(w, Who::Coach(n)));
            out.push(make(w, id, What::NewCoach, line, d, Some(Who::Coach(n)), hs, None, n as u64));
        }
        if !follows(w, i, &mut lv, &region) {
            continue;
        }
        if lv.school(w, hs) >= Level::Known
            && let Some(n) = now_coach
        {
            heads.push((hs as u32, n));
        }
        let coach = people::head_coach_of(w, hs);
        let name = p.name.clone();
        let pos = p.player.position;
        // Times run this week.
        for t in w.times.of(id) {
            if t.date <= from || t.date > d {
                continue;
            }
            match (t.event, t.timed, t.at) {
                (Run::Forty, Timed::Laser, At::Camp(_) | At::Regional | At::National) if t.secs <= fast(pos) => {
                    let host = if let At::Camp(j) = t.at { Some(j as usize) } else { None };
                    let place = match t.at {
                        At::Camp(j) => format!("the {} camp", w.program_name(j as usize)),
                        At::National => "the national showcase".into(),
                        _ => "a regional showcase".into(),
                    };
                    let line = format!("{name} ran a laser {:.2} forty at {place}, at {} lb.", t.secs, t.lb);
                    let src = trainer(w, i).or(coach);
                    let what = What::Laser((t.secs * 100.0).round() as u16, host.map(|j| j as u32));
                    out.push(make(w, id, what, line, t.date, src, hs, host, t.date.0 as u64));
                }
                (Run::M100, Timed::Fat, At::Meet | At::StateMeet) if t.secs <= FAST_100 => {
                    let meet = if t.at == At::StateMeet { "the state meet" } else { "a track meet" };
                    let line = format!("{name} ran a {:.2} 100 meters at {meet}.", t.secs);
                    let what = What::Track((t.secs * 100.0).round() as u16);
                    out.push(make(w, id, what, line, t.date, None, hs, None, t.date.0 as u64));
                }
                _ => {}
            }
        }
        // Offers from other programs this week: a coach passes on an offer
        // from your level or one below; offers from further down aren't
        // news to you (build's pick, proposed, not confirmed).
        for &(j, od) in &p.cal.early_offers {
            if j == prog || od <= from || od > d || rank(w.programs[j].division) + 1 < mine {
                continue;
            }
            let line = format!("{} offered {name}.", w.program_name(j));
            out.push(make(w, id, What::Offer(j as u32), line, od, coach, hs, None, j as u64));
        }
        // A kid who moved in: his last season was at another school.
        if let Some(l) = p.hs_lines.last()
            && l.school as usize != hs
            && l.year as i32 >= season - 1
        {
            let line = format!("{name} moved in from {}.", w.school_name(l.school as usize));
            out.push(make(w, id, What::MovedIn(l.school), line, d, coach, hs, None, l.school as u64));
        }
        // The spring weigh-in.
        if weigh_in {
            let age = p.player.age as f64 + f64::from(p.player.grow);
            let full = p.player.body.height;
            let grew = full * (height_share(age, p.player.bloom) - height_share(age - 1.0, p.player.bloom));
            if grew >= GREW {
                let now = body_now(&p.player);
                let line = format!(
                    "{name} grew {:.1} inches since last spring: {}, {} lb at the spring weigh-in.",
                    grew,
                    height_text(now.height.round()),
                    now.weight.round()
                );
                let what = What::Grew((grew * 10.0).round() as u16);
                out.push(make(w, id, what, line, d, coach, hs, None, d.year() as u64));
            }
        }
        // The academic line, for seniors from October.
        if fall && p.class_of == season + 1 {
            let gpa = crate::world::recruiting::eligibility_gpa(p.bg.gpa, p.cal.test_score, p.class_of);
            let tier = crate::world::recruiting::ncaa_tier(crate::world::data::Division::FBS, gpa);
            let was_not = w.ground.notq.binary_search(&id).is_ok();
            if tier == crate::world::recruiting::Eligibility::Qualifier {
                if was_not {
                    let line = format!("{name}'s new test score puts him over the academic line.");
                    out.push(make(w, id, What::Qualified, line, d, coach, hs, None, 0));
                }
            } else {
                notq.push(id);
            }
        }
        // First-team all-state (the paper).
        if december && p.cal.all_state >= 2 {
            let line = format!("{name} made first-team all-state.");
            out.push(make(w, id, What::AllState, line, d, None, hs, None, season as u64));
        }
    }
    for n in out {
        keep(w, n);
    }
    let g = &mut w.ground;
    // This scan's seniors under the line replace the last; out of the
    // senior year there are none.
    notq.sort_unstable();
    notq.dedup();
    g.notq = if fall { notq } else { Vec::new() };
    heads.sort_unstable();
    heads.dedup_by_key(|x| x.0);
    g.heads = heads;
}

/// A big line on a high school game's box score (build's picks): 300
/// passing yards or four touchdown passes, 200 rushing or three rushing
/// touchdowns, 150 receiving or three catches for scores, three sacks,
/// four tackles for loss, two interceptions.
fn big_line(s: &[i32; crate::world::hs_year::HS_STAT_COUNT]) -> Option<String> {
    use crate::world::hs_year::HsStat as S;
    let g = |k: S| s[k as usize];
    let half = |k: S| s[k as usize] as f64 / 2.0;
    if g(S::PassYds) >= 300 || g(S::PassTd) >= 4 {
        return Some(format!("threw for {} yards and {} touchdowns", g(S::PassYds), g(S::PassTd)));
    }
    if g(S::RushYds) >= 200 || g(S::RushTd) >= 3 {
        return Some(format!("ran for {} yards and {} touchdowns", g(S::RushYds), g(S::RushTd)));
    }
    if g(S::RecYds) >= 150 || g(S::RecTd) >= 3 {
        return Some(format!("caught {} for {} yards and {} touchdowns", g(S::Rec), g(S::RecYds), g(S::RecTd)));
    }
    if half(S::Sacks) >= 3.0 {
        return Some(format!("had {} sacks", half(S::Sacks)));
    }
    if half(S::Tfl) >= 4.0 {
        return Some(format!("had {} tackles for loss", half(S::Tfl)));
    }
    if g(S::Ints) >= 2 {
        return Some(format!("picked off {} passes", g(S::Ints)));
    }
    None
}

/// A high school game's lines (on the engine): a big game for a kid your
/// seat follows is news from his coach. Only reads the game.
pub(crate) fn on_hs_game(
    w: &mut World,
    d: Date,
    schools: [usize; 2],
    lines: &[Vec<(usize, [i32; crate::world::hs_year::HS_STAT_COUNT], bool)>; 2],
) {
    if w.ground.off || w.career.is_none() {
        return;
    }
    let Some(readers::Staff::Program(prog)) = readers::seat_staff(w) else { return };
    let season = w.dates.season;
    let region = region(w, prog);
    let mut lv = Levels::new(prog);
    let mut out = Vec::new();
    for side in 0..2 {
        for (i, s, _) in &lines[side] {
            let i = *i;
            let Some(p) = w.people.get(i) else { continue };
            if !recruit(p, season) {
                continue;
            }
            let Some(what) = big_line(s) else { continue };
            if !follows(w, i, &mut lv, &region) {
                continue;
            }
            let other = w.school_name(schools[1 - side]);
            let line = format!("{} {what} against {other}.", p.name);
            let coach = people::head_coach_of(w, p.hs);
            out.push(make(w, p.player.id, What::BigGame, line, d, coach, p.hs, None, d.0 as u64));
        }
    }
    for n in out {
        keep(w, n);
    }
}

/// Each day: the news whose day has come for you reaches you, as a lead.
pub(crate) fn deliver(w: &mut World, prog: usize, d: Date) {
    let due: Vec<usize> =
        (0..w.ground.news.len()).filter(|&k| !w.ground.news[k].delivered && w.ground.news[k].you.day <= d).collect();
    for k in due {
        w.ground.news[k].delivered = true;
        let n = w.ground.news[k].clone();
        super::leads::from_news(w, prog, &n);
    }
}
