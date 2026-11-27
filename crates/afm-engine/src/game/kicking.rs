//! Special teams (topic 7): kickoffs, punts, field goals and extra points.
//! Kicks follow ball flight (leg, accuracy, wind, cold, altitude); snaps,
//! holds and protection are their own battles; returns are full tracked
//! plays. For kickers and punters, composure is the biggest factor.

use super::geom::{DT, V2, back_line, field, goal, mid, v2, width};
use super::roster::GamePlayer;
use super::rules::Rules;
use super::scrimmage::escort;
use super::sim::*;
use crate::attributes::Attr;
use crate::player::Level;
use crate::rng::{Rng, clamp};
use crate::tendencies::{Instruction, MID as MID_T, Tend};

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
    /// Who plays hurt, and with what (update 3, F7).
    pub hurt: [&'a [Option<super::sim::InjuryKind>]; 2],
    /// What the game does to habits (kicking team, return team).
    pub mood: super::sim::Mood,
    /// How much the kicking team's plan kicks to a returner who brings
    /// everything out (tendencies topic 4.4), 0-1.
    pub kick_deep: f64,
    /// The kicking team's plan for the kick (update 2, G18): kickoffs 1
    /// deep, 2 squib, 3 pooch; punts 1 directional, 2 coffin corner; 0 the
    /// staff's usual.
    pub style: u8,
    /// A surprise onside kick: the return team's front line left early.
    pub surprise: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
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
    /// A blocked punt played out (engine gaps item 12), in the kicking
    /// team's frame; the flow reads where it ended.
    #[serde(skip)]
    pub live: Option<LiveBlock>,
    /// An NFL kickoff that came down short of the landing zone (from 2024).
    #[serde(default)]
    pub short: bool,
}

/// How deep the punter stands behind the line (scrimmage formations too).
pub fn punt_depth(level: Level) -> f64 {
    match level {
        Level::NFL => 15.0,
        Level::HS => 12.0,
        _ => 14.0,
    }
}

/// The rush's call (engine gaps item 12): the share of punts, field goals
/// and extra points it rushes all out to block, the rest a return call or
/// a safe rush with men held back for a fake (proposed, not confirmed).
pub const PUNT_ALL_OUT: f64 = 0.15;

/// How far the punter steps up from his depth before the ball leaves his
/// foot (two steps), yards.
pub const PUNT_STEP: f64 = 2.5;

/// The officials on a return (bug 403): how much more often a beaten
/// blocker grabs than on a scrimmage down, on a kickoff and on a punt (the
/// jammers on the gunners), how much more often a blocker throws a block in
/// the back (cover men run past the return's blockers), and the share of
/// the personal fouls a scrimmage down draws that stand on a return; sized
/// to real NFL returned kicks in 2025 by kind (proposed, not confirmed).
pub const RET_GRAB_KICKOFF: f64 = 1.2;
pub const RET_GRAB_PUNT: f64 = 5.4;
pub const RET_IBB: f64 = 6.5;
pub const RET_PERSONAL: f64 = 0.6;
pub const XP_ALL_OUT: f64 = 0.15;

/// The rush's all-out share on a field goal: short kicks are near the goal,
/// where a fake can score, so the rush keeps men home; on a long one a fake
/// is rare and it sends everyone (proposed, not confirmed).
pub fn fg_all_out(distance: f64) -> f64 {
    clamp(0.3 + 0.025 * (distance - 35.0), 0.03, 0.7)
}

/// The share of kickoffs a kicker of typical technique mis-hits (EG5;
/// proposed, not confirmed).
const MISHIT: f64 = 0.039;

/// High snaps in the wet (individual practice): in rain or snow the
/// snapper's skill reads the flaw lower (five rating points a step).
fn wet_snaps(snapper: &GamePlayer, cond: &Conditions, level: Level) -> f64 {
    if cond.rain || cond.snow { snapper.craft_vs(crate::craft::Craft::WetSnaps, level, 0.0) / 5.0 } else { 0.0 }
}

/// A kicker's accuracy on a field goal of this distance: short kick
/// accuracy to 30 yards, long from 50, blended between (one kicker is
/// lethal short, another long; Raichel, topic 7), so there is no cliff at 40.
pub fn fg_accuracy(kicker: &GamePlayer, distance: f64) -> f64 {
    let t = clamp((distance - 30.0) / 20.0, 0.0, 1.0);
    // Plant foot drift (individual practice): the long kicks miss more.
    let drift = kicker.craft.pts(crate::craft::Craft::PlantDrift, 0.0);
    (1.0 - t) * kicker.a(Attr::ShortKickAccuracy) + t * (kicker.a(Attr::LongKickAccuracy) + drift)
}

/// How far his leg carries a place kick today (yards in the air): his
/// Kick Power in the day's air, rain and wind along the kick's line. The
/// kick rolls about it (3 yards either way at one standard deviation), and
/// his staff reads his range from it (engine gaps item 14).
pub fn fg_reach(kicker: &GamePlayer, cond: &Conditions, along: f64) -> f64 {
    leg(kicker, Attr::KickPower, 40.0, 0.3, cond, along)
}

/// A place kick's miss spread in degrees before the snap (engine gaps
/// items 11 and 14), from the distance, his accuracy `acc` there
/// (`fg_accuracy`), how hard he must swing to get it there and the wind.
/// A kick near the end of his leg takes his full swing, and a full swing
/// sprays: the spread grows with the square of the share of his reach the
/// kick needs past `KICK_EASY` (real NFL kickers make 93% from 30-39 and
/// 60% from 55 on; FBS kickers 81% and 51% from 50 on). The kick rolls on
/// it with his consistency, the moment and his season's form on top, and
/// his staff's make estimate (`flow::fg_make_in`) reads the same number.
/// Sizes proposed, not confirmed.
pub fn fg_spread(distance: f64, acc: f64, reach: f64, cond: &Conditions) -> f64 {
    let effort = distance / reach.max(1.0);
    let strain = 1.0 + KICK_STRAIN * (effort - KICK_EASY).max(0.0).powi(2);
    (2.6 + 0.018 * (distance - 35.0).max(0.0)) * (2.681 - 0.02066 * acc).max(0.55) * strain * (1.0 + 0.012 * cond.wind)
}

/// How much a full swing widens a place kick (`fg_spread`), and the share
/// of his reach a kick can need before it starts to. Proposed, not
/// confirmed.
pub const KICK_STRAIN: f64 = 28.0;
pub const KICK_EASY: f64 = 0.8;

/// Kick distance for a leg (yards in the air), with weather and altitude:
/// kick power for place kicks, punt power for punts.
fn leg(kicker: &GamePlayer, power: Attr, base: f64, per: f64, cond: &Conditions, along: f64) -> f64 {
    let kp = kicker.a(power);
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

/// A kicker's form for a season (minor bug 334): his miss spread that year
/// against his usual, a keyed draw on his id and the season, about 12% up
/// or down at one standard deviation. Real kickers' seasons swing far more
/// than their skill does: their make rate over the league's under 40 yards
/// carries 0.15 from one season to the next, and 0.06 from 40 yards on
/// (nflverse 2018-25, `results_real.json`). Without it the shipped seed 41
/// world carried 0.37 under 40 (2021-25, kickers with 30 or more tries and
/// extra points), its spread 0.028 against real's 0.034. A 12% swing adds
/// about 0.022 to a season's make rate at a short kick's 2.1 standard
/// deviations of aim, which works out to about 0.23 and 0.036 (worked from
/// the world's season rows, not yet seen in a new world). The mean is held.
/// Proposed, not confirmed.
pub fn season_form(id: u32, year: i32) -> f64 {
    let z = crate::world::season::key_normal(0x4B1C_F0E3 ^ ((year as u64) << 20), id as u64);
    (KICK_FORM_SD * z - KICK_FORM_SD * KICK_FORM_SD / 2.0).exp()
}

/// The spread of a kicker's season form, in the log of his miss spread
/// ([`season_form`]). Proposed, not confirmed.
pub const KICK_FORM_SD: f64 = 0.12;

/// Composure under pressure (topic 7): how much a big moment widens his miss.
fn pressure_factor(kicker: &GamePlayer, pressure: f64, level: Level) -> f64 {
    let c = kicker.composure(level);
    1.0 + pressure * clamp(1.15 - c / 15.0, 0.0, 1.0) * 0.9
}

/// The hold's depth behind the line of scrimmage on a place kick.
pub const HOLD_DEPTH: f64 = 7.0;

/// The protection on a place kick or a punt and the rush against it
/// (engine gaps item 12), as player indexes into each team.
pub struct KickLine<'a> {
    pub kick: &'a [GamePlayer],
    pub rush: &'a [GamePlayer],
    /// The protection: the long snapper first, then the wall (a place
    /// kick), or the line and the shield (a punt).
    pub protect: &'a [usize],
    pub rushers: &'a [usize],
    /// The rush's call: all out for the block, an overload off one edge.
    pub all_out: bool,
}

