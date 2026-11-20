//! Asking people to do things (lane N3; `docs/design/live-talks-persona.md`,
//! sections 3.4 and 8.3, question 4).
//!
//! Whether he'll do it comes from his trust in you, his loyalty, his
//! leadership and what it costs or gains him (`willing`): a veteran asked
//! to mentor the kid who'll take his job weighs that, a friend asked to tell
//! on a friend weighs the friendship. When he agrees, the game runs the
//! action by its own rules (`run`), never the AI, and it can go wrong:
//!
//! - **Mentor a young player**: a real mentor tie (`ties::mentor`). The
//!   yearly mentor step then moves the young man's professionalism as any
//!   mentor's does, down as well as up (`ties::mentor_year`).
//! - **Talk to a teammate, calm a teammate**: the game plays the talk
//!   between them from their traits and their friendship. It lands or it
//!   doesn't: the teammate's mood moves (a dated reason, the locker room),
//!   and their friendship grows or strains. He reports back in his own
//!   words, honest or not, by his professionalism, loyalty and trust.
//! - **Recruit a friend, host a recruit**: a real point in your school's
//!   case with the recruit (`case.rs`, the people group), sized by today's
//!   pitch rule with the player as the man making it: how much the recruit's
//!   camp cares about people, how well the player knows him, how well he
//!   sells. An unhappy man sells it badly; a host with little
//!   professionalism can have the night go wrong.
//! - **Keep something quiet**: your secrets with him are marked as asked;
//!   the odds he passes one on are `passes_on`.
//! - **Tell you when something happens**: he's kept as a watcher
//!   (`memory::Watch`); `tells_you` says whether he tells you of a break he
//!   saw. A position coach told to watch a man gets your built line ("tell
//!   me if he's slipping"); told to work with one, "push him".
//!
//! The rest of 8.3 is weighed here and run by its facet lane (a position
//! move by your depth chart, a pay cut by the contract lane, a plan setting
//! by the staff lane): `run` says so in `Ran::done` and changes nothing.
//! Every size here is Claude's (proposed, not confirmed).

use super::Them;
use super::ear::{Act, Confirm};
use super::memory::Watch;
use super::terms::{Counter, Give, Request};
use super::wants::{Acting, Ran, Willing};
use crate::player::{PlayingTime, Trait};
use crate::world::folk::claims::{Group, Tier};
use crate::world::folk::talks::{self, Reason};
use crate::world::folk::{Team, case, person, roll, ties};
use crate::world::{Stage, World};

/// A score at or over this is a yes; down to `IF` he'll do it for
/// something. Proposed, not confirmed.
pub const YES: f64 = 0.5;
pub const IF: f64 = 0.35;
/// A teammate talk that lands moves the teammate's mood this much (more
/// with the speaker's leadership); one that fails costs this. Proposed, not
/// confirmed.
pub const TALK_UP: f32 = 0.06;
pub const TALK_DOWN: f32 = 0.04;
/// A failed talk strains their friendship this much; one that lands grows
/// it. Proposed, not confirmed.
pub const STRAIN: u8 = 8;
pub const GROW: u8 = 6;
/// A host whose night goes wrong: the recruit's case takes the point
/// floor (`case::POINT_RANGE`'s low end).
pub const BAD_NIGHT: f64 = 0.15;

/// Runs requests.
#[derive(Clone, Copy, Debug, Default)]
pub struct Asks;

fn you(w: &World) -> Option<u32> {
    w.career.as_ref().map(|c| c.coach as u32)
}

/// A coach whose job is with your team (not you).
fn on_your_staff(w: &World, c: usize) -> bool {
    use crate::world::coach::Employer;
    let Some(job) = w.coaches.get(c).and_then(|x| x.job) else { return false };
    let you = w.career.as_ref().map(|x| x.coach);
    you != Some(c)
        && match (Team::seat(w), job.employer) {
            (Some(Team::College(p)), Employer::College(q)) => p == q,
            (Some(Team::Nfl(t)), Employer::Nfl(q)) => t == q,
            (Some(Team::High(h)), Employer::HighSchool(q)) => h == q,
            (Some(Team::Other(f)), Employer::Other(q)) => f == q,
            _ => false,
        }
}

