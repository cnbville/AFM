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
    if hurt <= 0.0 {
        return;
    }
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
    // A close friend already there pulls a little (section 6, ties;
    // proposed, not confirmed: 4% a friend, two at most).
    let friends = super::folk::ties::friends_on(w, p.player.id, t).min(2) as f64;
    // One of your own guys weighs your club by his bond with you (update 2):
    // a strong one pulls more than having played for the staff, a grudge
    // keeps him away.
    let follow = super::legacy::guys::follow_q(w, p.player.id, t);
    let knew_q = match (knew, follow) {
        (_, f) if f < 1.0 => f,
        (true, f) => f.max(1.08),
        (false, f) => f,
    };
    (1.0 - 0.015 * grudge) * knew_q * (1.0 + 0.04 * friends)
}

// --- Topic 10: support staff --------------------------------------------------------------

/// The head of a support department (realism audit item 10): a person on
/// a market, named, rated 1-20 (hidden), with his age, his pay and how long
/// he has been there. A strength head is his head coach's man: a new head
/// coach nearly always brings his own.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Staffer {
    pub name: String,
    pub rating: u8,
    pub since: i32,
    /// His birth year (0 in older saves until the market's next year).
    #[serde(default)]
    pub born: i32,
    /// Dollars a year.
    #[serde(default)]
    pub pay: f32,
    /// The head coach whose man he is (strength heads only).
    #[serde(default)]
    pub coach: Option<usize>,
}

/// The support staff's heads (topic 10): strength and conditioning,
/// medical (the head athletic trainer), analytics and quality control,
/// and academics (college). Their assistants are counted in their rating
/// through the budget. Player engagement (update 2, Q4): life skills, money
/// and trouble, which a good one makes a little rarer.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Support {
    pub strength: Staffer,
    pub medical: Staffer,
    pub analytics: Staffer,
    pub academic: Staffer,
    #[serde(default)]
    pub engagement: Staffer,
    /// The head coach the market last saw here (item 10): a new one brings
    /// his strength coach. `seen` is false in older saves until it looks.
    #[serde(default)]
    pub boss: Option<usize>,
    #[serde(default)]
    pub seen: bool,
}

impl Support {
    /// -1 to 1 around an average head.
    pub fn z(r: u8) -> f64 {
        if r == 0 { 0.0 } else { (r as f64 - 10.0) / 10.0 }
    }
}

/// A support department.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Dept {
    Strength,
    Medical,
    Analytics,
    Academic,
    Engagement,
}

impl Dept {
    pub const ALL: [Dept; 5] = [Dept::Strength, Dept::Medical, Dept::Analytics, Dept::Academic, Dept::Engagement];
    pub fn name(self) -> &'static str {
        match self {
            Dept::Strength => "strength",
            Dept::Medical => "medical",
            Dept::Analytics => "analytics",
            Dept::Academic => "academic",
            Dept::Engagement => "engagement",
        }
    }
    pub fn post(self, s: &Support) -> &Staffer {
        match self {
            Dept::Strength => &s.strength,
            Dept::Medical => &s.medical,
            Dept::Analytics => &s.analytics,
            Dept::Academic => &s.academic,
            Dept::Engagement => &s.engagement,
        }
    }
    fn post_mut(self, s: &mut Support) -> &mut Staffer {
        match self {
            Dept::Strength => &mut s.strength,
            Dept::Medical => &mut s.medical,
            Dept::Analytics => &mut s.analytics,
            Dept::Academic => &mut s.academic,
            Dept::Engagement => &mut s.engagement,
        }
    }
    /// An NFL club keeps this head (no academic post).
    pub fn nfl(self) -> bool {
        self != Dept::Academic
    }
    /// At a college, paid from football's support staff line and hired by
    /// the head coach; the head athletic trainer and the academic head are
    /// the athletic department's (people topic 10).
    fn football(self) -> bool {
        matches!(self, Dept::Strength | Dept::Analytics | Dept::Engagement)
    }
    /// What a typical head is paid at each level in 2024 dollars: Power 4,
    /// Group of 5, FCS, Division II, NAIA and junior college, Division III,
    /// NFL. From the salary reports Claude knows
    /// (USA Today's strength coach pay: Iowa's Chris Doyle about $800,000 in
    /// 2019, most Power 5 heads $200-500K; head trainers, analytics,
    /// academic and engagement directors from job postings); proposed, not
    /// confirmed.
    fn going_rate(self, level: usize) -> f64 {
        let r: [f64; 6] = match self {
            Dept::Strength => [400e3, 150e3, 80e3, 45e3, 40e3, 350e3],
            Dept::Medical => [200e3, 85e3, 65e3, 50e3, 48e3, 250e3],
            Dept::Analytics => [150e3, 70e3, 50e3, 40e3, 35e3, 250e3],
            Dept::Academic => [110e3, 65e3, 55e3, 45e3, 40e3, 0.0],
            Dept::Engagement => [110e3, 60e3, 50e3, 40e3, 35e3, 150e3],
        };
        r[level.min(5)]
    }
}

