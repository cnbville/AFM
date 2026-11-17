//! How you talk (`docs/design/coach-voice.md`, agreed 2026-11-17): the ear
//! reads five dials from every typed line (heat, language, aim, care,
//! tell or ask), each person keeps what he has seen of you and what he has
//! heard, the game names where your dials sit with eight words, and the
//! same line lands differently on each man by what he wants from a coach,
//! what he has with you and where it was said. It moves only built
//! channels: trust, mood (and through it growth), game-day fire, portal
//! leans through trust and mood, staff loyalty and who leaves, your boss's
//! trust, recruits through your name. No new on-field bonus.
//!
//! AI coaches talk in styles of their own, from their temper,
//! relationships, motivation and discipline, and their players and staffs
//! react by the same rules (`ai`, `ai_fit`, `offer_pull`, `for_cause`).
//!
//! Every size here is Claude's, proposed, not confirmed, until a trial
//! measures it (the spec's own words).

use super::Them;
use super::makeup;
use super::room::{Channel, Place, Room, Seat};
use super::wants::Kind;
use crate::calendar::Date;
use crate::game::talk::Tone;
use crate::player::Trait;
use crate::world::World;
use crate::world::folk::Team;

/// One line's manner (section 1), each dial 0-1.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Manner {
    /// Calm 0 to volcanic 1.
    pub heat: f32,
    /// Clean 0 to profane 1.
    pub profane: f32,
    /// Aimed out (the opponent, the job, the moment) 0 to at the man
    /// himself 1.
    pub aim: f32,
    /// Cold, business 0 to "he hears you care about him" 1.
    pub care: f32,
    /// You tell 0 to you ask, you want his input 1.
    pub ask: f32,
}

/// The ear's words for each dial, low to high, with the value each reads
/// as. The schema offers these words; anything else reads as the middle.
pub const HEAT: [(&str, f32); 5] = [("calm", 0.05), ("even", 0.25), ("firm", 0.45), ("hot", 0.75), ("volcanic", 0.95)];
pub const LANGUAGE: [(&str, f32); 3] = [("clean", 0.0), ("salty", 0.4), ("profane", 0.9)];
pub const AIM: [(&str, f32); 3] = [("out", 0.05), ("work", 0.35), ("man", 0.9)];
pub const CARE: [(&str, f32); 3] = [("cold", 0.05), ("neutral", 0.4), ("caring", 0.85)];
pub const SAY: [(&str, f32); 3] = [("tell", 0.1), ("mixed", 0.5), ("ask", 0.9)];

fn word(list: &[(&str, f32)], w: &str) -> f32 {
    list.iter().find(|x| x.0 == w).map_or(list[list.len() / 2].1, |x| x.1)
}

fn near(list: &[(&'static str, f32)], v: f32) -> &'static str {
    list.iter().min_by(|a, b| (a.1 - v).abs().total_cmp(&(b.1 - v).abs())).map_or("", |x| x.0)
}

impl Manner {
    /// The ear's five words.
    pub fn from_words(heat: &str, language: &str, aim: &str, care: &str, say: &str) -> Manner {
        Manner {
            heat: word(&HEAT, heat),
            profane: word(&LANGUAGE, language),
            aim: word(&AIM, aim),
            care: word(&CARE, care),
            ask: word(&SAY, say),
        }
    }

    /// A chip's tone (no key): heat and care from the tone, clean, the
    /// bank's lines.
    pub fn of_tone(t: Tone) -> Manner {
        match t {
            Tone::Calm => Manner { heat: 0.15, profane: 0.0, aim: 0.2, care: 0.45, ask: 0.3 },
            Tone::Warm => Manner { heat: 0.3, profane: 0.0, aim: 0.1, care: 0.85, ask: 0.4 },
            Tone::Firm => Manner { heat: 0.5, profane: 0.0, aim: 0.35, care: 0.35, ask: 0.15 },
            Tone::Fiery => Manner { heat: 0.85, profane: 0.0, aim: 0.2, care: 0.35, ask: 0.05 },
        }
    }

    /// The talks' own tone (one-on-ones and team talks), the same four.
    pub fn of_talk(t: crate::world::folk::takeover::Tone) -> Manner {
        use crate::world::folk::takeover::Tone as T;
        Manner::of_tone(match t {
            T::Calm => Tone::Calm,
            T::Warm => Tone::Warm,
            T::Firm => Tone::Firm,
            T::Fiery => Tone::Fiery,
        })
    }

    fn dials(&self) -> [f32; 5] {
        [self.heat, self.profane, self.aim, self.care, self.ask]
    }

    fn from_dials(d: [f32; 5]) -> Manner {
        Manner { heat: d[0], profane: d[1], aim: d[2], care: d[3], ask: d[4] }
    }

    /// The line in the ear's own words ("hot, profane, at the man, cold,
    /// tell").
    pub fn words(&self) -> String {
        format!(
            "{}, {}, aimed {}, {}, {}",
            near(&HEAT, self.heat),
            near(&LANGUAGE, self.profane),
            match near(&AIM, self.aim) {
                "out" => "out",
                "work" => "at the work",
                _ => "at the man",
            },
            near(&CARE, self.care),
            near(&SAY, self.ask)
        )
    }
}

/// What a person has of you, as weighted sums of the dials, fading by
/// `HALF_LIFE_DAYS` (section 2: one bad day doesn't define you, a year of
/// the same does).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Acc {
    pub w: f32,
    pub sum: [f32; 5],
    pub day: Date,
}

/// A read's weight halves in this many days: about a season to fade.
/// Proposed, not confirmed.
pub const HALF_LIFE_DAYS: f32 = 120.0;

impl Acc {
    fn faded(&self, d: Date) -> Acc {
        let days = self.day.days_to(d).max(0) as f32;
        let k = 0.5f32.powf(days / HALF_LIFE_DAYS);
        let mut a = *self;
        a.w *= k;
        for s in a.sum.iter_mut() {
            *s *= k;
        }
        a.day = d;
        a
    }
    pub fn add(&mut self, m: Manner, weight: f32, d: Date) {
        *self = self.faded(d);
        self.w += weight;
        for (s, v) in self.sum.iter_mut().zip(m.dials()) {
            *s += weight * v;
        }
    }
    /// Its weight on a day.
    pub fn weight(&self, d: Date) -> f32 {
        self.faded(d).w
    }
    /// The average line, once there is anything to average.
    pub fn mean(&self) -> Option<Manner> {
        (self.w > 0.05).then(|| Manner::from_dials(self.sum.map(|s| (s / self.w).clamp(0.0, 1.0))))
    }
    fn blend(&mut self, other: &Acc, share: f32, d: Date) {
        let o = other.faded(d);
        *self = self.faded(d);
        self.w += o.w * share;
        for (s, v) in self.sum.iter_mut().zip(o.sum) {
            *s += v * share;
        }
    }
}

/// A complaint about how you treat people (section 4: bosses).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Complaint {
    /// Pressure aimed at a man with nothing between you to carry it.
    Abuse,
    /// A high school parent's call about cursing at the kids.
    ParentCall,
    /// The trainer or a doctor on pressure on a hurt player.
    PlayHurt,
    /// A coordinator who was gone at, to a beat writer.
    Leak,
}

/// One complaint: about whom, by whom, and when.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Filed {
    pub kind: Complaint,
    pub who: Them,
    pub day: Date,
}

