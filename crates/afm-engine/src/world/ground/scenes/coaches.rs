//! The coaches' side (`ground/week.md`, "The scenes"): your clinic (80 to
//! 300 high school coaches in your meeting room and at practice; the talk
//! afterwards is where the lists come out), open practice (coaches on your
//! sideline) and your state's coaches' convention in July (you speak, and
//! the hallway is where coaches talk). Each is an event with every coach
//! there (`standing::Contact::Event`, +3 each, the spec's table), the
//! clinic's times the scheme fit (1.0 to 1.5): the coaches who run what
//! you talk warm fastest.
//!
//! The dates come from each year's spring practice (`calendar::anchors`):
//! the clinic is the Friday of its third week (week 12 in 2026, the
//! months sheet), open practices the Tuesday of every other week of it,
//! the convention the Tuesday after the third Monday of July (week 30).
//! The calendar has no practice-period model for 2027 yet, so spring
//! practice's own dates hold every year (proposed, not confirmed).

use super::Kind;
use super::{LEAVE, State, put_doc, put_state, state};
use crate::calendar::{Date, Weekday};
use crate::world::World;
use crate::world::folk::scene::{Scene, Who as SWho};
use crate::world::ground::people::{self, Who};
use crate::world::ground::standing::{self, Contact, Level};
use crate::world::ground::week::{self, Slot, SlotKind, beat};
use crate::world::ground::{areas, leads};

/// Your clinic's crowd: 80 at the smallest program, 300 at the biggest
/// (the round file's range), by prestige.
const CLINIC: (f64, f64) = (80.0, 300.0);
/// How far coaches drive to a college's clinic.
const CLINIC_MILES: f64 = 400.0;
/// Coaches on your sideline at an open practice, by prestige (proposed,
/// not confirmed).
const SIDELINE: (f64, f64) = (8.0, 30.0);
/// How far a coach drives for an afternoon's practice.
const SIDELINE_MILES: f64 = 150.0;
/// An open practice is two hours on a sideline, not a day with you: half
/// an event (proposed, not confirmed).
const SIDELINE_SCALE: f32 = 0.5;
/// The convention: a share of your state's head coaches come, and your
/// session draws a share of those, more at a bigger program (proposed,
/// not confirmed).
const CONVENTION_COMES: f64 = 0.55;
const SESSION: (f64, f64) = (0.25, 0.75);
/// The talk on recruiting at your place lands with everyone a little
/// (proposed, not confirmed): a scheme talk lands 1.0 to 1.5 by how well
/// he knows the scheme.
const RECRUITING_FIT: f32 = 1.15;
/// Coaches you get time with in the hallway.
const HALLWAY: usize = 5;
/// On the sideline, the chance a coach brought his best junior along
/// (proposed, not confirmed).
const BROUGHT: f64 = 0.25;

const OFFENSE: u32 = 101;
const DEFENSE: u32 = 102;
const RECRUITING: u32 = 103;
const HALL: u32 = 110;
const WALK: u32 = 111;

// --- The dates ----------------------------------------------------------------------------------

/// Your clinic's day in the football year holding `d` (the Friday of the
/// third week of spring practice).
pub fn clinic_day(w: &World, _prog: usize, d: Date) -> Option<Date> {
    let y = beat::dates_for(w, d);
    let c = y.spring_practice.0.on_or_before(Weekday::Mon).add(18);
    (c <= y.spring_practice.1).then_some(c)
}

/// The open practices: the Tuesday of each week of spring practice but the
/// clinic's week and the spring game's.
fn open_days(w: &World, prog: usize, d: Date) -> Vec<Date> {
    let y = beat::dates_for(w, d);
    let clinic = clinic_day(w, prog, d).map(|c| c.on_or_before(Weekday::Mon));
    let game = y.spring_game.on_or_before(Weekday::Mon);
    let mut out = Vec::new();
    let mut mon = y.spring_practice.0.on_or_before(Weekday::Mon);
    while mon <= y.spring_practice.1 {
        let tue = mon.add(1);
        if Some(mon) != clinic && mon != game && tue >= y.spring_practice.0 {
            out.push(tue);
        }
        mon = mon.add(7);
    }
    out
}

