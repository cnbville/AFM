//! Coach and player bonds (`docs/design/coach-bonds.md`; Cain 2026-11-17:
//! "build up to a father figure, everything should count", then "yes to
//! all" on its seven questions).
//!
//! Every player keeps a bond with each coach who works with him: his head
//! coach, his coordinator, his position coach and his high school coach.
//! A bond has trust (does he believe your word) and closeness (how much
//! you mean to him, 0-1). Closeness grows slowly with time together, how
//! well the two fit, what they share, what he needs and the coach's way
//! with people, and it jumps at the hard moments. At its deepest a coach
//! becomes family: a big brother, a father figure or a grandfather, by the
//! age gap (question 1). It works only through what is already built:
//! instructions followed and talks landing (questions 3), how much of his
//! coaching sticks (question 4), staying, following a coach, free agency,
//! his family's trust and recruiting his brothers; and it costs: favoritism
//! the room sees, the hurt when a coach leaves and a colder room for the
//! next coach (question 5). AI coaches build bonds on the same rules
//! (question 6). Sizes are the spec's (Claude's proposals, agreed in
//! full); the build's own sizes are marked "proposed, not confirmed".
//!
//! Not kept yet: strength coaches (cast people today; their bonds come with
//! the support departments lane) and NFL position coaches (drawn, not
//! stored people).

use crate::calendar::Date;
use crate::player::Trait;
use crate::world::background::{FamilyStructure, ParentStyle};
use crate::world::coach::Group;
use crate::world::{Stage, World};
use std::collections::BTreeMap;

/// Closeness gained a full week of contact at every factor 1, times the
/// room left (`1 - closeness`). At the median a position coach reaches
/// "Knows you" in a season and "Trusts you" in his third (section 5).
pub const GAIN: f32 = 0.0045;
/// Without a hard moment shared, time alone stops short of "Family".
pub const TIME_TOP: f32 = 0.70;
/// Closeness held under his trust: he can't be close to a man he doesn't
/// trust.
pub const OVER_TRUST: f32 = 0.15;
/// Apart, closeness fades a tenth a year, never below half its best.
pub const FADE_YEAR: f32 = 0.9;
/// Moments kept a bond: the ones that moved it most.
pub const MOMENTS: usize = 8;

/// Where they met.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Met {
    /// He recruited him (or was on the staff that signed him).
    Recruited,
    /// He came to the job and found him on the roster.
    Inherited,
    WalkOn,
    Transfer,
    /// His high school coach.
    HighSchool,
    /// Met in the pros.
    Pro,
}

impl Met {
    pub fn words(self) -> &'static str {
        match self {
            Met::Recruited => "recruited him",
            Met::Inherited => "inherited him",
            Met::WalkOn => "took him as a walk-on",
            Met::Transfer => "took him as a transfer",
            Met::HighSchool => "his high school coach",
            Met::Pro => "coached him in the pros",
        }
    }
}

/// One player's bond with one coach.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Bond {
    pub coach: u32,
    /// How much the coach means to him, 0-1.
    pub close: f32,
    /// The best it has been (fading never takes it below half of this).
    pub best: f32,
    /// Does he believe this coach's word, 0-1. For his head coach it is his
    /// built trust (`Person::trust`), copied each week.
    pub trust: f32,
    /// How well the two fit, 0.6-1.4: read when the bond starts and each
    /// summer (makeup, what they share, how the coach talks).
    pub fit: f32,
    pub met: Met,
    /// Bit 0: a hard moment shared (the top stage needs one).
    pub flags: u8,
    /// Extra weeks of contact owed this week: each talk with you counts a
    /// full week (section 5).
    pub extra: u8,
    /// Weeks they have worked together.
    pub weeks: u16,
}

impl Bond {
    pub fn hard(&self) -> bool {
        self.flags & 1 != 0
    }
}

/// The stages, as words (section 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Step {
    JustMet,
    KnowsYou,
    TrustsYou,
    InHisCorner,
    Family,
}

/// What kind of bond the top stage is, by the age gap (question 1) and his
/// home (question 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    /// Under 12 years apart.
    Brother,
    /// 12 to 40 years apart.
    Father,
    /// 40 years or more.
    Grandfather,
    /// A father figure to a kid whose own dad is close: his dad is his dad.
    Family,
}

impl Kind {
    pub fn top(self) -> &'static str {
        match self {
            Kind::Brother => "Like a big brother",
            Kind::Father => "Like a father",
            Kind::Grandfather => "Like a grandfather",
            Kind::Family => "Like family",
        }
    }
}

impl Step {
    pub fn of(b: &Bond) -> Step {
        let (c, t) = (b.close, b.trust);
        if c >= 0.75 && t >= 0.75 && b.hard() {
            Step::Family
        } else if c >= 0.55 && t >= 0.65 {
            Step::InHisCorner
        } else if c >= 0.35 && t >= 0.55 {
            Step::TrustsYou
        } else if c >= 0.15 {
            Step::KnowsYou
        } else {
            Step::JustMet
        }
    }

    pub fn words(self, kind: Kind) -> &'static str {
        match self {
            Step::JustMet => "Just met",
            Step::KnowsYou => "Knows you",
            Step::TrustsYou => "Trusts you",
            Step::InHisCorner => "In his corner",
            Step::Family => kind.top(),
        }
    }

    /// What you'd see (section 4's table).
    pub fn sign(self) -> &'static str {
        match self {
            Step::JustMet => "He knows your name.",
            Step::KnowsYou => "He answers you, little more.",
            Step::TrustsYou => "He comes to you with problems and tells you things.",
            Step::InHisCorner => "He stays after practice to talk.",
            Step::Family => "He'd go through a wall for you.",
        }
    }
}

/// The moments that move a bond (section 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum What {
    Recruited = 1,
    FirstStart,
    Scholarship,
    InjuryThere,
    InjuryAlone,
    LossThere,
    LossAlone,
    IllThere,
    IllAlone,
    StoodBy,
    BenchedTold,
    BenchedChart,
    PromiseBroken,
    Graduation,
    HardSeason,
    Title,
    Cut,
    Traded,
    Left,
}

