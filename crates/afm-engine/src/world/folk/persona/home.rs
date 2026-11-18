//! His life away from football (personal talk, `docs/design/personal-talk.md`,
//! agreed 2026-11-17): what he can tell you about home, in three layers
//! (the surface, how he feels about home, his troubles), how open he is
//! and how deep he goes with you.
//!
//! What the world stores about his home comes first: who raised him, who
//! makes his calls, an older brother in college, a father who played, how
//! his folks show up at games, a mentor, his own wife and children, his
//! story. What it doesn't store (a parent's name and job, younger brothers
//! and sisters, his major, a girlfriend, his day off) is drawn here from his
//! background, keyed to him, the same every time and never against what is
//! stored, the way coaches' families are (`life::family`). Drawn details are
//! flavour only: nothing reads them but the talk.
//!
//! Every size is proposed, not confirmed.

use super::Them;
use super::makeup;
use crate::rng::Rng;
use crate::world::background::{Bloodline, Decider, FamilyStructure as F, ParentStyle};
use crate::world::ethnicity::Ethnicity;
use crate::world::folk::{self as folk, bonds, cast, facts, life};
use crate::world::{Stage, World};

/// How deep a thing about his life is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Layer {
    /// Shared with nearly anyone: home, family, school, his day off.
    Surface,
    /// How he feels about home: missing it, who he does it for.
    Feelings,
    /// His troubles: a loss, money at home, a hard childhood.
    Troubles,
}

/// One thing about his life he could tell you.
#[derive(Clone, Debug, PartialEq)]
pub struct Bit {
    /// Its kind, the same for him every time (what he's told you is kept
    /// by it).
    pub key: u16,
    pub layer: Layer,
    /// In the card's words: "He ..." is the man himself.
    pub words: String,
    /// The words of a question that ask for it.
    pub tags: &'static [&'static str],
}

/// A personal answer: what he says, what's new, and how the question went.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Said {
    pub released: Vec<String>,
    /// The things told for the first time, by key, with their words.
    pub new: Vec<(u16, String)>,
    /// The question asked again what he already told you.
    pub again: bool,
}

/// Openness at or above which he goes one layer deeper before he trusts
/// you, and at or below which one shallower.
pub const OPEN_DEEPER: f64 = 1.15;
pub const OPEN_SHALLOWER: f64 = 0.8;

/// Trust x openness bars for anyone without a bond: under the first the
/// surface, under the second how he feels, else his troubles.
pub const TRUST_FEELINGS: f64 = 0.5;
pub const TRUST_TROUBLES: f64 = 0.65;

/// A want weighing this much still comes up after a personal answer.
pub const PRESSING: f32 = 0.5;

/// The most things he tells you about his life a person, both ways together.
pub const KEEP_SHARED: usize = 12;

const MOM: &[&str] = &["mom", "mother", "mama", "momma", "ma", "parents", "folks"];
const DAD: &[&str] = &["dad", "father", "pops", "pop", "daddy", "parents", "folks"];
const PARENTS: &[&str] = &["mom", "mother", "mama", "dad", "father", "parents", "folks", "raised"];
const GRAND: &[&str] = &["grandma", "grandmother", "granny", "grandparents", "nana", "folks", "raised"];
const SIBS: &[&str] = &["brother", "brothers", "sister", "sisters", "siblings"];
const HOMETOWN: &[&str] = &["hometown", "from", "town", "grew"];
const SCHOOL: &[&str] = &["school", "class", "classes", "major", "study", "studying", "studies", "degree"];
const DAY_OFF: &[&str] = &["off", "weekend", "weekends", "fun", "free", "hobby", "hobbies", "relax", "downtime"];
const LOVE: &[&str] = &["girlfriend", "girl", "dating", "seeing"];
const OWN: &[&str] = &["wife", "kids", "kid", "baby", "son", "daughter", "children", "married", "family"];
const WORK: &[&str] = &["work", "job", "does", "do"];
const GAMES: &[&str] = &["games", "stands", "parents", "folks", "mom", "dad"];
const MENTOR: &[&str] = &["mentor", "grew"];
const FEEL: &[&str] = &["miss", "missing", "homesick", "far", "away"];

/// The key a person's drawn details hang on.
fn rng(w: &World, him: Them, part: u64) -> Rng {
    let u = folk::roll(w, 0x40_0E00 ^ makeup::key(him), part);
    Rng::new((u * u32::MAX as f64) as u32)
}

fn first(full: &str) -> String {
    full.split_whitespace().next().unwrap_or(full).to_string()
}

/// A woman's first name, by heritage and the year she was born.
fn woman(eth: Ethnicity, born: i32, r: &mut Rng) -> String {
    first(&cast::person_name(eth, born, true, r))
}

/// A man's first name, by heritage and the year he was born.
fn man(eth: Ethnicity, born: i32, r: &mut Rng) -> String {
    first(&crate::world::names::person_name(eth, born, r))
}

