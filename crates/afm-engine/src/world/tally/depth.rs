//! The depth counts of a player's game and season (Data overhaul items
//! 4-11 and 28, minor bug 44): what the position cards, facet grades,
//! throw maps, route maps, run paths and pressure maps read, counted off
//! each snap of every engine game at college and in the NFL. They live in
//! the players tally (`players::Season::depth`), sparse, by code: the
//! scalar counts (`Dc`) first, then the grids (`THROW`, `GAP`, `ROUTE`,
//! `PMAP`, `FACET`, `ALIGN_O`, `ALIGN_D`, `FG`). Append only: files keep
//! them by code.
//!
//! Counting only: nothing here makes a draw or changes a play.

use std::collections::HashMap;

use crate::game::stats::{GameStats, PlayRecord, PlayType, expected_completion_at};
use crate::player::Level;

use super::expect;

macro_rules! depth_codes {
    ($($name:ident),* $(,)?) => {
        /// The scalar depth counts, by code.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u16)]
        pub enum Dc { $($name),* }
        impl Dc {
            pub const ALL: &'static [Dc] = &[$(Dc::$name),*];
            pub fn key(self) -> &'static str {
                match self { $(Dc::$name => stringify!($name)),* }
            }
        }
    };
}

depth_codes!(
    // Passing, on his dropbacks: attempts and sacks (a scramble is its own).
    Db,
    Att,
    Cmp,
    PassYds,
    PassTd,
    Int,
    Sacked,
    SackYds,
    PassEpa,
    PassSucc,
    PassFirst,
    Scr,
    ScrYds,
    ScrEpa,
    ScrSucc,
    ScrFirst,
    // Aimed throws (not thrown away): their air yards, the air yards of the
    // ones caught, time to throw on every attempt, tight windows (under a
    // yard of separation), expected completions.
    Aimed,
    Air,
    CmpAir,
    Ttt,
    Tight,
    XCmp,
    ThrowAway,
    Btt,
    Twp,
    // Deep: 20 or more yards in the air.
    DeepAtt,
    DeepCmp,
    DeepYds,
    DeepEpa,
    DeepTd,
    DeepInt,
    DeepXCmp,
    // Play action, the blitz, under pressure (a sack is pressure).
    PaDb,
    PaAtt,
    PaCmp,
    PaYds,
    PaEpa,
    PaSucc,
    BlzDb,
    BlzAtt,
    BlzCmp,
    BlzYds,
    BlzEpa,
    BlzSacked,
    PrsDb,
    PrsAtt,
    PrsCmp,
    PrsYds,
    PrsEpa,
    PrsSacked,
    // Designed runs.
    RAtt,
    RYds,
    RTd,
    REpa,
    RSucc,
    RFirst,
    RYbc,
    RYaco,
    RXYds,
    RStuff,
    RExpl,
    RBreak,
    RBreakYds,
    RMtf,
    RBox8,
    RBox8Yds,
    RBox8Epa,
    RFum,
    // His targets.
    Tgt,
    Rec,
    RecYds,
    RecTd,
    RecFirst,
    RecEpa,
    RecSucc,
    TgtAir,
    RecAir,
    Yac,
    XYac,
    Sep,
    ContTgt,
    ContRec,
    Drop,
    TgtInt,
    Routes,
    TeamTgt,
    TeamAir,
    RecMtf,
    // Blocking, a snap at a time.
    PbSnaps,
    PbWins,
    PrsAllowed,
    SacksAllowed,
    HitsAllowed,
    HurriesAllowed,
    RbSnaps,
    RbWins,
    Pens,
    PenYds,
    Busts,
    // The pass rush and the run, a snap at a time.
    DefDbSnaps,
    DefRunSnaps,
    PrSnaps,
    PrWins,
    Prs,
    Sacks,
    Hits,
    Hurries,
    WinSpeed,
    WinPower,
    WinFinesse,
    Ttp,
    TtpN,
    GetOff,
    GetOffN,
    PrsFree,
    Blitz,
    BlitzPrs,
    BlitzSacks,
    RunStops,
    RunTkl,
    RunMiss,
    // Coverage: the throws at the man covering.
    CovSnaps,
    TgtA,
    CmpA,
    YdsA,
    TdA,
    IntA,
    PbuA,
    SepA,
    EpaA,
    YacA,
    AirA,
    FIncA,
    FirstA,
    // Tackling.
    Tkl,
    Miss,
    Stops,
    // A safety rolled down into the box (more men in the box than the front).
    InBox,
    // Kicking and returns.
    XpAtt,
    XpMade,
    ClutchAtt,
    ClutchMade,
    Ko,
    KoTb,
    KoHang,
    KoRet,
    KoRetYds,
    Punts,
    PuntYds,
    PuntNet,
    PuntIn20,
    PuntTb,
    PuntHang,
    PuntFair,
    PuntRet,
    PuntRetYds,
    Kr,
    KrYds,
    KrTd,
    KrLong,
    Pr,
    PrYds,
    PrTd,
    PrLong,
    FairCatches,
    // Designed runs that beat their expected yards.
    RBeat,
);