/// A support head out of work (item 10): let go, or pushed out for a
/// better one; he takes a job that pays about what he had, and his head
/// coach can bring him back.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Free {
    pub dept: Dept,
    pub who: Staffer,
    /// The year he lost his job.
    pub out: i32,
}

/// One year of the support staff market, for the measures and the reader.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MarketYear {
    pub year: i32,
    /// Posts filled with a new head (an assistant stepping up or a man from
    /// outside the game's posts).
    pub fresh: u16,
    /// Filled from the men out of work.
    pub rehired: u16,
    /// Hired away from another team for a raise.
    pub poached: u16,
    /// Strength heads a new head coach brought with him.
    pub brought: u16,
    /// Strength heads a new head coach kept.
    pub kept: u16,
    /// Strength heads a new head coach let go.
    pub let_go: u16,
    /// Weak heads replaced where the money buys much better.
    pub pushed: u16,
    pub retired: u16,
    /// Left football (another field, family, health).
    pub left: u16,
}

/// The support staff market (item 10): the men out of work and each
/// year's moves.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SupportMarket {
    pub free: Vec<Free>,
    pub years: Vec<MarketYear>,
}

impl SupportMarket {
    fn row(&mut self, year: i32) -> &mut MarketYear {
        if self.years.last().is_none_or(|y| y.year != year) {
            self.years.push(MarketYear { year, ..Default::default() });
        }
        self.years.last_mut().unwrap()
    }
}

/// A team with support heads: a program or an NFL club.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Who {
    Prog(usize),
    Club(usize),
}

fn support_of(w: &World, who: Who) -> &Support {
    match who {
        Who::Prog(p) => &w.programs[p].support,
        Who::Club(t) => &w.clubs[t].support,
    }
}

fn support_mut(w: &mut World, who: Who) -> &mut Support {
    match who {
        Who::Prog(p) => &mut w.programs[p].support,
        Who::Club(t) => &mut w.clubs[t].support,
    }
}

fn boss_of(w: &World, who: Who) -> Option<usize> {
    match who {
        Who::Prog(p) => w.programs[p].head_coach,
        Who::Club(t) => w.nfl.get(t).and_then(|x| x.head_coach),
    }
}

/// The head coach the market reads: none while an interim has the job (a
/// college's interim isn't its head coach; an NFL club's in-season interim
/// is, until Black Monday decides), so an interim changes nothing.
fn permanent_boss(w: &World, who: Who) -> Option<usize> {
    let b = boss_of(w, who)?;
    match who {
        Who::Club(t) if w.folk.staff.interims.iter().any(|x| x.0 as usize == t && x.1 as usize == b) => None,
        _ => Some(b),
    }
}

fn era_of(year: i32) -> f64 {
    clamp((year - 1996) as f64 / 30.0, 0.0, 1.0)
}

/// Every team with support heads this year.
fn teams(w: &World) -> Vec<Who> {
    let year = w.year;
    let progs = (0..w.programs.len()).filter(|&k| w.programs[k].plays(year)).map(Who::Prog);
    let clubs = (0..w.clubs.len()).filter(|&t| super::franchise::active_in(w, t, year)).map(Who::Club);
    progs.chain(clubs).collect()
}

fn depts(who: Who) -> &'static [Dept] {
    match who {
        Who::Prog(_) => &Dept::ALL,
        Who::Club(_) => &[Dept::Strength, Dept::Medical, Dept::Analytics, Dept::Engagement],
    }
}

/// The level a team hires new heads at, 0-1: a program's budget follows its
/// facilities, division and books; an NFL club's its owner. A fresh hire is
/// drawn around it (`hire`).
fn hire_level(w: &World, who: Who) -> f64 {
    match who {
        Who::Prog(k) => {
            let prog = &w.programs[k];
            let div = match prog.division {
                super::data::Division::FBS => 1.0,
                super::data::Division::FCS => 0.75,
                _ => 0.5,
            };
            let budget = clamp(0.25 + 0.75 * prog.facilities, 0.0, 1.0) * div;
            // What the football budget actually buys, from the books.
            match super::finance::college::football_level(w, k) {
                Some(l) => (0.6 * budget + 0.4 * l.max(0.0).sqrt()).min(1.0),
                None => budget,
            }
        }
        // Your owner's raises to the staff pool lift the level your club hires at.
        Who::Club(t) => {
            super::finance::nfl::spend_level(w, t)
                * (1.0 + super::folk::staffing::staff_bonus(w, super::folk::Team::Nfl(t)))
        }
    }
}

/// Each college level's median football staff line and athletic budget
/// this year, the yardstick a program's offers are read against.
struct Yardstick {
    staff: [f64; 5],
    athletic: [f64; 5],
}

/// A program's pay level: Power 4, Group of 5, FCS, Division II with the
/// NAIA and junior colleges, Division III.
fn pay_level(w: &World, k: usize) -> usize {
    match super::finance::college::tier(w, k, w.year) {
        t @ 0..=2 => t as usize,
        _ if w.programs[k].division == super::data::Division::D3 => 4,
        _ => 3,
    }
}

