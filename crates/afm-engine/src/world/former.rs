//! Former players the world keeps (coaching update C1, C2, C10, C11, C13):
//! every man who played college football carries what he played and how
//! good he was, and the ones whose lives stay in football are kept when
//! they leave the game: coaches, camp and 7-on-7 men, trainers, scouts,
//! reporters and agents. New coaches come from them ("a college player who
//! washed out becomes a graduate assistant or a high school coach, an NFL
//! veteran becomes a position coach", `docs/design/people.md`), his old
//! coaches hire him, and the position he played starts high in his
//! teaching: an ex-pro's at the playing roads' locked strengths (C1), a
//! former college player's a little lower (C2).
//!
//! Coaches the world starts with played before its records begin: each
//! gets a past drawn from the real shares (C2's research: 92.7% of FBS
//! coaches since 2000 played college football, CMSAC 2022; 57% of FBS head
//! coaches played at FBS, FootballScoop; a quarter of NFL head coaches
//! since 2000 played in the NFL, ESPN). Sizes the research doesn't settle
//! are Claude's picks (proposed, not confirmed; logged in `docs/game.md`).

use serde::{Deserialize, Serialize};

use crate::calendar::Date;
use crate::positions::Position;
use crate::world::coach::{Coach, CoachTier, Group};
use crate::world::data::Division;
use crate::world::ethnicity::Ethnicity;
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

/// How good a player he was: a college backup or starter, an NFL
/// journeyman, starter, Pro Bowler or star.
pub const BACKUP: u8 = 0;
pub const STARTER: u8 = 1;
pub const JOURNEYMAN: u8 = 2;
pub const NFL_STARTER: u8 = 3;
pub const PRO_BOWLER: u8 = 4;
pub const STAR: u8 = 5;

/// What a coach (or a former player) played (C11): his position, how good
/// he was, his last college's level and the college, his last season, and
/// his record in the world's books when he has one (a player id).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Played {
    pub pos: Position,
    pub grade: u8,
    /// His last college's division (`Division as u8`).
    pub div: u8,
    pub college: Option<usize>,
    pub last: i32,
    #[serde(default)]
    pub former: Option<u32>,
}

impl Played {
    pub fn nfl(&self) -> bool {
        self.grade >= JOURNEYMAN
    }

    pub fn group(&self) -> Group {
        Group::of(self.pos)
    }

    /// Where the teaching of his own position starts (C1: an ex-pro at the
    /// playing roads' locked strengths, star 19, Pro Bowler 18, starter 17,
    /// journeyman 16; C2: the former college player road's 15 for an FBS or
    /// FCS starter, a point less for a backup or a smaller college's
    /// starter, two for a smaller college's backup, proposed, not confirmed).
    pub fn floor(&self) -> f64 {
        let big = self.div <= Division::FCS as u8;
        match (self.grade, big) {
            (STAR, _) => 19.0,
            (PRO_BOWLER, _) => 18.0,
            (NFL_STARTER, _) => 17.0,
            (JOURNEYMAN, _) => 16.0,
            (STARTER, true) => 15.0,
            (STARTER, false) | (BACKUP, true) => 14.0,
            _ => 13.0,
        }
    }

    /// His name from playing (C10): the roads' agreed values for the
    /// pros (star 17, Pro Bowler 14, starter 11, journeyman 8) and the
    /// college player (5, an FBS starter); smaller and backup careers a
    /// little less (proposed, not confirmed).
    pub fn fame(&self) -> f64 {
        let fbs = self.div == Division::FBS as u8;
        match self.grade {
            STAR => 17.0,
            PRO_BOWLER => 14.0,
            NFL_STARTER => 11.0,
            JOURNEYMAN => 8.0,
            STARTER if fbs => 5.0,
            STARTER => 4.0,
            _ if fbs => 3.0,
            _ => 2.5,
        }
    }

    /// "a Pro Bowl linebacker", "a starting safety at Iowa".
    pub fn words(&self, w: &World) -> String {
        let pos = self.pos.name();
        let at = self.college.and_then(|c| w.programs.get(c)).map(|p| format!(" at {}", w.program_name(p.id)));
        match self.grade {
            STAR => format!("a star {pos} in the NFL"),
            PRO_BOWLER => format!("a Pro Bowl {pos}"),
            NFL_STARTER => format!("an NFL starter at {pos}"),
            JOURNEYMAN => format!("an NFL journeyman {pos}"),
            STARTER => format!("a starting {pos}{}", at.unwrap_or_default()),
            _ => format!("a backup {pos}{}", at.unwrap_or_default()),
        }
    }
}

