//! Game-day talks (dialogue D4-D6): the pregame talk in the locker room, the
//! halftime talk and the sideline word after a costly mistake. Every team
//! has them. An AI head coach reads his room by his motivation and picks;
//! yours are your own picks, or your own coach profile's when you leave them
//! (Cain, game day: his own ratings call what he doesn't take).
//!
//! A talk lands on each player by who he is: his temper on the field, his
//! ego, his composure, his years, how much he believes the man talking, and
//! what the day already does to him (the stakes, the rival, the paper, the
//! week of practice). It moves three things in him: how amped he is (fire),
//! how settled (poise) and how locked in (focus). Each position plays best at
//! its own pitch, and each man his own about it (coach fire): a guard can
//! use more fire than a quarterback, a kicker least of all, a hot-tempered
//! hitter more than a calm pocket passer. On the field fire works by the
//! kind of work (coach fire, `docs/design/coach-fire.md`): toward his pitch
//! it lifts power and effort (strength, leg drive, pursuit, motor, his hit,
//! a little burst), past it it costs fine and thinking work (the passer's
//! accuracy and touch, decisions, reads, hands, discipline and
//! concentration), and it moves how often he goes for the big hit, the
//! strip, comes downhill, seeks contact, guesses the count or jumps a
//! route; each at most 3 points. A talk also moves focus (pre-snap fouls,
//! busts, drops), composure (how hard the next mistake bites) and heat
//! (late hits), all fading through the quarter: the young settle after the
//! first hits, the veterans hold it longer. Every size here is Claude's
//! (proposed, not confirmed), except the caps of 3 (Cain, 2026-11-17).

use super::roster::{GamePlayer, GameTeam};
use super::rules::TRAIT_STEP;
use crate::attributes::Attr;
use crate::player::Level;
use crate::positions::Position;
use crate::tendencies::Tend;

/// What you do with the room (the one-on-one's moves, and the captain).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Move {
    Ask,
    #[default]
    Straight,
    Praise,
    Challenge,
    Criticize,
    Promise,
    Warn,
    Listen,
    /// Hand the talk to a captain (pregame and halftime).
    Captain,
    /// Say nothing: the room as it is.
    Nothing,
}

impl Move {
    pub const ALL: [Move; 9] = [
        Move::Ask,
        Move::Straight,
        Move::Praise,
        Move::Challenge,
        Move::Criticize,
        Move::Promise,
        Move::Warn,
        Move::Listen,
        Move::Captain,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Move::Ask => "ask",
            Move::Straight => "straight",
            Move::Praise => "praise",
            Move::Challenge => "challenge",
            Move::Criticize => "criticize",
            Move::Promise => "promise",
            Move::Warn => "warn",
            Move::Listen => "listen",
            Move::Captain => "captain",
            Move::Nothing => "nothing",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Move::Ask => "Ask",
            Move::Straight => "Tell them straight",
            Move::Praise => "Praise",
            Move::Challenge => "Challenge",
            Move::Criticize => "Criticize",
            Move::Promise => "Promise",
            Move::Warn => "Warn",
            Move::Listen => "Listen",
            Move::Captain => "Hand it to a captain",
            Move::Nothing => "Say nothing",
        }
    }
    pub fn from_key(k: &str) -> Option<Move> {
        Move::ALL.into_iter().chain([Move::Nothing]).find(|m| m.key() == k)
    }
}

/// How you say it (the one-on-one's four tones, in their order).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Tone {
    Calm,
    Warm,
    #[default]
    Firm,
    Fiery,
}

impl Tone {
    pub const ALL: [Tone; 4] = [Tone::Fiery, Tone::Warm, Tone::Firm, Tone::Calm];
    pub fn index(self) -> usize {
        match self {
            Tone::Calm => 0,
            Tone::Warm => 1,
            Tone::Firm => 2,
            Tone::Fiery => 3,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Tone::Calm => "calm",
            Tone::Warm => "warm",
            Tone::Firm => "firm",
            Tone::Fiery => "fiery",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Tone::Calm => "Calm",
            Tone::Warm => "Warm",
            Tone::Firm => "Firm",
            Tone::Fiery => "Fiery",
        }
    }
    pub fn from_key(k: &str) -> Option<Tone> {
        Tone::ALL.into_iter().find(|t| t.key() == k)
    }
}

/// What the talk is about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Subject {
    /// The game's stakes (at halftime: the half).
    Stakes,
    /// Us: our standard, who we are.
    #[default]
    Us,
    /// Them: the opponent, the rival, what they said.
    Them,
    /// The seniors (the veterans at the NFL).
    Seniors,
    /// The plan (at halftime: the adjustments).
    Plan,
    /// One player (the sideline word is always about him).
    One,
}

impl Subject {
    pub const ALL: [Subject; 6] =
        [Subject::Stakes, Subject::Us, Subject::Them, Subject::Seniors, Subject::Plan, Subject::One];
    pub fn key(self) -> &'static str {
        match self {
            Subject::Stakes => "stakes",
            Subject::Us => "us",
            Subject::Them => "them",
            Subject::Seniors => "seniors",
            Subject::Plan => "plan",
            Subject::One => "one",
        }
    }
    pub fn from_key(k: &str) -> Option<Subject> {
        Subject::ALL.into_iter().find(|s| s.key() == k)
    }
}

/// A talk as picked: the move, the tone, the subject and, for "one player"
/// or the captain, who.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Pick {
    pub mv: Move,
    pub tone: Tone,
    pub subject: Subject,
    /// The player the talk is about, or the captain who speaks (0: none).
    #[serde(default)]
    pub who: u32,
}

impl Pick {
    pub fn new(mv: Move, tone: Tone, subject: Subject) -> Pick {
        Pick { mv, tone, subject, who: 0 }
    }
}

/// The day as the world sees it for one team (set when the world builds the
/// team for a game; the default is a plain game).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Day {
    /// The game's stakes for this team, 0-1; 0 lets the engine read them
    /// from the playoffs and the season's push.
    pub stakes: f32,
    /// A rivalry game.
    pub rival: bool,
    /// A player of theirs quoted in the paper about us, and the unit he
    /// talked about (`Unit` index).
    pub quote: Option<(u32, u8)>,
    /// How worn each tone is from your last talks, 0-1 (`Tone::index`).
    pub worn: [f32; 4],
    /// Talks handed to a captain in the team's last four games.
    pub handed: u8,
    /// The seniors' last home game.
    pub seniors: bool,
    /// Last week: 1 a win, -1 a loss, 0 no game.
    pub last: i8,
    /// The team's captains (empty: the room's natural leaders stand in).
    pub captains: Vec<u32>,
    /// A starter out today (the room plays for him).
    pub hurt: Option<u32>,
    /// The other side's head coach handed this room bulletin-board
    /// material this week (coach voice: your fiery words about them at the
    /// press conference).
    pub provoked: bool,
    /// Players who sit the first half (coach fire: a targeting ejection in
    /// the second half of his last game, under the college rule).
    pub first_half: Vec<u32>,
    /// The talk the head coach gives his room today, pregame and at
    /// halftime, whatever his read, and the tone his sideline words keep
    /// to; None, he reads the room. The world never sets it: the measuring
    /// tools play a prototype's room with it (`fire_check`).
    pub room: Option<Pick>,
}

/// The groups a room splits into on the screens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Unit {
    Qb,
    Line,
    Skill,
    Front,
    Backers,
    Secondary,
    Specialists,
}

