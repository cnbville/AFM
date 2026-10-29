//! Hiring talks (C28-C34, C44-C56): your offer to a coach and the talks
//! after it. He weighs the job (the step up by level and program as well
//! as title, C22, C28; what a promotion is worth to him by his ambition
//! and age, C29, C44), play calling (C30), the money against his pay and
//! his ask (C31), a job outside his group against his adaptability (C39),
//! your promises (C46, C47) and results clauses (C54), the years (C34),
//! your tie to him and your name. His school can counter for real, up to
//! the top of its pay for his job, with a title or the same role (C32,
//! C49-C51); a counter in a worse role doesn't beat your promotion. A no
//! opens talks (C33): he names what it would take, stays at the table
//! while your offers move toward his number, and his patience decides when
//! it is futile (C52). A low offer brings a warning, and the next has to
//! close about a third of the gap (C45, C55). A big move takes him a day
//! or two (C53), unless it's a promotion he can't say no to. Promises are
//! tracked: kept ones build your word, broken ones cost his trust and his
//! network's, a quarter as much when you clearly fought for it (C48, C56).
//!
//! Sizes the list doesn't give are Claude's picks (proposed, not
//! confirmed): a job's standing is its employer's prestige plus 0.2 a rung
//! (position coach 0, coordinator 1, head coach 2, half a rung for a title),
//! so a step of 0.2 is one rung; his money counts 0.6 x the raise and his
//! ask 0.1 + 0.4 x what you add (to 25%) or -1.5 x how far under; the
//! lowball line sits about 25% under his ask (12-40% by man and day); he
//! starts the talks with patience 0.6 (+0.02 a point of loyalty, -0.03 a
//! point of temper) and loses 0.05 a round you move a third of the way,
//! 0.12 or more when you don't; can't-say-no is 0.9 over his bar; the
//! promises' worth and the clauses' odds are in `weigh`.

use super::{Team, coach_key, facts, network};
use crate::calendar::{Date, Loudness};
use crate::rng::clamp;
use crate::world::World;
use crate::world::coach::{Employer, Group, Role};

pub const PROMISE_CALLING: u8 = 1;
pub const PROMISE_TITLE: u8 = 2;
pub const PROMISE_RECRUIT: u8 = 4;
pub const PROMISE_ASSISTANT: u8 = 8;

/// A results clause (C54): the mark and what it pays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClauseTerm {
    /// 1 his side top 25 in points (top 10 in the NFL), 2 the postseason
    /// (a bowl, the playoffs), 3 a player from his room drafted.
    pub mark: u8,
    /// 1 a 15% raise, 2 a year added.
    pub reward: u8,
}

impl ClauseTerm {
    pub fn words(self, nfl: bool) -> String {
        let mark = match self.mark {
            1 if nfl => "his side top 10 in points",
            1 => "his side top 25 in points",
            2 if nfl => "a playoff berth",
            2 => "a bowl",
            _ => "a player from his room drafted",
        };
        let reward = if self.reward == 1 { "a 15% raise" } else { "a year added" };
        format!("{reward} for {mark}")
    }
}

/// What you offer.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Terms {
    pub salary: f64,
    pub years: u8,
    /// Promise bits (C46).
    pub promises: u8,
    pub clauses: Vec<ClauseTerm>,
}

/// Talks with one coach.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Talk {
    pub coach: usize,
    pub role: u64,
    pub opened: Date,
    pub rounds: u8,
    /// His ask when talks opened (C31).
    pub ask: f64,
    /// The salary he named as what it would take (C33).
    pub named: Option<f64>,
    /// Lowball warnings so far, and the salary the next offer has to reach
    /// (C45, C55).
    pub warned: u8,
    pub need: Option<f64>,
    pub patience: f32,
    /// Your last offer.
    pub last: Option<Terms>,
    /// Thinking it over until a day (C53).
    pub thinking: Option<(Date, Terms)>,
    /// His school countered: its pay and the rung it gave him.
    pub counter: Option<(f64, f32)>,
    pub closed: Option<Date>,
    /// What was said, newest last.
    pub log: Vec<(Date, String)>,
}

/// A promise you made to a hire (C46-C48, C56).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Promise {
    pub coach: usize,
    pub kind: u8,
    pub group: Group,
    pub made: Date,
    pub due: Date,
    /// Kept or broken, once settled.
    pub kept: Option<bool>,
    /// You fought for it (C56).
    pub effort: bool,
}

/// A results clause in a hire's deal (C54).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Clause {
    pub coach: usize,
    pub role: u64,
    pub term: ClauseTerm,
    /// The season it reads.
    pub season: i32,
    pub hit: Option<bool>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Hiring {
    pub talks: Vec<Talk>,
    pub promises: Vec<Promise>,
    pub clauses: Vec<Clause>,
    /// Titles given (coach, 1 co-coordinator, 2 assistant head coach).
    pub titles: Vec<(usize, u8)>,
}

pub fn title_words(t: u8) -> &'static str {
    if t == 2 { "assistant head coach" } else { "co-coordinator" }
}

/// A coach's title beside his job, if any.
pub fn title_of(w: &World, c: usize) -> Option<u8> {
    w.folk.hiring.titles.iter().rev().find(|x| x.0 == c).map(|x| x.1)
}

/// His rung on the ladder: position coach 0, coordinator 1, head coach 2;
/// a title half a rung; a junior post half a rung below.
fn rung(role: Option<Role>, title: Option<u8>, junior: bool) -> f64 {
    let r = match role {
        Some(Role::HeadCoach) => 2.0,
        Some(Role::OffensiveCoordinator | Role::DefensiveCoordinator) => 1.0,
        Some(Role::PositionCoach(_)) => 0.0,
        None if junior => -0.5,
        None => -0.3,
    };
    r + if title.is_some() { 0.5 } else { 0.0 }
}

