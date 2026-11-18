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

/// The setting for the voice's card. The scene's words are the coach's
/// ("Meeting with your boss", "your office"), and the man across read them
/// as his own: a coordinator who came by your office said "come on in and
/// sit down", and your athletic director read that he was meeting his boss
/// (the talks audit, 2026-11-16). On his card they're the coach's, and a
/// home visit is in his own home.
pub(crate) fn voice_setting(w: &World, s: &Scene, room: &Room) -> String {
    // He came to see the coach there: "Yeah, come on in, coach" to "got a
    // minute?" (the talks audit, 2026-11-16).
    let out = setting(w, s, room)
        .replace("your office", "the coach's office, where you came to see him")
        .replace("his home", "your home");
    his_side(&out)
}

/// The coach's things in the scene's words, as the coach's: on his card
/// "your" is his own. Proposed, not confirmed.
pub const COACHS: [(&str, &str); 5] = [
    ("your office", "the coach's office"),
    ("your boss", "the coach's boss"),
    ("your staff", "the coach's staff"),
    ("your case", "the coach's case"),
    ("your hire", "the coach's hire"),
];

/// Words told to the coach, from the side of the man across: a Scene line
/// in the talk so far ("Lowell Biggerstaff comes by your office") read as
/// his own office, and he said "come on in" though he was the one who came
/// (the talks audit, 2026-11-16).
pub(crate) fn his_side(words: &str) -> String {
    COACHS.iter().fold(words.to_string(), |out, (from, to)| {
        let cap = |x: &str| x[..1].to_uppercase() + &x[1..];
        out.replace(from, to).replace(&cap(from), &cap(to))
    })
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
    c.year = talk::year::words(w, him);
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
    // In front of the team or a room he answers for them: his own wants stay
    // off the card (the talks audit, 2026-11-16: the team's spokesman asked
    // for a real role, his own want on the card beside the line that said
    // never).
    let group = matches!(room.channel, Channel::Group(_));
    let wants = if group { Vec::new() } else { parts.wanting.wants(w, him) };
    // The same for his history with you: in front of the room he answers
    // for the team, so the card carries the team's history with you, not
    // his own promises and talks.
    let known_you = if group { talk::between::room(w).join(". ") } else { known_you };
    let mut stakes = talk::stakes::stakes(w, s, him, &wants);
    if group {
        stakes.push(
            "You're answering in front of the room, for the group: what the room feels and will do, never your own role, snaps, money or contract"
                .into(),
        );
    }
    let mut room_words: Vec<String> =
        room.present.iter().skip(1).map(|p| format!("{} ({})", person(w, p.who), seat_words(p.seat))).collect();
    room_words.extend(speaker);
    // His asks from earlier lines of this talk, never asked again; the one
    // his last line raised he may press once (the talks audit, 2026-11-16:
    // a coordinator asked where he stood on four lines of five).
    let open = store::open(w, s.id);
    let on_table = open.and_then(|o| o.asking());
    // Taken up by the coach (the ear heard no dodge), the ask on the table
    // is answered, and asked again it's the same question (the talks
    // audit, 2026-11-16: "You're already my first pick at DT", then "do I
    // have a real role on this D-line or not?").
    let pressing = on_table.is_some() && t.dodged;
    let asked_before: Vec<String> = open
        .map(|o| {
            o.raised
                .iter()
                .filter(|k| !pressing || Some(**k) != on_table)
                .map(|k| talk::ask_words_of(w, him, *k))
                .collect()
        })
        .unwrap_or_default();
    let mut heard = say.heard;
    if on_table.is_some() && !pressing {
        heard.push(
            "He answered the question you asked him on your last line: take his answer, doubt it, or ask about what he just said, and don't ask your question again"
                .into(),
        );
    }
    // The talk's last exchange (`Ends::Time`): he answers and says goodbye.
    // On his card only, never the scene's own words.
    let mut released = say.released;
    // Someone else answers this line (his parents do the talking, his
    // agent takes the money): first on the list, where the voice can't
    // miss it. In the room list alone, the kid still answered for his
    // parents (the talks audit, 2026-11-16).
    if let Some((x, seat)) = t.speaker.filter(|(x, _)| *x != him) {
        let who = person(w, x);
        released.insert(
            0,
            format!(
                "This line is answered by {who} ({}), not by you: write only what {who} says, in that voice, with no name in front (the screen shows who speaks)",
                seat_words(seat)
            ),
        );
    }
    // Said for the room, the room's answer is "we": "They'll do it" on the
    // scene, "we'll do it" from the man who answers for them (the talks
    // audit, 2026-11-16: "Yes sir, we'll do it. He'll do it.").
    if group {
        for r in released.iter_mut() {
            if let Some(rest) = r.strip_prefix("They") {
                *r = format!("We{rest}");
            }
        }
    }
    if matches!(t.ends, Some(Ends::Time)) {
        released.push(
            match room.channel {
                Channel::Phone => "The call is at its end: you answer, then say goodbye, with no new question",
                Channel::Text => "That's your last text for now: you answer and sign off",
                _ => "Time's up: you answer, then wrap it up and say goodbye, with no new question",
            }
            .into(),
        );
    }
    VoiceCard {
        setting: voice_setting(w, s, room),
        year: talk::year::words(w, him),
        coach: coach(w),
        them: person(w, him),
        background,
        personality,
        mood,
        football,
        known_you,
        // In front of the room he answers for the group, never his own
        // situation: no "Where you stand" of the man who speaks.
        standing: if group { Vec::new() } else { talk::standing::words(w, him) },
        stakes,
        room: room_words,
        swears,
        care,
        // Each with who he is: the coordinator named two of your coaches as
        // his players ("get Jace and Travis more reps", the talks audit,
        // 2026-11-16). The room as a whole has no name.
        may_name: keys
            .people
            .iter()
            .filter(|(_, x)| channels::name(w, *x) != "He")
            .map(|(tag, x)| Tagged { tag: tag.clone(), text: person(w, *x) })
            .chain(Team::seat(w).map(|t| Tagged { tag: "team".into(), text: t.name(w) }))
            .collect(),
        released,
        heard,
        stance: say.stance.map(|x| x.key().replace('_', " ")).unwrap_or_default(),
        tell: say.tell,
        holding_back: say.holding_back,
        lie: say.lie.unwrap_or_default(),
        asks,
        asked_by: t.speaker.filter(|(x, _)| *x != him).map(|(x, _)| person(w, x)).unwrap_or_default(),
        asked_before,
        pressing,
        heat,
        manner: manner_words(w, room, t, him),
        coach_read: coach_read(w, him),
        notebook: store::notebook(w, him),
        so_far: so_far(s),
        line: quoted(line),
    }
}

