//! One snap from scrimmage: alignment, assignments, then the tracked play
//! (topics 1-3). The offense goes toward +x from `los`.

use super::geom::{DT, MID, V2, WIDTH, v2};

/// The back of the end zone the offense attacks.
const END_LINE: f64 = 110.0;
use super::playbook::*;
use super::roster::GamePlayer;
use super::rules::Rules;
use super::sim::*;
use crate::attributes::Attr;
use crate::rng::{Rng, clamp};

/// Defensive spots. "L" is the offense's left (higher y).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
        ORole::X | ORole::Z | ORole::Slot | ORole::Slot2 => "WR",
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
    /// The offense needs the clock stopped (two-minute drill): runners get
    /// out of bounds and receivers work the sideline.
    pub hurry: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug)]
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
    pub yac: f64,
    pub before_contact: f64,
    pub time_to_throw: f64,
    pub separation: f64,
    pub throw_away: bool,
    pub drop: bool,
    pub pressured: bool,
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
        let gp = &s.teams[0][p];
        let pos = align(&f, r, los, by);
        agents.push(Agent::new(0, p, gp, orole_label(r), pos, s.energy[0][p], s.edge[0][p]));
        role_of.push(Some(r));
        slot_of.push(None);
    }
    let off_n = agents.len();
    let def_spots = align_defense(s, &agents, &role_of);
    for (k, &(d, p)) in s.defense.iter().enumerate() {
        let gp = &s.teams[1][p];
        agents.push(Agent::new(1, p, gp, d.label(), def_spots[k], s.energy[1][p], s.edge[1][p]));
        role_of.push(None);
        slot_of.push(Some(d));
    }
    let mut sim = Sim {
        teams: s.teams,
        agents,
        eng: Vec::new(),
        ball: Ball::Held(0),
        t: 0.0,
        los,
        rng,
        notes: Notes::default(),
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
    };
    let find = |r: ORole| role_of.iter().position(|x| *x == Some(r));
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
        yac: 0.0,
        before_contact: 0.0,
        time_to_throw: 0.0,
        separation: 0.0,
        throw_away: false,
        drop: false,
        pressured: false,
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
        PlayKind::Pass(c) => PASS_CONCEPTS[c].play_action,
        PlayKind::Run(RunConcept::Draw) => true,
        _ => false,
    };
    res.play_action = fake && matches!(s.call.kind, PlayKind::Pass(_));
    for i in off_n..sim.agents.len() {
        let pr = sim.agents[i].a(Attr::PlayRecognition);
        let base = 0.22 + 0.0075 * (100.0 - pr) + 0.12 * sim.rng.next();
        let extra = if fake { 0.15 + 0.012 * (100.0 - pr) * sim.rng.next() } else { 0.0 };
        sim.agents[i].read_at = base + extra;
    }

    // Busted assignments (topic 3): players who don't know the playbook.
    for i in 0..sim.agents.len() {
        let gp = sim.player(i);
        let iq = gp.a(Attr::Awareness);
        let know = gp.knowledge;
        let p = (1.0 - know) * 0.05 + (60.0 - iq).max(0.0) * 0.0006;
        if sim.rng.chance(p) {
            sim.agents[i].bust = true;
            let id = sim.agents[i].id;
            sim.notes.busts.push(id);
        }
    }

    let _carrier_rb = find(ORole::RB);
    let mut pass: Option<Pass> = None;
    let mut handoff_done = false;
    let mut aim: Option<V2> = None;
    let mut lead_puller: Option<usize> = None;
    let mut zone_read_keep = false;
    let mut throw_info: Option<(V2, usize, f64, f64, f64)> = None;

    // ----------------------------------------------------------- assignments
    match s.call.kind {
        PlayKind::Pass(ci) => {
            let c = &PASS_CONCEPTS[ci];
            res.passer = Some(sim.agents[qb].id);
            for i in 0..off_n {
                let r = role_of[i].unwrap();
                if r == ORole::QB {
                    sim.agents[i].job = Job::Passer;
                    continue;
                }
                let route = route_for(c, r);
                sim.agents[i].job = route_job(&sim.agents[i], route, los, by, s.hurry);
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
                // a few tenths): about 1.1 s quick, 1.6 s five-step, 2.1 s seven.
                Drop::Quick => 1.05,
                Drop::Medium => 1.6,
                Drop::Deep => 2.1,
                Drop::Boot => 2.2,
                Drop::Screen => 1.4,
                Drop::Bubble => 0.45,
            } + if c.play_action { 0.35 } else { 0.0 };
            let reads = c.reads.iter().filter_map(|r| find(*r)).collect::<Vec<_>>();
            pass = Some(Pass {
                reads,
                read: 0,
                read_since: set_at - 0.2,
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
            });
            if c.drop == Drop::Screen {
                sim.screen = pass.as_ref().and_then(|ps| ps.reads.first().copied());
            }
            assign_defense(&mut sim, s, off_n, &slot_of, &role_of, true);
            pass_protection(&mut sim, off_n);
        }
        PlayKind::Run(rc) => {
            let dir = s.call.dir;
            let (aim_y, pulls) = rc.aim();
            let aim_pt = v2(los + 1.0, by + aim_y * dir);
            aim = Some(aim_pt);
            let mesh = match (rc, f.backfield) {
                (RunConcept::Sneak, _) => sim.agents[qb].pos,
                (_, Backfield::UnderCenter) => v2(los - 4.0, by + 0.6 * dir),
                (_, Backfield::Pistol) => v2(los - 4.6, by + 0.5 * dir),
                (_, Backfield::Shotgun) => v2(los - 5.2, by + 0.4 * dir),
            };
            let mesh_t = match rc {
                RunConcept::Sneak => 0.0,
                RunConcept::Draw => 1.1,
                RunConcept::Toss => 0.45,
                // Gap runs wait for the pullers (the counter step).
                RunConcept::Power => {
                    if f.backfield == Backfield::UnderCenter {
                        0.95
                    } else {
                        0.8
                    }
                }
                RunConcept::Counter => {
                    if f.backfield == Backfield::UnderCenter {
                        1.15
                    } else {
                        1.0
                    }
                }
                _ => match f.backfield {
                    Backfield::UnderCenter => 0.8,
                    _ => 0.6,
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
            if rc == RunConcept::Sneak {
                sim.agents[qb].job = Job::Carrier;
                handoff_done = true;
                res.carrier = Some(sim.agents[qb].id);
            }
            assign_defense(&mut sim, s, off_n, &slot_of, &role_of, false);
            run_blocking(&mut sim, off_n, &role_of, rc, dir, pulls);
            // On a gap run the back follows the lead puller to the hole.
            lead_puller = (0..off_n).find(|&i| {
                matches!(sim.agents[i].job, Job::RunBlock { pull: true, .. }) && role_of[i].is_some_and(|r| r.is_line())
            });
        }
        _ => {}
    }

    // ------------------------------------------------------------- the play
    let max_t = 14.0;
    let mut trail: Vec<Vec<V2>> = vec![Vec::with_capacity(140); sim.agents.len()];
    let mut vel_trail: Vec<Vec<V2>> = vec![Vec::with_capacity(140); sim.agents.len()];
    let mut throw_start: Option<f64> = None;
    let mut catch_x = los;
    let mut thrown_from = los;
    sim.record_frame();
    while sim.t < max_t && !sim.dead {
        sim.t += DT;
        let t = sim.t;
        for i in 0..sim.agents.len() {
            trail[i].push(sim.agents[i].pos);
            vel_trail[i].push(sim.agents[i].vel);
        }
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
                let d = sim.agents[i].pos.dist(to);
                let left = (arrive - t).max(DT);
                let speed = (d / left).min(sim.agents[i].vmax);
                let dir = (to - sim.agents[i].pos).norm();
                sim.steer(i, dir.scale(speed));
                continue;
            }
            let job = sim.agents[i].job.clone();
            match job {
                Job::Passer => {}
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
                    if t < at && d > 0.3 {
                        let speed = (d / (at - t).max(DT)).min(sim.agents[i].vmax * 0.7);
                        sim.steer(i, (mesh - me).norm().scale(speed));
                    } else {
                        sim.steer(i, (hole - me).norm().scale(sim.agents[i].vmax * 0.55));
                    }
                    if t >= at && d < 1.3 && !handoff_done {
                        handoff_done = true;
                        // Zone read: the quarterback reads the backside end.
                        let rc = if let PlayKind::Run(rc) = s.call.kind { rc } else { RunConcept::InsideZone };
                        if rc == RunConcept::ZoneRead {
                            zone_read_keep = read_option(&mut sim, qb, s.call.dir, off_n);
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
                            // A rare bad exchange.
                            let carry = sim.agents[i].a(Attr::Carrying);
                            if sim.rng.chance(0.0025 * (1.6 - carry / 100.0)) {
                                sim.notes.fumbled = Some(sim.agents[i].id);
                                sim.fumble(i);
                            }
                        }
                    }
                }
                Job::Route { .. } => run_route(&mut sim, i, &pass, qb, s, &trail),
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
                            if me.dist(dp) < 1.5 {
                                sim.engage(i, d, true);
                            }
                        }
                        None => {
                            // Find the most dangerous free rusher near him, or help.
                            let me = sim.agents[i].pos;
                            let threat = (off_n..sim.agents.len())
                                .filter(|&d| {
                                    let a = &sim.agents[d];
                                    a.up(t)
                                        && matches!(a.job, Job::Rush { .. })
                                        && a.engaged.is_none()
                                        && a.pos.dist(me) < 4.5
                                })
                                .min_by(|&a, &b| sim.agents[a].pos.dist(q).total_cmp(&sim.agents[b].pos.dist(q)));
                            if let Some(d) = threat {
                                sim.agents[i].job = Job::PassBlock { target: Some(d) };
                            } else {
                                // Help the nearest teammate's block.
                                let help = sim
                                    .eng
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, e)| !e.over && e.pass && e.helper.is_none())
                                    .map(|(k, e)| (k, sim.agents[e.defender].pos.dist(me)))
                                    .filter(|&(_, d)| d < 2.5)
                                    .min_by(|a, b| a.1.total_cmp(&b.1));
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
                    match target.filter(|&d| sim.agents[d].up(t) && sim.agents[d].engaged.is_none()) {
                        Some(d) => {
                            let dp = sim.agents[d].pos;
                            // Get between him and the ball.
                            let ball = c.map(|c| sim.agents[c].pos).unwrap_or(v2(los - 3.0, by));
                            let spot = dp + (ball - dp).norm().scale(0.8) + sim.agents[d].vel.scale(0.25);
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
                            if (sim.agents[i].pos.dist(dp) < reach || sim.closest(i, d) < reach)
                                && sim.lands_block(i, d)
                            {
                                sim.engage(i, d, false);
                                sim.agents[i].job = Job::RunBlock { target: None, pull: false };
                            }
                        }
                        None => {
                            // Climb to the nearest unblocked defender ahead of the ball.
                            let me = sim.agents[i].pos;
                            let ballx = c.map(|c| sim.agents[c].pos.x).unwrap_or(los);
                            let near = (off_n..sim.agents.len())
                                .filter(|&d| {
                                    let a = &sim.agents[d];
                                    a.up(t) && a.engaged.is_none() && a.pos.dist(me) < 6.0 && a.pos.x > ballx - 1.0
                                })
                                .min_by(|&a, &b| sim.agents[a].pos.dist(me).total_cmp(&sim.agents[b].pos.dist(me)));
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

        // ---- defense
        for i in off_n..sim.agents.len() {
            if !sim.agents[i].up(t) || sim.agents[i].engaged.is_some() {
                continue;
            }
            defend(&mut sim, i, s, qb, &pass, &trail, &vel_trail, in_air);
        }

        // ---- ball carrier
        if let Some(c) = sim.carrier() {
            let agent_job = sim.agents[c].job.clone();
            if agent_job == Job::Carrier {
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
                // A quarterback slides rather than take a hit he doesn't need.
                if pass.is_some() && c == qb && slide(&mut sim, c, s) {
                    res.result = Result::Scramble;
                }
            }
        }

        sim.advance();

        // ---- the ball in the air
        if let Ball::Air { to, arrive, target, release, from, .. } = sim.ball
            && t + 1e-9 >= arrive
        {
            let tgt = target.unwrap_or(qb);
            let outcome = catch_point(&mut sim, tgt, to, from, release, s, &trail);
            res.target = Some(sim.agents[tgt].id);
            catch_x = to.x;
            res.air_yards = to.x - los;
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
                    // A catch at the sideline needs his feet in (topic 5).
                    if to.y < 0.6 || to.y > WIDTH - 0.6 {
                        let feet = s.rules.feet_inbounds as f64;
                        let aw = sim.agents[tgt].a(Attr::Awareness);
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
        if ((sim.t * 10.0).round() as i64) % 5 == 0 {
            sim.trace();
        }
    }
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
    if res.throw_away {
        res.result = Result::Incomplete;
    }
    let goal_line = 100.0;
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
    for a in &sim.agents {
        res.work.push((a.side, a.p, a.work));
        sim.notes.top_speed.push((a.id, a.top));
    }
    res.notes = std::mem::take(&mut sim.notes);
    res.frames = sim.record.take();
    res
}

// ------------------------------------------------------------- alignment

fn align_defense(s: &Snap, agents: &[Agent], roles: &[Option<ORole>]) -> Vec<V2> {
    let los = s.los;
    let by = s.ball_y;
    let st = s.call.formation.strength;
    let find = |r: ORole| roles.iter().position(|x| *x == Some(r)).map(|i| agents[i].pos);
    let lt = find(ORole::LT).unwrap_or(v2(los, by + 2.8));
    let rt = find(ORole::RT).unwrap_or(v2(los, by - 2.8));
    let (strong_t, weak_t) = if st > 0.0 { (lt, rt) } else { (rt, lt) };
    let te_strong = [ORole::TE, ORole::TE2, ORole::TE3].iter().filter_map(|r| find(*r)).find(|p| (p.y - by) * st > 0.0);
    // Receivers by side, outside in.
    let mut left: Vec<V2> = Vec::new();
    let mut right: Vec<V2> = Vec::new();
    for (i, r) in roles.iter().enumerate() {
        if let Some(r) = r
            && matches!(r, ORole::X | ORole::Z | ORole::Slot | ORole::Slot2)
        {
            if agents[i].pos.y > by { left.push(agents[i].pos) } else { right.push(agents[i].pos) }
        }
    }
    left.sort_by(|a, b| b.y.total_cmp(&a.y));
    right.sort_by(|a, b| a.y.total_cmp(&b.y));
    let press =
        matches!(s.dcall.coverage, Coverage::Cover0 | Coverage::Cover1 | Coverage::Cover2Man | Coverage::Cover2);
    let cush = if press { 1.2 } else { 6.5 };
    let wide = |list: &Vec<V2>, k: usize, side: f64| -> V2 {
        match list.get(k) {
            Some(p) => v2(los + cush, p.y - 0.8 * side),
            None => v2(los + 6.0, by + side * (7.0 + 4.0 * k as f64)),
        }
    };
    let two_high = s.dcall.coverage.high() == 2 || s.dcall.disguise;
    let three_four =
        s.defense.iter().any(|(d, _)| *d == DSlot::NT) && s.defense.iter().filter(|(d, _)| d.is_line()).count() == 3;
    s.defense
        .iter()
        .map(|(d, _)| {
            let dl = los + 0.9;
            match d {
                DSlot::LE | DSlot::RE => {
                    let side = if *d == DSlot::LE { 1.0 } else { -1.0 };
                    let strong = side * st > 0.0;
                    let t = if strong { strong_t } else { weak_t };
                    let y = match (strong, te_strong) {
                        (true, Some(te)) => te.y + side * 1.0,
                        _ => t.y + side * if three_four { 2.2 } else { 1.4 },
                    };
                    v2(dl, y)
                }
                DSlot::DTL | DSlot::DTR => {
                    let side = if *d == DSlot::DTL { 1.0 } else { -1.0 };
                    let strong = side * st > 0.0;
                    if three_four {
                        v2(dl, by + side * 2.6)
                    } else {
                        v2(dl, by + side * if strong { 2.0 } else { 0.7 })
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
                        Some(p) => v2(los + 4.0, p.y - 0.8 * side),
                        None => v2(los + 5.0, by + side * 5.5),
                    }
                }
                DSlot::DCB => {
                    let (list, side) = if st > 0.0 { (&right, -1.0) } else { (&left, 1.0) };
                    match list.get(1) {
                        Some(p) => v2(los + 4.0, p.y - 0.8 * side),
                        None => v2(los + 6.0, by + side * 5.5),
                    }
                }
                DSlot::FS => {
                    if two_high {
                        v2(los + 12.5, MID - 9.0 * st)
                    } else {
                        v2(los + 13.0, by)
                    }
                }
                DSlot::SS => {
                    if two_high {
                        v2(los + 12.5, MID + 9.0 * st)
                    } else {
                        v2(los + 7.5, by + st * 4.5)
                    }
                }
            }
        })
        .map(|p: V2| {
            // Near the goal line the defense tightens: corners and
            // linebackers come up, safeties sit at the back of the end zone.
            let room = END_LINE - los;
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
    let (pts, settle) = match route {
        Route::CheckRelease => (vec![a.pos + v2(1.5, 3.5 * out), a.pos + v2(4.0, 5.0 * out)], true),
        Route::Screen => (vec![a.pos + v2(0.5, 4.0 * out), a.pos + v2(0.5, 7.0 * out)], true),
        r => r.points(a.pos, los, out),
    };
    let delay = match route {
        Route::CheckRelease => 1.0,
        Route::Screen => 0.9,
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
        Pressure::LbBlitz | Pressure::Simulated => {
            vec![if slots.contains(&Some(DSlot::Will)) { DSlot::Will } else { DSlot::Mike }]
        }
        Pressure::DbBlitz => vec![if slots.contains(&Some(DSlot::NCB)) { DSlot::NCB } else { DSlot::SS }],
        Pressure::DoubleA => {
            vec![DSlot::Mike, if slots.contains(&Some(DSlot::Will)) { DSlot::Will } else { DSlot::Sam }]
        }
    };
    if cov == Coverage::Cover0 {
        blitzers.push(DSlot::FS);
        blitzers.push(DSlot::Mike);
    }
    blitzers.retain(|b| slots.contains(&Some(*b)));
    // In simulated pressure the weak end drops into the flat.
    let dropper =
        if s.dcall.pressure == Pressure::Simulated { Some(if st > 0.0 { DSlot::RE } else { DSlot::LE }) } else { None };

    // Near the goal the field shrinks: deep zones sit at the back of the end
    // zone and the underneath ones squeeze down with them.
    let zone = |spot: V2, deep: bool, width: f64| {
        let room = 108.0 - los;
        let x = if deep { spot.x.min(108.0) } else { spot.x.min(los + 0.55 * room) };
        Job::Zone { spot: v2(x, spot.y), deep, width }
    };
    let side_y = |side: f64, from_sideline: f64| if side > 0.0 { WIDTH - from_sideline } else { from_sideline };
    let deep_third = |side: f64| zone(v2(los + 16.0, side_y(side, 9.5)), true, 14.0);
    let deep_mid = zone(v2(los + 17.0, by * 0.5 + MID * 0.5), true, 18.0);
    let deep_half = |side: f64| zone(v2(los + 16.0, MID + side * 9.5), true, 26.0);
    let quarter_out = |side: f64| zone(v2(los + 13.0, side_y(side, 9.0)), true, 13.0);
    let quarter_in = |side: f64| zone(v2(los + 13.0, MID + side * 4.5), true, 13.0);
    let curl_flat = |side: f64| zone(v2(los + 9.0, by + side * 10.0), false, 12.0);
    let flat = |side: f64| zone(v2(los + 4.0, side_y(side, 9.0)), false, 10.0);
    let hook = |side: f64| zone(v2(los + 10.0, by + side * 4.5), false, 9.0);
    let middle = zone(v2(los + 10.5, by), false, 9.0);

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
                    let side = if d == DSlot::CBL { 1.0 } else { -1.0 };
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
        } else {
            let lb_side = side_of(sim);
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
            } else if let Job::Zone { spot, deep, width } = sim.agents[i].job.clone() {
                sim.agents[i].job =
                    Job::Zone { spot: v2(spot.x - 5.0, spot.y), deep: !deep && false, width: width * 0.6 };
            }
        }
    }
    // Press at the line (topic 3): a jam battle off the snap.
    for i in off_n..n {
        if let Job::Man { target, .. } = sim.agents[i].job {
            let (d, r) = (&sim.agents[i], &sim.agents[target]);
            if d.pos.x - los < 2.0 {
                let rel = 0.4 * r.a(Attr::RouteRunning) + 0.3 * r.a(Attr::Agility) + 0.3 * r.a(Attr::Strength);
                let jam = 0.5 * d.a(Attr::ManCoverage) + 0.3 * d.a(Attr::Strength) + 0.2 * d.a(Attr::Agility);
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

fn man_job(sim: &Sim, d: usize, r: usize) -> Job {
    let (da, ra) = (&sim.agents[d], &sim.agents[r]);
    // Reaction lag: route running against cover skill (topic 2).
    let lag = clamp(
        0.32 + 0.0065 * (ra.a(Attr::RouteRunning) - da.a(Attr::ManCoverage))
            + 0.003 * (ra.a(Attr::Agility) - da.a(Attr::Agility)),
        0.1,
        0.75,
    );
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
    for &b in &line {
        if pullers.contains(&b) || sim.agents[b].bust {
            continue;
        }
        let y = sim.agents[b].pos.y;
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
    // Pullers kick out or lead on whoever is left at the point of attack.
    for &b in &pullers {
        let poa = sim.agents[0].pos.y + dir * 4.0;
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
            Some(ORole::X | ORole::Z | ORole::Slot | ORole::Slot2) => {
                let me = sim.agents[i].pos;
                let t = (off_n..n)
                    .filter(|&d| !taken[d] && sim.agents[d].pos.dist(me) < 9.0)
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

fn run_route(sim: &mut Sim, i: usize, pass: &Option<Pass>, qb: usize, s: &Snap, trail: &[Vec<V2>]) {
    let t = sim.t;
    let Job::Route { pts, next, settle, release_delay, .. } = sim.agents[i].job.clone() else { return };
    if t < release_delay {
        // A back checks for blitzers before he releases.
        if let Some(r) = blitz_threat(sim, i, qb) {
            sim.agents[i].job = Job::PassBlock { target: Some(r) };
            return;
        }
        let p = sim.agents[i].pos;
        sim.go_to(i, p, 0.3);
        return;
    }
    // The scramble drill (topic 3.1): when the play breaks down, receivers
    // work back to open grass, if the coach lets them and they see it.
    let broken = pass.as_ref().is_some_and(|p| (p.scramble || t > 3.6) && p.thrown.is_none());
    if broken && freelance_ok(sim, i, s.strictness[0], s.trust[0][sim.agents[i].p]) {
        let q = sim.agents[qb].pos;
        let me = sim.agents[i].pos;
        let mut best = (f64::NEG_INFINITY, me);
        for k in 0..8 {
            let cand = v2(
                clamp(me.x + 3.0 * ((k % 3) as f64 - 1.0), s.los + 2.0, s.los + 25.0),
                clamp(me.y + 4.0 * ((k / 3) as f64 - 1.0) + (q.y - me.y) * 0.2, 2.0, WIDTH - 2.0),
            );
            let space = (sim.agents.len() / 2..sim.agents.len())
                .filter(|&d| sim.agents[d].side == 1)
                .map(|d| sim.agents[d].pos.dist(cand))
                .fold(f64::INFINITY, f64::min);
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
    let _ = trail;
    if next < pts.len() {
        let (target, speed, done) = route_step(&sim.agents[i], &pts, next, settle);
        if done && let Job::Route { next: n, .. } = &mut sim.agents[i].job {
            *n += 1;
        }
        let bust = sim.agents[i].bust;
        let target = if bust { v2(target.x + 3.0, target.y + 4.0) } else { target };
        let dir = (target - sim.agents[i].pos).norm();
        sim.steer(i, dir.scale(speed));
    } else if settle {
        // Sit down in space: drift away from the nearest defender.
        let me = sim.agents[i].pos;
        let near = (0..sim.agents.len())
            .filter(|&d| sim.agents[d].side == 1)
            .min_by(|&a, &b| sim.agents[a].pos.dist(me).total_cmp(&sim.agents[b].pos.dist(me)));
        let away = near.map(|d| (me - sim.agents[d].pos).norm().scale(1.5)).unwrap_or_default();
        let q = sim.agents[qb].pos;
        let spot = me + v2(0.0, away.y) + (q - me).norm().scale(0.3);
        sim.go_to(i, spot, 0.3);
    } else {
        let v = sim.agents[i].vel;
        let dir = if v.len() > 0.5 { v.norm() } else { v2(1.0, 0.0) };
        let me = sim.agents[i].pos;
        // Running out of room: he turns up the sideline.
        let dir = if me.y < 4.0 && dir.y < 0.0 || me.y > WIDTH - 4.0 && dir.y > 0.0 { v2(1.0, 0.0) } else { dir };
        let vm = sim.agents[i].vmax;
        sim.steer(i, dir.scale(vm));
    }
}

/// One tick of a route: the point he runs at, how fast, and whether he has
/// reached it (he moves on to the next point).
fn route_step(a: &Agent, pts: &[V2], next: usize, settle: bool) -> (V2, f64, bool) {
    let target = pts[next];
    // Good route runners cut sharper: they start the break later and keep speed.
    let rr = a.a(Attr::RouteRunning);
    let switch = 0.6 + 0.012 * rr;
    let done = a.pos.dist(target) < switch;
    // He keeps a couple of yards of room at the sideline.
    let target = v2(target.x, clamp(target.y, 2.5, WIDTH - 2.5));
    let urg = if next + 1 == pts.len() && settle { 0.85 } else { 1.0 };
    let dist = a.pos.dist(target);
    // Before the break of a route that stops (hitch, curl, comeback) he
    // sinks his hips: he brakes so he can plant at the stem.
    let brake = a.accel * 1.9;
    let speed = if next + 1 == pts.len() && settle {
        a.vmax.min(dist * 2.0)
    } else if settle && next + 2 == pts.len() {
        let stop = (2.0 * brake * 0.9 * dist).sqrt() + 1.0 + 0.03 * (rr - 50.0).max(0.0);
        (a.vmax * urg).min(stop)
    } else if next + 1 < pts.len() {
        // A sharp break needs him to gear down first; good route runners
        // carry more speed through it.
        let into = (target - a.pos).norm();
        let out = (pts[next + 1] - target).norm();
        let turn = into.dot(out).clamp(-1.0, 1.0).acos();
        let corner = a.vmax * (1.0 - (0.62 - 0.004 * (rr - 50.0)) * (turn / 1.57).min(1.4)).max(0.15);
        let limit = (corner * corner + 2.0 * brake * 0.9 * (dist - 0.5).max(0.0)).sqrt();
        (a.vmax * urg).min(limit)
    } else {
        a.vmax * urg
    };
    (target, speed, done)
}

fn freelance_ok(sim: &mut Sim, i: usize, strictness: f64, trust: f64) -> bool {
    let gp = sim.player(i);
    let iq = 0.5 * gp.a(Attr::Awareness) + 0.5 * gp.a(Attr::PlayRecognition);
    let p = clamp(0.25 + 0.008 * (iq - 50.0) + 0.5 * (trust - 0.5) - 0.025 * (strictness - 10.0), 0.05, 0.95);
    // Decide once per play: store in the bust flag's sibling via the random draw.
    let key = (sim.agents[i].id as u64 * 2654435761 + (sim.los * 10.0) as u64) % 1000;
    (key as f64 / 1000.0) < p
}

fn blitz_threat(sim: &Sim, i: usize, qb: usize) -> Option<usize> {
    let q = sim.agents[qb].pos;
    (0..sim.agents.len())
        .filter(|&d| {
            sim.agents[d].side == 1
                && matches!(sim.agents[d].job, Job::Rush { .. })
                && sim.agents[d].engaged.is_none()
                && sim.agents[d].up(sim.t)
        })
        .filter(|&d| sim.agents[d].pos.dist(q) < 6.0 && sim.agents[d].pos.dist(sim.agents[i].pos) < 4.5)
        .filter(|_| sim.agents[i].a(Attr::Awareness) > 25.0)
        .min_by(|&a, &b| sim.agents[a].pos.dist(q).total_cmp(&sim.agents[b].pos.dist(q)))
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
    let threat = (0..sim.agents.len())
        .filter(|&d| sim.agents[d].side != side && sim.agents[d].up(sim.t) && sim.agents[d].engaged.is_none())
        // Before the screen is caught the rushers are let through; the
        // blockers look for the second level.
        .filter(|&d| {
            !pre || (sim.agents[d].smell || !matches!(sim.agents[d].job, Job::Rush { .. }))
                && sim.agents[d].pos.dist(me) < 4.0
        })
        .filter(|&d| {
            let dp = sim.agents[d].pos;
            dp.dist(cp) < 10.0 || (dp.x - cp.x) * sim.goal > 0.0 && dp.dist(cp) < 30.0 && dp.dist(me) < 12.0
        })
        .min_by(|&a, &b| sim.agents[a].pos.dist(me).total_cmp(&sim.agents[b].pos.dist(me)));
    match threat {
        Some(d) if sim.closest(i, d) < 1.3 && sim.lands_block(i, d) => {
            // Blocks from behind are illegal.
            let from_behind =
                (sim.agents[d].pos - me).dot(sim.agents[d].vel.norm()) > 0.5 && sim.agents[d].vel.len() > 3.0;
            if from_behind && sim.rng.chance(0.03 * sim.crew.holding) {
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
    let side = if me.y > MID { 1.0 } else { -1.0 };
    let near = (0..sim.agents.len())
        .filter(|&d| sim.agents[d].side != sim.agents[c].side && sim.agents[d].up(sim.t))
        .map(|d| sim.agents[d].pos.dist(me))
        .fold(f64::INFINITY, f64::min);
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
    let near = (0..sim.agents.len())
        .filter(|&d| sim.agents[d].side == 1 && sim.agents[d].up(sim.t) && sim.agents[d].engaged.is_none())
        .map(|d| sim.agents[d].pos.dist(me))
        .fold(f64::INFINITY, f64::min);
    let past = me.x >= s.los + s.togo;
    let tough = sim.player(c).traits.toughness;
    if near < 2.2 && (past || tough < 12.0) {
        sim.progress = sim.progress.max(me.x - 0.5);
        sim.dead = true;
        sim.ball = Ball::Dead(me);
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
    let iq = sim.agents[qb].a(Attr::Awareness);
    let right = sim.rng.chance(clamp(0.55 + 0.005 * iq, 0.5, 0.97));
    if right { crash } else { !crash }
}

// ------------------------------------------------------------- quarterback

/// The quarterback: drop, read the progression, feel pressure, then throw,
/// scramble, throw it away or hold it (topics 1 and 2). Returns the throw.
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
    let mut threat = V2::default();
    let aw = sim.agents[qb].a(Attr::Awareness);
    for d in 0..sim.agents.len() {
        let a = &sim.agents[d];
        if a.side != 1 || !a.up(t) {
            continue;
        }
        let dist = a.pos.dist(me);
        let free = a.engaged.is_none() && matches!(a.job, Job::Rush { .. });
        let close = if free { dist } else { dist + 1.5 };
        if close < nearest {
            nearest = close;
        }
        if free && dist < 4.5 {
            let seen = sim.sees(qb, a.pos) || sim.rng.chance(0.15 + 0.005 * aw);
            if seen {
                pressure += (4.5 - dist) / 4.5;
                threat = threat + (me - a.pos).norm().scale((4.5 - dist) / 4.5);
            }
        }
    }
    if nearest < 2.6 && ps.pressured_at.is_none() {
        ps.pressured_at = Some(t);
        let ids: Vec<u32> = (0..sim.agents.len())
            .filter(|&d| sim.agents[d].side == 1 && sim.agents[d].pos.dist(me) < 2.8)
            .map(|d| sim.agents[d].id)
            .collect();
        for id in ids {
            sim.notes.pressures.push((id, t));
        }
    }
    // Footwork: drop, then hang in the pocket or slide away from pressure.
    if t < ps.set_at {
        let fake = ps.concept.play_action && t < 0.45;
        let to = if fake { v2(los - 3.5, me.y) } else { ps.drop_to };
        sim.go_to(qb, to, if fake { 0.6 } else { 0.95 });
    } else if pressure > 0.2 {
        let poise = sim.agents[qb].a(Attr::Poise);
        let step = threat.norm().scale(1.0 + 0.01 * poise) + v2(0.6, 0.0);
        sim.go_to(qb, me + step.scale(2.0), 0.6 + 0.004 * aw);
    } else {
        let spot = v2(ps.drop_to.x + 0.8, ps.drop_to.y);
        sim.go_to(qb, spot, 0.3);
    }
    if t < ps.set_at - 0.25 || ps.reads.is_empty() {
        return None;
    }
    // Hot read: an unblocked blitzer gets the ball thrown early.
    let screen = matches!(ps.concept.drop, Drop::Screen | Drop::Bubble);
    if pressure > 0.6 && t < 2.0 && !ps.hot && !screen {
        ps.hot = true;
        if sim.agents[qb].a(Attr::Awareness) > 45.0 || sim.rng.chance(0.4) {
            ps.read = 0;
        }
    }
    let per_read = clamp(0.62 - 0.0045 * (aw - 50.0), 0.3, 0.95);
    // He stays on a read until that route has made its break.
    let developing = ps.read < ps.reads.len()
        && matches!(&sim.agents[ps.reads[ps.read]].job, Job::Route { pts, next, .. } if *next == 0 && pts.len() > 1);
    if t - ps.read_since > per_read && !screen && !(developing && t - ps.read_since < 2.0 * per_read) {
        ps.read += 1;
        ps.read_since = t;
    }
    let over = ps.read >= ps.reads.len();
    // A good quarterback holds the safety with his eyes before coming back.
    ps.look_off = if aw > 70.0 && ps.read == 0 && t - ps.read_since < 0.2 { 1.0 } else { 0.0 };
    let tired = t - ps.set_at;
    let caution = 0.008 * (aw - 60.0) + 0.02 * (10.0 - s.strictness[0]) * 0.0;
    let poise = sim.agents[qb].a(Attr::Poise);
    let panic = pressure * (1.4 - poise / 100.0);
    let (cand, thresh): (Vec<usize>, f64) = if over {
        // Late: the checkdown, or anyone open.
        (ps.reads.clone(), 1.35 - 0.2 * tired.max(0.0) - 0.4 * panic)
    } else {
        (
            vec![ps.reads[ps.read]],
            1.6 + caution - 0.18 * tired.max(0.0) - 0.44 * panic + if ps.read == 0 { 0.1 } else { 0.0 },
        )
    };
    let noise = (1.25 - 0.0105 * (0.6 * aw + 0.4 * sim.agents[qb].a(Attr::Vision))).max(0.25) * (1.0 + 0.6 * panic);
    let mut best: Option<(usize, f64, V2, f64)> = None;
    for &r in if screen { &[][..] } else { &cand[..] } {
        if !sim.agents[r].up(t) || matches!(sim.agents[r].job, Job::PassBlock { .. } | Job::RunBlock { .. }) {
            continue;
        }
        if let Job::Route { release_delay, .. } = sim.agents[r].job
            && t < release_delay
        {
            continue;
        }
        let (w, p, flight) = window(sim, qb, r);
        let seen = w + noise * gauss(sim.rng);
        let need = thresh;
        if super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!("READ t={:.1} read {} r {} w {:.2} seen {:.2} need {:.2}", t, ps.read, r, w, seen, need);
        }
        if seen > need && best.is_none_or(|b| seen > b.1) {
            best = Some((r, seen, p, flight));
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
    if let Some((r, _, p, flight)) = best {
        return Some(throw(sim, ps, qb, r, p, flight, pressure, s));
    }
    // Nothing open: scramble, throw it away, or hold it.
    let speed = sim.agents[qb].a(Attr::Speed);
    let lane = (0..sim.agents.len()).filter(|&d| sim.agents[d].side == 1 && sim.agents[d].up(t)).all(|d| {
        let a = sim.agents[d].pos;
        !(a.x > me.x - 1.0 && a.x < me.x + 5.0 && (a.y - me.y).abs() < 2.5)
    });
    let want_run = (over || pressure > 0.7)
        && lane
        && t > 2.2
        && sim.rng.chance(clamp(0.015 + 0.0028 * (speed - 50.0), 0.006, 0.15));
    if want_run || (pressure > 1.0 && lane && speed > 75.0 && sim.rng.chance(0.25)) {
        ps.scramble = true;
        return None;
    }
    // Throwing it away takes a rusher on top of him (or a play long dead)
    // and the sense to give up on it; many hold on and take the sack.
    let outside = (me.y - s.ball_y).abs() > 4.0;
    let imminent = nearest < 2.2 && pressure > 0.4;
    let away =
        over && (imminent || tired > 3.2) && sim.rng.chance(0.06 + 0.003 * aw + if outside { 0.12 } else { 0.0 });
    if away {
        // Throw it away (outside the pocket it is legal past the line).
        let side = if me.y > MID { 1.0 } else { -1.0 };
        let to = v2(los + 3.0, if side > 0.0 { WIDTH + 2.0 } else { -2.0 });
        if !outside && sim.rng.chance(0.08) {
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
    let power = sim.agents[qb].a(Attr::ThrowPower);
    let speed = 16.5 + 0.13 * power;
    let mut p = ra.pos;
    let mut flight = 0.0;
    for _ in 0..3 {
        let dist = q.dist(p);
        flight = 0.28 + dist / (speed * (1.0 - 0.005 * (dist - 20.0).max(0.0)));
        p = predict(sim, r, flight);
    }
    let mut w = f64::INFINITY;
    for d in 0..sim.agents.len() {
        let a = &sim.agents[d];
        if a.side != 1 || !a.up(sim.t) || matches!(a.job, Job::Rush { .. }) {
            continue;
        }
        // A man defender already running with him needs no reaction time.
        let on_him = matches!(a.job, Job::Man { target, .. } if target == r);
        let (react, pace) = if on_him { (0.0, 1.0) } else { (0.3, 0.95) };
        // How far he can get there: he reacts, then speeds up from what
        // he is doing now (a man running away has to stop first).
        let tau = (flight - react).max(0.0);
        let v0 = clamp(a.vel.dot((p - a.pos).norm()), -2.0, a.vmax);
        let close = pace * (a.vmax * tau - (a.vmax - v0) * a.tau * (1.0 - (-tau / a.tau).exp())).max(0.0);
        let gap = a.pos.dist(p) - close;
        // In the lane: a defender between the quarterback and the spot.
        let seg = p - q;
        let k = clamp((a.pos - q).dot(seg) / seg.dot(seg).max(1e-6), 0.0, 1.0);
        let lane = if k > 0.3 && k < 0.95 { (q + seg.scale(k)).dist(a.pos) - 0.4 - 0.8 * k } else { f64::INFINITY };
        w = w.min(gap).min(lane + 0.6);
    }
    (w.min(8.0), p, flight)
}

/// Where a receiver will be: the quarterback knows his route and how his
/// man runs it, so he plays the route forward with the receiver's own
/// speed, acceleration and cutting.
fn predict(sim: &Sim, r: usize, dt: f64) -> V2 {
    let mut a = sim.agents[r].clone();
    if let Job::Route { pts, next, settle, .. } = &sim.agents[r].job {
        let mut n = *next;
        let steps = (dt / DT).round() as usize;
        for _ in 0..steps {
            if n < pts.len() {
                let (target, speed, done) = route_step(&a, pts, n, *settle);
                if done {
                    n += 1;
                }
                a.steer((target - a.pos).norm().scale(speed));
            } else if *settle {
                break;
            } else {
                let v = a.vel;
                let dir = if v.len() > 0.5 { v.norm() } else { v2(1.0, 0.0) };
                let me = a.pos;
                let dir =
                    if me.y < 4.0 && dir.y < 0.0 || me.y > WIDTH - 4.0 && dir.y > 0.0 { v2(1.0, 0.0) } else { dir };
                a.steer(dir.scale(a.vmax));
            }
            a.pos = a.pos + a.vel.scale(DT);
        }
        return v2(a.pos.x.min(109.0), clamp(a.pos.y, 0.5, WIDTH - 0.5));
    }
    let p = a.pos + a.vel.scale(dt);
    v2(p.x.min(109.0), clamp(p.y, 0.5, WIDTH - 0.5))
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
    let acc = sim.agents[qb].a(Attr::ThrowAccuracy);
    let power = sim.agents[qb].a(Attr::ThrowPower);
    let dist = q.dist(p);
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
        target = target + away.scale(0.45 * acc / 100.0);
    }
    let moving = sim.agents[qb].vel.len() > 3.0;
    let poise = sim.agents[qb].a(Attr::Poise);
    // Error grows gently as accuracy falls: real completion rates drop only a
    // few points from the NFL to college and high school, where receivers
    // and coverage are just as far behind (proposed, not confirmed).
    let mut sd = (0.25 + 0.036 * dist + 0.0007 * dist * dist) * 1.5 * (0.005 * (79.0 - acc)).exp();
    if pressure > 0.3 {
        sd *= 1.0 + 0.55 * pressure.min(1.5) * (1.35 - poise / 100.0);
    }
    if moving {
        sd *= 1.25 - 0.002 * sim.agents[qb].a(Attr::Agility);
    }
    let wind = s.cond.wind * (0.004 + 0.0004 * dist);
    sd *= 1.0 + wind;
    if s.cond.rain || s.cond.snow {
        sd *= 1.1;
    }
    let consistency = sim.player(qb).traits.consistency;
    sd *= 1.15 - 0.015 * consistency;
    let along = (target - q).norm();
    let across = along.perp();
    let deep = if dist > 25.0 { 1.35 } else { 1.0 };
    let e1 = sd * deep * gauss(sim.rng);
    let e2 = sd * gauss(sim.rng);
    let to = target + along.scale(e1) + across.scale(e2);
    let arrive = t + flight;
    if super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed) {
        let ra = &sim.agents[r];
        println!(
            "THROW t={t:.1} aim ({:.1},{:.1}) to ({:.1},{:.1}) rec ({:.1},{:.1}) vel ({:.1},{:.1}) vmax {:.1} flight {flight:.2} sd {sd:.2}",
            p.x, p.y, to.x, to.y, ra.pos.x, ra.pos.y, ra.vel.x, ra.vel.y, ra.vmax
        );
    }
    ps.thrown = Some(t);
    ps.target = Some(r);
    let height = 2.0 + 0.08 * dist;
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
        sim.agents[d].ball_react = if sees { t + base } else { (arrive - 0.45 + 0.004 * (100.0 - pr)).max(t + base) };
    }
    // The receiver looks for it: at once on a timing throw, a beat later
    // when he is still running away from the quarterback.
    let aw = sim.agents[r].a(Attr::Awareness);
    let facing_qb = sim.agents[r].face.dot((q - sim.agents[r].pos).norm()) > -0.2;
    sim.agents[r].ball_react = t + if facing_qb { 0.1 } else { 0.25 } + 0.003 * (100.0 - aw);
    // Roughing: a hit arriving after the ball is out.
    let late = (0..sim.agents.len())
        .find(|&d| sim.agents[d].side == 1 && sim.agents[d].engaged.is_none() && sim.agents[d].pos.dist(q) < 1.6);
    if let Some(d) = late {
        ps.hit = true;
        let id = sim.agents[d].id;
        sim.notes.hits.push(id);
        let disc = sim.player(d).traits.discipline;
        if sim.rng.chance(0.06 * (1.5 - disc / 20.0) * sim.crew.personal) {
            sim.notes.fouls.push(Foul { kind: FoulKind::RoughingPasser, side: 1, player: id, at: q.x });
        }
    }
    (to, r, dist, flight, sd)
}

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
    let mut tacklers: Vec<usize> = (0..sim.agents.len())
        .filter(|&d| {
            let a = &sim.agents[d];
            // Closest approach over the last tick, as with tackles.
            let rel = a.pos - me;
            let rv = (a.vel - sim.agents[qb].vel).scale(DT);
            let back = if rv.len() > 1e-6 { clamp(rel.dot(rv) / rv.dot(rv), 0.0, 1.0) } else { 0.0 };
            a.side == 1 && a.up(t) && a.engaged.is_none() && (rel - rv.scale(back)).len() < 1.25 && t - a.last_try > 0.6
        })
        .collect();
    if tacklers.is_empty() {
        return;
    }
    tacklers.sort_by(|&a, &b| sim.agents[a].pos.dist(me).total_cmp(&sim.agents[b].pos.dist(me)));
    let d = tacklers[0];
    sim.agents[d].last_try = t;
    ps.hit = true;
    // Escape: a mobile, strong quarterback slips some.
    let (q, r) = (&sim.agents[qb], &sim.agents[d]);
    let esc = 0.4 * q.a(Attr::Elusiveness) + 0.3 * q.a(Attr::Agility) + 0.3 * q.a(Attr::Strength);
    let tk = 0.6 * r.a(Attr::Tackling) + 0.2 * r.a(Attr::Strength) + 0.2 * r.a(Attr::Agility);
    let seen = sim.sees(qb, r.pos);
    let logit = -1.9 + (esc - tk) / 12.0 + if seen { 0.0 } else { -1.0 };
    if sim.rng.chance(1.0 / (1.0 + (-logit).exp())) {
        let (rid, qid) = (r.id, q.id);
        sim.notes.missed.push((rid, qid));
        sim.agents[d].down_until = t + 0.6;
        return;
    }
    // Sacked. Strip sacks come from the blind side.
    sim.notes.sackers =
        tacklers.iter().take(2).filter(|&&k| sim.agents[k].pos.dist(me) < 1.1).map(|&k| sim.agents[k].id).collect();
    sim.notes.hits.push(sim.agents[d].id);
    let force = sim.agents[d].mass * sim.agents[d].vel.len();
    let carry = sim.agents[qb].a(Attr::Carrying);
    let p_fumble = (if seen { 0.06 } else { 0.16 }) * (1.3 - carry / 120.0);
    sim.fight = vec![d];
    if sim.rng.chance(p_fumble) {
        sim.notes.fumbled = Some(sim.agents[qb].id);
        sim.notes.forced_fumble = Some(sim.agents[d].id);
        sim.fumble(qb);
    } else {
        sim.progress = me.x.min(los);
        sim.notes.tacklers = vec![sim.agents[d].id];
        sim.dead = true;
        sim.ball = Ball::Dead(me);
    }
    sim.injury_check(qb, force, if seen { 0.8 } else { 1.3 });
}

// ------------------------------------------------------------- defense

#[allow(clippy::too_many_arguments)]
fn defend(
    sim: &mut Sim,
    i: usize,
    s: &Snap,
    qb: usize,
    pass: &Option<Pass>,
    trail: &[Vec<V2>],
    vel_trail: &[Vec<V2>],
    in_air: bool,
) {
    let t = sim.t;
    let job = sim.agents[i].job.clone();
    let carrier = sim.carrier();
    let pass_play = pass.is_some();
    let knows = t >= sim.agents[i].read_at;
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
                    cover(sim, i, &job, s, qb, pass, trail, vel_trail);
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
            sim.go_to(i, to, 1.0);
            return;
        }
    }
    match job {
        Job::Rush { lane, contain, .. } => {
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
                    let x = s.los - sim.goal * 0.4;
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
                let vm = sim.agents[i].vmax;
                sim.steer(i, (aim - me).norm().scale(vm));
                return;
            }
            sim.go_to(i, target, 1.0);
        }
        Job::Gap { y } => {
            let spot = v2(s.los + 1.0, y);
            sim.go_to(i, spot, 0.9);
        }
        Job::Man { .. } | Job::Zone { .. } => {
            if !pass_play && knows {
                // Run fit: linebackers fill, defensive backs come up to force.
                let c = carrier.unwrap_or(qb);
                let me = sim.agents[i].pos;
                let cp = sim.agents[c].pos;
                let deep = matches!(job, Job::Zone { deep: true, .. }) || me.x - s.los > 9.0;
                // A corner in man stays on his receiver (who may be faking a
                // route or blocking him) until the runner is clearly through.
                let on_wr = matches!(job, Job::Man { target, .. } if !matches!(sim.agents[target].job, Job::RunBlock { target: None, .. }) && (sim.agents[target].pos.y - s.ball_y).abs() > 8.0);
                if on_wr && (cp.x - s.los) * sim.goal < 3.0 {
                    cover(sim, i, &job, s, qb, pass, trail, vel_trail);
                    return;
                }
                if deep {
                    // The last line fits the alley: he comes down to about
                    // seven yards and squares up, and only attacks once the
                    // runner is through the line.
                    // He comes down under control, taking his angle to the
                    // runner, and runs all out once the runner is through.
                    let pr = sim.agents[i].a(Attr::PlayRecognition);
                    // A safety with a deep zone is the last line: he fits
                    // deeper (about nine yards) so nothing gets over him.
                    let post = matches!(job, Job::Zone { deep: true, .. });
                    let (wait, fill) = if post { (3.5, 8.5) } else { (2.5, 6.5) };
                    let through = (cp.x - s.los) * sim.goal > wait - 0.02 * (pr - 50.0);
                    let wide = (me.y - s.ball_y).abs() > 12.0 && (cp.x - s.los) * sim.goal < 3.0;
                    if wide {
                        // A deep corner keeps outside leverage until it breaks.
                        let depth = s.los + sim.goal * 9.0;
                        let x = if (me.x - depth) * sim.goal > 0.0 { depth } else { me.x };
                        sim.go_to(i, v2(x, me.y + (cp.y - me.y) * 0.2), 0.6);
                    } else {
                        let floor = if through { None } else { Some(s.los + sim.goal * (fill - 0.02 * (pr - 50.0))) };
                        sim.pursue_cap(i, c, if through { 1.0 } else { 0.6 }, floor);
                    }
                } else {
                    sim.pursue(i, c);
                }
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
            cover(sim, i, &job, s, qb, pass, trail, vel_trail);
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

#[allow(clippy::too_many_arguments)]
fn cover(
    sim: &mut Sim,
    i: usize,
    job: &Job,
    s: &Snap,
    qb: usize,
    pass: &Option<Pass>,
    trail: &[Vec<V2>],
    vel_trail: &[Vec<V2>],
) {
    cover_inner(sim, i, job, s, qb, pass, trail, vel_trail);
    // The end line: nobody covers anything behind it.
    let a = &mut sim.agents[i];
    let back = END_LINE - 0.7;
    if (a.pos.x - back) * sim.goal > 0.0 && a.vel.x * sim.goal > 0.0 {
        a.vel.x = 0.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn cover_inner(
    sim: &mut Sim,
    i: usize,
    job: &Job,
    s: &Snap,
    qb: usize,
    pass: &Option<Pass>,
    trail: &[Vec<V2>],
    vel_trail: &[Vec<V2>],
) {
    let t = sim.t;
    match *job {
        Job::Man { target, cushion, lag } => {
            // He mirrors where he last saw his man going (topic 2): when the
            // receiver breaks, he is late by his reaction time.
            let n = trail[target].len();
            let k = ((lag / DT).round() as usize).min(n.saturating_sub(1));
            let (p, v) = if n > 0 {
                (trail[target][n - 1 - k], vel_trail[target][n - 1 - k])
            } else {
                (sim.agents[target].pos, sim.agents[target].vel)
            };
            // He is late on where his man turns, not on how fast he runs:
            // speed he feels at once, direction he sees late.
            let now = sim.agents[target].vel.len();
            let v = if v.len() > 0.3 { v.norm().scale(now.max(v.len())) } else { sim.agents[target].vel };
            let est = p + v.scale(lag);
            let me = sim.agents[i].pos;
            let inside = if est.y > s.ball_y { -0.6 } else { 0.6 };
            let deep = est.x - s.los > 8.0;
            // Off coverage keeps a cushion until the receiver eats it up.
            let cush = (cushion - (t * 2.2)).max(if deep { 0.6 } else { 0.0 });
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
            sim.steer(i, want);
            // Hand-fighting downfield can draw a flag (defensive holding / illegal contact).
            let rd = sim.agents[target].pos.dist(me);
            if rd < 0.9
                && t > 0.8
                && t < 2.6
                && sim.rng.chance(0.0016 * sim.crew.holding * (1.6 - sim.player(i).traits.discipline / 14.0))
            {
                let (id, at) = (sim.agents[i].id, me.x);
                let ok = pass.as_ref().is_some_and(|p| p.thrown.is_none());
                if ok {
                    sim.notes.fouls.push(Foul { kind: FoulKind::DefensiveHolding, side: 1, player: id, at });
                }
            }
        }
        Job::Zone { spot, deep, width } => {
            let me = sim.agents[i].pos;
            let q = sim.agents[qb].pos;
            // Threats in his area that he can see.
            let mut threats: Vec<usize> = Vec::new();
            for r in 0..sim.agents.len() {
                let a = &sim.agents[r];
                if a.side != 0 || !matches!(a.job, Job::Route { .. }) {
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
                if in_zone && (sim.sees(i, a.pos) || (deep || a.pos.x > me.x) && a.pos.dist(me) < 10.0) {
                    threats.push(r);
                }
            }
            // A sharp defender reads the quarterback's eyes; a looked-off one bites.
            let eyes = pass.as_ref().and_then(|p| p.reads.get(p.read).copied());
            let pr = sim.agents[i].a(Attr::PlayRecognition);
            let mut target = spot;
            // A deep defender keeps a cushion that grows with the threat's
            // speed and runs with him (a speed turn) rather than waiting.
            let mut carry = V2::default();
            if deep {
                if let Some(&r) = threats.iter().max_by(|&&a, &&b| sim.agents[a].pos.x.total_cmp(&sim.agents[b].pos.x))
                {
                    let rp = sim.agents[r].pos;
                    let rv = sim.agents[r].vel;
                    let cushion = 2.5 + 1.1 * rv.x.max(0.0);
                    // Two verticals in his area: he splits them, leaning to the deeper.
                    let mut verts: Vec<V2> =
                        threats.iter().map(|&k| sim.agents[k].pos).filter(|p| p.x > s.los + 8.0).collect();
                    verts.sort_by(|a, b| b.x.total_cmp(&a.x));
                    verts.truncate(2);
                    let ty = if verts.len() >= 2 {
                        let mean = verts.iter().map(|p| p.y).sum::<f64>() / verts.len() as f64;
                        0.6 * mean + 0.4 * rp.y
                    } else {
                        spot.y + (rp.y - spot.y) * 0.65
                    };
                    target = v2((rp.x + cushion).max(spot.x), ty.clamp(spot.y - width / 2.0, spot.y + width / 2.0));
                    if rv.x > 3.0 && target.x > me.x {
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
                let rp = sim.agents[r].pos;
                let under = rp + (q - rp).norm().scale(1.3);
                let lim = width / 2.0 + 3.0;
                let floor = if rp.x > s.los + 3.5 { s.los } else { (spot.x - 3.0).max(s.los + 2.0) };
                target = v2(under.x.clamp(floor, spot.x + 6.0), under.y.clamp(spot.y - lim, spot.y + lim));
                // He moves with the route he is under (not easing into a spot
                // the receiver has already left).
                carry = sim.agents[r].vel.scale(0.9);
            } else {
                // Nothing in his area: he sinks under a vertical running
                // through it (a cloud corner carrying #1) until he hands it off.
                let sink = (0..sim.agents.len()).filter(|&r| {
                    let a = &sim.agents[r];
                    a.side == 0
                        && matches!(a.job, Job::Route { .. })
                        && (a.pos.y - spot.y).abs() < width / 2.0 + 1.0
                        && a.vel.x > 4.0
                        && a.pos.x > spot.x
                        && a.pos.x < spot.x + 14.0
                });
                if let Some(r) = sink.min_by(|&a, &b| sim.agents[a].pos.dist(me).total_cmp(&sim.agents[b].pos.dist(me)))
                {
                    let rp = sim.agents[r].pos;
                    target = v2(rp.x - 1.5, rp.y + (q.y - rp.y).signum() * 0.8);
                }
            }
            if let Some(e) = eyes {
                let ep = sim.agents[e].pos;
                if ep.dist(target) < 9.0 {
                    let lean = clamp(0.004 * (pr - 40.0), 0.0, 0.3);
                    target = target + (ep - target).scale(lean);
                }
                // Jumping the route (topic 3.1): a gamble only smart, trusted players win.
                if !deep
                    && ep.dist(me) < 7.0
                    && freelance_ok(sim, i, s.strictness[1], s.trust[1][sim.agents[i].p])
                    && pr > 70.0
                {
                    let jump = 0.012 * (pr - 70.0) / 30.0;
                    if sim.rng.chance(jump) {
                        let id = sim.agents[i].id;
                        if !sim.notes.freelance.iter().any(|f| f.0 == id) {
                            sim.notes.freelance.push((id, false));
                        }
                        let under = ep + (q - ep).norm().scale(0.8);
                        target = under;
                    }
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

/// The ball arrives (topic 1): who can get to it, and who catches it.
fn catch_point(sim: &mut Sim, r: usize, to: V2, from: V2, release: f64, s: &Snap, trail: &[Vec<V2>]) -> Catch {
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
            let idx = ((tk / DT).round() as usize).min(trail[d].len().saturating_sub(1));
            let Some(&dp) = trail[d].get(idx) else { continue };
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
                if sim.rng.chance(0.3 * a.a(Attr::Catching) / 70.0) {
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
    let radius = 0.95 + 0.015 * (gp_h - 72.0) + 0.004 * (ra.a(Attr::Jumping) - 50.0) + 0.02 * (gp_arm - 32.0);
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
    if to.y < -0.5 || to.y > WIDTH + 0.5 || dr > radius + 1.3 {
        // Off target. A defender in the way may still pick it.
        if let Some(d) = dbest {
            let da = &sim.agents[d];
            let rd = 1.0 + 0.01 * (da.a(Attr::Jumping) - 50.0);
            if dd < rd * 0.8 && sim.rng.chance(0.18 * da.a(Attr::Catching) / 70.0) {
                return Catch::Intercepted(d);
            }
        }
        return Catch::Incomplete { drop: false, pbu: None, sep };
    }
    // Contest at the catch point.
    if let Some(d) = dbest {
        let da = sim.agents[d].clone();
        let reacted = sim.t >= da.ball_react;
        let d_reach = 1.05 + 0.012 * (sim.player(d).height - 72.0) + 0.004 * (da.a(Attr::Jumping) - 50.0);
        if dd < d_reach && reacted {
            let cov = da.a(Attr::ManCoverage).max(da.a(Attr::ZoneCoverage));
            let closeness = 1.0 - dd / d_reach;
            // He's in front of it: an interception chance.
            if dd < dr - 0.15 {
                // Playing the ball: a facing defender has a real chance; one
                // trailing with his back turned mostly plays through the hands.
                let sees_ball = da.face.dot((from - da.pos).norm()) > 0.0;
                let p_int = clamp(
                    0.22 * closeness * (0.45 + da.a(Attr::Catching) / 110.0) * if sees_ball { 1.0 } else { 0.45 },
                    0.0,
                    0.6,
                );
                if sim.rng.chance(p_int) {
                    return Catch::Intercepted(d);
                }
            }
            let p_pbu =
                clamp(closeness * (0.42 + 0.006 * (cov - 60.0) + 0.003 * (da.a(Attr::Jumping) - 60.0)), 0.0, 0.75);
            if sim.rng.chance(p_pbu) {
                pi_check(sim, d, r, to, air, true);
                if sim.rng.chance(0.06) {
                    return Catch::Intercepted(d);
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
    let mut p = 1.0 - (0.105 - 0.00095 * c).max(0.012);
    let reach = dr / radius;
    // Inside his frame it is routine; reaching away from his body or
    // diving costs more the farther it is.
    if reach > 0.75 {
        p *= 1.0 - 0.7 * clamp((reach - 0.75) / 1.3, 0.0, 1.0);
    }
    if dd < 1.4 {
        let fight = 0.5 * c + 0.25 * ra.a(Attr::Strength) + 0.25 * ra.a(Attr::Jumping);
        p *= 1.0 - 0.32 * (1.0 - dd / 1.4) * (1.25 - fight / 100.0);
    }
    if s.cond.rain || s.cond.snow {
        p *= 0.97;
    }
    let speed = from.dist(to) / (sim.t - release).max(0.2);
    if speed > 24.0 && dr < 1.0 {
        p *= 0.985;
    }
    if sim.rng.chance(p) {
        Catch::Complete(sep)
    } else {
        // A ball off his hands can pop up to a defender close by.
        if let Some(d) = dbest
            && dd < 3.0
            && sim.rng.chance(0.13 * (1.0 - dd / 3.0) * (0.5 + sim.agents[d].a(Attr::Catching) / 100.0))
        {
            return Catch::Intercepted(d);
        }
        let drop = reach < 0.8 && dd > 1.0;
        Catch::Incomplete { drop, pbu: None, sep }
    }
}

/// Pass interference: a beaten defender who plays the man, not the ball.
fn pi_check(sim: &mut Sim, d: usize, r: usize, to: V2, air: f64, broke_up: bool) {
    if air < 4.0 {
        return;
    }
    let (da, ra) = (&sim.agents[d], &sim.agents[r]);
    let trailing = (to - da.pos).dot(to - ra.pos) > 0.0 && da.pos.dist(to) > ra.pos.dist(to);
    let facing = da.face.dot((sim.ball_pos() - da.pos).norm()) > 0.3;
    let disc = sim.player(d).traits.discipline;
    let base = if trailing && !facing {
        0.55
    } else if trailing {
        0.22
    } else {
        0.05
    };
    let p = base
        * (1.35 - disc / 20.0)
        * sim.crew.pass_interference
        * if broke_up { 1.2 } else { 0.7 }
        * (0.6 + air / 40.0);
    if sim.rng.chance(p) {
        let id = da.id;
        sim.notes.fouls.push(Foul { kind: FoulKind::PassInterference, side: 1, player: id, at: to.x });
    } else if sim.rng.chance(0.012 * (1.3 - sim.player(r).traits.discipline / 20.0) * sim.crew.pass_interference) {
        let id = ra.id;
        sim.notes.fouls.push(Foul { kind: FoulKind::OffensivePassInterference, side: 0, player: id, at: to.x });
    }
}
