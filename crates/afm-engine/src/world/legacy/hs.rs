//! High school's five chases (L11-L15), on the built high school seat:
//! each state's classes and playoffs, the principal and the district, the
//! school and its feeders, the coach's word with college staffs. L11 is
//! your town's support and the bond vote it can carry; L12 tracks the
//! runs (titles, district crowns, playoff trips, the streak); L13 is your
//! placement record and your word; L14 is your feeder system, whose
//! freshmen arrive knowing your terms (the playbook learning the engine
//! already has starts further along); L15 is the jump to a college staff.
//! Sizes are proposed, not confirmed; the bond vote's majorities are each
//! state's real rule for school bonds.

use super::{ACT_BOND, Bank, key, letter, memo, pay};
use crate::calendar::{Date, Weekday};
use crate::world::World;
use crate::world::coach::{Employer, Role};
use crate::world::data::Division;
use crate::world::desk::{Answer, Head};
use crate::world::stories::TeamKey;
use crate::world::storylines::catalogue::{K, Rating};
use crate::world::{PlayingTime, Stage};

/// What the high school chases keep.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Hs {
    /// The town's support, 0-100, and each season's.
    pub support: f32,
    pub support_log: Vec<(i32, f32)>,
    pub bond: Option<Bond>,
    pub bond_fails: u8,
    /// Every bond vote: (year, yes share, passed).
    pub votes: Vec<(i32, f32, bool)>,
    /// Signees each class by level: FBS, FCS, Division II, Division III,
    /// NAIA, junior college.
    pub placement: Vec<(i32, [u16; 6])>,
    /// Your feeders' adoption of your playbook, by feeder name.
    pub feeders: Vec<FeederPlan>,
    /// College calls for you: clinic talks, camp jobs (years).
    pub talks: Vec<i32>,
    pub camps: Vec<i32>,
    /// Your son on your roster: (child index, player id).
    pub son: Option<(u32, u32)>,
}

/// A bond for a new stadium or field house.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Bond {
    /// 0 a plan, 1 the board put it forward, 2 on the ballot, 3 passed,
    /// 4 opened, 5 failed.
    pub stage: u8,
    pub what: String,
    pub cost: f64,
    pub started: i32,
    pub vote: Option<Date>,
    pub share: f32,
    pub opens: i32,
}

/// One feeder's take-up of your system.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FeederPlan {
    pub name: String,
    pub coach: String,
    /// What you've offered: the playbook, a clinic, an assistant on its
    /// staff, equipment.
    pub playbook: bool,
    pub clinic: bool,
    pub assistant: bool,
    pub equipment: bool,
    /// 0-1: how much of your system it runs.
    pub adopt: f32,
    /// The coach's openness, 0-1 (keyed).
    pub open: f32,
}

fn school(w: &World) -> Option<usize> {
    match super::you(w)?.1 {
        TeamKey::Hs(h) => Some(h as usize),
        _ => None,
    }
}

/// A division's place in the placement record.
fn level_index(d: Division) -> usize {
    match d {
        Division::FBS => 0,
        Division::FCS => 1,
        Division::D2 => 2,
        Division::D3 => 3,
        Division::NAIA => 4,
        Division::JUCO => 5,
    }
}

/// A school's signees in a class, by level.
pub fn signees(w: &World, h: usize, class: i32) -> [u16; 6] {
    let mut out = [0u16; 6];
    for p in &w.people {
        if p.hs != h || p.class_of != class {
            continue;
        }
        if let Some(s) = p.cal.signed.as_ref()
            && let Some(prog) = w.programs.get(s.program)
        {
            out[level_index(prog.division)] += 1;
        }
    }
    for a in &w.alumni {
        if a.high_school == h
            && a.class_of == class
            && let Some(&c) = a.colleges.first()
            && let Some(prog) = w.programs.get(c)
        {
            out[level_index(prog.division)] += 1;
        }
    }
    out
}

