//! What a coach can tell you of his own work (Cain 2026-11-17): the system
//! he runs or would run (`Topic::Scheme`) and his recruiting
//! (`Topic::Recruiting`), from the world: what he knows of each system
//! (`systems`, the same knowledge he installs from), how the roster fits it
//! (`systems::roster_fit`), what the team runs now, his recruiting area
//! (`scouting::dept::area_coach`), the recruits he's working and the ones
//! who signed through him. Before this a coordinator asked "what offense
//! will you be running?" had only his unit's points to give and dodged, and
//! a recruiter claimed "the best kid in the state" with nobody behind it.
//!
//! The words are Claude's (proposed, not confirmed); every fact is the
//! world's.

use super::Them;
use super::knows::{Known, Source, Topic};
use super::makeup;
use super::room::{Channel, Group as RoomGroup, Room};
use crate::world::World;
use crate::world::coach::{Group, Role};
use crate::world::folk::scene::Kind as SceneKind;
use crate::world::folk::{Team, coach_look, facts};
use crate::world::systems::{self, Kind as SysKind};

/// The most recruits he names from his board, and from his signings.
pub const NAMED: usize = 3;

/// The classes his signings are counted over: this one and the last.
pub const CLASSES: i32 = 2;

/// A coach you are interviewing for a job on your staff, today.
pub fn interviewing(w: &World, c: usize) -> bool {
    w.folk.scenes.iter().any(|s| s.kind == SceneKind::Interview && !s.done && s.a == c as u64)
}

fn known(topic: Topic, words: String) -> Known {
    Known {
        topic,
        about: None,
        from: Source::Himself,
        eye: 20.0,
        point: None,
        says: Vec::new(),
        words,
        sure: 1.0,
        day: None,
    }
}

/// The sides of the ball he'd speak to: (offense, defense). A head coach
/// both; a coordinator or position coach his own; a special teams coach
/// neither; a man out of work the side he came up on.
fn sides(w: &World, c: usize) -> (bool, bool) {
    let co = &w.coaches[c];
    match co.job.map(|j| j.role) {
        Some(Role::HeadCoach) => (true, true),
        Some(Role::OffensiveCoordinator) => (true, false),
        Some(Role::DefensiveCoordinator) => (false, true),
        Some(Role::PositionCoach(Group::Specialists)) => (false, false),
        Some(Role::PositionCoach(g)) => (g.offense(), !g.offense()),
        None if co.came_up == Group::Specialists => (false, false),
        None => (co.came_up.offense(), !co.came_up.offense()),
    }
}

/// The roster his scheme is read against: yours when he works for you or
/// is interviewing with you (he studied your tape), else his own team's.
fn roster_team(w: &World, c: usize) -> Option<Team> {
    let mine = Team::seat(w);
    let his = makeup::coach_team(w, c);
    if mine.is_some() && (his == mine || interviewing(w, c)) { mine } else { his }
}

fn offense_words(slot: usize) -> &'static str {
    [
        "a West Coast offense: quick, timed throws that stretch the field sideways, with the run game and play action built off them",
        "an air raid: four and five wide, spread the field and throw it, mesh and four verticals",
        "a wide zone offense: outside zone runs that stretch the line, with play action and bootlegs off them",
        "a power gap offense: downhill gap runs with pullers and tight ends, and play action shots off them",
        "a spread option offense: shotgun, the zone read and RPOs, with the quarterback a real run threat",
    ][slot]
}

fn front_words(slot: usize) -> &'static str {
    ["a one-gap 4-3 front that gets upfield", "a two-gap 3-4 front with the outside linebackers rushing the passer"]
        [slot]
}

fn shell_words(slot: usize) -> &'static str {
    [
        "single-high Cover 3 with a safety down in the box",
        "two-high quarters, the safeties reading the run",
        "man coverage and pressure, Cover 1 and Cover 0",
        "Cover 2 and Tampa 2, everything kept in front",
    ][slot]
}

/// A system's short name, by its slot in the fit arrays.
pub fn short_name(slot: usize) -> &'static str {
    [
        "West Coast",
        "air raid",
        "wide zone",
        "power gap",
        "spread option",
        "4-3",
        "3-4",
        "Cover 3",
        "quarters",
        "man pressure",
        "Cover 2",
    ][slot]
}

