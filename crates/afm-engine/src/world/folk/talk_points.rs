//! One-on-ones that read his facts (talk facts lane L4, item 6; design in
//! `/afm/talk-facts/proposal.md` in the project folder, sections 5, 8 and
//! 10). The fact sheet (`claims::player`) puts his talking points on the
//! table: each one you know is a chip, worded with its real values (the
//! charge, the rival, the opponent, his forty). He opens on what is most on
//! his mind, in a stance read from his traits and the truth; each point
//! offers three to five of the nine moves, worded for it; a point with a
//! question comes as one; and the game keeps what you talked through.
//!
//! `talks.rs` runs the talk; this file says what a point offers and how
//! much a move on it counts (`Rest`). The AI coaches' weekly talks run the
//! same reading (rule 8).
//!
//! Hidden numbers stay gates: whether he did it (`Point::hid`) picks his
//! stance and never a word of a line, and no label names a number that
//! isn't on the point. Every size here is Claude's (proposed, not
//! confirmed).

use super::claims::{self, Basis, Kind as K, Point, Sheet};
use super::rules::{self, RuleKind};
use super::scene::{Pick, PickKind};
use super::takeover::Tone;
use super::talks::{self, Move, PromiseKind, Reason, Subject};
use super::voice::Facts;
use crate::calendar::Level;
use crate::player::{PlayingTime, Trait};
use crate::world::{Stage, World};

/// A point this much on his mind is what he is really worried about.
/// Proposed, not confirmed.
pub const WORRY_FLOOR: f32 = 0.3;
/// A point this much on his mind, he raises himself before you start (an
/// open case always). Proposed, not confirmed.
pub const LEADS_WITH: f32 = 0.5;
/// The most talking points shown as chips. Proposed, not confirmed.
pub const POINT_CHIPS: usize = 6;
/// A move that moved trust and mood together by more than this landed
/// (the talk's own "good"). Proposed, not confirmed.
pub const LANDED: f32 = 0.06;
/// A softened answer: trust and mood a little up, more for a shaky man.
/// Proposed, not confirmed.
pub const SOFTEN: (f32, f32) = (0.03, 0.02);
/// What a shaky man (stability 0) adds to a softened answer.
pub const SOFTEN_SHAKY: (f32, f32) = (0.02, 0.03);
/// A dodge: he notices. Proposed, not confirmed.
pub const DODGE: (f32, f32) = (-0.04, -0.02);
/// What an arrest does to his mood, for an average man (stability 10);
/// from 0.6 to 1.5 times that by his stability. Proposed, not confirmed.
pub const ARREST_HIT: f32 = 0.15;
/// How much getting him help lowers his risk of another arrest, for a
/// year, not stacking. Proposed, not confirmed.
pub const HELP_CUT: f64 = 0.15;
pub const HELP_DAYS: i32 = 365;
/// How often an AI coach reads a man right in a one-on-one: 0.25 + 0.035 x
/// his relationships, the chance of a good talk the AI coaches have always
/// had (rule 8: "picking with noise from their ratings as they do now").
pub const AI_READ_RIGHT: (f32, f32) = (0.25, 0.035);

/// Bit of `Scene.b` that marks a talking point (the rest is its key's
/// hash), so a save keeps the point if the catalogue is reordered.
const POINT_TAG: u64 = 1 << 40;

/// What a one-on-one is about: one of the subjects you can always raise,
/// or one of his talking points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topic {
    Subject(Subject),
    Point(K),
}

fn hash(key: &str) -> u64 {
    let mut h: u32 = 0x811C_9DC5;
    for b in key.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h as u64
}

impl Topic {
    /// The topic a scene holds in `Scene.b`.
    pub fn of(b: u64) -> Topic {
        if b & POINT_TAG != 0 {
            let h = b & 0xFFFF_FFFF;
            return K::ALL
                .iter()
                .copied()
                .find(|k| talkable(*k) && hash(k.key()) == h)
                .map_or(Topic::Subject(Subject::Role), Topic::Point);
        }
        Topic::Subject(Subject::from_u64(b))
    }
    /// What `Scene.b` holds for it.
    pub fn code(self) -> u64 {
        match self {
            Topic::Subject(s) => s as u64,
            Topic::Point(k) => POINT_TAG | hash(k.key()),
        }
    }
    /// The chip's key: a subject's, or the point's kind (no kind a player
    /// talks about shares a key with a subject).
    pub fn key(self) -> &'static str {
        match self {
            Topic::Subject(s) => s.key(),
            Topic::Point(k) => k.key(),
        }
    }
    pub fn from_key(k: &str) -> Option<Topic> {
        Subject::from_key(k).map(Topic::Subject).or_else(|| K::from_key(k).filter(|k| talkable(*k)).map(Topic::Point))
    }
    /// The subject it belongs to (which of today's lines and moves it
    /// falls back on).
    pub fn subject(self) -> Subject {
        match self {
            Topic::Subject(s) => s,
            Topic::Point(k) => subject_of(k),
        }
    }
    pub fn point(self) -> Option<K> {
        match self {
            Topic::Point(k) => Some(k),
            Topic::Subject(_) => None,
        }
    }
}

impl From<Subject> for Topic {
    fn from(s: Subject) -> Topic {
        Topic::Subject(s)
    }
}

/// A kind you can raise in a one-on-one: his points in season and out,
/// and what is coming. A friend who left is the friend subject (built).
pub fn talkable(k: K) -> bool {
    matches!(k.family(), claims::Family::Player | claims::Family::Offseason) && k != K::FriendLeft || k == K::Ahead
}

/// The subject a kind of point belongs to.
pub fn subject_of(k: K) -> Subject {
    match k {
        K::Arrest | K::CaseClosed | K::Suspended | K::RuleBreak | K::Effort => Subject::Behavior,
        K::Pay => Subject::Money,
        K::FriendLeft => Subject::Friend,
        K::WantsOut
        | K::Portal
        | K::Draft
        | K::Grades
        | K::FailedTerm
        | K::Colleges
        | K::ProSpring
        | K::TeammateDrafted => Subject::Future,
        K::SeasonEnded | K::NewCoach | K::NewSystem => Subject::Team,
        k if k.is_life() => Subject::Life,
        _ => Subject::Role,
    }
}

/// What weighs on him: his top point that is not good news, when it is
/// big enough. An award or a big game is on his mind, but it is not a
/// worry, so it is never given as why he is unhappy (`talks::concern`).
pub fn on_mind(sheet: &Sheet) -> Option<&Point> {
    sheet
        .points
        .iter()
        .find(|p| p.whose != claims::Whose::Yours && talkable(p.kind) && lean_of(p) != Lean::Good)
        .filter(|p| p.mind >= WORRY_FLOOR)
}

/// What he raises himself before you start: an open case always, else
/// his top point when it weighs on him (`LEADS_WITH`). A point you don't
/// know is his to raise or not (`opens_up`).
pub fn leads(sheet: &Sheet) -> Option<&Point> {
    sheet.get(K::Arrest).or_else(|| sheet.top().filter(|p| talkable(p.kind) && p.mind >= LEADS_WITH))
}

/// The points you can raise: the ones you know, the one most on his mind
/// first, one chip a kind.
pub fn chips(sheet: &Sheet) -> Vec<&Point> {
    let mut v: Vec<&Point> = Vec::new();
    for p in sheet.known().filter(|p| talkable(p.kind)) {
        if !v.iter().any(|x| x.kind == p.kind) {
            v.push(p);
        }
        if v.len() == POINT_CHIPS {
            break;
        }
    }
    v
}

/// "a DUI arrest" to "The DUI arrest".
fn the(s: &str) -> String {
    format!("The {}", bare(s))
}

/// "a DUI arrest" to "DUI arrest".
fn bare(s: &str) -> &str {
    s.strip_prefix("a ").or_else(|| s.strip_prefix("an ")).unwrap_or(s)
}

fn first(s: &str) -> String {
    claims::words::first(s)
}

