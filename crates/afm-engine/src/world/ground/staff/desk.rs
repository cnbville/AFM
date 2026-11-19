//! The college assistant seat (GR69, GR78; staff.md, "The assistant
//! seat"): your area and your position group are yours, the rest is your
//! head coach's. His Monday plan gives you road days and you plan your own
//! route within them. You bring him up to two kids a week on the offer
//! chain from the other side: your side's coordinator watches the film
//! first, and he signs or says wait by his own read, the open spots and his
//! trust in your eye. A disagreement goes to his office. A kid he signs
//! while the world rates him low is your find; one he says wait on isn't
//! (GR66), but he, the high school coach you told and their ties remember,
//! and his trust in your eye grows as the world catches up. He gives you
//! goals for your area and your group at the February meeting, you pick
//! your own ground goal, and his August verdict counts when he reviews his
//! staff. When someone courts you, he can counter.
//!
//! Sizes are the build's picks, proposed, not confirmed: his trust starts
//! at 0.35 (0.5 if he knew you before), each bloom step is worth 0.04, a
//! settled call three years on 0.06; pushing back wins about 0.2 + half
//! his trust.

use super::{count, memo};
use crate::calendar::{Date, Weekday};
use crate::world::coach::{Employer, Group, Role};
use crate::world::ground::areas;
use crate::world::ground::people::{self, Holder, Who};
use crate::world::ground::standing::{self, Level};
use crate::world::ground::{Memory, Moment, word};
use crate::world::scouting::readers::{self, COACH, Staff};
use crate::world::scouting::{self, Kind, dept};
use crate::world::{Stage, World};

/// The most kids you bring him a week (GR69).
pub const PER_WEEK: usize = 2;
/// A bloom step's worth to his trust in your eye.
pub const STEP: f32 = 0.04;
/// A call settled three years on.
pub const SETTLED: f32 = 0.06;

/// One kid you brought him.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Push {
    pub id: u32,
    pub date: Date,
    /// His verdict comes once the coordinator's film look is written.
    pub ready: Date,
    /// Your case, in your words.
    pub case: String,
    /// The coordinator who watched his film, and his word from it.
    pub film: Option<u32>,
    pub film_word: Option<String>,
    /// 0 waiting, 1 he signed it (the offer went out), 2 he said wait,
    /// 3 the kid went elsewhere first.
    pub verdict: u8,
    pub reason: String,
    /// You pushed back in his office: whether he came around.
    pub pushed_back: Option<bool>,
    /// You let it ride.
    pub let_ride: bool,
    /// His stars and the programs that had offered him the day you pushed.
    pub stars: u8,
    pub had: Vec<u32>,
    /// Bloom steps seen (bit 0 a level-up offer, bit 1 his stars rose).
    pub steps: u8,
    /// Three years on: whether your eye was right.
    pub settled: Option<bool>,
    /// He's your find (GR66): signed while the world rated him low.
    pub find: bool,
}

/// One of your goals (GR69: his two, your own).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Goal {
    /// 1 your area, 2 your group, 3 your own ground goal.
    pub kind: u8,
    pub text: String,
    pub target: u16,
    pub got: u16,
    /// Areas it reads, or your group.
    pub areas: Vec<u16>,
    pub group: Option<u8>,
    /// For a ground goal: 0 Trusted with more head coaches, 1 Known at the
    /// biggest schools, 2 a find.
    pub own: Option<u8>,
    pub met: Option<bool>,
}

/// The August verdict (GR69).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Verdict {
    pub season: i32,
    /// Goals met less goals missed.
    pub score: i8,
    pub lines: Vec<String>,
}

/// The assistant's desk, on `StaffGround.desk`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Desk {
    pub program: Option<u32>,
    /// Your head coach's trust in your eye, 0-1.
    pub trust: f32,
    pub pushes: Vec<Push>,
    pub season: i32,
    pub goals: Vec<Goal>,
    /// The ground goals you can pick from this year.
    pub choices: Vec<Goal>,
    pub verdict: Option<Verdict>,
    /// His Monday plan for you: (Monday, days, area).
    pub given: Option<(Date, u8, u16)>,
    /// Jobs offered to you he has answered: (employer, role, the day).
    pub countered: Vec<(Employer, Role, Date)>,
}

/// Your assistant seat: the program, your coach id and your head coach.
pub fn seat(w: &World) -> Option<(usize, usize, usize)> {
    let prog = super::seat_program(w)?;
    if super::you_are_head(w) {
        return None;
    }
    let you = w.career.as_ref()?.coach;
    let hc = w.programs[prog].head_coach?;
    (hc != you).then_some((prog, you, hc))
}

/// Your position group (a coordinator's side counts its groups).
pub fn your_group(w: &World) -> Option<Group> {
    match w.career.as_ref()?.role? {
        Role::PositionCoach(g) => Some(g),
        _ => None,
    }
}

