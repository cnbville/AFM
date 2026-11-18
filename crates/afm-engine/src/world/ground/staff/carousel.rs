//! Losing a man (GR68, GR76; staff.md, "Losing a man"): the warning when
//! another school's search reaches one of your men and he means to go,
//! your counter in his office (a raise, a title or a promise, tracked),
//! and when he goes: his ties go with him (the memory is keyed by the man),
//! his commits waver (a warning and one call each) and some follow him, and
//! he joins your coaching tree or runs you down. Your men passed over for a
//! coordinator job, and the promise of the next one, are kept here too.
//!
//! Sizes are the build's picks, proposed, not confirmed: three days for
//! his answer; a commit's bond is about a third his recruiter's (`BOND`);
//! about half of the ones who reopen follow him to a school near your level
//! (`FOLLOW`).

use super::{Courted, Left, Promise, Waver, beats, count, last, memo, men};
use crate::calendar::Date;
use crate::rng::clamp;
use crate::world::coach::{Employer, Role};
use crate::world::folk::coach_key;
use crate::world::ground::people::{self, Holder, Who};
use crate::world::ground::standing::{self, Level};
use crate::world::ground::{Memory, word};
use crate::world::{CommitTiming, Stage, World};

/// Days he gives you before he answers.
pub const ANSWER_DAYS: i32 = 3;
/// The share of a commit's bond that is with his recruiter.
pub const BOND: f64 = 0.35;
/// The share of the commits who reopen that follow him, when his new
/// school is near your level.
pub const FOLLOW: f64 = 0.45;
/// Points the people who know a man who left angry hear against you.
pub const RUN_DOWN: f32 = 4.0;

/// Your counter (GR76).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// A raise from your staff money, up to what they offer (30% at most).
    Raise = 1,
    /// The assistant head coach title.
    Title = 2,
    /// Your next coordinator job (a position coach only), tracked.
    Promise = 3,
}

impl Counter {
    pub fn from_key(k: &str) -> Option<Counter> {
        match k {
            "raise" => Some(Counter::Raise),
            "title" => Some(Counter::Title),
            "promise" => Some(Counter::Promise),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Counter::Raise => "raise",
            Counter::Title => "title",
            Counter::Promise => "promise",
        }
    }
}

fn rung(r: Role) -> f64 {
    match r {
        Role::HeadCoach => 2.0,
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => 1.0,
        Role::PositionCoach(_) => 0.0,
    }
}

/// What the offer pulls, in prestige points (the build's own reading of
/// the carousel's parts: the step up, the bigger program, the money, his
/// mood).
fn pull_of(w: &World, prog: usize, man: usize, e: Employer, r: Role, pay: f64) -> f32 {
    let c = &w.coaches[man];
    let now_role = c.job.map_or(Role::PositionCoach(c.came_up), |j| j.role);
    let a = (c.personality.ambition - 10.0) / 10.0;
    let up = (rung(r) - rung(now_role)).max(0.0) * 0.2 * (1.0 + 0.5 * a);
    let bigger = crate::world::staff::employer_prestige(w, e) - w.programs[prog].prestige;
    let now = crate::world::folk::hiring::pay_now(w, man).max(1.0);
    let money = 0.05 * (pay / now).max(0.1).ln();
    let mood = 0.06 * (0.5 - super::men::mood(w, prog, man).value as f64);
    (up + bigger + money + mood).max(0.01) as f32
}

/// His line in the hallway, from what pulls him.
fn hallway(why: &str, r: Role) -> String {
    if r == Role::HeadCoach {
        "A head job, Coach. You know what that's meant to me since I got in this business.".into()
    } else if why.contains("home") {
        "It's home, Coach. My mother's down the road from there.".into()
    } else if why.contains("step up") {
        "It's the next step. I can't keep waiting for it.".into()
    } else if why.contains("raise") {
        "It's real money, Coach. I've got kids to put through school.".into()
    } else if why.contains("overruling") {
        "I need to work somewhere my word counts.".into()
    } else {
        "It's a chance I might not get again, Coach.".into()
    }
}

