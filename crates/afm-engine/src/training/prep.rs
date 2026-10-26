//! A player's weekly preparation (update 2, lane 4: TR36-TR43). Each week
//! the staff run something, a player's prep is read from what he really
//! did: his reps on the plan by his role, with the mental reps a backup
//! takes behind the play (TR37), the installs, film and walkthroughs he sat
//! through, his own study (his work ethic and professionalism; the tablet
//! from about 2011, Hudl at a high school), the sessions he missed, his
//! status (hurt, rested, new, away at another sport, absent from a
//! voluntary week), and an install too full to take in. It reads as one of
//! five words.
//!
//! Prep plays out only through the battles (TR38): a poorly prepared
//! player busts more assignments, fills the wrong gap, reads late and
//! throws off rhythm (up to about a third more mental errors); a sharp one
//! a little less. It is never a flat cut to his ratings. A run of poor
//! weeks rusts his reading and his technique (TR39: up to about 3 points,
//! won back over two or three good weeks) and slows his growth that season
//! (TR40). An average staff's default week is neutral (TR41), and the AI's
//! teams follow the same rule (TR42). The sizes were agreed with the lane
//! (2026-10-24, "4. yes"); the weights inside them are proposed, not
//! confirmed.

use super::plan::{Kind, Period, Week, Who};
use super::rules::Tier;
use super::work::{Role, prep_weight, reps};
use super::{FAMILIES, Family, Train, family_of};
use crate::attributes::Attr;
use crate::player::{Player, Trait};
use crate::positions::Position;
use crate::rng::clamp;
use crate::world::season::key_uniform;

/// The words, best last (0: no prep that week: the players' own time,
/// before he arrived).
pub const WORDS: [&str; 6] = ["None", "Unprepared", "Underprepared", "Patchy", "Prepared", "Sharp"];

/// A player's week (TR36).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Status {
    Full,
    /// Practising with a limit (a knock, a pitch count, coming back).
    Limited,
    /// A rest day or two (a veteran's day off).
    Rested,
    /// Hurt: the rehab group, the meetings he can make.
    Out,
    /// At another sport (a high schooler's season).
    Away,
    /// Not at a voluntary week.
    Absent,
    /// Arrived in the last two weeks.
    New,
}

impl Status {
    pub fn label(self) -> &'static str {
        ["Full", "Limited", "Rested", "Out, rehab", "At another sport", "Absent", "New"][self as usize]
    }
    pub fn name(self) -> &'static str {
        ["full", "limited", "rested", "out", "away", "absent", "new"][self as usize]
    }
    /// The share of the field work and of the classroom he is there for.
    pub fn there(self) -> (f64, f64) {
        match self {
            Status::Full | Status::New => (1.0, 1.0),
            Status::Limited => (0.55, 1.0),
            // His rest days are already out of his week.
            Status::Rested => (1.0, 1.0),
            Status::Out => (0.0, 0.75),
            Status::Away => (0.25, 0.45),
            Status::Absent => (0.0, 0.0),
        }
    }
}

/// What a week gives a role for its prep, in minutes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Parts {
    /// Plan reps (team, situations, 7-on-7 on this week's install), the
    /// ones he doesn't take counted at half as mental reps (TR37).
    pub reps: f64,
    /// Installs and unit and position meetings.
    pub install: f64,
    /// The opponent's film and cut-ups (in season), any film out of it.
    pub film: f64,
    pub walks: f64,
    /// The plan test, signals, VR and the tablet in the plan.
    pub study: f64,
    /// The staff's work he is in: practice and the strength staff's.
    pub work: f64,
    /// The install's size: minutes of new plays.
    pub calls: f64,
    /// Days with something counted (a miss takes a day).
    pub days: u8,
    pub game: bool,
    /// A game plan test this week (the staff's read is exact).
    pub test: bool,
}