/// A blocked kick (engine gaps item 12): the rusher who got a hand on it
/// (an index into `KickLine::rushers`), where (yards in front of the spot
/// it was kicked from, and across from the ball's line), and how far each
/// rusher got (yards past the line of scrimmage).
#[derive(Clone, Debug, Default)]
pub struct Block {
    pub by: usize,
    /// How he got there: 0 pushing the middle back, 1 off the edge, 2 the
    /// overload's free man, 3 through a punt's protection.
    pub how: u8,
    pub at: f64,
    pub across: f64,
    pub depth: Vec<f64>,
    /// Where each rusher is across the field from the ball's line.
    pub lane: Vec<f64>,
}

/// Seconds a man needs to run `l` yards from a standing start, on his top
/// speed and the build-up of his sprint (`GamePlayer::sprint_tau`).
fn sprint_time(p: &GamePlayer, l: f64) -> f64 {
    let (v, tau) = (p.top_speed().max(1.0), p.sprint_tau().max(0.05));
    let mut t = l / v + tau;
    for _ in 0..5 {
        let e = (-t / tau).exp();
        let f = v * (t - tau + tau * e) - l;
        t = (t - f / (v * (1.0 - e)).max(0.1)).max(0.05);
    }
    t
}

/// How high a man gets a hand at a kick, in yards: his shoulder height and
/// arm, and the spring he has on a lunge in traffic (Jumping: 4 inches at 40,
/// about 19 at 99). Proposed, not confirmed.
fn kick_reach(p: &GamePlayer) -> f64 {
    (0.818 * p.height + 1.05 * p.arm + 4.0 + 0.25 * (p.a(Attr::Jumping) - 40.0).max(0.0)) / 36.0
}

/// The rush against a place kick (engine gaps item 12), man by man against
/// the clock: the snap, the hold and his approach take `op` seconds. The
/// wall stands shoulder to shoulder, so nobody runs through the middle: the
/// two men over the snapper's shoulders push it back, each by his power
/// against the snapper's and the guard's anchor, and get a hand up as the
/// kick goes over. Whether it clears them is the ball's height there: its
/// launch reads his Kick Trajectory, a long kick he must drive goes lower,
/// and a mis-hit, a bad snap or a bad hold lower still. Off the edge each
/// wing takes the man outside him; one who beats him clean, or the extra
/// man of an overload, runs at the spot a yard in front of the kicker and
/// blocks it if he is there first. `hold` is the hold's depth behind the
/// line. Every size is proposed, not confirmed.
#[allow(clippy::too_many_arguments)]
fn place_block(
    line: &KickLine,
    kicker: &GamePlayer,
    snapper: &GamePlayer,
    distance: f64,
    hold: f64,
    bad: bool,
    rng: &mut Rng,
) -> Option<Block> {
    let n = line.rushers.len();
    if n == 0 || line.protect.is_empty() {
        return None;
    }
    let (ps, rs) = (line.kick, line.rush);
    let op = 1.32 - 0.0025 * (snapper.a(Attr::LongSnapping) - 60.0) - 0.003 * (kicker.a(Attr::GetOff) - 60.0)
        + rng.normal(0.0, 0.04)
        + if bad { 0.35 } else { 0.0 };
    let theta = 34.0 + 0.2 * (kicker.a(Attr::KickTrajectory) - 60.0) - 0.37 * (distance - 35.0) + rng.normal(0.0, 3.0)
        - if bad { 5.0 } else { 0.0 };
    let tan = theta.max(5.0).to_radians().tan();
    let height = |s: f64| 0.25 + s.max(0.0) * tan;
    let power = |p: &GamePlayer| 0.4 * p.a(Attr::Strength) + 0.35 * p.a(Attr::PowerRush) + 0.25 * p.a(Attr::LowerBody);
    let anchor =
        |p: &GamePlayer| 0.4 * p.a(Attr::Strength) + 0.35 * p.a(Attr::PassBlocking) + 0.25 * p.a(Attr::LowerBody);
    let snap = &ps[line.protect[0]];
    // The wall from the middle out by weight: the guards, the tackles, the
    // ends, and the two lightest outside as the wings.
    let mut wall: Vec<usize> = line.protect[1..].to_vec();
    wall.sort_by(|&a, &b| ps[b].weight.total_cmp(&ps[a].weight));
    // The rush: its two heaviest over the snapper's shoulders, its two
    // fastest of the rest off the edges, the others in the gaps between.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| rs[line.rushers[b]].weight.total_cmp(&rs[line.rushers[a]].weight));
    let inside: Vec<usize> = order.iter().take(2).copied().collect();
    let mut rest: Vec<usize> = order[inside.len()..].to_vec();
    rest.sort_by(|&a, &b| rs[line.rushers[b]].top_speed().total_cmp(&rs[line.rushers[a]].top_speed()));
    let mut depth = vec![0.4; n];
    let mut lane = vec![0.0; n];
    let wide = 4.6;
    for (k, &j) in rest.iter().enumerate() {
        lane[j] = match k {
            0 => -wide,
            1 => wide,
            _ => [-1.5, 1.5, -2.5, 2.5, -3.4, 3.4][(k - 2) % 6],
        };
    }
    let mut best: Option<(f64, usize, f64, f64, u8)> = None;
    for (g, &j) in inside.iter().enumerate() {
        let r = &rs[line.rushers[j]];
        let guard = wall.get(g).map_or(snap, |&i| &ps[i]);
        let hold_up = 0.5 * (anchor(snap) + anchor(guard));
        let push = 0.5 * (op - 0.2).max(0.0) * ((power(r) - hold_up) / 15.0).exp() * rng.normal(0.0, 0.35).exp();
        let push = push.min(hold - 1.0);
        let across = if g == 0 { -0.5 } else { 0.5 };
        depth[j] = push;
        lane[j] = across;
        let margin = kick_reach(r) - height(hold - push);
        if margin > 0.0 && best.is_none_or(|b| margin > b.0) {
            best = Some((margin, j, hold - push, across, 0));
        }
    }
    // Off the edges: the wings are the two lightest of the wall.
    let wings: Vec<usize> = wall.iter().rev().take(2).copied().collect();
    let run_in = |r: &GamePlayer, w: f64| {
        let l = ((hold - 1.0).powi(2) + w * w).sqrt() - (0.9 + r.arm / 36.0);
        0.25 - 0.002 * (r.a(Attr::Acceleration) - 60.0) + sprint_time(r, l.max(0.5))
    };
    let edges = rest.len().min(2);
    // An all-out call overloads one edge: its extra man outside the wing
    // makes it two on one there.
    let over_side = if line.all_out && rng.chance(0.5) { -1.0 } else { 1.0 };
    for (e, &j) in rest.iter().take(edges).enumerate() {
        let r = &rs[line.rushers[j]];
        let wing = wings.get(e).map_or(snap, |&i| &ps[i]);
        let rush = 0.4 * r.a(Attr::Speed) + 0.3 * r.a(Attr::Acceleration) + 0.3 * r.a(Attr::FinesseRush);
        let stop = 0.5 * wing.a(Attr::PassBlocking) + 0.3 * wing.a(Attr::Agility) + 0.2 * wing.a(Attr::Speed);
        let two_on_one = if line.all_out && lane[j].signum() == over_side { 25.0 } else { 1.0 };
        if !rng.chance(0.0015 * two_on_one * ((rush - stop) / 10.0).exp()) {
            continue;
        }
        let t = run_in(r, wide);
        depth[j] = (hold - 1.0) * (op / t).min(1.0);
        if t < op + 0.04 && best.is_none_or(|b| b.0 < 1.0) {
            best = Some((1.0, j, 1.0, lane[j].signum() * 0.6, 1));
        }
    }
    // An overload: the call's extra man off one edge, outside the wing.
    if line.all_out
        && let Some(&j) = rest.get(2)
    {
        let r = &rs[line.rushers[j]];
        let w = 6.2;
        let side = over_side;
        lane[j] = side * w;
        let t = run_in(r, w);
        depth[j] = (hold - 1.0) * (op / t).min(1.0);
        if t < op + 0.04 && best.is_none_or(|b| b.0 < 1.0) {
            best = Some((1.0, j, 1.0, side * 0.6, 2));
        }
    }
    best.map(|(_, by, at, across, how)| Block { by, how, at, across, depth, lane })
}

