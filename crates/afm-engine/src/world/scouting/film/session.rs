//! The film session (update 3, lane 9; SK43-SK53, SK57, SK58): one cut-up
//! watched snap by snap. It starts from one open question (SK47) and ends
//! with your verdict on it; the tape stops before the moment on about one
//! snap in three and asks you to call it (SK48); a half-seen moment asks
//! whether to run it back (SK49); your position coach sits in and talks
//! (SK50), and your analyst if you have one (SK57). What your own eye
//! catches moves your own read (SK43), and the session becomes your report
//! in his file (SK58). "Watch it quick" gives the first look only (SK53).
//!
//! Every roll is keyed (the world's seed, the day, the player, the snap),
//! so a session plays the same way on any machine. Sizes are Claude's,
//! proposed, not confirmed.

use super::tape::{self, Cut, Game, Role, Snap, Still};
use super::{Catch, Tag, Target, words};
use crate::attributes::Attr;
use crate::calendar::Date;
use crate::positions::Position;
use crate::rng::clamp;
use crate::world::World;
use crate::world::scouting::file;
use crate::world::scouting::readers::{self, COACH, Staff};
use crate::world::season::{key_normal, key_uniform};

/// A team cut-up's hints: the key, the words and the snaps that show it.
pub type Hints = Vec<(String, String, Vec<u8>)>;

/// How you watch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Mode {
    /// Every snap: calls, run-backs, your coach and analyst in the room.
    Full,
    /// The cut-up for the same time, first look only (SK53).
    Quick,
    /// His game from the stands (SK32): the same stills, no replays.
    Live,
}

impl Mode {
    pub fn key(self) -> &'static str {
        match self {
            Mode::Full => "full",
            Mode::Quick => "quick",
            Mode::Live => "live",
        }
    }
}

/// What your eye did with one thing on a snap.
pub const NOT_YET: u8 = 0;
pub const CLEAR: u8 = 1;
pub const HALF: u8 = 2;
pub const REPLAYED: u8 = 3;
pub const REPLAY_MISSED: u8 = 4;
pub const MOVED_ON: u8 = 5;
pub const SLIPPED: u8 = 6;

/// Minutes a snap takes (SK46: about 20 minutes a cut-up of five or six),
/// a run-back, and a quick cut-up.
pub const SNAP_MIN: f32 = 4.0;
pub const RUNBACK_MIN: f32 = 2.0;
pub const QUICK_MIN: f32 = 20.0;

/// Your tags (SK44).
pub const TAGS: [&str; 4] = ["Plus", "Minus", "Ask him about this", "Show the staff"];

/// Who sits in with you.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Room {
    /// The coach's index (a college or high school coach), or None for a
    /// drawn NFL position coach (the world keeps none; the coordinator of
    /// his side sits in).
    pub coach: Option<usize>,
    pub name: String,
    pub job: String,
    /// His eye, 1-20.
    pub eye: f64,
}

/// The session in progress.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub target: Target,
    pub cut: Cut,
    pub mode: Mode,
    pub date: Date,
    pub name: String,
    pub pos: Position,
    /// The question to settle (SK47): key, words, why it was picked, its
    /// attributes and the bar at your level.
    pub ask: Option<Question>,
    pub games: Vec<Game>,
    pub snaps: Vec<Snap>,
    /// How many snaps the whole cut-up holds.
    pub cut_n: usize,
    pub at: usize,
    /// 0 ready to play, 1 the tape stopped for a call, 2 watched, 3 your
    /// verdict.
    pub step: u8,
    pub picks: Vec<Option<u8>>,
    pub seen: Vec<Vec<u8>>,
    /// Whether the coach caught each thing (his notes, SK50).
    pub his: Vec<Vec<bool>>,
    pub marks: Vec<Option<u8>>,
    pub tired: Vec<bool>,
    pub minutes: f32,
    pub coach: Option<Room>,
    pub analyst: Option<(String, u8)>,
    /// What was said in the room: (snap, who, words).
    pub talk: Vec<(u8, String, String)>,
    /// The analyst's numbers for the cut-up.
    pub numbers: String,
    /// The game-plan hints a team's tape holds (key, words, snaps).
    #[serde(default)]
    pub hints: Hints,
    /// A team's tape: the kept tape it came from (date, home, away).
    #[serde(default)]
    pub tape_key: Option<(Date, usize, usize)>,
    /// Seconds of your day already watched when the session opened.
    pub salt: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Question {
    pub key: String,
    pub text: String,
    pub why: String,
    pub attrs: Vec<u8>,
    pub bar: f32,
}

/// One snap kept with a report, for the look-back (SK58).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Kept {
    pub sit: String,
    pub job: String,
    pub result: String,
    pub stills: Vec<Still>,
}

/// A finished session: your report in his file (SK58).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub target: Target,
    pub name: String,
    pub date: Date,
    pub cut: Cut,
    pub mode: Mode,
    pub ask: Option<Question>,
    /// 0 yes, 1 no, 2 not sure.
    pub verdict: Option<u8>,
    /// The look-back three years on: whether you were right.
    #[serde(default)]
    pub settled: Option<bool>,
    /// Calls right and made.
    pub calls: (u8, u8),
    /// Things your eye caught, and the lines.
    pub caught: Vec<String>,
    /// (snap, tag) and the snap's line.
    pub marks: Vec<(u8, u8, String)>,
    pub minutes: u16,
    pub coach: Option<String>,
    pub analyst: Option<String>,
    /// The game-plan hints you found (a team's tape).
    #[serde(default)]
    pub hints: Vec<String>,
    /// The snaps it rests on (kept with a verdict only).
    #[serde(default)]
    pub snaps: Vec<Kept>,
    /// Snaps watched.
    #[serde(default)]
    pub watched: u8,
    /// The analyst's view at the end.
    #[serde(default)]
    pub numbers: String,
    /// The coach's answer to the question.
    #[serde(default)]
    pub coach_answer: Option<u8>,
    /// A team's tape: the kept tape it came from (date, home, away).
    #[serde(default)]
    pub tape_key: Option<(Date, usize, usize)>,
}

