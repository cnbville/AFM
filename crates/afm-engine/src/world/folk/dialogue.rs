//! Talks off the game day (dialogue D7-D9, D11-D13, lane 14): a home
//! visit's talk with a recruit and the family member who decides (D7); a
//! combine interview as a timed talk (D8); the high school coach taking a
//! college staff's call about one of his players (D9); your own job
//! interviews (D11); the weekly staff meeting where a coordinator argues
//! for a change (D12); the players-only meeting a losing streak brings
//! (D13); agents in their own voice in contract talks (D10). What each
//! does comes from the people in it; the AI's head
//! coaches make the same home visits and their captains call the same
//! meetings. Sizes are Claude's picks (proposed, not confirmed).

use super::Team;
use super::cast::{self, Role as CastRole};
use super::claims::rooms;
use super::scene::{self, Kind, Pick, Scene, Who};
use super::voice::{Facts, Pen, Speaker};
use crate::calendar::{Date, Event, Kind as CalKind, Level, Loudness, Weekday};
use crate::player::{PlayingTime, Trait};
use crate::world::coach::{Employer, Role};
use crate::world::program::GradeCat;
use crate::world::{Stage, World};

/// What these talks keep between days.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct State {
    /// Your word to a college staff about one of your players (D9).
    pub calls: Vec<CallWord>,
    /// Your job interviews: (employer, role, how it went, the day).
    pub interviews: Vec<(Employer, Role, f32, Date)>,
    /// The last staff meeting's case, checked after the next game (D12).
    pub case: Option<Case>,
    /// Players-only meetings: (team, day), the last per team kept.
    pub players_only: Vec<(Team, Date)>,
    /// The last week a college staff called you (D9).
    pub called: Option<Date>,
    /// The last game a staff meeting followed (D12).
    pub met: Option<Date>,
}

/// What you told a college staff about a player (D9).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CallWord {
    pub id: u32,
    pub program: usize,
    pub date: Date,
    /// 0 sold him, 1 straight, 2 protected him.
    pub word: u8,
    /// What the staff asked about that was a real worry: 0 none, 1 his
    /// grades, 2 his character.
    pub concern: u8,
    pub settled: bool,
}

/// A coordinator's case at the staff meeting (D12).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Case {
    pub coach: usize,
    /// 0 more pressure, 1 more zone, 2 run more, 3 throw more.
    pub kind: u8,
    /// 0 agreed, 1 halfway, 2 overruled.
    pub said: u8,
    pub date: Date,
    /// The unit's points (scored for the offense, allowed for the defense)
    /// in the game before.
    pub before: i32,
    /// Your own case instead of his (live talks LT17): the move you made
    /// (`MOVES`), argued against his numbers. `said` is 2 then: you
    /// overruled his case.
    #[serde(default)]
    pub mine: Option<u8>,
}

fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

fn z(v: f64) -> f64 {
    (v - 10.0) / 10.0
}

fn line(w: &mut World, s: &Scene, k: u64, key: &str, who: &Speaker, f: &Facts) -> String {
    let mut pen = scene::pen(w, s, k);
    // A speaker no tagged version fits says one of the plain ones.
    let t = pen
        .part(key, who, f)
        .or_else(|| pen.part(key, &Speaker { key: who.key, tags: Vec::new() }, f))
        .map(|x| super::voice::tidy(&x))
        .unwrap_or_default();
    pen.put(w);
    t
}

// ---------------------------------------------------------------------------
// D7: the home visit's talk.

/// What a visit's pitch is about.
pub const PITCHES: [(&str, &str); 7] = [
    ("playing", "Playing time"),
    ("nil", "NIL money"),
    ("school", "The school"),
    ("distance", "Close to home"),
    ("family", "Family"),
    ("development", "Getting him to the next level"),
    ("winning", "Winning"),
];

/// The family's weight on each pitch, in shares of seven (a pitch they
/// care about as much as the rest is 1.0): their decider's wants, and how
/// much a parent or grandparent who decides wants to trust the man taking
/// their son (more where one parent or the grandparents raised him).
fn wants(p: &crate::world::Person) -> [f64; 7] {
    use crate::world::background::{Decider, FamilyStructure};
    let x = p.family.decider_wants;
    let v = [x.playing_time, x.money, x.academics, x.close_to_home, x.development, x.prestige];
    let s: f64 = v.iter().sum::<f64>().max(1e-9);
    let v = v.map(|a| 6.0 * a / s);
    let family = match p.family.decider {
        Decider::Himself => 0.5,
        Decider::Handler => 0.7,
        Decider::Parent | Decider::Grandparent => 1.0,
    } + match p.family.structure {
        FamilyStructure::SingleParent | FamilyStructure::Grandparents => 0.2,
        _ => 0.0,
    };
    [v[0], v[1], v[2], v[3], family, v[4], v[5]]
}

/// What the school can back each pitch up with, 0-1. The family pitch is
/// the head coach himself: how long he has been there, and how he is with
/// people.
fn backing(w: &World, i: usize, j: usize) -> [f64; 7] {
    // What the school really offers him: yours only what you promised.
    let nil = crate::world::recruiting::offer_money(w, j, i);
    let floor = 0.6 * w.programs[j].grades[GradeCat::Nil as usize] / 100.0;
    backing_with(w, i, j, (nil / 1.0e6).powf(0.6).min(1.0).max(floor))
}

/// `backing` with the money pitch's own backing (0-1) given.
fn backing_with(w: &World, i: usize, j: usize, money: f64) -> [f64; 7] {
    let p = &w.people[i];
    let prog = &w.programs[j];
    let g = |c: GradeCat| prog.grades[c as usize] / 100.0;
    let near = crate::world::recruiting::closeness(crate::world::recruiting::home_miles(&w.high_schools, p, prog));
    let family = prog.head_coach.map_or(0.3, |c| {
        let years = w.coaches[c].job.map_or(0, |jb| (w.year - jb.since).max(0)) as f64;
        (0.25 + 0.06 * years.min(8.0) + 0.02 * (w.coaches[c].ability().relationships - 10.0)).clamp(0.1, 1.0)
    });
    [
        g(GradeCat::PlayingTime),
        money,
        g(GradeCat::Academics),
        near,
        family,
        (g(GradeCat::ProPipeline) + g(GradeCat::CoachPrestige)) / 2.0,
        (g(GradeCat::TitleChances) + g(GradeCat::BrandExposure)) / 2.0,
    ]
}

/// How a pitch lands with a family, in appeal points (a visit's contact is
/// 0.004, an official visit 0.02): what they care about, what the school
/// can back up, and how well the man in the room reads and talks to
/// people. Overselling a weak point costs a little; listening first helps.
pub(crate) fn home_lean(w: &World, i: usize, j: usize, pitch: usize, coach: Option<usize>, listened: bool) -> f32 {
    let p = &w.people[i];
    pitch_lean(w, wants(p)[pitch], backing(w, i, j)[pitch], coach, listened)
}

/// A pitch's pull from what he weighs (`fit`, 1.0 as much as the rest) and
/// what the school can back up (`back`, 0-1), read by the man making it.
fn pitch_lean(w: &World, fit: f64, back: f64, coach: Option<usize>, listened: bool) -> f32 {
    let a = coach.map(|c| *w.coaches[c].ability());
    let skill =
        (0.6 + 0.04 * a.map_or(0.0, |a| a.recruiting - 10.0) + 0.03 * a.map_or(0.0, |a| a.relationships - 10.0))
            .clamp(0.3, 1.5);
    let mut lean = 0.012 * skill * (fit * (0.3 + 0.7 * back) - 0.6);
    if back < 0.3 && fit > 1.0 {
        lean -= 0.006;
    }
    if listened {
        lean += 0.003;
    }
    lean.clamp(-0.012, 0.03) as f32
}

/// A head coach's own pick in a living room: what he reads they want and
/// the school can sell, read better the better he recruits.
fn coach_pitch(w: &World, i: usize, j: usize, coach: Option<usize>, seed: u64) -> usize {
    let wt = wants(&w.people[i]);
    let back = backing(w, i, j);
    let r = coach.map_or(10.0, |c| w.coaches[c].ability().recruiting);
    let sd = 0.6 * (1.6 - 0.06 * r).max(0.2);
    (0..PITCHES.len())
        .max_by(|&a, &b| {
            let s = |k: usize| wt[k] * (0.3 + 0.7 * back[k]) + sd * crate::game::talk::normal(seed, k as u64);
            s(a).total_cmp(&s(b))
        })
        .unwrap_or(0)
}

/// What the home visits' talks add to a school's pull on a recruit.
pub(crate) fn home_talk(p: &crate::world::Person, j: usize) -> f64 {
    p.cal.home_talks.iter().filter(|x| x.0 == j).map(|x| x.1 as f64).sum()
}

fn keep_lean(w: &mut World, i: usize, j: usize, lean: f32) {
    let v = &mut w.people[i].cal.home_talks;
    v.retain(|x| x.0 != j);
    v.push((j, lean));
}

/// The AI's head coach in a living room (contact periods): the same pitch
/// read the same way.
pub(crate) fn ai_home_visit(w: &mut World, i: usize, j: usize, d: Date) {
    let hc = w.programs[j].head_coach;
    let seed = w.options.seed as u64 ^ (d.0 as u64) << 20 ^ (j as u64) << 40 ^ w.people[i].player.id as u64;
    let k = coach_pitch(w, i, j, hc, seed);
    let lean = home_lean(w, i, j, k, hc, false);
    keep_lean(w, i, j, lean);
}

/// Your home visit is set: its talk waits on the day.
pub(crate) fn home_visit_set(w: &mut World, id: u32, j: usize, d: Date) {
    let Some(i) = super::person(w, id) else { return };
    let name = w.people[i].name.clone();
    let mut s = Scene::new(Kind::HomeVisit, d, format!("Home visit: {name}"));
    s.place = format!("The {} family's living room, {}", last_name(&name), w.high_schools[w.people[i].hs].town.clone());
    s.a = id as u64;
    s.b = j as u64;
    s.who = vec![Who::Player(id), Who::Group(decider_words(&w.people[i]).to_string())];
    scene::add(w, s);
}

/// A visit called off takes its talk with it.
pub(crate) fn home_visit_off(w: &mut World, id: u32, why: &str) {
    for s in w.folk.scenes.iter_mut().filter(|s| s.kind == Kind::HomeVisit && s.a == id as u64 && !s.done) {
        s.outcome.push(why.to_string());
        s.handed = Some("nobody".into());
        s.end();
    }
}

fn last_name(n: &str) -> &str {
    n.split_whitespace().last().unwrap_or(n)
}

fn decider_words(p: &crate::world::Person) -> &'static str {
    use crate::world::background::{Decider, FamilyStructure};
    match p.family.decider {
        Decider::Himself => "He decides himself",
        Decider::Grandparent => "His grandmother",
        Decider::Handler => "His trainer",
        Decider::Parent => match p.family.structure {
            FamilyStructure::SingleParent => "His mother",
            FamilyStructure::Grandparents => "His grandmother",
            FamilyStructure::Foster => "His guardian",
            FamilyStructure::TwoParents => "His parents",
        },
    }
}

fn decider_tag(p: &crate::world::Person) -> &'static str {
    use crate::world::background::Decider;
    match p.family.decider {
        Decider::Himself => "himself",
        Decider::Grandparent => "grandparent",
        Decider::Handler => "handler",
        Decider::Parent => "parent",
    }
}

fn top_wants(p: &crate::world::Person) -> (usize, usize) {
    let wt = wants(p);
    let mut k: Vec<usize> = (0..PITCHES.len()).collect();
    k.sort_by(|&a, &b| wt[b].total_cmp(&wt[a]));
    (k[0], k[1])
}