fn number(n: usize) -> &'static str {
    ["no", "one", "two", "three", "four", "five", "six"][n.min(6)]
}

fn names(v: &[String]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].clone(),
        _ => format!("{} and {}", v[..v.len() - 1].join(", "), v[v.len() - 1]),
    }
}

/// A parent's work by how the family lives (real jobs, drawn).
fn job(money: f64, r: &mut Rng) -> &'static str {
    const POOR: [&str; 8] = [
        "works two jobs",
        "works nights at a warehouse",
        "cleans offices",
        "works at a nursing home",
        "cooks at a diner",
        "drives for a delivery company",
        "works the line at a plant",
        "works at a grocery store",
    ];
    const WORKING: [&str; 10] = [
        "is a nurse",
        "is a mail carrier",
        "is a corrections officer",
        "is a mechanic",
        "is a teacher's aide",
        "drives a truck",
        "is an electrician",
        "does hair",
        "works for the county",
        "is a school bus driver",
    ];
    const COMFORTABLE: [&str; 9] = [
        "is a teacher",
        "is an accountant",
        "is a pharmacist",
        "owns a small business",
        "is a police sergeant",
        "is a dentist",
        "is a lawyer",
        "works in sales",
        "is an engineer",
    ];
    match money {
        x if x < 0.25 => r.pick(&POOR),
        x if x < 0.6 => r.pick(&WORKING),
        _ => r.pick(&COMFORTABLE),
    }
}

/// His major, from the ones college football players most often take,
/// leaned on his grades (drawn).
fn major(gpa: f64, r: &mut Rng) -> &'static str {
    let strong = gpa >= 3.2;
    r.weighted(&[
        ("general studies", if strong { 0.08 } else { 0.22 }),
        ("business", 0.16),
        ("communications", 0.14),
        ("kinesiology", 0.12),
        ("sports management", 0.12),
        ("criminal justice", 0.1),
        ("sociology", 0.08),
        ("psychology", 0.06),
        ("finance", if strong { 0.05 } else { 0.01 }),
        ("engineering", if strong { 0.03 } else { 0.0 }),
    ])
}

/// What he does on a day off, by the man he is (drawn).
fn day_off(w: &World, i: usize, r: &mut Rng) -> &'static str {
    use crate::player::Trait;
    let p = &w.people[i].player.personality;
    if p.work_ethic >= 15.0 && r.chance(0.5) {
        return "spends his off days in the film room or the weight room";
    }
    if p.traits.z(Trait::Leadership) > 0.6 && r.chance(0.4) {
        return "takes the younger guys out to eat on his off days";
    }
    if p.traits.z(Trait::Professionalism) < -0.6 && r.chance(0.4) {
        return "plays video games with the guys all weekend";
    }
    r.pick(&[
        "goes fishing when he can",
        "cuts teammates' hair on his off days",
        "never misses church on Sunday",
        "cooks for the guys on his off days",
        "makes music on his laptop",
        "plays pickup basketball",
        "plays video games with the guys",
        "sleeps as much as he can",
        "calls home every Sunday",
    ])
}

/// The coach you are.
fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

/// How open he is with people, 0.6-1.4: a player's is his bond's own
/// openness (loyalty and coachability open him, ego and temper close him,
/// a man burned by a coach is guarded) with a little more for a man who
/// warms to new people; anyone else's from his makeup.
pub fn openness(w: &World, him: Them) -> f64 {
    if let Them::Player(id) = him
        && let Some(i) = folk::person(w, id)
    {
        let p = &w.people[i];
        let adapt = p.player.personality.traits.z(crate::player::Trait::Adaptability);
        return (bonds::openness_of(w, p) as f64 * (1.0 + 0.1 * adapt)).clamp(0.6, 1.4);
    }
    let m = makeup::makeup(w, him);
    (1.0 + 0.04 * (m.warmth - 10.0) + 0.02 * (m.loyalty - 10.0) - 0.02 * (m.ego - 10.0) - 0.015 * (m.temper - 10.0))
        .clamp(0.6, 1.4)
}

/// How deep he goes about his life with you now: your bond's stage, moved
/// a layer by his openness; for anyone without a bond, his trust in you
/// times his openness.
pub fn depth(w: &World, him: Them) -> Layer {
    let open = openness(w, him);
    let layer = |d: i32| match d {
        x if x <= 0 => Layer::Surface,
        1 => Layer::Feelings,
        _ => Layer::Troubles,
    };
    if let Them::Player(id) = him
        && let Some(b) = you(w).and_then(|c| bonds::bond(w, id, c))
    {
        let step = bonds::Step::of(b);
        let mut d = match step {
            bonds::Step::JustMet => 0,
            bonds::Step::KnowsYou => 1,
            _ => 2,
        };
        if step < bonds::Step::TrustsYou && open >= OPEN_DEEPER {
            d += 1;
        }
        if open <= OPEN_SHALLOWER {
            d -= 1;
        }
        return layer(d);
    }
    let t = makeup::trust(w, him) * open;
    layer(if t < TRUST_FEELINGS {
        0
    } else if t < TRUST_TROUBLES {
        1
    } else {
        2
    })
}