/// His life after football (C13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Life {
    /// Coaching: a graduate assistant, an analyst, a high school assistant,
    /// until a job the world keeps takes him (then `Former::coach`).
    Coaching,
    /// Runs a camp or a 7-on-7 program.
    Camp,
    /// A private trainer: quarterbacks, speed, the offseason work.
    Trainer,
    /// A scout for an NFL club.
    Scout,
    /// A reporter or a broadcaster.
    Reporter,
    /// An agent.
    Agent,
}

impl Life {
    pub const ALL: [Life; 6] = [Life::Coaching, Life::Camp, Life::Trainer, Life::Scout, Life::Reporter, Life::Agent];

    pub fn name(self) -> &'static str {
        match self {
            Life::Coaching => "Coaching",
            Life::Camp => "Runs a camp and a 7-on-7 team",
            Life::Trainer => "A private trainer",
            Life::Scout => "An NFL scout",
            Life::Reporter => "In the media",
            Life::Agent => "An agent",
        }
    }
}

/// A former player the world keeps.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Former {
    /// His player id (stable across the year-end book).
    pub id: u32,
    pub name: String,
    pub ethnicity: Ethnicity,
    pub born: Date,
    pub home_state: u8,
    pub hs: u32,
    pub played: Played,
    /// His colleges and years: (program, first season, last season).
    pub colleges: Vec<(u16, i16, i16)>,
    /// His NFL clubs and years.
    pub clubs: Vec<(u8, i16, i16)>,
    /// The head coaches and position coaches who coached him in college.
    pub coached_by: Vec<u32>,
    pub life: Life,
    /// His first year out of football.
    pub since: i16,
    /// The coach he became, once a job the world keeps took him.
    pub coach: Option<u32>,
    /// The state he works in.
    pub area: u8,
    /// Hidden, 1-20: his eye for a player, and how straight he talks.
    pub eye: u8,
    pub straight: u8,
    /// Hidden: his football IQ (1-99) and leadership (1-20) as a player,
    /// which a coach he becomes keeps in his scheme and motivation.
    pub iq: u8,
    pub lead: u8,
    /// His agent when he played (an index into the agents list).
    pub agent: Option<u16>,
}

impl Former {
    pub fn age(&self, year: i32) -> u32 {
        if self.born == crate::calendar::born::UNKNOWN { 22 } else { (year - self.born.year()).max(0) as u32 }
    }

    /// Whether he played at a college in a year.
    pub fn at_college(&self, prog: usize, a: i32, b: i32) -> bool {
        self.colleges.iter().any(|&(p, x, y)| p as usize == prog && (x as i32) <= b && (y as i32) >= a)
    }

    /// The seasons he overlapped with a stretch at an NFL club.
    pub fn at_club(&self, team: usize, a: i32, b: i32) -> bool {
        self.clubs.iter().any(|&(t, x, y)| t as usize == team && (x as i32) <= b && (y as i32) >= a)
    }

    /// What he does now, in words.
    pub fn now(&self, w: &World) -> String {
        if let Some(c) = self.coach.and_then(|c| w.coaches.get(c as usize)) {
            return match c.job {
                Some(j) => format!(
                    "{} at {}",
                    crate::world::front_office::market::role_title(j.role),
                    crate::world::front_office::market::employer_name(w, j.employer)
                ),
                None if c.retired => "Retired from coaching".into(),
                None => "Out of coaching".into(),
            };
        }
        match self.life {
            Life::Coaching if self.age(w.year) < 30 => "A graduate assistant".into(),
            Life::Coaching => {
                format!("A high school assistant in {}", w.states.get(self.area as usize).map_or("", |s| &s.code))
            }
            Life::Camp => format!("Runs a camp and a 7-on-7 team in {}", state_code(w, self.area)),
            Life::Trainer => format!("Trains players in {}", state_code(w, self.area)),
            l => l.name().into(),
        }
    }
}

