//! What a person knows (lane N1, sections 3.1 and 10.1): the world's own
//! facts, filtered by where he stands. Each fact reaches him through a
//! `Source` (he lived it, his family, a close friend, he was there, his job
//! shows it, the tape, he was at the other team, the grapevine, the public
//! story), and he holds it as the truth plus his misread plus his shade:
//! the rule `scouting::file::source_says` built for sources (lane 6), now
//! everyone's. What is true about a player today comes from his fact sheet
//! (`claims::player`), never from a second reader of the same thing; what
//! the sheet doesn't hold (his mood's why, his portal thinking, an unseen
//! rule break, a recruit's pitches, his wants, his old team's habits, his
//! football) comes from the store that keeps it.
//!
//! Nobody knows a player's true potential (3.1): nothing here reads it.
//! A hidden number (a portal lean, a grade point average, a mood) is read
//! only as a gate or a band, and his words carry the band.
//!
//! Every size here is Claude's (proposed, not confirmed).

use super::Them;
use super::knows::{Known, Source, Topic};
use super::makeup::{self, Makeup};
use crate::calendar::Date;
use crate::world::folk::claims::{self, Kind as Pk, Point};
use crate::world::folk::{Team, ties};
use crate::world::scouting::Kind as Look;
use crate::world::scouting::file::{self, Shade};
use crate::world::{Stage, World};

/// The tie strength from which a friend hears the why behind a mood, not
/// only the mood (3.1: "close friends see the mood, and the why by tie").
pub const WHY_TIE: u8 = 55;

/// The trust from which a player tells you a life event on his own (built:
/// `life.rs`, 0.55); a friend's chance of knowing one runs off his tie.
pub const LIFE_TIE_FLOOR: f64 = 0.3;

/// Where `him` stands to the man a fact is about.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stand {
    /// He's the man.
    pub own: bool,
    /// Family: his kin, or a brother (a family tie).
    pub family: bool,
    /// Their tie's strength, 0-100 (a friend from 35).
    pub tie: u8,
    /// The same team now.
    pub mate: bool,
    /// The same position group now (his unit sees his practice).
    pub unit: bool,
    /// One of the team's captains (he sees the room).
    pub captain: bool,
    /// The man's coach now: head coach, coordinator or position coach.
    pub coach: bool,
    /// His position coach now.
    pub position_coach: bool,
    /// His high school coach (now or when he was there).
    pub hs_coach: bool,
    /// His agent.
    pub agent: bool,
    /// His team's trainer.
    pub trainer: bool,
    /// His school's academic staff.
    pub academic: bool,
    /// Years together on one team.
    pub years: f64,
}

fn friend(t: u8) -> bool {
    t >= 35
}

/// The tie between two players, 0-100, and whether it's family.
fn tie(w: &World, a: u32, b: u32) -> (u8, bool) {
    let mut best = (0u8, false);
    for t in w.folk.ties.of_id(a) {
        let other = if t.a == a { t.b } else { t.a };
        if other != b {
            continue;
        }
        let fam = t.kind == ties::Kind::Family;
        if t.strength > best.0 || fam {
            best = (t.strength.max(best.0), fam || best.1);
        }
    }
    best
}

fn captain(w: &World, team: Team, i: usize) -> bool {
    match team {
        Team::Nfl(t) => w.clubs.get(t).is_some_and(|c| c.captains.contains(&i)),
        Team::College(p) => w.programs.get(p).is_some_and(|x| x.captains.contains(&i)),
        _ => false,
    }
}

/// Where `him` stands to the player at `i`.
pub fn stand(w: &World, him: Them, i: usize) -> Stand {
    let p = &w.people[i];
    let id = p.player.id;
    let team = makeup::team_of(w, i);
    let mut s = Stand::default();
    match him {
        Them::Player(x) if x == id => s.own = true,
        Them::Player(x) => {
            let (t, fam) = tie(w, x, id);
            s.tie = t;
            s.family = fam;
            if let Some(j) = crate::world::folk::person(w, x) {
                let tj = makeup::team_of(w, j);
                s.mate = team.is_some() && tj == team;
                s.unit = s.mate
                    && crate::world::coach::Group::of(w.people[j].player.position)
                        == crate::world::coach::Group::of(p.player.position);
                s.captain = s.mate && team.is_some_and(|t| captain(w, t, j));
                s.years = makeup::years_with(w, i, j);
            }
        }
        Them::Kin(x, _) => {
            s.family = x == id;
            s.tie = if s.family { 95 } else { 0 };
        }
        Them::Coach(c) => {
            let ct = makeup::coach_team(w, c);
            s.coach = team.is_some() && ct == team;
            if s.coach {
                let g = crate::world::coach::Group::of(p.player.position);
                s.position_coach = w
                    .coaches
                    .get(c)
                    .and_then(|x| x.job)
                    .is_some_and(|j| j.role == crate::world::coach::Role::PositionCoach(g));
            }
            s.hs_coach = w.high_schools.get(p.hs).is_some_and(|h| h.coach == Some(c));
            if s.coach {
                s.years = p.college_staff.iter().filter(|x| x.2 == Some(c) || x.3 == Some(c)).count() as f64;
            }
        }
        Them::Cast(r, _) => {
            let ours = team == Some(r.team) || matches!(r.team, Team::High(h) if h == p.hs);
            use crate::world::folk::cast::Role as R;
            s.trainer = ours && r.role == R::Trainer;
            s.academic = ours && r.role == R::Academic;
            s.coach = ours && matches!(r.role, R::Strength);
        }
        Them::Agent(a) => s.agent = p.pro.as_ref().and_then(|c| c.agent) == Some(a),
        _ => {}
    }
    s
}

