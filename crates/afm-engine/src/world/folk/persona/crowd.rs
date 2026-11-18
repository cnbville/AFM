//! A group talk as a room (thread "Make group talks feel like a room",
//! Cain 2026-11-17, nine items agreed): the team meeting, the council,
//! a position room, the captains, the parents and the feeder coaches.
//!
//! Before this the conversation ran on one man (`talk::across`, the first
//! captain): his wants, patience and temper were the room's, he answered
//! every line, a line to everyone landed on every man the same way, and a
//! promise to the team was his alone. Here:
//!
//! 1. Each man takes a room line his own way: the ear's card names the
//!    room's groups (`Bloc`: the starters, the backups, the seniors, the
//!    newcomers, the offense, the defense, each position room), a line can
//!    land on each group its own way, and each man in a group hears it as
//!    one of them (`Item::Group`), sized by his own makeup; a line about
//!    jobs weighs more on an ambitious man (`ROLE_AMBITION`).
//! 2. Leaders pull the room (`adjust`): a man moves part of the way toward
//!    the leader he's closest to (`PULL`).
//! 3. A promise to the room is a promise to every man it covers (`covers`).
//! 4. Calling a man out in front of the room hits him harder (`PUBLIC`) and
//!    his close friends take a share (`SPILL`).
//! 5. Who speaks depends on the room (`voices`): in a full team meeting a
//!    leader answers for the room or the unit the line went to, or the man
//!    with the most at stake speaks up; in a small room each man answers for
//!    himself; at most one more man answers after (`second`).
//! 6. The room has its own patience and heat (`start`, `patience`, `heat`):
//!    it's lost when most men are; one hothead can walk out alone (`walks`).
//! 7. Asking the room: men speak by their honesty in front of others; a man
//!    who held back may ask to see you the next day (`held`).
//! 8. The room's read, by groups (`read`).
//!
//! Item 9, position room meetings, is `start_position_room`. Every size
//! here is proposed, not confirmed. Staff rooms are their own (the office,
//! each coach for himself): `plays` leaves them out.

use super::Them;
use super::ear::{Act, Hit, How, Item, Reading, Sized, Turn};
use super::knows::Topic;
use super::room::{Channel, Group, Room, Seat};
use super::talk::feel::Feeling;
use super::talk::store::Open;
use super::talk::{self, hits, z};
use super::wants::{Boil, Heat, Kind as PKind, Patience};
use crate::player::Trait;
use crate::world::coach::Group as Pos;
use crate::world::folk::scene::{Kind, Scene, Who};
use crate::world::folk::{self as folk, Team, seen, talks, ties};
use crate::world::{Stage, World};

/// A man moves this share of the way toward the leader he's closest to, at
/// the closest tie and the most sway. Proposed, not confirmed.
pub const PULL: f32 = 0.3;
/// A man called out in front of the room takes it this many times as hard
/// as in private. Proposed, not confirmed.
pub const PUBLIC: f32 = 1.5;
/// His close friends in the room take this share of it. Proposed, not
/// confirmed.
pub const SPILL: f32 = 0.3;
/// A line about jobs weighs this much more on a man for each point of
/// ambition over the middle. Proposed, not confirmed.
pub const ROLE_AMBITION: f32 = 0.3;
/// The room's patience under which it starts to drift. Proposed, not
/// confirmed.
pub const DRIFT: f32 = 0.35;
/// What a man must have at stake, sized, to speak up on his own. Proposed,
/// not confirmed.
pub const SPEAK: f32 = 0.03;
/// What a second man must have at stake (times his boldness and the room's
/// size) to answer after the first. Proposed, not confirmed.
pub const SECOND: f32 = 0.035;
/// In a full team meeting a man speaks up this much less than in a small
/// room. Proposed, not confirmed.
pub const BIG_ROOM: f32 = 0.35;
/// The most leaders a room has. Proposed, not confirmed.
pub const MAX_LEADERS: usize = 8;
/// A man counts as a leader of the room past this much leadership (a
/// standard deviation over the middle). Proposed, not confirmed.
pub const LEADER_Z: f32 = 1.0;
/// How bothered a man must be (his mood under zero, or the line's sting)
/// before he has something to say when you ask the room. Proposed, not
/// confirmed.
pub const WORRY: f32 = 0.12;
/// How hard a line must hit a man himself before he can walk out of a room
/// (trust and mood together). Proposed, not confirmed.
pub const WALK_HURT: f32 = 0.05;

/// Whether a room plays as a crowd: any group but your staff (a staff room
/// is the office's, where each coach speaks for himself).
pub fn plays(room: &Room) -> bool {
    matches!(room.channel, Channel::Group(g) if g != Group::Staff)
}

/// A full team meeting: few men speak.
pub fn big(room: &Room) -> bool {
    matches!(room.channel, Channel::Group(Group::Team)) || listeners(room).count() > 24
}

/// The men listening (not your own staff at the front of the room).
pub fn listeners(room: &Room) -> impl Iterator<Item = Them> + '_ {
    room.present.iter().filter(|p| p.seat != Seat::YourStaff).map(|p| p.who)
}

// ---------- 1. the room's groups ----------

/// A group in the room the ear can hear a line go to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bloc {
    Starters,
    Backups,
    Seniors,
    Newcomers,
    Offense,
    Defense,
    Room(Pos),
}

/// The key a group carries on the ear's card (`Them::Drawn`).
const BLOC: u64 = 0x424C_4F43_0000;

impl Bloc {
    fn code(self) -> u8 {
        match self {
            Bloc::Starters => 1,
            Bloc::Backups => 2,
            Bloc::Seniors => 3,
            Bloc::Newcomers => 4,
            Bloc::Offense => 5,
            Bloc::Defense => 6,
            Bloc::Room(g) => 16 + g as u8,
        }
    }

    pub fn from_code(c: u8) -> Option<Bloc> {
        Some(match c {
            1 => Bloc::Starters,
            2 => Bloc::Backups,
            3 => Bloc::Seniors,
            4 => Bloc::Newcomers,
            5 => Bloc::Offense,
            6 => Bloc::Defense,
            x if x >= 16 => Bloc::Room(*Pos::ALL.get(usize::from(x - 16))?),
            _ => return None,
        })
    }

