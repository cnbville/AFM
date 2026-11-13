//! Your points to a recruit or a transfer (talk facts design, section 6,
//! "The points" and "How strong your proof is"): about thirty kinds in
//! seven groups, each a fact about a program that can be named, with your
//! proof graded against the same fact at his other top schools.
//!
//! Every fact is read the same way for every school, so the grade is fair:
//! `fact(c, j)` gives the fact's own size at program `j` (0-1) and, for
//! the program the sheet is for, the point with its real values. A fact a
//! school doesn't have (no brother there, not his childhood team) is no
//! point at all, and counts as nothing when another school is graded
//! against it.
//!
//! How much he cares comes from what he (or whoever decides at home) wants
//! from a school. Those wants are his own: they set `Point.care` and
//! `Point.mind`, engine numbers no line or screen shows. What you learn of
//! them comes from talking to him (section 7).

use super::alumni::{Alumni, Index};
use super::kinds::{Group, Kind, Whose};
use super::sheet::{ALUM_OTHERS, Point, Ref, Tier};
use super::{Reader, words};
use crate::positions::Position;
use crate::world::background::{Decider, FamilyStructure, Wants};
use crate::world::coach::Group as Room;
use crate::world::folk::{Team, facts};
use crate::world::program::GradeCat;
use crate::world::{Person, Stage, World, recruiting as rec};
use std::sync::Arc;

/// Who listens to your points: the recruit himself (a call, a message, a
/// transfer), or whoever decides at his home (the living room, the visit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ear {
    Himself,
    Decider,
}

/// Picks at his position in ten years that make the fact as big as it
/// gets. Proposed, not confirmed.
pub const PICKS_FULL: f32 = 6.0;
/// The same for the players a coach had himself.
/// Proposed, not confirmed.
pub const GUYS_FULL: f32 = 8.0;
/// Schoolmates on your roster or in your class that make that fact full.
/// Proposed, not confirmed.
pub const MATES_FULL: f32 = 2.0;
/// A following that makes him a kid with a name. Proposed, not confirmed.
pub const BRAND_FOLLOWERS: u32 = 25_000;
/// Household money under this is a family in need (0 poverty, 1 wealthy).
/// Proposed, not confirmed.
pub const NEED_MONEY: f64 = 0.3;
/// A high school pipeline from this strength is a tie worth naming.
/// Proposed, not confirmed.
pub const PIPELINE_TIE: f64 = 0.15;
/// A drive a family makes on a Saturday morning, in miles.
/// Proposed, not confirmed.
pub const DAY_DRIVE: f64 = 300.0;
/// A head coach whose last two seasons ran this far under what was
/// expected is on the hot seat (as the grapevine reads it).
/// Proposed, not confirmed.
pub const HOT_SEAT: f64 = -0.12;
/// How much he cares is kept inside these, in shares of an even split
/// (1.0: a want of average weight).
/// Proposed, not confirmed.
pub const CARE_RANGE: (f32, f32) = (0.2, 2.5);
/// How big a planned role is as a fact: starter, rotation, backup.
/// Proposed, not confirmed.
pub const ROLE_SIZE: [f32; 3] = [0.95, 0.6, 0.25];
/// Your offer reads as over the going rate from this share of it, under
/// it below this share. Proposed, not confirmed.
pub const OFFER_OVER: f64 = 1.15;
/// Below this share of the going rate an offer reads as under it. Proposed, not confirmed.
pub const OFFER_UNDER: f64 = 0.85;
/// A head coach is new before this many seasons, and long in the job
/// from this many.
/// Proposed, not confirmed.
pub const NEW_SEASONS: i32 = 2;
/// Long in the job from this many seasons. Proposed, not confirmed.
pub const LONG_SEASONS: i32 = 8;
/// The title-chances grade (0-1) that reads as a contender, and as in
/// the hunt. Proposed, not confirmed.
pub const CONTENDER: f32 = 0.9;
/// In the hunt from this grade. Proposed, not confirmed.
pub const IN_THE_HUNT: f32 = 0.75;
/// A brother he wants to follow weighs this much more on his mind, one
/// he wants out from under this much less.
/// Proposed, not confirmed.
pub const BROTHER_PULL: (f32, f32) = (1.5, 0.5);

