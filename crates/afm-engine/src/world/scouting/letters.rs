//! Scout reports as letters (scouting update SK41): after each trip or
//! assignment, whoever went writes to you in his own voice the day his
//! write-up lands: what he saw, who he talked to, how sure he is and his
//! gut call against the room, each player linked to his file.
//!
//! A trip is a look in person or a call around a player (a practice, a
//! game, a camp, a school call, a workout, a visit, a home visit, a call to
//! his coaches); film from the office writes no letter. One letter for each
//! scout a day, three players at most in it, the best first (proposed, not
//! confirmed: letters from up to four people a day).

use super::Kind;
use super::file::{self, Link, Speak};
use super::lines::{self, Voice};
use super::readers::{self, Staff};
use crate::calendar::Date;
use crate::world::desk::Head;
use crate::world::{Stage, World};

/// A trip writes a letter; film and measurements don't.
fn trip(k: Kind) -> bool {
    matches!(
        k,
        Kind::School
            | Kind::CrossCheck
            | Kind::AllStar
            | Kind::Interview
            | Kind::ProDay
            | Kind::Visit
            | Kind::Workout
            | Kind::LocalDay
            | Kind::Practice
            | Kind::Game
            | Kind::Camp
            | Kind::JuniorDay
            | Kind::Official
            | Kind::Unofficial
            | Kind::HomeVisit
            | Kind::Call
            | Kind::OldStaff
            | Kind::Tryout
    )
}

/// Who he talked to on the trip.
fn talked(k: Kind) -> &'static str {
    match k {
        Kind::School | Kind::Practice | Kind::Game => "his head coach, the strength coach and the trainer",
        Kind::Call | Kind::OldStaff => "his coaches, a teacher and his counselor, by phone",
        Kind::Camp | Kind::JuniorDay => "the camp staff and the kid himself",
        Kind::Visit | Kind::Official | Kind::Unofficial | Kind::HomeVisit => "him and his family",
        Kind::Interview => "him, across the table",
        Kind::ProDay | Kind::Workout | Kind::Tryout => "his position coach and the people running the workout",
        Kind::AllStar => "the all-star staff who coached him all week",
        Kind::LocalDay => "him and his agent",
        _ => "the people around him",
    }
}

/// The day's letters.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(staff) = readers::seat_staff(w) else { return };
    if matches!(staff, Staff::School(_)) {
        return;
    }
    let you = readers::you(w);
    // (reader code, player id, kind, date) of the recent trips: your
    // department's looks, and at a club its scouts' own files.
    let mut due: Vec<(u32, u32, Kind, Date)> = Vec::new();
    let since = d.add(-22);
    for (&id, looks) in w.scouting.dept.looks.iter() {
        due.extend(
            looks.iter().filter(|l| l.date >= since && l.date < d && trip(l.kind)).map(|l| (l.who, id, l.kind, l.date)),
        );
    }
    if let Staff::Club(t) = staff
        && let Some(c) = w.clubs.get(t)
    {
        for (&id, looks) in c.files.iter() {
            for l in looks.iter().filter(|l| l.date >= since && l.date < d) {
                let kind = Kind::from_nfl(l.kind);
                if trip(kind) {
                    due.push((l.who as u32, id, kind, l.date));
                }
            }
        }
    }
    due.retain(|x| !you.is_some_and(|c| x.0 == readers::COACH + c as u32));
    if due.is_empty() {
        return;
    }
    due.sort_by_key(|x| (x.0, x.1, x.3));
    // Keep the ones landing today: the reader's own lag on that look.
    let mut by_reader: Vec<(u32, Vec<(u32, Kind)>)> = Vec::new();
    for (who, id, kind, date) in due {
        let Some(i) = super::place(w, id) else { continue };
        let g = crate::world::coach::Group::of(w.people[i].player.position) as usize;
        let Some(r) = readers::reader(w, staff, who, g) else { continue };
        if r.link == Link::You || date.add(r.lag(kind, id, date)) != d {
            continue;
        }
        if by_reader.iter().any(|x| x.0 == who && x.1.iter().any(|y| y.0 == id)) {
            continue;
        }
        match by_reader.iter_mut().find(|x| x.0 == who) {
            Some(x) => x.1.push((id, kind)),
            None => by_reader.push((who, vec![(id, kind)])),
        }
    }
    for (who, players) in by_reader.into_iter().take(4) {
        write(w, staff, who, &players);
    }
}

