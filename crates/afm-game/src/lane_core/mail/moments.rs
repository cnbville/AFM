//! The big-moment reveals (screens topic 9, Cain's yes 2026-11-14): the
//! full-screen show for the few things that are really big. A recruit's
//! commitment and the one who chose another school, the job you are given
//! and the job you lose, a final won, a record that falls, a Hall of Fame
//! class and a player you wrote down at 14 to 16 making his first NFL
//! start. Nothing here raises a world event or changes a number: it reads
//! the lines and books the world already keeps, and the words are the
//! world's own (the why-lines of a commitment, the old holder of a record,
//! the years you were there). The firing's reveal is made where the job
//! ends (`front_office::market::left`), because by the next day you have
//! no team to write for.
//!
//! What gets one (Cain, 2026-11-14): recruits you are working (an offer
//! out, not a walk-on), finals (state, national, league, the Super Bowl),
//! school, conference, level and league records for a season or a career,
//! Hall of Fame classes only for men who were on your roster or staff, and
//! first starts only when your notes have a line on him from 14 to 16.

use afm_engine::calendar::{Date, Kind as EvKind, Level as CalLevel, Link};
use afm_engine::world::World;
use afm_engine::world::coach::Employer;
use afm_engine::world::desk::Reveal;
use afm_engine::world::marks::Tier;
use afm_engine::world::records::{Book, RECORD_STATS, Scope};

use super::{ABOUT_PLAYER, ABOUT_TEAM, Pen, expectation, team_code};
use crate::views::{self, TeamRef};

/// A team and its tier, for the record books.
fn tier_of(t: TeamRef) -> Option<(Tier, usize)> {
    match t {
        TeamRef::College(i) => Some((Tier::College, i)),
        TeamRef::Nfl(i) => Some((Tier::Nfl, i)),
        TeamRef::HighSchool(i) => Some((Tier::Hs, i)),
        TeamRef::Other(_) => None,
    }
}

