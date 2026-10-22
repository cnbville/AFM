//! Press conferences and the reporters (people topic 9; final build lane 6).
//! Your availabilities follow each level's habits: the introductory press
//! conference, a Monday availability and a postgame podium in college and
//! the NFL (the NFL's media policy: the head coach talks, and a coach who
//! skips is fined), the local paper's call in high school. Reporters ask
//! from the facts of the week: last game, the quarterback battle, injuries,
//! the rival, the hot seat, a player in trouble. You answer with the moves
//! and tones of a talk: deflect, praise, take the blame, criticize a player
//! (public words count double), fire back, give a little or a lot about an
//! injury, or hand the opponent bulletin-board material. Reporters you treat
//! well get more and treat you softer; ones you freeze out dig, and a
//! reporter with sources inside the building prints what he hears.
//!
//! Sizes are Claude's picks (proposed, not confirmed).

use super::cast::{self, Cast, Role};
use super::scene::{self, Kind, Pick, Scene, Who};
use super::talks::{self, PromiseKind, PromiseTo, Reason};
use super::voice::{Facts, Speaker};
use super::{Team, facts, player_link};
use crate::calendar::{Date, Level, Loudness, Weekday};
use crate::player::{PlayingTime, Trait};
use crate::world::World;

/// The questions a press conference can bring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Q {
    Vision = 1,
    OldCoach,
    Battle,
    LastGame,
    Injury,
    Rival,
    HotSeat,
    Trouble,
    Recruiting,
    Player,
}

impl Q {
    fn from(v: u8) -> Option<Q> {
        Some(match v {
            1 => Q::Vision,
            2 => Q::OldCoach,
            3 => Q::Battle,
            4 => Q::LastGame,
            5 => Q::Injury,
            6 => Q::Rival,
            7 => Q::HotSeat,
            8 => Q::Trouble,
            9 => Q::Recruiting,
            10 => Q::Player,
            _ => return None,
        })
    }
    fn key(self) -> &'static str {
        match self {
            Q::Vision => "vision",
            Q::OldCoach => "oldcoach",
            Q::Battle => "battle",
            Q::LastGame => "lastgame",
            Q::Injury => "injury",
            Q::Rival => "rival",
            Q::HotSeat => "hotseat",
            Q::Trouble => "trouble",
            Q::Recruiting => "recruiting",
            Q::Player => "player",
        }
    }
}

/// Packs up to four questions into a scene's `a`.
fn pack(qs: &[Q]) -> u64 {
    qs.iter().take(4).enumerate().fold(0u64, |a, (k, q)| a | (*q as u64) << (8 * k))
}

fn question(s: &Scene, k: u8) -> Option<Q> {
    Q::from(((s.a >> (8 * k as u64)) & 0xFF) as u8)
}

/// The reporters on the beat, best known first.
fn reporters(w: &World, team: Team) -> Vec<Cast> {
    cast::all(w, team, Role::Reporter)
}

/// A key player for the questions: the quarterback, or the best starter.
fn key_player(w: &World, team: Team) -> Option<usize> {
    let r = team.roster(w);
    r.iter()
        .copied()
        .filter(|&i| {
            w.people[i].last_role == Some(PlayingTime::Starter)
                && w.people[i].player.position == crate::positions::Position::QB
        })
        .max_by(|&a, &b| w.people[a].player.overall().total_cmp(&w.people[b].player.overall()))
        .or_else(|| {
            r.iter()
                .copied()
                .max_by(|&a, &b| super::takeover::standing(w, a).total_cmp(&super::takeover::standing(w, b)))
        })
}

/// A quarterback battle: two at the top within a couple of points.
fn battle(w: &World, team: Team) -> Option<(usize, usize)> {
    let mut qbs: Vec<usize> =
        team.roster(w).into_iter().filter(|&i| w.people[i].player.position == crate::positions::Position::QB).collect();
    qbs.sort_by(|&a, &b| w.people[b].player.overall().total_cmp(&w.people[a].player.overall()));
    match qbs.as_slice() {
        [a, b, ..] if (w.people[*a].player.overall() - w.people[*b].player.overall()).abs() < 2.5 => Some((*a, *b)),
        _ => None,
    }
}