impl What {
    pub fn words(self) -> &'static str {
        match self {
            What::Recruited => "Recruited him",
            What::FirstStart => "Gave him his first start",
            What::Scholarship => "Put him on scholarship in front of the team",
            What::InjuryThere => "Was there for him through an injury",
            What::InjuryAlone => "Left him alone with an injury",
            What::LossThere => "Was there when he lost someone",
            What::LossAlone => "Wasn't there when he lost someone",
            What::IllThere => "Stood by him when his family was sick",
            What::IllAlone => "Wasn't there when his family was sick",
            What::StoodBy => "Stood by him in trouble, with a fair punishment",
            What::BenchedTold => "Told him to his face he'd lost the job",
            What::BenchedChart => "Let him find out from the depth chart",
            What::PromiseBroken => "Broke a promise to him",
            What::Graduation => "Was at his graduation",
            What::HardSeason => "Came through a losing season together",
            What::Title => "Won a title together",
            What::Cut => "Cut him",
            What::Traded => "Traded him away",
            What::Left => "Left for another job",
        }
    }

    /// The same moment as he'd put it to the coach's face ("you").
    pub fn to_him(self) -> &'static str {
        match self {
            What::Recruited => "recruited you",
            What::FirstStart => "gave you your first start",
            What::Scholarship => "put you on scholarship in front of the team",
            What::InjuryThere => "was there for you through an injury",
            What::InjuryAlone => "left you alone with an injury",
            What::LossThere => "was there when you lost someone",
            What::LossAlone => "wasn't there when you lost someone",
            What::IllThere => "stood by you when your family was sick",
            What::IllAlone => "wasn't there when your family was sick",
            What::StoodBy => "stood by you in trouble, with a fair punishment",
            What::BenchedTold => "told you to your face you'd lost the job",
            What::BenchedChart => "let you find out from the depth chart that you'd lost the job",
            What::PromiseBroken => "broke a promise to you",
            What::Graduation => "came to your graduation",
            What::HardSeason => "came through a losing season with you",
            What::Title => "won a title with you",
            What::Cut => "cut you",
            What::Traded => "traded you away",
            What::Left => "left for another job",
        }
    }

    /// A hard moment shared (the top stage needs one).
    pub fn hard(self) -> bool {
        matches!(
            self,
            What::InjuryThere | What::LossThere | What::IllThere | What::StoodBy | What::HardSeason | What::Title
        )
    }
}

/// One dated moment of a bond.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Moment {
    pub id: u32,
    pub coach: u32,
    pub on: Date,
    pub what: What,
    /// The closeness it moved, in hundredths.
    pub by: i8,
}

/// A hard moment of one of your players, waiting on what you do (section
/// 6: for your seat each is your choice): a one-on-one with him inside
/// three weeks is being there.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Waiting {
    pub id: u32,
    pub coach: u32,
    pub on: Date,
    pub there: What,
    pub alone: What,
    pub up: f32,
    pub down: f32,
}

/// The bonds store (one per world, in `Folk`).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Bonds {
    /// Each player's bonds, by his id.
    pub by: BTreeMap<u32, Vec<Bond>>,
    /// Each player's moments, by his id (up to `MOMENTS` a bond).
    pub moments: BTreeMap<u32, Vec<Moment>>,
    pub waiting: Vec<Waiting>,
    /// A coach he was close to left him: the day and how close he was
    /// (section 8: a high-need kid's trouble risk rises for a season).
    pub lost: BTreeMap<u32, (Date, f32)>,
    /// Burned by a coach (a cut, a broken promise, a coach who left him):
    /// guarded with the next one for a year (section 5, his openness).
    pub burned: BTreeMap<u32, Date>,
    /// Passed over this season for a man his coach is close to (section
    /// 8, favoritism): the rating gap, set at fall camp.
    pub passed: BTreeMap<u32, f32>,
    /// His last one-on-one with you (a benching told to his face).
    pub talked: BTreeMap<u32, Date>,
    /// His last talk with you where something personal landed (personal
    /// talk, item 8): it counts once a week.
    pub personal: BTreeMap<u32, Date>,
}

impl Bonds {
    pub fn is_empty(&self) -> bool {
        self.by.is_empty()
            && self.moments.is_empty()
            && self.waiting.is_empty()
            && self.lost.is_empty()
            && self.burned.is_empty()
            && self.passed.is_empty()
            && self.talked.is_empty()
            && self.personal.is_empty()
    }

    pub fn get(&self, id: u32, coach: usize) -> Option<&Bond> {
        self.by.get(&id)?.iter().find(|b| b.coach == coach as u32)
    }

    fn get_mut(&mut self, id: u32, coach: usize) -> Option<&mut Bond> {
        self.by.get_mut(&id)?.iter_mut().find(|b| b.coach == coach as u32)
    }

    /// His closeness to a coach, 0 with no bond.
    pub fn close(&self, id: u32, coach: Option<usize>) -> f32 {
        coach.and_then(|c| self.get(id, c)).map_or(0.0, |b| b.close)
    }
}

/// His closeness to a coach (0 with none).
pub fn close(w: &World, id: u32, coach: Option<usize>) -> f32 {
    w.folk.bonds.close(id, coach)
}

/// His bond with a coach.
pub fn bond(w: &World, id: u32, coach: usize) -> Option<&Bond> {
    w.folk.bonds.get(id, coach)
}

/// The coach you are (None without a career).
fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

// --- Who works with him ---------------------------------------------------------------

/// The seat a coach holds over him.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seat {
    Head,
    Coordinator,
    Position,
    /// A high school head coach (he coaches the position too, most places).
    HsHead,
    HsCoordinator,
}

impl Seat {
    /// The share of his week with this coach at full contact (section 5).
    fn base(self) -> f32 {
        match self {
            Seat::Head => 0.25,
            Seat::Coordinator => 0.4,
            Seat::Position | Seat::HsHead => 1.0,
            Seat::HsCoordinator => 0.6,
        }
    }
}

/// The coaches working with person `i` today and their seats: a college
/// player's head coach, coordinator and position coach; a pro's head coach
/// and coordinator; a high school player's head coach and coordinator.
pub fn coaches_of(w: &World, i: usize) -> Vec<(usize, Seat)> {
    let p = &w.people[i];
    let group = Group::of(p.player.position);
    let side = |oc: Option<usize>, dc: Option<usize>| match group {
        Group::Specialists => None,
        g if g.offense() => oc,
        _ => dc,
    };
    let mut out: Vec<(usize, Seat)> = Vec::with_capacity(3);
    let mut push = |c: Option<usize>, s: Seat| {
        if let Some(c) = c
            && c < w.coaches.len()
            && !out.iter().any(|x| x.0 == c)
        {
            out.push((c, s));
        }
    };
    match p.stage {
        Stage::College => {
            let Some(j) = p.college else { return out };
            let Some(prog) = w.programs.get(j) else { return out };
            push(prog.sideline.or(prog.head_coach), Seat::Head);
            push(prog.position_coaches[group as usize], Seat::Position);
            push(side(prog.oc, prog.dc), Seat::Coordinator);
        }
        Stage::Pro => {
            let Some(t) = p.pro.as_ref().and_then(|c| c.team) else { return out };
            let Some(club) = w.nfl.get(t) else { return out };
            push(club.head_coach, Seat::Head);
            push(side(club.oc, club.dc), Seat::Coordinator);
        }
        Stage::HighSchool => {
            if !hs_kept(w, i) {
                return out;
            }
            let Some(h) = w.high_schools.get(p.hs) else { return out };
            push(h.coach, Seat::HsHead);
            push(side(h.oc, h.dc), Seat::HsCoordinator);
        }
        _ => {}
    }
    out
}

