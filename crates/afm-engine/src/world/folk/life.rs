//! Life away from football (people topics 6 and 13, the build 9 leftovers),
//! medical second opinions and disputes (final build P3) and star player
//! power (P4).
//!
//! Pros' life events run weekly at real rates by age (births, weddings, a
//! parent's illness, deaths, divorce, money trouble, community work), and
//! your own players' at every level. A birth in season can cost a game; a
//! funeral a few days. Each event moves the life part of his mood and his
//! stability.
//!
//! Coaches' families are drawn from the coach (never stored): a spouse,
//! children and their ages. Their health is a state read from age and
//! stress (the hot seat, losing): heart trouble, exhaustion, a medical leave
//! with an interim. A serious event moves him toward retiring.
//!
//! Rates no source publishes are Claude's picks (proposed, not confirmed).

use super::scene::{self, Kind, Pick, Scene, Who};
use super::talks::{self, Reason};
use super::{Team, coach_link, player_link};
use crate::calendar::{Date, Level, Loudness, Weekday};
use crate::player::{PlayingTime, Trait};
use crate::world::{Stage, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Event {
    Birth,
    Wedding,
    ParentIll,
    Death,
    Divorce,
    MoneyTrouble,
    Community,
    Homesick,
    Graduation,
}

impl Event {
    pub fn words(self) -> &'static str {
        match self {
            Event::Birth => "became a father",
            Event::Wedding => "got married",
            Event::ParentIll => "has a parent who is seriously ill",
            Event::Death => "lost someone close to him",
            Event::Divorce => "is going through a divorce",
            Event::MoneyTrouble => "lost money to a bad adviser",
            Event::Community => "started a youth camp back home",
            Event::Homesick => "is homesick",
            Event::Graduation => "finished his degree",
        }
    }
    /// The mood it brings (topic 6), before his personality.
    fn mood(self) -> f32 {
        match self {
            Event::Birth | Event::Wedding | Event::Graduation => 0.12,
            Event::Community => 0.05,
            Event::ParentIll => -0.18,
            Event::Death => -0.25,
            Event::Divorce => -0.2,
            Event::MoneyTrouble => -0.15,
            Event::Homesick => -0.12,
        }
    }
}

/// A life event for a player, kept while it still weighs (about four months).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Happening {
    pub id: u32,
    pub event: Event,
    pub date: Date,
    /// He told you (a player who trusts you tells you; topic 6).
    pub told: bool,
}

/// A coach's health episode (topic 13): leave until a day, or stepping away.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Episode {
    pub coach: usize,
    pub date: Date,
    pub what: String,
    pub back: Option<Date>,
    /// Moves him toward retiring at the season's end.
    pub serious: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Lives {
    pub events: Vec<Happening>,
    pub health: Vec<Episode>,
    /// The year's counts: (year, births, deaths, coach health episodes).
    pub years: Vec<(i32, u32, u32, u32)>,
    /// Second opinions sought this year, league-wide (P3).
    pub opinions: Vec<(i32, u32)>,
    /// Your own life: the last family moment and how you answered it.
    pub home: Vec<(Date, String)>,
}

/// A coach's family (topic 13), drawn from his id: a spouse or not, his
/// children's birth years.
pub fn family(w: &World, c: usize) -> (bool, Vec<i32>) {
    let co = &w.coaches[c];
    let k = 0xFA111 ^ c as u64;
    let married = super::roll(w, k, 1) < if co.age >= 30 { 0.84 } else { 0.45 };
    let n = if co.age < 28 {
        0
    } else {
        let u = super::roll(w, k, 2);
        if u < 0.15 {
            0
        } else if u < 0.35 {
            1
        } else if u < 0.7 {
            2
        } else if u < 0.9 {
            3
        } else {
            4
        }
    };
    let born_self = w.year - co.age as i32;
    let kids =
        (0..n).map(|j| born_self + 27 + (super::roll(w, k, 3 + j) * 12.0) as i32).filter(|&y| y <= w.year).collect();
    (married, kids)
}

/// His family in words for his page.
pub fn family_words(w: &World, c: usize) -> String {
    let (married, kids) = family(w, c);
    let ages: Vec<String> = kids.iter().map(|y| (w.year - y).to_string()).collect();
    match (married, kids.len()) {
        (false, 0) => "Single, no children.".into(),
        (true, 0) => "Married, no children.".into(),
        (m, _) => format!("{}{} children ({}).", if m { "Married, " } else { "" }, kids.len(), ages.join(", ")),
    }
}

