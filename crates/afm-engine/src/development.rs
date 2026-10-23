//! One development checkpoint covers a full football year: the season plus the
//! offseason that follows. It is meant to run at the yearly potential review
//! (W1 of the AFM calendar), after which the player is a year older.

use crate::attributes::{
    ATTRIBUTE_COUNT, ATTRIBUTE_PROFILES, Attr, AttributeGroup, Attributes, MAX_RATING, MIN_RATING,
};
use crate::player::{DevelopmentEvent as Ev, Events, Level, OptF64, Player, PlayingTime, SeasonRecord};
use crate::positions::{overall_for, weight};
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
    /// Section 6b topic 5: his plan's main focus, set by you. The focus
    /// grows up to 1.5 times its share and his other skills pay for it
    /// (choice 2, confirmed); how much sticks follows his coachability.
    pub focus: Option<Attr>,
    /// Section 6b topic 12 (choice 1, confirmed): the year's growth is
    /// decided here and lands week by week as the training happens
    /// (`training::land`), instead of all at once. Attributes that have
    /// stopped growing or decline this year still settle at once.
    pub defer: bool,
}

impl SeasonContext {
    pub fn new(level: Level, coaching: f64, playing_time: PlayingTime) -> Self {
        SeasonContext {
            level,
            coaching,
            playing_time,
            trouble: None,
            adjustment: 0.0,
            situation: 0.0,
            payday: false,
            snaps: None,
            style: None,
            scheme: None,
            tend_centre: [0.0; crate::tendencies::TEND_COUNT],
            morale: 0.0,
            physical: 0.0,
            medical: 0.0,
            focus: None,
            defer: false,
        }
    }
}

/// What sets how fast he grows this year, besides each attribute's own
/// profile and his age: the yearly step decides it, and the weekly landing
/// (`training::land`) reads it back from his `Train`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GrowthCtx {
    pub coaching_boost: f64,
    pub snaps_mult: f64,
    pub work_mult: f64,
    pub form: f64,
    pub growth_multiplier: f64,
    pub morale_mult: f64,
    pub physical_mult: f64,
    pub hs: bool,
    pub college: bool,
    pub redshirt: bool,
    pub focus: Option<Attr>,
    pub focus_take: f64,
}

impl GrowthCtx {
    /// The share of the gap to his ceiling an attribute closes in a full
    /// year (the yearly step's rate, unchanged).
    pub fn rate(&self, key: Attr, age_factor: f64) -> f64 {
        let profile = &ATTRIBUTE_PROFILES[key.index()];
        let coaching = self.coaching_boost.powf(profile.coachability);
        let snaps = self.snaps_mult.powf(profile.experience);
        let redshirt_lift = if self.redshirt && profile.group == AttributeGroup::Physical { 1.35 } else { 1.0 };
        // College staffs teach the technique and the reads high school never did, so the raw catch up there.
        let hs_lag = if self.hs {
            if key == Attr::Strength { HS_STRENGTH_GROWTH } else { hs_group_growth(profile.group) }
        } else if self.college && profile.group != AttributeGroup::Physical {
            COLLEGE_TEACHING
        } else {
            1.0
        };
        let focus_mult = match self.focus {
            Some(f) if f == key => 1.0 + 0.5 * self.focus_take,
            Some(f) if key.profile().group == f.profile().group => 1.0 - 0.035 * self.focus_take,
            _ => 1.0,
        };
        clamp(
            profile.growth_rate
                * age_factor
                * coaching
                * snaps
                * self.work_mult
                * self.form
                * self.growth_multiplier
                * redshirt_lift
                * hs_lag
                * self.morale_mult
                * focus_mult
                * if profile.group == AttributeGroup::Physical { self.physical_mult } else { 1.0 },
            0.0,
            0.5,
        )
    }
}