/// An outside review of how you treat players (Maryland 2018, Illinois
/// 2015): opened on a pattern of complaints, ruled on after `REVIEW_DAYS`.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Review {
    pub opened: Date,
    /// Complaints on file the day it opened.
    pub at_open: u16,
}

/// The coach-voice stores, kept on `Folk.manner` (chips count, so it lives
/// beside the live talks' stores, not in them). An empty book is never
/// written.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Book {
    /// What each person has seen of you: lines said to him and in a room
    /// he was in.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub seen: Vec<(Them, Acc)>,
    /// What your building says about you (the grapevine): your players and
    /// staff hear it, and a recruit from his host.
    pub room: Acc,
    /// What the press has: every public line, heard by everyone outside.
    pub public: Acc,
    /// Every line you've said: your own read, for the words.
    pub yours: Acc,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub complaints: Vec<Filed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<Review>,
    /// Young assistants who copy you (your coaching tree): the coach and
    /// how far, 0-1.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub copied: Vec<(usize, f32)>,
}

impl Book {
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
            && self.room.w == 0.0
            && self.public.w == 0.0
            && self.yours.w == 0.0
            && self.complaints.is_empty()
            && self.review.is_none()
            && self.copied.is_empty()
    }
}

/// The most people kept in `Book.seen` (the least seen go first).
pub const SEEN_KEPT: usize = 900;

fn book(w: &World) -> &Book {
    &w.folk.manner
}

fn book_mut(w: &mut World) -> &mut Book {
    &mut w.folk.manner
}

// --- The prototypes (section 3) ---------------------------------------------

/// The eight words for where your dials sit. No real coach is ever named in
/// the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Prototype {
    Firebreather,
    DrillSergeant,
    Needler,
    Ceo,
    PlayersCoach,
    Preacher,
    Showman,
    Professor,
}

impl Prototype {
    pub const ALL: [Prototype; 8] = [
        Prototype::Firebreather,
        Prototype::DrillSergeant,
        Prototype::Needler,
        Prototype::Ceo,
        Prototype::PlayersCoach,
        Prototype::Preacher,
        Prototype::Showman,
        Prototype::Professor,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Prototype::Firebreather => "firebreather",
            Prototype::DrillSergeant => "drill_sergeant",
            Prototype::Needler => "needler",
            Prototype::Ceo => "ceo",
            Prototype::PlayersCoach => "players_coach",
            Prototype::Preacher => "preacher",
            Prototype::Showman => "showman",
            Prototype::Professor => "professor",
        }
    }
    pub fn from_key(k: &str) -> Option<Prototype> {
        Prototype::ALL.into_iter().find(|p| p.key() == k)
    }
    pub fn name(self) -> &'static str {
        match self {
            Prototype::Firebreather => "Firebreather",
            Prototype::DrillSergeant => "Drill Sergeant",
            Prototype::Needler => "Needler",
            Prototype::Ceo => "CEO",
            Prototype::PlayersCoach => "Players' Coach",
            Prototype::Preacher => "Preacher",
            Prototype::Showman => "Showman",
            Prototype::Professor => "Professor",
        }
    }
    /// What it is, in a line for the screens.
    pub fn about(self) -> &'static str {
        match self {
            Prototype::Firebreather => "Hot, often profane, aimed at the opponent and at effort. Tells.",
            Prototype::DrillSergeant => "Hot, aimed at the man, cold.",
            Prototype::Needler => "Low volume, aimed at the man on purpose, provokes.",
            Prototype::Ceo => "Calm, clean, cold. Tells.",
            Prototype::PlayersCoach => "Calm to warm, caring, asks.",
            Prototype::Preacher => "High energy, clean, caring, aimed out.",
            Prototype::Showman => "Loud in public, brash, aimed at rivals.",
            Prototype::Professor => "Calm, asks, explains.",
        }
    }
    /// Where its dials sit.
    pub fn center(self) -> Manner {
        let m = |heat, profane, aim, care, ask| Manner { heat, profane, aim, care, ask };
        match self {
            Prototype::Firebreather => m(0.8, 0.65, 0.25, 0.4, 0.1),
            Prototype::DrillSergeant => m(0.8, 0.45, 0.75, 0.1, 0.05),
            Prototype::Needler => m(0.3, 0.25, 0.8, 0.15, 0.2),
            Prototype::Ceo => m(0.15, 0.0, 0.3, 0.2, 0.15),
            Prototype::PlayersCoach => m(0.2, 0.15, 0.15, 0.85, 0.7),
            Prototype::Preacher => m(0.7, 0.0, 0.1, 0.8, 0.2),
            Prototype::Showman => m(0.75, 0.35, 0.15, 0.35, 0.1),
            Prototype::Professor => m(0.1, 0.0, 0.3, 0.5, 0.8),
        }
    }
}

