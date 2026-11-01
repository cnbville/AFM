//! Names, and the real/fictional toggle chosen at new-game setup.
//!
//! People are always generated. Programs use their real names in `Real` mode;
//! `Fictional` swaps every name but keeps the same towns, sizes and strength.

use std::sync::LazyLock;

use super::ethnicity::Ethnicity;
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum NameMode {
    #[default]
    Real,
    Fictional,
}

const FIRST_NAMES_CSV: &str = include_str!("../../../../data/names/first_names.csv");
const SURNAMES_CSV: &str = include_str!("../../../../data/names/surnames.csv");
/// The Census surnames' Asian carriers by the family's origin
/// (`tools/names/asian_origins.py`, faces item F4).
const ASIAN_SURNAMES_CSV: &str = include_str!("../../../../data/names/asian_surnames.csv");

/// Where an Asian American family comes from: each origin sits in one of
/// the three Asian groups and keeps its own heritage first names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origin {
    Chinese,
    Korean,
    Japanese,
    Vietnamese,
    Filipino,
    Hmong,
    Mien,
    Cambodian,
    SouthAsian,
}

const ORIGINS: [Origin; 9] = [
    Origin::Chinese,
    Origin::Korean,
    Origin::Japanese,
    Origin::Vietnamese,
    Origin::Filipino,
    Origin::Hmong,
    Origin::Mien,
    Origin::Cambodian,
    Origin::SouthAsian,
];

impl Origin {
    fn from_key(k: &str) -> Option<Origin> {
        Some(match k {
            "chinese" => Origin::Chinese,
            "korean" => Origin::Korean,
            "japanese" => Origin::Japanese,
            "vietnamese" => Origin::Vietnamese,
            "filipino" => Origin::Filipino,
            "hmong" => Origin::Hmong,
            "mien" => Origin::Mien,
            "cambodian" => Origin::Cambodian,
            "south_asian" => Origin::SouthAsian,
            _ => return None,
        })
    }
    fn group(self) -> Ethnicity {
        match self {
            Origin::Chinese | Origin::Korean | Origin::Japanese => Ethnicity::EastAsian,
            Origin::SouthAsian => Ethnicity::SouthAsian,
            _ => Ethnicity::SoutheastAsian,
        }
    }
    /// The share of boys (born in the 2000s) who carry a heritage first name,
    /// and the names, most common first (the faces research's lists;
    /// estimates, proposed, not confirmed). The rest carry American names.
    fn heritage(self) -> (f64, &'static [&'static str]) {
        match self {
            Origin::SouthAsian => (0.6, SOUTH_ASIAN_FIRST),
            Origin::Vietnamese => (0.2, VIETNAMESE_FIRST),
            Origin::Korean => (0.06, &["Jae", "Hyun", "Joon", "Sung", "Seung", "Min", "Jin", "Dong", "Young", "Jun"]),
            Origin::Chinese => (0.05, &["Jian", "Yan", "Wei", "Hao", "Bo", "Shan", "Jun", "Eason", "Jacky"]),
            Origin::Japanese => {
                (0.08, &["Kenji", "Ryu", "Takumi", "Koji", "Kenta", "Hiro", "Shun", "Ren", "Daiki", "Kai"])
            }
            Origin::Hmong => {
                (0.1, &["Tou", "Pao", "Kou", "Fue", "Neng", "Chue", "Xeng", "Yeng", "Lue", "Cher", "Txeng"])
            }
            Origin::Filipino => {
                (0.06, &["Jericho", "Angelo", "Carlo", "Paolo", "Rafael", "Jomar", "Rommel", "Rodel", "Gian", "Jayson"])
            }
            Origin::Cambodian => (0.05, &["Sokha", "Vannak", "Visal", "Rithy", "Dara"]),
            Origin::Mien => (0.0, &[]),
        }
    }
}