/// The last class that sent a player to an FBS school.
pub(crate) fn last_fbs_signee(w: &World, h: usize) -> Option<i32> {
    let mut best: Option<i32> = None;
    for p in &w.people {
        if p.hs == h
            && let Some(s) = p.cal.signed.as_ref()
            && w.programs.get(s.program).is_some_and(|x| x.division == Division::FBS)
        {
            best = best.max(Some(p.class_of));
        }
    }
    for a in &w.alumni {
        if a.high_school == h
            && a.colleges.first().and_then(|&c| w.programs.get(c)).is_some_and(|x| x.division == Division::FBS)
        {
            best = best.max(Some(a.class_of));
        }
    }
    best
}

/// Players you coached who signed with a Division I school, in classes from
/// a year on.
pub(crate) fn d1_since(w: &World, team: TeamKey, year: i32) -> u32 {
    let TeamKey::Hs(h) = team else { return 0 };
    let ours: Vec<u32> = w.legacy.guys.iter().map(|g| g.id).collect();
    let mut n = 0;
    for p in &w.people {
        if p.hs == h as usize
            && p.class_of >= year
            && ours.binary_search(&p.player.id).is_ok()
            && let Some(s) = p.cal.signed.as_ref()
            && w.programs.get(s.program).is_some_and(|x| matches!(x.division, Division::FBS | Division::FCS))
        {
            n += 1;
        }
    }
    n
}

/// Why the jump is in reach.
pub(crate) fn jump_why(w: &World, h: usize) -> String {
    let state = crate::world::folk::facts::state_name(&w.states[w.high_schools[h].state].code);
    let staffs = w.scouting.hs.credibility.values().filter(|&&c| c >= 0.6).count();
    if staffs > 0 {
        format!("{staffs} college staffs trust your word. Gus Malzahn went from Springdale High to Arkansas in 2006.")
    } else {
        format!(
            "College staffs hire coaches with {state} ties. Chad Morris went from Lake Travis High to Tulsa in 2010."
        )
    }
}

/// Why a future NFL player is in reach.
pub(crate) fn nfl_why(w: &World, h: usize) -> String {
    let pros = w.people.iter().filter(|p| p.hs == h && p.pro.as_ref().is_some_and(|x| x.team.is_some())).count();
    match pros {
        0 => format!("Nobody from {} plays in the NFL today.", w.school_name(h)),
        1 => format!("One player from {} is in the NFL today.", w.school_name(h)),
        n => format!("{n} players from {} are in the NFL today.", w.school_name(h)),
    }
}

/// The building a high school would put to the voters.
pub(crate) fn building_title(w: &World, h: usize) -> String {
    if w.high_schools[h].enrollment >= 1200 { "A new stadium".into() } else { "A new field house".into() }
}

/// Why the building is a chase.
pub(crate) fn building_why(w: &World, h: usize) -> String {
    let town = &w.high_schools[h].town;
    format!("{town}'s voters decide a bond. Allen, Texas, opened a $60 million stadium in 2012 after one.")
}

/// The bond's steps.
pub(crate) fn bond_steps(w: &World) -> [bool; 5] {
    match &w.legacy.hs.bond {
        None => [false; 5],
        // A plan (stage 0 on), on the ballot (2 on, a failed vote too),
        // passed (3 or 4), opened (4). Stage 9 is a question not answered.
        Some(b) if b.stage == 9 => [false; 5],
        Some(b) => [true, b.stage >= 2, b.stage >= 2, (3..=4).contains(&b.stage), b.stage == 4],
    }
}

/// The share a bond needs: each state's rule for school bonds.
fn majority(code: &str) -> f32 {
    match code {
        "ID" => 0.667,
        "MO" => 0.5714,
        "WA" | "OK" | "IA" => 0.6,
        "CA" => 0.55,
        _ => 0.5,
    }
}

/// The town's support now, 0-100: the wins, the rivalry, the playoffs,
/// your years in town and how the people you answer to feel.
fn support_now(w: &World, h: usize) -> f32 {
    let team = TeamKey::Hs(h as u32);
    let seasons = super::seasons_at(w, team);
    let last: Vec<&super::Yr> = seasons.iter().rev().take(3).collect();
    let pct = last.first().map_or(0.5, |y| y.pct());
    let older =
        if last.len() > 1 { last[1..].iter().map(|y| y.pct()).sum::<f64>() / (last.len() - 1) as f64 } else { pct };
    let post = last.first().map_or(0, |y| y.post_wins) as f64;
    let rival = crate::world::folk::facts::rival(w, crate::world::folk::Team::High(h))
        .map(|r| crate::world::storylines::team_key(r.0));
    let rivalry = match rival {
        Some(r) => {
            let (wn, l, _) = super::record_vs(w, Some(r), None);
            (wn as f64 - l as f64).clamp(-3.0, 3.0)
        }
        None => 0.0,
    };
    let years = super::ambitions::years_at(w, team) as f64;
    let trust = super::boss_trust(w) as f64;
    (30.0 + 35.0 * pct + 15.0 * older + 4.0 * post + 4.0 * rivalry + years.min(10.0) + 10.0 * (trust - 0.5))
        .clamp(0.0, 100.0) as f32
}