fn side_groups(w: &World) -> Vec<Group> {
    match w.career.as_ref().and_then(|c| c.role) {
        Some(Role::PositionCoach(g)) => vec![g],
        Some(Role::OffensiveCoordinator) => Group::ALL.iter().copied().filter(|g| g.offense()).collect(),
        Some(Role::DefensiveCoordinator) => {
            Group::ALL.iter().copied().filter(|g| !g.offense() && *g != Group::Specialists).collect()
        }
        _ => Vec::new(),
    }
}

/// His memory of you (GR69: your boss is a person with one memory of you).
fn boss_memory(w: &mut World, hc: usize, you: usize, d: Date) -> &mut Memory {
    let who = Who::Coach(hc as u32);
    let holder = Holder::Coach(you as u32);
    w.ground.entry(who, holder, || {
        let mut m = Memory::new(who, holder, 40.0, d);
        m.contacts = 3;
        m.known_since = Some(d);
        m
    })
}

fn remember(w: &mut World, hc: usize, you: usize, d: Date, words: String, good: bool, by: f32) {
    let m = boss_memory(w, hc, you, d);
    m.keep(Moment { date: d, words, good, big: false, holds_to: None });
    standing::gain(m, by, d);
    m.last = d;
}

/// His side's coordinator (the film first).
fn film_man(w: &World, prog: usize, g: Group) -> Option<usize> {
    let p = &w.programs[prog];
    match g {
        Group::Specialists => p.position_coaches[g as usize],
        _ if g.offense() => p.oc,
        _ => p.dc,
    }
    .or(p.head_coach)
}

/// The kids at a position on your program's board, read by its staff:
/// the line the open spots draw and the offers already out.
fn line_at(w: &World, prog: usize, pos: crate::positions::Position) -> (f64, usize, usize) {
    let staff = Staff::Program(prog);
    let sign_year = crate::world::recruiting::next_class(w);
    let mut reads: Vec<f64> = Vec::new();
    let mut out = 0usize;
    for i in dept::watch(w, prog) {
        let p = &w.people[i];
        if p.player.position != pos || !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
            continue;
        }
        if p.class_of > sign_year + 1 || p.cal.signed.is_some_and(|s| s.program != prog) {
            continue;
        }
        if p.cal.offers.contains(&prog) || p.cal.early_offers.iter().any(|o| o.0 == prog) {
            out += 1;
        }
        if let Some((_, _, f)) = readers::read(w, staff, p.player.id) {
            reads.push(f.now.0);
        }
    }
    reads.sort_by(|a, b| b.total_cmp(a));
    let need = crate::world::recruiting::class_outlook(w, prog).need[pos.index()];
    let per = match w.programs[prog].division {
        crate::world::data::Division::FBS => 4.0,
        crate::world::data::Division::FCS => 3.0,
        _ => 2.5,
    };
    let cap = ((per * need as f64).round() as usize).max(1);
    let line = reads.get(cap.min(reads.len()).saturating_sub(1)).copied().unwrap_or(0.0);
    (line, cap, out)
}

/// The best stars a program has a real chance at.
fn reach(prestige: f64) -> u8 {
    (2.0 + (3.0 * prestige).ceil()).clamp(3.0, 5.0) as u8
}

/// You bring your head coach a kid (GR69, up to two a week): from your
/// area or your position group, with your case.
pub fn push(w: &mut World, id: u32, case: &str) -> Result<String, String> {
    let (prog, you, _) = seat(w).ok_or("Your list is the college assistant's.")?;
    ensure(w, prog, you);
    let d = w.today;
    let monday = d.on_or_before(Weekday::Mon);
    let this_week = w.ground.staff.desk.pushes.iter().filter(|p| p.date >= monday).count();
    if this_week >= PER_WEEK {
        return Err(format!("You've brought him {PER_WEEK} this week. Next Monday."));
    }
    if w.ground.staff.desk.pushes.iter().any(|p| p.id == id && p.verdict == 0) {
        return Err("He has that one already.".into());
    }
    let i = scouting::place(w, id).ok_or("No such player.")?;
    let q = &w.people[i];
    if !matches!(q.stage, Stage::HighSchool | Stage::PrepSchool) {
        return Err("He isn't a high school recruit.".into());
    }
    if q.cal.signed.is_some() || q.cal.commit.is_some_and(|c| c.0 != prog) {
        return Err("He's taken.".into());
    }
    if q.cal.offers.contains(&prog) || q.cal.early_offers.iter().any(|o| o.0 == prog) {
        return Err("He has your offer already.".into());
    }
    let area = w.high_schools.get(q.hs).map(|s| s.ground_area);
    let mine = super::beats::his_areas(w, prog, you).unwrap_or_default();
    let g = Group::of(q.player.position);
    if !area.is_some_and(|a| mine.contains(&a)) && !side_groups(w).contains(&g) {
        return Err("He's outside your area and your group: that's another man's to bring.".into());
    }
    let had: Vec<u32> =
        q.cal.offers.iter().map(|&x| x as u32).chain(q.cal.early_offers.iter().map(|o| o.0 as u32)).collect();
    let stars = q.stars;
    let name = q.name.clone();
    let film = film_man(w, prog, g);
    if let Some(c) = film {
        let kind = Kind::HsFilm;
        dept::add(w, id, COACH + c as u32, kind, d);
    }
    let case = if case.trim().is_empty() {
        format!("{name}: I've seen him, and I want him offered.")
    } else {
        case.trim().to_string()
    };
    w.ground.staff.desk.pushes.push(Push {
        id,
        date: d,
        ready: d.add(2),
        case,
        film: film.map(|c| c as u32),
        film_word: None,
        verdict: 0,
        reason: String::new(),
        pushed_back: None,
        let_ride: false,
        stars,
        had,
        steps: 0,
        settled: None,
        find: false,
    });
    let who = film.map_or_else(|| "the staff".to_string(), |c| super::last(w, c));
    Ok(format!("{name} is on his desk. {who} watches the film first; you'll have his word in two days."))
}