/// A chip's name for a point, with its real values.
pub fn chip_name(p: &Point) -> String {
    let v = |h: &str| p.get(h).unwrap_or("").to_string();
    let n = match p.kind {
        K::Arrest => p.get("charge").map_or("His arrest".into(), the),
        K::CaseClosed => "His case".into(),
        K::Suspended => "His suspension".into(),
        K::RuleBreak => p.get("rule").map_or("A team rule".into(), |r| format!("The {r} rule")),
        K::LostJob => "Losing his job".into(),
        K::Buried => p.get("ahead").map_or("Not getting on the field".into(), |a| format!("Stuck behind {}", first(a))),
        K::WonJob => "Winning the job".into(),
        K::BigGame => p.get("opp").map_or("His big game".into(), |o| format!("His game against {o}")),
        K::BadGame => p.get("opp").map_or("His last game".into(), |o| format!("The {o} game")),
        K::Slump => "His slump".into(),
        K::Award => p.get("award").map_or("His award".into(), |a| format!("The award: {a}")),
        K::Hurt => p.get("injury").map_or("His injury".into(), |i| format!("His {i}")),
        K::Back => "Coming back".into(),
        K::Birth => "The new baby".into(),
        K::Wedding => "The wedding".into(),
        K::SickParent => "His sick parent".into(),
        K::Death => "The death in his family".into(),
        K::Divorce => "His divorce".into(),
        K::MoneyLost => "The money he lost".into(),
        K::Homesick => "Missing home".into(),
        K::PromiseOpen => "Your promise".into(),
        K::PromiseBroken => "Your broken promise".into(),
        K::Pay => "His pay".into(),
        K::WantsOut => "Wanting out".into(),
        K::Portal => "The portal".into(),
        K::Draft => "The draft".into(),
        K::NewCoach => p.get("pc").map_or("His new coach".into(), |c| format!("{c}, his new coach")),
        K::Grades => "His grades".into(),
        K::ParentCalled => p.get("parent").map_or("The call from home".into(), |x| format!("The call from his {x}")),
        K::Effort => "His effort".into(),
        K::SeasonEnded => "The season".into(),
        K::TestingDay => "Testing day".into(),
        K::PracticeWeek => "His week of practice".into(),
        K::ReportDay => "Reporting to camp".into(),
        K::SpringStep => "His spring".into(),
        K::ChartMove => "Where he stands".into(),
        K::NewPosition => p.get("position").map_or("His new position".into(), |x| format!("Learning {x}")),
        K::RoomChanged => "His position room".into(),
        K::NewSystem => p.get("system").map_or("The new system".into(), |x| format!("The new {x}")),
        K::TeammateDrafted => p.get("name").map_or("A teammate drafted".into(), |x| format!("{} drafted", first(x))),
        K::FailedTerm => "His eligibility".into(),
        K::OtherSport => p.get("sport").map_or("His other sport".into(), |x| format!("His {x} season")),
        K::Colleges => "His recruiting".into(),
        K::ProSpring => "His spring".into(),
        K::Ahead => p.get("what").map_or("What's coming".into(), cap),
        _ => cap(&v("what")),
    };
    if n.is_empty() { cap(p.kind.key()) } else { n }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// What's on his mind, in the words your staff would use (`concern`).
pub fn worry_words(p: &Point) -> String {
    let g = |h: &str| p.get(h).unwrap_or("").to_string();
    match p.kind {
        K::Arrest => match p.get("charge") {
            Some(c) => format!("the {} is all he can think about", bare(c)),
            None => "his case is all he can think about".into(),
        },
        K::CaseClosed => "he's trying to put his case behind him".into(),
        K::Suspended => "the suspension is eating at him".into(),
        K::RuleBreak => "he's still sore about the ruling".into(),
        K::LostJob if p.get("rival").is_some() => format!("losing his job to {} is eating at him", first(&g("rival"))),
        K::LostJob => "losing his job is eating at him".into(),
        K::Buried => match p.get("ahead") {
            Some(a) => format!("he thinks he's stuck behind {}", first(a)),
            None => "he can't see a way onto the field".into(),
        },
        K::WonJob => "he's riding high on winning the job".into(),
        K::BigGame => "he's feeling good about his last game".into(),
        K::BadGame => "his last game is still with him".into(),
        K::Slump => "he knows he's in a slump".into(),
        K::Award => "he's proud of his award".into(),
        K::Hurt => "the injury is weighing on him".into(),
        K::Back => "he's worried about his body since he came back".into(),
        K::PromiseOpen => "he's waiting to see if you keep your word".into(),
        K::PromiseBroken => "he hasn't forgotten the promise you broke".into(),
        K::Pay => "he thinks he's underpaid".into(),
        K::WantsOut => "he wants out".into(),
        K::Portal => "he's been thinking about the portal".into(),
        K::Draft => "the draft is on his mind".into(),
        K::NewCoach => "he's still getting used to his new position coach".into(),
        K::Grades | K::FailedTerm => "his grades are on his mind".into(),
        K::ParentCalled => "the call from home has him on edge".into(),
        K::Effort => "he knows his coaches are on him about his effort".into(),
        K::SeasonEnded => "he's still thinking about how the season ended".into(),
        K::TestingDay => "his testing numbers are on his mind".into(),
        K::PracticeWeek => "his week of practice is on his mind".into(),
        K::ReportDay => "how he reported is on his mind".into(),
        K::SpringStep => "he's thinking about his spring".into(),
        K::ChartMove => "where he stands is on his mind".into(),
        K::NewPosition => "learning a new position is wearing on him".into(),
        K::RoomChanged => "the new faces at his spot are on his mind".into(),
        K::NewSystem => "the new system is on his mind".into(),
        K::TeammateDrafted => "a teammate getting drafted has him thinking".into(),
        K::OtherSport => "his other sport is on his mind".into(),
        K::Colleges => "recruiting is on his mind".into(),
        K::ProSpring => "his spring with the club is on his mind".into(),
        k if k.is_life() => format!("he {}", g("life")),
        _ => "something is on his mind".into(),
    }
}

// ---------------------------------------------------------------------------
// His stance.

/// How he stands on a point (section 5, "His stance"): it picks his
/// words, and a tell your read may catch. It moves no number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stance {
    Owns,
    PlaysDown,
    Denies,
    Defiant,
    Shaken,
    Accepts,
    PushesBack,
    Sulks,
    FiredUp,
    Humble,
    Proud,
    OpensUp,
    Guarded,
}

impl Stance {
    pub const ALL: [Stance; 13] = [
        Stance::Owns,
        Stance::PlaysDown,
        Stance::Denies,
        Stance::Defiant,
        Stance::Shaken,
        Stance::Accepts,
        Stance::PushesBack,
        Stance::Sulks,
        Stance::FiredUp,
        Stance::Humble,
        Stance::Proud,
        Stance::OpensUp,
        Stance::Guarded,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Stance::Owns => "owns",
            Stance::PlaysDown => "plays_down",
            Stance::Denies => "denies",
            Stance::Defiant => "defiant",
            Stance::Shaken => "shaken",
            Stance::Accepts => "accepts",
            Stance::PushesBack => "pushes_back",
            Stance::Sulks => "sulks",
            Stance::FiredUp => "fired_up",
            Stance::Humble => "humble",
            Stance::Proud => "proud",
            Stance::OpensUp => "opens_up",
            Stance::Guarded => "guarded",
        }
    }
    /// Kept in bits 16-19 of `Scene.c` (0: none).
    pub fn bits(self) -> i64 {
        (Stance::ALL.iter().position(|s| *s == self).unwrap_or(0) as i64 + 1) << 16
    }
    pub fn from_bits(c: i64) -> Option<Stance> {
        let k = (c >> 16) & 0xF;
        if k == 0 { None } else { Stance::ALL.get(k as usize - 1).copied() }
    }
}

/// Good news (an award, a job won, a big game): no worry, and no subject
/// line fits it.
pub fn is_good(p: &Point) -> bool {
    lean_of(p) == Lean::Good
}

/// The four kinds of stance a point takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lean {
    Trouble,
    Role,
    Good,
    Life,
}

fn lean_of(p: &Point) -> Lean {
    match p.kind {
        K::Arrest | K::CaseClosed | K::Suspended | K::RuleBreak | K::Effort => Lean::Trouble,
        K::Hurt => Lean::Life,
        k if k.is_life() => Lean::Life,
        K::WonJob | K::BigGame | K::Award | K::Back | K::Colleges | K::OtherSport => Lean::Good,
        K::ChartMove if p.is("move", "up") => Lean::Good,
        K::PracticeWeek if p.is("grade", "best") => Lean::Good,
        K::TestingDay if p.is("test", "better") || p.is("test", "first") => Lean::Good,
        K::ReportDay if p.is("shape", "great") => Lean::Good,
        K::SpringStep if !p.is("step", "missed") => Lean::Good,
        K::SeasonEnded if p.is("his", "good") => Lean::Good,
        K::NewPosition if p.is("stage", "ready") => Lean::Good,
        K::Draft if p.is("round", "early") || p.is("round", "middle") => Lean::Good,
        _ => Lean::Role,
    }
}

/// Whether he did what a trouble point says: only he knows for an open
/// case (`Point::hid`); a ruling, a suspension, a plea or a conviction
/// says he did; a case dropped or won says he didn't.
pub fn did(p: &Point) -> bool {
    match p.kind {
        K::Arrest => p.hid("did") == Some("yes"),
        K::CaseClosed => p.is("outcome", "convicted") || p.is("outcome", "pleaded"),
        _ => true,
    }
}

