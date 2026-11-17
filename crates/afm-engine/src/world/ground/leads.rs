//! Leads (GR67, part G; lane G4): every kid you find starts as a lead,
//! and leads come only from work and the crowded public paper:
//!
//! - his coach's list, by your standing (GR19): a Stranger gets the public
//!   answer a day or two later, Known his real list the same day, Trusted
//!   the truth about a kid and his tape before you ask;
//! - his coach (or his trainer) calling with news, in order of standing
//!   (`news`);
//! - a tip with tape from a trainer, a 7v7 man, an alum or a coach
//!   (`folk/tips.rs`);
//! - the camp walk-up, a kid who pops off another kid's tape;
//! - the paper: box scores, track results, all-state lists and other
//!   staffs' offers, which reach everyone at once.
//!
//! A lead has five blanks (measured size, timed speed, tape, a live look,
//! grades and character). They are the built looks in your file
//! (`world/scouting`), so a lead becomes a scouting file, and filling one
//! asks for the look that fills it (`fill`), which always fills it. His
//! word on the kid's size and speed settles when your camp measures him:
//! an oversold kid drops the teller's record with you. The sizes are the
//! build's picks (proposed, not confirmed).

use std::collections::HashMap;

use super::news::{self, News};
use super::people::{self, Who};
use super::standing::{self, Level};
use crate::body::{Drills, body_now, height_text};
use crate::calendar::Date;
use crate::world::scouting::{Kind, SeatLook, readers};
use crate::world::{Stage, World};

/// How a lead came to you.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum How {
    /// On his coach's list of prospects (GR19).
    List,
    /// His coach or trainer called you with news (rule 2).
    Called,
    /// He told you when your staff called around.
    Told,
    /// A tip with his tape (a trainer, a 7v7 man, an alum, a coach).
    Tip,
    /// He walked up at your camp.
    Camp,
    /// He popped off another kid's tape.
    Tape,
    /// The paper: everyone read it the same day.
    Paper,
    /// You picked him out yourself, in person (a school's weight room, your
    /// camp's stations: lane G5's scenes).
    Seen,
}

impl How {
    pub fn key(self) -> &'static str {
        match self {
            How::List => "list",
            How::Called => "called",
            How::Told => "told",
            How::Tip => "tip",
            How::Camp => "camp",
            How::Tape => "tape",
            How::Paper => "paper",
            How::Seen => "seen",
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            How::List => "on his coach's list",
            How::Called => "a call with news",
            How::Told => "told when your staff called",
            How::Tip => "a tip with his tape",
            How::Camp => "walked up at your camp",
            How::Tape => "popped off another kid's tape",
            How::Paper => "the paper",
            How::Seen => "you saw him yourself",
        }
    }
}

/// One lead.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Lead {
    /// The kid, by player id.
    pub kid: u32,
    /// The day you heard (a Stranger's public answer comes a day or two on).
    pub since: Date,
    pub how: How,
    /// The person on your ground who told you.
    pub from: Option<Who>,
    /// The tipper's name when he isn't one of your ground's people (an
    /// alum, a youth coach).
    pub tipper: Option<String>,
    /// Your man who heard it (credited with the find).
    pub man: Option<u32>,
    /// Your standing with the teller then (0 Stranger to 3 Home).
    pub level: u8,
    /// What he said, in his own voice.
    pub said: String,
    /// The size and speed he gave (inches, seconds): his word, settled
    /// when your camp measures the kid.
    pub height: Option<f32>,
    pub forty: Option<f32>,
    /// His claim: 0 worth a look, 1 can play for you, 2 a sure thing.
    pub claim: Option<u8>,
    /// The news about him that reached you, newest last (`news::News::key`).
    pub news: Vec<u64>,
    /// His word settled: held (the kid was what he said, or bloomed) or not.
    pub word: Option<bool>,
    /// You took him off your leads.
    pub dropped: bool,
}

/// A teller's record with your staff: how many of his kids held up.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Record {
    pub who: Who,
    pub held: u8,
    pub of: u8,
}

/// Leads your seat keeps (the oldest settled or dropped go first).
pub const KEPT: usize = 3000;
/// A coach gives his list once a month (build's pick).
pub const LIST_EVERY: i32 = 30;
/// The names on one list, at most.
pub const LIST_MAX: usize = 6;

/// The five blanks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Blank {
    Size,
    Speed,
    Tape,
    Live,
    Character,
}

impl Blank {
    pub const ALL: [Blank; 5] = [Blank::Size, Blank::Speed, Blank::Tape, Blank::Live, Blank::Character];

