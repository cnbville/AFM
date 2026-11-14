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
    /// A story you are in (update 2, the storyline engine).
    Story,
    /// A call from someone you know with an offer (coaching C15, C18, C19).
    Backchannel,
    // Dialogue talks (next update, lane 14).
    /// A college head coach in a recruit's living room (D7).
    HomeVisit,
    /// A college coach calls you, the high school coach, about a player (D9).
    CollegeCall,
    /// Your own job interview (D11).
    JobInterview,
    /// The weekly staff meeting: a coordinator argues for a change (D12).
    StaffMeeting,
    /// A timed combine interview (D8).
    CombineTalk,
    /// Your call to a transfer in the portal (portal recruiting).
    PortalCall,
    /// Your sit-down with a transfer: at his home, or at the end of his
    /// campus visit.
    PortalVisit,
    /// A December signee asks to be let out of his signing once his head
    /// coach is gone (Jan 1 start).
    SigneeRelease,
    // Talk facts lane L6: the recruit's other talks.
    /// Your call to a high school recruit, on the era's contact rules.
    RecruitCall,
    /// Your message to a high school recruit: one exchange.
    RecruitMessage,
    /// Whoever decides for a recruit calls you about something that changed.
    FamilyCall,
    /// Your case at the end of a recruit's official visit, in your office.
    OfficialTalk,
}

impl Kind {
    /// A recruit or transfer conversation on the fact sheet (`pitch`).
    pub fn is_pitch(self) -> bool {
        matches!(
            self,
            Kind::HomeVisit
                | Kind::PortalCall
                | Kind::PortalVisit
                | Kind::RecruitCall
                | Kind::RecruitMessage
                | Kind::FamilyCall
                | Kind::OfficialTalk
        )
    }

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
            Kind::Story => "Storyline",
            Kind::Backchannel => "A call from someone you know",
            Kind::HomeVisit => "Home visit",
            Kind::CollegeCall => "A college calls",
            Kind::JobInterview => "Job interview",
            Kind::StaffMeeting => "Weekly staff meeting",
            Kind::CombineTalk => "Combine interview",
            Kind::PortalCall => "A call to a transfer",
            Kind::PortalVisit => "Transfer visit",
            Kind::SigneeRelease => "A signee asks out",
            Kind::RecruitCall => "A call to a recruit",
            Kind::RecruitMessage => "A message to a recruit",
            Kind::FamilyCall => "His family calls",
            Kind::OfficialTalk => "Official visit: your case",
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
    /// How it went over (a one-on-one's answers): his trust and mood moving,
    /// as far as your read catches it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub went: Option<String>,
    /// Your own words, when you typed this line instead of picking a chip
    /// (live talks): `text` keeps the chip's words, which the game read it
    /// as, and the screen shows these.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typed: Option<String>,
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
    /// The talking point it stands on: its kind's key (`claims::Kind::key`,
    /// talk facts design, section 14). Empty on a plain pick.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub point: String,
    /// What picking it does to the talk.
    #[serde(skip_serializing_if = "PickKind::is_plain")]
    pub kind: PickKind,
}

/// What picking an option does to the talk (talk facts design, section 14;
/// a name shared with the live talks plan).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PickKind {
    /// An answer: it uses an exchange of the talk.
    #[default]
    Plain,
    /// It uses no exchange (asking what he is looking for).
    Free,
    /// It moves around the talk ("bring up something else", back). A typed
    /// line is never read as one.
    Nav,
    /// It can't be undone (a firing, a release, a ruling): a typed line
    /// that reaches it is asked about first.
    Final,
}

impl PickKind {
    pub fn is_plain(&self) -> bool {
        *self == PickKind::Plain
    }
}

impl Pick {
    /// The same pick, standing on a talking point (its kind's key).
    pub fn on(mut self, point: &str) -> Pick {
        self.point = point.to_string();
        self
    }
    /// The same pick, of a kind.
    pub fn of(mut self, kind: PickKind) -> Pick {
        self.kind = kind;
        self
    }
}

