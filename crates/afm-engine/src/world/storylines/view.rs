//! The screens' side of the storylines (S5, S11): the reads and the moves
//! the game crate needs for This week, the story page, the Continue stop
//! and "Previously", the season's chapters and your career shelf. The
//! layout lives in afm-game; here are the words and numbers.
//!
//! Wording thresholds are Claude's picks (proposed, not confirmed).

use super::{BeatKind, Family, Part, Phase, Story, Who, moves};
use crate::calendar::Date;
use crate::world::World;
use crate::world::stories::TeamKey;

/// One answer you can give on a story's page.
#[derive(Clone, Debug)]
pub struct Choice {
    pub code: u32,
    pub label: String,
    /// The move's short name ("Back him", "No comment").
    pub name: &'static str,
    pub tone: &'static str,
    /// What your staff expects it to do.
    pub hint: String,
    /// Said where the papers hear it.
    pub public: bool,
    /// What your staff expects it to do: (tag, tone 0 good, 1 plain, 2 bad).
    pub chips: Vec<(String, u8)>,
}

/// What you can do about a story you are in, public and private.
pub fn choices(w: &mut World, id: u32) -> Vec<Choice> {
    let Some(s) = w.storylines.get(id).cloned() else { return Vec::new() };
    if s.you.is_none() || !s.phase.open() {
        return Vec::new();
    }
    moves::menu(w, &s, false)
        .into_iter()
        .map(|o| Choice {
            code: o.mv.code(),
            chips: moves::chips(w, &s, o.mv),
            label: o.label,
            name: o.mv.name(),
            tone: o.mv.tone(),
            hint: o.hint,
            public: o.mv.public(),
        })
        .collect()
}

/// Makes a move from the story's page; returns what came of it.
pub fn choose(w: &mut World, id: u32, code: u32) -> Result<Vec<String>, String> {
    let Some(s) = w.storylines.get(id).cloned() else { return Err("No such story.".into()) };
    if s.you.is_none() {
        return Err("You aren't in that story.".into());
    }
    if !s.phase.open() {
        return Err("That story is over.".into());
    }
    let opt = moves::menu(w, &s, false).into_iter().find(|o| o.mv.code() == code).ok_or("That isn't on the table.")?;
    let mut out = moves::make(w, id, opt.mv, &opt.label);
    if out.is_empty() {
        out.push(format!("{}.", opt.mv.name()));
    }
    Ok(out)
}

/// Follows a story (its beats reach your inbox until it ends) or stops.
pub fn follow(w: &mut World, id: u32, on: bool) -> bool {
    let known = w.storylines.get(id).is_some_and(|s| s.public);
    let f = &mut w.storylines.followed;
    match (on, f.binary_search(&id)) {
        (true, Err(k)) if known => f.insert(k, id),
        (false, Ok(k)) => {
            f.remove(k);
        }
        _ => {}
    }
    f.binary_search(&id).is_ok()
}

pub fn following(w: &World, id: u32) -> bool {
    w.storylines.followed.binary_search(&id).is_ok()
}

/// The question with its names filled.
pub fn question(w: &World, s: &Story) -> String {
    super::tell::question(w, s)
}

/// Your part in it, in the board's words.
pub fn role(w: &World, s: &Story) -> &'static str {
    let me = crate::world::stories::seat_coach(w).map(|c| c as u32);
    match s.you {
        Some(Part::Lead) if me.is_some_and(|c| s.lead() == Some(Who::Coach(c))) => "lead",
        Some(Part::Lead) => match s.family() {
            Family::Recruiting => "his recruiter's boss",
            _ => "his coach",
        },
        Some(Part::Decider) => "decider",
        Some(Part::Rival) => "the other side",
        Some(Part::Boss) => "the boss",
        Some(Part::Suitor) => "a suitor",
        Some(_) => "in it",
        None => "",
    }
}

/// What the people inside know: which way it is really going.
pub fn inside(s: &Story) -> &'static str {
    match s.truth {
        t if t > 0.5 => "it is happening",
        t if t > 0.2 => "it leans that way",
        t if t >= -0.2 => "it could go either way",
        t if t >= -0.5 => "it leans the other way",
        _ => "it isn't happening",
    }
}

/// What the public believes, in words, and how sure it is (0-100).
pub fn outside(s: &Story) -> (String, u8) {
    (moves::public_read(s), (s.sure * 100.0).round().clamp(0.0, 100.0) as u8)
}

