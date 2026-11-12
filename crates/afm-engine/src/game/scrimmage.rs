//! One snap from scrimmage: alignment, assignments, then the tracked play
//! (topics 1-3). The offense goes toward +x from `los`.

use super::geom::{DT, V2, back_line, end_line, goal, mid, sq_ge, sq_lt, v2, width};

/// Extra bust chance per road offensive player per snap at full crowd
/// noise (trial 4 topic 3).
const NOISE_BUSTS: f64 = 0.06;
/// Awareness a road offensive player loses to full crowd noise. With
/// `NOISE_BUSTS`, the silent count and `SILENT_RELEASE` it is fitted so a
/// 0.65 crowd is worth the real home margin: 2.3 NFL points over 6,000
/// games (se 0.21; real 2.1, nflverse 2021-2025; 2020 without fans +0.1)
/// and 2.8 FBS (se 0.25; real 2.4 in conference games, CFBD 2014-2025).
/// Real road offenses lose it through the air: completions 63.8% against
/// 65.2% at home, runs about even (minor bug 143: the engine had drifted
/// to 0.8 and 1.2 points); proposed, not confirmed.
const NOISE_LOAD: f64 = 11.0;
/// A man new to his job (L1, after Cain's "if i put a will in sam the
/// results reflect that?", 2026-11-07): his read comes this much later in
/// seconds, his chance of a busted assignment and of a missed run fit
/// this much higher, at full newness (a job he never practiced and knows
/// nothing of). Small and capped; proposed, not confirmed.
const JOB_READ: f64 = 0.06;
const JOB_BUST: f64 = 0.015;
const JOB_MISFIT: f64 = 0.03;
/// Seconds a road receiver leaves late at full crowd noise: on a silent
/// count he watches the ball, not the cadence. (0.3 until every receiver
/// had a release (minor bug 135), which left the NFL edge at 1.5 points.)
const SILENT_RELEASE: f64 = 0.38;
use super::playbook::*;
use super::roster::GamePlayer;
use super::rules::Rules;
use super::sim::*;
use crate::attributes::Attr;
use crate::links::Link;
use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::tendencies::Tend;

/// Defensive spots. "L" is the offense's left (higher y).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DSlot {
    LE,
    RE,
    DTL,
    DTR,
    NT,
    Sam,
    Mike,
    Will,
    CBL,
    CBR,
    NCB,
    DCB,
    FS,
    SS,
}

impl DSlot {
    pub fn label(self) -> &'static str {
        match self {
            DSlot::LE => "DE",
            DSlot::RE => "DE",
            DSlot::DTL | DSlot::DTR | DSlot::NT => "DT",
            DSlot::Sam | DSlot::Mike | DSlot::Will => "LB",
            DSlot::CBL | DSlot::CBR => "CB",
            DSlot::NCB | DSlot::DCB => "NB",
            DSlot::FS => "FS",
            DSlot::SS => "SS",
        }
    }
    pub fn is_line(self) -> bool {
        matches!(self, DSlot::LE | DSlot::RE | DSlot::DTL | DSlot::DTR | DSlot::NT)
    }
    pub fn is_lb(self) -> bool {
        matches!(self, DSlot::Sam | DSlot::Mike | DSlot::Will)
    }
    pub fn is_db(self) -> bool {
        matches!(self, DSlot::CBL | DSlot::CBR | DSlot::NCB | DSlot::DCB | DSlot::FS | DSlot::SS)
    }
}

pub fn orole_label(r: ORole) -> &'static str {
    match r {
        ORole::QB => "QB",
        ORole::RB => "RB",
        ORole::FB => "FB",
        ORole::X | ORole::Z | ORole::Slot | ORole::Slot2 | ORole::Slot3 => "WR",
        ORole::TE | ORole::TE2 | ORole::TE3 => "TE",
        ORole::LT => "LT",
        ORole::LG => "LG",
        ORole::C => "C",
        ORole::RG => "RG",
        ORole::RT => "RT",
    }
}

