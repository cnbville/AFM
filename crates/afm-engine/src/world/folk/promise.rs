//! Promises he judges for himself (the promises design,
//! `afm/promises/proposal.md`, sections 3 to 9 and 11; agreed by Cain
//! 2026-11-13, "agree to all"; built in lane P1). Every size here is
//! Claude's working value (section 14: proposed, not confirmed).
//!
//! - **Terms** (section 3): what he gets, by when, on what condition (a
//!   stat line, practice, conduct, grades, his body, his health) and
//!   against whom. A stat bar is judged as you said it, and read per snap
//!   against your level's starters at his spot: a bar he can't reach on
//!   the snaps he'll get is heard as a setup.
//! - **Belief** (section 4): its own read for each promise, never a meter
//!   on screen, from six parts counted once: his trust in you, your record
//!   as it reaches him, how likely it looks from what he sees, what he's
//!   been through, who is in his ear, and the promise's shape. It moves a
//!   quarter of the way each week toward what the week showed him, and his
//!   trust moves a fifth as much.
//! - **The talk** (section 5): he answers by belief and, when he isn't sure
//!   or doubts, asks one follow-up; your answer reshapes the promise.
//! - **In season** (section 6): on track, behind, owed, missed, kept or
//!   broken; behind two straight weeks or owed a week, he comes to you.
//! - **Kept and broken** (section 7): judged over the games from its date
//!   to its term's end, 3 in every 4 healthy games; keeping pays more the
//!   more he doubted, breaking costs by what he counted on and the share
//!   missed.
//! - **"Stay out of the portal"** (section 8): in the meeting before he
//!   enters, a promise moves his lean by what it adds for him times his
//!   belief, at most halving it.
//! - **Recruits and transfers** (section 9): the same parts, with the
//!   trail's name and his family in place of trust.

use super::claims::{Ear, Group};
use super::seen;
use super::talks::{Promise, PromiseKind, PromiseTo};
use super::{Name, Team};
use crate::calendar::{Date, Level};
use crate::game::stats::StatLine;
use crate::player::{PlayingTime, Trait};
use crate::positions::Position;
use crate::world::{Person, Stage, World};

// ---------------------------------------------------------------------------
// Terms (section 3).

/// By when he gets it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum By {
    /// From his next game (the opener out of season).
    #[default]
    Now,
    Week4,
    Midseason,
    NextSeason,
}

impl By {
    pub fn words(self) -> &'static str {
        match self {
            By::Now => "now",
            By::Week4 => "by week 4",
            By::Midseason => "by midseason",
            By::NextSeason => "next season",
        }
    }
}

/// The condition he has to meet, only things you both can see.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Cond {
    #[default]
    None,
    /// A stat line in games (`Terms::stat`, `Terms::bar` a game).
    Stat,
    /// Winning it in practice: your staff judges it.
    Practice,
    /// No team rule broken, no arrest.
    Conduct,
    /// He stays eligible.
    Grades,
    /// A weight by the start of camp (`Terms::bar` pounds).
    Weight,
    /// Once he is cleared.
    Health,
}

/// The stat line a condition reads, the one that fits his spot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Stat {
    #[default]
    Tackles,
    Sacks,
    Catches,
    YardsCarry,
    CmpShare,
    Picks,
    Pbus,
    SacksAllowed,
    Penalties,
}

impl Stat {
    /// The line that fits his spot.
    pub fn of(pos: Position) -> Option<Stat> {
        Some(match pos {
            Position::QB => Stat::CmpShare,
            Position::RB => Stat::YardsCarry,
            Position::WR | Position::TE => Stat::Catches,
            Position::OT | Position::IOL => Stat::SacksAllowed,
            Position::EDGE => Stat::Sacks,
            Position::DT | Position::LB | Position::S => Stat::Tackles,
            Position::CB => Stat::Pbus,
            _ => return None,
        })
    }
    /// A rate over his touches (yards a carry, completion share) rather
    /// than a count a game.
    pub fn rate(self) -> bool {
        matches!(self, Stat::YardsCarry | Stat::CmpShare)
    }
    /// Lower is better (a lineman's sacks allowed and penalties).
    pub fn lower(self) -> bool {
        matches!(self, Stat::SacksAllowed | Stat::Penalties)
    }
    /// (what counts, what it is over) in one game line: a count over one
    /// game, a rate over its attempts.
    pub fn of_line(self, l: &StatLine) -> (f64, f64) {
        match self {
            Stat::Tackles => ((l.tackles + l.assists) as f64, 1.0),
            Stat::Sacks => (l.sacks, 1.0),
            Stat::Catches => (l.rec as f64, 1.0),
            Stat::YardsCarry => (l.rush_yds, l.rush as f64),
            Stat::CmpShare => (l.cmp as f64, l.att as f64),
            Stat::Picks => (l.ints as f64, 1.0),
            Stat::Pbus => (l.pbus as f64, 1.0),
            Stat::SacksAllowed => (l.sacks_allowed, 1.0),
            Stat::Penalties => (l.penalties as f64, 1.0),
        }
    }
    /// The bar in your words ("3 tackles a game"): your own words, so the
    /// number is printed.
    pub fn words(self, bar: f32) -> String {
        let n = |x: f32| if (x - x.round()).abs() < 0.05 { format!("{}", x.round() as i32) } else { format!("{x:.1}") };
        match self {
            Stat::Tackles => format!("{} tackles a game", n(bar)),
            Stat::Sacks => format!("{} sacks a game", n(bar)),
            Stat::Catches => format!("{} catches a game", n(bar)),
            Stat::YardsCarry => format!("{} yards a carry", n(bar)),
            Stat::CmpShare => format!("{}% of your throws", (bar * 100.0).round() as i32),
            Stat::Picks => format!("{} picks a game", n(bar)),
            Stat::Pbus => format!("{} passes broken up a game", n(bar)),
            Stat::SacksAllowed => format!("no more than {} sacks allowed a game", n(bar)),
            Stat::Penalties => format!("no more than {} flags a game", n(bar)),
        }
    }
    /// A starter's line at the level when the world's games don't say
    /// (real 2021-25 starters, proposed, not confirmed): a count per snap,
    /// or the rate.
    fn fallback(self, level: Level) -> f64 {
        let hs = level == Level::HighSchool;
        match self {
            Stat::Tackles => 0.09,
            Stat::Sacks => 0.009,
            Stat::Catches => 0.07,
            Stat::YardsCarry => {
                if hs {
                    5.6
                } else if level == Level::Nfl {
                    4.3
                } else {
                    5.0
                }
            }
            Stat::CmpShare => {
                if hs {
                    0.56
                } else if level == Level::Nfl {
                    0.645
                } else {
                    0.62
                }
            }
            Stat::Picks => 0.003,
            Stat::Pbus => 0.012,
            Stat::SacksAllowed => 0.003,
            Stat::Penalties => 0.006,
        }
    }
}