    /// The group as a person on the ear's card.
    pub fn them(self) -> Them {
        Them::Drawn(PKind::Player, BLOC | u64::from(self.code()))
    }

    /// The group a card's person is, if he's one.
    pub fn of(t: Them) -> Option<Bloc> {
        match t {
            Them::Drawn(PKind::Player, k) if k & !0xFF == BLOC => Bloc::from_code((k & 0xFF) as u8),
            _ => None,
        }
    }

    /// The item a man in this group hears a line to it on.
    pub fn item(self) -> Item {
        Item::Group(self.code())
    }

    /// A group about jobs: who plays.
    pub fn about_jobs(self) -> bool {
        matches!(self, Bloc::Starters | Bloc::Backups)
    }

    /// "the starters", "the defense", "the receivers". At the NFL the
    /// seniors are the veterans and the newcomers the rookies.
    pub fn words(self, level: crate::calendar::Level) -> String {
        let nfl = level == crate::calendar::Level::Nfl;
        match self {
            Bloc::Starters => "the starters".into(),
            Bloc::Backups => "the backups".into(),
            Bloc::Seniors if nfl => "the veterans".into(),
            Bloc::Seniors => "the seniors".into(),
            Bloc::Newcomers if nfl => "the rookies".into(),
            Bloc::Newcomers => "the freshmen".into(),
            Bloc::Offense => "the offense".into(),
            Bloc::Defense => "the defense".into(),
            Bloc::Room(g) => format!("the {}", g.name()),
        }
    }

    /// The words take "is" ("the defense is"), not "are".
    pub fn one(self) -> bool {
        matches!(self, Bloc::Offense | Bloc::Defense | Bloc::Room(Pos::OL) | Bloc::Room(Pos::DL))
    }

    /// Whether a man is in the group.
    pub fn has(self, w: &World, i: usize) -> bool {
        let p = &w.people[i];
        let g = Pos::of(p.player.position);
        match self {
            Bloc::Starters => seen::starter(w, i),
            Bloc::Backups => !seen::starter(w, i),
            Bloc::Seniors => match p.stage {
                Stage::Pro => p.pro.as_ref().is_some_and(|c| c.seasons.len() >= 4),
                Stage::College => p.college_seasons >= 3,
                _ => w.year - p.class_of + 12 >= 12,
            },
            Bloc::Newcomers => match p.stage {
                Stage::Pro => p.pro.as_ref().is_none_or(|c| c.seasons.is_empty()),
                Stage::College => p.college_seasons == 0,
                _ => w.year - p.class_of + 12 <= 9,
            },
            Bloc::Offense => g.offense(),
            Bloc::Defense => matches!(g, Pos::DL | Pos::LB | Pos::DB),
            Bloc::Room(x) => g == x,
        }
    }
}

/// The room's level, for its groups' words.
pub fn level(w: &World) -> crate::calendar::Level {
    Team::seat(w).map_or(crate::calendar::Level::College, |t| t.level())
}

/// The groups the ear's card names for a room: the ones with men in them
/// that aren't the whole room. A position room has no sides or rooms.
pub fn blocs(w: &World, room: &Room) -> Vec<Bloc> {
    let men: Vec<usize> = players(w, room);
    if men.len() < 2 {
        return Vec::new();
    }
    let position = matches!(room.channel, Channel::Group(Group::Position(_)));
    let mut all = vec![Bloc::Starters, Bloc::Backups, Bloc::Seniors, Bloc::Newcomers];
    if !position {
        all.extend([Bloc::Offense, Bloc::Defense]);
        all.extend(Pos::ALL.iter().map(|g| Bloc::Room(*g)));
    }
    all.into_iter()
        .filter(|b| {
            let n = men.iter().filter(|&&i| b.has(w, i)).count();
            n > 0 && n < men.len()
        })
        .collect()
}

/// The players in a room, by their place on the people list.
pub fn players(w: &World, room: &Room) -> Vec<usize> {
    listeners(room)
        .filter_map(|t| match t {
            Them::Player(id) => folk::person(w, id),
            _ => None,
        })
        .collect()
}

/// The men of the room in a group.
pub fn members(w: &World, room: &Room, b: Bloc) -> Vec<Them> {
    players(w, room).into_iter().filter(|&i| b.has(w, i)).map(|i| Them::Player(w.people[i].player.id)).collect()
}

/// The groups a man is in, of those the card names.
pub fn his_blocs(w: &World, i: usize, of: &[Bloc]) -> Vec<Bloc> {
    of.iter().copied().filter(|b| b.has(w, i)).collect()
}

