//! Section 7, the NFL's world side (`docs/design/scouting.md`, topics 1-3
//! and 9-12; the plan in `docs/design/build9-plan.md`), built 2026-10-19:
//! every club's personnel department as people, the file of looks each
//! club keeps on the draft class, each club's own board read from its
//! files, trades between clubs on draft day and the undrafted rush.
//!
//! A club's file holds only the looks (who, what kind, when). Its read of
//! a prospect is rebuilt from them when a decision needs it: each person's
//! lasting lean on the player plus the noise of each look, by category of
//! attributes, blended by each person's skill. The real rules and rhythms
//! (the scouting year, interview counts, the bonus pool) are researched;
//! the sizes and weights are Claude's, proposed, not confirmed.

use std::collections::{HashMap, HashSet};

use rayon::prelude::*;

use super::super::data::Division;
use super::super::season::{key_normal, key_uniform};
use super::money;
use crate::attributes::{Attr, Category};
use crate::calendar::Date;
use crate::league::depth_slots;
use crate::player::Trait;
use crate::positions::{POSITION_COUNT, Position, overall_for, weight};
use crate::rng::{Rng, clamp};
use crate::world::coach::{GROUP_COUNT, Group};
use crate::world::{Stage, World};

// --- The department ---------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Role {
    /// The director of college scouting: reads the top of the board.
    Director,
    /// National scouts (cross-checkers): a second look at the top names
    /// across regions.
    National,
    /// Area scouts: each owns a region's schools.
    Area,
}

/// A scout's hidden lean (topic 2): it moves his reads by up to about a
/// third of a typical misread, never decides them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Bias {
    /// Loves size for the position.
    Size,
    /// Distrusts players below the FBS.
    SmallSchools,
    /// Lets testing colour the tape.
    Testing,
    /// Trusts the tape and marks down workout warriors.
    Tape,
    /// Distrusts one conference's players.
    Conference,
    /// A soft spot for players from his own region.
    Region,
    /// "Speed kills."
    Speed,
}

const BIASES: [Bias; 7] =
    [Bias::Size, Bias::SmallSchools, Bias::Testing, Bias::Tape, Bias::Conference, Bias::Region, Bias::Speed];

/// A member of a club's personnel department (topic 2), rated 1-20 like
/// every staff member (hidden).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Scout {
    pub name: String,
    pub role: Role,
    /// An area scout's place among the club's area scouts: he covers the
    /// regions `r` with `r % areas == region`.
    pub region: u8,
    /// The region he comes from (his soft spot, if he has that bias).
    pub home: u8,
    pub judge: u8,
    pub potential: u8,
    /// Position knowledge by coaching group.
    pub know: [u8; GROUP_COUNT],
    pub athletic: u8,
    pub character: u8,
    pub network: u8,
    pub work: u8,
    pub conviction: u8,
    pub bias: Bias,
    /// His own key: his lasting lean on each player comes from it.
    pub key: u64,
    pub since: i32,
    pub born: i32,
}

/// What kind of look a file entry is (topic 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum LookKind {
    /// An area scout's fall school call: film, practice and the people
    /// around him.
    School,
    /// Film study from the office (underclassmen, once declared).
    Film,
    CrossCheck,
    Director,
    Gm,
    /// The head coach or a coordinator: spring film and pro day drills.
    Coach,
    AllStar,
    /// Measured at the combine (the same numbers for every club).
    Combine,
    Interview,
    /// Tested at his pro day (hand-timed).
    ProDay,
    /// A top-30 visit: interviews, a physical, no drills.
    Visit,
}

/// One look in a file: by whom (a scout's place in the department, or one
/// of the codes below), what kind and when.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Look {
    pub who: u8,
    pub kind: LookKind,
    pub date: Date,
}

pub const GM: u8 = 250;
pub const HC: u8 = 251;
pub const OC: u8 = 252;
pub const DC: u8 = 253;
/// A measurement everyone shares (the combine, a pro day).
pub const EVENT: u8 = 255;

const REGIONS: u8 = 8;

/// The eight scouting regions a club splits the country into.
pub(crate) fn region_of(state: usize) -> u8 {
    let code = crate::world::geo::STATE_CENTERS.get(state).map_or("", |s| s.0);
    match code {
        "ME" | "NH" | "VT" | "MA" | "RI" | "CT" | "NY" | "NJ" | "PA" => 0,
        "DE" | "MD" | "DC" | "VA" | "WV" | "NC" | "SC" | "KY" | "TN" => 1,
        "GA" | "FL" | "AL" | "MS" => 2,
        "OH" | "MI" | "IN" | "IL" | "WI" | "MN" | "IA" => 3,
        "ND" | "SD" | "NE" | "KS" | "MO" | "OK" => 4,
        "TX" | "LA" | "AR" => 5,
        "MT" | "WY" | "CO" | "UT" | "NM" | "AZ" | "ID" | "NV" => 6,
        _ => 7,
    }
}

/// A department's size by era (real: about a dozen people in the 1990s,
/// about two dozen in the 2020s; area scouts 6-9 and two or three national
/// scouts), set by the owner's budget: (area scouts, national scouts).
fn sizes(year: i32, owner: f64) -> (usize, usize) {
    let areas: i32 = match year {
        ..=1994 => 6,
        1995..=2004 => 7,
        2005..=2014 => 8,
        _ => 9,
    };
    let nationals = match year {
        ..=1994 => 1,
        1995..=2009 => 2,
        _ => 3,
    };
    let adj = if owner >= 15.0 {
        1
    } else if owner <= 5.0 {
        -1
    } else {
        0
    };
    ((areas + adj).clamp(5, 10) as usize, nationals)
}

fn hire(rng: &mut Rng, year: i32, role: Role, budget: f64) -> Scout {
    let mean = match role {
        Role::Director => 12.0,
        Role::National => 11.0,
        Role::Area => 9.0,
    } + 3.0 * (budget - 0.6);
    let mut r = |m: f64| clamp(rng.normal(m, 2.8), 1.0, 20.0).round() as u8;
    let judge = r(mean);
    let potential = r(mean);
    let athletic = r(mean);
    let character = r(mean);
    let network = r(mean);
    let work = r(mean);
    let conviction = r(10.0);
    let mut know = [0u8; GROUP_COUNT];
    for k in know.iter_mut() {
        *k = r(mean);
    }
    let born = year
        - match role {
            Role::Director => rng.int(38, 60),
            Role::National => rng.int(32, 58),
            Role::Area => rng.int(24, 55),
        };
    let eth = if rng.chance(0.25) {
        super::super::ethnicity::Ethnicity::Black
    } else {
        super::super::ethnicity::Ethnicity::White
    };
    let name = super::super::names::person_name(eth, born, rng);
    let bias = BIASES[rng.int(0, BIASES.len() as i32 - 1) as usize];
    let home = rng.int(0, REGIONS as i32 - 1) as u8;
    let key = ((rng.next_u32() as u64) << 32) | rng.next_u32() as u64;
    Scout {
        name,
        role,
        region: 0,
        home,
        judge,
        potential,
        know,
        athletic,
        character,
        network,
        work,
        conviction,
        bias,
        key,
        since: year,
        born,
    }
}