/// What the reporters will ask this time, from the week's facts.
fn questions(w: &World, team: Team, occasion: i64) -> (Vec<Q>, u64) {
    let mut v = Vec::new();
    let mut subject = 0u64;
    if occasion == 0 {
        v.push(Q::Vision);
        if w.folk.takeover.as_ref().is_some_and(|t| t.old_coach.is_some()) {
            v.push(Q::OldCoach);
        }
        if battle(w, team).is_some() {
            v.push(Q::Battle);
        } else if team.level() == Level::College {
            v.push(Q::Recruiting);
        }
        return (v, subject);
    }
    if facts::last_game(w, team).is_some() {
        v.push(Q::LastGame);
    }
    let ids: Vec<u32> = team.roster(w).iter().map(|&i| w.people[i].player.id).collect();
    if let Some(c) = w.folk.courts.open.iter().find(|c| ids.contains(&c.id) && c.opened.days_to(w.today) < 21) {
        v.push(Q::Trouble);
        subject = c.id as u64;
    }
    let (wn, l) = facts::record_now(w, team);
    if l >= wn + 2 && l >= 3 {
        v.push(Q::HotSeat);
    }
    let next = facts::team_games(w, team).into_iter().find(|g| g.0 >= w.today);
    if let (Some(n), Some((r, ..))) = (next, facts::rival(w, team))
        && n.1 == r
    {
        v.push(Q::Rival);
    }
    if battle(w, team).is_some() && v.len() < 3 {
        v.push(Q::Battle);
    }
    let hurt = team.roster(w).into_iter().find(|&i| {
        w.people[i].last_role == Some(PlayingTime::Starter) && w.people[i].pro.as_ref().is_some_and(|c| c.out_weeks > 0)
    });
    if let Some(i) = hurt
        && v.len() < 3
    {
        v.push(Q::Injury);
        if subject == 0 {
            subject = w.people[i].player.id as u64;
        }
    }
    if v.len() < 3
        && let Some(i) = key_player(w, team)
    {
        v.push(Q::Player);
        if subject == 0 {
            subject = w.people[i].player.id as u64;
        }
    }
    v.truncate(3);
    (v, subject)
}

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let occasion = if s.kind == Kind::PressIntro { 0 } else { s.c };
    let (qs, subject) = questions(w, team, occasion);
    s.a = pack(&qs);
    if s.b == 0 {
        s.b = subject;
    }
    s.c = occasion;
    let rs = reporters(w, team);
    for r in rs.iter().take(3) {
        s.who.push(Who::Cast(r.r.token()));
    }
    let room = match (team.level(), occasion) {
        (Level::HighSchool, _) => "The reporter from the local paper has a notebook and twenty minutes.".to_string(),
        (_, 0) => format!(
            "The room is full: the beat writers, the TV crews, the boosters along the back wall{}.",
            if team.level() == Level::College { ", the band's fight song still echoing from the lobby" } else { "" }
        ),
        (_, 2) => "Fifteen minutes after the final whistle. The cameras are already on.".to_string(),
        _ => "The Monday availability: the regulars, coffee, the same seats as always.".to_string(),
    };
    s.narrate(room);
    if qs.is_empty() {
        s.narrate("Nobody has much to ask. You take a couple of soft ones and it's over.");
        s.end();
        return;
    }
    ask_next(w, s, team, 0);
}

/// The reporter who asks question k.
fn asker(w: &World, team: Team, k: u8) -> Option<Cast> {
    let rs = reporters(w, team);
    if rs.is_empty() {
        return None;
    }
    Some(rs[k as usize % rs.len()].clone())
}