/// The group a line opens by calling out ("Defense, ...", "Seniors: ...",
/// "Alright, o-line, ..."), from the words themselves. The ear is told to
/// tag such a line on the group, and doesn't always; the words settle it.
pub fn addressed(words: &str) -> Option<Bloc> {
    const NAMES: &[(&str, Bloc)] = &[
        ("offense", Bloc::Offense),
        ("offence", Bloc::Offense),
        ("defense", Bloc::Defense),
        ("defence", Bloc::Defense),
        ("starters", Bloc::Starters),
        ("ones", Bloc::Starters),
        ("first team", Bloc::Starters),
        ("backups", Bloc::Backups),
        ("back-ups", Bloc::Backups),
        ("twos", Bloc::Backups),
        ("second team", Bloc::Backups),
        ("reserves", Bloc::Backups),
        ("seniors", Bloc::Seniors),
        ("veterans", Bloc::Seniors),
        ("vets", Bloc::Seniors),
        ("upperclassmen", Bloc::Seniors),
        ("older guys", Bloc::Seniors),
        ("freshmen", Bloc::Newcomers),
        ("rookies", Bloc::Newcomers),
        ("newcomers", Bloc::Newcomers),
        ("new guys", Bloc::Newcomers),
        ("young guys", Bloc::Newcomers),
        ("quarterbacks", Bloc::Room(Pos::QB)),
        ("qbs", Bloc::Room(Pos::QB)),
        ("running backs", Bloc::Room(Pos::RB)),
        ("rbs", Bloc::Room(Pos::RB)),
        ("receivers", Bloc::Room(Pos::Receivers)),
        ("wide receivers", Bloc::Room(Pos::Receivers)),
        ("wideouts", Bloc::Room(Pos::Receivers)),
        ("o-line", Bloc::Room(Pos::OL)),
        ("o line", Bloc::Room(Pos::OL)),
        ("offensive line", Bloc::Room(Pos::OL)),
        ("d-line", Bloc::Room(Pos::DL)),
        ("d line", Bloc::Room(Pos::DL)),
        ("defensive line", Bloc::Room(Pos::DL)),
        ("linebackers", Bloc::Room(Pos::LB)),
        ("backers", Bloc::Room(Pos::LB)),
        ("secondary", Bloc::Room(Pos::DB)),
        ("defensive backs", Bloc::Room(Pos::DB)),
        ("dbs", Bloc::Room(Pos::DB)),
        ("specialists", Bloc::Room(Pos::Specialists)),
    ];
    let lower = words.to_lowercase();
    let mut rest = lower.trim_start();
    for lead in ["alright,", "all right,", "okay,", "ok,", "hey,", "listen,", "now,", "and"] {
        if let Some(r) = rest.strip_prefix(lead) {
            rest = r.trim_start();
        }
    }
    let head = rest.split([',', ':', ';', '!', '.', '\u{2014}']).next()?.trim();
    let mut head = head.strip_prefix("my ").or_else(|| head.strip_prefix("the ")).unwrap_or(head).trim();
    for tail in [" guys", " group", " room", " unit"] {
        head = head.strip_suffix(tail).unwrap_or(head);
    }
    // Only a call at the head of the line, set off from the rest ("Defense,
    // ..."): the exact name, so "the defense played hard" is no call.
    NAMES.iter().find(|(n, _)| *n == head).map(|x| x.1)
}

/// A line that opens by calling out one group, read by the ear as to the
/// whole room, goes to that group: its hits on the room move to the group's
/// tag, when the room has that group.
pub fn aim(w: &World, room: &Room, words: &str, mut r: Reading) -> Reading {
    let all = super::gameday::group::ROOM;
    let Some(b) = addressed(words) else { return r };
    if !blocs(w, room).contains(&b) || r.hits.iter().any(|h| Bloc::of(h.on).is_some()) {
        return r;
    }
    for h in r.hits.iter_mut().filter(|h| h.on == all) {
        h.on = b.them();
    }
    r
}

/// Every hit on the whole room or a group becomes one hit on each man in
/// it: on the room as one of them (`Item::Room`), or on his group
/// (`Item::Group`). A man a group's hit reached takes that, not the whole
/// room's reading of the same line (the nearer reading wins).
pub fn expand(w: &World, room: &Room, mut r: Reading) -> Reading {
    let all = super::gameday::group::ROOM;
    let mut out: Vec<Hit> = Vec::new();
    let mut reached: Vec<Them> = Vec::new();
    let push = |out: &mut Vec<Hit>, h: Hit| {
        if !out.contains(&h) {
            out.push(h);
        }
    };
    for h in r.hits.iter().filter(|h| Bloc::of(h.on).is_some()) {
        let b = Bloc::of(h.on).expect("a group");
        for m in members(w, room, b) {
            reached.push(m);
            push(&mut out, Hit { on: m, item: b.item(), how: h.how });
        }
    }
    for h in r.hits.drain(..) {
        if h.on == all {
            for m in listeners(room).filter(|m| !reached.contains(m)) {
                push(&mut out, Hit { on: m, item: Item::Room, how: h.how });
            }
        } else if Bloc::of(h.on).is_none() {
            push(&mut out, h);
        }
    }
    r.hits = out;
    r
}

/// How much a line to a group weighs for him: a line about jobs weighs more
/// on an ambitious man (item 1, "what he wants").
pub fn group_weight(w: &World, on: Them, code: u8) -> f32 {
    let base = super::gameday::group::ROOM_WEIGHT;
    match Bloc::from_code(code) {
        Some(b) if b.about_jobs() => base * (1.0 + ROLE_AMBITION * z(w, on, Trait::Ambition).max(0.0)),
        _ => base,
    }
}

// ---------- 2 and 4: leaders, and a man called out ----------

/// The room's leaders: its captains (or council, or the position room's
/// captain), then the men with the most leadership, at most `MAX_LEADERS`.
pub fn leaders(w: &World, s: Option<&Scene>, room: &Room) -> Vec<Them> {
    let named: Vec<Them> = s.map(|s| super::gameday::group::leads(w, s)).unwrap_or_default();
    let mut out: Vec<Them> = listeners(room).filter(|x| named.contains(x)).collect();
    let mut rest: Vec<(Them, f32)> = listeners(room)
        .filter(|x| matches!(x, Them::Player(_)) && !out.contains(x))
        .map(|x| (x, z(w, x, Trait::Leadership)))
        .filter(|x| x.1 >= LEADER_Z)
        .collect();
    rest.sort_by(|a, b| b.1.total_cmp(&a.1));
    out.extend(rest.into_iter().map(|x| x.0));
    out.truncate(MAX_LEADERS);
    out
}

/// A leader's sway: how far the men close to him follow him.
fn sway(w: &World, l: Them) -> f32 {
    (0.5 + 0.25 * z(w, l, Trait::Leadership)).clamp(0.25, 1.0)
}

/// How close a man is to a leader: their tie, or the same position room,
/// or the same side of the ball.
fn closeness(w: &World, a: usize, b: usize) -> f32 {
    let (pa, pb) = (&w.people[a], &w.people[b]);
    let tie = f32::from(ties::between(w, pa.player.id, pb.player.id)) / 100.0;
    let (ga, gb) = (Pos::of(pa.player.position), Pos::of(pb.player.position));
    let room = if ga == gb {
        0.4
    } else if ga.offense() == gb.offense() && ga != Pos::Specialists && gb != Pos::Specialists {
        0.15
    } else {
        0.0
    };
    tie.max(room)
}

/// What a man took from a line: trust, mood and how it landed most.
pub type Took = (f32, f32, Option<How>);