/// Staffs each club's department. With `turnover` (after the draft, when
/// scouts' contracts end), about one in ten leaves, a good scout at a
/// thrifty club more often (hired away), the old retire, and the ladder
/// moves up: a national scout to director, an area scout to national.
/// Without it, only empty departments are filled (a new club, the first
/// year). Called only while the files are empty, since a file names
/// scouts by their place. The clubs staffed are those that pick in
/// `draft_year`: an expansion club hires its department the spring
/// before its first draft, as the Panthers and the Jaguars did in 1994
/// (real), and a club that has gone dark keeps none.
pub(crate) fn staff(w: &mut World, turnover: bool, draft_year: i32) {
    use super::super::franchise::{BROWNS, RAVENS, active};
    let year = w.year;
    let mut rng = Rng::new(w.options.seed ^ 0x5C07_0000 ^ (year as u32).wrapping_mul(104_729));
    // 1996 (real): the Browns' front office went to Baltimore with the
    // team, so their scouts work the 1996 class for the Ravens.
    if draft_year == 1996 && w.clubs.len() > BROWNS.max(RAVENS) && w.clubs[RAVENS].scouts.is_empty() {
        w.clubs[RAVENS].scouts = std::mem::take(&mut w.clubs[BROWNS].scouts);
    }
    for t in 0..w.clubs.len() {
        if !active(t, draft_year) {
            continue;
        }
        if !turnover && !w.clubs[t].scouts.is_empty() {
            continue;
        }
        let owner = w.nfl[t].owner;
        let budget = clamp(0.55 + 0.03 * (owner - 10.0), 0.3, 1.0);
        let (areas, nationals) = sizes(year, owner);
        let mut kept: Vec<Scout> = Vec::new();
        for s in std::mem::take(&mut w.clubs[t].scouts) {
            let age = year - s.born;
            let leave =
                0.1 + if s.judge >= 15 { 0.12 * (1.0 - budget) } else { 0.0 } + if age >= 64 { 0.5 } else { 0.0 };
            if !rng.chance(leave) {
                kept.push(s);
            }
        }
        let mut director = kept.iter().position(|s| s.role == Role::Director).map(|k| kept.remove(k));
        let mut nats: Vec<Scout> = kept.iter().filter(|s| s.role == Role::National).cloned().collect();
        let mut area: Vec<Scout> = kept.into_iter().filter(|s| s.role == Role::Area).collect();
        let best = |v: &Vec<Scout>| {
            (0..v.len()).max_by_key(|&k| v[k].judge as u32 * 2 + v[k].potential as u32 + v[k].work as u32)
        };
        if director.is_none() {
            director = Some(match best(&nats) {
                Some(k) if rng.chance(0.6) => {
                    let mut s = nats.remove(k);
                    s.role = Role::Director;
                    s
                }
                _ => hire(&mut rng, year, Role::Director, budget),
            });
        }
        nats.truncate(nationals);
        while nats.len() < nationals {
            let s = match best(&area) {
                Some(k) if rng.chance(0.6) => {
                    let mut s = area.remove(k);
                    s.role = Role::National;
                    s
                }
                _ => hire(&mut rng, year, Role::National, budget),
            };
            nats.push(s);
        }
        area.truncate(areas);
        while area.len() < areas {
            area.push(hire(&mut rng, year, Role::Area, budget));
        }
        for (k, s) in area.iter_mut().enumerate() {
            s.region = k as u8;
        }
        let mut all = vec![director.unwrap()];
        all.extend(nats);
        all.extend(area);
        w.clubs[t].scouts = all;
    }
}

/// The area scout who covers a region at a club.
fn area_scout(scouts: &[Scout], region: u8) -> Option<u8> {
    let areas = scouts.iter().filter(|s| s.role == Role::Area).count().max(1) as u8;
    scouts.iter().position(|s| s.role == Role::Area && s.region == region % areas).map(|k| k as u8)
}

fn director(scouts: &[Scout]) -> u8 {
    scouts.iter().position(|s| s.role == Role::Director).unwrap_or(0) as u8
}

fn nationals(scouts: &[Scout]) -> Vec<u8> {
    scouts.iter().enumerate().filter(|(_, s)| s.role == Role::National).map(|(k, _)| k as u8).collect()
}

// --- Reading a file -----------------------------------------------------------------------

const CATS: usize = 8;
const PHYSICAL: usize = 0;
const MENTAL: usize = 7;
/// One film look's misread of a category, in rating points, for an
/// average eye at FBS level (proposed, not confirmed).
const FILM_SD: f64 = 7.0;
/// What one misread is made of: his lasting lean and the noise of the
/// look (topic 1, proposed: about 60% lean, 40% noise).
const LEAN: f64 = 0.6;
const NOISE: f64 = 0.4;
const MIX_SD: f64 = 0.7211; // sqrt(0.6^2 + 0.4^2)

fn cat(a: Attr) -> usize {
    match a.category() {
        Category::Physical => 0,
        Category::Passing => 1,
        Category::Running => 2,
        Category::Receiving => 3,
        Category::LinePlay => 4,
        Category::Defense => 5,
        Category::KickingAndSnapping => 6,
        Category::Mental => 7,
    }
}

/// Each position's weight on each category (the position overall is a
/// weighted mean of attributes).
static WEIGHTS: std::sync::LazyLock<[[f64; CATS]; POSITION_COUNT]> = std::sync::LazyLock::new(|| {
    let mut w = [[0.0; CATS]; POSITION_COUNT];
    for pos in Position::ALL {
        for a in Attr::ALL {
            w[pos.index()][cat(a)] += weight(pos, a);
        }
    }
    w
});

impl LookKind {
    /// The base misread of a category on this kind of look, or None if it
    /// can't see it.
    fn sees(self, c: usize) -> Option<f64> {
        use LookKind::*;
        match self {
            School | AllStar => Some(if c == PHYSICAL { 6.0 } else { FILM_SD }),
            Film | CrossCheck | Director | Gm | Coach => Some(if c == PHYSICAL { 8.0 } else { FILM_SD }),
            Interview | Visit => (c == MENTAL).then_some(8.0),
            Combine | ProDay => None,
        }
    }

    /// A measurement's error on the athletic category.
    fn measured(self) -> Option<f64> {
        match self {
            LookKind::Combine => Some(2.0),
            LookKind::ProDay => Some(3.0),
            _ => None,
        }
    }

    /// A character read's base error (in the risk's standard deviations).
    fn character(self) -> Option<f64> {
        match self {
            LookKind::School => Some(1.3),
            LookKind::Interview => Some(0.9),
            LookKind::Visit => Some(0.8),
            LookKind::Director | LookKind::Gm => Some(1.2),
            _ => None,
        }
    }
}

/// How a person looks at players: his ratings for this player's group,
/// his say in the room, and his lean.
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct Eye {
    key: u64,
    judge: f64,
    potential: f64,
    athletic: f64,
    character: f64,
    say: f64,
    bias: Option<Bias>,
    home: u8,
}

