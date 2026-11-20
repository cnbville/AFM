//! Each level's rule book for fouls (engine gaps lane EG5, items 23-25 and
//! 28, `docs/design/engine-gaps.md`): the yards, the spot and the first
//! down of every foul by the NFL's, the NCAA's and the NFHS's rules and
//! their changes by season, half the distance near either goal line, a foul
//! by the offense in its own end zone, and where a missed field goal and a
//! kickoff out of bounds go. Texas high schools play the NCAA's book in
//! every season, Massachusetts through 2018 (`Rules::high_school`). The
//! other leagues (`books.rs`) keep the NFL's fouls.
//!
//! Every rule here is the real one; how often each foul is flagged is the
//! players' and the crew's (`sim.rs`, `scrimmage.rs`, `flow.rs`).

use super::rules::Rules;
use super::sim::{Foul, FoulKind};
use crate::player::Level;

/// Whose foul rules a game is played under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Fouls {
    Nfl,
    /// NCAA football: college, and high school in Texas (every season) and
    /// Massachusetts (through 2018).
    Ncaa,
    /// The National Federation's high school rules.
    Nfhs,
}

impl Fouls {
    /// A level's own book.
    pub fn of_level(level: Level) -> Fouls {
        match level {
            Level::NFL => Fouls::Nfl,
            Level::HS => Fouls::Nfhs,
            _ => Fouls::Ncaa,
        }
    }
}

/// A foul's penalty under a book in a season: its yards, whether it is an
/// automatic first down for the offense, and whether it costs the offense
/// the down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Penalty {
    pub yards: f64,
    pub first_down: bool,
    pub loss_of_down: bool,
}

/// The season a book is read in: 0 (a game with no season) reads as today.
fn season(year: i32) -> i32 {
    if year == 0 { 2026 } else { year }
}

/// The penalty for a foul (real, by book and season). `side` is the
/// fouling team (0 the offense, 1 the defense), for the fouls both sides
/// commit. `crossed`: a forward pass was thrown past the line (college
/// defensive holding before 2022).
pub fn penalty(book: Fouls, year: i32, kind: FoulKind, side: usize, crossed: bool) -> Penalty {
    use FoulKind as K;
    let year = season(year);
    let p = |yards: f64, first_down: bool| Penalty { yards, first_down, loss_of_down: false };
    match kind {
        K::OffensiveHolding | K::IllegalBlock => p(10.0, false),
        K::IllegalUseOfHands if side == 0 => p(10.0, false),
        // The defense's illegal hands: the NFL's 5 and a first down; in
        // college and high school it is holding's 10 (college's with a
        // first down from 2022).
        K::IllegalUseOfHands | K::DefensiveHolding => match book {
            Fouls::Nfl => p(5.0, true),
            // From 2022 every defensive hold is a first down; before, only
            // one on an eligible receiver on a pass thrown past the line.
            Fouls::Ncaa => p(10.0, year >= 2022 || crossed),
            Fouls::Nfhs => p(10.0, false),
        },
        // The NFL's 5-yard contact rule; college and high school have none.
        K::IllegalContact => p(5.0, true),
        // Its spot is the foul's (`defense`); the yards are the most from
        // the previous spot where a book caps them.
        K::PassInterference => match book {
            Fouls::Nfl => p(0.0, true),
            Fouls::Ncaa => p(15.0, true),
            // NFHS: 15 from the previous spot, a first down only to 2012.
            Fouls::Nfhs => p(15.0, year < 2013),
        },
        K::OffensivePassInterference => p(if book == Fouls::Nfl { 10.0 } else { 15.0 }, false),
        K::RoughingPasser => p(15.0, true),
        // NFHS gives an automatic first down only for roughing the passer,
        // kicker, holder or snapper.
        K::UnnecessaryRoughness | K::FaceMask | K::Targeting => p(15.0, book != Fouls::Nfhs),
        // The defense's carries a first down in the NFL and college, not
        // under NFHS rules.
        K::UnsportsmanlikeConduct => p(15.0, side == 1 && book != Fouls::Nfhs),
        // NFHS: a snap taken with 12 on the field is illegal participation.
        K::TooManyMen => p(if book == Fouls::Nfhs { 15.0 } else { 5.0 }, false),
        K::FalseStart | K::Offside | K::NeutralZone | K::DelayOfGame | K::IllegalFormation | K::IneligibleDownfield => {
            p(5.0, false)
        }
        // From the previous spot (NFL) or the spot of the pass (college,
        // NFHS): `offense` places it.
        K::IntentionalGrounding => Penalty {
            yards: match book {
                Fouls::Nfl => 10.0,
                Fouls::Ncaa => 0.0,
                Fouls::Nfhs => 5.0,
            },
            first_down: false,
            loss_of_down: true,
        },
        // Placed by the kickoff rules (`kick_out_spot`), not marked off.
        K::KickoffOutOfBounds | K::KickoffShort => p(0.0, false),
    }
}