const SOUTH_ASIAN_FIRST: &[&str] = &[
    "Rohan", "Aryan", "Mohammad", "Arjun", "Aditya", "Muhammad", "Nikhil", "Krish", "Pranav", "Mohammed", "Syed",
    "Ishaan", "Rishi", "Armaan", "Ajay", "Rahul", "Ayaan", "Anish", "Jai", "Aarav", "Abdullah", "Ayush", "Abhinav",
    "Sahil", "Varun", "Abdul", "Sameer", "Akash", "Yash", "Shaan", "Kiran", "Rohit", "Amar", "Dhruv", "Rayyan",
    "Akhil", "Arnav", "Ishan", "Shayan", "Sanjay", "Ayan", "Aayush", "Ashwin", "Raj", "Aarush", "Krishna", "Aaryan",
    "Kabir", "Ansh", "Vivek", "Ravi", "Shiv", "Farhan", "Karthik", "Vishnu", "Vikram", "Vishal", "Amaan", "Rehan",
    "Amit", "Tarun", "Suraj", "Sachin", "Soham", "Abhishek", "Sai", "Veer", "Roshan", "Saad", "Kush", "Saif",
    "Anirudh", "Eshaan", "Abhiram", "Anand", "Sohan", "Vijay", "Aakash", "Rishabh", "Shivam", "Adnan", "Vinay",
    "Kavin", "Surya", "Aneesh", "Yuvraj", "Vihaan", "Akshay", "Keshav", "Arav", "Arun", "Faisal", "Raghav", "Srikar",
    "Humza", "Asad", "Kishan", "Sriram", "Viraj", "Gaurav", "Zayan", "Anuj", "Naveen", "Abhay", "Manav", "Aamir",
    "Arham", "Hamzah", "Rajan", "Nitin", "Hari", "Ashish", "Anshul", "Avinash", "Pranay", "Arvind", "Navin", "Gautam",
    "Harish", "Pratham", "Asim", "Ved", "Prem", "Qasim", "Adithya", "Tenzin", "Dev", "Neil", "Sunny", "Jaskaran",
    "Gurpreet", "Harpreet", "Manpreet", "Jasdeep", "Karan", "Imran", "Bilal", "Zain", "Omar",
];

const VIETNAMESE_FIRST: &[&str] = &[
    "Minh", "Vinh", "Khang", "Khai", "Huy", "Long", "Quan", "Thien", "Nam", "Ky", "An", "Viet", "Tri", "Tam", "Khoi",
    "Tien", "Kien", "Khoa", "Dat", "Thanh", "Tuan", "Hieu", "Bao", "Duc", "Phuc", "Quang", "Trung", "Hung", "Phong",
    "Danh", "Loc",
];

/// Names carried in Hawaii and the Pacific that the first-name file reads as
/// Asian: kept out of the Asian American pools.
const NOT_ASIAN_FIRST: [&str; 4] = ["Keanu", "Kekoa", "Koa", "Kanoa"];

/// Samoan, Tongan and Hawaiian surnames (hand-picked common surnames; the
/// Census file groups Pacific Islanders with Asians, so it can't separate them).
const POLYNESIAN_SURNAMES: [&str; 48] = [
    "Tuiasosopo",
    "Fuimaono",
    "Mariota",
    "Tagovailoa",
    "Sewell",
    "Fifita",
    "Vea",
    "Taufa",
    "Tupou",
    "Fonoti",
    "Kalani",
    "Kahananui",
    "Leiataua",
    "Tuitele",
    "Faleafa",
    "Tuiloma",
    "Mauga",
    "Seau",
    "Polamalu",
    "Niumatalolo",
    "Kaufusi",
    "Havili",
    "Latu",
    "Moala",
    "Vainikolo",
    "Mata'afa",
    "Sapolu",
    "Tuimoloau",
    "Aiono",
    "Salave'a",
    "Fanene",
    "Kapisi",
    "Lealaimatafao",
    "Tafuna",
    "Toala",
    "Pouha",
    "Tuipulotu",
    "Kaho",
    "Fusitu'a",
    "Ulufonua",
    "Kealoha",
    "Kamakana",
    "Akana",
    "Nakoa",
    "Kaleo",
    "Palelei",
    "Soliai",
    "Taumoepeau",
];