/// A job's standing (C22, C28): its employer's prestige and its rung.
fn standing(w: &World, e: Employer, rung: f64) -> f64 {
    crate::world::staff::employer_prestige(w, e) + 0.2 * rung
}

/// What he makes now.
pub fn pay_now(w: &World, c: usize) -> f64 {
    let co = &w.coaches[c];
    if let Some(d) = w.folk.staff.deal(c) {
        return d.salary;
    }
    match (co.job, co.junior_now()) {
        (Some(j), _) => {
            let base = crate::world::staff::salary(w, j.employer, j.role);
            match (j.role, co.contract) {
                (Role::HeadCoach, Some(k)) => k.salary,
                _ => base * (0.8 + 0.4 * co.reputation / 20.0),
            }
        }
        (None, Some(j)) => crate::world::staff::junior_pay(w, j.employer, j.post),
        _ => 0.0,
    }
}

/// His ask for a job on your staff (C31): the job's market, more for a
/// bigger name, more again to leave a job, and never under what he makes
/// unless it's a step up.
pub fn ask(w: &World, c: usize, role: Role) -> f64 {
    let Some(team) = Team::seat(w) else { return 0.0 };
    let co = &w.coaches[c];
    let base = crate::world::staff::salary(w, team.employer(), role);
    let employed = co.job.is_some();
    let mut a = base * (0.85 + 0.35 * co.reputation / 20.0) * if employed { 1.15 } else { 0.95 };
    let now = pay_now(w, c);
    if employed && step(w, c, role, 0) < 0.15 {
        a = a.max(now * 1.05);
    }
    (a / 1000.0).round() * 1000.0
}

/// The step your job is for him, in standing (one rung = 0.2).
fn step(w: &World, c: usize, role: Role, promises: u8) -> f64 {
    let Some(team) = Team::seat(w) else { return 0.0 };
    let co = &w.coaches[c];
    let title = (promises & PROMISE_TITLE != 0).then_some(1);
    let to = standing(w, team.employer(), rung(Some(role), title, false));
    let from = match (co.job, co.junior_now()) {
        (Some(j), _) => standing(w, j.employer, rung(Some(j.role), title_of(w, c), false)),
        (None, Some(j)) => standing(w, j.employer, rung(None, None, true)),
        (None, None) => {
            co.history.last().map_or(0.2, |s| standing(w, s.employer, rung(Some(s.role), None, false)) - 0.1)
        }
    };
    to - from
}

/// How he weighs an offer: the parts and his bar.
#[derive(Clone, Debug, Default)]
pub struct Weigh {
    pub parts: Vec<(f64, String)>,
    pub score: f64,
    pub bar: f64,
    pub step: f64,
    /// Overwhelming: he takes it on the spot (C53).
    pub cant_say_no: bool,
}

/// What a promotion is worth to him (C29, C44): 0.5 for an average man,
/// 0.2 to 0.8 by ambition; twice that under 45, back to normal by 60.
pub fn promotion_worth(ambition: f64, age: u32) -> f64 {
    let base = clamp(0.5 + 0.03 * (ambition - 10.0), 0.2, 0.8);
    let a = age as f64;
    let young = if a < 45.0 {
        2.0
    } else if a < 60.0 {
        2.0 - (a - 45.0) / 15.0
    } else {
        1.0
    };
    base * young
}