/// The town's support in words.
pub fn support_words(s: f32) -> &'static str {
    match s {
        x if x >= 75.0 => "Strong",
        x if x >= 55.0 => "Behind you",
        x if x >= 40.0 => "Watching",
        x if x >= 25.0 => "Restless",
        _ => "Turning",
    }
}

/// Election day: the first Tuesday after the first Monday in November.
fn election_day(year: i32) -> Date {
    let mon = Date::nth_weekday(year, 11, Weekday::Mon, 1);
    mon.add(1)
}

/// Every day at your high school.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(h) = school(w) else { return };
    let year = w.year;
    // Spring: the superintendent's question on the bond; summer: the board.
    if d == Date::ymd(year, 3, 15) && super::head(w) {
        bond_letter(w, h);
    }
    if d == Date::ymd(year, 7, 15)
        && let Some(b) = w.legacy.hs.bond.as_mut()
        && b.stage == 0
    {
        let support = w.legacy.hs.support.max(support_now(w, h));
        let b = w.legacy.hs.bond.as_mut().expect("bond");
        if support >= 50.0 {
            b.stage = 2;
            b.vote = Some(election_day(year));
            let what = b.what.to_lowercase();
            super::story(
                w,
                K::BondVote,
                year as u64,
                what.clone(),
                format!("The school board put {what} on the November ballot."),
            );
            memo(
                w,
                "legacy-bond-board",
                "The bond is on the ballot".into(),
                vec![format!(
                    "The school board put the bond on the November ballot, for {}.",
                    election_day(year).pretty()
                )],
            );
        } else {
            memo(
                w,
                "legacy-bond-board",
                "The board waits".into(),
                vec![
                    "The school board didn't put the bond on this year's ballot. The town isn't behind it yet.".into(),
                ],
            );
        }
    }
    if let Some(b) = w.legacy.hs.bond.clone()
        && b.stage == 2
        && b.vote == Some(d)
    {
        vote(w, h, &b);
    }
    if let Some(b) = w.legacy.hs.bond.as_mut()
        && b.stage == 3
        && year >= b.opens
        && d == Date::ymd(year, 8, 15)
    {
        b.stage = 4;
        let what = b.what.clone();
        for p in w.finance.projects.iter_mut() {
            if p.owner == crate::world::finance::Owner::Hs(h) && !p.done {
                p.done = true;
            }
        }
        w.high_schools[h].prestige = (w.high_schools[h].prestige + 0.03).min(1.0);
        memo(
            w,
            "legacy-bond-open",
            format!("{what} opens"),
            vec![format!("{what} opens this fall. The town paid for it.")],
        );
    }
    // Your son starts ninth grade: on your roster.
    if d == Date::ymd(year, 8, 1) {
        son_arrives(w, h);
    }
    // The class signs: your placement record.
    if d == w.dates.signing_day.add(1) {
        let class = year + 1;
        let row = signees(w, h, class);
        w.legacy.hs.placement.retain(|x| x.0 != class);
        w.legacy.hs.placement.push((class, row));
        let fbs = row[0] as u32 + row[1] as u32;
        if fbs > 0 {
            pay(
                w,
                key("L13", h as u64, class as u64),
                "L13",
                format!("{fbs} Division I signees in the class of {class}"),
                (0.25 * fbs as f32).min(1.0),
                Bank::Rating(Rating::Network),
                0.3 * fbs.min(5) as f64,
            );
        }
    }
    // College staffs come through in the spring: a clinic talk or a camp.
    if d == Date::ymd(year, 4, 20) {
        college_calls(w, h);
    }
    // Winter: the jump's offers, when your résumé draws them.
    if d == Date::ymd(year + 1, 1, 12) {
        jump_offer(w, h);
    }
    // The feeders take up your system through the year.
    if d.weekday() == Weekday::Mon && d.parts().1 == 6 {
        feeders_grow(w, h);
    }
}