/// A promise's terms (section 3). The default is a bare promise: from his
/// next game, no condition, nobody named.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Terms {
    pub by: By,
    /// A date was said (a bare "now" isn't one).
    pub dated: bool,
    pub cond: Cond,
    pub stat: Stat,
    /// The stat bar a game (a rate for a rate), or the weight in pounds.
    pub bar: f32,
    /// The teammate he plays ahead of.
    pub ahead: Option<u32>,
    /// How he heard the condition: 1 fair, -1 a setup, 0 none.
    pub fair: i8,
    /// In writing: an NFL playing time incentive clause.
    pub written: bool,
    /// What he had before you said it: his snap share, or touches a game.
    pub base: f32,
}

/// The day a promise's window opens: from its date, never before you said
/// it.
pub fn opens(w: &World, made: Date, by: By) -> Date {
    let weeks = match Team::seat(w).map(|t| t.level()) {
        Some(Level::Nfl) => &w.dates.nfl_weeks,
        _ => &w.dates.cfb_weeks,
    };
    let first = weeks.first().copied().unwrap_or(w.dates.fall_camp.add(30));
    let roll = |d: Date| if d.add(3) < made { d.add(364) } else { d };
    let d = match by {
        By::Now => made,
        By::Week4 => roll(weeks.get(3).copied().unwrap_or(first.add(21))),
        By::Midseason => roll(weeks.get(weeks.len() / 2).copied().unwrap_or(first.add(42))),
        By::NextSeason => {
            let f = first;
            if f > made { f.add(364) } else { f.add(364).max(made.add(150)) }
        }
    };
    d.max(made)
}

// ---------------------------------------------------------------------------
// States (section 6).

/// What he sees of an open promise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    /// Its date hasn't come, or no game has been played since.
    #[default]
    Waiting = 0,
    OnTrack = 1,
    Behind = 2,
    /// He met his condition and doesn't have it yet.
    Owed = 3,
    /// He missed his condition.
    Missed = 4,
    /// His own injury: it waits until he's cleared.
    Paused = 5,
}