/// He weighs your offer. `now` is what he makes (after a counter).
pub fn weigh(w: &World, c: usize, role: Role, t: &Terms, now: f64) -> Weigh {
    let co = &w.coaches[c];
    let mut parts: Vec<(f64, String)> = Vec::new();
    let a = co.ability();
    let ambition = co.personality.ambition;
    let loyal = (co.personality.loyalty - 10.0) / 10.0;
    // The step (C22, C28, C29, C44).
    let st = step(w, c, role, t.promises);
    let worth = promotion_worth(ambition, co.age);
    let rungs = st / 0.2;
    if rungs > 0.0 {
        parts.push((worth * rungs, "the step up".into()));
    } else if rungs < 0.0 {
        parts.push((0.5 * rungs, "it's a step down for him".into()));
    }
    // A first head job is out of scope here (you hire assistants).
    // Calling (C30).
    let you = w.career.as_ref().map(|k| k.coach);
    let yours = you.and_then(|y| w.coaches[y].yours);
    let side = match role {
        Role::OffensiveCoordinator => Some(true),
        Role::DefensiveCoordinator => Some(false),
        _ => None,
    };
    if let Some(off) = side {
        let you_call = yours.is_some_and(|y| if off { y.calls_offense } else { y.calls_defense });
        let promised = t.promises & PROMISE_CALLING != 0;
        let believe = believe_word(w);
        if you_call && !promised {
            parts.push((-0.1, "you call the plays yourself".into()));
        } else if you_call {
            parts.push((0.2 * believe, "your promise he'd call the plays".into()));
        } else {
            parts.push((0.2 + if promised { 0.05 * believe } else { 0.0 }, "he'd call the plays".into()));
        }
    }
    // The money (C31): against his pay, and against his ask.
    let raise = if now > 0.0 { clamp(t.salary / now - 1.0, -0.5, 1.0) } else { 0.6 };
    parts.push((
        0.6 * raise,
        if raise > 0.15 {
            "a big raise".into()
        } else if raise < 0.0 {
            "a pay cut".into()
        } else {
            String::new()
        },
    ));
    let asked = ask(w, c, role).max(1.0);
    let r = t.salary / asked;
    parts.push((
        if r >= 1.0 { 0.1 + 0.4 * (r - 1.0).min(0.25) } else { -1.5 * (1.0 - r) },
        if r >= 1.0 { "the money's right".into() } else { "the money isn't there".into() },
    ));
    // A job outside his group (C39): less for an adaptable man, not at all
    // for a clear step up.
    let adapt = co.adaptability();
    let outside = match role {
        Role::PositionCoach(g) => {
            g != co.came_up
                && co.job.map(|j| j.role) != Some(Role::PositionCoach(g))
                && !co.history.iter().any(|s| s.role == Role::PositionCoach(g))
        }
        Role::OffensiveCoordinator => !co.came_up.offense(),
        Role::DefensiveCoordinator => co.came_up.offense(),
        Role::HeadCoach => false,
    };
    if outside && st < 0.15 {
        let k = if matches!(role, Role::PositionCoach(_)) { 0.35 } else { 0.4 };
        parts.push((-k * (1.0 - (adapt - 1.0) / 19.0), "it's outside what he coaches".into()));
    }
    // Promises (C46, C47): how much he wants each, times how believable.
    let promise = |bit: u8| t.promises & bit != 0;
    if promise(PROMISE_TITLE) {
        // The title's rung is in the step; this is the pride in it.
        parts.push((0.03 * believe_word(w), String::new()));
    }
    if let (true, Some(g)) = (promise(PROMISE_RECRUIT), group_of(role, co.came_up)) {
        let want = 0.08 + 0.04 * (ambition - 10.0) / 10.0 + 0.03 * (a.teaching[g as usize] - 10.0) / 10.0;
        parts.push((want.max(0.02) * believe_recruit(w, g), "a top recruit to coach".into()));
    }
    if promise(PROMISE_ASSISTANT) {
        parts.push(((0.05 + 0.03 * loyal.max(0.0)) * believe_word(w), "bringing his own assistant".into()));
    }
    // Results clauses (C54): what they're worth to a man who backs himself.
    let conf = 1.0 + 0.03 * (ambition - 10.0);
    for k in t.clauses.iter().take(2) {
        let p = clause_odds(w, k.mark) * conf;
        let v = if k.reward == 1 { 0.6 * 0.15 * p } else { 0.04 * (1.0 + loyal.max(0.0)) * p };
        parts.push((v, String::new()));
    }
    // The years (C34): security, more for a loyal man and an older one.
    let years = t.years.clamp(1, 5) as f64;
    parts.push((
        0.04 * (years - 1.0) * (1.0 + loyal.max(0.0)) + if co.age >= 50 { 0.02 * (years - 1.0) } else { 0.0 },
        if years >= 4.0 { "the years".into() } else { String::new() },
    ));
    // Your tie and your name.
    let warm = network::warmth(w, c) as f64;
    parts.push((0.4 * warm, if warm >= 0.4 { "he knows you".into() } else { String::new() }));
    let rep = facts::your_rep(w) / 20.0;
    parts.push((0.3 * (rep - 0.5), String::new()));
    // Uprooting: a man in a job pays to move; out of work, he wants one.
    let uproot = match (co.job, co.junior_now()) {
        (Some(_), _) => -(0.25 + 0.2 * loyal),
        (None, Some(_)) => -0.05,
        _ => 0.2,
    };
    parts.push((uproot, if uproot < -0.3 { "he's settled where he is".into() } else { String::new() }));
    let score: f64 = parts.iter().map(|p| p.0).sum();
    let u = super::roll(w, 0x0FFE, c as u64 ^ w.today.0 as u64);
    let bar = 0.05 + 0.1 * u;
    Weigh { cant_say_no: score > bar + 0.9, score, bar, step: st, parts }
}

fn group_of(role: Role, up: Group) -> Option<Group> {
    match role {
        Role::PositionCoach(g) => Some(g),
        Role::OffensiveCoordinator if up.offense() => Some(up),
        Role::DefensiveCoordinator if !up.offense() => Some(up),
        Role::OffensiveCoordinator => Some(Group::QB),
        Role::DefensiveCoordinator => Some(Group::DB),
        Role::HeadCoach => None,
    }
}

/// How believable your word is (C47): promises you kept against those you
/// made, from two of three.
pub fn believe_word(w: &World) -> f64 {
    let s = &w.folk.hiring.promises;
    let made = s.iter().filter(|p| p.kept.is_some()).count() as f64;
    let kept = s.iter().filter(|p| p.kept == Some(true)).count() as f64;
    (kept + 2.0) / (made + 3.0)
}

/// How believable a recruit promise is (C47): your last two classes at his
/// position, or your program's pull when you have none yet.
pub fn believe_recruit(w: &World, g: Group) -> f64 {
    let Some(Team::College(p)) = Team::seat(w) else { return 0.2 };
    let year = w.year;
    let n = w
        .people
        .iter()
        .filter(|x| signed_to(x, p) && (year - 1..=year).contains(&x.class_of))
        .filter(|x| x.stars >= 4 && Group::of(x.player.position) == g)
        .count();
    match n {
        0 => 0.25 + 0.45 * w.programs[p].prestige_rel,
        1 => 0.65,
        _ => 0.9,
    }
}

/// The odds a clause's mark is hit, as he'd see them at your program.
fn clause_odds(w: &World, mark: u8) -> f64 {
    let rel = match Team::seat(w) {
        Some(Team::College(p)) => w.programs[p].prestige_rel,
        Some(Team::Nfl(_)) => 0.5,
        _ => 0.3,
    };
    match mark {
        1 => clamp(0.9 * rel, 0.05, 0.8),
        2 => clamp(0.3 + 0.6 * rel, 0.1, 0.9),
        _ => clamp(0.2 + 0.7 * rel, 0.1, 0.9),
    }
}

/// The talks with a coach, if any are open.
pub fn talk(w: &World, c: usize) -> Option<&Talk> {
    w.folk.hiring.talks.iter().rev().find(|t| t.coach == c)
}