/// High school bonds are kept for your own school's players and for the
/// recruited juniors and seniors (3 stars or more): the ones whose high
/// school coach's pull reaches a college (memory: 986,000 high schoolers
/// in a full world; proposed, not confirmed).
fn hs_kept(w: &World, i: usize) -> bool {
    let p = &w.people[i];
    if super::Team::seat(w) == Some(super::Team::High(p.hs)) {
        return true;
    }
    p.stars >= 3 && p.class_of - w.year <= 1
}

/// How much of a week a level's coaches have with their players today:
/// in season and spring ball (OTAs in the NFL) full, the rest of the year
/// a share (section 5; the off-season shares proposed, not confirmed).
fn phase(w: &World, stage: Stage, d: Date) -> f32 {
    let y = &w.dates;
    match stage {
        Stage::College => {
            if (d >= y.fall_camp && d <= y.bowl_season.1) || (d >= y.spring_practice.0 && d <= y.spring_game) {
                1.0
            } else {
                0.3
            }
        }
        Stage::Pro => {
            if d >= y.veterans_report.add(-7) && d <= y.super_bowl {
                1.0
            } else if d >= y.otas.0 && d <= y.minicamp.1 {
                0.5
            } else {
                0.15
            }
        }
        Stage::HighSchool => {
            let m = d.month();
            if (8..=11).contains(&m) { 1.0 } else { 0.4 }
        }
        _ => 0.0,
    }
}

// --- How it grows ---------------------------------------------------------------------

/// The coach's way with people (his hidden relationships rating): 0.6 at
/// 1, 1.0 at 10, 1.4 at 20.
pub fn rel_factor(r: f64) -> f32 {
    let r = r.clamp(1.0, 20.0) as f32;
    if r <= 10.0 { 0.6 + 0.4 * (r - 1.0) / 9.0 } else { 1.0 + 0.04 * (r - 10.0) }
}

/// His need for a father figure, 0.9-1.5 (question 2), and whether his own
/// father is close (then his top stage reads "Like family").
pub fn need(p: &crate::world::Person) -> (f32, bool) {
    let f = &p.family;
    let dad = f.structure == FamilyStructure::TwoParents
        && f.parent_style != ParentStyle::Absent
        && p.bg.home_stability >= 0.55;
    if dad {
        return (0.9, true);
    }
    let base = match f.structure {
        FamilyStructure::TwoParents => {
            if f.parent_style == ParentStyle::Absent {
                1.2
            } else {
                1.05
            }
        }
        FamilyStructure::SingleParent => 1.3,
        FamilyStructure::Grandparents => 1.35,
        FamilyStructure::Foster => 1.45,
    };
    let shaky = 0.25 * (0.6 - p.bg.home_stability as f32).max(0.0);
    let poor = if p.bg.money < 0.25 { 0.05 } else { 0.0 };
    ((base + shaky + poor).clamp(0.9, 1.5), false)
}

/// How open he is to a coach (section 5): loyalty and coachability open
/// him up, ego and a guarded temper close him, and a man burned by a coach
/// in the last year starts guarded.
pub(crate) fn openness_of(w: &World, p: &crate::world::Person) -> f32 {
    let t = &p.player.personality.traits;
    let z = |x: Trait| t.z(x) as f32;
    let coach = (p.player.personality.coachability as f32 - 10.0) / 10.0;
    let o =
        (1.0 + 0.25 * z(Trait::Loyalty) + 0.2 * coach - 0.25 * z(Trait::Ego) - 0.15 * z(Trait::Temper)).clamp(0.6, 1.4);
    let burned = w.folk.bonds.burned.get(&p.player.id).is_some_and(|d| d.days_to(w.today) < 365);
    if burned { o * 0.7 } else { o }
}

/// The kind of bond by the age gap (question 1) and his home (question 2).
pub fn kind(w: &World, i: usize, coach: usize) -> Kind {
    let p = &w.people[i];
    let gap = w.coaches[coach].age as i32 - p.player.age as i32;
    if gap < 12 {
        Kind::Brother
    } else if gap >= 40 {
        Kind::Grandfather
    } else if need(p).1 {
        Kind::Family
    } else {
        Kind::Father
    }
}

/// How a coach grew up, for "what they share": yours from your profile, an
/// AI coach's keyed (his generation's shares, proposed, not confirmed).
/// (structure, poor).
fn coach_home(w: &World, c: usize) -> (FamilyStructure, bool) {
    if you(w) == Some(c)
        && let Some(r) = w.career.as_ref().and_then(|k| raised_from(&k.you.profile.raised))
    {
        return r;
    }
    let u = super::roll(w, 0xB04D_0001, c as u64);
    let s = if u < 0.58 {
        FamilyStructure::TwoParents
    } else if u < 0.88 {
        FamilyStructure::SingleParent
    } else if u < 0.97 {
        FamilyStructure::Grandparents
    } else {
        FamilyStructure::Foster
    };
    (s, super::roll(w, 0xB04D_0002, c as u64) < 0.22)
}

/// Your profile's line on how you grew up (`Profile::raised`).
pub fn raised_from(s: &str) -> Option<(FamilyStructure, bool)> {
    let poor = s.contains("poor");
    let st = match s.split(',').next().unwrap_or("").trim() {
        "both parents" => FamilyStructure::TwoParents,
        "single mother" | "single father" | "single parent" => FamilyStructure::SingleParent,
        "grandparents" => FamilyStructure::Grandparents,
        "foster" => FamilyStructure::Foster,
        _ => return None,
    };
    Some((st, poor))
}

/// The ways you can say you grew up, for the profile.
pub const RAISED: [&str; 5] = ["both parents", "single mother", "single father", "grandparents", "foster"];

