//! The ear's card and the voice's card for a talk (lane N4), filled from
//! the world in words and tags only (`card.rs`'s rule: no field for
//! potential, true ratings, hidden numbers or the depth chart).

use afm_engine::world::World;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::ear::{Confirm, Ends, Turn};
use afm_engine::world::folk::persona::room::{Channel, Group, Place, Room, Seat};
use afm_engine::world::folk::persona::talk::{self, Parts, channels, items, store, who};
use afm_engine::world::folk::persona::wants::Boil;
use afm_engine::world::folk::scene::{Scene, Who};
use afm_engine::world::folk::{self as folk, Team, claims, talk_points as tp, ties};

use super::{EarCard, EarKeys, VoiceCard};
use crate::live_talk::card::{self, Spoken, Tagged, quoted};

/// The most people of his position room and his friends the ear's card
/// lists beyond the room. Proposed, not confirmed.
pub const MAX_NAMED: usize = 10;

/// A person in words: name, role, class year.
pub fn person(w: &World, who: Them) -> String {
    match who {
        Them::Player(id) => card::person(w, &Who::Player(id)),
        Them::Coach(c) => card::person(w, &Who::Coach(c)),
        _ => channels::name(w, who),
    }
}

fn seat_words(s: Seat) -> &'static str {
    match s {
        Seat::Across => "the man across from you",
        Seat::Decider => "his decider",
        Seat::Family => "his family",
        Seat::Agent => "his agent",
        Seat::HsCoach => "his high school coach",
        Seat::YourStaff => "your staff, brought in",
        Seat::FormerPlayer => "a former player, brought in",
        Seat::Listener => "listening",
    }
}

/// The channel and place in words: the setting changes how he talks.
pub fn channel_words(c: Channel) -> String {
    match c {
        Channel::InPerson(p) => format!(
            "in person, {}",
            match p {
                Place::Office => "your office",
                Place::Home => "his home",
                Place::Facility => "the facility",
                Place::Field => "the practice field",
                Place::Sideline => "the sideline",
                Place::LockerRoom => "the locker room",
                Place::Out => "out, away from the building",
            }
        ),
        Channel::Phone => "a phone call".into(),
        Channel::Text => "a text message".into(),
        Channel::Video => "a video call".into(),
        Channel::Through(_) => "through someone else".into(),
        Channel::Letter => "a letter".into(),
        Channel::Group(g) => match g {
            Group::Team => "the whole team".into(),
            Group::Position(_) => "his position room".into(),
            Group::Staff => "your staff".into(),
            Group::Council => "the leadership council".into(),
            Group::Family(_) => "his family".into(),
        },
    }
}

fn setting(w: &World, s: &Scene, room: &Room) -> String {
    let mut out = format!("{}, {}", s.kind.name(), channel_words(room.channel));
    out.push_str(&format!(", {}", crate::views::date_text(s.date)));
    if let Some(t) = Team::seat(w) {
        out.push_str(&format!(", {}", format!("{:?}", t.level()).to_lowercase()));
    }
    out
}

fn coach(w: &World) -> String {
    let name = folk::scene::your_name(w);
    match Team::seat(w) {
        Some(t) => format!("{name}, head coach at {}", t.name(w)),
        None => name,
    }
}

/// The talk so far, from the scene: typed words where you typed them.
fn so_far(s: &Scene) -> Vec<Spoken> {
    let from = s.lines.len().saturating_sub(card::SO_FAR);
    s.lines[from..]
        .iter()
        .map(|l| Spoken {
            who: match l.by {
                0 => "Scene".to_string(),
                1 => "Coach".to_string(),
                _ => folk::plain(&l.name),
            },
            text: folk::plain(l.typed.as_deref().unwrap_or(&l.text)),
        })
        .collect()
}

