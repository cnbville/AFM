//! The two talks you call yourself (lane N5c): a sit-down with someone in
//! your building (an assistant, the team doctor, the academic or
//! compliance director, your boss, a booster) and a reference call about a
//! coach (`refs::start`). With a usable key you type; without one, each
//! talk has a few plain chips, and every chip is one line of the same
//! conversation (a question, a request), played by the same parts, so the
//! world after a chip is the world after the typed line that means it.
//! Clicking a chip that costs you is your confirm.

use super::super::Them;
use super::super::ear::{Act, Confirm, Reading, Resolving};
use super::super::knows::Topic;
use super::super::talk::{self, Conversation, Said, store};
use super::super::terms::Request;
use super::super::wants::Kind as WhoKind;
use super::{asks, doctor, rules};
use crate::world::World;
use crate::world::coach::Role;
use crate::world::folk::cast::{self, Role as CastRole};
use crate::world::folk::scene::{self, Kind, Pick, Scene, Who};
use crate::world::folk::{self as folk, Team, talks};

/// `Scene.b` on a sit-down with the compliance director (he is drawn, so
/// no `who` entry names him).
pub const COMPLIANCE: u64 = u64::MAX - 1;

/// `Scene.c` when a sit-down is about no one.
pub const NOBODY: i64 = -1;

/// The chips (codes): a general question, a man's injury and clearing him
/// (one per hurt man, up to `MAX_HURT`), a second opinion, a plan setting,
/// a booster's money, your boss's money, time and backing, the rules, a
/// reference's questions, and the end.
pub const HOW: u32 = 7300;
pub const INJURY: u32 = 7310;
pub const CLEAR: u32 = 7320;
pub const OPINION: u32 = 7330;
pub const PLAN: u32 = 7340;
pub const FUND: u32 = 7350;
pub const MONEY: u32 = 7351;
pub const TIME: u32 = 7352;
pub const BACKING: u32 = 7353;
pub const RULES: u32 = 7360;
pub const REF: u32 = 7370;
pub const END: u32 = 7399;

/// The most hurt men the doctor's chips list. Proposed, not confirmed.
pub const MAX_HURT: usize = 4;

/// A reference's questions, in order: his football, the man, his
/// recruiting, his work, whether he'll stay, the NCAA.
pub const REF_TOPICS: [(Topic, &str); 6] = [
    (Topic::Football, "How is he as a football coach?"),
    (Topic::Character, "What's he like with players?"),
    (Topic::Pitches, "Can he recruit?"),
    (Topic::Effort, "Does he get guys to play hard?"),
    (Topic::Jobs, "Will he stay, or is he looking to move up?"),
    (Topic::Case, "Any trouble with the NCAA?"),
];

/// Whether you can sit down with him: a coach on your staff, someone in
/// your building's cast (your boss, the GM, the team doctor, the academic
/// director, a booster) or the compliance director.
pub fn can_sit(w: &World, who: Them) -> bool {
    let Some(team) = Team::seat(w) else { return false };
    match who {
        Them::Coach(c) => asks::yours(w, c),
        Them::Cast(r, _) => {
            r.team == team
                && matches!(
                    r.role,
                    CastRole::AthleticDirector
                        | CastRole::Principal
                        | CastRole::Owner
                        | CastRole::President
                        | CastRole::Gm
                        | CastRole::Trainer
                        | CastRole::Academic
                        | CastRole::Booster
                )
        }
        Them::Drawn(WhoKind::Compliance, _) => rules::director(w, team) == Some(who),
        _ => false,
    }
}

/// Calls a sit-down with someone in your building, about one of your men
/// or no one. In person, today; each line costs its minutes.
pub fn start(w: &mut World, who: Them, about: Option<u32>) -> Result<u32, String> {
    if !can_sit(w, who) {
        return Err("He isn't someone in your building.".into());
    }
    if talks::hours_left(w) <= 0.0 {
        return Err("Your week is full.".into());
    }
    let name = talk::channels::name(w, who);
    let mut s = Scene::new(Kind::Sitdown, w.today, format!("A sit-down with {name}"));
    s.stops = false;
    s.a = u64::MAX;
    s.c = about.map_or(NOBODY, i64::from);
    match who {
        Them::Coach(c) => {
            s.a = c as u64;
            s.who = vec![Who::Coach(c)];
        }
        Them::Cast(r, _) => s.who = vec![Who::Cast(r.token())],
        Them::Drawn(..) => {
            s.b = COMPLIANCE;
            s.who = vec![Who::Group(name.clone())];
        }
        _ => {}
    }
    // You go to the doctor; anyone else comes to you ("Adrian Johnson
    // comes by the training room", the talks audit of 2026-11-16).
    let doctor = doctor::is_doctor(w, who);
    s.place = if doctor { "The training room".into() } else { "Your office".into() };
    s.narrate(if doctor {
        format!("You stop by the training room to see {name}.")
    } else {
        format!("{name} comes by your office.")
    });
    Ok(scene::add(w, s))
}

