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
    /// The program and the first day of the job whose recruiting file you
    /// took over (`inherit`), so it is taken over once.
    #[serde(default)]
    pub inherited: Option<(u32, Date)>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Followed {
    pub id: u32,
    pub since: Date,
    pub why: String,
    pub first_read: String,
    pub public_then: String,
}

/// The NCAA's evaluation-day limits for the whole staff, fall and spring
/// (minor bug B3): 42 fall days from September to November and 168 in the
/// spring (Bylaw 13.17; 56 in the fall of 2021), then from August 1, 2023
/// (Proposal 2023-14) 33 fall days and 140 recruiting-person days from
/// April 15. The service academies get 54 and 216, then 42 and 180. FCS
/// shares the FBS numbers; Division II and below use them too (proposed,
/// not confirmed).
pub fn eval_limits(season: i32, academy: bool) -> (u16, u16) {
    match (season, academy) {
        (2023.., false) => (33, 140),
        (2023.., true) => (42, 180),
        (2021, false) => (56, 168),
        (_, false) => (42, 168),
        (_, true) => (54, 216),
    }
}

/// The coaches who may recruit off campus, the head coach included (minor
/// bug B5): at FBS the head coach and nine assistants, ten from January 9,
/// 2018 when the tenth assistant came; at FCS eleven, then thirteen from
/// July 1, 2023 (Proposal 2022-28; the date proposed, not confirmed). From
/// June 25, 2024 any staff member may coach, but the recruiters stay at 11
/// and 13, and the service academies get four more (the date proposed, not
/// confirmed). None below Division I here.
pub fn off_campus_recruiters(prog: &crate::world::program::Program, d: Date) -> Option<u32> {
    use crate::world::data::Division;
    let base = match prog.division {
        Division::FBS => {
            if d >= Date::ymd(2018, 1, 9) {
                11
            } else {
                10
            }
        }
        Division::FCS => {
            if d >= Date::ymd(2023, 7, 1) {
                13
            } else {
                11
            }
        }
        _ => return None,
    };
    let academy = service_academy(prog) && d >= Date::ymd(2024, 6, 25);
    Some(base + if academy { 4 } else { 0 })
}