pub(super) struct Ctx<'a> {
    pub r: &'a Reader<'a>,
    pub w: &'a World,
    pub i: usize,
    pub p: &'a Person,
    pub pos: Position,
    pub transfer: bool,
    pub ix: Arc<Index>,
}

impl<'a> Ctx<'a> {
    pub fn new(r: &'a Reader<'a>, i: usize) -> Ctx<'a> {
        let p = &r.w.people[i];
        Ctx {
            r,
            w: r.w,
            i,
            p,
            pos: p.player.position,
            transfer: p.cal.portal.is_some() || p.stage == Stage::College,
            ix: Alumni::get(r.w),
        }
    }
    fn grade(&self, j: usize, c: GradeCat) -> f32 {
        (self.w.programs[j].grades[c as usize] / 100.0).clamp(0.0, 1.0) as f32
    }
    pub fn mine(&self, j: usize) -> bool {
        Team::seat(self.w) == Some(Team::College(j))
    }
}

/// His other top schools: the best six on his list besides `prog`, as his
/// race stands today.
pub(super) fn others(c: &Ctx, prog: usize) -> Vec<usize> {
    let race =
        if c.transfer { rec::portal_race(c.w, c.i) } else { crate::world::front_office::college::race_at(c.w, c.i) };
    race.into_iter().map(|x| x.0).filter(|&j| j != prog).take(crate::world::front_office::race::SCHOOLS).collect()
}

/// How much each group weighs with the listener, in shares of an even
/// split: his six wants, and for the people his trust in a coach's word
/// (as the pitches read it today, `dialogue::wants`).
pub(super) fn cares(p: &Person, ear: Ear) -> [f32; 7] {
    let f = &p.family;
    let own = ear == Ear::Himself || f.decider == Decider::Himself;
    // hidden: what he wants from a school sets how much he cares (an
    // engine number), never a value.
    let w: &Wants = if own { &f.own_wants } else { &f.decider_wants };
    let total = (w.money + w.close_to_home + w.playing_time + w.academics + w.development + w.prestige).max(1e-6);
    let share = |x: f64| ((6.0 * x / total) as f32).clamp(CARE_RANGE.0, CARE_RANGE.1);
    let people = if own {
        if p.cal.portal.is_some_and(|e| e.why == "coaching change") { 0.8 } else { 0.5 }
    } else {
        let base = if f.decider == Decider::Handler { 0.7 } else { 1.0 };
        base + if matches!(f.structure, FamilyStructure::SingleParent | FamilyStructure::Grandparents) {
            0.2
        } else {
            0.0
        }
    };
    let mut out = [1.0; 7];
    for g in Group::ALL {
        out[g as usize] = match g {
            Group::PlayingTime => share(w.playing_time),
            Group::Money => share(w.money),
            Group::School => share(w.academics),
            Group::Home => share(w.close_to_home),
            Group::People => people,
            Group::League => share(w.development),
            Group::Winning => share(w.prestige),
        };
    }
    out
}

type Fact = fn(&Ctx, usize) -> Option<(f32, Point)>;

/// Every point a program can make, in the catalogue's order.
const FACTS: [(Kind, Fact); 33] = [
    (Kind::Room, room),
    (Kind::Role, role),
    (Kind::QbRivals, qb_rivals),
    (Kind::Scheme, scheme),
    (Kind::OfferRate, offer_rate),
    (Kind::OfferOthers, offer_others),
    (Kind::FamilyNeed, family_need),
    (Kind::Brand, brand),
    (Kind::Degree, degree),
    (Kind::Support, support),
    (Kind::AfterFootball, after_football),
    (Kind::Drive, drive),
    (Kind::ChildhoodTeam, childhood_team),
    (Kind::HomeState, home_state),
    (Kind::HsMates, hs_mates),
    (Kind::Brother, brother),
    (Kind::Tenure, tenure),
    (Kind::PositionCoach, position_coach),
    (Kind::HsCoachTie, hs_coach_tie),
    (Kind::YourWord, your_word),
    (Kind::WhyLeft, why_left),
    (Kind::PicksAtPosition, picks_position),
    (Kind::PicksOverall, picks_all),
    (Kind::YourGuys, your_guys),
    (Kind::PositionU, position_u),
    (Kind::ProDay, pro_day),
    (Kind::Titles, titles),
    (Kind::TitleOdds, title_odds),
    (Kind::Stadium, stadium),
    (Kind::Exposure, exposure),
    (Kind::Conference, conference),
    (Kind::Rival, rival),
    (Kind::Buildings, buildings),
];