/// Polynesian first names (hand-picked); most Polynesian men in the US carry
/// common American names, so these are only part of the draw.
const POLYNESIAN_FIRST: [&str; 30] = [
    "Sione", "Siaosi", "Tevita", "Viliami", "Keoni", "Kainoa", "Kekoa", "Makana", "Penei", "Sefo", "Tua", "Ioane",
    "Lui", "Malakai", "Kaimana", "Noa", "Manu", "Saia", "Sitani", "Isileli", "Paki", "Kalani", "Ofa", "Falaniko",
    "Kaleb", "Tali", "Maka", "Junior", "Semisi", "Lopeti",
];

const DECADES: [i32; 8] = [1940, 1950, 1960, 1970, 1980, 1990, 2000, 2010];

/// Cumulative weights for a weighted draw: (names, running totals).
struct Pool {
    names: Vec<&'static str>,
    cum: Vec<f64>,
}

impl Pool {
    fn new(entries: Vec<(&'static str, f64)>) -> Pool {
        let mut total = 0.0;
        let mut names = Vec::with_capacity(entries.len());
        let mut cum = Vec::with_capacity(entries.len());
        for (n, w) in entries {
            if w > 0.0 {
                total += w;
                names.push(n);
                cum.push(total);
            }
        }
        Pool { names, cum }
    }
    fn draw(&self, rng: &mut Rng) -> &'static str {
        self.names[self.index_at(rng.next())]
    }
    /// The entry a uniform number `u` in [0, 1) picks.
    fn index_at(&self, u: f64) -> usize {
        let total = *self.cum.last().unwrap_or(&1.0);
        let x = u * total;
        self.cum.partition_point(|&c| c <= x).min(self.names.len().saturating_sub(1))
    }
}

/// The Asian groups' pools: surnames tagged with the family's origin, and
/// first names per birth decade and origin.
struct AsianPools {
    /// [group]: surnames, with each entry's origin in `origin`.
    last: Vec<(Pool, Vec<Origin>)>,
    /// [decade][origin]
    first: Vec<Vec<Pool>>,
}

static ASIAN: LazyLock<AsianPools> = LazyLock::new(|| {
    let num = |s: &str| s.parse::<f64>().unwrap_or(0.0);
    let rows: Vec<Vec<&'static str>> = csv_rows(ASIAN_SURNAMES_CSV).collect();
    let last = Ethnicity::ASIAN
        .iter()
        .map(|&g| {
            let mut entries = Vec::new();
            let mut tags = Vec::new();
            for r in &rows {
                if let Some(o) = Origin::from_key(r[1]).filter(|o| o.group() == g)
                    && num(r[2]) > 0.0
                {
                    entries.push((r[0], num(r[2])));
                    tags.push(o);
                }
            }
            (Pool::new(entries), tags)
        })
        .collect();
    // American first names Asian American boys carry: real-record shares
    // only (the letter-pattern guesses are left out), without heritage names
    // and, outside Filipino families, without mostly Hispanic names.
    let frows: Vec<Vec<&'static str>> = csv_rows(FIRST_NAMES_CSV).collect();
    let heritage_names: std::collections::HashSet<&str> =
        ORIGINS.iter().flat_map(|o| o.heritage().1.iter().copied()).collect();
    let first = (0..DECADES.len())
        .map(|d| {
            ORIGINS
                .iter()
                .map(|&o| {
                    let mut general: Vec<(&'static str, f64)> = frows
                        .iter()
                        .filter(|r| r[5] != "trigram_model")
                        .filter(|r| !heritage_names.contains(r[0]) && !NOT_ASIAN_FIRST.contains(&r[0]))
                        .filter(|r| o == Origin::Filipino || num(r[3]) < 0.5)
                        .map(|r| (r[0], num(r[6 + d]) * num(r[4])))
                        .filter(|e| e.1 > 0.0)
                        .collect();
                    let (share, names) = o.heritage();
                    let total: f64 = general.iter().map(|e| e.1).sum::<f64>().max(1e-9);
                    general.iter_mut().for_each(|e| e.1 *= (1.0 - share) / total);
                    // Heritage names: more common first (weight 1 / sqrt(rank)).
                    let hw: f64 = (0..names.len()).map(|k| 1.0 / ((k + 1) as f64).sqrt()).sum::<f64>().max(1e-9);
                    general.extend(names.iter().enumerate().map(|(k, &n)| (n, share / ((k + 1) as f64).sqrt() / hw)));
                    Pool::new(general)
                })
                .collect()
        })
        .collect();
    AsianPools { last, first }
});

/// Each surname's Asian carriers by group (East, Southeast, South).
static ASIAN_SURNAME_GROUPS: LazyLock<std::collections::HashMap<&'static str, [f64; 3]>> = LazyLock::new(|| {
    let mut m: std::collections::HashMap<&'static str, [f64; 3]> = std::collections::HashMap::new();
    for r in csv_rows(ASIAN_SURNAMES_CSV) {
        if let Some(o) = Origin::from_key(r[1]) {
            m.entry(r[0]).or_default()[o.group() as usize - Ethnicity::EastAsian as usize] +=
                r[2].parse::<f64>().unwrap_or(0.0);
        }
    }
    m
});