/// How many things he tells you about his life for one question.
pub fn how_many(open: f64) -> usize {
    1 + usize::from(open >= 0.95) + usize::from(open >= 1.2)
}

/// Everything about his life he could tell you, every layer.
pub fn bits(w: &World, him: Them) -> Vec<Bit> {
    match him {
        Them::Player(id) => folk::person(w, id).map_or_else(Vec::new, |i| player(w, i, him)),
        Them::Kin(id, k) => folk::person(w, id).map_or_else(Vec::new, |i| kin(w, i, k)),
        Them::Coach(c) if w.coaches.get(c).is_some() && you(w) != Some(c) => coach(w, c),
        _ => drawn_family(w, him, age_of(w, him)),
    }
}

fn bit(key: u16, layer: Layer, words: String, tags: &'static [&'static str]) -> Bit {
    Bit { key, layer, words, tags }
}

/// The people who raised him, drawn on what's stored: (mother's name,
/// father's name, grandmother's name), each only where his home has one.
struct Folks {
    mom: Option<String>,
    dad: Option<String>,
    grandma: Option<String>,
    /// The parent who raised him on her or his own is his dad.
    single_dad: bool,
    /// The woman who took him in, for a boy in foster care.
    foster: Option<String>,
}

fn folks(w: &World, i: usize, him: Them) -> Folks {
    let p = &w.people[i];
    let born = p.born.year();
    let mut r = rng(w, him, 1);
    let parent_born = born - 24 - r.int(0, 10);
    let mom = woman(p.ethnicity, parent_born, &mut r);
    let dad = man(p.ethnicity, parent_born - r.int(0, 4), &mut r);
    let grandma = woman(p.ethnicity, parent_born - 22 - r.int(0, 8), &mut r);
    // About four single-parent homes in five are a mother's (Census,
    // America's Families and Living Arrangements).
    let single_dad = r.chance(0.18);
    let foster = woman(p.ethnicity, parent_born - r.int(0, 15), &mut r);
    match p.family.structure {
        F::TwoParents => Folks { mom: Some(mom), dad: Some(dad), grandma: None, single_dad: false, foster: None },
        F::SingleParent if single_dad => Folks { mom: None, dad: Some(dad), grandma: None, single_dad, foster: None },
        F::SingleParent => Folks { mom: Some(mom), dad: None, grandma: None, single_dad: false, foster: None },
        F::Grandparents => Folks { mom: None, dad: None, grandma: Some(grandma), single_dad: false, foster: None },
        F::Foster => Folks { mom: None, dad: None, grandma: None, single_dad: false, foster: Some(foster) },
    }
}

/// Who he calls home, in a few words ("his mom", "his grandmother").
fn home_word(f: &Folks, s: F) -> &'static str {
    match s {
        F::TwoParents => "his folks",
        F::SingleParent if f.single_dad => "his dad",
        F::SingleParent => "his mom",
        F::Grandparents => "his grandmother",
        F::Foster => "his foster mom",
    }
}

/// His younger brothers and sisters, drawn: (brothers' names, sisters'
/// names). Proposed: 0 (30%), 1 (35%), 2 (20%), 3 (15%).
fn younger(w: &World, i: usize, him: Them) -> (Vec<String>, Vec<String>) {
    let p = &w.people[i];
    let mut r = rng(w, him, 2);
    let u = r.next();
    let n = if u < 0.3 {
        0
    } else if u < 0.65 {
        1
    } else if u < 0.85 {
        2
    } else {
        3
    };
    let (mut bros, mut sis) = (Vec::new(), Vec::new());
    for _ in 0..n {
        let born = p.born.year() + r.int(1, 9);
        if r.chance(0.5) {
            bros.push(man(p.ethnicity, born, &mut r))
        } else {
            sis.push(woman(p.ethnicity, born, &mut r))
        }
    }
    (bros, sis)
}

