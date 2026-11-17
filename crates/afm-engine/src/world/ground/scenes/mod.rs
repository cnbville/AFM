//! Ground's scenes (lane G5: GR32, GR51, GR52, GR55, GR72;
//! `docs/design/ground.md`, part F; the round file `ground/week.md`, "The
//! scenes").
//!
//! The week's scenes (G3's slots, `week::plan::draft_slots`) become
//! playable here, on the built scenes (`folk::scene`, one kind,
//! `scene::Kind::Ground`, with the Ground kind in `Scene.b`). Every scene
//! plays the same way:
//!
//! - **The place:** its time, the room and who's there (the opening lines).
//! - **What you notice:** rolled on your eye the way a film catch is
//!   (`notice`, the film room's clear-catch chance), or your man's on his.
//! - **The talk:** the scene's picks (chips) with the line bank when there's
//!   no key; with a usable key a typed line plays on live talks v3 with the
//!   person across (`folk::persona::meetings`, `room`).
//! - **What you walk away with** (rule 1): always something, said in the
//!   scene's last line and kept in the book (`Book.played`).
//!
//! Skipped (handed to your staff), a scene runs its default stops on your
//! man's skill with people and his eye and gives one line (GR32). The
//! coordinator's scenes (the ones the week didn't give you) play the same
//! way the day after they happen, so every slot gets its line.
//!
//! Of the 15 scenes (GR55), a recruit's call and film are the built ones
//! (`folk::reach::call`, the film room); the rest are here. Only your seat
//! plays them, so the world plays the same on every seed. Every size here
//! is the build's pick, proposed, not confirmed, unless the spec's
//! "Starting sizes" names it.

mod calls;
mod campus;
mod coaches;
mod visit;

pub use coaches::{clinic_day, convention_day};

use super::people::{self, Who};
use super::standing::{self, Contact, Level};
use super::week::{self, Slot, SlotKind};
use crate::calendar::{Date, Weekday};
use crate::rng::clamp;
use crate::world::World;
use crate::world::folk::scene::{self as fscene, Kind as SceneKind, Scene};

/// The 15 scenes (GR55), in the round file's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Meeting,
    SchoolVisit,
    CoachCall,
    RecruitCall,
    NightCalls,
    JuniorDay,
    Unofficial,
    Clinic,
    OpenPractice,
    SpringGame,
    Official,
    Camp,
    Announcement,
    Convention,
    Film,
}

impl Kind {
    pub const ALL: [Kind; 15] = [
        Kind::Meeting,
        Kind::SchoolVisit,
        Kind::CoachCall,
        Kind::RecruitCall,
        Kind::NightCalls,
        Kind::JuniorDay,
        Kind::Unofficial,
        Kind::Clinic,
        Kind::OpenPractice,
        Kind::SpringGame,
        Kind::Official,
        Kind::Camp,
        Kind::Announcement,
        Kind::Convention,
        Kind::Film,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Kind::Meeting => "meeting",
            Kind::SchoolVisit => "school_visit",
            Kind::CoachCall => "coach_call",
            Kind::RecruitCall => "recruit_call",
            Kind::NightCalls => "night_calls",
            Kind::JuniorDay => "junior_day",
            Kind::Unofficial => "unofficial",
            Kind::Clinic => "clinic",
            Kind::OpenPractice => "open_practice",
            Kind::SpringGame => "spring_game",
            Kind::Official => "official",
            Kind::Camp => "camp",
            Kind::Announcement => "announcement",
            Kind::Convention => "convention",
            Kind::Film => "film",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Meeting => "The meeting at the map",
            Kind::SchoolVisit => "A school visit",
            Kind::CoachCall => "A coach's call",
            Kind::RecruitCall => "A recruit's call",
            Kind::NightCalls => "The night calls",
            Kind::JuniorDay => "Junior day",
            Kind::Unofficial => "An unofficial visit",
            Kind::Clinic => "Your clinic",
            Kind::OpenPractice => "Open practice",
            Kind::SpringGame => "The spring game",
            Kind::Official => "An official visit weekend",
            Kind::Camp => "Your camp",
            Kind::Announcement => "The announcement",
            Kind::Convention => "Your state's coaches' convention",
            Kind::Film => "Film",
        }
    }

    pub fn from_key(k: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|x| x.key() == k)
    }

    pub fn code(self) -> u64 {
        self as u64
    }

    pub fn from_code(c: u64) -> Option<Kind> {
        Kind::ALL.get(c as usize).copied()
    }

    /// The built scene that plays it, for the two that aren't here.
    pub fn built(self) -> Option<&'static str> {
        match self {
            Kind::RecruitCall => Some("the call to a recruit (his page)"),
            Kind::Film => Some("the film room"),
            _ => None,
        }
    }

    /// A slot's scene.
    pub fn of_slot(k: SlotKind) -> Option<Kind> {
        Some(match k {
            SlotKind::Meeting => Kind::Meeting,
            SlotKind::SchoolVisit => Kind::SchoolVisit,
            SlotKind::CoachCall => Kind::CoachCall,
            SlotKind::NightCall => Kind::NightCalls,
            SlotKind::JuniorDay => Kind::JuniorDay,
            SlotKind::Unofficial => Kind::Unofficial,
            SlotKind::Official => Kind::Official,
            SlotKind::Camp => Kind::Camp,
            SlotKind::Clinic => Kind::Clinic,
            SlotKind::OpenPractice => Kind::OpenPractice,
            SlotKind::SpringGame => Kind::SpringGame,
            SlotKind::Convention => Kind::Convention,
            SlotKind::Film | SlotKind::Donor | SlotKind::DonorRoom => return None,
        })
    }
}

