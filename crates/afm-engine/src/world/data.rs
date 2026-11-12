//! Real-world data from `data/real-world/`, compiled into the engine.
//!
//! Every figure has a source in its CSV row or in that folder's README. Where
//! no public source exists (NIL dollars per school, for example), the world
//! models an estimate instead and says so.

use std::collections::HashMap;
use std::sync::LazyLock;

const COLLEGES_CSV: &str = include_str!("../../../../data/real-world/colleges.csv");
const COLLEGES_LOWER_CSV: &str = include_str!("../../../../data/real-world/colleges_lower.csv");
const COLLEGE_HISTORY_CSV: &str = include_str!("../../../../data/real-world/college_history.csv");
const COLLEGE_DROPPED_CSV: &str = include_str!("../../../../data/real-world/college_history_dropped.csv");
const COLLEGE_CANDIDATES_CSV: &str = include_str!("../../../../data/real-world/college_candidates.csv");
const COLLEGE_START_RECORDS_CSV: &str = include_str!("../../../../data/real-world/college_start_records.csv");
const COLLEGE_TITLES_CSV: &str = include_str!("../../../../data/real-world/college_titles.csv");
const COLLEGE_SEATS_CSV: &str = include_str!("../../../../data/real-world/college_seats.csv");
const HS_STATES_CSV: &str = include_str!("../../../../data/real-world/hs_states.csv");
const RIVALRIES_CSV: &str = include_str!("../../../../data/real-world/rivalries.csv");
const TALENT_CSV: &str = include_str!("../../../../data/real-world/talent_by_state.csv");
const NFL_TEAMS_CSV: &str = include_str!("../../../../data/real-world/nfl_teams.csv");

/// A college's division in the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Division {
    FBS,
    FCS,
    D2,
    /// NAIA football (scholarships, below Division II).
    NAIA,
    /// Division III: no athletic scholarships.
    D3,
    /// Junior colleges (NJCAA and California's CCCAA).
    JUCO,
}

/// The number of college divisions, for arrays indexed by `Division as usize`.
pub const DIVISION_COUNT: usize = 6;

impl Division {
    /// Every division, from the top level down (the order of the enum).
    pub const ALL: [Division; DIVISION_COUNT] =
        [Division::FBS, Division::FCS, Division::D2, Division::NAIA, Division::D3, Division::JUCO];

