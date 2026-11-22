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
    /// How many of each pass concept's throws came off a run fake (minor
    /// bug 412): they count as play action in the film's families.
    #[serde(default = "no_fakes")]
    pub faked: Vec<f32>,
    pub runs: [f32; RUN_CONCEPTS.len()],
    /// Coverages, blitzes and fronts shown.
    pub cov: [f32; COVERAGE_COUNT],
    pub blitz: f32,
    pub fronts: [f32; 6],
    /// Passes and plays by down.
    pub by_down: [[f32; 2]; 4],
}

fn no_fakes() -> Vec<f32> {
    vec![0.0; PASS_CONCEPTS.len()]
}

impl Default for Film {
    fn default() -> Film {
        Film {
            off_snaps: 0.0,
            def_snaps: 0.0,
            look: vec![[0.0; 2]; LOOK_COUNT],
            concepts: vec![0.0; PASS_CONCEPTS.len()],
            faked: no_fakes(),
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
        for (a, b) in self.faked.iter_mut().zip(&o.faked) {
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
        self.faked.iter_mut().for_each(|x| *x *= k);
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
            let fam = pass_family(pass_concept(i));
            // A throw off a run fake is play action, not its concept's
            // family (minor bug 412); a screen stays a screen.
            let faked = self.faked.get(i).map_or(0.0, |&x| x as f64).min(n as f64);
            let faked = if faked > 0.0 && faked_family(pass_concept(i), true) != fam { faked } else { 0.0 };
            f[Family::PlayAction as usize] += faked;
            f[fam as usize] += n as f64 - faked;
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
                let c = film_concept(i);
                if let Some(x) = self.concepts.get_mut(c) {
                    *x += 1.0;
                }
                if call.fake
                    && let Some(x) = self.faked.get_mut(c)
                {
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
    /// Their deep threat (speed update S10a-S10d): the receiver whose speed
    /// our corners can't run with as the film shows it (an offense player
    /// index), and how much the staff fears him, 0-1.
    #[serde(default)]
    pub burner: Option<usize>,
    #[serde(default)]
    pub fear: f64,
    /// The man coverage share your word promised a corner (the promises
    /// on usage): the coordinator plays at least about this much (0 none).
    #[serde(default)]
    pub man_floor: f32,
    /// The shells your word promised (lane N2): at least about this share
    /// two-high (help over the top), single-high (a post safety) and Cover
    /// 1 robber (0 none).
    #[serde(default)]
    pub shell_floor: [f32; 3],
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
    /// The other offense's plays in this game alone, without the prep
    /// snaps in `opp_plays` (update 3, F32-F33).
    #[serde(default)]
    pub opp_game_plays: f64,
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
            opp_game_plays: 0.0,
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
    /// How far the weather moves the call toward the run (minor bug 144).
    #[serde(default)]
    pub weather: f64,
    /// In the second quarter's last two minutes, whether the offense goes
    /// for points before the half (engine gaps item 17): the game's sum of
    /// its spot, the score and both sides' timeouts. A side that doesn't
    /// plays the half out.
    #[serde(default)]
    pub half_go: bool,
    /// What the staff reads into its fourth-down sums from the two teams and
    /// the level (engine gaps item 20); the default on every other down.
    #[serde(default)]
    pub fourth: FourthRead,
    /// A try after a touchdown, played from scrimmage. It is no down: it
    /// calls from the real tries' mix (`TRY_PASS`), and the short field's
    /// lean by down (`RED_ZONE_PASS`) passes it by (lane EG6).
    #[serde(default)]
    pub trying: bool,
}

/// The two teams and the level in a fourth-down sum (engine gaps item 20):
/// how far this matchup's short yardage moves the chance to convert (on its
/// log odds), the punt's expected net yards, and how far behind the line of
/// scrimmage a missed field goal gives the ball over. The default is the
/// NFL's typical teams: no edge, a 41-yard net punt and the spot of the
/// kick, 7 yards back.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FourthRead {
    pub conv: f64,
    pub punt: f64,
    pub miss_back: f64,
}

impl Default for FourthRead {
    fn default() -> FourthRead {
        FourthRead { conv: 0.0, punt: 41.0, miss_back: 7.0 }
    }
}

/// The chance to convert on fourth down from the yards to go (real NFL:
/// about 70% on fourth and 1, 50% on fourth and 5, 33% on fourth and 10),
/// moved on its log odds by the matchup (`FourthRead::conv`).
fn fourth_conv(togo: f64, edge: f64) -> f64 {
    let base = clamp(0.70 * (-0.085 * (togo - 1.0)).exp(), 0.12, 0.75);
    let lo = (base / (1.0 - base)).ln() + edge;
    clamp(1.0 / (1.0 + (-lo).exp()), 0.05, 0.9)
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
        (self.half_left <= 120.0 && self.score_diff <= 0 && self.quarter >= 4)
            || (self.quarter >= 4 && self.half_left <= 300.0 && self.score_diff < 0)
            || (self.quarter == 2 && self.half_left <= 120.0 && self.half_go)
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
    /// Designed plays on this week's sheet (update 3, F27): playbook
    /// number and the look it was drawn from.
    #[serde(default)]
    pub designs: [Option<(u16, Personnel, Backfield, Set)>; 8],
    /// How much the offense leans on its raw burner (speed update S10),
    /// 0-1; 0 without one who can go.
    #[serde(default)]
    pub burner: f64,
    /// How much more the caller goes deep for the man promised shots (the
    /// promises on usage), 0 without one.
    #[serde(default)]
    pub shots: f64,
    /// The game plan keeping your word (the promises on usage, lane N2):
    /// nothing for a team whose plan holds none (every AI team).
    #[serde(default)]
    pub lean: Lean,
    /// Players a side under this game's rule book: 11, the CFL's 12 or
    /// arena football's 8. The call is made in 11-man groups and the game
    /// fields the book's own set for them (`Personnel::for_players`), so
    /// the concept is picked among the ones that set can run (lane SF2).
    #[serde(default = "eleven")]
    pub players: u8,
    /// What each call gains from the offense's own men (lane SF10,
    /// `roster_fit`): nothing for the typical team, and nothing where no
    /// roster is read (a draft of your call sheet before the week).
    #[serde(skip)]
    pub fit: super::roster_fit::OffFit,
}

fn eleven() -> u8 {
    11
}

/// How the game plan leans the calls to keep your promises on usage
/// (`roster::Keep`, set from your open promises; lane N2). Each part is 0
/// (or None) unless a promise asks for it, so a plan without one calls
/// exactly as before. Sizes are Claude's (proposed, not confirmed).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Lean {
    /// Added to the pass rate (throws for the quarterback promised them).
    pub pass: f64,
    /// The run fake's odds weigh 1 + this (`fake_chance`).
    pub play_action: f64,
    /// Added to the gun lean: more shotgun (+) or under center (-).
    pub gun: f64,
    /// A personnel group the caller goes to more (21 for a back promised a
    /// fullback, 12 for a tight end promised the Y).
    pub personnel: Option<Personnel>,
    /// Run families (inside zone, outside zone, gap, option) weigh 1 + this.
    pub family: [f64; 4],
    /// The quarterback's designed runs weigh 1 + this; below 0, none.
    pub qb_runs: f64,
    /// Screens, the quick game (and RPO throws), jet sweeps and seven-man
    /// protections weigh 1 + this.
    pub screens: f64,
    pub quick: f64,
    pub jet: f64,
    pub protect: f64,
}

/// A run concept's family for the promises on usage (8.4): 0 inside zone
/// (with the zone read), 1 outside zone (with the toss), 2 gap (power,
/// counter, duo, trap, iso, buck sweep, QB power), 3 option; None for the
/// draw, the sneak and the jet sweep.
pub fn run_family(rc: RunConcept) -> Option<usize> {
    use RunConcept::*;
    match rc {
        InsideZone | ZoneRead => Some(0),
        OutsideZone | Toss => Some(1),
        Power | Counter | Duo | Trap | Iso | BuckSweep | QbPower => Some(2),
        SpeedOption | TripleOption => Some(3),
        Draw | Sneak | JetSweep => None,
    }
}

/// The quarterback carries by design (a sneak, QB power, the zone read's
/// keep, the speed option): what "no designed runs" takes off the sheet.
pub fn qb_designed(rc: RunConcept) -> bool {
    matches!(rc, RunConcept::Sneak | RunConcept::QbPower | RunConcept::ZoneRead | RunConcept::SpeedOption)
}

/// How hard the caller goes to the personnel group your word needs, as a
/// head coach's firm guideline does (proposed, not confirmed).
pub const PERSONNEL_LEAN: f64 = 0.8;

/// The designed plays on a team's sheets, dry and wet, first eight.
pub fn sheet_designs(prep: &super::roster::Prep) -> [Option<(u16, Personnel, Backfield, Set)>; 8] {
    let mut out: [Option<(u16, Personnel, Backfield, Set)>; 8] = [None; 8];
    let mut n = 0;
    for &(p, b, set, kind) in prep.sheet.iter().chain(&prep.wet).flat_map(|x| &x.1) {
        if let PlayKind::Pass(i) = kind
            && i >= PASS_CONCEPTS.len()
            && n < out.len()
            && !out[..n].iter().flatten().any(|d| d.0 as usize == i)
        {
            out[n] = Some((i as u16, p, b, set));
            n += 1;
        }
    }
    out
}

fn weigh<T: Copy>(rng: &mut Rng, items: &[(T, f64)]) -> T {
    weigh_where(rng, items, |_| true).expect("weigh: no positive weight")
}

/// `weigh` over only the items `keep` passes, as if they were copied out
/// first: the same sum, the same draw and the same walk. None, with no
/// draw taken, when none of them has a positive weight.
fn weigh_where<T: Copy>(rng: &mut Rng, items: &[(T, f64)], keep: impl Fn(T) -> bool) -> Option<T> {
    // Rng::weighted over only the positive weights, without copying them
    // out.
    let total: f64 = items.iter().filter(|x| x.1 > 0.0 && keep(x.0)).map(|(_, w)| w).sum();
    if total <= 0.0 {
        return None;
    }
    let mut roll = rng.next() * total;
    let mut last = None;
    for &(item, w) in items.iter().filter(|x| x.1 > 0.0 && keep(x.0)) {
        roll -= w;
        if roll < 0.0 {
            return Some(item);
        }
        last = Some(item);
    }
    last
}

/// What a memo is keyed on: the situation's slot in the caller's own list,
/// then only what the weights read of the snap.
type ConceptKey = (usize, bool, bool, Option<Personnel>, u8, bool);
type RunKey = (usize, bool, bool, bool);
type CoverageKey = (usize, usize, Personnel, DefPersonnel);

/// The weights a play caller has already worked out for a snap like this
/// one (the coordinators' draft at kickoff calls thousands of snaps from
/// one memory in a few situations). A memo holds for one staff, memory and
/// context only. It keeps the weights as they were worked out, so a call
/// draws from the same list with or without it.
#[derive(Default)]
pub struct CallMemo {
    concepts: Vec<(ConceptKey, Vec<(usize, f64)>)>,
    /// What the fake adds against the looks this caller expects
    /// (`fake_gain`): the same for every snap of one memory.
    fake_gain: Option<f64>,
    runs: Vec<(RunKey, Vec<(RunConcept, f64)>)>,
    coverages: Vec<(CoverageKey, [(Coverage, f64); COVERAGE_COUNT])>,
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

/// A straight drop's share under center, of the system's own under-center
/// lean, how far the caller's tell moves it for each point it is off the
/// typical caller's, and the hurry-up's share of it (proposed, not
/// confirmed: 5.9% of the test league's straight drops, a league with more
/// under-center systems than the world's, 4.5% on the world's mix; real
/// 4.0% of straight drops and 18.7% of screens, 5.6% together, FTN 2022-25).
const DROP_UC_SHARE: f64 = 0.08;
const DROP_UC_TELL: f64 = 4.0;
const TELL_TYPICAL: f64 = 0.2;
const DROP_UC_HURRY: f64 = 0.25;

/// The offensive call.
pub fn call_offense(staff: &Staff, mem: &Memory, sit: &Situation, ctx: &OffCtx, rng: &mut Rng) -> OffCall {
    call_offense_memo(staff, mem, sit, ctx, rng, None)
}

/// The offensive call with a memo of the weights already worked out for
/// this staff, memory and context (`CallMemo`), and this snap's slot in
/// the caller's list of situations.
pub fn call_offense_memo(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    ctx: &OffCtx,
    rng: &mut Rng,
    memo: Option<(&mut CallMemo, usize)>,
) -> OffCall {
    let c = &staff.offense;
    let scheme = staff.off_scheme;
    let year = staff.year;
    // A try from scrimmage is no down: it calls from the real tries' mix
    // (`TRY_PASS`, lane EG6), not from first and goal.
    let mut p_pass = if sit.trying {
        TRY_PASS
    } else {
        tables::real_pass_rate(sit.down, sit.togo, sit.score_diff, sit.clock(), sit.college)
    };
    // The short field (lane EG6): inside the 20 real offenses run more than
    // the table's down, distance, score and clock say, most on first down.
    if sit.yardline_100 <= 20.0 && !sit.trying {
        let z = if sit.yardline_100 <= 5.0 {
            0
        } else if sit.yardline_100 <= 10.0 {
            1
        } else if sit.yardline_100 <= 15.0 {
            2
        } else {
            3
        };
        p_pass = lean_odds(p_pass, RED_ZONE_PASS[z][usize::from(sit.down.clamp(1, 3) - 1)]);
    }
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
        // His habit as he calls it here: the league's book leans too.
        let own = super::guide::habit_pass(sit.college, scheme, c.aggressiveness) + sit.lean;
        p_pass += k * ((1.0 - run) - own);
    }
    // A line in your plan (G20): run it more.
    p_pass -= mem.plan.run_more;
    // Your word to the quarterback (the promises on usage): throw more.
    p_pass += ctx.lean.pass;
    // Bad weather: more runs, the same throws (`Game::weather_lean`).
    p_pass -= sit.weather;
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
    let short = short_call(sit);
    let hurry = sit.hurry();
    let tell = tell_of(c);
    // The run fake (scheme-fit item 3): decided with the throw, before the
    // look, so a fake comes from the run's own personnel and backfield (real
    // NFL 2024: 53% of play-action dropbacks under center, as 51% of runs;
    // 4% of straight drops). A straight drop is `drop`.
    let mut memo = memo;
    let p_fake = fake_chance(staff, mem, sit, ctx, memo.as_mut().map(|(m, _)| &mut **m));
    let mut fake = pass && rng.chance(p_fake);
    let drop = pass && !fake;
    let want = wanted(staff, ctx);
    // The group fits the call (lane SF2, item 1): the fake comes from the
    // run's groups, not the dropback's. 2024 NFL on 1st and 10, screens,
    // RPOs and sneaks left out: 12 personnel on 34% of runs, 37% of play
    // action and 24% of straight dropbacks; 13 on 5.5%, 5.8% and 2.9%
    // (nflverse with FTN charting).
    let personnel =
        weigh(rng, Groups::of(scheme, short, hurry, drop, tell, ctx, want, pass, trust(c.creativity)).all());
    let gun = rng.chance(gun_chance(scheme, ctx, drop, tell, hurry));
    // As `backfield_chance` reads it back. A two-back package goes under
    // center on a run or a fake; on a straight drop it lines up in the gun
    // as the draw above says (minor bug 411).
    let backfield = if matches!(personnel, Personnel::P21 | Personnel::P22) && !hurry && !drop {
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
    // No fake from empty (no back to fake to): the straight drops carry the
    // empty sets, so a dropback is in empty as often as before.
    let set =
        pick_set(scheme, personnel, drop, short, year, if drop { 1.0 / (1.0 - p_fake).max(0.1) } else { 1.0 }, rng);
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
            fake = formation.set != Set::Empty && rng.chance(p_fake);
            why = "breaking our run tendency from this look";
        } else if pass && p_run <= 0.15 && rng.chance(flip) {
            pass = false;
            fake = false;
            why = "breaking our pass tendency from this look";
        }
    }
    formation.motion = pick_motion(scheme, &formation, pass, year, ctx, rng);
    let mut call = if pass {
        let concept = pick_concept(staff, mem, sit, &formation, ctx, fake, rng, memo);
        // A designed play runs from the set it was drawn in.
        if let Some(&(_, _, b, set)) = ctx.designs.iter().flatten().find(|d| d.0 as usize == concept) {
            formation.backfield = b;
            formation.set = set;
        }
        // Empty needs a quick answer or a full progression; it can't use
        // six- or seven-man protection.
        if formation.set == Set::Empty && pass_concept(concept).protection != Protection::Standard {
            formation.set = Set::Trips;
        }
        let mut call = OffCall::new(formation, PlayKind::Pass(concept), strength, hurry);
        call.fake = fake && takes_fake(pass_concept(concept)) && formation.set != Set::Empty;
        // An RPO comes off the pass side of the sheet too: real ones
        // replace runs and quick throws alike, and the real pass rates
        // count their throws as passes and their handoffs as runs. Drawn
        // from runs alone, their throws came on top of the real rate (FBS
        // teams up 28+ passed 32% in the fourth quarter, real 28%; minor
        // bug 142, proposed, not confirmed).
        let pc = pass_concept(concept);
        if !call.play_action()
            && pc.depth != PassDepth::Screen
            && let Some(i) = pick_rpo(staff, sit, &formation, RunConcept::InsideZone, ctx, short, rng)
        {
            call.kind = PlayKind::Rpo(RunConcept::InsideZone, i);
            why = "a quick throw with a run tagged on for the defender in conflict";
        }
        call
    } else {
        if formation.set == Set::Empty {
            formation.set = Set::Trips;
        }
        let rc = pick_run(staff, mem, sit, &mut formation, ctx, short, rng, memo);
        // A jet sweep goes the way the motion man runs: across, away from
        // where he lined up.
        let dir = if rc == RunConcept::JetSweep || rc == RunConcept::Counter || rng.chance(0.3) {
            -strength
        } else {
            strength
        };
        let mut call = OffCall::new(formation, PlayKind::Run(rc), dir, hurry);
        // An RPO (topic 1): the same run with a quick throw attached.
        if let Some(i) = pick_rpo(staff, sit, &call.formation, rc, ctx, short, rng) {
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

/// A short-yardage call: third or fourth and two or less, or the ball
/// inside the three.
pub(crate) fn short_call(sit: &Situation) -> bool {
    sit.togo <= 2.0 && sit.down >= 3 || sit.yardline_100 <= 3.0
}

/// A tell (topic 7.1): how far a predictable caller's personnel and
/// formation give the play away; a creative one shows the same looks for
/// both.
fn tell_of(c: &Caller) -> f64 {
    clamp(0.33 - 0.013 * c.creativity, 0.07, 0.33)
}

/// The group the call leans to: the head coach's personnel identity
/// (update 2, G2), else the one your word needs.
fn wanted(staff: &Staff, ctx: &OffCtx) -> Option<(Personnel, f64)> {
    staff.guide.personnel.pull().or(ctx.lean.personnel.map(|p| (p, PERSONNEL_LEAN)))
}

/// His chance of lining up in the gun or the pistol from it: the scheme's
/// lean, more below the NFL and in the hurry-up; a run or a fake away from
/// it by his tell. A straight drop is nearly always from the gun whatever
/// the system (minor bug 411; real NFL 2022-25: 4% under center for drops,
/// 58% for runs and play action): the system's share of snaps under
/// center, a tenth of it, led by the caller's tell.
fn gun_chance(scheme: OffScheme, ctx: &OffCtx, drop: bool, tell: f64, hurry: bool) -> f64 {
    let lean = match scheme {
        OffScheme::AirRaid | OffScheme::SpreadOption => 0.92,
        OffScheme::WestCoast => 0.6,
        OffScheme::WideZone => 0.35,
        OffScheme::PowerGap => 0.4,
        OffScheme::WingT => 0.05,
    } + if ctx.level == Level::NFL { 0.0 } else { 0.08 }
        + ctx.lean.gun;
    if drop {
        let uc = DROP_UC_SHARE
            * (1.0 - lean)
            * clamp(1.0 + DROP_UC_TELL * (TELL_TYPICAL - tell), 0.3, 1.8)
            * if hurry { DROP_UC_HURRY } else { 1.0 };
        return 1.0 - clamp(uc, 0.0, 0.6);
    }
    clamp(lean - tell + if hurry { 0.3 } else { 0.0 }, 0.0, 0.98)
}

/// The chance of the backfield the call draws (`call_offense_memo`) when
/// he leans to the gun with chance `gun`: 21 and 22 under center but in the
/// hurry-up and on a straight drop (`drop`), a fifth of the gun's snaps from the pistol in the wide-zone
/// and spread-option schemes, and three in ten of wide zone's other snaps.
fn backfield_chance(scheme: OffScheme, p: Personnel, hurry: bool, drop: bool, gun: f64, b: Backfield) -> f64 {
    if matches!(p, Personnel::P21 | Personnel::P22) && !hurry && !drop {
        return if b == Backfield::UnderCenter { 1.0 } else { 0.0 };
    }
    let gun_pistol = if matches!(scheme, OffScheme::WideZone | OffScheme::SpreadOption) { 0.2 } else { 0.0 };
    let under_pistol = if scheme == OffScheme::WideZone { 0.3 } else { 0.0 };
    match b {
        Backfield::Shotgun => gun * (1.0 - gun_pistol),
        Backfield::Pistol => gun * gun_pistol + (1.0 - gun) * under_pistol,
        Backfield::UnderCenter => (1.0 - gun) * (1.0 - under_pistol),
    }
}

/// The chance a throw the call sheet names comes off a run fake
/// (scheme-fit item 3, lane SF2): the chance the drafted call that put it
/// on the sheet had one, from what a sheet line keeps of it (its group,
/// backfield, set and concept; it keeps no fake). The call decides the fake
/// with the throw, then fits the group, the backfield and the set to it and
/// draws the concept, so each tells: Bayes' rule on the call's own chances
/// in this snap's situation (`call_offense_memo`). A typical NFL caller's
/// throw from 13 personnel under center comes off a fake far more often
/// than one from 11 in the gun. Before, every sheet throw took his chance
/// of a fake on any throw here, or the fake of his own call when it was a
/// throw, whatever the line's look. Two rare turns are left out: the
/// self-scout flip, which a draft never makes, and an empty set's switch to
/// trips for a concept that needs six or seven to protect.
pub fn sheet_fake_chance(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    ctx: &OffCtx,
    f: &Formation,
    concept: usize,
) -> f64 {
    let pc = pass_concept(concept);
    if !takes_fake(pc) || f.set == Set::Empty {
        return 0.0;
    }
    let p_fake = fake_chance(staff, mem, sit, ctx, None);
    let scheme = staff.off_scheme;
    let short = short_call(sit);
    let hurry = sit.hurry();
    let tell = tell_of(&staff.offense);
    let want = wanted(staff, ctx);
    let share = |items: &[(Personnel, f64)], p: Personnel| {
        let total: f64 = items.iter().map(|x| x.1).sum();
        if total > 0.0 { items.iter().filter(|x| x.0 == p).map(|x| x.1).sum::<f64>() / total } else { 0.0 }
    };
    // How likely the line's look and concept are from a drop (`drop`) or a
    // fake.
    let like = |drop: bool| {
        let cr = trust(staff.offense.creativity);
        let group = share(Groups::of(scheme, short, hurry, drop, tell, ctx, want, true, cr).all(), f.personnel);
        let back =
            backfield_chance(scheme, f.personnel, hurry, drop, gun_chance(scheme, ctx, drop, tell, hurry), f.backfield);
        let empty = if drop { 1.0 / (1.0 - p_fake).max(0.1) } else { 1.0 };
        let sets = set_weights(scheme, f.personnel, drop, short, staff.year, empty);
        let total: f64 = sets.iter().map(|x| x.1).sum();
        let set = sets.iter().find(|x| x.0 == f.set).map_or(0.0, |x| x.1) / total.max(1e-12);
        // A straight drop's throw past the screens can still turn into an
        // RPO, and then the sheet would name the RPO.
        let rpo = if drop && pc.depth != PassDepth::Screen {
            rpo_chance(staff, sit, f, RunConcept::InsideZone, ctx, short).unwrap_or(0.0)
        } else {
            0.0
        };
        group * back * set * (1.0 - rpo) * concept_share(staff, mem, sit, f, ctx, !drop, concept)
    };
    let faked = p_fake * like(false);
    let straight = (1.0 - p_fake) * like(true);
    if faked + straight > 0.0 { faked / (faked + straight) } else { p_fake }
}

/// How often this caller throws off a run fake here (scheme-fit item 3;
/// Cain, 2026-11-17: any dropback play can carry the fake): the league's
/// real habit for the down, distance, score, clock and field
/// (`tables::real_play_action_rate`), leaned by the scheme and your word to
/// a back or quarterback promised play action, on the odds.
/// A sharp caller fakes more against the looks the fake beats (soft zones
/// whose linebackers read the run) and less against the ones it doesn't (a
/// blitz meets a quarterback with his back turned): the coaches' table's
/// faked rows against the straight ones, as he reads the defense.
fn fake_chance(staff: &Staff, mem: &Memory, sit: &Situation, ctx: &OffCtx, memo: Option<&mut CallMemo>) -> f64 {
    let real = tables::real_play_action_rate(sit.down, sit.togo, sit.yardline_100, sit.score_diff, sit.clock());
    let c = &staff.offense;
    let gain = match memo {
        Some(m) => *m.fake_gain.get_or_insert_with(|| fake_gain(mem, (c.reading + c.adaptation) / 40.0)),
        None => fake_gain(mem, (c.reading + c.adaptation) / 40.0),
    };
    let odds = real / (1.0 - real).max(1e-6)
        * fake_lean(staff.off_scheme)
        * (1.0 + ctx.lean.play_action)
        * (trust(c.reading) * gain).exp();
    odds / (1.0 + odds)
}

/// What the fake adds against the looks this caller expects, beyond what
/// it adds on average: the mean over the concepts that carry it. What it
/// adds on average is the league's habit (`tables::real_play_action_rate`).
fn fake_gain(mem: &Memory, sharp: f64) -> f64 {
    let (with, without) = (expected_values(mem, sharp, true), expected_values(mem, sharp, false));
    let avg = &values().fake_gain;
    let (mut sum, mut n) = (0.0, 0.0);
    for (i, pc) in PASS_CONCEPTS.iter().enumerate() {
        if takes_fake(pc) {
            sum += with[i] - without[i] - avg[i];
            n += 1.0;
        }
    }
    sum / f64::max(n, 1.0)
}

/// How much more than the league each system fakes, on the odds: wide zone
/// lives off play action and the boot, the Air Raid and the spread barely
/// sell a run. Sized so the systems spread over the real teams' range
/// (NFL 2022-25 team seasons: 15% to 35% of dropbacks; proposed, not
/// confirmed).
fn fake_lean(scheme: OffScheme) -> f64 {
    match scheme {
        OffScheme::WestCoast => 1.0,
        OffScheme::AirRaid => 0.55,
        OffScheme::WideZone => 1.75,
        OffScheme::PowerGap => 1.35,
        OffScheme::SpreadOption => 0.7,
        OffScheme::WingT => 1.6,
    }
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
            } else if call.play_action() {
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

/// The groups a call draws its personnel from (lane SF2, item 1), each one
/// the roster can field (`available`), with its weight: at most eight, on
/// the stack, since every call works them out.
struct Groups {
    items: [(Personnel, f64); 8],
    n: usize,
}

impl Groups {
    /// The call's groups (lane SF2, item 1). Short yardage and the
    /// hurry-up keep their own. On other downs: the scheme's weights, the
    /// head coach's personnel identity (update 2, G2) or the group your
    /// word needs as a lean on top (its firmness times the table's whole
    /// weight), then each group's fit to the call. Each wide receiver a
    /// group trades for a blocker (`Personnel::heavy_fit`) leans it toward
    /// the run's looks, runs and throws off a run fake alike, by the
    /// caller's tell, and away from a straight dropback (`drop`) by the
    /// same: a predictable caller matches his groups to his calls, a
    /// creative one shows the same looks for both. The fit is the fielded
    /// group's, so a lean to 13 with two healthy tight ends fits like 12.
    ///
    /// Then each group leans to what his own men win in it (lane SF10, item
    /// 23: `roster_fit`), the call's runs or passes (a fake is a pass), by
    /// the caller's creativity (`cr`): tight ends who block add to 12, 13
    /// and 22 on runs, a third receiver who gets open to 11 and 10 on
    /// throws.
    #[allow(clippy::too_many_arguments)]
    fn of(
        scheme: OffScheme,
        short: bool,
        hurry: bool,
        drop: bool,
        tell: f64,
        ctx: &OffCtx,
        want: Option<(Personnel, f64)>,
        pass: bool,
        cr: f64,
    ) -> Groups {
        let mut g = Groups::of_table(scheme, short, hurry, drop, tell, ctx, want);
        if cr > 0.0 && ctx.fit != super::roster_fit::OffFit::default() {
            for x in &mut g.items[..g.n] {
                x.1 *= (cr * ctx.fit.group_of(x.0, pass)).exp();
            }
        }
        g
    }

    /// The call's groups before the roster's lean.
    #[allow(clippy::too_many_arguments)]
    fn of_table(
        scheme: OffScheme,
        short: bool,
        hurry: bool,
        drop: bool,
        tell: f64,
        ctx: &OffCtx,
        want: Option<(Personnel, f64)>,
    ) -> Groups {
        let has_fb = ctx.has_fb;
        let fb = |w: f64| if has_fb { w } else { 0.0 };
        // The NFL rarely goes to four receivers: 10 personnel is 1.0% of real
        // dropbacks, against 72% in 11 and 18% in 12 (nflverse participation
        // 2023-24); college and high school spread teams live in it. NFL
        // callers weigh it at a tenth and stay in 11 when they hurry (minor bug
        // 137: 19% of NFL dropbacks came from 10 personnel; proposed, not
        // confirmed).
        let nfl = ctx.level == Level::NFL;
        let p10 = if nfl { 0.1 } else { 1.0 };
        if hurry {
            let p11 = if nfl { 0.95 } else { 0.7 };
            return Groups::new(&[(Personnel::P11, p11), (Personnel::P10, 1.0 - p11)], ctx);
        }
        if short {
            let te3 = if ctx.te_count >= 3 { 1.5 } else { 0.0 };
            return Groups::new(
                &[(Personnel::P12, 3.0), (Personnel::P22, fb(2.0)), (Personnel::P13, te3), (Personnel::P21, fb(1.5))],
                ctx,
            );
        }
        let table: &[(Personnel, f64)] = match scheme {
            OffScheme::AirRaid => &[(Personnel::P10, p10 * 4.0), (Personnel::P11, 6.0)],
            OffScheme::WestCoast => &[(Personnel::P11, 6.0), (Personnel::P12, 2.5), (Personnel::P21, fb(1.5))],
            OffScheme::WideZone => &[(Personnel::P11, 5.0), (Personnel::P12, 2.5), (Personnel::P21, fb(2.5))],
            OffScheme::PowerGap => {
                &[(Personnel::P11, 4.0), (Personnel::P12, 3.5), (Personnel::P21, fb(2.0)), (Personnel::P22, fb(0.8))]
            }
            OffScheme::SpreadOption => &[(Personnel::P11, 6.0), (Personnel::P10, p10 * 3.0)],
            OffScheme::WingT => &[(Personnel::P21, fb(5.0)), (Personnel::P22, fb(2.0)), (Personnel::P12, 3.0)],
        };
        let mut g = Groups::new(table, ctx);
        if let Some((p, k)) = want {
            let total: f64 = table.iter().map(|x| x.1).sum();
            g.add(available(p, ctx), k * total);
        }
        // The wing-T plays from its own sets whatever the call. For any
        // other caller, a group's share of the straight dropbacks over its
        // share of the runs and fakes, against 11's, is 0.47 for 12 and
        // 21, 0.34 for 13 and 0.19 for 22 for the NFL's typical caller
        // (creativity 14, tell 0.15). Real 2024 NFL on first and ten, where
        // the down doesn't pick the group (nflverse with FTN charting;
        // screens, RPOs and sneaks left out): 0.46 for 12, 0.47 for 21,
        // 0.34 for 13 and 0.20 for 22. Over every normal down real teams
        // also line up light to run on passing downs (0.31, 0.22, 0.19 and
        // 0.09), which the fit doesn't read (minor bug 418).
        if scheme != OffScheme::WingT {
            let h = (1.0 + if drop { -2.0 * tell } else { 2.0 * tell }).max(0.2);
            for x in &mut g.items[..g.n] {
                x.1 *= x.0.heavy_fit(h);
            }
        }
        g
    }

    /// The table's groups as the roster can field them, the same group's
    /// weights together.
    fn new(table: &[(Personnel, f64)], ctx: &OffCtx) -> Groups {
        let mut g = Groups { items: [(Personnel::P11, 0.0); 8], n: 0 };
        for &(p, w) in table {
            g.add(available(p, ctx), w);
        }
        g
    }

    fn add(&mut self, p: Personnel, w: f64) {
        match self.items[..self.n].iter_mut().find(|x| x.0 == p) {
            Some(x) => x.1 += w,
            None => {
                self.items[self.n] = (p, w);
                self.n += 1;
            }
        }
    }

    fn all(&self) -> &[(Personnel, f64)] {
        &self.items[..self.n]
    }
}

/// The group the roster can field: fewer than two healthy tight ends make
/// 12, 22 and 13 into 11, fewer than three make 13 into 12, and 21 and 22
/// need a fullback.
fn available(personnel: Personnel, ctx: &OffCtx) -> Personnel {
    let has_fb = ctx.has_fb;
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
fn pick_set(
    scheme: OffScheme,
    p: Personnel,
    pass: bool,
    short: bool,
    year: i32,
    empty_scale: f64,
    rng: &mut Rng,
) -> Set {
    if short {
        return if matches!(p, Personnel::P12 | Personnel::P21 | Personnel::P22) && rng.chance(0.4) {
            Set::Wing
        } else {
            Set::Doubles
        };
    }
    weigh(rng, &set_weights(scheme, p, pass, short, year, empty_scale))
}

/// Each set's weight in `pick_set` (short yardage's chance of the wing as
/// one), for the call and for reading a drafted look back
/// (`sheet_fake_chance`).
fn set_weights(
    scheme: OffScheme,
    p: Personnel,
    pass: bool,
    short: bool,
    year: i32,
    empty_scale: f64,
) -> [(Set, f64); SET_COUNT] {
    if short {
        let wing = if matches!(p, Personnel::P12 | Personnel::P21 | Personnel::P22) { 0.4 } else { 0.0 };
        return [
            (Set::Doubles, 1.0 - wing),
            (Set::Trips, 0.0),
            (Set::Bunch, 0.0),
            (Set::Stack, 0.0),
            (Set::Empty, 0.0),
            (Set::Wing, wing),
        ];
    }
    let wr = p.wide_receivers();
    let modern = era(year, 2008, 14.0);
    let spread = matches!(scheme, OffScheme::AirRaid | OffScheme::WestCoast | OffScheme::SpreadOption);
    let empty = if pass && wr >= 3 && spread { (0.06 + 0.06 * modern) * empty_scale } else { 0.0 };
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
    [
        (Set::Doubles, doubles),
        (Set::Trips, trips),
        (Set::Bunch, bunch),
        (Set::Stack, stack),
        (Set::Empty, empty),
        (Set::Wing, wing),
    ]
}

/// Motion (topic 3.1): about a quarter of snaps before 2015, more than half
/// by the 2020s (NFL Next Gen Stats); jet motion is about a third of it.
/// High school teams motion less (proposed, not confirmed).
fn pick_motion(scheme: OffScheme, f: &Formation, pass: bool, year: i32, ctx: &OffCtx, rng: &mut Rng) -> Motion {
    if f.set == Set::Empty {
        return Motion::None;
    }
    let rate = (0.22 + 0.33 * era(year, 2014, 9.0)) * if ctx.level == Level::HS { 0.6 } else { 1.0 };
    if !rng.chance(rate) {
        return Motion::None;
    }
    // A team with a burner runs more jet motion to get him the ball (S10).
    let jet =
        if matches!(scheme, OffScheme::WideZone | OffScheme::WingT | OffScheme::SpreadOption) { 1.5 } else { 1.0 }
            * (1.0 + ctx.burner);
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

#[allow(clippy::too_many_arguments)]
fn pick_run(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    f: &mut Formation,
    ctx: &OffCtx,
    short: bool,
    rng: &mut Rng,
    memo: Option<(&mut CallMemo, usize)>,
) -> RunConcept {
    use RunConcept::*;
    let backfield = f.backfield;
    // A promise of no designed runs (`Lean::qb_runs` below zero) takes the
    // sneak away too; the draw stays, so every other game is the same.
    if short && rng.chance(0.25) && backfield == Backfield::UnderCenter && ctx.lean.qb_runs >= 0.0 {
        return Sneak;
    }
    let rc = match memo {
        None => weigh(rng, &run_weights(staff, mem, f, ctx, short)),
        Some((m, slot)) => {
            // The weights read the formation only for the gun, a fullback's
            // personnel and jet motion; `short` comes from the situation.
            let fb = ctx.has_fb && matches!(f.personnel, Personnel::P21 | Personnel::P22);
            let key = (slot, backfield != Backfield::UnderCenter, fb, f.motion == Motion::Jet);
            let k = match m.runs.iter().position(|x| x.0 == key) {
                Some(k) => k,
                None => {
                    m.runs.push((key, run_weights(staff, mem, f, ctx, short)));
                    m.runs.len() - 1
                }
            };
            weigh(rng, &m.runs[k].1)
        }
    };
    // A jet sweep needs the jet motion; a sneak or QB power is called from
    // the gun or under center as it comes.
    if rc == JetSweep {
        f.motion = Motion::Jet;
    }
    let _ = sit;
    rc
}

/// Each run concept's weight on this snap, before `weigh` draws from them.
/// It draws nothing and reads only the staff, memory, context, `short` and
/// the formation's gun, personnel and motion.
fn run_weights(staff: &Staff, mem: &Memory, f: &Formation, ctx: &OffCtx, short: bool) -> Vec<(RunConcept, f64)> {
    use RunConcept::*;
    let scheme = staff.off_scheme;
    let c = &staff.offense;
    let backfield = f.backfield;
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
    // Your word (the promises on usage): the run family built on him, the
    // quarterback's designed runs, jet sweeps for the man in motion.
    let l = &ctx.lean;
    if l.family != [0.0; 4] || l.qb_runs != 0.0 || l.jet != 0.0 {
        let others = w.iter().filter(|x| !qb_designed(x.0) && x.1 > 0.0).count();
        for x in w.iter_mut() {
            if let Some(k) = run_family(x.0) {
                x.1 *= 1.0 + l.family[k];
            }
            if qb_designed(x.0) {
                if l.qb_runs < 0.0 && others > 0 {
                    x.1 = 0.0;
                } else if l.qb_runs > 0.0 {
                    x.1 *= 1.0 + l.qb_runs;
                }
            }
            if x.0 == JetSweep {
                x.1 *= 1.0 + l.jet;
            }
        }
    }
    // What the numbers say works against the fronts this defense shows
    // (topic 2), and what has worked today (topic 3.2), at his skill.
    // And what each run gains from his own men (lane SF10, items 10 and
    // 23: `roster_fit`), in the same yards, by his creativity: linemen and
    // tight ends who drive their men add most to the runs that lean on
    // them, a fast back to the toss and duo, a quarterback who can run to
    // his own runs.
    let fronts = &mem.fronts_seen;
    let ft: f64 = fronts.iter().sum::<f64>() + 1.0;
    let tr = trust(c.reading);
    let ad = c.adaptation / 20.0;
    let cr = trust(c.creativity);
    let table = run_values();
    for x in w.iter_mut() {
        let k = x.0 as usize;
        let v = table.get(k).copied().unwrap_or([0.0; 6]);
        let ev: f64 = (0..6).map(|f| (fronts[f] + 1.0 / 6.0) / ft * v[f]).sum();
        let [sum, n] = mem.run_res[k];
        let today = if n >= 3.0 { sum / n } else { 0.0 };
        let own = ctx.fit.run.get(k).copied().unwrap_or(0.0) as f64;
        x.1 *= (tr * 1.5 * ev + ad * 0.8 * today + cr * 1.5 * own).exp();
    }
    w
}

/// The pass half of an RPO (topic 1): real college spread offenses tag a
/// throw on a large share of their runs since the 2010s; NFL teams far
/// fewer (their linemen may not be past one yard downfield). His chance of
/// tagging one onto this call, or None when the call takes none: short
/// yardage, the hurry-up, a run that isn't a downhill one, a group short of
/// two wide receivers as the rule book fields it, an empty set, or a group
/// that can run none of the RPO throws (`readable`).
fn rpo_chance(
    staff: &Staff,
    sit: &Situation,
    formation: &Formation,
    rc: RunConcept,
    ctx: &OffCtx,
    short: bool,
) -> Option<f64> {
    let fielded = formation.personnel.for_players(ctx.players);
    if short
        || sit.hurry()
        || !matches!(rc, RunConcept::InsideZone | RunConcept::Duo | RunConcept::Power | RunConcept::Counter)
        || fielded.wide_receivers() < 2
        || formation.set == Set::Empty
        || !rpo_throws().iter().any(|t| readable_library(fielded) >> t.0 & 1 == 1)
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
    Some(scheme * level * era(staff.year, 2010, 8.0) * (1.0 + ctx.lean.quick))
}

/// The RPO throws (their concept numbers) and their weights.
fn rpo_throws() -> [(usize, f64); 3] {
    static THROWS: std::sync::OnceLock<[(usize, f64); 3]> = std::sync::OnceLock::new();
    *THROWS.get_or_init(|| [(concept("RPO glance"), 1.0), (concept("RPO bubble"), 1.0), (concept("RPO slant"), 0.8)])
}

/// The throw tagged onto the call, if he tags one (`rpo_chance`): among the
/// RPO throws the group can run as the rule book fields it (`readable`, one
/// of its reads on the field). Before, 12 and 21 personnel drew the bubble,
/// whose slots they don't have, and the snap played as a plain run filmed
/// as an RPO.
fn pick_rpo(
    staff: &Staff,
    sit: &Situation,
    formation: &Formation,
    rc: RunConcept,
    ctx: &OffCtx,
    short: bool,
    rng: &mut Rng,
) -> Option<usize> {
    let p = rpo_chance(staff, sit, formation, rc, ctx, short)?;
    if !rng.chance(p) {
        return None;
    }
    let library = readable_library(formation.personnel.for_players(ctx.players));
    weigh_where(rng, &rpo_throws(), |i| library >> i & 1 == 1)
}

/// Third or fourth down past short yardage: the caller weighs the routes
/// that reach the sticks (topic 6.2).
fn reads_sticks(sit: &Situation) -> bool {
    sit.down >= 3 && sit.togo > 3.0
}

/// How deep a concept's first two reads break as the group lines up, for
/// third down: a second or third tight end in a missing receiver's place
/// is read where he stands and runs his own route (lane SF2, `reads_in`,
/// `route_in`).
pub(crate) fn sticks_depth(c: &PassConcept, f: &Formation) -> f64 {
    reads_of(c, f.personnel)
        .take(2)
        .map(|r| {
            let rt = route_in(c, f, r);
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

/// The pass concept, among the concepts the group can run as the rule
/// book fields it (lane SF2): at least one of its reads on the field (13
/// and 22 personnel have no flanker for the tunnel screen; the CFL's and
/// arena football's sets for them do).
#[allow(clippy::too_many_arguments)]
fn pick_concept(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    f: &Formation,
    ctx: &OffCtx,
    fake: bool,
    rng: &mut Rng,
    memo: Option<(&mut CallMemo, usize)>,
) -> usize {
    let fielded = Formation { personnel: f.personnel.for_players(ctx.players), ..*f };
    // The library's concepts by their bits (every pass call reads them);
    // designed plays as they are.
    let library = readable_library(fielded.personnel);
    let fits = |i: usize| {
        if i < PASS_CONCEPTS.len() { (library >> i) & 1 == 1 } else { readable(pass_concept(i), &fielded) }
    };
    // Most groups can run every concept and design: then the draw is the
    // plain one, the same draw without asking each concept.
    let every =
        library == LIBRARY_ALL && ctx.designs.iter().flatten().all(|d| readable(pass_concept(d.0 as usize), &fielded));
    let draw = |rng: &mut Rng, w: &[(usize, f64)]| {
        if every { weigh(rng, w) } else { weigh_where(rng, w, fits).unwrap_or_else(|| weigh(rng, w)) }
    };
    let Some((m, slot)) = memo else {
        return draw(rng, &concept_weights(staff, mem, sit, f, ctx, fake));
    };
    // The weights read the formation only for a boot from the gun, an empty
    // set and, with designs on the sheet or on third and fourth down (the
    // routes that reach the sticks), the personnel; on those downs also the
    // set of a group whose extra tight ends run missing receivers' routes
    // (`route_set`: 22's second tight end runs the flanker's route from the
    // wing, his own from across the ball).
    let designs = ctx.designs.iter().any(|d| d.is_some());
    let sticks = reads_sticks(sit);
    let group = (designs || sticks).then_some(f.personnel);
    let set = if sticks { route_set(&fielded) } else { 0 };
    let key = (slot, f.backfield == Backfield::Shotgun, f.set == Set::Empty, group, set, fake);
    let k = match m.concepts.iter().position(|x| x.0 == key) {
        Some(k) => k,
        None => {
            m.concepts.push((key, concept_weights(staff, mem, sit, f, ctx, fake)));
            m.concepts.len() - 1
        }
    };
    draw(rng, &m.concepts[k].1)
}

/// The concept's chance in `pick_concept`'s draw (its weight over the
/// weights of the concepts the group can run), for reading a drafted throw
/// back (`sheet_fake_chance`).
fn concept_share(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    f: &Formation,
    ctx: &OffCtx,
    fake: bool,
    concept: usize,
) -> f64 {
    let fielded = Formation { personnel: f.personnel.for_players(ctx.players), ..*f };
    let library = readable_library(fielded.personnel);
    let fits = |i: usize| {
        if i < PASS_CONCEPTS.len() { (library >> i) & 1 == 1 } else { readable(pass_concept(i), &fielded) }
    };
    let w = concept_weights(staff, mem, sit, f, ctx, fake);
    let own = |keep: &dyn Fn(usize) -> bool| {
        let total: f64 = w.iter().filter(|x| x.1 > 0.0 && keep(x.0)).map(|x| x.1).sum();
        let mine: f64 = w.iter().filter(|x| x.0 == concept && x.1 > 0.0 && keep(x.0)).map(|x| x.1).sum();
        (total > 0.0).then(|| mine / total)
    };
    // As the draw: among the concepts the group can run, else among all.
    own(&fits).or_else(|| own(&|_| true)).unwrap_or(0.0)
}

/// Each pass concept's weight on this snap, before `weigh` draws from them.
/// It draws nothing and reads only the staff, memory, situation, context,
/// the formation's backfield, set and (for designs, and on third and
/// fourth down past short yardage) personnel, and whether the throw comes
/// off a run fake: then every concept that can carry the fake and the
/// play-action concepts, else every other.
fn concept_weights(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    f: &Formation,
    ctx: &OffCtx,
    fake: bool,
) -> Vec<(usize, f64)> {
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
    let ev = expected_values(mem, sharp, fake);
    let mean = ev.iter().sum::<f64>() / ev.len() as f64;
    let tr = trust(c.reading);
    let cr = trust(c.creativity);
    let sit_skill = clamp((c.situational - 4.0) / 14.0, 0.0, 1.0);
    // Third and fourth down read the routes as the group lines up and the
    // rule book fields it (`sticks_depth`).
    let sticks = reads_sticks(sit);
    let fielded = Formation { personnel: f.personnel.for_players(ctx.players), ..*f };
    // Designed plays on this week's sheet (update 3, F27), drawn for this
    // personnel, are weighed like the library concept each looks like,
    // without the scheme's favorite lean.
    let designs =
        ctx.designs.iter().flatten().filter(|d| d.1 == f.personnel).map(|d| (d.0 as usize, pass_concept(d.0 as usize)));
    PASS_CONCEPTS
        .iter()
        .enumerate()
        .chain(designs)
        .map(|(i, pc)| {
            if is_rpo_half(pc) || pc.play_action && !fake {
                return (i, 0.0);
            }
            let like = film_concept(i);
            let own = like == i;
            let name = PASS_CONCEPTS[like].name;
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
            // The boot rolls from under center more than from the gun.
            if f.backfield == Backfield::Shotgun && name == "play-action boot" {
                w *= 0.5;
            }
            if red {
                if pc.depth == PassDepth::Deep {
                    w *= 0.3;
                }
                if matches!(name, "fades" | "flood" | "slant-flat" | "snag" | "spacing") {
                    w *= 2.0;
                }
            } else if name == "fades" {
                w *= 0.15;
            }
            if goal && pc.depth == PassDepth::Screen {
                w *= 0.3;
            }
            if hurry {
                if pc.depth == PassDepth::Screen {
                    w *= 0.3;
                }
                if matches!(name, "flood" | "curl-flat" | "smash" | "speed outs") {
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
            if own && fav.contains(&name) {
                w *= 2.2;
            }
            // What beats what (topic 2): the caller leans toward the
            // concepts that gain the most against the looks he expects, and
            // (lane SF10, item 10) toward the ones his own men win, by his
            // creativity: the arm and the deep ball on the shots, the quick
            // game behind a shaky line, the concepts whose reads his best
            // route runners win (`roster_fit`).
            // A play-action concept's own row is its faked play.
            let faked = fake && takes_fake(&PASS_CONCEPTS[like]);
            let mine = if faked { ctx.fit.fake[like] } else { ctx.fit.pass[like] } as f64;
            w *= (tr * (ev[like] - mean) + cr * mine).exp();
            // Third and fourth down (topic 6.2): a caller who knows the
            // situation calls routes that reach the sticks.
            if sticks {
                let reach = sticks_depth(pc, &fielded) >= sit.togo;
                w *= 1.0 + 0.8 * sit_skill * if reach { 1.0 } else { -0.6 };
            }
            if pc.depth == PassDepth::Deep && mem.deeps >= 3.0 {
                let e = mem.deep_epa / mem.deeps;
                w *= clamp(1.0 + e * 0.4 * c.adaptation / 20.0, 0.5, 1.6);
            }
            let agg = c.aggressiveness + 4.0 * c.nerve * sit.pressure();
            w *= 1.0 + 0.03 * (agg - 10.0) * if pc.depth == PassDepth::Deep { 1.0 } else { -0.3 };
            // A few more shots with a burner to throw them to (S10).
            if pc.depth == PassDepth::Deep {
                w *= (1.0 + 0.3 * ctx.burner) * (1.0 + ctx.shots);
            }
            // Your word (the promises on usage): screens, the quick game and
            // max protection for the men promised them (play action leans
            // the fake, `fake_chance`).
            let l = &ctx.lean;
            match pc.depth {
                PassDepth::Screen => w *= 1.0 + l.screens,
                PassDepth::Quick => w *= 1.0 + l.quick,
                _ => {}
            }
            if pc.protection != Protection::Standard {
                w *= 1.0 + l.protect;
            }
            // Up big, a little less going deep (real NFL: 11.7% of throws
            // travel 20+ yards up 28+, 13.6% in close games).
            if pc.depth == PassDepth::Deep || name == "play-action shot" {
                w *= 1.0 - 0.3 * sit.ease;
            }
            // A small book carries fewer concepts (topic 4.6), and the week's
            // plan sets how far past his favorites the caller reaches (T6.2):
            // a small plan is predictable, a big one varied.
            let book = staff.playbook * ctx.plan / super::roster::INSTALL;
            if !(own && fav.contains(&name)) {
                w *= book.clamp(0.2, 1.4);
            }
            (i, w.max(0.01))
        })
        .collect()
}

/// Football knowledge: what each concept gains against each look, measured
/// on the match engine (`call_values` in afm-tools). Pass concepts by
/// coverage and pressure; runs by front.
const CALL_VALUES_CSV: &str = include_str!("../../data/call_values.csv");

/// Column of a coverage and pressure in the table: Cover 0 with its own
/// rush (`cover0_pressure`'s mix; the column keeps its old name,
/// `Cover0/DoubleA`), then each other coverage with four rushers and with
/// a linebacker blitz.
pub(crate) fn col(cov: Coverage, blitz: bool) -> usize {
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
    /// Each concept off a run fake (its "play-action" row; a play-action
    /// concept's own row, and the straight row where the table has none),
    /// and what the fake adds to it on average.
    fake: Vec<[f64; LOOK_COLS]>,
    fake_gain: Vec<f64>,
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
        // Centered like the runs (minor bug 135): a concept is chosen
        // against a look by how much better it does there than its own
        // average, so a sharp caller calls what beats the look, not the
        // concepts that gain the most everywhere. Uncentered, the deep
        // concepts' extra yards drew the world's play callers (about 14 of
        // 20, who trust the table twice as much as a 10) to 19% of NFL
        // throws at 20+ air yards (real 11.5%).
        let read = |name: &str| {
            let row = rows.get(name)?;
            let mut v = [0.0; LOOK_COLS];
            for (j, x) in v.iter_mut().enumerate() {
                *x = get(Some(row), pass_cols[j]);
            }
            Some(v)
        };
        let pass: Vec<[f64; LOOK_COLS]> =
            PASS_CONCEPTS.iter().map(|p| read(p.name).unwrap_or([0.0; LOOK_COLS])).collect();
        let fake: Vec<[f64; LOOK_COLS]> = PASS_CONCEPTS
            .iter()
            .zip(&pass)
            .map(|(p, v)| if takes_fake(p) { read(&format!("play-action {}", p.name)).unwrap_or(*v) } else { *v })
            .collect();
        let mean = |v: &[f64; LOOK_COLS]| v.iter().sum::<f64>() / LOOK_COLS as f64;
        let all = pass.iter().map(mean).sum::<f64>() / pass.len().max(1) as f64;
        // A faked row is centered as its straight row is: off the fake, the
        // caller weighs what the fake adds to each concept (a deep cross
        // gains from linebackers who bit, a quick slant loses the time the
        // fake took) as well as what it adds against the look.
        //
        // The play-action concepts fake on every snap and have no straight
        // row, so their own level used to go out with it, and each weighed
        // like any other concept (minor bug 410): the boot, the shot and the
        // dig were about 2% of play action each, and the boot gained more
        // than play action as a whole. Their straight row is the other
        // straight concepts of their depth, the weights' own yardstick.
        let straight = |p: &PassConcept| {
            let same: Vec<f64> = PASS_CONCEPTS
                .iter()
                .zip(&pass)
                .filter(|(q, _)| takes_fake(q) && q.depth == p.depth)
                .map(|(_, v)| mean(v))
                .collect();
            same.iter().sum::<f64>() / same.len().max(1) as f64
        };
        let base: Vec<f64> = PASS_CONCEPTS
            .iter()
            .zip(&pass)
            .map(|(p, v)| if p.play_action && !is_rpo_half(p) { straight(p) } else { mean(v) })
            .collect();
        let fake_gain = fake.iter().zip(&base).map(|(f, b)| mean(f) - b).collect();
        let fake = fake.iter().zip(&base).map(|(f, b)| f.map(|x| x - (b - all))).collect();
        let pass: Vec<[f64; LOOK_COLS]> = pass.iter().map(|v| v.map(|x| x - (mean(v) - all))).collect();
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
        Values { pass, fake, fake_gain, run }
    })
}

fn call_values() -> &'static Vec<[f64; LOOK_COLS]> {
    &values().pass
}

fn fake_values() -> &'static Vec<[f64; LOOK_COLS]> {
    &values().fake
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
/// expects, off a run fake or not. A sharp caller trusts what he has seen;
/// a poor one falls back on the league's habits.
fn expected_values(mem: &Memory, sharp: f64, fake: bool) -> Vec<f64> {
    let prior = league_mix();
    let blitz = mem.rate(mem.blitzes) * sharp + 0.25 * (1.0 - sharp);
    let p: Vec<f64> = (0..COVERAGE_COUNT)
        .map(|k| sharp * (mem.cov_seen[k] + 4.0 * prior[k]) / (mem.snaps_seen + 4.0) + (1.0 - sharp) * prior[k])
        .collect();
    if fake { fake_values() } else { call_values() }
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

/// The pass concepts a coverage is weighed against: every library concept
/// but the RPO halves. Worked out once: the table and the library never
/// change, so it is the same list on every call (world speed lane).
fn live_concepts() -> &'static [usize] {
    static LIVE: std::sync::OnceLock<Vec<usize>> = std::sync::OnceLock::new();
    LIVE.get_or_init(|| (0..call_values().len()).filter(|&i| !is_rpo_half(pass_concept(i))).collect())
}

/// What each coverage gives up (four rushers) against the pass concepts a
/// defensive coordinator expects. A sharp one trusts what he has seen; a
/// poor one assumes every concept is as likely.
fn coverage_values(mem: &Memory, sharp: f64) -> [f64; COVERAGE_COUNT] {
    let t = call_values();
    let seen: f64 = mem.opp_concepts.iter().sum();
    let live = live_concepts();
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
    /// His safeties' hitter read (S-A, `scrimmage::hitter`), the mean of
    /// the two: a staff with a thumper rotates him down more.
    #[serde(default)]
    pub hitter: f64,
    /// The blitz lean from his linebackers' and corners' weight on the
    /// field (L1, C1), in blitz-rate points.
    #[serde(default)]
    pub heavy: f64,
    /// What each look gives up with the defense's own men (lane SF10,
    /// `roster_fit`): nothing for the typical team.
    #[serde(skip)]
    pub fit: super::roster_fit::DefFit,
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
    call_defense_memo(staff, mem, sit, off, ctx, rng, None)
}

/// The defensive call with a memo of the coverage weights already worked
/// out for this staff, memory and context (`CallMemo`), and this snap's
/// slot in the caller's list of situations.
pub fn call_defense_memo(
    staff: &Staff,
    mem: &Memory,
    sit: &Situation,
    off: &Formation,
    ctx: &DefCtx,
    rng: &mut Rng,
    memo: Option<(&mut CallMemo, usize)>,
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
        // His own men (lane SF10, item 23): the nickel back against the
        // third linebacker, as each group plays three receivers' runs and
        // throws (real 2024 NFL: 11 personnel throws 65%), by the
        // coordinator's creativity (`roster_fit`).
        let f = &ctx.fit.dpers;
        let given = |k: usize| RUN_SHARE_11 * f[k][0] as f64 + (1.0 - RUN_SHARE_11) * f[k][1] as f64;
        let p = lean_odds(p, trust(c.creativity) * (given(0) - given(1)));
        if rng.chance(p) { DefPersonnel::Nickel } else { DefPersonnel::Base }
    } else {
        // A nickel base front (4-2-5, 3-3-5) keeps its fifth back on the
        // field against two receivers too, less against heavy sets.
        match staff.guide.front.pull() {
            Some((b, k)) if b.nickel_base() && rng.chance(k * if wr <= 1 { 0.45 } else { 0.9 }) => DefPersonnel::Nickel,
            _ => DefPersonnel::Base,
        }
    };
    let sharp = c.adaptation / 20.0;
    // What the defense believes the offense will do from this look (topic
    // 7.2): a run tell brings the safety down, a pass tell keeps two high.
    let (p_run, sure) = mem.belief(off.look());
    let read = (c.reading + c.adaptation) / 40.0;
    // `belief` is already centred on their usual run share (update 3, F31).
    let lean_run = if ctx.tip != 0.0 { -0.5 * ctx.tip * read.max(0.6) } else { p_run * sure * read };
    // Garbage time: the leading defense keeps everything in front of it
    // (two-high, fewer man looks, no Cover 0) instead of hunting stops.
    let ease = sit.def_ease;
    // The coordinator's family of coverages, then the situation. Worked
    // out in one place so a memo can keep it: it draws nothing and reads
    // only the staff, memory, situation, context, the look, their
    // personnel and ours.
    let weights = || -> [(Coverage, f64); COVERAGE_COUNT] {
        let base = shell_mix(staff.shell);
        let opp_pass = (mem.opp_pass + 2.0) / (mem.opp_plays + 4.0);
        // Deep hurt reads this game's plays only: prep snaps carry no deep
        // results (update 3, F33).
        let deep_hurting =
            if mem.opp_game_plays > 6.0 { (mem.opp_deep_epa / mem.opp_game_plays).max(0.0) } else { 0.0 };
        let run_hurting = if mem.opp_runs > 5.0 { (mem.opp_run_epa / mem.opp_runs).max(0.0) } else { 0.0 };
        // What each coverage gives up against the passes this coordinator
        // expects (topic 2): a sharp one leans away from the coverages they
        // beat.
        let given = coverage_values(mem, read);
        let mean = given.iter().sum::<f64>() / COVERAGE_COUNT as f64;
        let tr = trust(c.reading);
        // And what each coverage gives up with his own men (lane SF10, item
        // 23: `roster_fit`), by his creativity: a room of man corners gives
        // up less in man, a free safety who reads the field less in the
        // looks that lean on him (quarters, and Cover 1 with a blitz), so
        // he calls them more; without them, more zone.
        let cr = trust(c.creativity);
        let w = base.map(|(cov, w)| {
            let own = ctx.fit.look[col(cov, false)] as f64;
            let mut w = w * (-tr * (given[cov as usize] - mean) - cr * own).exp();
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
            // A hitter at safety rotates down into the box more, a rangy
            // pair stays two high more (S-A; proposed, not confirmed).
            // 0.15 a point of his read: about +0.07 men in the box per
            // 10 lb of safety on the field, as real.
            let k = 0.15 * ctx.hitter;
            w *= if cov.high() == 2 { (-k).exp() } else { k.exp() };
            // The red zone (lane EG6): man and Cover 0 lean in as the goal
            // comes nearer, from nothing at the 20 to `RED_ZONE_MAN` and
            // `RED_ZONE_ZERO` on the log-odds at the goal line.
            let near = clamp((20.0 - sit.yardline_100) / 20.0, 0.0, 1.0);
            if cov == Coverage::Cover0 {
                // Short yardage calls Cover 0 more, long yardage less. At
                // the goal line the red zone's lean already holds it, so
                // the short step fades as the goal nears: real Cover 0 on
                // third and fourth and goal from the 1 or 2 is 50.0% of
                // dropbacks, as on every snap inside the 3 (50.5%; nflverse
                // participation data, 2023-25).
                w *= if short {
                    COVER0_SHORT.powf(1.0 - near)
                } else if long {
                    0.3
                } else {
                    1.0
                };
            }
            if cov.is_man() {
                w *= (RED_ZONE_MAN * near).exp();
            }
            if cov == Coverage::Cover0 {
                w *= (RED_ZONE_ZERO * near).exp();
            }
            if sit.hurry() && cov.high() == 2 {
                w *= 1.6;
            }
            // Two deep safeties against a burner (S10d), as teams played
            // Tyreek Hill's Dolphins in 2023.
            if cov.high() == 2 {
                w *= 1.0 + 0.8 * mem.plan.fear;
            }
            if personnel == DefPersonnel::GoalLine && !cov.is_man() {
                w *= 0.3;
            }
            (cov, w)
        });
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
        // Your word to a corner (the promises on usage): at least so much
        // man coverage.
        if mem.plan.man_floor > 0.0 {
            let all: f64 = w.iter().map(|x| x.1).sum();
            let now = w.iter().filter(|x| x.0.is_man()).map(|x| x.1).sum::<f64>() / all.max(1e-9);
            let floor = mem.plan.man_floor as f64 + 0.05;
            if now < floor {
                super::guide::steer_man(&mut w, 0.7 * (floor - now));
            }
        }
        // Your word to a corner or a safety (lane N2): at least so much two-
        // high, single-high or Cover 1 robber.
        for (k, &f) in mem.plan.shell_floor.iter().enumerate() {
            if f > 0.0 {
                let pred: fn(Coverage) -> bool = match k {
                    0 => |c| c.high() == 2,
                    1 => |c| c.high() == 1,
                    _ => |c| c == Coverage::Cover1Robber,
                };
                super::guide::steer_floor(&mut w, pred, f as f64 + 0.05);
            }
        }
        // A line in your plan (G20): a safety down against their run.
        if mem.plan.box_up {
            for x in w.iter_mut().filter(|x| x.0.high() == 2) {
                x.1 *= 0.6;
            }
        }
        w.map(|(cov, x)| {
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
    };
    let coverage = match memo {
        None => weigh(rng, &weights()),
        Some((m, slot)) => {
            // The weights read the formation only for its look (the belief)
            // and its personnel (your answer to it).
            let key = (slot, off.look(), off.personnel, personnel);
            let k = match m.coverages.iter().position(|x| x.0 == key) {
                Some(k) => k,
                None => {
                    m.coverages.push((key, weights()));
                    m.coverages.len() - 1
                }
            };
            weigh(rng, &m.coverages[k].1)
        }
    };
    // Heavier linebackers and corners get sent more (L1, C1: the same real
    // defense blitzes 1.1 pts more per 10 lb of linebacker weight on the
    // field and 2 pts more per 10 lb of corner weight).
    let mut blitz = 0.24 + 0.012 * (c.aggressiveness - 10.0) - 0.2 * ease + ctx.heavy;
    if sit.down == 3 && (4.0..=9.0).contains(&sit.togo) {
        blitz += 0.1;
    }
    if sit.hurry() {
        blitz -= 0.1;
    }
    // Their quick throws over this game's plays, from a four-play prior at
    // the 30% mark so the first snaps read nothing (update 3, F32). Layered
    // (B3): the ball out quick beats the rush, so a coordinator blitzes a
    // quick-throwing offense less and one that holds the ball more; how far
    // he backs off is his own nature (a Brian Flores barely does), and the
    // snaps he backs off lean to simulated pressure and disguise, so the
    // quarterback still has to read whether it's zone or a blitz.
    let quick = (mem.opp_quick + 0.3 * 4.0) / (mem.opp_game_plays + 4.0);
    let restraint = clamp((19.0 - c.aggressiveness) / 10.0, 0.1, 1.0);
    let back_off = sharp * (quick - 0.3).max(0.0) * 0.5 * restraint;
    blitz += sharp * (0.3 - quick).max(0.0) * 0.5 - back_off;
    // The head coach's pressure rate (update 2, G2).
    if let Some(r) = answer.rush {
        blitz += 0.7 * (r as f64 - super::guide::habit_pressure(c.aggressiveness));
    } else if let Some((want, k)) = staff.guide.pressure.pull() {
        blitz += k * (want - super::guide::habit_pressure(c.aggressiveness));
    }
    let zone = !coverage.is_man() && coverage != Coverage::Cover0;
    // His own men (lane SF10, item 23): what a linebacker blitz takes away
    // in this coverage with his blitzers, rushers and cover men against
    // what four rushers do (`roster_fit`), by his creativity.
    let p_blitz = clamp(blitz, 0.05, 0.7);
    let p_blitz = if coverage == Coverage::Cover0 {
        p_blitz
    } else {
        let gain = ctx.fit.look[col(coverage, false)] - ctx.fit.look[col(coverage, true)];
        clamp(lean_odds(p_blitz, trust(c.creativity) * gain as f64), 0.05, 0.7)
    };
    let pressure = if coverage == Coverage::Cover0 {
        cover0_rush(rng)
    } else if rng.chance(p_blitz) {
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
    } else if back_off > 0.0 && zone && rng.chance(back_off * if c.skill > 9.0 { 1.5 } else { 0.6 }) {
        // A backed-off blitz still shows pressure: four rush from where the
        // quarterback doesn't expect them, zone behind (B3).
        Pressure::Simulated
    } else {
        Pressure::Four
    };
    let pressure =
        if personnel == DefPersonnel::Dime && pressure == Pressure::DoubleA { Pressure::DbBlitz } else { pressure };
    let disguise = rng.chance(clamp(0.05 + 0.025 * c.skill + 1.5 * back_off, 0.05, 0.7) * (1.0 - ease));
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

/// The short field (engine gaps lane EG6, item 8): how much less real NFL
/// offenses throw inside the 20 than the pass table's down, distance, score
/// and clock say, on the log-odds, by yards to the goal (1-5, 6-10, 11-15,
/// 16-20) and down (first, second, and third and fourth; tries keep their
/// own mix, `TRY_PASS`): nflverse play-by-play, regular seasons 2021-25,
/// outside the last two minutes of a half and close late games, each down
/// and distance read against the same down, distance and score outside the
/// 20. First and goal from the 8 is thrown 30% of the time, first and 10
/// from the 15 36%, both against 46% at midfield. The same at every level
/// (proposed, not confirmed).
const RED_ZONE_PASS: [[f64; 3]; 4] =
    [[-0.708, 0.276, -0.020], [-0.512, 0.093, -0.330], [-0.378, -0.216, 0.068], [-0.230, 0.044, -0.150]];

/// The defense's red zone calls (engine gaps lane EG6, item 8): man coverage
/// and Cover 0 more often as the goal comes nearer, on the log-odds, from
/// nothing at the 20 to these at the goal line. Real NFL dropbacks
/// (nflverse participation data, 2023-25) see man 75% of the time inside
/// the 5, 55% from 6 to 10, 47% and 44% from 11 to 20 and 38% outside it;
/// Cover 0 40%, 14%, 7%, 4% and 2%. On main 46%, 35%, 32%, 31% and 32%
/// man, Cover 0 11%, 8%, 4%, 3% and 3%, from one step (Cover 0 two and a
/// half times as often inside the 10). Proposed, not confirmed.
const RED_ZONE_MAN: f64 = 1.15;
const RED_ZONE_ZERO: f64 = 3.0;

/// Cover 0 on 2 yards or less to go, as a weight: two and a half times as
/// often at midfield, fading to nothing at the goal line, where the red
/// zone's lean already holds it (lane EG6). Proposed, not confirmed.
const COVER0_SHORT: f64 = 2.5;

/// How often a try from scrimmage is a dropback: real NFL two-point tries
/// 71.6% (nflverse play-by-play 2021-25, 663 tries; 44.0% of the dropbacks
/// made and 56.4% of the runs). A try is no down, so it doesn't call from
/// the pass table's first and goal from the 2 (engine gaps lane EG6). The
/// same at every level; proposed, not confirmed.
const TRY_PASS: f64 = 0.716;

/// Cover 0's rush (engine gaps lane EG6): man on every receiver with no deep
/// safety, and as real Cover 0 brings it, six or more on 45% of dropbacks
/// (the double-A pair), five on 30% (one linebacker or, a time in three, a
/// defensive back) and four on the rest, a man left over robbing the middle
/// (nflverse participation data, 2023-25: inside the 5 44%, 28% and 28%,
/// 5.26 rushers; 5.2 to 5.6 at every distance). It always sent seven: the
/// double-A pair, the Mike again and the free safety. Proposed, not
/// confirmed.
const COVER0_SIX: f64 = 0.45;
const COVER0_FIVE: f64 = 0.30;

/// Cover 0's rush for one snap (`COVER0_SIX`, `COVER0_FIVE`).
pub(crate) fn cover0_rush(rng: &mut Rng) -> Pressure {
    cover0_pressure(rng.next())
}

/// Cover 0's rush at a point `u` in [0, 1) of its mix: the double-A pair,
/// a linebacker, a defensive back, then four. The game draws `u`; the call
/// table and the roster table (`call_values` in afm-tools) measure Cover 0
/// on the same mix, so what a coach knows of it is what he calls.
pub fn cover0_pressure(u: f64) -> Pressure {
    if u < COVER0_SIX {
        Pressure::DoubleA
    } else if u < COVER0_SIX + COVER0_FIVE * 2.0 / 3.0 {
        Pressure::LbBlitz
    } else if u < COVER0_SIX + COVER0_FIVE {
        Pressure::DbBlitz
    } else {
        Pressure::Four
    }
}

/// A chance moved on its odds by `x` (the lean the weights use: a choice
/// worth `x` more weighs e^x as much).
fn lean_odds(p: f64, x: f64) -> f64 {
    if x == 0.0 || p <= 0.0 || p >= 1.0 {
        return p;
    }
    let o = p / (1.0 - p) * x.exp();
    o / (1.0 + o)
}

/// Real 2024 NFL: 11 personnel runs 35% of its snaps (the scheme-fit
/// audit's table, `afm/scheme-fit/full-pass.md`).
const RUN_SHARE_11: f64 = 0.35;

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
    let mut w: [(FrontLook, f64); 4] = match staff.front {
        Front::FourThree => [
            (FrontLook::Over, 4.0),
            (FrontLook::Under, 3.0),
            (FrontLook::Wide, if pass_down { 3.0 } else { 1.0 }),
            (FrontLook::Bear, if heavy || short { 1.5 } else { 0.3 }),
        ],
        Front::ThreeFour => [
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
    speeds: (&[(usize, f64)], &[f64]),
    rng: &mut Rng,
) -> Plan {
    let err = 0.8 * (20.0 - own.film_study).max(1.0);
    let seen = |v: f64, rng: &mut Rng| v + rng.normal(0.0, err);
    // How fast their receivers play deep and how fast our corners do
    // (S10a-S10d). Speed shows on film plainer than the rest: half the
    // error, read before the rest of the plan.
    let (their_deep, our_deep) = speeds;
    let deep_seen: Vec<(usize, f64)> = their_deep.iter().map(|&(i, v)| (i, v + rng.normal(0.0, 0.5 * err))).collect();
    let our_deep = if our_deep.is_empty() { 0.0 } else { our_deep.iter().sum::<f64>() / our_deep.len() as f64 };
    let edge = |i: usize| deep_seen.iter().find(|x| x.0 == i).map_or(0.0, |x| (x.1 - our_deep).max(0.0));
    // The man our corners can't run with: from 2 points faster than their
    // average, all-out fear at 10 (proposed, not confirmed).
    let (burner, fear) = deep_seen
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|&(i, v)| (i, clamp((v - our_deep - 2.0) / 8.0, 0.0, 1.0)))
        .filter(|x| x.1 > 0.0)
        .map_or((None, 0.0), |(i, f)| (Some(i), f));
    // Attack the weakest corner when he is clearly the weak one.
    let target_cb = {
        let mut cbs: Vec<(usize, f64)> = their_cbs.iter().map(|&(i, v)| (i, seen(v, rng))).collect();
        cbs.sort_by(|a, b| a.1.total_cmp(&b.1));
        if cbs.len() >= 2 && cbs[1].1 - cbs[0].1 > 4.0 { Some(cbs[0].0) } else { None }
    };
    // Bracket their best receiver when no corner of ours can match him,
    // read partly by deep speed (S10c): a Jameson Williams can draw it
    // over an Amon-Ra St. Brown.
    let bracket = {
        let best = their_wrs.iter().map(|&(i, v)| (i, seen(v, rng) + 0.6 * edge(i))).max_by(|a, b| a.1.total_cmp(&b.1));
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
        burner,
        fear,
        man_floor: 0.0,
        shell_floor: [0.0; 3],
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
pub fn fourth_down(staff: &Staff, sit: &Situation, level: Level, fg_make: f64, rng: &mut Rng) -> FourthDown {
    let y = sit.yardline_100;
    let togo = sit.togo;
    // The values from the level's tables and the two teams (engine gaps
    // item 20: `FourthRead`).
    let (_, ev_go, ev_punt, ev_fg) = fourth_values(sit, fg_make);
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
    // A long miss costs a coach more than its points: past 50 yards he wants
    // the kick to beat the punt by more the longer it is, a bold coach less
    // (`LONG_KICK_CAUTION`; engine gaps item 11).
    let kick_yards = y + super::flow::kick_add();
    let caution = LONG_KICK_CAUTION * (kick_yards - 50.0).max(0.0) * clamp(1.5 - agg / 20.0, 0.5, 1.5);
    let best_kick = if ev_fg > ev_punt + caution && fg_make > 0.3 {
        (FourthDown::FieldGoal, ev_fg)
    } else {
        (FourthDown::Punt, ev_punt)
    };
    // Real coaches still mostly kick: the real go rate as a habit. In the
    // NFL it is the rate outside the late window the last line fills itself
    // (nflverse 2021-2025: 0.18 of the real 1.46 tries a team game came in
    // the fourth quarter's last 2:30 down four or more; counted twice, they
    // helped NFL teams to 1.90 tries a game, minor bug 142).
    let habit = tables::real_fourth_go_habit(togo, y, sit.score_diff, sit.college);
    // How far he trusts the numbers over the habit. The real rates are every
    // kind of coach together, so the level's typical coach keeps the habit;
    // a sharper, bolder one moves toward what the numbers say, a weaker or
    // more cautious one away from it (every coach used to follow them on
    // top of the habit, minor bug 142; proposed, not confirmed).
    let follow = follow_share(gm, agg) - follow_share(super::rules::coach_norm(level), 10.0);
    let says = if ev_go_adj + noise > best_kick.1 + margin { 1.0 } else { 0.0 };
    let go = rng.chance(clamp(habit + follow * (says - habit), 0.0, 1.0));
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
    fourth_ev_read(sit.yardline_100, sit.togo, fg_make, sit.college, sit.fourth)
}

/// How much more than the punt a long field goal must be worth to a coach
/// of average boldness, in points a yard past 50: real NFL coaches kick from
/// their 39-41 a quarter of the time and punt a third (nflverse 2024-25,
/// outside a half's last two minutes), where the expected points say kick;
/// the coach's own aggressiveness scales it from half to one and a half
/// (proposed, not confirmed).
pub const LONG_KICK_CAUTION: f64 = 0.04;

/// Where a punt from `y` yards out leaves the other team, as its distance
/// to the goal: a 41-yard net from deep in his end; from their side of
/// midfield the punter pins them, so they start about 88 yards out (nflverse
/// 2024-25: 88 from the 40-44, 87 from the 45-54, 84 from the 55-59). It
/// was a touchback from inside their 41 and their 8 from the 41-49, so a
/// coach valued a punt from their 40 at a touchback and tried the 57-yard
/// field goal (engine gaps item 11).
pub fn punt_start(y: f64) -> f64 {
    punt_start_net(y, 41.0)
}

/// `punt_start` for a punter whose own net is `net` yards (engine gaps
/// item 20: `FourthRead::punt`, his level's gross punt less the cost,
/// moved by his leg), pinned at their 12 at every level (proposed, not
/// confirmed).
pub fn punt_start_net(y: f64, net: f64) -> f64 {
    100.0 - (y - net).max(12.0)
}

/// `fourth_values` from the spot alone (yards to the goal, yards to go):
/// also what the morning-after bots read your calls with (lane 8). Below
/// the NFL it reads the college table.
pub fn fourth_ev(y: f64, togo: f64, fg_make: f64, college: bool) -> (f64, f64, f64, f64) {
    fourth_ev_read(y, togo, fg_make, college, FourthRead::default())
}

/// `fourth_ev` with the two teams and the level read in (engine gaps item
/// 20): the chance to convert moved by the matchup, the punter's own net
/// punt and the level's spot for a miss.
pub fn fourth_ev_read(y: f64, togo: f64, fg_make: f64, college: bool, read: FourthRead) -> (f64, f64, f64, f64) {
    let conv = fourth_conv(togo, read.conv);
    let ep = |d: u8, t: f64, y: f64| tables::expected_points_in(college, d, t, y);
    let after_score = 0.95;
    let go_ok = if togo >= y { 6.95 - after_score } else { ep(1, 10.0_f64.min(y - togo), y - togo) };
    let go_fail = -ep(1, 10.0, 100.0 - y);
    let ev_go = conv * go_ok + (1.0 - conv) * go_fail;
    // Where the opponent starts after the punt, as its distance to the goal:
    // the punter's own net, pinned at their 12 (`punt_start_net`).
    let opp_punt = punt_start_net(y, read.punt);
    let ev_punt = -ep(1, 10.0, opp_punt.min(99.0));
    // A miss gives them the ball back from the level's spot (or the 20).
    let opp_miss = (100.0 - y - read.miss_back).min(80.0);
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

pub fn go_for_two(
    staff: &Staff,
    diff_after_td: i32,
    quarter: u8,
    game_left: f64,
    xp_make: f64,
    level: Level,
    rng: &mut Rng,
) -> bool {
    // The real habit by the score and the time (engine gaps item 19: real
    // NFL teams go for two on 9.8% of tries, the old chart 5.0%). A kicker
    // under 0.75 on extra points still sends most teams for two.
    let mut habit = tables::real_two_point_habit(diff_after_td, quarter, game_left);
    if xp_make < 0.75 {
        habit = habit.max(0.55);
    }
    // What the numbers say: late, the chart of what each score needs;
    // before that, two points at the real rate (47.5% in the NFL) against
    // the kick.
    let late = quarter >= 4 && game_left < 900.0;
    let chart = matches!(diff_after_td, -2 | -5 | -10 | -13 | 1 | 4 | 5 | 12 | -16 | 8 | -9);
    let says = if late { chart } else { 2.0 * TWO_POINT_MAKE > xp_make + 0.02 };
    // The coach moves off the habit toward the numbers the way he does on
    // fourth downs (`fourth_down`): the level's typical coach keeps it.
    let follow = follow_share(staff.game_management, staff.offense.aggressiveness)
        - follow_share(super::rules::coach_norm(level), 10.0);
    let says = if says { 1.0 } else { 0.0 };
    rng.chance(clamp(habit + follow * (says - habit), 0.0, 1.0))
}

/// The real NFL two-point success rate (2021-2025, 47.5%), the one the
/// coaches weigh against the kick.
const TWO_POINT_MAKE: f64 = 0.475;

/// How far a coach trusts the numbers over the real habit, by his game
/// management and boldness (fourth downs and tries).
fn follow_share(gm: f64, agg: f64) -> f64 {
    clamp(0.05 + 0.012 * gm + 0.01 * (agg - 10.0), 0.03, 0.45)
}
