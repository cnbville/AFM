//! Talks off the game day (dialogue D7-D9, D11-D13, lane 14): a home
//! visit's talk with a recruit and the family member who decides (D7); a
//! combine interview as a timed talk (D8); the high school coach taking a
//! college staff's call about one of his players (D9); your own job
//! interviews (D11); the weekly staff meeting where a coordinator argues
//! for a change (D12); the players-only meeting a losing streak brings
//! (D13); agents in their own voice in contract talks (D10). What each
//! does comes from the people in it; the AI's head
//! coaches make the same home visits and their captains call the same
//! meetings. Sizes are Claude's picks (proposed, not confirmed).

use super::Team;
use super::cast::{self, Role as CastRole};
use super::scene::{self, Kind, Pick, Scene, Who};
use super::voice::{Facts, Pen, Speaker};
use crate::calendar::{Date, Event, Kind as CalKind, Level, Loudness, Weekday};
use crate::player::Trait;
use crate::world::coach::{Employer, Role};
use crate::world::program::GradeCat;
use crate::world::{Stage, World};

/// What these talks keep between days.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct State {
    /// Your word to a college staff about one of your players (D9).
    pub calls: Vec<CallWord>,
    /// Your job interviews: (employer, role, how it went, the day).
    pub interviews: Vec<(Employer, Role, f32, Date)>,
    /// The last staff meeting's case, checked after the next game (D12).
    pub case: Option<Case>,
    /// Players-only meetings: (team, day), the last per team kept.
    pub players_only: Vec<(Team, Date)>,
    /// The last week a college staff called you (D9).
    pub called: Option<Date>,
    /// The last game a staff meeting followed (D12).
    pub met: Option<Date>,
}

/// What you told a college staff about a player (D9).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CallWord {
    pub id: u32,
    pub program: usize,
    pub date: Date,
    /// 0 sold him, 1 straight, 2 protected him.
    pub word: u8,
    /// What the staff asked about that was a real worry: 0 none, 1 his
    /// grades, 2 his character.
    pub concern: u8,
    pub settled: bool,
}

/// A coordinator's case at the staff meeting (D12).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Case {
    pub coach: usize,
    /// 0 more pressure, 1 more zone, 2 run more, 3 throw more.
    pub kind: u8,
    /// 0 agreed, 1 halfway, 2 overruled.
    pub said: u8,
    pub date: Date,
    /// The unit's points (scored for the offense, allowed for the defense)
    /// in the game before.
    pub before: i32,
}

fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

fn z(v: f64) -> f64 {
    (v - 10.0) / 10.0
}

fn line(w: &mut World, s: &Scene, k: u64, key: &str, who: &Speaker, f: &Facts) -> String {
    let mut pen = scene::pen(w, s, k);
    let t = pen.part(key, who, f).map(|x| super::voice::tidy(&x)).unwrap_or_default();
    pen.put(w);
    t
}

// ---------------------------------------------------------------------------
// D7: the home visit's talk.

/// What a visit's pitch is about.
pub const PITCHES: [(&str, &str); 7] = [
    ("playing", "Playing time"),
    ("nil", "NIL money"),
    ("school", "The school"),
    ("distance", "Close to home"),
    ("family", "Family"),
    ("development", "Getting him to the next level"),
    ("winning", "Winning"),
];

/// The family's weight on each pitch, in shares of seven (a pitch they
/// care about as much as the rest is 1.0): their decider's wants, and how
/// much a parent or grandparent who decides wants to trust the man taking
/// their son (more where one parent or the grandparents raised him).
fn wants(p: &crate::world::Person) -> [f64; 7] {
    use crate::world::background::{Decider, FamilyStructure};
    let x = p.family.decider_wants;
    let v = [x.playing_time, x.money, x.academics, x.close_to_home, x.development, x.prestige];
    let s: f64 = v.iter().sum::<f64>().max(1e-9);
    let v = v.map(|a| 6.0 * a / s);
    let family = match p.family.decider {
        Decider::Himself => 0.5,
        Decider::Handler => 0.7,
        Decider::Parent | Decider::Grandparent => 1.0,
    } + match p.family.structure {
        FamilyStructure::SingleParent | FamilyStructure::Grandparents => 0.2,
        _ => 0.0,
    };
    [v[0], v[1], v[2], v[3], family, v[4], v[5]]
}

/// What the school can back each pitch up with, 0-1. The family pitch is
/// the head coach himself: how long he has been there, and how he is with
/// people.
fn backing(w: &World, i: usize, j: usize) -> [f64; 7] {
    let p = &w.people[i];
    let prog = &w.programs[j];
    let g = |c: GradeCat| prog.grades[c as usize] / 100.0;
    let nil = crate::world::recruiting::usual_nil_offer(w, j, i);
    let money = (nil / 1.0e6).powf(0.6).min(1.0).max(0.6 * g(GradeCat::Nil));
    let near = crate::world::recruiting::closeness(crate::world::geo::miles(p.home_state, prog.state));
    let family = prog.head_coach.map_or(0.3, |c| {
        let years = w.coaches[c].job.map_or(0, |jb| (w.year - jb.since).max(0)) as f64;
        (0.25 + 0.06 * years.min(8.0) + 0.02 * (w.coaches[c].ability().relationships - 10.0)).clamp(0.1, 1.0)
    });
    [
        g(GradeCat::PlayingTime),
        money,
        g(GradeCat::Academics),
        near,
        family,
        (g(GradeCat::ProPipeline) + g(GradeCat::CoachPrestige)) / 2.0,
        (g(GradeCat::TitleChances) + g(GradeCat::BrandExposure)) / 2.0,
    ]
}

/// How a pitch lands with a family, in appeal points (a visit's contact is
/// 0.004, an official visit 0.02): what they care about, what the school
/// can back up, and how well the man in the room reads and talks to
/// people. Overselling a weak point costs a little; listening first helps.
pub(crate) fn home_lean(w: &World, i: usize, j: usize, pitch: usize, coach: Option<usize>, listened: bool) -> f32 {
    let p = &w.people[i];
    let fit = wants(p)[pitch];
    let back = backing(w, i, j)[pitch];
    let a = coach.map(|c| *w.coaches[c].ability());
    let skill =
        (0.6 + 0.04 * a.map_or(0.0, |a| a.recruiting - 10.0) + 0.03 * a.map_or(0.0, |a| a.relationships - 10.0))
            .clamp(0.3, 1.5);
    let mut lean = 0.012 * skill * (fit * (0.3 + 0.7 * back) - 0.6);
    if back < 0.3 && fit > 1.0 {
        lean -= 0.006;
    }
    if listened {
        lean += 0.003;
    }
    lean.clamp(-0.012, 0.03) as f32
}

/// A head coach's own pick in a living room: what he reads they want and
/// the school can sell, read better the better he recruits.
fn coach_pitch(w: &World, i: usize, j: usize, coach: Option<usize>, seed: u64) -> usize {
    let wt = wants(&w.people[i]);
    let back = backing(w, i, j);
    let r = coach.map_or(10.0, |c| w.coaches[c].ability().recruiting);
    let sd = 0.6 * (1.6 - 0.06 * r).max(0.2);
    (0..PITCHES.len())
        .max_by(|&a, &b| {
            let s = |k: usize| wt[k] * (0.3 + 0.7 * back[k]) + sd * crate::game::talk::normal(seed, k as u64);
            s(a).total_cmp(&s(b))
        })
        .unwrap_or(0)
}

