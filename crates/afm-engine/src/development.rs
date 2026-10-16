//! One development checkpoint covers a full football year: the season plus the
//! offseason that follows. It is meant to run at the yearly potential review
//! (W1 of the AFM calendar), after which the player is a year older.

use crate::attributes::{
    ATTRIBUTE_COUNT, ATTRIBUTE_PROFILES, Attr, AttributeGroup, Attributes, MAX_RATING, MIN_RATING,
};
use crate::player::{DevelopmentEvent as Ev, Events, Level, Player, PlayingTime, SeasonRecord};
use crate::positions::{overall_for, weight};
use crate::rng::{Rng, clamp, round_half_up};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trouble {
    Suspended,
    Dismissed,
}

#[derive(Clone, Copy, Debug)]
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
}

impl SeasonContext {
    pub fn new(level: Level, coaching: f64, playing_time: PlayingTime) -> Self {
        SeasonContext { level, coaching, playing_time, trouble: None, adjustment: 0.0, situation: 0.0 }
    }
}

/// Potential is reported in bands of this many points (one half-star).
pub const POTENTIAL_BAND: f64 = 5.0;
/// Most the yearly drift may move a player's true potential in one checkpoint,
/// up or down. A major injury, a body that stops developing and a proven
/// breakout can move it further, and a ceiling also rises when a player
/// outgrows it.
pub const MAX_POTENTIAL_SHIFT: f64 = POTENTIAL_BAND;

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