/// What a line did to one man, summed: (trust, mood), and the way it hit
/// him hardest.
pub fn take(hits: &[Sized], who: Them) -> Took {
    let mut t = (0.0f32, 0.0f32);
    let mut best: Option<(How, f32)> = None;
    for h in hits.iter().filter(|h| h.hit.on == who) {
        t.0 += h.trust;
        t.1 += h.mood;
        let m = (h.trust + h.mood).abs();
        if best.is_none_or(|b| m > b.1) {
            best = Some((h.hit.how, m));
        }
    }
    (t.0, t.1, best.map(|b| b.0))
}

/// Items 2 and 4, on the sized hits of a line to a room before the caps: a
/// man called out in front of the room takes it harder and his friends
/// take a share; then each man moves part of the way toward the leader
/// he's closest to.
pub fn adjust(w: &World, s: Option<&Scene>, room: &Room, hits: &mut Vec<Sized>) {
    // 4. Called out in front of the room.
    let present: Vec<Them> = listeners(room).collect();
    let mut spill: Vec<Sized> = Vec::new();
    for h in hits.iter_mut() {
        let Them::Player(id) = h.hit.on else { continue };
        let own = !matches!(h.hit.item, Item::Room | Item::Group(_) | Item::Friend(_));
        let harsh = matches!(h.hit.how, How::Stung | How::Insulted | How::Threatened | How::Doubted);
        if !own || !harsh || h.trust + h.mood >= 0.0 {
            continue;
        }
        h.trust *= PUBLIC;
        h.mood *= PUBLIC;
        for f in ties::friends(w, id) {
            let on = Them::Player(f);
            if present.contains(&on) {
                spill.push(Sized {
                    hit: Hit { on, item: Item::Friend(id), how: How::Stung },
                    trust: SPILL * h.trust,
                    mood: SPILL * h.mood,
                    faded: false,
                });
            }
        }
    }
    hits.extend(spill);
    // 2. Leaders pull the room.
    let leads = leaders(w, s, room);
    if leads.is_empty() {
        return;
    }
    let lead_i: Vec<(Them, usize, Took, f32)> = leads
        .iter()
        .filter_map(|&l| match l {
            Them::Player(id) => folk::person(w, id).map(|i| (l, i, take(hits, l), sway(w, l))),
            _ => None,
        })
        .collect();
    let mut pulls: Vec<Sized> = Vec::new();
    for i in players(w, room) {
        let me = Them::Player(w.people[i].player.id);
        if leads.contains(&me) {
            continue;
        }
        let Some((l, _, lt, sw, close)) = lead_i
            .iter()
            .map(|(l, li, lt, sw)| (*l, *li, *lt, *sw, closeness(w, i, *li)))
            .max_by(|a, b| (a.4 * a.3).total_cmp(&(b.4 * b.3)))
        else {
            continue;
        };
        let share = PULL * close * sw;
        let Some(lhow) = lt.2 else { continue };
        if share < 0.01 {
            continue;
        }
        let mine = take(hits, me);
        let (dt, dm) = (share * (lt.0 - mine.0), share * (lt.1 - mine.1));
        if dt.abs() + dm.abs() < 0.001 {
            continue;
        }
        let Them::Player(lid) = l else { continue };
        let how =
            if dt + dm >= 0.0 { if lhow == How::Flattered { How::Respected } else { lhow } } else { How::Doubted };
        pulls.push(Sized { hit: Hit { on: me, item: Item::Friend(lid), how }, trust: dt, mood: dm, faded: false });
    }
    hits.extend(pulls);
}

// ---------- 5 and 7: who speaks ----------

/// Who answers a line to a room.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Voices {
    /// The man who answers first; none when the room stays quiet.
    pub first: Option<Them>,
    /// He speaks for himself, not for the room.
    pub own: bool,
    /// The men with something to say who kept it to themselves.
    pub held: Vec<Them>,
}

/// How readily a man speaks up in front of others.
pub fn bold(w: &World, who: Them) -> f32 {
    (1.0 + 0.3 * z(w, who, Trait::Ego) + 0.2 * z(w, who, Trait::Temper) + 0.3 * z(w, who, Trait::Leadership)
        - 0.15 * z(w, who, Trait::Professionalism))
    .clamp(0.3, 2.0)
}

/// Each man's take on a line before the room's own moves (his hits sized
/// on his makeup): signed.
fn takes(w: &World, room: &Room, reading: &Reading, open: &Open) -> Vec<(Them, f32)> {
    let mut out: Vec<(Them, f32)> = listeners(room).map(|x| (x, 0.0)).collect();
    for h in &reading.hits {
        let Some(x) = out.iter_mut().find(|x| x.0 == h.on) else { continue };
        if let Some(sz) = hits::size(w, h, &[], open) {
            x.1 += sz.trust + sz.mood;
        }
    }
    out
}

/// The men a line names by their own things (not the room, a group, or a
/// friend): the one it touched most first.
fn named(reading: &Reading, room: &Room) -> Vec<Them> {
    let mut out: Vec<(Them, usize)> = Vec::new();
    for h in reading.hits.iter().filter(|h| !matches!(h.item, Item::Room | Item::Group(_))) {
        if !listeners(room).any(|x| x == h.on) {
            continue;
        }
        match out.iter_mut().find(|x| x.0 == h.on) {
            Some(x) => x.1 += 1,
            None => out.push((h.on, 1)),
        }
    }
    out.sort_by_key(|x| std::cmp::Reverse(x.1));
    out.into_iter().map(|x| x.0).collect()
}

/// Whether the line asks the room as a whole (an open floor, how the room
/// feels, whether anyone has a problem).
pub fn asks_room(reading: &Reading) -> bool {
    let all = super::gameday::group::ROOM;
    reading.acts.iter().any(|a| match a {
        Act::Question { topic, about, words } => {
            about.is_none_or(|x| x == all || Bloc::of(x).is_some())
                && (matches!(topic, Topic::Nothing | Topic::Mood | Topic::Wants | Topic::HisWord) || open_floor(words))
        }
        _ => false,
    })
}

/// "Anyone have a problem with that?", "Any questions?", "Who's got
/// something?": a question that opens the floor, whatever it's about.
fn open_floor(words: &str) -> bool {
    let l = words.trim().to_lowercase();
    [
        "any ",
        "any?",
        "anyone",
        "anybody",
        "does anyone",
        "does anybody",
        "who has",
        "who's got",
        "who else",
        "questions",
    ]
    .iter()
    .any(|p| l.starts_with(p))
}

