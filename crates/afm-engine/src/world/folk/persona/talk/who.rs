//! Who he is, in words for the voice (section 3.7): his age, where he's
//! from and how he grew up, his personality as behaviour in bands, his
//! mood, his football in words, how long he's known you. Words only, in
//! bands: no number, rating or hidden trait value leaves the engine. The
//! voice never sees race: his speech comes from his age, schooling,
//! personality and football, never from his background's ethnicity.

use super::super::Them;
use super::super::makeup::{self, Makeup};
use super::super::room::{Channel, Place, Room, Seat};
use super::super::wants::Kind;
use crate::attributes::Attr;
use crate::calendar::born;
use crate::player::Trait;
use crate::world::folk::{self as folk, facts};
use crate::world::{Stage, World};

/// The voice's facts about him.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Who {
    pub background: String,
    pub personality: Vec<String>,
    pub mood: String,
    pub football: String,
    pub known_you: String,
    pub swears: bool,
}

/// A band of a 1-20 trait, read as a z score.
const HIGH: f64 = 0.8;

/// The voice's facts about him: a player from his own stores, a coach
/// from his career and makeup, anyone else from his makeup and his trust
/// in you.
pub fn who(w: &World, him: Them, room: &Room) -> Who {
    let id = match him {
        Them::Player(id) => id,
        Them::Coach(c) if w.coaches.get(c).is_some() => return coach(w, c, him, room),
        _ => return other(w, him, room),
    };
    let Some(i) = folk::person(w, id) else { return Who::default() };
    let p = &w.people[i];
    let age = born::age_or(p.born, w.today, 18);
    let state = w.states.get(p.home_state).map_or("", |s| s.code.as_str());
    let school = w.high_schools.get(p.hs).map(|_| w.school_name(p.hs).to_string()).unwrap_or_default();
    let money = match p.bg.money {
        x if x < 0.25 => "grew up with very little",
        x if x < 0.6 => "grew up in a working family",
        _ => "grew up comfortable",
    };
    let mut background = format!("{age}, from {state}");
    if !school.is_empty() {
        background.push_str(&format!(", went to {school}"));
    }
    background.push_str(&format!("; {money}; {}", facts::class_words(w, i)));
    let me = &p.player.personality;
    let tr = &me.traits;
    let mut personality = Vec::new();
    let mut band = |t: Trait, hi: &str, lo: &str| {
        let z = tr.z(t);
        if z > HIGH && !hi.is_empty() {
            personality.push(hi.to_string());
        } else if z < -HIGH && !lo.is_empty() {
            personality.push(lo.to_string());
        }
    };
    band(Trait::Ego, "proud, needs to hear he matters", "humble, doesn't need the credit");
    band(Trait::Temper, "hot-headed", "even-keeled");
    band(Trait::Professionalism, "a pro: all business", "loose about the details");
    band(Trait::Leadership, "a leader in the room", "");
    band(Trait::Loyalty, "loyal to his people", "out for himself");
    band(Trait::Ambition, "driven, wants more", "");
    band(Trait::Greed, "minds the money", "");
    band(Trait::Adaptability, "takes change in stride", "slow to warm to change");
    if me.competitiveness >= 15.0 {
        personality.push("hates to lose".into());
    }
    if me.work_ethic >= 15.0 {
        personality.push("a grinder".into());
    }
    let mood = super::today::mood_words(p.mood).to_string();
    let iq = p.player.attributes.get(Attr::FootballIq);
    let years = match p.stage {
        Stage::Pro => p.pro.as_ref().map_or(0, |c| c.seasons.len()) + p.college_seasons as usize,
        _ => p.college_seasons as usize,
    };
    let football = format!(
        "{}; {} in college or the pros",
        match iq {
            x if x >= 70.0 => "knows the game deeply, talks it like a coach",
            x if x >= 45.0 => "knows his job and the scheme",
            _ => "plays on feel more than knowledge",
        },
        match years {
            0 => "no years yet".to_string(),
            1 => "one year".to_string(),
            n => format!("{n} years"),
        }
    );
    let since = w.folk.mine.since.map_or(0, |d| d.days_to(w.today));
    let trust = match p.trust {
        x if x > 0.7 => "trusts you",
        x if x > 0.45 => "is still making up his mind about you",
        _ => "doesn't trust you",
    };
    let known_you = match since {
        x if x < 60 => format!("you're new to him, and he {trust}"),
        x if x < 400 => format!("he's known you under a year, and he {trust}"),
        x => format!("he's known you {} years, and he {trust}", x / 365),
    };
    // He swears when that man would, in that setting: never in front of his
    // family or on a letter.
    let family = room.present.iter().any(|x| matches!(x.seat, Seat::Decider | Seat::Family));
    let setting = !matches!(room.channel, Channel::Letter | Channel::InPerson(Place::Home)) && !family;
    let swears = setting && (tr.z(Trait::Temper) > 0.3 || (age >= 21 && tr.z(Trait::Professionalism) < 0.0));
    Who { background, personality, mood, football, known_you, swears }
}

