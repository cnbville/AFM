//! College's five chases (L6-L10), on the built world: recruiting and
//! pipelines, conference moves, collectives, the portal and the playoff
//! committee. L6 counts your fence around your state's best prospects;
//! L7 reads your brand the way a conference does and pays the invitation
//! when the world's realignment sends one; L8 keeps every school's NFL
//! count by position group over ten years and gives the name to whoever
//! earns it; L9 is your collective, the money you raise and the strings
//! big donors attach; L10 is your résumé in the committee's eyes and the
//! non-conference games you book years ahead. Sizes are proposed, not
//! confirmed.

use super::{ACT_BOOKING, ACT_STRINGS, Bank, key, letter, memo, pay};
use crate::calendar::{Date, Weekday};
use crate::world::World;
use crate::world::coach::{Employer, Group, Role};
use crate::world::data::Division;
use crate::world::desk::{Answer, Head};
use crate::world::stories::TeamKey;
use crate::world::storylines::catalogue::{K, Rating};

/// What the college chases keep.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct College {
    pub fence: Vec<Fence>,
    /// Your brand each season: (year, your pull, the target conference's
    /// average, its name).
    pub brand: Vec<(i32, f32, f32, String)>,
    /// The position group you're "U" for, and since when.
    pub position_u: Option<(u8, i32)>,
    pub strings: Vec<StringAsk>,
    /// This year's collective split: stars, transfers, the class (shares).
    pub split: [f32; 3],
    /// The collective's money each year.
    pub raised: Vec<(i32, f64)>,
    pub booked: Vec<Booking>,
    /// The committee's rank of you each week this season: (date, rank).
    pub resume: Vec<(Date, u8)>,
    /// The years your AD said "not this year" to a building.
    pub refused: Vec<i32>,
    /// The invitation, when one came: (year, conference).
    pub invited: Option<(i32, String)>,
    /// Every school that holds a position's name this year (DBU and the
    /// rest), for every program, read the day after each draft:
    /// (program, group index in `Group::ALL`).
    pub u_names: Vec<(u32, u8)>,
}

/// One class's fence: how many of your state's top 25 signed with you.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Fence {
    pub year: i32,
    pub state: usize,
    pub program: usize,
    pub ours: u8,
    /// The rival who took the most, and how many.
    pub rival: Option<(usize, u8)>,
    /// The best one who left: (name, his rank in the state, where he went).
    pub lost: Option<(String, u8, usize)>,
    /// The state's top prospects counted (25, or fewer in a small state).
    #[serde(default)]
    pub n: u8,
    /// Signed or committed anywhere else, the rival included.
    #[serde(default)]
    pub gone: u8,
}

impl Fence {
    /// Those still deciding.
    pub fn open(&self) -> u8 {
        self.n.saturating_sub(self.ours + self.gone)
    }

    /// Your share of the ones who decided (0 while nobody has).
    pub fn share(&self) -> f32 {
        let decided = self.ours + self.gone;
        if decided == 0 { 0.0 } else { self.ours as f32 / decided as f32 }
    }
}

/// A big donor's string.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StringAsk {
    pub date: Date,
    pub donor: String,
    /// 0 a say in a hire, 1 a player's playing time, 2 a seat at practice.
    pub kind: u8,
    pub amount: f64,
    /// 0 waiting, 1 gave, 2 refused.
    pub answer: u8,
}

/// A non-conference game you booked.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Booking {
    pub year: i32,
    pub program: usize,
    pub opp: usize,
    /// 0 a home-and-home, 1 a neutral-site opener, 2 a home game you buy.
    pub kind: u8,
    pub home: bool,
    pub booked: i32,
}

fn program(w: &World) -> Option<usize> {
    match super::you(w)?.1 {
        TeamKey::College(p) => Some(p as usize),
        _ => None,
    }
}

/// A state's top prospects in a class, best first: the services' stars,
/// then their read.
pub fn state_top(w: &World, state: usize, class: i32, n: usize) -> Vec<usize> {
    let mut v: Vec<(u8, f64, u32, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.home_state == state && p.class_of == class && p.stars >= 3)
        .map(|(i, p)| (p.stars, p.cal.service_read.unwrap_or(0.0), p.player.id, i))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.total_cmp(&a.1)).then(a.2.cmp(&b.2)));
    v.into_iter().take(n).map(|x| x.3).collect()
}

