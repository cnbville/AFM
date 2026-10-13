//! Names, and the real/fictional toggle chosen at new-game setup.
//!
//! People are always generated. Programs use their real names in `Real` mode;
//! `Fictional` swaps every name but keeps the same towns, sizes and strength.

use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NameMode {
    #[default]
    Real,
    Fictional,
}

const FIRST: [&str; 64] = [
    "James", "Michael", "Marcus", "DeShawn", "Tyler", "Jalen", "Caleb", "Jordan", "Malik", "Bryce", "Cole", "Isaiah",
    "Darius", "Trey", "Xavier", "Garrett", "Jaylen", "Devin", "Andre", "Brandon", "Cameron", "Elijah", "Hunter",
    "Josiah", "Kendrick", "Lamar", "Mason", "Nate", "Owen", "Quentin", "Reggie", "Sam", "Terrell", "Tavion", "Wyatt",
    "Zion", "Austin", "Blake", "Carson", "Dante", "Eli", "Finn", "Grant", "Hayden", "Ivan", "Jace", "Kobe", "Luke",
    "Micah", "Noah", "Omar", "Peyton", "Quinn", "Rashad", "Shane", "Travis", "Ty", "Vince", "Will", "Xander", "Yusuf",
    "Zach", "Cedric", "Jamal",
];

const LAST: [&str; 64] = [
    "Smith",
    "Johnson",
    "Williams",
    "Brown",
    "Jones",
    "Davis",
    "Miller",
    "Wilson",
    "Moore",
    "Taylor",
    "Anderson",
    "Thomas",
    "Jackson",
    "White",
    "Harris",
    "Martin",
    "Thompson",
    "Robinson",
    "Clark",
    "Lewis",
    "Walker",
    "Hall",
    "Allen",
    "Young",
    "King",
    "Wright",
    "Scott",
    "Green",
    "Baker",
    "Adams",
    "Nelson",
    "Carter",
    "Mitchell",
    "Roberts",
    "Turner",
    "Phillips",
    "Campbell",
    "Parker",
    "Evans",
    "Edwards",
    "Collins",
    "Stewart",
    "Morris",
    "Rogers",
    "Reed",
    "Cook",
    "Bell",
    "Murphy",
    "Bailey",
    "Cooper",
    "Richardson",
    "Cox",
    "Howard",
    "Ward",
    "Peterson",
    "Gray",
    "Hughes",
    "Price",
    "Sanders",
    "Bennett",
    "Wood",
    "Barnes",
    "Henderson",
    "Coleman",
];

pub fn person_name(rng: &mut Rng) -> String {
    format!("{} {}", rng.pick(&FIRST), rng.pick(&LAST))
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