/// His open life event, in words, when he has told you (talks' concern).
pub fn open_event(w: &World, id: u32) -> Option<String> {
    w.folk
        .life
        .events
        .iter()
        .rev()
        .find(|e| e.id == id && e.event.mood() < 0.0 && e.date.days_to(w.today) < 90)
        .map(|e| format!("he {}", e.event.words()))
}

/// Time away you gave him (talks: "your family comes first"): a week out.
pub fn grant_leave(w: &mut World, id: u32) {
    let until = w.today.add(6);
    w.folk.out.retain(|x| x.0 != id);
    w.folk.out.push((id, until, "away with his family".into()));
    talks::mood_event(w, id, Reason::LifeEvent, 0.1);
    if let Some(e) = w.folk.life.events.iter_mut().rev().find(|e| e.id == id) {
        e.told = true;
    }
}

/// Yearly chances (proposed, not confirmed, from US rates by age for young
/// men, raised for pros' marriage and birth rates): births 9% a year past 24,
/// weddings 5%, a parent's serious illness 3%, a death in the family 5%,
/// divorce 1.5% past 26, money lost to an adviser 1% for pros, community
/// work 6% for pros with a name.
fn chance(e: Event, age: u32, pro: bool) -> f64 {
    let a = age as f64;
    match e {
        Event::Birth => {
            if a >= 24.0 {
                0.09
            } else if a >= 21.0 {
                0.03
            } else {
                0.005
            }
        }
        Event::Wedding => {
            if a >= 24.0 {
                0.05
            } else {
                0.01
            }
        }
        Event::ParentIll => 0.03,
        Event::Death => 0.05,
        Event::Divorce => {
            if a >= 26.0 {
                0.015
            } else {
                0.0
            }
        }
        Event::MoneyTrouble => {
            if pro {
                0.01
            } else {
                0.0
            }
        }
        Event::Community => {
            if pro {
                0.06
            } else {
                0.0
            }
        }
        Event::Homesick => {
            if !pro && a <= 19.0 {
                0.08
            } else {
                0.0
            }
        }
        Event::Graduation => 0.0,
    }
}

/// Weekly: life happens to every pro, and to your own players at every level.
fn players_week(w: &mut World, d: Date) {
    let in_season_nfl = d > w.dates.nfl_kickoff && d <= w.dates.super_bowl;
    let offseason = !in_season_nfl;
    let seat = Team::seat(w);
    let mut mine: Vec<usize> = seat.map_or(Vec::new(), |t| t.roster(w));
    mine.sort_unstable();
    let mut hits: Vec<(usize, Event)> = Vec::new();
    let all = [
        Event::Birth,
        Event::Wedding,
        Event::ParentIll,
        Event::Death,
        Event::Divorce,
        Event::MoneyTrouble,
        Event::Community,
        Event::Homesick,
    ];
    for i in 0..w.people.len() {
        let p = &w.people[i];
        let pro = p.stage == Stage::Pro && p.pro.as_ref().is_some_and(|c| c.team.is_some());
        if !pro && mine.binary_search(&i).is_err() {
            continue;
        }
        let id = p.player.id as u64;
        for (k, &e) in all.iter().enumerate() {
            if e == Event::Wedding && !offseason {
                continue;
            }
            let c = chance(e, p.player.age, pro) / 52.0;
            if c > 0.0 && super::roll(w, 0x11FE ^ (d.0 as u64) << 8 ^ k as u64, id) < c {
                hits.push((i, e));
                break;
            }
        }
    }
    let year = w.year;
    for (i, e) in hits {
        let id = w.people[i].player.id;
        let st = w.people[i].bg.stability;
        let swing = (1.0 + 0.05 * (10.0 - st) as f32).clamp(0.6, 1.5);
        talks::mood_event(w, id, Reason::LifeEvent, e.mood() * swing);
        let p = &mut w.people[i];
        if e.mood() < 0.0 {
            p.bg.stability = (p.bg.stability - 0.5).max(1.0);
        } else if matches!(e, Event::Wedding | Event::Birth) {
            p.bg.stability = (p.bg.stability + 0.4).min(20.0);
        }
        let age = p.player.age;
        p.note(format!("At {age} he {}.", e.words()));
        // A player who trusts you tells you (topic 6).
        let told = p.trust >= 0.55;
        w.folk.life.events.push(Happening { id, event: e, date: d, told });
        // Absences: a birth in season can cost a game; a funeral a few days.
        let away = match e {
            Event::Birth if in_season_nfl && super::roll(w, 0xB127, id as u64) < 0.35 => Some(6),
            Event::Death => Some(3),
            _ => None,
        };
        if let Some(days) = away {
            w.folk.out.push((id, d.add(days), format!("away: he {}", e.words())));
        }
        if mine.binary_search(&i).is_ok() && (told || matches!(e, Event::Birth | Event::Wedding | Event::Death)) {
            let text = format!("{} {}.", w.people[i].name, e.words());
            super::post_mine(w, d, Loudness::Small, false, "people: life", &text);
            super::note(w, "talk", player_link(w, i), format!("He {}.", e.words()), None);
        }
        match w.folk.life.years.iter_mut().find(|y| y.0 == year) {
            Some(y) => {
                y.1 += (e == Event::Birth) as u32;
                y.2 += (e == Event::Death) as u32;
            }
            None => w.folk.life.years.push((year, (e == Event::Birth) as u32, (e == Event::Death) as u32, 0)),
        }
    }
    let today = w.today;
    w.folk.life.events.retain(|e| e.date.days_to(today) < 150);
    if w.folk.life.events.len() > 1500 {
        let n = w.folk.life.events.len() - 1500;
        w.folk.life.events.drain(..n);
    }
}