/// How well a player and a coach fit, 0.6-1.4 (section 5): how the coach
/// talks against how he wants to be coached, the likeness of their makeup,
/// and what they share (home state, his position, an upbringing alike,
/// heritage: each a small part, none driving).
pub fn fit(w: &World, i: usize, c: usize) -> f32 {
    let p = &w.people[i];
    let co = &w.coaches[c];
    let t = &p.player.personality.traits;
    let a = co.ability();
    let mut f = 1.0f32;
    // How he talks: an AI coach's manner on this player (coach voice).
    if you(w) != Some(c) {
        f += (4.0 * super::persona::manner::ai_fit(w, Some(c), i) as f32).clamp(-0.2, 0.2);
    }
    // Their makeup: two hot tempers click or clash (keyed), a hot coach
    // and a big ego clash, two loyal men hold, a demanding coach suits a
    // worker and grates on a man who isn't one.
    let temper = t.get(Trait::Temper);
    let (ct, cl) = (co.personality.temper, co.personality.loyalty);
    if temper >= 14.0 && ct >= 14.0 {
        f += if super::roll(w, 0xB04D_0003 ^ c as u64, p.player.id as u64) < 0.5 { 0.08 } else { -0.08 };
    }
    if t.get(Trait::Ego) >= 14.0 && ct >= 14.0 {
        f -= 0.05;
    }
    if t.get(Trait::Loyalty) >= 13.0 && cl >= 13.0 {
        f += 0.04;
    }
    if p.player.personality.work_ethic >= 14.0 && a.discipline >= 14.0 {
        f += 0.04;
    }
    if t.get(Trait::Professionalism) <= 6.0 && a.discipline >= 14.0 {
        f -= 0.05;
    }
    if p.player.personality.competitiveness >= 14.0 && a.motivation >= 14.0 {
        f += 0.04;
    }
    // What they share.
    if co.home_state == Some(p.home_state) {
        f += 0.05;
    }
    let group = Group::of(p.player.position);
    if co.played.as_ref().is_some_and(|x| Group::of(x.pos) == group) || co.came_up == group {
        f += 0.05;
    }
    let (home, poor) = coach_home(w, c);
    if home != FamilyStructure::TwoParents && home == p.family.structure {
        f += 0.05;
    }
    if poor && p.bg.money < 0.25 {
        f += 0.03;
    }
    if co.ethnicity == p.ethnicity {
        f += 0.04;
    }
    if you(w) == Some(c)
        && let Some(k) = w.career.as_ref()
        && !k.you.profile.high_school.is_empty()
        && w.high_schools.get(p.hs).is_some_and(|h| h.name.eq_ignore_ascii_case(&k.you.profile.high_school))
    {
        f += 0.05;
    }
    f.clamp(0.6, 1.4)
}

/// The room factor of a position coach: a man with 18 players has less of
/// himself to give than one with 6 (proposed, not confirmed).
fn room(n: u16) -> f32 {
    (10.0 / n.max(1) as f32).sqrt().clamp(0.6, 1.25)
}

/// Where a new bond starts: the recruiting relationship is its seed (a
/// recruit's tie to the staff that signed him, 0.05-0.25), anyone else at
/// 0 (section 6, "Recruiting").
fn start(w: &World, i: usize, c: usize, seat: Seat) -> (f32, Met) {
    let p = &w.people[i];
    match p.stage {
        Stage::HighSchool => (0.0, Met::HighSchool),
        Stage::Pro => (0.0, Met::Pro),
        _ => {
            if p.walk_on {
                return (0.0, Met::WalkOn);
            }
            if p.transfers > 0 && p.college_staff.iter().all(|s| Some(s.1) != p.college) {
                return (0.0, Met::Transfer);
            }
            let first = p.college_staff.first();
            let signed_by = match first {
                Some(s) => Some(s.1) == p.college && (s.2 == Some(c) || s.3 == Some(c)),
                // A freshman's first fall: the staff at his seats signed him.
                None => p.college_seasons == 0 && matches!(seat, Seat::Head | Seat::Position),
            };
            if signed_by {
                let tie = p.college.map_or(0.0, |j| super::super::recruiting::signee_tie(p, j, w.today));
                ((0.05 + 0.2 * tie as f32).clamp(0.05, 0.25), Met::Recruited)
            } else {
                (0.0, Met::Inherited)
            }
        }
    }
}

/// One week of every bond (Mondays, with the training week): each player
/// grows closer to the coaches working with him today and drifts from the
/// ones who don't.
pub(crate) fn week(w: &mut World, d: Date) {
    use rayon::prelude::*;
    let n = w.people.len();
    // Each program's room sizes by group, for the position coaches.
    let mut rooms = vec![[0u16; crate::world::coach::GROUP_COUNT]; w.programs.len()];
    for p in &w.people {
        if p.stage == Stage::College
            && let Some(j) = p.college
            && j < rooms.len()
        {
            rooms[j][Group::of(p.player.position) as usize] += 1;
        }
    }
    let me = you(w);
    let world: &World = w;
    // Work out each player's week on all cores from what it doesn't
    // change; then write it in id order.
    let weeks: Vec<(u32, Vec<Bond>, Vec<Moment>)> = (0..n)
        .into_par_iter()
        .filter_map(|i| {
            let p = &world.people[i];
            let id = p.player.id;
            let now = coaches_of(world, i);
            let had = world.folk.bonds.by.get(&id);
            if now.is_empty() && had.is_none() {
                return None;
            }
            let ph = phase(world, p.stage, d);
            let (need, _) = need(p);
            let open = openness_of(world, p);
            let mut bonds: Vec<Bond> = had.cloned().unwrap_or_default();
            let mut seeds = Vec::new();
            for &(c, seat) in &now {
                if !bonds.iter().any(|b| b.coach == c as u32) {
                    let (close, met) = start(world, i, c, seat);
                    let trust = if p.trust_in == c as u32 { p.trust } else { crate::world::people::fresh_trust(p) };
                    bonds.push(Bond {
                        coach: c as u32,
                        close,
                        best: close,
                        trust,
                        fit: fit(world, i, c),
                        met,
                        flags: 0,
                        extra: 0,
                        weeks: 0,
                    });
                    if met == Met::Recruited && close > 0.0 {
                        seeds.push(Moment {
                            id,
                            coach: c as u32,
                            on: d,
                            what: What::Recruited,
                            by: (close * 100.0).round() as i8,
                        });
                    }
                }
            }
            for b in bonds.iter_mut() {
                let c = b.coach as usize;
                let here = now.iter().find(|x| x.0 == c).map(|x| x.1);
                let co = &world.coaches[c];
                let gap = co.age as i32 - p.player.age as i32;
                let (speed, fade) = if gap < 12 {
                    (1.15, 1.0)
                } else if gap >= 40 {
                    (0.8, 0.5)
                } else {
                    (1.0, 1.0)
                };
                match here {
                    Some(seat) => {
                        let mut contact = seat.base() * ph;
                        if seat == Seat::Position
                            && let Some(j) = p.college
                        {
                            contact *= room(rooms[j][Group::of(p.player.position) as usize]);
                        }
                        contact += b.extra as f32;
                        b.extra = 0;
                        b.weeks = b.weeks.saturating_add(1);
                        // Trust: his head coach's is the built one; any
                        // other coach's drifts toward what the coach's way
                        // with people and a fair role give.
                        if p.trust_in == b.coach {
                            b.trust = p.trust;
                        } else {
                            let r = co.ability().relationships as f32;
                            let fair = if p.mood_why == crate::world::people::Why::Role as u8 { -0.1 } else { 0.03 };
                            let target = (0.5 + 0.15 * (r - 10.0) / 10.0 + fair).clamp(0.2, 0.85);
                            b.trust += 0.02 * contact.min(1.5) * (target - b.trust);
                        }
                        let rel = rel_factor(co.ability().relationships)
                            * if Some(c) == me { you_effort(world) } else { 1.0 };
                        let gain = GAIN * contact * b.fit * need * rel * open * speed * (1.0 - b.close);
                        let top = if b.hard() { 1.0 } else { TIME_TOP };
                        if b.close < top {
                            b.close = (b.close + gain).min(top);
                        }
                    }
                    None => {
                        let k = FADE_YEAR.powf(fade / 52.0);
                        b.close = (b.close * k).max(0.5 * b.best).min(b.close);
                    }
                }
                b.close = b.close.min(b.trust + OVER_TRUST).max(0.0);
                b.best = b.best.max(b.close);
            }
            Some((id, bonds, seeds))
        })
        .collect();
    let bonds = &mut w.folk.bonds;
    for (id, b, seeds) in weeks {
        bonds.by.insert(id, b);
        if !seeds.is_empty() {
            bonds.moments.entry(id).or_default().extend(seeds);
        }
    }
}