/// The best and second-best slots he knows in a kind.
fn best_two(know: &[f64; 11], kind: SysKind) -> (usize, Option<usize>) {
    let mut v: Vec<usize> = kind.range().collect();
    v.sort_by(|&a, &b| know[b].total_cmp(&know[a]).then(a.cmp(&b)));
    (v[0], v.get(1).copied())
}

/// How the roster fits his system against the rest of its kind, in points
/// a game (`systems::roster_fit`): fits it, a stretch, or about even; and
/// the system it fits clearly better, if one does. Proposed, not confirmed:
/// 0.4 points either way is a clear fit or a stretch, 0.8 better a clearly
/// better fit.
fn fit_words(fit: &[f64; 11], kind: SysKind, slot: usize, here: bool) -> String {
    let r = kind.range();
    let n = r.len() as f64;
    let mean = r.clone().map(|s| fit[s]).sum::<f64>() / n;
    let top = r.clone().max_by(|&a, &b| fit[a].total_cmp(&fit[b]).then(b.cmp(&a))).unwrap_or(slot);
    let whose = if here { "the roster here" } else { "his roster" };
    let d = fit[slot] - mean;
    let mut s = if d >= 0.4 {
        format!("{whose} fits it")
    } else if d <= -0.4 {
        format!("{whose} is a stretch for it, and he'd recruit to it")
    } else {
        format!("{whose} fits it about as well as anything")
    };
    if top != slot && fit[top] - fit[slot] >= 0.8 {
        s += &format!(", though he'd grant it fits the {} better", short_name(top));
    }
    s
}

/// How deep he knows a system, in words.
fn depth_words(k: f64) -> &'static str {
    if k >= 0.85 {
        "he has run it for years and knows it inside out"
    } else if k >= 0.6 {
        "he knows it well"
    } else {
        "he isn't married to one system; it's the one he knows best"
    }
}

/// His system: what he runs or would run, how deep he knows it, how the
/// roster fits it and what the team runs now.
pub fn scheme(w: &World, him: Them) -> Vec<Known> {
    let Them::Coach(c) = him else { return Vec::new() };
    let Some(co) = w.coaches.get(c) else { return Vec::new() };
    let know = co.system_know();
    let (off, def) = sides(w, c);
    let team = roster_team(w, c);
    let here = team.is_some() && team == Team::seat(w);
    let roster = team.map(|t| t.roster(w)).unwrap_or_default();
    let fit = (!roster.is_empty()).then(|| systems::roster_fit(w, &roster));
    let installed = match team {
        Some(Team::College(p)) => Some(w.programs[p].installed),
        _ => None,
    };
    let mut v = Vec::new();
    if !off && !def {
        v.push(known(
            Topic::Scheme,
            "he coaches the kicking game, and he fits his units to whatever the coordinators run".into(),
        ));
        return v;
    }
    if off {
        let (best, second) = best_two(&know, SysKind::Offense);
        let mut s = format!("he runs {}; {}", offense_words(best), depth_words(know[best]));
        if let Some(sec) = second.filter(|&x| know[x] >= 0.5 && know[best] - know[x] <= 0.1) {
            s += &format!(", and he's comfortable in the {} too", short_name(sec));
        }
        let lean = crate::world::systems::OFFENSES[best].pass_lean();
        if lean > 0.05 {
            s += "; it throws more than most";
        } else if lean < -0.03 {
            s += "; it runs more than most";
        }
        v.push(known(Topic::Scheme, s));
        let mut f = Vec::new();
        if let Some(fit) = &fit {
            f.push(fit_words(fit, SysKind::Offense, best, here));
        }
        if let Some(inst) = installed {
            f.push(if inst.off == best {
                "it's what the team already runs".to_string()
            } else {
                format!(
                    "the team runs the {} now, and a switch costs the players a year of learning",
                    short_name(inst.off)
                )
            });
        }
        if !f.is_empty() {
            v.push(known(Topic::Scheme, f.join("; ")));
        }
    }
    if def {
        let (front, _) = best_two(&know, SysKind::Front);
        let (shell, _) = best_two(&know, SysKind::Shell);
        let fs = front - SysKind::Front.range().start;
        let ss = shell - SysKind::Shell.range().start;
        v.push(known(
            Topic::Scheme,
            format!(
                "on defense he runs {} behind {}; {}",
                front_words(fs),
                shell_words(ss),
                depth_words(know[front].min(know[shell]))
            ),
        ));
        let mut f = Vec::new();
        if let Some(fit) = &fit {
            f.push(fit_words(fit, SysKind::Front, front, here));
        }
        if let Some(inst) = installed {
            let same = inst.front == fs && inst.shell == ss;
            f.push(if same {
                "it's what the defense already runs".to_string()
            } else {
                format!(
                    "the defense runs the {} with {} now, and a switch costs the players a year of learning",
                    short_name(SysKind::Front.range().start + inst.front),
                    short_name(SysKind::Shell.range().start + inst.shell)
                )
            });
        }
        if !f.is_empty() {
            v.push(known(Topic::Scheme, format!("on defense {}", f.join("; "))));
        }
    }
    v
}

