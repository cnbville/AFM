//! One development checkpoint covers a full football year: the season plus the
//! offseason that follows. It is meant to run at the yearly potential review
//! (W1 of the AFM calendar), after which the player is a year older.

use crate::attributes::{
    ATTRIBUTE_COUNT, ATTRIBUTE_PROFILES, Attr, AttributeGroup, Attributes, MIN_RATING, RATING_BOUND,
};
use crate::player::{DevelopmentEvent as Ev, Events, Level, OptF64, Player, PlayingTime, SeasonRecord};
use crate::positions::{Position, overall_for, weight};
use crate::rng::{Rng, clamp, round_half_up};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Trouble {
    Suspended,
    Dismissed,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct SeasonContext {
    pub level: Level,
    /// Quality of the player's coaching staff, 1-20.
    pub coaching: f64,
    pub playing_time: PlayingTime,
    /// Off-field or academic trouble that costs him this season's games.
    pub trouble: Option<Trouble>,
    /// First-year dip after a step up, 0-1 (see `first_year_adjustment` in career).
    /// He plays below his rating while he adjusts, and grows a little slower.
    pub adjustment: f64,
    /// A holdout who missed camp (update 3, F1): extra dip on top of
    /// `adjustment`, 0-1. He plays below his rating while he catches up.
    pub missed_camp: f64,
    /// How his surroundings help or hurt him, -1 to 1: the supporting cast, the
    /// scheme and how well run the organisation is. A bad situation drags his
    /// play and his confidence; a good one lifts both.
    pub situation: f64,
    /// He signed a big NIL deal or a big contract before this season
    /// (fix list 4.5): a lost-drive year gets likelier.
    pub payday: bool,
    /// A pro's real snaps last season as a share of a starter's season,
    /// weighted by how hard they were (section 5 topic 1.1). When given, it
    /// replaces the playing-time class in growth, potential and experience;
    /// the class still decides the rest.
    pub snaps: Option<f64>,
    /// His unit's coaching style and the scheme it runs, which pull his
    /// tendencies (`tendencies::develop_year`). None: no pull.
    pub style: Option<crate::tendencies::Style>,
    pub scheme: Option<(crate::game::roster::OffScheme, crate::game::roster::Front)>,
    /// How far what his tools support sits from the middle on average at
    /// his position and level (`tendencies::Centres`); zero: read raw.
    pub tend_centre: [f64; crate::tendencies::TEND_COUNT],
    /// Section 6 topics 3 and 4: his mood and his room's, -1 to 1. It moves
    /// practice effort, so growth (proposed, not confirmed: up to 10%).
    pub morale: f64,
    /// Section 6 topics 1 and 10: the strength staff and his own
    /// professionalism, -1 to 1: the physical side of growth (up to 8%).
    pub physical: f64,
    /// Section 6 topic 10: the medical staff, -1 to 1: how fast he comes
    /// back from a minor or moderate injury (up to 15%).
    pub medical: f64,
    /// His dietitians' work over the level's typical department, in rating
    /// points (support departments, question 4; 0 none or typical): the
    /// weight room's pace toward his weight, never a ceiling.
    #[serde(default)]
    pub diet: f64,
    /// Section 6b topic 5: his plan's main focus, set by you. The focus
    /// grows faster the more minutes he spends on it, his position's other
    /// key skills pay a quarter of the gain, and a veteran's decline in it
    /// slows (individual practice, topic 5b); how much sticks follows his
    /// coachability.
    pub focus: Option<Attr>,
    /// His second point of emphasis's rating, if he has two (individual
    /// practice topic 5.1): the two share the minutes.
    #[serde(default)]
    pub focus2: Option<Attr>,
    /// He plays for your seat: his flaws and tools are worked week by week
    /// (`world::craft_work`), so the year's step works no staff points.
    #[serde(default)]
    pub seat_plan: bool,
    /// The minutes a week he spends on the focus (individual practice,
    /// topic 5b): the usual 30 when nothing says otherwise.
    #[serde(default = "usual_focus_minutes")]
    pub focus_minutes: f64,
    /// Section 6b topic 12 (choice 1, confirmed): the year's growth is
    /// decided here and lands week by week as the training happens
    /// (`training::land`), instead of all at once. Attributes that have
    /// stopped growing or decline this year still settle at once.
    pub defer: bool,
    /// The coaching mix for his physical attributes (minor bug 302), on the
    /// same 1-20 scale as `coaching`: the strength staff takes the position
    /// coach's share there. None where no strength staff works with him
    /// (high school, a year away): `coaching` stands.
    pub physical_coaching: Option<f64>,
    /// His strength head's rating against the level's typical one (rating
    /// points): taken out of `physical` for growth, since the strength staff
    /// now counts through `physical_coaching`.
    pub strength_dev: f64,
    /// Growth from results (`results`): Some when his season plays on the
    /// engine with results on. His snaps then land in the week he plays
    /// them (the year opens at a backup's rate for the skills training
    /// lands), and with `Dice::Replaced` the hidden dice on the skills
    /// results test give way to results, their average kept.
    pub results: Option<crate::results::Mode>,
    /// The injuries he really had since his last step (realism audit item
    /// 1): game and practice injuries at every level, logged as they land
    /// (`training::seat::SeatTraining::log_hurt`). Some when his last
    /// season played on the engine: the worst of them decides the year's
    /// injury cost, by what it was and how long it kept him out. None (the
    /// class sim, a season on the stand-in): the yearly roll decides.
    #[serde(default)]
    pub hurt: Option<YearHurt>,
    /// Coachability points his bond with the coach who teaches him adds to
    /// how much of the coaching sticks (coach and player bonds, question
    /// 4: up to 3, `world::folk::bonds::coachability_lift`).
    #[serde(default)]
    pub bond: f64,
    /// The share of his position coach's one-on-one time he keeps when the
    /// room runs projects he isn't on (design item 8,
    /// `progression::room_time`): 1 with none.
    #[serde(default = "one")]
    pub room_time: f64,
}

fn one() -> f64 {
    1.0
}

/// The injuries a player really had since his last development step
/// (realism audit item 1): the worst one, what it was, how long it kept him
/// out and the day it happened, and the days out over all of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct YearHurt {
    pub kind: Option<crate::game::sim::InjuryKind>,
    pub days: u16,
    pub on: crate::calendar::Date,
    pub total: u16,
    /// The worst one is already named on the season row it happened in
    /// (`world::season::mark_real_injury`).
    pub named: bool,
    /// A major injury at his last step that heals: the rehab loss comes
    /// back at this one (the year back).
    pub rehab: bool,
}

impl YearHurt {
    /// One injury landing: the worst stands (by class, then days out).
    pub fn add(&mut self, kind: crate::game::sim::InjuryKind, days: u32, on: crate::calendar::Date) {
        let days = days.min(u16::MAX as u32) as u16;
        let c = hurt_class(kind, days as u32);
        let now = self.kind.map_or(0, |k| hurt_class(k, self.days as u32));
        if self.kind.is_none() || c > now || (c == now && days > self.days) {
            self.kind = Some(kind);
            self.days = days;
            self.on = on;
            self.named = false;
        }
        self.total = self.total.saturating_add(days);
    }

    /// The worst injury's class: 0 a knock, 1 minor, 2 moderate, 3 major.
    pub fn class(&self) -> u8 {
        self.kind.map_or(0, |k| hurt_class(k, self.days as u32))
    }

    /// The class as the row's event.
    pub fn event(&self) -> Option<Ev> {
        match self.class() {
            1 => Some(Ev::MinorInjury),
            2 => Some(Ev::ModerateInjury),
            3 => Some(Ev::MajorInjury),
            _ => None,
        }
    }
}

/// Days out from which a real injury is minor, moderate and major for his
/// growth (realism audit item 1; proposed, not confirmed): under a week is
/// a knock that costs nothing, a week to three minor, three to eight
/// moderate, eight weeks or more major. A torn ACL or Achilles is always
/// major.
pub const HURT_MINOR_DAYS: u32 = 7;
pub const HURT_MODERATE_DAYS: u32 = 21;
pub const HURT_MAJOR_DAYS: u32 = 56;

/// A real injury's class for growth: 0 a knock, 1 minor, 2 moderate, 3 major.
pub fn hurt_class(kind: crate::game::sim::InjuryKind, days: u32) -> u8 {
    use crate::game::sim::InjuryKind as K;
    if matches!(kind, K::Acl | K::Achilles) || days >= HURT_MAJOR_DAYS {
        3
    } else if days >= HURT_MODERATE_DAYS {
        2
    } else if days >= HURT_MINOR_DAYS {
        1
    } else {
        0
    }
}

/// How a major real injury heals, by what it was (realism audit item 1;
/// proposed, not confirmed): the share that heals fully and the share that
/// heals with a small lasting loss; the rest never get their burst back.
/// None: it doesn't touch his legs (a shoulder, a neck, a concussion), so
/// it costs the year's growth and no burst. A torn ACL keeps the roll's
/// split (62-74% of NFL players back after a repair, fewer starts after);
/// a torn Achilles heals worse (engine gaps item 2: about a third of NFL
/// players never come back from one, and the rest lose some of their
/// explosion; proposed, not confirmed: 0.45 and 0.75); another leg injury
/// long enough to be major (a broken bone, a Lisfranc foot, a bad high
/// ankle) heals fully far more often.
pub fn major_heal(kind: crate::game::sim::InjuryKind) -> Option<(f64, f64)> {
    use crate::game::sim::InjuryKind as K;
    match kind {
        K::Acl => Some((0.55, 0.85)),
        K::Achilles => Some((0.45, 0.75)),
        K::Fracture => Some((0.8, 0.96)),
        K::Knee
        | K::AnkleSprain
        | K::HighAnkleSprain
        | K::Foot
        | K::Hamstring
        | K::Calf
        | K::Quad
        | K::Groin
        | K::Hip => Some((0.75, 0.95)),
        _ => None,
    }
}

