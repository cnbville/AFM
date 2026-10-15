//! Weather from the date and the place (A34-A38).
//!
//! Each state has the climate normals of its main football town: monthly
//! mean highs and lows, days with measurable precipitation, mean wind and a
//! midsummer dew point. A day's weather is drawn from those normals, the same
//! for every game in that state that day, so September heat in the South and
//! December cold in the North come from the calendar.
//!
//! Source and doubt: NOAA's 1991-2020 normals could not be downloaded (the
//! network refuses www.ncei.noaa.gov), so these are researched values written
//! from the published normals for each city, rounded. They should be checked
//! against the NOAA files when the network allows (logged as a doubt).

use super::date::Date;

/// One state's climate: (state code, town, highs, lows, precipitation days,
/// mean wind mph, July dew point F).
pub struct Climate {
    pub state: &'static str,
    pub town: &'static str,
    pub high: [f64; 12],
    pub low: [f64; 12],
    pub wet_days: [f64; 12],
    pub wind: f64,
    pub july_dew: f64,
}

macro_rules! c {
    ($s:expr, $t:expr, [$($h:expr),*], [$($l:expr),*], [$($p:expr),*], $w:expr, $d:expr) => {
        Climate { state: $s, town: $t, high: [$($h as f64),*], low: [$($l as f64),*], wet_days: [$($p as f64),*], wind: $w, july_dew: $d as f64 }
    };
}

