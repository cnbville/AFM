//! The conversation's stand-in parts, built only on what the world already
//! keeps, so a talk plays before lanes N1 (what he knows, tells and wants)
//! and N3 (deals and requests) land. They swap in their own parts where
//! the game builds `Parts`; nothing here is a design of theirs.
//!
//! - Knows: his own fact sheet (`claims::player`), his mood and its dated
//!   reasons, and a close friend's mood and life news (a tie of 35 and up,
//!   section 3.1's rule). Nothing about football or another team yet.
//! - Tells: by how far he opens up today, the one-on-one's own rule
//!   (`talks`, trust and adaptability), less with others in the room or
//!   about a friend he's loyal to. He never lies here: lies are lane N1's.
//! - Wants: his own priorities (`background::Family.own_wants`), the role he
//!   has, money for a pro.
//! - Deals and requests: he takes none up yet (lane N3).

use super::super::Them;
use super::super::knows::{Answer, AskCost, Known, Knows, Source, TellParts, Tells, Told, Topic};
use super::super::terms::{Best, Offer, Request, Valuing, Weighed};
use super::super::wants::{Acting, Kind, Ran, Want, WantKind, Wanting, Willing};
use crate::player::Trait;
use crate::world::folk::claims::{self, Kind as K};
use crate::world::folk::{self as folk, Team, talk_points as tp, talks};
use crate::world::{Stage, World};

/// The stand-in.
pub struct Today;

/// The kinds of point a topic covers, on his own sheet.
fn kinds(t: Topic) -> &'static [K] {
    match t {
        Topic::Life => &[K::Birth, K::Wedding, K::SickParent, K::Death, K::Divorce, K::MoneyLost, K::Homesick],
        Topic::Portal => &[K::Portal, K::WantsOut],
        Topic::Case => &[K::Arrest, K::CaseClosed, K::Suspended],
        Topic::RuleBreak => &[K::RuleBreak],
        Topic::Effort => &[K::Effort, K::PracticeWeek],
        Topic::Injury => &[K::Hurt, K::Back],
        Topic::Grades => &[K::Grades, K::FailedTerm],
        Topic::Football => {
            &[K::Buried, K::LostJob, K::WonJob, K::ChartMove, K::BigGame, K::BadGame, K::Slump, K::NewPosition]
        }
        Topic::Wants => &[K::Pay, K::WantsOut, K::Draft],
        _ => &[],
    }
}

/// His mood, in plain words.
pub fn mood_words(m: f32) -> &'static str {
    match m {
        x if x > 0.45 => "he's as happy as he's been",
        x if x > 0.15 => "he's in a good place",
        x if x > -0.15 => "he's steady, neither up nor down",
        x if x > -0.45 => "he's down",
        _ => "he's in a bad way",
    }
}

fn known(topic: Topic, about: Option<Them>, from: Source, words: String, eye: f64) -> Known {
    Known { topic, about, from, eye, point: None, says: Vec::new(), words, sure: 0.8, day: None }
}

impl Knows for Today {
    fn knows(&self, w: &World, him: Them, topic: Topic, about: Option<Them>) -> Vec<Known> {
        let Them::Player(me) = him else { return Vec::new() };
        let Some(i) = folk::person(w, me) else { return Vec::new() };
        let target = match about {
            None => me,
            Some(Them::Player(x)) => x,
            Some(_) => return Vec::new(),
        };
        let mine = target == me;
        if !mine && !super::super::super::ties::friends(w, me).contains(&target) {
            return Vec::new();
        }
        let Some(j) = folk::person(w, target) else { return Vec::new() };
        let from = if mine { Source::Himself } else { Source::Friend };
        let eye = 10.0 + 3.0 * w.people[i].player.personality.traits.z(Trait::Adaptability);
        let mut out = Vec::new();
        if topic == Topic::Mood {
            let mut words = mood_words(w.people[j].mood).to_string();
            if let Some((_, why, _)) = talks::reasons(w, target).into_iter().next() {
                words = format!("{words}: {why}");
            }
            out.push(known(topic, about, from, words, eye));
            return out;
        }
        // A friend knows his life news; the rest of a man's sheet is his.
        if !mine && topic != Topic::Life {
            return out;
        }
        let sheet = claims::player(w, j);
        for p in sheet.points.iter().filter(|p| kinds(topic).contains(&p.kind)) {
            let mut k = known(topic, about, from, tp::worry_words(p), eye);
            k.point = Some(p.kind);
            k.day = p.day;
            out.push(k);
        }
        out
    }
}