fn visit_open(w: &mut World, s: &mut Scene) {
    let Some(i) = super::person(w, s.a as u32) else { return s.end() };
    let p = &w.people[i];
    let kid = super::player_link(w, i);
    let who = decider_words(p);
    let (top, _) = top_wants(p);
    let mut sp = Speaker::parent(w, i);
    sp.tags.push(decider_tag(p));
    let f = Facts::new().with("name", p.name.split_whitespace().next().unwrap_or("").to_string());
    let ask = line(w, s, 1, &format!("homevisit.ask.{}", PITCHES[top].0), &sp, &f);
    let room = if who == "He decides himself" {
        format!("{kid} and his family sit across from you. He does the talking.")
    } else {
        format!("{kid} sits beside {}. {} does the talking.", who.to_lowercase(), who)
    };
    s.narrate(room);
    let name = if who == "He decides himself" { "Him".to_string() } else { who.to_string() };
    s.them(&name, ask, None);
    pitch_picks(s);
    s.pick(
        709,
        "\"Before I talk, tell me what matters most to you.\"",
        "Listen",
        "warm",
        "They tell you what they weigh.",
    );
}

fn pitch_picks(s: &mut Scene) {
    let say = [
        "\"He plays early here. Look at the depth chart.\"",
        "\"Let's talk about what our collective does for him.\"",
        "\"He'll leave here with a degree that means something.\"",
        "\"You'll be at every home game. It's a short drive.\"",
        "\"I'll look after him like he's my own. Ask my players' parents.\"",
        "\"We've sent his position to the league. Here's how.\"",
        "\"He'll play for championships here.\"",
    ];
    for (k, t) in say.iter().enumerate() {
        s.pick(701 + k as u32, *t, PITCHES[k].1, "warm", "");
    }
}

fn visit_answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let id = s.a as u32;
    let j = s.b as usize;
    let Some(i) = super::person(w, id) else { return s.end() };
    let p = &w.people[i];
    let who = decider_words(p);
    let name = if who == "He decides himself" { "Him".to_string() } else { who.to_string() };
    let mut sp = Speaker::parent(w, i);
    sp.tags.push(decider_tag(p));
    if code == 709 {
        let (a, b) = top_wants(p);
        let f = Facts::new().with("first", PITCHES[a].1.to_lowercase()).with("second", PITCHES[b].1.to_lowercase());
        let t = line(w, s, 2, "homevisit.tell", &sp, &f);
        s.you(&scene::your_name(w), "Before I talk, tell me what matters most to you.");
        s.them(&name, t, Some(format!("{} first, then {}.", PITCHES[a].1, PITCHES[b].1.to_lowercase())));
        s.c = 1;
        pitch_picks(s);
        return;
    }
    let k = (code.saturating_sub(701) as usize).min(PITCHES.len() - 1);
    s.you(&scene::your_name(w), picked.label.trim_matches('"'));
    let lean = home_lean(w, i, j, k, you(w), s.c == 1);
    keep_lean(w, i, j, lean);
    let mood = if lean > 0.012 {
        "good"
    } else if lean > 0.0 {
        "flat"
    } else {
        "bad"
    };
    let react = line(w, s, 3, &format!("homevisit.react.{mood}"), &sp, &Facts::new());
    s.them(&name, react, None);
    s.outcome.push(
        match mood {
            "good" => "They lean your way a little. The pitch was what they came for.",
            "flat" => "Polite. It landed, but it isn't what they weigh most.",
            _ => "It cooled the room: they don't believe you can back it up, or it isn't what they care about.",
        }
        .to_string(),
    );
    s.end();
}

fn visit_hand(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let j = s.b as usize;
    let Some(i) = super::person(w, id) else { return };
    // Your recruiting man goes: the position coach for his spot.
    let group = crate::world::coach::Group::of(w.people[i].player.position);
    let c = w.programs[j].position_coaches[group as usize].or(w.programs[j].head_coach);
    let k = coach_pitch(w, i, j, c, s.id as u64 ^ 0x7070);
    let lean = home_lean(w, i, j, k, c, false);
    keep_lean(w, i, j, lean);
    s.handed = Some(c.map_or("your staff".to_string(), |c| w.coaches[c].name.clone()));
    s.outcome.push(format!("He sold {}.", PITCHES[k].1.to_lowercase()));
}

// ---------------------------------------------------------------------------
// D7b: a transfer's call, home visit and campus talk (portal recruiting,
// Cain 2026-11-07: "there is no reason why I shouldn't have the same
// options with a portal player that I have with an HS player"). The same
// pitches read the same way as a living room's, on the transfer's own
// wants (in the portal he decides for himself) and on what the program
// would really give him: the role he'd have there and the money. Every
// program that wants him calls (real practice: within hours of his name
// going into the database), and his finalists host him on campus; yours
// are the calls and visits you make. Sizes are Claude's picks (proposed,
// not confirmed).

/// Where a transfer talk happens (a scene's `f[1]`).
pub(crate) const ON_PHONE: f32 = 0.0;
pub(crate) const AT_HOME: f32 = 1.0;
pub(crate) const ON_CAMPUS: f32 = 2.0;

/// A phone call lands half as hard as a sit-down (proposed, not confirmed).
pub(crate) const PHONE: f64 = 0.5;

/// A promise's pull at its fullest, in appeal points: a starting job, a
/// real role (proposed, not confirmed: a starting job about the size of
/// a sold official visit and a good living room together, as real
/// transfers, quarterbacks above all, choose on the job they're promised).
const PROMISE_PULL: [f64; 2] = [0.06, 0.035];

/// A transfer's weight on each pitch, as `wants` reads a family's: his own
/// wants, and the man he'd play for, more to one who left because his
/// coach did.
fn transfer_wants(p: &crate::world::Person) -> [f64; 7] {
    let x = p.family.own_wants;
    let v = [x.playing_time, x.money, x.academics, x.close_to_home, x.development, x.prestige];
    let s: f64 = v.iter().sum::<f64>().max(1e-9);
    let v = v.map(|a| 6.0 * a / s);
    let coach_left = p.cal.portal.is_some_and(|e| e.why == "coaching change");
    let coach = 0.5 + if coach_left { 0.3 } else { 0.0 };
    [v[0], v[1], v[2], v[3], coach, v[4], v[5]]
}

/// What the role he'd have backs a playing-time pitch with.
fn role_backing(role: PlayingTime) -> f64 {
    match role {
        PlayingTime::Starter => 0.95,
        PlayingTime::Rotation => 0.6,
        _ => 0.25,
    }
}

/// What a program can back each pitch up with for a transfer: the role
/// he'd have there, the money it would pay him, the rest as for a recruit.
fn transfer_backing(w: &World, i: usize, j: usize, role: PlayingTime, nil: f64) -> [f64; 7] {
    let mut b = backing_with(w, i, j, (nil.max(0.0) / 1.0e6).powf(0.6).min(1.0));
    b[0] = role_backing(role);
    b
}

/// How a pitch lands with a transfer, in appeal points (`pitch_lean`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn transfer_lean(
    w: &World,
    i: usize,
    j: usize,
    pitch: usize,
    coach: Option<usize>,
    listened: bool,
    role: PlayingTime,
    nil: f64,
) -> f32 {
    let fit = transfer_wants(&w.people[i])[pitch];
    let back = transfer_backing(w, i, j, role, nil)[pitch];
    pitch_lean(w, fit, back, coach, listened)
}

/// A staff's own pick for a transfer, as `coach_pitch` picks for a family.
#[allow(clippy::too_many_arguments)]
fn transfer_pitch(
    w: &World,
    i: usize,
    j: usize,
    coach: Option<usize>,
    role: PlayingTime,
    nil: f64,
    seed: u64,
) -> usize {
    let wt = transfer_wants(&w.people[i]);
    let back = transfer_backing(w, i, j, role, nil);
    let r = coach.map_or(10.0, |c| w.coaches[c].ability().recruiting);
    let sd = 0.6 * (1.6 - 0.06 * r).max(0.2);
    (0..PITCHES.len())
        .max_by(|&a, &b| {
            let s = |k: usize| wt[k] * (0.3 + 0.7 * back[k]) + sd * crate::game::talk::normal(seed, k as u64);
            s(a).total_cmp(&s(b))
        })
        .unwrap_or(0)
}

/// The man who calls a transfer for a staff: his position coach, else the
/// head coach.
fn caller(w: &World, i: usize, j: usize) -> Option<usize> {
    let group = crate::world::coach::Group::of(w.people[i].player.position);
    w.programs[j].position_coaches[group as usize].or(w.programs[j].head_coach)
}

fn talk_seed(w: &World, i: usize, j: usize, salt: u64) -> u64 {
    w.options.seed as u64 ^ (w.people[i].player.id as u64) << 20 ^ (j as u64) << 44 ^ salt
}

/// A staff's call to a transfer it wants: every AI staff's, and yours when
/// your staff works the portal (a phone call, `PHONE`).
pub(crate) fn staff_call(w: &World, i: usize, j: usize, role: PlayingTime, nil: f64) -> f64 {
    let c = caller(w, i, j);
    let k = transfer_pitch(w, i, j, c, role, nil, talk_seed(w, i, j, 0xCA11));
    PHONE * transfer_lean(w, i, j, k, c, false, role, nil) as f64
}

/// A finalist's head coach with him at the end of his campus visit.
pub(crate) fn staff_campus(w: &World, i: usize, j: usize, role: PlayingTime, nil: f64) -> f64 {
    let c = w.programs[j].head_coach;
    let k = transfer_pitch(w, i, j, c, role, nil, talk_seed(w, i, j, 0xCA5E));
    transfer_lean(w, i, j, k, c, false, role, nil) as f64
}

/// Your promise of a role to a transfer (0 a starting job, 1 a real role):
/// its pull at his decision and how far he believes it, 0-1. It pulls by
/// what playing time means to him and what it adds to the role he'd have
/// with you anyway; he believes it by your name for keeping your word
/// (straight with players), how you are with people, and, for a starting
/// job, how close his play is to the starter's he'd have to beat (`bar`,
/// both read at your level).
pub(crate) fn promise_pull(
    w: &World,
    i: usize,
    kind: u8,
    role: PlayingTime,
    perf: f64,
    bar: Option<f64>,
) -> (f64, f64) {
    let pt = transfer_wants(&w.people[i])[0];
    let k = (kind as usize).min(1);
    let adds = match (k, role) {
        (0, PlayingTime::Starter) => 0.15,
        (0, PlayingTime::Rotation) => 0.6,
        (0, _) => 1.0,
        (_, PlayingTime::Starter) => 0.0,
        (_, PlayingTime::Rotation) => 0.2,
        _ => 0.6,
    };
    let rel = you(w).map_or(10.0, |c| w.coaches[c].ability().relationships);
    let name = (0.55 + 0.3 * w.folk.mine.straight as f64 + 0.02 * (rel - 10.0)).clamp(0.2, 1.0);
    let plausible = match bar {
        Some(b) if k == 0 => (1.0 - 0.06 * (b - perf).max(0.0)).clamp(0.3, 1.0),
        _ => 1.0,
    };
    let belief = name * plausible;
    (PROMISE_PULL[k] * pt * adds * belief, belief)
}

