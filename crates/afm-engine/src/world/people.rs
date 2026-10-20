//! Section 6, the world side (`docs/design/people.md`, build 9 plan in
//! `docs/design/build9-plan.md`): personality traits that mature, mood
//! and trust, the room (captains and team mood), grudges, the support
//! staff's heads, and off-field trouble for pros. Your own screens (talks,
//! rules, press) come with section 8. Every size here no real source
//! publishes is Claude's, proposed, not confirmed.

use super::background::{Background, Family};
use super::pro::{self, Status, market, money};
use super::season::{key_normal, key_uniform};
use super::{Person, Stage, World};
use crate::calendar::{Date, Kind, Loudness};
use crate::player::{Personality, PlayingTime, Trait};
use crate::positions::Position;
use crate::rng::{Rng, clamp};

// --- Topic 1: traits ----------------------------------------------------------------------

/// A new 14-year-old's traits joined to his family: the family's ambition
/// draw is his ambition (one value, read everywhere), and a steady mind
/// adapts more easily (correlation proposed, not confirmed: 0.2).
pub(crate) fn sync_new(player: &mut crate::player::Player, bg: &Background, family: &Family) {
    let t = &mut player.personality.traits;
    t.set(Trait::Ambition, family.ambition);
    t.shift(Trait::Adaptability, 0.1 * (bg.stability - 10.0));
}

/// The yearly book: people mature (topic 1, "How traits change": from 18
/// into the early thirties professionalism rises and temper settles, on
/// average about 2 points each over those years, more for some, none for
/// others), stars' egos grow, and grudges fade.
pub(crate) fn year_book(w: &mut World) {
    let year = w.year as u64;
    let seed = w.options.seed as u64 ^ 0x9E09_1E00;
    for p in w.people.iter_mut() {
        let age = p.player.age;
        let id = p.player.id as u64;
        if (18..=32).contains(&age) {
            let k = seed ^ year << 32;
            // Each person settles at his own pace (0 to about 5 points).
            let pace = 0.08 + 0.12 * key_uniform(seed ^ 0x9ACE, id);
            if key_uniform(k ^ 1, id) < pace {
                p.player.personality.traits.shift(Trait::Professionalism, 1.0);
            }
            if key_uniform(k ^ 2, id) < pace {
                p.player.personality.traits.shift(Trait::Temper, -1.0);
            }
            if key_uniform(k ^ 3, id) < pace * 0.8 {
                p.bg.stability = (p.bg.stability + 0.6).min(20.0);
            }
        }
        if let Some(c) = p.pro.as_mut() {
            // Stardom feeds the ego; a release humbles some (topic 1).
            let star = c.seasons.iter().any(|s| s.year == w.year && (s.pro_bowl || s.all_pro > 0));
            if star && key_uniform(seed ^ 4 ^ year << 32, id) < 0.4 {
                p.player.personality.traits.shift(Trait::Ego, 1.0);
            }
            // Grudges fade; loyalty and stability soften them faster.
            let soften = 2.0 + 0.1 * (p.bg.stability - 10.0).max(0.0);
            for g in c.grudges.iter_mut() {
                g.1 = (g.1 as f64 - soften).max(0.0) as u8;
            }
            c.grudges.retain(|g| g.1 > 0);
        }
    }
}

// --- Topic 2: grudges ---------------------------------------------------------------------

/// A grudge born from being cut or traded (topic 2): strong when it hurt
/// (years there), with ego and temper; loyalty and stability soften it.
pub(crate) fn grudge(p: &mut Person, team: usize, hurt: f64) {
    let t = &p.player.personality.traits;
    let s = hurt * (6.0 + 3.0 * t.z(Trait::Ego) + 3.0 * t.z(Trait::Temper) - 2.0 * t.z(Trait::Loyalty))
        - 0.15 * (p.bg.stability - 10.0);
    if s < 2.0 {
        return;
    }
    let Some(c) = p.pro.as_mut() else { return };
    let s = clamp(s, 1.0, 20.0) as u8;
    match c.grudges.iter_mut().find(|g| g.0 as usize == team) {
        Some(g) => g.1 = g.1.max(s),
        None => c.grudges.push((team as u8, s)),
    }
}