/// What the home visits' talks add to a school's pull on a recruit.
pub(crate) fn home_talk(p: &crate::world::Person, j: usize) -> f64 {
    p.cal.home_talks.iter().filter(|x| x.0 == j).map(|x| x.1 as f64).sum()
}

fn keep_lean(w: &mut World, i: usize, j: usize, lean: f32) {
    let v = &mut w.people[i].cal.home_talks;
    v.retain(|x| x.0 != j);
    v.push((j, lean));
}

/// The AI's head coach in a living room (contact periods): the same pitch
/// read the same way.
pub(crate) fn ai_home_visit(w: &mut World, i: usize, j: usize, d: Date) {
    let hc = w.programs[j].head_coach;
    let seed = w.options.seed as u64 ^ (d.0 as u64) << 20 ^ (j as u64) << 40 ^ w.people[i].player.id as u64;
    let k = coach_pitch(w, i, j, hc, seed);
    let lean = home_lean(w, i, j, k, hc, false);
    keep_lean(w, i, j, lean);
}

/// Your home visit is set: its talk waits on the day.
pub(crate) fn home_visit_set(w: &mut World, id: u32, j: usize, d: Date) {
    let Some(i) = super::person(w, id) else { return };
    let name = w.people[i].name.clone();
    let mut s = Scene::new(Kind::HomeVisit, d, format!("Home visit: {name}"));
    s.place = format!("The {} family's living room, {}", last_name(&name), w.high_schools[w.people[i].hs].town.clone());
    s.a = id as u64;
    s.b = j as u64;
    s.who = vec![Who::Player(id), Who::Group(decider_words(&w.people[i]).to_string())];
    scene::add(w, s);
}

/// A visit called off takes its talk with it.
pub(crate) fn home_visit_off(w: &mut World, id: u32, why: &str) {
    for s in w.folk.scenes.iter_mut().filter(|s| s.kind == Kind::HomeVisit && s.a == id as u64 && !s.done) {
        s.outcome.push(why.to_string());
        s.handed = Some("nobody".into());
        s.end();
    }
}

fn last_name(n: &str) -> &str {
    n.split_whitespace().last().unwrap_or(n)
}

fn decider_words(p: &crate::world::Person) -> &'static str {
    use crate::world::background::{Decider, FamilyStructure};
    match p.family.decider {
        Decider::Himself => "He decides himself",
        Decider::Grandparent => "His grandmother",
        Decider::Handler => "His trainer",
        Decider::Parent => match p.family.structure {
            FamilyStructure::SingleParent => "His mother",
            FamilyStructure::Grandparents => "His grandmother",
            FamilyStructure::Foster => "His guardian",
            FamilyStructure::TwoParents => "His parents",
        },
    }
}

fn decider_tag(p: &crate::world::Person) -> &'static str {
    use crate::world::background::Decider;
    match p.family.decider {
        Decider::Himself => "himself",
        Decider::Grandparent => "grandparent",
        Decider::Handler => "handler",
        Decider::Parent => "parent",
    }
}

fn top_wants(p: &crate::world::Person) -> (usize, usize) {
    let wt = wants(p);
    let mut k: Vec<usize> = (0..PITCHES.len()).collect();
    k.sort_by(|&a, &b| wt[b].total_cmp(&wt[a]));
    (k[0], k[1])
}

fn visit_open(w: &mut World, s: &mut Scene) {
    let Some(i) = super::person(w, s.a as u32) else { return s.end() };
    let p = &w.people[i];
    let kid = super::player_link(w, i);
    let who = decider_words(p);
    let (top, _) = top_wants(p);
    let mut sp = Speaker::parent(w, i);
    sp.tags.push(decider_tag(p));
    let f = Facts::new().with("name", p.name.split_whitespace().next().unwrap_or("").to_string());
    let ask = line(w, s, 1, &format!("homevisit.ask.{}", PITCHES[top].0), &sp, &f);
    let room = if who == "He decides himself" {
        format!("{kid} and his family sit across from you. He does the talking.")
    } else {
        format!("{kid} sits beside {}. {} does the talking.", who.to_lowercase(), who)
    };
    s.narrate(room);
    let name = if who == "He decides himself" { "Him".to_string() } else { who.to_string() };
    s.them(&name, ask, None);
    pitch_picks(s);
    s.pick(
        709,
        "\"Before I talk, tell me what matters most to you.\"",
        "Listen",
        "warm",
        "They tell you what they weigh.",
    );
}

fn pitch_picks(s: &mut Scene) {
    let say = [
        "\"He plays early here. Look at the depth chart.\"",
        "\"Let's talk about what our collective does for him.\"",
        "\"He'll leave here with a degree that means something.\"",
        "\"You'll be at every home game. It's a short drive.\"",
        "\"I'll look after him like he's my own. Ask my players' parents.\"",
        "\"We've sent his position to the league. Here's how.\"",
        "\"He'll play for championships here.\"",
    ];
    for (k, t) in say.iter().enumerate() {
        s.pick(701 + k as u32, *t, PITCHES[k].1, "warm", "");
    }
}

fn visit_answer(w: &mut World, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    let j = s.b as usize;
    let Some(i) = super::person(w, id) else { return s.end() };
    let p = &w.people[i];
    let who = decider_words(p);
    let name = if who == "He decides himself" { "Him".to_string() } else { who.to_string() };
    let mut sp = Speaker::parent(w, i);
    sp.tags.push(decider_tag(p));
    if code == 709 {
        let (a, b) = top_wants(p);
        let f = Facts::new().with("first", PITCHES[a].1.to_lowercase()).with("second", PITCHES[b].1.to_lowercase());
        let t = line(w, s, 2, "homevisit.tell", &sp, &f);
        s.you(&scene::your_name(w), "Before I talk, tell me what matters most to you.");
        s.them(&name, t, Some(format!("{} first, then {}.", PITCHES[a].1, PITCHES[b].1.to_lowercase())));
        s.c = 1;
        pitch_picks(s);
        return;
    }
    let k = (code.saturating_sub(701) as usize).min(PITCHES.len() - 1);
    let lean = home_lean(w, i, j, k, you(w), s.c == 1);
    keep_lean(w, i, j, lean);
    let mood = if lean > 0.012 {
        "good"
    } else if lean > 0.0 {
        "flat"
    } else {
        "bad"
    };
    let react = line(w, s, 3, &format!("homevisit.react.{mood}"), &sp, &Facts::new());
    s.them(&name, react, None);
    s.outcome.push(
        match mood {
            "good" => "They lean your way a little. The pitch was what they came for.",
            "flat" => "Polite. It landed, but it isn't what they weigh most.",
            _ => "It cooled the room: they don't believe you can back it up, or it isn't what they care about.",
        }
        .to_string(),
    );
    s.end();
}