/// His trust in you, 0-1: a stranger's is 0.4.
fn trust(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    if you(w).is_some_and(|c| c == p.trust_in) { p.trust as f64 } else { 0.4 }
}

fn z(w: &World, i: usize, t: Trait) -> f64 {
    w.people[i].player.personality.traits.z(t)
}

/// A core trait (1-20) as a z score.
fn core(x: f64) -> f64 {
    (x - 10.0) / 10.0
}

/// The kid's a threat to his job: same position, he starts, and the kid is
/// within five points of him or better.
fn threat(w: &World, i: usize, j: usize) -> bool {
    let (a, b) = (&w.people[i], &w.people[j]);
    a.player.position == b.player.position
        && a.last_role == Some(PlayingTime::Starter)
        && b.player.overall() >= a.player.overall() - 5.0
}

/// How willing he is, as a score: trust in you at the base, then what the
/// ask weighs for a man like him.
fn score(w: &World, i: usize, ask: &Request) -> Option<f64> {
    let p = &w.people[i];
    let per = &p.player.personality;
    let (lead, prof, loyal, ego, temper, adapt) = (
        z(w, i, Trait::Leadership),
        z(w, i, Trait::Professionalism),
        z(w, i, Trait::Loyalty),
        z(w, i, Trait::Ego),
        z(w, i, Trait::Temper),
        z(w, i, Trait::Adaptability),
    );
    let base = 0.5 + 0.5 * (trust(w, i) - 0.5);
    let id = p.player.id;
    let other = |x: u32| person(w, x);
    let s = match *ask {
        Request::Mentor(kid) => {
            let j = other(kid)?;
            if w.people[j].player.age >= p.player.age {
                return Some(0.0);
            }
            let cost = if threat(w, i, j) { 0.25 * (1.0 + ego.max(0.0)) } else { 0.0 };
            base + 0.2 * lead + 0.12 * prof + 0.08 * loyal - 0.1 * ego - cost
        }
        Request::TalkTo(mate) | Request::Calm(mate) => {
            let j = other(mate)?;
            let tie = ties::between(w, id, mate) as f64 / 100.0;
            let rival = if p.player.position == w.people[j].player.position { 0.1 } else { 0.0 };
            base + 0.15 * lead + 0.15 * tie + 0.05 * prof - 0.08 * ego - rival
        }
        Request::Recruit(_) | Request::Host(_) => {
            // A happy man sells his school; an unhappy one won't.
            base + 0.25 * p.mood as f64 + 0.1 * lead + 0.05 * core(per.love_of_game)
        }
        Request::KeepQuiet => base + 0.1 * loyal + 0.1 * prof,
        Request::TellMe(about) => {
            let tie = about.map_or(0.0, |x| ties::between(w, id, x) as f64 / 100.0);
            base + 0.1 * lead - tie * (0.5 + 0.3 * loyal.max(0.0))
        }
        Request::Position(_) => {
            let role = match p.last_role {
                Some(PlayingTime::Starter) => -0.2,
                Some(PlayingTime::Rotation) => 0.0,
                _ => 0.15,
            };
            base + 0.2 * adapt - 0.15 * ego + role
        }
        Request::SpecialTeams => {
            let star = if p.last_role == Some(PlayingTime::Starter) { 0.1 + 0.15 * ego.max(0.0) } else { 0.0 };
            base + 0.15 * core(per.love_of_game) + 0.1 * core(per.work_ethic) - star
        }
        Request::PlayHurt => {
            let tough = (p.player.attributes.get(crate::attributes::Attr::Toughness) - 60.0) / 30.0;
            base + 0.15 * tough + 0.12 * core(per.competitiveness) - 0.1 * z(w, i, Trait::Greed)
        }
        Request::Weight(_) | Request::Class => base + 0.15 * core(per.work_ethic) + 0.1 * prof,
        Request::Report => base + 0.15 * prof - 0.15 * z(w, i, Trait::Greed) - 0.05 * temper,
        Request::PayCut => base - 0.1 + 0.2 * loyal - 0.25 * z(w, i, Trait::Greed),
        Request::Commit | Request::ShutDown | Request::EnrolEarly => base + 0.1 * loyal,
        _ => base,
    };
    Some(s.clamp(0.0, 1.0))
}

