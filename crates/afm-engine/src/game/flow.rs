//! The game (topics 4-9): downs, the clock, scoring, penalties, overtime,
//! substitutions, fatigue, injuries, momentum and play calling, snap by
//! snap. Every game is reproducible from its seed (topic 9.2).

use super::books::Rouge;
use super::calling::{self, FourthDown, Memory, Situation};
use super::experiment;
use super::garbage;
use super::geom::{FieldGuard, goal, mid, v2};
use super::kicking::{self, KickTeams};
use super::live::{
    self, Ask, CloseCall, Control, Input, Officiating, Pending, Replay, Reviewer, StopCtx, Tempo, TossChoice, View,
};
use super::playbook::*;
use super::roster::*;
use super::rules::{FirstDownStop, Overtime, Rules};
use super::scrimmage::{self, DSlot, Snap, SnapResult};
use super::sim::{Conditions, Crew, Foul, FoulKind, Frame, Injury, InjuryKind, Mood};
use super::stats::*;
use super::tables;
use crate::attributes::Attr;
use crate::player::Level;
use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::spots::{self, Spot, best_order, fill_in};

/// Seconds from the end of a play until the ball is spotted (and the chains
/// set) and the referee's ready signal starts a stopped clock again
/// (proposed, not confirmed).
const BALL_SPOTTED: f64 = 7.0;

/// The play clock after an administrative stoppage such as a penalty,
/// from the ready signal (40 seconds otherwise, from the end of the play).
const ADMIN_PLAY_CLOCK: f64 = 25.0;

/// Everything needed to play one game.
#[derive(Clone, Debug)]
pub struct GameSetup<'a> {
    pub home: &'a GameTeam,
    pub away: &'a GameTeam,
    pub rules: Rules,
    pub cond: Conditions,
    pub seed: u64,
    /// No home crowd.
    pub neutral: bool,
    pub playoff: bool,
    /// Keep the full tracking of every play (the user's games).
    pub record: bool,
    /// Rating points a player from the other side of the ball must beat a
    /// starter by to play both ways (topic 5.1), per team.
    pub two_way_margin: [f64; 2],
    /// What the season does to each head coach's read of a decided game.
    pub context: [garbage::Context; 2],
    /// Road wear per team (home first), extra fatigue per play as a share:
    /// travel, time zones against the body clock and thin air for a team
    /// not used to it (trial 4 topic 3).
    pub wear: [f64; 2],
    /// Per team, 0-1: how unused it is to this game's cold.
    pub cold_unused: [f64; 2],
    /// The season, for rules that changed by era (celebrations); 0 reads
    /// as today's rules.
    pub year: i32,
    /// Keep this game's setup in the result so it can be watched back
    /// (your games; `replay.rs`).
    pub keep: bool,
    /// For the views only: the local kickoff hour (0 when unknown) and a
    /// roof over the field.
    pub kickoff: f32,
    pub indoor: bool,
}

impl<'a> GameSetup<'a> {
    pub fn new(home: &'a GameTeam, away: &'a GameTeam, level: Level, seed: u64) -> GameSetup<'a> {
        // Two-way play: small high schools only (fix list 3.10, 3.11).
        let margin = match level {
            Level::HS => 4.0,
            _ => 99.0,
        };
        GameSetup {
            home,
            away,
            rules: Rules::for_level(level),
            cond: Conditions::default(),
            seed,
            neutral: false,
            playoff: false,
            record: false,
            two_way_margin: [margin, margin],
            context: [garbage::Context::default(); 2],
            wear: [0.0; 2],
            cold_unused: [0.0; 2],
            year: 0,
            keep: false,
            kickoff: 0.0,
            indoor: false,
        }
    }
}

/// A finished game: the stats database for it, the injuries, how each
/// freelancer's trust moved, and tracking for recorded plays.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameResult {
    pub stats: GameStats,
    pub injuries: Vec<(usize, Injury)>,
    /// (team, player id, trust change)
    pub trust: Vec<(usize, u32, f64)>,
    pub frames: Vec<(u32, Vec<Frame>)>,
    pub crew: Option<Crew>,
    /// Garbage-time calls and why: (play number, team, what he did).
    pub decisions: Vec<(u32, usize, String)>,
    /// What each team showed on film this game (home, away).
    pub film: [calling::Film; 2],
    /// The setup, when it was asked to be kept (`GameSetup::keep`), so the
    /// game can be played again from its seed.
    #[serde(skip)]
    pub replay: Option<Box<super::replay::ReplaySetup>>,
    /// The coin toss: the winner and his choice (game day topic 3).
    #[serde(default)]
    pub toss: Option<(usize, TossChoice)>,
    /// Timeouts, challenges and the other moments between plays: (after
    /// play number, team, what happened).
    #[serde(default)]
    pub log: Vec<(u32, usize, String)>,
}

struct Side<'a> {
    team: &'a GameTeam,
    depth: Depth,
    energy: Vec<f64>,
    edge: Vec<f64>,
    trust: Vec<f64>,
    out: Vec<bool>,
    hurt: Vec<Option<InjuryKind>>,
    timeouts: u8,
    mem: Memory,
    /// The head coach's read of a decided game, and who sits because of it.
    coach: garbage::Coach,
    benched: Vec<bool>,
    /// Can't substitute this snap (the offense went no-huddle).
    frozen: bool,
    /// Per player, the spots he knows (bits of `Spot`): his listed spots,
    /// and the spot he worked at in this week's practice (fix list 3.3).
    familiar: Vec<u32>,
    /// Snaps played this game, and the most he may play (update 2, G17:
    /// a snap limit back from injury; u16::MAX none).
    snaps: Vec<u16>,
    limit: Vec<u16>,
    /// Kept to base downs this week (G14), and the line rotation's role
    /// (G15: 1 the whole series, 2 every other series, 3 rush downs only).
    base_only: Vec<bool>,
    rot: Vec<u8>,
    /// This snap is a passing down, and the other team's drives so far (the
    /// defense's series).
    passing: bool,
    series: u16,
    /// The plan's lines that have fired (G20, bits by kind) and the tricks
    /// used (1 a fake punt, 2 a surprise onside kick).
    fired: u8,
    tricks: u8,
    /// The quarterback's checks and the "if" lines this game (G19, G20):
    /// your plan's, else what this staff reads from the other team's film.
    checks: u8,
    ifs: Vec<u8>,
    /// The call sheets this game: your week's, else what this staff drafts
    /// at kickoff (`world::plans::kickoff_card`).
    card: super::sheet::Card,
    /// Script calls used this game, offense then defense (G11): a script
    /// call counts once it is run.
    scripted: [usize; 2],
    /// Each "every other series" man's turn (G15): they split the series
    /// by their order on the board, the first playing the first series.
    turn: Vec<u8>,
    /// The scrimmage lineup is being picked (the week's plan sits men only
    /// there, never on a kick unit), and this side is on defense (the line
    /// rotation is a defense's).
    scrimmage: bool,
    defending: bool,
}

