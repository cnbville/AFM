//! Players, their hidden traits and season history, and prospect generation.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::attributes::{ATTRIBUTE_PROFILES, Attr, Attributes, MAX_RATING, MIN_RATING};
use crate::positions::{Position, elite_shape_overall, elite_shape_value, overall_for};
use crate::rng::{Rng, clamp, round_half_up};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Level {
    HS,
    FBS,
    FCS,
    SmallCollege,
    NFL,
}

impl Level {
    pub fn is_college(self) -> bool {
        matches!(self, Level::FBS | Level::FCS | Level::SmallCollege)
    }
    pub fn name(self) -> &'static str {
        match self {
            Level::HS => "HS",
            Level::FBS => "FBS",
            Level::FCS => "FCS",
            Level::SmallCollege => "SmallCollege",
            Level::NFL => "NFL",
        }
    }
}

/// Hidden personality traits on Football Manager's 1-20 scale.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Personality {
    /// Effort in training: speeds growth, slows decline, pushes potential up.
    pub work_ethic: f64,
    /// How well instruction sticks: amplifies good coaching and protects against bad.
    pub coachability: f64,
    /// Drive to beat the man across from him: helps breakouts and protects against stagnation.
    pub competitiveness: f64,
    /// Love of the game and durability of mind: some keep playing through the grind, others walk away early.
    pub love_of_game: f64,
    /// Section 6 topic 1: the traits read off the field, one byte each.
    pub traits: Traits,
}

/// Section 6 topic 1 (`docs/design/people.md`): the hidden traits beside
/// the four above, 1-20 each, stored as one byte. Drawn at 14 with real
/// correlations (Big Five meta-analyses, about 0.2-0.4; the sizes are
/// proposed, not confirmed) and maturing with age and events.
#[derive(Clone, Copy, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Traits {
    /// Pushes for more: a bigger stage, a starting job, the big contract.
    pub ambition: u8,
    /// Stays with his team, coach, school and town.
    pub loyalty: u8,
    /// How much money counts against everything else.
    pub greed: u8,
    /// Off-field anger: rows with coaches, public fights, firing his agent.
    pub temper: u8,
    /// Needs the ball, the spotlight and the credit; swagger too.
    pub ego: u8,
    /// Others follow him; he steadies a room.
    pub leadership: u8,
    /// Judgment off the field: curfews, weight, rehab, study at home.
    pub professionalism: u8,
    /// Handles a new city, scheme, coach, level or culture.
    pub adaptability: u8,
    /// Aura (F15): charisma, camera presence and style. Hidden; with fame
    /// it decides ads, brand NIL deals, endorsements and follower growth
    /// (`world::fame`). Not in `Trait::ALL`, so no trait list shows it.
    #[serde(default)]
    pub aura: u8,
}

/// One of the traits, as a number for the code that reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Trait {
    Ambition,
    Loyalty,
    Greed,
    Temper,
    Ego,
    Leadership,
    Professionalism,
    Adaptability,
}

impl Trait {
    pub const ALL: [Trait; 8] = [
        Trait::Ambition,
        Trait::Loyalty,
        Trait::Greed,
        Trait::Temper,
        Trait::Ego,
        Trait::Leadership,
        Trait::Professionalism,
        Trait::Adaptability,
    ];
}

impl Traits {
    pub fn get(&self, t: Trait) -> f64 {
        (match t {
            Trait::Ambition => self.ambition,
            Trait::Loyalty => self.loyalty,
            Trait::Greed => self.greed,
            Trait::Temper => self.temper,
            Trait::Ego => self.ego,
            Trait::Leadership => self.leadership,
            Trait::Professionalism => self.professionalism,
            Trait::Adaptability => self.adaptability,
        }) as f64
    }
    pub fn set(&mut self, t: Trait, v: f64) {
        let v = clamp(round_half_up(v), 1.0, 20.0) as u8;
        match t {
            Trait::Ambition => self.ambition = v,
            Trait::Loyalty => self.loyalty = v,
            Trait::Greed => self.greed = v,
            Trait::Temper => self.temper = v,
            Trait::Ego => self.ego = v,
            Trait::Leadership => self.leadership = v,
            Trait::Professionalism => self.professionalism = v,
            Trait::Adaptability => self.adaptability = v,
        }
    }
    pub fn shift(&mut self, t: Trait, by: f64) {
        let v = self.get(t) + by;
        self.set(t, v);
    }
    /// Centred on 0 (10 is average): -1 at 0, +1 at 20.
    pub fn z(&self, t: Trait) -> f64 {
        (self.get(t) - 10.0) / 10.0
    }

