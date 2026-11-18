//! How he tells it (lane N1, sections 3.2, 10.1 and 10.2): one of four
//! ways, straight, partly, deflects or lies, from his trust in you, his
//! professionalism, his loyalty to the man it's about, his own stake in
//! the answer and who else is in the room (`TellParts`), weighed against
//! how touchy the topic is and his honesty. The draw is keyed to him, the
//! question and the day, so the same question the same day gets the same
//! answer, and a reload plays the same world.
//!
//! What he releases is words from what he knows. A partial answer gives the
//! safe half. When he holds back, the voice gets that he's holding back and
//! the tell your read caught, never the secret. A lie goes to his lies
//! store with the day you could find out (`knowing::shows_on`).
//!
//! Every size here is Claude's (proposed, not confirmed).

use super::Them;
use super::knows::{Answer, AskCost, Known, Source, TellParts, Told, Topic};
use super::makeup::{self, Makeup};
use super::memory::HisLie;
use super::room::Room;
use crate::world::World;

/// Minutes a question takes, and a minute more for every two things he
/// tells (section 4: your time).
pub const ASK_MINUTES: f32 = 1.5;

/// The trust each push on the same man costs, before his loyalty (3.2:
/// "why do you keep asking me about Tre?").
pub const PUSH_TRUST: f32 = 0.02;

/// How touchy a topic is, 0 (football, open) to 1 (his own case): it
/// raises what he needs in trust before he talks.
pub fn touchy(t: Topic) -> f64 {
    match t {
        Topic::Case => 0.6,
        Topic::RuleBreak => 0.5,
        Topic::Portal => 0.4,
        Topic::Life | Topic::Character => 0.3,
        Topic::Nil | Topic::Pitches | Topic::Market | Topic::Grades => 0.3,
        Topic::Injury => 0.25,
        Topic::Mood | Topic::Wants => 0.15,
        Topic::Money | Topic::Club | Topic::ScoutFile | Topic::Goals => 0.35,
        Topic::Jobs | Topic::Grapevine | Topic::Opponent => 0.2,
        Topic::HisWord => 0.5,
        Topic::Rules | Topic::Football | Topic::Nothing | Topic::Home => 0.0,
        Topic::Job => 0.1,
        Topic::Effort => 0.2,
    }
}

/// His stake in the answer, -1 to 1 (3.2): a recruit oversells his offers
/// for leverage, an agent sells his client, a proud or tough man hides a
/// knock, a man in a case denies it, a player covers for himself.
pub fn interest(w: &World, him: Them, topic: Topic, about: Option<Them>) -> f64 {
    use super::wants::Kind as K;
    let kind = makeup::kind(w, him);
    let m = makeup::makeup(w, him);
    let own = about.is_none() || about == Some(him);
    let z = Makeup::z;
    match (topic, kind) {
        (Topic::Nil | Topic::Pitches, K::Recruit | K::Transfer | K::Family) => 0.35 + 0.3 * z(m.greed),
        (Topic::Market, K::Agent | K::Player | K::Prospect) => 0.4 + 0.3 * z(m.greed),
        (Topic::Character | Topic::Wants, K::Agent) => 0.5,
        (Topic::Injury, _) if own => (0.2 + 0.4 * z(m.ego) + 0.3 * z(m.professionalism).max(0.0)).clamp(-1.0, 1.0),
        (Topic::Case | Topic::RuleBreak, _) if own => 0.7,
        (Topic::Portal, K::Player) if own => 0.3,
        (Topic::Club | Topic::ScoutFile, K::Gm | K::OtherScout) => 0.3,
        (Topic::Character, K::Prospect) if own => 0.4,
        _ => 0.0,
    }
}

/// His loyalty to the man it's about, 0-1: his tie and his loyalty trait,
/// all of it for family, a coach to his own player, an agent to his client.
pub fn loyalty_to(w: &World, him: Them, about: Option<Them>) -> f64 {
    let Some(Them::Player(id)) = about else { return 0.0 };
    if about == Some(him) {
        return 0.0;
    }
    let Some(i) = crate::world::folk::person(w, id) else { return 0.0 };
    let s = super::knowing::stand(w, him, i);
    let m = makeup::makeup(w, him);
    let trait_ = (m.loyalty / 20.0).clamp(0.0, 1.0);
    let base = if s.family {
        0.95
    } else if s.agent {
        0.8
    } else if s.coach || s.hs_coach {
        0.45
    } else {
        (s.tie as f64 / 100.0).min(0.9)
    };
    (base * (0.6 + 0.6 * trait_)).clamp(0.0, 1.0)
}