/// The throw grid: 3 roles (the passer, the target, the man covering) by
/// 12 cells (4 depth rows by 3 thirds of the field) by 4 counts (attempts,
/// completions, yards, EPA).
pub const THROW: u16 = 300;
/// Run paths: 5 gaps by 6 counts (carries, yards, yards before contact,
/// EPA, successes, runs of 10 or more).
pub const GAP: u16 = 450;
/// Route map: each route of `ROUTES` by 5 counts (targets, catches, yards,
/// EPA, separation).
pub const ROUTE: u16 = 500;
/// Pressure map: 8 spots (the blocker beaten: left tackle, left guard,
/// center, right guard, right tackle, a tight end, a back or receiver; or
/// unblocked) by 3 counts (pressures, sacks, seconds to the pressure).
pub const PMAP: u16 = 620;
/// Facet grades: each facet's (sum, plays).
pub const FACET: u16 = 660;
/// Snaps at each offensive spot (`ORole` as a number).
pub const ALIGN_O: u16 = 700;
/// Snaps at each defensive spot (`DSlot` as a number).
pub const ALIGN_D: u16 = 720;
/// Field goals by band (under 30, 30-39, 40-49, 50+): tries, makes.
pub const FG: u16 = 740;
/// Codes run to here.
pub const LEN: usize = 748;

/// The routes the route map keeps, by `playbook::Route::name`.
pub const ROUTES: [&str; 20] = [
    "go",
    "fade",
    "seam",
    "slant",
    "out",
    "dig",
    "curl",
    "hitch",
    "comeback",
    "post",
    "corner",
    "flat",
    "wheel",
    "drag",
    "sit",
    "swing",
    "screen",
    "bubble",
    "angle",
    "check-release",
];

/// The pressure map's spots.
pub const SPOTS: [&str; 8] = [
    "left tackle",
    "left guard",
    "center",
    "right guard",
    "right tackle",
    "a tight end",
    "a back or receiver",
    "unblocked",
];

/// The throw grid's code: role (0 passer, 1 target, 2 covering), depth row
/// (`expect::depth_row`), third (`expect::third`), count.
pub fn throw_code(role: usize, row: usize, third: usize, k: usize) -> u16 {
    THROW + (role * 48 + (row * 3 + third) * 4 + k) as u16
}

pub fn gap_code(gap: usize, k: usize) -> u16 {
    GAP + (gap * 6 + k) as u16
}

pub fn route_code(route: usize, k: usize) -> u16 {
    ROUTE + (route * 5 + k) as u16
}

pub fn pmap_code(spot: usize, k: usize) -> u16 {
    PMAP + (spot * 3 + k) as u16
}

/// The offensive spots by number (`ORole as u8`), checked by a test.
pub mod orole {
    pub const QB: u8 = 0;
    pub const RB: u8 = 1;
    pub const FB: u8 = 2;
    pub const X: u8 = 3;
    pub const Z: u8 = 4;
    pub const SLOT: u8 = 5;
    pub const SLOT2: u8 = 6;
    pub const TE: u8 = 7;
    pub const TE2: u8 = 8;
    pub const TE3: u8 = 9;
    pub const LT: u8 = 10;
    pub const LG: u8 = 11;
    pub const C: u8 = 12;
    pub const RG: u8 = 13;
    pub const RT: u8 = 14;
    pub const SLOT3: u8 = 15;
    pub const N: usize = 16;
}

/// The defensive spots by number (`DSlot as u8`), checked by a test.
pub mod dslot {
    pub const LE: u8 = 0;
    pub const RE: u8 = 1;
    pub const DTL: u8 = 2;
    pub const DTR: u8 = 3;
    pub const NT: u8 = 4;
    pub const SAM: u8 = 5;
    pub const MIKE: u8 = 6;
    pub const WILL: u8 = 7;
    pub const CBL: u8 = 8;
    pub const CBR: u8 = 9;
    pub const NCB: u8 = 10;
    pub const DCB: u8 = 11;
    pub const FS: u8 = 12;
    pub const SS: u8 = 13;
    pub const N: usize = 14;
    pub fn line(s: u8) -> bool {
        s <= NT
    }
    pub fn lb(s: u8) -> bool {
        (SAM..=WILL).contains(&s)
    }
}