/// What he'd want for it, when he'd do it for something: a role for a
/// backup, a council seat for a leader, else your word you'll stay.
fn price(w: &World, i: usize) -> Counter {
    let p = &w.people[i];
    let give = if p.last_role != Some(PlayingTime::Starter) {
        Give::Role(1)
    } else if z(w, i, Trait::Leadership) > 0.2 && !w.folk.rules.council.contains(&p.player.id) {
        Give::Council
    } else {
        Give::Stay
    };
    // As he'd say it: the bare "a real role" was on his card as what he
    // says, and came back as "is that a real role for me?" (the talks
    // audit, 2026-11-16).
    Counter {
        asks: vec![format!("He'll do it if he gets {} for it", give.words())],
        gives: vec![give],
        terms: vec![],
        why: None,
    }
}

/// The odds he passes on something you asked him to keep quiet (10.3):
/// a loyal, professional man who trusts you keeps it; an ego talks.
pub fn passes_on(w: &World, him: Them) -> f64 {
    let Them::Player(id) = him else { return 0.25 };
    let Some(i) = person(w, id) else { return 0.25 };
    (0.35 - 0.2 * z(w, i, Trait::Loyalty) - 0.15 * z(w, i, Trait::Professionalism) + 0.1 * z(w, i, Trait::Ego)
        - 0.5 * (trust(w, i) - 0.5))
        .clamp(0.03, 0.8)
}

/// Whether a watcher who was there tells you of a break he saw (N1's
/// unseen breaks read it): his trust in you against his friendship with the
/// man, by his loyalty. A keyed draw, the same on a reload.
pub fn tells_you(w: &World, player: u32, there: &[u32], day: crate::calendar::Date) -> Option<Them> {
    w.folk.live.persona.watch.iter().find_map(|x| {
        let Them::Player(id) = x.by else { return None };
        if !there.contains(&id) || x.about.is_some_and(|a| a != player) || x.day > day {
            return None;
        }
        let i = person(w, id)?;
        let tie = ties::between(w, id, player) as f64 / 100.0;
        let odds = (0.3 + 0.4 * trust(w, i) - tie * (0.5 + 0.3 * z(w, i, Trait::Loyalty).max(0.0))).clamp(0.02, 0.9);
        (roll(w, 0x7E11 ^ day.0 as u64 ^ (player as u64) << 20, id as u64) < odds).then_some(x.by)
    })
}

/// Whether the game asks you to confirm an act before it does anything
/// (question 4): an offer, a decision, and a request that spends your
/// money, time, staff or a man's health, or changes his role or the plan.
/// A promise counts the moment you say it; a favour asked of goodwill
/// (mentor, talk to, keep quiet, tell me) runs when he agrees.
pub fn confirm(act: &Act) -> Option<Confirm> {
    match act {
        Act::Offer(o) => Some(Confirm::Offer(o.clone())),
        Act::Decision(d) => Some(Confirm::Decision(*d)),
        Act::Request(r) if costs_you(r) => Some(Confirm::Request(r.clone())),
        _ => None,
    }
}