impl SeasonContext {
    pub fn new(level: Level, coaching: f64, playing_time: PlayingTime) -> Self {
        SeasonContext {
            level,
            coaching,
            playing_time,
            trouble: None,
            adjustment: 0.0,
            missed_camp: 0.0,
            situation: 0.0,
            payday: false,
            snaps: None,
            style: None,
            scheme: None,
            tend_centre: [0.0; crate::tendencies::TEND_COUNT],
            morale: 0.0,
            physical: 0.0,
            medical: 0.0,
            diet: 0.0,
            focus: None,
            focus2: None,
            seat_plan: false,
            focus_minutes: USUAL_FOCUS_MINUTES,
            defer: false,
            physical_coaching: None,
            strength_dev: 0.0,
            results: None,
            hurt: None,
            bond: 0.0,
            room_time: 1.0,
        }
    }
}

/// The minutes a week a focus usually gets (individual practice, topic
/// 5b): the position coach's emphasis inside the individual period.
pub const USUAL_FOCUS_MINUTES: f64 = 30.0;

fn usual_focus_minutes() -> f64 {
    USUAL_FOCUS_MINUTES
}

/// A plan's focus as growth reads it (individual practice, topic 5b): the
/// rating it grows and the minutes a week he spends on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Focus {
    pub attr: Attr,
    pub minutes: f64,
    /// A second focus point's rating (individual practice topic 5.1: one
    /// or two points at a time). The two share his minutes.
    pub second: Option<Attr>,
}

impl Focus {
    pub fn new(attr: Attr, minutes: f64) -> Focus {
        Focus { attr, minutes, second: None }
    }

    /// Two points of emphasis, each on its rating; the same rating twice is
    /// one focus with all the minutes.
    pub fn two(attr: Attr, second: Option<Attr>, minutes: f64) -> Focus {
        Focus { attr, minutes, second: second.filter(|&b| b != attr) }
    }

    /// Whether a rating is one of his focus ratings.
    pub fn has(self, a: Attr) -> bool {
        self.attr == a || self.second == Some(a)
    }

    /// The ratings it grows (one or two).
    pub fn attrs(self) -> impl Iterator<Item = Attr> {
        std::iter::once(self.attr).chain(self.second)
    }

    /// The minutes on one of its ratings: two points split his time.
    pub fn minutes_on(self, a: Attr) -> f64 {
        if !self.has(a) {
            0.0
        } else if self.second.is_some() {
            self.minutes / 2.0
        } else {
            self.minutes
        }
    }

    /// The lift on one of its ratings (`lift` at that rating's minutes).
    pub fn lift_on(self, a: Attr, take: f64) -> f64 {
        Focus::new(a, self.minutes_on(a)).lift(take)
    }

    /// The lift on its growth rate, past 1 (proposed, not confirmed:
    /// take x minutes / 60, the minutes capped at 60, so a coachable
    /// player grows it 1.5 times as fast at the usual 30 minutes and twice
    /// as fast at 60). `take` is how much of the work sticks (0.6-1.0).
    pub fn lift(self, take: f64) -> f64 {
        take * clamp(self.minutes, 0.0, 60.0) / 60.0
    }
}

/// How much of a focus's work sticks with him, 0.6-1.0, from his
/// coachability (section 6b topic 5).
pub fn focus_take(coachability: f64) -> f64 {
    clamp(0.6 + 0.02 * coachability, 0.6, 1.0)
}

/// The share of a focus's extra gain his other key skills give up
/// (individual practice, topic 5b; proposed, not confirmed): the minutes
/// come out of their work.
pub const FOCUS_COST: f64 = 0.25;

/// The skills that pay for a focus (individual practice, topic 5b): the
/// skills with the most weight at his position, up to four, never the body
/// (individual time is technique and reading, not the weight room) and
/// never the focus itself, with their weights.
pub fn focus_cost_skills(pos: Position, focus: Attr) -> Vec<(Attr, f64)> {
    let mut v: Vec<(Attr, f64)> = Attr::ALL
        .iter()
        .copied()
        .filter(|&a| a != focus && a.profile().group != AttributeGroup::Physical && weight(pos, a) > 0.0)
        .map(|a| (a, weight(pos, a)))
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.index().cmp(&b.0.index())));
    v.truncate(4);
    v
}

/// Takes a focus's cost out of what his other key skills just gained
/// (individual practice, topic 5b): a quarter of the `extra` the focus gave
/// (its rating's gain with the focus less without, the same noise), spread
/// over the key skills by their weight, never more than each gained since
/// `before` in the same landing, so a focus steers growth and never takes
/// a skill down.
pub(crate) fn focus_cost(pos: Position, before: &Attributes, after: &mut Attributes, focus: Attr, extra: f64) -> f64 {
    if extra <= 0.0 {
        return 0.0;
    }
    let skills = focus_cost_skills(pos, focus);
    let total_w: f64 = skills.iter().map(|s| s.1).sum();
    if total_w <= 0.0 {
        return 0.0;
    }
    let mut paid = 0.0;
    for (a, w) in skills {
        let own = (after.get(a) - before.get(a)).max(0.0);
        let take_off = (FOCUS_COST * extra * w / total_w).min(own);
        if take_off > 0.0 {
            after.add(a, -take_off);
            paid += take_off;
        }
    }
    paid
}

/// The most of the gap to his ceiling a focused skill closes in a year
/// (topic 5b; proposed, not confirmed): the usual cap is half.
const FOCUS_RATE_CAP: f64 = 0.85;

/// What sets how fast he grows this year, besides each attribute's own
/// profile and his age: the yearly step decides it, and the weekly landing
/// (`training::land`) reads it back from his `Train`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GrowthCtx {
    pub coaching_boost: f64,
    /// Coaching as it sticks for the physical attributes (minor bug 302).
    pub physical_boost: f64,
    pub snaps_mult: f64,
    pub work_mult: f64,
    pub form: f64,
    pub growth_multiplier: f64,
    pub morale_mult: f64,
    pub physical_mult: f64,
    pub hs: bool,
    pub college: bool,
    pub redshirt: bool,
    pub focus: Option<Focus>,
    pub focus_take: f64,
    /// The skills results test when results replace the dice (item 14),
    /// and the year's form for them: the dice's average.
    pub tested: u128,
    pub form_tested: f64,
    /// His skill project (the progression update, design items 9 and 12):
    /// it grows at the 60-minute focus's pace, and the skills in
    /// `cost_mask` (his position's other technique skills) grow a quarter
    /// slower.
    pub project: Option<Attr>,
    pub cost_mask: u128,
}

impl GrowthCtx {
    /// The same year without the focus: what it adds is measured against it.
    pub fn plain(&self) -> GrowthCtx {
        GrowthCtx { focus: None, ..*self }
    }

    /// The share of the gap to his ceiling an attribute closes in a full
    /// year (the yearly step's rate, unchanged).
    pub fn rate(&self, key: Attr, age_factor: f64) -> f64 {
        let profile = &ATTRIBUTE_PROFILES[key.index()];
        let boost = if profile.group == AttributeGroup::Physical { self.physical_boost } else { self.coaching_boost };
        let coaching = boost.powf(profile.coachability);
        let snaps = self.snaps_mult.powf(profile.experience);
        let redshirt_lift = if self.redshirt && profile.group == AttributeGroup::Physical { 1.35 } else { 1.0 };
        // College staffs teach the technique and the reads high school never did, so the raw catch up there.
        let hs_lag = if self.hs {
            if matches!(key, Attr::Strength | Attr::LowerBody) {
                HS_STRENGTH_GROWTH
            } else {
                hs_group_growth(profile.group)
            }
        } else if self.college && profile.group != AttributeGroup::Physical {
            COLLEGE_TEACHING
        } else {
            1.0
        };
        let form = if crate::results::is_tested(self.tested, key) { self.form_tested } else { self.form };
        // The focus grows faster by its minutes (individual practice, topic
        // 5b); what it costs comes off the key skills' gains as they land
        // (`focus_cost`), not off their rates.
        let focus_mult = match self.focus {
            Some(f) if f.has(key) => 1.0 + f.lift_on(key, self.focus_take),
            _ => 1.0,
        };
        // A project takes the position coach's one-on-one minutes at the
        // 60-minute focus (design item 8); a focus on the same skill adds
        // nothing past it.
        let focus_mult = match self.project {
            Some(a) if a == key => focus_mult.max(1.0 + Focus::new(a, 60.0).lift(self.focus_take)),
            _ => focus_mult,
        };
        let project_cost =
            if self.cost_mask >> key.index() & 1 == 1 { 1.0 - crate::progression::PROJECT_COST } else { 1.0 };
        // The focus multiplies the year as it would have been, past the
        // usual half of the gap a year (topic 5b: at 60 minutes about double
        // his normal climb); the ceiling still caps it.
        let rate = clamp(
            profile.growth_rate
                * age_factor
                * coaching
                * snaps
                * self.work_mult
                * form
                * self.growth_multiplier
                * redshirt_lift
                * hs_lag
                * self.morale_mult
                * if profile.group == AttributeGroup::Physical { self.physical_mult } else { 1.0 }
                * project_cost,
            0.0,
            0.5,
        );
        if focus_mult > 1.0 { (rate * focus_mult).min(FOCUS_RATE_CAP) } else { rate }
    }
}

/// The weight the weight room pulls a grown frame toward (update 3, A4,
/// A14): 60% of the way from his frame to his position's weight for his
/// talent (`athlete::placed_weight`, the same share a move to a new
/// position takes), never past the position's combine weight. Until the
/// physical study (item 15, 2026-11-16) it took every frame under the
/// position's weight all the way there, so the light half of a position
/// vanished by the draft: drafted receivers ran 7.6 lb heavy and corners
/// 3.5 (proposed, not confirmed).
pub fn weight_room_room(position: crate::positions::Position, talent: f64, frame: f64) -> f64 {
    let target = crate::athlete::position_weight_for(talent, position).min(crate::body::combine_size(position).2);
    target.min(crate::athlete::placed_weight(frame, target))
}

/// The share of the way to `weight_room_room` a year closes: a third in
/// college and the pros, an eighth in high school, faster with his work
/// ethic and a strength staff (proposed, not confirmed).
pub fn weight_room_pace(hs: bool, work_ethic: f64, physical: f64) -> f64 {
    (if hs { 0.12 } else { 0.33 })
        * clamp(0.7 + 0.03 * work_ethic, 0.7, 1.3)
        * (1.0 + 0.15 * clamp(physical, -1.0, 1.0))
}