/// Everything a snap needs.
pub struct Snap<'a> {
    pub teams: [&'a [GamePlayer]; 2],
    pub offense: &'a [(ORole, usize)],
    pub defense: &'a [(DSlot, usize)],
    /// Energy (0-1) and the small momentum edge in rating points (topic 4.1),
    /// per player index, per team.
    pub energy: [&'a [f64]; 2],
    pub edge: [&'a [f64]; 2],
    /// The day's talks on each player (dialogue D4-D6).
    pub pulse: [&'a [super::talk::Pulse]; 2],
    /// Who plays hurt, and with what (update 3, F7: `sim::hampered`).
    pub hurt: [&'a [Option<super::sim::InjuryKind>]; 2],
    /// How new each defender is to the job he plays this snap, 0-1 per
    /// player index (L1, `Side::new_to_job`): 0 a job he has been listed
    /// at, up to 1 one he has never even practiced.
    pub new_job: &'a [f32],
    /// How much each player's coach lets him freelance, 0-1 (topic 3.1).
    pub trust: [&'a [f64]; 2],
    pub call: OffCall,
    pub dcall: DefCall,
    pub los: f64,
    pub ball_y: f64,
    pub down: u8,
    pub togo: f64,
    pub rules: &'a Rules,
    pub cond: Conditions,
    pub crew: Crew,
    /// Coach strictness, 1-20, per team (topic 3.1).
    pub strictness: [f64; 2],
    pub injury_scale: f64,
    pub record: bool,
    /// Keep the line chart (lane 9): your games.
    pub chart: bool,
    /// The offense needs the clock stopped (two-minute drill): runners get
    /// out of bounds and receivers work the sideline.
    pub hurry: bool,
    /// The defense plays a two-gap front (S9).
    pub two_gap: bool,
    pub tac: Tactics,
    /// What the game does to habits this snap (offense, defense).
    pub mood: super::sim::Mood,
    /// A fake punt (update 2, G18): the punt team in its punt formation
    /// against the punt return team, the snap straight to the up-back
    /// (`ORole::QB`; the punter stands deep as `ORole::RB`). Some(true)
    /// when the return team showed the look: its front turns to set up
    /// the return.
    pub fake_punt: Option<bool>,
}

/// What the plan and the game so far bring to one snap (plays and tactics
/// topics 2-7).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Tactics {
    /// What the defense believes this look means (topic 7.2): how much more
    /// often the offense runs from it than overall, and how sure it is (0-1).
    pub belief_run: f64,
    pub sure: f64,
    /// The defense staff's film study, 1-20: how well its players were
    /// taught the tells.
    pub study: f64,
    /// How much the quarterback knows the coverage before the snap (0-1),
    /// and whether a disguise fooled him (topic 3.5).
    pub pre_read: f64,
    pub fooled: bool,
    /// The corner the offense plans to attack (a defense player index) and
    /// the receiver the defense brackets (an offense player index), topic 5.4.
    pub target_cb: Option<usize>,
    pub bracket: Option<usize>,
    /// The deep threat the defense fears (an offense player index) and how
    /// much, 0-1 (speed update S10a-S10b).
    #[serde(default)]
    pub burner: Option<usize>,
    #[serde(default)]
    pub fear: f64,
    /// The pass rusher the offense slides its protection to, chips and
    /// doubles (A8; a defense player index).
    #[serde(default)]
    pub help_on: Option<usize>,
    /// The offensive line's chemistry, 0-1 (topic 4.9).
    pub chem: f64,
    /// How new the called family still is to defenses (topic 9), 0-1.
    pub novelty: f64,
    pub family: Family,
    /// College and high school linemen may be three yards downfield on a
    /// pass (RPOs), NFL linemen one.
    pub downfield: f64,
    /// Each side's game plan load (calendar T6.2), offense then defense:
    /// the plan's size against the week to learn it.
    pub load: [f64; 2],
    /// How much the offense planned double moves and pump fakes against
    /// route jumpers (tendencies topic 4.4), 0-1.
    pub double_moves: f64,
    /// New call words this week (update 2, G5), offense then defense: the
    /// extra chance a snap that a player busts.
    #[serde(default)]
    pub learn: [f64; 2],
}

impl Default for Tactics {
    fn default() -> Tactics {
        Tactics {
            belief_run: 0.0,
            sure: 0.0,
            study: 10.0,
            pre_read: 0.0,
            fooled: false,
            target_cb: None,
            bracket: None,
            burner: None,
            fear: 0.0,
            help_on: None,
            chem: 0.6,
            novelty: 0.0,
            family: Family::Quick,
            downfield: 1.0,
            load: [super::roster::INSTALL; 2],
            double_moves: 0.0,
            learn: [0.0; 2],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Result {
    Run,
    Complete,
    Incomplete,
    Interception,
    Sack,
    Scramble,
    Kneel,
    Spike,
}

/// What happened on the snap.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SnapResult {
    pub result: Result,
    /// Where the ball ended, x in the offense's frame.
    pub spot: f64,
    pub spot_y: f64,
    pub touchdown: bool,
    /// The defense scored (pick six, fumble return).
    pub defense_td: bool,
    pub safety: bool,
    /// The ball changed hands (interception or lost fumble).
    pub turnover: bool,
    pub out_of_bounds: bool,
    pub duration: f64,
    pub passer: Option<u32>,
    pub target: Option<u32>,
    pub carrier: Option<u32>,
    /// The player who intercepted or recovered.
    pub returner: Option<u32>,
    pub defender: Option<u32>,
    pub pbu: Option<u32>,
    pub air_yards: f64,
    /// The route the target ran (empty when nobody was targeted).
    #[serde(deserialize_with = "crate::save::leaked")]
    pub route: crate::save::Text,
    pub yac: f64,
    pub before_contact: f64,
    pub time_to_throw: f64,
    pub separation: f64,
    /// Across the field: where the ball was snapped and where the throw
    /// arrived (0 to 53.3, the offense's left high), for the pass chart.
    #[serde(default)]
    pub throw_y: Option<(f32, f32)>,
    pub throw_away: bool,
    pub drop: bool,
    pub pressured: bool,
    /// He threw on the move (rolling out, off his spot): the throw his
    /// throw-on-the-move rating places (growth from results reads it).
    pub moving: bool,
    pub hit: bool,
    pub blitz: bool,
    pub play_action: bool,
    pub box_count: u8,
    pub notes: Notes,
    pub frames: Option<Vec<Frame>>,
    /// Effort per agent: (side, player index, work).
    pub work: Vec<(usize, usize, f64)>,
}

struct Pass {
    reads: Vec<usize>,
    read: usize,
    read_since: f64,
    drop_to: V2,
    set_at: f64,
    thrown: Option<f64>,
    target: Option<usize>,
    pressured_at: Option<f64>,
    hit: bool,
    scramble: bool,
    concept: &'static PassConcept,
    hot: bool,
    look_off: f64,
    /// The receiver he has decided on and when the ball comes out: seeing
    /// it and letting it go takes a beat, shorter for a quick mind.
    commit: Option<(usize, f64, f64)>,
    /// Why he chose the man he throws to (debug counts, `QB_WHY`).
    why: usize,
    /// The lineman who got a hand up and batted the ball at the line, and
    /// where the throw was meant to go (its air yards count as thrown).
    batted: Option<(usize, V2)>,
    /// He was moving when he let it go (`throw`).
    moving: bool,
    /// The receiver he chose because of a size mismatch (D2): a jump ball
    /// or a back-shoulder throw put high, where only his man can reach.
    jump_r: Option<usize>,
    /// The thrown ball: whether it is a jump ball, and how high it arrives
    /// at the catch point, inches off the ground (D1).
    jump_ball: bool,
    ball_h: f64,
    /// Nothing open late and his man covered: he has given up on the play
    /// and throws it away as soon as he can get it off (minor bug 138).
    give_up: bool,
    /// Out of the pocket with his eyes still downfield (the scramble
    /// drill): the side he broke to and when (minor bug 138).
    extend: Option<(f64, f64)>,
}

/// Debug counts of how throws were chosen: on reads 1-4+, any open man
/// after the progression, the checkdown, a forced ball, a throwaway, a
/// scramble (read by `match_check` with `AFM_QBSTATS`).
pub static QB_WHY: [std::sync::atomic::AtomicU64; 9] = [const { std::sync::atomic::AtomicU64::new(0) }; 9];

pub static QB_WHY_COV: [std::sync::atomic::AtomicU64; COVERAGE_COUNT * 9] =
    [const { std::sync::atomic::AtomicU64::new(0) }; COVERAGE_COUNT * 9];

fn qb_why(k: usize, cov: Coverage) {
    QB_WHY[k].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    QB_WHY_COV[cov as usize * 9 + k].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Debug sums of the throw miss (sd, thousandths of a yard) and count, by
/// throw length (under 10, 10-20, over 20 yards): for `match_check`.
pub static THROW_SD: [std::sync::atomic::AtomicU64; 6] = [const { std::sync::atomic::AtomicU64::new(0) }; 6];

/// Debug count of passes batted at the line (read by `match_check` with
/// `AFM_QBSTATS`).
pub static BATTED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Every player's spot and velocity at each tick of the snap so far: one
/// row a tick, (spot, velocity) per player in agent order. The same numbers
/// the lists a player held (each got one entry every tick and nobody joins
/// or leaves a snap, so all were `ticks` long), written as one run a tick
/// instead of 44 pushes (world speed lane).
#[derive(Default)]
struct Trails {
    /// Players a row.
    n: usize,
    /// Rows so far.
    ticks: usize,
    rows: Vec<(V2, V2)>,
}

impl Trails {
    /// Empty, for a snap of `n` players (the room is kept).
    fn reset(&mut self, n: usize) {
        self.n = n;
        self.ticks = 0;
        self.rows.clear();
        self.rows.reserve(n * 144);
    }

    /// This tick's row: every player's spot and velocity now.
    #[inline]
    fn record(&mut self, agents: &[Agent]) {
        debug_assert_eq!(agents.len(), self.n);
        self.rows.extend(agents.iter().map(|a| (a.pos, a.vel)));
        self.ticks += 1;
    }

    /// Player `i`'s spot and velocity at tick `k` (0 the first).
    #[inline]
    fn at(&self, k: usize, i: usize) -> (V2, V2) {
        self.rows[k * self.n + i]
    }
}

thread_local! {
    /// The trails of `run_snap`, kept on this thread between snaps and
    /// emptied at each one's start: only their room is reused, so a snap
    /// allocates none of them.
    static TRAILS: std::cell::Cell<Trails> =
        const { std::cell::Cell::new(Trails { n: 0, ticks: 0, rows: Vec::new() }) };
}

/// Run a snap.
pub fn run_snap(s: &Snap, rng: &mut Rng) -> SnapResult {
    let los = s.los;
    let by = s.ball_y;
    let f = s.call.formation;
    let mut agents = Vec::with_capacity(22);
    let mut role_of: Vec<Option<ORole>> = Vec::new();
    let mut slot_of: Vec<Option<DSlot>> = Vec::new();
    for &(r, p) in s.offense {
        let gp = super::sim::hampered(&s.teams[0][p], s.hurt[0][p]);
        let pos = if s.fake_punt.is_some() { punt_spot(&f, r, los, by, s.rules.level) } else { align(&f, r, los, by) };
        let mut a = Agent::new(0, p, &gp, orole_label(r), pos, s.energy[0][p], s.edge[0][p]);
        if let Some(t) = s.pulse[0].get(p) {
            (a.focus20, a.poise20, a.heat) = (t.focus, t.poise, t.heat);
        }
        agents.push(a);
        role_of.push(Some(r));
        slot_of.push(None);
    }
    let off_n = agents.len();
    // The motion man is moving at the snap (topic 3): a jet man flat out
    // across the formation, an orbit man looping back the other way.
    let motion_man = f.motion_man().and_then(|r| role_of.iter().position(|x| *x == Some(r)));
    if let Some(m) = motion_man {
        let sgn = if f.strength > 0.0 { 1.0 } else { -1.0 };
        let vm = agents[m].vmax;
        agents[m].vel = match f.motion {
            Motion::Jet => v2(0.0, -sgn * 0.85 * vm),
            Motion::Orbit => v2(0.0, sgn * 0.6 * vm),
            Motion::Across => v2(0.0, -sgn * 0.35 * vm),
            _ => agents[m].vel,
        };
    }
    // A road offense works through the noise: hand signals, a silent count
    // and calls it can't hear take some of every player's attention
    // (trial 4 topic 3).
    // A focused player loses less of it (concentration, against the level).
    let load = NOISE_LOAD * s.cond.crowd;
    for a in agents.iter_mut() {
        let conc = (s.teams[0][a.p].trait20(Attr::Concentration, s.rules.level) + a.focus20).clamp(1.0, 20.0);
        let focus = clamp(1.6 - 0.06 * conc, 0.4, 1.6);
        a.r[Attr::FootballIq.index()] = (a.r[Attr::FootballIq.index()] - load * focus).max(1.0);
    }
    let def_spots =
        if s.fake_punt.is_some() { return_spots(s, &agents, &role_of) } else { align_defense(s, &agents, &role_of) };
    for (k, &(d, p)) in s.defense.iter().enumerate() {
        let gp = super::sim::hampered(&s.teams[1][p], s.hurt[1][p]);
        let spot = def_spots[k];
        let mut a = Agent::new(1, p, &gp, d.label(), spot, s.energy[1][p], s.edge[1][p]);
        if let Some(t) = s.pulse[1].get(p) {
            (a.focus20, a.poise20, a.heat) = (t.focus, t.poise, t.heat);
        }
        agents.push(a);
        role_of.push(None);
        slot_of.push(Some(d));
    }
    let mut sim = Sim {
        teams: s.teams,
        agents,
        // Room for a snap's blocks in one allocation: capacity changes no
        // value or index (world speed lane).
        eng: Vec::with_capacity(16),
        ball: Ball::Held(0),
        t: 0.0,
        los,
        rng,
        notes: Notes { chart: s.chart.then(Default::default), ..Notes::default() },
        record: if s.record { Some(Vec::new()) } else { None },
        cond: s.cond,
        crew: s.crew,
        goal: 1.0,
        carrier_side: 0,
        dead: false,
        out_of_bounds: false,
        progress: los,
        contact_force: 0.0,
        injury_scale: s.injury_scale,
        down_on_fall: s.rules.down_on_fall,
        fight: Vec::new(),
        fight_since: 0.0,
        screen: None,
        open_reach: SCRIMMAGE_REACH,
        press: 0.0,
        lane: None,
        bites: Vec::new(),
        two_gap: s.two_gap,
        help_on: None,
        chip: None,
        gap_run: None,
        stunt: None,
        level: s.rules.level,
        mood: super::sim::Mood { line: Some(s.los + s.togo), snap_y: s.ball_y, ..s.mood },
        tend_memo: super::sim::tend_memo(),
        reach_memo: super::sim::reach_memo(),
    };
    // The CFL's fifth receiver reads as the tight end in a set without one.
    let find = |r: ORole| {
        role_of
            .iter()
            .position(|x| *x == Some(r))
            .or_else(|| if r == ORole::TE { role_of.iter().position(|x| *x == Some(ORole::Slot3)) } else { None })
    };
    let qb = find(ORole::QB).unwrap_or(0);
    let center = find(ORole::C).unwrap_or(0);
    sim.ball = Ball::Held(qb);

    let mut res = SnapResult {
        result: Result::Run,
        spot: los,
        spot_y: by,
        touchdown: false,
        defense_td: false,
        safety: false,
        turnover: false,
        out_of_bounds: false,
        duration: 0.0,
        passer: None,
        target: None,
        carrier: None,
        returner: None,
        defender: None,
        pbu: None,
        air_yards: 0.0,
        route: "",
        yac: 0.0,
        before_contact: 0.0,
        time_to_throw: 0.0,
        separation: 0.0,
        throw_y: None,
        throw_away: false,
        drop: false,
        pressured: false,
        moving: false,
        hit: false,
        blitz: s.dcall.pressure.extra() > 0,
        play_action: false,
        box_count: 0,
        notes: Notes::default(),
        frames: None,
        work: Vec::new(),
    };
    res.box_count =
        sim.agents.iter().filter(|a| a.side == 1 && a.pos.x < los + 7.0 && (a.pos.y - by).abs() < 7.5).count() as u8;

    // Kneel and spike end at once.
    match s.call.kind {
        PlayKind::Kneel => {
            res.result = Result::Kneel;
            res.spot = los - 1.0;
            res.duration = 1.0;
            res.carrier = Some(sim.agents[qb].id);
            return res;
        }
        PlayKind::Spike => {
            res.result = Result::Spike;
            res.duration = 1.0;
            res.passer = Some(sim.agents[qb].id);
            return res;
        }
        _ => {}
    }

    // Reading the play (topic 2.1): each defender knows run or pass at his
    // own moment; smart ones sooner. Fakes hold the rest a little longer.
    let fake = match s.call.kind {
        PlayKind::Pass(c) => pass_concept(c).play_action,
        PlayKind::Run(RunConcept::Draw) => true,
        _ => false,
    };
    res.play_action = fake && matches!(s.call.kind, PlayKind::Pass(_));
    // What he expects from this look (topics 7.2, 2.1): a tell he was
    // taught from film and has seen this game lets him read the play a beat
    // sooner, and the wrong lean (play action from a run look) holds him
    // longer. Still thinking about his own job, he is a step slow (topic
    // 4.2); jet and orbit motion freeze linebackers; a new idea is read late
    // (topic 9). Sizes proposed, not confirmed.
    let tac = s.tac;
    let run_play = matches!(s.call.kind, PlayKind::Run(_) | PlayKind::Rpo(..));
    let lean = clamp(tac.belief_run * 2.5, -1.0, 1.0) * tac.sure;
    let taught = 0.5 + 0.5 * clamp(tac.study / 20.0, 0.0, 1.0);
    let jet = matches!(f.motion, Motion::Jet | Motion::Orbit);
    let cov_fam = s.dcall.coverage.family();
    // A quarterback who sells the fake holds them longer (ball handling); a
    // disciplined defender keeps his eyes where they belong.
    let sell = clamp(1.0 + 0.01 * (sim.agents[qb].a(Attr::BallHandling) - 60.0), 0.6, 1.4);
    // Recognition is read against the level, as the offense's timing is
    // (trial 9 finding 2): on the raw scale an average NFL defender read a
    // run 0.14 s sooner than an average FBS one while the blocking and the
    // back's path kept the same clock, so NFL runs were stuffed 33-36% of
    // the time (real 18%). Anchored at FBS, which the world's rosters
    // calibrate (proposed, not confirmed).
    let level_read = super::rules::trait_norm(sim.level, Attr::PlayRecognition)
        - super::rules::trait_norm(crate::Level::FBS, Attr::PlayRecognition);
    for i in off_n..sim.agents.len() {
        let pr = sim.agents[i].a(Attr::PlayRecognition);
        let bite = clamp(1.5 - 0.05 * sim.trait20(i, Attr::Discipline), 0.5, 1.5);
        let slot = slot_of[i].unwrap_or(DSlot::Mike);
        let hit = if matches!(slot, DSlot::FS | DSlot::SS) { hitter(sim.level, sim.player(i)) } else { 0.0 };
        sim.agents[i].hit = hit;
        // Flies downhill or reads first (tendencies): a linebacker or
        // safety who triggers on his first read gets there sooner, and
        // bites harder on the fake.
        let downhill =
            if slot.is_lb() || matches!(slot, DSlot::FS | DSlot::SS) { sim.lean(i, Tend::Downhill) } else { 0.0 };
        let base = 0.22 + 0.0075 * (100.0 - pr + level_read) + 0.12 * sim.rng.next() - 0.14 * downhill;
        let extra = if fake {
            (0.15 + 0.012 * (100.0 - pr) * sim.rng.next()) * sell * bite * (1.0 + 0.9 * downhill).max(0.1)
        } else {
            0.0
        };
        let keyed = lean * taught * clamp(pr / 80.0, 0.3, 1.2);
        let tend = if run_play { -0.16 * keyed } else { 0.16 * keyed };
        let fam = if slot.is_line() { Family::Front } else { cov_fam };
        // Measured against a starter who knows his job well (0.9).
        let slow = 0.12 * (0.9 - sim.player(i).know[fam as usize] as f64);
        // A week of poor prep reads late, a sharp one a touch early (TR38):
        // up to a tenth of a second for Unprepared, a beat (update 3, F34).
        let prep = 0.3 * (sim.player(i).prep as f64 - 1.0);
        let freeze = if jet && slot.is_lb() { 0.10 * bite } else { 0.0 };
        let new_job = JOB_READ * s.new_job.get(sim.agents[i].p).copied().unwrap_or(0.0) as f64;
        sim.agents[i].read_at = (base + extra + tend + slow + prep + freeze + 0.12 * tac.novelty + new_job).max(0.12);
    }
    // A fake punt into a return team that turned to set up the return: its
    // front has its back to the play and reads it late; a disciplined man
    // checks for the fake before he turns (proposed, not confirmed: a
    // quarter of a second for an average man, an eighth to three eighths).
    if s.fake_punt == Some(true) {
        for i in off_n..sim.agents.len() {
            let slot = slot_of[i].unwrap_or(DSlot::Mike);
            if matches!(slot, DSlot::FS | DSlot::CBL | DSlot::CBR | DSlot::NCB | DSlot::DCB) {
                continue;
            }
            let bite = clamp(1.5 - 0.05 * sim.trait20(i, Attr::Discipline), 0.5, 1.5);
            sim.agents[i].read_at += 0.25 * bite;
        }
    }

    // Busted assignments (topic 3): players who don't know the playbook.
    // A road offense in a loud stadium also misses calls it can't hear: the
    // line's protection calls and the receivers' adjustments (trial 4 topic
    // 3; size fitted to the real home margin, proposed, not confirmed).
    // He busts by what he knows of the family called (topic 4), and a
    // defense must check to trips, bunch, empty and motion before the snap:
    // a young defender misses the check (topic 3).
    let checks = matches!(f.set, Set::Trips | Set::Bunch | Set::Empty) || f.motion != Motion::None;
    // The calls (communication): the quarterback for the offense, the
    // center for the line's protection, the defense's green-dot man (its
    // best communicator among the linebackers and safeties) for the defense.
    let comm = |k: usize| clamp(1.0 - 0.006 * (sim.agents[k].a(Attr::Communication) - 60.0), 0.75, 1.25);
    let off_call = comm(qb);
    let line_call = comm(center);
    let def_caller = (off_n..sim.agents.len())
        .filter(|&k| slot_of[k].is_some_and(|d| d.is_lb() || matches!(d, DSlot::FS | DSlot::SS)))
        .max_by(|&a, &b| sim.agents[a].a(Attr::Communication).total_cmp(&sim.agents[b].a(Attr::Communication)));
    let def_call = def_caller.map_or(1.0, comm);
    for i in 0..sim.agents.len() {
        let gp = sim.player(i);
        let iq = gp.a(Attr::FootballIq);
        // His week's prep (TR38): up to about a third more busts.
        let prep = gp.prep as f64;
        let fam = if i < off_n {
            tac.family
        } else if slot_of[i].is_some_and(|d| d.is_line()) {
            Family::Front
        } else {
            cov_fam
        };
        let know = gp.know[fam as usize] as f64;
        let noise = if i < off_n { NOISE_BUSTS * s.cond.crowd * (1.3 - iq / 100.0).max(0.3) } else { 0.0 };
        let check = if i >= off_n && checks { 0.04 * (1.0 - know) } else { 0.0 };
        // The install (T6.2): a plan bigger than he can absorb this week
        // means mental errors (proposed, not confirmed).
        let load = tac.load[if i < off_n { 0 } else { 1 }];
        let install = 0.04 * (load - super::roster::absorb(iq)).max(0.0);
        // A tired mind wanders: late in a drive a low-concentration player
        // busts more, a focused one less (centred on the level).
        let energy = s.energy[if i < off_n { 0 } else { 1 }][sim.agents[i].p];
        let lapse = 0.03 * (1.0 - energy) * (10.0 - sim.trait20(i, Attr::Concentration)) / 9.0;
        let call = if i >= off_n {
            def_call
        } else if is_line(sim.agents[i].label) {
            line_call
        } else {
            off_call
        };
        // Chemistry (X5): a receiver new to his quarterback misreads more
        // sight adjustments and option routes, a secondary new to each
        // other blows more hand-offs (up to about a fifth either way).
        let chem = match sim.player(i).position {
            crate::positions::Position::WR
            | crate::positions::Position::TE
            | crate::positions::Position::RB
            | crate::positions::Position::CB
            | crate::positions::Position::S => 1.0 - 0.45 * (sim.player(i).chem - super::roster::TYPICAL_CHEM) as f64,
            _ => 1.0,
        };
        // New call words (G5): read through his football mind, as the crowd
        // noise is; about one extra bust in 70 snaps for a side of average
        // minds (a 60), more for a slow learner, less for a sharp one
        // (proposed, not confirmed).
        let words = tac.learn[if i < off_n { 0 } else { 1 }] * (1.3 - iq / 100.0).max(0.3) / 0.7;
        let job =
            if i >= off_n { JOB_BUST * s.new_job.get(sim.agents[i].p).copied().unwrap_or(0.0) as f64 } else { 0.0 };
        let p = ((1.0 - know) * 0.05 + (60.0 - iq).max(0.0) * 0.0006 + noise + check + install + lapse + words + job)
            .max(0.0)
            * call
            * chem
            * prep;
        if sim.rng.chance(p) {
            sim.agents[i].bust = true;
            let id = sim.agents[i].id;
            sim.notes.busts.push(id);
        }
    }

    // Run fits (topic 1): a linebacker or safety who misreads his fit fills
    // the wrong gap or comes late to the alley. Misdirection, motion and a
    // new idea cause more of it; discipline and knowing the front less
    // (proposed, not confirmed).
    let misdirect = matches!(
        s.call.kind,
        PlayKind::Run(rc) | PlayKind::Rpo(rc, _) if matches!(
            rc,
            RunConcept::Counter
                | RunConcept::Trap
                | RunConcept::BuckSweep
                | RunConcept::ZoneRead
                | RunConcept::SpeedOption
                | RunConcept::TripleOption
                | RunConcept::JetSweep
        )
    );
    if run_play {
        for i in off_n..sim.agents.len() {
            if slot_of[i].is_none_or(|d| d.is_line()) {
                continue;
            }
            let gp = sim.player(i);
            let pr = gp.a(Attr::PlayRecognition);
            let know = gp.know[Family::Front as usize] as f64;
            let disc = sim.trait20(i, Attr::Discipline);
            let prep = gp.prep as f64;
            let p = 0.035
                + 0.002 * (65.0 - pr).max(-15.0)
                + 0.06 * (1.0 - know)
                + 0.003 * (10.0 - disc)
                + if misdirect { 0.04 } else { 0.0 }
                + if jet { 0.03 } else { 0.0 }
                + 0.05 * tac.novelty
                + JOB_MISFIT * s.new_job.get(sim.agents[i].p).copied().unwrap_or(0.0) as f64;
            let p = p * prep;
            if sim.rng.chance(clamp(p, 0.01, 0.35)) {
                sim.agents[i].misfit = true;
                let id = sim.agents[i].id;
                sim.notes.misfits.push(id);
            }
        }
    }
    let mut pass: Option<Pass> = None;
    let mut handoff_done = false;
    let mut aim: Option<V2> = None;
    let mut lead_puller: Option<usize> = None;
    let mut zone_read_keep = false;
    // The option (topic 1): the quarterback may still pitch to this man.
    let mut option_live = false;
    let mut pitch_man: Option<usize> = None;
    let mut throw_info: Option<(V2, usize, f64, f64, f64)> = None;

    // ----------------------------------------------------------- assignments
    match s.call.kind {
        PlayKind::Pass(ci) => {
            let c = pass_concept(ci);
            res.passer = Some(sim.agents[qb].id);
            for i in 0..off_n {
                let r = role_of[i].unwrap();
                if r == ORole::QB {
                    sim.agents[i].job = Job::Passer;
                    continue;
                }
                let route = route_for(c, r);
                sim.agents[i].job = route_job(&sim.agents[i], route, los, by, s.hurry);
                let late = silent_late(&sim, s, i);
                if let Job::Route { release_delay, .. } = &mut sim.agents[i].job {
                    *release_delay += late;
                }
            }
            let side = if f.strength > 0.0 { 1.0 } else { -1.0 };
            let (dx, dy) = match (c.drop, f.backfield) {
                (Drop::Quick, Backfield::UnderCenter) => (-4.0, 0.0),
                (Drop::Quick, _) => (-6.0, 0.0),
                (Drop::Medium, Backfield::UnderCenter) => (-6.5, 0.0),
                (Drop::Medium, _) => (-7.0, 0.0),
                (Drop::Deep, _) => (-8.0, 0.0),
                (Drop::Boot, _) => (-6.0, 7.0 * side),
                (Drop::Screen, _) => (-8.5, 0.0),
                (Drop::Bubble, _) => (-5.5, 0.0),
            };
            let set_at = match c.drop {
                // Snap to the top of the drop (the shotgun snap itself takes
                // a few tenths): about 1.25 s quick, 1.6 s five-step, 2.1 s
                // seven. (Quick was 1.05 s and the bubble 0.45 s: a quarter
                // of NFL throws left before 1.6 s against a real tenth;
                // proposed, not confirmed.)
                Drop::Quick => 1.25,
                Drop::Medium => 1.6,
                Drop::Deep => 2.1,
                Drop::Boot => 2.2,
                Drop::Screen => 1.4,
                Drop::Bubble => 0.75,
            } + if c.play_action { 0.35 } else { 0.0 };
            // Clean feet get him to the top of the drop and set sooner, a
            // few hundredths either way at the extremes. Footwork, release
            // and anticipation were first read at twice these sizes: NFL
            // quarterbacks rate far above 60 in all three, so they threw in
            // 2.41 s (real 2.71) and were sacked on 4.6% of dropbacks (real
            // 6.8%). Now 2.57 s and 2.1 sacks a team game (proposed, not
            // confirmed).
            let feet = sim.agents[qb].a(Attr::Footwork);
            let mut set_at = set_at + clamp(-0.0015 * (feet - 60.0), -0.06, 0.08);
            // A bad shotgun snap (high, low or wide) costs him the top of his
            // drop: rare, about 0.4% at 60 and a handful a team season
            // (proposed, not confirmed). The fumbled snap is not modelled.
            if f.backfield != Backfield::UnderCenter {
                let snap = sim.agents[center].a(Attr::ShotgunSnap);
                if sim.rng.chance(clamp(0.004 - 0.0001 * (snap - 60.0), 0.001, 0.012)) {
                    set_at += 0.3 + 0.3 * sim.rng.next();
                }
            }
            let reads = c.reads.iter().filter_map(|r| find(*r)).collect::<Vec<_>>();
            pass = Some(Pass {
                reads,
                read: 0,
                read_since: set_at + READ_START + 0.05,
                drop_to: v2(los + dx, by + dy),
                set_at,
                thrown: None,
                target: None,
                pressured_at: None,
                hit: false,
                scramble: false,
                concept: c,
                hot: false,
                look_off: 0.0,
                commit: None,
                why: 0,
                batted: None,
                moving: false,
                jump_r: None,
                jump_ball: false,
                ball_h: 0.0,
                give_up: false,
                extend: None,
            });
            if c.drop == Drop::Screen {
                sim.screen = pass.as_ref().and_then(|ps| ps.reads.first().copied());
            }
            assign_defense(&mut sim, s, off_n, &slot_of, &role_of, true);
            pass_protection(&mut sim, off_n);
            stunt_pickup(&mut sim, s.tac.chem);
            slide_to(&mut sim, s, off_n);
            chart_rush(&mut sim, off_n, &slot_of, by);
            // The matchup plan (topic 5.4): the quarterback looks first at
            // the receiver on the corner the staff wants to attack, when that
            // man is in the progression.
            // Feed the best receiver (A9): real staffs build the plan around
            // their best target. The bigger his edge over the others in the
            // progression, the more often the play's first look is his.
            let mut fed = false;
            if let Some(ps) = pass.as_mut()
                && ps.reads.len() > 1
                && !matches!(c.drop, Drop::Screen | Drop::Bubble)
            {
                // Each man is rated at his own spot: read at the receiver's
                // spot, tight ends and backs never looked best and receivers
                // drew 73% of NFL targets, real 59% (minor bug 137).
                let rate = |sim: &Sim, r: usize| {
                    use crate::spots::Spot;
                    let spot = match sim.agents[r].label {
                        "TE" => Spot::TE,
                        "RB" => Spot::RB,
                        "FB" => Spot::FB,
                        _ => Spot::WR,
                    };
                    sim.player(r).spot_r[spot.index()]
                };
                let mut by: Vec<(usize, f64)> = ps.reads.iter().enumerate().map(|(k, &r)| (k, rate(&sim, r))).collect();
                by.sort_by(|a, b| b.1.total_cmp(&a.1));
                let gap = by[0].1 - by[1].1;
                if by[0].0 > 0 && sim.rng.chance(clamp(FEED_BASE + FEED_PER_POINT * gap, 0.0, FEED_MAX)) {
                    let r = ps.reads.remove(by[0].0);
                    ps.reads.insert(0, r);
                    fed = true;
                }
            }
            if let (Some(cb), Some(ps)) = (s.tac.target_cb, pass.as_mut())
                && let Some(d) = (off_n..sim.agents.len()).find(|&k| sim.agents[k].p == cb)
            {
                let on = match sim.agents[d].job {
                    Job::Man { target, .. } => Some(target),
                    _ => ps.reads.iter().copied().min_by(|&a, &b| {
                        sim.agents[a].pos.dist(sim.agents[d].pos).total_cmp(&sim.agents[b].pos.dist(sim.agents[d].pos))
                    }),
                };
                if let Some(k) = on.and_then(|r| ps.reads.iter().position(|&x| x == r))
                    && k > 0
                    && !fed
                    && sim.rng.chance(0.5)
                {
                    let r = ps.reads.remove(k);
                    ps.reads.insert(0, r);
                }
            }
        }
        PlayKind::Run(rc) | PlayKind::Rpo(rc, _) => {
            let dir = s.call.dir;
            let (aim_y, pulls) = rc.aim();
            let aim_pt = v2(los + 1.0, by + aim_y * dir);
            aim = Some(aim_pt);
            sim.press = rc.press();
            let mesh = match (rc, f.backfield) {
                (RunConcept::Sneak, _) => sim.agents[qb].pos,
                (_, Backfield::UnderCenter) => v2(los - 4.0, by + 0.6 * dir),
                (_, Backfield::Pistol) => v2(los - 4.6, by + 0.5 * dir),
                (_, Backfield::Shotgun) => v2(los - 5.2, by + 0.4 * dir),
            };
            // When the ball is handed off. (Each was 0.4 s sooner: with the
            // press it put backs on the line 0.7 s before the real Next Gen
            // time; the toss is still pitched at once. Proposed, not
            // confirmed.)
            let mesh_t = match rc {
                RunConcept::Sneak => 0.0,
                RunConcept::Draw => 1.5,
                RunConcept::Toss => 0.45,
                // Gap runs wait for the pullers (the counter step).
                RunConcept::Power => {
                    if f.backfield == Backfield::UnderCenter {
                        1.35
                    } else {
                        1.2
                    }
                }
                RunConcept::Counter => {
                    if f.backfield == Backfield::UnderCenter {
                        1.55
                    } else {
                        1.4
                    }
                }
                _ => match f.backfield {
                    Backfield::UnderCenter => 1.2,
                    _ => 1.0,
                },
            };
            for i in 0..off_n {
                let r = role_of[i].unwrap();
                sim.agents[i].job = match r {
                    ORole::QB => Job::Fake { to: mesh },
                    ORole::RB => Job::Handoff { aim: aim_pt, mesh, at: mesh_t },
                    ORole::FB => Job::RunBlock { target: None, pull: false },
                    r if r.is_line() || matches!(r, ORole::TE | ORole::TE2 | ORole::TE3) => {
                        Job::RunBlock { target: None, pull: false }
                    }
                    _ => Job::RunBlock { target: None, pull: false },
                };
            }
            if matches!(rc, RunConcept::Sneak | RunConcept::QbPower | RunConcept::SpeedOption) {
                sim.agents[qb].job = Job::Carrier;
                handoff_done = true;
                res.carrier = Some(sim.agents[qb].id);
                if let Some(rb) = find(ORole::RB) {
                    // QB power: the back leads through the hole. Speed
                    // option: he keeps the pitch relationship.
                    sim.agents[rb].job = if rc == RunConcept::SpeedOption {
                        option_live = true;
                        pitch_man = Some(rb);
                        Job::PitchMan
                    } else {
                        Job::RunBlock { target: None, pull: true }
                    };
                }
            }
            assign_defense(&mut sim, s, off_n, &slot_of, &role_of, false);
            sim.gap_run = match rc {
                RunConcept::InsideZone
                | RunConcept::OutsideZone
                | RunConcept::ZoneRead
                | RunConcept::Duo
                | RunConcept::TripleOption => Some(false),
                RunConcept::Power
                | RunConcept::Counter
                | RunConcept::BuckSweep
                | RunConcept::Trap
                | RunConcept::QbPower
                | RunConcept::Iso => Some(true),
                _ => None,
            };
            run_blocking(&mut sim, off_n, &role_of, rc, dir, pulls);
            if let Some(look) = s.fake_punt {
                return_jobs(&mut sim, s, off_n, &slot_of, &role_of, look);
            }
            // The jet sweep: the motion man takes it on the run at the
            // quarterback; the back fakes the other way.
            if rc == RunConcept::JetSweep
                && let Some(m) = motion_man
            {
                // The mesh sits on his path in front of the quarterback, who
                // steps to meet him.
                let (q, jp) = (sim.agents[qb].pos, sim.agents[m].pos);
                sim.agents[m].job = Job::Handoff { aim: aim_pt, mesh: v2(jp.x + 0.2, q.y), at: 0.25 };
                sim.agents[qb].job = Job::Fake { to: v2(jp.x + 0.7, q.y) };
                if let Some(rb) = find(ORole::RB) {
                    sim.agents[rb].job = Job::RunBlock { target: None, pull: false };
                }
            }
            // An RPO (topic 1): the line run-blocks, the receivers in the
            // throw run their routes.
            if let PlayKind::Rpo(_, ci) = s.call.kind {
                let c = pass_concept(ci);
                for i in 0..off_n {
                    let r = role_of[i].unwrap();
                    if r.is_line() || matches!(r, ORole::QB | ORole::RB) {
                        continue;
                    }
                    let route = route_for(c, r);
                    if !route.blocks() {
                        sim.agents[i].job = route_job(&sim.agents[i], route, los, by, false);
                    }
                }
            }
            // On a gap run the back follows the lead puller to the hole.
            lead_puller = (0..off_n).find(|&i| {
                matches!(sim.agents[i].job, Job::RunBlock { pull: true, .. }) && role_of[i].is_some_and(|r| r.is_line())
            });
        }
        _ => {}
    }

    // ------------------------------------------------------------- the play
    let max_t = 14.0;
    // The trails come from the last snap on this thread (`TRAILS`) and are
    // emptied here, so every snap starts with none, as before.
    let mut trail = TRAILS.take();
    trail.reset(sim.agents.len());
    let mut throw_start: Option<f64> = None;
    let mut catch_x = los;
    let mut thrown_from = los;
    sim.record_frame();
    while sim.t < max_t && !sim.dead {
        sim.t += DT;
        sim.link_speeds();
        let t = sim.t;
        trail.record(&sim.agents);
        let carrier = sim.carrier();
        let qb_has = carrier == Some(qb) && pass.as_ref().is_some_and(|p| p.thrown.is_none() && !p.scramble);
        let in_air = matches!(sim.ball, Ball::Air { .. });
        sim.update_blocks(if qb_has { Some(qb) } else { None }, carrier);

        // ---- screen: the line lets the rush through and leaks out in front of the back
        if let (Some(r), Some(ps)) = (sim.screen, pass.as_ref())
            && ps.thrown.is_none()
            && t >= 0.9
        {
            let side = if sim.agents[r].pos.y >= by { 1.0 } else { -1.0 };
            for i in 0..off_n {
                let a = &sim.agents[i];
                let inside = matches!(a.label, "LG" | "C" | "RG");
                let playside_tackle = matches!(a.label, "LT" | "RT") && (a.pos.y - by) * side > 0.0;
                if !(inside || playside_tackle) || !matches!(a.job, Job::PassBlock { .. }) {
                    continue;
                }
                if let Some(k) = sim.eng.iter().position(|e| !e.over && (e.blocker == i || e.helper == Some(i))) {
                    sim.release(k);
                }
                sim.agents[i].job = Job::Escort;
            }
        }

        // Combination blocks: the man with a linebacker to climb to leaves
        // the double team once the down lineman is controlled, when his
        // linebacker comes downhill at it, or after a second at most
        // (fix list S6.1; proposed, not confirmed).
        for i in 0..off_n {
            let Some(lb) = sim.agents[i].climb else { continue };
            let Some(k) = sim.agents[i].engaged else { continue };
            let e = &sim.eng[k];
            if e.over {
                sim.agents[i].climb = None;
                continue;
            }
            let Some(h) = e.helper else { continue };
            if !sim.agents[lb].up(t) || sim.agents[lb].engaged.is_some() {
                // His linebacker is taken: he stays on the double team.
                sim.agents[i].climb = None;
                continue;
            }
            let held = t - e.start;
            let lbp = sim.agents[lb].pos;
            let downhill = (lbp.x - los) * sim.goal < 2.0 || lbp.dist(sim.agents[i].pos) < 2.5;
            if (e.w < -0.15 && held > 0.35) || held > 1.0 || downhill {
                if e.blocker == i {
                    sim.eng[k].blocker = h;
                }
                sim.eng[k].helper = None;
                sim.agents[i].engaged = None;
                sim.agents[i].climb = None;
                sim.agents[i].job = Job::RunBlock { target: Some(lb), pull: false };
            }
        }
        // ---- offense
        for i in 0..off_n {
            if !sim.agents[i].up(t) || sim.agents[i].engaged.is_some() {
                continue;
            }
            // The target adjusts to the ball in the air: he works to the spot
            // and times his arrival with it.
            if let Ball::Air { to, arrive, target: Some(tg), .. } = sim.ball
                && tg == i
                && t >= sim.agents[i].ball_react
            {
                // A ball sailing out of bounds he chases only to the line:
                // his feet stay in, and he reaches for what he can.
                let to = feet_in(to);
                let me = sim.agents[i].pos;
                let d = me.dist(to);
                let left = (arrive - t).max(DT);
                let speed = (d / left).min(sim.agents[i].vmax);
                // The heading from the same distance (`toward`).
                sim.steer(i, toward(me, to, d).scale(speed));
                continue;
            }
            // A route is run in place: its waypoints are not copied each tick.
            if matches!(sim.agents[i].job, Job::Route { .. }) {
                run_route(&mut sim, i, &pass, qb, s);
                continue;
            }
            let job = sim.agents[i].job.clone();
            match job {
                // Once the ball is out he pulls up: he used to coast at his
                // release speed through the whole flight, out of the stadium
                // from a rollout near the sideline.
                Job::Passer => {
                    if pass.as_ref().is_some_and(|p| p.thrown.is_some()) {
                        sim.steer(i, V2::default());
                    }
                }
                Job::Carrier => {}
                Job::Fake { to } => {
                    if handoff_done {
                        // Carry out the fake away from the run.
                        let away = v2(los - 5.0, by - 4.0 * s.call.dir);
                        sim.go_to(i, away, 0.5);
                    } else {
                        sim.go_to(i, to, 0.8);
                    }
                }
                Job::Handoff { mesh, at, aim: hole } => {
                    // He times his path through the mesh and keeps moving.
                    let me = sim.agents[i].pos;
                    let d = me.dist(mesh);
                    // A jet man arrives at full speed; a back times the mesh.
                    let jet_man = Some(i) == motion_man && f.motion == Motion::Jet;
                    let cap = if jet_man { 1.0 } else { 0.7 };
                    if t < at && d > 0.3 {
                        let speed = (d / (at - t).max(DT)).min(sim.agents[i].vmax * cap);
                        sim.steer(i, (mesh - me).norm().scale(if jet_man { sim.agents[i].vmax } else { speed }));
                    } else {
                        sim.steer(i, (hole - me).norm().scale(sim.agents[i].vmax * if jet_man { 1.0 } else { 0.55 }));
                    }
                    if (t >= at || jet_man && t > 0.1) && d < 1.3 && !handoff_done {
                        handoff_done = true;
                        // The RPO read at the mesh: the quarterback pulls it
                        // and throws when the defender in conflict plays the
                        // run.
                        if let PlayKind::Rpo(_, ci) = s.call.kind {
                            let c = pass_concept(ci);
                            let reads: Vec<usize> =
                                c.reads.iter().filter_map(|r| role_of.iter().position(|x| *x == Some(*r))).collect();
                            if let Some(&tgt) = reads.first()
                                && rpo_pull(&mut sim, qb, tgt, off_n)
                            {
                                sim.agents[i].job = Job::RunBlock { target: None, pull: false };
                                res.passer = Some(sim.agents[qb].id);
                                sim.notes.rpo_pulled = true;
                                let q = sim.agents[qb].pos;
                                pass = Some(Pass {
                                    reads,
                                    read: 0,
                                    read_since: t,
                                    drop_to: q,
                                    set_at: t + 0.15,
                                    thrown: None,
                                    target: None,
                                    pressured_at: None,
                                    hit: false,
                                    scramble: false,
                                    concept: c,
                                    hot: false,
                                    look_off: 0.0,
                                    commit: None,
                                    why: 0,
                                    batted: None,
                                    moving: false,
                                    jump_r: None,
                                    jump_ball: false,
                                    ball_h: 0.0,
                                    give_up: false,
                                    extend: None,
                                });
                                sim.agents[qb].job = Job::Passer;
                                continue;
                            }
                        }
                        // Zone read: the quarterback reads the backside end.
                        let rc = match s.call.kind {
                            PlayKind::Run(rc) | PlayKind::Rpo(rc, _) => rc,
                            _ => RunConcept::InsideZone,
                        };
                        if matches!(rc, RunConcept::ZoneRead | RunConcept::TripleOption) {
                            zone_read_keep = read_option(&mut sim, qb, s.call.dir, off_n);
                        }
                        // The triple option's third phase: kept, the
                        // quarterback attacks the edge with a pitch man.
                        if zone_read_keep && rc == RunConcept::TripleOption {
                            let pm = find(ORole::FB)
                                .or_else(|| find(ORole::Slot))
                                .or_else(|| find(ORole::TE))
                                .filter(|&p| p != i);
                            if let Some(pm) = pm {
                                sim.agents[pm].job = Job::PitchMan;
                                pitch_man = Some(pm);
                                option_live = true;
                            }
                        }
                        if zone_read_keep {
                            sim.agents[qb].job = Job::Carrier;
                            sim.ball = Ball::Held(qb);
                            sim.progress = sim.agents[qb].pos.x;
                            sim.agents[i].job = Job::RunBlock { target: None, pull: false };
                            res.carrier = Some(sim.agents[qb].id);
                            aim = Some(v2(los + 1.0, by - 6.0 * s.call.dir));
                        } else {
                            sim.agents[i].job = Job::Carrier;
                            sim.ball = Ball::Held(i);
                            sim.progress = sim.agents[i].pos.x;
                            res.carrier = Some(sim.agents[i].id);
                            // A rare bad exchange: the back's ball security and
                            // the quarterback's ball handling.
                            let hands = 0.65 * sim.agents[i].a(Attr::BallSecurity)
                                + 0.35 * sim.agents[qb].a(Attr::BallHandling);
                            if sim.rng.chance(0.0025 * (1.6 - hands / 100.0)) {
                                sim.notes.fumbled = Some(sim.agents[i].id);
                                sim.fumble(i);
                            }
                        }
                    }
                    // A missed mesh (a back split wide, as in an empty set
                    // checked to a run): the quarterback tucks it and runs.
                    if !handoff_done && t >= at + 0.4 && matches!(sim.ball, Ball::Held(c) if c == qb) {
                        handoff_done = true;
                        sim.agents[qb].job = Job::Carrier;
                        sim.progress = sim.agents[qb].pos.x;
                        res.carrier = Some(sim.agents[qb].id);
                        sim.agents[i].job = Job::RunBlock { target: None, pull: false };
                    }
                }
                Job::Route { .. } => run_route(&mut sim, i, &pass, qb, s),
                Job::PitchMan => {
                    // He keeps his pitch relationship: about four yards
                    // outside the quarterback and a yard and a half behind.
                    let q = sim.agents[qb].pos;
                    let spot = v2(q.x - 1.5, q.y + 4.5 * s.call.dir);
                    sim.go_to(i, spot, 1.0);
                }
                Job::PassBlock { target } => {
                    let q = sim.agents[qb].pos;
                    match target.filter(|&d| sim.agents[d].up(t) && sim.agents[d].engaged.is_none()) {
                        Some(d) => {
                            // He sets between the rusher and the quarterback and
                            // mirrors him (kick slide), matching his speed.
                            let dp = sim.agents[d].pos;
                            let dv = sim.agents[d].vel;
                            let spot = dp + (q - dp).norm().scale(1.0) + dv.scale(0.15);
                            let me = sim.agents[i].pos;
                            sim.steer(i, dv.scale(0.8) + (spot - me).scale(3.0));
                            // (On the square, `sq_lt`: the same answer.)
                            if sq_lt(me.dist_sq(dp), 1.5) && sim.can_square(i, d, q) {
                                sim.engage(i, d, true);
                            }
                        }
                        None => {
                            // Find the most dangerous free rusher near him, or help.
                            // The same tests on the squares (`sq_lt`) and the
                            // same pick as `min_by` on the distance to the
                            // quarterback (`Nearest`; world speed lane).
                            let me = sim.agents[i].pos;
                            let mut near = Nearest::default();
                            for d in off_n..sim.agents.len() {
                                let a = &sim.agents[d];
                                if a.up(t)
                                    && matches!(a.job, Job::Rush { .. })
                                    && a.engaged.is_none()
                                    && sq_lt(a.pos.dist_sq(me), 4.5)
                                    && sim.can_square(i, d, q)
                                {
                                    near.offer(d, a.pos.dist_sq(q));
                                }
                            }
                            let threat = near.get();
                            if let Some(d) = threat {
                                sim.agents[i].job = Job::PassBlock { target: Some(d) };
                            } else {
                                // Help the nearest teammate's block; the rusher
                                // the plan slides to (A8) from farther, and
                                // before anyone else.
                                let star = sim.help_on;
                                let help = sim
                                    .eng
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, e)| !e.over && e.pass && e.helper.is_none())
                                    // Out of reach on the square (`sq_lt`)
                                    // he is dropped without the root; the
                                    // others keep the distance `dist` gives.
                                    .filter_map(|(k, e)| {
                                        let on_star = Some(e.defender) == star;
                                        let sq = sim.agents[e.defender].pos.dist_sq(me);
                                        if sq_lt(sq, if on_star { 4.5 } else { 2.5 }) {
                                            Some((k, sq.sqrt(), on_star))
                                        } else {
                                            None
                                        }
                                    })
                                    .min_by(|a, b| b.2.cmp(&a.2).then(a.1.total_cmp(&b.1)))
                                    .map(|(k, d, _)| (k, d));
                                if let Some((k, d)) = help {
                                    if d < 1.4 {
                                        let def = sim.eng[k].defender;
                                        sim.engage(i, def, true);
                                    } else {
                                        let dp = sim.agents[sim.eng[k].defender].pos;
                                        sim.go_to(i, dp, 0.8);
                                    }
                                } else {
                                    let hold = v2(los - 1.5, me.y);
                                    sim.go_to(i, hold, 0.5);
                                }
                            }
                        }
                    }
                }
                Job::RunBlock { target, pull } => {
                    let c = sim.carrier();
                    let combo = sim.agents[i].climb.is_some();
                    match target.filter(|&d| sim.agents[d].up(t) && (sim.agents[d].engaged.is_none() || combo)) {
                        Some(d) => {
                            let dp = sim.agents[d].pos;
                            // Get between him and the ball.
                            let ball = c.map(|c| sim.agents[c].pos).unwrap_or(v2(los - 3.0, by));
                            // Climbing to a man on the move he takes an angle
                            // to where he will be, as a pursuer does (he ran
                            // at where the man was, and a third of the
                            // linebackers he was sent to were never blocked;
                            // proposed, not confirmed).
                            let ahead = if (dp.x - los).abs() >= 2.0 {
                                (sim.agents[i].pos.dist(dp) / sim.agents[i].vmax.max(1.0)).min(0.8)
                            } else {
                                0.25
                            };
                            let spot = dp + (ball - dp).norm().scale(0.8) + sim.agents[d].vel.scale(ahead);
                            let lead = if pull { 1.0 } else { 0.95 };
                            // A moving target: run at him, matching his speed when close.
                            let me = sim.agents[i].pos;
                            let gap = spot - me;
                            let vm = sim.agents[i].vmax * lead;
                            let want =
                                if gap.len() > 1.5 { gap.norm().scale(vm) } else { sim.agents[d].vel + gap.scale(3.0) };
                            sim.steer(i, want);
                            // At the line the hands are already close: he fires
                            // out and gets his hands on a lineman at once.
                            let reach = if (dp.x - los).abs() < 2.0 && matches!(sim.agents[d].label, "DE" | "DT" | "NT")
                            {
                                1.6
                            } else {
                                1.2
                            };
                            // From `me` on the square (`sq_lt`): the steer
                            // set his velocity, not his spot, so it is the
                            // same distance and the same answer.
                            if (sq_lt(me.dist_sq(dp), reach) || sim.closest(i, d) < reach) && sim.lands_block(i, d) {
                                sim.engage(i, d, false);
                                // A double team already full: he goes on to
                                // his linebacker.
                                let next =
                                    if sim.agents[i].engaged.is_none() { sim.agents[i].climb.take() } else { None };
                                sim.agents[i].job = Job::RunBlock { target: next, pull: false };
                            }
                        }
                        None => {
                            // Climb to the nearest unblocked defender ahead of the ball.
                            let me = sim.agents[i].pos;
                            let ballx = c.map(|c| sim.agents[c].pos.x).unwrap_or(los);
                            // Before 0.3 s he goes ahead whoever is near, so
                            // nobody is looked for. Then ahead of the ball
                            // first, the distance on the square (`sq_lt`) and
                            // the same pick as `min_by` (`Nearest`): pure
                            // tests, the same man (world speed lane).
                            let near = if t > 0.3 {
                                let mut near = Nearest::default();
                                for d in off_n..sim.agents.len() {
                                    let a = &sim.agents[d];
                                    if a.up(t) && a.engaged.is_none() && a.pos.x > ballx - 1.0 {
                                        let sq = a.pos.dist_sq(me);
                                        if sq_lt(sq, 6.0) {
                                            near.offer(d, sq);
                                        }
                                    }
                                }
                                near.get()
                            } else {
                                None
                            };
                            match near {
                                Some(d) if t > 0.3 => {
                                    sim.agents[i].job = Job::RunBlock { target: Some(d), pull: false }
                                }
                                _ => {
                                    let ahead = v2(me.x + 1.0, me.y);
                                    sim.go_to(i, ahead, 0.5);
                                }
                            }
                        }
                    }
                }
                Job::Escort => escort(&mut sim, i),
                Job::Pursue => {
                    if let Some(c) = sim.carrier() {
                        sim.pursue(i, c);
                    }
                }
                _ => {}
            }
        }

        // ---- quarterback on a pass play
        if let Some(ps) = pass.as_mut()
            && ps.thrown.is_none()
            && !ps.scramble
            && sim.carrier() == Some(qb)
            && !sim.dead
        {
            if let Some(th) = quarterback(&mut sim, ps, qb, s, center) {
                // A lineman too far downfield when a pass crosses the line
                // (an RPO pulled late): the flag, when the crew sees it.
                if run_play && th.0.x > los {
                    let far = (0..off_n)
                        .find(|&k| is_line(sim.agents[k].label) && sim.agents[k].pos.x - los > s.tac.downfield);
                    if let Some(k) = far
                        && sim
                            .rng
                            .chance(0.6 * clamp(1.0 - 0.015 * (sim.agents[k].a(Attr::FootballIq) - 60.0), 0.4, 1.4))
                    {
                        let (id, at) = (sim.agents[k].id, sim.agents[k].pos.x);
                        sim.notes.fouls.push(Foul { kind: FoulKind::IneligibleDownfield, side: 0, player: id, at });
                    }
                }
                throw_info = Some(th);
                throw_start = Some(t);
                thrown_from = sim.agents[qb].pos.x;
            }
            if ps.scramble {
                res.result = Result::Scramble;
                res.carrier = Some(sim.agents[qb].id);
                sim.agents[qb].job = Job::Carrier;
                aim = None;
            }
        }

        // ---- the option pitch: the quarterback reads the man on the edge
        if option_live
            && let Some(pm) = pitch_man
            && sim.carrier() == Some(qb)
            && !sim.dead
            && t > 0.4
        {
            match option_pitch(&mut sim, qb, pm, s.call.dir, off_n) {
                Some(true) => {
                    option_live = false;
                    sim.agents[qb].job = Job::RunBlock { target: None, pull: false };
                    sim.agents[pm].job = Job::Carrier;
                    sim.ball = Ball::Held(pm);
                    sim.progress = sim.agents[pm].pos.x;
                    res.carrier = Some(sim.agents[pm].id);
                    aim = None;
                    sim.notes.pitched = true;
                    // A pitch on the ground is a live ball.
                    let carry = sim.agents[pm].a(Attr::Catching);
                    if sim.rng.chance(0.012 * (1.6 - carry / 100.0)) {
                        sim.notes.fumbled = Some(sim.agents[pm].id);
                        sim.fumble(pm);
                    }
                }
                Some(false) => option_live = false,
                None => {}
            }
        }

        // ---- defense
        for i in off_n..sim.agents.len() {
            if !sim.agents[i].up(t) || sim.agents[i].engaged.is_some() {
                continue;
            }
            defend(&mut sim, i, s, qb, &pass, &trail, in_air);
        }

        // ---- ball carrier
        if let Some(c) = sim.carrier()
            && matches!(sim.agents[c].job, Job::Carrier)
        {
            let mut a = if (sim.agents[c].pos.x - los) * sim.goal < -0.3 || t < 0.9 { aim } else { None };
            if let (Some(h), Some(g)) = (a, lead_puller) {
                // Patience: he doesn't pass the puller on the way to the
                // hole; he stays on his inside hip until the block is made.
                let gp = sim.agents[g].pos;
                let short = (h.y - gp.y) * s.call.dir > 1.0;
                let led = (gp.y - sim.agents[c].pos.y) * s.call.dir > 0.0;
                if sim.agents[g].up(t) && sim.agents[g].engaged.is_none() && short && led {
                    a = Some(v2(gp.x - sim.goal * 1.0, gp.y - s.call.dir * 0.8));
                }
            }
            if s.hurry && sim.agents[c].side == 0 {
                hurry_run(&mut sim, c);
            } else {
                sim.run_carrier(c, a);
            }
            // Where and when he crossed the line on a designed run.
            if aim.is_some()
                && pass.is_none()
                && sim.notes.tend.cross.is_none()
                && (sim.agents[c].pos.x - los) * sim.goal >= 0.0
            {
                sim.notes.tend.cross = Some((sim.agents[c].pos.y - sim.mood.snap_y, t));
            }
            // A quarterback slides rather than take a hit he doesn't need.
            if pass.is_some() && c == qb && slide(&mut sim, c, s) {
                res.result = Result::Scramble;
            }
        }

        sim.advance();

        // ---- the ball in the air
        if let Ball::Air { to, arrive, target, release, from, .. } = sim.ball
            && t + 1e-9 >= arrive
        {
            let tgt = target.unwrap_or(qb);
            let batted = pass.as_ref().and_then(|ps| ps.batted);
            let outcome = match batted {
                // Batted at the line: knocked down, or now and then caught by
                // the man who tipped it.
                Some((d, _)) => {
                    let sep = (off_n..sim.agents.len())
                        .map(|k| sim.agents[k].pos.dist(sim.agents[tgt].pos))
                        .fold(f64::INFINITY, f64::min);
                    if sim.rng.chance(0.04 * sim.agents[d].a(Attr::Catching) / 50.0) {
                        Catch::Intercepted(d)
                    } else {
                        Catch::Incomplete { drop: false, pbu: Some(sim.agents[d].id), sep }
                    }
                }
                None => {
                    let touch = sim.agents[qb].a(Attr::Touch);
                    let high = pass.as_ref().map_or((0.0, false), |ps| (ps.ball_h, ps.jump_ball));
                    catch_point(&mut sim, tgt, to, from, release, touch, s, &trail, high)
                }
            };
            res.target = Some(sim.agents[tgt].id);
            // The defender in coverage on the throw: the nearest one to the
            // catch point, when he is close enough to be charged with it.
            res.defender = (off_n..sim.agents.len())
                .map(|k| (k, sim.agents[k].pos.dist(to)))
                .filter(|&(_, d)| d < 4.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(k, _)| sim.agents[k].id);
            let meant = batted.map_or(to, |b| b.1);
            catch_x = meant.x;
            res.air_yards = meant.x - los;
            res.throw_y = Some((sim.mood.snap_y as f32, meant.y as f32));
            if let Job::Route { name, .. } = sim.agents[tgt].job {
                res.route = name;
            }
            match outcome {
                Catch::Complete(sep) => {
                    res.result = Result::Complete;
                    res.separation = sep;
                    sim.ball = Ball::Held(tgt);
                    sim.agents[tgt].job = Job::Carrier;
                    res.carrier = Some(sim.agents[tgt].id);
                    for k in 0..off_n {
                        if k != tgt && sim.agents[k].engaged.is_none() {
                            sim.agents[k].job = Job::Escort;
                        }
                    }
                    for k in off_n..sim.agents.len() {
                        sim.agents[k].job = Job::Pursue;
                    }
                    sim.progress = to.x;
                    // A catch at the sideline or the end line needs his feet
                    // in (topic 5).
                    if to.y < 0.6 || to.y > width() - 0.6 || to.x > end_line() - 0.6 {
                        let feet = s.rules.feet_inbounds as f64;
                        let aw = 0.6 * sim.agents[tgt].a(Attr::BodyControl) + 0.4 * sim.agents[tgt].a(Attr::FootballIq);
                        if !sim.rng.chance(clamp(0.35 + 0.006 * aw - 0.15 * (feet - 1.0), 0.1, 0.95)) {
                            res.result = Result::Incomplete;
                            sim.dead = true;
                        } else {
                            sim.out_of_bounds = true;
                            sim.dead = true;
                            sim.ball = Ball::Dead(to);
                        }
                    }
                }
                Catch::Incomplete { drop, pbu, sep } => {
                    res.result = Result::Incomplete;
                    res.drop = drop;
                    res.pbu = pbu;
                    res.separation = sep;
                    sim.dead = true;
                }
                Catch::Intercepted(d) => {
                    res.result = Result::Interception;
                    res.turnover = true;
                    res.returner = Some(sim.agents[d].id);
                    sim.ball = Ball::Held(d);
                    sim.goal = -1.0;
                    sim.carrier_side = 1;
                    sim.progress = sim.agents[d].pos.x;
                    sim.agents[d].job = Job::Carrier;
                    for k in 0..sim.agents.len() {
                        if k == d {
                            continue;
                        }
                        sim.agents[k].job = if sim.agents[k].side == 1 { Job::Escort } else { Job::Pursue };
                    }
                }
            }
            let _ = release;
        }

        // ---- tackles, sideline, end zone
        if let Some(c) = sim.carrier()
            && !sim.dead
        {
            let qb_pocket = c == qb && pass.as_ref().is_some_and(|p| p.thrown.is_none() && !p.scramble);
            if qb_pocket {
                // A sack attempt: the rusher against the quarterback's escape.
                sack_check(&mut sim, c, pass.as_mut().unwrap(), los);
            } else {
                sim.tackles(c);
            }
            if !sim.dead {
                sim.check_carrier(c);
            }
        }
        if let Some(f) = sim.notes.fumbled
            && sim.dead
            && res.carrier.is_none()
        {
            res.carrier = Some(f);
        }
        sim.record_frame();
        // Debug tooling only: the tenth is worked out only when tracing
        // (`trace` does nothing otherwise; world speed lane).
        if super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed) && ((sim.t * 10.0).round() as i64) % 5 == 0 {
            sim.trace();
        }
    }
    // The trails are not read after the play: their buffers go back for
    // the next snap on this thread.
    TRAILS.set(trail);
    if !sim.dead {
        // Time ran out on the sim (a long scramble): down where he is.
        sim.dead = true;
    }
    sim.close_blocks();

    // ---------------------------------------------------------- result
    let p = pass.as_ref();
    res.duration = sim.t;
    res.out_of_bounds = sim.out_of_bounds;
    if let Some(ps) = p {
        res.time_to_throw = ps.thrown.unwrap_or(sim.t);
        res.pressured = ps.pressured_at.is_some();
        res.moving = ps.moving && ps.thrown.is_some();
        res.hit = ps.hit;
        if ps.thrown.is_none() && !ps.scramble && res.result != Result::Scramble {
            res.result = Result::Sack;
            res.carrier = Some(sim.agents[qb].id);
        }
        if let Some((_, _, _, _, _)) = throw_info
            && ps.target.is_none()
        {
            res.throw_away = true;
        }
        if res.result == Result::Scramble || res.result == Result::Sack {
            res.passer = Some(sim.agents[qb].id);
        }
    }
    let _ = (throw_start, thrown_from);
    // A ball thrown away is meant for nobody: no target (not the passer).
    if res.throw_away {
        res.result = Result::Incomplete;
        res.target = None;
    }
    let goal_line = goal();
    if res.turnover {
        // A return the other way.
        res.spot = sim.progress;
        if sim.progress <= 0.0 {
            res.defense_td = true;
        }
    } else if let Some((side, _)) = sim.notes.recovered {
        res.spot = sim.progress;
        if side == 1 {
            res.turnover = true;
            res.returner = sim.notes.recovered.map(|r| r.1);
        }
    } else {
        match res.result {
            Result::Incomplete => res.spot = los,
            _ => {
                res.spot = sim.progress.min(goal_line);
                if res.spot >= goal_line {
                    res.touchdown = true;
                }
                if res.spot <= 0.0 {
                    res.safety = true;
                }
            }
        }
    }
    if res.result == Result::Complete {
        res.yac = res.spot - catch_x;
    }
    if let Some(fc) = sim.notes.first_contact {
        res.before_contact = fc - los;
    } else if matches!(res.result, Result::Run | Result::Scramble | Result::Complete) {
        res.before_contact = res.spot - los;
    }
    res.spot_y = match sim.ball {
        Ball::Dead(p) => p.y,
        _ => sim.ball_pos().y,
    };
    // Room for every player first: the same entries, one allocation each.
    res.work.reserve(sim.agents.len());
    sim.notes.top_speed.reserve(sim.agents.len());
    for a in &sim.agents {
        res.work.push((a.side, a.p, a.work));
        sim.notes.top_speed.push((a.id, a.top));
    }
    res.notes = std::mem::take(&mut sim.notes);
    res.frames = sim.record.take();
    res
}

// ------------------------------------------------------------- alignment

/// The punt team in its punt formation (a fake punt, G18): the long
/// snapper and four linemen with the usual splits, a wing a step outside
/// and behind each tackle, the gunners split wide, the up-back about five
/// and a half yards behind the ball and the punter at his depth (about 15
/// yards in the NFL, 14 in college, 12 at high school).
fn punt_spot(f: &Formation, r: ORole, los: f64, by: f64, level: crate::Level) -> V2 {
    let deep = match level {
        crate::Level::NFL => 15.0,
        crate::Level::HS => 12.0,
        _ => 14.0,
    };
    match r {
        ORole::QB => v2(los - 5.5, by),
        ORole::RB => v2(los - deep, by),
        ORole::TE => v2(los - 1.3, by + 4.4),
        ORole::TE2 => v2(los - 1.3, by - 4.4),
        // A twelfth man (the CFL) is a second up-back.
        ORole::Slot | ORole::Slot2 | ORole::Slot3 | ORole::FB => v2(los - 4.0, by - 1.8),
        _ => align(f, r, los, by),
    }
}

/// The punt return team against the punt formation (a fake punt, G18):
/// the returner deep where the punt would come down (about 40 yards off
/// in the NFL, 38 in college and 32 at high school, never inside his own
/// 10), a jammer on each gunner, the four biggest men head up on the
/// guards and tackles, two more on the line over the wings and two held
/// back five yards off the ball. In `s.defense` order.
fn return_spots(s: &Snap, agents: &[Agent], roles: &[Option<ORole>]) -> Vec<V2> {
    let (los, by) = (s.los, s.ball_y);
    let deep: f64 = match s.rules.level {
        crate::Level::NFL => 40.0,
        crate::Level::HS => 32.0,
        _ => 38.0,
    };
    let deep = deep.min(goal() - los - 10.0).max(8.0);
    let mut gunners: Vec<V2> =
        (0..agents.len()).filter(|&i| matches!(roles[i], Some(ORole::X | ORole::Z))).map(|i| agents[i].pos).collect();
    let mut extra = 0.0;
    s.defense
        .iter()
        .map(|&(d, _)| match d {
            DSlot::FS => v2(los + deep, by),
            DSlot::CBL | DSlot::CBR | DSlot::NCB | DSlot::DCB => {
                let want = if d == DSlot::CBR { -1.0 } else { 1.0 };
                let k = gunners.iter().position(|g| (g.y - by) * want > 0.0).or(if gunners.is_empty() {
                    None
                } else {
                    Some(0)
                });
                match k {
                    Some(k) => {
                        let g = gunners.remove(k);
                        v2(los + 1.5, g.y)
                    }
                    None => {
                        extra += 1.0;
                        v2(los + 8.0, by + 6.0 * extra)
                    }
                }
            }
            DSlot::DTL => v2(los + 1.0, by + 1.5),
            DSlot::DTR | DSlot::NT => v2(los + 1.0, by - 1.5),
            DSlot::LE => v2(los + 1.0, by + 3.6),
            DSlot::RE => v2(los + 1.0, by - 3.6),
            DSlot::Sam => v2(los + 1.0, by + 6.2),
            DSlot::Will => v2(los + 1.0, by - 6.2),
            DSlot::Mike => v2(los + 5.0, by + 1.8),
            DSlot::SS => v2(los + 5.0, by - 1.8),
        })
        .collect()
}

/// The return team's jobs on a fake punt (G18): the returner waits deep,
/// each jammer stays on his gunner, and the front either turns and runs
/// back to set up the return (the look the fake was called on) or holds
/// up at the line with the two men behind it watching for the fake. Each
/// comes back to the ball at his own moment of reading it. The punter
/// stays where he stood.
fn return_jobs(sim: &mut Sim, s: &Snap, off_n: usize, slots: &[Option<DSlot>], roles: &[Option<ORole>], look: bool) {
    let (los, by) = (s.los, s.ball_y);
    for i in 0..off_n {
        if roles[i] == Some(ORole::RB) {
            sim.agents[i].job = Job::Idle;
        }
    }
    let mut gunners: Vec<usize> = (0..off_n).filter(|&i| matches!(roles[i], Some(ORole::X | ORole::Z))).collect();
    for i in off_n..sim.agents.len() {
        let me = sim.agents[i].pos;
        let slot = slots[i].unwrap_or(DSlot::Mike);
        let side = if me.y > by { 1.0 } else { -1.0 };
        sim.agents[i].job = match slot {
            DSlot::FS => Job::Zone { spot: me, deep: true, width: 20.0, matched: false },
            DSlot::CBL | DSlot::CBR | DSlot::NCB | DSlot::DCB => {
                let near = gunners
                    .iter()
                    .copied()
                    .min_by(|&a, &b| sim.agents[a].pos.dist(me).total_cmp(&sim.agents[b].pos.dist(me)));
                match near {
                    Some(g) => {
                        gunners.retain(|&x| x != g);
                        man_job(sim, i, g)
                    }
                    None => Job::Zone { spot: v2(los + 12.0, me.y), deep: false, width: 8.0, matched: false },
                }
            }
            // The look: back to set up the wall, in his own lane.
            _ if look => {
                Job::Zone { spot: v2(los + 15.0, by + 0.6 * (me.y - by)), deep: false, width: 8.0, matched: false }
            }
            DSlot::Mike | DSlot::SS => Job::Zone { spot: me, deep: false, width: 6.0, matched: false },
            _ => {
                let edge = matches!(slot, DSlot::Sam | DSlot::Will);
                Job::Rush { lane: me.y, contain: if edge { side } else { 0.0 }, mv: Move::Power }
            }
        };
    }
}

/// A safety's hitter read (S-A, Cain's yes 2026-11-07): what his staff sees
/// of a man built to play the run. His weight over his level's typical
/// safety in tens of pounds, plus his hitting ratings (Tackling, Strength,
/// Play Recognition) over his range ones (Speed, Zone and Man Coverage)
/// against the typical safety's gap, twenty points a step. 0 for the
/// typical safety.
pub(crate) fn hitter(level: crate::player::Level, p: &GamePlayer) -> f64 {
    use crate::positions::Position as P;
    hit_read(p, &|a| super::synthetic::typical(level, P::S, a), super::synthetic::typical_body(level, P::S).1)
}

/// The hitter read against a typical safety's ratings and weight.
pub(crate) fn hit_read(p: &GamePlayer, typ: &dyn Fn(Attr) -> f64, typ_weight: f64) -> f64 {
    let gap = |g: &dyn Fn(Attr) -> f64| {
        (g(Attr::Tackling) + g(Attr::Strength) + g(Attr::PlayRecognition)) / 3.0
            - (g(Attr::Speed) + g(Attr::ZoneCoverage) + g(Attr::ManCoverage)) / 3.0
    };
    (p.weight - typ_weight) / 10.0 + (gap(&|a| p.attrs.get(a)) - gap(typ)) / 20.0
}

/// A draw in 0-1 for one defender's alignment on this snap, with no random
/// stream: the same snap gives the same number.
fn snap_jitter(los: f64, k: usize) -> f64 {
    let mut z =
        ((los * 10.0) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (k as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// How far outside the ball a run must get before the back on that side
/// takes the force (minor bug 253; 3 and 5 yards moved outside runs by 0.1
/// a carry either way; proposed, not confirmed).
const FORCE_WIDE: f64 = 4.0;

/// The rotated-down strong safety's depth (S-A): a hitter sits shallower, a
/// rangy one deeper, a yard a point of his read over the level's typical
/// strong safety, and 0.9 either way snap to snap.
fn ss_depth(s: &Snap, di: usize) -> f64 {
    let h = hitter(s.rules.level, &s.teams[1][di])
        - super::synthetic::typical_hitter(s.rules.level, crate::spots::Spot::SS);
    let j = snap_jitter(s.los, di) * 2.0 - 1.0;
    clamp(8.1 - h + 0.9 * j, 5.0, 10.0)
}

fn align_defense(s: &Snap, agents: &[Agent], roles: &[Option<ORole>]) -> Vec<V2> {
    let los = s.los;
    let by = s.ball_y;
    let st = s.call.formation.strength;
    let find = |r: ORole| roles.iter().position(|x| *x == Some(r)).map(|i| agents[i].pos);
    let lt = find(ORole::LT).unwrap_or(v2(los, by + 2.8));
    let rt = find(ORole::RT).unwrap_or(v2(los, by - 2.8));
    let (strong_t, weak_t) = if st > 0.0 { (lt, rt) } else { (rt, lt) };
    let te_strong = [ORole::TE, ORole::TE2, ORole::TE3].iter().filter_map(|r| find(*r)).find(|p| (p.y - by) * st > 0.0);
    // Receivers by side, outside in, and whether he is the deep threat the
    // defense fears (S10a).
    let fear = s.tac.fear;
    let mut left: Vec<(V2, bool)> = Vec::new();
    let mut right: Vec<(V2, bool)> = Vec::new();
    let mut burner_y: Option<f64> = None;
    // In zone nobody lines up over the jet man in mid-motion (minor bug
    // 342): the zone defenders set to where he stood before it, and the
    // strong safety spins with the motion (below). Before, the motion man's
    // spot at the snap pulled a zone defender inside with him.
    let jet = if s.call.formation.motion == Motion::Jet && !s.dcall.coverage.is_man() {
        s.call.formation.motion_man()
    } else {
        None
    };
    for (i, r) in roles.iter().enumerate() {
        if let Some(r) = r
            && matches!(r, ORole::X | ORole::Z | ORole::Slot | ORole::Slot2 | ORole::Slot3)
        {
            let pos = if jet == Some(*r) {
                align(&Formation { motion: Motion::None, ..s.call.formation }, *r, los, by)
            } else {
                agents[i].pos
            };
            let feared = fear > 0.0 && s.tac.burner == Some(agents[i].p);
            if feared {
                burner_y = Some(pos.y);
            }
            if pos.y > by { left.push((pos, feared)) } else { right.push((pos, feared)) }
        }
    }
    left.sort_by(|a, b| b.0.y.total_cmp(&a.0.y));
    right.sort_by(|a, b| a.0.y.total_cmp(&b.0.y));
    // Press in man; a Cover 2 corner rolls up to about 5 yards (a cloud
    // corner: he jams, then takes the flat); other zones play off. (Cover 2
    // corners pressed at the line and squatted every short throw: quick outs
    // were caught 43% of the time against them, real 77%.)
    let press = matches!(s.dcall.coverage, Coverage::Cover0 | Coverage::Cover1 | Coverage::Cover2Man);
    let cush = if press {
        1.2
    } else if matches!(s.dcall.coverage, Coverage::Cover2 | Coverage::Tampa2) {
        5.0
    } else {
        6.5
    };
    // Against a known burner the corner gives him room (S10a): real corners
    // play an elite deep threat 8 to 10 yards off; in press man he bails to
    // a soft press; a Cover 2 corner still squats with help over him.
    let cloud = matches!(s.dcall.coverage, Coverage::Cover2 | Coverage::Tampa2);
    let room = |feared: bool| -> f64 {
        if !feared {
            cush
        } else if press {
            cush + 6.0 * (fear - 0.5).max(0.0)
        } else if cloud {
            cush
        } else {
            cush + 3.0 * fear
        }
    };
    let wide = |list: &Vec<(V2, bool)>, k: usize, side: f64| -> V2 {
        match list.get(k) {
            Some(&(p, feared)) => v2(los + room(feared), p.y - 0.8 * side),
            None => v2(los + 6.0, by + side * (7.0 + 4.0 * k as f64)),
        }
    };
    let slot_room = |feared: bool| if feared { 3.0 * fear } else { 0.0 };
    // The deep safety leans to the burner's side before the snap (S10b),
    // which takes help from the middle, and the safeties play deeper (up to
    // 1.5 yards in two-high, 2 in single-high), which leaves room underneath
    // for his teammates (proposed, not confirmed).
    let lean = burner_y.map_or(0.0, |y| clamp((y - by) * 0.3 * fear, -5.0, 5.0));
    let two_high = s.dcall.coverage.high() == 2 || s.dcall.disguise;
    let three_four =
        s.defense.iter().any(|(d, _)| *d == DSlot::NT) && s.defense.iter().filter(|(d, _)| d.is_line()).count() == 3;
    // The front's techniques (topic 1): where each lineman shades.
    let look = s.dcall.front;
    s.defense
        .iter()
        .map(|(d, di)| {
            // A yard further off in the CFL.
            let dl = los + 0.9 + s.rules.code.line_off;
            match d {
                DSlot::LE | DSlot::RE => {
                    let side = if *d == DSlot::LE { 1.0 } else { -1.0 };
                    let strong = side * st > 0.0;
                    let t = if strong { strong_t } else { weak_t };
                    let y = match (look, strong, te_strong) {
                        // Wide nine: the ends split far outside.
                        (FrontLook::Wide, _, Some(te)) if strong => te.y + side * 2.6,
                        (FrontLook::Wide, _, _) => t.y + side * 3.4,
                        // Under: the strong end heads up on the tackle.
                        (FrontLook::Under, true, _) => t.y + side * 0.6,
                        // Bear: the ends cover the tight end spots.
                        (FrontLook::Bear, _, _) => t.y + side * 2.0,
                        (_, true, Some(te)) => te.y + side * 1.0,
                        _ => t.y + side * if three_four { 2.2 } else { 1.4 },
                    };
                    v2(dl, y)
                }
                DSlot::DTL | DSlot::DTR => {
                    let side = if *d == DSlot::DTL { 1.0 } else { -1.0 };
                    let strong = side * st > 0.0;
                    match look {
                        // Tite: two 4i techniques inside the tackles.
                        FrontLook::Tite => v2(dl, by + side * 2.0),
                        // Bear: center and both guards covered.
                        FrontLook::Bear if !three_four => v2(dl, by + side * 1.5),
                        _ if three_four => v2(dl, by + side * 2.6),
                        // Under: the 3 technique goes to the weak side.
                        FrontLook::Under => v2(dl, by + side * if strong { 0.7 } else { 2.0 }),
                        _ => v2(dl, by + side * if strong { 2.0 } else { 0.7 }),
                    }
                }
                DSlot::NT => v2(dl, by),
                DSlot::Sam => v2(los + 4.5, by + st * 3.8),
                DSlot::Mike => v2(los + 4.8, by + st * 0.6),
                DSlot::Will => v2(los + 4.5, by - st * 2.8),
                DSlot::CBL => wide(&left, 0, 1.0),
                DSlot::CBR => wide(&right, 0, -1.0),
                DSlot::NCB => {
                    let (list, side) = if st > 0.0 { (&left, 1.0) } else { (&right, -1.0) };
                    match list.get(1) {
                        Some(&(p, feared)) => v2(los + 4.0 + slot_room(feared), p.y - 0.8 * side),
                        None => v2(los + 5.0, by + side * 5.5),
                    }
                }
                DSlot::DCB => {
                    let (list, side) = if st > 0.0 { (&right, -1.0) } else { (&left, 1.0) };
                    match list.get(1) {
                        Some(&(p, feared)) => v2(los + 4.0 + slot_room(feared), p.y - 0.8 * side),
                        None => v2(los + 6.0, by + side * 5.5),
                    }
                }
                DSlot::FS => {
                    if two_high {
                        let y = mid() - 9.0 * st;
                        let near = burner_y.is_some_and(|b| (b - by) * (y - mid()) > 0.0);
                        v2(los + 12.5 + 1.5 * fear + if near { fear } else { 0.0 }, y + 0.5 * lean)
                    } else {
                        v2(los + 13.0 + 2.0 * fear, by + lean)
                    }
                }
                DSlot::SS => {
                    if two_high {
                        let y = mid() + 9.0 * st;
                        let near = burner_y.is_some_and(|b| (b - by) * (y - mid()) > 0.0);
                        v2(los + 12.5 + 1.5 * fear + if near { fear } else { 0.0 }, y + 0.5 * lean)
                    } else {
                        // The rotated-down safety (S-A): a hitter sits
                        // shallower, a rangy one deeper, a yard a point of
                        // his read over the level's typical strong safety
                        // (staffs list their hitter there), and 0.9 either
                        // way snap to snap. The typical man stays just
                        // outside the box count (7 yards), a heavy one steps
                        // inside it (proposed, not confirmed).
                        v2(los + ss_depth(s, *di), by + st * 4.5)
                    }
                }
            }
        })
        .zip(s.defense.iter())
        .map(|(p, (d, di))| {
            // The defense travels with motion (topic 3): against jet or
            // across motion the strong safety spins down toward where it
            // is going and the linebackers bump a gap that way; man
            // defenders follow their men (in `assign_defense`). Against an
            // orbit the spin goes the other way (proposed, not confirmed).
            let mv = match s.call.formation.motion {
                Motion::Jet | Motion::Across => -st,
                Motion::Orbit => st,
                _ => 0.0,
            };
            if mv == 0.0 {
                return p;
            }
            match d {
                DSlot::SS if !two_high || s.dcall.coverage.high() < 2 => {
                    // He spins at his own depth (minor bug 342): the typical
                    // man stays just outside the box, a hitter steps into
                    // it. Before, every spin put him at 5.5 yards, inside
                    // the box count, so motion alone made an eight-man box.
                    v2(los + ss_depth(s, *di), by + mv * 6.0)
                }
                DSlot::Sam | DSlot::Mike | DSlot::Will => v2(p.x, p.y + mv * 1.2),
                DSlot::NCB | DSlot::DCB if (p.y - by) * mv > 0.0 => v2(p.x - 1.0, p.y),
                _ => p,
            }
        })
        .map(|p: V2| {
            // Near the goal line the defense tightens: corners and
            // linebackers come up, safeties sit at the back of the end zone.
            let room = end_line() - los;
            let depth = p.x - los;
            let cap = if depth < 9.0 { 1.0 + 0.25 * room } else { 0.75 * room };
            v2(los + depth.min(cap), p.y)
        })
        .collect()
}

// ------------------------------------------------------------- jobs

fn route_job(a: &Agent, route: Route, los: f64, by: f64, hurry: bool) -> Job {
    if route.blocks() {
        return if matches!(a.label, "WR") {
            Job::RunBlock { target: None, pull: false }
        } else {
            Job::PassBlock { target: None }
        };
    }
    let out = if a.pos.y >= by { 1.0 } else { -1.0 };
    let mut route = route;
    if hurry {
        // In the two-minute drill, routes break to the sideline.
        route = match route {
            Route::In(d) => Route::Out(d),
            Route::Curl(d) => Route::Comeback(d.max(10.0)),
            Route::Drag => Route::Flat,
            r => r,
        };
    }
    let (mut pts, settle) = match route {
        Route::CheckRelease => (vec![a.pos + v2(1.5, 3.5 * out), a.pos + v2(4.0, 5.0 * out)], true),
        Route::Screen => (vec![a.pos + v2(0.5, 4.0 * out), a.pos + v2(0.5, 7.0 * out)], true),
        r => r.points(a.pos, los, out),
    };
    // Near the goal line a route ends inside the back of the end zone: a
    // go route becomes a fade to the back line, never a run through it.
    for p in &mut pts {
        p.x = p.x.min(end_line() - 1.0);
    }
    let delay = match route {
        Route::CheckRelease => 1.0,
        Route::Screen => 0.9,
        Route::Bubble | Route::Tunnel => 0.0,
        // A man in motion has his running start.
        _ if matches!(a.label, "WR" | "TE") && a.vel.len() < 1.5 => footwork(a),
        _ => 0.0,
    };
    Job::Route { pts, next: 0, settle, name: route.name(), release_delay: delay }
}

fn assign_defense(
    sim: &mut Sim,
    s: &Snap,
    off_n: usize,
    slots: &[Option<DSlot>],
    roles: &[Option<ORole>],
    _pass: bool,
) {
    let los = s.los;
    let by = s.ball_y;
    let st = s.call.formation.strength;
    let cov = s.dcall.coverage;
    let n = sim.agents.len();
    // Eligible receivers, outside in on each side.
    let mut recv: Vec<usize> = (0..off_n).filter(|&i| roles[i].is_some_and(|r| r.is_receiver())).collect();
    recv.sort_by(|&a, &b| (sim.agents[b].pos.y - by).abs().total_cmp(&(sim.agents[a].pos.y - by).abs()));
    let back = (0..off_n).find(|&i| roles[i] == Some(ORole::RB));
    let mut taken: Vec<usize> = Vec::new();
    let pick_wide = |sim: &Sim, taken: &Vec<usize>, side: f64| -> Option<usize> {
        recv.iter()
            .copied()
            .filter(|r| !taken.contains(r) && (sim.agents[*r].pos.y - by) * side > 0.0)
            .max_by(|&a, &b| ((sim.agents[a].pos.y - by) * side).total_cmp(&((sim.agents[b].pos.y - by) * side)))
    };
    // Who rushes (topic 3): the line, plus blitzers.
    let mut blitzers: Vec<DSlot> = match s.dcall.pressure {
        Pressure::Four => vec![],
        Pressure::LbBlitz | Pressure::Simulated => vec![lb_blitzer(sim, s, off_n, slots)],
        Pressure::DbBlitz => vec![db_blitzer(sim, s, off_n, slots)],
        Pressure::DoubleA => {
            vec![DSlot::Mike, if slots.contains(&Some(DSlot::Will)) { DSlot::Will } else { DSlot::Sam }]
        }
        // A fire zone (topic 1): a linebacker and a defensive back come, the
        // weak end drops, and three deep and three under play behind it.
        Pressure::FireZone => vec![
            if slots.contains(&Some(DSlot::Will)) { DSlot::Will } else { DSlot::Mike },
            if slots.contains(&Some(DSlot::NCB)) { DSlot::NCB } else { DSlot::SS },
        ],
    };
    if cov == Coverage::Cover0 {
        blitzers.push(DSlot::FS);
        blitzers.push(DSlot::Mike);
    }
    blitzers.retain(|b| slots.contains(&Some(*b)));
    // In simulated pressure the weak end drops into the flat.
    let dropper = if matches!(s.dcall.pressure, Pressure::Simulated | Pressure::FireZone) {
        Some(if st > 0.0 { DSlot::RE } else { DSlot::LE })
    } else {
        None
    };

    // Near the goal the field shrinks: deep zones sit at the back of the end
    // zone and the underneath ones squeeze down with them.
    let zone = |spot: V2, deep: bool, width: f64| {
        let back = end_line() - 2.0;
        let room = back - los;
        let x = if deep { spot.x.min(back) } else { spot.x.min(los + 0.55 * room) };
        Job::Zone { spot: v2(x, spot.y), deep, width, matched: false }
    };
    let side_y = |side: f64, from_sideline: f64| if side > 0.0 { width() - from_sideline } else { from_sideline };
    let deep_third = |side: f64| zone(v2(los + 16.0, side_y(side, 9.5)), true, 14.0);
    let deep_mid = zone(v2(los + 17.0, by * 0.5 + mid() * 0.5), true, 18.0);
    let deep_half = |side: f64| zone(v2(los + 16.0, mid() + side * 9.5), true, 26.0);
    let quarter_out = |side: f64| zone(v2(los + 13.0, side_y(side, 9.0)), true, 13.0);
    let quarter_in = |side: f64| zone(v2(los + 13.0, mid() + side * 4.5), true, 13.0);
    let curl_flat = |side: f64| zone(v2(los + 9.0, by + side * 10.0), false, 12.0);
    // The flat: from the sideline to about the numbers (it sat on the
    // numbers, right on top of the outside receiver's hitch).
    let flat = |side: f64| zone(v2(los + 4.0, side_y(side, 6.0)), false, 8.0);
    let hook = |side: f64| zone(v2(los + 10.0, by + side * 4.5), false, 9.0);
    let middle = zone(v2(los + 10.5, by), false, 9.0);

    // Against a deep threat he fears, the coordinator puts his better deep
    // corner on that man's side in man coverage (C1): the faster and taller
    // of his two outside corners, read as his staff sees them (proposed,
    // not confirmed: as often as he fears the burner).
    let travel = cov.is_man() && s.tac.fear > 0.0 && {
        let deep = |d: DSlot| {
            (off_n..n).find(|&i| slots[i] == Some(d)).map(|i| {
                let g = &s.teams[1][sim.agents[i].p];
                sim.agents[i].vmax0 + 0.08 * g.height
            })
        };
        let burner_side = (0..off_n)
            .find(|&i| s.tac.burner == Some(sim.agents[i].p))
            .map(|i| if sim.agents[i].pos.y > by { 1.0 } else { -1.0 });
        match (deep(DSlot::CBL), deep(DSlot::CBR), burner_side) {
            (Some(l), Some(r), Some(bs)) => {
                let better = if l >= r { 1.0 } else { -1.0 };
                better != bs && sim.rng.chance(s.tac.fear.min(1.0))
            }
            _ => false,
        }
    };
    let flip = if travel { -1.0 } else { 1.0 };
    let cb_side = |d: DSlot| flip * if d == DSlot::CBL { 1.0 } else { -1.0 };
    for i in off_n..n {
        let d = slots[i].unwrap();
        if d.is_line() && Some(d) != dropper || blitzers.contains(&d) {
            let side = if sim.agents[i].pos.y > by { 1.0 } else { -1.0 };
            let edge = matches!(d, DSlot::LE | DSlot::RE) || d.is_db();
            sim.agents[i].job =
                Job::Rush { lane: sim.agents[i].pos.y, contain: if edge { side } else { 0.0 }, mv: Move::Speed };
            continue;
        }
        if Some(d) == dropper {
            let side = if sim.agents[i].pos.y > by { 1.0 } else { -1.0 };
            sim.agents[i].job = curl_flat(side);
            continue;
        }
        let side_of = |sim: &Sim| if sim.agents[i].pos.y > by { 1.0 } else { -1.0 };
        sim.agents[i].job = if cov.is_man() {
            // Man: corners on the outside men, nickels on slots, safety and
            // linebackers on tight ends and backs; one deep player in Cover 1.
            match d {
                DSlot::CBL | DSlot::CBR => {
                    let side = cb_side(d);
                    match pick_wide(sim, &taken, side) {
                        Some(r) => {
                            taken.push(r);
                            man_job(sim, i, r)
                        }
                        None => hook(side),
                    }
                }
                DSlot::NCB | DSlot::DCB => {
                    let side = if (d == DSlot::NCB) == (st > 0.0) { 1.0 } else { -1.0 };
                    match pick_wide(sim, &taken, side).or_else(|| pick_wide(sim, &taken, -side)) {
                        Some(r) => {
                            taken.push(r);
                            man_job(sim, i, r)
                        }
                        None => middle.clone(),
                    }
                }
                DSlot::FS if cov != Coverage::Cover0 => {
                    if cov == Coverage::Cover2Man {
                        deep_half(-st)
                    } else {
                        deep_mid.clone()
                    }
                }
                DSlot::SS if cov == Coverage::Cover2Man => deep_half(st),
                // Cover 1 robber: the strong safety sits in the middle hole
                // at about ten yards, robbing crossers and digs.
                DSlot::SS if cov == Coverage::Cover1Robber => zone(v2(los + 9.5, by), false, 11.0),
                DSlot::Mike if s.dcall.spy => Job::Spy,
                _ => {
                    // Tight ends first, then the back, else rob the middle.
                    let te = recv
                        .iter()
                        .copied()
                        .find(|r| {
                            !taken.contains(r)
                                && roles[*r].is_some_and(|x| matches!(x, ORole::TE | ORole::TE2 | ORole::TE3))
                        })
                        .or_else(|| recv.iter().copied().find(|r| !taken.contains(r)));
                    let pick = if d == DSlot::SS || d == DSlot::Sam || d == DSlot::FS { te } else { None };
                    match pick.or_else(|| back.filter(|b| !taken.contains(b))) {
                        Some(r) => {
                            taken.push(r);
                            man_job(sim, i, r)
                        }
                        None => middle.clone(),
                    }
                }
            }
        } else if d == DSlot::Mike && s.dcall.spy {
            Job::Spy
        } else {
            let lb_side = side_of(sim);
            let cov = if cov == Coverage::Cover3Match { Coverage::Cover3 } else { cov };
            match (cov, d) {
                (Coverage::Cover3, DSlot::CBL) => deep_third(1.0),
                (Coverage::Cover3, DSlot::CBR) => deep_third(-1.0),
                (Coverage::Cover3, DSlot::FS) => deep_mid.clone(),
                // With a nickel on the slot the strong safety drops to the hook.
                (Coverage::Cover3, DSlot::SS) => {
                    if slots.contains(&Some(DSlot::NCB)) {
                        hook(st)
                    } else {
                        curl_flat(st)
                    }
                }
                (Coverage::Cover3, DSlot::NCB) => curl_flat(st),
                (Coverage::Cover3, DSlot::DCB) => curl_flat(-st),
                (Coverage::Cover3, DSlot::Will) => {
                    if slots.contains(&Some(DSlot::DCB)) {
                        hook(-st)
                    } else {
                        curl_flat(-st)
                    }
                }
                (Coverage::Cover3, DSlot::Sam) => hook(st),
                (Coverage::Cover3, DSlot::Mike) => middle.clone(),
                (Coverage::Cover2 | Coverage::Tampa2, DSlot::CBL) => flat(1.0),
                (Coverage::Cover2 | Coverage::Tampa2, DSlot::CBR) => flat(-1.0),
                (Coverage::Cover2 | Coverage::Tampa2, DSlot::FS) => deep_half(-st),
                (Coverage::Cover2 | Coverage::Tampa2, DSlot::SS) => deep_half(st),
                (Coverage::Tampa2, DSlot::Mike) => zone(v2(los + 14.0, by), true, 12.0),
                (Coverage::Cover2, DSlot::Mike) => middle.clone(),
                (Coverage::Cover4, DSlot::CBL) => quarter_out(1.0),
                (Coverage::Cover4, DSlot::CBR) => quarter_out(-1.0),
                (Coverage::Cover4, DSlot::FS) => quarter_in(-st),
                (Coverage::Cover4, DSlot::SS) => quarter_in(st),
                (Coverage::Cover4, DSlot::Mike) => middle.clone(),
                (Coverage::Cover6, DSlot::CBL) => {
                    if st > 0.0 {
                        quarter_out(1.0)
                    } else {
                        flat(1.0)
                    }
                }
                (Coverage::Cover6, DSlot::CBR) => {
                    if st > 0.0 {
                        flat(-1.0)
                    } else {
                        quarter_out(-1.0)
                    }
                }
                (Coverage::Cover6, DSlot::SS) => quarter_in(st),
                (Coverage::Cover6, DSlot::FS) => deep_half(-st),
                (Coverage::Cover6, DSlot::Mike) => middle.clone(),
                (_, DSlot::NCB) => curl_flat(st),
                (_, DSlot::DCB) => curl_flat(-st),
                (_, DSlot::Sam) => hook(st),
                (_, DSlot::Will) => {
                    if matches!(cov, Coverage::Cover4 | Coverage::Cover6) && !slots.contains(&Some(DSlot::NCB)) {
                        curl_flat(-st)
                    } else {
                        hook(lb_side)
                    }
                }
                (_, DSlot::Mike) => middle.clone(),
                _ => middle.clone(),
            }
        };
        // A blown coverage: he plays the wrong zone or loses his man.
        if sim.agents[i].bust {
            if let Job::Man { .. } = sim.agents[i].job {
                let side = side_of(sim);
                sim.agents[i].job = hook(side);
            } else if let Job::Zone { spot, deep, width, .. } = sim.agents[i].job.clone() {
                sim.agents[i].job = Job::Zone {
                    spot: v2(spot.x - 5.0, spot.y),
                    deep: !deep && false,
                    width: width * 0.6,
                    matched: false,
                };
            }
        }
    }
    // Cover 3 match: underneath zones carry a vertical in their area like
    // man coverage (pattern matching).
    if cov == Coverage::Cover3Match {
        for i in off_n..n {
            if let Job::Zone { matched, .. } = &mut sim.agents[i].job {
                *matched = true;
            }
        }
    }
    // The bracket (topic 5.4): the deep help shades to the receiver the
    // staff fears; in man, the free safety doubles him over the top.
    if let Some(b) = s.tac.bracket.and_then(|p| (0..off_n).find(|&k| sim.agents[k].p == p)) {
        let ry = sim.agents[b].pos.y;
        let helper = (off_n..n)
            .filter(|&i| matches!(sim.agents[i].job, Job::Zone { deep: true, .. }))
            .min_by(|&a, &c| (sim.agents[a].pos.y - ry).abs().total_cmp(&(sim.agents[c].pos.y - ry).abs()));
        if let Some(h) = helper {
            if cov.is_man() {
                let cushion = (sim.agents[h].pos.x - sim.agents[b].pos.x).max(5.0);
                sim.agents[h].job = Job::Man { target: b, cushion, lag: 0.25 };
            } else if let Job::Zone { spot, .. } = &mut sim.agents[h].job {
                spot.y += 0.35 * (ry - spot.y);
                spot.x += 1.0;
            }
        }
    }
    // Bunch and stack (topic 1): stacked receivers pick man defenders off
    // each other, unless the defenders banjo (switch by an in-out rule),
    // which smart ones do. A man trailing a receiver in motion across starts
    // a step behind.
    if cov.is_man() {
        let bunched = matches!(s.call.formation.set, Set::Bunch | Set::Stack);
        let db_pr = (off_n..n).filter(|&i| sim.agents[i].label == "CB").map(|i| sim.agents[i].a(Attr::PlayRecognition));
        let (sum, cnt) = db_pr.fold((0.0, 0.0), |a, x| (a.0 + x, a.1 + 1.0));
        let banjo = sim.rng.chance(clamp(0.25 + 0.01 * (sum / f64::max(cnt, 1.0) - 60.0), 0.05, 0.75));
        let moving = if matches!(s.call.formation.motion, Motion::Across | Motion::Jet) {
            s.call.formation.motion_man().and_then(|r| roles.iter().position(|x| *x == Some(r)))
        } else {
            None
        };
        for i in off_n..n {
            let Job::Man { target, .. } = sim.agents[i].job else { continue };
            let tp = sim.agents[target].pos;
            let stacked = bunched
                && (0..off_n).any(|k| {
                    k != target && roles[k].is_some_and(|r| r.is_receiver()) && sim.agents[k].pos.dist(tp) < 3.0
                });
            let mut add = 0.0;
            if stacked && !banjo {
                add += 0.15 + 0.1 * sim.rng.next();
                // Off the line, nobody presses a bunch.
                sim.agents[i].pos.x = sim.agents[i].pos.x.max(los + 3.0);
            }
            if Some(target) == moving {
                add += 0.1;
            }
            if let Job::Man { lag, .. } = &mut sim.agents[i].job {
                *lag += add;
            }
        }
    }
    // Stunts (topic 1): two rushers cross; the penetrator goes first into
    // his partner's lane and the looper comes behind him. A line that has
    // played together passes it off (topic 4.9).
    if s.dcall.stunt != Stunt::None {
        let side = if sim.rng.chance(0.5) { 1.0 } else { -1.0 };
        let on_side = |d: DSlot| match d {
            DSlot::LE | DSlot::DTL => side > 0.0,
            DSlot::RE | DSlot::DTR => side < 0.0,
            _ => false,
        };
        let find = |want: &dyn Fn(DSlot) -> bool| (off_n..n).find(|&i| slots[i].is_some_and(|d| want(d) && on_side(d)));
        let end = find(&|d| matches!(d, DSlot::LE | DSlot::RE));
        let tackle = find(&|d| matches!(d, DSlot::DTL | DSlot::DTR));
        let pair = match s.dcall.stunt {
            Stunt::TackleEnd => tackle.zip(end),
            Stunt::EndTackle => end.zip(tackle),
            _ => {
                let a = (off_n..n).find(|&i| slots[i] == Some(DSlot::DTL));
                let b = (off_n..n).find(|&i| slots[i] == Some(DSlot::DTR));
                a.zip(b)
            }
        };
        if let Some((pen, lp)) = pair
            && let (Job::Rush { lane: l1, .. }, Job::Rush { lane: l2, .. }) =
                (sim.agents[pen].job.clone(), sim.agents[lp].job.clone())
        {
            if let Job::Rush { lane, .. } = &mut sim.agents[pen].job {
                *lane = l2;
            }
            if let Job::Rush { lane, .. } = &mut sim.agents[lp].job {
                *lane = l1;
            }
            sim.agents[lp].hold_until = 0.45;
            sim.stunt = Some((pen, lp));
        }
    }
    // Press at the line (topic 3): a jam battle off the snap.
    for i in off_n..n {
        if let Job::Man { target, .. } = sim.agents[i].job {
            let (d, r) = (&sim.agents[i], &sim.agents[target]);
            if d.pos.x - los < 2.0 {
                // Dynamic Links at the line: Length lands the jam (or keeps
                // the receiver's frame clean), Sudden slips it.
                let rel = 0.55 * r.a(Attr::Release)
                    + 0.25 * r.a(Attr::Agility)
                    + 0.2 * r.a(Attr::Strength)
                    + sim.link(target, &[Link::Sudden, Link::Length]);
                let jam = 0.55 * d.a(Attr::Press)
                    + 0.25 * d.a(Attr::Strength)
                    + 0.2 * d.a(Attr::Agility)
                    + sim.link(i, &[Link::Length, Link::Sudden]);
                let x = rel - jam + 9.0 * gauss(sim.rng);
                if x < -9.0 {
                    if let Job::Route { release_delay, .. } = &mut sim.agents[target].job {
                        *release_delay += 0.45;
                    }
                } else if x > 9.0
                    && let Job::Man { lag, .. } = &mut sim.agents[i].job
                {
                    *lag += 0.3;
                }
            }
        }
    }
}

/// Which linebacker the coordinator sends (L1): real Mikes, Sams and Wills
/// rush about as often (pressures 0.74%, 0.77% and 0.72% of snaps), the
/// heavier man more (+0.37 pts of blitzes per 10 lb at the same draft slot)
/// and the long-armed man less (-0.21 pts an inch), with his Blitzing.
fn lb_blitzer(sim: &mut Sim, s: &Snap, off_n: usize, slots: &[Option<DSlot>]) -> DSlot {
    let men: Vec<(DSlot, f64)> = (off_n..sim.agents.len())
        .filter_map(|i| {
            let d = slots[i]?;
            if !d.is_lb() {
                return None;
            }
            let g = &s.teams[1][sim.agents[i].p];
            let (_, tw, ta) = super::synthetic::typical_body(sim.level, g.position);
            let tb = super::synthetic::typical(sim.level, g.position, Attr::Blitzing);
            // Weights 0.1 per 10 lb, -0.04 an inch of arm and 0.2 per 10
            // points of Blitzing; the Sam a little more (proposed, not
            // confirmed).
            let x = 0.1 * (g.weight - tw) / 10.0 - 0.04 * (g.arm - ta)
                + 0.2 * (sim.agents[i].a(Attr::Blitzing) - tb) / 10.0;
            let base = if d == DSlot::Sam { 1.1 } else { 1.0 };
            Some((d, base * x.exp()))
        })
        .collect();
    if men.is_empty() {
        return if slots.contains(&Some(DSlot::Will)) { DSlot::Will } else { DSlot::Mike };
    }
    sim.rng.weighted(&men)
}

/// Which back the coordinator sends on a back blitz (C1): the nickel, else
/// the strong safety, as before; with a nickel on the field, his body
/// against the strong safety's (real corners blitz 20% less an inch of
/// height, and the same defense blitzes 2 pts more per 10 lb of corner
/// weight on the field).
fn db_blitzer(sim: &mut Sim, s: &Snap, off_n: usize, slots: &[Option<DSlot>]) -> DSlot {
    let find = |d: DSlot| (off_n..sim.agents.len()).find(|&i| slots[i] == Some(d));
    let (Some(n), Some(ss)) = (find(DSlot::NCB), find(DSlot::SS)) else {
        return if slots.contains(&Some(DSlot::NCB)) { DSlot::NCB } else { DSlot::SS };
    };
    let body = |i: usize| {
        let g = &s.teams[1][sim.agents[i].p];
        let (th, tw, _) = super::synthetic::typical_body(sim.level, g.position);
        0.22 * (g.weight - tw) / 10.0 - 0.2 * (g.height - th)
    };
    // The nickel goes about three times as often as the strong safety
    // (weights 0.22 per 10 lb and -0.2 an inch; proposed, not confirmed).
    let w_n = body(n).exp();
    let w_s = 0.35 * body(ss).exp();
    sim.rng.weighted(&[(DSlot::NCB, w_n), (DSlot::SS, w_s)])
}

/// The seconds a linebacker's weight adds to his turn in coverage over his
/// level's typical linebacker (L2, bug 307); nothing for anyone else.
fn lb_hips(sim: &Sim, d: usize) -> f64 {
    let g = sim.player(d);
    if g.position != crate::positions::Position::LB {
        return 0.0;
    }
    let typ = super::synthetic::typical_body(sim.level, g.position).1;
    clamp(0.1 * (g.weight - typ) / 10.0, -0.1, 0.15)
}

fn man_job(sim: &Sim, d: usize, r: usize) -> Job {
    let (da, ra) = (&sim.agents[d], &sim.agents[r]);
    // Reaction lag: route running against cover skill (topic 2). A tenth of
    // a second quicker than it was: man coverage was completed 72% of the
    // time in the NFL against about 57% real (Cover 0, 1 and 2-man,
    // engine_targets.json), proposed, not confirmed.
    // (A tenth quicker again with the plays and tactics build: man was
    // still completed 74% of the time against 57% real.)
    // The typical receiver against the typical man defender keeps the old
    // 0.0065 a point; only how far each is from his own position's typical
    // man reads at `MAN_LAG_RR` (minor bug 283).
    // The man defender's Man Coverage reads at `MAN_LAG_MC` about his
    // level's typical starter at his position (minor bug 283: at 0.02 about
    // the dressed man it moved the catch rate he allows 0.03 of real
    // starters' spread), and no floor above none (it was 0.1 s): the best
    // cover men mirror the break.
    let t_rr = super::synthetic::typical(sim.level, sim.player(r).position, Attr::RouteRunning);
    let t_mc = super::synthetic::typical(sim.level, sim.player(d).position, Attr::ManCoverage);
    let s_mc = super::synthetic::typical_starter(sim.level, sim.player(d).position, Attr::ManCoverage);
    let gap_typ = t_rr - t_mc;
    let lag = clamp(
        0.12 + 0.0065 * gap_typ + MAN_LAG_RR * (ra.a(Attr::RouteRunning) - t_rr)
            - MAN_LAG_RR * (s_mc - t_mc)
            - MAN_LAG_MC * (da.a(Attr::ManCoverage) - s_mc)
            + 0.003 * (ra.a(Attr::Agility) - (0.6 * da.a(Attr::Agility) + 0.4 * da.a(Attr::Flexibility))),
        0.0,
        0.75,
    );
    // A heavy linebacker turns his hips late (L2, bug 307): real ones
    // allow 0.9 pts more catches per 10 lb at the same draft slot.
    let lag = lag + lb_hips(sim, d);
    let cushion = (da.pos.x - ra.pos.x).max(0.5);
    Job::Man { target: r, cushion, lag }
}

/// Pass protection: each lineman takes the rusher across from him; extra
/// blockers help, backs pick up blitzers.
fn pass_protection(sim: &mut Sim, off_n: usize) {
    let n = sim.agents.len();
    let mut rushers: Vec<usize> = (off_n..n).filter(|&i| matches!(sim.agents[i].job, Job::Rush { .. })).collect();
    rushers.sort_by(|&a, &b| sim.agents[a].pos.y.total_cmp(&sim.agents[b].pos.y));
    let mut blockers: Vec<usize> = (0..off_n)
        .filter(|&i| matches!(sim.agents[i].job, Job::PassBlock { .. }) && is_line(sim.agents[i].label))
        .collect();
    blockers.sort_by(|&a, &b| sim.agents[a].pos.y.total_cmp(&sim.agents[b].pos.y));
    let mut taken = vec![false; n];
    for &b in &blockers {
        if sim.agents[b].bust {
            continue;
        }
        let by = sim.agents[b].pos.y;
        let best = rushers
            .iter()
            .copied()
            .filter(|&r| !taken[r])
            .min_by(|&x, &y| (sim.agents[x].pos.y - by).abs().total_cmp(&(sim.agents[y].pos.y - by).abs()));
        if let Some(r) = best
            && (sim.agents[r].pos.y - by).abs() < 3.2
        {
            taken[r] = true;
            sim.agents[b].job = Job::PassBlock { target: Some(r) };
        }
    }
    // Backs and tight ends in protection take what's left, inside first.
    let others: Vec<usize> = (0..off_n)
        .filter(|&i| matches!(sim.agents[i].job, Job::PassBlock { target: None }) && !is_line(sim.agents[i].label))
        .collect();
    for b in others {
        let by = sim.agents[b].pos.y;
        if let Some(r) = rushers
            .iter()
            .copied()
            .filter(|&r| !taken[r])
            .min_by(|&x, &y| (sim.agents[x].pos.y - by).abs().total_cmp(&(sim.agents[y].pos.y - by).abs()))
            && !sim.agents[b].bust
        {
            taken[r] = true;
            sim.agents[b].job = Job::PassBlock { target: Some(r) };
        }
    }
}

/// Help on their best rusher (A8): when the plan names him and he rushes,
/// the protection slides his way, so a lineman with no man of his own (or
/// a back or tight end kept in) doubles him, and a back releasing on his
/// side chips him on the way out. Real coaches do this every week against
/// a dominant rusher; it frees others to win their one-on-ones.
fn slide_to(sim: &mut Sim, s: &Snap, off_n: usize) {
    let Some(star) = s.tac.help_on.and_then(|p| (off_n..sim.agents.len()).find(|&k| sim.agents[k].p == p)) else {
        return;
    };
    if !matches!(sim.agents[star].job, Job::Rush { .. }) {
        return;
    }
    sim.help_on = Some(star);
    let sy = sim.agents[star].pos.y;
    // A spare blocker on his side moves toward him at the snap.
    let spare = (0..off_n)
        .filter(|&i| matches!(sim.agents[i].job, Job::PassBlock { target: None }) && !sim.agents[i].bust)
        .min_by(|&a, &b| (sim.agents[a].pos.y - sy).abs().total_cmp(&(sim.agents[b].pos.y - sy).abs()));
    if spare.is_none() {
        // No spare man: a back or tight end releasing on his side chips him
        // first (a beat later into his route).
        if let Some(b) = (0..off_n)
            .filter(|&i| {
                matches!(sim.agents[i].label, "RB" | "FB" | "TE")
                    && matches!(sim.agents[i].job, Job::Route { .. })
                    && (sim.agents[i].pos.y - sy).abs() < 5.0
            })
            .min_by(|&a, &b| (sim.agents[a].pos.y - sy).abs().total_cmp(&(sim.agents[b].pos.y - sy).abs()))
            && let Job::Route { release_delay, .. } = &mut sim.agents[b].job
        {
            *release_delay += CHIP_TIME;
            sim.chip = Some((b, star));
        }
    }
}

/// The rush and the protection for the line chart (lane 9), when the snap
/// keeps one: the gap each rusher aimed at from the line's spots, whether a
/// blocker took him, and the protection the offense used.
fn chart_rush(sim: &mut Sim, off_n: usize, slots: &[Option<DSlot>], ball_y: f64) {
    if sim.notes.chart.is_none() {
        return;
    }
    use super::chart::{Protection, Rusher, gap_at};
    let ol: Vec<f64> = (0..off_n).filter(|&i| is_line(sim.agents[i].label)).map(|i| sim.agents[i].pos.y).collect();
    let taken = |sim: &Sim, d: usize| {
        (0..off_n).any(|b| matches!(sim.agents[b].job, Job::PassBlock { target: Some(t) } if t == d))
    };
    let mut rush = Vec::new();
    for i in off_n..sim.agents.len() {
        let Job::Rush { lane, .. } = sim.agents[i].job else { continue };
        let mut flags = 0;
        if slots.get(i).copied().flatten().is_some_and(|d| d.is_line()) {
            flags |= Rusher::LINEMAN;
        }
        if taken(sim, i) {
            flags |= Rusher::PICKED;
        }
        if sim.stunt.is_some_and(|(a, b)| a == i || b == i) {
            flags |= Rusher::STUNT;
        }
        rush.push(Rusher { id: sim.agents[i].id, gap: gap_at(lane, &ol), flags });
    }
    let blockers = (0..off_n).filter(|&i| matches!(sim.agents[i].job, Job::PassBlock { .. })).count();
    let protection = if blockers >= 7 {
        Protection::Max
    } else if let Some(star) = sim.help_on {
        if sim.agents[star].pos.y > ball_y { Protection::SlideLeft } else { Protection::SlideRight }
    } else {
        Protection::Man
    };
    if let Some(c) = sim.notes.chart.as_mut() {
        c.rush = rush;
        c.protection = protection;
        c.blockers = blockers as u8;
    }
}

/// How long a chip holds a back or tight end before he releases, and how
/// much ground it costs the rusher in his block (block-win units).
const CHIP_TIME: f64 = 0.6;
const CHIP_W: f64 = 0.25;

/// A stunt against the protection (topic 4.9): the blocker whose man
/// leaves takes the looper, and his neighbour takes the penetrator. A line
/// that has played together and sees it does; otherwise they chase their
/// first men and the looper comes clean (proposed, not confirmed).
fn stunt_pickup(sim: &mut Sim, chem: f64) {
    let Some((pen, lp)) = sim.stunt else { return };
    let off_n = sim.agents.iter().filter(|a| a.side == 0).count();
    let on = |sim: &Sim, d: usize| {
        (0..off_n).find(|&b| matches!(sim.agents[b].job, Job::PassBlock { target: Some(t) } if t == d))
    };
    let (Some(bp), Some(bl)) = (on(sim, pen), on(sim, lp)) else { return };
    let aw = 0.5 * (sim.agents[bp].a(Attr::FootballIq) + sim.agents[bl].a(Attr::FootballIq));
    let ok = sim.rng.chance(clamp(0.35 + 0.45 * chem + 0.004 * (aw - 60.0), 0.2, 0.92));
    if ok {
        sim.agents[bp].job = Job::PassBlock { target: Some(lp) };
        sim.agents[bl].job = Job::PassBlock { target: Some(pen) };
    }
    sim.notes.stunt = Some(ok);
}

fn is_line(label: &str) -> bool {
    matches!(label, "LT" | "LG" | "C" | "RG" | "RT")
}

/// Run blocking by scheme (topic 3): zone linemen take the man in their
/// play-side gap or climb to a linebacker; gap schemes down-block and pull.
fn run_blocking(sim: &mut Sim, off_n: usize, roles: &[Option<ORole>], rc: RunConcept, dir: f64, pulls: u8) {
    let n = sim.agents.len();
    let mut line: Vec<usize> = (0..off_n)
        .filter(|&i| roles[i].is_some_and(|r| r.is_line() || matches!(r, ORole::TE | ORole::TE2 | ORole::TE3)))
        .collect();
    // Play side first.
    line.sort_by(|&a, &b| (sim.agents[b].pos.y * dir).total_cmp(&(sim.agents[a].pos.y * dir)));
    let front: Vec<usize> = (off_n..n).filter(|&i| sim.agents[i].pos.x < sim.los + 2.5).collect();
    let second: Vec<usize> = (off_n..n)
        .filter(|&i| {
            sim.agents[i].pos.x >= sim.los + 2.5
                && sim.agents[i].pos.x < sim.los + 8.0
                && (sim.agents[i].pos.y - sim.agents[0].pos.y).abs() < 9.0
        })
        .collect();
    let mut taken = vec![false; n];
    // Pullers: backside guard (and tackle on counter, both guards on buck sweep).
    let mut pullers: Vec<usize> = Vec::new();
    if pulls > 0 {
        let backside: Vec<usize> = line
            .iter()
            .rev()
            .copied()
            .filter(|&i| matches!(roles[i], Some(ORole::LG) | Some(ORole::RG) | Some(ORole::LT) | Some(ORole::RT)))
            .collect();
        let g = backside.iter().copied().find(|&i| matches!(roles[i], Some(ORole::LG) | Some(ORole::RG)));
        if let Some(g) = g {
            pullers.push(g);
        }
        if pulls > 1 {
            let other = if rc == RunConcept::BuckSweep {
                line.iter().copied().find(|&i| matches!(roles[i], Some(ORole::LG) | Some(ORole::RG)) && Some(i) != g)
            } else {
                backside.iter().copied().find(|&i| matches!(roles[i], Some(ORole::LT) | Some(ORole::RT)))
            };
            if let Some(o) = other {
                pullers.push(o);
            }
        }
    }
    // A trap leaves the first lineman past the center unblocked on purpose:
    // the play-side guard climbs to a linebacker and the backside guard
    // kicks the trapped man out.
    let trap_guard = if rc == RunConcept::Trap {
        line.iter().copied().find(|&i| matches!(roles[i], Some(ORole::LG) | Some(ORole::RG)) && !pullers.contains(&i))
    } else {
        None
    };
    for &b in &line {
        if pullers.contains(&b) || sim.agents[b].bust {
            continue;
        }
        let y = sim.agents[b].pos.y;
        if Some(b) == trap_guard {
            let t = second
                .iter()
                .copied()
                .filter(|&d| !taken[d])
                .min_by(|&a, &c| (sim.agents[a].pos.y - y).abs().total_cmp(&(sim.agents[c].pos.y - y).abs()));
            if let Some(d) = t {
                taken[d] = true;
            }
            sim.agents[b].job = Job::RunBlock { target: t, pull: false };
            continue;
        }
        // The end man of the line also takes a man lined up outside him.
        let end = b == line[0];
        let gap = |d: usize, sim: &Sim| {
            let dy = (sim.agents[d].pos.y - y) * dir;
            if pulls > 0 {
                // Down block: the man inside him.
                (-1.8..0.8).contains(&dy)
            } else if end {
                (-0.6..4.0).contains(&dy)
            } else {
                (-0.6..1.9).contains(&dy)
            }
        };
        let t = front.iter().copied().find(|&d| !taken[d] && gap(d, sim));
        let t = t.or_else(|| {
            second.iter().copied().filter(|&d| !taken[d]).min_by(|&a, &c| {
                let f = |d: usize| (sim.agents[d].pos.y - (y + dir * 2.0)).abs();
                f(a).total_cmp(&f(c))
            })
        });
        if let Some(d) = t {
            taken[d] = true;
            sim.agents[b].job = Job::RunBlock { target: Some(d), pull: false };
        }
        if super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed) {
            println!(
                "ASSIGN {} ({:.1},{:.1}) -> {:?}",
                sim.agents[b].label,
                sim.agents[b].pos.x,
                sim.agents[b].pos.y,
                t.map(|d| (sim.agents[d].label, sim.agents[d].pos.x, sim.agents[d].pos.y))
            );
        }
    }
    // Combination blocks (fix list S6.1): an uncovered lineman sent to a
    // linebacker first helps the covered man beside him (play side first)
    // on his down lineman, and climbs to his linebacker once the down man
    // is controlled or the linebacker comes downhill. Zone and duo run
    // two, the gap runs one at the point of attack (proposed, not
    // confirmed).
    let combos = match rc {
        RunConcept::InsideZone | RunConcept::ZoneRead | RunConcept::Duo | RunConcept::Draw => 2,
        RunConcept::OutsideZone | RunConcept::Power | RunConcept::Counter | RunConcept::Iso => 1,
        _ => 0,
    };
    let mut made = 0;
    let mut doubled: Vec<usize> = Vec::new();
    for k in 0..line.len() {
        if made >= combos {
            break;
        }
        let b = line[k];
        let Job::RunBlock { target: Some(lb), pull: false } = sim.agents[b].job else { continue };
        if sim.agents[lb].pos.x < sim.los + 2.5 {
            continue;
        }
        for nb in [k.checked_sub(1), Some(k + 1)].into_iter().flatten() {
            let Some(&a) = line.get(nb) else { continue };
            if let Job::RunBlock { target: Some(d), pull: false } = sim.agents[a].job
                && sim.agents[d].pos.x < sim.los + 2.5
                && matches!(sim.agents[d].label, "DT" | "NT" | "DE")
                && !doubled.contains(&d)
            {
                sim.agents[b].job = Job::RunBlock { target: Some(d), pull: false };
                sim.agents[b].climb = Some(lb);
                doubled.push(d);
                made += 1;
                break;
            }
        }
    }
    // Pullers kick out or lead on whoever is left at the point of attack.
    for &b in &pullers {
        let poa = sim.agents[0].pos.y + dir * if rc == RunConcept::Trap { 1.0 } else { 4.0 };
        let by = sim.agents[0].pos.y;
        let t = (off_n..n)
            .filter(|&d| !taken[d] && sim.agents[d].pos.x < sim.los + 7.0 && (sim.agents[d].pos.y - by).abs() < 7.0)
            .min_by(|&a, &c| (sim.agents[a].pos.y - poa).abs().total_cmp(&(sim.agents[c].pos.y - poa).abs()));
        if let Some(d) = t {
            taken[d] = true;
        }
        sim.agents[b].job = Job::RunBlock { target: t, pull: true };
    }
    // Fullback leads to the first linebacker; receivers stalk the man on them.
    for i in 0..off_n {
        match roles[i] {
            Some(ORole::FB) => {
                let t = second.iter().copied().filter(|&d| !taken[d]).min_by(|&a, &c| {
                    (sim.agents[a].pos.y - sim.agents[i].pos.y - dir * 3.0)
                        .abs()
                        .total_cmp(&(sim.agents[c].pos.y - sim.agents[i].pos.y - dir * 3.0).abs())
                });
                if let Some(d) = t {
                    taken[d] = true;
                }
                sim.agents[i].job = Job::RunBlock { target: t, pull: true };
            }
            Some(ORole::X | ORole::Z | ORole::Slot | ORole::Slot2 | ORole::Slot3) => {
                let me = sim.agents[i].pos;
                // A receiver stalks a man off the line, never a down
                // lineman: he took the nearest man, a defensive end too, and
                // lost (minor bug 164).
                let t = (off_n..n)
                    .filter(|&d| !taken[d] && sim.agents[d].pos.dist(me) < 9.0)
                    .filter(|&d| !matches!(sim.agents[d].label, "DE" | "DT") && sim.agents[d].pos.x >= sim.los + 1.5)
                    .min_by(|&a, &c| sim.agents[a].pos.dist(me).total_cmp(&sim.agents[c].pos.dist(me)));
                if let Some(d) = t {
                    taken[d] = true;
                }
                sim.agents[i].job = Job::RunBlock { target: t, pull: false };
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------------- per tick

fn run_route(sim: &mut Sim, i: usize, pass: &Option<Pass>, qb: usize, s: &Snap) {
    let t = sim.t;
    let Job::Route { ref pts, next, settle, release_delay, .. } = sim.agents[i].job else { return };
    let n_pts = pts.len();
    if t < release_delay {
        // The chip (A8): he hits the rusher the plan slides to on his way
        // out, knocking him off his path, then releases.
        if let Some((b, d)) = sim.chip
            && b == i
        {
            let dp = sim.agents[d].pos;
            if sim.agents[i].pos.dist(dp) < 1.3 {
                if let Some(k) = sim.agents[d].engaged {
                    sim.eng[k].w -= CHIP_W * clamp(0.6 + sim.agents[i].a(Attr::PassBlocking) / 150.0, 0.6, 1.3);
                }
                sim.agents[d].vel = sim.agents[d].vel.scale(0.5);
                sim.chip = None;
                if let Job::Route { release_delay, .. } = &mut sim.agents[i].job {
                    *release_delay = t + 0.15;
                }
            } else {
                sim.go_to(i, dp, 1.0);
            }
            return;
        }
        // A receiver's release: footwork off the line, not a sprint (a jam
        // keeps him at it longer). On a silent count he first waits for the
        // ball to move.
        if matches!(sim.agents[i].label, "WR" | "TE") && t >= silent_late(sim, s, i) {
            let p = sim.agents[i].pos;
            sim.go_to(i, p + v2(2.0, 0.0), RELEASE_PACE);
            return;
        }
        // A back checks for blitzers before he releases.
        if let Some(r) = blitz_threat(sim, i, qb) {
            sim.agents[i].job = Job::PassBlock { target: Some(r) };
            return;
        }
        let p = sim.agents[i].pos;
        sim.go_to(i, p, 0.3);
        return;
    }
    // The scramble drill (topic 3.1): when the play breaks down (the
    // quarterback has left the pocket, or has held the ball far past his
    // clock), receivers work back to open grass, if the coach lets them and
    // they see it. Not on a timer in a clean pocket: a vertical stays a
    // vertical until the quarterback moves.
    let broken = pass.as_ref().is_some_and(|p| (p.scramble || t > 4.6) && p.thrown.is_none());
    if broken && freelance_ok(sim, i, s.strictness[0], s.trust[0][sim.agents[i].p]) {
        let q = sim.agents[qb].pos;
        let me = sim.agents[i].pos;
        let mut best = (f64::NEG_INFINITY, me);
        // Open grass stays inside the end line: near the goal line a
        // receiver who drifted past it ran on out of the stadium.
        let deep = (s.los + 25.0).min(end_line() - 1.5);
        // The same for every spot: worked out once.
        let shallow = (s.los + 2.0).min(deep);
        let wide = width() - 2.0;
        for k in 0..8 {
            let cand = v2(
                clamp(me.x + 3.0 * ((k % 3) as f64 - 1.0), shallow, deep),
                clamp(me.y + 4.0 * ((k / 3) as f64 - 1.0) + (q.y - me.y) * 0.2, 2.0, wide),
            );
            // The nearest defender: the root of the least square, the same
            // number as the least root (the root is monotone, and `min`
            // drops the same NaNs), one root a spot instead of one a
            // defender (world speed lane).
            let space = (sim.agents.len() / 2..sim.agents.len())
                .filter(|&d| sim.agents[d].side == 1)
                .map(|d| sim.agents[d].pos.dist_sq(cand))
                .fold(f64::INFINITY, f64::min)
                .sqrt();
            if space > best.0 {
                best = (space, cand);
            }
        }
        // He becomes a route to that grass, so the quarterback can read it.
        if let Job::Route { pts, next, settle, .. } = &mut sim.agents[i].job {
            *pts = vec![best.1];
            *next = 0;
            *settle = true;
        }
        let d = me.dist(best.1);
        sim.steer(i, (best.1 - me).norm().scale((d * 2.0).min(sim.agents[i].vmax * 0.9)));
        return;
    }
    if next < n_pts {
        // His level's typical route runner, looked up only at a break he
        // slows for (a fixed table: the same number whenever it is read).
        let rr_typ = || super::synthetic::typical(sim.level, sim.player(i).position, Attr::RouteRunning);
        let a = &sim.agents[i];
        let Job::Route { pts, .. } = &a.job else { return };
        let (target, speed, done, dist) = route_step(a, pts, next, settle, rr_typ);
        if done && let Job::Route { next: n, .. } = &mut sim.agents[i].job {
            *n += 1;
        }
        let me = sim.agents[i].pos;
        // A bust runs a few yards off, but inside the lines like any route.
        let dir = if sim.agents[i].bust {
            let target = v2((target.x + 3.0).min(end_line() - 1.0), (target.y + 4.0).min(width() - 2.5));
            (target - me).norm()
        } else {
            // His distance to the point from `route_step` (he hasn't moved
            // since): the same heading.
            toward(me, target, dist)
        };
        sim.steer(i, dir.scale(speed));
    } else if settle {
        // Sit down in space: drift away from the nearest defender.
        let me = sim.agents[i].pos;
        // The same pick as `min_by` on the distance (`Nearest`).
        let mut near = Nearest::default();
        for (d, a) in sim.agents.iter().enumerate() {
            if a.side == 1 {
                near.offer(d, a.pos.dist_sq(me));
            }
        }
        let near = near.get();
        // Route savvy: a smart route runner finds the hole and sits in it.
        let savvy = 0.6 * sim.agents[i].a(Attr::RouteRunning) + 0.4 * sim.agents[i].a(Attr::FootballIq);
        let room = 1.5 * clamp(1.0 + 0.015 * (savvy - 60.0), 0.6, 1.4);
        let away = near.map(|d| (me - sim.agents[d].pos).norm().scale(room)).unwrap_or_default();
        let q = sim.agents[qb].pos;
        let spot = me + v2(0.0, away.y) + (q - me).norm().scale(0.3);
        // He sits down with the room a route keeps at the sideline: the spot
        // is worked out from him each tick, and with his man inside him he
        // used to drift on through the sideline on a long play.
        let spot = v2(spot.x.min(end_line() - 1.5), clamp(spot.y, 2.5, width() - 2.5));
        sim.go_to(i, spot, 0.3);
    } else {
        let v = sim.agents[i].vel;
        let dir = if v.len() > 0.5 { v.norm() } else { v2(1.0, 0.0) };
        let me = sim.agents[i].pos;
        // Running out of room: he turns up the sideline.
        let dir = if me.y < 4.0 && dir.y < 0.0 || me.y > width() - 4.0 && dir.y > 0.0 { v2(1.0, 0.0) } else { dir };
        // At the back of the end zone he can't go on (a man who steps out
        // can't catch it): he slides along the end line toward the middle,
        // under control.
        if me.x > end_line() - 2.5 && dir.x > 0.0 {
            let across = if me.y < mid() { 1.0 } else { -1.0 };
            let spot = v2(end_line() - 1.5, clamp(me.y + across * 3.0, 2.0, width() - 2.0));
            sim.go_to(i, spot, 0.5);
            return;
        }
        let vm = route_pace(&sim.agents[i]);
        sim.steer(i, dir.scale(vm));
    }
}

/// The nearest spot to `p` with his feet inside the lines: a man who plays
/// a ball that sails out of bounds runs to the sideline or end line, not
/// into the bench: a receiver ran 6.4 yards past the sideline after an
/// overthrow. 3.6% of NFL throws in `match_check` land past a line, and
/// about 1,400 of them were caught in 4,000 games before and after (toe-taps
/// within his reach of the line): completions (66.0%) and yards an attempt
/// (7.04) held.
fn feet_in(p: V2) -> V2 {
    v2(clamp(p.x, back_line() + 0.3, end_line() - 0.3), clamp(p.y, 0.3, width() - 0.3))
}

/// The nearest of the men offered in turn, each with his squared distance
/// (`V2::dist_sq`): the same pick as `min_by` on `V2::dist` (the first of
/// the smallest roots, by `total_cmp`), and each root worked out once at
/// most (world speed lane). A man no nearer on the square than the best so
/// far can't have a smaller root (the root is monotone, and a sum of
/// squares is never -0), so his isn't taken; the others take the root
/// `dist` gives and are kept on the same test as `min_by`'s.
#[derive(Default)]
struct Nearest {
    /// The best so far: who, his squared distance and its root.
    top: Option<(usize, f64, f64)>,
}

impl Nearest {
    #[inline]
    fn offer(&mut self, i: usize, q: f64) {
        match self.top {
            Some((_, bq, _)) if q >= bq => {}
            Some((_, _, bk)) => {
                let k = q.sqrt();
                if bk.total_cmp(&k).is_gt() {
                    self.top = Some((i, q, k));
                }
            }
            None => self.top = Some((i, q, q.sqrt())),
        }
    }

    /// Who.
    #[inline]
    fn get(&self) -> Option<usize> {
        self.top.map(|b| b.0)
    }

    /// Who, and his distance (the root `dist` gives).
    #[inline]
    fn best(&self) -> Option<(usize, f64)> {
        self.top.map(|b| (b.0, b.2))
    }
}

/// A wide receiver's or tight end's release off the line (minor bug 135):
/// the stance, the release move and the first steps of the stem take about
/// half a second, less for a good Release, at a quarter of his speed. With
/// it, flatter slants and deep balls judged a little worse, NFL throws of
/// 20+ air yards fell from 15.5% to 13.6% of targets (real 11.5%), those of
/// 0-9 rose from 36.2% to 46.6% (real 50.1%) and 10-19 fell from 32.0% to
/// 23.7% (real 20.7%, nflverse 2021-2025; 6,000 `match_check` games):
/// receivers had left the line at a sprint and ran every route deeper at
/// the real time to throw (proposed, not confirmed).
const RELEASE: f64 = 0.5;
const RELEASE_PACE: f64 = 0.25;

/// How late a road receiver leaves on the silent count, less so a focused
/// one.
fn silent_late(sim: &Sim, s: &Snap, i: usize) -> f64 {
    let conc = s.teams[0][sim.agents[i].p].trait20(Attr::Concentration, s.rules.level);
    SILENT_RELEASE * s.cond.crowd * clamp(1.0 - 0.05 * (conc - 10.0), 0.5, 1.5)
}

/// How long his release takes.
fn footwork(a: &Agent) -> f64 {
    RELEASE * clamp(1.3 - 0.006 * a.a(Attr::Release), 0.7, 1.3)
}

/// How fast a receiver runs his route before the ball is in the air: under
/// control, short of his drill top speed, so he can break and find the
/// ball (real receivers run routes well under their combine speed). Once the
/// ball is thrown he goes full speed to it (proposed, not confirmed).
fn route_pace(a: &Agent) -> f64 {
    0.88 * a.vmax
}

/// `(to - from).norm()` when `len` is `from.dist(to)`: the same heading,
/// bit for bit (world speed lane). A length that is not NaN is the same
/// number both ways (`from - to` is `to - from` negated exactly), so it
/// isn't worked out again; a NaN one may carry another payload, so then it
/// is, as `norm` did.
#[inline]
fn toward(from: V2, to: V2, len: f64) -> V2 {
    if len.is_nan() {
        return (to - from).norm();
    }
    let d = to - from;
    if len < 1e-9 { V2::default() } else { v2(d.x / len, d.y / len) }
}

/// One tick of a route: the point he runs at, how fast, whether he has
/// reached it (he moves on to the next point), and how far he is from the
/// point (`a.pos.dist(target)`, for his heading with `toward`). His level's
/// typical route runner (`rr_typ`) is looked up only at a break he has to
/// slow for, the one place it is read (a fixed table: the same number).
fn route_step(a: &Agent, pts: &[V2], next: usize, settle: bool, rr_typ: impl FnOnce() -> f64) -> (V2, f64, bool, f64) {
    let raw = pts[next];
    // Good route runners cut sharper: they start the break later and keep speed.
    let rr = a.a(Attr::RouteRunning);
    let vmax = route_pace(a);
    let switch = 0.6 + 0.012 * rr;
    let d_raw = a.pos.dist(raw);
    let done = d_raw < switch;
    // He keeps a couple of yards of room at the sideline.
    let target = v2(raw.x, clamp(raw.y, 2.5, width() - 2.5));
    let urg = if next + 1 == pts.len() && settle { 0.85 } else { 1.0 };
    // Off the sideline's room it is the same point (the same bits), so the
    // same distance: it isn't worked out again.
    let dist = if target.y.to_bits() == raw.y.to_bits() { d_raw } else { a.pos.dist(target) };
    // Before the break of a route that stops (hitch, curl, comeback) he
    // sinks his hips: he brakes so he can plant at the stem.
    let brake = a.brake();
    let speed = if next + 1 == pts.len() && settle {
        vmax.min(dist * 2.0)
    } else if settle && next + 2 == pts.len() {
        let stop = (2.0 * brake * 0.9 * dist).sqrt() + 1.0 + 0.03 * (rr - 50.0).max(0.0);
        (vmax * urg).min(stop)
    } else if next + 1 < pts.len() {
        // A sharp break needs him to gear down first; good route runners
        // carry more speed through it.
        let room = 2.0 * brake * 0.9 * (dist - 0.5).max(0.0);
        let full = vmax * urg;
        // Far enough from the break to brake down for any corner: the limit
        // is at least room's root, so it can't bind and the turn isn't
        // worked out (the same speed, bit for bit). The root is taken only
        // near the edge (`sq_ge`, the same answer).
        if sq_ge(room, full) {
            full
        } else {
            let rr_typ = rr_typ();
            let into = toward(a.pos, target, dist);
            let out = (pts[next + 1] - target).norm();
            let turn = into.dot(out).clamp(-1.0, 1.0).acos();
            let corner = vmax
                * (1.0 - (0.62 - 0.004 * (rr - 50.0) - BREAK_RR * (rr - rr_typ)) * (turn / 1.57).min(1.4)).max(0.15);
            let limit = (corner * corner + room).sqrt();
            full.min(limit)
        }
    } else {
        vmax * urg
    };
    (target, speed, done, dist)
}

fn freelance_ok(sim: &mut Sim, i: usize, strictness: f64, trust: f64) -> bool {
    let gp = sim.player(i);
    let iq = 0.5 * gp.a(Attr::FootballIq) + 0.5 * gp.a(Attr::PlayRecognition);
    let p = clamp(0.25 + 0.008 * (iq - 50.0) + 0.5 * (trust - 0.5) - 0.025 * (strictness - 10.0), 0.05, 0.95);
    // Decide once per play: store in the bust flag's sibling via the random draw.
    let key = (sim.agents[i].id as u64 * 2654435761 + (sim.los * 10.0) as u64) % 1000;
    (key as f64 / 1000.0) < p
}

fn blitz_threat(sim: &Sim, i: usize, qb: usize) -> Option<usize> {
    let q = sim.agents[qb].pos;
    let me = sim.agents[i].pos;
    // The same tests on the squares (`sq_lt`) and the same pick as `min_by`
    // on the distance to the quarterback (`Nearest`; world speed lane).
    let mut near = Nearest::default();
    for (d, a) in sim.agents.iter().enumerate() {
        if a.side == 1 && matches!(a.job, Job::Rush { .. }) && a.engaged.is_none() && a.up(sim.t) {
            let to_q = a.pos.dist_sq(q);
            if sq_lt(to_q, 6.0) && sq_lt(a.pos.dist_sq(me), 4.5) && picks_up(sim, i, d) {
                near.offer(d, to_q);
            }
        }
    }
    near.get()
}

/// Does the back see and take this blitzer? His football IQ and blocking
/// against the blitzer's timing and disguise (blitzing). Decided once per
/// pair and snap, like `freelance_ok` (proposed, not confirmed: 85% at 60).
fn picks_up(sim: &Sim, i: usize, d: usize) -> bool {
    let (b, z) = (&sim.agents[i], &sim.agents[d]);
    let p = clamp(
        0.85 + 0.01 * (b.a(Attr::FootballIq) - 60.0) + 0.004 * (b.a(Attr::PassBlocking) - 60.0)
            - 0.008 * (z.a(Attr::Blitzing) - 60.0),
        0.4,
        0.98,
    );
    let key = (b.id as u64 * 2654435761 + z.id as u64 * 40503 + (sim.los * 10.0) as u64) % 1000;
    (key as f64 / 1000.0) < p
}

pub(crate) fn escort(sim: &mut Sim, i: usize) {
    // Before the screen is caught, block for the back, not the quarterback.
    let (c, pre) = match (sim.screen, sim.ball) {
        (Some(r), Ball::Held(h))
            if h != r && sim.agents[h].side == sim.agents[i].side && sim.agents[h].label == "QB" =>
        {
            (r, true)
        }
        (Some(r), Ball::Air { .. }) => (r, true),
        _ => match sim.carrier() {
            Some(c) => (c, false),
            None => return,
        },
    };
    let los = sim.los;
    let me = sim.agents[i].pos;
    let cp = sim.agents[c].pos;
    let side = sim.agents[i].side;
    // The man to block: a free pursuer near the ball, or one coming at it
    // from in front of the runner (a return wall picks them up early).
    // The same tests on the squared distances (`sq_lt`) and the same pick as
    // `min_by` on his distance (`Nearest`): roots only where they decide
    // (world speed lane). Every test is pure, so the order is free.
    let t = sim.t;
    let mut near = Nearest::default();
    for (d, a) in sim.agents.iter().enumerate() {
        if a.side == side || !a.up(t) || a.engaged.is_some() {
            continue;
        }
        let dp = a.pos;
        let q_me = dp.dist_sq(me);
        // Before the screen is caught the rushers are let through; the
        // blockers look for the second level.
        let second = !pre || (a.smell || !matches!(a.job, Job::Rush { .. })) && sq_lt(q_me, 4.0);
        let q_c = dp.dist_sq(cp);
        if second && (sq_lt(q_c, 10.0) || (dp.x - cp.x) * sim.goal > 0.0 && sq_lt(q_c, 30.0) && sq_lt(q_me, 12.0)) {
            near.offer(d, q_me);
        }
    }
    let threat = near.get();
    match threat {
        Some(d) if sim.closest(i, d) < 1.3 && sim.lands_block(i, d) => {
            // Blocks from behind are illegal.
            let from_behind =
                (sim.agents[d].pos - me).dot(sim.agents[d].vel.norm()) > 0.5 && sim.agents[d].vel.len() > 3.0;
            // Called on 7.5% of them (was 3%: illegal blocks ran 0.1 a team
            // game, real about 0.28; proposed, not confirmed).
            let restraint = clamp(
                1.0 - 0.05 * (sim.trait20(i, Attr::Discipline) - 10.0)
                    - 0.006 * (sim.agents[i].a(Attr::BlockingInSpace) - 60.0),
                0.4,
                1.6,
            );
            if from_behind && sim.rng.chance(0.075 * sim.crew.holding * restraint) {
                let (id, side, at) = (sim.agents[i].id, side, me.x);
                sim.notes.fouls.push(Foul { kind: FoulKind::IllegalBlock, side, player: id, at });
            }
            sim.engage(i, d, false);
        }
        Some(d) => {
            // Get in his path to the ball, at speed.
            let dp = sim.agents[d].pos;
            let spot = dp + (cp - dp).norm().scale(0.8) + sim.agents[d].vel.scale(0.2);
            let vm = sim.agents[i].vmax;
            let want = if spot.dist(me) > 2.0 {
                (spot - me).norm().scale(vm)
            } else {
                sim.agents[d].vel + (spot - me).scale(3.0)
            };
            sim.steer(i, want);
        }
        None => {
            let mut ahead = cp + v2(4.0 * sim.goal, (me.y - cp.y).clamp(-3.0, 3.0));
            if pre {
                // Set up the convoy just past the line, outside the back.
                ahead = v2(los + 1.5, cp.y + (me.y - cp.y).clamp(-4.0, 4.0) * 0.5);
            }
            sim.go_to(i, ahead, 0.8);
        }
    }
}

/// Two-minute drill: get what you can, then get out of bounds.
fn hurry_run(sim: &mut Sim, c: usize) {
    let me = sim.agents[c].pos;
    let side = if me.y > mid() { 1.0 } else { -1.0 };
    // The root of the least square: the least distance (see the scramble
    // drill), one root (world speed lane).
    let near = (0..sim.agents.len())
        .filter(|&d| sim.agents[d].side != sim.agents[c].side && sim.agents[d].up(sim.t))
        .map(|d| sim.agents[d].pos.dist_sq(me))
        .fold(f64::INFINITY, f64::min)
        .sqrt();
    if near < 4.0 {
        let dir = v2(0.45 * sim.goal, side).norm();
        let vm = sim.agents[c].vmax;
        sim.steer(c, dir.scale(vm));
        if (me.x - sim.progress) * sim.goal > 0.0 {
            sim.progress = me.x;
        }
    } else {
        sim.run_carrier(c, None);
    }
}

/// A scrambling quarterback slides past the sticks or before a big hit.
fn slide(sim: &mut Sim, c: usize, s: &Snap) -> bool {
    let me = sim.agents[c].pos;
    if me.x < s.los + 1.0 {
        return false;
    }
    // The root of the least square: the least distance (see the scramble
    // drill), one root (world speed lane).
    let near = (0..sim.agents.len())
        .filter(|&d| sim.agents[d].side == 1 && sim.agents[d].up(sim.t) && sim.agents[d].engaged.is_none())
        .map(|d| sim.agents[d].pos.dist_sq(me))
        .fold(f64::INFINITY, f64::min)
        .sqrt();
    let past = me.x >= s.los + s.togo;
    // Contact willingness (tendencies; toughness decided this before): one
    // who avoids contact slides early and short of the sticks, one who
    // seeks it (above 12, as toughness above 12 did) lowers his shoulder
    // unless he is past them.
    let contact = sim.tend(c, Tend::Contact);
    let early = 2.2 + 1.0 * ((9.0 - contact) / 8.0).max(0.0);
    if near < early && (past || contact < 12.0) {
        sim.progress = sim.progress.max(me.x - 0.5);
        sim.dead = true;
        sim.ball = Ball::Dead(me);
        sim.notes.tend.slid = true;
        return true;
    }
    false
}

/// The zone read: keep it if the backside end crashes on the back.
fn read_option(sim: &mut Sim, qb: usize, dir: f64, off_n: usize) -> bool {
    let q = sim.agents[qb].pos;
    let end = (off_n..sim.agents.len())
        .filter(|&d| {
            sim.agents[d].engaged.is_none()
                && (sim.agents[d].pos.y - q.y) * dir < -1.5
                && sim.agents[d].pos.x < sim.los + 3.0
        })
        .min_by(|&a, &b| sim.agents[a].pos.dist(q).total_cmp(&sim.agents[b].pos.dist(q)));
    let Some(e) = end else { return false };
    let crash = sim.agents[e].vel.y * dir > 1.0 || (sim.agents[e].pos.y - q.y).abs() < 2.5;
    let iq = sim.agents[qb].a(Attr::Decisions);
    let right = sim.rng.chance(clamp(0.55 + 0.005 * iq, 0.5, 0.97));
    if right { crash } else { !crash }
}

/// A run that starts one way and goes the other.
fn misdirection(s: &Snap) -> bool {
    matches!(
        s.call.kind,
        PlayKind::Run(RunConcept::Counter | RunConcept::Trap | RunConcept::BuckSweep | RunConcept::JetSweep)
    )
}

/// The RPO read (topic 1): the defender in conflict over the throw's
/// receiver. He comes up for the run, the quarterback pulls it and throws;
/// he stays, he hands off. A sharp quarterback reads it right more often.
fn rpo_pull(sim: &mut Sim, qb: usize, tgt: usize, off_n: usize) -> bool {
    let rp = sim.agents[tgt].pos;
    let by = sim.agents[qb].pos.y;
    let key = (off_n..sim.agents.len())
        .filter(|&d| {
            let a = &sim.agents[d];
            !matches!(a.job, Job::Rush { .. })
                && a.pos.x < sim.los + 9.0
                && (a.pos.y - by) * (rp.y - by) > 0.0
                && (a.pos.y - by).abs() <= (rp.y - by).abs() + 1.0
        })
        .min_by(|&a, &b| sim.agents[a].pos.dist(rp).total_cmp(&sim.agents[b].pos.dist(rp)));
    let Some(k) = key else { return true };
    let a = &sim.agents[k];
    let plays_run = sim.t >= a.read_at && a.vel.x < -0.8 || a.pos.x < sim.los + 2.5;
    let iq = 0.6 * sim.agents[qb].a(Attr::Decisions) + 0.4 * sim.agents[qb].a(Attr::PlayRecognition);
    let right = sim.rng.chance(clamp(0.6 + 0.0045 * iq, 0.6, 0.97));
    if right { plays_run } else { !plays_run }
}

/// The option pitch (topic 1): the quarterback pitches when the man on the
/// edge takes him, keeps when that man widens to the pitch. Some(true)
/// pitches, Some(false) keeps for good (he is through the line or has
/// committed), None reads on.
fn option_pitch(sim: &mut Sim, qb: usize, pm: usize, dir: f64, off_n: usize) -> Option<bool> {
    let q = sim.agents[qb].pos;
    if q.x > sim.los + 1.0 {
        return Some(false);
    }
    let p = sim.agents[pm].pos;
    // The same pick as `min_by` on the distance (`Nearest`), which keeps his
    // distance, `k.dist(q)` bit for bit (world speed lane).
    let mut near = Nearest::default();
    for d in off_n..sim.agents.len() {
        let a = &sim.agents[d];
        if a.engaged.is_none() && a.up(sim.t) && (a.pos.y - q.y) * dir > -1.0 && a.pos.x > q.x - 1.0 {
            near.offer(d, a.pos.dist_sq(q));
        }
    }
    let (key, kq) = near.best()?;
    let k = sim.agents[key].pos;
    if kq > 3.2 {
        return None;
    }
    let takes_qb = kq < k.dist(p);
    let iq = 0.6 * sim.agents[qb].a(Attr::Decisions) + 0.4 * sim.agents[qb].a(Attr::PlayRecognition);
    let right = sim.rng.chance(clamp(0.6 + 0.004 * iq, 0.6, 0.96));
    Some(if right { takes_qb } else { !takes_qb })
}

// ------------------------------------------------------------- quarterback

/// The quarterback: drop, read the progression, feel pressure, then throw,
/// scramble, throw it away or hold it (topics 1 and 2). Returns the throw.
/// The window (yards) a quarterback wants on a read before he lets it go.
/// It was 1.6: quarterbacks passed up open men, ran out of reads and threw
/// 29% of NFL targets to the flat (real 9%). At 0.3 the flat is 18%, the
/// rest is the play library's (proposed, not confirmed).
const READ_NEED: f64 = 0.3;

/// When the quarterback starts reading, from the top of his drop: after the
/// hitch step, a tenth of a second on (it was a quarter second before the
/// top, and real time to throw ran 0.35 s longer than the engine's, with
/// sacks a third short; proposed, not confirmed).
const READ_START: f64 = 0.1;

// Minor bugs batch 9 (the A/B tests, 2026-11-07; sizes proposed, not
// confirmed, each fitted so `ab_check` lands in the real band or spread).
// Each new read is taken about the level's typical player at the position
// (`synthetic::typical`), so the levels' averages stay where they were and
// only how much players differ within a level changes.

/// Route running against man coverage: the man defender's reaction lag, a
/// second a rating point of each man's distance from his position's typical
/// man (minor bug 283; it was 0.0065 on the raw gap, and Route Running
/// moved separation 0.02 of real starters' spread).
const MAN_LAG_RR: f64 = 0.02;

/// Route running through the break: the speed a sharp cut keeps, a rating
/// point about his level's typical route runner (minor bug 283).
const BREAK_RR: f64 = 0.004;

/// A fast quarterback holds the ball (minor bug 276): his clock gains this
/// a 10 Speed points over his level's typical passer (61 in the NFL, about
/// a 4.84 forty), and he waits this much longer before a rusher
/// he sees makes him hurry. Real fast quarterbacks extend plays and take
/// more sacks; the engine's took fewer.
const LEGS_HOLD: f64 = 0.2;
const LEGS_WAIT: f64 = 0.07;

/// The share of that extra time his choice to run waits too (minor bug
/// 276). All of it and a fast man's scrambles came late and lost their
/// extra yards (real fast scramblers gain more a run); none of it and his
/// scrambles rose 1.8 times as fast as real fast passers' and his sacks
/// fell to 0.4 times real's slope. At half, scrambles, yards a scramble
/// and sacks all land in the real bands (`ab_check qb_speed`; proposed,
/// not confirmed).
const LEGS_RUN: f64 = 0.5;

/// Decisions: the room a careful passer wants before he throws, a rating
/// point about his level's typical passer (minor bug 283; it was 0.008 from
/// 60, and Decisions moved picks 0.06 of real starters' spread). The level's
/// typical passer keeps the old room.
const DECISIONS_CAUTION: f64 = 0.02;

// Minor bug 283, the rest (2026-11-08; every size proposed, not confirmed,
// fitted with `ab_check` at the NFL, +15 against -15, so each rating moves
// its own stat by 0.1 to 1.0 of real starters' spread, and with
// `match_check` so the levels' totals hold). Each new slope sits about his
// level's typical starter (`synthetic::typical_starter`): starters rate
// above the dressed average, and a slope about the dressed man lifted or
// sank every starting lineup (picks ran 2.6% against 2.3%).

/// Decisions: the room he wants before he lets it go, less a rating point
/// for a better decider about his level's typical starter (the typical
/// starter keeps `DECISIONS_CAUTION`'s room). He sees the window as it is,
/// so he can trust a tight one; a poor decider sees more room than there
/// is and wants what he would want from a true read. At +0.02 a point
/// (more room for the better man) better decisions cut picks but threw
/// shorter and held the ball into sacks: EPA a dropback fell 0.008 at +15
/// against -15, and sacks rose.
const DECISIONS_TRUST: f64 = 0.06;

/// The share of Decisions in his read of the field (the noise on each
/// window, what he misses in his lane, where he puts the ball near a
/// defender); Play Recognition is the rest. It was half each: choosing the
/// throw is what Decisions rates.
const READ_DECIDE: f64 = 0.7;

/// Picking up the blitz: seconds sooner he sees the free man coming and
/// gets rid of it, a 10 points of Football IQ (and `BLITZ_ID_LOOK` of it
/// Play Recognition) about his level's typical starter. Football IQ moved
/// sacks 0.01 of real starters' spread; now 0.2.
const BLITZ_ID: f64 = 0.1;
const BLITZ_ID_LOOK: f64 = 0.4;

/// Giving up on a covered checkdown: log-odds a 10 points of Decisions and
/// of Football IQ about his level's typical starter.
const GIVE_UP_DECIDE: f64 = 0.3;
const GIVE_UP_IQ: f64 = 0.25;

/// Throwing it away once nothing is there, a point of Decisions on the
/// passer's scale about his level's typical starter (0.003 from 0 before).
const THROW_AWAY_DECIDE: f64 = 0.004;

/// A misread, in tens of points of his field read under his level's typical
/// starter's: a defender reading his eyes jumps the throw this many seconds
/// sooner than he allows for, and a man sitting in his lane takes this many
/// yards more of the window than he sees (both the other way for a better
/// reader, who sees it tighter than the typical man would).
const MISREAD_EYES: f64 = 0.3;
const MISREAD_LANE: f64 = 1.0;

/// Ball placement near a defender: yards farther from him, a 10 points of
/// field read about his level's typical starter, fading out when the
/// nearest defender is `PLACE_NEAR` yards from the spot.
const PLACE_READ: f64 = 1.0;
const PLACE_NEAR: f64 = 4.0;

/// Accuracy within a level: the log of the miss a rating point about his
/// level's typical starter, by depth, sized against real starters' spread
/// by depth (nflverse 2023-24, passer seasons: completion under 10 air
/// yards p5 68.7% to p95 77.8%, 10-19 45.5% to 65.0%, 20+ 26.9% to 49.3%;
/// the part that isn't sampling luck about 1.4, 3.7 and 4.3 points). Short
/// Accuracy moved completion 0.05 of real starters' spread; now 0.3.
const AIM_SHORT: f64 = 0.02;
/// The miss's base scale (1.3 before). A steeper slope spreads passers
/// wider, and a wider miss costs more catches than a tighter one gains: on
/// synthetic rosters NFL completion fell from 65.6% to 64.3% and FBS from
/// 60.4% to 59.2%. At 1.25 every level is back within half a point of
/// where it was.
const AIM_BASE: f64 = 1.25;
const AIM_MEDIUM: f64 = 0.025;
const AIM_DEEP: f64 = 0.014;

/// Man coverage within a level: the man defender's reaction lag, seconds a
/// point of Man Coverage about his level's typical starter at his position.
const MAN_LAG_MC: f64 = 0.03;

/// Zone coverage within a level: how far behind a break he plays, seconds a
/// point of his zone read (0.6 Zone Coverage, 0.4 Play Recognition) about
/// his level's typical starter at his position (0.004 before).
const ZONE_LAG: f64 = 0.012;

/// Coverage skill when the ball is thrown: seconds sooner a better cover
/// man breaks on it, a 10 points of his job's coverage (Man or Zone) about
/// his level's typical starter at his position.
const BALL_COVER: f64 = 0.15;

/// A man defender with his back turned feels the throw this many seconds
/// before the receiver's hands go up, from his man's eyes and hands. Man
/// coverage was completed 67.5% of the time in the NFL against about 57%
/// real (Cover 0, 1 and 2-man).
const MAN_FEEL: f64 = 0.25;

/// A breakup, a point of his job's coverage about his level's typical
/// starter at his position (0.004 from 60 before).
const BREAKUP_COVER: f64 = 0.012;

/// Interceptions where they come from. Real picks come on throws a
/// defender could play: FTN charting (NFL 2023-24) has 87% of picks on
/// throws it calls interception-worthy, 53% of those picked, and 0.33% of
/// the other throws picked. The engine's spread them over every throw (a tipped or
/// broken-up ball picked as often as one thrown to a defender in front of
/// it), so a careful passer saved few picks. Now a defender in front of the
/// ball picks it more often (`FRONT_PICK`, 0.22 before), and a tipped or
/// broken-up ball less (`TIP_PICK` 0.13 before, `PBU_PICK` 0.06), with the
/// league's picks held.
const FRONT_PICK: f64 = 0.27;
const TIP_PICK: f64 = 0.065;
const PBU_PICK: f64 = 0.06;

/// His read of the field (minor bug 283): Decisions and Play Recognition.
fn field_read(qa: &Agent) -> f64 {
    READ_DECIDE * qa.a(Attr::Decisions) + (1.0 - READ_DECIDE) * qa.a(Attr::PlayRecognition)
}

/// The same for his level's typical starting passer.
fn typical_field_read(level: crate::player::Level) -> f64 {
    let ts = |a: Attr| super::synthetic::typical_starter(level, Position::QB, a);
    READ_DECIDE * ts(Attr::Decisions) + (1.0 - READ_DECIDE) * ts(Attr::PlayRecognition)
}

/// A defender's reach to the ball at the catch point, yards an inch of
/// height and of arm about his level's typical man at his position (minor
/// bug 274; they were 0.012 and 0.02, and a corner's arms barely moved the
/// catch rate he allowed). Real height moves it little once the depth of
/// target is held (-0.13 pts an inch; tall corners cover the deeper routes),
/// arms -0.67 an inch (the cornerback research, 2026-11-07).
const D_REACH_HEIGHT: f64 = 0.025;
const D_REACH_ARM: f64 = 0.10;

/// Throws 15+ yards deep: the passer trusts a man who can run under it
/// (his deep speed over his level's typical receiver's, minor bug 280) and
/// his own arm (Arm Strength over his level's typical passer's, minor bug
/// 283), in windows of the read's scale a 10 points.
const DEEP_LEGS: f64 = 0.4;
const DEEP_ARM: f64 = 0.4;

/// A tall passer sees the middle over his line (minor bug 277): in the
/// pocket he trusts a tighter window inside, 0.7 of a window unit every 2
/// inches over his level's typical passer (6'3 in the NFL).
const OVER_LINE: f64 = 0.7;

/// A big receiver gets the harder balls (minor bug 275): downfield and in
/// the red zone the passer trusts his size in a tighter window, a window
/// unit every 4 inches of height (arm counting twice) over his level's
/// typical man at his position.
const BIG_TRUST: f64 = 1.0;

/// The passer takes his best target, so a trust that differs from man to
/// man makes him throw more often even where it averages zero: more deep
/// balls and picks, fewer throwaways and scrambles. These come off the big
/// receiver's and the deep man's trust so the level's throws stay where
/// they were (`match_check`, NFL and FBS: 20+ yard share, throwaways,
/// scrambles; proposed, not confirmed). High school takes none off the big
/// man: there 0.35 cut 0.3 points a team game, sent 0.9 points more of the
/// throws behind the line and added 0.13 sacks, where none keeps its throws
/// where they were. FCS and small colleges take college's.
const BIG_BASE: f64 = 0.35;
const DEEP_BASE: f64 = 0.1;
fn big_base(level: crate::player::Level) -> f64 {
    if level == crate::player::Level::HS { 0.0 } else { BIG_BASE }
}

/// Pocket Presence in the sack escape, logit a 10 points over his level's
/// typical passer (minor bug 283; it moved sacks 0.09 of real starters'
/// spread).
const POCKET_ESCAPE: f64 = 0.5;

/// When a quarterback is pressured (minor bug 284): the nearest rusher
/// within this many yards (an engaged one counts 1.5 yards farther unless
/// he is winning), from this many seconds after the snap; the men credited
/// are the nearest and any free or winning rusher within `PRESS_CAUSE`.
const PRESS_NEAR: f64 = 3.2;
const PRESS_FROM: f64 = 0.6;
const PRESS_CAUSE: f64 = 2.6;
/// How much a rusher winning his block within 3 yards weighs on the
/// quarterback's feel for the pocket, at full win and on top of him (bug
/// 358). It was 0.6: the bug 283 merge left NFL sacks at 2.57 a team game
/// (real 2.41) as the pocket collapsed and he held on. At 1.0 he slides or
/// gets it out a beat sooner: 2.39 on seeds 86-89. Proposed, not
/// confirmed.
const POCKET_FEEL: f64 = 1.0;

/// A typical quarterback's pre-snap read of the coverage (topic 3.5).
const PRE_READ: f64 = 0.45;

fn quarterback(
    sim: &mut Sim,
    ps: &mut Pass,
    qb: usize,
    s: &Snap,
    _center: usize,
) -> Option<(V2, usize, f64, f64, f64)> {
    let t = sim.t;
    let los = s.los;
    let me = sim.agents[qb].pos;
    // Pressure he can see or feel.
    let mut pressure = 0.0f64;
    let mut nearest = f64::INFINITY;
    let mut nearest_d = None;
    // The rushers close enough to be the pressure: free, or beating their
    // block (minor bug 284).
    let mut causing: Vec<usize> = Vec::new();
    let mut threat = V2::default();
    // How soon the first free rusher he knows about will be on him, seconds.
    let mut soon = f64::INFINITY;
    // He feels the rush he can't see (pocket presence); what he knows
    // (football IQ) sets his hot reads.
    let feel = sim.agents[qb].a(Attr::PocketPresence);
    let aw = sim.agents[qb].a(Attr::FootballIq);
    for d in 0..sim.agents.len() {
        let a = &sim.agents[d];
        if a.side != 1 || !a.up(t) {
            continue;
        }
        let dist = a.pos.dist(me);
        let free = a.engaged.is_none() && matches!(a.job, Job::Rush { .. });
        // A rusher winning his block and driving it into the pocket is
        // pressure too, before he sheds (dynasty item 1: the hand fight and
        // a collapsing pocket now make pressure, as they do for real
        // interior rushers).
        let winning =
            a.engaged.map_or(0.0, |k| if sim.eng[k].pass { clamp((sim.eng[k].w - 0.3) / 0.7, 0.0, 1.0) } else { 0.0 });
        let close = if free { dist } else { dist + 1.5 * (1.0 - winning) };
        if close < nearest {
            nearest = close;
            nearest_d = Some(d);
        }
        if (free || winning > 0.0) && close < PRESS_CAUSE && ps.pressured_at.is_none() && t >= PRESS_FROM {
            causing.push(d);
        }
        if free && dist < 7.0 {
            let seen = sim.sees(qb, a.pos) || sim.rng.chance(0.15 + 0.005 * feel);
            if seen && dist < 4.5 {
                pressure += (4.5 - dist) / 4.5;
                threat = threat + (me - a.pos).norm().scale((4.5 - dist) / 4.5);
            }
            if seen {
                let closing = a.vel.dot((me - a.pos).norm()).max(3.0);
                soon = soon.min((dist - 1.0).max(0.0) / closing);
            }
        } else if winning > 0.0 && dist < 3.0 {
            let feel = POCKET_FEEL * winning * (3.0 - dist) / 3.0;
            pressure += feel;
            threat = threat + (me - a.pos).norm().scale(feel);
        }
    }
    // A pressure (minor bug 284): the nearest rusher within 3.2 yards once
    // the snap and the first punch are past (0.6 s). Before that a tackle
    // lined up over the center sat within 2.6 yards of a quarterback under
    // center or in his first steps, and the old rule counted it: 60% of
    // the engine's pressures came then, and defensive tackles were
    // credited 8.6 times PFR's. The credit goes to the men who made it:
    // the nearest, and any free or winning rusher within 2.6 yards (it was
    // everyone within 2.8 of the quarterback). Team pressure lands at 22.6%
    // of dropbacks (PFR 22.2%); tackles and edges at 1.4 times PFR's a
    // snap, their ratio at real's 0.6 (proposed, not confirmed).
    if nearest < PRESS_NEAR && ps.pressured_at.is_none() && t >= PRESS_FROM {
        ps.pressured_at = Some(t);
        let mut by = causing;
        if let Some(n) = nearest_d
            && !by.contains(&n)
        {
            by.push(n);
        }
        for d in by {
            sim.notes.pressures.push((sim.agents[d].id, t));
        }
    }
    // Footwork: drop, then hang in the pocket or slide away from pressure.
    if t < ps.set_at {
        let fake = ps.concept.play_action && t < 0.45;
        let to = if fake { v2(los - 3.5, me.y) } else { ps.drop_to };
        sim.go_to(qb, to, if fake { 0.6 } else { 0.95 });
    } else if matches!(ps.concept.drop, Drop::Screen) {
        // The back screen: he drifts back and away to draw the rush past
        // the back, as real quarterbacks are coached to.
        sim.go_to(qb, me + v2(-1.0, 0.0), 0.35);
    } else if let Some((side, _)) = ps.extend {
        // Out of the pocket: on the move to his side, a step upfield,
        // eyes downfield, short of the sideline.
        let to = me + v2(0.6, 3.0 * side);
        sim.go_to(qb, v2(to.x, clamp(to.y, 5.0, width() - 5.0)), 0.9);
    } else if pressure > 0.2 {
        let step = threat.norm().scale(1.0 + 0.01 * feel) + v2(0.6, 0.0);
        sim.go_to(qb, me + step.scale(2.0), 0.6 + 0.004 * feel);
    } else {
        let spot = v2(ps.drop_to.x + 0.8, ps.drop_to.y);
        sim.go_to(qb, spot, 0.3);
    }
    if t < ps.set_at + READ_START || ps.reads.is_empty() {
        return None;
    }
    // Hot read: an unblocked blitzer gets the ball thrown early.
    let screen = matches!(ps.concept.drop, Drop::Screen | Drop::Bubble);
    if pressure > 0.6 && t < 2.0 && !ps.hot && !screen {
        ps.hot = true;
        if aw > 45.0 || sim.rng.chance(0.4) {
            ps.read = 0;
        }
    }
    // How fast he works through his reads: what he sees and how fast he decides.
    let iq = 0.5 * sim.agents[qb].a(Attr::PlayRecognition) + 0.5 * sim.agents[qb].a(Attr::Decisions);
    let lv = sim.level;
    // His level's typical starter, the man a new within-level slope sits
    // about (minor bug 283).
    let ts = |a: Attr| super::synthetic::typical_starter(lv, Position::QB, a);
    // What he saw before the snap (topic 3.5): a shell he read cleanly
    // speeds every read; a disguise that fooled him slows them.
    // Measured against a typical read (0.45): the engine's reads were
    // fitted with it.
    let shown = (1.0 - 0.2 * (s.tac.pre_read - PRE_READ)) * if s.tac.fooled { 1.15 } else { 1.0 };
    // A man who holds for the big play stays on each read a little longer.
    // A week of poor prep throws off his timing (TR38): each read runs as
    // much longer as his busts rise, up to a third (update 3, F34).
    let prep = sim.player(qb).prep as f64;
    let per_read =
        clamp(0.62 - 0.0045 * (iq - 50.0), 0.3, 0.95) * shown * (1.0 + 0.6 * sim.lean(qb, Tend::Hold)) * prep;
    // He stays on a read until that route has made its break.
    let developing = ps.read < ps.reads.len()
        && matches!(&sim.agents[ps.reads[ps.read]].job, Job::Route { pts, next, .. } if *next == 0 && pts.len() > 1);
    if t - ps.read_since > per_read && !screen && !(developing && t - ps.read_since < 2.0 * per_read) {
        ps.read += 1;
        ps.read_since = t;
    }
    let decide = sim.agents[qb].a(Attr::Decisions);
    let poise = sim.agents[qb].a(Attr::Composure);
    // His internal clock (topic 1): even in a clean pocket he gets the ball
    // out by a little over three seconds, to the checkdown if nothing else.
    // A quick mind gets rid of it sooner; a slow one hangs on, as lower
    // levels do (real time to throw rises from the NFL to college to high
    // school). A shot play buys a beat more (proposed, not confirmed).
    // (Raised 0.3 s: with the throwing motion counted, real time to throw
    // still ran 0.3 s longer, and a tenth of real throws come after 3.9 s.)
    // Holds for the big play or gets it out (tendencies): his clock runs
    // longer or shorter, and early on he wants more than an open man
    // underneath (or takes him at once).
    let hold = sim.lean(qb, Tend::Hold);
    let typ = |a: Attr| super::synthetic::typical(sim.level, Position::QB, a);
    let legs = (sim.agents[qb].a(Attr::Speed) - typ(Attr::Speed)) / 10.0;
    // Within a level his clock sits at the typical starter's (minor bug
    // 283): a better decider already gets through his reads sooner, and a
    // shorter clock on top sent him to the checkdown early.
    let clock = clamp(3.6 - 0.01 * (ts(Attr::Decisions) - 60.0) - 0.004 * (feel - 60.0), 3.1, 4.1)
        + if ps.concept.drop == Drop::Deep { 0.3 } else { 0.0 }
        + 1.0 * hold
        + LEGS_HOLD * legs;
    // Gunslinger or checkdown (tendencies): how much room he wants before
    // he lets it go, more so downfield; a checkdown man takes the short
    // throw sooner.
    let sling = sim.lean(qb, Tend::Gunslinger);
    let late = t > clock;
    // His legs buy time for the throw more than for the run: when he takes
    // off he waits only part of the extra time (`LEGS_RUN`).
    let late_run = t > clock - (1.0 - LEGS_RUN) * LEGS_HOLD * legs;
    // A free man coming at him (the beaten block, the blitzer nobody
    // picked up): he is done going through his reads and takes the best
    // he has now, the hot or check-down man included, rather than eat it
    // (proposed, not confirmed: about 0.85 of a second away, a little
    // sooner for a poised passer who trusts his pocket; the world's FBS
    // rosters then take a sack on about 6% of dropbacks, real 6.0%).
    // Seeing the blitz coming (minor bug 283): a passer who knows the
    // pressure (Football IQ, and Play Recognition for the look) sees the
    // free man sooner and gets rid of it before he arrives.
    let blitz_id = BLITZ_ID
        * ((1.0 - BLITZ_ID_LOOK) * (aw - ts(Attr::FootballIq))
            + BLITZ_ID_LOOK * (sim.agents[qb].a(Attr::PlayRecognition) - ts(Attr::PlayRecognition)))
        / 10.0;
    let urgent = soon < 0.85 - 0.004 * (poise - 60.0) - LEGS_WAIT * legs + blitz_id && !screen;
    let urgent_run = soon < 0.85 - 0.004 * (poise - 60.0) - LEGS_RUN * LEGS_WAIT * legs + blitz_id && !screen;
    // Off schedule out of the pocket, the progression is gone: he takes
    // whoever comes open.
    let over = ps.read >= ps.reads.len() || late || urgent || ps.extend.is_some();
    let over_run = ps.read >= ps.reads.len() || late_run || urgent_run || ps.extend.is_some();
    // A good quarterback holds the safety with his eyes before coming back.
    // (Eye manipulation: nothing for most, up to a full pull for the best.)
    let manip = clamp((sim.agents[qb].a(Attr::EyeManipulation) - 60.0) / 30.0, 0.0, 1.0);
    ps.look_off = if ps.read == 0 && t - ps.read_since < 0.6 { manip } else { 0.0 };
    let tired = t - ps.set_at;
    let caution = DECISIONS_CAUTION * (ts(Attr::Decisions) - typ(Attr::Decisions))
        + 0.008 * (typ(Attr::Decisions) - 60.0)
        - DECISIONS_TRUST * (sim.agents[qb].a(Attr::Decisions) - ts(Attr::Decisions));
    let panic = pressure * clamp(1.6 - 1.4 * poise / 100.0, 0.15, 1.5);
    // The reads he looks at, borrowed from the pass rather than copied (the
    // same men in the same order; world speed lane).
    let (cand, thresh): (&[usize], f64) = if urgent {
        // Under the gun: anyone with a window, a tighter one than he'd
        // take from a clean pocket.
        (&ps.reads[..], READ_NEED + caution - 0.44 * panic - 0.2)
    } else if over {
        // Late: the checkdown, or anyone open.
        (&ps.reads[..], 1.35 - 0.2 * tired.max(0.0) - 0.44 * panic)
    } else {
        // Quick game is one picture (plays and tactics 5j): the quarterback
        // keys one defender, the flat defender on stick, slant-flat or
        // curl-flat, or picks the side by leverage before the snap, and
        // throws whichever of the first two routes is open. (Read one at a
        // time, he forced the stick into the flat defender: 48% complete
        // against Cover 3 from a 3x1 set, real stick about three in four.)
        let pair = ps.concept.drop == Drop::Quick && ps.read == 0 && ps.reads.len() > 1;
        (
            if pair { &ps.reads[..2] } else { std::slice::from_ref(&ps.reads[ps.read]) },
            READ_NEED + caution - 0.18 * tired.max(0.0) - 0.44 * panic + if ps.read == 0 { 0.1 } else { 0.0 },
        )
    };
    // Pocket frame (Dynamic Links): seeing over the line, a read about as
    // clean as five football IQ points give.
    // How true his read of a window is: Decisions most, Play Recognition
    // the rest (`READ_DECIDE`; it was half each).
    let iq_seen = field_read(&sim.agents[qb]) + sim.link(qb, &[Link::PocketFrame]);
    let noise =
        (1.6 - 0.017 * pro(iq_seen)).max(0.2) * (1.0 + 0.6 * panic) * (1.0 - 0.15 * (s.tac.pre_read - PRE_READ))
            + if s.tac.fooled { 0.2 } else { 0.0 };
    let mut best: Option<(usize, f64, V2, f64)> = None;
    // Each read's window this tick, by his place in `cand`. Nothing on the
    // field moves between here and the checkdown and the throw below (only
    // the random stream and the pass's notes change, and `window` reads
    // neither), so they reuse it instead of playing the route forward again.
    let mut wins: [Option<(f64, V2, f64)>; 8] = [None; 8];
    let looked: &[usize] = if screen { &[] } else { cand };
    for (k, &r) in looked.iter().enumerate() {
        if !sim.agents[r].up(t) || matches!(sim.agents[r].job, Job::PassBlock { .. } | Job::RunBlock { .. }) {
            continue;
        }
        if let Job::Route { release_delay, .. } = sim.agents[r].job
            && t < release_delay
        {
            continue;
        }
        let (w, p, flight) = window(sim, qb, r);
        if let Some(slot) = wins.get_mut(k) {
            *slot = Some((w, p, flight));
        }
        // A covered big receiver can still be open up high (D2): a jump
        // ball or a back-shoulder throw where only he can reach it.
        let size = size_window(sim, qb, r, p, w, s);
        let seen = w + size + noise * gauss(sim.rng);
        // A deep ball needs less room: an incompletion there costs a down,
        // but a catch is worth a chunk, so real quarterbacks give their man
        // a chance at a contested ball far more readily than underneath.
        let depth = (p.x - s.los) * sim.goal;
        let run_under = if depth > 15.0 {
            let wr = |a: Attr| super::synthetic::typical(sim.level, Position::WR, a);
            let deep_typ = 0.4 * wr(Attr::Acceleration) + 0.6 * wr(Attr::Speed);
            DEEP_LEGS * (super::roster::deep_speed(sim.player(r)) - deep_typ) / 10.0
                + DEEP_ARM * (sim.agents[qb].a(Attr::ArmStrength) - typ(Attr::ArmStrength)) / 10.0
                - DEEP_BASE
        } else {
            0.0
        };
        // A tall passer sees the middle over his line (minor bug 277): he
        // trusts a tighter window inside, from the pocket (out of it there
        // is no line in front of him).
        let pocket = ps.extend.is_none() && (me.y - s.ball_y).abs() <= 4.0;
        let inside = pocket && (p.y - mid()).abs() < 9.0 && depth > 4.0;
        let over_line = if inside {
            OVER_LINE * (sim.player(qb).height - super::synthetic::typical_body(sim.level, Position::QB).0) / 2.0
        } else {
            0.0
        };
        // A big receiver gets the harder balls (minor bug 275): the passer
        // trusts his reach in a tighter window downfield and near the goal.
        let red_zone = s.los + depth >= 88.0;
        let big = if depth >= 5.0 || red_zone && depth >= 2.0 {
            let pl = sim.player(r);
            let (ht, _, arm) = super::synthetic::typical_body(sim.level, pl.position);
            BIG_TRUST * ((pl.height - ht) + 2.0 * (pl.arm - arm)) / 4.0 - big_base(sim.level)
        } else {
            0.0
        };
        let need = thresh
            - run_under
            - over_line
            - big
            - (0.05 * (depth - 12.0).max(0.0)).min(1.2)
            - 1.0 * sling
            - if depth > 12.0 { 0.4 * sling } else { 0.0 }
            + if over { 0.0 } else { 0.5 * hold + if depth < 10.0 { 0.9 * hold } else { 0.0 } };
        // A gunslinger looks to the deeper man when two are open.
        let seen_pref = seen + 0.03 * sling * depth;
        if super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!("READ t={:.1} read {} r {} w {:.2} seen {:.2} need {:.2}", t, ps.read, r, w, seen, need);
        }
        if seen > need && best.is_none_or(|b| seen_pref > b.1) {
            best = Some((r, seen_pref, p, flight));
            ps.why = if over { 4 } else { ps.read.min(3) };
            // Thrown up high only because of the size: he puts it where
            // only his man can get it.
            ps.jump_r = (size > 0.0 && seen - size <= need).then_some(r);
        }
    }
    if screen {
        if t < ps.set_at {
        } else if ps.concept.drop == Drop::Bubble {
            // The bubble: throw it now, to the side with the numbers.
            best = None;
            for &r in &ps.reads {
                let (w, p, flight) = window(sim, qb, r);
                let w = w + noise * 0.5 * gauss(sim.rng);
                if best.is_none_or(|b| w > b.1) {
                    best = Some((r, w, p, flight));
                }
            }
        } else {
            // The back screen: wait until he has turned around and the
            // blockers are out, unless the rush is already on him.
            let r = ps.reads[0];
            let set = matches!(&sim.agents[r].job, Job::Route { pts, next, .. } if *next >= pts.len());
            if set || t > ps.set_at + 0.6 || pressure > 0.8 {
                let (_, p, flight) = window(sim, qb, r);
                best = Some((r, 1.0, p, flight));
            } else {
                best = None;
            }
        }
    }
    // The checkdown (topic 1): past his clock with nobody open, he takes
    // the short throw in front of him, the back or a short route, whoever
    // has the most room; a beat later he forces it to whoever is least
    // covered, as real quarterbacks do far more often than they throw it
    // away. The catch point then decides: a contested catch, a breakup or
    // a pick.
    let ck_at = clock + 0.7 * sling.min(0.0);
    if best.is_none() && (t > ck_at || urgent) && !screen {
        let force = t > clock + 0.5 - 0.4 * sling.max(0.0) || urgent && soon < 0.5;
        let mut ck: Option<(usize, f64, V2, f64)> = None;
        for (k, &r) in cand.iter().enumerate() {
            if !sim.agents[r].up(t) || !matches!(sim.agents[r].job, Job::Route { .. }) {
                continue;
            }
            if let Job::Route { release_delay, .. } = sim.agents[r].job
                && t < release_delay
            {
                continue;
            }
            // The checkdown has to be open (real checkdowns are about 9% of
            // targets); forcing it takes at least a step on his man. With
            // neither, he holds it: a scramble, a throwaway or a sack, as
            // real dropbacks end about 15% of the time (proposed, not
            // confirmed: it took anyone within 7 yards and 2.5 yards beaten).
            // The reads above worked his window out this tick (the same
            // value: nothing has moved since).
            let (w, p, flight) = wins.get(k).copied().flatten().unwrap_or_else(|| window(sim, qb, r));
            if if force { w < -1.2 } else { (p.x - s.los) * sim.goal > 7.0 || w < 0.3 } {
                continue;
            }
            if ck.is_none_or(|c| w > c.1) {
                ck = Some((r, w, p, flight));
                ps.why = if force { 6 } else { 5 };
            }
        }
        // Force it or throw it away (minor bug 138): with nobody open and
        // his best man still covered, judgment decides. Real NFL passers
        // throw away 3.5% of dropbacks (FTN charting 2022-24) and force a
        // ball a defender should pick on 4.2% of throws; the engine's
        // forced it to anyone a step behind his man and threw away 0.4%.
        // At sense 75 (decisions and football IQ on the passer's scale)
        // he gives up 80% of the time, 15 points less 57%, 15 more 95%
        // (proposed, not confirmed). Within a level (minor bug 283) the
        // chance moves on the log-odds about his level's typical starter,
        // `GIVE_UP_DECIDE` and `GIVE_UP_IQ` a 10 points: on the old straight
        // line a level's typical man set every passer's slope, and
        // Decisions moved neither picks nor sacks enough.
        if force && !ps.give_up && ck.is_none_or(|c| c.1 < 0.0) {
            let sense = 0.5 * pro(ts(Attr::Decisions)) + 0.5 * pro(ts(Attr::FootballIq));
            let p0 = clamp(GIVE_UP + 0.015 * (sense - 75.0), 0.05, 0.95);
            let lg = (p0 / (1.0 - p0)).ln()
                + GIVE_UP_DECIDE * (decide - ts(Attr::Decisions)) / 10.0
                + GIVE_UP_IQ * (aw - ts(Attr::FootballIq)) / 10.0;
            if sim.rng.chance(1.0 / (1.0 + (-lg).exp())) {
                ps.give_up = true;
            }
        }
        best = if ps.give_up { None } else { ck };
    }
    // Getting it off (topic 1): the throwing motion takes a beat, quicker for
    // a clean, compact release. A free rusher who will be on him before the
    // ball is out doesn't let him throw: he has to escape, eat it or get it
    // away some other way.
    let quick = 0.75 * sim.agents[qb].a(Attr::QuickRelease) + 0.25 * sim.agents[qb].a(Attr::Footwork);
    let release = clamp(0.34 - 0.00125 * (quick - 60.0), 0.26, 0.42);
    // A screen is a soft flick lofted over the rush: only a rusher who can
    // get a hand on his arm stops it.
    let (release, reach) = if screen { (0.6 * release, 1.3) } else { (release, 2.8) };
    let blocked_throw = (0..sim.agents.len()).any(|d| {
        let a = &sim.agents[d];
        if a.side != 1 || !a.up(t) || a.engaged.is_some() || !matches!(a.job, Job::Rush { .. }) {
            return false;
        }
        let rel = me - a.pos;
        let dist = rel.len();
        let closing = a.vel.dot(rel.norm()).max(0.0);
        dist < 3.5 && dist - reach < closing * release
    });
    // A man coming at him makes him skip the wait and short-arm it; once
    // the arm is moving the ball comes out (or the hit lands first).
    let hurry = pressure > 0.8 || nearest < 2.0 || urgent;
    let in_motion = ps.commit.is_some_and(|(_, at, _)| hurry || t + release + 1e-9 >= at);
    if blocked_throw && !in_motion {
        best = None;
        ps.commit = None;
    }
    // Seeing it and letting it go: a quick mind throws on the break
    // (anticipation); a slow one waits to see the man open, and the ball
    // comes out late. Then the throwing motion: the ball leaves at the end
    // of it (it used to leave the moment he decided, so real time to throw,
    // snap to release, ran about half a second short). Pressure on top of
    // him makes him skip the wait and short-arm it.
    if !screen
        && let Some((r, ..)) = best
        && ps.commit.is_none()
    {
        // Throwing on the break is anticipation (half its first size, see
        // footwork above).
        // Chemistry (X5): he lets it go on timing to a receiver he knows,
        // and waits a beat on one he doesn't.
        let timing = 0.12 * (super::roster::TYPICAL_CHEM - sim.player(r).chem) as f64;
        let delay = clamp(0.25 - 0.0025 * (sim.agents[qb].a(Attr::Anticipation) - 50.0) + timing, 0.0, 0.6);
        ps.commit = Some((r, t + delay + release, t + 0.6 * release));
    }
    if !screen && let Some((r, at, hurried)) = ps.commit {
        let now = t + 1e-9 >= if hurry { hurried } else { at };
        if !now || blocked_throw && !in_motion {
            return None;
        }
        if sim.agents[r].up(t) {
            // His window from the reads this tick when he is one of them
            // (the same value), else worked out now.
            let kept = cand.iter().position(|&x| x == r).and_then(|k| wins.get(k).copied().flatten());
            let (_, p, flight) = kept.unwrap_or_else(|| window(sim, qb, r));
            return Some(throw(sim, ps, qb, r, p, flight, pressure, s));
        }
        ps.commit = None;
    }
    if let Some((r, _, p, flight)) = best {
        return Some(throw(sim, ps, qb, r, p, flight, pressure, s));
    }
    // Nothing open: scramble, throw it away, or hold it.
    // Read on the passer's scale (`pro`, A7): what he decides to do when
    // nothing is open faces no rating on the other side, so a high school
    // quarterback's choice to run or throw it away isn't held to the pros'
    // numbers (he ran at a quarter of the NFL's rate and took the sack).
    let speed = pro(sim.agents[qb].a(Attr::Speed));
    // A way out: to one side or the other no free defender is within three
    // yards, and nobody has hold of him. Rushers locked on blockers are
    // what he slides around (it used to need an empty box straight ahead,
    // which a pocket never has: scrambles ran at a tenth of the real 4.9%).
    let free_near = |side: f64| {
        (0..sim.agents.len()).any(|d| {
            let a = &sim.agents[d];
            // Rushes upfield or keeps contain (tendencies): an edge on the
            // wide arc often runs past the quarterback's depth and can't
            // stop him stepping up and out; a contain man shuts his side
            // from a yard further off (proposed, not confirmed).
            let up = if matches!(a.job, Job::Rush { contain, .. } if contain != 0.0) {
                sim.lean(d, Tend::Upfield)
            } else {
                0.0
            };
            let overran = up > 0.0 && play_draw(sim, d, 3) < 0.6 * up;
            let range = 3.0 + (-up).max(0.0) * 0.8;
            a.side == 1
                && a.up(t)
                && a.engaged.is_none()
                && !overran
                && a.pos.dist(me) < range
                && (a.pos.y - me.y) * side > -0.5
        })
    };
    let (free_l, free_r) = (free_near(1.0), free_near(-1.0));
    let lane = nearest > 1.2 && (!free_l || !free_r);
    // Scrambles (tendencies): how readily he leaves the pocket; his speed
    // decides what it gains. Out of the pocket a while with nothing come
    // open, or given up on the pass with a lane in front of him, he tucks it
    // and runs three times as readily.
    let leaves = (1.0 + 1.2 * sim.lean(qb, Tend::Scramble)).max(0.1);
    let extended = ps.give_up || ps.extend.is_some_and(|(_, at)| t - at > EXTEND_RUN);
    let want_run = (over_run || pressure > 0.5)
        && lane
        && t > 2.0
        && sim.rng.chance(clamp(0.03 + 0.004 * (speed - 50.0), 0.01, 0.2) * leaves * if extended { 3.0 } else { 1.0 });
    if want_run || (pressure > 1.0 && lane && speed > 75.0 && sim.rng.chance((0.25 * leaves).min(0.9))) {
        qb_why(8, s.dcall.coverage);
        ps.scramble = true;
        return None;
    }
    // The scramble drill (minor bug 138): with the same lane, a passer who
    // doesn't run breaks out of the pocket with his eyes still downfield,
    // to throw on the move, throw it away legally or run later. Real NFL
    // passers end 17% of dropbacks out of the pocket and throw away 12% of
    // those (FTN charting 2022-24); here a pocket passer leaves at 0.15 a
    // tenth of a second, a runner (who mostly takes off instead) at as
    // little as 0.045 (proposed, not confirmed).
    let lean = sim.lean(qb, Tend::Scramble);
    if ps.extend.is_none()
        && (over || pressure > 0.5)
        && lane
        && t > 1.6
        && sim.rng.chance(EXTEND * clamp(1.0 - 0.5 * lean, 0.3, 1.7))
    {
        let side = if !free_l && (free_r || sim.rng.chance(0.5)) { 1.0 } else { -1.0 };
        ps.extend = Some((side, t));
        return None;
    }
    // Throwing it away takes a rusher on top of him (or a play long dead)
    // and the sense to give up on it; many hold on and take the sack. It is
    // a flick, not a throw: half the motion gets it away (minor bug 138:
    // at the full motion a rusher in his face stopped nearly every one, and
    // the chance a tenth of a second was 0.3 of today's).
    let outside = (me.y - s.ball_y).abs() > 4.0;
    let flicked = (0..sim.agents.len()).any(|d| {
        let a = &sim.agents[d];
        if a.side != 1 || !a.up(t) || a.engaged.is_some() || !matches!(a.job, Job::Rush { .. }) {
            return false;
        }
        let rel = me - a.pos;
        let dist = rel.len();
        let closing = a.vel.dot(rel.norm()).max(0.0);
        dist < 3.5 && dist - reach < closing * release * 0.5
    });
    let imminent = nearest < 2.2 && pressure > 0.4 || urgent;
    let away = ps.give_up && (outside || !flicked)
        || (over || pressure > 0.6)
            && (outside || !flicked)
            && (imminent || t > clock + 0.5)
            && sim.rng.chance(
                0.06 + 0.003 * pro(ts(Attr::Decisions))
                    + THROW_AWAY_DECIDE * (pro(sim.agents[qb].a(Attr::Decisions)) - pro(ts(Attr::Decisions)))
                    + if outside { 0.12 } else { 0.0 },
            );
    if away {
        qb_why(7, s.dcall.coverage);
        // Throw it away (outside the pocket it is legal past the line).
        let side = if me.y > mid() { 1.0 } else { -1.0 };
        let to = v2(los + 3.0, if side > 0.0 { width() + 2.0 } else { -2.0 });
        // A smart passer throws it near someone (decisions, football IQ).
        let sense = 0.5 * sim.agents[qb].a(Attr::Decisions) + 0.5 * aw;
        if !outside && sim.rng.chance(0.08 * clamp(1.0 - 0.02 * (sense - 60.0), 0.4, 1.8)) {
            let (id, at) = (sim.agents[qb].id, me.x);
            sim.notes.fouls.push(Foul { kind: FoulKind::IntentionalGrounding, side: 0, player: id, at });
        }
        ps.thrown = Some(t);
        sim.ball = Ball::Air { from: me, to, release: t, arrive: t + 0.9, target: None, height: 3.0 };
        return Some((to, qb, 0.0, 0.0, 0.0));
    }
    None
}

/// How open a receiver will be when the ball gets there: the gap the
/// nearest defender can't close in the time the ball is in the air, and
/// any defender sitting in the throwing lane.
fn window(sim: &Sim, qb: usize, r: usize) -> (f64, V2, f64) {
    let q = sim.agents[qb].pos;
    let ra = &sim.agents[r];
    let power = sim.agents[qb].a(Attr::ArmStrength);
    let speed = 16.5 + 0.13 * power;
    let mut p = ra.pos;
    let mut flight = 0.0;
    // The look-ahead's spots go in this thread's buffer from the last call
    // (`PREDICT_SPOTS`), emptied first: only its room is reused.
    let mut ahead = Predict::new(sim, r, PREDICT_SPOTS.take());
    for _ in 0..3 {
        let dist = q.dist(p);
        flight = 0.28 + dist / (speed * (1.0 - 0.005 * (dist - 20.0).max(0.0)));
        p = ahead.at(flight);
    }
    PREDICT_SPOTS.set(ahead.spots);
    let mut w = f64::INFINITY;
    // How far his read of the field trails his level's typical starter's,
    // in tens of points (minor bug 283): a poor reader doesn't see the man
    // sitting in his lane or the defender reading his eyes, so the window
    // looks bigger to him than it is; a good one sees it smaller than the
    // typical man would and only throws what is there.
    let misread = (typical_field_read(sim.level) - field_read(&sim.agents[qb])) / 10.0;
    for d in 0..sim.agents.len() {
        let a = &sim.agents[d];
        if a.side != 1 || !a.up(sim.t) || matches!(a.job, Job::Rush { .. }) {
            continue;
        }
        // A man defender already running with him needs no reaction time.
        let on_him = matches!(a.job, Job::Man { target, .. } if target == r);
        let watching = !on_him && sim.sees(d, q);
        let (react, pace) = if on_him {
            (0.0, 1.0)
        } else {
            ((0.3 + if watching { MISREAD_EYES * misread } else { 0.0 }).max(0.0), 0.95)
        };
        // How far he can get there: he reacts, then speeds up from what
        // he is doing now (a man running away has to stop first).
        // While he reacts he keeps moving as he was: a deep defender
        // already running with the route loses no ground to it.
        let tau = (flight - react).max(0.0);
        let dist = a.pos.dist(p);
        // The most he can close is top speed all the way: `close` below is
        // never more (his speed toward the spot is at most his top speed,
        // the sprint curve only takes away, and each step rounds the same
        // way). When even that leaves him farther off than the room already
        // found, or than the 8 yards the window is capped at, his gap can't
        // change the answer, and his reach (its exponential) isn't worked
        // out: the window is the same number (world speed lane). A NaN
        // bound fails the test: worked out.
        let most = a.vmax * (flight - tau) + pace * (a.vmax * tau).max(0.0);
        let far = dist - most > w.min(8.0);
        if !far {
            let v0 = clamp(a.vel.dot((p - a.pos).norm()), -2.0, a.vmax);
            let drift = v0 * (flight - tau);
            let close = drift + pace * (a.vmax * tau - (a.vmax - v0) * a.tau * (1.0 - (-tau / a.tau).exp())).max(0.0);
            let gap = dist - close;
            w = w.min(gap);
        }
        // In the lane: a defender between the quarterback and the spot.
        let seg = p - q;
        let k = clamp((a.pos - q).dot(seg) / seg.dot(seg).max(1e-6), 0.0, 1.0);
        let lane = if k > 0.3 && k < 0.95 { (q + seg.scale(k)).dist(a.pos) - 0.4 - 0.8 * k } else { f64::INFINITY };
        // A taller linebacker in the lane takes more of it (bug 309: real
        // quarterbacks throw at taller linebackers less, -0.14 pts of
        // targets a snap an inch at the same draft slot).
        let lane = if a.label == "LB" && lane.is_finite() {
            let g = sim.player(d);
            lane - 0.3 * (g.height - super::synthetic::typical_body(sim.level, g.position).0)
        } else {
            lane
        };
        w = w.min(lane + 0.6 + MISREAD_LANE * misread);
    }
    // He knows his arm: a spot past his range is never open to him.
    if q.dist(p) > 25.0 + 0.45 * power {
        w = w.min(-3.0);
    }
    (w.min(8.0), p, flight)
}

/// His highest reach at the catch point, inches off the ground (D1): his
/// standing reach (shoulder height plus arm length, the shoulder raised)
/// plus the jump he has left. His vertical is the combine's read of his
/// jumping and burst for his weight; running full speed costs about a third
/// of it (he can't gather), and a man who can't see the ball can't time it.
/// Highpointer adds the timing at the top (five jumping points).
pub(crate) fn high_reach(sim: &Sim, i: usize, sees_ball: bool) -> f64 {
    let gp = sim.player(i);
    let a = &sim.agents[i];
    let standing = 0.82 * gp.height + gp.arm + 4.0;
    let burst = a.a(Attr::Jumping).max(0.95 * a.a(Attr::Acceleration));
    let vert = (20.9 + 0.182 * (burst + gp.links.edge(&[Link::Highpointer])) - 0.035 * (gp.weight - 220.0)).max(8.0);
    let pace = clamp(a.vel.len() / a.vmax0.max(a.vmax).max(1.0), 0.0, 1.0);
    standing + vert * (1.0 - 0.35 * pace * pace) * if sees_ball { 1.0 } else { 0.6 }
}

/// A size mismatch the quarterback sees (D2): his covered receiver is
/// still open up high, for a jump ball or a back-shoulder throw, when he
/// reaches clearly higher than the man on him. Only on the throws a ball
/// can be put up on: down the field on vertical and breaking routes, and
/// anything but the flat in the red zone. It counts as extra room, about
/// 0.07 of a yard per inch of reach past two, for a quarterback who reads
/// it (football IQ and decisions) and trusts his man's hands (contested
/// catches). Returns the yards of room it is worth to him.
fn size_window(sim: &Sim, qb: usize, r: usize, p: V2, w: f64, s: &Snap) -> f64 {
    if !(-2.5..1.6).contains(&w) {
        return 0.0;
    }
    let Job::Route { name, .. } = sim.agents[r].job else { return 0.0 };
    let depth = (p.x - s.los) * sim.goal;
    let red_zone = s.los + depth >= 88.0;
    let vertical = matches!(name, "go" | "fade" | "seam" | "post" | "corner" | "wheel" | "comeback" | "out" | "dig");
    let short_stuff = matches!(name, "flat" | "swing" | "check-release" | "screen" | "bubble" | "drag");
    if short_stuff || !(vertical && depth >= 8.0 || red_zone && depth >= 2.0) {
        return 0.0;
    }
    // The man on him: the nearest defender to the spot.
    let Some(d) = (0..sim.agents.len())
        .filter(|&d| sim.agents[d].side == 1 && sim.agents[d].up(sim.t))
        .min_by(|&a, &b| sim.agents[a].pos.dist(p).total_cmp(&sim.agents[b].pos.dist(p)))
    else {
        return 0.0;
    };
    if sim.agents[d].pos.dist(p) > 3.0 {
        return 0.0;
    }
    // A trailing defender with his back to the throw can't time his jump:
    // the back-shoulder throw.
    let sees = sim.agents[d].face.dot((sim.agents[qb].pos - sim.agents[d].pos).norm()) > 0.0;
    let adv = high_reach(sim, r, true) - high_reach(sim, d, sees);
    if adv <= 2.0 {
        return 0.0;
    }
    let q = &sim.agents[qb];
    let sense = clamp(0.6 + 0.01 * (0.5 * q.a(Attr::FootballIq) + 0.5 * q.a(Attr::Decisions) - 50.0), 0.4, 1.2);
    let hands = clamp(0.6 + sim.agents[r].a(Attr::ContestedCatch) / 150.0, 0.6, 1.3);
    (0.07 * (adv - 2.0)).min(1.2) * sense * hands
}

thread_local! {
    /// `Predict`'s spots, kept on this thread between `window` calls and
    /// emptied at each one: only the room is reused.
    static PREDICT_SPOTS: std::cell::Cell<Vec<V2>> = const { std::cell::Cell::new(Vec::new()) };
}

/// Where a receiver will be: the quarterback knows his route and how his
/// man runs it, so he plays the route forward with the receiver's own
/// speed, acceleration and cutting. The route is played forward once and
/// kept, so asking for several flight times costs one run.
struct Predict<'s> {
    sim: &'s Sim<'s>,
    r: usize,
    /// The receiver as he moves along the route (his job is not copied).
    a: Agent,
    next: usize,
    /// He has settled in his spot: he stays where he is.
    stopped: bool,
    /// Where he is after each tick, from now (index 0).
    spots: Vec<V2>,
}

impl<'s> Predict<'s> {
    /// `spots` is a buffer to reuse: it is emptied, then holds his spot
    /// now, as `vec![a.pos]` did.
    fn new(sim: &'s Sim<'s>, r: usize, mut spots: Vec<V2>) -> Predict<'s> {
        let src = &sim.agents[r];
        let next = if let Job::Route { next, .. } = &src.job { *next } else { 0 };
        let a = src.without_job();
        spots.clear();
        spots.push(a.pos);
        Predict { sim, r, next, stopped: false, spots, a }
    }

    fn at(&mut self, dt: f64) -> V2 {
        let Job::Route { pts, settle, .. } = &self.sim.agents[self.r].job else {
            let a = &self.sim.agents[self.r];
            let p = a.pos + a.vel.scale(dt);
            return v2(p.x.min(end_line() - 1.0), clamp(p.y, 1.5, width() - 1.5));
        };
        let steps = (dt / DT).round() as usize;
        let rr_typ = super::synthetic::typical(self.sim.level, self.sim.player(self.r).position, Attr::RouteRunning);
        while self.spots.len() <= steps {
            if !self.stopped {
                let a = &mut self.a;
                if self.next < pts.len() {
                    let (target, speed, done, dist) = route_step(a, pts, self.next, *settle, || rr_typ);
                    if done {
                        self.next += 1;
                    }
                    a.steer(toward(a.pos, target, dist).scale(speed));
                    a.pos = a.pos + a.vel.scale(DT);
                } else if *settle {
                    self.stopped = true;
                } else {
                    let v = a.vel;
                    let dir = if v.len() > 0.5 { v.norm() } else { v2(1.0, 0.0) };
                    let me = a.pos;
                    let dir = if me.y < 4.0 && dir.y < 0.0 || me.y > width() - 4.0 && dir.y > 0.0 {
                        v2(1.0, 0.0)
                    } else {
                        dir
                    };
                    a.steer(dir.scale(route_pace(a)));
                    a.pos = a.pos + a.vel.scale(DT);
                }
            }
            self.spots.push(self.a.pos);
        }
        let p = self.spots[steps];
        v2(p.x.min(end_line() - 1.0), clamp(p.y, 1.5, width() - 1.5))
    }
}

/// The throw: ball speed from arm strength, error from accuracy, distance,
/// pressure, throwing on the move and the wind (topic 1).
#[allow(clippy::too_many_arguments)]
fn throw(
    sim: &mut Sim,
    ps: &mut Pass,
    qb: usize,
    r: usize,
    p: V2,
    flight: f64,
    pressure: f64,
    s: &Snap,
) -> (V2, usize, f64, f64, f64) {
    let t = sim.t;
    let q = sim.agents[qb].pos;
    let power = sim.agents[qb].a(Attr::ArmStrength);
    let dist = q.dist(p);
    // Placement by air yards to the aim point, blended between the bands
    // (short to 10, medium 10-19, deep 20 and out) so there is no cliff.
    let air = ((p.x - q.x) * sim.goal).max(0.0);
    let mut acc = pass_accuracy(&sim.agents[qb], air);
    // Touch: over the linebacker and under the safety. On a medium throw
    // with a defender under its path, touch takes up to a 0.3 share of the
    // placement (proposed, not confirmed).
    let mid = clamp((air - 5.0) / 10.0, 0.0, 1.0) - clamp((air - 15.0) / 10.0, 0.0, 1.0);
    if mid > 0.0 && dist > 1.0 {
        let line = (p - q).norm();
        let under = (0..sim.agents.len()).any(|d| {
            let rel = sim.agents[d].pos - q;
            let k = rel.dot(line) / dist;
            sim.agents[d].side == 1 && (0.25..0.75).contains(&k) && rel.dot(line.perp()).abs() < 1.5
        });
        if under {
            acc += 0.3 * mid * (sim.agents[qb].a(Attr::Touch) - acc);
        }
    }
    // A ball he can't reach falls short.
    let range = 25.0 + 0.45 * power;
    let mut target = p;
    if dist > range {
        target = q + (p - q).norm().scale(range);
    }
    // Good passers put it where only their man can get it.
    if let Some(d) = (0..sim.agents.len())
        .filter(|&d| sim.agents[d].side == 1)
        .min_by(|&a, &b| sim.agents[a].pos.dist(p).total_cmp(&sim.agents[b].pos.dist(p)))
    {
        let away = (p - sim.agents[d].pos).norm();
        // With a defender close, he puts it on the side away from him as
        // well as he reads the leverage (minor bug 283: his read of the
        // field about his level's typical starter, `PLACE_READ` yards a 10
        // points, fading out by `PLACE_NEAR` yards; a ball to an open man
        // goes where it always did).
        let read = (field_read(&sim.agents[qb]) - typical_field_read(sim.level)) / 10.0;
        let near = clamp(1.0 - sim.agents[d].pos.dist(p) / PLACE_NEAR, 0.0, 1.0);
        target = target + away.scale((0.45 * acc / 100.0 + PLACE_READ * read * near).max(0.0));
    }
    // He leaves his man room to catch it in bounds (aim points stay a yard
    // and a half inside the lines; they used to sit on them, and a third of
    // wide-open quick outs sailed out of bounds).
    target.y = clamp(target.y, 1.5, width() - 1.5);
    let moving = sim.agents[qb].vel.len() > 3.0;
    ps.moving = moving;
    let poise = sim.agents[qb].a(Attr::Composure);
    // Error grows gently as accuracy falls: real completion rates drop only a
    // few points from the NFL to college and high school, where receivers
    // and coverage are just as far behind (proposed, not confirmed).
    let down = ((target.x - q.x) * sim.goal).max(0.0);
    let mut sd = aim_sd(acc, typical_accuracy(sim.level, air), air, dist, down);
    // Pressure in his face throws him off: completions under pressure ran
    // 53% in the NFL against 49% real, so the effect is about two thirds
    // stronger than it was (proposed, not confirmed).
    if pressure > 0.3 {
        sd *= 1.0 + 0.9 * pressure.min(1.5) * (1.35 - poise / 100.0);
    }
    if moving {
        sd *= 1.25 - 0.002 * sim.agents[qb].a(Attr::ThrowOnMove);
    }
    // Wind, a little more on the long ones: from 5 to 18 mph 2.1 points of
    // completions and 7.3% of the yards an attempt, real 2.0 and 6.9% from
    // 5-9 mph to 15+ (nflverse 2021-2025). Minor bug 144: 0.004 + 0.0004 a
    // yard with the old bad-weather sheet's short throws cost 13% of the
    // yards and raised completions (proposed, not confirmed).
    let wind = s.cond.wind * (0.010 + 0.0001 * dist);
    sd *= 1.0 + wind;
    if s.cond.rain || s.cond.snow {
        sd *= 1.1;
    }
    sd *= 1.0 + 0.08 * s.cond.cold_unused;
    let consistency = sim.player(qb).traits.consistency;
    sd *= 1.15 - 0.015 * consistency;
    // Chemistry (X5): ball placement where his man expects it.
    sd *= 1.0 - 0.12 * (sim.player(r).chem - super::roster::TYPICAL_CHEM) as f64;
    let along = (target - q).norm();
    let across = along.perp();
    // Depth is judged less well the farther downfield (the miss grows along
    // the line from 12 yards downfield: real NFL completion is 54.5% at
    // 10-19 air yards and 36% at 20+, the engine ran 60% and 46%; a long
    // throw across the field is not a deep ball), and deep balls need the
    // arm too: a weak arm has to put more air under it. (Minor bug 135:
    // 0.022 a yard left 20+ throws complete 39%; 0.035 gives 37.4%.)
    let judge = 1.0 + 0.035 * (down - 12.0).max(0.0);
    let deep = judge * if dist > 25.0 { 1.1 * clamp(1.0 + 0.01 * (75.0 - power), 0.8, 1.4) } else { 1.0 };
    let e1 = sd * deep * gauss(sim.rng);
    let e2 = sd * gauss(sim.rng);
    let mut to = target + along.scale(e1) + across.scale(e2);
    let mut arrive = t + flight;
    // Batted at the line: the ball pops up off his hands and comes down a
    // yard or two past him. A screen is lofted over the rush.
    if !matches!(ps.concept.drop, Drop::Screen | Drop::Bubble)
        && let Some(d) = batter(sim, qb, along, dist)
    {
        ps.batted = Some((d, to));
        sim.notes.tend.hands_up = Some(sim.agents[d].id);
        BATTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        to = sim.agents[d].pos + along.scale(1.0 + sim.rng.next());
        arrive = t + 0.5 + 0.4 * sim.rng.next();
    }
    if super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed) {
        let ra = &sim.agents[r];
        println!(
            "THROW t={t:.1} aim ({:.1},{:.1}) to ({:.1},{:.1}) rec ({:.1},{:.1}) vel ({:.1},{:.1}) vmax {:.1} flight {flight:.2} sd {sd:.2}",
            p.x, p.y, to.x, to.y, ra.pos.x, ra.pos.y, ra.vel.x, ra.vel.y, ra.vmax
        );
    }
    let band = if dist < 10.0 {
        0
    } else if dist < 20.0 {
        1
    } else {
        2
    };
    THROW_SD[2 * band].fetch_add((sd * 1000.0) as u64, std::sync::atomic::Ordering::Relaxed);
    THROW_SD[2 * band + 1].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    ps.thrown = Some(t);
    ps.target = Some(r);
    if !matches!(ps.concept.drop, Drop::Screen | Drop::Bubble) {
        qb_why(ps.why, s.dcall.coverage);
    }
    let height = if ps.batted.is_some() { 4.0 } else { 2.0 + 0.08 * dist };
    // How high it arrives (D1): a jump ball is put up where only his man
    // reaches, just under his high point, and misses high or low with the
    // placement; a long ball comes down over his head; the rest come in at
    // the chest.
    ps.jump_ball = ps.jump_r == Some(r) && ps.batted.is_none();
    let rec_h = sim.player(r).height;
    ps.ball_h = if ps.jump_ball {
        high_reach(sim, r, true) - 4.0 + (2.0 + 6.0 * sd).min(9.0) * gauss(sim.rng)
    } else if down >= 20.0 && flight > 1.2 {
        0.95 * rec_h + 6.0
    } else {
        0.72 * rec_h
    };
    sim.ball = Ball::Air { from: q, to, release: t, arrive, target: Some(r), height };
    // The defense reacts to the throw (topic 2.1): those watching the
    // quarterback see it at once; a man with his back turned reacts when the
    // receiver's hands go up.
    for d in 0..sim.agents.len() {
        if sim.agents[d].side != 1 {
            continue;
        }
        let pr = sim.agents[d].a(Attr::PlayRecognition);
        let base = 0.16 + 0.004 * (100.0 - pr) + 0.1 * sim.rng.next();
        let sees = sim.sees(d, q);
        let which = if matches!(sim.agents[d].job, Job::Man { .. }) { Attr::ManCoverage } else { Attr::ZoneCoverage };
        let cover = BALL_COVER
            * (sim.agents[d].a(which) - super::synthetic::typical_starter(sim.level, sim.player(d).position, which))
            / 10.0;
        // Coverage skill reads here too (minor bug 283): a better cover man
        // breaks on the ball sooner, `BALL_COVER` seconds a 10 points about
        // his level's typical starter at his position. A man defender with
        // his back turned feels the throw from his man's eyes and hands,
        // `MAN_FEEL` before they go up.
        let man_feel = if matches!(sim.agents[d].job, Job::Man { .. }) { MAN_FEEL } else { 0.0 };
        sim.agents[d].ball_react = if sees {
            t + (base - cover).max(0.05)
        } else {
            (arrive - 0.45 - man_feel + 0.004 * (100.0 - pr) - cover).max(t + (base - cover).max(0.05))
        };
        // A route jumper who guessed right broke before the ball was out;
        // one who guessed wrong turned the wrong way and finds it late.
        match sim.notes.tend.jump {
            Some((id, true)) if id == sim.agents[d].id => sim.agents[d].ball_react = t,
            Some((id, false)) if id == sim.agents[d].id => sim.agents[d].ball_react = arrive + 0.3,
            _ => {}
        }
    }
    // The receiver looks for it: at once on a timing throw, a beat later
    // when he is still running away from the quarterback.
    let aw = sim.agents[r].a(Attr::BallTracking);
    let facing_qb = sim.agents[r].face.dot((q - sim.agents[r].pos).norm()) > -0.2;
    // He can't know where a long ball will come down until he has seen
    // some of its flight: until then he keeps running his route, then he
    // adjusts to it. Good hands and eyes read it sooner (proposed, not
    // confirmed).
    let skill = 0.5 * aw + 0.5 * sim.agents[r].a(Attr::Catching);
    let read = (flight - 0.6).max(0.0) * clamp(0.55 - 0.002 * (skill - 60.0), 0.4, 0.7);
    sim.agents[r].ball_react = t + (if facing_qb { 0.1 } else { 0.25 } + 0.003 * (100.0 - aw)).max(read);
    // A rusher closing on him as the ball comes out can't pull up: a hit on
    // the quarterback, and roughing the passer when it lands well after
    // the release or high (real NFL: roughing about 0.15 a team game;
    // proposed, not confirmed). It used to take a man within 1.6 yards, and
    // the throw is never let go with one that close, so it never came.
    let late = (0..sim.agents.len())
        .filter(|&d| sim.agents[d].side == 1 && sim.agents[d].engaged.is_none() && sim.agents[d].up(t))
        .filter_map(|d| {
            let a = &sim.agents[d];
            let rel = q - a.pos;
            let closing = a.vel.dot(rel.norm());
            let contact = (rel.len() - 0.8) / closing.max(0.1);
            (rel.len() < 4.5 && closing > 2.0 && contact < 0.7).then_some((d, contact))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((d, contact)) = late {
        ps.hit = true;
        let id = sim.agents[d].id;
        sim.notes.hits.push(id);
        let disc = sim.trait20(d, Attr::Discipline);
        let base = if contact > 0.4 { 0.1 } else { 0.015 };
        if sim.rng.chance(base * (1.5 - disc / 20.0) * sim.crew.personal) {
            sim.notes.fouls.push(Foul { kind: FoulKind::RoughingPasser, side: 1, player: id, at: q.x });
        }
    }
    (to, r, dist, flight, sd)
}

/// A lineman in the throwing lane who gets a hand on the ball at the line:
/// long arms, a jump and a read of the quarterback's eyes, against a tall
/// passer with a quick release. A high, long throw clears him more often.
/// Real NFL: about 1.5-2% of attempts are batted (proposed, not confirmed).
fn batter(sim: &mut Sim, qb: usize, along: V2, dist: f64) -> Option<usize> {
    let q = sim.agents[qb].pos;
    let t = sim.t;
    // Pocket frame (Dynamic Links): a big quarterback's high release, about
    // what five quick-release points save.
    // The passer's own height is left out (minor bug 277: real batted
    // balls don't move with it); his level's typical height keeps the
    // level's rate where it was (it read -6% an inch from 6'3).
    let tall = super::synthetic::typical_body(sim.level, Position::QB).0 - 75.0;
    let passer = clamp(
        1.0 - 0.06 * tall - 0.008 * (sim.agents[qb].a(Attr::QuickRelease) + sim.link(qb, &[Link::PocketFrame]) - 60.0),
        0.4,
        1.8,
    );
    let arc = clamp(1.3 - dist / 30.0, 0.3, 1.2);
    for d in 0..sim.agents.len() {
        let a = &sim.agents[d];
        if a.side != 1 || !a.up(t) || !matches!(a.job, Job::Rush { .. }) {
            continue;
        }
        let rel = a.pos - q;
        let ahead = rel.dot(along);
        let off = rel.dot(along.perp()).abs();
        if !(1.0..8.0).contains(&ahead) || off > 1.0 {
            continue;
        }
        let gp = sim.player(d);
        let reach = clamp(
            1.0 + 0.015 * (a.a(Attr::Jumping) - 60.0)
                + 0.08 * (gp.arm - 33.5)
                + 0.06 * (gp.height - 75.0)
                + 0.01 * (a.a(Attr::Anticipation) - 60.0)
                + 0.015 * sim.link(d, &[Link::Length]),
            0.3,
            2.0,
        );
        // Gets his hands up (tendencies): he stops to get them in the lane.
        let hands = clamp(1.0 + 1.1 * sim.lean(d, Tend::HandsUp), 0.15, 2.4);
        let p = BAT * (1.0 - off) * reach * passer * arc * hands;
        if sim.rng.chance(p) {
            return Some(d);
        }
    }
    None
}

/// Feeding the best receiver (A9): the chance his first look goes to the
/// best receiver in the progression when he isn't already first, by how
/// much better he is (rating points). Real top receivers draw 25-30% of
/// their team's targets; the engine gave the best one too few (Claude's
/// pick, set by `match_check`).
const FEED_BASE: f64 = 0.1;
const FEED_PER_POINT: f64 = 0.02;
const FEED_MAX: f64 = 0.55;
/// The base chance he throws it away rather than force it to a covered man
/// (minor bug 138).
const GIVE_UP: f64 = 0.8;
/// The chance a tenth of a second a pocket passer with a lane breaks out
/// with his eyes downfield, and how long before he tucks it and runs more
/// readily (minor bug 138).
const EXTEND: f64 = 0.15;
const EXTEND_RUN: f64 = 1.2;

/// The base chance a lineman square in the lane bats the ball.
const BAT: f64 = 0.17;

/// How far a clean throw misses on average at this distance (yards). Each
/// accuracy point is worth 0.5% less miss (was 1.7%, first 2%): on rosters
/// like the world's, NFL passing ran 7.4 yards a throw and 68% complete
/// against FBS 6.4 and 57%, where real NFL passing runs 7.05 and 64.5% and
/// FBS 7.29 and 60.6%. Now NFL 7.0 and 64.6%, FBS 6.5 and 58% (proposed,
/// not confirmed).
/// The miss grows with the length of the throw and faster with how far
/// downfield it goes (a long throw across the field to an out is a firm,
/// flat ball; the curve used to take throw length alone, and a third of
/// wide-open quick outs from the far hash missed).
/// A passer's placement for a throw of this many air yards: short
/// accuracy to 5, medium from 15, deep from 25, blended between (PFF's
/// 0-9, 10-19 and 20+ bands, without their cliffs).
pub fn pass_accuracy(qb: &Agent, air: f64) -> f64 {
    let t1 = clamp((air - 5.0) / 10.0, 0.0, 1.0);
    let t2 = clamp((air - 15.0) / 10.0, 0.0, 1.0);
    (1.0 - t1) * qb.a(Attr::ShortAccuracy) + (t1 - t2) * qb.a(Attr::MediumAccuracy) + t2 * qb.a(Attr::DeepAccuracy)
}

/// The same blend for the level's typical starting passer.
fn typical_accuracy(level: crate::player::Level, air: f64) -> f64 {
    let t1 = clamp((air - 5.0) / 10.0, 0.0, 1.0);
    let t2 = clamp((air - 15.0) / 10.0, 0.0, 1.0);
    let ts = |a: Attr| super::synthetic::typical_starter(level, Position::QB, a);
    (1.0 - t1) * ts(Attr::ShortAccuracy) + (t1 - t2) * ts(Attr::MediumAccuracy) + t2 * ts(Attr::DeepAccuracy)
}

/// The miss, yards of spread, for a passer of placement `acc` where his
/// level's typical starter has `typ`. The level's typical starter misses as
/// he did (0.005 a point on the passer's scale); within a level each point
/// counts `AIM_SHORT`, `AIM_MEDIUM` or `AIM_DEEP` by the depth, blended as
/// `pass_accuracy` blends the skills (minor bug 283: on the old curve Short
/// Accuracy moved completion 0.05 of real starters' spread).
fn aim_sd(acc: f64, typ: f64, air: f64, dist: f64, down: f64) -> f64 {
    let t1 = clamp((air - 5.0) / 10.0, 0.0, 1.0);
    let t2 = clamp((air - 15.0) / 10.0, 0.0, 1.0);
    let k = (1.0 - t1) * AIM_SHORT + (t1 - t2) * AIM_MEDIUM + t2 * AIM_DEEP;
    let skill = (0.005 * (110.0 - pro(typ))).exp() * (-k * (acc - typ)).exp();
    (0.25 + 0.036 * dist + 0.0007 * down * down) * AIM_BASE * skill
}

/// A quarterback skill on the scale where it decides games: through the pro
/// range each point counts fully; below it the gap narrows, since the
/// receivers and coverage he plays with are just as far behind.
fn pro(x: f64) -> f64 {
    let knee = 68.0;
    if x >= knee { x } else { knee - (knee - x) * 0.25 }
}

/// An edge's burst on a pass rush (minor bug 286): an edge slower off the
/// mark than the typical edge at his level builds up to speed on his rush
/// as the typical edge does; a quicker one keeps his own. Real edges' 10-yard
/// split says nothing about their pressures once talent is known (real
/// -0.49 pts of pressures a defense snap a 0.1 s, band -0.99 to +0.01), so
/// a slow first step costs him nothing here, while a quick man's burst, the
/// freak's (Cain 2026-11-08: no caps on rare freaks), stays his. His weight,
/// turn and top speed are his own: with the typical edge's weight on the
/// rush as well, heavy edges took 35% of the top 10% of pressure seasons
/// (11% of all), light ones none (8% of all; real 18% of the top, 19% of
/// all), and as recruited 10 lb added +0.40 pts (real flat). The edge lane,
/// 2026-11-08; proposed, not confirmed. On runs, in pursuit and when the
/// quarterback breaks out his burst counts in full.
/// Sets it for one step; returns his own (tau, accel) to put back after it.
fn rush_feet(sim: &mut Sim, i: usize) -> (f64, f64) {
    let a = &sim.agents[i];
    let keep = (a.tau, a.accel);
    let t = super::synthetic::typical(sim.level, sim.player(i).position, Attr::Acceleration);
    // As `GamePlayer::sprint_tau` and `accel` read them, at the typical
    // edge's Acceleration, his own weight and how tired he is; he keeps
    // whichever burst is quicker, his own or that one. Compared on the burst
    // itself, not his game-day rating, so the day's form and a pulse never
    // decide which he gets (his burst reads neither).
    let w = a.mass;
    let tau = clamp(0.78 + 0.003 * (60.0 - t) + 0.0006 * (w - 220.0), 0.55, 1.0) * (1.0 + 0.1 * a.tired);
    let ac = if t >= 60.0 { 4.65 + 0.008 * (t - 60.0) } else { 4.65 + 0.0046 * (t - 60.0) };
    let accel = clamp(ac - 0.006 * (w - 220.0), 2.2, 7.5) * (1.0 - 0.1 * a.tired);
    let a = &mut sim.agents[i];
    (a.tau, a.accel) = (tau.min(keep.0), accel.max(keep.1));
    keep
}

/// A rusher's reach to the quarterback, in yards, beyond the 1.25 a free
/// man needs and the 1.2 one nearly through his block needs (bug 339 and
/// proposals E1 and E2, Cain's yes 2026-11-08; all proposed, not
/// confirmed):
/// - Finish: 0.2 yards ten points of Tackling and Pursuit over 60
///   (`FINISH_MARK`), read half his own and half the typical man's at his
///   position and level (`FINISH_OWN`), never below nothing. Real NFL
///   rushers finish 0.28 of their pressures (PFR) and the engine's NFL 0.22
///   (2.00 sacks a team game, real 2.41). An NFL edge is about 74 in both,
///   an FBS one 54 and a high school one 33, so college and high school
///   barely move (they were at or above real). Read on his own in full the
///   best finishers ran away with it (34 sacks in a season, the top 1% of
///   edge seasons 22; real best 22.5 and 23); half his own keeps the level
///   and halves the spread.
/// - Bend (E1): an edge's Flexibility about the typical edge's, 0.2 yards
///   ten points, -0.3 to +0.6: a man who bends the corner flattens to the
///   quarterback. The 3-cone turns -0.25 pts of sacks a pressure a 0.1 s
///   into +0.63 (real +0.40, band -0.02 to +0.82; pure, 4,000 games).
/// - Arms (E2): an edge's arm over the typical edge's, 0.18 yards an inch
///   while he is still on his blocker, scaled by his weight over the
///   typical edge's (0 at 13 lb under, 1 at the typical, up to 3): the
///   power rusher's long-arm stab. Pure arms add +0.68 pts of sacks a
///   pressure an inch (real +0.65), most for heavy men (real heavy +1.85,
///   light -0.15), and no pressures; short-armed men lose nothing.
/// - Weight (bug 358): an edge's weight about the typical edge's, 0.07
///   yards less reach every 10 lb over and more every 10 lb under, free or
///   on his block. Real heavy edges finish less of their pressures (-0.49
///   pts of sacks a pressure per 10 lb held; the engine was +0.53, so
///   heavy power rushers ran away with the sack tail); now -0.41 pure
///   (2,000 games an arm, seed 86). It touches no strength or force: a heavy
///   freak still sheds and drives the pocket as before.
const FINISH_REACH: f64 = 0.2;
const FINISH_MARK: f64 = 60.0;
const FINISH_OWN: f64 = 0.5;
const BEND_REACH: f64 = 0.2;
const ARM_REACH: f64 = 0.18;
const WEIGHT_REACH: f64 = 0.07;
/// How far from the quarterback a second rusher, free or winning his
/// block, shares the sack, in yards (bug 358). Real 10.8% of NFL sacks
/// are shared (PFR 2018-25); 2.25 gives 10.8% on seeds 86-95, and the
/// share halves a man's count, as the stat line keeps it. Proposed, not
/// confirmed.
const SHARE_RADIUS: f64 = 2.25;

fn sack_check(sim: &mut Sim, qb: usize, ps: &mut Pass, los: f64) {
    let t = sim.t;
    let me = sim.agents[qb].pos;
    if super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed) {
        for d in 0..sim.agents.len() {
            let a = &sim.agents[d];
            if a.side == 1 && a.engaged.is_none() && a.pos.dist(me) < 3.0 {
                println!("NEAR t={t:.1} {} dist {:.2} v {:.1} up {}", a.label, a.pos.dist(me), a.vel.len(), a.up(t));
            }
        }
    }
    // A rusher who has beaten his man but not yet shed him can still reach
    // the quarterback and drag him down with the blocker on him (dynasty
    // item 1: the hand fight and a collapsing pocket make sacks, not only a
    // free rusher).
    // His reach beyond the base (`FINISH_REACH`): the finisher's, an
    // edge's bend, and a long-armed edge's stab from his block.
    let finish = |d: usize| {
        let a = &sim.agents[d];
        let pos = sim.player(d).position;
        let typ = |x: Attr| super::synthetic::typical(sim.level, pos, x);
        let own = 0.5 * a.a(Attr::Tackling) + 0.5 * a.a(Attr::Pursuit);
        let t = 0.5 * typ(Attr::Tackling) + 0.5 * typ(Attr::Pursuit);
        FINISH_REACH * (FINISH_OWN * own + (1.0 - FINISH_OWN) * t - FINISH_MARK).max(0.0) / 10.0
    };
    let bend_reach = |d: usize| {
        let a = &sim.agents[d];
        if !matches!(a.job, Job::Rush { contain, .. } if contain != 0.0) {
            return 0.0;
        }
        let typ = super::synthetic::typical(sim.level, sim.player(d).position, Attr::Flexibility);
        clamp(BEND_REACH * (a.a(Attr::Flexibility) - typ) / 10.0, -0.3, 0.6)
    };
    let arm_reach = |d: usize| {
        let g = sim.player(d);
        if g.position != Position::EDGE {
            return 0.0;
        }
        let (_, tw, ta) = super::synthetic::typical_body(sim.level, g.position);
        ARM_REACH * (g.arm - ta).max(0.0) * clamp(1.0 + (g.weight - tw) / 13.0, 0.0, 3.0)
    };
    let weight_reach = |d: usize| {
        let g = sim.player(d);
        if g.position != Position::EDGE {
            return 0.0;
        }
        let (_, tw, _) = super::synthetic::typical_body(sim.level, g.position);
        -WEIGHT_REACH * (g.weight - tw) / 10.0
    };
    // (Within his reach on the square, `sq_lt`: the same answer for any
    // reach.)
    let through = |d: usize| {
        let a = &sim.agents[d];
        a.engaged.is_some_and(|k| sim.eng[k].pass && sim.eng[k].w > 0.45)
            && sq_lt(a.pos.dist_sq(me), 1.2 + arm_reach(d) + finish(d) + weight_reach(d))
    };
    let qv = sim.agents[qb].vel;
    // Closest approach over the last tick, as with tackles. It is worked
    // out only for an up defender past his last try and off a block: every
    // test is pure, so testing the cheap ones first gives the same answer.
    let free = |d: usize| {
        let a = &sim.agents[d];
        let rel = a.pos - me;
        let rv = (a.vel - qv).scale(DT);
        // His reach, summed as it always was (each part reads only him,
        // his job and his level's typical man: no dice).
        let reach = 1.25 + bend_reach(d) + finish(d) + weight_reach(d);
        // More than his reach and 0.05 off on either axis beyond the tick's
        // own movement, he can't have come within it (the closest point is
        // `rel` less at most all of `rv`, `back` being 0 to 1): the same
        // `false` without the roots and the division, the margin dwarfing
        // any rounding for a reach under a million yards (world speed
        // lane). A bigger, infinite or NaN reach is worked out in full, a
        // NaN or infinite axis fails the test or gives `false` both ways,
        // and a reach below nothing is `false` both ways.
        if reach < 1e6 && (rel.x.abs() - rv.x.abs() > reach + 0.05 || rel.y.abs() - rv.y.abs() > reach + 0.05) {
            return false;
        }
        let back = if rv.len() > 1e-6 { clamp(rel.dot(rv) / rv.dot(rv), 0.0, 1.0) } else { 0.0 };
        (rel - rv.scale(back)).len() < reach
    };
    let mut tacklers: Vec<usize> = (0..sim.agents.len())
        .filter(|&d| {
            let a = &sim.agents[d];
            a.side == 1 && a.up(t) && t - a.last_try > 0.6 && ((a.engaged.is_none() && free(d)) || through(d))
        })
        .collect();
    if tacklers.is_empty() {
        return;
    }
    tacklers.sort_by(|&a, &b| sim.agents[a].pos.dist(me).total_cmp(&sim.agents[b].pos.dist(me)));
    let d = tacklers[0];
    sim.agents[d].last_try = t;
    ps.hit = true;
    let grabbing = sim.agents[d].engaged.is_some();
    // Escape: a mobile, strong quarterback slips some.
    let (q, r) = (&sim.agents[qb], &sim.agents[d]);
    // Dynamic Links: Pocket frame shrugs off the arm tackle; Length wraps
    // him up from farther.
    // His moves read his weight (A3); strength stays his own.
    let esc = 0.4 * q.a(Attr::Elusiveness) + 0.3 * q.a(Attr::Agility) + 0.3 * q.a(Attr::Strength)
        - 0.7 * super::sim::size_cost(q.mass, 222.0)
        + if grabbing { sim.link(qb, &[Link::PocketFrame]) } else { 0.0 };
    let tk =
        0.6 * r.a(Attr::Tackling) + 0.2 * r.a(Attr::Strength) + 0.2 * r.a(Attr::Agility) + sim.link(d, &[Link::Length]);
    let seen = sim.sees(qb, r.pos);
    let logit = -2.4
        + (esc - tk) / 12.0
        + if seen { 0.0 } else { -1.0 }
        + if grabbing { 1.2 } else { 0.0 }
        + POCKET_ESCAPE
            * (q.a(Attr::PocketPresence) - super::synthetic::typical(sim.level, Position::QB, Attr::PocketPresence))
            / 10.0;
    if sim.rng.chance(1.0 / (1.0 + (-logit).exp())) {
        let (rid, qid) = (r.id, q.id);
        sim.notes.missed.push((rid, qid));
        sim.agents[d].down_until = t + 0.6;
        return;
    }
    // Sacked. Strip sacks come from the blind side. The man who brings him
    // down always gets the sack; a second rusher on him shares it.
    // The nearest other rusher on the quarterback when he goes down, free
    // or winning his own pass block, inside `SHARE_RADIUS`, shares it (bug
    // 358: it used to take a second man inside 1.1 yards among the
    // tacklers, and none ever was).
    let other = (0..sim.agents.len())
        .filter(|&k| {
            let a = &sim.agents[k];
            k != d
                && a.side == 1
                && a.up(t)
                && a.pos.dist(me) < SHARE_RADIUS
                && a.engaged.is_none_or(|e| sim.eng[e].pass && sim.eng[e].w > 0.45)
        })
        .min_by(|&x, &y| sim.agents[x].pos.dist(me).total_cmp(&sim.agents[y].pos.dist(me)));
    sim.notes.sackers = std::iter::once(d).chain(other).map(|k| sim.agents[k].id).collect();
    sim.notes.hits.push(sim.agents[d].id);
    let force = sim.agents[d].mass * sim.agents[d].vel.len();
    let carry = sim.agents[qb].a(Attr::BallSecurity);
    // His blind side is behind his throwing shoulder: the left for a
    // right-hander, the right for a lefty (proposed, not confirmed).
    let left = (sim.agents[d].pos.y - me.y) * sim.goal > 0.0;
    let blind = if sim.player(qb).left_handed { !left } else { left };
    let p_fumble = (if seen { 0.06 } else { 0.16 }) * (1.3 - carry / 120.0) * if blind { 1.2 } else { 0.8 };
    sim.fight = vec![d];
    if sim.rng.chance(p_fumble) {
        sim.notes.fumbled = Some(sim.agents[qb].id);
        sim.notes.forced_fumble = Some(sim.agents[d].id);
        sim.fumble(qb);
    } else {
        sim.progress = me.x.min(los);
        // The man who brings him down has the tackle; a second sacker is in
        // on it (an assist), as the NFL and the NCAA keep a shared sack.
        sim.notes.tacklers = sim.notes.sackers.clone();
        sim.dead = true;
        sim.ball = Ball::Dead(me);
    }
    sim.injury_check(qb, force, if seen { 0.8 } else { 1.3 });
}

// ------------------------------------------------------------- defense

fn defend(sim: &mut Sim, i: usize, s: &Snap, qb: usize, pass: &Option<Pass>, trail: &Trails, in_air: bool) {
    let t = sim.t;
    let job = sim.agents[i].job.clone();
    let carrier = sim.carrier();
    // The man returning an interception runs with it (the ball carrier's
    // step moves him). He used to chase himself here first, braking to a
    // stop every tick: 2 pick sixes in 999 NFL games, real 0.05 a team
    // game (minor bug 132).
    if carrier == Some(i) {
        return;
    }
    let pass_play = pass.is_some();
    let knows = t >= sim.agents[i].read_at;
    if let Some(spot) = bitten(sim, i) {
        let (me, vm) = (sim.agents[i].pos, sim.agents[i].vmax);
        sim.steer(i, (spot - me).norm().scale(vm));
        return;
    }
    // Once the ball is caught or handed off and he has seen it, he chases.
    if let Some(c) = carrier {
        let handed = c != qb || pass.as_ref().is_some_and(|p| p.scramble);
        let chase = handed && (knows || sim.agents[i].pos.dist(sim.agents[c].pos) < 4.0);
        // On a run, coverage players fit by their run-fit rules below.
        let fit = !pass_play && knows && matches!(job, Job::Man { .. } | Job::Zone { .. } | Job::Rush { .. });
        if chase && !fit && !matches!(job, Job::Escort) {
            if matches!(job, Job::Man { .. } | Job::Zone { .. }) && pass_play && c == qb {
                // Coverage players stay on their men a beat when the
                // quarterback breaks the pocket, then come up.
                if sim.agents[c].pos.x < s.los + 0.5 {
                    cover(sim, i, &job, s, qb, pass, trail);
                    return;
                }
            }
            sim.pursue(i, c);
            return;
        }
    }
    // The screen: the line lets him through too easily and the back leaks
    // out; a defender who reads it peels off for the back.
    if let (Some(r), Some(c)) = (sim.screen, carrier)
        && c == qb
        && t > 1.25
        && matches!(job, Job::Rush { .. } | Job::Zone { .. } | Job::Man { .. })
    {
        let near = sim.agents[i].pos.dist(sim.agents[r].pos) < 13.0;
        let pr = sim.agents[i].a(Attr::PlayRecognition);
        if !sim.agents[i].smell && near && sim.rng.chance(0.08 * (pr / 70.0).powi(2)) {
            sim.agents[i].smell = true;
        }
        if sim.agents[i].smell {
            sim.pursue(i, r);
            return;
        }
    }
    if in_air
        && t >= sim.agents[i].ball_react
        && let Ball::Air { to, .. } = sim.ball
    {
        // Break on the ball: everyone rallies to where it is going (a
        // safety reading the quarterback comes downhill at once).
        let d = sim.agents[i].pos.dist(to);
        if d < 25.0 {
            sim.go_to(i, feet_in(to), 1.0);
            return;
        }
    }
    match job {
        Job::Rush { lane, contain, .. } => {
            // Rushes upfield or keeps contain (tendencies): an upfield edge
            // squeezes a run from deeper in the backfield (his pass rush is
            // in `rush_power` and the quarterback's escape lanes).
            let up = if contain != 0.0 { sim.lean(i, Tend::Upfield) } else { 0.0 };
            let deep = 1.0 * up.max(0.0);
            // The looper on a stunt waits a beat for the penetrator to cross.
            if t < sim.agents[i].hold_until {
                let me = sim.agents[i].pos;
                sim.go_to(i, v2(s.los + 0.4, me.y + clamp(lane - me.y, -1.0, 1.0)), 0.35);
                return;
            }
            let q = match carrier {
                Some(c) => sim.agents[c].pos,
                None => sim.agents[qb].pos,
            };
            // Before the quarterback, go through the lane; keep contain.
            let me = sim.agents[i].pos;
            // Through the line first (the switch sits short of the aim point so
            // he never stalls easing into it), then at the quarterback.
            let target =
                if me.x > s.los - 1.0 { v2(s.los - 2.0, lane + contain * 1.2) } else { v2(q.x, q.y + contain * 0.8) };
            // On a run he squeezes his gap rather than rushing upfield.
            if !pass_play
                && knows
                && let Some(c) = carrier
            {
                // A lineman plays his gap at the line: he squeezes down
                // toward the run (an end keeps outside leverage) rather
                // than running up the field past it.
                let cp = sim.agents[c].pos;
                let behind = (s.los - cp.x) * sim.goal > 0.5;
                if behind && me.dist(cp) > 1.6 {
                    let x = s.los - sim.goal * (0.4 + deep.max(0.0));
                    let y = clamp(cp.y + contain * 1.2, lane - 1.6, lane + 1.6);
                    let aim = v2(x, me.y + clamp(y - me.y, -3.0, 3.0));
                    let vm = sim.agents[i].vmax * 0.5;
                    let gap = aim - me;
                    sim.steer(i, if gap.len() > 0.6 { gap.norm().scale(vm) } else { gap.scale(2.0) });
                    return;
                }
                sim.pursue(i, c);
                return;
            }
            if me.x <= s.los - 1.0 {
                // Past the line he runs through the quarterback, not to him;
                // the contain man stays a step outside until he is close.
                let qv = sim.agents[qb].vel;
                let close = me.dist(q) < 2.5;
                let aim = v2(q.x, q.y + if close { 0.0 } else { contain * 0.8 }) + qv.scale(0.3);
                // His burst on a pass rush (`rush_feet`).
                if pass_play && contain != 0.0 {
                    let keep = rush_feet(sim, i);
                    let vm = sim.agents[i].vmax;
                    sim.steer(i, (aim - me).norm().scale(vm));
                    (sim.agents[i].tau, sim.agents[i].accel) = keep;
                    return;
                }
                let vm = sim.agents[i].vmax;
                sim.steer(i, (aim - me).norm().scale(vm));
                return;
            }
            // His burst on a pass rush (`rush_feet`).
            if pass_play && contain != 0.0 {
                let keep = rush_feet(sim, i);
                sim.go_to(i, target, 1.0);
                (sim.agents[i].tau, sim.agents[i].accel) = keep;
                return;
            }
            sim.go_to(i, target, 1.0);
        }
        Job::Gap { y } => {
            let spot = v2(s.los + 1.0, y);
            sim.go_to(i, spot, 0.9);
        }
        Job::Spy => {
            // He mirrors the quarterback five yards off the line and comes
            // when he runs (topic 5.4).
            let q = sim.agents[qb].pos;
            if let Some(c) = carrier
                && (c != qb || q.x > s.los - 0.5 || !pass_play && knows)
            {
                sim.pursue(i, c);
                return;
            }
            let me = sim.agents[i].pos;
            let spot = v2(s.los + 5.0, me.y + clamp(q.y - me.y, -2.0, 2.0));
            sim.go_to(i, spot, 0.8);
        }
        Job::Man { .. } | Job::Zone { .. } => {
            if !pass_play && knows {
                // Run fit: linebackers fill, defensive backs come up to force.
                let c = carrier.unwrap_or(qb);
                let me = sim.agents[i].pos;
                let cp = sim.agents[c].pos;
                // Defensive backs fit the alley under control from wherever
                // they are (a safety rotated down too); a linebacker attacks.
                let back = matches!(sim.agents[i].label, "CB" | "NB" | "FS" | "SS");
                let deep = matches!(job, Job::Zone { deep: true, .. })
                    || (me.x - s.los) * sim.goal > 9.0
                    || (back && (me.x - s.los) * sim.goal > 5.0);
                // A corner in man stays on his receiver (who may be faking a
                // route or blocking him) until the runner is clearly through.
                // When his man blocks instead of running a route, that is his
                // run key: he comes up to play the run.
                let on_wr = matches!(job, Job::Man { target, .. } if !matches!(sim.agents[target].job, Job::RunBlock { .. }) && (sim.agents[target].pos.y - s.ball_y).abs() > 8.0);
                if on_wr && (cp.x - s.los) * sim.goal < 3.0 {
                    cover(sim, i, &job, s, qb, pass, trail);
                    return;
                }
                if deep {
                    // The last line fits the alley: he comes down to about
                    // eight and a half yards and squares up, and only
                    // attacks once the runner is through the line.
                    // He comes down under control, taking his angle to the
                    // runner, and runs all out once the runner is through.
                    let pr = sim.agents[i].a(Attr::PlayRecognition);
                    // A safety with a deep zone is the last line.
                    let post = matches!(job, Job::Zone { deep: true, .. });
                    // A back rolled down into the box (inside 7 yards and 7.5
                    // to the side of the ball, the box count) is an extra run
                    // fitter: he fits at 4.5 yards and attacks once the
                    // runner is 1.5 past the line. He fitted at 6.5 like any
                    // safety, so an eighth man in the box changed little
                    // (minor bug 136: 8-man boxes on neutral downs 3.85 a
                    // carry against 4.11 with 6, real 4.27 against 4.75).
                    // The others fit at 9.75 and attack at 4.75: at 6.5 and
                    // 2.5 they made 42% of first tackles, at 6.7 yards (real
                    // 29%, at 7.0; PFF roles, nflverse 2021-24), at 8.5 and
                    // 3.5 38%, and now 34% with the linebackers' fills
                    // (bugs 292 and 305; proposed, not confirmed).
                    // The force (minor bug 253): a back on the side the run
                    // goes wide to, once the runner is `FORCE_WIDE` yards
                    // outside the ball on his side, takes the box fit and
                    // attacks the edge, as a real force player turns a wide
                    // run back in; a deep zone stays the last line.
                    if !post && back {
                        let side = if me.y > s.ball_y { 1.0 } else { -1.0 };
                        if (cp.y - s.ball_y) * side > FORCE_WIDE {
                            sim.agents[i].force = true;
                        }
                    }
                    let boxed = sim.agents[i].force
                        || (!post && back && (me.y - s.ball_y).abs() < 7.5 && (me.x - s.los) * sim.goal < 7.0);
                    let (wait, fill) = if boxed { (1.5, 4.5) } else { (4.75, 9.75) };
                    // A hitter fits shallower and attacks sooner (S-B): 0.25
                    // and 0.2 yards a point of his read.
                    let h = sim.agents[i].hit;
                    let (wait, fill) = (wait - 0.2 * h, fill - 0.25 * h);
                    // A misfit safety is late to the alley.
                    let (wait, fill) = if sim.agents[i].misfit { (wait + 2.0, fill + 2.0) } else { (wait, fill) };
                    let through = (cp.x - s.los) * sim.goal > wait - 0.02 * (pr - 50.0);
                    // Only a corner holds his outside leverage out wide; a
                    // safety comes down to fill the alley.
                    let corner = matches!(sim.agents[i].label, "CB" | "NB");
                    let wide = !sim.agents[i].force
                        && corner
                        && (me.y - s.ball_y).abs() > 12.0
                        && (cp.x - s.los) * sim.goal < 3.0;
                    if wide {
                        // A deep corner keeps outside leverage until it breaks.
                        let depth = s.los + sim.goal * 9.0;
                        let x = if (me.x - depth) * sim.goal > 0.0 { depth } else { me.x };
                        sim.go_to(i, v2(x, me.y + (cp.y - me.y) * 0.2), 0.6);
                    } else {
                        let floor = if through { None } else { Some(s.los + sim.goal * (fill - 0.02 * (pr - 50.0))) };
                        sim.pursue_cap(i, c, if through { 1.0 } else { 0.6 }, floor);
                    }
                } else if sim.agents[i].label == "LB"
                    && (s.los - cp.x) * sim.goal > 0.0
                    && me.dist(cp) > 1.8 + 1.5 * sim.lean(i, Tend::Downhill)
                {
                    // A linebacker fills his gap: downhill to the line on the
                    // runner's side, where the blocks meet him, and he takes
                    // the runner on when he gets there (not in the backfield).
                    // He leaves the line for the runner only when he is
                    // nearly on him, from farther if he flies downhill (he
                    // went from four yards, and with backs at the real pace
                    // linebackers made 9.6 tackles for loss a season, real
                    // 5.3; proposed, not confirmed). He fills to 2.6 yards
                    // past the line, where the runner meets him after the
                    // line has moved: filling to half a yard he met the back
                    // at the line, so linebackers' stops gained 1.5 yards,
                    // real 3.7 (minor bug 164), and at 2 yards his stops
                    // came too short once he slipped more blocks (bug 305;
                    // proposed, not confirmed).
                    // A run more than 3 yards wide of him and going away: he
                    // flows over the top at full speed to where it turns up,
                    // holding his depth, so his speed is his range (L2,
                    // bug 306).
                    let cv = sim.agents[c].vel;
                    let wide = (cp.y - me.y).abs() > 3.0;
                    if wide && cv.y * (cp.y - me.y).signum() > 1.5 {
                        sim.pursue_cap(i, c, 1.0, Some(s.los + sim.goal * 2.0));
                        return;
                    }
                    let mut aim = v2(s.los + sim.goal * 2.6, me.y + clamp(cp.y - me.y, -2.5, 2.5));
                    // A misfit: he flows a gap too far, or the false way on
                    // misdirection, and the runner cuts back behind him.
                    if sim.agents[i].misfit && t < sim.agents[i].read_at + 0.8 {
                        let dir = if misdirection(s) { -s.call.dir } else { s.call.dir };
                        aim.y += 2.5 * dir;
                    }
                    sim.go_to(i, aim, 1.0);
                } else if sim.agents[i].label == "LB" && (s.los - cp.x) * sim.goal > 0.0 {
                    // Nearly on him with the runner still behind the line:
                    // he takes his angle but holds 2.1 yards past the line
                    // and lets the runner come (he chased him into the
                    // backfield; minor bug 164; 1.5 before bug 305).
                    sim.pursue_cap(i, c, 1.0, Some(s.los + sim.goal * 2.1));
                } else {
                    sim.pursue(i, c);
                }
                return;
            }
            let fake = pass.as_ref().is_some_and(|p| p.concept.play_action);
            if !knows && sim.agents[i].label == "LB" && !fake {
                // A linebacker reads before he commits, run or pass alike: a
                // read step at his own depth, shuffling with the back he keys
                // (or the ball), rather than dropping or running to the back.
                let me = sim.agents[i].pos;
                let key = match job {
                    Job::Man { target, .. } if (sim.agents[target].pos.x - s.los) * sim.goal < -1.0 => {
                        sim.agents[target].pos.y
                    }
                    _ => sim.ball_pos().y,
                };
                sim.go_to(i, v2(me.x, me.y + clamp(key - me.y, -2.0, 2.0) * 0.5), 0.5);
                return;
            }
            if pass_play && !knows {
                // Fooled for now: a play fake pulls him toward the line.
                // A man defender on a wide receiver has his eyes on his man.
                let on_wide =
                    matches!(job, Job::Man { target, .. } if (sim.agents[target].pos.y - s.ball_y).abs() > 8.0);
                if pass.as_ref().is_some_and(|p| p.concept.play_action)
                    && !matches!(job, Job::Zone { deep: true, .. })
                    && !on_wide
                {
                    let me = sim.agents[i].pos;
                    let spot = v2(me.x - 1.5, me.y + (sim.agents[qb].pos.y - me.y) * 0.2);
                    sim.go_to(i, spot, 0.6);
                    return;
                }
            }
            cover(sim, i, &job, s, qb, pass, trail);
        }
        Job::Pursue => {
            if let Some(c) = carrier {
                sim.pursue(i, c);
            }
        }
        Job::Escort => escort(sim, i),
        _ => {}
    }
}

/// A per-play draw for one player that needs no random stream: the same
/// player on the same snap gets the same number, so a decision made on
/// every tick holds for the whole play.
fn play_draw(sim: &Sim, i: usize, salt: u64) -> f64 {
    let mut z = (sim.agents[i].id as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((sim.los * 10.0) as u64)
        .wrapping_add(salt.wrapping_mul(0xD6E8_FEB8_6659_FD93));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Jumping the route (topic 3.1, tendencies): when the quarterback commits
/// to a throw near him (his own man in man coverage, any route within
/// four and a half yards in zone), a route jumper breaks on it before the ball is
/// out. How often he gambles is his tendency; whether he guessed right is
/// his anticipation and read, and a planned double move or pump fake makes
/// the guess harder. Right, he breaks to the catch point and plays the
/// ball at once (an interception chance); wrong, he bit on the fake: he
/// drives for the break he guessed, his man turns it upfield past him (the
/// sight adjustment a receiver is taught against a jumper), the throw leads
/// the man deep, and the jumper needs a moment to plant and turn before he
/// can chase (trial 8: wrong guesses gave up no more big plays, because he
/// trailed his man three yards underneath all the way). His coach's
/// strictness and trust still decide whether he may (proposed, not
/// confirmed: chance a throw near him 0.07 at an average tendency, times
/// 1 + 4 lean, none below about 8; the turn takes 0.5-0.9 s by his
/// anticipation).
fn jump_route(sim: &mut Sim, i: usize, s: &Snap, qb: usize, pass: &Option<Pass>, man: Option<usize>) -> Option<V2> {
    // He gambles once a play.
    if sim.bites.iter().any(|b| b.0 == i) {
        return None;
    }
    let p = pass.as_ref()?;
    if p.thrown.is_some() {
        return None;
    }
    let (w, ..) = p.commit?;
    let me = sim.agents[i].pos;
    let wp = sim.agents[w].pos;
    if man.is_some_and(|m| m != w) || wp.dist(me) > 4.5 {
        return None;
    }
    if !freelance_ok(sim, i, s.strictness[1], s.trust[1][sim.agents[i].p]) {
        return None;
    }
    let lean = sim.lean(i, Tend::JumpRoutes);
    if play_draw(sim, i, 1) > clamp(0.07 * (1.0 + 4.0 * lean), 0.0, 0.9) {
        return None;
    }
    let antic = sim.agents[i].a(Attr::Anticipation);
    let pr = sim.agents[i].a(Attr::PlayRecognition);
    let p_right = clamp(0.35 + 0.012 * (antic - 60.0) + 0.006 * (pr - 60.0) - 0.25 * s.tac.double_moves, 0.1, 0.9);
    let right = play_draw(sim, i, 2) < p_right;
    let id = sim.agents[i].id;
    if sim.notes.tend.jump.is_none() {
        sim.notes.tend.jump = Some((id, right));
    }
    if !sim.notes.freelance.iter().any(|f| f.0 == id) {
        sim.notes.freelance.push((id, false));
    }
    // Right, he reads where the ball is going (the spot the quarterback
    // will lead his man to) and gets in front of it.
    let (_, spot, _) = window(sim, qb, w);
    let to_qb = (sim.agents[qb].pos - spot).norm();
    if right {
        return Some(spot + to_qb.scale(0.4));
    }
    let guess = wp + to_qb.scale(3.5) - sim.agents[w].vel.scale(0.8);
    sim.bites.push((i, guess, sim.t));
    // His man sees him bite and goes over the top.
    let goal = sim.goal;
    if let Job::Route { pts, next, settle, .. } = &mut sim.agents[w].job {
        // Over the top, but no deeper than any route: the end line.
        *pts = vec![v2((wp.x + 30.0 * goal).clamp(back_line() + 1.0, end_line() - 1.0), wp.y)];
        *next = 0;
        *settle = false;
    }
    Some(guess)
}

/// A route jumper who guessed wrong keeps going for the break he guessed
/// until he has planted and turned (see `jump_route`).
fn bitten(sim: &Sim, i: usize) -> Option<V2> {
    let &(_, spot, at) = sim.bites.iter().find(|b| b.0 == i)?;
    let turn = (0.9 - 0.008 * (sim.agents[i].a(Attr::Anticipation) - 40.0)).clamp(0.5, 0.9);
    (sim.t < at + turn).then_some(spot)
}

fn cover(sim: &mut Sim, i: usize, job: &Job, s: &Snap, qb: usize, pass: &Option<Pass>, trail: &Trails) {
    cover_inner(sim, i, job, s, qb, pass, trail);
    // The end line: nobody covers anything behind it.
    let a = &mut sim.agents[i];
    let back = end_line() - 0.7;
    if (a.pos.x - back) * sim.goal > 0.0 && a.vel.x * sim.goal > 0.0 {
        a.vel.x = 0.0;
    }
}

fn cover_inner(sim: &mut Sim, i: usize, job: &Job, s: &Snap, qb: usize, pass: &Option<Pass>, trail: &Trails) {
    let t = sim.t;
    match *job {
        Job::Man { target, cushion, lag } => {
            // He mirrors where he last saw his man going (topic 2): when the
            // receiver breaks, he is late by his reaction time.
            let n = trail.ticks;
            let k = ((lag / DT).round() as usize).min(n.saturating_sub(1));
            let (p, v) =
                if n > 0 { trail.at(n - 1 - k, target) } else { (sim.agents[target].pos, sim.agents[target].vel) };
            // He is late on where his man turns, not on how fast he runs:
            // speed he feels at once, direction he sees late.
            let now = sim.agents[target].vel.len();
            let v = if v.len() > 0.3 { v.norm().scale(now.max(v.len())) } else { sim.agents[target].vel };
            let est = p + v.scale(lag);
            let me = sim.agents[i].pos;
            let inside = if est.y > s.ball_y { -0.6 } else { 0.6 };
            let deep = est.x - s.los > 8.0;
            // Off coverage keeps a cushion until the receiver eats it up.
            // Before he has read the play, a man whose receiver isn't
            // releasing (he blocks, or stays in) keeps his cushion: he
            // doesn't come downhill on a blocker until he reads run.
            let releasing = sim.agents[target].vel.x * sim.goal > 1.0 || (est.x - s.los) * sim.goal > 1.5;
            let cush = if t < sim.agents[i].read_at && !releasing {
                cushion
            } else {
                (cushion - (t * 2.2)).max(if deep { 0.6 } else { 0.0 })
            };
            let spot = v2(est.x + cush - if deep { 0.3 } else { 0.0 }, est.y + inside);
            // Bust: lost his man.
            if sim.agents[i].bust {
                let s2 = v2(me.x, me.y);
                sim.go_to(i, s2, 0.4);
                return;
            }
            // He runs with his man's (lagged) velocity and closes the gap to
            // his spot on top of that.
            let want = v + (spot - me).scale(2.0);
            if let Some(j) = jump_route(sim, i, s, qb, pass, Some(target)) {
                let vm = sim.agents[i].vmax;
                sim.steer(i, (j - me).norm().scale(vm));
                return;
            }
            sim.steer(i, want);
            // Hand-fighting downfield can draw a flag (defensive holding,
            // illegal contact, illegal hands: real NFL about 0.65 a team game
            // together, Claude's reading of 2023; it ran 0.09).
            let rd = sim.agents[target].pos.dist(me);
            if rd < 0.9
                && t > 0.8
                && t < 2.6
                && sim.rng.chance(
                    0.0105
                        * sim.crew.holding
                        * (0.886 + 0.7 * (10.0 - sim.trait20(i, Attr::Discipline)) / 14.0).max(0.1)
                        * hold_body(sim, i),
                )
            {
                let (id, at) = (sim.agents[i].id, me.x);
                let ok = pass.as_ref().is_some_and(|p| p.thrown.is_none());
                if ok {
                    sim.notes.fouls.push(Foul { kind: FoulKind::DefensiveHolding, side: 1, player: id, at });
                }
            }
        }
        Job::Zone { spot, deep, width, matched } => {
            let me = sim.agents[i].pos;
            let q = sim.agents[qb].pos;
            // Threats in his area that he can see, on the stack (a list only
            // past 32, which an offense never has): the same men in the same
            // order as the list they used to go in.
            let mut threat_buf = [0usize; 32];
            let mut n_threats = 0usize;
            let mut spill: Vec<usize> = Vec::new();
            for r in 0..sim.agents.len() {
                let a = &sim.agents[r];
                // The offense comes first in `agents` (`run_snap` puts its
                // men in before the defense's, and nobody changes sides):
                // past it nobody runs a route into his area, and the rest
                // were all passed over (world speed lane).
                if a.side != 0 {
                    break;
                }
                if !matches!(a.job, Job::Route { .. }) {
                    continue;
                }
                let in_zone = (a.pos.y - spot.y).abs() < width / 2.0 + 2.0
                    && if deep {
                        a.pos.x > s.los + 6.0 || a.pos.x > s.los + 1.5 && a.vel.x > 4.0
                    } else {
                        a.pos.x > s.los - 1.0 && a.pos.x < spot.x + 9.0
                    };
                // A deep defender running with a man feels him over his shoulder.
                // An underneath defender feels a route going behind him (he
                // reads the release and hears the call) when it is close.
                // The felt test goes first: both tests are pure, and it often
                // spares the vision cone's arc cosine.
                if in_zone && ((deep || a.pos.x > me.x) && a.pos.dist(me) < 10.0 || sim.sees(i, a.pos)) {
                    if n_threats < threat_buf.len() {
                        threat_buf[n_threats] = r;
                        n_threats += 1;
                    } else {
                        if spill.is_empty() {
                            spill.extend_from_slice(&threat_buf);
                        }
                        spill.push(r);
                    }
                }
            }
            let threats: &[usize] = if spill.is_empty() { &threat_buf[..n_threats] } else { &spill };
            // A sharp defender reads the quarterback's eyes; a looked-off one bites.
            let eyes = pass.as_ref().and_then(|p| p.reads.get(p.read).copied());
            // Pattern reading in zone (zone coverage with recognition) sets
            // how late he sees a break; anticipation sets how he leans to
            // the quarterback's eyes and jumps a route.
            let pr = 0.6 * sim.agents[i].a(Attr::ZoneCoverage) + 0.4 * sim.agents[i].a(Attr::PlayRecognition);
            let antic = sim.agents[i].a(Attr::Anticipation);
            let mut target = spot;
            // A deep defender keeps a cushion that grows with the threat's
            // speed and runs with him (a speed turn) rather than waiting.
            let mut carry = V2::default();
            if deep {
                if let Some(&r) = threats.iter().max_by(|&&a, &&b| sim.agents[a].pos.x.total_cmp(&sim.agents[b].pos.x))
                {
                    let rp = sim.agents[r].pos;
                    let rv = sim.agents[r].vel;
                    let cushion = 1.5 + 0.4 * rv.x.max(0.0);
                    // Two verticals in his area: he splits them, leaning to the deeper.
                    // The two deepest past 8 yards, deepest first and an
                    // equal one behind the one met first, as the stable sort
                    // of the list they went in put them; kept on the stack
                    // (world speed lane).
                    let mut verts: [Option<V2>; 2] = [None, None];
                    for p in threats.iter().map(|&k| sim.agents[k].pos).filter(|p| p.x > s.los + 8.0) {
                        let first = verts[0];
                        match first {
                            Some(a) if p.x.total_cmp(&a.x).is_le() => {
                                if verts[1].is_none_or(|b| p.x.total_cmp(&b.x).is_gt()) {
                                    verts[1] = Some(p);
                                }
                            }
                            _ => verts = [Some(p), first],
                        }
                    }
                    let ty = if let [Some(a), Some(b)] = verts {
                        // The same sum over the same two, in the same order.
                        let mean = [a, b].iter().map(|p| p.y).sum::<f64>() / 2.0;
                        0.6 * mean + 0.4 * rp.y
                    } else {
                        spot.y + (rp.y - spot.y) * 0.65
                    };
                    target = v2((rp.x + cushion).max(spot.x), ty.clamp(spot.y - width / 2.0, spot.y + width / 2.0));
                    // He matches the vertical's speed and lets the gap to
                    // his cushion set how much faster or slower he goes, so
                    // he never stops on his spot and has to restart.
                    if rv.x > 3.0 {
                        carry = v2(rv.x * 0.9, 0.0);
                    }
                } else {
                    target = v2(spot.x.max(me.x), spot.y);
                }
            } else if let Some(&r) = threats
                .iter()
                .filter(|&&k| sim.agents[k].pos.x > s.los + 3.5)
                // The deepest route in his area: he walls it and lets the
                // short ones come to him.
                .max_by(|&&a, &&b| sim.agents[a].pos.x.total_cmp(&sim.agents[b].pos.x))
                .or_else(|| {
                    threats
                        .iter()
                        .min_by(|&&a, &&b| sim.agents[a].pos.dist(spot).total_cmp(&sim.agents[b].pos.dist(spot)))
                })
            {
                // He sits under a route at depth; a shallow one he only
                // squeezes, keeping his depth for what comes behind him.
                // His eyes are on the quarterback, so he sees a break late:
                // he plays where the route was a beat ago (quicker for a
                // sharp reader). Within a level it reads at `ZONE_LAG` about
                // the level's typical starter at his position, down to
                // 0.05 s (minor bug 283: at 0.004 from 60 Zone Coverage
                // moved the catch rate allowed 0.01-0.09 of real starters'
                // spread).
                let pos_i = sim.player(i).position;
                let ts = |a: Attr| super::synthetic::typical_starter(sim.level, pos_i, a);
                let pr_s = 0.6 * ts(Attr::ZoneCoverage) + 0.4 * ts(Attr::PlayRecognition);
                let lag = clamp(0.3 - 0.004 * (pr_s - 60.0) - ZONE_LAG * (pr - pr_s), 0.05, 0.6) + lb_hips(sim, i);
                let n = trail.ticks;
                let k = ((lag / DT).round() as usize).min(n.saturating_sub(1));
                let (rp, rv) = if n > 0 { trail.at(n - 1 - k, r) } else { (sim.agents[r].pos, sim.agents[r].vel) };
                let under = rp + (q - rp).norm().scale(1.3);
                let lim = width / 2.0 + 3.0;
                let floor = if rp.x > s.los + 3.5 { s.los } else { (spot.x - 3.0).max(s.los + 2.0) };
                target = v2(under.x.clamp(floor, spot.x + 6.0), under.y.clamp(spot.y - lim, spot.y + lim));
                // He moves with the route he is under (not easing into a spot
                // the receiver has already left).
                carry = rv.scale(0.9);
                // Match: a vertical in his area becomes his man.
                if matched && rv.x > 4.0 && rp.x > spot.x - 1.0 && rp.x < spot.x + 12.0 {
                    let inside = if rp.y > s.ball_y { -0.7 } else { 0.7 };
                    target = v2(rp.x - 0.3, rp.y + inside);
                    carry = rv;
                }
            } else {
                // Nothing in his area: he sinks under a vertical running
                // through it (a cloud corner carrying #1) until he hands it off.
                // Only the offense, which comes first (as above), and the
                // same pick as `min_by` on the distance (`Nearest`).
                let mut sink = Nearest::default();
                for (r, a) in sim.agents.iter().enumerate().take_while(|(_, a)| a.side == 0) {
                    if matches!(a.job, Job::Route { .. })
                        && (a.pos.y - spot.y).abs() < width / 2.0 + 1.0
                        && a.vel.x > 4.0
                        && a.pos.x > spot.x
                        && a.pos.x < spot.x + 14.0
                    {
                        sink.offer(r, a.pos.dist_sq(me));
                    }
                }
                if let Some(r) = sink.get() {
                    let rp = sim.agents[r].pos;
                    target = v2(rp.x - 1.5, rp.y + (q.y - rp.y).signum() * 0.8);
                }
            }
            if let Some(e) = eyes {
                let ep = sim.agents[e].pos;
                if ep.dist(target) < 9.0 {
                    let lean = clamp(0.004 * (antic - 40.0), 0.0, 0.3);
                    target = target + (ep - target).scale(lean);
                }
                // Looked off: a quarterback who holds the safety with his
                // eyes on the first read pulls a deep man toward it; a safety
                // who anticipates bites less (proposed, not confirmed).
                let held = pass.as_ref().map_or(0.0, |p| p.look_off);
                if deep && held > 0.0 {
                    let bite = clamp(1.0 - 0.01 * (antic - 60.0), 0.3, 1.5);
                    let pull = clamp(0.3 * held * bite * (ep.y - target.y), -4.0, 4.0);
                    target.y += pull;
                }
                // Jumping the route (topic 3.1, tendencies): see `jump_route`.
                if !deep && let Some(j) = jump_route(sim, i, s, qb, pass, None) {
                    target = j;
                }
            }
            if super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed)
                && deep
                && ((t * 10.0).round() as i64) % 5 == 0
            {
                println!(
                    "ZONE t={t:.1} {} me ({:.1},{:.1}) spot ({:.1},{:.1}) w {:.1} threats {} target ({:.1},{:.1}) carry {:.1}",
                    sim.agents[i].label,
                    me.x,
                    me.y,
                    spot.x,
                    spot.y,
                    width,
                    threats.len(),
                    target.x,
                    target.y,
                    carry.x
                );
            }
            if carry.len() > 0.0 {
                let want = (target - me).scale(2.2) + carry;
                let vm = sim.agents[i].vmax;
                sim.steer(i, if want.len() > vm { want.norm().scale(vm) } else { want });
            } else {
                sim.go_to(i, target, 1.0);
            }
            // Keep facing the quarterback while dropping, unless he has
            // turned to run with a vertical.
            // A zone defender keeps his eyes on the quarterback, even when he
            // turns to run (he looks over his shoulder).
            sim.agents[i].face = (q - me).norm();
            sim.agents[i].face_lock = t + 1.5 * DT;
        }
        _ => {}
    }
}

enum Catch {
    Complete(f64),
    Incomplete { drop: bool, pbu: Option<u32>, sep: f64 },
    Intercepted(usize),
}

/// A defender's hands on a ball he can play, read half against his level
/// (A10): the throws at a lower level are already worse, so a high school
/// ball hawk is a ball hawk among high school passers. Read raw, high
/// school defenses picked off 1.5% of throws against the NFL's 2.3%, where
/// real high school passers throw more interceptions, not fewer.
fn ball_hands(sim: &Sim, d: usize) -> f64 {
    use super::rules::trait_norm;
    let gap = trait_norm(crate::player::Level::NFL, Attr::Catching) - trait_norm(sim.level, Attr::Catching);
    sim.agents[d].a(Attr::Catching) + HANDS_LEVEL * gap
}

/// The share of the level gap a defender's hands are read against (A10).
const HANDS_LEVEL: f64 = 0.5;

/// The ball arrives (topic 1): who can get to it, and who catches it.
#[allow(clippy::too_many_arguments)]
fn catch_point(
    sim: &mut Sim,
    r: usize,
    to: V2,
    from: V2,
    release: f64,
    touch: f64,
    s: &Snap,
    trail: &Trails,
    high: (f64, bool),
) -> Catch {
    let (ball_h, jump_ball) = high;
    // Through the lane: a short, flat ball passes defenders on its way. One
    // with his eyes on the quarterback can get a hand to it, and now and
    // then he catches it (undercut routes, tipped balls).
    let len = from.dist(to);
    let flight = (sim.t - release).max(0.1);
    for d in 0..sim.agents.len() {
        let a = &sim.agents[d];
        if a.side != 1 || !a.up(sim.t) {
            continue;
        }
        for step in 3..17 {
            let k = step as f64 * 0.05;
            // A long ball is above everyone but at its two ends.
            if len > 16.0 && k > 0.15 && k < 0.85 {
                continue;
            }
            let tk = release + k * flight;
            let idx = ((tk / DT).round() as usize).min(trail.ticks.saturating_sub(1));
            if idx >= trail.ticks {
                continue;
            }
            let dp = trail.at(idx, d).0;
            let bp = from + (to - from).scale(k);
            let gap = dp.dist(bp);
            if gap > 1.1 || bp.dist(from) < 2.5 || bp.dist(to) < 1.5 {
                continue;
            }
            let eyes = a.face.dot((from - dp).norm()) > 0.2;
            let cov = a.a(Attr::ZoneCoverage).max(a.a(Attr::ManCoverage));
            let p_touch = 0.6 * (1.0 - gap / 1.1) * (0.5 + cov / 200.0) * if eyes { 1.0 } else { 0.35 };
            if sim.rng.chance(p_touch) {
                let id = a.id;
                if sim.rng.chance(0.3 * ball_hands(sim, d) / 70.0) {
                    return Catch::Intercepted(d);
                }
                let sep = sim.agents[r].pos.dist(dp);
                return Catch::Incomplete { drop: false, pbu: Some(id), sep };
            }
            break;
        }
    }
    let ra = sim.agents[r].clone();
    let gp_h = sim.player(r).height;
    let gp_arm = sim.player(r).arm;
    let dr = ra.pos.dist(to);
    // His catch radius: frame, jump and reach, and body control to adjust
    // to a ball off his frame (proposed, not confirmed).
    // His jump and body control count on the passer's scale (`pro`, A7):
    // below the pro range the gap narrows, as nothing on the other side of
    // this test (the coverage) is read the same way. A high school
    // receiver's radius was a third smaller than a pro's, and 41% of high
    // school throws missed their man (22% in the NFL).
    // Height and arm length (minor bug 275): 0.0075 yard an inch and 0.01
    // within the level, about his level's typical man at his position,
    // whose radius stays what it was (0.015 and 0.02 an inch from 6'0 and
    // 32 inches).
    let (typ_h, _, typ_arm) = super::synthetic::typical_body(sim.level, sim.player(r).position);
    let radius = 0.95
        + 0.015 * (typ_h - 72.0)
        + 0.02 * (typ_arm - 32.0)
        + 0.0075 * (gp_h - typ_h)
        + 0.004 * (pro(ra.a(Attr::Jumping)) - 50.0)
        + 0.01 * (gp_arm - typ_arm)
        + 0.0015 * (pro(ra.a(Attr::BodyControl)) - 60.0);
    // His high point (D1): a ball put over his reach sails past him.
    let rec_reach = high_reach(sim, r, true);
    // Nearest defender to the ball.
    let mut dd = f64::INFINITY;
    let mut dbest = None;
    for d in 0..sim.agents.len() {
        let a = &sim.agents[d];
        if a.side != 1 || !a.up(sim.t) {
            continue;
        }
        let x = a.pos.dist(to);
        if x < dd {
            dd = x;
            dbest = Some(d);
        }
    }
    let sep = dbest.map(|d| sim.agents[d].pos.dist(ra.pos)).unwrap_or(10.0);
    let air = to.x - s.los;
    if super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed) {
        println!(
            "CATCH t={:.1} to ({:.1},{:.1}) rec ({:.1},{:.1}) dr {:.2} radius {:.2} nearest def {:.2} throw dist {:.1}",
            sim.t,
            to.x,
            to.y,
            ra.pos.x,
            ra.pos.y,
            dr,
            radius,
            dd,
            from.dist(to)
        );
    }
    // An open man can stop, come back or lay out for a loose ball; a
    // covered one can't, or the defender gets there first.
    let reach = 1.3 + 0.4 * (dd - 1.5).clamp(0.0, 4.0);
    // Past the end line is out of bounds too (it used to be caught there,
    // and red zone passes went for touchdowns half again as often as real).
    let over_him = jump_ball && ball_h > rec_reach + 2.0;
    if to.y < -0.5 || to.y > width() + 0.5 || to.x > end_line() + 0.5 || dr > radius + reach || over_him {
        // Off target. A defender in the way may still pick it.
        if let Some(d) = dbest {
            let da = &sim.agents[d];
            let rd = 1.0 + 0.01 * (da.a(Attr::Jumping) - 50.0);
            if dd < rd * 0.8 && sim.rng.chance(0.18 * ball_hands(sim, d) / 70.0) {
                return Catch::Intercepted(d);
            }
        }
        return Catch::Incomplete { drop: false, pbu: None, sep };
    }
    // Contest at the catch point.
    let (mut va_all, mut hp_r_all) = (0.0, 0.0);
    if let Some(d) = dbest {
        let da = sim.agents[d].clone();
        let reacted = sim.t >= da.ball_react;
        // A route jumper who broke on the throw runs through the catch
        // point at speed, square to the ball: more reach, and a real chance
        // to pick it (proposed, not confirmed).
        let broke = sim.notes.tend.jump == Some((da.id, true));
        // His reach to the ball: height, jump and arms (D1: the arms were
        // left out, so a long corner played like a short-armed one).
        // Within the level the slope is D_REACH_HEIGHT and D_REACH_ARM
        // (minor bug 274); his level's typical man at his position reaches
        // what he did (0.012 and 0.02 an inch from 6'0 and 31.5 inches).
        let (typ_h, _, typ_arm) = super::synthetic::typical_body(sim.level, sim.player(d).position);
        let d_reach = 1.05
            + 0.012 * (typ_h - 72.0)
            + 0.02 * (typ_arm - 31.5)
            + D_REACH_HEIGHT * (sim.player(d).height - typ_h)
            + 0.004 * (da.a(Attr::Jumping) - 50.0)
            + D_REACH_ARM * (sim.player(d).arm - typ_arm)
            + if broke { 0.6 } else { 0.0 };
        // Size at the catch point (D1): each man's highest reach is his
        // standing reach plus the jump he has left at his speed, against
        // how high the ball arrives. On a ball at the chest it hardly
        // matters; on a ball up high the longer man wins it more often,
        // and a ball over the defender's reach is one he can't play.
        let sees_ball = da.face.dot((from - da.pos).norm()) > 0.0;
        let def_reach = high_reach(sim, d, sees_ball || broke);
        let up_high = clamp((ball_h - 72.0) / 30.0, 0.0, 1.0);
        let va = clamp((rec_reach - def_reach) / 10.0, -1.5, 1.5) * up_high;
        let over_def = ball_h > def_reach + 2.0;
        // Highpointer on both sides: the timing at the top (five contested
        // catch points, only on a ball up high).
        let hp_d = sim.player(d).links.edge(&[Link::Highpointer]) * up_high;
        let hp_r = sim.player(r).links.edge(&[Link::Highpointer]) * up_high;
        let size_on_def = (-0.5 * va).exp() * if over_def { 0.3 } else { 1.0 };
        va_all = va;
        hp_r_all = hp_r;
        if dd < d_reach && reacted {
            // His coverage skill at the catch point is his job's (Man or
            // Zone), `BREAKUP_COVER` a point about his level's typical
            // starter at his position (minor bug 283: it was the better of
            // the two at 0.004 from 60).
            let which = if matches!(da.job, Job::Man { .. }) { Attr::ManCoverage } else { Attr::ZoneCoverage };
            let s_cov = super::synthetic::typical_starter(sim.level, sim.player(d).position, which);
            let cov_term = 0.004 * (s_cov - 60.0) + BREAKUP_COVER * (da.a(which) - s_cov);
            let closeness = 1.0 - dd / d_reach;
            // He's in front of it: an interception chance.
            if dd < dr - 0.15 {
                // Playing the ball: a facing defender has a real chance; one
                // trailing with his back turned mostly plays through the hands.
                let p_int = clamp(
                    FRONT_PICK
                        * closeness
                        * (0.45 + ball_hands(sim, d) / 110.0)
                        * if sees_ball || broke { 1.0 } else { 0.45 }
                        * if broke { 3.5 } else { 1.0 }
                        * size_on_def,
                    0.0,
                    0.7,
                );
                if sim.rng.chance(p_int) {
                    return Catch::Intercepted(d);
                }
            }
            // His hands at the catch point: coverage gets him there, his
            // contested-catch skill and jump get the ball out (proposed, not
            // confirmed). 0.39 from 0.42 since the hit at the catch below
            // takes its share, so the league's breakups hold.
            let p_pbu = clamp(
                closeness
                    * (0.39
                        + cov_term
                        + 0.003 * (da.a(Attr::ContestedCatch) + hp_d - 60.0)
                        + 0.003 * (da.a(Attr::Jumping) - 60.0))
                    * size_on_def,
                0.0,
                0.75,
            );
            if sim.rng.chance(p_pbu) {
                pi_check(sim, d, r, to, air, true);
                if sim.rng.chance(PBU_PICK) {
                    return Catch::Intercepted(d);
                }
                return Catch::Incomplete { drop: false, pbu: Some(da.id), sep };
            }
            // His hit as the ball arrives (S-C): it can knock it loose.
            let p_hit = hit_at_catch(sim, d, r, to, from, closeness);
            if p_hit > 0.0 && sim.rng.chance(p_hit) {
                // A hit on a defenseless receiver can draw a flag, read
                // from his Discipline like the other personal fouls.
                let disc = sim.trait20(d, Attr::Discipline);
                let wild = (1.0 + 0.7 * sim.lean(d, Tend::BigHit)).max(0.2);
                if sim.rng.chance(0.05 * (1.35 - disc / 20.0).max(0.2) * wild) {
                    sim.notes.fouls.push(Foul {
                        kind: FoulKind::UnnecessaryRoughness,
                        side: 1,
                        player: da.id,
                        at: to.x,
                    });
                }
                return Catch::Incomplete { drop: false, pbu: Some(da.id), sep };
            }
        }
        if dd < 1.4 {
            pi_check(sim, d, r, to, air, false);
        }
    }
    // The catch itself.
    let c = ra.a(Attr::Catching);
    let mut p = 1.0 - (0.15 - 0.00095 * c).max(0.012);
    let reach = dr / radius;
    // Inside his frame it is routine; reaching away from his body or
    // diving costs more the farther it is. (Gentler than it was: 28% of NFL
    // throws and 40% of FBS throws ended off target or out of reach,
    // against about 18-20% real off-target throws; proposed, not
    // confirmed.)
    // Body control keeps a ball away from his frame in his hands.
    if reach > 0.9 {
        // Length (Dynamic Links): the catch window, five body control
        // points on a ball away from his frame.
        let bc = ra.a(Attr::BodyControl) + sim.player(r).links.edge(&[Link::Length]);
        let adjust = clamp(1.0 - 0.005 * (bc - 60.0), 0.75, 1.25);
        p *= 1.0 - 0.6 * adjust * clamp((reach - 0.9) / 1.6, 0.0, 1.0);
    }
    // Catching in a crowd is its own skill: contested catches, with the
    // strength and jump to win the ball.
    if dd < 1.4 {
        // The size edge at the top (D1) counts like eighteen points of the
        // fight per ten inches of reach, on a ball up high.
        let fight = 0.5 * (ra.a(Attr::ContestedCatch) + hp_r_all)
            + 0.25 * ra.a(Attr::Strength)
            + 0.25 * ra.a(Attr::Jumping)
            + 18.0 * va_all;
        p *= 1.0 - 0.32 * (1.0 - dd / 1.4) * (1.25 - fight / 100.0);
        // Over the middle with a hitter arriving, one who avoids contact
        // short-arms it (tendencies: contact willingness).
        let middle = (to.y - mid()).abs() < 9.0;
        if middle {
            p *= 1.0 - 0.12 * (1.0 - dd / 1.4) * (-sim.lean(r, Tend::Contact)).max(0.0);
        }
    }
    if s.cond.rain || s.cond.snow {
        p *= 0.97;
    }
    p *= 1.0 - 0.03 * s.cond.cold_unused;
    let speed = from.dist(to) / (sim.t - release).max(0.2);
    // A hard ball on a short throw; a passer with touch takes something off
    // it (proposed, not confirmed).
    if speed > 24.0 && dr < 1.0 {
        p *= 1.0 - 0.015 * clamp(1.0 - 0.02 * (touch - 60.0), 0.4, 1.6);
    }
    if sim.rng.chance(p) {
        Catch::Complete(sep)
    } else {
        // A ball off his hands can pop up to a defender close by.
        if let Some(d) = dbest
            && dd < 3.0
            && sim.rng.chance(TIP_PICK * (1.0 - dd / 3.0) * (0.5 + ball_hands(sim, d) / 100.0))
        {
            return Catch::Intercepted(d);
        }
        let drop = reach < 0.8 && dd > 1.0;
        Catch::Incomplete { drop, pbu: None, sep }
    }
}

/// The chance a defender's hit at the catch knocks the ball loose (safety
/// proposal C, Cain's yes 2026-11-07; proposed, not confirmed in its
/// sizes): he has to arrive with the ball and closing fast; his Tackling,
/// Physicality, Strength and Big Hit habit, with a small part from his
/// weight over his position's typical man, against the receiver's
/// Contested Catch, Strength and hands and his contact habit. Mostly over
/// the middle and on a throw he saw coming; at most one in five. A little
/// comes off the hands breakups (`p_pbu`), so the league's breakups and
/// completions hold.
fn hit_at_catch(sim: &Sim, d: usize, r: usize, to: V2, from: V2, closeness: f64) -> f64 {
    let (da, ra) = (&sim.agents[d], &sim.agents[r]);
    let toward = (ra.pos - da.pos).norm();
    let closing = (da.vel.dot(toward) - ra.vel.dot(toward)).max(0.0);
    let speed = clamp((closing - 2.0) / 5.0, 0.0, 1.0);
    if speed <= 0.0 {
        return 0.0;
    }
    let power = 0.35 * da.a(Attr::Tackling)
        + 0.25 * da.a(Attr::Physicality)
        + 0.2 * da.a(Attr::Strength)
        + 0.2 * da.a(Attr::Pursuit);
    let hold = 0.5 * ra.a(Attr::ContestedCatch) + 0.25 * ra.a(Attr::Strength) + 0.25 * ra.a(Attr::Catching);
    let g = sim.player(d);
    let tw = super::synthetic::typical_body(sim.level, g.position).1;
    let mass = clamp(1.0 + 0.004 * (g.weight - tw), 0.8, 1.25);
    let style = (1.0 + 0.6 * sim.lean(d, Tend::BigHit) - 0.3 * sim.lean(r, Tend::Contact)).max(0.3);
    let middle = if (to.y - mid()).abs() < 9.0 { 1.0 } else { 0.35 };
    let sees = if da.face.dot((from - da.pos).norm()) > 0.0 { 1.0 } else { 0.6 };
    let p = closeness * speed * clamp(1.0 + 0.02 * (power - hold), 0.4, 1.8) * mass * style * middle * sees;
    p.min(0.2)
}

/// Pass interference by who draws it (C3, bug 299): real corners draw it
/// on 0.23% of their snaps (the engine's 0.07%), and the league about 0.42
/// a team game (the engine's 0.23), so corners three times and everyone
/// else in coverage half: corners land at about 0.21% and the league at
/// 0.39-0.40 (proposed, not confirmed).
fn pi_level(sim: &Sim, d: usize) -> f64 {
    if sim.player(d).position == crate::positions::Position::CB { 3.0 } else { 0.5 }
}

/// A defender's body in the hand fight downfield (corner proposal C3,
/// bug 301): real NFL corners 2018-2025 at the same draft slot hold 12%
/// more an inch of arm and 17% more per 10 lb, with the depth of target
/// held; about his level's typical man at his position, so the league's
/// total stays where Discipline and the crew put it.
fn hold_body(sim: &Sim, d: usize) -> f64 {
    let g = sim.player(d);
    let (_, typ_w, typ_arm) = super::synthetic::typical_body(sim.level, g.position);
    clamp((1.0 + HOLD_ARM * (g.arm - typ_arm)) * (1.0 + HOLD_WEIGHT * (g.weight - typ_w)), 0.5, 1.8)
}
const HOLD_ARM: f64 = 0.12;
const HOLD_WEIGHT: f64 = 0.017;
/// Pass interference rises 6% an inch of a corner's height (C3, the same
/// real corners, with the depth of target held: the tall man fights the
/// receiver's frame on the ball he can reach).
const PI_HEIGHT: f64 = 0.06;

/// Pass interference: a beaten defender who plays the man, not the ball.
fn pi_check(sim: &mut Sim, d: usize, r: usize, to: V2, air: f64, broke_up: bool) {
    if air < 4.0 {
        return;
    }
    let (da, ra) = (&sim.agents[d], &sim.agents[r]);
    let trailing = (to - da.pos).dot(to - ra.pos) > 0.0 && da.pos.dist(to) > ra.pos.dist(to);
    let facing = da.face.dot((sim.ball_pos() - da.pos).norm()) > 0.3;
    let disc = sim.trait20(d, Attr::Discipline);
    // Half again what it was: pass interference ran 0.27 a team game, real
    // about 0.42, offensive 0.05 against 0.17 (proposed, not confirmed).
    let base = if trailing && !facing {
        0.8
    } else if trailing {
        0.33
    } else {
        0.075
    };
    let (typ_h, _, _) = super::synthetic::typical_body(sim.level, sim.player(d).position);
    let tall = clamp(1.0 + PI_HEIGHT * (sim.player(d).height - typ_h), 0.6, 1.6);
    let p = base
        * (1.35 - disc / 20.0)
        * sim.crew.pass_interference
        * if broke_up { 1.2 } else { 0.7 }
        * (0.6 + air / 40.0)
        * tall
        * pi_level(sim, d);
    if sim.rng.chance(p) {
        let id = da.id;
        sim.notes.fouls.push(Foul { kind: FoulKind::PassInterference, side: 1, player: id, at: to.x });
    } else if sim.rng.chance(0.029 * (1.3 - sim.trait20(r, Attr::Discipline) / 20.0) * sim.crew.pass_interference) {
        let id = ra.id;
        sim.notes.fouls.push(Foul { kind: FoulKind::OffensivePassInterference, side: 0, player: id, at: to.x });
    }
}
