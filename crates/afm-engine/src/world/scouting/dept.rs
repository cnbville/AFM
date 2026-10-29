//! Your department at work (topics 2, 3, 14-17): who looks at whom, when,
//! and under which rules. At an NFL club your scouts' fall, the cross-checks,
//! the all-star weeks, the combine and the spring are the built year
//! (`pro::scouts`); here your own assignments add looks on top (a scout sent
//! to see one player again, to a pro day, a cross-check), and your standing
//! instructions steer the director. At a college the on-field coaches spend
//! the NCAA's evaluation days on the board's recruits, the personnel staff
//! study film and make calls, camps and visits are looks, and portal
//! entrants come with college film and their old staff's word. At a high
//! school you read your own players every day.
//!
//! Everything a look leaves behind is a 12-byte `SeatLook` by player id.

use std::collections::HashMap;

use super::readers::{self, COACH, PERSONNEL, Staff};
use super::{Kind, SeatLook};
use crate::calendar::{Date, Period, Weekday};
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

/// What a person is sent to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Target {
    /// One player (by id): see him again.
    Player(u32),
    /// A region (an NFL area scout's eight regions) or a state (college).
    Region(u8),
    /// A position group (`coach::Group` index): every one in the class.
    Group(u8),
    /// A second look at the top of another scout's list.
    CrossCheck,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Assignment {
    /// Reader code (`readers`).
    pub who: u32,
    pub target: Target,
    pub since: Date,
}

/// A standing instruction to your director or coordinator (calendar topic
/// 10), in plain choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Instruction {
    /// Focus on one position group.
    Focus(u8),
    /// Small schools too (more looks below the FBS).
    SmallSchools,
    /// Players who fit what we run.
    SchemeFit,
    /// Ask me before taking anyone off the board.
    AskBeforeDnd,
    /// Lean on the analytics: age, production, testing.
    Analytics,
}