/// The family origin most of a surname's Asian carriers have, in words
/// ("Vietnamese", "Korean"; "South Asian" for the South Asian names, which
/// don't tell India from Pakistan or Bangladesh).
pub fn asian_origin_of(surname: &str) -> Option<&'static str> {
    static BY: LazyLock<std::collections::HashMap<&'static str, (Origin, f64)>> = LazyLock::new(|| {
        let mut m: std::collections::HashMap<&'static str, (Origin, f64)> = std::collections::HashMap::new();
        for r in csv_rows(ASIAN_SURNAMES_CSV) {
            if let Some(o) = Origin::from_key(r[1]) {
                let n = r[2].parse::<f64>().unwrap_or(0.0);
                let e = m.entry(r[0]).or_insert((o, 0.0));
                if n > e.1 {
                    *e = (o, n);
                }
            }
        }
        m
    });
    let s = surname.split('-').next().unwrap_or(surname);
    BY.get(s).map(|&(o, _)| match o {
        Origin::Chinese => "Chinese",
        Origin::Korean => "Korean",
        Origin::Japanese => "Japanese",
        Origin::Vietnamese => "Vietnamese",
        Origin::Filipino => "Filipino",
        Origin::Hmong => "Hmong",
        Origin::Mien => "Mien",
        Origin::Cambodian => "Cambodian",
        Origin::SouthAsian => "South Asian",
    })
}

/// Which Asian group a surname's carriers mostly belong to, for worlds
/// written before the split. A hyphenated name reads its first part; a
/// suffix (Jr., III) is dropped.
pub fn asian_group_of(surname: &str) -> Option<Ethnicity> {
    let s = surname.split('-').next().unwrap_or(surname);
    let by = ASIAN_SURNAME_GROUPS.get(s)?;
    let k = (0..3).max_by(|&a, &b| by[a].total_cmp(&by[b]))?;
    (by[k] > 0.0).then_some(Ethnicity::ASIAN[k])
}

/// A full name's surname: everything after the first name, without a
/// suffix ("De La Cruz" in "Mark De La Cruz Jr.").
pub fn surname_of(name: &str) -> &str {
    let name = name.trim();
    let mut end = name.len();
    for suffix in [" Jr.", " Sr.", " II", " III", " IV", " V"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            end = stripped.len();
            break;
        }
    }
    let body = &name[..end];
    match body.find(' ') {
        Some(i) => body[i + 1..].trim_start(),
        None => body,
    }
}

/// Name pools by ethnicity (and birth decade for first names). First names:
/// Social Security male births per decade, split by the race shares of each
/// name (NYC births by mother's ethnicity 2011-2021, blended with Tzioumis
/// 2018). Surnames: the 2000 Census surname file with race shares.
struct NamePools {
    /// [decade][ethnicity]
    first: Vec<Vec<Pool>>,
    last: Vec<Pool>,
}