fn eye(w: &World, t: usize, who: u8, group: usize) -> Eye {
    let club = &w.clubs[t];
    match who {
        GM => {
            let g = &club.office.gm;
            Eye {
                key: (t as u64) << 40 ^ (g.since as u64) << 8 ^ 0x6D,
                judge: g.skill,
                potential: g.skill,
                athletic: 10.0,
                character: 10.0,
                // The final say (topic 8): the GM's word carries most,
                // unless the coach or the owner holds it.
                say: if club.office.power == super::office::Power::Gm { 1.4 } else { 1.1 },
                bias: None,
                home: REGIONS,
            }
        }
        HC | OC | DC => {
            let tm = &w.nfl[t];
            let c = match who {
                HC => tm.head_coach,
                OC => tm.oc.or(tm.head_coach),
                _ => tm.dc.or(tm.head_coach),
            };
            let (key, teach, rel) = c.map_or(((t as u64) << 20 ^ who as u64, 10.0, 10.0), |c| {
                let a = w.coaches[c].ability();
                (w.coaches[c].id as u64 ^ 0xC0AC_0000, a.teaching[group], a.relationships)
            });
            Eye {
                key,
                judge: teach,
                potential: teach,
                athletic: 10.0,
                character: rel,
                say: if who == HC && club.office.power == super::office::Power::Coach { 1.5 } else { 1.0 },
                bias: None,
                home: REGIONS,
            }
        }
        k => {
            let s = &club.scouts[k as usize];
            let know = s.know[group] as f64;
            Eye {
                key: s.key,
                judge: 0.6 * s.judge as f64 + 0.4 * know,
                potential: 0.7 * s.potential as f64 + 0.3 * know,
                athletic: s.athletic as f64,
                character: s.character as f64,
                say: (0.8 + 0.02 * s.conviction as f64) * if s.role == Role::Director { 1.2 } else { 1.0 },
                bias: Some(s.bias),
                home: s.home,
            }
        }
    }
}

/// How sharp a rating makes a read (10 is 1.0, 20 is 0.6, 1 is 1.36).
fn sharp(r: f64) -> f64 {
    1.4 - 0.04 * r
}

/// What the scouts' biases react to, standardised against the class at
/// each position.
pub(crate) struct Ctx {
    size: Vec<(f64, f64, f64, f64)>,
    speed: Vec<(f64, f64)>,
}

impl Ctx {
    fn new(w: &World, people: &[usize]) -> Ctx {
        let mut h = vec![Vec::new(); POSITION_COUNT];
        for &i in people {
            let p = &w.people[i].player;
            h[p.position.index()].push((p.body.height, p.body.weight, p.attributes.get(Attr::Speed)));
        }
        let ms = |v: &[f64]| {
            let n = v.len().max(1) as f64;
            let m = v.iter().sum::<f64>() / n;
            let sd = (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n).sqrt().max(0.5);
            (m, sd)
        };
        let mut size = Vec::new();
        let mut speed = Vec::new();
        for v in &h {
            let (hm, hs) = ms(&v.iter().map(|x| x.0).collect::<Vec<_>>());
            let (wm, ws) = ms(&v.iter().map(|x| x.1).collect::<Vec<_>>());
            size.push((hm, hs, wm, ws));
            speed.push(ms(&v.iter().map(|x| x.2).collect::<Vec<_>>()));
        }
        Ctx { size, speed }
    }
}

/// A club's read of one prospect: its grade on the December scale (before
/// the position alignment), how sure it is (the grade's variance) and its
/// character read (in standard deviations of risk; above zero is worse).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Read {
    pub grade: f64,
    pub var: f64,
    pub risk: f64,
}

/// A prospect's real risk of trouble from his personality, in standard
/// deviations: low professionalism and a short temper (section 6).
fn true_risk(p: &crate::world::Person) -> f64 {
    let t = &p.player.personality.traits;
    clamp((0.6 * -t.z(Trait::Professionalism) + 0.5 * t.z(Trait::Temper)) / 0.28, -3.0, 4.0)
}

/// The red flags everyone knows: a dismissal or a suspension in college.
fn flags(p: &crate::world::Person) -> f64 {
    use crate::career::PIPELINE;
    p.player
        .history
        .iter()
        .map(|s| {
            if s.events.has(crate::player::DevelopmentEvent::Dismissed) {
                PIPELINE.red_flag_discount.dismissed
            } else if s.events.has(crate::player::DevelopmentEvent::Suspended) {
                PIPELINE.red_flag_discount.suspended
            } else {
                0.0
            }
        })
        .sum()
}

/// Grade points a full measured link adds to a club's read (D3, Claude's
/// pick: real clubs draft the freak a round or so early).
const DRAFT_GIFT: f64 = 2.0;

fn competition_discount(d: Division) -> f64 {
    match d {
        Division::FBS => 0.0,
        Division::FCS => 1.0,
        Division::D2 | Division::JUCO => 5.0,
        Division::NAIA => 6.0,
        Division::D3 => 7.0,
    }
}