pub(super) fn points(r: &Reader, i: usize, prog: usize, ear: Ear, out: &mut Vec<Point>) {
    if prog >= r.w.programs.len() {
        return;
    }
    let c = Ctx::new(r, i);
    let rest = others(&c, prog);
    let care = cares(c.p, ear);
    for (kind, fact) in FACTS {
        let Some((mine, pt)) = fact(&c, prog) else { continue };
        debug_assert_eq!(pt.kind, kind);
        let g = kind.group().map_or(1.0, |g| care[g as usize]);
        let pt = if pt.whose == Whose::His {
            pt
        } else {
            let theirs: Vec<f32> = rest.iter().map(|&o| fact(&c, o).map_or(0.0, |x| x.0)).collect();
            pt.proof(mine, Tier::grade(mine, &theirs), theirs.len())
        };
        out.push(pt.care(g));
    }
}

fn unit(x: f64) -> f32 {
    x.clamp(0.0, 1.0) as f32
}

// ----- Playing time.

/// The room at his spot: who will still be there when he arrives, and how
/// many of them leave after this season. Counts and classes only, never a
/// rating.
fn room(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let w = c.w;
    let at: Vec<usize> = w.programs[j]
        .roster
        .iter()
        .copied()
        .filter(|&k| k != c.i && w.people[k].player.position == c.pos && w.people[k].stage == Stage::College)
        .collect();
    let leaving = at.iter().filter(|&&k| facts::leaving(w, k)).count();
    let staying = at.len() - leaving;
    let slots = crate::game::roster::starters(c.pos).max(1);
    let strength = unit(1.0 - staying as f64 / (3.0 * slots as f64));
    let depth = if staying <= slots {
        "thin"
    } else if staying <= 2 * slots {
        "normal"
    } else {
        "crowded"
    };
    Some((
        strength,
        Point::new(Kind::Room)
            .text("spot", words::spot(c.pos))
            .count("staying", staying as i64)
            .count("leaving", leaving as i64)
            .count("starters", slots as i64)
            .state("depth", depth),
    ))
}

/// The role a staff plans for a transfer while he shops: starter, in the
/// rotation or a backup. He can check it at fall camp. A high school
/// recruit has no planned role in the world.
fn role(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    use crate::player::PlayingTime as T;
    let (role, ..) = rec::portal_place(c.w, c.i, j)?;
    let (key, strength) = match role {
        T::Starter => ("starter", ROLE_SIZE[0]),
        T::Rotation => ("rotation", ROLE_SIZE[1]),
        _ => ("backup", ROLE_SIZE[2]),
    };
    let camp = c.w.dates.fall_camp;
    let due = if camp >= c.r.today { camp } else { camp.add(364) };
    Some((strength, Point::new(Kind::Role).due(due).text("spot", words::spot(c.pos)).state("role", key)))
}

/// A quarterback's rivals: the blue-chip quarterbacks in the room who
/// will still be there in his first seasons, as a family reads the depth
/// chart it can see (their stars and their class).
fn qb_rivals(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    if c.pos != Position::QB {
        return None;
    }
    let w = c.w;
    let season = w.dates.season;
    let mut rivals: Vec<(f64, usize)> = w.programs[j]
        .roster
        .iter()
        .copied()
        .filter(|&k| {
            let q = &w.people[k];
            k != c.i && q.player.position == Position::QB && q.stage == Stage::College && q.cal.portal.is_none()
        })
        .filter_map(|k| {
            let q = &w.people[k];
            let pedigree = match rec::recruiting_stars(q) {
                5.. => 1.0,
                4 => 0.5,
                _ => return None,
            };
            let left = (5 - crate::world::eligibility::school_year(q, season)).clamp(0, 3) as f64 / 3.0;
            (left > 0.0).then_some((pedigree * left, k))
        })
        .collect();
    rivals.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let load: f64 = rivals.iter().map(|x| x.0).sum::<f64>().min(2.0);
    let mut pt = Point::new(Kind::QbRivals)
        .count("rivals", rivals.len() as i64)
        .state("path", if rivals.is_empty() { "open" } else { "blocked" });
    if let Some(&(_, k)) = rivals.first() {
        pt = pt
            .text("rival", w.people[k].name.clone())
            .text("rival_class", facts::class_words(w, k))
            .refer(Ref::Player(w.people[k].player.id));
    }
    Some((unit(1.0 - load / 2.0), pt))
}