impl Unit {
    pub const ALL: [Unit; 7] =
        [Unit::Qb, Unit::Line, Unit::Skill, Unit::Front, Unit::Backers, Unit::Secondary, Unit::Specialists];
    pub fn of(p: Position) -> Unit {
        match p {
            Position::QB => Unit::Qb,
            Position::OT | Position::IOL => Unit::Line,
            Position::RB | Position::WR | Position::TE => Unit::Skill,
            Position::EDGE | Position::DT => Unit::Front,
            Position::LB => Unit::Backers,
            Position::CB | Position::S => Unit::Secondary,
            Position::K | Position::P | Position::LS => Unit::Specialists,
        }
    }
    pub fn index(self) -> usize {
        Unit::ALL.iter().position(|&u| u == self).unwrap_or(0)
    }
    pub fn name(self) -> &'static str {
        match self {
            Unit::Qb => "Quarterbacks",
            Unit::Line => "Offensive line",
            Unit::Skill => "Backs, receivers",
            Unit::Front => "Defensive front",
            Unit::Backers => "Linebackers",
            Unit::Secondary => "Secondary",
            Unit::Specialists => "Specialists",
        }
    }
}

/// A player's state of mind on game day, each 0-1: how amped, how settled,
/// how locked in.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mind {
    pub fire: f32,
    pub poise: f32,
    pub focus: f32,
}

/// Why a talk landed the way it did on one player, for the screens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Why {
    /// It barely reached him (veteran, low belief, worn tone).
    #[default]
    Unmoved,
    /// Settled: calmer and steadier.
    Settled,
    /// Fired up toward his best pitch.
    FiredUp,
    /// Over the top: too much fire for his spot.
    OverAmped,
    /// Wound tight: composure down.
    Tight,
    /// Shrank: it hit his belief in himself.
    Shrank,
    /// Took it personally (a challenge to a proud competitor).
    Personal,
    /// Locked in: focus up.
    Locked,
    /// It rang hollow (praise after a loss, a worn tone).
    Hollow,
    /// He goes where the captain goes.
    Follows,
    /// Flattened: the fire went out of him.
    Flat,
}

impl Why {
    pub fn words(self) -> &'static str {
        match self {
            Why::Unmoved => "Unmoved.",
            Why::Settled => "Settled.",
            Why::FiredUp => "Fired up.",
            Why::OverAmped => "Over the top.",
            Why::Tight => "Wound tight.",
            Why::Shrank => "Shrinks a little.",
            Why::Personal => "Took it personally.",
            Why::Locked => "Locked in.",
            Why::Hollow => "It rang hollow.",
            Why::Follows => "Goes where the captain goes.",
            Why::Flat => "Flat.",
        }
    }
    /// Whether the screens mark it as a worry (amber).
    pub fn worry(self) -> bool {
        matches!(self, Why::OverAmped | Why::Tight | Why::Shrank | Why::Hollow | Why::Flat)
    }
}

/// How a talk lands on one player: his mind before and after, what it does
/// on the field, and why.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Landing {
    pub id: u32,
    pub before: Mind,
    pub after: Mind,
    /// What it is worth on the field for him, in rating points: the power
    /// and fine work it moves, weighed by how much of his job each is, and
    /// how much it settled him (the screens' and the head coach's read;
    /// coach fire: no longer a lift on every rating).
    pub lift: f32,
    /// Rating points of power and effort (Strength, Lower Body, Pursuit,
    /// Motor, his hit, a little burst): fire toward his own pitch.
    #[serde(default)]
    pub power: f32,
    /// Rating points of fine and thinking work (the accuracies, Touch,
    /// Decisions, Play Recognition, Anticipation, hands, Discipline,
    /// Concentration): fire past his pitch costs them, cooling him back
    /// toward it gives them back.
    #[serde(default)]
    pub fine: f32,
    /// Tendency points on the fire habits (`Tend::fiery`).
    #[serde(default)]
    pub habits: f32,
    /// His own best pitch of fire (`spot`).
    #[serde(default)]
    pub spot: f32,
    /// Discipline and Concentration, in 1-20 trait points.
    pub focus: f32,
    /// Composure, in 1-20 trait points: how hard a mistake bites.
    pub poise: f32,
    /// More (or fewer) late hits: a share added to his temper's.
    pub heat: f32,
    /// Snaps for the effect to halve.
    pub half_life: f32,
    /// How much of it reached him, 0-1.
    pub ear: f32,
    pub why: Why,
}

/// What a talk does on the field to one player through the game: the
/// engine's running copy of his landing, fading snap by snap.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pulse {
    /// Rating points of power and effort, and of fine and thinking work.
    #[serde(default)]
    pub power: f64,
    #[serde(default)]
    pub fine: f64,
    /// Tendency points on the fire habits.
    #[serde(default)]
    pub habits: f64,
    pub focus: f64,
    pub poise: f64,
    pub heat: f64,
    /// His fire before the talk, and what the talks added to it (fading):
    /// how much of a teammate's spark reaches him (coach fire, item 5).
    #[serde(default)]
    pub base: f64,
    #[serde(default)]
    pub fire: f64,
    /// Contact tendency points a big hit took from him: he hears
    /// footsteps for a stretch (coach fire, item 5). It fades at its own
    /// pace (`FLINCH_KEEP`), and a talk doesn't take it away.
    #[serde(default)]
    pub flinch: f64,
    /// His own best pitch of fire (`spot`).
    #[serde(default)]
    pub spot: f64,
    /// The share kept each snap.
    pub keep: f64,
}

impl Pulse {
    pub fn of(l: &Landing) -> Pulse {
        Pulse {
            power: l.power as f64,
            fine: l.fine as f64,
            habits: l.habits as f64,
            focus: l.focus as f64,
            poise: l.poise as f64,
            heat: l.heat as f64,
            base: l.before.fire as f64,
            fire: (l.after.fire - l.before.fire) as f64,
            flinch: 0.0,
            spot: l.spot as f64,
            keep: 0.5f64.powf(1.0 / (l.half_life as f64).max(4.0)),
        }
    }
    pub fn fade(&mut self) {
        self.power *= self.keep;
        self.fine *= self.keep;
        self.habits *= self.keep;
        self.focus *= self.keep;
        self.poise *= self.keep;
        self.heat *= self.keep;
        self.fire *= self.keep;
        self.flinch *= FLINCH_KEEP;
    }
    /// A word on the sideline adds to what is left of the talks.
    pub fn add(&mut self, l: &Landing) {
        let p = Pulse::of(l);
        self.power = (self.power + p.power).clamp(-FIRE_CAP, FIRE_CAP);
        self.fine = (self.fine + p.fine).clamp(-FIRE_CAP, FIRE_CAP);
        self.habits = (self.habits + p.habits).clamp(-FIRE_CAP, FIRE_CAP);
        self.focus = (self.focus + p.focus).clamp(-2.5, 2.5);
        self.poise = (self.poise + p.poise).clamp(-3.0, 3.0);
        self.heat = (self.heat + p.heat).clamp(-0.5, 1.2);
        self.fire = (self.base + self.fire + p.fire).clamp(0.02, 0.99) - self.base;
        if self.spot == 0.0 {
            self.spot = p.spot;
        }
        self.keep = self.keep.max(p.keep);
    }
    /// Trait points on Discipline and Concentration: the talk's focus, and
    /// the fine work fire costs read on their 1-20 scale.
    pub fn focus20(&self) -> f64 {
        self.focus + self.fine / TRAIT_STEP
    }
    /// His fire now, 0-1.
    pub fn fire_now(&self) -> f64 {
        (self.base + self.fire).clamp(0.0, 1.0)
    }
    /// How much of a teammate's spark reaches him (coach fire, item 5): all
    /// of it at his own pitch or past it, less the cooler he is.
    pub fn catches(&self) -> f64 {
        if self.spot <= 0.0 { 0.0 } else { (self.fire_now() / self.spot).clamp(0.0, 1.0) }
    }
}