/// The NFL's fines and suspensions for flagged hits (the league office
/// fines nearly every one, `world/folk/rules.rs`): its men pull up on a
/// late hit and keep their hands off the face mask more than college and
/// high school players, who face no fine. Real on scrimmage plays: NFL
/// personal fouls with horse collars and taunting 0.308 flagged a team
/// game and face masks 0.134 (nflverse 2021-25); FBS about 0.40 and 0.19
/// (cfbfastR 2023-24, FBS against FBS). A share of the same draws, read on
/// the same men (EG5; proposed, not confirmed).
pub fn hit_restraint(book: Fouls) -> f64 {
    if book == Fouls::Nfl { 0.7 } else { 1.0 }
}

/// The share of the same hits on a passer each book flags as roughing:
/// the NFL's protection is the widest (no landing on him with the body's
/// weight since 2018, no low hit since 2006, the head and neck). Real
/// NFL 0.190 flagged a team game on scrimmage plays, FBS about 0.10
/// (cfbfastR 2023-24); high school is read as college (no count). EG5,
/// proposed, not confirmed.
pub fn roughing_share(book: Fouls) -> f64 {
    if book == Fouls::Nfl { 1.0 } else { 0.62 }
}

/// How far past the line an ineligible man may be when a forward pass
/// crosses it: the NFL's one yard, the NCAA's three (college, and Texas
/// and Massachusetts high schools on its book), the NFHS's two.
pub fn downfield(book: Fouls) -> f64 {
    match book {
        Fouls::Nfl => 1.0,
        Fouls::Ncaa => 3.0,
        Fouls::Nfhs => 2.0,
    }
}

/// Whether a foul at the snap lets the play run (a live-ball foul): the
/// offense's illegal formation, shift or motion and the defense's too many
/// men at every level, the defense's offside at the snap in the NFL and in
/// college. False starts, delays, neutral zone infractions and NFHS
/// encroachment kill the play.
pub fn live_at_snap(book: Fouls, kind: FoulKind) -> bool {
    match kind {
        FoulKind::IllegalFormation | FoulKind::TooManyMen => true,
        FoulKind::Offside => book != Fouls::Nfhs,
        _ => false,
    }
}

/// What a play did, for marking off a foul during it (the offense's frame:
/// its own goal line 0, the goal it attacks `goal`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Play {
    /// The previous spot (the line of scrimmage).
    pub los: f64,
    /// Where the ball ended.
    pub end: f64,
    pub goal: f64,
    /// A running play: a designed run or a scramble.
    pub run: bool,
    /// A completed forward pass (its run after the catch is a running play).
    pub complete: bool,
    pub turnover: bool,
    /// A forward pass was thrown past the line.
    pub crossed: bool,
}

