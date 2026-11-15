//! Who he is (lane N1, sections 3 and 6): his kind, read from where the
//! world holds him, and his makeup on one 1-20 scale from the store that
//! keeps each trait. Players have their hidden traits, coaches their
//! personality and ratings, the cast its eight traits, agents their
//! honesty and temper, scouts their eye. A trait a store doesn't keep is a
//! steady keyed draw for that person (as `dialogue::agent_temper` already
//! draws an agent's temper), leaned on the traits the store does keep, so
//! the same man is the same man in every talk. Every lean and spread here
//! is Claude's (proposed, not confirmed).
//!
//! Nothing here changes the world or makes a draw from the world's stream.

use super::Them;
use super::wants::Kind;
use crate::attributes::Attr;
use crate::player::Trait;
use crate::world::coach::Role as CoachRole;
use crate::world::folk::Team;
use crate::world::folk::cast::{self, Role as CastRole};
use crate::world::{Stage, World};

/// His makeup, every trait 1-20 (10 is ordinary). Engine numbers, never
/// shown: the cards carry them only as words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Makeup {
    pub temper: f64,
    pub loyalty: f64,
    pub professionalism: f64,
    pub ego: f64,
    /// How straight he is: how far he'd lie for his own interest.
    pub honesty: f64,
    /// How long he gives a talk that goes nowhere.
    pub patience: f64,
    /// How warm and open he is with people.
    pub warmth: f64,
    pub leadership: f64,
    pub greed: f64,
    pub ambition: f64,
    /// His football IQ on the 1-20 scale (section 9.1).
    pub iq: f64,
    /// His eye for people (3.1: a teammate's eye is his Football IQ and his
    /// years with the man, a high school coach's his recruiting rating, a
    /// GM's his judgment).
    pub judgement: f64,
}

impl Default for Makeup {
    fn default() -> Makeup {
        Makeup {
            temper: 10.0,
            loyalty: 10.0,
            professionalism: 10.0,
            ego: 10.0,
            honesty: 10.0,
            patience: 10.0,
            warmth: 10.0,
            leadership: 10.0,
            greed: 10.0,
            ambition: 10.0,
            iq: 10.0,
            judgement: 10.0,
        }
    }
}

impl Makeup {
    /// A trait as -1 to 1 around 10.
    pub fn z(v: f64) -> f64 {
        (v - 10.0) / 10.0
    }
}

/// The spread of a drawn trait, in trait points (the hidden traits' own,
/// `Traits::draw`: 10 + 3.5 x z).
const DRAW_SD: f64 = 3.5;

/// A steady key for anyone: draws, trust and memory use it.
pub fn key(him: Them) -> u64 {
    match him {
        Them::Player(id) => 0x1000_0000_0000 | id as u64,
        Them::Kin(id, k) => 0x2000_0000_0000 | (k as u64) << 32 | id as u64,
        Them::Coach(c) => crate::world::folk::coach_key(c),
        Them::Cast(r, term) => r.key(term),
        Them::Agent(a) => 0xA6E0_0000 ^ a as u64,
        Them::ProScout(k) => 0x5C00_0000_0000 | k as u64,
        Them::Drawn(kind, k) => 0xD000_0000_0000 ^ (kind as u64) << 40 ^ k,
    }
}

/// A trait the store doesn't keep: 10 + 3.5 x z, keyed to him, from the
/// people's own stream.
fn drawn(w: &World, him: Them, salt: u64) -> f64 {
    (10.0 + DRAW_SD * crate::world::folk::roll_n(w, 0x9E50_0000 ^ salt, key(him))).clamp(1.0, 20.0)
}

fn c20(v: f64) -> f64 {
    v.clamp(1.0, 20.0)
}

/// An NFL club's scout by `Them::ProScout`'s key: the club times 256 plus
/// his place in its department (lane N1 sets it; `pro_scout_key` builds it).
pub fn pro_scout(k: usize) -> (usize, usize) {
    (k / 256, k % 256)
}

pub fn pro_scout_key(club: usize, idx: usize) -> usize {
    club * 256 + idx.min(255)
}

/// The player's place in `w.people`, for a player or his family.
pub fn index(w: &World, him: Them) -> Option<usize> {
    match him {
        Them::Player(id) | Them::Kin(id, _) => crate::world::folk::person(w, id),
        _ => None,
    }
}