fn csv_rows(text: &'static str) -> impl Iterator<Item = Vec<&'static str>> {
    text.lines().skip(1).filter(|l| !l.is_empty()).map(|l| l.split(',').collect())
}

static POOLS: LazyLock<NamePools> = LazyLock::new(|| {
    let rows: Vec<Vec<&'static str>> = csv_rows(FIRST_NAMES_CSV).collect();
    let num = |s: &str| s.parse::<f64>().unwrap_or(0.0);
    let first = (0..DECADES.len())
        .map(|d| {
            Ethnicity::ALL
                .iter()
                .map(|&e| {
                    // The Asian groups draw from their own pools (`ASIAN`).
                    if e.is_asian() {
                        return Pool::new(Vec::new());
                    }
                    let entries = rows
                        .iter()
                        .map(|r| {
                            let births = num(r[6 + d]);
                            let (w, b, h, a) = (num(r[1]), num(r[2]), num(r[3]), num(r[4]));
                            let share = match e {
                                Ethnicity::White => w,
                                Ethnicity::Black => b,
                                Ethnicity::Hispanic => h,
                                // Common American names, as Polynesian families use them.
                                _ => 0.4 * w + 0.3 * a + 0.3 * b,
                            };
                            (r[0], births * share)
                        })
                        .collect();
                    Pool::new(entries)
                })
                .collect()
        })
        .collect();
    let srows: Vec<Vec<&'static str>> = csv_rows(SURNAMES_CSV).collect();
    let last = Ethnicity::ALL
        .iter()
        .map(|&e| {
            if e == Ethnicity::Polynesian {
                return Pool::new(POLYNESIAN_SURNAMES.iter().map(|&n| (n, 1.0)).collect());
            }
            let col = match e {
                Ethnicity::White => 2,
                Ethnicity::Black => 3,
                Ethnicity::Hispanic => 4,
                _ => return Pool::new(Vec::new()),
            };
            Pool::new(srows.iter().map(|r| (r[0], num(r[1]) * num(r[col]))).collect())
        })
        .collect();
    NamePools { first, last }
});

/// Prefixes that families write with a capital or an apostrophe inside the
/// name (DeAndre, Ja'Marr, LaDainian). The SSA file stores them flat.
const INNER_CAPS: [&str; 11] = ["De", "Da", "Ja", "La", "Le", "Ke", "Ty", "Tre", "Ra", "Sha", "Ka"];

/// A first name as families write it: "Deandre" may be DeAndre or De'Andre
/// when the rest is itself a common name (proposed, not confirmed).
fn styled_first(first: &str, ethnicity: Ethnicity, rng: &mut Rng) -> String {
    if !matches!(ethnicity, Ethnicity::Black | Ethnicity::Polynesian) {
        return first.to_string();
    }
    for pre in INNER_CAPS {
        if let Some(rest) = first.strip_prefix(pre)
            && rest.len() >= 4
            && let Some(c) = rest.chars().next()
            && is_common_first(&format!("{}{}", c.to_ascii_uppercase(), &rest[c.len_utf8()..]))
        {
            let rest_cap = format!("{}{}", c.to_ascii_uppercase(), &rest[c.len_utf8()..]);
            let x = rng.next();
            return if x < 0.7 {
                format!("{pre}{rest_cap}")
            } else if x < 0.85 {
                format!("{pre}'{rest_cap}")
            } else {
                first.to_string()
            };
        }
    }
    first.to_string()
}

/// True for a first name with at least 2,000 US boys since 1940.
pub fn is_common_first(name: &str) -> bool {
    COMMON_FIRST.contains(name)
}

static COMMON_FIRST: LazyLock<std::collections::HashSet<&'static str>> = LazyLock::new(|| {
    csv_rows(FIRST_NAMES_CSV)
        .filter(|r| r[6..].iter().map(|v| v.parse::<f64>().unwrap_or(0.0)).sum::<f64>() >= 2000.0)
        .map(|r| r[0])
        .collect()
});

