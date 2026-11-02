//! U-d: each state's real high school playoff format and private-school
//! rules (`data/real-world/hs_playoff_formats.csv`, researched in
//! `docs/research/hs-playoff-formats.md`).
//!
//! A state's format says how its schools are classed (how many classes, how
//! big, and whether private schools play apart, count their enrollment
//! higher or move up after winning), who qualifies for each bracket (the
//! best by the state's own rating, district champions first, the top few of
//! each district, or everyone), how big each bracket is, and whether an
//! Open Division takes the strongest teams first. A bracket that isn't a
//! power of two gives its top seeds byes, and a small bracket starts later
//! so every final lands on the state's finals day.

use std::sync::LazyLock;

use super::World;
use super::data::split_csv_line;
use super::season::TeamResult;

const FORMATS_CSV: &str = include_str!("../../../../data/real-world/hs_playoff_formats.csv");

/// An empty bracket slot (a bye, or a team already out).
pub const NONE: u32 = u32::MAX;

/// How a state treats its private schools.
#[derive(Clone, Debug, PartialEq)]
pub enum PrivateRule {
    None,
    /// Louisiana: each class splits into a Non-Select (public) and a Select
    /// (private) bracket; the regular season stays mixed.
    Select,
    /// New Jersey: every private school plays in these non-public brackets
    /// (largest first), whatever its group in the season.
    NonPublic(Vec<String>),
    /// Georgia: private schools in this class and below play one bracket.
    Private {
        from: u32,
        label: String,
    },
    /// Illinois: a private school's enrollment counts this much more when
    /// the classes are formed.
    Multiplier(f64),
    /// Connecticut: a private school that reached a state semifinal in the
    /// last two years plays a class up.
    PlayUp,
    /// Wisconsin: the tournament performance factor moves a school that won
    /// a lot in the playoffs over the last three years up a division.
    Tpf,
}

/// How a state's brackets are formed.
#[derive(Clone, Debug, PartialEq)]
pub enum Brackets {
    /// One per class.
    ByClass,
    /// California's competitive equity: the qualifiers are placed into
    /// brackets by strength, not size.
    Equity(Vec<String>),
    /// Illinois and Michigan: the first N classes' qualifiers are pooled,
    /// then split into N brackets by enrollment after selection.
    Enrollment(usize),
}

/// Who gets into a bracket.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Qualify {
    /// The best by the state's rating.
    Top,
    /// District (league, region) champions first, then the best of the rest.
    ChampTop,
    /// The top N of each district by district record.
    District(u32),
    /// Every school enters.
    All,
}

/// How a state rates its teams for selection and seeding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rating {
    WinPct,
    /// Wins first, then opponents' wins (Illinois, Wisconsin).
    Wins,
    /// Shares of own, opponents' and opponents' opponents' winning percentage.
    Rpi(f64, f64, f64),
    /// Points per win, more for beating teams that win (Ohio's Harbin
    /// points, New Jersey's power points, West Virginia, Michigan's playoff
    /// average).
    Power,
    /// A computer ranking (MaxPreps, CalPreps): record, schedule and how
    /// well the team plays.
    MaxPreps,
}

/// An Open Division taken before the class brackets.
#[derive(Clone, Debug, PartialEq)]
pub struct Open {
    pub teams: u32,
    /// The class it draws from, or every class.
    pub from: Option<u32>,
    /// Two pools of four play a round robin, then semifinals (Florida).
    pub pools: bool,
}

/// One state's format.
#[derive(Clone, Debug)]
pub struct Format {
    pub code: String,
    /// Season classes, biggest first.
    pub classes: Vec<String>,
    /// Relative class sizes (empty: equal).
    pub sizes: Vec<f64>,
    /// Classes only private schools play in (season and playoffs).
    pub private_classes: Vec<String>,
    pub private: PrivateRule,
    pub brackets: Brackets,
    /// Per bracket, the last repeating.
    pub qualify: Vec<Qualify>,
    /// Real full-size field per bracket, the last repeating.
    pub fields: Vec<u32>,
    pub rating: Rating,
    pub open: Option<Open>,
}