/// The groups a line went to, from the hits `expand` gave their men.
fn sent_to(reading: &Reading) -> Vec<Bloc> {
    let mut out = Vec::new();
    for h in &reading.hits {
        if let Item::Group(c) = h.item
            && let Some(b) = Bloc::from_code(c)
            && !out.contains(&b)
        {
            out.push(b);
        }
    }
    out
}

/// Who answers this line (item 5), and who keeps quiet when you ask the
/// room (item 7). `reading` reaches each man already (`expand`).
pub fn voices(w: &World, s: Option<&Scene>, room: &Room, reading: &Reading, open: &Open) -> Voices {
    let present: Vec<Them> = listeners(room).collect();
    let leads = leaders(w, s, room);
    let lead = || leads.first().copied().or_else(|| present.first().copied());
    // A man the line asks about by name answers it himself: he's sitting
    // there.
    for a in &reading.acts {
        if let Act::Question { about: Some(x), .. } = a
            && present.contains(x)
        {
            return Voices { first: Some(*x), own: true, held: Vec::new() };
        }
    }
    // A man the line is to, by his own things.
    if let Some(x) = named(reading, room).first() {
        return Voices { first: Some(*x), own: true, held: Vec::new() };
    }
    let takes = takes(w, room, reading, open);
    let at = |x: Them| takes.iter().find(|t| t.0 == x).map_or(0.0, |t| t.1);
    let big = big(room);
    // You asked the room: the men with something on their mind speak by
    // how bold they are, how much they trust you and how many are
    // listening; the rest keep it for later.
    if asks_room(reading) {
        let mut worried: Vec<(Them, f32, f32)> = present
            .iter()
            .filter_map(|&x| {
                let mood = match x {
                    Them::Player(id) => folk::person(w, id).map_or(0.0, |i| w.people[i].mood),
                    _ => 0.0,
                };
                let worry = (-mood).max(0.0) + 5.0 * (-at(x)).max(0.0);
                (worry >= WORRY).then(|| {
                    let trust = talk::trust_of(w, x);
                    let room_k = if big { BIG_ROOM } else { 1.0 };
                    (x, worry, worry * bold(w, x) * (0.4 + trust) * room_k)
                })
            })
            .collect();
        worried.sort_by(|a, b| b.2.total_cmp(&a.2));
        let bar = if big { 0.2 } else { 0.08 };
        // In front of the whole team only a leader stands up, and he
        // speaks for the room, not for his own job (the replay: a backup
        // asking "where do I stand?" in front of a hundred men).
        let speaks = worried.iter().filter(|x| !big || leads.contains(&x.0)).find(|x| x.2 >= bar).map(|x| x.0);
        let mut held: Vec<(Them, f32)> = worried.iter().filter(|x| Some(x.0) != speaks).map(|x| (x.0, x.1)).collect();
        held.sort_by(|a, b| b.1.total_cmp(&a.1));
        let held: Vec<Them> = held.into_iter().take(3).map(|x| x.0).collect();
        return match speaks {
            Some(x) => Voices { first: Some(x), own: !big, held },
            // A full room stays quiet; a small one has its leader say so.
            None if big => Voices { first: None, own: false, held },
            None => Voices { first: lead(), own: false, held },
        };
    }
    // A line to one group: that group's leader answers for it, or the man in
    // it with the most at stake.
    let to = sent_to(reading);
    let whole = reading.hits.iter().any(|h| h.item == Item::Room);
    if to.len() == 1 && !whole {
        let mine = members(w, room, to[0]);
        let first = leads.iter().copied().find(|l| mine.contains(l)).or_else(|| {
            mine.iter().copied().max_by(|a, b| (z(w, *a, Trait::Leadership)).total_cmp(&z(w, *b, Trait::Leadership)))
        });
        if first.is_some() {
            return Voices { first, own: false, held: Vec::new() };
        }
    }
    // A small room: the man with the most at stake speaks for himself.
    if !big
        && let Some((x, v)) = present.iter().map(|&x| (x, at(x).abs() * bold(w, x))).max_by(|a, b| a.1.total_cmp(&b.1))
        && v >= SPEAK
    {
        return Voices { first: Some(x), own: true, held: Vec::new() };
    }
    // A leader answers for the room: the one on the side of the ball the
    // line went to most, else the first.
    let side = to.iter().find_map(|b| match b {
        Bloc::Offense => Some(true),
        Bloc::Defense => Some(false),
        Bloc::Room(g) if *g != Pos::Specialists => Some(g.offense()),
        _ => None,
    });
    let on_side = side.and_then(|off| {
        leads.iter().copied().find(|l| match l {
            Them::Player(id) => {
                folk::person(w, *id).is_some_and(|i| Pos::of(w.people[i].player.position).offense() == off)
            }
            _ => false,
        })
    });
    Voices { first: on_side.or_else(lead), own: false, held: Vec::new() }
}

/// The man who answers after the first, if one has enough at stake to
/// speak up in front of this room (item 5): a man the line moved the
/// other way from the first counts half again.
pub fn second(w: &World, room: &Room, hits: &[Sized], first: Them) -> Option<Them> {
    let k = if big(room) { BIG_ROOM } else { 1.0 };
    let f = take(hits, first);
    let fs = (f.0 + f.1).signum();
    listeners(room)
        .filter(|x| *x != first && matches!(x, Them::Player(_)))
        .map(|x| {
            let t = take(hits, x);
            let v = t.0 + t.1;
            let other = if fs != 0.0 && v.signum() != fs { 1.5 } else { 1.0 };
            (x, v.abs() * bold(w, x) * k * other)
        })
        .filter(|x| x.1 >= SECOND)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|x| x.0)
}

// ---------- 6: the room's patience and heat ----------

/// The room's patience as the talk opens: its men's, on average.
pub fn start(w: &World, room: &Room, feel: &dyn Feeling) -> Patience {
    let v: Vec<f32> = listeners(room).map(|x| feel.start(w, x).0).collect();
    Patience(if v.is_empty() { 1.0 } else { v.iter().sum::<f32>() / v.len() as f32 })
}