// --- Opening ----------------------------------------------------------------------------------

/// The bar "at our level" for a question's attributes (SK47): an NFL
/// starter's, an FBS starter's, and so on down.
pub fn bar(w: &World) -> f32 {
    use crate::world::data::Division;
    match readers::seat_staff(w) {
        Some(Staff::Club(_)) => 70.0,
        Some(Staff::Program(p)) => match w.programs[p].division {
            Division::FBS => 62.0,
            Division::FCS => 57.0,
            _ => 52.0,
        },
        Some(Staff::School(_)) => 46.0,
        None => 60.0,
    }
}

/// The questions on offer for a player (SK47), the default first: where
/// your own read is widest, where your staff disagree most, and the rest.
pub fn questions(w: &World, id: u32) -> Vec<Question> {
    let Some(i) = super::super::place(w, id) else { return Vec::new() };
    let pos = w.people[i].player.position;
    let Some(staff) = readers::seat_staff(w) else { return Vec::new() };
    let staff_read = readers::read(w, staff, id);
    let own = super::super::yours::own_read(w, staff, id, staff_read.as_ref().map(|x| &x.2));
    let asks = words::asks(pos);
    let wide = |a: &words::Ask| -> f64 {
        let o = own.as_ref();
        a.attrs
            .iter()
            .map(|x| o.and_then(|r| r.attrs.get(x.index()).copied().flatten()).map_or(9.0, |v| v.1))
            .sum::<f64>()
            / a.attrs.len().max(1) as f64
    };
    let split = |a: &words::Ask| -> f64 {
        let Some((_, _, f)) = &staff_read else { return 0.0 };
        let mut s = 0.0;
        for x in a.attrs {
            let cs: Vec<f64> =
                f.per_reader.iter().filter_map(|r| r.get(x.index()).copied().flatten()).map(|v| v.0).collect();
            if cs.len() >= 2 {
                let lo = cs.iter().cloned().fold(f64::MAX, f64::min);
                let hi = cs.iter().cloned().fold(f64::MIN, f64::max);
                s += hi - lo;
            }
        }
        s
    };
    let b = bar(w);
    let make = |a: &words::Ask, why: String| Question {
        key: a.key.into(),
        text: a.text.into(),
        why,
        attrs: a.attrs.iter().map(|x| x.index() as u8).collect(),
        bar: b,
    };
    let mut out = Vec::new();
    if let Some(a) = asks.iter().max_by(|x, y| wide(x).total_cmp(&wide(y))) {
        out.push(make(a, "Your own read is widest here: you haven't seen it yourself.".into()));
    }
    if let Some(a) = asks.iter().filter(|a| split(a) >= 3.0).max_by(|x, y| split(x).total_cmp(&split(y)))
        && !out.iter().any(|q| q.key == a.key)
    {
        out.push(make(a, "Your staff disagree on this one.".into()));
    }
    for a in asks {
        if !out.iter().any(|q| q.key == a.key) {
            out.push(make(a, "Your own pick.".into()));
        }
    }
    out
}

/// Who sits in (SK50): his position coach at a college or high school; at
/// an NFL club the coordinator of his side (the world keeps no NFL
/// position coaches).
fn room_coach(w: &World, pos: Position) -> Option<Room> {
    use crate::world::coach::Group;
    let g = Group::of(pos);
    let you = readers::you(w);
    let mk = |c: usize, job: &str| Room {
        coach: Some(c),
        name: w.coaches[c].name.clone(),
        job: job.to_string(),
        eye: w.coaches[c].evaluation(),
    };
    match readers::seat_staff(w)? {
        Staff::Program(p) => {
            let pr = &w.programs[p];
            let c = pr.position_coaches[g as usize].filter(|&c| Some(c) != you).or_else(|| {
                let side = if g.offense() { pr.oc } else { pr.dc };
                side.filter(|&c| Some(c) != you)
            })?;
            let job = if pr.position_coaches[g as usize] == Some(c) {
                cap(g.coach_title())
            } else if g.offense() {
                "Offensive coordinator".to_string()
            } else {
                "Defensive coordinator".to_string()
            };
            Some(mk(c, &job))
        }
        Staff::Club(t) => {
            let tm = &w.nfl[t];
            let c = if g.offense() { tm.oc } else { tm.dc }.filter(|&c| Some(c) != you)?;
            Some(mk(c, if g.offense() { "Offensive coordinator" } else { "Defensive coordinator" }))
        }
        // A high school head coach's assistants are teachers the world
        // keeps no record of: you watch alone.
        Staff::School(_) => None,
    }
}

/// Your analyst, if you have one (SK57): an NFL club's analytics director
/// (from 2003), an FBS program's (from 2016); high schools have none.
fn room_analyst(w: &World) -> Option<(String, u8)> {
    let staff = readers::seat_staff(w)?;
    if !readers::has_analytics(w, staff) {
        return None;
    }
    let s = match staff {
        Staff::Club(t) => &w.clubs.get(t)?.support.analytics,
        Staff::Program(p) => &w.programs[p].support.analytics,
        Staff::School(_) => return None,
    };
    let r = if s.rating == 0 { 10 } else { s.rating };
    Some((s.name.clone(), r))
}