/// Whether an ask is about one man by name (a teammate to mentor, a
/// recruit to host, a prospect to look into). Put to a whole room, the man
/// who answers for it takes it on himself, at his own price and on your
/// confirm; any other ask is the room's, with no price, no confirm and
/// nothing of his own run (`talk::Conversation`).
pub fn names_a_man(r: &Request) -> bool {
    matches!(
        r,
        Request::Mentor(_)
            | Request::TalkTo(_)
            | Request::Recruit(_)
            | Request::Host(_)
            | Request::Steer(_)
            | Request::NotOurDivision(_)
            | Request::Scout(_)
            | Request::FindOut(..)
            | Request::Evaluate(_)
            | Request::Plant(_)
            | Request::Target(_)
            | Request::WorkWith(_)
            | Request::SecondOpinion(_)
            | Request::Clear(_)
            | Request::Waiver(_)
            | Request::Appeal(_)
            | Request::Calm(_)
            | Request::TellMe(Some(_))
            | Request::HeadsUp(Some(_))
    )
}

/// A request that does something to your team or your money once he says
/// yes.
pub fn costs_you(r: &Request) -> bool {
    matches!(
        r,
        Request::Position(_)
            | Request::SpecialTeams
            | Request::PlayHurt
            | Request::PayCut
            | Request::Plan(_)
            | Request::Target(_)
            | Request::Scout(_)
            | Request::FindOut(..)
            | Request::Plant(_)
            | Request::Hire(_)
            | Request::Money(_)
            | Request::Time
            | Request::Fund
            | Request::SelfReport
            | Request::Waiver(_)
            | Request::Appeal(_)
            | Request::Steer(_)
            | Request::TakeOffer
            | Request::NotOurDivision(_)
            | Request::SecondOpinion(_)
            | Request::Clear(_)
    )
}

impl Confirm {
    /// The confirm card's line.
    pub fn words(&self) -> String {
        match self {
            Confirm::Offer(o) if !o.words.is_empty() => format!("Make the offer: {}", o.words),
            Confirm::Offer(o) => {
                let mut v: Vec<String> = o.gives.iter().map(|g| g.words()).collect();
                v.extend(o.terms.iter().map(|u| u.words()));
                format!("Make the offer: {}", v.join(", "))
            }
            Confirm::Decision(d) => format!("Go ahead: {}", decision_words(*d)),
            Confirm::Request(r) | Confirm::RequestOf(_, r) => format!("Ask him: {}", request_words(r)),
            Confirm::Action(a) => a.card.clone(),
        }
    }
}

/// A decision in words for the confirm card.
pub fn decision_words(d: super::terms::Decision) -> String {
    use super::terms::Decision as D;
    match d {
        D::Benched => "bench him".into(),
        D::Starting => "start him".into(),
        D::Suspended => "suspend him".into(),
        D::Cut => "cut him".into(),
        D::Traded => "trade him".into(),
        D::OfferPulled => "pull the offer".into(),
        D::CoachLetGo => "let the coach go".into(),
        D::Redshirt => "redshirt him".into(),
        D::PositionMoved(p) => format!(
            "move him to {}",
            crate::world::folk::claims::words::spot(
                crate::positions::Position::ALL[p as usize % crate::positions::POSITION_COUNT]
            )
        ),
    }
}

/// A request in words for the confirm card, read against your world: a
/// money ask names the bar you put up when it's on the table, the bar a
/// yes holds you to (`asked::on_table`, read at your level: a class rank
/// is college only).
pub fn request_words_in(w: &crate::world::World, r: &Request) -> String {
    use crate::world::folk::{Team, asked};
    if let (Request::Money(s), Some(team)) = (r, Team::seat(w)) {
        let what = super::office::asks::money_kind(asked::strip_goal(s)).words();
        return match asked::on_table(w, team, &asked::Case::read_in(w, team, s)).bar {
            Some(g) => format!("find {what}, and you're held to {}", asked::bar_words(g)),
            None => format!("find {what}"),
        };
    }
    request_words(r)
}