fn state_code(w: &World, s: u8) -> &str {
    w.states.get(s as usize).map_or("", |s| &s.code)
}

/// The former players the world keeps, sorted by player id.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Formers {
    pub list: Vec<Former>,
}

impl Formers {
    pub fn get(&self, id: u32) -> Option<&Former> {
        self.list.binary_search_by_key(&id, |f| f.id).ok().map(|k| &self.list[k])
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Former> {
        self.list.binary_search_by_key(&id, |f| f.id).ok().map(move |k| &mut self.list[k])
    }
}

/// The share of coaches at a level who played college football (C2):
/// 92.7% of FBS coaches since 2000 (CMSAC 2022), about 90% of Division I
/// head coaches; the high school share has no study (proposed, not
/// confirmed: 60%).
pub fn college_share(tier: CoachTier) -> f64 {
    match tier {
        CoachTier::HighSchool => 0.6,
        CoachTier::SmallCollege => 0.9,
        CoachTier::Fcs => 0.92,
        CoachTier::Fbs => 0.93,
        CoachTier::Nfl => 0.95,
    }
}

/// How good a player he was, from his record.
fn grade_of(p: &crate::world::Person) -> u8 {
    if let Some(c) = p.pro.as_ref() {
        let played: Vec<&crate::world::pro::ProSeason> =
            c.seasons.iter().filter(|s| s.team.is_some() && s.games > 0).collect();
        if played.len() >= 2 {
            let bowls = played.iter().filter(|s| s.pro_bowl).count();
            let first = played.iter().filter(|s| s.all_pro == 1).count();
            let starts = played.iter().filter(|s| s.starts >= 8).count();
            return if bowls >= 4 || first >= 2 {
                STAR
            } else if bowls >= 1 {
                PRO_BOWLER
            } else if starts >= 3 {
                NFL_STARTER
            } else {
                JOURNEYMAN
            };
        }
    }
    // Two seasons' snaps as a regular (proposed, not confirmed: 1,200).
    if p.college_snaps >= 1200 { STARTER } else { BACKUP }
}

/// The yearly chances of each life after football for a former college
/// player and an NFL veteran (proposed, not confirmed), before his traits.
fn life_base(life: Life, nfl: bool) -> f64 {
    match (life, nfl) {
        (Life::Coaching, false) => 0.08,
        (Life::Coaching, true) => 0.14,
        (Life::Camp, false) => 0.008,
        (Life::Camp, true) => 0.03,
        (Life::Trainer, false) => 0.012,
        (Life::Trainer, true) => 0.03,
        (Life::Scout, false) => 0.006,
        (Life::Scout, true) => 0.015,
        (Life::Reporter, false) => 0.003,
        (Life::Reporter, true) => 0.025,
        (Life::Agent, false) => 0.002,
        (Life::Agent, true) => 0.004,
    }
}

/// Everyone leaving football this year who played in college: the ones
/// whose lives stay in football are kept (run by the year-end book before
/// the people list drops them).
pub(crate) fn intake(w: &mut World) {
    let year = w.year;
    let seed = w.options.seed as u64 ^ 0xF0_3E3;
    let mut add: Vec<Former> = Vec::new();
    for p in w.people.iter().filter(|p| p.stage == Stage::Done && !p.college_staff.is_empty()) {
        let id = p.player.id;
        if w.formers.get(id).is_some() {
            continue;
        }
        let grade = grade_of(p);
        let nfl = grade >= JOURNEYMAN;
        let t = &p.player.personality.traits;
        let z = |v: f64, m: f64, s: f64| ((v - m) / s).clamp(-2.5, 2.5);
        let iq = p.player.attributes.get(crate::attributes::Attr::FootballIq);
        let lead = z(t.leadership as f64, 10.0, 3.5);
        let love = z(p.player.personality.love_of_game, 10.0, 3.5);
        let smart = z(iq, 55.0, 12.0);
        // Who goes into it follows the person (people design, "Where staff
        // come from"): the leaders and the students of the game coach.
        let lean = |l: Life| -> f64 {
            let x = match l {
                Life::Coaching => 0.35 * lead + 0.25 * smart + 0.2 * love,
                Life::Camp => 0.3 * z(t.ambition as f64, 10.0, 3.5) + 0.3 * z(t.aura as f64, 10.0, 3.5),
                Life::Trainer => 0.35 * z(p.player.personality.work_ethic, 10.0, 3.5) + 0.15 * love,
                Life::Scout => 0.4 * smart + 0.15 * love,
                Life::Reporter => 0.5 * z(t.aura as f64, 10.0, 3.5) + 0.2 * z(t.ego as f64, 10.0, 3.5),
                Life::Agent => 0.3 * z(t.greed as f64, 10.0, 3.5) + 0.3 * z(t.professionalism as f64, 10.0, 3.5),
            };
            let skill = matches!(p.player.position, Position::QB | Position::WR | Position::RB | Position::CB);
            let pos = if matches!(l, Life::Camp | Life::Trainer) && skill { 1.5 } else { 1.0 };
            let star = if l == Life::Reporter && grade >= PRO_BOWLER { 2.5 } else { 1.0 };
            life_base(l, nfl) * (0.8 * x).exp() * pos * star
        };
        let u = key_uniform(seed ^ year as u64, id as u64);
        let mut acc = 0.0;
        let Some(life) = Life::ALL.iter().copied().find(|&l| {
            acc += lean(l);
            u < acc
        }) else {
            continue;
        };
        // His colleges and years, from the coaches he had each season.
        let mut colleges: Vec<(u16, i16, i16)> = Vec::new();
        let mut coached_by: Vec<u32> = Vec::new();
        for &(y, prog, hc, pc) in &p.college_staff {
            match colleges.iter_mut().find(|c| c.0 as usize == prog) {
                Some(c) => {
                    c.1 = c.1.min(y as i16);
                    c.2 = c.2.max(y as i16);
                }
                None => colleges.push((prog as u16, y as i16, y as i16)),
            }
            for c in [hc, pc].into_iter().flatten() {
                if !coached_by.contains(&(c as u32)) && coached_by.len() < 8 {
                    coached_by.push(c as u32);
                }
            }
        }
        let mut clubs: Vec<(u8, i16, i16)> = Vec::new();
        let mut last = colleges.iter().map(|c| c.2 as i32).max().unwrap_or(year);
        if let Some(c) = p.pro.as_ref() {
            for s in c.seasons.iter().filter(|s| s.games > 0) {
                let Some(t) = s.team else { continue };
                last = last.max(s.year);
                match clubs.iter_mut().find(|c| c.0 as usize == t) {
                    Some(c) => c.2 = c.2.max(s.year as i16),
                    None => clubs.push((t as u8, s.year as i16, s.year as i16)),
                }
            }
        }
        let last_college = p.path.last().copied().or(colleges.last().map(|c| c.0 as usize));
        let div = last_college.and_then(|c| w.programs.get(c)).map_or(Division::D3 as u8, |q| q.division as u8);
        let area = match life {
            Life::Camp | Life::Trainer | Life::Reporter
                if key_uniform(seed ^ 0xA2EA, id as u64) < 0.3 && last_college.is_some() =>
            {
                w.programs[last_college.unwrap()].state as u8
            }
            _ => p.home_state as u8,
        };
        let eye = (10.0 + 0.08 * (iq - 55.0) + 3.0 * crate::world::season::key_normal(seed ^ 0xE7E, id as u64))
            .clamp(1.0, 20.0)
            .round() as u8;
        let straight = (10.0 + 0.3 * (t.professionalism as f64 - 10.0) - 0.2 * (t.ego as f64 - 10.0)
            + 3.0 * crate::world::season::key_normal(seed ^ 0x57A1, id as u64))
        .clamp(1.0, 20.0)
        .round() as u8;
        add.push(Former {
            id,
            name: p.name.clone(),
            ethnicity: p.ethnicity,
            born: p.born,
            home_state: p.home_state as u8,
            hs: p.hs as u32,
            played: Played { pos: p.player.position, grade, div, college: last_college, last, former: Some(id) },
            colleges,
            clubs,
            coached_by,
            life,
            since: year as i16,
            coach: None,
            area,
            eye,
            straight,
            iq: iq.round().clamp(1.0, 99.0) as u8,
            lead: t.leadership,
            agent: p.pro.as_ref().and_then(|c| c.agent).map(|a| a as u16),
        });
    }
    w.formers.list.extend(add);
    w.formers.list.sort_by_key(|f| f.id);
}

/// The year's turnover (proposed, not confirmed): most young coaches never
/// get a job the world keeps, and about one in twelve a year leaves
/// coaching; one in thirty leaves the other lives; everyone is gone by 70.
pub(crate) fn year_end(w: &mut World) {
    let year = w.year;
    let seed = w.options.seed as u64 ^ 0xF0_3E4 ^ (year as u64) << 20;
    let coaches = &w.coaches;
    w.formers.list.retain(|f| {
        if let Some(c) = f.coach {
            // A coach is the world's own now: kept while he is (his link).
            return coaches.get(c as usize).is_some_and(|c| !c.retired || f.age(year) < 70);
        }
        let leave = if f.life == Life::Coaching { 0.08 } else { 0.033 };
        f.age(year) < 70 && key_uniform(seed, f.id as u64) >= leave
    });
}

/// What a new coach should be (`new_coach`): the level, his age, the
/// job's state, the head coach hiring him, the group he'll coach, and
/// whether the search wants a former pro.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Want {
    pub state: Option<usize>,
    pub hc: Option<usize>,
    pub group: Option<Group>,
    pub nfl: Option<bool>,
}