/// The things a player could tell you about his life.
fn player(w: &World, i: usize, him: Them) -> Vec<Bit> {
    use Layer::*;
    let p = &w.people[i];
    let f = &p.family;
    let bg = &p.bg;
    let folks = folks(w, i, him);
    let home = home_word(&folks, f.structure);
    let mut out = Vec::new();
    // Who raised him, with their names.
    match f.structure {
        F::TwoParents => out.push(bit(
            1,
            Surface,
            format!(
                "He was raised by his mom, {}, and his dad, {}",
                folks.mom.clone().unwrap_or_default(),
                folks.dad.clone().unwrap_or_default()
            ),
            PARENTS,
        )),
        F::SingleParent if folks.single_dad => out.push(bit(
            1,
            Surface,
            format!("His dad, {}, raised him on his own", folks.dad.clone().unwrap_or_default()),
            DAD,
        )),
        F::SingleParent => out.push(bit(
            1,
            Surface,
            format!("His mom, {}, raised him on her own", folks.mom.clone().unwrap_or_default()),
            MOM,
        )),
        F::Grandparents => out.push(bit(
            1,
            Surface,
            format!("His grandmother, {}, raised him", folks.grandma.clone().unwrap_or_default()),
            GRAND,
        )),
        F::Foster => out.push(bit(
            1,
            Surface,
            format!(
                "{} took him in as a foster kid, and he calls her his mom",
                folks.foster.clone().unwrap_or_default()
            ),
            MOM,
        )),
    }
    // What the parent who raised him does for work.
    let mut r = rng(w, him, 3);
    let who = match f.structure {
        F::TwoParents if r.chance(0.5) => folks.dad.clone().map(|d| format!("His dad {d}")),
        F::TwoParents | F::SingleParent if folks.mom.is_some() => folks.mom.clone().map(|m| format!("His mom {m}")),
        F::TwoParents | F::SingleParent => folks.dad.clone().map(|d| format!("His dad {d}")),
        F::Grandparents => folks.grandma.clone().map(|g| format!("His grandmother {g}")),
        F::Foster => folks.foster.clone().map(|m| format!("His foster mom {m}")),
    };
    if let Some(who) = who {
        // A grandmother raising a teenager is often past working age.
        let work = if f.structure == F::Grandparents && r.chance(0.5) { "is retired" } else { job(bg.money, &mut r) };
        out.push(bit(2, Surface, format!("{who} {work}"), if f.structure == F::Grandparents { GRAND } else { WORK }));
    }
    // Who makes his big calls with him.
    let decides = match f.decider {
        Decider::Himself => "He makes his own big decisions".to_string(),
        Decider::Parent | Decider::Grandparent => format!("He makes the big decisions with {home}"),
        Decider::Handler => "A trainer back home, close to the family, helps him with the big decisions".into(),
    };
    out.push(bit(3, Surface, decides, PARENTS));
    // Brothers and sisters: an older brother in college football is stored.
    let (bros, sis) = younger(w, i, him);
    let mut sib = Vec::new();
    if !bros.is_empty() {
        sib.push(format!(
            "{} younger brother{} ({})",
            number(bros.len()),
            if bros.len() > 1 { "s" } else { "" },
            names(&bros)
        ));
    }
    if !sis.is_empty() {
        sib.push(format!(
            "{} younger sister{} ({})",
            number(sis.len()),
            if sis.len() > 1 { "s" } else { "" },
            names(&sis)
        ));
    }
    if !sib.is_empty() {
        out.push(bit(4, Surface, format!("He has {}", sib.join(" and ")), SIBS));
    }
    if let Some(prog) = f.older_brother_at.filter(|&j| j < w.programs.len()) {
        out.push(bit(5, Surface, format!("His older brother plays at {}", w.program_name(prog)), SIBS));
        let follow = if f.follows_brother {
            "He grew up chasing his older brother"
        } else {
            "He wants to make his own name, apart from his older brother"
        };
        out.push(bit(23, Feelings, follow.into(), SIBS));
    }
    // Football in the family.
    match f.bloodline {
        Bloodline::CollegePlayer => out.push(bit(6, Surface, "His father played college football".into(), DAD)),
        Bloodline::NflPlayer => {
            out.push(bit(6, Surface, "His father played in the NFL".into(), DAD));
            out.push(bit(24, Feelings, "Carrying his father's name in football brings pressure".into(), DAD));
        }
        Bloodline::None => {}
    }
    // How his folks show up at his games.
    match f.parent_style {
        ParentStyle::LoudAtGames => out.push(bit(
            7,
            Surface,
            format!("{} never misses a game, and you can hear it in the stands", cap(home)),
            GAMES,
        )),
        ParentStyle::SocialMedia => {
            out.push(bit(7, Surface, format!("{} posts about every game he plays", cap(home)), GAMES))
        }
        ParentStyle::CalmPrivate => {
            out.push(bit(7, Surface, format!("{} are quiet people who keep to themselves", cap(home)), GAMES))
        }
        ParentStyle::Absent => {
            out.push(bit(21, Feelings, format!("{} doesn't make it to many of his games", cap(home)), GAMES))
        }
    }
    if bg.mentor {
        out.push(bit(8, Surface, "A coach back home kept him on track growing up".into(), MENTOR));
    }
    // Where he's from.
    if let Some(h) = w.high_schools.get(p.hs) {
        let state = w.states.get(h.state).map_or("", |s| facts::state_name(&s.code));
        out.push(bit(
            9,
            Surface,
            format!("He's from {}, {state}, and went to {}", h.town, w.school_name(p.hs)),
            HOMETOWN,
        ));
    }
    let mut r = rng(w, him, 4);
    out.push(bit(10, Surface, format!("He {}", day_off(w, i, &mut r)), DAY_OFF));
    // School.
    let in_college = matches!(p.stage, Stage::College) || (p.college.is_some() && p.stage != Stage::Pro);
    if in_college {
        let mut r = rng(w, him, 5);
        let grades = match bg.gpa {
            x if x >= 3.3 => ", and he does well in class",
            x if x < 2.3 => ", and the classes are a grind for him",
            _ => "",
        };
        out.push(bit(11, Surface, format!("He's studying {}{grades}", major(bg.gpa, &mut r)), SCHOOL));
    } else if p.stage == Stage::HighSchool {
        let grades = match bg.gpa {
            x if x >= 3.3 => "He does well in school",
            x if x < 2.3 => "School is a grind for him",
            _ => "He gets by in school",
        };
        out.push(bit(11, Surface, grades.into(), SCHOOL));
    }
    // His own family: his life events say it; a pro's before the world's
    // records is drawn.
    let ev: Vec<&life::Happening> = w.folk.life.events.iter().filter(|e| e.id == p.player.id).collect();
    let last = |e: life::Event| ev.iter().filter(|x| x.event == e).map(|x| x.date).max();
    let mut r = rng(w, him, 6);
    let age = crate::calendar::born::age_or(p.born, w.today, 18);
    let married = match (last(life::Event::Wedding), last(life::Event::Divorce)) {
        (Some(a), Some(b)) => a > b,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        // Proposed: a pro 25 or older is married about half the time.
        (None, None) => p.stage == Stage::Pro && age >= 25 && r.chance(0.55),
    };
    let kids = ev.iter().filter(|x| x.event == life::Event::Birth).count();
    if married || kids > 0 {
        let wife = woman(p.ethnicity, p.born.year() + r.int(-2, 2), &mut r);
        let mut words = if married { format!("He's married to {wife}") } else { String::new() };
        if kids > 0 {
            let k = format!("{} kid{}", number(kids), if kids > 1 { "s" } else { "" });
            words = if words.is_empty() { format!("He has {k}") } else { format!("{words}, and they have {k}") };
        }
        out.push(bit(13, Surface, words, OWN));
    } else if p.stage != Stage::Pro {
        // Proposed: a girlfriend for about a third of high schoolers and
        // two in five college players.
        let share = if p.stage == Stage::HighSchool { 0.35 } else { 0.4 };
        if r.chance(share) {
            let girl = woman(p.ethnicity, p.born.year() + r.int(-1, 1), &mut r);
            let where_ = if r.chance(0.5) { "back home" } else { "at school" };
            out.push(bit(12, Surface, format!("He has a girlfriend {where_}, {girl}"), LOVE));
        }
    }
    // How he feels about home.
    let away = p.college.and_then(|j| w.programs.get(j)).is_some_and(|pr| pr.state != p.home_state);
    let homesick = ev.iter().any(|x| x.event == life::Event::Homesick && x.date.days_to(w.today) < 180);
    if homesick {
        out.push(bit(20, Feelings, "He misses home more than he lets on".into(), FEEL));
    } else if away {
        out.push(bit(20, Feelings, "He's a long way from home, and some weeks he feels it".into(), FEEL));
    }
    let family_first = f.own_wants.money >= 0.3 || bg.money < 0.25;
    if family_first {
        let who = match f.structure {
            F::TwoParents => "his family",
            _ => home,
        };
        let more = if bg.money < 0.25 { format!(": he wants to buy {who} a house one day") } else { String::new() };
        out.push(bit(22, Feelings, format!("Everything he does in football is for {who}{more}"), FEEL));
    }
    if bg.home_stability < 0.35 {
        out.push(bit(25, Feelings, "Home was never simple for him".into(), PARENTS));
    }
    // His troubles: what happened at home growing up (his story).
    for s in &bg.story {
        let age = s.trim_start_matches("At ").split(',').next().unwrap_or("").trim().to_string();
        if s.contains("a death in the family") {
            out.push(bit(40, Troubles, format!("He lost someone close to him when he was {age}"), FEEL));
        } else if s.contains("lost a job") {
            out.push(bit(
                41,
                Troubles,
                format!("Money got tight at home when a parent lost a job, when he was {age}"),
                PARENTS,
            ));
        } else if s.contains("family moved") {
            out.push(bit(14, Surface, format!("His family moved when he was {age}"), HOMETOWN));
        } else if s.contains("NIL money changed") {
            out.push(bit(26, Feelings, "His NIL money changed things for his family".into(), PARENTS));
        }
    }
    if f.structure == F::Foster {
        out.push(bit(42, Troubles, "He moved between homes in foster care before he found a family".into(), PARENTS));
    }
    if bg.money < 0.25 && matches!(p.stage, Stage::Pro) || (bg.money < 0.25 && p.nil_income > 20_000.0) {
        out.push(bit(43, Troubles, "He sends money home, and the family leans on it".into(), PARENTS));
    }
    dedup(out)
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}