fn talk_mut(w: &mut World, c: usize, role: Role) -> &mut Talk {
    let today = w.today;
    let open = w.folk.hiring.talks.iter().rposition(|t| {
        t.coach == c && t.closed.is_none_or(|d| d.days_to(today) < 45) && t.role == super::staffing::role_code(role)
    });
    let k = match open {
        Some(k) => k,
        None => {
            let co = &w.coaches[c];
            let patience =
                clamp(0.6 + 0.02 * (co.personality.loyalty - 10.0) - 0.03 * (co.personality.temper - 10.0), 0.25, 1.0)
                    as f32;
            let ask = ask(w, c, role);
            w.folk.hiring.talks.push(Talk {
                coach: c,
                role: super::staffing::role_code(role),
                opened: today,
                ask,
                patience,
                ..Default::default()
            });
            if w.folk.hiring.talks.len() > 80 {
                w.folk.hiring.talks.remove(0);
            }
            w.folk.hiring.talks.len() - 1
        }
    };
    &mut w.folk.hiring.talks[k]
}

/// The salary that would get him to yes with these terms (C33), if any
/// under two and a half times his ask.
fn would_take(w: &World, c: usize, role: Role, t: &Terms, now: f64) -> Option<f64> {
    let asked = ask(w, c, role).max(1.0);
    let at = |s: f64| {
        let mut x = t.clone();
        x.salary = s;
        let k = weigh(w, c, role, &x, now);
        k.score > 0.12
    };
    let (mut lo, mut hi) = (t.salary.max(asked * 0.5), asked * 2.5);
    if !at(hi) {
        return None;
    }
    for _ in 0..24 {
        let m = 0.5 * (lo + hi);
        if at(m) { hi = m } else { lo = m }
    }
    Some((hi / 5000.0).ceil() * 5000.0)
}

/// His school's counter (C32, C49-C51): only for a man it rates high or
/// one close to a promotion there, rarely in his first year. It pays up to
/// the top of its pay for his job, never above; against a promotion it can
/// give a title, or the role itself when it is open. Returns the pay and
/// the rung it leaves him on, and does it in the world.
fn counter(w: &mut World, c: usize, role: Role, t: &Terms) -> Option<(f64, f32, String)> {
    let co = w.coaches[c].clone();
    let job = co.job?;
    let Employer::College(p) = job.employer else {
        // The NFL blocks laterals instead; a high school can't pay.
        return None;
    };
    let top = crate::world::staff::salary(w, job.employer, job.role) * 1.2;
    let now = pay_now(w, c);
    let rated = co.reputation >= 12.0 || co.units.iter().any(|u| u.seasons >= 6.0 && u.growth_read() >= 0.5);
    let side_coord = match job.role {
        Role::PositionCoach(g) if g != Group::Specialists => {
            if g.offense() {
                w.programs[p].oc
            } else {
                w.programs[p].dc
            }
        }
        _ => None,
    };
    let coord_open = matches!(job.role, Role::PositionCoach(g) if g != Group::Specialists) && side_coord.is_none();
    let close = coord_open || side_coord.is_some_and(|k| w.coaches[k].age >= 63 || w.coaches[k].reputation >= 14.0);
    let first = job.since >= w.year;
    let chance = (if rated {
        0.7
    } else if close {
        0.5
    } else {
        0.08
    }) * if first { 0.2 } else { 1.0 };
    if super::roll(w, 0xC0C7, c as u64 ^ w.today.0 as u64) >= chance {
        return None;
    }
    let mine = rung(Some(role), (t.promises & PROMISE_TITLE != 0).then_some(1), false);
    let his = rung(Some(job.role), title_of(w, c), false);
    let school = Team::College(p).name(w);
    // The role itself, when it's open and yours is that promotion.
    if coord_open && (1.0..1.5).contains(&mine) {
        let r = if job.role.offense() == Some(true) { Role::OffensiveCoordinator } else { Role::DefensiveCoordinator };
        if role == r || matches!(role, Role::OffensiveCoordinator | Role::DefensiveCoordinator) {
            crate::world::staff::leave(w, c, "promoted");
            crate::world::staff::hire(w, c, job.employer, r);
            let pay = (top.max(now * 1.1)).min(crate::world::staff::salary(w, job.employer, r) * 1.2);
            set_pay(w, c, pay);
            return Some((pay, 1.0, format!("{school} promoted him to {} on the spot", r.name())));
        }
    }
    let pay = top.min(now.max(t.salary)).max(now);
    let raised = pay > now * 1.02;
    let title = (mine > his && title_of(w, c).is_none())
        .then_some(if matches!(job.role, Role::OffensiveCoordinator | Role::DefensiveCoordinator) { 2 } else { 1 });
    if !raised && title.is_none() {
        return None;
    }
    if raised {
        set_pay(w, c, pay);
    }
    let mut words = Vec::new();
    if raised {
        words.push(format!("a raise to ${}", facts::thousands((pay / 1000.0) as u32) + ",000"));
    }
    if let Some(tt) = title {
        w.folk.hiring.titles.push((c, tt));
        words.push(format!("the title of {}", title_words(tt)));
    }
    let new_rung = his + if title.is_some() { 0.5 } else { 0.0 };
    Some((pay, new_rung as f32, format!("{school} countered with {}", words.join(" and "))))
}

fn set_pay(w: &mut World, c: usize, pay: f64) {
    let year = w.year;
    match w.folk.staff.deals.iter_mut().find(|d| d.coach == c) {
        Some(d) => {
            d.salary = pay.round();
            d.through = d.through.max(year + 1);
        }
        None => super::staffing::sign(w, c),
    }
}