#[derive(Clone, Copy, Debug)]
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
    let mut events = Events::default();
    let age = player.age as f64;
    let pt = ctx.playing_time;
    let no_games = matches!(pt, PlayingTime::Redshirt | PlayingTime::PracticeSquad | PlayingTime::Unsigned)
        || ctx.trouble.is_some();
    let dip = ctx.adjustment;
    let situation = ctx.situation;
    let mut games_missed = 0;

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
                // Torn ACLs and Achilles tendons take explosiveness, more so with age.
                let age_hit = if age >= 28.0 {
                    1.6
                } else if age >= 24.0 {
                    1.2
                } else {
                    1.0
                };
                for key in Attr::ALL {
                    if !key.profile().injury_sensitive {
                        continue;
                    }
                    player.attributes[key] -= rng.range(1.5, 5.0) * age_hit;
                    player.ceilings[key] -= rng.range(0.5, 4.0) * age_hit;
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
            player.ceilings[key] = player.ceilings[key].min(player.attributes[key] + rng.range(0.0, 2.0));
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
    let mut form = rng.log_normal(0.38) * (1.0 - 0.15 * dip) * (1.0 + 0.12 * situation * youth);
    let mut setback = 0.0;
    let mut performance = rng.normal(0.0, 2.0) - 7.0 * dip + 2.5 * situation * youth;
    let mut breakout_lift = 0.0;
    let n = player.history.len();
    let last = if n >= 1 { player.history.get(n - 1) } else { None };
    let before = if n >= 2 { player.history.get(n - 2) } else { None };
    let hot_last_year = match (last, before) {
        (Some(l), b) => l.events.has(Ev::Breakout) || b.is_some_and(|b| l.overall - b.overall >= 6.0),
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
    let lost_season_chance = if (17.0..=27.0).contains(&age) {
        (0.03 + (10.0 - personality.work_ethic) * 0.004 + (10.0 - personality.competitiveness) * 0.002).max(0.005)
    } else {
        0.0
    };
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
        form *= 0.4;
        setback = 1.8;
        performance -= rng.range(3.0, 7.0);
    } else if !no_games && rng.chance(slump_chance) {
        // Production falls hard; the ratings dip a little as confidence and timing go.
        events.add(Ev::Slump);
        form *= 0.6;
        performance -= rng.range(4.0, 9.0);
        for key in Attr::ALL {
            if key.profile().group != AttributeGroup::Physical {
                player.attributes[key] -= rng.range(0.5, 3.0);
            }
        }
    } else if rng.chance(stagnation_chance) {
        events.add(Ev::Stagnation);
        form *= 0.3;
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
            + (if young_pro { 0.0 } else { playing_time_potential(pt) }) * snaps_factor
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
            let current = player.ceilings[key];
            delta[key] = clamp(current + d, MIN_RATING, MAX_RATING) - current;
        }
        // A potential shift can move at most one band per checkpoint: no overnight stars.
        let shift = overall_for(position, &delta);
        let scale = if shift.abs() > MAX_POTENTIAL_SHIFT { MAX_POTENTIAL_SHIFT / shift.abs() } else { 1.0 };
        for i in 0..ATTRIBUTE_COUNT {
            player.ceilings.0[i] += delta.0[i] * scale;
        }
    }
    // Reading defenses, poise and route timing keep deepening with real snaps
    // well after the body has stopped growing: veterans earn new ceiling and
    // most of it shows up on the field straight away.
    if (21.0..=31.0).contains(&age) && ctx.trouble.is_none() {
        let snaps_share = match pt {
            PlayingTime::Starter => 1.0,
            PlayingTime::Rotation => 0.6,
            _ => 0.2,
        };
        for key in Attr::ALL {
            let profile = key.profile();
            if profile.experience < 0.6 {
                continue;
            }
            let learned = rng.range(0.0, 1.2) * profile.experience * snaps_share * snaps_factor;
            player.ceilings[key] += learned;
            player.attributes[key] += learned * 0.8;
        }
    }
    if breakout_lift > 0.0 {
        for key in Attr::ALL {
            if weight(position, key) == 0.0 || (stalled && key.profile().group == AttributeGroup::Physical) {
                continue;
            }
            player.ceilings[key] = (player.ceilings[key] + breakout_lift * rng.range(0.6, 1.2)).min(MAX_RATING);
        }
    }

    // --- Growth towards the ceiling --------------------------------------------
    let coaching_mult = 0.75 + ctx.coaching * 0.025; // 10 -> 1.0, 20 -> 1.25
    let coachability_lean = 0.6 + personality.coachability * 0.04; // how much of coaching sticks
    let work_mult = 0.75 + personality.work_ethic * 0.025;
    let snaps_mult = playing_time_growth(pt) * (0.7 + 0.3 * snaps_factor) * level_growth(ctx.level);
    let hs = ctx.level == Level::HS;
    let college = ctx.level.is_college();
    let development_age = age - player.bloom;
    // Coachability scales how far coaching moves him from average, good or bad.
    let coaching_boost = 1.0 + (coaching_mult - 1.0) * coachability_lean;
    let mut settling = [false; ATTRIBUTE_COUNT];
    for key in Attr::ALL {
        let profile = &ATTRIBUTE_PROFILES[key.index()];
        let age_factor = growth_age_factor(development_age, profile.growth_window);
        settling[key.index()] = age_factor <= 0.01;
        let gap = player.ceilings[key] - player.attributes[key];
        if gap <= 0.0 || age_factor <= 0.01 {
            continue;
        }
        let coaching = coaching_boost.powf(profile.coachability);
        let snaps = snaps_mult.powf(profile.experience);
        let redshirt_lift =
            if pt == PlayingTime::Redshirt && profile.group == AttributeGroup::Physical { 1.35 } else { 1.0 };
        // College staffs teach the technique and the reads high school never did, so the raw catch up there.
        let hs_lag = if hs {
            if key == Attr::Strength { HS_STRENGTH_GROWTH } else { hs_group_growth(profile.group) }
        } else if college && profile.group != AttributeGroup::Physical {
            COLLEGE_TEACHING
        } else {
            1.0
        };
        let rate = clamp(
            profile.growth_rate
                * age_factor
                * coaching
                * snaps
                * work_mult
                * form
                * growth_multiplier
                * redshirt_lift
                * hs_lag,
            0.0,
            0.5,
        );
        // Proportional growth plus a fixed step, so a well-developed player can
        // actually reach his ceiling instead of creeping towards it forever.
        // Even a monster year adds at most about a dozen points to one attribute.
        player.attributes[key] +=
            gap.min(gap * rate + FINISHING_STEP * rate).min(MAX_YEARLY_GAIN) + rng.normal(0.0, 0.5);
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
        player.attributes[key] -= loss;
        player.ceilings[key] -= loss;
    }

    // --- Tidy up ---------------------------------------------------------------
    for i in 0..ATTRIBUTE_COUNT {
        let (attr, ceil) = (&mut player.attributes.0[i], &mut player.ceilings.0[i]);
        // Once an attribute has stopped growing or started to decline, the ceiling
        // above it is out of reach: veterans don't keep upside they can't use.
        if settling[i] && *ceil > *attr {
            *ceil = *attr + (*ceil - *attr) / 2.0;
        }
        *ceil = clamp(*ceil, MIN_RATING, MAX_RATING);
        *attr = clamp(*attr, MIN_RATING, MAX_RATING);
        // Over-performing a ceiling means the ceiling was wrong, not the player.
        if *attr > *ceil {
            *ceil = *attr;
        }
    }

    let overall_after = player.overall();
    let potential_after = player.potential();
    // A big year on the field from a player whose rating still fell is a career year, not a breakout.
    if overall_after < overall_before {
        events.remove(Ev::Breakout);
    }
    // He improves (or fades) through the year, so he plays the season between his two ratings.
    player.history.push(SeasonRecord {
        position: player.position,
        age: player.age,
        level: ctx.level,
        playing_time: pt,
        overall_start: overall_before,
        overall: overall_after,
        potential: potential_after,
        performance: if no_games {
            None
        } else {
            Some(clamp((overall_before + overall_after) / 2.0 + performance, MIN_RATING, MAX_RATING))
        },
        level_rating: None,
        level_rating_end: None,
        coaching: ctx.coaching,
        events,
        games_missed,
        adjustment: if dip > 0.0 { Some(round_half_up(dip * 100.0) / 100.0) } else { None },
        team: None,
        depth: None,
        situation: None,
    });
    player.age += 1;
    SeasonReport { overall_before, overall_after, potential_before, potential_after, events, games_missed }
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
        let room = (player.ceilings[key] - player.attributes[key]).max(0.0);
        player.attributes[key] += room * rate * rng.range(0.5, 1.5);
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
        let room = (player.ceilings[key] - player.attributes[key]).max(0.0);
        player.attributes[key] += room * clamp(share * rng.range(0.7, 1.3), 0.0, 0.9);
    }
    player.overall() - before
}