/// L6: a class's fence for a program.
pub fn fence_of(w: &World, p: usize, class: i32) -> Fence {
    let state = w.programs[p].state;
    let top = state_top(w, state, class, 25);
    let mut ours = 0u8;
    let mut gone = 0u8;
    let mut by: Vec<(usize, u8)> = Vec::new();
    let mut lost = None;
    for (rank, &i) in top.iter().enumerate() {
        let person = &w.people[i];
        let Some(to) = person.cal.signed.as_ref().map(|s| s.program).or(person.cal.commit.map(|c| c.0)) else {
            continue;
        };
        if to == p {
            ours += 1;
        } else {
            gone += 1;
            match by.iter_mut().find(|x| x.0 == to) {
                Some(x) => x.1 += 1,
                None => by.push((to, 1)),
            }
            if lost.is_none() {
                lost = Some((person.name.clone(), (rank + 1) as u8, to));
            }
        }
    }
    let rival = by.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)));
    Fence { year: class, state, program: p, ours, rival, lost, n: top.len() as u8, gone }
}

/// L7: your brand against the conference you'd climb to.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Brand {
    pub target: String,
    /// (label, yours, the target's average), each 0-1.
    pub bars: Vec<(String, f64, f64)>,
    /// Your place among the target's members by pull (1 the best).
    pub rank: u32,
    pub members: u32,
    pub interest: bool,
    pub next: String,
    pub risk: String,
}

fn win_share(prog: &crate::world::program::Program, n: usize) -> f64 {
    let s: Vec<_> = prog.seasons.iter().rev().take(n).collect();
    let (wn, l) = s.iter().fold((0, 0), |a, x| (a.0 + x.wins, a.1 + x.losses));
    wn as f64 / (wn + l).max(1) as f64
}

fn academics(t: crate::world::data::AcademicTier) -> f64 {
    use crate::world::data::AcademicTier::*;
    match t {
        Aau => 1.0,
        R1 => 0.75,
        R2 => 0.5,
        Other => 0.25,
    }
}

/// The conference a program would climb to: the poorer power conference
/// for a Group of 5 school, the Group of 5 for an FCS school.
fn target_conf(w: &World, p: usize) -> Option<String> {
    use crate::world::finance::college::{media_value, tier};
    let prog = &w.programs[p];
    let t = tier(w, p, w.year);
    if t == 0 {
        return None;
    }
    let want = if t == 1 { 0 } else { 1 };
    let mut confs: Vec<(f64, String)> = Vec::new();
    for (q, x) in w.programs.iter().enumerate() {
        if x.division != Division::FBS || x.conference == prog.conference || !x.plays(w.year) {
            continue;
        }
        if tier(w, q, w.year) != want {
            continue;
        }
        let v = media_value(w, q);
        match confs.iter_mut().find(|c| c.1 == x.conference) {
            Some(c) => c.0 += v,
            None => confs.push((v, x.conference.clone())),
        }
    }
    // The nearest climb: the conference with the most members in your state
    // or the poorest of the level.
    let in_state = |c: &str| w.programs.iter().filter(|x| x.conference == c && x.state == prog.state).count();
    confs.sort_by(|a, b| in_state(&b.1).cmp(&in_state(&a.1)).then(a.0.total_cmp(&b.0)));
    confs.into_iter().next().map(|c| c.1)
}

/// L7: your brand now, against the conference you'd climb to.
pub fn brand(w: &World, p: usize) -> Option<Brand> {
    let target = target_conf(w, p)?;
    brand_vs(w, p, target)
}

/// L7 for a school already at the top: its brand inside its own
/// conference (realignment leaves the bottom of a list behind). None for an
/// independent.
pub fn brand_home(w: &World, p: usize) -> Option<Brand> {
    let conf = w.programs[p].conference.clone();
    if conf.contains("Independent") {
        return None;
    }
    brand_vs(w, p, conf)
}

