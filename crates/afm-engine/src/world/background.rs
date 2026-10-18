//! A player's hidden background (topic 1) and his family as actors (topic 5).
//!
//! Every influence here is noteworthy but never the driving factor: each one
//! nudges odds and adds texture, and personality decides how much it bites.

use crate::attributes::Attr;
use crate::player::Player;
use crate::rng::{Rng, clamp};

/// Who raised him (1.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FamilyStructure {
    TwoParents,
    SingleParent,
    Grandparents,
    Foster,
}

impl FamilyStructure {
    pub fn phrase(self) -> &'static str {
        match self {
            FamilyStructure::TwoParents => "raised by both parents",
            FamilyStructure::SingleParent => "raised by a single parent",
            FamilyStructure::Grandparents => "raised by his grandparents",
            FamilyStructure::Foster => "grew up in foster care",
        }
    }
}

/// Who makes his big football decisions (topic 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decider {
    Himself,
    Parent,
    Grandparent,
    /// A trainer or mentor close to the family.
    Handler,
}

/// How a parent shows up around his football (topic 5). Scouts pick this up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParentStyle {
    CalmPrivate,
    LoudAtGames,
    SocialMedia,
    Absent,
}

/// Football bloodline (topic 5): noteworthy, never a driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bloodline {
    None,
    CollegePlayer,
    NflPlayer,
}

/// What the decider wants from a school. Weights sum to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wants {
    pub money: f64,
    pub close_to_home: f64,
    pub playing_time: f64,
    pub academics: f64,
    /// Coaching, development and the pro pipeline.
    pub development: f64,
    /// Winning and the brand: titles, the stadium, being on TV.
    pub prestige: f64,
}

impl Wants {
    fn normalized(mut self) -> Wants {
        let s = self.money + self.close_to_home + self.playing_time + self.academics + self.development + self.prestige;
        let s = s.max(1e-9);
        self.money /= s;
        self.close_to_home /= s;
        self.playing_time /= s;
        self.academics /= s;
        self.development /= s;
        self.prestige /= s;
        self
    }
}

#[derive(Clone, Debug)]
pub struct Family {
    pub structure: FamilyStructure,
    pub decider: Decider,
    pub parent_style: ParentStyle,
    pub bloodline: Bloodline,
    /// The decider's priorities, which can differ from the kid's own.
    pub decider_wants: Wants,
    /// His ambition, 1-20 (hidden): an ambitious kid weighs the best path to
    /// the NFL over money (note 13).
    pub ambition: f64,
    /// The kid's own priorities, which take over once he is free to choose.
    pub own_wants: Wants,
    /// An older brother playing college football: index of his program, if any.
    pub older_brother_at: Option<usize>,
    /// Does he want to follow that brother, or make his own name elsewhere?
    pub follows_brother: bool,
}

/// The six parts of a hidden background (1.1-1.6), plus mental health.
#[derive(Clone, Debug)]
pub struct Background {
    /// Household money, 0 (poverty) to 1 (wealthy). Decides trainers, camps, travel ball.
    pub money: f64,
    /// Neighborhood safety, 0 (high crime) to 1 (safe).
    pub safety: f64,
    /// How steady his home is, 0-1. A two-parent home can still be chaotic.
    pub home_stability: f64,
    /// His high school's resources, 0-1 (weight room, staff, film, academic support).
    pub school_resources: f64,
    /// His exposure network, 0-1: coach connections, 7v7, camps. Rebuilt yearly.
    pub exposure_network: f64,
    /// Grade point average, 0-4.
    pub gpa: f64,
    /// Hidden mental health stability, 1-20 (follow-up answer 1).
    pub stability: f64,
    /// Hidden grit, 1-20: incidental hardship effect, read later by the match engine.
    pub grit: f64,
    /// Hidden tendency to fight for yards after contact, 1-20.
    pub yac_tendency: f64,
    /// A coach or mentor who keeps him busy and out of trouble.
    pub mentor: bool,
    /// Life events so far, in plain words, for character reports.
    pub story: Vec<String>,
}

/// Hardship 0-1 from money and safety.
pub fn hardship(bg: &Background) -> f64 {
    clamp(0.6 * (1.0 - bg.money) + 0.4 * (1.0 - bg.safety), 0.0, 1.0)
}

