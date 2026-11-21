//! What beats what on the field: every pass and run concept against every
//! defensive look, played on the match engine between two typical NFL
//! teams. The result is the football knowledge coordinators call from
//! (`crates/afm-engine/data/call_values.csv`): a sharp one trusts it, a
//! poor one barely uses it. Rerun after engine changes (2,000 snaps a cell
//! since minor bug 135; at 400 one cell could be off by a yard or more):
//! `cargo run --release --bin call_values -- 2000 > crates/afm-engine/data/call_values.csv`
//! Each value is mean yards on the snap (capped at 40) minus 45 for a turnover.
//! Every concept that can carry the run fake has a second row with it,
//! "play-action <name>" (scheme-fit item 3), so a caller weighs the fake
//! against the look as he weighs the concept.
//!
//! Both teams are the level's typical team (`fit::base_team`), each man at
//! the typical man of his place on the depth chart (lane SF10, minor bug
//! 405: the fixed synthetic teams the table played before had a first
//! tight end 17 points of Catching under the typical starter's, so a change
//! to how receivers play moved single cells far).
//!
//! `call_values roster [draws] > crates/afm-engine/data/roster_values.csv`
//! measures what each call gains from a team's own men (lane SF10,
//! `roster_fit`): on every draw each watched man's ratings and weight move
//! at random about the typical man's, and every call plays one snap; a
//! call's weights are its yards (the same unit) on each point of a rating,
//! from one regression over the draws, pooled with the calls of its kind
//! and kept only past the noise (`shrink`).

use afm_engine::attributes::Attr;
use afm_engine::game::calling::shell_mix;
use afm_engine::game::flow::{trace_snap_at, trace_snap_opt};
use afm_engine::game::playbook::*;
use afm_engine::game::roster::GameTeam;
use afm_engine::game::roster_fit::{self, Cell, SLOTS, Slot, WEIGHT};
use afm_engine::game::{Front, OffScheme, Shell};
use afm_engine::player::{Level, Player};
use afm_tools::ab::{Knob, Part, change_one, synthetic};
use afm_tools::fit::{Base, base_team};
use rayon::prelude::*;

const LEVEL: Level = Level::NFL;

/// The two typical teams every cell plays.
struct Teams {
    off: GameTeam,
    off_people: Vec<Player>,
    def: GameTeam,
    /// The same defense with a 3-4 staff, for the odd and tite fronts (a
    /// nose and two ends); the rest are a 4-3 staff's.
    def34: GameTeam,
    def_people: Vec<Player>,
    /// Each slot's man: on the offense for its slots, the defense for its.
    at: [usize; SLOTS],
}

/// Each slot's place in `fit::base_team`'s list (`fit::ROLES`). The
/// fullback is the second back, set to the typical fullback, so he is a man
/// of his own and not the second or third tight end.
fn role(s: Slot) -> usize {
    use Slot as S;
    match s {
        S::Qb => 0,
        S::Rb => 1,
        S::Fb => 2,
        S::Wr1 => 4,
        S::Wr2 => 5,
        S::Wr3 => 6,
        S::Te1 => 7,
        S::Te2 => 8,
        S::Te3 => 9,
        S::Ot1 => 10,
        S::Iol1 => 11,
        S::Iol2 => 12,
        S::Iol3 => 13,
        S::Ot2 => 14,
        S::Edge1 => 15,
        S::Edge2 => 16,
        S::Dt1 => 17,
        S::Dt2 => 18,
        S::Sam => 20,
        S::Mike => 21,
        S::Will => 22,
        S::Cb1 => 23,
        S::Cb2 => 24,
        S::Nb => 25,
        S::Fs => 26,
        S::Ss => 27,
    }
}

/// Moves a man to the typical man at his slot in every rating and his
/// weight (`roster_fit::reference`).
fn set_typical(team: &mut GameTeam, people: &[Player], k: usize, slot: Slot) {
    let gp = &mut team.players[k];
    let mut ks: Vec<Knob> = Attr::ALL
        .iter()
        .map(|&a| Knob::Rating(a, roster_fit::reference(LEVEL, slot, a.index() as u8) - gp.attrs.get(a)))
        .collect();
    ks.push(Knob::Set(Part::Weight, roster_fit::reference(LEVEL, slot, WEIGHT)));
    change_one(gp, &people[k], &ks, LEVEL);
}