impl State {
    pub fn of(k: u8) -> State {
        match k {
            1 => State::OnTrack,
            2 => State::Behind,
            3 => State::Owed,
            4 => State::Missed,
            5 => State::Paused,
            _ => State::Waiting,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            State::Waiting => "waiting",
            State::OnTrack => "on_track",
            State::Behind => "behind",
            State::Owed => "owed",
            State::Missed => "missed",
            State::Paused => "paused",
        }
    }
    /// Where a week of it moves his belief (section 4): on track 0.9, owed
    /// 0.4, behind 0.25; the rest hold.
    fn pull(self) -> Option<f64> {
        match self {
            State::OnTrack => Some(0.9),
            State::Owed => Some(0.4),
            State::Behind => Some(0.25),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Sizes (section 14, proposed, not confirmed).

/// Belief bands for your own player: believes from 0.65, isn't sure from
/// 0.4, doubts under.
pub const BELIEVES: f64 = 0.65;
pub const UNSURE: f64 = 0.4;
/// A transfer or a recruit believes from 0.6 (as built).
pub const BELIEVES_OUT: f64 = 0.6;
/// The weekly step toward what the week showed him, and his trust's share
/// of his belief's move.
pub const WEEK_STEP: f64 = 0.25;
pub const TRUST_SHARE: f64 = 0.2;
/// Kept in 3 of every 4 healthy games.
pub const KEPT_SHARE: f64 = 0.75;
/// A new date takes a third off his belief.
pub const NEW_DATE: f64 = 2.0 / 3.0;
/// Moving the goalposts once he met them: trust.
pub const GOALPOSTS: f32 = 0.08;
/// The named teammate who hears of it: trust.
pub const OVER_TRUST: f32 = 0.04;
/// What the named teammate's portal lean does when he hears (proposed).
pub const OVER_LEAN: f64 = 1.15;
/// A snap share that is a real role.
pub const ROLE_SHARE: f64 = 0.25;
/// His own deadline passed and he doesn't have it: his lean (proposed).
pub const DEADLINE_LEAN: f64 = 1.5;
/// The portal meeting's pull at most (section 8).
pub const PULL_CAP: f64 = 0.5;
/// Weeks behind, or owed, before he comes to you.
pub const BEHIND_WEEKS: u8 = 2;
pub const OWED_WEEKS: u8 = 1;

// ---------------------------------------------------------------------------
// Belief (section 4).

/// The six parts, each counted once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parts {
    /// His trust in you (a recruit's: the trail, his family and his high
    /// school coach), 0-1.
    pub trust: f64,
    /// Your record as it reaches him, 0-1 (0.5 neutral).
    pub record: f64,
    /// How likely it looks from what he sees, 0.3-1.
    pub likely: f64,
    /// What he's been through, 0.75-1.
    pub past: f64,
    /// Who is in his ear and how long he's been around, 0.8-1.05.
    pub ear: f64,
    /// The promise's shape, 0.5-1.2.
    pub shape: f64,
}

impl Parts {
    pub fn belief(&self) -> f64 {
        ((0.25 + 0.45 * self.trust + 0.30 * self.record) * self.likely * self.past * self.ear * self.shape)
            .clamp(0.05, 0.97)
    }
}

/// Who hears it: decides the trust part and the name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hearer {
    /// Your own player.
    Own,
    /// A recruit's camp (him or whoever decides), or a transfer, with your
    /// program; the case's straight answers count.
    Out(usize, Ear),
}

fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

/// His trust part: your own player's trust in you; out of your building,
/// the trail's name, the answers you gave him straight and his family.
pub fn trust_part(w: &World, i: usize, h: Hearer) -> f64 {
    let p = &w.people[i];
    match h {
        Hearer::Own => {
            if you(w).is_some_and(|c| c as u32 == p.trust_in) {
                p.trust as f64
            } else {
                0.5
            }
        }
        Hearer::Out(j, ear) => {
            let straight = super::case::of(p, j)
                .map(|c| c.belief())
                .or_else(|| crate::world::front_office::portal::case_of(w, p.player.id).map(|c| c.belief()))
                .unwrap_or(0.0);
            // A family that runs it reads you warier than the boy does.
            let family = if ear == Ear::Decider { -0.03 } else { 0.0 };
            let rel = you(w).map_or(10.0, |c| w.coaches[c].ability().relationships);
            (0.5 + 0.25 * w.folk.mine.name(Name::Trail) as f64 + 0.01 * (rel - 10.0) + 2.0 * straight + family)
                .clamp(0.1, 0.9)
        }
    }
}

/// The name his audience holds: the room for your players (agents too in
/// the NFL), the trail out of your building.
pub fn heard_name(w: &World, i: usize, h: Hearer) -> f64 {
    let m = &w.folk.mine;
    match h {
        Hearer::Own if w.people[i].stage == Stage::Pro => 0.5 * (m.name(Name::Room) + m.name(Name::Agents)) as f64,
        Hearer::Own => m.name(Name::Room) as f64,
        Hearer::Out(..) => m.name(Name::Trail) as f64,
    }
}

/// Your record as it reaches him: your name with his audience, then what
/// his close friends got from you in the last two years and what everyone
/// knows (promises said in public).
pub fn record(w: &World, i: usize, h: Hearer) -> f64 {
    let id = w.people[i].player.id;
    let friends = super::ties::friends(w, id);
    let mut r = 0.5 + 0.25 * heard_name(w, i, h);
    for p in w.folk.promises.iter().filter(|p| p.due.days_to(w.today) <= 730) {
        let PromiseTo::Player(to) = p.to else { continue };
        if to == id {
            continue;
        }
        if !(friends.contains(&to) || p.public) {
            continue;
        }
        match p.kept {
            Some(true) if p.missed == 0.0 && !p.released => r += 0.03,
            Some(false) => r -= 0.06,
            _ => {}
        }
    }
    r.clamp(0.0, 1.0)
}

/// What he's been through: a coach who left him, or a transfer over snaps
/// or a coach.
pub fn past(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    let mut k: f64 = 1.0;
    // A head coach who left him: two head coaches at one program.
    let s = &p.college_staff;
    if s.windows(2).any(|x| x[0].1 == x[1].1 && x[0].2.is_some() && x[1].2.is_some() && x[0].2 != x[1].2) {
        k *= 0.9;
    }
    let why = p.cal.portal.as_ref().map(|e| e.why.to_string()).unwrap_or_default();
    match why.as_str() {
        "playing time" | "spring depth chart" | "coaching change" => k *= 0.85,
        _ if p.transfers > 0 => k *= 0.92,
        _ => {}
    }
    k.clamp(0.75, 1.0)
}

/// Who is in his ear and how long he's been around: a trainer, an agent or
/// a loud parent pulls him toward their doubt; a fifth-year has heard it
/// all before; a young man with little ego takes a coach at his word.
pub fn ear(w: &World, i: usize) -> f64 {
    use crate::world::background::{Decider, ParentStyle};
    let p = &w.people[i];
    let t = &p.player.personality.traits;
    let mut k: f64 = 1.0;
    if matches!(p.family.parent_style, ParentStyle::LoudAtGames | ParentStyle::SocialMedia) {
        k *= 0.94;
    }
    if p.family.decider == Decider::Handler {
        k *= 0.92;
    }
    if p.pro.as_ref().is_some_and(|c| c.agent.is_some()) {
        k *= 0.95;
    }
    let old = p.college_seasons >= 4 || p.pro.as_ref().is_some_and(|_| p.row_age() >= 29);
    if old {
        k *= 0.94;
    } else if p.college_seasons <= 1 && p.stage != Stage::Pro && t.z(Trait::Ego) < 0.0 {
        k *= 1.05;
    }
    k.clamp(0.8, 1.05)
}

/// A start or playing ahead of a man: the promises that need someone
/// passed.
fn needs_passing(kind: PromiseKind) -> bool {
    matches!(kind, PromiseKind::Start | PromiseKind::Join(0) | PromiseKind::Place(0))
        || matches!(kind, PromiseKind::Recruit(0, _))
}

/// How good another man looks to him: his play from practice every day,
/// his starts, his class.
fn looks(w: &World, j: usize) -> f64 {
    let q = &w.people[j];
    let starts = seen::games(w, q.player.id, w.today.add(-seen::SEASON_DAYS), w.today, None)
        .iter()
        .filter(|g| g.started())
        .count()
        .min(4) as f64;
    q.player.overall() + 0.75 * starts + if q.college_seasons >= 3 || q.stage == Stage::Pro { 1.0 } else { 0.0 }
}

/// How likely it looks to your own player from what he sees: the man he
/// has to pass against his own view of himself (ego and ambition inflate
/// it), and for a stat bar how far it sits above his line now.
pub fn likely_own(w: &World, i: usize, kind: PromiseKind, t: &Terms) -> f64 {
    let p = &w.people[i];
    let me = crate::world::people::self_view(p);
    let ahead: Vec<u32> = match t.ahead {
        Some(a) => vec![a],
        None => seen::ahead(w, i),
    };
    let best = ahead.iter().filter_map(|&a| super::person(w, a)).map(|j| looks(w, j)).fold(f64::MIN, f64::max);
    let gap = if best == f64::MIN { 6.0 } else { me - best };
    let mut k = match kind {
        k if needs_passing(k) => 0.65 + 0.06 * gap,
        PromiseKind::Snaps | PromiseKind::Touches | PromiseKind::Join(_) | PromiseKind::Recruit(1, _) => {
            0.8 + 0.05 * gap
        }
        _ => 0.9,
    };
    if t.cond == Cond::Stat {
        k *= bar_likely(w, i, kind, t);
    }
    k.clamp(0.3, 1.0)
}

/// How reachable a stat bar looks: his line now against the bar.
fn bar_likely(w: &World, i: usize, kind: PromiseKind, t: &Terms) -> f64 {
    let can = reach(w, i, kind, t);
    if can <= 0.0 {
        return 1.0;
    }
    let ratio = if t.stat.lower() { can / (t.bar as f64).max(0.05) } else { t.bar as f64 / can };
    (1.3 - 0.6 * ratio).clamp(0.3, 1.0)
}

/// The promise's shape: a named teammate, a date and a fair condition make
/// it real; a big vague promise from a man he barely knows sounds like a
/// pitch; a long one weighs your job security; the same spot promised to
/// a man he knows halves it; and what you backed it with (a real strength
/// of his, or one that isn't, playtest bug 15's 2c).
pub fn shape(w: &World, i: usize, kind: PromiseKind, t: &Terms, backed: i8) -> f64 {
    let p = &w.people[i];
    let mut k: f64 = 1.0;
    if t.ahead.is_some() {
        k += 0.08;
    }
    if t.dated {
        k += 0.05;
    }
    k += match t.fair {
        1 => 0.07,
        -1 => -0.15,
        _ => 0.0,
    };
    k += match backed {
        1 => 0.15,
        -1 => -0.2,
        _ => 0.0,
    };
    if t.written {
        k += 0.15;
    }
    let known = w.folk.mine.since.map_or(0, |d| d.days_to(w.today));
    let vague = t.ahead.is_none() && !t.dated && t.cond == Cond::None;
    if vague && needs_passing(kind) && known < 90 && p.stage != Stage::HighSchool {
        k *= 0.85;
    }
    // A long promise weighs your job: a coach on the hot seat is believed
    // less.
    let long = t.by == By::NextSeason || kind == PromiseKind::Stay;
    if long && you(w).is_some_and(|c| super::claims::hot_seat(w, c)) {
        k *= 0.8;
    }
    if promised_twice_known(w, i, kind) {
        k *= 0.5;
    }
    k.clamp(0.5, 1.2)
}

/// The same spot promised to a man he knows: another open start promise
/// at his position, to a friend of his.
fn promised_twice_known(w: &World, i: usize, kind: PromiseKind) -> bool {
    if !needs_passing(kind) {
        return false;
    }
    let p = &w.people[i];
    let (id, pos) = (p.player.id, p.player.position);
    let friends = super::ties::friends(w, id);
    w.folk.promises.iter().any(|x| {
        let PromiseTo::Player(o) = x.to else { return false };
        o != id
            && x.kept.is_none()
            && needs_passing(x.kind)
            && friends.contains(&o)
            && super::person(w, o).is_some_and(|j| w.people[j].player.position == pos)
    })
}

/// The six parts for your own player.
pub fn parts_own(w: &World, i: usize, kind: PromiseKind, t: &Terms, backed: i8) -> Parts {
    Parts {
        trust: trust_part(w, i, Hearer::Own),
        record: record(w, i, Hearer::Own),
        likely: likely_own(w, i, kind, t),
        past: past(w, i),
        ear: ear(w, i),
        shape: shape(w, i, kind, t, backed),
    }
}

/// His belief in a promise of yours as he hears it now.
pub fn belief_own(w: &World, i: usize, kind: PromiseKind, t: &Terms, backed: i8) -> f64 {
    parts_own(w, i, kind, t, backed).belief()
}

/// A recruit's or a transfer's belief: the same parts, with the trail in
/// place of trust and your room, and `likely` from the public roster (a
/// recruit) or his play against the man he'd beat (a transfer).
pub fn belief_out(w: &World, i: usize, j: usize, ear_of: Ear, likely: f64) -> f64 {
    let h = Hearer::Out(j, ear_of);
    Parts {
        trust: trust_part(w, i, h),
        record: record(w, i, h),
        likely: likely.clamp(0.3, 1.0),
        past: past(w, i),
        ear: ear(w, i),
        shape: 1.0,
    }
    .belief()
}

/// He'd have it anyway, from what he sees: a start for a starter, a role
/// for a man who already plays one.
pub fn knows(w: &World, i: usize, kind: PromiseKind) -> bool {
    let first = seen::first_now(w, i).unwrap_or(w.people[i].last_role == Some(PlayingTime::Starter));
    match kind {
        PromiseKind::Start | PromiseKind::Join(0) | PromiseKind::Place(0) => first,
        PromiseKind::Snaps | PromiseKind::Join(_) => first || share_now(w, i) >= ROLE_SHARE,
        _ => false,
    }
}

/// His snap share over his last two healthy games this season.
pub fn share_now(w: &World, i: usize) -> f64 {
    let id = w.people[i].player.id;
    let gs: Vec<seen::Game> =
        seen::games(w, id, w.today.add(-seen::SEASON_DAYS), w.today, None).into_iter().filter(|g| !g.hurt).collect();
    let last: Vec<&seen::Game> = gs.iter().rev().take(seen::WINDOW).collect();
    if last.is_empty() {
        return match w.people[i].last_role {
            Some(PlayingTime::Starter) => 0.8,
            Some(PlayingTime::Rotation) => 0.35,
            _ => 0.05,
        };
    }
    last.iter().map(|g| g.share()).sum::<f64>() / last.len() as f64
}

/// Touches a game over his last four games.
fn touches_now(w: &World, i: usize) -> f64 {
    let id = w.people[i].player.id;
    let gs: Vec<seen::Game> = seen::games(w, id, w.today.add(-seen::SEASON_DAYS), w.today, None);
    let last: Vec<&seen::Game> = gs.iter().rev().filter(|g| !g.hurt).take(4).collect();
    if last.is_empty() {
        return 0.0;
    }
    last.iter().map(|g| g.touches() as f64).sum::<f64>() / last.len() as f64
}

/// His band as your player hears it: believes, isn't sure, doubts.
pub fn band(b: f64) -> &'static str {
    if b >= BELIEVES {
        "good"
    } else if b >= UNSURE {
        "flat"
    } else {
        "bad"
    }
}