fn bond_letter(w: &mut World, h: usize) {
    // One at a time: a new ask only when none is alive (a failed one can
    // come back a year later).
    if w.legacy.hs.bond.as_ref().is_some_and(|b| b.stage != 5) {
        return;
    }
    let support = support_now(w, h);
    if support < 45.0 {
        return;
    }
    let what = building_title(w, h);
    let enroll = w.high_schools[h].enrollment as f64;
    let idx = crate::world::finance::data::hs_index(w.year);
    let cost = if what == "A new stadium" { 8.0e6 + 9_000.0 * enroll } else { 2.5e6 + 1_500.0 * enroll } * idx;
    let year = w.year;
    let id = letter(
        w,
        &format!("legacy-bond-{year}"),
        Head::School,
        ("Superintendent".into(), "District office".into()),
        format!("{what}: put it to the voters?"),
        vec![
            format!(
                "The district could ask the town for a {} bond for {}.",
                crate::world::front_office::dollars(cost),
                what.to_lowercase()
            ),
            "If you push for it, you'll be the face of it: at the board, in the paper, at the churches.".into(),
        ],
        vec![
            Answer {
                label: "Push for it".into(),
                does: "The board decides in July; the town votes in November.".into(),
                act: ACT_BOND,
                target: 1,
            },
            Answer { label: "Not now".into(), does: "Nothing changes.".into(), act: ACT_BOND, target: 0 },
        ],
    );
    if id.is_some() {
        w.legacy.hs.bond = Some(Bond { stage: 9, what, cost, started: year, vote: None, share: 0.0, opens: 0 });
    }
}

/// Your answer to the superintendent.
pub(crate) fn answer_bond(w: &mut World, target: u32) {
    let Some(b) = w.legacy.hs.bond.as_mut() else { return };
    if b.stage != 9 {
        return;
    }
    if target == 1 {
        b.stage = 0;
    } else {
        w.legacy.hs.bond = None;
    }
}

/// Election night.
fn vote(w: &mut World, h: usize, b: &Bond) {
    let year = w.year;
    let code = w.states[w.high_schools[h].state].code.clone();
    let need = majority(&code);
    let support = support_now(w, h);
    let wealth = w.high_schools[h].resources as f32;
    let u = crate::world::season::key_uniform(w.options.seed as u64 ^ 0xB0D, (h as u64) << 20 ^ year as u64) as f32;
    let share =
        (0.36 + 0.25 * support / 100.0 + 0.1 * wealth + 0.02 * w.legacy.hs.bond_fails as f32 + 0.08 * (u - 0.5))
            .clamp(0.2, 0.8);
    let passed = share >= need;
    w.legacy.hs.votes.push((year, share, passed));
    let pct = (share * 100.0).round();
    let bb = w.legacy.hs.bond.as_mut().expect("bond");
    bb.share = share;
    if passed {
        bb.stage = 3;
        bb.opens = year + 2;
        let what = bb.what.clone();
        let cost = bb.cost;
        w.finance.projects.push(crate::world::finance::Project {
            owner: crate::world::finance::Owner::Hs(h),
            kind: if what == "A new stadium" {
                crate::world::finance::ProjectKind::Stadium
            } else {
                crate::world::finance::ProjectKind::Facility
            },
            name: what.to_lowercase().trim_start_matches("a new ").to_string(),
            cost: cost as f32,
            public: cost as f32,
            pledged: 0.0,
            started: year,
            opens: year + 2,
            capacity: 0,
            step: 0.03,
            done: false,
            asked: true,
        });
        super::story_end(w, K::BondVote, year as u64, "passed");
        memo(
            w,
            "legacy-bond-vote",
            format!("The bond passes with {pct}%"),
            vec![format!("{what} will open in {}. The town voted for it: {pct}% yes.", year + 2)],
        );
        pay(
            w,
            key("L11", h as u64, year as u64),
            "L11",
            format!("The bond passed with {pct}%"),
            0.75,
            Bank::Rating(Rating::Relationships),
            1.5,
        );
        super::boss_by(w, 0.05);
    } else {
        bb.stage = 5;
        w.legacy.hs.bond_fails += 1;
        let line = format!("The bond failed: {pct}% yes, {:.0}% needed.", need * 100.0);
        super::story_end(w, K::BondVote, year as u64, "failed");
        memo(w, "legacy-bond-vote", "The bond fails".into(), vec![line.clone()]);
        super::cost(w, key("L11", h as u64, 100 + year as u64), "L11", line, 0.3);
        let _ = b;
    }
}