fn teams(seed: u32) -> Teams {
    let sq = synthetic(LEVEL, seed);
    let Base { team: mut off, at: off_at } = base_team(&sq[0], LEVEL);
    let Base { team: mut def, at: def_at } = base_team(&sq[1], LEVEL);
    let mut at = [0; SLOTS];
    for s in Slot::ALL {
        if s.offense() {
            at[s as usize] = off_at[role(s)];
            set_typical(&mut off, &sq[0].people, at[s as usize], s);
        } else {
            at[s as usize] = def_at[role(s)];
            set_typical(&mut def, &sq[1].people, at[s as usize], s);
        }
    }
    off.staff.off_scheme = OffScheme::WestCoast;
    def.staff.front = Front::FourThree;
    let mut def34 = def.clone();
    def34.staff.front = Front::ThreeFour;
    Teams { off, off_people: sq[0].people.clone(), def, def34, def_people: sq[1].people.clone(), at }
}

/// The play's value in the table's unit: yards to 40, less 45 for a
/// turnover.
fn value(r: &afm_engine::game::stats::PlayRecord) -> f64 {
    r.yards.min(40.0) - if r.turnover { 45.0 } else { 0.0 }
}

/// Each run from the look it is run from: gun for the reads and QB runs, a
/// fullback for the iso, jet motion for the sweep.
fn run_call(rc: RunConcept, pers: Option<Personnel>) -> OffCall {
    let gun =
        matches!(rc, RunConcept::ZoneRead | RunConcept::QbPower | RunConcept::SpeedOption | RunConcept::TripleOption);
    let own = match rc {
        RunConcept::Iso => Personnel::P21,
        RunConcept::Sneak => Personnel::P12,
        _ => Personnel::P11,
    };
    let mut f =
        Formation::plain(pers.unwrap_or(own), if gun { Backfield::Shotgun } else { Backfield::UnderCenter }, 1.0);
    let mut dir = 1.0;
    if rc == RunConcept::JetSweep {
        f.motion = Motion::Jet;
        dir = -1.0;
    }
    OffCall::new(f, PlayKind::Run(rc), dir, false)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("roster") {
        let draws = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20_000);
        roster(draws);
        return;
    }
    let n: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let t = teams(11);
    let (a, b, b34) = (&t.off, &t.def, &t.def34);
    // Each coverage with four rushers and with a linebacker blitz (Cover 0
    // always sends the double-A pressure).
    let mut looks = Vec::new();
    for c in Coverage::ALL {
        if c == Coverage::Cover0 {
            looks.push((c, Pressure::DoubleA));
        } else {
            looks.push((c, Pressure::Four));
            looks.push((c, Pressure::LbBlitz));
        }
    }
    // Runs against each front (plays and tactics topic 1), from Cover 3
    // with four rushers.
    let fronts = [FrontLook::Over, FrontLook::Under, FrontLook::Wide, FrontLook::Odd, FrontLook::Tite, FrontLook::Bear];
    let runs: Vec<RunConcept> = RUN_CONCEPTS.iter().copied().filter(|&r| r != RunConcept::Sneak).collect();
    let mut plays: Vec<(String, PlayKind, bool)> =
        PASS_CONCEPTS.iter().enumerate().map(|(i, p)| (p.name.to_string(), PlayKind::Pass(i), false)).collect();
    plays.extend(
        PASS_CONCEPTS
            .iter()
            .enumerate()
            .filter(|(_, p)| takes_fake(p))
            .map(|(i, p)| (format!("play-action {}", p.name), PlayKind::Pass(i), true)),
    );
    // AFM_CELL=concept:Coverage prints what happens on one cell (debug).
    if let Ok(cell) = std::env::var("AFM_CELL") {
        let (pn, cn) = cell.split_once(':').unwrap();
        let i = PASS_CONCEPTS.iter().position(|p| p.name == pn).unwrap();
        let cov = *Coverage::ALL.iter().find(|c| format!("{c:?}") == cn).unwrap();
        let call =
            OffCall::new(Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0), PlayKind::Pass(i), 1.0, false);
        let dcall = DefCall::new(DefPersonnel::Nickel, cov, Pressure::Four);
        let mut hist = std::collections::BTreeMap::new();
        let (mut comp, mut air, mut yac, mut sep, mut ttt) = (0.0, 0.0, 0.0, 0.0, 0.0);
        // Completions and attempts by time to throw (half-second buckets).
        let mut by_t = [(0u32, 0u32, 0.0f64); 16];
        for s in 0..n {
            let r = trace_snap_opt(a, b, LEVEL, call, dcall, 35.0, 1 + s, false);
            *hist.entry(format!("{:?}{}", r.kind, if r.interception { "-INT" } else { "" })).or_insert(0) += 1;
            if r.complete {
                comp += 1.0;
                air += r.air_yards;
                yac += r.yac;
            }
            sep += r.separation;
            ttt += r.time_to_throw;
            let b = ((r.time_to_throw / 0.5) as usize).min(15);
            by_t[b].1 += 1;
            by_t[b].0 += r.complete as u32;
            by_t[b].2 += r.yards;
        }
        let nf = n as f64;
        for (b, (c, n, y)) in by_t.iter().enumerate() {
            if *n > 0 {
                println!(
                    "  ttt {:.1}-{:.1}s: {n} att, {c} comp, {:.1} yds/att",
                    b as f64 * 0.5,
                    b as f64 * 0.5 + 0.5,
                    y / *n as f64
                );
            }
        }
        println!(
            "{hist:?} comp {:.3} air/comp {:.1} yac/comp {:.1} sep {:.2} ttt {:.2}",
            comp / nf,
            air / comp,
            yac / comp,
            sep / nf,
            ttt / nf
        );
        return;
    }
    let blank_fronts = vec![String::new(); fronts.len()];
    let mut rows: Vec<String> = plays
        .par_iter()
        .map(|(name, kind, fake)| {
            let mut call = OffCall::new(Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0), *kind, 1.0, false);
            call.fake = *fake;
            let cells: Vec<String> = looks
                .iter()
                .map(|&(coverage, pressure)| {
                    let dcall = DefCall::new(DefPersonnel::Nickel, coverage, pressure);
                    let mut v = 0.0;
                    for s in 0..n {
                        v += value(&trace_snap_opt(a, b, LEVEL, call, dcall, 35.0, 1 + s, false));
                    }
                    format!("{:.2}", v / n as f64)
                })
                .collect();
            format!("{name},{},{}", cells.join(","), blank_fronts.join(","))
        })
        .collect();
    let blank_looks = vec![String::new(); looks.len()];
    rows.extend(
        runs.par_iter()
            .map(|&rc| {
                let call = run_call(rc, None);
                let cells: Vec<String> = fronts
                    .iter()
                    .map(|&front| {
                        let mut dcall = DefCall::new(DefPersonnel::Base, Coverage::Cover3, Pressure::Four);
                        dcall.front = front;
                        let d = if matches!(front, FrontLook::Odd | FrontLook::Tite) { b34 } else { b };
                        let mut v = 0.0;
                        for s in 0..n {
                            v += value(&trace_snap_opt(a, d, LEVEL, call, dcall, 35.0, 1 + s, false));
                        }
                        format!("{:.2}", v / n as f64)
                    })
                    .collect();
                format!("{rc:?},{},{}", blank_looks.join(","), cells.join(","))
            })
            .collect::<Vec<_>>(),
    );
    let mut head: Vec<String> = looks.iter().map(|(c, p)| format!("{c:?}/{p:?}")).collect();
    head.extend(fronts.iter().map(|f| format!("Front:{f:?}")));
    println!("concept,{}", head.join(","));
    for r in rows {
        println!("{r}");
    }
}