// ---------------------------------------------------------------------------
// A stat bar against what he can reach.

/// A starter's line per snap at his spot and your level (a rate for a rate
/// stat), from your level's games kept whole; the real starter's line when
/// they are too few.
pub fn starter_rate(w: &World, pos: Position, stat: Stat) -> f64 {
    let level = Team::seat(w).map_or(Level::College, |t| t.level());
    let off = seen::offense(pos);
    let mut v: Vec<f64> = Vec::new();
    for g in w.seat_games.iter().filter(|g| g.level == level).rev().take(40) {
        for side in 0..2 {
            let most = g.stats.players[side].values().map(|l| seen::snaps(l, off)).max().unwrap_or(0);
            for (&id, l) in &g.stats.players[side] {
                let s = seen::snaps(l, off);
                if most == 0 || (s as f64) < seen::START_SHARE * most as f64 {
                    continue;
                }
                let at = super::person(w, id).is_some_and(|j| w.people[j].player.position == pos);
                if !at {
                    continue;
                }
                let (n, d) = stat.of_line(l);
                if stat.rate() {
                    if d >= 5.0 {
                        v.push(n / d);
                    }
                } else if s > 0 {
                    v.push(n / s as f64);
                }
            }
        }
    }
    if v.len() < 8 {
        return stat.fallback(level);
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

/// The most snaps a game on his side in your recent games (a starter's
/// load), else a typical one.
fn full_load(w: &World, i: usize) -> f64 {
    let id = w.people[i].player.id;
    let gs = seen::games(w, id, w.today.add(-seen::SEASON_DAYS), w.today, None);
    let v: Vec<u32> = gs.iter().rev().take(4).map(|g| g.most).filter(|&m| m > 0).collect();
    if v.is_empty() { 62.0 } else { v.iter().sum::<u32>() as f64 / v.len() as f64 }
}

/// The snaps a game he'll play while the bar runs: a promise he gets now
/// runs on its own snaps; one he has to earn first runs on the snaps he
/// gets now.
pub fn snaps_under(w: &World, i: usize, kind: PromiseKind, t: &Terms) -> f64 {
    let load = full_load(w, i);
    let now = share_now(w, i) * load;
    if t.by != By::Now {
        return now;
    }
    match kind {
        k if needs_passing(k) => load * 0.95,
        PromiseKind::Snaps | PromiseKind::Join(_) | PromiseKind::Recruit(1, _) => (load * 0.4).max(now),
        _ => now,
    }
}

/// What a starter's line would give him on those snaps (a rate for a rate
/// stat): what he can reach at best.
pub fn reach(w: &World, i: usize, kind: PromiseKind, t: &Terms) -> f64 {
    let pos = w.people[i].player.position;
    let r = starter_rate(w, pos, t.stat);
    if t.stat.rate() { r } else { r * snaps_under(w, i, kind, t) }
}

/// A fair bar (section 3): within what a starter's line gives on the snaps
/// he'll play; above it, or for a lower-is-better stat below it, a setup.
pub fn fair(w: &World, i: usize, kind: PromiseKind, t: &Terms) -> bool {
    let can = reach(w, i, kind, t);
    if t.stat.lower() { t.bar as f64 >= 0.85 * can } else { t.bar as f64 <= 1.1 * can }
}

/// Two bars to offer: one a starter makes on the snaps he'll play, and one
/// well above it.
pub fn bars(w: &World, i: usize, kind: PromiseKind, stat: Stat, by: By) -> (f32, f32) {
    let t = Terms { stat, by, ..Terms::default() };
    let can = reach(w, i, kind, &t);
    let round = |x: f64| {
        if stat.rate() && stat == Stat::CmpShare {
            (x * 100.0).round() / 100.0
        } else if x >= 2.0 {
            x.round()
        } else {
            (x * 2.0).round() / 2.0
        }
    };
    if stat.lower() {
        (round(can * 1.4).max(0.5) as f32, round(can * 0.4) as f32)
    } else {
        (round(can * 0.85).max(0.5) as f32, round(can * 1.7).max(1.0) as f32)
    }
}

// ---------------------------------------------------------------------------
// Making it.

/// A new promise to your player, with its belief as he hears it. Sets the
/// promise's belief, what he had before, and how he heard its condition;
/// an unfair bar costs trust as a move against the facts does (-0.03 to
/// -0.06, more with ego). Returns his belief.
pub fn hear(w: &mut World, k: usize) -> f64 {
    let p = w.folk.promises[k].clone();
    let PromiseTo::Player(id) = p.to else { return 0.0 };
    let Some(i) = super::person(w, id) else { return 0.0 };
    let mut t = p.terms;
    t.base = match p.kind {
        PromiseKind::Touches => touches_now(w, i) as f32,
        _ => share_now(w, i) as f32,
    };
    if t.cond == Cond::Stat {
        let ok = fair(w, i, p.kind, &t);
        t.fair = if ok { 1 } else { -1 };
        if !ok {
            let ego = w.people[i].player.personality.traits.z(Trait::Ego).clamp(0.0, 1.0) as f32;
            let q = &mut w.people[i];
            q.trust = (q.trust - 0.03 - 0.03 * ego).max(0.0);
        }
    } else if t.cond != Cond::None {
        t.fair = 1;
    }
    let b = belief_own(w, i, p.kind, &t, p.backed);
    let x = &mut w.folk.promises[k];
    x.terms = t;
    x.belief = b as f32;
    x.said = b as f32;
    b
}

/// The open promise to a player made today, newest first.
pub fn fresh(w: &World, id: u32) -> Option<usize> {
    (0..w.folk.promises.len())
        .rev()
        .find(|&k| w.folk.promises[k].to == PromiseTo::Player(id) && w.folk.promises[k].kept.is_none())
}

/// The named teammate can find out (section 7): the promised player tells
/// his close friends (more with a big ego, less with professionalism), a
/// loud parent talks, or it was said in public. He loses trust, leans out
/// a little and it is on his mind. Two men promised the same spot who
/// find out both lose half their belief, and your room name drops as for a
/// broken promise. Returns what happened, in words.
pub fn spread(w: &mut World, k: usize) -> Vec<String> {
    let mut out = Vec::new();
    let p = w.folk.promises[k].clone();
    let PromiseTo::Player(id) = p.to else { return out };
    let Some(i) = super::person(w, id) else { return out };
    let t = &w.people[i].player.personality.traits;
    let loud = matches!(
        w.people[i].family.parent_style,
        crate::world::background::ParentStyle::LoudAtGames | crate::world::background::ParentStyle::SocialMedia
    );
    let tells = (0.25 + 0.15 * t.z(Trait::Ego) - 0.1 * t.z(Trait::Professionalism)
        + if loud { 0.15 } else { 0.0 }
        + if p.public { 1.0 } else { 0.0 })
    .clamp(0.05, 0.95);
    let name = w.people[i].name.clone();
    if let Some(o) = p.terms.ahead
        && let Some(j) = super::person(w, o)
        && super::roll(w, 0x0BE4 ^ (k as u64) << 8 ^ p.made.0 as u64, id as u64) < tells
    {
        let q = &mut w.people[j];
        q.trust = (q.trust - OVER_TRUST).max(0.0);
        if q.stage == Stage::College && q.cal.lean > 0.0 {
            q.cal.lean *= OVER_LEAN;
            let today = w.today;
            w.people[j].cal.lean_events.push((today, "heard his head coach promised his spot", OVER_LEAN));
        }
        super::talks::mood_event(w, o, super::talks::Reason::Benched, -0.04);
        out.push(format!("{} heard you promised {name} the snaps ahead of him.", w.people[j].name));
    }
    // Two men promised one spot.
    if needs_passing(p.kind) {
        let pos = w.people[i].player.position;
        let others: Vec<usize> = (0..w.folk.promises.len())
            .filter(|&x| x != k)
            .filter(|&x| {
                let q = &w.folk.promises[x];
                q.kept.is_none()
                    && needs_passing(q.kind)
                    && matches!(q.to, PromiseTo::Player(o) if o != id
                        && super::person(w, o).is_some_and(|j| w.people[j].player.position == pos))
            })
            .collect();
        for x in others {
            let PromiseTo::Player(o) = w.folk.promises[x].to else { continue };
            let tie = super::ties::friends(w, id).contains(&o)
                || super::ties::friends(w, id).iter().any(|f| super::ties::friends(w, o).contains(f));
            let leak = if tie { 0.6 } else { 0.15 } + if p.public { 0.4 } else { 0.0 };
            if super::roll(w, 0x7A1C ^ (x as u64) << 8 ^ k as u64, id as u64) >= leak {
                continue;
            }
            for y in [k, x] {
                let b = &mut w.folk.promises[y].belief;
                *b *= 0.5;
            }
            w.folk.mine.word(Name::Room, -0.08);
            let other = super::person(w, o).map(|j| w.people[j].name.clone()).unwrap_or_default();
            out.push(format!("{name} and {other} found out you promised them both the same spot."));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// What a game showed him.

/// He got what you promised in a game: 1, half (the man ahead was hurt),
/// or nothing.
fn got(kind: PromiseKind, t: &Terms, g: &seen::Game) -> f64 {
    let share = g.share();
    let ok = match kind {
        PromiseKind::Start | PromiseKind::Join(0) | PromiseKind::Place(0) | PromiseKind::Recruit(0, _) => {
            g.started() && g.mate.is_none_or(|m| g.snaps() >= m.0)
        }
        PromiseKind::Snaps => share >= ROLE_SHARE.max(t.base as f64 + 0.1).min(0.6),
        PromiseKind::Join(_) | PromiseKind::Recruit(1, _) | PromiseKind::Place(1) => share >= ROLE_SHARE,
        PromiseKind::Touches => g.touches() as f64 >= (1.25 * t.base as f64).max(4.0),
        _ => share > 0.0,
    };
    if !ok {
        return 0.0;
    }
    // He knows it was the injury: playing then counts as half kept.
    if g.mate.is_some_and(|m| m.1) { 0.5 } else { 1.0 }
}

/// Promises judged over the games (section 7): playing time in its kinds.
pub fn over_games(kind: PromiseKind) -> bool {
    matches!(
        kind,
        PromiseKind::Start
            | PromiseKind::Snaps
            | PromiseKind::Touches
            | PromiseKind::Join(_)
            | PromiseKind::Place(0)
            | PromiseKind::Place(1)
            | PromiseKind::Recruit(0, _)
            | PromiseKind::Recruit(1, _)
    )
}

/// Whether he met his condition, from what you both can see (None while it
/// can't be told yet: fewer than two games for a stat line).
pub fn cond_met(w: &World, i: usize, p: &Promise, games: &[seen::Game]) -> Option<bool> {
    let t = &p.terms;
    let q = &w.people[i];
    match t.cond {
        Cond::None => Some(true),
        Cond::Stat => {
            let played: Vec<&StatLine> = games.iter().filter(|g| !g.hurt).filter_map(|g| g.line.as_ref()).collect();
            if played.len() < 2 {
                return None;
            }
            let (n, d) = played.iter().map(|l| t.stat.of_line(l)).fold((0.0, 0.0), |a, x| (a.0 + x.0, a.1 + x.1));
            if d <= 0.0 {
                return Some(false);
            }
            let v = n / d;
            Some(if t.stat.lower() { v <= t.bar as f64 } else { v >= t.bar as f64 })
        }
        Cond::Practice => {
            // Your staff's call: his work in practice against the man he
            // has to pass (the block's grades, else what they see of him).
            let grade = |id: u32| {
                w.training.grades.iter().find(|x| x.0 == id).filter(|x| x.2 > 0).map(|x| (x.1 / x.2 as f32) as f64)
            };
            let other = t.ahead.or_else(|| seen::ahead(w, i).first().copied());
            let Some(o) = other.and_then(|o| super::person(w, o)) else { return Some(true) };
            let (a, b) = (grade(q.player.id), grade(w.people[o].player.id));
            Some(match (a, b) {
                (Some(a), Some(b)) => a >= b,
                _ => q.player.overall() >= w.people[o].player.overall(),
            })
        }
        Cond::Conduct => {
            Some(!q.incident && !w.folk.rules.rulings.iter().any(|r| r.id == q.player.id && r.date >= p.made))
        }
        Cond::Grades => Some(!q.cal.spring_ineligible && !q.cal.fall_ineligible),
        Cond::Weight => {
            if w.today < w.dates.fall_camp && p.made < w.dates.fall_camp {
                return None;
            }
            let lbs = q.player.body.weight as f32;
            Some((lbs - t.bar).abs() <= 3.0 || weight_toward(q, t.bar))
        }
        Cond::Health => Some(!seen::hurt_on(w, q.player.id, w.today)),
    }
}

/// A weight condition is met past the mark in the direction asked (down
/// for a man asked to lose it, up for one asked to gain).
fn weight_toward(q: &Person, bar: f32) -> bool {
    let lbs = q.player.body.weight as f32;
    let ideal_heavy = matches!(q.player.position, Position::OT | Position::IOL | Position::DT);
    if ideal_heavy { lbs >= bar } else { lbs <= bar }
}

/// The games in a promise's window so far: from its date to today (or its
/// due day), with the named teammate's snaps.
fn window(w: &World, p: &Promise, to: Date) -> Vec<seen::Game> {
    let PromiseTo::Player(id) = p.to else { return Vec::new() };
    let from = opens(w, p.made, p.terms.by);
    seen::games(w, id, from, to, p.terms.ahead)
}

/// What he sees of an open promise today.
pub fn state(w: &World, p: &Promise) -> State {
    let PromiseTo::Player(id) = p.to else { return State::Waiting };
    let Some(i) = super::person(w, id) else { return State::Waiting };
    if !over_games(p.kind) {
        return State::Waiting;
    }
    if (p.terms.cond == Cond::Health || p.kind != PromiseKind::Touches) && seen::hurt_on(w, id, w.today) {
        return State::Paused;
    }
    let gs = window(w, p, w.today);
    let Some(last) = gs.iter().rev().find(|g| !g.hurt) else { return State::Waiting };
    let met = cond_met(w, i, p, &gs);
    if got(p.kind, &p.terms, last) > 0.0 {
        return if met == Some(false) && p.terms.cond == Cond::Stat { State::Missed } else { State::OnTrack };
    }
    match met {
        Some(false) => State::Missed,
        Some(true) if p.terms.cond != Cond::None => State::Owed,
        _ => State::Behind,
    }
}

// ---------------------------------------------------------------------------
// The week (section 4, "How it moves"; section 6).

/// Each Monday: every open promise to your players moves a quarter of the
/// way toward what the week showed him, his trust a fifth as much; behind
/// two straight weeks or owed a week, he comes to you; his own deadline
/// passed with nothing, his lean jumps.
pub fn week(w: &mut World, d: Date) {
    if w.seat.is_none() {
        return;
    }
    let open: Vec<usize> = (0..w.folk.promises.len())
        .filter(|&k| w.folk.promises[k].kept.is_none() && matches!(w.folk.promises[k].to, PromiseTo::Player(_)))
        .collect();
    for k in open {
        let p = w.folk.promises[k].clone();
        let PromiseTo::Player(id) = p.to else { continue };
        let Some(i) = super::person(w, id) else { continue };
        // A promise from before lane P1: he hears it now.
        if p.belief == 0.0 {
            let b = belief_own(w, i, p.kind, &p.terms, p.backed) as f32;
            w.folk.promises[k].belief = b;
            w.folk.promises[k].said = b;
        }
        let st = state(w, &w.folk.promises[k]);
        let x = &mut w.folk.promises[k];
        x.state = st as u8;
        match st {
            State::Behind => {
                x.behind = x.behind.saturating_add(1);
                x.owed = 0;
            }
            State::Owed => {
                x.owed = x.owed.saturating_add(1);
                x.behind = 0;
            }
            _ => {
                x.behind = 0;
                x.owed = 0;
            }
        }
        if let Some(target) = st.pull() {
            let b = x.belief as f64;
            let nb = b + WEEK_STEP * (target - b);
            x.belief = nb as f32;
            let q = &mut w.people[i];
            q.trust = (q.trust + (TRUST_SHARE * (nb - b)) as f32).clamp(0.0, 1.0);
        }
        // His own deadline.
        if let Some(by) = w.folk.promises[k].his_by
            && by <= d
        {
            w.folk.promises[k].his_by = None;
            if st != State::OnTrack && w.people[i].stage == Stage::College {
                let q = &mut w.people[i];
                q.cal.lean = (q.cal.lean.max(0.05) * DEADLINE_LEAN).min(1.0);
                q.cal.lean_events.push((d, "his own deadline on your promise passed", DEADLINE_LEAN));
                let text = format!("{} told you he'd look elsewhere if he didn't have it by now.", q.name);
                super::note(w, "talk", "Your promises".into(), text, None);
            }
        }
        // He comes to you.
        let x = &w.folk.promises[k];
        let comes = x.behind >= BEHIND_WEEKS || x.owed >= OWED_WEEKS;
        if comes && x.came.is_none_or(|c| c.days_to(d) >= 21) && !super::talks::has_open_talk(w, id) {
            w.folk.promises[k].came = Some(d);
            super::talks::promise_asks(w, i, d);
        }
    }
    w.folk.mine.fade_week();
}

// ---------------------------------------------------------------------------
// Judging (section 7).

/// What a promise came to on its day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    Kept,
    /// Broken, by the share missed (0-1].
    Broken(f64),
    /// He missed his condition: you are released, as far as he sees it.
    /// The share of the cost that still holds (a practice call he thinks
    /// unfair).
    Released(f64),
    /// Nothing to judge: hurt all its term, or he left on his own.
    Lapsed,
}

/// Judges a promise over its games, when it is one of those kinds.
pub fn judge(w: &World, p: &Promise) -> Option<Verdict> {
    if !over_games(p.kind) {
        return None;
    }
    let PromiseTo::Player(id) = p.to else { return None };
    let i = super::person(w, id)?;
    let gs = window(w, p, p.due.min(w.today));
    let healthy: Vec<&seen::Game> = gs.iter().filter(|g| !g.hurt).collect();
    if healthy.is_empty() {
        return Some(Verdict::Lapsed);
    }
    let r = healthy.iter().map(|g| got(p.kind, &p.terms, g)).sum::<f64>() / healthy.len() as f64;
    // He got it: kept, whatever his condition came to.
    if r >= KEPT_SHARE {
        return Some(Verdict::Kept);
    }
    let met = cond_met(w, i, p, &gs);
    let share = (1.0 - r).clamp(0.0, 1.0);
    if met == Some(false) {
        // A stat, his conduct, his grades, his weight: plain, released in
        // full. A practice call rests on his belief that you judged fairly.
        let holds = if p.terms.cond == Cond::Practice { share * (1.0 - p.belief as f64) } else { 0.0 };
        return Some(Verdict::Released(holds));
    }
    Some(Verdict::Broken(share))
}

/// What keeping a promise pays: trust +0.10 x (2 - his belief when you said
/// it), up to +0.2 from a doubter.
pub fn kept_trust(said: f32) -> f32 {
    let said = if said <= 0.0 { 1.0 } else { said };
    0.10 * (2.0 - said.clamp(0.0, 1.0))
}

/// What breaking it costs, as a share of the old cost: 0.4 + 0.6 x his
/// belief, times the share missed.
pub fn broken_scale(belief: f32, share: f64) -> f32 {
    let b = if belief <= 0.0 { 1.0 } else { belief.clamp(0.0, 1.0) };
    (0.4 + 0.6 * b) * share as f32
}

/// The names a promise to this person reaches: your room for your players
/// (and the agents in the NFL); the trail for recruits and transfers.
pub fn audiences(w: &World, p: &Promise) -> Vec<Name> {
    let pro = match p.to {
        PromiseTo::Player(id) => super::person(w, id).is_some_and(|i| w.people[i].stage == Stage::Pro),
        _ => false,
    };
    let mut v = vec![Name::Room];
    if pro {
        v.push(Name::Agents);
    }
    if matches!(p.kind, PromiseKind::Recruit(..) | PromiseKind::Join(_)) {
        v.push(Name::Trail);
    }
    v
}

// ---------------------------------------------------------------------------
// The portal meeting (section 8).

/// Playing time's weight among his wants: 1 first, 0.6 second, 0.35 third,
/// 0.15 otherwise.
pub fn time_weight(p: &Person) -> f64 {
    let wn = &p.family.own_wants;
    let me = wn.playing_time;
    let above =
        [wn.money, wn.close_to_home, wn.academics, wn.development, wn.prestige].iter().filter(|&&x| x > me).count();
    match above {
        0 => 1.0,
        1 => 0.6,
        2 => 0.35,
        _ => 0.15,
    }
}

/// What a promise adds over the role he sees now: a start to a backup 1.0,
/// to a rotation player 0.6, to a starter 0.15; a role to a backup 0.6, to
/// a rotation player 0.2, to a starter 0.
pub fn adds(w: &World, i: usize, kind: PromiseKind) -> f64 {
    let first = seen::starter(w, i);
    let share = share_now(w, i);
    let role = if first {
        PlayingTime::Starter
    } else if share >= ROLE_SHARE {
        PlayingTime::Rotation
    } else {
        PlayingTime::Backup
    };
    let start = needs_passing(kind);
    match (start, role) {
        (true, PlayingTime::Starter) => 0.15,
        (true, PlayingTime::Rotation) => 0.6,
        (true, _) => 1.0,
        (false, PlayingTime::Starter) => 0.0,
        (false, PlayingTime::Rotation) => 0.2,
        _ => 0.6,
    }
}

/// The portal meeting's pull, 0 to 0.5: 0.5 x weight x adds x belief. A
/// fair condition lowers it a little (a sure thing is worth more than a
/// chance).
pub fn portal_pull(w: &World, i: usize, p: &Promise) -> f64 {
    let cond = if p.terms.cond != Cond::None { 0.85 } else { 1.0 };
    (PULL_CAP * time_weight(&w.people[i]) * adds(w, i, p.kind) * p.belief as f64 * cond).clamp(0.0, PULL_CAP)
}

/// A playing-time promise: the kinds the portal meeting reads by what they
/// add.
pub fn playing_time(kind: PromiseKind) -> bool {
    matches!(
        kind,
        PromiseKind::Start | PromiseKind::Snaps | PromiseKind::Touches | PromiseKind::Place(0) | PromiseKind::Place(1)
    )
}

/// Weight of playing time for a recruit's camp (section 9).
pub fn time_care(p: &Person, ear: Ear) -> f64 {
    super::claims::group_cares(p, ear)[Group::PlayingTime as usize] as f64
}

// ---------------------------------------------------------------------------
// The NFL (question 3): a club's free agency start promise is judged on his
// starts in its first 4 games.

/// He started 3 of every 4 of his club's games he was healthy for (the
/// season's starts from the game book), or, with no book, he heads its
/// chart. None while he hasn't been healthy for any.
pub fn nfl_start_kept(w: &World, i: usize, club: usize) -> Option<bool> {
    let id = w.people[i].player.id;
    let line = w
        .tallies
        .players
        .get(crate::world::tally::Level::Nfl, id)
        .filter(|s| s.team as usize == club && w.tallies.season == w.dates.season);
    match line {
        Some(s) if s.games > 0 => Some(s.starts as f64 >= KEPT_SHARE * s.games.min(4) as f64),
        Some(_) => None,
        None => {
            let pos = w.people[i].player.position;
            Some(
                w.clubs[club]
                    .depth
                    .get(pos.index())
                    .is_some_and(|d| d.iter().take(crate::league::depth_slots(pos).starters).any(|&j| j == i)),
            )
        }
    }
}
