//! Coordinators call the game (topics 3 and 4.2). They start from how real
//! teams call plays by situation (nflverse), bend it by scheme and
//! aggressiveness, learn what works during the game, and pick concepts that
//! beat what the other side keeps showing. How sharp all of it is comes
//! from the coach's hidden ability.

use super::playbook::*;
use super::roster::{Caller, OffScheme, Shell, Staff};
use super::tables::{self, Clock};
use crate::rng::{Rng, clamp};

/// What a coordinator has learned about this game.
#[derive(Clone, Debug, Default)]
pub struct Memory {
    pub run_epa: f64,
    pub runs: f64,
    pub pass_epa: f64,
    pub passes: f64,
    pub deep_epa: f64,
    pub deeps: f64,
    /// What the other defense has shown.
    pub blitzes: f64,
    /// Snaps of each coverage, in `Coverage` order.
    pub cov_seen: [f64; 8],
    pub snaps_seen: f64,
    /// What the other offense has done.
    pub opp_pass: f64,
    pub opp_plays: f64,
    pub opp_deep_epa: f64,
    pub opp_run_epa: f64,
    pub opp_runs: f64,
    pub opp_quick: f64,
    /// Pass concepts the other offense has run, in `PASS_CONCEPTS` order.
    pub opp_concepts: [f64; 16],
}

impl Memory {
    pub fn saw_defense(&mut self, d: &DefCall) {
        self.snaps_seen += 1.0;
        self.blitzes += (d.pressure.extra() > 0) as u8 as f64;
        self.cov_seen[d.coverage as usize] += 1.0;
    }
    /// Game prep (topic 3.2): film of the opponent's past games. A sharp
    /// coordinator starts the game already knowing the other side's habits,
    /// as if he had seen that many snaps of them (proposed, not confirmed).
    pub fn prepare(&mut self, opp: &Staff, own: &Staff) {
        let film = 20.0;
        let n = film * own.offense.skill / 20.0;
        if n <= 0.0 {
            return;
        }
        let mix = shell_mix(opp.shell);
        let tot: f64 = mix.iter().map(|x| x.1).sum();
        for &(cov, w) in &mix {
            self.cov_seen[cov as usize] += n * w / tot;
        }
        self.blitzes += n * clamp(0.24 + 0.012 * (opp.defense.aggressiveness - 10.0), 0.05, 0.6);
        self.snaps_seen += n;
        let nd = film * own.defense.skill / 20.0;
        self.opp_pass += nd * (0.58 + opp.off_scheme.pass_lean());
        self.opp_plays += nd;
    }

    fn rate(&self, x: f64) -> f64 {
        (x + 1.0) / (self.snaps_seen + 4.0)
    }
}

/// The game situation a call is made in.
#[derive(Clone, Copy, Debug)]
pub struct Situation {
    pub down: u8,
    pub togo: f64,
    pub yardline_100: f64,
    pub score_diff: i32,
    pub quarter: u8,
    /// Seconds left in the half and in the game.
    pub half_left: f64,
    pub game_left: f64,
    pub timeouts: u8,
    pub opp_timeouts: u8,
}

impl Situation {
    pub fn clock(&self) -> Clock {
        if self.half_left <= 120.0 {
            Clock::TwoMinute
        } else if self.quarter >= 4 && self.half_left <= 420.0 {
            Clock::Late
        } else {
            Clock::Normal
        }
    }
    /// The offense needs to stop the clock.
    pub fn hurry(&self) -> bool {
        (self.half_left <= 120.0 && self.score_diff <= 0 && self.quarter >= 2)
            || (self.quarter >= 4 && self.half_left <= 300.0 && self.score_diff < 0)
            || (self.quarter == 2 && self.half_left <= 120.0)
    }
}

fn weigh<T: Copy>(rng: &mut Rng, items: &[(T, f64)]) -> T {
    let pos: Vec<(T, f64)> = items.iter().copied().filter(|x| x.1 > 0.0).collect();
    rng.weighted(&pos)
}