    pub fn key(self) -> &'static str {
        match self {
            Blank::Size => "size",
            Blank::Speed => "speed",
            Blank::Tape => "tape",
            Blank::Live => "live",
            Blank::Character => "character",
        }
    }

    pub fn from_key(k: &str) -> Option<Blank> {
        Blank::ALL.into_iter().find(|b| b.key() == k)
    }

    pub fn name(self) -> &'static str {
        match self {
            Blank::Size => "Measured size",
            Blank::Speed => "Timed speed",
            Blank::Tape => "Tape",
            Blank::Live => "A live look",
            Blank::Character => "Grades and character",
        }
    }

    /// What fills it, and its time.
    pub fn fills(self) -> (&'static str, &'static str) {
        match self {
            Blank::Size => ("Measure him at your camp in June", "a camp day"),
            Blank::Speed => ("Laser-time him at your camp in June", "a camp day"),
            Blank::Tape => ("Watch his tape", "about 40 minutes of film"),
            Blank::Live => ("See him live at practice or a game", "one evaluation day"),
            Blank::Character => ("Call his coach, teachers and counselor", "15 minutes"),
        }
    }

    /// Whether a look fills it.
    fn by(self, l: &SeatLook) -> bool {
        let spring = matches!(l.date.parts().1, 4 | 5);
        match self {
            // His school measures him in the spring; your camp and a
            // showcase measure him too.
            Blank::Size => matches!(l.kind, Kind::Camp | Kind::Showcase) || (spring && l.kind == Kind::Practice),
            Blank::Speed => matches!(l.kind, Kind::Camp | Kind::Showcase),
            Blank::Tape => matches!(l.kind, Kind::HsFilm | Kind::Film | Kind::SideFilm | Kind::CollegeFilm),
            Blank::Live => matches!(l.kind, Kind::Practice | Kind::Game | Kind::Camp | Kind::Showcase),
            Blank::Character => matches!(l.kind, Kind::Call | Kind::Transcript | Kind::HomeVisit),
        }
    }

    /// The look you ask for to fill it.
    fn ask(self, college: bool) -> Kind {
        match self {
            Blank::Size | Blank::Speed => Kind::Camp,
            Blank::Tape if college => Kind::CollegeFilm,
            Blank::Tape => Kind::HsFilm,
            Blank::Live => Kind::Practice,
            Blank::Character => Kind::Call,
        }
    }
}

/// One blank on a lead: who filled it and when.
#[derive(Clone, Debug)]
pub struct Filled {
    pub blank: Blank,
    pub filled: Option<(Date, u32)>,
    /// You asked for its look and it waits for its day.
    pub asked: bool,
}

/// The five blanks of a kid's file, each with the first look that filled it.
pub fn blanks(w: &World, id: u32) -> [Filled; 5] {
    let looks = w.scouting.dept.looks.get(&id);
    Blank::ALL.map(|b| {
        let filled = looks.and_then(|v| v.iter().filter(|l| b.by(l)).min_by_key(|l| l.date).map(|l| (l.date, l.who)));
        let asked = w.scouting.dept.asks.iter().any(|a| a.0 == id && a.1 == b.ask(false))
            || w.scouting.dept.asks.iter().any(|a| a.0 == id && a.1 == b.ask(true));
        Filled { blank: b, filled, asked }
    })
}

/// Your seat's program, when it is a college staff with ground.
pub(crate) fn seat(w: &World) -> Option<usize> {
    if w.ground.off || w.career.is_none() {
        return None;
    }
    match readers::seat_staff(w)? {
        readers::Staff::Program(p) => Some(p),
        _ => None,
    }
}

/// The lead on a kid, if you have one.
pub fn lead(w: &World, kid: u32) -> Option<&Lead> {
    let g = &w.ground;
    g.leads.binary_search_by_key(&kid, |l| l.kid).ok().map(|k| &g.leads[k])
}

/// Your leads that have reached you (today or before), newest first.
pub fn open(w: &World) -> Vec<&Lead> {
    let d = w.today;
    let mut v: Vec<&Lead> = w.ground.leads.iter().filter(|l| l.since <= d && !l.dropped).collect();
    v.sort_by(|a, b| b.since.cmp(&a.since).then(a.kid.cmp(&b.kid)));
    v
}