/// Weekly: coaches' health (topic 13). The risk reads age and stress (a
/// losing season, the hot seat): heart trouble and strokes from the fifties,
/// exhaustion at any age. Most are a few days off; some are weeks of leave
/// with an interim; a few end in stepping away (proposed, not confirmed:
/// about 1% of head coaches a year).
fn coaches_week(w: &mut World, d: Date) {
    let you = w.career.as_ref().map(|c| c.coach);
    let mut hits: Vec<(usize, u8)> = Vec::new();
    for c in 0..w.coaches.len() {
        let co = &w.coaches[c];
        let Some(job) = co.job else { continue };
        if Some(c) == you {
            continue;
        }
        let head = job.role == crate::world::coach::Role::HeadCoach;
        if !head
            && !matches!(
                job.role,
                crate::world::coach::Role::OffensiveCoordinator | crate::world::coach::Role::DefensiveCoordinator
            )
        {
            continue;
        }
        let stress = co.recent.last().map_or(0.0, |r| (-r).max(0.0))
            + if co.wins + co.losses > 0 && co.losses > co.wins { 0.2 } else { 0.0 };
        let age = co.age as f64;
        let base = 0.004
            + 0.0006 * (age - 45.0).max(0.0)
            + 0.006 * stress
            + 0.002 * (co.personality.temper - 10.0).max(0.0) / 10.0;
        if super::roll(w, 0xC0A1 ^ d.0 as u64, c as u64) < base / 52.0 {
            let u = super::roll(w, 0xC0A2 ^ d.0 as u64, c as u64);
            let kind = if u < 0.55 {
                0
            } else if u < 0.9 {
                1
            } else {
                2
            };
            hits.push((c, kind));
        }
    }
    let year = w.year;
    for (c, kind) in hits {
        let age = w.coaches[c].age;
        let (what, back, serious) = match kind {
            0 => ("was hospitalized with exhaustion and missed a few days", Some(d.add(5)), false),
            1 => (
                if age >= 50 { "is on medical leave after a heart procedure" } else { "is on medical leave" },
                Some(d.add(28 + (super::roll(w, 0xC0A3, c as u64) * 28.0) as i32)),
                false,
            ),
            _ => ("is stepping away for his health", None, true),
        };
        w.folk.life.health.push(Episode { coach: c, date: d, what: what.into(), back, serious });
        let team = super::facts::team_of_coach(w, c);
        let mine = team.is_some() && team == Team::seat(w);
        let text = format!("{} {what}.", w.coaches[c].name);
        if mine {
            super::post_mine(w, d, Loudness::Medium, false, "people: health", &text);
        }
        match w.folk.life.years.iter_mut().find(|y| y.0 == year) {
            Some(y) => y.3 += 1,
            None => w.folk.life.years.push((year, 0, 0, 1)),
        }
    }
    let today = w.today;
    w.folk.life.health.retain(|e| e.date.days_to(today) < 400);
}