/// The offensive call.
pub fn call_offense(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    has_fb: bool,
    te_count: usize,
    rng: &mut Rng,
) -> OffCall {
    let c = &staff.offense;
    let scheme = staff.off_scheme;
    let mut p_pass = tables::real_pass_rate(sit.down, sit.togo, sit.score_diff, sit.clock());
    p_pass += scheme.pass_lean() + 0.006 * (c.aggressiveness - 10.0);
    // In-game learning (topic 3.2): lean toward what has worked, faster
    // and more accurately for a sharp coordinator.
    if mem.runs + mem.passes >= 8.0 {
        // Passing always averages more EPA than running (about +0.12 a play in
        // the NFL) and coaches still run 42%: only a gap beyond that moves them.
        let diff = mem.pass_epa / mem.passes.max(1.0) - mem.run_epa / mem.runs.max(1.0) - 0.12;
        let sharp = c.adaptation / 20.0;
        let noise = rng.normal(0.0, 0.25 * (1.0 - sharp));
        p_pass += clamp((diff + noise) * 0.22 * sharp, -0.12, 0.12);
    }
    if sit.hurry() {
        p_pass = p_pass.max(0.85);
    }
    if sit.togo <= 1.0 && sit.down >= 3 {
        p_pass = p_pass.min(0.45);
    }
    let p_pass = clamp(p_pass, 0.03, 0.97);
    let pass = rng.chance(p_pass);
    let strength = if rng.chance(0.5) { 1.0 } else { -1.0 };
    let short = sit.togo <= 2.0 && sit.down >= 3 || sit.yardline_100 <= 3.0;
    let hurry = sit.hurry();
    // Personnel and formation by scheme and situation.
    let personnel = if hurry {
        if rng.chance(0.7) { Personnel::P11 } else { Personnel::P10 }
    } else if short {
        weigh(
            rng,
            &[
                (Personnel::P12, 3.0),
                (Personnel::P22, if has_fb { 2.0 } else { 0.0 }),
                (Personnel::P13, if te_count >= 3 { 1.5 } else { 0.0 }),
                (Personnel::P21, if has_fb { 1.5 } else { 0.0 }),
            ],
        )
    } else {
        match scheme {
            OffScheme::AirRaid => weigh(rng, &[(Personnel::P10, 4.0), (Personnel::P11, 6.0)]),
            OffScheme::WestCoast => weigh(
                rng,
                &[(Personnel::P11, 6.0), (Personnel::P12, 2.5), (Personnel::P21, if has_fb { 1.5 } else { 0.0 })],
            ),
            OffScheme::WideZone => weigh(
                rng,
                &[(Personnel::P11, 5.0), (Personnel::P12, 2.5), (Personnel::P21, if has_fb { 2.5 } else { 0.0 })],
            ),
            OffScheme::PowerGap => weigh(
                rng,
                &[
                    (Personnel::P11, 4.0),
                    (Personnel::P12, 3.5),
                    (Personnel::P21, if has_fb { 2.0 } else { 0.0 }),
                    (Personnel::P22, if has_fb { 0.8 } else { 0.0 }),
                ],
            ),
            OffScheme::SpreadOption => weigh(rng, &[(Personnel::P11, 6.0), (Personnel::P10, 3.0)]),
            OffScheme::WingT => weigh(
                rng,
                &[
                    (Personnel::P21, if has_fb { 5.0 } else { 0.0 }),
                    (Personnel::P22, if has_fb { 2.0 } else { 0.0 }),
                    (Personnel::P12, 3.0),
                ],
            ),
        }
    };
    let personnel = if te_count < 2 && matches!(personnel, Personnel::P12 | Personnel::P22 | Personnel::P13) {
        Personnel::P11
    } else {
        personnel
    };
    let personnel = if te_count < 3 && personnel == Personnel::P13 { Personnel::P12 } else { personnel };
    let personnel =
        if !has_fb && matches!(personnel, Personnel::P21 | Personnel::P22) { Personnel::P11 } else { personnel };
    let gun_lean = match scheme {
        OffScheme::AirRaid | OffScheme::SpreadOption => 0.92,
        OffScheme::WestCoast => 0.6,
        OffScheme::WideZone => 0.35,
        OffScheme::PowerGap => 0.4,
        OffScheme::WingT => 0.05,
    };
    let gun = rng.chance(clamp(gun_lean + if pass { 0.2 } else { -0.15 } + if hurry { 0.3 } else { 0.0 }, 0.0, 0.98));
    let backfield = if matches!(personnel, Personnel::P21 | Personnel::P22) && !hurry {
        Backfield::UnderCenter
    } else if gun {
        Backfield::Shotgun
    } else if scheme == OffScheme::WideZone && rng.chance(0.3) {
        Backfield::Pistol
    } else {
        Backfield::UnderCenter
    };
    let trips = personnel.wide_receivers() >= 3 && rng.chance(0.35);
    let formation = Formation { personnel, backfield, trips, strength };
    if pass {
        let concept = pick_concept(staff, mem, sit, backfield, rng);
        return OffCall { formation, kind: PlayKind::Pass(concept), dir: strength, tempo: hurry };
    }
    let rc = pick_run(scheme, short, backfield, has_fb && matches!(personnel, Personnel::P21 | Personnel::P22), rng);
    let dir = if rc == RunConcept::Counter || rng.chance(0.3) { -strength } else { strength };
    OffCall { formation, kind: PlayKind::Run(rc), dir, tempo: hurry }
}

