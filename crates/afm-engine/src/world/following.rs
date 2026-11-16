//! A following and famous names (Ground, part I: GR42-GR46).
//!
//! - **Famous names** (GR42): the world's famous players (a Pro Bowl or
//!   more, or a Heisman) pass their name and its hype to a son or nephew
//!   coming through high school, like Arch Manning. A son is one of the
//!   class's boys whose father played in the NFL (the bloodline's 1 in 200
//!   stays as it was); now his father has a name. The hype adds to his fame
//!   (`fame::fame`), so to his followers, his deals and the staffs' eye.
//! - **News** (GR43): a commitment's news reads his following as well as
//!   his stars: the class's 10 biggest followings make big news and the
//!   very biggest huge news (`recruiting::commit_now`).
//! - **Brand exposure** (GR44): a commitment lifts the program's brand
//!   exposure grade for a year, up to 3 points for the class's biggest
//!   following (`Lift`, `day`).
//! - **His pick** (GR45): a recruit weighs a program's brand exposure by
//!   his following, up to twice as much for the class's biggest followings
//!   (`Person::spotlight`, read in `recruiting::utility_fit`).
//! - **Storms** (GR46): a college player or recruit with 500,000 followers
//!   gets a pro's chance of a social media storm
//!   (`storylines::detect::weekly`).
//!
//! Sizes the agreed items don't settle are Claude's picks (proposed, not
//! confirmed); each says so where it is set.

use serde::{Deserialize, Serialize};

use super::program::GradeCat;
use super::{Alumnus, Stage, World};
use crate::calendar::Date;
use crate::world::background::Bloodline;
use crate::world::season::key_uniform;

/// The biggest lift on a brand exposure grade, in grade points: the class's
/// biggest following (GR44). Only the class's 10 biggest followings lift a
/// grade (the ones that make big news), and a program holds its biggest
/// lift, not the sum (both proposed, not confirmed: with every commitment
/// lifting and the lifts summed, 515 programs held a lift on a small test
/// world, most at the cap, so the hype kid stood out from nobody).
pub const LIFT_TOP: f64 = 3.0;
/// How long a commitment lifts the grade (GR44).
pub const LIFT_DAYS: i32 = 365;
/// The class's biggest followings that make big news (GR43).
pub const BIG_NEWS: usize = 10;
/// A following too small to make news on its own, whatever its rank (a
/// tiny test world's or an early era's class; proposed, not confirmed).
const NEWS_FLOOR: u32 = 10_000;
/// The followers that open a social media storm's chance (GR46: a pro's).
pub const STORM_FOLLOWERS: u32 = 500_000;

/// A boy who carries a famous name (GR42).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Name {
    /// The boy's person id.
    pub id: u32,
    /// The famous man: his place in `World::alumni`, his player id when the
    /// books have it, and his name.
    pub alumnus: u32,
    pub father_id: Option<u32>,
    pub who: String,
    /// Son or nephew.
    pub nephew: bool,
    /// What the man was, in words: "a four-time Pro Bowl receiver".
    pub what: String,
    /// How famous the man is, 0-1 (`man_fame`).
    pub fame: f32,
    /// What it adds to the boy's fame (`HYPE_SON`, `HYPE_NEPHEW`).
    pub hype: f32,
    pub year: i32,
}

impl Name {
    /// "His father is Darnell Hayes, a four-time Pro Bowl receiver."
    pub fn line(&self) -> String {
        let kin = if self.nephew { "His uncle" } else { "His father" };
        format!("{kin} is {}, {}.", self.who, self.what)
    }

    /// "son of four-time Pro Bowl receiver Darnell Hayes" (the news).
    pub fn tag(&self) -> String {
        let kin = if self.nephew { "nephew" } else { "son" };
        let what = self.what.trim_start_matches("an ").trim_start_matches("a ");
        format!("{kin} of {what} {}", self.who)
    }
}

/// A class's biggest followings (GR43-GR45), set at the year's book.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ClassTop {
    pub class_of: i32,
    /// The biggest following and the tenth biggest.
    pub top: u32,
    pub tenth: u32,
}