/// The two sides of a story as one split: who backs the lead and who
/// doesn't, among the audience whose split tells most for its family.
pub struct Split {
    pub a: String,
    pub b: String,
    /// The share on side a, 0-100.
    pub share: u8,
    pub audience: &'static str,
}

fn last_name(w: &World, who: Option<Who>) -> Option<String> {
    let full = match who? {
        Who::Player(id) => super::pi(w, id).map(|i| w.people[i].name.clone())?,
        Who::Coach(c) => w.coaches.get(c as usize)?.name.clone(),
        Who::Team(t) => crate::world::stories::short_name(w, t),
        _ => return None,
    };
    Some(full.split_whitespace().last().unwrap_or(&full).to_string())
}

pub fn split(w: &World, s: &Story) -> Split {
    let lead = last_name(w, s.lead());
    let rival = last_name(w, s.rival());
    let pct = |x: f32| (x * 100.0).round().clamp(0.0, 100.0) as u8;
    let team = s.team.map(|t| crate::world::stories::short_name(w, t));
    let other = s.other.map(|t| crate::world::stories::short_name(w, t));
    let (a, b, k, audience) = match s.family() {
        Family::Battle => (lead.clone().unwrap_or("The starter".into()), rival.unwrap_or("A change".into()), 0, "locker room"),
        Family::Seat => ("He stays".into(), "He goes".into(), 1, "fans"),
        Family::Wanted | Family::Leaving => ("He stays".into(), "He goes".into(), 1, "fans"),
        Family::Recruiting => {
            (team.clone().unwrap_or("Here".into()), other.unwrap_or("Elsewhere".into()), 2, "insiders' picks")
        }
        Family::Rivalry => (team.clone().unwrap_or("Yes".into()), other.unwrap_or("No".into()), 1, "fans"),
        Family::OffField => ("Stand by him".into(), "Act now".into(), 1, "fans"),
        Family::Draft => ("Goes high".into(), "Slides".into(), 2, "mock drafts"),
        Family::Program | Family::You => ("Yes".into(), "Not yet".into(), 1, "fans"),
        _ => ("Yes".into(), "No".into(), 1, "fans"),
    };
    Split { a, b, share: pct(s.sides[k]), audience }
}

