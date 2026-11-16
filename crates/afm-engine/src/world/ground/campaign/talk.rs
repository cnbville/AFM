//! Your AD's talk the day after signing day (GR35): his reaction to the
//! class you just signed, then the one or two goals he holds you to, each
//! from his real motives (`motives`). A boss meeting (`folk::boss`,
//! `Scene.c` 4), so with a key it plays as a live talks v3 talk with him
//! as himself (lane N5c's office; your typed line lands on these same
//! options), and with no key on chips: take it as it is, argue it down,
//! take it bigger for help, swap it for another of his motives, or ask him
//! why. The game decides every answer, from his patience, his trust in you
//! and his money (the built boss meeting's pattern). Sizes are the build's
//! picks (proposed, not confirmed).

use super::goals::{self, Aim};
use super::motives::{self, Motive};
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::Team;
use crate::world::folk::boss::{self, AskKind};
use crate::world::folk::scene::{self, Doc, DocRow, Kind, Pick, Scene, Who};

/// The talk's scene code (`Scene.c`) in the boss meeting.
pub const CODE: i64 = 4;

/// Puts the talk on the calendar for the day after signing day, 9 AM.
pub(crate) fn schedule(w: &mut World, prog: usize, when: Date) {
    if w.folk.scenes.iter().any(|s| s.kind == Kind::BossMeeting && s.c == CODE && s.date == when) {
        return;
    }
    let mut s = Scene::new(Kind::BossMeeting, when, "The class, and what's next");
    s.c = CODE;
    s.team = Some(Team::College(prog));
    s.stops = true;
    s.place = "The athletic director's office".into();
    s.minute = Some(9 * 60);
    scene::add(w, s);
    w.ground.campaign.talked = Some(when);
}

/// The motives on the table: the ones he holds you to first.
fn table(w: &World, prog: usize, class: i32) -> (Vec<Motive>, Vec<Motive>) {
    let ms = motives::motives(w, prog, class);
    let k = motives::held(w, prog);
    let held: Vec<Motive> = ms.iter().take(k).cloned().collect();
    let rest: Vec<Motive> = ms.into_iter().skip(k).take(5).collect();
    (held, rest)
}

pub(crate) fn open(w: &mut World, s: &mut Scene, team: Team) {
    let Team::College(prog) = team else {
        s.end();
        return;
    };
    let b = boss::boss(w, team);
    s.who.push(Who::Cast(b.r.token()));
    // The class that just signed is the one before the next.
    let class = super::class_now(w);
    let (held, rest) = table(w, prog, class);
    w.ground.campaign.draft = held.clone();
    w.ground.campaign.wishes = rest.clone();
    let signed = goals::class_of(w, prog, class - 1).len();
    let rank = boss::class_rank(w, prog, class - 1);
    let bar = motives::conference_bar(w, prog);
    let pleased = rank.is_some_and(|r| r <= bar);
    s.narrate(format!(
        "{} has the class on {} desk: {signed} signed{}.",
        b.link(),
        b.his(),
        rank.map_or(String::new(), |r| format!(", {} in the conference", ordinal(r)))
    ));
    s.them(
        &b.name,
        if pleased { "Good class. People noticed." } else { "We'll talk about that class another day." },
        Some(boss::trust_words(boss::trust(w, team), b.she)),
    );
    let mut doc = Doc {
        title: format!("What {} wants from the class of {class}", b.first()),
        note: "He holds you to the first; the rest are on his mind.".into(),
        cols: vec!["Goal".into(), "Why".into(), "Whose".into()],
        rows: Vec::new(),
    };
    for (k, m) in held.iter().chain(rest.iter()).enumerate() {
        doc.rows.push(DocRow {
            pid: None,
            cells: vec![
                format!("{}{}", goals::words(w, &m.aim), if k < held.len() { "" } else { " (a wish)" }),
                m.why.clone(),
                m.who.clone(),
            ],
        });
    }
    s.docs.push(doc);
    s.a = 0;
    ask_about(w, s, &b.name);
}

