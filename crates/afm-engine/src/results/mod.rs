//! Growth from game results (Cain 2026-11-07, thread "Explain playing time
//! and player growth"; the brief is `afm/game-reps/proposal.md` in the
//! project folder). A player's skills move after each game toward the
//! level his own plays showed, skill by skill, judged against what his own
//! ratings should produce in this engine.
//!
//! - Each play gives evidence on the skills it tests (`trials`): a short
//!   throw on short accuracy, a carry on vision, a pass-block rep on pass
//!   blocking. The evidence is his result on that play against what the
//!   play's context gives a level's typical player (expected completion,
//!   expected rush yards), signed so that good is positive.
//! - What his ratings should produce is fitted on the engine's own plays
//!   at each level and position (`table::FIT`, from `results_fit`, with
//!   every man's tested skills moved at random game by game so each
//!   skill's own slope shows): the result an average man gets, how much
//!   more a rating point of the skill gives, how much more a point of his
//!   overall gives (the rest of his game) and how much a stronger opponent
//!   takes away. His shown level for a skill is the rating whose expected
//!   result matches his: his rating plus his result over expected, divided
//!   by the skill's slope (`evidence`). A read whose result doesn't move
//!   with its rating in this engine is off there.
//! - The season so far decides the move (`apply`, `target`): the mean
//!   shown level over his trials past his own normal (`Ledger::own`, what
//!   he has shown over his ratings in past seasons, so a make-up that
//!   always beats its ratings doesn't lift him every year), times the
//!   share of it his skill explains against what teammates, scheme and
//!   luck add in this engine (`Fit::weight`, credibility from the engine's
//!   own spreads: small samples and noisy stats count less), times the
//!   share of a starter's season he has played (he grows snap by snap). A
//!   star who plays exactly to his ratings holds, whatever his past
//!   seasons showed: his normal only takes in a season's move, never turns
//!   a season at his ratings into a pull-down (minor bug 351). Results above his
//!   ceiling lift him past it and the ceiling moves up with him.
//! - Snaps against stronger teams count more: 10% per standard deviation
//!   of opponent strength, at most 20% either way (item 5).
//! - Snaps grow him in the week he plays them (`reps`): the yearly step
//!   opens his year at a backup's rate and each game lands the part of the
//!   year his snaps add, today's size (a full starter's season about x1.15
//!   on the rate against x0.8), instead of reading last season's snaps a
//!   year late.
//! - The body (speed, strength and the like) doesn't grow from results.
//!
//! The results only read the play log; they draw nothing, so the games
//! play exactly as before. Players whose games run on the stand-in have no
//! plays and keep today's yearly step.

pub mod table;

use std::collections::HashMap;

use crate::attributes::{ATTRIBUTE_COUNT, Attr, MAX_RATING, MIN_RATING};
use crate::game::sim::Move;
use crate::game::stats::{GameStats, PlayRecord, PlayType, expected_completion_at};
use crate::game::{GamePlayer, GameTeam};
use crate::player::{Level, Player};
use crate::positions::Position;
use crate::rng::clamp;

/// How many skill reads there are.
pub const READS: usize = 30;

/// One skill read: which result on which plays tells about which rating.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Read {
    /// Short accuracy: completion over expected on throws under 10 air yards.
    ShortThrow,
    /// Medium accuracy: 10-19 air yards.
    MediumThrow,
    /// Deep accuracy: 20 or more.
    DeepThrow,
    /// Throw on the move: completion over expected on throws on the move.
    MovingThrow,
    /// Pocket presence: sacks taken per pressure, against expected.
    Pocket,
    /// Decisions: interceptions per attempt, against expected for the depth.
    Picks,
    /// Composure: completion over expected on throws under pressure, where
    /// the engine's poise works.
    Poise,
    /// Ball carrier vision: rush yards over expected on designed runs.
    Vision,
    /// Elusiveness: missed tackles forced per carry.
    Broken,
    /// Leg drive: yards after contact per carry.
    LegDrive,
    /// Ball security: fumbles per touch.
    Security,
    /// Route running: separation per target, for the depth.
    Separation,
    /// Release: separation per target against man coverage, where press
    /// happens (the engine charts no jam result; nearest it charts).
    Release,
    /// Catching: catch rate over expected.
    Hands,
    /// Contested catch: catch rate over expected with under a yard of room.
    Contested,
    /// Ball tracking: catch rate over expected on targets 20+ yards downfield.
    Tracking,
    /// Elusiveness after the catch: yards after the catch over expected.
    AfterCatch,
    /// Pass blocking: pressures allowed per pass-block rep.
    PassBlock,
    /// Run blocking: run-block reps won.
    RunBlock,
    /// Anchor: pass-block reps lost to a power rush.
    Anchor,
    /// Power rush: pass-rush reps won with power.
    PowerRush,
    /// Finesse rush: pass-rush reps won with speed or a finesse move.
    FinesseRush,
    /// Block shedding: run-block reps won as the defender.
    Shed,
    /// Man coverage: completions allowed in man, against what an open man
    /// catches at the throw's depth and pressure.
    ManCover,
    /// Zone coverage: the same in zone.
    ZoneCover,
    /// Tackling: missed tackles per tackle attempt.
    Tackle,
    /// Play recognition: run stops per run snap on the field.
    Recognition,
    /// Short kick accuracy: field goals under 40 yards and extra points made,
    /// for the distance.
    ShortKick,
    /// Long kick accuracy: field goals from 40 yards out, for the distance.
    LongKick,
    /// Punt placement: punts downed or fair caught inside the 20, for where
    /// they were punted from.
    Placement,
}