/// Keeps a lead: a kid's first lead stays (who told you first counts),
/// later news joins it. False if he was a lead already.
pub(crate) fn add(w: &mut World, l: Lead) -> bool {
    let g = &mut w.ground;
    match g.leads.binary_search_by_key(&l.kid, |x| x.kid) {
        Ok(k) => {
            let x = &mut g.leads[k];
            for n in l.news {
                if !x.news.contains(&n) {
                    x.news.push(n);
                }
            }
            if x.news.len() > 8 {
                x.news.remove(0);
            }
            // A dropped lead someone tells you about again is back.
            if x.dropped && l.how != How::Paper {
                x.dropped = false;
            }
            false
        }
        Err(k) => {
            g.leads.insert(k, l);
            if g.leads.len() > KEPT {
                // The oldest dropped or settled lead goes, else the oldest.
                let pick = |f: &dyn Fn(&Lead) -> bool| {
                    g.leads.iter().enumerate().filter(|(_, x)| f(x)).min_by_key(|(_, x)| x.since).map(|(k, _)| k)
                };
                let drop = pick(&|x: &Lead| x.dropped || x.word.is_some()).or_else(|| pick(&|_: &Lead| true));
                if let Some(k) = drop {
                    g.leads.remove(k);
                }
            }
            true
        }
    }
}

/// A teller's record with your staff.
pub fn record(w: &World, who: Who) -> (u8, u8) {
    let g = &w.ground;
    g.records.binary_search_by(|r| r.who.cmp(&who)).ok().map_or((0, 0), |k| (g.records[k].held, g.records[k].of))
}

/// His record in words: "4 of his 6 kids held up".
pub fn record_words(w: &World, who: Who) -> String {
    match record(w, who) {
        (_, 0) => "no record with you yet".into(),
        (h, n) => format!("{h} of his {n} {} held up", if n == 1 { "kid" } else { "kids" }),
    }
}

/// Settles a lead's word once: the teller's record moves.
pub(crate) fn settle(w: &mut World, kid: u32, held: bool) -> Option<Who> {
    let g = &mut w.ground;
    let k = g.leads.binary_search_by_key(&kid, |l| l.kid).ok()?;
    if g.leads[k].word.is_some() {
        return None;
    }
    g.leads[k].word = Some(held);
    let who = g.leads[k].from?;
    match g.records.binary_search_by(|r| r.who.cmp(&who)) {
        Ok(i) => {
            let r = &mut g.records[i];
            r.of = r.of.saturating_add(1);
            r.held = r.held.saturating_add(held as u8);
        }
        Err(i) => g.records.insert(i, Record { who, held: held as u8, of: 1 }),
    }
    Some(who)
}

/// Each sorted class's film values (the coach's read is placed in his
/// kid's class), worked out once a day.
#[derive(Clone, Debug, Default)]
pub(crate) struct Classes {
    pub(crate) day: Option<Date>,
    pub(crate) by: Vec<(i32, Vec<f64>)>,
    /// Each school's recruits (people indexes), for his coach's list.
    pub(crate) kids: HashMap<usize, Vec<usize>>,
}

fn classes(w: &mut World) -> &Classes {
    let d = w.today;
    if w.ground.classes.day != Some(d) {
        let season = w.dates.season;
        let played = crate::world::recruiting::hs_played(w);
        let mut by: Vec<(i32, Vec<f64>)> = (1..=3).map(|k| (season + k, Vec::new())).collect();
        let mut kids: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, q) in w.people.iter().enumerate() {
            if matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)
                && let Some(x) = by.iter_mut().find(|x| x.0 == q.class_of)
            {
                x.1.push(crate::world::recruiting::film_raw(q, &played));
                if news::recruit(q, season) {
                    kids.entry(q.hs).or_default().push(i);
                }
            }
        }
        for x in &mut by {
            x.1.sort_by(|a, b| a.total_cmp(b));
        }
        w.ground.classes = Classes { day: Some(d), by, kids };
    }
    &w.ground.classes
}

/// Where a read ranks in a class, 0-1.
fn percentile(c: &Classes, class_of: i32, read: f64) -> f64 {
    let Some(v) = c.by.iter().find(|x| x.0 == class_of).map(|x| &x.1) else { return 0.0 };
    if v.is_empty() {
        return 0.0;
    }
    v.partition_point(|&x| x < read) as f64 / v.len() as f64
}

/// A person's eye (1-20) and straightness (1-20, how little he oversells),
/// from his makeup (lane G7), and how hard his agenda pushes his kids
/// (0-1, build's picks): a trainer pushes his paying clients hardest, a
/// family loves its son, a coach his kids, a 7v7 man his team's name, a
/// junior college or prep coach his movers, a counselor least.
fn teller(w: &World, who: Who) -> (f64, f64, f64) {
    use super::people::Agenda as A;
    let m = super::people::makeup(w, who);
    let push = match m.agenda {
        A::Clients => 0.8,
        A::Family => 0.9,
        A::HisKids => 0.6,
        A::MovingUp => 0.6,
        A::Exposure => 0.5,
        A::Leavers => 0.4,
        A::Graduates => 0.2,
    };
    (m.eye as f64, m.straight as f64, push)
}

