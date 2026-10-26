//! Coordinators call the game (section 3 topics 3 and 4.2, plays and tactics
//! topics 2-7). They start from how real teams call plays by situation
//! (nflverse, cfbfastR), bend it by scheme and aggressiveness, study the
//! other side's film, learn what works during the game, and pick concepts
//! that beat what the other side keeps showing. How sharp all of it is
//! comes from the coach's hidden skills: reading, situational knowledge,
//! creativity and adjustment (topic 6.2). A good call wins only through
//! the battles it sets up on the field, never through a bonus.

use super::playbook::*;
use super::roster::{Caller, Front, OffScheme, Shell, Staff};
use super::tables::{self, Clock};
use crate::player::Level;
use crate::rng::{Rng, clamp};

/// What a team has shown this season (topics 5.1 and 7.1): the trail the
/// other staff studies.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Film {
    pub off_snaps: f32,
    pub def_snaps: f32,
    /// Runs and passes by look (personnel, backfield and set).
    pub look: Vec<[f32; 2]>,
    /// Pass concepts and run concepts called.
    pub concepts: Vec<f32>,
    pub runs: [f32; RUN_CONCEPTS.len()],
    /// Coverages, blitzes and fronts shown.
    pub cov: [f32; COVERAGE_COUNT],
    pub blitz: f32,
    pub fronts: [f32; 6],
    /// Passes and plays by down.
    pub by_down: [[f32; 2]; 4],
}

impl Default for Film {
    fn default() -> Film {
        Film {
            off_snaps: 0.0,
            def_snaps: 0.0,
            look: vec![[0.0; 2]; LOOK_COUNT],
            concepts: vec![0.0; PASS_CONCEPTS.len()],
            runs: [0.0; RUN_CONCEPTS.len()],
            cov: [0.0; COVERAGE_COUNT],
            blitz: 0.0,
            fronts: [0.0; 6],
            by_down: [[0.0; 2]; 4],
        }
    }
}

impl Film {
    pub fn add(&mut self, o: &Film) {
        self.off_snaps += o.off_snaps;
        self.def_snaps += o.def_snaps;
        for (a, b) in self.look.iter_mut().zip(&o.look) {
            a[0] += b[0];
            a[1] += b[1];
        }
        for (a, b) in self.concepts.iter_mut().zip(&o.concepts) {
            *a += b;
        }
        for (a, b) in self.runs.iter_mut().zip(&o.runs) {
            *a += b;
        }
        for (a, b) in self.cov.iter_mut().zip(&o.cov) {
            *a += b;
        }
        for (a, b) in self.fronts.iter_mut().zip(&o.fronts) {
            *a += b;
        }
        for (a, b) in self.by_down.iter_mut().zip(&o.by_down) {
            a[0] += b[0];
            a[1] += b[1];
        }
        self.blitz += o.blitz;
    }
    /// Last season's film as this season starts: staffs still study it, at
    /// less weight (topic 7.7).
    pub fn fade(&mut self, k: f32) {
        self.off_snaps *= k;
        self.def_snaps *= k;
        self.look.iter_mut().flatten().for_each(|x| *x *= k);
        self.concepts.iter_mut().for_each(|x| *x *= k);
        self.runs.iter_mut().for_each(|x| *x *= k);
        self.cov.iter_mut().for_each(|x| *x *= k);
        self.fronts.iter_mut().for_each(|x| *x *= k);
        self.by_down.iter_mut().flatten().for_each(|x| *x *= k);
        self.blitz *= k;
    }
    /// The share of snaps in each concept family (topic 9.9): offense
    /// families over offensive snaps, defense over defensive ones.
    pub fn families(&self) -> [f64; FAMILY_COUNT] {
        let mut f = [0.0; FAMILY_COUNT];
        for (i, &n) in self.concepts.iter().enumerate() {
            f[pass_family(pass_concept(i)) as usize] += n as f64;
        }
        for (i, &n) in self.runs.iter().enumerate() {
            f[RUN_CONCEPTS[i].family() as usize] += n as f64;
        }
        let off: f64 = f[..Family::Front as usize].iter().sum();
        for x in f[..Family::Front as usize].iter_mut() {
            *x /= off.max(1.0);
        }
        let d = (self.def_snaps as f64).max(1.0);
        for c in Coverage::ALL {
            f[c.family() as usize] += self.cov[c as usize] as f64 / d;
        }
        f[Family::Pressure as usize] = self.blitz as f64 / d;
        // Fronts beyond the base four-man look (odd, tite, bear).
        f[Family::Front as usize] = self.fronts[3..].iter().map(|&x| x as f64).sum::<f64>() / d;
        f
    }
    /// The offense's snap on film.
    pub fn offense(&mut self, call: &OffCall, down: u8) {
        self.off_snaps += 1.0;
        let pass = !matches!(call.kind, PlayKind::Run(_));
        self.look[call.formation.look()][pass as usize] += 1.0;
        match call.kind {
            PlayKind::Pass(i) => {
                if let Some(x) = self.concepts.get_mut(i) {
                    *x += 1.0;
                }
            }
            PlayKind::Run(r) => self.runs[r as usize] += 1.0,
            PlayKind::Rpo(r, i) => {
                self.runs[r as usize] += 0.5;
                if let Some(x) = self.concepts.get_mut(i) {
                    *x += 0.5;
                }
            }
            _ => {}
        }
        let d = (down.clamp(1, 4) - 1) as usize;
        self.by_down[d][1] += 1.0;
        self.by_down[d][0] += pass as u8 as f32;
    }
    /// The defense's snap on film.
    pub fn defense(&mut self, d: &DefCall) {
        self.def_snaps += 1.0;
        self.cov[d.coverage as usize] += 1.0;
        self.blitz += (d.pressure.extra() > 0) as u8 as f32;
        self.fronts[d.front as usize] += 1.0;
    }
}

/// A staff's matchup plan for one opponent (topic 5.4): their corner to
/// attack, their receiver to bracket, whether to spy their quarterback.
/// Player indexes are into the other team's players.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Plan {
    pub target_cb: Option<usize>,
    pub bracket: Option<usize>,
    pub spy: bool,
    /// Their pass rusher to slide to, chip and double (A8).
    #[serde(default)]
    pub help_on: Option<usize>,
    /// Moves against the other team's tendencies (tendencies topic 4.4),
    /// each 0-1 by how sure the staff is.
    pub vs: VsHabits,
    /// Your answer to each of their personnel groups (update 2, G16): 11,
    /// 12, 10 and 21.
    #[serde(default)]
    pub answers: [Answer; 4],
    /// Moves an "if this, then that" line made during the game (G20): a
    /// safety down in the box, and how much more to run.
    #[serde(default)]
    pub box_up: bool,
    #[serde(default)]
    pub run_more: f64,
}

/// Your defense's answer to one of their personnel groups (update 2,
/// G16): the package (0 the staff's, 1 base, 2 nickel, 3 dime), the man
/// coverage share and the share with five or more rushers.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Answer {
    pub pkg: u8,
    pub man: Option<f32>,
    pub rush: Option<f32>,
}

/// The slot of a personnel group among the answers.
pub fn answer_slot(p: Personnel) -> Option<usize> {
    match p {
        Personnel::P11 => Some(0),
        Personnel::P12 => Some(1),
        Personnel::P10 => Some(2),
        Personnel::P21 => Some(3),
        _ => None,
    }
}

/// Matchup moves that target the other team's habits (tendencies topic
/// 4.4): hard counts against a snap guesser, double moves and pump fakes
/// against route jumpers, kicking short of the goal line to a returner who
/// brings everything out, draws against edges who rush upfield. Each is
/// 0-1: how much the staff leans on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VsHabits {
    pub hard_count: f64,
    pub double_moves: f64,
    pub kick_deep: f64,
    pub draws: f64,
}

/// What a staff's film study makes of a group's habit: the true mean with
/// an error that shrinks with study, and how hard it leans on a move
/// against it (none below about 13 on the scale).
pub fn read_habit(true_mean: f64, film_study: f64, rng: &mut Rng) -> f64 {
    let seen = true_mean + rng.normal(0.0, 0.15 * (21.0 - film_study).max(1.0));
    clamp((seen - 12.5) / 4.0, 0.0, 1.0)
}