/// A former player in coaching who fits the job (C11), weighted: his old
/// coaches hire him first, then men from the state, then men who played
/// the group; an NFL job leans to former pros.
pub(crate) fn draw(w: &mut World, tier: CoachTier, age: u32, want: Want) -> Option<usize> {
    let year = w.year;
    let mut total = 0.0;
    let mut pool: Vec<(usize, f64)> = Vec::new();
    for (k, f) in w.formers.list.iter().enumerate() {
        if f.life != Life::Coaching || f.coach.is_some() {
            continue;
        }
        let a = f.age(year);
        if a + 4 < age || a > age + 4 || a < 22 {
            continue;
        }
        let nfl = f.played.nfl();
        if want.nfl == Some(true) && !nfl {
            continue;
        }
        let mut wt = 1.0;
        if want.hc.is_some_and(|h| f.coached_by.contains(&(h as u32))) {
            wt *= 4.0;
        }
        if want.state.is_some_and(|s| s == f.area as usize) {
            wt *= 2.5;
        }
        if want.group.is_some_and(|g| g == f.played.group()) {
            wt *= 2.0;
        }
        match tier {
            CoachTier::Nfl if nfl => wt *= 6.0,
            CoachTier::Nfl | CoachTier::Fbs if f.played.div == Division::FBS as u8 => wt *= 2.0,
            CoachTier::HighSchool if f.played.div != Division::FBS as u8 => wt *= 1.5,
            _ => {}
        }
        total += wt;
        pool.push((k, wt));
    }
    if pool.is_empty() {
        return None;
    }
    let mut x = w.rng.next() * total;
    for (k, wt) in pool {
        if x < wt {
            return Some(k);
        }
        x -= wt;
    }
    None
}