/// His verdict on a push, once the film is in.
fn decide(w: &mut World, prog: usize, you: usize, hc: usize, k: usize, d: Date) {
    let x = w.ground.staff.desk.pushes[k].clone();
    let Some(i) = scouting::place(w, x.id) else {
        w.ground.staff.desk.pushes[k].verdict = 3;
        return;
    };
    let q = &w.people[i];
    if q.cal.signed.is_some_and(|s| s.program != prog) || q.cal.commit.is_some_and(|c| c.0 != prog) {
        w.ground.staff.desk.pushes[k].verdict = 3;
        w.ground.staff.desk.pushes[k].reason = "He committed elsewhere before the film was in.".into();
        return;
    }
    let pos = q.player.position;
    let name = q.name.clone();
    let stars = q.stars;
    let class_of = q.class_of;
    let (line, cap, out) = line_at(w, prog, pos);
    let read = readers::read(w, Staff::Program(prog), x.id).map(|(_, _, f)| (f.now.0, f.now.1.max(2.0)));
    let trust = w.ground.staff.desk.trust;
    let (gap, film_word) = match read {
        Some((c, sd)) => {
            let g = (c - line) / sd;
            let word = if g >= 0.5 {
                "He's better on tape than his stars say."
            } else if g >= -0.3 {
                "Tape's good enough. He's a fit if there's room."
            } else {
                "I don't see it on tape yet."
            };
            (g, word.to_string())
        }
        None => (-0.5, "The film's thin: one look isn't enough to say.".to_string()),
    };
    let score = gap + 2.0 * (trust as f64 - 0.4);
    let sign_year = crate::world::recruiting::next_class(w);
    let (verdict, reason) = if out >= cap {
        (2, format!("No room at {pos}: we have {} out for the spots we've got.", count(out, "offer", "offers")))
    } else if stars > reach(w.programs[prog].prestige) {
        (2, "A kid rated like that won't look at us. Spend your time where we can win.".to_string())
    } else if score >= 0.0 {
        (1, format!("I'll sign it. {name} gets our offer."))
    } else if class_of > sign_year {
        (2, "Wait for his junior tape.".to_string())
    } else if trust < 0.4 {
        (2, "Bring me more on him. You haven't been right for me yet.".to_string())
    } else {
        (2, format!("I don't see it yet: he's under our {pos} line."))
    };
    let p = &mut w.ground.staff.desk.pushes[k];
    p.film_word = Some(film_word);
    p.reason = reason.clone();
    p.verdict = verdict;
    if verdict == 1 {
        let low = stars <= 3 && !level_up_offer(w, prog, &x.had, x.id);
        offer(w, prog, i, d);
        let p = &mut w.ground.staff.desk.pushes[k];
        p.find = low;
        remember(w, hc, you, d, format!("I went with you on {name}."), true, 1.0);
    }
    let subject = format!("{name}: {}", if verdict == 1 { "he signed it" } else { "he says wait" });
    memo(w, d, 7 * 60 + 15, hc, subject, vec![reason], 1);
}

/// The offer goes out from your program (his signature).
fn offer(w: &mut World, prog: usize, i: usize, d: Date) {
    let school = w.programs[prog].real_name.clone();
    let p = &mut w.people[i];
    if !p.cal.offers.contains(&prog) {
        p.cal.offers.push(prog);
    }
    if !p.cal.early_offers.iter().any(|o| o.0 == prog) {
        p.cal.early_offers.push((prog, d));
    }
    let age = p.row_age();
    p.note(format!("At {age} he got an offer from {school}."));
}

/// A program at your level or higher (division and name) offered him since.
fn level_up_offer(w: &World, prog: usize, had: &[u32], id: u32) -> bool {
    let Some(i) = scouting::place(w, id) else { return false };
    let q = &w.people[i];
    let mine = &w.programs[prog];
    let rank = |d: crate::world::data::Division| crate::world::folk::tips::rank(d);
    q.cal.offers.iter().copied().chain(q.cal.early_offers.iter().map(|o| o.0)).any(|j| {
        j != prog
            && !had.contains(&(j as u32))
            && rank(w.programs[j].division) >= rank(mine.division)
            && w.programs[j].prestige > mine.prestige
    })
}