/// The ear's card and its keys, for one line in a scene's talk.
pub fn ear(w: &World, s: &Scene, room: &Room, parts: &Parts, line: &str) -> (EarCard, EarKeys) {
    let mut keys = EarKeys::default();
    let mut c = EarCard {
        setting: setting(w, s, room),
        coach: coach(w),
        so_far: so_far(s),
        line: quoted(line),
        ..EarCard::default()
    };
    let Some(him) = talk::across(room) else { return (c, keys) };
    c.them = person(w, him);
    let add = |c: &mut EarCard, keys: &mut EarKeys, who: Them, why: &str| {
        if keys.people.iter().any(|(_, x)| *x == who) {
            return;
        }
        let tag = format!("p{}", keys.people.len() + 1);
        c.people.push(Tagged { tag: tag.clone(), text: format!("{} ({why})", person(w, who)) });
        keys.people.push((tag, who));
    };
    for p in &room.present {
        add(&mut c, &mut keys, p.who, seat_words(p.seat));
    }
    // Anyone else the line may name: his friends, then his position room,
    // then your staff.
    if let Them::Player(id) = him {
        let mut named = 0;
        for f in ties::friends(w, id) {
            if named == MAX_NAMED {
                break;
            }
            if folk::person(w, f).is_some() {
                add(&mut c, &mut keys, Them::Player(f), "his friend");
                named += 1;
            }
        }
        if let (Some(i), Some(team)) = (folk::person(w, id), Team::seat(w)) {
            let pos = w.people[i].player.position;
            for j in team.roster(w).into_iter().filter(|&j| j != i && w.people[j].player.position == pos) {
                if named == MAX_NAMED {
                    break;
                }
                add(&mut c, &mut keys, Them::Player(w.people[j].player.id), "in his position room");
                named += 1;
            }
        }
    }
    if let Some(team) = Team::seat(w) {
        for (k, co) in w.coaches.iter().enumerate() {
            if co.job.is_some_and(|j| Team::from_employer(j.employer) == team) {
                add(&mut c, &mut keys, Them::Coach(k), "your staff");
            }
        }
    }
    let wants = parts.wanting.wants(w, him);
    for (k, (item, words)) in items::items(w, him, &wants).into_iter().enumerate() {
        let tag = format!("c{}", k + 1);
        c.items.push(Tagged { tag: tag.clone(), text: words });
        keys.items.push((tag, item));
    }
    c.on_table = on_table(w, s);
    (c, keys)
}

/// His asks still on the table: the last line he raised.
fn on_table(w: &World, s: &Scene) -> Vec<String> {
    let Some(o) = store::open(w, s.id) else { return Vec::new() };
    o.raised.last().map(|k| talk::ask_words(*k)).into_iter().collect()
}

/// The voice's card from the game's turn.
pub fn voice(w: &World, s: &Scene, room: &Room, parts: &Parts, t: &Turn, line: &str, keys: &EarKeys) -> VoiceCard {
    let Some(him) = talk::across(room) else { return VoiceCard::default() };
    let who::Who { background, personality, mood, football, known_you, swears } = who::who(w, him, room);
    let (say, asks) = talk::say_of(t);
    let care = match him {
        Them::Player(id) => folk::person(w, id).is_some_and(|i| {
            let sheet = claims::player(w, i);
            [claims::Kind::Death, claims::Kind::SickParent, claims::Kind::Arrest, claims::Kind::Divorce]
                .iter()
                .any(|k| sheet.get(*k).is_some())
        }),
        _ => false,
    };
    // Heat against his own point (lane N1): a hothead is hot sooner.
    let heat = match (t.heat.0 / parts.feel.point(w, him).max(0.01), t.ends) {
        (_, Some(Ends::Him(Some(Boil::WalksOut)))) => "past his point: he walks out".to_string(),
        (h, _) if h >= 1.0 => "past his point".into(),
        (h, _) if h >= 0.6 => "getting hot".into(),
        (h, _) if h >= 0.3 => "a little heated".into(),
        _ => "calm".into(),
    };
    let speaker =
        t.speaker.map(|(x, seat)| format!("{} ({}) answers this one for him", person(w, x), seat_words(seat)));
    let mut room_words: Vec<String> =
        room.present.iter().skip(1).map(|p| format!("{} ({})", person(w, p.who), seat_words(p.seat))).collect();
    room_words.extend(speaker);
    VoiceCard {
        setting: setting(w, s, room),
        coach: coach(w),
        them: person(w, him),
        background,
        personality,
        mood,
        football,
        known_you,
        room: room_words,
        swears,
        care,
        may_name: keys
            .people
            .iter()
            .map(|(tag, x)| Tagged { tag: tag.clone(), text: channels::name(w, *x) })
            .chain(Team::seat(w).map(|t| Tagged { tag: "team".into(), text: t.name(w) }))
            .collect(),
        released: say.released,
        stance: say.stance.map(|x| x.key().replace('_', " ")).unwrap_or_default(),
        tell: say.tell,
        holding_back: say.holding_back,
        lie: say.lie.unwrap_or_default(),
        asks,
        heat,
        notebook: store::notebook(w, him),
        so_far: so_far(s),
        line: quoted(line),
    }
}