/// A week's prep parts for a role.
pub fn parts(wk: &Week, role: Role, camp: bool) -> Parts {
    let game = wk.days.iter().any(|d| d.game);
    let mut x = Parts { game, ..Default::default() };
    for d in &wk.days {
        if d.periods.iter().any(|p| p.counted()) {
            x.days += 1;
        }
        for p in &d.periods {
            let m = p.min as f64;
            // In season only this week's plan counts; out of it, the
            // system (the new one, the self-scout, the winter's film).
            let w = if game { prep_weight(p) } else { 1.0 };
            match p.kind {
                Kind::Install => {
                    x.install += m * w;
                    x.calls += m * if game { w } else { 1.0 };
                }
                Kind::Meeting => x.install += m * w * if p.what == 0 { 0.5 } else { 1.0 },
                Kind::Film => x.film += m * w,
                Kind::RulesClinic | Kind::Vision => x.film += m * 0.5,
                Kind::Walkthrough => x.walks += m * w,
                Kind::PlanTest | Kind::Signals | Kind::VrReps | Kind::HomeStudy => {
                    x.study += m * w;
                    x.test |= p.kind == Kind::PlanTest;
                }
                Kind::Team | Kind::Situations | Kind::SevenOnSeven if game => {
                    x.reps += m * w * rep_share(p, role, camp)
                }
                _ => {}
            }
            if p.counted() && (p.practice() || p.kind.strength_staff() || p.kind == Kind::WeightRoom) {
                x.work += m * reps(p.who, role, camp);
            }
        }
    }
    x
}

/// His share of a plan period's reps: his own, and the rest at half as
/// mental reps behind the play (TR37). The scout team runs the opponent's
/// plays, so its reps aren't his plan's.
fn rep_share(p: &Period, role: Role, camp: bool) -> f64 {
    let own = reps(p.who, role, camp);
    let own = if p.who == Who::Ones && role == Role::Young { 0.0 } else { own.min(1.0) };
    own + (1.0 - own) * 0.5
}

/// A team's week for prep against the neutral week's, per role: the AI's
/// shared shape (`week::TeamWeek`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PrepWeek {
    /// The staff run nothing: no word this week.
    pub none: bool,
    pub game: bool,
    pub test: bool,
    /// Per role: reps, installs, film, walkthroughs, plan study, work, the
    /// install's size, each against the neutral week's.
    pub r: [[f32; 7]; super::work::ROLES],
    pub days: u8,
}

fn ratio(a: f64, b: f64, pad: f64, cap: f64) -> f64 {
    ((a + pad) / (b + pad)).min(cap)
}

/// This week's parts against the neutral week's for a role (`mine` is the
/// week he really had: his rest days and rehab taken out).
pub fn against(mine: &Parts, neutral: &Parts, starter: &Parts) -> [f32; 7] {
    // Reps read half against his role's usual, half against a starter's:
    // a backup is less ready for the plan than the starter, but a neutral
    // week leaves him well prepared for his job.
    let reps = 0.5 * ratio(mine.reps, neutral.reps, 5.0, 1.2) + 0.5 * ratio(mine.reps, starter.reps, 5.0, 1.2);
    [
        reps as f32,
        ratio(mine.install, neutral.install, 10.0, 1.2) as f32,
        ratio(mine.film, neutral.film, 10.0, 1.2) as f32,
        ratio(mine.walks, neutral.walks, 10.0, 1.2) as f32,
        ratio(mine.study, neutral.study, 10.0, 1.2) as f32,
        ratio(mine.work, neutral.work, 30.0, 1.15) as f32,
        ratio(mine.calls, neutral.calls, 15.0, 2.0) as f32,
    ]
}

/// A team's week for prep (the AI's, and your staff's before a player's own
/// changes).
pub fn prep_week(wk: &Week, nk: &Week, camp: bool, none: bool) -> PrepWeek {
    let ns: Parts = parts(nk, Role::Starter, camp);
    let mut out = PrepWeek { none, ..Default::default() };
    for role in Role::ALL {
        let mine = parts(wk, role, camp);
        let n = parts(nk, role, camp);
        out.r[role as usize] = against(&mine, &n, &ns);
        out.game = mine.game;
        out.test = mine.test;
        out.days = mine.days;
    }
    out
}

/// The football IQ the level's install is built for (proposed, not
/// confirmed: about a level's middle starter).
fn iq_norm(tier: Tier) -> f64 {
    match tier {
        Tier::Pro => 66.0,
        Tier::College => 56.0,
        Tier::High => 44.0,
    }
}