impl<'a> Side<'a> {
    fn new(team: &'a GameTeam, margin: f64, timeouts: u8) -> Side<'a> {
        let n = team.players.len();
        let depth = depth_chart(team, margin);
        // The week's practice: the starters work at the spots they will
        // play, so a backup stepping in for a man out this week knows his
        // spot by Saturday. A coach keeps a man where he knows the job
        // unless moving him is clearly better.
        let mut familiar: Vec<u32> = team.players.iter().map(|p| p.spots_played | p.spot.bit()).collect();
        // A left-handed starter's blind side is his right (`spots::blind_side`).
        let lefty = depth.list(Position::QB).first().is_some_and(|&i| team.players[i].left_handed);
        for pos in Position::ALL {
            let spots = spots::starter_spots(pos);
            if spots.len() < 2 {
                continue;
            }
            let list: Vec<usize> = depth.list(pos).iter().take(spots.len()).copied().collect();
            let value = |i: usize, sp: Spot| {
                let p = &team.players[i];
                let cost = if familiar[i] & sp.bit() != 0 { 0.0 } else { fill_in(sp, p.spot) };
                p.spot_r[spots::blind_side(sp, lefty).index()] - 0.5 * cost
            };
            let order = best_order(&list, spots, &value);
            for (k, i) in order.into_iter().enumerate() {
                if let Some(i) = i {
                    familiar[i] |= spots[k].bit();
                }
            }
        }
        Side {
            team,
            depth,
            energy: vec![team.prep.fresh.clamp(0.5, 1.0); n],
            edge: vec![0.0; n],
            trust: team.players.iter().map(|p| p.trust).collect(),
            out: vec![false; n],
            hurt: vec![None; n],
            timeouts,
            mem: Memory::default(),
            coach: garbage::Coach::default(),
            benched: vec![false; n],
            frozen: false,
            familiar,
            snaps: vec![0; n],
            base_only: team.players.iter().map(|p| team.prep.base_only.contains(&p.id)).collect(),
            rot: team
                .players
                .iter()
                .map(|p| team.prep.rotation.iter().find(|r| r.0 == p.id).map_or(0, |r| r.1))
                .collect(),
            passing: false,
            series: 0,
            fired: 0,
            tricks: 0,
            checks: team.prep.checks,
            ifs: team.prep.ifs.clone(),
            card: super::sheet::Card::of(&team.prep),
            scripted: [0, 0],
            turn: team
                .players
                .iter()
                .map(|p| {
                    let order = team.prep.rotation.iter().filter(|r| r.1 == 2).position(|r| r.0 == p.id);
                    order.map_or(0, |k| (k % 2) as u8)
                })
                .collect(),
            scrimmage: false,
            defending: false,
            limit: team
                .players
                .iter()
                .map(|p| team.prep.limits.iter().find(|l| l.0 == p.id).map_or(u16::MAX, |l| l.1))
                .collect(),
        }
    }

    /// He has played his snap limit (G17): he stays on the sideline.
    fn spent(&self, i: usize) -> bool {
        self.snaps[i] >= self.limit[i]
    }

    /// He sits this snap by the week's plan: his snap limit; at scrimmage,
    /// base downs only on a passing down, and on defense the line rotation
    /// (G14, G15, G17). The series is the other team's drive, the first
    /// one 1: an "every other series" man plays his turn's series.
    fn held(&self, i: usize) -> bool {
        if self.spent(i) {
            return true;
        }
        if !self.scrimmage {
            return false;
        }
        (self.base_only[i] && self.passing)
            || (self.defending
                && match self.rot[i] {
                    2 => (self.series.max(1) - 1 + self.turn[i] as u16) % 2 == 1,
                    3 => !self.passing,
                    _ => false,
                })
    }

    /// A kick unit's `n` men: the returner or the kicker first, then the
    /// backups who fit the job.
    fn kick_unit(&self, receiving: bool, n: usize) -> Vec<usize> {
        let ps = &self.team.players;
        let mut used = Vec::new();
        let mut on = Vec::new();
        if receiving {
            let rets = self.depth.kick_returners.iter().copied().filter(|&i| !self.out[i]).collect::<Vec<_>>();
            on.extend(rets.iter().take(1));
        } else {
            on.push(self.kickoff_kicker());
        }
        used.extend(on.iter().copied());
        // Coverage and return units: fast backups first.
        let mut pool: Vec<usize> = (0..ps.len())
            .filter(|&i| !self.out[i] && !used.contains(&i) && !ps[i].position.is_specialist() && !self.benched[i])
            .collect();
        let starters: Vec<usize> =
            [Position::QB].iter().flat_map(|p| self.depth.list(*p).iter().take(1).copied()).collect();
        pool.retain(|i| !starters.contains(i));
        // A starter only beats a backup who is clearly worse (part C 3: real
        // units are mostly backups and young players; proposed, not
        // confirmed: a starter counts 10 points less).
        // A return unit wants men who can block in space (tight ends,
        // linebackers, fullbacks; under the 2025 NFL kickoff, bigger bodies
        // in the setup zone); a coverage unit wants speed, tackling and the
        // hands to get off a block (proposed, not confirmed: both used to
        // pick by speed, tackling and blocking alike).
        let score: Vec<f64> = (0..ps.len())
            .map(|i| {
                let p = &ps[i];
                let skill = if receiving {
                    0.3 * p.a(Attr::Speed) + 0.4 * p.a(Attr::BlockingInSpace) + 0.3 * p.a(Attr::Strength)
                } else {
                    0.4 * p.a(Attr::Speed) + 0.35 * p.a(Attr::Tackling) + 0.25 * p.a(Attr::BlockShedding)
                };
                skill - if self.starts(i) { 10.0 } else { 0.0 }
            })
            .collect();
        pool.sort_by(|&a, &b| score[b].total_cmp(&score[a]));
        on.extend(pool.into_iter().take(n - on.len()));
        // A thin roster in a decided game (an arena team's 20-odd, a small
        // school's): the pulled starters come back before a unit takes the
        // field short, as they do on offense and defense, and a kick always
        // has someone to field it.
        if on.len() < n {
            let mut back: Vec<usize> = (0..ps.len())
                .filter(|&i| !self.out[i] && !on.contains(&i) && !ps[i].position.is_specialist())
                .collect();
            back.sort_by(|&a, &b| score[b].total_cmp(&score[a]));
            on.extend(back.into_iter().take(n - on.len()));
        }
        if on.is_empty() {
            on.push((0..ps.len()).find(|&i| !self.out[i]).unwrap_or(0));
        }
        // Your kickoff units (section 6b): your returner first on a return.
        if receiving {
            match self.package(4).first() {
                Some(&kr) => self.with_package(4, &[kr], on),
                None => on,
            }
        } else {
            let k = on[0];
            self.with_package(5, &[k], on)
        }
    }

    /// Rating points he gives up at a spot he doesn't know (3.3): footwork
    /// from the other side, the center's snap and calls, the middle
    /// linebacker's calls.
    fn unfamiliar(&self, i: usize, sp: Spot) -> f64 {
        if self.familiar[i] & sp.bit() != 0 { 0.0 } else { fill_in(sp, self.team.players[i].spot) }
    }

    /// What he gives at a spot this snap.
    fn at_spot(&self, i: usize, sp: Spot) -> f64 {
        self.team.players[i].spot_r[sp.index()] - self.unfamiliar(i, sp)
    }

    /// A lineman at a spot on the line, facing the demands of the blind side
    /// when his quarterback is a lefty.
    fn on_line(&self, i: usize, sp: Spot, lefty: bool) -> f64 {
        self.team.players[i].spot_r[spots::blind_side(sp, lefty).index()] - self.unfamiliar(i, sp)
    }

    /// The fullback (3.4): a back or a tight end, never anyone else; the
    /// one who plays it best.
    fn pick_fb(&self, used: &mut Vec<usize>) -> Option<usize> {
        let th = if self.frozen { 0.0 } else { self.threshold(Position::RB) };
        let ok = |i: &usize| !self.out[*i] && !used.contains(i) && !self.benched[*i] && !self.held(*i);
        let cands: Vec<usize> = self
            .depth
            .list(Position::RB)
            .iter()
            .chain(self.depth.list(Position::TE))
            .copied()
            .filter(|i| ok(i) && matches!(self.team.players[*i].position, Position::RB | Position::TE))
            .collect();
        let fresh = cands.iter().copied().filter(|&i| self.energy[i] >= th);
        let pick =
            fresh.max_by(|&a, &b| self.at_spot(a, Spot::FB).total_cmp(&self.at_spot(b, Spot::FB))).or_else(|| {
                cands.iter().copied().max_by(|&a, &b| self.at_spot(a, Spot::FB).total_cmp(&self.at_spot(b, Spot::FB)))
            });
        if let Some(i) = pick {
            used.push(i);
        }
        pick
    }

    /// Fatigue threshold for swapping a player out (topic 6): defensive
    /// linemen rotate, backs share, linemen and quarterbacks rarely come out.
    /// A strict coach rides his starters harder.
    fn threshold(&self, pos: Position) -> f64 {
        let base = match pos {
            Position::EDGE | Position::DT => 0.84,
            Position::RB => 0.60,
            Position::WR | Position::TE => 0.74,
            Position::LB | Position::CB | Position::S => 0.68,
            _ => 0.5,
        };
        base - 0.006 * (self.team.staff.strictness - 10.0)
    }

    /// The best `n` available at a spot, swapping out tired starters when a
    /// fresh backup isn't much worse.
    /// Whether a player starts at his own position.
    fn starts(&self, i: usize) -> bool {
        let pos = self.team.players[i].position;
        self.depth.list(pos).iter().take(crate::league::depth_slots(pos).starters).any(|&j| j == i)
    }

    /// A coverage or return unit's men at a spot (part C 3): the backups
    /// first, as real units are mostly backups and young players, then the
    /// starters when the backups run out.
    /// Your package for a board (section 6b): the players you put on it,
    /// in slot order, who can go.
    fn package(&self, board: u8) -> Vec<usize> {
        let Some(ids) = self.team.prep.package(board) else { return Vec::new() };
        ids.iter()
            .filter_map(|&id| self.team.players.iter().position(|p| p.id == id))
            .filter(|&i| !self.out[i] && !self.benched[i])
            .collect()
    }

    /// A unit with your package's men on it: `keep` stays (the kicker, the
    /// punter), then your men, then the staff's own choice fills the rest.
    fn with_package(&self, board: u8, keep: &[usize], default: Vec<usize>) -> Vec<usize> {
        let mine = self.package(board);
        if mine.is_empty() {
            return default;
        }
        let n = default.len();
        let mut out: Vec<usize> = keep.to_vec();
        for i in mine.into_iter().chain(default) {
            if out.len() >= n {
                break;
            }
            if !out.contains(&i) {
                out.push(i);
            }
        }
        out
    }

    /// Picks at a position with your package's men first (a safety you
    /// listed as the nickel plays there).
    fn pick_pref(&self, pos: Position, n: usize, used: &mut Vec<usize>, board: Option<u8>) -> Vec<usize> {
        let Some(b) = board else { return self.pick(pos, n, used) };
        let Some(ids) = self.team.prep.package(b) else { return self.pick(pos, n, used) };
        let slots = crate::training::seat::Board::ALL.get(b as usize).map_or(&[][..], |x| x.slots());
        let mut out = Vec::new();
        // Package depth (G15): the slot's second and third man when the
        // first can't go.
        let backs = self.team.prep.backups.iter().find(|(x, _)| *x == b).map_or(&[][..], |(_, v)| v.as_slice());
        for (k, &id) in ids.iter().enumerate() {
            if out.len() >= n || slots.get(k).is_none_or(|s| s.1 != pos) {
                continue;
            }
            let first = [id, backs.get(2 * k).copied().unwrap_or(0), backs.get(2 * k + 1).copied().unwrap_or(0)];
            let pick = first.into_iter().filter(|&id| id != 0).find_map(|id| {
                self.team.players.iter().position(|p| p.id == id).filter(|&i| {
                    !self.out[i] && !self.benched[i] && !self.held(i) && !used.contains(&i) && !out.contains(&i)
                })
            });
            if let Some(i) = pick {
                out.push(i);
                used.push(i);
            }
        }
        if out.len() < n {
            out.extend(self.pick(pos, n - out.len(), used));
        }
        out
    }

    /// The punt team's gunners (speed update S10): the two backups who play
    /// fastest and get off a jam best, receivers, corners or safeties (the
    /// burner among them), corners from the cover list when short.
    fn gunners(&self, used: &mut Vec<usize>) -> Vec<usize> {
        let score = |i: usize| {
            let p = &self.team.players[i];
            0.5 * super::roster::deep_speed(p) + 0.3 * p.a(Attr::Release) + 0.2 * p.a(Attr::Strength)
        };
        let mut pool: Vec<usize> = [Position::WR, Position::CB, Position::S]
            .iter()
            .flat_map(|&pos| self.depth.list(pos).iter().copied())
            .filter(|&i| !self.out[i] && !used.contains(&i) && !self.benched[i] && !self.held(i) && !self.starts(i))
            .collect();
        pool.sort_by(|&a, &b| score(b).total_cmp(&score(a)));
        pool.dedup();
        let mut out: Vec<usize> = pool.into_iter().take(2).collect();
        used.extend(out.iter().copied());
        if out.len() < 2 {
            out.extend(self.cover_pick(Position::CB, 2 - out.len(), used));
        }
        out
    }

    /// How much the offense leans on its burner, while he can go (S10).
    fn burner_lean(&self) -> f64 {
        match self.depth.burner {
            Some(b) if !self.out[b] && !self.benched[b] => self.depth.burner_lean,
            _ => 0.0,
        }
    }

    fn cover_pick(&self, pos: Position, n: usize, used: &mut Vec<usize>) -> Vec<usize> {
        let list: Vec<usize> = self
            .depth
            .list(pos)
            .iter()
            .copied()
            .filter(|&i| !self.out[i] && !used.contains(&i) && !self.benched[i] && !self.held(i))
            .collect();
        let (starters, backups): (Vec<usize>, Vec<usize>) = list.iter().partition(|&&i| self.starts(i));
        let out: Vec<usize> = backups.into_iter().chain(starters).take(n).collect();
        if out.len() < n {
            return self.pick(pos, n, used);
        }
        used.extend(out.iter().copied());
        out
    }

    fn pick(&self, pos: Position, n: usize, used: &mut Vec<usize>) -> Vec<usize> {
        let list: Vec<usize> = self
            .depth
            .list(pos)
            .iter()
            .copied()
            .filter(|&i| !self.out[i] && !used.contains(&i) && !self.benched[i] && !self.held(i))
            .collect();
        let th = if self.frozen { 0.0 } else { self.threshold(pos) };
        let mut out = Vec::new();
        let mut skipped = Vec::new();
        for &i in &list {
            if out.len() == n {
                break;
            }
            // The rotation's whole-series men stay on (G15).
            if self.energy[i] < th && self.rot[i] != 1 {
                // The more worn he is, the bigger the drop a coach accepts to
                // spell him, so a star back or rusher still comes out for a
                // breather. It used to be a flat 9 points: a star never came
                // out and took nine of ten carries (trial 7 finding 2). Now
                // the lead back takes about 65% of the backs' carries in an
                // NFL game (real 70%, 59% over a season with the games he
                // misses), 53% in FBS, and a star 15 points better about 82%
                // (real stars about 80-85%; proposed, not confirmed).
                let allow = 60.0 * (th - self.energy[i]);
                let next = list.iter().copied().find(|&j| {
                    j != i
                        && !out.contains(&j)
                        && !skipped.contains(&j)
                        && self.energy[j] >= th
                        && self.rating(j, pos) >= self.rating(i, pos) - allow
                });
                if next.is_some() {
                    skipped.push(i);
                    continue;
                }
            }
            out.push(i);
        }
        for &i in &skipped {
            if out.len() < n {
                out.push(i);
            }
        }
        // A thin roster: a rested man from the same position group (fix
        // list 3.2); in a decided game the pulled starters come back before
        // anyone plays out of his group, the least important first; only
        // then the best athlete left from anywhere, an emergency (3.6).
        let all = 0..self.team.players.len();
        let group = spots::group(pos);
        let in_group = |i: usize| spots::group(self.team.players[i].position) == group;
        while out.len() < n {
            let free = |i: usize| !self.out[i] && !used.contains(&i) && !out.contains(&i);
            let best = |f: &dyn Fn(usize) -> bool| {
                all.clone()
                    .filter(|&i| free(i) && f(i))
                    .max_by(|&a, &b| self.rating(a, pos).total_cmp(&self.rating(b, pos)))
            };
            // The week's plan holds until the group runs out: a man at his
            // snap limit, kept to base downs or resting his series comes
            // back only before anyone from outside the group.
            let any = best(&|i| !self.benched[i] && !self.held(i) && in_group(i))
                .or_else(|| self.depth.list(pos).iter().rev().copied().find(|&i| free(i) && !self.held(i)))
                .or_else(|| best(&|i| !self.benched[i] && in_group(i)))
                .or_else(|| self.depth.list(pos).iter().rev().copied().find(|&i| free(i)))
                .or_else(|| best(&in_group))
                .or_else(|| best(&|i| !self.benched[i]))
                .or_else(|| best(&|_| true));
            match any {
                Some(i) => out.push(i),
                None => break,
            }
        }
        used.extend(out.iter().copied());
        out
    }

    fn rating(&self, i: usize, pos: Position) -> f64 {
        self.team.players[i].overall_at(pos)
    }

    /// The first player on a list who is still in the game.
    fn first_up(&self, list: &[usize]) -> Option<usize> {
        list.iter().copied().find(|&i| !self.out[i])
    }

    /// The best leg left in the game at a kicking job.
    fn best_leg(&self, pos: Position) -> usize {
        (0..self.team.players.len())
            .filter(|&i| !self.out[i])
            .max_by(|&a, &b| self.rating(a, pos).total_cmp(&self.rating(b, pos)))
            .unwrap_or(0)
    }

    /// The field goal kicker (fix list 1.9): the K list, then the punter's
    /// leg, then whoever kicks best.
    fn fg_kicker(&self) -> usize {
        self.first_up(self.depth.list(Position::K))
            .or_else(|| self.first_up(self.depth.list(Position::P)))
            .unwrap_or_else(|| self.best_leg(Position::K))
    }

    /// The punter (fix list 1.9): the P list, then the kicker's leg.
    fn punter(&self) -> usize {
        self.first_up(self.depth.list(Position::P))
            .or_else(|| self.first_up(self.depth.list(Position::K)))
            .unwrap_or_else(|| self.best_leg(Position::P))
    }

    /// The kickoff kicker (fix list 1.8): the best leg among the kickers.
    fn kickoff_kicker(&self) -> usize {
        self.first_up(&self.depth.kickoff).unwrap_or_else(|| self.fg_kicker())
    }

    /// The long snapper: the depth chart's; if he is hurt, or kicking or
    /// punting after an injury to a leg, the next listed long snapper, else
    /// the best snap left in the game who is not kicking or punting.
    fn snapper(&self) -> usize {
        let d = &self.depth;
        let legs = [self.fg_kicker(), self.punter()];
        if !self.out[d.snapper] && !legs.contains(&d.snapper) {
            return d.snapper;
        }
        let ps = &self.team.players;
        let free = |i: usize| !self.out[i] && !legs.contains(&i);
        let snap = |i: usize| ps[i].a(Attr::LongSnapping) + 0.1 * ps[i].a(Attr::FootballIq);
        d.list(Position::LS)
            .iter()
            .copied()
            .find(|&i| free(i) && ps[i].position == Position::LS)
            .or_else(|| (0..ps.len()).filter(|&i| free(i)).max_by(|&a, &b| snap(a).total_cmp(&snap(b))))
            .unwrap_or(d.snapper)
    }

    /// The holder for a kick: the depth chart's (the punter, or the backup
    /// quarterback); if he is hurt, kicking or snapping, a quarterback (the
    /// backup first), else the surest hands left.
    fn holder(&self, kicker: usize, snapper: usize) -> usize {
        let free = |i: usize| !self.out[i] && i != kicker && i != snapper;
        if free(self.depth.holder) {
            return self.depth.holder;
        }
        let ps = &self.team.players;
        let qbs = self.depth.list(Position::QB);
        let hands = |i: usize| ps[i].a(Attr::Catching) + ps[i].a(Attr::Concentration);
        qbs.iter()
            .skip(1)
            .chain(qbs.iter().take(1))
            .copied()
            .find(|&i| free(i))
            .or_else(|| (0..ps.len()).filter(|&i| free(i)).max_by(|&a, &b| hands(a).total_cmp(&hands(b))))
            .unwrap_or(self.depth.holder)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum Next {
    Kickoff { team: usize, from: f64, onside: bool },
    Pat(usize),
    Scrimmage,
}

/// A fake punt's two units on the field, by their scrimmage roles, and
/// whether the return unit showed the look (G18).
type FakeUnits = (Vec<(ORole, usize)>, Vec<(DSlot, usize)>, bool);

struct Game<'a> {
    s: &'a GameSetup<'a>,
    sides: [Side<'a>; 2],
    rng: Rng,
    crew: Crew,
    stats: GameStats,
    res: GameResult,
    quarter: u8,
    clock: f64,
    running: bool,
    poss: usize,
    x: f64,
    down: u8,
    togo: f64,
    ball_y: f64,
    next: Next,
    over: bool,
    receive_first: usize,
    n: u32,
    after_turnover: bool,
    two_min_warned: bool,
    // overtime
    ot: bool,
    ot_round: u32,
    ot_poss: [u32; 2],
    ot_first: usize,
    drive_start_poss: usize,
    /// The coaches' own judgment calls draw from their own stream, so a game
    /// that never gets out of hand plays out the same with or without them.
    mind: Rng,
    /// The last garbage-time review: (drives started, quarter).
    reviewed: Option<(u32, u8)>,
    /// Time since the last snap as a share of a normal huddle's (S10):
    /// players get their breath back with time, not per play.
    pace: f64,
    /// The offense went to the line without a huddle (S10): the defense
    /// can't substitute.
    no_huddle: bool,
    /// Game day control (section 9): your link to the game, set only on a
    /// game you play live.
    ctl: Option<Control>,
    /// The game stopped for you; everything after is thrown away.
    paused: bool,
    /// A point that stops for you: the game plays on until your staff's
    /// choice is known, then stops.
    probe: Option<Ask>,
    /// Your answer for this snap.
    snap_input: Option<Input>,
    /// The model's numbers for this snap's 4th down, for its play record.
    fourth_note: Option<[f32; 4]>,
    /// Your marks for the snap's record: (you called the offense, you
    /// called the defense, off the sheet, the staff's call you replaced).
    mark: (bool, bool, bool, String),
    /// The officials (game day topic 6): their eye and their own stream.
    eye: live::Eye,
    refs: Rng,
    replay: Replay,
    challenges: [u8; 2],
    challenge_wins: [u8; 2],
    /// The ends were swapped at the toss (a winner took the wind).
    flipped: bool,
    /// Your offense's tempo.
    tempo: Tempo,
    /// The coin toss and overtime toss draw here.
    toss_rng: Rng,
    /// Yards the next kickoff moves back: an excessive celebration after a
    /// touchdown is enforced on it.
    kick_back: f64,
    /// A try after a touchdown is being played: untimed, and it scores
    /// only as a try (one, two or three points), never as a touchdown.
    trying: bool,
    /// This snap must be played, not punted or kicked: the other leagues'
    /// fourth-down play in place of an onside kick (X7).
    must_go: bool,
    /// The defense's call this snap came off its script (G11).
    def_script: bool,
    /// The return team showed the look on this punt (G18), for its record.
    kick_look: bool,
    /// A fake punt's two units on the field and the look (G18): the next
    /// scrimmage snap is played with them.
    fake: Option<FakeUnits>,
}

/// Play a game.
pub fn play_game(setup: &GameSetup) -> GameResult {
    play_game_ctl(setup, Control { team: 9, ..Control::default() }).0
}

/// Play a game with your control (`live`): to the end, or to the next
/// point that waits for you. Returns what has been played, the control
/// (with the decision that waits, if any) and where the game stands.
pub fn play_game_ctl(setup: &GameSetup, ctl: Control) -> (GameResult, Control, View) {
    // The field this game is played on (the other leagues' books, X7).
    let _field = FieldGuard::set(setup.rules.code.field);
    let mut rng = Rng::new(((setup.seed >> 32) as u32) ^ (setup.seed as u32) ^ 0x9E37_79B9);
    let crew = Crew::draw(&mut rng);
    let t = setup.rules.timeouts;
    let mut sides =
        [Side::new(setup.home, setup.two_way_margin[0], t), Side::new(setup.away, setup.two_way_margin[1], t)];
    // Game prep: each staff studies the other's film (plays and tactics
    // topic 5.1) and makes its matchup plan (5.4).
    let (hs, as_) = (setup.home.staff, setup.away.staff);
    let mut prep = Rng::new(((setup.seed >> 17) as u32) ^ (setup.seed as u32).rotate_left(5) ^ 0x3C6E_F372);
    sides[0].mem.prepare(&as_, &setup.away.film, &hs, &mut prep);
    sides[1].mem.prepare(&hs, &setup.home.film, &as_, &mut prep);
    // The quarterback's checks and the "if" lines (G19, G20): each staff
    // reads them off the other team's film, the same way for every team;
    // your week's plan sets yours.
    for (k, side) in sides.iter_mut().enumerate() {
        if !side.team.prep.planned {
            let film = if k == 0 { &setup.away.film } else { &setup.home.film };
            side.checks = staff_checks(film);
            side.ifs = staff_ifs(film);
        }
    }
    // The call sheets (update 2, G7-G13): a team whose week gave it none
    // has its coordinators draft one now, from the other team's film and
    // the tells in its own, the same draft as yours. Its own key, so the
    // game's stream is the one it was.
    for (k, side) in sides.iter_mut().enumerate() {
        if side.team.prep.planned || !side.card.is_empty() || !super::sheet::drafts_at_kickoff(k, setup.rules.level) {
            continue;
        }
        let (opp, own) = if k == 0 { (setup.away, setup.home) } else { (setup.home, setup.away) };
        let qb_run = side
            .depth
            .list(Position::QB)
            .iter()
            .copied()
            .find(|&i| !side.out[i])
            .map_or(0.0, |i| qb_run(&side.team.players[i]));
        let inp = crate::world::plans::DraftIn {
            film: opp.film.clone(),
            level: setup.rules.level,
            two_from: setup.rules.two_point_from,
            qb_run,
            seed: ((setup.seed >> 29) as u32) ^ (setup.seed as u32).rotate_left(11) ^ 0x51EE_7000 ^ k as u32,
            favs: Vec::new(),
            tells: crate::world::plans::film_tells(&own.film),
            designs: Vec::new(),
        };
        side.card = crate::world::plans::kickoff_card(side.team, &inp);
    }
    let plans: Vec<calling::Plan> = (0..2)
        .map(|k| {
            let (me, them) = (&sides[k], &sides[1 - k]);
            let rated = |side: &Side, pos: Position, n: usize, sp: Spot| -> Vec<(usize, f64)> {
                side.depth.list(pos).iter().take(n).map(|&i| (i, side.team.players[i].spot_r[sp.index()])).collect()
            };
            let their_cbs = rated(them, Position::CB, 2, Spot::CB);
            let their_wrs = rated(them, Position::WR, 3, Spot::WR);
            let our_cbs: Vec<f64> = rated(me, Position::CB, 2, Spot::CB).iter().map(|x| x.1).collect();
            let qb_run = them.depth.list(Position::QB).first().map_or(0.0, |&i| qb_run(&them.team.players[i]));
            // Their rushers on our pass blockers (A8).
            let mut rushers = rated(them, Position::EDGE, 2, Spot::EDGE);
            rushers.extend(rated(them, Position::DT, 2, Spot::DT));
            let our_ol: Vec<f64> = rated(me, Position::OT, 2, Spot::LT).iter().map(|x| x.1).collect();
            // Their receivers' and our corners' deep speed (S10a-S10d): their
            // top three and their burner against our top two.
            let deep = |side: &Side, i: usize| super::roster::deep_speed(&side.team.players[i]);
            let mut their_deep: Vec<(usize, f64)> = their_wrs.iter().map(|&(i, _)| (i, deep(them, i))).collect();
            if let Some(b) = them.depth.burner.filter(|b| !their_deep.iter().any(|x| x.0 == *b)) {
                their_deep.push((b, deep(them, b)));
            }
            let our_deep: Vec<f64> = me.depth.list(Position::CB).iter().take(2).map(|&i| deep(me, i)).collect();
            calling::make_plan(
                &me.team.staff,
                &their_cbs,
                &their_wrs,
                &our_cbs,
                qb_run,
                &rushers,
                &our_ol,
                (&their_deep, &our_deep),
                &mut prep,
            )
        })
        .collect();
    sides[0].mem.plan = plans[0];
    sides[1].mem.plan = plans[1];
    // Your answers to their personnel groups this week (update 2, G16).
    for side in sides.iter_mut() {
        side.mem.plan.answers = side.team.prep.answers;
    }
    // Moves against the other team's habits (tendencies topic 4.4), from
    // what each staff's film study reads of them. Own stream, so the plans
    // above keep their draws.
    let mut habit_rng = Rng::new(((setup.seed >> 23) as u32) ^ (setup.seed as u32).rotate_left(7) ^ 0x7E4D_A11B);
    for k in 0..2 {
        let them = &sides[1 - k];
        let study = sides[k].team.staff.film_study;
        let mean = |pos: &[Position], n: usize, t: crate::tendencies::Tend| {
            let v: Vec<f64> = pos
                .iter()
                .flat_map(|&p| them.depth.list(p).iter().take(n).map(|&i| them.team.players[i].tend.get(t)))
                .collect();
            if v.is_empty() { crate::tendencies::MID } else { v.iter().sum::<f64>() / v.len() as f64 }
        };
        use crate::tendencies::Tend;
        let snap = mean(&[Position::EDGE, Position::DT], 2, Tend::SnapTiming);
        let jump = mean(&[Position::CB, Position::S], 2, Tend::JumpRoutes);
        let up = mean(&[Position::EDGE], 2, Tend::Upfield);
        let ret = them
            .depth
            .kick_returners
            .first()
            .map_or(crate::tendencies::MID, |&i| them.team.players[i].tend.get(Tend::ReturnRisk));
        sides[k].mem.plan.vs = calling::VsHabits {
            hard_count: calling::read_habit(snap, study, &mut habit_rng),
            double_moves: calling::read_habit(jump, study, &mut habit_rng),
            draws: calling::read_habit(up, study, &mut habit_rng),
            kick_deep: calling::read_habit(ret, study, &mut habit_rng),
        };
    }
    // Your own film study (game day topic 1): no draws.
    if ctl.team < 2
        && let Some(f) = &ctl.prep.study
    {
        sides[ctl.team].mem.study(f);
    }
    let mut mind = Rng::new(((setup.seed >> 29) as u32) ^ (setup.seed as u32).rotate_left(11) ^ 0x6A09_E667);
    let habits = garbage::habits(setup.rules.level);
    for (k, side) in sides.iter_mut().enumerate() {
        let st = side.team.staff;
        side.coach = garbage::Coach::new(st.game_management, st.sportsmanship, setup.context[k], &habits, &mut mind);
    }
    let mut g = Game {
        s: setup,
        sides,
        rng,
        crew,
        stats: GameStats::default(),
        res: GameResult::default(),
        quarter: 1,
        clock: setup.rules.quarter,
        running: false,
        poss: 0,
        x: 25.0,
        down: 1,
        togo: 10.0,
        ball_y: mid(),
        next: Next::Scrimmage,
        over: false,
        receive_first: 0,
        n: 0,
        after_turnover: false,
        two_min_warned: false,
        pace: 1.0,
        no_huddle: false,
        kick_back: 0.0,
        ot: false,
        ot_round: 0,
        ot_poss: [0, 0],
        ot_first: 0,
        drive_start_poss: 0,
        mind,
        reviewed: None,
        ctl: if ctl.team < 2 { Some(ctl) } else { None },
        paused: false,
        probe: None,
        snap_input: None,
        fourth_note: None,
        mark: (false, false, false, String::new()),
        eye: live::Eye { miss: 1.0 },
        refs: Rng::new(live::ref_seed(setup.seed)),
        replay: Replay::of(setup.rules.level, setup.home.staff.year),
        challenges: [0; 2],
        challenge_wins: [0; 2],
        flipped: false,
        tempo: Tempo::Normal,
        toss_rng: Rng::new(live::toss_seed(setup.seed)),
        trying: false,
        must_go: false,
        def_script: false,
        kick_look: false,
        fake: None,
    };
    if let Some(c) = g.ctl.as_mut() {
        c.asked = 0;
        c.pending = None;
        c.pause_at = None;
    }
    // Your matchup plan and tempo for the game (topic 2).
    if let Some(c) = g.ctl.as_ref() {
        let (team, plan, tempo) = (c.team, c.prep.plan, c.prep.tempo);
        if plan != live::PlanChange::default() {
            g.apply_plan(team, plan);
        }
        g.tempo = tempo;
    }
    g.eye = live::draw_eye(&mut g.refs);
    g.challenges = [g.replay.challenges; 2];
    let drawn = if g.rng.chance(0.5) { 0 } else { 1 };
    g.receive_first = g.coin_toss(drawn);
    let kick_from = setup.rules.kickoff_from;
    g.next = Next::Kickoff { team: 1 - g.receive_first, from: kick_from, onside: false };
    g.stats.quarters.push([0, 0]);
    let mut guard = 0;
    while !g.over && guard < 600 {
        guard += 1;
        if g.paused {
            break;
        }
        match g.next {
            Next::Kickoff { team, from, onside } => g.kickoff(team, from, onside),
            Next::Pat(team) => g.pat(team),
            Next::Scrimmage => {
                let before = g.stats.plays.len();
                g.scrimmage();
                // The 4th down's numbers go on the snap that was played.
                if let Some(v) = g.fourth_note.take()
                    && let Some(r) = g.stats.plays[before..].iter_mut().find(|r| r.kind != PlayType::NoPlay)
                {
                    r.fourth = Some(v);
                }
            }
        }
        if g.paused {
            break;
        }
        g.end_of_period();
    }
    let view = g.view();
    g.stats.score = g.stats.team.each_ref().map(|t| t.points);
    g.res.crew = Some(g.crew);
    for side in 0..2 {
        for (i, p) in g.sides[side].team.players.iter().enumerate() {
            let d = g.sides[side].trust[i] - p.trust;
            if d.abs() > 1e-9 {
                g.res.trust.push((side, p.id, d));
            }
            if let Some(l) = g.stats.players[side].get_mut(&p.id) {
                l.games = 1;
            }
        }
    }
    g.res.stats = g.stats;
    let mut ctl = g.ctl.take().unwrap_or_default();
    if !g.paused {
        ctl.pending = None;
        ctl.pause_at = None;
    }
    // Your game keeps its setup and, when you played it live, your answers,
    // so its film plays again exactly (`replay.rs`).
    if setup.keep && !g.paused {
        let live = (ctl.team < 2).then(|| ctl.clone());
        g.res.replay = Some(Box::new(super::replay::ReplaySetup::of(setup, live)));
    }
    (g.res, ctl, view)
}

impl<'a> Game<'a> {
    fn rules(&self) -> &Rules {
        &self.s.rules
    }

    /// Below the NFL: the college expected-points table (update 3, F30).
    fn below_nfl(&self) -> bool {
        self.rules().level != Level::NFL
    }

    /// Late in a half, when the clock waits for the snap after a stoppage:
    /// the last two minutes of each half, and at the NFL the last five of
    /// the game (NFL rule 4-3; NCAA and high school rule 3-3).
    fn clock_late(&self) -> bool {
        let second_half = if self.rules().level == Level::NFL { 300.0 } else { 120.0 };
        (self.quarter == 2 && self.half_left() <= 120.0) || (self.quarter >= 4 && self.half_left() <= second_half)
    }

    fn half_left(&self) -> f64 {
        if self.ot {
            return self.clock;
        }
        self.clock + if self.quarter % 2 == 1 { self.rules().quarter } else { 0.0 }
    }

    fn game_left(&self) -> f64 {
        if self.ot {
            return self.clock;
        }
        self.clock + (4 - self.quarter.min(4)) as f64 * self.rules().quarter
    }

    fn diff(&self, team: usize) -> i32 {
        self.stats.team[team].points - self.stats.team[1 - team].points
    }

    /// What the score and the clock do to habits (tendencies topics 3.7 and
    /// 3.8), team `a` first: down two scores late every player gambles
    /// more, protecting a lead every player protects the ball; a playoff
    /// game or a close one late is a big moment, when old habits come back
    /// (sizes proposed, not confirmed).
    fn mood(&self, a: usize) -> Mood {
        let q = self.rules().quarter;
        let left = self.game_left();
        let late = clamp((q - left) / (0.67 * q), 0.0, 1.0);
        let gamble = |t: usize| match self.diff(t) {
            d if d <= -9 => 2.5 * late,
            d if d < 0 && left < q / 3.0 => 1.5 * late,
            d if d >= 9 => -2.0 * late,
            d if d > 0 && left < q / 3.0 => -1.5 * late,
            _ => 0.0,
        };
        let close = self.diff(a).abs() <= 8 && left < 0.55 * q;
        let stakes = (if self.s.playoff { 0.5 } else { 0.0 }) + if close { 0.4 * late } else { 0.0 };
        Mood {
            gamble: [gamble(a), gamble(1 - a)],
            stakes: [stakes; 2],
            strict: [self.sides[a].team.staff.strictness, self.sides[1 - a].team.staff.strictness],
            line: None,
            snap_y: self.ball_y,
        }
    }

    fn situation(&self) -> Situation {
        let o = self.poss;
        Situation {
            down: nfl_down(self.down),
            togo: self.togo,
            yardline_100: if self.rules().code.american() { goal() - self.x } else { (goal() - self.x).min(99.0) },
            score_diff: self.diff(o),
            quarter: self.quarter,
            half_left: self.half_left(),
            game_left: self.game_left(),
            timeouts: self.sides[o].timeouts,
            opp_timeouts: self.sides[1 - o].timeouts,
            ease: self.sides[o].coach.read.ease,
            concede: self.sides[o].coach.read.concede,
            def_ease: self.sides[1 - o].coach.read.ease,
            def_concede: self.sides[1 - o].coach.read.concede,
            college: self.rules().level != Level::NFL,
            lean: self.rules().code.pass_lean,
        }
    }

    /// Garbage time (`garbage`): at each new drive and each new quarter both
    /// head coaches look at the score and the clock and decide whether the
    /// game is won or lost, and who plays.
    fn review(&mut self) {
        if self.ot {
            return;
        }
        let tag = (self.stats.team[0].drives + self.stats.team[1].drives, self.quarter);
        if Some(tag) == self.reviewed {
            return;
        }
        self.reviewed = Some(tag);
        self.plan_lines();
        let habits = garbage::habits(self.rules().level);
        let len = 4.0 * self.rules().quarter;
        let left = self.game_left();
        for t in 0..2 {
            let z = garbage::comeback_z(self.diff(t), left, len);
            let before = self.sides[t].coach.read;
            let read = self.sides[t].coach.review(z, self.quarter, &habits, &mut self.mind);
            if read.rest_back != before.rest_back || read.step != before.step {
                let benched = garbage::benched(&self.sides[t].depth, self.sides[t].team.players.len(), &read);
                self.sides[t].benched = benched;
            }
            if read.step != before.step {
                let side = &self.sides[t];
                let clock =
                    format!("Q{} {}:{:02}", self.quarter, (self.clock / 60.0) as i32, (self.clock % 60.0) as i32);
                let lead = self.diff(t);
                let what = if read.concede || (before.concede && read.step == 0) { "behind" } else { "ahead" };
                let odds = garbage::comeback_odds(read.seen);
                let why = format!(
                    "{clock}: {} {} {} by {}, as he reads it a comeback is about 1 in {:.0}.",
                    side.team.name,
                    garbage::step_name(read.step),
                    what,
                    lead.abs(),
                    odds
                );
                self.res.decisions.push((self.n + 1, t, why));
            }
        }
    }

    /// Your four-minute plan when this is your team's offense (G4).
    fn four_minute_rule(&self, o: usize) -> Option<live::FourMinute> {
        self.ctl.as_ref().filter(|c| c.team == o).map(|c| c.prep.four).filter(|f| *f != live::FourMinute::Staff)
    }

    /// Your timeout rule (G4): whether a timeout beats the play clock
    /// instead of a delay of game. Keep two for the end spends only the
    /// third, keep one the second too; use them freely spends any but in
    /// the last two minutes of a half. The staff's call takes the five.
    fn timeout_saves_delay(&self, o: usize) -> bool {
        let Some(rule) = self.ctl.as_ref().filter(|c| c.team == o).map(|c| c.prep.timeouts) else { return false };
        let t = self.sides[o].timeouts;
        let late = self.half_left() <= 120.0;
        match rule {
            live::TimeoutRule::Staff => false,
            live::TimeoutRule::KeepTwo => t >= 3 && !late,
            live::TimeoutRule::KeepOne => t >= 2 && !late,
            live::TimeoutRule::Free => t >= 1 && !late,
        }
    }

    /// The return team's habits on a kick, with your plan's return rule
    /// (G18): bring it out, or take the touchback and the fair catch.
    fn return_mood(&self, kicking: usize, rec: usize, punt: bool) -> super::sim::Mood {
        let mut m = self.mood(kicking);
        let t = self.sides[rec].team.prep.teams;
        let rule = if punt { t.pret } else { t.ret };
        m.gamble[1] += match rule {
            1 => 2.5,
            2 => -4.0,
            _ => 0.0,
        };
        m
    }

    /// The return team's look on this kick (G18), from the men on the
    /// field: on a punt, set up for the return with nobody home (a returner
    /// who wants the ball gets returns set up for him); on a kickoff, the
    /// front line turning to run before the ball is kicked. Men short on
    /// discipline for their level leave early more often (proposed, not
    /// confirmed: about one punt in seven and one kickoff in fifteen from
    /// an average unit). Drawn from its own key, so the game's stream stays
    /// as it was; the play log keeps it for the film.
    fn return_look(&self, rec: usize, punt: bool) -> bool {
        use crate::tendencies::{MID, Tend};
        let side = &self.sides[rec];
        let ps = &side.team.players;
        let (ret, front) = if punt {
            let mut used = vec![side.depth.punt_returner];
            let mut front = side.cover_pick(Position::LB, 3, &mut used);
            front.extend(side.cover_pick(Position::DT, 2, &mut used));
            front.extend(side.cover_pick(Position::EDGE, 2, &mut used));
            (Some(side.depth.punt_returner), front)
        } else {
            let on = side.kick_unit(true, self.rules().code.players as usize);
            (on.first().copied(), on.into_iter().skip(1).take(5).collect())
        };
        let ret = ret.filter(|&i| i < ps.len()).map_or(MID, |i| ps[i].tend.get(Tend::ReturnRisk));
        let norm = super::rules::trait_norm(self.s.rules.level, Attr::Discipline);
        let disc = if front.is_empty() {
            norm
        } else {
            front.iter().map(|&i| ps[i].a(Attr::Discipline)).sum::<f64>() / front.len() as f64
        };
        let loose = clamp(1.0 + (norm - disc) / 20.0, 0.5, 1.6);
        let p = if punt { clamp(0.15 * (ret / MID) * loose, 0.04, 0.35) } else { clamp(0.07 * loose, 0.02, 0.16) };
        crate::world::season::key_uniform(self.s.seed ^ 0x4C00_C0DE, self.n as u64 * 2 + punt as u64) < p
    }

    /// Whether the kicking team's staff sees the look and makes the call:
    /// its film study of the return team (the week's tape) sets how ready
    /// it is (proposed, not confirmed: half to nineteen in twenty).
    fn sees_look(&self, team: usize) -> bool {
        let study = self.sides[team].team.staff.film_study;
        crate::world::season::key_uniform(self.s.seed ^ 0x5EE5_100C, self.n as u64)
            < clamp(0.45 + 0.025 * study, 0.5, 0.95)
    }

    /// A fake punt (G18): fourth and four or less between your own 25 and
    /// their 40, not protecting a late lead; only when their return team
    /// shows the look and your staff reads it (`return_look`), or called at
    /// the first chance. Once a game.
    fn fake_punt(&mut self, o: usize, look: bool) -> bool {
        let rule = self.sides[o].team.prep.teams.fake;
        if rule == 0 || self.sides[o].tricks & 1 != 0 || self.ot {
            return false;
        }
        let yl = goal() - self.x;
        let protecting = self.quarter >= 4 && self.diff(o) > 0;
        if self.togo > 4.0 || !(40.0..=75.0).contains(&yl) || protecting {
            return false;
        }
        let go = rule == 2 || (look && self.sees_look(o));
        if go {
            self.sides[o].tricks |= 1;
            self.log(o, "Fake punt: their return team left the middle open.".to_string());
        }
        go
    }

    /// A surprise onside kick (G18): when their front line bails early and
    /// your staff reads it (`return_look`); called, at the first kickoff
    /// after the opening one. Once a game, never in overtime, and only
    /// where the rules let it be kicked.
    fn surprise_onside(&mut self, team: usize, look: bool) -> bool {
        let rule = self.sides[team].team.prep.teams.onside;
        if rule == 0 || self.sides[team].tricks & 2 != 0 || self.ot || !self.onside_allowed(team) {
            return false;
        }
        if self.sides[team].coach.read.ease > 0.0 {
            return false;
        }
        let kicked = self.stats.plays.iter().any(|r| r.kind == PlayType::Kickoff && r.offense == team);
        rule == 2 && kicked || rule == 1 && look && self.sees_look(team)
    }

    /// The plan's "if they do this, we go to that" lines (update 2, G20),
    /// checked at each new drive: a line fires once, when its trigger shows
    /// in this game's snaps.
    fn plan_lines(&mut self) {
        for t in 0..2 {
            if self.sides[t].ifs.is_empty() {
                continue;
            }
            let them = 1 - t;
            let ids: Vec<u32> = self.sides[them].team.players.iter().map(|p| p.id).collect();
            let at = |id: u32| ids.iter().position(|&x| x == id);
            let plays = &self.stats.plays;
            let real = |r: &&PlayRecord| matches!(r.kind, PlayType::Run | PlayType::Pass) && !r.nullified;
            let mut fire: Vec<(u8, live::PlanChange, String)> = Vec::new();
            let ifs = self.sides[t].ifs.clone();
            for &k in &ifs {
                if k >= 6 || self.sides[t].fired & (1 << k) != 0 {
                    continue;
                }
                let mut pc = live::PlanChange::default();
                let why = match k {
                    0 => {
                        let pd: Vec<&PlayRecord> = plays
                            .iter()
                            .filter(|r| r.offense == t)
                            .filter(real)
                            .filter(|r| (r.down >= 3 && r.togo >= 4.0) || (r.down == 2 && r.togo >= 8.0))
                            .collect();
                        let b = pd.iter().filter(|r| r.blitz).count();
                        if pd.len() < 8 || (b as f64) < 0.4 * pd.len() as f64 {
                            continue;
                        }
                        pc.draws = Some(true);
                        format!("they blitzed on {b} of {} passing downs", pd.len())
                    }
                    1 => {
                        let n = plays.iter().filter(|r| r.offense == them && r.scramble).count();
                        if n < 3 {
                            continue;
                        }
                        pc.spy = Some(true);
                        format!("their quarterback has scrambled {n} times")
                    }
                    2 => {
                        let mut by: Vec<(u32, f64)> = Vec::new();
                        for r in plays.iter().filter(|r| r.offense == them && r.complete && !r.nullified) {
                            if let Some(id) = r.target {
                                match by.iter_mut().find(|x| x.0 == id) {
                                    Some(x) => x.1 += r.yards,
                                    None => by.push((id, r.yards)),
                                }
                            }
                        }
                        let Some(&(id, y)) = by.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else { continue };
                        let Some(i) = at(id).filter(|_| y >= 60.0) else { continue };
                        pc.bracket = Some(Some(i));
                        format!("their top receiver has {y:.0} yards")
                    }
                    3 => {
                        let runs: Vec<&PlayRecord> = plays
                            .iter()
                            .filter(|r| r.offense == them && r.kind == PlayType::Run && !r.nullified)
                            .collect();
                        let ypc = runs.iter().map(|r| r.yards).sum::<f64>() / runs.len().max(1) as f64;
                        if runs.len() < 10 || ypc < 5.0 {
                            continue;
                        }
                        format!("they are running for {ypc:.1} a carry")
                    }
                    4 => {
                        let ours: Vec<&PlayRecord> = plays.iter().filter(|r| r.offense == t).filter(real).collect();
                        let light = ours.iter().filter(|r| r.box_count > 0 && r.box_count <= 6).count();
                        if ours.len() < 12 || (light as f64) < 0.6 * ours.len() as f64 {
                            continue;
                        }
                        format!("they have shown six or fewer in the box on {light} of {} snaps", ours.len())
                    }
                    _ => {
                        let mut by: Vec<(u32, u32)> = Vec::new();
                        for r in plays.iter().filter(|r| r.offense == t && r.complete && r.yards >= 15.0) {
                            if let Some(d) = r.defender {
                                match by.iter_mut().find(|x| x.0 == d) {
                                    Some(x) => x.1 += 1,
                                    None => by.push((d, 1)),
                                }
                            }
                        }
                        let found = by.iter().filter(|x| x.1 >= 2).find_map(|x| {
                            at(x.0).filter(|&i| self.sides[them].team.players[i].position == Position::CB)
                        });
                        let Some(i) = found else { continue };
                        pc.target_cb = Some(Some(i));
                        "one of their corners has given up two catches of 15 yards".to_string()
                    }
                };
                fire.push((k, pc, why));
            }
            for (k, pc, why) in fire {
                self.sides[t].fired |= 1 << k;
                match k {
                    3 => self.sides[t].mem.plan.box_up = true,
                    4 => self.sides[t].mem.plan.run_more = 0.08,
                    _ => {}
                }
                let then = crate::game::roster::IFS[k as usize].1.to_lowercase();
                self.log(t, format!("The plan's line: {why}, so {then}."));
                if pc != live::PlanChange::default() {
                    self.apply_plan(t, pc);
                }
            }
        }
    }

    /// Rain, snow or a wind of 15 miles an hour or more: the call sheets'
    /// bad-weather versions (update 2, G13).
    fn bad_weather(&self) -> bool {
        self.s.cond.rain || self.s.cond.snow || self.s.cond.wind >= 15.0
    }

    fn wind_along(&self, team: usize) -> f64 {
        // Teams switch ends each quarter.
        let toward = if (self.quarter % 2 == 1) == (team == 0) { 1.0 } else { -1.0 };
        let flip = if self.flipped { -1.0 } else { 1.0 };
        self.s.cond.wind_along * toward * flip
    }

    fn home_crowd(&self, team: usize) -> f64 {
        // A week practising with the speakers on gets a road team used to it.
        if self.s.neutral || team == 0 { 0.0 } else { self.s.cond.crowd * self.sides[team].team.prep.noise }
    }

    fn add_points(&mut self, team: usize, pts: i32) {
        self.stats.team[team].points += pts;
        let q = self.stats.quarters.len() - 1;
        self.stats.quarters[q][team] += pts;
    }

    fn new_record(&mut self, kind: PlayType) -> PlayRecord {
        self.n += 1;
        let mut r = PlayRecord::new(self.n, self.quarter, self.clock, self.poss, kind);
        r.down = self.down;
        r.togo = self.togo;
        r.yardline_100 = goal() - self.x;
        r.score = [self.stats.team[0].points, self.stats.team[1].points];
        r.ep_before = ep_at(self.below_nfl(), self.down.max(1), self.togo, goal() - self.x);
        r.wp_before = wp_at(self.diff(self.poss), self.game_left(), goal() - self.x);
        r.timeouts = Some([self.sides[0].timeouts, self.sides[1].timeouts]);
        r.after_turnover = self.after_turnover;
        r.garbage = [self.sides[0].coach.read.step, self.sides[1].coach.read.step];
        r.hash = match super::sheet::hash_of(self.ball_y) {
            0 => 3,
            h => h,
        };
        r.hurry = self.no_huddle;
        r
    }

    /// Run the clock: a play's time, then the time to the next snap.
    fn tick(&mut self, secs: f64) {
        if self.trying || self.ot && !self.ot_timed() {
            return;
        }
        let before = self.clock;
        self.clock -= secs;
        // Two-minute warning (NFL; the CFL's comes at three minutes).
        let warning = if self.rules().code.american() { 120.0 } else { self.rules().code.warning };
        if self.rules().two_minute_warning
            && warning > 0.0
            && (self.quarter == 2 || self.quarter == 4)
            && before > warning
            && self.clock <= warning
            && !self.two_min_warned
        {
            self.two_min_warned = true;
            self.clock = warning;
            self.running = false;
            self.recover(0.02);
            // The CFL: only one timeout may be used after the three-minute
            // warning of the fourth quarter.
            if self.quarter == 4 && self.rules().code.timeouts_per_game {
                for s in self.sides.iter_mut() {
                    s.timeouts = s.timeouts.min(1);
                }
            }
        }
    }

    /// Mercy rule (high school): with a big enough lead in the second half the
    /// clock keeps running through incompletions, first downs and out of
    /// bounds; only scores, turnovers and timeouts stop it.
    fn mercy_running(&self) -> bool {
        self.rules().mercy.is_some_and(|(lead, q)| self.quarter >= q && self.diff(0).abs() >= lead)
    }

    fn recover(&mut self, amt: f64) {
        for s in self.sides.iter_mut() {
            for e in s.energy.iter_mut() {
                *e = (*e + amt).min(1.0);
            }
        }
    }

    fn end_of_period(&mut self) {
        if self.over || self.clock > 0.0 {
            return;
        }
        if matches!(self.next, Next::Pat(_)) {
            return;
        }
        if self.ot {
            // A timed overtime period ends: a tie in the regular season,
            // another period in the playoffs.
            if self.ot_timed() {
                if self.diff(0) != 0 || !self.s.playoff {
                    self.over = true;
                } else {
                    self.clock = self.ot_period();
                }
            }
            return;
        }
        match self.quarter {
            2 => {
                // Halftime (topic 9): one stop with no clock.
                if let Some(Input::Halftime(pc)) = self.ask(Ask::Halftime, false) {
                    self.apply_plan(self.ctl_team(), pc);
                }
                if self.probe == Some(Ask::Halftime) {
                    let p = self.pending(Ask::Halftime);
                    self.hold(p);
                    return;
                }
                self.quarter = 3;
                self.clock = self.rules().quarter;
                self.two_min_warned = false;
                self.stats.timeouts_left[0] = [self.sides[0].timeouts, self.sides[1].timeouts];
                // The CFL's timeouts are a game's, not a half's (from 2013).
                let per_game = self.s.rules.code.timeouts_per_game;
                for s in self.sides.iter_mut() {
                    if !per_game {
                        s.timeouts = self.s.rules.timeouts;
                    }
                    for e in s.energy.iter_mut() {
                        *e = (*e + 0.35).min(1.0);
                    }
                }
                self.stats.quarters.push([0, 0]);
                let from = self.rules().kickoff_from;
                self.next = Next::Kickoff { team: self.receive_first, from, onside: false };
            }
            4 => {
                self.stats.timeouts_left[1] = [self.sides[0].timeouts, self.sides[1].timeouts];
                if self.diff(0) != 0 {
                    self.over = true;
                } else {
                    self.start_overtime();
                }
            }
            _ => {
                self.quarter += 1;
                self.clock = self.rules().quarter;
                self.recover(0.05);
                self.stats.quarters.push([0, 0]);
            }
        }
    }

    fn start_overtime(&mut self) {
        self.ot = true;
        self.stats.overtime = true;
        self.quarter = 5;
        self.stats.quarters.push([0, 0]);
        self.recover(0.15);
        let drawn = if self.rng.chance(0.5) { 0 } else { 1 };
        let first = self.ot_toss(drawn);
        if self.paused {
            return;
        }
        self.ot_first = first;
        self.ot_poss = [0, 0];
        match self.rules().overtime {
            Overtime::Shootout { rounds, from } => {
                self.clock = 1.0e9;
                self.shootout(first, rounds, from as f64);
            }
            _ if self.ot_timed() => {
                self.clock = self.ot_period();
                for s in self.sides.iter_mut() {
                    s.timeouts = 2;
                }
                let from = self.rules().kickoff_from;
                self.next = Next::Kickoff { team: 1 - first, from, onside: false };
            }
            _ => {
                self.clock = 1.0e9;
                self.ot_round = 1;
                self.ot_series(first);
            }
        }
    }

    /// Overtime that runs on a clock with kickoffs: the NFL's period, the
    /// old sudden death and arena football's period. The others are
    /// series from a spot, or a shootout.
    fn ot_timed(&self) -> bool {
        matches!(
            self.rules().overtime,
            Overtime::NflPeriod | Overtime::NflModified { .. } | Overtime::SuddenDeath | Overtime::Arena
        )
    }

    /// A timed overtime's possessions start with a kickoff.
    fn ot_kickoffs(&self) -> bool {
        self.ot_timed()
    }

    /// The length of a timed overtime period: ten minutes in today's NFL
    /// regular season, fifteen in its playoffs (update 3, F25), under the
    /// old rules and in arena football.
    fn ot_period(&self) -> f64 {
        match self.rules().overtime {
            _ if self.s.playoff => 900.0,
            Overtime::NflPeriod => 600.0,
            Overtime::NflModified { minutes } => minutes as f64 * 60.0,
            _ => 900.0,
        }
    }

    /// An overtime series from its spot: the opponent's 10 in high school
    /// and the AAF, its 25 in college, its 20 in the 2001 XFL, its 35 in
    /// the CFL.
    fn ot_series(&mut self, team: usize) {
        self.poss = team;
        self.drive_start_poss = team;
        let from = match self.rules().overtime {
            Overtime::HighSchool | Overtime::Aaf => 10.0,
            Overtime::Xfl2001 => 20.0,
            Overtime::Cfl { .. } => 35.0,
            _ => 25.0,
        };
        self.x = goal() - from;
        self.down = 1;
        self.togo = 10.0f64.min(from);
        self.ball_y = mid();
        self.next = Next::Scrimmage;
        for s in self.sides.iter_mut() {
            s.timeouts = 1;
        }
    }

    /// The other leagues' shootout (the XFL of 2020 and 2023, the USFL of
    /// 2022-23, the UFL): the teams take turns at plays from `from` yards,
    /// two points each; after `rounds` rounds the one with more wins, then
    /// round by round until one makes it and the other doesn't. It ends as
    /// soon as one side can't catch up.
    fn shootout(&mut self, first: usize, rounds: u8, from: f64) {
        let order = [first, 1 - first];
        let mut made = [0u32; 2];
        let mut taken = [0u32; 2];
        let rounds = rounds as u32;
        for round in 1..=30u32 {
            for &team in &order {
                self.poss = team;
                self.drive_start_poss = team;
                self.x = goal() - from;
                self.down = 1;
                self.togo = from;
                self.ball_y = mid();
                self.trying = true;
                let mut r = self.snap_play(true);
                self.trying = false;
                if self.paused {
                    return;
                }
                r.kind = PlayType::TwoPoint;
                let good = r.touchdown && !r.turnover;
                taken[team] += 1;
                if good {
                    made[team] += 1;
                    self.add_points(team, 2);
                }
                r.why =
                    format!("Overtime shootout, round {round}: {}. {}", if good { "good" } else { "no good" }, r.why);
                self.stats.plays.push(r);
                // Decided inside the regulation rounds once one side can't
                // catch up with the tries it has left.
                if round <= rounds {
                    let left = |t: usize| rounds - taken[t];
                    if made[0] + left(0) < made[1] || made[1] + left(1) < made[0] {
                        self.over = true;
                        return;
                    }
                }
            }
            if round >= rounds && made[0] != made[1] {
                self.over = true;
                return;
            }
        }
        self.over = true;
    }

    /// A possession ended: in overtime decide what comes next.
    fn possession_over(&mut self, scored: bool) {
        if !self.ot {
            return;
        }
        let team = self.drive_start_poss;
        self.ot_poss[team] += 1;
        match self.rules().overtime {
            Overtime::NflPeriod | Overtime::Arena => {
                // Both teams get the ball, then sudden death.
                if self.ot_poss[0] >= 1 && self.ot_poss[1] >= 1 && self.diff(0) != 0 {
                    self.over = true;
                }
            }
            Overtime::SuddenDeath => {
                // The first score wins.
                if scored && self.diff(0) != 0 {
                    self.over = true;
                }
            }
            Overtime::NflModified { .. } => {
                // A first-possession field goal gives the other side the
                // ball; any other score, or any score after that, wins.
                let first = self.ot_poss[0] + self.ot_poss[1] == 1;
                let diff = self.diff(0);
                if diff != 0 && (!first || diff.abs() != 3) {
                    self.over = true;
                }
            }
            Overtime::Shootout { .. } => {}
            _ => {
                // The CFL and the AAF end a regular-season game tied after
                // two rounds (one in the AAF).
                let limit = match self.rules().overtime {
                    Overtime::Cfl { .. } => Some(2),
                    Overtime::Aaf => Some(1),
                    _ => None,
                };
                if self.ot_poss[0] == self.ot_poss[1]
                    && self.diff(0) == 0
                    && !self.s.playoff
                    && limit.is_some_and(|l| self.ot_round >= l)
                {
                    self.over = true;
                    return;
                }
                if self.ot_poss[0] == self.ot_poss[1] {
                    if self.diff(0) != 0 {
                        self.over = true;
                        return;
                    }
                    self.ot_round += 1;
                    let first = if self.ot_round % 2 == 1 { self.ot_first } else { 1 - self.ot_first };
                    // College's two-point shootout by the era's rule (update
                    // 3, F26): one try a side a round, from the try's line.
                    if let Some((_, shootout)) = self.rules().college_ot()
                        && self.ot_round >= shootout as u32
                    {
                        let from = self.rules().two_point_from;
                        self.shootout(first, 1, from);
                        return;
                    }
                    self.ot_series(first);
                } else {
                    self.ot_series(1 - team);
                }
            }
        }
    }

    // ------------------------------------------------------------ kickoffs

    fn kick_unit(&self, team: usize, receiving: bool) -> Vec<usize> {
        self.sides[team].kick_unit(receiving, self.rules().code.players as usize)
    }

    fn kickoff(&mut self, team: usize, from: f64, onside: bool) {
        let rec = 1 - team;
        let kick_on = self.kick_unit(team, false);
        let ret_on = self.kick_unit(rec, true);
        // Onside kicks: trailing late, or a surprise from an aggressive coach.
        let late = self.quarter >= 4 && self.game_left() < 180.0;
        let behind = self.diff(team) < 0 && self.diff(team) >= -16;
        let read = self.sides[team].coach.read;
        let onside = onside
            || (late && behind && !read.concede)
            || (!self.ot
                && read.ease == 0.0
                && self.rng.chance(0.002 * self.sides[team].team.staff.offense.aggressiveness / 10.0));
        // Your kickoff: an onside kick is yours to call (topic 6), within
        // the rules (the NFL's 2024 kickoff: declared, and only when
        // trailing; in 2024 only in the fourth quarter).
        let mut onside = onside;
        // A surprise onside kick in your plan (update 2, G18): only when the
        // return team's front line bails early, or called once. Spent and
        // logged once the kick really goes onside (your answer first).
        let look = self.return_look(rec, false);
        let surprise = !onside && !late && self.surprise_onside(team, look);
        if surprise {
            onside = true;
        }
        if self.user_is(team) {
            match self.ask(Ask::Onside, false) {
                Some(Input::Onside(b)) => onside = b && self.onside_allowed(team),
                Some(_) => {}
                None if self.probe == Some(Ask::Onside) => {
                    let mut p = self.pending(Ask::Onside);
                    p.staff_onside = onside;
                    if !self.onside_allowed(team) {
                        p.advice.push("Onside kick not allowed now: under the NFL's kickoff rules it must be declared, and only by a team that trails.".into());
                    }
                    self.hold(p);
                    return;
                }
                None => {}
            }
        }
        if surprise && onside {
            self.sides[team].tricks |= 2;
            self.log(team, "A surprise onside kick: their front line turned and ran early.".to_string());
        }
        // The other leagues' fourth-down play in place of an onside kick
        // (X7): the kicking team keeps the ball if it makes `togo` from its
        // own `from` (a team trailing by `behind` or more, or anyone in the
        // last `minutes`).
        let code = self.rules().code;
        if let Some(alt) = code.onside_alt
            && onside
            && !self.ot
            && (-self.diff(team) >= alt.behind || self.game_left() <= alt.minutes * 60.0)
        {
            self.running = false;
            self.after_turnover = false;
            self.must_go = true;
            self.log(
                team,
                format!("Fourth and {:.0} from its own {:.0} in place of an onside kick.", alt.togo, alt.from),
            );
            self.take_ball(team, alt.from, self.rules().downs(), alt.togo);
            return;
        }
        // No kickoffs (the AAF): the ball at the receiving team's 25.
        if !code.kickoffs {
            self.running = false;
            self.after_turnover = false;
            self.take_ball(rec, code.no_kick_start, 1, 10.0);
            return;
        }
        let mut r = self.new_record(PlayType::Kickoff);
        r.offense = team;
        r.look = look;
        let kt = KickTeams {
            kick: &self.sides[team].team.players,
            ret: &self.sides[rec].team.players,
            kick_on: kick_on.clone(),
            ret_on: ret_on.clone(),
            snapper: self.sides[team].depth.snapper,
            holder: self.sides[team].depth.holder,
            energy: [&self.sides[team].energy, &self.sides[rec].energy],
            hurt: [&self.sides[team].hurt, &self.sides[rec].hurt],
            mood: self.return_mood(team, rec, false),
            kick_deep: self.sides[team].mem.plan.vs.kick_deep,
            style: if self.rules().nfl_kickoff { 0 } else { self.sides[team].team.prep.teams.kick },
            surprise,
        };
        let mut rules = self.s.rules;
        rules.kickoff_from = from;
        let along = self.wind_along(team);
        let kr = kicking::kickoff(
            &kt,
            &rules,
            &self.s.cond,
            &self.crew,
            along,
            onside,
            1.0,
            self.recording(),
            &mut self.rng,
        );
        // The return's tracking, for the views (the returning team is side 0).
        if let Some(f) = &kr.frames {
            self.res.frames.push((r.n, f.clone()));
        }
        r.kicker = Some(kt.kick[kick_on[0]].id);
        r.on_field = [kick_on.iter().map(|&i| kt.kick[i].id).collect(), ret_on.iter().map(|&i| kt.ret[i].id).collect()];
        r.kick_distance = kr.distance;
        r.touchback = kr.touchback;
        r.returner = kr.returner;
        r.return_yards = kr.return_yards;
        r.duration = kr.duration;
        r.tacklers = kr.notes.tacklers.clone();
        r.missed_tackles = kr.notes.missed.clone();
        r.injuries = kr.notes.injuries.clone();
        r.tend = kr.notes.tend;
        r.tend.start = Some(kr.spot);
        r.call = if onside { "onside kick".into() } else { "kickoff".into() };
        // On a return the returning team is side 0 and the kicking team side 1.
        self.apply_injuries(&kr.notes.injuries, [rec, team]);
        let elapsed = if kr.returned || onside { kr.duration } else { 0.0 };
        if self.clock > 0.0 && !(self.ot && !self.ot_timed()) {
            self.tick(elapsed);
        }
        self.running = false;
        self.after_turnover = false;
        let kick_x = if onside { goal() - kr.spot } else { kr.spot };
        if kr.recovered_by_kicking && kick_x >= goal() {
            // Fumbled in the returner's own end zone and recovered there by
            // the kicking team: their touchdown.
            self.poss = team;
            r.touchdown = true;
            r.turnover = true;
            r.why = "Fumbled in the end zone and recovered by the kicking team for a touchdown.".into();
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.add_points(team, 6);
            self.next = Next::Pat(team);
            self.drive_start_poss = team;
            return;
        } else if kr.recovered_by_kicking {
            self.poss = team;
            self.x = kick_x.clamp(1.0, goal() - 1.0);
            r.turnover = true;
            r.why = if onside {
                "Onside kick recovered by the kicking team.".into()
            } else {
                "Muffed return recovered by the kicking team.".into()
            };
        } else if kr.return_td {
            self.poss = rec;
            r.touchdown = true;
            r.offense = team;
            r.why = format!("Kick returned for a touchdown by #{}.", kr.returner.unwrap_or(0));
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.add_points(rec, 6);
            self.next = Next::Pat(rec);
            self.drive_start_poss = rec;
            return;
        } else {
            self.poss = rec;
            self.x = kr.spot.clamp(1.0, goal() - 1.0);
            // The rouge (CFL): a kickoff not brought out of the end zone is
            // a single for the kicking team, the ball at the 35; under the
            // 2026 rule only one that settles there scores, and one through
            // the end zone comes out to the 40.
            let single = kr.touchback && self.rouge(team, &mut r);
            r.why = if single {
                format!(
                    "Single: the kickoff isn't brought out of the end zone. {} ball at its {:.0}.",
                    "The receiving team's", self.x
                )
            } else if kr.touchback && code.rouge != Rouge::None {
                format!("The kickoff goes through the end zone: no single, the ball at the {:.0}.", self.x)
            } else if kr.touchback {
                "Touchback.".to_string()
            } else if onside {
                "Onside kick recovered by the receiving team.".into()
            } else {
                format!(
                    "Returned {:.0} {} by #{}.",
                    kr.return_yards,
                    crate::world::folk::noun(kr.return_yards.round_ties_even() as i64, "yard"),
                    kr.returner.unwrap_or(0)
                )
            };
        }
        self.down = 1;
        self.togo = 10.0f64.min(goal() - self.x);
        self.ball_y = mid();
        self.next = Next::Scrimmage;
        self.drive_start_poss = self.poss;
        self.stats.team[self.poss].drives += 1;
        self.stats.team[self.poss].start_sum += self.x;
        self.stats.book(&r);
        self.stats.plays.push(r);
    }

    /// A defensive call made legal under the book (X7): no more rushers
    /// than the book allows (the AAF five, arena four of its eight), and
    /// arena football's man coverage (a zone shell becomes the man look
    /// with the same safeties: two deep or one).
    fn legal_defense(&self, mut dc: DefCall) -> DefCall {
        let code = self.rules().code;
        if code.man_only && !dc.coverage.is_man() && dc.coverage != Coverage::Cover0 {
            dc.coverage = if dc.coverage.high() >= 2 { Coverage::Cover2Man } else { Coverage::Cover1 };
            dc.disguise = false;
        }
        let line = if code.players <= 8 { 3 } else { 4 };
        while line + dc.pressure.extra() > code.max_rushers {
            dc.pressure = match dc.pressure {
                Pressure::DoubleA => Pressure::LbBlitz,
                _ => Pressure::Four,
            };
        }
        dc
    }

    /// A team takes the ball without a kick (the AAF's start, the play in
    /// place of an onside kick): its own `x`, on `down` and `togo`.
    fn take_ball(&mut self, team: usize, x: f64, down: u8, togo: f64) {
        self.poss = team;
        self.x = x;
        self.down = down;
        self.togo = togo.min(goal() - x);
        self.ball_y = mid();
        self.next = Next::Scrimmage;
        self.drive_start_poss = team;
        self.stats.team[team].drives += 1;
        self.stats.team[team].start_sum += x;
    }

    /// The rouge (CFL) on a kick the receiving team doesn't bring out of
    /// its end zone: one point to the kicking team and the ball at the
    /// receiving team's 35; from 2026 only a kick that settles in the end
    /// zone scores (about three in five of them: the rest go through or
    /// out of it untouched, and the ball comes out to the 40). Returns
    /// whether a single was scored; sets the spot either way.
    fn rouge(&mut self, kicking: usize, r: &mut PlayRecord) -> bool {
        let code = self.rules().code;
        let single = match code.rouge {
            Rouge::None => return false,
            Rouge::Full => true,
            Rouge::Settled => self.rng.chance(0.6),
        };
        if single {
            self.add_points(kicking, 1);
            r.single = true;
            self.x = code.single_spot;
        } else {
            self.x = code.no_single_spot;
        }
        single
    }

    // ------------------------------------------------------------ PAT

    fn pat(&mut self, team: usize) {
        self.poss = team;
        let code = self.rules().code;
        // XFL 2001 overtime: a touchdown is all a series can score.
        if self.ot && self.rules().overtime == Overtime::Xfl2001 {
            self.possession_over(true);
            return;
        }
        let staff = self.sides[team].team.staff;
        let kicker = &self.sides[team].team.players[self.sides[team].fg_kicker()];
        let xp_make =
            clamp(0.55 + 0.005 * kicker.a(Attr::ShortKickAccuracy) - 0.002 * (self.rules().xp_from - 3.0), 0.2, 0.99);
        let diff = self.diff(team);
        let forced_two = self.ot
            && match self.rules().overtime {
                Overtime::College | Overtime::CollegeEra { .. } => {
                    self.rules().college_ot().is_some_and(|(two, _)| self.ot_round >= two as u32)
                }
                Overtime::Cfl { two_from } => code.year >= two_from,
                Overtime::Aaf => true,
                _ => false,
            };
        // The other leagues' tries (X7): a kick where the book has one, and
        // plays for one, two or three points from their own spots (the
        // XFL's and the UFL's). The American game has the kick and two.
        let kick_allowed = code.american() || code.try_kick.is_some();
        let one_from = if code.american() { None } else { code.try_plays[0] };
        let three_from = if code.american() { None } else { code.try_plays[2] };
        // Without a kick, the safe try is the play for one: about 60% from
        // the 2 in the XFL of 2020 (and the two-point play about 45% from
        // the 5, the three-point play a third from the 10).
        let safe = if kick_allowed { xp_make } else { 0.6 };
        // Up big, nobody goes for two (it reads as running up the score).
        let easing = self.sides[team].coach.read.ease >= 0.5;
        let mut two = forced_two
            || (!easing && calling::go_for_two(&staff, diff, self.quarter, self.game_left(), safe, &mut self.rng));
        let three = !forced_two
            && !easing
            && three_from.is_some()
            && calling::go_for_three(&staff, diff, self.quarter, self.game_left(), &mut self.rng);
        if three {
            two = false;
        }
        // Your two-point chart, on your touchdowns.
        if !forced_two && let Some(c) = self.ctl.as_ref().filter(|c| c.team == team) {
            match c.prep.two {
                live::TwoChart::Staff => {}
                live::TwoChart::Kick => two = false,
                live::TwoChart::Chart => two = !self.ot && self.quarter >= 4 && live::chart_says_two(diff),
            }
        }
        // Your touchdown: kick or go for two (topic 6).
        self.snap_input = None;
        self.mark = (false, false, false, String::new());
        if self.user_is(team) {
            match self.ask(Ask::Pat, false) {
                Some(Input::Pat { two: t, offense }) => {
                    two = t || forced_two;
                    self.snap_input = Some(Input::Pat { two, offense });
                }
                Some(_) => {}
                None if self.probe == Some(Ask::Pat) && !two => {
                    // The staff kicks: the two-point call it would make,
                    // from the same box.
                    self.x = goal() - self.rules().two_point_from;
                    self.down = 1;
                    self.togo = self.rules().two_point_from;
                    let mut p = self.pending(Ask::Pat);
                    p.staff_two = false;
                    p.off_options = self.go_options();
                    self.hold(p);
                    return;
                }
                None => {}
            }
        }
        // The play for the try: three, two, or one where there is no kick.
        let play = if three {
            three_from.map(|f| (3, f))
        } else if two {
            Some((2, self.rules().two_point_from))
        } else if !kick_allowed {
            one_from.map(|f| (1, f)).or(Some((2, self.rules().two_point_from)))
        } else {
            None
        };
        if let Some((pts, from)) = play {
            self.x = goal() - from;
            self.down = 1;
            self.togo = from;
            self.trying = true;
            let res = self.snap_play(true);
            self.trying = false;
            if self.paused {
                return;
            }
            let mut r = res;
            r.kind = PlayType::TwoPoint;
            let good = r.touchdown && !r.turnover;
            if good {
                self.add_points(team, pts);
            }
            let what = match pts {
                1 => "One-point try",
                3 => "Three-point try",
                _ => "Two-point try",
            };
            r.why = format!("{what} {}. {}", if good { "good" } else { "failed" }, r.why);
            self.stats.plays.push(r);
        } else {
            let mut r = self.new_record(PlayType::ExtraPoint);
            let dist = self.rules().xp_from + kick_add();
            let fg = self.kick(team, dist, 0.1, &mut r);
            if fg {
                self.add_points(team, 1);
            }
            self.stats.book(&r);
            self.stats.plays.push(r);
        }
        if self.ot {
            self.possession_over(true);
            if self.over || !self.ot_kickoffs() {
                return;
            }
        }
        let from = self.rules().kickoff_from - std::mem::take(&mut self.kick_back);
        self.next = Next::Kickoff { team, from, onside: false };
    }

    /// A field goal or extra point try. Returns whether it was good.
    fn kick(&mut self, team: usize, dist: f64, pressure: f64, r: &mut PlayRecord) -> bool {
        let side = &self.sides[team];
        let ps = &side.team.players;
        let k = side.fg_kicker();
        let sn = side.snapper();
        let h = side.holder(k, sn);
        let snapper = &ps[sn];
        let holder = &ps[h];
        // The unit is 11: the kicker, the holder and a nine-man wall with
        // the long snapper in the middle (guards, tackles, ends and wings:
        // three interior linemen, two tackles and three tight ends).
        let mut used = vec![k, sn, h];
        let protect_on: Vec<usize> = [sn]
            .into_iter()
            .chain(side.pick(Position::IOL, 3, &mut used))
            .chain(side.pick(Position::OT, 2, &mut used))
            .chain(side.pick(Position::TE, 3, &mut used))
            .collect();
        // Your field goal unit (section 6b): your wall around the snapper.
        let protect_on: Vec<usize> = {
            let wall = side.with_package(8, &[sn], protect_on);
            wall.into_iter().filter(|&i| i != k && i != h).take(9).collect()
        };
        let opp = &self.sides[1 - team];
        let mut used2 = Vec::new();
        let rush_on: Vec<usize> = opp
            .pick(Position::DT, 3, &mut used2)
            .into_iter()
            .chain(opp.pick(Position::EDGE, 2, &mut used2))
            .chain(opp.pick(Position::LB, 2, &mut used2))
            .collect();
        let protect = kicking::kick_unit_strength(ps, &protect_on, false);
        let rush = kicking::kick_unit_strength(&opp.team.players, &rush_on, true);
        let along = self.wind_along(team);
        let fg = kicking::field_goal(
            &ps[k],
            snapper,
            holder,
            rush,
            protect,
            dist,
            pressure,
            &self.s.cond,
            along,
            self.s.rules.level,
            &mut self.rng,
        );
        r.kicker = Some(ps[k].id);
        r.kick_distance = dist;
        r.kick_good = fg.made;
        r.blocked = fg.blocked;
        r.kick_pressure = pressure;
        r.why = fg.why.clone();
        r.on_field = [
            protect_on.iter().map(|&i| ps[i].id).chain([ps[h].id, ps[k].id]).collect(),
            rush_on.iter().map(|&i| opp.team.players[i].id).collect(),
        ];
        r.duration = 3.0;
        fg.made
    }

    // ------------------------------------------------------------ scrimmage

    fn scrimmage(&mut self) {
        self.review();
        // The fourth-down play in place of an onside kick (X7) is this snap only.
        let must_go = std::mem::take(&mut self.must_go);
        self.mark = (false, false, false, String::new());
        // Your offense's snap (game day topics 4-6): a timeout, a word to
        // the coordinator, the tempo, then the call; or the staff's.
        self.snap_input = None;
        if self.user_is(self.poss) {
            let mut again = false;
            loop {
                match self.ask(Ask::Snap, again) {
                    Some(Input::Timeout) => self.user_timeout(self.poss),
                    Some(Input::Plan(pc)) => self.apply_plan(self.poss, pc),
                    Some(Input::Tempo(t)) => {
                        self.tempo = t;
                        let name = match t {
                            Tempo::Normal => "normal tempo",
                            Tempo::Hurry => "the no-huddle",
                            Tempo::Milk => "milking the clock",
                        };
                        self.log(self.poss, format!("Your offense goes to {name}."));
                    }
                    other => {
                        self.snap_input = other;
                        break;
                    }
                }
                again = true;
            }
        }
        // Your own choice on this snap: it replaces the staff's automatic
        // ones (the kick at the end of a half, the kneel, the spike).
        let (yours_fourth, yours_call) = match &self.snap_input {
            Some(Input::Call { fourth, offense, .. }) => (*fourth, *offense),
            _ => (None, None),
        };
        let own = yours_fourth.is_some() || yours_call.is_some();
        if let Some(f) = yours_fourth.filter(|f| *f != FourthDown::Go) {
            let yl = goal() - self.x;
            if f == FourthDown::Punt {
                self.punt();
            } else {
                self.field_goal(yl + kick_add());
            }
            return;
        }
        let o = self.poss;
        let sit = self.situation();
        let staff = self.sides[o].team.staff;
        let yl = goal() - self.x;
        // End of half: kick it if in range, else take a shot.
        let fg_dist = yl + kick_add();
        let kicker_range = self.kicker_range(o);
        let half_end = !self.ot && (self.quarter == 2 || self.quarter == 4) && self.half_left() <= 6.0;
        if !own && half_end && fg_dist <= kicker_range && (self.quarter == 2 || self.diff(o) <= 0 && self.diff(o) >= -3)
        {
            if self.probe == Some(Ask::Snap) {
                let mut p = self.pending(Ask::Snap);
                p.staff_fourth = Some(FourthDown::FieldGoal);
                p.advice.push("Time is nearly out: your staff sends the field goal unit on.".into());
                p.off_options = self.go_options();
                self.hold(p);
                return;
            }
            self.field_goal(fg_dist);
            return;
        }
        // Kneel to run out the clock.
        if !own && !self.ot && self.should_kneel() {
            let call = OffCall::new(
                Formation::plain(Personnel::P12, Backfield::UnderCenter, 1.0),
                PlayKind::Kneel,
                1.0,
                false,
            );
            if self.probe == Some(Ask::Snap) {
                let mut p = self.pending(Ask::Snap);
                p.staff_off = Some(call);
                p.advice.push("Victory formation: the clock can be run out from here.".into());
                p.off_options = self.go_options();
                self.hold(p);
                return;
            }
            let r = self.run_scrimmage_call(call, None);
            self.stats.plays.push(r);
            return;
        }
        // Fourth down.
        if self.down == self.rules().downs() && !(self.ot && self.rules().overtime != Overtime::NflPeriod && false) {
            let make = self.fg_make_estimate(o, fg_dist);
            let mut choice = calling::fourth_down(&staff, &sit, make, &mut self.rng);
            if self.ot && !self.ot_timed() && choice == FourthDown::Punt {
                choice = if make > 0.35 { FourthDown::FieldGoal } else { FourthDown::Go };
            }
            if choice == FourthDown::FieldGoal && fg_dist > kicker_range + 3.0 {
                choice = if yl < 40.0 { FourthDown::Go } else { FourthDown::Punt };
            }
            choice = self.garbage_fourth(choice, yl);
            if let Some(c) = self.chart_fourth(o, make, fg_dist <= kicker_range + 3.0) {
                choice = c;
            }
            self.fourth_note = Some(self.fourth_model(o, if fg_dist <= kicker_range + 3.0 { make } else { 0.0 }));
            if self.probe == Some(Ask::Snap) && choice != FourthDown::Go {
                let mut p = self.pending(Ask::Snap);
                p.staff_fourth = Some(choice);
                // If you go for it instead: the plays your coordinator
                // would call here.
                p.off_options = self.go_options();
                self.hold(p);
                return;
            }
            if own {
                choice = FourthDown::Go;
            }
            // The book's limits (X7): no punting in arena football; none from
            // the other side of midfield in the UFL from 2026 outside each
            // half's last two minutes; only a touchdown counts in the AAF's
            // and the 2001 XFL's overtime; the play in place of an onside
            // kick is played.
            let code = self.rules().code;
            if choice == FourthDown::Punt
                && (!code.punting || code.no_punt_past_50 && yl < goal() / 2.0 && self.half_left() > 120.0)
            {
                choice =
                    if make > 0.35 && fg_dist <= kicker_range + 3.0 { FourthDown::FieldGoal } else { FourthDown::Go };
            }
            // Arena football goes for it on fourth and short unless the kick
            // through its narrow posts is close to sure.
            if !code.punting && choice == FourthDown::FieldGoal && self.togo <= 4.0 && make < 0.85 && !self.ot {
                choice = FourthDown::Go;
            }
            if must_go || self.ot && matches!(self.rules().overtime, Overtime::Aaf | Overtime::Xfl2001) {
                choice = FourthDown::Go;
            }
            match choice {
                FourthDown::Punt => {
                    // A fake punt in your plan (update 2, G18), when their
                    // return team shows the look.
                    let look = self.return_look(1 - o, true);
                    if self.fake_punt(o, look) {
                        let r = self.fake_punt_play(look);
                        if self.paused {
                            return;
                        }
                        self.stats.plays.push(r);
                        return;
                    }
                    self.kick_look = look;
                    self.punt();
                    return;
                }
                FourthDown::FieldGoal => {
                    self.field_goal(fg_dist);
                    return;
                }
                FourthDown::Go => {}
            }
        }
        // Spike to stop the clock.
        if !own
            && self.running
            && sit.hurry()
            && self.half_left() < 30.0
            && self.sides[o].timeouts == 0
            && self.down < self.rules().downs()
            && self.half_left() > 3.0
        {
            let call =
                OffCall::new(Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0), PlayKind::Spike, 1.0, true);
            if self.probe == Some(Ask::Snap) {
                let mut p = self.pending(Ask::Snap);
                p.staff_off = Some(call);
                p.advice.push("No timeouts left: your staff spikes it to stop the clock.".into());
                p.off_options = self.go_options();
                self.hold(p);
                return;
            }
            let r = self.run_scrimmage_call(call, None);
            self.stats.plays.push(r);
            return;
        }
        let r = self.snap_play(false);
        if self.paused {
            return;
        }
        self.stats.plays.push(r);
    }

    /// Your 4th-down chart on your own snaps (topic 2): from the true win
    /// chance of each choice. None leaves the staff's judgment.
    fn chart_fourth(&self, o: usize, make: f64, in_range: bool) -> Option<FourthDown> {
        let prep = &self.ctl.as_ref().filter(|c| c.team == o)?.prep;
        let chart = prep.fourth;
        // College and high school overtime has no punts: the staff decides.
        if self.ot && !self.ot_timed() {
            return None;
        }
        // Your own cells on the chart (update 2, G4), outside the last five
        // minutes, where the game's numbers take over.
        let late = self.ot || (self.quarter >= 4 && self.game_left() <= 300.0);
        let yl = goal() - self.x;
        if !late && let Some(c) = live::chart_cell(&prep.cells, self.togo, yl) {
            return Some(match c {
                FourthDown::FieldGoal if !in_range => {
                    if yl < 40.0 {
                        FourthDown::Go
                    } else {
                        FourthDown::Punt
                    }
                }
                c => c,
            });
        }
        let edge = match chart {
            live::FourthChart::Staff => return None,
            live::FourthChart::Numbers => 0.0,
            live::FourthChart::Aggressive => -0.02,
            live::FourthChart::Conservative => 0.04,
        };
        let make = if in_range { make } else { 0.0 };
        let (_, go, punt, fg) = calling::fourth_values(&self.situation(), make);
        let len = 4.0 * self.rules().quarter;
        let left = if self.ot { 300.0 } else { self.game_left() };
        let lead = self.diff(o) as f64;
        let wp = |ev: f64| live::win_prob(lead, ev, left, len);
        let kick = if make > 0.3 && wp(fg) >= wp(punt) {
            (FourthDown::FieldGoal, wp(fg))
        } else {
            (FourthDown::Punt, wp(punt))
        };
        Some(if wp(go) >= kick.1 + edge { FourthDown::Go } else { kick.0 })
    }

    /// Garbage time on 4th down: a leader with the game won punts in his own
    /// half, and in field-goal range often goes for it with the backups
    /// rather than add three (real FBS up 28+ in the 4th: go 62%, kick 38%;
    /// close games 33% and 66%). A good sport refuses the kick more often.
    fn garbage_fourth(&mut self, choice: FourthDown, yl: f64) -> FourthDown {
        let read = self.sides[self.poss].coach.read;
        // Beaten, three points change nothing: he lets the offense play for
        // the touchdown (real FBS, down 28+ in the 4th quarter: 83% go for
        // it in field goal range).
        if read.concede && choice == FourthDown::FieldGoal && self.mind.chance(garbage::CONCEDED_GO) {
            return FourthDown::Go;
        }
        if read.ease <= 0.0 {
            return choice;
        }
        if yl > 50.0 {
            if choice == FourthDown::Go && self.mind.chance(0.7 * read.ease) {
                return FourthDown::Punt;
            }
            return choice;
        }
        // With the backups in, the kick reads as running it up.
        if choice == FourthDown::FieldGoal && read.step >= 1 {
            let s = self.sides[self.poss].team.staff.sportsmanship;
            let base = garbage::habits(self.rules().level).fg_go;
            if self.mind.chance(clamp(base + 0.04 * (s - 10.0), 0.05, 0.95)) {
                return FourthDown::Go;
            }
        }
        choice
    }

    fn kicker_range(&self, team: usize) -> f64 {
        let side = &self.sides[team];
        let p = &side.team.players[side.fg_kicker()];
        38.0 + 0.3 * p.a(Attr::KickPower) - 4.0
    }

    fn fg_make_estimate(&self, team: usize, dist: f64) -> f64 {
        let side = &self.sides[team];
        fg_make(&side.team.players[side.fg_kicker()], dist)
    }

    fn should_kneel(&self) -> bool {
        let o = self.poss;
        let lead = self.diff(o);
        if lead <= 0 || !self.running && self.quarter < 4 {
            return false;
        }
        if self.quarter == 4 {
            // A beaten team that has given up doesn't spend its timeouts.
            let opp_to = if self.sides[1 - o].coach.read.concede { 0.0 } else { self.sides[1 - o].timeouts as f64 };
            let downs = (self.rules().downs() - self.down + 1) as f64;
            let need = self.game_left() + opp_to * 38.0;
            return need <= downs * 39.0 && self.game_left() < 160.0;
        }
        self.quarter == 2 && self.half_left() < 30.0 && self.x < 60.0 && self.running
    }

    fn field_goal(&mut self, dist: f64) {
        let o = self.poss;
        let mut r = self.new_record(PlayType::FieldGoal);
        let late = self.quarter >= 4 && self.game_left() < 150.0;
        let d = self.diff(o);
        let decides = late && (-3..=0).contains(&d) || self.ot;
        let mut pressure: f64 = if decides {
            0.8
        } else if late && d.abs() <= 8 {
            0.4
        } else {
            0.1
        };
        if self.s.playoff {
            pressure += 0.15;
        }
        // Icing: the defense calls time before a big kick.
        if decides && self.sides[1 - o].timeouts > 0 && self.rng.chance(0.5) {
            self.sides[1 - o].timeouts -= 1;
            pressure += 0.1;
            let left = self.sides[1 - o].timeouts;
            self.log(1 - o, format!("Timeout to ice the kicker ({left} left)."));
        }
        let pressure: f64 = pressure.min(1.0);
        let good = self.kick(o, dist, pressure, &mut r);
        self.tick(r.duration);
        self.running = false;
        let code = self.rules().code;
        if good {
            // Four points from long range (NFL Europe from 50 yards, the UFL
            // from 60 in 2026).
            let pts = code.long_field_goal.filter(|&(min, _)| dist >= min).map_or(3, |x| x.1);
            self.add_points(o, pts);
            if pts != 3 {
                r.why = format!("{} A {pts}-point field goal from {dist:.0} yards.", r.why);
            }
            r.epa = pts as f64 - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.end_drive_score();
        } else if code.rouge != Rouge::None && !r.blocked {
            // A missed field goal in the CFL is a live kick: run out of the
            // end zone, or a single.
            r.turnover = true;
            let spot = if self.rng.chance(0.45) {
                let out = self.rng.range(3.0, 22.0);
                r.why = format!("{} Run out of the end zone to the {out:.0}.", r.why);
                out
            } else if self.rouge(o, &mut r) {
                r.why = format!("{} A single.", r.why);
                self.x
            } else {
                r.why = format!("{} Through the end zone: no single.", r.why);
                self.x
            };
            r.epa = -ep_at(self.below_nfl(), 1, 10.0, goal() - spot) - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.change_possession(spot, false);
        } else if code.field.nets && !r.blocked && self.rng.chance(0.5) {
            // Arena football: a miss off the rebound nets stays live and is
            // returned, usually not far.
            r.turnover = true;
            let spot = self.rng.range(3.0, 20.0);
            r.why = format!("{} Off the net and returned to the {spot:.0}.", r.why);
            r.epa = -ep_at(self.below_nfl(), 1, 10.0, goal() - spot) - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.change_possession(spot, false);
        } else {
            // The other team takes over at the spot of the kick (or the 20).
            let spot = (goal() - (self.x - 7.0)).max(20.0f64.min(goal() / 5.0));
            r.turnover = true;
            r.epa = -ep_at(self.below_nfl(), 1, 10.0, goal() - spot) - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.change_possession(spot, false);
        }
    }

    fn end_drive_score(&mut self) {
        let o = self.poss;
        if self.ot {
            self.possession_over(true);
            if self.over || !self.ot_kickoffs() {
                return;
            }
        }
        let from = self.rules().kickoff_from;
        self.next = Next::Kickoff { team: o, from, onside: false };
    }

    fn change_possession(&mut self, x_new: f64, turnover: bool) {
        let old = self.poss;
        self.poss = 1 - old;
        self.x = x_new.clamp(1.0, goal() - 1.0);
        self.down = 1;
        self.togo = 10.0f64.min(goal() - self.x);
        self.running = false;
        self.after_turnover = turnover;
        self.next = Next::Scrimmage;
        if self.ot {
            self.drive_start_poss = old;
            self.possession_over(false);
            if !self.ot_kickoffs() {
                return;
            }
        }
        self.drive_start_poss = self.poss;
        self.stats.team[self.poss].drives += 1;
        self.stats.team[self.poss].start_sum += self.x;
    }

    /// The fake punt itself (G18, R25): the punt team stays in its punt
    /// formation against the punt return team on the field, and the snap
    /// goes straight to the up-back, who runs behind the pulling guard
    /// (`scrimmage::return_jobs`). The men on the two units take the jobs
    /// they fit: the punt team's two fastest are the gunners, its four
    /// heaviest the guards and tackles, its best runner of the rest the
    /// up-back; on the return team the returner waits deep, its two fastest
    /// jam the gunners, its four heaviest line up on the guards and
    /// tackles, the next two over the wings and the last two hold five
    /// yards off the ball (proposed, not confirmed).
    fn fake_punt_play(&mut self, look: bool) -> PlayRecord {
        let o = self.poss;
        let d = 1 - o;
        let (kick_on, ret_on, sn) = self.punt_units();
        let (ps, qs) = (&self.sides[o].team.players, &self.sides[d].team.players);
        let take = |pool: &mut Vec<usize>, n: usize, key: &dyn Fn(usize) -> f64| -> Vec<usize> {
            pool.sort_by(|&a, &b| key(b).total_cmp(&key(a)));
            pool.drain(..n.min(pool.len())).collect()
        };
        let punter = kick_on[0];
        let mut pool: Vec<usize> = kick_on.iter().copied().filter(|&i| i != punter && i != sn).collect();
        let gun = take(&mut pool, 2, &|i| ps[i].a(Attr::Speed));
        let line = take(&mut pool, 4, &|i| ps[i].weight);
        let runner = |i: usize| {
            let p = &ps[i];
            0.4 * p.a(Attr::BallCarrierVision) + 0.3 * p.a(Attr::BallSecurity) + 0.3 * p.a(Attr::Speed)
        };
        let up = take(&mut pool, 1, &runner);
        let mut off: Vec<(ORole, usize)> = vec![(ORole::C, sn), (ORole::RB, punter)];
        off.extend(line.iter().zip([ORole::LG, ORole::RG, ORole::LT, ORole::RT]).map(|(&i, r)| (r, i)));
        off.extend(gun.iter().zip([ORole::X, ORole::Z]).map(|(&i, r)| (r, i)));
        off.extend(up.iter().map(|&i| (ORole::QB, i)));
        off.extend(pool.iter().zip([ORole::TE, ORole::TE2, ORole::Slot]).map(|(&i, r)| (r, i)));
        let returner = ret_on[0];
        let mut pool: Vec<usize> = ret_on.iter().copied().skip(1).collect();
        let jam = take(&mut pool, 2, &|i| qs[i].a(Attr::Speed));
        let big = take(&mut pool, 4, &|i| qs[i].weight);
        let wings = take(&mut pool, 2, &|i| qs[i].weight);
        let mut def: Vec<(DSlot, usize)> = vec![(DSlot::FS, returner)];
        def.extend(jam.iter().zip([DSlot::CBL, DSlot::CBR]).map(|(&i, s)| (s, i)));
        def.extend(big.iter().zip([DSlot::DTL, DSlot::DTR, DSlot::LE, DSlot::RE]).map(|(&i, s)| (s, i)));
        def.extend(wings.iter().zip([DSlot::Sam, DSlot::Will]).map(|(&i, s)| (s, i)));
        def.extend(pool.iter().zip([DSlot::Mike, DSlot::SS, DSlot::NCB]).map(|(&i, s)| (s, i)));
        self.fake = Some((off, def, look));
        let call = OffCall::new(
            Formation::plain(Personnel::P12, Backfield::Shotgun, 1.0),
            PlayKind::Run(RunConcept::QbPower),
            if self.rng.chance(0.5) { 1.0 } else { -1.0 },
            false,
        );
        let dcall = DefCall::new(DefPersonnel::Base, Coverage::Cover1, Pressure::Four);
        let mut r = self.run_scrimmage_call(call, Some(dcall));
        r.call = "punt formation, the snap straight to the up-back".into();
        r.personnel = "punt".into();
        r.call_why = if look { "a fake punt: the look was there" } else { "a fake punt, called" };
        r.look = look;
        r
    }

    /// The punt team (the punter first, the long snapper in the middle)
    /// and the punt return team (the returner first) on this punt, and the
    /// long snapper.
    fn punt_units(&self) -> (Vec<usize>, Vec<usize>, usize) {
        let o = self.poss;
        let d = 1 - o;
        let side = &self.sides[o];
        let p = side.punter();
        let sn = side.snapper();
        let mut kick_on = vec![p];
        let mut used = vec![p, sn];
        kick_on.extend(side.gunners(&mut used));
        kick_on.extend(side.cover_pick(Position::LB, 3, &mut used));
        kick_on.extend(side.cover_pick(Position::TE, 2, &mut used));
        kick_on.extend(side.cover_pick(Position::IOL, 2, &mut used));
        // Your punt team (section 6b): your men around the punter.
        let mut kick_on: Vec<usize> = side.with_package(7, &[p], kick_on).into_iter().filter(|&i| i != sn).collect();
        kick_on.truncate(10);
        // The long snapper is the unit's center and covers down the middle lane.
        kick_on.insert(5.min(kick_on.len()), sn);
        let opp = &self.sides[d];
        let pr = opp.depth.punt_returner;
        let mut ret_on = vec![pr];
        let mut used2 = vec![pr];
        ret_on.extend(opp.cover_pick(Position::CB, 2, &mut used2));
        ret_on.extend(opp.cover_pick(Position::LB, 3, &mut used2));
        ret_on.extend(opp.cover_pick(Position::S, 1, &mut used2));
        ret_on.extend(opp.cover_pick(Position::DT, 2, &mut used2));
        ret_on.extend(opp.cover_pick(Position::EDGE, 2, &mut used2));
        // Twelve a side in the CFL: a safety more on each unit.
        if self.rules().code.players > 11 {
            kick_on.extend(side.cover_pick(Position::S, 1, &mut used));
            ret_on.extend(opp.cover_pick(Position::S, 1, &mut used2));
        }
        // Your punt return team, your returner first.
        let ret_on = match opp.package(6).first() {
            Some(&r) => opp.with_package(6, &[r], ret_on),
            None => ret_on,
        };
        (kick_on, ret_on, sn)
    }

    fn punt(&mut self) {
        let o = self.poss;
        let d = 1 - o;
        let mut r = self.new_record(PlayType::Punt);
        r.look = std::mem::take(&mut self.kick_look);
        let (kick_on, ret_on, sn) = self.punt_units();
        let p = kick_on[0];
        let side = &self.sides[o];
        let opp = &self.sides[d];
        let kt = KickTeams {
            kick: &side.team.players,
            ret: &opp.team.players,
            kick_on: kick_on.clone(),
            ret_on: ret_on.clone(),
            snapper: sn,
            holder: side.depth.holder,
            energy: [&side.energy, &opp.energy],
            hurt: [&side.hurt, &opp.hurt],
            mood: self.return_mood(o, d, true),
            kick_deep: 0.0,
            style: side.team.prep.teams.punt,
            surprise: false,
        };
        let along = self.wind_along(o);
        let kr = kicking::punt(
            &kt,
            self.x,
            &self.s.rules,
            &self.s.cond,
            &self.crew,
            along,
            1.0,
            self.recording(),
            &mut self.rng,
        );
        if let Some(f) = &kr.frames {
            self.res.frames.push((r.n, f.clone()));
        }
        r.kicker = Some(kt.kick[p].id);
        r.on_field = [kick_on.iter().map(|&i| kt.kick[i].id).collect(), ret_on.iter().map(|&i| kt.ret[i].id).collect()];
        r.kick_distance = kr.distance;
        r.touchback = kr.touchback;
        r.fair_catch = kr.fair_catch;
        r.inside20 = kr.inside20;
        r.returner = kr.returner;
        r.return_yards = if kr.returned { kr.return_yards } else { 0.0 };
        r.blocked = kr.blocked;
        r.duration = kr.duration;
        r.tacklers = kr.notes.tacklers.clone();
        r.missed_tackles = kr.notes.missed.clone();
        r.injuries = kr.notes.injuries.clone();
        r.tend = kr.notes.tend;
        r.tend.muffed = kr.muffed;
        r.tend.start = Some(kr.spot);
        self.apply_injuries(&kr.notes.injuries, [d, o]);
        self.tick(kr.duration);
        self.running = false;
        r.why = if kr.blocked {
            "Punt blocked.".into()
        } else if kr.touchback {
            format!("Punt of {:.0} yards into the end zone for a touchback.", kr.distance)
        } else if kr.fair_catch {
            format!("Punt of {:.0} yards, fair caught.", kr.distance)
        } else if kr.returned {
            format!("Punt of {:.0} yards, returned {:.0}.", kr.distance, kr.return_yards)
        } else {
            format!("Punt of {:.0} yards, downed.", kr.distance)
        };
        if kr.return_td {
            r.touchdown = true;
            r.epa = -6.95 - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.add_points(d, 6);
            self.poss = d;
            self.drive_start_poss = o;
            self.next = Next::Pat(d);
            return;
        }
        if kr.recovered_by_kicking && kr.spot >= goal() {
            // Muffed in the returner's end zone and recovered there by the
            // punting team: their touchdown.
            r.touchdown = true;
            r.turnover = true;
            r.why = format!("{} Muffed in the end zone and recovered by the punting team for a touchdown.", r.why);
            r.epa = 6.95 - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.add_points(o, 6);
            self.poss = o;
            self.drive_start_poss = o;
            self.next = Next::Pat(o);
            return;
        }
        if kr.recovered_by_kicking {
            r.epa = ep_at(self.below_nfl(), 1, 10.0, goal() - kr.spot) - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.x = kr.spot.clamp(1.0, goal() - 1.0);
            self.down = 1;
            self.togo = 10.0f64.min(goal() - self.x);
            self.next = Next::Scrimmage;
            return;
        }
        let code = self.rules().code;
        let mut spot = kr.spot;
        // The rouge (CFL): a punt into the end zone must be run out or it
        // is a single; a returner brings a bit under half of them out.
        if kr.touchback && code.rouge != Rouge::None {
            if !kr.out_of_bounds && self.rng.chance(0.45) {
                spot = self.rng.range(2.0, 14.0);
                r.touchback = false;
                r.why = format!("Punt of {:.0} yards into the end zone, run out to the {:.0}.", kr.distance, spot);
            } else if self.rouge(o, &mut r) {
                spot = self.x;
                r.why = format!("Punt of {:.0} yards into the end zone: a single.", kr.distance);
            } else {
                spot = self.x;
                r.why = format!(
                    "Punt of {:.0} yards through the end zone: no single, the ball at the {:.0}.",
                    kr.distance, spot
                );
            }
        }
        // The XFL's and the UFL's punts: into the end zone, or out of bounds
        // inside the 35, the ball comes out to the 35.
        if code.punt_touchback_35 && (kr.touchback || kr.out_of_bounds && spot < 35.0) {
            spot = 35.0;
        }
        r.epa = -ep_at(self.below_nfl(), 1, 10.0, goal() - spot.clamp(1.0, goal() - 1.0)) - r.ep_before;
        self.stats.book(&r);
        self.stats.plays.push(r);
        self.ball_y = mid();
        self.change_possession(spot, false);
    }

    /// Players on the field for a call.
    #[allow(clippy::type_complexity)]
    /// The burner's snaps (speed update S10): on a jet sweep he is the man
    /// in motion, and on a deep shot he comes in at Z (else X), more often
    /// the more the team leans on his speed. A keyed draw, so the game's
    /// own stream is untouched.
    fn speed_package(&self, call: &OffCall, off: &mut [(ORole, usize)]) {
        let side = &self.sides[self.poss];
        let Some(b) = side.depth.burner else { return };
        if side.out[b] || side.benched[b] || side.held(b) || off.iter().any(|x| x.1 == b) {
            return;
        }
        let lean = side.depth.burner_lean;
        let (role, p) = match call.kind {
            PlayKind::Run(RunConcept::JetSweep) => (call.formation.motion_man(), 0.5 + 0.4 * lean),
            PlayKind::Pass(i) if pass_concept(i).depth == PassDepth::Deep => {
                let r = [ORole::Z, ORole::X].into_iter().find(|r| off.iter().any(|x| x.0 == *r));
                (r, 0.25 + 0.5 * lean)
            }
            _ => return,
        };
        let Some(role) = role else { return };
        if crate::world::season::key_uniform(self.s.seed ^ 0x5BEE_D000, self.n as u64) >= p {
            return;
        }
        if let Some(x) = off.iter_mut().find(|x| x.0 == role) {
            x.1 = b;
        }
    }

    fn lineup(&self, call: &OffCall, dcall: &DefCall) -> (Vec<(ORole, usize)>, Vec<(DSlot, usize)>) {
        let o = &self.sides[self.poss];
        let d = &self.sides[1 - self.poss];
        let mut used = Vec::new();
        let mut off = Vec::new();
        let roles = call.formation.personnel.roles();
        let add = |list: Vec<usize>, rs: &[ORole], off: &mut Vec<(ORole, usize)>| {
            for (k, r) in rs.iter().enumerate() {
                if let Some(&i) = list.get(k) {
                    off.push((*r, i));
                }
            }
        };
        // The line: each man to the spot he plays best together with the
        // others, a fill-in paying for a spot he doesn't know (3.3).
        let line = |list: Vec<usize>, rs: &[ORole], off: &mut Vec<(ORole, usize)>| {
            let lefty = off.iter().any(|&(r, i)| r == ORole::QB && o.team.players[i].left_handed);
            let spots: Vec<Spot> = rs.iter().map(|&r| orole_spot(r)).collect();
            let order = best_order(&list, &spots, &|i, sp| o.on_line(i, sp, lefty));
            for (k, i) in order.into_iter().enumerate() {
                if let Some(i) = i {
                    off.push((rs[k], i));
                }
            }
        };
        // Your goal line offense (section 6b) inside the 3 with heavy personnel.
        let gl = if call.formation.personnel.wide_receivers() > 1 {
            None
        } else if self.situation().yardline_100 <= 3.0 {
            Some(3u8)
        } else {
            // Your jumbo package outside the goal line (G15).
            self.sides[self.poss].team.prep.package(9).map(|_| 9u8)
        };
        add(o.pick_pref(Position::QB, 1, &mut used, gl), &[ORole::QB], &mut off);
        // Arena football's line is a center and two guards (8 a side).
        if self.rules().code.players > 8 {
            line(o.pick_pref(Position::OT, 2, &mut used, gl), &[ORole::LT, ORole::RT], &mut off);
        }
        line(o.pick_pref(Position::IOL, 3, &mut used, gl), &[ORole::LG, ORole::C, ORole::RG], &mut off);
        let backs: Vec<ORole> = roles.iter().copied().filter(|r| r.is_back() && *r != ORole::FB).collect();
        add(o.pick_pref(Position::RB, backs.len(), &mut used, gl), &backs, &mut off);
        let wrs: Vec<ORole> = roles
            .iter()
            .copied()
            .filter(|r| matches!(r, ORole::X | ORole::Z | ORole::Slot | ORole::Slot2 | ORole::Slot3))
            .collect();
        add(o.pick(Position::WR, wrs.len(), &mut used), &wrs, &mut off);
        let tes: Vec<ORole> =
            roles.iter().copied().filter(|r| matches!(r, ORole::TE | ORole::TE2 | ORole::TE3)).collect();
        add(o.pick_pref(Position::TE, tes.len(), &mut used, gl), &tes, &mut off);
        if roles.contains(&ORole::FB) {
            match o.pick_fb(&mut used) {
                Some(i) => off.push((ORole::FB, i)),
                None => add(o.pick(Position::RB, 1, &mut used), &[ORole::FB], &mut off),
            }
        }
        let three_four = d.team.staff.front == Front::ThreeFour;
        let slots: Vec<(DSlot, Position)> = match dcall.personnel {
            DefPersonnel::Base if three_four => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::RE, Position::EDGE),
                (DSlot::DTL, Position::DT),
                (DSlot::NT, Position::DT),
                (DSlot::DTR, Position::DT),
                (DSlot::Mike, Position::LB),
                (DSlot::Will, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::FS, Position::S),
                (DSlot::SS, Position::S),
            ],
            DefPersonnel::Base => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::RE, Position::EDGE),
                (DSlot::DTL, Position::DT),
                (DSlot::DTR, Position::DT),
                (DSlot::Sam, Position::LB),
                (DSlot::Mike, Position::LB),
                (DSlot::Will, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::FS, Position::S),
                (DSlot::SS, Position::S),
            ],
            DefPersonnel::Nickel => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::RE, Position::EDGE),
                (DSlot::DTL, Position::DT),
                (DSlot::DTR, Position::DT),
                (DSlot::Mike, Position::LB),
                (DSlot::Will, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::NCB, Position::CB),
                (DSlot::FS, Position::S),
                (DSlot::SS, Position::S),
            ],
            DefPersonnel::Dime => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::RE, Position::EDGE),
                (DSlot::DTL, Position::DT),
                (DSlot::DTR, Position::DT),
                (DSlot::Mike, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::NCB, Position::CB),
                (DSlot::DCB, Position::CB),
                (DSlot::FS, Position::S),
                (DSlot::SS, Position::S),
            ],
            DefPersonnel::GoalLine => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::RE, Position::EDGE),
                (DSlot::DTL, Position::DT),
                (DSlot::NT, Position::DT),
                (DSlot::DTR, Position::DT),
                (DSlot::Sam, Position::LB),
                (DSlot::Mike, Position::LB),
                (DSlot::Will, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::SS, Position::S),
            ],
        };
        // The other leagues (X7): arena football's eight are three linemen,
        // the Mac and Jack linebackers and three backs; the CFL's twelfth
        // is a fifth defensive back (a halfback), or the man the set lacks.
        let slots = match self.rules().code.players {
            8 => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::NT, Position::DT),
                (DSlot::RE, Position::EDGE),
                (DSlot::Mike, Position::LB),
                (DSlot::Will, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::FS, Position::S),
            ],
            12 => {
                let mut v = slots;
                let has = |v: &Vec<(DSlot, Position)>, x: DSlot| v.iter().any(|s| s.0 == x);
                let extra = [
                    (DSlot::NCB, Position::CB),
                    (DSlot::DCB, Position::CB),
                    (DSlot::Will, Position::LB),
                    (DSlot::Sam, Position::LB),
                    (DSlot::FS, Position::S),
                ]
                .into_iter()
                .find(|e| !has(&v, e.0));
                v.extend(extra);
                v
            }
            _ => slots,
        };
        let mut dused = Vec::new();
        let mut def = Vec::new();
        for pos in [Position::EDGE, Position::DT, Position::LB, Position::CB, Position::S] {
            let want: Vec<DSlot> = slots.iter().filter(|s| s.1 == pos).map(|s| s.0).collect();
            let board = match dcall.personnel {
                DefPersonnel::Nickel => Some(0),
                DefPersonnel::Dime => Some(1),
                DefPersonnel::GoalLine => Some(2),
                _ => None,
            };
            let got = d.pick_pref(pos, want.len(), &mut dused, board);
            let spots: Vec<Spot> = want.iter().map(|&s| dslot_spot(s)).collect();
            let order = best_order(&got, &spots, &|i, sp| d.at_spot(i, sp));
            for (k, i) in order.into_iter().enumerate() {
                if let Some(i) = i {
                    def.push((want[k], i));
                }
            }
        }
        (off, def)
    }

    /// Pre-snap fouls (topic 4): jumping early, delay, formation. Discipline,
    /// crowd noise, fatigue and knowing the playbook decide them.
    fn pre_snap_foul(&mut self, off: &[(ORole, usize)], def: &[(DSlot, usize)]) -> Option<Foul> {
        let o = self.poss;
        let crowd = self.home_crowd(o);
        // The week's rules clinic (TR29) on each side.
        let crew = self.crew.pre_snap * self.sides[o].team.prep.rules;
        let def_crew = self.crew.pre_snap * self.sides[1 - o].team.prep.rules;
        for &(r, i) in off {
            if !(r.is_line() || r.is_receiver()) {
                continue;
            }
            let p = &self.sides[o].team.players[i];
            let tired = 1.0 - self.sides[o].energy[i];
            let level = self.s.rules.level;
            // Road teams jump 21% more often in real FBS conference games
            // (1.28 a game against 1.06, cfbfastR 2023-24). Concentration
            // joins discipline, and decides how much the noise gets to him
            // (proposed, not confirmed).
            let conc = p.trait20(Attr::Concentration, level);
            let focus = 0.6 * p.trait20(Attr::Discipline, level) + 0.4 * conc;
            let chance = 0.00162
                * (1.6 - focus / 15.0).max(0.3)
                * (1.0 + 0.32 * crowd * clamp(1.5 - 0.05 * conc, 0.5, 1.5))
                * crew
                * (1.0 + tired)
                * (1.25 - 0.25 * p.knowledge);
            if self.rng.chance(chance) {
                return Some(Foul { kind: FoulKind::FalseStart, side: 0, player: p.id, at: self.x });
            }
        }
        let qb = off.iter().find(|x| x.0 == ORole::QB).map(|x| x.1).unwrap_or(0);
        let qp = &self.sides[o].team.players[qb];
        // The hard count on 3rd or 4th and short: a smart quarterback draws
        // the defense offside (proposed, not confirmed).
        // Against a front of snap guessers the plan uses it on more downs
        // (tendencies topic 4.4).
        let vs = self.sides[o].mem.plan.vs.hard_count;
        let hard = if self.down >= 3 && self.togo <= 2.0 || vs > 0.0 && (self.down >= 3 || self.togo <= 3.0) {
            clamp(1.5 + 0.05 * (qp.trait20(Attr::FootballIq, self.s.rules.level) - 10.0), 1.0, 2.5) * (1.0 + 0.5 * vs)
        } else {
            1.0
        };
        for &(s, i) in def {
            if !s.is_line() {
                continue;
            }
            let p = &self.sides[1 - o].team.players[i];
            // The best snap anticipators get off early, and now and then too
            // early (read against his level; proposed, not confirmed).
            let jump = clamp(1.0 + 0.04 * (p.trait20(Attr::Anticipation, self.s.rules.level) - 10.0), 0.6, 1.5);
            // Times the snap (tendencies): a guesser jumps more, and a hard
            // count gets him most. A "no snap guessing" order he follows
            // sets him to 1.
            let level = self.s.rules.level;
            let guess = match p.orders.sets(crate::tendencies::Tend::SnapTiming) {
                Some(v)
                    if self.rng.chance(crate::tendencies::obey_chance(
                        p.trait20(Attr::Discipline, level),
                        p.personality.coachability,
                        p.trust,
                        self.sides[1 - o].team.staff.strictness,
                    )) =>
                {
                    v
                }
                _ => p.tend.get(crate::tendencies::Tend::SnapTiming),
            };
            let lean = (guess - crate::tendencies::MID) / 9.5;
            let guess = (1.0 + 1.1 * lean).max(0.1) * hard.powf(1.0 + 0.8 * lean.max(0.0)) / hard.max(1.0);
            // (0.0019 before tendencies: guessers are the less disciplined
            // men and a hard count now targets them, so 0.00172 keeps the
            // league's offside where it was.)
            let chance = 0.00172
                * (1.6 - p.trait20(Attr::Discipline, self.s.rules.level) / 15.0).max(0.3)
                * jump
                * hard
                * guess
                * def_crew;
            if self.rng.chance(chance) {
                return Some(Foul { kind: FoulKind::Offside, side: 1, player: p.id, at: self.x });
            }
        }
        // Road delays: 34% more (0.34 a game against 0.26).
        if self.rng.chance(0.0050 * (1.3 - qp.a(Attr::FootballIq) / 120.0) * (1.0 + 0.52 * crowd) * crew) {
            // Your timeout rule (update 2, G4): a timeout instead of five
            // yards when the rule leaves one to spend.
            if self.timeout_saves_delay(o) {
                self.sides[o].timeouts -= 1;
                self.running = false;
                let left = self.sides[o].timeouts;
                self.log(o, format!("Timeout to beat the play clock ({left} left)."));
                return None;
            }
            return Some(Foul { kind: FoulKind::DelayOfGame, side: 0, player: qp.id, at: self.x });
        }
        // Illegal formation stands for every 5-yard offensive alignment foul
        // (shifts, motion, ineligible downfield, too many men): real about
        // 0.6 a team game together (Claude's reading of 2023, proposed, not
        // confirmed). False starts ran a fifth high, delay a quarter low.
        // Lining up right is the quarterback's and the line's football IQ
        // (proposed, not confirmed).
        let callers: Vec<f64> = off
            .iter()
            .filter(|x| x.0.is_line() || x.0 == ORole::QB)
            .map(|x| self.sides[o].team.players[x.1].a(Attr::FootballIq))
            .collect();
        let line_iq = callers.iter().sum::<f64>() / callers.len().max(1) as f64;
        let align = clamp(1.0 - 0.008 * (line_iq - 60.0), 0.6, 1.4);
        if self.rng.chance(0.0078 * crew * (1.3 - qp.knowledge * 0.3) * align) {
            let id =
                off.iter().find(|x| x.0.is_receiver()).map(|x| self.sides[o].team.players[x.1].id).unwrap_or(qp.id);
            return Some(Foul { kind: FoulKind::IllegalFormation, side: 0, player: id, at: self.x });
        }
        None
    }

    fn snap_play(&mut self, two_point: bool) -> PlayRecord {
        self.snap_play_as(two_point, None)
    }

    /// A snap with the call held to a run or a pass (`run`), as a practice
    /// period asks for: inside run is all runs, seven on seven all passes.
    fn snap_play_as(&mut self, two_point: bool, run: Option<bool>) -> PlayRecord {
        let o = self.poss;
        let sit = self.situation();
        let staff_o = self.sides[o].team.staff;
        let staff_d = self.sides[1 - o].team.staff;
        let has_fb = self.sides[o].depth.list(Position::RB).len() >= 2;
        let te_count = self.sides[o].depth.list(Position::TE).iter().filter(|&&i| !self.sides[o].out[i]).count();
        let qb = self.sides[o].depth.list(Position::QB).iter().copied().find(|&i| !self.sides[o].out[i]);
        let qb_threat = qb.map_or(0.0, |i| qb_run(&self.sides[o].team.players[i]));
        let ctx = calling::OffCtx {
            has_fb,
            te_count,
            qb_run: qb_threat,
            level: self.s.rules.level,
            plan: self.sides[o].team.plan().0,
            designs: calling::sheet_designs(&self.sides[o].team.prep),
            burner: self.sides[o].burner_lean(),
        };
        let mut call = calling::call_offense(&staff_o, &self.sides[o].mem, &sit, &ctx, &mut self.rng);
        // Your four-minute plan (update 2, G4): run it out, or keep
        // attacking.
        let run = run.or_else(|| {
            let four = self.ctl.as_ref().filter(|c| c.team == o)?.prep.four;
            if !super::sheet::four_minute(&sit) || sit.hurry() || two_point {
                return None;
            }
            match four {
                live::FourMinute::RunItOut if !call.kind.is_run() && self.rng.chance(0.6) => Some(true),
                live::FourMinute::KeepAttacking if call.kind.is_run() && self.rng.chance(0.35) => Some(false),
                _ => None,
            }
        });
        if let Some(want) = run {
            for _ in 0..8 {
                if call.kind.is_run() == want {
                    break;
                }
                call = calling::call_offense(&staff_o, &self.sides[o].mem, &sit, &ctx, &mut self.rng);
            }
        }
        // Your call sheet (section 6b; update 2, G7-G13): the coordinator
        // calls the week's plays for the situation, run or pass as he read
        // it, the opening script in order, by the hash, the bad-weather
        // version in bad weather.
        if !self.sides[o].card.sheet.is_empty() {
            let sctx = super::sheet::Ctx {
                scripted: self.sides[o].scripted[0],
                hash: super::sheet::hash_of(self.ball_y),
                wet: self.bad_weather(),
                two_point,
                hurry: sit.hurry(),
                forced: run.is_some(),
            };
            let side = &self.sides[o];
            let again = |rng: &mut Rng| calling::call_offense(&staff_o, &side.mem, &sit, &ctx, rng);
            if let Some((c, _)) = super::sheet::off_call(&side.card, &sit, &call, sctx, &mut self.rng, again) {
                call = c;
            }
        }
        if two_point && matches!(call.kind, PlayKind::Pass(_)) && self.rng.chance(0.3) {
            call.kind = PlayKind::Pass(0);
        }
        // Your offense (topic 4): the play caller's call stands, or yours.
        if self.user_is(o) {
            if matches!(self.probe, Some(Ask::Snap) | Some(Ask::Pat)) {
                let ask = self.probe.unwrap_or(Ask::Snap);
                let mut p = self.pending(ask);
                p.staff_off = Some(call);
                p.staff_two = two_point;
                p.off_options = self.off_options(&sit, &ctx, &call);
                self.hold(p);
                return PlayRecord::new(0, self.quarter, self.clock, o, PlayType::NoPlay);
            }
            let yours = match &self.snap_input {
                Some(Input::Call { offense: Some(c), .. }) | Some(Input::Pat { offense: Some(c), .. }) => Some(*c),
                _ => None,
            };
            if let Some(mut c) = yours {
                let sheet = self.off_options(&sit, &ctx, &call);
                let on_sheet = (call.kind == c.kind && call.formation.personnel == c.formation.personnel)
                    || sheet.iter().any(|x| x.kind == c.kind && x.formation.personnel == c.formation.personnel);
                self.mark.0 = true;
                self.mark.2 = !on_sheet;
                self.mark.3 = call.describe();
                c.why = if on_sheet { "your call" } else { "your call, off this week's sheet" };
                call = c;
            }
        }
        // The other leagues' sets (X7): the CFL's twelfth man is a fifth
        // receiver, arena football plays a back and three receivers.
        call.formation.personnel = call.formation.personnel.for_players(self.rules().code.players);
        // Your defense: what they sent on is all you see before you call.
        let mut dinput = None;
        if self.user_is(1 - o) {
            let mut again = false;
            loop {
                match self.ask(Ask::Defense, again) {
                    Some(Input::Timeout) => self.user_timeout(1 - o),
                    Some(Input::Plan(pc)) => self.apply_plan(1 - o, pc),
                    Some(Input::Tempo(_)) => {}
                    other => {
                        dinput = other;
                        break;
                    }
                }
                again = true;
            }
        }
        let mut dctx = calling::DefCtx { qb_run: qb_threat, ..Default::default() };
        // Our call words in their hands (update 2, G5): someone who left
        // with them reads the call like a tell.
        let leak = self.sides[o].team.prep.leak[0];
        if leak > 0.0 && self.rng.chance(leak) {
            dctx.tip = if call.kind.is_run() { -1.0 } else { 1.0 };
        }
        let mut dcall =
            calling::call_defense(&staff_d, &self.sides[1 - o].mem, &sit, &call.formation, &dctx, &mut self.rng);
        // Your defensive call sheet (update 2, G8), inside the
        // coordinator's own read of the snap.
        self.def_script = false;
        if !self.sides[1 - o].card.dsheet.is_empty() {
            let dctx_sheet = super::sheet::DefCtx {
                wet: self.bad_weather(),
                two_point,
                scripted: self.sides[1 - o].scripted[1],
                tipped: dctx.tip != 0.0,
            };
            let side = &self.sides[1 - o];
            let formation = call.formation;
            let again = |rng: &mut Rng| calling::call_defense(&staff_d, &side.mem, &sit, &formation, &dctx, rng);
            if let Some((c, scripted)) =
                super::sheet::def_call(&side.card, &sit, &dcall, dctx_sheet, &mut self.rng, again)
            {
                dcall = c;
                self.def_script = scripted;
            }
        }
        let leak = self.sides[1 - o].team.prep.leak[1];
        if leak > 0.0 && self.rng.chance(leak) {
            dcall.disguise = false;
        }
        if self.user_is(1 - o) {
            if self.probe == Some(Ask::Defense) {
                let mut p = self.pending(Ask::Defense);
                p.staff_def = Some(dcall);
                p.their_personnel = Some(call.formation.personnel);
                p.def_options = self.def_options(&sit, &call, &dcall, qb_threat);
                self.hold(p);
                return PlayRecord::new(0, self.quarter, self.clock, o, PlayType::NoPlay);
            }
            if let Some(Input::Call { defense: Some(dc), .. }) = dinput {
                let sheet = self.def_options(&sit, &call, &dcall, qb_threat);
                let on_sheet = (dcall.coverage == dc.coverage && dcall.pressure == dc.pressure)
                    || sheet.iter().any(|x| x.coverage == dc.coverage && x.pressure == dc.pressure);
                self.mark.1 = true;
                self.mark.2 |= !on_sheet;
                self.mark.3 = dcall.describe();
                dcall = dc;
                self.def_script = false;
            }
        }
        self.run_scrimmage_call(call, Some(dcall))
    }

    fn run_scrimmage_call(&mut self, mut call: OffCall, dcall: Option<DefCall>) -> PlayRecord {
        let o = self.poss;
        let d = 1 - o;
        let sit = self.situation();
        // The scripts move on only when their call is run (an audible
        // still used it; a foul before the snap did not).
        let from_script = call.why == "the opening script";
        let def_script = std::mem::take(&mut self.def_script);
        // A fake punt brings its own two units (`fake_punt_play`).
        let fake = self.fake.take();
        call.formation.personnel = call.formation.personnel.for_players(self.rules().code.players);
        let dcall =
            self.legal_defense(dcall.unwrap_or(DefCall::new(DefPersonnel::Base, Coverage::Cover3, Pressure::Four)));
        self.sides[d].frozen = self.no_huddle;
        // Who sits by the week's plan this snap (G14, G15).
        let passing = (self.down >= 3 && self.togo >= 4.0) || (self.down == 2 && self.togo >= 8.0) || sit.hurry();
        for k in 0..2 {
            self.sides[k].passing = passing;
            self.sides[k].series = self.stats.team[1 - k].drives as u16;
            self.sides[k].scrimmage = true;
            self.sides[k].defending = k == d;
        }
        let (off, def) = match &fake {
            Some((off, def, _)) => (off.clone(), def.clone()),
            None => {
                let (mut off, def) = self.lineup(&call, &dcall);
                self.speed_package(&call, &mut off);
                (off, def)
            }
        };
        for side in self.sides.iter_mut() {
            side.scrimmage = false;
        }
        self.sides[d].frozen = false;
        for &(_, i) in &off {
            self.sides[o].snaps[i] = self.sides[o].snaps[i].saturating_add(1);
        }
        for &(_, i) in &def {
            self.sides[d].snaps[i] = self.sides[d].snaps[i].saturating_add(1);
        }
        // Before the snap (plays and tactics topic 3): the quarterback reads
        // the numbers in the box and the shell, as far as his coach lets him
        // act on it (3.3): no freedom, a two-play "kill" call, or full
        // audibles. Trust grows with experience; a strict coach gives less.
        let qb = off.iter().find(|x| x.0 == ORole::QB).map(|x| x.1).unwrap_or(0);
        let qp = &self.sides[o].team.players[qb];
        let box_n = def
            .iter()
            .filter(|(s, _)| s.is_line() || s.is_lb() || *s == DSlot::SS && dcall.coverage.high() < 2)
            .count() as i32;
        let blockers =
            5 + off.iter().filter(|x| matches!(x.0, ORole::TE | ORole::TE2 | ORole::TE3 | ORole::FB)).count() as i32;
        let numbers = box_n - blockers;
        let freedom =
            clamp(0.85 - 0.035 * self.sides[o].team.staff.strictness + 0.04 * qp.experience.min(5.0), 0.1, 0.95);
        let iq = 0.5 * qp.a(Attr::PlayRecognition) + 0.5 * qp.a(Attr::FootballIq);
        // His read of the shell: a disguise fools him unless he out-thinks
        // the secondary (3.5).
        let shows_two = dcall.coverage.high() == 2 || dcall.disguise;
        let db_iq = def
            .iter()
            .filter(|(s, _)| s.is_db())
            .map(|&(_, i)| self.sides[d].team.players[i].a(Attr::Disguise))
            .sum::<f64>()
            / def.iter().filter(|(s, _)| s.is_db()).count().max(1) as f64;
        let fooled =
            dcall.disguise && dcall.coverage.high() < 2 && !self.rng.chance(clamp(0.5 + 0.02 * (iq - db_iq), 0.1, 0.9));
        // A road crowd drowns out checks at the line: hand signals carry
        // fewer of them.
        let noise = 1.0 - 0.5 * self.home_crowd(o);
        let sees = clamp((iq - 40.0) / 50.0, 0.05, 0.95) * noise;
        let mut audible = false;
        if !sit.hurry() && call.kind != PlayKind::Kneel && fake.is_none() {
            let run_called = matches!(call.kind, PlayKind::Run(_));
            let pass_called = matches!(call.kind, PlayKind::Pass(c) if pass_concept(c).depth != PassDepth::Screen);
            // Run into a loaded box, or throw against a light one with two deep.
            // A defense is meant to be one up against the run (an extra
            // man the back must beat); two up is a loaded box.
            // The quarterback's checks this week (update 2, G19): the first
            // two are on unless switched off, the last two off unless on.
            let checks = self.sides[o].checks;
            let on = |b: u8| (checks >> b) & 1 == 1;
            let want = if !on(0) && run_called && numbers >= 2 && sit.togo > 2.0 {
                Some(true)
            } else if !on(1) && pass_called && numbers <= -1 && (shows_two && !fooled) && sit.togo <= 6.0 {
                Some(false)
            } else if on(2) && run_called && dcall.coverage == Coverage::Cover0 && !fooled && sit.togo > 2.0 {
                Some(true)
            } else if on(3) && pass_called && sit.down <= 2 && shows_two && !fooled && numbers <= 0 {
                Some(false)
            } else {
                None
            };
            if let Some(to_pass) = want
                && self.rng.chance(sees)
            {
                let alt = call.alt.filter(|a| matches!(a, PlayKind::Pass(_)) == to_pass);
                let new = if freedom >= 0.35 && alt.is_some() {
                    alt
                } else if freedom >= 0.7 {
                    Some(if to_pass {
                        PlayKind::Pass(calling::quick_check(sit.togo))
                    } else {
                        PlayKind::Run(RunConcept::InsideZone)
                    })
                } else {
                    None
                };
                if let Some(k) = new {
                    call.kind = k;
                    audible = true;
                }
            }
        }
        let fam = call_family(call.kind);
        let (belief_run, sure) = self.sides[d].mem.belief(call.formation.look());
        let mut r = self.new_record(match call.kind {
            PlayKind::Run(_) | PlayKind::Rpo(..) => PlayType::Run,
            PlayKind::Pass(_) => PlayType::Pass,
            PlayKind::Kneel => PlayType::Kneel,
            PlayKind::Spike => PlayType::Spike,
        });
        r.call = format!(
            "{} {} {} {}{}",
            call.formation.personnel.name(),
            match call.formation.backfield {
                Backfield::Shotgun => "gun",
                Backfield::Pistol => "pistol",
                Backfield::UnderCenter => "under center",
            },
            call.formation.set.name(),
            call.describe(),
            if audible { " (audible)" } else { "" }
        );
        r.call_why = if audible {
            if call.kind.is_pass() {
                "the quarterback saw a loaded box and checked to a throw"
            } else {
                "the quarterback saw a light box and checked to a run"
            }
        } else {
            call.why
        };
        r.defense_call = dcall.describe();
        r.dpers = 1 + dcall.personnel as u8;
        let mark = std::mem::take(&mut self.mark);
        r.yours = mark.0 as u8 | (mark.1 as u8) << 1;
        r.off_sheet = mark.2;
        r.staff_call = mark.3;
        r.personnel = call.formation.personnel.name().into();
        r.blitz = dcall.pressure.extra() > 0;
        r.coverage_man = dcall.coverage.is_man();
        // Pre-snap foul: no play.
        if !matches!(call.kind, PlayKind::Kneel)
            && let Some(f) = self.pre_snap_foul(&off, &def)
        {
            return self.no_play(r, f);
        }
        if from_script {
            self.sides[o].scripted[0] += 1;
        }
        if def_script {
            self.sides[d].scripted[1] += 1;
        }
        let labels: Vec<(u32, String)> = off
            .iter()
            .map(|&(rl, i)| (self.sides[o].team.players[i].id, scrimmage::orole_label(rl).to_string()))
            .chain(def.iter().map(|&(s, i)| (self.sides[d].team.players[i].id, s.label().to_string())))
            .collect();
        let label =
            |id: u32| labels.iter().find(|l| l.0 == id).map(|l| format!("{} #{}", l.1, id)).unwrap_or(format!("#{id}"));
        // A man playing hurt loses what his injury limits (update 3, F7:
        // `sim::hampered`, read in `run_snap`), not a flat cut.
        let mut hurt_edge_o: Vec<f64> = self.sides[o].edge.clone();
        let mut hurt_edge_d: Vec<f64> = self.sides[d].edge.clone();
        // A man at a spot he doesn't know plays below his rating there (3.3);
        // the punt units' jobs are their own.
        if fake.is_none() {
            for &(rl, i) in &off {
                hurt_edge_o[i] -= self.sides[o].unfamiliar(i, orole_spot(rl));
            }
            for &(sl, i) in &def {
                hurt_edge_d[i] -= self.sides[d].unfamiliar(i, dslot_spot(sl));
            }
        }
        // An emergency (3.6): a man from another position group, when his
        // own group is out of bodies. Tight ends at fullback and corners at
        // receiver (or the other way) are no emergency (3.4, 3.5), nor is a
        // high school two-way player his coach listed there (3.10).
        let stranger = |side: &Side, i: usize, sp: Spot| {
            !same_group(side.team.players[i].position, sp) && !side.depth.list(sp.position()).contains(&i)
        };
        r.emergency = fake.is_none()
            && (off.iter().any(|&(rl, i)| stranger(&self.sides[o], i, orole_spot(rl)))
                || def.iter().any(|&(sl, i)| stranger(&self.sides[d], i, dslot_spot(sl))));
        let crowd = self.home_crowd(o);
        // What he knows of the coverage before the snap (topic 3): his own
        // read, a disguise that fooled him, and motion against man (the
        // defender follows his man across unless the coordinator bumps).
        let mut pre_read = if fooled { 0.0 } else { clamp((iq - 45.0) / 50.0, 0.0, 1.0) * noise };
        if call.formation.motion == Motion::Across
            && dcall.coverage.is_man()
            && !self.rng.chance(0.15 + 0.02 * self.sides[d].team.staff.defense.reading)
        {
            // What motion shows him: his recognition against the
            // secondary's disguise (proposed, not confirmed).
            let shows = clamp(1.0 + 0.01 * (self.sides[o].team.players[qb].a(Attr::PlayRecognition) - db_iq), 0.5, 1.5);
            pre_read = (pre_read + 0.35 * shows).min(1.0);
        }
        let tac = scrimmage::Tactics {
            belief_run,
            sure,
            study: self.sides[d].team.staff.film_study,
            pre_read,
            fooled,
            target_cb: self.sides[o].mem.plan.target_cb,
            bracket: self.sides[d].mem.plan.bracket,
            burner: self.sides[d].mem.plan.burner,
            fear: self.sides[d].mem.plan.fear,
            help_on: self.sides[o].mem.plan.help_on,
            chem: self.sides[o].team.line_chem,
            novelty: self.sides[o].team.novelty[fam as usize],
            family: fam,
            // Linemen downfield on a pass: NFL one yard, NCAA three, high
            // school (NFHS) two.
            downfield: match self.s.rules.level {
                Level::NFL => 1.0,
                Level::HS => 2.0,
                _ => 3.0,
            },
            // A call off the week's sheet was barely practiced: more
            // mental errors on it (game day topic 4; proposed, not
            // confirmed: as much as a third more of a plan to learn).
            load: [
                self.sides[o].team.plan().1 + if r.off_sheet && r.yours & 1 != 0 { 0.35 } else { 0.0 },
                self.sides[d].team.plan().1 + if r.off_sheet && r.yours & 2 != 0 { 0.35 } else { 0.0 },
            ],
            double_moves: self.sides[o].mem.plan.vs.double_moves,
            learn: [self.sides[o].team.prep.learn[0], self.sides[d].team.prep.learn[1]],
        };
        let snap = Snap {
            teams: [&self.sides[o].team.players, &self.sides[d].team.players],
            offense: &off,
            defense: &def,
            energy: [&self.sides[o].energy, &self.sides[d].energy],
            edge: [&hurt_edge_o, &hurt_edge_d],
            hurt: [&self.sides[o].hurt, &self.sides[d].hurt],
            trust: [&self.sides[o].trust, &self.sides[d].trust],
            call,
            dcall,
            los: self.x,
            ball_y: self.ball_y,
            down: self.down,
            togo: self.togo,
            rules: &self.s.rules,
            cond: Conditions { crowd, cold_unused: self.s.cold_unused[o], ..self.s.cond },
            crew: self.crew,
            strictness: [self.sides[o].team.staff.strictness, self.sides[d].team.staff.strictness],
            injury_scale: 1.0,
            record: self.recording(),
            // Your games keep the line chart (lane 9); no other game does.
            chart: self.s.keep || self.ctl.is_some(),
            hurry: sit.hurry(),
            two_gap: self.sides[d].team.staff.front == Front::ThreeFour,
            tac,
            mood: self.mood(o),
            fake_punt: fake.as_ref().map(|f| f.2),
        };
        let res = scrimmage::run_snap(&snap, &mut self.rng);
        // The officials' ruling, and a review (topic 6).
        let res =
            if matches!(call.kind, PlayKind::Kneel | PlayKind::Spike) { res } else { self.officiate(res, &mut r) };
        if self.paused {
            return r;
        }
        r.on_field = [
            off.iter().map(|&(_, i)| self.sides[o].team.players[i].id).collect(),
            def.iter().map(|&(_, i)| self.sides[d].team.players[i].id).collect(),
        ];
        r.box_count = res.box_count;
        r.play_action = res.play_action;
        // Fatigue (topic 6).
        for &(side, p, work) in &res.work {
            let team = if side == 0 { o } else { d };
            // Lungs (Dynamic Links): five stamina points of wind.
            let gp = &self.sides[team].team.players[p];
            let st = gp.a(Attr::Stamina) + gp.links.edge(&[crate::links::Link::Lungs]);
            let wear = 1.0 + self.s.wear[team];
            self.sides[team].energy[p] = (self.sides[team].energy[p] - 0.041 * drain(st, work) * wear).max(0.25);
        }
        for t in [o, d] {
            let on: Vec<usize> =
                if t == o { off.iter().map(|x| x.1).collect() } else { def.iter().map(|x| x.1).collect() };
            for i in 0..self.sides[t].energy.len() {
                // Lungs: he gets his wind back sooner (a full link a tenth faster).
                let lungs = 1.0 + 0.1 * self.sides[t].team.players[i].links.share(&[crate::links::Link::Lungs]);
                let rec = self.pace * lungs * if on.contains(&i) { RECOVER_ON_FIELD } else { RECOVER_ON_BENCH };
                self.sides[t].energy[i] = (self.sides[t].energy[i] + rec).min(1.0);
            }
        }
        self.apply_injuries(&res.notes.injuries, [o, d]);
        if let Some(f) = res.frames.clone() {
            self.res.frames.push((r.n, f));
        }
        self.resolve(r, res, &label, call, dcall)
    }

    fn no_play(&mut self, mut r: PlayRecord, f: Foul) -> PlayRecord {
        let o = self.poss;
        let (yds, _) = f.kind.yards();
        r.kind = PlayType::NoPlay;
        let team = if f.side == 0 { o } else { 1 - o };
        if f.side == 0 {
            let y = yds.min(self.x / 2.0);
            self.x -= y;
            self.togo += y;
            r.penalties.push((f.kind, team, f.player, y, true));
            r.yards = -y;
        } else {
            let y = yds.min((goal() - self.x) / 2.0);
            self.x += y;
            self.togo -= y;
            r.penalties.push((f.kind, team, f.player, y, true));
            r.yards = y;
            if self.togo <= 0.0 {
                self.down = 1;
                self.togo = 10.0f64.min(goal() - self.x);
                r.first_down = true;
            }
        }
        r.why = format!("{} on #{} before the snap.", f.kind.name(), f.player);
        r.epa = ep_at(self.below_nfl(), self.down, self.togo, goal() - self.x) - r.ep_before;
        self.edge_hit(team, f.player, -1.0);
        self.stats.book(&r);
        // A running clock starts again on the ready signal once the penalty
        // is marked off, except late in a half (a mercy-rule clock just runs).
        if self.running && self.mercy_running() {
            self.after_snap_clock(&r, None);
        } else if self.running && !self.clock_late() {
            let lost = self.rules().play_clock - ADMIN_PLAY_CLOCK;
            self.after_snap_clock(&r, Some(lost));
        } else {
            self.running = false;
        }
        r
    }

    fn apply_injuries(&mut self, inj: &[Injury], teams: [usize; 2]) {
        for i in inj {
            let team = teams[i.side];
            // Concussion protocol per level (topic 6): spotters in the NFL,
            // trainers in college, and some high schools have nobody.
            let mut i2 = i.clone();
            if i.kind == InjuryKind::Concussion {
                let caught = match self.rules().level {
                    Level::NFL => 0.92,
                    Level::HS => 0.6,
                    _ => 0.8,
                };
                i2.plays_on = !self.rng.chance(caught);
            }
            if i2.plays_on {
                self.sides[team].hurt[i.p] = Some(i.kind);
            } else {
                self.sides[team].out[i.p] = true;
            }
            self.res.injuries.push((team, i2));
        }
    }

    fn edge_hit(&mut self, team: usize, id: u32, amount: f64) {
        // Momentum (topic 4.1): small, and composure decides how much it bites.
        if let Some(i) = self.sides[team].team.players.iter().position(|p| p.id == id) {
            let c = self.sides[team].team.players[i].composure(self.s.rules.level);
            let delta = if amount < 0.0 { amount * 1.5 * (1.25 - c / 20.0) } else { amount * 0.8 };
            let e = &mut self.sides[team].edge[i];
            *e = clamp(*e + delta, -3.0, 2.0);
        }
    }

    fn decay_edges(&mut self) {
        let level = self.s.rules.level;
        for s in self.sides.iter_mut() {
            for (i, e) in s.edge.iter_mut().enumerate() {
                let c = s.team.players[i].composure(level);
                let k = if *e < 0.0 { (0.8 - 0.03 * c).max(0.2) } else { 0.7 };
                *e *= k;
            }
        }
    }

    /// Enforce what happened: fouls, spot, downs, score, clock.
    fn resolve(
        &mut self,
        mut r: PlayRecord,
        res: SnapResult,
        label: &dyn Fn(u32) -> String,
        call: OffCall,
        dcall: DefCall,
    ) -> PlayRecord {
        use scrimmage::Result as R;
        let o = self.poss;
        let d = 1 - o;
        let los = self.x;
        r.duration = res.duration;
        r.passer = res.passer;
        r.target = res.target;
        r.carrier = res.carrier;
        r.air_yards = res.air_yards;
        r.throw_y = res.throw_y;
        r.route = res.route;
        r.yac = res.yac;
        r.before_contact = res.before_contact;
        r.time_to_throw = res.time_to_throw;
        r.separation = res.separation;
        r.throw_away = res.throw_away;
        r.drop = res.drop;
        r.pressured = res.pressured;
        r.out_of_bounds = res.out_of_bounds;
        r.pbu = res.pbu;
        r.tacklers = res.notes.tacklers.clone();
        r.missed_tackles = res.notes.missed.clone();
        // A sack with no rusher on the quarterback (he went down in the
        // pocket to a tackle) goes to the man who tackled him.
        r.sackers = if res.notes.sackers.is_empty() && res.result == R::Sack {
            res.notes.tacklers.iter().take(1).copied().collect()
        } else {
            res.notes.sackers.clone()
        };
        r.hits = res.notes.hits.clone();
        r.pressures = res.notes.pressures.iter().map(|p| p.0).collect();
        r.forced_fumble = res.notes.forced_fumble;
        r.fumbled = res.notes.fumbled;
        r.blocks = res.notes.blocks.clone();
        r.rush_wins = res.notes.rush_wins.clone();
        r.line = res.notes.chart.clone();
        r.tend = res.notes.tend;
        r.busts = res.notes.busts.clone();
        r.injuries = res.notes.injuries.clone();
        r.top_speed = res
            .notes
            .top_speed
            .iter()
            .copied()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, v)| (id, v * 2.045))
            .unwrap_or((0, 0.0));
        r.kind = match res.result {
            R::Run => PlayType::Run,
            R::Complete | R::Incomplete | R::Interception => PlayType::Pass,
            R::Sack => PlayType::Sack,
            R::Scramble => PlayType::Scramble,
            R::Kneel => PlayType::Kneel,
            R::Spike => PlayType::Spike,
        };
        r.complete = res.result == R::Complete;
        r.interception = res.result == R::Interception;
        r.scramble = res.result == R::Scramble;
        if r.interception {
            r.interceptor = res.returner;
        }
        // The nearest defender to the target is the man in coverage.
        if r.kind == PlayType::Pass && !r.throw_away {
            r.defender = res.pbu.or(res.defender);
        }
        // Freelancing results: a jump that made the play, or one that got burned.
        for &(id, _) in &res.notes.freelance {
            let won = r.interceptor == Some(id) || r.pbu == Some(id);
            r.freelance.push((id, won));
            if let Some(i) = self.sides[d].team.players.iter().position(|p| p.id == id) {
                let burned = r.complete && res.spot - los >= 15.0;
                let t = &mut self.sides[d].trust[i];
                if won {
                    *t = (*t + 0.05).min(1.0);
                } else if burned {
                    *t = (*t - 0.08).max(0.0);
                }
            }
        }
        let gain = res.spot - los;
        let mut new_x = res.spot;
        let mut fouls: Vec<Foul> = res.notes.fouls.clone();
        // One foul per team; both teams: offsetting.
        fouls.dedup_by_key(|f| f.side);
        let off_foul = fouls.iter().find(|f| f.side == 0).cloned();
        let def_foul = fouls.iter().find(|f| f.side == 1).cloned();
        let mut replay = false;
        // The play stands with the penalty added: a personal foul after a
        // gain is marked off from where the play ended, and intentional
        // grounding is an incomplete pass.
        let mut stands = false;
        let mut auto_first = false;
        let mut lose_down = false;
        let mut scored_td = res.touchdown;
        let mut turnover = res.turnover;
        let mut safety = res.safety;
        let college = self.below_nfl();
        let ep = |down: u8, togo: f64, x: f64| ep_at(college, down, togo, goal() - x.clamp(1.0, goal() - 1.0));
        if let (Some(of), Some(df)) = (off_foul.as_ref(), def_foul.as_ref()) {
            replay = true;
            new_x = los;
            scored_td = false;
            turnover = false;
            safety = false;
            for f in [of, df] {
                let team = if f.side == 0 { o } else { d };
                r.penalties.push((f.kind, team, f.player, 0.0, true));
            }
        } else if let Some(f) = def_foul {
            let (yds, first) = f.kind.yards();
            let pen_x = match f.kind {
                FoulKind::PassInterference => {
                    let spot_dist = (f.at - los).max(1.0);
                    let lim = if self.rules().level == Level::NFL { spot_dist } else { spot_dist.min(15.0) };
                    (los + lim).min(goal() - 1.0)
                }
                FoulKind::RoughingPasser | FoulKind::FaceMask | FoulKind::UnnecessaryRoughness
                    if !turnover && gain > 0.0 =>
                {
                    res.spot + yds
                }
                _ => los + yds,
            };
            let pen_x = if pen_x >= goal() { los + (goal() - los) / 2.0 } else { pen_x };
            // The offense takes the better of the play and the penalty.
            let play_val = if scored_td {
                7.0
            } else if turnover {
                -ep(1, 10.0, goal() - new_x)
            } else {
                self.value_after(gain, new_x)
            };
            let pen_val = ep(1, 10.0f64.min(goal() - pen_x), pen_x);
            let accept = pen_val > play_val;
            r.penalties.push((f.kind, d, f.player, if accept { pen_x - los } else { 0.0 }, accept));
            if accept {
                stands =
                    matches!(f.kind, FoulKind::RoughingPasser | FoulKind::FaceMask | FoulKind::UnnecessaryRoughness)
                        && !turnover
                        && gain > 0.0;
                new_x = pen_x;
                scored_td = false;
                turnover = false;
                safety = false;
                auto_first = first || pen_x - los >= self.togo;
                replay = !auto_first;
                self.edge_hit(d, f.player, -1.0);
            }
        } else if let Some(f) = off_foul {
            let (yds, _) = f.kind.yards();
            let from = if f.kind == FoulKind::IntentionalGrounding { f.at.min(los) } else { los };
            let pen_x = if f.kind == FoulKind::IntentionalGrounding {
                (los - yds).min(from)
            } else {
                from - yds.min(from / 2.0)
            };
            let play_val = if scored_td {
                7.0
            } else if turnover {
                -ep(1, 10.0, goal() - new_x)
            } else {
                self.value_after(gain, new_x)
            };
            let pen_val =
                ep(self.down + (f.kind == FoulKind::IntentionalGrounding) as u8, self.togo + (los - pen_x), pen_x);
            // The defense takes whichever is worse for the offense.
            let accept = pen_val < play_val;
            r.penalties.push((f.kind, o, f.player, if accept { los - pen_x } else { 0.0 }, accept));
            if accept {
                new_x = pen_x.max(1.0);
                scored_td = false;
                turnover = false;
                safety = false;
                replay = f.kind != FoulKind::IntentionalGrounding;
                lose_down = f.kind == FoulKind::IntentionalGrounding;
                stands = f.kind == FoulKind::IntentionalGrounding;
                self.edge_hit(o, f.player, -1.0);
            }
        }
        let accepted_pen = r.penalties.iter().any(|p| p.4);
        r.nullified = accepted_pen && !stands;
        r.yards = if r.nullified { 0.0 } else { gain };
        if !r.nullified {
            match r.kind {
                PlayType::Pass if !r.complete => r.yards = 0.0,
                PlayType::Pass if r.interception => r.yards = 0.0,
                _ => {}
            }
        }
        if r.interception && !accepted_pen {
            r.yards = 0.0;
        }
        // Momentum and coordinator memory.
        self.decay_edges();
        if !accepted_pen {
            if r.interception {
                if let Some(p) = r.passer {
                    self.edge_hit(o, p, -1.0);
                }
                if let Some(i) = r.interceptor {
                    self.edge_hit(d, i, 1.0);
                }
            }
            if r.drop
                && let Some(t) = r.target
            {
                self.edge_hit(o, t, -1.0);
            }
            for &s in &r.sackers.clone() {
                self.edge_hit(d, s, 1.0);
            }
            // A missed tackle or a fumble can fall to either team (on an
            // interception return the offense tackles and the defense
            // carries), so the hit goes to the team he plays for.
            for &(tk, _) in &r.missed_tackles {
                self.edge_hit(r.side_of(tk).unwrap_or(d), tk, -0.6);
            }
            if let Some(f) = r.fumbled {
                self.edge_hit(r.side_of(f).unwrap_or(o), f, -1.0);
            }
            if scored_td && let Some(c) = r.carrier {
                self.edge_hit(o, c, 1.0);
            }
        }
        // Clock: the play, then whether it keeps running.
        let clock_running_at_snap = self.running;
        let _ = clock_running_at_snap;
        let dur = res.duration;
        self.tick(dur);
        let mut stop =
            matches!(res.result, R::Incomplete | R::Spike) || accepted_pen || turnover || scored_td || safety;
        // Seconds of the usual time between plays that pass before the clock
        // starts again on the referee's ready signal, when it does (trial 8
        // finding 6: the clock waited for the snap after every stoppage, so
        // a stopped play was followed by the next snap 5.6 s of game time on,
        // real 15.2, and NFL teams ran 70 plays a game, real 62).
        let mut ready: Option<f64> = None;
        let late = self.clock_late();
        let nfl = self.rules().level == Level::NFL;
        // Out of bounds at the NFL: the clock starts again once the ball is
        // spotted, except late in a half; elsewhere it waits for the snap.
        if res.out_of_bounds && !turnover {
            stop = true;
            if nfl && !late {
                ready = Some(BALL_SPOTTED);
            }
        }
        // A penalty on a play that would have left the clock running: it
        // starts on the ready signal once the penalty is marked off, with
        // the shorter play clock, except late in a half.
        let play_stops = matches!(res.result, R::Incomplete | R::Spike) || (res.out_of_bounds && !nfl);
        if accepted_pen && !late && !play_stops {
            ready = Some(self.rules().play_clock - ADMIN_PLAY_CLOCK);
        }
        // A celebration (lane 8, C17), on its own keyed draw so no other
        // play changes: from 2011 a college player who taunts or high-steps
        // before the goal line has the touchdown taken off the board, the
        // foul marked from that spot; any level flags an excessive
        // celebration after the score, 15 yards on the kickoff (the NFL
        // loosened its rule in 2017).
        if scored_td && !turnover {
            let scorer = r.carrier.or(if r.complete { r.target } else { None });
            if let Some((ego, id)) = scorer
                .and_then(|id| self.sides[o].team.players.iter().find(|p| p.id == id))
                .map(|p| (p.personality.traits.ego as f64, p.id))
            {
                let showman = ((ego - 8.0).max(0.0) / 6.0).powi(2);
                let u = |k: u64| crate::world::season::key_uniform(self.s.seed ^ 0xCE1E_B8A7, (self.n as u64) << 8 | k);
                let year = if self.s.year == 0 { 2025 } else { self.s.year };
                let college = matches!(self.rules().level, Level::FBS | Level::FCS | Level::SmallCollege);
                let foul_at = 100.0 - (2.0 + 5.0 * u(1)).round();
                if college && year >= 2011 && gain >= 15.0 && u(2) < 0.0012 * showman {
                    scored_td = false;
                    new_x = foul_at - 15.0;
                    auto_first = true;
                    r.penalties.push((FoulKind::UnsportsmanlikeConduct, o, id, -15.0, true));
                    r.why = format!("{} Taunting before the goal line: the touchdown comes off the board.", r.why);
                } else {
                    let base = match self.rules().level {
                        Level::NFL if year < 2017 => 0.012,
                        Level::NFL => 0.004,
                        Level::HS => 0.006,
                        _ => 0.008,
                    };
                    if u(3) < base * showman {
                        self.kick_back = 15.0;
                        r.penalties.push((FoulKind::UnsportsmanlikeConduct, o, id, 15.0, true));
                    }
                }
            }
        }
        // Down and distance.
        let mut first_down = false;
        let fumble_lost = turnover && !r.interception;
        r.fumble_lost = fumble_lost;
        if !turnover && !scored_td && !safety && (auto_first || (!replay && new_x - los >= self.togo)) {
            first_down = true;
        }
        r.first_down = first_down && !accepted_pen || (accepted_pen && auto_first);
        r.touchdown = scored_td;
        r.turnover = turnover;
        r.safety = safety;
        // Spot near the hash.
        let h = self.rules().hash;
        self.ball_y = clamp(res.spot_y, mid() - h, mid() + h);
        if accepted_pen {
            self.ball_y = mid();
        }
        // Why (topic 1): the chain that decided it.
        r.why = self.why(&r, &res, label, &call, &dcall, accepted_pen);
        // The tape (topic 5.1): every real snap goes on both teams' film.
        if matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble) {
            self.res.film[o].offense(&call, r.down);
            self.res.film[d].defense(&dcall);
        }
        // Coordinator memory.
        if matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble) && !accepted_pen {
            let epa_est = self.estimate_epa(&r, new_x, first_down, turnover, scored_td);
            let m = &mut self.sides[o].mem;
            m.saw_defense(&dcall);
            if r.kind == PlayType::Run {
                m.runs += 1.0;
                m.run_epa += epa_est;
            } else {
                m.passes += 1.0;
                m.pass_epa += epa_est;
                if r.air_yards >= 15.0 {
                    m.deeps += 1.0;
                    m.deep_epa += epa_est;
                }
            }
            // Tendencies (topic 7): the look and what came out of it, for
            // this offense's self-scout and the defense's keys; run
            // concepts by result.
            let look = call.formation.look();
            let pass_look = !matches!(call.kind, PlayKind::Run(_) | PlayKind::Rpo(..)) as usize;
            m.own_look[look][pass_look] += 1.0;
            let ran = match call.kind {
                PlayKind::Run(rc) => Some(rc),
                PlayKind::Rpo(rc, _) if r.kind == PlayType::Run => Some(rc),
                _ => None,
            };
            if let Some(rc) = ran {
                m.run_res[rc as usize][0] += epa_est;
                m.run_res[rc as usize][1] += 1.0;
            }
            let md = &mut self.sides[d].mem;
            md.opp_look[look][pass_look] += 1.0;
            if let Some(rc) = ran {
                md.opp_run_res[rc as usize][0] += epa_est;
                md.opp_run_res[rc as usize][1] += 1.0;
            }
            md.opp_plays += 1.0;
            md.opp_game_plays += 1.0;
            if let PlayKind::Pass(i) = call.kind
                && let Some(x) = md.opp_concepts.get_mut(super::playbook::film_concept(i))
            {
                *x += 1.0;
            }
            if r.kind != PlayType::Run {
                md.opp_pass += 1.0;
                if r.air_yards >= 15.0 {
                    md.opp_deep_epa += epa_est;
                }
                if r.time_to_throw < 2.4 {
                    md.opp_quick += 1.0;
                }
            } else {
                md.opp_runs += 1.0;
                md.opp_run_epa += epa_est;
            }
        }
        // A try scores only as a try (its points are added by `pat`): the
        // defense scores two by taking it the other way, except in high
        // school, where the try ends when the defense has the ball.
        if self.trying {
            r.epa = 0.0;
            if turnover && (scored_td || res.defense_td) || res.defense_td {
                r.turnover = true;
                r.touchdown = false;
                if self.rules().level != Level::HS {
                    r.defense_td = true;
                    self.add_points(d, 2);
                }
            }
            self.running = false;
            return r;
        }
        // Score and possession.
        if scored_td {
            r.epa = 6.95 - r.ep_before;
            if turnover {
                // Defensive score.
                r.epa = -6.95 - r.ep_before;
                self.add_points(d, 6);
                self.stats.book(&r);
                self.poss = d;
                self.drive_start_poss = o;
                self.next = Next::Pat(d);
                self.running = false;
                return r;
            }
            self.add_points(o, 6);
            self.stats.book(&r);
            if self.ot && self.rules().overtime == Overtime::NflPeriod {
                // Both teams still get a possession.
            }
            self.next = Next::Pat(o);
            self.running = false;
            return r;
        }
        if res.defense_td {
            r.defense_td = true;
            r.epa = -6.95 - r.ep_before;
            self.add_points(d, 6);
            self.stats.book(&r);
            self.poss = d;
            self.next = Next::Pat(d);
            self.running = false;
            return r;
        }
        if safety {
            r.epa = -2.0 - r.ep_before;
            self.add_points(d, 2);
            self.stats.book(&r);
            self.running = false;
            if self.ot {
                self.drive_start_poss = o;
                self.possession_over(true);
                if self.over || !self.ot_kickoffs() {
                    return r;
                }
            }
            self.next = Next::Kickoff { team: o, from: 20.0, onside: false };
            return r;
        }
        if turnover {
            // The other team's ball where it ended (in its own frame).
            let their_x = goal() - new_x;
            let their_x = if their_x <= 0.0 { 20.0 } else { their_x };
            r.epa = -ep(1, 10.0, their_x) - r.ep_before;
            self.stats.book(&r);
            self.change_possession(their_x, true);
            return r;
        }
        if replay {
            self.x = new_x.clamp(1.0, goal() - 1.0);
            self.togo = (self.togo + (los - self.x)).max(0.5);
            if self.togo > goal() - self.x {
                self.togo = goal() - self.x;
            }
            r.epa = ep(self.down, self.togo, self.x) - r.ep_before;
            self.stats.book(&r);
            self.running = ready.is_some();
            self.after_snap_clock(&r, ready);
            return r;
        }
        self.x = new_x.clamp(0.5, goal() - 0.5);
        if first_down {
            self.down = 1;
            self.togo = 10.0f64.min(goal() - self.x);
            // The clock stops to set the chains and starts again on the
            // ready signal.
            let chains = match self.rules().first_down_stop {
                FirstDownStop::Always => true,
                FirstDownStop::LastTwoMinutes => self.half_left() <= 120.0,
                FirstDownStop::Never => false,
            };
            if chains && !stop {
                stop = true;
                ready = Some(BALL_SPOTTED);
            }
        } else {
            self.down = self.down + 1 + lose_down as u8;
            self.togo -= new_x - los;
            if self.down > self.rules().downs() {
                // Turnover on downs.
                r.epa = -ep(1, 10.0, goal() - self.x) - r.ep_before;
                self.stats.book(&r);
                self.ball_y = mid();
                self.change_possession(goal() - self.x, false);
                return r;
            }
        }
        r.epa = ep(self.down, self.togo, self.x) - r.ep_before;
        self.stats.book(&r);
        if self.mercy_running() {
            stop = false;
            ready = None;
        }
        // The other leagues' clocks (X7): arena football's runs through
        // incompletions and out of bounds except in each half's last
        // minute; after the CFL's three-minute warning it stops after
        // every play and starts on the snap.
        let code = self.rules().code;
        if code.running_clock > 0.0
            && self.half_left() > code.running_clock
            && !turnover
            && !scored_td
            && !safety
            && !accepted_pen
        {
            stop = false;
            ready = None;
        }
        // The CFL: before the three-minute warning a stopped clock starts
        // again on the referee's whistle (after an incomplete pass or a
        // play out of bounds too); after it the clock stops after every
        // play, starting on the whistle after a play in bounds and on the
        // snap after one that would have stopped it anyway.
        if code.late_every_play && !turnover && !scored_td && !safety && !accepted_pen {
            let late = (self.quarter == 2 || self.quarter >= 4) && self.half_left() <= code.warning;
            if !late && stop && ready.is_none() {
                ready = Some(BALL_SPOTTED);
            } else if late && !stop {
                stop = true;
                ready = Some(BALL_SPOTTED);
            }
        }
        self.running = !stop || ready.is_some();
        self.after_snap_clock(&r, ready);
        r
    }

    /// Time from the end of one play to the next snap, and timeouts. `ready`:
    /// the seconds of it before the clock starts again on the ready signal.
    fn after_snap_clock(&mut self, _r: &PlayRecord, ready: Option<f64>) {
        let o = self.poss;
        let sit = self.situation();
        // Defense trailing late: stop the clock with a timeout.
        if self.running && !self.ot && (self.quarter == 4 || self.quarter == 2) {
            let d = 1 - o;
            let trailing =
                self.diff(d) < 0 || (self.quarter == 2 && self.half_left() < 90.0 && self.diff(d) <= 0 && false);
            let gm = self.sides[d].team.staff.game_management;
            if trailing
                && !self.sides[d].coach.read.concede
                && self.quarter == 4
                && self.half_left() < 180.0
                && self.sides[d].timeouts > 0
                && self.rng.chance(clamp(0.4 + 0.03 * gm, 0.4, 0.98))
            {
                self.sides[d].timeouts -= 1;
                self.running = false;
                self.recover(0.02);
                let left = self.sides[d].timeouts;
                self.log(d, format!("Timeout to stop the clock ({left} left)."));
                return;
            }
        }
        self.pace = 1.0;
        self.no_huddle = false;
        if !self.running {
            // Clock restarts on the snap (or on the ready signal for some stops).
            return;
        }
        let gap = if sit.hurry() {
            // Offense trailing late uses a timeout after an in-bounds play.
            if self.sides[o].timeouts > 0 && self.half_left() < 75.0 && self.rng.chance(0.75) {
                self.sides[o].timeouts -= 1;
                self.running = false;
                let left = self.sides[o].timeouts;
                self.log(o, format!("Timeout to stop the clock ({left} left)."));
                return;
            }
            self.no_huddle = true;
            13.0
        } else if self.four_minute_rule(o) == Some(live::FourMinute::RunItOut) && super::sheet::four_minute(&sit) {
            39.5
        } else if (self.quarter == 4
            && self.diff(o) > 0
            && !(self.four_minute_rule(o) == Some(live::FourMinute::KeepAttacking) && super::sheet::four_minute(&sit)))
            || self.sides[o].coach.read.ease > 0.0
            || (self.quarter == 2 && self.half_left() < 120.0 && self.x < 50.0 && self.diff(o) >= 0)
        {
            38.0
        } else if self.user_is(o) && self.tempo != Tempo::Normal {
            // Your tempo (topic 5).
            if self.tempo == Tempo::Hurry {
                self.no_huddle = true;
                24.0
            } else {
                39.0
            }
        } else {
            // Tempo (S10): up-tempo systems go to the line without a huddle
            // on about a third of their snaps, everyone else now and then
            // (real NFL about 7% outside the two-minute drill); the averages
            // stay 33 and 37.5 seconds (proposed, not confirmed).
            let staff = &self.sides[o].team.staff;
            let (share, huddle) = match staff.off_scheme {
                OffScheme::AirRaid | OffScheme::SpreadOption => (0.35, 38.0),
                _ => (0.06, 38.2),
            };
            // The head coach's tempo (update 2, G2), by its firmness.
            let share = staff.guide.pace.pull().map_or(share, |(p, k)| share + k * (p.share() - share));
            if self.rng.chance(share) {
                self.no_huddle = true;
                24.0 + self.rng.range(-2.0, 2.0)
            } else {
                huddle + self.rng.range(-3.0, 3.0)
            }
        };
        // The other leagues' play clocks (X7): the CFL's 20 seconds from
        // the whistle, the spring leagues' 25 from the ready signal.
        let gap = if self.rules().code.american() { gap } else { gap.min(self.rules().play_clock - 1.0) };
        self.pace = gap / 37.5;
        self.tick((gap - ready.unwrap_or(0.0)).max(0.0));
    }

    fn value_after(&self, gain: f64, new_x: f64) -> f64 {
        let college = self.below_nfl();
        let ep = |down: u8, togo: f64, x: f64| ep_at(college, down, togo, goal() - x.clamp(1.0, goal() - 1.0));
        if gain >= self.togo {
            ep(1, 10.0f64.min(goal() - new_x), new_x)
        } else if self.down >= self.rules().downs() {
            -ep(1, 10.0, goal() - new_x)
        } else {
            ep(self.down + 1, self.togo - gain, new_x)
        }
    }

    fn estimate_epa(&self, r: &PlayRecord, new_x: f64, first: bool, turnover: bool, td: bool) -> f64 {
        let college = self.below_nfl();
        let ep = |down: u8, togo: f64, x: f64| ep_at(college, down, togo, goal() - x.clamp(1.0, goal() - 1.0));
        let after = if td {
            6.95
        } else if turnover {
            -ep(1, 10.0, goal() - new_x)
        } else if first {
            ep(1, 10.0, new_x)
        } else if self.down >= self.rules().downs() {
            -ep(1, 10.0, goal() - new_x)
        } else {
            ep(self.down + 1, (self.togo - (new_x - self.x)).max(1.0), new_x)
        };
        after - r.ep_before
    }

    #[allow(clippy::too_many_arguments)]
    fn why(
        &self,
        r: &PlayRecord,
        res: &SnapResult,
        label: &dyn Fn(u32) -> String,
        call: &OffCall,
        dcall: &DefCall,
        pen: bool,
    ) -> String {
        use scrimmage::Result as R;
        let mut w;
        let yd = |y: f64| format!("{:.0}", y);
        match res.result {
            R::Sack => {
                let s = r.sackers.first().copied();
                if let Some(s) = s {
                    let beat = res.notes.blocks.iter().find(|b| b.1 == s && !b.3);
                    let mv = res.notes.rush_wins.iter().find(|x| x.0 == s);
                    w = match (beat, mv) {
                        (Some(b), Some(m)) => format!(
                            "Sack by {}: he beat {} with a {} in {:.1}s, and the quarterback held it {:.1}s.",
                            label(s),
                            label(b.0),
                            m.1.name(),
                            m.2,
                            res.time_to_throw
                        ),
                        (Some(b), None) => format!(
                            "Sack by {}: he got off {}'s block after {:.1}s while nobody came open.",
                            label(s),
                            label(b.0),
                            b.4
                        ),
                        _ => format!(
                            "Sack by {}: he came free ({}), and the quarterback held it {:.1}s.",
                            label(s),
                            dcall.pressure.name(),
                            res.time_to_throw
                        ),
                    };
                } else {
                    w = format!("Sacked after {:.1}s.", res.time_to_throw);
                }
            }
            R::Interception => {
                let i = res.returner.map(label).unwrap_or_default();
                w = format!(
                    "Intercepted by {} on {} against {}: thrown into {:.1} yd of separation{}.",
                    i,
                    call.describe(),
                    dcall.coverage.name(),
                    res.separation,
                    if res.pressured { " under pressure" } else { "" }
                );
                if r.freelance.iter().any(|f| f.1) {
                    w.push_str(" He jumped the route.");
                }
            }
            R::Complete => {
                let t = res.target.map(label).unwrap_or_default();
                w = format!(
                    "Complete to {} for {} on {} against {} ({:.1} yd of separation, {} after the catch).",
                    t,
                    yd(r.yards),
                    call.describe(),
                    dcall.coverage.name(),
                    res.separation,
                    yd(res.yac)
                );
                if let Some(&(tk, _)) = res.notes.missed.first() {
                    w.push_str(&format!(" {} missed a tackle.", label(tk)));
                }
            }
            R::Incomplete => {
                let t = res.target.map(label).unwrap_or_default();
                w = if res.throw_away {
                    "Thrown away: nobody open and the pocket closing.".to_string()
                } else if res.drop {
                    format!("Dropped by {}.", t)
                } else if let Some(p) = res.pbu {
                    format!("Broken up by {} on a throw to {}.", label(p), t)
                } else {
                    format!("Incomplete to {}: off target.", t)
                };
            }
            R::Run | R::Scramble => {
                let c = res.carrier.map(label).unwrap_or_default();
                let what = if res.result == R::Scramble { "Scramble".to_string() } else { call.describe() };
                w = format!("{} by {} for {}", what, c, yd(r.yards));
                if r.yards <= 0.0 {
                    if let Some(t) = r.tacklers.first() {
                        let shed = res.notes.blocks.iter().find(|b| b.1 == *t && !b.3);
                        match shed {
                            Some(b) => w.push_str(&format!(": {} shed {} and made the stop.", label(*t), label(b.0))),
                            None => w.push_str(&format!(": {} came free and made the stop.", label(*t))),
                        }
                    } else {
                        w.push('.');
                    }
                } else {
                    w.push_str(&format!(" ({} before contact", yd(res.before_contact.max(0.0))));
                    let broken = res.notes.missed.len();
                    if broken > 0 {
                        w.push_str(&format!(", {} broken tackle{}", broken, if broken > 1 { "s" } else { "" }));
                    }
                    w.push_str(").");
                }
            }
            R::Kneel => w = "Kneel.".into(),
            R::Spike => w = "Spiked to stop the clock.".into(),
        }
        if let Some(f) = res.notes.fumbled {
            w.push_str(&format!(
                " Fumble by {}{}.",
                label(f),
                match res.notes.recovered {
                    Some((s, id)) =>
                        format!(", recovered by {} ({})", label(id), if s == 0 { "offense" } else { "defense" }),
                    None => String::new(),
                }
            ));
        }
        if !res.notes.busts.is_empty() {
            w.push_str(&format!(" Busted assignment: {}.", label(res.notes.busts[0])));
        }
        if pen {
            for p in &r.penalties {
                if p.4 {
                    w.push_str(&format!(
                        " Penalty: {} on {}, {:.0} {}.",
                        p.0.name(),
                        label(p.2),
                        p.3,
                        crate::world::folk::noun(p.3.round_ties_even() as i64, "yard")
                    ));
                }
            }
        }
        w
    }
}