/// Tendency points of fire habits at which a play his habit goes for is
/// tagged as fire's ("he came downhill hot"; coach fire, item 8).
pub const HOT: f64 = 1.0;

/// Rating points a spark gives his teammates on the field, at most
/// (coach fire, item 5: up to 0.4, fading with the momentum edge over a
/// few snaps). Proposed, not confirmed.
pub const SPARK: f64 = 0.4;

/// Contact tendency points a big hit takes from the man hit, for the
/// softest and most rattled man (coach fire, item 5); one tough and
/// composed (15 on both, on the level's scale) barely flinches. Proposed,
/// not confirmed.
pub const FLINCH: f64 = 3.0;

/// The force of a big hit at each level (coach fire, item 5): the top 3%
/// of the hardest hit on plays with one, measured on the engine's test
/// rosters (`fire_check hits`, 400 games a level, seed 1, 2026-11-17: the
/// 97th percentile NFL 2,118, FBS 1,806, FCS 1,679, high school 1,356).
/// Proposed, not confirmed.
pub fn big_hit(level: Level) -> f64 {
    match level {
        Level::NFL => BIG_HIT[0],
        Level::FBS => BIG_HIT[1],
        Level::FCS | Level::SmallCollege => BIG_HIT[2],
        _ => BIG_HIT[3],
    }
}
/// NFL, FBS, the other college levels, high school.
pub const BIG_HIT: [f64; 4] = [2120.0, 1810.0, 1680.0, 1360.0];

/// The share of a flinch kept each snap: about half gone after 14 snaps of
/// the game, the rest of his drive and into the next (coach fire, item 5:
/// "for a stretch"). Proposed, not confirmed.
pub const FLINCH_KEEP: f64 = 0.95;

/// When the talk is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Moment {
    Pregame,
    /// Halftime, with the score from this team's side.
    Halftime {
        margin: i32,
    },
    /// A word to one player after a mistake.
    Sideline {
        mistake: Mistake,
    },
}

/// The mistakes a sideline word follows (Cain 2026-10-27: costly ones, or a
/// pattern of three small ones in a game).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Mistake {
    Interception,
    Fumble,
    /// A foul that kept their drive alive.
    ExtendedDrive,
    /// A foul on a drive that died with it.
    KilledDrive,
    /// A blown assignment on their score.
    BlownScore,
    /// Three small ones today (drops, missed tackles, small fouls, busts).
    Pattern,
}

impl Mistake {
    pub fn words(self) -> &'static str {
        match self {
            Mistake::Interception => "threw an interception",
            Mistake::Fumble => "lost a fumble",
            Mistake::ExtendedDrive => "a flag that kept their drive alive",
            Mistake::KilledDrive => "a flag that killed our drive",
            Mistake::BlownScore => "a blown assignment on their score",
            Mistake::Pattern => "his third small mistake today",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Mistake::Interception => "interception",
            Mistake::Fumble => "fumble",
            Mistake::ExtendedDrive => "extended",
            Mistake::KilledDrive => "killed",
            Mistake::BlownScore => "blown",
            Mistake::Pattern => "pattern",
        }
    }
}

/// Who is talking: the head coach's motivation (1-20) and his own belief
/// lever (`Staff::motivation`), or a captain's voice.
#[derive(Clone, Copy, Debug)]
pub struct Speaker {
    /// Motivation, 1-20: how well his words carry and how well he reads
    /// the room.
    pub motivation: f64,
}

/// Everything a landing needs about the moment.
#[derive(Clone, Copy, Debug)]
pub struct Ctx<'a> {
    pub level: Level,
    pub moment: Moment,
    pub stakes: f64,
    pub day: &'a Day,
    pub speaker: Speaker,
    /// The room's week of practice, -1 (loose) to 1 (sharp): its prep
    /// words (`practice`).
    pub practice: f64,
}

/// One player's week of practice, -1 (unprepared) to 1 (sharp), from his
/// game-day prep factor (training TR38: 0.93 sharp, 1.0 prepared, 1.2
/// underprepared).
pub fn week_of(p: &GamePlayer) -> f64 {
    ((1.0 - p.prep as f64) / 0.07).clamp(-1.5, 1.0)
}

/// The room's week of practice: its players' weeks, the men who will play
/// first.
pub fn practice(team: &GameTeam) -> f64 {
    let w = weights(team);
    let n: f64 = w.iter().sum();
    if n <= 0.0 {
        return 0.0;
    }
    (team.players.iter().zip(&w).map(|(p, k)| k * week_of(p)).sum::<f64>() / n).clamp(-1.0, 1.0)
}

fn z(t: u8) -> f64 {
    (t as f64 - 10.0) / 10.0
}

/// The pitch each position plays best at.
pub fn ideal_fire(p: Position) -> f64 {
    match p {
        Position::OT | Position::IOL => 0.62,
        Position::EDGE | Position::DT => 0.66,
        Position::LB => 0.63,
        Position::RB | Position::TE => 0.58,
        Position::S => 0.56,
        Position::WR | Position::CB => 0.52,
        Position::QB => 0.42,
        Position::K | Position::P | Position::LS => 0.34,
    }
}

/// His own best pitch (coach fire, item 1): his position's, moved by his
/// makeup. A hot temper, a fierce competitor and a hitter play best hotter
/// (up to 0.08 in all); a calm temper, a man who rattles, and a pocket
/// passer against a gunslinger or a scrambler play best cooler (down to
/// 0.06 in all). A Branch-type safety (temper 3, competitiveness 17,
/// Physicality 17 on the level's scale) sits at about 0.62 against the
/// safeties' 0.56; a calm pocket passer about 0.38 against 0.42. Real:
/// each athlete has his own zone of best arousal (Hanin's individual zones
/// of optimal functioning). Proposed, not confirmed: every size.
pub fn spot(p: &GamePlayer, level: Level) -> f64 {
    let hot = ((10.0 - p.traits.temperament) / 10.0).clamp(-0.9, 0.9);
    let comp = ((p.personality.competitiveness - 10.0) / 10.0).clamp(-1.0, 1.0);
    let phys = (p.trait20(Attr::Physicality, level) - 10.0) / 10.0;
    let rattles = (10.0 - p.composure(level)) / 10.0;
    let mut up = 0.035 * hot.max(0.0) + 0.025 * comp.max(0.0) + 0.03 * phys.max(0.0);
    let mut down = 0.035 * (-hot).max(0.0) + 0.03 * rattles.max(0.0);
    if p.position == Position::QB {
        // A pocket passer's job is fine work snap after snap; a
        // gunslinger's and a scrambler's less so.
        let lean = |t: Tend| (p.tend.get(t) - crate::tendencies::MID) / 9.5;
        let (sling, scram) = (lean(Tend::Gunslinger), lean(Tend::Scramble));
        up += 0.02 * sling.max(0.0) + 0.01 * scram.max(0.0);
        down += 0.02 * (-sling).max(0.0) + 0.015 * (-scram).max(0.0);
    }
    ideal_fire(p.position) + up.min(0.08) - down.min(0.06)
}

/// Rating points of power and effort a unit of fire toward his pitch
/// gives, and of fine and thinking work a unit of fire past it takes;
/// tendency points a unit of fire moves the fire habits under his pitch
/// and past it. Each effect at most 3 points either way (Cain's answer 2,
/// 2026-11-17). Proposed, not confirmed: the slopes (a strong talk moves a
/// man's fire about 0.15, so it gives about 1.8 points).
pub const POWER_PER_FIRE: f64 = 12.0;
pub const FINE_PER_FIRE: f64 = 12.0;
pub const HABITS_UNDER: f64 = 4.0;
pub const HABITS_OVER: f64 = 12.0;
pub const FIRE_CAP: f64 = 3.0;