/// A sparse list of depth counts, by code, sorted.
pub type Depth = Vec<(u16, f32)>;

/// Adds one sparse list into another, keeping the codes sorted.
pub fn add(into: &mut Depth, from: &[(u16, f32)]) {
    if from.is_empty() {
        return;
    }
    if into.is_empty() {
        into.extend_from_slice(from);
        return;
    }
    let mut out = Vec::with_capacity(into.len() + from.len());
    let (mut i, mut j) = (0, 0);
    while i < into.len() || j < from.len() {
        match (into.get(i), from.get(j)) {
            (Some(a), Some(b)) if a.0 == b.0 => {
                out.push((a.0, a.1 + b.1));
                i += 1;
                j += 1;
            }
            (Some(a), Some(b)) if a.0 < b.0 => {
                out.push(*a);
                i += 1;
            }
            (Some(_), Some(b)) => {
                out.push(*b);
                j += 1;
            }
            (Some(a), None) => {
                out.push(*a);
                i += 1;
            }
            (None, Some(b)) => {
                out.push(*b);
                j += 1;
            }
            (None, None) => break,
        }
    }
    // A season's list is kept all year: no more room than it needs.
    if out.capacity() > out.len() + out.len() / 4 + 8 {
        out.shrink_to_fit();
    }
    *into = out;
}

/// Takes one sparse list off another (the charting delay's games).
pub fn sub(into: &mut Depth, from: &[(u16, f32)]) {
    // A longest stays: the season's can't be taken apart by game.
    let neg: Depth = from.iter().filter(|x| !is_max(x.0)).map(|&(k, v)| (k, -v)).collect();
    add(into, &neg);
}

/// A sparse list's count for a code.
pub fn get(d: &[(u16, f32)], code: u16) -> f64 {
    d.binary_search_by_key(&code, |x| x.0).map_or(0.0, |k| d[k].1 as f64)
}

/// The most a "long" count keeps across games (the longest return).
pub fn is_max(code: u16) -> bool {
    code == Dc::KrLong as u16 || code == Dc::PrLong as u16
}

/// Adds a season's games, keeping the longest return as a longest.
pub fn add_game(into: &mut Depth, from: &[(u16, f32)]) {
    let longs: Vec<(u16, f32)> = from.iter().filter(|x| is_max(x.0)).copied().collect();
    if longs.is_empty() {
        return add(into, from);
    }
    let rest: Depth = from.iter().filter(|x| !is_max(x.0)).copied().collect();
    add(into, &rest);
    for (k, v) in longs {
        match into.binary_search_by_key(&k, |x| x.0) {
            Ok(i) => into[i].1 = into[i].1.max(v),
            Err(i) => into.insert(i, (k, v)),
        }
    }
}

/// One game's counts per player, dense while the plays are read.
struct Dense(HashMap<(usize, u32), Box<[f32; LEN]>>);

impl Dense {
    fn at(&mut self, side: usize, id: u32) -> &mut [f32; LEN] {
        self.0.entry((side, id)).or_insert_with(|| Box::new([0.0; LEN]))
    }
    fn add(&mut self, side: usize, id: u32, code: u16, v: f64) {
        self.at(side, id)[code as usize] += v as f32;
    }
    fn one(&mut self, side: usize, id: u32, c: Dc) {
        self.add(side, id, c as u16, 1.0);
    }
    fn put(&mut self, side: usize, id: u32, c: Dc, v: f64) {
        self.add(side, id, c as u16, v);
    }
    fn max(&mut self, side: usize, id: u32, c: Dc, v: f64) {
        let x = &mut self.at(side, id)[c as u16 as usize];
        *x = x.max(v as f32);
    }
}

/// The spot a blocker held on the snap, for the pressure map.
fn spot_of(r: &PlayRecord, blocker: u32) -> usize {
    let role = r.on_field[0].iter().position(|&x| x == blocker).and_then(|i| r.roles[0].get(i)).copied();
    match role {
        Some(orole::LT) => 0,
        Some(orole::LG) => 1,
        Some(orole::C) => 2,
        Some(orole::RG) => 3,
        Some(orole::RT) => 4,
        Some(orole::TE | orole::TE2 | orole::TE3) => 5,
        _ => 6,
    }
}