fn brand_vs(w: &World, p: usize, target: String) -> Option<Brand> {
    use crate::world::finance::college::media_value;
    let members: Vec<usize> =
        (0..w.programs.len()).filter(|&q| w.programs[q].conference == target && w.programs[q].plays(w.year)).collect();
    if members.is_empty() {
        return None;
    }
    let avg = |f: &dyn Fn(usize) -> f64| members.iter().map(|&q| f(q)).sum::<f64>() / members.len() as f64;
    let budget = |q: usize| crate::world::finance::college::athletic_budget(w, q).unwrap_or(0.0);
    let top_budget = members.iter().map(|&q| budget(q)).fold(1.0, f64::max).max(budget(p));
    let market = |q: usize| {
        let pop = w.states[w.programs[q].state].class_share;
        (pop * 12.0).sqrt().min(1.0)
    };
    let prog = &w.programs[p];
    let bars = vec![
        ("Wins".to_string(), win_share(prog, 5), avg(&|q| win_share(&w.programs[q], 5))),
        ("TV and market".to_string(), market(p), avg(&market)),
        ("Attendance".to_string(), prog.fans, avg(&|q| w.programs[q].fans)),
        ("Facilities".to_string(), prog.facilities, avg(&|q| w.programs[q].facilities)),
        ("Academics".to_string(), academics(prog.academic_tier), avg(&|q| academics(w.programs[q].academic_tier))),
        ("Budget".to_string(), budget(p) / top_budget, avg(&|q| budget(q) / top_budget)),
    ];
    let mine = media_value(w, p);
    let mean = avg(&|q| media_value(w, q));
    let rank = 1 + members.iter().filter(|&&q| media_value(w, q) > mine).count() as u32;
    let interest = mine >= 0.97 * mean;
    let invited = w.realign.iter().any(|r| r.0 == p);
    let weakest =
        bars.iter().filter(|b| b.1 < b.2).min_by(|a, b| (a.1 - a.2).total_cmp(&(b.1 - b.2))).map(|b| b.0.clone());
    let next = if invited {
        "The invitation is accepted: you move next season.".to_string()
    } else if interest {
        format!("You'd raise the {target}'s pay. An invitation comes when it shops.")
    } else {
        match weakest {
            Some(b) => format!("{b}: the gap the {target} looks at first."),
            None => "Keep winning: the pull follows.".into(),
        }
    };
    let risk = if prog.division == Division::FCS {
        "Moving up means a transition: two seasons without a title chance.".into()
    } else {
        format!("A step up is a harder schedule: {} members to play.", members.len())
    };
    Some(Brand { target, bars, rank, members: members.len() as u32, interest, next, risk })
}

/// L8: every school's NFL picks by position group over ten years: the
/// leaders for a group (program, picks).
pub fn position_counts(w: &World, g: Group, year: i32) -> Vec<(usize, u32)> {
    let mut by: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
    for p in &w.people {
        let Some(pro) = p.pro.as_ref() else { continue };
        let Some((y, _, _, _)) = pro.draft else { continue };
        if y < year - 9 || y > year || Group::of(p.player.position) != g {
            continue;
        }
        if let Some(c) = p.college {
            *by.entry(c).or_default() += 1;
        }
    }
    for a in &w.alumni {
        if a.pick.is_none() || a.year < year - 9 || a.year > year || Group::of(a.position) != g {
            continue;
        }
        // Still in the people list as a pro: counted above.
        if let Some(&c) = a.colleges.last()
            && a.pro.is_some()
        {
            *by.entry(c).or_default() += 1;
        }
    }
    let mut v: Vec<(usize, u32)> = by.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// The name a group's leader earns.
pub fn u_name(g: Group) -> &'static str {
    match g {
        Group::QB => "QBU",
        Group::RB => "RBU",
        Group::Receivers => "WRU",
        Group::OL => "O-Line U",
        Group::DL => "D-Line U",
        Group::LB => "Linebacker U",
        Group::DB => "DBU",
        Group::Specialists => "Kicker U",
    }
}