/// Another school's search reached one of your men and he means to go
/// (`staff::fill_opening`): the warning, and the job waits a few days for
/// your counter. Returns true when it waits.
pub(crate) fn courting(w: &mut World, c: usize, e: Employer, r: Role, prestige: f64, why: &str, pay: f64) -> bool {
    let Some(prog) = super::seat_program(w) else { return false };
    if !super::you_are_head(w) || w.ground.staff.program != Some(prog as u32) || !men(w, prog).contains(&c) {
        return false;
    }
    // A job on your own staff is yours to give (`folk::staffing`).
    if e == Employer::College(prog) {
        return false;
    }
    if w.ground.staff.courted.iter().any(|x| x.man == c as u32 && x.employer == e && x.role == r) {
        return false;
    }
    let d = w.today;
    let pull = pull_of(w, prog, c, e, r, pay);
    let line = hallway(why, r);
    let until = d.add(ANSWER_DAYS);
    w.ground.staff.courted.push(Courted {
        man: c as u32,
        employer: e,
        role: r,
        prestige,
        pay,
        why: why.to_string(),
        made: d,
        until,
        pull,
        counter: None,
        state: 0,
        line: line.clone(),
    });
    let name = w.coaches[c].name.clone();
    let there = super::men::employer_name(w, e);
    let mut body = vec![format!(
        "{there} wants {name} as {}: {why}. He's told them he'll answer by {}.",
        r.name(),
        crate::world::ground::week::beat::short(until)
    )];
    if let Some(es) = w.dates.early_signing
        && es > d
        && d.days_to(es) <= 30
    {
        body.push(format!("The early signing day is {} days away.", d.days_to(es)));
    }
    let m = super::men::mood(w, prog, c);
    if let Some(r0) = m.reasons.first() {
        body.push(format!("What's been weighing on him: {r0}."));
    }
    let leaves = super::men::what_leaves(w, prog, c);
    let shown: Vec<String> = leaves
        .iter()
        .filter(|x| x.1 != x.2)
        .map(|x| format!("{} goes from {} to {}", crate::world::ground::areas::area(x.0).nickname, x.1, x.2))
        .collect();
    if !shown.is_empty() {
        body.push(format!("If he goes: {}.", shown.join("; ")));
    }
    body.push(format!("In the hallway: \"{line}\""));
    body.push("You can counter in your office: a raise, a title or a promise.".into());
    memo(w, d, 9 * 60, c, format!("{there} is after {}", last(w, c)), body, 2);
    true
}

/// The carousel asks about one of your men again: he stays (None), he goes
/// (his reason and the pay), or nothing is settled (the caller weighs him).
pub(crate) fn courted_answer(w: &mut World, c: usize, e: Employer, r: Role) -> Option<Option<(String, f64)>> {
    let k = w.ground.staff.courted.iter().position(|x| x.man == c as u32 && x.employer == e && x.role == r)?;
    if w.ground.staff.courted[k].state == 0 {
        // Asked again before his answer (fall camp's last call): no counter
        // came, so he goes.
        w.ground.staff.courted[k].state = 2;
    }
    let x = &w.ground.staff.courted[k];
    Some(if x.state == 2 { Some((x.why.clone(), x.pay)) } else { None })
}

/// What a counter is worth to him (prestige points) and the raise it
/// takes, before his answer.
fn worth(w: &World, man: usize, x: &Courted, kind: Counter) -> (f64, f64) {
    let co = &w.coaches[man];
    let a = (co.personality.ambition - 10.0) / 10.0;
    let now = crate::world::folk::hiring::pay_now(w, man).max(1.0);
    let share = (x.pay / now - 1.0).clamp(0.05, 0.30);
    let v = match kind {
        Counter::Raise => 0.05 * (1.0 + share).ln() + 0.01,
        Counter::Title => 0.06 + 0.04 * a,
        Counter::Promise => 0.07 * (1.0 + 0.5 * a),
    };
    (if x.role == Role::HeadCoach { v * 0.5 } else { v }, share)
}

/// The chance a counter keeps him (GR76's "how far it reaches").
pub fn reach(w: &World, man: usize, kind: Counter) -> Option<f64> {
    let prog = super::seat_program(w)?;
    let x = w.ground.staff.courted.iter().find(|x| x.man == man as u32 && x.state == 0)?;
    let (v, _) = worth(w, man, x, kind);
    let mood = super::men::mood(w, prog, man).value as f64;
    let loyalty = w.coaches[man].personality.loyalty;
    Some(clamp(0.5 + 4.0 * (v - x.pull as f64) + 0.4 * (mood - 0.5) + 0.02 * (loyalty - 10.0), 0.05, 0.95))
}

