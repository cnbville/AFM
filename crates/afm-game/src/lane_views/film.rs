//! The film of one of your games for the 2D and 3D views (later extras
//! topics 2 and 3, screens topic 12). The game is played again from its
//! seed with frames on, so it is the same game snap for snap, and the film
//! is kept only while you watch it.
//!
//! Every snap comes out in the stadium's own frame: x from the left end
//! line (0) to the right one (120), y across from the far sideline (0) to
//! the near one (53.3), so the 2D and 3D views, the drawings and every
//! camera agree. The engine tracks plays from the snap and kick returns
//! from the catch; the film adds what a camera would see around them: the
//! set before the snap, the kick in the air, and the plays the engine
//! settles without tracking (field goals, extra points, kneels, spikes and
//! fouls before the snap), drawn from the play log.

use std::collections::HashMap;

use afm_engine::game::geom::WIDTH;
use afm_engine::game::pose::{Act, BallMode, NONE, Pose};
use afm_engine::game::sim::Frame;
use afm_engine::game::stats::{GameStats, PlayRecord, PlayType};
use afm_engine::game::{GameResult, ReplaySetup};
use afm_engine::positions::Position;
use afm_engine::world::World;
use afm_engine::world::ethnicity::Ethnicity;
use afm_engine::world::seat::SeatGame;
use serde_json::{Value, json};

use super::{numbers, venue};

/// A kneel, for the film only (the engine never tracks one).
const KNEEL: u8 = 18;
/// Seconds of the set before the snap.
const PRE: f64 = 1.4;

/// One player as the views draw him.
#[derive(Clone, Debug)]
pub struct Body {
    /// 0 home, 1 away.
    pub side: usize,
    pub num: u8,
    pub pos: Position,
    pub name: String,
    pub height: f64,
    pub weight: f64,
    pub arm: f64,
    pub skin: String,
}

/// A game's film while it is being watched.
pub struct Film {
    pub key: (u32, [usize; 2]),
    pub res: GameResult,
    pub setup: ReplaySetup,
    pub bodies: HashMap<u32, Body>,
    /// Your side in the game (0 home, 1 away), when it is your team's.
    pub mine: Option<usize>,
    pub level: afm_engine::player::Level,
    pub year: i32,
    pub frames: HashMap<u32, usize>,
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^ (x >> 33)
}