/// The room's patience after a line: each man's own wear, on average (a
/// man the line reached wears as on a line that landed).
pub fn patience(
    w: &World,
    room: &Room,
    feel: &dyn Feeling,
    open: &Open,
    t: &Turn,
    progress: talk::feel::Progress,
) -> Patience {
    let mut sum = 0.0f32;
    let mut n = 0usize;
    for x in listeners(room) {
        let moved = take(&t.hits, x);
        let p = if (moved.0 + moved.1).abs() > 0.01 { talk::feel::Progress::Landed } else { progress };
        sum += feel.patience(w, x, open, t, p).0;
        n += 1;
    }
    if n == 0 { t.patience } else { Patience(sum / n as f32) }
}

/// The room's heat after a line: its men's, on average.
pub fn heat(w: &World, room: &Room, feel: &dyn Feeling, open: &Open, t: &Turn, caught: usize) -> Heat {
    let v: Vec<f32> = listeners(room).map(|x| feel.heat(w, x, open, t, caught).0).collect();
    Heat(if v.is_empty() { open.heat.0 } else { v.iter().sum::<f32>() / v.len() as f32 })
}

/// The men a line sends out of the room on their own (a hothead past his
/// point walks out; the meeting goes on).
pub fn walks(w: &World, room: &Room, feel: &dyn Feeling, open: &Open, t: &Turn) -> Vec<Them> {
    let calm = Open { heat: Heat(0.0), ..open.clone() };
    listeners(room)
        // Only a line that hit him hard himself (called out, or his group
        // taken apart): a hard word to the whole room sends no one out (the
        // replay: a man stormed out on "every job is open").
        .filter(|x| {
            let (tr, m, _) = take(&t.hits, *x);
            tr + m <= -WALK_HURT
        })
        .filter(|x| feel.boil(w, *x, &calm, feel.heat(w, *x, &calm, t, 0)) == Some(Boil::WalksOut))
        .collect()
}

// ---------- 3: a promise to the room ----------

/// The men a promise made in the room is to: the man or group the ear
/// named, else everyone listening. One that puts him ahead of a teammate
/// is to the man who answered.
pub fn covers(w: &World, room: &Room, to: Option<Them>, kind: &str, answering: Them) -> Vec<Them> {
    if kind == "ahead_of" {
        return vec![answering];
    }
    match to {
        Some(x) if x == super::gameday::group::ROOM => listeners(room).collect(),
        Some(x) => match Bloc::of(x) {
            Some(b) => members(w, room, b),
            None if listeners(room).any(|y| y == x) => vec![x],
            None => listeners(room).collect(),
        },
        None => listeners(room).collect(),
    }
}

// ---------- 8: the room's read, by groups ----------

/// How a line went over in the room: a sentence for the screen, and the
/// short read for "Heard as".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Read {
    pub line: String,
    pub heard: String,
}

/// The groups the read names: the sides and the position rooms in a full
/// room, the starters and the backups in a position room.
fn read_blocs(room: &Room) -> Vec<Bloc> {
    if matches!(room.channel, Channel::Group(Group::Position(_))) {
        vec![Bloc::Starters, Bloc::Backups, Bloc::Seniors, Bloc::Newcomers]
    } else {
        let mut v = vec![Bloc::Starters, Bloc::Backups, Bloc::Seniors, Bloc::Newcomers, Bloc::Offense, Bloc::Defense];
        v.extend(Pos::ALL.iter().map(|g| Bloc::Room(*g)));
        v
    }
}

/// "nod along", "trade looks": what a group does, by how the line landed.
fn does(b: Bloc, v: f32) -> String {
    let one = b.one();
    if v > 0.03 {
        if one { "is with you".into() } else { "are with you".into() }
    } else if v > 0.0 {
        "nod along".into()
    } else if v < -0.03 {
        "trade looks".into()
    } else {
        if one { "isn't sold".into() } else { "aren't sold".into() }
    }
}

fn how_word(h: How) -> &'static str {
    match h {
        How::Reassured => "reassured",
        How::Respected => "respected",
        How::Stung => "stung",
        How::Insulted => "insulted",
        How::Doubted => "doubted",
        How::Flattered => "flattered",
        How::Threatened => "threatened",
        How::Staked => "backed",
    }
}

