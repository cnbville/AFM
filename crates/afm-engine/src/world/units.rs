//! Unit seasons (C27, C42): every team's season by side of the ball, ranked
//! at its level, with the coach in charge of each unit that season. One
//! shape for college (the program's engine totals), high school (the
//! school's season line) and the NFL (points from the league's book), so a
//! coach's page, the "coaches to watch" list and the coming data section
//! all read the same numbers.
//!
//! Only the engine keeps unit totals: a stand-in season has none and is
//! left out.

use super::World;
use super::coach::{Employer, Group, Role};
use super::data::Division;
use super::folk::Team;

/// One team's season, by side.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct UnitSeason {
    pub year: i32,
    pub games: u32,
    pub points_for: u32,
    pub points_against: u32,
    pub yards: f64,
    pub yards_allowed: f64,
    pub sacks: u32,
    pub sacks_allowed: u32,
    pub takeaways: u32,
    pub turnovers: u32,
}

impl UnitSeason {
    fn per(&self, x: f64) -> f64 {
        x / self.games.max(1) as f64
    }
    /// A stat a game, for one side (offense: higher is better unless
    /// noted; defense: lower points and yards are better).
    pub fn stat(&self, s: Stat) -> f64 {
        match s {
            Stat::Points => self.per(self.points_for as f64),
            Stat::PointsAllowed => self.per(self.points_against as f64),
            Stat::Yards => self.per(self.yards),
            Stat::YardsAllowed => self.per(self.yards_allowed),
            Stat::Sacks => self.per(self.sacks as f64),
            Stat::SacksAllowed => self.per(self.sacks_allowed as f64),
            Stat::Takeaways => self.per(self.takeaways as f64),
        }
    }
}

/// The unit stats a level keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Points,
    PointsAllowed,
    Yards,
    YardsAllowed,
    Sacks,
    SacksAllowed,
    Takeaways,
}

impl Stat {
    pub fn name(self) -> &'static str {
        match self {
            Stat::Points => "points a game",
            Stat::PointsAllowed => "points allowed a game",
            Stat::Yards => "yards a game",
            Stat::YardsAllowed => "yards allowed a game",
            Stat::Sacks => "sacks a game",
            Stat::SacksAllowed => "sacks allowed a game",
            Stat::Takeaways => "takeaways a game",
        }
    }
    /// Whether a bigger number is better.
    pub fn up(self) -> bool {
        matches!(self, Stat::Points | Stat::Yards | Stat::Sacks | Stat::Takeaways)
    }
    pub fn offense(self) -> bool {
        matches!(self, Stat::Points | Stat::Yards | Stat::SacksAllowed)
    }
    pub const ALL: [Stat; 7] = [
        Stat::Points,
        Stat::Yards,
        Stat::SacksAllowed,
        Stat::PointsAllowed,
        Stat::YardsAllowed,
        Stat::Sacks,
        Stat::Takeaways,
    ];
    /// Whether the level keeps it (the NFL's book has points only).
    pub fn kept(self, team: Team) -> bool {
        !matches!(team, Team::Nfl(_)) || matches!(self, Stat::Points | Stat::PointsAllowed)
    }
}

/// Who a team's season is ranked against: its division, its state's
/// high schools, the NFL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pool {
    College(Division),
    State(usize),
    Nfl,
}

impl Pool {
    pub fn of(w: &World, team: Team) -> Option<Pool> {
        match team {
            Team::College(p) => Some(Pool::College(w.programs[p].division)),
            Team::High(h) => Some(Pool::State(w.high_schools[h].state)),
            Team::Nfl(_) => Some(Pool::Nfl),
            Team::Other(_) => None,
        }
    }
    pub fn name(self, w: &World) -> String {
        match self {
            Pool::College(d) => d.name().to_string(),
            Pool::State(s) => format!("{} high schools", crate::world::folk::facts::state_name(&w.states[s].code)),
            Pool::Nfl => "the NFL".into(),
        }
    }
}

/// A team's season, if the engine kept it.
pub fn season(w: &World, team: Team, year: i32) -> Option<UnitSeason> {
    match team {
        Team::College(p) => w.programs[p].seasons.iter().find(|s| s.year == year && s.totals.games > 0).map(|s| {
            let t = &s.totals;
            UnitSeason {
                year,
                games: t.games,
                points_for: t.points_for.max(0) as u32,
                points_against: t.points_against.max(0) as u32,
                yards: t.yards,
                yards_allowed: t.yards_allowed,
                sacks: t.sacks,
                sacks_allowed: t.sacks_allowed,
                takeaways: t.takeaways,
                turnovers: t.turnovers,
            }
        }),
        Team::High(h) => w.high_schools[h].seasons.iter().find(|s| s.year == year && s.unit.games > 0).map(|s| {
            let u = &s.unit;
            UnitSeason {
                year,
                games: u.games as u32,
                points_for: s.points[0],
                points_against: s.points[1],
                yards: u.yards as f64,
                yards_allowed: u.yards_allowed as f64,
                sacks: u.sacks as u32,
                sacks_allowed: u.sacks_allowed as u32,
                takeaways: u.takeaways as u32,
                turnovers: u.turnovers as u32,
            }
        }),
        Team::Nfl(t) => w.nfl_units.iter().find(|x| x.0 == year && x.1 as usize == t).map(|x| UnitSeason {
            year,
            games: x.2 as u32,
            points_for: x.3 as u32,
            points_against: x.4 as u32,
            ..Default::default()
        }),
        Team::Other(_) => None,
    }
}

