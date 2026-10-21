//! Real figures behind the books (later extras topic 1). Each table names
//! its source; values no source publishes are marked "proposed, not
//! confirmed" and are the build's own (final build, Claude's pick).

/// Piecewise-linear lookup over (year, value) anchors; flat outside them,
/// or grown at `after` a year past the last anchor.
pub fn interp(anchors: &[(i32, f64)], year: i32, after: f64) -> f64 {
    let first = anchors[0];
    if year <= first.0 {
        return first.1;
    }
    for w in anchors.windows(2) {
        let (a, b) = (w[0], w[1]);
        if year <= b.0 {
            let t = (year - a.0) as f64 / (b.0 - a.0).max(1) as f64;
            return a.1 + t * (b.1 - a.1);
        }
    }
    let last = anchors[anchors.len() - 1];
    last.1 * (1.0 + after).powi(year - last.0)
}

// --- The NFL -------------------------------------------------------------------------------

/// Total NFL revenue, billions of dollars (league and team reports via
/// Forbes, Sportico and the spec: about $8.5B in 2010 and $23B in 2024;
/// COVID's empty stadiums in 2020). Years between anchors are interpolated;
/// after 2026 the league grows with its deals (below).
pub const LEAGUE_REVENUE: [(i32, f64); 19] = [
    (1984, 0.80),
    (1987, 0.95),
    (1990, 1.40),
    (1994, 1.90),
    (1998, 3.00),
    (2002, 4.40),
    (2006, 6.20),
    (2010, 8.50),
    (2014, 11.1),
    (2016, 13.0),
    (2019, 15.3),
    (2020, 12.2),
    (2021, 17.2),
    (2022, 18.6),
    (2023, 20.2),
    (2024, 23.0),
    (2025, 24.5),
    (2026, 26.0),
    (2029, 30.5),
];

/// The share of league revenue paid out equally (national TV, NFL
/// Properties, league sponsors): about 60% in 2024 from the Packers'
/// $432.6M national revenue; higher when TV dominated, and in 2020 when the
/// gates were empty.
pub const NATIONAL_SHARE: [(i32, f64); 8] =
    [(1984, 0.64), (1994, 0.62), (2004, 0.57), (2010, 0.53), (2016, 0.57), (2019, 0.58), (2020, 0.74), (2021, 0.61)];

/// After the last anchor: the current deals run through 2033, with the
/// league's opt-out after 2029. From 2030 the trend of the last three
/// deals (proposed, not confirmed: about 6% a year).
pub fn league_revenue(year: i32) -> f64 {
    if year <= 2029 {
        interp(&LEAGUE_REVENUE, year, 0.0) * 1e9
    } else {
        30.5e9 * 1.06f64.powi(year - 2029)
    }
}

pub fn national_share(year: i32) -> f64 {
    interp(&NATIONAL_SHARE, year, 0.0).min(0.64)
}

/// The average NFL ticket (Team Marketing Report's Fan Cost Index: $25.21
/// in 1991, $49.35 in 2000, $76.47 in 2010, $100.75 in 2019), then about 4%
/// a year (proposed, not confirmed).
pub const TICKET_PRICE: [(i32, f64); 10] = [
    (1984, 16.0),
    (1991, 25.2),
    (1995, 33.5),
    (2000, 49.4),
    (2005, 59.0),
    (2010, 76.5),
    (2015, 86.0),
    (2019, 100.8),
    (2022, 115.0),
    (2024, 125.0),
];

pub fn nfl_ticket(year: i32) -> f64 {
    interp(&TICKET_PRICE, year, 0.04)
}

/// Forbes value as a multiple of revenue (average club value over average
/// revenue: about 3.5 in 2000, 3.8 in 2010, 7.9 in 2024), rising slowly
/// after (proposed, not confirmed).
pub const VALUE_MULTIPLE: [(i32, f64); 8] =
    [(1984, 2.4), (1995, 2.7), (2000, 3.5), (2005, 4.0), (2010, 3.8), (2015, 5.0), (2019, 5.9), (2024, 7.9)];

pub fn value_multiple(year: i32) -> f64 {
    let v = interp(&VALUE_MULTIPLE, year, 0.0);
    if year > 2024 { (v + 0.12 * (year - 2024) as f64).min(10.5) } else { v }
}