/// The college staffs' spring: a clinic talk or a summer camp job for a
/// coach whose word they trust.
fn college_calls(w: &mut World, h: usize) {
    let year = w.year;
    let trusted = w.scouting.hs.credibility.values().filter(|&&c| c >= 0.6).count() as f64;
    let titles = super::seasons_at(w, TeamKey::Hs(h as u32)).iter().filter(|s| s.champion).count() as f64;
    let named = w.legacy.scheme.named.is_some() as u8 as f64;
    let u = crate::world::season::key_uniform(w.options.seed as u64 ^ 0xCA11, (h as u64) << 20 ^ year as u64);
    let chance = (0.05 + 0.04 * trusted + 0.1 * titles.min(3.0) + 0.15 * named).min(0.7);
    if u >= chance {
        return;
    }
    let camp = u < chance * 0.5;
    if camp {
        w.legacy.hs.camps.push(year);
    } else {
        w.legacy.hs.talks.push(year);
    }
    let what = if camp { "work its summer camp" } else { "speak at its spring clinic" };
    memo(
        w,
        "legacy-college-call",
        "A college staff called".into(),
        vec![format!(
            "A college staff in the state asked you to {what}. College coaches will know your name a little better."
        )],
    );
    pay(
        w,
        key("L15", year as u64, camp as u64),
        "L15",
        if camp { "Worked a college camp".into() } else { "Spoke at a college clinic".to_string() },
        0.25,
        Bank::Rating(Rating::Network),
        0.3,
    );
}

/// The jump: a college that wants your state ties or your system offers a
/// job on its staff.
fn jump_offer(w: &mut World, h: usize) {
    let Some(me) = super::you(w).map(|x| x.0) else { return };
    let year = w.year;
    let state = w.high_schools[h].state;
    let seasons = super::seasons_at(w, TeamKey::Hs(h as u32));
    let titles = seasons.iter().filter(|s| s.champion).count() as f64;
    let d1: f64 = w.legacy.hs.placement.iter().filter(|x| x.0 >= year - 4).map(|x| (x.1[0] + x.1[1]) as f64).sum();
    let named = w.legacy.scheme.named.is_some() as u8 as f64;
    let calls = (w.legacy.hs.talks.len() + w.legacy.hs.camps.len()) as f64;
    let score = 0.03 + 0.06 * titles.min(3.0) + 0.012 * d1.min(15.0) + 0.1 * named + 0.03 * calls.min(4.0);
    let u = crate::world::season::key_uniform(w.options.seed as u64 ^ 0x1A3B, (h as u64) << 20 ^ year as u64);
    if u >= score.min(0.5) {
        return;
    }
    // A college in the state, the closer to your level the likelier.
    let mut cands: Vec<(f64, usize)> = w
        .programs
        .iter()
        .enumerate()
        .filter(|(_, p)| p.state == state && p.plays(year) && matches!(p.division, Division::FBS | Division::FCS))
        .map(|(q, p)| {
            (crate::world::season::key_uniform(w.options.seed as u64 ^ 0x1A3C, q as u64 ^ year as u64) + p.prestige, q)
        })
        .collect();
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));
    let Some(&(_, p)) = cands.first() else { return };
    let coord = score >= 0.35 && u < score * 0.25;
    let came_up = w.coaches[me].came_up;
    let role = if coord {
        if came_up.offense() { Role::OffensiveCoordinator } else { Role::DefensiveCoordinator }
    } else {
        Role::PositionCoach(came_up)
    };
    let prestige = crate::world::staff::employer_prestige(w, Employer::College(p));
    crate::world::front_office::market::offer(w, Employer::College(p), role, prestige);
}