/// What a coordinator has learned about this game.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    pub cov_seen: [f64; COVERAGE_COUNT],
    pub fronts_seen: [f64; 6],
    pub snaps_seen: f64,
    /// What the other offense has done.
    pub opp_pass: f64,
    pub opp_plays: f64,
    pub opp_deep_epa: f64,
    pub opp_run_epa: f64,
    pub opp_runs: f64,
    pub opp_quick: f64,
    /// Pass concepts the other offense has run, in `PASS_CONCEPTS` order.
    pub opp_concepts: [f64; PASS_CONCEPTS.len()],
    /// The other offense's runs and passes by look, film and this game
    /// (topic 7): the tells its defenders key.
    pub opp_look: Vec<[f64; 2]>,
    /// This offense's own runs and passes by look this game (self-scouting,
    /// topic 7.6).
    pub own_look: Vec<[f64; 2]>,
    /// Run concepts: own (EPA sum, count) and the other offense's.
    pub run_res: [[f64; 2]; RUN_CONCEPTS.len()],
    pub opp_run_res: [[f64; 2]; RUN_CONCEPTS.len()],
    /// Snaps of film studied on the other side (topic 7.7: little early in
    /// the season, a lot by the playoffs).
    pub film: f64,
    pub plan: Plan,
}

impl Default for Memory {
    fn default() -> Memory {
        Memory {
            run_epa: 0.0,
            runs: 0.0,
            pass_epa: 0.0,
            passes: 0.0,
            deep_epa: 0.0,
            deeps: 0.0,
            blitzes: 0.0,
            cov_seen: [0.0; COVERAGE_COUNT],
            fronts_seen: [0.0; 6],
            snaps_seen: 0.0,
            opp_pass: 0.0,
            opp_plays: 0.0,
            opp_deep_epa: 0.0,
            opp_run_epa: 0.0,
            opp_runs: 0.0,
            opp_quick: 0.0,
            opp_concepts: [0.0; PASS_CONCEPTS.len()],
            opp_look: vec![[0.0; 2]; LOOK_COUNT],
            own_look: vec![[0.0; 2]; LOOK_COUNT],
            run_res: [[0.0; 2]; RUN_CONCEPTS.len()],
            opp_run_res: [[0.0; 2]; RUN_CONCEPTS.len()],
            film: 0.0,
            plan: Plan::default(),
        }
    }
}

impl Memory {
    pub fn saw_defense(&mut self, d: &DefCall) {
        self.snaps_seen += 1.0;
        self.blitzes += (d.pressure.extra() > 0) as u8 as f64;
        self.cov_seen[d.coverage as usize] += 1.0;
        self.fronts_seen[d.front as usize] += 1.0;
    }

    /// Game prep (topic 5.1): the staff breaks down the other team's film,
    /// as deep as its film study allows and with errors. A team with no
    /// film yet (the opener) is studied from its staff's habits. The scheme
    /// a staff has always run is on everyone's tape: a few snaps' worth of
    /// it is known from day one (proposed, not confirmed).
    pub fn prepare(&mut self, opp: &Staff, opp_film: &Film, own: &Staff, rng: &mut Rng) {
        let study = clamp(own.film_study / 20.0, 0.05, 1.0);
        // How many of their snaps the staff can break down well: about 60
        // for a top staff, 20 for a poor one, never more than exist.
        let cap = 20.0 + 45.0 * study;
        // Habits: the shell and the scheme everyone knows.
        let habit = 12.0 * (0.4 + 0.6 * study);
        let mix = shell_mix(opp.shell);
        let tot: f64 = mix.iter().map(|x| x.1).sum();
        for &(cov, w) in &mix {
            self.cov_seen[cov as usize] += habit * w / tot;
        }
        self.blitzes += habit * clamp(0.24 + 0.012 * (opp.defense.aggressiveness - 10.0), 0.05, 0.6);
        self.snaps_seen += habit;
        self.opp_pass += habit * (0.58 + opp.off_scheme.pass_lean());
        self.opp_plays += habit;
        // The tape itself, scaled to what the staff can break down, with a
        // count error that grows as study falls.
        let noisy = |x: f32, rng: &mut Rng| (x as f64 * (1.0 + rng.normal(0.0, 0.35 * (1.0 - study)))).max(0.0);
        if opp_film.def_snaps > 0.0 {
            let k = (cap / opp_film.def_snaps as f64).min(1.0);
            for c in 0..COVERAGE_COUNT {
                self.cov_seen[c] += k * noisy(opp_film.cov[c], rng);
            }
            for f in 0..6 {
                self.fronts_seen[f] += k * noisy(opp_film.fronts[f], rng);
            }
            self.blitzes += k * noisy(opp_film.blitz, rng);
            self.snaps_seen += k * opp_film.def_snaps as f64;
        }
        if opp_film.off_snaps > 0.0 {
            let k = (cap / opp_film.off_snaps as f64).min(1.0);
            for (a, b) in self.opp_look.iter_mut().zip(&opp_film.look) {
                a[0] += k * noisy(b[0], rng);
                a[1] += k * noisy(b[1], rng);
            }
            for (a, b) in self.opp_concepts.iter_mut().zip(&opp_film.concepts) {
                *a += k * noisy(*b, rng);
            }
            let passes: f32 = opp_film.by_down.iter().map(|x| x[0]).sum();
            self.opp_pass += k * passes as f64;
            self.opp_plays += k * opp_film.off_snaps as f64;
            self.film = k * opp_film.off_snaps as f64;
        }
    }

    /// Film studied with your own eyes (game day topic 1): every snap of
    /// it read as it really was, no cap and no count error.
    pub fn study(&mut self, f: &Film) {
        for c in 0..COVERAGE_COUNT {
            self.cov_seen[c] += f.cov[c] as f64;
        }
        for k in 0..6 {
            self.fronts_seen[k] += f.fronts[k] as f64;
        }
        self.blitzes += f.blitz as f64;
        self.snaps_seen += f.def_snaps as f64;
        for (a, b) in self.opp_look.iter_mut().zip(&f.look) {
            a[0] += b[0] as f64;
            a[1] += b[1] as f64;
        }
        for (a, b) in self.opp_concepts.iter_mut().zip(&f.concepts) {
            *a += *b as f64;
        }
        self.opp_pass += f.by_down.iter().map(|x| x[0] as f64).sum::<f64>();
        self.opp_plays += f.off_snaps as f64;
        self.film += f.off_snaps as f64;
    }

    fn rate(&self, x: f64) -> f64 {
        (x + 1.0) / (self.snaps_seen + 4.0)
    }

    /// What this defense believes a look tells it (topic 7.2): how much
    /// more often the other offense runs from it than it runs overall (a
    /// look that runs at the usual rate tells nothing), and how sure it is
    /// (0-1).
    pub fn belief(&self, look: usize) -> (f64, f64) {
        let [r, p] = self.opp_look[look];
        let n = r + p;
        let league = 1.0 - (self.opp_pass + 2.0) / (self.opp_plays + 4.0);
        let p_run = (r + 3.0 * league) / (n + 3.0);
        let sure = n / (n + 6.0);
        (p_run - league, sure)
    }
}

/// The quarterback's check to a pass at the line (topic 3.3): a quick
/// answer every offense carries, slant-flat on short yardage, curl-flat on
/// longer.
pub fn quick_check(togo: f64) -> usize {
    if togo <= 4.0 { concept("slant-flat") } else { concept("curl-flat") }
}

/// The game situation a call is made in.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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
    /// Garbage time (`garbage`): how much the offense plays to end a won
    /// game (0-1), whether it has given up on a lost one, and the same for
    /// the defense.
    pub ease: f64,
    pub concede: bool,
    pub def_ease: f64,
    pub def_concede: bool,
    /// Below the NFL: calls start from the real college habits.
    pub college: bool,
    /// The league's lean toward passing (the other leagues' books, X7).
    pub lean: f64,
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
    /// The offense needs to stop the clock. A team that has decided the game
    /// is won, or lost, doesn't.
    pub fn hurry(&self) -> bool {
        if self.ease >= 0.5 || self.concede {
            return false;
        }
        (self.half_left <= 120.0 && self.score_diff <= 0 && self.quarter >= 2)
            || (self.quarter >= 4 && self.half_left <= 300.0 && self.score_diff < 0)
            || (self.quarter == 2 && self.half_left <= 120.0)
    }
    /// The pressure a caller feels (topic 6.5): behind in the second half.
    pub fn pressure(&self) -> f64 {
        if self.quarter >= 3 && self.score_diff < 0 {
            clamp(-self.score_diff as f64 / 14.0, 0.0, 1.0) * if self.quarter >= 4 { 1.0 } else { 0.6 }
        } else {
            0.0
        }
    }
}

/// What the offense has to call with.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct OffCtx {
    pub has_fb: bool,
    pub te_count: usize,
    /// How much of a runner the quarterback is, 0-1.
    pub qb_run: f64,
    pub level: Level,
    /// This week's game plan as a share of the playbook (calendar T6.2).
    pub plan: f64,
}

fn weigh<T: Copy>(rng: &mut Rng, items: &[(T, f64)]) -> T {
    let pos: Vec<(T, f64)> = items.iter().copied().filter(|x| x.1 > 0.0).collect();
    rng.weighted(&pos)
}