/// A request in words for the confirm card and the voice.
pub fn request_words(r: &Request) -> String {
    match r {
        Request::Mentor(_) => "take a young player under his wing".into(),
        Request::TalkTo(_) => "talk to a teammate for you".into(),
        Request::Recruit(_) => "recruit a friend".into(),
        Request::Host(_) => "host a recruit".into(),
        Request::KeepQuiet => "keep it between you".into(),
        Request::Position(p) => format!(
            "move to {}",
            crate::world::folk::claims::words::spot(
                crate::positions::Position::ALL[*p as usize % crate::positions::POSITION_COUNT]
            )
        ),
        Request::SpecialTeams => "play special teams".into(),
        Request::Weight(_) => "make weight".into(),
        Request::Class => "go to class".into(),
        Request::TellMe(_) => "tell you when something happens".into(),
        Request::Report => "report to camp".into(),
        Request::PayCut => "take a pay cut".into(),
        Request::PlayHurt => "play through it".into(),
        Request::Commit => "commit".into(),
        Request::ShutDown => "shut his recruitment down".into(),
        Request::EnrolEarly => "enrol early".into(),
        Request::Calm(_) => "calm a teammate down".into(),
        Request::Plan(k) => format!("set the game plan: {k}"),
        Request::WorkWith(_) => "work with a player".into(),
        Request::FindOut(..) => "find something out about a prospect".into(),
        Request::Evaluate(_) => "give you his read on a player".into(),
        Request::Whiteboard => "go up to the whiteboard".into(),
        Request::Plant(_) => "let word out that you like a prospect".into(),
        Request::SecondOpinion(_) => "get an outside specialist's opinion".into(),
        Request::Clear(_) => "clear him to play sooner".into(),
        Request::Money(s) => {
            format!("find {}", super::office::asks::money_kind(crate::world::folk::asked::strip_goal(s)).words())
        }
        Request::Hire(_) => "find the money for a hire".into(),
        Request::Time => "give you more time".into(),
        Request::Backing => "back you with the big booster".into(),
        Request::Fund => "put more into the collective".into(),
        Request::Other(s) => s.clone(),
        other => format!("{other:?}").to_lowercase(),
    }
}

impl Asks {
    /// A teammate talk the game plays between them: whether it lands, from
    /// the speaker's leadership, their friendship, the teammate's
    /// coachability and how hot or proud he runs. A keyed draw.
    fn talk_lands(w: &World, i: usize, j: usize, calm: bool) -> bool {
        let (a, b) = (&w.people[i], &w.people[j]);
        let tie = ties::between(w, a.player.id, b.player.id) as f64 / 100.0;
        let temper = z(w, j, Trait::Temper);
        let odds =
            0.35 + 0.25 * z(w, i, Trait::Leadership) + 0.2 * tie + 0.15 * core(b.player.personality.coachability)
                - if calm { 0.25 } else { 0.15 } * temper
                - 0.1 * z(w, j, Trait::Ego)
                + if w.folk.rules.council.contains(&a.player.id) { 0.05 } else { 0.0 };
        roll(w, 0x7A1C ^ w.today.0 as u64 ^ (b.player.id as u64) << 24, a.player.id as u64) < odds.clamp(0.05, 0.95)
    }

    /// He reports a talk that went badly as it went: by his
    /// professionalism, loyalty and trust in you.
    fn honest(w: &World, i: usize) -> bool {
        let odds =
            0.5 + 0.25 * z(w, i, Trait::Professionalism) + 0.2 * z(w, i, Trait::Loyalty) + 0.5 * (trust(w, i) - 0.5);
        roll(w, 0x40E5 ^ w.today.0 as u64, w.people[i].player.id as u64) < odds.clamp(0.1, 0.95)
    }