/// One of your own players (his practices and games are your tape).
pub fn is_ours(w: &World, id: u32) -> bool {
    use crate::world::Stage;
    let Some(i) = super::super::place(w, id) else { return false };
    let p = &w.people[i];
    match readers::seat_staff(w) {
        Some(Staff::Club(t)) => p.stage == Stage::Pro && p.pro.as_ref().is_some_and(|c| c.team == Some(t)),
        Some(Staff::Program(c)) => p.stage == Stage::College && p.college == Some(c),
        Some(Staff::School(h)) => matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && p.hs == h,
        None => false,
    }
}

/// Opens a player's cut-up (SK45): his tape on the engine, the snaps of
/// the cut, the question, who sits in. Time is spent as you watch.
pub fn open_player(w: &mut World, id: u32, cut: Cut, mode: Mode, ask: Option<&str>) -> Result<String, String> {
    let staff = readers::seat_staff(w).ok_or("Start a career first.")?;
    readers::you(w).ok_or("Start a career first.")?;
    let i = super::super::place(w, id).ok_or("No such player.")?;
    super::time::can_watch(w, mode)?;
    let ours = is_ours(w, id);
    let tape = if ours { tape::our_tape(w, id)? } else { tape::player_tape(w, id)? };
    let cut = if ours && cut == Cut::Ours { Cut::Ours } else { cut };
    let n = if mode == Mode::Quick { 12 } else { 6 };
    // From the stands (SK32): his snaps of tonight's game, in order.
    let picks = if mode == Mode::Live {
        (0..tape.snaps.len())
            .filter(|&k| tape.snaps[k].game == 0 && tape.snaps[k].role != Role::Away)
            .take(10)
            .collect()
    } else {
        tape::cut(&tape, cut, n)
    };
    // Watch it quick takes whatever cut-up his tape has.
    let (cut, picks) = if picks.is_empty() && mode == Mode::Quick {
        Cut::ALL.iter().map(|&c| (c, tape::cut(&tape, c, n))).find(|x| !x.1.is_empty()).unwrap_or((cut, picks))
    } else {
        (cut, picks)
    };
    if picks.is_empty() {
        return Err("He has no snaps of that kind on his tape: try another cut-up.".into());
    }
    let all = tape::cut(&tape, cut, 99).len();
    let p = &w.people[i];
    let (name, pos) = (p.name.clone(), p.player.position);
    let qs = questions(w, id);
    let ask = match ask {
        Some(k) => qs.iter().find(|q| q.key == k).cloned().or_else(|| qs.first().cloned()),
        None => qs.first().cloned(),
    };
    let mut snaps: Vec<Snap> = picks.iter().map(|&k| tape.snaps[k].clone()).collect();
    // The tape stops on about one snap in three, the ones that bear on the
    // question (SK48).
    if mode == Mode::Quick {
        for s in &mut snaps {
            s.call = None;
        }
    } else {
        let qa: Vec<u8> = ask.as_ref().map(|q| q.attrs.clone()).unwrap_or_default();
        let bears = |s: &Snap| s.cues.iter().any(|c| qa.contains(&(c.0.index() as u8)));
        let want = snaps.len().div_ceil(3).max(1);
        let mut order: Vec<usize> = (0..snaps.len()).filter(|&k| snaps[k].call.is_some()).collect();
        order.sort_by_key(|&k| if bears(&snaps[k]) { 0 } else { 1 });
        let keep: Vec<usize> = order.into_iter().take(want).collect();
        for (k, s) in snaps.iter_mut().enumerate() {
            if !keep.contains(&k) {
                s.call = None;
            }
        }
    }
    let (coach, analyst) = if mode == Mode::Full { (room_coach(w, pos), room_analyst(w)) } else { (None, None) };
    let numbers = analyst.as_ref().map_or(String::new(), |(nm, _)| numbers_line(&tape, cut, pos, nm));
    let len = snaps.len();
    let s = Session {
        target: Target::Player(id),
        cut,
        mode,
        date: w.today,
        name: name.clone(),
        pos,
        ask,
        games: tape.games.clone(),
        snaps,
        cut_n: all,
        at: 0,
        step: 0,
        picks: vec![None; len],
        seen: vec![Vec::new(); len],
        his: vec![Vec::new(); len],
        marks: vec![None; len],
        tired: vec![false; len],
        minutes: 0.0,
        coach,
        analyst,
        talk: Vec::new(),
        numbers,
        hints: Vec::new(),
        tape_key: None,
        salt: w.scouting.film.version as u64 ^ (w.scouting.film.reports.len() as u64) << 20,
    };
    let _ = staff;
    w.scouting.film.open = Some(s);
    if mode == Mode::Quick {
        return quick(w);
    }
    Ok(format!("{}: {} snaps of {name}.", cut.words(pos), len))
}

/// Opens a team's tape (SK54): snaps built by the game's film room from a
/// kept tape of your next opponent, with the hints it holds.
#[allow(clippy::too_many_arguments)]
pub fn open_team(
    w: &mut World,
    team: u16,
    name: String,
    games: Vec<Game>,
    snaps: Vec<Snap>,
    hints: Hints,
    tape_key: (Date, usize, usize),
    mode: Mode,
) -> Result<String, String> {
    readers::you(w).ok_or("Start a career first.")?;
    super::time::can_watch(w, mode)?;
    if snaps.is_empty() {
        return Err("That tape has no snaps worth a cut-up.".into());
    }
    let len = snaps.len();
    let coach = if mode == Mode::Full { room_coordinator(w) } else { None };
    let analyst = if mode == Mode::Full { room_analyst(w) } else { None };
    let s = Session {
        target: Target::Team(team),
        cut: Cut::ThirdDowns,
        mode,
        date: w.today,
        name: name.clone(),
        pos: Position::QB,
        ask: Some(Question {
            key: "opp".into(),
            text: format!("What are {name} running when it matters?"),
            why: "Their habits on the downs that decide games.".into(),
            attrs: Vec::new(),
            bar: 0.0,
        }),
        games,
        snaps,
        cut_n: len,
        at: 0,
        step: 0,
        picks: vec![None; len],
        seen: vec![Vec::new(); len],
        his: vec![Vec::new(); len],
        marks: vec![None; len],
        tired: vec![false; len],
        minutes: 0.0,
        coach,
        analyst,
        talk: Vec::new(),
        numbers: String::new(),
        hints,
        tape_key: Some(tape_key),
        salt: w.scouting.film.version as u64 ^ 0x7EA4,
    };
    w.scouting.film.open = Some(s);
    if mode == Mode::Quick {
        return quick(w);
    }
    Ok(format!("{name}'s tape: {len} snaps."))
}