/// Rebuilds a club's read of a prospect from its file (topic 1). Each
/// person who looked keeps his lean on the player, and more looks by him
/// shrink only his noise; different people's leans average out, weighted
/// by their skill and their say in the room. Combine and pro day numbers
/// are shared by every club.
pub(crate) fn read(w: &World, t: usize, i: usize, looks: &[Look], ctx: &Ctx) -> Read {
    let p = &w.people[i];
    let pos = p.cal.pro_position.unwrap_or(p.player.position);
    let id = p.player.id as u64;
    let group = Group::of(pos) as usize;
    let division = p.college.map_or(Division::FBS, |c| w.programs[c].division);
    let vis = match division {
        Division::FBS => 1.0,
        Division::FCS => 1.2,
        _ => 1.5,
    };
    let prog = p.college.map(|c| &w.programs[c]);
    let region = prog.map_or(REGIONS, |g| region_of(g.state));
    let home = region_of(p.home_state);
    let conf_key =
        prog.map_or(0u64, |g| g.conference.bytes().fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64)));
    let k = pos.index();
    let (hm, hs, wm, ws) = ctx.size[k];
    let z_size = clamp(0.5 * (p.player.body.height - hm) / hs + 0.5 * (p.player.body.weight - wm) / ws, -1.5, 1.5);
    let (sm, ss) = ctx.speed[k];
    let z_speed = clamp((p.player.attributes.get(Attr::Speed) - sm) / ss, -1.5, 1.5);
    let z_ath = z_speed;

    // The people who looked, with their looks counted by category.
    let mut whos: Vec<(u8, [u8; CATS], f64, u8, f64, u8)> = Vec::new(); // who, n per cat, best sd per cat (shared), n pot, best char sd, n char
    let mut best_sd: Vec<[f64; CATS]> = Vec::new();
    let mut meas: Option<f64> = None;
    for l in looks {
        if l.who == EVENT {
            if let Some(sd) = l.kind.measured() {
                meas = Some(meas.map_or(sd, |m: f64| m.min(sd)));
            }
            continue;
        }
        let at = match whos.iter().position(|x| x.0 == l.who) {
            Some(a) => a,
            None => {
                whos.push((l.who, [0; CATS], 0.0, 0, f64::MAX, 0));
                best_sd.push([f64::MAX; CATS]);
                whos.len() - 1
            }
        };
        for (c, sd) in best_sd[at].iter_mut().enumerate() {
            if let Some(s) = l.kind.sees(c) {
                whos[at].1[c] += 1;
                *sd = sd.min(s);
            }
        }
        if l.kind.measured().is_none() {
            whos[at].3 += 1;
            let f = if matches!(l.kind, LookKind::Interview | LookKind::Visit) { 1.6 } else { 1.0 };
            whos[at].2 = if whos[at].2 == 0.0 { f } else { whos[at].2.min(f) };
        }
        if let Some(s) = l.kind.character() {
            whos[at].4 = whos[at].4.min(s);
            whos[at].5 += 1;
        }
    }

    // Each category: the blend of everyone who saw it.
    let mut err = [0.0f64; CATS];
    let mut prec = [0.0f64; CATS];
    let mut pot_err = 0.0;
    let mut pot_prec = 0.0;
    let mut risk_err = 0.0;
    let mut risk_prec = 0.0;
    let plast = crate::development::potential_plasticity(p.player.age);
    for (a, x) in whos.iter().enumerate() {
        let e = eye(w, t, x.0, group);
        for c in 0..CATS {
            let n = x.1[c];
            if n == 0 {
                continue;
            }
            let f = if c == PHYSICAL { sharp(e.athletic) } else { sharp(e.judge) };
            let sd = f * best_sd[a][c] * vis;
            let lean = key_normal(e.key ^ 0x1EA4 ^ (c as u64) << 56, id);
            let noise = key_normal(e.key ^ 0x0153 ^ (c as u64) << 56 ^ (w.year as u64) << 40, id);
            let mut v = sd * (LEAN * lean + NOISE * noise / (n as f64).sqrt()) / MIX_SD;
            // His bias (topic 2): up to about a third of a misread.
            let tilt = 0.33 * sd;
            v += match e.bias {
                Some(Bias::Size) => tilt * z_size,
                Some(Bias::SmallSchools) if division != Division::FBS => -tilt,
                Some(Bias::Testing) if c != PHYSICAL => tilt * z_ath,
                Some(Bias::Tape) if c == PHYSICAL => -tilt * z_ath,
                Some(Bias::Conference) if conf_key % 12 == e.key % 12 => -tilt,
                Some(Bias::Region) if region == e.home || home == e.home => tilt,
                Some(Bias::Speed) => tilt * z_speed,
                _ => 0.0,
            };
            let var = sd * sd * (LEAN * LEAN + NOISE * NOISE / n as f64) / (MIX_SD * MIX_SD);
            let wgt = e.say / var;
            err[c] += wgt * v;
            prec[c] += wgt;
        }
        if x.3 > 0 {
            let sd = (3.0 + 7.0 * plast) * sharp(e.potential) * vis * x.2;
            let lean = key_normal(e.key ^ 0x9071, id);
            let noise = key_normal(e.key ^ 0x9072 ^ (w.year as u64) << 40, id);
            let n = x.3 as f64;
            let v = sd * (LEAN * lean + NOISE * noise / n.sqrt()) / MIX_SD;
            let var = sd * sd * (LEAN * LEAN + NOISE * NOISE / n) / (MIX_SD * MIX_SD);
            pot_err += e.say * v / var;
            pot_prec += e.say / var;
        }
        if x.5 > 0 {
            let sd = x.4 * sharp(e.character);
            let n = x.5 as f64;
            let lean = key_normal(e.key ^ 0xC4A2, id);
            let noise = key_normal(e.key ^ 0xC4A3 ^ (w.year as u64) << 40, id);
            let v = sd * (LEAN * lean + NOISE * noise / n.sqrt()) / MIX_SD;
            let var = sd * sd * (LEAN * LEAN + NOISE * NOISE / n) / (MIX_SD * MIX_SD);
            risk_err += v / var;
            risk_prec += 1.0 / var;
        }
    }
    // Shared measurements of the athletic side.
    if let Some(sd) = meas {
        let v = sd * key_normal(0x3EA5 ^ (w.year as u64) << 40, id);
        err[PHYSICAL] += v / (sd * sd);
        prec[PHYSICAL] += 1.0 / (sd * sd);
    }
    // A category nobody saw: the services' word only (topic 9: a club
    // leans on the consensus for what it barely saw).
    let prior = 9.0 * vis;
    let wt = &WEIGHTS[k];
    let mut now_err = 0.0;
    let mut now_var = 0.0;
    for c in 0..CATS {
        if prec[c] == 0.0 {
            let v = prior * key_normal((t as u64) << 48 ^ 0x9A10 ^ (c as u64) << 40, id);
            err[c] = v / (prior * prior);
            prec[c] = 1.0 / (prior * prior);
        }
        now_err += wt[c] * err[c] / prec[c];
        now_var += wt[c] * wt[c] / prec[c];
    }
    if pot_prec == 0.0 {
        let sd = (3.0 + 7.0 * plast) * 1.6 * vis;
        pot_err = sd * key_normal((t as u64) << 48 ^ 0x9A11, id) / (sd * sd);
        pot_prec = 1.0 / (sd * sd);
    }
    let pot_err = pot_err / pot_prec;
    let now = overall_for(pos, &p.player.attributes) + now_err;
    let pot = now.max(overall_for(pos, &p.player.ceilings) + pot_err);
    let youth = clamp((23.0 - p.player.age as f64) / 3.0, 0.0, 1.0);
    let q = 0.25 + 0.2 * youth;
    // Traits that can't be coached (D3): clubs pay for measured size,
    // length, speed and burst (the links), about two grade points for one
    // full link once he has tested, less on a film guess. A workout warrior
    // without the skills still grades on his skills first.
    let gift = crate::links::gift(&crate::links::links_at(&p.player, pos));
    let gift_pts = if meas.is_some() { DRAFT_GIFT } else { 0.5 * DRAFT_GIFT } * gift;
    let grade = now + q * (pot - now) + gift_pts - competition_discount(division) - flags(p);
    let var = now_var * (1.0 - q) * (1.0 - q) + q * q / pot_prec;
    // The character read: the services' talk only, if nobody asked.
    let risk = if risk_prec > 0.0 {
        true_risk(p) + risk_err / risk_prec
    } else {
        true_risk(p) * 0.3 + 1.2 * key_normal((t as u64) << 48 ^ 0xC4A4, id)
    };
    Read { grade, var, risk }
}

/// Every club's read of the prospects in its files, on all cores (each
/// read is keyed, so the result is the same on any machine).
fn reads(w: &World, people: &[usize]) -> Vec<HashMap<usize, Read>> {
    let ctx = Ctx::new(w, people);
    (0..w.clubs.len())
        .into_par_iter()
        .map(|t| {
            let files = &w.clubs[t].files;
            people
                .iter()
                .filter_map(|&i| files.get(&w.people[i].player.id).map(|f| (i, read(w, t, i, f, &ctx))))
                .collect()
        })
        .collect()
}

fn file(w: &mut World, t: usize, i: usize, who: u8, kind: LookKind, date: Date) {
    // A scout's look needs a scout in that place (a club with no
    // department has no one to send).
    if who < GM && who as usize >= w.clubs[t].scouts.len() {
        debug_assert!(false, "club {t} has no scout {who}");
        return;
    }
    let id = w.people[i].player.id;
    w.clubs[t].files.entry(id).or_default().push(Look { who, kind, date });
    let year = w.year;
    w.market.year_mut(year).scout_looks += 1;
}

/// The draft the clubs are working toward: this year's until its first
/// night, then next year's (the fall, the all-star games in January).
fn coming_draft(w: &World) -> i32 {
    if w.today <= w.dates.draft[0].date { w.year } else { w.year + 1 }
}

/// The clubs that scout now: those that pick in the coming draft (a new
/// club from the spring before its first draft; trial 9 crashed in 1995
/// when the Panthers and the Jaguars had a draft but no scouts).
fn clubs(w: &World) -> Vec<usize> {
    let draft_year = coming_draft(w);
    (0..w.clubs.len()).filter(|&t| super::super::franchise::active(t, draft_year)).collect()
}

// --- The scouting year (topic 3) ------------------------------------------------------------