/// Makes an offer (C23: any coach, any open job on your staff). Returns
/// his answer in words.
pub fn offer(w: &mut World, c: usize, role: Role, mut t: Terms) -> Result<String, String> {
    let team = Team::seat(w).ok_or("You have no team.")?;
    if !super::staffing::openings(w).contains(&role) {
        return Err("That job isn't open on your staff. Let someone go first.".into());
    }
    let co = w.coaches.get(c).ok_or("No such coach.")?.clone();
    if co.retired || co.job.is_some_and(|j| Team::from_employer(j.employer) == team) {
        return Err("He can't take it.".into());
    }
    super::backchannel::hs_hire(w, c)?;
    t.years = t.years.clamp(1, 5);
    t.clauses.truncate(2);
    if side_of(role).is_none() {
        t.promises &= !PROMISE_CALLING;
    }
    if !matches!(team, Team::College(_)) {
        t.promises &= !PROMISE_RECRUIT;
    }
    let today = w.today;
    {
        let tk = talk_mut(w, c, role);
        if tk.thinking.is_some() {
            return Err("He's still thinking it over.".into());
        }
        if tk.closed.is_some() {
            return Err("He's done talking about this job for now.".into());
        }
    }
    let emp = team.employer();
    // An NFL club can block a lateral move under contract.
    let same_level = co.job.is_some_and(|j| std::mem::discriminant(&j.employer) == std::mem::discriminant(&emp));
    if matches!(emp, Employer::Nfl(_)) && same_level && step(w, c, role, 0) < 0.15 && co.job.is_some() {
        let blocks = super::roll(w, 0xB10C, c as u64 ^ today.0 as u64) < 0.7;
        if blocks {
            return Ok(close(
                w,
                c,
                format!(
                    "{}'s club denied permission: a lateral move under contract can be blocked. He stays where he is.",
                    co.name
                ),
            ));
        }
    }
    let k = talk(w, c).cloned().unwrap_or_default();
    let asked = k.ask.max(1.0);
    // The lowball (C45, C55): no fixed line; a warning, then the next offer
    // has to close about a third of the gap.
    let line = asked * (1.0 - clamp(0.25 + 0.06 * super::roll_n(w, 0x10BA ^ c as u64, today.0 as u64), 0.12, 0.4));
    let short_of_need = k.need.is_some_and(|n| t.salary < n);
    if t.salary < line || short_of_need {
        let second = k.warned >= 1 && (short_of_need || k.need.is_none());
        let third = clamp(0.33 + 0.06 * super::roll_n(w, 0x7A1D ^ c as u64, today.0 as u64), 0.2, 0.45);
        let need = (t.salary + (asked - t.salary) * third).max(k.need.unwrap_or(0.0));
        let name = co.name.clone();
        if second {
            network::touch(w, c, -0.05);
        }
        let tk = talk_mut(w, c, role);
        tk.warned += 1;
        tk.need = Some((need / 1000.0).ceil() * 1000.0);
        tk.rounds += 1;
        tk.patience -= if second { 0.2 } else { 0.15 };
        tk.last = Some(t.clone());
        let text = if second {
            format!("{name} is quiet a long moment. \"We're not getting closer.\" It cooled him on you a little.")
        } else {
            format!("{name} laughs, not unkindly. \"That's well short. Come back closer and we'll talk.\"")
        };
        tk.log.push((today, text.clone()));
        return Ok(after_no(w, c, role, &t, text));
    }
    {
        let tk = talk_mut(w, c, role);
        tk.need = None;
    }
    // A high school needs a teaching opening.
    if let Employer::HighSchool(_) = emp {
        let opening = super::roll(w, 0x7EAC, c as u64 ^ w.year as u64) < 0.55;
        if !opening {
            return Ok(close(
                w,
                c,
                format!(
                    "The principal has no teaching opening in {}'s subject this year. The board won't approve a coaching-only hire.",
                    co.name
                ),
            ));
        }
    }
    let mut now = pay_now(w, c);
    let mut wg = weigh(w, c, role, &t, now);
    let mut said = String::new();
    // His school counters a man it means to keep (once in these talks).
    if wg.score > wg.bar && !wg.cant_say_no && k.counter.is_none() && co.job.is_some() {
        let mine = rung(Some(role), (t.promises & PROMISE_TITLE != 0).then_some(1), false);
        if let Some((pay, his_rung, words)) = counter(w, c, role, &t) {
            now = pay;
            wg = weigh(w, c, role, &t, now);
            let tk = talk_mut(w, c, role);
            tk.counter = Some((pay, his_rung));
            said = format!("{words}. ");
            // A counter in the same role or better competes; a raise in a
            // worse role doesn't beat your promotion (C50).
            if his_rung as f64 >= mine {
                wg.score -= 0.15;
            }
            if w.coaches[c].job.map(|j| j.role) != co.job.map(|j| j.role) {
                // They gave him the role itself: he stays (C50).
                let text = format!("{said}He's staying. He thanked you for pushing them.");
                return Ok(close(w, c, text));
            }
        }
    } else if let Some((pay, his_rung)) = k.counter {
        now = now.max(pay);
        wg = weigh(w, c, role, &t, now);
        let mine = rung(Some(role), (t.promises & PROMISE_TITLE != 0).then_some(1), false);
        if his_rung as f64 >= mine {
            wg.score -= 0.15;
        }
    }
    if wg.score <= wg.bar {
        let name = co.name.clone();
        let why = wg
            .parts
            .iter()
            .filter(|p| p.0 < -0.02 && !p.1.is_empty())
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map_or("it isn't enough to move him".to_string(), |p| p.1.clone());
        let text = format!("{said}{name} said no for now: {why}.");
        let tk = talk_mut(w, c, role);
        let moved = match (&tk.last, tk.named) {
            (Some(l), Some(n)) => t.salary - l.salary >= (n - l.salary) / 3.0,
            _ => true,
        };
        tk.rounds += 1;
        tk.patience -= if moved { 0.05 } else { 0.12 + 0.06 * ((co.personality.temper - 10.0) / 10.0).max(0.0) as f32 };
        tk.last = Some(t.clone());
        tk.log.push((today, text.clone()));
        return Ok(after_no(w, c, role, &t, text));
    }
    // Yes. A big move takes a day or two, unless he can't say no (C53).
    let big = co.job.is_some()
        && (std::mem::discriminant(&co.job.unwrap().employer) != std::mem::discriminant(&emp)
            || crate::world::staff::state_of(w, co.job.unwrap().employer) != crate::world::staff::state_of(w, emp));
    if big && !wg.cant_say_no {
        let days = 1 + (super::roll(w, 0x7D4E, c as u64 ^ today.0 as u64) < 0.5) as i32;
        let name = co.name.clone();
        let text = format!(
            "{said}{name} wants {} to talk it over with his family. He'll call you {}.",
            if days == 1 { "a day" } else { "two days" },
            today.add(days).pretty()
        );
        let tk = talk_mut(w, c, role);
        tk.thinking = Some((today.add(days), t.clone()));
        tk.last = Some(t);
        tk.rounds += 1;
        tk.log.push((today, text.clone()));
        return Ok(text);
    }
    let text = hire_now(w, c, role, &t)?;
    Ok(format!("{said}{text}"))
}