// ------------------------------------------------------------------ the roster's weights

/// The features the table watches at each slot: the ratings the engine
/// reads for the man's work there, and his weight.
fn watched(s: Slot) -> Vec<u8> {
    use Attr as A;
    let v: &[Attr] = match s {
        Slot::Qb => &[
            A::ArmStrength,
            A::ShortAccuracy,
            A::MediumAccuracy,
            A::DeepAccuracy,
            A::Touch,
            A::ThrowOnMove,
            A::QuickRelease,
            A::PocketPresence,
            A::Decisions,
            A::Anticipation,
            A::EyeManipulation,
            A::BallHandling,
            A::Speed,
            A::Acceleration,
            A::Elusiveness,
            A::BallCarrierVision,
        ],
        Slot::Rb => &[
            A::Speed,
            A::Acceleration,
            A::Agility,
            A::Elusiveness,
            A::BallCarrierVision,
            A::LegDrive,
            A::Strength,
            A::Balance,
            A::BallSecurity,
            A::Catching,
            A::RouteRunning,
            A::PassBlocking,
        ],
        Slot::Fb => &[A::RunBlocking, A::BlockingInSpace, A::Strength, A::HandUse, A::LegDrive],
        Slot::Wr1 | Slot::Wr2 | Slot::Wr3 => &[
            A::Speed,
            A::Acceleration,
            A::Agility,
            A::RouteRunning,
            A::Release,
            A::Catching,
            A::ContestedCatch,
            A::BallTracking,
            A::BodyControl,
            A::Elusiveness,
            A::BlockingInSpace,
        ],
        Slot::Te1 | Slot::Te2 | Slot::Te3 => &[
            A::Speed,
            A::RouteRunning,
            A::Release,
            A::Catching,
            A::ContestedCatch,
            A::RunBlocking,
            A::PassBlocking,
            A::BlockingInSpace,
            A::Strength,
        ],
        Slot::Ot1 | Slot::Ot2 | Slot::Iol1 | Slot::Iol2 | Slot::Iol3 => {
            &[A::PassBlocking, A::RunBlocking, A::HandUse, A::Anchor, A::Strength, A::Agility, A::BlockingInSpace]
        }
        Slot::Edge1 | Slot::Edge2 => &[
            A::PowerRush,
            A::FinesseRush,
            A::BlockShedding,
            A::HandUse,
            A::Speed,
            A::Acceleration,
            A::Strength,
            A::Pursuit,
            A::Tackling,
        ],
        Slot::Dt1 | Slot::Dt2 => &[
            A::PowerRush,
            A::FinesseRush,
            A::BlockShedding,
            A::HandUse,
            A::Strength,
            A::Acceleration,
            A::Pursuit,
            A::Tackling,
        ],
        Slot::Sam | Slot::Mike | Slot::Will => &[
            A::Tackling,
            A::Pursuit,
            A::PlayRecognition,
            A::ZoneCoverage,
            A::ManCoverage,
            A::Speed,
            A::BlockShedding,
            A::Blitzing,
            A::Anticipation,
        ],
        Slot::Cb1 | Slot::Cb2 | Slot::Nb => &[
            A::ManCoverage,
            A::ZoneCoverage,
            A::Press,
            A::Speed,
            A::Acceleration,
            A::Agility,
            A::BallTracking,
            A::Anticipation,
            A::Tackling,
        ],
        Slot::Fs | Slot::Ss => &[
            A::ZoneCoverage,
            A::ManCoverage,
            A::Speed,
            A::FootballIq,
            A::Anticipation,
            A::BallTracking,
            A::Tackling,
            A::Pursuit,
            A::PlayRecognition,
        ],
    };
    let mut f: Vec<u8> = v.iter().map(|&a| a.index() as u8).collect();
    f.push(WEIGHT);
    f
}