/// Every player's depth counts in one game, by (side, id).
pub fn game(g: &GameStats) -> HashMap<(usize, u32), Depth> {
    let level = g.level.unwrap_or(Level::NFL);
    let mut m = Dense(HashMap::new());
    // The team's aimed targets and air yards, for each receiver's share.
    let mut team = [(0f64, 0f64); 2];
    for r in &g.plays {
        let o = r.offense.min(1);
        let d = 1 - o;
        let side = |id: u32, rule: usize| r.side_of(id).unwrap_or(rule).min(1);
        // Fouls and busts count on every snap, a wiped-out one too.
        for &(_, t, id, yds, accepted) in &r.penalties {
            if accepted {
                m.one(t.min(1), id, Dc::Pens);
                m.put(t.min(1), id, Dc::PenYds, yds);
            }
        }
        for &id in &r.busts {
            m.one(side(id, d), id, Dc::Busts);
        }
        if r.nullified {
            continue;
        }
        let dropback = matches!(r.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble);
        let scrimmage = dropback || r.kind == PlayType::Run;
        if scrimmage {
            // Where each lined up.
            for (k, &id) in r.on_field[0].iter().enumerate() {
                if let Some(&role) = r.roles[0].get(k) {
                    m.add(o, id, ALIGN_O + role as u16, 1.0);
                }
            }
            let front = r.roles[1].iter().filter(|&&s| dslot::line(s) || dslot::lb(s)).count();
            let mut rolled = (r.box_count as usize).saturating_sub(front);
            // The strong safety rolls down first, then the free safety.
            let mut order: Vec<(usize, u8)> = r.roles[1].iter().copied().enumerate().collect();
            order.sort_by_key(|&(_, s)| match s {
                dslot::SS => 0,
                dslot::FS => 1,
                _ => 2,
            });
            for (k, s) in order {
                let Some(&id) = r.on_field[1].get(k) else { continue };
                m.add(d, id, ALIGN_D + s as u16, 1.0);
                if rolled > 0 && matches!(s, dslot::SS | dslot::FS) {
                    m.one(d, id, Dc::InBox);
                    rolled -= 1;
                }
            }
            for &id in &r.on_field[1] {
                m.one(d, id, if dropback { Dc::DefDbSnaps } else { Dc::DefRunSnaps });
            }
        }
        if dropback {
            pass_play(&mut m, r, o, d, level, &mut team[o]);
        }
        if r.kind == PlayType::Run
            && let Some(c) = r.carrier
        {
            run_play(&mut m, r, o, c, level);
        }
        if scrimmage {
            blocks_and_rush(&mut m, r, o, d, dropback);
            // Tackling.
            for &id in &r.tacklers {
                let s = side(id, d);
                m.one(s, id, Dc::Tkl);
                if !r.success() {
                    m.one(s, id, Dc::Stops);
                }
                if r.kind == PlayType::Run {
                    m.one(s, id, Dc::RunTkl);
                    if !r.success() {
                        m.one(s, id, Dc::RunStops);
                    }
                }
            }
            for &(id, _) in &r.missed_tackles {
                let s = side(id, d);
                m.one(s, id, Dc::Miss);
                if r.kind == PlayType::Run {
                    m.one(s, id, Dc::RunMiss);
                }
            }
        }
        kicks(&mut m, r, o, d);
    }
    // The team's targets and air yards, for every man who played offense.
    for side in 0..2 {
        for (&id, l) in &g.players[side] {
            if l.snaps_off > 0 {
                m.put(side, id, Dc::TeamTgt, team[side].0);
                m.put(side, id, Dc::TeamAir, team[side].1);
            }
            if let Some(f) = &l.facets {
                for (k, v) in f.iter().enumerate() {
                    if *v != 0.0 {
                        m.add(side, id, FACET + k as u16, *v as f64);
                    }
                }
            }
        }
    }
    m.0.into_iter()
        .map(|(k, v)| {
            let d: Depth = v.iter().enumerate().filter(|(_, x)| **x != 0.0).map(|(c, x)| (c as u16, *x)).collect();
            (k, d)
        })
        .collect()
}