fn yardstick(w: &World) -> Yardstick {
    use super::finance::college;
    let mut staff: [Vec<f64>; 5] = Default::default();
    let mut ath: [Vec<f64>; 5] = Default::default();
    for k in 0..w.programs.len() {
        let Some(d) = w.finance.college.get(k).filter(|d| d.budget.year != 0) else { continue };
        if !w.programs[k].plays(w.year) {
            continue;
        }
        let t = pay_level(w, k);
        staff[t].push(d.budget.l[college::STAFF] as f64);
        ath[t].push(college::Dept::costs(&d.budget));
    }
    let med = |v: &mut Vec<f64>| {
        if v.is_empty() {
            return 0.0;
        }
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    };
    let mut y = Yardstick { staff: [0.0; 5], athletic: [0.0; 5] };
    for t in 0..5 {
        y.staff[t] = med(&mut staff[t]);
        y.athletic[t] = med(&mut ath[t]);
    }
    y
}

/// What a post pays a new head this year (item 10): the level's going rate,
/// moved by the team's money against its level's median (a college's
/// football staff line for the head coach's hires, its athletic budget for
/// the department's; an NFL club's spending level), in the year's dollars.
/// Proposed, not confirmed.
fn offer(w: &World, who: Who, d: Dept, ys: &Yardstick) -> f64 {
    let idx = super::finance::data::coach_index(w.year);
    match who {
        Who::Prog(k) => {
            use super::finance::college;
            let t = pay_level(w, k);
            let ratio = match w.finance.college.get(k).filter(|x| x.budget.year != 0) {
                Some(x) if d.football() && ys.staff[t] > 0.0 => x.budget.l[college::STAFF] as f64 / ys.staff[t],
                Some(x) if ys.athletic[t] > 0.0 => college::Dept::costs(&x.budget) / ys.athletic[t],
                _ => 1.0,
            };
            d.going_rate(t) * clamp(ratio, 0.35, 10.0).powf(0.8) * idx
        }
        Who::Club(_) => d.going_rate(5) * (hire_level(w, who) / 0.55).powf(0.8) * idx,
    }
}

/// The rating a new head drawn at `budget` centres on. The market keeps
/// the best of a fresh man, the men out of work and the heads it can hire
/// away, so a fresh man sits a little under the old single draw
/// (`FRESH_SHIFT`): the lower levels hold where they were before the
/// market (measured with `support_check`; proposed, not confirmed).
fn fresh_mean(budget: f64, era: f64) -> f64 {
    5.5 + 7.0 * budget + 2.0 * era - FRESH_SHIFT
}

/// Measured on seed 1's world, 15 market years: at 0 the FBS and NFL heads
/// climbed 2.5 to 4.5 points; at 1 the lower levels hold (Division II
/// 9.5 to 9.0, Division III 8.8 to 8.8) and the money sorts the best up
/// (FBS strength 11.9 to 13.7, NFL 10.2 to 13.4, flat by the last years).
const FRESH_SHIFT: f64 = 1.0;

fn hire(rng: &mut Rng, year: i32, budget: f64, era: f64) -> Staffer {
    // Minor bug 19: support staff of every background, as the scouts.
    let eth = super::pro::scouts::staff_ethnicity(rng, year);
    let born = year - rng.int(28, 58);
    let name = super::names::person_name(eth, born, rng);
    let rating = clamp(rng.normal(fresh_mean(budget, era), 2.8), 1.0, 20.0).round() as u8;
    Staffer { name, rating, since: year, born, pay: 0.0, coach: None }
}

/// A raise big enough to move a head to another team (item 10; proposed,
/// not confirmed).
const POACH_RAISE: f64 = 1.25;