/// One scene played, by you or your staff: what it came to, for the slot's
/// line, the school visit board (S3) and the ground report.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Played {
    pub date: Date,
    /// Local minutes after midnight.
    pub minute: u16,
    pub kind: Kind,
    pub title: String,
    /// Its one line (GR32).
    pub line: String,
    /// You held it (false: your staff played it).
    pub yours: bool,
    /// The man who played it when it wasn't you.
    pub by: Option<u32>,
    pub who: Option<Who>,
    pub school: Option<u32>,
    /// The kids it touched (player ids): the new names first.
    pub kids: Vec<u32>,
    /// New leads it made (player ids).
    pub leads: Vec<u32>,
    /// Standing it moved: (who, level before, level after).
    pub levels: Vec<(Who, u8, u8)>,
    /// The stops in order: (key, what it showed).
    pub stops: Vec<(String, String)>,
    /// The folk scene it was (0 for a scene your staff played unseen).
    pub scene: u32,
}

/// The scenes' store, on `Ground.scenes`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Book {
    /// What each scene came to, newest last (`KEEP`).
    pub played: Vec<Played>,
    /// The scenes made from slots: (date, minute, slot kind key, the folk
    /// scene id), so a slot makes one.
    pub made: Vec<(Date, u16, String, u32)>,
    /// Coaches you asked to your clinic: (who, season).
    pub invited: Vec<(Who, i32)>,
    /// Your call on the spring game from 2027: (season, hold it).
    pub spring_game: Vec<(i32, bool)>,
    /// Each kid with your offer and where he stood: (kid, the program he
    /// was committed to, or none), for the announcement's call.
    pub commits: Vec<(u32, Option<u32>)>,
    /// The night calls already turned into a scene (log ids).
    pub nights: Vec<u32>,
}

/// Played scenes kept.
pub const KEEP: usize = 300;

/// The year the spring game becomes a choice (`ground/week.md`: the 2027
/// rules don't mention it, and real programs are starting to drop it).
pub const SPRING_GAME_CHOICE: i32 = 2027;

// --- What you notice (GR52) -----------------------------------------------------------------

/// The chance your eye catches a thing in a scene: the film room's
/// clear-catch chance (`scouting::film::session`, `resolve`: 0.30 +
/// 0.035 × (eye − 10) − 0.40 × how hard it is to see, 0.04 to 0.95),
/// less a long week's tiredness (× 1 − 0.5 × tiredness). A 16 eye catches
/// a plain thing about half the time, a 9 eye about a quarter.
pub fn notice_chance(eye: f64, subtle: f64, tired: f64) -> f64 {
    clamp(0.30 + 0.035 * (eye - 10.0) - 0.40 * subtle, 0.04, 0.95) * (1.0 - 0.5 * tired.clamp(0.0, 1.0))
}

/// Whether the eye catches it this time (keyed: the same scene rolls the
/// same).
pub(crate) fn notice(w: &World, eye: f64, subtle: f64, tired: f64, parts: &[u64]) -> bool {
    super::unit(w.options.seed, parts) < notice_chance(eye, subtle, tired)
}