/// His eye for a fact about someone else (3.1): a teammate's is his
/// Football IQ and his years with the man, a coach's his relationships and
/// teaching, a high school coach's his recruiting, family's how close they
/// are, everyone else's his judgement.
pub fn eye(m: &Makeup, s: &Stand) -> f64 {
    let base = if s.own {
        20.0
    } else if s.family {
        14.0 + 0.2 * (m.judgement - 10.0)
    } else if s.mate {
        0.55 * m.iq + 0.25 * m.judgement + 1.5 * s.years.min(4.0)
    } else if s.coach || s.hs_coach {
        0.6 * m.judgement + 0.4 * m.iq + 0.8 * s.years.min(4.0)
    } else {
        m.judgement
    };
    base.clamp(1.0, 20.0)
}

/// How sure an eye makes him, 0-1.
pub fn sure_of(eye: f64) -> f32 {
    (0.35 + 0.65 * eye / 20.0).clamp(0.0, 1.0) as f32
}

fn k(topic: Topic, about: Option<Them>, from: Source, eye: f64, words: String) -> Known {
    Known { topic, about, from, eye, point: None, says: Vec::new(), words, sure: sure_of(eye), day: None }
}

/// A sheet point in plain words: what it is, then its values. The voice
/// gets these only once the game releases them.
pub fn point_words(pt: &Point) -> String {
    let label = match pt.kind {
        Pk::Arrest => "a case is open",
        Pk::CaseClosed => "a case closed lately",
        Pk::Suspended => "suspended",
        Pk::RuleBreak => "broke a team rule",
        Pk::LostJob => "lost his job lately",
        Pk::Buried => "buried on the depth chart",
        Pk::WonJob => "won a job lately",
        Pk::BigGame => "had a big game",
        Pk::BadGame => "had a bad game",
        Pk::Slump => "in a slump",
        Pk::Award => "won an award",
        Pk::Hurt => "hurt",
        Pk::Back => "back from an injury",
        Pk::PromiseOpen => "a promise from you is open",
        Pk::PromiseBroken => "a promise from you was broken",
        Pk::Pay => "his pay",
        Pk::WantsOut => "wants out",
        Pk::Portal => "the portal is on his mind",
        Pk::Draft => "the draft",
        Pk::FriendLeft => "a friend left the team",
        Pk::NewCoach => "a new position coach",
        Pk::Grades => "his grades",
        Pk::ParentCalled => "a parent called you",
        Pk::Effort => "the staff flagged his effort",
        Pk::PracticeWeek => "his week of practice",
        Pk::TestingDay => "testing day",
        Pk::ReportDay => "report day",
        Pk::NewPosition => "learning a new position",
        Pk::FailedTerm => "failed a term",
        _ if pt.kind.is_life() => "at home",
        _ => pt.kind.key(),
    };
    let mut parts: Vec<String> = Vec::new();
    for (h, v) in pt.values() {
        parts.push(format!("{h}: {v}"));
    }
    for (h, v) in pt.states() {
        parts.push(format!("{h}: {v}"));
    }
    if parts.is_empty() { label.to_string() } else { format!("{label} ({})", parts.join(", ")) }
}

/// The topic a sheet point answers.
pub fn topic_of(kind: Pk) -> Option<Topic> {
    Some(match kind {
        _ if kind.is_life() => Topic::Life,
        Pk::Arrest | Pk::CaseClosed | Pk::Suspended => Topic::Case,
        Pk::RuleBreak => Topic::RuleBreak,
        Pk::Effort | Pk::PracticeWeek | Pk::ReportDay | Pk::TestingDay => Topic::Effort,
        Pk::Hurt | Pk::Back => Topic::Injury,
        Pk::Portal | Pk::WantsOut => Topic::Portal,
        Pk::Grades | Pk::FailedTerm => Topic::Grades,
        Pk::Pay => Topic::Market,
        Pk::LostJob | Pk::WonJob | Pk::Buried | Pk::PromiseOpen | Pk::PromiseBroken | Pk::FriendLeft => Topic::Mood,
        Pk::BigGame | Pk::BadGame | Pk::Slump => Topic::Football,
        _ => return None,
    })
}

/// Who knows a sheet point about the man, and how (3.1's table).
fn point_source(pt: &Point, s: &Stand) -> Option<Source> {
    let t = topic_of(pt.kind)?;
    if s.own {
        return Some(Source::Himself);
    }
    Some(match t {
        Topic::Life => {
            if s.family {
                Source::Family
            } else if friend(s.tie) {
                Source::Friend
            } else {
                return None;
            }
        }
        Topic::Case => {
            if s.family {
                Source::Family
            } else if friend(s.tie) {
                Source::Friend
            } else {
                // The public step: everyone.
                Source::Public
            }
        }
        Topic::RuleBreak => {
            if s.coach {
                Source::Job
            } else if s.mate || friend(s.tie) {
                Source::There
            } else {
                return None;
            }
        }
        Topic::Effort => {
            if s.position_coach || s.coach {
                Source::Job
            } else if s.unit {
                Source::There
            } else {
                return None;
            }
        }
        Topic::Injury => {
            if s.trainer {
                Source::Job
            } else if s.family {
                Source::Family
            } else if s.mate || s.coach {
                Source::There
            } else {
                Source::Public
            }
        }
        Topic::Grades => {
            if s.family {
                Source::Family
            } else if s.academic || s.hs_coach {
                Source::Job
            } else {
                return None;
            }
        }
        Topic::Market => {
            if s.agent || s.family {
                Source::Job
            } else {
                return None;
            }
        }
        Topic::Football => {
            if s.mate || s.coach {
                Source::Tape
            } else {
                Source::Public
            }
        }
        _ => {
            if s.family {
                Source::Family
            } else if friend(s.tie) {
                Source::Friend
            } else if s.coach || s.mate {
                Source::There
            } else {
                return None;
            }
        }
    })
}

