//! What each body is doing in a tracked frame, for the 2D and 3D game views
//! (later extras topics 2 and 3): his facing, his action and with whom, and
//! the ball's height. Output only: it reads the play as the engine already
//! has it and never draws a number or changes a player, so every seed gives
//! the same game with or without frames (`docs/match-engine.md`, "Frames
//! for the views").

use super::geom::{DT, V2};
use super::sim::{Ball, Frame, Job, Sim};

/// No partner, no holder.
pub const NONE: u8 = 255;

/// What a player is doing in one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Act {
    /// Standing or walking.
    #[default]
    Stand = 0,
    /// Set in his stance before the snap.
    Stance = 1,
    /// Running (gait by speed).
    Run = 2,
    /// Blocking, locked on `with`.
    Block = 3,
    /// A defender taking on the block of `with`.
    Engaged = 4,
    /// Rushing the passer, free.
    Rush = 5,
    /// In coverage, man or zone (a backpedal when he moves away from where
    /// he faces).
    Cover = 6,
    /// The quarterback in the pocket with the ball, eyes downfield.
    Pocket = 7,
    /// The throwing motion: the release and its follow-through.
    Throw = 8,
    /// Hands up for the ball arriving, and securing it.
    Catch = 9,
    /// Carrying the ball.
    Carry = 10,
    /// Tackling `with`, the carrier.
    Tackle = 11,
    /// The carrier wrapped up by `with`.
    Wrapped = 12,
    /// Going to the ground.
    Fall = 13,
    /// On the ground.
    Down = 14,
    /// Getting back up.
    GetUp = 15,
    /// Celebrating a touchdown.
    Celebrate = 16,
    /// A run fake or the handoff mesh.
    Fake = 17,
}

impl Act {
    pub fn from_u8(v: u8) -> Act {
        match v {
            1 => Act::Stance,
            2 => Act::Run,
            3 => Act::Block,
            4 => Act::Engaged,
            5 => Act::Rush,
            6 => Act::Cover,
            7 => Act::Pocket,
            8 => Act::Throw,
            9 => Act::Catch,
            10 => Act::Carry,
            11 => Act::Tackle,
            12 => Act::Wrapped,
            13 => Act::Fall,
            14 => Act::Down,
            15 => Act::GetUp,
            16 => Act::Celebrate,
            17 => Act::Fake,
            _ => Act::Stand,
        }
    }

    pub fn name(self) -> &'static str {
        [
            "standing",
            "in his stance",
            "running",
            "blocking",
            "taking on a block",
            "rushing",
            "in coverage",
            "in the pocket",
            "throwing",
            "catching",
            "carrying",
            "tackling",
            "wrapped up",
            "falling",
            "down",
            "getting up",
            "celebrating",
            "faking",
        ][self as usize]
    }

    fn grounded(self) -> bool {
        matches!(self, Act::Fall | Act::Down)
    }
}

/// One player's pose in a frame, in the same order as `Frame::players`.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pose {
    /// Where he faces, radians in the play's frame (0 toward the snapping
    /// team's goal, positive toward its left).
    pub face: f32,
    pub act: Act,
    /// The player he is blocking, shedding or tackling: an index into
    /// `Frame::players`, or `NONE`.
    pub with: u8,
}

/// Where the ball is, for the views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum BallMode {
    #[default]
    Held = 0,
    Air = 1,
    Loose = 2,
    Dead = 3,
}

/// Yards per second squared: a thrown ball's arc follows gravity over the
/// engine's own flight time.
const G: f64 = 32.174 / 3.0;
/// The ball's height in a carrier's arm, at a quarterback's release and at
/// the catch, yards (about 40, 84 and 48 inches).
const CARRY_Z: f64 = 1.1;
const CATCH_Z: f64 = 1.35;

/// The ball's height above the turf, yards.
fn ball_z(sim: &Sim) -> f64 {
    match sim.ball {
        Ball::Held(i) => {
            if matches!(sim.agents[i].job, Job::Passer) {
                // Held high in the pocket, at the shoulder.
                CARRY_Z + 0.35
            } else if sim.t < sim.agents[i].down_until {
                0.35
            } else {
                CARRY_Z
            }
        }
        Ball::Air { release, arrive, target, from, .. } => {
            let t = sim.t.clamp(release, arrive);
            let span = (arrive - release).max(0.01);
            let k = (t - release) / span;
            // Released over the passer's head, caught at the chest or, on a
            // throw meant for nobody, landing on the ground.
            let thrower_h = sim
                .agents
                .iter()
                .min_by(|a, b| a.pos.dist(from).total_cmp(&b.pos.dist(from)))
                .map_or(75.0, |a| sim.teams[a.side][a.p].height);
            let h0 = thrower_h * 1.12 / 36.0;
            let h1 = if target.is_some() { CATCH_Z } else { 0.0 };
            h0 + (h1 - h0) * k + 0.5 * G * (t - release) * (arrive - t)
        }
        Ball::Loose(_) | Ball::Dead(_) => 0.12,
    }
}

