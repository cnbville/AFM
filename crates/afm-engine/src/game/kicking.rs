//! Special teams (topic 7): kickoffs, punts, field goals and extra points.
//! Kicks follow ball flight (leg, accuracy, wind, cold, altitude); snaps,
//! holds and protection are their own battles; returns are full tracked
//! plays. For kickers and punters, composure is the biggest factor.

use super::geom::{DT, MID, V2, WIDTH, v2};
use super::roster::GamePlayer;
use super::rules::Rules;
use super::scrimmage::escort;
use super::sim::*;
use crate::attributes::Attr;
use crate::rng::{Rng, clamp};

/// The kicking team and the return team for a kick.
pub struct KickTeams<'a> {
    pub kick: &'a [GamePlayer],
    pub ret: &'a [GamePlayer],
    /// The 11 on each side (player indexes). The kicker or punter is
    /// `kick_on[0]`, the returners `ret_on[0]` (and `ret_on[1]` on kickoffs).
    pub kick_on: Vec<usize>,
    pub ret_on: Vec<usize>,
    pub snapper: usize,
    pub holder: usize,
    pub energy: [&'a [f64]; 2],
}

#[derive(Clone, Debug, Default)]
pub struct KickResult {
    /// Where the receiving team takes over, yards from its own goal line.
    pub spot: f64,
    pub touchback: bool,
    pub fair_catch: bool,
    pub out_of_bounds: bool,
    pub returned: bool,
    pub return_yards: f64,
    pub return_td: bool,
    /// Kicking team got it back (onside, muff) at `spot` in the kicking team's frame.
    pub recovered_by_kicking: bool,
    pub muffed: bool,
    pub blocked: bool,
    pub bad_snap: bool,
    /// Gross distance of the kick, and hang time.
    pub distance: f64,
    pub hang: f64,
    pub inside20: bool,
    pub duration: f64,
    pub returner: Option<u32>,
    pub notes: Notes,
    pub frames: Option<Vec<Frame>>,
}

/// Kick distance for a leg (yards in the air), with weather and altitude.
fn leg(kicker: &GamePlayer, base: f64, per: f64, cond: &Conditions, along: f64) -> f64 {
    let kp = kicker.a(Attr::KickPower);
    let mut d = base + per * kp;
    // Thin air carries the ball; cold air doesn't.
    d *= 1.0 + 0.000015 * cond.altitude;
    d *= 1.0 - 0.0018 * (55.0 - cond.temperature).max(0.0);
    d += 0.35 * cond.wind * along;
    if cond.rain || cond.snow {
        d *= 0.96;
    }
    d
}

/// Composure under pressure (topic 7): how much a big moment widens his miss.
fn pressure_factor(kicker: &GamePlayer, pressure: f64) -> f64 {
    let c = kicker.composure();
    1.0 + pressure * clamp(1.15 - c / 15.0, 0.0, 1.0) * 0.9
}

#[derive(Clone, Debug)]
pub struct FieldGoal {
    pub made: bool,
    pub blocked: bool,
    pub distance: f64,
    pub bad_snap: bool,
    pub why: String,
}