/// He brings up the goal under discussion and the five ways to answer.
fn ask_about(w: &World, s: &mut Scene, name: &str) {
    let k = s.a as usize;
    let Some(m) = w.ground.campaign.draft.get(k).cloned() else {
        s.end();
        return;
    };
    s.them(name, m.why.clone(), None);
    s.picks.clear();
    let goal = goals::words(w, &m.aim);
    s.pick(60, format!("\"{goal}. You'll have it.\""), "Agree", "calm", "Taken as he set it.");
    s.pick(
        61,
        format!(
            "\"{}\"",
            lower_words(w, &m.aim).map_or("I can't promise that one.".into(), |l| format!("{l} is the honest bar."))
        ),
        "Argue it down",
        "calm",
        "A patient AD who trusts you lowers it; another remembers that you asked.",
    );
    s.pick(
        62,
        format!("\"Make it {}. Get me the money to chase it.\"", higher_words(w, &m.aim)),
        "Take it bigger",
        "fiery",
        "A bigger goal, and he puts in for recruiting money; you'll be judged by it.",
    );
    if !w.ground.campaign.wishes.is_empty() {
        let alt = goals::words(w, &w.ground.campaign.wishes[0].aim);
        s.pick(
            63,
            format!("\"Hold me to {} instead.\"", lower_first(&alt)),
            "Swap",
            "calm",
            "Another of his own motives, if he lets this one go.",
        );
    }
    s.pick(64, "\"Why that one?\"", "Ask", "calm", "He tells you who's behind it.");
}

/// A lower bar: a looser rank, one fewer, the fence for a named kid.
fn lower(a: &Aim) -> Option<Aim> {
    let d = a.def();
    match d.id {
        1 | 2 | 7 => Some(Aim { n: a.n + (a.n / 2).max(2), ..*a }),
        5 | 9 | 14 | 15 | 16 | 17 | 26 | 37 | 38 | 41 | 48 | 49 | 57 | 58 | 62 if a.n > 1 => {
            Some(Aim { n: a.n - 1, ..*a })
        }
        11 | 22 | 23 | 24 | 25 | 55 | 56 => Some(Aim { goal: 14, n: 4, of: 0, ..*a }),
        12 | 13 => Some(Aim { goal: 11, ..*a }),
        50 => Some(Aim { n: a.n / 2, ..*a }),
        _ => None,
    }
}

/// A higher bar: a tighter rank, one more, the named kid for two years.
fn higher(a: &Aim) -> Aim {
    let d = a.def();
    match d.id {
        1 | 2 | 7 => Aim { n: (a.n * 2 / 3).max(1), ..*a },
        11 => Aim { goal: 12, ..*a },
        12 => Aim { goal: 13, ..*a },
        14 | 15 => Aim { n: (a.n + 1).min(10), ..*a },
        50 => Aim { n: a.n * 2, ..*a },
        _ if d.n > 0 => Aim { n: a.n + 1, ..*a },
        _ => *a,
    }
}

fn lower_words(w: &World, a: &Aim) -> Option<String> {
    lower(a).map(|l| goals::words(w, &l))
}

fn higher_words(w: &World, a: &Aim) -> String {
    lower_first(&goals::words(w, &higher(a)))
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn ordinal(n: u32) -> String {
    let suf = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suf}")
}