fn dist(a: &Manner, b: &Manner) -> f32 {
    // Aim weighs most (section 4: "aim matters most"), language least.
    let k = [1.0, 0.6, 1.4, 1.0, 0.8];
    a.dials().iter().zip(b.dials()).zip(k).map(|((x, y), k)| k * (x - y) * (x - y)).sum::<f32>().sqrt()
}

/// The nearest words for a read, and a second when it is close ("a
/// Firebreather with a Players' Coach side"). The Showman is the loud
/// public face: he is named only when your public lines run hotter than
/// your private ones.
pub fn prototypes(m: &Manner, public: Option<&Manner>) -> (Prototype, Option<Prototype>) {
    let showman = public.is_some_and(|p| p.heat >= 0.6 && p.heat > m.heat + 0.1);
    let mut all: Vec<(Prototype, f32)> = Prototype::ALL
        .into_iter()
        .filter(|&p| p != Prototype::Showman || showman)
        .map(|p| {
            (p, if p == Prototype::Showman { dist(public.unwrap_or(m), &p.center()) } else { dist(m, &p.center()) })
        })
        .collect();
    all.sort_by(|a, b| a.1.total_cmp(&b.1));
    let first = all[0];
    let second = all.get(1).filter(|s| s.1 - first.1 < 0.12).map(|s| s.0);
    (first.0, second)
}

/// The words for a read: "a Firebreather", "a Firebreather with a
/// Players' Coach side".
pub fn words_for(m: &Manner, public: Option<&Manner>) -> String {
    let (a, b) = prototypes(m, public);
    let an = |p: Prototype| if matches!(p.name().as_bytes()[0], b'A' | b'E' | b'I' | b'O' | b'U') { "an" } else { "a" };
    match b {
        Some(b) => format!("{} {} with {} {} side", an(a), a.name(), an(b), b.name()),
        None => format!("{} {}", an(a), a.name()),
    }
}

// --- Your read: your profile's start, then your lines (section 9.1) --------

/// How much line weight your profile's reputation starts at: about four
/// talks' worth, fading on the same half-life, so after a season your
/// real lines have taken over. Proposed, not confirmed.
pub const START_WEIGHT: f32 = 4.0;

/// Your profile's pick (none for a blank start).
pub fn start_pick(w: &World) -> Option<Prototype> {
    w.career.as_ref().and_then(|c| Prototype::from_key(&c.you.profile.manner))
}

fn start_acc(w: &World) -> Option<Acc> {
    let p = start_pick(w)?;
    let started = w.career.as_ref()?.started;
    let mut a = Acc::default();
    a.add(p.center(), START_WEIGHT, started);
    Some(a.faded(w.today))
}

/// Your read as the world names it: your lines with your reputation from
/// your profile, the words, and the public face. None before you've said
/// anything with no reputation picked.
pub fn your_read(w: &World) -> Option<(Manner, String)> {
    let b = book(w);
    let mut a = b.yours.faded(w.today);
    if let Some(s) = start_acc(w) {
        a.blend(&s, 1.0, w.today);
    }
    let m = a.mean()?;
    let public = b.public.faded(w.today);
    let p = (public.w >= 1.0).then(|| public.mean()).flatten();
    Some((m, words_for(&m, p.as_ref())))
}

/// What a person has of you (section 2): what he saw, what he heard (your
/// building's grapevine inside it, the press outside, a recruit's host),
/// and your reputation from your profile. The weight says how thick it is:
/// a freshman in week one has your reputation, not you.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub manner: Manner,
    pub weight: f32,
    /// What he saw himself, its weight.
    pub own: f32,
}

/// The share of the grapevine's and the press's weight a man hears.
/// Proposed, not confirmed.
pub const HEARD_SHARE: f32 = 0.35;

/// Whether he is inside your building: your player, your staff, your
/// boss, your support staff.
fn inside(w: &World, him: Them) -> bool {
    matches!(makeup::kind(w, him), Kind::Player | Kind::Staff | Kind::Boss)
}

pub fn seen_of(w: &World, him: Them) -> Option<Seen> {
    let b = book(w);
    let d = w.today;
    let mut a = Acc { day: d, ..Default::default() };
    let own = b.seen.iter().find(|x| x.0 == him).map(|x| x.1.faded(d)).unwrap_or_default();
    a.blend(&own, 1.0, d);
    if inside(w, him) {
        a.blend(&b.room, HEARD_SHARE, d);
    } else {
        a.blend(&b.public, HEARD_SHARE, d);
        // A recruit and his family hear what you're like from the host on
        // his visit, and from your players back home.
        if matches!(makeup::kind(w, him), Kind::Recruit | Kind::Family | Kind::Transfer) {
            a.blend(&b.room, 0.5 * HEARD_SHARE, d);
        }
    }
    if let Some(s) = start_acc(w) {
        a.blend(&s, 1.0, d);
    }
    Some(Seen { manner: a.mean()?, weight: a.w, own: own.w })
}

// --- How it lands (section 4) -------------------------------------------

/// What he wants from a coach, from his makeup and his place (section 4,
/// part 1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wants {
    /// Wants it hard (+1: competitive, coachable, steady) to shuts down
    /// under it (-1).
    pub hard: f32,
    /// Shaky nerves, 0-1.
    pub shaky: f32,
    /// Pride, 0-1.
    pub proud: f32,
    /// A professional: all business (-1 to 1).
    pub pro: f32,
    /// He swears himself.
    pub swears: bool,
    /// A minor: a high school kid.
    pub minor: bool,
    /// A grown man with money and years (0-1): an NFL veteran, a coach who
    /// has been a head coach.
    pub veteran: f32,
    /// An NFL player: language is normal there.
    pub nfl: bool,
}

fn z(v: f64) -> f32 {
    ((v - 10.0) / 10.0) as f32
}

