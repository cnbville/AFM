//! The roster shapes the call (scheme-fit items 10, 11 and 23, lane SF10).
//!
//! A play caller knows what each call gains against the looks he expects
//! (the coaches' table, `calling`) and, with this, what it gains from his
//! own men: each call's yards over the typical team's, from how far each
//! of his men sits from the typical man at his slot in the ratings that
//! call rewards. On today's table a passer's deep ball and arm add most to
//! the deep crossers, smash and mills, and a shaky right tackle takes most
//! from them, so a weak line throws quick; linemen and tight ends who drive
//! their men add to the runs that lean on them, a fast back to the toss
//! and duo, a quarterback who can run to his own power and speed option;
//! tight ends who block add to 12, 13 and 22 on runs; a room of man
//! corners gives up less in man, a free safety who reads the field less in
//! quarters and Cover 1 with a blitz; a heavy back who drives his legs
//! converts third and short.
//!
//! Every weight is measured on the match engine, none is set by hand: the
//! typical NFL team (`afm-tools`, `fit::base_team`) plays each call on many
//! snaps with its men's watched ratings moved at random around the typical
//! man's (`call_values roster`), and a weight is the yards one point of a
//! rating at a slot adds to the call (the table's own unit: yards to 40,
//! less 45 for a turnover). Each call's weights are pooled with the calls
//! of its kind and kept only past the noise (`data/roster_values.csv`).
//! The men are read about the level's typical man at their place on a
//! depth chart (`synthetic::typical_at`), so the one table serves every
//! level and a team like the level's typical one leans nowhere.
//!
//! The callers lean on it as they lean on the look table, by their
//! creativity (topic 6.2: "calls that fit his players"): a sharp one leans
//! harder, and every call still keeps its weight from the scheme, the
//! situation, the looks and today's results (`calling`).

use super::playbook::{COVERAGE_COUNT, Coverage, PASS_CONCEPTS, Personnel, Pressure, RUN_CONCEPTS};
use super::roster::GamePlayer;
use super::synthetic::{typical_at, typical_fb};
use crate::attributes::Attr;
use crate::player::Level;
use crate::positions::Position;

/// The measured weights (`call_values roster` writes them): one line a
/// weight, `cell,slot,feature,weight`.
const ROSTER_VALUES_CSV: &str = include_str!("../../data/roster_values.csv");

/// A watched man: the offense's by depth order, the defense's by the job
/// his staff names him to (L1, C1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Slot {
    Qb,
    Rb,
    Fb,
    Wr1,
    Wr2,
    Wr3,
    Te1,
    Te2,
    Te3,
    Ot1,
    Ot2,
    Iol1,
    Iol2,
    Iol3,
    Edge1,
    Edge2,
    Dt1,
    Dt2,
    Sam,
    Mike,
    Will,
    Cb1,
    Cb2,
    Nb,
    Fs,
    Ss,
}

pub const SLOTS: usize = 26;

impl Slot {
    pub const ALL: [Slot; SLOTS] = [
        Slot::Qb,
        Slot::Rb,
        Slot::Fb,
        Slot::Wr1,
        Slot::Wr2,
        Slot::Wr3,
        Slot::Te1,
        Slot::Te2,
        Slot::Te3,
        Slot::Ot1,
        Slot::Ot2,
        Slot::Iol1,
        Slot::Iol2,
        Slot::Iol3,
        Slot::Edge1,
        Slot::Edge2,
        Slot::Dt1,
        Slot::Dt2,
        Slot::Sam,
        Slot::Mike,
        Slot::Will,
        Slot::Cb1,
        Slot::Cb2,
        Slot::Nb,
        Slot::Fs,
        Slot::Ss,
    ];