impl Read {
    pub const ALL: [Read; READS] = [
        Read::ShortThrow,
        Read::MediumThrow,
        Read::DeepThrow,
        Read::MovingThrow,
        Read::Pocket,
        Read::Picks,
        Read::Poise,
        Read::Vision,
        Read::Broken,
        Read::LegDrive,
        Read::Security,
        Read::Separation,
        Read::Release,
        Read::Hands,
        Read::Contested,
        Read::Tracking,
        Read::AfterCatch,
        Read::PassBlock,
        Read::RunBlock,
        Read::Anchor,
        Read::PowerRush,
        Read::FinesseRush,
        Read::Shed,
        Read::ManCover,
        Read::ZoneCover,
        Read::Tackle,
        Read::Recognition,
        Read::ShortKick,
        Read::LongKick,
        Read::Placement,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Read {
        Read::ALL[i.min(READS - 1)]
    }

    /// The rating the read moves.
    pub fn attr(self) -> Attr {
        use Read::*;
        match self {
            ShortThrow => Attr::ShortAccuracy,
            MediumThrow => Attr::MediumAccuracy,
            DeepThrow => Attr::DeepAccuracy,
            MovingThrow => Attr::ThrowOnMove,
            Pocket => Attr::PocketPresence,
            Picks => Attr::Decisions,
            Poise => Attr::Composure,
            Vision => Attr::BallCarrierVision,
            Broken | AfterCatch => Attr::Elusiveness,
            LegDrive => Attr::LegDrive,
            Security => Attr::BallSecurity,
            Separation => Attr::RouteRunning,
            Release => Attr::Release,
            Hands => Attr::Catching,
            Contested => Attr::ContestedCatch,
            Tracking => Attr::BallTracking,
            PassBlock => Attr::PassBlocking,
            RunBlock => Attr::RunBlocking,
            Anchor => Attr::Anchor,
            PowerRush => Attr::PowerRush,
            FinesseRush => Attr::FinesseRush,
            Shed => Attr::BlockShedding,
            ManCover => Attr::ManCoverage,
            ZoneCover => Attr::ZoneCoverage,
            Tackle => Attr::Tackling,
            Recognition => Attr::PlayRecognition,
            ShortKick => Attr::ShortKickAccuracy,
            LongKick => Attr::LongKickAccuracy,
            Placement => Attr::PuntPlacement,
        }
    }

    /// The positions whose growth the read moves (the brief's table).
    pub fn reads(self, pos: Position) -> bool {
        use Position::*;
        use Read::*;
        match self {
            ShortThrow | MediumThrow | DeepThrow | MovingThrow | Pocket | Picks | Poise => pos == QB,
            Vision | Broken | LegDrive => pos == RB,
            Security | Hands | AfterCatch => matches!(pos, RB | WR | TE),
            Separation | Release | Contested | Tracking => matches!(pos, WR | TE),
            PassBlock | RunBlock => matches!(pos, OT | IOL | TE),
            Anchor => matches!(pos, OT | IOL),
            PowerRush | FinesseRush => matches!(pos, EDGE | DT),
            Shed => matches!(pos, EDGE | DT | LB),
            ManCover | ZoneCover | Recognition => matches!(pos, LB | S | CB),
            Tackle => matches!(pos, EDGE | DT | LB | S | CB),
            ShortKick | LongKick => pos == K,
            Placement => pos == P,
        }
    }

    /// Plain words for pages and the news.
    pub fn name(self) -> &'static str {
        use Read::*;
        match self {
            ShortThrow => "short throws",
            MediumThrow => "intermediate throws",
            DeepThrow => "deep throws",
            MovingThrow => "throws on the move",
            Pocket => "pocket presence",
            Picks => "decisions",
            Poise => "throws under pressure",
            Vision => "vision",
            Broken => "broken tackles",
            LegDrive => "yards after contact",
            Security => "ball security",
            Separation => "route running",
            Release => "release",
            Hands => "hands",
            Contested => "contested catches",
            Tracking => "deep ball tracking",
            AfterCatch => "yards after the catch",
            PassBlock => "pass blocking",
            RunBlock => "run blocking",
            Anchor => "anchor",
            PowerRush => "power rush",
            FinesseRush => "finesse rush",
            Shed => "block shedding",
            ManCover => "man coverage",
            ZoneCover => "zone coverage",
            Tackle => "tackling",
            Recognition => "run stops",
            ShortKick => "short field goals",
            LongKick => "long field goals",
            Placement => "punt placement",
        }
    }
}

/// One play's evidence for one man: his result (good is positive) and the
/// play's context the model reads (the kick's distance, the punt's spot,
/// the throw's depth; 0 where the result already has its context out).
#[derive(Clone, Copy, Debug)]
pub struct Trial {
    pub id: u32,
    pub read: Read,
    pub y: f32,
    pub cx: f32,
}