/// Fills open posts best-paying first (item 10). Each post weighs a new
/// head the budget attracts (an assistant stepping up or a man from
/// outside), the best man out of work who takes the pay, and the best head
/// elsewhere whom the pay moves (a raise of a quarter or more); the best of
/// those who say yes gets the job. A head hired away opens his old post,
/// which the market fills in its turn. A strength head is the head coach's
/// man.
fn fill(w: &mut World, rng: &mut Rng, mut open: Vec<(Who, Dept)>, ys: &Yardstick) {
    let year = w.year;
    let era = era_of(year);
    let mut done: Vec<(Who, Dept)> = Vec::new();
    let all = teams(w);
    while !open.is_empty() {
        // The best-paying post first.
        let (i, pay) = open
            .iter()
            .enumerate()
            .map(|(i, &(who, d))| (i, offer(w, who, d, ys)))
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
            .unwrap();
        let (who, d) = open.swap_remove(i);
        if done.len() > 20_000 {
            break;
        }
        done.push((who, d));
        let fresh = hire(rng, year, hire_level(w, who), era);
        let mut best = (fresh.rating, 0u8, None::<usize>, None::<Who>);
        // The men out of work who'd take it (at least 70% of what they had).
        if let Some((k, f)) = w
            .support_market
            .free
            .iter()
            .enumerate()
            .filter(|(_, f)| f.dept == d && (f.who.pay as f64) * 0.7 <= pay)
            .max_by_key(|(k, f)| (f.who.rating, usize::MAX - k))
            && f.who.rating > best.0
            && rng.chance(0.85)
        {
            best = (f.who.rating, 1, Some(k), None);
        }
        // The best head elsewhere this pay moves. One tied to his head coach
        // mostly stays with him.
        let mut others: Vec<(u8, Who)> = all
            .iter()
            .copied()
            .filter(|&o| o != who && depts(o).contains(&d))
            .filter_map(|o| {
                let s = d.post(support_of(w, o));
                (s.rating > best.0 && s.pay > 0.0 && pay >= POACH_RAISE * s.pay as f64).then_some((s.rating, o))
            })
            .collect();
        others.sort_by_key(|x| std::cmp::Reverse(x.0));
        for (r, o) in others.into_iter().take(3) {
            let s = d.post(support_of(w, o));
            let tied = d == Dept::Strength && s.coach.is_some() && s.coach == permanent_boss(w, o);
            if rng.chance(if tied { 0.25 } else { 0.6 }) {
                best = (r, 2, None, Some(o));
                break;
            }
        }
        let mut man = match best {
            (_, 1, Some(k), _) => {
                w.support_market.row(year).rehired += 1;
                w.support_market.free.swap_remove(k).who
            }
            (_, 2, _, Some(o)) => {
                w.support_market.row(year).poached += 1;
                let old = std::mem::take(d.post_mut(support_mut(w, o)));
                if !open.contains(&(o, d)) {
                    open.push((o, d));
                }
                old
            }
            _ => {
                w.support_market.row(year).fresh += 1;
                fresh
            }
        };
        man.pay = pay.max(man.pay as f64) as f32;
        man.since = year;
        man.coach = if d == Dept::Strength { permanent_boss(w, who) } else { None };
        *d.post_mut(support_mut(w, who)) = man;
    }
}

/// The yearly market (topic 11 for the AI; realism audit item 10), at NFL
/// camp: heads retire (from 60, all by 72), leave football (one in 25), or
/// are pushed out where the money buys a much better one; pay keeps up
/// with the year's dollars; then every open post is filled (`fill`).
/// Sports science grows by era. Sizes proposed, not confirmed.
pub(crate) fn staff_support(w: &mut World) {
    let year = w.year;
    let era = era_of(year);
    let mut rng = Rng::new(w.options.seed ^ 0x5077_0000 ^ (year as u32).wrapping_mul(7919));
    support_bosses(w);
    let ys = yardstick(w);
    let grow = {
        let (a, b) = (super::finance::data::coach_index(year), super::finance::data::coach_index(year - 1));
        if b > 0.0 { a / b } else { 1.0 }
    };
    w.support_market.row(year);
    // The men out of work three years, or past 65, are gone from it.
    w.support_market.free.retain(|f| year - f.out <= 3 && year - f.who.born < 66);
    let mut open = Vec::new();
    for who in teams(w) {
        let level = hire_level(w, who);
        for &d in depts(who) {
            let pay_now = offer(w, who, d, &ys);
            let s = d.post_mut(support_mut(w, who));
            if s.rating == 0 {
                open.push((who, d));
                continue;
            }
            // Older saves: an age from his name, his pay the post's.
            if s.born == 0 {
                let h = s.name.bytes().fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
                s.born = year - 30 - (h % 32) as i32;
            }
            if s.pay <= 0.0 {
                s.pay = pay_now as f32;
            } else {
                s.pay *= grow as f32;
            }
            let age = year - s.born;
            let why = if age >= 72 || (age >= 60 && rng.chance(0.06 + 0.04 * (age - 60) as f64)) {
                1
            } else if rng.chance(0.04) {
                2
            } else if (s.rating as f64) + 4.0 < fresh_mean(level, era) && rng.chance(0.25) {
                3
            } else {
                0
            };
            if why == 0 {
                continue;
            }
            let old = std::mem::take(s);
            let row = w.support_market.row(year);
            match why {
                1 => row.retired += 1,
                2 => row.left += 1,
                _ => {
                    row.pushed += 1;
                    w.support_market.free.push(Free { dept: d, who: old, out: year });
                }
            }
            open.push((who, d));
        }
    }
    fill(w, &mut rng, open, &ys);
}