/// How much his health pushes a coach toward retiring this winter (read by
/// the coaching carousel): a serious episode this year.
pub fn retire_push(w: &World, c: usize) -> f64 {
    w.folk.life.health.iter().filter(|e| e.coach == c && e.serious && e.date.year() >= w.year - 1).count().min(1) as f64
        * 0.6
}

/// A coach on leave today (the interim runs the team).
pub fn on_leave(w: &World, c: usize) -> Option<String> {
    w.folk
        .life
        .health
        .iter()
        .rev()
        .find(|e| e.coach == c && e.back.is_some_and(|b| b > w.today))
        .map(|e| e.what.clone())
}

// ---------------------------------------------------------------------------
// P3: second opinions and medical disputes.

/// A long injury to one of your players: he (or his agent or family) wants
/// a second opinion, or disputes the timeline (P3). World-wide, the year's
/// count of second opinions is kept for the league's story.
fn opinions_week(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let roster = team.roster(w);
    let year = w.year;
    for i in roster {
        let p = &w.people[i];
        let id = p.player.id;
        let weeks = p.pro.as_ref().map_or(0, |c| c.out_weeks);
        let college_hurt = p
            .player
            .history
            .last()
            .is_some_and(|h| h.events.iter().any(|e| matches!(e, crate::player::DevelopmentEvent::MajorInjury)))
            && p.stage != Stage::Pro
            && d.days_to(w.dates.fall_camp).abs() < 4;
        if weeks < 4 && !college_hurt {
            continue;
        }
        if w.folk.scenes.iter().any(|s| s.kind == Kind::SecondOpinion && s.a == id as u64 && s.date.days_to(d) < 120) {
            continue;
        }
        let greed = p.player.personality.traits.z(Trait::Ego) + 0.5 * (1.0 - p.trust as f64);
        if super::roll(w, 0x2A0B ^ d.0 as u64, id as u64) > 0.35 + 0.25 * greed {
            continue;
        }
        let mut s = Scene::new(Kind::SecondOpinion, d, format!("{}'s injury", p.name));
        s.a = id as u64;
        s.c = weeks as i64;
        s.stops = false;
        s.team = Some(team);
        s.place = "The training room".into();
        scene::add(w, s);
        match w.folk.life.opinions.iter_mut().find(|x| x.0 == year) {
            Some(x) => x.1 += 1,
            None => w.folk.life.opinions.push((year, 1)),
        }
    }
}

fn opinion_open(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    s.who.push(Who::Player(id));
    let p = &w.people[i];
    let weeks = s.c;
    let who =
        if p.pro.as_ref().is_some_and(|c| c.agent.is_some()) { "His agent" } else { super::talks::parent_word(w, i) };
    let team = s.team.unwrap_or(Team::College(0));
    let trainer = super::cast::get(w, super::cast::CastRef::new(team, super::cast::Role::Trainer, 0));
    s.who.push(Who::Cast(trainer.r.token()));
    let estimate = if weeks > 0 {
        format!("The team's estimate is {weeks} weeks.")
    } else {
        "The team's doctors want him to have the surgery now.".into()
    };
    s.narrate(format!(
        "{} has the scans. {estimate} {who} wants an outside specialist to look at {} first.",
        trainer.link(),
        player_link(w, i)
    ));
    s.them(&trainer.name, "Our doctors are as good as anyone's. But if it buys his trust, I won't fight it.", None);
    s.pick(
        700,
        "Support the second opinion",
        "Praise",
        "warm",
        "Trust grows; the plan may change, and he may be out longer or shorter.",
    );
    s.pick(
        701,
        "Our doctors make the call",
        "Tell him straight",
        "firm",
        "The team's timeline holds; he and his camp take it as a no.",
    );
    s.pick(
        702,
        "Ask the trainer to bring him back early",
        "Challenge",
        "fiery",
        "He's back sooner, with a higher chance of reinjury. A trainer with good judgment pushes back.",
    );
}