/// His stance on a point: his traits, his trust in you and the truth, with
/// the day he's having (a seeded draw).
pub fn stance(w: &World, i: usize, p: &Point, seed: u64) -> Stance {
    let me = &w.people[i];
    let tr = &me.player.personality.traits;
    let z = |t: Trait| tr.z(t) as f32;
    let trust = me.trust - 0.5;
    let shaky = ((10.0 - me.bg.stability) / 10.0) as f32;
    let comp = ((me.player.personality.competitiveness - 10.0) / 10.0) as f32;
    let id = me.player.id as u64;
    let noise = |k: u64| 0.25 * super::roll_n(w, seed ^ 0x57A_0000 ^ k << 32, id) as f32;
    let pick = |v: &[(Stance, f32)]| {
        v.iter()
            .enumerate()
            .map(|(k, (s, x))| (*s, x + noise(k as u64)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(Stance::Accepts, |x| x.0)
    };
    match lean_of(p) {
        Lean::Trouble => {
            let did = did(p);
            let again = p.is("again", "yes") as u8 as f32;
            let court = p.is("step", "court") as u8 as f32;
            pick(&[
                (
                    Stance::Owns,
                    0.4 + 0.5 * z(Trait::Professionalism) + 0.6 * trust - 0.3 * z(Trait::Ego)
                        + if did { 0.3 } else { -1.0 },
                ),
                (Stance::PlaysDown, 0.2 + 0.3 * z(Trait::Ego) - 0.2 * z(Trait::Professionalism) + 0.15 * again),
                (
                    Stance::Denies,
                    if did { -0.1 + 0.3 * z(Trait::Ego) - 0.4 * z(Trait::Professionalism) - 0.4 * trust } else { 0.9 },
                ),
                (Stance::Defiant, -0.1 + 0.5 * z(Trait::Temper) + 0.3 * z(Trait::Ego) - 0.3 * trust + 0.2 * again),
                (Stance::Shaken, -0.1 + 0.5 * shaky + 0.2 * court - 0.2 * z(Trait::Ego) + 0.2 * (1.0 - again)),
            ])
        }
        Lean::Role => pick(&[
            (Stance::Accepts, 0.3 + 0.5 * z(Trait::Professionalism) + 0.4 * trust - 0.3 * z(Trait::Ego)),
            (Stance::PushesBack, 0.1 + 0.4 * z(Trait::Ego) + 0.3 * comp - 0.2 * z(Trait::Professionalism)),
            (Stance::Sulks, 0.4 * shaky - 0.3 * comp + 0.2 * z(Trait::Ego) - 0.3 * trust),
            (Stance::FiredUp, 0.6 * comp + 0.2 * z(Trait::Leadership) - 0.2 * shaky),
        ]),
        Lean::Good => pick(&[
            (Stance::Humble, 0.5 - 0.6 * z(Trait::Ego) + 0.3 * z(Trait::Professionalism)),
            (Stance::Proud, 0.6 * z(Trait::Ego) + 0.2 * comp),
        ]),
        Lean::Life => pick(&[
            (Stance::OpensUp, 0.2 + 0.8 * trust + 0.3 * z(Trait::Adaptability) - 0.2 * z(Trait::Ego)),
            (Stance::Guarded, 0.1 - 0.5 * trust + 0.3 * z(Trait::Ego) + 0.2 * z(Trait::Temper)),
        ]),
    }
}

/// The conduct facts a line in this talk may stand on (`CLAIM_FACTS`):
/// what the world backs, and on an open case what his stance makes true
/// to say (he did it and owns it: he may admit; he didn't and denies it:
/// he may deny). A guilty man who denies it gets no fact, so only words
/// that deny nothing outright.
pub fn conduct(sheet: &Sheet, p: Option<&Point>, st: Option<Stance>, f: &mut Facts) {
    sheet.fill_conduct(f);
    let [blameless, at_fault] = super::voice::CLAIM_FACTS;
    if let (Some(p), Some(st)) = (p, st)
        && p.kind == K::Arrest
    {
        match (st, did(p)) {
            (Stance::Owns, true) => {
                f.set(at_fault, "yes");
            }
            // He may deny the charge, but not what else is on his record.
            (Stance::Denies, false) if !other_fault(sheet) => {
                f.set(blameless, "yes");
            }
            _ => {}
        }
    }
}

/// A fault on his sheet besides an open case (`Sheet::conduct`'s list).
fn other_fault(sheet: &Sheet) -> bool {
    sheet.points.iter().any(|q| match q.kind {
        K::RuleBreak | K::Suspended | K::Effort => true,
        K::CaseClosed => did(q),
        _ => false,
    })
}

/// What your read catches of his stance: a sharp read sees a denial that
/// doesn't hold (the tells of topic 5).
pub fn stance_tell(p: &Point, st: Stance, sharp: f64) -> Option<String> {
    let s = match st {
        Stance::Denies if sharp >= 0.45 && did(p) => "He doesn't hold your eye when he says it.",
        Stance::Denies if sharp >= 0.45 => "He looks straight at you when he says it.",
        Stance::Owns => "He sits up straight and doesn't look away.",
        Stance::Defiant => "His jaw is set. He's ready for a fight.",
        Stance::Shaken => "His knee won't stop bouncing.",
        Stance::PlaysDown => "He shrugs, like it's nothing.",
        Stance::Sulks => "He's slumped in the chair.",
        Stance::FiredUp => "He's on the edge of his seat.",
        Stance::PushesBack => "He's already shaking his head.",
        Stance::Proud => "He's grinning before he sits down.",
        Stance::Guarded => "His arms are folded.",
        _ => return None,
    };
    Some(s.into())
}

// ---------------------------------------------------------------------------
// Your moves on a point.

/// One option on a point: the move, its tone, your words, what it tends
/// to do.
pub struct Opt {
    pub m: Move,
    pub tone: Tone,
    pub label: String,
    pub hint: &'static str,
}

const HINT_ASK: &str = "He opens up if he trusts you; that's how you learn what you couldn't see.";
const HINT_STRAIGHT: &str = "Honesty costs mood now and builds trust.";
const HINT_PRAISE: &str = "Praise for something real lands.";
const HINT_CHALLENGE: &str = "A competitor rises to it; a fragile player sinks.";
const HINT_CRITICIZE: &str = "Lands as an insult on a big ego; a professional takes it.";
const HINT_LISTEN: &str = "Helps one who needs to be heard; frustrates one who wants answers.";
/// The warning's hint (section 5: its old words promised less trouble with
/// the police, which a warning never did).
pub const HINT_WARN: &str = "Lowers his chance of breaking a team rule for two months; a hothead bristles.";
const HINT_HELP: &str = "Gets him help: a little less risk of more trouble for a year.";
const HINT_LEAVE: &str = "Time off from practice; the room sees how you treat people.";
const HINT_PLACE: &str = "Logged: his place doesn't change by the season's end, or it's broken.";
const HINT_START: &str = "Lifts him now. Logged: break it and he, his friends and the room remember.";

/// Your words for a move or an answer on a point: the bank's part for it
/// (the point's values and states, and what your rules say, are its facts),
/// else `fallback`, which says the same from the code. One wording a day
/// for the same moment (`voice::Look`).
pub(crate) fn you_words(w: &World, i: usize, p: &Point, key: &str, fallback: String) -> String {
    let base = talks::player_facts(w, i);
    let mut f = p.facts(&base);
    if let Some(d) = p.due {
        f.set("when_due", claims::words::when_words(d, w.today));
    }
    if p.kind == K::Arrest {
        let level = super::Team::seat(w).map_or(Level::College, |t| t.level());
        f.set("book", book_says(w, w.people[i].player.id, level));
    }
    f.set("starter", if talks::is_starter(w, i) { "yes" } else { "no" });
    let me = super::voice::Speaker::you(w);
    let seed = ((w.people[i].player.id as u64) << 24) ^ hash(key) ^ 0x1220;
    let look = super::voice::Look::new(w, seed);
    look.part(key, &me, &f).map(|x| super::voice::tidy(&x)).unwrap_or(fallback)
}

/// The moves a point offers (section 5, "Your moves"): three to five,
/// worded with its real values.
pub fn moves(w: &World, i: usize, p: &Point) -> Vec<Opt> {
    let mut v: Vec<Opt> = Vec::new();
    let g = |h: &str| p.get(h).map(str::to_string);
    // Your words come from the bank (`talk.point.you.<point>.<move>`, lane
    // L12) when it has a version that fits; else today's.
    let mut add = |m: Move, tone: Tone, label: String, hint: &'static str| {
        let label = you_words(w, i, p, &format!("talk.point.you.{}.{}", p.kind.key(), m.key()), label);
        v.push(Opt { m, tone, label: format!("\"{label}\""), hint });
    };
    let nfl = w.people[i].stage == Stage::Pro;
    let level = super::Team::seat(w).map_or(Level::College, |t| t.level());
    match p.kind {
        K::Arrest => {
            add(
                Move::Ask,
                Tone::Warm,
                match g("when") {
                    Some(d) if d != "today" => format!("Tell me what happened {d}."),
                    _ => "Tell me what happened.".into(),
                },
                HINT_ASK,
            );
            // What comes next, by the case's step: the charging decision, the
            // court date, or (a failed test, gambling) the league's ruling.
            let due = p.due.map(|d| {
                let when = claims::words::when_words(d, w.today);
                match p.check("step") {
                    Some("arrest") => format!(" The charging decision comes {when}."),
                    Some("league") => format!(" The league rules {when}."),
                    _ => format!(" You're due in court {when}."),
                }
            });
            add(
                Move::Straight,
                Tone::Calm,
                format!(
                    "Our rules say what comes next: {}.{}",
                    book_says(w, w.people[i].player.id, level),
                    due.unwrap_or_default()
                ),
                HINT_STRAIGHT,
            );
            add(Move::Warn, Tone::Firm, "There can't be a next time. Not here.".into(), HINT_WARN);
            add(Move::Hand, Tone::Warm, "I want you talking to someone. I'll set it up.".into(), HINT_HELP);
            add(Move::Promise, Tone::Calm, "Handle this the right way and your place here is safe.".into(), HINT_PLACE);
        }
        K::CaseClosed => {
            if did(p) {
                add(
                    Move::Straight,
                    Tone::Calm,
                    if p.is("outcome", "pleaded") {
                        "It's over, but you pleaded to a lesser charge. Here's what that means here.".into()
                    } else {
                        "It's over, but you were convicted. Here's what that means here.".into()
                    },
                    HINT_STRAIGHT,
                );
                add(Move::Challenge, Tone::Firm, "Show everybody that's not who you are.".into(), HINT_CHALLENGE);
                add(Move::Hand, Tone::Warm, "You keep seeing someone. I'll make sure of it.".into(), HINT_HELP);
                add(Move::Warn, Tone::Firm, "That was your one. There isn't another.".into(), HINT_WARN);
            } else {
                add(Move::Ask, Tone::Warm, "How are you doing, now that it's over?".into(), HINT_ASK);
                add(Move::Straight, Tone::Calm, "It's behind you. Here it's a clean slate.".into(), HINT_STRAIGHT);
                add(Move::Listen, Tone::Calm, "Say nothing. Let him talk.".into(), HINT_LISTEN);
            }
        }
        K::Suspended => {
            let games = g("games").map_or("these".into(), |n| format!("{n} more"));
            add(
                Move::Straight,
                Tone::Calm,
                format!("You sit {games} games. Here's how you stay ready."),
                HINT_STRAIGHT,
            );
            add(Move::Challenge, Tone::Firm, "Use this time. Come back better than you left.".into(), HINT_CHALLENGE);
            add(Move::Listen, Tone::Calm, "Hear him out. Promise nothing.".into(), HINT_LISTEN);
            add(Move::Promise, Tone::Calm, "When you're back, your spot is there.".into(), HINT_PLACE);
        }
        K::RuleBreak => {
            let rule = g("rule").unwrap_or("the rule".into());
            match g("punishment") {
                Some(x) => add(
                    Move::Straight,
                    Tone::Calm,
                    format!("Breaking the {rule} rule cost you this: {x}. That's the rule."),
                    HINT_STRAIGHT,
                ),
                None => add(
                    Move::Straight,
                    Tone::Calm,
                    format!("You broke the {rule} rule. There's a price for that here."),
                    HINT_STRAIGHT,
                ),
            }
            add(
                Move::Warn,
                Tone::Firm,
                if p.is("again", "yes") {
                    "Twice is a pattern. Next time it costs more.".into()
                } else {
                    "Next time it's not a talk.".into()
                },
                HINT_WARN,
            );
            add(
                Move::Criticize,
                Tone::Fiery,
                format!("The {rule} rule is for everybody. The whole room saw."),
                HINT_CRITICIZE,
            );
            add(Move::Ask, Tone::Warm, "What's going on with you lately?".into(), HINT_ASK);
        }
        K::Effort => {
            let how = if p.is("prep", "unprepared") { "unprepared" } else { "underprepared" };
            add(
                Move::Straight,
                Tone::Calm,
                format!("Your coaches say you've been coming in {how}. That stops now."),
                HINT_STRAIGHT,
            );
            add(Move::Challenge, Tone::Firm, "You're better than this week. Show me.".into(), HINT_CHALLENGE);
            add(Move::Criticize, Tone::Fiery, "Everybody in that room is working but you.".into(), HINT_CRITICIZE);
            add(Move::Ask, Tone::Warm, "What's getting in the way?".into(), HINT_ASK);
        }
        K::LostJob => {
            let rival = g("rival").map(|r| first(&r));
            add(Move::Ask, Tone::Warm, "How are you taking it?".into(), HINT_ASK);
            let straight = match (p.check("why"), &rival) {
                (Some("injury"), Some(r)) => format!("{r} took over while you were hurt, and he's played well."),
                (Some("injury"), None) => "You lost the job while you were hurt. That's how it goes here.".into(),
                (Some("discipline"), _) => "You lost the job for what happened off the field. That's on you.".into(),
                (_, Some(r)) => format!("{r} outplayed you. That's why he's starting."),
                _ => "You lost the job on the field. That's the truth of it.".into(),
            };
            add(Move::Straight, Tone::Calm, straight, HINT_STRAIGHT);
            add(
                Move::Challenge,
                Tone::Firm,
                rival.as_ref().map_or("Then take it back.".into(), |r| format!("Then take it back from {r}.")),
                HINT_CHALLENGE,
            );
            add(Move::Promise, Tone::Warm, "Keep working like this and the job comes back to you.".into(), HINT_START);
            add(Move::Listen, Tone::Calm, "Hear him out. Promise nothing.".into(), HINT_LISTEN);
        }
        K::Buried => {
            let ahead = g("ahead").map(|a| first(&a)).unwrap_or("the man ahead of you".into());
            add(
                Move::Straight,
                Tone::Calm,
                match (g("ahead_all"), g("ahead_class")) {
                    (Some(all), _) => format!("{all} have the reps and the snaps right now. That's the truth of it."),
                    (None, Some(c)) => format!("You're behind {ahead}, and he's a {c}. That's the truth of it."),
                    (None, None) => format!("You're behind {ahead}. That's the truth of it."),
                },
                HINT_STRAIGHT,
            );
            add(Move::Challenge, Tone::Firm, format!("Pass {ahead}. Make me play you."), HINT_CHALLENGE);
            add(Move::Promise, Tone::Warm, "Stay with it. Your time here is coming.".into(), HINT_START);
            add(Move::Ask, Tone::Warm, "Where's your head at?".into(), HINT_ASK);
        }
        K::WonJob => {
            add(
                Move::Praise,
                Tone::Warm,
                g("rival").map_or("You earned that job.".into(), |r| {
                    format!("You earned that job. You beat out {}.", first(&r))
                }),
                HINT_PRAISE,
            );
            add(Move::Challenge, Tone::Firm, "Now keep it. Somebody wants it back.".into(), HINT_CHALLENGE);
            add(Move::Ask, Tone::Warm, "How does it feel?".into(), HINT_ASK);
        }
        K::BigGame => {
            let opp = g("opp").unwrap_or("them".into());
            add(
                Move::Praise,
                Tone::Warm,
                match g("line") {
                    Some(l) => format!("{} against {opp}. That's the player I know.", cap(&l)),
                    None => format!("What you did against {opp} was big."),
                },
                HINT_PRAISE,
            );
            add(Move::Challenge, Tone::Firm, "Good. Now do it again this week.".into(), HINT_CHALLENGE);
            add(Move::Ask, Tone::Warm, format!("What clicked for you against {opp}?"), HINT_ASK);
        }
        K::BadGame => {
            let opp = g("opp").unwrap_or("them".into());
            add(Move::Ask, Tone::Warm, format!("What happened out there against {opp}?"), HINT_ASK);
            add(
                Move::Straight,
                Tone::Calm,
                match g("mistake") {
                    Some(m) => format!("{} against {opp}. We fix it this week.", cap(&m)),
                    None => format!("That wasn't you against {opp}. We fix it this week."),
                },
                HINT_STRAIGHT,
            );
            add(
                Move::Criticize,
                Tone::Fiery,
                g("mistake").map_or("That can't happen.".into(), |m| format!("{}. That can't happen.", cap(&m))),
                HINT_CRITICIZE,
            );
            add(Move::Challenge, Tone::Firm, "Answer it this week.".into(), HINT_CHALLENGE);
        }
        K::Slump => {
            let games = g("games").unwrap_or("a few".into());
            add(Move::Ask, Tone::Warm, format!("{games} games now. What's going on?"), HINT_ASK);
            add(
                Move::Straight,
                Tone::Calm,
                format!("You haven't been yourself for {games} games. Here's what I see."),
                HINT_STRAIGHT,
            );
            add(Move::Challenge, Tone::Firm, "Break out of it this week.".into(), HINT_CHALLENGE);
            add(Move::Listen, Tone::Calm, "Hear him out. Promise nothing.".into(), HINT_LISTEN);
        }
        K::Award => {
            add(
                Move::Praise,
                Tone::Warm,
                g("award").map_or("You earned that.".into(), |a| format!("{}. You earned it.", cap(&a))),
                HINT_PRAISE,
            );
            add(Move::Challenge, Tone::Firm, "Good. Now go get the next one.".into(), HINT_CHALLENGE);
            add(Move::Ask, Tone::Warm, "What does it mean to you?".into(), HINT_ASK);
        }
        K::Hurt => {
            let inj = g("injury").unwrap_or("injury".into());
            add(Move::Ask, Tone::Warm, format!("How's the {inj}?"), HINT_ASK);
            add(
                Move::Straight,
                Tone::Calm,
                g("weeks").map_or("Take the time. Don't rush it.".into(), |n| {
                    format!("{n} weeks. Take them. Don't rush it.")
                }),
                HINT_STRAIGHT,
            );
            add(Move::Promise, Tone::Calm, "Your place is here when you're healthy.".into(), HINT_PLACE);
            add(Move::Listen, Tone::Calm, "Say nothing. Let him talk.".into(), HINT_LISTEN);
        }
        K::Back => {
            add(
                Move::Ask,
                Tone::Warm,
                g("injury").map_or("How does it feel?".into(), |x| format!("How does the {x} feel?")),
                HINT_ASK,
            );
            add(Move::Straight, Tone::Calm, "We bring you back slow. No rushing it.".into(), HINT_STRAIGHT);
            add(Move::Challenge, Tone::Firm, "Show me you're all the way back.".into(), HINT_CHALLENGE);
        }
        K::Birth | K::Wedding => {
            add(Move::Ask, Tone::Warm, "Congratulations. How's everybody doing?".into(), HINT_ASK);
            add(Move::Hand, Tone::Warm, "Take a few days. Be with your family.".into(), HINT_LEAVE);
            add(Move::Listen, Tone::Calm, "Let him tell you about it.".into(), HINT_LISTEN);
        }
        k if k.is_life() => {
            add(Move::Listen, Tone::Calm, "Say nothing. Let him talk.".into(), HINT_LISTEN);
            add(Move::Hand, Tone::Warm, "Take the time you need. Your family comes first.".into(), HINT_LEAVE);
            add(Move::Ask, Tone::Warm, "How are you holding up?".into(), HINT_ASK);
            add(Move::Promise, Tone::Calm, "Take care of home. Your spot is here when you're back.".into(), HINT_PLACE);
        }
        K::PromiseOpen => {
            let what = g("promise").unwrap_or("what I promised".into());
            add(Move::Straight, Tone::Calm, format!("I told you {what}. I haven't forgotten."), HINT_STRAIGHT);
            add(Move::Ask, Tone::Warm, "How do you feel about where things are?".into(), HINT_ASK);
            add(Move::Listen, Tone::Calm, "Hear him out. Promise nothing.".into(), HINT_LISTEN);
        }
        K::PromiseBroken => {
            let what = g("promise").unwrap_or("something".into());
            add(
                Move::Straight,
                Tone::Calm,
                format!("I promised you {what} and it didn't happen. I owe you the truth about why."),
                HINT_STRAIGHT,
            );
            add(Move::Ask, Tone::Warm, "Where do we stand, you and me?".into(), HINT_ASK);
            add(Move::Listen, Tone::Calm, "Let him say it. All of it.".into(), HINT_LISTEN);
            add(Move::Promise, Tone::Warm, "Whatever else, your place here doesn't change.".into(), HINT_PLACE);
        }
        K::Pay => {
            add(
                Move::Promise,
                Tone::Calm,
                if nfl {
                    "I'll get the general manager to sit down with your agent.".into()
                } else {
                    "I'll talk to the collective about you.".into()
                },
                if nfl { "A contract talk, logged; it's kept if a new deal follows." } else { "An NIL push, logged." },
            );
            add(
                Move::Straight,
                Tone::Calm,
                "Play like it and the money follows. That's how this works.".into(),
                "Honest; a greedy player hears a no.",
            );
            add(Move::Ask, Tone::Warm, "What would make this right for you?".into(), HINT_ASK);
            add(Move::Listen, Tone::Calm, "Hear him out. Promise nothing.".into(), HINT_LISTEN);
        }
        K::WantsOut => {
            add(Move::Straight, Tone::Calm, "You're under contract, and I need you here.".into(), HINT_STRAIGHT);
            add(Move::Ask, Tone::Warm, "What would it take to make this work?".into(), HINT_ASK);
            add(
                Move::Promise,
                Tone::Calm,
                "I'll get the general manager to sit down with your agent.".into(),
                "A contract talk, logged; it's kept if a new deal follows.",
            );
            add(Move::Challenge, Tone::Firm, "Play your way to what you want.".into(), HINT_CHALLENGE);
        }
        K::Portal => {
            add(
                Move::Straight,
                Tone::Calm,
                "Here's what next year looks like for you here, honestly.".into(),
                "Trust now; a player who isn't getting on the field may still go.",
            );
            add(
                Move::Promise,
                Tone::Warm,
                "Stay, and you'll start next year.".into(),
                "Keeps him more often. Logged: a start next season.",
            );
            add(Move::Ask, Tone::Warm, "What would make you stay?".into(), HINT_ASK);
            add(
                Move::Challenge,
                Tone::Fiery,
                "If you want to run from competition, that's your call.".into(),
                "A competitor stays to prove you wrong; others take the door.",
            );
        }
        K::Draft => {
            let read = match p.check("round") {
                Some("early") => "The scouts I talk to have you going early.",
                Some("middle") => "The scouts I talk to have you in the middle rounds.",
                Some("late") => "The scouts I talk to have you going late.",
                _ => "The scouts I talk to don't have you drafted yet.",
            };
            add(Move::Straight, Tone::Calm, read.into(), HINT_STRAIGHT);
            add(Move::Ask, Tone::Warm, "What are you leaning toward?".into(), HINT_ASK);
            add(Move::Challenge, Tone::Firm, "Another year here and you go higher.".into(), HINT_CHALLENGE);
            add(Move::Listen, Tone::Calm, "Hear him out. Promise nothing.".into(), HINT_LISTEN);
        }
        K::NewCoach => {
            let pc = g("pc").unwrap_or("your new coach".into());
            add(Move::Ask, Tone::Warm, format!("How are you getting on with {pc}?"), HINT_ASK);
            add(
                Move::Straight,
                Tone::Calm,
                format!("{pc} is here to make you better. Give him a chance."),
                HINT_STRAIGHT,
            );
            add(Move::Listen, Tone::Calm, "Hear him out. Promise nothing.".into(), HINT_LISTEN);
        }
        K::Grades | K::FailedTerm => {
            let line = if p.kind == K::FailedTerm {
                format!(
                    "You're out for the {}. Here's the way back.",
                    if p.is("out", "fall") { "fall" } else { "spring" }
                )
            } else if p.is("line", "under") {
                "You're under the line. Class comes first, or you don't play.".into()
            } else {
                "You're close to the line. Class comes first.".into()
            };
            add(Move::Straight, Tone::Calm, line, HINT_STRAIGHT);
            add(Move::Hand, Tone::Warm, "I'm setting you up with a tutor.".into(), "Extra help in the classroom.");
            add(Move::Warn, Tone::Firm, "Miss class and you don't play.".into(), HINT_WARN);
            add(Move::Ask, Tone::Warm, "What's getting in the way in class?".into(), HINT_ASK);
        }
        K::ParentCalled => {
            let who = g("parent").unwrap_or("family".into());
            add(Move::Ask, Tone::Warm, format!("Your {who} called me. What's going on?"), HINT_ASK);
            add(
                Move::Straight,
                Tone::Calm,
                format!("Your {who} called. I'll tell you what I told them."),
                HINT_STRAIGHT,
            );
            add(Move::Listen, Tone::Calm, "Hear him out. Promise nothing.".into(), HINT_LISTEN);
        }
        K::SeasonEnded => {
            add(Move::Ask, Tone::Warm, "How do you feel about the year?".into(), HINT_ASK);
            if p.is("his", "good") {
                add(
                    Move::Praise,
                    Tone::Warm,
                    "You had a good year. I want you to hear that from me.".into(),
                    HINT_PRAISE,
                );
            }
            add(
                Move::Straight,
                Tone::Calm,
                g("record").map_or("We have to be better, and so do you.".into(), |r| {
                    format!("{r}. We have to be better, and so do you.")
                }),
                HINT_STRAIGHT,
            );
            add(Move::Challenge, Tone::Firm, "Next year starts now.".into(), HINT_CHALLENGE);
        }
        K::TestingDay => {
            let nums = match (g("forty"), g("bench")) {
                (Some(f), Some(b)) => format!("{f} in the forty, {b} on the bench"),
                (Some(f), None) => format!("{f} in the forty"),
                _ => "Your numbers".into(),
            };
            if p.is("test", "better") || p.is("test", "first") {
                add(Move::Praise, Tone::Warm, format!("{nums}. The work shows."), HINT_PRAISE);
            }
            if p.is("test", "worse") {
                add(
                    Move::Challenge,
                    Tone::Firm,
                    format!("{nums}. That went backward. Fix it this summer."),
                    HINT_CHALLENGE,
                );
            }
            add(Move::Straight, Tone::Calm, format!("{nums}. Here's what I want next time."), HINT_STRAIGHT);
            add(Move::Ask, Tone::Warm, "How did testing feel?".into(), HINT_ASK);
        }
        K::PracticeWeek => {
            if p.is("grade", "best") {
                add(Move::Praise, Tone::Warm, "You were one of the best out there this week.".into(), HINT_PRAISE);
            }
            if p.is("grade", "worst") {
                add(Move::Challenge, Tone::Firm, "That was a rough week. Answer it.".into(), HINT_CHALLENGE);
            }
            if let (Some(won), Some(lost)) = (g("won"), g("lost")) {
                add(
                    Move::Straight,
                    Tone::Calm,
                    format!("You won {won} of your one-on-ones and lost {lost}. Here's what I saw."),
                    HINT_STRAIGHT,
                );
            }
            add(Move::Ask, Tone::Warm, "How did the week go for you?".into(), HINT_ASK);
        }
        K::ReportDay => {
            if p.is("shape", "great") {
                add(Move::Praise, Tone::Warm, "You came in ready. That's what I want to see.".into(), HINT_PRAISE);
            } else {
                add(
                    Move::Criticize,
                    Tone::Fiery,
                    g("pounds").map_or("You didn't come in ready.".into(), |n| {
                        format!("{n} pounds over. That's not a camp body.")
                    }),
                    HINT_CRITICIZE,
                );
                add(Move::Challenge, Tone::Firm, "Get it off in camp. I'll be watching.".into(), HINT_CHALLENGE);
            }
            add(Move::Straight, Tone::Calm, "Here's what camp needs from you.".into(), HINT_STRAIGHT);
        }
        K::SpringStep => {
            match p.check("step") {
                Some("came") => {
                    add(Move::Praise, Tone::Warm, "It came this spring. Everybody saw it.".into(), HINT_PRAISE)
                }
                Some("tipped") => {
                    add(Move::Praise, Tone::Warm, "Your coaches see something in you this spring.".into(), HINT_PRAISE)
                }
                _ => add(
                    Move::Straight,
                    Tone::Calm,
                    "The jump didn't come this spring. Here's what's next.".into(),
                    HINT_STRAIGHT,
                ),
            }
            add(Move::Challenge, Tone::Firm, "Keep going. Summer is where it's won.".into(), HINT_CHALLENGE);
            add(Move::Ask, Tone::Warm, "How do you feel about your spring?".into(), HINT_ASK);
        }
        K::ChartMove => {
            let rival = g("rival").map(|r| first(&r));
            if p.is("move", "up") {
                add(
                    Move::Praise,
                    Tone::Warm,
                    rival
                        .as_ref()
                        .map_or("You moved up. You earned it.".into(), |r| format!("You passed {r}. You earned it.")),
                    HINT_PRAISE,
                );
            } else {
                add(
                    Move::Straight,
                    Tone::Calm,
                    rival
                        .as_ref()
                        .map_or("You moved down, and here's why.".into(), |r| format!("{r} passed you. Here's why.")),
                    HINT_STRAIGHT,
                );
                add(Move::Challenge, Tone::Firm, "Take your spot back.".into(), HINT_CHALLENGE);
            }
            add(Move::Ask, Tone::Warm, "How do you see it?".into(), HINT_ASK);
        }
        K::NewPosition => {
            let pos = g("position").unwrap_or("the new spot".into());
            add(Move::Ask, Tone::Warm, format!("How's {pos} coming?"), HINT_ASK);
            if p.is("stage", "ready") {
                add(Move::Praise, Tone::Warm, format!("You look like you belong at {pos} now."), HINT_PRAISE);
            } else {
                add(Move::Straight, Tone::Calm, format!("It takes time at {pos}. Stay with it."), HINT_STRAIGHT);
            }
            add(Move::Challenge, Tone::Firm, "Own it. Make it your position.".into(), HINT_CHALLENGE);
        }
        K::RoomChanged => {
            let names = g("names").unwrap_or("the new guys".into());
            if p.is("change", "open") {
                add(Move::Straight, Tone::Calm, format!("With {names} gone, the road is open for you."), HINT_STRAIGHT);
            } else {
                add(
                    Move::Straight,
                    Tone::Calm,
                    format!("{names} came in at your spot. Nobody's job is safe."),
                    HINT_STRAIGHT,
                );
                add(
                    Move::Challenge,
                    Tone::Firm,
                    "They're here to take your job. Don't let them.".into(),
                    HINT_CHALLENGE,
                );
            }
            add(Move::Ask, Tone::Warm, "How do you feel about the room now?".into(), HINT_ASK);
        }
        K::NewSystem => {
            let sys = g("system").unwrap_or("system".into());
            add(Move::Ask, Tone::Warm, format!("How's the new {sys} coming?"), HINT_ASK);
            if p.is("grasp", "has_it") {
                add(Move::Praise, Tone::Warm, format!("You've got the {sys} down. It shows."), HINT_PRAISE);
            } else {
                add(
                    Move::Straight,
                    Tone::Calm,
                    format!("You're behind in the {sys}. Get in the playbook."),
                    HINT_STRAIGHT,
                );
            }
            add(Move::Challenge, Tone::Firm, "Know it better than anybody.".into(), HINT_CHALLENGE);
        }
        K::TeammateDrafted => {
            let name = g("name").map(|n| first(&n)).unwrap_or("him".into());
            add(Move::Ask, Tone::Warm, format!("How do you feel about {name} going?"), HINT_ASK);
            add(Move::Straight, Tone::Calm, "That can be you. Here's how.".into(), HINT_STRAIGHT);
            add(Move::Challenge, Tone::Firm, "That's the road. You're next.".into(), HINT_CHALLENGE);
        }
        K::OtherSport => {
            match (g("time"), g("event")) {
                (Some(t), Some(e)) => {
                    add(Move::Praise, Tone::Warm, format!("{t} in the {e}. That's real speed."), HINT_PRAISE)
                }
                _ => add(Move::Praise, Tone::Warm, "I hear you're having a good season.".into(), HINT_PRAISE),
            }
            add(Move::Ask, Tone::Warm, "How's the other season going?".into(), HINT_ASK);
            add(Move::Straight, Tone::Calm, "Enjoy it. Come back to us in shape.".into(), HINT_STRAIGHT);
        }
        K::Colleges => {
            let school = g("school").unwrap_or("the schools".into());
            add(Move::Ask, Tone::Warm, format!("How's recruiting going? I hear {school} is on you."), HINT_ASK);
            add(
                Move::Straight,
                Tone::Calm,
                "Keep your grades and your film right and the offers keep coming.".into(),
                HINT_STRAIGHT,
            );
            add(Move::Praise, Tone::Warm, "You earned that attention.".into(), HINT_PRAISE);
        }
        K::ProSpring => {
            match p.check("what") {
                Some("battle") => add(
                    Move::Challenge,
                    Tone::Firm,
                    g("rival").map_or("Somebody's coming for your job. Win it.".into(), |r| {
                        format!("{} is coming for your job. Win it.", first(&r))
                    }),
                    HINT_CHALLENGE,
                ),
                Some("talks") => add(Move::Ask, Tone::Warm, "How are the talks going?".into(), HINT_ASK),
                _ => add(
                    Move::Straight,
                    Tone::Calm,
                    "The club tagged you. Have the best year of your life.".into(),
                    HINT_STRAIGHT,
                ),
            }
            add(Move::Listen, Tone::Calm, "Hear him out. Promise nothing.".into(), HINT_LISTEN);
            add(Move::Straight, Tone::Calm, "Here's what this spring needs from you.".into(), HINT_STRAIGHT);
        }
        K::Ahead => {
            let what = g("what").unwrap_or("what's coming".into());
            add(Move::Ask, Tone::Warm, format!("Are you ready for {what}?"), HINT_ASK);
            add(
                Move::Straight,
                Tone::Calm,
                g("when").map_or(format!("{} is coming. Be ready.", cap(&what)), |d| {
                    format!("{} is {d}. Be ready.", cap(&what))
                }),
                HINT_STRAIGHT,
            );
            add(Move::Challenge, Tone::Firm, "Be the best one out there.".into(), HINT_CHALLENGE);
        }
        _ => {
            add(Move::Ask, Tone::Warm, "Tell me about it.".into(), HINT_ASK);
            add(Move::Listen, Tone::Calm, "Hear him out. Promise nothing.".into(), HINT_LISTEN);
        }
    }
    // One option a move.
    let mut seen: Vec<Move> = Vec::new();
    v.retain(|o| {
        let fresh = !seen.contains(&o.m);
        seen.push(o.m);
        fresh
    });
    v
}

/// What your team rules say comes next for an arrest: your book's conduct
/// rule as your staff would rule it (`rules::staff_response`).
pub fn book_says(w: &World, id: u32, level: Level) -> String {
    let prior = w.folk.rules.rulings.iter().filter(|r| r.id == id).count().min(2) as u8;
    rules::staff_response(w.folk.rules.strict(RuleKind::Conduct) + prior, level).words(level)
}

/// The picks for a point: its moves, each standing on the point; a
/// promise can't be taken back.
pub fn picks(w: &World, i: usize, p: &Point, used: &[Move], code: impl Fn(Move, Tone) -> u32) -> Vec<Pick> {
    moves(w, i, p)
        .into_iter()
        .filter(|o| !used.contains(&o.m))
        .map(|o| {
            Pick {
                label: o.label,
                mv: o.m.name().into(),
                tone: o.tone.name().into(),
                hint: o.hint.into(),
                code: code(o.m, o.tone),
                ..Pick::default()
            }
            .on(p.kind.key())
            .of(if o.m == Move::Promise { PickKind::Final } else { PickKind::Plain })
        })
        .collect()
}

/// The promise a move on a point logs, as its words say: a contract talk;
/// a start (the job back, his time coming, a start next year), since a
/// man who lost his job or sits behind someone doesn't start today; else
/// his place. Any tone of the move logs the same (`scene::same_move`).
pub fn promise_kind(w: &World, i: usize, k: K) -> PromiseKind {
    let starter = talks::is_starter(w, i);
    let rank = if starter { 0 } else { w.people[i].last_role.map_or(PlayingTime::Backup as u8, |r| r as u8) };
    match k {
        K::Pay | K::WantsOut => PromiseKind::Contract,
        K::Portal | K::LostJob | K::Buried => PromiseKind::Start,
        _ => PromiseKind::Place(rank),
    }
}

// ---------------------------------------------------------------------------
// How much a move counts (section 5, "How much it counts").

/// What a move rests on: the multiplier of its size, the trust a move
/// against the facts costs, whether he needs an ear, and how true praise
/// rings.
#[derive(Clone, Copy, Debug)]
pub struct Rest {
    pub k: f32,
    pub against: bool,
    pub cost: f32,
    pub hurting: bool,
    pub truth: Option<f32>,
}

/// A move on a topic: on a point, the sheet's basis (on his mind x1.3, a
/// true fact x1.0), smaller each time you come back to it (`Talked::
/// again`); a warning or criticism of a man who did nothing is against
/// the facts. On a subject, today's rule: x1.3 on his real worry, x0.7 off
/// it, and a warning with nothing behind it is against the facts.
pub fn rest(w: &World, i: usize, sheet: &Sheet, topic: Topic, m: Move, revealed: bool) -> Rest {
    let me = &w.people[i];
    let worry = talks::worry_on(w, i, sheet);
    let hurting = worry.as_ref().is_some_and(|x| matches!(x.0.subject(), Subject::Life | Subject::Friend));
    let ego = me.player.personality.traits.z(Trait::Ego);
    let scold = matches!(m, Move::Warn | Move::Criticize);
    let mut r = Rest { k: 1.0, against: false, cost: 0.0, hurting, truth: None };
    match topic {
        Topic::Point(k) => {
            let p = sheet.get(k);
            // Against the facts only when the facts are public: a case that
            // was dropped or that he won. While a case is open only he knows
            // (`did` reads the hidden truth), so nothing is against it.
            let against = scold && k == K::CaseClosed && p.is_some_and(|p| !did(p));
            let basis = if against { Basis::Against } else { sheet.basis(k) };
            let again = p.map_or(1.0, |p| w.folk.talked.again(w.today, me.player.id, p));
            let (plain, lift) = (claims::sheet::PLAIN, basis.lift());
            r.k = if again >= 1.0 { lift } else { plain + (lift - plain).max(0.0) * again };
            r.against = against;
            r.cost = basis.trust_cost(ego);
            // Praise of a fact is true: a game, an award, a job won.
            if m == Move::Praise && lean_of_kind(sheet, k) {
                r.truth = Some(1.0);
            }
        }
        Topic::Subject(s) => {
            let on = revealed || worry.as_ref().is_some_and(|x| x.0.subject() == s);
            r.k = if on { claims::sheet::ON_MIND } else { claims::sheet::PLAIN };
            if scold && s == Subject::Behavior && !sheet.trouble() && !sheet.has(K::Effort) {
                r.against = true;
                r.cost = Basis::Against.trust_cost(ego);
            }
        }
    }
    r
}

/// A kind that is trouble (what getting him help answers).
pub fn lean_is_trouble(k: K) -> bool {
    k.is_trouble() || k == K::Effort
}

fn lean_of_kind(sheet: &Sheet, k: K) -> bool {
    sheet.get(k).is_some_and(|p| lean_of(p) == Lean::Good)
}

// ---------------------------------------------------------------------------
// He asks you (section 5, "He asks you").

/// Your four answers to his question, in code order (200 + 10 x answer +
/// tone).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Straight,
    Soften,
    Promise,
    Dodge,
}

