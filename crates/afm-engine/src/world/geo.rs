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