/// Tests only: the field goal kicker, the punter and the long snapper a
/// team sends out with the players at `out` (indexes) hurt.
#[cfg(test)]
pub(crate) fn kick_unit_men(team: &GameTeam, out: &[usize]) -> [usize; 3] {
    let mut side = Side::new(team, 99.0, 3);
    for &i in out {
        side.out[i] = true;
    }
    [side.fg_kicker(), side.punter(), side.snapper()]
}

/// A kick unit of `n` with some men hurt and some pulled in a decided game.
#[cfg(test)]
pub(crate) fn kick_unit_with(
    team: &GameTeam,
    out: &[usize],
    benched: &[usize],
    receiving: bool,
    n: usize,
) -> Vec<usize> {
    let mut side = Side::new(team, 99.0, 3);
    for &i in out {
        side.out[i] = true;
    }
    for &i in benched {
        side.benched[i] = true;
    }
    side.kick_unit(receiving, n)
}

// ------------------------------------------------------------ game day control

impl<'a> Game<'a> {
    /// Your team in this game (9 when nobody plays it live).
    fn ctl_team(&self) -> usize {
        self.ctl.as_ref().map_or(9, |c| c.team)
    }

    fn user_is(&self, team: usize) -> bool {
        self.ctl.as_ref().is_some_and(|c| c.team == team)
    }