/// Pay in the NFL relative to 2024 (head coaches about $1M in 1990 and
/// $10-20M in 2024; front offices and business staff grew with revenue).
pub fn nfl_pay_index(year: i32) -> f64 {
    interp(&[(1984, 0.06), (1990, 0.09), (2000, 0.25), (2010, 0.5), (2020, 0.85), (2024, 1.0)], year, 0.04)
}

/// A franchise's national pull (Forbes revenue ranks and TV ratings over
/// the decades), 0-1, in `league::NFL_TEAM_NAMES` order. Proposed, not
/// confirmed.
pub const BRAND: [f64; 32] = [
    0.50, 0.60, 0.65, 0.55, 0.55, 0.80, 0.45, 0.60, 1.00, 0.70, 0.55, 0.85, 0.70, 0.60, 0.45, 0.70, 0.70, 0.50, 0.70,
    0.65, 0.65, 0.85, 0.60, 0.85, 0.70, 0.80, 0.80, 0.80, 0.70, 0.55, 0.55, 0.75,
];

/// The NFL's whole-league rules on the gate (real): the visitors' share is
/// 40% of ticket money after a 15% stadium allowance, pooled and split by
/// all clubs since 2001 (paid to the visitor before that).
pub const VISITOR_SHARE: f64 = 0.40 * 0.85;

/// One stretch of a franchise's home: the stadium, its seats, the year it
/// was built or rebuilt (what its age counts from), the share of its cost
/// paid with public money and the cost in millions of dollars of the day.
/// From each stadium's public record (Wikipedia, the stadium authorities);
/// capacities are the football seating of the time.
#[derive(Clone, Copy, Debug)]
pub struct Stint {
    pub team: usize,
    pub from: i32,
    pub to: i32,
    pub name: &'static str,
    pub capacity: u32,
    pub opened: i32,
    pub public: f64,
    pub cost: f64,
}

const fn s(team: usize, from: i32, to: i32, name: &'static str, capacity: u32, opened: i32, public: f64, cost: f64) -> Stint {
    Stint { team, from, to, name, capacity, opened, public, cost }
}

const OLD: i32 = 1900;
const FAR: i32 = 9999;