/// One bit a key (a story told twice keeps the first).
fn dedup(mut v: Vec<Bit>) -> Vec<Bit> {
    let mut seen = Vec::new();
    v.retain(|b| {
        if seen.contains(&b.key) {
            false
        } else {
            seen.push(b.key);
            true
        }
    });
    v
}

/// A parent or guardian across: their own life and his, in their words
/// ("You ..." is the parent; the boy by his first name).
fn kin(w: &World, i: usize, k: u8) -> Vec<Bit> {
    use Layer::*;
    let p = &w.people[i];
    let kid = first(&p.name);
    let him = Them::Player(p.player.id);
    let folks = folks(w, i, him);
    let mut out = Vec::new();
    // The decider (slot 0) is who raised him; any other slot a parent.
    let mut r = rng(w, him, 3);
    if k == 0 || p.family.structure == F::TwoParents {
        let work = if p.family.structure == F::Grandparents && r.chance(0.5) {
            "are retired"
        } else {
            job(p.bg.money, &mut r)
        };
        let work = work.strip_prefix("is ").map_or_else(|| work.to_string(), |x| format!("are {x}"));
        let work = work.replace("works ", "work ").replace("drives ", "drive ").replace("cleans ", "clean ");
        let work = work.replace("cooks ", "cook ").replace("owns ", "own ").replace("does ", "do ");
        out.push(bit(90, Surface, format!("You {work}"), WORK));
    }
    let raised = match p.family.structure {
        F::TwoParents => format!(
            "You raised {kid} together, {} and {}",
            folks.mom.clone().unwrap_or_default(),
            folks.dad.clone().unwrap_or_default()
        ),
        F::SingleParent => format!("You raised {kid} on your own"),
        F::Grandparents => format!("You've raised {kid} since he was small"),
        F::Foster => format!("You took {kid} in when he was in foster care"),
    };
    out.push(bit(91, Surface, raised, PARENTS));
    let (bros, sis) = younger(w, i, him);
    let n = bros.len() + sis.len() + usize::from(p.family.older_brother_at.is_some());
    if n > 0 {
        let mut all = bros.clone();
        all.extend(sis.iter().cloned());
        let older = p
            .family
            .older_brother_at
            .filter(|&j| j < w.programs.len())
            .map(|j| format!("; his older brother plays at {}", w.program_name(j)))
            .unwrap_or_default();
        let young = if all.is_empty() { String::new() } else { format!(": the younger ones are {}", names(&all)) };
        out.push(bit(92, Surface, format!("{kid} has {} brothers and sisters{young}{older}", number(n)), SIBS));
    }
    if let Some(h) = w.high_schools.get(p.hs) {
        let state = w.states.get(h.state).map_or("", |s| facts::state_name(&s.code));
        out.push(bit(93, Surface, format!("You live in {}, {state}", h.town), HOMETOWN));
    }
    let away = p.college.and_then(|j| w.programs.get(j)).is_some_and(|pr| pr.state != p.home_state);
    if away {
        out.push(bit(94, Feelings, format!("You miss having {kid} close to home"), FEEL));
    }
    if p.bg.money < 0.25 {
        out.push(bit(95, Feelings, format!("You've worked hard so {kid} could play"), WORK));
    }
    for s in &p.bg.story {
        let age = s.trim_start_matches("At ").split(',').next().unwrap_or("").trim().to_string();
        if s.contains("a death in the family") {
            out.push(bit(96, Troubles, format!("The family lost someone close when {kid} was {age}"), FEEL));
        } else if s.contains("lost a job") {
            out.push(bit(97, Troubles, format!("Money got tight when a job was lost, when {kid} was {age}"), WORK));
        }
    }
    dedup(out)
}

