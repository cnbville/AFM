//! A coach's references before you hire (question 10, brought back
//! 2026-11-15): you call someone who worked with him and ask what you want
//! to know. Each answer is his read of one side of the man, through his eye
//! (the seasons they shared) and his agenda (C43's: a head coach keeping
//! his man undersells him, his athletic director a little, an old
//! staff-mate talks him up), straighter the warmer you are with him and the
//! more honest he is. What he tells you goes on the coach's page as an
//! honest read of that facet (`coach_look::Asked` with its facet), and you
//! owe him a favour for the call (C43's).
//!
//! Who you can call is C43's list (`coach_look::sources`). Anyone else who
//! worked beside him (your own staff included) can be asked about him in
//! any talk, through the same reads, but only a reference call puts it on
//! his page as one.

use super::super::Them;
use super::super::ear::{Act, Reading, Turn};
use super::super::knows::{Answer, Known, Source, Topic};
use crate::world::World;
use crate::world::coach::{Employer, GROUP_COUNT};
use crate::world::folk::cast::{CastRef, Role as CastRole};
use crate::world::folk::coach_look::{self, AMBITION, Asked};
use crate::world::folk::network::{self, Other, Whom};
use crate::world::folk::scene::{self, Kind, Scene, Who};
use crate::world::folk::{self as folk, Team};

/// A reference as a person: C43's source as the talk holds him.
pub fn them(w: &World, src: coach_look::Source) -> Them {
    match src {
        coach_look::Source::Coach(k) => Them::Coach(k),
        coach_look::Source::Ad(p, _) => {
            super::cast_them(w, CastRef::new(Team::College(p), CastRole::AthleticDirector, 0))
        }
    }
}

/// Opens a reference call about coach `c` with one of his sources (C43's
/// codes: `c<coach>` or `a<program>.<year>`). Half an hour of your week,
/// as C43's honest read costs, and a favour you owe him.
pub fn start(w: &mut World, c: usize, src: &str) -> Result<u32, String> {
    w.coaches.get(c).ok_or("No such coach.")?;
    let src = coach_look::Source::parse(src).ok_or("No such reference.")?;
    let list = coach_look::sources(w, c);
    let (label, _, _) = list.into_iter().find(|x| x.1 == src).ok_or("He can't speak to that coach.")?;
    if !folk::talks::spend(w, 0.5) {
        return Err("Your week is full.".into());
    }
    let him = them(w, src);
    let who = match him {
        Them::Coach(k) => Who::Coach(k),
        Them::Cast(r, _) => Who::Cast(r.token()),
        _ => return Err("No such reference.".into()),
    };
    let name = w.coaches[c].name.clone();
    let mut s = Scene::new(Kind::Reference, w.today, format!("A reference call about {name}"));
    s.a = c as u64;
    s.b = match src {
        coach_look::Source::Coach(k) => k as u64,
        coach_look::Source::Ad(p, y) => 1 << 40 | (p as u64) << 16 | (y as u64 & 0xFFFF),
    };
    s.stops = false;
    s.place = "A phone call".into();
    s.who = vec![who, Who::Coach(c)];
    s.narrate(format!("{label} picks up. You want to know about {}.", folk::coach_link(w, c)));
    let id = scene::add(w, s);
    // The favour (C43): you owe him one for his time and his honesty.
    match src {
        coach_look::Source::Coach(k) => {
            network::touch(w, k, 0.02);
            network::owe(w, Whom::Coach(k), format!("a reference on {name}"));
        }
        coach_look::Source::Ad(p, y) => {
            network::touch_other(w, Other::Ad(p, y), 0.02, "a call about a coach");
            network::owe(w, Whom::Other(Other::Ad(p, y)), format!("a reference on {name}"));
        }
    }
    Ok(id)
}

/// The facets a question reads (`coach_look::FACETS`'s indexes; ambition
/// is `AMBITION`): his football (scheme, calling, game management, his own
/// group's teaching), the man (relationships, discipline, motivation), his
/// recruiting (recruiting, network), his work (motivation, adaptability),
/// whether he'll stay (ambition).
pub fn facets(w: &World, c: usize, topic: Topic) -> Vec<usize> {
    match topic {
        Topic::Football | Topic::Opponent => vec![8, 10, 9, w.coaches[c].came_up as usize],
        Topic::Character | Topic::HisWord => vec![13, 12, 11],
        Topic::Pitches | Topic::Nil => vec![14, 15],
        Topic::Effort => vec![11, 16],
        Topic::Jobs | Topic::Job | Topic::Wants | Topic::Market => vec![AMBITION],
        _ => Vec::new(),
    }
}