/// Army, Navy and Air Force: the NCAA gives the service academies more
/// evaluation days.
pub fn service_academy(prog: &crate::world::program::Program) -> bool {
    matches!(prog.real_name.as_str(), "Army" | "Navy" | "Air Force")
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

/// NFL: your assignments on top of the built year, each Wednesday from
/// September to April. A scout on a player sees him every second week
/// (fall school calls, spring film); on a position group he writes film
/// reports on its top names, five a week; on a region he makes its school
/// calls in the fall and studies its prospects' film after
/// (`pro::scouts::region_week`); on cross-checks he takes three a week from
/// the top of your board that he hasn't seen himself (minor bug 265). The
/// days he spends on them are holes in his own region's fall calls
/// (`pro::scouts::fall`). The director follows your focus.
fn nfl_day(w: &mut World, t: usize, d: Date) {
    let (_, m, _) = d.parts();
    let season_on = matches!(m, 9..=12 | 1..=4);
    if !season_on || d.weekday() != Weekday::Wed {
        return;
    }
    let assignments = w.scouting.dept.assignments.clone();
    let week = d.0 / 7;
    let scouts = w.clubs.get(t).map_or(0, |c| c.scouts.len());
    for a in assignments {
        let who = a.who;
        // A scout's place in your department (a guard: the May review
        // fills a let-go man's place, and the place keeps its assignment).
        if who >= 250 || who as usize >= scouts {
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
            Target::Region(r) => crate::world::pro::scouts::region_week(w, t, who as u8, r, d, a.since),
            Target::CrossCheck => cross_check_week(w, t, who as u8, d),
        }
    }
    // F21 "Small schools too": your area scouts add six film looks a week
    // on the best prospects below the FBS nobody of yours has seen.
    if w.scouting.dept.instructions.contains(&Instruction::SmallSchools) {
        let draft_year = if w.today <= w.dates.draft[2].date { w.year } else { w.year + 1 };
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

/// Your scout on cross-checks (minor bug 265): a second look at the top of
/// your board (`board::seat_grades`: your staff's grades, the public's read
/// where you have no file yet), three a week on three days, at men he
/// hasn't seen himself. The director's looks are his own kind.
fn cross_check_week(w: &mut World, t: usize, k: u8, d: Date) {
    use crate::world::pro::scouts::{LookKind, Role};
    let Some(role) = w.clubs.get(t).and_then(|c| c.scouts.get(k as usize)).map(|s| s.role) else { return };
    let grades = super::board::seat_grades(w);
    let files = &w.clubs[t].files;
    let mut top: Vec<(f64, u32)> = grades
        .into_iter()
        .filter(|(id, _)| !files.get(id).is_some_and(|f| f.iter().any(|l| l.who == k)))
        .map(|(id, g)| (g, id))
        .collect();
    top.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let kind = if role == Role::Director { LookKind::Director } else { LookKind::CrossCheck };
    for (j, &(_, id)) in top.iter().take(3).enumerate() {
        club_look(w, t, id, k, kind, d.add(j as i32));
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
    let draft_year = if w.today <= w.dates.draft[2].date { w.year } else { w.year + 1 };
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
            let away: Vec<u32> = w
                .scouting
                .dept
                .assignments
                .iter()
                .filter(|a| (PERSONNEL..PERSONNEL + staff.len() as u32).contains(&a.who))
                .map(|a| a.who)
                .collect();
            film_slice(w, prog, d, &board, staff.len(), hudl, &away);
            assigned_film(w, prog, d, &board, staff.len(), hudl);
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
        let spring = matches!(d.parts().1, 4 | 5) && matches!(period, Period::Contact | Period::Evaluation);
        if (fall && (wd == Weekday::Thu || wd == Weekday::Fri)) || (spring && wd == Weekday::Thu) {
            let (lim_fall, lim_spring) = eval_limits(season, service_academy(&w.programs[prog]));
            let (_, used_f, used_s) = w.scouting.dept.eval_days;
            let left = if fall { lim_fall.saturating_sub(used_f) } else { lim_spring.saturating_sub(used_s) };
            // How many days a week the staff spends (the fall's days spread
            // over its 12 weeks; the spring's over six).
            let per = if fall { ((lim_fall as f64) / 24.0).ceil() as u16 } else { 14 };
            let today = left.min(per);
            if today > 0 {
                // Seen this recruiting year: since the fall's first day
                // (the spring window reads back to last fall; before, it
                // read from the coming fall and saw no one, so the spring
                // picked the fall's kids again).
                let seen_since = recruiting_year(d);
                let seen = |w: &World, i: usize| {
                    w.scouting.dept.looks.get(&w.people[i].player.id).is_some_and(|v| {
                        v.iter()
                            .any(|l| matches!(l.kind, Kind::Practice | Kind::Game | Kind::Camp) && l.date >= seen_since)
                    })
                };
                // Your coaches on assignments go first, one place a day
                // each (minor bug 265): his player; the best recruit of his
                // state or his position group nobody of yours has seen in
                // person this season; or, on cross-checks, the top of the
                // board another coach has seen and he hasn't. A coach no
                // longer on the staff goes nowhere, and with more coaches
                // out than the day's quota they take turns.
                let mut picks: Vec<(u32, usize)> = Vec::new();
                let mut sent: Vec<Assignment> = w
                    .scouting
                    .dept
                    .assignments
                    .iter()
                    .filter(|a| a.who >= COACH && on_staff(&w.programs[prog], (a.who - COACH) as usize))
                    .cloned()
                    .collect();
                if !sent.is_empty() {
                    let used = if fall { used_f } else { used_s } as usize;
                    let n = sent.len();
                    sent.rotate_left(used % n);
                }
                for a in sent {
                    let open =
                        |w: &World, i: usize, picks: &[(u32, usize)]| !seen(w, i) && !picks.iter().any(|p| p.1 == i);
                    let pick = match a.target {
                        Target::Player(id) => super::place(w, id)
                            .filter(|&i| matches!(w.people[i].stage, Stage::HighSchool | Stage::PrepSchool)),
                        Target::Region(st) => recruits_where(w, prog, |q| q.home_state as u8 == st)
                            .into_iter()
                            .find(|&i| open(w, i, &picks)),
                        Target::Group(g) => {
                            recruits_where(w, prog, |q| crate::world::coach::Group::of(q.player.position) as u8 == g)
                                .into_iter()
                                .find(|&i| open(w, i, &picks))
                        }
                        Target::CrossCheck => cross_check_list(w, &board, a.who, Some(seen_since))
                            .into_iter()
                            .find(|&i| !picks.iter().any(|p| p.1 == i)),
                    };
                    if let Some(i) = pick {
                        picks.push((a.who, i));
                    }
                }
                let out: Vec<u32> = picks.iter().map(|p| p.0).collect();
                let asks: Vec<u32> =
                    w.scouting.dept.asks.iter().filter(|a| a.1 == Kind::Practice).map(|a| a.0).collect();
                for id in asks {
                    if let Some(i) = super::place(w, id)
                        && let Some(c) = evaluator_for(w, prog, i)
                    {
                        picks.push((COACH + c as u32, i));
                    }
                }
                for &i in &board {
                    if picks.len() >= today as usize {
                        break;
                    }
                    if w.people[i].stage == Stage::College {
                        continue;
                    }
                    // A coach out on your assignment is not here to see him.
                    if !seen(w, i)
                        && !picks.iter().any(|p| p.1 == i)
                        && let Some(c) = evaluator_for(w, prog, i)
                        && !out.contains(&(COACH + c as u32))
                    {
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
    // for the board's recruits within a day's drive whose families can pay
    // (minor bug 22: as the world's camps, `times::camps_wanted`, a family
    // with no money comes at 0.3 the rate). The times are stored that day
    // (minor bug 23).
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
            let can = 0.45 * (0.3 + 0.7 * q.bg.money);
            if near && readers::chance(Staff::Program(prog), 0xCA4B ^ d.0 as u64, q.player.id, can) {
                let who = evaluator_for(w, prog, i).or(hc).map_or(PERSONNEL, |c| COACH + c as u32);
                let (id, kicker) = (q.player.id, q.player.position.is_specialist());
                add(w, id, who, Kind::Camp, d);
                if !kicker {
                    crate::world::times::your_camp(w, i, prog, d);
                }
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

/// One Monday's film study by the personnel staff: a quarter of the board
/// each week, shared out among the `n` staffers.
fn film_slice(w: &mut World, prog: usize, d: Date, board: &[usize], n: usize, hudl: bool, away: &[u32]) {
    let slice = (board.len() / 4).max(1);
    let k = (d.0 / 7) as usize % 4;
    for (j, &i) in board.iter().enumerate().skip(k * slice).take(slice) {
        let who = PERSONNEL + (j % n) as u32;
        // A staffer on one of your assignments leaves his share undone.
        if away.contains(&who) {
            continue;
        }
        film_look(w, prog, d, i, who, hudl);
    }
}

/// One film look by a staffer. Before Hudl a small school's tape was what
/// a coach mailed: it gets through 35% of the time.
fn film_look(w: &mut World, prog: usize, d: Date, i: usize, who: u32, hudl: bool) {
    let id = w.people[i].player.id;
    let small = w.high_schools.get(w.people[i].hs).is_some_and(|h| h.classification <= 2);
    if !hudl && small && !readers::chance(Staff::Program(prog), 0x7A9E ^ d.0 as u64, id, 0.35) {
        return;
    }
    let kind = if w.people[i].stage == Stage::College { Kind::CollegeFilm } else { Kind::HsFilm };
    add(w, id, who, kind, d);
}

/// Your personnel staffers on assignments (minor bug 265): on Mondays each
/// studies film on his target instead of his share of the board, as many
/// tapes as his share: one player's latest tape; a state's recruits or a
/// position group's, the best first and on from where he left off; or, on
/// cross-checks, the top of the board someone else has seen and he hasn't.
fn assigned_film(w: &mut World, prog: usize, d: Date, board: &[usize], n: usize, hudl: bool) {
    let share = ((board.len() / 4).max(1)).div_ceil(n.max(1)).max(1);
    for a in w.scouting.dept.assignments.clone() {
        if !(PERSONNEL..PERSONNEL + n as u32).contains(&a.who) {
            continue;
        }
        let week = ((d.0 - a.since.0).max(0) / 7) as usize;
        let list: Vec<usize> = match a.target {
            Target::Player(id) => super::place(w, id).into_iter().collect(),
            Target::Region(st) => recruits_where(w, prog, |q| q.home_state as u8 == st),
            Target::Group(g) => {
                recruits_where(w, prog, |q| crate::world::coach::Group::of(q.player.position) as u8 == g)
            }
            Target::CrossCheck => cross_check_list(w, board, a.who, None),
        };
        if list.is_empty() {
            continue;
        }
        let rotate = matches!(a.target, Target::Region(_) | Target::Group(_));
        let take = if matches!(a.target, Target::Player(_)) { 1 } else { share.min(list.len()) };
        for j in 0..take {
            let i = if rotate { list[(week * take + j) % list.len()] } else { list[j] };
            film_look(w, prog, d, i, a.who, hudl);
        }
    }
}

/// The recruits of the next two classes who pass `f`, ranked as the board
/// ranks them (yours first, then stars and how near), beyond the board's
/// cut: an assignment is how a staff finds the kids its board misses.
fn recruits_where(w: &World, prog: usize, f: impl Fn(&crate::world::Person) -> bool) -> Vec<usize> {
    let p = &w.programs[prog];
    let season = w.dates.season;
    let mut scored: Vec<(f64, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, q)| {
            matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)
                && q.class_of > season
                && q.class_of <= season + 2
                && q.player.age >= 15
                && f(q)
        })
        .map(|(i, q)| {
            let ours = q.cal.offers.contains(&prog)
                || q.cal.visits.iter().any(|v| v.0 == prog)
                || q.cal.commit.is_some_and(|c| c.0 == prog);
            let near = p.footprint[q.home_state.min(50)] + 0.5 * p.pipeline[q.home_state.min(50)];
            (if ours { 100.0 } else { 0.0 } + q.stars as f64 * 2.0 + near * 3.0, i)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.truncate(400);
    scored.into_iter().map(|x| x.1).collect()
}

/// The top of the board seen by someone else of yours and not by `who`:
/// in person since `in_person` (a coach's cross-check, this recruiting
/// year) or by any look (a staffer's film cross-check).
fn cross_check_list(w: &World, board: &[usize], who: u32, in_person: Option<Date>) -> Vec<usize> {
    let counts = |l: &SeatLook| {
        in_person.is_none_or(|since| matches!(l.kind, Kind::Practice | Kind::Game | Kind::Camp) && l.date >= since)
    };
    board
        .iter()
        .copied()
        .filter(|&i| matches!(w.people[i].stage, Stage::HighSchool | Stage::PrepSchool))
        .filter(|&i| {
            w.scouting.dept.looks.get(&w.people[i].player.id).is_some_and(|v| {
                v.iter().any(|l| l.who != who && counts(l)) && !v.iter().any(|l| l.who == who && counts(l))
            })
        })
        .collect()
}

/// The recruiting year a day belongs to starts with the fall's evaluation
/// period: September 1 of this year from September, else of last year.
fn recruiting_year(d: Date) -> Date {
    let (y, m, _) = d.parts();
    Date::ymd(if m >= 9 { y } else { y - 1 }, 9, 1)
}

/// Whether a coach is on a program's staff now: head coach, a coordinator
/// or a position coach.
fn on_staff(p: &crate::world::program::Program, c: usize) -> bool {
    p.head_coach == Some(c) || p.oc == Some(c) || p.dc == Some(c) || p.position_coaches.contains(&Some(c))
}

/// The first day of your present job (the career's first day for the job
/// you started in).
pub fn job_start(w: &World) -> Option<Date> {
    let c = w.career.as_ref()?;
    Some(c.market.job_since.unwrap_or(c.started))
}

/// A new staff takes over the program's recruiting file (minor bug 248,
/// Cain 2026-11-04: an inherited signee showed no staff read at all). The
/// file belongs to the program, and the personnel staff who keep it stay
/// through a coaching change, so you walk into what the department had:
/// eight weeks of its Monday film study on the board; film on every
/// recruit the program offered, had commit or signed, from his first
/// contact and (signed) his signing day; the visits he took, with the head
/// coach of the day; for a committed or signed recruit the fall evaluation
/// in person by his position coach and, as a senior, his transcript and
/// social media. Looks are dated before your first day, so reports are
/// written; your own read still starts with your own looks. Once per job
/// (a save from before takes it over at load).
pub(crate) fn inherit(w: &mut World) {
    let Some(Staff::Program(prog)) = readers::seat_staff(w) else { return };
    let Some(since) = job_start(w) else { return };
    let key = Some((prog as u32, since));
    if w.scouting.dept.inherited == key {
        return;
    }
    let same = w.scouting.dept.inherited.is_some_and(|(p, _)| p == prog as u32);
    w.scouting.dept.inherited = key;
    // A promotion at the same program keeps the file you already worked
    // in; a save loaded long after the job began keeps what it has.
    let worked = same && w.scouting.dept.looks.values().any(|v| v.iter().any(|l| l.date >= since.add(-60)));
    if worked || since.days_to(w.today) > 60 {
        return;
    }
    if !w.programs[prog].plays(w.year) && !w.programs[prog].plays(w.year + 1) {
        return;
    }
    let you = readers::you(w);
    let n = readers::personnel(w, prog).len();
    let board = watch(w, prog);
    if n > 0 {
        let mut d = since.add(-1).on_or_before(Weekday::Mon);
        for _ in 0..8 {
            film_slice(w, prog, d, &board, n, d.year() >= 2008, &[]);
            d = d.add(-7);
        }
    }
    let staffer = |k: usize| PERSONNEL + if n == 0 { 0 } else { (k % n) as u32 };
    use crate::world::coach::{Employer, Role};
    // The carousel's season label: a day before fall camp belongs to the
    // season before (`staff::cycle_year`).
    let label = |d: Date| if d.month() < 8 { d.year() - 1 } else { d.year() };
    // A coach's first season in his job (older saves: the one after his
    // hiring label).
    let first = |j: &crate::world::coach::Job| if j.first > 0 { j.first } else { j.since + 1 };
    // Who had the program's head job on a day, other than you: the sitting
    // head coach from his first season, or one whose stint covered it.
    let head_on = |w: &World, d: Date| -> u32 {
        let season = label(d);
        let e = Employer::College(prog);
        w.coaches
            .iter()
            .enumerate()
            .find(|(c, k)| {
                Some(*c) != you
                    && (k
                        .job
                        .as_ref()
                        .is_some_and(|j| j.employer == e && j.role == Role::HeadCoach && first(j) <= season)
                        || k.history.iter().any(|s| {
                            s.employer == e && s.role == Role::HeadCoach && s.from <= season && season <= s.to
                        }))
            })
            .map_or(staffer(0), |(c, _)| COACH + c as u32)
    };
    // His position coach if he was on the staff then, else the head coach.
    let evaluator_on = |w: &World, i: usize, d: Date| -> u32 {
        let g = crate::world::coach::Group::of(w.people[i].player.position) as usize;
        match w.programs[prog].position_coaches[g] {
            Some(c)
                if Some(c) != you
                    && w.coaches[c]
                        .job
                        .as_ref()
                        .is_some_and(|j| j.employer == Employer::College(prog) && first(j) <= label(d)) =>
            {
                COACH + c as u32
            }
            _ => head_on(w, d),
        }
    };
    let last_fall = |d: Date| if d >= Date::ymd(d.year(), 10, 15) { d.year() } else { d.year() - 1 };
    let mut adds: Vec<(u32, u32, Kind, Date)> = Vec::new();
    for (i, q) in w.people.iter().enumerate() {
        let c = &q.cal;
        let signed = c.signed.is_some_and(|s| s.program == prog);
        let committed = c.commit.is_some_and(|x| x.0 == prog);
        let early = c.early_offers.iter().filter(|o| o.0 == prog).map(|o| o.1).min();
        let visits: Vec<(Date, crate::world::Visit)> =
            c.visits.iter().filter(|v| v.0 == prog && v.1 < since).map(|v| (v.1, v.2)).collect();
        let ours = signed || committed || early.is_some() || c.offers.contains(&prog) || !visits.is_empty();
        let recruit = matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)
            || (q.stage == Stage::College && q.college != Some(prog) && (signed || committed));
        if !ours || !recruit {
            continue;
        }
        let id = q.player.id;
        let film = if q.stage == Stage::College { Kind::CollegeFilm } else { Kind::HsFilm };
        // Film from his first contact with the program.
        let first = [early, visits.iter().map(|v| v.0).min(), c.commit.filter(|x| x.0 == prog).map(|x| x.1)]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(since);
        adds.push((id, staffer(0), film, first.min(since.add(-45))));
        if let Some(s) = c.signed.filter(|s| s.program == prog && s.date < since) {
            adds.push((id, staffer(1), film, s.date));
        }
        for (d, v) in visits {
            let kind = match v {
                crate::world::Visit::Official => Kind::Official,
                crate::world::Visit::Unofficial => Kind::Unofficial,
                crate::world::Visit::DropIn => Kind::Practice,
                crate::world::Visit::Home => Kind::HomeVisit,
            };
            adds.push((id, head_on(w, d), kind, d));
        }
        if (signed || committed) && q.stage != Stage::College {
            let fall = last_fall(since).min(q.class_of - 1);
            if fall >= q.class_of - 2 {
                let d = Date::nth_weekday(fall, 10, Weekday::Fri, 2);
                adds.push((id, evaluator_on(w, i, d), Kind::Game, d));
            }
            if q.class_of - 1 <= last_fall(since) {
                let d = Date::ymd(q.class_of - 1, 10, 1);
                adds.push((id, readers::ACADEMIC, Kind::Transcript, d));
                adds.push((id, staffer(0), Kind::Social, d));
            }
        }
    }
    for (id, who, kind, d) in adds {
        add(w, id, who, kind, d);
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
