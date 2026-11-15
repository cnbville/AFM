//! Your scouting people as talkers (question 10: "sending a scout with a
//! question", "asking pro scouts about other clubs' players", "an NFL
//! security director"). Each answers from what your department really
//! holds: his own looks in the file, read with his own eye. Asked something
//! he hasn't seen, he says so, and you can send him to find out: the trip
//! or the call he makes is a real look in your file under his name, and he
//! writes back with the answer on the day it lands.
//!
//! - A college scout: a school call (his coaches, the trainer, people
//!   around the program) for character, work, what drives him, his trouble
//!   or his medical history as they know it; a cross-check for football.
//! - Your director of security: the background check (police records,
//!   court records, the people around him), five days.
//! - A pro scout: another club's man on film, three days.
//!
//! Sizes are Claude's picks, proposed, not confirmed.

use super::super::Them;
use super::super::knows::Topic;
use crate::calendar::Date;
use crate::world::World;
use crate::world::scouting::readers::{self, Staff};
use crate::world::scouting::{Kind, file, lines};

/// A question you sent someone to find out.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Question {
    pub by: Them,
    pub id: u32,
    pub topic: Topic,
    pub asked: Date,
    pub due: Date,
    /// The look he makes (a school call, the background check, film).
    pub kind: Kind,
    pub done: bool,
}

/// Days until each kind of answer lands. Proposed, not confirmed.
pub const SCHOOL_DAYS: i32 = 7;
pub const CROSS_DAYS: i32 = 10;
pub const BACKGROUND_DAYS: i32 = 5;
pub const FILM_DAYS: i32 = 3;
pub const CALL_DAYS: i32 = 3;

/// A reader code for one of your people: a college scout's place, your
/// director of security, a pro scout.
pub fn code_of(w: &World, him: Them) -> Option<u32> {
    let t = super::club(w)?;
    if let Some((sec, k)) = super::drawn_of(him) {
        return Some(if sec { readers::SECURITY } else { readers::PRO_SCOUT + k as u32 });
    }
    if let Them::ProScout(key) = him {
        let (club, k) = super::super::makeup::pro_scout(key);
        if club == t && w.clubs.get(t).is_some_and(|c| k < c.scouts.len()) {
            return Some(k as u32);
        }
    }
    None
}

/// Whether he is one of your own scouting people.
pub fn yours(w: &World, him: Them) -> bool {
    code_of(w, him).is_some()
}

/// The look a question sends him on, and its days.
fn look_for(w: &World, him: Them, topic: Topic) -> Option<(Kind, i32)> {
    let code = code_of(w, him)?;
    Some(if code == readers::SECURITY {
        (Kind::Background, BACKGROUND_DAYS)
    } else if code >= readers::PRO_SCOUT {
        (Kind::ProFilm, FILM_DAYS)
    } else {
        match topic {
            Topic::Football | Topic::ScoutFile | Topic::Opponent => (Kind::CrossCheck, CROSS_DAYS),
            Topic::Injury => (Kind::Call, CALL_DAYS),
            _ => (Kind::School, SCHOOL_DAYS),
        }
    })
}

/// You send him to find out (`Request::FindOut`): kept until its day.
/// Returns what he says he'll do.
pub fn send(w: &mut World, him: Them, id: u32, topic: Topic) -> Option<String> {
    let (kind, days) = look_for(w, him, topic)?;
    crate::world::folk::person(w, id)?;
    let today = w.today;
    let b = &mut w.folk.live.persona.draft;
    if b.questions.iter().any(|q| q.by == him && q.id == id && q.topic == topic && !q.done) {
        return Some("He's already on it.".into());
    }
    b.questions.push(Question { by: him, id, topic, asked: today, due: today.add(days), kind, done: false });
    Some(match kind {
        Kind::Background => format!("He'll run the full background check: about {days} days."),
        Kind::ProFilm => format!("He'll get him on film: about {days} days."),
        Kind::CrossCheck => format!("He'll go see him again: about {days} days."),
        Kind::Call => format!("He'll make some calls about his health: about {days} days."),
        _ => format!("He'll get on the phone with the school and the people around him: about {days} days."),
    })
}