/// The evidence one play gives, for everyone on it. Only reads the record.
pub fn trials(r: &PlayRecord, level: Level, out: &mut Vec<Trial>) {
    if r.nullified {
        return;
    }
    let mut put = |id: u32, read: Read, y: f64, cx: f64| out.push(Trial { id, read, y: y as f32, cx: cx as f32 });
    match r.kind {
        PlayType::Pass | PlayType::Sack | PlayType::Scramble => {
            // Pocket presence: a pressured dropback he got out of without a sack.
            if r.pressured
                && let Some(qb) = r.passer
            {
                put(qb, Read::Pocket, (r.kind != PlayType::Sack) as u8 as f64, 0.0);
            }
            if r.kind == PlayType::Pass {
                pass_trials(r, level, &mut put);
            }
            // The rush and the protection.
            for &(b, def, pass, won, secs, _) in &r.blocks {
                if !pass {
                    continue;
                }
                let gave = !won && (r.pressures.contains(&def) || r.sackers.contains(&def));
                put(b, Read::PassBlock, -(gave as u8 as f64), 0.0);
                let power_loss = !won && r.rush_wins.iter().any(|w| w.0 == def && w.1 == Move::Power);
                put(b, Read::Anchor, -(power_loss as u8 as f64), 0.0);
                let _ = secs;
            }
            let mut rushers: Vec<u32> = r.blocks.iter().filter(|b| b.2).map(|b| b.1).collect();
            rushers.sort_unstable();
            rushers.dedup();
            for d in rushers {
                let power = r.rush_wins.iter().any(|w| w.0 == d && w.1 == Move::Power);
                let finesse = r.rush_wins.iter().any(|w| w.0 == d && w.1 != Move::Power);
                put(d, Read::PowerRush, power as u8 as f64, 0.0);
                put(d, Read::FinesseRush, finesse as u8 as f64, 0.0);
            }
            if r.kind == PlayType::Scramble
                && let Some(c) = r.carrier
            {
                touch(r, c, &mut put);
            }
        }
        PlayType::Run => {
            if let Some(c) = r.carrier {
                let gap = crate::world::tally::expect::run_gap(&r.call);
                let x = crate::world::tally::expect::expected_rush(
                    level,
                    r.box_count,
                    matches!(gap, 0 | 4),
                    r.yardline_100,
                    r.togo,
                );
                // A long run's yards are capped, as the heavy tail would
                // drown every other carry.
                put(c, Read::Vision, r.yards.clamp(-10.0, 25.0) - x, 0.0);
                let after = (r.yards - r.before_contact).clamp(0.0, 20.0);
                put(c, Read::LegDrive, after, 0.0);
                put(c, Read::Broken, r.missed_tackles.iter().filter(|m| m.1 == c).count() as f64, 0.0);
                touch(r, c, &mut put);
            }
            for &(b, def, pass, won, _, _) in &r.blocks {
                if pass {
                    continue;
                }
                put(b, Read::RunBlock, won as u8 as f64, 0.0);
                put(def, Read::Shed, (!won) as u8 as f64, 0.0);
            }
            // Run stops: every defender on the field gets a trial.
            let stop = !r.success();
            for &id in &r.on_field[1] {
                let made = stop && r.tacklers.contains(&id);
                put(id, Read::Recognition, made as u8 as f64, 0.0);
            }
        }
        PlayType::FieldGoal | PlayType::ExtraPoint => {
            if let Some(k) = r.kicker
                && !r.blocked
            {
                let d = r.kick_distance;
                let read = if r.kind == PlayType::FieldGoal && d >= 40.0 { Read::LongKick } else { Read::ShortKick };
                put(k, read, r.kick_good as u8 as f64, d);
            }
        }
        PlayType::Punt => {
            if let Some(k) = r.kicker
                && !r.blocked
            {
                put(k, Read::Placement, r.inside20 as u8 as f64, r.yardline_100);
            }
        }
        _ => {}
    }
    // Tackling, on every play with a ball carrier (kicks included: a
    // gunner's miss is a miss).
    if matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Scramble | PlayType::Punt | PlayType::Kickoff) {
        for &id in &r.tacklers {
            put(id, Read::Tackle, 0.0, 0.0);
        }
        for &(tk, _) in &r.missed_tackles {
            put(tk, Read::Tackle, -1.0, 0.0);
        }
    }
}

/// A touch's ball security.
fn touch(r: &PlayRecord, c: u32, put: &mut impl FnMut(u32, Read, f64, f64)) {
    put(c, Read::Security, -((r.fumbled == Some(c)) as u8 as f64), 0.0);
}

fn pass_trials(r: &PlayRecord, level: Level, put: &mut impl FnMut(u32, Read, f64, f64)) {
    let air = r.air_yards;
    if !air.is_finite() {
        return;
    }
    // Interceptions count on every attempt, a throw-away included.
    if let Some(qb) = r.passer {
        put(qb, Read::Picks, -(r.interception as u8 as f64), air.clamp(-5.0, 50.0));
    }
    if r.throw_away {
        return;
    }
    let xc = expected_completion_at(level, air, r.separation, r.pressured);
    let over = r.complete as u8 as f64 - xc;
    if let Some(qb) = r.passer {
        let read = if air < 10.0 {
            Read::ShortThrow
        } else if air < 20.0 {
            Read::MediumThrow
        } else {
            Read::DeepThrow
        };
        put(qb, read, over, 0.0);
        if r.moving {
            put(qb, Read::MovingThrow, over, 0.0);
        }
        if r.pressured {
            put(qb, Read::Poise, over, 0.0);
        }
    }
    if let Some(t) = r.target {
        put(t, Read::Hands, over, 0.0);
        put(t, Read::Separation, r.separation.clamp(0.0, 8.0), air.clamp(-5.0, 50.0));
        if r.coverage_man {
            put(t, Read::Release, r.separation.clamp(0.0, 8.0), air.clamp(-5.0, 50.0));
        }
        if r.separation < 1.0 {
            put(t, Read::Contested, over, 0.0);
        }
        if air >= 20.0 {
            put(t, Read::Tracking, over, 0.0);
        }
        if r.complete {
            let middle = r.throw_y.is_some_and(|(_, at)| crate::world::tally::expect::third(at as f64) == 1);
            let x = crate::world::tally::expect::expected_yac(level, air, r.separation, r.coverage_man, middle);
            put(t, Read::AfterCatch, r.yac.clamp(-5.0, 40.0) - x, 0.0);
        }
    }
    if let Some(d) = r.defender {
        // The separation is his doing, so his read is the catch against
        // what the throw's depth and pressure give an open man (minor bug
        // 283: against the receiver's separation it left out most of what
        // coverage does).
        let open = r.complete as u8 as f64 - expected_completion_at(level, air, 2.0, r.pressured);
        put(d, if r.coverage_man { Read::ManCover } else { Read::ZoneCover }, -open, 0.0);
    }
}

/// What follows a man into a fit besides his ratings: his body (height,
/// weight, arms) and his hidden game traits (consistency, temperament,
/// recovery), in that order (`own`).
pub const OWN: usize = 6;