/// The fall (called with the December reads, before they are redone):
/// each club's area scouts make their school calls on the spring list
/// (last December's read of the rising seniors, the services' list), two
/// or three schools a week at their work rate, big programs twice, and
/// note the unlisted seniors their eye and network catch. Then the
/// cross-checkers and the director see the top of the club's own list.
pub(crate) fn fall(w: &mut World) {
    if w.clubs.is_empty() || w.nfl.is_empty() {
        return;
    }
    let year = w.year;
    let draft_year = year + 1;
    staff(w, false, draft_year);
    for c in w.clubs.iter_mut() {
        c.files.clear();
    }
    // The seniors at each program, and the spring list.
    let mut seniors: Vec<Vec<usize>> = vec![Vec::new(); w.programs.len()];
    let mut listed: Vec<(usize, f64)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::College || draft_year - p.class_of < 4 {
            continue;
        }
        let Some(c) = p.college else { continue };
        if w.programs[c].division == Division::JUCO {
            continue;
        }
        seniors[c].push(i);
        if let Some(r) = p.cal.draft_read {
            listed.push((i, r.score));
        }
    }
    let slots = (crate::career::PIPELINE.draft_picks * w.options.scale).round().max(1.0) as usize;
    // The world's first fall has no reads from last year: the services
    // list by what a senior is now.
    if listed.is_empty() {
        listed = seniors.iter().flatten().map(|&i| (i, w.people[i].player.overall())).collect();
    }
    listed.sort_by(|a, b| b.1.total_cmp(&a.1));
    listed.truncate(slots * 5);
    let on_list: HashSet<usize> = listed.iter().map(|x| x.0).collect();
    let mut overalls: Vec<f64> = listed.iter().map(|x| w.people[x.0].player.overall()).collect();
    overalls.sort_by(|a, b| a.total_cmp(b));
    let bar = overalls.get(overalls.len() / 2).copied().unwrap_or(60.0);
    // Each program's draw for a scout: its listed seniors, the best counting most.
    let top = listed.first().map_or(0.0, |x| x.1);
    let mut pull = vec![0.0f64; w.programs.len()];
    let mut count = vec![0usize; w.programs.len()];
    for &(i, s) in &listed {
        if let Some(c) = w.people[i].college {
            pull[c] += (-(top - s) / 8.0).exp() + 0.2;
            count[c] += 1;
        }
    }
    let by_region: Vec<Vec<usize>> = (0..REGIONS)
        .map(|r| {
            let mut v: Vec<usize> = (0..w.programs.len())
                .filter(|&c| !seniors[c].is_empty() && region_of(w.programs[c].state) == r)
                .collect();
            v.sort_by(|&a, &b| pull[b].total_cmp(&pull[a]).then(a.cmp(&b)));
            v
        })
        .collect();
    let fall_start = Date::ymd(year, 9, 1);
    for t in clubs(w) {
        let scouts = w.clubs[t].scouts.clone();
        let areas = scouts.iter().filter(|s| s.role == Role::Area).count().max(1) as u8;
        for (k, s) in scouts.iter().enumerate() {
            if s.role != Role::Area {
                continue;
            }
            // About 35-45 school calls a fall (topic 3), fewer with the
            // long flights of the West and the mountains.
            let regions: Vec<u8> = (0..REGIONS).filter(|r| r % areas == s.region).collect();
            let travel = if regions.iter().any(|&r| r >= 6) { 0.85 } else { 1.0 };
            let calls = (38.0 * (0.75 + 0.025 * s.work as f64) * travel).round() as usize;
            let mut schools: Vec<usize> = Vec::new();
            for &r in &regions {
                schools.extend(by_region[r as usize].iter().copied());
            }
            schools.sort_by(|&a, &b| pull[b].total_cmp(&pull[a]).then(a.cmp(&b)));
            // A scout who distrusts small schools gets to fewer of them.
            if s.bias == Bias::SmallSchools {
                schools.sort_by_key(|&c| w.programs[c].division != Division::FBS);
            }
            let mut visits: Vec<usize> = Vec::new();
            for &c in &schools {
                if visits.len() >= calls {
                    break;
                }
                visits.push(c);
                if count[c] >= 3 && visits.len() < calls {
                    visits.push(c);
                }
            }
            let n = visits.len().max(1);
            for (j, &c) in visits.iter().enumerate() {
                let date = fall_start.add((j * 90 / n) as i32);
                for &i in &seniors[c] {
                    let id = w.people[i].player.id as u64;
                    let seen = if on_list.contains(&i) {
                        true
                    } else {
                        // An unlisted senior: his eye and his sources
                        // (trainers, coaches) put him on the radar.
                        let eye =
                            w.people[i].player.overall() + 5.0 * sharp(s.judge as f64) * key_normal(s.key ^ 0xD15C, id);
                        eye >= bar && key_uniform(s.key ^ 0xD15D, id) < 0.4 + 0.03 * (s.network as f64 - 10.0)
                    };
                    if seen {
                        file(w, t, i, k as u8, LookKind::School, date);
                    }
                }
            }
        }
    }
    cross_checks(w, Date::ymd(year, 12, 1), 150, 60, 80, false);
}

/// The second looks at the top of each club's own list: national scouts
/// on the top `nat` (a second on the top `nat2`), the director on the top
/// `dir`; in spring the GM on the top 40 too.
fn cross_checks(w: &mut World, date: Date, nat: usize, nat2: usize, dir: usize, gm: bool) {
    let people: Vec<usize> = filed(w);
    let r = reads(w, &people);
    for t in clubs(w) {
        let mut list: Vec<(usize, f64)> = r[t].iter().map(|(&i, x)| (i, x.grade)).collect();
        list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let nats = nationals(&w.clubs[t].scouts);
        let d = director(&w.clubs[t].scouts);
        for (rank, &(i, _)) in list.iter().enumerate() {
            if !nats.is_empty() && rank < nat {
                file(w, t, i, nats[rank % nats.len()], LookKind::CrossCheck, date);
                if rank < nat2 && nats.len() >= 2 {
                    file(w, t, i, nats[(rank + 1) % nats.len()], LookKind::CrossCheck, date.add(20));
                }
            }
            if rank < dir {
                file(w, t, i, d, LookKind::Director, date.add(10));
            }
            if gm && rank < 40 {
                file(w, t, i, GM, LookKind::Gm, date.add(5));
            }
        }
    }
}

/// The people any club has a file on (place in the world), sorted.
fn filed(w: &World) -> Vec<usize> {
    let ids: HashSet<u32> = w.clubs.iter().flat_map(|c| c.files.keys().copied()).collect();
    (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            matches!(p.stage, Stage::College | Stage::Declared) && ids.contains(&p.player.id)
        })
        .collect()
}

/// An all-star game (its practice week): every club's area scout for the
/// player's region watches him all week, and at the Senior Bowl a
/// national scout too.
pub(crate) fn all_star(w: &mut World, game: &str, players: &[usize]) {
    let date = w.today;
    for t in clubs(w) {
        let scouts = &w.clubs[t].scouts;
        let nats = nationals(scouts);
        let mut looks = Vec::new();
        for (k, &i) in players.iter().enumerate() {
            let region = w.people[i].college.map_or(0, |c| region_of(w.programs[c].state));
            if let Some(a) = area_scout(scouts, region) {
                looks.push((i, a));
            }
            if game == "Senior Bowl" && !nats.is_empty() {
                looks.push((i, nats[k % nats.len()]));
            }
        }
        for (i, who) in looks {
            file(w, t, i, who, LookKind::AllStar, date);
        }
    }
}