fn opinion_answer(w: &mut World, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    let name = w.people[i].name.clone();
    match code {
        700 => {
            let u = super::roll(w, 0x2A0C, id as u64);
            w.people[i].trust = (w.people[i].trust + 0.08).min(1.0);
            let change = if u < 0.2 {
                -1
            } else if u < 0.35 {
                2
            } else {
                0
            };
            if let Some(c) = w.people[i].pro.as_mut() {
                c.out_for((c.out_days as i32 + 7 * change).max(1) as u32);
            }
            s.outcome.push(match change {
                -1 => format!(
                    "The specialist agreed and saw a quicker path: {name} is a week ahead of the first estimate."
                ),
                2 => format!("The specialist found more than the scans showed: {name} needs another two weeks."),
                _ => format!("The specialist agreed with your doctors. {name} trusts the plan now."),
            });
        }
        701 => {
            w.people[i].trust = (w.people[i].trust - 0.06).max(0.0);
            talks::mood_event(w, id, Reason::Talk, -0.05);
            s.outcome.push(format!("{name} stays with the team's doctors, and isn't happy about it."));
        }
        _ => {
            let team = s.team.unwrap_or(Team::College(0));
            let trainer = super::cast::get(w, super::cast::CastRef::new(team, super::cast::Role::Trainer, 0));
            if trainer.judgement >= 14.0 {
                s.them(&trainer.name, "No. Not with this injury. Put it on me if anyone asks.", None);
                w.folk.mine.nudge(trainer.key(), -0.05);
                s.outcome.push(format!("{} refused to rush {name}. The timeline holds.", trainer.name));
            } else {
                if let Some(c) = w.people[i].pro.as_mut() {
                    c.out_for((c.out_days * 3 / 4).max(1));
                }
                w.folk.rushed.push((id, w.today));
                let comp = w.people[i].player.personality.competitiveness;
                talks::mood_event(w, id, Reason::Talk, if comp >= 14.0 { 0.04 } else { -0.08 });
                s.outcome
                    .push(format!("{name} will be back sooner than the doctors wanted. The risk is his, and yours."));
            }
        }
    }
    s.end();
}

// ---------------------------------------------------------------------------
// P4: star player power.

/// A star on your team pushes for something (P4): the ball, a coordinator
/// out, a friend kept, a new deal. How hard he pushes reads his ego and his
/// mood; how much weight it carries reads his standing.
fn star_week(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    if w.folk.scenes.iter().any(|s| s.kind == Kind::StarPower && s.date.days_to(d) < 45) {
        return;
    }
    let roster = team.roster(w);
    let star = roster
        .iter()
        .copied()
        .filter(|&i| {
            let p = &w.people[i];
            p.last_role == Some(PlayingTime::Starter)
                && p.player.personality.traits.get(Trait::Ego) >= 14.0
                && p.mood < -0.15
                && (p.stars >= 5
                    || p.pro.as_ref().is_some_and(|c| c.seasons.iter().any(|s| s.pro_bowl || s.all_pro > 0))
                    || p.player.position == crate::positions::Position::QB)
        })
        .max_by(|&a, &b| w.people[a].player.overall().total_cmp(&w.people[b].player.overall()));
    let Some(i) = star else { return };
    if super::roll(w, 0x57A2 ^ d.0 as u64, i as u64) > 0.3 {
        return;
    }
    let p = &w.people[i];
    let want = match crate::world::people::Why::from_u8(p.mood_why) {
        crate::world::people::Why::Money if p.stage == Stage::Pro => 3,
        crate::world::people::Why::Role => 0,
        crate::world::people::Why::Coach | crate::world::people::Why::Losing => 1,
        _ => 0,
    };
    let mut s = Scene::new(Kind::StarPower, d, format!("{} wants a word", p.name));
    s.a = p.player.id as u64;
    s.b = want;
    s.stops = false;
    s.team = Some(team);
    s.place = "Your office, door closed".into();
    scene::add(w, s);
}