/// A coach of anyone's staff: his own family (`life::family`), with drawn
/// names, and where he's from.
fn coach(w: &World, c: usize) -> Vec<Bit> {
    use Layer::*;
    let co = &w.coaches[c];
    let him = Them::Coach(c);
    let (married, kids) = life::family(w, c);
    let mut r = rng(w, him, 7);
    let born = w.year - co.age as i32;
    let mut out = Vec::new();
    if married {
        let wife = woman(co.ethnicity, born + r.int(-4, 2), &mut r);
        out.push(bit(60, Surface, format!("He's married to {wife}"), OWN));
    }
    if !kids.is_empty() {
        let named: Vec<String> = kids
            .iter()
            .map(|y| {
                let n = if r.chance(0.5) { man(co.ethnicity, *y, &mut r) } else { woman(co.ethnicity, *y, &mut r) };
                format!("{n} ({})", w.year - y)
            })
            .collect();
        out.push(bit(
            61,
            Surface,
            format!("He has {} kid{}: {}", number(kids.len()), if kids.len() > 1 { "s" } else { "" }, names(&named)),
            OWN,
        ));
        if kids.iter().any(|y| w.year - y < 12) {
            out.push(bit(70, Feelings, "The hours are hard on his family with the kids still young".into(), OWN));
        }
    }
    if let Some(s) = co.home_state.and_then(|s| w.states.get(s)) {
        out.push(bit(62, Surface, format!("He's from {}", facts::state_name(&s.code)), HOMETOWN));
    }
    if co.ex_pro {
        out.push(bit(63, Surface, "He played in the pros before he coached".into(), &["played", "playing"]));
    }
    let moves = co.history.len();
    if moves >= 4 {
        out.push(bit(71, Feelings, format!("He has moved his family for jobs {} times", number(moves - 1)), HOMETOWN));
    }
    let recent = w.folk.life.health.iter().any(|e| e.coach == c && e.date.days_to(w.today) < 365);
    if recent {
        out.push(bit(80, Troubles, "He had a health scare this past year".into(), &["health", "feeling", "doing"]));
    }
    out.push(bit(64, Surface, format!("He {}", coach_off(&mut r)), DAY_OFF));
    out
}