/// A new head coach brings his own strength coach (item 10; real life: a
/// head coach hires his strength coach, and nearly every new one brings
/// the man he worked with). Run each morning: a team whose head coach
/// changed lets its strength head go (kept one time in seven at a college,
/// three in ten in the NFL), and the new man's own strength coach follows
/// him nine times in ten, from his old job or out of work; otherwise the
/// post goes to the market. An interim head coach changes nothing.
/// Sizes proposed, not confirmed.
pub(crate) fn support_bosses(w: &mut World) {
    let year = w.year;
    let mut changed = Vec::new();
    for who in teams(w) {
        let boss = permanent_boss(w, who);
        let s = support_mut(w, who);
        if !s.seen {
            // Older saves: the strength head is the head coach's man.
            s.seen = true;
            s.boss = boss;
            if s.strength.coach.is_none() && s.strength.rating > 0 {
                s.strength.coach = boss;
            }
            continue;
        }
        if let Some(b) = boss
            && s.boss != Some(b)
        {
            s.boss = Some(b);
            changed.push((who, b));
        }
    }
    if changed.is_empty() {
        return;
    }
    let day = w.today.0 as u32;
    let mut rng = Rng::new(w.options.seed ^ 0x5077_B055 ^ day.wrapping_mul(2_654_435_761));
    let ys = yardstick(w);
    let mut open = Vec::new();
    for (who, b) in changed {
        let cur = &support_of(w, who).strength;
        if cur.rating > 0 && cur.coach == Some(b) {
            continue;
        }
        // His own man: out of work, or at another team.
        let free = w.support_market.free.iter().position(|f| f.dept == Dept::Strength && f.who.coach == Some(b));
        let there = teams(w).into_iter().find(|&o| o != who && support_of(w, o).strength.coach == Some(b));
        let follows = (free.is_some() || there.is_some()) && rng.chance(0.9);
        let keep = if matches!(who, Who::Prog(_)) { 1.0 / 7.0 } else { 0.3 };
        if !follows && cur.rating > 0 && rng.chance(keep) {
            support_mut(w, who).strength.coach = Some(b);
            w.support_market.row(year).kept += 1;
            continue;
        }
        let old = std::mem::take(&mut support_mut(w, who).strength);
        if old.rating > 0 {
            w.support_market.row(year).let_go += 1;
            w.support_market.free.push(Free { dept: Dept::Strength, who: old, out: year });
        }
        if follows {
            let mut man = match (free, there) {
                (Some(k), _) => w.support_market.free.swap_remove(k).who,
                (None, Some(o)) => {
                    open.push((o, Dept::Strength));
                    std::mem::take(&mut support_mut(w, o).strength)
                }
                _ => unreachable!(),
            };
            let pay = offer(w, who, Dept::Strength, &ys);
            man.pay = pay.max(man.pay as f64) as f32;
            man.since = year;
            support_mut(w, who).strength = man;
            w.support_market.row(year).brought += 1;
        } else {
            open.push((who, Dept::Strength));
        }
    }
    fill(w, &mut rng, open, &ys);
}

/// A new team's first heads (lane 10, SK68), hired the day it is founded
/// rather than at the next camp: an expansion club's (`Some(t)`) or a
/// college's starting football (`None`, with its program).
pub(crate) fn first_hires(w: &mut World, club: Option<usize>, program: usize) {
    let year = w.year;
    let key = club.map_or(program as u32, |t| 0x8000 ^ t as u32);
    let mut rng = Rng::new(w.options.seed ^ 0x5077_0E11 ^ key.wrapping_mul(7919) ^ year as u32);
    let who = match club {
        Some(t) => Who::Club(t),
        None => Who::Prog(program),
    };
    let open: Vec<(Who, Dept)> =
        depts(who).iter().copied().filter(|&d| d.post(support_of(w, who)).rating == 0).map(|d| (who, d)).collect();
    let ys = yardstick(w);
    fill(w, &mut rng, open, &ys);
    let boss = permanent_boss(w, who);
    let s = support_mut(w, who);
    s.seen = true;
    s.boss = boss;
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// An AI head coach's manner on him over the season (coach voice,
    /// `persona::manner::ai_fit`): a trust change. 0 under you, whose
    /// lines land one by one.
    coach_fit: f64,
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
    let trust = clamp(p.trust as f64 + fair + 0.03 * (coach_motivation - 10.0) / 10.0 + parts.coach_fit, 0.0, 1.0);
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
pub(crate) fn self_view(p: &Person) -> f64 {
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
                coach_fit: super::folk::persona::manner::ai_fit(w, coach, i),
            };
            read(&mut w.people[i], coach, motivation, parts);
        }
        settle_room(w, Room::College(k), &roster, &role_of);
    }
    let mut read_ids: Vec<usize> = role_of.keys().copied().collect();
    read_ids.sort_unstable();
    super::folk::talks::reapply(w, &read_ids);
}

/// A state's high school camp: each player's own mood only (no rooms
/// kept), for that state's schools. The bug check of build 9 found one
/// read of every school and every college room for each of the 51
/// states, 22 seconds a season.
pub(crate) fn hs_camp_mood(w: &mut World, schools: &[usize], roles: &[(usize, PlayingTime)]) {
    use rayon::prelude::*;
    let role_of: std::collections::HashMap<usize, PlayingTime> = roles.iter().copied().collect();
    // Each school's reads worked out on all cores from what the reads
    // don't change (ratings, traits, home), then made in school order
    // (world speed lane).
    let world: &World = w;
    let reads: Vec<(usize, Option<usize>, f64, Parts)> = schools
        .par_iter()
        .flat_map_iter(|&s| {
            let roster = &world.high_schools[s].roster;
            let mut out = Vec::new();
            if roster.is_empty() {
                return out;
            }
            let bars = starter_bars(world, roster);
            let coach = world.high_schools[s].coach;
            let motivation = coach.map_or(10.0, |c| world.coaches[c].ability().motivation);
            for &i in roster {
                let Some(&role) = role_of.get(&i) else { continue };
                let p = &world.people[i];
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
                    coach_fit: super::folk::persona::manner::ai_fit(world, coach, i),
                };
                out.push((i, coach, motivation, parts));
            }
            out
        })
        .collect();
    let mut read_ids = Vec::with_capacity(reads.len());
    for (i, coach, motivation, parts) in reads {
        read(&mut w.people[i], coach, motivation, parts);
        read_ids.push(i);
    }
    read_ids.sort_unstable();
    super::folk::talks::reapply(w, &read_ids);
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
    Some((apy, market::market_price(w, pos, view, c.credited, p.player.age)))
}