/// You push back in his office (GR69): a scene until lane G5 makes it one;
/// his answer comes now.
pub fn push_back(w: &mut World, id: u32) -> Result<String, String> {
    let (prog, you, hc) = seat(w).ok_or("Your list is the college assistant's.")?;
    let d = w.today;
    let k = w
        .ground
        .staff
        .desk
        .pushes
        .iter()
        .position(|p| p.id == id && p.verdict == 2 && p.pushed_back.is_none())
        .ok_or("Nothing of yours waits on his word.")?;
    let trust = w.ground.staff.desk.trust;
    let chance = (0.2 + 0.5 * trust as f64).clamp(0.05, 0.8);
    let won = crate::world::front_office::key(w, 0xB0_55BA, id as u64 ^ ((d.0 as u64) << 20)) < chance;
    let name = scouting::place(w, id).map_or_else(String::new, |i| w.people[i].name.clone());
    w.ground.staff.desk.pushes[k].pushed_back = Some(won);
    let boss = super::last(w, hc);
    if won {
        let x = w.ground.staff.desk.pushes[k].clone();
        if let Some(i) = scouting::place(w, id) {
            let low = x.stars <= 3 && !level_up_offer(w, prog, &x.had, id);
            offer(w, prog, i, d);
            let p = &mut w.ground.staff.desk.pushes[k];
            p.verdict = 1;
            p.find = low;
        }
        remember(w, hc, you, d, format!("You went to the mat for {name}. We'll see."), true, 0.5);
        Ok(format!("{boss} hears you out, then signs it. \"Your name's on this one.\""))
    } else {
        remember(w, hc, you, d, format!("You argued for {name}. I said no."), false, 0.0);
        Ok(format!("{boss} holds: \"I heard you. The answer's still wait.\""))
    }
}

/// You let his wait ride.
pub fn let_ride(w: &mut World, id: u32) -> Result<String, String> {
    seat(w).ok_or("Your list is the college assistant's.")?;
    let p = w
        .ground
        .staff
        .desk
        .pushes
        .iter_mut()
        .find(|p| p.id == id && p.verdict == 2 && p.pushed_back.is_none() && !p.let_ride)
        .ok_or("Nothing of yours waits on his word.")?;
    p.let_ride = true;
    Ok("You let it ride. If the world catches up on him, the people who saw you push will remember.".into())
}

/// Your pick of a ground goal (GR69).
pub fn pick_goal(w: &mut World, k: usize) -> Result<String, String> {
    seat(w).ok_or("Your goals are the college assistant's.")?;
    let desk = &mut w.ground.staff.desk;
    if desk.goals.iter().any(|g| g.kind == 3) {
        return Err("You've picked your ground goal for this year.".into());
    }
    let g = desk.choices.get(k).cloned().ok_or("No such goal.")?;
    let text = g.text.clone();
    desk.goals.push(g);
    Ok(format!("Your own goal: {text}."))
}

/// The desk belongs to this job.
fn ensure(w: &mut World, prog: usize, you: usize) {
    if w.ground.staff.desk.program == Some(prog as u32) {
        return;
    }
    let hc = w.programs[prog].head_coach;
    let knew = hc.is_some_and(|h| w.career.as_ref().is_some_and(|c| c.known.iter().any(|k| k.coach == h)));
    w.ground.staff.desk =
        Desk { program: Some(prog as u32), trust: if knew { 0.5 } else { 0.35 }, ..Default::default() };
    let _ = you;
}

/// His Monday plan for you (lane G3's draft): your days and your area.
pub(crate) fn on_turn(w: &mut World, monday: Date) {
    let Some((_, you, _)) = seat(w) else { return };
    let row = w.ground.week.roads.iter().find(|r| r.man == you as u32);
    w.ground.staff.desk.given = Some((monday, row.map_or(0, |r| r.days.count_ones() as u8), row.map_or(0, |r| r.area)));
}

/// Whether you may set a road row as the assistant: only your own, on
/// your areas, within his days.
pub(crate) fn check_road(w: &World, man: u32, area: u16, bits: u8) -> Result<(), String> {
    let Some((prog, you, _)) = seat(w) else { return Ok(()) };
    if man != you as u32 {
        return Err("Your head coach plans the other men's days.".into());
    }
    let mine = super::beats::his_areas(w, prog, you).unwrap_or_default();
    if !mine.is_empty() && !mine.contains(&area) {
        return Err("That's not your area: your head coach set yours at the February meeting.".into());
    }
    let given = w.ground.staff.desk.given.map_or(0, |g| g.1);
    if bits.count_ones() > given as u32 {
        return Err(format!("His plan gives you {}.", count(given as usize, "road day", "road days")));
    }
    Ok(())
}