    /// A point a player makes to a recruit for your school: today's pitch
    /// rule (`case::RATE` x skill x (care x (0.3 + 0.7 x proof) - CLEAR)),
    /// with the player's own skill and how well he knows the recruit as the
    /// proof.
    fn pitch(w: &World, i: usize, r: usize, host: bool) -> f64 {
        let p = &w.people[i];
        let t = &p.player.personality.traits;
        let skill = (0.6 + 0.04 * (t.leadership as f64 - 10.0) + 0.03 * (t.aura as f64 - 10.0)).clamp(0.3, 1.5);
        let tie = ties::between(w, p.player.id, w.people[r].player.id);
        let mut proof = if tie >= 45 {
            Tier::Strong
        } else if tie >= 15 || p.hs == w.people[r].hs {
            Tier::Fair
        } else {
            Tier::Weak
        };
        if p.mood < 0.0 {
            proof = match proof {
                Tier::Elite => Tier::Strong,
                Tier::Strong => Tier::Fair,
                _ => Tier::Weak,
            };
        }
        let ear = crate::world::folk::pulls::deciding_ear(w, r);
        let care = crate::world::folk::claims::group_cares(&w.people[r], ear)[Group::People as usize] as f64;
        let lean = case::RATE * skill * (care * (0.3 + 0.7 * proof.value() as f64) - case::CLEAR);
        let lean = if host { lean * 0.75 } else { lean };
        if host {
            let bad = (BAD_NIGHT - 0.1 * z(w, i, Trait::Professionalism)).max(0.02);
            if roll(w, 0x4057 ^ w.today.0 as u64 ^ (r as u64) << 20, p.player.id as u64) < bad {
                return case::POINT_RANGE.0 as f64;
            }
        }
        lean.clamp(case::POINT_RANGE.0 as f64, case::POINT_RANGE.1 as f64)
    }
}

impl Acting for Asks {
    fn willing(&self, w: &World, him: Them, ask: &Request) -> Willing {
        match him {
            Them::Player(id) => {
                let Some(i) = person(w, id) else { return Willing::No };
                let Some(s) = score(w, i, ask) else { return Willing::No };
                if s >= YES {
                    Willing::Yes
                } else if s >= IF {
                    Willing::If(price(w, i))
                } else {
                    Willing::No
                }
            }
            // A coach on your staff does what his head coach asks of his
            // job.
            Them::Coach(c) => {
                if on_your_staff(w, c) {
                    Willing::Yes
                } else {
                    Willing::No
                }
            }
            _ => Willing::No,
        }
    }