    pub fn name(self) -> &'static str {
        [
            "QB", "RB", "FB", "WR1", "WR2", "WR3", "TE1", "TE2", "TE3", "OT1", "OT2", "IOL1", "IOL2", "IOL3", "EDGE1",
            "EDGE2", "DT1", "DT2", "SAM", "MIKE", "WILL", "CB1", "CB2", "NB", "FS", "SS",
        ][self as usize]
    }

    pub fn from_name(s: &str) -> Option<Slot> {
        Slot::ALL.into_iter().find(|x| x.name() == s)
    }

    pub fn offense(self) -> bool {
        (self as u8) < Slot::Edge1 as u8
    }

    /// The slot's man on a depth chart: his position and place (0 the
    /// first), the defense's jobs by place as a named order names them
    /// (`spots::board_jobs`: SAM, MIKE, WILL; two corners, then the nickel;
    /// FS, SS). The fullback is a back or tight end past the first, read
    /// about the typical fullback (`synthetic::typical_fb`).
    pub fn typical_man(self) -> (Position, usize) {
        use Position as P;
        match self {
            Slot::Qb => (P::QB, 0),
            Slot::Rb => (P::RB, 0),
            Slot::Fb => (P::RB, 1),
            Slot::Wr1 => (P::WR, 0),
            Slot::Wr2 => (P::WR, 1),
            Slot::Wr3 => (P::WR, 2),
            Slot::Te1 => (P::TE, 0),
            Slot::Te2 => (P::TE, 1),
            Slot::Te3 => (P::TE, 2),
            Slot::Ot1 => (P::OT, 0),
            Slot::Ot2 => (P::OT, 1),
            Slot::Iol1 => (P::IOL, 0),
            Slot::Iol2 => (P::IOL, 1),
            Slot::Iol3 => (P::IOL, 2),
            Slot::Edge1 => (P::EDGE, 0),
            Slot::Edge2 => (P::EDGE, 1),
            Slot::Dt1 => (P::DT, 0),
            Slot::Dt2 => (P::DT, 1),
            Slot::Sam => (P::LB, 0),
            Slot::Mike => (P::LB, 1),
            Slot::Will => (P::LB, 2),
            Slot::Cb1 => (P::CB, 0),
            Slot::Cb2 => (P::CB, 1),
            Slot::Nb => (P::CB, 2),
            Slot::Fs => (P::S, 0),
            Slot::Ss => (P::S, 1),
        }
    }
}

/// A feature the table reads: a rating (its index) or his weight.
pub const WEIGHT: u8 = u8::MAX;

pub fn feature_name(f: u8) -> String {
    if f == WEIGHT { "Weight".to_string() } else { format!("{:?}", Attr::ALL[f as usize]) }
}

fn feature_of(s: &str) -> Option<u8> {
    if s == "Weight" {
        return Some(WEIGHT);
    }
    Attr::ALL.iter().position(|a| format!("{a:?}") == s).map(|k| k as u8)
}

/// The typical man's number in a feature at a slot, at the level: what the
/// table's weights are read about. He is the level's typical man at that
/// place on a depth chart (`synthetic::typical_at`: a team's second tight
/// end about the typical second tight end, not the typical starter), and
/// the fullback the typical fullback; the measuring team's men sit there,
/// so a team like the level's typical one leans nowhere.
pub fn reference(level: Level, slot: Slot, f: u8) -> f64 {
    let (pos, depth) = slot.typical_man();
    let man = if slot == Slot::Fb { typical_fb(level) } else { typical_at(level, pos, depth) };
    if f == WEIGHT { man.1 } else { man.0[f as usize] }
}

/// How far a man sits from the typical man at the slot in a feature.
pub fn deviation(level: Level, slot: Slot, p: &GamePlayer, f: u8) -> f64 {
    let v = if f == WEIGHT { p.weight } else { p.a(Attr::ALL[f as usize]) };
    v - reference(level, slot, f)
}

// ------------------------------------------------------------------ cells

/// The personnel groups the table weighs (`group:<name>:<run|pass>`).
pub const GROUPS: [Personnel; 6] =
    [Personnel::P11, Personnel::P12, Personnel::P13, Personnel::P21, Personnel::P22, Personnel::P10];