/// The schools that hold a position's name: the leader in NFL picks at a
/// group over ten years with a clear lead (two picks) over the next one
/// and at least three. One pass over the world's players and alumni.
fn u_holders(w: &World) -> Vec<(u32, u8)> {
    let year = w.year;
    let mut by: std::collections::HashMap<(u8, usize), u32> = std::collections::HashMap::new();
    let gi = |g: Group| Group::ALL.iter().position(|&x| x == g).unwrap_or(0) as u8;
    for p in &w.people {
        let Some((y, _, _, _)) = p.pro.as_ref().and_then(|pro| pro.draft) else { continue };
        if y >= year - 9
            && y <= year
            && let Some(c) = p.college
        {
            *by.entry((gi(Group::of(p.player.position)), c)).or_default() += 1;
        }
    }
    for a in &w.alumni {
        if a.pick.is_some()
            && a.year >= year - 9
            && a.year <= year
            && a.pro.is_some()
            && let Some(&c) = a.colleges.last()
        {
            *by.entry((gi(Group::of(a.position)), c)).or_default() += 1;
        }
    }
    let mut out = Vec::new();
    for (k, &g) in Group::ALL.iter().enumerate() {
        if g == Group::Specialists {
            continue;
        }
        let mut v: Vec<(usize, u32)> = by.iter().filter(|e| e.0.0 == k as u8).map(|e| (e.0.1, *e.1)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        if let Some(&(c, n)) = v.first()
            && n >= 3
            && v.get(1).is_none_or(|x| n >= x.1 + 2)
        {
            out.push((c as u32, k as u8));
        }
    }
    out
}

/// The day after each draft, for every world: who holds each name now.
pub(crate) fn world_day(w: &mut World, d: Date) {
    if d == w.dates.draft[2].date.add(1) {
        w.legacy.college.u_names = u_holders(w);
    }
}

/// How much a recruit leans to a school that holds his position's name
/// (L8: LSU's DBU pitch): more for a four- or five-star, who weighs the
/// NFL path most. Proposed, not confirmed: 0.06 and 0.03, a third and a
/// sixth of the childhood lean.
pub(crate) fn u_lean(w: &World, prog: usize, pos: crate::positions::Position, stars: u8) -> f64 {
    let names = &w.legacy.college.u_names;
    if names.is_empty() {
        return 0.0;
    }
    let g = Group::ALL.iter().position(|&x| x == Group::of(pos)).unwrap_or(0) as u8;
    if names.iter().any(|&(c, k)| c as usize == prog && k == g) { if stars >= 4 { 0.06 } else { 0.03 } } else { 0.0 }
}

/// Your best group, its leaders and whether you hold the name: a clear
/// lead (two picks) over the next school.
pub fn position_u(w: &World, p: usize) -> (Group, Vec<(usize, u32)>, bool) {
    let year = w.year;
    let mut best = (Group::DB, Vec::new(), 0u32);
    for g in Group::ALL {
        if g == Group::Specialists {
            continue;
        }
        let v = position_counts(w, g, year);
        let mine = v.iter().find(|x| x.0 == p).map_or(0, |x| x.1);
        if mine > best.2 {
            best = (g, v, mine);
        }
    }
    let (g, v, mine) = best;
    let holds = v.first().is_some_and(|x| x.0 == p) && v.get(1).is_none_or(|x| mine >= x.1 + 2);
    (g, v, holds)
}

/// L10: your résumé in the committee's eyes this season.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Resume {
    pub ranks: Vec<(Date, u8)>,
    pub sos: f64,
    pub quality_wins: u32,
    pub bad_losses: u32,
    pub booked: Vec<String>,
    /// The season it reads: the one under way, or the last one.
    pub season: i32,
}

pub fn resume(w: &World, p: usize) -> Resume {
    let me = TeamKey::College(p as u32);
    // The season under way, or the last one until the new one's first game.
    let has = |y: i32| w.legacy.games.iter().any(|g| g.season == y && g.team == me);
    let year = if has(w.year) || !has(w.year - 1) { w.year } else { w.year - 1 };
    let top: Vec<usize> = w
        .college
        .as_ref()
        .and_then(|c| {
            c.polls.iter().rev().find(|x| {
                x.kind == crate::world::college::PollKind::Cfp || x.kind == crate::world::college::PollKind::Ap
            })
        })
        .map_or(Vec::new(), |x| x.top.iter().take(25).copied().collect());
    let games: Vec<&super::Played> = w.legacy.games.iter().filter(|g| g.season == year && g.team == me).collect();
    let mut sos = 0.0;
    let mut qw = 0;
    let mut bl = 0;
    for g in &games {
        let TeamKey::College(o) = g.opp else { continue };
        let o = o as usize;
        let r = w.college.as_ref().and_then(|c| c.results.get(o));
        let pct = r.map_or(0.5, |r| r.wins as f64 / (r.wins + r.losses).max(1) as f64);
        sos += pct;
        if g.won() && top.contains(&o) {
            qw += 1;
        }
        if !g.won() && !top.contains(&o) && pct < 0.5 {
            bl += 1;
        }
    }
    let sos = sos / games.len().max(1) as f64;
    let booked = w
        .legacy
        .college
        .booked
        .iter()
        .filter(|b| b.program == p && b.year > w.year)
        .map(|b| {
            let kind = match b.kind {
                0 => "home-and-home",
                1 => "neutral-site opener",
                _ => "home game",
            };
            format!("{} {}, {kind}", b.year, w.program_name(b.opp))
        })
        .collect();
    let ranks =
        w.legacy.college.resume.iter().filter(|x| x.0.year() == year || x.0.year() == year + 1).copied().collect();
    Resume { ranks, sos, quality_wins: qw, bad_losses: bl, booked, season: year }
}