/// How far into the modern game the season is (topic 9.1): motion, RPOs
/// and empty sets grew through the 2010s.
fn era(year: i32, from: i32, years: f64) -> f64 {
    clamp((year - from) as f64 / years, 0.0, 1.0)
}

/// How hard the caller leans on what the numbers say (topic 6.3): about
/// nothing for a poor one, a strong lean for an elite one. Fitted on
/// `dynasty_check value` so an 18 against a 4 swings the real 3-7 points a
/// game (proposed, not confirmed).
pub const TRUST: f64 = 0.55;

fn trust(skill: f64) -> f64 {
    let x = clamp((skill - 1.0) / 19.0, 0.0, 1.0);
    TRUST * x * x
}

/// The offensive call.
pub fn call_offense(staff: &Staff, mem: &Memory, sit: &Situation, ctx: &OffCtx, rng: &mut Rng) -> OffCall {
    let c = &staff.offense;
    let scheme = staff.off_scheme;
    let year = staff.year;
    let mut p_pass = tables::real_pass_rate(sit.down, sit.togo, sit.score_diff, sit.clock(), sit.college);
    // Pressure changes coaches (topic 6.5): behind late a gambler throws
    // more, a tight one sticks to the run.
    let agg = c.aggressiveness + 4.0 * c.nerve * sit.pressure();
    p_pass += scheme.pass_lean() + 0.006 * (agg - 10.0) + sit.lean;
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
    // The head coach's run share (update 2, G2): the caller leans toward it
    // by its firmness on every down; the clock and the score still rule.
    if let Some((run, k)) = staff.guide.run.pull() {
        let own = super::guide::habit_pass(sit.college, scheme, c.aggressiveness);
        p_pass += k * ((1.0 - run) - own);
    }
    // A line in your plan (G20): run it more.
    p_pass -= mem.plan.run_more;
    if sit.hurry() {
        p_pass = p_pass.max(0.85);
    }
    // Garbage time: the leader runs to end it, but still throws about one
    // play in four (real: 28% of 4th-quarter plays when up 28+, FBS and NFL);
    // a beaten team stops forcing it (real: 51% down 28+ in FBS against 62%
    // in close games).
    if sit.ease > 0.0 {
        let target = RUN_OUT_PASS
            * if sit.down >= 3 && sit.togo >= 7.0 {
                2.0
            } else if sit.togo <= 2.0 {
                0.5
            } else {
                1.0
            };
        p_pass += (target - p_pass) * sit.ease;
    }
    if sit.concede {
        p_pass += (CONCEDED_PASS - p_pass) * 0.7;
    }
    if sit.togo <= 1.0 && sit.down >= 3 {
        p_pass = p_pass.min(0.45);
    }
    let p_pass = clamp(p_pass, 0.03, 0.97);
    let mut pass = rng.chance(p_pass);
    let strength = if rng.chance(0.5) { 1.0 } else { -1.0 };
    let short = sit.togo <= 2.0 && sit.down >= 3 || sit.yardline_100 <= 3.0;
    let hurry = sit.hurry();
    // A tell (topic 7.1): a predictable caller's personnel and formation
    // give the play away; a creative one shows the same looks for both.
    let tell = clamp(0.33 - 0.013 * c.creativity, 0.07, 0.33);
    let personnel = pick_personnel(scheme, short, hurry, pass, tell, ctx, staff.guide.personnel.pull(), rng);
    let gun_lean = match scheme {
        OffScheme::AirRaid | OffScheme::SpreadOption => 0.92,
        OffScheme::WestCoast => 0.6,
        OffScheme::WideZone => 0.35,
        OffScheme::PowerGap => 0.4,
        OffScheme::WingT => 0.05,
    } + if ctx.level == Level::NFL { 0.0 } else { 0.08 };
    let gun = rng.chance(clamp(gun_lean + if pass { tell } else { -tell } + if hurry { 0.3 } else { 0.0 }, 0.0, 0.98));
    let backfield = if matches!(personnel, Personnel::P21 | Personnel::P22) && !hurry {
        Backfield::UnderCenter
    } else if gun {
        if matches!(scheme, OffScheme::WideZone | OffScheme::SpreadOption) && rng.chance(0.2) {
            Backfield::Pistol
        } else {
            Backfield::Shotgun
        }
    } else if scheme == OffScheme::WideZone && rng.chance(0.3) {
        Backfield::Pistol
    } else {
        Backfield::UnderCenter
    };
    let set = pick_set(scheme, personnel, pass, short, year, rng);
    let mut formation = Formation { personnel, backfield, set, strength, motion: Motion::None };
    // Self-scouting (topic 7.6): a creative caller who sees he has become
    // predictable from a look breaks it on purpose.
    let [r0, p0] = mem.own_look[formation.look()];
    let n0 = r0 + p0;
    let mut why = "";
    if n0 >= 4.0 && !hurry && !short {
        let p_run = r0 / n0;
        let flip = 0.45 * c.creativity / 20.0;
        if !pass && p_run >= 0.75 && rng.chance(flip) {
            pass = true;
            why = "breaking our run tendency from this look";
        } else if pass && p_run <= 0.15 && rng.chance(flip) {
            pass = false;
            why = "breaking our pass tendency from this look";
        }
    }
    formation.motion = pick_motion(scheme, &formation, pass, year, ctx.level, rng);
    let mut call = if pass {
        let concept = pick_concept(staff, mem, sit, &formation, ctx, rng);
        // Empty needs a quick answer or a full progression; it can't use
        // six- or seven-man protection.
        if formation.set == Set::Empty && pass_concept(concept).protection != Protection::Standard {
            formation.set = Set::Trips;
        }
        if pass_concept(concept).play_action && formation.backfield == Backfield::Shotgun && rng.chance(0.4) {
            formation.backfield = Backfield::Pistol;
        }
        OffCall::new(formation, PlayKind::Pass(concept), strength, hurry)
    } else {
        if formation.set == Set::Empty {
            formation.set = Set::Trips;
        }
        let rc = pick_run(staff, mem, sit, &mut formation, ctx, short, rng);
        // A jet sweep goes the way the motion man runs: across, away from
        // where he lined up.
        let dir = if rc == RunConcept::JetSweep || rc == RunConcept::Counter || rng.chance(0.3) {
            -strength
        } else {
            strength
        };
        let mut call = OffCall::new(formation, PlayKind::Run(rc), dir, hurry);
        // An RPO (topic 1): the same run with a quick throw attached.
        if let Some(i) = pick_rpo(staff, sit, &call, ctx, short, rng) {
            call.kind = PlayKind::Rpo(rc, i);
            why = "a run with a throw tagged on for the defender in conflict";
        }
        call
    };
    // A kill call (topic 3.3): two plays in the huddle, the quarterback
    // picks at the line. Coaches who give the quarterback that freedom do
    // it on early downs.
    if !hurry && sit.down <= 2 && !short {
        call.alt = Some(match call.kind {
            PlayKind::Run(_) | PlayKind::Rpo(..) => {
                PlayKind::Pass(if sit.togo >= 8.0 { concept("curl-flat") } else { concept("slant-flat") })
            }
            _ => PlayKind::Run(RunConcept::InsideZone),
        });
    }
    if why.is_empty() {
        why = call_why(staff, mem, sit, &call);
    }
    call.why = why;
    call
}

/// The coordinator's reason, as he'd say it (topic 6.6).
fn call_why(staff: &Staff, mem: &Memory, sit: &Situation, call: &OffCall) -> &'static str {
    let cover_two =
        mem.cov_seen.iter().enumerate().filter(|(k, _)| Coverage::ALL[*k].high() == 2).map(|x| x.1).sum::<f64>()
            / mem.cov_seen.iter().sum::<f64>().max(1.0);
    let blitz = mem.rate(mem.blitzes);
    match call.kind {
        PlayKind::Run(_) if cover_two > 0.55 => "they keep two safeties deep, so the box is light",
        PlayKind::Run(_) if sit.down == 1 => "first down: stay ahead of the chains",
        PlayKind::Run(_) => "our run game is the call here",
        PlayKind::Pass(i) => {
            let c = pass_concept(i);
            if c.depth == PassDepth::Screen && blitz > 0.3 {
                "they're blitzing, so a screen behind it"
            } else if c.play_action {
                "they've been biting on the run"
            } else if sit.down >= 3 && staff.offense.situational >= 12.0 {
                "routes past the sticks on third down"
            } else if cover_two > 0.55 {
                "a concept built to beat two-high"
            } else {
                "the concept that beats what they've shown"
            }
        }
        _ => "",
    }
}