pub const PASS_N: usize = PASS_CONCEPTS.len();
pub const RUN_N: usize = RUN_CONCEPTS.len();
pub const LOOKS: usize = 2 * COVERAGE_COUNT - 1;

/// One call or look the table weighs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// A pass concept, straight or off the run fake (a play-action
    /// concept's own row is its faked one).
    Pass(usize, bool),
    /// A run concept (`RUN_CONCEPTS` order).
    Run(usize),
    /// A personnel group's runs (false) or passes (true).
    Group(usize, bool),
    /// The back on the field on short yardage (true) or a passing down:
    /// the chance the snap converts.
    Sit(bool),
    /// A coverage and pressure (`calling::col`): yards the offense gains.
    Look(usize),
    /// Base (false) or nickel (true) personnel against three receivers,
    /// against their runs (false) or passes (true).
    DPers(bool, bool),
}

const CELLS: usize = 2 * PASS_N + RUN_N + 2 * GROUPS.len() + 2 + LOOKS + 4;

impl Cell {
    fn id(self) -> usize {
        match self {
            Cell::Pass(i, fake) => i + if fake { PASS_N } else { 0 },
            Cell::Run(k) => 2 * PASS_N + k,
            Cell::Group(g, pass) => 2 * PASS_N + RUN_N + 2 * g + pass as usize,
            Cell::Sit(short) => 2 * PASS_N + RUN_N + 2 * GROUPS.len() + (!short) as usize,
            Cell::Look(c) => 2 * PASS_N + RUN_N + 2 * GROUPS.len() + 2 + c,
            Cell::DPers(nickel, pass) => {
                2 * PASS_N + RUN_N + 2 * GROUPS.len() + 2 + LOOKS + 2 * nickel as usize + pass as usize
            }
        }
    }

    /// The cell's name in the table.
    pub fn name(self) -> String {
        match self {
            Cell::Pass(i, false) => format!("pass:{}", PASS_CONCEPTS[i].name),
            Cell::Pass(i, true) => format!("fake:{}", PASS_CONCEPTS[i].name),
            Cell::Run(k) => format!("run:{:?}", RUN_CONCEPTS[k]),
            Cell::Group(g, pass) => format!("group:{}:{}", GROUPS[g].name(), if pass { "pass" } else { "run" }),
            Cell::Sit(true) => "sit:short".to_string(),
            Cell::Sit(false) => "sit:third".to_string(),
            Cell::Look(c) => format!("look:{}", look_name(c)),
            Cell::DPers(nickel, pass) => {
                format!("dpers:{}:{}", if nickel { "Nickel" } else { "Base" }, if pass { "pass" } else { "run" })
            }
        }
    }

    pub fn from_name(s: &str) -> Option<Cell> {
        let (kind, rest) = s.split_once(':')?;
        match kind {
            "pass" | "fake" => PASS_CONCEPTS.iter().position(|p| p.name == rest).map(|i| Cell::Pass(i, kind == "fake")),
            "run" => RUN_CONCEPTS.iter().position(|r| format!("{r:?}") == rest).map(Cell::Run),
            "group" => {
                let (g, k) = rest.split_once(':')?;
                GROUPS.iter().position(|p| p.name() == g).map(|g| Cell::Group(g, k == "pass"))
            }
            "sit" => Some(Cell::Sit(rest == "short")),
            "look" => (0..LOOKS).find(|&c| look_name(c) == rest).map(Cell::Look),
            "dpers" => {
                let (p, k) = rest.split_once(':')?;
                Some(Cell::DPers(p == "Nickel", k == "pass"))
            }
            _ => None,
        }
    }
}

/// A look column's coverage and pressure (`calling::col`): Cover 0 with
/// the double-A blitz, then each other coverage with four rushers and with
/// a linebacker blitz.
pub fn look_of(c: usize) -> (Coverage, Pressure) {
    if c == 0 {
        return (Coverage::Cover0, Pressure::DoubleA);
    }
    let k = c.div_ceil(2);
    (Coverage::ALL[k], if c % 2 == 1 { Pressure::Four } else { Pressure::LbBlitz })
}