pub const CLIMATES: [Climate; 51] = [
    c!("AL", "Birmingham", [54, 59, 67, 75, 82, 88, 91, 90, 85, 75, 64, 56], [34, 37, 44, 51, 60, 68, 72, 71, 65, 53, 42, 36], [11, 10, 11, 9, 10, 11, 12, 10, 7, 7, 9, 10], 6.5, 71),
    c!("AK", "Anchorage", [23, 27, 34, 44, 55, 62, 65, 63, 55, 41, 28, 25], [11, 13, 18, 29, 39, 47, 52, 50, 42, 29, 17, 13], [9, 8, 8, 6, 6, 8, 11, 14, 15, 12, 10, 11], 6.5, 48),
    c!("AZ", "Phoenix", [67, 71, 77, 85, 95, 104, 106, 104, 100, 89, 76, 66], [46, 49, 54, 61, 70, 79, 85, 84, 78, 66, 53, 45], [4, 4, 4, 2, 1, 1, 4, 5, 3, 2, 2, 4], 6.2, 55),
    c!("AR", "Little Rock", [51, 56, 65, 74, 82, 89, 93, 93, 86, 75, 63, 53], [32, 36, 44, 52, 61, 69, 73, 72, 64, 52, 41, 34], [9, 9, 10, 9, 10, 8, 8, 7, 7, 7, 8, 10], 7.5, 71),
    c!("CA", "Los Angeles", [68, 68, 70, 72, 74, 78, 83, 84, 83, 79, 73, 68], [48, 50, 52, 55, 58, 62, 65, 66, 64, 60, 53, 48], [6, 6, 5, 3, 1, 0, 0, 0, 1, 2, 3, 5], 7.0, 60),
    c!("CO", "Denver", [45, 46, 54, 61, 71, 82, 89, 87, 79, 66, 53, 45], [18, 20, 27, 34, 44, 53, 60, 58, 49, 37, 25, 18], [5, 6, 8, 9, 11, 9, 9, 9, 6, 5, 5, 5], 8.5, 48),
    c!("CT", "Hartford", [35, 38, 47, 60, 71, 79, 85, 83, 75, 63, 51, 40], [18, 20, 27, 38, 48, 57, 63, 61, 53, 41, 32, 24], [11, 10, 11, 11, 12, 11, 10, 9, 9, 10, 10, 11], 8.0, 63),
    c!("DE", "Wilmington", [41, 44, 53, 64, 74, 82, 87, 85, 79, 67, 56, 45], [25, 27, 34, 43, 53, 62, 68, 66, 59, 47, 37, 30], [10, 9, 10, 11, 11, 10, 9, 9, 8, 8, 9, 10], 8.5, 66),
    c!("DC", "Washington", [44, 48, 57, 68, 77, 86, 90, 88, 81, 70, 59, 48], [29, 31, 38, 48, 57, 67, 72, 70, 63, 51, 41, 33], [10, 9, 11, 10, 11, 10, 10, 9, 8, 8, 8, 10], 8.5, 67),
    c!("FL", "Orlando", [72, 75, 79, 84, 89, 91, 92, 92, 90, 85, 79, 74], [51, 54, 58, 63, 68, 73, 74, 75, 73, 67, 59, 54], [6, 7, 7, 5, 7, 15, 17, 17, 14, 7, 5, 6], 7.5, 73),
    c!("GA", "Atlanta", [53, 57, 65, 73, 80, 87, 90, 89, 84, 74, 64, 55], [35, 38, 44, 51, 60, 68, 71, 71, 65, 54, 44, 37], [11, 10, 11, 9, 9, 12, 12, 10, 7, 7, 9, 10], 8.5, 69),
    c!("HI", "Honolulu", [80, 81, 81, 83, 85, 87, 88, 89, 89, 87, 84, 81], [66, 66, 67, 69, 71, 73, 75, 75, 74, 73, 71, 68], [8, 7, 8, 7, 5, 5, 6, 5, 6, 8, 9, 10], 10.5, 66),
    c!("ID", "Boise", [38, 45, 54, 62, 72, 81, 92, 90, 79, 65, 49, 38], [24, 27, 32, 37, 45, 52, 59, 58, 49, 39, 30, 24], [10, 9, 10, 9, 8, 5, 2, 2, 3, 6, 9, 10], 8.5, 42),
    c!("IL", "Chicago", [32, 36, 47, 59, 70, 80, 84, 82, 75, 62, 48, 36], [18, 21, 30, 40, 50, 60, 66, 65, 57, 45, 34, 23], [11, 9, 11, 12, 12, 11, 10, 9, 9, 10, 10, 11], 10.0, 61),
    c!("IN", "Indianapolis", [36, 40, 51, 63, 73, 82, 85, 84, 78, 65, 52, 40], [21, 24, 32, 42, 52, 62, 66, 64, 56, 44, 34, 25], [12, 10, 12, 12, 13, 11, 10, 9, 8, 9, 10, 12], 9.0, 64),
    c!("IA", "Des Moines", [31, 36, 49, 62, 72, 82, 86, 84, 77, 63, 48, 35], [15, 19, 30, 41, 52, 62, 67, 65, 56, 43, 31, 19], [8, 8, 9, 11, 12, 11, 9, 9, 8, 8, 7, 8], 10.0, 63),
    c!("KS", "Wichita", [42, 47, 58, 67, 76, 87, 92, 91, 82, 69, 55, 44], [22, 26, 35, 44, 54, 64, 69, 68, 59, 46, 34, 25], [5, 5, 7, 8, 11, 10, 8, 8, 7, 7, 5, 5], 11.5, 63),
    c!("KY", "Louisville", [43, 48, 57, 69, 77, 85, 89, 88, 82, 70, 57, 46], [27, 30, 37, 47, 57, 66, 70, 69, 61, 49, 38, 31], [11, 11, 12, 12, 12, 11, 10, 8, 8, 8, 10, 11], 8.0, 66),
    c!("LA", "Baton Rouge", [62, 66, 73, 79, 86, 91, 92, 92, 89, 81, 71, 64], [42, 46, 52, 58, 66, 72, 74, 74, 70, 59, 49, 44], [10, 9, 9, 7, 8, 12, 14, 12, 9, 6, 8, 9], 7.0, 74),
    c!("ME", "Portland", [31, 34, 42, 53, 64, 73, 79, 78, 70, 58, 47, 36], [14, 16, 25, 35, 44, 54, 60, 59, 51, 39, 31, 21], [11, 10, 11, 12, 12, 11, 10, 9, 9, 11, 11, 12], 8.5, 59),
    c!("MD", "Baltimore", [43, 46, 55, 66, 75, 84, 89, 86, 79, 68, 57, 46], [26, 28, 35, 45, 54, 64, 69, 67, 60, 47, 37, 30], [10, 9, 11, 11, 11, 10, 9, 9, 8, 8, 9, 10], 8.5, 66),
    c!("MA", "Boston", [37, 39, 46, 56, 67, 76, 82, 80, 73, 62, 52, 42], [23, 25, 31, 41, 50, 60, 66, 65, 58, 47, 38, 29], [11, 10, 12, 11, 12, 11, 10, 9, 9, 10, 10, 12], 12.0, 61),
    c!("MI", "Detroit", [32, 35, 46, 59, 70, 79, 84, 81, 74, 61, 48, 36], [19, 20, 28, 38, 48, 58, 63, 62, 54, 43, 34, 25], [13, 10, 12, 12, 12, 10, 10, 9, 10, 10, 11, 13], 10.0, 61),
    c!("MN", "Minneapolis", [23, 28, 41, 57, 69, 79, 83, 80, 72, 58, 41, 27], [8, 12, 24, 36, 48, 58, 63, 61, 52, 39, 26, 13], [9, 7, 9, 10, 12, 12, 10, 9, 9, 9, 8, 9], 10.5, 61),
    c!("MS", "Jackson", [57, 61, 69, 76, 84, 90, 92, 92, 88, 78, 67, 59], [37, 40, 46, 53, 62, 69, 72, 72, 66, 54, 44, 39], [10, 9, 10, 8, 9, 10, 11, 10, 7, 7, 8, 10], 7.0, 72),
    c!("MO", "Columbia", [39, 44, 56, 66, 75, 84, 89, 88, 80, 68, 54, 42], [21, 25, 35, 45, 55, 65, 69, 67, 58, 46, 35, 25], [7, 7, 9, 11, 12, 11, 9, 9, 8, 8, 7, 7], 10.0, 65),
    c!("MT", "Billings", [34, 38, 47, 57, 67, 77, 87, 86, 74, 60, 45, 35], [17, 19, 26, 34, 43, 52, 59, 57, 48, 37, 26, 18], [7, 7, 9, 10, 12, 11, 7, 6, 7, 7, 6, 7], 10.5, 45),
    c!("NE", "Lincoln", [34, 39, 52, 64, 74, 84, 88, 86, 78, 65, 50, 37], [15, 19, 29, 40, 52, 62, 67, 65, 55, 42, 29, 19], [6, 6, 8, 10, 12, 11, 9, 9, 8, 7, 5, 6], 10.5, 63),
    c!("NV", "Las Vegas", [58, 63, 71, 78, 88, 99, 105, 103, 95, 81, 67, 57], [39, 43, 49, 56, 66, 75, 82, 80, 72, 59, 46, 38], [3, 4, 3, 2, 1, 1, 2, 2, 2, 2, 2, 3], 9.0, 40),
    c!("NH", "Concord", [31, 35, 44, 57, 69, 77, 82, 81, 73, 60, 47, 36], [11, 14, 23, 33, 43, 53, 58, 56, 48, 36, 28, 18], [11, 10, 11, 12, 13, 12, 11, 10, 9, 10, 11, 11], 7.0, 59),
    c!("NJ", "Newark", [39, 42, 51, 63, 73, 82, 88, 86, 78, 66, 55, 44], [25, 27, 33, 44, 53, 63, 69, 68, 60, 48, 39, 30], [10, 10, 11, 11, 12, 10, 10, 9, 8, 9, 9, 11], 9.5, 64),
    c!("NM", "Albuquerque", [48, 54, 62, 70, 80, 90, 92, 89, 82, 70, 57, 47], [26, 30, 35, 42, 51, 60, 66, 64, 57, 44, 32, 25], [4, 4, 5, 3, 4, 4, 9, 10, 6, 4, 3, 4], 8.5, 44),
    c!("NY", "Syracuse", [31, 33, 42, 56, 68, 77, 81, 80, 72, 59, 47, 36], [17, 18, 25, 36, 46, 56, 61, 60, 52, 41, 33, 23], [18, 15, 15, 13, 13, 11, 11, 10, 11, 13, 15, 18], 9.5, 60),
    c!("NC", "Raleigh", [51, 55, 63, 72, 80, 87, 90, 88, 82, 72, 62, 53], [31, 34, 40, 48, 57, 66, 70, 69, 62, 50, 40, 34], [10, 9, 10, 9, 10, 10, 11, 10, 8, 7, 8, 9], 7.5, 68),
    c!("ND", "Fargo", [18, 23, 36, 54, 68, 77, 82, 81, 72, 56, 37, 22], [1, 5, 18, 31, 44, 54, 59, 57, 47, 34, 20, 7], [8, 7, 8, 8, 10, 12, 10, 9, 8, 8, 7, 8], 11.5, 58),
    c!("OH", "Columbus", [37, 41, 52, 64, 74, 82, 85, 84, 78, 66, 53, 41], [22, 24, 32, 42, 52, 61, 65, 64, 56, 45, 35, 27], [13, 11, 13, 13, 13, 11, 11, 9, 9, 10, 11, 13], 8.5, 63),
    c!("OK", "Oklahoma City", [49, 54, 63, 71, 79, 88, 94, 93, 85, 73, 60, 50], [28, 32, 40, 49, 59, 68, 72, 71, 63, 51, 39, 30], [5, 6, 7, 8, 10, 9, 7, 7, 7, 7, 5, 5], 12.0, 66),
    c!("OR", "Eugene", [47, 51, 56, 61, 67, 73, 82, 82, 77, 64, 53, 46], [36, 36, 39, 42, 47, 52, 56, 57, 52, 46, 40, 35], [18, 16, 18, 16, 12, 8, 3, 3, 6, 12, 18, 19], 7.5, 52),
    c!("PA", "State College", [37, 40, 50, 63, 72, 80, 84, 82, 76, 64, 52, 41], [22, 23, 30, 40, 50, 59, 63, 62, 55, 44, 35, 27], [16, 14, 14, 14, 13, 12, 11, 10, 10, 11, 13, 15], 8.5, 61),
    c!("RI", "Providence", [38, 41, 48, 59, 69, 78, 83, 82, 75, 64, 53, 43], [22, 24, 31, 40, 49, 59, 65, 64, 56, 45, 36, 28], [11, 10, 11, 11, 11, 10, 9, 9, 8, 9, 10, 11], 9.5, 62),
    c!("SC", "Columbia", [57, 61, 69, 77, 85, 91, 94, 92, 86, 77, 67, 59], [34, 37, 43, 50, 60, 68, 72, 71, 64, 52, 41, 36], [9, 9, 9, 8, 8, 10, 12, 10, 7, 6, 7, 9], 6.5, 69),
    c!("SD", "Sioux Falls", [27, 32, 45, 59, 70, 80, 85, 83, 75, 61, 44, 31], [7, 12, 23, 34, 46, 57, 62, 60, 50, 37, 23, 12], [6, 6, 7, 9, 11, 11, 9, 8, 7, 7, 5, 6], 10.5, 61),
    c!("TN", "Nashville", [49, 54, 63, 72, 80, 87, 90, 90, 84, 73, 61, 51], [30, 33, 40, 48, 58, 66, 70, 69, 62, 50, 39, 33], [11, 10, 11, 10, 11, 10, 10, 8, 7, 8, 9, 11], 7.5, 68),
    c!("TX", "Dallas", [57, 61, 69, 77, 84, 92, 96, 97, 89, 79, 67, 58], [37, 41, 49, 56, 65, 72, 76, 76, 69, 58, 47, 39], [7, 7, 8, 7, 9, 7, 5, 5, 6, 7, 7, 7], 10.0, 67),
    c!("UT", "Salt Lake City", [38, 44, 55, 62, 72, 84, 93, 91, 80, 65, 50, 38], [23, 27, 34, 40, 48, 57, 65, 64, 54, 42, 31, 23], [9, 9, 9, 9, 8, 5, 4, 5, 5, 6, 8, 9], 8.5, 42),
    c!("VT", "Burlington", [28, 31, 40, 55, 68, 77, 82, 80, 72, 58, 45, 33], [11, 13, 22, 34, 46, 55, 60, 59, 51, 39, 30, 18], [15, 12, 13, 13, 14, 14, 12, 12, 12, 13, 14, 15], 8.5, 59),
    c!("VA", "Richmond", [48, 51, 60, 70, 78, 86, 90, 88, 81, 71, 61, 51], [29, 31, 38, 47, 56, 65, 70, 68, 61, 49, 39, 32], [10, 9, 11, 10, 11, 10, 11, 9, 8, 8, 8, 10], 7.5, 67),
    c!("WA", "Seattle", [48, 50, 54, 58, 65, 70, 77, 77, 71, 60, 52, 47], [37, 37, 39, 42, 47, 52, 56, 57, 53, 46, 40, 36], [18, 15, 17, 14, 10, 8, 4, 4, 7, 13, 18, 18], 8.5, 53),
    c!("WV", "Charleston", [44, 48, 58, 69, 77, 84, 87, 86, 80, 69, 57, 47], [26, 28, 35, 43, 52, 61, 65, 64, 57, 45, 36, 29], [15, 13, 15, 14, 14, 12, 13, 11, 10, 11, 12, 14], 6.0, 64),
    c!("WI", "Madison", [27, 31, 43, 57, 69, 79, 83, 80, 72, 59, 45, 32], [11, 14, 24, 35, 45, 55, 61, 59, 50, 38, 28, 17], [10, 9, 11, 12, 12, 11, 10, 10, 9, 10, 9, 11], 9.5, 61),
    c!("WY", "Laramie", [39, 40, 47, 54, 64, 76, 83, 81, 72, 59, 46, 38], [17, 18, 24, 30, 39, 48, 55, 53, 44, 33, 23, 17], [6, 7, 9, 10, 12, 10, 10, 9, 7, 7, 6, 6], 12.5, 42),
];