fn list(cell: &str) -> Vec<String> {
    cell.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn parse_qualify(s: &str) -> Qualify {
    match s {
        "top" => Qualify::Top,
        "champ+top" => Qualify::ChampTop,
        "all" => Qualify::All,
        _ => s.strip_prefix("district:").and_then(|n| n.parse().ok()).map_or(Qualify::ChampTop, Qualify::District),
    }
}

fn parse_rating(s: &str) -> Rating {
    match s {
        "wins" => Rating::Wins,
        "power" => Rating::Power,
        "maxpreps" => Rating::MaxPreps,
        _ => match s.strip_prefix("rpi:") {
            Some(w) => {
                let v: Vec<f64> = w.split('/').filter_map(|x| x.parse().ok()).collect();
                let t = v.iter().sum::<f64>().max(1.0);
                Rating::Rpi(
                    v.first().unwrap_or(&40.0) / t,
                    v.get(1).unwrap_or(&40.0) / t,
                    v.get(2).unwrap_or(&20.0) / t,
                )
            }
            None => Rating::WinPct,
        },
    }
}

fn parse_private(s: &str) -> PrivateRule {
    let s = s.trim();
    if s == "select" {
        PrivateRule::Select
    } else if s == "play_up" {
        PrivateRule::PlayUp
    } else if s == "tpf" {
        PrivateRule::Tpf
    } else if let Some(x) = s.strip_prefix("multiplier:") {
        PrivateRule::Multiplier(x.parse().unwrap_or(1.0))
    } else if let Some(x) = s.strip_prefix("nonpublic:") {
        PrivateRule::NonPublic(list(x))
    } else if let Some(x) = s.strip_prefix("private:") {
        let (from, label) = x.split_once(':').unwrap_or((x, "Private"));
        PrivateRule::Private { from: from.parse().unwrap_or(0), label: label.to_string() }
    } else {
        PrivateRule::None
    }
}

fn parse_open(s: &str) -> Option<Open> {
    let mut it = s.split(':');
    let teams: u32 = it.next()?.trim().parse().ok()?;
    let from = it.next().and_then(|x| x.parse().ok());
    let pools = it.next() == Some("pools");
    Some(Open { teams, from, pools })
}

/// Every state's format, from the data.
pub static FORMATS: LazyLock<Vec<Format>> = LazyLock::new(|| {
    let mut lines = FORMATS_CSV.lines().filter(|l| !l.trim().is_empty());
    let header = split_csv_line(lines.next().unwrap_or(""));
    let col = |n: &str| header.iter().position(|h| h == n).unwrap_or_else(|| panic!("missing column {n}"));
    let c = [
        col("state"),
        col("classes"),
        col("sizes"),
        col("private_classes"),
        col("private_rule"),
        col("brackets"),
        col("qualify"),
        col("fields"),
        col("rating"),
        col("open"),
    ];
    lines
        .map(split_csv_line)
        .map(|r| {
            let brackets = r[c[5]].trim();
            let brackets = if let Some(x) = brackets.strip_prefix("equity:") {
                Brackets::Equity(list(x))
            } else if let Some(x) = brackets.strip_prefix("enrollment:") {
                Brackets::Enrollment(x.parse().unwrap_or(1))
            } else {
                Brackets::ByClass
            };
            Format {
                code: r[c[0]].trim().to_string(),
                classes: list(&r[c[1]]),
                sizes: list(&r[c[2]]).iter().filter_map(|x| x.parse().ok()).collect(),
                private_classes: list(&r[c[3]]),
                private: parse_private(&r[c[4]]),
                brackets,
                qualify: list(&r[c[6]]).iter().map(|x| parse_qualify(x)).collect(),
                fields: list(&r[c[7]]).iter().filter_map(|x| x.parse().ok()).collect(),
                rating: parse_rating(r[c[8]].trim()),
                open: parse_open(&r[c[9]]),
            }
        })
        .collect()
});

/// The default for a state with no row: its classes by enrollment, district
/// champions then the best by record, 32 a bracket (the most common shape
/// in the research: about half the teams, district finish plus a rating).
static DEFAULT: LazyLock<Format> = LazyLock::new(|| Format {
    code: String::new(),
    classes: Vec::new(),
    sizes: Vec::new(),
    private_classes: Vec::new(),
    private: PrivateRule::None,
    brackets: Brackets::ByClass,
    qualify: vec![Qualify::ChampTop],
    fields: vec![32],
    rating: Rating::WinPct,
    open: None,
});

pub fn format(code: &str) -> &'static Format {
    FORMATS.iter().find(|f| f.code == code).unwrap_or(&DEFAULT)
}

