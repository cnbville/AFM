//! Scenes (people topic 5, takeover item 1): a meeting, a talk, a press
//! conference or a call, played as a scene. The person, where you are, what
//! he says, and your answers picked as moves, over two to five exchanges.
//! Each kind of scene lives in its own module; this one keeps them, opens
//! them on their day, hands the ones you skip to your staff, and stops
//! Continue on the morning of a meeting that waits for you.

use super::{Team, voice};
use crate::calendar::{Date, Loudness};
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    // Taking over (T1-T23).
    TeamMeeting,
    PressIntro,
    BossFirst,
    Holdover,
    StaffReport,
    OneOnOne,
    Recruiting,
    Academic,
    Trainer,
    Booster,
    Media,
    Tour,
    Plan,
    Parents,
    Feeder,
    HireReaction,
    OldCoach,
    // The rest of a career.
    Press,
    TeamTalk,
    BossMeeting,
    StaffLine,
    Interview,
    Firing,
    Discipline,
    ReachOut,
    Trip,
    StarPower,
    SecondOpinion,
    Ritual,
    ParentCall,
    Proposal,
    JobCall,
    Leaders,
    FlightRisks,
    Gm,
    Family,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::TeamMeeting => "Team meeting",
            Kind::PressIntro => "Introductory press conference",
            Kind::BossFirst => "Meeting with your boss",
            Kind::Holdover => "Staff meeting",
            Kind::StaffReport => "The state of the team",
            Kind::OneOnOne => "One-on-one",
            Kind::Recruiting => "Recruiting meeting",
            Kind::Academic => "Academic and conduct report",
            Kind::Trainer => "Trainer's report",
            Kind::Booster => "Booster meeting",
            Kind::Media => "The beat reporters",
            Kind::Tour => "Program tour",
            Kind::Plan => "The first weeks' plan",
            Kind::Parents => "Parents' meeting",
            Kind::Feeder => "Feeder tour",
            Kind::HireReaction => "How the world took your hire",
            Kind::OldCoach => "The old coach",
            Kind::Press => "Press conference",
            Kind::TeamTalk => "Team talk",
            Kind::BossMeeting => "Meeting with your boss",
            Kind::StaffLine => "Staff talk",
            Kind::Interview => "Interview",
            Kind::Firing => "Letting a coach go",
            Kind::Discipline => "Discipline",
            Kind::ReachOut => "Reaching out",
            Kind::Trip => "On the road",
            Kind::StarPower => "Star player",
            Kind::SecondOpinion => "Second opinion",
            Kind::Ritual => "Rookie ritual",
            Kind::ParentCall => "A parent calls",
            Kind::Proposal => "Staff proposal",
            Kind::JobCall => "A call",
            Kind::Leaders => "The leaders of the room",
            Kind::FlightRisks => "Who might leave",
            Kind::Gm => "The general manager",
            Kind::Family => "At home",
        }
    }
}

/// Who is in the scene, for the screens' links and faces.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Who {
    Player(u32),
    Coach(usize),
    /// A cast person by link token ("college:12:Booster:0").
    Cast(String),
    Group(String),
}

/// One line said in a scene: by the scene's narration (0), you (1), the
/// other side (2) or someone else in the room (3); a tell your read catches.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Said {
    pub by: u8,
    pub name: String,
    pub text: String,
    pub tell: Option<String>,
}

/// An answer you can pick: what you say, the move and tone it is, and what
/// it tends to do (people topic 5; inbox item 10i).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Pick {
    pub label: String,
    pub mv: String,
    pub tone: String,
    pub hint: String,
    pub code: u32,
}