/// Your talk with a transfer is set: a call now, a sit-down at his home or
/// at the end of his campus visit on its day.
pub(crate) fn transfer_talk(w: &mut World, id: u32, j: usize, d: Date, at: f32) -> u32 {
    let Some(i) = super::person(w, id) else { return 0 };
    let name = w.people[i].name.clone();
    let from = w.people[i].college.map(|c| w.program_name(c).to_string()).unwrap_or_default();
    let (kind, title, place) = if at == ON_CAMPUS {
        (
            Kind::PortalVisit,
            format!("Official visit: {name}"),
            format!("Your office, the end of {}'s visit", first_of(&name)),
        )
    } else if at == AT_HOME {
        (
            Kind::PortalVisit,
            format!("Home visit: {name}"),
            format!("The {} family's living room, {}", last_name(&name), w.high_schools[w.people[i].hs].town.clone()),
        )
    } else {
        (Kind::PortalCall, format!("A call to {name}"), format!("On the phone with {} of {from}", first_of(&name)))
    };
    let mut s = Scene::new(kind, d, title);
    s.place = place;
    s.a = id as u64;
    s.b = j as u64;
    s.f[1] = at;
    s.stops = at != ON_PHONE;
    s.who = vec![Who::Player(id)];
    scene::add(w, s)
}

/// A transfer's visit called off takes its talk with it (the talk at
/// that place on that day).
pub(crate) fn transfer_visit_off(w: &mut World, id: u32, d: Date, at: f32, why: &str) {
    for s in w
        .folk
        .scenes
        .iter_mut()
        .filter(|s| s.kind == Kind::PortalVisit && s.a == id as u64 && s.date == d && s.f[1] == at && !s.done)
    {
        s.outcome.push(why.to_string());
        s.handed = Some("nobody".into());
        s.end();
    }
}

fn first_of(n: &str) -> &str {
    n.split_whitespace().next().unwrap_or(n)
}

/// What a transfer weighs most, then second (`PITCHES` indexes): what your
/// staff hears once it has talked to him.
pub fn transfer_weighs(w: &World, i: usize) -> (usize, usize) {
    transfer_top(&w.people[i])
}

fn transfer_top(p: &crate::world::Person) -> (usize, usize) {
    let wt = transfer_wants(p);
    let mut k: Vec<usize> = (0..PITCHES.len()).collect();
    k.sort_by(|&a, &b| wt[b].total_cmp(&wt[a]));
    (k[0], k[1])
}

fn transfer_open(w: &mut World, s: &mut Scene) {
    let Some(i) = super::person(w, s.a as u32) else { return s.end() };
    let p = &w.people[i];
    if p.cal.portal.is_none() {
        s.outcome.push("He has made his choice: there is nothing left to talk about.".into());
        return s.end();
    }
    let kid = super::player_link(w, i);
    let (top, _) = transfer_top(p);
    let sp = Speaker::player(w, i);
    let f = Facts::new().with("name", first_of(&p.name).to_string());
    let ask = line(w, s, 1, &format!("portal.ask.{}", PITCHES[top].0), &sp, &f);
    s.narrate(if s.f[1] == ON_CAMPUS {
        format!(
            "{kid} sits across your desk at the end of his visit. He has seen the place; now he wants straight answers."
        )
    } else if s.f[1] == AT_HOME {
        format!("{kid} sits across from you in his family's living room. He does the talking.")
    } else {
        format!("{kid} picks up. Every staff with a place for him has called today.")
    });
    s.them("Him", ask, None);
    transfer_picks(s);
    s.pick(
        709,
        "\"Before I sell you anything, tell me what you're looking for.\"",
        "Listen",
        "warm",
        "He tells you what he weighs.",
    );
}

/// Your pitches to a transfer, said to him (the same seven a family hears).
fn transfer_picks(s: &mut Scene) {
    let say = [
        "\"You'll play here. Look at who's ahead of you.\"",
        "\"Let's talk about what we can pay you.\"",
        "\"Your credits come with you. You'll finish your degree here.\"",
        "\"Your family can be at every game. It's a short drive.\"",
        "\"You'll have a staff that has your back. Ask my players.\"",
        "\"We'll get you ready for the draft. Here's the plan.\"",
        "\"You'll play for something here.\"",
    ];
    for (k, t) in say.iter().enumerate() {
        s.pick(701 + k as u32, *t, PITCHES[k].1, "warm", "");
    }
}

fn promise_picks(s: &mut Scene) {
    s.pick(
        711,
        "\"You'll start for us. You have my word.\"",
        "Promise a starting job",
        "firm",
        "Checked after his first game with you.",
    );
    s.pick(
        712,
        "\"You'll have a real role, every week.\"",
        "Promise a real role",
        "warm",
        "Checked at the season's end.",
    );
    s.pick(719, "\"Think it over. Call me anytime.\"", "Leave it there", "calm", "");
}

fn transfer_answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let id = s.a as u32;
    let j = s.b as usize;
    let Some(i) = super::person(w, id) else { return s.end() };
    // A talk left open past his decision has nothing left to settle.
    if w.people[i].cal.portal.is_none() {
        s.outcome.push("He has made his choice: there is nothing left to talk about.".into());
        return s.end();
    }
    let sp = Speaker::player(w, i);
    let me = scene::your_name(w);
    match code {
        709 => {
            let (a, b) = transfer_top(&w.people[i]);
            let f = Facts::new().with("first", PITCHES[a].1.to_lowercase()).with("second", PITCHES[b].1.to_lowercase());
            let t = line(w, s, 2, "portal.tell", &sp, &f);
            s.you(&me, "Before I sell you anything, tell me what you're looking for.");
            s.them("Him", t, Some(format!("{} first, then {}.", PITCHES[a].1, PITCHES[b].1.to_lowercase())));
            s.c = 1;
            s.picks.clear();
            transfer_picks(s);
        }
        711 | 712 => {
            let kind = (code - 711) as u8;
            s.you(&me, picked.label.trim_matches('"'));
            let Some(read) = crate::world::front_office::portal::promise(w, id, kind) else {
                s.outcome
                    .push("Your staff has no place for him to promise: the roster is full, or he can't come.".into());
                return finish_transfer(w, s, id);
            };
            let key = match read {
                (_, _, PlayingTime::Starter) if kind == 0 => "knows",
                (_, b, _) if b >= 0.6 => "believes",
                _ => "doubts",
            };
            let t = line(w, s, 4, &format!("portal.promise.{key}"), &sp, &Facts::new());
            s.them("Him", t, None);
            s.outcome.push(
                match key {
                    "knows" => "He knows he'd start anyway: the promise adds little, and it binds you.",
                    "believes" => "He believes you. It's the promise he wanted to hear.",
                    _ => "He's heard promises before. He isn't sure he believes yours.",
                }
                .to_string(),
            );
            s.outcome
                .push(format!("You promised {}.", if kind == 0 { "him a starting job" } else { "him a real role" }));
            finish_transfer(w, s, id);
        }
        719 => {
            s.you(&me, picked.label.trim_matches('"'));
            finish_transfer(w, s, id);
        }
        _ => {
            let k = (code.saturating_sub(701) as usize).min(PITCHES.len() - 1);
            s.you(&me, picked.label.trim_matches('"'));
            let (role, nil) = crate::world::front_office::portal::place(w, i, j);
            let mut lean = transfer_lean(w, i, j, k, you(w), s.c == 1, role, nil);
            if s.f[1] == ON_PHONE {
                lean *= PHONE as f32;
            }
            crate::world::front_office::portal::talked(w, id, s.f[1], lean, PITCHES[k].1);
            let mood = if lean > 0.006 {
                "good"
            } else if lean > 0.0 {
                "flat"
            } else {
                "bad"
            };
            let react = line(w, s, 3, &format!("portal.react.{mood}"), &sp, &Facts::new());
            s.them("Him", react, None);
            s.outcome.push(
                match mood {
                    "good" => "He leans your way a little. It was what he wanted to hear.",
                    "flat" => "It landed, but it isn't what he weighs most.",
                    _ => "It cooled him: he doesn't believe you can back it up, or it isn't what he cares about.",
                }
                .to_string(),
            );
            s.c = 2;
            s.picks.clear();
            if crate::world::front_office::portal::promised(w, id).is_none() {
                promise_picks(s);
            } else {
                finish_transfer(w, s, id);
            }
        }
    }
}

fn finish_transfer(w: &mut World, s: &mut Scene, id: u32) {
    s.end();
    if let Some(t) = crate::world::front_office::portal::after_talk(w, id) {
        s.outcome.push(t);
    }
}

fn transfer_hand(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let j = s.b as usize;
    let Some(i) = super::person(w, id) else { return };
    // You made your pitch and left before a promise: it stands, and
    // nobody promises for you.
    if s.c == 2 {
        s.handed = Some("nobody".into());
        s.outcome.push("Your pitch stands. Nobody made him a promise for you.".into());
        if let Some(t) = crate::world::front_office::portal::after_talk(w, id) {
            s.outcome.push(t);
        }
        return;
    }
    // Your position coach for his spot makes the call or the pitch.
    let c = caller(w, i, j);
    let (role, nil) = crate::world::front_office::portal::place(w, i, j);
    let k = transfer_pitch(w, i, j, c, role, nil, s.id as u64 ^ 0x7071);
    let mut lean = transfer_lean(w, i, j, k, c, false, role, nil);
    if s.f[1] == ON_PHONE {
        lean *= PHONE as f32;
    }
    crate::world::front_office::portal::talked(w, id, s.f[1], lean, PITCHES[k].1);
    s.handed = Some(c.map_or("your staff".to_string(), |c| w.coaches[c].name.clone()));
    s.outcome.push(format!("He sold {}.", PITCHES[k].1.to_lowercase()));
    if let Some(t) = crate::world::front_office::portal::after_talk(w, id) {
        s.outcome.push(t);
    }
}

// ---------------------------------------------------------------------------
// D9: a college staff calls the high school coach.

/// Mondays in the recruiting year: now and then a college staff that wants
/// one of your players calls you about him.
pub(crate) fn college_calls(w: &mut World, d: Date) {
    let Some(h) = crate::world::scouting::hs::school(w) else { return };
    if d.weekday() != Weekday::Wed {
        return;
    }
    let period = crate::calendar::recruiting_period(&w.dates, d);
    if matches!(period, crate::calendar::Period::Dead) {
        return;
    }
    if w.folk.dialogue.called.is_some_and(|c| c.days_to(d) < 13) {
        return;
    }
    let sign = w.dates.season + 1;
    let mut cands: Vec<(usize, usize)> = Vec::new();
    for &i in &w.high_schools[h].roster {
        let p = &w.people[i];
        if p.stage != Stage::HighSchool || p.cal.signed.is_some() || p.class_of > sign + 1 {
            continue;
        }
        if let Some(&j) = p.cal.offers.iter().find(|&&j| super::roll(w, 0xD9, (j as u64) << 32 ^ d.0 as u64) < 0.5) {
            cands.push((i, j));
        }
    }
    if cands.is_empty() || super::roll(w, 0xD9A, d.0 as u64) > 0.6 {
        return;
    }
    let (i, j) = cands[(super::roll(w, 0xD9B, d.0 as u64) * cands.len() as f64) as usize % cands.len()];
    w.folk.dialogue.called = Some(d);
    let name = w.people[i].name.clone();
    let mut s = Scene::new(Kind::CollegeCall, d, format!("{} calls about {name}", w.program_name(j)));
    s.stops = false;
    s.place = "Your office, between classes".into();
    s.a = w.people[i].player.id as u64;
    s.b = j as u64;
    s.c = concern(&w.people[i]) as i64;
    s.who = vec![Who::Player(w.people[i].player.id)];
    scene::add(w, s);
}