/// Whether an attribute is past its decline onset this year, by the
/// ageing step's rule.
pub(crate) fn declines(position: crate::positions::Position, longevity: f64, key: Attr, age: f64) -> bool {
    age + 1.0 - decline_onset(position, longevity, key) > 0.0
}

/// The age an attribute starts to decline. A position moves the body's
/// decline at most 1.5 years either way, so a running back's speed goes
/// 1.5 years early, not 2.5 (update 3, F15; `docs/development-model.md`).
fn decline_onset(position: crate::positions::Position, longevity: f64, key: Attr) -> f64 {
    let profile = key.profile();
    let pos_offset = position.profile().aging_offset;
    let physical = profile.group == AttributeGroup::Physical;
    profile.decline_age + longevity + if physical { pos_offset.clamp(-1.5, 1.5) } else { pos_offset }
}

/// Positions a player really moves to from each spot (update 3, A6): what
/// they ask overlaps with his, so those skills keep growing, the way a
/// quarterback who becomes a receiver still has the hands he built
/// (proposed, not confirmed).
fn moves_to(pos: Position) -> &'static [Position] {
    use Position::*;
    match pos {
        QB => &[WR, RB, S, TE],
        RB => &[WR, CB, S, LB],
        WR => &[CB, S, TE, RB],
        TE => &[WR, OT, EDGE, LB],
        OT => &[IOL, TE, DT],
        IOL => &[OT, DT],
        EDGE => &[LB, DT, TE],
        DT => &[EDGE, IOL, OT],
        LB => &[EDGE, S, RB, TE],
        CB => &[S, WR],
        S => &[CB, LB, WR],
        K => &[P],
        P => &[K],
        LS => &[IOL, TE, LB],
    }
}

/// A skill clearly unrelated to his position (update 3, A6; Cain
/// 2026-10-24: "only very unrelated skills"): neither his position nor any
/// he really moves to uses it, like a tackle's man coverage. It stops
/// growing. The body is not a skill, and elusiveness is rated for his size
/// (A2), so both keep growing for everyone. High school kickers and
/// snappers play another position, so kicking and snapping are never
/// unrelated there.
pub(crate) fn unrelated(pos: Position, key: Attr, hs: bool) -> bool {
    if key.profile().group == AttributeGroup::Physical || key == Attr::Elusiveness {
        return false;
    }
    if hs && key.category() == crate::attributes::Category::KickingAndSnapping {
        return false;
    }
    weight(pos, key) <= 0.0 && moves_to(pos).iter().all(|&q| weight(q, key) <= 0.0)
}

/// The share of his top speed and burst his body has reached by the end
/// of this year (speed update S4): speed comes with growing up, not with a
/// birthday, so a senior who has his height has about 95% of it by senior
/// spring and a late bloomer less (Biology of Sport, 2026; proposed, not
/// confirmed: 66% at the start of growth, 96% when grown, straight between;
/// the year's growth adds the rest).
pub(crate) fn speed_with_body(age: f64, bloom: f64) -> f64 {
    let grown = clamp((age + 0.75 - 13.0 - 0.6 * bloom) / 6.0, 0.0, 1.0);
    0.66 + 0.30 * grown
}

/// Wear one season adds (update 3, F14, as the class sim always did): backs
/// take the most hits, quarterbacks who run take more than pocket passers,
/// kickers none. `played` is his share of a starter's season.
pub(crate) fn season_wear(p: &Player, played: f64) -> f64 {
    let per_season = match p.position {
        Position::RB => 1.0,
        Position::QB => {
            let legs = (p.attributes.get(Attr::Speed)
                + p.attributes.get(Attr::Acceleration)
                + p.attributes.get(Attr::Agility))
                / 3.0;
            if legs >= 78.0 { 0.6 } else { 0.15 }
        }
        Position::K | Position::P | Position::LS => 0.0,
        _ => 0.35,
    };
    per_season * played
}

/// One installment of a year's growth (section 6b topic 12): `share` of
/// the year for this attribute, `done` the share already landed. The
/// proportional part compounds on the gap plus the fixed steps already
/// given, so shares adding to one give the yearly step's growth exactly;
/// the fixed step and the cap split by share; the noise keeps its yearly
/// spread. Returns the rating points added.
pub(crate) fn grow_share(
    attrs: &mut crate::attributes::Attributes,
    ceilings: &mut crate::attributes::Attributes,
    key: Attr,
    rate: f64,
    share: f64,
    done: f64,
    noise: f64,
) -> f64 {
    let gap = ceilings.get(key) - attrs.get(key);
    if gap <= 0.0 || share <= 0.0 || rate <= 0.0 {
        return 0.0;
    }
    let prop = 1.0 - (share * (1.0 - rate).ln()).exp();
    let given = FINISHING_STEP * rate * done.clamp(0.0, 1.0);
    let add = gap.min((gap + given) * prop + FINISHING_STEP * rate * share) + noise;
    let before = attrs.get(key);
    let attr = clamp(before + add, MIN_RATING, RATING_BOUND);
    let attr = crate::athlete::top_up_attr(key, before, attr);
    attrs.set(key, attr);
    if attr > ceilings.get(key) {
        ceilings.set(key, attr);
    }
    attr - before
}

/// Potential is reported in bands of this many points (one half-star).
pub const POTENTIAL_BAND: f64 = 5.0;
/// Most the yearly drift may move a player's true potential in one checkpoint,
/// up or down. A major injury, a body that stops developing and a proven
/// breakout can move it further, and a ceiling also rises when a player
/// outgrows it.
pub const MAX_POTENTIAL_SHIFT: f64 = POTENTIAL_BAND;

/// Rating points a year a skill only his old position used fades (3.8).
const OLD_SKILL_FADE: f64 = 1.5;

fn level_injury_rate(level: Level) -> f64 {
    match level {
        Level::HS => 0.13,
        Level::SmallCollege => 0.16,
        Level::FCS => 0.17,
        Level::FBS => 0.19,
        Level::NFL => 0.24,
    }
}

/// Points per unit of growth rate added on top of proportional growth.
const FINISHING_STEP: f64 = 1.6;
// No yearly limit on a rating's gain (physical study item 16, Cain
// 2026-11-15): a growth spurt or a project that clicks grows as far as
// its year takes it; the ceiling still caps it.
/// Yearly chance, at 18-20, that an average bloomer's body stops developing (higher for early bloomers).
const STALL_CHANCE: f64 = 0.025;

/// The quality of the man across from you. A D2 star dominates but is rarely
/// pushed, so what he learns from games (reading defenses, technique) grows
/// more slowly. This barely touches physical growth: weight rooms and strength
/// staffs come through the coaching rating, which averages lower at D2.
fn level_growth(level: Level) -> f64 {
    if level == Level::SmallCollege { 0.72 } else { 1.0 }
}

fn playing_time_growth(pt: PlayingTime) -> f64 {
    match pt {
        PlayingTime::Starter => 1.15,
        PlayingTime::Rotation => 1.0,
        PlayingTime::Backup => 0.8,
        // A redshirt year is a full year of practice and weight room without game snaps.
        PlayingTime::Redshirt => 0.85,
        PlayingTime::PracticeSquad => 0.85,
        // Training alone while waiting for a call.
        PlayingTime::Unsigned => 0.55,
    }
}

/// The rate a year opens at when his snaps land in season (growth from
/// results): a backup's with no snaps yet; a redshirt, practice squad or
/// unsigned year keeps its own.
/// Whether last season at this level is read at this opening, as before
/// growth from results: its snaps didn't land in season (`Train` flags bit
/// 3, kept through `close_year` and cleared by `results::settle_reps` when
/// none landed). A first season at a level has nothing to read.
pub(crate) fn reads_late(p: &Player, level: Level) -> bool {
    p.train.year.flags & 8 == 0 && p.history.last().is_some_and(|h| h.level == level)
}

pub(crate) fn reps_base(pt: PlayingTime) -> f64 {
    match pt {
        PlayingTime::Redshirt | PlayingTime::PracticeSquad | PlayingTime::Unsigned => playing_time_growth(pt),
        _ => snaps_growth(0.0),
    }
}

/// The year's rate against its base after a share of a starter's season
/// of snaps, as the yearly step's snaps would give it.
pub(crate) fn reps_ratio(share: f64) -> f64 {
    snaps_growth(share) / snaps_growth(0.0)
}

/// High school practice, weight rooms and strength staffs are a fraction of
/// college ones, so recruits arrive rawer than their talent: technique and the
/// mental game lag most, and the strength linemen need comes in college. Speed
/// and the rest of the athletic gifts grow on nature's schedule.
fn hs_group_growth(g: AttributeGroup) -> f64 {
    match g {
        AttributeGroup::Physical => 1.0,
        AttributeGroup::Technical => 0.65,
        AttributeGroup::Mental => 0.7,
    }
}
const HS_STRENGTH_GROWTH: f64 = 0.6;
/// Extra technical and mental growth in college, where the teaching happens.
const COLLEGE_TEACHING: f64 = 1.3;

/// Games in a season by level, for games missed to injury.
pub fn season_games(level: Level) -> u32 {
    match level {
        Level::HS => 10,
        Level::SmallCollege | Level::FCS => 11,
        Level::FBS => 12,
        Level::NFL => 17,
    }
}

/// Growth from a pro's real snaps (section 5 topic 1.1): a backup's
/// practice reps at none, a starter's season at a full share, a little more
/// for a heavy load against strong teams (proposed, not confirmed: the
/// playing-time classes' values, joined in a line).
fn snaps_growth(share: f64) -> f64 {
    0.8 + 0.35 * clamp(share, 0.0, 1.15)
}

/// Potential's lean from real snaps, as `playing_time_potential`.
fn snaps_potential(share: f64) -> f64 {
    -0.25 + 0.4 * clamp(share, 0.0, 1.15)
}