/// A commitment lifting a program's brand exposure (GR44).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Lift {
    pub id: u32,
    pub program: u32,
    pub day: Date,
    pub points: f32,
}

/// The world's store for this part (`World::following`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Following {
    /// Famous names, in the boys' id order.
    pub names: Vec<Name>,
    /// Each high school class's biggest followings.
    pub classes: Vec<ClassTop>,
    pub lifts: Vec<Lift>,
}

impl Following {
    pub fn name_of(&self, id: u32) -> Option<&Name> {
        self.names.binary_search_by_key(&id, |n| n.id).ok().map(|k| &self.names[k])
    }

    pub fn class(&self, class_of: i32) -> Option<&ClassTop> {
        self.classes.iter().find(|c| c.class_of == class_of)
    }

    /// The hype a boy's famous name adds to his fame (0 for most).
    pub fn hype(&self, id: u32) -> f64 {
        self.name_of(id).map_or(0.0, |n| n.hype as f64)
    }
}

// --- Famous names (GR42) ---------------------------------------------------

/// What a famous father's name adds to his son's fame, at the most famous
/// man (a Hall of Famer): about what a fifth star adds in high school, so
/// a Hall of Famer's two-star son is as known as a four-star, and his
/// five-star son leads the class (proposed, not confirmed).
const HYPE_SON: f64 = 0.35;
/// A nephew carries about half of it (proposed, not confirmed).
const HYPE_NEPHEW: f64 = 0.18;
/// How famous a man has to be to pass his name on: a Pro Bowl.
const FAMOUS: f64 = 0.4;

/// His pro career and books, as the public knows him, 0-1 (proposed, not
/// confirmed): a Hall of Famer 1; an MVP, a Player of the Year or three
/// first-team All-Pros 0.85; a first-team All-Pro, four Pro Bowls or a
/// Heisman 0.65; a Pro Bowl 0.4; anyone else 0 (not famous here).
pub fn man_fame(w: &World, a: &Alumnus) -> f64 {
    let id = a.player.as_ref().map(|p| p.id);
    let career = a.pro.as_deref().or_else(|| {
        let id = id?;
        let i = super::stories::person_index(w, id)?;
        w.people[i].pro.as_deref()
    });
    let hall = id.is_some_and(|id| w.nfl_book.hall.iter().any(|h| h.1 == id));
    let heisman =
        id.is_some_and(|id| w.stats.honors.get(&id).is_some_and(|h| h.awards.iter().any(|x| x.1 == "Heisman Trophy")));
    let (pb, ap, big) = career.map_or((0, 0, false), |c| {
        let pb = c.seasons.iter().filter(|s| s.pro_bowl).count();
        let ap = c.seasons.iter().filter(|s| s.all_pro == 1).count();
        let big = c.seasons.iter().any(|s| {
            s.awards.iter().any(|x| {
                matches!(*x, "MVP" | "Offensive Player of the Year" | "Defensive Player of the Year" | "Super Bowl MVP")
            })
        });
        (pb, ap, big)
    });
    if hall {
        1.0
    } else if big || ap >= 3 {
        0.85
    } else if ap >= 1 || pb >= 4 || heisman {
        0.65
    } else if pb >= 1 {
        0.4
    } else {
        0.0
    }
}