impl Format {
    fn nth<T: Copy>(v: &[T], i: usize, fallback: T) -> T {
        v.get(i).or(v.last()).copied().unwrap_or(fallback)
    }
    pub fn qualify_of(&self, i: usize) -> Qualify {
        Self::nth(&self.qualify, i, Qualify::ChampTop)
    }
    pub fn field_of(&self, i: usize) -> u32 {
        Self::nth(&self.fields, i, 32)
    }
    /// Season classes, public then private-only (a state with no row keeps
    /// the count it was generated with).
    pub fn class_count(&self, generated: usize) -> usize {
        if self.classes.is_empty() { generated } else { self.classes.len() + self.private_classes.len() }
    }
    /// The brackets that come between the public classes and the private
    /// classes (Select, Non-Public, Private).
    fn middle(&self) -> Vec<String> {
        match &self.private {
            PrivateRule::Select => self.classes.iter().map(|c| format!("Select {c}")).collect(),
            PrivateRule::NonPublic(l) => l.clone(),
            PrivateRule::Private { label, .. } => vec![label.clone()],
            _ => Vec::new(),
        }
    }
    /// Every bracket's name, in the order the data's lists follow (the Open
    /// Division, if any, comes first and is not in it).
    pub fn bracket_labels(&self) -> Vec<String> {
        if let Brackets::Equity(l) = &self.brackets {
            return l.clone();
        }
        let select = self.private == PrivateRule::Select;
        let mut out: Vec<String> =
            self.classes.iter().map(|c| if select { format!("Non-Select {c}") } else { c.clone() }).collect();
        out.extend(self.middle());
        out.extend(self.private_classes.iter().cloned());
        out
    }
    /// A season class's name.
    pub fn class_label(&self, class: u32) -> Option<&str> {
        let c = class as usize;
        self.classes.get(c).or_else(|| self.private_classes.get(c.checked_sub(self.classes.len())?)).map(|s| s.as_str())
    }
}

/// Real private schools among the states' named powerhouses, by the words
/// their names carry (Catholic and other religious schools, prep schools and
/// private academies). Charter and magnet schools stay public.
const PRIVATE_WORDS: [&str; 44] = [
    "St. ",
    "Saint ",
    "Bishop ",
    "Catholic",
    "Christian",
    "Jesuit",
    "Mater Dei",
    "La Salle",
    "Salesianum",
    "Brother ",
    "Baylor",
    "Montgomery Bell",
    "Lipscomb",
    "Trinity",
    "Loyola",
    "Gonzaga",
    "DeMatha",
    "McDonogh",
    "Archbishop",
    "Monsignor",
    "Canisius",
    "Iona",
    "Xavier",
    "Dowling",
    "Valor",
    "Sierra Canyon",
    "Santa Margarita",
    "Providence Day",
    "IMG",
    "Kapaun",
    "Mt. Carmel",
    "Our Lady",
    "Malvern",
    "Lutheran",
    "Hendricken",
    "Brentwood Academy",
    "Caravel",
    "Prep",
    "Thornton Academy",
    "St. Johnsbury",
    "Burr & Burton",
    "Episcopal",
    "Spalding",
    "Guertin",
];

/// Whether a real school is private, from its name.
pub fn real_private(name: &str) -> bool {
    PRIVATE_WORDS.iter().any(|w| name.contains(w))
}