/// How hard he studies on his own (TR36): his work ethic and
/// professionalism; the tablet from 2011 (college and the NFL), Hudl at a
/// high school from 2012 (proposed, not confirmed).
pub fn own_study(p: &Player, tier: Tier, season: i32) -> f64 {
    let t = &p.personality.traits;
    let era = match tier {
        Tier::High if season >= 2012 => 0.06,
        Tier::College | Tier::Pro if season >= 2011 => 0.08,
        _ => 0.0,
    };
    clamp(0.82 + 0.025 * (p.personality.work_ethic - 10.0) + 0.12 * t.z(Trait::Professionalism) + era, 0.45, 1.15)
}

/// How much of the room's work he takes in (engagement): work ethic,
/// professionalism, a little ambition.
fn engaged(p: &Player) -> f64 {
    let t = &p.personality.traits;
    clamp(
        0.88 + 0.018 * (p.personality.work_ethic - 10.0)
            + 0.07 * t.z(Trait::Professionalism)
            + 0.03 * t.z(Trait::Ambition),
        0.6,
        1.08,
    )
}

/// The chance he misses a session day (late, sick, academics, a family
/// matter), by work ethic and professionalism.
fn miss_chance(p: &Player) -> f64 {
    let t = &p.personality.traits;
    clamp(0.012 + 0.008 * (10.0 - p.personality.work_ethic).max(0.0) - 0.012 * t.z(Trait::Professionalism), 0.0, 0.14)
}

/// One player's prep for one week, with the reasons a screen shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prep {
    pub word: u8,
    pub score: f64,
    pub status: Status,
    /// Each part against an average staff's week (1 is the usual).
    pub reps: f64,
    pub install: f64,
    pub film: f64,
    pub walks: f64,
    pub own: f64,
    pub work: f64,
    /// The share of the install he took in.
    pub took: f64,
    /// Session days missed.
    pub missed: u8,
    pub days: u8,
    /// The mental share of what he lacked (rust splits by it).
    pub mental: f64,
    pub game: bool,
    pub test: bool,
}

impl Prep {
    /// No prep this week (the players' own time).
    pub fn none(status: Status) -> Prep {
        Prep {
            word: 0,
            score: 0.0,
            status,
            reps: 0.0,
            install: 0.0,
            film: 0.0,
            walks: 0.0,
            own: 0.0,
            work: 0.0,
            took: 1.0,
            missed: 0,
            days: 0,
            mental: 0.5,
            game: false,
            test: false,
        }
    }

    /// What his staff would say is behind a poor week, in a few words.
    pub fn why(&self) -> &'static str {
        match self.status {
            Status::Out => return "in rehab, he only makes the meetings",
            Status::Absent => return "not at the voluntary sessions",
            _ => {}
        }
        if self.missed >= 2 {
            "missed two or more days"
        } else if self.took < 0.8 {
            "the install was more than he could take in"
        } else if self.install < 0.85 || self.film < 0.85 {
            "short on installs and film"
        } else if self.own < 0.7 {
            "isn't studying on his own"
        } else if self.game && self.reps < 0.75 {
            "few reps on the plan"
        } else if self.missed == 1 {
            "missed a day"
        } else {
            "not locked in this week"
        }
    }
}

/// The word for a score.
pub fn word_of(score: f64) -> u8 {
    if score >= 0.92 {
        5
    } else if score >= 0.8 {
        4
    } else if score >= 0.66 {
        3
    } else if score >= 0.5 {
        2
    } else {
        1
    }
}