/// How he took your manner on this line (coach voice, section 7), for him
/// to answer in kind.
fn manner_words(w: &World, room: &Room, t: &Turn, him: Them) -> String {
    let family = afm_engine::world::folk::persona::manner::setting(w, room).family;
    t.manner.iter().find(|x| x.0 == him).map(|x| x.1.as_.voice(family).to_string()).unwrap_or_default()
}

/// How you come across to him (coach voice, section 2): by what he has
/// seen of you, or only by your reputation while that's thin.
fn coach_read(w: &World, him: Them) -> String {
    use afm_engine::world::folk::persona::manner;
    let Some(s) = manner::seen_of(w, him) else { return String::new() };
    if s.weight < 1.0 {
        return String::new();
    }
    let words = manner::words_for(&s.manner, None);
    if s.own < 2.0 {
        format!("you know him mostly by reputation, as {words}")
    } else {
        format!("from what you've seen and heard of him, {words}")
    }
}

impl EarCard {
    /// The ear's text: the card, the talk so far and your line.
    pub fn text(&self, today: &str) -> String {
        let mut out = String::from("<card>\n");
        out.push_str(&format!("Setting: {}.\nToday: {today}.\n", self.setting));
        if !self.year.is_empty() {
            out.push_str(&format!("Time of year: {}.\n", self.year));
        }
        out.push_str(&format!("Coach: {}.\n", self.coach));
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
        // One stop: "How you come across: He's choosing his words.." (the
        // talks audit, 2026-11-16).
        let mut put = |k: &str, v: &str| {
            let v = v.trim().trim_end_matches('.');
            if !v.is_empty() {
                out.push_str(&format!("{k}: {v}.\n"));
            }
        };
        put("Setting", &self.setting);
        put("Time of year (his team's calendar: talk in it)", &self.year);
        put("Coach", &self.coach);
        put("You are", &self.them);
        put("Background", &self.background);
        put("Who you are", &self.personality.join("; "));
        put("Your mood", &self.mood);
        put("Your football", &self.football);
        put(
            "You and the coach (true, and you know all of it already: this is all the history you have with him, so never invent more)",
            &self.known_you,
        );
        put("Where you stand (true, and you know all of it already)", &self.standing.join(". "));
        put("What this talk is to you (play toward it)", &self.stakes.join(". "));
        put("Also in the room", &self.room.join("; "));
        put("Language", if self.swears { "you may swear, as you would here" } else { "no swearing here" });
        if self.care {
            put("Care", "something hard is going on in your life: if it comes up, speak of it with care");
        }
        let names: Vec<String> = self.may_name.iter().map(|t| t.text.clone()).collect();
        put("The only people and teams you may name", &names.join("; "));
        put("How the coach comes across to you", &self.coach_read);
        put("Heat", &self.heat);
        put("How you took the coach's manner on this line (answer in kind)", &self.manner);
        put("Your stance on the hard topic", &self.stance);
        put("How you come across", &self.tell);
        if self.holding_back {
            put("Holding back", "you are holding something back: let it show, never say what");
        }
        put("Say this, not the truth", &self.lie);
        if !self.heard.is_empty() {
            out.push_str("What the coach just did (react to it first, in your own words):\n");
            for h in &self.heard {
                out.push_str(&format!("- {h}\n"));
            }
        }
        // "He won't do that" came back as "but he won't do that, not for
        // anybody", said by the doctor himself (the talks audit,
        // 2026-11-16).
        let he = if self.asked_by.is_empty() {
            "a line that starts with \"He\" or \"His\" is about you: say it as yourself (\"He won't do that\" is \"I won't do that\")".to_string()
        } else {
            format!("a line that starts with \"He\" or \"His\" is about {}", self.them)
        };
        out.push_str(&format!("What the game decided you say this turn (say all of it, add nothing; {he}):\n"));
        if self.released.is_empty() {
            out.push_str("- nothing beyond a natural answer to the coach's words\n");
        }
        for r in &self.released {
            out.push_str(&format!("- {r}\n"));
        }
        if !self.asks.is_empty() && self.asked_by.is_empty() {
            out.push_str(
                "What you ask the coach yourself, after you answer the coach's line, as a natural follow-on to it (here \"he\" is you and \"you\" is the coach: \"he asks about his own next step\" is your own next step):\n",
            );
        } else if !self.asks.is_empty() {
            // His parents answer this line: they ask for him, in their own
            // words (the talks audit, 2026-11-16: "he's going to want to
            // hear what your plan is... then ask him").
            let him = self.them.split(',').next().unwrap_or(&self.them);
            let by = &self.asked_by;
            out.push_str(&format!(
                "Questions {by} put to the coach for {him}, after answering the coach's line, as a natural follow-on to it, in {by}'s own words (here \"he\" and \"his\" are {him}):\n"
            ));
        }
        for a in &self.asks {
            // Asked for him, the ask is its subject alone ("about his
            // money"): "he asks about his money" came back word for word.
            let a = if self.asked_by.is_empty() { a.as_str() } else { a.strip_prefix("he asks ").unwrap_or(a) };
            out.push_str(&format!("- {a}\n"));
        }
        if !self.asked_before.is_empty() {
            // "Let it show in how you sound" came back as "but I'd still like
            // a straight answer on where I stand" on every line (the talks
            // audit, 2026-11-16).
            out.push_str("Already asked earlier in this talk (here \"he\" is you): never ask these again or bring them back up in other words (\"I'd still like an answer on that\" is asking again). If the coach answered, react to that and move on; if he didn't, you've let it go for this talk: answer what he says now:\n");
            for a in &self.asked_before {
                out.push_str(&format!("- {a}\n"));
            }
        }
        if !self.notebook.is_empty() {
            out.push_str("Details you've mentioned before (stay the same person):\n");
            for n in &self.notebook {
                out.push_str(&format!("- {n}\n"));
            }
        }
        // The scene's words are the coach's ("Lowell Biggerstaff comes by
        // your office"): the man across took them as his own and said
        // "come on in" (the talks audit, 2026-11-16).
        out.push_str(
            "In the talk so far, Scene lines are told to the coach: there \"you\" and \"your\" mean the coach.\n",
        );
        // Last before the talk: with the list of what he's already asked
        // above, a coordinator still said "I'd still like a straight answer
        // on where I stand" on three lines running (the talks audit,
        // 2026-11-16).
        if !self.asked_before.is_empty() {
            out.push_str("This turn: what you already asked earlier in this talk stays let go. Don't ask it, hint at it or say you're still waiting on it; answer what the coach just said.\n");
        }
        out.push_str("</card>\n<talk_so_far>\n");
        for l in &self.so_far {
            let text = if l.who == "Scene" { his_side(&l.text) } else { l.text.clone() };
            out.push_str(&format!("{}: {}\n", l.who, quoted(&text)));
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
    // An ask put to someone else in the room, by his name (ear audit item
    // 4): "a request to Gil Foster".
    for (doer, _, _) in &t.asked_of {
        parts.push(format!("a request to {}", folk::plain(&channels::name(w, *doer))));
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
    // How you said it is not one of his things: his answer shows how he
    // took it, and it counts in how the line went over ("respected on his
    // pride" stood under "First time meeting, nice to meet you", Cain
    // 2026-11-18).
    let manner = |h: &afm_engine::world::folk::persona::ear::Sized| {
        h.hit.item == afm_engine::world::folk::persona::ear::Item::Manner
    };
    // Each hit by what it touched, once you know that; one on something
    // he hasn't shown you (what he wants) by how it hit alone, so a line
    // to someone whose hits are all on his wants still says how it went.
    let mut bare: Vec<String> = Vec::new();
    for h in t.hits.iter().filter(|h| h.hit.on == him && (h.trust + h.mood).abs() > 0.005 && !dodge(h) && !manner(h)) {
        let how = format!("{:?}", h.hit.how).to_lowercase();
        let what = if items::known(w, him, &h.hit.item) { items::short(w, him, &h.hit.item) } else { String::new() };
        if !what.is_empty() {
            parts.push(format!("{how} on {what}"));
        } else if !bare.contains(&how) {
            bare.push(how);
        }
    }
    parts.extend(bare);
    // How it went over, as the game read it (`Turn.landing`: a recruit's
    // line on your case, anyone else's on his own bars); nothing when the
    // line touched nothing of his ("Heard as a request.").
    let landed = t.landing.words(channels::object(w, him));
    if !landed.is_empty() {
        parts.push(landed);
    }
    parts.join("; ")
}