/// The rush against a punt (engine gaps item 12), man by man against the
/// clock: the snap and his catch to his kick take `op` seconds, and he kicks
/// `deep` yards behind the line. Each rusher takes on the man in front of
/// him, and one who beats him clean is through; the three in the shield
/// pick up who comes through, one each; a man still free runs at the spot of
/// the kick and blocks it if he is there first, diving at the ball off the
/// foot. On a return call half the front only holds its man up. Every size
/// is proposed, not confirmed.
fn punt_block(line: &KickLine, op: f64, deep: f64, rng: &mut Rng) -> Option<Block> {
    let n = line.rushers.len();
    if n == 0 || line.protect.is_empty() {
        return None;
    }
    let (ps, rs) = (line.kick, line.rush);
    let mut men: Vec<usize> = line.protect.to_vec();
    let snap = men.remove(0);
    men.sort_by(|&a, &b| ps[b].weight.total_cmp(&ps[a].weight));
    let shield: Vec<usize> = men.split_off(men.len().min(4));
    let front: Vec<usize> = std::iter::once(snap).chain(men).collect();
    let mut depth = vec![0.4; n];
    let lane: Vec<f64> = (0..n).map(|j| -4.0 + 8.0 * j as f64 / (n.max(2) - 1) as f64).collect();
    let mut free = Vec::new();
    for j in 0..n {
        if !line.all_out && j % 2 == 1 {
            continue;
        }
        let r = &rs[line.rushers[j]];
        let man = &ps[front[j * front.len() / n]];
        let rush = 0.35 * r.a(Attr::Speed)
            + 0.35 * r.a(Attr::FinesseRush).max(r.a(Attr::PowerRush))
            + 0.3 * r.a(Attr::Acceleration);
        let stop = 0.5 * man.a(Attr::PassBlocking) + 0.3 * man.a(Attr::Agility) + 0.2 * man.a(Attr::Strength);
        if rng.chance(0.0075 * ((rush - stop) / 10.0).exp()) {
            free.push(j);
        }
    }
    let mut pick = shield.len();
    free.retain(|&j| {
        let r = &rs[line.rushers[j]];
        let take = pick > 0 && {
            let s = &ps[shield[shield.len() - pick]];
            rng.chance(clamp(0.9 + 0.004 * (s.a(Attr::PassBlocking) - r.a(Attr::FinesseRush)), 0.5, 0.99))
        };
        if take {
            pick -= 1;
            depth[j] = 5.0;
        }
        !take
    });
    let mut best: Option<(f64, usize)> = None;
    for j in free {
        let r = &rs[line.rushers[j]];
        let ahead = deep - PUNT_STEP;
        let l = (ahead * ahead + lane[j] * lane[j]).sqrt() - (1.4 + r.arm / 36.0);
        let t = 0.25 - 0.002 * (r.a(Attr::Acceleration) - 60.0) + sprint_time(r, l.max(0.5));
        depth[j] = deep * (op / t).min(1.0);
        if t < op + 0.03 && best.is_none_or(|b| t < b.0) {
            best = Some((t, j));
        }
    }
    best.map(|(_, by)| Block { by, how: 3, at: 0.7, across: lane[by].signum() * 0.4, depth, lane })
}

/// A blocked kick played out (engine gaps item 12, on item 4's loose ball):
/// how it ended in the kicking team's frame (its goal line at 0), whether it
/// was recovered past the line of scrimmage, the side that touched it last,
/// the man who came up with it and how far he ran, and the play's time and
/// notes.
#[derive(Clone, Debug)]
pub struct LiveBlock {
    pub end: LooseEnd,
    pub past: bool,
    pub touched: usize,
    pub by: Option<u32>,
    /// Where he came up with it (the kicking team's frame).
    pub rec_x: Option<f64>,
    pub ran: f64,
    pub duration: f64,
    pub notes: Notes,
    pub frames: Option<Vec<Frame>>,
}