    pub fn name(self) -> &'static str {
        match self {
            Division::FBS => "FBS",
            Division::FCS => "FCS",
            Division::D2 => "D2",
            Division::NAIA => "NAIA",
            Division::D3 => "D3",
            Division::JUCO => "JUCO",
        }
    }

    pub fn parse(s: &str) -> Option<Division> {
        Some(match s {
            "FBS" => Division::FBS,
            "FCS" => Division::FCS,
            "D2" => Division::D2,
            "NAIA" => Division::NAIA,
            "D3" => Division::D3,
            "JUCO" => Division::JUCO,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AcademicTier {
    Aau,
    R1,
    R2,
    Other,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CollegeRow {
    pub name: String,
    pub division: Division,
    pub conference: String,
    pub city: String,
    pub state: String,
    pub enrollment: u32,
    pub national_titles: u32,
    pub last_5yr_win_pct: Option<f64>,
    pub stadium_capacity: Option<u32>,
    pub academic_tier: AcademicTier,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StateRow {
    pub code: String,
    pub governing_body: String,
    pub programs: u32,
    pub number_of_classes: u32,
    pub transfer_rule: String,
    /// The state's top 5 programs (name, town) on the ranking date.
    pub powerhouses: Vec<(String, String)>,
    pub fbs_signees_per_year: f64,
    pub draftees_per_year: f64,
    pub top_hometowns: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NflTeamRow {
    pub name: String,
    pub city: String,
    pub state: String,
    pub conference: String,
    pub division: String,
    pub market_rank: u32,
}

/// Splits one CSV line, honouring double quotes.
pub fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// The whole number a cell starts with, if any ("7 (plus ...)" is 7).
fn leading_number(cell: &str) -> Option<u32> {
    let s = cell.trim();
    s[..s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len())].parse().ok()
}

struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    fn parse(text: &str) -> Table {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let header = split_csv_line(lines.next().unwrap_or(""));
        let rows = lines.map(split_csv_line).collect();
        Table { header, rows }
    }
    fn col(&self, name: &str) -> usize {
        self.header.iter().position(|h| h == name).unwrap_or_else(|| panic!("missing column {name}"))
    }
}

/// Every real college program playing in 2026: FBS, FCS and Division II
/// (`colleges.csv`), then Division III, NAIA and the junior colleges
/// (`colleges_lower.csv`).
pub static COLLEGES: LazyLock<Vec<CollegeRow>> = LazyLock::new(|| {
    let mut rows = college_rows(COLLEGES_CSV);
    rows.extend(college_rows(COLLEGES_LOWER_CSV));
    rows
});

/// The first and last seasons the college history covers.
pub const HISTORY_FIRST: i32 = 1984;
pub const HISTORY_LAST: i32 = 2025;

/// One span of a program's past (1984-2025) that differs from its 2026 row:
/// its division and conference in those seasons, or no team at all.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HistoryRow {
    pub name: String,
    pub first: i32,
    pub last: i32,
    /// None: the school fielded no team (not yet started, paused or dropped).
    pub division: Option<Division>,
    pub conference: String,
}

/// The past of the programs playing today (`college_history.csv`) and of
/// the programs that dropped football since 1984 (`college_history_dropped.csv`).
pub static COLLEGE_HISTORY: LazyLock<Vec<HistoryRow>> = LazyLock::new(|| {
    let mut out = Vec::new();
    for csv in [COLLEGE_HISTORY_CSV, COLLEGE_DROPPED_CSV] {
        let t = Table::parse(csv);
        let (name, first, last, div, conf) =
            (t.col("name"), t.col("first_season"), t.col("last_season"), t.col("division"), t.col("conference"));
        for r in &t.rows {
            out.push(HistoryRow {
                name: r[name].clone(),
                first: r[first].parse().unwrap_or(1984),
                last: r[last].parse().unwrap_or(2025),
                division: Division::parse(&r[div]),
                conference: r[conf].clone(),
            });
        }
    }
    out
});

/// One season's record: season, wins, losses, ties.
pub type SeasonRecord = (i32, u32, u32, u32);

/// Each program's real records in the seasons before the world starts
/// (`college_start_records.csv`, 1979-83, minor bug 294).
pub static START_RECORDS: LazyLock<HashMap<String, Vec<SeasonRecord>>> = LazyLock::new(|| {
    let t = Table::parse(COLLEGE_START_RECORDS_CSV);
    let (name, season, w, l, tie) = (t.col("name"), t.col("season"), t.col("wins"), t.col("losses"), t.col("ties"));
    let mut out: HashMap<String, Vec<SeasonRecord>> = HashMap::new();
    for r in &t.rows {
        let n = |c: usize| r[c].parse().unwrap_or(0);
        out.entry(r[name].clone()).or_default().push((r[season].parse().unwrap_or(0), n(w), n(l), n(tie)));
    }
    out
});

/// A program's real win share (ties count half) over the five seasons
/// before `season`, from the seasons it played (minor bug 294); None when
/// none is known.
pub fn start_record(name: &str, season: i32) -> Option<f64> {
    let (mut w, mut g) = (0.0, 0.0);
    for &(_, wins, losses, ties) in START_RECORDS.get(name)?.iter().filter(|x| (season - 5..season).contains(&x.0)) {
        w += wins as f64 + 0.5 * ties as f64;
        g += (wins + losses + ties) as f64;
    }
    (g > 0.0).then(|| w / g)
}

/// Every real national title of the world's programs, by season and the
/// level it was won at (`college_titles.csv`, minor bug 295).
pub static COLLEGE_TITLES: LazyLock<HashMap<String, Vec<(i32, Division)>>> = LazyLock::new(|| {
    let t = Table::parse(COLLEGE_TITLES_CSV);
    let (name, season, level) = (t.col("name"), t.col("season"), t.col("level"));
    let mut out: HashMap<String, Vec<(i32, Division)>> = HashMap::new();
    for r in &t.rows {
        if let Some(d) = Division::parse(&r[level]) {
            out.entry(r[name].clone()).or_default().push((r[season].parse().unwrap_or(0), d));
        }
    }
    out
});

/// A program's real national titles won before `season`, by level
/// (`Division as usize`).
pub fn titles_before(name: &str, season: i32) -> [u32; DIVISION_COUNT] {
    let mut out = [0u32; DIVISION_COUNT];
    for &(_, d) in COLLEGE_TITLES.get(name).into_iter().flatten().filter(|x| x.0 < season) {
        out[d as usize] += 1;
    }
    out
}

/// Seats over a run of seasons: first season, last season, seats.
pub type SeatSpan = (i32, i32, u32);

/// Each program's real home seats by season (`college_seats.csv`, minor
/// bug 296).
pub static COLLEGE_SEATS: LazyLock<HashMap<String, Vec<SeatSpan>>> = LazyLock::new(|| {
    let t = Table::parse(COLLEGE_SEATS_CSV);
    let (name, first, last, seats) = (t.col("name"), t.col("first_season"), t.col("last_season"), t.col("seats"));
    let mut out: HashMap<String, Vec<SeatSpan>> = HashMap::new();
    for r in &t.rows {
        let (Ok(a), Ok(b), Ok(n)) = (r[first].parse(), r[last].parse(), r[seats].parse()) else { continue };
        out.entry(r[name].clone()).or_default().push((a, b, n));
    }
    out
});

/// A program's real home seats in `season` (1984-2025), when known. After
/// 2025 its seats today (`colleges.csv`): a temporary home in 2025 gives way
/// to the new stadium (Northwestern's 12,023-seat Martin Stadium to the new
/// Ryan Field's 35,000 in 2026).
pub fn real_seats(name: &str, season: i32) -> Option<u32> {
    let span = COLLEGE_SEATS.get(name).and_then(|v| v.iter().find(|x| (x.0..=x.1).contains(&season)).map(|x| x.2));
    span.or_else(|| {
        (season > HISTORY_LAST)
            .then(|| COLLEGES.iter().find(|r| r.name == name).and_then(|r| r.stadium_capacity))
            .flatten()
    })
}

/// A listed rivalry's real past (`rivalries.csv`, part B 28-30): the
/// seasons it was played and its all-time record and trophy holder after
/// 1983 and after 1995. Wins are [first listed, second listed].
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RivalryPast {
    pub first: i32,
    /// The last season played, when the series has ended for now.
    pub last: Option<i32>,
    /// Seasons since 1984 it was not played.
    pub pauses: Vec<i32>,
    pub thru_1983: ([u32; 2], u32, Option<usize>),
    pub thru_1995: ([u32; 2], u32, Option<usize>),
}

impl RivalryPast {
    /// Played in this season. Up to 2025 the real years; after that every
    /// year unless the series has ended.
    pub fn played(&self, season: i32) -> bool {
        season >= self.first && !self.pauses.contains(&season) && self.last.is_none_or(|l| season <= l)
    }
}

/// Each rivalry's past by its two names (first listed, second listed).
pub static RIVALRY_PAST: LazyLock<HashMap<(String, String), RivalryPast>> = LazyLock::new(|| {
    let t = Table::parse(RIVALRIES_CSV);
    let c = |n: &str| t.col(n);
    let num = |r: &Vec<String>, n: &str| r[c(n)].trim().parse::<u32>().unwrap_or(0);
    let holder = |r: &Vec<String>, n: &str| match r[c(n)].trim() {
        "a" => Some(0),
        "b" => Some(1),
        _ => None,
    };
    let mut out = HashMap::new();
    for r in &t.rows {
        let mut pauses = Vec::new();
        for part in r[c("pauses")].split(';').map(str::trim).filter(|x| !x.is_empty()) {
            let (from, to) = part.split_once('-').unwrap_or((part, part));
            let (from, to) = (from.trim().parse().unwrap_or(0), to.trim().parse().unwrap_or(0));
            pauses.extend(from..=to);
        }
        out.insert(
            (r[c("a")].clone(), r[c("b")].clone()),
            RivalryPast {
                first: r[c("first_year")].trim().parse().unwrap_or(HISTORY_FIRST),
                last: r[c("last_year")].trim().parse().ok(),
                pauses,
                thru_1983: (
                    [num(r, "a_wins_thru_1983"), num(r, "b_wins_thru_1983")],
                    num(r, "ties_thru_1983"),
                    holder(r, "holder_after_1983"),
                ),
                thru_1995: (
                    [num(r, "a_wins_thru_1995"), num(r, "b_wins_thru_1995")],
                    num(r, "ties_thru_1995"),
                    holder(r, "holder_after_1995"),
                ),
            },
        );
    }
    out
});

/// A listed rivalry's real past, by its two names as listed.
pub fn rivalry_past(a: &str, b: &str) -> Option<&'static RivalryPast> {
    RIVALRY_PAST.get(&(a.to_string(), b.to_string()))
}

/// Programs that played since 1984 and have since dropped football, with
/// their first span's division and conference (`college_history_dropped.csv`).
pub static DROPPED_COLLEGES: LazyLock<Vec<CollegeRow>> = LazyLock::new(|| {
    let t = Table::parse(COLLEGE_DROPPED_CSV);
    let (name, div, conf, city, state) =
        (t.col("name"), t.col("division"), t.col("conference"), t.col("city"), t.col("state"));
    let mut out: Vec<CollegeRow> = Vec::new();
    for r in &t.rows {
        if out.iter().any(|c| c.name == r[name]) {
            continue;
        }
        let Some(division) = Division::parse(&r[div]) else { continue };
        out.push(CollegeRow {
            name: r[name].clone(),
            division,
            conference: r[conf].clone(),
            city: r[city].clone(),
            state: r[state].clone(),
            enrollment: 8000,
            national_titles: 0,
            last_5yr_win_pct: None,
            stadium_capacity: None,
            academic_tier: AcademicTier::Other,
        });
    }
    out
});

/// Real colleges without a football team that could start one (SK68):
/// Division I schools start at FCS, the rest at their own level
/// (`college_candidates.csv`; Claude's list, proposed, not confirmed).
pub static CANDIDATE_COLLEGES: LazyLock<Vec<CollegeRow>> = LazyLock::new(|| {
    let t = Table::parse(COLLEGE_CANDIDATES_CSV);
    let (name, div, city, state, enr) =
        (t.col("name"), t.col("division"), t.col("city"), t.col("state"), t.col("enrollment"));
    t.rows
        .iter()
        .filter_map(|r| {
            let division = Division::parse(&r[div])?;
            Some(CollegeRow {
                name: r[name].clone(),
                division,
                conference: "Independent".to_string(),
                city: r[city].clone(),
                state: r[state].clone(),
                enrollment: r[enr].parse().unwrap_or(3000),
                national_titles: 0,
                last_5yr_win_pct: None,
                stadium_capacity: None,
                academic_tier: AcademicTier::Other,
            })
        })
        .collect()
});

fn college_rows(csv: &str) -> Vec<CollegeRow> {
    let t = Table::parse(csv);
    let (name, div, conf, city, state, enr, titles, win, stadium, acad) = (
        t.col("name"),
        t.col("division"),
        t.col("conference"),
        t.col("city"),
        t.col("state"),
        t.col("enrollment"),
        t.col("national_titles"),
        t.col("last_5yr_win_pct"),
        t.col("stadium_capacity"),
        t.col("academic_tier"),
    );
    let notes = t.col("notes");
    t.rows
        .iter()
        // A school in the list that played no games in the data has no team.
        .filter(|r| !r[notes].contains("no completed games"))
        .map(|r| CollegeRow {
            name: r[name].clone(),
            division: Division::parse(&r[div]).unwrap_or(Division::D2),
            conference: r[conf].clone(),
            city: r[city].clone(),
            state: r[state].clone(),
            enrollment: r[enr].parse().unwrap_or(3000),
            national_titles: r[titles].parse().unwrap_or(0),
            last_5yr_win_pct: r[win].parse().ok(),
            stadium_capacity: r[stadium].parse().ok(),
            academic_tier: match r[acad].as_str() {
                "AAU" => AcademicTier::Aau,
                "R1" => AcademicTier::R1,
                "R2" => AcademicTier::R2,
                _ => AcademicTier::Other,
            },
        })
        .collect()
}

pub static STATES: LazyLock<Vec<StateRow>> = LazyLock::new(|| {
    let t = Table::parse(HS_STATES_CSV);
    let talent = Table::parse(TALENT_CSV);
    let (code, body, programs, classes, transfer, top5) = (
        t.col("state"),
        t.col("governing_body"),
        t.col("approx_programs"),
        t.col("number_of_classes"),
        t.col("transfer_rule_summary"),
        t.col("top_5_powerhouses"),
    );
    let (tcode, signees, draftees, towns) = (
        talent.col("state"),
        talent.col("avg_fbs_signees_per_year"),
        talent.col("avg_nfl_draftees_per_year"),
        talent.col("top_hometowns"),
    );
    t.rows
        .iter()
        .map(|r| {
            let trow = talent.rows.iter().find(|x| x[tcode] == r[code]);
            let powerhouses = r[top5]
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| match (s.find(" ("), s.rfind(')')) {
                    (Some(a), Some(b)) if b > a => (s[..a].to_string(), s[a + 2..b].to_string()),
                    _ => (s.to_string(), String::new()),
                })
                .collect();
            StateRow {
                code: r[code].clone(),
                governing_body: r[body].clone(),
                programs: r[programs].parse().unwrap_or(100),
                // The cell's leading number: Florida's reads "7 (plus Independent
                // League and FIT)". DC's is blank (the source covers the 50
                // states), so it keeps the default of 4.
                number_of_classes: leading_number(&r[classes]).unwrap_or(4),
                transfer_rule: r[transfer].clone(),
                powerhouses,
                fbs_signees_per_year: trow.and_then(|x| x[signees].parse().ok()).unwrap_or(1.0),
                draftees_per_year: trow.and_then(|x| x[draftees].parse().ok()).unwrap_or(0.1),
                top_hometowns: trow
                    .map(|x| x[towns].split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                    .unwrap_or_default(),
            }
        })
        .collect()
});

pub static NFL_TEAMS: LazyLock<Vec<NflTeamRow>> = LazyLock::new(|| {
    let t = Table::parse(NFL_TEAMS_CSV);
    let (name, city, state, conf, div, market) = (
        t.col("team"),
        t.col("city"),
        t.col("state"),
        t.col("conference"),
        t.col("division"),
        t.col("market_size_rank"),
    );
    t.rows
        .iter()
        .map(|r| NflTeamRow {
            name: r[name].clone(),
            city: r[city].clone(),
            state: r[state].clone(),
            conference: r[conf].clone(),
            division: r[div].clone(),
            market_rank: r[market].parse().unwrap_or(16),
        })
        .collect()
});

/// Whether a state's high school players may earn NIL money, from the
/// `hs_nil_allowed` column (yes/no) when the data has it.
pub fn hs_nil(code: &str) -> bool {
    let t = Table::parse(HS_STATES_CSV);
    let Some(col) = t.header.iter().position(|h| h == "hs_nil_allowed") else { return false };
    let state = t.col("state");
    t.rows.iter().any(|r| r[state] == code && r.get(col).is_some_and(|v| v.trim().eq_ignore_ascii_case("yes")))
}

/// Who a state's high school NIL rules let earn (NIL update N3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HsNilScope {
    /// Barred for everyone (Alabama, South Carolina).
    Barred,
    /// Any high school player, within the usual limits: no school marks,
    /// no pay for play, no recruiting inducement.
    All,
    /// Only a senior who has signed with a college in the state (Arkansas'
    /// Act 589, Missouri's 2023 law).
    SignedInState,
    /// Deals only with a college, paid once his high school eligibility
    /// ends (Texas' HB 126, 2025): nothing while he plays high school ball.
    CollegeAfterHs,
}

/// A state's high school NIL rule from `hs_states.csv` (N3): who may earn,
/// from which season, from what age, and its note for the screens.
#[derive(Clone, Debug)]
pub struct HsNilRule {
    pub scope: HsNilScope,
    pub from: Option<i32>,
    pub min_age: u32,
    pub note: String,
}

static HS_NIL_RULES: LazyLock<HashMap<String, HsNilRule>> = LazyLock::new(|| {
    let t = Table::parse(HS_STATES_CSV);
    let at = |name: &str| t.header.iter().position(|h| h == name);
    let (allowed, from, scope, age, note) =
        (at("hs_nil_allowed"), at("hs_nil_from"), at("hs_nil_scope"), at("hs_nil_min_age"), at("hs_nil_note"));
    let state = t.col("state");
    let cell = |r: &Vec<String>, c: Option<usize>| c.and_then(|c| r.get(c)).map_or("", |v| v.trim()).to_string();
    t.rows
        .iter()
        .map(|r| {
            let allowed = cell(r, allowed).to_ascii_lowercase();
            let scope = match (allowed.as_str(), cell(r, scope).as_str()) {
                ("yes", _) => HsNilScope::All,
                ("limited", "signed_in_state") => HsNilScope::SignedInState,
                ("limited", _) => HsNilScope::CollegeAfterHs,
                _ => HsNilScope::Barred,
            };
            let rule = HsNilRule {
                scope,
                from: if scope == HsNilScope::Barred { None } else { cell(r, from).parse().ok() },
                min_age: cell(r, age).parse().unwrap_or(0),
                note: cell(r, note),
            };
            (r[state].clone(), rule)
        })
        .collect()
});

/// A state's high school NIL rule (N3); a state missing from the data is
/// barred.
pub fn hs_nil_rule(code: &str) -> HsNilRule {
    HS_NIL_RULES.get(code).cloned().unwrap_or(HsNilRule {
        scope: HsNilScope::Barred,
        from: None,
        min_age: 0,
        note: String::new(),
    })
}

/// The first football season a state's high school players could earn NIL
/// money (minor bug B19): California first, in 2021; the year each
/// association or legislature allowed it, from the `hs_nil_from` column
/// (a ruling after August counts from the next season). None where it is
/// still barred.
pub fn hs_nil_from(code: &str) -> Option<i32> {
    if !hs_nil(code) {
        return None;
    }
    let t = Table::parse(HS_STATES_CSV);
    let col = t.header.iter().position(|h| h == "hs_nil_from")?;
    let state = t.col("state");
    t.rows.iter().find(|r| r[state] == code).and_then(|r| r.get(col)).and_then(|v| v.trim().parse().ok())
}