fn pass_play(m: &mut Dense, r: &PlayRecord, o: usize, d: usize, level: Level, team: &mut (f64, f64)) {
    let pressured = r.pressured || r.kind == PlayType::Sack;
    if let Some(qb) = r.passer {
        m.one(o, qb, Dc::Db);
        if r.play_action {
            m.one(o, qb, Dc::PaDb);
        }
        if r.blitz {
            m.one(o, qb, Dc::BlzDb);
        }
        if pressured {
            m.one(o, qb, Dc::PrsDb);
        }
        match r.kind {
            PlayType::Sack => {
                m.one(o, qb, Dc::Sacked);
                m.put(o, qb, Dc::SackYds, -r.yards);
                m.put(o, qb, Dc::PassEpa, r.epa);
                m.put(o, qb, Dc::PassSucc, r.success() as u8 as f64);
                if r.blitz {
                    m.one(o, qb, Dc::BlzSacked);
                    m.put(o, qb, Dc::BlzEpa, r.epa);
                }
                if r.play_action {
                    m.put(o, qb, Dc::PaEpa, r.epa);
                }
                m.one(o, qb, Dc::PrsSacked);
                m.put(o, qb, Dc::PrsEpa, r.epa);
            }
            PlayType::Pass => {
                let yds = if r.complete { r.yards } else { 0.0 };
                let mut each = |c: Dc, v: f64| m.put(o, qb, c, v);
                each(Dc::Att, 1.0);
                each(Dc::Cmp, r.complete as u8 as f64);
                each(Dc::PassYds, yds);
                each(Dc::PassTd, (r.complete && r.touchdown) as u8 as f64);
                each(Dc::Int, r.interception as u8 as f64);
                each(Dc::PassEpa, r.epa);
                each(Dc::PassSucc, r.success() as u8 as f64);
                each(Dc::PassFirst, (r.complete && r.first_down) as u8 as f64);
                each(Dc::Ttt, r.time_to_throw);
                if r.throw_away {
                    each(Dc::ThrowAway, 1.0);
                } else {
                    let x = expected_completion_at(level, r.air_yards, r.separation, r.pressured);
                    each(Dc::Aimed, 1.0);
                    each(Dc::Air, r.air_yards);
                    each(Dc::XCmp, x);
                    if r.complete {
                        each(Dc::CmpAir, r.air_yards);
                    }
                    if r.separation < 1.0 {
                        each(Dc::Tight, 1.0);
                    }
                    if r.air_yards >= 20.0 {
                        each(Dc::DeepAtt, 1.0);
                        each(Dc::DeepCmp, r.complete as u8 as f64);
                        each(Dc::DeepYds, yds);
                        each(Dc::DeepEpa, r.epa);
                        each(Dc::DeepTd, (r.complete && r.touchdown) as u8 as f64);
                        each(Dc::DeepInt, r.interception as u8 as f64);
                        each(Dc::DeepXCmp, x);
                    }
                }
                // The book's own reads of a big-time throw and a
                // turnover-worthy one.
                if r.complete && r.air_yards >= 15.0 && r.separation < 1.5 {
                    each(Dc::Btt, 1.0);
                }
                if r.interception || (r.pbu.is_some() && r.separation < 0.8) {
                    each(Dc::Twp, 1.0);
                }
                if r.play_action {
                    each(Dc::PaAtt, 1.0);
                    each(Dc::PaCmp, r.complete as u8 as f64);
                    each(Dc::PaYds, yds);
                    each(Dc::PaEpa, r.epa);
                    each(Dc::PaSucc, r.success() as u8 as f64);
                }
                if r.blitz {
                    each(Dc::BlzAtt, 1.0);
                    each(Dc::BlzCmp, r.complete as u8 as f64);
                    each(Dc::BlzYds, yds);
                    each(Dc::BlzEpa, r.epa);
                }
                if pressured {
                    each(Dc::PrsAtt, 1.0);
                    each(Dc::PrsCmp, r.complete as u8 as f64);
                    each(Dc::PrsYds, yds);
                    each(Dc::PrsEpa, r.epa);
                }
            }
            // A scramble is a dropback: its EPA counts in the dropback's
            // buckets (its yards are the carrier's).
            PlayType::Scramble => {
                if r.play_action {
                    m.put(o, qb, Dc::PaEpa, r.epa);
                }
                if r.blitz {
                    m.put(o, qb, Dc::BlzEpa, r.epa);
                }
                if pressured {
                    m.put(o, qb, Dc::PrsEpa, r.epa);
                }
            }
            _ => {}
        }
        if r.kind == PlayType::Pass && !r.throw_away {
            // The throw grid, by where the ball arrived.
            let row = expect::depth_row(r.air_yards);
            let third = r.throw_y.map_or(1, |(_, at)| expect::third(at as f64));
            let yds = if r.complete { r.yards } else { 0.0 };
            let vals = [1.0, r.complete as u8 as f64, yds, r.epa];
            for (k, v) in vals.iter().enumerate() {
                m.add(o, qb, throw_code(0, row, third, k), *v);
                if let Some(t) = r.target {
                    m.add(o, t, throw_code(1, row, third, k), *v);
                }
                if let Some(cb) = r.defender {
                    m.add(d, cb, throw_code(2, row, third, k), *v);
                }
            }
            team.0 += 1.0;
            team.1 += r.air_yards;
        }
    }
    if r.kind == PlayType::Scramble
        && let Some(c) = r.carrier
    {
        m.one(o, c, Dc::Scr);
        m.put(o, c, Dc::ScrYds, r.yards);
        m.put(o, c, Dc::ScrEpa, r.epa);
        m.put(o, c, Dc::ScrSucc, r.success() as u8 as f64);
        m.put(o, c, Dc::ScrFirst, r.first_down as u8 as f64);
    }
    // Routes: every man in the pattern.
    for &id in &r.on_field[0] {
        if Some(id) != r.passer && !r.blocks.iter().any(|b| b.0 == id && b.2) {
            m.one(o, id, Dc::Routes);
        }
    }
    if r.kind == PlayType::Pass
        && !r.throw_away
        && let Some(t) = r.target
    {
        let yds = if r.complete { r.yards } else { 0.0 };
        m.one(o, t, Dc::Tgt);
        m.put(o, t, Dc::TgtAir, r.air_yards);
        m.put(o, t, Dc::RecEpa, r.epa);
        m.put(o, t, Dc::RecSucc, r.success() as u8 as f64);
        m.put(o, t, Dc::Sep, r.separation);
        m.put(o, t, Dc::Drop, r.drop as u8 as f64);
        m.put(o, t, Dc::TgtInt, r.interception as u8 as f64);
        if r.separation < 1.0 {
            m.one(o, t, Dc::ContTgt);
            m.put(o, t, Dc::ContRec, r.complete as u8 as f64);
        }
        if r.complete {
            let middle = r.throw_y.is_some_and(|(_, at)| expect::third(at as f64) == 1);
            m.one(o, t, Dc::Rec);
            m.put(o, t, Dc::RecYds, yds);
            m.put(o, t, Dc::RecTd, r.touchdown as u8 as f64);
            m.put(o, t, Dc::RecFirst, r.first_down as u8 as f64);
            m.put(o, t, Dc::RecAir, r.air_yards);
            m.put(o, t, Dc::Yac, r.yac);
            m.put(o, t, Dc::XYac, expect::expected_yac(level, r.air_yards, r.separation, r.coverage_man, middle));
            m.put(o, t, Dc::RecMtf, r.missed_tackles.iter().filter(|x| x.1 == t).count() as f64);
        }
        // The route he ran.
        if let Some(k) = ROUTES.iter().position(|x| *x == r.route) {
            let vals = [1.0, r.complete as u8 as f64, yds, r.epa, r.separation];
            for (j, v) in vals.iter().enumerate() {
                m.add(o, t, route_code(k, j), *v);
            }
        }
        if let Some(cb) = r.defender {
            m.one(d, cb, Dc::TgtA);
            m.put(d, cb, Dc::CmpA, r.complete as u8 as f64);
            m.put(d, cb, Dc::YdsA, yds);
            m.put(d, cb, Dc::TdA, (r.complete && r.touchdown) as u8 as f64);
            m.put(d, cb, Dc::SepA, r.separation);
            m.put(d, cb, Dc::EpaA, r.epa);
            m.put(d, cb, Dc::AirA, r.air_yards);
            if r.complete {
                m.put(d, cb, Dc::YacA, r.yac);
                m.put(d, cb, Dc::FirstA, r.first_down as u8 as f64);
            } else if r.pbu == Some(cb) || r.interceptor == Some(cb) {
                m.one(d, cb, Dc::FIncA);
            }
        }
    }
    if let Some(p) = r.pbu {
        m.one(d, p, Dc::PbuA);
    }
    if let Some(p) = r.interceptor {
        m.one(d, p, Dc::IntA);
    }
    // Coverage snaps: every defender who didn't rush.
    for &id in &r.on_field[1] {
        if !r.pressures.contains(&id) && !r.sackers.contains(&id) && !r.blocks.iter().any(|b| b.1 == id && b.2) {
            m.one(d, id, Dc::CovSnaps);
        }
    }
}

