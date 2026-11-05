//! Minor bug 265: the assignments your department screen offers do the
//! work. An NFL scout sent to a region calls on its schools and leaves
//! holes at home; one on cross-checks takes the top of your board; the
//! December fall keeps what your staff filed. A college coach sent to a
//! state, a position group or cross-checks spends his evaluation days
//! there, and a personnel staffer on a state studies its film instead of
//! his share of the board.

use std::collections::HashSet;

use crate::calendar::{Date, Period, Weekday};
use crate::world::data::Division;
use crate::world::pro::scouts::{self, LookKind, Role};
use crate::world::scouting::dept::{self, Assignment, Target};
use crate::world::scouting::readers::{COACH, PERSONNEL};
use crate::world::scouting::{Kind, readers};
use crate::world::seat::{Job, You};
use crate::world::{DraftRead, Stage, World, WorldOptions};

fn you() -> You {
    let teaching = vec![12.0; crate::world::coach::GROUP_COUNT];
    serde_json::from_value(serde_json::json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
    }))
    .unwrap()
}

fn empty() -> World {
    World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() })
}

/// A world with no history has no college players: pros are made into
/// seniors at the FBS programs, a few at each, with a December read.
fn seniors(w: &mut World, n: usize) {
    let fbs: Vec<usize> = (0..w.programs.len()).filter(|&c| w.programs[c].division == Division::FBS).collect();
    let class_of = w.year + 1 - 4;
    for (k, p) in w.people.iter_mut().filter(|p| p.stage == Stage::Pro).take(n).enumerate() {
        p.stage = Stage::College;
        p.pro = None;
        p.class_of = class_of;
        p.college = Some(fbs[k % fbs.len()]);
        let o = p.player.overall();
        p.cal.draft_read = Some(DraftRead { grade: o, score: o, round: 3 });
    }
}

fn wednesdays(w: &World) -> Vec<Date> {
    let start = Date::ymd(w.year, 9, 1);
    (0..89).map(|k| start.add(k)).filter(|d| d.weekday() == Weekday::Wed).collect()
}

#[test]
fn an_nfl_scout_sent_to_a_region_calls_on_its_schools_and_leaves_holes_at_home() {
    let mut w = empty();
    let t = (0..w.clubs.len())
        .filter(|&t| crate::world::franchise::active_in(&w, t, w.year))
        .max_by_key(|&t| w.clubs[t].roster.len())
        .unwrap();
    w.start_career(&you(), Job::Nfl(t), 7);
    // The departments are hired with the first fall's work.
    let draft_year = w.year + 1;
    scouts::staff(&mut w, false, draft_year);
    seniors(&mut w, 600);
    // The fall's list is 5 × the draft's picks at the world's scale: at
    // full scale every one of these seniors is on it.
    w.options.scale = 1.0;
    let areas = w.clubs[t].scouts.iter().filter(|s| s.role == Role::Area).count();
    let (k, home) = w.clubs[t]
        .scouts
        .iter()
        .enumerate()
        .find(|(_, s)| s.role == Role::Area)
        .map(|(k, s)| (k as u8, scouts::covers(s.region, areas)))
        .unwrap();
    // Texas, Louisiana and Arkansas, unless that's his own.
    let r = if home.contains(&5) { 2 } else { 5 };
    let region_of = |w: &World, i: usize| w.people[i].college.map(|c| scouts::region_of(w.programs[c].state));
    w.scouting.dept.assignments.push(Assignment {
        who: k as u32,
        target: Target::Region(r),
        since: Date::ymd(w.year, 9, 1),
    });
    for d in wednesdays(&w) {
        w.today = d;
        dept::day(&mut w, d);
    }
    let looks = |w: &World, kind: LookKind| -> Vec<(usize, Date)> {
        w.clubs[t]
            .files
            .iter()
            .flat_map(|(&id, v)| v.iter().filter(|l| l.who == k && l.kind == kind).map(move |l| (id, l.date)))
            .filter_map(|(id, d)| crate::world::scouting::place(w, id).map(|i| (i, d)))
            .collect()
    };
    let away = looks(&w, LookKind::School);
    assert!(away.len() >= 20, "a fall in the region files its seniors: {} looks", away.len());
    assert!(away.iter().all(|&(i, _)| region_of(&w, i) == Some(r)), "every call is in the region he was sent to");
    let away_days: HashSet<Date> = away.iter().map(|x| x.1).collect();
    assert!(away_days.len() >= 26, "two to four calls a week, a day each: {} days", away_days.len());
    let schools: HashSet<usize> = away.iter().filter_map(|&(i, _)| w.people[i].college).collect();
    let in_region: HashSet<usize> = w
        .people
        .iter()
        .filter(|p| p.stage == Stage::College)
        .filter_map(|p| p.college)
        .filter(|&c| scouts::region_of(w.programs[c].state) == r)
        .collect();
    let mut by_day: Vec<(Date, usize)> = away
        .iter()
        .filter_map(|&(i, d)| w.people[i].college.map(|c| (d, c)))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    by_day.sort();
    assert!(
        in_region.len() >= 8 && schools.len() >= 8,
        "on from where he left off, not the same school each week: {} of {} ({by_day:?})",
        schools.len(),
        in_region.len()
    );

    // December: the fall keeps his work and makes his own calls on the
    // days he was home.
    w.today = Date::ymd(w.year, 12, 6);
    scouts::fall(&mut w);
    let after = looks(&w, LookKind::School);
    let kept = after.iter().filter(|&&(i, _)| region_of(&w, i) == Some(r)).count();
    assert!(kept >= away.len(), "the region's looks stay in the file: {kept} of {}", away.len());
    let at_home: Vec<Date> =
        after.iter().filter(|&&(i, _)| region_of(&w, i).is_some_and(|x| home.contains(&x))).map(|x| x.1).collect();
    assert!(!at_home.is_empty(), "he still calls on his own region");
    assert!(at_home.iter().all(|d| !away_days.contains(d)), "no call at home on a day he was away");

    // January: the region's prospects on film, five a week, on from where
    // he left off.
    let jan: Vec<Date> =
        (0..21).map(|k| Date::ymd(w.year + 1, 1, 5).add(k)).filter(|d| d.weekday() == Weekday::Wed).take(2).collect();
    let mut films: Vec<HashSet<usize>> = Vec::new();
    for &d in &jan {
        w.today = d;
        dept::day(&mut w, d);
        let f: HashSet<usize> = looks(&w, LookKind::Film).into_iter().filter(|x| x.1 == d).map(|x| x.0).collect();
        assert_eq!(f.len(), 5, "five tapes a week");
        assert!(f.iter().all(|&i| region_of(&w, i) == Some(r)), "the region's prospects");
        films.push(f);
    }
    assert!(films[0].is_disjoint(&films[1]), "a new five each week");
}