/// "a Hall of Fame quarterback", "a four-time Pro Bowl receiver".
fn man_words(w: &World, a: &Alumnus, fame: f64) -> String {
    let spot = super::folk::claims::words::spot(a.position);
    let id = a.player.as_ref().map(|p| p.id);
    let career = a.pro.as_deref().or_else(|| {
        let i = super::stories::person_index(w, id?)?;
        w.people[i].pro.as_deref()
    });
    let pb = career.map_or(0, |c| c.seasons.iter().filter(|s| s.pro_bowl).count());
    let times = |n: usize| match n {
        2 => "two-time ".to_string(),
        3 => "three-time ".to_string(),
        4 => "four-time ".to_string(),
        5 => "five-time ".to_string(),
        6 => "six-time ".to_string(),
        n if n >= 7 => format!("{n}-time "),
        _ => String::new(),
    };
    let mvp = career.is_some_and(|c| c.seasons.iter().any(|s| s.awards.contains(&"MVP")));
    if fame >= 1.0 {
        format!("a Hall of Fame {spot}")
    } else if mvp {
        format!("an MVP {spot}")
    } else if fame >= 0.85 || (fame >= 0.65 && career.is_some_and(|c| c.seasons.iter().any(|s| s.all_pro == 1))) {
        format!("an All-Pro {spot}")
    } else if pb > 0 {
        format!("a {}Pro Bowl {spot}", times(pb))
    } else {
        format!("a Heisman-winning {spot}")
    }
}

/// His kin who come through high school, keyed on the man (proposed, not
/// confirmed): a son who plays for 45% of men, two for 13% (about 0.7 a
/// man, a 14-year-old when the man is 36 to 50), and a nephew for 30%
/// (14 when the man is 30 to 55). (nephew, the year the boy is 14).
pub(crate) fn kin(seed: u64, key: u64, born: i32) -> Vec<(bool, i32)> {
    let mut out = Vec::new();
    let sons: u64 = match key_uniform(seed ^ 0xF0_5045, key) {
        x if x < 0.13 => 2,
        x if x < 0.58 => 1,
        _ => 0,
    };
    for k in 0..sons {
        let at = 36 + (key_uniform(seed ^ 0xF0_5046 ^ k, key) * 15.0) as i32;
        out.push((false, born + at));
    }
    if key_uniform(seed ^ 0xF0_5047, key) < 0.3 {
        let at = 30 + (key_uniform(seed ^ 0xF0_5048, key) * 26.0) as i32;
        out.push((true, born + at));
    }
    out
}

