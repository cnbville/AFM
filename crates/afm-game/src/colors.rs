//! Team colours for Showtime's `accent` and `accent-2` (section 10, topic 2:
//! the subject's own colours light the screen). Real colours for the NFL
//! and the FBS programs best known for them; every other team gets a
//! school pair drawn from the colours American schools really use, fixed
//! by its id so it never changes.

/// NFL clubs by name: (primary, secondary).
const NFL: &[(&str, &str, &str)] = &[
    ("Arizona Cardinals", "#97233F", "#FFB612"),
    ("Atlanta Falcons", "#A71930", "#A5ACAF"),
    ("Baltimore Ravens", "#241773", "#9E7C0C"),
    ("Buffalo Bills", "#00338D", "#C60C30"),
    ("Carolina Panthers", "#0085CA", "#BFC0BF"),
    ("Chicago Bears", "#0B162A", "#C83803"),
    ("Cincinnati Bengals", "#FB4F14", "#101820"),
    ("Cleveland Browns", "#311D00", "#FF3C00"),
    ("Dallas Cowboys", "#041E42", "#869397"),
    ("Denver Broncos", "#FB4F14", "#002244"),
    ("Detroit Lions", "#0076B6", "#B0B7BC"),
    ("Green Bay Packers", "#203731", "#FFB612"),
    ("Houston Texans", "#03202F", "#A71930"),
    ("Indianapolis Colts", "#002C5F", "#A2AAAD"),
    ("Jacksonville Jaguars", "#006778", "#D7A22A"),
    ("Kansas City Chiefs", "#E31837", "#FFB81C"),
    ("Las Vegas Raiders", "#000000", "#A5ACAF"),
    ("Oakland Raiders", "#000000", "#A5ACAF"),
    ("Los Angeles Raiders", "#000000", "#A5ACAF"),
    ("Los Angeles Chargers", "#0080C6", "#FFC20E"),
    ("San Diego Chargers", "#0080C6", "#FFC20E"),
    ("Los Angeles Rams", "#003594", "#FFA300"),
    ("St. Louis Rams", "#002244", "#B3995D"),
    ("Miami Dolphins", "#008E97", "#FC4C02"),
    ("Minnesota Vikings", "#4F2683", "#FFC62F"),
    ("New England Patriots", "#002244", "#C60C30"),
    ("New Orleans Saints", "#101820", "#D3BC8D"),
    ("New York Giants", "#0B2265", "#A71930"),
    ("New York Jets", "#125740", "#FFFFFF"),
    ("Philadelphia Eagles", "#004C54", "#A5ACAF"),
    ("Pittsburgh Steelers", "#101820", "#FFB612"),
    ("San Francisco 49ers", "#AA0000", "#B3995D"),
    ("Seattle Seahawks", "#002244", "#69BE28"),
    ("Tampa Bay Buccaneers", "#D50A0A", "#34302B"),
    ("Tennessee Titans", "#0C2340", "#4B92DB"),
    ("Tennessee Oilers", "#0C2340", "#4B92DB"),
    ("Houston Oilers", "#003C7E", "#C8102E"),
    ("Washington Commanders", "#5A1414", "#FFB612"),
    ("Washington Football Team", "#5A1414", "#FFB612"),
    ("Washington Redskins", "#5A1414", "#FFB612"),
];