fn look_name(c: usize) -> String {
    let (cov, p) = look_of(c);
    format!("{cov:?}/{p:?}")
}

/// Every cell, in table order.
pub fn all_cells() -> Vec<Cell> {
    let mut v: Vec<Cell> = (0..PASS_N).map(|i| Cell::Pass(i, false)).collect();
    v.extend((0..PASS_N).map(|i| Cell::Pass(i, true)));
    v.extend((0..RUN_N).map(Cell::Run));
    for g in 0..GROUPS.len() {
        v.push(Cell::Group(g, false));
        v.push(Cell::Group(g, true));
    }
    v.push(Cell::Sit(true));
    v.push(Cell::Sit(false));
    v.extend((0..LOOKS).map(Cell::Look));
    for nickel in [false, true] {
        for pass in [false, true] {
            v.push(Cell::DPers(nickel, pass));
        }
    }
    v
}

// ------------------------------------------------------------------ the table

struct Table {
    /// The distinct men and features the table reads.
    pairs: Vec<(Slot, u8)>,
    /// Each cell's weights: (pair, yards a point).
    cells: Vec<Vec<(u16, f32)>>,
}

fn table() -> &'static Table {
    static T: std::sync::OnceLock<Table> = std::sync::OnceLock::new();
    T.get_or_init(|| parse(ROSTER_VALUES_CSV))
}

fn parse(csv: &str) -> Table {
    let mut t = Table { pairs: Vec::new(), cells: vec![Vec::new(); CELLS] };
    for line in csv.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() < 4 {
            continue;
        }
        let (Some(cell), Some(slot), Some(feat), Ok(w)) =
            (Cell::from_name(f[0]), Slot::from_name(f[1]), feature_of(f[2]), f[3].trim().parse::<f32>())
        else {
            continue;
        };
        if w == 0.0 {
            continue;
        }
        let k = match t.pairs.iter().position(|&p| p == (slot, feat)) {
            Some(k) => k,
            None => {
                t.pairs.push((slot, feat));
                t.pairs.len() - 1
            }
        };
        t.cells[cell.id()].push((k as u16, w));
    }
    t
}

/// The men a side has at each slot this snap (indices into its players).
pub type Men = [Option<usize>; SLOTS];

/// Each read man's deviation for every pair the table reads (zero for a
/// slot nobody fills).
fn deviations(players: &[GamePlayer], men: &Men, level: Level, offense: bool) -> Vec<f64> {
    table()
        .pairs
        .iter()
        .map(|&(slot, f)| {
            if slot.offense() != offense {
                return 0.0;
            }
            men[slot as usize].map_or(0.0, |i| deviation(level, slot, &players[i], f))
        })
        .collect()
}

fn value(dev: &[f64], cell: Cell) -> f32 {
    table().cells[cell.id()].iter().map(|&(k, w)| w as f64 * dev[k as usize]).sum::<f64>() as f32
}

/// What each offensive call gains from the offense's men, in the table's
/// yards over the typical team's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OffFit {
    pub pass: [f32; PASS_N],
    pub fake: [f32; PASS_N],
    pub run: [f32; RUN_N],
    /// Each group's runs and passes (`GROUPS` order).
    pub group: [[f32; 2]; 6],
}

/// What each defensive look gives up with the defense's men, in yards the
/// offense gains over what it gains on the typical team (below zero, his
/// men take yards away).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DefFit {
    pub look: [f32; LOOKS],
    /// Base and nickel personnel against three receivers' runs and passes.
    pub dpers: [[f32; 2]; 2],
}