/// Your own way with people counts what you actually do (section 5): the
/// rating's factor is the same; what you do comes as talks (each a week of
/// contact) and as your choices at the moments.
fn you_effort(_w: &World) -> f32 {
    1.0
}

// --- The moments ----------------------------------------------------------------------

/// A moment between player `id` and coach `c`: closeness moves by `by`
/// times (1 + closeness) both ways (section 6), a grandfather's bond bends
/// less to a bad one; a hard moment opens the top stage. Kept as one of
/// the bond's moments when it is among the ones that moved it most.
pub(crate) fn moment(w: &mut World, id: u32, c: usize, what: What, by: f32) {
    let today = w.today;
    let gap =
        super::person(w, id).map(|i| w.coaches.get(c).map_or(20, |co| co.age as i32) - w.people[i].player.age as i32);
    let Some(b) = w.folk.bonds.get_mut(id, c) else { return };
    let mut d = by * (1.0 + b.close);
    if d < 0.0 && gap.is_some_and(|g| g >= 40) {
        d *= 0.7;
    }
    b.close = (b.close + d).clamp(0.0, 1.0).min(b.trust + OVER_TRUST);
    b.best = b.best.max(b.close);
    if what.hard() && by > 0.0 {
        b.flags |= 1;
    }
    let m = Moment { id, coach: c as u32, on: today, what, by: (d * 100.0).round().clamp(-100.0, 100.0) as i8 };
    let ms = w.folk.bonds.moments.entry(id).or_default();
    let mine: Vec<usize> = (0..ms.len()).filter(|&k| ms[k].coach == c as u32).collect();
    if mine.len() >= MOMENTS {
        // Drop the one that moved it least (the oldest of those).
        let k = *mine.iter().min_by_key(|&&k| (ms[k].by as i32).abs()).expect("non-empty");
        if (ms[k].by as i32).abs() > (m.by as i32).abs() {
            return;
        }
        ms.remove(k);
    }
    ms.push(m);
}

/// A moment for every coach working with him today, the AI ones by their
/// own ways (relationships and temper, keyed); you through `waiting`,
/// decided by what you do.
pub(crate) fn hard_moment(w: &mut World, id: u32, there: What, alone: What, up: f32, down: f32) {
    let Some(i) = super::person(w, id) else { return };
    let me = you(w);
    let today = w.today;
    for (c, _) in coaches_of(w, i) {
        if w.folk.bonds.get(id, c).is_none() {
            continue;
        }
        if Some(c) == me {
            w.folk.bonds.waiting.push(Waiting { id, coach: c as u32, on: today, there, alone, up, down });
            continue;
        }
        let a = w.coaches[c].ability();
        let shows =
            (0.35 + 0.05 * (a.relationships - 10.0) - 0.02 * (w.coaches[c].personality.temper - 10.0)).clamp(0.1, 0.9);
        let key = (id as u64) << 20 ^ today.0 as u64;
        if super::roll(w, 0xB04D_0010 ^ c as u64, key) < shows {
            moment(w, id, c, there, up);
        } else {
            moment(w, id, c, alone, down);
        }
    }
}

/// A one-on-one with you ended (`talks::finish_talk`): a week of contact,
/// and being there for any hard moment of his still waiting on you.
pub(crate) fn sat_down(w: &mut World, id: u32) {
    let Some(me) = you(w) else { return };
    let today = w.today;
    w.folk.bonds.talked.insert(id, today);
    if let Some(b) = w.folk.bonds.get_mut(id, me) {
        b.extra = b.extra.saturating_add(1).min(4);
    }
    let due: Vec<Waiting> =
        w.folk.bonds.waiting.iter().copied().filter(|x| x.id == id && x.on.days_to(today) <= 21).collect();
    w.folk.bonds.waiting.retain(|x| x.id != id);
    for x in due {
        moment(w, x.id, x.coach as usize, x.there, x.up);
    }
}

/// A one-on-one with you where something personal landed (personal talk,
/// item 8): one more week of contact with you, on top of the sit-down's own,
/// once a week at most. The caps stay: four extra weeks owed, and time alone
/// stops at `TIME_TOP` without a hard moment.
pub(crate) fn personal(w: &mut World, id: u32) {
    let Some(me) = you(w) else { return };
    let today = w.today;
    if w.folk.bonds.personal.get(&id).is_some_and(|d| d.days_to(today) < 7) {
        return;
    }
    w.folk.bonds.personal.insert(id, today);
    if let Some(b) = w.folk.bonds.get_mut(id, me) {
        b.extra = b.extra.saturating_add(1).min(4);
    }
}

/// His position coach had the talk you handed over (`talks::hand`): a week
/// of contact with him.
pub(crate) fn staff_talk(w: &mut World, id: u32, c: Option<usize>) {
    if let Some(c) = c
        && let Some(b) = w.folk.bonds.get_mut(id, c)
    {
        b.extra = b.extra.saturating_add(1).min(4);
    }
}

/// The hard moments you let pass: three weeks without a word from you.
fn settle_waiting(w: &mut World) {
    let today = w.today;
    let late: Vec<Waiting> = w.folk.bonds.waiting.iter().copied().filter(|x| x.on.days_to(today) > 21).collect();
    w.folk.bonds.waiting.retain(|x| x.on.days_to(today) <= 21);
    for x in late {
        moment(w, x.id, x.coach as usize, x.alone, x.down);
    }
}

/// A life event (`life.rs`): a death or a sick parent is a hard moment
/// for every coach with him (section 6: +0.2 there, -0.08 ignored, the
/// biggest; a sick parent +0.12 and -0.05, proposed, not confirmed).
pub(crate) fn life_event(w: &mut World, id: u32, e: super::life::Event) {
    use super::life::Event;
    match e {
        Event::Death => hard_moment(w, id, What::LossThere, What::LossAlone, 0.2, -0.08),
        Event::ParentIll => hard_moment(w, id, What::IllThere, What::IllAlone, 0.12, -0.05),
        _ => {}
    }
}