fn playing_time_potential(pt: PlayingTime) -> f64 {
    match pt {
        PlayingTime::Starter => 0.15,
        PlayingTime::Rotation => 0.1,
        PlayingTime::Backup => -0.25,
        PlayingTime::Redshirt => 0.0,
        PlayingTime::PracticeSquad => -0.1,
        PlayingTime::Unsigned => -0.4,
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(not(feature = "debug"), allow(dead_code))]
pub struct SeasonReport {
    pub overall_before: f64,
    pub overall_after: f64,
    pub(crate) potential_before: f64,
    pub(crate) potential_after: f64,
    pub events: Events,
    pub games_missed: u32,
    /// A major injury at this step that heals: its rehab loss comes back at
    /// the next one (`YearHurt::rehab`, with real injuries).
    #[serde(default)]
    pub rehab_due: bool,
    /// What his skill project's click added to his ceiling this step (0
    /// when it didn't click), for your staff's note.
    #[serde(default)]
    pub project_click: f64,
}

/// What one yearly step drew and why (world reader lane R2): every term
/// behind the year's growth and potential move, filled in as the step runs.
/// Writing it draws nothing and changes nothing; the world log reads it
/// back with `take_why` right after the step, on the same thread.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct GrowthWhy {
    /// Ceiling points everyday coaching closed toward his limits, and a
    /// breakout's (the progression update).
    pub room_closed: f64,
    pub room_breakout: f64,
    /// His skill project's ceiling lift this year, and its click's part.
    pub project_lift: f64,
    pub project_click: f64,
    pub age: f64,
    /// Age less bloom: what the growth windows read.
    pub development_age: f64,
    /// Ratings regained in the year back from a major injury.
    pub rehab_regain: f64,
    pub injury_chance: f64,
    pub injury: Option<Ev>,
    /// A major injury: 0 heals fully, 1 a small lasting loss, 2 his burst gone.
    pub heal: Option<u8>,
    /// Rating points the injury took now, and ceiling points it took for good.
    pub injury_points: f64,
    pub injury_ceiling: f64,
    pub growth_multiplier: f64,
    pub snaps_factor: f64,
    /// Form before the year's event, and after it.
    pub form_base: f64,
    pub form: f64,
    /// Points off (or on) his on-field level this year.
    pub performance: f64,
    pub breakout_chance: f64,
    pub slump_chance: f64,
    pub lost_season_chance: f64,
    pub stagnation_chance: f64,
    pub payday_mult: f64,
    pub hot_last_year: bool,
    pub breakout_lift: f64,
    /// Rating points a slump takes after growth (summed over attributes).
    pub slump_points: f64,
    pub weight_gain: f64,
    pub poor_coaching_chance: f64,
    pub poor_coaching_points: f64,
    pub plasticity: f64,
    /// The potential drive and its parts: work ethic, coachability,
    /// coaching, situation, playing time (times snaps), setback, luck.
    pub drive: f64,
    pub drive_work: f64,
    pub drive_coachability: f64,
    pub drive_coaching: f64,
    pub drive_situation: f64,
    pub drive_playing: f64,
    pub drive_setback: f64,
    pub drive_luck: f64,
    pub spurt: bool,
    /// The overall of the ceilings' move before the cap, and the cap's scale.
    pub potential_shift: f64,
    pub shift_scale: f64,
    /// Ceiling points earned by veteran learning (summed).
    pub veteran_learned: f64,
    pub coaching_boost: f64,
    /// The strength staff's boost on the physical attributes' growth, and
    /// its extra move on their ceilings (minor bug 302).
    pub physical_boost: f64,
    pub drive_physical: f64,
    pub snaps_mult: f64,
    pub work_mult: f64,
    pub morale_mult: f64,
    pub physical_mult: f64,
    pub focus_take: f64,
    /// Growth landed at the checkpoint (summed), and how many attributes'
    /// growth was left to land week by week through training.
    pub landed: f64,
    pub deferred: u32,
    /// Ageing's loss (summed over attributes) and old skills faded.
    pub decline_points: f64,
    pub faded: u32,
    /// Each group's change at the checkpoint (summed): physical, technical, mental.
    pub change: [f64; 3],
    pub overall_before: f64,
    pub potential_before: f64,
}

thread_local! {
    static LAST_WHY: std::cell::Cell<Option<GrowthWhy>> = const { std::cell::Cell::new(None) };
}

/// The why of the last `develop_season` on this thread, once.
pub fn take_why() -> Option<GrowthWhy> {
    LAST_WHY.with(|c| c.take())
}