/// A skin tone from a player's heritage, varied the way real skin varies
/// within each group, fixed by his id.
fn skin(e: Option<Ethnicity>, id: u32) -> String {
    let u = (mix(id as u64 ^ 0x534B) % 1000) as f64 / 1000.0;
    let (a, b): ([f64; 3], [f64; 3]) = match e {
        Some(Ethnicity::White) => ([241.0, 205.0, 172.0], [205.0, 152.0, 112.0]),
        Some(Ethnicity::Black) => ([150.0, 98.0, 62.0], [62.0, 38.0, 26.0]),
        Some(Ethnicity::Hispanic) => ([224.0, 178.0, 132.0], [158.0, 108.0, 70.0]),
        Some(Ethnicity::Polynesian) => ([192.0, 138.0, 94.0], [128.0, 84.0, 54.0]),
        Some(Ethnicity::Asian) => ([238.0, 200.0, 160.0], [196.0, 150.0, 106.0]),
        None => ([200.0, 150.0, 110.0], [120.0, 80.0, 52.0]),
    };
    let c: Vec<u8> = (0..3).map(|i| (a[i] + (b[i] - a[i]) * u).round() as u8).collect();
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

impl Film {
    /// Plays one of your kept games again with frames on.
    pub fn open(w: &World, g: &SeatGame) -> Result<Film, String> {
        if g.replay.is_empty() {
            return Err("This game was played before the film room existed, so it has no film.".into());
        }
        let setup = ReplaySetup::unpack(&g.replay).ok_or("This game's film could not be read.")?;
        let res = setup.play(true);
        let year = g.date.year();
        let level = setup.rules.level;
        let nfl = level == afm_engine::player::Level::NFL;
        let people: HashMap<u32, usize> = {
            let want: std::collections::HashSet<u32> =
                setup.home.players.iter().chain(&setup.away.players).map(|p| p.id).collect();
            w.people.iter().enumerate().filter(|(_, p)| want.contains(&p.player.id)).map(|(i, p)| (p.player.id, i)).collect()
        };
        let mut bodies = HashMap::new();
        for (side, team) in [&setup.home, &setup.away].into_iter().enumerate() {
            let list: Vec<(u32, Position)> = team.players.iter().map(|p| (p.id, p.position)).collect();
            let nums: HashMap<u32, u8> = numbers::assign(&list, nfl, year).into_iter().collect();
            for p in &team.players {
                let person = people.get(&p.id).map(|&i| &w.people[i]);
                bodies.insert(
                    p.id,
                    Body {
                        side,
                        num: nums.get(&p.id).copied().unwrap_or(0),
                        pos: p.position,
                        name: person.map_or_else(|| format!("#{}", p.id), |x| x.name.clone()),
                        height: p.height,
                        weight: p.weight,
                        arm: p.arm,
                        skin: skin(person.map(|x| x.ethnicity), p.id),
                    },
                );
            }
        }
        let mine = match w.seat {
            Some(s) => {
                let t = match s.level {
                    afm_engine::calendar::Level::Nfl => s.nfl_team,
                    _ => s.program,
                };
                t.and_then(|t| g.teams.iter().position(|&x| x == t)).filter(|_| s.level == g.level)
            }
            None => None,
        };
        let frames = res.frames.iter().enumerate().map(|(k, (n, _))| (*n, k)).collect();
        Ok(Film { key: (g.date.0 as u32, g.teams), res, setup, bodies, mine, level, year, frames })
    }

    pub fn side_of(&self, id: u32) -> Option<usize> {
        self.bodies.get(&id).map(|b| b.side)
    }

    /// The film's index: the teams, the venue and every play with its tags
    /// (for cut-ups), whether it has film, and its plain words.
    pub fn index(&self, w: &World, g: &SeatGame, k: usize) -> Value {
        let page = crate::views::game(w, k).ok();
        let texts = page.as_ref().and_then(|p| p.get("plays")).and_then(Value::as_array);
        let s = &self.res.stats;
        let colors = |side: usize| {
            let t = g.teams[side];
            match g.level {
                afm_engine::calendar::Level::Nfl => crate::colors::nfl(&w.nfl_team_name(t), t),
                afm_engine::calendar::Level::HighSchool => crate::colors::high_school(t),
                _ => crate::colors::college(&w.programs[t].real_name, t),
            }
        };
        let teams: Vec<Value> = (0..2)
            .map(|side| {
                let refj = match g.level {
                    afm_engine::calendar::Level::Nfl => json!({ "kind": "nfl", "id": g.teams[side] }),
                    afm_engine::calendar::Level::HighSchool => json!({ "kind": "hs", "id": g.teams[side] }),
                    _ => json!({ "kind": "college", "id": g.teams[side] }),
                };
                json!({ "name": g.names[side], "short": short(&g.names[side]), "colors": colors(side), "ref": refj, "score": s.score[side] })
            })
            .collect();
        let mut bodies: Vec<(&u32, &Body)> = self.bodies.iter().collect();
        bodies.sort_by_key(|(id, b)| (b.side, b.num, **id));
        let players: Vec<Value> = bodies
            .iter()
            .map(|(id, b)| {
                json!([**id, b.side, b.num, b.pos.name(), b.name, (b.height * 10.0).round() / 10.0, b.weight.round(), (b.arm * 10.0).round() / 10.0, b.skin])
            })
            .collect();
        let plays: Vec<Value> = s
            .plays
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let t = texts.and_then(|a| a.get(i));
                let txt = |k: &str| t.and_then(|v| v.get(k)).cloned().unwrap_or(Value::Null);
                let wp_after = s.plays.get(i + 1).map_or(if s.score[0] > s.score[1] { 1.0 } else { 0.0 }, |n| {
                    if n.offense == r.offense { n.wp_before } else { 1.0 - n.wp_before }
                });
                let swing = wp_after - r.wp_before;
                json!({
                    "n": r.n,
                    "i": i,
                    "q": r.quarter,
                    "clock": txt("clock"),
                    "off": r.offense.min(1),
                    "kind": format!("{:?}", r.kind),
                    "down": txt("down"),
                    "togo": txt("togo"),
                    "spot": txt("spot"),
                    "text": t.and_then(|v| v.get("text")).cloned().unwrap_or_else(|| Value::from(r.why.clone())),
                    "why": r.why,
                    "call": r.call,
                    "defcall": r.defense_call,
                    "personnel": r.personnel,
                    "score": r.score,
                    "film": true,
                    "tracked": self.frames.contains_key(&r.n),
                    "yards": (r.yards * 10.0).round() / 10.0,
                    "epa": (r.epa * 100.0).round() / 100.0,
                    "wp": (r.wp_before * 1000.0).round() / 1000.0,
                    "swing": (swing * 1000.0).round() / 1000.0,
                    "key": key_play(r, swing),
                    "tags": tags(r),
                    "on": [r.on_field[0].clone(), r.on_field[1].clone()],
                })
            })
            .collect();
        json!({
            "index": k,
            "date": crate::views::date_text(g.date),
            "level": match g.level {
                afm_engine::calendar::Level::Nfl => "nfl",
                afm_engine::calendar::Level::HighSchool => "hs",
                _ => "college",
            },
            "year": self.year,
            "hash": self.setup.rules.hash,
            "mine": self.mine,
            "teams": teams,
            "venue": venue::venue(w, g, &self.setup),
            "players": players,
            "plays": plays,
            "quarters": s.quarters,
            "marks": g.marks.iter().map(|m| json!({ "play": m.play, "t": m.t, "kind": m.kind, "color": m.color, "pts": m.pts })).collect::<Vec<_>>(),
        })
    }

    /// One play's film, in the stadium's frame.
    pub fn play(&self, n: u32) -> Result<Value, String> {
        let s = &self.res.stats;
        let r = s.plays.iter().find(|p| p.n == n).ok_or("No such play.")?;
        let clip = match self.frames.get(&n) {
            Some(&k) => {
                let frames = &self.res.frames[k].1;
                if matches!(r.kind, PlayType::Kickoff | PlayType::Punt) {
                    self.kick_clip(r, frames)
                } else {
                    self.snap_clip(r, frames)
                }
            }
            None => self.drawn_clip(r),
        };
        Ok(self.clip_json(r, clip))
    }

    /// The set before the snap and the snap itself, then the engine's frames.
    fn snap_clip(&self, r: &PlayRecord, frames: &[Frame]) -> Clip {
        let first = &frames[0];
        let ids: Vec<u32> = first.players.iter().map(|p| p.0).collect();
        let snapper = self.team_of_frame(first, r);
        let los = 100.0 - r.yardline_100;
        // The center: the offensive lineman nearest the ball's spot across.
        let qb = if first.holder != NONE { first.holder as usize } else { 0 };
        let snap_y = first.players.get(qb).map_or(WIDTH / 2.0, |p| p.2 as f64);
        let center = (0..ids.len())
            .filter(|&i| self.side_of(ids[i]) == Some(snapper))
            .filter(|&i| self.bodies.get(&ids[i]).is_some_and(|b| matches!(b.pos, Position::IOL | Position::OT | Position::LS)))
            .min_by(|&a, &b| {
                let d = |i: usize| {
                    let p = first.players[i];
                    (p.1 as f64 - los).abs() + (p.2 as f64 - snap_y).abs()
                };
                d(a).total_cmp(&d(b))
            });
        let mut out = Clip::new(ids.clone(), snapper);
        let steps = (PRE / 0.1).round() as usize;
        for k in 0..=steps {
            let t = -PRE + k as f64 * 0.1;
            let mut f = CFrame::from_engine(first, t);
            for p in &mut f.p {
                p.3 = Act::Stance as u8;
                p.4 = NONE;
            }
            // Facing the other way: every man squared to the line.
            for (i, p) in f.p.iter_mut().enumerate() {
                p.2 = if self.side_of(ids[i]) == Some(snapper) { 0.0 } else { std::f64::consts::PI };
            }
            match center {
                Some(c) => {
                    let cp = first.players[c];
                    f.ball = (los.min(cp.1 as f64 + 0.6), cp.2 as f64, 0.15);
                    f.mode = BallMode::Held as u8;
                    f.holder = c as u8;
                }
                None => {
                    f.ball = (los, snap_y, 0.15);
                    f.mode = BallMode::Dead as u8;
                    f.holder = NONE;
                }
            }
            out.frames.push(f);
        }
        for fr in frames {
            out.frames.push(CFrame::from_engine(fr, fr.t.max(0.05)));
        }
        out.hold_end(0.9);
        out
    }

    /// Which team sim side 0 is: the man with the ball in the first frame,
    /// else the play's offense.
    fn team_of_frame(&self, first: &Frame, r: &PlayRecord) -> usize {
        if first.holder != NONE
            && let Some(p) = first.players.get(first.holder as usize)
            && let Some(s) = self.side_of(p.0)
        {
            return s;
        }
        r.offense.min(1)
    }

    /// A kick in the air before the engine's return frames: the kicking
    /// team covers from its line, the ball flies the kick's distance.
    fn kick_clip(&self, r: &PlayRecord, frames: &[Frame]) -> Clip {
        let first = &frames[0];
        let ids: Vec<u32> = first.players.iter().map(|p| p.0).collect();
        let returner = self.team_of_frame(first, r);
        let catch = (first.ball.x, first.ball.y);
        let kick_x = (catch.0 + r.kick_distance).min(100.0);
        let punt = r.kind == PlayType::Punt;
        let from = if punt { kick_x + 14.0 } else { kick_x };
        let hang = if punt { 3.9 + (r.kick_distance - 40.0) * 0.02 } else { 3.6 + (r.kick_distance - 55.0) * 0.02 }.clamp(2.8, 4.8);
        // Pre-kick: the punt snap or the kicker's approach.
        let lead = if punt { 2.3 } else { 1.6 };
        let mut out = Clip::new(ids.clone(), returner);
        let n = ids.len();
        let nfl_kick = self.setup.rules.nfl_kickoff && !punt;
        // Where each man starts: the coverage across its line, the return
        // unit back near its own line or, on a punt, at the line.
        let mut cover: Vec<usize> = (0..n).filter(|&i| self.side_of(ids[i]) != Some(returner)).collect();
        let mut ret: Vec<usize> = (0..n).filter(|&i| self.side_of(ids[i]) == Some(returner)).collect();
        cover.sort_by(|&a, &b| first.players[a].2.total_cmp(&first.players[b].2));
        ret.sort_by(|&a, &b| first.players[a].2.total_cmp(&first.players[b].2));
        let mut start = vec![(0.0f64, 0.0f64); n];
        let kicker = cover.iter().copied().find(|&i| self.bodies.get(&ids[i]).is_some_and(|b| matches!(b.pos, Position::K | Position::P)));
        let snapper = cover.iter().copied().find(|&i| self.bodies.get(&ids[i]).is_some_and(|b| b.pos == Position::LS));
        let line_x = if nfl_kick { 40.0 } else if punt { kick_x } else { kick_x + 1.0 };
        let mut k2 = 0usize;
        let cover_n = cover.len().saturating_sub(kicker.is_some() as usize).max(1);
        for &i in &cover {
            if Some(i) == kicker {
                start[i] = (from + if punt { 0.0 } else { 7.0 }, WIDTH / 2.0 - 1.0);
                continue;
            }
            if punt && Some(i) == snapper {
                start[i] = (kick_x + 0.4, WIDTH / 2.0);
                k2 += 1;
                continue;
            }
            let y = if punt {
                // A punt line: splits inside, gunners split wide.
                let slots = [2.0, 6.0, 16.0, 22.0, 24.6, 28.7, 31.3, 37.0, 47.3, 51.3];
                slots.get(k2).copied().unwrap_or(WIDTH / 2.0)
            } else {
                4.0 + (WIDTH - 8.0) * k2 as f64 / (cover_n.max(2) - 1) as f64
            };
            let x = if punt && (6.0..=47.0).contains(&y) && !(15.0..=38.0).contains(&y) { line_x + 0.3 } else if punt { line_x + 0.6 } else { line_x };
            start[i] = (x, y);
            k2 += 1;
        }
        let back = if nfl_kick { 32.5 } else if punt { kick_x - 1.0 } else { (kick_x - 15.0).max(25.0) };
        for (j, &i) in ret.iter().enumerate() {
            let p = first.players[i];
            let deep = (p.1 as f64) < catch.0 + 6.0 && (p.1 as f64 - catch.0).abs() < 8.0;
            start[i] = if deep {
                (p.1 as f64, p.2 as f64)
            } else {
                let y = 5.0 + (WIDTH - 10.0) * j as f64 / (ret.len().max(2) - 1) as f64;
                (back, y)
            };
        }
        let total = lead + hang;
        let steps = (total / 0.1).round() as usize;
        for k in 0..=steps {
            let t = -total + k as f64 * 0.1;
            let since = t + total;
            let mut f = CFrame::from_engine(first, t);
            for i in 0..n {
                let (sx, sy) = start[i];
                let (ex, ey) = (first.players[i].1 as f64, first.players[i].2 as f64);
                // Coverage releases at the kick (at the catch in the 2025
                // NFL format); the return unit sets and then turns to block.
                let go = if nfl_kick && self.side_of(ids[i]) != Some(returner) {
                    ((since - lead - hang * 0.82) / (hang * 0.18)).clamp(0.0, 1.0)
                } else {
                    ((since - lead * if punt { 0.6 } else { 1.0 }) / (total - lead * if punt { 0.6 } else { 1.0 })).clamp(0.0, 1.0)
                };
                let u = go * go * (3.0 - 2.0 * go);
                let (x, y) = (sx + (ex - sx) * u, sy + (ey - sy) * u);
                let moving = go > 0.0 && go < 1.0;
                let face = if self.side_of(ids[i]) == Some(returner) { 0.0 } else { std::f64::consts::PI };
                let face = if moving && ((ex - sx).abs() + (ey - sy).abs()) > 1.0 { (ey - sy).atan2(ex - sx) } else { face };
                f.p[i] = (x, y, face, if moving { Act::Run as u8 } else if since < lead { Act::Stance as u8 } else { Act::Stand as u8 }, NONE);
            }
            // The ball: snapped back and punted, or kicked off the tee.
            if since < lead {
                if punt {
                    let snap_at = lead - 1.25;
                    if since < snap_at {
                        f.ball = (kick_x + 0.4, WIDTH / 2.0, 0.15);
                    } else {
                        let u = ((since - snap_at) / 0.8).min(1.0);
                        f.ball = (kick_x + 0.4 + 13.6 * u, WIDTH / 2.0, 0.4 + 0.6 * u);
                    }
                } else {
                    f.ball = (from, WIDTH / 2.0, 0.12);
                    if let Some(kk) = kicker {
                        let u = (since / lead).clamp(0.0, 1.0);
                        f.p[kk] = (from + 7.0 - 7.2 * u, WIDTH / 2.0 - 1.0 + u, std::f64::consts::PI, if u > 0.05 { Act::Run as u8 } else { Act::Stance as u8 }, NONE);
                    }
                }
                f.mode = BallMode::Dead as u8;
                f.holder = NONE;
                if punt && let Some(kk) = kicker {
                    f.p[kk].0 = from;
                    f.p[kk].1 = WIDTH / 2.0;
                    f.p[kk].3 = Act::Stand as u8;
                }
            } else {
                let u = ((since - lead) / hang).clamp(0.0, 1.0);
                let tt = since - lead;
                let (bx, by) = (from + (catch.0 - from) * u, WIDTH / 2.0 + (catch.1 - WIDTH / 2.0) * u);
                let z0 = if punt { 1.0 } else { 0.1 };
                let z = z0 + (1.3 - z0) * u + 0.5 * (32.174 / 3.0) * tt * (hang - tt);
                f.ball = (bx, by, z);
                f.mode = BallMode::Air as u8;
                f.holder = NONE;
            }
            out.frames.push(f);
        }
        for fr in frames {
            out.frames.push(CFrame::from_engine(fr, fr.t.max(0.05)));
        }
        out.hold_end(0.9);
        out
    }

    /// A play the engine settles without tracking, drawn from the log:
    /// field goals and extra points, kneels, spikes, fouls before the snap.
    fn drawn_clip(&self, r: &PlayRecord) -> Clip {
        let off = r.offense.min(1);
        let def = 1 - off;
        let team = |side: usize| if side == 0 { &self.setup.home } else { &self.setup.away };
        // Who was out there: the log's men, else the dressed team's first
        // man at each spot.
        let pick = |side: usize, listed: &[u32], want: &[Position]| -> Vec<(u32, Position)> {
            if listed.len() >= 9 {
                return listed.iter().filter_map(|id| self.bodies.get(id).map(|b| (*id, b.pos))).collect();
            }
            let mut got = Vec::new();
            for &p in want {
                if let Some(pl) = team(side).players.iter().find(|x| x.position == p && !got.iter().any(|g: &(u32, Position)| g.0 == x.id)) {
                    got.push((pl.id, p));
                }
            }
            got
        };
        use Position::*;
        let kick = matches!(r.kind, PlayType::FieldGoal | PlayType::ExtraPoint);
        let o_want: &[Position] =
            if kick { &[LS, IOL, IOL, IOL, IOL, OT, OT, TE, TE, P, K] } else { &[QB, RB, WR, WR, WR, TE, OT, OT, IOL, IOL, IOL] };
        let d_want: &[Position] = if kick { &[DT, DT, DT, EDGE, EDGE, LB, LB, LB, CB, CB, S] } else { &[DT, DT, EDGE, EDGE, LB, LB, LB, CB, CB, S, S] };
        let o = pick(off, &r.on_field[0], o_want);
        let d = pick(def, &r.on_field[1], d_want);
        let los = if r.kind == PlayType::ExtraPoint { 100.0 - self.setup.rules.xp_from } else { 100.0 - r.yardline_100 };
        let mid = WIDTH / 2.0;
        let mut ids = Vec::new();
        let mut at: Vec<(f64, f64)> = Vec::new();
        // The offense's spots.
        let mut ol = 0;
        let mut te = 0;
        let mut wr = 0;
        let mut rb = 0;
        let mut holder = None;
        let mut kicker = None;
        let mut snapper = None;
        let mut qb = None;
        for &(id, pos) in &o {
            let k = ids.len();
            ids.push(id);
            let spot = if kick {
                match pos {
                    K => {
                        kicker = Some(k);
                        (los - 9.5, mid + 1.6)
                    }
                    P | QB => {
                        holder = Some(k);
                        (los - 7.0, mid)
                    }
                    LS if snapper.is_none() => {
                        snapper = Some(k);
                        (los - 0.5, mid)
                    }
                    _ => {
                        let slots: [f64; 10] = [-1.2, 1.2, -2.4, 2.4, -3.6, 3.6, -4.6, 4.6, -5.2, 5.2];
                        let y = slots.get(ol).copied().unwrap_or(0.0);
                        ol += 1;
                        let back = if y.abs() > 4.0 { 1.3 } else { 0.6 };
                        (los - back, mid + y)
                    }
                }
            } else {
                match pos {
                    QB => {
                        qb = Some(k);
                        holder = Some(k);
                        (los - 1.1, mid)
                    }
                    RB => {
                        rb += 1;
                        (los - 6.5 - rb as f64 * 0.5, mid + if rb > 1 { 1.5 } else { 0.0 })
                    }
                    WR => {
                        let ys = [mid + 19.0, mid - 19.0, mid + 12.0, mid - 12.0];
                        let y = ys.get(wr).copied().unwrap_or(mid + 8.0);
                        wr += 1;
                        (los - if wr == 3 { 1.2 } else { 0.5 }, y)
                    }
                    TE => {
                        let y = if te == 0 { mid - 4.0 } else { mid + 4.0 };
                        te += 1;
                        (los - 0.7, y)
                    }
                    _ => {
                        let slots = [0.0, -1.3, 1.3, -2.6, 2.6, -3.8, 3.8];
                        let y = slots.get(ol).copied().unwrap_or(0.0);
                        if ol == 0 {
                            snapper = Some(k);
                        }
                        ol += 1;
                        (los - 0.5, mid + y)
                    }
                }
            };
            at.push(spot);
        }
        let n_off = ids.len();
        let mut dl = 0;
        let mut lb = 0;
        let mut db = 0;
        for &(id, pos) in &d {
            ids.push(id);
            let spot = match pos {
                DT | EDGE => {
                    let slots: &[f64] = if kick { &[-0.6, 0.6, -1.8, 1.8, -3.0, 3.0, -4.2, 4.2, -5.6, 5.6] } else { &[-1.0, 1.0, -3.4, 3.4, -4.6, 4.6] };
                    let y = slots.get(dl).copied().unwrap_or(0.0);
                    dl += 1;
                    (los + 0.9, mid + y)
                }
                LB => {
                    let slots: &[f64] = if kick { &[-6.4, 6.4, -7.4, 7.4] } else { &[0.0, -4.0, 4.0] };
                    let y = slots.get(lb).copied().unwrap_or(0.0);
                    lb += 1;
                    (los + if kick { 1.0 } else { 4.5 }, mid + y)
                }
                _ => {
                    let slots: &[(f64, f64)] = if kick {
                        &[(10.0, 0.0), (1.2, -9.0), (1.2, 9.0), (8.0, -6.0)]
                    } else {
                        &[(6.0, 19.0), (6.0, -19.0), (12.0, 7.0), (12.0, -7.0), (6.0, 12.0)]
                    };
                    let (dx, y) = slots.get(db).copied().unwrap_or((8.0, 0.0));
                    db += 1;
                    (los + dx, mid + y)
                }
            };
            at.push(spot);
        }
        let n = ids.len();
        let mut out = Clip::new(ids.clone(), off);
        let why = r.why.to_lowercase();
        let snap_at = 1.0;
        let end: f64 = if kick { 4.6 } else if r.kind == PlayType::NoPlay { 2.6 } else { 3.0 };
        let fouler = r.penalties.first().and_then(|p| ids.iter().position(|&x| x == p.2));
        // A field goal's flight: through the posts or past them.
        let posts_x = 110.0;
        let hold = (los - 7.0, mid);
        let kick_t = snap_at + 1.25;
        let blocked = r.blocked || why.contains("blocked");
        let miss_y = if why.contains("left") { -1.0 } else if why.contains("right") { 1.0 } else { 0.0 };
        let half_posts = if self.level == afm_engine::player::Level::HS { 23.33 / 6.0 } else { 18.5 / 6.0 };
        let short = why.contains("short");
        let good = r.kick_good || (kick && !blocked && !why.contains("wide") && !short && !why.contains("no good") && !why.contains("missed"));
        // The kick goes toward the posts, a little left or right of the
        // middle when it misses wide (the offense's left is higher y).
        let land_y = mid + if good { 0.0 } else { miss_y * (half_posts + 1.4) };
        let flight = 1.2 + (posts_x - hold.0) / 45.0;
        let steps = ((end + 1.0) / 0.1).round() as usize;
        for k in 0..=steps {
            let t = -1.0 + k as f64 * 0.1;
            let mut f = CFrame { t, ball: (los, mid, 0.15), mode: BallMode::Held as u8, holder: snapper.map_or(NONE, |s| s as u8), p: Vec::with_capacity(n) };
            for i in 0..n {
                let (x, y) = at[i];
                let mine = i < n_off;
                let face = if mine { 0.0 } else { std::f64::consts::PI };
                f.p.push((x, y, face, Act::Stance as u8, NONE));
            }
            let live = t >= snap_at && r.kind != PlayType::NoPlay;
            if r.kind == PlayType::NoPlay {
                // The foul: the man flinches, the flag flies, the whistle.
                if let Some(fi) = fouler
                    && t >= 0.6
                {
                    let u = ((t - 0.6) / 0.4).min(1.0);
                    let dir = if fi < n_off { 1.0 } else { -1.0 };
                    f.p[fi].0 += dir * 0.7 * u;
                    f.p[fi].3 = Act::Run as u8;
                }
                if t >= 1.2 {
                    for p in &mut f.p {
                        p.3 = Act::Stand as u8;
                    }
                }
                f.mode = BallMode::Dead as u8;
                f.holder = NONE;
            } else if live {
                let since = t - snap_at;
                if kick {
                    // Up front, the protection holds its gaps.
                    for i in 0..n {
                        if Some(i) == kicker || Some(i) == holder {
                            continue;
                        }
                        let (x, y) = at[i];
                        if i < n_off {
                            f.p[i] = (x - 0.4 * since.min(1.0), y, 0.0, Act::Block as u8, NONE);
                        } else if (x - los).abs() < 2.5 {
                            f.p[i] = (x - 0.6 * since.min(1.0), y, std::f64::consts::PI, Act::Engaged as u8, NONE);
                        } else {
                            f.p[i] = (x - 1.5 * since.min(1.0), y, std::f64::consts::PI, Act::Run as u8, NONE);
                        }
                    }
                    // Pair each rusher with the nearest blocker.
                    for i in n_off..n {
                        if f.p[i].3 == Act::Engaged as u8 {
                            let (x, y) = (f.p[i].0, f.p[i].1);
                            if let Some(b) = (0..n_off).filter(|&b| f.p[b].3 == Act::Block as u8).min_by(|&a, &b| {
                                let d = |j: usize| (f.p[j].0 - x).hypot(f.p[j].1 - y);
                                d(a).total_cmp(&d(b))
                            }) {
                                f.p[i].4 = b as u8;
                                f.p[b].4 = i as u8;
                            }
                        }
                    }
                    if let Some(h) = holder {
                        f.p[h] = (hold.0 + 0.3, hold.1, std::f64::consts::PI / 2.0, if since > 0.3 { KNEEL } else { Act::Stance as u8 }, NONE);
                    }
                    if let Some(kk) = kicker {
                        let u = ((since - 0.2) / 1.05).clamp(0.0, 1.0);
                        let (sx, sy) = at[kk];
                        let (ex, ey) = (hold.0 - 0.7, hold.1 + 0.5);
                        f.p[kk] = (sx + (ex - sx) * u, sy + (ey - sy) * u, 0.0, if u > 0.0 && u < 1.0 { Act::Run as u8 } else { Act::Stand as u8 }, NONE);
                        if t > kick_t {
                            f.p[kk].0 = ex + ((t - kick_t) * 1.2).min(1.2);
                        }
                    }
                    if t < snap_at + 0.35 {
                        let u = since / 0.35;
                        f.ball = (los + (hold.0 - los) * u, mid, 0.3 + 0.5 * u);
                        f.mode = BallMode::Air as u8;
                        f.holder = NONE;
                    } else if t < kick_t {
                        f.ball = (hold.0, hold.1, 0.25);
                        f.mode = BallMode::Held as u8;
                        f.holder = holder.map_or(NONE, |h| h as u8);
                    } else if blocked {
                        let u = ((t - kick_t) / 0.9).min(1.0);
                        f.ball = (hold.0 + 3.0 * u, mid + 1.5 * u, 0.25 + 2.6 * u * (1.0 - u) * 2.0);
                        f.mode = if u < 1.0 { BallMode::Air as u8 } else { BallMode::Loose as u8 };
                        f.holder = NONE;
                    } else {
                        let ft = t - kick_t;
                        let u = (ft / flight).min(1.25);
                        let dist = posts_x + 8.0 - hold.0;
                        let reach = if short { 0.9 } else { 1.0 };
                        let x = hold.0 + dist * reach * u;
                        let y = mid + (land_y - mid) * u;
                        // An arc that clears the bar (3.3 yards) when good and
                        // dies under it when short.
                        let apex = if short { 5.5 } else { 7.0 + (posts_x - hold.0) / 9.0 };
                        let z = (0.25 + apex * 4.0 * u * (1.0 - u * 0.78)).max(0.0);
                        f.ball = (x.min(posts_x + 9.0), y, z);
                        f.mode = if z > 0.05 { BallMode::Air as u8 } else { BallMode::Dead as u8 };
                        f.holder = NONE;
                    }
                } else {
                    // A kneel or a spike under center.
                    for p in f.p.iter_mut() {
                        p.3 = Act::Stand as u8;
                    }
                    if let Some(q) = qb {
                        let (x, y) = at[q];
                        if r.kind == PlayType::Kneel {
                            f.p[q] = (x - 1.0 * since.min(0.5) * 2.0, y, 0.0, if since > 0.5 { KNEEL } else { Act::Run as u8 }, NONE);
                            f.ball = (f.p[q].0 + 0.3, y, if since > 0.5 { 0.5 } else { 1.0 });
                            f.mode = BallMode::Held as u8;
                            f.holder = q as u8;
                        } else {
                            let u = ((since - 0.4) / 0.25).clamp(0.0, 1.0);
                            f.p[q] = (x - 0.6, y, 0.0, if since > 0.3 && since < 0.9 { Act::Throw as u8 } else { Act::Stand as u8 }, NONE);
                            f.ball = (x - 0.3 + 1.4 * u, y, 1.6 * (1.0 - u) + 0.1);
                            f.mode = if u < 1.0 { BallMode::Held as u8 } else { BallMode::Loose as u8 };
                            f.holder = if u < 1.0 { q as u8 } else { NONE };
                        }
                    }
                }
            }
            out.frames.push(f);
        }
        out.drawn = true;
        out
    }

    fn clip_json(&self, r: &PlayRecord, mut clip: Clip) -> Value {
        // Which way the clip's team attacks: home goes right in the first
        // and third quarters and in overtime, left in the second and fourth.
        let right = (clip.team == 0) == (r.quarter % 2 == 1 || r.quarter > 4);
        let map = |x: f64, y: f64| if right { (x + 10.0, WIDTH - y) } else { (110.0 - x, y) };
        let face = |a: f64| if right { -a } else { std::f64::consts::PI - a };
        let mut flat: Vec<f64> = Vec::with_capacity(clip.frames.len() * (6 + clip.ids.len() * 5));
        for f in &clip.frames {
            let (bx, by) = map(f.ball.0, f.ball.1);
            flat.extend_from_slice(&[r2(f.t), r2(bx), r2(by), r2(f.ball.2), f.mode as f64, f.holder as f64]);
            for p in &f.p {
                let (x, y) = map(p.0, p.1);
                flat.extend_from_slice(&[r2(x), r2(y), r2(face(p.2)), p.3 as f64, p.4 as f64]);
            }
        }
        // The line and the line to gain in the stadium's frame, for the
        // offense of the play (kicks have none).
        let scrim = !matches!(r.kind, PlayType::Kickoff | PlayType::Punt);
        let off_right = (r.offense.min(1) == 0) == (r.quarter % 2 == 1 || r.quarter > 4);
        let to_stadium = |x: f64| if off_right { x + 10.0 } else { 110.0 - x };
        let los_x = if r.kind == PlayType::ExtraPoint {
            100.0 - self.setup.rules.xp_from
        } else if r.kind == PlayType::TwoPoint {
            100.0 - self.setup.rules.two_point_from
        } else {
            100.0 - r.yardline_100
        };
        let goal_to_go = r.togo >= r.yardline_100 - 0.5;
        let ltg = if matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble | PlayType::Kneel | PlayType::Spike | PlayType::NoPlay | PlayType::Punt | PlayType::FieldGoal) && r.down > 0 {
            Some(if goal_to_go { 100.0 } else { (los_x + r.togo).min(100.0) })
        } else {
            None
        };
        // Grades: your position coaches grade your own men only.
        let grades = self.grades(r, &clip);
        let flags: Vec<Value> = r
            .penalties
            .iter()
            .map(|&(kind, _team, id, yds, accepted)| {
                json!({ "id": id, "what": kind.name(), "yards": yds.round(), "accepted": accepted, "t": clip.foul_time(id) })
            })
            .collect();
        clip.ids.shrink_to_fit();
        json!({
            "n": r.n,
            "kind": format!("{:?}", r.kind),
            "ids": clip.ids,
            "team": clip.team,
            "drawn": clip.drawn,
            "stride": 6 + clip.ids.len() * 5,
            "f": flat,
            "los": if scrim { Some(r2(to_stadium(los_x))) } else { None },
            "ltg": ltg.filter(|_| scrim).map(|x| r2(to_stadium(x))),
            "series": ltg.filter(|_| scrim && !goal_to_go).map(|x| r2(to_stadium(x - 10.0))),
            "dir": if off_right { 1 } else { -1 },
            "passer": r.passer,
            "target": r.target,
            "carrier": r.carrier,
            "defender": r.defender,
            "ng": {
                "ttt": if r.time_to_throw > 0.0 { Some(r2(r.time_to_throw)) } else { None },
                "sep": if r.target.is_some() { Some(r2(r.separation)) } else { None },
                "air": r2(r.air_yards),
                "yac": r2(r.yac),
                "top": [r.top_speed.0, r2(r.top_speed.1)],
                "epa": r2(r.epa),
                "wp": r2(r.wp_before),
                "ybc": r2(r.before_contact),
                "yards": r2(r.yards),
                "pressured": r.pressured,
                "blitz": r.blitz,
                "box": r.box_count,
                "pa": r.play_action,
            },
            "flags": flags,
            "grades": grades,
        })
    }

    /// Each of your men's grade on this snap, the way position coaches grade
    /// film the next day: plus or minus on the snap, with what he did on
    /// his assignment, his technique and his effort.
    fn grades(&self, r: &PlayRecord, clip: &Clip) -> Value {
        let Some(me) = self.mine else { return json!({}) };
        let mut st = GameStats::default();
        st.book(r);
        let mut out = serde_json::Map::new();
        let name = |id: u32| self.bodies.get(&id).map_or(format!("#{id}"), |b| format!("#{} {}", b.num, b.name.split(' ').next_back().unwrap_or("")));
        let loafs = clip.loafs(self, me);
        for side_ids in &r.on_field {
            for &id in side_ids {
                if self.side_of(id) != Some(me) {
                    continue;
                }
                let line = st.players[me].get(&id);
                let mut g = line.map_or(0.0, |l| l.grade_sum);
                let mut notes: Vec<String> = Vec::new();
                let mut assignment = 1i8;
                let mut technique = 0i8;
                let mut effort = 1i8;
                if r.busts.contains(&id) {
                    notes.push("Busted assignment".into());
                    assignment = -1;
                    g -= 1.0;
                }
                for &(b, d, pass, won, held, pressure) in &r.blocks {
                    if b == id {
                        if won {
                            technique = technique.max(1);
                            g += if pass { 0.15 } else { 0.35 };
                            notes.push(format!("Won his block on {} ({:.1} s)", name(d), held));
                        } else {
                            technique = -1;
                            g -= if pressure { 0.7 } else { 0.4 };
                            notes.push(format!("Beaten by {}{}", name(d), if pressure { ", gave up pressure" } else { "" }));
                        }
                    } else if d == id {
                        if won {
                            technique = -1;
                            g -= 0.3;
                            notes.push(format!("Blocked by {}", name(b)));
                        } else {
                            technique = technique.max(1);
                            g += 0.4;
                            notes.push(format!("Beat {}", name(b)));
                        }
                    }
                }
                for &(who, mv, t) in &r.rush_wins {
                    if who == id {
                        notes.push(format!("Won with a {} in {:.1} s", mv.name(), t));
                        technique = 1;
                    }
                }
                if r.sackers.contains(&id) {
                    notes.push("Sack".into());
                }
                if r.pressures.contains(&id) && !r.sackers.contains(&id) {
                    notes.push("Pressure".into());
                }
                if r.tacklers.contains(&id) {
                    notes.push("Made the tackle".into());
                }
                for &(t, _) in &r.missed_tackles {
                    if t == id {
                        notes.push("Missed tackle".into());
                        technique = -1;
                        g -= 0.6;
                    }
                }
                if r.drop && r.target == Some(id) {
                    notes.push("Dropped it".into());
                    technique = -1;
                }
                if r.pbu == Some(id) {
                    notes.push("Broke it up".into());
                }
                if r.interceptor == Some(id) {
                    notes.push("Interception".into());
                }
                if r.forced_fumble == Some(id) {
                    notes.push("Forced a fumble".into());
                    g += 1.0;
                }
                if r.fumbled == Some(id) {
                    notes.push("Fumbled".into());
                    g -= 1.5;
                }
                for &(kind, _, who, _, accepted) in &r.penalties {
                    if who == id && accepted {
                        notes.push(format!("Penalty: {}", kind.name()));
                    }
                }
                if loafs.contains(&id) {
                    notes.push("Loaf: didn't run to the ball".into());
                    effort = -1;
                    g -= 0.8;
                }
                if notes.is_empty() {
                    notes.push("Did his job".into());
                }
                let mark = if g > 0.25 {
                    "+"
                } else if g < -0.25 {
                    "-"
                } else {
                    "0"
                };
                out.insert(
                    id.to_string(),
                    json!({ "g": r2(g.clamp(-2.0, 2.0)), "mark": mark, "a": assignment, "t": technique, "e": effort, "notes": notes }),
                );
            }
        }
        Value::Object(out)
    }
}

fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// A short name for the score bug: initials of a long name, or the first
/// word.
fn short(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    if name.len() <= 10 {
        return name.to_uppercase();
    }
    if words.len() >= 3 {
        return words.iter().filter(|w| w.chars().next().is_some_and(|c| c.is_uppercase())).map(|w| &w[..1]).collect::<String>();
    }
    words.first().map_or(String::new(), |w| w.to_uppercase())
}

/// The plays that decided the game: scores, turnovers, fourth downs, big
/// swings in the win probability and explosive plays.
fn key_play(r: &PlayRecord, swing: f64) -> bool {
    r.touchdown
        || r.turnover
        || r.safety
        || (r.kind == PlayType::FieldGoal)
        || (r.down == 4 && matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble))
        || swing.abs() >= 0.06
        || r.yards >= 20.0
        || r.epa.abs() >= 2.5
}

/// The film's tags for cut-ups (section 9 topic 1: what your staff tags on
/// every snap): down, distance, field zone, the clock, personnel,
/// formation, the play, the coverage and the pressure, and the result.
fn tags(r: &PlayRecord) -> Value {
    let dist = if r.down == 0 {
        ""
    } else if r.togo <= 3.0 {
        "short"
    } else if r.togo <= 6.0 {
        "medium"
    } else {
        "long"
    };
    let yl = r.yardline_100;
    let zone = if matches!(r.kind, PlayType::Kickoff | PlayType::ExtraPoint | PlayType::TwoPoint) {
        ""
    } else if yl >= 90.0 {
        "coming out"
    } else if yl > 20.0 {
        "open field"
    } else if yl > 10.0 {
        "high red zone"
    } else if yl > 3.0 {
        "low red zone"
    } else {
        "goal line"
    };
    let two_min = (r.quarter == 2 || r.quarter == 4) && r.clock <= 120.0;
    // "11 gun doubles motion across, stick": personnel, alignment, set,
    // then the motion and the play.
    let call = r.call.as_str();
    let words: Vec<&str> = call.split_whitespace().collect();
    let align = if call.contains("under center") {
        "under center"
    } else if call.contains("pistol") {
        "pistol"
    } else if call.contains("gun") {
        "shotgun"
    } else {
        ""
    };
    let sets = ["doubles", "trips", "bunch", "empty", "stack", "wing", "twins", "ace", "i-form", "wishbone", "spread"];
    let set = words.iter().find(|w| sets.contains(w)).copied().unwrap_or("");
    let play = match call.split_once(", ") {
        Some((_, p)) => p.to_string(),
        None => {
            let idx = words.iter().position(|w| sets.contains(w)).map_or(words.len().min(3), |i| i + 1);
            words[idx.min(words.len())..].join(" ")
        }
    };
    let mut parts = r.defense_call.split(", ");
    let cover = parts.next().unwrap_or("").to_string();
    let rush = parts.next().unwrap_or("").to_string();
    let result = if r.touchdown {
        "touchdown"
    } else if r.turnover {
        "turnover"
    } else if r.kind == PlayType::Sack {
        "sack"
    } else if r.kind == PlayType::NoPlay || r.nullified {
        "penalty"
    } else if r.yards >= 20.0 || (r.kind == PlayType::Run && r.yards >= 10.0) {
        "explosive"
    } else if r.first_down {
        "first down"
    } else if r.kind == PlayType::Pass && !r.complete {
        "incomplete"
    } else {
        ""
    };
    let unit = match r.kind {
        PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble | PlayType::TwoPoint | PlayType::Kneel | PlayType::Spike | PlayType::NoPlay => "scrimmage",
        _ => "special teams",
    };
    json!({
        "down": r.down,
        "dist": dist,
        "zone": zone,
        "two_min": two_min,
        "personnel": r.personnel,
        "align": align,
        "formation": if set.is_empty() { align.to_string() } else { format!("{align} {set}").trim().to_string() },
        "play": play,
        "run_pass": match r.kind { PlayType::Run => "run", PlayType::Pass | PlayType::Sack | PlayType::Scramble => "pass", _ => "" },
        "cover": cover,
        "rush": rush,
        "result": result,
        "unit": unit,
        "pa": r.play_action,
        "blitz": r.blitz,
    })
}