/// How a free agent weighs a team he holds a grudge against (1 is none,
/// down to 0.7) and one where a coach he played for now works (1.08).
pub(crate) fn ties_q(w: &World, i: usize, t: usize) -> f64 {
    let p = &w.people[i];
    let grudge = p.pro.as_ref().and_then(|c| c.grudges.iter().find(|g| g.0 as usize == t)).map_or(0.0, |g| g.1 as f64);
    let tm = &w.nfl[t];
    let staff = [tm.head_coach, tm.oc, tm.dc];
    let knew = p.college_staff.iter().any(|&(_, _, hc, pc)| staff.iter().any(|&s| s.is_some() && (s == hc || s == pc)));
    (1.0 - 0.015 * grudge) * if knew { 1.08 } else { 1.0 }
}

// --- Topic 10: support staff --------------------------------------------------------------

/// The head of a support department: named, rated 1-20 (hidden), and how
/// long he has been there.
#[derive(Clone, Debug, Default)]
pub struct Staffer {
    pub name: String,
    pub rating: u8,
    pub since: i32,
}

/// The support staff's heads (topic 10): strength and conditioning,
/// medical (the head athletic trainer), analytics and quality control,
/// and academics (college). Their assistants are counted in their rating
/// through the budget.
#[derive(Clone, Debug, Default)]
pub struct Support {
    pub strength: Staffer,
    pub medical: Staffer,
    pub analytics: Staffer,
    pub academic: Staffer,
}

impl Support {
    /// -1 to 1 around an average head.
    pub fn z(r: u8) -> f64 {
        if r == 0 { 0.0 } else { (r as f64 - 10.0) / 10.0 }
    }
}

fn hire(rng: &mut Rng, year: i32, budget: f64, era: f64) -> Staffer {
    let eth = if rng.chance(0.3) { super::ethnicity::Ethnicity::Black } else { super::ethnicity::Ethnicity::White };
    let name = super::names::person_name(eth, year - rng.int(28, 58), rng);
    let rating = clamp(rng.normal(5.5 + 7.0 * budget + 2.0 * era, 2.8), 1.0, 20.0).round() as u8;
    Staffer { name, rating, since: year }
}

/// Fills empty posts and runs the yearly turnover (topic 11 for the AI:
/// about one head in eight moves on a year, a good one at a poor program
/// more often, hired away). A program's budget follows its facilities and
/// division; an NFL club's its owner. Sports science grows by era.
pub(crate) fn staff_support(w: &mut World) {
    let year = w.year;
    let era = clamp((year - 1996) as f64 / 30.0, 0.0, 1.0);
    let mut rng = Rng::new(w.options.seed ^ 0x5077_0000 ^ (year as u32).wrapping_mul(7919));
    for k in 0..w.programs.len() {
        let prog = &w.programs[k];
        if !prog.plays(year) {
            continue;
        }
        let div = match prog.division {
            super::data::Division::FBS => 1.0,
            super::data::Division::FCS => 0.75,
            _ => 0.5,
        };
        let budget = clamp(0.25 + 0.75 * prog.facilities, 0.0, 1.0) * div;
        let s = &mut w.programs[k].support;
        for post in [&mut s.strength, &mut s.medical, &mut s.analytics, &mut s.academic] {
            let leave = 0.12 + if post.rating >= 15 { 0.15 * (1.0 - budget) } else { 0.0 };
            if post.rating == 0 || rng.chance(leave) {
                *post = hire(&mut rng, year, budget, era);
            }
        }
    }
    for t in 0..w.clubs.len() {
        if !super::franchise::active(t, year) {
            continue;
        }
        let budget = clamp(0.55 + 0.03 * (w.nfl[t].owner - 10.0), 0.3, 1.0);
        let s = &mut w.clubs[t].support;
        for post in [&mut s.strength, &mut s.medical, &mut s.analytics] {
            let leave = 0.1 + if post.rating >= 15 { 0.1 * (1.0 - budget) } else { 0.0 };
            if post.rating == 0 || rng.chance(leave) {
                *post = hire(&mut rng, year, budget, era);
            }
        }
    }
}

/// The development inputs the staff and the person give: (physical,
/// medical), each -1 to 1 (`SeasonContext`).
pub(crate) fn support_ctx(s: Option<&Support>, pers: &Personality) -> (f64, f64) {
    let prof = pers.traits.z(Trait::Professionalism);
    match s {
        Some(s) => (0.6 * Support::z(s.strength.rating) + 0.4 * prof, Support::z(s.medical.rating)),
        // No staff (high school): his own habits, and most schools have a
        // trainer part time or none (the CATCH ON study).
        None => (0.4 * prof - 0.2, -0.3),
    }
}

// --- Topic 3: mood and trust --------------------------------------------------------------

/// The main reason behind a mood (`Person::mood_why`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Why {
    Settled = 0,
    Role,
    HappyRole,
    Losing,
    Winning,
    Coach,
    Life,
    Room,
    Money,
}