fn visit_hand(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let j = s.b as usize;
    let Some(i) = super::person(w, id) else { return };
    // Your recruiting man goes: the position coach for his spot.
    let group = crate::world::coach::Group::of(w.people[i].player.position);
    let c = w.programs[j].position_coaches[group as usize].or(w.programs[j].head_coach);
    let k = coach_pitch(w, i, j, c, s.id as u64 ^ 0x7070);
    let lean = home_lean(w, i, j, k, c, false);
    keep_lean(w, i, j, lean);
    s.handed = Some(c.map_or("your staff".to_string(), |c| w.coaches[c].name.clone()));
    s.outcome.push(format!("He sold {}.", PITCHES[k].1.to_lowercase()));
}

// ---------------------------------------------------------------------------
// D9: a college staff calls the high school coach.

/// Mondays in the recruiting year: now and then a college staff that wants
/// one of your players calls you about him.
pub(crate) fn college_calls(w: &mut World, d: Date) {
    let Some(h) = crate::world::scouting::hs::school(w) else { return };
    if d.weekday() != Weekday::Wed {
        return;
    }
    let period = crate::calendar::recruiting_period(&w.dates, d);
    if matches!(period, crate::calendar::Period::Dead) {
        return;
    }
    if w.folk.dialogue.called.is_some_and(|c| c.days_to(d) < 13) {
        return;
    }
    let sign = w.dates.season + 1;
    let mut cands: Vec<(usize, usize)> = Vec::new();
    for &i in &w.high_schools[h].roster {
        let p = &w.people[i];
        if p.stage != Stage::HighSchool || p.cal.signed.is_some() || p.class_of > sign + 1 {
            continue;
        }
        if let Some(&j) = p.cal.offers.iter().find(|&&j| super::roll(w, 0xD9, (j as u64) << 32 ^ d.0 as u64) < 0.5) {
            cands.push((i, j));
        }
    }
    if cands.is_empty() || super::roll(w, 0xD9A, d.0 as u64) > 0.6 {
        return;
    }
    let (i, j) = cands[(super::roll(w, 0xD9B, d.0 as u64) * cands.len() as f64) as usize % cands.len()];
    w.folk.dialogue.called = Some(d);
    let name = w.people[i].name.clone();
    let mut s = Scene::new(Kind::CollegeCall, d, format!("{} calls about {name}", w.program_name(j)));
    s.stops = false;
    s.place = "Your office, between classes".into();
    s.a = w.people[i].player.id as u64;
    s.b = j as u64;
    s.c = concern(&w.people[i]) as i64;
    s.who = vec![Who::Player(w.people[i].player.id)];
    scene::add(w, s);
}

/// A real worry a college staff would ask about: 1 his grades, 2 his
/// character, 0 none.
fn concern(p: &crate::world::Person) -> u8 {
    let trouble = p.incident
        || p.player.history.iter().any(|h| {
            h.events.has(crate::player::DevelopmentEvent::Suspended)
                || h.events.has(crate::player::DevelopmentEvent::Dismissed)
        });
    if p.bg.gpa < 2.5 {
        1
    } else if trouble || p.player.personality.traits.get(Trait::Professionalism) <= 6.0 {
        2
    } else {
        0
    }
}

fn call_open(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let j = s.b as usize;
    let Some(i) = super::person(w, id) else { return s.end() };
    let kid = super::player_link(w, i);
    let school = w.program_name(j).to_string();
    let caller = w.programs[j].position_coaches[crate::world::coach::Group::of(w.people[i].player.position) as usize]
        .or(w.programs[j].head_coach);
    let (cname, sp) = match caller {
        Some(c) => (w.coaches[c].name.clone(), Speaker::coach(w, c)),
        None => (
            format!("{school}'s recruiting coordinator"),
            Speaker { key: 0xD9C0 ^ j as u64, tags: vec!["coach", "college"] },
        ),
    };
    if let Some(c) = caller {
        s.who.push(Who::Coach(c));
    }
    let about = ["football", "grades", "character"][s.c.clamp(0, 2) as usize];
    let f = Facts::new().with("name", w.people[i].name.split_whitespace().next().unwrap_or("").to_string());
    let ask = line(w, s, 1, &format!("hscall.ask.{about}"), &sp, &f);
    s.narrate(format!("{cname} from {school} is on the line about {kid}."));
    s.them(&cname, ask, None);
    let worry = s.c != 0;
    s.pick(
        901,
        "\"He's the best player I've coached. Take him.\"",
        "Sell him",
        "warm",
        "More staffs look at his film. If he busts, your word costs you with them.",
    );
    s.pick(
        902,
        if worry {
            "\"I'll be straight with you. Here's what you need to know.\""
        } else {
            "\"Here's who he is, the good and the work he has left.\""
        },
        "Be straight",
        "calm",
        "Your word holds its value with them, whatever he becomes.",
    );
    s.pick(
        903,
        if worry {
            "\"There's nothing there. He's a good kid.\""
        } else {
            "\"He's a good player. You've seen the film.\""
        },
        if worry { "Protect him" } else { "Keep it short" },
        "firm",
        if worry {
            "It may help him now. If it comes out later, they won't trust you again."
        } else {
            "Says little either way."
        },
    );
}

fn call_answer(w: &mut World, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    let j = s.b as usize;
    let word = (code.saturating_sub(901)).min(2) as u8;
    let concern = s.c.clamp(0, 2) as u8;
    // Selling and a straight word both reach his recruitment the way a
    // call you made would (scouting/hs.rs promos).
    let strength = match word {
        0 => 2,
        1 if concern == 0 => 1,
        1 => 0,
        _ => 1,
    };
    w.scouting.hs.promoted.push(crate::world::scouting::hs::Promo {
        id,
        program: Some(j),
        date: w.today,
        strength,
        settled: None,
        warmth: 0.0,
    });
    w.folk.dialogue.calls.push(CallWord { id, program: j, date: w.today, word, concern, settled: false });
    w.folk.mine.straight = (w.folk.mine.straight
        + if word == 1 {
            0.02
        } else if word == 2 && concern > 0 {
            -0.03
        } else {
            0.0
        })
    .clamp(-1.0, 1.0);
    s.you(
        &scene::your_name(w),
        s.picks.iter().find(|p| p.code == code).map_or(String::new(), |p| p.label.trim_matches('"').to_string()),
    );
    s.outcome.push(
        match (word, concern) {
            (0, _) => "You sold him. They'll look harder at his film.",
            (1, 0) => "You told them who he is. They'll weigh it with the film.",
            (1, _) => "You told them. They'll weigh it, and they'll remember you were straight.",
            (_, 0) => "You kept it short.",
            _ => "You covered for him. If it comes out, they'll remember that too.",
        }
        .to_string(),
    );
    s.end();
}