fn ask_next(w: &mut World, s: &mut Scene, team: Team, k: u8) {
    let Some(q) = question(s, k) else {
        wrap(w, s, team);
        return;
    };
    let r = asker(w, team, k);
    let mut f = super::takeover::base_facts(w, team);
    let sub = (s.b != 0).then(|| super::person(w, s.b as u32)).flatten();
    if let Some(i) = sub {
        f.set("player", w.people[i].name.clone());
    }
    if let Some((won, us, them, opp)) = facts::last_game(w, team) {
        // Said the way a score is said: the winner's number first.
        f.set("score", format!("{}-{}", us.max(them), us.min(them)));
        f.set("opp", opp.name(w));
        f.set("result", if won { "win" } else { "loss" });
    }
    let (wn, l) = facts::record_now(w, team);
    f.set("record", format!("{wn}-{l}"));
    let text = {
        let spk = r.as_ref().map_or_else(Speaker::default, |c| Speaker::cast(w, c));
        let mut pen = scene::pen(w, s, 10 + k as u64);
        let line = pen.line(&[&format!("press.q.{}", q.key())], &spk, &f);
        pen.put(w);
        if line.is_empty() { q_fallback(w, q, &f, team) } else { line }
    };
    let name = r.as_ref().map_or_else(
        || "A reporter".to_string(),
        |c| match &c.reporter {
            Some(info) => format!("{}, {}", c.name, info.outlet),
            None => c.name.clone(),
        },
    );
    s.them(
        &name,
        text,
        r.as_ref().and_then(|c| c.reporter.as_ref()).map(|info| {
            if info.lean < -0.3 {
                "He's been hard on you in print.".to_string()
            } else if info.leaks {
                "He knows more than he's asking.".to_string()
            } else {
                "A fair question.".to_string()
            }
        }),
    );
    let base = 100 * (k as u32 + 1);
    match q {
        Q::Vision => {
            s.pick(
                base,
                "\"We're here to win championships. Not someday. Now.\"",
                "Promise",
                "fiery",
                "The room loves it. The bar you'll be judged by goes up.",
            );
            s.pick(
                base + 1,
                "\"It's a process. We'll build it the right way.\"",
                "Tell him straight",
                "calm",
                "Lowers what people expect of your first year.",
            );
            s.pick(
                base + 2,
                "\"I want a team this town is proud of, on and off the field.\"",
                "Praise",
                "warm",
                "Safe and warm; your boss likes it.",
            );
        }
        Q::OldCoach => {
            s.pick(
                base,
                "\"He built a lot here. I'm grateful for what he left.\"",
                "Praise",
                "warm",
                "His people warm to you.",
            );
            s.pick(
                base + 1,
                "\"That's the past. I'm here to talk about what's next.\"",
                "Deflect",
                "calm",
                "Clean break; his loyalists notice.",
            );
        }
        Q::Battle => {
            if let Some((a, b)) = battle(w, team) {
                s.b = w.people[a].player.id as u64;
                s.pick(
                    base,
                    format!("\"{} is our starter.\"", w.people[a].name),
                    "Promise",
                    "firm",
                    "A public promise: break it and it costs double.",
                );
                s.pick(
                    base + 1,
                    format!("\"It's open. {} and {} will compete.\"", w.people[a].name, w.people[b].name),
                    "Challenge",
                    "calm",
                    "Competitors lift; nobody's promised anything.",
                );
                s.pick(
                    base + 2,
                    "\"I'm not going to get into the depth chart.\"",
                    "Deflect",
                    "calm",
                    "Gives the writers nothing.",
                );
            }
        }
        Q::LastGame => {
            let won = facts::last_game(w, team).is_some_and(|x| x.0);
            if won {
                s.pick(
                    base,
                    "\"That's on the players. They earned it.\"",
                    "Praise",
                    "warm",
                    "The room hears you give them the credit.",
                );
                s.pick(
                    base + 1,
                    "\"We left a lot out there. We can play better.\"",
                    "Challenge",
                    "firm",
                    "Keeps them hungry; some feel the win is never enough.",
                );
            } else {
                s.pick(
                    base,
                    "\"That's on me. I have to coach better.\"",
                    "Take the blame",
                    "calm",
                    "Players respect it; your boss reads it either way.",
                );
                s.pick(base + 1, "\"We didn't execute. Simple as that.\"", "Deflect", "calm", "Neutral.");
                if let Some(i) = sub {
                    s.pick(
                        base + 2,
                        format!("\"{} has to play better. Period.\"", w.people[i].name),
                        "Criticize",
                        "fiery",
                        "Public criticism counts double with him and the room.",
                    );
                }
            }
        }
        Q::Injury => {
            s.pick(
                base,
                "\"It's a high ankle. He'll miss a few weeks.\"",
                "Tell him straight",
                "calm",
                "Honest: the opponent knows too.",
            );
            s.pick(
                base + 1,
                "\"Day to day.\"",
                "Deflect",
                "calm",
                if team.level() == Level::Nfl {
                    "The NFL fines clubs for injury reports that don't hold."
                } else {
                    "Gives nothing away."
                },
            );
        }
        Q::Rival => {
            s.pick(
                base,
                "\"We're going to go down there and beat them.\"",
                "Promise",
                "fiery",
                "Bulletin-board material. Your fans love it; their locker room will too.",
            );
            s.pick(
                base + 1,
                "\"They're a good football team. It'll be a great game.\"",
                "Praise",
                "calm",
                "Nothing for their locker room wall.",
            );
        }
        Q::HotSeat => {
            s.pick(
                base,
                "\"I'm not worried about my job. I'm worried about Saturday.\"",
                "Deflect",
                "calm",
                "The standard answer; nobody's convinced.",
            );
            s.pick(
                base + 1,
                "\"The people writing that have never coached a down.\"",
                "Fire back",
                "fiery",
                "Your players like the fight. The writers and your boss don't.",
            );
            s.pick(
                base + 2,
                "\"The record is the record. I own it.\"",
                "Take the blame",
                "firm",
                "Honest; your boss respects it more than the writers do.",
            );
        }
        Q::Trouble => {
            s.pick(
                base,
                "\"We're aware of it. It's being handled internally.\"",
                "Deflect",
                "calm",
                "The standard line.",
            );
            s.pick(
                base + 1,
                "\"He's suspended indefinitely. We hold our players to a standard.\"",
                "Warn",
                "firm",
                "Firm in public; he hears it from the TV first.",
            );
            s.pick(
                base + 2,
                "\"He's a good kid. Let the process play out.\"",
                "Praise",
                "warm",
                "Loyal to him; the boss would rather you said less.",
            );
        }
        Q::Recruiting => {
            s.pick(
                base,
                "\"We're going to build a fence around this state.\"",
                "Promise",
                "fiery",
                "High school coaches in the state like hearing it.",
            );
            s.pick(
                base + 1,
                "\"We'll recruit everywhere. Best players we can find.\"",
                "Tell him straight",
                "calm",
                "Neutral.",
            );
        }
        Q::Player => {
            if let Some(i) = sub {
                s.pick(
                    base,
                    format!("\"{} is one of the best players I've coached.\"", w.people[i].name),
                    "Praise",
                    "warm",
                    "Public praise counts double; an ego grows.",
                );
                s.pick(
                    base + 1,
                    format!("\"{} knows what he has to do better.\"", w.people[i].name),
                    "Challenge",
                    "firm",
                    "A public challenge, counted double.",
                );
                s.pick(base + 2, "\"I'm not going to single anyone out.\"", "Deflect", "calm", "Nothing either way.");
            } else {
                s.pick(base, "\"It's a team game.\"", "Deflect", "calm", "Nothing either way.");
            }
        }
    }
}

