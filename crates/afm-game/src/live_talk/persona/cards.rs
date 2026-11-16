//! The ear's card and the voice's card for a talk (lane N4), filled from
//! the world in words and tags only (`card.rs`'s rule: no field for
//! potential, true ratings, hidden numbers or the depth chart).

use afm_engine::world::World;
use afm_engine::world::folk::persona::ear::{Confirm, Ends, Turn};
use afm_engine::world::folk::persona::negotiate;
use afm_engine::world::folk::persona::room::{Channel, Group, Place, Room, Seat};
use afm_engine::world::folk::persona::talk::{self, Parts, channels, items, store, who};
use afm_engine::world::folk::persona::wants::Boil;
use afm_engine::world::folk::persona::{Them, office};
use afm_engine::world::folk::scene::{Scene, Who};
use afm_engine::world::folk::{self as folk, Team, claims, ties};

use super::{EarCard, EarKeys, VoiceCard};
use crate::live_talk::card::{self, Spoken, Tagged, quoted};

/// The most people of his position room and his friends the ear's card
/// lists beyond the room. Proposed, not confirmed.
pub const MAX_NAMED: usize = 10;

/// The most men a staff coach's card lists of the ones he coaches (a
/// coordinator's side runs past forty in college). Proposed, not confirmed.
pub const MAX_COACHED: usize = 60;

/// A person in words: name, role, class year.
pub fn person(w: &World, who: Them) -> String {
    match who {
        Them::Player(id) => card::person(w, &Who::Player(id)),
        Them::Coach(c) => card::person(w, &Who::Coach(c)),
        Them::Cast(r, _) => card::person(w, &Who::Cast(r.token())),
        Them::Drawn(afm_engine::world::folk::persona::wants::Kind::Compliance, _) => {
            format!("{}, director of compliance", channels::name(w, who))
        }
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
            Group::Parents => "the players' parents".into(),
            Group::Coaches => "the feeder schools' coaches".into(),
        },
    }
}

pub(crate) fn setting(w: &World, s: &Scene, room: &Room) -> String {
    let mut out = format!("{}, {}", s.kind.name(), channel_words(room.channel));
    out.push_str(&format!(", {}", crate::views::date_text(s.date)));
    if let Some(t) = Team::seat(w) {
        out.push_str(&format!(", {}", format!("{:?}", t.level()).to_lowercase()));
    }
    out
}

pub(crate) fn coach(w: &World) -> String {
    let name = folk::scene::your_name(w);
    match Team::seat(w) {
        Some(t) => format!("{name}, {} at {}", talk::stakes::your_role(w), t.name(w)),
        None => name,
    }
}