/// The scheme and his position in it: the system by name. How well it
/// shows him off is the fact's size, never said.
fn scheme(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    use crate::world::systems;
    let inst = &c.w.programs[j].installed;
    let system = systems::system_words(c.pos, inst)?;
    let fits = systems::fits(&c.p.player, c.pos, &c.w.system_norms);
    let show = systems::showcase(&fits, c.pos, inst);
    let known = if words::on_offense(c.pos) { inst.off_known } else { inst.def_known };
    Some((
        unit((show + 1.0) / 2.0),
        Point::new(Kind::Scheme)
            .text("system", system)
            .text("spot", words::spot(c.pos))
            .state("new", if known < 1.0 { "yes" } else { "no" }),
    ))
}

// ----- Money.

fn dollars(x: f64) -> String {
    let n = (x / 1000.0).round() as i64 * 1000;
    let s = n.to_string();
    let mut out = String::new();
    for (k, ch) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    format!("${out}")
}

/// What signing day pays: checked on the day he signs.
fn money_day(c: &Ctx) -> crate::calendar::Date {
    let y = &c.w.dates;
    [y.early_signing, Some(y.signing_day)]
        .into_iter()
        .flatten()
        .find(|&d| d >= c.r.today)
        .unwrap_or(y.signing_day.add(364))
}

/// Your offer against the going rate at his spot: what your program
/// usually pays a recruit like him. The number is yours to say.
fn offer_rate(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let offer = rec::offer_money(c.w, j, c.i);
    let usual = rec::usual_nil_offer(c.w, j, c.i);
    if offer <= 0.0 || usual <= 0.0 {
        return None;
    }
    let ratio = offer / usual;
    let gap = if ratio >= OFFER_OVER {
        "over"
    } else if ratio <= OFFER_UNDER {
        "under"
    } else {
        "at"
    };
    Some((
        unit(ratio / 1.5),
        Point::new(Kind::OfferRate)
            .due(money_day(c))
            .text("offer", dollars(offer))
            .text("spot", words::spot(c.pos))
            .state("gap", gap),
    ))
}

/// Your offer against his other offers: its size is what the grade
/// against his list reads.
fn offer_others(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let offer = rec::offer_money(c.w, j, c.i);
    if offer <= 0.0 {
        return None;
    }
    Some((
        unit((offer / 1.0e6).powf(0.6)),
        Point::new(Kind::OfferOthers).due(money_day(c)).text("offer", dollars(offer)),
    ))
}

/// What the money does for a family in need. You know the need once your
/// staff has been in touch with the family.
fn family_need(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let offer = rec::offer_money(c.w, j, c.i);
    let money = c.p.family_money();
    if offer <= 0.0 || money >= NEED_MONEY {
        return None;
    }
    let need = if money < NEED_MONEY / 2.0 { "high" } else { "some" };
    Some((
        unit((offer / 1.0e6).powf(0.6) * (1.0 - money)),
        Point::new(Kind::FamilyNeed)
            .due(money_day(c))
            .text("offer", dollars(offer))
            .state("need", need)
            .known_if(contacted(c)),
    ))
}

/// Whether your staff has been in touch with him or his family: a call, a
/// home visit, his visit to you, your camp.
pub(super) fn contacted(c: &Ctx) -> bool {
    use crate::world::scouting::Kind as Look;
    c.w.scouting.dept.looks.get(&c.p.player.id).is_some_and(|v| {
        v.iter()
            .any(|l| matches!(l.kind, Look::Call | Look::HomeVisit | Look::Official | Look::Unofficial | Look::Camp))
    })
}

/// Brand deals for a kid with a name: his following is public, and so is
/// how much your program is seen.
fn brand(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    if c.p.followers < BRAND_FOLLOWERS || rec::offer_money(c.w, j, c.i) <= 0.0 {
        return None;
    }
    let big = c.p.followers >= 4 * BRAND_FOLLOWERS;
    Some((c.grade(j, GradeCat::BrandExposure), Point::new(Kind::Brand).state("name", if big { "big" } else { "some" })))
}