fn q_fallback(w: &World, q: Q, f: &Facts, team: Team) -> String {
    let get = |k: &str| f.get(k).unwrap_or("").to_string();
    match q {
        Q::Vision => format!("Coach, what should {} fans expect in year one?", get("team")),
        Q::OldCoach => format!("How do you follow what {} built here?", get("old")),
        Q::Battle => "Have you decided on a starting quarterback?".into(),
        Q::LastGame => format!("Walk us through the {} against {}.", get("result"), get("opp")),
        Q::Injury => format!("What's the latest on {}?", get("player")),
        Q::Rival => format!("It's {} week. What does this game mean to you?", get("rival")),
        Q::HotSeat => format!("You're {}. Are you worried about your job?", get("record")),
        Q::Trouble => format!("Can you address the situation with {}?", get("player")),
        Q::Recruiting => format!(
            "How do you plan to recruit the state of {}?",
            team.state(w).and_then(|s| w.states.get(s)).map_or("this", |s| super::facts::state_name(&s.code))
        ),
        Q::Player => format!("What have you seen from {} this week?", get("player")),
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let k = (code / 100).saturating_sub(1) as u8;
    let opt = code % 100;
    let Some(q) = question(s, k) else {
        wrap(w, s, team);
        return;
    };
    s.you(&scene::your_name(w), picked.label.clone());
    let sub = (s.b != 0).then(|| super::person(w, s.b as u32)).flatten();
    let deflect = picked.mv == "Deflect";
    // Reporters warm to answers with substance and cool on deflections.
    for r in reporters(w, team) {
        w.folk.mine.nudge(r.key(), if deflect { -0.015 } else { 0.01 });
    }
    let boss = super::boss::boss(w, team);
    match (q, opt) {
        (Q::Vision, 0) => {
            w.folk.mine.expectation = (w.folk.mine.expectation + 0.3).min(1.0);
            s.outcome.push("You promised championships, in public.".into());
        }
        (Q::Vision, 1) => {
            w.folk.mine.expectation = (w.folk.mine.expectation - 0.15).max(-1.0);
            w.folk.mine.straight = (w.folk.mine.straight + 0.05).min(1.0);
        }
        (Q::Vision, _) => w.folk.mine.nudge(boss.key(), 0.03),
        (Q::OldCoach, 0) => {
            let old = w.folk.takeover.as_ref().and_then(|t| t.old_coach);
            for i in team.roster(w) {
                if super::takeover::tie_to_old(w, i, old) > 0.4 {
                    w.people[i].trust = (w.people[i].trust + 0.03).min(1.0);
                }
            }
        }
        (Q::Battle, 0) => {
            if let Some(i) = sub {
                let id = w.people[i].player.id;
                talks::promise(w, PromiseTo::Player(id), PromiseKind::Start, 0);
                if let Some(p) = w.folk.promises.last_mut() {
                    p.public = true;
                }
                talks::mood_event(w, id, Reason::Praise, 0.15);
                if let Some((_, b)) = battle(w, team) {
                    let other = w.people[b].player.id;
                    talks::mood_event(w, other, Reason::Benched, -0.15);
                }
                s.outcome.push(format!("You named {} the starter in public.", w.people[i].name));
            }
        }
        (Q::Battle, 1) => {
            if let Some((a, b)) = battle(w, team) {
                for x in [a, b] {
                    let comp = (w.people[x].player.personality.competitiveness - 10.0) / 10.0;
                    let id = w.people[x].player.id;
                    talks::mood_event(w, id, Reason::Noise, 0.04 * comp as f32);
                }
            }
        }
        (Q::LastGame, 0) => {
            for i in team.roster(w) {
                w.people[i].trust = (w.people[i].trust + 0.01).min(1.0);
            }
            if facts::last_game(w, team).is_some_and(|x| !x.0) {
                w.folk.mine.nudge(boss.key(), if boss.honesty >= 12.0 { 0.02 } else { -0.01 });
            }
        }
        (Q::LastGame, 2) | (Q::Player, 1) => {
            if let Some(i) = sub {
                let id = w.people[i].player.id;
                let ego = w.people[i].player.personality.traits.z(Trait::Ego) as f32;
                let comp = ((w.people[i].player.personality.competitiveness - 10.0) / 10.0) as f32;
                let crit = q == Q::LastGame;
                let d = if crit { -0.24 - 0.1 * ego.max(0.0) } else { -0.04 + 0.12 * comp - 0.08 * ego.max(0.0) };
                w.people[i].trust = (w.people[i].trust + d * 0.6).clamp(0.0, 1.0);
                talks::mood_event(w, id, Reason::Noise, d);
                if crit {
                    for j in team.roster(w) {
                        if j != i {
                            w.people[j].trust = (w.people[j].trust - 0.008).max(0.0);
                        }
                    }
                    s.outcome.push(format!("You criticized {} in public. The room heard it too.", w.people[i].name));
                }
            }
        }
        (Q::Injury, 1) => {
            if team.level() == Level::Nfl && super::roll(w, 0x1A7, w.today.0 as u64) < 0.08 {
                let text = "The league fined the club $25,000 for an inaccurate injury report.".to_string();
                super::post_mine(w, w.today.add(9), Loudness::Small, false, "league: injury report", &text);
                s.outcome.push(text);
            }
        }
        (Q::Rival, 0) => {
            for i in team.roster(w) {
                let comp = ((w.people[i].player.personality.competitiveness - 10.0) / 10.0) as f32;
                w.people[i].mood = (w.people[i].mood + 0.03 * comp.max(0.0)).min(1.0);
            }
            w.folk.bulletin = facts::rival(w, team).map(|x| (x.0, w.today));
            s.outcome.push("Your words are on the rival's locker room wall by tonight.".into());
        }
        (Q::HotSeat, 1) => {
            for r in reporters(w, team) {
                w.folk.mine.nudge(r.key(), -0.08);
            }
            w.folk.mine.nudge(boss.key(), -0.04);
            for i in team.roster(w) {
                w.people[i].mood = (w.people[i].mood + 0.02).min(1.0);
            }
            s.outcome.push("You fired back at your critics.".into());
        }
        (Q::HotSeat, 2) => {
            w.folk.mine.nudge(boss.key(), 0.02);
            w.folk.mine.straight = (w.folk.mine.straight + 0.04).min(1.0);
        }
        (Q::Trouble, 1) => {
            if let Some(i) = sub {
                let id = w.people[i].player.id;
                w.people[i].trust = (w.people[i].trust - 0.1).max(0.0);
                w.folk.out.retain(|x| x.0 != id);
                let until = facts::team_games(w, team)
                    .iter()
                    .map(|g| g.0)
                    .filter(|&d| d >= w.today)
                    .nth(1)
                    .unwrap_or(w.today.add(14));
                w.folk.out.push((id, until, "suspended indefinitely".into()));
                w.folk.mine.hard = (w.folk.mine.hard + 0.1).min(1.0);
                w.folk.mine.nudge(boss.key(), 0.03);
            }
        }
        (Q::Trouble, 2) => {
            if let Some(i) = sub {
                w.people[i].trust = (w.people[i].trust + 0.08).min(1.0);
                w.folk.mine.nudge(boss.key(), -0.04);
            }
        }
        (Q::Recruiting, 0) => {
            if let Some(st) = team.state(w) {
                w.folk.fence = Some((st, w.today));
            }
        }
        (Q::Player, 0) => {
            if let Some(i) = sub {
                let id = w.people[i].player.id;
                talks::mood_event(w, id, Reason::Praise, 0.16);
                if w.people[i].player.personality.traits.get(Trait::Ego) >= 13.0
                    && super::roll(w, 0xE90, id as u64) < 0.2
                {
                    w.people[i].player.personality.traits.shift(Trait::Ego, 0.5);
                }
            }
        }
        _ => {}
    }
    let next = k + 1;
    if question(s, next).is_some() {
        ask_next(w, s, team, next);
    } else {
        wrap(w, s, team);
    }
}

fn wrap(w: &mut World, s: &mut Scene, team: Team) {
    let comms = cast::has(w, team, Role::Comms).then(|| cast::get(w, cast::CastRef::new(team, Role::Comms, 0)));
    s.narrate(match comms {
        Some(c) => format!("{} calls the last question. The cameras go dark.", c.name),
        None => "That's the last question.".into(),
    });
    if s.outcome.is_empty() {
        s.outcome.push("Nothing that will make a headline.".into());
    }
    s.end();
}

/// Skipped: the communications office reads a statement. The NFL fines a
/// head coach who skips his media obligations (proposed, not confirmed:
/// $25,000 for a first offense, as the league fined coaches in the 2010s);
/// in college and high school the reporters dig instead.
pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else { return };
    for r in reporters(w, team) {
        w.folk.mine.nudge(r.key(), -0.04);
    }
    s.handed = Some("the communications office".into());
    if team.level() == Level::Nfl && s.kind == Kind::Press {
        let text = "The league fined you $25,000 for skipping your media obligations.".to_string();
        super::post_mine(w, w.today.add(3), Loudness::Small, false, "league: media policy", &text);
        s.outcome.push(text);
    } else {
        s.outcome.push(
            "You skipped it. The communications office read a statement, and the writers filled the space themselves."
                .into(),
        );
    }
}

