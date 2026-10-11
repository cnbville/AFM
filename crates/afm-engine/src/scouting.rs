//! What staff believe about a player. The user never sees true ratings or the
//! true ceiling: only reports like these, as good as the scout who wrote them.

use crate::career::NFL_LEVELS;
use crate::development::{POTENTIAL_BAND, potential_plasticity};
use crate::player::{DevelopmentEvent, Level, Player};
use crate::rng::{Rng, clamp, round_half_up};

/// A scout's report. Every number in it is an estimate with the scout's error.
#[derive(Clone, Copy, Debug)]
pub struct ScoutReport {
    pub overall: f64,
    /// Central estimate of potential. Engine-internal: the player-facing side
    /// gets the range and stars, or the vaguer `PotentialOutlook`.
    pub(crate) potential: f64,
    /// Displayed range, rounded out to whole potential bands.
    pub potential_low: f64,
    pub potential_high: f64,
    /// Football Manager style stars (0.5-5) for current ability and potential.
    pub current_stars: f64,
    pub potential_stars: f64,
}

/// How hard a level is to evaluate: small schools play weaker opponents and get less tape.
fn level_visibility(level: Level) -> f64 {
    match level {
        Level::HS => 1.25,
        Level::SmallCollege => 1.5,
        Level::FCS => 1.2,
        Level::FBS => 1.0,
        Level::NFL => 0.7,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ScoutOptions {
    /// Scout's judging ability, 1-20.
    pub accuracy: f64,
    pub level: Level,
}

pub fn scout_player(player: &Player, opts: ScoutOptions, rng: &mut Rng) -> ScoutReport {
    let skill = 1.4 - opts.accuracy * 0.04; // 10 -> 1.0, 20 -> 0.6
    let visibility = level_visibility(opts.level);
    let true_overall = player.overall();
    let true_potential = player.potential();

    let overall_sd = 2.5 * skill * visibility;
    // Potential is hardest to read in young players whose ceilings still move.
    let potential_sd = (2.5 + 7.0 * potential_plasticity(player.age)) * skill * visibility;

    let est_overall = clamp(true_overall + rng.normal(0.0, overall_sd), 1.0, 99.0);
    let est_potential = clamp(est_overall.max(true_potential + rng.normal(0.0, potential_sd)), 1.0, 99.0);
    let half_width = potential_sd;
    ScoutReport {
        overall: est_overall,
        potential: est_potential,
        potential_low: est_overall.max(((est_potential - half_width) / POTENTIAL_BAND).floor() * POTENTIAL_BAND),
        potential_high: 99f64.min(((est_potential + half_width) / POTENTIAL_BAND).ceil() * POTENTIAL_BAND),
        current_stars: to_stars(est_overall),
        potential_stars: to_stars(est_potential),
    }
}

/// Map a rating to half-stars on a pro scale: 50 = 1 star, 70 = 3 stars, 90 = 5 stars.
pub fn to_stars(rating: f64) -> f64 {
    clamp(round_half_up((rating - 40.0) / 5.0) / 2.0, 0.5, 5.0)
}

// --- What the player-facing side sees ------------------------------------------

/// A vague sense of how good a player could become, in words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PotentialTier {
    /// Unlikely to play beyond high school.
    HighSchool,
    /// Could play college football.
    College,
    /// Could make an NFL roster.
    NflRoster,
    /// Could become an NFL starter.
    NflStarter,
    /// Could become a Pro Bowl player.
    ProBowl,
    /// Could become one of the best in the league.
    Elite,
}

impl PotentialTier {
    fn from_rating(r: f64) -> Self {
        if r >= NFL_LEVELS.elite {
            PotentialTier::Elite
        } else if r >= NFL_LEVELS.pro_bowl {
            PotentialTier::ProBowl
        } else if r >= NFL_LEVELS.starter {
            PotentialTier::NflStarter
        } else if r >= NFL_LEVELS.roster {
            PotentialTier::NflRoster
        } else if r >= 52.0 {
            PotentialTier::College
        } else {
            PotentialTier::HighSchool
        }
    }

    pub fn phrase(self) -> &'static str {
        match self {
            PotentialTier::HighSchool => "unlikely to play beyond high school",
            PotentialTier::College => "could play college football",
            PotentialTier::NflRoster => "could make an NFL roster",
            PotentialTier::NflStarter => "could become an NFL starter",
            PotentialTier::ProBowl => "could become a Pro Bowl player",
            PotentialTier::Elite => "could become one of the best in the league",
        }
    }
}