/// FBS programs by name.
const COLLEGE: &[(&str, &str, &str)] = &[
    ("Air Force", "#003087", "#8A8D8F"),
    ("Alabama", "#9E1B32", "#828A8F"),
    ("Arizona", "#CC0033", "#003366"),
    ("Arizona State", "#8C1D40", "#FFC627"),
    ("Arkansas", "#9D2235", "#FFFFFF"),
    ("Army", "#000000", "#D3BC8D"),
    ("Auburn", "#0C2340", "#E87722"),
    ("Baylor", "#154734", "#FFB81C"),
    ("Boise State", "#0033A0", "#D64309"),
    ("Boston College", "#8A100B", "#B29D6C"),
    ("BYU", "#002E5D", "#FFFFFF"),
    ("California", "#003262", "#FDB515"),
    ("Cincinnati", "#E00122", "#000000"),
    ("Clemson", "#F56600", "#522D80"),
    ("Colorado", "#000000", "#CFB87C"),
    ("Duke", "#003087", "#FFFFFF"),
    ("Florida", "#0021A5", "#FA4616"),
    ("Florida State", "#782F40", "#CEB888"),
    ("Georgia", "#BA0C2F", "#000000"),
    ("Georgia Tech", "#B3A369", "#003057"),
    ("Hawaii", "#024731", "#FFFFFF"),
    ("Houston", "#C8102E", "#FFFFFF"),
    ("Illinois", "#E84A27", "#13294B"),
    ("Indiana", "#990000", "#EEEDEB"),
    ("Iowa", "#000000", "#FFCD00"),
    ("Iowa State", "#C8102E", "#F1BE48"),
    ("Kansas", "#0051BA", "#E8000D"),
    ("Kansas State", "#512888", "#D1D1D1"),
    ("Kentucky", "#0033A0", "#FFFFFF"),
    ("Louisville", "#AD0000", "#000000"),
    ("LSU", "#461D7C", "#FDD023"),
    ("Maryland", "#E21833", "#FFD200"),
    ("Memphis", "#003087", "#898D8D"),
    ("Miami (FL)", "#F47321", "#005030"),
    ("Michigan", "#00274C", "#FFCB05"),
    ("Michigan State", "#18453B", "#FFFFFF"),
    ("Minnesota", "#7A0019", "#FFCC33"),
    ("Mississippi State", "#660000", "#FFFFFF"),
    ("Missouri", "#000000", "#F1B82D"),
    ("Navy", "#00205B", "#C5B783"),
    ("NC State", "#CC0000", "#000000"),
    ("Nebraska", "#E41C38", "#FDF2D9"),
    ("North Carolina", "#7BAFD4", "#13294B"),
    ("Northwestern", "#4E2A84", "#FFFFFF"),
    ("Notre Dame", "#0C2340", "#C99700"),
    ("Ohio State", "#BB0000", "#666666"),
    ("Oklahoma", "#841617", "#FDF9D8"),
    ("Oklahoma State", "#FF7300", "#000000"),
    ("Ole Miss", "#CE1126", "#14213D"),
    ("Oregon", "#154733", "#FEE123"),
    ("Oregon State", "#DC4405", "#000000"),
    ("Penn State", "#041E42", "#FFFFFF"),
    ("Pittsburgh", "#003594", "#FFB81C"),
    ("Purdue", "#CEB888", "#000000"),
    ("Rutgers", "#CC0033", "#5F6A72"),
    ("SMU", "#0033A0", "#C8102E"),
    ("South Carolina", "#73000A", "#000000"),
    ("Stanford", "#8C1515", "#FFFFFF"),
    ("Syracuse", "#F76900", "#000E54"),
    ("TCU", "#4D1979", "#A3A9AC"),
    ("Tennessee", "#FF8200", "#FFFFFF"),
    ("Texas", "#BF5700", "#FFFFFF"),
    ("Texas A&M", "#500000", "#FFFFFF"),
    ("Texas Tech", "#CC0000", "#000000"),
    ("Tulane", "#006747", "#418FDE"),
    ("UCF", "#BA9B37", "#000000"),
    ("UCLA", "#2D68C4", "#F2A900"),
    ("USC", "#990000", "#FFCC00"),
    ("Utah", "#CC0000", "#FFFFFF"),
    ("Vanderbilt", "#866D4B", "#000000"),
    ("Virginia", "#232D4B", "#F84C1E"),
    ("Virginia Tech", "#630031", "#CF4420"),
    ("Wake Forest", "#9E7E38", "#000000"),
    ("Washington", "#4B2E83", "#B7A57A"),
    ("Washington State", "#981E32", "#5E6A71"),
    ("West Virginia", "#002855", "#EAAA00"),
    ("Wisconsin", "#C5050C", "#FFFFFF"),
    ("Appalachian State", "#222222", "#FFCC00"),
    ("Marshall", "#00B140", "#FFFFFF"),
    ("James Madison", "#450084", "#CBB677"),
    ("Liberty", "#002D62", "#C41230"),
    ("Toledo", "#15397F", "#FFDA00"),
    ("Tulsa", "#002D72", "#C8102E"),
    ("UNLV", "#CF0A2C", "#666666"),
    ("San Diego State", "#A6192E", "#000000"),
    ("Fresno State", "#DB0032", "#002E6D"),
    ("North Dakota State", "#0A5640", "#FFC72A"),
    ("Montana", "#70002E", "#999999"),
    ("South Dakota State", "#0033A0", "#FFD100"),
];