/// For a team's tape the coordinator of the side that faces it sits in.
fn room_coordinator(w: &World) -> Option<Room> {
    let you = readers::you(w);
    let c = match readers::seat_staff(w)? {
        Staff::Program(p) => w.programs[p].dc.or(w.programs[p].oc),
        Staff::Club(t) => w.nfl[t].dc.or(w.nfl[t].oc),
        Staff::School(_) => None,
    }
    .filter(|&c| Some(c) != you)?;
    Some(Room {
        coach: Some(c),
        name: w.coaches[c].name.clone(),
        job: "Coordinator".into(),
        eye: w.coaches[c].evaluation(),
    })
}

/// The analyst's numbers for the cut-up (SK57).
fn numbers_line(t: &tape::Tape, cut: Cut, pos: Position, who: &str) -> String {
    let ks = tape::cut(t, cut, 99);
    let mut s = tape::Stat::default();
    for &k in &ks {
        let x = t.snaps[k].stat;
        s.targets += x.targets;
        s.catches += x.catches;
        s.yards += x.yards;
        s.carries += x.carries;
        s.att += x.att;
        s.comp += x.comp;
        s.sacks += x.sacks;
        s.pressures += x.pressures;
        s.reps += x.reps;
        s.wins += x.wins;
        s.tackles += x.tackles;
        s.missed += x.missed;
        s.pbu += x.pbu;
        s.ints += x.ints;
        s.td += x.td;
    }
    let first = who.split_whitespace().last().unwrap_or(who);
    let what = cut.words(pos).to_lowercase();
    let body = match pos {
        Position::QB => {
            format!("{} of {} for {} yards, {} sacks, {} interceptions", s.comp, s.att, s.yards, s.sacks, s.ints)
        }
        Position::RB => {
            format!("{} carries, {} yards, {} targets, {} caught", s.carries, s.yards, s.targets, s.catches)
        }
        Position::WR | Position::TE => format!("{} targets, {} caught, {} yards", s.targets, s.catches, s.yards),
        Position::OT | Position::IOL => format!("{} reps, {} lost or pressures allowed", s.reps, s.pressures),
        Position::EDGE | Position::DT => {
            format!("{} rush reps, {} pressures, {} sacks, {} tackles", s.reps, s.pressures, s.sacks, s.tackles)
        }
        Position::CB | Position::S | Position::LB => format!(
            "{} targets, {} caught for {} yards, {} broken up, {} tackles, {} missed",
            s.targets, s.catches, s.yards, s.pbu, s.tackles, s.missed
        ),
        _ => format!("{} snaps", ks.len()),
    };
    format!("{first}: “{} on {what}: {body}.”", ks.len().to_string() + " snaps")
}

// --- Watching -----------------------------------------------------------------------------------

fn take(w: &mut World) -> Result<Session, String> {
    w.scouting.film.open.take().ok_or_else(|| "No film is open.".to_string())
}

fn put(w: &mut World, s: Session) {
    w.scouting.film.open = Some(s);
}

/// Your eye on this session's player (SK43, SK63): your evaluation, what
/// your hours at his position taught you, less a long day's tiredness.
fn eye(w: &World, pos: Position) -> f64 {
    let base = super::super::yours::your_eye(w).unwrap_or(10.0);
    base + super::time::position_edge(w, pos)
}

fn roll(s: &Session, w: &World, snap: usize, k: usize, salt: u64) -> f64 {
    let id = match s.target {
        Target::Player(id) => id as u64,
        Target::Team(t) => 0x7EA0_0000 | t as u64,
    };
    key_uniform(
        w.options.seed as u64 ^ (s.date.0 as u64) << 32 ^ s.salt << 8 ^ salt,
        id ^ (snap as u64) << 40 ^ (k as u64) << 52,
    )
}

/// Plays the snap (the first two stills, or to the stop for a call).
pub fn play(w: &mut World) -> Result<(), String> {
    let mut s = take(w)?;
    if s.step != 0 {
        put(w, s);
        return Err("Play the next snap first.".into());
    }
    if s.mode == Mode::Full {
        if let Err(e) = super::time::spend(w, SNAP_MIN / 60.0, s.pos) {
            put(w, s);
            return Err(e);
        }
        s.minutes += SNAP_MIN;
    } else if s.mode == Mode::Live {
        s.minutes += SNAP_MIN;
    }
    if s.snaps[s.at].call.is_some() {
        s.step = 1;
        put(w, s);
        return Ok(());
    }
    resolve(w, &mut s);
    put(w, s);
    Ok(())
}

/// Your call on the stopped snap (SK48).
pub fn call(w: &mut World, pick: u8) -> Result<(), String> {
    let mut s = take(w)?;
    if s.step != 1 {
        put(w, s);
        return Err("The tape isn't stopped for a call.".into());
    }
    let at = s.at;
    s.picks[at] = Some(pick);
    if let (Some(c), Some(r)) = (s.snaps[at].call.clone(), s.coach.clone()) {
        let right = pick == c.right;
        let first = first_name(&r.name);
        let line = if right {
            match roll(&s, w, at, 9, 0xCA11) {
                x if x < 0.5 => format!("{first} nods: “You're seeing him.”"),
                _ => format!("{first}: “Yep. {}”", cap(&c.truth)),
            }
        } else {
            format!("{first}: “Not this time, Coach: {}”", c.truth)
        };
        s.talk.push((at as u8, r.name.clone(), line));
    }
    resolve(w, &mut s);
    put(w, s);
    Ok(())
}