/// The college program a coach recruits for.
fn program_of(w: &World, c: usize) -> Option<usize> {
    match makeup::coach_team(w, c)? {
        Team::College(p) => Some(p),
        _ => None,
    }
}

/// The states whose recruiting is his for his program.
pub fn area(w: &World, c: usize) -> Vec<usize> {
    let Some(p) = program_of(w, c) else { return Vec::new() };
    (0..w.states.len().min(51))
        .filter(|&st| crate::world::scouting::dept::area_coach(w, p, st as u8) == Some(c))
        .collect()
}

/// The recruits he's working for his program, best first: this class's
/// high schoolers in his area that the program offered, hosted or holds a
/// commitment from (`dept::recruiter_for`); without any, the best of the
/// class in his area, the ones he'd be on first.
pub fn board(w: &World, c: usize) -> Vec<usize> {
    let Some(p) = program_of(w, c) else { return Vec::new() };
    let class = crate::world::front_office::college::next_class(w);
    let states = area(w, c);
    let mut ours = Vec::new();
    let mut open = Vec::new();
    for (i, x) in w.people.iter().enumerate() {
        if !matches!(x.stage, crate::world::Stage::HighSchool | crate::world::Stage::PrepSchool)
            || x.class_of != class
            || x.cal.signed.is_some_and(|s| s.program != p)
            || x.stars < 2
        {
            continue;
        }
        let after = x.cal.offers.contains(&p)
            || x.cal.early_offers.iter().any(|o| o.0 == p)
            || x.cal.visits.iter().any(|v| v.0 == p)
            || x.cal.commit.is_some_and(|k| k.0 == p)
            || x.cal.signed.is_some_and(|s| s.program == p);
        if after {
            if crate::world::scouting::dept::recruiter_for(w, p, i) == Some(c) {
                ours.push(i);
            }
        } else if states.contains(&x.home_state) && x.stars >= 3 {
            open.push(i);
        }
    }
    let key = |i: &usize| {
        let x = &w.people[*i];
        (std::cmp::Reverse(x.stars), std::cmp::Reverse((x.cal.service_read.unwrap_or(0.0) * 100.0) as i64), x.player.id)
    };
    ours.sort_by_key(key);
    open.sort_by_key(key);
    if ours.is_empty() { open } else { ours }
}

/// The recruits who signed with his program through him over the last
/// `CLASSES` classes, best first: on the roster now (this class's early
/// enrollees too) or signed and coming.
pub fn signed_through(w: &World, c: usize) -> Vec<usize> {
    let Some(p) = program_of(w, c) else { return Vec::new() };
    let class = crate::world::front_office::college::next_class(w);
    let first = class - CLASSES;
    let mut v: Vec<usize> = Vec::new();
    for &i in &w.programs[p].roster {
        let x = &w.people[i];
        if x.class_of >= first && x.class_of <= class && x.transfers == 0 && !x.juco {
            v.push(i);
        }
    }
    for (i, x) in w.people.iter().enumerate() {
        if x.class_of >= first
            && x.class_of <= class
            && x.cal.signed.is_some_and(|s| s.program == p && !s.walk_on)
            && !v.contains(&i)
            && matches!(x.stage, crate::world::Stage::HighSchool | crate::world::Stage::PrepSchool)
        {
            v.push(i);
        }
    }
    v.retain(|&i| crate::world::scouting::dept::recruiter_for(w, p, i) == Some(c));
    v.sort_by_key(|&i| (std::cmp::Reverse(w.people[i].stars), w.people[i].player.id));
    v
}