/// A real worry a college staff would ask about: 1 his grades, 2 his
/// character, 0 none.
fn concern(p: &crate::world::Person) -> u8 {
    let trouble = p.incident
        || p.player.history.iter().any(|h| {
            h.events.has(crate::player::DevelopmentEvent::Suspended)
                || h.events.has(crate::player::DevelopmentEvent::Dismissed)
        });
    if p.bg.gpa < 2.5 {
        1
    } else if trouble || p.player.personality.traits.get(Trait::Professionalism) <= 6.0 {
        2
    } else {
        0
    }
}

fn call_open(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let j = s.b as usize;
    let Some(i) = super::person(w, id) else { return s.end() };
    let kid = super::player_link(w, i);
    let school = w.program_name(j).to_string();
    let caller = w.programs[j].position_coaches[crate::world::coach::Group::of(w.people[i].player.position) as usize]
        .or(w.programs[j].head_coach);
    let (cname, sp) = match caller {
        Some(c) => (w.coaches[c].name.clone(), Speaker::coach(w, c)),
        None => (
            format!("{school}'s recruiting coordinator"),
            Speaker { key: 0xD9C0 ^ j as u64, tags: vec!["coach", "college"] },
        ),
    };
    if let Some(c) = caller {
        s.who.push(Who::Coach(c));
    }
    let about = ["football", "grades", "character"][s.c.clamp(0, 2) as usize];
    let f = Facts::new().with("name", w.people[i].name.split_whitespace().next().unwrap_or("").to_string());
    let ask = line(w, s, 1, &format!("hscall.ask.{about}"), &sp, &f);
    s.narrate(format!("{cname} from {school} is on the line about {kid}."));
    s.them(&cname, ask, None);
    let worry = s.c != 0;
    s.pick(
        901,
        "\"He's the best player I've coached. Take him.\"",
        "Sell him",
        "warm",
        "More staffs look at his film. If he busts, your word costs you with them.",
    );
    s.pick(
        902,
        if worry {
            "\"I'll be straight with you. Here's what you need to know.\""
        } else {
            "\"Here's who he is, the good and the work he has left.\""
        },
        "Be straight",
        "calm",
        "Your word holds its value with them, whatever he becomes.",
    );
    s.pick(
        903,
        if worry {
            "\"There's nothing there. He's a good kid.\""
        } else {
            "\"He's a good player. You've seen the film.\""
        },
        if worry { "Protect him" } else { "Keep it short" },
        "firm",
        if worry {
            "It may help him now. If it comes out later, they won't trust you again."
        } else {
            "Says little either way."
        },
    );
}

fn call_answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let id = s.a as u32;
    let j = s.b as usize;
    let word = (code.saturating_sub(901)).min(2) as u8;
    let concern = s.c.clamp(0, 2) as u8;
    // Selling and a straight word both reach his recruitment the way a
    // call you made would (scouting/hs.rs promos).
    let strength = match word {
        0 => 2,
        1 if concern == 0 => 1,
        1 => 0,
        _ => 1,
    };
    w.scouting.hs.promoted.push(crate::world::scouting::hs::Promo {
        id,
        program: Some(j),
        date: w.today,
        strength,
        settled: None,
        warmth: 0.0,
    });
    w.folk.dialogue.calls.push(CallWord { id, program: j, date: w.today, word, concern, settled: false });
    w.folk.mine.straight = (w.folk.mine.straight
        + if word == 1 {
            0.02
        } else if word == 2 && concern > 0 {
            -0.03
        } else {
            0.0
        })
    .clamp(-1.0, 1.0);
    s.you(&scene::your_name(w), picked.label.trim_matches('"'));
    s.outcome.push(
        match (word, concern) {
            (0, _) => "You sold him. They'll look harder at his film.",
            (1, 0) => "You told them who he is. They'll weigh it with the film.",
            (1, _) => "You told them. They'll weigh it, and they'll remember you were straight.",
            (_, 0) => "You kept it short.",
            _ => "You covered for him. If it comes out, they'll remember that too.",
        }
        .to_string(),
    );
    s.end();
}

/// A year after each call: a worry you hid that came true costs you with
/// that staff; one you told them about earns your word some weight,
/// whatever he became.
pub(crate) fn settle_calls(w: &mut World, d: Date) {
    let mut changes: Vec<(usize, f64, usize)> = Vec::new();
    for (k, c) in w.folk.dialogue.calls.iter().enumerate() {
        if c.settled || c.date.add(365) > d {
            continue;
        }
        let Some(i) = super::person(w, c.id) else {
            changes.push((c.program, 0.0, k));
            continue;
        };
        let p = &w.people[i];
        let came_true = match c.concern {
            1 => p.cal.academic_sit || p.cal.spring_ineligible || p.cal.fall_ineligible || p.college.is_none(),
            2 => p.cal.trouble || p.incident,
            _ => false,
        };
        let delta = match (c.word, c.concern > 0, came_true) {
            (1, true, _) => 0.04,
            (2, true, true) => -0.10,
            (0, true, true) => -0.06,
            _ => 0.0,
        };
        changes.push((c.program, delta, k));
    }
    for (prog, delta, k) in changes {
        w.folk.dialogue.calls[k].settled = true;
        if delta != 0.0 {
            let now = crate::world::scouting::hs::credibility(w, prog);
            w.scouting.hs.credibility.insert(prog, (now + delta).clamp(0.05, 0.95) as f32);
        }
    }
    w.folk.dialogue.calls.retain(|c| !c.settled || c.date.add(800) > d);
}

// ---------------------------------------------------------------------------
// D11: your job interviews.

/// You put your name in for a head job whose search decides later: the
/// committee sits you down first.
/// A seat nobody holds sits you down today and fills when the talk is
/// over (`decides` None).
pub(crate) fn interview_set(w: &mut World, e: Employer, r: Role, decides: Option<Date>) -> Option<Date> {
    if r != Role::HeadCoach {
        return None;
    }
    let today = w.today;
    let d = decides.map_or(today, |x| today.add(2).min(x.add(-1)).max(today.add(1)));
    let team = Team::from_employer(e);
    let panel = panel(w, team);
    let mut s = Scene::new(
        Kind::JobInterview,
        d,
        format!("Interview: {}", crate::world::front_office::market::employer_name(w, e)),
    );
    s.team = Some(team);
    s.place = match team {
        Team::Nfl(_) | Team::Other(_) => "The owner's suite".into(),
        Team::College(_) => "A hotel conference room near the airport".into(),
        Team::High(_) => "The principal's office".into(),
    };
    s.a = employer_key(e);
    s.c = decides.is_none() as i64;
    s.who = panel.iter().map(|c| Who::Cast(c.r.token())).collect();
    scene::add(w, s);
    Some(d)
}

fn employer_key(e: Employer) -> u64 {
    match e {
        Employer::College(p) => p as u64,
        Employer::Nfl(t) => 1 << 32 | t as u64,
        Employer::HighSchool(h) => 2 << 32 | h as u64,
        Employer::Other(f) => 3 << 32 | f as u64,
    }
}

fn employer_of(k: u64) -> Employer {
    let i = (k & 0xFFFF_FFFF) as usize;
    match k >> 32 {
        0 => Employer::College(i),
        1 => Employer::Nfl(i),
        2 => Employer::HighSchool(i),
        _ => Employer::Other(i),
    }
}

/// The committee: a high school's principal and athletic director, a
/// college's athletic director and president, an NFL owner and his GM.
fn panel(w: &World, team: Team) -> Vec<cast::Cast> {
    let roles: &[CastRole] = match team {
        Team::High(_) => &[CastRole::Principal, CastRole::SchoolAd],
        Team::College(_) => &[CastRole::AthleticDirector, CastRole::President],
        Team::Nfl(_) | Team::Other(_) => &[CastRole::Owner, CastRole::Gm],
    };
    roles.iter().filter_map(|&r| cast::all(w, team, r).into_iter().next()).collect()
}

/// What a committee wants to hear, from the job and the people on it:
/// the plan (0 win now, 1 build it, 2 develop), the staff (0 bring your
/// own, 1 keep the good ones, 2 the best available) and the past (0 own a
/// failure, 1 point to results, 2 the people you made).
fn wants_of(w: &World, team: Team, panel: &[cast::Cast]) -> [u8; 3] {
    let patience = panel.iter().map(|c| c.patience).sum::<f64>() / panel.len().max(1) as f64;
    let warm = panel.iter().map(|c| c.warmth).sum::<f64>() / panel.len().max(1) as f64;
    let meddle = panel.iter().map(|c| c.meddling).sum::<f64>() / panel.len().max(1) as f64;
    let good = match team {
        Team::College(p) => w.programs[p].prestige_rel >= 0.6,
        Team::Nfl(t) => {
            let (wi, l, _) = w.nfl_record(t);
            wi >= l
        }
        _ => false,
    };
    let plan = if good && patience < 11.0 {
        0
    } else if matches!(team, Team::High(_)) || patience >= 13.0 {
        2
    } else {
        1
    };
    let staff = if meddle >= 13.0 {
        1
    } else if good {
        2
    } else {
        0
    };
    let past = if warm >= 13.0 {
        2
    } else if patience >= 12.0 {
        0
    } else {
        1
    };
    [plan, staff, past]
}

const QUESTIONS: [(&str, [&str; 3]); 3] = [
    (
        "plan",
        [
            "\"We win now. This roster can.\"",
            "\"We build it the right way, year by year.\"",
            "\"We find players and develop them.\"",
        ],
    ),
    (
        "staff",
        [
            "\"I'm bringing my own people.\"",
            "\"I'll keep the good ones you have.\"",
            "\"I'll hire the best coaches I can get.\"",
        ],
    ),
    (
        "past",
        [
            "\"Let me tell you about the year it went wrong.\"",
            "\"Look at what my teams did.\"",
            "\"Look at the people who came through my rooms.\"",
        ],
    ),
];

const LABELS: [[&str; 3]; 3] = [
    ["Win now", "Build it", "Develop"],
    ["Your own staff", "Keep the good ones", "The best available"],
    ["Own a failure", "Point to results", "The people"],
];

/// What you can say to a question, given your record (item 19): the code
/// of each answer, its words, and the label on its button. An answer your
/// record contradicts is not offered: you can't point to results you don't
/// have, own a failure you never had, or name people who never came
/// through your rooms. When none of the three is left, a plain answer
/// stands in (`PLAIN_ANSWER`), so the question is never left without one.
fn interview_answers(q: usize, past: &rooms::Past) -> Vec<(u32, u8, String, String)> {
    let mut out = Vec::new();
    for k in 0..3usize {
        let base = 931 + 3 * q as u32 + k as u32;
        let (text, label) = if q == 2 {
            match k {
                0 => match past.failure() {
                    Some(x) => (
                        format!("\"Let me tell you about {x}. It went wrong, and I know why.\""),
                        LABELS[q][k].to_string(),
                    ),
                    None => continue,
                },
                1 => match past.record_words(true).filter(|_| past.strong()) {
                    Some(r) => (format!("\"Look at what my teams did: {r}.\""), LABELS[q][k].to_string()),
                    None => continue,
                },
                _ => match past.people() {
                    Some(d) => (
                        format!(
                            "\"Look at the people who came through my rooms. {}, a {}-round pick in {}.\"",
                            d.name,
                            super::claims::words::round_word(d.round),
                            d.year
                        ),
                        LABELS[q][k].to_string(),
                    ),
                    None => continue,
                },
            }
        } else {
            (QUESTIONS[q].1[k].to_string(), LABELS[q][k].to_string())
        };
        out.push((base, k as u8, text, label));
    }
    if q == 2 && out.is_empty() {
        // Said as the record allows: a coach with games behind him has a
        // record, only not one of the three kinds.
        let (text, label) = if past.games() == 0 && !past.as_head {
            (PLAIN_TEXT, "No record yet")
        } else {
            (PLAIN_TEXT_RECORD, "Judge the plan")
        };
        out.push((PLAIN_ANSWER, 3, text.to_string(), label.to_string()));
    }
    out
}