impl Instruction {
    pub fn words(self) -> String {
        match self {
            Instruction::Focus(g) => {
                format!("Focus on {}", crate::world::coach::Group::ALL[(g as usize).min(7)].name().to_lowercase())
            }
            Instruction::SmallSchools => "Small schools too".into(),
            Instruction::SchemeFit => "Players who fit what we run".into(),
            Instruction::AskBeforeDnd => "Ask me before taking anyone off the board".into(),
            Instruction::Analytics => "Lean on the analytics".into(),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Dept {
    /// Your seat's looks beyond the built club files, by player id.
    #[serde(default)]
    pub looks: HashMap<u32, Vec<SeatLook>>,
    #[serde(default)]
    pub assignments: Vec<Assignment>,
    #[serde(default)]
    pub instructions: Vec<Instruction>,
    /// Evaluation days used this fall and this spring (college): (season,
    /// fall, spring), one coach out for one day is one day.
    #[serde(default)]
    pub eval_days: (i32, u16, u16),
    /// Each evaluation day's date and who went where, this season (for the
    /// 33 squares on the department screen): date, reader code, player id.
    #[serde(default)]
    pub days_log: Vec<(Date, u32, u32)>,
    /// Looks you asked for that wait for a free day (player id, kind).
    #[serde(default)]
    pub asks: Vec<(u32, Kind, Date)>,
    /// The "my players" list (topic 18): player id, when added, why, and
    /// your staff's first grade of him then (role words) with the public's.
    #[serde(default)]
    pub my_players: Vec<Followed>,
    /// Each coach's recruiting area (college): coach index, states.
    #[serde(default)]
    pub areas: Vec<(usize, Vec<u8>)>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Followed {
    pub id: u32,
    pub since: Date,
    pub why: String,
    pub first_read: String,
    pub public_then: String,
}

/// The NCAA's evaluation-day limits for the whole staff (FBS): 42 days
/// from September to November (Bylaw 13.17, through 2022-23), about 33
/// under the football recruiting model adopted in 2023; 168 in the spring
/// evaluation period. Below the FBS the same by rule (proposed, not
/// confirmed: the real FCS limits are close).
pub fn eval_limits(season: i32) -> (u16, u16) {
    (if season >= 2023 { 33 } else { 42 }, 168)
}

/// Adds a look for your seat (deduplicated by day, kind and person).
pub fn add(w: &mut World, id: u32, who: u32, kind: Kind, date: Date) {
    let v = w.scouting.dept.looks.entry(id).or_default();
    if v.iter().any(|l| l.who == who && l.kind == kind && l.date == date) {
        return;
    }
    v.push(SeatLook { who, kind, date });
}

/// Your college program's board: the recruits it looks at (topic 15): the
/// ones it offered, who visited, who committed, and the best in its
/// footprint and pipelines for the next two classes, by the public's word
/// and its need; plus portal entrants while the portal is open.
pub fn watch(w: &World, prog: usize) -> Vec<usize> {
    let p = &w.programs[prog];
    let season = w.dates.season;
    let mut scored: Vec<(f64, usize)> = Vec::new();
    for (i, q) in w.people.iter().enumerate() {
        let recruit = matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)
            && q.class_of > season
            && q.class_of <= season + 2
            && q.player.age >= 15;
        let portal = q.stage == Stage::College && q.cal.lean > 0.99 && q.college != Some(prog);
        if !recruit && !portal {
            continue;
        }
        let ours = q.cal.offers.contains(&prog)
            || q.cal.visits.iter().any(|v| v.0 == prog)
            || q.cal.commit.is_some_and(|c| c.0 == prog)
            || q.cal.signed.is_some_and(|s| s.program == prog);
        let near = p.footprint[q.home_state.min(50)] + 0.5 * p.pipeline[q.home_state.min(50)];
        let s = if ours { 100.0 } else { 0.0 } + q.stars as f64 * 2.0 + near * 3.0 + if portal { 4.0 } else { 0.0 };
        if ours || q.stars >= 3 || near > 0.6 {
            scored.push((s, i));
        }
    }
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let n = match p.division {
        crate::world::data::Division::FBS => 320,
        crate::world::data::Division::FCS => 200,
        _ => 120,
    };
    scored.truncate(n);
    scored.into_iter().map(|x| x.1).collect()
}

/// The day's work for your seat.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(staff) = readers::seat_staff(w) else { return };
    match staff {
        Staff::Club(t) => nfl_day(w, t, d),
        Staff::Program(p) => college_day(w, p, d),
        Staff::School(_) => {}
    }
    asks_day(w, staff, d);
}

/// F22: the looks you asked for from a file become looks when the rules
/// and the calendar allow: a background check by your security within the
/// week, a medical recheck by your doctors (the combine's recheck in April,
/// else a visit to your building), a private workout after the combine and
/// before the draft (your head coach runs it; a quarterback who throws
/// only for some clubs throws for you only if he throws for you), film
/// study, a call, a transcript or a social media check within the week,
/// and a scout's school call or a cross-check in the fall. Evaluating in
/// person waits for an evaluation day (`college_day`). A look the season
/// no longer allows (his draft has passed) is dropped.
fn asks_day(w: &mut World, staff: Staff, d: Date) {
    if w.scouting.dept.asks.is_empty() {
        return;
    }
    let (_, m, _) = d.parts();
    let wd = d.weekday();
    let draft_day = w.dates.draft[0].date;
    let spring = matches!(m, 3 | 4) && d < draft_day && d >= Date::ymd(d.year(), 3, 10);
    let asks = w.scouting.dept.asks.clone();
    let mut done: Vec<(u32, Kind)> = Vec::new();
    for (id, kind, asked) in asks {
        let Some(i) = super::place(w, id) else {
            done.push((id, kind));
            continue;
        };
        let waited = d.0 - asked.0;
        let stage = w.people[i].stage;
        let club = if let Staff::Club(t) = staff { Some(t) } else { None };
        let hc = match staff {
            Staff::Club(t) => w.nfl.get(t).and_then(|x| x.head_coach),
            Staff::Program(p) => w.programs[p].head_coach,
            Staff::School(h) => w.high_schools.get(h).and_then(|x| x.coach),
        };
        let (who, ready): (Option<u32>, bool) = match kind {
            Kind::Background => (Some(readers::SECURITY), waited >= 5),
            Kind::Medical => {
                // The combine's recheck is in April; otherwise he comes in.
                let recheck = club.is_some() && m == 4 && d.parts().2 >= 8;
                (Some(readers::MEDICAL), recheck || (waited >= 7 && club.is_none()) || (spring && waited >= 10))
            }
            Kind::Workout => {
                let throws = club.is_none_or(|t| super::predraft::throws_for(w, i, t));
                let ok = stage == Stage::Declared && spring && matches!(wd, Weekday::Tue | Weekday::Thu) && waited >= 3;
                if stage != Stage::Declared && d > draft_day && asked < draft_day {
                    done.push((id, kind));
                    continue;
                }
                (hc.map(|c| COACH + c as u32), ok && throws)
            }
            Kind::Film | Kind::Call | Kind::Transcript | Kind::Social => {
                let who = match kind {
                    Kind::Transcript => readers::ACADEMIC,
                    Kind::Social => readers::PERSONNEL,
                    _ => hc.map_or(readers::PERSONNEL, |c| COACH + c as u32),
                };
                let days = if kind == Kind::Film { super::film::time::staff_days(w, 3) } else { 3 };
                (Some(who), waited >= days && wd != Weekday::Sun)
            }
            Kind::School | Kind::CrossCheck => {
                let scout = club.and_then(|t| {
                    let role = if kind == Kind::CrossCheck {
                        crate::world::pro::scouts::Role::National
                    } else {
                        crate::world::pro::scouts::Role::Area
                    };
                    w.clubs.get(t)?.scouts.iter().position(|s| s.role == role)
                });
                (scout.map(|k| k as u32), waited >= 7 && wd == Weekday::Wed)
            }
            Kind::ProFilm => (Some(readers::PRO_SCOUT), waited >= super::film::time::staff_days(w, 3)),
            // A tape sent in (SK72): the coach of his spot at a college, the
            // area scout at a club, within the week; film uses no
            // evaluation days.
            Kind::HsFilm | Kind::CollegeFilm => {
                let who = match staff {
                    Staff::Program(p) => evaluator_for(w, p, i).map(|c| COACH + c as u32),
                    Staff::Club(t) => w
                        .clubs
                        .get(t)
                        .and_then(|c| c.scouts.iter().position(|s| s.role == crate::world::pro::scouts::Role::Area))
                        .map(|k| k as u32),
                    Staff::School(_) => None,
                };
                (who, waited >= super::film::time::staff_days(w, 2) && wd != Weekday::Sun)
            }
            _ => continue,
        };
        let Some(who) = who else {
            done.push((id, kind));
            continue;
        };
        if !ready {
            // An ask older than the season that allows it is let go.
            if waited > 200 {
                done.push((id, kind));
            }
            continue;
        }
        // Out of the fall, seeing him again is his film.
        let kind_now = if kind == Kind::School && !matches!(m, 9..=12) { Kind::Film } else { kind };
        use crate::world::pro::scouts::{HC, LookKind as L};
        // At a club the head coach is read by his club code, so the
        // workout and the board's drill look are one man's.
        let who = if club.is_some() && kind_now == Kind::Workout { HC as u32 } else { who };
        match (club, kind_now) {
            // The board reads the club's own file: these go there too (a
            // recheck reaches the board through `medical_cost`).
            (Some(t), Kind::Workout) => club_look(w, t, id, HC, L::Coach, d),
            (Some(t), Kind::School) => club_look(w, t, id, who as u8, L::School, d),
            (Some(t), Kind::Film) if who < 250 => club_look(w, t, id, who as u8, L::Film, d),
            (Some(t), Kind::CrossCheck) => club_look(w, t, id, who as u8, L::CrossCheck, d),
            (Some(t), Kind::CollegeFilm | Kind::HsFilm) if who < 250 => club_look(w, t, id, who as u8, L::Film, d),
            _ => {}
        }
        if who >= 250
            || club.is_none()
            || !matches!(kind_now, Kind::School | Kind::Film | Kind::CrossCheck | Kind::CollegeFilm | Kind::HsFilm)
        {
            add(w, id, who, kind_now, d);
        }
        done.push((id, kind));
    }
    w.scouting.dept.asks.retain(|a| !done.iter().any(|x| x.0 == a.0 && x.1 == a.1));
}

/// NFL: your assignments on top of the built year. A scout on a player sees
/// him every second week from September to April (fall school calls, all-star
/// week, spring film); a national scout on cross-checks takes the top of the
/// club's list; the director follows your focus.
fn nfl_day(w: &mut World, t: usize, d: Date) {
    let (_, m, _) = d.parts();
    let season_on = matches!(m, 9..=12 | 1..=4);
    if !season_on || d.weekday() != Weekday::Wed {
        return;
    }
    let assignments = w.scouting.dept.assignments.clone();
    let week = d.0 / 7;
    for a in assignments {
        let who = a.who;
        if who >= 250 {
            continue;
        }
        match a.target {
            Target::Player(id) => {
                if week % 2 == 0
                    && let Some(i) = super::place(w, id)
                    && matches!(w.people[i].stage, Stage::College | Stage::Declared)
                {
                    let kind = if m >= 9 {
                        crate::world::pro::scouts::LookKind::School
                    } else {
                        crate::world::pro::scouts::LookKind::Film
                    };
                    club_look(w, t, id, who as u8, kind, d);
                }
            }
            Target::Group(g) => {
                // A scout on a group writes film reports on its top names,
                // five a week.
                let ids = top_of_group(w, g, 60);
                let k = (week as usize * 5) % ids.len().max(1);
                for &id in ids.iter().cycle().skip(k).take(5.min(ids.len())) {
                    club_look(w, t, id, who as u8, crate::world::pro::scouts::LookKind::Film, d);
                }
            }
            Target::CrossCheck | Target::Region(_) => {}
        }
    }
    // F21 "Small schools too": your area scouts add six film looks a week
    // on the best prospects below the FBS nobody of yours has seen.
    if w.scouting.dept.instructions.contains(&Instruction::SmallSchools) {
        let draft_year = if w.today <= w.dates.draft[0].date { w.year } else { w.year + 1 };
        let mut small: Vec<(f64, u32)> = w
            .people
            .iter()
            .filter(|p| {
                (p.stage == Stage::Declared || (p.stage == Stage::College && draft_year - p.class_of >= 3))
                    && p.college.is_some_and(|c| w.programs[c].division != crate::world::data::Division::FBS)
                    && !w.clubs.get(t).is_some_and(|c| c.files.contains_key(&p.player.id))
            })
            .map(|p| (p.cal.draft_read.map_or(p.last_performance.unwrap_or(0.0), |r| r.score), p.player.id))
            .collect();
        small.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let areas: Vec<usize> = w.clubs.get(t).map_or(Vec::new(), |c| {
            c.scouts
                .iter()
                .enumerate()
                .filter(|s| s.1.role == crate::world::pro::scouts::Role::Area)
                .map(|s| s.0)
                .collect()
        });
        for (k, &(_, id)) in small.iter().take(6).enumerate() {
            if let Some(&who) = areas.get(k % areas.len().max(1)) {
                club_look(w, t, id, who as u8, crate::world::pro::scouts::LookKind::Film, d);
            }
        }
    }
    // Your focus: the director studies five more of the group's names a week.
    let focus: Vec<u8> = w
        .scouting
        .dept
        .instructions
        .iter()
        .filter_map(|i| if let Instruction::Focus(g) = i { Some(*g) } else { None })
        .collect();
    if let Some(dir) =
        w.clubs.get(t).and_then(|c| c.scouts.iter().position(|s| s.role == crate::world::pro::scouts::Role::Director))
    {
        for g in focus {
            let ids = top_of_group(w, g, 80);
            let k = (week as usize * 5) % ids.len().max(1);
            for &id in ids.iter().cycle().skip(k).take(5.min(ids.len())) {
                club_look(w, t, id, dir as u8, crate::world::pro::scouts::LookKind::Director, d);
            }
        }
    }
}

/// A look in your club's own file (the board reads it).
pub(crate) fn club_look(w: &mut World, t: usize, id: u32, who: u8, kind: crate::world::pro::scouts::LookKind, d: Date) {
    if let Some(c) = w.clubs.get_mut(t) {
        let f = c.files.entry(id).or_default();
        if !f.iter().any(|l| l.who == who && l.kind == kind && l.date == d) {
            f.push(crate::world::pro::scouts::Look { who, kind, date: d });
        }
    }
}

/// The class's best at a position group by the public's December read.
fn top_of_group(w: &World, g: u8, n: usize) -> Vec<u32> {
    let draft_year = if w.today <= w.dates.draft[0].date { w.year } else { w.year + 1 };
    let mut v: Vec<(f64, u32)> = w
        .people
        .iter()
        .filter(|p| {
            (p.stage == Stage::Declared || (p.stage == Stage::College && draft_year - p.class_of >= 4))
                && crate::world::coach::Group::of(p.player.position) as u8 == g
        })
        .filter_map(|p| p.cal.draft_read.map(|r| (r.score, p.player.id)))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.truncate(n);
    v.into_iter().map(|x| x.1).collect()
}

/// College: the recruiting year's looks for your program.
fn college_day(w: &mut World, prog: usize, d: Date) {
    let season = w.dates.season;
    if w.scouting.dept.eval_days.0 != season {
        w.scouting.dept.eval_days = (season, 0, 0);
        w.scouting.dept.days_log.clear();
    }
    let period = crate::calendar::recruiting_period(&w.dates, d);
    let wd = d.weekday();
    // Mondays: the board is refreshed and the personnel staff study film,
    // a slice of the board each week; calls go out on Tuesdays in contact
    // and evaluation periods.
    if wd == Weekday::Mon || wd == Weekday::Tue || wd == Weekday::Fri || wd == Weekday::Sat || wd == Weekday::Thu {
        let board = watch(w, prog);
        let staff = readers::personnel(w, prog);
        let hudl = w.year >= 2008;
        if wd == Weekday::Mon && !staff.is_empty() {
            let slice = (board.len() / 4).max(1);
            let k = (d.0 / 7) as usize % 4;
            for (j, &i) in board.iter().enumerate().skip(k * slice).take(slice) {
                let id = w.people[i].player.id;
                let who = PERSONNEL + (j % staff.len()) as u32;
                let small = w.high_schools.get(w.people[i].hs).is_some_and(|h| h.classification <= 2);
                // Before Hudl a small school's tape was what a coach mailed.
                if !hudl && small && !readers::chance(Staff::Program(prog), 0x7A9E ^ d.0 as u64, id, 0.35) {
                    continue;
                }
                let kind = if w.people[i].stage == Stage::College { Kind::CollegeFilm } else { Kind::HsFilm };
                add(w, id, who, kind, d);
            }
        }
        if wd == Weekday::Tue && matches!(period, Period::Contact | Period::Evaluation | Period::Quiet) {
            // The area recruiter calls his kids' coaches, teachers and counselor.
            let slice = (board.len() / 6).max(1);
            let k = (d.0 / 7) as usize % 6;
            for &i in board.iter().skip(k * slice).take(slice) {
                let id = w.people[i].player.id;
                // A coach overruled again and again makes fewer calls (SK29).
                if let Some(c) = recruiter_for(w, prog, i)
                    && readers::chance(
                        Staff::Program(prog),
                        0x4EA7 ^ d.0 as u64,
                        id,
                        crate::world::front_office::chain::heart(w, c),
                    )
                {
                    let kind = if w.people[i].stage == Stage::College { Kind::OldStaff } else { Kind::Call };
                    add(w, id, COACH + c as u32, kind, d);
                }
            }
        }
        // Evaluation days: Thursdays and Fridays (practice and the game) in
        // the fall evaluation period, weekdays in the spring window, up to
        // the staff's limit; your assignments first, then the coordinator's
        // asks, then the top of the board nobody has seen in person.
        let fall = period == Period::Evaluation && matches!(d.parts().1, 9..=11);
        let spring = period == Period::Contact && d.parts().1 == 4
            || (d.parts().1 == 5 && period == Period::Quiet && d.parts().2 <= 23);
        if (fall && (wd == Weekday::Thu || wd == Weekday::Fri)) || (spring && wd == Weekday::Thu) {
            let (lim_fall, lim_spring) = eval_limits(season);
            let (_, used_f, used_s) = w.scouting.dept.eval_days;
            let left = if fall { lim_fall.saturating_sub(used_f) } else { lim_spring.saturating_sub(used_s) };
            // How many days a week the staff spends (the fall's days spread
            // over its 12 weeks; the spring's over six).
            let per = if fall { ((lim_fall as f64) / 24.0).ceil() as u16 } else { 14 };
            let today = left.min(per);
            if today > 0 {
                let mut picks: Vec<(u32, usize)> = Vec::new();
                for a in w.scouting.dept.assignments.clone() {
                    if let Target::Player(id) = a.target
                        && a.who >= COACH
                        && let Some(i) = super::place(w, id)
                        && matches!(w.people[i].stage, Stage::HighSchool | Stage::PrepSchool)
                    {
                        picks.push((a.who, i));
                    }
                }
                let asks: Vec<u32> =
                    w.scouting.dept.asks.iter().filter(|a| a.1 == Kind::Practice).map(|a| a.0).collect();
                for id in asks {
                    if let Some(i) = super::place(w, id)
                        && let Some(c) = evaluator_for(w, prog, i)
                    {
                        picks.push((COACH + c as u32, i));
                    }
                }
                let seen_since = Date::ymd(season, 9, 1);
                for &i in &board {
                    if picks.len() >= today as usize {
                        break;
                    }
                    if w.people[i].stage == Stage::College {
                        continue;
                    }
                    let id = w.people[i].player.id;
                    let seen = w.scouting.dept.looks.get(&id).is_some_and(|v| {
                        v.iter()
                            .any(|l| matches!(l.kind, Kind::Practice | Kind::Game | Kind::Camp) && l.date >= seen_since)
                    });
                    if !seen && let Some(c) = evaluator_for(w, prog, i) {
                        picks.push((COACH + c as u32, i));
                    }
                }
                picks.truncate(today as usize);
                for (who, i) in picks {
                    let id = w.people[i].player.id;
                    let kind = if wd == Weekday::Fri { Kind::Game } else { Kind::Practice };
                    add(w, id, who, kind, d);
                    w.scouting.dept.days_log.push((d, who, id));
                    w.scouting.dept.asks.retain(|a| a.0 != id || a.1 != Kind::Practice);
                    if fall {
                        w.scouting.dept.eval_days.1 += 1;
                    } else {
                        w.scouting.dept.eval_days.2 += 1;
                    }
                }
            }
        }
    }
    // Camps: your one-day camps in June (laser times, a day of coaching),
    // for the board's recruits within a day's drive whose families can pay.
    if d == Date::nth_weekday(season, 6, Weekday::Sat, 2) || d == Date::nth_weekday(season, 6, Weekday::Sat, 3) {
        let board = watch(w, prog);
        let home = w.programs[prog].state;
        let hc = w.programs[prog].head_coach;
        for &i in &board {
            let q = &w.people[i];
            if q.stage != Stage::HighSchool {
                continue;
            }
            let near = crate::world::geo::miles(home, q.home_state) <= 400.0;
            let can = q.bg.money >= 0.3 || near;
            if near && can && readers::chance(Staff::Program(prog), 0xCA4B ^ d.0 as u64, q.player.id, 0.45) {
                let who = evaluator_for(w, prog, i).or(hc).map_or(PERSONNEL, |c| COACH + c as u32);
                let id = q.player.id;
                add(w, id, who, Kind::Camp, d);
            }
        }
    }
    // Visits the recruits took with you become looks (junior days,
    // officials, unofficials, your home visits): the family, the kid on
    // campus.
    let today_visits: Vec<(u32, Kind)> = w
        .people
        .iter()
        .filter(|q| matches!(q.stage, Stage::HighSchool | Stage::PrepSchool))
        .flat_map(|q| {
            q.cal.visits.iter().filter(|v| v.0 == prog && v.1 == d).map(move |v| {
                (
                    q.player.id,
                    match v.2 {
                        crate::world::Visit::Official => Kind::Official,
                        crate::world::Visit::Unofficial => Kind::Unofficial,
                        crate::world::Visit::DropIn => Kind::Practice,
                        crate::world::Visit::Home => Kind::HomeVisit,
                    },
                )
            })
        })
        .collect();
    if !today_visits.is_empty() {
        let hc = w.programs[prog].head_coach.map_or(PERSONNEL, |c| COACH + c as u32);
        for (id, kind) in today_visits {
            add(w, id, hc, kind, d);
        }
    }
    // Transcripts through the Eligibility Center each fall for seniors on
    // the board, and their social media.
    if d == Date::ymd(season, 10, 1) {
        let board = watch(w, prog);
        for &i in &board {
            if w.people[i].stage == Stage::HighSchool && w.people[i].class_of == season + 1 {
                let id = w.people[i].player.id;
                add(w, id, readers::ACADEMIC, Kind::Transcript, d);
                add(w, id, PERSONNEL, Kind::Social, d);
            }
        }
    }
}

/// The coach who evaluates a recruit: his position coach (topic 15: the
/// position coach evaluates and sells the role).
pub fn evaluator_for(w: &World, prog: usize, i: usize) -> Option<usize> {
    let p = &w.programs[prog];
    let g = crate::world::coach::Group::of(w.people[i].player.position) as usize;
    p.position_coaches[g].or(p.head_coach)
}

/// The coach whose recruiting area holds a recruit's state: your set areas
/// first, else by the coach's ties (his home state, then nearest).
pub fn recruiter_for(w: &World, prog: usize, i: usize) -> Option<usize> {
    area_coach(w, prog, w.people[i].home_state as u8)
}

/// The coach who recruits a state for a program: the area you set, else
/// the staff's own split (each state to one on-field assistant).
pub fn area_coach(w: &World, prog: usize, st: u8) -> Option<usize> {
    if let Some((c, _)) = w.scouting.dept.areas.iter().find(|a| a.1.contains(&st)) {
        return Some(*c);
    }
    default_area_coach(w, prog, st)
}

/// The staff's own split of the states, before you set any: a state goes
/// to an assistant from it (his ties there), else to one of the staff.
pub fn default_area_coach(w: &World, prog: usize, st: u8) -> Option<usize> {
    let p = &w.programs[prog];
    let staff: Vec<usize> = p.position_coaches.iter().flatten().copied().collect();
    if staff.is_empty() {
        return p.head_coach;
    }
    let k = |n: usize| (key_uniform(prog as u64 ^ 0xA2EA, st as u64) * n as f64) as usize;
    let home: Vec<usize> = staff
        .iter()
        .copied()
        .filter(|&c| w.coaches.get(c).is_some_and(|c| c.home_state == Some(st as usize)))
        .collect();
    if !home.is_empty() {
        return Some(home[k(home.len()).min(home.len() - 1)]);
    }
    Some(staff[k(staff.len()).min(staff.len() - 1)])
}

/// You give a state to one of your assistants' recruiting areas (scouting
/// topic 15), or back to the staff's own split with `None`.
pub fn set_area(w: &mut World, st: u8, coach: Option<usize>) {
    for a in w.scouting.dept.areas.iter_mut() {
        a.1.retain(|&s| s != st);
    }
    if let Some(c) = coach {
        match w.scouting.dept.areas.iter_mut().find(|a| a.0 == c) {
            Some(a) => a.1.push(st),
            None => w.scouting.dept.areas.push((c, vec![st])),
        }
    }
    w.scouting.dept.areas.retain(|a| !a.1.is_empty());
}

/// Prunes looks of players nobody will read again (signed elsewhere and
/// gone, out of football), each new football year.
pub(crate) fn prune(w: &mut World) {
    let keep: std::collections::HashSet<u32> =
        w.people.iter().filter(|p| !matches!(p.stage, Stage::Done)).map(|p| p.player.id).collect();
    let mut mine: std::collections::HashSet<u32> = w.scouting.dept.my_players.iter().map(|f| f.id).collect();
    if let Some(c) = &w.career {
        mine.extend(c.desk.listed.iter().map(|l| l.id));
    }
    w.scouting.dept.looks.retain(|id, _| keep.contains(id) || mine.contains(id));
    w.scouting.dept.looks.shrink_to_fit();
}