// ----- The school.

fn degree(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    use crate::world::data::AcademicTier as A;
    let prog = &c.w.programs[j];
    let tier = match prog.academic_tier {
        A::Aau => "top",
        A::R1 => "research",
        A::R2 => "solid",
        A::Other => "plain",
    };
    Some((
        c.grade(j, GradeCat::Academics),
        Point::new(Kind::Degree).text("school", c.w.program_name(j)).state("academics", tier),
    ))
}

/// His own grades and your academic support: a point only for a recruit
/// whose grades put him near the line. Which side he is on picks the
/// words; the grade is never a value. You know it once your staff has
/// seen his transcript or talked to his school.
fn support(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    use crate::world::scouting::Kind as Look;
    // hidden: read as a gate only.
    let gpa = c.p.bg.gpa;
    let line = super::player::GRADE_LINE;
    if gpa >= line + super::player::GRADE_NEAR {
        return None;
    }
    let seen =
        c.w.scouting
            .dept
            .looks
            .get(&c.p.player.id)
            .is_some_and(|v| v.iter().any(|l| matches!(l.kind, Look::Transcript | Look::Call)));
    Some((
        unit(0.3 + 0.7 * c.w.programs[j].facilities),
        Point::new(Kind::Support).state("line", if gpa < line { "under" } else { "near" }).known_if(seen),
    ))
}

fn after_football(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let s = 0.7 * c.grade(j, GradeCat::Academics) + 0.3 * c.grade(j, GradeCat::CampusLife);
    Some((
        s,
        Point::new(Kind::AfterFootball).text("school", c.w.program_name(j)).text("town", c.w.programs[j].city.clone()),
    ))
}

// ----- Close to home.

pub(super) fn miles(c: &Ctx, j: usize) -> f64 {
    rec::home_miles(&c.w.high_schools, c.p, &c.w.programs[j])
}

/// The drive: the miles from his home, and whether family can make his
/// games in a day.
fn drive(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let m = miles(c, j);
    let round = ((m / 10.0).round() as i64 * 10).max(10);
    let hours = ((m / 60.0).round() as i64).max(1);
    Some((
        unit(rec::closeness(m)),
        Point::new(Kind::Drive)
            .count("miles", round)
            .count("hours", hours)
            .text("home", c.w.high_schools[c.p.hs].town.clone())
            .state("drive", if m <= DAY_DRIVE { "day" } else { "far" }),
    ))
}

/// The team he grew up on: a point only for that school.
fn childhood_team(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    (c.p.childhood_team == Some(j)).then(|| (1.0, Point::new(Kind::ChildhoodTeam).text("school", c.w.program_name(j))))
}

/// Staying in his state: a point only for a school in it.
fn home_state(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let st = c.w.programs[j].state;
    if st != c.p.home_state {
        return None;
    }
    let name = c.w.states.get(st).map_or("", |s| facts::state_name(&s.code));
    Some((1.0, Point::new(Kind::HomeState).text("state", name)))
}

/// Players from his high school on the team, or in the class.
fn hs_mates(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let w = c.w;
    let mut mates: Vec<usize> = w.programs[j]
        .roster
        .iter()
        .copied()
        .filter(|&k| k != c.i && w.people[k].hs == c.p.hs && w.people[k].stage == Stage::College)
        .collect();
    // His own school's classmates who committed or signed there.
    for &k in &w.high_schools[c.p.hs].roster {
        let q = &w.people[k];
        if k != c.i
            && (q.cal.commit.is_some_and(|x| x.0 == j) || q.cal.signed.is_some_and(|s| s.program == j))
            && !mates.contains(&k)
        {
            mates.push(k);
        }
    }
    if mates.is_empty() {
        return None;
    }
    let names: Vec<String> = mates.iter().take(3).map(|&k| w.people[k].name.clone()).collect();
    let mut pt = Point::new(Kind::HsMates)
        .count("mates", mates.len() as i64)
        .text("names", words::list(&names))
        .text("hs", w.school_name(c.p.hs));
    for &k in mates.iter().take(3) {
        pt = pt.refer(Ref::Player(w.people[k].player.id));
    }
    Some((unit(mates.len() as f64 / MATES_FULL as f64), pt))
}