/// Your eye in a scene (your evaluation) and today's tiredness.
pub fn your_eye(w: &World) -> (f64, f64) {
    let e = crate::world::scouting::yours::your_eye(w).unwrap_or(10.0);
    (e, crate::world::scouting::film::time::tiredness(w))
}

/// A man's eye on his own visit (his evaluation), rested.
pub fn man_eye(w: &World, c: usize) -> f64 {
    w.coaches.get(c).map_or(10.0, |c| c.evaluation())
}

// --- The talk's sizes ----------------------------------------------------------------------

/// A pick that gives a person what he wants from you (`people::wants`:
/// to be asked about his kids, the truth about them, a clinic talk; a
/// family's questions answered by the head coach himself): points on top of
/// the contact, inside the gates (proposed, not confirmed).
pub const WANT: f32 = 2.0;

/// A typed line's trust in a ground scene becomes standing at this rate:
/// the office's cap on a line (+0.02) is +1 point, its floor (-0.05) -2.5
/// (proposed, not confirmed).
pub const TALK_POINTS: f32 = 50.0;

/// What a pick that meets his want does: points with your program and your
/// head coach inside the gates, and a moment in his words.
pub(crate) fn wanted(w: &mut World, prog: usize, who: Who, words: &str, d: Date) {
    if w.ground.off {
        return;
    }
    let before = standing::level(w, prog, who);
    let by = WANT * people::makeup(w, who).warmth;
    standing::grant(w, prog, who, by, Some(words.to_string()), d);
    standing::announce(w, prog, who, before, d);
}

/// After a typed line in a ground scene (live talks v3): what it did to
/// each ground person's trust lands as standing (`TALK_POINTS`), inside the
/// gates.
pub fn after_talk(w: &mut World, id: u32, hits: &[(crate::world::folk::persona::Them, f32)]) {
    let Some(prog) = standing::seat_program(w) else { return };
    let Some(s) = w.folk.scenes.iter().find(|s| s.id == id) else { return };
    if s.kind != SceneKind::Ground {
        return;
    }
    let d = w.today;
    for &(them, trust) in hits {
        let who = match them {
            crate::world::folk::persona::Them::Coach(c) => Who::Coach(c as u32),
            crate::world::folk::persona::Them::Kin(id, _) => match crate::world::folk::person(w, id) {
                Some(i) => people::family_of(w, i),
                None => continue,
            },
            _ => continue,
        };
        if trust.abs() < 1e-4 || w.ground.off {
            continue;
        }
        let before = standing::level(w, prog, who);
        standing::grant(w, prog, who, trust * TALK_POINTS, None, d);
        standing::announce(w, prog, who, before, d);
    }
}

// --- Scenes from slots ------------------------------------------------------------------------

/// The scenes in a folk scene's state: the Ground kind.
pub fn kind_of(s: &Scene) -> Option<Kind> {
    (s.kind == SceneKind::Ground).then(|| Kind::from_code(s.b)).flatten()
}

/// A ground person kept in a scene's `a`: its kind in the top byte.
pub fn who_code(who: Who) -> u64 {
    let (t, v) = match who {
        Who::Coach(c) => (1u64, c as u64),
        Who::Trainer(f) => (2, f as u64),
        Who::SevenOnSeven(f) => (3, f as u64),
        Who::Counselor(h) => (4, h as u64),
        Who::Family(k) => (5, k as u64),
        Who::Prep(k) => (6, k as u64),
    };
    t << 56 | v
}

pub fn who_of(code: u64) -> Option<Who> {
    let v = code & 0x00FF_FFFF_FFFF_FFFF;
    Some(match code >> 56 {
        1 => Who::Coach(v as u32),
        2 => Who::Trainer(v as u32),
        3 => Who::SevenOnSeven(v as u32),
        4 => Who::Counselor(v as u32),
        5 => Who::Family(v as u32),
        6 => Who::Prep(v as u16),
        _ => return None,
    })
}

/// Your coach (the one holding the seat).
pub(crate) fn you(w: &World) -> Option<usize> {
    crate::world::scouting::readers::you(w)
}