/// The class day (after the new class is in, from `start`): the famous
/// men's sons and nephews who turn 14 this year join it under their
/// names. A son is a boy of the class whose father played in the NFL, in
/// the man's home state where one fits (any state otherwise), of his
/// ethnicity; a nephew any boy of the class without a football father.
pub(crate) fn names(w: &mut World, start: usize) {
    let year = w.year;
    let seed = w.options.seed as u64;
    let mut due: Vec<(usize, bool, u64)> = Vec::new();
    for (k, a) in w.alumni.iter().enumerate() {
        if a.player.is_none() && a.pro.is_none() {
            continue;
        }
        let born = a.born.year();
        if a.born == crate::calendar::born::UNKNOWN || !(year - 55..=year - 30).contains(&born) {
            continue;
        }
        let key = a.player.as_ref().map_or(k as u64 | 1 << 40, |p| p.id as u64);
        for (j, (nephew, at)) in kin(seed, key, born).into_iter().enumerate() {
            if at == year {
                due.push((k, nephew, key ^ (j as u64) << 48));
            }
        }
    }
    if due.is_empty() {
        return;
    }
    let mut made: Vec<Name> = Vec::new();
    for (k, nephew, key) in due {
        let a = &w.alumni[k];
        let fame = man_fame(w, a);
        if fame < FAMOUS {
            continue;
        }
        let what = man_words(w, a, fame);
        let (who, ethnicity, home, father_id) =
            (a.name.clone(), a.ethnicity, a.home_state, a.player.as_ref().map(|p| p.id));
        let taken = |id: u32| made.iter().any(|n| n.id == id) || w.legacy.sons.iter().any(|s| s.0 == id);
        // A son's father played in the NFL; a nephew's father didn't play.
        let want = if nephew { Bloodline::None } else { Bloodline::NflPlayer };
        let fits = |i: usize, at_home: bool| {
            let p = &w.people[i];
            p.stage == Stage::HighSchool
                && p.ethnicity == ethnicity
                && (!at_home || p.home_state == home)
                && p.family.bloodline == want
                && !taken(p.player.id)
        };
        let pick = |at_home: bool| {
            (start..w.people.len())
                .filter(|&i| fits(i, at_home))
                .min_by_key(|&i| (w.people[i].player.id as u64 ^ key).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        };
        let Some(i) = pick(true).or_else(|| pick(false)) else { continue };
        let surname = who.split_whitespace().last().unwrap_or("").to_string();
        let first = w.people[i].name.split_whitespace().next().unwrap_or("").to_string();
        let id = w.people[i].player.id;
        // A son named for his father, like Marvin Harrison Jr. (about one in
        // eight, proposed, not confirmed); a nephew is a brother's son, with
        // the name, about two times in three.
        let roll = key_uniform(seed ^ 0xF0_5049, key);
        let name = if !nephew && roll < 0.12 {
            Some(format!("{who} Jr."))
        } else if !nephew || roll < 0.65 {
            Some(format!("{first} {surname}"))
        } else {
            None
        };
        if let Some(n) = name {
            w.rename(i, n);
        }
        made.push(Name {
            id,
            alumnus: k as u32,
            father_id,
            who,
            nephew,
            what,
            fame: fame as f32,
            hype: ((if nephew { HYPE_NEPHEW } else { HYPE_SON }) * fame) as f32,
            year,
        });
    }
    w.following.names.extend(made);
    w.following.names.sort_by_key(|n| n.id);
}

// --- The class's biggest followings (GR43, GR45) ----------------------------

/// After the year's followers are set (`fame::year_book`): each high
/// school class's biggest and tenth-biggest following, and each recruit's
/// spotlight, his following as a share of his class's biggest (GR45).
pub(crate) fn class_tops(w: &mut World) {
    let school = |s: Stage| matches!(s, Stage::HighSchool | Stage::PrepSchool);
    let mut tops: Vec<(i32, Vec<u32>)> = Vec::new();
    for p in w.people.iter().filter(|p| school(p.stage)) {
        let row = match tops.iter().position(|t| t.0 == p.class_of) {
            Some(k) => &mut tops[k].1,
            None => {
                tops.push((p.class_of, Vec::new()));
                &mut tops.last_mut().unwrap().1
            }
        };
        if row.len() < BIG_NEWS || p.followers > row[row.len() - 1] {
            let at = row.partition_point(|&f| f >= p.followers);
            row.insert(at, p.followers);
            row.truncate(BIG_NEWS);
        }
    }
    tops.sort_by_key(|t| t.0);
    w.following.classes = tops
        .iter()
        .map(|(c, row)| ClassTop {
            class_of: *c,
            top: row.first().copied().unwrap_or(0),
            tenth: row.last().copied().unwrap_or(0),
        })
        .collect();
    let classes = w.following.classes.clone();
    use rayon::prelude::*;
    w.people.par_iter_mut().for_each(|p| {
        p.spotlight = if school(p.stage) {
            classes.iter().find(|c| c.class_of == p.class_of).map_or(0.0, |c| share(p.followers, c.top) as f32)
        } else {
            0.0
        };
    });
}

/// A following as a share of the class's biggest, 0-1.
fn share(followers: u32, top: u32) -> f64 {
    if top == 0 { 0.0 } else { (followers as f64 / top as f64).min(1.0) }
}

/// How loud his commitment is by his following (GR43): huge for the
/// class's biggest, big for its ten biggest, None for the rest (his stars
/// decide then).
pub fn commit_news(w: &World, i: usize) -> Option<crate::calendar::events::Loudness> {
    use crate::calendar::events::Loudness;
    let p = &w.people[i];
    if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) || p.followers < NEWS_FLOOR {
        return None;
    }
    let c = w.following.class(p.class_of)?;
    if p.followers >= c.top {
        Some(Loudness::Huge)
    } else if p.followers >= c.tenth {
        Some(Loudness::Big)
    } else {
        None
    }
}