impl Why {
    pub fn from_u8(v: u8) -> Why {
        match v {
            1 => Why::Role,
            2 => Why::HappyRole,
            3 => Why::Losing,
            4 => Why::Winning,
            5 => Why::Coach,
            6 => Why::Life,
            7 => Why::Room,
            8 => Why::Money,
            _ => Why::Settled,
        }
    }
    pub fn words(self) -> &'static str {
        match self {
            Why::Settled => "settled",
            Why::Role => "unhappy with his role",
            Why::HappyRole => "happy with his role",
            Why::Losing => "frustrated by losing",
            Why::Winning => "enjoying the winning",
            Why::Coach => "doesn't trust his head coach",
            Why::Life => "carrying things away from football",
            Why::Room => "pulled down by the locker room",
            Why::Money => "unhappy with his money",
        }
    }
}

fn role_value(r: PlayingTime) -> f64 {
    match r {
        PlayingTime::Starter => 1.0,
        PlayingTime::Rotation => 0.6,
        PlayingTime::Redshirt => 0.35,
        PlayingTime::Backup | PlayingTime::PracticeSquad => 0.2,
        PlayingTime::Unsigned => 0.0,
    }
}

/// The parts of a mood, -1 to 1 each.
struct Parts {
    role: f64,
    win: f64,
    money: f64,
    life: f64,
    room: f64,
}

/// One mood read (topic 3): the parts weighted by what he is, then
/// blended with where he was (stability sets the swing). Trust in his head
/// coach moves first: a new coach starts it fresh; fair playing time
/// builds it, a role below what he thinks he deserves costs it.
fn read(p: &mut Person, coach: Option<usize>, coach_motivation: f64, parts: Parts) {
    let t = p.player.personality.traits;
    let pers = p.player.personality;
    let cid = coach.map_or(u32::MAX, |c| c as u32);
    if p.trust_in != cid {
        p.trust_in = cid;
        p.trust = clamp(0.5 + 0.08 * t.z(Trait::Loyalty) + 0.08 * t.z(Trait::Adaptability), 0.2, 0.8) as f32;
    }
    let fair = if parts.role >= 0.0 { 0.06 } else { 0.18 * parts.role };
    let trust = clamp(p.trust as f64 + fair + 0.03 * (coach_motivation - 10.0) / 10.0, 0.0, 1.0);
    p.trust = trust as f32;
    let comp = (pers.competitiveness - 10.0) / 10.0;
    let greed = t.z(Trait::Greed);
    let weighted = [
        (Why::Role, Why::HappyRole, 0.4, parts.role),
        (Why::Losing, Why::Winning, 0.15 + 0.1 * comp, parts.win),
        (Why::Coach, Why::Coach, 0.15, (trust - 0.55) * 2.0),
        (Why::Money, Why::Money, 0.1 + 0.15 * greed.max(0.0), parts.money),
        (Why::Life, Why::Life, 0.1, parts.life),
        (Why::Room, Why::Room, 0.12, parts.room),
    ];
    let wsum: f64 = weighted.iter().map(|x| x.2).sum();
    let target = weighted.iter().map(|x| x.2 * x.3).sum::<f64>() / wsum * 1.6;
    let swing = clamp(0.75 + 0.04 * (10.0 - p.bg.stability), 0.5, 1.1);
    let mood = clamp(0.3 * p.mood as f64 + 0.7 * target * swing, -1.0, 1.0);
    p.mood = mood as f32;
    // The reason: the part pulling hardest in the mood's direction.
    let (lo, hi) = weighted.iter().fold(((Why::Settled, 0.0), (Why::Settled, 0.0)), |acc, x| {
        let v = x.2 * x.3;
        let lo = if v < acc.0.1 { (x.0, v) } else { acc.0 };
        let hi = if v > acc.1.1 { (x.1, v) } else { acc.1 };
        (lo, hi)
    });
    p.mood_why = if mood < -0.15 {
        lo.0 as u8
    } else if mood > 0.15 && hi.0 != Why::Coach && hi.0 != Why::Money && hi.0 != Why::Life && hi.0 != Why::Room {
        hi.0 as u8
    } else {
        Why::Settled as u8
    };
}

/// His view of himself (topic 3): his real level, overrated by ego and
/// ambition (proposed, not confirmed: up to about 4 points each).
fn self_view(p: &Person) -> f64 {
    let t = &p.player.personality.traits;
    p.player.overall() + 4.0 * t.z(Trait::Ego).max(0.0) + 1.5 * t.z(Trait::Ambition)
}

