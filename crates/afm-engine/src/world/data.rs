//! Real-world data from `data/real-world/`, compiled into the engine.
//!
//! Every figure has a source in its CSV row or in that folder's README. Where
//! no public source exists (NIL dollars per school, for example), the world
//! models an estimate instead and says so.

use std::sync::LazyLock;

const COLLEGES_CSV: &str = include_str!("../../../../data/real-world/colleges.csv");
const HS_STATES_CSV: &str = include_str!("../../../../data/real-world/hs_states.csv");
const TALENT_CSV: &str = include_str!("../../../../data/real-world/talent_by_state.csv");
const NFL_TEAMS_CSV: &str = include_str!("../../../../data/real-world/nfl_teams.csv");

/// A college's division in the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Division {
    FBS,
    FCS,
    D2,
    JUCO,
}

impl Division {
    pub fn name(self) -> &'static str {
        match self {
            Division::FBS => "FBS",
            Division::FCS => "FCS",
            Division::D2 => "D2",
            Division::JUCO => "JUCO",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcademicTier {
    Aau,
    R1,
    R2,
    Other,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

pub static COLLEGES: LazyLock<Vec<CollegeRow>> = LazyLock::new(|| {
    let t = Table::parse(COLLEGES_CSV);
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
            division: match r[div].as_str() {
                "FBS" => Division::FBS,
                "FCS" => Division::FCS,
                _ => Division::D2,
            },
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
});

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