/// How far his word leans his way: his agenda, how little he holds back,
/// and how cool he is with you (as tips read it).
fn lean(w: &World, prog: usize, who: Who) -> f64 {
    let (_, straight, agenda) = teller(w, who);
    let warmth = (standing::staff(w, prog, who).0 / 100.0) as f64;
    (agenda * (1.0 - straight / 20.0) * (1.2 - warmth)).clamp(0.0, 1.0)
}

/// What he says a kid measures and runs: the truth, shaded his way, and
/// blurred by his eye.
fn numbers(w: &World, prog: usize, who: Who, i: usize, salt: u64) -> (f32, f32) {
    let p = &w.people[i];
    let id = p.player.id as u64;
    let (eye, _, _) = teller(w, who);
    let lean = lean(w, prog, who);
    let blur = (21.0 - eye) / 20.0;
    let h = body_now(&p.player).height + 0.6 * lean + 0.5 * blur * crate::world::season::key_normal(salt ^ 1, id);
    let f = Drills::of(&p.player).expected().forty - 0.08 * lean
        + 0.05 * blur * crate::world::season::key_normal(salt ^ 2, id);
    (h as f32, f as f32)
}

/// His claim on a kid from his read against your level's bar (as tips),
/// a level up when he leans: None below the floor (he doesn't call).
fn claim(w: &mut World, prog: usize, who: Who, i: usize, salt: u64) -> Option<u8> {
    let level = crate::world::folk::tips::rank(w.programs[prog].division);
    let (eye, _, _) = teller(w, who);
    let lean = lean(w, prog, who);
    let played = crate::world::recruiting::hs_played(w);
    let (value, class_of, id) = {
        let p = &w.people[i];
        (crate::world::recruiting::film_raw(p, &played), p.class_of, p.player.id)
    };
    let sd = 1.5 + 0.45 * (20.0 - eye).max(0.0);
    let read = value + sd * crate::world::season::key_normal(salt, id as u64);
    let pct = percentile(classes(w), class_of, read);
    let bar = crate::world::folk::tips::bar(false, level);
    if pct < crate::world::folk::tips::floor(false, level) {
        return None;
    }
    let mut c = if pct >= 1.0 - 0.35 * (1.0 - bar) {
        2
    } else if pct >= bar {
        1
    } else {
        0
    };
    if crate::world::folk::roll(w, salt ^ 0x0E5E, id as u64) < lean {
        c = (c + 1).min(2);
    }
    Some(c)
}

fn class_word(w: &World, class_of: i32) -> &'static str {
    match class_of - w.dates.season {
        1 => "senior",
        2 => "junior",
        3 => "sophomore",
        _ => "freshman",
    }
}

/// The truth about a kid (GR19, Trusted): his work, his family, the
/// injury he plays through.
fn truth(w: &World, i: usize) -> Vec<String> {
    use crate::world::background::FamilyStructure as F;
    let p = &w.people[i];
    let mut v = Vec::new();
    let we = p.player.personality.work_ethic;
    v.push(
        if we >= 14.0 {
            "He's the first one in the weight room and the last one out."
        } else if we >= 9.0 {
            "He works when he's pushed."
        } else {
            "Honestly, he coasts. You'll have to stay on him."
        }
        .to_string(),
    );
    v.push(match p.family.structure {
        F::TwoParents => "Both his parents are at every game.".to_string(),
        F::SingleParent => "His mom's raising him on her own, and she'll make the call.".to_string(),
        F::Grandparents => "His grandmother raised him. Win her over.".to_string(),
        F::Foster => "He's been through foster homes. We're his family as much as anyone.".to_string(),
    });
    let missed =
        p.player.history.iter().rev().find(|r| r.level == crate::player::Level::HS).map_or(0, |r| r.games_missed);
    v.push(if missed >= 2 {
        format!("He missed {missed} games hurt last fall and played through worse than that.")
    } else {
        "He's never missed a game hurt.".to_string()
    });
    v
}

/// His words about a kid on his list.
fn list_words(w: &World, i: usize, c: u8, (h, f): (f32, f32), trusted: bool) -> String {
    let p = &w.people[i];
    let pos = p.player.position.name();
    let weight = body_now(&p.player).weight.round();
    let fast = if p.player.position.is_specialist() { String::new() } else { format!(", runs about a {f:.2}") };
    let mut s = format!(
        "\"{}, a {} {pos}: {}, {weight}{fast}. {}.",
        p.name,
        class_word(w, p.class_of),
        height_text((h as f64 * 2.0).round() / 2.0),
        crate::world::folk::tips::CLAIMS[c as usize]
    );
    if trusted {
        for t in truth(w, i) {
            s.push(' ');
            s.push_str(&t);
        }
    }
    s.push('"');
    s
}