/// A facet in words, for his answer.
fn facet_words(f: usize) -> &'static str {
    match f {
        x if x < GROUP_COUNT => "teaching his position group",
        8 => "his scheme",
        9 => "running a game",
        10 => "calling plays",
        11 => "getting players to play hard for him",
        12 => "discipline",
        13 => "how he is with players",
        14 => "recruiting",
        15 => "who he knows",
        16 => "changing with the game",
        _ => "what he wants next",
    }
}

/// Seasons two coaches shared a staff, and whether `a` was `b`'s boss.
fn shared(w: &World, a: usize, b: usize) -> (u32, bool) {
    let spans = |c: usize| {
        let co = &w.coaches[c];
        let mut v: Vec<(Employer, i32, i32, bool)> = co
            .history
            .iter()
            .map(|s| (s.employer, s.from, s.to, s.role == crate::world::coach::Role::HeadCoach))
            .collect();
        if let Some(j) = co.job {
            v.push((j.employer, j.since, w.year, j.role == crate::world::coach::Role::HeadCoach));
        }
        v
    };
    let (x, y) = (spans(a), spans(b));
    let mut years = 0;
    let mut boss = false;
    for p in &x {
        for q in y.iter().filter(|q| q.0 == p.0) {
            let n = (p.2.min(q.2) - p.1.max(q.1) + 1).max(0) as u32;
            years += n;
            boss |= n > 0 && p.3;
        }
    }
    (years, boss)
}

/// What a reference brings to a facet read: how straight he is (0-1) and
/// which way his agenda leans it (rating points, + oversold). C43's
/// agenda: his current boss keeps a good man (-1), an athletic director a
/// little (-0.6), an old staff-mate talks him up (+0.7); how straight:
/// 0.25 + 0.4 x your warmth + 0.25 x his honesty / 20 + 0.2 x a keyed draw
/// for the day (C43's 0.35 + 0.45 x warmth + 0.3 x draw, with his honesty
/// in it). Proposed, not confirmed.
pub fn agenda(w: &World, him: Them, c: usize) -> Option<(f32, f32, f64)> {
    let co = &w.coaches[c];
    let (keep, eye, warm) = match him {
        Them::Coach(k) if k != c => {
            let (years, boss) = shared(w, k, c);
            if years == 0 {
                return None;
            }
            let now_boss = co.job.is_some_and(|j| coach_look::boss_of(w, j.employer) == Some(k));
            let keep = if now_boss {
                -1.0
            } else if boss {
                0.0
            } else {
                0.7
            };
            let m = super::super::makeup::makeup(w, him);
            // His eye for a man he worked with: his judgment of people,
            // and 0.8 a season shared (5 at most).
            let eye = (0.6 * m.judgement + 0.8 * f64::from(years.min(5)) + 2.0).min(20.0);
            (keep, eye, network::warmth(w, k))
        }
        Them::Cast(r, _) if r.role == CastRole::AthleticDirector => {
            let Team::College(p) = r.team else { return None };
            let there = co.job.is_some_and(|j| j.employer == Employer::College(p))
                || co.history.iter().any(|s| s.employer == Employer::College(p));
            if !there {
                return None;
            }
            let since = w.programs[p].ad.as_ref().map_or(0, |a| a.since);
            (-0.6, 11.0, network::warmth_other(w, Other::Ad(p, since)))
        }
        _ => return None,
    };
    let honesty = super::super::makeup::makeup(w, him).honesty;
    let u = folk::roll(w, 0x4E58 ^ c as u64, w.today.0 as u64) as f32;
    let straight = (0.25 + 0.4 * warm + 0.25 * honesty as f32 / 20.0 + 0.2 * u).clamp(0.0, 1.0);
    let lean = keep * (1.0 - straight) * 3.0;
    Some((straight, lean, eye))
}