/// The climate row for a state code.
pub fn climate(code: &str) -> &'static Climate {
    CLIMATES.iter().find(|c| c.state == code).unwrap_or(&CLIMATES[42])
}

/// States where hurricanes reach football towns (August to October).
const HURRICANE_STATES: [&str; 8] = ["FL", "LA", "TX", "MS", "AL", "GA", "SC", "NC"];
/// Snow-belt states where a blizzard can stop a game (November to January).
const BLIZZARD_STATES: [&str; 15] =
    ["NY", "MI", "MN", "ND", "SD", "WI", "IA", "NE", "ME", "VT", "NH", "MA", "PA", "WY", "MT"];

/// One place's weather at one moment.
#[derive(Clone, Copy, Debug, Default)]
pub struct Weather {
    pub temperature: f64,
    pub dew_point: f64,
    pub wind: f64,
    pub rain: bool,
    pub snow: bool,
    /// A storm with lightning in the area during the window.
    pub lightning: bool,
    /// Severe weather that can postpone or move a game (A38).
    pub severe: Option<Severe>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severe {
    Hurricane,
    Blizzard,
}

impl Weather {
    /// The heat index (NWS Rothfusz regression) in F; below 80 F it is the temperature.
    pub fn heat_index(&self) -> f64 {
        let t = self.temperature;
        if t < 80.0 {
            return t;
        }
        let rh = relative_humidity(t, self.dew_point);
        -42.379 + 2.049_015_23 * t + 10.143_331_27 * rh
            - 0.224_755_41 * t * rh
            - 0.006_837_83 * t * t
            - 0.054_817_17 * rh * rh
            + 0.001_228_74 * t * t * rh
            + 0.000_852_82 * t * rh * rh
            - 0.000_001_99 * t * t * rh * rh
    }
}

/// Relative humidity (percent) from temperature and dew point (Magnus).
fn relative_humidity(t: f64, dew: f64) -> f64 {
    let c = |f: f64| (f - 32.0) / 1.8;
    let (tc, dc) = (c(t), c(dew.min(t)));
    let es = |x: f64| (17.625 * x / (243.04 + x)).exp();
    (100.0 * es(dc) / es(tc)).clamp(1.0, 100.0)
}

fn mix(a: u64, b: u64) -> u64 {
    let mut x = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_add(0x632B_E59B_D9B4_E019).rotate_left(17);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn uniform(a: u64, b: u64) -> f64 {
    (mix(a, b) >> 11) as f64 / (1u64 << 53) as f64
}

fn normal(a: u64, b: u64) -> f64 {
    let u1 = uniform(a, b).max(1e-12);
    let u2 = uniform(a ^ 0x5DEE_CE66, b.wrapping_add(11));
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// The climate normals for a date, interpolated between mid-month values.
fn normals(c: &Climate, d: Date) -> (f64, f64, f64, f64) {
    let (_, m, day) = d.parts();
    let m0 = m as usize - 1;
    let (a, b, f) = if day >= 15 {
        (m0, (m0 + 1) % 12, (day as f64 - 15.0) / 30.0)
    } else {
        ((m0 + 11) % 12, m0, (day as f64 + 15.0) / 30.0)
    };
    let lerp = |v: &[f64; 12]| v[a] + (v[b] - v[a]) * f;
    let wet = lerp(&c.wet_days) / 30.4;
    // Dew points track the season: midsummer value, falling about 0.9 F per F
    // of mean temperature below July's.
    let mean = (lerp(&c.high) + lerp(&c.low)) / 2.0;
    let july = (c.high[6] + c.low[6]) / 2.0;
    let dew = c.july_dew - 0.9 * (july - mean).max(0.0);
    (lerp(&c.high), lerp(&c.low), wet, dew)
}

/// The weather in a state at a date and hour (Eastern), drawn from its
/// normals with a day-to-day swing shared by every place in the state.
pub fn weather_at(seed: u64, state: usize, code: &str, d: Date, hour: u32) -> Weather {
    let c = climate(code);
    let (high, low, wet, dew) = normals(c, d);
    let key = seed ^ ((state as u64) << 40);
    let day = d.0 as u64;
    // Day-to-day anomalies are wider in winter and inland.
    let (_, m, _) = d.parts();
    let winter = matches!(m, 11 | 12 | 1 | 2 | 3);
    let swing = if winter { 9.0 } else { 6.0 };
    let anomaly = swing * normal(key, day);
    // Temperature through the day: the low near dawn, the high mid-afternoon.
    let local = hour as f64 - 1.0;
    let shape = match local {
        h if h <= 6.0 => 0.05,
        h if h <= 15.0 => (h - 6.0) / 9.0,
        h => (1.0 - (h - 15.0) / 12.0).max(0.15),
    };
    let temperature = low + (high - low) * shape + anomaly;
    let wet_day = uniform(key ^ 0x7A1, day) < wet;
    // A wet day is wet for part of it: about 55% chance of rain in a 3-hour window.
    let raining = wet_day && uniform(key ^ 0x7A2, day * 24 + hour as u64) < 0.55;
    let snow = raining && temperature < 34.0;
    let rain = raining && !snow;
    let warm = temperature >= 60.0 && dew >= 55.0;
    let lightning = raining && uniform(key ^ 0x7A3, day) < if warm { 0.35 } else { 0.05 };
    let wind = (c.wind * (1.0 + 0.45 * normal(key ^ 0x7A4, day))).max(0.0) * if raining { 1.2 } else { 1.0 };
    let mut severe = None;
    if HURRICANE_STATES.contains(&code) && (8..=10).contains(&m) {
        let peak = if m == 9 { 0.0012 } else { 0.0006 };
        // A storm affects several days around landfall: key it on the week.
        if uniform(key ^ 0x4C1, (day / 4) * 4) < peak * 4.0 {
            severe = Some(Severe::Hurricane);
        }
    }
    if BLIZZARD_STATES.contains(&code) && matches!(m, 11 | 12 | 1) && snow && uniform(key ^ 0x4C2, day) < 0.08 {
        severe = Some(Severe::Blizzard);
    }
    Weather { temperature, dew_point: dew.min(temperature), wind, rain, snow, lightning, severe }
}

/// Indoor venues: no weather.
pub fn indoors() -> Weather {
    Weather { temperature: 70.0, dew_point: 50.0, wind: 0.0, ..Weather::default() }
}

/// Minutes a game is held for lightning under each level's rule (A37): play
/// stops for 30 minutes after each strike within 8 miles, and the clock
/// restarts with every new strike (NCAA and NFHS; the NFL follows the same
/// 30-minute rule). Proposed, not confirmed: strikes per storm.
pub fn lightning_delay(seed: u64, key: u64) -> u32 {
    let restarts = (uniform(seed ^ 0x11, key) * 3.0) as u32;
    30 + restarts * (10 + (uniform(seed ^ 0x12, key) * 20.0) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seasons_come_from_the_calendar() {
        let fl = crate::world::geo::state_index("FL").unwrap();
        let mn = crate::world::geo::state_index("MN").unwrap();
        let mut sep_fl = 0.0;
        let mut dec_mn = 0.0;
        let mut snow = 0;
        for y in 2000..2030 {
            let s = Date::ymd(y, 9, 12);
            let d = Date::ymd(y, 12, 5);
            sep_fl += weather_at(7, fl, "FL", s, 15).temperature;
            let w = weather_at(7, mn, "MN", d, 13);
            dec_mn += w.temperature;
            snow += w.snow as u32;
        }
        sep_fl /= 30.0;
        dec_mn /= 30.0;
        assert!((82.0..96.0).contains(&sep_fl), "Florida September afternoons {sep_fl:.0}F");
        assert!((15.0..38.0).contains(&dec_mn), "Minnesota December {dec_mn:.0}F");
        assert!(snow <= 12, "{snow} snowy December games in 30 years");
        let hot = Weather { temperature: 92.0, dew_point: 74.0, ..Weather::default() };
        assert!((100.0..115.0).contains(&hot.heat_index()), "heat index {:.0}", hot.heat_index());
    }
}