pub static STADIUMS: &[Stint] = &[
    s(0, OLD, 1987, "Busch Memorial Stadium", 54_392, 1966, 0.2, 20.0),
    s(0, 1988, 2005, "Sun Devil Stadium", 73_379, 1958, 1.0, 1.0),
    s(0, 2006, FAR, "State Farm Stadium", 63_400, 2006, 0.68, 455.0),
    s(1, OLD, 1991, "Atlanta-Fulton County Stadium", 59_643, 1965, 1.0, 18.0),
    s(1, 1992, 2016, "Georgia Dome", 71_228, 1992, 1.0, 214.0),
    s(1, 2017, FAR, "Mercedes-Benz Stadium", 71_000, 2017, 0.44, 1600.0),
    s(2, 1996, 1997, "Memorial Stadium", 65_248, 1950, 1.0, 1.0),
    s(2, 1998, FAR, "M&T Bank Stadium", 71_008, 1998, 0.88, 220.0),
    s(3, OLD, 2025, "Highmark Stadium", 71_608, 1973, 1.0, 22.0),
    s(3, 2026, FAR, "New Highmark Stadium", 62_000, 2026, 0.5, 1700.0),
    s(4, 1995, 1995, "Memorial Stadium (Clemson)", 81_500, 1942, 1.0, 1.0),
    s(4, 1996, FAR, "Bank of America Stadium", 74_867, 1996, 0.2, 248.0),
    s(5, OLD, 2001, "Soldier Field", 66_944, 1924, 1.0, 1.0),
    s(5, 2002, 2002, "Memorial Stadium (Champaign)", 70_904, 1923, 1.0, 1.0),
    s(5, 2003, FAR, "Soldier Field", 61_500, 2003, 0.68, 632.0),
    s(6, OLD, 1999, "Riverfront Stadium", 60_389, 1970, 1.0, 45.0),
    s(6, 2000, FAR, "Paycor Stadium", 65_515, 2000, 0.93, 455.0),
    s(7, OLD, 1995, "Cleveland Stadium", 78_512, 1931, 1.0, 3.0),
    s(7, 1999, FAR, "Huntington Bank Field", 67_431, 1999, 0.75, 283.0),
    s(8, OLD, 2008, "Texas Stadium", 65_675, 1971, 0.7, 35.0),
    s(8, 2009, FAR, "AT&T Stadium", 80_000, 2009, 0.25, 1300.0),
    s(9, OLD, 2000, "Mile High Stadium", 76_273, 1948, 1.0, 1.0),
    s(9, 2001, FAR, "Empower Field at Mile High", 76_125, 2001, 0.75, 400.0),
    s(10, OLD, 2001, "Pontiac Silverdome", 80_311, 1975, 1.0, 56.0),
    s(10, 2002, FAR, "Ford Field", 65_000, 2002, 0.25, 500.0),
    s(11, OLD, 2002, "Lambeau Field", 60_890, 1957, 1.0, 1.0),
    s(11, 2003, FAR, "Lambeau Field", 81_441, 2003, 0.57, 295.0),
    s(12, 2002, FAR, "NRG Stadium", 72_220, 2002, 0.6, 352.0),
    s(13, OLD, 1983, "Memorial Stadium", 60_020, 1950, 1.0, 1.0),
    s(13, 1984, 2007, "RCA Dome", 57_980, 1984, 1.0, 78.0),
    s(13, 2008, FAR, "Lucas Oil Stadium", 63_000, 2008, 0.86, 720.0),
    s(14, 1995, FAR, "EverBank Stadium", 67_814, 1995, 1.0, 121.0),
    s(15, OLD, 2009, "Arrowhead Stadium", 79_451, 1972, 1.0, 43.0),
    s(15, 2010, FAR, "GEHA Field at Arrowhead Stadium", 76_416, 2010, 0.67, 375.0),
    s(16, OLD, 1994, "Los Angeles Memorial Coliseum", 92_516, 1923, 1.0, 1.0),
    s(16, 1995, 2019, "Oakland Coliseum", 56_057, 1996, 1.0, 200.0),
    s(16, 2020, FAR, "Allegiant Stadium", 65_000, 2020, 0.39, 1900.0),
    s(17, OLD, 2016, "Qualcomm Stadium", 70_561, 1967, 1.0, 27.0),
    s(17, 2017, 2019, "Dignity Health Sports Park", 27_000, 2003, 0.0, 150.0),
    s(17, 2020, FAR, "SoFi Stadium", 70_240, 2020, 0.0, 5500.0),
    s(18, OLD, 1994, "Anaheim Stadium", 69_008, 1966, 1.0, 24.0),
    s(18, 1995, 2015, "Edward Jones Dome", 66_000, 1995, 1.0, 280.0),
    s(18, 2016, 2019, "Los Angeles Memorial Coliseum", 93_607, 1923, 1.0, 1.0),
    s(18, 2020, FAR, "SoFi Stadium", 70_240, 2020, 0.0, 5500.0),
    s(19, OLD, 2015, "Joe Robbie Stadium", 75_192, 1987, 0.0, 115.0),
    s(19, 2016, FAR, "Hard Rock Stadium", 64_767, 2016, 0.0, 500.0),
    s(20, OLD, 2013, "Hubert H. Humphrey Metrodome", 64_121, 1982, 1.0, 55.0),
    s(20, 2014, 2015, "TCF Bank Stadium", 52_525, 2009, 1.0, 1.0),
    s(20, 2016, FAR, "U.S. Bank Stadium", 66_655, 2016, 0.45, 1100.0),
    s(21, OLD, 2001, "Foxboro Stadium", 60_292, 1971, 0.0, 7.0),
    s(21, 2002, FAR, "Gillette Stadium", 65_878, 2002, 0.0, 325.0),
    s(22, OLD, 2005, "Louisiana Superdome", 69_703, 1975, 1.0, 163.0),
    s(22, 2006, 2023, "Louisiana Superdome", 73_208, 2006, 1.0, 185.0),
    s(22, 2024, FAR, "Caesars Superdome", 73_208, 2024, 0.8, 560.0),
    s(23, OLD, 2009, "Giants Stadium", 80_242, 1976, 1.0, 78.0),
    s(23, 2010, FAR, "MetLife Stadium", 82_500, 2010, 0.0, 1600.0),
    s(24, OLD, 1983, "Shea Stadium", 60_372, 1964, 1.0, 1.0),
    s(24, 1984, 2009, "Giants Stadium", 80_242, 1976, 1.0, 78.0),
    s(24, 2010, FAR, "MetLife Stadium", 82_500, 2010, 0.0, 1600.0),
    s(25, OLD, 2002, "Veterans Stadium", 65_352, 1971, 1.0, 50.0),
    s(25, 2003, FAR, "Lincoln Financial Field", 69_796, 2003, 0.37, 512.0),
    s(26, OLD, 2000, "Three Rivers Stadium", 59_000, 1970, 1.0, 55.0),
    s(26, 2001, FAR, "Acrisure Stadium", 68_400, 2001, 0.6, 281.0),
    s(27, OLD, 2013, "Candlestick Park", 69_732, 1960, 1.0, 25.0),
    s(27, 2014, FAR, "Levi's Stadium", 68_500, 2014, 0.09, 1300.0),
    s(28, OLD, 1999, "Kingdome", 66_000, 1976, 1.0, 67.0),
    s(28, 2000, 2001, "Husky Stadium", 72_500, 1920, 1.0, 1.0),
    s(28, 2002, FAR, "Lumen Field", 68_740, 2002, 0.7, 430.0),
    s(29, OLD, 1997, "Tampa Stadium", 74_301, 1967, 1.0, 4.0),
    s(29, 1998, FAR, "Raymond James Stadium", 65_618, 1998, 1.0, 168.0),
    s(30, OLD, 1996, "Astrodome", 62_439, 1965, 1.0, 35.0),
    s(30, 1997, 1997, "Liberty Bowl Memorial Stadium", 62_380, 1965, 1.0, 1.0),
    s(30, 1998, 1998, "Vanderbilt Stadium", 41_000, 1981, 1.0, 1.0),
    s(30, 1999, FAR, "Nissan Stadium", 69_143, 1999, 0.76, 290.0),
    s(31, OLD, 1996, "RFK Stadium", 56_692, 1961, 1.0, 24.0),
    s(31, 1997, 2011, "FedExField", 85_000, 1997, 0.25, 250.0),
    s(31, 2012, 2021, "FedExField", 79_000, 1997, 0.25, 250.0),
    s(31, 2022, FAR, "Northwest Stadium", 62_000, 1997, 0.25, 250.0),
];