/// A clip in the engine's frame of its team (side 0 of the sim attacks
/// toward x = 100): every frame with each man's spot, facing, act and
/// partner.
pub struct Clip {
    pub ids: Vec<u32>,
    /// The team (0 home, 1 away) whose frame this is.
    pub team: usize,
    pub frames: Vec<CFrame>,
    /// Drawn from the log, not tracked.
    pub drawn: bool,
}

pub struct CFrame {
    pub t: f64,
    pub ball: (f64, f64, f64),
    pub mode: u8,
    pub holder: u8,
    /// x, y, facing (radians), act, partner.
    pub p: Vec<(f64, f64, f64, u8, u8)>,
}

impl CFrame {
    fn from_engine(f: &Frame, t: f64) -> CFrame {
        let poses: &[Pose] = &f.poses;
        CFrame {
            t,
            ball: (f.ball.x, f.ball.y, f.ball_z as f64),
            mode: f.ball_mode as u8,
            holder: f.holder,
            p: f
                .players
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let pose = poses.get(i).copied().unwrap_or_default();
                    (p.1 as f64, p.2 as f64, pose.face as f64, pose.act as u8, pose.with)
                })
                .collect(),
        }
    }
}

impl Clip {
    fn new(ids: Vec<u32>, team: usize) -> Clip {
        Clip { ids, team, frames: Vec::new(), drawn: false }
    }