/// Personnel by scheme and situation, with the caller's tell: a
/// predictable one goes heavy to run and light to throw. The head coach's
/// personnel identity (update 2, G2) wins some of the normal downs by its
/// firmness; short yardage and the hurry-up keep their own groups.
#[allow(clippy::too_many_arguments)]
fn pick_personnel(
    scheme: OffScheme,
    short: bool,
    hurry: bool,
    pass: bool,
    tell: f64,
    ctx: &OffCtx,
    want: Option<(Personnel, f64)>,
    rng: &mut Rng,
) -> Personnel {
    let has_fb = ctx.has_fb;
    let heavy = 1.0 + if pass { -2.0 * tell } else { 2.0 * tell };
    let personnel = if hurry {
        if rng.chance(0.7) { Personnel::P11 } else { Personnel::P10 }
    } else if short {
        weigh(
            rng,
            &[
                (Personnel::P12, 3.0),
                (Personnel::P22, if has_fb { 2.0 } else { 0.0 }),
                (Personnel::P13, if ctx.te_count >= 3 { 1.5 } else { 0.0 }),
                (Personnel::P21, if has_fb { 1.5 } else { 0.0 }),
            ],
        )
    } else {
        let h = heavy.max(0.2);
        match scheme {
            OffScheme::AirRaid => weigh(rng, &[(Personnel::P10, 4.0 / h), (Personnel::P11, 6.0)]),
            OffScheme::WestCoast => weigh(
                rng,
                &[
                    (Personnel::P11, 6.0),
                    (Personnel::P12, 2.5 * h),
                    (Personnel::P21, if has_fb { 1.5 * h } else { 0.0 }),
                ],
            ),
            OffScheme::WideZone => weigh(
                rng,
                &[
                    (Personnel::P11, 5.0),
                    (Personnel::P12, 2.5 * h),
                    (Personnel::P21, if has_fb { 2.5 * h } else { 0.0 }),
                ],
            ),
            OffScheme::PowerGap => weigh(
                rng,
                &[
                    (Personnel::P11, 4.0),
                    (Personnel::P12, 3.5 * h),
                    (Personnel::P21, if has_fb { 2.0 * h } else { 0.0 }),
                    (Personnel::P22, if has_fb { 0.8 * h } else { 0.0 }),
                ],
            ),
            OffScheme::SpreadOption => weigh(rng, &[(Personnel::P11, 6.0), (Personnel::P10, 3.0 / h)]),
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
    let personnel = match want {
        Some((p, k)) if !hurry && !short && p != personnel && rng.chance(0.55 * k) => p,
        _ => personnel,
    };
    let te = ctx.te_count;
    let personnel = if te < 2 && matches!(personnel, Personnel::P12 | Personnel::P22 | Personnel::P13) {
        Personnel::P11
    } else {
        personnel
    };
    let personnel = if te < 3 && personnel == Personnel::P13 { Personnel::P12 } else { personnel };
    if !has_fb && matches!(personnel, Personnel::P21 | Personnel::P22) { Personnel::P11 } else { personnel }
}

/// The set (topic 1). Real 2020s NFL: trips on about a third of three-wide
/// snaps, bunch and stack sets near a tenth, empty on about one dropback in
/// twelve; wings with two tight ends in wide zone and gap schemes
/// (proposed, not confirmed shares).
fn pick_set(scheme: OffScheme, p: Personnel, pass: bool, short: bool, year: i32, rng: &mut Rng) -> Set {
    let wr = p.wide_receivers();
    let modern = era(year, 2008, 14.0);
    if short {
        return if matches!(p, Personnel::P12 | Personnel::P21 | Personnel::P22) && rng.chance(0.4) {
            Set::Wing
        } else {
            Set::Doubles
        };
    }
    let spread = matches!(scheme, OffScheme::AirRaid | OffScheme::WestCoast | OffScheme::SpreadOption);
    let empty = if pass && wr >= 3 && spread { 0.06 + 0.06 * modern } else { 0.0 };
    let bunch = if wr >= 3 || p == Personnel::P11 { 0.03 + 0.06 * modern } else { 0.0 };
    let stack = if wr >= 2 { 0.02 + 0.04 * modern } else { 0.0 };
    let trips = if wr >= 3 { 0.3 } else { 0.0 };
    let wing = if matches!(p, Personnel::P12 | Personnel::P21 | Personnel::P22)
        && matches!(scheme, OffScheme::WideZone | OffScheme::PowerGap | OffScheme::WingT)
    {
        0.3
    } else {
        0.0
    };
    let doubles = (1.0 - empty - bunch - stack - trips - wing).max(0.05);
    weigh(
        rng,
        &[
            (Set::Doubles, doubles),
            (Set::Trips, trips),
            (Set::Bunch, bunch),
            (Set::Stack, stack),
            (Set::Empty, empty),
            (Set::Wing, wing),
        ],
    )
}

/// Motion (topic 3.1): about a quarter of snaps before 2015, more than half
/// by the 2020s (NFL Next Gen Stats); jet motion is about a third of it.
/// High school teams motion less (proposed, not confirmed).
fn pick_motion(scheme: OffScheme, f: &Formation, pass: bool, year: i32, level: Level, rng: &mut Rng) -> Motion {
    if f.set == Set::Empty {
        return Motion::None;
    }
    let rate = (0.22 + 0.33 * era(year, 2014, 9.0)) * if level == Level::HS { 0.6 } else { 1.0 };
    if !rng.chance(rate) {
        return Motion::None;
    }
    let jet =
        if matches!(scheme, OffScheme::WideZone | OffScheme::WingT | OffScheme::SpreadOption) { 1.5 } else { 1.0 };
    weigh(
        rng,
        &[
            (Motion::Jet, jet),
            (Motion::Orbit, if pass { 0.4 } else { 0.2 }),
            (Motion::Across, 1.6),
            (Motion::Shift, 0.8),
        ],
    )
}

/// Pass rates a team drifts toward when it plays out a decided game
/// (proposed, not confirmed; calibrated on `match_check garbage`).
const RUN_OUT_PASS: f64 = 0.22;
const CONCEDED_PASS: f64 = 0.45;

fn pick_run(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    f: &mut Formation,
    ctx: &OffCtx,
    short: bool,
    rng: &mut Rng,
) -> RunConcept {
    use RunConcept::*;
    let scheme = staff.off_scheme;
    let c = &staff.offense;
    let backfield = f.backfield;
    if short && rng.chance(0.25) && backfield == Backfield::UnderCenter {
        return Sneak;
    }
    let gun = backfield != Backfield::UnderCenter;
    let fb = ctx.has_fb && matches!(f.personnel, Personnel::P21 | Personnel::P22);
    let qb = ctx.qb_run;
    // Each level's own plays (topic 1): the triple option lives in high
    // school wing-T and flexbone offenses; speed option in spread offenses
    // below the NFL.
    let below_nfl = ctx.level != Level::NFL;
    let hs = ctx.level == Level::HS;
    let mut w: Vec<(RunConcept, f64)> = match scheme {
        OffScheme::WideZone => vec![
            (OutsideZone, 5.0),
            (InsideZone, 3.0),
            (Counter, 0.8),
            (Toss, 0.6),
            (Draw, 0.4),
            (Duo, 0.6),
            (Trap, 0.3),
        ],
        OffScheme::PowerGap => vec![
            (Power, 4.0),
            (Counter, 2.5),
            (InsideZone, 2.0),
            (Duo, 1.5),
            (Trap, 0.8),
            (Toss, 0.6),
            (Draw, 0.6),
            (Iso, if fb { 1.5 } else { 0.0 }),
        ],
        OffScheme::SpreadOption => vec![
            (ZoneRead, if gun { 5.0 } else { 0.0 }),
            (InsideZone, 3.0),
            (Draw, 1.5),
            (Counter, 1.0),
            (Power, 1.0),
            (SpeedOption, if gun && below_nfl { 1.0 } else { 0.0 }),
            (QbPower, if gun { 2.5 * qb } else { 0.0 }),
            (TripleOption, if hs { 1.0 } else { 0.0 }),
        ],
        OffScheme::WingT => vec![
            (BuckSweep, 4.0),
            (Counter, 2.5),
            (Power, 2.0),
            (Toss, 1.5),
            (InsideZone, 1.0),
            (Trap, 1.5),
            (TripleOption, if below_nfl { 1.5 } else { 0.0 }),
        ],
        OffScheme::WestCoast => {
            vec![(InsideZone, 4.0), (OutsideZone, 2.0), (Draw, 1.2), (Power, 1.0), (Toss, 0.6), (Duo, 0.8), (Trap, 0.4)]
        }
        OffScheme::AirRaid => vec![
            (InsideZone, 4.0),
            (Draw, 2.0),
            (ZoneRead, if gun { 1.5 } else { 0.0 }),
            (Counter, 1.0),
            (QbPower, if gun { 1.0 * qb } else { 0.0 }),
        ],
    };
    if fb {
        w.push((Power, 1.5));
        w.push((Iso, 1.0));
    }
    // A jet man in motion can take it.
    if f.motion == Motion::Jet {
        w.push((JetSweep, 2.5));
    }
    // Draws against edges who rush upfield (tendencies topic 4.4).
    for x in w.iter_mut() {
        if x.0 == Draw {
            x.1 *= 1.0 + 1.5 * mem.plan.vs.draws;
        }
    }
    if short {
        for x in w.iter_mut() {
            if matches!(x.0, Toss | Draw | OutsideZone | JetSweep | SpeedOption) {
                x.1 *= 0.3;
            }
        }
    }
    // What the numbers say works against the fronts this defense shows
    // (topic 2), and what has worked today (topic 3.2), at his skill.
    let fronts = &mem.fronts_seen;
    let ft: f64 = fronts.iter().sum::<f64>() + 1.0;
    let tr = trust(c.reading);
    let ad = c.adaptation / 20.0;
    let table = run_values();
    for x in w.iter_mut() {
        let k = x.0 as usize;
        let v = table.get(k).copied().unwrap_or([0.0; 6]);
        let ev: f64 = (0..6).map(|f| (fronts[f] + 1.0 / 6.0) / ft * v[f]).sum();
        let [sum, n] = mem.run_res[k];
        let today = if n >= 3.0 { sum / n } else { 0.0 };
        x.1 *= (tr * 1.5 * ev + ad * 0.8 * today).exp();
    }
    let rc = weigh(rng, &w);
    // A jet sweep needs the jet motion; a sneak or QB power is called from
    // the gun or under center as it comes.
    if rc == JetSweep {
        f.motion = Motion::Jet;
    }
    let _ = sit;
    rc
}

/// The pass half of an RPO (topic 1): real college spread offenses tag a
/// throw on a large share of their runs since the 2010s; NFL teams far
/// fewer (their linemen may not be past one yard downfield).
fn pick_rpo(staff: &Staff, sit: &Situation, call: &OffCall, ctx: &OffCtx, short: bool, rng: &mut Rng) -> Option<usize> {
    let PlayKind::Run(rc) = call.kind else { return None };
    if short
        || sit.hurry()
        || !matches!(rc, RunConcept::InsideZone | RunConcept::Duo | RunConcept::Power | RunConcept::Counter)
        || call.formation.personnel.wide_receivers() < 2
    {
        return None;
    }
    let scheme = match staff.off_scheme {
        OffScheme::SpreadOption => 0.4,
        OffScheme::AirRaid => 0.35,
        OffScheme::WestCoast => 0.15,
        OffScheme::PowerGap => 0.15,
        OffScheme::WideZone => 0.08,
        OffScheme::WingT => 0.0,
    };
    let level = match ctx.level {
        Level::NFL => 0.4,
        Level::HS => 0.8,
        _ => 1.0,
    };
    let p = scheme * level * era(staff.year, 2010, 8.0);
    if !rng.chance(p) {
        return None;
    }
    Some(weigh(rng, &[(concept("RPO glance"), 1.0), (concept("RPO bubble"), 1.0), (concept("RPO slant"), 0.8)]))
}

/// How deep a concept's first two reads break, for third down.
fn sticks_depth(c: &PassConcept) -> f64 {
    c.reads
        .iter()
        .take(2)
        .map(|r| {
            let rt = route_for(c, *r);
            match rt {
                Route::Go | Route::Seam | Route::Fade | Route::Post(_) | Route::Corner(_) | Route::Wheel => 15.0,
                Route::Slant => 6.0,
                Route::Flat | Route::Swing | Route::Bubble | Route::Screen | Route::Tunnel | Route::Drag => 2.0,
                Route::Angle => 5.0,
                r => r.depth(),
            }
        })
        .fold(0.0, f64::max)
}

fn pick_concept(staff: &Staff, mem: &Memory, sit: &Situation, f: &Formation, ctx: &OffCtx, rng: &mut Rng) -> usize {
    let c = &staff.offense;
    let scheme = staff.off_scheme;
    let long = sit.togo >= 8.0 && sit.down >= 2;
    let short = sit.togo <= 3.0;
    let red = sit.yardline_100 <= 20.0;
    let goal = sit.yardline_100 <= 8.0;
    let hurry = sit.hurry();
    // What the defense has been showing, as well as this coordinator can
    // read it, and what each concept is worth against that.
    let sharp = (c.reading + c.adaptation) / 40.0;
    let ev = expected_values(mem, sharp);
    let mean = ev.iter().sum::<f64>() / ev.len() as f64;
    let tr = trust(c.reading);
    let sit_skill = clamp((c.situational - 4.0) / 14.0, 0.0, 1.0);
    let w: Vec<(usize, f64)> = PASS_CONCEPTS
        .iter()
        .enumerate()
        .map(|(i, pc)| {
            if is_rpo_half(pc) {
                return (i, 0.0);
            }
            let mut w = 1.0;
            match pc.depth {
                // Screens are a staple, called on about 9% of real NFL
                // dropbacks whatever their average gain: they slow the rush
                // and punish blitzes (proposed, not confirmed).
                PassDepth::Screen => w *= if long { 1.2 } else { 1.6 },
                PassDepth::Quick => {
                    w *= if short {
                        2.2
                    } else if long {
                        0.7
                    } else {
                        1.5
                    }
                }
                PassDepth::Medium => w *= if long { 1.4 } else { 0.9 },
                PassDepth::Deep => {
                    w *= if long {
                        1.1
                    } else if short {
                        0.4
                    } else {
                        0.6
                    }
                }
            }
            if pc.play_action {
                // Play action needs a run game to sell, and a set back.
                w *= if sit.down == 1 || short { 1.4 } else { 0.5 };
                if f.backfield == Backfield::Shotgun && pc.name == "play-action boot" {
                    w *= 0.5;
                }
            }
            if red {
                if pc.depth == PassDepth::Deep {
                    w *= 0.3;
                }
                if matches!(pc.name, "fades" | "flood" | "slant-flat" | "snag" | "spacing") {
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
                if matches!(pc.name, "flood" | "curl-flat" | "smash" | "speed outs") {
                    w *= 1.8;
                }
            }
            // Empty: a quick answer or a full progression; a back route needs
            // a back in the backfield.
            if f.set == Set::Empty && (pc.protection != Protection::Standard || pc.play_action) {
                w *= 0.05;
            }
            if f.set == Set::Empty && pc.routes.iter().any(|r| r.0 == ORole::RB) {
                w *= 0.3;
            }
            // The scheme's bread and butter.
            let fav: &[&str] = match scheme {
                OffScheme::AirRaid => {
                    &["mesh", "four verticals", "stick", "Y-cross", "hitches", "shallow cross", "spacing"]
                }
                OffScheme::WestCoast => &["slant-flat", "curl-flat", "stick", "RB screen", "drive", "spacing", "Texas"],
                OffScheme::WideZone => {
                    &["play-action boot", "play-action shot", "play-action dig", "flood", "drive", "tunnel screen"]
                }
                OffScheme::PowerGap => &["play-action shot", "curl-flat", "dagger", "smash", "mills", "Y-option"],
                OffScheme::SpreadOption => {
                    &["bubble screen", "hitches", "four verticals", "RB screen", "snag", "levels"]
                }
                OffScheme::WingT => &["play-action boot", "play-action shot", "flood"],
            };
            if fav.contains(&pc.name) {
                w *= 2.2;
            }
            // What beats what (topic 2): the caller leans toward the
            // concepts that gain the most against the looks he expects.
            w *= (tr * (ev[i] - mean)).exp();
            // Third and fourth down (topic 6.2): a caller who knows the
            // situation calls routes that reach the sticks.
            if sit.down >= 3 && !short {
                let reach = sticks_depth(pc) >= sit.togo;
                w *= 1.0 + 0.8 * sit_skill * if reach { 1.0 } else { -0.6 };
            }
            if pc.depth == PassDepth::Deep && mem.deeps >= 3.0 {
                let e = mem.deep_epa / mem.deeps;
                w *= clamp(1.0 + e * 0.4 * c.adaptation / 20.0, 0.5, 1.6);
            }
            let agg = c.aggressiveness + 4.0 * c.nerve * sit.pressure();
            w *= 1.0 + 0.03 * (agg - 10.0) * if pc.depth == PassDepth::Deep { 1.0 } else { -0.3 };
            // Up big, a little less going deep (real NFL: 11.7% of throws
            // travel 20+ yards up 28+, 13.6% in close games).
            if pc.depth == PassDepth::Deep || pc.name == "play-action shot" {
                w *= 1.0 - 0.3 * sit.ease;
            }
            // A small book carries fewer concepts (topic 4.6), and the week's
            // plan sets how far past his favorites the caller reaches (T6.2):
            // a small plan is predictable, a big one varied.
            let book = staff.playbook * ctx.plan / super::roster::INSTALL;
            if !fav.contains(&pc.name) {
                w *= book.clamp(0.2, 1.4);
            }
            (i, w.max(0.01))
        })
        .collect();
    weigh(rng, &w)
}

/// Football knowledge: what each concept gains against each look, measured
/// on the match engine (`call_values` in afm-tools). Pass concepts by
/// coverage and pressure; runs by front.
const CALL_VALUES_CSV: &str = include_str!("../../data/call_values.csv");

/// Column of a coverage and pressure in the table: Cover 0 with the
/// double-A blitz, then each other coverage with four rushers and with a
/// linebacker blitz.
fn col(cov: Coverage, blitz: bool) -> usize {
    match cov {
        Coverage::Cover0 => 0,
        c => {
            let k = Coverage::ALL.iter().position(|x| *x == c).unwrap_or(1);
            2 * k - 1 + blitz as usize
        }
    }
}

const LOOK_COLS: usize = 2 * COVERAGE_COUNT - 1;

struct Values {
    pass: Vec<[f64; LOOK_COLS]>,
    run: Vec<[f64; 6]>,
}

fn values() -> &'static Values {
    static T: std::sync::OnceLock<Values> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let mut lines = CALL_VALUES_CSV.lines();
        let head: Vec<String> = lines.next().unwrap_or_default().split(',').map(|s| s.to_string()).collect();
        let mut rows = std::collections::HashMap::new();
        for line in lines {
            let mut f = line.split(',');
            let name = f.next().unwrap_or_default().to_string();
            let vals: Vec<f64> = f.map(|s| s.trim().parse().unwrap_or(0.0)).collect();
            rows.insert(name, vals);
        }
        // Columns by header name, so the table can grow.
        let at = |name: &str| head.iter().position(|h| h == name).map(|k| k - 1);
        let pass_cols: Vec<Option<usize>> = Coverage::ALL
            .iter()
            .flat_map(|&c| {
                if c == Coverage::Cover0 {
                    vec![at("Cover0/DoubleA")]
                } else {
                    vec![at(&format!("{c:?}/Four")), at(&format!("{c:?}/LbBlitz"))]
                }
            })
            .collect();
        let front_cols: Vec<Option<usize>> =
            ["Over", "Under", "Wide", "Odd", "Tite", "Bear"].iter().map(|f| at(&format!("Front:{f}"))).collect();
        let get = |row: Option<&Vec<f64>>, k: Option<usize>| {
            row.and_then(|r| k.and_then(|k| r.get(k).copied())).unwrap_or(0.0)
        };
        let pass = PASS_CONCEPTS
            .iter()
            .map(|p| {
                let row = rows.get(p.name);
                let mut v = [0.0; LOOK_COLS];
                for (j, x) in v.iter_mut().enumerate() {
                    *x = get(row, pass_cols[j]);
                }
                v
            })
            .collect();
        let run = RUN_CONCEPTS
            .iter()
            .map(|r| {
                let row = rows.get(&format!("{r:?}"));
                let mut v = [0.0; 6];
                for (j, x) in v.iter_mut().enumerate() {
                    *x = get(row, front_cols[j]);
                }
                // Centered: a run is chosen against a front by how much
                // better it does there than its own average.
                let m = v.iter().sum::<f64>() / 6.0;
                v.map(|x| x - m)
            })
            .collect();
        Values { pass, run }
    })
}