fn side_of(role: Role) -> Option<bool> {
    match role {
        Role::OffensiveCoordinator => Some(true),
        Role::DefensiveCoordinator => Some(false),
        _ => None,
    }
}

/// After a no: he names his number, or the talks run out (C33, C52).
fn after_no(w: &mut World, c: usize, role: Role, t: &Terms, text: String) -> String {
    let now = talk(w, c).and_then(|k| k.counter.map(|x| x.0)).unwrap_or_else(|| pay_now(w, c)).max(pay_now(w, c));
    let named = would_take(w, c, role, t, now);
    let name = w.coaches[c].name.clone();
    let tk = talk_mut(w, c, role);
    let futile = tk.patience <= 0.0 || (tk.rounds >= 4 && named.is_none());
    if futile {
        return close(w, c, format!("{text} {name} is done talking: past a point it's futile. He's staying put."));
    }
    tk.named = named.or(tk.named);
    let more = match (named, tk.need) {
        (_, Some(n)) => {
            format!(" The next offer has to be closer: think ${}.", facts::thousands((n / 1000.0) as u32) + ",000")
        }
        (Some(s), None) => format!(" He'd come for ${} a year.", facts::thousands((s / 1000.0) as u32) + ",000"),
        (None, None) => " It isn't about the money.".to_string(),
    };
    let warm = if tk.patience < 0.25 { " His patience is wearing thin." } else { "" };
    format!("{text}{more}{warm}")
}

fn close(w: &mut World, c: usize, text: String) -> String {
    let today = w.today;
    if let Some(k) = w.folk.hiring.talks.iter_mut().rev().find(|t| t.coach == c) {
        k.closed = Some(today);
        k.thinking = None;
        k.log.push((today, text.clone()));
    }
    text
}

/// He's hired: the deal you offered, its promises and clauses.
fn hire_now(w: &mut World, c: usize, role: Role, t: &Terms) -> Result<String, String> {
    let team = Team::seat(w).ok_or("You have no team.")?;
    let emp = team.employer();
    let co = w.coaches[c].clone();
    if co.job.is_some() {
        if let Some(d) = w.folk.staff.deal(c).copied()
            && d.buyout > 0.0
            && d.through > w.year
        {
            let owed = d.salary * (d.through - w.year) as f64 * d.buyout as f64;
            match w.folk.staff.owed.iter_mut().find(|x| x.0 == w.year) {
                Some(x) => x.1 += owed,
                None => w.folk.staff.owed.push((w.year, owed)),
            }
        }
        crate::world::staff::leave(w, c, "left for another job");
    }
    crate::world::staff::hire(w, c, emp, role);
    let years = t.years.clamp(1, 5) as i32;
    w.folk.staff.deals.retain(|d| d.coach != c);
    let buy = if years >= 2 && matches!(emp, Employer::College(_)) { 0.4 } else { 0.0 };
    w.folk.staff.deals.push(super::staffing::Deal {
        coach: c,
        salary: t.salary.round(),
        through: w.year + years - 1,
        buyout: buy,
    });
    let warm = network::warmth(w, c);
    w.folk.mine.set_trust(coach_key(c), 0.6 + 0.2 * warm);
    network::touch(w, c, 0.15);
    let today = w.today;
    let g = group_of(role, co.came_up).unwrap_or(Group::QB);
    // Promises (C46): calling handed over, a title given, an assistant he
    // brings; a recruit due by next signing day.
    let mut kept_now = Vec::new();
    if t.promises & PROMISE_CALLING != 0
        && let (Some(off), Some(you)) = (side_of(role), w.career.as_ref().map(|k| k.coach))
        && let Some(y) = w.coaches[you].yours.as_mut()
    {
        if off {
            y.calls_offense = false;
        } else {
            y.calls_defense = false;
        }
        w.folk.hiring.promises.push(Promise {
            coach: c,
            kind: PROMISE_CALLING,
            group: g,
            made: today,
            due: crate::calendar::Date::ymd(w.year + 1, 1, 20),
            kept: None,
            effort: false,
        });
    }
    if t.promises & PROMISE_TITLE != 0 {
        let tt = if side_of(role).is_some() { 2 } else { 1 };
        w.folk.hiring.titles.push((c, tt));
        kept_now.push(PROMISE_TITLE);
    }
    if t.promises & PROMISE_ASSISTANT != 0 && crate::world::staff::bring_junior(w, emp, g, c) {
        kept_now.push(PROMISE_ASSISTANT);
    }
    for kind in kept_now {
        w.folk.hiring.promises.push(Promise {
            coach: c,
            kind,
            group: g,
            made: today,
            due: today,
            kept: Some(true),
            effort: true,
        });
    }
    if t.promises & PROMISE_RECRUIT != 0 {
        let sd = w.dates.signing_day;
        let due = if today < sd { sd } else { crate::calendar::Date::ymd(w.year + 1, 2, 5) };
        w.folk.hiring.promises.push(Promise {
            coach: c,
            kind: PROMISE_RECRUIT,
            group: g,
            made: today,
            due,
            kept: None,
            effort: false,
        });
    }
    // Results clauses read this season, or the next one past the season.
    let season =
        if today.month() >= 8 || today.month() <= 1 { w.year + if today.month() <= 1 { 0 } else { 1 } } else { w.year };
    for k in &t.clauses {
        w.folk.hiring.clauses.push(Clause {
            coach: c,
            role: super::staffing::role_code(role),
            term: *k,
            season,
            hit: None,
        });
    }
    let text = format!(
        "{} is your new {}: {} year{} at ${} a year.",
        co.name,
        role.name(),
        years,
        if years == 1 { "" } else { "s" },
        facts::thousands((t.salary / 1000.0) as u32) + ",000"
    );
    close(w, c, text.clone());
    super::post_mine(w, today, Loudness::Small, false, "people: staff", &text);
    Ok(text)
}