/// Facts from the sheet about the man on one topic, as `him` holds them.
fn from_sheet(w: &World, him: Them, i: usize, topic: Topic, about: Option<Them>, m: &Makeup, s: &Stand) -> Vec<Known> {
    let sheet = claims::player(w, i);
    let e = eye(m, s);
    let mut v = Vec::new();
    for pt in &sheet.points {
        if topic_of(pt.kind) != Some(topic) {
            continue;
        }
        let Some(src) = point_source(pt, s) else { continue };
        // A friend knows a life event as far as their tie goes (keyed, so
        // he knows it or not all season).
        if src == Source::Friend && topic == Topic::Life {
            let p = LIFE_TIE_FLOOR + 0.6 * s.tie as f64 / 100.0;
            if crate::world::folk::roll(w, 0x9E51 ^ makeup::key(him), pt.kind as u64) >= p {
                continue;
            }
        }
        let mut words = point_words(pt);
        // Whether he did it (`Point::hide`): the truth for himself, a
        // rumour for those close to him (right by their eye), nothing for
        // everyone else.
        if let Some(did) = pt.hid("did") {
            match src {
                Source::Himself => words.push_str(&format!("; the truth: he {}", did_words(did))),
                Source::Family | Source::Friend => {
                    let right =
                        crate::world::folk::roll(w, 0x9E52 ^ makeup::key(him), pt.kind as u64) < 0.5 + 0.4 * e / 20.0;
                    let rumour = if right { did } else { flip(did) };
                    words.push_str(&format!("; what he believes: he {}", did_words(rumour)));
                }
                _ => {}
            }
        }
        let mut kn = k(topic, about, src, e, words);
        kn.point = Some(pt.kind);
        kn.day = pt.day.or(pt.due);
        if src == Source::Public {
            kn.sure = kn.sure.min(0.6);
        }
        v.push(kn);
    }
    v
}

fn did_words(v: &str) -> &'static str {
    if v == "yes" { "did it" } else { "didn't do it" }
}

fn flip(v: &str) -> &'static str {
    if v == "yes" { "no" } else { "yes" }
}

/// A mood in words: its band, and the why when he can see it.
pub fn mood_words(w: &World, i: usize, why: bool) -> String {
    use crate::world::people::Why;
    let p = &w.people[i];
    // hidden: the mood is read only as a band.
    let band = match p.mood {
        x if x <= -0.5 => "in a bad place",
        x if x <= -0.15 => "down",
        // "About even" read as a stance ("about even on the draft", the
        // talks audit of 2026-11-16).
        x if x < 0.15 => "steady, neither up nor down",
        x if x < 0.5 => "in a good place",
        _ => "flying",
    };
    if !why {
        return band.into();
    }
    let because = match Why::from_u8(p.mood_why) {
        Why::Settled => return band.into(),
        Why::Role => "his role",
        Why::HappyRole => "his role",
        Why::Losing => "the losing",
        Why::Winning => "the winning",
        Why::Coach => "how he gets on with his coach",
        Why::Life => "things away from football",
        Why::Room => "the locker room",
        Why::Money => "his money",
    };
    format!("{band}, mostly {because}")
}

/// His portal thinking in words, from his lean as a band (hidden: the lean
/// is read only as a gate).
fn portal_words(w: &World, i: usize) -> Option<String> {
    let p = &w.people[i];
    if p.stage != Stage::College {
        return None;
    }
    // His college career ends this year: there's no transfer for him to
    // weigh (`talk::standing`).
    match super::talk::standing::road(w, i) {
        super::talk::standing::Road::LastSeason => {
            return Some("done with college football after this season, so the portal isn't for him".into());
        }
        super::talk::standing::Road::DraftBound => {
            return Some("done with college football, so the portal isn't for him".into());
        }
        _ => {}
    }
    if let Some(e) = &p.cal.portal {
        return Some(format!("in the portal since {}", claims::words::date_words(e.entered)));
    }
    Some(
        match p.cal.lean {
            x if x > 0.5 => "thinking hard about the portal",
            x if x > 0.25 => "has thought about the portal",
            _ => "not thinking about leaving",
        }
        .into(),
    )
}

/// An unseen rule break about the man (3.1): he knows it, the teammates
/// who were there know it, and his close friends heard.
fn unseen(w: &World, him: Them, i: usize, about: Option<Them>, m: &Makeup, s: &Stand) -> Vec<Known> {
    let id = w.people[i].player.id;
    let mut v = Vec::new();
    for u in w.folk.live.persona.unseen.iter().filter(|u| u.player == id) {
        let src = match him {
            _ if s.own => Source::Himself,
            Them::Player(x) if u.there.contains(&x) => Source::There,
            _ if friend(s.tie) => Source::Friend,
            _ if s.family => Source::Family,
            _ => continue,
        };
        let words = format!(
            "broke the {} rule on {}; the staff never saw it{}",
            u.rule.name().to_lowercase(),
            claims::words::date_words(u.day),
            if u.found { ", and it got out since" } else { "" }
        );
        let mut kn = k(Topic::RuleBreak, about, src, eye(m, s), words);
        kn.day = Some(u.day);
        v.push(kn);
    }
    v
}

/// The traits each stand's look shows (`source_says`' kinds): a teammate
/// sees practice, a friend the social side, family defends him, his high
/// school coach answers a call, his coach knows him from the staff room,
/// an agent sells him, everyone else has his background.
fn look_of(s: &Stand, kind: super::wants::Kind) -> (Look, Shade) {
    use super::wants::Kind as K;
    if s.own {
        (Look::Interview, Shade::Warm)
    } else if s.family {
        (Look::HomeVisit, Shade::Defends)
    } else if s.hs_coach {
        (Look::Call, Shade::Warm)
    } else if s.coach {
        (Look::OldStaff, Shade::True)
    } else if s.unit || s.mate {
        (Look::Practice, Shade::True)
    } else if friend(s.tie) {
        (Look::Social, Shade::Warm)
    } else if s.agent {
        (Look::Interview, Shade::Warm)
    } else if matches!(kind, K::Gm | K::OtherScout | K::Boss) {
        (Look::Background, Shade::True)
    } else {
        (Look::Background, Shade::Cold)
    }
}