/// You plan your own route (GR69): the schools you go to first on your days.
pub fn route(w: &mut World, schools: &[u32]) -> Result<String, String> {
    let (prog, you, _) = seat(w).ok_or("Your route is the college assistant's.")?;
    crate::world::ground::week::plan::ensure_now(w, prog);
    let row = w.ground.week.roads.iter_mut().find(|r| r.man == you as u32).ok_or("His plan has you home this week.")?;
    let area = row.area;
    let keep: Vec<u32> = schools
        .iter()
        .copied()
        .filter(|&h| w.high_schools.get(h as usize).is_some_and(|s| s.ground_area == area))
        .collect();
    if keep.is_empty() {
        return Err(format!("Pick schools in {}.", areas::area(area).nickname));
    }
    row.focus = keep.clone();
    row.yours = true;
    let names: Vec<String> = keep.iter().take(4).map(|&h| w.school_name(h as usize).to_string()).collect();
    Ok(format!("Your route: {} first.", names.join(", ")))
}

/// The goals for your area and your group, and the ground goals to pick
/// from, at the February meeting (GR69).
fn set_goals(w: &mut World, prog: usize, you: usize, hc: usize, d: Date) {
    let season = w.dates.season;
    let mine = super::beats::his_areas(w, prog, you).unwrap_or_default();
    let next = crate::world::recruiting::next_class(w);
    // What your areas have sent the program over the last three classes.
    let from_area = w
        .people
        .iter()
        .filter(|q| q.cal.signed.is_some_and(|s| s.program == prog) || q.college == Some(prog))
        .filter(|q| q.class_of >= next - 3 && q.class_of < next)
        .filter(|q| w.high_schools.get(q.hs).is_some_and(|s| mine.contains(&s.ground_area)))
        .count();
    let n_area = ((from_area as f64 / 3.0).round() as u16).clamp(1, 4);
    let mut goals = Vec::new();
    if !mine.is_empty() {
        let names: Vec<&str> = mine.iter().map(|&a| areas::area(a).nickname).collect();
        goals.push(Goal {
            kind: 1,
            text: format!("Land {} from {}", count(n_area as usize, "commit", "commits"), names.join(" and ")),
            target: n_area,
            got: 0,
            areas: mine.clone(),
            group: None,
            own: None,
            met: None,
        });
    }
    if let Some(g) = your_group(w).or_else(|| side_groups(w).first().copied()) {
        let outlook = crate::world::recruiting::class_outlook(w, prog);
        let need: usize = crate::positions::Position::ALL
            .iter()
            .filter(|p| Group::of(**p) == g)
            .map(|p| outlook.need[p.index()])
            .sum();
        let n = (need as u16).clamp(1, 3);
        goals.push(Goal {
            kind: 2,
            text: format!(
                "Land {} at three stars or better in the {}",
                count(n as usize, "commit", "commits"),
                g.name().to_lowercase()
            ),
            target: n,
            got: 0,
            areas: Vec::new(),
            group: Some(g as u8),
            own: None,
            met: None,
        });
    }
    // The ground goals to pick from.
    let mut choices = Vec::new();
    if let Some(&a) = mine.first() {
        let schools: Vec<usize> = (0..w.high_schools.len()).filter(|&h| w.high_schools[h].ground_area == a).collect();
        let (_, t, h) = super::beats::holder_levels(w, Holder::Coach(you as u32), &schools);
        let nick = areas::area(a).nickname;
        choices.push(Goal {
            kind: 3,
            text: format!("Trusted with {} in {nick}", count(t + h + 2, "head coach", "head coaches")),
            target: (t + h + 2) as u16,
            got: (t + h) as u16,
            areas: vec![a],
            group: None,
            own: Some(0),
            met: None,
        });
        choices.push(Goal {
            kind: 3,
            text: format!("Known with the head coaches of {nick}'s ten biggest schools"),
            target: 10,
            got: 0,
            areas: vec![a],
            group: None,
            own: Some(1),
            met: None,
        });
    }
    choices.push(Goal {
        kind: 3,
        text: "A find: a kid you push, rated three stars or less, who gets an offer from a level up by August".into(),
        target: 1,
        got: 0,
        areas: Vec::new(),
        group: None,
        own: Some(2),
        met: None,
    });
    let desk = &mut w.ground.staff.desk;
    desk.season = season;
    desk.goals = goals.clone();
    desk.choices = choices;
    let mut body: Vec<String> = goals.iter().map(|g| format!("{}.", g.text)).collect();
    body.push("Pick your own ground goal; I'll judge the spring in August.".into());
    memo(w, d, 8 * 60, hc, "Your goals this offseason".into(), body, 1);
}