/// A long injury (four weeks or more): +0.1 for a coach who is there,
/// -0.05 ignored (section 6).
pub(crate) fn injured(w: &mut World, id: u32) {
    hard_moment(w, id, What::InjuryThere, What::InjuryAlone, 0.1, -0.05);
}

/// Leaving him (a cut, a trade): built costs grow with closeness, and the
/// bond takes the blow (section 6, "Leaving him").
pub(crate) fn left_him(w: &mut World, id: u32, c: Option<usize>, what: What) {
    let Some(c) = c else { return };
    if w.folk.bonds.get(id, c).is_none() {
        return;
    }
    let today = w.today;
    w.folk.bonds.burned.insert(id, today);
    moment(w, id, c, what, -0.15);
}

/// How much a betrayal costs next to the built cost: x (1 + closeness)
/// (section 8).
pub fn betrayal(w: &World, id: u32, c: Option<usize>) -> f32 {
    1.0 + close(w, id, c)
}

/// A broken promise from coach `c` (`talks::settle_word`): the bond's
/// moment, and he's guarded for a year.
pub(crate) fn broke_word(w: &mut World, id: u32, c: Option<usize>) {
    let Some(c) = c else { return };
    if w.folk.bonds.get(id, c).is_none() {
        return;
    }
    let today = w.today;
    w.folk.bonds.burned.insert(id, today);
    moment(w, id, c, What::PromiseBroken, -0.06);
}

/// A first start (fall camp's roles): +0.08 with the head coach and his
/// position coach (section 6, "His chance").
pub(crate) fn first_start(w: &mut World, id: u32) {
    let Some(i) = super::person(w, id) else { return };
    for (c, seat) in coaches_of(w, i) {
        if matches!(seat, Seat::Head | Seat::Position | Seat::HsHead) {
            moment(w, id, c, What::FirstStart, 0.08);
        }
    }
}

/// A walk-on put on scholarship: +0.15 with his head coach.
pub(crate) fn scholarship(w: &mut World, id: u32) {
    let Some(i) = super::person(w, id) else { return };
    if let Some((c, _)) = coaches_of(w, i).into_iter().find(|x| x.1 == Seat::Head) {
        moment(w, id, c, What::Scholarship, 0.15);
    }
}

/// Benched: told to his face (a talk about it with you, or his position
/// coach's word) moves trust a little; finding out from the depth chart
/// costs closeness (section 6).
pub(crate) fn benched(w: &mut World, id: u32, c: usize, told: bool) {
    if told {
        if let Some(b) = w.folk.bonds.get_mut(id, c) {
            b.trust = (b.trust + 0.02).min(1.0);
        }
        moment(w, id, c, What::BenchedTold, 0.0);
    } else {
        moment(w, id, c, What::BenchedChart, -0.05);
    }
}

/// Trouble: a coach who stood by him with a fair punishment (+0.1).
pub(crate) fn stood_by(w: &mut World, id: u32, c: usize) {
    moment(w, id, c, What::StoodBy, 0.1);
}

/// Commencement (May): a coach who is there, +0.05 (AI coaches by their
/// way with people, keyed; you, if you're his head coach).
pub(crate) fn graduation(w: &mut World, ids: &[u32]) {
    let me = you(w);
    for &id in ids {
        let Some(i) = super::person(w, id) else { continue };
        for (c, _) in coaches_of(w, i) {
            if w.folk.bonds.get(id, c).is_none() {
                continue;
            }
            let shows = Some(c) == me
                || super::roll(w, 0xB04D_0020 ^ c as u64, id as u64)
                    < (0.4 + 0.04 * (w.coaches[c].ability().relationships - 10.0)).clamp(0.1, 0.85);
            if shows {
                moment(w, id, c, What::Graduation, 0.05);
            }
        }
    }
}

/// The season's end for a college program (`hard seasons survived`): a
/// losing year or a title won, for everyone who came through it with this
/// staff (+0.03 and +0.06; section 6).
pub(crate) fn season_end(w: &mut World, prog: usize, win_share: f64, title: bool) {
    let what = if title {
        What::Title
    } else if win_share < 0.35 {
        What::HardSeason
    } else {
        return;
    };
    let by = if title { 0.06 } else { 0.03 };
    let roster: Vec<usize> = w.programs[prog].roster.clone();
    for i in roster {
        if w.people[i].stage != Stage::College || w.people[i].college != Some(prog) {
            continue;
        }
        let id = w.people[i].player.id;
        for (c, _) in coaches_of(w, i) {
            if w.folk.bonds.get(id, c).is_some_and(|b| b.weeks >= 20) {
                moment(w, id, c, what, by);
            }
        }
    }
}

// --- What it does ---------------------------------------------------------------------

/// His closeness to his head coach and his position coach today (the
/// coordinator for a seat with no position coach): (head, position).
pub fn staff_close(w: &World, i: usize) -> (f32, f32) {
    let id = w.people[i].player.id;
    let mut head = 0.0f32;
    let mut pos = 0.0f32;
    for (c, seat) in coaches_of(w, i) {
        let x = close(w, id, Some(c));
        match seat {
            Seat::Head | Seat::HsHead => head = head.max(x),
            _ => pos = pos.max(x),
        }
    }
    (head, pos)
}

/// The closeness of the coach who teaches him most (his position coach in
/// college, the coordinator in the pros, his high school head coach), for
/// how much of the coaching sticks (question 4: up to +3 coachability).
pub fn teacher_close(w: &World, i: usize) -> f32 {
    let id = w.people[i].player.id;
    let now = coaches_of(w, i);
    let pick = now
        .iter()
        .find(|x| x.1 == Seat::Position)
        .or_else(|| now.iter().find(|x| matches!(x.1, Seat::HsHead | Seat::Coordinator)))
        .map(|x| x.0);
    close(w, id, pick)
}

/// Coachability points a close bond adds to how much of his coaching
/// sticks (question 4: at most 3).
pub fn coachability_lift(w: &World, i: usize) -> f64 {
    3.0 * teacher_close(w, i) as f64
}

/// Instructions followed (`tendencies::obey_chance`, through the game's
/// `GamePlayer::bond`): his closeness to his head coach.
pub fn obey_bond(w: &World, i: usize) -> f32 {
    staff_close(w, i).0
}

/// A talk from coach `c` lands harder by 0.3 x his closeness (section 7,
/// "Talks"): what the built ear (0.6 + 0.8 x trust) adds.
pub fn ear(w: &World, id: u32, c: Option<usize>) -> f32 {
    0.3 * close(w, id, c)
}

/// The ear added for a talk from you.
pub fn ear_you(w: &World, id: u32) -> f32 {
    ear(w, id, you(w))
}