/// The power and effort ratings fire lifts toward his pitch (Oxendine,
/// "Emotional Arousal and Motor Performance", 1970: gross strength, speed
/// and effort tasks gain from arousal). Motor, his hit and a little burst
/// are read where they act (`sim::Agent::fire_up`, `Sim::trait20`,
/// `hit_effects`).
pub const POWER_WORK: [Attr; 3] = [Attr::Strength, Attr::LowerBody, Attr::Pursuit];

/// The fine and thinking ratings fire costs past his pitch (the same
/// table: fine, complex and decision tasks lose from too much arousal).
/// Discipline and Concentration are read on their 1-20 scale
/// (`Pulse::focus20`).
pub const FINE_WORK: [Attr; 9] = [
    Attr::ShortAccuracy,
    Attr::MediumAccuracy,
    Attr::DeepAccuracy,
    Attr::ThrowOnMove,
    Attr::Touch,
    Attr::Decisions,
    Attr::PlayRecognition,
    Attr::Anticipation,
    Attr::Catching,
];

/// How much of his job is power and effort work and how much is fine and
/// thinking work: his position's own weights on each (`positions::weight`).
pub fn work_shares(pos: Position) -> (f64, f64) {
    let w = |a: Attr| crate::positions::weight(pos, a);
    let power = POWER_WORK.iter().map(|&a| w(a)).sum::<f64>() + w(Attr::Motor);
    let fine = FINE_WORK.iter().map(|&a| w(a)).sum::<f64>() + w(Attr::Discipline) + w(Attr::Concentration);
    (power, fine)
}

/// What fire does on the field between two pitches of it (coach fire,
/// items 2 and 3): rating points of power and effort, of fine and thinking
/// work, and tendency points on the fire habits, from `f0` to `f1` about
/// his own pitch `spot`. Under his pitch more fire is more effort and
/// nothing is lost in thinking; past it the effort holds and the fine work
/// goes. The habits climb with fire, three times as fast past his pitch.
pub fn fire_work(spot: f64, f0: f64, f1: f64) -> (f64, f64, f64) {
    let under = |f: f64| (spot - f).max(0.0);
    let over = |f: f64| (f - spot).max(0.0);
    let power = (POWER_PER_FIRE * (under(f0) - under(f1))).clamp(-FIRE_CAP, FIRE_CAP);
    let fine = (-FINE_PER_FIRE * (over(f1) - over(f0))).clamp(-FIRE_CAP, FIRE_CAP);
    let habits =
        (HABITS_UNDER * (f1.min(spot) - f0.min(spot)) + HABITS_OVER * (over(f1) - over(f0))).clamp(-FIRE_CAP, FIRE_CAP);
    (power, fine, habits)
}

/// The fire a room takes from the other coach's words in the paper this
/// week: under a rivalry week's 0.05 (coach voice). Proposed, not
/// confirmed.
pub const PROVOKED_FIRE: f64 = 0.03;

/// His mind going in: the stakes, his temper, his years and his week.
pub fn mind_now(p: &GamePlayer, cx: &Ctx) -> Mind {
    let t = &p.personality.traits;
    let hot = ((10.0 - p.traits.temperament) / 10.0).clamp(-0.9, 0.9);
    let comp = (p.personality.competitiveness - 10.0) / 10.0;
    let nerves = (1.0 - p.experience / 3.0).clamp(0.0, 1.0);
    let s = cx.stakes;
    let mood = p.mood as f64;
    let poise20 = p.composure(cx.level);
    let conc20 = p.trait20(Attr::Concentration, cx.level);
    let mut fire = 0.30 + 0.36 * s + 0.10 * hot + 0.06 * comp + 0.10 * nerves * s + 0.04 * mood;
    let mut poise = 0.52 + 0.42 * (poise20 - 10.0) / 10.0 - 0.22 * s * nerves + 0.06 * mood;
    let mut focus = 0.52
        + 0.30 * (conc20 - 10.0) / 10.0
        + 0.12 * z(t.professionalism)
        + 0.08 * mood.min(0.0) * 2.0
        + 0.06 * week_of(p);
    if cx.day.rival {
        fire += 0.05;
    }
    // Their coach's words on the locker room wall (coach voice): a small
    // lift, never past a rivalry week's. Proposed, not confirmed.
    if cx.day.provoked {
        fire += PROVOKED_FIRE;
    }
    match cx.moment {
        Moment::Pregame => {}
        Moment::Halftime { margin } => {
            let m = margin as f64;
            if m <= -14.0 {
                // Down two scores or more: the air goes out.
                fire -= 0.10 + 0.04 * (1.0 - comp.max(0.0));
                poise -= 0.10;
            } else if m >= 17.0 {
                // Up big: comfortable, a step slow to start the half.
                fire -= 0.08;
                focus -= 0.08;
            } else if m.abs() <= 7.0 {
                fire += 0.05;
            }
        }
        Moment::Sideline { mistake } => {
            // Just after the mistake: the hot ones angry, the fragile rattled.
            let big = !matches!(mistake, Mistake::Pattern);
            fire += 0.12 * hot.max(0.0) + if big { 0.04 } else { 0.0 };
            poise -= (0.10 + if big { 0.06 } else { 0.0 }) * (1.0 - 0.6 * (poise20 - 10.0).max(0.0) / 10.0);
            focus -= 0.04;
        }
    }
    Mind {
        fire: fire.clamp(0.04, 0.97) as f32,
        poise: poise.clamp(0.04, 0.97) as f32,
        focus: focus.clamp(0.04, 0.97) as f32,
    }
}

/// How far a captain's voice carries to one listener, 0-1: his leadership,
/// his years, his standing, and whether the listener is on his side of the
/// ball (Cain's mock-up: "how far a voice carries comes from leadership,
/// years in the league, how he plays and how he treats people").
pub fn reach(cap: &GamePlayer, listener: &GamePlayer) -> f64 {
    let lead = (cap.personality.traits.leadership as f64 - 4.0) / 14.0;
    let years = (cap.experience / 6.0).min(1.0);
    let same_side = side_of(cap.position) == side_of(listener.position);
    let r = 0.25 + 0.5 * lead.clamp(0.0, 1.0) + 0.2 * years;
    (if same_side || cap.position == Position::QB || lead > 0.85 { r } else { 0.55 * r }).clamp(0.05, 1.0)
}

fn side_of(p: Position) -> u8 {
    match p {
        Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL => 0,
        Position::EDGE | Position::DT | Position::LB | Position::CB | Position::S => 1,
        _ => 2,
    }
}