/// The underclassmen who declared (after the January deadline): no club
/// had them on its spring list, so the area scout of each one's school
/// writes him up from film, and the top names get a cross-check.
pub(crate) fn underclassmen(w: &mut World) {
    if w.clubs.is_empty() {
        return;
    }
    let draft_year = w.year;
    let date = w.today.add(7);
    let under: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.stage == Stage::Declared && draft_year - p.class_of < 4 && p.college.is_some()
        })
        .collect();
    for t in clubs(w) {
        let nats = nationals(&w.clubs[t].scouts);
        for (k, &i) in under.iter().enumerate() {
            let region = region_of(w.programs[w.people[i].college.unwrap()].state);
            if let Some(a) = area_scout(&w.clubs[t].scouts, region) {
                file(w, t, i, a, LookKind::Film, date);
            }
            if w.people[i].cal.draft_read.is_some_and(|r| r.round <= 3) && !nats.is_empty() {
                file(w, t, i, nats[k % nats.len()], LookKind::CrossCheck, date.add(14));
            }
        }
    }
}

/// The combine (topic 10): its measurements go into every club's file
/// (the same numbers for all; none in a year its workouts were
/// cancelled), and each club's formal interviews, 60 of 15 minutes until
/// 2019 and 45 of 18 from 2020 (real), go to the top of its board.
pub(crate) fn combine(w: &mut World, invited: &[usize]) {
    if w.clubs.is_empty() {
        return;
    }
    let date = w.today;
    let workouts = !crate::calendar::anchors::cancelled("combine workouts", w.dates.season);
    let interviews = if w.year >= 2020 { 45 } else { 60 };
    let r = reads(w, invited);
    for t in clubs(w) {
        if workouts {
            for &i in invited {
                // A prospect whose camp keeps him out of the workouts tests at his pro day.
                if !skips(w, i) {
                    file(w, t, i, EVENT, LookKind::Combine, date);
                }
            }
        }
        let mut list: Vec<(usize, f64)> = invited
            .iter()
            .map(|&i| (i, r[t].get(&i).map_or(w.people[i].cal.draft_read.map_or(0.0, |x| x.grade), |x| x.grade)))
            .collect();
        list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        // Your war room's own interview list goes first.
        let mine = super::super::scouting::predraft::seat_list(w, t, super::super::scouting::predraft::LIST_INTERVIEW);
        if !mine.is_empty() {
            let pick: Vec<usize> =
                mine.iter().filter_map(|id| invited.iter().copied().find(|&i| w.people[i].player.id == *id)).collect();
            list.retain(|x| !pick.contains(&x.0));
            for (k, i) in pick.into_iter().enumerate() {
                list.insert(k, (i, 0.0));
            }
        }
        let d = director(&w.clubs[t].scouts);
        for &(i, _) in list.iter().take(interviews) {
            file(w, t, i, d, LookKind::Interview, date);
        }
    }
    super::super::scouting::predraft::combine_moments(w, invited);
}

fn skips(w: &World, i: usize) -> bool {
    w.scouting.predraft.has(w.people[i].player.id, super::super::scouting::predraft::SKIP_COMBINE)
}

/// A program's pro day: the prospects the combine didn't test (or every
/// one, in a year without combine workouts) are measured, and each club
/// with one of them high on its board sends a coach to run the drills.
pub(crate) fn pro_day(w: &mut World, program: usize, date: Date) {
    if w.clubs.is_empty() {
        return;
    }
    let workouts = !crate::calendar::anchors::cancelled("combine workouts", w.dates.season);
    let here: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.stage == Stage::Declared && p.college.is_some_and(|c| w.programs[c].id == program)
        })
        .collect();
    if here.is_empty() {
        return;
    }
    let r = reads(w, &here);
    for t in clubs(w) {
        for &i in &here {
            let tested = workouts && w.people[i].cal.combine_invite && !skips(w, i);
            if !tested {
                file(w, t, i, EVENT, LookKind::ProDay, date);
            }
            // A club sends a coach to a prospect it grades as draftable.
            if r[t].get(&i).is_some_and(|x| x.grade >= 66.0) {
                let pos = w.people[i].player.position;
                let who = if pos.is_specialist() {
                    HC
                } else if Group::of(pos) as usize <= Group::OL as usize {
                    OC
                } else {
                    DC
                };
                file(w, t, i, who, LookKind::Coach, date);
            }
        }
    }
}

/// The spring (the top-30 deadline): the coaches' film study of the top of
/// the board, the GM's own looks at the very top, the final cross-checks,
/// and each club's 30 visits for the players it might take with its first
/// picks (real: 30 out-of-town visits).
pub(crate) fn spring(w: &mut World) {
    if w.clubs.is_empty() {
        return;
    }
    let date = w.today;
    cross_checks(w, date.add(-30), 100, 40, 60, true);
    let people = filed(w);
    let r = reads(w, &people);
    let order = super::super::nfl_year::draft_order(w);
    for t in clubs(w) {
        let mut list: Vec<(usize, f64)> = r[t].iter().map(|(&i, x)| (i, x.grade)).collect();
        list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        for &(i, _) in list.iter().take(100) {
            let pos = w.people[i].player.position;
            let who = if Group::of(pos) as usize <= Group::OL as usize { OC } else { DC };
            file(w, t, i, who, LookKind::Coach, date.add(-20));
        }
        for &(i, _) in list.iter().take(30) {
            file(w, t, i, HC, LookKind::Coach, date.add(-15));
        }
        // Visits for players near its first pick: from about ten places
        // above it to two rounds below.
        let first = order.as_ref().and_then(|o| o.iter().position(|&x| x == t)).unwrap_or(16);
        let from = first.saturating_sub(10);
        let mine = super::super::scouting::predraft::seat_list(w, t, super::super::scouting::predraft::LIST_VISIT);
        let visits: Vec<usize> = if mine.is_empty() {
            list.iter().skip(from).take(30).map(|x| x.0).collect()
        } else {
            mine.iter().filter_map(|&id| super::super::scouting::place(w, id)).take(30).collect()
        };
        for i in visits {
            // A top prospect's camp turns some visits down (the club's
            // file gets no visit).
            if super::super::scouting::predraft::declines(w, i, t) {
                continue;
            }
            file(w, t, i, GM, LookKind::Visit, date.add(-10));
        }
    }
}

// --- The board (topic 9) --------------------------------------------------------------------

/// A club's board: its grade of each prospect on the aligned scale, and
/// its do-not-draft list.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Board {
    pub grade: HashMap<usize, f64>,
    pub dnd: HashSet<usize>,
    /// How many of the prospects it had a file on.
    pub filed: usize,
    /// Its character read of each filed prospect (standard deviations of
    /// risk; above zero is worse), for the war room's character line.
    #[serde(default)]
    pub risk: HashMap<usize, f32>,
}

/// Each club's board from its own files (topic 9): its read of each
/// prospect, aligned across positions like the December read, blended
/// with the public grade by how much it saw (a GM who trusts his own
/// grades leans on the public less), less what its character read costs;
/// a do-not-draft line for the worst reads. Prospects it never looked at
/// get the public grade and a rough read of its own from the services.
pub(crate) fn boards(w: &World, pool: &[(usize, f64)], align: &crate::level_rating::PositionAligner) -> Vec<Board> {
    let people: Vec<usize> = pool.iter().map(|x| x.0).collect();
    let r = reads(w, &people);
    (0..w.nfl.len()).into_par_iter().map(|t| board_from(w, t, pool, align, r.get(t))).collect()
}