fn first_name(n: &str) -> String {
    n.split_whitespace().last().unwrap_or(n).to_string()
}

fn cap(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// What your eye catches on the snap as it plays out (SK43), what the
/// coach and the analyst say (SK50, SK57), how tired you are (SK59).
fn resolve(w: &mut World, s: &mut Session) {
    let at = s.at;
    let snap = s.snaps[at].clone();
    let e = eye(w, s.pos);
    let t = super::time::tiredness(w);
    s.tired[at] = t > 0.25;
    let qa: Vec<u8> = s.ask.as_ref().map(|q| q.attrs.clone()).unwrap_or_default();
    let mut seen = Vec::with_capacity(snap.cues.len().max(1));
    let mut his = Vec::with_capacity(snap.cues.len().max(1));
    let n_cues = if matches!(s.target, Target::Team(_)) { 1 } else { snap.cues.len() };
    for k in 0..n_cues {
        let (subtle, on_q) = match s.target {
            Target::Player(_) => {
                let a = snap.cues[k].0;
                (words::subtle(a), qa.contains(&(a.index() as u8)))
            }
            // A team's habit: your call shows it, else your eye.
            Target::Team(_) => (0.55, true),
        };
        let called_right =
            matches!(s.target, Target::Team(_)) && snap.call.as_ref().is_some_and(|c| s.picks[at] == Some(c.right));
        let p_clear = clamp(0.30 + 0.035 * (e - 10.0) - 0.40 * subtle + if on_q { 0.12 } else { 0.0 }, 0.04, 0.95)
            * (1.0 - 0.5 * t);
        let p_half = if s.mode == Mode::Quick { 0.0 } else { 0.35 * (1.0 - 0.6 * t) };
        let u = roll(s, w, at, k, 0x5EE0);
        let v = if called_right || u < p_clear {
            CLEAR
        } else if u < p_clear + p_half {
            if s.mode == Mode::Live { MOVED_ON } else { HALF }
        } else {
            SLIPPED
        };
        seen.push(v);
        // The coach in the room saw it with his own eye.
        let ce = s.coach.as_ref().map_or(0.0, |c| c.eye);
        let pc = clamp(0.40 + 0.035 * (ce - 10.0) - 0.30 * subtle, 0.05, 0.95);
        his.push(s.coach.is_some() && roll(s, w, at, k, 0xC0AC) < pc);
    }
    s.seen[at] = seen;
    s.his[at] = his;
    // The room talks.
    if let Some(r) = s.coach.clone() {
        let first = first_name(&r.name);
        let line = match s.target {
            Target::Player(_) => coach_line(s, at, &first),
            Target::Team(_) => {
                let c = snap.call.as_ref();
                let truth = c.map(|c| c.opts[c.right as usize].clone()).unwrap_or_default();
                if s.his[at].first() == Some(&true) {
                    format!("{first}: “{truth}. Quality control had it too.”")
                } else {
                    format!("{first}: “{truth}, I think. Let's see it again on another tape.”")
                }
            }
        };
        if !line.is_empty() {
            s.talk.push((at as u8, r.name.clone(), line));
        }
    }
    if let Some((nm, rating)) = s.analyst.clone()
        && matches!(s.target, Target::Player(_))
        && snap.role != Role::Away
    {
        let first = first_name(&nm);
        let st = snap.stat;
        let line = if st.targets > 0 {
            format!(
                "{first}: “{} on this one. {}”",
                if st.catches > 0 { "Caught" } else { "Not caught" },
                s.numbers_tail(rating)
            )
        } else if st.reps > 0 {
            format!(
                "{first}: “{} on this rep. {}”",
                if st.pressures > 0 { "A loss" } else { "A win" },
                s.numbers_tail(rating)
            )
        } else if st.carries > 0 {
            format!("{first}: “{} yards on the carry. {}”", st.yards, s.numbers_tail(rating))
        } else {
            String::new()
        };
        if !line.is_empty() {
            s.talk.push((at as u8, nm.clone(), line));
        }
    }
    s.step = 2;
}

impl Session {
    /// The analyst's tail: where the numbers and the tape disagree (SK6,
    /// SK57); a sharper analyst says more.
    fn numbers_tail(&self, rating: u8) -> String {
        let watched: Vec<&Snap> = self.snaps.iter().take(self.at + 1).collect();
        let g: i32 = watched.iter().map(|s| s.grade as i32).sum();
        let st: i32 = watched
            .iter()
            .map(|s| {
                let x = s.stat;
                -((x.catches as i32 > 0 && matches!(s.role, Role::Cover)) as i32)
                    + (x.pbu + x.ints) as i32
                    + x.wins as i32
                    - x.pressures as i32 * (matches!(s.role, Role::PassPro | Role::RunBlock) as i32)
            })
            .sum();
        if rating >= 13 && g.signum() != st.signum() && st != 0 {
            if st < g {
                "The numbers like him less than the tape does.".into()
            } else {
                "The numbers like him more than the tape does.".into()
            }
        } else if rating >= 9 {
            "The numbers agree with the tape so far.".into()
        } else {
            String::new()
        }
    }
}

/// The coach's word on a snap (SK50): what he saw, and when you caught
/// something he missed.
fn coach_line(s: &Session, at: usize, first: &str) -> String {
    let snap = &s.snaps[at];
    let seen = &s.seen[at];
    let his = &s.his[at];
    // You caught it, he didn't.
    if let Some(k) = (0..seen.len()).find(|&k| seen[k] == CLEAR && !his[k]) {
        let a = snap.cues[k].0;
        return format!("{first}: “I didn't have that one. Good eye on his {}.”", words::short(a));
    }
    // He points out what he saw.
    if let Some(k) = (0..his.len()).find(|&k| his[k]) {
        let (a, b) = snap.cues[k];
        let line = words::cue(a, b as usize);
        return match b {
            3 | 4 => format!("{first}: “{line} That's what I like.”"),
            0 | 1 => format!("{first}: “{line} We can coach that, but it's there.”"),
            _ => format!("{first}: “{line}”"),
        };
    }
    String::new()
}

/// A half-seen moment: run it back (SK49).
pub fn runback(w: &mut World) -> Result<(), String> {
    let mut s = take(w)?;
    let at = s.at;
    let Some(k) = s.seen.get(at).and_then(|v| v.iter().position(|&x| x == HALF)) else {
        put(w, s);
        return Err("Nothing to run back on this snap.".into());
    };
    if let Err(e) = super::time::spend(w, RUNBACK_MIN / 60.0, s.pos) {
        put(w, s);
        return Err(e);
    }
    s.minutes += RUNBACK_MIN;
    let e = eye(w, s.pos);
    let t = super::time::tiredness(w);
    let p = clamp(0.60 + 0.03 * (e - 10.0), 0.30, 0.97) * (1.0 - 0.3 * t);
    let got = roll(&s, w, at, k, 0x2BAC) < p;
    s.seen[at][k] = if got { REPLAYED } else { REPLAY_MISSED };
    if got && let (Some(r), Target::Player(_)) = (s.coach.clone(), s.target) {
        let first = first_name(&r.name);
        let line = if s.his[at].get(k) == Some(&true) {
            format!("{first}: “You saw it too. Good.”")
        } else {
            format!("{first}: “Huh. I missed that the first time.”")
        };
        s.talk.push((at as u8, r.name.clone(), line));
    }
    put(w, s);
    Ok(())
}

/// Move on and keep the time (SK49).
pub fn move_on(w: &mut World) -> Result<(), String> {
    let mut s = take(w)?;
    let at = s.at;
    for x in s.seen[at].iter_mut() {
        if *x == HALF {
            *x = MOVED_ON;
        }
    }
    put(w, s);
    Ok(())
}

/// Your mark on the snap (SK44); your coach pushes back when it goes
/// against his grade (SK50).
pub fn mark(w: &mut World, tag: u8) -> Result<(), String> {
    let mut s = take(w)?;
    let at = s.at;
    if tag as usize >= TAGS.len() || s.step != 2 {
        put(w, s);
        return Err("Mark a snap once it has played.".into());
    }
    s.marks[at] = if s.marks[at] == Some(tag) { None } else { Some(tag) };
    if let (Some(r), Target::Player(_)) = (s.coach.clone(), s.target)
        && tag <= 1
        && s.marks[at] == Some(tag)
    {
        let g = coach_grade(&s, w, at);
        let first = first_name(&r.name);
        if tag == 0 && g < 0 {
            s.talk.push((at as u8, r.name.clone(), format!("{first}: “I had that one as a minus, Coach.”")));
        } else if tag == 1 && g > 0 {
            s.talk.push((at as u8, r.name.clone(), format!("{first}: “I'd give him that one. He did his job.”")));
        }
    }
    put(w, s);
    Ok(())
}

/// The coach's grade of a snap: the engine's, read through his eye.
fn coach_grade(s: &Session, w: &World, at: usize) -> i8 {
    let g = s.snaps[at].grade;
    let ce = s.coach.as_ref().map_or(10.0, |c| c.eye);
    let miss = clamp(0.25 - 0.012 * (ce - 10.0), 0.03, 0.4);
    if roll(s, w, at, 7, 0x96AD) < miss { if g == 0 { 1 } else { 0 } } else { g }
}

/// On to the next snap, or your verdict.
pub fn next(w: &mut World) -> Result<(), String> {
    let mut s = take(w)?;
    if s.step != 2 {
        put(w, s);
        return Err("Play this snap first.".into());
    }
    for x in s.seen[s.at].iter_mut() {
        if *x == HALF {
            *x = MOVED_ON;
        }
    }
    if s.at + 1 < s.snaps.len() {
        s.at += 1;
        s.step = 0;
    } else {
        s.step = 3;
    }
    put(w, s);
    Ok(())
}

/// Your verdict (SK47): 0 yes, 1 no, 2 not sure; the session ends and
/// becomes your report.
pub fn verdict(w: &mut World, answer: u8) -> Result<String, String> {
    let s = take(w)?;
    if s.step != 3 {
        put(w, s);
        return Err("Watch the cut-up first.".into());
    }
    Ok(finish(w, s, Some(answer.min(2))))
}

/// Leave the film room: what you watched still counts.
pub fn close(w: &mut World) -> Result<String, String> {
    let s = take(w)?;
    let watched = s.seen.iter().filter(|v| !v.is_empty()).count();
    if watched == 0 {
        return Ok("Closed before a snap was played.".into());
    }
    Ok(finish(w, s, None))
}

/// "Watch it quick" (SK53): the cut-up for twenty minutes, what your eye
/// catches the first time through, no calls, replays or coach talk.
fn quick(w: &mut World) -> Result<String, String> {
    let mut s = take(w)?;
    super::time::spend(w, QUICK_MIN / 60.0, s.pos)?;
    s.minutes = QUICK_MIN;
    for k in 0..s.snaps.len() {
        s.at = k;
        resolve(w, &mut s);
    }
    s.step = 3;
    Ok(finish(w, s, None))
}

/// The session ends (SK58): catches into your own read, the looks into
/// his file (yours, and your coach's notes into your staff's), the calls
/// into your coach's respect for your eye, the tags carried forward, the
/// report kept, the hours counted (SK63), and maybe a kid off the tape
/// (SK34).
fn finish(w: &mut World, s: Session, verdict: Option<u8>) -> String {
    let d = w.today;
    let you = readers::you(w).map(|c| COACH + c as u32);
    let e = eye(w, s.pos);
    let mut caught_lines = Vec::new();
    let mut found_hints: Vec<String> = Vec::new();
    let mut catches: Vec<Catch> = Vec::new();
    let watched = s.seen.iter().filter(|v| !v.is_empty()).count();
    match s.target {
        Target::Player(id) => {
            if let Some(i) = super::super::place(w, id) {
                let (c, lines) = caught(w, &s);
                catches.extend(c);
                caught_lines.extend(lines);
                // The look in his file under your name; your coach's notes
                // in your staff's file (his read, not yours).
                let kind = film_kind(&w.people[i]);
                // From the stands the trip already put the game in his
                // file; tape is a film look under your name.
                if let Some(you) = you
                    && s.mode != Mode::Live
                {
                    super::super::dept::add(w, id, you, kind, d);
                }
                super::queue::watched(w, id);
                if let Some(Staff::Club(c)) = readers::seat_staff(w)
                    && w.nfl.get(c).and_then(|x| x.head_coach) == readers::you(w)
                {
                    super::super::dept::club_look(
                        w,
                        c,
                        id,
                        crate::world::pro::scouts::HC,
                        crate::world::pro::scouts::LookKind::Film,
                        d,
                    );
                }
                if let Some(r) = &s.coach
                    && let Some(c) = r.coach
                {
                    match readers::seat_staff(w) {
                        Some(Staff::Club(t)) => {
                            let role = if w.nfl[t].oc == Some(c) {
                                crate::world::pro::scouts::OC
                            } else {
                                crate::world::pro::scouts::DC
                            };
                            super::super::dept::club_look(w, t, id, role, crate::world::pro::scouts::LookKind::Film, d);
                        }
                        _ => super::super::dept::add(w, id, COACH + c as u32, kind, d),
                    }
                }
                w.scouting.dept.asks.retain(|a| a.0 != id || a.1 != super::super::Kind::Film);
                // Found on another kid's tape (SK34).
                let others: Vec<u32> = s
                    .snaps
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| !s.seen[*k].is_empty())
                    .flat_map(|(_, sn)| sn.others.iter().copied())
                    .collect();
                if let Some(f) = super::finds::pop(w, id, &others, watched, e) {
                    caught_lines.push(f);
                }
            }
        }
        Target::Team(_) => {
            // A hint is yours once you caught the habit on one of its snaps.
            for (key, text, ks) in &s.hints {
                if ks
                    .iter()
                    .any(|&k| s.seen.get(k as usize).is_some_and(|v| v.contains(&CLEAR) || v.contains(&REPLAYED)))
                {
                    found_hints.push(key.clone());
                    caught_lines.push(text.clone());
                }
            }
        }
    }
    // Your calls with your coach in the room earn his respect for your
    // eye (SK48).
    let made = s.picks.iter().filter(|x| x.is_some()).count() as u8;
    let right = s
        .picks
        .iter()
        .zip(&s.snaps)
        .filter(|(p, sn)| p.is_some() && sn.call.as_ref().is_some_and(|c| Some(c.right) == **p))
        .count() as u8;
    if let Some(r) = &s.coach
        && let Some(c) = r.coach
        && made > 0
    {
        let by = (right as f32 - (made - right) as f32 * 0.6) * 0.006;
        w.folk.mine.nudge(crate::world::folk::coach_key(c), by.clamp(-0.03, 0.03));
    }
    // Tags carried forward (SK44): "ask him" to the interview room, "show
    // the staff" to the board meetings.
    if let Target::Player(id) = s.target {
        for (k, m) in s.marks.iter().enumerate() {
            if let Some(m) = *m
                && m >= 2
            {
                w.scouting.film.tags.push(Tag { id, date: d, kind: m, text: s.snaps[k].chart.clone(), used: false });
            }
        }
        let n = w.scouting.film.tags.len();
        if n > 400 {
            w.scouting.film.tags.drain(..n - 400);
        }
    }
    // The coach's answer to the question, from his read.
    let coach_answer = match (&s.target, &s.ask, &s.coach) {
        (Target::Player(id), Some(q), Some(_)) => super::super::place(w, *id).map(|i| {
            let p = &w.people[i];
            let m: f64 = q.attrs.iter().map(|&a| p.player.attributes.get(Attr::ALL[a as usize])).sum::<f64>()
                / q.attrs.len().max(1) as f64;
            let lean = key_normal(0xC0A5 ^ (d.0 as u64), *id as u64) * 4.0;
            let v = m + lean - q.bar as f64;
            if v >= 2.0 {
                0
            } else if v <= -2.0 {
                1
            } else {
                2
            }
        }),
        _ => None,
    };
    let keep_stills = verdict.is_some();
    let report = Report {
        target: s.target,
        name: s.name.clone(),
        date: d,
        cut: s.cut,
        mode: s.mode,
        ask: s.ask.clone(),
        verdict,
        settled: None,
        calls: (right, made),
        caught: caught_lines.clone(),
        marks: s
            .marks
            .iter()
            .enumerate()
            .filter_map(|(k, m)| m.map(|m| (k as u8, m, s.snaps[k].chart.clone())))
            .collect(),
        minutes: s.minutes.round() as u16,
        coach: s.coach.as_ref().map(|r| r.name.clone()),
        analyst: s.analyst.as_ref().map(|a| a.0.clone()),
        hints: found_hints,
        snaps: if keep_stills {
            s.snaps
                .iter()
                .enumerate()
                .filter(|(k, _)| !s.seen[*k].is_empty())
                .map(|(_, sn)| Kept {
                    sit: sn.sit.clone(),
                    job: sn.job.clone(),
                    result: sn.result.clone(),
                    stills: sn.stills.clone(),
                })
                .collect()
        } else {
            Vec::new()
        },
        watched: watched as u8,
        numbers: s.numbers.clone(),
        coach_answer,
        tape_key: s.tape_key,
    };
    let fr = &mut w.scouting.film;
    fr.catches.extend(catches);
    if fr.catches.len() > 6000 {
        let n = fr.catches.len();
        fr.catches.drain(..n - 6000);
    }
    fr.reports.push(report);
    // Stills only for the newest 60 reports with a verdict.
    let with: Vec<usize> = (0..fr.reports.len()).filter(|&k| !fr.reports[k].snaps.is_empty()).collect();
    if with.len() > 60 {
        for &k in &with[..with.len() - 60] {
            fr.reports[k].snaps.clear();
        }
    }
    if fr.reports.len() > 400 {
        let n = fr.reports.len();
        fr.reports.drain(..n - 400);
    }
    fr.version = fr.version.wrapping_add(1);
    let n = caught_lines.len();
    match s.mode {
        Mode::Quick => {
            format!("Watched quick: your eye caught {n} thing{} the first time through.", if n == 1 { "" } else { "s" })
        }
        _ => format!(
            "Saved to his file: {watched} snaps, {right} of {made} calls right, {n} thing{} caught.",
            if n == 1 { "" } else { "s" }
        ),
    }
}