/// Schools that play a national schedule and no state playoffs.
pub const INDEPENDENTS: [&str; 1] = ["IMG Academy"];

pub fn independent(name: &str) -> bool {
    INDEPENDENTS.iter().any(|x| name.starts_with(x))
}

/// Playoff wins over the last `years` seasons before `year`.
fn recent_playoff_wins(h: &super::high_school::HighSchool, year: i32, years: i32) -> u32 {
    h.seasons.iter().filter(|s| s.year >= year - years && s.year < year).map(|s| s.playoff_wins as u32).sum()
}

/// Playoff wins over three years that move a Wisconsin school up a division
/// (proposed, not confirmed: about two deep runs).
pub const TPF_WINS: u32 = 7;

/// A new football year's classes in every state, by its format: enrollment
/// (with Illinois' private multiplier), private schools apart where the
/// state does that, then Connecticut's play-up rule and Wisconsin's
/// tournament performance factor from the last seasons' playoff runs.
pub(crate) fn reclassify(w: &mut World) {
    let year = w.year;
    for s in 0..w.states.len() {
        let f = format(&w.states[s].code);
        let schools = w.states[s].schools.clone();
        for &h in &schools {
            let hs = &mut w.high_schools[h];
            if hs.real {
                hs.private = real_private(&hs.name);
            }
        }
        if f.classes.is_empty() {
            continue;
        }
        let n = f.classes.len();
        let apart = !f.private_classes.is_empty();
        let mult = if let PrivateRule::Multiplier(m) = f.private { m } else { 1.0 };
        let eff = |h: usize| {
            let x = &w.high_schools[h];
            x.enrollment as f64 * if x.private { mult } else { 1.0 }
        };
        let (mut privs, mut pubs): (Vec<usize>, Vec<usize>) =
            schools.iter().partition(|&&h| apart && w.high_schools[h].private);
        let order = |v: &mut Vec<usize>| v.sort_by(|&a, &b| eff(b).total_cmp(&eff(a)).then(a.cmp(&b)));
        order(&mut pubs);
        order(&mut privs);
        let weights: Vec<f64> = if f.sizes.len() == n { f.sizes.clone() } else { vec![1.0; n] };
        let total: f64 = weights.iter().sum();
        let mut classes: Vec<(usize, u32)> = Vec::with_capacity(schools.len());
        for (k, &h) in pubs.iter().enumerate() {
            let at = (k as f64 + 0.5) / pubs.len() as f64 * total;
            let mut acc = 0.0;
            let mut c = n - 1;
            for (j, wt) in weights.iter().enumerate() {
                acc += wt;
                if at < acc {
                    c = j;
                    break;
                }
            }
            classes.push((h, c as u32));
        }
        let m = f.private_classes.len().max(1);
        for (k, &h) in privs.iter().enumerate() {
            classes.push((h, (n + k * m / privs.len().max(1)) as u32));
        }
        for (h, mut c) in classes {
            let x = &w.high_schools[h];
            let base = if c as usize >= n { n as u32 } else { 0 };
            let up = match f.private {
                PrivateRule::PlayUp => x.private && x.seasons.iter().any(|s| s.year >= year - 2 && s.semifinal),
                PrivateRule::Tpf => recent_playoff_wins(x, year, 3) >= TPF_WINS,
                _ => false,
            };
            if up && c > base {
                c -= 1;
            }
            w.high_schools[h].classification = c;
        }
    }
}

/// A season's regular-season results for the ratings: each school's games
/// as (opponent, won).
pub(crate) fn opponents(n: usize, played: &[(u32, u32)]) -> Vec<Vec<(u32, bool)>> {
    let mut v: Vec<Vec<(u32, bool)>> = vec![Vec::new(); n];
    for &(wn, l) in played {
        if let Some(x) = v.get_mut(wn as usize) {
            x.push((l, true));
        }
        if let Some(x) = v.get_mut(l as usize) {
            x.push((wn, false));
        }
    }
    v
}