/// Offers from colleges you turned down since a day.
pub(crate) fn jump_declined(w: &World, since: Date) -> u32 {
    w.career.as_ref().map_or(0, |c| {
        c.market.passed.iter().filter(|x| matches!(x.0, Employer::College(_)) && x.2 >= since).count() as u32
    })
}

/// The Reach ambition's steps at a high school.
pub(crate) fn reach_steps(w: &World, set: Date) -> [bool; 5] {
    let Some(c) = w.career.as_ref() else { return [false; 5] };
    let y = set.year();
    let offer = c.market.offers.iter().any(|o| matches!(o.employer, Employer::College(_)) && o.made >= set)
        || jump_declined(w, set) > 0;
    let job = w.coaches[c.coach].job;
    let at_college = job.is_some_and(|j| matches!(j.employer, Employer::College(_)));
    [
        w.legacy.hs.talks.iter().any(|&t| t >= y) || at_college,
        w.legacy.hs.camps.iter().any(|&t| t >= y) || at_college,
        offer || at_college,
        at_college,
        at_college
            && job.is_some_and(|j| {
                matches!(j.role, Role::OffensiveCoordinator | Role::DefensiveCoordinator | Role::HeadCoach)
            }),
    ]
}

/// Your son starts ninth grade at your school: one of the freshmen is
/// him, by name, from now on.
fn son_arrives(w: &mut World, h: usize) {
    let Some(c) = w.career.as_ref() else { return };
    if w.legacy.hs.son.is_some() {
        return;
    }
    let year = w.year;
    let Some((k, name)) = c.you.profile.children.iter().enumerate().find_map(|(k, (n, b))| {
        let age = year - b;
        let first = n.split_whitespace().next().unwrap_or("").to_string();
        ((14..=15).contains(&age) && crate::world::names::is_common_first(&first)).then(|| (k, n.clone()))
    }) else {
        return;
    };
    let fresh = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.hs == h && p.stage == Stage::HighSchool && p.class_of == year + 4)
        .min_by_key(|(_, p)| p.player.id ^ 0x5011)
        .map(|(i, _)| i);
    let Some(i) = fresh else { return };
    let mut full = name.clone();
    if !full.contains(' ')
        && let Some(last) = w.coaches[c.coach].name.split_whitespace().last()
    {
        full = format!("{full} {last}");
    }
    w.people[i].name = full.clone();
    let id = w.people[i].player.id;
    w.legacy.hs.son = Some((k as u32, id));
    memo(
        w,
        "legacy-son",
        format!("{full} is a freshman"),
        vec![format!("{full} starts ninth grade at {}. He's on your roster now.", w.school_name(h))],
    );
}

/// The Son ambition's steps.
pub(crate) fn son_steps(w: &World, a: &super::ambitions::Ambition) -> [bool; 5] {
    let Some((k, id)) = w.legacy.hs.son else { return [false; 5] };
    if k != a.subject {
        return [false; 5];
    }
    let Some(i) = crate::world::storylines::pi(w, id) else { return [false; 5] };
    let p = &w.people[i];
    let Some(h) = school(w) else { return [false; 5] };
    let team = TeamKey::Hs(h as u32);
    let on = p.hs == h && p.stage == Stage::HighSchool;
    let from = p.class_of - 4;
    let games: Vec<&super::Played> = w.legacy.games.iter().filter(|g| g.team == team && g.season > from).collect();
    [
        on || p.class_of <= w.year,
        p.last_role == Some(PlayingTime::Starter),
        games.iter().any(|g| g.playoff),
        games.iter().any(|g| g.playoff && g.won()),
        w.legacy.seasons.iter().any(|s| s.team == team && s.year == p.class_of - 1),
    ]
}

/// Your son plays somewhere else now.
pub(crate) fn son_gone(w: &World, a: &super::ambitions::Ambition) -> bool {
    let Some((k, id)) = w.legacy.hs.son else { return false };
    if k != a.subject {
        return false;
    }
    let Some(h) = school(w) else { return false };
    crate::world::storylines::pi(w, id).is_some_and(|i| {
        let p = &w.people[i];
        p.stage == Stage::HighSchool && p.hs != h
    })
}