/// The talk so far, from the scene: typed words where you typed them.
pub(crate) fn so_far(s: &Scene) -> Vec<Spoken> {
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
    // A negotiation's men: the client, the men in a trade, your roster at
    // a prospect's spot (lane N5b).
    let (extra_people, extra_items) = negotiate::card_extras(w, s);
    for (who, why) in extra_people {
        add(&mut c, &mut keys, who, &why);
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
    // Someone in your building (lane N5c): the coach a reference is about,
    // the man a sit-down is about, the hurt men the doctor would talk of.
    for (who, why) in office::sit::named(w, s) {
        add(&mut c, &mut keys, who, why);
    }
    // A coach on your staff: the men he coaches, so a line can name one
    // ("work with Hill on his releases").
    if let Them::Coach(co) = him {
        for pid in folk::talks::coached_by(w, co).into_iter().take(MAX_COACHED) {
            add(&mut c, &mut keys, Them::Player(pid), "a player he coaches");
        }
    }
    // One of the older meetings: the players it is about and the men on the
    // sheet in front of you.
    for (who, why) in afm_engine::world::folk::persona::meetings::named(w, s) {
        add(&mut c, &mut keys, who, why);
    }
    // Your staff, never yourself: your own name on the list read to the
    // voice as a third man in the room ("does Coach Buu know?").
    let me = w.career.as_ref().map(|x| x.coach);
    if let Some(team) = Team::seat(w) {
        for (k, co) in w.coaches.iter().enumerate() {
            if Some(k) != me && co.job.is_some_and(|j| Team::from_employer(j.employer) == team) {
                add(&mut c, &mut keys, Them::Coach(k), "your staff");
            }
        }
    }
    // A holdover can name the head coach you replaced.
    if let Them::Coach(co) = him
        && talk::stakes::holdover(w, co).is_some()
        && let Some(o) = w.folk.takeover.as_ref().and_then(|t| t.old_coach)
        && w.coaches.get(o).is_some()
    {
        add(&mut c, &mut keys, Them::Coach(o), "the head coach you replaced");
    }
    // A college staff's call: the player he called about can be named.
    if talk::hs_call::is_call(s.kind) {
        add(&mut c, &mut keys, Them::Player(s.a as u32), "your player, the one he called about");
    }
    let wants = parts.wanting.wants(w, him);
    // A recruit's card and a caller's are their facet's (lane N5a).
    let list = talk::recruit::items(w, s, &wants)
        .or_else(|| talk::hs_call::items(w, s, &wants))
        .unwrap_or_else(|| items::items(w, him, &wants));
    for (k, (item, words)) in list.into_iter().chain(extra_items).enumerate() {
        let tag = format!("c{}", k + 1);
        c.items.push(Tagged { tag: tag.clone(), text: words });
        keys.items.push((tag, item));
    }
    c.on_table = on_table(w, s, him);
    (c, keys)
}

/// His ask still on the table: the one he raised on his last line, in
/// his own words for what it means to him. Once the coach has answered a
/// line it is off the table, so an answer to an older ask is never read
/// as a dodge.
fn on_table(w: &World, s: &Scene, him: Them) -> Vec<String> {
    let Some(o) = store::open(w, s.id) else { return Vec::new() };
    o.asking().map(|k| talk::ask_words_of(w, him, k)).into_iter().collect()
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
    // What the talk is to him: who you are to him, what rides on it, what
    // he wants from it (Cain 2026-11-15: "give the AI targets").
    let stakes = talk::stakes::stakes(w, s, him, &parts.wanting.wants(w, him));
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
        stakes,
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
        put("What this talk is to you (play toward it)", &self.stakes.join(". "));
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
            out.push_str(
                "What you ask the coach yourself (after you answer his line, as a natural follow-on to it):\n",
            );
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
    // A commitment: what he did with it (wrote it down, already had it,
    // asked for something he can measure).
    for (_, took) in &t.committed {
        parts.push(took.heard().into());
    }
    if t.dodged {
        parts.push("you didn't answer his question".into());
    }
    // The dodge's own hit is said above, not as a doubt on his word.
    let dodge = |h: &afm_engine::world::folk::persona::ear::Sized| {
        t.dodged
            && h.hit.item == afm_engine::world::folk::persona::ear::Item::HisWord
            && h.hit.how == afm_engine::world::folk::persona::ear::How::Doubted
    };
    // Each hit by what it touched, once you know that; one on something
    // he hasn't shown you (what he wants) by how it hit alone, so a line
    // to someone whose hits are all on his wants still says how it went.
    let mut bare: Vec<String> = Vec::new();
    for h in t.hits.iter().filter(|h| h.hit.on == him && (h.trust + h.mood).abs() > 0.005 && !dodge(h)) {
        let how = format!("{:?}", h.hit.how).to_lowercase();
        let what = if items::known(w, him, &h.hit.item) { items::short(w, him, &h.hit.item) } else { String::new() };
        if !what.is_empty() {
            parts.push(format!("{how} on {what}"));
        } else if !bare.contains(&how) {
            bare.push(how);
        }
    }
    parts.extend(bare);
    // What moved with him: the hits, and a commitment he wrote down.
    let net: f32 = t.hits.iter().filter(|h| h.hit.on == him).map(|h| h.trust + h.mood).sum::<f32>()
        + t.committed.iter().map(|(_, k)| k.trust()).sum::<f32>();
    // His own bars: a person in your building moves less in a line than a
    // player does (`hits::landed_bars`).
    let (up, down) = talk::hits::landed_bars(him);
    let landed = if net > up {
        "it landed"
    } else if net < down {
        "it didn't land"
    } else {
        "hard to tell how it landed"
    };
    parts.push(landed.into());
    parts.join("; ")
}
