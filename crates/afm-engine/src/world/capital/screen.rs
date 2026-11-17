//! The development office's wealth screening (Capital lane C1, CP3).
//!
//! The true number is hidden. Real screening reads public assets (homes,
//! stock filings, a company's last valuation) and misses debt, so it reads
//! a donor's paper wealth, not what he can give: a man stretched thin reads
//! as far richer than he is. The read is a range of words, never dollars
//! (CP39), and it narrows with a better office and with each meeting.
//! Sizes are Claude's (proposed, not confirmed).

use super::donors::Donor;
use crate::world::World;
use crate::world::data::Division;

/// The bands a screening speaks in, from the floor of each (dollars of 2026).
pub const BANDS: [(f64, &str); 6] = [
    (0.0, "gives every year"),
    (250_000.0, "could make a major gift"),
    (2_000_000.0, "could name a room"),
    (10_000_000.0, "could fund a wing"),
    (40_000_000.0, "could fund a whole building"),
    (150_000_000.0, "could carry a whole campaign"),
];

/// The office's error in logs: 0.9 at the weakest office, less 0.5 x its
/// quality, less 0.08 a meeting (up to four).
pub const ERROR_BASE: f64 = 0.9;
pub const ERROR_OFFICE: f64 = 0.5;
pub const ERROR_MET: f64 = 0.08;

/// The read on a donor.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    /// The low and high band (indexes into `BANDS`).
    pub lo: usize,
    pub hi: usize,
    /// "Could name a room, maybe fund a whole building."
    pub words: String,
}

/// How good a program's development office is, 0-1: the Power 4 0.75, the
/// rest of the FBS 0.55, FCS 0.4, below 0.3, plus 0.2 x prestige.
pub fn office(w: &World, p: usize) -> f64 {
    let prog = &w.programs[p];
    let base = match prog.division {
        Division::FBS if prog.power_conference(w.year) => 0.75,
        Division::FBS => 0.55,
        Division::FCS => 0.4,
        _ => 0.3,
    };
    (base + 0.2 * (prog.prestige - 0.5)).clamp(0.1, 0.95)
}

fn band(x: f64) -> usize {
    BANDS.iter().rposition(|b| x >= b.0).unwrap_or(0)
}

/// The words of a range of bands.
pub fn words(lo: usize, hi: usize) -> String {
    let cap = |s: &str| {
        let mut c = s.chars();
        c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
    };
    if lo == hi {
        return format!("{}.", cap(BANDS[lo].1));
    }
    let high = BANDS[hi].1.trim_start_matches("could ");
    format!("{}, maybe {high}.", cap(BANDS[lo].1))
}

/// Program `p`'s office reads donor `d` after `met` meetings with your
/// staff. The office's error for him is drawn once per school (the file it
/// keeps on him); meetings narrow the range around it.
pub fn read(w: &World, p: usize, d: &Donor, met: u16) -> Read {
    let sd = (ERROR_BASE - ERROR_OFFICE * office(w, p) - ERROR_MET * met.min(4) as f64).max(0.1);
    let u1 = super::draw(w, 9, (d.id as u64) << 20 ^ p as u64).max(1e-9);
    let u2 = super::draw(w, 10, (d.id as u64) << 20 ^ p as u64);
    let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
    let est = d.paper / super::donors::dollars(w.year) * (z * sd).exp();
    let lo = band(est * (-sd).exp());
    let hi = band(est * sd.exp());
    Read { lo, hi, words: words(lo, hi) }
}