fn pick_run(scheme: OffScheme, short: bool, backfield: Backfield, fb: bool, rng: &mut Rng) -> RunConcept {
    use RunConcept::*;
    if short && rng.chance(0.25) && backfield == Backfield::UnderCenter {
        return Sneak;
    }
    let gun = backfield != Backfield::UnderCenter;
    let w: Vec<(RunConcept, f64)> = match scheme {
        OffScheme::WideZone => vec![(OutsideZone, 5.0), (InsideZone, 3.0), (Counter, 0.8), (Toss, 0.6), (Draw, 0.4)],
        OffScheme::PowerGap => vec![(Power, 4.0), (Counter, 2.5), (InsideZone, 2.5), (Toss, 0.6), (Draw, 0.6)],
        OffScheme::SpreadOption => {
            vec![(ZoneRead, if gun { 5.0 } else { 0.0 }), (InsideZone, 3.0), (Draw, 1.5), (Counter, 1.0), (Power, 1.0)]
        }
        OffScheme::WingT => vec![(BuckSweep, 4.0), (Counter, 2.5), (Power, 2.0), (Toss, 1.5), (InsideZone, 1.0)],
        OffScheme::WestCoast => vec![(InsideZone, 4.0), (OutsideZone, 2.0), (Draw, 1.2), (Power, 1.0), (Toss, 0.6)],
        OffScheme::AirRaid => {
            vec![(InsideZone, 4.0), (Draw, 2.0), (ZoneRead, if gun { 1.5 } else { 0.0 }), (Counter, 1.0)]
        }
    };
    let mut w = w;
    if fb {
        w.push((Power, 1.5));
    }
    if short {
        for x in w.iter_mut() {
            if matches!(x.0, Toss | Draw | OutsideZone) {
                x.1 *= 0.3;
            }
        }
    }
    weigh(rng, &w)
}