#[test]
fn an_nfl_scout_on_cross_checks_takes_the_top_of_your_board() {
    let mut w = empty();
    let t = (0..w.clubs.len())
        .filter(|&t| crate::world::franchise::active_in(&w, t, w.year))
        .max_by_key(|&t| w.clubs[t].roster.len())
        .unwrap();
    w.start_career(&you(), Job::Nfl(t), 7);
    // The departments are hired with the first fall's work.
    let draft_year = w.year + 1;
    scouts::staff(&mut w, false, draft_year);
    seniors(&mut w, 300);
    let k = w.clubs[t].scouts.iter().position(|s| s.role == Role::National).unwrap() as u8;
    w.scouting.dept.assignments.push(Assignment {
        who: k as u32,
        target: Target::CrossCheck,
        since: Date::ymd(w.year, 9, 1),
    });
    let days: Vec<Date> = wednesdays(&w).into_iter().take(2).collect();
    let mut seen: Vec<u32> = Vec::new();
    for d in days {
        w.today = d;
        let top: Vec<u32> = {
            let g = crate::world::scouting::board::seat_grades(&w);
            let mine: HashSet<u32> =
                w.clubs[t].files.iter().filter(|(_, v)| v.iter().any(|l| l.who == k)).map(|(&id, _)| id).collect();
            let mut v: Vec<(f64, u32)> =
                g.into_iter().filter(|(id, _)| !mine.contains(id)).map(|(id, g)| (g, id)).collect();
            v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            v.into_iter().take(3).map(|x| x.1).collect()
        };
        dept::day(&mut w, d);
        let today: Vec<u32> = w.clubs[t]
            .files
            .iter()
            .filter(|(_, v)| {
                v.iter().any(|l| l.who == k && l.kind == LookKind::CrossCheck && l.date >= d && l.date < d.add(3))
            })
            .map(|(&id, _)| id)
            .collect();
        assert_eq!(today.len(), 3, "three cross-checks a week");
        assert!(today.iter().all(|id| top.contains(id)), "the top of your board he hasn't seen");
        assert!(today.iter().all(|id| !seen.contains(id)), "not the same men twice");
        seen.extend(today);
    }
}