/// The day's scenes for your seat, at dawn: each of today's slots that
/// comes to you becomes a scene at its hour (Continue stops for it, and at
/// the next office hour it goes to your staff if you haven't held it);
/// yesterday's slots that went to your coordinator get their line.
pub(crate) fn dawn(w: &mut World, d: Date) {
    let Some(prog) = week::seat_program(w) else { return };
    if w.ground.off {
        return;
    }
    // Ground's weeks: January (after the takeover and the portal) to
    // August; from September the season runs and Ground goes on under it
    // (`week::beat`), and a week off is off.
    if !in_season(w, d) || week::off_now(w) {
        campus::announcements(w, prog, d);
        return;
    }
    week::plan::ensure(w, prog, d);
    let slots: Vec<Slot> = w.ground.week.slots.clone();
    for s in slots.iter().filter(|s| s.date == d && s.yours) {
        let Some(k) = Kind::of_slot(s.kind) else { continue };
        if made(w, s).is_some() {
            continue;
        }
        if let Some(id) = start(w, prog, k, s) {
            w.ground.scenes.made.push((s.date, s.minute, s.kind.key().to_string(), id));
        }
    }
    // The coordinator's scenes of yesterday: played on his skill, a line.
    let y = d.add(-1);
    for s in slots.iter().filter(|s| s.date == y && !s.yours) {
        let Some(k) = Kind::of_slot(s.kind) else { continue };
        if made(w, s).is_some() || matches!(k, Kind::Meeting | Kind::CoachCall | Kind::SchoolVisit) {
            continue;
        }
        let line = staff_plays(w, prog, k, s);
        w.ground.scenes.made.push((s.date, s.minute, s.kind.key().to_string(), 0));
        if let Some(line) = line {
            set_line(w, s, &line);
        }
    }
    let cut = d.add(-42);
    w.ground.scenes.made.retain(|m| m.0 >= cut);
    campus::announcements(w, prog, d);
}

/// Whether a day is in Ground's months (GR56): January to August, after
/// the takeover's weeks.
pub fn in_season(w: &World, d: Date) -> bool {
    d.month() <= 8 && !week::beat::beat(w, d.on_or_before(Weekday::Mon)).beat.starts_with("Ground waits")
}

/// The night after the road: a night call that matters becomes a scene
/// (GR13, GR14): last night's calls, with the one that stopped your
/// evening first.
pub(crate) fn after_road(w: &mut World, d: Date) {
    let Some(prog) = week::seat_program(w) else { return };
    if w.ground.off || week::off_now(w) {
        return;
    }
    calls::night(w, prog, d);
}

/// The folk scene a slot made, if any.
pub fn made(w: &World, s: &Slot) -> Option<u32> {
    w.ground.scenes.made.iter().find(|m| m.0 == s.date && m.1 == s.minute && m.2 == s.kind.key()).map(|m| m.3)
}

/// A slot's line from what its scene came to.
pub fn line_for(w: &World, s: &Slot) -> Option<String> {
    let id = made(w, s)?;
    w.ground
        .scenes
        .played
        .iter()
        .rev()
        .find(|p| if id == 0 { p.date == s.date && p.minute == s.minute && p.scene == 0 } else { p.scene == id })
        .map(|p| p.line.clone())
}

fn set_line(w: &mut World, s: &Slot, line: &str) {
    for x in w.ground.week.slots.iter_mut() {
        if x.date == s.date && x.minute == s.minute && x.kind == s.kind {
            x.line = Some(line.to_string());
        }
    }
}

/// Makes the folk scene for a slot of yours: the place, who is there, and
/// what the scene keeps.
pub(crate) fn start(w: &mut World, prog: usize, k: Kind, slot: &Slot) -> Option<u32> {
    let mut s = Scene::new(SceneKind::Ground, slot.date, slot.title.clone());
    s.b = k.code();
    s.minute = Some(slot.minute);
    s.stops = true;
    let ok = match k {
        Kind::Meeting => calls::setup_meeting(w, prog, &mut s),
        Kind::SchoolVisit => visit::setup(w, prog, &mut s, slot),
        Kind::CoachCall => calls::setup_call(w, prog, &mut s, slot),
        Kind::JuniorDay | Kind::Unofficial | Kind::Official | Kind::SpringGame => {
            campus::setup(w, prog, &mut s, k, slot)
        }
        Kind::Camp => campus::setup_camp(w, prog, &mut s),
        Kind::Clinic | Kind::OpenPractice | Kind::Convention => coaches::setup(w, prog, &mut s, k),
        _ => false,
    };
    ok.then(|| fscene::add(w, s))
}