/// A band of a 1-20 makeup trait (lane N1's scale). Proposed, not
/// confirmed.
const MK_HIGH: f64 = 13.5;
const MK_LOW: f64 = 6.5;

/// His personality as behaviour from his makeup (anyone who isn't a
/// player).
fn makeup_words(m: &Makeup) -> Vec<String> {
    let mut out = Vec::new();
    let mut band = |v: f64, hi: &str, lo: &str| {
        if v >= MK_HIGH && !hi.is_empty() {
            out.push(hi.to_string());
        } else if v <= MK_LOW && !lo.is_empty() {
            out.push(lo.to_string());
        }
    };
    band(m.ego, "proud, needs to hear he matters", "humble, doesn't need the credit");
    band(m.temper, "hot-headed", "even-keeled");
    band(m.professionalism, "a pro: all business", "loose about the details");
    band(m.leadership, "a leader in the room", "");
    band(m.loyalty, "loyal to his people", "out for himself");
    band(m.ambition, "driven, wants more", "content where he is");
    band(m.greed, "minds the money", "");
    band(m.warmth, "warm and open", "guarded");
    band(m.honesty, "straight with people", "says what serves him");
    out
}

/// How long he's known you and his trust in you, in words.
fn known(w: &World, him: Them) -> String {
    let since = w.folk.mine.since.map_or(0, |d| d.days_to(w.today));
    let trust = match makeup::trust(w, him) {
        x if x > 0.7 => "trusts you",
        x if x > 0.45 => "is still making up his mind about you",
        _ => "doesn't trust you",
    };
    // Only someone in your building has known you since you came; anyone
    // else knows you by what he's heard.
    let building =
        matches!(makeup::kind(w, him), Kind::Staff | Kind::Boss | Kind::Player | Kind::Family | Kind::Booster);
    match since {
        _ if !building => format!("he knows you mostly by what he's heard, and he {trust}"),
        x if x < 60 => format!("you're new to him, and he {trust}"),
        x if x < 400 => format!("he's known you under a year, and he {trust}"),
        x => format!("he's known you {} years, and he {trust}", x / 365),
    }
}

/// He swears when that man would, in that setting: never in front of a
/// family, at a home or in a letter.
fn swears_here(room: &Room, m: &Makeup) -> bool {
    let family = room.present.iter().any(|x| matches!(x.seat, Seat::Decider | Seat::Family));
    let setting = !matches!(room.channel, Channel::Letter | Channel::InPerson(Place::Home)) && !family;
    setting && (m.temper >= 13.0 || m.professionalism < 9.0)
}

/// A coach: his age, his years in coaching, where he came up, his makeup,
/// his football, and you.
fn coach(w: &World, c: usize, him: Them, room: &Room) -> Who {
    let co = &w.coaches[c];
    let m = makeup::makeup(w, him);
    let age = born::age_or(co.born, w.today, co.age);
    let first = co.history.iter().map(|s| s.from).chain(co.job.map(|j| j.first)).min().unwrap_or(w.year);
    let years = (w.year - first).max(0);
    let mut background = format!(
        "{age}, coaching {}; came up as a {}",
        match years {
            0 => "less than a year".to_string(),
            1 => "one year".to_string(),
            n => format!("{n} years"),
        },
        co.came_up.coach_title()
    );
    if co.ex_pro {
        background.push_str("; played in the pros");
    }
    let football = match m.iq {
        x if x >= 14.0 => "a sharp football mind: talks scheme in detail, like a coach",
        x if x >= 9.0 => "knows his side of the ball and talks it like a coach",
        _ => "a grinder more than a schemer, but talks football like a coach",
    }
    .to_string();
    Who {
        background,
        personality: makeup_words(&m),
        mood: String::new(),
        football,
        known_you: known(w, him),
        swears: swears_here(room, &m),
    }
}

/// Anyone else: his makeup and you (his card's name and role say the
/// rest).
fn other(w: &World, him: Them, room: &Room) -> Who {
    let m = makeup::makeup(w, him);
    Who { personality: makeup_words(&m), known_you: known(w, him), swears: swears_here(room, &m), ..Who::default() }
}