/// The look kind a player's film goes in his file as.
pub fn film_kind(p: &crate::world::Person) -> super::super::Kind {
    use crate::world::Stage;
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => super::super::Kind::HsFilm,
        Stage::College | Stage::Declared => super::super::Kind::CollegeFilm,
        _ => super::super::Kind::ProFilm,
    }
}

/// What your eye (and your analyst beside you) caught on the snaps
/// watched: the catches and their lines. Pure, so the film room can
/// show your read narrowing as you watch (SK52).
pub fn caught(w: &World, s: &Session) -> (Vec<Catch>, Vec<String>) {
    let mut catches: Vec<Catch> = Vec::new();
    let mut caught_lines: Vec<String> = Vec::new();
    let Target::Player(id) = s.target else { return (catches, caught_lines) };
    let Some(i) = super::super::place(w, id) else { return (catches, caught_lines) };
    let d = w.today;
    let e = eye(w, s.pos);
    let t = super::time::tiredness(w);
    let p = &w.people[i];
    let sd_c = (3.5 * file::sharp(e) * (1.0 + 0.3 * t)) as f32;
    for (k, snap) in s.snaps.iter().enumerate() {
        if s.seen[k].is_empty() {
            continue;
        }
        for (j, &(a, b)) in snap.cues.iter().enumerate() {
            let v = s.seen[k].get(j).copied().unwrap_or(0);
            if v == CLEAR || v == REPLAYED {
                let noise = key_normal(
                    w.options.seed as u64 ^ 0xCA7C ^ (d.0 as u64) << 24,
                    id as u64 ^ (k as u64) << 40 ^ (j as u64) << 50,
                );
                let truth = p.player.attributes.get(a);
                catches.push(Catch {
                    id,
                    attr: a.index() as u8,
                    v: (truth + noise * sd_c as f64) as f32,
                    sd: sd_c,
                    date: d,
                    by: 0,
                });
                caught_lines.push(words::cue(a, b as usize).to_string());
            }
        }
        // A weak eye mostly sees the result (SK43): one wide,
        // result-leaning impression of his main skill a snap.
        if snap.role != Role::Away
            && let Some(&(a, _)) = snap.cues.first()
        {
            let noise = key_normal(w.options.seed as u64 ^ 0x2E5u64 ^ (d.0 as u64) << 24, id as u64 ^ (k as u64) << 40);
            let truth = p.player.attributes.get(a);
            catches.push(Catch {
                id,
                attr: a.index() as u8,
                v: (truth + 4.0 * snap.grade as f64 + 6.0 * noise) as f32,
                sd: 8.0,
                date: d,
                by: 0,
            });
        }
        // The analyst's points count toward your own read too
        // (SK57): a top analyst beside you can take it all the
        // way, for what the tape shows.
        if let Some((_, r)) = &s.analyst {
            let r = *r as f64;
            let f = super::super::footage::footage(w, i);
            let sd_a = (6.0 * (1.5 - 0.065 * r).max(0.15)) as f32;
            let pa = clamp(0.35 + 0.04 * (r - 10.0), 0.1, 0.95);
            for (j, &(a, _)) in snap.cues.iter().enumerate() {
                if !super::super::footage::numbers_see(&f, a, p.player.position)
                    && !matches!(snap.role, Role::Cover | Role::Target | Role::PassPro | Role::Rusher)
                {
                    continue;
                }
                let u = key_uniform(
                    w.options.seed as u64 ^ 0xA7A1 ^ (d.0 as u64) << 24,
                    id as u64 ^ (k as u64) << 40 ^ (j as u64) << 50,
                );
                if u < pa {
                    let noise = key_normal(
                        w.options.seed as u64 ^ 0xA7A2 ^ (d.0 as u64) << 24,
                        id as u64 ^ (k as u64) << 40 ^ (j as u64) << 50,
                    );
                    catches.push(Catch {
                        id,
                        attr: a.index() as u8,
                        v: (p.player.attributes.get(a) + noise * sd_a as f64) as f32,
                        sd: sd_a,
                        date: d,
                        by: 1,
                    });
                }
            }
        }
    }
    (catches, caught_lines)
}
