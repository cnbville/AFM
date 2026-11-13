//! Press conferences and the reporters (people topic 9; final build lane 6).
//! Your availabilities follow each level's habits: the introductory press
//! conference, a Monday availability and a postgame podium in college, three
//! availabilities a week (Monday, Wednesday, Friday) and the postgame podium
//! in the NFL (the NFL's media policy: the head coach talks at least three
//! days each practice week and after every game, and a coach who skips is
//! fined), the local paper's call in high school. Reporters ask
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
use super::claims::{self, Point};
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
    /// A story you are in (update 2): its id is in the scene's `f[2]`.
    Story,
    /// A second story: its id in `f[3]`.
    Story2,
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
            11 => Q::Story,
            12 => Q::Story2,
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
            Q::Story | Q::Story2 => "story",
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

/// What the reporters will ask this time, from the week's facts, and the
/// stories you are in that they ask about (update 2).
fn questions(w: &World, team: Team, occasion: i64) -> (Vec<Q>, Vec<u32>) {
    let mut v = week_questions(w, team, occasion);
    if occasion == 0 {
        return (v, Vec::new());
    }
    use crate::world::storylines::{Family, moves};
    let ids = moves::press_stories(w, crate::world::storylines::team_key(team));
    // A story question replaces the week's own question on the same thing.
    for &id in &ids {
        match moves::press_family(w, id) {
            Some(Family::Seat) => v.retain(|q| *q != Q::HotSeat),
            Some(Family::Battle) => v.retain(|q| *q != Q::Battle),
            Some(Family::OffField) => v.retain(|q| *q != Q::Trouble),
            Some(Family::Rivalry) => v.retain(|q| *q != Q::Rival),
            _ => {}
        }
    }
    // The last game first, then the stories, then the rest.
    let at = usize::from(v.first() == Some(&Q::LastGame));
    for (k, _) in ids.iter().enumerate().rev() {
        v.insert(at.min(v.len()), if k == 0 { Q::Story } else { Q::Story2 });
    }
    v.truncate(4);
    let kept: Vec<u32> = ids.into_iter().take(2).collect();
    (v, kept)
}

/// The man in trouble: an open case from the last three weeks.
fn in_trouble(w: &World, team: Team) -> Option<usize> {
    let ids: Vec<u32> = team.roster(w).iter().map(|&i| w.people[i].player.id).collect();
    let c = w.folk.courts.open.iter().find(|c| ids.contains(&c.id) && c.opened.days_to(w.today) < 21)?;
    super::person(w, c.id)
}

/// The talking point behind a trouble question (talk facts item 13): the
/// case he is in, with its charge, the day and the step it has reached
/// (`arrest`, `charged`, `court`), read from the fact sheet. A case the
/// courts are not handling (an appeal, a grievance) has none.
fn case_of(sheet: &claims::Sheet) -> Option<Point> {
    sheet.get(claims::Kind::Arrest).filter(|p| p.get("charge").is_some()).cloned()
}

/// A starter who is out hurt.
fn hurt_starter(w: &World, team: Team) -> Option<usize> {
    team.roster(w).into_iter().find(|&i| {
        w.people[i].last_role == Some(PlayingTime::Starter) && w.people[i].pro.as_ref().is_some_and(|c| c.out_weeks > 0)
    })
}

/// The man a question is about: each question its own, the way reporters
/// ask about each man by name (minor bug 90).
fn subject(w: &World, team: Team, q: Q) -> Option<usize> {
    match q {
        Q::Trouble => in_trouble(w, team),
        Q::Injury => hurt_starter(w, team),
        Q::Battle => battle(w, team).map(|x| x.0),
        Q::LastGame | Q::Player => key_player(w, team),
        _ => None,
    }
}

fn week_questions(w: &World, team: Team, occasion: i64) -> Vec<Q> {
    let mut v = Vec::new();
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
        return v;
    }
    // Only a game from the last week is asked about (talk facts item 30).
    if facts::fresh_game(w, team).is_some() {
        v.push(Q::LastGame);
    }
    if in_trouble(w, team).is_some() {
        v.push(Q::Trouble);
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
    if hurt_starter(w, team).is_some() && v.len() < 3 {
        v.push(Q::Injury);
    }
    if v.len() < 3 && key_player(w, team).is_some() {
        v.push(Q::Player);
    }
    v.truncate(3);
    v
}

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let occasion = if s.kind == Kind::PressIntro { 0 } else { s.c };
    let (qs, stories) = questions(w, team, occasion);
    s.a = pack(&qs);
    for (k, id) in stories.iter().enumerate() {
        s.f[2 + k] = *id as f32;
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
        _ => format!(
            "The {} availability: the regulars, coffee, the same seats as always.",
            match s.date.weekday() {
                Weekday::Wed => "Wednesday",
                Weekday::Fri => "Friday",
                _ => "Monday",
            }
        ),
    };
    s.narrate(room);
    if qs.is_empty() {
        s.narrate("Nobody has much to ask. You take a couple of soft ones and it's over.");
        s.end();
        return;
    }
    ask_next(w, s, team, 0);
}