/// How a talk lands on one player.
pub fn land(p: &GamePlayer, pick: &Pick, cx: &Ctx, captain: Option<&GamePlayer>) -> Landing {
    let before = mind_now(p, cx);
    let t = &p.personality.traits;
    let hot = ((10.0 - p.traits.temperament) / 10.0).clamp(-0.9, 0.9);
    let comp = (p.personality.competitiveness - 10.0) / 10.0;
    let ego = z(t.ego);
    let prof = z(t.professionalism);
    let lead = z(t.leadership);
    let nerves = (1.0 - p.experience / 3.0).clamp(0.0, 1.0);
    let vet = (p.experience / 4.0).min(1.0);
    let poise20 = p.composure(cx.level);
    let fragile = ((10.0 - poise20) / 10.0 + 0.5 * nerves).clamp(0.0, 1.0);
    let mut why = Why::Unmoved;
    let me = pick.subject == Subject::One && pick.who == p.id;
    // How much of it reaches him: his belief in the man talking, how well
    // that man talks to a room, how many speeches he has heard and how worn
    // the tone is. A word to him alone reaches him most.
    let worn = cx.day.worn[pick.tone.index()] as f64;
    let coach = (cx.speaker.motivation - 10.0) / 10.0;
    let heard = 1.0 - (0.035 * p.experience).min(0.35);
    let mut ear = (0.45 + 0.75 * p.belief as f64)
        * (0.78 + 0.22 * coach)
        * heard
        * (1.0 - 0.45 * worn)
        * (0.85 + 0.15 * (p.personality.coachability - 10.0) / 10.0);
    if matches!(cx.moment, Moment::Sideline { .. }) || me {
        ear *= 1.35;
    }
    let (mut df, mut dp, mut dc) = (0.0f64, 0.0f64, 0.0f64);
    let mut heat_kick = 0.0f64;
    match pick.mv {
        Move::Straight => {
            dp += 0.04;
            dc += 0.07 + 0.03 * prof;
            df += 0.02;
        }
        Move::Ask => {
            dp += 0.03 + 0.03 * lead;
            dc += 0.06;
            dp -= 0.02 * nerves * (1.0 - lead.max(0.0));
        }
        Move::Praise => {
            df += 0.04;
            dp += 0.04 + 0.06 * (1.0 - (ego + 1.0) / 2.0) + 0.03 * fragile;
            // Unearned praise after a loss rings hollow, the pros first.
            if cx.day.last < 0 && matches!(cx.moment, Moment::Pregame) {
                dp -= 0.07;
                dc -= 0.03 + 0.03 * prof.max(0.0);
            }
            if matches!(cx.moment, Moment::Halftime { margin } if margin <= -14) {
                dp -= 0.05;
            }
        }
        Move::Challenge => {
            df += 0.10 + 0.06 * comp + 0.04 * ego.max(0.0);
            dp += 0.05 * comp - 0.09 * fragile;
            dc += 0.02;
            heat_kick += 0.3 * hot.max(0.0);
        }
        Move::Criticize => {
            df += 0.06;
            dp += -0.05 - 0.05 * ego.max(0.0) - 0.07 * fragile + 0.04 * comp;
            dc += 0.03 + 0.06 * prof;
        }
        Move::Promise => {
            df += 0.05;
            dp += 0.03;
        }
        Move::Warn => {
            df -= 0.03;
            dp -= 0.03 * fragile;
            dc += 0.08 + 0.06 * hot.max(0.0);
            heat_kick -= 0.4 * hot.max(0.0);
        }
        Move::Listen => {
            df -= 0.04;
            dp += 0.02 + 0.05 * vet;
            dc += 0.02;
        }
        Move::Captain => {
            // The room hears its own: his reach replaces your ear, and the
            // more often you hand it over the less it is his moment.
            if let Some(c) = captain {
                let r = if c.id == p.id { 0.9 } else { reach(c, p) };
                let fresh = 1.0 - 0.22 * cx.day.handed.min(4) as f64;
                ear = r * fresh * (0.8 + 0.2 * heard);
                df += 0.06;
                dp += 0.08;
                dc += 0.05;
                if r > 0.6 {
                    why = Why::Follows;
                }
            }
        }
        Move::Nothing => {
            ear = 0.0;
        }
    }
    match pick.tone {
        Tone::Fiery => {
            df += 0.09 * (1.0 + 0.6 * hot.max(0.0));
            dp += 0.03 * comp - 0.04 * nerves;
            dc -= 0.04 * hot.max(0.0);
            heat_kick += 0.25 * hot.max(0.0);
        }
        Tone::Warm => {
            df -= 0.02;
            dp += 0.04 + 0.04 * fragile - 0.02 * ego.max(0.0) * vet;
        }
        Tone::Firm => {
            df += 0.02;
            dp += 0.02;
            dc += 0.04 + 0.03 * prof;
        }
        Tone::Calm => {
            df -= 0.08;
            dp += 0.03 + 0.04 * nerves;
            dc += 0.03;
        }
    }
    let unit = Unit::of(p.position).index() as u8;
    match pick.subject {
        Subject::Stakes => {
            df += 0.04 + 0.05 * cx.stakes;
            dp -= 0.04 * nerves * cx.stakes;
        }
        Subject::Us => {
            dp += 0.03;
            dc += 0.02;
        }
        Subject::Them => {
            df += 0.05 + if cx.day.rival { 0.03 } else { 0.0 };
            if cx.day.quote.is_some_and(|q| q.1 == unit) {
                // What they said in the paper was about him and his room.
                df += 0.06;
                dp += 0.05 * comp - 0.06 * fragile;
                heat_kick += 0.3 * hot.max(0.0);
                if comp > 0.3 && hot > 0.1 {
                    why = Why::Personal;
                }
            }
        }
        Subject::Seniors => {
            if p.experience >= 3.0 {
                df += 0.05;
                dp += 0.04;
            } else {
                df += 0.02;
            }
        }
        Subject::Plan => {
            dc += 0.06;
            df -= 0.02;
            dp += 0.02 + 0.04 * cx.practice;
        }
        Subject::One => {
            if !me {
                // Talking about one man to the room: a hurt teammate rallies
                // it; the rest hear it from the side.
                if cx.day.hurt.is_some_and(|h| h == pick.who) {
                    df += 0.04;
                    dp += 0.01;
                }
                ear *= 0.4;
            }
        }
    }
    if me {
        df *= 1.6;
        dp *= 1.8;
        dc *= 1.5;
    }
    let ear = ear.clamp(0.0, 1.4);
    let after = Mind {
        fire: (before.fire as f64 + ear * df).clamp(0.02, 0.99) as f32,
        poise: (before.poise as f64 + ear * dp).clamp(0.02, 0.99) as f32,
        focus: (before.focus as f64 + ear * dc).clamp(0.02, 0.99) as f32,
    };
    // On the field (coach fire): his fire against his own pitch by the
    // kind of work, his composure and his focus, each against how he came
    // in. What it is worth to him weighs the power and fine work by how
    // much of his job each is, and his settling as the flat lift did.
    let spot = spot(p, cx.level);
    let over = |f: f32| (f as f64 - spot).max(0.0);
    let (f0, f1) = (before.fire, after.fire);
    let (power, fine, habits) = fire_work(spot, f0 as f64, f1 as f64);
    let focus = (6.0 * (after.focus - before.focus) as f64).clamp(-2.0, 2.0);
    let poise = (8.0 * (after.poise - before.poise) as f64).clamp(-2.5, 2.5);
    let (ps, fs) = work_shares(p.position);
    let net = if ps + fs > 0.0 { (ps * power + fs * fine) / (ps + fs) } else { 0.0 };
    let lift = (net + 0.25 * poise).clamp(-FIRE_CAP, FIRE_CAP);
    let hot01 = hot.max(0.0);
    let heat = (2.5 * ((f1 as f64 - 0.72).max(0.0) - (f0 as f64 - 0.72).max(0.0)) * hot01 + ear * heat_kick * 0.4)
        .clamp(-0.5, 1.0);
    let half_life = match cx.moment {
        Moment::Sideline { .. } => 10.0 + 10.0 * vet,
        _ => 16.0 + 20.0 * vet,
    };
    if why == Why::Unmoved {
        let d_fire = (f1 - f0) as f64;
        let d_poise = (after.poise - before.poise) as f64;
        let d_focus = (after.focus - before.focus) as f64;
        why = if ear < 0.25 || (d_fire.abs() < 0.02 && d_poise.abs() < 0.02 && d_focus.abs() < 0.02) {
            Why::Unmoved
        } else if pick.mv == Move::Praise && d_poise < -0.02 {
            Why::Hollow
        } else if over(f1) > 0.08 && over(f1) > over(f0) + 0.02 {
            // Past his pitch by enough to cost a point of fine work.
            Why::OverAmped
        } else if d_poise < -0.04 && fragile > 0.4 && ego < 0.0 {
            Why::Shrank
        } else if d_poise < -0.03 {
            Why::Tight
        } else if f1 as f64 + 0.12 < spot && d_fire < -0.03 {
            Why::Flat
        } else if d_focus > 0.04 && d_focus >= d_fire.abs() {
            Why::Locked
        } else if d_fire > 0.04 {
            Why::FiredUp
        } else if d_poise > 0.03 || d_fire < -0.03 {
            Why::Settled
        } else {
            Why::Locked
        }
    }
    Landing {
        id: p.id,
        before,
        after,
        lift: lift as f32,
        power: power as f32,
        fine: fine as f32,
        habits: habits as f32,
        spot: spot as f32,
        focus: focus as f32,
        poise: poise as f32,
        heat: heat as f32,
        half_life: half_life as f32,
        ear: ear as f32,
        why,
    }
}