impl EarCard {
    /// The ear's text: the card, the talk so far and your line.
    pub fn text(&self, today: &str) -> String {
        let mut out = String::from("<card>\n");
        out.push_str(&format!("Setting: {}.\nToday: {today}.\nCoach: {}.\n", self.setting, self.coach));
        if !self.them.is_empty() {
            out.push_str(&format!("Across from him: {}.\n", self.them));
        }
        out.push_str("People (tag: who):\n");
        for p in &self.people {
            out.push_str(&format!("- {}: {}\n", p.tag, p.text));
        }
        out.push_str("His items (tag: what):\n");
        for i in &self.items {
            out.push_str(&format!("- {}: {}\n", i.tag, i.text));
        }
        if !self.measures.is_empty() {
            out.push_str("Measures:\n");
            for m in &self.measures {
                out.push_str(&format!("- {}: {}\n", m.tag, m.text));
            }
        }
        if !self.on_table.is_empty() {
            out.push_str(&format!("On the table: {}.\n", self.on_table.join("; ")));
        }
        out.push_str("</card>\n<talk_so_far>\n");
        for l in &self.so_far {
            out.push_str(&format!("{}: {}\n", l.who, quoted(&l.text)));
        }
        out.push_str(&format!("</talk_so_far>\n<coach_line>{}</coach_line>", self.line));
        out
    }
}

impl VoiceCard {
    /// The voice's text: who he is, what the game released, the talk so
    /// far and your line.
    pub fn text(&self) -> String {
        let mut out = String::from("<card>\n");
        let mut put = |k: &str, v: &str| {
            if !v.is_empty() {
                out.push_str(&format!("{k}: {v}.\n"));
            }
        };
        put("Setting", &self.setting);
        put("Coach", &self.coach);
        put("You are", &self.them);
        put("Background", &self.background);
        put("Who you are", &self.personality.join("; "));
        put("Your mood", &self.mood);
        put("Your football", &self.football);
        put("You and the coach", &self.known_you);
        put("Also in the room", &self.room.join("; "));
        put("Language", if self.swears { "you may swear, as you would here" } else { "no swearing here" });
        if self.care {
            put("Care", "something hard is going on in your life: if it comes up, speak of it with care");
        }
        let names: Vec<String> = self.may_name.iter().map(|t| t.text.clone()).collect();
        put("The only people and teams you may name", &names.join("; "));
        put("Heat", &self.heat);
        put("Your stance on the hard topic", &self.stance);
        put("How you come across", &self.tell);
        if self.holding_back {
            put("Holding back", "you are holding something back: let it show, never say what");
        }
        put("Say this, not the truth", &self.lie);
        out.push_str("What the game decided you say this turn (say all of it, add nothing):\n");
        if self.released.is_empty() {
            out.push_str("- nothing beyond a natural answer to the coach's words\n");
        }
        for r in &self.released {
            out.push_str(&format!("- {r}\n"));
        }
        if !self.asks.is_empty() {
            out.push_str("What you ask the coach yourself:\n");
            for a in &self.asks {
                out.push_str(&format!("- {a}\n"));
            }
        }
        if !self.notebook.is_empty() {
            out.push_str("Details you've mentioned before (stay the same person):\n");
            for n in &self.notebook {
                out.push_str(&format!("- {n}\n"));
            }
        }
        out.push_str("</card>\n<talk_so_far>\n");
        for l in &self.so_far {
            out.push_str(&format!("{}: {}\n", l.who, quoted(&l.text)));
        }
        out.push_str(&format!("</talk_so_far>\n<coach_line>{}</coach_line>", self.line));
        out
    }
}

/// "Heard as" (question 2): what your line did, and which of his things it
/// touched only when you already know them; otherwise only whether it
/// landed.
pub fn heard(w: &World, room: &Room, t: &Turn) -> String {
    let Some(him) = talk::across(room) else { return String::new() };
    let mut parts: Vec<String> = Vec::new();
    if !t.told.is_empty() {
        parts.push(if t.told.len() == 1 { "a question".into() } else { format!("{} questions", t.told.len()) });
    }
    if !t.promised.is_empty() {
        parts.push("a promise".into());
    }
    if !t.claims.is_empty() {
        parts.push("a claim".into());
    }
    if !t.weighed.is_empty() {
        parts.push("an offer".into());
    }
    if !t.asked.is_empty() {
        parts.push("a request".into());
    }
    if t.confirm.iter().any(|c| matches!(c, Confirm::Decision(_))) {
        parts.push("a decision".into());
    }
    for h in t.hits.iter().filter(|h| h.hit.on == him && (h.trust + h.mood).abs() > 0.005) {
        if items::known(w, him, &h.hit.item) {
            let what = items::short(w, him, &h.hit.item);
            if !what.is_empty() {
                parts.push(format!("{} on {what}", format!("{:?}", h.hit.how).to_lowercase()));
            }
        }
    }
    let net: f32 = t.hits.iter().filter(|h| h.hit.on == him).map(|h| h.trust + h.mood).sum();
    let landed = if net > tp::LANDED {
        "it landed"
    } else if net < -0.03 {
        "it didn't land"
    } else {
        "hard to tell how it landed"
    };
    parts.push(landed.into());
    parts.join("; ")
}
