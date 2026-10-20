//! Field geometry. x runs from the offense's own goal line (0) to the
//! opponent's (100); end zones reach -10 and 110. y runs across the field
//! from 0 to 53.33, with the offense's left at higher y.

pub const WIDTH: f64 = 160.0 / 3.0;
pub const MID: f64 = WIDTH / 2.0;
pub const DT: f64 = 0.1;

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}

#[inline]
pub fn v2(x: f64, y: f64) -> V2 {
    V2 { x, y }
}

impl V2 {
    #[inline]
    pub fn len(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    #[inline]
    pub fn dist(self, o: V2) -> f64 {
        (self - o).len()
    }
    #[inline]
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l < 1e-9 { V2::default() } else { v2(self.x / l, self.y / l) }
    }
    #[inline]
    pub fn dot(self, o: V2) -> f64 {
        self.x * o.x + self.y * o.y
    }
    #[inline]
    pub fn scale(self, k: f64) -> V2 {
        v2(self.x * k, self.y * k)
    }
    #[inline]
    pub fn perp(self) -> V2 {
        v2(-self.y, self.x)
    }
    pub fn angle_between(self, o: V2) -> f64 {
        let d = self.norm().dot(o.norm()).clamp(-1.0, 1.0);
        d.acos()
    }
}

impl std::ops::Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl std::ops::Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