/// One club's board (your war room's, on its screens), the same way.
pub(crate) fn board_for(
    w: &World,
    t: usize,
    pool: &[(usize, f64)],
    align: &crate::level_rating::PositionAligner,
) -> Board {
    let people: Vec<usize> = pool.iter().map(|x| x.0).collect();
    let ctx = Ctx::new(w, &people);
    let r: HashMap<usize, Read> = match w.clubs.get(t) {
        Some(c) => people
            .iter()
            .filter_map(|&i| c.files.get(&w.people[i].player.id).map(|f| (i, read(w, t, i, f, &ctx))))
            .collect(),
        None => HashMap::new(),
    };
    board_from(w, t, pool, align, Some(&r))
}

fn board_from(
    w: &World,
    t: usize,
    pool: &[(usize, f64)],
    align: &crate::level_rating::PositionAligner,
    r: Option<&HashMap<usize, Read>>,
) -> Board {
    let seed = w.options.seed as u64 ^ (w.year as u64) << 24;
    let mut b = Board::default();
    if t >= w.clubs.len() {
        return b;
    }
    let trust_public = 1.5 - w.clubs[t].office.gm.overconfidence;
    for &(i, public) in pool {
        let p = &w.people[i];
        let id = p.player.id as u64;
        match r.and_then(|r| r.get(&i)) {
            Some(x) => {
                b.filed += 1;
                let own = align.align(p.player.position, x.grade);
                let v_own = 1.0 / x.var.max(0.25);
                let v_pub = trust_public / 16.0;
                let mut g = (v_own * own + v_pub * public) / (v_own + v_pub);
                // His character as the club reads it (section 6
                // traits): a real worry costs up to a few points.
                g -= 1.0 * (x.risk - 0.5).max(0.0);
                if x.risk >= 2.5 || (flags(p) >= crate::career::PIPELINE.red_flag_discount.dismissed && x.risk >= 1.0) {
                    b.dnd.insert(i);
                }
                b.grade.insert(i, g);
                b.risk.insert(i, x.risk as f32);
            }
            None => {
                b.grade.insert(i, public + 3.0 * key_normal(seed ^ (t as u64) << 40 ^ 0xB0A2, id));
            }
        }
    }
    b
}

/// Draft-day news on the player's side: a slide far below where the public
/// had him (topic 11) is written on his page.
pub(crate) fn note_slide(w: &mut World, i: usize, public_rank: usize, pick: u32) {
    if public_rank < 32 && pick as usize > public_rank + 40 {
        let p = &mut w.people[i];
        p.note(format!(
            "Slid on draft night: the public boards had him {} overall and he went with pick {pick}.",
            public_rank + 1
        ));
    }
}

// --- Draft-day trades (topic 11) ----------------------------------------------------------

/// A trade on the clock: the club holding `pick` (index `slot`) moves down
/// to a club picking within the next dozen places whose top player is
/// worth clearly more to it than what it expects at its own pick (its
/// tier is about to empty), priced on the seller's chart with a later
/// pick from this draft or next year's. The seller says yes when its own
/// tier is deep (it gets a player it likes as much later) or its GM
/// values picks. `value(team, person)` is a club's value of a prospect at
/// this pick, or None if it won't take him. Returns the buyer, the swapped
/// slot and what was added.
pub struct Swap {
    pub buyer: usize,
    pub seller: usize,
    pub their_slot: usize,
    pub extra: Option<Extra>,
}

pub enum Extra {
    /// A later slot in this draft.
    Slot(usize),
    /// Next year's pick in this round.
    Future(u32),
}

pub(crate) fn trade_on_clock(
    w: &World,
    board: &[(usize, u32)],
    slot: usize,
    left: &[usize],
    value: &dyn Fn(usize, usize) -> Option<f64>,
    promised: &[(usize, u32)],
    skip: Option<usize>,
) -> Option<Swap> {
    let seller = board[slot].0;
    if seller >= w.clubs.len() || skip == Some(seller) {
        return None;
    }
    let teams = super::super::franchise::active_teams(w.year).len().max(1);
    let top_of = |t: usize, n: usize| -> Vec<f64> {
        let mut v: Vec<f64> = left.iter().filter_map(|&i| value(t, i)).collect();
        v.sort_by(|a, b| b.total_cmp(a));
        v.truncate(n);
        v
    };
    let mine = top_of(seller, 4);
    let &best = mine.first()?;
    let tier = mine.iter().filter(|&&v| v >= best - 1.5).count();
    let sg = &w.clubs[seller].office.gm;
    let willing =
        tier >= 3 || (!sg.classic_chart && tier >= 2) || w.clubs[seller].office.mode == super::office::Mode::Rebuild;
    if !willing {
        return None;
    }
    let seed = w.options.seed as u64 ^ (w.year as u64) << 20 ^ 0xD7AD;
    // Fewer calls deep into the draft (real: most trades are in the first
    // three rounds).
    let round = board[slot].1;
    let call = match round {
        1 => 0.55,
        2 | 3 => 0.4,
        _ => 0.2,
    };
    if key_uniform(seed, slot as u64) > call {
        return None;
    }
    let chart = |t: usize, s: usize| super::trades::chart(w.clubs[t].office.gm.classic_chart, s as f64 + 1.0);
    let mut best_buyer: Option<(usize, usize, f64)> = None;
    let mut seen: Vec<usize> = vec![seller];
    for k in slot + 1..(slot + 13).min(board.len()) {
        let c = board[k].0;
        if seen.contains(&c) || c >= w.clubs.len() || skip == Some(c) {
            continue;
        }
        seen.push(c);
        let theirs = top_of(c, k - slot + 1);
        let (Some(&want), Some(&expect)) = (theirs.first(), theirs.last()) else { continue };
        let g = &w.clubs[c].office.gm;
        let eager = (want - expect) * (0.6 + 0.5 * g.impatience + 0.4 * g.risk);
        if eager >= 3.0 && best_buyer.is_none_or(|b| eager > b.2) {
            best_buyer = Some((c, k, eager));
        }
    }
    let (buyer, k, _) = best_buyer?;
    // The price on the seller's chart.
    let gap = chart(seller, slot) - chart(seller, k);
    let later = (k + 1..board.len()).filter(|&q| board[q].0 == buyer).find(|&q| chart(seller, q) >= 0.9 * gap);
    let extra = match later {
        Some(q) => Some(Extra::Slot(q)),
        None => {
            // The cheapest of next year's picks that covers the gap: the
            // latest round worth enough, or one round dearer if the buyer
            // has already dealt that one, never more (trial 9: the search
            // ran from round 1, so moving up ten places in round 7 cost a
            // first; the test world then had a first paid to move up
            // twelve places in round 3, with the rounds between not his to
            // give). A pick promised earlier in this draft isn't his to
            // give again.
            let year = w.year + 1;
            let covers = |r: u32| {
                let mid = ((r - 1) as usize * teams + teams / 2) as f64;
                super::trades::chart(sg.classic_chart, mid) * 0.85 >= 0.9 * gap
            };
            (1..=7u32).rev().find(|&r| covers(r)).and_then(|cheapest| {
                (cheapest.saturating_sub(1).max(1)..=cheapest)
                    .rev()
                    .find(|&r| w.market.picks.owner_of(year, r, buyer) == buyer && !promised.contains(&(buyer, r)))
                    .map(Extra::Future)
            })
        }
    };
    // A swap worth almost nothing on the chart needs nothing added.
    if extra.is_none() && gap > 0.05 * chart(seller, slot) {
        return None;
    }
    Some(Swap { buyer, seller, their_slot: k, extra })
}