/// His step with you: at "Trusts you" he tells you things whatever his
/// trust (section 7, "Telling you things").
pub fn step_with(w: &World, id: u32, c: Option<usize>) -> Step {
    c.and_then(|c| bond(w, id, c)).map_or(Step::JustMet, Step::of)
}

pub fn tells_you(w: &World, id: u32) -> bool {
    step_with(w, id, you(w)) >= Step::TrustsYou
}

/// Staying (section 7): his portal lean x (1 - 0.4 x closeness to the
/// head coach) x (1 - 0.25 x closeness to his position coach).
pub fn stay_factor(w: &World, i: usize) -> f64 {
    let (h, p) = staff_close(w, i);
    ((1.0 - 0.4 * h) * (1.0 - 0.25 * p)) as f64
}

/// The rating points an AI coach's read of a man he's close to rises by
/// when he sets his depth chart: the real "coach's guy" bias, inside the
/// read's own noise (section 8; proposed, not confirmed: up to 1.5).
pub fn guy_bias(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    let Some(j) = p.college else { return 0.0 };
    let head = w.programs.get(j).and_then(|x| x.head_coach);
    if head.is_none() || head == you(w) {
        return 0.0;
    }
    1.5 * close(w, p.player.id, head) as f64
}

/// A pro's or a player's tie to a coach for the built following reads:
/// what the old tie gave (years under him, recruited, a pro's seasons),
/// half of it, with his closeness (section 7, "Following"; proposed, not
/// confirmed: half the old tie plus his closeness).
pub fn follow_tie(w: &World, id: u32, c: usize, built: f32) -> f32 {
    (0.5 * built + close(w, id, Some(c))).clamp(0.0, 1.0)
}

/// The trust a new head coach starts with in a room that loved the old one
/// (question 5): down by 0.15 x the room's average closeness to him.
pub fn successor_cold(w: &World, roster: &[usize], old: u32) -> f32 {
    if old == u32::MAX || roster.is_empty() {
        return 0.0;
    }
    let sum: f32 = roster.iter().map(|&i| close(w, w.people[i].player.id, Some(old as usize))).sum();
    0.15 * sum / roster.len() as f32
}

/// A coach leaves a job (`staff::leave`): every player close to him feels
/// it, mood -0.25 x closeness (at "Family" the size of a death in the
/// family), and a high-need kid's trouble risk rises for a season
/// (section 8).
pub(crate) fn coach_left(w: &mut World, coach: usize, employer: crate::world::coach::Employer) {
    use crate::world::coach::Employer;
    let roster: Vec<usize> = match employer {
        Employer::College(j) => w.programs.get(j).map_or_else(Vec::new, |p| p.roster.clone()),
        Employer::HighSchool(h) => w.high_schools.get(h).map_or_else(Vec::new, |s| s.roster.clone()),
        Employer::Nfl(t) => {
            (0..w.people.len()).filter(|&i| w.people[i].pro.as_ref().is_some_and(|c| c.team == Some(t))).collect()
        }
        Employer::Other(_) => Vec::new(),
    };
    let mut mine: Vec<usize> = super::Team::seat(w).map_or(Vec::new(), |t| t.roster(w));
    mine.sort_unstable();
    let today = w.today;
    for i in roster {
        let id = w.people[i].player.id;
        let c = close(w, id, Some(coach));
        if c < 0.15 {
            continue;
        }
        let hurt = -0.25 * c;
        if mine.binary_search(&i).is_ok() {
            super::talks::mood_event(w, id, super::talks::Reason::CoachLeft, hurt);
        } else {
            let p = &mut w.people[i];
            p.mood = (p.mood + hurt).clamp(-1.0, 1.0);
        }
        moment(w, id, coach, What::Left, 0.0);
        // A coach who left him: guarded with the next one for a year.
        w.folk.bonds.burned.insert(id, today);
        if need(&w.people[i]).0 >= 1.25 {
            w.folk.bonds.lost.insert(id, (today, c));
        }
    }
}

/// A high-need kid whose father figure left in the last year: his trouble
/// risk x (1 + 0.5 x how close he was) (section 8; proposed, not
/// confirmed).
pub fn loss_risk(w: &World, id: u32) -> f32 {
    w.folk.bonds.lost.get(&id).filter(|x| x.0.days_to(w.today) < 365).map_or(1.0, |x| 1.0 + 0.5 * x.1)
}

/// Favoritism at fall camp (section 8): at each spot, a man the head coach
/// is close to ("Trusts you" or more) who starts over a teammate rated 3
/// points better costs the passed-over man trust in that coach (0.012 a
/// point, up to 6) and leans him toward the portal; the room's mood dips.
/// The room watches fairness, not closeness: a fair chart costs nothing.
pub(crate) fn favoritism(w: &mut World, roles: &[(usize, crate::world::PlayingTime)]) {
    use crate::world::PlayingTime;
    w.folk.bonds.passed.clear();
    let role: std::collections::HashMap<usize, PlayingTime> = roles.iter().copied().collect();
    let mut hits: Vec<(usize, usize, f32)> = Vec::new();
    for (j, prog) in w.programs.iter().enumerate() {
        let Some(head) = prog.head_coach else { continue };
        let mut by_pos: [Vec<usize>; crate::positions::POSITION_COUNT] = Default::default();
        for &i in &prog.roster {
            if role.contains_key(&i) {
                by_pos[w.people[i].player.position.index()].push(i);
            }
        }
        for v in by_pos.iter() {
            let starters: Vec<usize> = v.iter().copied().filter(|i| role[i] == PlayingTime::Starter).collect();
            let rest: Vec<usize> = v.iter().copied().filter(|i| role[i] != PlayingTime::Starter).collect();
            for &s in &starters {
                let b = w.folk.bonds.get(w.people[s].player.id, head);
                if !b.is_some_and(|b| Step::of(b) >= Step::TrustsYou) {
                    continue;
                }
                let so = w.people[s].player.overall();
                if let Some(&best) =
                    rest.iter().max_by(|&&a, &&b| w.people[a].player.overall().total_cmp(&w.people[b].player.overall()))
                {
                    let gap = (w.people[best].player.overall() - so) as f32;
                    if gap >= 3.0 && w.people[best].player.position == w.people[s].player.position {
                        hits.push((j, best, gap.min(6.0)));
                    }
                }
            }
        }
    }
    for (j, i, gap) in hits {
        let id = w.people[i].player.id;
        let head = w.programs[j].head_coach;
        let p = &mut w.people[i];
        if head.is_some_and(|h| p.trust_in == h as u32) {
            p.trust = (p.trust - 0.012 * gap).max(0.0);
        }
        w.folk.bonds.passed.insert(id, gap);
        w.programs[j].team_mood = (w.programs[j].team_mood - 0.01 * gap / 6.0).clamp(-1.0, 1.0);
    }
}