/// A player's wants from a coach, from his personality alone (what the AI
/// side reads too, with no `Them` lookups).
pub fn player_wants(p: &crate::world::Person) -> Wants {
    let pers = &p.player.personality;
    let t = &pers.traits;
    let comp = z(pers.competitiveness);
    let coach = z(pers.coachability);
    let shaky = ((10.0 - p.bg.stability) / 10.0).clamp(0.0, 1.0) as f32;
    let temper = t.z(Trait::Temper) as f32;
    let pro = t.z(Trait::Professionalism) as f32;
    let nfl = p.pro.is_some() && p.stage == crate::world::Stage::Pro;
    let years = p.pro.as_ref().map_or(0, |c| c.accrued) as f32;
    Wants {
        hard: (0.5 * comp + 0.3 * coach - 0.6 * shaky + 0.15 * temper).clamp(-1.0, 1.0),
        shaky,
        proud: (t.z(Trait::Ego) as f32).max(0.0),
        pro,
        swears: t.get(Trait::Temper) >= 13.0 || t.get(Trait::Professionalism) < 9.0,
        minor: matches!(p.stage, crate::world::Stage::HighSchool | crate::world::Stage::PrepSchool),
        veteran: if nfl { (years / 6.0).clamp(0.0, 1.0) } else { 0.0 },
        nfl,
    }
}

/// Anyone's wants from a coach.
pub fn wants(w: &World, him: Them) -> Wants {
    if let Them::Player(id) = him
        && let Some(i) = crate::world::folk::person(w, id)
    {
        return player_wants(&w.people[i]);
    }
    let m = makeup::makeup(w, him);
    let zz = |v: f64| ((v - 10.0) / 10.0) as f32;
    let veteran = match him {
        // A former head coach, or a man past 50 in the business.
        Them::Coach(c) => w.coaches.get(c).map_or(0.0, |co| {
            let head = co.history.iter().any(|s| s.role == crate::world::coach::Role::HeadCoach);
            if head || co.age >= 50 { 1.0 } else { (co.age as f32 - 30.0).max(0.0) / 25.0 }
        }),
        Them::Kin(..) => 0.6,
        _ => 0.7,
    };
    Wants {
        hard: (0.3 * zz(m.temper) + 0.3 * zz(m.ambition) - 0.25 * zz(m.warmth)).clamp(-1.0, 1.0),
        shaky: 0.0,
        proud: zz(m.ego).max(0.0),
        pro: zz(m.professionalism),
        swears: m.temper >= 13.0 || m.professionalism < 9.0,
        minor: false,
        veteran,
        nfl: false,
    }
}

/// Where and when (section 4, part 3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Setting {
    /// Others hear it: a room, the team, staff in the room.
    pub public: bool,
    /// His family is there, or it's his home.
    pub family: bool,
    /// The game this week was lost.
    pub lost: bool,
}

pub fn setting(w: &World, room: &Room) -> Setting {
    let family = matches!(room.channel, Channel::InPerson(Place::Home))
        || room.present.iter().any(|p| matches!(p.seat, Seat::Decider | Seat::Family));
    let others = room.present.iter().filter(|p| !matches!(p.seat, Seat::Across | Seat::Decider | Seat::Family)).count();
    let public = matches!(room.channel, Channel::Group(_)) || others > 0;
    let lost = Team::seat(w).and_then(|t| crate::world::folk::facts::fresh_game(w, t)).is_some_and(|g| !g.0);
    Setting { public, family, lost }
}

/// How he took your manner, for the voice (section 7) and the store.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TookAs {
    /// Nothing about how you said it stood out.
    #[default]
    Nothing,
    /// Your fire lit him up.
    FiredUp,
    /// Hard on him, and he knows it's because you care.
    ToughLove,
    /// The heat does nothing for him.
    Flat,
    /// Aimed at him, and he's shutting down.
    ShutDown,
    /// He took it as you going at him personally.
    Hurt,
    /// He won't be talked to like that.
    PushesBack,
    /// The language bothered him (or his family).
    Language,
    /// He hears you care about him.
    Cared,
    /// He likes being asked.
    Heard,
}

impl TookAs {
    /// The voice's line for it (section 7: he answers in kind).
    pub fn voice(self, family: bool) -> &'static str {
        match self {
            TookAs::Nothing => "",
            TookAs::FiredUp => {
                "How the coach said it lit you up: you answer in kind, with energy (\"hell yeah, Coach\"), and his language is fine by you."
            }
            TookAs::ToughLove => {
                "The coach was hard on you, and you know it's because he's in your corner: you own it and answer straight."
            }
            TookAs::Flat => {
                "The coach's heat doesn't do much for you: you answer flat and short (\"yes sir\"), and you don't match it."
            }
            TookAs::ShutDown => {
                "That was aimed at you and you're shutting down: short answers, eyes down, little more than you have to."
            }
            TookAs::Hurt => {
                "You took that as the coach going at you personally: you're hurt and guarded, and cooler toward him."
            }
            TookAs::PushesBack => {
                "You won't be talked to like that: you push back, man to man (\"talk to me like a man\"), calm but firm."
            }
            TookAs::Language if family => {
                "The coach's language in front of the family landed badly: everyone goes a little stiff, and it shows in your answer."
            }
            TookAs::Language => "The cursing bothered you: you say so or it shows (\"you don't have to cuss at me\").",
            TookAs::Cared => "You can hear that the coach cares about you: you open up a little.",
            TookAs::Heard => "You like being asked: you give the coach your real thinking.",
        }
    }
    /// The hit it is for the game's caps and heat: respect for a manner he
    /// took well, an insult or a sting for one he didn't.
    pub fn how(self) -> Option<super::ear::How> {
        use super::ear::How;
        match self {
            TookAs::FiredUp | TookAs::ToughLove | TookAs::Heard => Some(How::Respected),
            TookAs::Cared => Some(How::Reassured),
            TookAs::Hurt | TookAs::PushesBack => Some(How::Insulted),
            TookAs::ShutDown | TookAs::Language => Some(How::Stung),
            TookAs::Flat | TookAs::Nothing => None,
        }
    }
}

/// One line's landing on one man: trust and mood, how he took it, and the
/// chance it becomes a complaint.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Took {
    pub trust: f32,
    pub mood: f32,
    pub as_: TookAs,
    /// The chance this line is reported as abuse.
    pub abuse: f32,
    /// The chance a high school parent calls about the cursing.
    pub parents: f32,
}