/// The day: each question due today becomes his look in your file, and
/// his answer comes as a letter.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.folk.live.persona.draft.questions.iter().all(|q| q.done || q.due > d) {
        return;
    }
    let due: Vec<(usize, Question)> = w
        .folk
        .live
        .persona
        .draft
        .questions
        .iter()
        .enumerate()
        .filter(|(_, q)| !q.done && q.due <= d)
        .map(|(k, q)| (k, *q))
        .collect();
    for (k, q) in due {
        w.folk.live.persona.draft.questions[k].done = true;
        let Some(code) = code_of(w, q.by) else { continue };
        if crate::world::scouting::place(w, q.id).is_none() {
            continue;
        }
        crate::world::scouting::dept::add(w, q.id, code, q.kind, q.asked.add(1));
        let words = answer(w, code, q.id, q.topic);
        write(w, q.by, q.id, q.topic, words);
    }
}

/// What he can say about a man now, from his own looks in your file, by
/// topic. Empty when he has no look of his own.
pub fn answer(w: &World, code: u32, id: u32, topic: Topic) -> Vec<String> {
    let Some(t) = super::club(w) else { return Vec::new() };
    let staff = Staff::Club(t);
    if code == readers::SECURITY {
        return security_words(w, staff, id);
    }
    match topic {
        Topic::Character | Topic::Effort | Topic::Mood | Topic::Wants | Topic::Life | Topic::Case => {
            let mut v = character_words(w, staff, code, id, topic);
            if v.is_empty() {
                v = his_read(w, staff, code, id);
            }
            v
        }
        Topic::Injury => medical_words(w, staff, code, id),
        _ => his_read(w, staff, code, id),
    }
}

/// His read of a man's football: his range against the room, what he
/// likes and what worries him, and how sure he is (the letters' words).
pub fn his_read(w: &World, staff: Staff, code: u32, id: u32) -> Vec<String> {
    let Some(i) = crate::world::scouting::place(w, id) else { return Vec::new() };
    let (rds, looks, codes) = readers::evidence(w, staff, id);
    let Some(r) = codes.iter().position(|&c| c == code) else { return Vec::new() };
    if !looks.iter().any(|l| l.reader == r) {
        return Vec::new();
    }
    let f = file::read(w, i, &rds, &looks, &readers::norms(w, i), readers::medical_rating(w, staff), Some(w.today));
    let Some((centre, best, worst)) = own_centre(w, i, &f, r) else { return Vec::new() };
    let p = &w.people[i];
    let (lo, hi) = crate::world::scouting::yours::range((centre, (f.now.1 * 1.4).max(2.0)));
    let room = if centre >= f.now.0 + 1.5 {
        "higher than the room"
    } else if centre <= f.now.0 - 1.5 {
        "lower than the room"
    } else {
        "about where the room has him"
    };
    vec![
        format!("I have {} at {lo}-{hi} as a {}, {room}.", p.name, p.player.position),
        format!(
            "What I like: {}. What worries me: {}.",
            lines::attr_phrase(best, true),
            lines::attr_phrase(worst, false)
        ),
        format!("I'm {} on him.", file::sure_words(f.now.1)),
    ]
}