fn run_play(m: &mut Dense, r: &PlayRecord, o: usize, c: u32, level: Level) {
    let gap = expect::run_gap(&r.call);
    let outside = matches!(gap, 0 | 4);
    let x = expect::expected_rush(level, r.box_count, outside, r.yardline_100, r.togo);
    let mut each = |k: Dc, v: f64| m.put(o, c, k, v);
    each(Dc::RAtt, 1.0);
    each(Dc::RYds, r.yards);
    each(Dc::RTd, r.touchdown as u8 as f64);
    each(Dc::REpa, r.epa);
    each(Dc::RSucc, r.success() as u8 as f64);
    each(Dc::RFirst, r.first_down as u8 as f64);
    each(Dc::RYbc, r.before_contact);
    each(Dc::RYaco, (r.yards - r.before_contact).max(0.0));
    each(Dc::RXYds, x);
    each(Dc::RBeat, (r.yards > x) as u8 as f64);
    each(Dc::RStuff, (r.yards <= 0.0) as u8 as f64);
    each(Dc::RExpl, (r.yards >= 10.0) as u8 as f64);
    if r.yards >= 15.0 {
        each(Dc::RBreak, 1.0);
        each(Dc::RBreakYds, r.yards);
    }
    each(Dc::RMtf, r.missed_tackles.iter().filter(|x| x.1 == c).count() as f64);
    if r.box_count >= 8 {
        each(Dc::RBox8, 1.0);
        each(Dc::RBox8Yds, r.yards);
        each(Dc::RBox8Epa, r.epa);
    }
    each(Dc::RFum, (r.fumbled == Some(c)) as u8 as f64);
    let g = gap as usize;
    let vals = [1.0, r.yards, r.before_contact, r.epa, r.success() as u8 as f64, (r.yards >= 10.0) as u8 as f64];
    for (k, v) in vals.iter().enumerate() {
        m.add(o, c, gap_code(g, k), *v);
    }
}