/// The game's stakes for a team, 0-1: the world's own read, else the
/// playoffs and the season's push.
pub fn stakes(day: &Day, playoff: bool, ctx: &super::garbage::Context) -> f64 {
    if day.stakes > 0.0 {
        return (day.stakes as f64).clamp(0.0, 1.0);
    }
    let mut s: f64 = if playoff { 0.8 } else { 0.25 };
    if ctx.race {
        s += 0.2;
    }
    if ctx.grudge {
        s += 0.15;
    }
    if ctx.hot_seat {
        s += 0.08;
    }
    if ctx.bowl_push {
        s += 0.12;
    }
    if day.rival {
        s += 0.25;
    }
    s.clamp(0.0, 1.0)
}

/// How much each player's mind counts toward a team's day: the starters
/// at each position first (the screens and the head coach's read weigh the
/// men who will play).
pub fn weights(team: &GameTeam) -> Vec<f64> {
    let starters = |p: Position| match p {
        Position::QB | Position::RB | Position::TE | Position::K | Position::P | Position::LS => 1,
        Position::WR | Position::LB => 3,
        Position::OT | Position::CB | Position::S | Position::EDGE | Position::DT => 2,
        Position::IOL => 3,
    };
    let mut w = vec![0.3; team.players.len()];
    for pos in Position::ALL {
        let mut at: Vec<usize> = (0..team.players.len()).filter(|&i| team.players[i].position == pos).collect();
        at.sort_by(|&a, &b| {
            let ra = team.players[a].spot_r[team.players[a].spot.index()];
            let rb = team.players[b].spot_r[team.players[b].spot.index()];
            rb.total_cmp(&ra)
        });
        for &i in at.iter().take(starters(pos)) {
            w[i] = 1.0;
        }
    }
    w
}

/// The room's natural captains: the day's named ones, else the two who
/// lead most (leadership, then years).
pub fn captains(team: &GameTeam, day: &Day) -> Vec<usize> {
    let named: Vec<usize> =
        day.captains.iter().filter_map(|id| team.players.iter().position(|p| p.id == *id)).collect();
    if !named.is_empty() {
        return named;
    }
    let mut v: Vec<usize> = (0..team.players.len()).collect();
    v.sort_by(|&a, &b| {
        let s = |i: usize| {
            let p = &team.players[i];
            p.personality.traits.leadership as f64 + 0.6 * p.experience.min(5.0)
        };
        s(b).total_cmp(&s(a))
    });
    v.truncate(2);
    v
}

/// A talk to the whole room: every player's landing.
pub fn room(team: &GameTeam, pick: &Pick, cx: &Ctx) -> Vec<Landing> {
    let cap = if pick.mv == Move::Captain {
        team.players
            .iter()
            .find(|p| p.id == pick.who)
            .or_else(|| captains(team, cx.day).first().map(|&i| &team.players[i]))
    } else {
        None
    };
    team.players.iter().map(|p| land(p, pick, cx, cap)).collect()
}

/// A typed speech's own word to one man (live talks v3, lane N5d): the
/// word to him alone that his part of your speech was, in the talk's tone,
/// and how much of his landing it takes, 0-1. His landing lies between the
/// room's and that word's, so no typed line moves a man past what a chip
/// could (lane 9's rule: wording alone never beats the best chip).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tilt {
    pub id: u32,
    pub pick: Pick,
    pub share: f32,
}

/// One landing taken `s` of the way from `a` to `b` (0 keeps `a`).
pub fn mix(a: &Landing, b: &Landing, s: f32) -> Landing {
    let s = s.clamp(0.0, 1.0);
    let l = |x: f32, y: f32| x + (y - x) * s;
    let m =
        |x: &Mind, y: &Mind| Mind { fire: l(x.fire, y.fire), poise: l(x.poise, y.poise), focus: l(x.focus, y.focus) };
    Landing {
        id: a.id,
        before: a.before,
        after: m(&a.after, &b.after),
        lift: l(a.lift, b.lift),
        power: l(a.power, b.power),
        fine: l(a.fine, b.fine),
        habits: l(a.habits, b.habits),
        spot: a.spot,
        focus: l(a.focus, b.focus),
        poise: l(a.poise, b.poise),
        heat: l(a.heat, b.heat),
        half_life: l(a.half_life, b.half_life),
        ear: l(a.ear, b.ear),
        why: if s >= 0.5 { b.why } else { a.why },
    }
}

/// A talk to the whole room with each man's own part of it (a typed
/// speech): every landing, tilted toward the word to him alone by its
/// share. No tilts: the same as `room`.
pub fn room_tilted(team: &GameTeam, pick: &Pick, cx: &Ctx, tilts: &[Tilt]) -> Vec<Landing> {
    let mut ls = room(team, pick, cx);
    for t in tilts {
        let Some(i) = team.players.iter().position(|p| p.id == t.id) else { continue };
        let own = Pick { subject: Subject::One, who: t.id, ..t.pick };
        let b = land(&team.players[i], &own, cx, None);
        ls[i] = mix(&ls[i], &b, t.share);
    }
    ls
}

/// A word on the sideline with only `share` of it reaching him (a typed
/// word that touched less than his heaviest thing): between saying
/// nothing and the word itself.
pub fn land_share(p: &GamePlayer, pick: &Pick, cx: &Ctx, share: f32) -> Landing {
    let full = land(p, pick, cx, None);
    let none = land(p, &Pick { mv: Move::Nothing, ..*pick }, cx, None);
    mix(&none, &full, share)
}

/// What a room's landings are worth on the field, weighted to the men who
/// will play: the head coach's read of a talk.
pub fn worth(ls: &[Landing], w: &[f64]) -> f64 {
    ls.iter()
        .zip(w)
        .map(|(l, &k)| {
            k * (l.lift as f64 + 0.35 * l.focus as f64 + 0.15 * l.poise as f64 - 0.8 * l.heat.max(0.0) as f64)
        })
        .sum::<f64>()
        / w.iter().sum::<f64>().max(1.0)
}