/// A trait read in words: "work ethic: high".
pub fn trait_words(says: &[file::SourceSays]) -> String {
    let mut v = Vec::new();
    for x in says {
        let name = file::TRAIT_NAMES.get(x.k).copied().unwrap_or("character").to_lowercase();
        let band = match x.said {
            s if s >= 15.0 => "very high",
            s if s >= 12.5 => "high",
            s if s > 7.5 => "ordinary",
            s if s > 5.0 => "low",
            _ => "very low",
        };
        v.push(format!("{name}: {band}"));
    }
    v.join(", ")
}

fn character(w: &World, him: Them, i: usize, about: Option<Them>, m: &Makeup, s: &Stand) -> Vec<Known> {
    let kind = makeup::kind(w, him);
    let (look, shade) = look_of(s, kind);
    // His own read of himself: his professionalism sharpens it and his ego
    // bends it.
    let e = if s.own {
        (10.0 + 0.4 * (m.professionalism - 10.0) - 0.4 * (m.ego - 10.0)).clamp(1.0, 20.0)
    } else {
        eye(m, s)
    };
    let warmth = makeup::trust(w, him).clamp(0.0, 1.0);
    let all: Vec<usize> = (0..file::TRAITS).collect();
    let says = file::source_says(&w.people[i], look, shade, e, warmth, makeup::key(him), &all);
    if says.is_empty() {
        return Vec::new();
    }
    let from = if s.own {
        Source::Himself
    } else if s.family {
        Source::Family
    } else if s.mate {
        Source::There
    } else if friend(s.tie) {
        Source::Friend
    } else if s.coach || s.hs_coach || s.agent {
        Source::Job
    } else {
        Source::Public
    };
    let mut kn = k(Topic::Character, about, from, e, trait_words(&says));
    kn.says = says;
    vec![kn]
}

/// A recruit's pitches (3.1, `case.rs`): which schools came and what each
/// talked about.
fn pitches(w: &World, i: usize, about: Option<Them>, m: &Makeup, s: &Stand) -> Vec<Known> {
    let p = &w.people[i];
    let from = if s.own {
        Source::Himself
    } else if s.family {
        Source::Family
    } else if s.hs_coach {
        Source::Job
    } else if friend(s.tie) {
        Source::Friend
    } else {
        return Vec::new();
    };
    let mut v = Vec::new();
    for c in &p.cal.cases {
        let groups: Vec<&str> = claims::Group::ALL.iter().filter(|g| c.has(**g)).map(|g| g.name()).collect();
        if groups.is_empty() {
            continue;
        }
        let words = format!("{} talked about {}", w.program_name(c.prog as usize), groups.join(", "));
        let mut kn = k(Topic::Pitches, about, from, eye(m, s), words);
        kn.day = Some(c.on);
        v.push(kn);
    }
    // His high school coach saw every school that came through his door.
    if s.hs_coach || s.own || s.family {
        let offers: Vec<String> = p.cal.offers.iter().take(8).map(|&j| w.program_name(j).to_string()).collect();
        if !offers.is_empty() {
            v.push(k(Topic::Pitches, about, from, eye(m, s), format!("offers from {}", offers.join(", "))));
        }
        for (j, d, _) in p.cal.visits.iter().rev().take(4) {
            let mut kn = k(Topic::Pitches, about, from, eye(m, s), format!("visited {}", w.program_name(*j)));
            kn.day = Some(*d);
            v.push(kn);
        }
    }
    v
}

/// A recruit's or a player's money (3.1: NIL offers, known to him, his
/// decider and his agent).
fn nil(w: &World, i: usize, about: Option<Them>, m: &Makeup, s: &Stand) -> Vec<Known> {
    let p = &w.people[i];
    let from = if s.own {
        Source::Himself
    } else if s.family {
        Source::Family
    } else if s.agent {
        Source::Job
    } else {
        return Vec::new();
    };
    let mut v = Vec::new();
    let money: Vec<&str> =
        p.cal.cases.iter().filter(|c| c.has(claims::Group::Money)).map(|c| w.program_name(c.prog as usize)).collect();
    if !money.is_empty() {
        v.push(k(Topic::Nil, about, from, eye(m, s), format!("schools that talked money: {}", money.join(", "))));
    }
    if p.nil_income > 0.0 {
        v.push(k(Topic::Nil, about, from, eye(m, s), format!("makes about ${:.0} a year in NIL", p.nil_income)));
    }
    v
}

/// His grades as a band (hidden: the grade point average is a gate).
fn grades(w: &World, him: Them, i: usize, about: Option<Them>, m: &Makeup, s: &Stand) -> Vec<Known> {
    let from = if s.own {
        Source::Himself
    } else if s.family {
        Source::Family
    } else if s.academic || s.hs_coach {
        Source::Job
    } else {
        return from_sheet(w, him, i, Topic::Grades, about, m, s);
    };
    let gpa = w.people[i].bg.gpa;
    let band = match gpa {
        g if g < 2.3 => "struggling in class, eligibility at risk",
        g if g < 2.8 => "getting by in class",
        g if g < 3.4 => "a solid student",
        _ => "a very good student",
    };
    let mut v = from_sheet(w, him, i, Topic::Grades, about, m, s);
    v.push(k(Topic::Grades, about, from, eye(m, s), band.into()));
    v
}

/// His wants as he'd say them, or as those close to him read them.
fn wants_known(w: &World, i: usize, about: Option<Them>, m: &Makeup, s: &Stand) -> Vec<Known> {
    let id = w.people[i].player.id;
    let from = if s.own {
        Source::Himself
    } else if s.family {
        Source::Family
    } else if s.agent {
        Source::Job
    } else if friend(s.tie) {
        Source::Friend
    } else {
        return Vec::new();
    };
    let wants = super::wanting::raw(w, Them::Player(id));
    let n = if s.own || s.family || s.agent { 3 } else { 1 };
    let words: Vec<&str> = wants.iter().take(n).map(|x| super::wanting::want_words(x.0)).collect();
    if words.is_empty() {
        return Vec::new();
    }
    vec![k(Topic::Wants, about, from, eye(m, s), format!("wants {}", words.join(", then ")))]
}