/// Every day at your college.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(p) = program(w) else { return };
    if !super::head(w) {
        return;
    }
    let year = w.year;
    // The committee's rank, each Wednesday it's out.
    if d.weekday() == Weekday::Wed
        && let Some(poll) = w.college.as_ref().and_then(|c| c.polls.last())
        && poll.kind == crate::world::college::PollKind::Cfp
        && !w.legacy.college.resume.iter().any(|x| x.0 == poll.date)
    {
        let rank = poll.top.iter().position(|&t| t == p).map_or(0, |k| (k + 1) as u8);
        let date = poll.date;
        w.legacy.college.resume.push((date, rank));
        w.legacy.college.resume.retain(|x| x.0.year() >= year - 1);
    }
    // The day after February signing: the fence.
    if d == w.dates.signing_day.add(1) {
        let f = fence_of(w, p, year + 1);
        w.legacy.college.fence.retain(|x| !(x.year == f.year && x.program == p));
        let share = f.share();
        let (ours, n) = (f.ours, f.n);
        let lost = f.lost.clone();
        w.legacy.college.fence.push(f);
        if share >= 0.4 && ours >= 2 {
            pay(
                w,
                key("L6", p as u64, (year + 1) as u64),
                "L6",
                format!("You kept {ours} of your state's top {n} home"),
                if share >= 0.6 { 0.75 } else { 0.5 },
                Bank::Rating(Rating::Recruiting),
                0.75,
            );
        }
        if let Some((name, rank, to)) = lost
            && rank <= 3
        {
            let rival = w.program_name(to).to_string();
            memo(
                w,
                "legacy-fence-lost",
                format!("{name} signs with {rival}"),
                vec![format!(
                    "The state's No. {rank} prospect signed with {rival}. The fence has a hole the boosters will talk about."
                )],
            );
        }
    }
    // Spring: the AD's scheduling call, a donor's string, the money's split.
    if d == Date::ymd(year, 3, 10) {
        booking_letter(w, p);
    }
    if d == Date::ymd(year, 4, 15) {
        string_letter(w, p);
        let budget = w.programs[p].collective.budget;
        w.legacy.college.raised.retain(|x| x.0 != year);
        w.legacy.college.raised.push((year, budget));
    }
    // A building you asked for and were refused.
    if d == Date::ymd(year, 12, 31)
        && w.finance.college.get(p).is_some_and(|x| x.asked == year)
        && !w.finance.projects.iter().any(|x| x.owner == crate::world::finance::Owner::College(p) && x.started == year)
        && !w.legacy.college.refused.contains(&year)
    {
        w.legacy.college.refused.push(year);
    }
    // The invitation, when the world's realignment sends one.
    if w.legacy.college.invited.is_none()
        && let Some((_, conf)) = w.realign.iter().find(|r| r.0 == p).cloned()
    {
        w.legacy.college.invited = Some((year, conf.clone()));
        pay(
            w,
            key("L7", p as u64, year as u64),
            "L7",
            format!("Invited to the {conf}"),
            1.0,
            Bank::Rating(Rating::Network),
            2.0,
        );
        super::boss_by(w, 0.08);
    }
}