/// A small stream of its own for each draw (the snaps keep the engine's).
struct Draw(u64);

impl Draw {
    fn u(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }

    fn chance(&mut self, p: f64) -> bool {
        self.u() < p
    }

    fn pick<T: Copy>(&mut self, v: &[T]) -> T {
        v[((self.u() * v.len() as f64) as usize).min(v.len() - 1)]
    }

    fn weighted(&mut self, w: &[f64]) -> usize {
        let tot: f64 = w.iter().sum();
        let mut x = self.u() * tot;
        for (k, &v) in w.iter().enumerate() {
            if x < v {
                return k;
            }
            x -= v;
        }
        w.len() - 1
    }
}

/// The league's coverages (`calling::league_mix`: the four shells' mixes,
/// each weighed alike).
fn league_coverages() -> Vec<f64> {
    let mut m = vec![0.0; COVERAGE_COUNT];
    for shell in [Shell::Cover3, Shell::Quarters, Shell::ManPressure, Shell::Cover2] {
        let mix = shell_mix(shell);
        let tot: f64 = mix.iter().map(|x| x.1).sum();
        for (cov, w) in mix {
            m[cov as usize] += w / tot / 4.0;
        }
    }
    m
}

/// The share of NFL snaps with five or more rushers outside Cover 0 the
/// offense's cells see (real 2021-25 about 30%, as the look table's
/// blitz columns stand).
const BLITZ: f64 = 0.3;
/// Real NFL: 11 personnel met with nickel 88% of the time (`Staff::nickel`).
const NICKEL_11: f64 = 0.88;
/// The run share the defense's cells meet three receivers with (real 2024
/// NFL 11 personnel, as `calling` reads it).
const RUN_11: f64 = 0.35;
/// How far a draw moves each rating (up to this many points either way:
/// about the spread of the level's starters and backups, so a slope is the
/// average over the men a staff has) and his weight (this share of it
/// either way).
const SPREAD: f64 = 18.0;
const WEIGHT_SPREAD: f64 = 0.08;

/// The snaps of a draw: the looks drawn from the league's.
struct Looks {
    cov: Vec<f64>,
    straight: Vec<usize>,
    runs: Vec<RunConcept>,
}

impl Looks {
    fn dcall(&self, r: &mut Draw, personnel: DefPersonnel) -> DefCall {
        let cov = Coverage::ALL[r.weighted(&self.cov)];
        let pressure = if cov == Coverage::Cover0 {
            Pressure::DoubleA
        } else if r.chance(BLITZ) {
            Pressure::LbBlitz
        } else {
            Pressure::Four
        };
        DefCall::new(personnel, cov, pressure)
    }