/// His body and hidden game traits as a fit reads them.
pub fn own(p: &GamePlayer) -> [f64; OWN] {
    [p.height, p.weight, p.arm, p.traits.consistency, p.traits.temperament, p.traits.recovery]
}

/// What a fitted read holds, per level and position (`table::FIT`): the
/// expected result `a + w . (his ratings, his body and hidden traits) +
/// d z + e cx` for a man as he is, against an opponent `z` standard
/// deviations strong, with the play's context `cx` (mean `c0`); `b`, the
/// weight on the read's own skill; the spread of a single trial; the
/// trials in a full starter's season; what the model leaves in a man's
/// results (`tau`; `tp` the part that follows him to another team, the
/// rest his teammates and scheme); and the skill's spread among the
/// level's regulars at the position. `b` 0: the read is off there (the
/// result doesn't move with the rating).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub a: f64,
    pub w: [f32; ATTRIBUTE_COUNT + OWN],
    pub b: f64,
    pub d: f64,
    pub e: f64,
    pub c0: f64,
    pub sd: f64,
    pub season: f64,
    pub tau: f64,
    pub tp: f64,
    pub sk: f64,
}

impl Fit {
    pub const OFF: Fit = Fit {
        a: 0.0,
        w: [0.0; ATTRIBUTE_COUNT + OWN],
        b: 0.0,
        d: 0.0,
        e: 0.0,
        c0: 0.0,
        sd: 0.0,
        season: 0.0,
        tau: 0.0,
        tp: 0.0,
        sk: 0.0,
    };

    pub fn on(&self) -> bool {
        self.b > 0.0 && self.season > 0.0 && self.sk > 0.0
    }

    pub fn set_w(&mut self, k: usize, v: f64) {
        self.w[k] = v as f32;
    }

    /// How much of a season's shown level over his rating, on `n` trials,
    /// the season keeps: the share of what the results show that a real
    /// change in his skill this season explains, against what the rest of
    /// the engine adds (credibility, `c^2 / (c^2 + (tau^2 + sd^2 / n) /
    /// b^2)`, `c` the most a season really changes a skill, `CHANGE`, or
    /// the skill's spread among regulars if that is smaller; `Mode.change`
    /// sets it). Small samples
    /// count less (item 9) and a stat the engine makes noisy counts less.
    /// A season's move then spreads at most `c / 2` however the read is
    /// fitted, so seasons don't add up to a random walk that widens the
    /// level's spread.
    pub fn weight(&self, n: f64, change: f64) -> f64 {
        if !self.on() || n <= 0.0 {
            return 0.0;
        }
        let c = self.sk.min(change);
        let k = c * c * self.b * self.b;
        k / (k + self.tau * self.tau + self.sd * self.sd / n)
    }

    /// The result a man as he is should get.
    pub fn expected(&self, attrs: &crate::attributes::Attributes, own: &[f64; OWN], z: f64, cx: f64) -> f64 {
        let mut e = self.a + self.d * z + self.e * cx;
        for k in 0..ATTRIBUTE_COUNT {
            e += self.w[k] as f64 * attrs.at(k);
        }
        for (k, x) in own.iter().enumerate() {
            e += self.w[ATTRIBUTE_COUNT + k] as f64 * x;
        }
        e
    }
}

/// The table row of a level (as `XCOMP`'s: HS, FBS, FCS, small colleges, NFL).
pub fn row(level: Level) -> usize {
    level as usize
}

pub fn fit(level: Level, read: Read, pos: Position) -> &'static Fit {
    &table::FIT[table::index(row(level), read.index(), pos.index())]
}

/// A team's strength for the opponent weight: the mean overall of its 22
/// best men at their listed positions, kickers left out.
pub fn strength(t: &GameTeam) -> f64 {
    let mut o: Vec<f64> =
        t.players.iter().filter(|p| !p.position.is_specialist()).map(|p| p.overall_at(p.position)).collect();
    o.sort_unstable_by(|a, b| b.total_cmp(a));
    let n = o.len().min(22);
    if n == 0 {
        return 0.0;
    }
    o[..n].iter().sum::<f64>() / n as f64
}

/// A team's strength in standard deviations of its level's teams.
pub fn strength_z(level: Level, t: &GameTeam) -> f64 {
    let (m, s) = table::TEAM[row(level)];
    if s <= 0.0 { 0.0 } else { (strength(t) - m) / s }
}

/// How much a snap counts against an opponent `z` standard deviations
/// strong (item 5: the NFL's 10% per standard deviation, now at every level).
pub fn opp_weight(z: f64) -> f64 {
    clamp(1.0 + 0.1 * z, 0.8, 1.2)
}

/// One man's evidence from one game: per read, the trials (weighted by
/// the opponent) and their summed shown level over his rating, in rating
/// points; his offensive and defensive snaps; the opponent's strength.
#[derive(Clone, Debug, Default)]
pub struct Man {
    pub reads: Vec<(Read, f32, f32)>,
    pub snaps: u32,
    pub opp: f32,
    /// Practice evidence (item 16): it moves skills but isn't a game (no
    /// snaps land, no game is counted or named).
    pub practice: bool,
}