/// A slot the week gave your coordinator: he plays it on his skill and
/// eye (GR32), and it comes to one line.
fn staff_plays(w: &mut World, prog: usize, k: Kind, slot: &Slot) -> Option<String> {
    let mut s = Scene::new(SceneKind::Ground, slot.date, slot.title.clone());
    s.b = k.code();
    s.minute = Some(slot.minute);
    let ok = match k {
        Kind::SchoolVisit => visit::setup(w, prog, &mut s, slot),
        Kind::JuniorDay | Kind::Unofficial | Kind::Official | Kind::SpringGame => {
            campus::setup(w, prog, &mut s, k, slot)
        }
        Kind::Camp => campus::setup_camp(w, prog, &mut s),
        Kind::Clinic | Kind::OpenPractice | Kind::Convention => coaches::setup(w, prog, &mut s, k),
        _ => false,
    };
    if !ok {
        return None;
    }
    s.date = slot.date;
    run_hand(w, prog, &mut s);
    let line = s.outcome.join(" ");
    let mut p = played_of(w, &s, false);
    p.line = line.clone();
    p.scene = 0;
    keep(w, p);
    Some(line)
}

// --- The scene's life (called from `folk::scene`) ------------------------------------------------

/// Its first lines: the place and what you notice walking in.
pub(crate) fn open(w: &mut World, s: &mut Scene) {
    let Some(prog) = week::seat_program(w) else {
        s.narrate("Ground is for a college seat.");
        s.end();
        return;
    };
    match kind_of(s) {
        Some(Kind::Meeting) => calls::open_meeting(w, prog, s),
        Some(Kind::SchoolVisit) => visit::open(w, prog, s),
        Some(Kind::CoachCall) => calls::open_call(w, prog, s),
        Some(Kind::NightCalls) => calls::open_night(w, prog, s),
        Some(k @ (Kind::JuniorDay | Kind::Unofficial | Kind::Official | Kind::SpringGame)) => {
            campus::open(w, prog, s, k)
        }
        Some(Kind::Camp) => campus::open_camp(w, prog, s),
        Some(Kind::Announcement) => campus::open_announcement(w, prog, s),
        Some(k @ (Kind::Clinic | Kind::OpenPractice | Kind::Convention)) => coaches::open(w, prog, s, k),
        _ => {
            s.narrate("This one plays elsewhere.");
            s.end();
        }
    }
    if s.done {
        finish(w, s, true);
    }
}

/// Your pick.
pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32) {
    let Some(prog) = week::seat_program(w) else {
        s.end();
        return;
    };
    match kind_of(s) {
        Some(Kind::Meeting) => calls::answer_meeting(w, prog, s, code),
        Some(Kind::SchoolVisit) => visit::answer(w, prog, s, code),
        Some(Kind::CoachCall) => calls::answer_call(w, prog, s, code),
        Some(Kind::NightCalls) => calls::answer_night(w, prog, s, code),
        Some(k @ (Kind::JuniorDay | Kind::Unofficial | Kind::Official | Kind::SpringGame)) => {
            campus::answer(w, prog, s, k, code)
        }
        Some(Kind::Camp) => campus::answer_camp(w, prog, s, code),
        Some(Kind::Announcement) => campus::answer_announcement(w, prog, s, code),
        Some(k @ (Kind::Clinic | Kind::OpenPractice | Kind::Convention)) => coaches::answer(w, prog, s, k, code),
        _ => s.end(),
    }
    if s.done {
        finish(w, s, true);
    }
}

/// Handed to your staff: the default stops on your man's skill and eye,
/// one line.
pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    let Some(prog) = week::seat_program(w) else { return };
    let to = hand_label(w, s);
    s.handed = Some(to.trim_start_matches("Hand to ").to_string());
    run_hand(w, prog, s);
    finish(w, s, false);
}

fn run_hand(w: &mut World, prog: usize, s: &mut Scene) {
    match kind_of(s) {
        Some(Kind::Meeting) => calls::hand_meeting(w, prog, s),
        Some(Kind::SchoolVisit) => visit::hand(w, prog, s),
        Some(Kind::CoachCall) => calls::hand_call(w, prog, s),
        Some(Kind::NightCalls) => calls::hand_night(w, prog, s),
        Some(k @ (Kind::JuniorDay | Kind::Unofficial | Kind::Official | Kind::SpringGame)) => {
            campus::hand(w, prog, s, k)
        }
        Some(Kind::Camp) => campus::hand_camp(w, prog, s),
        Some(Kind::Announcement) => campus::hand_announcement(w, prog, s),
        Some(k @ (Kind::Clinic | Kind::OpenPractice | Kind::Convention)) => coaches::hand(w, prog, s, k),
        _ => {}
    }
    if s.outcome.is_empty() {
        s.outcome.push(format!("{}: your staff had it.", s.title));
    }
}