/// The talks' days: answers after thinking it over (C53), promises (C48,
/// C56) and results clauses (C54) settled.
pub(crate) fn day(w: &mut World, d: Date) {
    // Thinking it over.
    let due: Vec<(usize, u64, Terms)> = w
        .folk
        .hiring
        .talks
        .iter()
        .filter_map(|t| t.thinking.as_ref().filter(|x| x.0 <= d).map(|x| (t.coach, t.role, x.1.clone())))
        .collect();
    for (c, code, terms) in due {
        let role = super::staffing::role_from(code);
        if let Some(t) = w.folk.hiring.talks.iter_mut().rev().find(|t| t.coach == c) {
            t.thinking = None;
        }
        let open = super::staffing::openings(w).contains(&role);
        let now = talk(w, c).and_then(|k| k.counter.map(|x| x.0)).unwrap_or(0.0).max(pay_now(w, c));
        let wg = weigh(w, c, role, &terms, now);
        let name = w.coaches[c].name.clone();
        let text = if !open {
            close(w, c, format!("{name} called back, but the job is filled."))
        } else if wg.score > wg.bar {
            match hire_now(w, c, role, &terms) {
                Ok(t) => format!("{name} called back: yes. {t}"),
                Err(e) => e,
            }
        } else {
            let base = format!("{name} called back: he's staying, but he'd listen to a better offer.");
            if let Some(t) = w.folk.hiring.talks.iter_mut().rev().find(|t| t.coach == c) {
                t.log.push((d, base.clone()));
                t.patience -= 0.05;
            }
            after_no(w, c, role, &terms, base)
        };
        super::post_mine(w, d, Loudness::Small, true, "people: staff", &text);
    }
    settle_promises(w, d);
    if d.month() == 5 && d.day() == 1 {
        settle_clauses(w, d);
    }
}

/// Promises come due (C48, C56): play calling through his first season,
/// a recruit by signing day.
fn settle_promises(w: &mut World, d: Date) {
    let you = w.career.as_ref().map(|k| k.coach);
    let mut out: Vec<(usize, usize, bool, bool)> = Vec::new();
    for (k, p) in w.folk.hiring.promises.iter().enumerate() {
        if p.kept.is_some() {
            continue;
        }
        let on_staff = w.coaches[p.coach].job.is_some_and(|j| Some(Team::from_employer(j.employer)) == Team::seat(w));
        match p.kind {
            PROMISE_CALLING => {
                let calls = you.and_then(|y| w.coaches[y].yours).is_some_and(|y| {
                    match w.coaches[p.coach].job.map(|j| j.role) {
                        Some(Role::OffensiveCoordinator) => y.calls_offense,
                        Some(Role::DefensiveCoordinator) => y.calls_defense,
                        _ => false,
                    }
                });
                if on_staff && calls {
                    out.push((k, p.coach, false, false));
                } else if d >= p.due || !on_staff {
                    out.push((k, p.coach, true, false));
                }
            }
            PROMISE_RECRUIT if d > p.due => {
                let (kept, effort) = recruit_kept(w, p.group, p.due.year());
                out.push((k, p.coach, kept, effort));
            }
            _ => {}
        }
    }
    for (k, c, kept, effort) in out {
        w.folk.hiring.promises[k].kept = Some(kept);
        w.folk.hiring.promises[k].effort = effort;
        let name = w.coaches[c].name.clone();
        let what = if w.folk.hiring.promises[k].kind == PROMISE_CALLING { "the play calling" } else { "a top recruit" };
        if kept {
            w.folk.mine.nudge(coach_key(c), 0.05);
            continue;
        }
        // A broken promise (C48): his trust and his network's ties, a
        // quarter as much when you clearly fought for it (C56).
        let k = if effort { 0.25 } else { 1.0 };
        w.folk.mine.nudge(coach_key(c), -0.2 * k);
        network::touch(w, c, -0.15 * k);
        let mates: Vec<usize> = w
            .folk
            .network
            .contacts
            .iter()
            .filter_map(|x| x.coach_id())
            .filter(|&m| m != c && worked_with(w, m, c))
            .take(6)
            .collect();
        for m in mates {
            network::touch(w, m, -0.04 * k);
        }
        let text = if effort {
            format!("You promised {name} {what} and didn't deliver, but he saw how hard you fought for it.")
        } else {
            format!("You promised {name} {what} and didn't deliver. He hasn't forgotten, and neither have his friends.")
        };
        super::post_mine(w, d, Loudness::Small, false, "people: staff", &text);
    }
}

fn worked_with(w: &World, a: usize, b: usize) -> bool {
    let spans = |c: usize| {
        let co = &w.coaches[c];
        let mut v: Vec<(Employer, i32, i32)> = co.history.iter().map(|s| (s.employer, s.from, s.to)).collect();
        if let Some(j) = co.job {
            v.push((j.employer, j.since, w.year));
        }
        v
    };
    let (x, y) = (spans(a), spans(b));
    x.iter().any(|p| y.iter().any(|q| p.0 == q.0 && p.1 <= q.2 && q.1 <= p.2))
}