/// A field goal or extra point try from `distance` yards (spot of the hold
/// plus ten for the end zone). `pressure` is 0-1: the game on the line, an
/// icing timeout, a playoff.
#[allow(clippy::too_many_arguments)]
pub fn field_goal(
    kicker: &GamePlayer,
    snapper: &GamePlayer,
    holder: &GamePlayer,
    rush: f64,
    protect: f64,
    distance: f64,
    pressure: f64,
    cond: &Conditions,
    along: f64,
    rng: &mut Rng,
) -> FieldGoal {
    // The snap and the hold are their own battles (topic 7).
    let snap_skill = snapper.traits.snapping;
    let bad_snap = rng.chance(0.004 * (1.0 + (10.0 - snap_skill).max(-5.0) * 0.18));
    let hold = holder.a(Attr::Catching) * 0.5 + holder.a(Attr::Awareness) * 0.5;
    let bad_hold = !bad_snap && rng.chance(0.004 * (1.6 - hold / 100.0));
    let mut fg = FieldGoal { made: false, blocked: false, distance, bad_snap, why: String::new() };
    if bad_snap && rng.chance(0.6) {
        fg.why = format!("The snap got away from holder #{}.", holder.id);
        return fg;
    }
    // Protection: a low, long kick is easier to block.
    let traj = (distance - 35.0).max(0.0) * 0.0005;
    let p_block =
        (0.008 + traj) * (1.0 + (rush - protect) / 30.0).max(0.3) * if bad_snap || bad_hold { 3.0 } else { 1.0 };
    if rng.chance(p_block) {
        fg.blocked = true;
        fg.why = format!("Blocked: the rush got a hand on a {:.0}-yard try.", distance);
        return fg;
    }
    let short = distance < 40.0;
    let acc = kicker.a(Attr::KickAccuracy) + if short { kicker.traits.kick_short } else { kicker.traits.kick_long };
    // Spread in degrees: an average NFL kicker misses by about 2.5 degrees.
    let mut sd = (2.4 + 0.014 * (distance - 30.0)) * (1.82 - 0.0105 * acc).max(0.55);
    sd *= 1.18 - 0.018 * kicker.traits.consistency;
    sd *= pressure_factor(kicker, pressure);
    if bad_snap || bad_hold {
        sd *= 1.6;
    }
    sd *= 1.0 + 0.012 * cond.wind;
    let angle = sd * rng.normal(0.0, 1.0);
    let lateral = distance * angle.to_radians().tan();
    let reach = leg(kicker, 40.0, 0.3, cond, along) + rng.normal(0.0, 3.0);
    let wide = lateral.abs() > 3.083;
    let short_of = reach < distance;
    fg.made = !wide && !short_of;
    let composure = kicker.composure();
    fg.why = if fg.made {
        if pressure > 0.6 && composure > 14.0 {
            format!("Good from {:.0}: ice in his veins with the game on the line.", distance)
        } else {
            format!("Good from {:.0}.", distance)
        }
    } else if short_of {
        format!("Short from {:.0}: his leg ran out at about {:.0} yards.", distance, reach)
    } else if pressure > 0.6 && composure < 8.0 {
        format!("Wide {} from {:.0}: the moment got to him.", if lateral > 0.0 { "left" } else { "right" }, distance)
    } else {
        format!("Wide {} from {:.0}.", if lateral > 0.0 { "left" } else { "right" }, distance)
    };
    fg
}