fn pick_concept(staff: &Staff, mem: &Memory, sit: &Situation, backfield: Backfield, rng: &mut Rng) -> usize {
    let c = &staff.offense;
    let scheme = staff.off_scheme;
    let long = sit.togo >= 8.0 && sit.down >= 2;
    let short = sit.togo <= 3.0;
    let red = sit.yardline_100 <= 20.0;
    let goal = sit.yardline_100 <= 8.0;
    let hurry = sit.hurry();
    // What the defense has been showing, as well as this coordinator can
    // read it, and what each concept is worth against that.
    let sharp = (c.skill + c.adaptation) / 40.0;
    let ev = expected_values(mem, sharp);
    let mean = ev.iter().sum::<f64>() / ev.len() as f64;
    let trust = 0.2 * sharp;
    let w: Vec<(usize, f64)> = PASS_CONCEPTS
        .iter()
        .enumerate()
        .map(|(i, pc)| {
            let mut w = 1.0;
            match pc.depth {
                PassDepth::Screen => w *= if long { 0.6 } else { 0.9 },
                PassDepth::Quick => {
                    w *= if short {
                        2.2
                    } else if long {
                        0.6
                    } else {
                        1.4
                    }
                }
                PassDepth::Medium => w *= if long { 1.6 } else { 1.0 },
                PassDepth::Deep => {
                    w *= if long {
                        1.3
                    } else if short {
                        0.5
                    } else {
                        0.8
                    }
                }
            }
            if pc.play_action {
                // Play action needs a run game to sell, and a set back.
                w *= if sit.down == 1 || short { 1.4 } else { 0.5 };
                if backfield == Backfield::Shotgun && pc.name == "play-action boot" {
                    w *= 0.5;
                }
            }
            if red {
                if pc.depth == PassDepth::Deep {
                    w *= 0.3;
                }
                if pc.name == "fades" || pc.name == "flood" || pc.name == "slant-flat" {
                    w *= 2.0;
                }
            } else if pc.name == "fades" {
                w *= 0.15;
            }
            if goal && pc.depth == PassDepth::Screen {
                w *= 0.3;
            }
            if hurry {
                if pc.play_action || pc.depth == PassDepth::Screen {
                    w *= 0.3;
                }
                if pc.name == "flood" || pc.name == "curl-flat" || pc.name == "smash" {
                    w *= 1.8;
                }
            }
            // The scheme's bread and butter.
            let fav: &[&str] = match scheme {
                OffScheme::AirRaid => &["mesh", "four verticals", "stick", "Y-cross", "hitches"],
                OffScheme::WestCoast => &["slant-flat", "curl-flat", "stick", "RB screen", "flood"],
                OffScheme::WideZone => &["play-action boot", "play-action shot", "play-action dig", "flood"],
                OffScheme::PowerGap => &["play-action shot", "curl-flat", "dagger", "smash"],
                OffScheme::SpreadOption => &["bubble screen", "hitches", "four verticals", "RB screen"],
                OffScheme::WingT => &["play-action boot", "play-action shot", "flood"],
            };
            if fav.contains(&pc.name) {
                w *= 2.2;
            }
            // What beats what (topic 9.1): the caller leans toward the
            // concepts that gain the most against the looks he expects.
            w *= (trust * (ev[i] - mean)).exp();
            if pc.depth == PassDepth::Deep && mem.deeps >= 3.0 {
                let e = mem.deep_epa / mem.deeps;
                w *= clamp(1.0 + e * 0.4 * c.adaptation / 20.0, 0.5, 1.6);
            }
            w *= 1.0 + 0.03 * (c.aggressiveness - 10.0) * if pc.depth == PassDepth::Deep { 1.0 } else { -0.3 };
            (i, w.max(0.01))
        })
        .collect();
    weigh(rng, &w)
}

/// Football knowledge: what each pass concept gains against each look,
/// measured on the match engine (`call_values` in afm-tools). Rows follow
/// `PASS_CONCEPTS`; columns are Cover 0 with the double-A blitz, then each
/// other coverage with four rushers and with a linebacker blitz.
const CALL_VALUES_CSV: &str = include_str!("../../data/call_values.csv");

fn call_values() -> &'static Vec<[f64; 15]> {
    static T: std::sync::OnceLock<Vec<[f64; 15]>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let mut rows = std::collections::HashMap::new();
        for line in CALL_VALUES_CSV.lines().skip(1) {
            let mut f = line.split(',');
            let name = f.next().unwrap_or_default();
            let mut v = [0.0; 15];
            for (x, s) in v.iter_mut().zip(f) {
                *x = s.trim().parse().unwrap_or(0.0);
            }
            rows.insert(name.to_string(), v);
        }
        PASS_CONCEPTS.iter().map(|p| rows.get(p.name).copied().unwrap_or([0.0; 15])).collect()
    })
}