/// How much the room hears (5.6): 0 alone, more with every other person,
/// most for strangers to him.
pub fn room_weight(room: &Room) -> f64 {
    use super::room::{Channel, Seat};
    let others = room.present.iter().filter(|p| p.seat != Seat::Across).count() as f64;
    let group = matches!(room.channel, Channel::Group(_));
    let base = if group { 0.8 } else { 0.0 };
    (base + 0.2 * others).min(1.0)
}

/// What weighs on how he answers, from the world (3.2).
pub fn parts(w: &World, him: Them, topic: Topic, about: Option<Them>, room: Option<&Room>, pushed: u8) -> TellParts {
    let m = makeup::makeup(w, him);
    // A player close to you answers straight more often: openness + 0.3 x
    // closeness, through the trust part's 1.2 (coach and player bonds).
    let close = match him {
        Them::Player(id) => f64::from(crate::world::folk::bonds::ear_you(w, id)) / 0.3 * 0.25,
        _ => 0.0,
    };
    TellParts {
        trust: makeup::trust(w, him) + close,
        professionalism: m.professionalism,
        loyalty: loyalty_to(w, him, about),
        interest: interest(w, him, topic, about),
        room: room.map_or(0.0, room_weight),
        pushed,
    }
}

/// His odds of each way, from what weighs on him: (straight, partly,
/// deflects, lies), summing to 1.
pub fn odds(m: &Makeup, topic: Topic, about_other: bool, parts: &TellParts) -> [f64; 4] {
    let z = Makeup::z;
    // Openness: trust most, then professionalism (a pro answers a fair
    // question), less for a touchy topic, a loyal man about his friend, a
    // room, and a man you keep pushing.
    let open = 0.55 + 1.2 * (parts.trust - 0.5) + 0.25 * z(parts.professionalism) + 0.15 * z(m.warmth)
        - 0.6 * touchy(topic)
        - if about_other { 0.6 * parts.loyalty } else { 0.0 }
        - 0.3 * parts.room * (0.5 + touchy(topic))
        - 0.12 * parts.pushed as f64;
    let open = open.clamp(0.02, 0.98);
    // Lying: his stake, against his honesty and professionalism; a loyal
    // man covers for a friend.
    let cover = if about_other { 0.35 * parts.loyalty } else { 0.0 };
    let lie = ((parts.interest.max(0.0) + cover) * (0.55 - 0.45 * z(m.honesty) - 0.15 * z(parts.professionalism))
        - 0.1 * (parts.trust - 0.5))
        .clamp(0.0, 0.7);
    let straight = open * (1.0 - lie);
    let rest = (1.0 - open) * (1.0 - lie);
    // What he doesn't tell straight he tells partly or deflects: a warm man
    // gives the safe half, a proud or hot one shuts it down.
    let half = (0.5 + 0.2 * z(m.warmth) - 0.15 * z(m.ego) - 0.1 * z(m.temper)).clamp(0.1, 0.9);
    [straight, rest * half, rest * (1.0 - half), lie]
}

/// The tell your read catches when he holds back or lies (3.2,
/// `talks::sharpness`): a sharp read sees it, a weak one a sign at best.
pub fn tell_words(w: &World, m: &Makeup, answer: Answer, salt: u64) -> String {
    let sharp = crate::world::folk::talks::sharpness(w);
    let caught = crate::world::folk::roll(w, 0x9E60 ^ salt, w.today.0 as u64) < (sharp * 0.9).clamp(0.05, 0.95);
    if !caught {
        return String::new();
    }
    let weak = sharp < 0.45;
    match answer {
        Answer::Straight => String::new(),
        Answer::Lies if weak => "He answers a beat too fast.".into(),
        Answer::Lies if m.temper >= 13.0 => "He gets loud about it, louder than it needs.".into(),
        Answer::Lies => "He holds your eye a little too long.".into(),
        _ if weak => "He shifts in his chair.".into(),
        _ if m.ego >= 13.0 => "He cuts it short. He doesn't like the question.".into(),
        _ if m.loyalty >= 13.0 => "He looks away. He's protecting someone.".into(),
        _ => "He's choosing his words.".into(),
    }
}

/// The safe half of what he knows: the first fact, its label without its
/// details, and nothing he'd hold against himself.
fn half(k: &Known) -> String {
    match k.words.split_once(" (") {
        Some((label, _)) => label.to_string(),
        None => k.words.split(';').next().unwrap_or(&k.words).to_string(),
    }
}