/// Develop a player through one year and age him by one. Mutates the player.
pub fn develop_season(player: &mut Player, ctx: &SeasonContext, rng: &mut Rng) -> SeasonReport {
    let personality = player.personality;
    let position = player.position;
    let overall_before = player.overall();
    let potential_before = player.potential();
    let speed_before = player.attributes.get(Attr::Speed);
    let mut events = Events::default();
    let age = player.age as f64;
    let pt = ctx.playing_time;
    let no_games = matches!(pt, PlayingTime::Redshirt | PlayingTime::PracticeSquad | PlayingTime::Unsigned)
        || ctx.trouble.is_some();
    let dip = clamp(ctx.adjustment + ctx.missed_camp, 0.0, 1.0);
    let situation = ctx.situation;
    let mut games_missed = 0;
    let attrs_before = player.attributes;
    let ceilings_before = player.ceilings;
    let mut why = GrowthWhy {
        age,
        development_age: age - player.bloom,
        overall_before,
        potential_before,
        ..GrowthWhy::default()
    };
    // The hidden traits drift a little each year, on their own stream.
    player.game.age_one_year(player.age, &mut Rng::new(player.id.wrapping_mul(0x2545_F491) ^ player.age));
    // The year back from a major injury (fix list 4.3): a player who healed
    // gets back the rehab loss his ceilings kept room for. One who never got
    // his burst back has no room to regain.
    // With real injuries (audit item 1) the step that took the loss says
    // so itself, since the row naming the injury is the season it happened.
    let year_back = match ctx.hurt {
        Some(h) => h.rehab,
        None => player
            .history
            .last()
            .is_some_and(|last| last.events.has(Ev::MajorInjury) && !last.events.has(Ev::LostBurst)),
    };
    if year_back {
        let age_hit = injury_age_hit(age - 1.0);
        for key in Attr::ALL {
            if key.profile().injury_sensitive {
                let room = (player.ceilings.get(key) - player.attributes.get(key)).max(0.0);
                let back = room.min(rng.range(1.5, 5.0) * age_hit);
                player.attributes.add(key, back);
                why.rehab_regain += back;
            }
        }
    }
    let mut rehab = false;
    // A first year at a new level is named on the row, so a page or a slump
    // list can give it as the reason he played below his rating. (A mark
    // only: it draws nothing.)
    if ctx.adjustment > 0.0 {
        events.add(Ev::FirstYear);
    }

    // --- Injuries: the main source of setbacks -------------------------------
    let mut growth_multiplier = 1.0;
    let mut snaps_factor = 1.0;
    let guarded = crate::game::sim::guarded_mix(
        &crate::game::sim::GAME_INJURIES,
        crate::game::sim::strength_guard(10.0 + ctx.strength_dev, 10.0),
    )
    .1;
    let real = ctx.hurt;
    let injury_chance = level_injury_rate(ctx.level)
        * position.profile().injury_risk
        * (0.55 + player.injury_proneness * 0.045)
        // A suspended or dismissed player is at most at practice, like a redshirt.
        * if ctx.trouble.is_some() || no_games { 0.5 } else if pt == PlayingTime::Starter { 1.15 } else { 0.9 }
        // His strength staff's guard on muscle strains and backs (minor bug
        // 303), on their share of the game mix.
        * guarded;
    why.injury_chance = if real.is_some() { 0.0 } else { injury_chance };
    // Real injuries (audit item 1): the worst one since his last step, by
    // its days out (a torn ACL always major), in place of the roll. Its
    // games are on the season rows already, counted game by game.
    let real_severity = real.and_then(|h| h.event());
    let hurt_now = match real {
        Some(_) => real_severity.is_some(),
        None => rng.chance(injury_chance),
    };
    let mut major = false;
    if hurt_now {
        let roll = if real.is_some() { 0.0 } else { rng.next() };
        // Strains and backs are minor or moderate, so the guard leaves the
        // major injury's chance where it was: its share of the roll moves
        // against the guard and minor and moderate split the rest 62:27.
        let (minor, moderate) = if guarded == 1.0 {
            (0.62, 0.89)
        } else {
            let major = (0.11 / guarded).min(1.0);
            ((1.0 - major) * 0.62 / 0.89, 1.0 - major)
        };
        let severity = if let Some(e) = real_severity {
            e
        } else if roll < minor {
            Ev::MinorInjury
        } else if roll < moderate {
            Ev::ModerateInjury
        } else {
            Ev::MajorInjury
        };
        // A real injury is named on the row of the season it happened in
        // (`world::season::mark_real_injury`, or the caller for one before
        // this row); one since the season ended is this row's.
        let here = real.is_none_or(|h| !h.named);
        if here {
            events.add(severity);
        }
        why.injury = Some(severity);
        let games = season_games(ctx.level) as f64;
        if real.is_none() {
            games_missed = match severity {
                Ev::MinorInjury => rng.int(0, 2),
                Ev::ModerateInjury => rng.int(2, 6),
                _ => rng.int(round_half_up(games * 0.5) as i32, games as i32),
            } as u32;
        }
        // Some bodies heal faster (hidden recovery; proposed, not confirmed:
        // up to about a quarter quicker or slower than average).
        if severity != Ev::MajorInjury && real.is_none() {
            games_missed = round_half_up(
                games_missed as f64
                    * (1.3 - 0.03 * player.game.recovery)
                    * (1.0 - 0.15 * clamp(ctx.medical, -1.0, 1.0)),
            ) as u32;
        }
        if no_games {
            games_missed = 0;
        }
        match severity {
            Ev::MinorInjury => {
                growth_multiplier *= 0.95;
                snaps_factor = 0.9;
            }
            Ev::ModerateInjury => {
                growth_multiplier *= 0.85;
                snaps_factor = 0.65;
            }
            _ => {
                major = true;
                growth_multiplier *= 0.5;
                snaps_factor = 0.2;
                // Torn ACLs and Achilles tendons take explosiveness, more so with
                // age. How it heals (fix list 4.3): in real NFL studies 62-74% of
                // players are back on the field after an ACL repair, and those who
                // return start fewer games (81% before, 57% after). Proposed, not
                // confirmed: 55% heal fully (the rehab loss comes back next year),
                // 30% keep a small lasting loss and 15% never get their burst back.
                // A real one heals by what it was (`major_heal`): one that
                // isn't in his legs costs the year and no burst.
                let age_hit = injury_age_hit(age);
                let split = match real.and_then(|h| h.kind) {
                    Some(k) => major_heal(k),
                    None => Some((0.55, 0.85)),
                };
                let heal = if split.is_some() { rng.next() } else { 0.0 };
                let (full, part) = split.unwrap_or((1.0, 1.0));
                let (now, lasting) = if split.is_none() {
                    ((0.0, 0.0), (0.0, 0.0))
                } else if heal < full {
                    ((1.5, 5.0), (0.0, 0.0))
                } else if heal < part {
                    ((1.5, 5.0), (1.0, 3.0))
                } else {
                    if here {
                        events.add(Ev::LostBurst);
                    } else if let Some(row) = player.history.last_mut() {
                        row.events.add(Ev::LostBurst);
                    }
                    ((3.0, 7.0), (3.0, 7.0))
                };
                rehab = split.is_some() && heal < part;
                why.heal = Some(if heal < full {
                    0
                } else if heal < part {
                    1
                } else {
                    2
                });
                for key in Attr::ALL {
                    if split.is_none() {
                        break;
                    }
                    if !key.profile().injury_sensitive {
                        continue;
                    }
                    let lost = rng.range(now.0, now.1) * age_hit;
                    player.attributes.add(key, -(lost));
                    why.injury_points += lost;
                    if lasting.1 > 0.0 {
                        let kept = rng.range(lasting.0, lasting.1) * age_hit;
                        player.ceilings.add(key, -(kept));
                        why.injury_ceiling += kept;
                    }
                }
                player.injury_proneness = (player.injury_proneness + 1.0).min(20.0);
                player.wear += 1.5;
            }
        }
    }

    // --- A body that stops growing ----------------------------------------------
    // High school stars who were simply bigger and faster than boys their age
    // often stop growing in college while everyone else catches up. Early
    // bloomers are the most likely to be done.
    // Once stalled, his physical ceilings stay where they settled.
    let mut stalled = player.has_had(Ev::BodyStalled);
    if (18.0..=20.0).contains(&age) && !stalled && rng.chance(STALL_CHANCE * (-0.7 * player.bloom).exp()) {
        stalled = true;
        events.add(Ev::BodyStalled);
        for key in Attr::ALL {
            if key.profile().group != AttributeGroup::Physical {
                continue;
            }
            player.ceilings.set(key, player.ceilings.get(key).min(player.attributes.get(key) + rng.range(0.0, 2.0)));
        }
    }

    // --- Form for the year: breakouts, slumps and lost seasons ----------------
    // Development is lumpy: a single season can be worth three normal ones, and a
    // good year is often followed by a dip as opponents adjust.
    // A first year at a new level and the situation around him show on the field
    // straight away; they touch growth only a little.
    // Young players feel their surroundings most: a bad team can shake a
    // young quarterback's confidence for years, and a good one can revive him.
    let youth = if age <= 25.0 { 2.0 } else { 1.0 };
    // Growth swings wider than it used to (fix list 4.2; was 0.38). Proposed, not confirmed.
    let mut form = rng.log_normal(0.5) * (1.0 - 0.15 * dip) * (1.0 + 0.12 * situation * youth);
    why.form_base = form;
    let mut setback = 0.0;
    let mut performance = rng.normal(0.0, 2.0) - 7.0 * dip + 2.5 * situation * youth;
    let mut breakout_lift = 0.0;
    // Losses that land after the year's growth (slumps, bad teaching).
    let mut late_loss = Attributes::filled(0.0);
    let n = player.history.len();
    let last = if n >= 1 { player.history.get(n - 1) } else { None };
    let before = if n >= 2 { player.history.get(n - 2) } else { None };
    let hot_last_year = match (last, before) {
        // A spring step is a real jump, not a lucky year that regresses
        // (part D 1): its gain is left out.
        (Some(l), b) => {
            l.events.has(Ev::Breakout) || b.is_some_and(|b| l.overall - b.overall - l.spring_gain.unwrap_or(0.0) >= 6.0)
        }
        _ => false,
    };
    let young_starter = pt == PlayingTime::Starter && age <= 22.0;
    let breakout_chance =
        (if age <= 23.0 {
            0.035
        } else if age <= 27.0 {
            0.02
        } else {
            0.01
        }) * (1.0 + (personality.competitiveness - 10.0) * 0.05 + (personality.work_ethic - 10.0) * 0.05)
            * if young_starter {
                1.5
            } else if pt == PlayingTime::Backup {
                0.6
            } else {
                1.0
            };
    let slump_chance = if hot_last_year { 0.16 } else { 0.06 };
    let stagnation_chance =
        0.05 - (personality.work_ethic - 10.0) * 0.003 - (personality.competitiveness - 10.0) * 0.0015;
    // Complacency, a bad scheme fit, a coach who never trusts him: the quiet reasons prospects bust.
    let base_lost =
        (0.03 + (10.0 - personality.work_ethic) * 0.004 + (10.0 - personality.competitiveness) * 0.002).max(0.005);
    // A big NIL deal or a big contract (fix list 4.5): the drive can go, more
    // so for a low work ethic. Proposed, not confirmed: 2.5 times as likely
    // at an average work ethic, 4 times at 4, about even at 16 and up.
    let payday_mult = if ctx.payday { clamp(2.5 + 0.25 * (10.0 - personality.work_ethic), 1.0, 4.5) } else { 1.0 };
    let lost_season_chance = if (17.0..=27.0).contains(&age) || ctx.payday { base_lost * payday_mult } else { 0.0 };
    why.breakout_chance = breakout_chance;
    why.slump_chance = slump_chance;
    why.lost_season_chance = lost_season_chance;
    why.stagnation_chance = stagnation_chance;
    why.payday_mult = payday_mult;
    why.hot_last_year = hot_last_year;
    if let Some(trouble) = ctx.trouble {
        // A year away from games and from the staff's trust: like a lost season, and it shows on the field.
        events.add(match trouble {
            Trouble::Suspended => Ev::Suspended,
            Trouble::Dismissed => Ev::Dismissed,
        });
        form *= 0.4;
        setback = 1.8;
        performance -= rng.range(3.0, 7.0);
    } else if !no_games && snaps_factor >= 0.65 && rng.chance(breakout_chance) {
        // (No breakout in a redshirt year or one mostly lost to injury.)
        events.add(Ev::Breakout);
        form *= 2.2;
        performance += rng.range(3.0, 7.0);
        // A proven breakout by a player already pressing against his ceiling shows
        // the ceiling was wrong: it can rise past the normal yearly limit, by less
        // as a player gets older. Further from the ceiling, a breakout is growth.
        let headroom = potential_before - overall_before;
        if headroom < 6.0 {
            breakout_lift = rng.range(2.0, 8.0)
                * if age <= 23.0 {
                    1.0
                } else if age <= 27.0 {
                    0.6
                } else {
                    0.35
                };
        }
    } else if rng.chance(lost_season_chance) {
        events.add(Ev::LostSeason);
        if ctx.payday {
            events.add(Ev::Complacent);
        }
        form *= 0.4;
        setback = 1.8;
        performance -= rng.range(3.0, 7.0);
    } else if !no_games && rng.chance(slump_chance) {
        // Production falls hard; the ratings dip a little as confidence and timing go.
        events.add(Ev::Slump);
        form *= 0.6;
        performance -= rng.range(4.0, 9.0);
        // A kicker's, punter's or snapper's rating is nearly all skill, so his
        // loss is halved to cost him about what a slump costs others (part B 17).
        // Fix list 4.1: 2-6 points (was 0.5-3), taken after the year's growth
        // so the year's growth can't hide it; it grows back over the next
        // years, and a veteran whose skills have settled keeps part of it.
        let size = if player.position.is_specialist() { 0.5 } else { 1.0 };
        for key in Attr::ALL {
            if key.profile().group != AttributeGroup::Physical {
                let lost = size * rng.range(2.0, 6.0);
                late_loss.add(key, lost);
                why.slump_points += lost;
            }
        }
    } else if rng.chance(stagnation_chance) {
        events.add(Ev::Stagnation);
        form *= 0.3;
    }
    // Item 14 (growth from results, A/B tested): with results replacing the
    // dice, the skills results test take the dice's average instead of the
    // year's draw: the mean form, the mean slump loss and the mean breakout
    // lift, so a player who barely plays grows as he does today, without
    // the surprise. A lost season, trouble and injuries stay as they are.
    let replace =
        ctx.trouble.is_none() && matches!(ctx.results, Some(m) if m.on && m.dice == crate::results::Dice::Replaced);
    let tested = if replace { crate::results::tested(position, ctx.level) } else { 0 };
    let mut form_tested = form;
    let mut lift_mean = 0.0;
    // A breakout's chance-weighted share, for the room it closes on the
    // skills results test (the progression update, everyday item 5).
    let mut breakout_mean = 0.0;
    if replace {
        let base = (0.125f64).exp() * (1.0 - 0.15 * dip) * (1.0 + 0.12 * situation * youth);
        if events.has(Ev::LostSeason) {
            form_tested = base * 0.4;
        } else {
            let pb = if !no_games && snaps_factor >= 0.65 { breakout_chance } else { 0.0 };
            let pl = (1.0 - pb) * lost_season_chance;
            let ps = (1.0 - pb) * (1.0 - lost_season_chance) * if no_games { 0.0 } else { slump_chance };
            let pst = (1.0 - pb)
                * (1.0 - lost_season_chance)
                * (1.0 - if no_games { 0.0 } else { slump_chance })
                * stagnation_chance;
            let kept = (1.0 - pl).max(1e-6);
            let none = 1.0 - pb - pl - ps - pst;
            form_tested = base * (pb * 2.2 + ps * 0.6 + pst * 0.3 + none) / kept;
            breakout_mean = pb / kept;
            let size = if player.position.is_specialist() { 0.5 } else { 1.0 };
            let slump_mean = ps / kept * 4.0 * size;
            for key in Attr::ALL {
                if crate::results::is_tested(tested, key) && key.profile().group != AttributeGroup::Physical {
                    late_loss.set(key, slump_mean);
                }
            }
            if potential_before - overall_before < 6.0 {
                let age_mult = if age <= 23.0 {
                    1.0
                } else if age <= 27.0 {
                    0.6
                } else {
                    0.35
                };
                lift_mean = pb / kept * 5.0 * age_mult * 0.9;
            }
        }
    }

    // --- Weight gain (fix list 4.4) ----------------------------------------------
    // Bad weight from a lax diet and offseason: it costs speed and quickness,
    // and only part of it ever comes off. Low work ethic and low discipline
    // raise the chance. Proposed, not confirmed: 2.5% a year for an average
    // player, about 6% for the laxest, about 1% for the most driven.
    if (18.0..=32.0).contains(&age) {
        let lax = clamp(
            1.0 + 0.12 * (10.0 - personality.work_ethic)
                + 0.06 * (10.0 - player.attributes.get(Attr::Discipline) / 5.0),
            0.3,
            2.5,
        );
        if rng.chance(0.025 * lax) {
            events.add(Ev::WeightGain);
            let pounds = rng.range(8.0, 25.0);
            player.body.weight += pounds;
            why.weight_gain = pounds;
            for key in [Attr::Speed, Attr::Acceleration, Attr::Agility] {
                player.attributes.add(key, -(rng.range(1.0, 3.0)));
                player.ceilings.add(key, -(rng.range(0.5, 2.0)));
            }
        }
    }

    // --- Teaching that costs ability (fix list 4.6) ------------------------------
    // A weak position coach teaches bad habits, and a scheme that doesn't fit
    // him asks for what he can't do: his technique can go backwards, not only
    // grow slower. Coachability protects him. Proposed, not confirmed: 4% a
    // year per coaching point under 8, plus up to 9% for a bad situation.
    let poor = clamp(0.04 * (8.0 - ctx.coaching), 0.0, 0.2) + clamp(0.15 * (-situation - 0.4), 0.0, 0.09);
    let poor_chance = poor * clamp(1.5 - 0.05 * personality.coachability, 0.5, 1.5);
    why.poor_coaching_chance = if poor > 0.0 { poor_chance } else { 0.0 };
    if poor > 0.0 && rng.chance(poor_chance) {
        events.add(Ev::PoorCoaching);
        for key in Attr::ALL {
            if key.profile().group == AttributeGroup::Technical && weight(position, key) > 0.0 {
                let lost = rng.range(1.0, 3.0);
                late_loss.add(key, lost);
                why.poor_coaching_points += lost;
            }
        }
    }
    why.form = form;
    why.drive_setback = -setback;
    why.breakout_lift = breakout_lift;
    why.growth_multiplier = growth_multiplier;
    why.snaps_factor = snaps_factor;

    // --- Dynamic potential -----------------------------------------------------
    // Ceilings move most in the mid-teens, settle through the early 20s and only
    // fall after that. Direction is driven by personality, coaching and snaps;
    // luck plays a smaller part.
    let plasticity = potential_plasticity(player.age);
    // Strength, speed and conditioning are the strength staff's (minor bug
    // 302): its mix replaces the position coach's share for the physical
    // attributes, here on their ceilings and below on their growth.
    let physical_coaching = ctx.physical_coaching.map_or(ctx.coaching, |c| clamp(c, 1.0, 20.0));
    if plasticity > 0.0 {
        // A young pro sitting behind a veteran still practises against NFL players
        // with an NFL staff: unlike a buried college backup, his ceiling doesn't sag.
        let young_pro =
            ctx.level == Level::NFL && age <= 24.0 && matches!(pt, PlayingTime::Backup | PlayingTime::PracticeSquad);
        // The same sum as one line, with its parts named for the log.
        let work = ((personality.work_ethic - 10.0) / 10.0) * 0.75;
        let coachability = ((personality.coachability - 10.0) / 10.0) * 0.25 * ((ctx.coaching - 10.0) / 10.0 + 0.5);
        let coaching = ((ctx.coaching - 10.0) / 10.0) * 0.35;
        let surroundings = 0.25 * situation;
        let playing = (if young_pro { 0.0 } else { ctx.snaps.map_or(playing_time_potential(pt), snaps_potential) })
            * snaps_factor;
        let luck = rng.normal(0.0, 0.9);
        let drive = work + coachability + coaching + surroundings + playing - setback + luck;
        // The physical attributes' drive reads the strength staff's mix in
        // place of the teaching mix (minor bug 302).
        let coachability_p =
            ((personality.coachability - 10.0) / 10.0) * 0.25 * ((physical_coaching - 10.0) / 10.0 + 0.5);
        let coaching_p = ((physical_coaching - 10.0) / 10.0) * 0.35;
        // Grouped so equal mixes give exactly 0 (no strength staff: as before).
        let physical_extra = ((coachability_p - coachability) + (coaching_p - coaching)) * 1.7 * plasticity;
        // Speed and burst read only the strength head's own part: the same
        // drive at his staff's mix against the mix with a typical strength
        // head in it, so the position coach and the room don't move them.
        let staff_mix = |c: f64| {
            ((personality.coachability - 10.0) / 10.0) * 0.25 * ((c - 10.0) / 10.0 + 0.5) + ((c - 10.0) / 10.0) * 0.35
        };
        let speed_staff = if ctx.physical_coaching.is_some() {
            (staff_mix(physical_coaching) - staff_mix(physical_coaching - 0.4 * ctx.strength_dev)) * 1.7 * plasticity
        } else {
            0.0
        };
        why.drive_physical = physical_extra;
        why.plasticity = plasticity;
        why.drive = drive;
        why.drive_work = work;
        why.drive_coachability = coachability;
        why.drive_coaching = coaching;
        why.drive_situation = surroundings;
        why.drive_playing = playing;
        why.drive_luck = luck;
        let shared = drive * 1.7 * plasticity;
        let spurt = age <= 17.0 && rng.chance(0.09);
        why.spurt = spurt;
        if spurt {
            events.add(Ev::GrowthSpurt);
        }
        let mut delta = Attributes::filled(0.0);
        for key in Attr::ALL {
            let physical = key.profile().group == AttributeGroup::Physical;
            let mut d = shared + rng.normal(0.0, 1.2 * plasticity);
            if physical {
                d += physical_extra;
            }
            if spurt && physical {
                d += rng.range(2.0, 6.0) * if matches!(key, Attr::Strength | Attr::LowerBody) { 1.3 } else { 0.8 };
            }
            // Speed and burst are his body's (the speed lane, 2026-11-17):
            // his work, his coaches, his snaps and the year's luck don't move
            // what he can run. Measured on the class sim, this drive lifted a
            // drafted man's speed ceiling 8.8 points from 14 (about 0.11 s of
            // forty, sd 6), against real college forties that don't change
            // over four years (Jacobson et al. 2013) and the weight room's
            // 1-point speed leash. The strength head's own part over the
            // level's typical one stays (minor bug 302), and so do a growth
            // spurt and the year's own noise; the weight room and a speed
            // project train them on their leashes. Proposed, not confirmed.
            if matches!(key, Attr::Speed | Attr::Acceleration) {
                d += speed_staff - shared - physical_extra;
            }
            if stalled && d > 0.0 && physical {
                d = 0.0;
            }
            let current = player.ceilings.get(key);
            delta.set(key, clamp(current + d, MIN_RATING, RATING_BOUND) - current);
        }
        // A potential shift can move at most one band per checkpoint: no overnight stars.
        let shift = overall_for(position, &delta);
        let scale = if shift.abs() > MAX_POTENTIAL_SHIFT { MAX_POTENTIAL_SHIFT / shift.abs() } else { 1.0 };
        why.potential_shift = shift;
        why.shift_scale = scale;
        for i in 0..ATTRIBUTE_COUNT {
            player.ceilings.set_at(i, player.ceilings.at(i) + delta.at(i) * scale);
        }
    }
    // Reading defenses, poise and route timing keep deepening with real snaps
    // well after the body has stopped growing: veterans earn new ceiling and
    // most of it shows up on the field straight away.
    if (21.0..=31.0).contains(&age) && ctx.trouble.is_none() {
        let snaps_share = match (ctx.snaps, pt) {
            (Some(x), _) => clamp(x, 0.2, 1.0),
            (_, PlayingTime::Starter) => 1.0,
            (_, PlayingTime::Rotation) => 0.6,
            _ => 0.2,
        };
        for key in Attr::ALL {
            let profile = key.profile();
            if profile.experience < 0.6 {
                continue;
            }
            let learned = rng.range(0.0, 1.2) * profile.experience * snaps_share * snaps_factor;
            if unrelated(position, key, false) {
                continue;
            }
            why.veteran_learned += learned;
            player.ceilings.add(key, learned);
            player.attributes.add(key, learned * 0.8);
        }
    }
    if breakout_lift > 0.0 || lift_mean > 0.0 {
        for key in Attr::ALL {
            if weight(position, key) == 0.0 || (stalled && key.profile().group == AttributeGroup::Physical) {
                continue;
            }
            // A tested skill takes the dice's average lift (item 14).
            let add = if crate::results::is_tested(tested, key) {
                lift_mean
            } else if breakout_lift > 0.0 {
                breakout_lift * rng.range(0.6, 1.2)
            } else {
                0.0
            };
            if add > 0.0 {
                player.ceilings.set(key, (player.ceilings.get(key) + add).min(RATING_BOUND));
            }
        }
    }
    // The split ceiling (the progression update): everyday coaching closes
    // part of the room between his everyday ceiling and what his body and
    // makeup allow, by his knack (everyday item 1), and a breakout closes
    // it at a click's size (item 5). On its own stream.
    // His position coach's one-on-one time, thinner when the room runs
    // projects he isn't on (design item 8): a little less of his coaching
    // sticks, in the room's closing and his climb.
    let room_coaching = ctx.coaching - crate::progression::room_coaching_cut(ctx.coaching, ctx.room_time);
    why.room_closed = crate::progression::everyday(player, room_coaching, ctx.level, pt, ctx.trouble.is_some());
    why.room_breakout =
        crate::progression::breakout(player, ctx.level, events.has(Ev::Breakout), breakout_mean, tested);
    // His skill project's year (design items 9-11): its ceiling, a click.
    let project = crate::progression::project_year(player, ctx.coaching, ctx.level, ctx.trouble.is_some());
    why.project_lift = project.lift;
    why.project_click = project.click;

    // --- Growth towards the ceiling --------------------------------------------
    let coaching_mult = 0.75 + room_coaching * 0.025; // 10 -> 1.0, 20 -> 1.25
    // How much of coaching sticks, a close bond with his teacher adding up
    // to 3 points of coachability (coach and player bonds, question 4).
    let coachability_lean = 0.6 + (personality.coachability + ctx.bond.clamp(0.0, 3.0)) * 0.04;
    let work_mult = 0.75 + personality.work_ethic * 0.025;
    let snaps_mult =
        ctx.snaps.map_or(playing_time_growth(pt), snaps_growth) * (0.7 + 0.3 * snaps_factor) * level_growth(ctx.level);
    let hs = ctx.level == Level::HS;
    let college = ctx.level.is_college();
    let development_age = age - player.bloom;
    // Coachability scales how far coaching moves him from average, good or bad.
    let coaching_boost = 1.0 + (coaching_mult - 1.0) * coachability_lean;
    // His mood (up to 10%) and his room's (up to 5%, `people::morale`
    // gives -1.5 to 1.5; it was cut at 1, so the room added nothing to a
    // player already at his limit).
    let morale_mult = 1.0 + 0.10 * clamp(ctx.morale, -1.5, 1.5);
    // The strength staff's mix for the physical attributes (minor bug 302),
    // and its part of `physical` comes out so it counts once.
    let physical_boost = 1.0 + (0.75 + physical_coaching * 0.025 - 1.0) * coachability_lean;
    let physical_mult = 1.0 + 0.08 * clamp(ctx.physical - 0.06 * ctx.strength_dev, -1.0, 1.0);
    let mut settling = [false; ATTRIBUTE_COUNT];
    // The plan's focus (section 6b topic 5, individual practice topic 5b).
    let focus_take = focus_take(personality.coachability);
    let focus = ctx.focus.map(|a| Focus::two(a, ctx.focus2, ctx.focus_minutes));
    why.coaching_boost = coaching_boost;
    why.physical_boost = physical_boost;
    why.snaps_mult = snaps_mult;
    why.work_mult = work_mult;
    why.morale_mult = morale_mult;
    why.physical_mult = physical_mult;
    why.focus_take = focus_take;
    let g = GrowthCtx {
        coaching_boost,
        physical_boost,
        snaps_mult,
        work_mult,
        form,
        growth_multiplier,
        morale_mult,
        physical_mult,
        hs,
        college,
        redshirt: pt == PlayingTime::Redshirt,
        focus,
        focus_take,
        tested,
        form_tested,
        project: player.prog.project(),
        cost_mask: player.prog.project().map_or(0, |a| crate::progression::cost_mask(position, a)),
    };
    // Speed and burst come with the body (S4): what his growing body has
    // reached by the end of the year, he has.
    if age < 21.0 && !stalled {
        let share = speed_with_body(age, player.bloom);
        for key in [Attr::Speed, Attr::Acceleration] {
            let lift = share * player.ceilings.get(key) - player.attributes.get(key);
            if lift > 0.0 {
                player.attributes.add(key, lift);
            }
        }
    }
    // A high school corner also plays offense or returns kicks (A7): his
    // snaps with the ball are practice, so his moves and vision grow like a
    // ball carrier's and keep finding room, most for the gifted returner
    // (proposed, not confirmed: 2.5% more room a year).
    let carries = hs && position == Position::CB;
    if carries {
        for key in [Attr::Elusiveness, Attr::BallCarrierVision] {
            let c = player.ceilings.get(key);
            player.ceilings.set(key, (c * 1.025).min(RATING_BOUND));
        }
    }
    let grown_from = player.attributes;
    let mut focus_extra: Vec<(Attr, f64)> = Vec::new();
    for key in Attr::ALL {
        let profile = &ATTRIBUTE_PROFILES[key.index()];
        let age_factor = growth_age_factor(development_age, profile.growth_window);
        settling[key.index()] = age_factor <= 0.01;
        let gap = player.ceilings.get(key) - player.attributes.get(key);
        if gap <= 0.0 || age_factor <= 0.01 {
            continue;
        }
        let rate_of = |g: &GrowthCtx| {
            if carries && matches!(key, Attr::Elusiveness | Attr::BallCarrierVision) {
                (g.rate(key, age_factor) / hs_group_growth(profile.group)).min(0.5)
            } else {
                g.rate(key, age_factor)
            }
        };
        let rate = rate_of(&g);
        let noise = rng.normal(0.0, 0.5);
        // A skill his position has nothing to do with gets no practice (A6).
        if unrelated(position, key, hs) {
            continue;
        }
        // Growth that lands through the year's training (choice 1): the
        // draw is still taken, so every later draw is the one it was.
        if ctx.defer
            && crate::training::defers(player.position, key)
            && !declines(player.position, player.longevity, key, age)
        {
            why.deferred += 1;
            continue;
        }
        // Proportional growth plus a fixed step, so a well-developed player can
        // actually reach his ceiling instead of creeping towards it forever.
        let add = gap.min(gap * rate + FINISHING_STEP * rate) + noise;
        if focus.is_some_and(|f| f.has(key)) {
            let r0 = rate_of(&g.plain());
            focus_extra.push((key, add - (gap.min(gap * r0 + FINISHING_STEP * r0) + noise)));
        }
        player.attributes.add(key, add);
        why.landed += add;
    }
    // What the focus grew here, its key skills pay for (topic 5b).
    for (a, x) in focus_extra {
        why.landed -= focus_cost(position, &grown_from, &mut player.attributes, a, x);
    }
    if ctx.defer {
        let old = player.train;
        // Snaps land in the week he plays them (growth from results, item
        // 1): the year opens at a backup's rate and each game's snaps add
        // their part (`results::reps`), instead of last season's snaps
        // read a year late. The skills that settle here keep the old read.
        // Kickers, punters and snappers take no snaps to land: their role
        // decides, as before (minor bug 264). A season whose snaps never
        // landed in season (results off, a team on the stand-in, unsigned
        // at camp, an older save; `results::settle_reps`) is read here as
        // before, once, and this one's land as he plays.
        let in_season = ctx.results.is_some_and(|m| m.on) && !position.is_specialist();
        let mut gt = g;
        if in_season && !reads_late(player, ctx.level) {
            gt.snaps_mult = reps_base(pt) * (0.7 + 0.3 * snaps_factor) * level_growth(ctx.level);
        }
        player.train = crate::training::Train::open(&gt, player.age, development_age, player.position, old.load);
        player.train.open_results(in_season, replace, ctx.level, form_tested * growth_multiplier);
        player.train.grade = old.grade;
        player.train.prep = old.prep;
        player.train.rust = old.rust;
        player.train.pos |= old.focus_bits();
    }

    for key in Attr::ALL {
        player.attributes.add(key, -(late_loss.get(key)));
    }

    // --- Ageing ----------------------------------------------------------------
    let decline_resistance = 1.15 - personality.work_ethic * 0.015; // 10 -> 1.0, 20 -> 0.85
    // Years of heavy hits wear a body down faster; a rare body (high longevity) shrugs off more of it.
    let wear_mult = 1.0 + 0.06 * (player.wear - 3.0).max(0.0) * clamp(1.0 - 0.15 * player.longevity, 0.5, 1.5);
    for key in Attr::ALL {
        let profile = key.profile();
        let physical = profile.group == AttributeGroup::Physical;
        let years_past = age + 1.0 - decline_onset(position, player.longevity, key);
        if years_past <= 0.0 {
            continue;
        }
        settling[key.index()] = true;
        // A veteran's focus on a skill slows its decline, by up to a third
        // at 60 minutes a week (individual practice, topic 5b; proposed, not
        // confirmed). The body is not slowed by individual work.
        let worked = match focus {
            Some(f) if f.has(key) && !physical => 1.0 - f.lift_on(key, focus_take) / 3.0,
            _ => 1.0,
        };
        let loss = profile.decline_rate
            * years_past
            * decline_resistance
            * rng.log_normal(0.3)
            * if physical { wear_mult } else { 1.0 }
            * worked;
        player.attributes.add(key, -(loss));
        player.ceilings.add(key, -(loss));
        why.decline_points += loss;
    }

    // --- Old skills fade (fix list 3.8) ------------------------------------------
    // A skill only an old position used fades each year he doesn't practice
    // it: a converted quarterback's passing, a former receiver's routes.
    // Only positions he held from 18 on count: a high school freshman's
    // first spot is rarely drilled (faded from 14, it put 9 first-rounders a
    // year from outside the consensus top 50 against a real 1-3).
    // (Proposed, not confirmed: 1.5 points a year, the room above it too.)
    if player.position_history.iter().any(|h| h.1 != position) {
        for key in Attr::ALL {
            if key.profile().group == AttributeGroup::Physical || weight(position, key) > 0.01 {
                continue;
            }
            let old = player
                .position_history
                .iter()
                .filter(|h| h.1 != position && h.0 >= 18)
                .map(|h| weight(h.1, key))
                .fold(0.0, f64::max);
            if old >= 0.04 {
                player.attributes.add(key, -(OLD_SKILL_FADE));
                player.ceilings.add(key, -(OLD_SKILL_FADE));
                why.faded += 1;
            }
        }
    }

    // --- The weight room (update 3, A4, A14) -------------------------------------
    // Every player's strength program pulls him toward his position's weight,
    // not only a player who changes position: up to about 30 lb past where
    // his frame put him, never past the position's weight, and only up, so a
    // naturally big receiver keeps his size. A strength staff, his work ethic
    // and college or pro programs move him faster (proposed, not confirmed: a
    // third of the way a year in college and the pros, an eighth in high
    // school, from 15 to 25).
    if (15.0..=25.0).contains(&age) && !position.is_specialist() {
        let room = weight_room_room(position, player.original_potential, player.body.frame_weight);
        // The dietitians move the plan faster (support departments, Q4).
        let pace =
            weight_room_pace(hs, personality.work_ethic, ctx.physical) * crate::world::departments::diet_pace(ctx.diet);
        // A body project's pounds sit past this room (design item 2), so
        // they never count against it.
        let gap = room + player.prog.body_lbs as f64 - player.body.weight;
        if gap > 0.0 {
            player.body.weight += gap * clamp(pace, 0.0, 1.0);
        }
    }

    // The normal weight room inside the leashes (everyday item 6): his body
    // knack moves his physical ceilings from the typical path.
    crate::progression::weight_room(player, ctx.level, ctx.trouble.is_some());
    // His body project, if he has one (design items 1, 2 and 7): the
    // strength staff moves his ceilings on its leash and puts weight on him.
    crate::progression::body_year(player, ctx.level, physical_coaching, ctx.trouble.is_some());

    // --- Wear (update 3, F14) ------------------------------------------------------
    // Every season played wears a body down, in college at half the pros'
    // rate, not only a major injury.
    let wear_level = match ctx.level {
        Level::NFL => 1.0,
        Level::HS => 0.0,
        _ => 0.5,
    };
    let played_share = match (ctx.snaps, pt) {
        _ if no_games => 0.0,
        (Some(x), _) => clamp(x, 0.0, 1.0),
        (_, PlayingTime::Starter) => 1.0,
        (_, PlayingTime::Rotation) => 0.5,
        _ => 0.0,
    };
    player.wear += season_wear(player, played_share) * wear_level;

    // --- Tidy up ---------------------------------------------------------------
    for i in 0..ATTRIBUTE_COUNT {
        let (mut attr, mut ceil) = (player.attributes.at(i), player.ceilings.at(i));
        // Once an attribute has stopped growing or started to decline, the ceiling
        // above it is out of reach: veterans don't keep upside they can't use.
        // (A healing injury keeps its room: the rehab loss comes back next year.)
        if settling[i] && ceil > attr && !(rehab && Attr::ALL[i].profile().injury_sensitive) {
            ceil = attr + (ceil - attr) / 2.0;
        }
        // The thin top (physical study items 1 and 17): a year's rise past
        // the knee counts for less the nearer he is to the top, as at
        // creation. Strength and the legs keep a fat top.
        if !matches!(Attr::ALL[i], Attr::Strength | Attr::LowerBody) {
            ceil = crate::athlete::top_up(ceilings_before.at(i), ceil);
            attr = crate::athlete::top_up(attrs_before.at(i), attr);
        }
        ceil = clamp(ceil, MIN_RATING, RATING_BOUND);
        attr = clamp(attr, MIN_RATING, RATING_BOUND);
        // Over-performing a ceiling means the ceiling was wrong, not the player.
        if attr > ceil {
            ceil = attr;
        }
        player.attributes.set_at(i, attr);
        player.ceilings.set_at(i, ceil);
    }
    // A natural rise past a limit lifts it (everyday item 2).
    crate::progression::settle(player, &ceilings_before);

    // His habits (tendencies topic 3): results, coaching, the staff's
    // style, age and a major injury. On their own stream.
    let played = if no_games {
        0.0
    } else {
        match pt {
            PlayingTime::Starter => 1.0,
            PlayingTime::Rotation => 0.7,
            _ => 0.3,
        }
    };
    let year = crate::tendencies::YearCtx {
        level: ctx.level,
        teaching: ctx.coaching,
        style: ctx.style,
        scheme: ctx.scheme,
        played,
        major_injury: major,
        speed_lost: (speed_before - player.attributes.get(Attr::Speed)).max(0.0),
        centre: ctx.tend_centre,
    };
    crate::tendencies::develop_year(&mut player.tend, &player.attributes, &personality, player.age, &year, player.id);

    // With growth landing through the year, the row's end is what the
    // year's growth makes of him; `training::close_year` sets it to what
    // really landed when the next checkpoint opens.
    let overall_after = if ctx.defer { crate::training::projected_overall(player) } else { player.overall() };
    let potential_after = player.potential();
    // A big year on the field from a player whose rating still fell is a career year, not a breakout.
    if overall_after < overall_before {
        events.remove(Ev::Breakout);
    }
    // He improves (or fades) through the year, so he plays the season between his two ratings.
    // One row more, not double the room (memory: a world holds two million rows).
    player.history.reserve_exact(1);
    player.history.push(SeasonRecord {
        position: player.position,
        age: player.age,
        level: ctx.level,
        playing_time: pt,
        overall_start: overall_before,
        overall: overall_after,
        potential: potential_after,
        performance: if no_games {
            OptF64::NONE
        } else {
            Some(clamp((overall_before + overall_after) / 2.0 + performance, MIN_RATING, RATING_BOUND)).into()
        },
        level_rating: None,
        level_rating_end: None,
        coaching: ctx.coaching,
        events,
        games_missed,
        adjustment: if dip > 0.0 { Some(round_half_up(dip * 100.0) / 100.0).into() } else { OptF64::NONE },
        spring_gain: OptF64::NONE,
        spring_open: OptF64::NONE,
        team: None,
        depth: None,
        situation: OptF64::NONE,
        results: OptF64::NONE,
    });
    // His flaws and tools through the year (individual practice topic 3):
    // his staff fixes and adds, reps teach tools, on his own stream.
    let playing = match pt {
        PlayingTime::Starter => 1.0,
        PlayingTime::Rotation => 0.6,
        PlayingTime::Backup => 0.3,
        _ => 0.1,
    };
    let staff = (!ctx.seat_plan).then(|| ctx.style.map_or(10.0, |s| s.mould));
    crate::craft::year(player, ctx.coaching, ctx.level, playing, staff);
    player.age += 1;
    // The day's body stays where it was: a year of age moves out of `grow`.
    player.grow = (player.grow - 1.0).max(0.0);
    why.performance = performance;
    for key in Attr::ALL {
        let g = match key.profile().group {
            AttributeGroup::Physical => 0,
            AttributeGroup::Technical => 1,
            AttributeGroup::Mental => 2,
        };
        why.change[g] += player.attributes.get(key) - attrs_before.get(key);
    }
    LAST_WHY.with(|c| c.set(Some(why)));
    SeasonReport {
        overall_before,
        overall_after,
        potential_before,
        potential_after,
        events,
        games_missed,
        rehab_due: rehab,
        project_click: project.click,
    }
}