fn coach_off(r: &mut Rng) -> &'static str {
    r.pick(&[
        "gets to his kids' games when the schedule lets him",
        "plays golf in the summer",
        "goes fishing in the summer",
        "hardly has a day off in season",
        "goes to church with his family on Sunday",
        "grills for the staff in the summer",
    ])
}

/// His age, for a drawn family: a cast person's own, else about 45.
fn age_of(w: &World, him: Them) -> u32 {
    match him {
        Them::Cast(r, _) => cast::get(w, r).age,
        _ => 45,
    }
}

/// Anyone else (a boss, a booster, an agent, a scout): a family drawn from
/// his key and age, the coaches' rule.
fn drawn_family(w: &World, him: Them, age: u32) -> Vec<Bit> {
    use Layer::*;
    let mut r = rng(w, him, 8);
    let married = r.chance(if age >= 30 { 0.8 } else { 0.45 });
    let u = r.next();
    let n = if age < 28 || u < 0.18 {
        0
    } else if u < 0.4 {
        1
    } else if u < 0.75 {
        2
    } else {
        3
    };
    let mut out = Vec::new();
    let mut words = if married { "He's married".to_string() } else { "He's single".to_string() };
    if n > 0 {
        let grown = age >= 52;
        words = format!(
            "{words}, with {} {}{}",
            number(n),
            if grown { "grown kid" } else { "kid" },
            if n > 1 { "s" } else { "" }
        );
    }
    out.push(bit(100, Surface, words, OWN));
    if n > 0 && age < 50 {
        out.push(bit(110, Feelings, "His work keeps him away from his family more than he'd like".into(), OWN));
    }
    out
}

/// The words of a question, lower case.
fn words_of(q: &str) -> Vec<String> {
    q.to_lowercase()
        .replace('\u{2019}', "'")
        .split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|x| !x.is_empty())
        .map(|x| x.trim_end_matches("'s").to_string())
        .collect()
}

/// The bits a question asks for by its words.
fn asked_for<'a>(bits: &'a [Bit], q: &str) -> Vec<&'a Bit> {
    let ws = words_of(q);
    bits.iter().filter(|b| b.tags.iter().any(|t| ws.iter().any(|x| x == t))).collect()
}

/// Days inside which asking again what he told you reads as not
/// listening; after it, asking after it again is caring.
pub const AGAIN_DAYS: i32 = 14;

/// What a question asks, and what he has told you: `told` all he has told
/// you before, `recent` what he told you in the last `AGAIN_DAYS` or this
/// talk, `asked` a general question about his life already asked in this
/// talk, `closed` he went short earlier in this talk.
#[derive(Clone, Copy, Debug, Default)]
pub struct Asked<'a> {
    pub told: &'a [u16],
    pub recent: &'a [u16],
    pub asked: bool,
    pub closed: bool,
}