/// The lie he'd tell on a topic (10.2): a cover, a denial, an oversell.
/// The voice gets the lie, never the truth.
pub fn lie_words(topic: Topic, about_other: bool, k: Option<&Known>) -> String {
    match topic {
        Topic::Case if !about_other => "He didn't do it. It's a misunderstanding.".into(),
        Topic::Case => "He wasn't there. It's not what people say.".into(),
        Topic::RuleBreak if !about_other => "He hasn't broken any rule.".into(),
        Topic::RuleBreak => "Nothing happened. He'd have heard.".into(),
        Topic::Injury => "He's fine. Nothing's wrong.".into(),
        Topic::Portal => "He's not going anywhere.".into(),
        Topic::Mood | Topic::Life => "He's good. Everything's fine.".into(),
        Topic::Nil | Topic::Pitches | Topic::Market => match k {
            // An oversell: more schools, more money than there is.
            Some(k) => format!("{}, and more on the way, bigger than anyone's said", half(k)),
            None => "Everyone's calling. The offers are bigger than you think.".into(),
        },
        Topic::Grades => "His grades are fine.".into(),
        Topic::Character => "He's a great kid, works as hard as anyone.".into(),
        Topic::Effort => "He's working as hard as anyone.".into(),
        Topic::Club | Topic::ScoutFile => "We're happy with what we have there.".into(),
        Topic::Opponent => "They don't do anything special. Base stuff.".into(),
        _ => "He doesn't know anything about that.".into(),
    }
}

/// His answer to one question (3.2).
pub fn tell(w: &World, him: Them, known: &[Known], parts: &TellParts) -> Told {
    let m = makeup::makeup(w, him);
    let topic = known.first().map_or(Topic::Nothing, |k| k.topic);
    let about = known.first().and_then(|k| k.about);
    let about_other = about.is_some_and(|a| a != him);
    let minutes = ASK_MINUTES + 0.5 * known.len().min(6) as f32;
    let push = if about_other { PUSH_TRUST * parts.pushed as f32 * (1.0 + parts.loyalty as f32) } else { 0.0 };
    let cost = AskCost { minutes, trust: push };
    // He doesn't know: he says so (a proud man may bluff, but there is
    // nothing to bluff with, so the game keeps it straight).
    if known.is_empty() {
        return Told { answer: Answer::Straight, released: vec!["He doesn't know.".into()], cost, ..Told::default() };
    }
    let o = odds(&m, topic, about_other, parts);
    let salt = makeup::key(him)
        ^ (topic as u64) << 48
        ^ about.map_or(0, makeup::key).rotate_left(17)
        ^ (parts.pushed as u64) << 56;
    let u = crate::world::folk::roll(w, 0x9E61 ^ salt, w.today.0 as u64);
    let answer = if u < o[3] {
        Answer::Lies
    } else if u < o[3] + o[0] {
        Answer::Straight
    } else if u < o[3] + o[0] + o[1] {
        Answer::Partly
    } else {
        Answer::Deflects
    };
    // Football and what he saw on the tape, a man tells straight unless
    // it's touchy; nothing about football is a secret worth a lie.
    let answer = if topic == Topic::Football && answer == Answer::Lies { Answer::Partly } else { answer };
    // Asked about his own game, he talks about it, if only the safe half
    // (the talks audit, 2026-11-16: a recruit asked "how was your game
    // Friday?" would rather not get into it).
    let answer =
        if topic == Topic::Football && !about_other && answer == Answer::Deflects { Answer::Partly } else { answer };
    // Asked about his own job, a man makes his case: he wants to keep it.
    let answer = if topic == Topic::Job && !about_other { Answer::Straight } else { answer };
    // Your boss tells you the goals he holds you to, where you stand with
    // him and how the money works: setting them is his job, never a secret
    // (the talks audit, 2026-11-16: asked "what are you looking for from
    // us this year?" in his goals meeting, he'd rather not get into it).
    let boss = super::makeup::kind(w, him) == super::wants::Kind::Boss;
    let answer =
        if boss && matches!(topic, Topic::Goals | Topic::Money | Topic::Wants | Topic::Football) && !about_other {
            Answer::Straight
        } else {
            answer
        };
    let mut told = Told { answer, cost, ..Told::default() };
    match answer {
        Answer::Straight => told.released = known.iter().map(|k| k.words.clone()).collect(),
        Answer::Partly => {
            told.released = vec![half(&known[0])];
            told.holding_back = true;
        }
        Answer::Deflects => {
            told.released =
                vec![if about_other { "Ask him yourself.".into() } else { "He'd rather not get into it.".into() }];
            told.holding_back = true;
        }
        Answer::Lies => {
            let k = known.iter().find(|k| k.from == Source::Himself).or(known.first());
            let words = lie_words(topic, about_other, k);
            told.released = vec![words.clone()];
            told.lie = Some(HisLie {
                by: him,
                topic,
                about,
                said: w.today,
                shows: k.and_then(|k| super::knowing::shows_on(k, w.today)),
                words,
                caught: false,
            });
        }
    }
    told.tell = tell_words(w, &m, answer, salt);
    told
}