/// The man of yours who works a kid: the area man for his state, else his
/// position coach.
fn man_for(w: &World, prog: usize, i: usize) -> Option<u32> {
    crate::world::scouting::dept::recruiter_for(w, prog, i)
        .or_else(|| crate::world::scouting::dept::evaluator_for(w, prog, i))
        .map(|c| c as u32)
}

/// His coach's list (GR19), when your man visits or calls his school:
/// once a month a school. A Stranger gets the public answer (the names
/// everyone has) a day or two later; Known gets his real list the same
/// day; Trusted the truth about each kid and his tape before you ask.
pub(crate) fn coach_list(w: &mut World, prog: usize, man: Option<u32>, hs: usize, d: Date) {
    let Some(coach) = people::head_coach_of(w, hs) else { return };
    if let Ok(k) = w.ground.lists.binary_search_by(|x| x.0.cmp(&coach)) {
        if w.ground.lists[k].1.days_to(d) < LIST_EVERY {
            return;
        }
        w.ground.lists[k].1 = d;
    } else {
        let k = w.ground.lists.binary_search_by(|x| x.0.cmp(&coach)).unwrap_err();
        w.ground.lists.insert(k, (coach, d));
    }
    let lv = standing::level(w, prog, coach);
    let season = w.dates.season;
    let kids: Vec<usize> = classes(w)
        .kids
        .get(&hs)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|&i| w.people.get(i).is_some_and(|q| q.hs == hs && news::recruit(q, season)))
        .collect();
    let salt = super::key(&[0x115A, d.0 as u64, hs as u64]);
    let mut picked: Vec<(usize, u8)> = Vec::new();
    if lv == Level::Stranger {
        // The names everyone has: three stars or an offer.
        for &i in &kids {
            let p = &w.people[i];
            if p.stars >= 3 || !p.cal.offers.is_empty() {
                picked.push((i, 0));
            }
        }
    } else {
        for &i in &kids {
            if let Some(c) = claim(w, prog, coach, i, salt) {
                picked.push((i, c));
            }
        }
    }
    picked.sort_by_key(|x| std::cmp::Reverse(x.1));
    picked.truncate(LIST_MAX);
    let trusted = lv >= Level::Trusted;
    for (i, c) in picked {
        let id = w.people[i].player.id;
        if lead(w, id).is_some_and(|l| !l.dropped) {
            continue;
        }
        let (since, said, nums, claim) = if lv == Level::Stranger {
            let later = d.add(1 + (super::unit(w.options.seed, &[0x115B, salt, id as u64]) < 0.5) as i32);
            let p = &w.people[i];
            (later, format!("\"{} is the one everybody's asking about.\"", p.name), None, None)
        } else {
            let nums = numbers(w, prog, coach, i, salt);
            (d, list_words(w, i, c, nums, trusted), Some(nums), Some(c))
        };
        let man = man.or_else(|| man_for(w, prog, i));
        add(
            w,
            Lead {
                kid: id,
                since,
                how: How::List,
                from: Some(coach),
                tipper: None,
                man,
                level: lv as u8,
                said,
                height: nums.map(|x| x.0),
                forty: nums.map(|x| x.1),
                claim,
                news: Vec::new(),
                word: None,
                dropped: false,
            },
        );
        if trusted {
            // The truth is a read of his character, and his tape comes
            // before you ask.
            let who = man.map_or(readers::PERSONNEL, |c| readers::COACH + c);
            let v = w.scouting.dept.looks.entry(id).or_default();
            if !v.iter().any(|l| l.kind == Kind::Call && l.date == d) {
                v.push(SeatLook { who, kind: Kind::Call, date: d });
            }
            if !w.scouting.dept.asks.iter().any(|a| a.0 == id && a.1 == Kind::HsFilm) {
                w.scouting.dept.asks.push((id, Kind::HsFilm, d));
            }
        }
    }
}