    /// Keep this play's tracking: your games, and in a live game the snaps
    /// since you last decided (the 2D view of the play you just called).
    fn recording(&self) -> bool {
        self.s.record || self.ctl.as_ref().is_some_and(|c| c.record_from > 0 && self.n >= c.record_from)
    }

    fn log(&mut self, team: usize, what: String) {
        self.res.log.push((self.n, team, what));
    }

    fn stop_ctx(&self) -> StopCtx {
        let yl = goal() - self.x;
        StopCtx {
            down: self.down,
            yardline_100: yl,
            half_left: self.half_left(),
            quarter: self.quarter,
            goal_to_go: yl <= 10.0 && self.togo >= yl - 0.01,
            trailing: self.ctl_team() < 2 && self.diff(self.ctl_team()) < 0,
        }
    }

    /// A point where you may decide: your answer if you gave one here, or
    /// None (the staff decides; if the point stops for you, `probe` is set
    /// and the game stops once the staff's choice is known). `again`: the
    /// same snap asks again after a timeout or a word to a coach.
    fn ask(&mut self, ask: Ask, again: bool) -> Option<Input> {
        if self.paused {
            return None;
        }
        let sc = self.stop_ctx();
        let ctl = self.ctl.as_mut()?;
        let a = ctl.asked;
        ctl.asked += 1;
        if let Some(i) = ctl.input_at(a) {
            return Some(i.clone());
        }
        if a < ctl.frontier || !(again || ctl.stops(ask, &sc)) {
            return None;
        }
        ctl.pause_at = Some(a);
        self.probe = Some(ask);
        None
    }