/// Pass rush strength of a field goal unit, and its protection.
pub fn kick_unit_strength(players: &[GamePlayer], on: &[usize], rush: bool) -> f64 {
    let v: Vec<f64> = on
        .iter()
        .map(|&i| {
            let p = &players[i];
            if rush {
                0.5 * p.a(Attr::PassRush) + 0.3 * p.a(Attr::Strength) + 0.2 * p.a(Attr::Jumping)
            } else {
                0.6 * p.a(Attr::PassBlocking) + 0.4 * p.a(Attr::Strength)
            }
        })
        .collect();
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

/// A kickoff (topic 7), in the receiving team's frame: it goes toward +x
/// from its own goal line. `onside` is a called onside kick.
#[allow(clippy::too_many_arguments)]
pub fn kickoff(
    k: &KickTeams,
    rules: &Rules,
    cond: &Conditions,
    crew: &Crew,
    along: f64,
    onside: bool,
    injury_scale: f64,
    record: bool,
    rng: &mut Rng,
) -> KickResult {
    let kicker = &k.kick[k.kick_on[0]];
    let from = 100.0 - rules.kickoff_from;
    let mut res = KickResult::default();
    if onside {
        // A short kick and a scramble for it; the hands team waits for it.
        let acc = kicker.a(Attr::KickAccuracy);
        let p = clamp(0.08 + 0.0025 * (acc - 50.0) + if rules.nfl_kickoff { -0.02 } else { 0.03 }, 0.03, 0.25);
        res.distance = 11.0 + rng.range(0.0, 3.0);
        let spot = from - res.distance;
        res.spot = spot;
        res.recovered_by_kicking = rng.chance(p);
        res.duration = 2.5;
        return res;
    }
    let max = leg(kicker, 44.0, 0.23, cond, along) + rng.normal(0.0, 3.0);
    // Where he aims: the NFL's 2025 landing zone makes kickers drop it short
    // of the goal line; elsewhere a big leg aims for a touchback.
    let aim = if rules.nfl_kickoff {
        let deep = from - max;
        if deep < -6.0 && rng.chance(0.55) { -4.0 } else { 3.0 + rng.range(0.0, 4.0) }
    } else if from - max < -2.0 {
        -5.0
    } else {
        from - max
    };
    let land_x = (from - max).max(aim) + rng.normal(0.0, 2.5);
    let land_y = clamp(MID + rng.normal(0.0, 7.0), 2.0, WIDTH - 2.0);
    let hang = clamp(3.4 + 0.014 * (kicker.a(Attr::KickPower) - 60.0) + rng.normal(0.0, 0.2), 2.8, 4.8);
    res.distance = from - land_x;
    res.hang = hang;
    // Out of bounds: rare, the ball goes to the 40.
    if rng.chance(0.006 * (1.4 - kicker.a(Attr::KickAccuracy) / 100.0)) {
        res.out_of_bounds = true;
        res.spot = 40.0;
        res.duration = hang;
        return res;
    }
    if rules.nfl_kickoff && land_x > 20.0 {
        // Short of the landing zone.
        res.spot = 40.0;
        res.duration = hang;
        return res;
    }
    if land_x < -9.5 {
        res.touchback = true;
        res.spot = rules.kickoff_touchback;
        return res;
    }
    // The returner's call: take a knee deep in the end zone, or bring it out.
    let returner = k.ret_on[0];
    let rp = &k.ret[returner];
    let judgment = rp.a(Attr::Awareness);
    let tb_value = rules.kickoff_touchback;
    let kneel = land_x < 0.0 && {
        let expected = land_x + 22.0 + 0.1 * (rp.a(Attr::Speed) - 70.0);
        let mut kneel = expected < tb_value;
        if rng.chance(clamp(0.35 - 0.003 * judgment, 0.05, 0.4)) {
            kneel = !kneel;
        }
        kneel
    };
    if kneel {
        res.touchback = true;
        res.spot = tb_value;
        res.duration = hang;
        return res;
    }
    // College: a fair catch inside the 25 is a touchback.
    if !rules.nfl_kickoff && rules.kickoff_touchback >= 25.0 && land_x < 25.0 && land_x > -1.0 && rng.chance(0.3) {
        res.fair_catch = true;
        res.spot = 25.0;
        res.duration = hang;
        return res;
    }
    // The return: a full tracked play.
    let catch = v2(land_x, land_y);
    let start_cover = if rules.nfl_kickoff { hang } else { 0.0 };
    run_return(k, catch, hang, start_cover, from, rules, cond, crew, injury_scale, record, rng, &mut res, true, None);
    res
}

/// A punt from the kicking team's `los`. Everything in the receiving team's
/// frame on the way out.
#[allow(clippy::too_many_arguments)]
pub fn punt(
    k: &KickTeams,
    los: f64,
    rules: &Rules,
    cond: &Conditions,
    crew: &Crew,
    along: f64,
    injury_scale: f64,
    record: bool,
    rng: &mut Rng,
) -> KickResult {
    let punter = &k.kick[k.kick_on[0]];
    let snapper = &k.kick[k.snapper];
    let mut res = KickResult::default();
    // The snap.
    let snap_skill = snapper.traits.snapping;
    let bad = rng.chance(0.005 * (1.0 + (10.0 - snap_skill).max(-5.0) * 0.2));
    res.bad_snap = bad;
    // The rush against the shield; a slow operation gets blocked.
    let op = 2.05 + if bad { 0.7 } else { 0.0 } + rng.normal(0.0, 0.08);
    let p_block = 0.0028 * (op / 2.05).powi(8);
    let from = 100.0 - los; // receiving team frame: the kicking spot
    if rng.chance(p_block) {
        res.blocked = true;
        res.spot = (from + 12.0 - rng.range(0.0, 15.0)).min(99.0);
        res.recovered_by_kicking = rng.chance(0.4);
        res.duration = 2.5;
        if res.recovered_by_kicking {
            res.spot = 100.0 - res.spot;
        }
        return res;
    }
    let max = leg(punter, 17.0, 0.215, cond, along) + 14.0 + rng.normal(0.0, 4.5);
    // Placement: from midfield he aims inside the 10 rather than the end zone.
    let dist_goal = from + 15.0;
    let pin = from < 62.0;
    let acc = punter.a(Attr::KickAccuracy);
    let aim_land = if pin { 8.0 + rng.normal(0.0, 6.0 - 0.04 * acc) } else { from - max };
    let land_x = aim_land.max(from - max);
    let _ = dist_goal;
    let hang = clamp(
        3.5 + 0.018 * (punter.a(Attr::KickPower) - 60.0) + 0.006 * (acc - 60.0) + rng.normal(0.0, 0.25)
            - if pin { 0.2 } else { 0.0 },
        2.9,
        5.2,
    );
    let mut land_y = clamp(MID + rng.normal(0.0, 8.0), -1.0, WIDTH + 1.0);
    res.distance = from - land_x;
    res.hang = hang;
    // Out of bounds (coffin corner or a shank).
    if !(0.0..=WIDTH).contains(&land_y) || rng.chance(0.03 * (1.3 - punter.composure() / 20.0)) {
        res.out_of_bounds = true;
        land_y = land_y.clamp(0.0, WIDTH);
        let _ = land_y;
        res.spot = land_x.max(rules.touchback.min(1.0));
        if land_x <= 0.0 {
            res.touchback = true;
            res.spot = rules.touchback;
        }
        res.inside20 = res.spot <= 20.0 && !res.touchback;
        res.duration = hang;
        return res;
    }
    // Gunners against the jammers: how close coverage is when it lands.
    let gunner = k
        .kick_on
        .iter()
        .skip(1)
        .take(2)
        .map(|&i| &k.kick[i])
        .map(|p| 0.5 * p.a(Attr::Speed) + 0.25 * p.a(Attr::Agility) + 0.25 * p.a(Attr::Strength))
        .fold(0.0, f64::max);
    let jammer = k
        .ret_on
        .iter()
        .skip(1)
        .take(2)
        .map(|&i| &k.ret[i])
        .map(|p| 0.4 * p.a(Attr::ManCoverage) + 0.3 * p.a(Attr::Strength) + 0.3 * p.a(Attr::Speed))
        .fold(0.0, f64::max);
    let cover_gap = clamp(4.6 - 1.6 * (hang - 4.2) - 0.08 * (gunner - jammer) + rng.normal(0.0, 2.0), 0.0, 15.0);
    let returner = &k.ret[k.ret_on[0]];
    let judgment = returner.a(Attr::Awareness);
    // Inside his 10, a returner lets it go: it bounces.
    if land_x < 10.0 {
        let field = rng.chance(clamp(0.1 + 0.004 * (60.0 - judgment), 0.02, 0.4));
        if !field {
            let roll = rng.normal(2.0, 7.0);
            let end = land_x - roll;
            if end <= 0.0 {
                res.touchback = true;
                res.spot = rules.touchback;
            } else {
                // Downed by the coverage team.
                res.spot = end;
                res.inside20 = true;
            }
            res.distance = from - end.max(0.0);
            res.duration = hang + 1.5;
            return res;
        }
    }
    // Fair catch when coverage is on him.
    if cover_gap < 5.0 && rng.chance(clamp(0.95 - 0.004 * (judgment - 50.0), 0.7, 0.98)) || cover_gap < 1.5 {
        res.fair_catch = true;
        res.spot = land_x.max(1.0);
        res.inside20 = res.spot <= 20.0;
        res.duration = hang;
        res.returner = Some(returner.id);
        if muff(returner, cond, rng) {
            res.muffed = true;
            res.recovered_by_kicking = rng.chance(0.45);
            if res.recovered_by_kicking {
                res.spot = 100.0 - res.spot;
            }
        }
        return res;
    }
    if muff(returner, cond, rng) {
        res.muffed = true;
        res.returner = Some(returner.id);
        res.recovered_by_kicking = rng.chance(0.5);
        res.spot = if res.recovered_by_kicking { 100.0 - land_x } else { land_x };
        res.duration = hang + 1.0;
        return res;
    }
    let catch = v2(land_x, land_y.clamp(2.0, WIDTH - 2.0));
    run_return(
        k,
        catch,
        hang,
        0.0,
        from,
        rules,
        cond,
        crew,
        injury_scale,
        record,
        rng,
        &mut res,
        false,
        Some(cover_gap),
    );
    res.inside20 = res.spot <= 20.0;
    res
}

fn muff(p: &GamePlayer, cond: &Conditions, rng: &mut Rng) -> bool {
    let wet = if cond.rain || cond.snow { 2.0 } else { 1.0 };
    rng.chance(0.006 * wet * (1.5 - p.a(Attr::Catching) / 100.0) * (1.0 + 0.02 * cond.wind))
}

/// A tracked return (topic 7): the returner catches at `catch`, his
/// blockers set up, the coverage arrives from where it was when the ball
/// came down, and the return plays out like any run.
#[allow(clippy::too_many_arguments)]
fn run_return(
    k: &KickTeams,
    catch: V2,
    hang: f64,
    start_cover: f64,
    from: f64,
    rules: &Rules,
    cond: &Conditions,
    crew: &Crew,
    injury_scale: f64,
    record: bool,
    rng: &mut Rng,
    res: &mut KickResult,
    kickoff: bool,
    gap: Option<f64>,
) {
    let mut agents = Vec::with_capacity(22);
    let ret_id = k.ret[k.ret_on[0]].id;
    res.returner = Some(ret_id);
    res.returned = true;
    // Coverage has run (hang - start) seconds by the catch, in lanes.
    let run = (hang - start_cover).max(0.0);
    let cover_start = if kickoff && rules.nfl_kickoff { 40.0 } else { from - 1.0 };
    for (n, &i) in k.kick_on.iter().enumerate() {
        let p = &k.kick[i];
        let lane = 4.0 + (n as f64) * (WIDTH - 8.0) / 10.0;
        let speed = p.top_speed() * 0.9;
        let x = if n == 0 {
            // The kicker trails as the safety: under the 2025 NFL rule he
            // stays at the kick spot until the ball lands.
            if kickoff && rules.nfl_kickoff { from } else { catch.x + 30.0 + rng.range(0.0, 8.0) }
        } else if let Some(g) = gap {
            // A punt: the gunners are cover_gap away when it comes down,
            // the rest of the unit a few yards behind them in lanes.
            let behind = if n <= 2 { rng.range(0.0, 2.0) } else { 5.0 + rng.range(0.0, 8.0) };
            catch.x + g + behind
        } else {
            (cover_start - speed * run).max(catch.x + 5.0 + rng.range(0.0, 6.0))
        };
        let y = if gap.is_some() && (n == 1 || n == 2) {
            // Gunners close on the returner from either side.
            catch.y + if n == 1 { -4.5 } else { 4.5 } + rng.range(-2.0, 2.0)
        } else {
            lane + (catch.y - lane) * (1.0 - (x - catch.x).abs() / 40.0).clamp(0.0, 0.6)
        };
        let mut a = Agent::new(1, i, p, if n == 0 { "K" } else { "ST" }, v2(x, y), k.energy[0][i], 0.0);
        a.job = Job::Pursue;
        // Under the NFL's 2025 rules nobody moves until the ball lands.
        a.vel = if kickoff && rules.nfl_kickoff { V2::default() } else { v2(-speed.min(8.0), 0.0) };
        a.read_at = 0.0;
        agents.push(a);
    }
    for (n, &i) in k.ret_on.iter().enumerate() {
        let p = &k.ret[i];
        let pos = if n == 0 {
            catch
        } else {
            // The wall forms between him and the coverage.
            let lane = 6.0 + ((n - 1) as f64) * (WIDTH - 12.0) / 9.0;
            // NFL 2025: nine of the return team line up in the setup zone
            // (their 30 to 35) and may not move until the ball lands.
            let x = if kickoff && rules.nfl_kickoff {
                30.0 + rng.range(0.0, 5.0)
            } else {
                catch.x + 8.0 + rng.range(0.0, 8.0)
            };
            v2(x, lane)
        };
        let mut a = Agent::new(0, i, p, if n == 0 { "KR" } else { "ST" }, pos, k.energy[1][i], 0.0);
        a.job = if n == 0 { Job::Carrier } else { Job::Escort };
        agents.push(a);
    }
    let ret_idx = agents.len() - k.ret_on.len();
    let mut sim = Sim {
        teams: [k.ret, k.kick],
        agents: Vec::new(),
        eng: Vec::new(),
        ball: Ball::Held(0),
        t: 0.0,
        los: catch.x,
        rng,
        notes: Notes::default(),
        record: if record { Some(Vec::new()) } else { None },
        cond: *cond,
        crew: *crew,
        goal: 1.0,
        carrier_side: 0,
        dead: false,
        out_of_bounds: false,
        progress: catch.x,
        contact_force: 0.0,
        injury_scale,
        down_on_fall: rules.down_on_fall,
        fight: Vec::new(),
        fight_since: 0.0,
        screen: None,
        open_reach: 1.6,
    };
    // Reorder so the return team (side 0) comes first.
    let mut ret_agents: Vec<Agent> = agents.drain(ret_idx..).collect();
    ret_agents.extend(agents);
    sim.agents = ret_agents;
    sim.ball = Ball::Held(0);
    // Each cover man's lane: he stays in it until he is near the returner,
    // so the coverage doesn't bunch up and open a seam.
    let lanes: Vec<f64> =
        sim.agents.iter().map(|a| if a.side == 1 && a.label == "ST" { a.pos.y } else { f64::NAN }).collect();
    // The widest cover man on each side keeps contain: outside the returner.
    let mut contain = vec![0.0; lanes.len()];
    let widest = |f: &dyn Fn(f64, f64) -> bool| {
        let mut best: Option<usize> = None;
        for (i, &l) in lanes.iter().enumerate() {
            if !l.is_nan() && best.is_none_or(|b| f(l, lanes[b])) {
                best = Some(i);
            }
        }
        best
    };
    if let Some(i) = widest(&|a, b| a < b) {
        contain[i] = -1.0;
    }
    if let Some(i) = widest(&|a, b| a > b) {
        contain[i] = 1.0;
    }
    let ktrace = super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed);
    let mut buf = String::new();
    while sim.t < 12.0 && !sim.dead {
        sim.t += DT;
        if ktrace && ((sim.t / DT).round() as i64) % 5 == 0 {
            buf.push_str(&sim.snapshot());
            buf.push('\n');
        }
        sim.update_blocks(None, Some(0));
        for i in 1..sim.agents.len() {
            if !sim.agents[i].up(sim.t) || sim.agents[i].engaged.is_some() {
                continue;
            }
            match sim.agents[i].job {
                Job::Escort => escort(&mut sim, i),
                Job::Pursue => {
                    let (me, rp) = (sim.agents[i].pos, sim.agents[0].pos);
                    let ahead = me.x > rp.x + 1.0;
                    if contain[i] != 0.0 && ahead && me.dist(rp) > 4.0 {
                        let aim = v2(rp.x + 3.0, rp.y + contain[i] * 2.5);
                        let vm = sim.agents[i].vmax;
                        sim.steer(i, (aim - me).norm().scale(vm));
                    } else {
                        sim.pursue(i, 0);
                    }
                }
                _ => {}
            }
        }
        sim.run_carrier(0, None);
        sim.advance();
        sim.tackles(0);
        if !sim.dead {
            sim.check_carrier(0);
        }
        sim.record_frame();
        if super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed) && ((sim.t / DT).round() as i64) % 5 == 0 {
            sim.trace();
        }
    }
    sim.close_blocks();
    res.duration = hang + sim.t;
    let end = sim.progress;
    if let Some((side, _)) = sim.notes.recovered {
        res.recovered_by_kicking = side == 1;
        res.spot = if res.recovered_by_kicking { 100.0 - end } else { end };
    } else {
        res.spot = end;
    }
    if !res.recovered_by_kicking && end >= 100.0 {
        res.return_td = true;
    }
    if end <= 0.0 && !res.recovered_by_kicking {
        // Tackled in his own end zone after catching it there: a touchback.
        res.touchback = true;
        res.spot = rules.touchback;
    }
    res.return_yards = end - catch.x;
    if ktrace && res.return_yards >= 40.0 {
        eprintln!(
            "=== {} return {:.0} yds from {:.0}\n{buf}",
            if kickoff { "kickoff" } else { "punt" },
            res.return_yards,
            catch.x
        );
    }
    res.out_of_bounds = sim.out_of_bounds;
    res.notes = std::mem::take(&mut sim.notes);
    res.frames = sim.record.take();
}