/// A brother here. Whether he wants to follow him or make his own name is
/// his alone to know.
fn brother(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let f = &c.p.family;
    if f.older_brother_at != Some(j) {
        return None;
    }
    let pt = Point::new(Kind::Brother)
        .text("school", c.w.program_name(j))
        .hide("follows", if f.follows_brother { "yes" } else { "no" })
        .weigh(if f.follows_brother { BROTHER_PULL.0 } else { BROTHER_PULL.1 });
    Some((1.0, pt))
}

// ----- The people.

/// Whether a head coach's last two seasons ran well under what was
/// expected of him.
pub(super) fn hot_seat(w: &World, coach: usize) -> bool {
    let r = &w.coaches[coach].recent;
    r.len() >= 2 && r[r.len() - 1] + r[r.len() - 2] < HOT_SEAT
}

/// Your years here and how safe your job is.
fn tenure(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let w = c.w;
    let hc = w.programs[j].head_coach?;
    let job = w.coaches[hc].job?;
    let years = (w.dates.season - job.first_season() + 1).max(0);
    let hot = hot_seat(w, hc);
    let seat = if hot {
        "hot"
    } else if years < NEW_SEASONS {
        "new"
    } else if years >= LONG_SEASONS {
        "long"
    } else {
        "settled"
    };
    let strength = unit(0.25 + 0.06 * years.min(8) as f64 - if hot { 0.3 } else { 0.0 });
    Some((
        strength,
        Point::new(Kind::Tenure)
            .count("years", years as i64)
            .text("seasons", words::counted(years.max(0) as u32, "season"))
            .state("seat", seat)
            .refer(Ref::Coach(hc)),
    ))
}

/// The position coach he'd have, and that man's record: the draft picks
/// he coached at this spot.
fn position_coach(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let w = c.w;
    let g = Room::of(c.pos);
    let pc = w.programs[j].position_coaches[g as usize]?;
    let years = w.coaches[pc].job.map_or(0, |x| (w.dates.season - x.first_season() + 1).max(0));
    let picks: Vec<_> =
        c.ix.coached(pc).filter(|a| a.position_coach == Some(pc) && Room::of(a.position) == g).take(12).collect();
    let mut pt = Point::new(Kind::PositionCoach)
        .text("pc", format!("Coach {}", super::player::last_name(&w.coaches[pc].name)))
        .text("title", g.coach_title())
        .count("years", years as i64)
        .count("picks", picks.len() as i64)
        .refer(Ref::Coach(pc));
    if let Some(a) = picks.first() {
        pt = pt.text("best", a.name.clone()).text("best_round", words::round_word(a.round as u32));
    }
    Some((unit(0.3 + 0.1 * picks.len() as f64 + 0.03 * years.min(8) as f64), pt))
}

/// Your tie with his high school coach: the pipeline from his school.
fn hs_coach_tie(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let w = c.w;
    let &(v, _) = w.programs[j].hs_pipeline.get(&c.p.hs)?;
    if v < PIPELINE_TIE {
        return None;
    }
    let mut pt = Point::new(Kind::HsCoachTie).text("hs", w.school_name(c.p.hs));
    if let Some(hc) = w.high_schools[c.p.hs].coach {
        pt = pt
            .text("hs_coach", format!("Coach {}", super::player::last_name(&w.coaches[hc].name)))
            .refer(Ref::Coach(hc));
    }
    Some((unit(v), pt))
}

/// Your name for keeping your word: the promises of yours the world has
/// judged. The world keeps them for you alone, so only your program has
/// the point.
fn your_word(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    if !c.mine(j) {
        return None;
    }
    let p = &c.w.folk.promises;
    let kept = p.iter().filter(|x| x.kept == Some(true)).count();
    let broken = p.iter().filter(|x| x.kept == Some(false)).count();
    if kept + broken == 0 {
        return None;
    }
    let record = if broken == 0 {
        "clean"
    } else if kept > 2 * broken {
        "mixed"
    } else {
        "poor"
    };
    Some((
        unit(kept as f64 / (kept + 2 * broken) as f64),
        Point::new(Kind::YourWord).count("kept", kept as i64).count("broken", broken as i64).state("record", record),
    ))
}