impl Tells for Today {
    fn tell(&self, w: &World, him: Them, known: &[Known], parts: &TellParts) -> Told {
        let cost =
            AskCost { minutes: 1.0, trust: if parts.pushed >= 2 { 0.02 * f32::from(parts.pushed - 1) } else { 0.0 } };
        if known.is_empty() {
            return Told {
                answer: Answer::Straight,
                released: vec!["He doesn't know.".into()],
                cost,
                ..Told::default()
            };
        }
        let adapt = super::z(w, him, Trait::Adaptability) as f64;
        let open = 0.15 + 0.7 * parts.trust + 0.1 * adapt - 0.25 * parts.room - 0.4 * parts.loyalty;
        let words: Vec<String> = known.iter().map(|k| k.words.clone()).collect();
        let (answer, released) = if open >= 0.55 {
            (Answer::Straight, words)
        } else if open >= 0.35 {
            (Answer::Partly, words.into_iter().take(1).collect())
        } else {
            (Answer::Deflects, Vec::new())
        };
        let holding_back = answer != Answer::Straight;
        let tell = if holding_back { "He's holding something back.".into() } else { String::new() };
        Told { answer, released, holding_back, tell, lie: None, cost }
    }
}

impl Wanting for Today {
    fn kind(&self, w: &World, him: Them) -> Kind {
        match him {
            Them::Player(id) => match folk::person(w, id).map(|i| w.people[i].stage) {
                Some(Stage::HighSchool | Stage::PrepSchool) => Kind::Recruit,
                Some(Stage::Declared) => Kind::Prospect,
                _ => Kind::Player,
            },
            Them::Kin(..) => Kind::Family,
            Them::Coach(c) => {
                let mine = Team::seat(w).is_some_and(|t| {
                    w.coaches.get(c).and_then(|c| c.job).is_some_and(|j| Team::from_employer(j.employer) == t)
                }) || crate::world::folk::staffing::my_junior(w, c);
                if mine { Kind::Staff } else { Kind::OtherCoach }
            }
            Them::Cast(..) => Kind::Boss,
            Them::Agent(_) => Kind::Agent,
            Them::ProScout(_) => Kind::OtherScout,
            Them::Drawn(k, _) => k,
        }
    }

    fn wants(&self, w: &World, him: Them) -> Vec<Want> {
        let Them::Player(id) = him else { return Vec::new() };
        let Some(i) = folk::person(w, id) else { return Vec::new() };
        let p = &w.people[i];
        let o = p.family.own_wants;
        let starter = talks::is_starter(w, i);
        let mut v = vec![
            Want { kind: WantKind::Role, weight: o.playing_time as f32, met: starter },
            Want { kind: WantKind::Money, weight: o.money as f32, met: false },
            Want { kind: WantKind::Place, weight: o.close_to_home as f32, met: false },
            Want { kind: WantKind::Academics, weight: o.academics as f32, met: p.stage == Stage::Pro },
            Want { kind: WantKind::Development, weight: o.development as f32, met: false },
            Want { kind: WantKind::Winning, weight: o.prestige as f32, met: false },
        ];
        if p.pro.is_some() {
            v.push(Want { kind: WantKind::Security, weight: 0.2, met: false });
        }
        let loyal = p.player.personality.traits.z(Trait::Loyalty) as f32;
        if loyal > 0.5 {
            v.push(Want { kind: WantKind::Loyalty, weight: 0.08 * loyal, met: false });
        }
        if !super::super::super::ties::friends(w, id).is_empty() {
            v.push(Want { kind: WantKind::Friends, weight: 0.08, met: false });
        }
        v.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        v
    }
}

impl Valuing for Today {
    fn best(&self, _: &World, _: Them) -> Best {
        Best::Stay
    }
    fn weigh(&self, _: &World, _: Them, _: &Offer) -> Weighed {
        Weighed::No
    }
}

impl Acting for Today {
    fn willing(&self, _: &World, _: Them, _: &Request) -> Willing {
        Willing::No
    }
    fn run(&self, _: &mut World, _: Them, _: &Request) -> Ran {
        Ran::default()
    }
}