/// A person's name, drawn by ethnicity and birth year, weighted by real
/// frequencies (trial 1 note 3). A white Jalen is possible but very rare.
/// The pools cover every boy's name the SSA lists since 1940 and every
/// surname the Census lists (100+ people), so repeats come at real rates.
/// On top: family suffixes (Jr., II, III), hyphenated surnames, and players
/// who go by initials (shares proposed, not confirmed).
pub fn person_name(ethnicity: Ethnicity, birth_year: i32, rng: &mut Rng) -> String {
    let pools = &*POOLS;
    let d = DECADES.iter().rposition(|&y| birth_year >= y).unwrap_or(0);
    let (first, mut last) = if ethnicity.is_asian() {
        // Same draws as before the split (one for the first name, one for
        // the surname), but the first name follows the family's origin,
        // which the surname tells.
        let asian = &*ASIAN;
        let g = ethnicity as usize - Ethnicity::EastAsian as usize;
        let u_first = rng.next();
        let (pool, tags) = &asian.last[g];
        let i = pool.index_at(rng.next());
        let origin = ORIGINS.iter().position(|&o| o == tags[i]).unwrap_or(0);
        let fp = &asian.first[d][origin];
        (fp.names[fp.index_at(u_first)].to_string(), pool.names[i].to_string())
    } else {
        let first = if ethnicity == Ethnicity::Polynesian && rng.chance(0.35) {
            rng.pick(&POLYNESIAN_FIRST).to_string()
        } else {
            styled_first(pools.first[d][ethnicity as usize].draw(rng), ethnicity, rng)
        };
        (first, pools.last[ethnicity as usize].draw(rng).to_string())
    };
    // Hyphenated surnames (Smith-Njigba, Robinson-Earl): about 1 in 100,
    // more often among Hispanic families.
    let hyphen = if ethnicity == Ethnicity::Hispanic { 0.025 } else { 0.008 };
    if rng.chance(hyphen) {
        let second = if ethnicity.is_asian() {
            ASIAN.last[ethnicity as usize - Ethnicity::EastAsian as usize].0.draw(rng)
        } else {
            pools.last[ethnicity as usize].draw(rng)
        };
        if second != last {
            last = format!("{last}-{second}");
        }
    }
    // Named after his father: Jr., II, III. More common in Black families.
    let k = if ethnicity == Ethnicity::Black { 1.6 } else { 1.0 };
    let x = rng.next();
    let suffix = if x < 0.045 * k {
        " Jr."
    } else if x < 0.057 * k {
        " II"
    } else if x < 0.065 * k {
        " III"
    } else if x < 0.066 * k {
        " IV"
    } else {
        ""
    };
    // Some go by initials (CJ, AJ, TJ): about 3 in 100.
    let shown = if rng.chance(0.03) {
        let a = first.chars().next().unwrap_or('J');
        let b = *rng.pick(&['J', 'J', 'J', 'D', 'C', 'T', 'R', 'M', 'K', 'L']);
        format!("{a}{b}")
    } else {
        first
    };
    format!("{shown} {last}{suffix}")
}

const MASCOT_WORDS: [&str; 16] = [
    "North", "South", "East", "West", "Central", "Valley", "Ridge", "Lake", "Heights", "Park", "Hills", "Grove",
    "Prairie", "Creek", "Summit", "Bay",
];

const SAINTS: [&str; 10] = [
    "St. Thomas",
    "St. Joseph's",
    "St. John's",
    "St. Michael's",
    "St. Augustine",
    "Bishop Moore",
    "Cardinal Hayes",
    "Holy Cross",
    "Trinity",
    "St. Peter's",
];