fn call_values() -> &'static Vec<[f64; LOOK_COLS]> {
    &values().pass
}

fn run_values() -> &'static Vec<[f64; 6]> {
    &values().run
}

/// The league's coverage mix, the starting belief before any film.
fn league_mix() -> [f64; COVERAGE_COUNT] {
    let mut m = [0.0; COVERAGE_COUNT];
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
    let p: Vec<f64> = (0..COVERAGE_COUNT)
        .map(|k| sharp * (mem.cov_seen[k] + 4.0 * prior[k]) / (mem.snaps_seen + 4.0) + (1.0 - sharp) * prior[k])
        .collect();
    call_values()
        .iter()
        .map(|v| {
            let mut e = p[0] * v[0];
            for (k, &c) in Coverage::ALL.iter().enumerate().skip(1) {
                e += p[k] * ((1.0 - blitz) * v[col(c, false)] + blitz * v[col(c, true)]);
            }
            e
        })
        .collect()
}

/// What each coverage gives up (four rushers) against the pass concepts a
/// defensive coordinator expects. A sharp one trusts what he has seen; a
/// poor one assumes every concept is as likely.
fn coverage_values(mem: &Memory, sharp: f64) -> [f64; COVERAGE_COUNT] {
    let t = call_values();
    let seen: f64 = mem.opp_concepts.iter().sum();
    let live: Vec<usize> = (0..t.len()).filter(|&i| !is_rpo_half(pass_concept(i))).collect();
    let n = live.len() as f64;
    let mut out = [0.0; COVERAGE_COUNT];
    for (k, o) in out.iter_mut().enumerate() {
        let c = col(Coverage::ALL[k], false);
        *o = live
            .iter()
            .map(|&i| {
                let q = sharp * (mem.opp_concepts[i] + 4.0 / n) / (seen + 4.0) + (1.0 - sharp) / n;
                q * t[i][c]
            })
            .sum();
    }
    out
}