    fn run(&self, w: &mut World, him: Them, ask: &Request) -> Ran {
        let mut ran = Ran { honest: true, ..Ran::default() };
        let today = w.today;
        match (him, ask) {
            (Them::Player(id), Request::Mentor(kid)) => {
                let Some(i) = person(w, id) else { return ran };
                let strength =
                    (40.0 + 2.0 * (w.people[i].player.personality.traits.leadership as f64 - 10.0)).clamp(20.0, 70.0);
                ties::mentor(w, id, *kid, strength as u8);
                ties::bond(w, id, *kid, 5);
                let kid_name = person(w, *kid).map_or_else(|| "the kid".into(), |j| w.people[j].name.clone());
                ran.done.push(format!("{} mentors {kid_name}.", w.people[i].name));
                ran.report.push("I'll take him under my wing.".into());
            }
            (Them::Player(id), Request::TalkTo(mate) | Request::Calm(mate)) => {
                let (Some(i), Some(j)) = (person(w, id), person(w, *mate)) else { return ran };
                let calm = matches!(ask, Request::Calm(_));
                let lands = Self::talk_lands(w, i, j, calm);
                let (a, b) = (w.people[i].name.clone(), w.people[j].name.clone());
                if lands {
                    let up = TALK_UP * (1.0 + z(w, i, Trait::Leadership).max(0.0) as f32);
                    talks::mood_event(w, *mate, Reason::Room, up);
                    ties::bond(w, id, *mate, GROW);
                    ran.done.push(format!("{a} talked to {b}. It helped."));
                    ran.report.push("We talked. He heard me.".into());
                } else {
                    talks::mood_event(w, *mate, Reason::Room, -TALK_DOWN);
                    ties::strain(w, id, *mate, STRAIN);
                    ran.done.push(format!("{a} talked to {b}. It went badly."));
                    ran.honest = Self::honest(w, i);
                    ran.report.push(if ran.honest {
                        "Honestly? It didn't go well. He didn't want to hear it from me.".into()
                    } else {
                        "We're good, coach. He's fine.".into()
                    });
                }
            }
            (Them::Player(id), Request::Recruit(rid) | Request::Host(rid)) => {
                let (Some(i), Some(r)) = (person(w, id), person(w, *rid)) else { return ran };
                let Some(Team::College(j)) = Team::seat(w) else { return ran };
                if w.people[r].stage != Stage::HighSchool && w.people[r].cal.portal.is_none() {
                    return ran;
                }
                let host = matches!(ask, Request::Host(_));
                let lean = Self::pitch(w, i, r, host) as f32;
                let moved = case::case_mut(w, r, j).add(Group::People, lean, 1.0, today);
                let (a, b) = (w.people[i].name.clone(), w.people[r].name.clone());
                ran.done.push(if lean < 0.0 {
                    format!("{a} {} {b}. It hurt your case.", if host { "hosted" } else { "talked to" })
                } else if moved > 0.0 {
                    format!("{a} {} {b}. It helped your case.", if host { "hosted" } else { "talked to" })
                } else {
                    format!("{a} {} {b}. It added nothing new.", if host { "hosted" } else { "talked to" })
                });
                ran.honest = lean >= 0.0 || Self::honest(w, i);
                ran.report.push(if lean >= 0.0 || !ran.honest {
                    "He liked it here. I told him how it is.".into()
                } else {
                    "It got out of hand. I'm sorry, coach.".into()
                });
            }
            (_, Request::KeepQuiet) => {
                for s in w.folk.live.persona.secrets.iter_mut().filter(|s| s.yours && s.to == him && !s.out) {
                    s.asked_quiet = true;
                }
                ran.done.push("He said he'd keep it between you.".into());
                ran.report.push("It stays with me.".into());
            }
            (Them::Coach(c), _) if !on_your_staff(w, c) => {
                ran.done.push("He isn't on your staff.".into());
            }
            (Them::Coach(_), Request::TellMe(Some(pid))) => {
                talks::set_line(w, *pid, Some(talks::Line::Watch));
                ran.done.push("Your position coach will tell you if he's slipping.".into());
            }
            // His read is said in the line itself (`evaluate::words`).
            (Them::Coach(_), Request::Evaluate(pid)) => {
                let name = person(w, *pid).map_or_else(|| "the player".into(), |j| w.people[j].name.clone());
                ran.done.push(format!("He gave you his read on {name}."));
            }
            (Them::Coach(_), Request::WorkWith(pid)) => {
                talks::set_line(w, *pid, Some(talks::Line::Push));
                ran.done.push("Your position coach will push him.".into());
            }
            (_, Request::TellMe(about)) => {
                w.folk.live.persona.watch.retain(|x| !(x.by == him && x.about == *about));
                w.folk.live.persona.watch.push(Watch { by: him, about: *about, day: today });
                ran.done.push("He'll tell you when something happens.".into());
                ran.report.push("I'll let you know.".into());
            }
            // Nothing the game runs: the ask is his word, and the card says
            // where the real thing is made, or that nothing holds him to it
            // (talk replies item 13: the catch-all "He agreed to ..." ran
            // nothing).
            (_, other) => ran.done.push(not_run(other)),
        }
        ran
    }
}

/// Why an ask that he agreed to runs nothing here (talk replies item 13):
/// where the screens make the real thing, or that nothing in the game
/// holds him to it. The game's side runs the asks a screen can make
/// (`afm-game`, `persona::actions::of_request`) before this.
pub fn not_run(r: &Request) -> String {
    let what = request_words(r);
    match r {
        Request::Position(_) => format!("He agreed to {what}. The move itself is made on his practice plan."),
        Request::SpecialTeams => {
            format!("He agreed to {what}. His spot on a unit is set on the depth chart's special teams boards.")
        }
        Request::Weight(_) => format!("He agreed to {what}. His target is set on his practice plan."),
        Request::Target(_) => format!("He agreed to {what}. The recruit goes on your board from the recruiting desk."),
        _ => format!("He gave you his word to {what}. Nothing in the game holds him to it."),
    }
}