/// A generated high school's name in a town. `k` is how many schools the town already has.
pub fn high_school_name(town: &str, k: usize, private: bool, rng: &mut Rng) -> String {
    if private {
        return if rng.chance(0.5) { format!("{} ({town})", rng.pick(&SAINTS)) } else { format!("{town} Christian") };
    }
    // A town's first school is its own; a city's others are named for a
    // side of town or, as often in real big districts (Lincoln, Jefferson,
    // Lamar, Madison), for a person, each name used once.
    match k {
        0 => format!("{town} High"),
        _ if k % 2 == 1 && (k - 1) / 2 < MASCOT_WORDS.len() => format!("{town} {}", MASCOT_WORDS[(k - 1) / 2]),
        _ if k.is_multiple_of(2) && k / 2 - 1 < NAMESAKES.len() => format!("{} ({town})", NAMESAKES[k / 2 - 1]),
        _ => format!("{town} {} {}", MASCOT_WORDS[k % MASCOT_WORDS.len()], NAMESAKES[k % NAMESAKES.len()]),
    }
}

/// The people big-city high schools are most often named for.
const NAMESAKES: [&str; 24] = [
    "Lincoln",
    "Washington",
    "Jefferson",
    "Roosevelt",
    "Kennedy",
    "Madison",
    "Franklin",
    "Wilson",
    "Douglass",
    "King",
    "Edison",
    "Hamilton",
    "Monroe",
    "Grant",
    "Marshall",
    "Jackson",
    "Adams",
    "Truman",
    "Carver",
    "Eisenhower",
    "Lamar",
    "Austin",
    "Mann",
    "Whitman",
];

/// The fictional name for a real program, stable for a given world seed.
pub fn fictional_program_name(city: &str, index: usize, seed: u32) -> String {
    let mut rng = Rng::new(seed ^ (index as u32).wrapping_mul(0x9e37_79b9));
    match rng.int(0, 3) {
        0 => format!("{city} State"),
        1 => format!("University of {city}"),
        2 => format!("{city} Tech"),
        _ => format!("{city} A&M"),
    }
}

/// Every conference name the world's data carries, today's and history's,
/// in order: the fictional names are dealt out by place in this list, so
/// no two real conferences share a fictional one.
const CONFERENCES: [&str; 131] = [
    "ACC",
    "ACFC",
    "AIC",
    "ARC",
    "ASC",
    "ASUN",
    "American",
    "American West",
    "Appalachian",
    "Atlantic 10",
    "Big 12",
    "Big East",
    "Big Eight",
    "Big Sky",
    "Big South",
    "Big Ten",
    "Big West",
    "CAA",
    "CCIW",
    "CIAA",
    "CNE",
    "CSFL",
    "CUSA",
    "Centennial",
    "Colonial League",
    "Conference Carolinas",
    "ECFC",
    "EFC",
    "Empire 8",
    "Freedom",
    "Frontier",
    "G-MAC",
    "GAC",
    "GLIAC",
    "GLVC",
    "GNAC",
    "GPAC",
    "GSC",
    "Gateway",
    "Great Lakes Football",
    "Great West",
    "Gulf Star",
    "HAAC",
    "HCAC",
    "Heart",
    "Heartland Collegiate",
    "ICCAC",
    "Independent",
    "Ivy",
    "KCAC",
    "KJCCC",
    "LSC",
    "Landmark",
    "Liberty",
    "MAAC",
    "MAC",
    "MACCC",
    "MASCAC",
    "MCAC",
    "MEAC",
    "MEC",
    "MIAA",
    "MIAC",
    "MIFC",
    "MVFC",
    "MWC",
    "Metropolitan",
    "Michigan IAA",
    "Mid-Continent",
    "Mid-South",
    "Mid-States",
    "Middle Atlantic",
    "Missouri Valley",
    "Mountain West",
    "NACC",
    "NCAC",
    "NCC",
    "NCFC Bay 6",
    "NCFC Golden Coast",
    "NCFC NorCal",
    "NCFC Pacific 7",
    "NCFC Valley",
    "NE-10",
    "NEC",
    "NESCAC",
    "NEWMAC",
    "NIC",
    "NJAC",
    "NSIC",
    "NWC",
    "OAC",
    "ODAC",
    "OIC",
    "OVC-Big South",
    "Ohio Valley",
    "PAC",
    "PCAA",
    "PFL",
    "PSAC",
    "Pac-10",
    "Pac-12",
    "Patriot",
    "RMAC",
    "SAA",
    "SAC",
    "SCAC",
    "SCFA American Metro",
    "SCFA American Pacific",
    "SCFA National Central",
    "SCFA National Northern",
    "SCFA National Southern",
    "SCIAC",
    "SEC",
    "SIAC",
    "SWAC",
    "SWC",
    "SWJCFC",
    "SoCon",
    "Sooner",
    "Southland",
    "Sun",
    "Sun Belt",
    "TIAA",
    "UAC",
    "UMAC",
    "USA South",
    "WAC",
    "WIAC",
    "WVIAC",
    "Western Football",
    "Yankee",
];