/// The acts the last few frames gave player `i` (newest first).
fn history(prev: &[Frame], i: usize, n: usize) -> impl Iterator<Item = Act> + '_ {
    prev.iter().rev().take(n).map(move |f| f.poses.get(i).map_or(Act::Stand, |p| p.act))
}

/// Builds this tick's frame from the sim as it stands.
pub(crate) fn frame(sim: &Sim, prev: &[Frame]) -> Frame {
    let t = sim.t;
    let n = sim.agents.len();
    let ball = sim.ball_pos();
    let last = prev.last();
    let (mode, holder) = match sim.ball {
        Ball::Held(i) => (BallMode::Held, i as u8),
        Ball::Air { .. } => (BallMode::Air, NONE),
        Ball::Loose(_) => (BallMode::Loose, NONE),
        Ball::Dead(_) => (BallMode::Dead, NONE),
    };
    let was_air = last.is_some_and(|f| f.ball_mode == BallMode::Air);
    let was_holder = last.and_then(|f| (f.ball_mode == BallMode::Held).then_some(f.holder));
    // A touchdown on the last frame: the scorer's side celebrates.
    let scored = sim.dead
        && match sim.ball {
            Ball::Dead(p) => (sim.goal > 0.0 && p.x >= 100.0) || (sim.goal < 0.0 && p.x <= 0.0),
            _ => false,
        };
    let snap = prev.is_empty() && t <= DT * 0.5;
    let mut poses = Vec::with_capacity(n);
    for (i, a) in sim.agents.iter().enumerate() {
        let mut face = a.face;
        let mut with = NONE;
        let speed = a.vel.len();
        let down = t < a.down_until || matches!(a.job, Job::Out);
        let act = if snap {
            Act::Stance
        } else if down {
            // The first moments on the way down, then flat.
            let falling = history(prev, i, 4).take_while(|x| *x == Act::Fall).count();
            let was_up = history(prev, i, 1).next().is_none_or(|x| !x.grounded());
            if was_up || (falling > 0 && falling < 4) { Act::Fall } else { Act::Down }
        } else if history(prev, i, 1).next().is_some_and(|x| x.grounded())
            || (history(prev, i, 2).any(|x| x == Act::GetUp)
                && history(prev, i, 3).filter(|x| *x == Act::GetUp).count() < 2
                && speed < 4.0)
        {
            Act::GetUp
        } else if let Some(k) = a.engaged {
            let e = &sim.eng[k];
            if e.defender == i {
                with = e.blocker as u8;
                face = (sim.agents[e.blocker].pos - a.pos).norm();
                Act::Engaged
            } else {
                with = e.defender as u8;
                face = (sim.agents[e.defender].pos - a.pos).norm();
                Act::Block
            }
        } else if sim.fight.contains(&i)
            && let Ball::Held(c) = sim.ball
        {
            with = c as u8;
            face = (sim.agents[c].pos - a.pos).norm();
            Act::Tackle
        } else if scored && a.side == sim.carrier_side {
            Act::Celebrate
        } else if mode == BallMode::Air
            && was_holder == Some(i as u8)
            && let Ball::Air { to, .. } = sim.ball
        {
            face = (to - a.pos).norm();
            Act::Throw
        } else if history(prev, i, 2).any(|x| x == Act::Throw) && history(prev, i, 3).filter(|x| *x == Act::Throw).count() < 3 {
            Act::Throw
        } else if let Ball::Air { to, arrive, target, .. } = sim.ball
            && arrive - t <= 0.35
            && (target == Some(i) || a.pos.dist(to) < 1.6)
        {
            face = (ball - a.pos).norm();
            Act::Catch
        } else if mode == BallMode::Held && holder as usize == i && was_air {
            Act::Catch
        } else if mode == BallMode::Held && holder as usize == i {
            if !sim.fight.is_empty() {
                with = sim.fight[0] as u8;
                Act::Wrapped
            } else if matches!(a.job, Job::Passer) {
                Act::Pocket
            } else if history(prev, i, 2).any(|x| x == Act::Catch)
                && history(prev, i, 3).filter(|x| *x == Act::Catch).count() < 3
            {
                Act::Catch
            } else {
                Act::Carry
            }
        } else {
            match a.job {
                Job::Rush { .. } => Act::Rush,
                Job::Man { .. } | Job::Zone { .. } | Job::Spy => Act::Cover,
                Job::Fake { .. } => Act::Fake,
                Job::Passer => Act::Pocket,
                _ if speed > 1.2 => Act::Run,
                _ => Act::Stand,
            }
        };
        poses.push(Pose { face: face.y.atan2(face.x) as f32, act, with });
    }
    Frame {
        t,
        ball,
        players: sim.agents.iter().map(|a| (a.id, a.pos.x as f32, a.pos.y as f32)).collect(),
        poses,
        ball_z: ball_z(sim) as f32,
        ball_mode: mode,
        holder,
    }
}

/// A frame's facing as a unit vector.
pub fn facing(p: &Pose) -> V2 {
    super::geom::v2((p.face as f64).cos(), (p.face as f64).sin())
}