/// Reads a game: every man's evidence. `teams` are the sides as they
/// played (home, away), their ratings as they stood in the game.
pub fn evidence(level: Level, stats: &GameStats, teams: [&GameTeam; 2]) -> HashMap<u32, Man> {
    let z = [strength_z(level, teams[0]), strength_z(level, teams[1])];
    let mut who: HashMap<u32, (usize, &GamePlayer)> = HashMap::new();
    for (side, t) in teams.iter().enumerate() {
        for p in &t.players {
            who.insert(p.id, (side, p));
        }
    }
    let mut out: HashMap<u32, Man> = HashMap::new();
    let mut buf = Vec::new();
    for r in &stats.plays {
        buf.clear();
        trials(r, level, &mut buf);
        for t in &buf {
            let Some(&(side, p)) = who.get(&t.id) else { continue };
            if !t.read.reads(p.position) {
                continue;
            }
            let f = fit(level, t.read, p.position);
            if !f.on() {
                continue;
            }
            let opp = z[1 - side];
            let e = excess(f, p, opp, t.y as f64, t.cx as f64);
            let w = opp_weight(opp);
            let m = out.entry(t.id).or_default();
            m.opp = opp as f32;
            match m.reads.iter_mut().find(|x| x.0 == t.read) {
                Some(x) => {
                    x.1 += w as f32;
                    x.2 += (w * e) as f32;
                }
                None => m.reads.push((t.read, w as f32, (w * e) as f32)),
            }
        }
    }
    for (side, lines) in stats.players.iter().enumerate() {
        for (&id, l) in lines {
            let m = out.entry(id).or_default();
            m.snaps = l.snaps_off + l.snaps_def;
            m.opp = z[1 - side] as f32;
        }
    }
    out
}

/// One trial's shown level over his rating, in rating points.
pub fn excess(f: &Fit, p: &GamePlayer, opp: f64, y: f64, cx: f64) -> f64 {
    (y - f.expected(&p.attrs, &own(p), opp, cx)) / f.b
}

/// How results grow players (`WorldOptions.results`). The arms of the A/B
/// test are its switches (items 14 and 15); `practice` is item 16.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mode {
    /// Results move skills after each game, and snaps land the week they
    /// are played.
    pub on: bool,
    /// Item 14: the hidden yearly dice (form, breakouts, slumps) on the
    /// skills results test, kept with results on top, or replaced by
    /// results (their average kept, so a player who barely plays grows
    /// today's average without the surprise).
    pub dice: Dice,
    /// Item 15: a season below his ratings pulls the skill down the way a
    /// good one lifts it.
    pub pull_down: bool,
    /// Item 16 (waiting on Cain): practice snaps on the engine count as
    /// results too, at a quarter of a game snap.
    pub practice: bool,
    /// The per-snap rate against the credibility weight (1: a full
    /// starter's season moves him by what the engine credits his skill
    /// with). The A/B tries a half rate.
    pub scale: f64,
    /// The most a season really changes a skill, the prior in
    /// `Fit::weight` (`CHANGE`; infinite reads the skill's whole spread
    /// among regulars, as the first build did).
    #[serde(default = "default_change")]
    pub change: f64,
}

fn default_change() -> f64 {
    CHANGE
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Dice {
    Kept,
    Replaced,
}

impl Default for Mode {
    fn default() -> Self {
        Mode { on: true, dice: Dice::Kept, pull_down: true, practice: false, scale: 1.0, change: CHANGE }
    }
}

impl Mode {
    pub const OFF: Mode =
        Mode { on: false, dice: Dice::Kept, pull_down: true, practice: false, scale: 1.0, change: CHANGE };
}

/// Most a season's results move one skill, either way (as the yearly
/// step's most a skill grows in a year).
pub const CAP: f64 = 12.0;
/// How far a season can really change a skill beyond the yearly step, in
/// rating points (one standard deviation): the prior in `Fit::weight`.
/// About the yearly step's own surprise on a skill (its noise, slumps and
/// breakouts) or a little more, so results add about a year's surprise. With the level's
/// whole skill spread here instead (9 points on the line), seasons moved a
/// skill up to 4 points each way at random and NFL line skills spread 25
/// to 57% wider in a seed 75 world (proposed, not confirmed).
pub const CHANGE: f64 = 3.0;
/// Within this many points of his ceiling, he is at his top: a results
/// move there moves the ceiling too (proposed, not confirmed).
pub const AT_TOP: f64 = 0.5;
/// A game whose results move one skill this many rating points up is
/// named on his season row and in the news (the "mini explosion"; about
/// one starter's season in seven has one in the A/B, the 86th percentile
/// of a starter's best game; proposed, not confirmed).
pub const BIG_GAME: f64 = 0.3;
/// A season whose results move one skill this many points either way
/// names the season: a breakout from results, or a slump (about one
/// starter's season in ten each way in the A/B, his season's top move at
/// the 90th and 10th percentiles; proposed, not confirmed).
pub const BIG_SEASON: f64 = 1.0;

/// One read's season so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub read: u8,
    /// Trials, weighted by the opponent.
    pub n: f32,
    /// Summed shown level over his rating, weighted.
    pub s: f32,
    /// Rating points already moved this season.
    pub applied: f32,
    /// The league's centre on the read at his last game (`Book::centre`):
    /// his season's games are read against it.
    #[serde(default)]
    pub c: f32,
    /// Of `n`, the practice trials (item 16), read against practice's own
    /// centre at his last practice (`cp`).
    #[serde(default)]
    pub np: f32,
    #[serde(default)]
    pub cp: f32,
}

impl Entry {
    /// The season's summed shown level over his rating, each trial read
    /// against its own league's latest centre (games and practice apart).
    pub fn centred(&self) -> f64 {
        self.s as f64 - (self.n - self.np) as f64 * self.c as f64 - self.np as f64 * self.cp as f64
    }
}

/// A read's past seasons for a player: how many, and the sum of each
/// season's shown level over his rating (rating points) and trials.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Past {
    pub read: u8,
    pub seasons: u16,
    pub s: f32,
    pub n: f32,
}

/// A player's season of results, kept from his checkpoint to the next,
/// and his past seasons' (`past`, kept while he plays).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Ledger {
    pub year: i32,
    pub reads: Vec<Entry>,
    #[serde(default)]
    pub past: Vec<Past>,
    /// The share of a full starter's season his snaps have landed.
    pub reps: f32,
    pub games: u16,
    /// Overall points the results have moved this season.
    pub total: f32,
    /// The biggest move of one skill in a single game (rating points), and its read.
    pub best: f32,
    pub best_read: u8,
}