/// How far along each goal is.
fn progress(w: &mut World, prog: usize, you: usize) {
    let next = crate::world::recruiting::next_class(w);
    let goals = w.ground.staff.desk.goals.clone();
    let mut out = goals.clone();
    for (k, g) in goals.iter().enumerate() {
        let got = match (g.kind, g.own) {
            (1, _) => w
                .people
                .iter()
                .filter(|q| q.class_of == next)
                .filter(|q| {
                    q.cal.commit.is_some_and(|c| c.0 == prog) || q.cal.signed.is_some_and(|s| s.program == prog)
                })
                .filter(|q| w.high_schools.get(q.hs).is_some_and(|s| g.areas.contains(&s.ground_area)))
                .count(),
            (2, _) => w
                .people
                .iter()
                .filter(|q| q.class_of == next && q.stars >= 3)
                .filter(|q| Some(Group::of(q.player.position) as u8) == g.group)
                .filter(|q| {
                    q.cal.commit.is_some_and(|c| c.0 == prog) || q.cal.signed.is_some_and(|s| s.program == prog)
                })
                .count(),
            (3, Some(0)) => {
                let schools: Vec<usize> =
                    (0..w.high_schools.len()).filter(|&h| g.areas.contains(&w.high_schools[h].ground_area)).collect();
                let (_, t, h) = super::beats::holder_levels(w, Holder::Coach(you as u32), &schools);
                t + h
            }
            (3, Some(1)) => {
                let talent = standing::talents(w);
                let mut schools: Vec<usize> =
                    (0..w.high_schools.len()).filter(|&h| g.areas.contains(&w.high_schools[h].ground_area)).collect();
                schools.sort_by(|a, b| talent[*b].total_cmp(&talent[*a]).then(a.cmp(b)));
                schools
                    .iter()
                    .take(10)
                    .filter(|&&h| {
                        people::head_coach_of(w, h).is_some_and(|c| {
                            Level::of(standing::points(w, c, Holder::Coach(you as u32))) >= Level::Known
                        })
                    })
                    .count()
            }
            (3, Some(2)) => w.ground.staff.desk.pushes.iter().filter(|p| p.stars <= 3 && p.steps & 1 != 0).count(),
            _ => 0,
        };
        out[k].got = got as u16;
    }
    w.ground.staff.desk.goals = out;
}

/// The August verdict (GR69): each goal met or short, his word on the
/// spring, kept for his staff review.
fn august(w: &mut World, prog: usize, you: usize, hc: usize, d: Date) {
    progress(w, prog, you);
    let season = w.dates.season;
    let mut score: i8 = 0;
    let mut lines = Vec::new();
    let goals = w.ground.staff.desk.goals.clone();
    for g in &goals {
        let met = g.got >= g.target;
        score += if met { 1 } else { -1 };
        lines.push(format!("{}: {} of {}{}", g.text, g.got, g.target, if met { ", done" } else { ", short" }));
    }
    for (k, g) in goals.iter().enumerate() {
        w.ground.staff.desk.goals[k].met = Some(g.got >= g.target);
    }
    let word = match score {
        2.. => "That's the spring I wanted from you. I'll say so when I review the staff.",
        1 => "A good spring. Keep it going.",
        0 => "Some of it landed, some didn't. We'll talk.",
        _ => "I expected more from your ground this spring.",
    };
    lines.insert(0, word.to_string());
    w.ground.staff.desk.verdict = Some(Verdict { season, score, lines: lines.clone() });
    remember(
        w,
        hc,
        you,
        d,
        format!("Your spring: {}", word.to_lowercase()),
        score >= 0,
        if score > 0 { 2.0 } else { 0.0 },
    );
    memo(w, d, 8 * 60, hc, "My word on your spring".into(), lines, 2);
}

/// His August verdict this season, for his staff review (`staff::staff_review`):
/// a man who met his goals is kept more readily, one who missed less so.
pub(crate) fn review_factor(w: &World, c: usize) -> f64 {
    let Some((_, you, _)) = seat(w) else { return 1.0 };
    if c != you {
        return 1.0;
    }
    match w.ground.staff.desk.verdict.as_ref().filter(|v| v.season == w.dates.season || v.season == w.dates.season - 1)
    {
        Some(v) if v.score >= 2 => 0.6,
        Some(v) if v.score == 1 => 0.8,
        Some(v) if v.score <= -2 => 1.6,
        Some(v) if v.score < 0 => 1.3,
        _ => 1.0,
    }
}

/// The world catches up on the kids you pushed (GR69: the bloom's steps):
/// a level-up offer, his stars rising; three years on, the call settles.
fn steps(w: &mut World, prog: usize, you: usize, hc: usize, d: Date) {
    let div = w.programs[prog].division;
    for k in 0..w.ground.staff.desk.pushes.len() {
        let x = w.ground.staff.desk.pushes[k].clone();
        if x.verdict == 0 || x.settled.is_some() {
            continue;
        }
        let Some(i) = scouting::place(w, x.id) else { continue };
        let name = w.people[i].name.clone();
        let mut new_steps = 0u8;
        if x.steps & 1 == 0 && level_up_offer(w, prog, &x.had, x.id) {
            new_steps |= 1;
        }
        if x.steps & 2 == 0 && w.people[i].stars > x.stars {
            new_steps |= 2;
        }
        if new_steps != 0 {
            w.ground.staff.desk.pushes[k].steps |= new_steps;
            let n = new_steps.count_ones() as f32;
            let t = &mut w.ground.staff.desk.trust;
            *t = (*t + STEP * n).min(0.95);
            let what = if new_steps & 1 != 0 { "a bigger program offered him" } else { "the services moved him up" };
            remember(w, hc, you, d, format!("You were on {name} early: {what}."), true, 1.0);
            if x.verdict == 2 {
                told_coach(w, i, &name, d);
            }
        }
        if x.date.add(3 * 365) <= d {
            let (hit, _) = crate::world::front_office::chain::turned(w, x.id, div);
            let t = &mut w.ground.staff.desk.trust;
            *t = (*t + if hit { SETTLED } else { -SETTLED }).clamp(0.05, 0.95);
            w.ground.staff.desk.pushes[k].settled = Some(hit);
        }
    }
    w.ground.staff.desk.pushes.retain(|p| p.date >= d.add(-4 * 365));
}