/// Is he "driven": work ethic and competitiveness together, -1 to 1.
pub fn drive(p: &Player) -> f64 {
    clamp((p.personality.work_ethic + p.personality.competitiveness - 20.0) / 14.0, -1.0, 1.0)
}

/// Inputs from the world that shape a new background.
#[derive(Clone, Copy, Debug)]
pub struct BackgroundSeed {
    /// The area's typical household money, 0-1.
    pub area_money: f64,
    /// The area's typical safety, 0-1.
    pub area_safety: f64,
    /// His high school's resources, 0-1.
    pub school_resources: f64,
}

pub fn generate_background(seed: BackgroundSeed, rng: &mut Rng) -> (Background, FamilyStructure) {
    let money = clamp(seed.area_money + rng.normal(0.0, 0.18), 0.0, 1.0);
    // Safety follows money but not perfectly: poor towns can be safe, rich cities rough.
    let safety = clamp(0.5 * seed.area_safety + 0.3 * money + 0.2 + rng.normal(0.0, 0.15), 0.0, 1.0);
    let structure = rng.weighted(&[
        (FamilyStructure::TwoParents, 0.45 + 0.25 * money),
        (FamilyStructure::SingleParent, 0.38 - 0.15 * money),
        (FamilyStructure::Grandparents, 0.09),
        (FamilyStructure::Foster, 0.03),
    ]);
    // Foster homes are rarely wealthy households (trial 1 flag 3).
    let money = if structure == FamilyStructure::Foster { clamp(0.55 * money + 0.1, 0.0, 0.7) } else { money };
    let base_stability = match structure {
        FamilyStructure::TwoParents => 0.68,
        FamilyStructure::SingleParent => 0.55,
        FamilyStructure::Grandparents => 0.55,
        FamilyStructure::Foster => 0.35,
    };
    let home_stability = clamp(base_stability + 0.1 * (money - 0.5) + rng.normal(0.0, 0.18), 0.0, 1.0);
    let gpa = clamp(
        3.05 + 0.5 * (home_stability - 0.5) + 0.35 * (seed.school_resources - 0.5) + rng.normal(0.0, 0.44),
        0.8,
        4.0,
    );
    let stability = clamp(10.0 + 3.0 * (home_stability - 0.5) + rng.normal(0.0, 3.2), 1.0, 20.0);
    let bg = Background {
        money,
        safety,
        home_stability,
        school_resources: seed.school_resources,
        exposure_network: 0.0,
        gpa,
        stability,
        grit: 10.0,
        yac_tendency: 10.0,
        mentor: rng.chance(0.12 + 0.1 * (1.0 - money)),
        story: Vec::new(),
    };
    (bg, structure)
}

pub fn generate_family(bg: &Background, structure_hint: Option<FamilyStructure>, rng: &mut Rng) -> Family {
    let structure = structure_hint.unwrap_or(if bg.home_stability > 0.6 {
        FamilyStructure::TwoParents
    } else {
        FamilyStructure::SingleParent
    });
    let decider = rng.weighted(&[
        (Decider::Himself, 0.45),
        (
            match structure {
                FamilyStructure::Grandparents => Decider::Grandparent,
                _ => Decider::Parent,
            },
            0.45,
        ),
        (Decider::Handler, 0.06 + 0.06 * (1.0 - bg.money)),
    ]);
    let parent_style = rng.weighted(&[
        (ParentStyle::CalmPrivate, 0.5),
        (ParentStyle::LoudAtGames, 0.2),
        (ParentStyle::SocialMedia, 0.18),
        (ParentStyle::Absent, if structure == FamilyStructure::Foster { 0.5 } else { 0.08 }),
    ]);
    let bloodline =
        rng.weighted(&[(Bloodline::None, 0.955), (Bloodline::CollegePlayer, 0.04), (Bloodline::NflPlayer, 0.005)]);
    let own_wants = Wants {
        money: rng.range(0.05, 0.3),
        close_to_home: rng.range(0.03, 0.25),
        playing_time: rng.range(0.1, 0.35),
        academics: rng.range(0.02, 0.2),
        development: rng.range(0.1, 0.35),
        prestige: rng.range(0.1, 0.35),
    }
    .normalized();
    // A decider other than the kid has his own agenda: money for a struggling
    // household or a handler, closeness for a protective parent, school for grandparents.
    let decider_wants = match decider {
        Decider::Himself => own_wants,
        Decider::Parent => Wants {
            money: own_wants.money + 0.25 * (1.0 - bg.money) * rng.next(),
            close_to_home: own_wants.close_to_home + 0.2 * rng.next(),
            academics: own_wants.academics + 0.1 * rng.next(),
            ..own_wants
        }
        .normalized(),
        Decider::Grandparent => Wants {
            close_to_home: own_wants.close_to_home + 0.25 * rng.next(),
            academics: own_wants.academics + 0.15 * rng.next(),
            ..own_wants
        }
        .normalized(),
        Decider::Handler => Wants { money: own_wants.money + 0.45 * rng.next(), ..own_wants }.normalized(),
    };
    let ambition = clamp(rng.normal(10.0, 3.5), 1.0, 20.0);
    Family {
        ambition,
        structure,
        decider,
        parent_style,
        bloodline,
        decider_wants,
        own_wants,
        older_brother_at: None,
        follows_brother: rng.chance(0.55),
    }
}