fn owp(h: usize, opps: &[Vec<(u32, bool)>], results: &[TeamResult]) -> f64 {
    let o = &opps[h];
    if o.is_empty() {
        return 0.5;
    }
    o.iter().map(|&(x, _)| results[x as usize].win_pct()).sum::<f64>() / o.len() as f64
}

/// A team's rating under its state's formula.
pub(crate) fn rate(r: Rating, h: usize, opps: &[Vec<(u32, bool)>], results: &[TeamResult]) -> f64 {
    let t = &results[h];
    let wp = t.win_pct();
    match r {
        Rating::WinPct => wp + 0.01 * owp(h, opps, results),
        Rating::Wins => {
            t.wins as f64 + 0.001 * opps[h].iter().map(|&(x, _)| results[x as usize].wins).sum::<u32>() as f64
        }
        Rating::Rpi(a, b, c) => {
            let o = &opps[h];
            let oo = if o.is_empty() {
                0.5
            } else {
                o.iter().map(|&(x, _)| owp(x as usize, opps, results)).sum::<f64>() / o.len() as f64
            };
            a * wp + b * owp(h, opps, results) + c * oo
        }
        Rating::Power => {
            let g = t.games().max(1) as f64;
            let pts: f64 = opps[h]
                .iter()
                .map(|&(x, won)| {
                    let ow = results[x as usize].wins as f64;
                    if won { 10.0 + ow } else { 0.5 * ow }
                })
                .sum();
            pts / g
        }
        // A computer ranking reads scores, and scores follow how well a
        // team plays: record and schedule, plus the team's level of play.
        Rating::MaxPreps => 0.5 * wp + 0.5 * owp(h, opps, results) + 0.02 * (t.strength - 50.0),
    }
}

/// One playoff bracket.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Bracket {
    pub state: u16,
    pub label: String,
    /// Teams by seed, best first.
    pub seeds: Vec<u32>,
    /// This round's slots in bracket order (NONE: a bye or a team out).
    pub slots: Vec<u32>,
    /// The state's playoff round it starts in.
    pub start: u8,
    /// The state's next round it plays.
    pub round: u8,
    /// Pool wins by seed while a pool round robin runs (Florida's Open).
    pub pool: Vec<u8>,
    /// Playoff wins by seed.
    pub wins: Vec<u8>,
    /// Reached the last four, by seed.
    pub semi: Vec<bool>,
    pub champion: u32,
}

/// Round robin pairings of a pool of four (indexes into the pool).
const POOL_ROUNDS: [[(usize, usize); 2]; 3] = [[(0, 3), (1, 2)], [(0, 2), (3, 1)], [(0, 1), (2, 3)]];
/// Florida's pools: seeds 1, 4, 5 and 8, and seeds 2, 3, 6 and 7.
const POOLS: [[usize; 4]; 2] = [[0, 3, 4, 7], [1, 2, 5, 6]];

/// Seed order in a bracket of `size` (a power of two): 1 v 16, 8 v 9 and so
/// on, so the top seeds can only meet late.
pub fn bracket_order(size: usize) -> Vec<usize> {
    let mut order = vec![0usize];
    while order.len() < size {
        let n = order.len() * 2;
        order = order.iter().flat_map(|&x| [x, n - 1 - x]).collect();
    }
    order
}

/// Rounds a knockout field needs.
pub fn rounds_for(field: usize) -> usize {
    field.max(2).next_power_of_two().trailing_zeros() as usize
}

impl Bracket {
    /// A bracket of `seeds` (best first) that ends on the state's last
    /// round. A field too big for the rounds there are keeps its best.
    pub fn new(state: usize, label: String, mut seeds: Vec<u32>, rounds: usize, pools: bool) -> Bracket {
        let pools = pools && seeds.len() == 8 && rounds >= 5;
        let need = if pools { 5 } else { rounds_for(seeds.len()) };
        let need = if need > rounds {
            seeds.truncate(1 << rounds);
            rounds
        } else {
            need
        };
        let start = (rounds - need) as u8;
        let n = seeds.len();
        let slots = if pools {
            Vec::new()
        } else {
            let size = 1usize << rounds_for(n);
            bracket_order(size).into_iter().map(|k| seeds.get(k).copied().unwrap_or(NONE)).collect()
        };
        Bracket {
            state: state as u16,
            label,
            slots,
            start,
            round: start,
            pool: if pools { vec![0; n] } else { Vec::new() },
            wins: vec![0; n],
            semi: vec![n <= 4; n],
            champion: if n == 1 { seeds[0] } else { NONE },
            seeds,
        }
    }