    /// Holds the last frame a moment, so the end of a play can be seen.
    fn hold_end(&mut self, secs: f64) {
        let Some(last) = self.frames.last() else { return };
        let t0 = last.t;
        let p = last.p.clone();
        let (ball, mode, holder) = (last.ball, last.mode, last.holder);
        let steps = (secs / 0.1).round() as usize;
        for k in 1..=steps {
            let mut q = p.clone();
            // Men on the ground start to get up near the end.
            for m in &mut q {
                if m.3 == Act::Down as u8 && k > steps / 2 {
                    m.3 = Act::GetUp as u8;
                }
            }
            self.frames.push(CFrame { t: t0 + k as f64 * 0.1, ball, mode, holder, p: q });
        }
    }

    /// When the flag comes out: about when the fouler was last engaged, or
    /// at the end of the action.
    fn foul_time(&self, id: u32) -> f64 {
        let Some(i) = self.ids.iter().position(|&x| x == id) else { return self.frames.last().map_or(0.0, |f| f.t) - 0.8 };
        let engaged = self
            .frames
            .iter()
            .filter(|f| f.t > 0.0 && matches!(Act::from_u8(f.p[i].3), Act::Block | Act::Engaged | Act::Tackle))
            .map(|f| f.t)
            .fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.max(t))));
        r2(engaged.unwrap_or_else(|| {
            let live: Vec<f64> = self.frames.iter().filter(|f| f.t > 0.0).map(|f| f.t).collect();
            live.get(live.len() / 2).copied().unwrap_or(0.5)
        }))
    }

    /// Your defenders who stopped running to the ball: up and free, well
    /// away from the carrier, hardly moving for a second and a half while
    /// the carrier runs on.
    fn loafs(&self, film: &Film, me: usize) -> Vec<u32> {
        if self.drawn {
            return Vec::new();
        }
        let n = self.ids.len();
        let mut slow = vec![0usize; n];
        let mut out = Vec::new();
        for w in self.frames.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            if a.t <= 0.0 || b.mode != BallMode::Held as u8 || b.holder == NONE {
                slow.iter_mut().for_each(|s| *s = 0);
                continue;
            }
            let c = b.holder as usize;
            if film.side_of(self.ids[c]) == Some(me) || Act::from_u8(b.p[c].3) == Act::Pocket {
                slow.iter_mut().for_each(|s| *s = 0);
                continue;
            }
            let cspeed = (b.p[c].0 - a.p[c].0).hypot(b.p[c].1 - a.p[c].1) / 0.1;
            for i in 0..n {
                if film.side_of(self.ids[i]) != Some(me) {
                    continue;
                }
                let act = Act::from_u8(b.p[i].3);
                let free = matches!(act, Act::Run | Act::Stand | Act::Cover | Act::Rush);
                let d = (b.p[i].0 - b.p[c].0).hypot(b.p[i].1 - b.p[c].1);
                let v = (b.p[i].0 - a.p[i].0).hypot(b.p[i].1 - a.p[i].1) / 0.1;
                if free && (6.0..30.0).contains(&d) && v < 1.2 && cspeed > 4.0 {
                    slow[i] += 1;
                    if slow[i] == 15 && !out.contains(&self.ids[i]) {
                        out.push(self.ids[i]);
                    }
                } else {
                    slow[i] = 0;
                }
            }
        }
        out
    }
}