/// Why a transfer left his old school: the reason on his portal entry.
fn why_left(c: &Ctx, _j: usize) -> Option<(f32, Point)> {
    let e = c.p.cal.portal?;
    let why = match e.why {
        "coaching change" => "coaching_change",
        "steered" => "steered",
        "move up" => "move_up",
        "dismissed" => "dismissed",
        "money" => "money",
        "spring depth chart" => "depth_chart",
        "graduate transfer" => "graduate",
        "program dropped" => "program_dropped",
        "cut" => "cut",
        _ => "playing_time",
    };
    let from = c.p.path.last().map(|&x| c.w.program_name(x).to_string()).unwrap_or_default();
    Some((1.0, Point::new(Kind::WhyLeft).on(e.entered).text("old_school", from).state("why", why)))
}

// ----- The league.

/// Draft picks at his position in ten years, by name. A pick whose
/// coaches have all left counts less: his question is what you did.
fn picks_position(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let w = c.w;
    let prog = &w.programs[j];
    let now = [prog.head_coach, prog.position_coaches[Room::of(c.pos) as usize]];
    let picks: Vec<_> = c.ix.recent(j, c.pos, w.year).collect();
    if picks.is_empty() {
        return None;
    }
    let weight: f32 =
        picks.iter().map(|a| if now.iter().flatten().any(|&x| a.coached_by(x)) { 1.0 } else { ALUM_OTHERS }).sum();
    let yours = picks.iter().filter(|a| now.iter().flatten().any(|&x| a.coached_by(x))).count();
    let best = picks.iter().min_by_key(|a| a.pick)?;
    let names: Vec<String> = picks.iter().take(3).map(|a| a.name.clone()).collect();
    let mut pt = Point::new(Kind::PicksAtPosition)
        .count("picks", picks.len() as i64)
        .text("picks_words", words::counted(picks.len() as u32, "pick"))
        .text("spot", words::spot(c.pos))
        .text("names", words::list(&names))
        .text("best", best.name.clone())
        .text("best_round", words::round_word(best.round as u32))
        .count("best_year", best.year as i64)
        .text("best_club", best.club.clone())
        .state(
            "coached",
            if yours == 0 {
                "others"
            } else if yours == picks.len() {
                "yours"
            } else {
                "some"
            },
        );
    for a in picks.iter().take(3) {
        if let Some(id) = a.id {
            pt = pt.refer(Ref::Player(id));
        }
    }
    Some(((weight / PICKS_FULL).min(1.0), pt))
}

/// Draft picks overall: the last three drafts.
fn picks_all(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let w = c.w;
    let recent: Vec<u32> = w.programs[j].draft_picks.iter().filter(|x| x.0 > w.year - 3).map(|x| x.1).collect();
    if recent.is_empty() {
        return None;
    }
    let first = recent.iter().filter(|&&r| r == 1).count();
    Some((
        c.grade(j, GradeCat::ProPipeline),
        Point::new(Kind::PicksOverall)
            .count("picks", recent.len() as i64)
            .text("picks_words", words::counted(recent.len() as u32, "pick"))
            .count("first_round", first as i64)
            .state("first_round", if first > 0 { "yes" } else { "no" }),
    ))
}

/// The players a head coach had himself, anywhere: drafted men he coached
/// as their head coach or their position coach.
fn your_guys(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let hc = c.w.programs[j].head_coach?;
    let guys: Vec<_> = c.ix.coached(hc).collect();
    if guys.is_empty() {
        return None;
    }
    let at_spot: Vec<_> = guys.iter().filter(|a| Room::of(a.position) == Room::of(c.pos)).collect();
    let show = if at_spot.is_empty() { guys.iter().collect::<Vec<_>>() } else { at_spot.clone() };
    let names: Vec<String> = show.iter().take(3).map(|a| a.name.clone()).collect();
    Some((
        (guys.len() as f32 / GUYS_FULL).min(1.0),
        Point::new(Kind::YourGuys)
            .count("guys", guys.len() as i64)
            .count("at_spot", at_spot.len() as i64)
            .text("names", words::list(&names))
            .state("spot", if at_spot.is_empty() { "no" } else { "yes" })
            .refer(Ref::Coach(hc)),
    ))
}