/// Item 8: how the line went over, read by groups, with a man by name only
/// where he stands out. Never who answers: the reply says that.
pub fn read(w: &World, room: &Room, hits: &[Sized], speaker: Option<Them>) -> Read {
    let lvl = level(w);
    let men: Vec<(usize, Them, f32, Option<How>)> = players(w, room)
        .into_iter()
        .map(|i| {
            let x = Them::Player(w.people[i].player.id);
            let t = take(hits, x);
            (i, x, t.0 + t.1, t.2)
        })
        .collect();
    let others: Vec<(Them, f32)> = listeners(room)
        .filter(|x| !matches!(x, Them::Player(_)))
        .map(|x| {
            let t = take(hits, x);
            (x, t.0 + t.1)
        })
        .collect();
    let all: Vec<f32> = men.iter().map(|m| m.2).chain(others.iter().map(|o| o.1)).collect();
    if all.is_empty() || all.iter().all(|v| v.abs() <= 0.005) {
        return Read { line: "They take it in.".into(), heard: "hard to tell how the room took it".into() };
    }
    let n = all.len() as f32;
    let up = all.iter().filter(|v| **v > 0.01).count() as f32;
    let down = all.iter().filter(|v| **v < -0.01).count() as f32;
    let mean = all.iter().sum::<f32>() / n;
    let (overall, overall_heard) = if up >= 0.66 * n {
        ("It lands with the room.", "it landed with most of the room")
    } else if down >= 0.66 * n {
        ("The room goes quiet.", "it didn't land with most of the room")
    } else if up >= 0.25 * n && down >= 0.25 * n {
        ("The room is split.", "the room is split")
    } else {
        ("They take it in.", "hard to tell how the room took it")
    };
    // The groups the line went to, as the ear aimed it: those are named
    // first, however close to the room they sit ("Defense, ..." reads as
    // the defense, never as its linebackers and its secondary).
    let mut aimed: Vec<Bloc> = Vec::new();
    for h in hits {
        if let Item::Group(c) = h.hit.item
            && let Some(b) = Bloc::from_code(c)
            && !aimed.contains(&b)
        {
            aimed.push(b);
        }
    }
    // The groups that stand out from the room, the furthest first. To the
    // whole room, a group is named only when it clearly stands apart: half
    // again the room's own read, and a position room only when it went the
    // other way from the room (a few men in a small room always differ a
    // little, which is no read).
    let mut stand: Vec<(Bloc, f32, Option<How>)> = Vec::new();
    for b in aimed.iter().copied().chain(read_blocs(room).into_iter().filter(|b| !aimed.contains(b))) {
        let mine: Vec<&(usize, Them, f32, Option<How>)> = men.iter().filter(|m| b.has(w, m.0)).collect();
        if mine.len() < 2 || mine.len() == men.len() {
            continue;
        }
        let v = mine.iter().map(|m| m.2).sum::<f32>() / mine.len() as f32;
        if v.abs() < 0.008 {
            continue;
        }
        if !aimed.contains(&b) {
            let apart = (v - mean).abs();
            if apart < 0.012f32.max(0.5 * mean.abs()) {
                continue;
            }
            if matches!(b, Bloc::Room(_)) && (v > 0.0) == (mean > 0.0) {
                continue;
            }
            if !aimed.is_empty() && (v > 0.0) == (mean > 0.0) {
                continue;
            }
        }
        // The way the line hit most of them.
        let mut hows: Vec<(How, usize)> = Vec::new();
        for m in &mine {
            if let Some(h) = m.3 {
                match hows.iter_mut().find(|x| x.0 == h) {
                    Some(x) => x.1 += 1,
                    None => hows.push((h, 1)),
                }
            }
        }
        let how = hows.into_iter().max_by_key(|x| x.1).map(|x| x.0);
        stand.push((b, v, how));
    }
    // The aimed groups first, then the furthest from the room.
    // A side before its rooms, so "Defense, ..." tagged on the defense and
    // its linebackers reads as the defense alone.
    let side = |x: Bloc| matches!(x, Bloc::Offense | Bloc::Defense);
    stand.sort_by(|a, b| {
        aimed
            .contains(&b.0)
            .cmp(&aimed.contains(&a.0))
            .then(side(b.0).cmp(&side(a.0)))
            .then((b.1 - mean).abs().total_cmp(&(a.1 - mean).abs()))
    });
    // A side covers its rooms: keep a room only when its side isn't named.
    let mut named: Vec<(Bloc, f32, Option<How>)> = Vec::new();
    for x in stand {
        let covered = named.iter().any(|y| match (y.0, x.0) {
            (Bloc::Offense, Bloc::Room(g)) => g.offense(),
            (Bloc::Defense, Bloc::Room(g)) => !g.offense() && g != Pos::Specialists,
            _ => false,
        });
        if !covered && named.len() < 2 {
            named.push(x);
        }
    }
    let cap = |s: String| {
        let mut c = s.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    };
    let mut line: Vec<String> = Vec::new();
    if named.is_empty() {
        line.push(overall.into());
    }
    for (b, v, _) in &named {
        line.push(format!("{} {}.", cap(b.words(lvl)), does(*b, *v)));
    }
    // A man who stands out from his group, by name: never the one who is
    // about to answer.
    let out = men
        .iter()
        .filter(|m| Some(m.1) != speaker)
        .filter(|m| m.2.abs() >= 0.04 && (m.2 - mean).abs() >= 0.03)
        .max_by(|a, b| (a.2 - mean).abs().total_cmp(&(b.2 - mean).abs()));
    if let Some(m) = out {
        let name = folk::player_link(w, m.0);
        line.push(if m.2 < 0.0 { format!("{name} stares at the floor.") } else { format!("{name} is nodding hard.") });
    }
    // Parents or coaches in the room: how many of them it reached.
    if men.is_empty() && !others.is_empty() {
        line.clear();
        line.push(overall.into());
    }
    let mut heard: Vec<String> =
        named.iter().filter_map(|(b, _, h)| h.map(|h| format!("{} {}", b.words(lvl), how_word(h)))).collect();
    heard.push(overall_heard.into());
    Read { line: line.join(" "), heard: heard.join("; ") }
}

/// A man's own take for his tag in the room list: how your last line landed
/// with him ("With you", "Stung"), or none when it barely reached him.
pub fn tag(v: f32, how: How) -> Option<&'static str> {
    if v.abs() <= 0.005 {
        return None;
    }
    Some(match how {
        How::Insulted if v < 0.0 => "Insulted",
        How::Threatened if v < 0.0 => "Rattled",
        How::Stung if v < 0.0 => "Stung",
        _ if v < -0.03 => "Not buying it",
        _ if v < 0.0 => "Unsure",
        How::Flattered => "Pumped",
        _ if v > 0.04 => "With you",
        _ => "Nodding",
    })
}

// ---------- 7: a man who held back ----------

/// The days a man who held back waits before he asks to see you.
pub const HELD_DAYS: i32 = 1;

/// A man who kept it to himself in front of the room asks to see you the
/// next day (item 7): the most bothered one who trusts you enough to come,
/// at most one a meeting, never one already on your calendar.
pub fn held(w: &mut World, held: &[Them]) {
    let Some(team) = Team::seat(w) else { return };
    let d = w.today.add(HELD_DAYS);
    for x in held {
        let Them::Player(id) = *x else { continue };
        let Some(i) = folk::person(w, id) else { continue };
        if !team.roster(w).contains(&i) || talk::trust_of(w, *x) < 0.35 {
            continue;
        }
        let booked = w.folk.scenes.iter().any(|s| !s.done && s.kind == Kind::OneOnOne && s.a == u64::from(id));
        if booked {
            continue;
        }
        let subject = if seen::starter(w, i) { talks::Subject::Team } else { talks::Subject::Role };
        let mut s = Scene::new(Kind::OneOnOne, d, format!("{} wants a word after the meeting", w.people[i].name));
        s.a = u64::from(id);
        s.b = subject as u64;
        s.place = "Your office".into();
        folk::scene::add(w, s);
        return;
    }
}