    /// The draw at 14, keyed by the player (no draw from his stream, so
    /// everything else about him stays as it was). Correlations (proposed,
    /// not confirmed): professionalism with work ethic (0.35); temper
    /// against loyalty (-0.25) and with ego (0.2); ego against
    /// coachability (-0.3) and loyalty (-0.2), with competitiveness (0.2);
    /// leadership with competitiveness and work ethic (0.2 each) and ego
    /// (0.15); adaptability with coachability (0.2); greed against love of
    /// the game (-0.2); ambition with competitiveness (0.3).
    pub fn draw(key: u64, base: &Personality) -> Traits {
        let n = |salt: u64| keyed_normal(key, salt);
        let zs = |v: f64| (v - 10.0) / 3.5;
        let (we, co, cp, love) =
            (zs(base.work_ethic), zs(base.coachability), zs(base.competitiveness), zs(base.love_of_game));
        let mix = |parts: &[(f64, f64)], salt: u64| {
            let r: f64 = parts.iter().map(|x| x.0 * x.0).sum::<f64>();
            let own = (1.0 - r).max(0.2).sqrt();
            let z = parts.iter().map(|x| x.0 * x.1).sum::<f64>() + own * n(salt);
            clamp(round_half_up(10.0 + 3.5 * z), 1.0, 20.0) as u8
        };
        let loyalty = mix(&[(0.15, love)], 2);
        let lz = zs(loyalty as f64);
        let ego = mix(&[(-0.3, co), (-0.2, lz), (0.2, cp)], 5);
        let ez = zs(ego as f64);
        let leadership = mix(&[(0.2, cp), (0.2, we), (0.15, ez)], 6);
        Traits {
            ambition: mix(&[(0.3, cp)], 1),
            loyalty,
            greed: mix(&[(-0.2, love)], 3),
            temper: mix(&[(-0.25, lz), (0.2, ez)], 4),
            ego,
            leadership,
            professionalism: mix(&[(0.35, we)], 7),
            adaptability: mix(&[(0.2, co)], 8),
            aura: Traits::draw_aura(key, ego, leadership),
        }
    }

    /// Aura at 14 (F15), keyed like the rest (salt 9): a small link to ego
    /// and leadership (proposed correlations 0.2 and 0.15; swagger and a
    /// presence people follow both read on camera), the rest his own.
    pub fn draw_aura(key: u64, ego: u8, leadership: u8) -> u8 {
        let zs = |v: u8| (v as f64 - 10.0) / 3.5;
        let parts = [(0.2, zs(ego)), (0.15, zs(leadership))];
        let r: f64 = parts.iter().map(|x| x.0 * x.0).sum();
        let z = parts.iter().map(|x| x.0 * x.1).sum::<f64>() + (1.0 - r).sqrt() * keyed_normal(key, 9);
        clamp(round_half_up(10.0 + 3.5 * z), 1.0, 20.0) as u8
    }
}