fn star_open(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    s.who.push(Who::Player(id));
    let name = w.people[i].name.clone();
    let team = s.team.unwrap_or(Team::College(0));
    let oc = match team {
        Team::College(p) => w.programs[p].oc,
        Team::Nfl(t) => w.nfl[t].oc,
        Team::Other(f) => crate::world::pro::other::slot(w, f, crate::world::coach::Role::OffensiveCoordinator),
        Team::High(_) => None,
    };
    let (ask, line) = match s.b {
        1 => (
            match oc {
                Some(c) => format!("He wants {} gone, and he says the room agrees.", coach_link(w, c)),
                None => "He wants the offense changed, and he says the room agrees.".into(),
            },
            "With respect, Coach, this scheme isn't working and everybody in that room knows it.",
        ),
        3 => (
            "He wants a new deal, or he wants out.".into(),
            "I've outplayed this contract. Either they pay me or I'd like to play somewhere that will.",
        ),
        _ => (
            "He wants the ball more, and a say in the game plan.".into(),
            "I'm the best player on this team. I need the ball in my hands when it matters.",
        ),
    };
    s.narrate(format!("{} closes the door behind him. {ask}", player_link(w, i)));
    s.them(&name, line, Some("He's said this to someone else first. You can tell.".into()));
    s.pick(
        800,
        "Give him what he wants",
        "Promise",
        "warm",
        "He's happy; the room sees one rule for the star and one for everyone else.",
    );
    s.pick(
        801,
        "Meet him halfway: a voice, not a veto",
        "Hand him something",
        "calm",
        "Some of what he wants, on your terms. Most stars take it.",
    );
    s.pick(
        802,
        "Tell him the team doesn't work that way",
        "Tell him straight",
        "firm",
        "The room respects it if you're consistent; he may go public or ask out.",
    );
}

fn star_answer(w: &mut World, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    let team = s.team.or(Team::seat(w));
    let name = w.people[i].name.clone();
    let ego = w.people[i].player.personality.traits.z(Trait::Ego) as f32;
    match code {
        800 => {
            talks::mood_event(w, id, Reason::Talk, 0.25);
            w.people[i].trust = (w.people[i].trust + 0.1).min(1.0);
            if let Some(t) = team {
                for j in t.roster(w) {
                    if j != i {
                        w.people[j].trust = (w.people[j].trust - 0.012).max(0.0);
                    }
                }
            }
            if s.b == 3 {
                talks::promise(w, talks::PromiseTo::Player(id), talks::PromiseKind::Contract, 0);
            }
            w.folk.mine.hard = (w.folk.mine.hard - 0.1).max(-1.0);
            s.outcome.push(format!("{name} got his way. The room noticed."));
        }
        801 => {
            talks::mood_event(w, id, Reason::Talk, 0.1 - 0.05 * ego.max(0.0));
            w.people[i].trust = (w.people[i].trust + 0.04).min(1.0);
            s.outcome.push(format!("{name} took the middle ground, for now."));
        }
        _ => {
            talks::mood_event(w, id, Reason::Talk, -0.15 - 0.1 * ego.max(0.0));
            w.people[i].trust = (w.people[i].trust - 0.08).max(0.0);
            if let Some(t) = team {
                for j in t.roster(w) {
                    if j != i {
                        w.people[j].trust = (w.people[j].trust + 0.006).min(1.0);
                    }
                }
            }
            w.folk.mine.hard = (w.folk.mine.hard + 0.08).min(1.0);
            let public = super::roll(w, 0x57A3, id as u64) < 0.25 + 0.25 * ego.max(0.0) as f64;
            if public {
                let text =
                    format!("{name} went public with his frustration after a closed-door meeting with his head coach.");
                super::post_mine(w, w.today.add(2), Loudness::Medium, false, "people: star", &text);
                s.outcome.push(format!("You told {name} no. Two days later it was in the papers."));
            } else {
                s.outcome.push(format!("You told {name} no. He didn't like it, but he kept it in the building."));
            }
        }
    }
    s.end();
}

// ---------------------------------------------------------------------------
// Your own life (topic 13): family moments the calendar lands on.

fn home_week(w: &mut World, d: Date) {
    let Some(c) = w.career.as_ref() else { return };
    let kids = c.you.profile.children.clone();
    let married = c.you.profile.status.eq_ignore_ascii_case("married");
    if kids.is_empty() && !married {
        return;
    }
    if w.folk.life.home.last().is_some_and(|x| x.0.days_to(d) < 90) {
        return;
    }
    if super::roll(w, 0x40E3 ^ d.0 as u64, 1) > 0.06 {
        return;
    }
    let pick = kids.iter().find(|k| (8..=18).contains(&(w.year - k.1)));
    let (title, about) = match pick {
        Some((name, born)) => {
            let age = w.year - born;
            (
                format!("{name}'s big night"),
                format!("{name} ({age}) has a game of {} own on Friday night: the night before your game.", "their"),
            )
        }
        None => ("Your anniversary".to_string(), "Your anniversary is Friday, the night before the game.".to_string()),
    };
    let mut s = Scene::new(Kind::Family, d, title);
    s.stops = false;
    s.place = "At home".into();
    s.narrate(about);
    scene::add(w, s);
    w.folk.life.home.push((d, "asked".into()));
}