fn booking_letter(w: &mut World, p: usize) {
    let year = w.year;
    let target = year + 4;
    if w.legacy.college.booked.iter().any(|b| b.program == p && b.year == target) {
        return;
    }
    let prog = &w.programs[p];
    let d = prog.division;
    if !matches!(d, Division::FBS | Division::FCS) {
        return;
    }
    // Three real calls: a big name home-and-home, a neutral-site opener on
    // national TV, a home game you pay for.
    let my_p = prog.prestige;
    let pick = |want: &dyn Fn(&crate::world::program::Program) -> bool, salt: u64| -> Option<usize> {
        let mut c: Vec<(f64, usize)> = w
            .programs
            .iter()
            .enumerate()
            .filter(|(q, x)| *q != p && x.plays(year) && x.conference != prog.conference && want(x))
            .map(|(q, _)| {
                (crate::world::season::key_uniform(w.options.seed as u64 ^ salt, (q as u64) << 20 ^ year as u64), q)
            })
            .collect();
        c.sort_by(|a, b| a.0.total_cmp(&b.0));
        c.first().map(|x| x.1)
    };
    let big = pick(&|x| x.division == Division::FBS && x.prestige > my_p + 0.1, 0xB16);
    let neutral = pick(&|x| x.division == Division::FBS && (x.prestige - my_p).abs() < 0.15, 0x7E0);
    let small =
        pick(&|x| if d == Division::FBS { x.division == Division::FCS } else { x.division == Division::D2 }, 0x5A1);
    let ad = w.programs[p].ad.as_ref().map_or("Athletic director".to_string(), |a| a.name.clone());
    let mut answers = Vec::new();
    let name = |q: usize| w.program_name(q).to_string();
    if let Some(q) = big {
        answers.push(Answer {
            label: format!("Home-and-home with {}", name(q)),
            does: "A résumé game twice; a likely loss twice.".into(),
            act: ACT_BOOKING,
            target: (q as u32) << 2,
        });
    }
    if let Some(q) = neutral {
        answers.push(Answer {
            label: format!("Neutral-site opener with {}", name(q)),
            does: "National TV and a big check; a coin flip on opening day.".into(),
            act: ACT_BOOKING,
            target: (q as u32) << 2 | 1,
        });
    }
    if let Some(q) = small {
        answers.push(Answer {
            label: format!("Buy a home game with {}", name(q)),
            does: "A win and a gate; nothing for the résumé.".into(),
            act: ACT_BOOKING,
            target: (q as u32) << 2 | 2,
        });
    }
    if answers.is_empty() {
        return;
    }
    letter(
        w,
        &format!("legacy-booking-{year}"),
        Head::School,
        (ad, "Athletic director".into()),
        format!("A non-conference date in {target}"),
        vec![format!("We have an open date in {target}. Schools book these years ahead. Which call do I return?")],
        answers,
    );
}

/// Your answer on a booking.
pub(crate) fn answer_booking(w: &mut World, target: u32) {
    let Some(p) = program(w) else { return };
    let year = w.year;
    let opp = (target >> 2) as usize;
    let kind = (target & 3) as u8;
    if opp >= w.programs.len() {
        return;
    }
    let first = year + 4;
    w.legacy.college.booked.push(Booking { year: first, program: p, opp, kind, home: kind != 1, booked: year });
    if kind == 0 {
        w.legacy.college.booked.push(Booking { year: first + 1, program: p, opp, kind, home: false, booked: year });
    }
}

/// Booked games for a season, for the schedule builder: (home, away).
pub(crate) fn booked_games(w: &World, season: i32) -> Vec<(usize, usize)> {
    w.legacy
        .college
        .booked
        .iter()
        .filter(|b| b.year == season)
        .map(|b| if b.home { (b.program, b.opp) } else { (b.opp, b.program) })
        .collect()
}

fn string_letter(w: &mut World, p: usize) {
    let year = w.year;
    let prog = &w.programs[p];
    let Some(big) = prog.collective.big_booster else { return };
    if big <= 0.0 || w.legacy.college.strings.iter().any(|s| s.date.year() == year) {
        return;
    }
    let boss =
        crate::world::folk::cast::all(w, crate::world::folk::Team::College(p), crate::world::folk::cast::Role::Booster);
    let Some(donor) = boss.first() else { return };
    let meddle = donor.meddling / 20.0;
    let u = crate::world::season::key_uniform(w.options.seed as u64 ^ 0x5721, (p as u64) << 20 ^ year as u64);
    if u >= 0.25 + 0.4 * meddle {
        return;
    }
    let kind = (crate::world::season::key_uniform(w.options.seed as u64 ^ 0x5722, year as u64) * 3.0) as u8 % 3;
    let amount = (big * 0.25).max(250_000.0);
    let (subject, body) = match kind {
        0 => ("A word on your next hire", "He wants a say in the next assistant you hire."),
        1 => ("About a player", "He wants a player he's close to on the field more."),
        _ => ("A seat at practice", "He wants to watch practice from the field, every week."),
    };
    let name = donor.name.clone();
    let id = letter(
        w,
        &format!("legacy-strings-{year}"),
        Head::Email,
        (name.clone(), "Booster".into()),
        subject.into(),
        vec![
            format!("{name}: {body}"),
            format!("He'd add {} to the collective this year.", crate::world::front_office::dollars(amount)),
        ],
        vec![
            Answer {
                label: "Give it".into(),
                does: "The money comes; your AD notices.".into(),
                act: ACT_STRINGS,
                target: 1,
            },
            Answer {
                label: "Refuse".into(),
                does: "He keeps his money and remembers.".into(),
                act: ACT_STRINGS,
                target: 2,
            },
        ],
    );
    if id.is_some() {
        w.legacy.college.strings.push(StringAsk { date: w.today, donor: name, kind, amount, answer: 0 });
    }
}