/// The four audiences and their share backing the lead, 0-100.
pub fn audiences(s: &Story) -> [(&'static str, u8); 4] {
    let pct = |x: f32| (x * 100.0).round().clamp(0.0, 100.0) as u8;
    [("Locker room", pct(s.sides[0])), ("Fans", pct(s.sides[1])), ("Media", pct(s.sides[2])), ("Boosters", pct(s.sides[3]))]
}

/// Heat each Monday, then today's: the sparkline.
pub fn heat_path(s: &Story) -> Vec<u8> {
    let mut v = s.weeks.clone();
    v.push(s.heat.round().clamp(0.0, 100.0) as u8);
    v
}

/// The heat a week ago, for "+6 this week".
pub fn heat_week_ago(s: &Story) -> u8 {
    let n = s.weeks.len();
    if n == 0 { 0 } else { s.weeks[n - 1] }
}

/// What forces it next, in words, and the date when there is one.
pub fn next(w: &World, s: &Story) -> (&'static str, Option<Date>) {
    if !s.phase.open() {
        return ("", None);
    }
    let what = match s.family() {
        Family::Battle => "the depth chart",
        Family::Seat => "the decision on his future",
        Family::Wanted => "the other job's decision",
        Family::Recruiting => "his decision",
        Family::Leaving => "the deadline",
        Family::Comeback => "his next game",
        Family::Streak | Family::Rivalry => "the next game",
        Family::Rise => "the next few games",
        Family::Draft => "the draft",
        Family::OffField => "the decision on his status",
        Family::Program => "the next decision",
        Family::Ending => "the last one",
        Family::You => "the answer",
    };
    (what, s.due.filter(|&d| d >= w.today))
}

/// Whether a story waits on you: a meeting about it is open, or it is
/// yours to decide and running hot.
pub fn needs_you(w: &World, s: &Story) -> bool {
    use crate::world::folk::scene::Kind;
    s.phase.open()
        && (w.folk.scenes.iter().any(|x| x.kind == Kind::Story && !x.done && x.a == s.id as u64)
            || (matches!(s.you, Some(Part::Decider | Part::Lead)) && s.heat >= 55.0 && s.phase == Phase::Peak))
}

/// Your stories, hottest first: the open ones you are in.
pub fn mine(w: &World) -> Vec<&Story> {
    let mut v: Vec<&Story> = w.storylines.open.iter().filter(|s| s.you.is_some() && s.public && s.phase.open()).collect();
    v.sort_by(|a, b| b.heat.total_cmp(&a.heat).then(a.id.cmp(&b.id)));
    v
}

/// Your private stories: the ones only the building knows.
pub fn private(w: &World) -> Vec<&Story> {
    let mut v: Vec<&Story> = w.storylines.open.iter().filter(|s| s.you.is_some() && !s.public && s.phase.open()).collect();
    v.sort_by(|a, b| b.heat.total_cmp(&a.heat).then(a.id.cmp(&b.id)));
    v
}

/// The hottest public stories you are not in, at your level first.
pub fn elsewhere(w: &World, n: usize) -> Vec<&Story> {
    let level = w.seat.map(|s| s.level);
    let mut v: Vec<&Story> = w
        .storylines
        .open
        .iter()
        .filter(|s| s.public && s.phase.open() && s.you.is_none())
        .filter(|s| level.is_none_or(|l| s.level == l || s.family() == Family::Recruiting && l == crate::calendar::Level::College))
        .collect();
    v.sort_by(|a, b| b.heat.total_cmp(&a.heat).then(a.id.cmp(&b.id)));
    v.truncate(n);
    v
}

/// Stories that ended since a day, yours or followed first.
pub fn ended_since(w: &World, from: Date) -> Vec<&Story> {
    let mut v: Vec<&Story> = w
        .storylines
        .open
        .iter()
        .chain(w.storylines.ended.iter())
        .filter(|s| s.public && s.phase != Phase::Fizzled && s.closed.is_some_and(|c| c >= from))
        .filter(|s| s.you.is_some() || following(w, s.id) || s.near)
        .collect();
    v.sort_by(|a, b| b.closed.cmp(&a.closed).then(b.peak_heat.total_cmp(&a.peak_heat)));
    v.dedup_by_key(|s| s.id);
    v
}

/// How a story ended: its last answer, when it has one.
pub fn ended_text(s: &Story) -> Option<String> {
    s.beats.iter().rev().find(|b| b.what == BeatKind::Resolve && !b.text.is_empty()).map(|b| b.text.clone())
}

/// The beats the public saw, newest last, with their source.
pub fn public_beats(s: &Story) -> Vec<&super::Beat> {
    s.beats.iter().filter(|b| b.what.public() && !b.text.is_empty()).collect()
}

/// Your staff's private reads on it, newest first.
pub fn staff_reads(s: &Story) -> Vec<&super::Beat> {
    s.beats.iter().rev().filter(|b| b.what == BeatKind::Memo && !b.text.is_empty()).take(3).collect()
}

/// The source of a beat in the board's words.
pub fn source(s: &Story, b: &super::Beat) -> &'static str {
    use super::Voice;
    match (b.by, b.what) {
        (Voice::World, BeatKind::Resolve) => "The answer",
        (Voice::World, _) => "The record",
        (Voice::Beat, BeatKind::Leak) => "Sources",
        (Voice::Insider, BeatKind::Leak) => "Recruiting sources",
        (v, _) if s.family() == Family::Recruiting && v == Voice::Player => "His own feed",
        (v, _) => v.name(),
    }
}

/// A cast member's side in words.
pub fn side_words(side: i8) -> &'static str {
    match side {
        x if x >= 40 => "with him",
        x if x >= 10 => "leans with him",
        x if x > -10 => "on the fence",
        x if x > -40 => "leans against",
        _ => "against him",
    }
}

/// A player's mood in a word.
pub fn mood_word(m: f32) -> &'static str {
    match m {
        x if x > 0.45 => "great",
        x if x > 0.15 => "good",
        x if x > -0.15 => "steady",
        x if x > -0.45 => "edgy",
        _ => "unhappy",
    }
}

/// Marks "Previously" as seen today.
pub fn previously_seen(w: &mut World) {
    w.storylines.previously_seen = Some(w.today);
}

/// The stories a team carries, open or ended this season.
pub fn about(w: &World, t: TeamKey) -> Vec<&Story> {
    w.storylines.open.iter().filter(|s| s.public && s.about(t)).collect()
}

/// A story's opening day and its end (or today), for a chapter's span.
pub fn span(w: &World, s: &Story) -> (Date, Date) {
    (s.public_on.unwrap_or(s.opened), s.closed.unwrap_or(w.today))
}