/// His week's prep from the parts against the neutral week (`r`, from
/// `against`). `years` is his seasons in this system; `key` the week's key
/// (the same in the view and the Monday pass).
#[allow(clippy::too_many_arguments)]
pub fn read(
    p: &Player,
    r: &[f32; 7],
    game: bool,
    test: bool,
    days: u8,
    status: Status,
    tier: Tier,
    season: i32,
    years: u32,
    key: u64,
) -> Prep {
    // A player at another sport is there by agreement: his staff reads his
    // week against the share he agreed to (lifting on his off days, the
    // film at home), so the sport itself never makes him poorly prepared.
    let (field, room) = if status == Status::Away { (1.0, 1.0) } else { status.there() };
    // Session days missed (keyed by the week and the player).
    let chance = miss_chance(p);
    let missed = (0..days).filter(|&k| key_uniform(key ^ 0x9E9 ^ (k as u64) << 40, p.id as u64) < chance).count() as u8;
    let kept = if days == 0 { 1.0 } else { 1.0 - missed as f64 / days as f64 };
    let r = r.map(|x| x as f64);
    let own = own_study(p, tier, season);
    let e = engaged(p);
    // The mental side: installs, film, walkthroughs and study; out of
    // season his own study weighs more (the playbook on his own).
    let mind = if game {
        0.35 * r[1] + 0.25 * r[2] + 0.2 * r[3] + 0.2 * (0.5 * r[4] + 0.5 * own)
    } else {
        0.3 * r[1] + 0.2 * r[2] + 0.1 * r[3] + 0.4 * (0.25 * r[4] + 0.75 * own)
    };
    let m = mind * e * room * kept;
    // An install too full to take in (TR36): his capacity against this
    // week's new plays.
    let iq = p.attributes.get(Attr::FootballIq);
    let cap = 1.15 + 0.008 * (iq - iq_norm(tier)) + 0.04 * years.min(3) as f64
        - if status == Status::New { 0.2 } else { 0.0 };
    let took = clamp(cap / r[6].max(0.5), 0.4, 1.0);
    let (score, body) = if game {
        let reps = r[0] * field * kept;
        ((0.3 * reps + 0.7 * m) * (0.6 + 0.4 * took), reps)
    } else {
        let work = r[5] * field * kept;
        ((0.4 * work + 0.6 * m) * (0.85 + 0.15 * took), work)
    };
    let (dm, db) = ((1.0 - m).max(0.0), (1.0 - body).max(0.0));
    let mental = if dm + db > 0.0 { dm / (dm + db) } else { 0.5 };
    Prep {
        word: word_of(score),
        score,
        status,
        reps: r[0],
        install: r[1],
        film: r[2],
        walks: r[3],
        own,
        work: r[5],
        took,
        missed,
        days,
        mental,
        game,
        test,
    }
}

/// The week's key for prep: the same in the Monday pass and in a screen
/// that reads the week ahead.
pub fn key(seed: u64, monday: crate::calendar::Date) -> u64 {
    seed ^ (monday.0 as u64) << 28 ^ 0x9E9A_0000
}

/// Out hurt for this week's practices. An NFL injury heals on Tuesday,
/// after Monday's pass, so a player with one week left practices from
/// Wednesday and plays on Sunday.
pub fn out_this_week(out_weeks: u32) -> bool {
    out_weeks > 1
}

/// Bit 15 of `Train::prep`: this week's word was read while he was out
/// hurt. A page later in the week reads his week as the pass did, after
/// Tuesday's healing or a game's new injury.
const OUT_BIT: u16 = 0x8000;

/// Marks this week's recorded word as read while he was out hurt.
pub fn mark_out(t: &mut Train) {
    t.prep |= OUT_BIT;
}

/// Whether this week's recorded word was read while he was out hurt.
pub fn was_out(t: &Train) -> bool {
    t.prep & OUT_BIT != 0
}

/// The last five weeks' words, this week's first.
pub fn words(t: &Train) -> [u8; 5] {
    std::array::from_fn(|k| ((t.prep >> (3 * k)) & 7) as u8)
}

/// His latest week's word.
pub fn latest(t: &Train) -> u8 {
    (t.prep & 7) as u8
}

/// What a word does on game day to his mental errors (TR38): busts, wrong
/// fits and late reads (proposed, not confirmed inside the agreed third).
pub fn game_factor(word: u8) -> f64 {
    [1.0, 1.33, 1.2, 1.08, 0.975, 0.93][word.min(5) as usize]
}

/// How much of his growth in the families prep feeds lands this week after
/// a run of poor weeks (TR40): five unprepared weeks cost about an eighth.
pub fn growth_factor(t: &Train) -> f64 {
    let s: f64 = words(t)
        .iter()
        .map(|&w| match w {
            1 => 1.0,
            2 => 0.6,
            _ => 0.0,
        })
        .sum();
    1.0 - 0.024 * s
}

/// The families a run of poor prep slows (technique, kicking, the mental
/// side).
pub fn slows(f: usize) -> bool {
    matches!(Family::ALL[f], Family::Technique | Family::Kicking | Family::Mental)
}