/// One reader's own centre of a man's overall and his best and worst seen
/// attributes, weighed by the spot (as the letters read it).
pub fn own_centre(
    w: &World,
    i: usize,
    f: &file::FileRead,
    r: usize,
) -> Option<(f64, crate::attributes::Attr, crate::attributes::Attr)> {
    let pos = w.people[i].player.position;
    let mine = f.per_reader.get(r)?;
    let (mut num, mut den) = (0.0, 0.0);
    let mut seen: Vec<(f64, crate::attributes::Attr)> = Vec::new();
    for &a in crate::attributes::Attr::ALL.iter() {
        let wt = crate::positions::weight(pos, a);
        den += wt;
        if let Some((c, _)) = mine.get(a.index()).copied().flatten() {
            num += wt * (c - f.centres.get(a));
            seen.push((wt * (c - f.now.0), a));
        }
    }
    if seen.is_empty() {
        return None;
    }
    seen.sort_by(|a, b| b.0.total_cmp(&a.0));
    let centre = f.now.0 + if den > 0.0 { num / den } else { 0.0 };
    Some((centre, seen[0].1, seen[seen.len() - 1].1))
}

/// The file's character read on a topic's traits, in the file's words,
/// once he has a look that shows character.
fn character_words(w: &World, staff: Staff, code: u32, id: u32, topic: Topic) -> Vec<String> {
    let (_, looks, codes) = readers::evidence(w, staff, id);
    let Some(r) = codes.iter().position(|&c| c == code) else { return Vec::new() };
    if !looks.iter().any(|l| l.reader == r && matches!(l.kind, Kind::School | Kind::Call | Kind::Background)) {
        return Vec::new();
    }
    let Some((_, _, f)) = readers::read(w, staff, id) else { return Vec::new() };
    let traits: &[usize] = match topic {
        Topic::Effort => &[0, 1],
        Topic::Wants | Topic::Life => &[4, 6, 3],
        Topic::Case => &[7, 10, 12],
        Topic::Mood => &[12, 8],
        _ => &[0, 10, 7, 9],
    };
    let said: Vec<&str> = traits
        .iter()
        .filter_map(|&k| f.traits.get(k).filter(|t| t.seen).map(|t| lines::trait_word(k, t.centre)))
        .collect();
    if said.is_empty() {
        return Vec::new();
    }
    vec![format!("The people around him say: {}.", said.join("; "))]
}

/// What his medical history looks like from the calls (the doctors
/// examine; a scout only hears).
fn medical_words(w: &World, staff: Staff, code: u32, id: u32) -> Vec<String> {
    let (_, looks, codes) = readers::evidence(w, staff, id);
    let Some(r) = codes.iter().position(|&c| c == code) else { return Vec::new() };
    if !looks.iter().any(|l| l.reader == r) {
        return Vec::new();
    }
    let Some((_, _, f)) = readers::read(w, staff, id) else { return Vec::new() };
    vec![
        match f.medical.grade {
            0 => "Nothing on his health that worries anyone there.",
            1 => "A minor thing in his history; the doctors should look.",
            2 => "A real medical concern: our doctors need him in the building.",
            _ => "People there don't think he holds up.",
        }
        .into(),
    ]
}

/// Your director of security on a man: what the background check found
/// (his trouble on record, a suspension or dismissal at school) and his
/// risk call from the file's character read. Before a check, he says he
/// hasn't run him.
pub fn security_words(w: &World, staff: Staff, id: u32) -> Vec<String> {
    let Some(i) = crate::world::scouting::place(w, id) else { return Vec::new() };
    let checked = w
        .scouting
        .dept
        .looks
        .get(&id)
        .is_some_and(|v| v.iter().any(|l| l.kind == Kind::Background && l.date <= w.today));
    if !checked {
        return vec!["We haven't run him yet. Give me five days and I'll have the full background.".into()];
    }
    let p = &w.people[i];
    let mut v = Vec::new();
    match crate::world::folk::claims::rooms::incident(w, i) {
        Some(x) => v.push(format!("On record: {} at {}.", x.kind, x.age)),
        None => v.push("Nothing on record with the police or the courts.".into()),
    }
    let school = p.player.history.iter().rev().find_map(|h| {
        if h.events.has(crate::player::DevelopmentEvent::Dismissed) {
            Some(format!("dismissed from the team at {}", h.age))
        } else if h.events.has(crate::player::DevelopmentEvent::Suspended) {
            Some(format!("suspended at school at {}", h.age))
        } else {
            None
        }
    });
    if let Some(s) = school {
        v.push(format!("At school: {s}."));
    }
    if let Some((_, _, f)) = readers::read(w, staff, id) {
        let prof = f.traits.get(10).filter(|t| t.seen).map_or(10.5, |t| t.centre);
        let temper = f.traits.get(7).filter(|t| t.seen).map_or(10.5, |t| t.centre);
        let risk = 0.6 * (10.5 - prof) / 3.5 + 0.5 * (temper - 10.5) / 3.5 + if v.len() > 1 { 0.4 } else { 0.0 };
        v.push(
            if risk < 0.3 {
                "My call: a low risk."
            } else if risk < 1.0 {
                "My call: some risk. I'd want his position coach's word on him."
            } else {
                "My call: a high risk. I'd want a lot more before we take him."
            }
            .into(),
        );
    }
    v
}