/// Who it goes to when you hand it on, for the hand-off button.
pub fn hand_label(w: &World, s: &Scene) -> String {
    let prog = week::seat_program(w);
    let man = match kind_of(s) {
        Some(Kind::NightCalls) => Some(s.a as usize).filter(|&m| m < w.coaches.len()),
        _ => prog.and_then(|p| week::coordinator(w, p)),
    };
    match man {
        Some(c) => format!("Hand to {}", w.coaches[c].name),
        None => "Hand to your staff".to_string(),
    }
}

/// The scene is over: its line on the slot and in the book.
fn finish(w: &mut World, s: &mut Scene, yours: bool) {
    if s.outcome.is_empty() {
        s.outcome.push(format!("{}: done.", s.title));
    }
    let p = played_of(w, s, yours);
    let line = p.line.clone();
    keep(w, p);
    let slot = w.ground.week.slots.iter().find(|x| made(w, x) == Some(s.id)).cloned();
    if let Some(slot) = slot {
        set_line(w, &slot, &line);
    }
}

fn played_of(w: &World, s: &Scene, yours: bool) -> Played {
    let st = state(s);
    Played {
        date: s.date,
        minute: s.minute.unwrap_or(0),
        kind: kind_of(s).unwrap_or(Kind::Meeting),
        title: s.title.clone(),
        line: s.outcome.join(" "),
        yours,
        by: if yours { None } else { week::seat_program(w).and_then(|p| week::coordinator(w, p)).map(|c| c as u32) },
        who: st.who,
        school: st.school,
        kids: st.kids.clone(),
        leads: st.leads.clone(),
        levels: st.levels.clone(),
        stops: st.stops.clone(),
        scene: s.id,
    }
}

fn keep(w: &mut World, p: Played) {
    let b = &mut w.ground.scenes.played;
    b.push(p);
    if b.len() > KEEP {
        b.remove(0);
    }
}

// --- A scene's running state ---------------------------------------------------------------

/// What a scene has done so far, kept on the folk scene's `docs` (a table
/// with no rows the screens skip): the person it's about, the school, the
/// kids, the stops, the standing moved. Serialized as one cell.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct State {
    pub who: Option<Who>,
    pub school: Option<u32>,
    pub man: Option<u32>,
    pub kids: Vec<u32>,
    pub leads: Vec<u32>,
    pub levels: Vec<(Who, u8, u8)>,
    pub stops: Vec<(String, String)>,
    /// Minutes left (a visit's four hours).
    pub minutes: u16,
    /// The talk's exchanges used in the current stop.
    pub talk: u8,
    /// The stop the talk is in ("" for none).
    pub at: String,
    /// The people the scene gathered (coaches at a clinic, the families on
    /// campus): ground people and player ids.
    pub people: Vec<Who>,
    pub ids: Vec<u32>,
    /// The talk's topic at a clinic (0 none, 1 your offense, 2 your defense,
    /// 3 recruiting at your place).
    pub topic: u8,
}

const STATE_TITLE: &str = "ground:state";

pub fn state(s: &Scene) -> State {
    s.docs.iter().find(|d| d.title == STATE_TITLE).and_then(|d| serde_json::from_str(&d.note).ok()).unwrap_or_default()
}

pub(crate) fn put_state(s: &mut Scene, st: &State) {
    let note = serde_json::to_string(st).unwrap_or_default();
    match s.docs.iter_mut().find(|d| d.title == STATE_TITLE) {
        Some(d) => d.note = note,
        None => s.docs.push(crate::world::folk::scene::Doc {
            title: STATE_TITLE.into(),
            note,
            cols: Vec::new(),
            rows: Vec::new(),
        }),
    }
}

/// A table for the screens (the sign-in sheet, the list).
pub(crate) fn put_doc(s: &mut Scene, title: &str, note: &str, cols: &[&str], rows: Vec<(Option<u32>, Vec<String>)>) {
    s.docs.retain(|d| d.title != title);
    s.docs.push(crate::world::folk::scene::Doc {
        title: title.into(),
        note: note.into(),
        cols: cols.iter().map(|c| c.to_string()).collect(),
        rows: rows.into_iter().map(|(pid, cells)| crate::world::folk::scene::DocRow { pid, cells }).collect(),
    });
}