/// The man a sit-down is with.
fn him(w: &World, s: &Scene) -> Option<Them> {
    super::across(w, s)
}

/// Your hurt men, by id, for the doctor's chips: the man the talk is about
/// first, then the longest out.
pub fn hurt_men(w: &World, s: &Scene) -> Vec<u32> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    if s.c >= 0 {
        return vec![s.c as u32];
    }
    let mut v: Vec<(u32, u32)> = team
        .roster(w)
        .into_iter()
        .filter_map(|i| {
            let c = doctor::chart(w, i);
            c.hurt().then_some((w.people[i].player.id, c.days))
        })
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.into_iter().take(MAX_HURT).map(|x| x.0).collect()
}

fn player_name(w: &World, id: u32) -> String {
    folk::person(w, id).map_or_else(|| "him".into(), |i| w.people[i].name.clone())
}

/// His side of the ball when he's your coordinator: true offense.
fn coordinator(w: &World, him: Them) -> Option<bool> {
    let Them::Coach(c) = him else { return None };
    match w.coaches[c].job?.role {
        Role::OffensiveCoordinator => Some(true),
        Role::DefensiveCoordinator => Some(false),
        _ => None,
    }
}

/// A plan move as a coordinator hears it, by the staff meeting's index.
fn plan_words(m: u8) -> &'static str {
    super::asks::CHIP_WORDS[(m as usize).min(5)]
}

/// The plan moves on each side.
fn plan_moves(offense: bool) -> &'static [u8] {
    if offense { &[2, 3] } else { &[0, 4, 1, 5] }
}

/// Lays the chips on the table for the man across.
fn chips(w: &World, s: &mut Scene) {
    s.picks.clear();
    let Some(him) = him(w, s) else {
        s.pick(END, "That's all.", "end", "plain", "");
        return;
    };
    let kind = super::super::makeup::kind(w, him);
    if s.kind == Kind::Reference {
        for (k, (_, words)) in REF_TOPICS.iter().enumerate() {
            s.pick(REF + k as u32, *words, "ask", "plain", "");
        }
        s.pick(END, "Thanks for your time.", "end", "plain", "");
        return;
    }
    let how = match kind {
        WhoKind::Boss => "Where do I stand with you?",
        WhoKind::Booster => "How do you feel about where the program is?",
        WhoKind::Compliance => "Anything I should know?",
        _ if doctor::is_doctor(w, him) => "How's the training room?",
        _ => "How's your room?",
    };
    s.pick(HOW, how, "ask", "plain", "");
    if doctor::is_doctor(w, him) {
        for (k, id) in hurt_men(w, s).into_iter().enumerate() {
            let name = player_name(w, id);
            s.pick(INJURY + k as u32, format!("Where is {name} at?"), "ask", "plain", "");
            s.pick(CLEAR + k as u32, format!("I need {name} back. Can you clear him?"), "ask", "firm", "");
            s.pick(OPINION + k as u32, format!("Get {name} a second opinion."), "ask", "plain", "");
        }
    }
    if let Some(off) = coordinator(w, him) {
        for &m in plan_moves(off) {
            s.pick(PLAN + u32::from(m), plan_words(m), "ask", "plain", "");
        }
    }
    if kind == WhoKind::Booster {
        s.pick(FUND, "Could you put more into the collective?", "ask", "plain", "");
    }
    if kind == WhoKind::Boss {
        s.pick(MONEY, "I need more money for the staff.", "ask", "plain", "");
        s.pick(TIME, "I need more time to build this.", "ask", "plain", "");
        if matches!(Team::seat(w), Some(Team::College(_))) {
            s.pick(BACKING, "I need you behind me with the boosters.", "ask", "plain", "");
        }
    }
    if kind == WhoKind::Compliance || matches!(him, Them::Cast(r, _) if r.role == CastRole::AthleticDirector) {
        s.pick(RULES, "What's allowed right now?", "ask", "plain", "");
    }
    s.pick(END, "That's all.", "end", "plain", "");
}