/// The high school coach you told about a kid your head coach said wait on
/// remembers when the world catches up, and so do his ties (half).
fn told_coach(w: &mut World, i: usize, name: &str, d: Date) {
    let Some(you) = w.career.as_ref().map(|c| c.coach) else { return };
    let Some(coach) = people::head_coach_of(w, w.people[i].hs) else { return };
    let holder = Holder::Coach(you as u32);
    let start = standing::base(w, coach, holder);
    let m = standing::entry(&mut w.ground, start, coach, holder, d);
    let by = 3.0;
    standing::gain(m, by, d);
    m.keep(Moment {
        date: d,
        words: format!("You pushed for {name} before anybody did."),
        good: true,
        big: false,
        holds_to: None,
    });
    word::spread_to(w, holder, coach, by, Some(name.to_string()), d);
}

/// Someone courts you (GR69): he can counter to keep you, by his trust and
/// his verdict.
fn courted(w: &mut World, prog: usize, you: usize, hc: usize, d: Date) {
    let offers: Vec<(Employer, Role)> =
        w.career.as_ref().map_or(Vec::new(), |c| c.market.offers.iter().map(|o| (o.employer, o.role)).collect());
    let trust = w.ground.staff.desk.trust;
    let good = w.ground.staff.desk.verdict.as_ref().is_some_and(|v| v.score >= 1);
    for (e, r) in offers {
        if w.ground.staff.desk.countered.iter().any(|x| x.0 == e && x.1 == r) {
            continue;
        }
        w.ground.staff.desk.countered.push((e, r, d));
        if trust < 0.45 && !good {
            continue;
        }
        let there = super::men::employer_name(w, e);
        let title = crate::world::folk::hiring::title_of(w, you);
        let what = if title.is_none() {
            "he'll make you assistant head coach if you stay"
        } else {
            "he'll put your name first for the next coordinator job if you stay"
        };
        memo(
            w,
            d,
            8 * 60,
            hc,
            format!("{there} called about you"),
            vec![format!("I heard {there} wants you as {}. I want you here: {what}.", r.name())],
            2,
        );
        remember(w, hc, you, d, format!("{there} wanted you. I wanted you more."), true, 1.0);
    }
    // You turned it down and stayed: his counter is kept.
    let still = |e: Employer, r: Role, w: &World| {
        w.career.as_ref().is_some_and(|c| c.market.offers.iter().any(|o| o.employer == e && o.role == r))
    };
    let due: Vec<(Employer, Role, Date)> =
        w.ground.staff.desk.countered.iter().copied().filter(|x| x.2 < d && !still(x.0, x.1, w)).collect();
    for x in due {
        w.ground.staff.desk.countered.retain(|y| *y != x);
        if (trust >= 0.45 || good) && crate::world::folk::hiring::title_of(w, you).is_none() {
            w.folk.hiring.titles.push((you, 2));
            remember(w, hc, you, d, "You stayed. I kept my word.".into(), true, 2.0);
        }
    }
    let _ = prog;
}

/// Each day at the assistant seat: verdicts as the film comes in, the
/// goals at the February meeting, the world's steps on Mondays, the
/// settled calls in September, the August verdict, a counter when you're
/// courted.
pub(crate) fn day(w: &mut World, prog: usize, d: Date) {
    let Some((p, you, hc)) = seat(w) else { return };
    if p != prog {
        return;
    }
    ensure(w, prog, you);
    let due: Vec<usize> = (0..w.ground.staff.desk.pushes.len())
        .filter(|&k| w.ground.staff.desk.pushes[k].verdict == 0 && w.ground.staff.desk.pushes[k].ready <= d)
        .collect();
    for k in due {
        decide(w, prog, you, hc, k, d);
    }
    let season = w.dates.season;
    if d == super::february_meeting(w, d)
        || (w.ground.staff.desk.season != season && w.ground.staff.desk.goals.is_empty())
    {
        set_goals(w, prog, you, hc, d);
    }
    if d.weekday() == Weekday::Mon {
        steps(w, prog, you, hc, d);
        progress(w, prog, you);
    }
    if d.parts().1 == 8 && d.parts().2 == 1 && w.ground.staff.desk.season == season {
        august(w, prog, you, hc, d);
    }
    courted(w, prog, you, hc, d);
}