/// His letter with the answer.
fn write(w: &mut World, by: Them, id: u32, topic: Topic, words: Vec<String>) {
    use crate::world::desk::Head;
    let (name, job) = who(w, by);
    let Some(i) = crate::world::folk::person(w, id) else { return };
    let link = crate::world::folk::player_link(w, i);
    let first = name.split_whitespace().next().unwrap_or("").to_string();
    let what = match topic {
        Topic::Case => "his record",
        Topic::Injury => "his health",
        Topic::Character | Topic::Effort | Topic::Mood | Topic::Wants | Topic::Life => "the kind of man he is",
        _ => "his football",
    };
    let mut body = vec![format!("You asked me to find out about {what}: {link}.")];
    if words.is_empty() {
        body.push("I came up short: nobody I reached could speak to it.".into());
    } else {
        body.extend(words);
    }
    body.push(format!("It's in the file. {first}"));
    let mut m = crate::world::storylines::credit::mail(
        w,
        &format!("n5eask{}:{id}:{}", super::super::makeup::key(by), w.today.0),
        Head::Email,
        name.clone(),
        crate::world::folk::takeover::cap_first(&job),
        format!("What you asked about {}", w.people[i].name),
    );
    m.body = body;
    m.sign = name;
    m.keys = 1 << 7;
    m.about = Some((0, id));
    m.ids = vec![id];
    m.minute = 16 * 60 + 30;
    crate::world::storylines::credit::send(w, m);
}

/// His name and job.
pub fn who(w: &World, him: Them) -> (String, String) {
    let t = super::club(w).unwrap_or(0);
    if let Some((sec, k)) = super::drawn_of(him) {
        if sec {
            return (security_name(w, t), "director of security".into());
        }
        let s = w.scouting.pro.scouts.get(k);
        return (
            s.map_or("Your pro scout".into(), |s| s.name.clone()),
            if k == 0 { "director of pro personnel".into() } else { "pro scout".into() },
        );
    }
    if let Them::ProScout(key) = him {
        let (club, k) = super::super::makeup::pro_scout(key);
        if let Some(s) = w.clubs.get(club).and_then(|c| c.scouts.get(k)) {
            let job = match s.role {
                crate::world::pro::scouts::Role::Director => "director of college scouting",
                crate::world::pro::scouts::Role::National => "national scout",
                _ => "area scout",
            };
            return (s.name.clone(), job.into());
        }
    }
    ("Your staff".into(), "staff".into())
}

/// Your director of security's name: a steady draw for the club.
pub fn security_name(w: &World, t: usize) -> String {
    let seed = w.options.seed ^ 0x5EC0 ^ (t as u32) << 8;
    let mut rng = crate::rng::Rng::new(seed);
    let born = w.year - rng.int(45, 62);
    let eth = crate::world::pro::scouts::staff_ethnicity(&mut rng, w.year);
    crate::world::names::person_name(eth, born, &mut rng)
}