/// Whether an attribute is past its decline onset this year, by the
/// ageing step's rule.
pub(crate) fn declines(position: crate::positions::Position, longevity: f64, key: Attr, age: f64) -> bool {
    let profile = key.profile();
    let physical = profile.group == AttributeGroup::Physical;
    let pos_offset = position.profile().aging_offset;
    let onset = profile.decline_age + longevity + if physical { pos_offset.min(1.5) } else { pos_offset };
    age + 1.0 - onset > 0.0
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
    let add = gap.min((gap + given) * prop + FINISHING_STEP * rate * share).min(MAX_YEARLY_GAIN * share) + noise;
    let before = attrs.get(key);
    let attr = clamp(before + add, MIN_RATING, MAX_RATING);
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
const MAX_YEARLY_GAIN: f64 = 12.0;
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
    let dip = ctx.adjustment;
    let situation = ctx.situation;
    let mut games_missed = 0;
    // The hidden traits drift a little each year, on their own stream.
    player.game.age_one_year(player.age, &mut Rng::new(player.id.wrapping_mul(0x2545_F491) ^ player.age));
    // The year back from a major injury (fix list 4.3): a player who healed
    // gets back the rehab loss his ceilings kept room for. One who never got
    // his burst back has no room to regain.
    if let Some(last) = player.history.last()
        && last.events.has(Ev::MajorInjury)
        && !last.events.has(Ev::LostBurst)
    {
        let age_hit = injury_age_hit(age - 1.0);
        for key in Attr::ALL {
            if key.profile().injury_sensitive {
                let room = (player.ceilings.get(key) - player.attributes.get(key)).max(0.0);
                player.attributes.add(key, room.min(rng.range(1.5, 5.0) * age_hit));
            }
        }
    }
    let mut rehab = false;
    // A first year at a new level is named on the row, so a page or a slump
    // list can give it as the reason he played below his rating. (A mark
    // only: it draws nothing.)
    if dip > 0.0 {
        events.add(Ev::FirstYear);
    }

    // --- Injuries: the main source of setbacks -------------------------------
    let mut growth_multiplier = 1.0;
    let mut snaps_factor = 1.0;
    let injury_chance = level_injury_rate(ctx.level)
        * position.profile().injury_risk
        * (0.55 + player.injury_proneness * 0.045)
        // A suspended or dismissed player is at most at practice, like a redshirt.
        * if ctx.trouble.is_some() || no_games { 0.5 } else if pt == PlayingTime::Starter { 1.15 } else { 0.9 };
    if rng.chance(injury_chance) {
        let roll = rng.next();
        let severity = if roll < 0.62 {
            Ev::MinorInjury
        } else if roll < 0.89 {
            Ev::ModerateInjury
        } else {
            Ev::MajorInjury
        };
        events.add(severity);
        let games = season_games(ctx.level) as f64;
        games_missed = match severity {
            Ev::MinorInjury => rng.int(0, 2),
            Ev::ModerateInjury => rng.int(2, 6),
            _ => rng.int(round_half_up(games * 0.5) as i32, games as i32),
        } as u32;
        // Some bodies heal faster (hidden recovery; proposed, not confirmed:
        // up to about a quarter quicker or slower than average).
        if severity != Ev::MajorInjury {
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
                growth_multiplier *= 0.5;
                snaps_factor = 0.2;
                // Torn ACLs and Achilles tendons take explosiveness, more so with
                // age. How it heals (fix list 4.3): in real NFL studies 62-74% of
                // players are back on the field after an ACL repair, and those who
                // return start fewer games (81% before, 57% after). Proposed, not
                // confirmed: 55% heal fully (the rehab loss comes back next year),
                // 30% keep a small lasting loss and 15% never get their burst back.
                let age_hit = injury_age_hit(age);
                let heal = rng.next();
                let (now, lasting) = if heal < 0.55 {
                    ((1.5, 5.0), (0.0, 0.0))
                } else if heal < 0.85 {
                    ((1.5, 5.0), (1.0, 3.0))
                } else {
                    events.add(Ev::LostBurst);
                    ((3.0, 7.0), (3.0, 7.0))
                };
                rehab = heal < 0.85;
                for key in Attr::ALL {
                    if !key.profile().injury_sensitive {
                        continue;
                    }
                    player.attributes.add(key, -(rng.range(now.0, now.1) * age_hit));
                    if lasting.1 > 0.0 {
                        player.ceilings.add(key, -(rng.range(lasting.0, lasting.1) * age_hit));
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
                late_loss.add(key, size * rng.range(2.0, 6.0));
            }
        }
    } else if rng.chance(stagnation_chance) {
        events.add(Ev::Stagnation);
        form *= 0.3;
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
            player.body.weight += rng.range(8.0, 25.0);
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
    if poor > 0.0 && rng.chance(poor * clamp(1.5 - 0.05 * personality.coachability, 0.5, 1.5)) {
        events.add(Ev::PoorCoaching);
        for key in Attr::ALL {
            if key.profile().group == AttributeGroup::Technical && weight(position, key) > 0.0 {
                late_loss.add(key, rng.range(1.0, 3.0));
            }
        }
    }

    // --- Dynamic potential -----------------------------------------------------
    // Ceilings move most in the mid-teens, settle through the early 20s and only
    // fall after that. Direction is driven by personality, coaching and snaps;
    // luck plays a smaller part.
    let plasticity = potential_plasticity(player.age);
    if plasticity > 0.0 {
        // A young pro sitting behind a veteran still practises against NFL players
        // with an NFL staff: unlike a buried college backup, his ceiling doesn't sag.
        let young_pro =
            ctx.level == Level::NFL && age <= 24.0 && matches!(pt, PlayingTime::Backup | PlayingTime::PracticeSquad);
        let drive = ((personality.work_ethic - 10.0) / 10.0) * 0.75
            + ((personality.coachability - 10.0) / 10.0) * 0.25 * ((ctx.coaching - 10.0) / 10.0 + 0.5)
            + ((ctx.coaching - 10.0) / 10.0) * 0.35
            + 0.25 * situation
            + (if young_pro { 0.0 } else { ctx.snaps.map_or(playing_time_potential(pt), snaps_potential) })
                * snaps_factor
            - setback
            + rng.normal(0.0, 0.9);
        let shared = drive * 1.7 * plasticity;
        let spurt = age <= 17.0 && rng.chance(0.09);
        if spurt {
            events.add(Ev::GrowthSpurt);
        }
        let mut delta = Attributes::filled(0.0);
        for key in Attr::ALL {
            let physical = key.profile().group == AttributeGroup::Physical;
            let mut d = shared + rng.normal(0.0, 1.2 * plasticity);
            if spurt && physical {
                d += rng.range(2.0, 6.0) * if key == Attr::Strength { 1.3 } else { 0.8 };
            }
            if stalled && d > 0.0 && physical {
                d = 0.0;
            }
            let current = player.ceilings.get(key);
            delta.set(key, clamp(current + d, MIN_RATING, MAX_RATING) - current);
        }
        // A potential shift can move at most one band per checkpoint: no overnight stars.
        let shift = overall_for(position, &delta);
        let scale = if shift.abs() > MAX_POTENTIAL_SHIFT { MAX_POTENTIAL_SHIFT / shift.abs() } else { 1.0 };
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
            player.ceilings.add(key, learned);
            player.attributes.add(key, learned * 0.8);
        }
    }
    if breakout_lift > 0.0 {
        for key in Attr::ALL {
            if weight(position, key) == 0.0 || (stalled && key.profile().group == AttributeGroup::Physical) {
                continue;
            }
            player.ceilings.set(key, (player.ceilings.get(key) + breakout_lift * rng.range(0.6, 1.2)).min(MAX_RATING));
        }
    }

    // --- Growth towards the ceiling --------------------------------------------
    let coaching_mult = 0.75 + ctx.coaching * 0.025; // 10 -> 1.0, 20 -> 1.25
    let coachability_lean = 0.6 + personality.coachability * 0.04; // how much of coaching sticks
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
    let physical_mult = 1.0 + 0.08 * clamp(ctx.physical, -1.0, 1.0);
    let mut settling = [false; ATTRIBUTE_COUNT];
    // The plan's focus (section 6b topic 5; proposed, not confirmed: the
    // share each other skill gives up so the year's total stays about even).
    let focus_take = clamp(0.6 + 0.02 * personality.coachability, 0.6, 1.0);
    let g = GrowthCtx {
        coaching_boost,
        snaps_mult,
        work_mult,
        form,
        growth_multiplier,
        morale_mult,
        physical_mult,
        hs,
        college,
        redshirt: pt == PlayingTime::Redshirt,
        focus: ctx.focus,
        focus_take,
    };
    for key in Attr::ALL {
        let profile = &ATTRIBUTE_PROFILES[key.index()];
        let age_factor = growth_age_factor(development_age, profile.growth_window);
        settling[key.index()] = age_factor <= 0.01;
        let gap = player.ceilings.get(key) - player.attributes.get(key);
        if gap <= 0.0 || age_factor <= 0.01 {
            continue;
        }
        let rate = g.rate(key, age_factor);
        let noise = rng.normal(0.0, 0.5);
        // Growth that lands through the year's training (choice 1): the
        // draw is still taken, so every later draw is the one it was.
        if ctx.defer
            && crate::training::defers(player.position, key)
            && !declines(player.position, player.longevity, key, age)
        {
            continue;
        }
        // Proportional growth plus a fixed step, so a well-developed player can
        // actually reach his ceiling instead of creeping towards it forever.
        // Even a monster year adds at most about a dozen points to one attribute.
        player.attributes.add(key, gap.min(gap * rate + FINISHING_STEP * rate).min(MAX_YEARLY_GAIN) + noise);
    }
    if ctx.defer {
        let grade = player.train.grade;
        player.train =
            crate::training::Train::open(&g, player.age, development_age, player.position, player.train.load);
        player.train.grade = grade;
    }

    for key in Attr::ALL {
        player.attributes.add(key, -(late_loss.get(key)));
    }

    // --- Ageing ----------------------------------------------------------------
    let decline_resistance = 1.15 - personality.work_ethic * 0.015; // 10 -> 1.0, 20 -> 0.85
    let pos_offset = position.profile().aging_offset;
    // Years of heavy hits wear a body down faster; a rare body (high longevity) shrugs off more of it.
    let wear_mult = 1.0 + 0.06 * (player.wear - 3.0).max(0.0) * clamp(1.0 - 0.15 * player.longevity, 0.5, 1.5);
    for key in Attr::ALL {
        let profile = key.profile();
        let physical = profile.group == AttributeGroup::Physical;
        let onset = profile.decline_age + player.longevity + if physical { pos_offset.min(1.5) } else { pos_offset };
        let years_past = age + 1.0 - onset;
        if years_past <= 0.0 {
            continue;
        }
        settling[key.index()] = true;
        let loss = profile.decline_rate
            * years_past
            * decline_resistance
            * rng.log_normal(0.3)
            * if physical { wear_mult } else { 1.0 };
        player.attributes.add(key, -(loss));
        player.ceilings.add(key, -(loss));
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
            }
        }
    }

    // --- Tidy up ---------------------------------------------------------------
    for i in 0..ATTRIBUTE_COUNT {
        let (mut attr, mut ceil) = (player.attributes.at(i), player.ceilings.at(i));
        // Once an attribute has stopped growing or started to decline, the ceiling
        // above it is out of reach: veterans don't keep upside they can't use.
        // (A healing injury keeps its room: the rehab loss comes back next year.)
        if settling[i] && ceil > attr && !(rehab && Attr::ALL[i].profile().injury_sensitive) {
            ceil = attr + (ceil - attr) / 2.0;
        }
        ceil = clamp(ceil, MIN_RATING, MAX_RATING);
        attr = clamp(attr, MIN_RATING, MAX_RATING);
        // Over-performing a ceiling means the ceiling was wrong, not the player.
        if attr > ceil {
            ceil = attr;
        }
        player.attributes.set_at(i, attr);
        player.ceilings.set_at(i, ceil);
    }

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
        major_injury: events.has(Ev::MajorInjury),
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
            Some(clamp((overall_before + overall_after) / 2.0 + performance, MIN_RATING, MAX_RATING)).into()
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
    });
    player.age += 1;
    SeasonReport { overall_before, overall_after, potential_before, potential_after, events, games_missed }
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