/// The playing past of a coach the world makes up (the coaches it starts
/// with, and new ones when no former player fits): drawn from the real
/// shares (C2), on his own keyed stream so the world's stream is untouched.
/// `college`: whether he played in college (an ex-pro always did).
pub(crate) fn made_up(w: &World, c: &Coach, tier: CoachTier, college: bool) -> Option<Played> {
    let key = w.options.seed as u64 ^ 0x9A57 ^ (c.id as u64) << 8;
    let u = |n: u64| key_uniform(key, n);
    let born = if c.born == crate::calendar::born::UNKNOWN { w.year - c.age as i32 } else { c.born.year() };
    // An ex-pro's career (proposed, not confirmed: half journeymen, three
    // in ten starters, one in seven a Pro Bowler, one in twenty a star).
    let grade = if c.ex_pro {
        match u(1) {
            x if x < 0.5 => JOURNEYMAN,
            x if x < 0.8 => NFL_STARTER,
            x if x < 0.95 => PRO_BOWLER,
            _ => STAR,
        }
    } else if college {
        if u(2) < 0.55 { STARTER } else { BACKUP }
    } else {
        return None;
    };
    // Where he played (FootballScoop: 57% of FBS head coaches played at
    // FBS; the rest of the split is proposed, not confirmed).
    let shares: [f64; 3] = match tier {
        CoachTier::Nfl | CoachTier::Fbs => [0.57, 0.2, 0.23],
        CoachTier::Fcs => [0.25, 0.45, 0.3],
        CoachTier::SmallCollege => [0.1, 0.2, 0.7],
        CoachTier::HighSchool => [0.15, 0.2, 0.65],
    };
    let div = if grade >= JOURNEYMAN || u(3) < shares[0] {
        if grade >= JOURNEYMAN && u(3) > 0.9 { Division::FCS } else { Division::FBS }
    } else if u(3) < shares[0] + shares[1] {
        Division::FCS
    } else if u(4) < 0.5 {
        Division::D2
    } else {
        Division::D3
    };
    let college = {
        let list: Vec<usize> = w.programs.iter().filter(|p| p.division == div).map(|p| p.id).collect();
        (!list.is_empty()).then(|| list[(u(5) * list.len() as f64) as usize % list.len()])
    };
    // Most coaches coach what they played (proposed, not confirmed: three
    // in four the group they came up in, else a position on that side).
    let side: Vec<Position> = Position::ALL
        .iter()
        .copied()
        .filter(
            |&p| {
                if u(6) < 0.75 { Group::of(p) == c.came_up } else { Group::of(p).offense() == c.came_up.offense() }
            },
        )
        .filter(|&p| !matches!(p, Position::LS))
        .collect();
    let pos = if side.is_empty() { Position::LB } else { side[(u(7) * side.len() as f64) as usize % side.len()] };
    let years = match grade {
        STAR => 11,
        PRO_BOWLER => 9,
        NFL_STARTER => 7,
        JOURNEYMAN => 4,
        _ => 0,
    };
    Some(Played { pos, grade, div: div as u8, college, last: born + 22 + years, former: None })
}