/// The desk for the screen (GR78).
#[derive(Clone, Debug, serde::Serialize)]
pub struct DeskView {
    pub boss: String,
    pub boss_id: usize,
    pub trust: &'static str,
    pub pushes: Vec<PushView>,
    pub left_this_week: usize,
    pub plan: Option<(Date, u8, String)>,
    pub memory: Vec<(Date, String, bool)>,
    pub level: &'static str,
    pub goals: Vec<Goal>,
    pub choices: Vec<Goal>,
    pub verdict: Option<Verdict>,
    pub group: Option<String>,
    pub areas: Vec<String>,
    pub finds: Vec<String>,
    /// What brings you offers: your record, your group, your ground, who's
    /// courting you.
    pub offers: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PushView {
    pub id: u32,
    pub name: String,
    pub pos: String,
    pub stars: u8,
    pub school: String,
    pub date: Date,
    pub case: String,
    pub film: Option<String>,
    pub film_word: Option<String>,
    pub verdict: &'static str,
    pub reason: String,
    pub can_push_back: bool,
    pub pushed_back: Option<bool>,
    pub find: bool,
}

/// His trust in your eye, in words (never the number).
pub fn trust_words(t: f32) -> &'static str {
    match t {
        x if x >= 0.7 => "he trusts your eye",
        x if x >= 0.5 => "he listens when you push",
        x if x >= 0.35 => "you haven't been right for him yet",
        _ => "he doubts your eye",
    }
}

/// The assistant's desk for the screen.
pub fn view(w: &World) -> Option<DeskView> {
    let (prog, you, hc) = seat(w)?;
    let desk = &w.ground.staff.desk;
    let monday = w.today.on_or_before(Weekday::Mon);
    let pushes = desk
        .pushes
        .iter()
        .rev()
        .take(12)
        .filter_map(|p| {
            let i = scouting::place(w, p.id)?;
            let q = &w.people[i];
            Some(PushView {
                id: p.id,
                name: q.name.clone(),
                pos: q.player.position.to_string(),
                stars: q.stars,
                school: w.high_schools.get(q.hs).map_or_else(String::new, |s| s.name.clone()),
                date: p.date,
                case: p.case.clone(),
                film: p.film.map(|c| super::last(w, c as usize)),
                film_word: p.film_word.clone(),
                verdict: match p.verdict {
                    0 => "waiting",
                    1 => "signed",
                    2 => "wait",
                    _ => "gone",
                },
                reason: p.reason.clone(),
                can_push_back: p.verdict == 2 && p.pushed_back.is_none() && !p.let_ride,
                pushed_back: p.pushed_back,
                find: p.find,
            })
        })
        .collect();
    let mem = w.ground.get(Who::Coach(hc as u32), Holder::Coach(you as u32));
    let memory = mem.map_or(Vec::new(), |m| {
        let mut v: Vec<(Date, String, bool)> = m.moments.iter().map(|x| (x.date, x.words.clone(), x.good)).collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.0));
        v
    });
    let level = Level::of(mem.map_or(40.0, |m| m.points)).name();
    let areas_txt = super::beats::his_areas(w, prog, you)
        .unwrap_or_default()
        .into_iter()
        .map(|a| areas::area(a).nickname.to_string())
        .collect();
    let finds = desk
        .pushes
        .iter()
        .filter(|p| p.find)
        .filter_map(|p| scouting::place(w, p.id).map(|i| w.people[i].name.clone()))
        .collect();
    let mut offers = Vec::new();
    let signed = desk.pushes.iter().filter(|p| p.verdict == 1).count();
    let right = desk.pushes.iter().filter(|p| p.settled == Some(true) || p.steps != 0).count();
    offers.push(format!("Your record: {} signed, {} the world caught up on", count(signed, "push", "pushes"), right));
    if let Some(g) = your_group(w) {
        offers.push(format!("Your group: the {}", g.name().to_lowercase()));
    }
    let (_, t, h) = {
        let schools: Vec<usize> = (0..w.high_schools.len())
            .filter(|&x| {
                super::beats::his_areas(w, prog, you).unwrap_or_default().contains(&w.high_schools[x].ground_area)
            })
            .collect();
        super::beats::holder_levels(w, Holder::Coach(you as u32), &schools)
    };
    offers.push(format!("Your ground: Trusted or better with {}", count(t + h, "head coach", "head coaches")));
    if let Some(c) = w.career.as_ref() {
        for o in &c.market.offers {
            offers.push(format!("{} wants you as {}", super::men::employer_name(w, o.employer), o.role.name()));
        }
    }
    Some(DeskView {
        boss: w.coaches[hc].name.clone(),
        boss_id: hc,
        trust: trust_words(desk.trust),
        pushes,
        left_this_week: PER_WEEK.saturating_sub(desk.pushes.iter().filter(|p| p.date >= monday).count()),
        plan: desk.given.map(|g| (g.0, g.1, areas::area(g.2).nickname.to_string())),
        memory,
        level,
        goals: desk.goals.clone(),
        choices: if desk.goals.iter().any(|g| g.kind == 3) { Vec::new() } else { desk.choices.clone() },
        verdict: desk.verdict.clone(),
        group: your_group(w).map(|g| g.name().to_string()),
        areas: areas_txt,
        finds,
        offers,
    })
}