/// What a typed line carried (live talks, the core): the reader's labels,
/// turned into the scene's own terms by the game. `say` puts it on
/// `Scene.heard` for the one answer it lands on, so an answer function can
/// read what it needs beyond the pick, and clears it right after. It is
/// never saved: the reading is saved as the pick.
///
/// Every field a later lane needs is here, so no lane adds one: lane 6
/// reads `words`, `len`, `facts` and `plan`; lane 7 `facts`; lanes 9 and 10
/// `tone`, `facts` and `promise`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heard {
    /// The option it landed on (a code in the scene's `picks` or `reach`).
    pub pick: u32,
    /// The tone it was said in: "fiery", "warm", "firm" or "calm", or empty.
    pub tone: String,
    /// Your words, as typed (at most `LIMIT` characters).
    pub words: String,
    /// Their length in characters.
    pub len: u16,
    /// Each fact you stated, as the reader labelled it. The game checks
    /// them (`claims::check`), never the AI.
    pub facts: Vec<super::claims::Stated>,
    /// A promise the line made, as read, or none. The game judges it, never
    /// the AI (the promise design, `afm/promises/proposal.md`, decides how).
    pub promise: Option<PromiseRead>,
    /// A game plan ask (a staff meeting): which setting, which way, how far.
    pub plan: Option<PlanAsk>,
    /// The people the line named, by their ids.
    pub named: Vec<u32>,
}

/// The longest typed line a talk takes, in characters.
pub const LIMIT: usize = 400;

/// A promise as the reader heard it, in the terms a promise can carry
/// (the promise design's section 3, proposed, not confirmed): what he gets,
/// by when, on what condition and against whom. Labels, not sizes: the game
/// decides what each means and what keeping it takes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PromiseRead {
    /// "start", "role", "touches", "ahead_of", "position", "place",
    /// "contract", "stay".
    pub kind: String,
    /// "next_game", "opener", "midseason", "this_season", "next_season" or
    /// empty for none said.
    pub by: String,
    /// "stat", "practice", "conduct", "grades", "body", "health" or empty.
    pub condition: String,
    /// The condition in the coach's words ("three tackles a game"), empty
    /// when none.
    pub condition_words: String,
    /// The teammate he would play ahead of, by id.
    pub against: Option<u32>,
    /// The claim it was heard from, when the line stated where he stands
    /// rather than promising it in so many words (the promise design,
    /// section 11: "you're my starter" is a promise about what he'll see).
    pub claim: Option<super::claims::Stated>,
}

/// A game plan ask read from a typed line (lane 6's staff meeting).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlanAsk {
    /// The setting's key ("pressure", "man", "run").
    pub setting: String,
    /// +1 more, -1 less.
    pub direction: i8,
    /// How far, in the setting's own steps (1 for one step).
    pub step: u8,
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
    /// When it is held, minutes after your local midnight (the six-part
    /// day: a meeting keeps its real hour; None opens it at the day's first
    /// stop).
    #[serde(default)]
    pub minute: Option<u16>,
    /// The options behind "bring up something else" (talk facts design,
    /// section 14): a talk shows a few answers at a time and keeps the
    /// rest here, so a typed line can reach every one of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reach: Vec<Pick>,
    /// The talking points made in a one-on-one (talk facts lane L4): each
    /// kind's key, what you did with it last, and what it did to trust and
    /// mood together; kept in `Folk.talked` when the talk ends.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub made: Vec<(String, String, f32)>,
    /// What a typed line carried beyond its pick (live talks), set by `say`
    /// for the one answer it lands on and cleared right after. Never saved.
    #[serde(skip)]
    pub heard: Option<Heard>,
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
            minute: None,
            reach: Vec::new(),
            made: Vec::new(),
            heard: None,
        }
    }

    pub fn narrate(&mut self, text: impl Into<String>) {
        self.lines.push(Said { by: 0, name: String::new(), text: text.into(), tell: None, went: None, typed: None });
    }
    pub fn you(&mut self, name: &str, text: impl Into<String>) {
        self.lines.push(Said { by: 1, name: name.to_string(), text: text.into(), tell: None, went: None, typed: None });
    }
    pub fn them(&mut self, name: &str, text: impl Into<String>, tell: Option<String>) {
        self.lines.push(Said { by: 2, name: name.to_string(), text: text.into(), tell, went: None, typed: None });
    }
    pub fn other(&mut self, name: &str, text: impl Into<String>) {
        self.lines.push(Said { by: 3, name: name.to_string(), text: text.into(), tell: None, went: None, typed: None });
    }
    pub fn pick(&mut self, code: u32, label: impl Into<String>, mv: &str, tone: &str, hint: &str) {
        self.picks.push(Pick {
            label: label.into(),
            mv: mv.into(),
            tone: tone.into(),
            hint: hint.into(),
            code,
            ..Pick::default()
        });
    }
    pub fn end(&mut self) {
        self.done = true;
        self.picks.clear();
        self.reach.clear();
    }
}