/// Your answer to a donor's string.
pub(crate) fn answer_strings(w: &mut World, target: u32) {
    let Some(p) = program(w) else { return };
    let Some(k) = w.legacy.college.strings.iter().rposition(|s| s.answer == 0) else { return };
    w.legacy.college.strings[k].answer = target.min(2) as u8;
    let amount = w.legacy.college.strings[k].amount;
    let c = &mut w.programs[p].collective;
    if target == 1 {
        c.budget += amount;
        super::boss_by(w, -0.02);
    } else {
        c.base = (c.base - amount * 0.5).max(0.0);
        super::boss_by(w, -0.01);
    }
}

/// Your collective split for the year (stars, transfers, the class).
pub fn set_split(w: &mut World, s: [f32; 3]) {
    let t: f32 = s.iter().sum::<f32>().max(0.01);
    w.legacy.college.split = s.map(|x| (x / t).clamp(0.0, 1.0));
}

/// How your split moves one part of your collective's money (0 your stars'
/// deals, 1 transfers, 2 the class): the share you set against the usual
/// one (0.4, 0.3, 0.3), so the same money goes further where you put it.
/// 1.0 at every other program and until you set one. Proposed, not
/// confirmed: the usual shares.
pub(crate) fn split_q(w: &World, j: usize, part: usize) -> f64 {
    const USUAL: [f32; 3] = [0.4, 0.3, 0.3];
    let s = w.legacy.college.split;
    if s.iter().sum::<f32>() <= 0.0 || program(w) != Some(j) {
        return 1.0;
    }
    (s[part] / USUAL[part]) as f64
}

/// After your season: the brand, Position U, the résumé's rewards.
pub(crate) fn season(w: &mut World) {
    let Some(p) = program(w) else { return };
    let year = w.year;
    if let Some(b) = brand(w, p) {
        let mine = crate::world::finance::college::media_value(w, p) as f32;
        let mean = b.bars.len() as f32;
        let _ = mean;
        let target_mean = {
            let members: Vec<usize> = (0..w.programs.len()).filter(|&q| w.programs[q].conference == b.target).collect();
            members.iter().map(|&q| crate::world::finance::college::media_value(w, q)).sum::<f64>()
                / members.len().max(1) as f64
        } as f32;
        w.legacy.college.brand.retain(|x| x.0 != year);
        w.legacy.college.brand.push((year, mine, target_mean, b.target.clone()));
        if b.interest {
            pay(
                w,
                key("L7", p as u64, 1),
                "L7",
                format!("The {} would take you", b.target),
                0.5,
                Bank::Rating(Rating::Network),
                1.0,
            );
        }
    }
    // Position U.
    let (g, _, holds) = position_u(w, p);
    match (holds, w.legacy.college.position_u) {
        (true, None) => {
            w.legacy.college.position_u = Some((g as u8, year));
            let name = u_name(g);
            pay(
                w,
                key("L8", p as u64, g as u64),
                "L8",
                format!("The press calls you {name}"),
                0.75,
                Bank::Teach(g as usize),
                1.5,
            );
            super::story(
                w,
                K::PositionU,
                g as u64,
                name.to_string(),
                format!("The most {} in the NFL from one school in ten years.", g.name().to_lowercase()),
            );
            memo(
                w,
                "legacy-position-u",
                format!("\"{name}\""),
                vec![format!(
                    "More {} from your school went to the NFL in ten years than from anyone. The press has a name for it: {name}.",
                    g.name().to_lowercase()
                )],
            );
        }
        (false, Some((og, since))) => {
            w.legacy.college.position_u = None;
            let name = u_name(Group::ALL[og as usize % Group::ALL.len()]);
            super::cost(w, key("L8", p as u64, 100 + since as u64), "L8", format!("Lost the {name} name"), 0.3);
            super::story_end(w, K::PositionU, og as u64, "lost");
        }
        (true, Some((og, since))) if since < year => super::story_end(w, K::PositionU, og as u64, "claimed"),
        _ => {}
    }
    // The résumé: a playoff place pays, more for a seed with a bye.
    let s = w.legacy.seasons.iter().rev().find(|s| s.year == year && s.team == TeamKey::College(p as u32)).cloned();
    if let Some(s) = s
        && s.playoff > 0
    {
        let bye = s.rank.is_some_and(|r| r <= 4);
        pay(
            w,
            key("L10", p as u64, year as u64),
            "L10",
            if bye { "A playoff seed with a bye".into() } else { "A playoff place".to_string() },
            if bye { 0.75 } else { 0.5 },
            Bank::Rating(Rating::GameManagement),
            1.0,
        );
    }
}