/// Blocks, the pass rush and the blitz, a snap at a time.
fn blocks_and_rush(m: &mut Dense, r: &PlayRecord, o: usize, d: usize, dropback: bool) {
    let side = |id: u32, rule: usize| r.side_of(id).unwrap_or(rule).min(1);
    // Each blocker's snap: pass or run, held or beaten, and what it cost.
    let mut blockers: Vec<u32> = r.blocks.iter().map(|b| b.0).collect();
    blockers.sort_unstable();
    blockers.dedup();
    for b in blockers {
        let mine: Vec<_> = r.blocks.iter().filter(|x| x.0 == b).collect();
        let s = side(b, o);
        if mine.iter().any(|x| x.2) {
            m.one(s, b, Dc::PbSnaps);
            let beaten: Vec<u32> = mine.iter().filter(|x| x.2 && !x.3).map(|x| x.1).collect();
            let sack = beaten.iter().any(|x| r.sackers.contains(x));
            let pressure = sack || beaten.iter().any(|x| r.pressures.contains(x));
            let hit = !sack && beaten.iter().any(|x| r.hits.contains(x));
            let held = mine.iter().filter(|x| x.2).all(|x| x.3 || x.4 >= 2.5);
            if held && !pressure {
                m.one(s, b, Dc::PbWins);
            }
            if pressure {
                m.one(s, b, Dc::PrsAllowed);
                if sack {
                    m.one(s, b, Dc::SacksAllowed);
                } else if hit {
                    m.one(s, b, Dc::HitsAllowed);
                } else {
                    m.one(s, b, Dc::HurriesAllowed);
                }
            }
        }
        if mine.iter().any(|x| !x.2) {
            m.one(s, b, Dc::RbSnaps);
            if mine.iter().filter(|x| !x.2).all(|x| x.3) {
                m.one(s, b, Dc::RbWins);
            }
        }
    }
    if !dropback {
        return;
    }
    // The pass rush: every man blocked on a pass, who pressured or sacked.
    let mut rushers: Vec<u32> = r.blocks.iter().filter(|b| b.2).map(|b| b.1).collect();
    rushers.extend(r.pressures.iter().copied());
    rushers.extend(r.sackers.iter().copied());
    rushers.sort_unstable();
    rushers.dedup();
    let share = if r.sackers.len() > 1 { 0.5 } else { 1.0 };
    for id in rushers {
        let s = side(id, d);
        if s != d {
            continue;
        }
        m.one(s, id, Dc::PrSnaps);
        let slot = r.on_field[1].iter().position(|&x| x == id).and_then(|k| r.roles[1].get(k)).copied();
        let blitz = slot.is_some_and(|x| !dslot::line(x));
        if blitz {
            m.one(s, id, Dc::Blitz);
        }
        if r.blocks.iter().any(|b| b.1 == id && b.2 && !b.3 && b.4 < 2.5) {
            m.one(s, id, Dc::PrWins);
        }
        for &(rid, mv, _) in &r.rush_wins {
            if rid == id {
                let c = match mv {
                    crate::game::sim::Move::Speed => Dc::WinSpeed,
                    crate::game::sim::Move::Power => Dc::WinPower,
                    crate::game::sim::Move::Finesse => Dc::WinFinesse,
                };
                m.one(s, id, c);
            }
        }
        if let Some(&(_, t)) = r.contact.iter().find(|c| c.0 == id) {
            m.put(s, id, Dc::GetOff, t as f64);
            m.one(s, id, Dc::GetOffN);
        }
        let sacked = r.sackers.iter().take(2).any(|&x| x == id);
        let pressured = sacked || r.pressures.contains(&id);
        if !pressured {
            continue;
        }
        m.one(s, id, Dc::Prs);
        if sacked {
            m.put(s, id, Dc::Sacks, share);
        } else if r.hits.contains(&id) {
            m.one(s, id, Dc::Hits);
        } else {
            m.one(s, id, Dc::Hurries);
        }
        if blitz {
            m.one(s, id, Dc::BlitzPrs);
            if sacked {
                m.put(s, id, Dc::BlitzSacks, share);
            }
        }
        let at = if r.pressure_at > 0.0 { r.pressure_at as f64 } else { r.time_to_throw };
        if at > 0.0 {
            m.put(s, id, Dc::Ttp, at);
            m.one(s, id, Dc::TtpN);
        }
        // Where he won: the blocker he beat, else unblocked.
        let beat =
            r.blocks.iter().find(|b| b.1 == id && b.2 && !b.3).or_else(|| r.blocks.iter().find(|b| b.1 == id && b.2));
        let spot = match beat {
            Some(b) => spot_of(r, b.0),
            None => {
                m.one(s, id, Dc::PrsFree);
                7
            }
        };
        m.add(s, id, pmap_code(spot, 0), 1.0);
        m.add(s, id, pmap_code(spot, 1), if sacked { share } else { 0.0 });
        m.add(s, id, pmap_code(spot, 2), at);
    }
}