impl Play {
    /// A running play (or the run after a catch) that ended beyond the
    /// line, with the ball still the offense's: its end is the basic spot.
    fn run_beyond(&self) -> bool {
        (self.run || self.complete) && !self.turnover && self.end > self.los
    }
}

/// A foul marked off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marked {
    /// Where the ball goes (the offense's frame).
    pub spot: f64,
    pub first_down: bool,
    pub loss_of_down: bool,
    /// The foul was by the offense in its own end zone: a safety.
    pub safety: bool,
    /// The play's yards stand, the penalty marked from its end.
    pub stands: bool,
    /// The enforcement spot: the yards marked off are `spot - from`.
    pub from: f64,
}

/// Half the distance: no distance penalty moves the ball more than half
/// the way from its enforcement spot to the offending team's goal line
/// (every level). `toward` is that goal line's x.
fn half(from: f64, yards: f64, toward: f64) -> f64 {
    let room = (toward - from).abs();
    let y = yards.min(room / 2.0);
    if toward > from { from + y } else { from - y }
}

/// A foul by the defense marked off (real, by book and season): from the
/// previous spot, from the end of a run that ended beyond the line for a
/// foul during it, or (pass interference) from its own spot; half the
/// distance at most.
pub fn defense(rules: &Rules, f: &Foul, p: &Play) -> Marked {
    use FoulKind as K;
    let book = rules.foul_book();
    let pen = penalty(book, rules.year, f.kind, 1, p.crossed);
    let mut m = Marked {
        spot: p.los,
        first_down: pen.first_down,
        loss_of_down: false,
        safety: false,
        stands: false,
        from: p.los,
    };
    match f.kind {
        K::PassInterference => {
            let at = f.at.max(p.los + 1.0);
            m.spot = match book {
                // The spot of the foul; in the end zone the 1, or half the
                // distance from a previous spot inside the 2.
                Fouls::Nfl if at < p.goal => at,
                Fouls::Nfl if p.los > p.goal - 2.0 => half(p.los, p.goal, p.goal),
                Fouls::Nfl => p.goal - 1.0,
                // The spot of the foul up to 15 yards from the previous
                // spot; in the end zone 15 yards or the 2, whichever comes
                // first, and half the distance from on or inside the 2.
                Fouls::Ncaa if at < p.goal => at.min(p.los + pen.yards),
                Fouls::Ncaa if p.los >= p.goal - 2.0 => half(p.los, p.goal, p.goal),
                Fouls::Ncaa => (p.los + pen.yards).min(p.goal - 2.0),
                // 15 from the previous spot, halved like any.
                Fouls::Nfhs => half(p.los, pen.yards, p.goal),
            };
            return m;
        }
        // Fouls at the snap: the previous spot.
        K::Offside | K::TooManyMen | K::NeutralZone => {}
        // Roughing the passer on a completion run beyond the line, a
        // personal foul or any other foul during a run (or the run after a
        // catch) that ended beyond the line: the end of the run, the play's
        // yards standing (every level).
        K::RoughingPasser if p.complete && p.run_beyond() => m.stands = true,
        K::RoughingPasser => {}
        _ if f.kind.personal() && p.end > p.los && !p.turnover => m.stands = true,
        K::DefensiveHolding | K::IllegalUseOfHands | K::IllegalContact if p.run && p.run_beyond() => m.stands = true,
        _ => {}
    }
    if m.stands {
        m.from = p.end;
    }
    m.spot = half(m.from, pen.yards, p.goal);
    m
}