/// The stadium a franchise played in that season (real through 2026).
pub fn stint(team: usize, year: i32) -> Option<&'static Stint> {
    STADIUMS.iter().find(|x| x.team == team && x.from <= year && year <= x.to)
}

/// Stadium projects under way when the world takes over in 2026 (real):
/// team, the stadium's name, seats, the season it opens, cost and public
/// money in millions, and whether it is a renovation of the same building.
pub const PROJECTS_2026: [(usize, &str, u32, i32, f64, f64, bool); 4] = [
    // The Titans' new enclosed stadium beside the old one: $2.1B, $1.26B public.
    (30, "New Nissan Stadium", 60_000, 2027, 2100.0, 1260.0, false),
    // Jacksonville's "stadium of the future" rebuild: $1.4B, the city $775M.
    (14, "EverBank Stadium", 62_000, 2028, 1400.0, 775.0, true),
    // The Browns' domed stadium in Brook Park: $2.4B, Ohio $600M.
    (7, "Brook Park Stadium", 65_000, 2029, 2400.0, 600.0, false),
    // The Commanders' stadium on the RFK site: $3.7B, the District about $1.1B.
    (31, "Commanders Stadium at RFK", 65_000, 2030, 3700.0, 1100.0, false),
];

/// Cities without a team that real bids and studies have named, with
/// their state, a market size like a team's rank (1 New York) and the
/// public money they put up front for a stadium (share of cost).
/// Proposed, not confirmed.
pub const OPEN_CITIES: [(&str, &str, u32, f64); 8] = [
    ("San Antonio", "TX", 31, 0.45),
    ("St. Louis", "MO", 23, 0.40),
    ("San Diego", "CA", 28, 0.30),
    ("Portland", "OR", 22, 0.35),
    ("Salt Lake City", "UT", 30, 0.40),
    ("Oakland", "CA", 10, 0.25),
    ("Austin", "TX", 35, 0.35),
    ("Orlando", "FL", 17, 0.40),
];

