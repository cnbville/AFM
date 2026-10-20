//! Seeded pseudo-random generator (sfc32). Every simulation takes an `Rng` so
//! that careers, saves and tests are fully reproducible from a seed.
//!
//! The generator and its seeding match the original TypeScript model, so a
//! label like `"national-class"` gives the same raw stream in both.

use std::f64::consts::PI;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Rng {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    spare_normal: Option<f64>,
}

/// A seed: a number or a label (hashed).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Seed {
    Number(u32),
    Label(String),
}

impl From<u32> for Seed {
    fn from(n: u32) -> Self {
        Seed::Number(n)
    }
}
impl From<&str> for Seed {
    fn from(s: &str) -> Self {
        Seed::Label(s.to_string())
    }
}
impl From<String> for Seed {
    fn from(s: String) -> Self {
        Seed::Label(s)
    }
}

impl Rng {
    pub fn new(seed: impl Into<Seed>) -> Self {
        let mut h = match seed.into() {
            Seed::Number(n) => n,
            Seed::Label(s) => hash_string(&s),
        };
        // splitmix32 to spread the seed over the four state words
        let mut next = || {
            h = h.wrapping_add(0x9e37_79b9);
            let mut z = h;
            z = (z ^ (z >> 16)).wrapping_mul(0x85eb_ca6b);
            z = (z ^ (z >> 13)).wrapping_mul(0xc2b2_ae35);
            z ^ (z >> 16)
        };
        let (a, b, c, d) = (next(), next(), next(), next());
        let mut rng = Rng { a, b, c, d, spare_normal: None };
        for _ in 0..12 {
            rng.next_u32();
        }
        rng
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let t = self.a.wrapping_add(self.b).wrapping_add(self.d);
        self.d = self.d.wrapping_add(1);
        self.a = self.b ^ (self.b >> 9);
        self.b = self.c.wrapping_add(self.c << 3);
        self.c = self.c.rotate_right(11);
        self.c = self.c.wrapping_add(t);
        t
    }

    /// Uniform float in [0, 1).
    #[inline]
    pub fn next(&mut self) -> f64 {
        self.next_u32() as f64 / 4_294_967_296.0
    }

    #[inline]
    pub fn range(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.next()
    }

    /// Integer in [min, max] inclusive.
    #[inline]
    pub fn int(&mut self, min: i32, max: i32) -> i32 {
        // Wrapping, as release builds already did: a full-range call
        // (`int(0, i32::MAX)`, the game seeds) must not panic in debug, and
        // seeds must stay the same.
        min.wrapping_add((self.next() * max.wrapping_sub(min).wrapping_add(1) as f64).floor() as i32)
    }

    #[inline]
    pub fn chance(&mut self, p: f64) -> bool {
        self.next() < p
    }

    /// Standard normal draw scaled to `mean` and `sd` (Box-Muller, spare kept).
    pub fn normal(&mut self, mean: f64, sd: f64) -> f64 {
        if let Some(z) = self.spare_normal.take() {
            return mean + sd * z;
        }
        let mut u = 0.0;
        while u == 0.0 {
            u = self.next();
        }
        let v = self.next();
        let r = (-2.0 * u.ln()).sqrt();
        self.spare_normal = Some(r * (2.0 * PI * v).sin());
        mean + sd * r * (2.0 * PI * v).cos()
    }

    /// Multiplicative noise centred on 1 (median 1, always positive).
    #[inline]
    pub fn log_normal(&mut self, sd: f64) -> f64 {
        self.normal(0.0, sd).exp()
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next() * items.len() as f64) as usize]
    }

    pub fn weighted<T: Copy>(&mut self, entries: &[(T, f64)]) -> T {
        let total: f64 = entries.iter().map(|(_, w)| w).sum();
        let mut roll = self.next() * total;
        for &(item, w) in entries {
            roll -= w;
            if roll < 0.0 {
                return item;
            }
        }
        entries[entries.len() - 1].0
    }

    /// Derive an independent child generator (e.g. one per player).
    pub fn fork(&mut self) -> Rng {
        Rng::new(self.next_u32())
    }
}

fn hash_string(s: &str) -> u32 {
    // FNV-1a over UTF-16 code units, as JavaScript's charCodeAt sees them.
    let mut h: u32 = 2_166_136_261;
    for unit in s.encode_utf16() {
        h ^= unit as u32;
        h = h.wrapping_mul(16_777_619);
    }
    h
}

#[inline]
pub fn clamp(x: f64, min: f64, max: f64) -> f64 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

/// Rounds half up, like JavaScript's Math.round (the TypeScript model's rounding).
#[inline]
pub fn round_half_up(x: f64) -> f64 {
    (x + 0.5).floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_typescript_stream() {
        // First draws of `new Rng('national-class')` and `new Rng(7)` in the TypeScript model.
        let mut a = Rng::new("national-class");
        let mut b = Rng::new(7);
        let xs: Vec<u32> = (0..3).map(|_| a.next_u32()).collect();
        let ys: Vec<u32> = (0..3).map(|_| b.next_u32()).collect();
        assert_eq!(xs, [4_115_287_930, 2_628_163_823, 409_197_335]);
        assert_eq!(ys, [838_155_491, 1_550_620_431, 3_610_122_010]);
    }
}