/// "412,000 followers", "1.2 million followers".
pub fn followers_words(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1} million followers", n as f64 / 1e6)
    } else if n >= 1_000 {
        let k = (n as f64 / 1000.0).round() as u32;
        if k >= 1000 { "1.0 million followers".into() } else { format!("{k},000 followers") }
    } else {
        format!("{n} followers")
    }
}

// --- Brand exposure (GR44) ---------------------------------------------------

/// His commitment to program `j` on day `d` lifts its brand exposure for a
/// year by his share of the class's biggest following, when his is one of
/// the class's 10 biggest (GR44). A flip moves the lift with him.
pub(crate) fn committed(w: &mut World, i: usize, j: usize, d: Date) {
    committed_in(&mut w.following, &w.people[i], j, d);
}

/// [`committed`] on the store and the boy, where the world is borrowed in
/// parts.
pub(crate) fn committed_in(f: &mut Following, p: &super::Person, j: usize, d: Date) {
    let id = p.player.id;
    f.lifts.retain(|l| l.id != id);
    if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
        return;
    }
    let big = f.class(p.class_of).is_some_and(|c| p.followers >= c.tenth.max(NEWS_FLOOR));
    let points = LIFT_TOP * p.spotlight as f64;
    if big && points > 0.0 {
        f.lifts.push(Lift { id, program: j as u32, day: d, points: points as f32 });
    }
}

/// Whether the boy behind a lift still holds it: committed to, signed with
/// or playing for the program.
fn holds(w: &World, l: &Lift) -> bool {
    let Some(i) = super::stories::person_index(w, l.id) else { return false };
    let p = &w.people[i];
    let j = l.program as usize;
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            p.cal.commit.is_some_and(|c| c.0 == j) || p.cal.signed.as_ref().is_some_and(|s| s.program == j)
        }
        Stage::College | Stage::Declared => p.college == Some(j),
        _ => false,
    }
}

/// Each morning: lifts end after a year or when the commitment does, and
/// every program's brand exposure grade carries the biggest lift it holds
/// now.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.following.lifts.is_empty() && w.programs.iter().all(|p| p.brand_lift == 0.0) {
        return;
    }
    let keep: Vec<bool> = w.following.lifts.iter().map(|l| l.day.days_to(d) < LIFT_DAYS && holds(w, l)).collect();
    let mut k = keep.iter();
    w.following.lifts.retain(|_| *k.next().unwrap());
    let mut sum = vec![0.0f64; w.programs.len()];
    for l in &w.following.lifts {
        if let Some(s) = sum.get_mut(l.program as usize) {
            *s = s.max(l.points as f64);
        }
    }
    for (p, s) in w.programs.iter_mut().zip(sum) {
        set_lift(p, s.min(LIFT_TOP));
    }
}

/// Moves a program's brand exposure grade by its lift's change: the grade
/// is its own read plus the lift, never past 99.
fn set_lift(p: &mut super::program::Program, lift: f64) {
    if (lift - p.brand_lift).abs() < 1e-9 {
        return;
    }
    let k = GradeCat::BrandExposure as usize;
    let base = p.grades[k] - p.brand_lift;
    p.grades[k] = (base + lift).min(99.0).max(base);
    p.brand_lift = p.grades[k] - base;
}

/// Before the year's grades are read again (`staff.rs`): every lift comes
/// off, so the grade moves from its own read.
pub(crate) fn strip(w: &mut World) {
    for p in w.programs.iter_mut() {
        set_lift(p, 0.0);
    }
}

/// A world file from before this part: the class tables and spotlights as
/// February's book would have left them.
pub(crate) fn after_load(w: &mut World) {
    if w.following.classes.is_empty() {
        class_tops(w);
    }
}

/// His name as the news gives it: "Cole Hayes, son of four-time Pro Bowl
/// receiver Darnell Hayes," for a famous name (GR42), his name otherwise.
pub fn news_name(w: &World, i: usize) -> String {
    let p = &w.people[i];
    match w.following.name_of(p.player.id) {
        Some(n) => format!("{}, {},", p.name, n.tag()),
        None => p.name.clone(),
    }
}