    pub fn seed_of(&self, t: u32) -> usize {
        self.seeds.iter().position(|&x| x == t).unwrap_or(usize::MAX)
    }

    pub fn done(&self) -> bool {
        self.champion != NONE || self.seeds.is_empty()
    }

    /// This round's games, the better seed first (he hosts).
    pub fn games(&self) -> Vec<(u32, u32)> {
        if self.done() {
            return Vec::new();
        }
        if !self.pool.is_empty() {
            let r = (self.round - self.start) as usize;
            return POOLS
                .iter()
                .flat_map(|p| POOL_ROUNDS[r.min(2)].iter().map(move |&(a, b)| (p[a], p[b])))
                .filter_map(|(a, b)| Some((*self.seeds.get(a)?, *self.seeds.get(b)?)))
                .collect();
        }
        self.slots
            .chunks(2)
            .filter(|p| p.len() == 2 && p[0] != NONE && p[1] != NONE)
            .map(|p| if self.seed_of(p[0]) <= self.seed_of(p[1]) { (p[0], p[1]) } else { (p[1], p[0]) })
            .collect()
    }

    /// Files this round's results, (winner, loser), and moves on.
    pub fn advance(&mut self, results: &[(u32, u32)]) {
        if self.done() {
            return;
        }
        for &(wn, _) in results {
            let k = self.seed_of(wn);
            if k < self.wins.len() {
                self.wins[k] = self.wins[k].saturating_add(1);
                if !self.pool.is_empty() {
                    self.pool[k] += 1;
                }
            }
        }
        self.round += 1;
        if !self.pool.is_empty() {
            if self.round - self.start < 3 {
                return;
            }
            // The two best of each pool go to the semifinals, crossed.
            let rank = |p: &[usize; 4]| {
                let mut v: Vec<usize> = p.iter().copied().filter(|&k| k < self.seeds.len()).collect();
                v.sort_by_key(|&k| (std::cmp::Reverse(self.pool[k]), k));
                v
            };
            let (a, b) = (rank(&POOLS[0]), rank(&POOLS[1]));
            let at = |v: &[usize], i: usize| v.get(i).map_or(NONE, |&k| self.seeds[k]);
            self.slots = vec![at(&a, 0), at(&b, 1), at(&b, 0), at(&a, 1)];
            self.pool.clear();
            for t in self.slots.clone() {
                let k = self.seed_of(t);
                if k < self.semi.len() {
                    self.semi[k] = true;
                }
            }
            return;
        }
        let next: Vec<u32> = self
            .slots
            .chunks(2)
            .map(|p| match (p[0], p.get(1).copied().unwrap_or(NONE)) {
                (NONE, b) => b,
                (a, NONE) => a,
                (a, b) => {
                    if results.iter().any(|r| r.0 == a && r.1 == b) {
                        a
                    } else if results.iter().any(|r| r.0 == b && r.1 == a) {
                        b
                    } else {
                        // Not played (should not happen): the better seed goes on.
                        if self.seed_of(a) <= self.seed_of(b) { a } else { b }
                    }
                }
            })
            .collect();
        self.slots = next;
        if self.slots.len() <= 4 {
            for t in self.slots.clone() {
                let k = self.seed_of(t);
                if k < self.semi.len() {
                    self.semi[k] = true;
                }
            }
        }
        if self.slots.len() == 1 {
            self.champion = self.slots[0];
        }
    }

    /// Still alive.
    pub fn alive(&self, t: u32) -> bool {
        !self.done() && (self.slots.contains(&t) || !self.pool.is_empty() && self.seeds.contains(&t))
    }
}