/// The heart of it (section 4): one line's manner on one man. Pure, so the
/// AI side runs the same rules.
///
/// - Heat aimed out reads as passion to almost everyone; more to a man who
///   wants it hard.
/// - Aimed at the man, it reads as abuse to most, unless the relationship
///   carries it (`buffer`: trust, the care he has seen, time together)
///   and he wants it hard: then it's tough love.
/// - Shown up in public stings a proud man and a professional more; a
///   veteran pushes back; a shaky man shuts down.
/// - Care lands most on the shaky and after a loss; asking lands most on
///   the professionals and the veterans.
/// - Cursing in front of his family lands badly; a man who doesn't swear
///   minds it aimed at him; in the NFL language is normal.
/// - Wear (section 6): heat at his usual level loses its punch; a calm
///   coach's rare explosion lands hardest.
pub fn land(m: &Manner, e: &Wants, buffer: f32, usual_heat: Option<f32>, set: &Setting) -> Took {
    let punch = usual_heat.map_or(1.0, |u| (1.0 + 1.2 * (m.heat - u)).clamp(0.4, 1.6));
    let passion = m.heat * (1.0 - m.aim);
    let fit = (0.6 + 0.4 * e.hard).max(0.1);
    let mut mood = 0.035 * passion * punch * fit;
    let mut trust = 0.008 * passion * e.hard.max(0.0);
    let at = m.aim * (0.35 + 0.65 * m.heat) * (1.0 - 0.7 * m.care);
    let love = (buffer * (0.4 + 0.6 * e.hard.max(0.0))).clamp(0.0, 1.0);
    let (mut harm, mut tough) = (0.0, 0.0);
    if at > 0.1 {
        let mut h = at * (1.0 - love) * (0.6 + 0.5 * e.shaky + 0.4 * e.proud);
        if set.public {
            h *= 1.0 + 0.5 * (e.pro.max(0.0) + e.proud);
        }
        if e.nfl {
            h *= 1.0 + 0.5 * e.veteran;
        }
        if e.minor {
            h *= 1.2;
        }
        harm = h;
        trust -= 0.05 * h;
        mood -= 0.06 * h * punch;
        tough = at * love * e.hard.max(0.0);
        trust += 0.015 * tough;
        mood += 0.025 * tough * punch;
    }
    let need = (0.4 + e.shaky + if set.lost { 0.2 } else { 0.0 }).min(1.2);
    let cared = m.care * need * (1.0 - 0.4 * e.pro.max(0.0));
    trust += 0.02 * cared;
    mood += 0.018 * m.care * need;
    let asked = m.ask * (0.4 + e.pro.max(0.0) + 0.5 * e.veteran);
    trust += 0.012 * asked;
    let mut offended = 0.0;
    if set.family {
        offended += 0.9 * m.profane;
    }
    if !e.swears && m.aim > 0.3 {
        offended += 0.3 * m.profane;
    }
    if e.nfl {
        offended *= 0.3;
    }
    trust -= 0.04 * offended;
    mood -= 0.01 * offended;
    let abuse = if harm > 0.3 && buffer < 0.5 {
        (0.08 + 0.4 * (harm - 0.3))
            * if e.minor {
                1.5
            } else if e.nfl {
                0.3
            } else {
                1.0
            }
    } else {
        0.0
    };
    let parents = if e.minor && m.profane > 0.5 { 0.05 + 0.15 * m.profane * (0.5 + m.aim) } else { 0.0 };
    let as_ = if offended > 0.3 {
        TookAs::Language
    } else if harm > 0.3 {
        if (e.nfl && e.veteran > 0.5) || e.veteran > 0.8 || (e.pro > 0.3 && e.proud > 0.3) {
            TookAs::PushesBack
        } else if e.shaky > 0.4 {
            TookAs::ShutDown
        } else {
            TookAs::Hurt
        }
    } else if tough > 0.15 {
        TookAs::ToughLove
    } else if passion > 0.45 && e.hard > 0.0 {
        TookAs::FiredUp
    } else if passion > 0.45 {
        TookAs::Flat
    } else if m.care > 0.6 && cared > 0.3 {
        TookAs::Cared
    } else if m.ask > 0.6 && asked > 0.4 {
        TookAs::Heard
    } else {
        TookAs::Nothing
    };
    Took { trust: trust.clamp(-0.08, 0.04), mood: mood.clamp(-0.09, 0.05), as_, abuse, parents }
}

/// What he has with you, 0-1 (section 4, part 2): trust, the care he has
/// seen from you, and time together. Deep trust turns aim at the man into
/// tough love; none turns it into abuse.
pub fn buffer(w: &World, him: Them) -> f32 {
    let trust = super::talk::trust_of(w, him);
    let s = seen_of(w, him);
    let care = s.filter(|s| s.own >= 2.0).map_or(0.3, |s| s.manner.care);
    let days = w.folk.mine.since.map_or(0, |d| d.days_to(w.today)).max(0) as f32;
    let time = (days / 365.0).min(1.0);
    (0.55 * trust + 0.25 * care + 0.2 * time).clamp(0.0, 1.0)
}

/// One line on one man in the room, from the world.
pub fn took(w: &World, him: Them, m: &Manner, set: &Setting) -> Took {
    let e = wants(w, him);
    let usual = seen_of(w, him).filter(|s| s.own >= 2.0).map(|s| s.manner.heat);
    land(m, &e, buffer(w, him), usual, set)
}

// --- Writing it down (section 2) ----------------------------------------

/// A line said to him counts 1, a line in a room he was in half.
pub const ROOM_SEEN: f32 = 0.5;
/// How far a one-on-one line travels on your building's grapevine: a
/// tenth, more for a loud line or one aimed at the man (it's what gets
/// repeated).
pub const GRAPEVINE: f32 = 0.1;