/// Your state's convention: the Tuesday after the third Monday of July.
pub fn convention_day(season: i32) -> Date {
    Date::nth_weekday(season, 7, Weekday::Mon, 3).add(1)
}

/// The week's coaches' scenes as slots, ranked for the meeting at the map
/// (`week::plan`): the clinic and the spring game 7 (a day on your
/// campus), the convention 5, an open practice 2 (an afternoon you can
/// leave to your staff).
pub(super) fn slots(w: &World, prog: usize, monday: Date) -> Vec<(u8, Slot)> {
    let slot = |date: Date, minute: u16, kind: SlotKind, title: String, reason: &str| Slot {
        date,
        minute,
        kind,
        title,
        reason: reason.to_string(),
        yours: false,
        man: None,
        who: None,
        school: None,
        line: None,
    };
    let end = monday.add(7);
    let within = |d: Date| d >= monday && d < end;
    let mut v = Vec::new();
    if let Some(c) = clinic_day(w, prog, monday).filter(|&c| within(c)) {
        let n = clinic_size(w, prog);
        v.push((
            7,
            slot(
                c,
                8 * 60,
                SlotKind::Clinic,
                format!("Your clinic: about {n} coaches"),
                "A step with many coaches at once, fastest with those who run your scheme",
            ),
        ));
    }
    for d in open_days(w, prog, monday).into_iter().filter(|&d| within(d)) {
        v.push((2, slot(d, 16 * 60, SlotKind::OpenPractice, "Open practice".into(), "Coaches on your sideline")));
    }
    let y = beat::dates_for(w, monday);
    if within(y.spring_game) && super::holds_spring_game(w, y.spring_game.year()) {
        v.push((
            7,
            slot(
                y.spring_game,
                12 * 60,
                SlotKind::SpringGame,
                "The spring game".into(),
                "Recruits in the stands and coaches in the suite",
            ),
        ));
    }
    let conv = convention_day(y.season);
    if within(conv) {
        v.push((
            5,
            slot(
                conv,
                9 * 60,
                SlotKind::Convention,
                format!("The {} coaches' convention", state_name(w, prog)),
                "Standing with coaches across the state",
            ),
        ));
    }
    v
}

fn state_name(w: &World, prog: usize) -> &'static str {
    let code = crate::world::geo::STATE_CENTERS[w.programs[prog].state.min(50)].0;
    crate::world::folk::facts::state_name(code)
}

fn prestige(w: &World, prog: usize) -> f64 {
    w.programs.get(prog).map_or(0.5, |p| p.prestige).clamp(0.0, 1.0)
}

fn clinic_size(w: &World, prog: usize) -> usize {
    (CLINIC.0 + (CLINIC.1 - CLINIC.0) * prestige(w, prog)).round() as usize
}

// --- Who comes ----------------------------------------------------------------------------------

/// High school head coaches near your campus: the ones who know you first
/// (Home, then Trusted, then Known), then the nearest; `known_only` keeps
/// only Known and up.
pub(super) fn nearby(w: &World, prog: usize, miles: f64, known_only: bool) -> Vec<Who> {
    let home = areas::campus(w, prog);
    let mut v: Vec<(u8, f64, Who)> = Vec::new();
    for (h, s) in w.high_schools.iter().enumerate() {
        let Some(c) = s.coach else { continue };
        let m = crate::world::geo::miles_between(home, areas::school_place(w, h));
        if m > miles {
            continue;
        }
        let who = Who::Coach(c as u32);
        let lv = standing::level(w, prog, who);
        if known_only && lv == Level::Stranger {
            continue;
        }
        v.push((lv as u8, m, who));
    }
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)));
    v.into_iter().map(|x| x.2).collect()
}