    /// Stop the game here for you.
    fn hold(&mut self, p: Pending) {
        if let Some(c) = self.ctl.as_mut() {
            c.pending = Some(p);
        }
        self.paused = true;
        self.probe = None;
    }

    /// Where the game stands, for the screen.
    pub(crate) fn view(&self) -> View {
        let len = 4.0 * self.rules().quarter;
        let o = self.poss;
        let ep = ep_at(self.below_nfl(), self.down.max(1), self.togo, goal() - self.x);
        let lead = self.diff(0) as f64;
        let left = if self.ot { 300.0f64.min(self.clock) } else { self.game_left() };
        let wp_home = if self.over {
            match self.diff(0) {
                d if d > 0 => 1.0,
                d if d < 0 => 0.0,
                _ => 0.5,
            }
        } else {
            live::win_prob(lead, if o == 0 { ep } else { -ep }, left, len)
        };
        View {
            quarter: self.quarter,
            clock: if self.ot && !self.ot_timed() { 0.0 } else { self.clock.max(0.0) },
            overtime: self.ot,
            poss: o,
            x: self.x,
            down: self.down,
            togo: self.togo,
            ball_y: self.ball_y,
            score: [self.stats.team[0].points, self.stats.team[1].points],
            timeouts: [self.sides[0].timeouts, self.sides[1].timeouts],
            challenges: self.challenges,
            running: self.running,
            wp_home,
            plays: self.n,
            tempo: self.tempo,
            wind_home: self.wind_along(0),
        }
    }