/// A counter's reach in words.
pub fn reach_words(p: f64) -> &'static str {
    match p {
        x if x >= 0.75 => "likely to keep him",
        x if x >= 0.55 => "better than even",
        x if x >= 0.4 => "a coin flip",
        x if x >= 0.2 => "a long shot",
        _ => "it won't match this job",
    }
}

/// What a counter costs you besides (its side effect).
pub fn side_effect(w: &World, man: usize, kind: Counter) -> String {
    match kind {
        Counter::Raise => "out of your staff money; the rest of the room hears what he got".into(),
        Counter::Title if crate::world::folk::hiring::title_of(w, man) == Some(2) => {
            "he's your assistant head coach already".into()
        }
        Counter::Title => "the next man you hire will want a title too".into(),
        Counter::Promise if !matches!(w.coaches[man].job.map(|j| j.role), Some(Role::PositionCoach(_))) => {
            "he's a coordinator already".into()
        }
        Counter::Promise => "tracked: break it and his trust goes, and his network hears".into(),
    }
}

/// You counter (GR76): one counter, his answer at once.
pub fn counter(w: &mut World, man: usize, kind: Counter) -> Result<String, String> {
    super::seat_program(w).ok_or("No college staff.")?;
    if !super::you_are_head(w) {
        return Err("Only the head coach can counter.".into());
    }
    let k = w
        .ground
        .staff
        .courted
        .iter()
        .position(|x| x.man == man as u32 && x.state == 0)
        .ok_or("Nobody is after him right now.")?;
    let x = w.ground.staff.courted[k].clone();
    let chance = reach(w, man, kind).unwrap_or(0.5);
    let (_, share) = worth(w, man, &x, kind);
    let d = w.today;
    let said = match kind {
        Counter::Raise => {
            let t = crate::world::folk::staffing::raise(w, man, share)?;
            format!("You raise him {:.0}%. {t}", share * 100.0)
        }
        Counter::Title => {
            if crate::world::folk::hiring::title_of(w, man) == Some(2) {
                return Err("He's your assistant head coach already.".into());
            }
            w.folk.hiring.titles.push((man, 2));
            "You make him assistant head coach. The next man you hire will want a title too.".into()
        }
        Counter::Promise => {
            if !matches!(w.coaches[man].job.map(|j| j.role), Some(Role::PositionCoach(_))) {
                return Err("He's a coordinator already: there's no next coordinator job to promise him.".into());
            }
            w.ground.staff.promises.push(Promise { man: man as u32, kind: 1, made: d, kept: None });
            "You promise him your next coordinator job. It's tracked: break it and his trust goes, and his network hears."
                .into()
        }
    };
    let stays = crate::world::front_office::key(w, 0x57A7_C0DE, man as u64 ^ ((d.0 as u64) << 24)) < chance;
    let there = super::men::employer_name(w, x.employer);
    let c = &mut w.ground.staff.courted[k];
    c.counter = Some(kind as u8);
    c.state = if stays { 1 } else { 2 };
    let name = last(w, man);
    let text = if stays {
        w.folk.mine.nudge(coach_key(man), 0.05);
        format!("{said} {name} stays: he calls {there} tonight and tells them no.")
    } else {
        format!("{said} {name} thanks you, and takes the job at {there}.")
    };
    crate::world::staff::fill_now(w, x.employer, x.role, x.prestige);
    Ok(text)
}

/// What keeps him (GR76: what's pulling him each way, the staying side).
pub fn keeps_him(w: &World, prog: usize, man: usize) -> Vec<String> {
    let mut v = super::men::mood(w, prog, man).lifts;
    let co = &w.coaches[man];
    if co.personality.loyalty >= 13.0 {
        v.push("he's a loyal man".into());
    }
    if co.home_state == Some(w.programs[prog].state) {
        v.push("he's home here".into());
    }
    let years = co.job.map_or(0, |j| w.year - j.since);
    if years >= 4 {
        v.push(format!("{years} years on your staff"));
    }
    v
}