/// What he expects for himself on this team: to start if he thinks he is
/// as good as its last starter at his spot, to rotate if close.
fn expected(view: f64, bar: f64) -> f64 {
    if view >= bar {
        1.0
    } else if view >= bar - 4.0 {
        0.6
    } else {
        0.25
    }
}

fn starter_bars(w: &World, roster: &[usize]) -> [f64; crate::positions::POSITION_COUNT] {
    let mut by: [Vec<f64>; crate::positions::POSITION_COUNT] = Default::default();
    for &i in roster {
        by[w.people[i].player.position.index()].push(w.people[i].player.overall());
    }
    let mut bars = [0.0; crate::positions::POSITION_COUNT];
    for pos in Position::ALL {
        let v = &mut by[pos.index()];
        v.sort_by(|a, b| b.total_cmp(a));
        let n = crate::league::depth_slots(pos).starters.max(1);
        bars[pos.index()] = v.get(n - 1).or(v.last()).copied().unwrap_or(0.0);
    }
    bars
}

/// College fall camp's mood read (topics 3 and 4), from the roles the
/// staffs just set: then each program's room votes its captains and
/// settles its mood. Called before the year's development lands, which
/// reads it.
pub(crate) fn college_camp_mood(w: &mut World, roles: &[(usize, PlayingTime)]) {
    let role_of: std::collections::HashMap<usize, PlayingTime> = roles.iter().copied().collect();
    for k in 0..w.programs.len() {
        let roster = w.programs[k].roster.clone();
        if roster.is_empty() {
            continue;
        }
        let bars = starter_bars(w, &roster);
        let last = w.programs[k].seasons.last().map(|s| (s.wins, s.losses));
        let win = last.map_or(0.0, |(a, b)| if a + b == 0 { 0.0 } else { 2.0 * a as f64 / (a + b) as f64 - 1.0 });
        let coach = w.programs[k].head_coach;
        let motivation = coach.map_or(10.0, |c| w.coaches[c].ability().motivation);
        let room = w.programs[k].team_mood as f64;
        for &i in &roster {
            let Some(&role) = role_of.get(&i) else { continue };
            let p = &w.people[i];
            let first = p.college_seasons == 0;
            let exp =
                expected(self_view(p), bars[p.player.position.index()]) - if first && p.stars < 4 { 0.3 } else { 0.0 };
            let amb = p.player.personality.traits.z(Trait::Ambition);
            let parts = Parts {
                role: clamp((role_value(role) - exp) * (0.8 + 0.4 * amb), -1.0, 1.0),
                win,
                money: 0.0,
                life: 0.5 * (p.bg.stability - 10.0) / 10.0,
                room,
            };
            read(&mut w.people[i], coach, motivation, parts);
        }
        settle_room(w, Room::College(k), &roster);
    }
}

/// A state's high school camp: each player's own mood only (no rooms
/// kept), for that state's schools. The bug check of build 9 found one
/// read of every school and every college room for each of the 51
/// states, 22 seconds a season.
pub(crate) fn hs_camp_mood(w: &mut World, schools: &[usize], roles: &[(usize, PlayingTime)]) {
    let role_of: std::collections::HashMap<usize, PlayingTime> = roles.iter().copied().collect();
    for &s in schools {
        let roster = w.high_schools[s].roster.clone();
        if roster.is_empty() {
            continue;
        }
        let bars = starter_bars(w, &roster);
        let coach = w.high_schools[s].coach;
        let motivation = coach.map_or(10.0, |c| w.coaches[c].ability().motivation);
        for &i in &roster {
            let Some(&role) = role_of.get(&i) else { continue };
            let p = &w.people[i];
            let parts = Parts {
                role: clamp(
                    role_value(role) - expected(self_view(p), bars[p.player.position.index()]) + 0.2,
                    -1.0,
                    1.0,
                ),
                win: 0.0,
                money: 0.0,
                life: 0.5 * (p.bg.stability - 10.0) / 10.0,
                room: 0.0,
            };
            read(&mut w.people[i], coach, motivation, parts);
        }
    }
}