fn kicks(m: &mut Dense, r: &PlayRecord, o: usize, d: usize) {
    match r.kind {
        PlayType::FieldGoal => {
            if let Some(k) = r.kicker {
                let b = crate::game::stats::fg_band(r.kick_distance) as u16;
                m.add(o, k, FG + 2 * b, 1.0);
                m.add(o, k, FG + 2 * b + 1, r.kick_good as u8 as f64);
                if r.kick_pressure > 0.6 {
                    m.one(o, k, Dc::ClutchAtt);
                    m.put(o, k, Dc::ClutchMade, r.kick_good as u8 as f64);
                }
            }
        }
        PlayType::ExtraPoint => {
            if let Some(k) = r.kicker {
                m.one(o, k, Dc::XpAtt);
                m.put(o, k, Dc::XpMade, r.kick_good as u8 as f64);
            }
        }
        PlayType::Kickoff => {
            if let Some(k) = r.kicker {
                m.one(o, k, Dc::Ko);
                m.put(o, k, Dc::KoTb, r.touchback as u8 as f64);
                m.put(o, k, Dc::KoHang, r.hang as f64);
                if r.returner.is_some() && !r.touchback {
                    m.one(o, k, Dc::KoRet);
                    m.put(o, k, Dc::KoRetYds, r.return_yards);
                }
            }
            if let Some(kr) = r.returner
                && !r.touchback
            {
                m.one(d, kr, Dc::Kr);
                m.put(d, kr, Dc::KrYds, r.return_yards);
                m.put(d, kr, Dc::KrTd, (r.touchdown && !r.turnover) as u8 as f64);
                m.max(d, kr, Dc::KrLong, r.return_yards);
            }
        }
        PlayType::Punt => {
            if let Some(k) = r.kicker {
                m.one(o, k, Dc::Punts);
                m.put(o, k, Dc::PuntYds, r.kick_distance);
                m.put(o, k, Dc::PuntNet, r.kick_distance - r.return_yards - if r.touchback { 20.0 } else { 0.0 });
                m.put(o, k, Dc::PuntIn20, r.inside20 as u8 as f64);
                m.put(o, k, Dc::PuntTb, r.touchback as u8 as f64);
                m.put(o, k, Dc::PuntHang, r.hang as f64);
                m.put(o, k, Dc::PuntFair, r.fair_catch as u8 as f64);
                if r.returner.is_some() && !r.fair_catch && !r.touchback {
                    m.one(o, k, Dc::PuntRet);
                    m.put(o, k, Dc::PuntRetYds, r.return_yards);
                }
            }
            if let Some(pr) = r.returner {
                if r.fair_catch {
                    m.one(d, pr, Dc::FairCatches);
                } else if !r.touchback {
                    m.one(d, pr, Dc::Pr);
                    m.put(d, pr, Dc::PrYds, r.return_yards);
                    m.put(d, pr, Dc::PrTd, (r.touchdown && !r.turnover) as u8 as f64);
                    m.max(d, pr, Dc::PrLong, r.return_yards);
                }
            }
        }
        _ => {}
    }
}