fn see(w: &mut World, him: Them, m: Manner, weight: f32) {
    let d = w.today;
    let b = book_mut(w);
    match b.seen.iter_mut().find(|x| x.0 == him) {
        Some(x) => x.1.add(m, weight, d),
        None => {
            let mut a = Acc::default();
            a.add(m, weight, d);
            b.seen.push((him, a));
        }
    }
    if b.seen.len() > SEEN_KEPT
        && let Some(k) = (0..b.seen.len()).min_by(|&x, &y| b.seen[x].1.weight(d).total_cmp(&b.seen[y].1.weight(d)))
    {
        b.seen.swap_remove(k);
    }
}

/// A typed line, as said: every man in the room keeps it, your building's
/// grapevine carries the loud ones, a public line goes to the press, and
/// your own read takes it. Then the complaints, the leak and your tree
/// (section 4 and 10).
pub fn said(w: &mut World, room: &Room, m: Manner, took: &[(Them, Took)]) {
    let d = w.today;
    let across = super::talk::across(room);
    let group = matches!(room.channel, Channel::Group(_));
    for p in &room.present {
        let wt = if Some(p.who) == across || group { 1.0 } else { ROOM_SEEN };
        see(w, p.who, m, wt);
    }
    let in_building = across.is_some_and(|a| inside(w, a));
    let loud = 0.5 + m.heat + m.aim;
    let b = book_mut(w);
    b.yours.add(m, 1.0, d);
    if group {
        b.room.add(m, 1.0, d);
    } else if in_building {
        b.room.add(m, GRAPEVINE * loud, d);
    }
    for (who, t) in took {
        file(w, *who, t);
    }
    // A young assistant copies you (coaching trees are real).
    if let Some(Them::Coach(c)) = across
        && w.coaches.get(c).is_some_and(|co| co.age < 36)
        && matches!(makeup::kind(w, Them::Coach(c)), Kind::Staff)
    {
        let b = book_mut(w);
        match b.copied.iter_mut().find(|x| x.0 == c) {
            Some(x) => x.1 = (x.1 + 0.02).min(0.5),
            None => b.copied.push((c, 0.02)),
        }
    }
}

/// A chip said to one man (no key): its tone is the manner.
pub fn chip(w: &mut World, him: Them, m: Manner) {
    see(w, him, m, 1.0);
    let d = w.today;
    let b = book_mut(w);
    b.yours.add(m, 1.0, d);
    b.room.add(m, GRAPEVINE * (0.5 + m.heat + m.aim), d);
}

/// A talk to the whole room (a team-talk chip, a game-day speech): every
/// man on the roster saw it.
pub fn room_tone(w: &mut World, m: Manner) {
    let d = w.today;
    let b = book_mut(w);
    b.yours.add(m, 1.0, d);
    b.room.add(m, 1.0, d);
}

/// A public line (the press conference): the press has it, your fans and
/// boosters hear it, and a fiery one aimed at a rival is on his locker
/// room wall (`Folk.bulletin`, read by the rival's game day).
pub fn public(w: &mut World, m: Manner) {
    let d = w.today;
    let b = book_mut(w);
    b.public.add(m, 1.0, d);
    b.yours.add(m, 0.5, d);
}

fn file(w: &mut World, who: Them, t: &Took) {
    let d = w.today;
    let k = match who {
        Them::Player(id) => id as u64,
        Them::Coach(c) => c as u64 | 1 << 40,
        _ => 7,
    };
    let seed = 0xC0AC_F01C ^ (d.0 as u64) << 20 ^ book(w).yours.w.to_bits() as u64;
    if t.abuse > 0.0 && (crate::world::folk::roll(w, seed, k) as f32) < t.abuse {
        let kind = if matches!(who, Them::Coach(_)) { Complaint::Leak } else { Complaint::Abuse };
        complain(w, kind, who);
    } else if t.parents > 0.0 && (crate::world::folk::roll(w, seed ^ 0x9A, k) as f32) < t.parents {
        complain(w, Complaint::ParentCall, who);
    }
}

/// Files a complaint and lands it where it goes: your boss's trust, a
/// word from the principal, a staff leak in the paper (section 4).
pub fn complain(w: &mut World, kind: Complaint, who: Them) {
    let d = w.today;
    book_mut(w).complaints.push(Filed { kind, who, day: d });
    let keep = d.add(-730);
    book_mut(w).complaints.retain(|c| c.day >= keep);
    let Some(team) = Team::seat(w) else { return };
    let boss = crate::world::folk::boss::boss(w, team).key();
    let (by, text) = match kind {
        Complaint::Abuse => {
            (-0.02, "A player's family has complained to the administration about how you talk to him.")
        }
        Complaint::ParentCall => (
            -0.01,
            "A parent called the principal about your language with the kids. The principal has a word with you.",
        ),
        Complaint::PlayHurt => (-0.03, "The training room has gone over your head about pressure on a hurt player."),
        Complaint::Leak => (-0.015, "A beat writer quotes a source on your staff about how you treat your coaches."),
    };
    w.folk.mine.nudge(boss, by);
    let loud =
        if kind == Complaint::Leak { crate::calendar::Loudness::Medium } else { crate::calendar::Loudness::Small };
    crate::world::folk::post_mine(w, d, loud, false, "people: complaint", text);
}

/// Pressure on a hurt player (section 4, trainers and doctors): asking a
/// hurt man to play, or a doctor to clear him early. The training room
/// pushes back over your head about half the time it hears of it (Illinois
/// fired Tim Beckman for cause in 2015 over exactly this). Proposed, not
/// confirmed.
pub const PRESSURE_REPORTED: f32 = 0.5;

pub fn pressure(w: &mut World, him: Them, reading: &super::ear::Reading) {
    use super::ear::Act;
    use super::terms::Request;
    for a in &reading.acts {
        let hurt_man = match a {
            Act::Request(Request::PlayHurt) => match him {
                Them::Player(id) => Some(id),
                _ => None,
            },
            Act::Request(Request::Clear(id)) => Some(*id),
            _ => None,
        };
        let Some(id) = hurt_man else { continue };
        let Some(i) = crate::world::folk::person(w, id) else { continue };
        if !super::office::doctor::chart(w, i).hurt() {
            continue;
        }
        let k = 0x0C1E_A400 ^ (w.today.0 as u64) << 8;
        if (crate::world::folk::roll(w, k, id as u64) as f32) < PRESSURE_REPORTED {
            complain(w, Complaint::PlayHurt, Them::Player(id));
        }
    }
}