/// Your feeders: each coach takes your system up by his openness and what
/// you've given, a little each June.
fn feeders_grow(w: &mut World, h: usize) {
    let list = crate::world::front_office::school::feeders(w, h);
    for f in list.iter().filter(|f| !f.youth) {
        if !w.legacy.hs.feeders.iter().any(|x| x.name == f.name) {
            let plan = new_plan(w, h, f);
            w.legacy.hs.feeders.push(plan);
        }
    }
    let year = w.year;
    for x in w.legacy.hs.feeders.iter_mut() {
        let gave = x.playbook as u8 as f32 * 0.5
            + x.clinic as u8 as f32 * 0.2
            + x.assistant as u8 as f32 * 0.3
            + x.equipment as u8 as f32 * 0.15;
        if gave <= 0.0 {
            x.adopt = (x.adopt - 0.05).max(0.0);
            continue;
        }
        // Some coaches never do: the least open stay their own.
        let cap = if x.open < 0.15 { 0.2 } else { 1.0 };
        x.adopt = (x.adopt + 0.25 * gave * x.open).min(cap);
    }
    let adopted = w.legacy.hs.feeders.iter().filter(|x| x.adopt >= 0.5).count();
    if adopted >= 2 {
        pay(
            w,
            key("L14", h as u64, adopted as u64),
            "L14",
            format!("{adopted} feeder schools run your system"),
            0.5,
            Bank::Rating(Rating::Scheme),
            0.5,
        );
    }
    let _ = year;
}

/// A feeder middle school's plan before you've given it anything: its
/// coach's openness is keyed to the feeder, the same whenever it is made.
fn new_plan(w: &World, h: usize, f: &crate::world::front_office::school::Feeder) -> FeederPlan {
    let open = crate::world::season::key_uniform(
        w.options.seed as u64 ^ 0xFEED,
        f.name.bytes().map(|b| b as u64).sum::<u64>() ^ h as u64,
    ) as f32;
    FeederPlan {
        name: f.name.clone(),
        coach: f.coach.clone(),
        playbook: false,
        clinic: false,
        assistant: false,
        equipment: false,
        adopt: 0.0,
        open,
    }
}

/// What you give a feeder (the screen's buttons): "playbook", "clinic",
/// "assistant", "equipment". Equipment costs the booster club $2,500. A
/// feeder you help before its first June gets its plan that day; the
/// take-up still comes each June.
pub fn give_feeder(w: &mut World, name: &str, what: &str) -> Result<String, String> {
    if !w.legacy.hs.feeders.iter().any(|x| x.name == name)
        && let Some(h) = school(w)
    {
        let list = crate::world::front_office::school::feeders(w, h);
        if let Some(f) = list.iter().find(|f| !f.youth && f.name == name) {
            let plan = new_plan(w, h, f);
            w.legacy.hs.feeders.push(plan);
        }
    }
    let Some(x) = w.legacy.hs.feeders.iter_mut().find(|x| x.name == name) else {
        return Err("No such feeder.".into());
    };
    match what {
        "playbook" => x.playbook = true,
        "clinic" => x.clinic = true,
        "assistant" => x.assistant = true,
        "equipment" => {
            if w.front_office.school.boosters.balance < 2_500.0 {
                return Err("The booster club doesn't have $2,500.".into());
            }
            w.front_office.school.boosters.balance -= 2_500.0;
            let today = w.today;
            w.front_office.school.boosters.ledger.push((today, format!("Equipment for {name}"), -2_500.0));
            let x = w.legacy.hs.feeders.iter_mut().find(|x| x.name == name).expect("feeder");
            x.equipment = true;
        }
        _ => return Err("Unknown help.".into()),
    }
    Ok(format!("{name}: done."))
}

/// A freshman's head start on your playbook: what his feeder took up.
pub(crate) fn head_start(w: &World, feeders: &[crate::world::front_office::school::Feeder], h: usize, i: usize) -> f64 {
    if w.legacy.hs.feeders.is_empty() || w.seat_school() != Some(h) {
        return 0.0;
    }
    let p = &w.people[i];
    if p.player.age > 15 {
        return 0.0;
    }
    let Some(k) = crate::world::front_office::school::feeder_in(feeders, w, h, i) else { return 0.0 };
    let mid: Vec<&crate::world::front_office::school::Feeder> = feeders.iter().filter(|f| !f.youth).collect();
    let Some(f) = mid.get(k) else { return 0.0 };
    w.legacy.hs.feeders.iter().find(|x| x.name == f.name).map_or(0.0, |x| 0.12 * x.adopt as f64)
}