/// A pro's pay a year against what he thinks he is worth: the league's
/// read of him on the pay ladder, the price the market itself sets (a
/// backup on the minimum is paid what his place on the ladder pays).
/// Trial 9 bug 6: a straight line from the top five's pay put most of the
/// league under its worth, and NFL moods averaged -0.10. A deal the rules
/// price (the rookie scale, an undrafted or reserve/future deal, the
/// practice squad, a restricted tender) is read against that scale until
/// he may negotiate after three seasons, as the CBA has allowed since 2011;
/// then a star on his rookie deal feels it (proposed, not confirmed).
pub fn pay_and_worth(w: &World, i: usize) -> Option<(f64, f64)> {
    let p = &w.people[i];
    let c = p.pro.as_ref()?;
    let apy = c.contract.as_ref().map_or(0.0, |k| k.apy());
    let scale = c.contract.as_ref().is_some_and(|k| match k.kind {
        money::Kind::Tender => true,
        money::Kind::Rookie | money::Kind::Undrafted | money::Kind::Future | money::Kind::PracticeSquad => {
            c.accrued < 3
        }
        _ => false,
    });
    if scale {
        return Some((apy, apy));
    }
    let pos = p.player.position;
    let view = market::market_grade(w, pos, market::consensus(w, i));
    Some((apy, market::market_price(w.year, pos, view, c.accrued)))
}

/// NFL camp's mood read (topics 3 and 4): role against his own view, last
/// season's record, his pay against his market (greed weights it), the
/// room; then each club's captains and team mood. The year's support
/// staff turnover runs here, once (NFL camp opens before college camp).
pub(crate) fn pro_mood(w: &mut World) {
    staff_support(w);
    let year = w.year;
    for t in 0..w.clubs.len() {
        if !super::franchise::active(t, year) {
            continue;
        }
        let roster: Vec<usize> = w.clubs[t]
            .roster
            .iter()
            .copied()
            .filter(|&i| w.people[i].pro.as_ref().is_some_and(|c| c.status != Status::Reserve))
            .collect();
        if roster.is_empty() {
            continue;
        }
        let bars = starter_bars(w, &roster);
        let win = w.nfl_season.as_ref().filter(|s| s.year == year - 1 && t < s.wins.len()).map_or(0.0, |s| {
            let g = s.wins[t] + s.losses[t] + s.ties[t];
            if g == 0 { 0.0 } else { 2.0 * s.wins[t] as f64 / g as f64 - 1.0 }
        });
        let coach = w.nfl[t].head_coach;
        let motivation = coach.map_or(10.0, |c| w.coaches[c].ability().motivation);
        let room = w.clubs[t].team_mood as f64;
        for &i in &roster {
            let p = &w.people[i];
            let c = p.pro.as_ref().unwrap();
            let role = match c.status {
                Status::PracticeSquad => PlayingTime::PracticeSquad,
                _ => {
                    let depth = &w.clubs[t].depth[p.player.position.index()];
                    let slot = depth.iter().position(|&j| j == i).unwrap_or(99);
                    let s = crate::league::depth_slots(p.player.position);
                    if slot < s.starters {
                        PlayingTime::Starter
                    } else if slot < s.starters + s.rotation {
                        PlayingTime::Rotation
                    } else {
                        PlayingTime::Backup
                    }
                }
            };
            let (apy, worth) = pay_and_worth(w, i).unwrap_or((0.0, 0.0));
            let money = if worth <= 0.0 { 0.0 } else { clamp(apy / worth - 1.0, -1.0, 0.5) };
            let amb = p.player.personality.traits.z(Trait::Ambition);
            let parts = Parts {
                role: clamp(
                    (role_value(role) - expected(self_view(p), bars[p.player.position.index()])) * (0.8 + 0.4 * amb),
                    -1.0,
                    1.0,
                ),
                win,
                money,
                life: 0.5 * (p.bg.stability - 10.0) / 10.0,
                room,
            };
            read(&mut w.people[i], coach, motivation, parts);
        }
        settle_room(w, Room::Club(t), &roster);
    }
}

#[derive(Clone, Copy)]
enum Room {
    College(usize),
    Club(usize),
}