/// How sure the staff is of its read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Confidence {
    Unsure,
    FairlySure,
    Sure,
}

/// Small clues about where a player is heading. Only good scouts pick them up,
/// and even they are sometimes wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hint {
    StillGrowingIntoHisBody,
    PhysicallyMatureAlready,
    BodyMayBeDoneGrowing,
    TirelessWorker,
    WorkEthicQuestions,
    SoaksUpCoaching,
}

impl Hint {
    pub fn phrase(self) -> &'static str {
        match self {
            Hint::StillGrowingIntoHisBody => "still growing into his body",
            Hint::PhysicallyMatureAlready => "looks physically mature already",
            Hint::BodyMayBeDoneGrowing => "his body may be done growing",
            Hint::TirelessWorker => "a tireless worker",
            Hint::WorkEthicQuestions => "questions about his work ethic",
            Hint::SoaksUpCoaching => "soaks up coaching",
        }
    }
}

/// Everything the player-facing side may know about a player's potential: a
/// tier in words, how sure the staff is, and a few hints. Never a ceiling.
#[derive(Clone, Debug)]
pub struct PotentialOutlook {
    pub tier: PotentialTier,
    pub confidence: Confidence,
    pub hints: Vec<Hint>,
}

/// Scout accuracy (1-20) from which hints start to show up.
pub const HINT_MIN_ACCURACY: f64 = 13.0;

/// A staff member's vague read on a player's potential. Better scouts read the
/// tier more often correctly, are surer of it, and notice more true hints.
pub fn potential_outlook(player: &Player, opts: ScoutOptions, rng: &mut Rng) -> PotentialOutlook {
    let report = scout_player(player, opts, rng);
    let width = report.potential_high - report.potential_low;
    let confidence = if width <= 10.0 {
        Confidence::Sure
    } else if width <= 20.0 {
        Confidence::FairlySure
    } else {
        Confidence::Unsure
    };
    let mut hints = Vec::new();
    if opts.accuracy >= HINT_MIN_ACCURACY {
        // Chance to notice each true clue, and to report a false one instead.
        let notice = clamp(0.3 + (opts.accuracy - HINT_MIN_ACCURACY) * 0.08, 0.0, 0.85);
        let misread = clamp(0.25 - (opts.accuracy - HINT_MIN_ACCURACY) * 0.03, 0.03, 0.25);
        let young = player.age <= 21;
        let p = &player.personality;
        let mut clue = |truth: bool, hint: Hint, opposite: Option<Hint>, rng: &mut Rng| {
            if truth && rng.chance(notice) {
                hints.push(if rng.chance(misread) { opposite.unwrap_or(hint) } else { hint });
            }
        };
        clue(young && player.bloom >= 1.0, Hint::StillGrowingIntoHisBody, Some(Hint::PhysicallyMatureAlready), rng);
        clue(young && player.bloom <= -1.0, Hint::PhysicallyMatureAlready, Some(Hint::StillGrowingIntoHisBody), rng);
        clue(young && player.has_had(DevelopmentEvent::BodyStalled), Hint::BodyMayBeDoneGrowing, None, rng);
        clue(p.work_ethic >= 15.0, Hint::TirelessWorker, Some(Hint::WorkEthicQuestions), rng);
        clue(p.work_ethic <= 5.0, Hint::WorkEthicQuestions, Some(Hint::TirelessWorker), rng);
        clue(p.coachability >= 15.0, Hint::SoaksUpCoaching, None, rng);
        hints.dedup();
    }
    PotentialOutlook { tier: PotentialTier::from_rating(report.potential), confidence, hints }
}