/// The relocation fee owners set (real: the Rams $550M in 2016, the
/// Chargers $650M in 2017, the Raiders $378M in 2017), grown with values.
pub fn relocation_fee(year: i32, market_rank: u32) -> f64 {
    let base = if market_rank <= 5 { 650e6 } else { 450e6 };
    base * value_multiple(year) / value_multiple(2017) * league_revenue(year) / league_revenue(2017)
}

/// The NFLPA team report card's categories (2024 survey), in order.
pub const CARD_PARTS: [&str; 11] = [
    "Treatment of families",
    "Food and dining",
    "Nutritionist",
    "Locker room",
    "Training room",
    "Training staff",
    "Weight room",
    "Strength coaches",
    "Team travel",
    "Head coach",
    "Team owner",
];

/// The grades as the survey writes them, worst first.
pub const GRADES: [&str; 13] = ["F-", "F", "D-", "D", "D+", "C-", "C", "C+", "B-", "B", "B+", "A-", "A+"];

/// The survey's first year and its last public one (the NFLPA published it
/// in 2023 and 2024; an arbitrator's ruling in 2025 kept it internal).
pub const CARDS_FROM: i32 = 2023;
pub const CARDS_PUBLIC_TO: i32 = 2024;

// --- College -------------------------------------------------------------------------------

/// College athletics money relative to 2024 (the Knight Commission and
/// USA Today's NCAA finances: Division I revenue roughly tripled from 2000
/// to 2024; COVID cut 2020-21), then 4% a year (proposed, not confirmed).
pub fn college_index(year: i32) -> f64 {
    interp(
        &[
            (1984, 0.12),
            (1990, 0.18),
            (2000, 0.33),
            (2005, 0.45),
            (2010, 0.60),
            (2015, 0.75),
            (2019, 0.88),
            (2020, 0.70),
            (2021, 0.85),
            (2024, 1.0),
        ],
        year,
        0.04,
    )
}

/// Football coaches' pay relative to 2024 (top head coaches about $0.4M in
/// 1990, $1.5M in 2000, $4M in 2007, $11M in 2024).
pub fn coach_index(year: i32) -> f64 {
    interp(&[(1984, 0.025), (1990, 0.04), (2000, 0.13), (2007, 0.35), (2015, 0.65), (2024, 1.0)], year, 0.04)
}

/// What a school's conference paid each member in fiscal 2024, millions
/// (conference tax filings via USA Today and the spec: the Big Ten about
/// $60M, the SEC $52M, Group of 5 conferences a few million; FCS and below
/// mostly NCAA units). Old conferences by their peers.
pub fn conference_2024(conf: &str) -> (f64, bool) {
    match conf {
        "Big Ten" => (60.0, true),
        "SEC" => (52.0, true),
        "Big 12" | "Big Eight" | "SWC" => (44.0, true),
        "ACC" => (44.0, true),
        "Pac-12" | "Pac-10" => (37.0, true),
        "Big East" => (24.0, true),
        "American" => (7.0, false),
        "Mountain West" => (4.5, false),
        "WAC" | "Big West" | "PCAA" => (3.0, false),
        "Sun Belt" | "MAC" => (2.5, false),
        "CUSA" => (1.6, false),
        _ => (0.0, false),
    }
}

/// How a power conference's payout grew to 2024 (SEC per school: about
/// $9M in 2000, $17M in 2009, $31M with its network in 2015, $45M in 2020;
/// the new ESPN and Big Ten deals lift 2025-26).
pub fn power_curve(year: i32) -> f64 {
    interp(
        &[
            (1984, 0.02),
            (1990, 0.05),
            (1995, 0.10),
            (2000, 0.17),
            (2005, 0.21),
            (2009, 0.33),
            (2014, 0.42),
            (2015, 0.60),
            (2019, 0.86),
            (2020, 0.88),
            (2022, 0.96),
            (2024, 1.0),
            (2025, 1.22),
            (2026, 1.30),
        ],
        year,
        0.05,
    )
}

