//! Where things are: state positions for distance from home and recruiting reach.
//!
//! Positions are approximate state centers (latitude, longitude), close enough
//! for "how far from home" and "is this inside a staff's usual territory".

pub const STATE_CENTERS: [(&str, f64, f64); 51] = [
    ("AL", 32.8, -86.8),
    ("AK", 61.4, -152.3),
    ("AZ", 34.2, -111.7),
    ("AR", 34.9, -92.4),
    ("CA", 37.2, -119.5),
    ("CO", 39.0, -105.5),
    ("CT", 41.6, -72.7),
    ("DE", 39.0, -75.5),
    ("DC", 38.9, -77.0),
    ("FL", 28.6, -82.4),
    ("GA", 32.7, -83.4),
    ("HI", 20.8, -156.3),
    ("ID", 44.4, -114.6),
    ("IL", 40.0, -89.2),
    ("IN", 39.9, -86.3),
    ("IA", 42.1, -93.5),
    ("KS", 38.5, -98.4),
    ("KY", 37.5, -85.3),
    ("LA", 31.1, -92.0),
    ("ME", 45.4, -69.2),
    ("MD", 39.0, -76.8),
    ("MA", 42.3, -71.8),
    ("MI", 44.3, -85.4),
    ("MN", 46.3, -94.3),
    ("MS", 32.7, -89.7),
    ("MO", 38.4, -92.5),
    ("MT", 47.0, -109.6),
    ("NE", 41.5, -99.8),
    ("NV", 39.3, -116.6),
    ("NH", 43.7, -71.6),
    ("NJ", 40.2, -74.7),
    ("NM", 34.4, -106.1),
    ("NY", 42.9, -75.5),
    ("NC", 35.6, -79.4),
    ("ND", 47.5, -100.5),
    ("OH", 40.3, -82.8),
    ("OK", 35.6, -97.5),
    ("OR", 43.9, -120.6),
    ("PA", 40.9, -77.8),
    ("RI", 41.7, -71.5),
    ("SC", 33.9, -80.9),
    ("SD", 44.4, -100.2),
    ("TN", 35.9, -86.4),
    ("TX", 31.5, -99.3),
    ("UT", 39.3, -111.7),
    ("VT", 44.1, -72.7),
    ("VA", 37.5, -78.9),
    ("WA", 47.4, -120.5),
    ("WV", 38.6, -80.6),
    ("WI", 44.6, -89.9),
    ("WY", 43.0, -107.6),
];

/// Index of a state code in `STATE_CENTERS`, if known.
pub fn state_index(code: &str) -> Option<usize> {
    STATE_CENTERS.iter().position(|s| s.0 == code)
}

static MILES: std::sync::LazyLock<Vec<f64>> =
    std::sync::LazyLock::new(|| (0..51 * 51).map(|k| great_circle(k / 51, k % 51)).collect());

/// Distance in miles between two states' centers (60 within a state).
#[inline]
pub fn miles(a: usize, b: usize) -> f64 {
    MILES[a * 51 + b]
}

/// Where a town is: its own place when the towns table has it, else its
/// state's center.
pub fn place(state: usize, town: &str) -> (f64, f64) {
    let (code, lat, lon) = STATE_CENTERS[state.min(50)];
    super::high_school::town_place(code, town).unwrap_or((lat, lon))
}

/// Campuses whose town the towns table doesn't list under the college's
/// own spelling (a campus town, a split city), with their real places.
const CAMPUSES: [(&str, &str, f64, f64); 23] = [
    ("CO", "USAF Academy", 38.998, -104.862),
    ("NY", "West Point", 41.391, -73.956),
    ("IL", "Urbana-Champaign", 40.110, -88.228),
    ("FL", "Coral Gables", 25.721, -80.268),
    ("IL", "DeKalb", 41.930, -88.750),
    ("PA", "University Park", 40.798, -77.860),
    ("TX", "University Park", 32.850, -96.800),
    ("CT", "Storrs", 41.808, -72.250),
    ("NC", "Winston-Salem", 36.100, -80.244),
    ("MN", "St. Paul", 44.954, -93.090),
    ("MO", "St. Louis", 38.627, -90.199),
    ("AL", "Normal", 34.787, -86.572),
    ("TX", "Prairie View", 30.093, -95.988),
    ("PA", "Wilkes-Barre", 41.246, -75.881),
    ("MO", "St. Charles", 38.788, -90.497),
    ("NC", "Boiling Springs", 35.254, -81.667),
    ("NC", "Buies Creek", 35.413, -78.736),
    ("FL", "DeLand", 29.028, -81.303),
    ("MA", "Easton", 42.024, -71.129),
    ("UT", "St. George", 37.096, -113.568),
    ("TX", "Brownsville/Edinburg", 26.302, -98.163),
    ("PA", "Moon Township", 40.521, -80.211),
    ("NY", "Brookville", 40.813, -73.567),
];

/// Where a college's campus is: its town's place, a listed campus, or its
/// state's center.
pub fn campus(state: usize, city: &str) -> (f64, f64) {
    let code = STATE_CENTERS[state.min(50)].0;
    match CAMPUSES.iter().find(|c| c.0 == code && c.1 == city) {
        Some(c) => (c.2, c.3),
        None => place(state, city),
    }
}

/// Great-circle miles between two places (latitude, longitude).
pub fn miles_between(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (p1, p2) = (a.0.to_radians(), b.0.to_radians());
    let dp = p2 - p1;
    let dl = (b.1 - a.1).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    3959.0 * 2.0 * h.sqrt().min(1.0).asin()
}

fn great_circle(a: usize, b: usize) -> f64 {
    if a == b {
        return 60.0;
    }
    let (_, la1, lo1) = STATE_CENTERS[a];
    let (_, la2, lo2) = STATE_CENTERS[b];
    let (p1, p2) = (la1.to_radians(), la2.to_radians());
    let dp = p2 - p1;
    let dl = (lo2 - lo1).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    3959.0 * 2.0 * h.sqrt().asin()
}

const ALTITUDE_CSV: &str = include_str!("../../../../data/real-world/college_altitude.csv");

static ALTITUDES: std::sync::LazyLock<Vec<(&'static str, &'static str, f64)>> = std::sync::LazyLock::new(|| {
    ALTITUDE_CSV
        .lines()
        .skip(1)
        .filter_map(|l| {
            let c: Vec<&str> = l.split(',').collect();
            Some((c.first()?.trim(), c.get(1)?.trim(), c.get(3)?.trim().parse().ok()?))
        })
        .collect()
});

/// A college's home field above sea level in feet (city elevations from
/// `college_altitude.csv`; every place not listed sits under 2,000 feet,
/// taken as 500).
pub fn college_altitude(name: &str) -> f64 {
    ALTITUDES.iter().find(|a| a.0 == name).map_or(500.0, |a| a.2)
}

/// A town's altitude in feet, for neutral sites (500 when not listed).
pub fn town_altitude(town: &str) -> f64 {
    ALTITUDES.iter().find(|a| a.1 == town).map_or(500.0, |a| a.2)
}