/// The room (topic 4): standing from role, years, star status and the
/// quarterback spot; captains voted on leadership, standing, play and ego
/// (ego wins some early votes, Grijalva 2015), or picked by a strict head
/// coach on leadership and professionalism. Team mood is the moods
/// weighted by standing; leaders in a good mood damp a sour one.
fn settle_room(w: &mut World, room: Room, roster: &[usize]) {
    let (coach, n) = match room {
        Room::College(k) => (w.programs[k].head_coach, 4),
        Room::Club(t) => (w.nfl[t].head_coach, 6),
    };
    let strict = coach.is_some_and(|c| w.coaches[c].ability().discipline >= 14.0);
    let standing = |p: &Person| -> f64 {
        let years = match room {
            Room::College(_) => p.college_seasons as f64,
            Room::Club(t) => {
                p.pro.as_ref().map_or(0.0, |c| c.seasons.iter().filter(|s| s.team == Some(t)).count() as f64)
            }
        };
        let star = (p.player.overall() - 60.0) / 10.0;
        let qb = if p.player.position == Position::QB { 1.0 } else { 0.0 };
        1.0 + 0.3 * years.min(5.0) + star.max(0.0) + qb
    };
    let mut votes: Vec<(usize, f64)> = roster
        .iter()
        .map(|&i| {
            let p = &w.people[i];
            let t = &p.player.personality.traits;
            let lead = t.get(Trait::Leadership);
            let v = if strict {
                lead + 0.5 * t.get(Trait::Professionalism) + 2.0 * standing(p)
            } else {
                lead + 0.3 * t.get(Trait::Ego) + 3.0 * standing(p)
            };
            (i, v + 1.5 * key_normal(w.options.seed as u64 ^ 0xCA97 ^ (w.year as u64) << 20, i as u64))
        })
        .collect();
    votes.sort_by(|a, b| b.1.total_cmp(&a.1));
    let captains: Vec<usize> = votes.iter().take(n).map(|x| x.0).collect();
    let (mut sum, mut wsum) = (0.0, 0.0);
    for &i in roster {
        let p = &w.people[i];
        let wt = standing(p) * if captains.contains(&i) { 2.0 } else { 1.0 };
        sum += wt * p.mood as f64;
        wsum += wt;
    }
    let mut mood = if wsum > 0.0 { sum / wsum } else { 0.0 };
    if mood < 0.0 {
        let lead = captains
            .iter()
            .map(|&i| {
                let p = &w.people[i];
                p.player.personality.traits.z(Trait::Leadership) * if p.mood >= 0.0 { 1.0 } else { 0.3 }
            })
            .sum::<f64>()
            / captains.len().max(1) as f64;
        mood *= clamp(1.0 - 0.4 * lead, 0.6, 1.2);
    }
    let mood = clamp(mood * 1.5, -1.0, 1.0) as f32;
    match room {
        Room::College(k) => {
            w.programs[k].team_mood = mood;
            w.programs[k].captains = captains;
        }
        Room::Club(t) => {
            w.clubs[t].team_mood = mood;
            w.clubs[t].captains = captains;
        }
    }
}

/// What his mood and his room give development (topics 3 and 4): his own
/// mood up to 10% of growth, the room's up to 5%.
pub(crate) fn morale(p: &Person, room: f32) -> f64 {
    clamp(p.mood as f64 + 0.5 * room as f64, -1.5, 1.5)
}

// --- Topic 7: off-field trouble for pros ---------------------------------------------------

/// The kinds of trouble (topic 7), in the real mix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trouble {
    Dui,
    Assault,
    Domestic,
    Drugs,
    Weapons,
    Theft,
    Ped,
    Gambling,
}

impl Trouble {
    pub fn words(self) -> &'static str {
        match self {
            Trouble::Dui => "a DUI arrest",
            Trouble::Assault => "an assault charge after a fight",
            Trouble::Domestic => "a domestic violence case",
            Trouble::Drugs => "a drug case",
            Trouble::Weapons => "a weapons charge",
            Trouble::Theft => "a theft or fraud case",
            Trouble::Ped => "a failed test for performance-enhancing drugs",
            Trouble::Gambling => "a gambling violation",
        }
    }
}

/// A case waiting on the league's ruling.
#[derive(Clone, Debug)]
pub struct Case {
    pub person: usize,
    pub kind: Trouble,
    pub ruling: Date,
    /// Convicted or pleaded (an acquittal or a dropped charge weighs far less).
    pub guilty: bool,
}

/// The real mix of kinds by era (USA Today's arrest database: DUI and
/// domestic cases fell after 2014; marijuana stopped costing games in
/// 2020; sports betting grew from 2018). Shares proposed, not confirmed.
fn kind_mix(year: i32) -> [(Trouble, f64); 8] {
    let after14 = year >= 2015;
    [
        (Trouble::Dui, if after14 { 0.2 } else { 0.28 }),
        (Trouble::Assault, 0.17),
        (Trouble::Domestic, if after14 { 0.09 } else { 0.13 }),
        (Trouble::Drugs, if year >= 2020 { 0.06 } else { 0.13 }),
        (Trouble::Weapons, 0.06),
        (Trouble::Theft, 0.04),
        (Trouble::Ped, 0.11),
        (Trouble::Gambling, if year >= 2018 { 0.05 } else { 0.01 }),
    ]
}