/// His answer to a question about his life away from football: what the
/// question names first, then what he hasn't told you yet, as many as his
/// openness gives, never deeper than he goes with you (his troubles only
/// when the question names them).
pub fn answer(w: &World, him: Them, question: &str, q: Asked) -> Said {
    let (told, closed) = (q.told, q.closed);
    let all = bits(w, him);
    let open = openness(w, him);
    let deep = if closed { Layer::Surface } else { depth(w, him) };
    let n = if closed { 1 } else { how_many(open) };
    let named = asked_for(&all, question);
    let fits = |b: &&Bit| b.layer <= deep && (b.layer < Layer::Troubles || named.iter().any(|x| x.key == b.key));
    let named_ok: Vec<&Bit> = named.iter().copied().filter(fits).collect();
    let mut out = Said::default();
    // Asked again what he just told you: he says so. Asked after it weeks
    // later, he tells you how it is, and nothing is new.
    if !named_ok.is_empty() && named_ok.iter().all(|b| q.recent.contains(&b.key)) {
        out.again = true;
        out.released.push(format!("He already told you this: {}", named_ok[0].words));
        return out;
    }
    if !named_ok.is_empty() && named_ok.iter().all(|b| told.contains(&b.key)) {
        out.released.push(format!("As he told you before: {}", named_ok[0].words));
        return out;
    }
    // The same general question twice in one talk.
    if named.is_empty() && q.asked {
        out.again = true;
        out.released.push("He already told you how things are at home".into());
        return out;
    }
    let mut pick: Vec<&Bit> = named_ok.iter().copied().filter(|b| !told.contains(&b.key)).collect();
    // Nothing named he'd go into this far: he answers in general.
    if pick.is_empty() {
        // A general question ("how's the family?", "tell me about
        // yourself"): what he hasn't told you, the surface first.
        let mut rest: Vec<&Bit> = all.iter().filter(fits).filter(|b| !told.contains(&b.key)).collect();
        rest.sort_by_key(|b| b.layer);
        pick.extend(rest);
    }
    pick.truncate(n);
    if pick.is_empty() {
        // Nothing he hasn't told you, nothing named he'd say: home is as it
        // was.
        out.released.push(match all.iter().find(|b| b.layer == Layer::Surface && told.contains(&b.key)) {
            Some(b) => format!("Nothing new at home since he last told you: {}", b.words),
            None => "Nothing new at home to tell: things are fine".into(),
        });
        return out;
    }
    for b in pick {
        out.released.push(b.words.clone());
        out.new.push((b.key, b.words.clone()));
    }
    out
}

/// A question about his troubles at home before he'd open up that far: he
/// goes short, and it costs a little trust.
pub fn too_deep(w: &World, him: Them) -> bool {
    depth(w, him) == Layer::Surface
}

/// The trust asking about his troubles too soon costs (0.015 x (1.6 -
/// openness)).
pub fn too_deep_cost(w: &World, him: Them) -> f32 {
    (0.015 * (1.6 - openness(w, him))) as f32
}

/// The trust asking again what he already told you costs.
pub const ASKED_AGAIN: f32 = 0.01;

/// Whether this is the first time you and he sit down: no one-on-one with
/// you before and no bond past "just met". A player meeting a new coach
/// says hello before business (personal talk, first meetings).
pub fn first_meeting(w: &World, id: u32) -> bool {
    if w.folk.bonds.talked.contains_key(&id) {
        return false;
    }
    you(w).and_then(|c| bonds::bond(w, id, c)).is_none_or(|b| bonds::Step::of(b) == bonds::Step::JustMet)
}

/// What you and he really share (your profile against his home): your
/// home state, how you grew up, money growing up, your high school. At
/// most 3 count.
pub fn common(w: &World, him: Them) -> u8 {
    let Them::Player(id) = him else { return 0 };
    let Some(i) = folk::person(w, id) else { return 0 };
    let Some(k) = w.career.as_ref() else { return 0 };
    let p = &w.people[i];
    let prof = &k.you.profile;
    let mut n = 0u8;
    if w.coaches.get(k.coach).and_then(|c| c.home_state) == Some(p.home_state) {
        n += 1;
    }
    if let Some((s, poor)) = bonds::raised_from(&prof.raised) {
        if s != F::TwoParents && s == p.family.structure {
            n += 1;
        }
        if poor && p.bg.money < 0.25 {
            n += 1;
        }
    }
    if !prof.high_school.is_empty()
        && w.high_schools.get(p.hs).is_some_and(|h| h.name.eq_ignore_ascii_case(&prof.high_school))
    {
        n += 1;
    }
    n.min(3)
}

/// What a man across knows of your own life, from your profile: what a
/// player or anyone around the program hears (your hometown, your
/// family), never more.
pub fn your_life(w: &World) -> Vec<String> {
    let Some(k) = w.career.as_ref() else { return Vec::new() };
    let p = &k.you.profile;
    let mut out = Vec::new();
    if !p.hometown.is_empty() {
        out.push(format!("the coach is from {}", p.hometown));
    }
    if !p.spouse.is_empty() {
        out.push(format!("the coach is married to {}", p.spouse));
    }
    if !p.children.is_empty() {
        let kids: Vec<String> = p.children.iter().map(|(n, _)| n.clone()).collect();
        out.push(format!("the coach's kids are {}", names(&kids)));
    }
    out
}