/// The coaches at a scene: the ones you asked to the clinic always come;
/// the rest by how well they know you and how far they drive.
fn crowd(w: &World, prog: usize, k: Kind, d: Date) -> Vec<Who> {
    match k {
        Kind::Clinic => {
            let season = beat::dates_for(w, d).season;
            let mut v: Vec<Who> = w.ground.scenes.invited.iter().filter(|x| x.1 == season).map(|x| x.0).collect();
            for c in nearby(w, prog, CLINIC_MILES, false) {
                if v.len() >= clinic_size(w, prog) {
                    break;
                }
                if !v.contains(&c) {
                    v.push(c);
                }
            }
            v
        }
        Kind::OpenPractice => {
            let n = (SIDELINE.0 + (SIDELINE.1 - SIDELINE.0) * prestige(w, prog)).round() as usize;
            // A different few each week: the nearest who know you first,
            // then a keyed draw through the rest.
            let mut v = nearby(w, prog, SIDELINE_MILES, false);
            let seed = w.options.seed;
            v.sort_by_key(|&c| {
                let lv = standing::level(w, prog, c) as u8;
                (
                    std::cmp::Reverse(lv.min(1)),
                    (crate::world::ground::unit(seed, &[0x51DE, super::who_code(c), d.0 as u64]) * 1e6) as u64,
                )
            });
            v.truncate(n);
            v
        }
        Kind::Convention => {
            let st = w.programs[prog].state;
            let share = SESSION.0 + (SESSION.1 - SESSION.0) * prestige(w, prog);
            let seed = w.options.seed;
            w.high_schools
                .iter()
                .filter(|s| s.state == st)
                .filter_map(|s| s.coach)
                .map(|c| Who::Coach(c as u32))
                .filter(|&c| {
                    let u = crate::world::ground::unit(seed, &[0xC0E7, super::who_code(c), d.year() as u64]);
                    u < CONVENTION_COMES * share
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// How well a coach knows what you talk (0-1): your installed offense, or
/// your front and coverage together.
fn knows(w: &World, prog: usize, who: Who, topic: u8) -> f64 {
    let Who::Coach(c) = who else { return 0.0 };
    let Some(coach) = w.coaches.get(c as usize) else { return 0.0 };
    let k = coach.system_know();
    let inst = &w.programs[prog].installed;
    match topic {
        1 => k[inst.off.min(4)],
        2 => 0.5 * (k[5 + inst.front.min(1)] + k[7 + inst.shell.min(3)]),
        _ => 0.0,
    }
}

/// The scheme fit of a talk with a coach (the spec's 1.0 to 1.5).
fn fit(w: &World, prog: usize, who: Who, topic: u8) -> f32 {
    match topic {
        1 | 2 => 1.0 + 0.5 * knows(w, prog, who, topic).clamp(0.0, 1.0) as f32,
        3 => RECRUITING_FIT,
        _ => 1.0,
    }
}

fn offense_words(w: &World, prog: usize) -> &'static str {
    w.programs[prog].installed.offense().name()
}

fn defense_words(w: &World, prog: usize) -> String {
    let inst = &w.programs[prog].installed;
    let f = match inst.front() {
        crate::game::Front::FourThree => "4-3",
        crate::game::Front::ThreeFour => "3-4",
    };
    let s = match inst.shell() {
        crate::game::Shell::Cover3 => "Cover 3",
        crate::game::Shell::Quarters => "quarters",
        crate::game::Shell::ManPressure => "man and pressure",
        crate::game::Shell::Cover2 => "Cover 2",
    };
    format!("your {f} and {s}")
}

// --- The scene ----------------------------------------------------------------------------------

pub(super) fn setup(w: &mut World, prog: usize, s: &mut Scene, k: Kind) -> bool {
    let people = crowd(w, prog, k, s.date);
    if people.is_empty() {
        return false;
    }
    s.place = match k {
        Kind::Clinic => "Your football building: the meeting room, then practice".into(),
        Kind::OpenPractice => "Your practice field, the sideline".into(),
        _ => format!("The {} coaches' convention, a ballroom and its hallway", state_name(w, prog)),
    };
    s.who = people
        .iter()
        .take(4)
        .filter_map(|c| match c {
            Who::Coach(c) => Some(SWho::Coach(*c as usize)),
            _ => None,
        })
        .collect();
    let st = State { people, ..State::default() };
    put_state(s, &st);
    true
}

pub(super) fn open(w: &mut World, prog: usize, s: &mut Scene, k: Kind) {
    let st = state(s);
    let n = st.people.len();
    let known = st.people.iter().filter(|&&c| standing::level(w, prog, c) >= Level::Known).count();
    let off = st.people.iter().filter(|&&c| knows(w, prog, c, 1) >= 0.5).count();
    let def = st.people.iter().filter(|&&c| knows(w, prog, c, 2) >= 0.5).count();
    let invited = st.people.iter().filter(|c| w.ground.scenes.invited.iter().any(|x| x.0 == **c)).count();
    match k {
        Kind::Clinic => {
            let mut t = format!(
                "Your clinic. {n} high school coaches in the meeting room at 8, {known} of them who know you. {off} run something close to the {} and {def} play {}.",
                offense_words(w, prog),
                defense_words(w, prog)
            );
            if invited > 0 {
                t.push_str(&format!(" {invited} came because you asked them."));
            }
            t.push_str(" Your hour on the stage, then practice, then the hallway.");
            s.narrate(t);
        }
        Kind::OpenPractice => {
            let names: Vec<String> = st.people.iter().take(3).map(|&c| people::name(w, c)).collect();
            s.narrate(format!(
                "Open practice. {n} high school coaches on your sideline at 4: {}{}. Two hours of periods, and the walk to the parking lot after.",
                names.join(", "),
                if n > 3 { format!(" and {} more", n - 3) } else { String::new() }
            ));
        }
        _ => {
            s.narrate(format!(
                "The {} coaches' convention. {n} head coaches from your state in your session's ballroom, {known} of them who know you. You speak at 9, and the hallway after is where coaches talk.",
                state_name(w, prog)
            ));
        }
    }
    let rows: Vec<(Option<u32>, Vec<String>)> = st
        .people
        .iter()
        .filter(|&&c| standing::level(w, prog, c) >= Level::Known)
        .take(40)
        .map(|&c| {
            (
                None,
                vec![
                    people::name(w, c),
                    people::school(w, c).map(|h| w.school_name(h).to_string()).unwrap_or_default(),
                    super::level_words(w, prog, c).to_string(),
                ],
            )
        })
        .collect();
    put_doc(s, "Who came", "The coaches who know you", &["Coach", "School", "With you"], rows);
    picks(s, k, &st);
}

fn picks(s: &mut Scene, k: Kind, st: &State) {
    s.picks.clear();
    if k == Kind::OpenPractice {
        if !st.stops.iter().any(|x| x.0 == "walk") {
            s.pick(
                WALK,
                "Walk them to the parking lot",
                "Ask about their kids",
                "warm",
                "The ones who know you, and their lists",
            );
        }
        s.pick(LEAVE, "Back to the staff room", "Done", "neutral", "");
        return;
    }
    if st.topic == 0 {
        s.pick(OFFENSE, "Talk your offense", "The stage", "warm", "The coaches who run it warm fastest");
        s.pick(DEFENSE, "Talk your defense", "The stage", "warm", "The coaches who play it warm fastest");
        s.pick(
            RECRUITING,
            "Talk recruiting at your place",
            "The stage",
            "warm",
            "How you evaluate, and how a kid gets an offer",
        );
        return;
    }
    if !st.stops.iter().any(|x| x.0 == "hall") {
        s.pick(HALL, "Work the hallway", "Ask about their kids", "warm", "Lists from the coaches who know you");
    }
    s.pick(LEAVE, "Head home", "Done", "neutral", "");
}

pub(super) fn answer(w: &mut World, prog: usize, s: &mut Scene, k: Kind, code: u32) {
    let mut st = state(s);
    let me = super::you(w);
    let d = s.date;
    let (eye, tired) = super::your_eye(w);
    match code {
        OFFENSE | DEFENSE | RECRUITING if st.topic == 0 && k != Kind::OpenPractice => {
            st.topic = (code - 100) as u8;
            let line = talk(w, prog, &mut st, me, k, d);
            s.narrate(line.clone());
            st.stops.push(("talk".into(), line));
        }
        HALL if k != Kind::OpenPractice => {
            let line = hallway(w, prog, s, &mut st, me, eye, tired, d);
            s.narrate(line.clone());
            st.stops.push(("hall".into(), line));
        }
        WALK if k == Kind::OpenPractice => {
            sideline(w, prog, &mut st, me, d);
            let line = hallway(w, prog, s, &mut st, me, eye, tired, d);
            s.narrate(line.clone());
            st.stops.push(("walk".into(), line));
        }
        LEAVE => {
            if k == Kind::OpenPractice && !st.stops.iter().any(|x| x.0 == "walk") {
                sideline(w, prog, &mut st, me, d);
            }
            if st.topic == 0 && k != Kind::OpenPractice {
                // Your hour on the stage happens either way: the talk you
                // always give, recruiting at your place.
                st.topic = 3;
                let line = talk(w, prog, &mut st, me, k, d);
                st.stops.push(("talk".into(), line));
            }
            finish(w, prog, s, k, &mut st);
            put_state(s, &st);
            return;
        }
        _ => {}
    }
    put_state(s, &st);
    picks(s, k, &st);
}

/// Your hour on the stage: an event with every coach in the room, times
/// the fit (the clinic: how well he knows what you talked); the ones who
/// run it stay to ask more.
fn talk(w: &mut World, prog: usize, st: &mut State, man: Option<usize>, k: Kind, d: Date) -> String {
    let topic = st.topic;
    let people = st.people.clone();
    let mut stayed = 0;
    for &c in &people {
        let f = if k == Kind::Clinic { fit(w, prog, c, topic) } else { 1.0 };
        super::touch(w, prog, st, man, c, Contact::Event(f), 1.0, d);
        if topic != 3 && knows(w, prog, c, topic) >= 0.5 {
            stayed += 1;
        }
    }
    let what = match topic {
        1 => format!("the {}", offense_words(w, prog)),
        2 => defense_words(w, prog),
        _ => "how you evaluate and how a kid earns an offer".to_string(),
    };
    let ups = st.levels.len();
    let mut line = format!("You talk {what} for an hour.");
    if topic == 3 {
        line.push_str(" Every coach in the room wants to know how his kids get seen.");
    } else if stayed > 0 {
        line.push_str(&format!(" {stayed} who run it stay at the front to ask more."));
    } else {
        line.push_str(" Few in the room run it; they take notes and go to lunch.");
    }
    if ups > 0 {
        line.push_str(&format!(" {ups} {} a step closer.", if ups == 1 { "coach is" } else { "coaches are" }));
    }
    line
}

/// The sideline: an event with each coach there (half a clinic day), and
/// what your eye catches at the fence.
fn sideline(w: &mut World, prog: usize, st: &mut State, man: Option<usize>, d: Date) {
    if st.stops.iter().any(|x| x.0 == "sideline") {
        return;
    }
    let people = st.people.clone();
    for &c in &people {
        let f = fit(w, prog, c, if prog_offense_first(w, prog) { 1 } else { 2 });
        super::touch(w, prog, st, man, c, Contact::Event(f), SIDELINE_SCALE, d);
    }
    let (eye, tired) = super::your_eye(w);
    let mut seen = Vec::new();
    for &c in &people {
        let Some(h) = people::school(w, c) else { continue };
        if crate::world::ground::unit(w.options.seed, &[0xB207, super::who_code(c), d.0 as u64]) >= BROUGHT {
            continue;
        }
        // His best junior, on the walk over with his coach.
        let Some(i) = best_junior(w, prog, h) else { continue };
        let id = w.people[i].player.id;
        if !super::notice(w, eye, 0.3, tired, &[0xFE2C, id as u64, d.0 as u64]) {
            continue;
        }
        let said = format!(
            "\"{} brought him to watch practice; you saw him at the fence during individual periods.\"",
            super::last(&people::name(w, c))
        );
        if leads::seen(w, man.map(|m| m as u32), id, d, said) {
            seen.push(id);
            st.kids.push(id);
            st.leads.push(id);
        }
    }
    let line = if seen.is_empty() {
        "The coaches watch your periods and take their notes.".to_string()
    } else {
        format!(
            "Your eye catches {} at the fence with {}: new on your board.",
            week::road::names(w, &seen),
            if seen.len() == 1 { "his coach" } else { "their coaches" }
        )
    };
    st.stops.push(("sideline".into(), line));
}

/// Which side the sideline sees best: the side you coach.
fn prog_offense_first(w: &World, prog: usize) -> bool {
    let inst = &w.programs[prog].installed;
    inst.off_known >= inst.def_known
}

fn best_junior(w: &mut World, prog: usize, h: usize) -> Option<usize> {
    let season = w.dates.season;
    let roster: Vec<usize> = w.high_schools.get(h).map(|s| s.roster.clone()).unwrap_or_default();
    let mut best: Option<(f64, usize)> = None;
    for i in roster {
        let Some(q) = w.people.get(i) else { continue };
        if q.stage != crate::world::Stage::HighSchool || q.class_of != season + 1 {
            continue;
        }
        if q.cal.offers.contains(&prog) || leads::lead(w, q.player.id).is_some() {
            continue;
        }
        if let Some(pct) = leads::worth(w, prog, i)
            && best.is_none_or(|b| pct > b.0)
        {
            best = Some((pct, i));
        }
    }
    best.map(|b| b.1)
}

/// The hallway (or the walk to the lot): the coaches who know you give
/// their lists, the best schools on your board first; your eye catches who
/// is selling a name and who sells short (his straightness, the tips'
/// reading).
#[allow(clippy::too_many_arguments)]
fn hallway(
    w: &mut World,
    prog: usize,
    s: &mut Scene,
    st: &mut State,
    me: Option<usize>,
    eye: f64,
    tired: f64,
    d: Date,
) -> String {
    let mut talked: Vec<Who> =
        st.people.iter().copied().filter(|&c| standing::level(w, prog, c) >= Level::Known).collect();
    talked.truncate(HALLWAY);
    if talked.is_empty() {
        // Nobody knows you yet: the first ones who walk up.
        talked = st.people.iter().copied().take(2).collect();
    }
    let before: Vec<u32> = w.ground.leads.iter().map(|l| l.kid).collect();
    let mut reads = Vec::new();
    for &c in &talked {
        if let Some(h) = people::school(w, c) {
            leads::coach_list(w, prog, me.map(|m| m as u32), h, d);
        }
        if people::wants(w, c).contains(&"to be asked about his kids") {
            super::wanted(w, prog, c, "You asked about my kids. Most of them don't.", d);
        }
        let mk = people::makeup(w, c);
        if super::notice(w, eye, 0.5, tired, &[0x4A11, super::who_code(c), d.0 as u64]) {
            let n = super::last(&people::name(w, c));
            if mk.straight < 9.0 {
                reads.push(format!("{n} sells his top name hard; you'd want the film first."));
            } else if mk.straight >= 15.0 {
                reads.push(format!("{n} talks his kids down; his second name is worth a look."));
            }
        }
    }
    let new: Vec<&leads::Lead> = w
        .ground
        .leads
        .iter()
        .filter(|l| !before.contains(&l.kid) && talked.iter().any(|c| l.from == Some(*c)))
        .collect();
    let ids: Vec<u32> = new.iter().map(|l| l.kid).collect();
    let rows: Vec<(Option<u32>, Vec<String>)> = new
        .iter()
        .filter_map(|l| {
            let i = crate::world::scouting::place(w, l.kid)?;
            let p = &w.people[i];
            Some((
                Some(l.kid),
                vec![
                    p.name.clone(),
                    p.player.position.name().to_string(),
                    w.school_name(p.hs).to_string(),
                    l.from.map(|c| people::name(w, c)).unwrap_or_default(),
                ],
            ))
        })
        .collect();
    if !rows.is_empty() {
        put_doc(
            s,
            "The hallway's names",
            "The lists the coaches gave you",
            &["Name", "Position", "School", "From"],
            rows,
        );
    }
    for id in &ids {
        if !st.kids.contains(id) {
            st.kids.push(*id);
        }
        if !st.leads.contains(id) {
            st.leads.push(*id);
        }
    }
    let mut line = if ids.is_empty() {
        format!(
            "{} {} you; no new names today.",
            talked.len(),
            if talked.len() == 1 { "coach talks with" } else { "coaches talk with" }
        )
    } else {
        format!(
            "{} {} you, and their lists give you {} new {}: {}.",
            talked.len(),
            if talked.len() == 1 { "coach talks with" } else { "coaches talk with" },
            ids.len(),
            if ids.len() == 1 { "name" } else { "names" },
            week::road::names(w, &ids)
        )
    };
    for r in reads {
        line.push(' ');
        line.push_str(&r);
    }
    line
}

fn finish(w: &mut World, prog: usize, s: &mut Scene, k: Kind, st: &mut State) {
    let n = st.people.len();
    let ups = st.levels.len();
    let homes = st.levels.iter().filter(|x| x.2 == Level::Home as u8).count();
    let what = match k {
        Kind::Clinic => format!("Your clinic: {n} coaches"),
        Kind::OpenPractice => format!("Open practice: {n} coaches on the sideline"),
        _ => format!("The {} convention: {n} coaches in your session", state_name(w, prog)),
    };
    let mut parts = vec![what];
    if ups > 0 {
        parts.push(format!(
            "{ups} a step closer{}",
            if homes > 0 { format!(" ({homes} now Home)") } else { String::new() }
        ));
    }
    if !st.leads.is_empty() {
        parts.push(format!(
            "{} new {} ({})",
            st.leads.len(),
            if st.leads.len() == 1 { "name" } else { "names" },
            week::road::names(w, &st.leads.iter().take(3).copied().collect::<Vec<_>>())
        ));
    }
    let line = format!("{}.", parts.join(", "));
    s.narrate(line.clone());
    s.outcome = vec![line];
    s.end();
}

/// Handed to your staff: your coordinator gives the talk on his own side,
/// works the hallway on his skill and eye, one line (GR32).
pub(super) fn hand(w: &mut World, prog: usize, s: &mut Scene, k: Kind) {
    let mut st = state(s);
    let man = week::coordinator(w, prog).or_else(|| super::you(w));
    let d = s.date;
    let eye = man.map_or(10.0, |c| super::man_eye(w, c));
    if k == Kind::OpenPractice {
        sideline(w, prog, &mut st, man, d);
    } else if st.topic == 0 {
        // His side of the ball.
        st.topic = match man.and_then(|c| w.coaches.get(c)).map(|c| c.job.map(|j| j.role)) {
            Some(Some(crate::world::coach::Role::DefensiveCoordinator)) => 2,
            Some(Some(crate::world::coach::Role::OffensiveCoordinator)) => 1,
            _ => 3,
        };
        let line = talk(w, prog, &mut st, man, k, d);
        st.stops.push(("talk".into(), line));
    }
    if !st.stops.iter().any(|x| x.0 == "hall" || x.0 == "walk") {
        let line = hallway(w, prog, s, &mut st, man, eye, 0.0, d);
        st.stops.push(("hall".into(), line));
    }
    finish(w, prog, s, k, &mut st);
    if let Some(c) = man {
        let who = w.coaches[c].name.clone();
        if let Some(l) = s.outcome.first_mut() {
            *l = format!("{} ({} had it)", l.trim_end_matches('.'), super::last(&who)) + ".";
        }
    }
    put_state(s, &st);
}