/// The biggest conferences get names of their own; the rest are dealt
/// from regions and kinds (Claude's pick, lane 10: a fictional world
/// renames the leagues as well as the schools).
const BIG_CONFERENCES: [(&str, &str); 17] = [
    ("SEC", "Southern Union"),
    ("Big Ten", "Midlands Conference"),
    ("ACC", "Atlantic Seaboard Conference"),
    ("Big 12", "Plains Twelve"),
    ("Pac-12", "Pacific Rim Conference"),
    ("Pac-10", "Pacific Rim Conference"),
    ("Big East", "Eastern Collegiate League"),
    ("American", "National Collegiate Conference"),
    ("Mountain West", "Rocky Range Conference"),
    ("MAC", "Lakes and Rivers Conference"),
    ("Sun Belt", "Southern Coast Conference"),
    ("CUSA", "Crossroads Conference"),
    ("WAC", "Western Frontier Conference"),
    ("SWC", "Southwest Union"),
    ("Big Eight", "Prairie Eight"),
    ("Ivy", "Old Colonies League"),
    ("Independent", "Independent"),
];

const CONF_REGIONS: [&str; 27] = [
    "Heartland",
    "Coastal",
    "Great Plains",
    "Piedmont",
    "Prairie",
    "Lakeshore",
    "Tidewater",
    "Cascade",
    "Bluegrass",
    "Delta",
    "Keystone",
    "Gulf",
    "High Plains",
    "Ozark",
    "Allegheny",
    "Redwood",
    "Granite",
    "Sierra",
    "Mesa",
    "Riverland",
    "Northwoods",
    "Capital",
    "Chesapeake",
    "Cumberland",
    "Great Rivers",
    "Magnolia",
    "Harbor",
];

const CONF_KINDS: [&str; 6] = [
    "Conference",
    "Athletic Conference",
    "Football Conference",
    "Intercollegiate Conference",
    "Athletic Association",
    "League",
];

/// A conference's name in a fictional world.
pub fn fictional_conference(real: &str) -> String {
    if let Some(x) = BIG_CONFERENCES.iter().find(|x| x.0 == real) {
        return x.1.to_string();
    }
    let k = CONFERENCES.iter().position(|c| *c == real).unwrap_or_else(|| {
        real.bytes().fold(7usize, |h, b| h.wrapping_mul(31).wrapping_add(b as usize)) % CONFERENCES.len()
    });
    // Stride through the grid so neighbours in the list differ in both parts.
    let n = (k * 37) % (CONF_REGIONS.len() * CONF_KINDS.len());
    format!("{} {}", CONF_REGIONS[n % CONF_REGIONS.len()], CONF_KINDS[n / CONF_REGIONS.len()])
}

#[cfg(test)]
mod conference_tests {
    use super::*;

    #[test]
    fn fictional_conferences_never_collide() {
        let mut seen = std::collections::HashMap::new();
        for c in CONFERENCES {
            let f = fictional_conference(c);
            if let Some(prev) = seen.insert(f.clone(), c) {
                // The Pac-10 became the Pac-12: one league, one name.
                assert!(
                    matches!((prev, c), ("Pac-10", "Pac-12") | ("Pac-12", "Pac-10")),
                    "{prev} and {c} are both {f}"
                );
            }
        }
    }
}