/// What a coach's playing past does to him: the position he played starts
/// high in his teaching (C1, C2), it's the group he came up coaching, the
/// side of the ball he knows best, and his name starts from it (C10).
pub(crate) fn apply(c: &mut Coach, p: Played, year: i32) {
    let g = p.group();
    if g != Group::Specialists {
        // Coaches come up on the side they played: the side bonus of his
        // generation (+1 on his side, -1 on the other) follows it.
        if c.came_up.offense() != g.offense() {
            for h in Group::ALL {
                let d = if h.offense() == g.offense() { 2.0 } else { -2.0 };
                c.ability.teaching[h as usize] = (c.ability.teaching[h as usize] + d).clamp(1.0, 20.0);
            }
        }
        c.came_up = g;
    }
    let t = &mut c.ability.teaching[g as usize];
    *t = t.max(p.floor());
    c.ex_pro = p.nfl();
    // His name from playing fades slowly (proposed, not confirmed: 1% a
    // season since he last played).
    c.fame = p.fame() * 0.99f64.powi((year - p.last).max(0));
    c.renown = c.fame.max(0.5 * c.reputation);
    c.played = Some(p);
}

/// A coach from a former player (C11): the world's coach generation at the
/// job's level, his own name, birth and past, his football IQ in his scheme
/// and his leadership in his motivation (proposed, not confirmed: up to
/// about a point and a half each).
pub(crate) fn to_coach(w: &mut World, k: usize, tier: CoachTier) -> usize {
    let id = w.coaches.len();
    let f = w.formers.list[k].clone();
    let age = f.age(w.year);
    let mut c = crate::world::coach::generate_coach_as(
        id,
        f.name.clone(),
        f.ethnicity,
        tier,
        age,
        Some(f.played.nfl()),
        &mut w.rng,
    );
    c.born = f.born;
    c.ability.scheme = (c.ability.scheme + ((f.iq as f64 - 55.0) / 12.0).clamp(-1.5, 1.5)).clamp(1.0, 20.0);
    c.ability.motivation = (c.ability.motivation + ((f.lead as f64 - 10.0) / 3.5).clamp(-1.5, 1.5)).clamp(1.0, 20.0);
    c.home_state = Some(f.home_state as usize);
    apply(&mut c, f.played, w.year);
    w.coaches.push(c);
    w.formers.list[k].coach = Some(id as u32);
    id
}