/// His yes to arguing a goal down: his patience and his trust in you.
fn yields(w: &World, team: Team) -> bool {
    let b = boss::boss(w, team);
    let t = boss::trust(w, team) as f64;
    let p = (0.35 + 0.03 * (b.patience - 10.0) + 0.5 * (t - 0.5)).clamp(0.05, 0.9);
    let key = w.today.0 as u64 ^ 0x7A1C;
    crate::world::folk::roll(w, key, b.key()) < p
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let b = boss::boss(w, team);
    s.you(&scene::your_name(w), picked.label.clone());
    let k = s.a as usize;
    let Some(mut m) = w.ground.campaign.draft.get(k).cloned() else {
        s.end();
        return;
    };
    match code {
        60 => {
            w.folk.mine.nudge(b.key(), 0.02);
            s.them(&b.name, "Good. Then we understand each other.", None);
            s.outcome.push(format!("You took his goal: {}.", goals::words(w, &m.aim)));
        }
        61 => match lower(&m.aim) {
            Some(l) if yields(w, team) => {
                m.aim = l;
                s.them(&b.name, "All right. That's the bar. Don't make me regret it.", None);
                s.outcome.push(format!("He lowered it: {}.", goals::words(w, &m.aim)));
            }
            _ => {
                w.folk.mine.nudge(b.key(), -0.01);
                s.them(&b.name, "No. That's what this job is.", None);
                s.outcome.push(format!("He kept it: {}, and noted that you asked.", goals::words(w, &m.aim)));
            }
        },
        62 => {
            m.aim = higher(&m.aim);
            w.folk.mine.nudge(b.key(), 0.04);
            let (yes, words) = if boss::trust(w, team) >= 0.35 {
                (true, boss::grant(w, team, AskKind::RecruitingMoney))
            } else {
                boss::ask_now(w, team, AskKind::RecruitingMoney)
            };
            s.them(&b.name, format!("That's the coach I hired. {words}"), None);
            s.outcome.push(format!(
                "You took it bigger: {}. {}",
                goals::words(w, &m.aim),
                if yes { "He put in for recruiting money." } else { "The money is a no for now." }
            ));
        }
        63 if !w.ground.campaign.wishes.is_empty() => {
            let patient = boss::boss(w, team).patience >= 8.0 || boss::trust(w, team) >= 0.55;
            if patient {
                let alt = w.ground.campaign.wishes.remove(0);
                w.ground.campaign.wishes.insert(0, m.clone());
                m = alt;
                s.them(&b.name, "Fine. Then that's the one I'll hold you to.", None);
                s.outcome.push(format!("He swapped it: {}.", goals::words(w, &m.aim)));
            } else {
                w.folk.mine.nudge(b.key(), -0.01);
                s.them(&b.name, "I didn't ask what you'd like to be judged on.", None);
                s.outcome.push("He wouldn't swap it.".into());
            }
        }
        64 => {
            let line = if m.who == "him" {
                format!("Because I'm the one who answers for it. {}", m.why)
            } else {
                format!("That's {}'s, and I don't say no to it lightly. {}", m.who, m.why)
            };
            s.them(&b.name, line, None);
            // Asking is not an answer: the goal stays on the table.
            s.picks.retain(|p| p.code != 64);
            return;
        }
        _ => {}
    }
    w.ground.campaign.draft[k] = m;
    s.a += 1;
    if (s.a as usize) < w.ground.campaign.draft.len() {
        ask_about(w, s, &b.name);
        return;
    }
    finish(w, s, team);
}

fn finish(w: &mut World, s: &mut Scene, team: Team) {
    let Team::College(prog) = team else { return };
    let set = std::mem::take(&mut w.ground.campaign.draft);
    let words: Vec<String> = set.iter().map(|m| goals::words(w, &m.aim)).collect();
    super::set_ad(w, prog, set);
    s.outcome.push(format!("He'll hold you to: {}.", words.join("; ")));
    s.end();
}

/// You skipped the talk: his goals stand as he set them, and he noticed.
pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else { return };
    let Team::College(prog) = team else { return };
    let b = boss::boss(w, team);
    w.folk.mine.nudge(b.key(), -0.04);
    if w.ground.campaign.draft.is_empty() {
        let class = super::class_now(w);
        let (held, rest) = table(w, prog, class);
        w.ground.campaign.draft = held;
        w.ground.campaign.wishes = rest;
    }
    let set = std::mem::take(&mut w.ground.campaign.draft);
    super::set_ad(w, prog, set);
    s.handed = Some("nobody: you sent word you couldn't make it".into());
    s.outcome.push(format!("You missed {}'s talk. His goals stand as he set them.", b.name));
}