/// You let him go without a counter.
pub fn let_go(w: &mut World, man: usize) -> Result<String, String> {
    let k = w
        .ground
        .staff
        .courted
        .iter()
        .position(|x| x.man == man as u32 && x.state == 0)
        .ok_or("Nobody is after him right now.")?;
    w.ground.staff.courted[k].state = 2;
    let x = w.ground.staff.courted[k].clone();
    crate::world::staff::fill_now(w, x.employer, x.role, x.prestige);
    Ok(format!("You wish {} well. He takes the job at {}.", last(w, man), super::men::employer_name(w, x.employer)))
}

/// Your staff against yesterday's: who left, who came, and the
/// coordinator jobs (passed over, the promise kept or broken).
pub(crate) fn roster(w: &mut World, prog: usize, d: Date) {
    let you = w.career.as_ref().map(|c| c.coach as u32);
    let now: Vec<u32> = men(w, prog).into_iter().map(|c| c as u32).filter(|&c| Some(c) != you).collect();
    let was = std::mem::take(&mut w.ground.staff.roster);
    for &m in was.iter().filter(|m| !now.contains(m)) {
        departed(w, prog, m as usize, d);
    }
    for &m in now.iter().filter(|m| !was.contains(m)) {
        arrived(w, prog, m as usize, d);
    }
    w.ground.staff.roster = now.clone();
    let p = &w.programs[prog];
    let coords = (p.oc.map(|c| c as u32), p.dc.map(|c| c as u32));
    let old = w.ground.staff.coords;
    w.ground.staff.coords = coords;
    for (side, (before, after)) in [(true, (old.0, coords.0)), (false, (old.1, coords.1))] {
        let Some(n) = after else { continue };
        if before == Some(n) || before.is_none() && old == (None, None) {
            continue;
        }
        // The promise of the next coordinator job.
        let due: Vec<usize> = (0..w.ground.staff.promises.len())
            .filter(|&k| w.ground.staff.promises[k].kept.is_none() && w.ground.staff.promises[k].kind == 1)
            .collect();
        for k in due {
            let m = w.ground.staff.promises[k].man;
            if m == n {
                w.ground.staff.promises[k].kept = Some(true);
                w.folk.mine.nudge(coach_key(m as usize), 0.08);
                keep_moment(w, Who::Coach(m), "You kept your word on the coordinator job.", true, d);
            } else if now.contains(&m) {
                w.ground.staff.promises[k].kept = Some(false);
                w.folk.mine.nudge(coach_key(m as usize), -0.15);
                keep_moment(w, Who::Coach(m), "You promised me that job.", false, d);
                memo(
                    w,
                    d,
                    8 * 60,
                    m as usize,
                    "The coordinator job".into(),
                    vec![format!(
                        "You promised me the next one, Coach. {} got it. I won't pretend that doesn't change things.",
                        last(w, n as usize)
                    )],
                    2,
                );
            }
        }
        // Ambitious men on his side who didn't get it.
        for &m in &now {
            if m == n {
                continue;
            }
            let Some(c) = w.coaches.get(m as usize) else { continue };
            let on_side = matches!(c.job.map(|j| j.role), Some(Role::PositionCoach(g)) if g.offense() == side);
            if on_side && c.personality.ambition >= 12.0 && c.age < 56 {
                w.ground.staff.passed_over.push((m, d));
            }
        }
    }
    w.ground.staff.passed_over.retain(|p| p.1 >= d.add(-730));
}

/// A moment in the man's memory of you.
fn keep_moment(w: &mut World, who: Who, words: &str, good: bool, d: Date) {
    let Some(you) = w.career.as_ref().map(|c| c.coach as u32) else { return };
    let holder = Holder::Coach(you);
    let start = standing::base(w, who, holder);
    let m = standing::entry(&mut w.ground, start.max(40.0), who, holder, d);
    m.keep(crate::world::ground::Moment {
        date: d,
        words: words.into(),
        good,
        big: !good,
        holds_to: (!good).then(|| d.add(3 * 365)),
    });
}