fn write(w: &mut World, staff: Staff, who: u32, players: &[(u32, Kind)]) {
    let mut body: Vec<String> = Vec::new();
    let mut ids: Vec<u32> = Vec::new();
    let mut name = String::new();
    let mut job = String::new();
    let mut parts: Vec<(f64, String, u32)> = Vec::new();
    for &(id, kind) in players {
        let Some(i) = super::place(w, id) else { continue };
        let (rds, looks, codes) = readers::evidence(w, staff, id);
        let Some(r) = codes.iter().position(|&c| c == who) else { continue };
        let f = file::read(w, i, &rds, &looks, &readers::norms(w, i), readers::medical_rating(w, staff), Some(w.today));
        name = rds[r].name.clone();
        job = rds[r].job.clone();
        let p = &w.people[i];
        let pos = p.player.position;
        let mine = &f.per_reader[r];
        // What he saw: his own read against the room's, weighed by the spot.
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
            continue;
        }
        seen.sort_by(|a, b| b.0.total_cmp(&a.0));
        let best = lines::attr_phrase(seen[0].1, true);
        let worst = lines::attr_phrase(seen[seen.len() - 1].1, false);
        let centre = f.now.0 + if den > 0.0 { num / den } else { 0.0 };
        let half = (f.now.1 * 1.4).max(2.0);
        let read = super::yours::range((centre, half));
        let speak = match p.stage {
            Stage::HighSchool | Stage::PrepSchool => Speak::Recruit,
            Stage::Pro => Speak::Pro,
            _ => Speak::Prospect,
        };
        let looks = w.scouting.dept.looks.get(&id).map_or(0, |v| v.iter().filter(|l| l.who == who).count());
        let v = Voice {
            name: &p.name,
            read,
            room: f.now.0,
            looks,
            tier: file::tier_words(speak, f.ceiling.0),
            fit: None,
            numbers: false,
        };
        let gut = lines::voice(&v, rds[r].key, id as u64);
        let link = crate::world::folk::player_link(w, i);
        let school = match p.stage {
            Stage::College | Stage::Declared => p.college.map(|c| w.program_name(c).to_string()),
            _ => w.high_schools.get(p.hs).map(|_| w.school_name(p.hs).to_string()),
        };
        let text = format!(
            "{link} ({pos}{}): {}. I talked to {}. What I saw: {best}, but {worst}. I'm {} on him. {gut}",
            school.map_or(String::new(), |s| format!(", {s}")),
            kind.name(),
            talked(kind),
            file::sure_words(f.now.1),
        );
        parts.push((centre, text, id));
    }
    if parts.is_empty() {
        return;
    }
    parts.sort_by(|a, b| b.0.total_cmp(&a.0));
    let n = parts.len().min(3);
    let first = name.split_whitespace().next().unwrap_or("").to_string();
    body.push(if n == 1 {
        "Back from the trip. Here's what I've got.".into()
    } else {
        format!("Back from the road: {n} kids worth your time.")
    });
    for (_, t, id) in parts.into_iter().take(3) {
        body.push(t);
        ids.push(id);
    }
    body.push(format!("It's all in the file. {first}"));
    let nfl = matches!(staff, Staff::Club(_));
    let subject = if n == 1 { "From the road".to_string() } else { format!("From the road: {n} reports") };
    let mut m = crate::world::storylines::credit::mail(
        w,
        &format!("l7letter{who}:{}", ids[0]),
        Head::Email,
        name.clone(),
        crate::world::folk::takeover::cap_first(&job),
        subject,
    );
    m.body = body;
    m.sign = name;
    m.keys = if nfl { 1 << 7 } else { 1 << 2 };
    m.about = Some((0, ids[0]));
    m.ids = ids;
    m.minute = 17 * 60 + (who % 90) as u16;
    crate::world::storylines::credit::send(w, m);
}