impl Ledger {
    /// A new season: last season's reads go into his past, the rest starts over.
    pub fn roll(&mut self, year: i32) {
        let mut past = std::mem::take(&mut self.past);
        for e in self.reads.iter().filter(|e| e.n >= 1.0) {
            let i = match past.iter().position(|x| x.read == e.read) {
                Some(i) => i,
                None => {
                    past.push(Past { read: e.read, ..Past::default() });
                    past.len() - 1
                }
            };
            let x = &mut past[i];
            x.seasons = x.seasons.saturating_add(1);
            x.s += (e.centred() / e.n as f64) as f32;
            x.n += e.n;
        }
        *self = Ledger { year, past, ..Ledger::default() };
    }

    /// His own normal on a read: what he has shown over his rating season
    /// after season, the part of it that follows him whatever his team and
    /// luck (`Fit::tp` against what a season adds). A man whose make-up
    /// (his body, his mix of skills) always beats or trails his ratings
    /// is judged against that, so it doesn't lift or sink him every year;
    /// a first season at it has none. It only takes in a season's move
    /// (`target`): it never pulls down a season at his ratings.
    pub fn own(&self, level: Level, read: Read, pos: Position) -> f64 {
        let Some(x) = self.past.iter().find(|x| x.read == read as u8).filter(|x| x.seasons > 0) else {
            return 0.0;
        };
        let f = fit(level, read, pos);
        if !f.on() {
            return 0.0;
        }
        let k = x.seasons as f64;
        let (own, rest) = ((f.tp / f.b).powi(2), (f.tau * f.tau - f.tp * f.tp).max(0.0) / (f.b * f.b));
        let noise = (f.sd / f.b).powi(2) / (x.n as f64 / k).max(1.0);
        x.s as f64 / k * own / (own + (rest + noise) / k)
    }

    /// The season's shown level over his rating for a read, and its trials.
    pub fn shown(&self, read: Read) -> Option<(f64, f64)> {
        self.reads.iter().find(|e| e.read == read as u8).map(|e| (e.centred() / e.n.max(1e-6) as f64, e.n as f64))
    }
}

/// The move a season so far makes: the mean shown level over his rating
/// past his own normal (`own`), times what the season keeps of it
/// (`Fit::weight`), the share of a starter's season it has seen (so he
/// grows snap by snap and one big game moves him only its part), and the
/// rate scale.
///
/// He is judged against his ratings; his normal only takes in a season's
/// move, between none of it and all of it (minor bug 351). The engine
/// plays him at his ratings as results moved them, so a season at them
/// shows nothing over his rating: read against a normal that past luck
/// left above 0, every such season would pull him down, and a great
/// season would turn into a loss over the following years. A make-up that
/// always beats his ratings still lifts him less each season, and a
/// season below his ratings still pulls him down by what it showed.
pub fn target(level: Level, read: Read, pos: Position, n: f64, s: f64, own: f64, mode: &Mode) -> f64 {
    let f = fit(level, read, pos);
    if n <= 0.0 || f.season <= 0.0 {
        return 0.0;
    }
    let shown = s / n;
    let own = own.clamp(shown.min(0.0), shown.max(0.0));
    let t = mode.scale * (shown - own) * f.weight(n, mode.change) * (n / f.season).min(1.0);
    let t = if mode.pull_down { t } else { t.max(0.0) };
    clamp(t, -CAP, CAP)
}

/// What one game did to him.
#[derive(Clone, Copy, Debug, Default)]
pub struct Moved {
    /// His overall's change from the results.
    pub overall: f64,
    /// The read that moved most, and by how many rating points.
    pub read: Option<(Read, f64)>,
    /// The read that rose most, and by how many rating points (a big game
    /// is named on this, whatever else fell).
    pub up: Option<(Read, f64)>,
}

/// Moves his skills after a game. The season so far decides each skill's
/// move; what this game adds is the change from what was already moved.
/// Judged against the fit alone (a centre of 0); the world reads each
/// game against its league with `apply_in`.
pub fn apply(p: &mut Player, l: &mut Ledger, m: &Man, level: Level, mode: &Mode) -> Moved {
    apply_in(p, l, m, level, mode, &|_| 0.0)
}

/// `apply` with the league's centre on each read (`Book::centre`): the
/// whole season so far is read against the latest centre, so the early
/// games' centre, before the league had played much, doesn't stay in it.
pub fn apply_in(
    p: &mut Player,
    l: &mut Ledger,
    m: &Man,
    level: Level,
    mode: &Mode,
    centre: &dyn Fn(Read) -> f64,
) -> Moved {
    let before = p.overall();
    let mut top: Option<(Read, f64)> = None;
    let mut up: Option<(Read, f64)> = None;
    for &(read, n, s) in &m.reads {
        let k = match l.reads.iter().position(|e| e.read == read as u8) {
            Some(k) => k,
            None => {
                l.reads.push(Entry { read: read as u8, ..Entry::default() });
                l.reads.len() - 1
            }
        };
        let own = l.own(level, read, p.position);
        let e = &mut l.reads[k];
        e.n += n;
        e.s += s;
        if m.practice {
            e.np += n;
            e.cp = centre(read) as f32;
        } else {
            e.c = centre(read) as f32;
        }
        let t = target(level, read, p.position, e.n as f64, e.centred(), own, mode);
        let a = read.attr();
        let was = p.attributes.get(a);
        let now = clamp(was + t - e.applied as f64, MIN_RATING, MAX_RATING);
        p.attributes.set(a, now);
        // What really moved (a rating at the scale's end moves less), so a
        // later fall in the target takes back only what was given.
        let d = now - was;
        e.applied += d as f32;
        // At his top, a move takes his top with it, either way; below it,
        // it is growth early or late that the yearly step evens out. A
        // ceiling that only rose kept every lucky season at the top and
        // grew back every unlucky one, a ratchet.
        let top_now = p.ceilings.get(a);
        let c = if top_now - was <= AT_TOP { top_now + (now - was) } else { top_now };
        p.ceilings.set(a, clamp(c, now, MAX_RATING));
        if top.is_none_or(|(_, v)| d.abs() > v.abs()) {
            top = Some((read, d));
        }
        if d > 0.0 && up.is_none_or(|(_, v)| d > v) {
            up = Some((read, d));
        }
    }
    let moved = p.overall() - before;
    l.total += moved as f32;
    let rise = up.map_or(0.0, |t| t.1);
    if !m.practice {
        l.games += 1;
        if rise > l.best as f64 {
            l.best = rise as f32;
            l.best_read = up.map_or(0, |t| t.0 as u8);
        }
    }
    if let Some(h) = p.history.last_mut() {
        h.results = crate::player::OptF64::some(l.total as f64);
        if rise >= BIG_GAME && !m.practice {
            h.events.add(crate::player::DevelopmentEvent::BigGame);
        }
    }
    Moved { overall: if m.practice { 0.0 } else { moved }, read: top, up }
}