/// A reporter as the scene names him: his name and his outlet.
fn reporter_name(r: Option<&Cast>) -> String {
    r.map_or_else(
        || "A reporter".to_string(),
        |c| match &c.reporter {
            Some(info) => format!("{}, {}", c.name, info.outlet),
            None => c.name.clone(),
        },
    )
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
    // This question's own man, kept for its answer.
    let sub = subject(w, team, q);
    s.b = sub.map_or(0, |i| w.people[i].player.id as u64);
    if let Some(i) = sub {
        f.set("player", w.people[i].name.clone());
    }
    // A trouble or an injury question reads the man's sheet (talk facts item
    // 13): the charge and the step he has reached, the injury and the weeks.
    let sheet = sub.filter(|_| matches!(q, Q::Trouble | Q::Injury)).map(|i| claims::player(w, i));
    let case = sheet.as_ref().filter(|_| q == Q::Trouble).and_then(case_of);
    if let Some(c) = &case {
        c.fill(&mut f);
    }
    // A line that names the score, the opponent or the result gets them only
    // while the game is fresh; with none, such lines are skipped (talk facts
    // item 30), so no question dates a game from months ago.
    if let Some((won, us, them, opp)) = facts::fresh_game(w, team) {
        // Said the way a score is said: the winner's number first.
        f.set("score", format!("{}-{}", us.max(them), us.min(them)));
        f.set("opp", opp.name(w));
        f.set("result", if won { "win" } else { "loss" });
    }
    let (wn, l) = facts::record_now(w, team);
    f.set("record", format!("{wn}-{l}"));
    let story = story_of(s, q);
    let text = if let Some(id) = story {
        crate::world::storylines::moves::press_question(w, id)
    } else {
        let spk = r.as_ref().map_or_else(Speaker::default, |c| Speaker::cast(w, c));
        let mut pen = scene::pen(w, s, 10 + k as u64);
        // The step he has reached picks the reporter's words; a step with
        // no lines yet falls back to the plain question.
        let mut line = String::new();
        if let Some(step) = case.as_ref().and_then(|c| c.check("step")) {
            line = pen.line(&[&format!("press.q.trouble.{step}")], &spk, &f);
        }
        if line.is_empty() {
            line = pen.line(&[&format!("press.q.{}", q.key())], &spk, &f);
        }
        pen.put(w);
        if line.is_empty() { q_fallback(w, q, &f, team) } else { line }
    };
    let name = reporter_name(r.as_ref());
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
        Q::Story | Q::Story2 => {
            if let Some(id) = story {
                for (j, o) in crate::world::storylines::moves::press_menu(w, id).into_iter().enumerate() {
                    s.pick(base + j as u32, o.label, o.mv.name(), o.mv.tone(), &o.hint);
                }
            }
        }
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
            // The injury and the weeks are his, from the sheet. Without
            // them the answer says only what is known (talk facts item 13).
            let hurt = sheet.as_ref().and_then(|x| x.get(claims::Kind::Hurt));
            let weeks = hurt.and_then(|p| p.get("weeks")).and_then(|x| x.parse::<u32>().ok());
            let injury = hurt.and_then(|p| p.get("injury"));
            s.pick(
                base,
                injury_straight(injury, weeks),
                "Tell him straight",
                "calm",
                "Honest: the opponent knows too.",
            );
            s.pick(
                base + 1,
                injury_deflect(weeks),
                "Deflect",
                "calm",
                if matches!(team, Team::Nfl(_)) {
                    "The NFL fines clubs over injury reports."
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
            // Your answers fit the case (talk facts item 13): what you say
            // about the process fits the step it has reached, a man already
            // out can't be suspended again, and a man who has been here
            // before is not "a good kid" in the same breath.
            let step = case.as_ref().and_then(|c| c.check("step"));
            let charge = case.as_ref().and_then(|c| c.get("charge"));
            let again = case.as_ref().is_some_and(|c| c.is("again", "yes"));
            let out = sheet.as_ref().is_some_and(|x| x.has(claims::Kind::Suspended));
            s.pick(base, trouble_deflect(step), "Deflect", "calm", "The standard line.");
            if !out {
                s.pick(
                    base + 1,
                    trouble_warn(charge),
                    "Warn",
                    "firm",
                    "Firm in public; he hears it from the TV first.",
                );
            }
            s.pick(
                base + 2,
                trouble_praise(again),
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

/// What you tell the press about his injury: the real one and how long
/// he is out, when the sheet has them.
fn injury_straight(injury: Option<&str>, weeks: Option<u32>) -> String {
    let out = weeks.map(|n| format!("He'll miss {}", claims::words::counted(n, "week")));
    match (injury, out) {
        (Some(i), Some(o)) => format!("\"It's {} {i}. {o}.\"", super::a_an(i)),
        (Some(i), None) => format!("\"It's {} {i}.\"", super::a_an(i)),
        (None, Some(o)) => format!("\"{o}.\""),
        (None, None) => "\"He's hurt, and he'll miss some time.\"".into(),
    }
}

/// The non-answer about his injury. "Day to day" is said only of a man
/// who is, since the league fines a report that doesn't hold.
fn injury_deflect(weeks: Option<u32>) -> &'static str {
    if weeks.is_some_and(|n| n <= SHORT_WEEKS) {
        "\"Day to day.\""
    } else {
        "\"We'll let the doctors speak to the timeline.\""
    }
}

/// A man out this many weeks or fewer is day to day. Claude's size,
/// proposed, not confirmed.
const SHORT_WEEKS: u32 = 2;

/// Your standard answer at the step his case has reached.
fn trouble_deflect(step: Option<&str>) -> &'static str {
    match step {
        Some("arrest") => {
            "\"We're aware he was arrested and we're gathering the facts. I won't say more until we have them.\""
        }
        Some("charged") => {
            "\"He's been charged, and he'll have his day in court. I'm going to let that process work.\""
        }
        Some("court") => "\"It's in front of a judge. I'm not going to comment on a case that's in court.\"",
        _ => "\"We're aware of it. It's being handled internally.\"",
    }
}

/// Your firm answer. It names the charge when the sheet has it.
fn trouble_warn(charge: Option<&str>) -> String {
    match charge {
        Some(c) => format!("\"He's suspended indefinitely while {c} plays out. We hold our players to a standard.\""),
        None => "\"He's suspended indefinitely. We hold our players to a standard.\"".into(),
    }
}

/// Your loyal answer. A man in trouble before is not simply "a good kid".
fn trouble_praise(again: bool) -> &'static str {
    if again {
        "\"He's been through this before, and I'm standing by him while the process plays out.\""
    } else {
        "\"He's a good kid. Let the process play out.\""
    }
}

/// The story a story question is about.
fn story_of(s: &Scene, q: Q) -> Option<u32> {
    let id = match q {
        Q::Story => s.f[2],
        Q::Story2 => s.f[3],
        _ => return None,
    };
    (id >= 1.0).then_some(id as u32)
}

fn q_fallback(w: &World, q: Q, f: &Facts, team: Team) -> String {
    let get = |k: &str| f.get(k).unwrap_or("").to_string();
    match q {
        Q::Story | Q::Story2 => "Coach, can you address the story everyone is talking about?".into(),
        Q::Vision => format!("Coach, what should {} fans expect in year one?", get("team")),
        Q::OldCoach => format!("How do you follow what {} built here?", get("old")),
        Q::Battle => "Have you decided on a starting quarterback?".into(),
        Q::LastGame => format!("Walk us through the {} against {}.", get("result"), get("opp")),
        Q::Injury => format!("What's the latest on {}?", get("player")),
        Q::Rival => format!("It's {} week. What does this game mean to you?", get("rival")),
        Q::HotSeat => format!("You're {}. Are you worried about your job?", get("record")),
        Q::Trouble if f.get("charge").is_some() => {
            format!("Can you address {} involving {}?", get("charge"), get("player"))
        }
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
        (Q::Story | Q::Story2, o) => {
            if let Some(id) = story_of(s, q) {
                let out = crate::world::storylines::moves::press_answer(w, id, o);
                s.outcome.extend(out);
            }
        }
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
            if matches!(team, Team::Nfl(_)) && super::roll(w, 0x1A7, w.today.0 as u64) < 0.08 {
                let text = "The league fined the club $25,000 over the injury report.".to_string();
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
    if q == Q::Trouble {
        follow_up(w, s, team, k, opt);
    }
    let next = k + 1;
    if question(s, next).is_some() {
        ask_next(w, s, team, next);
    } else {
        wrap(w, s, team);
    }
}

/// The reporter who asked about the man in trouble does not let it go: he
/// follows up on what you just said, in words that fit the case as it
/// stands now (talk facts item 13). It is a line and no new choice, so the
/// answer you gave stands and nothing else moves.
fn follow_up(w: &mut World, s: &mut Scene, team: Team, k: u8, opt: u32) {
    let key = match opt {
        0 => "deflect",
        1 => "warn",
        2 => "praise",
        _ => return,
    };
    let Some(i) = (s.b != 0).then(|| super::person(w, s.b as u32)).flatten() else { return };
    let mut f = super::takeover::base_facts(w, team);
    f.set("player", w.people[i].name.clone());
    if let Some(c) = case_of(&claims::player(w, i)) {
        c.fill(&mut f);
    }
    let r = asker(w, team, k);
    let spk = r.as_ref().map_or_else(Speaker::default, |c| Speaker::cast(w, c));
    let mut pen = scene::pen(w, s, 40 + k as u64);
    let line = pen.line(&[&format!("press.follow.trouble.{key}")], &spk, &f);
    pen.put(w);
    if !line.is_empty() {
        s.them(&reporter_name(r.as_ref()), line, None);
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
    // The NFL's media policy and its fines; another league has neither
    // (minor bug B40).
    if matches!(team, Team::Nfl(_)) && s.kind == Kind::Press {
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
/// A program's weekly press day (proposed, not confirmed: Monday for two
/// programs in five, Tuesday for two, Wednesday for one).
fn press_day(w: &World, team: Team) -> Weekday {
    let k = match team {
        Team::College(p) | Team::Nfl(p) | Team::High(p) | Team::Other(p) => p as u64,
    };
    let x = crate::world::season::key_uniform(w.options.seed as u64 ^ 0x9E55, k);
    if x < 0.4 {
        Weekday::Mon
    } else if x < 0.8 {
        Weekday::Tue
    } else {
        Weekday::Wed
    }
}

/// A press scene's hour, local (proposed, not confirmed): the weekly one at
/// the program's own hour (11 AM, noon or 1 PM), the NFL's at noon, the
/// local paper's call after practice (6:45 PM), the postgame podium a
/// quarter hour after the final whistle.
pub(super) fn press_hour(w: &World, s: &Scene) -> Option<u16> {
    let team = s.team?;
    if s.c == 2 {
        let (_, whistle) = w.seat_game_window(s.date)?;
        return Some(w.local(whistle.add_minutes(15)).minute);
    }
    Some(match team {
        Team::High(_) => 18 * 60 + 45,
        Team::College(p) => [11 * 60, 12 * 60, 13 * 60]
            [(crate::world::season::key_uniform(w.options.seed as u64 ^ 0x9E56, p as u64) * 3.0) as usize % 3],
        _ => 12 * 60,
    })
}

pub(crate) fn day(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let Some(since) = w.folk.mine.since else { return };
    if since.days_to(d) < 14 {
        return;
    }
    let level = team.level();
    let games = facts::team_games(w, team);
    // The NFL's policy runs from training camp (veterans report).
    let nfl = matches!(team, Team::Nfl(_));
    let camp = nfl.then_some(w.dates.veterans_report);
    let in_season = games.first().is_some_and(|g| d >= camp.map_or(g.0.add(-6), |c| c.min(g.0.add(-6))))
        && games.last().is_some_and(|g| d <= g.0.add(1));
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
    // The NFL's media policy asks the head coach for at least three days
    // in each practice week, camp through the season (minor bug B39): the
    // Monday after the game, then Wednesday and Friday, the practice days
    // (a game on one of them moves nothing; that day's podium is enough).
    // College and the other leagues keep a Monday availability; the local
    // paper calls a high school coach on Thursday.
    let weekly = match level {
        Level::HighSchool => d.weekday() == Weekday::Thu,
        _ if nfl => {
            matches!(d.weekday(), Weekday::Mon | Weekday::Wed | Weekday::Fri) && !games.iter().any(|g| g.0 == d)
        }
        // Each program holds its weekly press on its own day (the six-part
        // day, topic 8: South Carolina's is Tuesday at 1 PM).
        Level::College => d.weekday() == press_day(w, team),
        _ => d.weekday() == Weekday::Mon,
    };
    if weekly {
        let title = match (level, d.weekday()) {
            (Level::HighSchool, _) => "The local paper calls",
            (_, Weekday::Tue) => "Tuesday press conference",
            (_, Weekday::Wed) => "Wednesday press availability",
            (_, Weekday::Fri) => "Friday press availability",
            _ => "Monday press availability",
        };
        let mut s = Scene::new(Kind::Press, d, title);
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