/// The answer that stands in when your record backs none of the three
/// answers to "your past" (a first-time head coach with no seasons, no
/// stretch that went wrong and no one drafted, or a record that is neither
/// a winning one nor a losing one over enough games). It lands between a hit and
/// a miss, like an answer one step from what they want. Proposed, not
/// confirmed.
pub const PLAIN_ANSWER: u32 = 940;
const PLAIN_TEXT: &str = "\"I haven't run my own program yet. Judge me on how I'd build one.\"";
const PLAIN_TEXT_RECORD: &str = "\"My record is what it is. Judge me on how I'd build yours.\"";

fn interview_ask(w: &mut World, s: &mut Scene) {
    let q = s.step.min(2) as usize;
    let team = s.team.unwrap_or(Team::from_employer(employer_of(s.a)));
    let panel = panel(w, team);
    let Some(c) = panel.get(q % panel.len().max(1)) else { return s.end() };
    let sp = Speaker::cast(w, c);
    // The numbers behind the case reach the words (item 19): the chair's
    // last season and your own record. The fact part comes first; a part
    // with nothing to say falls back to today's plain question.
    let past = rooms::past(w);
    let mut f = Facts::new();
    rooms::chair(w, team, &mut f);
    past.fill(&mut f);
    let key = QUESTIONS[q].0;
    let mut t = line(w, s, 10 + q as u64, &format!("interview.ask.{key}.facts"), &sp, &f);
    if t.is_empty() {
        t = line(w, s, 10 + q as u64, &format!("interview.ask.{key}"), &sp, &Facts::new());
    }
    s.them(&c.name, t, None);
    for (code, _, text, label) in interview_answers(q, &past) {
        s.pick(code, text, &label, "calm", "");
    }
}

fn interview_open(w: &mut World, s: &mut Scene) {
    let team = s.team.unwrap_or(Team::from_employer(employer_of(s.a)));
    let names: Vec<String> =
        panel(w, team).iter().map(|c| format!("{}, {}", c.link(), c.title.to_lowercase())).collect();
    s.narrate(format!("Across the table: {}.", names.join(" and ")));
    interview_ask(w, s);
}

fn interview_answer(w: &mut World, s: &mut Scene, code: u32) {
    let (q, k) = if code == PLAIN_ANSWER { (2, 3u8) } else { (((code - 931) / 3) as usize, ((code - 931) % 3) as u8) };
    let team = s.team.unwrap_or(Team::from_employer(employer_of(s.a)));
    let panel = panel(w, team);
    let want = wants_of(w, team, &panel);
    let aura = w.career.as_ref().map_or(10.0, |c| c.you.aura());
    // Each answer: on what they want, a miss, or between; how you come
    // across (Aura) carries every answer a little. The plain answer of a
    // coach with no record is between.
    let hit = if k == 3 {
        0.0
    } else if k == want[q] {
        0.45
    } else if (k as i16 - want[q] as i16).abs() == 1 {
        0.0
    } else {
        -0.3
    };
    s.f[q] = (hit + 0.015 * (aura - 10.0)) as f32;
    let past = rooms::past(w);
    let said = interview_answers(q, &past)
        .into_iter()
        .find(|x| x.1 == k)
        .map_or_else(|| QUESTIONS[q].1[(k as usize).min(2)].to_string(), |x| x.2);
    s.you(&scene::your_name(w), said.trim_matches('"'));
    if let Some(c) = panel.get(q % panel.len().max(1)) {
        let sp = Speaker::cast(w, c);
        // What the panel makes of the number you just gave them (item 19):
        // a line that names it, before how they take it.
        if q == 2 && k < 3 {
            let mut f = Facts::new();
            rooms::chair(w, team, &mut f);
            past.fill(&mut f);
            let part = ["failure", "results", "people"][k as usize];
            let t = line(w, s, 40 + k as u64, &format!("interview.react.past.{part}"), &sp, &f);
            if !t.is_empty() {
                s.them(&c.name, t, None);
            }
        }
        let mood = if hit > 0.0 {
            "good"
        } else if hit < 0.0 {
            "bad"
        } else {
            "flat"
        };
        let t = line(w, s, 20 + q as u64, &format!("interview.react.{mood}"), &sp, &Facts::new());
        s.them(&c.name, t, None);
    }
    if q < 2 {
        s.step = (q + 1) as u8;
        interview_ask(w, s);
        s.step = q as u8;
        return;
    }
    let score = s.f[0] + s.f[1] + s.f[2];
    let e = employer_of(s.a);
    w.folk.dialogue.interviews.retain(|x| !(x.0 == e && x.1 == Role::HeadCoach));
    w.folk.dialogue.interviews.push((e, Role::HeadCoach, score, w.today));
    s.outcome.push(
        if score > 0.6 {
            "They liked what they heard. You'll be on the short list."
        } else if score > -0.2 {
            "A fair interview. It's up to the rest of your record now."
        } else {
            "It didn't go your way. They wanted to hear something else."
        }
        .to_string(),
    );
    s.end();
    if s.c == 1 {
        fill_after(w, s, e);
    }
}

fn interview_hand(w: &mut World, s: &mut Scene) {
    let e = employer_of(s.a);
    w.folk.dialogue.interviews.retain(|x| !(x.0 == e && x.1 == Role::HeadCoach));
    w.folk.dialogue.interviews.push((e, Role::HeadCoach, -1.0, w.today));
    s.handed = Some("nobody".into());
    s.outcome.push("You missed the interview. The committee noticed.".into());
    if s.c == 1 {
        fill_after(w, s, e);
    }
}

/// A seat nobody holds is filled once you have sat down with them: by you,
/// or by the best name they have.
fn fill_after(w: &mut World, s: &mut Scene, e: Employer) {
    use crate::world::front_office::market;
    let prestige = crate::world::staff::prestige_of(w, e);
    crate::world::staff::fill_now(w, e, Role::HeadCoach, prestige);
    let name = market::employer_name(w, e);
    let offered =
        w.career.as_ref().is_some_and(|c| c.market.offers.iter().any(|o| o.employer == e && o.role == Role::HeadCoach));
    s.outcome.push(if offered {
        format!("{name} wants you. Their offer is on your job market page.")
    } else {
        match crate::world::staff::holder(w, e, Role::HeadCoach) {
            Some(h) => format!("{name} went with {}.", w.coaches[h].name),
            None => format!("{name} didn't find its man yet."),
        }
    });
}

/// What your interview adds to a committee's read of you (market `bonus`).
pub(crate) fn interview_bonus(w: &World, e: Employer, r: Role) -> f64 {
    w.folk
        .dialogue
        .interviews
        .iter()
        .filter(|x| x.0 == e && x.1 == r && x.3.days_to(w.today) <= 120)
        .map(|x| x.2 as f64)
        .sum()
}

// ---------------------------------------------------------------------------
// D12: the weekly staff meeting.

/// The case a coordinator brings after a game, from what he saw in it:
/// the coordinator, the case and his unit's points.
fn case_from(w: &World, g: &crate::world::seat::SeatGame, me: usize) -> Option<(usize, u8, i32)> {
    let team = Team::seat(w)?;
    let [oc, dc] = super::gametalk::coordinators(w, team);
    let side = if g.teams[0] == me { 0 } else { 1 };
    let them = 1 - side;
    let st = &g.stats;
    let ours = &st.team[side];
    let theirs = &st.team[them];
    let carries = ours.plays.saturating_sub(ours.dropbacks).max(1) as f64;
    let ypc = ours.rush_yards / carries;
    let run_share = carries / ours.plays.max(1) as f64;
    let pressure = theirs.pressures as f64 / theirs.dropbacks.max(1) as f64;
    let big_allowed = theirs.explosive;
    let allowed = st.score[them];
    let scored = st.score[side];
    let bad_d = allowed >= 28;
    if bad_d && let Some(c) = dc {
        return Some(if pressure < 0.28 {
            (c, 0, allowed)
        } else if big_allowed >= 6 {
            (c, 1, allowed)
        } else {
            (c, 0, allowed)
        });
    }
    if let Some(c) = oc {
        if ypc >= 5.0 && run_share < 0.48 {
            return Some((c, 2, scored));
        }
        if ypc < 3.3 && run_share > 0.45 {
            return Some((c, 3, scored));
        }
    }
    None
}

/// The morning after each of your games the staff meets (college on
/// Sunday, the NFL on Monday, a high school on Saturday): your
/// coordinator's case waits for you, and the last case is checked against
/// the game.
pub(crate) fn staff_meeting_day(w: &mut World, d: Date) {
    if w.career.as_ref().is_some_and(|c| c.role.is_some()) {
        return;
    }
    let Some((_, me)) = w.seat_team() else { return };
    let Some(g) = w.seat_games.last().filter(|g| g.date <= d && g.date.days_to(d) <= 1) else { return };
    if w.folk.dialogue.met.is_some_and(|m| m >= g.date) {
        return;
    }
    let g = g.clone();
    w.folk.dialogue.met = Some(g.date);
    check_case(w, &g, me);
    let Some((coach, kind, before)) = case_from(w, &g, me) else { return };
    // The morning after the game (this step runs before the day's games).
    let date = g.date.add(1);
    let role = if kind <= 1 { "DC" } else { "OC" };
    let mut s = Scene::new(Kind::StaffMeeting, date, format!("Staff meeting: your {role} wants a change"));
    s.place = "The staff room, 7:00 a.m.".into();
    s.a = coach as u64;
    s.b = kind as u64;
    s.c = before as i64;
    s.who = vec![Who::Coach(coach)];
    scene::add(w, s);
}

const CASES: [(&str, &str); 4] = [
    ("pressure", "more pressure"),
    ("zone", "more zone coverage"),
    ("run", "running it more"),
    ("pass", "throwing it more"),
];

/// Every move the plan can make at a staff meeting: his four cases, then
/// the two the other way on defense (live talks LT17: your own case).
pub const MOVES: [&str; 6] =
    ["more pressure", "more zone coverage", "running it more", "throwing it more", "less pressure", "more man coverage"];

/// The moves you can make instead of his case (LT17, picks 954-959): the
/// others on his side of the ball, the menu short.
pub fn own_moves(his: u8) -> Vec<u8> {
    let side: &[u8] = if his <= 1 { &[0, 1, 4, 5] } else { &[2, 3] };
    side.iter().copied().filter(|&m| m != his).collect()
}

/// A move as the setting it changes, its way and its step, the terms a
/// typed game plan ask is heard in (`scene::PlanAsk`).
pub fn move_plan(m: u8) -> (&'static str, i8) {
    match m {
        0 => ("pressure", 1),
        1 => ("man", -1),
        2 => ("run", 1),
        3 => ("run", -1),
        4 => ("pressure", -1),
        _ => ("man", 1),
    }
}

/// The game the meeting follows: the morning after it (`staff_meeting_day`
/// sets the scene for the day after the game).
fn meeting_game(w: &World, s: &Scene) -> Option<crate::world::seat::SeatGame> {
    let (_, me) = w.seat_team()?;
    w.seat_games.iter().rev().find(|g| g.date.add(1) == s.date && (g.teams[0] == me || g.teams[1] == me)).cloned()
}