/// Every team's season in a pool.
pub fn pool_seasons(w: &World, pool: Pool, year: i32) -> Vec<(Team, UnitSeason)> {
    match pool {
        Pool::College(d) => (0..w.programs.len())
            .filter(|&p| w.programs[p].division == d)
            .filter_map(|p| season(w, Team::College(p), year).map(|s| (Team::College(p), s)))
            .collect(),
        Pool::State(st) => w.states[st]
            .schools
            .iter()
            .filter_map(|&h| season(w, Team::High(h), year).map(|s| (Team::High(h), s)))
            .collect(),
        Pool::Nfl => {
            (0..w.nfl.len()).filter_map(|t| season(w, Team::Nfl(t), year).map(|s| (Team::Nfl(t), s))).collect()
        }
    }
}

/// A team's rank in its pool for a stat (1 best) and how many teams.
pub fn rank(w: &World, team: Team, year: i32, stat: Stat) -> Option<(usize, usize)> {
    let pool = Pool::of(w, team)?;
    let all = pool_seasons(w, pool, year);
    let mine = all.iter().find(|x| x.0 == team)?.1.stat(stat);
    let better = all.iter().filter(|x| if stat.up() { x.1.stat(stat) > mine } else { x.1.stat(stat) < mine }).count();
    Some((better + 1, all.len()))
}

/// The coach in charge of a unit in a season: a role's holder that year,
/// from his stints (the job he holds counts to this season).
pub fn in_charge(w: &World, team: Team, year: i32, role: Role) -> Option<usize> {
    let emp = team.employer();
    w.coaches.iter().position(|c| {
        c.job.is_some_and(|j| j.employer == emp && j.role == role && j.since <= year)
            || c.history.iter().any(|s| s.employer == emp && s.role == role && s.from <= year && year <= s.to)
    })
}

/// Who held each job in a season, gathered once for a whole list: the man
/// whose stint covers the season first (one hired after it, in that
/// year's carousel, didn't coach it), then the holder now.
pub struct Charge(std::collections::HashMap<(Employer, Role), usize>);

impl Charge {
    pub fn new(w: &World, year: i32) -> Charge {
        let mut m = std::collections::HashMap::new();
        for (i, c) in w.coaches.iter().enumerate() {
            for s in c.history.iter().filter(|s| s.from <= year && year <= s.to) {
                m.entry((s.employer, s.role)).or_insert(i);
            }
        }
        for (i, c) in w.coaches.iter().enumerate() {
            if let Some(j) = c.job
                && j.since <= year
            {
                m.entry((j.employer, j.role)).or_insert(i);
            }
        }
        Charge(m)
    }

    pub fn get(&self, team: Team, role: Role) -> Option<usize> {
        self.0.get(&(team.employer(), role)).copied()
    }
}

/// One season of a side a coordinator ran.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct SideSeason {
    pub year: i32,
    pub team: Team,
    pub offense: bool,
    pub season: UnitSeason,
    /// Rank in its pool on points (scored or allowed) and the pool's size.
    pub rank: Option<(usize, usize)>,
}

/// A coordinator's seasons (C27): each season he ran a side, its points a
/// game and its rank at his level, newest first, eight at most.
pub fn coordinator_record(w: &World, c: usize) -> Vec<SideSeason> {
    let co = &w.coaches[c];
    let mut spans: Vec<(Employer, Role, i32, i32)> =
        co.history.iter().map(|s| (s.employer, s.role, s.from, s.to)).collect();
    if let Some(j) = co.job {
        spans.push((j.employer, j.role, j.since, w.year));
    }
    let mut v = Vec::new();
    for (e, r, from, to) in spans {
        let off = match r {
            Role::OffensiveCoordinator => true,
            Role::DefensiveCoordinator => false,
            _ => continue,
        };
        let team = Team::from_employer(e);
        for y in from.max(to - 7)..=to {
            if let Some(s) = season(w, team, y) {
                let stat = if off { Stat::Points } else { Stat::PointsAllowed };
                v.push(SideSeason { year: y, team, offense: off, season: s, rank: rank(w, team, y, stat) });
            }
        }
    }
    v.sort_by_key(|x| std::cmp::Reverse(x.year));
    v.truncate(8);
    v
}

/// A unit that stood out (C42): a team's side well above its level's
/// average, and who ran it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Standout {
    pub team: Team,
    pub year: i32,
    pub stat: &'static str,
    pub value: f64,
    pub average: f64,
    /// Standard deviations better than the level.
    pub z: f64,
    pub rank: usize,
    pub of: usize,
    /// The coaches in charge: (coach, role words).
    pub coaches: Vec<(usize, String)>,
}