/// A look by your staff (`scouting::dept::add`): a visit or call at his
/// school brings his coach's list, your camp brings walk-ups and measures
/// the kids his coach talked up, and a kid who pops off another kid's tape
/// is a lead.
pub(crate) fn on_look(w: &mut World, id: u32, code: u32, kind: Kind, d: Date) {
    let Some(prog) = seat(w) else { return };
    let Some(i) = crate::world::scouting::place(w, id) else { return };
    if !matches!(w.people[i].stage, Stage::HighSchool | Stage::PrepSchool) {
        return;
    }
    let man = (code >= readers::COACH).then(|| code - readers::COACH).filter(|&c| (c as usize) < w.coaches.len());
    let hs = w.people[i].hs;
    match kind {
        Kind::Practice | Kind::Game | Kind::Call => coach_list(w, prog, man, hs, d),
        Kind::Camp | Kind::Showcase => {
            let p = &w.people[i];
            let yours = p.cal.offers.contains(&prog);
            if kind == Kind::Camp && lead(w, id).is_none() && p.stars <= 2 && !yours {
                let b = body_now(&p.player);
                let said = format!(
                    "He walked up at your camp: {}, {} lb, and nobody had him.",
                    height_text(b.height.round()),
                    b.weight.round()
                );
                let man = man.or_else(|| man_for(w, prog, i));
                add(w, walk_up(id, d, How::Camp, man, said));
            }
            measured(w, i, d);
        }
        Kind::SideFilm if lead(w, id).is_none() => {
            let said = "He jumped off the film you were watching of another kid.".to_string();
            let man = man.or_else(|| man_for(w, prog, i));
            add(w, walk_up(id, d, How::Tape, man, said));
        }
        _ => {}
    }
}

/// A kid your own eye picked out in person (lane G5: a school's weight
/// room, your camp's stations): a lead you found yourself, unless he is
/// one already. True when it's new.
pub(crate) fn seen(w: &mut World, man: Option<u32>, id: u32, d: Date, said: String) -> bool {
    if lead(w, id).is_some_and(|l| !l.dropped) {
        return false;
    }
    add(w, walk_up(id, d, How::Seen, man, said))
}

/// Where a kid's film ranks in his class (0-1) when it is at or above your
/// level's floor (the tips' floor, `folk/tips.rs`): a kid worth noticing
/// in person. His real film, not anyone's read: whether you notice him is
/// your eye's roll.
pub(crate) fn worth(w: &mut World, prog: usize, i: usize) -> Option<f64> {
    let level = crate::world::folk::tips::rank(w.programs[prog].division);
    let played = crate::world::recruiting::hs_played(w);
    let (value, class_of) = {
        let p = w.people.get(i)?;
        (crate::world::recruiting::film_raw(p, &played), p.class_of)
    };
    let pct = percentile(classes(w), class_of, value);
    (pct >= crate::world::folk::tips::floor(false, level)).then_some(pct)
}

/// "senior", "junior"...
pub(crate) fn class_name(w: &World, class_of: i32) -> &'static str {
    class_word(w, class_of)
}

fn walk_up(kid: u32, d: Date, how: How, man: Option<u32>, said: String) -> Lead {
    Lead {
        kid,
        since: d,
        how,
        from: None,
        tipper: None,
        man,
        level: 0,
        said,
        height: None,
        forty: None,
        claim: None,
        news: Vec::new(),
        word: None,
        dropped: false,
    }
}

/// He was measured and timed (your camp, a showcase): the teller's word on
/// his size and speed settles if it was off (an inch, a tenth of a second).
fn measured(w: &mut World, i: usize, d: Date) {
    let id = w.people[i].player.id;
    let Some(l) = lead(w, id) else { return };
    if l.word.is_some() || l.height.is_none() {
        return;
    }
    let (said_h, said_f) = (l.height.unwrap_or(0.0) as f64, l.forty);
    let p = &w.people[i];
    let b = body_now(&p.player);
    let ran = w
        .times
        .of(id)
        .iter()
        .rev()
        .find(|t| t.event == crate::world::times::Event::Forty && t.timed == crate::world::times::Timed::Laser)
        .map_or_else(|| Drills::of(&p.player).expected().forty, |t| t.secs as f64);
    let short = said_h - b.height > 1.0;
    let slow = said_f.is_some_and(|f| ran - f as f64 > 0.10) && !p.player.position.is_specialist();
    if !short && !slow {
        return;
    }
    let name = p.name.clone();
    let Some(who) = settle(w, id, false) else { return };
    let teller = people::name(w, who);
    let what = if short {
        format!("{} at your camp, not the {} {teller} said", height_text(b.height.round()), height_text(said_h.round()))
    } else {
        format!("a {ran:.2} at your camp, not the {:.2} {teller} said", said_f.unwrap_or(0.0))
    };
    let body = vec![format!("{name} measured {what}."), format!("{teller}'s word with you: {}.", record_words(w, who))];
    standing::post_memo(w, who, d, format!("{name} was oversold"), body, 1);
}