/// Where the game plan's guideline stands now and where an answer to the
/// case would move it, as shares (`shift_plan` moves it: `k` is 1.0 for
/// agreeing, 0.5 for halfway). Pressure, man coverage and the run share are
/// what the four cases move.
fn plan_step(w: &World, kind: u8, k: f64) -> (f64, f64) {
    let g = &w.plans.guide;
    match kind {
        0 => {
            let now = g.pressure.to.unwrap_or(0.3);
            (now, (now + 0.08 * k).clamp(0.05, 0.7))
        }
        1 => {
            let now = g.man.to.unwrap_or(0.4);
            (now, (now - 0.1 * k).clamp(0.05, 0.9))
        }
        2 => {
            let now = g.run.to.unwrap_or(0.44);
            (now, (now + 0.06 * k).clamp(0.2, 0.75))
        }
        3 => {
            let now = g.run.to.unwrap_or(0.44);
            (now, (now - 0.06 * k).clamp(0.2, 0.75))
        }
        4 => {
            let now = g.pressure.to.unwrap_or(0.3);
            (now, (now - 0.08 * k).clamp(0.05, 0.7))
        }
        _ => {
            let now = g.man.to.unwrap_or(0.4);
            (now, (now + 0.1 * k).clamp(0.05, 0.9))
        }
    }
}

/// What the plan's guideline is for each move, in words.
const PLAN_WORDS: [&str; 6] =
    ["Pressure", "Man coverage", "The run share", "The run share", "Pressure", "Man coverage"];

fn pct(x: f64) -> i64 {
    (100.0 * x).round() as i64
}

fn meeting_open(w: &mut World, s: &mut Scene) {
    let c = s.a as usize;
    let kind = (s.b as usize).min(3);
    let sp = Speaker::coach(w, c);
    let name = w.coaches[c].name.clone();
    // The numbers behind the case reach the words (item 19): the game he is
    // arguing from. The fact part comes first; with the game gone from the
    // file, today's plain case stands.
    let game = meeting_game(w, s).zip(w.seat_team()).map(|(g, (_, me))| rooms::case(&g, me));
    let mut f = Facts::new();
    if let Some(n) = &game {
        n.fill(&mut f);
    }
    let mut t = String::new();
    if game.is_some() {
        t = line(w, s, 1, &format!("staffmeet.case.{}.facts", CASES[kind].0), &sp, &f);
    }
    if t.is_empty() {
        t = line(w, s, 1, &format!("staffmeet.case.{}", CASES[kind].0), &sp, &Facts::new());
    }
    s.narrate(format!("{} has the floor first, as he asked.", super::coach_link(w, c)));
    s.them(&name, t, None);
    if let Some(n) = &game {
        s.docs.push(case_doc(kind, n));
    }
    let (now, full) = plan_step(w, kind as u8, 1.0);
    let (_, half) = plan_step(w, kind as u8, 0.5);
    s.pick(
        951,
        format!("\"Do it. {} this week.\"", cap(CASES[kind].1)),
        "Agree",
        "calm",
        &format!(
            "{} in the plan goes from {}% to {}%. He owns it if it goes wrong.",
            PLAN_WORDS[kind],
            pct(now),
            pct(full)
        ),
    );
    s.pick(
        952,
        "\"Some of it. Meet me halfway.\"",
        "Halfway",
        "calm",
        &format!("{} goes from {}% to {}%. Nobody's happy, nobody's wrong.", PLAN_WORDS[kind], pct(now), pct(half)),
    );
    s.pick(
        953,
        "\"No. We stay with the plan.\"",
        "Overrule",
        "firm",
        "If his unit struggles again, he'll remember you said no.",
    );
    // Your own case instead (live talks LT17, question 5's short menu).
    for m in own_moves(kind as u8) {
        let (now, to) = plan_step(w, m, 1.0);
        s.pick(
            954 + m as u32,
            format!("\"No. I want {} this week.\"", MOVES[m as usize]),
            &format!("Your own call: {}", MOVES[m as usize]),
            "firm",
            &format!(
                "{} goes from {}% to {}%. He'll argue from his numbers, and it's on you if it goes wrong.",
                PLAN_WORDS[m as usize],
                pct(now),
                pct(to)
            ),
        );
    }
}