/// The Group of 5 and FCS curve (the playoff's Group of 5 share from 2014).
pub fn small_curve(year: i32) -> f64 {
    interp(&[(1984, 0.15), (1995, 0.3), (2005, 0.5), (2014, 0.8), (2020, 0.95), (2024, 1.0)], year, 0.03)
}

/// Per-school conference money in a season, dollars, from the real curves
/// (through 2026; the world's own deals after).
pub fn conference_payout(conf: &str, division: super::super::data::Division, year: i32) -> f64 {
    use super::super::data::Division;
    let (m, power) = conference_2024(conf);
    let base = if m > 0.0 {
        m
    } else {
        match division {
            Division::FBS => 1.5,
            Division::FCS => 0.6,
            Division::D2 => 0.08,
            Division::NAIA | Division::D3 => 0.02,
            Division::JUCO => 0.0,
        }
    };
    // Notre Dame's NBC deal and its playoff access (about $25M in 2024).
    let base = if conf == "Independent" && division == Division::FBS { base.max(2.5) } else { base };
    base * 1e6 * if power { power_curve(year) } else { small_curve(year) }
}

/// The NCAA's yearly distributions to a Division I school (basketball
/// tournament units, the grant-in-aid and sport-sponsorship funds): about
/// $1.5-4M in 2024 by basketball strength; Division II gets a little.
pub fn ncaa_distribution(division: super::super::data::Division, hoops: f64, year: i32) -> f64 {
    use super::super::data::Division;
    let base = match division {
        Division::FBS | Division::FCS => 1.4e6 + 2.6e6 * hoops,
        Division::D2 => 0.1e6,
        _ => 0.02e6,
    };
    base * college_index(year)
}

/// Guarantee game prices (real: about $300,000-700,000 for an FCS team in
/// the 2020s, $1-2M for a Group of 5 team; a fifth of that in 1990).
pub fn guarantee(visitor_tier: u8, year: i32) -> f64 {
    let now = match visitor_tier {
        0 => 1.6e6,
        1 => 1.2e6,
        2 => 0.5e6,
        _ => 0.12e6,
    };
    now * interp(&[(1984, 0.15), (1995, 0.3), (2005, 0.5), (2015, 0.75), (2024, 1.0)], year, 0.03)
}

/// The House settlement's revenue sharing cap ($20.5M in 2025-26, 4% a
/// year) and the yearly back-damages repayment a Power 4 school carries
/// (about $2.8B over ten years from NCAA distributions; Power 4 schools
/// about $2-3M a year, others less).
pub const HOUSE_START: i32 = 2025;
pub fn share_cap(year: i32) -> f64 {
    if year < HOUSE_START { 0.0 } else { 20.5e6 * 1.04f64.powi(year - HOUSE_START) }
}

// --- High school ---------------------------------------------------------------------------

/// High school prices relative to 2024: a $7 ticket in 2024, about $3 in
/// 1990 (proposed, not confirmed).
pub fn hs_index(year: i32) -> f64 {
    interp(&[(1984, 0.35), (1990, 0.42), (2000, 0.58), (2010, 0.78), (2024, 1.0)], year, 0.03)
}

/// The share of a state's schools that charge players a fee to play
/// (state surveys and the University of Michigan's 2024 poll: common in
/// Ohio, Michigan, Massachusetts, Colorado and the Northwest; barred in
/// California's public schools; rare in Texas and the South). Proposed,
/// not confirmed where no survey exists.
pub fn pay_to_play_share(state: &str) -> f64 {
    match state {
        "MA" => 0.6,
        "MI" | "OR" | "CO" | "MN" => 0.5,
        "OH" => 0.45,
        "WI" | "UT" | "WA" | "ID" => 0.4,
        "PA" | "IN" | "IL" | "CT" | "AZ" | "NH" | "ME" | "VT" | "MT" => 0.3,
        "NJ" => 0.25,
        "KS" | "IA" | "NM" | "NE" | "WY" | "SD" | "ND" | "AK" => 0.2,
        "MO" | "NV" => 0.15,
        "NY" | "MD" | "VA" | "DE" | "RI" | "HI" => 0.1,
        "CA" | "TX" => 0.0,
        _ => 0.05,
    }
}