/// A tip with tape (`folk/tips.rs`): a lead from the person who sent it.
pub(crate) fn from_tip(
    w: &mut World,
    sender: (usize, Option<crate::world::folk::network::Other>),
    id: u32,
    c: u8,
    text: &str,
    nums: Option<(f64, f64)>,
) {
    let Some(prog) = seat(w) else { return };
    let Some(i) = crate::world::scouting::place(w, id) else { return };
    if !matches!(w.people[i].stage, Stage::HighSchool | Stage::PrepSchool) {
        return;
    }
    use crate::world::folk::network::Other;
    let (from, tipper) = match sender.1 {
        None if people::coach_school(w, sender.0).is_some() => (Some(Who::Coach(sender.0 as u32)), None),
        None => (None, w.coaches.get(sender.0).map(|c| c.name.clone())),
        Some(Other::Former(f)) => match w.formers.get(f).map(|x| x.life) {
            Some(crate::world::former::Life::Trainer) => (Some(Who::Trainer(f)), None),
            Some(crate::world::former::Life::Camp) => (Some(Who::SevenOnSeven(f)), None),
            _ => (None, w.formers.get(f).map(|x| x.name.clone())),
        },
        Some(_) => return,
    };
    let level = from.map_or(0, |who| standing::level(w, prog, who) as u8);
    let man = man_for(w, prog, i);
    add(
        w,
        Lead {
            kid: id,
            since: w.today,
            how: How::Tip,
            from,
            tipper,
            man,
            level,
            said: text.to_string(),
            height: nums.map(|x| x.1 as f32),
            forty: nums.map(|x| x.0 as f32),
            claim: Some(c),
            news: Vec::new(),
            word: None,
            dropped: false,
        },
    );
}

/// News that reached you (`news::deliver`): a new lead when the public
/// doesn't rate him yet, else it joins his lead. A Home or Trusted coach
/// with a kid you didn't have calls you himself, at his hour.
pub(crate) fn from_news(w: &mut World, prog: usize, n: &News) {
    let Some(i) = crate::world::scouting::place(w, n.kid) else { return };
    if !matches!(w.people[i].stage, Stage::HighSchool | Stage::PrepSchool) {
        return;
    }
    let (how, from) = match n.source {
        _ if n.you.why == news::Why::There => (How::Camp, None),
        Some(s) if n.place > 0 && n.you.level >= Level::Trusted as u8 => (How::Called, Some(s)),
        Some(s) if n.place > 0 => (How::Told, Some(s)),
        _ => (How::Paper, None),
    };
    let said = match how {
        How::Called if n.you.level >= Level::Home as u8 => format!("\"{} Wanted you to hear it first.\"", n.line),
        How::Called => format!("\"{} Thought you'd want to know.\"", n.line),
        How::Told => format!("\"{}\"", n.line),
        _ => n.line.clone(),
    };
    // Only news that names a prospect makes a new lead, and only while the
    // public doesn't rate him yet: two stars or fewer from the paper, three
    // from a person.
    let had = lead(w, n.kid).is_some();
    let stars = w.people[i].stars;
    // An offer makes a lead only from your own level or above: a kid a
    // lower program wants is no news to your board (build's pick).
    let below = match n.what {
        news::What::Offer(j) => {
            let r = crate::world::folk::tips::rank;
            r(w.programs[j as usize].division) < r(w.programs[prog].division)
        }
        _ => false,
    };
    let names = !below && !matches!(n.what, news::What::NewCoach | news::What::Qualified);
    if !had && (!names || stars > if how == How::Paper { 2 } else { 3 }) {
        return;
    }
    let man = man_for(w, prog, i);
    let new = add(
        w,
        Lead {
            kid: n.kid,
            since: n.you.day,
            how,
            from,
            tipper: None,
            man,
            level: n.you.level,
            said: said.clone(),
            height: None,
            forty: None,
            claim: None,
            news: vec![n.key],
            word: None,
            dropped: false,
        },
    );
    // Only a man who calls you home rings you (build's pick); the rest
    // land on the board quietly.
    if new
        && how == How::Called
        && n.you.level >= Level::Home as u8
        && let Some(who) = from
    {
        let name = w.people[i].name.clone();
        let body = vec![said, news::order_words(w, n)];
        standing::post_call(w, who, n.you.day, format!("About {name}"), body, 1);
    }
}