/// A major injury costs an older body more.
fn injury_age_hit(age: f64) -> f64 {
    if age >= 28.0 {
        1.6
    } else if age >= 24.0 {
        1.2
    } else {
        1.0
    }
}

/// How freely a ceiling can still move at a given age (0 = only falls with age).
pub fn potential_plasticity(age: u32) -> f64 {
    match age {
        0..=17 => 1.0,
        18..=20 => 0.75,
        21..=23 => 0.5,
        24..=26 => 0.2,
        _ => 0.0,
    }
}

/// Share of the normal growth rate available at a development age.
pub fn growth_age_factor(age: f64, window: (f64, f64)) -> f64 {
    let (start, end) = window;
    if age < start {
        0.75
    } else if age <= end {
        1.0
    } else {
        0.55f64.powf(age - end)
    }
}

/// Extra practices outside the yearly checkpoint (section 4, A21: bowl
/// practices): a small step toward his ceilings for a young player, more
/// with good coaching and work ethic. Returns the overall gained. Proposed,
/// not confirmed: 15 practices close about 4% of the gap for an 18-year-old.
pub fn extra_practices(player: &mut Player, practices: u32, coaching: f64, rng: &mut Rng) -> f64 {
    let age = player.age as f64;
    if age > 22.0 || practices == 0 {
        return 0.0;
    }
    let before = player.overall();
    let youth = clamp((23.0 - age) / 5.0, 0.0, 1.0);
    let rate = 0.0027
        * practices as f64
        * youth
        * (0.75 + clamp(coaching, 1.0, 20.0) * 0.025)
        * (0.75 + player.personality.work_ethic * 0.025);
    let position = player.position;
    for key in Attr::ALL {
        if weight(position, key) == 0.0 {
            continue;
        }
        let room = (player.ceilings.get(key) - player.attributes.get(key)).max(0.0);
        player.attributes.add(key, room * rate * rng.range(0.5, 1.5));
    }
    player.overall() - before
}