/// A year after each call: a worry you hid that came true costs you with
/// that staff; one you told them about earns your word some weight,
/// whatever he became.
pub(crate) fn settle_calls(w: &mut World, d: Date) {
    let mut changes: Vec<(usize, f64, usize)> = Vec::new();
    for (k, c) in w.folk.dialogue.calls.iter().enumerate() {
        if c.settled || c.date.add(365) > d {
            continue;
        }
        let Some(i) = super::person(w, c.id) else {
            changes.push((c.program, 0.0, k));
            continue;
        };
        let p = &w.people[i];
        let came_true = match c.concern {
            1 => p.cal.academic_sit || p.cal.spring_ineligible || p.cal.fall_ineligible || p.college.is_none(),
            2 => p.cal.trouble || p.incident,
            _ => false,
        };
        let delta = match (c.word, c.concern > 0, came_true) {
            (1, true, _) => 0.04,
            (2, true, true) => -0.10,
            (0, true, true) => -0.06,
            _ => 0.0,
        };
        changes.push((c.program, delta, k));
    }
    for (prog, delta, k) in changes {
        w.folk.dialogue.calls[k].settled = true;
        if delta != 0.0 {
            let now = crate::world::scouting::hs::credibility(w, prog);
            w.scouting.hs.credibility.insert(prog, (now + delta).clamp(0.05, 0.95) as f32);
        }
    }
    w.folk.dialogue.calls.retain(|c| !c.settled || c.date.add(800) > d);
}

// ---------------------------------------------------------------------------
// D11: your job interviews.

/// You put your name in for a head job whose search decides later: the
/// committee sits you down first.
/// A seat nobody holds sits you down today and fills when the talk is
/// over (`decides` None).
pub(crate) fn interview_set(w: &mut World, e: Employer, r: Role, decides: Option<Date>) -> Option<Date> {
    if r != Role::HeadCoach {
        return None;
    }
    let today = w.today;
    let d = decides.map_or(today, |x| today.add(2).min(x.add(-1)).max(today.add(1)));
    let team = Team::from_employer(e);
    let panel = panel(w, team);
    let mut s = Scene::new(
        Kind::JobInterview,
        d,
        format!("Interview: {}", crate::world::front_office::market::employer_name(w, e)),
    );
    s.team = Some(team);
    s.place = match team {
        Team::Nfl(_) | Team::Other(_) => "The owner's suite".into(),
        Team::College(_) => "A hotel conference room near the airport".into(),
        Team::High(_) => "The principal's office".into(),
    };
    s.a = employer_key(e);
    s.c = decides.is_none() as i64;
    s.who = panel.iter().map(|c| Who::Cast(c.r.token())).collect();
    scene::add(w, s);
    Some(d)
}

fn employer_key(e: Employer) -> u64 {
    match e {
        Employer::College(p) => p as u64,
        Employer::Nfl(t) => 1 << 32 | t as u64,
        Employer::HighSchool(h) => 2 << 32 | h as u64,
        Employer::Other(f) => 3 << 32 | f as u64,
    }
}

fn employer_of(k: u64) -> Employer {
    let i = (k & 0xFFFF_FFFF) as usize;
    match k >> 32 {
        0 => Employer::College(i),
        1 => Employer::Nfl(i),
        2 => Employer::HighSchool(i),
        _ => Employer::Other(i),
    }
}

/// The committee: a high school's principal and athletic director, a
/// college's athletic director and president, an NFL owner and his GM.
fn panel(w: &World, team: Team) -> Vec<cast::Cast> {
    let roles: &[CastRole] = match team {
        Team::High(_) => &[CastRole::Principal, CastRole::SchoolAd],
        Team::College(_) => &[CastRole::AthleticDirector, CastRole::President],
        Team::Nfl(_) | Team::Other(_) => &[CastRole::Owner, CastRole::Gm],
    };
    roles.iter().filter_map(|&r| cast::all(w, team, r).into_iter().next()).collect()
}

/// What a committee wants to hear, from the job and the people on it:
/// the plan (0 win now, 1 build it, 2 develop), the staff (0 bring your
/// own, 1 keep the good ones, 2 the best available) and the past (0 own a
/// failure, 1 point to results, 2 the people you made).
fn wants_of(w: &World, team: Team, panel: &[cast::Cast]) -> [u8; 3] {
    let patience = panel.iter().map(|c| c.patience).sum::<f64>() / panel.len().max(1) as f64;
    let warm = panel.iter().map(|c| c.warmth).sum::<f64>() / panel.len().max(1) as f64;
    let meddle = panel.iter().map(|c| c.meddling).sum::<f64>() / panel.len().max(1) as f64;
    let good = match team {
        Team::College(p) => w.programs[p].prestige_rel >= 0.6,
        Team::Nfl(t) => {
            let (wi, l, _) = w.nfl_record(t);
            wi >= l
        }
        _ => false,
    };
    let plan = if good && patience < 11.0 {
        0
    } else if matches!(team, Team::High(_)) || patience >= 13.0 {
        2
    } else {
        1
    };
    let staff = if meddle >= 13.0 {
        1
    } else if good {
        2
    } else {
        0
    };
    let past = if warm >= 13.0 {
        2
    } else if patience >= 12.0 {
        0
    } else {
        1
    };
    [plan, staff, past]
}

const QUESTIONS: [(&str, [&str; 3]); 3] = [
    (
        "plan",
        [
            "\"We win now. This roster can.\"",
            "\"We build it the right way, year by year.\"",
            "\"We find players and develop them.\"",
        ],
    ),
    (
        "staff",
        [
            "\"I'm bringing my own people.\"",
            "\"I'll keep the good ones you have.\"",
            "\"I'll hire the best coaches I can get.\"",
        ],
    ),
    (
        "past",
        [
            "\"Let me tell you about the year it went wrong.\"",
            "\"Look at what my teams did.\"",
            "\"Look at the people who came through my rooms.\"",
        ],
    ),
];

const LABELS: [[&str; 3]; 3] = [
    ["Win now", "Build it", "Develop"],
    ["Your own staff", "Keep the good ones", "The best available"],
    ["Own a failure", "Point to results", "The people"],
];

fn interview_ask(w: &mut World, s: &mut Scene) {
    let q = s.step.min(2) as usize;
    let team = s.team.unwrap_or(Team::from_employer(employer_of(s.a)));
    let panel = panel(w, team);
    let Some(c) = panel.get(q % panel.len().max(1)) else { return s.end() };
    let sp = Speaker::cast(w, c);
    let t = line(w, s, 10 + q as u64, &format!("interview.ask.{}", QUESTIONS[q].0), &sp, &Facts::new());
    s.them(&c.name, t, None);
    for (k, a) in QUESTIONS[q].1.iter().enumerate() {
        s.pick(931 + 3 * q as u32 + k as u32, *a, LABELS[q][k], "calm", "");
    }
}

fn interview_open(w: &mut World, s: &mut Scene) {
    let team = s.team.unwrap_or(Team::from_employer(employer_of(s.a)));
    let names: Vec<String> =
        panel(w, team).iter().map(|c| format!("{}, {}", c.link(), c.title.to_lowercase())).collect();
    s.narrate(format!("Across the table: {}.", names.join(" and ")));
    interview_ask(w, s);
}