    /// A decision for you, with the numbers your staff would give.
    fn pending(&self, ask: Ask) -> Pending {
        let mut p = Pending {
            ask,
            view: self.view(),
            staff_fourth: None,
            staff_off: None,
            staff_def: None,
            staff_two: false,
            staff_onside: false,
            staff_toss: None,
            staff_challenge: false,
            off_options: Vec::new(),
            def_options: Vec::new(),
            their_personnel: None,
            advice: Vec::new(),
            ruling: None,
            fourth_wp: None,
            fg_make: None,
        };
        let o = self.poss;
        if ask == Ask::Snap {
            let yl = goal() - self.x;
            let dist = yl + kick_add();
            if dist <= self.kicker_range(o) + 6.0 {
                p.fg_make = Some(self.fg_make_estimate(o, dist));
            }
            if self.down == 4 {
                let make = p.fg_make.unwrap_or(0.0);
                let (conv, go, punt, fg) = calling::fourth_values(&self.situation(), make);
                let togo =
                    if self.togo >= yl - 0.01 { "goal".to_string() } else { format!("{:.0}", self.togo.max(1.0)) };
                if self.rules().level == Level::HS {
                    p.advice.push(format!(
                        "No analyst on a high school staff: the book says 4th and {togo} converts about {:.0}% of the time.",
                        100.0 * conv
                    ));
                } else {
                    // The analytics staff's numbers (Q37), with its error:
                    // a sharper staff is closer to the truth (proposed, not
                    // confirmed: game management stands in for the analyst).
                    let len = 4.0 * self.rules().quarter;
                    let left = if self.ot { 300.0 } else { self.game_left() };
                    let lead = self.diff(o) as f64;
                    let gm = self.sides[o].team.staff.game_management;
                    let mut nr = Rng::new((self.s.seed as u32) ^ self.n.wrapping_mul(0x2C1B_3C6D) ^ 0xA5A5);
                    let sd = 0.025 * (1.3 - gm / 20.0);
                    let mut wp =
                        |ev: f64| clamp(live::win_prob(lead, ev, left, len) + nr.normal(0.0, sd), 0.001, 0.999);
                    let w = [wp(go), wp(punt), if make > 0.05 { wp(fg) } else { 0.0 }];
                    p.fourth_wp = Some(w);
                    let kick = if make > 0.05 { format!(", kick {:.0}%", 100.0 * w[2]) } else { String::new() };
                    p.advice.push(format!(
                        "Analytics, win chance: go {:.0}%, punt {:.0}%{kick}. Teams convert 4th and {togo} about {:.0}% of the time.",
                        100.0 * w[0],
                        100.0 * w[1],
                        100.0 * conv
                    ));
                }
            }
        }
        p
    }