/// A spring breakout (section 4, topic 4.1): a sudden real step by the spring
/// game, closing `share` of the gap to his ceilings on the attributes his
/// position uses. Room under the ceiling bounds it. Returns the overall gained.
pub fn spring_jump(player: &mut Player, share: f64, rng: &mut Rng) -> f64 {
    let before = player.overall();
    let position = player.position;
    for key in Attr::ALL {
        if weight(position, key) == 0.0 {
            continue;
        }
        let room = (player.ceilings.get(key) - player.attributes.get(key)).max(0.0);
        player.attributes.add(key, room * clamp(share * rng.range(0.7, 1.3), 0.0, 0.9));
    }
    player.overall() - before
}

/// An early enrollee's college spring step, taken before his first college
/// row (F2): that row opens with it. The row starts where his high school
/// row ended, before the step, and carries the step on its way to its end.
/// The step is the row's `spring_open` (and in its `spring_gain` total).
pub fn open_row_with_spring_step(player: &mut Player, gain: f64) {
    if let Some(h) = player.history.last_mut() {
        h.overall_start -= gain;
        h.spring_gain = Some(h.spring_gain.unwrap_or(0.0) + gain).into();
        h.spring_open = Some(h.spring_open.unwrap_or(0.0) + gain).into();
        h.events.add(Ev::SpringBreakout);
    }
}

/// Puts a spring breakout of `gain` overall on his latest season row
/// (section 4, T4.1): a row covers the season and the offseason after it,
/// so its end runs through the step and the next row starts where it ends.
pub fn record_spring_step(player: &mut Player, gain: f64) {
    if let Some(h) = player.history.last_mut() {
        h.overall += gain;
        h.spring_gain = Some(h.spring_gain.unwrap_or(0.0) + gain).into();
        h.events.add(Ev::SpringBreakout);
    }
}