/// A pro's market: the offers waiting on him (his agent, him, his family,
/// and the GM who made one).
fn market(w: &World, him: Them, i: usize, about: Option<Them>, m: &Makeup, s: &Stand) -> Vec<Known> {
    let mut v = from_sheet(w, him, i, Topic::Market, about, m, s);
    // A college man with the draft in front of him hears the NFL scouts'
    // word through his school (`talk::standing::scouts_word`), and so do
    // his family and his agent: asked what the scouts say, he tells it, or
    // that there's nothing solid yet (the talks audit, 2026-11-16).
    let p = &w.people[i];
    if matches!(p.stage, Stage::College | Stage::Declared)
        && (s.own || s.family || s.agent)
        && super::talk::standing::road(w, i).draft_near()
    {
        let from = if s.own {
            Source::Himself
        } else if s.family {
            Source::Family
        } else {
            Source::Job
        };
        let words = match super::talk::standing::scouts_word(w, i) {
            Some(word) => format!("what he hears from the NFL scouts through the school: {word}"),
            None => "nothing solid from the NFL scouts yet".to_string(),
        };
        v.push(k(Topic::Market, about, from, eye(m, s), words));
    }
    let gm_of = |t: usize| matches!(him, Them::Cast(r, _) if r.team == Team::Nfl(t) && r.role == crate::world::folk::cast::Role::Gm);
    for pd in w.market.pending.iter().filter(|x| x.person == i) {
        let from = if s.own {
            Source::Himself
        } else if s.agent || s.family || gm_of(pd.offer.team) {
            Source::Job
        } else {
            continue;
        };
        let words = format!(
            "{} offered {} years at ${:.1}M a year, ${:.1}M guaranteed, open till {}",
            w.nfl_team_name(pd.offer.team),
            pd.offer.years,
            pd.offer.apy / 1e6,
            pd.offer.gtd / 1e6,
            claims::words::date_words(pd.until)
        );
        let mut kn = k(Topic::Market, about, from, eye(m, s), words);
        kn.day = Some(pd.until);
        v.push(kn);
    }
    v
}

/// His old team's habits (9.7): a man who came from another team reads
/// their calls on his side of the ball, by his Football IQ, his years
/// there and (when he tells) his loyalty to his old coaches. Your staff
/// reads the next opponent off the tape.
fn opponent(w: &World, him: Them, about: Option<Them>, m: &Makeup) -> Vec<Known> {
    let mut v = Vec::new();
    match him {
        Them::Player(id) => {
            let Some(i) = crate::world::folk::person(w, id) else { return v };
            let p = &w.people[i];
            let now = makeup::team_of(w, i);
            let offense = crate::world::coach::Group::of(p.player.position).offense();
            // His last team before this one.
            let old: Option<(Team, f64)> = if p.path.len() >= 2 {
                let j = p.path[p.path.len() - 2];
                Some((Team::College(j), 1.0 + p.college_seasons.saturating_sub(1).min(4) as f64 * 0.5))
            } else {
                p.pro.as_ref().and_then(|c| {
                    let n = c.teams.len();
                    (n >= 2).then(|| (Team::Nfl(c.teams[n - 2]), 1.0 + 0.5 * c.accrued.min(4) as f64))
                })
            };
            let Some((team, years)) = old.filter(|x| Some(x.0) != now) else { return v };
            // His eye: Football IQ, his years there.
            let e = (0.7 * m.iq + 1.2 * years.min(4.0)).clamp(1.0, 20.0);
            if let Some(words) = habits(w, team, offense, e) {
                let mut kn = k(Topic::Opponent, about, Source::WasThere, e, words);
                kn.sure = sure_of(e);
                v.push(kn);
            }
        }
        Them::Coach(c) => {
            let Some(t) = makeup::coach_team(w, c) else { return v };
            if Some(t) != Team::seat(w) {
                return v;
            }
            // Your coordinators read the next opponent off the tape.
            let Some(opp) = next_opponent(w, t) else { return v };
            let side = w.coaches[c].job.and_then(|j| j.role.offense());
            let e = m.iq;
            for off in [true, false] {
                if side.is_some_and(|s| s != off) {
                    continue;
                }
                if let Some(words) = habits(w, opp, off, e) {
                    v.push(k(Topic::Opponent, about, Source::Tape, e, words));
                }
            }
        }
        _ => {}
    }
    v
}

fn next_opponent(w: &World, t: Team) -> Option<Team> {
    claims::Reader::new(w).games(t).into_iter().find(|g| g.0 >= w.today).map(|g| g.1)
}