impl OffFit {
    pub fn of(players: &[GamePlayer], men: &Men, level: Level) -> OffFit {
        let dev = deviations(players, men, level, true);
        let mut f = OffFit::default();
        for i in 0..PASS_N {
            f.pass[i] = value(&dev, Cell::Pass(i, false));
            f.fake[i] = value(&dev, Cell::Pass(i, true));
        }
        for k in 0..RUN_N {
            f.run[k] = value(&dev, Cell::Run(k));
        }
        for g in 0..GROUPS.len() {
            f.group[g] = [value(&dev, Cell::Group(g, false)), value(&dev, Cell::Group(g, true))];
        }
        f
    }

    /// A group's runs or passes (zero for a group the table doesn't weigh).
    pub fn group_of(&self, p: Personnel, pass: bool) -> f64 {
        GROUPS.iter().position(|&g| g == p).map_or(0.0, |g| self.group[g][pass as usize] as f64)
    }
}

impl DefFit {
    pub fn of(players: &[GamePlayer], men: &Men, level: Level) -> DefFit {
        let dev = deviations(players, men, level, false);
        let mut f = DefFit::default();
        for c in 0..LOOKS {
            f.look[c] = value(&dev, Cell::Look(c));
        }
        for nickel in [false, true] {
            for pass in [false, true] {
                f.dpers[nickel as usize][pass as usize] = value(&dev, Cell::DPers(nickel, pass));
            }
        }
        f
    }
}

/// The offense's men by a depth chart alone, everyone dressed: for a draft
/// of your call sheet before the week (`world::plans`). The fullback is a
/// back or tight end past the first, the one rated best there.
pub fn depth_men(team: &super::roster::GameTeam, depth: &super::roster::Depth) -> Men {
    use Slot as S;
    let mut m: Men = [None; SLOTS];
    let fill = |pos: Position, slots: &[Slot], m: &mut Men| {
        for (k, &s) in slots.iter().enumerate() {
            m[s as usize] = depth.list(pos).get(k).copied();
        }
    };
    fill(Position::QB, &[S::Qb], &mut m);
    fill(Position::RB, &[S::Rb], &mut m);
    fill(Position::WR, &[S::Wr1, S::Wr2, S::Wr3], &mut m);
    fill(Position::TE, &[S::Te1, S::Te2, S::Te3], &mut m);
    fill(Position::OT, &[S::Ot1, S::Ot2], &mut m);
    fill(Position::IOL, &[S::Iol1, S::Iol2, S::Iol3], &mut m);
    let fb = crate::spots::Spot::FB.index();
    m[S::Fb as usize] = depth
        .list(Position::RB)
        .iter()
        .skip(1)
        .chain(depth.list(Position::TE).iter().skip(1))
        .copied()
        .max_by(|&a, &b| team.players[a].spot_r[fb].total_cmp(&team.players[b].spot_r[fb]));
    m
}

/// How much more often a down's snaps must convert with another back for an
/// AI staff to put him on for it (item 11): one point of the chance, so a
/// back only a shade better stays on the bench, as a staff keeps its
/// starter on for his rhythm and to show the defense nothing (proposed,
/// not confirmed).
pub const SIT_EDGE: f64 = 0.01;

/// A back's worth on short yardage (true) or a passing down: how much more
/// often the snap converts with him on the field than with the typical
/// starting back (item 11).
pub fn back_value(p: &GamePlayer, level: Level, short: bool) -> f64 {
    let t = table();
    t.cells[Cell::Sit(short).id()]
        .iter()
        .map(|&(k, w)| {
            let (slot, f) = t.pairs[k as usize];
            if slot == Slot::Rb { w as f64 * deviation(level, Slot::Rb, p, f) } else { 0.0 }
        })
        .sum()
}

/// The table's lines for a cell, for the tools and the docs: (slot,
/// feature, weight).
pub fn weights(cell: Cell) -> Vec<(Slot, u8, f32)> {
    let t = table();
    t.cells[cell.id()].iter().map(|&(k, w)| (t.pairs[k as usize].0, t.pairs[k as usize].1, w)).collect()
}