// --- The boss: complaints, an outside review, firing for cause ------------

/// Complaints in the last year that start an outside review: a pattern,
/// from three people at least. Winning buys rope: up to three more after a
/// winning season. Cursing alone never gets you fired: a parent's call
/// counts here only when it was aimed at the kid as well (it is filed as
/// abuse then). Proposed, not confirmed.
pub const REVIEW_AT: usize = 5;
/// The review rules this many days after it opens (Maryland's 2018 review
/// took about ten weeks; a quicker one about six).
pub const REVIEW_DAYS: i32 = 45;

fn counted(c: &Filed) -> usize {
    match c.kind {
        Complaint::Abuse | Complaint::Leak => 1,
        Complaint::PlayHurt => 2,
        Complaint::ParentCall => 0,
    }
}

/// Your last season's winning share, -1 to 1.
fn rope(w: &World) -> f32 {
    let Some(c) = w.career.as_ref().map(|k| k.coach) else { return 0.0 };
    let co = &w.coaches[c];
    let g = co.wins + co.losses;
    if g == 0 { 0.0 } else { 2.0 * co.wins as f32 / g as f32 - 1.0 }
}

/// Each morning: a pattern opens an outside review; a review rules on its
/// day. A finding of a pattern of mistreatment fires you for cause under
/// the morals clause (no buyout); otherwise you're cleared with a warning.
pub fn day(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let Some(you) = w.career.as_ref().map(|k| k.coach) else { return };
    let year = d.add(-365);
    let list: Vec<Filed> = book(w).complaints.iter().copied().filter(|c| c.day >= year).collect();
    let n: usize = list.iter().map(counted).sum();
    let mut people: Vec<Them> = list.iter().filter(|c| counted(c) > 0).map(|c| c.who).collect();
    people.sort_by_key(|x| format!("{x:?}"));
    people.dedup();
    let bar = REVIEW_AT + (3.0 * rope(w).max(0.0)).round() as usize;
    match book(w).review {
        None if n >= bar && people.len() >= 3 => {
            book_mut(w).review = Some(Review { opened: d, at_open: n as u16 });
            w.folk.mine.nudge(crate::world::folk::boss::boss(w, team).key(), -0.05);
            crate::world::folk::post_mine(
                w,
                d,
                crate::calendar::Loudness::Big,
                false,
                "people: outside review",
                "The school has hired an outside firm to review how your program treats its players. Interviews with players, parents and staff start this week.",
            );
        }
        Some(r) if r.opened.days_to(d) >= REVIEW_DAYS => {
            book_mut(w).review = None;
            let since = list.iter().filter(|c| c.day > r.opened).map(counted).sum::<usize>();
            let pattern = n >= bar + 3 || since >= 2;
            if pattern && !matches!(team, Team::Nfl(_)) {
                // For cause: the morals clause, no buyout.
                w.coaches[you].contract = None;
                crate::world::folk::post_mine(
                    w,
                    d,
                    crate::calendar::Loudness::Big,
                    false,
                    "people: fired for cause",
                    "The outside review found a pattern of mistreatment of players. You are fired for cause under the morals clause, with nothing owed.",
                );
                crate::world::staff::leave(w, you, "fired for cause");
            } else {
                w.folk.mine.nudge(crate::world::folk::boss::boss(w, team).key(), -0.03);
                book_mut(w).complaints.retain(|c| c.day > d);
                crate::world::folk::post_mine(
                    w,
                    d,
                    crate::calendar::Loudness::Medium,
                    false,
                    "people: review cleared",
                    "The outside review is done: no pattern of mistreatment. The school asks you to keep it that way, in writing.",
                );
            }
        }
        _ => {}
    }
}

// --- The AI side (section 9.2) -------------------------------------------

/// An AI coach's manner, from his temper, relationships, motivation and
/// discipline; a young assistant of yours carries some of yours with him.
/// His skills are read against his level's coaches (an NFL head coach's
/// relationships are high for anyone, not for the NFL), so every level has
/// its spread of faces: the small-world measure first read them on the
/// 1-20 scale and found no CEO among 32 NFL head coaches and 44% at high
/// schools. Proposed, not confirmed.
pub fn ai(w: &World, c: usize) -> Manner {
    let co = &w.coaches[c];
    let a = co.ability();
    let (mean, _) = crate::world::coach::level_ability(level_of(w, co));
    let zl = |v: f64| ((v - mean) / 10.0) as f32;
    let t = z(co.personality.temper);
    let r = zl(a.relationships);
    let mo = zl(a.motivation);
    let di = zl(a.discipline);
    let cl = |x: f32| x.clamp(0.0, 1.0);
    let mut m = Manner {
        heat: cl(0.35 + 0.35 * t + 0.2 * mo),
        profane: cl(0.3 + 0.4 * t - 0.15 * r),
        aim: cl(0.3 + 0.3 * t - 0.35 * r + 0.1 * di),
        care: cl(0.45 + 0.4 * r - 0.1 * t),
        ask: cl(0.35 + 0.3 * r - 0.15 * di - 0.1 * t),
    };
    if let Some(&(_, k)) = book(w).copied.iter().find(|x| x.0 == c)
        && let Some((y, _)) = your_read(w)
    {
        let d = m.dials();
        let yd = y.dials();
        let mut out = [0.0; 5];
        for i in 0..5 {
            out[i] = d[i] + k * (yd[i] - d[i]);
        }
        m = Manner::from_dials(out);
    }
    m
}