/// He's gone (GR68, GR76): his ties went with him (the memory is his),
/// your tree or a man who runs you down, his commits waver, his areas wait
/// for the coordinator's Monday.
fn departed(w: &mut World, prog: usize, man: usize, d: Date) {
    let Some(co) = w.coaches.get(man) else { return };
    let to = co.job.map(|j| j.employer);
    let ended = co.history.last().map_or(String::new(), |s| s.ended.to_string());
    let let_go = ended.contains("let go") || ended.contains("fired");
    let retired = co.retired || ended.contains("retire") || ended.contains("stepped away");
    let mood = super::men::mood(w, prog, man).value;
    let broke = w.ground.staff.promises.iter().any(|p| p.man == man as u32 && p.kept == Some(false));
    let kept = w.ground.staff.promises.iter().any(|p| p.man == man as u32 && p.kept == Some(true));
    let terms = if broke || (let_go && mood < 0.5) || mood < 0.3 {
        2
    } else if retired || let_go {
        3
    } else if mood >= 0.7 || kept {
        1
    } else if mood >= 0.45 {
        0
    } else {
        3
    };
    let name = w.coaches[man].name.clone();
    let you = w.career.as_ref().map(|c| c.coach as u32);
    let who = Who::Coach(man as u32);
    let since = w.coaches[man].history.last().map_or(d.year() - 1, |s| s.from);
    let met = Date::ymd(since.max(d.year() - 30), 7, 1);
    let mut lines: Vec<String> = Vec::new();
    match terms {
        0 | 1 => {
            let pts = if terms == 1 { standing::TREE_HOME } else { standing::TREE };
            for h in [you.map(Holder::Coach), Some(Holder::Program(prog as u32))].into_iter().flatten() {
                let m = w.ground.entry(who, h, || Memory::new(who, h, 0.0, met));
                m.points = m.points.max(pts);
                m.met = m.met.min(met);
                m.contacts = m.contacts.max(6);
                m.known_since = Some(m.known_since.unwrap_or(met));
                m.roots.tree = true;
                m.keep(crate::world::ground::Moment {
                    date: d,
                    words: if terms == 1 {
                        "I owe you more than I can say, Coach.".into()
                    } else {
                        "You treated me right. I won't forget it.".into()
                    },
                    good: true,
                    big: terms == 1,
                    holds_to: None,
                });
            }
            lines.push(format!(
                "{name} leaves on good terms: he's your coaching tree now, {} with you wherever he lands.",
                if terms == 1 { "Home" } else { "Trusted" }
            ));
        }
        2 => {
            keep_moment(w, who, "You ran me off.", false, d);
            // He runs you down to the people who know him.
            let knows: Vec<Who> = w
                .ground
                .memories
                .iter()
                .filter(|m| m.holder == Holder::Coach(man as u32) && m.points >= 25.0)
                .map(|m| m.who)
                .take(30)
                .collect();
            let words = format!("he heard what {} says about you", last(w, man));
            for (k, t) in knows.iter().enumerate() {
                let lag = 4 + (people::u(w, *t, 0x5A1 + k as u64) * 10.0) as i32;
                w.ground.travel.push(word::Travel {
                    due: d.add(lag),
                    prog: prog as u32,
                    to: *t,
                    by: -RUN_DOWN,
                    good: false,
                    words: words.clone(),
                    close: false,
                    holder: None,
                });
            }
            w.ground.travel.sort_by_key(|t| t.due);
            lines.push(format!(
                "{name} leaves angry. The people who know him will hear his side of it: {}.",
                count(knows.len(), "person", "people")
            ));
        }
        _ => {}
    }
    // His commits waver (a warning and one call each).
    let mut shaken = Vec::new();
    for (i, q) in w.people.iter().enumerate() {
        if !matches!(q.stage, Stage::HighSchool | Stage::PrepSchool) || q.cal.signed.is_some() {
            continue;
        }
        if q.cal.commit.is_none_or(|c| c.0 != prog) {
            continue;
        }
        if beats::recruiter_for_school(w, prog, q.hs) == Some(man) {
            shaken.push(i);
        }
    }
    for &i in &shaken {
        let id = w.people[i].player.id;
        if w.ground.staff.wavers.iter().any(|x| x.id == id && x.done.is_none()) {
            continue;
        }
        w.ground.staff.wavers.push(Waver {
            id,
            man: man as u32,
            made: d,
            until: d.add(7),
            called: false,
            done: None,
            line: None,
        });
        let kid = w.people[i].name.clone();
        let fam = people::family_voice(w, people::family_of(w, i));
        memo(
            w,
            d,
            10 * 60,
            w.programs[prog].head_coach.unwrap_or(man),
            format!("{kid} is wavering"),
            vec![
                format!(
                    "{kid} committed to {} as much as to us. With him gone, {fam} wants to hear from you.",
                    last(w, man)
                ),
                "Call him this week, or he decides without you.".into(),
            ],
            2,
        );
    }
    if !shaken.is_empty() {
        lines.push(format!("{} committed to him are wavering.", count(shaken.len(), "kid", "kids")));
    }
    let leaves: Vec<String> = w
        .ground
        .staff
        .beats
        .iter()
        .filter(|b| b.man == man as u32)
        .map(|b| crate::world::ground::areas::area(b.area).nickname.to_string())
        .collect();
    if !leaves.is_empty() {
        lines.push(format!(
            "His areas ({}) still answer to the program's own ties, but the next lead there reaches a rival first until your new man climbs.",
            leaves.join(", ")
        ));
    }
    w.ground.staff.beats.retain(|b| b.man != man as u32);
    w.ground.staff.rides.retain(|r| r.young != man as u32 && r.veteran != man as u32);
    let why = match to {
        Some(e) => format!("left for {}", super::men::employer_name(w, e)),
        None => ended.clone(),
    };
    w.ground.staff.left.push(Left { man: man as u32, date: d, terms, to, why: why.clone() });
    if w.ground.staff.left.len() > 30 {
        w.ground.staff.left.remove(0);
    }
    if let Some(c) = super::super::week::coordinator(w, prog) {
        let mut body = vec![format!("{name} has {why}. What he built with people goes with him.")];
        body.extend(lines);
        memo(w, d, 8 * 60, c, format!("{} is gone", last(w, man)), body, 2);
    }
}