/// Signed with the program, or enrolled there out of high school.
fn signed_to(x: &crate::world::Person, p: usize) -> bool {
    x.cal.signed.as_ref().is_some_and(|s| s.program == p) || x.path.first() == Some(&p)
}

/// Whether your class signed a 4- or 5-star at the group, and whether you
/// fought for one (C56: an offer at his market NIL or more, and an
/// official visit).
fn recruit_kept(w: &World, g: Group, class: i32) -> (bool, bool) {
    let Some(Team::College(p)) = Team::seat(w) else { return (false, false) };
    let kept = w
        .people
        .iter()
        .any(|x| x.class_of == class && x.stars >= 4 && Group::of(x.player.position) == g && signed_to(x, p));
    let desk = &w.front_office.college;
    let effort = desk.board.iter().any(|t| {
        t.offered.is_some()
            && super::person(w, t.id).is_some_and(|i| {
                let x = &w.people[i];
                x.class_of == class
                    && x.stars >= 4
                    && Group::of(x.player.position) == g
                    && t.nil.is_none_or(|n| n >= crate::world::front_office::college::usual_offer(w, p, t.id))
            })
            && desk
                .visits
                .iter()
                .any(|v| v.id == t.id && v.kind == crate::world::front_office::college::VisitKind::Official)
    });
    (kept, effort)
}

/// Results clauses settle after the draft (C54).
fn settle_clauses(w: &mut World, d: Date) {
    let season = d.year() - 1;
    let Some(team) = Team::seat(w) else { return };
    let nfl = matches!(team, Team::Nfl(_));
    let mut done: Vec<(usize, bool)> = Vec::new();
    for (k, cl) in w.folk.hiring.clauses.iter().enumerate() {
        if cl.hit.is_some() || cl.season != season {
            continue;
        }
        let on = w.coaches[cl.coach].job.is_some_and(|j| Team::from_employer(j.employer) == team);
        if !on {
            done.push((k, false));
            continue;
        }
        let role = super::staffing::role_from(cl.role);
        let off = role.offense().unwrap_or(true);
        let hit = match cl.term.mark {
            1 => {
                let stat =
                    if off { crate::world::units::Stat::Points } else { crate::world::units::Stat::PointsAllowed };
                crate::world::units::rank(w, team, season, stat).is_some_and(|r| r.0 <= if nfl { 10 } else { 25 })
            }
            2 => match team {
                Team::College(p) => w.programs[p].seasons.iter().any(|s| s.year == season && s.wins >= 7),
                Team::Nfl(t) => {
                    w.nfl_prev.as_ref().is_some_and(|s| s.year == season && s.seed.get(t).is_some_and(|x| x.is_some()))
                }
                _ => false,
            },
            _ => {
                let p = match team {
                    Team::College(p) => Some(p),
                    _ => None,
                };
                w.people.iter().any(|x| {
                    x.pro.as_ref().and_then(|r| r.draft).is_some_and(|dr| dr.0 == d.year())
                        && p.is_some_and(|p| x.college_staff.last().is_some_and(|s| s.1 == p))
                        && match role {
                            Role::PositionCoach(g) => Group::of(x.player.position) == g,
                            _ => Group::of(x.player.position).offense() == off,
                        }
                })
            }
        };
        done.push((k, hit));
    }
    for (k, hit) in done {
        let cl = w.folk.hiring.clauses[k].clone();
        w.folk.hiring.clauses[k].hit = Some(hit);
        if !hit {
            continue;
        }
        let name = w.coaches[cl.coach].name.clone();
        if let Some(dl) = w.folk.staff.deals.iter_mut().find(|x| x.coach == cl.coach) {
            if cl.term.reward == 1 {
                dl.salary = (dl.salary * 1.15).round();
            } else {
                dl.through += 1;
            }
        }
        w.folk.mine.nudge(coach_key(cl.coach), 0.04);
        let text = format!("{name} hit his mark: {}.", cl.term.words(nfl));
        super::post_mine(w, d, Loudness::Small, false, "people: staff", &text);
    }
    w.folk.hiring.clauses.retain(|c| c.season >= season - 3);
}

/// A note about a talk for the screens: his ask, his number, warnings, a
/// counter, how patient he is.
pub fn talk_words(w: &World, c: usize) -> Option<String> {
    let t = talk(w, c)?;
    if t.closed.is_some_and(|d| d.days_to(w.today) >= 45) {
        return None;
    }
    let money = |x: f64| format!("${}", facts::thousands((x / 1000.0) as u32) + ",000");
    let mut v = vec![format!("He asked about {}", money(t.ask))];
    if let Some(n) = t.named {
        v.push(format!("he'd come for {}", money(n)));
    }
    if let Some(n) = t.need {
        v.push(format!("your next offer has to reach about {}", money(n)));
    }
    if let Some((p, r)) = t.counter {
        v.push(format!("his school countered at {}{}", money(p), if r.fract() > 0.0 { " with a title" } else { "" }));
    }
    if let Some((d, _)) = &t.thinking {
        v.push(format!("he'll answer {}", d.pretty()));
    }
    if t.closed.is_some() {
        v.push("the talks are closed".into());
    } else if t.patience < 0.25 {
        v.push("his patience is thin".into());
    }
    let s = v.join("; ");
    Some(format!("{s}."))
}

/// Whether talks are open with him (not closed or long closed).
pub fn talking(w: &World, c: usize) -> bool {
    talk(w, c).is_some_and(|t| t.closed.is_none())
}

/// For the screens: you can't call a side you promised away.
pub fn promised_calling(w: &World, off: bool) -> Option<usize> {
    w.folk.hiring.promises.iter().find_map(|p| {
        (p.kind == PROMISE_CALLING && p.kept.is_none()).then_some(p.coach).filter(|&c| {
            w.coaches[c].job.map(|j| j.role)
                == Some(if off { Role::OffensiveCoordinator } else { Role::DefensiveCoordinator })
        })
    })
}