/// Pairs American school teams really wear, for teams without their own entry.
const SCHOOL: &[(&str, &str)] = &[
    ("#7A0019", "#FFC72C"),
    ("#002D72", "#C8102E"),
    ("#00553D", "#FFC72C"),
    ("#4B2E83", "#F2C75C"),
    ("#C8102E", "#1A1A1A"),
    ("#003A70", "#FFFFFF"),
    ("#E35205", "#1A1A1A"),
    ("#8C1D40", "#C5B358"),
    ("#0C2340", "#FFB81C"),
    ("#006747", "#FFFFFF"),
    ("#5F259F", "#FFFFFF"),
    ("#B5121B", "#FFFFFF"),
    ("#1F3B73", "#8A8D8F"),
    ("#000000", "#C99700"),
    ("#00843D", "#FFD100"),
    ("#862633", "#B9975B"),
];

fn find(table: &[(&str, &str, &str)], name: &str) -> Option<[String; 2]> {
    table.iter().find(|t| t.0 == name).map(|t| [t.1.to_string(), t.2.to_string()])
}

fn school_pair(key: u64) -> [String; 2] {
    let k = (key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) as usize % SCHOOL.len();
    [SCHOOL[k].0.to_string(), SCHOOL[k].1.to_string()]
}

pub fn nfl(name: &str, id: usize) -> [String; 2] {
    find(NFL, name).unwrap_or_else(|| school_pair(0xAF ^ id as u64))
}

pub fn college(real_name: &str, id: usize) -> [String; 2] {
    find(COLLEGE, real_name).unwrap_or_else(|| school_pair(0xC0 ^ id as u64))
}

/// Other leagues' clubs by name: the CFL's and the best known of the rest.
const OTHER: &[(&str, &str, &str)] = &[
    ("BC Lions", "#F15A22", "#000000"),
    ("Calgary Stampeders", "#C8102E", "#000000"),
    ("Edmonton Eskimos", "#2B5134", "#FFB819"),
    ("Edmonton Elks", "#2B5134", "#FFB819"),
    ("Saskatchewan Roughriders", "#00633E", "#A5ACAF"),
    ("Winnipeg Blue Bombers", "#041E42", "#B9975B"),
    ("Hamilton Tiger-Cats", "#FFB819", "#000000"),
    ("Toronto Argonauts", "#0C2340", "#6CACE4"),
    ("Ottawa Rough Riders", "#C8102E", "#000000"),
    ("Ottawa Renegades", "#CE1126", "#000000"),
    ("Ottawa Redblacks", "#C8102E", "#000000"),
    ("Montreal Alouettes", "#002D72", "#C8102E"),
    ("Amsterdam Admirals", "#F47920", "#00205B"),
    ("Los Angeles KISS", "#000000", "#A5ACAF"),
    ("San Antonio Brahmas", "#000000", "#C99700"),
    ("DC Defenders", "#C8102E", "#041E42"),
    ("Birmingham Stallions", "#9E1B32", "#FFB81C"),
];

/// A club in another league: its real colours when known, else a pair
/// fixed by its franchise.
pub fn other(name: &str, fr: usize) -> [String; 2] {
    find(OTHER, name).unwrap_or_else(|| school_pair(0x07 ^ (fr as u64) << 12))
}

pub fn high_school(id: usize) -> [String; 2] {
    school_pair(0x45 ^ (id as u64) << 8)
}