/// The reading a chip stands for: one question or one request.
fn reading(w: &World, s: &Scene, code: u32, label: &str) -> Option<Reading> {
    let him = him(w, s)?;
    let q = |topic: Topic, about: Option<Them>| Act::Question { topic, about, words: label.to_string() };
    let hurt = |k: u32| hurt_men(w, s).get(k as usize).copied();
    let act = match code {
        HOW => match super::super::makeup::kind(w, him) {
            WhoKind::Boss => q(Topic::Goals, None),
            WhoKind::Booster => q(Topic::Money, None),
            WhoKind::Compliance => q(Topic::Rules, None),
            _ => q(Topic::Mood, None),
        },
        x if (INJURY..INJURY + MAX_HURT as u32).contains(&x) => q(Topic::Injury, Some(Them::Player(hurt(x - INJURY)?))),
        x if (CLEAR..CLEAR + MAX_HURT as u32).contains(&x) => Act::Request(Request::Clear(hurt(x - CLEAR)?)),
        x if (OPINION..OPINION + MAX_HURT as u32).contains(&x) => {
            Act::Request(Request::SecondOpinion(hurt(x - OPINION)?))
        }
        x if (PLAN..PLAN + 6).contains(&x) => Act::Request(Request::Plan(plan_words((x - PLAN) as u8).into())),
        FUND => Act::Request(Request::Fund),
        MONEY => Act::Request(Request::Money("the staff pool".into())),
        TIME => Act::Request(Request::Time),
        BACKING => Act::Request(Request::Backing),
        RULES => Act::Request(Request::RulesQuestion),
        x if (REF..REF + REF_TOPICS.len() as u32).contains(&x) => {
            let c = s.a as usize;
            if c >= w.coaches.len() {
                return None;
            }
            q(REF_TOPICS[(x - REF) as usize].0, Some(Them::Coach(c)))
        }
        _ => return None,
    };
    Some(Reading { acts: vec![act], ..Reading::default() })
}

/// The scene's open: his greeting and the chips.
pub fn open(w: &mut World, s: &mut Scene) {
    chips(w, s);
}

/// A chip: one line of the conversation, played and made real. A request
/// that costs you runs at once (the click is your confirm).
pub fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let id = s.id;
    if code == END {
        s.outcome.push("You wrap it up.".into());
        s.end();
        store::close(w, id);
        return;
    }
    let Some(reading) = reading(w, s, code, &picked.label) else {
        s.narrate("That isn't on the table any more.");
        chips(w, s);
        return;
    };
    // The conversation reads and writes the scene in the world: put this
    // one there, play the line, then take it back.
    put(w, s);
    let Some(room) = store::room(w, id).or_else(|| {
        let r = super::room(w, s)?;
        store::begin(w, id, r.clone());
        Some(r)
    }) else {
        s.narrate("There's no one here to talk to.");
        s.end();
        return;
    };
    let him = room.present[0].who;
    let conv = Conversation {
        parts: super::parts(),
        said: Said { scene: id, words: picked.label.clone(), reading, promises: Vec::new() },
    };
    let t = conv.turn(w, &room, &conv.said.reading);
    conv.apply(w, &t);
    let mut lines: Vec<String> = t.say.released.iter().map(|x| folk::plain(x)).collect();
    lines.extend(super::after(w, id, &room, &conv.said.reading, &t));
    for c in &t.confirm {
        match super::confirm(w, id, him, c) {
            Some(done) => lines.extend(done),
            None => {
                if let Confirm::Request(r) = c {
                    lines.extend(talk::Parts::today().acting.run(w, him, r).done);
                }
            }
        }
    }
    if let Some(x) = w.folk.scenes.iter().find(|x| x.id == id) {
        *s = x.clone();
    }
    for l in lines {
        s.narrate(l);
    }
    if s.done {
        return;
    }
    chips(w, s);
}

/// Writes a scene back to the world's list.
fn put(w: &mut World, s: &Scene) {
    if let Some(slot) = w.folk.scenes.iter_mut().find(|x| x.id == s.id) {
        *slot = s.clone();
    }
}

/// His name for a sit-down's who list (the drawn compliance director's).
pub fn name(w: &World, who: Them) -> Option<String> {
    match who {
        Them::Drawn(WhoKind::Compliance, _) => rules::name(w, who),
        Them::Cast(r, _) => Some(cast::get(w, r).name),
        _ => None,
    }
}

/// The people a line in one of this lane's talks may name beyond the room,
/// for the ear's card: the coach a reference or an interview is about, the
/// hurt men the doctor would talk about, the man a sit-down is about.
pub fn named(w: &World, s: &Scene) -> Vec<(Them, &'static str)> {
    let mut v: Vec<(Them, &'static str)> = Vec::new();
    if matches!(s.kind, Kind::Reference) && (s.a as usize) < w.coaches.len() {
        v.push((Them::Coach(s.a as usize), "the coach you're asking about"));
    }
    if s.kind == Kind::Sitdown && s.c >= 0 {
        v.push((Them::Player(s.c as u32), "the player this is about"));
    }
    if let Some(him) = him(w, s)
        && doctor::is_doctor(w, him)
    {
        for id in hurt_men(w, s) {
            v.push((Them::Player(id), "hurt, on your roster"));
        }
    }
    v
}