/// A team's habits on one side of the ball from its tally, read through
/// an eye: the share of passes on 3rd and long, the blitz share and man
/// coverage on defense. A weak eye rounds harder and can be wrong.
pub fn habits(w: &World, team: Team, offense: bool, eye: f64) -> Option<String> {
    use crate::world::tally::Level as L;
    let (level, id) = match team {
        Team::College(p) => (L::College, p as u32),
        Team::Nfl(t) => (L::Nfl, t as u32),
        _ => return None,
    };
    let season = w.tally_open_season();
    let s = w.tally_team(level, season, id).or_else(|| w.tally_team(level, season - 1, id))?;
    let side = if offense { "o" } else { "d" };
    let sum = |pre: &str| -> f32 { s.cells.iter().filter(|(key, _)| key.starts_with(pre)).map(|(_, x)| x[0]).sum() };
    // A sharp eye keeps the share; a weak one is off by up to a fifth.
    let off = ((20.0 - eye) / 20.0 * 0.2) as f32;
    let shade = |x: f32, salt: u64| {
        let u = crate::world::folk::roll(w, 0x9E53 ^ salt, team.key()) as f32 * 2.0 - 1.0;
        (x + u * off).clamp(0.0, 1.0)
    };
    let pct = |x: f32| format!("{:.0}%", 100.0 * x);
    let mut parts = Vec::new();
    if offense {
        let long_pass = sum(&format!("{side}|third|7-10|pass")) + sum(&format!("{side}|third|11+|pass"));
        let long_run = sum(&format!("{side}|third|7-10|run")) + sum(&format!("{side}|third|11+|run"));
        if long_pass + long_run >= 10.0 {
            parts.push(format!("they throw {} on 3rd and long", pct(shade(long_pass / (long_pass + long_run), 1))));
        }
        let pa = sum(&format!("{side}|play action|true"));
        let db = pa + sum(&format!("{side}|play action|false"));
        if db >= 30.0 {
            parts.push(format!("play action on {} of dropbacks", pct(shade(pa / db, 2))));
        }
    } else {
        let man = sum(&format!("{side}|coverage|man"));
        let zone = sum(&format!("{side}|coverage|zone"));
        if man + zone >= 30.0 {
            parts.push(format!("man coverage {} of the time", pct(shade(man / (man + zone), 3))));
        }
        let blitz: f32 = (1..=4).map(|d| sum(&format!("{side}|blitz|{d}|true"))).sum();
        let calm: f32 = (1..=4).map(|d| sum(&format!("{side}|blitz|{d}|false"))).sum();
        if blitz + calm >= 30.0 {
            parts.push(format!("they blitz {} of dropbacks", pct(shade(blitz / (blitz + calm), 4))));
        }
        let b3: f32 = sum(&format!("{side}|blitz|3|true"));
        let c3: f32 = sum(&format!("{side}|blitz|3|false"));
        if b3 + c3 >= 15.0 {
            parts.push(format!("{} on 3rd down", pct(shade(b3 / (b3 + c3), 5))));
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!("{} ({}): {}", team.name(w), if offense { "offense" } else { "defense" }, parts.join("; ")))
}

/// What the man's boss knows about your job (3.1: "your boss's goals and
/// trust in you"; `boss.rs`): the goals he set for the season they're for
/// (the one his goals meeting sets, else the one being played), and where
/// you stand with him.
fn goals(w: &World, him: Them) -> Vec<Known> {
    if makeup::kind(w, him) != super::wants::Kind::Boss {
        return Vec::new();
    }
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let mut v = Vec::new();
    let ahead = crate::world::folk::boss::goal_season(w, team);
    let season = [ahead, w.dates.season]
        .into_iter()
        .find(|s| w.folk.goals.iter().any(|g| g.season == *s))
        .unwrap_or(w.dates.season);
    let goals: Vec<String> = w
        .folk
        .goals
        .iter()
        .filter(|g| g.season == season)
        .map(|g| crate::world::folk::commit::goal_words(w, g.kind, team.level()))
        .collect();
    if !goals.is_empty() {
        v.push(k(Topic::Goals, None, Source::Himself, 20.0, format!("the goals for {season}: {}", goals.join(", "))));
    }
    let t = crate::world::folk::boss::trust(w, team);
    v.push(k(Topic::Goals, None, Source::Himself, 20.0, crate::world::folk::boss::trust_short(t).to_string()));
    v
}

/// Your boss on the money (`Topic::Money`, asked of him): how the
/// program's money stands, in words and never a figure (`boss::rich`, his
/// tight hand), and how more is found: a case, and the bar you put up for
/// it (`asked::money`). Before this he didn't know, or deflected the
/// budget he holds (the talks audit, 2026-11-16).
fn budget(w: &World, him: Them) -> Vec<Known> {
    if makeup::kind(w, him) != super::wants::Kind::Boss {
        return Vec::new();
    }
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let b = crate::world::folk::boss::boss(w, team);
    let rich = crate::world::folk::boss::rich(w, team);
    let stands = if rich < 0.6 {
        "money is tight here next to the big programs, so the budget is what it is"
    } else if b.tight >= 13.0 {
        "the money is there, but he keeps the budget tight"
    } else {
        "the budget for the season is set, and there's room when something earns it"
    };
    vec![k(
        Topic::Money,
        None,
        Source::Himself,
        20.0,
        format!(
            "{stands}: more for the staff, recruiting or the collective comes with a case, and the bar the coach puts up for it"
        ),
    )]
}

/// Your boss on how it's going (`Topic::Football` asked of him: "how do you
/// think we're doing so far?"): where you stand with him and the season's
/// record, never the tape he doesn't watch. Before this he had no idea
/// (the talks audit, 2026-11-16).
fn boss_view(w: &World, him: Them) -> Vec<Known> {
    if makeup::kind(w, him) != super::wants::Kind::Boss {
        return Vec::new();
    }
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let t = crate::world::folk::boss::trust(w, team);
    let mut v =
        vec![k(Topic::Football, None, Source::Himself, 20.0, crate::world::folk::boss::trust_short(t).to_lowercase())];
    let (won, lost) = crate::world::folk::facts::record_now(w, team);
    if won + lost > 0 {
        v.push(k(Topic::Football, None, Source::Himself, 20.0, format!("the team is {won}-{lost} this season")));
    }
    v
}

/// A coach on his unit or the team, asked with no one named: his unit's
/// year and the team's record since he came (`claims::rooms::unit`), and
/// always his own read from the tape, which he gives in his own words.
fn unit_view(w: &World, him: Them) -> Vec<Known> {
    let Them::Coach(c) = him else { return Vec::new() };
    let mut v = Vec::new();
    if let Some(u) = claims::rooms::unit(w, c) {
        if let Some(y) = u.year_words() {
            v.push(k(Topic::Football, None, Source::Himself, 20.0, y.trim_end_matches('.').to_string()));
        }
        if let Some(r) = u.record_words() {
            v.push(k(Topic::Football, None, Source::Himself, 20.0, format!("the team is {r} since he took the job")));
        }
    }
    v.push(k(
        Topic::Football,
        None,
        Source::Tape,
        20.0,
        "his own read of the unit from the tape and practice: what it does well and where it falls short, in general terms".into(),
    ));
    v
}

/// His own job here (`Topic::Job`): a coach of yours makes his case from
/// what he can point to (his years, the team since he came, his unit, his
/// deal; the facts `talk::stakes` puts on his card); your boss tells you
/// where you stand with him (his goals and his trust). Nobody else holds a
/// job of his you'd ask about.
fn own_job(w: &World, him: Them, about: Option<Them>) -> Vec<Known> {
    if about.is_some_and(|a| a != him) {
        return Vec::new();
    }
    let kind = makeup::kind(w, him);
    if kind == super::wants::Kind::Boss {
        return goals(w, him)
            .into_iter()
            .map(|mut x| {
                x.topic = Topic::Job;
                x
            })
            .collect();
    }
    let Them::Coach(c) = him else { return Vec::new() };
    if kind != super::wants::Kind::Staff {
        return Vec::new();
    }
    super::talk::stakes::case_facts(w, c)
        .into_iter()
        .map(|words| k(Topic::Job, None, Source::Himself, 20.0, words))
        .collect()
}

/// Jobs opening (3.1: coaches in the network, agents, ADs know).
fn jobs(w: &World, him: Them, m: &Makeup) -> Vec<Known> {
    use super::wants::Kind as K;
    let kind = makeup::kind(w, him);
    let network = match him {
        Them::Coach(c) => w.coaches.get(c).map_or(10.0, |x| x.ability().network),
        _ => m.judgement,
    };
    if !matches!(kind, K::Staff | K::OtherCoach | K::HsCoach | K::Agent | K::Boss | K::Outside | K::Gm) {
        return Vec::new();
    }
    // The better his network, the more of the carousel he hears.
    let reach = (0.25 + 0.04 * network).clamp(0.2, 1.0);
    let mut v = Vec::new();
    for (n, job) in crate::world::front_office::market::openings(w).into_iter().enumerate() {
        if crate::world::folk::roll(w, 0x9E54 ^ makeup::key(him), n as u64) >= reach {
            continue;
        }
        let team = Team::from_employer(job.employer);
        let mut words = format!("{} is looking for a {}", team.name(w), job.role.name());
        if let Some(d) = job.decides {
            words.push_str(&format!(", deciding by {}", claims::words::date_words(d)));
        }
        let mut kn = k(Topic::Jobs, None, Source::Grapevine, network, words);
        kn.day = job.decides;
        v.push(kn);
        if v.len() >= 6 {
            break;
        }
    }
    v
}

/// What came to him on the grapevine (10.5).
fn grapevine(w: &World, him: Them, about: Option<Them>) -> Vec<Known> {
    w.folk
        .live
        .persona
        .grapevine
        .iter()
        .filter(|h| h.to == him && h.day <= w.today && (about.is_none() || h.about == about))
        .map(|h| {
            let mut kn = k(Topic::Grapevine, h.about, Source::Grapevine, 10.0, h.words.clone());
            kn.sure = (0.8 - 0.15 * h.hops as f32).max(0.2);
            kn.day = Some(h.day);
            kn
        })
        .collect()
}

/// What he told you before, and whether it was true (10.2): he knows his
/// own lies.
fn his_word(w: &World, him: Them, about: Option<Them>) -> Vec<Known> {
    // What is between you, from the one memory (Ground, GR62): the moments
    // he keeps, in his words.
    let mut kept: Vec<Known> = if about.is_none() {
        crate::world::ground::card::talk_lines(w, him)
            .into_iter()
            .map(|(d, m)| {
                let mut kn = k(Topic::HisWord, None, Source::Himself, 20.0, format!("remembers \"{m}\""));
                kn.day = Some(d);
                kn
            })
            .collect()
    } else {
        Vec::new()
    };
    let lies: Vec<Known> = w
        .folk
        .live
        .persona
        .his_lies
        .iter()
        .filter(|l| l.by == him && (about.is_none() || l.about == about))
        .map(|l| {
            let words = format!(
                "told you \"{}\" on {}; it wasn't true{}",
                l.words,
                claims::words::date_words(l.said),
                if l.caught { ", and you know it" } else { "" }
            );
            let mut kn = k(Topic::HisWord, l.about, Source::Himself, 20.0, words);
            kn.day = Some(l.said);
            kn
        })
        .collect();
    kept.extend(lies);
    kept
}

/// An injury as he feels it (3.1): the sheet's diagnosis for the trainer,
/// and how his body feels for himself (his wear).
fn injury(w: &World, him: Them, i: usize, about: Option<Them>, m: &Makeup, s: &Stand) -> Vec<Known> {
    let mut v = from_sheet(w, him, i, Topic::Injury, about, m, s);
    if s.own || s.trainer {
        let wear = w.people[i].player.wear;
        let words = match wear {
            x if x > 0.6 => "his body is beat up: everything hurts",
            x if x > 0.35 => "sore, the usual knocks",
            _ => "feels fresh",
        };
        v.push(k(Topic::Injury, about, if s.own { Source::Himself } else { Source::Job }, 20.0, words.into()));
    }
    v
}

/// Everything `him` knows on a topic about `about` (or himself).
pub fn knows(w: &World, him: Them, topic: Topic, about: Option<Them>) -> Vec<Known> {
    let m = makeup::makeup(w, him);
    // The man the fact is about: `about`, or him when he is a player.
    let subject = match about {
        Some(Them::Player(id)) => crate::world::folk::person(w, id),
        Some(_) => None,
        None => makeup::index(w, him),
    };
    // A family member asked about "himself" is asked about his son.
    let s = subject.map(|i| stand(w, him, i)).unwrap_or_default();
    match topic {
        Topic::Grapevine => return grapevine(w, him, about),
        Topic::HisWord => return his_word(w, him, about),
        Topic::Opponent => return opponent(w, him, about, &m),
        // Asked his goals, a man with no goal sheet to set says what he
        // wants: "what do you want to get done before the draft?" got "He
        // doesn't know" from a senior (the talks audit, 2026-11-16).
        Topic::Goals if makeup::kind(w, him) != super::wants::Kind::Boss => {
            return knows(w, him, Topic::Wants, about);
        }
        Topic::Goals => return goals(w, him),
        Topic::Money if makeup::kind(w, him) == super::wants::Kind::Boss => return budget(w, him),
        // Asked what he's looking for, your boss means his goals for you
        // ("what are you looking for from us this year?" in his goals
        // meeting got "nothing new there to tell", the talks audit of
        // 2026-11-16).
        Topic::Wants if makeup::kind(w, him) == super::wants::Kind::Boss && about.is_none_or(|a| a == him) => {
            return goals(w, him);
        }
        Topic::Jobs => return jobs(w, him, &m),
        // A player's "job" is his place: what he wants and where he stands.
        Topic::Job if matches!(him, Them::Player(_) | Them::Kin(..)) => return knows(w, him, Topic::Wants, about),
        Topic::Job => return own_job(w, him, about),
        // Asked how it's going, your boss tells you how he sees the season.
        Topic::Football if makeup::kind(w, him) == super::wants::Kind::Boss && about.is_none_or(|a| a == him) => {
            return boss_view(w, him);
        }
        // A coach of yours asked about his unit ("what's the biggest thing
        // holding the unit back?"): what he knows of its year, never "he
        // doesn't know" (the talks audit, 2026-11-16: a coordinator of three
        // seasons said he hadn't had time with the group).
        Topic::Football
            if matches!(him, Them::Coach(_))
                && about.is_none_or(|a| a == him)
                && makeup::kind(w, him) == super::wants::Kind::Staff =>
        {
            return unit_view(w, him);
        }
        Topic::Football => return super::football::knows(w, him, about, &m),
        Topic::Nothing => return Vec::new(),
        _ => {}
    }
    let Some(i) = subject else { return Vec::new() };
    match topic {
        Topic::Life | Topic::Case | Topic::Effort => {
            let mut v = from_sheet(w, him, i, topic, about, &m, &s);
            if topic == Topic::Effort && v.is_empty() && !s.own && (s.unit || s.coach) {
                // No flag on the sheet: what his unit and coaches see of
                // his work, as a trait read.
                let (look, shade) = (Look::Practice, Shade::True);
                let says = file::source_says(&w.people[i], look, shade, eye(&m, &s), 0.5, makeup::key(him), &[0, 2]);
                if !says.is_empty() {
                    let mut kn = k(Topic::Effort, about, Source::There, eye(&m, &s), trait_words(&says));
                    kn.says = says;
                    v.push(kn);
                }
            }
            v
        }
        Topic::Mood => {
            let mut v = from_sheet(w, him, i, topic, about, &m, &s);
            let why = s.own || s.family || s.tie >= WHY_TIE;
            let from = if s.own {
                Some(Source::Himself)
            } else if s.family {
                Some(Source::Family)
            } else if friend(s.tie) {
                Some(Source::Friend)
            } else if s.captain {
                Some(Source::Room)
            } else if s.unit || s.coach {
                Some(Source::There)
            } else {
                None
            };
            if let Some(from) = from {
                v.insert(0, k(Topic::Mood, about, from, eye(&m, &s), mood_words(w, i, why)));
            }
            v
        }
        Topic::Wants => wants_known(w, i, about, &m, &s),
        Topic::Portal => {
            let mut v = from_sheet(w, him, i, topic, about, &m, &s);
            let from = if s.own {
                Some(Source::Himself)
            } else if s.family {
                Some(Source::Family)
            } else if friend(s.tie) {
                Some(Source::Friend)
            } else if w.people[i].cal.portal.is_some() {
                Some(Source::Public)
            } else if w.people[i].cal.whispered {
                Some(Source::Grapevine)
            } else {
                None
            };
            if let (Some(from), Some(words)) = (from, portal_words(w, i)) {
                v.push(k(Topic::Portal, about, from, eye(&m, &s), words));
            }
            v
        }
        Topic::RuleBreak => {
            let mut v = from_sheet(w, him, i, topic, about, &m, &s);
            v.extend(unseen(w, him, i, about, &m, &s));
            v
        }
        Topic::Injury => injury(w, him, i, about, &m, &s),
        Topic::Pitches => pitches(w, i, about, &m, &s),
        Topic::Nil => nil(w, i, about, &m, &s),
        Topic::Grades => grades(w, him, i, about, &m, &s),
        Topic::Character => character(w, him, i, about, &m, &s),
        Topic::Market => market(w, him, i, about, &m, &s),
        // Topics no player is the subject of.
        _ => Vec::new(),
    }
}

/// The day a fact could show to you: its sheet day, or a week on.
pub fn shows_on(k: &Known, today: Date) -> Option<Date> {
    match k.topic {
        // Whether he did it shows at his court day, if ever.
        Topic::Case => k.day.filter(|d| *d > today),
        // An injury he hides shows in the next game.
        Topic::Injury => Some(today.add(7)),
        // An inflated offer shows when he signs.
        Topic::Nil | Topic::Pitches | Topic::Market => k.day.filter(|d| *d > today).or(Some(today.add(60))),
        Topic::Portal => Some(today.add(30)),
        // A friend's secret, an unseen break: only if someone else tells.
        Topic::RuleBreak | Topic::Life | Topic::Mood => None,
        _ => Some(today.add(21)),
    }
}

/// Whether he knows anything at all on a topic about the man (for the ear's
/// card: "he'd know about this").
pub fn knows_any(w: &World, him: Them, topic: Topic, about: Option<Them>) -> bool {
    !knows(w, him, topic, about).is_empty()
}