/// Applies the background's small, lasting marks to a new 14-year-old.
///
/// - Hardship incidentally adds grit, a yards-after-contact tendency and a
///   little mental toughness, more so for a driven kid (1.1, 1.8). No nutrition effect.
/// - A fragile kid in hardship loses a little stability instead (1.8).
/// - A kid at a school without resources arrives a bit raw, with a small extra
///   share of untapped upside; a powerhouse kid arrives polished with a little
///   less room left (1.4). Neither is capped.
/// - A football bloodline means a slightly better frame and earlier teaching (topic 5).
pub fn apply_background(p: &mut Player, bg: &mut Background, family: &Family, rng: &mut Rng) {
    let h = hardship(bg);
    let d = drive(p);
    bg.grit = clamp(10.0 + 3.0 * h + 2.0 * d + rng.normal(0.0, 3.0), 1.0, 20.0);
    bg.yac_tendency = clamp(10.0 + 2.0 * h + 1.0 * d + rng.normal(0.0, 3.0), 1.0, 20.0);
    if d > 0.2 {
        // Hardship as fuel: a little extra toughness and hunger.
        p.personality.competitiveness = clamp(p.personality.competitiveness + h * rng.range(0.0, 1.5), 1.0, 20.0);
        let fuel = h * rng.range(0.0, 2.0);
        p.ceilings[Attr::Toughness] = clamp(p.ceilings[Attr::Toughness] + fuel, 1.0, 99.0);
        p.ceilings[Attr::Competitiveness] = clamp(p.ceilings[Attr::Competitiveness] + 0.5 * fuel, 1.0, 99.0);
    } else if d < -0.2 {
        bg.stability = clamp(bg.stability - h * rng.range(0.0, 2.5), 1.0, 20.0);
    }
    // Untapped upside vs polish. Technical skills are what a weight room, film
    // and staff teach early; the ceiling share is small.
    let raw = 0.5 - bg.school_resources; // +0.5 poorest, -0.5 richest
    for key in Attr::ALL {
        let profile = key.profile();
        if profile.group == crate::attributes::AttributeGroup::Physical {
            continue;
        }
        let room = p.ceilings[key] - p.attributes[key];
        // Arrives rawer or more polished: up to 6% of the gap.
        p.attributes[key] = clamp(p.attributes[key] - 0.12 * raw * room, 1.0, p.ceilings[key]);
        // Hidden upside: up to about 1.5 points either way.
        p.ceilings[key] = clamp(p.ceilings[key] + 3.0 * raw * rng.range(0.3, 1.0), 1.0, 99.0);
    }
    match family.bloodline {
        Bloodline::NflPlayer => {
            for key in [Attr::Strength, Attr::Speed, Attr::FootballIq] {
                p.ceilings[key] = clamp(p.ceilings[key] + rng.range(0.0, 2.5), 1.0, 99.0);
            }
            p.attributes[Attr::FootballIq] =
                clamp(p.attributes[Attr::FootballIq] + 2.0, 1.0, p.ceilings[Attr::FootballIq]);
        }
        Bloodline::CollegePlayer => {
            p.attributes[Attr::FootballIq] =
                clamp(p.attributes[Attr::FootballIq] + 1.0, 1.0, p.ceilings[Attr::FootballIq]);
        }
        Bloodline::None => {}
    }
    p.original_potential = p.potential();
}