/// The skills results test for a position at a level: the dice item 14
/// replaces are these.
pub fn tested(pos: Position, level: Level) -> u128 {
    let mut m = 0u128;
    for r in Read::ALL {
        if r.reads(pos) && fit(level, r, pos).on() {
            m |= 1u128 << r.attr().index();
        }
    }
    m
}

/// Whether a skill is in a tested mask.
pub fn is_tested(mask: u128, a: Attr) -> bool {
    a.index() < ATTRIBUTE_COUNT && mask & (1u128 << a.index()) != 0
}

/// A full starter's season of snaps at a level (the depth chart lane's
/// shares: college 750, high school 550, the NFL 1,000).
pub fn season_snaps(level: Level) -> f64 {
    match level {
        Level::HS => crate::world::season::HS_STARTER_SNAPS as f64,
        Level::NFL => crate::world::pro::games::NFL_SEASON_SNAPS as f64,
        _ => crate::world::season::SEASON_SNAPS as f64,
    }
}

/// Lands the growth a game's snaps add (item 1): his share of a starter's
/// season, weighted by the opponent, moves the year's rate from a
/// backup's toward a starter's as the yearly step's snaps did. Only a year
/// opened with snaps landing in season takes it.
pub fn reps(p: &mut Player, l: &mut Ledger, snaps: u32, level: Level, opp: f64, focus: Option<Attr>) -> f64 {
    if snaps == 0 || p.position.is_specialist() {
        return 0.0;
    }
    let d = snaps as f64 / season_snaps(level) * opp_weight(opp);
    let gained = crate::training::reps_land(p, l.reps as f64, d, focus);
    l.reps += d as f32;
    gained
}

/// At his checkpoint, before the year closes: a year opened with snaps
/// landing in season whose games landed none (a team on the stand-in, a
/// season he never took an engine snap) is marked as not landed, so the
/// next opening reads the season as before (his role, or his snaps) and it
/// isn't lost (`development::develop_season`, `Train` flags bit 3).
pub fn settle_reps(p: &mut Player, l: Option<&Ledger>) {
    if l.is_none_or(|l| l.reps <= 0.0) {
        p.train.year.flags &= !8;
    }
}

/// Closes a season's ledger at his next checkpoint: the season's results
/// name the row (a breakout from results, or a slump) and the ledger goes.
pub fn close(p: &mut Player, l: &Ledger) {
    let total = l.total as f64;
    let skill = l.reads.iter().map(|e| e.applied as f64).fold(0.0, |m: f64, v| if v.abs() > m.abs() { v } else { m });
    // The row the season was played on is the last one until the next
    // checkpoint pushes its own.
    if let Some(h) = p.history.last_mut() {
        h.results = crate::player::OptF64::some(total);
        if skill >= BIG_SEASON {
            h.events.add(crate::player::DevelopmentEvent::BigSeason);
        } else if skill <= -BIG_SEASON {
            h.events.add(crate::player::DevelopmentEvent::RoughSeason);
        }
    }
}

/// The world's open seasons of results, one ledger per player who played
/// an engine game since his last checkpoint, and each level's season so
/// far per read and position (`centre`), saved with the world.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Book {
    pub ledgers: HashMap<u32, Ledger>,
    #[serde(default)]
    pub league: HashMap<u32, League>,
}

/// A level's season so far on one read at one position: every man's
/// trials and summed shown level over his rating.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct League {
    pub year: i32,
    pub n: f64,
    pub s: f64,
}

/// The league evidence the centre starts from, in starter's seasons of
/// trials at the read (proposed, not confirmed): until the league has
/// played that much, its centre leans toward the fit's.
pub const CENTRE_PRIOR: f64 = 2.0;
/// A man's peers for the league's centre: the men at his level and
/// position within the same this many rating points of the read's skill
/// (60-69, 70-79 and so on). Without it the fit's straight line read the
/// best pass rushers as playing under their ratings each season, so the
/// NFL's starters lost rush and sacks a dropback fell 7 and 11% on seeds
/// 77 and 78 (proposed, not confirmed).
pub const BAND: f64 = 10.0;
/// A man's peers by load: the men who played the same this many snaps in
/// the game (0-14, 15-29, 30-44, 45 and more). The engine tires a man as
/// he plays, so a defensive tackle who plays every down shows less than a
/// rested one of the same rating; read against all of them, the starters
/// lost rush season after season (proposed, not confirmed).
pub const LOAD: u32 = 15;

fn league_key(level: Level, read: Read, pos: Position, practice: bool, peer: (f64, u32)) -> u32 {
    let band = (peer.0 / BAND).floor().clamp(0.0, 15.0) as u32;
    let load = (peer.1 / LOAD).min(3);
    table::index(row(level), read.index(), pos.index()) as u32 | (practice as u32) << 16 | band << 17 | load << 21
}