/// The talks a head coach weighs before a game or at halftime.
pub fn options(cx: &Ctx, team: &GameTeam) -> Vec<Pick> {
    use Move as M;
    use Subject as S;
    use Tone as T;
    let mut v = vec![
        Pick::new(M::Straight, T::Firm, S::Plan),
        Pick::new(M::Straight, T::Calm, S::Us),
        Pick::new(M::Challenge, T::Fiery, S::Them),
        Pick::new(M::Challenge, T::Firm, S::Stakes),
        Pick::new(M::Praise, T::Warm, S::Us),
        Pick::new(M::Warn, T::Firm, S::Plan),
        Pick::new(M::Straight, T::Fiery, S::Stakes),
        Pick::new(M::Listen, T::Calm, S::Us),
        Pick::new(M::Ask, T::Calm, S::Plan),
    ];
    if cx.day.last < 0 || matches!(cx.moment, Moment::Halftime { margin } if margin < 0) {
        v.push(Pick::new(M::Criticize, T::Firm, S::Us));
        v.push(Pick::new(M::Criticize, T::Fiery, S::Us));
    }
    if cx.day.seniors {
        v.push(Pick::new(M::Praise, T::Warm, S::Seniors));
    }
    if let Some(h) = cx.day.hurt {
        v.push(Pick { mv: M::Challenge, tone: T::Fiery, subject: S::One, who: h });
    }
    if let Some(&c) = captains(team, cx.day).first() {
        v.push(Pick { mv: M::Captain, tone: T::Calm, subject: S::Us, who: team.players[c].id });
    }
    v
}

/// A head coach's pick: he reads his room and picks the talk he thinks
/// works best. A great motivator reads it nearly right; a poor one is off
/// by more, and his temper leans him to the fiery talk.
pub fn coach_pick(team: &GameTeam, cx: &Ctx, seed: u64) -> Pick {
    let w = weights(team);
    let read_sd = 0.12 * (1.7 - 0.07 * cx.speaker.motivation).max(0.25);
    let fiery_lean = 0.02 * (team.staff.temper - 10.0) / 10.0;
    let mut best = (f64::MIN, Pick::default());
    for (k, pick) in options(cx, team).into_iter().enumerate() {
        let ls = room(team, &pick, cx);
        let mut score = worth(&ls, &w) + read_sd * normal(seed, k as u64);
        if pick.tone == Tone::Fiery {
            score += fiery_lean;
        }
        if score > best.0 {
            best = (score, pick);
        }
    }
    best.1
}

/// The words a coach picks for one player after his mistake.
pub fn word_options(id: u32) -> Vec<Pick> {
    use Move as M;
    use Subject as S;
    use Tone as T;
    let one = |mv: M, tone: T| Pick { mv, tone, subject: S::One, who: id };
    vec![
        one(M::Straight, T::Calm),
        one(M::Straight, T::Firm),
        one(M::Praise, T::Warm),
        one(M::Challenge, T::Fiery),
        one(M::Criticize, T::Fiery),
        one(M::Criticize, T::Firm),
        one(M::Warn, T::Firm),
        one(M::Listen, T::Calm),
    ]
}

/// A position coach's word to one player (the staff's, when you don't say
/// it yourself): read the same way, by the head coach's motivation.
pub fn coach_word(p: &GamePlayer, cx: &Ctx, seed: u64) -> Pick {
    let read_sd = 0.18 * (1.7 - 0.07 * cx.speaker.motivation).max(0.25);
    let mut best = (f64::MIN, Pick::default());
    for (k, pick) in word_options(p.id).into_iter().enumerate() {
        if cx.day.room.is_some_and(|r| r.tone != pick.tone) {
            continue;
        }
        let l = land(p, &pick, cx, None);
        let score = l.lift as f64 + 0.35 * l.focus as f64 + 0.25 * l.poise as f64 + read_sd * normal(seed, k as u64);
        if score > best.0 {
            best = (score, pick);
        }
    }
    best.1
}

/// What a word does to his trust in you (the world reads it after your
/// game): a word that settles him earns a little, one that rattles him or
/// shames a proud man costs more.
pub fn word_trust(l: &Landing, pick: &Pick) -> f32 {
    let good = l.lift + 0.3 * l.poise;
    let shamed = matches!(pick.mv, Move::Criticize) && pick.tone == Tone::Fiery;
    (0.02 * good.clamp(-1.5, 1.5) - if shamed { 0.015 } else { 0.0 }).clamp(-0.05, 0.04)
}

/// A seeded normal draw (splitmix and Box-Muller), so a coach's read never
/// takes from the game's own stream.
pub fn normal(seed: u64, k: u64) -> f64 {
    let u1 = uniform(seed, k.wrapping_mul(2).wrapping_add(1)).max(1e-12);
    let u2 = uniform(seed, k.wrapping_mul(2).wrapping_add(2));
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

pub fn uniform(seed: u64, k: u64) -> f64 {
    let mut x = seed ^ k.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// One unit's minds before and after, averaged over the men who will play.
pub fn by_unit(team: &GameTeam, ls: &[Landing], unit: Unit) -> Option<(Mind, Mind)> {
    unit_minds(team, ls, &weights(team), unit)
}

fn unit_minds(team: &GameTeam, ls: &[Landing], w: &[f64], unit: Unit) -> Option<(Mind, Mind)> {
    let mut sum = [0.0f64; 6];
    let mut n = 0.0;
    for (k, l) in ls.iter().enumerate() {
        if Unit::of(team.players[k].position) != unit {
            continue;
        }
        let x = w[k];
        sum[0] += x * l.before.fire as f64;
        sum[1] += x * l.before.poise as f64;
        sum[2] += x * l.before.focus as f64;
        sum[3] += x * l.after.fire as f64;
        sum[4] += x * l.after.poise as f64;
        sum[5] += x * l.after.focus as f64;
        n += x;
    }
    (n > 0.0).then(|| {
        let m = |a: f64| (a / n) as f32;
        (
            Mind { fire: m(sum[0]), poise: m(sum[1]), focus: m(sum[2]) },
            Mind { fire: m(sum[3]), poise: m(sum[4]), focus: m(sum[5]) },
        )
    })
}

/// What a sideline word finds: a player who has made a costly mistake, or a
/// third small one, waiting for a word when he next comes off.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WordDue {
    pub team: usize,
    pub id: u32,
    pub mistake: Mistake,
    /// The play it came on.
    pub play: u32,
    /// His unit: 0 offense, 1 defense.
    pub unit: u8,
}

/// The talks of one game, for the world and the game page: each side's
/// pregame and halftime pick with what it did to the room, and every
/// sideline word with what it did to the player's trust in you.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Record {
    pub pregame: [Option<Pick>; 2],
    pub halftime: [Option<Pick>; 2],
    pub words: Vec<WordNote>,
    pub rooms: Vec<RoomNote>,
}

/// A sideline word as given.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct WordNote {
    pub side: u8,
    pub id: u32,
    pub mistake: Mistake,
    pub pick: Pick,
    /// You gave it (not your staff).
    pub yours: bool,
    /// His trust in you, changed by it (your words only).
    pub trust: f32,
    /// The play the mistake came on.
    pub play: u32,
    pub why: Why,
    pub lift: f32,
}

/// What a talk did to one room, kept for the game page: each unit's minds
/// before and after, the players it moved most and why, and what it was
/// worth on the field at the start and over the next quarter.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RoomNote {
    pub side: u8,
    pub halftime: bool,
    /// (`Unit` index, before, after), the men who will play.
    pub units: Vec<(u8, Mind, Mind)>,
    /// The players it moved most: (player, why, what it is worth to him on
    /// the field in rating points).
    pub stood: Vec<(u32, Why, f32)>,
    /// What it is worth on the field at the start, in rating points, the
    /// men who play (each man's power and fine work by his job, and his
    /// settling).
    pub lift: f32,
    /// The same averaged over the next quarter's snaps (both sides', about
    /// 30), as it fades.
    pub quarter: f32,
    /// Composure points at the start, the men who play.
    pub poise: f32,
    /// Rating points of power and effort, and of fine and thinking work,
    /// at the start, the men who play (coach fire).
    pub power: f32,
    pub fine: f32,
    /// Starters it rattled, over-amped or left flat.
    pub worries: u16,
}

