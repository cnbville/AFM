//! Names, and the real/fictional toggle chosen at new-game setup.
//!
//! People are always generated. Programs use their real names in `Real` mode;
//! `Fictional` swaps every name but keeps the same towns, sizes and strength.

use std::sync::LazyLock;

use super::ethnicity::Ethnicity;
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NameMode {
    #[default]
    Real,
    Fictional,
}

const FIRST_NAMES_CSV: &str = include_str!("../../../../data/names/first_names.csv");
const SURNAMES_CSV: &str = include_str!("../../../../data/names/surnames.csv");

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

const DECADES: [i32; 5] = [1970, 1980, 1990, 2000, 2010];

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
        let total = *self.cum.last().unwrap_or(&1.0);
        let x = rng.next() * total;
        let k = self.cum.partition_point(|&c| c <= x).min(self.names.len() - 1);
        self.names[k]
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
                    let entries = rows
                        .iter()
                        .map(|r| {
                            let births = num(r[6 + d]);
                            let (w, b, h, a) = (num(r[1]), num(r[2]), num(r[3]), num(r[4]));
                            let share = match e {
                                Ethnicity::White => w,
                                Ethnicity::Black => b,
                                Ethnicity::Hispanic => h,
                                Ethnicity::Asian => a,
                                // Common American names, as Polynesian families use them.
                                Ethnicity::Polynesian => 0.4 * w + 0.3 * a + 0.3 * b,
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
                _ => 5,
            };
            Pool::new(srows.iter().map(|r| (r[0], num(r[1]) * num(r[col]))).collect())
        })
        .collect();
    NamePools { first, last }
});

/// A person's name, drawn by ethnicity and birth year, weighted by real
/// frequencies (trial 1 note 3). A white Jalen is possible but very rare.
pub fn person_name(ethnicity: Ethnicity, birth_year: i32, rng: &mut Rng) -> String {
    let pools = &*POOLS;
    let d = DECADES.iter().rposition(|&y| birth_year >= y).unwrap_or(0);
    let first = if ethnicity == Ethnicity::Polynesian && rng.chance(0.35) {
        *rng.pick(&POLYNESIAN_FIRST)
    } else {
        pools.first[d][ethnicity as usize].draw(rng)
    };
    let last = pools.last[ethnicity as usize].draw(rng);
    // The Census file is upper case: McNeal, not Mcneal.
    if let Some(rest) = last.strip_prefix("Mc")
        && let Some(c) = rest.chars().next()
    {
        return format!("{first} Mc{}{}", c.to_ascii_uppercase(), &rest[c.len_utf8()..]);
    }
    format!("{first} {last}")
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
    match k {
        0 => format!("{town} High"),
        _ => format!("{town} {}", MASCOT_WORDS[(k - 1) % MASCOT_WORDS.len()]),
    }
}

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