    /// A front for a run, alike among the six, and the staff that plays it.
    fn front<'a>(&self, r: &mut Draw, t: &'a Teams, dcall: &mut DefCall) -> &'a GameTeam {
        dcall.front = r.pick(&[
            FrontLook::Over,
            FrontLook::Under,
            FrontLook::Wide,
            FrontLook::Odd,
            FrontLook::Tite,
            FrontLook::Bear,
        ]);
        if matches!(dcall.front, FrontLook::Odd | FrontLook::Tite) { &t.def34 } else { &t.def }
    }

    fn vs(&self, r: &mut Draw, pers: Personnel) -> DefPersonnel {
        if matches!(pers, Personnel::P10 | Personnel::P11) && r.chance(NICKEL_11) {
            DefPersonnel::Nickel
        } else {
            DefPersonnel::Base
        }
    }
}

/// One snap of an offensive cell with the offense `off` (its men moved
/// this draw) against the typical defense.
fn off_snap(cell: Cell, off: &GameTeam, t: &Teams, lk: &Looks, r: &mut Draw, seed: u64) -> f64 {
    match cell {
        Cell::Pass(i, fake) => {
            let mut call =
                OffCall::new(Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0), PlayKind::Pass(i), 1.0, false);
            call.fake = fake;
            let pers = lk.vs(r, Personnel::P11);
            let dcall = lk.dcall(r, pers);
            value(&trace_snap_opt(off, &t.def, LEVEL, call, dcall, 35.0, seed, false))
        }
        Cell::Run(k) => {
            let rc = RUN_CONCEPTS[k];
            let call = run_call(rc, None);
            let pers = lk.vs(r, call.formation.personnel);
            let mut dcall = lk.dcall(r, pers);
            let d = lk.front(r, t, &mut dcall);
            // The sneak is a short-yardage call: third and one.
            let (down, togo) = if rc == RunConcept::Sneak { (3, 1.0) } else { (1, 10.0) };
            value(&trace_snap_at(off, d, LEVEL, call, dcall, 35.0, down, togo, seed))
        }
        Cell::Group(g, pass) => {
            let pers = roster_fit::GROUPS[g];
            let dp = lk.vs(r, pers);
            let mut dcall = lk.dcall(r, dp);
            if pass {
                let i = r.pick(&lk.straight);
                let call = OffCall::new(Formation::plain(pers, Backfield::Shotgun, 1.0), PlayKind::Pass(i), 1.0, false);
                value(&trace_snap_opt(off, &t.def, LEVEL, call, dcall, 35.0, seed, false))
            } else {
                let rc = r.pick(&lk.runs);
                let dir = if r.chance(0.5) { 1.0 } else { -1.0 };
                let call =
                    OffCall::new(Formation::plain(pers, Backfield::UnderCenter, 1.0), PlayKind::Run(rc), dir, false);
                let d = lk.front(r, t, &mut dcall);
                value(&trace_snap_opt(off, d, LEVEL, call, dcall, 35.0, seed, false))
            }
        }
        Cell::Sit(short) => {
            // Third and short: a power call from two backs or two tight
            // ends; third and long: a dropback from three receivers. The
            // snap's worth is whether it moves the chains.
            let (call, down, togo, d, dcall) = if short {
                let pers = if r.chance(0.5) { Personnel::P21 } else { Personnel::P12 };
                let rc = r.pick(&[RunConcept::Power, RunConcept::InsideZone, RunConcept::Duo, RunConcept::Iso]);
                let rc = if rc == RunConcept::Iso && pers != Personnel::P21 { RunConcept::Power } else { rc };
                let dir = if r.chance(0.5) { 1.0 } else { -1.0 };
                let call =
                    OffCall::new(Formation::plain(pers, Backfield::UnderCenter, 1.0), PlayKind::Run(rc), dir, false);
                let mut dcall = lk.dcall(r, DefPersonnel::Base);
                let d = lk.front(r, t, &mut dcall);
                (call, 3, if r.chance(0.5) { 1.0 } else { 2.0 }, d, dcall)
            } else {
                let i = r.pick(&lk.straight);
                let call = OffCall::new(
                    Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0),
                    PlayKind::Pass(i),
                    1.0,
                    false,
                );
                let pers = lk.vs(r, Personnel::P11);
                (call, 3, 5.0 + (r.u() * 5.0).floor(), &t.def, lk.dcall(r, pers))
            };
            let p = trace_snap_at(off, d, LEVEL, call, dcall, 35.0, down, togo, seed);
            ((p.first_down || p.touchdown) && !p.turnover) as u8 as f64
        }
        Cell::Look(_) | Cell::DPers(..) => unreachable!("a defense's cell"),
    }
}