/// The suspension a ruling brings, in games, by the policy of the day:
/// the personal conduct policy from 2007 (domestic cases 6 games from
/// December 2014), DUI 2 games under the 2014 policy and 3 from the 2020
/// CBA, PEDs 4 games and 6 from 2020, substances of abuse 4 games for a
/// repeat offender before 2020 and none after, gambling 2 games for
/// non-NFL bets at a facility from 2023 (a year for betting on games).
fn games_for(kind: Trouble, year: i32, guilty: bool, roll: f64) -> u32 {
    let conduct = year >= 2007;
    match kind {
        Trouble::Dui => match year {
            ..=2013 => 0,
            2014..=2019 => 2,
            _ => 3,
        },
        Trouble::Assault if guilty && conduct => 2 + (roll * 4.0) as u32,
        Trouble::Domestic if conduct && year >= 2015 => {
            if guilty || roll < 0.6 {
                6
            } else {
                0
            }
        }
        Trouble::Domestic if guilty && conduct => 2,
        Trouble::Drugs if year < 2020 && roll < 0.3 => 4,
        Trouble::Weapons if guilty && conduct => 1 + (roll * 3.0) as u32,
        Trouble::Ped => {
            if year >= 2020 {
                6
            } else {
                4
            }
        }
        Trouble::Gambling => {
            if roll < 0.15 {
                17
            } else if year >= 2023 {
                2
            } else {
                4
            }
        }
        _ => 0,
    }
}

/// Each Tuesday, all year (topic 7): a pro's risk of trouble, matched to
/// the NFL's real rate (about 2.5% of players a year, a little more before
/// 2014), scaled by professionalism and stability most, temper, greed
/// (gambling), age (it falls through the twenties) and his room and head
/// coach. A case gets its ruling weeks later; suspended players count
/// their games down in season.
pub(crate) fn pro_trouble_week(w: &mut World, d: Date) {
    let year = w.year;
    let in_season = d > w.dates.nfl_kickoff && d <= w.dates.super_bowl;
    let base = if year < 2014 { 0.03 } else { 0.022 } / 52.0;
    let seed = w.options.seed as u64 ^ 0x7E0B_1E00 ^ (d.0 as u64) << 24;
    let mut new: Vec<(usize, Trouble)> = Vec::new();
    for i in 0..w.people.len() {
        let p = &w.people[i];
        let Some(c) = p.pro.as_ref() else { continue };
        if p.stage != Stage::Pro || !c.held() {
            continue;
        }
        let t = &p.player.personality.traits;
        let age = p.player.age as f64;
        let room = c.team.map_or(0.0, |t| w.clubs[t].team_mood as f64);
        let risk =
            base * clamp(
                1.0 - 0.5 * t.z(Trait::Professionalism) - 0.3 * (p.bg.stability - 10.0) / 10.0
                    + 0.3 * t.z(Trait::Temper),
                0.25,
                2.5,
            ) * clamp(1.0 - 0.04 * (age - 25.0), 0.6, 1.3)
                * (1.0 - 0.15 * room);
        if key_uniform(seed, i as u64) >= risk {
            continue;
        }
        let mut mix = kind_mix(year);
        for m in mix.iter_mut() {
            match m.0 {
                Trouble::Gambling => m.1 *= 1.0 + t.z(Trait::Greed).max(0.0),
                Trouble::Assault | Trouble::Domestic => m.1 *= 1.0 + t.z(Trait::Temper).max(0.0),
                _ => {}
            }
        }
        let total: f64 = mix.iter().map(|m| m.1).sum();
        let mut u = key_uniform(seed ^ 0x51, i as u64) * total;
        let mut kind = Trouble::Dui;
        for m in mix {
            if u < m.1 {
                kind = m.0;
                break;
            }
            u -= m.1;
        }
        new.push((i, kind));
    }
    for (i, kind) in new {
        let guilty = matches!(kind, Trouble::Ped | Trouble::Gambling | Trouble::Dui)
            || key_uniform(seed ^ 0x9117, i as u64) < 0.45;
        let weeks =
            if matches!(kind, Trouble::Ped) { 3 } else { 6 + (key_uniform(seed ^ 0x4EE, i as u64) * 14.0) as i32 };
        w.market.cases.push(Case { person: i, kind, ruling: d.add(weeks * 7), guilty });
        let team = w.people[i].pro.as_ref().and_then(|c| c.team);
        let age = w.people[i].row_age();
        w.people[i].note(format!("Off-field trouble at {age}: {}.", kind.words()));
        w.market.year_mut(year).trouble += 1;
        // The commissioner exempt list (2015 on): paid leave while a
        // violent charge is open.
        if year >= 2015
            && matches!(kind, Trouble::Domestic)
            && !guilty
            && let Some(c) = w.people[i].pro.as_mut()
        {
            c.exempt = weeks as u32;
        }
        if w.recording {
            let who = pro::describe(w, i);
            market::post(w, d, team, Loudness::Medium, Kind::News, format!("{who}: {}.", kind.words()));
        }
    }
    // The rulings due today.
    let due: Vec<Case> = {
        let (now, later): (Vec<Case>, Vec<Case>) =
            std::mem::take(&mut w.market.cases).into_iter().partition(|c| c.ruling <= d);
        w.market.cases = later;
        now
    };
    for case in due {
        rule(w, d, &case);
    }
    if in_season {
        for p in w.people.iter_mut() {
            if let Some(c) = p.pro.as_mut() {
                if c.suspended > 0 && c.team.is_some() {
                    c.suspended -= 1;
                }
                if c.exempt > 0 {
                    c.exempt -= 1;
                }
            }
        }
    } else {
        for p in w.people.iter_mut() {
            if let Some(c) = p.pro.as_mut()
                && c.exempt > 0
            {
                c.exempt -= 1;
            }
        }
    }
}