/// A pro's trust in his club, read at camp beside his trust in the head
/// coach (minor bug 114: it began at 1 and only fell, so it averaged 0.99
/// and drove nothing). A club he has just joined starts at three quarters
/// (it chose him; starting at a half, like a new head coach, held out 55%
/// more players a year than before), a tenth higher when it has just paid
/// him; after that a fair role earns a little each year and a short one
/// costs, as in college. Pay under his market wears it, the more for a
/// greedy man, and so does each tag he plays under (real players resent
/// the tag: Le'Veon Bell sat out 2018 rather than play a second one).
/// Broken promises and shopping rumors still cost it on their days. Sizes
/// proposed, not confirmed.
fn club_trust(p: &mut Person, role: f64, pay: f64) {
    let t = &p.player.personality.traits;
    let (loyal, adapt, greed) = (t.z(Trait::Loyalty), t.z(Trait::Adaptability), t.z(Trait::Greed));
    let Some(c) = p.pro.as_mut() else { return };
    let mut x = if c.new_team {
        clamp(0.75 + 0.08 * loyal + 0.08 * adapt, 0.5, 0.95)
    } else {
        c.trust + if role >= 0.0 { 0.06 } else { 0.18 * role }
    };
    if c.paid {
        x += 0.1;
    }
    if pay < 0.0 {
        x += 0.1 * pay * clamp(1.0 + 0.3 * greed, 0.4, 1.6);
    }
    let tagged =
        c.contract.as_ref().is_some_and(|k| matches!(k.kind, money::Kind::Franchise | money::Kind::Transition));
    if tagged {
        x -= 0.08 * c.tags as f64;
    }
    c.trust = clamp(x, 0.0, 1.0);
}

/// NFL camp's mood read (topics 3 and 4): role against his own view, last
/// season's record, his pay against his market (greed weights it), the
/// room; then each club's captains and team mood. The year's support
/// staff turnover runs here, once (NFL camp opens before college camp).
pub(crate) fn pro_mood(w: &mut World) {
    staff_support(w);
    let year = w.year;
    let mut read_ids = Vec::new();
    for t in 0..w.clubs.len() {
        if !super::franchise::active_in(w, t, year) {
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
        let mut roles = std::collections::HashMap::new();
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
            roles.insert(i, role);
            let (apy, worth) = pay_and_worth(w, i).unwrap_or((0.0, 0.0));
            let money = if worth <= 0.0 { 0.0 } else { clamp(apy / worth - 1.0, -1.0, 0.5) };
            let amb = p.player.personality.traits.z(Trait::Ambition);
            // A rookie taken after round 2, or undrafted, knows he is
            // earning his place, as a college freshman under four stars
            // does; and a role at least as big as he thinks he deserves is
            // a lift, not nothing. Trial 9 finding 9: an NFL starter's role
            // part could not go above zero, and 10 of 2,141 NFL players
            // were happy with their role against 13,168 in college
            // (proposed, not confirmed: the 0.3 and the 0.2).
            let earning = c.first_year == year && c.draft.is_none_or(|d| d.1 >= 3);
            let exp = expected(self_view(p), bars[p.player.position.index()]) - if earning { 0.3 } else { 0.0 };
            let got = role_value(role);
            let lift = if got >= exp { 0.2 } else { 0.0 };
            let parts = Parts {
                role: clamp((got - exp + lift) * (0.8 + 0.4 * amb), -1.0, 1.0),
                win,
                money,
                life: 0.5 * (p.bg.stability - 10.0) / 10.0,
                room,
                coach_fit: super::folk::persona::manner::ai_fit(w, coach, i),
            };
            let role = parts.role;
            read(&mut w.people[i], coach, motivation, parts);
            club_trust(&mut w.people[i], role, money);
        }
        read_ids.extend_from_slice(&roster);
        settle_room(w, Room::Club(t), &roster, &roles);
    }
    read_ids.sort_unstable();
    super::folk::talks::reapply(w, &read_ids);
}

/// What a starter's and a rotation player's role add to his standing in
/// the room (minor bug 184; proposed, not confirmed).
const ROLE_STANDING: (f64, f64) = (2.5, 1.25);

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
enum Room {
    College(usize),
    Club(usize),
}