// --- The undrafted rush (topic 12) ----------------------------------------------------------

/// An undrafted player's deal from the rush: the club, its signing bonus
/// and the base salary it guarantees (outside the bonus pool).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Offer {
    pub team: usize,
    pub bonus: f64,
    pub guarantee: f64,
}

/// The bonus pool per club for undrafted rookies (real: about $75,000 in
/// 2011, rising with the rookie pool, about $92,000 in 2016). Before 2011
/// there was no pool; clubs spent about the same (proposed, not confirmed).
pub fn udfa_pool(year: i32) -> f64 {
    75_000.0 * money::price_cap(year) / money::price_cap(2011)
}

/// The rush (topic 12): each club calls the best players left on its own
/// board (its priority free agents, a few more than it has room for, never
/// one on its do-not-draft list), splits its bonus pool over them, the top
/// ones getting more and a guaranteed salary too; each player, best first,
/// takes the offer with the clearest path to the 53 by his own view of
/// himself, the money, a coach who knew him from college and home, with
/// his agent shaping the call. A club that loses its calls to other clubs
/// calls again, further down its board, with what is left of its pool,
/// until its room is full or its board runs out (trial 9: one round of
/// calls left most of the room empty, about 180 signings a year against
/// about 500 real). `room[t]` is each club's room left.
/// `seat` holds your club and the offers your war room made: your club
/// calls only those players, once, with those terms.
pub(crate) fn rush(
    w: &World,
    boards: &[Board],
    left: &[usize],
    room: &mut [usize],
    seat: Option<&(usize, Vec<(usize, Offer)>)>,
) -> Vec<(usize, Offer)> {
    let year = w.year;
    let pool = udfa_pool(year);
    let floor = 0.00002 * money::price_cap(year);
    let min_pay = money::minimum(year, 0);
    let money_unit = 0.0004 * money::price_cap(year);
    let seed = w.options.seed as u64 ^ (year as u64) << 20 ^ 0x0DFA;
    let teams = clubs(w);
    // Each club's board of the players left, best first.
    let mut lists: Vec<Vec<(usize, f64)>> = vec![Vec::new(); w.nfl.len()];
    for &t in &teams {
        let b = &boards[t];
        let mut list: Vec<(usize, f64)> = left
            .iter()
            .filter(|&&i| !b.dnd.contains(&i))
            .filter_map(|&i| b.grade.get(&i).map(|&g| (i, g)))
            .filter(|&(_, g)| g >= 50.0)
            .collect();
        list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        lists[t] = list;
    }
    let mut cursor = vec![0usize; w.nfl.len()];
    let mut pool_left = vec![pool; w.nfl.len()];
    let mut taken: HashSet<usize> = HashSet::new();
    let mut signed = Vec::new();
    loop {
        let mut offers: HashMap<usize, Vec<Offer>> = HashMap::new();
        let mut best_grade: HashMap<usize, f64> = HashMap::new();
        for &t in &teams {
            if room[t] == 0 {
                continue;
            }
            if let Some((st, mine)) = seat
                && *st == t
            {
                // Your calls go out once, on the first ring.
                if cursor[t] == 0 {
                    cursor[t] = usize::MAX / 2;
                    for &(i, o) in mine {
                        if !taken.contains(&i) && left.contains(&i) {
                            offers.entry(i).or_default().push(o);
                            let g = boards[t].grade.get(&i).copied().unwrap_or(50.0);
                            let e = best_grade.entry(i).or_insert(g);
                            *e = e.max(g);
                        }
                    }
                }
                continue;
            }
            let mut calls: Vec<(usize, usize, f64)> = Vec::new();
            while calls.len() < room[t] + 4 && cursor[t] < lists[t].len() {
                let (i, g) = lists[t][cursor[t]];
                if !taken.contains(&i) {
                    calls.push((cursor[t], i, g));
                }
                cursor[t] += 1;
            }
            let n = calls.len();
            let weights: Vec<f64> = (0..n).map(|k| ((n - k) as f64).powi(2)).collect();
            let total: f64 = weights.iter().sum::<f64>().max(1.0);
            for (k, &(at, i, g)) in calls.iter().enumerate() {
                let bonus = (pool_left[t] * weights[k] / total).max(floor);
                // The club's very top undrafted targets also get some salary guaranteed.
                let guarantee = match at {
                    0 => 0.5,
                    1 => 0.3,
                    2 => 0.15,
                    _ => 0.0,
                } * min_pay;
                offers.entry(i).or_default().push(Offer { team: t, bonus, guarantee });
                let e = best_grade.entry(i).or_insert(g);
                *e = e.max(g);
            }
        }
        if offers.is_empty() {
            break;
        }
        let mut order: Vec<(usize, f64)> = best_grade.into_iter().collect();
        order.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let before = signed.len();
        for (i, _) in order {
            let p = &w.people[i];
            let pos = p.player.position;
            // His view of himself (section 6): ego lifts it.
            let me = p.player.overall() + 3.0 * p.player.personality.traits.z(Trait::Ego);
            let agent =
                p.pro.as_ref().and_then(|c| c.agent).and_then(|a| w.market.agents.get(a)).map_or(10.0, |g| g.skill)
                    / 20.0;
            let mut best: Option<(Offer, f64)> = None;
            for o in offers.get(&i).into_iter().flatten() {
                if room[o.team] == 0 {
                    continue;
                }
                let t = o.team;
                let ahead = w.nfl[t].depth[pos.index()].iter().filter(|m| m.rating >= me).count() as f64;
                let path = depth_slots(pos).roster as f64 - ahead;
                let cash = (o.bonus + o.guarantee) / money_unit;
                let knew = (super::super::people::ties_q(w, i, t) > 1.0) as u8 as f64;
                let state = super::super::franchise::era(t, year).and_then(|e| super::super::geo::state_index(e.state));
                let home = state.is_some_and(|s| s == p.home_state) as u8 as f64;
                let luck = (1.6 - agent) * key_normal(seed ^ (t as u64) << 40, p.player.id as u64);
                let u = path + 0.6 * cash + 0.8 * knew + 0.3 * home + luck;
                if best.is_none_or(|b| u > b.1) {
                    best = Some((*o, u));
                }
            }
            if let Some((o, _)) = best {
                room[o.team] -= 1;
                pool_left[o.team] = (pool_left[o.team] - o.bonus).max(0.0);
                taken.insert(i);
                signed.push((i, o));
            }
        }
        if signed.len() == before {
            break;
        }
    }
    signed
}

/// After the rush: the files are done with (they named scouts by place)
/// and the scouts' contracts end, so the departments turn over.
pub(crate) fn after_rush(w: &mut World) {
    for c in w.clubs.iter_mut() {
        c.files.clear();
        c.files.shrink_to_fit();
    }
    // The draft is over: the departments get ready for next year's.
    let next = w.year + 1;
    staff(w, true, next);
}

/// A prospect's public rank in the class (0 is first), by the aligned
/// public grade the draft orders the pool by.
pub(crate) fn public_ranks(pool: &[(usize, f64)]) -> HashMap<usize, usize> {
    pool.iter().enumerate().map(|(k, x)| (x.0, k)).collect()
}