/// A seeded pen for one step of one scene: the same scene replays the same.
/// Put it back with `pen.put(w)` when the step is written.
pub fn pen(w: &mut World, s: &Scene, k: u64) -> voice::Pen {
    let seed = (w.folk.base as u64) << 20 ^ (s.id as u64) << 8 ^ s.step as u64 ^ k << 40;
    voice::Pen::take(w, seed)
}

/// A line from the bank for one step of a scene, or `fallback` while the
/// part has no version this speaker can say. A tree that reads the facts
/// asks for its new part this way, so it plays from its first merge and
/// never says something its facts don't back (talk facts design, section
/// 15, rule 3).
pub(super) fn bank_line(
    w: &mut World,
    s: &Scene,
    k: u64,
    key: &str,
    who: &voice::Speaker,
    f: &voice::Facts,
    fallback: &str,
) -> String {
    let mut pen = pen(w, s, k);
    let line = pen.line(&[key], who, f);
    pen.put(w);
    if line.is_empty() { fallback.to_string() } else { line }
}

/// Adds a scene for you: it gets its id and, when it stops Continue, a line
/// on the calendar waiting on your answer.
pub fn add(w: &mut World, mut s: Scene) -> u32 {
    w.folk.next_scene += 1;
    s.id = w.folk.next_scene;
    s.team = s.team.or(Team::seat(w));
    if s.minute.is_none() {
        s.minute = hour_in(&s.place).or_else(|| default_hour(w, &s));
    }
    let id = s.id;
    if s.stops {
        let text = cal_text(&s);
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

/// A scene's line on the calendar: its title, after the kind of meeting
/// when the title alone doesn't say it ("Team meeting: Your first athletics
/// period", but "The trainer's report" alone).
fn cal_text(s: &Scene) -> String {
    let title = super::plain(&s.title);
    let kind = s.kind.name();
    let last = kind.rsplit(' ').next().unwrap_or(kind).trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    if title.is_empty() {
        kind.to_string()
    } else if title.to_lowercase().contains(&last) {
        title
    } else {
        format!("{kind}: {title}")
    }
}

/// Clears the calendar's "waiting on you" mark for a scene once it is held
/// or handed.
fn clear_mark(w: &mut World, s: &Scene) {
    let text = cal_text(s);
    // Its world day: a line posted after midnight Eastern is filed under
    // the next date.
    for (_, list) in w.calendar.events.range_mut(s.date..=s.date.add(1)) {
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
    land(w, id, code, None)
}

/// The scene kinds that take no typed line (live talks, the core): press
/// and media were decided against (Cain 2026-11-07), and a story meeting's
/// moves are public statements on a page of their own. A one-on-one takes
/// typed lines once its hook is filled (`talks::ONE_ON_ONE_TYPED`, lane 9).
pub const NO_BOX: [Kind; 4] = [Kind::Press, Kind::PressIntro, Kind::Media, Kind::Story];

/// What `say` answers when a typed line reaches a pick that can't be undone
/// and the reading was not confirmed. The game layer asks first.
pub const ASK_FIRST: &str = "That can't be undone, so say which you meant first.";

/// The options a typed line can reach in this exchange: the picks on the
/// table and those behind "bring up something else" (`reach`), never one
/// that only moves around the talk.
pub fn options(s: &Scene) -> Vec<&Pick> {
    s.picks.iter().chain(s.reach.iter()).filter(|p| p.kind != PickKind::Nav).collect()
}

/// Whether a scene takes a typed line now: it is open and not over, its
/// kind takes them, a person is in it, and it has an option a line can
/// land on.
pub fn takes_typed(s: &Scene) -> bool {
    let kind_ok = !NO_BOX.contains(&s.kind) && (s.kind != Kind::OneOnOne || super::talks::ONE_ON_ONE_TYPED);
    kind_ok && s.opened && !s.done && !s.who.is_empty() && !options(s).is_empty()
}

/// Whether landing on this pick from a typed line asks first: the pick is
/// marked final, or the core's table lists it (`finals`).
pub fn asks_first(s: &Scene, p: &Pick) -> bool {
    p.kind == PickKind::Final || super::finals::is_final(s.kind, p.code)
}

/// A typed line (live talks, the core): it lands on one of the options the
/// scene can reach in this exchange (`options`), through the same dispatch
/// as a click, so the line bank picks its line with the same seeds and the
/// world after is the same as after the chip. What the line carried beyond
/// the pick rides on `Scene.heard` for that one answer. Your words go on the
/// scene's own line for the pick (`Said.typed`), never on a second line.
///
/// A pick that can't be undone (`asks_first`) is refused unless the reading
/// was confirmed, so no caller can skip the ask-first step. A one-on-one
/// reads its option from its own hook (`talks::one_on_one_say`).
pub fn say(w: &mut World, id: u32, mut heard: Heard, confirmed: bool) -> Result<Scene, String> {
    let s = w.folk.scenes.iter().find(|s| s.id == id).ok_or("No such meeting.")?;
    if s.done {
        return Err("That meeting is over.".into());
    }
    if !takes_typed(s) {
        return Err("This talk takes no typed lines.".into());
    }
    if heard.words.trim().is_empty() {
        return Err("Say something first.".into());
    }
    if heard.words.chars().count() > LIMIT {
        return Err(format!("A line is {LIMIT} characters at most."));
    }
    let code = if s.kind == Kind::OneOnOne {
        super::talks::one_on_one_say(w, s, &heard).ok_or("That answer isn't on the table.")?
    } else {
        heard.pick
    };
    let p = options(s).into_iter().find(|p| p.code == code).ok_or("That answer isn't on the table.")?;
    if asks_first(s, p) && !confirmed {
        return Err(ASK_FIRST.into());
    }
    heard.pick = code;
    heard.len = heard.words.chars().count().min(u16::MAX as usize) as u16;
    // A claim about where your player stands is a promise about what he'll
    // see (the promise design, section 11).
    if let Some(p) = super::claims::truth::standing_promise(w, s, &heard) {
        heard.promise = Some(p);
    }
    let promise = heard.promise.clone().zip(super::claims::truth::your_player(w, s));
    let out = land(w, id, code, Some(heard))?;
    if let Some((p, him)) = promise {
        super::talks::typed_promise(w, id, him, &p);
    }
    Ok(out)
}

/// Lands an answer, clicked (`heard` none) or typed. The scene is written
/// back into the world after, and `heard` is cleared first and after, so a
/// later chip never finds an old one.
fn land(w: &mut World, id: u32, code: u32, heard: Option<Heard>) -> Result<Scene, String> {
    let k = w.folk.scenes.iter().position(|s| s.id == id).ok_or("No such meeting.")?;
    let mut s = w.folk.scenes[k].clone();
    s.heard = None;
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
        .chain(s.reach.iter())
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
    let before = s.lines.len();
    let words = heard.as_ref().map(|h| h.words.clone());
    s.heard = heard;
    dispatch_answer(w, &mut s, code, &picked);
    s.heard = None;
    if let Some(words) = words {
        put_typed(&mut s, before, &picked.label, words, your_name(w));
    }
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

/// Puts typed words on the line the answer wrote for you. An answer that
/// ends the meeting without a line of yours gets one, with the chip's
/// words, just before what it wrote, so the transcript still shows what you
/// said. A meeting that goes on gets no extra line (scenes count your
/// lines: an interview offers fewer questions after three), so there the
/// words ride only on the poll's answer.
fn put_typed(s: &mut Scene, before: usize, label: &str, words: String, name: String) {
    let from = before.min(s.lines.len());
    match s.lines[from..].iter_mut().find(|l| l.by == 1) {
        Some(l) => l.typed = Some(words),
        None if s.done => {
            let line = Said { by: 1, name, text: label.to_string(), typed: Some(words), ..Said::default() };
            s.lines.insert(from, line);
        }
        None => {}
    }
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
        // A recruit talk you never opened is your staff's alone: opening it
        // would put his first question and the news on your own file.
        if !s.kind.is_pitch() {
            dispatch_open(w, &mut s);
        }
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

/// Ends a scene you left open without handing it on: `end` writes what
/// it came to, and what was said stands.
pub(crate) fn leave(w: &mut World, id: u32, end: impl FnOnce(&mut World, &mut Scene)) {
    let Some(k) = w.folk.scenes.iter().position(|s| s.id == id) else { return };
    let mut s = w.folk.scenes[k].clone();
    if s.done {
        return;
    }
    end(w, &mut s);
    s.done = true;
    s.picks.clear();
    clear_mark(w, &s);
    log_scene(w, &s);
    if let Some(slot) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
        *slot = s;
    }
}

/// The meetings waiting for you on a morning: Continue stops for them.
pub fn due(w: &World, d: Date) -> Option<String> {
    let n = w.folk.scenes.iter().filter(|s| !s.done && s.stops && s.date == d).count();
    if n == 0 {
        return None;
    }
    let first = w.folk.scenes.iter().find(|s| !s.done && s.stops && s.date == d)?;
    let title = super::plain(&first.title);
    Some(if n == 1 { cal_text(first) } else { format!("{} meetings today, starting with {title}", n) })
}

/// The end of a day: a meeting you didn't hold is handed to your staff; a
/// scene that waited a week (a press availability, a call) lapses the same
/// way.
pub(crate) fn day(w: &mut World, d: Date) {
    // A meeting with its hour waits for it: it is handed only once its hour
    // has passed (the six-part day, topic 8), the next morning's office
    // hours at the latest.
    let now = crate::calendar::zone::moment(d, super::super::clock::hour::OFFICE);
    let z = w.seat_zone();
    let due = |s: &Scene| match s.minute {
        Some(m) if s.date == d => z.to_eastern(crate::calendar::Moment::at(s.date, m / 60, m % 60)) < now,
        _ => s.date <= d,
    };
    let late: Vec<u32> = w
        .folk
        .scenes
        .iter()
        .filter(|s| !s.done && ((s.stops && due(s)) || (!s.stops && s.date.add(7) <= d)))
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
        Kind::ReachOut | Kind::Trip | Kind::JobCall | Kind::Backchannel => "network",
        Kind::Story => "press",
        Kind::HomeVisit
        | Kind::CollegeCall
        | Kind::CombineTalk
        | Kind::PortalCall
        | Kind::PortalVisit
        | Kind::SigneeRelease
        | Kind::RecruitCall
        | Kind::RecruitMessage
        | Kind::FamilyCall
        | Kind::OfficialTalk => "recruiting",
        Kind::JobInterview => "network",
        Kind::StaffMeeting => "staff",
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
        Kind::Story => crate::world::storylines::moves::open(w, s),
        Kind::Backchannel => super::backchannel::open(w, s),
        Kind::HomeVisit
        | Kind::CollegeCall
        | Kind::JobInterview
        | Kind::StaffMeeting
        | Kind::CombineTalk
        | Kind::PortalCall
        | Kind::PortalVisit
        | Kind::RecruitCall
        | Kind::RecruitMessage
        | Kind::FamilyCall
        | Kind::OfficialTalk => super::dialogue::open(w, s),
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
        Kind::Story => crate::world::storylines::moves::answer(w, s, code, picked),
        Kind::Backchannel => super::backchannel::answer(w, s, code, picked),
        Kind::HomeVisit
        | Kind::CollegeCall
        | Kind::JobInterview
        | Kind::StaffMeeting
        | Kind::CombineTalk
        | Kind::PortalCall
        | Kind::PortalVisit
        | Kind::RecruitCall
        | Kind::RecruitMessage
        | Kind::FamilyCall
        | Kind::OfficialTalk => super::dialogue::answer(w, s, code, picked),
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
        Kind::Story => crate::world::storylines::moves::hand(w, s),
        Kind::Backchannel => super::backchannel::hand(w, s),
        Kind::HomeVisit
        | Kind::CollegeCall
        | Kind::JobInterview
        | Kind::StaffMeeting
        | Kind::CombineTalk
        | Kind::PortalCall
        | Kind::PortalVisit
        | Kind::RecruitCall
        | Kind::RecruitMessage
        | Kind::FamilyCall
        | Kind::OfficialTalk => super::dialogue::hand(w, s),
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

/// The hour a place line names ("the team room, 7:30 p.m.", "11 a.m."), in
/// minutes after midnight.
pub fn hour_in(place: &str) -> Option<u16> {
    let low = place.to_lowercase();
    let word = |x: &str| low.split(|c: char| !c.is_ascii_alphanumeric()).any(|t| t == x);
    let (k, pm) = match (low.rfind(" a.m."), low.rfind(" p.m.")) {
        (Some(a), Some(p)) => (a.max(p), p > a),
        (Some(a), None) => (a, false),
        (None, Some(p)) => (p, true),
        _ if word("noon") => return Some(12 * 60),
        _ if word("midnight") => return Some(0),
        _ => return None,
    };
    let start = low[..k].rfind(|c: char| !(c.is_ascii_digit() || c == ':')).map_or(0, |i| i + 1);
    let t = &low[start..k];
    let (h, m) = match t.split_once(':') {
        Some((h, m)) => (h.parse::<u16>().ok()?, m.parse::<u16>().ok()?),
        None => (t.parse::<u16>().ok()?, 0),
    };
    if !(1..=12).contains(&h) || m > 59 {
        return None;
    }
    let h24 = match (h, pm) {
        (12, false) => 0,
        (12, true) => 12,
        (h, true) => h + 12,
        (h, false) => h,
    };
    Some(h24 * 60 + m)
}

/// A meeting's hour where its line names none (the six-part day, topic 8,
/// proposed, not confirmed): the boss in office hours (10 AM), a booster
/// or the parents in the evening (7 PM), a recruit by phone at night
/// (8 PM), a player or a staff talk after practice, the weekly press at the
/// program's own hour. Others open at the day's first stop.
fn default_hour(w: &World, s: &Scene) -> Option<u16> {
    let after_practice = || {
        let (level, t) = w.seat_team()?;
        let tier = match level {
            crate::calendar::Level::HighSchool => crate::training::rules::Tier::High,
            crate::calendar::Level::Nfl => crate::training::rules::Tier::Pro,
            _ => crate::training::rules::Tier::College,
        };
        let d =
            crate::training::days::practice_start(w, tier, t, s.date) + crate::training::days::practice_length(tier);
        Some(d + 15)
    };
    match s.kind {
        Kind::BossFirst | Kind::BossMeeting | Kind::Firing | Kind::Interview | Kind::Plan => Some(10 * 60),
        Kind::Booster | Kind::Parents => Some(19 * 60),
        Kind::Recruiting | Kind::ReachOut => Some(20 * 60),
        Kind::OneOnOne | Kind::Discipline | Kind::StarPower | Kind::Trainer | Kind::StaffReport | Kind::StaffLine => {
            after_practice()
        }
        Kind::Press => super::press::press_hour(w, s),
        Kind::PressIntro => Some(12 * 60),
        _ => None,
    }
}

#[cfg(test)]
mod hour_tests {
    #[test]
    fn hours_in_place_lines() {
        assert_eq!(super::hour_in("Ross-Ade Pavilion, the team room, 7:30 p.m."), Some(19 * 60 + 30));
        assert_eq!(super::hour_in("The media room, 11 a.m."), Some(11 * 60));
        assert_eq!(super::hour_in("The athletic director's office, 3 p.m."), Some(15 * 60));
        assert_eq!(super::hour_in("The media room"), None);
        assert_eq!(super::hour_in("The facility's auditorium, noon"), Some(12 * 60));
        assert_eq!(super::hour_in("The weight room, in the afternoon"), None);
    }
}