/// Builds a state's brackets at the end of its regular season.
/// `district` is each school's district (by school id), `opps` each school's
/// games, `rounds` the state's playoff rounds, `scale` the world's size.
pub(crate) fn build(
    w: &World,
    s: usize,
    results: &[TeamResult],
    district: &[u32],
    opps: &[Vec<(u32, bool)>],
    rounds: usize,
) -> Vec<Bracket> {
    let f = format(&w.states[s].code);
    let scale = w.options.scale;
    let schools: Vec<usize> = w.states[s]
        .schools
        .iter()
        .copied()
        .filter(|&h| results.get(h).is_some_and(|r| r.roster > 0 && r.games() > 0))
        .filter(|&h| !independent(&w.high_schools[h].name))
        .collect();
    let rating: std::collections::HashMap<usize, f64> =
        schools.iter().map(|&h| (h, rate(f.rating, h, opps, results))).collect();
    let by_rating = |v: &mut Vec<usize>| v.sort_by(|a, b| rating[b].total_cmp(&rating[a]).then(a.cmp(b)));
    // District champions and standings, by district record then rating.
    let dist = |h: usize| district.get(h).copied().unwrap_or(NONE);
    let mut standing: std::collections::HashMap<u32, Vec<usize>> = std::collections::HashMap::new();
    for &h in &schools {
        standing.entry(dist(h)).or_default().push(h);
    }
    for v in standing.values_mut() {
        v.sort_by(|&a, &b| {
            let (ra, rb) = (&results[a], &results[b]);
            let da = ra.conf_wins as i64 - ra.conf_losses as i64;
            let db = rb.conf_wins as i64 - rb.conf_losses as i64;
            db.cmp(&da).then(rating[&b].total_cmp(&rating[&a])).then(a.cmp(&b))
        });
    }
    let place = |h: usize| -> usize {
        if dist(h) == NONE {
            return 0;
        }
        standing[&dist(h)].iter().position(|&x| x == h).unwrap_or(usize::MAX)
    };
    let scaled = |real: u32| ((real as f64 * scale).round() as usize).max(2);
    let max_field = 1usize << rounds.min(10);
    // Picks a field from its pool (for one bracket, `pick` below caps it at
    // the rounds the state plays).
    let pick_n = |pool: &[usize], q: Qualify, field: usize| -> Vec<usize> {
        let field = field.min(pool.len());
        let mut sorted = pool.to_vec();
        by_rating(&mut sorted);
        let mut out = match q {
            Qualify::All | Qualify::Top => sorted.into_iter().take(field).collect::<Vec<_>>(),
            Qualify::ChampTop => {
                let mut champs: Vec<usize> = sorted.iter().copied().filter(|&h| place(h) == 0).collect();
                champs.truncate(field);
                let rest: Vec<usize> = sorted.iter().copied().filter(|h| !champs.contains(h)).collect();
                let room = field - champs.len();
                champs.extend(rest.into_iter().take(room));
                champs
            }
            Qualify::District(n) => {
                let mut v: Vec<usize> = sorted.into_iter().filter(|&h| place(h) < n as usize).collect();
                v.truncate(field);
                v
            }
        };
        by_rating(&mut out);
        out
    };
    let pick = |pool: &[usize], q: Qualify, field: usize| pick_n(pool, q, field.min(max_field));
    let labels = f.bracket_labels();
    let n = f.classes.len();
    let middle = labels.len() - n - f.private_classes.len();
    let mut out: Vec<Bracket> = Vec::new();
    let mut taken: Vec<usize> = Vec::new();
    // The Open Division first.
    if let Some(o) = &f.open {
        let mut pool: Vec<usize> = schools
            .iter()
            .copied()
            .filter(|&h| o.from.is_none_or(|c| w.high_schools[h].classification == c))
            .filter(|&h| (w.high_schools[h].classification as usize) < n.max(1))
            .collect();
        by_rating(&mut pool);
        let field = scaled(o.teams).min(pool.len());
        if field >= 2 {
            let seeds: Vec<usize> = pool.into_iter().take(field).collect();
            taken.extend(&seeds);
            let seeds = seeds.into_iter().map(|h| h as u32).collect();
            out.push(Bracket::new(s, "Open".into(), seeds, rounds, o.pools));
        }
    }
    let left: Vec<usize> = schools.iter().copied().filter(|h| !taken.contains(h)).collect();
    let make = |label: &str, seeds: Vec<usize>, out: &mut Vec<Bracket>| {
        if seeds.len() >= 2 {
            out.push(Bracket::new(s, label.to_string(), seeds.into_iter().map(|h| h as u32).collect(), rounds, false));
        } else if seeds.len() == 1 {
            // A class of one: crowned without a game.
            out.push(Bracket::new(s, label.to_string(), vec![seeds[0] as u32], rounds, false));
        }
    };
    match &f.brackets {
        Brackets::Equity(_) => {
            // Everyone who qualifies, placed by strength. The rounds cap each
            // bracket, not the whole pool: capping the pool left California
            // three of its fifteen brackets (minor bug 170).
            let field = |i: usize| scaled(f.field_of(i)).min(max_field);
            let total: usize = (0..labels.len()).map(field).sum();
            let mut q = pick_n(&left, f.qualify_of(0), total.min(left.len()));
            by_rating(&mut q);
            let mut at = 0;
            for (i, label) in labels.iter().enumerate() {
                let k = field(i).min(q.len() - at);
                make(label, q[at..at + k].to_vec(), &mut out);
                at += k;
            }
        }
        _ => {
            let pooled = if let Brackets::Enrollment(k) = f.brackets { k.min(n) } else { 0 };
            // Each school's bracket.
            let mut privs: Vec<usize> = left.iter().copied().filter(|&h| w.high_schools[h].private).collect();
            privs.sort_by(|&a, &b| w.high_schools[b].enrollment.cmp(&w.high_schools[a].enrollment).then(a.cmp(&b)));
            let bracket_of = |h: usize| -> usize {
                let x = &w.high_schools[h];
                let c = (x.classification as usize).min(labels.len().saturating_sub(1 + middle));
                if c >= n {
                    return n + middle + (c - n);
                }
                if !x.private {
                    return c;
                }
                match &f.private {
                    PrivateRule::Select => n + c,
                    PrivateRule::NonPublic(l) => {
                        let k = privs.iter().position(|&p| p == h).unwrap_or(0);
                        n + k * l.len().max(1) / privs.len().max(1)
                    }
                    PrivateRule::Private { from, .. } if c >= *from as usize => n,
                    _ => c,
                }
            };
            let mut pools: Vec<Vec<usize>> = vec![Vec::new(); labels.len()];
            for &h in &left {
                pools[bracket_of(h).min(labels.len() - 1)].push(h);
            }
            if pooled > 0 {
                // Selected together, then split by enrollment.
                let pool: Vec<usize> = pools[..pooled].concat();
                // Each bracket's field is capped by the rounds, not the pool
                // (Illinois and Michigan seeded eight brackets from one
                // bracket's worth of teams; minor bug 170).
                let total: usize = (0..pooled).map(|i| scaled(f.field_of(i)).min(max_field)).sum();
                let mut q = pick_n(&pool, f.qualify_of(0), total);
                q.sort_by(|&a, &b| w.high_schools[b].enrollment.cmp(&w.high_schools[a].enrollment).then(a.cmp(&b)));
                for (i, label) in labels.iter().enumerate().take(pooled) {
                    let part: Vec<usize> = q[i * q.len() / pooled..(i + 1) * q.len() / pooled].to_vec();
                    let mut part = part;
                    by_rating(&mut part);
                    make(label, part, &mut out);
                }
            }
            for (i, label) in labels.iter().enumerate().skip(pooled) {
                let seeds = pick(&pools[i], f.qualify_of(i), scaled(f.field_of(i)));
                make(label, seeds, &mut out);
            }
        }
    }
    out
}