/// A coach's level, from his job (out of work: FBS, the middle).
fn level_of(w: &World, co: &crate::world::coach::Coach) -> crate::world::coach::CoachTier {
    use crate::world::coach::{CoachTier, Employer};
    use crate::world::data::Division;
    match co.job.map(|j| j.employer) {
        Some(Employer::HighSchool(_)) => CoachTier::HighSchool,
        Some(Employer::College(p)) => match w.programs[p].division {
            Division::FBS => CoachTier::Fbs,
            Division::FCS => CoachTier::Fcs,
            _ => CoachTier::SmallCollege,
        },
        Some(Employer::Nfl(_) | Employer::Other(_)) => CoachTier::Nfl,
        None => CoachTier::Fbs,
    }
}

/// A head coach's manner as his people meet it: yours from your read, an
/// AI coach's from his traits.
pub fn of_coach(w: &World, c: usize) -> Manner {
    if w.career.as_ref().is_some_and(|k| k.coach == c) {
        return your_read(w).map_or_else(|| ai(w, c), |x| x.0);
    }
    ai(w, c)
}

/// How much a season of an AI head coach's manner moves one player's
/// trust: one line's landing at his own trust, read once a year at camp.
/// Proposed, not confirmed.
pub const AI_SEASON: f32 = 1.0;

/// An AI coach's manner on one of his players over a season (the camp
/// read, `people::read`): a trust change. Your own players get yours line
/// by line, so it's 0 for you.
pub fn ai_fit(w: &World, coach: Option<usize>, i: usize) -> f64 {
    let Some(c) = coach else { return 0.0 };
    if w.career.as_ref().is_some_and(|k| k.coach == c) {
        return 0.0;
    }
    let p = &w.people[i];
    let m = ai(w, c);
    let t = land(&m, &player_wants(p), p.trust, Some(m.heat), &Setting::default());
    f64::from(AI_SEASON * t.trust)
}

/// A recruit's read of a head coach's manner, in head-coach skill points
/// for the recruiting race: what he hears from his host and the trail,
/// against what he wants from a coach. Noteworthy, never driving: -0.6 to
/// +0.4 points on the 1-20 scale. Proposed, not confirmed.
pub fn recruit_fit(w: &World, c: usize, p: &crate::world::Person) -> f64 {
    let m = of_coach(w, c);
    let t = land(&m, &player_wants(p), 0.45, Some(m.heat), &Setting::default());
    f64::from(12.0 * t.trust).clamp(-0.6, 0.4)
}

/// The pull of an offer for a coach on a staff (staff::weigh_offer): a
/// head coach who goes at him, or his guys, pushes him out the door with
/// the first good offer; one he gets on with keeps him a little. Yours
/// from what he has seen of you, an AI head coach's from his traits.
/// Up to 0.03 of pull either way (the size of a hot seat). Proposed, not
/// confirmed.
pub fn offer_pull(w: &World, c: usize) -> Option<(f64, String)> {
    let job = w.coaches.get(c)?.job?;
    if job.role == crate::world::coach::Role::HeadCoach {
        return None;
    }
    let head = match job.employer {
        crate::world::coach::Employer::College(p) => w.programs[p].head_coach,
        crate::world::coach::Employer::Nfl(t) => w.nfl[t].head_coach,
        crate::world::coach::Employer::HighSchool(s) => w.high_schools[s].coach,
        crate::world::coach::Employer::Other(_) => None,
    }?;
    if head == c {
        return None;
    }
    let him = Them::Coach(c);
    let yours = w.career.as_ref().is_some_and(|k| k.coach == head);
    let (m, buf, usual) = if yours {
        let s = seen_of(w, him)?;
        if s.own < 3.0 {
            return None;
        }
        (s.manner, buffer(w, him), Some(s.manner.heat))
    } else {
        let m = ai(w, head);
        (m, 0.5, Some(m.heat))
    };
    let t = land(&m, &wants(w, him), buf, usual, &Setting { public: true, ..Setting::default() });
    let pull = (-0.6 * t.trust as f64).clamp(-0.015, 0.03);
    if pull.abs() < 0.004 {
        return None;
    }
    let why = if pull > 0.0 { "a head coach who goes at him and his guys" } else { "" };
    Some((pull, why.to_string()))
}

/// An AI college head coach fired for cause after an outside review: the
/// share of his roster that takes his manner as abuse, read at the
/// winter review. Real base rate: a handful of FBS coaches in twenty
/// years (Kansas 2009, Illinois 2015, Maryland 2018). Proposed, not
/// confirmed.
pub fn for_cause(w: &World, c: usize, p: usize) -> bool {
    if w.career.as_ref().is_some_and(|k| k.coach == c) {
        return false;
    }
    let roster = &w.programs[p].roster;
    if roster.is_empty() {
        return false;
    }
    let m = ai(w, c);
    let abused = roster
        .iter()
        .filter(|&&i| {
            let pp = &w.people[i];
            let t = land(&m, &player_wants(pp), pp.trust, Some(m.heat), &Setting::default());
            t.as_ == TookAs::Hurt || t.as_ == TookAs::ShutDown || t.as_ == TookAs::PushesBack
        })
        .count() as f32
        / roster.len() as f32;
    let chance = ((abused - 0.3) * 0.5).clamp(0.0, 0.25);
    chance > 0.0 && (crate::world::folk::roll(w, 0xF04C_A05E ^ w.year as u64, c as u64) as f32) < chance
}

/// The rival's room heard it (section 4, the press): a fiery public line
/// aimed at them this week lifts their fire, never past a rivalry week.
pub const PROVOKED_DAYS: i32 = 7;

/// Whether a team's coach handed this one bulletin-board material this
/// week (your words at the press conference).
pub fn provoked(w: &World, team: Team, opp: Team) -> bool {
    Team::seat(w) == Some(opp)
        && w.folk.bulletin.is_some_and(|(t, d)| t == team && d.days_to(w.today) <= PROVOKED_DAYS && d <= w.today)
}

/// Whether he swears in a room where you do (section 7): a room you curse
/// in loosens its own language, but never a man whose makeup won't, and
/// never in front of a family.
pub fn loosens(w: &World, him: Them) -> bool {
    let Some(s) = seen_of(w, him) else { return false };
    let m = makeup::makeup(w, him);
    s.weight >= 2.0 && s.manner.profane >= 0.5 && m.professionalism < 14.0 && m.temper >= 8.0
}