    /// The pieces of the offense's call context on this snap.
    fn off_ctx(&self) -> (calling::OffCtx, f64) {
        let o = self.poss;
        let has_fb = self.sides[o].depth.list(Position::RB).len() >= 2;
        let te_count = self.sides[o].depth.list(Position::TE).iter().filter(|&&i| !self.sides[o].out[i]).count();
        let qb = self.sides[o].depth.list(Position::QB).iter().copied().find(|&i| !self.sides[o].out[i]);
        let qb_threat = qb.map_or(0.0, |i| qb_run(&self.sides[o].team.players[i]));
        let ctx = calling::OffCtx {
            has_fb,
            te_count,
            qb_run: qb_threat,
            level: self.s.rules.level,
            plan: self.sides[o].team.plan().0,
            designs: calling::sheet_designs(&self.sides[o].team.prep),
            burner: self.sides[o].burner_lean(),
        };
        (ctx, qb_threat)
    }

    /// The play caller's other calls from this box, in his order of
    /// preference (sending a call back gives the next one). Drawn on a
    /// stream of their own, so asking never moves the game.
    fn off_options(&self, sit: &Situation, ctx: &calling::OffCtx, staff_call: &OffCall) -> Vec<OffCall> {
        let o = self.poss;
        let staff = self.sides[o].team.staff;
        let base = (self.s.seed as u32) ^ self.n.wrapping_mul(0x9E37_79B9) ^ 0x0FF5_EA11;
        let mut out: Vec<OffCall> = Vec::new();
        let mut seen: Vec<(PlayKind, Personnel, usize)> = vec![(staff_call.kind, staff_call.formation.personnel, 1)];
        for j in 0..40u32 {
            let mut r = Rng::new(base.wrapping_add(j.wrapping_mul(0x85EB_CA6B)));
            let c = calling::call_offense(&staff, &self.sides[o].mem, sit, ctx, &mut r);
            if let Some(x) = seen.iter_mut().find(|x| x.0 == c.kind && x.1 == c.formation.personnel) {
                x.2 += 1;
                if let Some(k) =
                    out.iter().position(|y| y.kind == c.kind && y.formation.personnel == c.formation.personnel)
                {
                    let _ = k;
                }
                continue;
            }
            seen.push((c.kind, c.formation.personnel, 1));
            out.push(c);
        }
        // Most often drawn first: his preference.
        out.sort_by_key(|c| {
            std::cmp::Reverse(seen.iter().find(|x| x.0 == c.kind && x.1 == c.formation.personnel).map_or(0, |x| x.2))
        });
        out.truncate(8);
        out
    }

    fn def_options(&self, sit: &Situation, call: &OffCall, staff_call: &DefCall, qb_run: f64) -> Vec<DefCall> {
        let d = 1 - self.poss;
        let staff = self.sides[d].team.staff;
        let dctx = calling::DefCtx { qb_run, tip: 0.0 };
        let base = (self.s.seed as u32) ^ self.n.wrapping_mul(0x7FEB_352D) ^ 0x0DEF_0BEE;
        let mut seen: Vec<(DefCall, usize)> = vec![(*staff_call, 1)];
        for j in 0..40u32 {
            let mut r = Rng::new(base.wrapping_add(j.wrapping_mul(0x846C_A68B)));
            let c = calling::call_defense(&staff, &self.sides[d].mem, sit, &call.formation, &dctx, &mut r);
            match seen.iter_mut().find(|x| x.0.coverage == c.coverage && x.0.pressure == c.pressure) {
                Some(x) => x.1 += 1,
                None => seen.push((c, 1)),
            }
        }
        let mut out: Vec<(DefCall, usize)> = seen.into_iter().skip(1).collect();
        out.sort_by_key(|x| std::cmp::Reverse(x.1));
        out.into_iter().take(8).map(|x| x.0).collect()
    }