/// After your season: the town's support, the runs.
pub(crate) fn season(w: &mut World) {
    let Some(h) = school(w) else { return };
    let year = w.year;
    let s = support_now(w, h);
    w.legacy.hs.support = s;
    w.legacy.hs.support_log.retain(|x| x.0 != year);
    w.legacy.hs.support_log.push((year, s));
    // What support pays: the booster club.
    let gift = (s as f64 - 40.0).max(0.0) * 150.0 * crate::world::finance::data::hs_index(year);
    if gift > 0.0 {
        w.front_office.school.boosters.balance += gift;
        let today = w.today;
        w.front_office.school.boosters.ledger.push((today, "The town's year-end giving".into(), gift));
    }
    // The principal hears the town: a town that turned costs his trust, and
    // one that backs you buys patience in a losing year.
    if s < 25.0 {
        super::boss_by(w, -0.04);
    } else if s >= 65.0 && w.legacy.seasons.last().is_some_and(|y| y.year == year && y.losses > y.wins) {
        super::boss_by(w, 0.03);
    }
    // L12: the runs.
    let r = runs(w, h);
    for (n, lesson) in [(20u32, 0.5f32), (30, 0.75), (50, 1.0)] {
        if r.streak >= n {
            pay(
                w,
                key("L12", 1, n as u64),
                "L12",
                format!("{n} straight wins"),
                lesson,
                Bank::Rating(Rating::Motivation),
                1.0,
            );
        }
    }
    for (n, lesson) in [(3u32, 0.25f32), (5, 0.5), (8, 0.75)] {
        if r.districts >= n {
            pay(
                w,
                key("L12", 2, n as u64),
                "L12",
                format!("{n} district titles in a row"),
                lesson,
                Bank::Rating(Rating::Motivation),
                0.5,
            );
        }
    }
    for (n, lesson) in [(5u32, 0.25f32), (10, 0.5), (15, 0.75)] {
        if r.playoffs_row >= n {
            pay(
                w,
                key("L12", 3, n as u64),
                "L12",
                format!("{n} playoff trips in a row"),
                lesson,
                Bank::Rating(Rating::GameManagement),
                0.5,
            );
        }
    }
}

/// L12's runs at a school under you.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct Runs {
    pub titles: u32,
    pub finals: u32,
    pub districts: u32,
    pub playoffs: u32,
    pub playoffs_row: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub semis: u32,
}

pub fn runs(w: &World, h: usize) -> Runs {
    let team = TeamKey::Hs(h as u32);
    let s = super::seasons_at(w, team);
    let mut r = Runs {
        titles: s.iter().filter(|y| y.champion).count() as u32,
        finals: s.iter().filter(|y| y.final_game).count() as u32,
        playoffs: s.iter().filter(|y| y.playoff > 0).count() as u32,
        ..Default::default()
    };
    r.districts = s.iter().rev().take_while(|y| y.title).count() as u32;
    r.playoffs_row = s.iter().rev().take_while(|y| y.playoff > 0).count() as u32;
    // A semifinal: the round before the final, in the class's own field (a
    // finalist played one, whatever the bracket's size that year).
    let fmt = crate::world::hs_playoffs::format(&w.states[w.high_schools[h].state].code);
    let rounds =
        crate::world::hs_playoffs::rounds_for(fmt.field_of(w.high_schools[h].classification as usize) as usize);
    r.semis = s.iter().filter(|y| y.final_game || (y.playoff > 0 && y.playoff as usize + 1 >= rounds)).count() as u32;
    let mut cur = 0;
    for g in w.legacy.games.iter().filter(|g| g.team == team) {
        if g.won() {
            cur += 1;
            r.best_streak = r.best_streak.max(cur);
        } else {
            cur = 0;
        }
    }
    r.streak = cur;
    r
}