/// The league rules on a case (topic 7): games without pay, the year's
/// guarantee voided where the deal says so, and a team that cuts a
/// low-value player over a serious case.
fn rule(w: &mut World, d: Date, case: &Case) {
    let year = w.year;
    let i = case.person;
    let roll = key_uniform(w.options.seed as u64 ^ 0x2017 ^ d.0 as u64, i as u64);
    let games = games_for(case.kind, year, case.guilty, roll);
    let Some(c) = w.people[i].pro.as_mut() else { return };
    c.exempt = 0;
    if games == 0 {
        let age = w.people[i].row_age();
        w.people[i].note(format!("At {age} the case ({}) ended without a suspension.", case.kind.words()));
        return;
    }
    c.suspended += games;
    let team = c.team;
    if let Some(k) = c.contract.as_mut()
        && let Some(y) = k.years.iter_mut().find(|y| y.year == year && !y.void)
    {
        let share = games as f64 / 17.0;
        y.base *= (1.0 - share).max(0.0);
        y.guaranteed = 0.0;
        y.injury = 0.0;
    }
    let age = w.people[i].row_age();
    w.people[i].note(format!(
        "Suspended {games} game{} by the NFL at {age} for {}.",
        if games == 1 { "" } else { "s" },
        case.kind.words()
    ));
    w.market.year_mut(year).suspensions += 1;
    let who = pro::describe(w, i);
    market::post(
        w,
        d,
        team,
        Loudness::Medium,
        Kind::News,
        format!("{who} is suspended {games} games ({}).", case.kind.words()),
    );
    // A team cuts a fringe player over a violent case (post-2014 more often).
    if let Some(t) = team
        && matches!(case.kind, Trouble::Domestic | Trouble::Assault | Trouble::Weapons)
        && market::consensus(w, i) < 64.0
        && roll < if year >= 2015 { 0.6 } else { 0.3 }
        && w.clubs[t].office.owner.pet != Some(i)
    {
        pro::release(w, i, d, "released after his suspension");
    }
}

/// Whether a pro can dress: not suspended and not on the exempt list.
pub(crate) fn available(c: &pro::ProCareer) -> bool {
    c.suspended == 0 && c.exempt == 0
}

/// College and high school trouble gets a kind (topic 7): the roll stays
/// as built, the kind comes from the real mix of his level (proposed, not
/// confirmed: no national data).
pub(crate) fn amateur_kind(rng: &mut Rng, college: bool, nil: bool) -> &'static str {
    let mix: &[(&'static str, f64)] = if college {
        &[
            ("an arrest after a fight", 0.24),
            ("a marijuana or drug case", 0.2),
            ("a DUI", 0.12),
            ("academic misconduct", 0.12),
            ("a theft case", 0.08),
            ("a failed drug test", 0.08),
            ("a team rules violation", 0.1),
            (if nil { "a gambling case" } else { "an NCAA benefits case" }, 0.06),
        ]
    } else {
        &[("a fight at school", 0.35), ("failing grades", 0.3), ("a team rules violation", 0.2), ("an arrest", 0.15)]
    };
    let total: f64 = mix.iter().map(|x| x.1).sum();
    let mut u = rng.next() * total;
    for &(k, s) in mix {
        if u < s {
            return k;
        }
        u -= s;
    }
    mix[0].0
}