/// A table beside a scene (a report): column heads and rows of cells with
/// link marks; a row about a player carries his id, so the screens can add
/// your staff's read.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Doc {
    pub title: String,
    pub note: String,
    pub cols: Vec<String>,
    pub rows: Vec<DocRow>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DocRow {
    pub pid: Option<u32>,
    pub cells: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Scene {
    pub id: u32,
    pub kind: Kind,
    pub date: Date,
    pub title: String,
    /// Where and when ("Ross-Ade Pavilion, the team room, 7:30 p.m.").
    pub place: String,
    pub who: Vec<Who>,
    pub lines: Vec<Said>,
    pub picks: Vec<Pick>,
    pub docs: Vec<Doc>,
    pub step: u8,
    pub done: bool,
    /// Continue stops on its morning (a dated meeting), or it waits for you
    /// without stopping (a press availability, a call).
    pub stops: bool,
    pub opened: bool,
    /// What changed, in words, once it ends.
    pub outcome: Vec<String>,
    /// Handed to your staff (who), when you didn't hold it yourself.
    pub handed: Option<String>,
    /// The scene's own state: the person or thing it is about and what has
    /// happened so far (subject, running effects).
    pub a: u64,
    pub b: u64,
    pub c: i64,
    pub f: [f32; 4],
    pub team: Option<Team>,
}

impl Scene {
    pub fn new(kind: Kind, date: Date, title: impl Into<String>) -> Scene {
        Scene {
            id: 0,
            kind,
            date,
            title: title.into(),
            place: String::new(),
            who: Vec::new(),
            lines: Vec::new(),
            picks: Vec::new(),
            docs: Vec::new(),
            step: 0,
            done: false,
            stops: true,
            opened: false,
            outcome: Vec::new(),
            handed: None,
            a: 0,
            b: 0,
            c: 0,
            f: [0.0; 4],
            team: None,
        }
    }

    pub fn narrate(&mut self, text: impl Into<String>) {
        self.lines.push(Said { by: 0, name: String::new(), text: text.into(), tell: None });
    }
    pub fn you(&mut self, name: &str, text: impl Into<String>) {
        self.lines.push(Said { by: 1, name: name.to_string(), text: text.into(), tell: None });
    }
    pub fn them(&mut self, name: &str, text: impl Into<String>, tell: Option<String>) {
        self.lines.push(Said { by: 2, name: name.to_string(), text: text.into(), tell });
    }
    pub fn other(&mut self, name: &str, text: impl Into<String>) {
        self.lines.push(Said { by: 3, name: name.to_string(), text: text.into(), tell: None });
    }
    pub fn pick(&mut self, code: u32, label: impl Into<String>, mv: &str, tone: &str, hint: &str) {
        self.picks.push(Pick { label: label.into(), mv: mv.into(), tone: tone.into(), hint: hint.into(), code });
    }
    pub fn end(&mut self) {
        self.done = true;
        self.picks.clear();
    }
}

/// A seeded pen for one step of one scene: the same scene replays the same.
/// Put it back with `pen.put(w)` when the step is written.
pub fn pen(w: &mut World, s: &Scene, k: u64) -> voice::Pen {
    let seed = (w.folk.base as u64) << 20 ^ (s.id as u64) << 8 ^ s.step as u64 ^ k << 40;
    voice::Pen::take(w, seed)
}

/// Adds a scene for you: it gets its id and, when it stops Continue, a line
/// on the calendar waiting on your answer.
pub fn add(w: &mut World, mut s: Scene) -> u32 {
    w.folk.next_scene += 1;
    s.id = w.folk.next_scene;
    s.team = s.team.or(Team::seat(w));
    let id = s.id;
    if s.stops {
        let text = format!("{}: {}", s.kind.name(), super::plain(&s.title));
        super::post_mine(w, s.date, Loudness::Medium, true, "people: meeting", &text);
    }
    super::letters::for_scene(w, &s);
    w.folk.scenes.push(s);
    // Keep the newest scenes; drop old finished ones first.
    if w.folk.scenes.len() > 80 {
        if let Some(k) = w.folk.scenes.iter().position(|x| x.done) {
            w.folk.scenes.remove(k);
        } else {
            w.folk.scenes.remove(0);
        }
    }
    id
}

/// Clears the calendar's "waiting on you" mark for a scene once it is held
/// or handed.
fn clear_mark(w: &mut World, s: &Scene) {
    let text = super::plain(&format!("{}: {}", s.kind.name(), s.title));
    if let Some(list) = w.calendar.events.get_mut(&s.date) {
        for e in list.iter_mut().filter(|e| e.needs_decision && e.text == text) {
            e.needs_decision = false;
        }
    }
}

/// The scene with this id, opened (its first lines written) if it hasn't
/// been yet.
pub fn open(w: &mut World, id: u32) -> Option<Scene> {
    let k = w.folk.scenes.iter().position(|s| s.id == id)?;
    if !w.folk.scenes[k].opened && !w.folk.scenes[k].done {
        let mut s = w.folk.scenes[k].clone();
        s.opened = true;
        dispatch_open(w, &mut s);
        if let Some(slot) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
            *slot = s;
        }
    }
    w.folk.scenes.iter().find(|s| s.id == id).cloned()
}