/// The moment's context for one team, the engine's and the screens' own.
pub fn ctx<'a>(
    team: &'a GameTeam,
    level: Level,
    playoff: bool,
    context: &super::garbage::Context,
    moment: Moment,
) -> Ctx<'a> {
    Ctx {
        level,
        moment,
        stakes: stakes(&team.day, playoff, context),
        day: &team.day,
        speaker: Speaker { motivation: team.staff.motivation },
        practice: practice(team),
    }
}

/// A room's note from its landings.
pub fn note(team: &GameTeam, ls: &[Landing], side: usize, halftime: bool) -> RoomNote {
    let w = weights(team);
    let units =
        Unit::ALL.iter().filter_map(|&u| unit_minds(team, ls, &w, u).map(|(a, b)| (u.index() as u8, a, b))).collect();
    let n: f64 = w.iter().sum::<f64>().max(1.0);
    let lift = ls.iter().zip(&w).map(|(l, k)| k * l.lift as f64).sum::<f64>() / n;
    let poise = ls.iter().zip(&w).map(|(l, k)| k * l.poise as f64).sum::<f64>() / n;
    let power = ls.iter().zip(&w).map(|(l, k)| k * l.power as f64).sum::<f64>() / n;
    let fine = ls.iter().zip(&w).map(|(l, k)| k * l.fine as f64).sum::<f64>() / n;
    let quarter = ls
        .iter()
        .zip(&w)
        .map(|(l, k)| {
            let keep = Pulse::of(l).keep;
            k * l.lift as f64 * (1.0 - keep.powi(30)) / (30.0 * (1.0 - keep).max(1e-6))
        })
        .sum::<f64>()
        / n;
    let mut moved: Vec<usize> = (0..ls.len()).filter(|&i| w[i] >= 1.0).collect();
    let size = |i: usize| (ls[i].lift + 0.3 * ls[i].poise).abs() + if ls[i].why.worry() { 0.15 } else { 0.0 };
    moved.sort_by(|&a, &b| size(b).total_cmp(&size(a)));
    let worries = (0..ls.len()).filter(|&i| w[i] >= 1.0 && ls[i].why.worry()).count() as u16;
    RoomNote {
        side: side as u8,
        halftime,
        units,
        stood: moved.iter().take(5).map(|&i| (ls[i].id, ls[i].why, ls[i].lift)).collect(),
        lift: lift as f32,
        quarter: quarter as f32,
        poise: poise as f32,
        power: power as f32,
        fine: fine as f32,
        worries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kicker_is_never_better_for_a_fiery_talk_than_a_guard() {
        assert!(ideal_fire(Position::K) < ideal_fire(Position::IOL));
        assert!(ideal_fire(Position::QB) < ideal_fire(Position::DT));
    }

    #[test]
    fn a_tilt_lies_between_the_room_and_the_word_to_him() {
        let a = Landing { lift: 1.0, focus: -2.0, heat: 0.1, ear: 0.5, ..Landing::default() };
        let b = Landing { lift: 3.0, focus: 2.0, heat: -0.1, ear: 1.0, ..Landing::default() };
        let at = |s| mix(&a, &b, s);
        assert_eq!((at(0.0).lift, at(0.0).focus), (a.lift, a.focus));
        assert_eq!((at(1.0).lift, at(1.0).focus), (b.lift, b.focus));
        assert_eq!(at(0.5).lift, 2.0);
        assert_eq!(at(7.0).lift, b.lift, "never past the word itself");
        assert_eq!(at(-1.0).lift, a.lift);
        let m = at(0.3);
        assert!(m.heat < a.heat && m.heat > b.heat && m.ear > a.ear && m.ear < b.ear);
    }

    #[test]
    fn fire_works_by_the_kind_of_work() {
        let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
        // Toward his pitch: effort, and no thinking lost.
        let (p, f, h) = fire_work(0.6, 0.4, 0.55);
        assert!(near(p, 12.0 * 0.15) && near(f, 0.0) && near(h, 4.0 * 0.15), "{p} {f} {h}");
        // Past it: the effort holds, the fine work goes, the habits climb.
        let (p, f, h) = fire_work(0.6, 0.55, 0.75);
        assert!(near(p, 12.0 * 0.05) && near(f, -12.0 * 0.15), "{p} {f}");
        assert!(near(h, 4.0 * 0.05 + 12.0 * 0.15), "{h}");
        // Cooled back to his pitch, his reads come back.
        let (_, f, h) = fire_work(0.6, 0.75, 0.6);
        assert!(near(f, 12.0 * 0.15) && h < 0.0);
        // Under his pitch he loses effort, not thinking.
        let (p, f, _) = fire_work(0.6, 0.5, 0.35);
        assert!(p < 0.0 && near(f, 0.0));
        // Each at most 3 points (Cain, 2026-11-17).
        let (p, f, h) = fire_work(0.6, 0.05, 0.99);
        assert!(near(p, FIRE_CAP) && near(f, -FIRE_CAP) && near(h, FIRE_CAP), "{p} {f} {h}");
    }

    #[test]
    fn a_hot_hitter_plays_best_hotter_and_a_calm_pocket_passer_cooler() {
        use crate::game::synthetic::synthetic_league;
        let level = Level::NFL;
        let t = &synthetic_league(level, 2, 7)[0];
        let mut s = t.players.iter().find(|p| p.position == Position::S).unwrap().clone();
        s.traits.temperament = 3.0;
        s.personality.competitiveness = 17.0;
        s.attrs.set(Attr::Physicality, crate::game::rules::trait_norm(level, Attr::Physicality) + 7.0 * TRAIT_STEP);
        s.attrs.set(Attr::Composure, crate::game::rules::trait_norm(level, Attr::Composure));
        let branch = spot(&s, level);
        assert!((0.60..0.65).contains(&branch), "a Branch-type safety: {branch}");
        let mut q = t.players.iter().find(|p| p.position == Position::QB).unwrap().clone();
        q.traits.temperament = 16.0;
        q.personality.competitiveness = 10.0;
        q.attrs.set(Attr::Physicality, crate::game::rules::trait_norm(level, Attr::Physicality));
        q.attrs.set(Attr::Composure, crate::game::rules::trait_norm(level, Attr::Composure));
        q.tend.set(Tend::Gunslinger, 6.0);
        q.tend.set(Tend::Scramble, 5.0);
        let pocket = spot(&q, level);
        assert!((0.36..0.40).contains(&pocket), "a calm pocket passer: {pocket}");
        // His makeup moves it at most +0.08 and -0.06.
        assert!(branch <= ideal_fire(Position::S) + 0.08 + 1e-9);
        assert!(pocket >= ideal_fire(Position::QB) - 0.06 - 1e-9);
    }

    #[test]
    fn a_pulse_fades_and_a_flinch_outlasts_it() {
        let l = Landing { power: 2.0, fine: -1.0, habits: 1.5, spot: 0.6, half_life: 10.0, ..Landing::default() };
        let mut p = Pulse::of(&l);
        p.flinch = 2.0;
        for _ in 0..10 {
            p.fade();
        }
        assert!((p.power - 1.0).abs() < 1e-9 && (p.fine + 0.5).abs() < 1e-9, "half gone after its half-life");
        assert!((p.flinch - 2.0 * FLINCH_KEEP.powi(10)).abs() < 1e-9);
        // The fine work's cost reads on Discipline and Concentration too.
        assert!(p.focus20() < 0.0);
    }

    #[test]
    fn the_draws_are_even() {
        let n = 4000;
        let mean: f64 = (0..n).map(|k| normal(7, k)).sum::<f64>() / n as f64;
        assert!(mean.abs() < 0.06, "{mean}");
    }
}