    /// The two-point calls your play caller has for this try.
    /// The plays your coordinator would call from here when his own choice
    /// is not a play (a try, a kick, a kneel or a spike).
    fn go_options(&self) -> Vec<OffCall> {
        let sit = self.situation();
        let (ctx, _) = self.off_ctx();
        let none = OffCall::new(Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0), PlayKind::Kneel, 1.0, false);
        self.off_options(&sit, &ctx, &none)
    }

    /// The model's win chance of going, punting and kicking on this 4th
    /// down (no analyst's error), and the chance to convert.
    fn fourth_model(&self, o: usize, make: f64) -> [f32; 4] {
        let (conv, go, punt, fg) = calling::fourth_values(&self.situation(), make);
        let len = 4.0 * self.rules().quarter;
        let left = if self.ot { 300.0 } else { self.game_left() };
        let lead = self.diff(o) as f64;
        let wp = |ev: f64| live::win_prob(lead, ev, left, len) as f32;
        [wp(go), wp(punt), if make > 0.05 { wp(fg) } else { 0.0 }, conv as f32]
    }

    /// Your timeout.
    fn user_timeout(&mut self, team: usize) {
        if self.sides[team].timeouts == 0 {
            self.log(team, "No timeouts left.".into());
            return;
        }
        self.sides[team].timeouts -= 1;
        self.running = false;
        self.recover(0.02);
        let left = self.sides[team].timeouts;
        self.log(team, format!("Your timeout ({left} left)."));
    }

    /// A word to a coordinator: the matchup plan changes.
    fn apply_plan(&mut self, team: usize, pc: live::PlanChange) {
        if team > 1 {
            return;
        }
        let ids: Vec<u32> = self.sides[1 - team].team.players.iter().map(|p| p.id).collect();
        let id = |i: usize| ids.get(i).copied().unwrap_or(0);
        let mut said = Vec::new();
        let plan = &mut self.sides[team].mem.plan;
        if let Some(t) = pc.target_cb {
            plan.target_cb = t;
            said.push(t.map_or("stop picking on one corner".to_string(), |i| format!("attack #{}", id(i))));
        }
        if let Some(b) = pc.bracket {
            plan.bracket = b;
            said.push(b.map_or("no more bracket".to_string(), |i| format!("bracket #{}", id(i))));
        }
        if let Some(sp) = pc.spy {
            plan.spy = sp;
            said.push(if sp { "spy the quarterback".into() } else { "no spy".into() });
        }
        if let Some(k) = pc.kick_away {
            plan.vs.kick_deep = if k { 1.0 } else { 0.0 };
            said.push(if k { "kick away from their returner".into() } else { "kick it deep".into() });
        }
        if let Some(d) = pc.draws {
            plan.vs.draws = if d { plan.vs.draws.max(0.7) } else { 0.0 };
            said.push(if d { "draws and screens against their upfield rush".into() } else { "no extra draws".into() });
        }
        if !said.is_empty() {
            self.log(team, format!("Plan: {}.", said.join(", ")));
        }
    }

    /// May this team kick onside now? The NFL's kickoff from 2024 on asks
    /// for it to be declared, and only by a team that trails (in 2024 only
    /// in the fourth quarter).
    fn onside_allowed(&self, team: usize) -> bool {
        if self.rules().level != Level::NFL || !self.rules().nfl_kickoff {
            return true;
        }
        let year = self.sides[team].team.staff.year;
        let trailing = self.diff(team) < 0;
        match year {
            y if y >= 2025 => trailing,
            2024 => trailing && self.quarter >= 4,
            _ => true,
        }
    }

    /// The coin toss (topic 3): the winner chooses. AI coaches choose by
    /// habit; you choose when you win it. `drawn` is the team the old flip
    /// sent the ball to: the winner is drawn so that, when the coaches'
    /// habits allow, the same team receives (so a seed's games mostly play
    /// as before). Returns the team that receives first.
    fn coin_toss(&mut self, drawn: usize) -> usize {
        let level = self.rules().level;
        let year = self.s.home.staff.year;
        let gm = [self.sides[0].team.staff.game_management, self.sides[1].team.staff.game_management];
        let ch = [
            live::ai_toss(level, year, gm[0], &mut self.toss_rng),
            live::ai_toss(level, year, gm[1], &mut self.toss_rng),
        ];
        let a = ch[drawn] == TossChoice::Receive;
        let b = ch[1 - drawn] == TossChoice::Defer;
        let coin = self.toss_rng.chance(0.5);
        let winner = match (a, b) {
            (true, false) => drawn,
            (false, true) => 1 - drawn,
            _ if coin => drawn,
            _ => 1 - drawn,
        };
        let mut choice = ch[winner];
        if self.user_is(winner) {
            match self.ask(Ask::Toss, false) {
                Some(Input::Toss(c)) if !matches!(c, TossChoice::DefenseFirst | TossChoice::OffenseFirst) => {
                    choice = c;
                    if choice == TossChoice::Defer && level == Level::NFL && year < 2008 {
                        choice = TossChoice::Receive;
                    }
                }
                Some(_) => {}
                None if self.probe == Some(Ask::Toss) => {
                    let mut p = self.pending(Ask::Toss);
                    p.staff_toss = Some(choice);
                    p.advice.push(self.toss_advice());
                    if level == Level::NFL && year < 2008 {
                        p.advice.push("Deferring is not allowed in the NFL before 2008.".into());
                    }
                    self.hold(p);
                    return drawn;
                }
                None => {}
            }
        }
        self.res.toss = Some((winner, choice));
        let name = self.sides[winner].team.name.clone();
        let receiver = match choice {
            TossChoice::Receive | TossChoice::OffenseFirst => winner,
            TossChoice::Goal => {
                self.flipped = self.s.cond.wind_along * if winner == 0 { 1.0 } else { -1.0 } < 0.0;
                1 - winner
            }
            _ => 1 - winner,
        };
        let what = match choice {
            TossChoice::Receive => "will receive",
            TossChoice::Defer => "defers to the second half",
            TossChoice::Kick => "will kick",
            TossChoice::Goal => "takes the wind",
            _ => "chooses",
        };
        self.res.log.push((0, winner, format!("{name} won the toss and {what}.")));
        receiver
    }

    fn toss_advice(&self) -> String {
        let w = self.s.cond.wind_along.abs();
        if w >= 10.0 {
            format!(
                "A {:.0} mph wind blows along the field. Taking it gives you the wind at your back in the first and third quarters; deferring lets you choose again in the second half.",
                self.s.cond.wind
            )
        } else if self.rules().level == Level::NFL && self.s.home.staff.year < 2008 {
            "Little wind today. Most winners take the ball.".into()
        } else {
            "Little wind today. Most winners defer, to get the ball to start the second half.".into()
        }
    }

    /// The overtime toss: NFL winners mostly receive, college and high
    /// school winners take defense first. Returns the team with the first
    /// possession (NFL: the team that receives).
    fn ot_toss(&mut self, drawn: usize) -> usize {
        let level = self.rules().level;
        let ch = [live::ai_ot_toss(level, &mut self.toss_rng), live::ai_ot_toss(level, &mut self.toss_rng)];
        let gives = |t: usize, c: TossChoice| -> usize {
            match c {
                TossChoice::Receive | TossChoice::OffenseFirst => t,
                _ => 1 - t,
            }
        };
        let a = gives(drawn, ch[drawn]) == drawn;
        let b = gives(1 - drawn, ch[1 - drawn]) == drawn;
        let coin = self.toss_rng.chance(0.5);
        let winner = match (a, b) {
            (true, false) => drawn,
            (false, true) => 1 - drawn,
            _ if coin => drawn,
            _ => 1 - drawn,
        };
        let mut choice = ch[winner];
        if self.user_is(winner) {
            match self.ask(Ask::Toss, false) {
                Some(Input::Toss(c)) => choice = c,
                Some(_) => {}
                None if self.probe == Some(Ask::Toss) => {
                    let mut p = self.pending(Ask::Toss);
                    p.staff_toss = Some(choice);
                    p.advice.push(if level == Level::NFL {
                        "Overtime: both teams get the ball (since 2025 in the regular season too), then the next score wins.".into()
                    } else {
                        "Overtime from the 25: going second, you know what you need.".into()
                    });
                    self.hold(p);
                    return drawn;
                }
                None => {}
            }
        }
        let name = self.sides[winner].team.name.clone();
        self.log(winner, format!("{name} won the overtime toss."));
        gives(winner, choice)
    }

    /// What a ruling is worth to the offense, in expected points.
    fn ruling_value(&self, r: &SnapResult) -> f64 {
        // The tables are in yards to the goal on any field.
        let college = self.below_nfl();
        let ep = |down: u8, togo: f64, x: f64| ep_at(college, down, togo, goal() - x.clamp(1.0, goal() - 1.0));
        if r.touchdown && !r.turnover {
            return 6.95;
        }
        if r.defense_td {
            return -6.95;
        }
        if r.turnover {
            return -ep_at(self.below_nfl(), 1, 10.0, r.spot.clamp(1.0, goal() - 1.0));
        }
        let spot = if r.result == scrimmage::Result::Incomplete { self.x } else { r.spot };
        let line = self.x + self.togo;
        if spot >= line {
            ep(1, 10.0f64.min(goal() - spot), spot)
        } else if self.down >= 4 {
            -ep_at(self.below_nfl(), 1, 10.0, spot.clamp(1.0, goal() - 1.0))
        } else {
            ep(self.down + 1, line - spot, spot)
        }
    }

    /// The officials' ruling on a snap (topic 6): on a close call the crew
    /// can rule it wrong, at real rates by its eye; the booth, replay assist
    /// or a coach's challenge may set it right, by each level's rules and
    /// era. Every draw is on the officials' own stream.
    fn officiate(&mut self, truth: SnapResult, r: &mut PlayRecord) -> SnapResult {
        use scrimmage::Result as R;
        let o = self.poss;
        let d = 1 - o;
        let los = self.x;
        let line = los + self.togo;
        let mut alt = truth.clone();
        let carrier_or_target = truth.carrier.or(truth.target);
        // (what, chance it is close, truth favours the offense)
        let cand: Option<(CloseCall, f64, bool)> = if truth.result == R::Interception {
            alt.result = R::Incomplete;
            alt.turnover = false;
            alt.defense_td = false;
            alt.returner = None;
            alt.spot = los;
            Some((CloseCall::Interception, 0.15, false))
        } else if truth.turnover && truth.notes.fumbled.is_some() {
            alt.turnover = false;
            alt.defense_td = false;
            alt.returner = None;
            alt.notes.fumbled = None;
            alt.notes.recovered = None;
            let ahead =
                if truth.result == R::Complete { truth.air_yards + 1.0 } else { truth.before_contact.max(0.0) + 1.5 };
            alt.spot = (los + ahead).clamp(los - 10.0, (goal() - 0.6).max(los - 10.0));
            Some((CloseCall::Fumble, 0.5, false))
        } else if truth.touchdown && !truth.turnover && !truth.defense_td {
            alt.touchdown = false;
            alt.spot = goal() - 0.6;
            Some((CloseCall::GoalLine, if los >= goal() - 5.0 { 0.2 } else { 0.05 }, true))
        } else if truth.result == R::Complete {
            alt.result = R::Incomplete;
            alt.spot = los;
            alt.out_of_bounds = false;
            alt.carrier = None;
            alt.yac = 0.0;
            alt.turnover = false;
            alt.notes.fumbled = None;
            let p = if truth.out_of_bounds {
                0.16
            } else if truth.air_yards >= 15.0 && truth.separation < 1.0 {
                0.10
            } else {
                0.02
            };
            Some((CloseCall::Catch, p, true))
        } else if truth.result == R::Incomplete && !truth.throw_away && truth.target.is_some() {
            alt.result = R::Complete;
            alt.carrier = truth.target;
            alt.spot = (los + truth.air_yards).min(goal());
            alt.touchdown = alt.spot >= goal();
            alt.yac = 0.0;
            Some((CloseCall::Catch, 0.05, false))
        } else if !truth.touchdown && !truth.turnover && (goal() - 1.5..goal()).contains(&truth.spot) {
            alt.touchdown = true;
            alt.spot = goal();
            Some((CloseCall::GoalLine, 0.25, false))
        } else if matches!(truth.result, R::Run | R::Complete | R::Scramble)
            && !truth.turnover
            && (truth.spot - line).abs() < 0.5
            && line < goal()
        {
            // A spot against the line to gain matters most on 3rd and 4th
            // down, but a first down is a first down on any.
            let over = truth.spot >= line;
            alt.spot = if over { line - 0.4 } else { line + 0.1 };
            Some((CloseCall::Spot, if self.down >= 3 { 0.5 } else { 0.3 }, over))
        } else if matches!(truth.result, R::Run | R::Complete | R::Scramble)
            && !truth.turnover
            && !truth.touchdown
            && !truth.out_of_bounds
            && !truth.notes.tacklers.is_empty()
        {
            alt.turnover = true;
            alt.returner = truth.notes.tacklers.first().copied();
            alt.notes.fumbled = carrier_or_target;
            Some((CloseCall::Fumble, 0.003, true))
        } else {
            None
        };
        let Some((call, p_close, fav_off)) = cand else { return truth };
        if !self.refs.chance(p_close) {
            return truth;
        }
        let wrong = self.refs.chance(live::WRONG_SHARE * self.eye.miss);
        let against = if wrong == fav_off { o } else { d };
        let (ruled, counter) = if wrong { (alt, truth) } else { (truth, alt) };
        let rp = self.replay;
        let nfl = self.rules().level == Level::NFL;
        let late = self.ot || ((self.quarter == 2 || self.quarter >= 4) && self.half_left() <= 120.0);
        let scoring = ruled.touchdown || counter.touchdown || ruled.defense_td || counter.defense_td;
        let turnover = ruled.turnover || counter.turnover;
        let booth_rule = (rp.booth_late && late && rp.challenges > 0)
            || (scoring && rp.booth_scores)
            || (turnover && rp.booth_turnovers);
        // The booth looks at every play it covers, but stops the game only
        // when the replay raises a doubt: a wrong ruling it nearly always
        // sees, a right one sometimes needs a look to confirm (proposed,
        // not confirmed: one in eight). The college booth sees most close
        // plays, not all.
        let covered = booth_rule || (rp.booth_all && self.refs.chance(0.85));
        let booth = covered && (wrong || self.refs.chance(0.12));
        let assist = !covered && rp.assist && wrong && self.refs.chance(0.35);
        let can_challenge = !covered
            && !assist
            && rp.challenges > 0
            && self.challenges[against] > 0
            && self.sides[against].timeouts > 0
            && !(nfl && late);
        // The replay assistant upstairs (stand-in for his eye: the head
        // coach's game management): "throw it" or "don't".
        let gm = self.sides[against].team.staff.game_management;
        let acc = clamp(0.55 + 0.025 * gm, 0.55, 0.92);
        let says = if wrong { self.refs.chance(acc) } else { self.refs.chance((1.0 - acc) * 0.6) };
        let swing = (self.ruling_value(&counter) - self.ruling_value(&ruled)).abs();
        let worth = swing >= 0.8 || self.refs.chance(swing / 0.8);
        let staff_throw = can_challenge && says && worth;
        let mut throw = staff_throw;
        let ruled_text = self.ruling_text(call, &ruled);
        if can_challenge && self.user_is(against) {
            match self.ask(Ask::Challenge, false) {
                Some(Input::Challenge(b)) => throw = b,
                Some(_) => {}
                None if self.probe == Some(Ask::Challenge) => {
                    let mut p = self.pending(Ask::Challenge);
                    p.staff_challenge = staff_throw;
                    p.ruling = Some(ruled_text.clone());
                    p.advice.push(if says {
                        "Your replay assistant upstairs: \"Throw it. I think they missed it.\"".into()
                    } else {
                        "Your replay assistant upstairs: \"Don't. It looks right from here.\"".into()
                    });
                    let cost = "a lost challenge costs a timeout";
                    p.advice.push(format!(
                        "{} challenge{} and {} timeout{} left; {cost}.",
                        self.challenges[against],
                        if self.challenges[against] == 1 { "" } else { "s" },
                        self.sides[against].timeouts,
                        if self.sides[against].timeouts == 1 { "" } else { "s" },
                    ));
                    self.hold(p);
                    return ruled;
                }
                None => {}
            }
        }
        let reviewer = if booth {
            Reviewer::Booth
        } else if assist {
            Reviewer::Assist
        } else if throw {
            Reviewer::Challenge(against)
        } else {
            Reviewer::None
        };
        let overturned = match reviewer {
            Reviewer::Assist => true,
            Reviewer::None => false,
            _ => wrong && self.refs.chance(live::EVIDENCE),
        };
        if let Reviewer::Challenge(t) = reviewer {
            self.challenges[t] = self.challenges[t].saturating_sub(1);
            let name = self.sides[t].team.name.clone();
            if overturned {
                self.challenge_wins[t] += 1;
                if rp.third && self.challenge_wins[t] == 2 {
                    self.challenges[t] += 1;
                }
                self.log(t, format!("{name} challenge won: the ruling is reversed."));
            } else {
                self.sides[t].timeouts = self.sides[t].timeouts.saturating_sub(1);
                let left = self.sides[t].timeouts;
                self.log(
                    t,
                    format!("{name} challenge lost: the ruling stands, and the timeout is charged ({left} left)."),
                );
            }
        }
        if reviewer == Reviewer::None && !wrong {
            return ruled;
        }
        let truth_text = self.ruling_text(call, if wrong { &counter } else { &ruled });
        let note = match (reviewer, overturned, wrong) {
            (Reviewer::Assist, _, _) => format!("Ruled {ruled_text}; fixed by replay assist: {truth_text}."),
            (Reviewer::Booth, true, _) => {
                format!("Ruled {ruled_text}; the booth reversed it after review: {truth_text}.")
            }
            (Reviewer::Booth, false, true) => format!(
                "Ruled {ruled_text}; reviewed by the booth, the ruling stands. The replays suggest {truth_text}."
            ),
            (Reviewer::Booth, false, false) => format!("Ruled {ruled_text}; confirmed by the booth."),
            (Reviewer::Challenge(_), true, _) => format!("Ruled {ruled_text}; reversed on a challenge: {truth_text}."),
            (Reviewer::Challenge(_), false, true) => format!(
                "Ruled {ruled_text}; challenged, the ruling stands (not enough to overturn). The replays suggest {truth_text}."
            ),
            (Reviewer::Challenge(_), false, false) => {
                format!("Ruled {ruled_text}; challenged, and the ruling is confirmed.")
            }
            (Reviewer::None, _, _) => format!("Ruled {ruled_text}; not reviewed. The replays show {truth_text}."),
        };
        r.officiating = Some(Officiating { call, wrong, reviewer, overturned, against, note });
        if overturned { counter } else { ruled }
    }

    fn ruling_text(&self, call: CloseCall, x: &SnapResult) -> String {
        use scrimmage::Result as R;
        let who = |id: Option<u32>| id.map_or(String::new(), |i| format!(" by #{i}"));
        match call {
            CloseCall::Interception => {
                if x.result == R::Interception {
                    format!("an interception{}", who(x.returner))
                } else {
                    "incomplete, no interception".into()
                }
            }
            CloseCall::Fumble => {
                if x.turnover {
                    format!("a fumble{}, recovered by the defense", who(x.notes.fumbled))
                } else {
                    "down by contact before the ball came out".into()
                }
            }
            CloseCall::GoalLine => {
                if x.touchdown {
                    "a touchdown, the ball across the goal line".into()
                } else {
                    "down short of the goal line".into()
                }
            }
            CloseCall::Catch => {
                if x.result == R::Complete {
                    format!("a catch{}", who(x.carrier.or(x.target)))
                } else {
                    format!("incomplete, the pass to{} not caught", who(x.target).replacen(" by", "", 1))
                }
            }
            CloseCall::Spot => {
                if x.spot >= self.x + self.togo {
                    "a first down".into()
                } else {
                    "short of the line to gain".into()
                }
            }
        }
    }
}

/// Expected points from the real NFL table for a down, distance and the
/// yards to the goal on this game's field: three-down football reads its
/// first down as the NFL's second (two plays left to make ten), and another
/// field's yards as their share of a 100-yard one. The American field
/// passes straight through.
pub(crate) fn ep_at(college: bool, down: u8, togo: f64, yards_to_goal: f64) -> f64 {
    tables::expected_points_in(college, nfl_down(down), togo, super::geom::to_100(yards_to_goal))
}

/// Win probability from the real NFL table, on this game's field.
pub(crate) fn wp_at(diff: i32, left: f64, yards_to_goal: f64) -> f64 {
    tables::win_probability(diff, left, super::geom::to_100(yards_to_goal))
}

/// The NFL down a down reads as: the same in four-down football; in the
/// CFL's three downs the first is the NFL's second, the third its fourth.
pub(crate) fn nfl_down(down: u8) -> u8 {
    let f = super::geom::field();
    if f.downs >= 4 || down == 0 { down } else { (down + 4 - f.downs).min(4) }
}

/// Yards a kick travels past the line of scrimmage to the goalposts: the
/// seven-yard hold and the posts behind the goal line (17 on the American
/// field, 7 in the CFL to 2026).
pub(crate) fn kick_add() -> f64 {
    7.0 + super::geom::field().post_depth
}

/// Standard normal CDF.
fn phi(x: f64) -> f64 {
    crate::athlete::phi(x)
}

/// Play a game on a fixed neutral field, mostly for tools and tests.
pub fn quick_game(home: &GameTeam, away: &GameTeam, level: Level, seed: u64) -> GameResult {
    quick_game_with(home, away, level, seed, [garbage::Context::default(); 2])
}

/// `quick_game` with what the season does to each head coach's read of a
/// decided game.
pub fn quick_game_with(
    home: &GameTeam,
    away: &GameTeam,
    level: Level,
    seed: u64,
    context: [garbage::Context; 2],
) -> GameResult {
    if let Some((h, a)) = experiment::adjust(home, away) {
        let mut s = GameSetup::new(&h, &a, level, seed);
        s.cond = Conditions { crowd: 0.5, ..Conditions::default() };
        s.context = context;
        return play_game(&s);
    }
    let mut s = GameSetup::new(home, away, level, seed);
    s.cond = Conditions { crowd: 0.5, ..Conditions::default() };
    s.context = context;
    play_game(&s)
}

/// Where a game is played and what it asks of each team (section 4 and
/// trial 4 topic 3): the weather, the crowd and altitude in `cond`, a
/// neutral site, each team's road wear and how unused it is to the cold.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Venue {
    pub cond: Conditions,
    /// No home crowd.
    pub neutral: bool,
    pub wear: [f64; 2],
    pub cold_unused: [f64; 2],
    /// The season (0 for today's rules).
    #[serde(default)]
    pub year: i32,
    /// For the views only: the local kickoff hour and a roof over the field.
    #[serde(default)]
    pub kickoff: f32,
    #[serde(default)]
    pub indoor: bool,
    /// Keep the setup for replays (your games).
    #[serde(default)]
    pub keep: bool,
    /// A playoff game: overtime is played out (update 3, F25).
    #[serde(default)]
    pub playoff: bool,
}

/// A quick game at a venue.
pub fn quick_game_in(
    home: &GameTeam,
    away: &GameTeam,
    level: Level,
    seed: u64,
    context: [garbage::Context; 2],
    venue: Venue,
) -> GameResult {
    let adjusted = experiment::adjust(home, away);
    let (home, away) = match &adjusted {
        Some((h, a)) => (h, a),
        None => (home, away),
    };
    let mut s = GameSetup::new(home, away, level, seed);
    // The season's own rule book (update 3, F25).
    s.rules = Rules::for_year(level, venue.year, venue.playoff);
    s.playoff = venue.playoff;
    s.cond = venue.cond;
    s.neutral = venue.neutral;
    s.wear = venue.wear;
    s.cold_unused = venue.cold_unused;
    s.year = venue.year;
    s.context = context;
    s.keep = venue.keep;
    s.kickoff = venue.kickoff;
    s.indoor = venue.indoor;
    play_game(&s)
}

/// A quick game under another league's rule book (X7): pro players, the
/// book's field, rules and clock.
pub fn quick_game_book(
    home: &GameTeam,
    away: &GameTeam,
    book: super::books::Book,
    year: i32,
    seed: u64,
    venue: Venue,
    playoff: bool,
) -> GameResult {
    let mut s = GameSetup::new(home, away, Level::NFL, seed);
    s.rules = Rules::for_book(book, year);
    s.year = year;
    // A playoff game plays its overtime out: no ties.
    s.playoff = playoff;
    s.cond = venue.cond;
    s.neutral = venue.neutral;
    s.wear = venue.wear;
    s.cold_unused = venue.cold_unused;
    play_game(&s)
}

#[allow(dead_code)]
fn unused() -> crate::game::geom::V2 {
    v2(0.0, 0.0)
}

/// A bare game at one spot, for the trace tools.
fn bare_game<'a>(setup: &'a GameSetup, off: &'a GameTeam, def: &'a GameTeam, los: f64, seed: u64) -> Game<'a> {
    let mut g = Game {
        s: setup,
        sides: [Side::new(off, 99.0, 3), Side::new(def, 99.0, 3)],
        rng: Rng::new(seed as u32),
        crew: Crew { holding: 1.0, pass_interference: 1.0, pre_snap: 0.0, personal: 1.0 },
        stats: GameStats::default(),
        res: GameResult::default(),
        quarter: 1,
        clock: 900.0,
        running: false,
        poss: 0,
        x: los,
        down: 1,
        togo: 10.0,
        ball_y: mid(),
        next: Next::Scrimmage,
        over: false,
        receive_first: 0,
        n: 0,
        after_turnover: false,
        two_min_warned: false,
        pace: 1.0,
        no_huddle: false,
        kick_back: 0.0,
        ot: false,
        ot_round: 0,
        ot_poss: [0, 0],
        ot_first: 0,
        drive_start_poss: 0,
        mind: Rng::new(seed as u32 ^ 0x6A09_E667),
        reviewed: None,
        ctl: None,
        paused: false,
        probe: None,
        snap_input: None,
        fourth_note: None,
        mark: (false, false, false, String::new()),
        // A trace shows the play as it happened: no officials' mistakes.
        eye: live::Eye { miss: 0.0 },
        refs: Rng::new(live::ref_seed(seed)),
        replay: Replay::of(Level::HS, 0),
        challenges: [0; 2],
        challenge_wins: [0; 2],
        flipped: false,
        tempo: Tempo::Normal,
        toss_rng: Rng::new(live::toss_seed(seed)),
        trying: false,
        must_go: false,
        def_script: false,
        kick_look: false,
        fake: None,
    };
    g.stats.quarters.push([0, 0]);
    g
}

/// Debug tooling: run one snap with a given call from a spot, printing the
/// tracked play as it goes.
pub fn trace_snap(
    off: &GameTeam,
    def: &GameTeam,
    level: Level,
    call: OffCall,
    dcall: DefCall,
    los: f64,
    seed: u64,
) -> PlayRecord {
    trace_snap_opt(off, def, level, call, dcall, los, seed, true)
}

/// Run one snap with or without the agent trace.
#[allow(clippy::too_many_arguments)]
pub fn trace_snap_opt(
    off: &GameTeam,
    def: &GameTeam,
    level: Level,
    call: OffCall,
    dcall: DefCall,
    los: f64,
    seed: u64,
    trace: bool,
) -> PlayRecord {
    let setup = GameSetup::new(off, def, level, seed);
    let mut g = bare_game(&setup, off, def, los, seed);
    super::sim::TRACE.store(trace, std::sync::atomic::Ordering::Relaxed);
    let r = g.run_scrimmage_call(call, Some(dcall));
    super::sim::TRACE.store(false, std::sync::atomic::Ordering::Relaxed);
    r
}

/// One kickoff by `off` to `def` with the level's rules, for the trace
/// tools: the play record and where the receiving team takes over.
pub fn trace_kickoff(off: &GameTeam, def: &GameTeam, level: Level, seed: u64, trace: bool) -> (PlayRecord, f64) {
    let setup = GameSetup::new(off, def, level, seed);
    let mut g = bare_game(&setup, off, def, 35.0, seed);
    super::sim::TRACE.store(trace, std::sync::atomic::Ordering::Relaxed);
    let from = g.rules().kickoff_from;
    g.kickoff(0, from, false);
    super::sim::TRACE.store(false, std::sync::atomic::Ordering::Relaxed);
    (g.stats.plays.pop().expect("a kickoff record"), g.x)
}

/// A practice snap's situation (training topic 8): the down, distance and
/// spot the period sets, the time left in the half for two-minute work,
/// and whether the period is all runs or all passes.
#[derive(Clone, Copy, Debug)]
pub struct PracticeSnap {
    pub down: u8,
    pub togo: f64,
    pub yardline_100: f64,
    pub half_left: f64,
    pub run: Option<bool>,
}

/// Practice snaps on the engine (training topic 8, choice 3): `off`'s
/// offense against `def`'s defense, each snap called by the two staffs for
/// its situation as on game day (the week's call sheet included). Every
/// snap starts fresh: practice has its own fatigue and injuries (the
/// training model), so the engine's are cleared between snaps. The plays
/// come back booked, with their grades, in a game's stats.
pub fn practice_snaps(off: &GameTeam, def: &GameTeam, level: Level, snaps: &[PracticeSnap], seed: u64) -> GameStats {
    let setup = GameSetup::new(off, def, level, seed);
    let mut g = bare_game(&setup, off, def, 35.0, seed);
    let fresh: [Vec<f64>; 2] = [g.sides[0].energy.clone(), g.sides[1].energy.clone()];
    let mut stats = GameStats::default();
    for (k, s) in snaps.iter().enumerate() {
        for (side, e) in g.sides.iter_mut().zip(fresh.iter()) {
            side.energy.clone_from(e);
            side.out.iter_mut().for_each(|o| *o = false);
            side.hurt.iter_mut().for_each(|h| *h = None);
        }
        g.rng = Rng::new((seed as u32) ^ (k as u32).wrapping_mul(0x9E37_79B9));
        g.poss = 0;
        g.x = clamp(100.0 - s.yardline_100, 1.0, 99.0);
        g.down = s.down.clamp(1, 4);
        g.togo = s.togo.clamp(1.0, 30.0).min(s.yardline_100);
        g.quarter = if s.half_left < 900.0 { 2 } else { 1 };
        g.clock = s.half_left.min(900.0);
        g.ball_y = super::geom::mid();
        g.stats.plays.clear();
        let r = g.snap_play_as(false, s.run);
        stats.book(&r);
        stats.plays.push(r);
    }
    stats
}

/// The depth-chart spot an offensive role asks for.
fn orole_spot(r: ORole) -> Spot {
    match r {
        ORole::QB => Spot::QB,
        ORole::RB => Spot::RB,
        ORole::FB => Spot::FB,
        ORole::X | ORole::Z | ORole::Slot | ORole::Slot2 | ORole::Slot3 => Spot::WR,
        ORole::TE | ORole::TE2 | ORole::TE3 => Spot::TE,
        ORole::LT => Spot::LT,
        ORole::RT => Spot::RT,
        ORole::LG | ORole::RG => Spot::G,
        ORole::C => Spot::C,
    }
}

/// The depth-chart spot a defensive slot asks for.
fn dslot_spot(s: DSlot) -> Spot {
    match s {
        DSlot::LE | DSlot::RE => Spot::EDGE,
        DSlot::DTL | DSlot::DTR => Spot::DT,
        DSlot::NT => Spot::NT,
        DSlot::Mike => Spot::MLB,
        DSlot::Sam | DSlot::Will => Spot::OLB,
        DSlot::CBL | DSlot::CBR => Spot::CB,
        DSlot::NCB | DSlot::DCB => Spot::NB,
        DSlot::FS => Spot::FS,
        DSlot::SS => Spot::SS,
    }
}

/// Whether a listed position may play a spot without it being an
/// emergency: his own group, a tight end at fullback (3.4), a corner at
/// receiver or a receiver at corner (3.5).
fn same_group(pos: Position, sp: Spot) -> bool {
    let to = sp.position();
    spots::group(pos) == spots::group(to)
        || pos == Position::TE && sp == Spot::FB
        || pos == Position::CB && to == Position::WR
        || pos == Position::WR && to == Position::CB
}

/// A kicker's chance to make a field goal from a distance, as his staff
/// reckons it (also the Plans page's 4th-down chart, update 2 G4).
pub fn fg_make(p: &GamePlayer, dist: f64) -> f64 {
    let acc = super::kicking::fg_accuracy(p, dist);
    let range = 38.0 + 0.3 * p.a(Attr::KickPower);
    let sd = (2.95 + 0.012 * (dist - 30.0)) * (1.82f64 - 0.0105 * acc).max(0.55);
    let half = (super::geom::field().posts / dist).atan().to_degrees();
    let z = half / sd;
    let p_line = 1.0 - 2.0 * (1.0 - phi(z));
    let p_range = phi((range - dist) / 3.0);
    p_line * p_range
}

/// How much a quarterback threatens to run, 0-1 (topic 5.4): a spy and the
/// option game key on it (proposed, not confirmed).
/// A quarterback's running threat (0 to 1) from his speed and elusiveness.
/// Read for his weight, as the forty is (A3).
pub fn qb_run(p: &GamePlayer) -> f64 {
    let moves = 0.6 * p.a(Attr::Speed) + 0.4 * p.a(Attr::Elusiveness) - super::sim::size_cost(p.weight, 222.0);
    clamp((moves - 68.0) / 20.0, 0.0, 1.0)
}