/// You pick an answer.
pub fn answer(w: &mut World, id: u32, code: u32) -> Result<Scene, String> {
    let k = w.folk.scenes.iter().position(|s| s.id == id).ok_or("No such meeting.")?;
    let mut s = w.folk.scenes[k].clone();
    if s.done {
        return Err("That meeting is over.".into());
    }
    if s.date > w.today && !s.opened {
        return Err(format!("That meeting is set for {}.", s.date.pretty()));
    }
    if !s.opened {
        s.opened = true;
        dispatch_open(w, &mut s);
    }
    // A talk's move may be said in any tone (screens topic 8).
    let talk = matches!(s.kind, Kind::OneOnOne);
    let Some(mut picked) = s
        .picks
        .iter()
        .find(|p| p.code == code)
        .or_else(|| s.picks.iter().find(|p| talk && super::talks::same_move(p.code, code)))
        .cloned()
    else {
        return Err("That answer isn't on the table.".into());
    };
    if picked.code != code {
        picked.tone = super::takeover::Tone::from(code % 10).name().to_string();
        picked.code = code;
    }
    s.picks.clear();
    dispatch_answer(w, &mut s, code, &picked);
    s.step = s.step.saturating_add(1);
    if s.done {
        clear_mark(w, &s);
        log_scene(w, &s);
    }
    if let Some(slot) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
        *slot = s.clone();
    }
    Ok(s)
}

/// Hands a scene to your staff: it plays out the way your staff would
/// play it (calendar topic 10).
pub fn hand(w: &mut World, id: u32) -> Result<Scene, String> {
    let k = w.folk.scenes.iter().position(|s| s.id == id).ok_or("No such meeting.")?;
    let mut s = w.folk.scenes[k].clone();
    if s.done {
        return Err("That meeting is over.".into());
    }
    if !s.opened {
        s.opened = true;
        dispatch_open(w, &mut s);
    }
    dispatch_hand(w, &mut s);
    s.done = true;
    s.picks.clear();
    clear_mark(w, &s);
    log_scene(w, &s);
    if let Some(slot) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
        *slot = s.clone();
    }
    Ok(s)
}

/// The meetings waiting for you on a morning: Continue stops for them.
pub fn due(w: &World, d: Date) -> Option<String> {
    let n = w.folk.scenes.iter().filter(|s| !s.done && s.stops && s.date == d).count();
    if n == 0 {
        return None;
    }
    let first = w.folk.scenes.iter().find(|s| !s.done && s.stops && s.date == d)?;
    let title = super::plain(&first.title);
    Some(if n == 1 {
        format!("{}: {title}", first.kind.name())
    } else {
        format!("{} meetings today, starting with {title}", n)
    })
}

/// The end of a day: a meeting you didn't hold is handed to your staff; a
/// scene that waited a week (a press availability, a call) lapses the same
/// way.
pub(crate) fn day(w: &mut World, d: Date) {
    let late: Vec<u32> = w
        .folk
        .scenes
        .iter()
        .filter(|s| !s.done && ((s.stops && s.date <= d) || (!s.stops && s.date.add(7) <= d)))
        .map(|s| s.id)
        .collect();
    for id in late {
        let _ = hand(w, id);
    }
}