impl Book {
    /// Ledgers older than last season go (a player who left football; a
    /// player still playing rolls his at each checkpoint); last season's
    /// league centres go.
    pub fn sweep(&mut self, year: i32) {
        self.ledgers.retain(|_, l| l.year >= year - 1);
        self.league.retain(|_, x| x.year >= year);
    }

    /// Adds a man's evidence on a read to his level's season, with his
    /// peers on the read's skill (`BAND`).
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &mut self,
        level: Level,
        read: Read,
        pos: Position,
        practice: bool,
        peer: (f64, u32),
        year: i32,
        n: f64,
        s: f64,
    ) {
        let x = self.league.entry(league_key(level, read, pos, practice, peer)).or_default();
        if x.year != year {
            *x = League { year, ..League::default() };
        }
        x.n += n;
        x.s += s;
    }

    /// The league's centre on a read at a position this season, among a
    /// man's peers (`peer`: his rating on the read's skill, in the same
    /// `BAND` of rating points, and his snaps in the game, in the same
    /// `LOAD` of snaps, since a man who plays every down plays more of
    /// them tired):
    /// their mean shown level over their ratings at his level so far,
    /// leaning on the fit's (0) until `CENTRE_PRIOR` seasons of trials are
    /// in. A man is judged against men rated like him as the league plays,
    /// as real stats over expected are each season: a shift his peers
    /// share (his matchups in this world against the fit's synthetic
    /// teams, a line that wins more than the fit's, a straight-line fit
    /// that misses at the top of the scale) moves nobody, so it can't feed
    /// on itself. Practice keeps its own centre.
    pub fn centre(&self, level: Level, read: Read, pos: Position, practice: bool, peer: (f64, u32), year: i32) -> f64 {
        let prior = CENTRE_PRIOR * fit(level, read, pos).season.max(1.0);
        match self.league.get(&league_key(level, read, pos, practice, peer)) {
            Some(x) if x.year == year => x.s / (x.n + prior),
            _ => 0.0,
        }
    }
}

/// A game in the world: every man's results move his skills and his
/// snaps land (`person` maps a player id to his person; men not in it, a
/// stand-in or a player the world doesn't keep, are skipped). A big game
/// is named on his row and, in college and the NFL, in the news.
pub(crate) fn world_game(
    w: &mut crate::world::World,
    level: Level,
    ev: HashMap<u32, Man>,
    person: &dyn Fn(u32) -> Option<(usize, Option<crate::world::stories::TeamKey>)>,
) {
    let mode = w.options.results;
    if !mode.on || ev.is_empty() {
        return;
    }
    let year = w.year;
    let mut big: Vec<(usize, crate::world::stories::TeamKey, Read, f64)> = Vec::new();
    // In id order, so the world plays the same on every run.
    let mut men: Vec<(u32, usize, Option<crate::world::stories::TeamKey>, Man)> = ev
        .into_iter()
        .filter_map(|(id, m)| {
            let (i, team) = person(id)?;
            (w.people.get(i)?.player.id == id).then_some((id, i, team, m))
        })
        // Practice counts only in a season his checkpoint has opened (its
        // ledger this year): spring practice before it would roll last
        // season's ledger early and be closed as if it were the season.
        .filter(|(id, _, _, m)| !m.practice || w.results.ledgers.get(id).is_some_and(|l| l.year == year))
        .collect();
    men.sort_by_key(|x| x.0);
    // The league's season first, this game in it; then each man's season
    // is read against it (`Book::centre`, `apply_in`).
    for (_, i, _, m) in &men {
        let p = &w.people[*i].player;
        for &(r, n, s) in &m.reads {
            let peer = (p.attributes.get(r.attr()), m.snaps);
            w.results.add(level, r, p.position, m.practice, peer, year, n as f64, s as f64);
        }
    }
    for (id, i, team, m) in men {
        let focus = w.focus_of(id);
        let q = &w.people[i].player;
        let cs: Vec<(Read, f64)> = m
            .reads
            .iter()
            .map(|x| {
                (
                    x.0,
                    w.results.centre(level, x.0, q.position, m.practice, (q.attributes.get(x.0.attr()), m.snaps), year),
                )
            })
            .collect();
        let centre = |r: Read| cs.iter().find(|x| x.0 == r).map_or(0.0, |x| x.1);
        let p = &mut w.people[i].player;
        let l = w.results.ledgers.entry(id).or_insert_with(|| Ledger { year, ..Ledger::default() });
        if l.year != year {
            l.roll(year);
        }
        let moved = apply_in(p, l, &m, level, &mode, &centre);
        reps(p, l, m.snaps, level, m.opp as f64, focus);
        if let (Some(t), Some((r, d))) = (team, moved.up)
            && d >= BIG_GAME
            && !m.practice
        {
            big.push((i, t, r, d));
        }
    }
    if !w.recording {
        return;
    }
    for (i, t, r, d) in big {
        use crate::calendar::{Level as CalLevel, Loudness};
        use crate::world::stories::{TeamKey, news, post, team_name};
        let cal = match t {
            TeamKey::Nfl(_) => CalLevel::Nfl,
            TeamKey::College(_) => CalLevel::College,
            _ => CalLevel::HighSchool,
        };
        let (name, id, pos) = (w.people[i].name.clone(), w.people[i].player.id, w.people[i].player.position);
        let club = team_name(w, t);
        let size = if d >= 2.0 * BIG_GAME { "a big step" } else { "a step" };
        let text =
            format!("{name} ({}) had the game of his season for {club}: his {} took {size}.", pos.name(), r.name());
        let e = news(w, cal, Loudness::Small, Some(t), text).person(id, &name);
        post(w, e);
    }
}