/// The room (topic 4): standing from role, years, star status and the
/// quarterback spot; captains voted on leadership, standing, play and ego
/// (ego wins some early votes, Grijalva 2015), or picked by a strict head
/// coach on leadership and professionalism. Team mood is the moods
/// weighted by standing; leaders in a good mood damp a sour one.
///
/// His role this season is part of his standing (minor bug 184): a room
/// votes the men it watches play, and a strict coach picks among the men
/// he plays. Real captains are starters (Ohio State's, Alabama's and
/// Michigan's 2025 captains all were). Without it a three-year backup with
/// 22 snaps was one of South Carolina's captains on leadership alone, and
/// 47% of FBS captains were backups or redshirts. A starter +2.5 and a
/// rotation player +1.25 (proposed, not confirmed): seed 1 played on to
/// fall camp 2026 votes 92% starters, 5% rotation players and 2.7%
/// backups or redshirts.
fn settle_room(w: &mut World, room: Room, roster: &[usize], roles: &std::collections::HashMap<usize, PlayingTime>) {
    let (coach, n) = match room {
        Room::College(k) => (w.programs[k].head_coach, 4),
        Room::Club(t) => (w.nfl[t].head_coach, 6),
    };
    let strict = coach.is_some_and(|c| w.coaches[c].ability().discipline >= 14.0);
    // His standing before his role this season: years, star status, the
    // quarterback spot.
    let base = |i: usize| -> f64 {
        let p = &w.people[i];
        let years = match room {
            Room::College(_) => p.college_seasons as f64,
            Room::Club(t) => {
                p.pro.as_ref().map_or(0.0, |c| c.seasons.iter().filter(|s| s.team == Some(t)).count() as f64)
            }
        };
        // A kicker's or snapper's rating isn't a room's standing: the
        // specialists rarely lead one (bug 34).
        let spec = if p.player.position.is_specialist() { 0.25 } else { 1.0 };
        let star = spec * (p.player.overall() - 60.0) / 10.0;
        let qb = if p.player.position == Position::QB { 1.0 } else { 0.0 };
        1.0 + 0.3 * years.min(5.0) + star.max(0.0) + qb
    };
    let role = |i: usize| match roles.get(&i) {
        Some(PlayingTime::Starter) => ROLE_STANDING.0,
        Some(PlayingTime::Rotation) => ROLE_STANDING.1,
        _ => 0.0,
    };
    let standing = |i: usize| base(i) + role(i);
    let mut votes: Vec<(usize, f64)> = roster
        .iter()
        // A college team doesn't make a first-year player a captain.
        .filter(|&&i| !matches!(room, Room::College(_)) || w.people[i].college_seasons > 0 || roster.len() < 8)
        .map(|&i| {
            let p = &w.people[i];
            let t = &p.player.personality.traits;
            let lead = t.get(Trait::Leadership);
            // A coach who picks his own captains picks among the men he
            // plays, as surely as a room does.
            let v = if strict {
                lead + 0.5 * t.get(Trait::Professionalism) + 2.0 * base(i) + 3.0 * role(i)
            } else {
                lead + 0.3 * t.get(Trait::Ego) + 3.0 * standing(i)
            };
            (i, v + 1.5 * key_normal(w.options.seed as u64 ^ 0xCA97 ^ (w.year as u64) << 20, i as u64))
        })
        .collect();
    votes.sort_by(|a, b| b.1.total_cmp(&a.1));
    let captains: Vec<usize> = votes.iter().take(n).map(|x| x.0).collect();
    let (mut sum, mut wsum) = (0.0, 0.0);
    for &i in roster {
        let p = &w.people[i];
        let wt = standing(i) * if captains.contains(&i) { 2.0 } else { 1.0 };
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Case {
    pub person: usize,
    pub kind: Trouble,
    pub ruling: Date,
    /// Convicted or pleaded (an acquittal or a dropped charge weighs far less).
    pub guilty: bool,
    /// The day it happened (day 0 for a case opened before the day was kept).
    #[serde(default)]
    pub opened: Date,
}

/// A pro's last off-field case, kept on his career record from the day it
/// happens (live talks lane 8; the market's list forgets a case at the
/// ruling): the market prices a recent one (`market::case_cut`), and a
/// contract talk can check a claim about it.
/// Only what the public knows: the kind, the day and the league's ruling.
/// Whether a court will find him guilty is not here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PastCase {
    pub kind: Trouble,
    /// The day it happened.
    pub day: Date,
    /// The league's ruling: none while the case is open, then the games he
    /// was suspended (0 when it ended without a suspension). A cut on
    /// appeal is written in.
    pub games: Option<u8>,
    /// Another case of his was still recent when this one happened.
    pub again: bool,
}

impl PastCase {
    /// How much of the case the market still holds against him on a day:
    /// all of it for `CASE_FULL_DAYS`, then less in a straight line to
    /// nothing at `CASE_GONE_DAYS`.
    pub fn recent(&self, today: Date) -> f64 {
        let days = self.day.days_to(today).max(0) as f64;
        clamp((CASE_GONE_DAYS - days) / (CASE_GONE_DAYS - CASE_FULL_DAYS), 0.0, 1.0)
    }
}

/// How long a case stays recent (proposed, not confirmed): at full weight
/// for a year, gone after three. Real returns to a full price: Brandon
/// Marshall (cases in 2007 and 2008, top receiver money in April 2010),
/// Calvin Ridley (suspended March 2022, four years and $92M in March 2024),
/// Kareem Hunt (February 2018, an extension at a running back's going rate
/// in September 2020), Michael Vick (2007, six years and $100M in 2011).
pub const CASE_FULL_DAYS: f64 = 365.0;
pub const CASE_GONE_DAYS: f64 = 1095.0;

/// The league's ruling goes onto his record. A ruling on an older case
/// leaves a later one alone: the record is his last case.
fn record_ruling(c: &mut pro::ProCareer, case: &Case, d: Date, games: u32) {
    let games = Some(games.min(255) as u8);
    match c.case.as_mut() {
        Some(pc) if pc.kind == case.kind && pc.day == case.opened => pc.games = games,
        Some(pc) if pc.day > case.opened => {}
        // A case opened before records were kept (an older save): the
        // ruling's day stands for its own.
        _ => {
            let day = if case.opened == Date::default() { d } else { case.opened };
            c.case = Some(PastCase { kind: case.kind, day, games, again: false });
        }
    }
}

/// An appeal changed the games of a league suspension (`folk::courts`):
/// the record follows, when it is the same case.
pub(crate) fn record_appeal(c: &mut pro::ProCareer, what: &str, from: u32, to: u32) {
    if let Some(pc) = c.case.as_mut()
        && pc.kind.words() == what
        && pc.games == Some(from.min(255) as u8)
    {
        pc.games = Some(to.min(255) as u8);
    }
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
pub(crate) fn pro_arrests_day(w: &mut World, d: Date) {
    let year = w.year;
    // The year's rate a day at a time (the six-part day: trouble lands on
    // the night it happens, not in Tuesday's batch).
    let base = if year < 2014 { 0.03 } else { 0.022 } / 365.25;
    let seed = w.options.seed as u64 ^ 0x7E0B_1E00 ^ (d.0 as u64) << 24;
    // A man you got help for after trouble (talk facts lane L4).
    let helped = super::folk::talk_points::helped_ids(w);
    // Every pro's roll is keyed by the day and the person, so the walk runs
    // on all cores and keeps person order (world speed lane).
    use rayon::prelude::*;
    let new: Vec<(usize, Trouble)> = (0..w.people.len())
        .into_par_iter()
        .with_min_len(4096)
        .filter_map(|i| {
            let p = &w.people[i];
            let c = p.pro.as_ref()?;
            if p.stage != Stage::Pro || !c.held() {
                return None;
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
                    * (1.0 - 0.15 * room)
                    * if helped.contains(&p.player.id) { 1.0 - super::folk::talk_points::HELP_CUT } else { 1.0 };
            if key_uniform(seed, i as u64) >= risk {
                return None;
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
            Some((i, kind))
        })
        .collect();
    for (i, kind) in new {
        let guilty = matches!(kind, Trouble::Ped | Trouble::Gambling | Trouble::Dui)
            || key_uniform(seed ^ 0x9117, i as u64) < 0.45;
        let weeks =
            if matches!(kind, Trouble::Ped) { 3 } else { 6 + (key_uniform(seed ^ 0x4EE, i as u64) * 14.0) as i32 };
        w.market.cases.push(Case { person: i, kind, ruling: d.add(weeks * 7), guilty, opened: d });
        // His record (lane 8): this is now his last case.
        if let Some(c) = w.people[i].pro.as_mut() {
            let again = c.case.is_some_and(|pc| pc.recent(d) > 0.0);
            c.case = Some(PastCase { kind, day: d, games: None, again });
        }
        let team = w.people[i].pro.as_ref().and_then(|c| c.team);
        let age = w.people[i].row_age();
        w.people[i].note(format!("Off-field trouble at {age}: {}.", kind.words()));
        w.market.year_mut(year).trouble += 1;
        // The arrest moves his mood: shame or anger (talk facts lane L4).
        super::folk::talk_points::arrest_mood(w, i);
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
            market::post(w, d, team, Some(i), Loudness::Medium, Kind::News, format!("{who}: {}.", kind.words()));
        }
    }
}

/// The league's rulings due today, in office hours.
pub(crate) fn pro_rulings_day(w: &mut World, d: Date) {
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
}

/// A week of games served: suspensions and the exempt list count down each
/// Tuesday.
pub(crate) fn pro_trouble_week(w: &mut World, d: Date) {
    use rayon::prelude::*;
    let in_season = d > w.dates.nfl_kickoff && d <= w.dates.super_bowl;
    // Each person's own counters: any order gives the same world.
    w.people.par_iter_mut().with_min_len(4096).for_each(|p| {
        if let Some(c) = p.pro.as_mut() {
            if in_season && c.suspended > 0 && c.team.is_some() {
                c.suspended -= 1;
            }
            if c.exempt > 0 {
                c.exempt -= 1;
            }
        }
    });
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
    record_ruling(c, case, d, games);
    if games == 0 {
        let age = w.people[i].row_age();
        w.people[i].note(format!("At {age} the case ({}) ended without a suspension.", case.kind.words()));
        return;
    }
    c.suspended += games;
    let team = c.team;
    super::folk::courts::suspension(w, i, games, case.kind.words(), matches!(case.kind, Trouble::Ped | Trouble::Drugs));
    let Some(c) = w.people[i].pro.as_mut() else { return };
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
        Some(i),
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