/// One snap of a defensive cell with the defense `def` (and its 3-4 twin)
/// moved this draw, against the typical offense: the yards the offense
/// gains, three receivers' runs at their real share.
fn def_snap(cell: Cell, def: &GameTeam, def34: &GameTeam, t: &Teams, lk: &Looks, r: &mut Draw, seed: u64) -> f64 {
    let (personnel, look) = match cell {
        Cell::Look(c) => (DefPersonnel::Nickel, Some(roster_fit::look_of(c))),
        Cell::DPers(nickel, _) => (if nickel { DefPersonnel::Nickel } else { DefPersonnel::Base }, None),
        _ => unreachable!("an offense's cell"),
    };
    let mut dcall = match look {
        Some((cov, p)) => DefCall::new(personnel, cov, p),
        None => lk.dcall(r, personnel),
    };
    let run = match cell {
        Cell::DPers(_, pass) => !pass,
        _ => r.chance(RUN_11),
    };
    if run {
        let rc = r.pick(&lk.runs);
        let call = run_call(rc, Some(Personnel::P11));
        dcall.front = r.pick(&[
            FrontLook::Over,
            FrontLook::Under,
            FrontLook::Wide,
            FrontLook::Odd,
            FrontLook::Tite,
            FrontLook::Bear,
        ]);
        let d = if matches!(dcall.front, FrontLook::Odd | FrontLook::Tite) { def34 } else { def };
        value(&trace_snap_opt(&t.off, d, LEVEL, call, dcall, 35.0, seed, false))
    } else {
        let i = r.pick(&lk.straight);
        let call =
            OffCall::new(Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0), PlayKind::Pass(i), 1.0, false);
        value(&trace_snap_opt(&t.off, def, LEVEL, call, dcall, 35.0, seed, false))
    }
}

/// Moves every watched man on one side at random about his typical
/// man's numbers: returns the moved team and each pair's deviation.
fn moved(
    team: &GameTeam,
    people: &[Player],
    at: &[usize; SLOTS],
    pairs: &[(Slot, u8)],
    r: &mut Draw,
) -> (GameTeam, Vec<f64>) {
    let mut t = team.clone();
    for s in Slot::ALL {
        let mine: Vec<u8> = pairs.iter().filter(|p| p.0 == s).map(|p| p.1).collect();
        if mine.is_empty() {
            continue;
        }
        let k = at[s as usize];
        let ks: Vec<Knob> = mine
            .iter()
            .map(|&f| {
                if f == WEIGHT {
                    let w = roster_fit::reference(LEVEL, s, WEIGHT);
                    Knob::Set(Part::Weight, w * (1.0 + WEIGHT_SPREAD * (2.0 * r.u() - 1.0)))
                } else {
                    Knob::Rating(Attr::ALL[f as usize], SPREAD * (2.0 * r.u() - 1.0))
                }
            })
            .collect();
        change_one(&mut t.players[k], &people[k], &ks, LEVEL);
    }
    let dev = pairs.iter().map(|&(s, f)| roster_fit::deviation(LEVEL, s, &t.players[at[s as usize]], f)).collect();
    (t, dev)
}