/// His recruiting record as one number, for who speaks up in a staff room:
/// a point a star of each signing, half a point a star of each recruit he's
/// working. Proposed, not confirmed.
pub fn record_score(w: &World, c: usize) -> f64 {
    let signed: f64 = signed_through(w, c).iter().map(|&i| f64::from(w.people[i].stars)).sum();
    let working: f64 = board(w, c).iter().take(NAMED).map(|&i| f64::from(w.people[i].stars)).sum();
    signed + 0.5 * working
}

fn recruit_words(w: &World, i: usize, p: usize) -> String {
    let x = &w.people[i];
    let state = w.states.get(x.home_state).map_or("", |s| facts::state_name(&s.code));
    let mut s = format!(
        "{}, a {}-star {} at {} ({state})",
        x.name,
        x.stars,
        crate::world::folk::claims::words::spot(x.player.position),
        w.school_name(x.hs)
    );
    let ours = Team::College(p).name(w);
    if x.cal.signed.is_some_and(|k| k.program == p) {
        s += &format!(", signed with {ours}");
    } else if let Some((q, _, _)) = x.cal.commit {
        s += &format!(", committed to {}", Team::College(q).name(w));
    } else if let Some(&(q, _)) = crate::world::front_office::college::race_at(w, i).first() {
        if q == p {
            s += &format!(", leaning {ours}");
        } else {
            s += &format!(", leaning {}", Team::College(q).name(w));
        }
    } else {
        s += ", still wide open";
    }
    s
}

/// His recruiting: his area, the recruits he's working by name, and the
/// ones who signed through him. An NFL or a high school coach doesn't
/// recruit, and says so.
pub fn recruiting(w: &World, him: Them) -> Vec<Known> {
    let Them::Coach(c) = him else { return Vec::new() };
    if w.coaches.get(c).is_none() {
        return Vec::new();
    }
    let Some(p) = program_of(w, c) else {
        let words = match makeup::coach_team(w, c) {
            Some(Team::High(_)) => "he doesn't recruit: his players come up through his school and its feeders",
            Some(Team::Nfl(_) | Team::Other(_)) => {
                "he doesn't recruit: in the pros the scouts and the draft find the players"
            }
            _ => "he isn't recruiting for anyone right now",
        };
        return vec![known(Topic::Recruiting, words.into())];
    };
    let mut v = Vec::new();
    let states: Vec<&str> =
        area(w, c).into_iter().filter_map(|s| w.states.get(s).map(|x| facts::state_name(&x.code))).collect();
    v.push(known(
        Topic::Recruiting,
        if states.is_empty() {
            "he has no recruiting area of his own; he helps where the staff needs him".into()
        } else {
            format!("he recruits {}", list(&states))
        },
    ));
    let b: Vec<String> = board(w, c).into_iter().take(NAMED).map(|i| recruit_words(w, i, p)).collect();
    if !b.is_empty() {
        v.push(known(Topic::Recruiting, format!("the best of the class he's working: {}", b.join("; "))));
    }
    let s = signed_through(w, c);
    if s.is_empty() {
        v.push(known(Topic::Recruiting, "nobody from his area signed in the last two classes".into()));
    } else {
        let top: Vec<String> = s
            .iter()
            .take(NAMED)
            .map(|&i| {
                let x = &w.people[i];
                format!("{} ({}-star {})", x.name, x.stars, crate::world::folk::claims::words::spot(x.player.position))
            })
            .collect();
        v.push(known(
            Topic::Recruiting,
            format!(
                "{} signed through him in the last two classes, the best of them {}",
                count_words(s.len()),
                list(&top.iter().map(|x| x.as_str()).collect::<Vec<_>>())
            ),
        ));
    }
    v
}

/// The people he can name when he talks about his recruiting: his board and
/// his signings.
pub fn named(w: &World, c: usize) -> Vec<usize> {
    let mut v: Vec<usize> = board(w, c).into_iter().take(NAMED).collect();
    for i in signed_through(w, c).into_iter().take(NAMED) {
        if !v.contains(&i) {
            v.push(i);
        }
    }
    v
}

fn count_words(n: usize) -> String {
    match n {
        1 => "one recruit".into(),
        2 => "two recruits".into(),
        3 => "three recruits".into(),
        4 => "four recruits".into(),
        5 => "five recruits".into(),
        n => format!("{n} recruits"),
    }
}