/// Whether a scene's table is the running state (the screens skip it).
pub fn is_state(d: &crate::world::folk::scene::Doc) -> bool {
    d.title == STATE_TITLE || d.title == crate::world::capital::scenes::STATE_TITLE
}

/// A level moved with someone, kept for the scene's book.
pub(crate) fn moved(st: &mut State, who: Who, before: Level, after: Level) {
    if after != before {
        match st.levels.iter_mut().find(|x| x.0 == who) {
            Some(x) => x.2 = after as u8,
            None => st.levels.push((who, before as u8, after as u8)),
        }
    }
}

/// A contact with the scene's book keeping the level move.
#[allow(clippy::too_many_arguments)]
pub(crate) fn touch(
    w: &mut World,
    prog: usize,
    st: &mut State,
    man: Option<usize>,
    who: Who,
    kind: Contact,
    scale: f32,
    d: Date,
) {
    if w.ground.off {
        return;
    }
    let before = standing::level(w, prog, who);
    standing::contact(w, prog, man, who, kind, scale, d);
    let after = standing::level(w, prog, who);
    moved(st, who, before, after);
}

/// "Known", "Trusted"...
pub(crate) fn level_words(w: &World, prog: usize, who: Who) -> &'static str {
    standing::level(w, prog, who).name()
}

/// The bank's level fact for a person.
pub(crate) fn level_key(w: &World, prog: usize, who: Who) -> &'static str {
    match standing::level(w, prog, who) {
        Level::Stranger => "stranger",
        Level::Known => "known",
        Level::Trusted => "trusted",
        Level::Home => "home",
    }
}

/// "8:40 AM".
pub(crate) fn clock(minute: u16) -> String {
    week::phone::clock(minute)
}

/// The last name of a person's full name.
pub(crate) fn last(n: &str) -> String {
    crate::world::names::surname_of(n).to_string()
}

/// A line from the ground bank in a coach's voice, or the fallback.
pub(crate) fn coach_says(
    w: &mut World,
    s: &Scene,
    k: u64,
    key: &str,
    c: usize,
    f: &crate::world::folk::voice::Facts,
    fallback: &str,
) -> String {
    if c >= w.coaches.len() {
        return fallback.to_string();
    }
    let spk = crate::world::folk::voice::Speaker::coach(w, c);
    fscene::bank_line(w, s, k, key, &spk, f, fallback)
}

/// Picks are numbered by what they do: stops from 1, talk picks from 100,
/// a family or kid from 200 (+ his place), an offer from 300 (+ place),
/// leaving 900.
pub const LEAVE: u32 = 900;

/// Whether a day is a weekday.
pub(crate) fn weekday(d: Date) -> bool {
    !matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
}

/// Your seat's spring game this season: held (always before 2027; from
/// 2027 your call, held unless you skip it).
pub fn holds_spring_game(w: &World, season: i32) -> bool {
    season < SPRING_GAME_CHOICE || w.ground.scenes.spring_game.iter().find(|x| x.0 == season).is_none_or(|x| x.1)
}

/// You hold or skip this year's spring game (from 2027).
pub fn set_spring_game(w: &mut World, hold: bool) -> Result<String, String> {
    week::seat_program(w).ok_or("Ground is for a college seat.")?;
    let y = week::beat::dates_for(w, w.today);
    let season = y.spring_game.year();
    if season < SPRING_GAME_CHOICE {
        return Err(format!("Until {SPRING_GAME_CHOICE} every program holds its spring game."));
    }
    if w.today > y.spring_game {
        return Err("This year's spring game has passed.".into());
    }
    w.ground.scenes.spring_game.retain(|x| x.0 != season);
    w.ground.scenes.spring_game.push((season, hold));
    Ok(if hold {
        format!("The spring game is on for {}.", y.spring_game.pretty())
    } else {
        "No spring game this year: the last Saturday of practice is a closed scrimmage.".into()
    })
}

/// The scenes of the coaches' side your seat's week holds (GR55): your
/// clinic, open practice, the spring game and the convention, as slots for
/// the meeting at the map. Read by `week::plan`.
pub(crate) fn coach_slots(w: &World, prog: usize, monday: Date) -> Vec<(u8, Slot)> {
    coaches::slots(w, prog, monday)
}