/// Fundraisers a high school staff runs (what real booster clubs do), with
/// what each takes in a good community (2024 dollars) and the weeks of a
/// coach's time it eats.
#[derive(Clone, Copy, Debug)]
pub struct Fundraiser {
    pub key: &'static str,
    pub name: &'static str,
    pub what: &'static str,
    pub base: f64,
    pub weeks: u8,
}

pub const FUNDRAISERS: [Fundraiser; 6] = [
    Fundraiser {
        key: "cards",
        name: "Discount card drive",
        what: "Players sell local discount cards door to door.",
        base: 9_000.0,
        weeks: 2,
    },
    Fundraiser {
        key: "liftathon",
        name: "Lift-a-thon",
        what: "Pledges per pound lifted in the weight room.",
        base: 6_500.0,
        weeks: 1,
    },
    Fundraiser {
        key: "golf",
        name: "Golf outing",
        what: "A summer scramble with sponsors at every hole.",
        base: 14_000.0,
        weeks: 3,
    },
    Fundraiser {
        key: "banquet",
        name: "Banquet and auction",
        what: "Dinner, a silent auction and a former player as speaker.",
        base: 18_000.0,
        weeks: 3,
    },
    Fundraiser {
        key: "carwash",
        name: "Car wash and cookout",
        what: "A Saturday at the school lot.",
        base: 2_500.0,
        weeks: 1,
    },
    Fundraiser {
        key: "ads",
        name: "Program ads and banners",
        what: "Local businesses buy ads in the program and banners on the fence.",
        base: 11_000.0,
        weeks: 2,
    },
];

/// What a high school program can buy (2024 prices from equipment
/// catalogs, proposed, not confirmed), and what it does for the program.
#[derive(Clone, Copy, Debug)]
pub struct Purchase {
    pub key: &'static str,
    pub name: &'static str,
    pub what: &'static str,
    pub price: f64,
    /// How much the school's resources (weight room, film, staff support) rise.
    pub resources: f64,
}

pub const PURCHASES: [Purchase; 7] = [
    Purchase {
        key: "helmets",
        name: "New helmets (a full set)",
        what: "Replaces every helmet past ten years or failing recertification.",
        price: 32_000.0,
        resources: 0.0,
    },
    Purchase {
        key: "recert",
        name: "Recondition and recertify helmets",
        what: "NOCSAE reconditioning, about $50 a helmet; due every year or two.",
        price: 4_000.0,
        resources: 0.0,
    },
    Purchase {
        key: "video",
        name: "Film system",
        what: "A sideline camera tower, end-zone camera and a film-sharing subscription.",
        price: 9_500.0,
        resources: 0.035,
    },
    Purchase {
        key: "sled",
        name: "Blocking sleds and bags",
        what: "A seven-man sled, chutes and tackling wheels.",
        price: 7_000.0,
        resources: 0.015,
    },
    Purchase {
        key: "weights",
        name: "Weight room upgrade",
        what: "Racks, platforms and bars for a full team lift.",
        price: 28_000.0,
        resources: 0.04,
    },
    Purchase {
        key: "uniforms",
        name: "New uniforms",
        what: "Home and away sets; the town notices.",
        price: 16_000.0,
        resources: 0.005,
    },
    Purchase {
        key: "meals",
        name: "Team meals for the season",
        what: "A pregame meal every Friday.",
        price: 6_000.0,
        resources: 0.01,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_franchise_has_a_home_each_year_it_plays() {
        for t in 0..32 {
            for y in 1984..=2026 {
                if crate::world::franchise::active(t, y) {
                    assert!(stint(t, y).is_some(), "team {t} year {y}");
                }
            }
        }
        assert_eq!(stint(18, 2016).unwrap().name, "Los Angeles Memorial Coliseum");
        assert_eq!(stint(3, 2026).unwrap().opened, 2026);
    }

    #[test]
    fn the_curves_hit_their_anchors() {
        assert!((league_revenue(2010) - 8.5e9).abs() < 1.0);
        assert!((league_revenue(2024) - 23.0e9).abs() < 1.0);
        assert!(league_revenue(2035) > league_revenue(2029));
        assert!((nfl_ticket(2000) - 49.4).abs() < 0.01);
        let sec = conference_payout("SEC", crate::world::data::Division::FBS, 2024);
        assert!((sec - 52.0e6).abs() < 1.0);
        assert!(guarantee(2, 2024) > guarantee(2, 1990));
    }
}