/// The game's numbers beside the case, as the box score has them.
fn case_doc(kind: usize, n: &rooms::Case) -> super::scene::Doc {
    use super::scene::{Doc, DocRow};
    let row = |a: &str, b: String| DocRow { pid: None, cells: vec![a.to_string(), b] };
    let rows = if kind <= 1 {
        vec![
            row("Points allowed", n.allowed.to_string()),
            row("Their passers pressured", format!("{}%", n.pressure)),
            row("Explosive plays they had", n.big.to_string()),
        ]
    } else {
        vec![
            row("Points scored", n.scored.to_string()),
            row("Yards a carry", format!("{:.1}", n.ypc)),
            row("Share of your plays that were runs", format!("{}%", n.run_share)),
        ]
    };
    Doc {
        title: format!(
            "{} {} {}-{}",
            match n.score[0].cmp(&n.score[1]) {
                std::cmp::Ordering::Greater => "Beat",
                std::cmp::Ordering::Less => "Lost to",
                std::cmp::Ordering::Equal => "Tied",
            },
            n.opp,
            n.score[0],
            n.score[1]
        ),
        note: format!("The game against {}, the numbers he is pointing at.", n.opp),
        cols: vec![String::new(), "Last game".into()],
        rows,
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

fn meeting_answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let c = s.a as usize;
    let kind = (s.b as u8).min(3);
    // Your own case (LT17): a move on his side of the ball, one full step.
    let mine = code.checked_sub(954).map(|m| m as u8).filter(|m| own_moves(kind).contains(m));
    let said = if mine.is_some() { 2 } else { (code.saturating_sub(951)).min(2) as u8 };
    if let Some(m) = mine {
        shift_plan(w, m, 1.0);
    } else if said < 2 {
        shift_plan(w, kind, if said == 0 { 1.0 } else { 0.5 });
    }
    w.folk.dialogue.case = Some(Case { coach: c, kind, said, date: w.today, before: s.c as i32, mine });
    s.you(&scene::your_name(w), picked.label.trim_matches('"'));
    let sp = Speaker::coach(w, c);
    let mood = ["yes", "half", "no"][said as usize];
    let t = line(w, s, 2, &format!("staffmeet.reply.{mood}"), &sp, &Facts::new());
    s.them(&w.coaches[c].name.clone(), t, None);
    s.outcome.push(match (mine, said) {
        (Some(m), _) => {
            format!("Your own call: {} this week. He argued from his numbers; it's yours now.", MOVES[m as usize])
        }
        (_, 0) => format!("You agreed: {} this week.", CASES[kind as usize].1),
        (_, 1) => format!("Halfway: a little {} this week.", CASES[kind as usize].1.trim_start_matches("more ")),
        _ => "You overruled him. The plan stands.".into(),
    });
    s.end();
}

fn meeting_hand(w: &mut World, s: &mut Scene) {
    // Your assistant head coach runs the meeting: the coordinator gets
    // half of what he asked.
    let kind = (s.b as u8).min(3);
    shift_plan(w, kind, 0.5);
    w.folk.dialogue.case =
        Some(Case { coach: s.a as usize, kind, said: 1, date: w.today, before: s.c as i32, mine: None });
    s.handed = Some("your assistant head coach".into());
    s.outcome.push("Half the change went in.".into());
}

/// Moves the game plan's guideline toward the coordinator's case.
fn shift_plan(w: &mut World, kind: u8, k: f64) {
    use crate::game::guide::{Firm, Want};
    let g = &mut w.plans.guide;
    let firm = Firm::default();
    match kind {
        0 => {
            let now = g.pressure.to.unwrap_or(0.3);
            g.pressure = Want::new((now + 0.08 * k).clamp(0.05, 0.7), g.pressure.to.map_or(firm, |_| g.pressure.firm));
        }
        1 => {
            let now = g.man.to.unwrap_or(0.4);
            g.man = Want::new((now - 0.1 * k).clamp(0.05, 0.9), g.man.to.map_or(firm, |_| g.man.firm));
        }
        2 => {
            let now = g.run.to.unwrap_or(0.44);
            g.run = Want::new((now + 0.06 * k).clamp(0.2, 0.75), g.run.to.map_or(firm, |_| g.run.firm));
        }
        3 => {
            let now = g.run.to.unwrap_or(0.44);
            g.run = Want::new((now - 0.06 * k).clamp(0.2, 0.75), g.run.to.map_or(firm, |_| g.run.firm));
        }
        4 => {
            let now = g.pressure.to.unwrap_or(0.3);
            g.pressure = Want::new((now - 0.08 * k).clamp(0.05, 0.7), g.pressure.to.map_or(firm, |_| g.pressure.firm));
        }
        _ => {
            let now = g.man.to.unwrap_or(0.4);
            g.man = Want::new((now + 0.1 * k).clamp(0.05, 0.9), g.man.to.map_or(firm, |_| g.man.firm));
        }
    }
    w.plans.guide_set = Some(w.today);
}

/// Last week's case, checked against the game since: overruled and his
/// unit struggled again costs his trust in you; agreed and it worked earns
/// a little.
fn check_case(w: &mut World, g: &crate::world::seat::SeatGame, me: usize) {
    let Some(case) = w.folk.dialogue.case.take() else { return };
    if g.date <= case.date {
        w.folk.dialogue.case = Some(case);
        return;
    }
    let side = if g.teams[0] == me { 0 } else { 1 };
    let pts = if case.kind <= 1 { g.stats.score[1 - side] } else { g.stats.score[side] };
    let defense = case.kind <= 1;
    let bad = if defense { pts >= 28 } else { pts < 17 };
    let good = if defense { pts <= 17 } else { pts >= 28 };
    // Your own case (LT17) at today's sizes: it went wrong, as an overrule
    // that went wrong; it worked, as an agreed case that worked.
    let by = match (case.mine.is_some(), case.said, bad, good) {
        (true, _, true, _) => -0.06,
        (true, _, _, true) => 0.03,
        (true, ..) => 0.0,
        (_, 2, true, _) => -0.06,
        (_, 2, _, true) => 0.01,
        (_, 0, _, true) => 0.03,
        (_, 0, true, _) => -0.01,
        _ => 0.0,
    };
    if by != 0.0 {
        w.folk.mine.nudge(super::coach_key(case.coach), by);
    }
    if case.said == 2 && bad {
        let name = w.coaches[case.coach].name.clone();
        let text = match case.mine {
            Some(_) => format!("{name} hasn't forgotten his numbers said otherwise. His unit struggled after your call."),
            None => format!("{name} hasn't forgotten you overruled him. His unit struggled again."),
        };
        super::note(w, "staff", "Staff meeting".into(), text, None);
    }
}

// ---------------------------------------------------------------------------
// D13: the players-only meeting.

/// Mondays: a team on a losing streak (three in a row) may hold a
/// players-only meeting, called by its captains; what it does to the room
/// comes from them. Every college and NFL team, and yours at any level.
pub(crate) fn players_only_day(w: &mut World, d: Date) {
    if d.weekday() != Weekday::Mon {
        return;
    }
    let mut teams: Vec<(Team, Vec<usize>)> = Vec::new();
    if let Some(cs) = w.college.as_ref() {
        let mut lost: std::collections::BTreeMap<usize, Vec<(Date, bool)>> = std::collections::BTreeMap::new();
        for g in cs.games.iter().filter(|g| g.date < d && g.date.days_to(d) <= 40) {
            let Some(p) = g.played.filter(|p| !p.cancelled) else { continue };
            lost.entry(g.home).or_default().push((g.date, p.score[0] < p.score[1]));
            lost.entry(g.away).or_default().push((g.date, p.score[1] < p.score[0]));
        }
        for (t, mut v) in lost {
            v.sort_by_key(|x| x.0);
            if v.len() >= 3 && v.iter().rev().take(3).all(|x| x.1) && v.last().is_some_and(|x| x.0.days_to(d) <= 3) {
                teams.push((Team::College(t), w.programs[t].captains.clone()));
            }
        }
    }
    if w.nfl_season.is_some() {
        let mut lost: std::collections::BTreeMap<usize, Vec<(Date, bool)>> = std::collections::BTreeMap::new();
        for f in w.nfl_fixtures(None) {
            let (Some(when), Some(sc)) = (f.when, f.score) else { continue };
            if when >= d || when.days_to(d) > 30 {
                continue;
            }
            lost.entry(f.home).or_default().push((when, sc[0] < sc[1]));
            lost.entry(f.away).or_default().push((when, sc[1] < sc[0]));
        }
        for (t, mut v) in lost {
            v.sort_by_key(|x| x.0);
            if v.len() >= 3 && v.iter().rev().take(3).all(|x| x.1) && v.last().is_some_and(|x| x.0.days_to(d) <= 2) {
                let caps = w.clubs.get(t).map(|c| c.captains.clone()).unwrap_or_default();
                teams.push((Team::Nfl(t), caps));
            }
        }
    }
    // Your high school (or other league) team, from your own games.
    if let Some(seat) = Team::seat(w)
        && matches!(seat, Team::High(_) | Team::Other(_))
    {
        let me = match seat {
            Team::High(h) | Team::Other(h) => h,
            _ => 0,
        };
        let last: Vec<bool> = w
            .seat_games
            .iter()
            .rev()
            .take(3)
            .filter(|g| g.date.days_to(d) <= 30)
            .map(|g| {
                let s = if g.teams[0] == me { 0 } else { 1 };
                g.stats.score[s] < g.stats.score[1 - s]
            })
            .collect();
        if last.len() == 3 && last.iter().all(|&x| x) {
            teams.push((seat, Vec::new()));
        }
    }
    let mut usual: Vec<(std::mem::Discriminant<Team>, (f64, f64))> = Vec::new();
    for (team, caps) in teams {
        if w.folk.dialogue.players_only.iter().any(|x| x.0 == team && x.1.days_to(d) < 21) {
            continue;
        }
        let kind = std::mem::discriminant(&team);
        let norm = match usual.iter().find(|x| x.0 == kind) {
            Some(x) => x.1,
            None => {
                let n = usual_leaders(w, team);
                usual.push((kind, n));
                n
            }
        };
        players_only(w, team, caps, norm, d);
    }
}

/// Who leads a players-only meeting: the captains, or when a team has
/// none its natural leaders (leadership, then years).
fn meeting_leaders(w: &World, team: Team, caps: &[usize]) -> Vec<usize> {
    if !caps.is_empty() {
        return caps.to_vec();
    }
    let mut v = team.roster(w);
    v.sort_by(|&a, &b| {
        let s = |i: usize| {
            w.people[i].player.personality.traits.get(Trait::Leadership) + 0.3 * w.people[i].player.age as f64
        };
        s(b).total_cmp(&s(a))
    });
    v.truncate(2);
    v
}

/// The best leadership and the professionalism a meeting's leaders bring
/// at teams of this kind, on average. Captains are picked for both, so a
/// meeting goes well or badly against the usual captain, not the average
/// player: with ordinary captains it neither lifts the room nor sinks it.
fn usual_leaders(w: &World, team: Team) -> (f64, f64) {
    let teams: Vec<(Team, &[usize])> = match team {
        Team::College(_) => w
            .programs
            .iter()
            .enumerate()
            .filter(|(_, p)| p.plays(w.year))
            .map(|(k, p)| (Team::College(k), p.captains.as_slice()))
            .collect(),
        Team::Nfl(_) => w.clubs.iter().enumerate().map(|(k, c)| (Team::Nfl(k), c.captains.as_slice())).collect(),
        Team::High(_) => (0..w.high_schools.len()).map(|h| (Team::High(h), &[][..])).collect(),
        Team::Other(_) => (0..w.others.teams.len()).map(|f| (Team::Other(f), &[][..])).collect(),
    };
    let (mut lead, mut pro, mut n) = (0.0, 0.0, 0.0);
    for (t, caps) in teams {
        let caps = meeting_leaders(w, t, caps);
        if caps.is_empty() {
            continue;
        }
        let (l, p) = leaders_read(w, &caps);
        lead += l;
        pro += p;
        n += 1.0;
    }
    if n == 0.0 { (10.0, 10.0) } else { (lead / n, pro / n) }
}

/// The leaders' best leadership and their average professionalism.
fn leaders_read(w: &World, caps: &[usize]) -> (f64, f64) {
    let t = |i: usize, k: Trait| w.people[i].player.personality.traits.get(k);
    let lead = caps.iter().map(|&i| t(i, Trait::Leadership)).fold(0.0, f64::max);
    let pro = caps.iter().map(|&i| t(i, Trait::Professionalism)).sum::<f64>() / caps.len() as f64;
    (lead, pro)
}

fn players_only(w: &mut World, team: Team, caps: Vec<usize>, usual: (f64, f64), d: Date) {
    let roster = team.roster(w);
    let caps = meeting_leaders(w, team, &caps);
    if caps.is_empty() {
        return;
    }
    let t = |i: usize, k: Trait| w.people[i].player.personality.traits.get(k);
    let (lead, pro) = leaders_read(w, &caps);
    let ego = caps.iter().map(|&i| t(i, Trait::Ego)).fold(0.0, f64::max);
    let temper = caps.iter().map(|&i| t(i, Trait::Temper)).fold(0.0, f64::max);
    let key = match team {
        Team::College(p) => p as u64,
        Team::Nfl(t) => 1 << 32 | t as u64,
        Team::High(h) => 2 << 32 | h as u64,
        Team::Other(f) => 3 << 32 | f as u64,
    };
    // They call one if their leaders are the kind who would.
    if super::roll(w, 0xD13, key ^ (d.0 as u64) << 36) > (0.2 + 0.05 * (lead - 10.0)).clamp(0.05, 0.6) {
        return;
    }
    let noise = 0.03 * crate::game::talk::normal(w.options.seed as u64 ^ 0xD130, key ^ (d.0 as u64) << 36);
    let heated = ego >= 15.0 && temper >= 13.0;
    // How it goes comes from the captains (D13), against the usual ones.
    let delta = (0.06 * z(10.0 + lead - usual.0) + 0.03 * z(10.0 + pro - usual.1) - if heated { 0.06 } else { 0.0 }
        + noise)
        .clamp(-0.08, 0.1) as f32;
    w.folk.dialogue.players_only.retain(|x| x.0 != team);
    w.folk.dialogue.players_only.push((team, d));
    let yours = Team::seat(w) == Some(team);
    let ids: Vec<u32> = roster.iter().map(|&i| w.people[i].player.id).collect();
    if yours {
        for id in ids {
            super::talks::mood_event(w, id, super::talks::Reason::Room, delta);
        }
    } else {
        for &i in &roster {
            w.people[i].mood = (w.people[i].mood + delta).clamp(-1.0, 1.0);
        }
    }
    // Word gets out: to you through your staff, sometimes to the papers.
    let name = team.name(w);
    let cap = w.people[caps[0]].name.clone();
    let how = if delta > 0.03 {
        "Word is it cleared the air."
    } else if delta < -0.02 {
        "Word is it got heated."
    } else {
        "Nobody is saying much about it."
    };
    let leak = super::roll(w, 0xD131, key ^ (d.0 as u64) << 36) < if delta < 0.0 { 0.5 } else { 0.3 };
    if yours {
        // You weren't in the room: a staff member hears it from a player,
        // unless the papers have it first.
        let text = if leak {
            format!(
                "You read it before anyone told you: your captains, {cap} first among them, held a players-only meeting after the third loss. {how}"
            )
        } else {
            format!(
                "Your captains, {cap} first among them, held a players-only meeting after the third loss. A position coach heard about it from a player. {how}"
            )
        };
        super::note(w, "talk", "Players-only meeting".into(), text, None);
    }
    let loud = matches!(team, Team::Nfl(_))
        || matches!(team, Team::College(p) if w.programs[p].division == crate::world::Division::FBS)
        || yours;
    if leak && loud {
        let level = match team {
            Team::Nfl(_) | Team::Other(_) => Level::Nfl,
            Team::High(_) => Level::HighSchool,
            Team::College(_) => Level::College,
        };
        let mut e = Event::new(
            d,
            level,
            CalKind::News,
            Loudness::Small,
            format!("Sources: {name} held a players-only meeting after three straight losses. {how}"),
        );
        match team {
            Team::College(p) => e = e.program(p),
            Team::Nfl(t) => e = e.team(t),
            _ => {}
        }
        w.calendar.post(e);
    }
}

// ---------------------------------------------------------------------------
// D8: the combine interview as a timed talk.

/// The minutes each topic takes, and a follow-up.
/// Minutes each topic takes in an 18-minute combine room; a longer room
/// (a top-30 visit, an official visit's sit-down) goes deeper on each.
const COMBINE_COST: [u8; 5] = [6, 4, 4, 3, 3];

fn topic_minutes(s: &Scene, k: usize) -> u8 {
    ((COMBINE_COST[k] as f32 * s.f[0] / 18.0).round() as u8).clamp(1, 60)
}

fn press_cost(s: &Scene) -> u8 {
    ((2.0 * s.f[0] / 18.0).round() as u8).clamp(2, 10)
}

/// A flagged worry's question (live talks LT11): the shortest topics'
/// minutes in an 18-minute room, deeper in a longer one.
fn worry_cost(s: &Scene) -> u8 {
    ((crate::world::scouting::own::WORRY_MINUTES as f32 * s.f[0] / 18.0).round() as u8).clamp(1, 60)
}

/// The minutes a typed question takes beyond its topic's, by its length
/// (live talks LT11): one for every 150 characters. They come off the
/// clock and never add to the topic's minutes, so a long question only
/// costs. Proposed, not confirmed.
pub const TYPED_CHARS_A_MINUTE: u16 = 150;

fn typed_minutes(s: &Scene) -> u8 {
    s.heard.as_ref().map_or(0, |h| (h.len / TYPED_CHARS_A_MINUTE).min(10) as u8)
}

/// The worries your scouts flagged (bits 32-44 of `b`) and those you asked
/// (bits 48-60), by trait.
const FLAGGED: u32 = 32;
const ASKED: u32 = 48;

fn worry_bits(s: &Scene, at: u32) -> Vec<usize> {
    (0..crate::world::scouting::file::TRAITS).filter(|k| (s.b >> (at + *k as u32)) & 1 == 1).collect()
}

/// Opens a timed interview with a player (an NFL seat's combine
/// interview, or any interview room open with him today).
pub fn start_combine_talk(w: &mut World, id: u32, room: &str) -> Result<u32, String> {
    let rooms = crate::world::scouting::own::rooms(w, id);
    let (room, minutes) = rooms.into_iter().find(|r| r.0 == room).ok_or("That room isn't open with him today.")?;
    let i = super::person(w, id).ok_or("No such player.")?;
    let name = w.people[i].name.clone();
    let mut s = Scene::new(Kind::CombineTalk, w.today, format!("{room}: {name}"));
    s.stops = false;
    s.place = if room.starts_with("Combine") { "A hotel suite in Indianapolis".into() } else { "Your office".into() };
    s.a = id as u64;
    s.c = minutes as i64;
    s.f[0] = minutes as f32;
    s.who = vec![Who::Player(id)];
    s.outcome.clear();
    // The room's name rides in the scene's title; the minutes in `c`.
    Ok(scene::add(w, s))
}

fn spent_of(s: &Scene) -> [u8; 5] {
    let mut v = [0u8; 5];
    for (k, x) in v.iter_mut().enumerate() {
        *x = ((s.b >> (6 * k)) & 0x3F) as u8;
    }
    v
}

fn set_spent(s: &mut Scene, v: [u8; 5]) {
    let keep = s.b & !((1u64 << 30) - 1);
    s.b = keep | v.iter().enumerate().fold(0u64, |a, (k, &x)| a | (x.min(63) as u64) << (6 * k));
}

fn combine_picks(s: &mut Scene) {
    let left = s.c.max(0) as u8;
    let spent = spent_of(s);
    for (k, t) in crate::world::scouting::own::TOPICS.iter().enumerate() {
        let cost = topic_minutes(s, k);
        if cost <= left && spent[k] == 0 {
            s.pick(961 + k as u32, format!("{} ({cost} min)", t.words()), t.words(), "calm", "");
        }
    }
    // The worries your scouts flagged, each its own question (LT11).
    let asked = worry_bits(s, ASKED);
    let cost = worry_cost(s);
    for k in worry_bits(s, FLAGGED) {
        if cost <= left && !asked.contains(&k) {
            s.pick(
                970 + k as u32,
                format!("{} ({cost} min)", crate::world::scouting::own::worry_words(k)),
                "Flagged worry",
                "firm",
                "Your scouts flagged it. A question aimed at it reads it closer than the topic around it.",
            );
        }
    }
    let press = press_cost(s);
    if s.f[1] > 0.0 && left >= press {
        s.pick(
            967,
            format!("\"Stay on that. Why?\" ({press} min)"),
            "Press him",
            "firm",
            "A follow-up shows how he handles being pushed.",
        );
    }
    s.pick(969, "\"That's our time. Thanks for coming in.\"", "Wrap it up", "calm", "");
}

fn combine_open(w: &mut World, s: &mut Scene) {
    let Some(i) = super::person(w, s.a as u32) else { return s.end() };
    let kid = super::player_link(w, i);
    for k in crate::world::scouting::own::flagged_worries(w, s.a as u32) {
        s.b |= 1u64 << (FLAGGED + k as u32);
    }
    s.narrate(format!("{kid} sits down. The clock on the wall says {} minutes.", s.c));
    combine_picks(s);
}

fn combine_answer(w: &mut World, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else { return s.end() };
    let name = w.people[i].name.clone();
    if code == 969 || s.c <= 0 {
        return combine_close(w, s);
    }
    let long = typed_minutes(s);
    if (970..970 + crate::world::scouting::file::TRAITS as u32).contains(&code) {
        // A flagged worry, asked straight out (LT11): his answer is his
        // topic's, and the file reads that trait closer.
        let k = (code - 970) as usize;
        let t = crate::world::scouting::own::worry_topic(k);
        let cost = worry_cost(s);
        s.b |= 1u64 << (ASKED + k as u32);
        s.c -= cost as i64;
        s.you(&scene::your_name(w), crate::world::scouting::own::worry_question(k));
        let answer = crate::world::scouting::own::answer_line(w, i, t);
        s.them(&name, answer, None);
        s.f[1] = 1.0;
        s.f[2] = crate::world::scouting::own::TOPICS.iter().position(|x| *x == t).unwrap_or(4) as f32;
    } else if code == 967 {
        // The follow-up: pushed, a composed man answers, a hot one bristles.
        let p = &w.people[i];
        let comp = p.player.attributes.get(crate::attributes::Attr::Composure);
        let temper = p.player.personality.traits.get(Trait::Temper);
        let sp = Speaker::player(w, i);
        let mood = if comp >= 60.0 && temper < 13.0 {
            "steady"
        } else if temper >= 14.0 {
            "bristles"
        } else {
            "shaken"
        };
        let t = line(w, s, 30 + s.step as u64, &format!("combine.press.{mood}"), &sp, &Facts::new());
        let tell = match mood {
            "steady" => "He held your eyes and didn't hurry.",
            "bristles" => "His jaw set. He leaned back.",
            _ => "He looked at the table and started over.",
        };
        s.you(&scene::your_name(w), "Stay on that. Why?");
        s.them(&name, t, Some(tell.into()));
        let press = press_cost(s);
        s.c -= press as i64;
        s.f[1] = 0.0;
        let mut v = spent_of(s);
        let k = s.f[2] as usize;
        v[k.min(4)] = v[k.min(4)].saturating_add(press).min(63);
        set_spent(s, v);
    } else {
        let k = ((code - 961) as usize).min(4);
        let t = crate::world::scouting::own::TOPICS[k];
        let cost = topic_minutes(s, k);
        let mut v = spent_of(s);
        v[k] = cost;
        set_spent(s, v);
        s.c -= cost as i64;
        s.you(&scene::your_name(w), t.words());
        let answer = crate::world::scouting::own::answer_line(w, i, t);
        s.them(&name, answer, None);
        // A follow-up fits the harder questions.
        s.f[1] = if matches!(k, 1 | 2) { 1.0 } else { 0.0 };
        s.f[2] = k as f32;
    }
    if long > 0 {
        // A long question eats his answer time (LT11).
        s.c -= long as i64;
        s.narrate(format!(
            "Your question took {} to ask.",
            if long == 1 { "a minute".to_string() } else { format!("{long} minutes") }
        ));
    }
    if s.c <= 0 {
        s.narrate("A knock on the door: time.");
        return combine_close(w, s);
    }
    s.narrate(format!("{} minutes left.", s.c));
    combine_picks(s);
}

fn combine_close(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let spent: Vec<(crate::world::scouting::own::Topic, u8)> =
        crate::world::scouting::own::TOPICS.iter().zip(spent_of(s)).filter(|x| x.1 > 0).map(|(t, m)| (*t, m)).collect();
    let room = s.title.split(':').next().unwrap_or("Combine interview").to_string();
    let worry = worry_cost(s);
    let worries: Vec<(u8, u8)> = worry_bits(s, ASKED).into_iter().map(|k| (k as u8, worry)).collect();
    if spent.is_empty() && worries.is_empty() {
        s.outcome.push("You let him go without a question.".into());
    } else {
        match crate::world::scouting::own::interview_with(w, id, &room, spent, worries) {
            Ok(t) => s.outcome.push(format!("{t} His answers are in his file.")),
            Err(e) => s.outcome.push(e),
        }
    }
    s.end();
}

// ---------------------------------------------------------------------------
// D10: agents in their own voice.

/// An agent's temper, 1-20: a steady draw per agent (the market keeps his
/// honesty, not his temper).
pub fn agent_temper(w: &World, a: usize) -> f64 {
    (10.0 + 3.5 * crate::world::season::key_normal(w.options.seed as u64 ^ 0xA6E7_7E30, a as u64)).clamp(1.0, 20.0)
}

fn agent_speaker(w: &World, a: usize) -> Speaker {
    let ag = &w.market.agents[a];
    let mut tags: Vec<&'static str> = vec!["agent"];
    if ag.honesty >= 0.75 {
        tags.push("straight");
    } else if ag.honesty < 0.45 {
        tags.push("slick");
    }
    let t = agent_temper(w, a);
    if t >= 14.0 {
        tags.push("hot");
    } else if t <= 7.0 {
        tags.push("calm");
    }
    match ag.size {
        3 => tags.push("big"),
        1 => tags.push("small"),
        _ => {}
    }
    Speaker { key: 0xA6E0_0000 ^ a as u64, tags }
}

/// What a player's agent says in a contract talk (D10): the market decides
/// the part (in range, apart, a bluff) and the numbers; the words are his,
/// from his honesty and temper. A bluff from an honest man comes out
/// forced; a slick one's comes out smooth, and vague all the same. With no
/// agent, or no line, the plain words stand. He remembers what he has said
/// to you, as everyone in the talks does (minor bug 314).
pub fn agent_says(w: &mut World, i: usize, part: &str, facts: &Facts, plain: String) -> String {
    match agent_part(w, i, part, facts) {
        Some(t) => format!("\"{t}\""),
        None => plain,
    }
}

/// One part of his agent's bank in his voice, or None when the player has
/// no agent or the bank has no version he can say with these facts.
fn agent_part(w: &mut World, i: usize, part: &str, facts: &Facts) -> Option<String> {
    let a = w.people[i].pro.as_ref().and_then(|c| c.agent).filter(|&a| a < w.market.agents.len())?;
    let who = agent_speaker(w, a);
    let salt = part.bytes().fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
    let seed = w.today.0 as u64 ^ (w.people[i].player.id as u64) << 24 ^ salt << 40;
    let mut pen = Pen::take(w, seed);
    let said = pen.part(&format!("agent.{part}"), &who, facts).map(|x| super::voice::tidy(&x));
    pen.put(w);
    said.filter(|t| !t.is_empty())
}

/// What his agent says his client wants besides money (item 25): the
/// strongest of his role, winning, city, security, staying near home and
/// (with his own club) loyalty, when it pulls him past the bar, in the
/// agent's voice, next to a fact about your club where the want has one
/// (the man ahead of him on your chart, your last record). None when only
/// the money pulls him, or when the bank has no line yet: the talk then
/// goes on as it did. Returns the want's key with the words, in quotes.
pub fn agent_wants(w: &mut World, i: usize, club: usize, own_club: bool) -> Option<(&'static str, String)> {
    use rooms::Want;
    let want = rooms::top_want(w, i, own_club)?;
    let mut f = Facts::new().with("name", w.people[i].name.clone()).with("club", w.nfl_team_name(club));
    rooms::club_facts(w, i, club, want, &mut f);
    let parts: Vec<String> = match want {
        Want::Role if f.get("ahead").is_some() => vec!["wants.role.ahead".into(), "wants.role".into()],
        Want::Winning => match f.get("form") {
            Some(form) => vec![format!("wants.winning.{form}")],
            None => Vec::new(),
        },
        _ => vec![format!("wants.{}", want.key())],
    };
    parts.iter().find_map(|p| agent_part(w, i, p, &f)).map(|t| (want.key(), format!("\"{t}\"")))
}

// ---------------------------------------------------------------------------
// The scenes' dispatch (scene.rs).

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::HomeVisit => visit_open(w, s),
        Kind::PortalCall | Kind::PortalVisit => transfer_open(w, s),
        Kind::CollegeCall => call_open(w, s),
        Kind::JobInterview => interview_open(w, s),
        Kind::StaffMeeting => meeting_open(w, s),
        Kind::CombineTalk => combine_open(w, s),
        _ => s.end(),
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    match s.kind {
        Kind::HomeVisit => visit_answer(w, s, code, picked),
        Kind::PortalCall | Kind::PortalVisit => transfer_answer(w, s, code, picked),
        Kind::CollegeCall => call_answer(w, s, code, picked),
        Kind::JobInterview => interview_answer(w, s, code),
        Kind::StaffMeeting => meeting_answer(w, s, code, picked),
        Kind::CombineTalk => combine_answer(w, s, code),
        _ => s.end(),
    }
}

pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::HomeVisit => visit_hand(w, s),
        Kind::PortalCall | Kind::PortalVisit => transfer_hand(w, s),
        Kind::CollegeCall => {
            s.handed = Some("your voicemail".into());
            s.outcome.push("You never called back. They'll go on the film.".into());
        }
        Kind::JobInterview => interview_hand(w, s),
        Kind::StaffMeeting => meeting_hand(w, s),
        Kind::CombineTalk => combine_close(w, s),
        _ => {}
    }
}