/// The two risk recipes from 1.2, each 0-1. Worth noticing, never decisive.
#[derive(Clone, Copy, Debug)]
pub struct Risk {
    /// High potential + bad surroundings + a bad family structure.
    pub disaster: f64,
    /// A perfect family and surroundings + low ambition.
    pub complacency: f64,
}

pub fn risk(p: &Player, bg: &Background) -> Risk {
    let gifted = clamp((p.potential() - 60.0) / 20.0, 0.0, 1.0);
    let rough = (1.0 - bg.safety) * (1.0 - bg.home_stability);
    let mentor = if bg.mentor { 0.5 } else { 1.0 };
    let comfortable = bg.money * bg.safety * bg.home_stability;
    let low_ambition = clamp(-drive(p), 0.0, 1.0);
    Risk { disaster: gifted * rough * mentor, complacency: gifted * comfortable * low_ambition }
}

/// Multiplier on the chance of off-field trouble this year (about 0.7-1.9).
pub fn trouble_multiplier(p: &Player, bg: &Background) -> f64 {
    let r = risk(p, bg);
    let base = 1.0 + 0.55 * (1.0 - bg.safety) * if bg.mentor { 0.4 } else { 1.0 } - 0.25 * bg.home_stability;
    clamp(base * (1.0 + 1.2 * r.disaster) * (1.0 + (10.0 - bg.stability) * 0.03), 0.6, 2.2)
}

/// Background's push on his season situation, about -0.3 to +0.1. Fed into
/// development's `situation`, which nudges growth drive and form.
pub fn situation_push(p: &Player, bg: &Background) -> f64 {
    let r = risk(p, bg);
    -0.35 * r.complacency - 0.25 * r.disaster - (10.0 - bg.stability).max(0.0) * 0.012
        + if bg.mentor { 0.04 } else { 0.0 }
}

/// Chance he walks away from football this year for reasons off the field.
pub fn quit_chance(p: &Player, bg: &Background) -> f64 {
    let love = p.personality.love_of_game;
    clamp(0.004 + (10.0 - bg.stability).max(0.0) * 0.002 + (10.0 - love).max(0.0) * 0.002, 0.0, 0.05)
}

/// A yearly family or life event, "a bit more than rarely" (1.7, topic 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifeEvent {
    FamilyMove,
    JobLoss,
    Death,
    NewBaby,
    MentorStepsIn,
    NilChangesFamilyLife,
}

impl LifeEvent {
    pub fn phrase(self) -> &'static str {
        match self {
            LifeEvent::FamilyMove => "his family moved",
            LifeEvent::JobLoss => "a parent lost a job",
            LifeEvent::Death => "a death in the family",
            LifeEvent::NewBaby => "a new baby at home",
            LifeEvent::MentorStepsIn => "a mentor stepped into his life",
            LifeEvent::NilChangesFamilyLife => "NIL money changed his family's life",
        }
    }
}