/// 1st, 2nd, 3rd, 4th.
pub(super) fn ord(n: u32) -> String {
    let t = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{t}")
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// "You're hired": the job you were given, in the boss's expectation.
pub(super) fn hire(w: &World, me: TeamRef, d: Date) -> Reveal {
    let team = views::team_name(w, me);
    let role = w.career.as_ref().and_then(|c| c.role).map_or("Head coach", |r| r.name());
    let (rec, note) = views::season_record(w, me);
    let mut facts = vec![("Last season".to_string(), if note.is_empty() { rec } else { format!("{rec} ({note})") })];
    let roster = views::roster_people(w, me).len();
    if roster > 0 {
        facts.push(("On the roster".into(), roster.to_string()));
    }
    Reveal {
        date: d,
        kind: "hire".into(),
        kicker: "The job is yours".into(),
        number: d.year().to_string(),
        name: team,
        sub: format!("{} · {}", capital(role), views::team_line(w, me)),
        verdict: expectation(w, me),
        about: Some((ABOUT_TEAM, team_code(me))),
        colors: views::team_colors(w, me),
        token: String::new(),
        facts,
        ..Default::default()
    }
}

/// A recruit commits to you (a college seat, `mine` your program) or signs
/// in your gym (a high school seat: `hs`, the program he signs with): his
/// finalists' caps on the table, then his name, his number of stars, your
/// staff's read in words and why he chose the school.
pub(super) fn commit(w: &World, me: TeamRef, k: usize, mine: usize, d: Date, hs: bool) -> Reveal {
    let p = &w.people[k];
    let prog = if hs { p.cal.signed.map_or(mine, |s| s.program) } else { mine };
    let others: Vec<usize> = views::offers_of(p).into_iter().filter(|&o| o != prog).take(3).collect();
    let mut hats: Vec<(String, [String; 2])> =
        others.iter().map(|&o| (w.program_name(o).to_string(), views::team_colors(w, TeamRef::College(o)))).collect();
    let chosen = if hats.is_empty() { 0 } else { p.player.id as usize % (hats.len() + 1) };
    hats.insert(chosen, (w.program_name(prog).to_string(), views::team_colors(w, TeamRef::College(prog))));
    if hats.len() < 2 {
        hats.clear();
    }
    let tier = crate::reads::potential_words(w, k)["tier"].as_str().map(str::to_string);
    let why: Vec<String> = crate::lane_help::pick_why(w, k, prog, None).into_iter().take(3).collect();
    let school = w.program_name(prog).to_string();
    Reveal {
        date: d,
        kind: if hs { "signing" } else { "commit" }.into(),
        kicker: if hs { format!("He signs with {school}") } else { format!("He chose {school}") },
        number: if p.stars > 0 { format!("{}\u{2605}", p.stars) } else { String::new() },
        name: p.name.clone(),
        sub: format!("{} \u{b7} {} \u{b7} {}", p.player.position.name(), views::hometown(w, p), w.school_name(p.hs)),
        verdict: match (&tier, others.len()) {
            (Some(t), 0) => format!("Your staff: {t}."),
            (Some(t), n) => format!(
                "Your staff: {t}. He picks {school} over {n} other {}.",
                if n == 1 { "offer" } else { "offers" }
            ),
            (None, 0) => format!("He picks {school}."),
            (None, n) => format!("He picks {school} over {n} other {}.", if n == 1 { "offer" } else { "offers" }),
        },
        token: p.player.position.name().to_string(),
        about: Some((ABOUT_PLAYER, p.player.id)),
        colors: views::team_colors(w, TeamRef::College(prog)),
        hats,
        chosen,
        room: if hs { views::team_name(w, me) } else { w.school_name(p.hs).to_string() },
        cards: if why.is_empty() { Vec::new() } else { vec![("Why he chose it".into(), why)] },
        facts: vec![("Class of".into(), p.class_of.to_string())],
        ..Default::default()
    }
}

/// A recruit on your board chose another school: the quiet reveal, with the
/// why-lines of the lost race.
pub(super) fn miss(w: &World, k: usize, chosen: usize, mine: usize, d: Date) -> Reveal {
    let p = &w.people[k];
    let why: Vec<String> = crate::lane_help::pick_why(w, k, chosen, Some(mine)).into_iter().take(3).collect();
    let school = w.program_name(chosen).to_string();
    Reveal {
        date: d,
        kind: "miss".into(),
        kicker: "He chose another school".into(),
        name: p.name.clone(),
        sub: format!("{} \u{b7} {} \u{b7} {}", p.player.position.name(), views::hometown(w, p), w.school_name(p.hs)),
        verdict: format!("{school} won him. You were in the race."),
        about: Some((ABOUT_PLAYER, p.player.id)),
        colors: views::team_colors(w, TeamRef::College(chosen)),
        quiet: true,
        cards: if why.is_empty() { Vec::new() } else { vec![("What decided it".into(), why)] },
        facts: vec![("He chose".into(), school), ("Class of".into(), p.class_of.to_string())],
        ..Default::default()
    }
}

/// The reveals the world's lines and books give for these days.
pub(super) fn gather(pen: &Pen, from: Date, to: Date, out: &mut Vec<Reveal>) {
    let w = pen.w;
    let me = pen.me;
    if let Some(c) = w.career.as_ref() {
        // A new job: the first one has its welcome, a later one comes here.
        if c.desk.hired_for.is_some_and(|t| t != team_code(me)) {
            out.push(hire(w, me, w.today));
        }
    }
    records(pen, from, to, out);
    titles(pen, from, to, out);
    hall(pen, from, to, out);
    first_starts(pen, out);
}

/// Where a record book applies, in a few words.
fn scope_words(w: &World, f: &afm_engine::world::marks::Fell) -> String {
    match (&f.scope, f.team) {
        (Some(Scope::Division(d)), _) => format!("{} record", d.name()),
        (Some(Scope::Conference(_, c)), _) => format!("{} record", w.conference_name(c)),
        (Some(Scope::State(i)), _) => {
            format!(
                "{} record",
                w.states.get(*i).map_or_else(|| "State".into(), |x| views::state_name(&x.code).to_string())
            )
        }
        (Some(Scope::Nfl), _) => "NFL record".into(),
        (None, _) => match f.tier() {
            Tier::Nfl => "Club record".into(),
            _ => "School record".into(),
        },
    }
}

/// How wide a book's reach is: the wider, the bigger the moment.
fn reach(f: &afm_engine::world::marks::Fell) -> u8 {
    match &f.scope {
        Some(Scope::Nfl) | Some(Scope::Division(_)) => 0,
        Some(Scope::Conference(..)) | Some(Scope::State(_)) => 1,
        None => 2,
    }
}

/// A record falls: one reveal a player a day, the widest book first, and
/// at most three in one stretch.
fn records(pen: &Pen, from: Date, to: Date, out: &mut Vec<Reveal>) {
    let w = pen.w;
    let Some((tier, idx)) = tier_of(pen.me) else { return };
    let mut best: Vec<&afm_engine::world::marks::Fell> = Vec::new();
    for f in w.marks.fell.iter().filter(|f| f.date >= from && f.date <= to) {
        if !matches!(f.book, Book::Season(_) | Book::Career(_)) || f.tier() != tier || f.program != idx {
            continue;
        }
        // A level's book or your own team's: both count.
        let mine = match (&f.scope, f.team) {
            (Some(_), _) => true,
            (None, Some((t, i))) => t == tier && i as usize == idx,
            _ => false,
        };
        if !mine || f.old_id == f.id {
            continue;
        }
        match best.iter().position(|b| b.id == f.id && b.date == f.date) {
            Some(k) => {
                if reach(f) < reach(best[k]) {
                    best[k] = f;
                }
            }
            None => best.push(f),
        }
    }
    best.sort_by_key(|f| (reach(f), std::cmp::Reverse(f.date)));
    for f in best.into_iter().take(3) {
        let (Book::Season(k) | Book::Career(k)) = f.book else { continue };
        let stat = RECORD_STATS[k];
        let new = crate::lane_data::fmt_num(f.value);
        let old = crate::lane_data::fmt_num(f.old_value);
        let who_old = if f.old_name.is_empty() { "a former player".to_string() } else { f.old_name.clone() };
        let ratio = if f.value > 0.0 { (f.old_value / f.value).clamp(0.05, 1.0) as f32 } else { 1.0 };
        let held = (f.year - f.old_year).max(0);
        let (_, pos) = views::person_index(w, f.id).map_or((0, None), |i| (i, Some(w.people[i].player.position)));
        out.push(Reveal {
            date: f.date,
            kind: "record".into(),
            kicker: format!("A record falls · {}", scope_words(w, f)),
            number: new.clone(),
            name: f.name.clone(),
            sub: format!("{} {}", capital(&f.book.name()), pos.map_or(String::new(), |p| format!("· {}", p.name()))),
            verdict: format!(
                "He passes {who_old}'s {old}, set in {}{}.",
                f.old_year,
                if held >= 2 { format!(" and held for {held} years") } else { String::new() }
            ),
            token: if held >= 2 { format!("{held} yrs") } else { String::new() },
            about: Some((ABOUT_PLAYER, f.id)),
            colors: views::team_colors(w, pen.me),
            bars: vec![
                (format!("{who_old}, {}", f.old_year), old, ratio),
                (format!("{}, {}", f.name, f.year), new, 1.0),
            ],
            also: Some(("Open the record book".into(), 9, 0)),
            facts: vec![("Stat".into(), stat.name().to_string())],
            ..Default::default()
        });
    }
}

/// A final won: the state title of your high school, the Super Bowl of your
/// club. (A college's national title is made in `news_mail`, where its
/// story is.)
fn titles(pen: &Pen, from: Date, to: Date, out: &mut Vec<Reveal>) {
    let w = pen.w;
    match pen.me {
        TeamRef::Nfl(t) => {
            for (_, evs) in w.calendar.events.range(from..=to) {
                for e in evs {
                    if e.level == CalLevel::Nfl
                        && e.kind == EvKind::Game
                        && e.nfl_team == Some(t)
                        && e.text.contains("win the Super Bowl")
                    {
                        let d = e.when.date;
                        let (rec, _) = views::season_record(w, pen.me);
                        out.push(Reveal {
                            date: d,
                            kind: "title".into(),
                            kicker: "Super Bowl champions".into(),
                            number: d.year().to_string(),
                            name: views::team_name(w, pen.me),
                            sub: e.text.clone(),
                            verdict: String::new(),
                            about: Some((ABOUT_TEAM, team_code(pen.me))),
                            colors: views::team_colors(w, pen.me),
                            facts: vec![("Season".into(), rec)],
                            ..Default::default()
                        });
                    }
                }
            }
        }
        TeamRef::HighSchool(s) => {
            let Some(c) = w.career.as_ref() else { return };
            let Some(sea) = w.high_schools[s].seasons.last() else { return };
            // Only a title won with you in the chair, once.
            if !sea.state_title || sea.coach != Some(c.coach) || sea.year < from.year() - 1 {
                return;
            }
            let titles = w.high_schools[s].state_titles;
            let state = views::state_name(&w.states[w.high_schools[s].state].code);
            out.push(Reveal {
                date: w.today,
                kind: "title".into(),
                kicker: "State champions".into(),
                number: sea.year.to_string(),
                name: views::team_name(w, pen.me),
                sub: sea.title.clone().unwrap_or_else(|| format!("{state} {}", views::hs_class(w, s))),
                verdict: String::new(),
                token: ord(titles),
                about: Some((ABOUT_TEAM, team_code(pen.me))),
                colors: views::team_colors(w, pen.me),
                facts: vec![("Season".into(), format!("{}\u{2013}{}", sea.wins, sea.losses))],
                ..Default::default()
            });
        }
        _ => {}
    }
}

/// The years you and a coach (or a player) shared a team: the first and the
/// last season of the overlap, or None.
fn years_with_you(w: &World, employer: Employer, from: i32, to: i32) -> Option<(i32, i32)> {
    let c = w.career.as_ref()?;
    let me = &w.coaches[c.coach];
    let mut spans: Vec<(i32, i32)> =
        me.history.iter().filter(|s| s.employer == employer).map(|s| (s.from, s.to)).collect();
    if let Some(j) = me.job.filter(|j| j.employer == employer) {
        spans.push((j.since, w.year));
    }
    let mut got: Option<(i32, i32)> = None;
    for (a, b) in spans {
        let (lo, hi) = (a.max(from), b.min(to));
        if lo <= hi {
            got = Some(got.map_or((lo, hi), |(x, y)| (x.min(lo), y.max(hi))));
        }
    }
    got
}

/// A Hall of Fame class names a man who was on your roster or staff.
fn hall(pen: &Pen, from: Date, to: Date, out: &mut Vec<Reveal>) {
    let w = pen.w;
    let mut seen: Vec<u32> = Vec::new();
    for (&d, evs) in w.calendar.events.range(from..=to) {
        for e in evs {
            let pro = e.text.starts_with("Pro Football Hall of Fame class")
                || e.text.contains("Pro Football Hall of Fame elects");
            let college = e.text.contains("College Football Hall of Fame announces");
            if !pro && !college {
                continue;
            }
            let hall = if pro { "Pro Football Hall of Fame" } else { "College Football Hall of Fame" };
            for r in &e.refs {
                let (id, is_coach) = match r.to {
                    Link::Person(id) => (id, false),
                    Link::Coach(c) => (c, true),
                    _ => continue,
                };
                if seen.contains(&(id + if is_coach { 1 << 30 } else { 0 })) {
                    continue;
                }
                let name = e.text.get(r.at as usize..(r.at + r.len) as usize).unwrap_or("").to_string();
                let found = if is_coach { coach_with_you(w, id as usize) } else { player_with_you(w, id) };
                let Some((employer, a, b)) = found else { continue };
                seen.push(id + if is_coach { 1 << 30 } else { 0 });
                let team = match employer {
                    Employer::College(p) => w.program_name(p).to_string(),
                    Employer::Nfl(t) => w.nfl_team_name(t),
                    Employer::HighSchool(s) => w.school_name(s).to_string(),
                    Employer::Other(f) => afm_engine::world::pro::other::name(f, w.year).to_string(),
                };
                let (pos, why) = hall_line(w, id, is_coach, pro);
                let span = if a == b { a.to_string() } else { format!("{a} to {b}") };
                out.push(Reveal {
                    date: d,
                    kind: "hall".into(),
                    kicker: format!("{hall} · Class of {}", d.year()),
                    number: d.year().to_string(),
                    name,
                    sub: pos,
                    verdict: format!(
                        "{} you at {team}, {span}.",
                        if is_coach { "He worked with" } else { "He played for" }
                    ),
                    token: "HoF".into(),
                    about: Some((if is_coach { 1 } else { ABOUT_PLAYER }, id)),
                    colors: ["#8a6a1f".into(), "#e9c36a".into()],
                    cards: if why.is_empty() { Vec::new() } else { vec![("Why he is in".into(), vec![why])] },
                    ..Default::default()
                });
            }
        }
    }
}

/// His position and why he is in, from the Hall's own list.
fn hall_line(w: &World, id: u32, coach: bool, pro: bool) -> (String, String) {
    if coach {
        let why = w.stats.hall.iter().find(|h| h.coach && h.who == Some(id)).map(|h| h.why.clone()).unwrap_or_default();
        return ("Coach".into(), why);
    }
    if pro && let Some(h) = w.nfl_book.hall.iter().find(|h| h.1 == id) {
        return (h.3.to_string(), h.4.clone());
    }
    if let Some(h) = w.stats.hall.iter().find(|h| h.who == Some(id)) {
        let pos = w.stats.honors.get(&id).and_then(|x| x.pos).map_or(String::new(), |p| p.name().to_string());
        return (pos, h.why.clone());
    }
    (String::new(), String::new())
}

/// A coach you shared a team with: where and which years.
fn coach_with_you(w: &World, c: usize) -> Option<(Employer, i32, i32)> {
    let theirs = w.coaches.get(c)?;
    let mut stints: Vec<(Employer, i32, i32)> = theirs.history.iter().map(|s| (s.employer, s.from, s.to)).collect();
    if let Some(j) = theirs.job {
        stints.push((j.employer, j.since, w.year));
    }
    stints.into_iter().find_map(|(e, a, b)| years_with_you(w, e, a, b).map(|(x, y)| (e, x, y)))
}

/// A player you coached: at your college, or on your NFL club.
fn player_with_you(w: &World, id: u32) -> Option<(Employer, i32, i32)> {
    for r in w.stats.career(id) {
        if let Some((a, b)) = years_with_you(w, Employer::College(r.program as usize), r.year as i32, r.year as i32) {
            return Some((Employer::College(r.program as usize), a, b));
        }
    }
    let k = views::person_index(w, id)?;
    let pro = w.people[k].pro.as_ref()?;
    for s in &pro.seasons {
        if let Some(t) = s.team
            && let Some((a, b)) = years_with_you(w, Employer::Nfl(t), s.year, s.year)
        {
            return Some((Employer::Nfl(t), a, b));
        }
    }
    None
}

/// A player you wrote a line on at 14 to 16 starts his first NFL game.
fn first_starts(pen: &Pen, out: &mut Vec<Reveal>) {
    let w = pen.w;
    let Some(c) = w.career.as_ref() else { return };
    for (id, notes) in &c.notes {
        if c.desk.first_told.contains(id) {
            continue;
        }
        let Some(k) = views::person_index(w, *id) else { continue };
        let p = &w.people[k];
        let Some(pro) = p.pro.as_ref() else { continue };
        let starts: u32 = pro.seasons.iter().map(|s| s.starts_all()).sum();
        // Only the first few starts: a man who has started for years is
        // no news.
        if starts == 0 || starts > 4 {
            continue;
        }
        let early = notes.iter().filter(|n| {
            let age = afm_engine::calendar::born::age_on(p.born, n.date);
            (14..=16).contains(&age)
        });
        let Some(first) = early.min_by_key(|n| n.date) else { continue };
        let Some(club) = pro.seasons.iter().rev().find_map(|s| s.team).or(pro.team) else { continue };
        let age_at = |d: Date| afm_engine::calendar::born::age_on(p.born, d).to_string();
        let mut steps = vec![(age_at(first.date), "Your first note".to_string())];
        if let Some(r) = w.stats.career(*id).first() {
            steps.push((
                age_at(Date::ymd(r.year as i32, 9, 1)),
                format!("Played at {}", w.program_name(r.program as usize)),
            ));
        }
        if let Some((y, round, pick, _)) = pro.draft {
            steps.push((age_at(Date::ymd(y, 4, 25)), format!("Round {round}, pick {pick}")));
        }
        steps.push((age_at(w.today), "First start".to_string()));
        let note = first.text.trim();
        let note = if note.chars().count() > 160 {
            format!("{}…", note.chars().take(158).collect::<String>())
        } else {
            note.to_string()
        };
        out.push(Reveal {
            date: w.today,
            kind: "first".into(),
            kicker: "He's starting".into(),
            number: String::new(),
            name: p.name.clone(),
            sub: format!("{} · {}", p.player.position.name(), w.nfl_team_name(club)),
            verdict: String::new(),
            token: String::new(),
            about: Some((ABOUT_PLAYER, *id)),
            colors: views::team_colors(w, TeamRef::Nfl(club)),
            cards: vec![(
                format!("Your note, {}", views::date_text(first.date)),
                vec![format!("\u{201c}{note}\u{201d}")],
            )],
            steps,
            ..Default::default()
        });
    }
}