/// The portal lean of a man passed over for the coach's guy this season:
/// x (1 + 0.04 x the gap), up to 1.24 (proposed, not confirmed).
pub fn passed_over(w: &World, id: u32) -> f64 {
    w.folk.bonds.passed.get(&id).map_or(1.0, |g| 1.0 + 0.04 * *g as f64)
}

/// Each summer: the fit of every bond read again (people change), the old
/// moments and the bonds of people who left football dropped.
pub(crate) fn year(w: &mut World) {
    let today = w.today;
    let ids: std::collections::BTreeSet<u32> = w.people.iter().map(|p| p.player.id).collect();
    w.folk.bonds.by.retain(|id, _| ids.contains(id));
    w.folk.bonds.moments.retain(|id, _| ids.contains(id));
    w.folk.bonds.lost.retain(|_, x| x.0.days_to(today) < 365);
    w.folk.bonds.burned.retain(|_, d| d.days_to(today) < 365);
    w.folk.bonds.talked.retain(|_, d| d.days_to(today) < 60);
    w.folk.bonds.waiting.retain(|x| x.on.days_to(today) <= 21);
    let refits: Vec<(u32, Vec<f32>)> = {
        use rayon::prelude::*;
        let world: &World = w;
        world
            .folk
            .bonds
            .by
            .par_iter()
            .filter_map(|(&id, bonds)| {
                let i = super::person(world, id)?;
                Some((id, bonds.iter().map(|b| fit(world, i, b.coach as usize)).collect()))
            })
            .collect()
    };
    for (id, fits) in refits {
        if let Some(v) = w.folk.bonds.by.get_mut(&id) {
            for (b, f) in v.iter_mut().zip(fits) {
                b.fit = f;
            }
        }
    }
}

/// The evening's part: the hard moments you let pass; on Mondays the
/// week's growth; each July 1 the year's refit.
pub(crate) fn day(w: &mut World, d: Date) {
    if !w.folk.bonds.waiting.is_empty() {
        settle_waiting(w);
    }
    if d.weekday() == crate::calendar::Weekday::Mon {
        week(w, d);
    }
    if d.month() == 7 && d.parts().2 == 1 {
        year(w);
    }
}

// --- What you see ---------------------------------------------------------------------

/// One bond as your screens show it: no number, the stage in words.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Seen {
    pub coach: usize,
    pub name: String,
    pub role: String,
    pub step: String,
    pub sign: String,
    pub met: String,
    pub moments: Vec<(Date, String)>,
}

/// His bonds with the coaches working with him, closest first: yours with
/// what built it, your staff's in their words (section 9).
pub fn seen(w: &World, i: usize) -> Vec<Seen> {
    let id = w.people[i].player.id;
    let mut out: Vec<(f32, Seen)> = Vec::new();
    let me = you(w);
    let Some(bonds) = w.folk.bonds.by.get(&id) else { return Vec::new() };
    for b in bonds {
        let c = b.coach as usize;
        let current = coaches_of(w, i).into_iter().find(|x| x.0 == c);
        if current.is_none() && Some(c) != me && b.close < 0.35 {
            continue;
        }
        let step = Step::of(b);
        let k = kind(w, i, c);
        let mut moments: Vec<(Date, String)> = w
            .folk
            .bonds
            .moments
            .get(&id)
            .into_iter()
            .flatten()
            .filter(|m| m.coach == b.coach)
            .map(|m| (m.on, m.what.words().to_string()))
            .collect();
        moments.sort_by_key(|a| std::cmp::Reverse(a.0));
        let role = w.coaches[c].job.map_or_else(|| "former coach".to_string(), |j| j.role.name().to_string());
        out.push((
            if Some(c) == me { 2.0 } else { b.close },
            Seen {
                coach: c,
                name: if Some(c) == me { "You".to_string() } else { w.coaches[c].name.clone() },
                role,
                step: step.words(k).to_string(),
                sign: step.sign().to_string(),
                met: b.met.words().to_string(),
                moments,
            },
        ));
    }
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    out.into_iter().map(|x| x.1).collect()
}

/// Your staff's word on a player's bonds: who he's closest to, as sure as
/// they can be ("closest to his position coach; he'd follow him anywhere").
pub fn staff_words(w: &World, i: usize) -> Option<String> {
    let id = w.people[i].player.id;
    let me = you(w);
    let (c, b) = coaches_of(w, i)
        .into_iter()
        .filter(|x| Some(x.0) != me)
        .filter_map(|(c, s)| bond(w, id, c).map(|b| ((c, s), *b)))
        .max_by(|a, b| a.1.close.total_cmp(&b.1.close))?;
    let who = match c.1 {
        Seat::Head => "his head coach",
        Seat::Coordinator => "his coordinator",
        Seat::Position => "his position coach",
        Seat::HsHead => "his high school coach",
        Seat::HsCoordinator => "his high school coordinator",
    };
    let step = Step::of(&b);
    Some(match step {
        Step::JustMet => return None,
        Step::KnowsYou => format!("Gets on with {who}."),
        Step::TrustsYou => format!("Closest to {who}; he takes his problems to him."),
        Step::InHisCorner => format!("Closest to {who}; he'd follow him anywhere."),
        Step::Family => format!("{who} is {} to him.", kind_words(kind(w, i, c.0))).replacen('h', "H", 1),
    })
}

fn kind_words(k: Kind) -> &'static str {
    match k {
        Kind::Brother => "like a big brother",
        Kind::Father => "like a father",
        Kind::Grandfather => "like a grandfather",
        Kind::Family => "like family",
    }
}

/// A coach's closest players today (his page): name ids and the step, the
/// closest first, at "Trusts you" or more.
pub fn closest(w: &World, coach: usize, n: usize) -> Vec<(u32, String)> {
    let mut v: Vec<(f32, u32, String)> = Vec::new();
    for (&id, bonds) in &w.folk.bonds.by {
        if let Some(b) = bonds.iter().find(|b| b.coach == coach as u32) {
            let s = Step::of(b);
            if s >= Step::TrustsYou
                && let Some(i) = super::person(w, id)
            {
                v.push((b.close, id, s.words(kind(w, i, coach)).to_string()));
            }
        }
    }
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().take(n).map(|x| (x.1, x.2)).collect()
}

/// His step with you and its kind in words, for the talk card ("Trusts
/// you", "Like a father"), and the moments that built it, newest first.
pub fn with_you(w: &World, id: u32) -> Option<(String, Vec<What>)> {
    let me = you(w)?;
    let i = super::person(w, id)?;
    let b = bond(w, id, me)?;
    let step = Step::of(b).words(kind(w, i, me)).to_string();
    let mut ms: Vec<&Moment> =
        w.folk.bonds.moments.get(&id).into_iter().flatten().filter(|m| m.coach == me as u32).collect();
    ms.sort_by_key(|a| std::cmp::Reverse(a.on));
    Some((step, ms.into_iter().map(|m| m.what).collect()))
}