/// The league's coverage mix, the starting belief before any film.
fn league_mix() -> [f64; 8] {
    let mut m = [0.0; 8];
    for shell in [Shell::Cover3, Shell::Quarters, Shell::ManPressure, Shell::Cover2] {
        let mix = shell_mix(shell);
        let tot: f64 = mix.iter().map(|x| x.1).sum();
        for (cov, w) in mix {
            m[cov as usize] += w / tot / 4.0;
        }
    }
    m
}

/// Expected value of each pass concept against the looks this caller
/// expects. A sharp caller trusts what he has seen; a poor one falls back
/// on the league's habits.
fn expected_values(mem: &Memory, sharp: f64) -> Vec<f64> {
    let prior = league_mix();
    let blitz = mem.rate(mem.blitzes) * sharp + 0.25 * (1.0 - sharp);
    let p: Vec<f64> = (0..8)
        .map(|k| sharp * (mem.cov_seen[k] + 4.0 * prior[k]) / (mem.snaps_seen + 4.0) + (1.0 - sharp) * prior[k])
        .collect();
    call_values()
        .iter()
        .map(|v| {
            let mut e = p[0] * v[0];
            for k in 1..8 {
                e += p[k] * ((1.0 - blitz) * v[2 * k - 1] + blitz * v[2 * k]);
            }
            e
        })
        .collect()
}

/// What each coverage gives up (four rushers) against the pass concepts a
/// defensive coordinator expects. A sharp one trusts what he has seen; a
/// poor one assumes every concept is as likely.
fn coverage_values(mem: &Memory, sharp: f64) -> [f64; 8] {
    let t = call_values();
    let seen: f64 = mem.opp_concepts.iter().sum();
    let n = t.len() as f64;
    let p: Vec<f64> =
        (0..t.len()).map(|i| sharp * (mem.opp_concepts[i] + 4.0 / n) / (seen + 4.0) + (1.0 - sharp) / n).collect();
    let mut out = [0.0; 8];
    for (k, o) in out.iter_mut().enumerate() {
        let col = if k == 0 { 0 } else { 2 * k - 1 };
        *o = t.iter().zip(&p).map(|(v, q)| q * v[col]).sum();
    }
    out
}

/// A defensive coordinator's family of coverages: how often he calls each.
pub fn shell_mix(shell: Shell) -> [(Coverage, f64); 8] {
    match shell {
        Shell::Cover3 => [
            (Coverage::Cover3, 42.0),
            (Coverage::Cover1, 20.0),
            (Coverage::Cover4, 10.0),
            (Coverage::Cover2, 9.0),
            (Coverage::Cover6, 6.0),
            (Coverage::Cover2Man, 4.0),
            (Coverage::Cover0, 4.0),
            (Coverage::Tampa2, 5.0),
        ],
        Shell::Quarters => [
            (Coverage::Cover3, 18.0),
            (Coverage::Cover1, 14.0),
            (Coverage::Cover4, 30.0),
            (Coverage::Cover2, 12.0),
            (Coverage::Cover6, 14.0),
            (Coverage::Cover2Man, 5.0),
            (Coverage::Cover0, 3.0),
            (Coverage::Tampa2, 4.0),
        ],
        Shell::ManPressure => [
            (Coverage::Cover3, 22.0),
            (Coverage::Cover1, 38.0),
            (Coverage::Cover4, 8.0),
            (Coverage::Cover2, 6.0),
            (Coverage::Cover6, 4.0),
            (Coverage::Cover2Man, 10.0),
            (Coverage::Cover0, 9.0),
            (Coverage::Tampa2, 3.0),
        ],
        Shell::Cover2 => [
            (Coverage::Cover3, 20.0),
            (Coverage::Cover1, 12.0),
            (Coverage::Cover4, 10.0),
            (Coverage::Cover2, 28.0),
            (Coverage::Cover6, 8.0),
            (Coverage::Cover2Man, 6.0),
            (Coverage::Cover0, 2.0),
            (Coverage::Tampa2, 14.0),
        ],
    }
}