fn interview_answer(w: &mut World, s: &mut Scene, code: u32) {
    let q = ((code - 931) / 3) as usize;
    let k = ((code - 931) % 3) as u8;
    let team = s.team.unwrap_or(Team::from_employer(employer_of(s.a)));
    let panel = panel(w, team);
    let want = wants_of(w, team, &panel);
    let aura = w.career.as_ref().map_or(10.0, |c| c.you.aura());
    // Each answer: on what they want, a miss, or between; how you come
    // across (Aura) carries every answer a little.
    let hit = if k == want[q] {
        0.45
    } else if (k as i16 - want[q] as i16).abs() == 1 {
        0.0
    } else {
        -0.3
    };
    s.f[q] = (hit + 0.015 * (aura - 10.0)) as f32;
    s.you(&scene::your_name(w), QUESTIONS[q].1[k as usize].trim_matches('"'));
    if let Some(c) = panel.get(q % panel.len().max(1)) {
        let sp = Speaker::cast(w, c);
        let mood = if hit > 0.0 {
            "good"
        } else if hit < 0.0 {
            "bad"
        } else {
            "flat"
        };
        let t = line(w, s, 20 + q as u64, &format!("interview.react.{mood}"), &sp, &Facts::new());
        s.them(&c.name, t, None);
    }
    if q < 2 {
        s.step = (q + 1) as u8;
        interview_ask(w, s);
        s.step = q as u8;
        return;
    }
    let score = s.f[0] + s.f[1] + s.f[2];
    let e = employer_of(s.a);
    w.folk.dialogue.interviews.retain(|x| !(x.0 == e && x.1 == Role::HeadCoach));
    w.folk.dialogue.interviews.push((e, Role::HeadCoach, score, w.today));
    s.outcome.push(
        if score > 0.6 {
            "They liked what they heard. You'll be on the short list."
        } else if score > -0.2 {
            "A fair interview. It's up to the rest of your record now."
        } else {
            "It didn't go your way. They wanted to hear something else."
        }
        .to_string(),
    );
    s.end();
    if s.c == 1 {
        fill_after(w, s, e);
    }
}

fn interview_hand(w: &mut World, s: &mut Scene) {
    let e = employer_of(s.a);
    w.folk.dialogue.interviews.retain(|x| !(x.0 == e && x.1 == Role::HeadCoach));
    w.folk.dialogue.interviews.push((e, Role::HeadCoach, -1.0, w.today));
    s.handed = Some("nobody".into());
    s.outcome.push("You missed the interview. The committee noticed.".into());
    if s.c == 1 {
        fill_after(w, s, e);
    }
}

/// A seat nobody holds is filled once you have sat down with them: by you,
/// or by the best name they have.
fn fill_after(w: &mut World, s: &mut Scene, e: Employer) {
    use crate::world::front_office::market;
    let prestige = crate::world::staff::prestige_of(w, e);
    crate::world::staff::fill_now(w, e, Role::HeadCoach, prestige);
    let name = market::employer_name(w, e);
    let offered =
        w.career.as_ref().is_some_and(|c| c.market.offers.iter().any(|o| o.employer == e && o.role == Role::HeadCoach));
    s.outcome.push(if offered {
        format!("{name} wants you. Their offer is on your job market page.")
    } else {
        match crate::world::staff::holder(w, e, Role::HeadCoach) {
            Some(h) => format!("{name} went with {}.", w.coaches[h].name),
            None => format!("{name} didn't find its man yet."),
        }
    });
}

/// What your interview adds to a committee's read of you (market `bonus`).
pub(crate) fn interview_bonus(w: &World, e: Employer, r: Role) -> f64 {
    w.folk
        .dialogue
        .interviews
        .iter()
        .filter(|x| x.0 == e && x.1 == r && x.3.days_to(w.today) <= 120)
        .map(|x| x.2 as f64)
        .sum()
}

// ---------------------------------------------------------------------------
// D12: the weekly staff meeting.

/// The case a coordinator brings after a game, from what he saw in it:
/// the coordinator, the case and his unit's points.
fn case_from(w: &World, g: &crate::world::seat::SeatGame, me: usize) -> Option<(usize, u8, i32)> {
    let team = Team::seat(w)?;
    let [oc, dc] = super::gametalk::coordinators(w, team);
    let side = if g.teams[0] == me { 0 } else { 1 };
    let them = 1 - side;
    let st = &g.stats;
    let ours = &st.team[side];
    let theirs = &st.team[them];
    let carries = ours.plays.saturating_sub(ours.dropbacks).max(1) as f64;
    let ypc = ours.rush_yards / carries;
    let run_share = carries / ours.plays.max(1) as f64;
    let pressure = theirs.pressures as f64 / theirs.dropbacks.max(1) as f64;
    let big_allowed = theirs.explosive;
    let allowed = st.score[them];
    let scored = st.score[side];
    let bad_d = allowed >= 28;
    if bad_d && let Some(c) = dc {
        return Some(if pressure < 0.28 {
            (c, 0, allowed)
        } else if big_allowed >= 6 {
            (c, 1, allowed)
        } else {
            (c, 0, allowed)
        });
    }
    if let Some(c) = oc {
        if ypc >= 5.0 && run_share < 0.48 {
            return Some((c, 2, scored));
        }
        if ypc < 3.3 && run_share > 0.45 {
            return Some((c, 3, scored));
        }
    }
    None
}

/// The morning after each of your games the staff meets (college on
/// Sunday, the NFL on Monday, a high school on Saturday): your
/// coordinator's case waits for you, and the last case is checked against
/// the game.
pub(crate) fn staff_meeting_day(w: &mut World, d: Date) {
    if w.career.as_ref().is_some_and(|c| c.role.is_some()) {
        return;
    }
    let Some((_, me)) = w.seat_team() else { return };
    let Some(g) = w.seat_games.last().filter(|g| g.date <= d && g.date.days_to(d) <= 1) else { return };
    if w.folk.dialogue.met.is_some_and(|m| m >= g.date) {
        return;
    }
    let g = g.clone();
    w.folk.dialogue.met = Some(g.date);
    check_case(w, &g, me);
    let Some((coach, kind, before)) = case_from(w, &g, me) else { return };
    let date = d.add(1);
    let role = if kind <= 1 { "DC" } else { "OC" };
    let mut s = Scene::new(Kind::StaffMeeting, date, format!("Staff meeting: your {role} wants a change"));
    s.place = "The staff room, 7:00 a.m.".into();
    s.a = coach as u64;
    s.b = kind as u64;
    s.c = before as i64;
    s.who = vec![Who::Coach(coach)];
    scene::add(w, s);
}

const CASES: [(&str, &str); 4] = [
    ("pressure", "more pressure"),
    ("zone", "more zone coverage"),
    ("run", "running it more"),
    ("pass", "throwing it more"),
];