/// Applies a run of poor prep to a week's growth shares.
pub fn slow(t: &Train, s: &mut [f64; FAMILIES]) {
    let g = growth_factor(t);
    if g < 1.0 {
        for (f, x) in s.iter_mut().enumerate() {
            if slows(f) {
                *x *= g;
            }
        }
    }
}

/// The reading attributes rust takes (TR39).
pub const READING: [Attr; 2] = [Attr::PlayRecognition, Attr::Anticipation];

/// The technique attributes rust takes: the two his position leans on most.
pub fn technique(pos: Position) -> [Attr; 2] {
    let mut best: [(Attr, f64); 2] = [(Attr::FootballIq, -1.0), (Attr::FootballIq, -1.0)];
    for a in Attr::ALL {
        if family_of(a) != Family::Technique {
            continue;
        }
        let w = crate::positions::weight(pos, a);
        if w > best[0].1 {
            best[1] = best[0];
            best[0] = (a, w);
        } else if w > best[1].1 {
            best[1] = (a, w);
        }
    }
    [best[0].0, best[1].0]
}

/// Moves attributes by `by` points (negative: rust; positive: won back,
/// never past his ceiling unless he is already past it).
fn shift(p: &mut Player, attrs: &[Attr], by: f64) {
    for &a in attrs {
        let now = p.attributes.get(a);
        let to = if by > 0.0 { (now + by).min(p.ceilings.get(a).max(now)) } else { (now + by).max(1.0) };
        p.attributes.set(a, to);
    }
}

/// Rust in points: reading and technique.
pub fn rust(t: &Train) -> (f64, f64) {
    (t.rust[0] as f64 / 10.0, t.rust[1] as f64 / 10.0)
}

/// Records a week's word (`mental`: the mental share of what he lacked),
/// and rusts or wins back his reading and technique (TR39): two poor weeks
/// in a row start the rust (an unprepared week a point, an underprepared
/// one 0.6, split by what he lacked), up to 3 points each; a patchy week
/// wins back 0.3, a prepared one 1.2, a sharp one 1.5, a week with no prep
/// a quarter point.
pub fn record(p: &mut Player, word: u8, mental: f64) {
    let prev = latest(&p.train);
    p.train.prep = ((p.train.prep << 3) | word.min(5) as u16) & 0x7FFF;
    let poor = |w: u8| w == 1 || w == 2;
    let add = if poor(word) && poor(prev) { if word == 1 { 1.0 } else { 0.6 } } else { 0.0 };
    let back = match word {
        0 => 0.25,
        3 => 0.3,
        4 => 1.2,
        5 => 1.5,
        _ => 0.0,
    };
    let a = clamp(mental, 0.0, 1.0);
    let (rd, tc) = rust(&p.train);
    let nrd = clamp(rd + add * (0.5 + 0.5 * a) - back, 0.0, 3.0);
    let ntc = clamp(tc + add * (0.5 + 0.5 * (1.0 - a)) - back, 0.0, 3.0);
    let (nrd, ntc) = ((nrd * 10.0).round() / 10.0, (ntc * 10.0).round() / 10.0);
    if nrd != rd {
        shift(p, &READING, rd - nrd);
    }
    if ntc != tc {
        let tech = technique(p.position);
        shift(p, &tech, tc - ntc);
    }
    p.train.rust = [(nrd * 10.0) as u8, (ntc * 10.0) as u8];
}

/// Gives all rust back at once (before a position change: the technique it
/// took is his old position's).
pub fn unrust(p: &mut Player) {
    let (rd, tc) = rust(&p.train);
    if rd > 0.0 {
        shift(p, &READING, rd);
    }
    if tc > 0.0 {
        let tech = technique(p.position);
        shift(p, &tech, tc);
    }
    p.train.rust = [0, 0];
}

/// The staff's read of his word (TR30): exact with a game plan test that
/// week, otherwise one word off now and then.
pub fn staff_read(word: u8, test: bool, key: u64, id: u32) -> u8 {
    if word == 0 || test {
        return word;
    }
    let u = key_uniform(key ^ 0x5EAD, id as u64);
    if u < 0.12 {
        (word + 1).min(5)
    } else if u < 0.24 {
        word.saturating_sub(1).max(1)
    } else {
        word
    }
}