/// Fills a blank (rule 1: it costs its time and always fills it): asks for
/// the look that fills it, which waits for its day (a camp in June, an
/// evaluation day, your staff's film within the week, a call within three
/// days).
pub fn fill(w: &mut World, id: u32, b: Blank) -> Result<String, String> {
    seat(w).ok_or("Ground works for a college staff.")?;
    let i = crate::world::scouting::place(w, id).ok_or("No such player.")?;
    let college = w.people[i].stage == Stage::College;
    if blanks(w, id).iter().any(|f| f.blank == b && f.filled.is_some()) {
        return Err(format!("{} is filled already.", b.name()));
    }
    let kind = b.ask(college);
    if w.scouting.dept.asks.iter().any(|a| a.0 == id && a.1 == kind) {
        return Err("You asked for it already: it waits for its day.".into());
    }
    let d = w.today;
    w.scouting.dept.asks.push((id, kind, d));
    let (what, time) = b.fills();
    let when = match b {
        Blank::Size | Blank::Speed => {
            let y = if d < Date::nth_weekday(d.year(), 6, crate::calendar::Weekday::Sat, 3) {
                d.year()
            } else {
                d.year() + 1
            };
            format!("He's invited to your camp in June {y}.")
        }
        Blank::Live => "A coach goes on your next evaluation day.".into(),
        Blank::Tape => "Your staff will watch it within the week.".into(),
        Blank::Character => "The calls go out within three days.".into(),
    };
    Ok(format!("{what} ({time}). {when}"))
}

/// Takes a kid off your leads (he can come back if someone tells you again).
pub fn drop(w: &mut World, id: u32) -> Result<String, String> {
    let k = w.ground.leads.binary_search_by_key(&id, |l| l.kid).map_err(|_| "He isn't one of your leads.")?;
    w.ground.leads[k].dropped = true;
    Ok("Off your leads.".into())
}

/// Who else knows about him (the lead card): the programs that heard his
/// news before you, and the programs that offered him.
pub fn who_else(w: &World, kid: u32) -> Vec<(usize, String)> {
    let mut v: Vec<(usize, String)> = Vec::new();
    let you = seat(w);
    for n in w.ground.news.iter().filter(|n| n.kid == kid) {
        for h in &n.order {
            let p = h.prog as usize;
            if Some(p) != you && h.day <= n.you.day && !v.iter().any(|x| x.0 == p) {
                v.push((p, format!("heard {} on {}", n.what.words(), news::weekday_name(h.day))));
            }
        }
    }
    if let Some(i) = crate::world::scouting::place(w, kid) {
        for &j in &w.people[i].cal.offers {
            if Some(j) == you {
                continue;
            }
            match v.iter_mut().find(|x| x.0 == j) {
                Some(x) => x.1.push_str(", offered"),
                None => v.push((j, "offered".into())),
            }
        }
    }
    v
}

/// The news about a kid your seat keeps, oldest first.
pub fn news_of(w: &World, kid: u32) -> Vec<&News> {
    let mut v: Vec<&News> = w.ground.news.iter().filter(|n| n.kid == kid && n.delivered).collect();
    v.sort_by_key(|n| n.date);
    v
}

/// Your board's ground part (`scouting::dept::watch`): your leads, and
/// your staff's level with each recruit's coach in place of footprint.
pub(crate) struct Board {
    leads: HashMap<u32, f64>,
    levels: news::Levels,
}

impl Board {
    /// None when Ground doesn't drive the board (no college seat, or the
    /// test switch is off): the built board stays.
    pub(crate) fn of(w: &World, prog: usize) -> Option<Board> {
        if seat(w) != Some(prog) {
            return None;
        }
        let d = w.today;
        let leads = w
            .ground
            .leads
            .iter()
            .filter(|l| !l.dropped && l.since <= d)
            .map(|l| {
                let filled = blanks(w, l.kid).iter().filter(|f| f.filled.is_some()).count() as f64;
                (l.kid, 4.0 + 2.0 * l.claim.unwrap_or(0) as f64 + filled)
            })
            .collect();
        Some(Board { leads, levels: news::Levels::new(prog) })
    }

    /// A lead's weight on the board, 0 if he isn't one.
    pub(crate) fn lead(&self, id: u32) -> f64 {
        self.leads.get(&id).copied().unwrap_or(0.0)
    }

    /// How near he is in your ground: your level with his coach, 0-1.
    pub(crate) fn near(&mut self, w: &World, hs: usize) -> f64 {
        [0.0, 0.4, 0.7, 1.0][self.levels.school(w, hs) as usize]
    }
}

/// Each day for your seat: the news reaching you, Monday's look at what
/// happened to the kids you follow, and your finds.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(prog) = seat(w) else { return };
    if d.weekday() == crate::calendar::Weekday::Mon {
        news::week(w, prog, d);
    }
    news::deliver(w, prog, d);
    super::finds::day(w, prog, d);
}