fn roster(draws: usize) {
    let t = teams(11);
    let cells = roster_fit::all_cells();
    let fake_ok = |c: &Cell| !matches!(*c, Cell::Pass(i, true) if !takes_fake(&PASS_CONCEPTS[i]));
    let off_cells: Vec<Cell> = cells
        .iter()
        .copied()
        .filter(|c| matches!(c, Cell::Pass(..) | Cell::Run(_) | Cell::Group(..) | Cell::Sit(_)))
        .filter(fake_ok)
        .collect();
    let def_cells: Vec<Cell> = cells.iter().copied().filter(|c| matches!(c, Cell::Look(_) | Cell::DPers(..))).collect();
    let lk = Looks {
        cov: league_coverages(),
        straight: (0..PASS_CONCEPTS.len()).filter(|&i| takes_fake(&PASS_CONCEPTS[i])).collect(),
        runs: vec![
            RunConcept::InsideZone,
            RunConcept::OutsideZone,
            RunConcept::Power,
            RunConcept::Counter,
            RunConcept::Duo,
            RunConcept::Toss,
        ],
    };
    let pairs = |off: bool| -> Vec<(Slot, u8)> {
        Slot::ALL
            .iter()
            .filter(|s| s.offense() == off)
            .flat_map(|&s| watched(s).into_iter().map(move |f| (s, f)))
            .collect()
    };
    let started = std::time::Instant::now();
    let mut out = vec!["cell,slot,feature,weight".to_string()];
    for off in [true, false] {
        let pairs = pairs(off);
        let list = if off { &off_cells } else { &def_cells };
        let rows: Vec<(Vec<f64>, Vec<f64>)> = (0..draws)
            .into_par_iter()
            .map(|d| {
                let mut r = Draw(0xC0FF_EE00 ^ ((d as u64) << 1 | off as u64).wrapping_mul(0x2545_F491_4F6C_DD1D));
                let seed0 = 1 + d as u64 * 4096 + if off { 0 } else { 2048 };
                if off {
                    let (team, dev) = moved(&t.off, &t.off_people, &t.at, &pairs, &mut r);
                    let y = list
                        .iter()
                        .enumerate()
                        .map(|(k, &c)| off_snap(c, &team, &t, &lk, &mut r, seed0 + k as u64))
                        .collect();
                    (dev, y)
                } else {
                    let (def, dev) = moved(&t.def, &t.def_people, &t.at, &pairs, &mut r);
                    let mut def34 = def.clone();
                    def34.staff.front = Front::ThreeFour;
                    let y = list
                        .iter()
                        .enumerate()
                        .map(|(k, &c)| def_snap(c, &def, &def34, &t, &lk, &mut r, seed0 + k as u64))
                        .collect();
                    (dev, y)
                }
            })
            .collect();
        eprintln!(
            "{}: {draws} draws, {} cells, {} pairs, {:.0} s",
            if off { "offense" } else { "defense" },
            list.len(),
            pairs.len(),
            started.elapsed().as_secs_f64()
        );
        let fits = regress(&rows, list.len());
        for (cell, w) in list.iter().zip(shrink(list, &fits)) {
            for (j, &(s, f)) in pairs.iter().enumerate() {
                if w[j] != 0.0 {
                    out.push(format!("{},{},{},{:.5}", cell.name(), s.name(), roster_fit::feature_name(f), w[j]));
                }
            }
        }
        if std::env::var("AFM_ROSTER_RAW").is_ok() {
            for (cell, (b, se)) in list.iter().zip(&fits) {
                for (j, &(s, f)) in pairs.iter().enumerate() {
                    eprintln!(
                        "raw,{},{},{},{:.5},{:.5}",
                        cell.name(),
                        s.name(),
                        roster_fit::feature_name(f),
                        b[j],
                        se[j]
                    );
                }
            }
        }
    }
    for l in out {
        println!("{l}");
    }
}

/// Each cell's slopes and their standard errors: least squares of the
/// cell's values on every pair's deviation (and a constant) over the
/// draws, all cells on the one design.
fn regress(rows: &[(Vec<f64>, Vec<f64>)], cells: usize) -> Vec<(Vec<f64>, Vec<f64>)> {
    let p = rows[0].0.len() + 1;
    let n = rows.len();
    let mut xtx = vec![0.0; p * p];
    let mut xty = vec![vec![0.0; p]; cells];
    let mut yy = vec![0.0; cells];
    for (x, y) in rows {
        let xr: Vec<f64> = std::iter::once(1.0).chain(x.iter().copied()).collect();
        for a in 0..p {
            for b in a..p {
                xtx[a * p + b] += xr[a] * xr[b];
            }
        }
        for c in 0..cells {
            for a in 0..p {
                xty[c][a] += xr[a] * y[c];
            }
            yy[c] += y[c] * y[c];
        }
    }
    for a in 0..p {
        for b in 0..a {
            xtx[a * p + b] = xtx[b * p + a];
        }
    }
    let inv = invert(&xtx, p);
    (0..cells)
        .map(|c| {
            let beta: Vec<f64> = (0..p).map(|a| (0..p).map(|b| inv[a * p + b] * xty[c][b]).sum()).collect();
            let fit: f64 = (0..p).map(|a| beta[a] * xty[c][a]).sum();
            let s2 = ((yy[c] - fit) / (n - p) as f64).max(0.0);
            let se: Vec<f64> = (1..p).map(|a| (s2 * inv[a * p + a]).sqrt()).collect();
            (beta[1..].to_vec(), se)
        })
        .collect()
}