/// The defensive call.
pub fn call_defense(staff: &Staff, mem: &Memory, sit: &Situation, off: &Formation, rng: &mut Rng) -> DefCall {
    let c: &Caller = &staff.defense;
    let wr = off.personnel.wide_receivers();
    let long = sit.togo >= 8.0 && sit.down >= 2;
    let short = sit.togo <= 2.0;
    let personnel = if sit.yardline_100 <= 3.0 && wr <= 2 {
        DefPersonnel::GoalLine
    } else if wr >= 4 || (long && sit.down == 3 && wr >= 3 && rng.chance(0.5)) {
        DefPersonnel::Dime
    } else if wr == 3 {
        DefPersonnel::Nickel
    } else {
        DefPersonnel::Base
    };
    // The coordinator's family of coverages, then the situation.
    let base = shell_mix(staff.shell);
    let sharp = c.adaptation / 20.0;
    let opp_pass = (mem.opp_pass + 2.0) / (mem.opp_plays + 4.0);
    let deep_hurting = if mem.opp_plays > 6.0 { (mem.opp_deep_epa / mem.opp_plays.max(1.0)).max(0.0) } else { 0.0 };
    let run_hurting = if mem.opp_runs > 5.0 { (mem.opp_run_epa / mem.opp_runs).max(0.0) } else { 0.0 };
    // What each coverage gives up against the passes this coordinator
    // expects (topic 9.1): a sharp one leans away from the coverages they
    // beat.
    let read = (c.skill + c.adaptation) / 40.0;
    let given = coverage_values(mem, read);
    let mean = given.iter().sum::<f64>() / 8.0;
    let trust = 0.2 * read;
    let w: Vec<(Coverage, f64)> = base
        .iter()
        .map(|&(cov, w)| {
            let mut w = w * (-trust * (given[cov as usize] - mean)).exp();
            if cov.high() == 2 {
                w *= if long {
                    1.5
                } else if short {
                    0.55
                } else {
                    1.0
                };
                w *= 1.0 + sharp * (deep_hurting * 3.0 + (opp_pass - 0.55).max(0.0) * 1.5);
                w *= 1.0 - (sharp * run_hurting * 2.0).min(0.5);
            } else {
                w *= 1.0 + sharp * run_hurting * 2.5;
            }
            if cov == Coverage::Cover0 {
                w *= if short || sit.yardline_100 <= 10.0 {
                    2.5
                } else if long {
                    0.3
                } else {
                    1.0
                };
            }
            if sit.hurry() && cov.high() == 2 {
                w *= 1.6;
            }
            if personnel == DefPersonnel::GoalLine && !cov.is_man() {
                w *= 0.3;
            }
            (cov, w)
        })
        .collect();
    let coverage = weigh(rng, &w);
    let mut blitz = 0.24 + 0.012 * (c.aggressiveness - 10.0);
    if sit.down == 3 && (4.0..=9.0).contains(&sit.togo) {
        blitz += 0.1;
    }
    if sit.hurry() {
        blitz -= 0.1;
    }
    blitz += sharp * (mem.opp_quick / mem.opp_plays.max(1.0) - 0.3).min(0.0) * 0.5;
    let pressure = if coverage == Coverage::Cover0 {
        Pressure::DoubleA
    } else if rng.chance(clamp(blitz, 0.05, 0.6)) {
        weigh(
            rng,
            &[
                (Pressure::LbBlitz, 4.0),
                (Pressure::DbBlitz, 2.0),
                (Pressure::DoubleA, 1.0),
                (Pressure::Simulated, if c.skill > 9.0 { 2.0 } else { 0.5 }),
            ],
        )
    } else {
        Pressure::Four
    };
    let pressure =
        if personnel == DefPersonnel::Dime && pressure == Pressure::DoubleA { Pressure::DbBlitz } else { pressure };
    let disguise = rng.chance(clamp(0.05 + 0.025 * c.skill, 0.05, 0.6));
    DefCall { personnel, coverage, pressure, disguise }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FourthDown {
    Go,
    Punt,
    FieldGoal,
}

/// Fourth down (topic 4.2): expected points of going, punting and kicking
/// from the real EP table, then the head coach's own lean. Sharp and
/// aggressive coaches follow the numbers; weaker ones stay too cautious.
pub fn fourth_down(staff: &Staff, sit: &Situation, fg_make: f64, rng: &mut Rng) -> FourthDown {
    let y = sit.yardline_100;
    let togo = sit.togo;
    // Real NFL fourth-down conversion by distance (about 70% on 4th and 1,
    // 50% on 4th and 5, 33% on 4th and 10).
    let conv = clamp(0.70 * (-0.085 * (togo - 1.0)).exp(), 0.12, 0.75);
    let ep = tables::expected_points;
    let after_score = 0.95;
    let go_ok = if togo >= y { 6.95 - after_score } else { ep(1, 10.0_f64.min(y - togo), y - togo) };
    let go_fail = -ep(1, 10.0, 100.0 - y);
    let ev_go = conv * go_ok + (1.0 - conv) * go_fail;
    let net = 41.0;
    // Where the opponent starts, as their distance to the goal.
    let land = y - net;
    let opp_punt = if land < 0.0 { 80.0 } else { 100.0 - land.max(8.0) };
    let ev_punt = -ep(1, 10.0, opp_punt.min(99.0));
    // A miss gives them the ball at the spot of the kick (or the 20).
    let opp_miss = (100.0 - y - 7.0).min(80.0);
    let ev_fg = fg_make * (3.0 - after_score) + (1.0 - fg_make) * -ep(1, 10.0, opp_miss.max(1.0));
    let mut ev_go_adj = ev_go;
    // The scoreboard late: a team that must score goes; a team ahead plays it safe.
    let late = sit.quarter >= 4 && sit.game_left < 300.0;
    if late && sit.score_diff < 0 {
        let need = -sit.score_diff;
        if need > 3 || sit.game_left < 120.0 {
            ev_go_adj += 1.2 + (300.0 - sit.game_left) / 100.0;
        }
    }
    if late && sit.score_diff > 0 {
        ev_go_adj -= 0.6;
    }
    let gm = staff.game_management;
    let agg = staff.offense.aggressiveness;
    // Caution: what a coach needs to see before he goes for it.
    let margin = 0.06 * (11.0 - agg) + 0.03 * (11.0 - gm) + 0.35;
    let noise = rng.normal(0.0, 0.5 * (1.3 - gm / 20.0));
    let best_kick =
        if ev_fg > ev_punt && fg_make > 0.3 { (FourthDown::FieldGoal, ev_fg) } else { (FourthDown::Punt, ev_punt) };
    // Real coaches still mostly kick: the real go rate as a habit.
    let habit = tables::real_fourth_go_rate(togo, y, sit.score_diff);
    let by_numbers = ev_go_adj + noise > best_kick.1 + margin;
    let follow = clamp(0.05 + 0.012 * gm + 0.01 * (agg - 10.0), 0.03, 0.45);
    let go = if rng.chance(follow) { by_numbers } else { rng.chance(habit) };
    if go || (late && sit.score_diff < -3 && sit.game_left < 150.0) { FourthDown::Go } else { best_kick.0 }
}

/// Go for two after a touchdown (the chart, plus the coach's lean and how
/// much he trusts his kicker).
pub fn go_for_two(staff: &Staff, diff_after_td: i32, quarter: u8, game_left: f64, xp_make: f64, rng: &mut Rng) -> bool {
    let late = quarter >= 4 && game_left < 900.0;
    let chart = late && matches!(diff_after_td, -2 | -5 | -10 | -13 | 1 | 4 | 5 | 12 | -16 | 8 | -9);
    let weak_kicker = xp_make < 0.75;
    let lean = 0.01 + 0.004 * (staff.offense.aggressiveness - 10.0);
    chart && rng.chance(0.8) || weak_kicker && rng.chance(0.55) || rng.chance(lean.max(0.0))
}