/// When a past day was, from today: "on Tuesday" within the week, else
/// "on April 14".
pub(crate) fn ago(day: Date, today: Date) -> String {
    let n = day.days_to(today);
    if n == 0 {
        "today".into()
    } else if n == 1 {
        "yesterday".into()
    } else if n < 7 {
        format!("on {}", super::news::weekday_name(day))
    } else {
        format!("on {}", day.pretty())
    }
}

// --- Live talks v3: who is across in a ground scene -----------------------------------------------

/// Who is in the room when you type a line in a ground scene (live talks
/// v3, `folk::persona::meetings::room`): the coach across in his office or
/// on the phone, your coordinator at the map, your man on the night line,
/// the families on campus, the coaches at your clinic, sideline or
/// convention (a room of them), the kid and his family the night he
/// decides. None when nobody stored is there.
pub(crate) fn room(w: &World, s: &Scene) -> Option<crate::world::folk::persona::room::Room> {
    use crate::world::folk::persona::Them;
    use crate::world::folk::persona::meetings::parent;
    use crate::world::folk::persona::room::{Channel, Group, Place, Present, Room, Seat};
    let k = kind_of(s)?;
    let st = state(s);
    let across = |who: Them| Present { who, seat: Seat::Across, speaks: false };
    let listen = |who: Them| Present { who, seat: Seat::Listener, speaks: false };
    let coach = |who: Option<Who>| match who {
        Some(Who::Coach(c)) => Some(Them::Coach(c as usize)),
        _ => None,
    };
    /// The most people a room of them names (proposed, not confirmed).
    const ROOM: usize = 12;
    Some(match k {
        Kind::Meeting => {
            let prog = week::seat_program(w)?;
            Room {
                channel: Channel::InPerson(Place::Office),
                present: vec![across(Them::Coach(week::coordinator(w, prog)?))],
            }
        }
        Kind::SchoolVisit => Room { channel: Channel::InPerson(Place::Office), present: vec![across(coach(st.who)?)] },
        Kind::CoachCall => Room { channel: Channel::Phone, present: vec![across(coach(st.who)?)] },
        Kind::NightCalls => {
            let man = s.a as usize;
            if man >= w.coaches.len() {
                return None;
            }
            Room { channel: Channel::Phone, present: vec![across(Them::Coach(man))] }
        }
        Kind::JuniorDay | Kind::Unofficial | Kind::Official | Kind::SpringGame => {
            let present: Vec<Present> = st.ids.iter().take(ROOM).map(|&id| listen(parent(w, id))).collect();
            if present.is_empty() {
                return None;
            }
            Room { channel: Channel::Group(Group::Parents), present }
        }
        Kind::Camp | Kind::Clinic | Kind::OpenPractice | Kind::Convention => {
            // At camp, the coaches who brought the campers.
            let mut gathered = st.people.clone();
            if k == Kind::Camp {
                for &id in &st.ids {
                    let Some(i) = crate::world::folk::person(w, id) else { continue };
                    if let Some(c) = people::head_coach_of(w, w.people[i].hs)
                        && !gathered.contains(&c)
                    {
                        gathered.push(c);
                    }
                }
            }
            let present: Vec<Present> =
                gathered.iter().filter_map(|&c| coach(Some(c))).take(ROOM).map(listen).collect();
            if present.is_empty() {
                return None;
            }
            Room { channel: Channel::Group(Group::Coaches), present }
        }
        Kind::Announcement => {
            let id = s.a as u32;
            crate::world::folk::person(w, id)?;
            Room {
                channel: Channel::Phone,
                present: vec![
                    Present { who: Them::Player(id), seat: Seat::Across, speaks: true },
                    Present { who: parent(w, id), seat: Seat::Decider, speaks: true },
                ],
            }
        }
        Kind::RecruitCall | Kind::Film => return None,
    })
}

/// A room of people: the families on campus, the coaches at a clinic, a
/// sideline, a camp or the convention.
pub fn is_group(s: &Scene) -> bool {
    matches!(
        kind_of(s),
        Some(
            Kind::JuniorDay
                | Kind::Unofficial
                | Kind::Official
                | Kind::SpringGame
                | Kind::Camp
                | Kind::Clinic
                | Kind::OpenPractice
                | Kind::Convention
        )
    )
}

/// The whole room in words, for the ear's card.
pub fn whole_words(s: &Scene) -> &'static str {
    match kind_of(s) {
        Some(Kind::JuniorDay | Kind::Unofficial | Kind::Official | Kind::SpringGame) => "the families as a whole",
        _ => "the coaches as a whole",
    }
}