/// His read of one facet, 1-20: the truth, his agenda's lean and his eye's
/// miss (up to 3 points either way for a poor eye).
pub fn read(w: &World, him: Them, c: usize, f: usize) -> Option<f64> {
    let (_, lean, eye) = agenda(w, him, c)?;
    let miss =
        folk::roll_n(w, 0x4E59 ^ (c as u64) << 8 ^ f as u64, super::super::makeup::key(him)) * 3.0 * (20.0 - eye)
            / 20.0;
    Some((coach_look::truth(w, c, f) + lean as f64 + miss).clamp(1.0, 20.0))
}

fn band(x: f64) -> &'static str {
    match x {
        x if x >= 16.0 => "as good as anyone he's worked with at",
        x if x >= 13.0 => "good at",
        x if x >= 10.0 => "fine at",
        x if x >= 7.0 => "not strong at",
        _ => "a real weakness:",
    }
}

/// What he knows about the coach a question is about.
pub fn knows(w: &World, him: Them, topic: Topic, about: Option<Them>) -> Vec<Known> {
    let Some(Them::Coach(c)) = about else { return Vec::new() };
    if c >= w.coaches.len() || Them::Coach(c) == him {
        return Vec::new();
    }
    let Some((_, _, eye)) = agenda(w, him, c) else { return Vec::new() };
    let name = w.coaches[c].name.clone();
    let mut words: Vec<String> = Vec::new();
    match topic {
        Topic::Case | Topic::RuleBreak | Topic::Rules => {
            words.push(match w.coaches[c].show_cause {
                Some(y) if y >= w.year => format!("{name} is under an NCAA show-cause order through {y}"),
                Some(y) => format!("{name} had an NCAA show-cause order that ran out in {y}"),
                None => format!("nothing on {name} with the NCAA that he knows of"),
            });
        }
        _ => {
            for f in facets(w, c, topic) {
                let Some(r) = read(w, him, c, f) else { continue };
                words.push(if f == AMBITION {
                    match r {
                        x if x >= 15.0 => format!("{name} wants a head job, and soon"),
                        x if x >= 11.0 => format!("{name} wants to move up when it's right"),
                        _ => format!("{name} would stay somewhere he's happy"),
                    }
                } else {
                    format!("{name} is {} {}", band(r), facet_words(f))
                });
            }
        }
    }
    if words.is_empty() {
        return Vec::new();
    }
    vec![Known {
        topic,
        about,
        from: Source::WasThere,
        eye,
        point: None,
        says: Vec::new(),
        words: words.join("; "),
        sure: (0.35 + 0.65 * eye / 20.0) as f32,
        day: None,
    }]
}

/// After a line on a reference call: each question he answered about the
/// coach goes on the coach's page as an honest read of its facets (a
/// partial answer the first only), with his straightness and lean.
pub fn after(w: &mut World, id: u32, him: Them, reading: &Reading, t: &Turn) -> Vec<String> {
    let Some(s) = w.folk.scenes.iter().find(|s| s.id == id) else { return Vec::new() };
    if s.kind != Kind::Reference {
        return Vec::new();
    }
    let label = super::super::talk::channels::name(w, him);
    let questions = reading.acts.iter().filter_map(|a| match a {
        Act::Question { topic, about, .. } => Some((*topic, *about)),
        _ => None,
    });
    let mut out = Vec::new();
    for ((topic, about), told) in questions.zip(t.told.iter()) {
        let Some(Them::Coach(c)) = about else { continue };
        let Some((straight, lean, _)) = agenda(w, him, c) else { continue };
        let take = match told.answer {
            Answer::Straight => usize::MAX,
            Answer::Partly => 1,
            _ => 0,
        };
        let today = w.today;
        let fs: Vec<usize> = facets(w, c, topic).into_iter().take(take).collect();
        for &f in &fs {
            let words = told.released.first().cloned().unwrap_or_default();
            coach_look::look_mut(w, c).asked.push(Asked {
                who: label.clone(),
                date: today,
                straight,
                lean,
                words,
                facet: Some(f as u8),
            });
        }
        if !fs.is_empty() {
            out.push(format!("His read is on {}'s page.", w.coaches[c].name));
        }
    }
    out
}