/// Gauss-Jordan inverse of a symmetric positive matrix.
fn invert(m: &[f64], p: usize) -> Vec<f64> {
    let mut a = m.to_vec();
    let mut inv = vec![0.0; p * p];
    for i in 0..p {
        inv[i * p + i] = 1.0;
    }
    for col in 0..p {
        let piv = (col..p).max_by(|&x, &y| a[x * p + col].abs().total_cmp(&a[y * p + col].abs())).unwrap();
        if piv != col {
            for k in 0..p {
                a.swap(col * p + k, piv * p + k);
                inv.swap(col * p + k, piv * p + k);
            }
        }
        let d = a[col * p + col];
        for k in 0..p {
            a[col * p + k] /= d;
            inv[col * p + k] /= d;
        }
        for row in 0..p {
            if row != col {
                let f = a[row * p + col];
                if f != 0.0 {
                    for k in 0..p {
                        a[row * p + k] -= f * a[col * p + k];
                        inv[row * p + k] -= f * inv[col * p + k];
                    }
                }
            }
        }
    }
    inv
}

/// A cell's kind: its weights are pooled with the other cells of its kind.
fn family(c: Cell) -> u8 {
    match c {
        Cell::Pass(_, false) => 0,
        Cell::Pass(_, true) => 1,
        Cell::Run(_) => 2,
        Cell::Group(_, false) => 3,
        Cell::Group(_, true) => 4,
        Cell::Sit(true) => 5,
        Cell::Sit(false) => 6,
        Cell::Look(_) => 7,
        Cell::DPers(_, false) => 8,
        Cell::DPers(_, true) => 9,
    }
}

/// How far past its noise a weight must stand to be kept (standard errors).
const KEEP: f64 = 2.5;

/// The weights kept: for each pair, a kind's cells share one weight unless
/// they differ past their noise (a chi-square test at 5%), and then each
/// cell's own is pulled toward the kind's by its noise (empirical Bayes,
/// DerSimonian-Laird); a weight is kept only as far as it stands past
/// `KEEP` of its noise (a soft threshold), so a pair the engine doesn't
/// reward reads nothing.
// `j` is the pair in every member cell's fit and in `out[k]`, with `k`
// running over the kind's cells, so no one slice's iterator stands in for it.
#[allow(clippy::needless_range_loop)]
fn shrink(cells: &[Cell], fits: &[(Vec<f64>, Vec<f64>)]) -> Vec<Vec<f64>> {
    let pairs = fits[0].0.len();
    let mut out = vec![vec![0.0; pairs]; cells.len()];
    let mut kinds: Vec<u8> = cells.iter().map(|&c| family(c)).collect();
    kinds.sort_unstable();
    kinds.dedup();
    for kind in kinds {
        let members: Vec<usize> = (0..cells.len()).filter(|&k| family(cells[k]) == kind).collect();
        for j in 0..pairs {
            let b: Vec<f64> = members.iter().map(|&k| fits[k].0[j]).collect();
            let v: Vec<f64> = members.iter().map(|&k| fits[k].1[j].powi(2).max(1e-12)).collect();
            let w: Vec<f64> = v.iter().map(|x| 1.0 / x).collect();
            let sw: f64 = w.iter().sum();
            let m = b.iter().zip(&w).map(|(b, w)| b * w).sum::<f64>() / sw;
            let q: f64 = b.iter().zip(&w).map(|(b, w)| w * (b - m).powi(2)).sum();
            let df = members.len().saturating_sub(1) as f64;
            let tau2 = if df > 0.0 && q > chi2_95(df) {
                let sw2: f64 = w.iter().map(|w| w * w).sum();
                ((q - df) / (sw - sw2 / sw)).max(0.0)
            } else {
                0.0
            };
            let ws: Vec<f64> = v.iter().map(|v| 1.0 / (v + tau2)).collect();
            let sws: f64 = ws.iter().sum();
            let ms = b.iter().zip(&ws).map(|(b, w)| b * w).sum::<f64>() / sws;
            let se_m = (1.0 / sws).sqrt();
            for (i, &k) in members.iter().enumerate() {
                let pull = if tau2 > 0.0 { tau2 / (tau2 + v[i]) } else { 0.0 };
                let post = ms + pull * (b[i] - ms);
                let sd = (pull * v[i] + (1.0 - pull).powi(2) * se_m * se_m).sqrt();
                out[k][j] = post.signum() * (post.abs() - KEEP * sd).max(0.0);
            }
        }
    }
    out
}

/// The chi-square's 95th percentile at `df` degrees of freedom
/// (Wilson-Hilferty).
fn chi2_95(df: f64) -> f64 {
    let z = 1.6449;
    let a = 2.0 / (9.0 * df);
    df * (1.0 - a + z * a.sqrt()).powi(3)
}