/// Weekly in season: your availabilities, and stories from reporters with
/// sources inside your building.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let Some(since) = w.folk.mine.since else { return };
    if since.days_to(d) < 14 {
        return;
    }
    let level = team.level();
    let games = facts::team_games(w, team);
    let in_season = games.first().is_some_and(|g| d >= g.0.add(-6)) && games.last().is_some_and(|g| d <= g.0.add(1));
    if !in_season {
        return;
    }
    // Postgame: the podium the day of a game (it waits for you).
    if games.iter().any(|g| g.0 == d) && level != Level::HighSchool {
        let mut s = Scene::new(Kind::Press, d, "Postgame press conference");
        s.c = 2;
        s.stops = false;
        s.team = Some(team);
        s.place = "The interview room under the stands".into();
        scene::add(w, s);
    }
    // Monday availability (college and NFL); the local paper's call on
    // Thursday at a high school.
    let weekly = match level {
        Level::HighSchool => d.weekday() == Weekday::Thu,
        _ => d.weekday() == Weekday::Mon,
    };
    if weekly {
        let mut s = Scene::new(
            Kind::Press,
            d,
            if level == Level::HighSchool { "The local paper calls" } else { "Monday press availability" },
        );
        s.c = 1;
        s.stops = false;
        s.team = Some(team);
        s.place = if level == Level::HighSchool { "On the phone".into() } else { "The media room".into() };
        scene::add(w, s);
    }
    // A reporter with sources inside prints what he hears when trust in you
    // runs low in the building (proposed, not confirmed: rare).
    if d.weekday() == Weekday::Wed {
        let leaker = reporters(w, team).into_iter().find(|r| r.reporter.as_ref().is_some_and(|x| x.leaks));
        if let Some(r) = leaker {
            let sour = team.roster(w).into_iter().filter(|&i| w.people[i].trust < 0.3).collect::<Vec<_>>();
            let rt = w.folk.mine.trust_of(r.key()).unwrap_or(0.5);
            if sour.len() >= 4 && rt < 0.5 && super::roll(w, 0x1EA4 ^ d.0 as u64, 1) < 0.35 {
                let who = player_link(w, sour[0]);
                let outlet = r.reporter.as_ref().map_or(String::new(), |x| x.outlet.clone());
                let text = format!(
                    "{} ({outlet}): sources describe growing frustration with the head coach inside the locker room.",
                    r.name
                );
                super::post_mine(w, d, Loudness::Medium, false, "media: report", &text);
                super::note(
                    w,
                    "press",
                    r.link(),
                    format!("Reported frustration in the room; your staff thinks one source is close to {who}."),
                    Some("He prints what his sources give him.".into()),
                );
                let b = super::boss::boss(w, team);
                w.folk.mine.nudge(b.key(), -0.03);
            }
        }
    }
}