/// A foul by the offense marked off (real, by book and season). A foul in
/// its own end zone is a safety (holding, an illegal block, grounding). A
/// hold or illegal block behind the line goes from the previous spot (the
/// NFL, college, and NFHS from 2023; before, NFHS marked it from the spot
/// of the foul); one beyond the line during a run that ended beyond it from
/// its own spot, or the end of the run when it was past it. Grounding: the
/// NFL's 10 from the previous spot or the spot of the pass when that is
/// farther back, college's spot of the pass, the NFHS's 5 from it; each
/// with the loss of the down. Every other foul from the previous spot. Half
/// the distance at most.
pub fn offense(rules: &Rules, f: &Foul, p: &Play) -> Marked {
    use FoulKind as K;
    let book = rules.foul_book();
    let year = season(rules.year);
    let pen = penalty(book, rules.year, f.kind, 0, p.crossed);
    let mut m = Marked {
        spot: p.los,
        first_down: false,
        loss_of_down: pen.loss_of_down,
        safety: false,
        stands: false,
        from: p.los,
    };
    let safety = |mut m: Marked| {
        m.safety = true;
        m.spot = 0.0;
        m.from = 0.0;
        m
    };
    let from = match f.kind {
        K::IntentionalGrounding => {
            let at = f.at.min(p.los);
            if at <= 0.0 {
                return safety(m);
            }
            m.stands = true;
            match book {
                Fouls::Nfl if at <= p.los - pen.yards => {
                    m.spot = at;
                    return m;
                }
                Fouls::Nfl => p.los,
                Fouls::Ncaa | Fouls::Nfhs => at,
            }
        }
        K::OffensiveHolding | K::IllegalUseOfHands | K::IllegalBlock => {
            if f.at <= 0.0 {
                return safety(m);
            }
            if f.at < p.los {
                if book == Fouls::Nfhs && year < 2023 { f.at } else { p.los }
            } else if p.run_beyond() && (p.run || f.kind == K::IllegalBlock) {
                f.at.min(p.end)
            } else {
                p.los
            }
        }
        _ => p.los,
    };
    m.from = from;
    m.spot = half(from, pen.yards, 0.0);
    m
}

/// Where the other team takes over after a missed field goal it didn't
/// field (its own frame: yards from its own goal line). `los` and `kick` are
/// the line and the spot of the kick in the kicking team's frame; `landed`
/// where the ball came down (the same frame), for a short one. The NFL: the
/// spot of the kick, or the 20 when that is inside it. College: the
/// previous spot, or the 20 (an untouched miss past the line). NFHS: a try
/// is a scrimmage kick, so a miss into the end zone is a touchback at the
/// 20, and a short one the defense's ball where it came down.
pub fn missed_fg_spot(rules: &Rules, los: f64, kick: f64, landed: Option<f64>, goal: f64) -> f64 {
    let twenty = 20.0f64.min(goal / 5.0);
    match rules.foul_book() {
        Fouls::Nfl => (goal - kick).max(twenty),
        Fouls::Ncaa => (goal - los).max(twenty),
        Fouls::Nfhs => match landed {
            Some(x) if x < goal => (goal - x).max(1.0),
            _ => twenty,
        },
    }
}

/// Where the receiving team takes a kickoff that goes out of bounds (its
/// own frame), from a free kick at `from` (the kicking team's yard line).
/// The NFL its 40 (and the other leagues their own book's spot). College:
/// its 35 through 2007, its 40 in 2008-2011, then 30 yards beyond the spot
/// of the kick (the 35 from the 35). NFHS: 25 yards beyond it (the 35 from
/// the 40). A spot measured from the kick moves with a free kick from
/// elsewhere (after a safety or a penalty).
pub fn kick_out_spot(rules: &Rules, from: f64, goal: f64) -> f64 {
    if !rules.code.american() {
        return rules.code.kick_out_spot;
    }
    let beyond = |y: f64| (goal - from - y).clamp(1.0, goal - 1.0);
    match rules.foul_book() {
        Fouls::Nfl => rules.code.kick_out_spot,
        Fouls::Ncaa => match season(rules.year) {
            ..=2007 => 35.0,
            2008..=2011 => 40.0,
            _ => beyond(30.0),
        },
        Fouls::Nfhs => beyond(25.0),
    }
}