/// A college seat with recruits from one state: pros made into high
/// school juniors and seniors at home in `st`.
fn college_world() -> (World, usize, u8) {
    let mut w = empty();
    let prog = w.programs.iter().position(|p| p.division == Division::FBS).unwrap();
    w.start_career(&you(), Job::College(prog), 7);
    let st = w.programs[prog].state as u8;
    let season = w.dates.season;
    for (k, p) in w.people.iter_mut().filter(|p| p.stage == Stage::Pro).take(240).enumerate() {
        p.stage = Stage::HighSchool;
        p.pro = None;
        p.class_of = season + 1 + (k % 2) as i32;
        p.player.age = 17;
        p.home_state = if k % 3 == 0 { st as usize } else { (st as usize + 1 + k % 40) % 51 };
        p.stars = 3;
    }
    (w, prog, st)
}

/// The fall's evaluation days: Thursdays and Fridays in the evaluation
/// period from September to November.
fn eval_days(w: &World) -> Vec<Date> {
    let s = w.dates.season;
    (0..91)
        .map(|k| Date::ymd(s, 9, 1).add(k))
        .filter(|&d| matches!(d.weekday(), Weekday::Thu | Weekday::Fri))
        .filter(|&d| crate::calendar::recruiting_period(&w.dates, d) == Period::Evaluation)
        .collect()
}

#[test]
fn a_college_coach_sent_to_a_state_or_a_group_spends_his_days_there() {
    let (mut w, prog, st) = college_world();
    let days = eval_days(&w);
    assert!(days.len() >= 4, "a fall with evaluation days");
    let coaches: Vec<usize> = w.programs[prog].position_coaches.iter().flatten().copied().collect();
    let (a, b) = (coaches[0], coaches[1]);
    let g = 3u8;
    w.scouting.dept.assignments.push(Assignment { who: COACH + a as u32, target: Target::Region(st), since: days[0] });
    w.scouting.dept.assignments.push(Assignment { who: COACH + b as u32, target: Target::Group(g), since: days[0] });
    for &d in days.iter().take(4) {
        w.today = d;
        dept::day(&mut w, d);
    }
    let in_person = |w: &World, who: u32| -> Vec<usize> {
        w.scouting
            .dept
            .looks
            .iter()
            .filter(|(_, v)| v.iter().any(|l| l.who == who && matches!(l.kind, Kind::Practice | Kind::Game)))
            .filter_map(|(&id, _)| crate::world::scouting::place(w, id))
            .collect()
    };
    let by_a = in_person(&w, COACH + a as u32);
    assert_eq!(by_a.len(), 4, "one recruit a day, a new one each day");
    assert!(by_a.iter().all(|&i| w.people[i].home_state as u8 == st), "all from the state he was sent to");
    let by_b = in_person(&w, COACH + b as u32);
    assert_eq!(by_b.len(), 4, "one recruit a day at his group");
    assert!(
        by_b.iter().all(|&i| crate::world::coach::Group::of(w.people[i].player.position) as u8 == g),
        "all at the group he was sent to"
    );
    // A third coach on cross-checks: the top of the board another coach
    // saw (alone, as the staff sends two a day).
    w.scouting.dept.assignments.clear();
    let c = coaches[2];
    w.scouting.dept.assignments.push(Assignment { who: COACH + c as u32, target: Target::CrossCheck, since: days[4] });
    w.today = days[4];
    dept::day(&mut w, days[4]);
    let by_c = in_person(&w, COACH + c as u32);
    assert_eq!(by_c.len(), 1, "his day's cross-check");
    let id = w.people[by_c[0]].player.id;
    assert!(
        w.scouting.dept.looks[&id]
            .iter()
            .any(|l| l.who != COACH + c as u32 && matches!(l.kind, Kind::Practice | Kind::Game)),
        "a second look at a recruit another coach saw"
    );
}

#[test]
fn a_personnel_staffer_on_a_state_studies_its_film_instead_of_his_share() {
    let (mut w, prog, st) = college_world();
    let staff = readers::personnel(&w, prog).len();
    assert!(staff >= 2, "a personnel staff");
    let monday = (0..7).map(|k| Date::ymd(w.dates.season, 9, 7).add(k)).find(|d| d.weekday() == Weekday::Mon).unwrap();
    w.scouting.dept.assignments.push(Assignment { who: PERSONNEL, target: Target::Region(st), since: monday });
    w.today = monday;
    dept::day(&mut w, monday);
    let film = |w: &World, who: u32| -> Vec<usize> {
        w.scouting
            .dept
            .looks
            .iter()
            .filter(|(_, v)| v.iter().any(|l| l.who == who && l.kind == Kind::HsFilm && l.date == monday))
            .filter_map(|(&id, _)| crate::world::scouting::place(w, id))
            .collect()
    };
    let his = film(&w, PERSONNEL);
    assert!(!his.is_empty(), "he studies the state's film");
    assert!(his.iter().all(|&i| w.people[i].home_state as u8 == st), "only the state's recruits");
    assert!(!film(&w, PERSONNEL + 1).is_empty(), "the rest of the staff keep their share of the board");
}