/// Units well above their level's average in a season (C42): each pool's
/// teams by each stat it keeps, z of 1.5 or better, with the coaches in
/// charge (the head coach, the coordinator on that side, and the position
/// coach whose room drives the stat). Players' growth comes from the
/// coaches' unit records (`growth_standouts`).
pub fn standouts(w: &World, pools: &[Pool], year: i32, min_z: f64) -> Vec<Standout> {
    let charge = Charge::new(w, year);
    let mut out = Vec::new();
    for &pool in pools {
        let all = pool_seasons(w, pool, year);
        if all.len() < 8 {
            continue;
        }
        for stat in Stat::ALL {
            if matches!(pool, Pool::Nfl) && !matches!(stat, Stat::Points | Stat::PointsAllowed) {
                continue;
            }
            let xs: Vec<f64> = all.iter().map(|x| x.1.stat(stat)).collect();
            let n = xs.len() as f64;
            let mean = xs.iter().sum::<f64>() / n;
            let sd = (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n).sqrt();
            if sd <= 1e-9 {
                continue;
            }
            let mut ranked: Vec<(usize, f64)> = xs.iter().copied().enumerate().collect();
            ranked.sort_by(|a, b| if stat.up() { b.1.total_cmp(&a.1) } else { a.1.total_cmp(&b.1) });
            for (r, &(k, x)) in ranked.iter().enumerate() {
                let z = if stat.up() { (x - mean) / sd } else { (mean - x) / sd };
                if z < min_z {
                    break;
                }
                let team = all[k].0;
                out.push(Standout {
                    team,
                    year,
                    stat: stat.name(),
                    value: x,
                    average: mean,
                    z,
                    rank: r + 1,
                    of: xs.len(),
                    coaches: unit_coaches(&charge, team, stat),
                });
            }
        }
    }
    out.sort_by(|a, b| b.z.total_cmp(&a.z));
    out
}

/// The coaches behind a stat that season.
fn unit_coaches(charge: &Charge, team: Team, stat: Stat) -> Vec<(usize, String)> {
    let mut v = Vec::new();
    let off = stat.offense();
    let room = match stat {
        Stat::Sacks => Some(Group::DL),
        Stat::SacksAllowed => Some(Group::OL),
        Stat::Takeaways => Some(Group::DB),
        Stat::Points | Stat::Yards => Some(Group::QB),
        Stat::PointsAllowed | Stat::YardsAllowed => None,
    };
    let coord = if off { Role::OffensiveCoordinator } else { Role::DefensiveCoordinator };
    for (role, words) in
        [(Role::HeadCoach, "head coach"), (coord, if off { "offensive coordinator" } else { "defensive coordinator" })]
    {
        if let Some(c) = charge.get(team, role) {
            v.push((c, words.to_string()));
        }
    }
    if let (Some(g), Team::College(_)) = (room, team)
        && let Some(c) = charge.get(team, Role::PositionCoach(g))
    {
        v.push((c, g.coach_title().to_string()));
    }
    v
}

/// Position coaches whose rooms grew well above the norm (C42, players'
/// growth): the visible unit record, at least two seasons of players.
pub fn growth_standouts(w: &World, min_read: f64) -> Vec<(usize, Group, f64)> {
    let mut v: Vec<(usize, Group, f64)> = w
        .coaches
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.retired)
        .filter_map(|(i, c)| match c.job.map(|j| j.role) {
            Some(Role::PositionCoach(g)) => {
                let u = &c.units[g as usize];
                (u.seasons >= 6.0 && u.growth_read() >= min_read).then_some((i, g, u.growth_read()))
            }
            _ => None,
        })
        .collect();
    v.sort_by(|a, b| b.2.total_cmp(&a.2));
    v
}

/// Books last NFL season's points by team (the league keeps them only for
/// the season at hand), at the year's turn.
pub(crate) fn book_nfl(w: &mut World) {
    let Some(s) = w.nfl_season.as_ref() else { return };
    let year = s.year;
    if w.nfl_units.iter().any(|x| x.0 == year) {
        return;
    }
    let n = w.nfl.len();
    let mut t: Vec<(u16, u16, u8)> = vec![(0, 0, 0); n];
    for (k, week) in s.weeks.iter().enumerate() {
        for (g, &(h, a)) in week.iter().enumerate() {
            let Some([x, y]) = s.slots.get(k).and_then(|r| r.get(g)).and_then(|x| x.score) else { continue };
            if h < n && a < n {
                t[h] = (t[h].0 + x as u16, t[h].1 + y as u16, t[h].2 + 1);
                t[a] = (t[a].0 + y as u16, t[a].1 + x as u16, t[a].2 + 1);
            }
        }
    }
    for (team, (pf, pa, g)) in t.into_iter().enumerate() {
        if g > 0 {
            w.nfl_units.push((year, team as u8, g, pf, pa));
        }
    }
    // Twenty seasons are enough for any coach's record on screen.
    w.nfl_units.retain(|x| x.0 > year - 20);
}
