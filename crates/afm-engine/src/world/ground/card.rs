//! A person's page (GR62, GR18, GR26, for the person screen, lane S5):
//! everything between you and one person in words. Who he is, his face,
//! what he pushes, what he wants from you, his ties and his life; your
//! standing and its why; the moments he keeps; his kids with you; your
//! word and his; favors either way. Never a hidden number: his eye, how
//! straight he talks and his nature show only through his record and your
//! staff's read.
//!
//! Live talks read the same memory (`talk_lines`): the moments he brings
//! up, in his words, so the voice only words what's there.

use super::people::{self, Holder, Who};
use super::standing::{self, Level};
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::persona::Them;

/// One person's page.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Card {
    pub who: Who,
    pub name: String,
    pub role: String,
    pub kind: &'static str,
    /// The face recipe and heritage the screens draw from.
    pub looks: u32,
    pub heritage: String,
    pub level: &'static str,
    pub why: String,
    /// Who holds your best standing with him: the program or one of your
    /// men (GR29).
    pub held_by: String,
    pub agenda: &'static str,
    pub wants: Vec<&'static str>,
    pub life: Vec<String>,
    pub ties: Vec<Tie>,
    /// The moments he keeps, in his words, newest first.
    pub moments: Vec<(String, String, bool)>,
    /// His kids with you: (name, how).
    pub kids: Vec<(String, &'static str)>,
    pub favors: Option<String>,
    pub favor_list: Vec<(String, String)>,
    pub your_word: Option<String>,
    pub his_word: Option<String>,
    /// Your staff's read of how straight he talks, in words, once there is
    /// a record to read.
    pub read: Option<String>,
    /// His kids now (a coach's best recruits, a trainer's clients, a
    /// family's son): player ids.
    pub players: Vec<u32>,
    /// When he writes or calls ("1:20 PM").
    pub hour: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Tie {
    pub who: Who,
    pub name: String,
    pub role: String,
    pub level: &'static str,
}

/// The page, for your program.
pub fn card(w: &World, prog: usize, who: Who) -> Card {
    let mk = people::makeup(w, who);
    let (pts, holder) = standing::staff(w, prog, who);
    let mem = w.ground.get(who, holder);
    let mut moments: Vec<(String, String, bool)> = Vec::new();
    let mut all: Vec<&super::Moment> = w.ground.with(who).iter().flat_map(|m| m.moments.iter()).collect();
    all.sort_by_key(|m| std::cmp::Reverse(m.date));
    for m in all {
        if !moments.iter().any(|x| x.1 == m.words) {
            moments.push((people::month_year(m.date), m.words.clone(), m.good));
        }
        if moments.len() == super::memory::MOMENTS {
            break;
        }
    }
    let ties = people::ties(w, who)
        .into_iter()
        .take(8)
        .map(|t| Tie {
            who: t,
            name: people::name(w, t),
            role: people::role(w, t),
            level: standing::level(w, prog, t).name(),
        })
        .collect();
    let players = match who {
        Who::Coach(_) | Who::Counselor(_) => {
            people::school(w, who).map(|h| people::kids_at(w, h, 6)).unwrap_or_default()
        }
        Who::Trainer(_) | Who::SevenOnSeven(_) => people::clients(w, who, 6),
        Who::Family(k) => crate::world::folk::person(w, k).into_iter().collect(),
        Who::Prep(_) => Vec::new(),
    }
    .into_iter()
    .map(|i| w.people[i].player.id)
    .collect();
    let held_by = match holder {
        Holder::Program(p) => w.program_name(p as usize).to_string(),
        Holder::Coach(c) if Some(c as usize) == w.career.as_ref().map(|x| x.coach) => "you".into(),
        Holder::Coach(c) => w.coaches.get(c as usize).map_or_else(String::new, |c| c.name.clone()),
    };
    let your_word = mem.and_then(|m| m.your_word_words());
    let his_word = mem.and_then(|m| m.his_word_words());
    Card {
        who,
        name: people::name(w, who),
        role: people::role(w, who),
        kind: people::kind(w, who).words(),
        looks: people::looks(w, who).0,
        heritage: format!("{:?}", people::heritage(w, who)),
        level: Level::of(pts).name(),
        why: standing::why(w, prog, who),
        held_by,
        agenda: mk.agenda.words(),
        wants: people::wants(w, who),
        life: people::life(w, who),
        ties,
        moments,
        kids: mem.map(|m| m.kids.iter().rev().map(|k| (k.1.clone(), k.2.words())).collect()).unwrap_or_default(),
        favors: mem.and_then(|m| m.favor_words()),
        favor_list: mem
            .map(|m| m.favors.iter().rev().map(|f| (people::month_year(f.date), f.words.clone())).collect())
            .unwrap_or_default(),
        your_word,
        read: mem.and_then(|m| read(m.his_word)),
        his_word,
        players,
        hour: clock(mk.hour),
    }
}

/// Your staff's read of his word, once three of his tips have settled: it
/// follows his record, never the number behind it.
fn read(rec: (u8, u8)) -> Option<String> {
    let (n, held) = rec;
    if n < 3 {
        return None;
    }
    let r = held as f32 / n as f32;
    Some(
        match r {
            x if x >= 0.75 => "his word holds up",
            x if x >= 0.5 => "he oversells a little",
            _ => "he oversells his kids",
        }
        .into(),
    )
}

fn clock(min: u16) -> String {
    let (h, m) = (min / 60, min % 60);
    let (h12, am) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    format!("{h12}:{m:02} {am}")
}

/// The ground person a live talk is with: a coach on your ground, or a
/// recruit's family.
pub fn who_of(w: &World, him: Them) -> Option<Who> {
    match him {
        Them::Coach(c) => {
            let who = Who::Coach(c as u32);
            people::on_ground(w, who).then_some(who)
        }
        Them::Kin(id, _) => crate::world::folk::person(w, id).map(|i| people::family_of(w, i)),
        _ => None,
    }
}

/// What he brings up in a talk (GR62: he brings up his moments, and the AI
/// only words what's there): his moments with your staff, the big ones
/// first, then the newest, up to three, each with its day.
pub fn talk_lines(w: &World, him: Them) -> Vec<(Date, String)> {
    let Some(prog) = standing::seat_program(w) else { return Vec::new() };
    let Some(who) = who_of(w, him) else { return Vec::new() };
    let hs = standing::holders(w, prog);
    let mut v: Vec<&super::Moment> =
        w.ground.with(who).iter().filter(|m| hs.contains(&m.holder)).flat_map(|m| m.moments.iter()).collect();
    v.sort_by_key(|m| (std::cmp::Reverse(m.big), std::cmp::Reverse(m.date)));
    let mut out: Vec<(Date, String)> = Vec::new();
    for m in v {
        if !out.iter().any(|x| x.1 == m.words) {
            out.push((m.date, m.words.clone()));
        }
        if out.len() == 3 {
            break;
        }
    }
    out
}

/// Where your staff stands with him, in words, for a talk ("Trusted: four
/// visits this year, ...").
pub fn talk_standing(w: &World, him: Them) -> Option<String> {
    let prog = standing::seat_program(w)?;
    let who = who_of(w, him)?;
    if w.ground.with(who).is_empty() {
        return None;
    }
    Some(standing::why(w, prog, who))
}