/// A normal draw from a key (order-independent, no stream consumed).
pub(crate) fn keyed_normal(a: u64, b: u64) -> f64 {
    let mut z = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_add(0x632B_E59B_D9B4_E019);
    let mut next = || {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = z;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((x ^ (x >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    };
    let u1 = next().max(1e-12);
    let u2 = next();
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// Practice squad players train all year without playing; an unsigned free
/// agent trains on his own while he waits for a call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PlayingTime {
    Starter,
    Rotation,
    Backup,
    Redshirt,
    PracticeSquad,
    Unsigned,
}

impl PlayingTime {
    pub fn name(self) -> &'static str {
        match self {
            PlayingTime::Starter => "starter",
            PlayingTime::Rotation => "rotation",
            PlayingTime::Backup => "backup",
            PlayingTime::Redshirt => "redshirt",
            PlayingTime::PracticeSquad => "practiceSquad",
            PlayingTime::Unsigned => "unsigned",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum DevelopmentEvent {
    GrowthSpurt,
    /// His body stopped developing: physical ceilings settle where they are and don't rise again.
    BodyStalled,
    Breakout,
    Stagnation,
    LostSeason,
    /// Suspended or academically ineligible for the season.
    Suspended,
    /// Dismissed from his college team.
    Dismissed,
    Slump,
    MinorInjury,
    ModerateInjury,
    MajorInjury,
    /// His first year at a new level: he played below his rating while he
    /// adjusted (the first-year dip; its size is the row's `adjustment`).
    FirstYear,
    /// A spring breakout (section 4, T4.1); its gain is the row's
    /// `spring_gain`. It comes in the offseason after the season, except an
    /// early enrollee's first college spring, which opens his first college
    /// row (F2; that part is the row's `spring_open`).
    SpringBreakout,
    /// A major injury that left a lasting loss: he never got all of his
    /// burst back (fix list 4.3).
    LostBurst,
    /// He put on bad weight and lost speed and quickness (fix list 4.4).
    WeightGain,
    /// A lost-drive year after a big NIL deal or a big contract (fix list 4.5).
    Complacent,
    /// Weak teaching or a scheme that doesn't fit him cost him ability, not
    /// only growth (fix list 4.6).
    PoorCoaching,
}

impl DevelopmentEvent {
    pub const ALL: [DevelopmentEvent; 17] = [
        DevelopmentEvent::GrowthSpurt,
        DevelopmentEvent::BodyStalled,
        DevelopmentEvent::Breakout,
        DevelopmentEvent::Stagnation,
        DevelopmentEvent::LostSeason,
        DevelopmentEvent::Suspended,
        DevelopmentEvent::Dismissed,
        DevelopmentEvent::Slump,
        DevelopmentEvent::MinorInjury,
        DevelopmentEvent::ModerateInjury,
        DevelopmentEvent::MajorInjury,
        DevelopmentEvent::FirstYear,
        DevelopmentEvent::SpringBreakout,
        DevelopmentEvent::LostBurst,
        DevelopmentEvent::WeightGain,
        DevelopmentEvent::Complacent,
        DevelopmentEvent::PoorCoaching,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DevelopmentEvent::GrowthSpurt => "growthSpurt",
            DevelopmentEvent::BodyStalled => "bodyStalled",
            DevelopmentEvent::Breakout => "breakout",
            DevelopmentEvent::Stagnation => "stagnation",
            DevelopmentEvent::LostSeason => "lostSeason",
            DevelopmentEvent::Suspended => "suspended",
            DevelopmentEvent::Dismissed => "dismissed",
            DevelopmentEvent::Slump => "slump",
            DevelopmentEvent::MinorInjury => "minorInjury",
            DevelopmentEvent::ModerateInjury => "moderateInjury",
            DevelopmentEvent::MajorInjury => "majorInjury",
            DevelopmentEvent::FirstYear => "firstYear",
            DevelopmentEvent::SpringBreakout => "springBreakout",
            DevelopmentEvent::LostBurst => "lostBurst",
            DevelopmentEvent::WeightGain => "weightGain",
            DevelopmentEvent::Complacent => "complacent",
            DevelopmentEvent::PoorCoaching => "poorCoaching",
        }
    }
}

/// A season's events, as a small bit set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Events(u32);

impl Events {
    #[inline]
    pub fn has(self, e: DevelopmentEvent) -> bool {
        self.0 & (1 << e as u32) != 0
    }
    #[inline]
    pub fn add(&mut self, e: DevelopmentEvent) {
        self.0 |= 1 << e as u32;
    }
    #[inline]
    pub fn remove(&mut self, e: DevelopmentEvent) {
        self.0 &= !(1 << e as u32);
    }
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
    pub fn iter(self) -> impl Iterator<Item = DevelopmentEvent> {
        DevelopmentEvent::ALL.into_iter().filter(move |e| self.has(*e))
    }
}

/// An optional rating-sized number in 8 bytes instead of 16 (section 8,
/// memory): "none" is a NaN, every real value is kept bit for bit. Written
/// and read as a plain `Option<f64>`, so world files keep their shape.
#[derive(Clone, Copy)]
pub struct OptF64(f64);

impl Default for OptF64 {
    fn default() -> Self {
        OptF64::NONE
    }
}

impl OptF64 {
    pub const NONE: OptF64 = OptF64(f64::NAN);

    #[inline]
    pub fn get(self) -> Option<f64> {
        let v = self.0;
        (!v.is_nan()).then_some(v)
    }
    #[inline]
    pub fn is_some(self) -> bool {
        !self.0.is_nan()
    }
    #[inline]
    pub fn is_none(self) -> bool {
        self.0.is_nan()
    }
    #[inline]
    pub fn unwrap_or(self, d: f64) -> f64 {
        self.get().unwrap_or(d)
    }
}

impl From<Option<f64>> for OptF64 {
    /// A NaN value (never a real rating) would read back as none.
    #[inline]
    fn from(v: Option<f64>) -> Self {
        OptF64(v.unwrap_or(f64::NAN))
    }
}

impl PartialEq for OptF64 {
    fn eq(&self, o: &Self) -> bool {
        self.get() == o.get()
    }
}

impl PartialEq<Option<f64>> for OptF64 {
    fn eq(&self, o: &Option<f64>) -> bool {
        self.get() == *o
    }
}

impl std::fmt::Debug for OptF64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

impl serde::Serialize for OptF64 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.get().serialize(s)
    }
}

impl<'de> serde::Deserialize<'de> for OptF64 {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Option::<f64>::deserialize(d).map(OptF64::from)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeasonRecord {
    pub age: u32,
    pub level: Level,
    /// Where he played that season.
    pub position: Position,
    pub playing_time: PlayingTime,
    /// Rating going into the season: what he played the season at, and what his role was picked from.
    pub overall_start: f64,
    /// Rating at the end of the year, after the season and the offseason that follows.
    pub overall: f64,
    /// True potential at the end of the year. Hidden: see `debug`.
    pub(crate) potential: f64,
    /// On-field level shown that season: his rating during the season plus form,
    /// slumps and breakouts. None for a season without games (a redshirt year or
    /// one lost to suspension or dismissal).
    pub performance: OptF64,
    /// 1-99 for the rating he played the season at, placed against his own
    /// position and then ranked nationally: high schoolers against everyone their
    /// age, college players against all FBS players, pros simply their overall
    /// (filled in by a cohort simulation).
    pub level_rating: Option<u8>,
    /// Where his rating at the end of the year ranks against everyone else's at the end of theirs.
    pub level_rating_end: Option<u8>,
    /// Quality of the coaching he had that season, 1-20.
    pub coaching: f64,
    pub events: Events,
    /// Games missed through injury.
    pub games_missed: u32,
    /// Size of his first-year dip after a step up (0 = ready at once, 1 = a very hard first year).
    pub adjustment: OptF64,
    /// Overall gained in spring breakouts on this row (section 4, T4.1):
    /// one in the offseason after the season is already in `overall`, so the
    /// next row starts where this one ends; an early enrollee's step before
    /// his first college season is in the row's rise (`spring_open`).
    pub spring_gain: OptF64,
    /// The part of `spring_gain` an early enrollee took in his first college
    /// spring, before this row's season (F2): the row starts before it, so
    /// the rating he played the season at is `overall_start` plus this.
    pub spring_open: OptF64,
    /// Pro team he spent the season with (index into the league).
    pub team: Option<usize>,
    /// Depth chart spot, e.g. "WR2", "practice squad" or "free agent".
    pub depth: Option<String>,
    /// How his pro surroundings helped (+) or hurt (-) him that season, -1 to 1.
    pub situation: OptF64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Player {
    pub id: u32,
    pub position: Position,
    pub age: u32,
    /// Current ability, visible on the field.
    pub attributes: Attributes,
    /// Hidden ceiling per attribute. Moves over time; never shown directly.
    pub(crate) ceilings: Attributes,
    /// Potential when the player was generated. Hidden: see `debug`.
    pub(crate) original_potential: f64,
    /// Years the player's growth window is shifted: negative = early bloomer, positive = late bloomer.
    pub bloom: f64,
    /// Years added to (or removed from) every decline age.
    pub longevity: f64,
    /// 1-20, higher means more injuries.
    pub injury_proneness: f64,
    /// Wear on his body from heavy workloads and major injuries; speeds physical decline.
    pub wear: f64,
    pub personality: Personality,
    /// The adult body he grows into: height, weight, arms, hands (note 8).
    pub body: crate::body::Body,
    /// Every position he has been placed at, with his age then (topic 5.2).
    pub position_history: Vec<(u32, Position)>,
    /// His depth-chart spot inside his position (fix list topic 3), and
    /// every spot he has ever been listed at, as bits of `Spot`.
    pub spot: crate::spots::Spot,
    pub spots_played: u32,
    pub history: Vec<SeasonRecord>,
    /// Hidden game-day traits for the match engine (section 3).
    pub game: GameTraits,
    /// How often he picks each option the play gives him
    /// (`docs/design/tendencies.md`). Hidden: staffs see scouted reads.
    pub tend: crate::tendencies::Tendencies,
    /// Section 6b: the year's growth still to land through training, and
    /// how much of each family's year has landed (`training::Train`).
    #[serde(default)]
    pub train: crate::training::Train,
    /// How far past his whole-year age he is today, in years (update 3,
    /// A16): the world sets it each day from his birthday, so his body grows
    /// by the date and the oldest in a class are the biggest. 0 outside a
    /// world.
    #[serde(default)]
    pub grow: f32,
}

/// Hidden game-day traits on Football Manager's 1-20 scale (section 3). The
/// game never shows them; scouts and staffs only get hints. Discipline,
/// toughness, motor, composure, long snapping and the kicker's short-or-long
/// lean were hidden here before 2026-10-18 and are visible attributes now.
/// They come from the player's own id stream, so adding them changed no
/// other draw.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameTraits {
    /// How often he plays to his ratings, game to game (form) and snap to snap.
    pub consistency: f64,
    /// Self-control under provocation: late hits, retaliation, taunting, ejections.
    pub temperament: f64,
    /// How fast his body comes back between games and from injury.
    pub recovery: f64,
}

impl GameTraits {
    pub fn for_id(id: u32) -> GameTraits {
        let mut r = Rng::new(id ^ 0x6A4D_E5A1);
        let mut t = || clamp(round_half_up(r.normal(10.0, 3.5)), 1.0, 20.0);
        let (consistency, temperament, recovery) = (t(), t(), t());
        GameTraits { consistency, temperament, recovery }
    }

    /// A year older (proposed, not confirmed): steadiness and self-control
    /// come with maturity into the mid-twenties; recovery slows from 30.
    pub fn age_one_year(&mut self, age: u32, rng: &mut Rng) {
        let a = age as f64;
        if a < 26.0 {
            self.consistency = clamp(self.consistency + rng.range(0.0, 0.25), 1.0, 20.0);
            self.temperament = clamp(self.temperament + rng.range(0.0, 0.3), 1.0, 20.0);
        }
        if a >= 30.0 {
            self.recovery = clamp(self.recovery - rng.range(0.1, 0.4), 1.0, 20.0);
        }
    }
}

impl Player {
    /// Current overall rating at the player's position.
    #[inline]
    pub fn overall(&self) -> f64 {
        overall_for(self.position, &self.attributes)
    }

    /// True potential: the overall the player would have if every attribute
    /// reached its current ceiling. Hidden from the user, who only sees scouted
    /// estimates; exposed outside the engine only through `debug`.
    #[inline]
    pub(crate) fn potential(&self) -> f64 {
        overall_for(self.position, &self.ceilings)
    }

    pub fn has_had(&self, e: DevelopmentEvent) -> bool {
        self.history.iter().any(|h| h.events.has(e))
    }
}

/// Talent at maturity for a random high school player. A normal core with a
/// stretched upper tail: most kids top out as decent high school players, a few
/// per thousand have a college ceiling, and a handful per national class have
/// an elite pro ceiling.
pub fn talent_from_z(z: f64) -> f64 {
    let tail = (z - 1.6).max(0.0);
    45.0 + 6.5 * z + 2.75 * tail * tail
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ProspectOptions {
    pub id: Option<u32>,
    pub position: Option<Position>,
    pub age: Option<u32>,
    /// Override the latent talent draw (standard normal z-score).
    pub talent_z: Option<f64>,
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

fn trait_1_20(rng: &mut Rng) -> f64 {
    clamp(round_half_up(rng.normal(10.0, 3.5)), 1.0, 20.0)
}

pub fn generate_prospect(rng: &mut Rng, opts: ProspectOptions) -> Player {
    let age = opts.age.unwrap_or(14);
    let z = match opts.talent_z {
        Some(z) => z,
        None => rng.normal(0.0, 1.0),
    };
    let talent = clamp(talent_from_z(z), 20.0, 99.0);
    let id = opts.id.unwrap_or_else(|| NEXT_ID.fetch_add(1, Ordering::Relaxed));

    // Athlete first (topic 5.2): body, athletic traits and skill leanings come
    // before any position. A fixed position (tests, tools) builds the player
    // around that position's elite shape instead.
    let (position, ceilings, body, compete) = match opts.position {
        None => {
            let (ceilings, body, natural, compete) = crate::athlete::create(rng, talent);
            (natural, ceilings, body, compete)
        }
        Some(position) => {
            let scale = talent / elite_shape_overall(position);
            let mut ceilings = Attributes::default();
            for key in Attr::ALL {
                ceilings.set(
                    key,
                    clamp(elite_shape_value(position, key) * scale + rng.normal(0.0, 4.0), MIN_RATING, MAX_RATING),
                );
            }
            let potential = overall_for(position, &ceilings);
            // His build: how his strength and speed ceilings sit against his position's norm.
            let dev = |a: Attr| (ceilings.get(a) - elite_shape_value(position, a) * scale) / 4.0;
            let body = crate::body::adult_body(id, position, potential, dev(Attr::Strength), dev(Attr::Speed));
            (position, ceilings, body, rng.normal(0.0, 1.0))
        }
    };

    let bloom = clamp(rng.normal(0.0, 1.1), -2.5, 3.0);
    let mut attributes = Attributes::default();
    for key in Attr::ALL {
        let ceiling = ceilings.get(key);
        let profile = &ATTRIBUTE_PROFILES[key.index()];
        // Early bloomers arrive more physically mature; late bloomers less so.
        let maturity = clamp(
            profile.maturity_at_14 - 0.035 * bloom + 0.03 * (age as f64 - 14.0) + rng.normal(0.0, 0.05),
            0.2,
            0.98,
        );
        // Even the most gifted 14-year-old is still a boy: soften the top end.
        let raw = ceiling * maturity;
        attributes.set(key, clamp(if raw > 40.0 { 40.0 + (raw - 40.0) * 0.5 } else { raw }, MIN_RATING, ceiling));
    }
    // Talent is what he can become at the spot that suits him best.
    let original_potential = overall_for(position, &ceilings);
    let longevity = clamp(rng.normal(0.0, 1.3), -3.0, 3.5);
    let injury_proneness = trait_1_20(rng);
    let personality = Personality {
        work_ethic: trait_1_20(rng),
        coachability: trait_1_20(rng),
        // The drive to win he trains with is the will he plays with
        // (proposed, not confirmed: correlation 0.6).
        competitiveness: clamp(round_half_up(10.0 + 3.5 * (0.6 * compete + 0.8 * rng.normal(0.0, 1.0))), 1.0, 20.0),
        love_of_game: trait_1_20(rng),
        traits: Traits::default(),
    };
    let mut personality = personality;
    let key =
        (id as u64) << 20 ^ personality.work_ethic.to_bits() ^ attributes.get(Attr::Speed).to_bits().rotate_left(17);
    personality.traits = Traits::draw(key, &personality);
    let tend = crate::tendencies::Tendencies::at_14(id, &ceilings, body.weight, body.height, &personality);
    Player {
        id,
        position,
        age,
        attributes,
        ceilings,
        original_potential,
        bloom,
        longevity,
        injury_proneness,
        wear: 0.0,
        personality,
        body,
        position_history: vec![(age, position)],
        spot: crate::spots::spots_of(position)[0],
        spots_played: crate::spots::spots_of(position)[0].bit(),
        history: Vec::new(),
        game: GameTraits::for_id(id),
        tend,
        train: Default::default(),
        grow: 0.0,
    }
}