/// A new man on your staff: what his ground looks like, and (a high school
/// coach hired for his area) his old school's people know him.
fn arrived(w: &mut World, prog: usize, man: usize, d: Date) {
    let Some(co) = w.coaches.get(man) else { return };
    if let Some(s) = co.history.last()
        && let Employer::HighSchool(h) = s.employer
        && s.to >= d.year() - 1
    {
        let holder = Holder::Coach(man as u32);
        let (oc, dc) = w.high_schools.get(h).map_or((None, None), |x| (x.oc, x.dc));
        let mut who: Vec<Who> = [oc, dc].into_iter().flatten().map(|c| Who::Coach(c as u32)).collect();
        who.push(Who::Counselor(h as u32));
        if let Some(hc) = people::head_coach_of(w, h) {
            who.push(hc);
        }
        let met = d.add(-5 * 365);
        for p in who {
            let m = w.ground.entry(p, holder, || Memory::new(p, holder, 0.0, met));
            m.points = m.points.max(60.0);
            m.contacts = m.contacts.max(6);
            m.known_since = Some(m.known_since.unwrap_or(met.add(365)));
            m.roots.old_job = true;
        }
    }
    let signs = super::men::signs(w, prog, man);
    if let Some(c) = super::super::week::coordinator(w, prog)
        && c != man
    {
        let mut body = vec![format!("{} joins the staff. What we know of his ground:", w.coaches[man].name)];
        body.extend(signs);
        body.push("I'll give him areas at Monday's meeting.".into());
        memo(w, d, 8 * 60, c, format!("{}'s ground", last(w, man)), body, 1);
    }
}

/// You call a wavering commit (the scene's stand-in until lane G5).
pub fn call_commit(w: &mut World, id: u32) -> Result<String, String> {
    let prog = super::seat_program(w).ok_or("No college staff.")?;
    let k = w.ground.staff.wavers.iter().position(|x| x.id == id && x.done.is_none()).ok_or("He isn't wavering.")?;
    w.ground.staff.wavers[k].called = true;
    if let Some(i) = crate::world::scouting::place(w, id) {
        let fam = people::family_of(w, i);
        standing::contact(w, prog, w.career.as_ref().map(|c| c.coach), fam, standing::Contact::Call, 1.0, w.today);
    }
    let d = w.today;
    Ok(settle_waver(w, prog, k, d))
}