fn list(v: &[&str]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].to_string(),
        2 => format!("{} and {}", v[0], v[1]),
        n => format!("{}, and {}", v[..n - 1].join(", "), v[n - 1]),
    }
}

/// Your staff's read of one of his facets, in words: the range the staff
/// screen shows, said plainly. Proposed, not confirmed: a range 9 or wider
/// is "still hard to read"; its middle 16 and up "a real strength", 13
/// "good", 9 "about average", 6 "below average", lower "a weak spot" (the
/// interview's words, `live_talk::say::hiring::read_words`).
pub fn read_words(w: &World, c: usize, f: usize) -> &'static str {
    let (lo, hi) = coach_look::read(w, c, f);
    let mid = (f32::from(lo) + f32::from(hi)) / 2.0;
    if hi.saturating_sub(lo) >= 9 {
        "still hard to read"
    } else if mid >= 16.0 {
        "a real strength"
    } else if mid >= 13.0 {
        "good"
    } else if mid >= 9.0 {
        "about average"
    } else if mid >= 6.0 {
        "below average"
    } else {
        "a weak spot"
    }
}

/// The scene's note under a coach's answer on his scheme or his
/// recruiting: what your staff makes of it, for you, never in his mouth.
pub fn read_note(w: &World, c: usize, topic: Topic) -> Option<String> {
    let name = &w.coaches.get(c)?.name;
    match topic {
        Topic::Scheme => Some(format!("Your staff's read of {name}'s scheme: {}.", read_words(w, c, 8))),
        Topic::Recruiting => Some(format!(
            "Your staff's read of {name}'s recruiting: {}; his network: {}.",
            read_words(w, c, 14),
            read_words(w, c, 15)
        )),
        _ => None,
    }
}

/// The holdover staff in their meeting about the whole room: the position
/// coaches still on your staff from the last one (at a high school, every
/// assistant held over).
pub fn holdover_room(w: &World) -> Option<Room> {
    use super::room::{Present, Seat};
    let team = Team::seat(w)?;
    let present: Vec<Present> = facts::assistants(w, team)
        .into_iter()
        .filter(|(c, role)| {
            (matches!(role, Role::PositionCoach(_)) || matches!(team, Team::High(_)))
                && super::talk::stakes::holdover(w, *c).is_some()
        })
        .map(|(c, _)| Present { who: Them::Coach(c), seat: Seat::Listener, speaks: false })
        .collect();
    (!present.is_empty()).then_some(Room { channel: Channel::Group(RoomGroup::Staff), present })
}

/// Who in a staff room answers a line: the coach a question names, else
/// for a question on recruiting the man with the best record, for one on
/// the scheme or the unit the man who knows his system best. None leaves
/// the room as it sits (Cain 2026-11-17: one nameless "he" answered "who
/// here can get me the best recruits?").
pub fn answerer(w: &World, room: &Room, reading: &super::ear::Reading) -> Option<Them> {
    if !matches!(room.channel, Channel::Group(RoomGroup::Staff)) {
        return None;
    }
    let coaches: Vec<usize> = room
        .present
        .iter()
        .filter_map(|p| match p.who {
            Them::Coach(c) if w.coaches.get(c).is_some() => Some(c),
            _ => None,
        })
        .collect();
    for a in &reading.acts {
        let super::ear::Act::Question { topic, about, .. } = a else { continue };
        if let Some(Them::Coach(x)) = about
            && coaches.contains(x)
        {
            return Some(Them::Coach(*x));
        }
        let best = |f: &dyn Fn(usize) -> f64| {
            coaches.iter().copied().max_by(|&a, &b| f(a).total_cmp(&f(b)).then(b.cmp(&a))).map(Them::Coach)
        };
        match topic {
            Topic::Recruiting | Topic::Pitches => return best(&|c| record_score(w, c)),
            Topic::Scheme | Topic::Football | Topic::Opponent => {
                return best(&|c| w.coaches[c].system_know().iter().copied().fold(0.0, f64::max));
            }
            _ => {}
        }
    }
    None
}

/// The room with one man moved to the front, where the talk reads the man
/// who answers.
pub fn lead_with(mut room: Room, who: Them) -> Room {
    if let Some(k) = room.present.iter().position(|p| p.who == who) {
        let p = room.present.remove(k);
        room.present.insert(0, p);
    }
    room
}