/// A defensive coordinator's family of coverages: how often he calls each.
/// The robber and pattern-matched Cover 3 are the modern versions of each
/// family's single-high calls (proposed, not confirmed shares).
pub fn shell_mix(shell: Shell) -> [(Coverage, f64); COVERAGE_COUNT] {
    match shell {
        Shell::Cover3 => [
            (Coverage::Cover3, 26.0),
            (Coverage::Cover3Match, 16.0),
            (Coverage::Cover1, 15.0),
            (Coverage::Cover1Robber, 5.0),
            (Coverage::Cover4, 10.0),
            (Coverage::Cover2, 9.0),
            (Coverage::Cover6, 6.0),
            (Coverage::Cover2Man, 4.0),
            (Coverage::Cover0, 4.0),
            (Coverage::Tampa2, 5.0),
        ],
        Shell::Quarters => [
            (Coverage::Cover3, 10.0),
            (Coverage::Cover3Match, 8.0),
            (Coverage::Cover1, 10.0),
            (Coverage::Cover1Robber, 4.0),
            (Coverage::Cover4, 30.0),
            (Coverage::Cover2, 12.0),
            (Coverage::Cover6, 14.0),
            (Coverage::Cover2Man, 5.0),
            (Coverage::Cover0, 3.0),
            (Coverage::Tampa2, 4.0),
        ],
        Shell::ManPressure => [
            (Coverage::Cover3, 14.0),
            (Coverage::Cover3Match, 8.0),
            (Coverage::Cover1, 26.0),
            (Coverage::Cover1Robber, 12.0),
            (Coverage::Cover4, 8.0),
            (Coverage::Cover2, 6.0),
            (Coverage::Cover6, 4.0),
            (Coverage::Cover2Man, 10.0),
            (Coverage::Cover0, 9.0),
            (Coverage::Tampa2, 3.0),
        ],
        Shell::Cover2 => [
            (Coverage::Cover3, 14.0),
            (Coverage::Cover3Match, 6.0),
            (Coverage::Cover1, 9.0),
            (Coverage::Cover1Robber, 3.0),
            (Coverage::Cover4, 10.0),
            (Coverage::Cover2, 28.0),
            (Coverage::Cover6, 8.0),
            (Coverage::Cover2Man, 6.0),
            (Coverage::Cover0, 2.0),
            (Coverage::Tampa2, 14.0),
        ],
    }
}

/// What the defense sees of the offense's quarterback and receivers before
/// the call (topic 5.4).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DefCtx {
    /// The quarterback's running threat, 0-1.
    pub qb_run: f64,
    /// The offense's call read from its call words (update 2, G5): +1 a
    /// pass, -1 a run, 0 nothing read.
    #[serde(default)]
    pub tip: f64,
}