/// Rolls this year's life event, if any, and applies it. Returns it so the
/// world can react (a crisis can pull him toward a school near home).
pub fn roll_life_event(bg: &mut Background, age: u32, nil_income: f64, rng: &mut Rng) -> Option<LifeEvent> {
    let nil_line = LifeEvent::NilChangesFamilyLife.phrase();
    // It changes a family's life once (trial 1 flag 4).
    if nil_income > 150_000.0 && bg.money < 0.6 && !bg.story.iter().any(|s| s.contains(nil_line)) && rng.chance(0.5) {
        bg.money = clamp(bg.money + 0.3, 0.0, 1.0);
        bg.stability = clamp(bg.stability + 0.5, 1.0, 20.0);
        bg.story.push(format!("At {age}, {}.", LifeEvent::NilChangesFamilyLife.phrase()));
        return Some(LifeEvent::NilChangesFamilyLife);
    }
    if !rng.chance(0.065) {
        return None;
    }
    let ev = rng.weighted(&[
        (LifeEvent::FamilyMove, 0.3),
        (LifeEvent::JobLoss, 0.25 + 0.1 * (1.0 - bg.money)),
        (LifeEvent::Death, 0.15),
        (LifeEvent::NewBaby, 0.15),
        (LifeEvent::MentorStepsIn, if bg.mentor { 0.0 } else { 0.15 }),
    ]);
    match ev {
        LifeEvent::FamilyMove => bg.home_stability = clamp(bg.home_stability - 0.05, 0.0, 1.0),
        LifeEvent::JobLoss => {
            bg.money = clamp(bg.money - rng.range(0.1, 0.25), 0.0, 1.0);
            bg.stability = clamp(bg.stability - rng.range(0.5, 2.0), 1.0, 20.0);
        }
        LifeEvent::Death => {
            bg.stability = clamp(bg.stability - rng.range(1.5, 4.0), 1.0, 20.0);
            bg.home_stability = clamp(bg.home_stability - 0.1, 0.0, 1.0);
        }
        LifeEvent::NewBaby => bg.money = clamp(bg.money - 0.05, 0.0, 1.0),
        LifeEvent::MentorStepsIn => {
            bg.mentor = true;
            bg.stability = clamp(bg.stability + 1.0, 1.0, 20.0);
        }
        LifeEvent::NilChangesFamilyLife => {}
    }
    bg.story.push(format!("At {age}, {}.", ev.phrase()));
    Some(ev)
}

/// Stability slowly recovers toward his baseline: family, a good coach
/// relationship and team staff lift it (follow-up answer 1).
pub fn recover_stability(bg: &mut Background, support: f64) {
    let target = 10.0 + 3.0 * (bg.home_stability - 0.5) + 2.0 * support;
    bg.stability += (target - bg.stability) * 0.15;
    bg.stability = clamp(bg.stability, 1.0, 20.0);
}

/// What a scout can see of his background, sharper with a better scout (1.7).
/// `accuracy` is the scout's judging skill, 1-20.
pub fn character_report(p: &Player, bg: &Background, family: &Family, accuracy: f64, rng: &mut Rng) -> Vec<String> {
    let mut lines = Vec::new();
    let sees = |rng: &mut Rng, difficulty: f64| rng.chance(clamp((accuracy - difficulty) / 10.0 + 0.5, 0.05, 0.98));
    lines.push(format!("Home: {}.", family.structure.phrase()));
    if sees(rng, 6.0) {
        lines.push(
            match bg.money {
                m if m < 0.25 => "Grew up with very little money.",
                m if m < 0.5 => "Working-class household.",
                m if m < 0.8 => "Comfortable household.",
                _ => "Wealthy household.",
            }
            .to_string(),
        );
    }
    if sees(rng, 10.0) && bg.safety < 0.35 {
        lines.push("Comes from a rough neighborhood.".to_string());
    }
    if sees(rng, 8.0) {
        lines.push(
            match family.parent_style {
                ParentStyle::CalmPrivate => "Family is calm and private.",
                ParentStyle::LoudAtGames => "A parent is loud at games.",
                ParentStyle::SocialMedia => "A parent is very active on social media about him.",
                ParentStyle::Absent => "Parents are not around his football.",
            }
            .to_string(),
        );
    }
    if family.decider == Decider::Handler && sees(rng, 12.0) {
        lines.push("A handler close to the family steers his decisions.".to_string());
    }
    if family.bloodline == Bloodline::NflPlayer {
        lines.push("His father played in the NFL.".to_string());
    }
    if sees(rng, 14.0) {
        let s = bg.stability + rng.normal(0.0, (20.0 - accuracy).max(1.0) * 0.25);
        lines.push(
            if s < 7.0 {
                "Staff worry about how he handles setbacks."
            } else if s > 14.0 {
                "Unflappable; handles pressure well."
            } else {
                "Nothing unusual in how he handles pressure."
            }
            .to_string(),
        );
    }
    if sees(rng, 9.0) {
        let r = risk(p, bg);
        if r.disaster > 0.25 {
            lines.push("Big talent in a tough situation: needs structure around him.".to_string());
        }
        if r.complacency > 0.25 {
            lines.push("Everything has come easy; questions about his hunger.".to_string());
        }
    }
    if bg.mentor && sees(rng, 7.0) {
        lines.push("Has a mentor who keeps him on track.".to_string());
    }
    for s in &bg.story {
        if sees(rng, 8.0) {
            lines.push(s.clone());
        }
    }
    lines
}