/// A wavering commit decides: he holds, reopens, or follows his recruiter.
fn settle_waver(w: &mut World, prog: usize, k: usize, d: Date) -> String {
    let x = w.ground.staff.wavers[k].clone();
    let Some(i) = crate::world::scouting::place(w, x.id) else {
        w.ground.staff.wavers[k].done = Some(0);
        return String::new();
    };
    let q = &w.people[i];
    if q.cal.commit.is_none_or(|c| c.0 != prog) || q.cal.signed.is_some() {
        w.ground.staff.wavers[k].done = Some(0);
        return "It settled itself.".into();
    }
    let pers = &q.player.personality;
    let loyal = clamp(1.0 - 0.04 * (pers.love_of_game - 10.0) - 0.03 * (pers.coachability - 10.0), 0.5, 1.4);
    let fam = people::family_of(w, i);
    let known = standing::level(w, prog, fam) >= Level::Known;
    let chance = BOND * loyal * if x.called { 0.5 } else { 1.0 } * if known { 0.7 } else { 1.0 };
    let roll = crate::world::front_office::key(w, 0x3A7E_4000, x.id as u64);
    let name = q.name.clone();
    let stars = q.stars;
    if roll >= chance {
        w.ground.staff.wavers[k].done = Some(0);
        let line = if x.called {
            format!("{name} holds: \"Thank you for calling, Coach. We're still with you.\"")
        } else {
            format!("{name} holds his commitment.")
        };
        w.ground.staff.wavers[k].line = Some(line.clone());
        return line;
    }
    // He reopens; some follow the man.
    let new = w.coaches.get(x.man as usize).and_then(|c| c.job).and_then(|j| match j.employer {
        Employer::College(q) if q != prog => Some(q),
        _ => None,
    });
    let near = new.filter(|&q| {
        w.programs[q].prestige >= w.programs[prog].prestige - 0.2
            && stars as f64 <= 2.0 + (3.0 * w.programs[q].prestige).ceil()
    });
    let follows = near.is_some() && crate::world::front_office::key(w, 0x3A7E_4001, x.id as u64) < FOLLOW;
    let age = w.people[i].row_age();
    let school = w.programs[prog].real_name.clone();
    let p = &mut w.people[i];
    let line = if follows {
        let q = near.unwrap();
        p.cal.commit = Some((q, d, false));
        if !p.cal.offers.contains(&q) {
            p.cal.offers.push(q);
        }
        let to = w.programs[q].real_name.clone();
        let p = &mut w.people[i];
        p.note(format!("At {age} he left {school} to follow the coach who recruited him to {to}."));
        w.ground.staff.wavers[k].done = Some(2);
        format!("{name} follows {} to {to}.", last(w, x.man as usize))
    } else {
        p.cal.commit = None;
        p.cal.commit_timing = Some(CommitTiming::Late);
        p.note(format!("At {age} he reopened his recruitment when the {school} coach who recruited him left."));
        w.ground.staff.wavers[k].done = Some(1);
        format!("{name} reopens his recruitment.")
    };
    w.log.decommits += 1;
    w.ground.staff.wavers[k].line = Some(line.clone());
    line
}

/// Each day: answers that came due, and wavering commits who decide
/// without your call.
pub(crate) fn day(w: &mut World, prog: usize, d: Date) {
    let due: Vec<usize> = (0..w.ground.staff.courted.len())
        .filter(|&k| w.ground.staff.courted[k].state == 0 && w.ground.staff.courted[k].until <= d)
        .collect();
    for k in due {
        w.ground.staff.courted[k].state = 2;
        let x = w.ground.staff.courted[k].clone();
        crate::world::staff::fill_now(w, x.employer, x.role, x.prestige);
    }
    w.ground.staff.courted.retain(|x| x.state == 0 || x.made >= d.add(-400));
    let due: Vec<usize> = (0..w.ground.staff.wavers.len())
        .filter(|&k| w.ground.staff.wavers[k].done.is_none() && w.ground.staff.wavers[k].until <= d)
        .collect();
    for k in due {
        let line = settle_waver(w, prog, k, d);
        if !line.is_empty()
            && let Some(c) = super::super::week::coordinator(w, prog)
        {
            memo(w, d, 17 * 60, c, "A wavering commit decided".into(), vec![line], 1);
        }
    }
    w.ground.staff.wavers.retain(|x| x.done.is_none() || x.made >= d.add(-400));
}