fn log_scene(w: &mut World, s: &Scene) {
    let text = if s.outcome.is_empty() {
        match &s.handed {
            Some(h) => format!("Handed to {h}."),
            None => "Held.".to_string(),
        }
    } else {
        s.outcome.join(" ")
    };
    let source = match &s.handed {
        Some(h) => format!("{} (handed to {h})", s.kind.name()),
        None => s.kind.name().to_string(),
    };
    let topic = match s.kind {
        Kind::Press | Kind::PressIntro | Kind::Media => "press",
        Kind::BossFirst | Kind::BossMeeting => "boss",
        Kind::Booster => "boosters",
        Kind::OneOnOne | Kind::TeamMeeting | Kind::TeamTalk | Kind::Parents | Kind::ParentCall => "talk",
        Kind::Holdover | Kind::StaffLine | Kind::Interview | Kind::Firing | Kind::Proposal => "staff",
        Kind::Discipline | Kind::Ritual => "rules",
        Kind::ReachOut | Kind::Trip | Kind::JobCall => "network",
        _ => "place",
    };
    super::note(w, topic, source, format!("{}: {text}", s.title), None);
}

fn dispatch_open(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::OneOnOne | Kind::TeamMeeting | Kind::TeamTalk | Kind::Parents | Kind::ParentCall | Kind::StaffLine => {
            super::talks::open(w, s)
        }
        Kind::Press | Kind::PressIntro => super::press::open(w, s),
        Kind::BossFirst | Kind::BossMeeting => super::boss::open(w, s),
        Kind::Discipline | Kind::Ritual => super::rules::open(w, s),
        Kind::ReachOut | Kind::Trip | Kind::JobCall => super::network::open(w, s),
        Kind::Interview | Kind::Firing | Kind::Proposal => super::staffing::open(w, s),
        Kind::StarPower | Kind::SecondOpinion | Kind::Family => super::life::open(w, s),
        _ => super::takeover::open(w, s),
    }
}

fn dispatch_answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    match s.kind {
        Kind::OneOnOne | Kind::TeamMeeting | Kind::TeamTalk | Kind::Parents | Kind::ParentCall | Kind::StaffLine => {
            super::talks::answer(w, s, code, picked)
        }
        Kind::Press | Kind::PressIntro => super::press::answer(w, s, code, picked),
        Kind::BossFirst | Kind::BossMeeting => super::boss::answer(w, s, code, picked),
        Kind::Discipline | Kind::Ritual => super::rules::answer(w, s, code, picked),
        Kind::ReachOut | Kind::Trip | Kind::JobCall => super::network::answer(w, s, code, picked),
        Kind::Interview | Kind::Firing | Kind::Proposal => super::staffing::answer(w, s, code, picked),
        Kind::StarPower | Kind::SecondOpinion | Kind::Family => super::life::answer(w, s, code, picked),
        _ => super::takeover::answer(w, s, code, picked),
    }
}

fn dispatch_hand(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::OneOnOne | Kind::TeamMeeting | Kind::TeamTalk | Kind::Parents | Kind::ParentCall | Kind::StaffLine => {
            super::talks::hand(w, s)
        }
        Kind::Press | Kind::PressIntro => super::press::hand(w, s),
        Kind::BossFirst | Kind::BossMeeting => super::boss::hand(w, s),
        Kind::Discipline | Kind::Ritual => super::rules::hand(w, s),
        Kind::ReachOut | Kind::Trip | Kind::JobCall => super::network::hand(w, s),
        Kind::Interview | Kind::Firing | Kind::Proposal => super::staffing::hand(w, s),
        Kind::StarPower | Kind::SecondOpinion | Kind::Family => super::life::hand(w, s),
        _ => super::takeover::hand(w, s),
    }
}

/// Your name for scenes: "Coach Hale" to players, your full name in print.
pub fn your_last(w: &World) -> String {
    let n = w.career.as_ref().map_or("Coach".to_string(), |c| c.you.name.clone());
    let parts: Vec<&str> = n.split_whitespace().collect();
    match parts.as_slice() {
        [.., l, s] if matches!(*s, "Jr." | "II" | "III" | "IV") => l.to_string(),
        [.., l] => l.to_string(),
        _ => n,
    }
}

pub fn your_name(w: &World) -> String {
    w.career.as_ref().map_or("You".to_string(), |c| c.you.name.clone())
}