impl Answer {
    pub const ALL: [Answer; 4] = [Answer::Straight, Answer::Soften, Answer::Promise, Answer::Dodge];
    pub fn key(self) -> &'static str {
        match self {
            Answer::Straight => "straight",
            Answer::Soften => "soften",
            Answer::Promise => "promise",
            Answer::Dodge => "dodge",
        }
    }
    fn tone(self) -> Tone {
        match self {
            Answer::Straight => Tone::Calm,
            Answer::Soften | Answer::Promise => Tone::Warm,
            Answer::Dodge => Tone::Firm,
        }
    }
    pub fn code(self) -> u32 {
        200 + 10 * Answer::ALL.iter().position(|a| *a == self).unwrap_or(0) as u32
            + match self.tone() {
                Tone::Calm => 0,
                Tone::Warm => 1,
                Tone::Firm => 2,
                Tone::Fiery => 3,
            }
    }
    pub fn of(code: u32) -> Option<Answer> {
        if !(200..240).contains(&code) {
            return None;
        }
        Answer::ALL.get(((code - 200) / 10) as usize).copied()
    }
}

/// A question he asks: the point it's on (none for your job, which is no
/// point of his), his words when the bank has none, and your answers.
pub struct Question {
    pub key: &'static str,
    pub fallback: String,
    pub answers: Vec<(Answer, String, &'static str)>,
}

/// The question a point comes as, if it has one.
pub fn question(w: &World, i: usize, p: &Point) -> Option<Question> {
    let g = |h: &str| p.get(h).map(str::to_string);
    let q = match p.kind {
        K::LostJob | K::Buried => Question {
            key: p.kind.key(),
            fallback: if p.kind == K::LostJob {
                "What do I have to do to get my job back, Coach?".into()
            } else {
                "Be honest with me, Coach. What do I have to do to get on the field?".into()
            },
            answers: vec![
                (
                    Answer::Straight,
                    match (p.kind, g("rival").or(g("ahead"))) {
                        (_, Some(r)) => format!("Be better than {} every day in practice. That's it.", first(&r)),
                        _ => "Be better than the man ahead of you every day. That's it.".into(),
                    },
                    HINT_STRAIGHT,
                ),
                (
                    Answer::Soften,
                    "You're closer than you think. Keep at it.".into(),
                    "Easier to hear; he may hear more than you meant.",
                ),
                (Answer::Promise, "Keep working like this and the job is yours.".into(), HINT_START),
                (Answer::Dodge, "We'll see how the week goes.".into(), "He notices a dodge."),
            ],
        },
        K::PromiseOpen => Question {
            key: p.kind.key(),
            // A promise of his place reads in the third person ("that his
            // place wouldn't change"): he says it his own way.
            fallback: match g("promise").filter(|x| !x.contains("he'd") && !x.contains("his ")) {
                Some(x) => format!("You promised me {x}, Coach. Is that still happening?"),
                None => "You made me a promise, Coach. Is that still happening?".into(),
            },
            answers: vec![
                (Answer::Straight, "Here's exactly where it stands.".into(), HINT_STRAIGHT),
                (Answer::Soften, "I haven't forgotten. Be patient with me.".into(), "Easier to hear; he waits."),
                (
                    Answer::Promise,
                    "I gave you my word, and I'll keep it.".into(),
                    "He holds you to the promise you made.",
                ),
                (Answer::Dodge, "Let's not worry about that today.".into(), "He notices a dodge."),
            ],
        },
        K::Draft => Question {
            key: p.kind.key(),
            fallback: "What are you hearing about me, Coach? From the scouts?".into(),
            answers: vec![
                (
                    Answer::Straight,
                    match p.check("round") {
                        Some("early") => "They have you going early. That's what I hear.".into(),
                        Some("middle") => "They have you in the middle rounds. That's what I hear.".into(),
                        Some("late") => "They have you going late. That's what I hear.".into(),
                        _ => "Honestly? They don't have you drafted yet.".into(),
                    },
                    HINT_STRAIGHT,
                ),
                (
                    Answer::Soften,
                    "Some teams like you. Let's see where it goes.".into(),
                    "Easier to hear; he may hear more than you meant.",
                ),
                (Answer::Dodge, "Worry about this season first.".into(), "He notices a dodge."),
            ],
        },
        _ => return None,
    };
    Some(with_bank_answers(w, i, p, q))
}

/// Your answers in the bank's words where it has them (`talk.point.answer.
/// <point>.<answer>`, lane L12).
fn with_bank_answers(w: &World, i: usize, p: &Point, mut q: Question) -> Question {
    for (a, label, _) in q.answers.iter_mut() {
        let key = format!("talk.point.answer.{}.{}", q.key, a.key());
        *label = you_words(w, i, p, &key, std::mem::take(label));
    }
    q
}

/// The question when your name is in a job story (your seat at a college
/// on the hot seat): asked once a season.
pub fn job_question(w: &World, i: usize) -> Option<Question> {
    let Some(super::Team::College(j)) = super::Team::seat(w) else { return None };
    let k = w.programs.get(j)?.head_coach?;
    if !claims::hot_seat(w, k) {
        return None;
    }
    let id = w.people[i].player.id;
    if w.folk.talked.remembers(w.today, id, &Point::new(K::YourJob)) {
        return None;
    }
    Some(with_bank_answers(w, i, &Point::new(K::YourJob), Question {
        key: K::YourJob.key(),
        fallback: "Can I ask you something, Coach? Everybody's saying you're on the hot seat. Are you going to be here next year?".into(),
        answers: vec![
            (Answer::Straight, "Honestly? It depends on how we finish. That's the business.".into(), HINT_STRAIGHT),
            (Answer::Soften, "Don't worry about me. Worry about your game.".into(), "Easier to hear; he may hear more than you meant."),
            (Answer::Promise, "I'm not going anywhere. I'll be here.".into(), "Logged: you stay a year."),
            (Answer::Dodge, "That's not your concern.".into(), "He notices a dodge."),
        ],
    }))
}

/// What an answer does: (trust, mood), before his trust colours it. A
/// straight answer and a promise are their moves; a softened one is a
/// small lift, more for a shaky man; a dodge costs a little.
pub fn answer_landing(w: &World, i: usize, a: Answer) -> Option<(f32, f32)> {
    let shaky = ((10.0 - w.people[i].bg.stability) / 10.0).max(0.0) as f32;
    match a {
        Answer::Soften => Some((SOFTEN.0 + SOFTEN_SHAKY.0 * shaky, SOFTEN.1 + SOFTEN_SHAKY.1 * shaky)),
        Answer::Dodge => Some(DODGE),
        _ => None,
    }
}

/// The move an answer counts as, when it is one.
pub fn answer_move(a: Answer) -> Option<Move> {
    match a {
        Answer::Straight => Some(Move::Straight),
        Answer::Promise => Some(Move::Promise),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Off the talk: an arrest, and getting him help.

/// An arrest moves his mood (section 5, "What an arrest does outside the
/// talk"): shame for a professional, anger for one who isn't, bigger for
/// a shaky man.
pub fn arrest_mood(w: &mut World, i: usize) {
    let p = &w.people[i];
    let shame = p.player.personality.traits.z(Trait::Professionalism) >= 0.0;
    let size = ARREST_HIT * (1.0 + 0.05 * (10.0 - p.bg.stability) as f32).clamp(0.6, 1.5);
    let id = p.player.id;
    talks::mood_event(w, id, if shame { Reason::ArrestShame } else { Reason::ArrestAnger }, -size);
}

/// Whether you got him help in the last year.
pub fn helped(w: &World, id: u32) -> bool {
    w.folk.rules.helped.iter().any(|x| x.0 == id && x.1.days_to(w.today) < HELP_DAYS)
}

/// The players you got help for in the last year (for the trouble rolls,
/// read before they run on all cores).
pub fn helped_ids(w: &World) -> std::collections::HashSet<u32> {
    w.folk.rules.helped.iter().filter(|x| x.1.days_to(w.today) < HELP_DAYS).map(|x| x.0).collect()
}

/// You get him help: his risk of more trouble falls `HELP_CUT` for a year,
/// once (it doesn't stack).
pub fn help(w: &mut World, id: u32) -> bool {
    if helped(w, id) {
        return false;
    }
    let today = w.today;
    w.folk.rules.helped.retain(|x| x.1.days_to(today) < HELP_DAYS);
    w.folk.rules.helped.push((id, today));
    true
}

// ---------------------------------------------------------------------------
// The AI coaches' talks (rule 8: the same reading).

/// An AI head coach's one-on-one with an unhappy player: the topic is what
/// is on his mind that the coach knows (else the subject of his worry).
/// The coach can say anything you could: every move the topic offers, in
/// every tone, so a wrong word or a fiery tone to a fragile man is on his
/// table as it is on yours. He reads the man right as often as an AI coach
/// always has, 0.25 + 0.035 x his relationships (`AI_READ_RIGHT`): then he
/// picks the move that lands best; else he misreads him and picks one of
/// the moves that land worse than the middle one. It lands as yours would.
/// No promise (an AI coach's word is not logged) and no side effects.
/// Returns (trust, mood).
pub fn ai_talk(w: &World, i: usize, rel: f32, seed: u64) -> (f32, f32) {
    let sheet = claims::player(w, i);
    let topic = sheet
        .known()
        .find(|p| talkable(p.kind) && p.mind >= WORRY_FLOOR)
        .map(|p| Topic::Point(p.kind))
        .or_else(|| talks::worry_on(w, i, &sheet).map(|x| x.0))
        .unwrap_or(Topic::Subject(Subject::Role));
    let mut ms: Vec<Move> = match topic {
        Topic::Point(k) => sheet.get(k).map_or_else(Vec::new, |p| moves(w, i, p).into_iter().map(|o| o.m).collect()),
        Topic::Subject(_) => Vec::new(),
    };
    if ms.is_empty() {
        ms = AI_SUBJECT_MOVES.to_vec();
    }
    ms.retain(|m| *m != Move::Promise);
    let subject = topic.subject();
    let mut opts: Vec<(f32, Move, Tone, Rest)> = ms
        .iter()
        .flat_map(|&m| [Tone::Calm, Tone::Warm, Tone::Firm, Tone::Fiery].map(|t| (m, t)))
        .map(|(m, t)| {
            let r = rest(w, i, &sheet, topic, m, false);
            let (dt, dm) = talks::landing(w, i, m, t, subject, r, seed);
            (dt + dm, m, t, r)
        })
        .collect();
    if opts.is_empty() {
        return (0.0, 0.0);
    }
    // Best first; ties keep the order they were made in.
    opts.sort_by(|a, b| b.0.total_cmp(&a.0));
    let id = w.people[i].player.id as u64;
    let right = (super::roll(w, seed ^ 0xA1_0000, id) as f32) < AI_READ_RIGHT.0 + AI_READ_RIGHT.1 * rel;
    let k = if right {
        0
    } else {
        let low = opts.len() / 2;
        low + ((super::roll(w, seed ^ 0xA2_0000, id) * (opts.len() - low) as f64) as usize).min(opts.len() - low - 1)
    };
    let (_, m, t, r) = opts[k];
    talks::landing(w, i, m, t, subject, r, seed)
}

/// What an AI coach can say on a subject with no point behind it: the
/// moves anyone can make, short of a promise, a warning or a hand.
const AI_SUBJECT_MOVES: [Move; 6] =
    [Move::Ask, Move::Straight, Move::Praise, Move::Challenge, Move::Criticize, Move::Listen];

/// The player's own speaker and his facts for a line on a point.
pub fn point_facts(base: &Facts, p: &Point, st: Option<Stance>) -> Facts {
    let mut f = p.facts(base);
    if let Some(s) = st {
        f.set("stance", s.key());
    }
    f
}