/// The position group's name ("DBU"): a point only for the school that
/// holds it this year.
fn position_u(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let g = Room::of(c.pos);
    let k = Room::ALL.iter().position(|&x| x == g)? as u8;
    c.w.legacy
        .college
        .u_names
        .iter()
        .any(|&(prog, group)| prog as usize == j && group == k)
        .then(|| (1.0, Point::new(Kind::PositionU).text("u", crate::world::legacy::college::u_name(g))))
}

/// Your pro day: how many clubs came, and how many sent a head coach, a
/// coordinator or a general manager (the last two springs).
fn pro_day(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let y = c.w.year as i16;
    let d =
        c.w.scouting.pro_days.iter().filter(|x| x.program as usize == j && x.year >= y - 2).max_by_key(|x| x.year)?;
    if d.clubs == 0 {
        return None;
    }
    Some((
        unit((d.clubs as f64 + 2.0 * d.heads as f64) / 40.0),
        Point::new(Kind::ProDay)
            .count("clubs", d.clubs as i64)
            .count("heads", d.heads as i64)
            .count("year", d.year as i64),
    ))
}

// ----- Winning and the name.

/// Titles and the last season.
fn titles(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let prog = &c.w.programs[j];
    let last = prog.seasons.last();
    let mut pt = Point::new(Kind::Titles)
        .count("titles", prog.titles as i64)
        .count("conference_titles", prog.conference_titles as i64)
        .state("titles", if prog.titles > 0 { "yes" } else { "no" });
    if let Some(y) = prog.seasons.iter().rev().find(|s| s.national_title).map(|s| s.year) {
        pt = pt.count("last_title", y as i64);
    }
    if let Some(s) = last {
        pt = pt.text("last_record", format!("{}-{}", s.wins, s.losses)).count("last_year", s.year as i64);
    }
    Some((c.grade(j, GradeCat::Tradition), pt))
}

/// This year's chances: how the program stands for a title, in words.
/// The world keeps no odds for a talk to quote.
fn title_odds(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let g = c.grade(j, GradeCat::TitleChances);
    let level = if g >= CONTENDER {
        "contender"
    } else if g >= IN_THE_HUNT {
        "in_the_hunt"
    } else {
        "long_shot"
    };
    Some((g, Point::new(Kind::TitleOdds).state("level", level)))
}

fn stadium(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let seats = c.w.programs[j].stadium;
    (seats > 0).then(|| {
        (
            c.grade(j, GradeCat::StadiumAtmosphere),
            Point::new(Kind::Stadium)
                .count("seats", (seats as i64 / 100) * 100)
                .text("town", c.w.programs[j].city.clone()),
        )
    })
}

fn exposure(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let power = c.w.programs[j].power_conference(c.w.year);
    Some((
        c.grade(j, GradeCat::BrandExposure),
        Point::new(Kind::Exposure).state("power", if power { "yes" } else { "no" }),
    ))
}

fn conference(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let name = c.w.programs[j].conference.clone();
    (!name.is_empty())
        .then(|| (c.grade(j, GradeCat::ConferenceStrength), Point::new(Kind::Conference).text("conference", name)))
}

/// The rival: the game by name and the series.
fn rival(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    let (other, game, ours, theirs) = facts::rival(c.w, Team::College(j))?;
    let share = ours as f64 / (ours + theirs).max(1) as f64;
    Some((
        unit(0.3 + 0.7 * share),
        Point::new(Kind::Rival)
            .text("rival", other.name(c.w))
            .text("game", game)
            .count("won", ours as i64)
            .count("lost", theirs as i64)
            .state(
                "series",
                if ours > theirs {
                    "ahead"
                } else if ours < theirs {
                    "behind"
                } else {
                    "even"
                },
            )
            .refer(Ref::Team(other)),
    ))
}

/// Buildings: one under way, or one that opened in the last two seasons.
fn buildings(c: &Ctx, j: usize) -> Option<(f32, Point)> {
    use crate::world::finance::Owner;
    let w = c.w;
    let x = w
        .finance
        .projects
        .iter()
        .filter(|x| x.owner == Owner::College(j) && (!x.done || x.opens >= w.year - 1))
        .max_by_key(|x| (!x.done, x.opens))?;
    Some((
        c.grade(j, GradeCat::Facilities),
        Point::new(Kind::Buildings)
            .text("building", x.name.clone())
            .count("opens", x.opens as i64)
            .state("stage", if x.done { "open" } else { "building" }),
    ))
}