// ---------- 9: position room meetings ----------

/// The men in a position room on your roster today (not the ones gone for
/// the draft).
pub fn position_men(w: &World, team: Team, g: Pos) -> Vec<usize> {
    super::gameday::group::in_room(w, team).into_iter().filter(|&i| Pos::of(w.people[i].player.position) == g).collect()
}

/// Opens a meeting with one position room (item 9): once a week a room,
/// half an hour of your week, with the room's position coach at the front.
pub fn start_position_room(w: &mut World, g: Pos) -> Result<u32, String> {
    let team = Team::seat(w).ok_or("You have no team.")?;
    let code = g as u64;
    if let Some(s) = w.folk.scenes.iter().find(|s| !s.done && s.kind == Kind::PositionRoom && s.a == code) {
        return Ok(s.id);
    }
    if position_men(w, team, g).is_empty() {
        return Err(format!("You have no {} to meet.", g.name()));
    }
    let wk = w.today.add(-w.today.weekday().index());
    if w.folk.scenes.iter().any(|s| s.kind == Kind::PositionRoom && s.a == code && s.done && s.date >= wk) {
        return Err(format!("You met the {} this week.", g.name()));
    }
    if !talks::spend(w, 0.5) {
        return Err("Your week is full.".into());
    }
    let mut s = Scene::new(Kind::PositionRoom, w.today, format!("The {} room", g.name()));
    s.stops = false;
    s.team = Some(team);
    s.a = code;
    s.place = format!("The {} meeting room", g.name());
    Ok(folk::scene::add(w, s))
}

/// The position room a scene meets.
pub fn position_of(s: &Scene) -> Option<Pos> {
    (s.kind == Kind::PositionRoom).then(|| Pos::ALL.get(s.a as usize).copied()).flatten()
}

/// Your position coach for a room, on your staff today.
pub fn position_coach(w: &World, team: Team, g: Pos) -> Option<usize> {
    crate::world::staff::holder(w, team.employer(), crate::world::coach::Role::PositionCoach(g))
}

/// The position room's opening: who's there, and the chips.
pub fn position_open(w: &mut World, s: &mut Scene) {
    let (Some(team), Some(g)) = (s.team.or(Team::seat(w)), position_of(s)) else {
        s.end();
        return;
    };
    let men = position_men(w, team, g);
    s.who.push(Who::Group(format!("The {} ({} players)", g.name(), men.len())));
    for &i in &men {
        s.who.push(Who::Player(w.people[i].player.id));
    }
    let names: Vec<String> = men.iter().take(12).map(|&i| folk::player_link(w, i)).collect();
    let more = men.len().saturating_sub(12);
    let coach =
        position_coach(w, team, g).map(|c| format!("{} is at the board. ", w.coaches[c].name)).unwrap_or_default();
    s.narrate(format!(
        "{}. {coach}Around the table: {}{}.",
        s.place,
        names.join(", "),
        if more > 0 { format!(" and {more} more") } else { String::new() }
    ));
    s.pick(610, "Where's this room at?", "ask", "calm", "They tell you what they see, as far as they will.");
    s.pick(611, "That's all for today.", "end", "calm", "");
}

/// The position room's chips: 610 asks how the room is (once), 611 ends it.
pub fn position_answer(w: &mut World, s: &mut Scene, code: u32) {
    s.you(&folk::scene::your_name(w), if code == 610 { "Where's this room at?" } else { "That's all for today." });
    let (Some(team), Some(g)) = (s.team.or(Team::seat(w)), position_of(s)) else {
        s.end();
        return;
    };
    if code != 610 {
        s.end();
        return;
    }
    let men = position_men(w, team, g);
    for line in super::gameday::group::read_of(w, &men, &men) {
        match line {
            (Some(by), text) => s.them(&by, text, None),
            (None, text) => s.narrate(text),
        }
    }
    s.pick(611, "That's all for today.", "end", "calm", "");
}

/// What a line to a room did beyond one man's answer (`Turn.crowd`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Turned {
    /// The man who answered speaks for himself, not for the room.
    pub own: bool,
    /// Nobody answered: you asked a full room and it stayed quiet.
    pub silent: bool,
    /// A second man answers after the first.
    pub second: Option<Them>,
    /// The men with something to say who kept it to themselves.
    pub held: Vec<Them>,
    /// The men who walked out on this line.
    pub walked: Vec<Them>,
    /// The room started to drift on this line.
    pub drift: bool,
}

/// What the second man heard in a line, for his voice card (item 5): the
/// group he heard it as one of, and which way it took him.
pub fn take_words(w: &World, hits: &[Sized], who: Them) -> String {
    let t = take(hits, who);
    let group = hits.iter().filter(|h| h.hit.on == who).find_map(|h| match h.hit.item {
        Item::Group(c) => Bloc::from_code(c),
        _ => None,
    });
    let as_one = group.map(|b| format!(" as one of {}", b.words(level(w)))).unwrap_or_default();
    let how = t.2.map_or("unmoved", how_word);
    let way = if t.0 + t.1 >= 0.0 { "he's with the coach on it" } else { "he doesn't like it" };
    format!(
        "He heard the coach's line{as_one} and felt {how}: {way}. He speaks up after the first man, for himself, briefly, in front of everyone"
    )
}

/// The second man, said plainly when there's no voice to say it.
pub fn second_words(w: &World, hits: &[Sized], who: Them) -> String {
    let t = take(hits, who);
    let name = match who {
        Them::Player(id) => folk::person(w, id).map(|i| folk::player_link(w, i)),
        _ => None,
    }
    .unwrap_or_else(|| super::talk::channels::name(w, who));
    if t.0 + t.1 >= 0.0 {
        format!("{name} speaks up too: he's with you on it.")
    } else {
        format!("{name} speaks up too: he doesn't like it.")
    }
}

/// A man walking out, for the screen.
pub fn walked_words(w: &World, who: Them) -> String {
    let name = match who {
        Them::Player(id) => folk::person(w, id).map(|i| folk::player_link(w, i)),
        _ => None,
    }
    .unwrap_or_else(|| super::talk::channels::name(w, who));
    format!("{name} shoves his chair back and walks out.")
}