/// Where everyone is when a kick is blocked (engine gaps item 12), in the
/// kicking team's frame: each side's men (player index and spot), the
/// blocker (an index into the second side's men), the ball and the line.
pub struct LiveSetup<'a> {
    pub teams: [&'a [GamePlayer]; 2],
    pub men: [Vec<(usize, V2)>; 2],
    pub blocker: usize,
    pub ball: V2,
    pub punt: bool,
    /// A try: the book may end it when the defense has the ball (`defense_ends`).
    pub try_ends: bool,
    pub energy: [&'a [f64]; 2],
    pub hurt: [&'a [Option<super::sim::InjuryKind>]; 2],
    pub mood: super::sim::Mood,
    pub los: f64,
}

/// A blocked kick is a live ball (engine gaps item 12, on item 4's loose
/// ball): it comes off the hand that blocked it, back toward the kicking
/// team's goal or out to the side (a punt off the foot harder), now and
/// then on past the line, and everyone goes for it. Behind the line either
/// side may pick it up and run; the kicking team falls on it. Past the
/// line it is a kick again: the receiving team may run it back, and the
/// kicking team only downs it (the flow reads `past` and `touched`).
pub fn blocked_live(
    set: &LiveSetup,
    rules: &Rules,
    cond: &Conditions,
    crew: &Crew,
    record: bool,
    rng: &mut Rng,
) -> LiveBlock {
    let mut agents = Vec::with_capacity(22);
    for side in 0..2 {
        for (n, &(i, pos)) in set.men[side].iter().enumerate() {
            let p = &*super::sim::hampered(&set.teams[side][i], set.hurt[side].get(i).copied().flatten());
            let label = if side == 0 && n == 0 { "K" } else { "ST" };
            let mut a = Agent::new(side, i, p, label, pos, set.energy[side][i], 0.0);
            a.job = Job::Pursue;
            agents.push(a);
        }
    }
    let k0 = set.men[0].len();
    let ang = if rng.chance(if set.punt { 0.04 } else { 0.15 }) {
        rng.normal(0.0, 0.5)
    } else {
        std::f64::consts::PI + rng.normal(0.0, if set.punt { 0.8 } else { 1.1 })
    };
    let speed = if set.punt { rng.range(7.0, 16.0) } else { rng.range(3.0, 12.0) };
    let mut sim = kick_sim(set.teams, rng, rules, cond, crew, 1.0, record, set.mood, set.los);
    sim.agents = agents;
    sim.progress = set.ball.x;
    sim.fumble_rule = FumbleSnap { book: rules.fumble_rule(), fumbler_only: false, defense_ends: set.try_ends };
    sim.loose = Some(Loose {
        vel: v2(ang.cos(), ang.sin()).scale(speed),
        at: set.ball,
        side: 0,
        by: 0,
        how: super::sim::FumbleHow::Hit,
        since: 0.0,
        bounce: 0.25,
        // The block's own touch behind the line does not count: past the
        // line the ball is the kicking team's kick until a receiver touches it.
        touched: 0,
        rec: None,
        out: None,
    });
    sim.ball = Ball::Loose(set.ball);
    // The man who blocked it knows where it went.
    if let Some(a) = sim.agents.get_mut(k0 + set.blocker) {
        a.last_try = -1.0;
    }
    while sim.t < 12.0 && !sim.dead {
        sim.t += DT;
        sim.link_speeds();
        if !live_tick(&mut sim) {
            break;
        }
    }
    sim.close_blocks();
    // The play's work, as on a return: time in blocks, and strains and
    // plants with nobody touching him (engine gaps items 1 and 2).
    sim.wear();
    let l = sim.loose;
    // Out of bounds in the field of play: the kicking team's ball where it
    // went out behind the line, the receiving team's past it (a kick).
    let end = match l.and_then(|l| l.out) {
        Some(o) if o.x > 0.0 && o.x < goal() => LooseEnd::Ball { side: usize::from(o.x > set.los), x: o.x },
        _ => sim.loose_end().unwrap_or(LooseEnd::Ball { side: 0, x: set.ball.x }),
    };
    let past = l.and_then(|l| l.rec).is_some_and(|(_, at)| at.x > set.los)
        || l.and_then(|l| l.out).is_some_and(|o| o.x > set.los);
    let touched = l.map_or(1, |l| l.touched);
    let (by, ran) = match l.and_then(|l| l.rec) {
        Some((w, at)) => {
            let dir = if sim.agents[w].side == 0 { 1.0 } else { -1.0 };
            let to = match end {
                LooseEnd::Ball { x, .. } => x,
                LooseEnd::Touchdown(0) => goal(),
                LooseEnd::Touchdown(_) => 0.0,
                _ => sim.progress,
            };
            (Some(sim.agents[w].id), ((to - at.x) * dir).max(0.0))
        }
        None => (None, 0.0),
    };
    let rec_x = l.and_then(|l| l.rec).map(|(_, at)| at.x);
    LiveBlock {
        end,
        past,
        touched,
        by,
        rec_x,
        ran,
        duration: sim.t,
        notes: std::mem::take(&mut sim.notes),
        frames: sim.record.take(),
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FieldGoal {
    pub made: bool,
    pub blocked: bool,
    /// Who blocked it and where (engine gaps item 12).
    #[serde(skip)]
    pub block: Option<Block>,
    pub distance: f64,
    pub bad_snap: bool,
    pub why: String,
    /// How far the kick carried, yards from the spot of the hold (0 when
    /// it never got off): a short one comes down in front of the posts.
    #[serde(default)]
    pub carry: f64,
}

/// A field goal or extra point try from `distance` yards (spot of the hold
/// plus ten for the end zone). `pressure` is 0-1: the game on the line, an
/// icing timeout, a playoff. `form` is his season's form ([`season_form`]).
#[allow(clippy::too_many_arguments)]
pub fn field_goal(
    kicker: &GamePlayer,
    snapper: &GamePlayer,
    holder: &GamePlayer,
    line: &KickLine,
    distance: f64,
    pressure: f64,
    form: f64,
    cond: &Conditions,
    along: f64,
    level: Level,
    rng: &mut Rng,
) -> FieldGoal {
    // The snap and the hold are their own battles (topic 7).
    let snap_skill = snapper.on20(Attr::LongSnapping) + wet_snaps(snapper, cond, level);
    let bad_snap = rng.chance(0.004 * (1.0 + (10.0 - snap_skill).max(-5.0) * 0.18));
    let hold = holder.a(Attr::Catching) * 0.5 + holder.a(Attr::Concentration) * 0.5;
    let bad_hold = !bad_snap && rng.chance(0.004 * (1.6 - hold / 100.0));
    let mut fg =
        FieldGoal { made: false, blocked: false, block: None, distance, bad_snap, why: String::new(), carry: 0.0 };
    if bad_snap && rng.chance(0.6) {
        fg.why = format!("The snap got away from holder #{}.", holder.id);
        return fg;
    }
    // The rush against the wall and off the edges, man by man against the
    // operation's clock (engine gaps item 12, `place_block`), from the hold
    // seven yards behind the line.
    if let Some(b) = place_block(line, kicker, snapper, distance, HOLD_DEPTH, bad_snap || bad_hold, rng) {
        fg.blocked = true;
        let yards = format!("{distance:.0}");
        let by = line.rush[line.rushers[b.by]].id;
        fg.why = match b.how {
            0 => format!(
                "Blocked: #{by} pushed the middle back and got a hand on {}-yard try.",
                crate::text::with_article(&yards)
            ),
            _ => format!(
                "Blocked: #{by} came free off the edge and got a hand on {}-yard try.",
                crate::text::with_article(&yards)
            ),
        };
        fg.block = Some(b);
        return fg;
    }
    // Plant foot drift read against the level's typical kicker (a coach's
    // estimate, `fg_accuracy`, reads his own).
    let t = clamp((distance - 30.0) / 20.0, 0.0, 1.0);
    let acc = fg_accuracy(kicker, distance)
        - t * super::synthetic::typical_craft(level, kicker.position, crate::craft::Craft::PlantDrift);
    // Spread in degrees: an average NFL kicker misses by about 2.5 degrees,
    // more near the end of his leg (`fg_spread`).
    let reach_today = fg_reach(kicker, cond, along);
    let mut sd = fg_spread(distance, acc, reach_today, cond);
    sd *= 1.18 - 0.018 * kicker.traits.consistency;
    sd *= pressure_factor(kicker, pressure, level);
    sd *= form;
    if bad_snap || bad_hold {
        sd *= 1.6;
    }
    let angle = sd * rng.normal(0.0, 1.0);
    let lateral = distance * angle.to_radians().tan();
    let reach = reach_today + rng.normal(0.0, 3.0);
    let wide = lateral.abs() > field().posts;
    let short_of = reach < distance;
    fg.made = !wide && !short_of;
    fg.carry = reach;
    let composure = kicker.composure(level);
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
    let from = goal() - rules.kickoff_from;
    let mut res = KickResult::default();
    if onside {
        // A short kick and a scramble for it; the hands team waits for it.
        // The onside touch (individual practice).
        let acc =
            kicker.a(Attr::KickoffTechnique) + kicker.craft_vs(crate::craft::Craft::OnsideTouch, rules.level, 0.0);
        let p = clamp(0.08 + 0.0025 * (acc - 50.0) + if rules.nfl_kickoff { -0.02 } else { 0.03 }, 0.03, 0.25);
        // A true surprise is recovered about half the time (real NFL
        // 2009-2023, unexpected onside kicks).
        let p = if k.surprise { clamp(0.5 + 0.003 * (acc - 60.0), 0.35, 0.65) } else { p };
        res.distance = 11.0 + rng.range(0.0, 3.0);
        let spot = from - res.distance;
        res.spot = spot;
        res.recovered_by_kicking = rng.chance(p);
        res.duration = 2.5;
        return res;
    }
    // A real NFL kickoff leg carries about 70 yards (before 2024 most went
    // five to nine yards deep from the 35); a college leg a few less, a high
    // school one far less (proposed, not confirmed: was 44 + 0.23 per point,
    // which put almost no kick in the end zone at any level, then 49, which
    // left NFL touchbacks at 14% under the 2025 rule, real 22%, and 45%
    // before 2024, real about 55%; minor bug 139).
    let max = leg(kicker, Attr::KickPower, 55.0, 0.23, cond, along) + rng.normal(0.0, 3.0);
    // Where he aims: under the NFL's 2025 rule a kick into the end zone is a
    // touchback to the 35, so most kickers drop it in the landing zone and
    // make the return team bring it out (real 2025: 75% returned). A
    // dangerous returner gets kicked away from more often (proposed, not
    // confirmed). Elsewhere a big leg aims for a touchback.
    // Against a returner who brings everything out, the plan kicks it to
    // land just inside the goal line and lets him (tendencies topic 4.4).
    let deep = k.kick_deep > 0.0 && rng.chance(0.6 * k.kick_deep);
    let aim = if deep && from - max < -1.0 {
        -1.0 - rng.range(0.0, 3.0)
    } else if rules.nfl_kickoff {
        let r = &k.ret[k.ret_on[0]];
        // Read for his weight, as the forty is (A3).
        let danger = 0.5 * (r.a(Attr::Speed) + r.a(Attr::Elusiveness)) - super::sim::size_cost(r.weight, 200.0) - 80.0;
        let away = clamp(0.2 + 0.012 * danger, 0.08, 0.45);
        if from - max < -1.0 && rng.chance(away) { -2.0 - rng.range(0.0, 5.0) } else { 2.0 + rng.range(0.0, 8.0) }
    } else if from - max < -2.0 {
        -5.0
    } else {
        from - max
    };
    // Technique puts it where he wants it, with hang to let the coverage
    // arrive (proposed, not confirmed).
    let tech = kicker.a(Attr::KickoffTechnique);
    let spread = clamp(1.0 - 0.01 * (tech - 60.0), 0.6, 1.4);
    // Your plan's kickoff (update 2, G18): a squib skips along the ground
    // to the up-backs about the 25 (a ground ball: no fair catch, fielded
    // after its bounces with the coverage on top of it); under the NFL's
    // landing-zone rule it is a line drive that bounces in the zone (short
    // of it the ball goes to the 40). A pooch hangs high to about the 12
    // (a fair catch or a short return). Proposed, not confirmed.
    let aim = match k.style {
        2 if rules.nfl_kickoff => (from - max).max(12.0 + rng.range(0.0, 6.0)),
        2 => (from - max).max(24.0 + rng.range(0.0, 8.0)),
        3 => (from - max).max(10.0 + rng.range(0.0, 7.0)),
        _ => aim,
    };
    // A mis-hit (EG5, item 28): now and then he catches it wrong and it
    // carries little more than half its usual, by his technique. Under the
    // NFL's landing zone one that comes down short of it goes to the 40
    // (real 2025: 0.079 a team game, about 1 kickoff in 60); elsewhere it
    // is a short kick to return. Proposed, not confirmed.
    let mishit = rng.chance(MISHIT * clamp(1.4 - tech / 100.0, 0.4, 1.2));
    let land_x =
        if mishit { from - max * rng.range(0.45, 0.7) } else { (from - max).max(aim) + rng.normal(0.0, 2.5 * spread) };
    let land_y = clamp(mid() + rng.normal(0.0, 7.0), 2.0, width() - 2.0);
    let hang = clamp(
        3.4 + 0.014 * (kicker.a(Attr::KickPower) - 60.0) + 0.006 * (tech - 60.0) + rng.normal(0.0, 0.2),
        2.8,
        4.8,
    );
    let hang = match k.style {
        2 => 1.8 + rng.range(0.0, 0.4),
        3 => hang + 0.5,
        _ => hang,
    };
    res.distance = from - land_x;
    res.hang = hang;
    // Out of bounds: rare; the ball goes where the level's rule puts it
    // (the NFL's 40, college's 35 from the 35, high school's 35 from the
    // 40: `fouls::kick_out_spot`, engine gaps lane EG5, item 25).
    // (0.006 before EG5 booked it: 1.7% of the test league's kickoffs, real
    // NFL 2025 1.4%, 0.072 flagged a team game; proposed, not confirmed.)
    if rng.chance(0.0042 * (1.4 - kicker.a(Attr::KickoffTechnique) / 100.0)) {
        res.out_of_bounds = true;
        res.spot = super::fouls::kick_out_spot(rules, rules.kickoff_from, goal());
        res.duration = hang;
        return res;
    }
    if rules.nfl_kickoff && land_x > 20.0 {
        // Short of the landing zone.
        res.short = true;
        res.spot = rules.code.short_kick_spot;
        res.duration = hang;
        return res;
    }
    // Arena football: a kick into the rebound nets stays live and comes
    // back into the end zone, and the returner brings it out.
    let land_x =
        if rules.code.arena_kickoff && land_x < back_line() + 0.5 { back_line() + rng.range(1.0, 4.0) } else { land_x };
    if land_x < back_line() + 0.5 {
        res.touchback = true;
        res.spot = rules.kickoff_touchback;
        return res;
    }
    // The returner's call: take a knee deep in the end zone, or bring it out.
    let returner = k.ret_on[0];
    let rp = &k.ret[returner];
    let judgment = rp.a(Attr::Decisions);
    let tb_value = rules.kickoff_touchback;
    // Returner risk (tendencies): a risk-taker believes he will get more
    // out of it than he will. An order to kneel past a depth decides it
    // when he follows it.
    let level = rules.level;
    let risk = rp.tend_now(Tend::ReturnRisk, k.mood.gamble[1], k.mood.stakes[1], k.energy[1][returner], level);
    let ordered = rp.orders.kneel_depth().filter(|_| rp.follows(k.mood.strict[1], level, rng));
    // Following the order, he weighs the kick at its returner risk too
    // (update 3, F17: `Instruction::sets`), not his own.
    let risk = if ordered.is_some() { rp.orders.sets(Tend::ReturnRisk).unwrap_or(risk) } else { risk };
    // The order only adds kneels: deeper than his depth he takes the knee
    // whatever he thinks of his chances.
    let kneel = land_x < 0.0 && !rules.code.arena_kickoff && {
        // He weighs where he thinks he will get to against the touchback;
        // good judgment makes it a sharper call.
        let expected = land_x + 22.0 + 0.1 * (rp.a(Attr::Speed) - 70.0) + 0.9 * (risk - MID_T);
        let width = clamp(5.5 - 0.04 * judgment, 1.5, 4.5);
        let own = !rng.chance(1.0 / (1.0 + ((tb_value - expected) / width).exp()));
        own || ordered.is_some_and(|d| -land_x > d)
    };
    let mut res_deep = None;
    if land_x < 0.0 {
        res_deep = Some(!kneel);
    }
    res.notes.tend.deep_kick = res_deep;
    if kneel {
        res.touchback = true;
        res.spot = tb_value;
        res.duration = hang;
        return res;
    }
    // A squib is a ground ball: it skips on a few yards and an up-back
    // fields it on the bounce, about 3.5 to 4 seconds after the kick, with
    // the coverage running all that time (under the NFL's rule they go
    // when it hits in the zone). Nobody fair-catches a bouncing ball.
    if k.style == 2 && land_x > 0.0 {
        let field_x = (land_x - rng.range(2.0, 6.0)).max(1.0);
        let fielded = 3.5 + rng.range(0.0, 0.5);
        let mut ret_on = k.ret_on.clone();
        if ret_on.len() > 2 {
            // The up-back in front of the deep men fields it.
            let up = ret_on.remove(2);
            ret_on.insert(0, up);
        }
        let up = KickTeams {
            kick: k.kick,
            ret: k.ret,
            kick_on: k.kick_on.clone(),
            ret_on,
            snapper: k.snapper,
            holder: k.holder,
            energy: k.energy,
            hurt: k.hurt,
            mood: k.mood,
            kick_deep: k.kick_deep,
            style: k.style,
            surprise: k.surprise,
        };
        let catch = v2(field_x, land_y);
        let start_cover = if rules.nfl_kickoff { hang } else { 0.0 };
        run_return(
            &up,
            catch,
            fielded,
            start_cover,
            from,
            rules,
            cond,
            crew,
            injury_scale,
            record,
            rng,
            &mut res,
            true,
            None,
        );
        return res;
    }
    // A fair catch inside the 25 is a touchback: college from 2018, the NFL
    // in 2023 (minor bug B47).
    if rules.fair_catch_touchback && land_x < 25.0 && land_x > -1.0 && rng.chance(0.3) {
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
    let snap_skill = snapper.on20(Attr::LongSnapping) + wet_snaps(snapper, cond, rules.level);
    let bad = rng.chance(0.005 * (1.0 + (10.0 - snap_skill).max(-5.0) * 0.2));
    res.bad_snap = bad;
    // The rush against the shield; a slow operation gets blocked. Snap to
    // kick is the snap (about 0.75 s) and the punter's catch to kick (about
    // 1.3 s), 2.05 s together for an average pair (proposed, not confirmed).
    let snap_time = clamp(0.75 - 0.003 * (snapper.a(Attr::LongSnapping) - 60.0), 0.62, 0.95);
    let get_off = clamp(1.3 - 0.004 * (punter.a(Attr::GetOff) - 60.0), 1.12, 1.55);
    let op = snap_time + get_off + if bad { 0.7 } else { 0.0 } + rng.normal(0.0, 0.08);
    let from = goal() - los; // receiving team frame: the kicking spot
    // The rush (engine gaps item 12, `punt_block`): mostly a return call
    // with half the front holding its man up, now and then all out for the
    // block; the punter kicks from his depth behind the line.
    let all_out = rng.chance(PUNT_ALL_OUT);
    let deep = punt_depth(rules.level);
    let gunners: Vec<usize> = k.kick_on.iter().skip(1).take(2).copied().collect();
    let protect: Vec<usize> = std::iter::once(k.snapper)
        .chain(k.kick_on.iter().skip(1).copied().filter(|&i| i != k.snapper && !gunners.contains(&i)))
        .collect();
    let rushers: Vec<usize> = k.ret_on.iter().skip(3).copied().collect();
    let line = KickLine { kick: k.kick, rush: k.ret, protect: &protect, rushers: &rushers, all_out };
    if let Some(b) = punt_block(&line, op, deep, rng) {
        res.blocked = true;
        let mid = super::geom::mid();
        let mut men0 = vec![(k.kick_on[0], v2(los - deep, mid))];
        for (g, &i) in gunners.iter().enumerate() {
            men0.push((i, v2(los + 9.0, mid + if g == 0 { -15.0 } else { 15.0 })));
        }
        men0.push((k.snapper, v2(los - 0.3, mid)));
        let mut rest: Vec<usize> = protect[1..].to_vec();
        rest.sort_by(|&a, &b| k.kick[b].weight.total_cmp(&k.kick[a].weight));
        let spots = [(-0.5, -1.2), (-0.5, 1.2), (-0.5, -2.4), (-0.5, 2.4), (-3.0, -1.8), (-3.0, 1.8), (-6.0, 0.5)];
        for (n, &i) in rest.iter().enumerate() {
            let (dx, dy) = spots[n.min(spots.len() - 1)];
            men0.push((i, v2(los + dx, mid + dy)));
        }
        let mut men1: Vec<(usize, V2)> =
            rushers.iter().enumerate().map(|(j, &i)| (i, v2(los - b.depth[j], mid + b.lane[j]))).collect();
        for (n, &i) in k.ret_on.iter().enumerate().take(3) {
            let at = match n {
                0 => v2((los + 42.0).min(goal() - 2.0), mid),
                1 => v2(los + 9.0, mid - 14.0),
                _ => v2(los + 9.0, mid + 14.0),
            };
            men1.push((i, at));
        }
        let set = LiveSetup {
            teams: [k.kick, k.ret],
            men: [men0, men1],
            blocker: b.by,
            ball: v2(los - deep + PUNT_STEP + b.at, mid + b.across),
            punt: true,
            try_ends: false,
            energy: k.energy,
            hurt: k.hurt,
            mood: k.mood,
            los,
        };
        let live = blocked_live(&set, rules, cond, crew, record, rng);
        res.duration = op + live.duration;
        res.live = Some(live);
        return res;
    }
    // Punt Power: 0.14 yard a point about his level's typical punter, who
    // kicks as far as he did at 0.215 a point (minor bug 282: at 0.215 a ±15
    // swing moved the net punt 1.28 times real regulars' spread, at 0.14
    // 0.90; proposed, not confirmed).
    let per = 0.14;
    let typ = super::synthetic::typical(rules.level, crate::positions::Position::P, Attr::PuntPower);
    // Drop inconsistency (individual practice): a wider spread, so more
    // shanks.
    let drop = punter.craft_vs(crate::craft::Craft::DropInconsistent, rules.level, 0.0);
    let max = leg(punter, Attr::PuntPower, 17.0 + (0.215 - per) * typ, per, cond, along)
        + 14.0
        + rng.normal(0.0, 4.5 * (1.0 - 0.06 * drop));
    // Placement: from midfield he aims inside the 10 rather than the end zone.
    let dist_goal = from + 15.0;
    // Your plan's punt (update 2, G18): the coffin corner from their 45 in,
    // aimed at the corner by the sideline; farther out it is a punt like
    // any other.
    let pin = from < 62.0;
    let coffin = k.style == 2 && from <= 45.0;
    // The pooch and coffin corner (individual practice): pinning from
    // midfield, his placement reads the tool higher.
    let acc = punter.a(Attr::PuntPlacement)
        + if pin { punter.craft_vs(crate::craft::Craft::CoffinCorner, rules.level, 0.0) } else { 0.0 };
    let aim_land = if pin { 8.0 + rng.normal(0.0, 6.0 - 0.04 * acc) } else { from - max };
    let land_x = aim_land.max(from - max);
    let _ = dist_goal;
    let hang = clamp(
        3.5 + 0.018 * (punter.a(Attr::PuntPower) - 60.0) + 0.006 * (acc - 60.0) + rng.normal(0.0, 0.25)
            - if pin { 0.2 } else { 0.0 },
        2.9,
        5.2,
    );
    // Pinning, he aims at the numbers on one side, away from the middle of
    // the field where the return has room; placement sets how close he
    // comes (proposed, not confirmed).
    // A directional punt goes to the numbers away from the middle on every
    // punt, a little shorter and lower (proposed, not confirmed).
    let directional = k.style == 1 && !pin;
    let mut land_y = if pin || directional {
        let side = if rng.chance(0.5) { -1.0 } else { 1.0 };
        let out = if coffin { 19.0 } else { 14.0 };
        mid() + side * out + rng.normal(0.0, clamp(5.0 - 0.05 * (acc - 60.0), 3.0, 7.0))
    } else {
        mid() + rng.normal(0.0, 8.0)
    };
    let land_x = if directional { land_x + 2.5 } else { land_x };
    let hang = if directional { hang - 0.15 } else { hang };
    land_y = clamp(land_y, -1.0, width() + 1.0);
    res.distance = from - land_x;
    res.hang = hang;
    // Out of bounds (coffin corner or a shank).
    if !(0.0..=width()).contains(&land_y) || rng.chance(0.03 * (1.3 - punter.composure(rules.level) / 20.0)) {
        res.out_of_bounds = true;
        land_y = land_y.clamp(0.0, width());
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
        .map(|p| 0.5 * p.a(Attr::Speed) + 0.3 * p.a(Attr::Release) + 0.2 * p.a(Attr::Strength))
        .fold(0.0, f64::max);
    let jammer = k
        .ret_on
        .iter()
        .skip(1)
        .take(2)
        .map(|&i| &k.ret[i])
        .map(|p| 0.4 * p.a(Attr::Press) + 0.3 * p.a(Attr::Strength) + 0.3 * p.a(Attr::Speed))
        .fold(0.0, f64::max);
    // How close the gunners are sets the fair catches: 29% of NFL punts
    // with 5.2, real 27.5% (nflverse 2021-2025; minor bug 139: 4.6 made
    // 37%; proposed, not confirmed).
    let mut cover_gap = clamp(5.2 - 1.6 * (hang - 4.2) - 0.08 * (gunner - jammer) + rng.normal(0.0, 2.0), 0.0, 15.0);
    // Near the numbers the sideline helps the coverage.
    if directional {
        cover_gap = (cover_gap - 0.8).max(0.0);
    }
    // No fair catch (the CFL, the XFL of 2001): the cover men must give the
    // returner his halo until he touches the ball; one who gets there
    // first draws "no yards", 15 yards on the kicking team (5 in the XFL).
    let no_fc = !rules.code.fair_catch;
    let mut no_yards = 0.0;
    if no_fc {
        if cover_gap < 0.5 * rules.code.halo && rng.chance(0.25) {
            no_yards = if rules.code.book == super::books::Book::Cfl { 15.0 } else { 5.0 };
        }
        // He has his halo when it comes down, and the cover men, held off
        // until he touches it, are still closing when he starts to run
        // (real CFL punt returns about 10 yards against the NFL's 9.5 on
        // returned ones; Claude's reading).
        cover_gap = cover_gap.max(rules.code.halo) + 0.5 * rules.code.halo;
    }
    let returner = &k.ret[k.ret_on[0]];
    let judgment = returner.a(Attr::Decisions);
    // Returner risk (tendencies): a risk-taker fields punts he could let
    // bounce and returns ones he could fair-catch; an order to fair-catch
    // inside the 10 and let the rest bounce decides it when he follows it.
    let level = rules.level;
    let r_i = k.ret_on[0];
    let risk = returner.tend_now(Tend::ReturnRisk, k.mood.gamble[1], k.mood.stakes[1], k.energy[1][r_i], level);
    let safe = returner.orders.has(|o| matches!(o, Instruction::FairCatchInsideTen))
        && returner.follows(k.mood.strict[1], level, rng);
    // Following the order, the rest of his calls read its returner risk
    // (update 3, F17).
    let risk = if safe { returner.orders.sets(Tend::ReturnRisk).unwrap_or(risk) } else { risk };
    let lean = (risk - MID_T) / 9.5;
    // Inside his 10, a returner lets it go: it bounces.
    if land_x < 10.0 {
        let field =
            !safe && rng.chance(clamp((0.1 + 0.004 * (60.0 - judgment)) * (1.0 + 1.3 * lean).max(0.1), 0.02, 0.7));
        res.notes.tend.fielded_inside_10 = Some(field);
        if !field {
            let roll = rng.normal(2.0, 7.0);
            let end = land_x - roll;
            // A live punt (the XFL of 2001): past 25 yards it is anybody's
            // ball once it hits the ground.
            if rules.code.punt_live && end > 0.0 && rng.chance(0.08) {
                res.recovered_by_kicking = true;
                res.spot = goal() - end;
                res.distance = from - end;
                res.duration = hang + 2.0;
                return res;
            }
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
    // Fair catch when coverage is on him: a risk-taker brings more of them
    // back and muffs more doing it.
    let fc = if safe { 0.99 } else { clamp(0.95 - 0.004 * (judgment - 50.0) - 0.3 * lean, 0.45, 0.99) };
    let reach = if safe { 7.0 } else { 5.0 - 1.5 * lean };
    if !no_fc && (cover_gap < reach && rng.chance(fc) || cover_gap < 1.5) {
        res.fair_catch = true;
        res.spot = land_x.max(1.0);
        res.inside20 = res.spot <= 20.0;
        res.duration = hang;
        res.returner = Some(returner.id);
        if muff(returner, punter, cond, rng) {
            res.muffed = true;
            res.recovered_by_kicking = rng.chance(0.45);
            if res.recovered_by_kicking {
                res.spot = goal() - res.spot;
            }
        }
        return res;
    }
    // Returning one with coverage close is how muffs happen.
    let crowded = if cover_gap < 5.0 { 2 } else { 1 };
    if (0..crowded).any(|_| muff(returner, punter, cond, rng)) {
        res.muffed = true;
        res.returner = Some(returner.id);
        res.recovered_by_kicking = rng.chance(0.5);
        res.spot = if res.recovered_by_kicking { goal() - land_x } else { land_x };
        res.duration = hang + 1.0;
        return res;
    }
    let catch = v2(land_x, land_y.clamp(2.0, width() - 2.0));
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
    if no_yards > 0.0 && !res.return_td && !res.recovered_by_kicking {
        res.spot = (res.spot + no_yards).min(goal() - 1.0);
    }
    res.inside20 = res.spot <= 20.0;
    res
}

/// A muffed punt: tracking the ball off the foot, then the hands. A
/// left-footed punter's ball spins the other way, which returners see less
/// (about 20% more muffs; proposed, not confirmed).
fn muff(p: &GamePlayer, punter: &GamePlayer, cond: &Conditions, rng: &mut Rng) -> bool {
    let wet = if cond.rain || cond.snow { 2.0 } else { 1.0 };
    let hands = 0.6 * p.a(Attr::BallTracking) + 0.4 * p.a(Attr::Catching);
    let spin = if punter.left_footed { 1.2 } else { 1.0 };
    rng.chance(0.006 * wet * spin * (1.5 - hands / 100.0) * (1.0 + 0.02 * cond.wind))
}

/// A tracked return (topic 7): the returner catches at `catch`, his
/// blockers set up, the coverage arrives from where it was when the ball
/// came down, and the return plays out like any run.
/// A return blocker on his assigned man: get between him and the ball and
/// put hands on him. False once the man is down, blocked by someone else or
/// past the returner, and the blocker goes back to walling off.
fn block_man(sim: &mut Sim, i: usize, d: usize) -> bool {
    let (me, dp, cp) = (sim.agents[i].pos, sim.agents[d].pos, sim.agents[0].pos);
    if !sim.agents[d].up(sim.t) || sim.agents[d].engaged.is_some() || dp.x < cp.x - 1.0 {
        return false;
    }
    if sim.closest(i, d) < 1.3 && sim.lands_block(i, d) {
        sim.engage(i, d, false);
        return true;
    }
    // Stay square in front of him, a few yards toward the ball, and let him
    // come (a blocker who runs at a cover man in space whiffs); close the
    // last yards matching his speed.
    let vm = sim.agents[i].vmax;
    // A man still getting started a few yards away (the 2025 NFL setup
    // zone, five yards from the coverage): go get him before he builds
    // speed and an angle, as on the line of scrimmage.
    let want = if sim.agents[d].vel.len() < 4.0 && me.dist(dp) < 7.0 {
        (dp - me).norm().scale(vm)
    } else if me.dist(dp) > 3.0 {
        // The first spot on his path to the ball that the blocker reaches
        // before him.
        let (path, len) = ((cp - dp).norm(), dp.dist(cp));
        let vd = sim.agents[d].vel.len().max(sim.agents[d].vmax * 0.8);
        // He wants to be there and set (feet under him) half a second
        // before the man arrives, so he slows into the spot.
        let mut k = 1.5;
        while k < len && me.dist(dp + path.scale(k)) / vm + 0.5 > k / vd {
            k += 1.0;
        }
        let set = dp + path.scale(k.min(len));
        let gap = set.dist(me);
        if gap > 0.3 { (set - me).norm().scale(vm.min(2.0 * gap)) } else { V2::default() }
    } else {
        // Set: only slide across to stay on his line to the ball.
        let path = (cp - dp).norm();
        let rel = me - dp;
        let off = rel - path.scale(rel.dot(path));
        off.scale(-3.0)
    };
    sim.steer(i, want);
    true
}

/// The sim for a play on a kick (a return, or a blocked kick's live ball):
/// the two teams (side 0 first), no silent count, the same lunge in the
/// open field as on a scrimmage play, and the book's fumble rules.
#[allow(clippy::too_many_arguments)]
fn kick_sim<'a>(
    teams: [&'a [GamePlayer]; 2],
    rng: &'a mut Rng,
    rules: &Rules,
    cond: &Conditions,
    crew: &Crew,
    injury_scale: f64,
    record: bool,
    mood: super::sim::Mood,
    los: f64,
) -> Sim<'a> {
    Sim {
        teams,
        agents: Vec::new(),
        // Room for a return's blocks in one allocation: capacity changes no
        // value or index (world speed lane).
        eng: Vec::with_capacity(16),
        ball: Ball::Held(0),
        t: 0.0,
        los,
        rng,
        notes: Notes::default(),
        record: if record { Some(Vec::new()) } else { None },
        // No silent count on kicks.
        cond: Conditions { crowd: 0.0, ..*cond },
        crew: *crew,
        goal: 1.0,
        carrier_side: 0,
        dead: false,
        out_of_bounds: false,
        pocket: None,
        progress: los,
        contact_force: 0.0,
        injury_scale,
        down_on_fall: rules.down_on_fall,
        fight: Vec::new(),
        fight_since: 0.0,
        screen: None,
        hold_edge: None,
        hold_slip: [0.0; super::sim::MAX_AGENTS],
        // The same lunge in the open field as on a scrimmage play.
        open_reach: SCRIMMAGE_REACH,
        press: 0.0,
        clock_run: ClockRun::Normal,
        designed_run: false,
        lane: None,
        bites: Vec::new(),
        triggers: Vec::new(),
        two_gap: false,
        help_on: None,
        chip: None,
        gap_run: None,
        fake: None,
        read_key: None,
        read_sit: false,
        read_at: 0.0,
        stunt: None,
        level: rules.level,
        mood,
        targeting: rules.targeting,
        concussion: rules.concussion,
        fouls: rules.foul_book(),
        loose: None,
        fumble_rule: FumbleSnap::plain(rules),
        grab: 1.0,
        ibb: 1.0,
        resume: None,
        caught: -9.0,
        tend_memo: super::sim::tend_memo(),
        reach_memo: super::sim::reach_memo(),
    }
}

/// One tick of a loose ball and of whoever comes up with it (engine gaps
/// items 4 and 12): the ball on the ground with everyone after it; then the
/// man with it runs, his side blocking for him and the other chasing. False
/// once nobody has it.
fn live_tick(sim: &mut Sim) -> bool {
    if matches!(sim.ball, Ball::Loose(_)) {
        sim.loose_tick();
        sim.record_frame();
        return true;
    }
    let Some(c) = sim.carrier() else { return false };
    sim.update_blocks(None, Some(c));
    for i in 0..sim.agents.len() {
        if i == c || !sim.agents[i].up(sim.t) || sim.agents[i].engaged.is_some() {
            continue;
        }
        match sim.agents[i].job {
            Job::Escort => escort(sim, i),
            Job::Pursue => sim.pursue(i, c),
            _ => {}
        }
    }
    sim.run_carrier(c, None);
    sim.advance();
    sim.tackles(c);
    if !sim.dead {
        sim.check_carrier(c);
    }
    sim.record_frame();
    true
}

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
    let cover_start = if kickoff && rules.nfl_kickoff { rules.code.cover_line } else { from - 1.0 };
    for (n, &i) in k.kick_on.iter().enumerate() {
        let p = &*super::sim::hampered(&k.kick[i], k.hurt[0].get(i).copied().flatten());
        let lane = 4.0 + (n as f64) * (width() - 8.0) / (rules.code.players as f64 - 1.0);
        let speed = p.top_speed() * 0.9;
        let x = if n == 0 {
            // The kicker trails as the safety: under the 2025 NFL rule he
            // stays at the kick spot until the ball lands.
            if kickoff && rules.nfl_kickoff { from } else { catch.x + 30.0 + rng.range(0.0, 8.0) }
        } else if let Some(g) = gap {
            // A punt: the gunners are cover_gap away when it comes down,
            // the rest of the unit 5 to 19 yards behind them in lanes, the
            // linemen who blocked first trailing as a second wave (minor
            // bug 139: one wave 5-13 back let 2% of returns go the
            // distance, real 0.8%; proposed, not confirmed).
            let behind = if n <= 2 { rng.range(0.0, 2.0) } else { 5.0 + rng.range(0.0, 14.0) };
            catch.x + g + behind
        } else {
            (cover_start - speed * run).max(catch.x + 5.0 + rng.range(0.0, 6.0))
        };
        let y = if n == 0 {
            // The kicker kicks from between the hashes and trails in the
            // middle as the last man (it used to start him at the sideline).
            mid() + rng.range(-3.0, 3.0)
        } else if gap.is_some() && (n == 1 || n == 2) {
            // Gunners close on the returner from either side.
            // Inside the lines: a catch near the sideline put one up to
            // 4.5 yards out of bounds, his jammer driving him farther.
            (catch.y + if n == 1 { -4.5 } else { 4.5 } + rng.range(-2.0, 2.0)).clamp(1.0, width() - 1.0)
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
        let p = &*super::sim::hampered(&k.ret[i], k.hurt[1].get(i).copied().flatten());
        let pos = if n == 0 {
            catch
        } else if gap.is_some() && n <= 2 && agents.len() > n {
            // A punt: each jammer has run down the field on his gunner and
            // is on him when the ball comes down, a step in front of him or
            // a step beaten (real vise play). They used to start with the
            // wall, so the gunners came free and NFL punt returns went 5.5
            // yards against the real 9.5 (minor bug 139).
            let g = agents[n].pos;
            v2(g.x + rng.range(-1.5, 1.0), (g.y + rng.range(-0.7, 0.7)).clamp(0.5, width() - 0.5))
        } else {
            // The wall forms between him and the coverage.
            let lane = 6.0 + ((n - 1) as f64) * (width() - 12.0) / (rules.code.players as f64 - 2.0);
            // NFL 2025: nine of the return team line up in the setup zone
            // (their 30 to 35) and may not move until the ball lands.
            // Elsewhere the front line starts ten yards off the ball and drops
            // to set the wall about 25 yards in front of the catch, the middle
            // men 15, the up-back 8 (real return schemes; proposed, not
            // confirmed: they all used to sit 8 to 16 yards in front).
            let x = if kickoff && rules.nfl_kickoff {
                30.0 + rng.range(0.0, 5.0)
            } else if kickoff {
                let depth = match n {
                    1 => 8.0 + rng.range(0.0, 4.0),
                    2..=5 => 15.0 + rng.range(0.0, 5.0),
                    _ => 23.0 + rng.range(0.0, 6.0),
                };
                catch.x + depth
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
    // The sim's side 0 is the return team.
    // The officials on a return (bug 403): the blocks and tackles draw their
    // fouls as on a scrimmage down, a beaten blocker in space grabs more often
    // (most of all a jammer on a gunner), and fewer personal fouls stand
    // (sized to real returned kicks by kind, `RET_*`).
    let crew = Crew { personal: crew.personal * RET_PERSONAL, ..*crew };
    let mut sim = kick_sim([k.ret, k.kick], rng, rules, cond, &crew, injury_scale, record, k.mood.swap(), catch.x);
    sim.grab = if kickoff { RET_GRAB_KICKOFF } else { RET_GRAB_PUNT };
    sim.ibb = RET_IBB;
    // Reorder so the return team (side 0) comes first: the same order as
    // moving them into a new list, done in place (world speed lane).
    agents.rotate_left(ret_idx);
    sim.agents = agents;
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
    // Kickoff return blocking is man to man: each blocker takes the cover
    // man in his lane, counted from the outside in, as real return units
    // are taught; the kicker is left for the returner. Punt returns keep
    // the old rule (hold the gunners, then wall off).
    // Two cover men, one on each side inside the contain men, are the
    // safeties: they trail the first wave by about ten yards, so a returner
    // who breaks the wall still has men to beat (real kickoff coverage; the
    // first wave is eight). The two spare blockers double the first wave.
    let mut man: Vec<Option<usize>> = vec![None; sim.agents.len()];
    let mut safety = vec![false; sim.agents.len()];
    if kickoff {
        let mut blockers: Vec<usize> = (1..sim.agents.len()).filter(|&i| sim.agents[i].side == 0).collect();
        let mut cover: Vec<usize> =
            (0..sim.agents.len()).filter(|&i| sim.agents[i].side == 1 && sim.agents[i].label == "ST").collect();
        blockers.sort_by(|&a, &b| sim.agents[a].pos.y.total_cmp(&sim.agents[b].pos.y));
        cover.sort_by(|&a, &b| sim.agents[a].pos.y.total_cmp(&sim.agents[b].pos.y));
        if cover.len() >= 8 {
            let n = cover.len();
            for k in [2, n - 3] {
                safety[cover[k]] = true;
            }
            cover.retain(|&c| !safety[c]);
        }
        let (nb, nc) = (blockers.len(), cover.len());
        for (j, &b) in blockers.iter().enumerate() {
            man[b] = cover.get(j * nc / nb.max(1)).copied();
        }
    } else if gap.is_some() {
        // On a punt the two jammers stay on the gunners (the return team
        // comes first, then the kicker and his two gunners).
        let r = k.ret_on.len();
        for j in 1..=2 {
            if j < r && r + j < sim.agents.len() {
                man[j] = Some(r + j);
            }
        }
    }
    // How far in front of the returner each deep man holds: the safeties
    // ten yards, the kicker fifteen (proposed, not confirmed).
    let deep = |i: usize| if safety[i] { 10.0 } else { 15.0 };
    let ktrace = super::sim::TRACE.load(std::sync::atomic::Ordering::Relaxed);
    let mut buf = String::new();
    // The first wave's spots: one list for the whole return, refilled each
    // tick (the same values in the same order; world speed lane).
    let mut wave: Vec<f64> = Vec::with_capacity(sim.agents.len());
    while sim.t < 12.0 && !sim.dead {
        sim.t += DT;
        sim.link_speeds();
        if ktrace && ((sim.t / DT).round() as i64) % 5 == 0 {
            buf.push_str(&sim.snapshot());
            buf.push('\n');
        }
        // A fumbled return (engine gaps item 4): the ball loose on the
        // ground, everyone after it; then whoever came up with it runs, his
        // side blocking for him and the other chasing.
        if matches!(sim.ball, Ball::Loose(_)) || sim.loose.is_some() {
            if !live_tick(&mut sim) {
                break;
            }
            continue;
        }
        sim.update_blocks(None, Some(0));
        // Where the first wave is: the safeties trail it by about ten yards
        // and the kicker sits behind them as the last man, so a returner
        // who breaks the wall still meets two levels (real kickoff coverage).
        wave.clear();
        wave.extend(
            (1..sim.agents.len())
                .filter(|&i| sim.agents[i].side == 1 && !safety[i] && sim.agents[i].label == "ST")
                .map(|i| sim.agents[i].pos.x),
        );
        let wave_x = if wave.is_empty() { sim.agents[0].pos.x } else { wave.iter().sum::<f64>() / wave.len() as f64 };
        // The kicker stays the last man while a cover man is still free in
        // front of the returner (realism audit item 3, who makes the
        // tackle): he used to come up as soon as the returner was inside his
        // depth and made 0.44 tackles a team game on world seed 321's NFL
        // clubs, a tenth of all kickoff tackles (real NFL 2023-24: 0.03).
        // Holding, he makes 0.07; returns of 40 yards or more go from 0.037
        // a team game to 0.057 (real 0.050).
        let rx = sim.agents[0].pos.x;
        let front = (1..sim.agents.len()).any(|j| {
            let a = &sim.agents[j];
            a.side == 1 && a.label == "ST" && a.up(sim.t) && a.engaged.is_none() && a.pos.x > rx + 1.0
        });
        for i in 1..sim.agents.len() {
            if !sim.agents[i].up(sim.t) || sim.agents[i].engaged.is_some() {
                continue;
            }
            match sim.agents[i].job {
                Job::Escort => {
                    if !man[i].is_some_and(|d| block_man(&mut sim, i, d)) {
                        escort(&mut sim, i)
                    }
                }
                Job::Pursue => {
                    let (me, rp) = (sim.agents[i].pos, sim.agents[0].pos);
                    let ahead = me.x > rp.x + 1.0;
                    if contain[i] != 0.0 && ahead && me.dist(rp) > 4.0 {
                        let aim = v2(rp.x + 3.0, rp.y + contain[i] * 2.5);
                        let vm = sim.agents[i].vmax;
                        sim.steer(i, (aim - me).norm().scale(vm));
                    } else if kickoff
                        && (safety[i] || sim.agents[i].label == "K")
                        && (me.x - rp.x > deep(i) || (front && sim.agents[i].label == "K" && me.x - rp.x > 5.0))
                    {
                        // A safety or the kicker holds his level behind the
                        // wave, shaded to the ball, until the returner is on
                        // him; then he attacks with a deep angle.
                        let level = (rp.x + deep(i)).max(wave_x + deep(i) - 3.0);
                        let shade = if safety[i] { (lanes[i] - mid()) * 0.3 } else { 0.0 };
                        let aim = v2(level, rp.y + shade);
                        let vm = sim.agents[i].vmax;
                        let gap = (me.x - level).abs();
                        sim.steer(i, (aim - me).norm().scale(vm.min(2.5 * gap + 2.0)));
                    } else if kickoff && !lanes[i].is_nan() && me.x - rp.x > 20.0 {
                        // Lane discipline: down the field in his lane, spread
                        // across the ball, until he is about 20 yards from it,
                        // so the return can't bounce to an empty side (real
                        // coverage rules; the distance is sized against real
                        // return averages, proposed, not confirmed).
                        // His lane moves with the ball and narrows as he
                        // closes, so the unit arrives spread across it.
                        let squeeze = clamp((me.x - rp.x) / 35.0, 0.6, 1.0);
                        let aim = v2(rp.x, rp.y + (lanes[i] - mid()) * squeeze);
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
    // The play's work: time in blocks, and strains and plants with nobody
    // touching him (engine gaps items 1 and 2).
    sim.wear();
    res.duration = hang + sim.t;
    let mut end = sim.progress;
    // A fumbled return (engine gaps item 4): who has it where, or the score,
    // the kicking team's spot in its own frame. A safety on a return is
    // kept as a touchback (`GAPS.md`); the return's yards run to where the
    // ball came out.
    if let Some(e) = sim.loose_end() {
        let at = sim.loose.map_or(end, |l| l.at.x);
        match e {
            LooseEnd::Ball { side: 0, x } => res.spot = x,
            LooseEnd::Ball { x, .. } => {
                res.recovered_by_kicking = true;
                res.spot = goal() - x;
            }
            LooseEnd::Touchdown(0) => {
                res.spot = goal();
                res.return_td = true;
            }
            LooseEnd::Touchdown(_) => {
                res.recovered_by_kicking = true;
                res.spot = goal();
            }
            LooseEnd::Safety(0) | LooseEnd::Touchback(0) => {
                res.touchback = true;
                res.spot = rules.touchback;
            }
            LooseEnd::Safety(_) | LooseEnd::Touchback(_) => {
                res.recovered_by_kicking = true;
                res.spot = rules.touchback;
            }
        }
        if let (Some(l), Some(f)) = (sim.loose, sim.notes.fumble.as_mut())
            && let Some((w, rat)) = l.rec
        {
            let dir = if sim.agents[w].side == 0 { 1.0 } else { -1.0 };
            let to = match e {
                LooseEnd::Ball { x, .. } => x,
                LooseEnd::Touchdown(0) => goal(),
                LooseEnd::Touchdown(_) => 0.0,
                _ => end,
            };
            f.ran = ((to - rat.x) * dir).max(0.0) as f32;
        }
        end = at;
    } else {
        res.spot = end;
        if end >= goal() {
            res.return_td = true;
        }
        if end <= 0.0 {
            // Tackled in his own end zone after catching it there: a touchback.
            res.touchback = true;
            res.spot = rules.touchback;
        }
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
    let before = res.notes.tend;
    res.notes = std::mem::take(&mut sim.notes);
    res.notes.tend.deep_kick = before.deep_kick;
    res.notes.tend.fielded_inside_10 = before.fielded_inside_10;
    res.frames = sim.record.take();
}