fn meeting_open(w: &mut World, s: &mut Scene) {
    let c = s.a as usize;
    let kind = (s.b as usize).min(3);
    let sp = Speaker::coach(w, c);
    let name = w.coaches[c].name.clone();
    let t = line(w, s, 1, &format!("staffmeet.case.{}", CASES[kind].0), &sp, &Facts::new());
    s.narrate(format!("{} has the floor first, as he asked.", super::coach_link(w, c)));
    s.them(&name, t, None);
    s.pick(
        951,
        format!("\"Do it. {} this week.\"", cap(CASES[kind].1)),
        "Agree",
        "calm",
        "The plan's guideline moves to his number. He owns it if it goes wrong.",
    );
    s.pick(
        952,
        "\"Some of it. Meet me halfway.\"",
        "Halfway",
        "calm",
        "Half the change. Nobody's happy, nobody's wrong.",
    );
    s.pick(
        953,
        "\"No. We stay with the plan.\"",
        "Overrule",
        "firm",
        "If his unit struggles again, he'll remember you said no.",
    );
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

fn meeting_answer(w: &mut World, s: &mut Scene, code: u32) {
    let c = s.a as usize;
    let kind = (s.b as u8).min(3);
    let said = (code.saturating_sub(951)).min(2) as u8;
    if said < 2 {
        shift_plan(w, kind, if said == 0 { 1.0 } else { 0.5 });
    }
    w.folk.dialogue.case = Some(Case { coach: c, kind, said, date: w.today, before: s.c as i32 });
    let sp = Speaker::coach(w, c);
    let mood = ["yes", "half", "no"][said as usize];
    let t = line(w, s, 2, &format!("staffmeet.reply.{mood}"), &sp, &Facts::new());
    s.them(&w.coaches[c].name.clone(), t, None);
    s.outcome.push(match said {
        0 => format!("You agreed: {} this week.", CASES[kind as usize].1),
        1 => format!("Halfway: a little {} this week.", CASES[kind as usize].1.trim_start_matches("more ")),
        _ => "You overruled him. The plan stands.".into(),
    });
    s.end();
}

fn meeting_hand(w: &mut World, s: &mut Scene) {
    // Your assistant head coach runs the meeting: the coordinator gets
    // half of what he asked.
    let kind = (s.b as u8).min(3);
    shift_plan(w, kind, 0.5);
    w.folk.dialogue.case = Some(Case { coach: s.a as usize, kind, said: 1, date: w.today, before: s.c as i32 });
    s.handed = Some("your assistant head coach".into());
    s.outcome.push("Half the change went in.".into());
}

/// Moves the game plan's guideline toward the coordinator's case.
fn shift_plan(w: &mut World, kind: u8, k: f64) {
    use crate::game::guide::{Firm, Want};
    let g = &mut w.plans.guide;
    let firm = Firm::default();
    match kind {
        0 => {
            let now = g.pressure.to.unwrap_or(0.3);
            g.pressure = Want::new((now + 0.08 * k).clamp(0.05, 0.7), g.pressure.to.map_or(firm, |_| g.pressure.firm));
        }
        1 => {
            let now = g.man.to.unwrap_or(0.4);
            g.man = Want::new((now - 0.1 * k).clamp(0.05, 0.9), g.man.to.map_or(firm, |_| g.man.firm));
        }
        2 => {
            let now = g.run.to.unwrap_or(0.44);
            g.run = Want::new((now + 0.06 * k).clamp(0.2, 0.75), g.run.to.map_or(firm, |_| g.run.firm));
        }
        _ => {
            let now = g.run.to.unwrap_or(0.44);
            g.run = Want::new((now - 0.06 * k).clamp(0.2, 0.75), g.run.to.map_or(firm, |_| g.run.firm));
        }
    }
    w.plans.guide_set = Some(w.today);
}

/// Last week's case, checked against the game since: overruled and his
/// unit struggled again costs his trust in you; agreed and it worked earns
/// a little.
fn check_case(w: &mut World, g: &crate::world::seat::SeatGame, me: usize) {
    let Some(case) = w.folk.dialogue.case.take() else { return };
    if g.date <= case.date {
        w.folk.dialogue.case = Some(case);
        return;
    }
    let side = if g.teams[0] == me { 0 } else { 1 };
    let pts = if case.kind <= 1 { g.stats.score[1 - side] } else { g.stats.score[side] };
    let defense = case.kind <= 1;
    let bad = if defense { pts >= 28 } else { pts < 17 };
    let good = if defense { pts <= 17 } else { pts >= 28 };
    let by = match (case.said, bad, good) {
        (2, true, _) => -0.06,
        (2, _, true) => 0.01,
        (0, _, true) => 0.03,
        (0, true, _) => -0.01,
        _ => 0.0,
    };
    if by != 0.0 {
        w.folk.mine.nudge(super::coach_key(case.coach), by);
    }
    if case.said == 2 && bad {
        let name = w.coaches[case.coach].name.clone();
        super::note(
            w,
            "staff",
            "Staff meeting".into(),
            format!("{name} hasn't forgotten you overruled him. His unit struggled again."),
            None,
        );
    }
}

// ---------------------------------------------------------------------------
// D13: the players-only meeting.

/// Mondays: a team on a losing streak (three in a row) may hold a
/// players-only meeting, called by its captains; what it does to the room
/// comes from them. Every college and NFL team, and yours at any level.
pub(crate) fn players_only_day(w: &mut World, d: Date) {
    if d.weekday() != Weekday::Mon {
        return;
    }
    let mut teams: Vec<(Team, Vec<usize>)> = Vec::new();
    if let Some(cs) = w.college.as_ref() {
        let mut lost: std::collections::BTreeMap<usize, Vec<(Date, bool)>> = std::collections::BTreeMap::new();
        for g in cs.games.iter().filter(|g| g.date < d && g.date.days_to(d) <= 40) {
            let Some(p) = g.played.filter(|p| !p.cancelled) else { continue };
            lost.entry(g.home).or_default().push((g.date, p.score[0] < p.score[1]));
            lost.entry(g.away).or_default().push((g.date, p.score[1] < p.score[0]));
        }
        for (t, mut v) in lost {
            v.sort_by_key(|x| x.0);
            if v.len() >= 3 && v.iter().rev().take(3).all(|x| x.1) && v.last().is_some_and(|x| x.0.days_to(d) <= 3) {
                teams.push((Team::College(t), w.programs[t].captains.clone()));
            }
        }
    }
    if w.nfl_season.is_some() {
        let mut lost: std::collections::BTreeMap<usize, Vec<(Date, bool)>> = std::collections::BTreeMap::new();
        for f in w.nfl_fixtures(None) {
            let (Some(when), Some(sc)) = (f.when, f.score) else { continue };
            if when >= d || when.days_to(d) > 30 {
                continue;
            }
            lost.entry(f.home).or_default().push((when, sc[0] < sc[1]));
            lost.entry(f.away).or_default().push((when, sc[1] < sc[0]));
        }
        for (t, mut v) in lost {
            v.sort_by_key(|x| x.0);
            if v.len() >= 3 && v.iter().rev().take(3).all(|x| x.1) && v.last().is_some_and(|x| x.0.days_to(d) <= 2) {
                let caps = w.clubs.get(t).map(|c| c.captains.clone()).unwrap_or_default();
                teams.push((Team::Nfl(t), caps));
            }
        }
    }
    // Your high school (or other league) team, from your own games.
    if let Some(seat) = Team::seat(w)
        && matches!(seat, Team::High(_) | Team::Other(_))
    {
        let me = match seat {
            Team::High(h) | Team::Other(h) => h,
            _ => 0,
        };
        let last: Vec<bool> = w
            .seat_games
            .iter()
            .rev()
            .take(3)
            .filter(|g| g.date.days_to(d) <= 30)
            .map(|g| {
                let s = if g.teams[0] == me { 0 } else { 1 };
                g.stats.score[s] < g.stats.score[1 - s]
            })
            .collect();
        if last.len() == 3 && last.iter().all(|&x| x) {
            teams.push((seat, Vec::new()));
        }
    }
    for (team, caps) in teams {
        if w.folk.dialogue.players_only.iter().any(|x| x.0 == team && x.1.days_to(d) < 21) {
            continue;
        }
        players_only(w, team, caps, d);
    }
}

fn players_only(w: &mut World, team: Team, caps: Vec<usize>, d: Date) {
    let roster = team.roster(w);
    let caps: Vec<usize> = if caps.is_empty() {
        // The natural leaders stand in: leadership, then years.
        let mut v = roster.clone();
        v.sort_by(|&a, &b| {
            let s = |i: usize| {
                w.people[i].player.personality.traits.get(Trait::Leadership) + 0.3 * w.people[i].player.age as f64
            };
            s(b).total_cmp(&s(a))
        });
        v.truncate(2);
        v
    } else {
        caps
    };
    if caps.is_empty() {
        return;
    }
    let t = |i: usize, k: Trait| w.people[i].player.personality.traits.get(k);
    let lead = caps.iter().map(|&i| t(i, Trait::Leadership)).fold(0.0, f64::max);
    let pro = caps.iter().map(|&i| t(i, Trait::Professionalism)).sum::<f64>() / caps.len() as f64;
    let ego = caps.iter().map(|&i| t(i, Trait::Ego)).fold(0.0, f64::max);
    let temper = caps.iter().map(|&i| t(i, Trait::Temper)).fold(0.0, f64::max);
    let key = match team {
        Team::College(p) => p as u64,
        Team::Nfl(t) => 1 << 32 | t as u64,
        Team::High(h) => 2 << 32 | h as u64,
        Team::Other(f) => 3 << 32 | f as u64,
    };
    // They call one if their leaders are the kind who would.
    if super::roll(w, 0xD13, key ^ (d.0 as u64) << 36) > (0.2 + 0.05 * (lead - 10.0)).clamp(0.05, 0.6) {
        return;
    }
    let noise = 0.03 * crate::game::talk::normal(w.options.seed as u64 ^ 0xD130, key ^ (d.0 as u64) << 36);
    let heated = ego >= 15.0 && temper >= 13.0;
    let delta =
        (0.06 * z(lead) + 0.03 * z(pro) - if heated { 0.06 } else { 0.0 } + 0.02 + noise).clamp(-0.08, 0.1) as f32;
    w.folk.dialogue.players_only.retain(|x| x.0 != team);
    w.folk.dialogue.players_only.push((team, d));
    let yours = Team::seat(w) == Some(team);
    let ids: Vec<u32> = roster.iter().map(|&i| w.people[i].player.id).collect();
    if yours {
        for id in ids {
            super::talks::mood_event(w, id, super::talks::Reason::Room, delta);
        }
    } else {
        for &i in &roster {
            w.people[i].mood = (w.people[i].mood + delta).clamp(-1.0, 1.0);
        }
    }
    // Word gets out: to you through your staff, sometimes to the papers.
    let name = team.name(w);
    let cap = w.people[caps[0]].name.clone();
    let how = if delta > 0.03 {
        "Word is it cleared the air."
    } else if delta < -0.02 {
        "Word is it got heated."
    } else {
        "Nobody is saying much about it."
    };
    let leak = super::roll(w, 0xD131, key ^ (d.0 as u64) << 36) < if delta < 0.0 { 0.5 } else { 0.3 };
    if yours {
        // You weren't in the room: a staff member hears it from a player,
        // unless the papers have it first.
        let text = if leak {
            format!(
                "You read it before anyone told you: your captains, {cap} first among them, held a players-only meeting after the third loss. {how}"
            )
        } else {
            format!(
                "Your captains, {cap} first among them, held a players-only meeting after the third loss. A position coach heard about it from a player. {how}"
            )
        };
        super::note(w, "talk", "Players-only meeting".into(), text, None);
    }
    let loud = matches!(team, Team::Nfl(_))
        || matches!(team, Team::College(p) if w.programs[p].division == crate::world::Division::FBS)
        || yours;
    if leak && loud {
        let level = match team {
            Team::Nfl(_) | Team::Other(_) => Level::Nfl,
            Team::High(_) => Level::HighSchool,
            Team::College(_) => Level::College,
        };
        let mut e = Event::new(
            d,
            level,
            CalKind::News,
            Loudness::Small,
            format!("Sources: {name} held a players-only meeting after three straight losses. {how}"),
        );
        match team {
            Team::College(p) => e = e.program(p),
            Team::Nfl(t) => e = e.team(t),
            _ => {}
        }
        w.calendar.post(e);
    }
}

// ---------------------------------------------------------------------------
// D8: the combine interview as a timed talk.

/// The minutes each topic takes, and a follow-up.
/// Minutes each topic takes in an 18-minute combine room; a longer room
/// (a top-30 visit, an official visit's sit-down) goes deeper on each.
const COMBINE_COST: [u8; 5] = [6, 4, 4, 3, 3];

fn topic_minutes(s: &Scene, k: usize) -> u8 {
    ((COMBINE_COST[k] as f32 * s.f[0] / 18.0).round() as u8).clamp(1, 60)
}

fn press_cost(s: &Scene) -> u8 {
    ((2.0 * s.f[0] / 18.0).round() as u8).clamp(2, 10)
}

/// Opens a timed interview with a player (an NFL seat's combine
/// interview, or any interview room open with him today).
pub fn start_combine_talk(w: &mut World, id: u32, room: &str) -> Result<u32, String> {
    let rooms = crate::world::scouting::own::rooms(w, id);
    let (room, minutes) = rooms.into_iter().find(|r| r.0 == room).ok_or("That room isn't open with him today.")?;
    let i = super::person(w, id).ok_or("No such player.")?;
    let name = w.people[i].name.clone();
    let mut s = Scene::new(Kind::CombineTalk, w.today, format!("{room}: {name}"));
    s.stops = false;
    s.place = if room.starts_with("Combine") { "A hotel suite in Indianapolis".into() } else { "Your office".into() };
    s.a = id as u64;
    s.c = minutes as i64;
    s.f[0] = minutes as f32;
    s.who = vec![Who::Player(id)];
    s.outcome.clear();
    // The room's name rides in the scene's title; the minutes in `c`.
    Ok(scene::add(w, s))
}

fn spent_of(s: &Scene) -> [u8; 5] {
    let mut v = [0u8; 5];
    for (k, x) in v.iter_mut().enumerate() {
        *x = ((s.b >> (6 * k)) & 0x3F) as u8;
    }
    v
}

fn set_spent(s: &mut Scene, v: [u8; 5]) {
    s.b = v.iter().enumerate().fold(0u64, |a, (k, &x)| a | (x.min(63) as u64) << (6 * k));
}

fn combine_picks(s: &mut Scene) {
    let left = s.c.max(0) as u8;
    let spent = spent_of(s);
    for (k, t) in crate::world::scouting::own::TOPICS.iter().enumerate() {
        let cost = topic_minutes(s, k);
        if cost <= left && spent[k] == 0 {
            s.pick(961 + k as u32, format!("{} ({cost} min)", t.words()), t.words(), "calm", "");
        }
    }
    let press = press_cost(s);
    if s.f[1] > 0.0 && left >= press {
        s.pick(
            967,
            format!("\"Stay on that. Why?\" ({press} min)"),
            "Press him",
            "firm",
            "A follow-up shows how he handles being pushed.",
        );
    }
    s.pick(969, "\"That's our time. Thanks for coming in.\"", "Wrap it up", "calm", "");
}

fn combine_open(w: &mut World, s: &mut Scene) {
    let Some(i) = super::person(w, s.a as u32) else { return s.end() };
    let kid = super::player_link(w, i);
    s.narrate(format!("{kid} sits down. The clock on the wall says {} minutes.", s.c));
    combine_picks(s);
}

fn combine_answer(w: &mut World, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else { return s.end() };
    let name = w.people[i].name.clone();
    if code == 969 || s.c <= 0 {
        return combine_close(w, s);
    }
    if code == 967 {
        // The follow-up: pushed, a composed man answers, a hot one bristles.
        let p = &w.people[i];
        let comp = p.player.attributes.get(crate::attributes::Attr::Composure);
        let temper = p.player.personality.traits.get(Trait::Temper);
        let sp = Speaker::player(w, i);
        let mood = if comp >= 60.0 && temper < 13.0 {
            "steady"
        } else if temper >= 14.0 {
            "bristles"
        } else {
            "shaken"
        };
        let t = line(w, s, 30 + s.step as u64, &format!("combine.press.{mood}"), &sp, &Facts::new());
        let tell = match mood {
            "steady" => "He held your eyes and didn't hurry.",
            "bristles" => "His jaw set. He leaned back.",
            _ => "He looked at the table and started over.",
        };
        s.you(&scene::your_name(w), "Stay on that. Why?");
        s.them(&name, t, Some(tell.into()));
        let press = press_cost(s);
        s.c -= press as i64;
        s.f[1] = 0.0;
        let mut v = spent_of(s);
        let k = s.f[2] as usize;
        v[k.min(4)] = v[k.min(4)].saturating_add(press).min(63);
        set_spent(s, v);
    } else {
        let k = ((code - 961) as usize).min(4);
        let t = crate::world::scouting::own::TOPICS[k];
        let cost = topic_minutes(s, k);
        let mut v = spent_of(s);
        v[k] = cost;
        set_spent(s, v);
        s.c -= cost as i64;
        s.you(&scene::your_name(w), t.words());
        let answer = crate::world::scouting::own::answer_line(w, i, t);
        s.them(&name, answer, None);
        // A follow-up fits the harder questions.
        s.f[1] = if matches!(k, 1 | 2) { 1.0 } else { 0.0 };
        s.f[2] = k as f32;
    }
    if s.c <= 0 {
        s.narrate("A knock on the door: time.");
        return combine_close(w, s);
    }
    s.narrate(format!("{} minutes left.", s.c));
    combine_picks(s);
}

fn combine_close(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let spent: Vec<(crate::world::scouting::own::Topic, u8)> =
        crate::world::scouting::own::TOPICS.iter().zip(spent_of(s)).filter(|x| x.1 > 0).map(|(t, m)| (*t, m)).collect();
    let room = s.title.split(':').next().unwrap_or("Combine interview").to_string();
    if spent.is_empty() {
        s.outcome.push("You let him go without a question.".into());
    } else {
        match crate::world::scouting::own::interview(w, id, &room, spent) {
            Ok(t) => s.outcome.push(format!("{t} His answers are in his file.")),
            Err(e) => s.outcome.push(e),
        }
    }
    s.end();
}

// ---------------------------------------------------------------------------
// D10: agents in their own voice.

/// An agent's temper, 1-20: a steady draw per agent (the market keeps his
/// honesty, not his temper).
pub fn agent_temper(w: &World, a: usize) -> f64 {
    (10.0 + 3.5 * crate::world::season::key_normal(w.options.seed as u64 ^ 0xA6E7_7E30, a as u64)).clamp(1.0, 20.0)
}

fn agent_speaker(w: &World, a: usize) -> Speaker {
    let ag = &w.market.agents[a];
    let mut tags: Vec<&'static str> = vec!["agent"];
    if ag.honesty >= 0.75 {
        tags.push("straight");
    } else if ag.honesty < 0.45 {
        tags.push("slick");
    }
    let t = agent_temper(w, a);
    if t >= 14.0 {
        tags.push("hot");
    } else if t <= 7.0 {
        tags.push("calm");
    }
    match ag.size {
        3 => tags.push("big"),
        1 => tags.push("small"),
        _ => {}
    }
    Speaker { key: 0xA6E0_0000 ^ a as u64, tags }
}

/// What a player's agent says in a contract talk (D10): the market decides
/// the part (in range, apart, a bluff) and the numbers; the words are his,
/// from his honesty and temper. A bluff from an honest man comes out
/// forced; a slick one's comes out smooth, and vague all the same. With no
/// agent, or no line, the plain words stand.
pub fn agent_says(w: &World, i: usize, part: &str, facts: &Facts, plain: String) -> String {
    let Some(a) = w.people[i].pro.as_ref().and_then(|c| c.agent).filter(|&a| a < w.market.agents.len()) else {
        return plain;
    };
    let who = agent_speaker(w, a);
    let salt = part.bytes().fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
    let mut pen = Pen::loose(w.today.0 as u64 ^ (w.people[i].player.id as u64) << 24 ^ salt << 40);
    match pen.part(&format!("agent.{part}"), &who, facts).map(|x| super::voice::tidy(&x)) {
        Some(t) if !t.is_empty() => format!("\"{t}\""),
        _ => plain,
    }
}

// ---------------------------------------------------------------------------
// The scenes' dispatch (scene.rs).

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::HomeVisit => visit_open(w, s),
        Kind::CollegeCall => call_open(w, s),
        Kind::JobInterview => interview_open(w, s),
        Kind::StaffMeeting => meeting_open(w, s),
        Kind::CombineTalk => combine_open(w, s),
        _ => s.end(),
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, _picked: &Pick) {
    match s.kind {
        Kind::HomeVisit => visit_answer(w, s, code),
        Kind::CollegeCall => call_answer(w, s, code),
        Kind::JobInterview => interview_answer(w, s, code),
        Kind::StaffMeeting => meeting_answer(w, s, code),
        Kind::CombineTalk => combine_answer(w, s, code),
        _ => s.end(),
    }
}

pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::HomeVisit => visit_hand(w, s),
        Kind::CollegeCall => {
            s.handed = Some("your voicemail".into());
            s.outcome.push("You never called back. They'll go on the film.".into());
        }
        Kind::JobInterview => interview_hand(w, s),
        Kind::StaffMeeting => meeting_hand(w, s),
        Kind::CombineTalk => combine_close(w, s),
        _ => {}
    }
}