/// Your coach's id, when you have a career.
pub fn your_coach(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

/// The team a player is on now.
pub fn team_of(w: &World, i: usize) -> Option<Team> {
    crate::world::folk::claims::team_of(w, i)
}

/// The team a coach works for now.
pub fn coach_team(w: &World, c: usize) -> Option<Team> {
    w.coaches.get(c)?.job.map(|j| Team::from_employer(j.employer))
}

/// You're an assistant: your head coach is your boss (6.14).
fn assistant(w: &World) -> bool {
    your_coach(w).and_then(|c| w.coaches.get(c)).and_then(|c| c.job).is_some_and(|j| j.role != CoachRole::HeadCoach)
}

/// His kind, from where the world holds him (section 6).
pub fn kind(w: &World, him: Them) -> Kind {
    let seat = Team::seat(w);
    match him {
        Them::Player(id) => {
            let Some(i) = crate::world::folk::person(w, id) else { return Kind::Player };
            let p = &w.people[i];
            let mine = team_of(w, i).is_some_and(|t| Some(t) == seat);
            match p.stage {
                _ if mine => Kind::Player,
                Stage::HighSchool | Stage::PrepSchool => Kind::Recruit,
                Stage::College if p.cal.portal.is_some() => Kind::Transfer,
                Stage::Declared => Kind::Prospect,
                // Every college player is a prospect to an NFL staff.
                Stage::College if matches!(seat, Some(Team::Nfl(_))) => Kind::Prospect,
                _ => Kind::Player,
            }
        }
        Them::Kin(..) => Kind::Family,
        Them::Coach(c) => {
            let Some(coach) = w.coaches.get(c) else { return Kind::OtherCoach };
            let team = coach_team(w, c);
            if seat.is_some() && team == seat {
                if assistant(w) && coach.job.is_some_and(|j| j.role == CoachRole::HeadCoach) {
                    Kind::Boss
                } else {
                    Kind::Staff
                }
            } else if matches!(team, Some(Team::High(_))) && !matches!(seat, Some(Team::High(_))) {
                Kind::HsCoach
            } else {
                Kind::OtherCoach
            }
        }
        Them::Cast(r, _) => {
            let ours = seat == Some(r.team);
            match r.role {
                CastRole::AthleticDirector
                | CastRole::President
                | CastRole::Principal
                | CastRole::SchoolAd
                | CastRole::Superintendent
                | CastRole::Owner
                | CastRole::OwnerKin
                | CastRole::TeamPresident => {
                    if ours {
                        Kind::Boss
                    } else {
                        Kind::Outside
                    }
                }
                CastRole::Gm => {
                    if ours {
                        Kind::Boss
                    } else {
                        Kind::Gm
                    }
                }
                CastRole::Booster | CastRole::ClubPresident => Kind::Booster,
                CastRole::Reporter => Kind::Outside,
                CastRole::Feeder | CastRole::Youth => Kind::Circle,
                CastRole::Personnel if !ours => Kind::OtherScout,
                _ => {
                    if ours {
                        Kind::Staff
                    } else {
                        Kind::Outside
                    }
                }
            }
        }
        Them::Agent(_) => Kind::Agent,
        Them::ProScout(_) => Kind::OtherScout,
        Them::Drawn(k, _) => k,
    }
}

/// His makeup, from the store that holds him.
pub fn makeup(w: &World, him: Them) -> Makeup {
    match him {
        Them::Player(_) => match index(w, him) {
            Some(i) => player(w, i, him),
            None => drawn_makeup(w, him),
        },
        Them::Kin(_, k) => kin(w, him, k),
        Them::Coach(c) => match w.coaches.get(c) {
            Some(_) => coach(w, c, him),
            None => drawn_makeup(w, him),
        },
        Them::Cast(r, _) => cast_makeup(w, r, him),
        Them::Agent(a) if a < w.market.agents.len() => agent(w, a, him),
        Them::ProScout(k) => scout(w, k, him),
        _ => drawn_makeup(w, him),
    }
}

fn player(w: &World, i: usize, him: Them) -> Makeup {
    let p = &w.people[i];
    let per = &p.player.personality;
    let t = &per.traits;
    let g = |x: Trait| t.get(x);
    let z = |x: Trait| t.z(x);
    let iq = c20(p.player.attributes.get(Attr::FootballIq) / 5.0);
    Makeup {
        temper: g(Trait::Temper),
        loyalty: g(Trait::Loyalty),
        professionalism: g(Trait::Professionalism),
        ego: g(Trait::Ego),
        // Players keep no honesty: a pro is straighter, a proud or greedy
        // man shades the truth his way.
        honesty: c20(drawn(w, him, 1) + 3.0 * z(Trait::Professionalism) + 1.5 * z(Trait::Loyalty)
            - 2.0 * z(Trait::Ego)
            - 1.5 * z(Trait::Greed)),
        patience: c20(10.0 + 3.0 * z(Trait::Professionalism) - 4.0 * z(Trait::Temper) + 1.5 * z(Trait::Adaptability)),
        warmth: c20(10.0 + 2.5 * z(Trait::Leadership) + 2.5 * z(Trait::Loyalty) - 2.0 * z(Trait::Ego)
            + 0.15 * (p.bg.stability - 10.0)),
        leadership: g(Trait::Leadership),
        greed: g(Trait::Greed),
        ambition: g(Trait::Ambition),
        iq,
        judgement: c20(0.6 * iq + 0.25 * g(Trait::Leadership) + 0.15 * g(Trait::Adaptability)),
    }
}

/// His family: the decider at slot 0, then parents, grandparents,
/// siblings. The world keeps the family's shape and style, not each
/// member's traits, so these are drawn per slot, leaned on how the parent
/// shows up (`ParentStyle`).
fn kin(w: &World, him: Them, k: u8) -> Makeup {
    use crate::world::background::ParentStyle as S;
    let mut m = drawn_makeup(w, him);
    let Some(i) = index(w, him) else { return m };
    let f = &w.people[i].family;
    m.loyalty = c20(m.loyalty + 5.0);
    m.ambition = c20(0.5 * m.ambition + 0.5 * f.ambition);
    m.iq = c20(m.iq - 3.0);
    match f.parent_style {
        S::LoudAtGames => {
            m.temper = c20(m.temper + 3.0);
            m.patience = c20(m.patience - 2.0);
        }
        S::SocialMedia => {
            m.professionalism = c20(m.professionalism - 2.5);
            m.ego = c20(m.ego + 2.0);
        }
        S::CalmPrivate => {
            m.professionalism = c20(m.professionalism + 2.0);
            m.temper = c20(m.temper - 2.0);
        }
        S::Absent if k > 0 => {
            m.warmth = c20(m.warmth - 3.0);
        }
        S::Absent => {}
    }
    m
}

fn coach(w: &World, c: usize, him: Them) -> Makeup {
    let co = &w.coaches[c];
    let a = co.ability();
    let pe = &co.personality;
    Makeup {
        temper: pe.temper,
        loyalty: pe.loyalty,
        professionalism: c20(0.5 * drawn(w, him, 3) + 0.3 * a.discipline + 0.2 * a.relationships),
        ego: c20(drawn(w, him, 4) + 0.15 * (co.reputation - 10.0) + 1.0 * co.ex_pro as u8 as f64),
        honesty: c20(drawn(w, him, 1) + 0.2 * (pe.loyalty - 10.0)),
        patience: c20(10.0 - 0.4 * (pe.temper - 10.0) + 0.2 * (a.relationships - 10.0)),
        warmth: a.relationships,
        leadership: a.motivation,
        greed: c20(drawn(w, him, 5) + 0.25 * (pe.ambition - 10.0)),
        ambition: pe.ambition,
        iq: c20(0.6 * a.scheme + 0.4 * a.game_management),
        judgement: c20(0.5 * a.recruiting + 0.5 * a.relationships),
    }
}

fn cast_makeup(w: &World, r: cast::CastRef, him: Them) -> Makeup {
    let c = cast::get(w, r);
    let football = matches!(r.role, CastRole::Gm | CastRole::Personnel | CastRole::Strength | CastRole::Feeder);
    Makeup {
        temper: c.temper,
        loyalty: c20(drawn(w, him, 2) + 0.2 * (c.warmth - 10.0)),
        professionalism: c20(10.0 + 0.4 * (c.judgement - 10.0) - 0.3 * (c.meddling - 10.0)),
        ego: c.ego,
        honesty: c.honesty,
        patience: c.patience,
        warmth: c.warmth,
        leadership: c20(drawn(w, him, 6) + 0.2 * (c.judgement - 10.0)),
        greed: c20(10.0 + 0.5 * (c.tight - 10.0)),
        ambition: drawn(w, him, 7),
        iq: if football { c20(c.judgement + 1.0) } else { c20(0.5 * c.judgement + 2.0) },
        judgement: c.judgement,
    }
}

fn agent(w: &World, a: usize, him: Them) -> Makeup {
    let ag = &w.market.agents[a];
    let temper = crate::world::folk::dialogue::agent_temper(w, a);
    Makeup {
        temper,
        loyalty: c20(drawn(w, him, 2)),
        professionalism: c20(0.5 * ag.skill + 0.5 * 20.0 * ag.honesty),
        ego: c20(drawn(w, him, 4) + 1.5 * (ag.size as f64 - 2.0)),
        honesty: c20(20.0 * ag.honesty),
        patience: c20(10.0 - 0.4 * (temper - 10.0) + 0.2 * (ag.skill - 10.0)),
        warmth: drawn(w, him, 8),
        leadership: drawn(w, him, 6),
        greed: c20(drawn(w, him, 5) + 1.0 * (ag.size as f64 - 2.0)),
        ambition: drawn(w, him, 7),
        iq: c20(0.5 * ag.skill + 4.0),
        judgement: ag.skill,
    }
}

fn scout(w: &World, k: usize, him: Them) -> Makeup {
    let (club, idx) = pro_scout(k);
    let Some(s) = w.clubs.get(club).and_then(|c| c.scouts.get(idx)) else { return drawn_makeup(w, him) };
    let mut m = drawn_makeup(w, him);
    m.professionalism = c20(0.5 * m.professionalism + 0.5 * s.work as f64);
    m.iq = c20(s.judge as f64);
    m.judgement = c20(s.character as f64);
    m
}

/// Someone the world keeps no traits for: every trait drawn.
fn drawn_makeup(w: &World, him: Them) -> Makeup {
    let d = |s| drawn(w, him, s);
    Makeup {
        temper: d(9),
        loyalty: d(2),
        professionalism: d(3),
        ego: d(4),
        honesty: d(1),
        patience: d(10),
        warmth: d(8),
        leadership: d(6),
        greed: d(5),
        ambition: d(7),
        iq: d(11),
        judgement: d(12),
    }
}

/// Whether this player's trust in his head coach is his trust in you:
/// you coach him now.
pub fn coached_by_you(w: &World, i: usize) -> bool {
    your_coach(w).is_some_and(|c| w.people[i].trust_in as usize == c) && team_of(w, i) == Team::seat(w)
}

/// His trust in you, 0-1, from the store that keeps it: a player's trust
/// in his head coach when you're it, your contacts' warmth, your trust
/// with the cast and coaches (`Mine.trust`), an agent's trust in your word
/// (`Live.word`). Someone you've never dealt with starts where your name
/// with his audience puts him.
pub fn trust(w: &World, him: Them) -> f64 {
    use crate::world::folk::Name;
    let mine = &w.folk.mine;
    let name = |n: Name| mine.name(n) as f64;
    match him {
        Them::Player(_) => {
            let Some(i) = index(w, him) else { return 0.45 };
            let p = &w.people[i];
            if coached_by_you(w, i) {
                return p.trust as f64;
            }
            if let Some(t) = mine.trust_of(key(him)) {
                return t as f64;
            }
            // Someone else's player hears of you through the trail.
            (0.42 + 0.12 * name(Name::Trail)).clamp(0.2, 0.7)
        }
        Them::Kin(id, _) => {
            if let Some(t) = mine.trust_of(key(him)) {
                return t as f64;
            }
            // A family trusts you as far as their son does, and a little
            // less than him (they weren't in the room).
            let son = trust(w, Them::Player(id));
            (0.9 * son + 0.03).clamp(0.15, 0.85)
        }
        Them::Coach(c) => {
            if let Some(t) = mine.trust_of(crate::world::folk::coach_key(c)) {
                return t as f64;
            }
            let warm = w.folk.network.contacts.iter().find(|x| x.coach_id() == Some(c)).map(|x| x.warmth as f64);
            match warm {
                Some(x) => (0.35 + 0.5 * x).clamp(0.1, 0.95),
                None => 0.45,
            }
        }
        Them::Cast(r, term) => mine.trust_of(r.key(term)).map_or(0.5, |t| t as f64),
        Them::Agent(_) => w.folk.live.word_of(key(him)) as f64,
        Them::ProScout(_) | Them::Drawn(..) => mine.trust_of(key(him)).map_or(0.45, |t| t as f64),
    }
}

/// How long he has known the man (years on the same team), for a
/// teammate's eye (3.1).
pub fn years_with(w: &World, a: usize, b: usize) -> f64 {
    crate::world::chemistry::pair(w, a, b).map_or(0.0, |chem| {
        // chem = 1 - e^(-0.55 x seasons): back out the seasons.
        let c = (chem as f64).clamp(0.0, 0.99);
        -(1.0 - c).ln() / 0.55
    })
}

/// Stage words for a player, for a line ("a junior", "a ten-year vet").
pub fn stage_words(w: &World, i: usize) -> &'static str {
    let p = &w.people[i];
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => "a high school player",
        Stage::College | Stage::Declared => match p.college_seasons {
            0 | 1 => "a young college player",
            2 | 3 => "a college veteran",
            _ => "a fifth-year college player",
        },
        Stage::Pro => match p.pro.as_ref().map_or(0, |c| c.accrued) {
            0..=1 => "a young pro",
            2..=5 => "a pro in his prime",
            _ => "a veteran pro",
        },
        _ => "a former player",
    }
}