fn home_open(_w: &mut World, s: &mut Scene) {
    s.pick(
        850,
        "Be there. The staff can finish the game plan.",
        "Hand him something",
        "warm",
        "Four hours of your week go home. Your family remembers you came.",
    );
    s.pick(
        851,
        "Stay at the office. It's game week.",
        "Tell him straight",
        "calm",
        "The plan gets your eyes. Home gets a phone call.",
    );
}

fn home_answer(w: &mut World, s: &mut Scene, code: u32) {
    if code == 850 {
        let _ = talks::spend(w, 4.0);
        s.outcome.push("You were there. On the drive home nobody talked about football.".into());
        w.folk.life.home.push((w.today, "went".into()));
    } else {
        s.outcome.push("You called from the office at halftime of their night. It wasn't the same.".into());
        w.folk.life.home.push((w.today, "stayed".into()));
    }
    s.end();
}

// ---------------------------------------------------------------------------

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::SecondOpinion => opinion_open(w, s),
        Kind::StarPower => star_open(w, s),
        Kind::Family => home_open(w, s),
        _ => s.end(),
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, _picked: &Pick) {
    match s.kind {
        Kind::SecondOpinion => opinion_answer(w, s, code),
        Kind::StarPower => star_answer(w, s, code),
        Kind::Family => home_answer(w, s, code),
        _ => s.end(),
    }
}

pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::SecondOpinion => {
            s.handed = Some("the medical staff".into());
            s.outcome.push("The medical staff arranged the second opinion. The timeline held.".into());
            if let Some(i) = super::person(w, s.a as u32) {
                w.people[i].trust = (w.people[i].trust + 0.03).min(1.0);
            }
        }
        Kind::StarPower => {
            s.handed = Some("nobody".into());
            s.outcome.push("You never got back to him. He took that as an answer.".into());
            talks::mood_event(w, s.a as u32, Reason::Talk, -0.12);
        }
        Kind::Family => {
            s.handed = Some("nobody".into());
            s.outcome.push("The week got away from you.".into());
        }
        _ => {}
    }
}

/// Weekly: a player you brought back early gets hurt again more often for
/// eight weeks (proposed, not confirmed: 8% a week against about 2% for a
/// healed player, the soft-tissue reinjury studies' rough ratio).
fn rushed_week(w: &mut World, d: Date) {
    let list = w.folk.rushed.clone();
    for (id, when) in list {
        if when.days_to(d) > 56 {
            continue;
        }
        let Some(i) = super::person(w, id) else { continue };
        let back = w.people[i].pro.as_ref().is_some_and(|c| c.out_weeks == 0);
        if !back || super::roll(w, 0x8E1A ^ d.0 as u64, id as u64) >= 0.08 {
            continue;
        }
        let weeks = 2 + (super::roll(w, 0x8E1B ^ d.0 as u64, id as u64) * 5.0) as u32;
        if let Some(c) = w.people[i].pro.as_mut() {
            c.out_for(7 * weeks);
            w.hurt_pros.0.push(i);
        }
        w.folk.rushed.retain(|x| x.0 != id);
        let name = w.people[i].name.clone();
        let age = w.people[i].row_age();
        w.people[i].note(format!("At {age} he was hurt again after coming back early ({weeks} weeks)."));
        talks::mood_event(w, id, Reason::Talk, -0.06);
        let link = player_link(w, i);
        super::post_mine(
            w,
            d,
            crate::calendar::Loudness::Medium,
            false,
            "Injury",
            &format!("{name} is hurt again after coming back early. He's out {weeks} more weeks."),
        );
        super::note(
            w,
            "injury",
            "Trainer".into(),
            format!("{link} re-injured the same area. Out {weeks} weeks."),
            None,
        );
    }
}

pub(crate) fn day(w: &mut World, d: Date) {
    match d.weekday() {
        Weekday::Thu => {
            players_week(w, d);
            rushed_week(w, d);
        }
        Weekday::Fri => coaches_week(w, d),
        Weekday::Wed if w.seat.is_some() => {
            opinions_week(w, d);
            star_week(w, d);
            home_week(w, d);
        }
        _ => {}
    }
    w.folk.rushed.retain(|x| x.1.days_to(d) < 120);
    let _ = Level::Nfl;
}