/// Why a new football building matters, when there's room to build.
pub(crate) fn building_why(w: &World, p: usize) -> Option<String> {
    let prog = &w.programs[p];
    if prog.facilities >= 0.85 {
        return None;
    }
    let conf: Vec<f64> = w
        .programs
        .iter()
        .filter(|x| x.conference == prog.conference && x.plays(w.year))
        .map(|x| x.facilities)
        .collect();
    let rank = 1 + conf.iter().filter(|&&f| f > prog.facilities).count();
    Some(format!(
        "Your facilities rank {} of {} in the {}.",
        super::ambitions::ordinal_word(rank as u32),
        conf.len(),
        prog.conference
    ))
}

fn project(w: &World, p: usize) -> Option<&crate::world::finance::Project> {
    w.finance.projects.iter().rev().find(|x| {
        x.owner == crate::world::finance::Owner::College(p) && x.kind == crate::world::finance::ProjectKind::Facility
    })
}

/// The building ambition's steps.
pub(crate) fn building_steps(w: &World, p: usize) -> [bool; 5] {
    let year = w.year;
    match project(w, p) {
        None => [w.finance.college.get(p).is_some_and(|x| x.asked == year), false, false, false, false],
        Some(x) => [true, x.pledged >= 0.4 * x.cost, x.started <= year, x.done, x.done && x.opens <= year - 3],
    }
}

/// The plan was shelved: the AD said no twice in three years.
pub(crate) fn building_shelved(w: &World, p: usize) -> bool {
    let year = w.year;
    project(w, p).is_none() && w.legacy.college.refused.iter().filter(|&&y| y >= year - 2).count() >= 2
}

/// The Reach ambition's steps at a college.
pub(crate) fn reach_steps(w: &World, a: &super::ambitions::Ambition) -> [bool; 5] {
    let set = a.set.unwrap_or(w.today);
    let Some(c) = w.career.as_ref() else { return [false; 5] };
    let head = c.role.is_none();
    let since = |d: Date| d >= set;
    let big = |e: Employer| matches!(e, Employer::College(q) if matches!(w.programs[q].conference.as_str(), "SEC" | "Big Ten"));
    let me = &w.coaches[c.coach];
    let my_job = me.job;
    if a.title.starts_with("A job in") {
        let here: Vec<super::Yr> = super::seasons_at(w, a.team);
        let offer = c.market.offers.iter().any(|o| big(o.employer) && o.role == Role::HeadCoach && since(o.made))
            || c.market.passed.iter().any(|x| big(x.0) && x.1 == Role::HeadCoach && since(x.2));
        [
            here.iter().any(|s| s.post > 0 && s.year >= set.year()),
            here.iter().any(|s| s.rank.is_some() && s.year >= set.year()),
            c.market.applied.iter().any(|x| big(x.0) && since(x.2)),
            offer || my_job.is_some_and(|j| big(j.employer) && j.role == Role::HeadCoach),
            my_job.is_some_and(|j| big(j.employer) && j.role == Role::HeadCoach),
        ]
    } else {
        let is_head = my_job.is_some_and(|j| j.role == Role::HeadCoach) && head;
        let offer = c.market.offers.iter().any(|o| o.role == Role::HeadCoach && since(o.made))
            || c.market.passed.iter().any(|x| x.1 == Role::HeadCoach && since(x.2));
        let winning = is_head
            && my_job
                .and_then(|j| super::team_of_employer(j.employer))
                .is_some_and(|t| super::seasons_at(w, t).iter().any(|s| s.wins > s.losses && s.year >= set.year()));
        [
            matches!(c.role, Some(Role::OffensiveCoordinator | Role::DefensiveCoordinator)) || is_head,
            c.market.applied.iter().any(|x| x.1 == Role::HeadCoach && since(x.2)) || is_head,
            offer || is_head,
            is_head,
            winning,
        ]
    }
}