/// The defensive call.
pub fn call_defense(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    off: &Formation,
    ctx: &DefCtx,
    rng: &mut Rng,
) -> DefCall {
    let c: &Caller = &staff.defense;
    let wr = off.personnel.wide_receivers();
    let long = sit.togo >= 8.0 && sit.down >= 2;
    let short = sit.togo <= 2.0;
    let answer = answer_slot(off.personnel).map(|k| mem.plan.answers[k]).unwrap_or_default();
    let personnel = if sit.yardline_100 <= 3.0 && wr <= 2 {
        DefPersonnel::GoalLine
    } else if answer.pkg > 0 && !(long && sit.down == 3) {
        // Your answer to this group this week (G16).
        [DefPersonnel::Base, DefPersonnel::Nickel, DefPersonnel::Dime][(answer.pkg as usize - 1).min(2)]
    } else if wr >= 4 || (long && sit.down == 3 && wr >= 3 && rng.chance(0.5)) {
        DefPersonnel::Dime
    } else if wr == 3 {
        // The base front the head coach asks for (update 2, G2) leans the
        // habit: a nickel base nearly always, a 4-3 or 3-4 team a little
        // less, but it still goes to nickel.
        let p = match staff.guide.front.pull() {
            Some((b, k)) if b.nickel_base() => staff.nickel + k * (0.98 - staff.nickel),
            Some((_, k)) => staff.nickel * (1.0 - 0.12 * k),
            None => staff.nickel,
        };
        if rng.chance(p) { DefPersonnel::Nickel } else { DefPersonnel::Base }
    } else {
        // A nickel base front (4-2-5, 3-3-5) keeps its fifth back on the
        // field against two receivers too, less against heavy sets.
        match staff.guide.front.pull() {
            Some((b, k)) if b.nickel_base() && rng.chance(k * if wr <= 1 { 0.45 } else { 0.9 }) => DefPersonnel::Nickel,
            _ => DefPersonnel::Base,
        }
    };
    // The coordinator's family of coverages, then the situation.
    let base = shell_mix(staff.shell);
    let sharp = c.adaptation / 20.0;
    let opp_pass = (mem.opp_pass + 2.0) / (mem.opp_plays + 4.0);
    let deep_hurting = if mem.opp_plays > 6.0 { (mem.opp_deep_epa / mem.opp_plays.max(1.0)).max(0.0) } else { 0.0 };
    let run_hurting = if mem.opp_runs > 5.0 { (mem.opp_run_epa / mem.opp_runs).max(0.0) } else { 0.0 };
    // What the defense believes the offense will do from this look (topic
    // 7.2): a run tell brings the safety down, a pass tell keeps two high.
    let (p_run, sure) = mem.belief(off.look());
    let read = (c.reading + c.adaptation) / 40.0;
    let lean_run = if ctx.tip != 0.0 { -0.5 * ctx.tip * read.max(0.6) } else { (p_run - 0.5) * sure * read };
    // What each coverage gives up against the passes this coordinator
    // expects (topic 2): a sharp one leans away from the coverages they
    // beat.
    let given = coverage_values(mem, read);
    let mean = given.iter().sum::<f64>() / COVERAGE_COUNT as f64;
    let tr = trust(c.reading);
    let w: Vec<(Coverage, f64)> = base
        .iter()
        .map(|&(cov, w)| {
            let mut w = w * (-tr * (given[cov as usize] - mean)).exp();
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
                w *= (1.0 - 2.0 * lean_run).max(0.2);
            } else {
                w *= 1.0 + sharp * run_hurting * 2.5;
                w *= (1.0 + 2.0 * lean_run).max(0.2);
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
    // The head coach's man-coverage share (update 2, G2): the coordinator
    // moves his man share toward it by its firmness and keeps the shape
    // the down gives him.
    let mut w = w;
    if let Some((want, k)) = staff.guide.man.pull() {
        super::guide::steer_man(&mut w, k * (want - super::guide::habit_man(staff.shell)));
    }
    // Your answer to this group (G16): the coordinator plays close to it.
    if let Some(m) = answer.man {
        let all: f64 = w.iter().map(|x| x.1).sum();
        let now = w.iter().filter(|x| x.0.is_man()).map(|x| x.1).sum::<f64>() / all.max(1e-9);
        super::guide::steer_man(&mut w, 0.7 * (m as f64 - now));
    }
    // A line in your plan (G20): a safety down against their run.
    if mem.plan.box_up {
        for x in w.iter_mut().filter(|x| x.0.high() == 2) {
            x.1 *= 0.6;
        }
    }
    // Garbage time: the leading defense keeps everything in front of it
    // (two-high, fewer man looks, no Cover 0) instead of hunting stops.
    let ease = sit.def_ease;
    let w: Vec<(Coverage, f64)> = w
        .into_iter()
        .map(|(cov, x)| {
            let mut x = x;
            if cov == Coverage::Cover0 {
                x *= 1.0 - ease;
            } else if cov.is_man() {
                x *= 1.0 - 0.5 * ease;
            }
            if cov.high() == 2 {
                x *= 1.0 + ease;
            }
            (cov, x)
        })
        .collect();
    let coverage = weigh(rng, &w);
    let mut blitz = 0.24 + 0.012 * (c.aggressiveness - 10.0) - 0.2 * ease;
    if sit.down == 3 && (4.0..=9.0).contains(&sit.togo) {
        blitz += 0.1;
    }
    if sit.hurry() {
        blitz -= 0.1;
    }
    blitz += sharp * (mem.opp_quick / mem.opp_plays.max(1.0) - 0.3).min(0.0) * 0.5;
    // The head coach's pressure rate (update 2, G2).
    if let Some(r) = answer.rush {
        blitz += 0.7 * (r as f64 - super::guide::habit_pressure(c.aggressiveness));
    } else if let Some((want, k)) = staff.guide.pressure.pull() {
        blitz += k * (want - super::guide::habit_pressure(c.aggressiveness));
    }
    let zone = !coverage.is_man() && coverage != Coverage::Cover0;
    let pressure = if coverage == Coverage::Cover0 {
        Pressure::DoubleA
    } else if rng.chance(clamp(blitz, 0.05, 0.7)) {
        weigh(
            rng,
            &[
                (Pressure::LbBlitz, 4.0),
                (Pressure::DbBlitz, 2.0),
                (Pressure::DoubleA, 1.0),
                (Pressure::Simulated, if c.skill > 9.0 { 2.0 } else { 0.5 }),
                // A fire zone plays three-deep, three-under behind it.
                (Pressure::FireZone, if zone && coverage.high() == 1 { 1.0 + 0.1 * c.skill } else { 0.0 }),
            ],
        )
    } else {
        Pressure::Four
    };
    let pressure =
        if personnel == DefPersonnel::Dime && pressure == Pressure::DoubleA { Pressure::DbBlitz } else { pressure };
    let disguise = rng.chance(clamp(0.05 + 0.025 * c.skill, 0.05, 0.6) * (1.0 - ease));
    let front = pick_front(staff, mem, sit, off, lean_run, rng);
    // Stunts (topic 1): on four-man rushes, more on passing downs; real
    // NFL lines run one on roughly a fifth of dropbacks (proposed, not
    // confirmed).
    let stunt = if pressure == Pressure::Four || pressure == Pressure::Simulated {
        let p = clamp(
            0.12 + 0.008 * (c.aggressiveness - 10.0) + if long { 0.08 } else { 0.0 } - lean_run * 0.2,
            0.03,
            0.35,
        );
        if rng.chance(p) {
            weigh(rng, &[(Stunt::TackleEnd, 2.0), (Stunt::EndTackle, 1.5), (Stunt::Twist, 1.2)])
        } else {
            Stunt::None
        }
    } else {
        Stunt::None
    };
    // A spy on a running quarterback on passing downs (topic 5.4).
    let spy = (mem.plan.spy || ctx.qb_run > 0.75)
        && (long || sit.down >= 3)
        && !coverage.is_man()
        && rng.chance(0.25 + 0.02 * c.reading);
    DefCall { personnel, coverage, pressure, disguise, front, stunt, spy }
}

/// The front (topic 1): a 4-3 staff plays over, under and wide-nine looks,
/// a 3-4 staff odd and tite fronts; both bear up against heavy personnel
/// and short yardage. A sharp coordinator fits the front to the runs that
/// hurt him (proposed, not confirmed shares).
fn pick_front(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    off: &Formation,
    lean_run: f64,
    rng: &mut Rng,
) -> FrontLook {
    let heavy = matches!(off.personnel, Personnel::P12 | Personnel::P21 | Personnel::P22 | Personnel::P13);
    let short = sit.togo <= 2.0;
    let pass_down = sit.down >= 2 && sit.togo >= 7.0;
    let mut w: Vec<(FrontLook, f64)> = match staff.front {
        Front::FourThree => vec![
            (FrontLook::Over, 4.0),
            (FrontLook::Under, 3.0),
            (FrontLook::Wide, if pass_down { 3.0 } else { 1.0 }),
            (FrontLook::Bear, if heavy || short { 1.5 } else { 0.3 }),
        ],
        Front::ThreeFour => vec![
            (FrontLook::Odd, 4.0),
            (FrontLook::Tite, 3.0),
            (FrontLook::Under, 1.0),
            (FrontLook::Bear, if heavy || short { 1.5 } else { 0.3 }),
        ],
    };
    if lean_run > 0.0 {
        for x in w.iter_mut() {
            if matches!(x.0, FrontLook::Bear | FrontLook::Tite) {
                x.1 *= 1.0 + 3.0 * lean_run;
            }
        }
    }
    // The runs that hurt today: tite and bear against inside runs, wide
    // and over against the perimeter.
    let c = &staff.defense;
    let ad = c.adaptation / 20.0;
    let inside: f64 = [RunConcept::InsideZone, RunConcept::Duo, RunConcept::Power, RunConcept::Trap, RunConcept::Iso]
        .iter()
        .map(|r| mem.opp_run_res[*r as usize])
        .filter(|v| v[1] >= 3.0)
        .map(|v| v[0] / v[1])
        .fold(0.0, f64::max);
    let outside: f64 = [RunConcept::OutsideZone, RunConcept::Toss, RunConcept::JetSweep, RunConcept::BuckSweep]
        .iter()
        .map(|r| mem.opp_run_res[*r as usize])
        .filter(|v| v[1] >= 3.0)
        .map(|v| v[0] / v[1])
        .fold(0.0, f64::max);
    for x in w.iter_mut() {
        match x.0 {
            FrontLook::Tite | FrontLook::Bear => x.1 *= 1.0 + 4.0 * ad * inside.max(0.0),
            FrontLook::Wide | FrontLook::Over => x.1 *= 1.0 + 4.0 * ad * outside.max(0.0),
            _ => {}
        }
    }
    weigh(rng, &w)
}

/// How much better than our tackles (rating points, as the staff sees
/// it) a rusher must be before the plan slides to him (A8, Claude's pick).
const HELP_GAP: f64 = 3.0;

/// A staff's matchup plan (topic 5.4), made at the start of the game from
/// what its scouts see of the other team's starters: rating estimates with
/// an error that shrinks with film study.
#[allow(clippy::too_many_arguments)]
pub fn make_plan(
    own: &Staff,
    their_cbs: &[(usize, f64)],
    their_wrs: &[(usize, f64)],
    our_cbs: &[f64],
    their_qb_run: f64,
    their_rushers: &[(usize, f64)],
    our_blockers: &[f64],
    rng: &mut Rng,
) -> Plan {
    let err = 0.8 * (20.0 - own.film_study).max(1.0);
    let seen = |v: f64, rng: &mut Rng| v + rng.normal(0.0, err);
    // Attack the weakest corner when he is clearly the weak one.
    let target_cb = {
        let mut cbs: Vec<(usize, f64)> = their_cbs.iter().map(|&(i, v)| (i, seen(v, rng))).collect();
        cbs.sort_by(|a, b| a.1.total_cmp(&b.1));
        if cbs.len() >= 2 && cbs[1].1 - cbs[0].1 > 4.0 { Some(cbs[0].0) } else { None }
    };
    // Bracket their best receiver when no corner of ours can match him.
    let bracket = {
        let best = their_wrs.iter().map(|&(i, v)| (i, seen(v, rng))).max_by(|a, b| a.1.total_cmp(&b.1));
        let our = our_cbs.iter().copied().fold(0.0, f64::max);
        best.filter(|b| b.1 - our > 5.0).map(|b| b.0)
    };
    let spy = their_qb_run > 0.6 && rng.chance(0.4 + 0.03 * own.defense.reading);
    // Help against their best rusher (A8): every staff slides to a rusher
    // its tackles can't handle alone, the more clearly he is better than
    // the men who'd block him. Read from film like the rest of the plan.
    let help_on = {
        let best = their_rushers.iter().map(|&(i, v)| (i, seen(v, rng))).max_by(|a, b| a.1.total_cmp(&b.1));
        let ours =
            if our_blockers.is_empty() { 0.0 } else { our_blockers.iter().sum::<f64>() / our_blockers.len() as f64 };
        best.filter(|b| b.1 - ours > HELP_GAP).map(|b| b.0)
    };
    Plan {
        target_cb,
        bracket,
        spy,
        help_on,
        vs: VsHabits::default(),
        answers: Default::default(),
        box_up: false,
        run_more: 0.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    // A beaten team that has given up plays it straight.
    if late && sit.score_diff < 0 && !sit.concede {
        let need = -sit.score_diff;
        if need > 3 || sit.game_left < 120.0 {
            ev_go_adj += 1.2 + (300.0 - sit.game_left) / 100.0;
        }
    }
    if late && sit.score_diff > 0 {
        ev_go_adj -= 0.6;
    }
    let gm = staff.game_management;
    // Behind late, a gambler goes for it more and a tight coach less (topic 6.5).
    let agg = staff.offense.aggressiveness + 4.0 * staff.offense.nerve * sit.pressure();
    // Caution: what a coach needs to see before he goes for it.
    let margin = 0.06 * (11.0 - agg) + 0.03 * (11.0 - gm) + 0.35;
    let noise = rng.normal(0.0, 0.5 * (1.3 - gm / 20.0));
    let best_kick =
        if ev_fg > ev_punt && fg_make > 0.3 { (FourthDown::FieldGoal, ev_fg) } else { (FourthDown::Punt, ev_punt) };
    // Real coaches still mostly kick: the real go rate as a habit.
    let habit = tables::real_fourth_go_rate(togo, y, sit.score_diff, sit.college);
    let by_numbers = ev_go_adj + noise > best_kick.1 + margin;
    let follow = clamp(0.05 + 0.012 * gm + 0.01 * (agg - 10.0), 0.03, 0.45);
    let go = if rng.chance(follow) { by_numbers } else { rng.chance(habit) };
    if go || (late && sit.score_diff < -3 && sit.game_left < 150.0 && !sit.concede) {
        FourthDown::Go
    } else {
        best_kick.0
    }
}

/// The numbers behind a fourth down, for the analytics staff (game day
/// topic 6): the chance to convert and the expected points of going,
/// punting and kicking, from the same real tables `fourth_down` uses.
pub fn fourth_values(sit: &Situation, fg_make: f64) -> (f64, f64, f64, f64) {
    fourth_ev(sit.yardline_100, sit.togo, fg_make)
}

/// `fourth_values` from the spot alone (yards to the goal, yards to go):
/// also what the morning-after bots read your calls with (lane 8).
pub fn fourth_ev(y: f64, togo: f64, fg_make: f64) -> (f64, f64, f64, f64) {
    let conv = clamp(0.70 * (-0.085 * (togo - 1.0)).exp(), 0.12, 0.75);
    let ep = tables::expected_points;
    let after_score = 0.95;
    let go_ok = if togo >= y { 6.95 - after_score } else { ep(1, 10.0_f64.min(y - togo), y - togo) };
    let go_fail = -ep(1, 10.0, 100.0 - y);
    let ev_go = conv * go_ok + (1.0 - conv) * go_fail;
    let land = y - 41.0;
    let opp_punt = if land < 0.0 { 80.0 } else { 100.0 - land.max(8.0) };
    let ev_punt = -ep(1, 10.0, opp_punt.min(99.0));
    let opp_miss = (100.0 - y - 7.0).min(80.0);
    let ev_fg = fg_make * (3.0 - after_score) + (1.0 - fg_make) * -ep(1, 10.0, opp_miss.max(1.0));
    (conv, ev_go, ev_punt, ev_fg)
}

/// Go for two after a touchdown (the chart, plus the coach's lean and how
/// much he trusts his kicker).
/// The three-point try, where a book has one (the XFL of 2020 and 2023 and
/// the UFL's, from the 10 or the 8): late, when the score needs three
/// (down three or ten, or up four or five to go up by a touchdown and
/// more); early, now and then from an aggressive staff (about one try in
/// ten in the XFL of 2020).
pub fn go_for_three(staff: &Staff, diff_after_td: i32, quarter: u8, game_left: f64, rng: &mut Rng) -> bool {
    let late = quarter >= 4 && game_left < 900.0;
    let chart = late && matches!(diff_after_td, -3 | -10 | -11 | 4 | 5 | -17);
    let lean = 0.05 + 0.01 * (staff.offense.aggressiveness - 10.0);
    chart && rng.chance(0.7) || !late && rng.chance(lean.max(0.0))
}

pub fn go_for_two(staff: &Staff, diff_after_td: i32, quarter: u8, game_left: f64, xp_make: f64, rng: &mut Rng) -> bool {
    let late = quarter >= 4 && game_left < 900.0;
    let chart = late && matches!(diff_after_td, -2 | -5 | -10 | -13 | 1 | 4 | 5 | 12 | -16 | 8 | -9);
    let weak_kicker = xp_make < 0.75;
    let lean = 0.01 + 0.004 * (staff.offense.aggressiveness - 10.0);
    chart && rng.chance(0.8) || weak_kicker && rng.chance(0.55) || rng.chance(lean.max(0.0))
}
