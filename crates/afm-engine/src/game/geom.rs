//! Field geometry. x runs from the offense's own goal line (0) to the
//! opponent's (100 on the American field, `goal()` on any); end zones reach
//! -10 and 110 (`back_line()`, `end_line()`). y runs across the field from 0
//! to 53.33 (`width()`), with the offense's left at higher y.

pub const WIDTH: f64 = 160.0 / 3.0;
pub const MID: f64 = WIDTH / 2.0;
pub const DT: f64 = 0.1;

/// The field a game is played on (the other leagues, X7): yards between
/// the goal lines, the width, the end zones' depth and the downs to make
/// ten yards. The engine works in the offense's frame from its own goal
/// line (0) to the other one (`goal()`); every game sets its field for the
/// thread it runs on (`FieldGuard`), and the default is the American field
/// with four downs, so the NFL, college and high school play exactly as
/// before.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub length: f64,
    pub width: f64,
    pub end_zone: f64,
    pub downs: u8,
    /// Half the width between the goalposts' uprights (18 ft 6 in apart on
    /// the American field; 9 ft in arena football), yards.
    pub posts: f64,
    /// How far behind the goal line the goalposts stand: on the end line
    /// (10), on the goal line (the CFL to 2026), at the back of a 15-yard
    /// end zone (the CFL from 2027) or of an 8-yard one (arena).
    pub post_depth: f64,
    /// Rebound nets beside the goalposts (arena): kicks and passes that hit
    /// them stay live.
    pub nets: bool,
}

impl Field {
    pub const AMERICAN: Field =
        Field { length: 100.0, width: WIDTH, end_zone: 10.0, downs: 4, posts: 3.083, post_depth: 10.0, nets: false };
    /// The American field with its four downs (NFL, college, high school
    /// and the American spring leagues).
    pub fn is_american(&self) -> bool {
        *self == Field::AMERICAN
    }
}

impl Default for Field {
    fn default() -> Field {
        Field::AMERICAN
    }
}

thread_local! {
    static FIELD: std::cell::Cell<Field> = const { std::cell::Cell::new(Field::AMERICAN) };
}

/// The field of the game running on this thread.
#[inline]
pub fn field() -> Field {
    FIELD.with(|f| f.get())
}

/// Width of the field (53.33 yards on the American field).
#[inline]
pub fn width() -> f64 {
    FIELD.with(|f| f.get().width)
}

/// The middle of the field across.
#[inline]
pub fn mid() -> f64 {
    width() / 2.0
}

/// The goal line the offense attacks (100 on the American field).
#[inline]
pub fn goal() -> f64 {
    FIELD.with(|f| f.get().length)
}

/// The back of the end zone the offense attacks (110 on the American field).
#[inline]
pub fn end_line() -> f64 {
    FIELD.with(|f| {
        let f = f.get();
        f.length + f.end_zone
    })
}

/// The back of the offense's own end zone (-10 on the American field).
#[inline]
pub fn back_line() -> f64 {
    -FIELD.with(|f| f.get().end_zone)
}

/// A yard line as the real NFL tables read it (yards to the goal on a
/// 100-yard field): the same number on the American field, scaled on the
/// others.
#[inline]
pub fn to_100(yards_to_goal: f64) -> f64 {
    let len = goal();
    if len == 100.0 { yards_to_goal } else { yards_to_goal * 100.0 / len }
}

/// Sets the field for the game on this thread and puts the old one back
/// when dropped.
pub struct FieldGuard(Field);

impl FieldGuard {
    pub fn set(f: Field) -> FieldGuard {
        let old = FIELD.with(|c| c.replace(f));
        FieldGuard(old)
    }
}

impl Drop for FieldGuard {
    fn drop(&mut self) {
        FIELD.with(|c| c.set(self.0));
    }
}

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
    /// The squared distance: `dist` before its root. The same subtraction,
    /// products and sum, so `dist(o)` is `dist_sq(o).sqrt()`, bit for bit.
    /// A sum of squares, it is never negative or -0 (world speed lane).
    #[inline]
    pub fn dist_sq(self, o: V2) -> f64 {
        let d = self - o;
        d.x * d.x + d.y * d.y
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

/// `q.sqrt() < r`, bit for bit, mostly without the root (world speed
/// lane). Below `r² (1 - 1e-9)` the root is below the double before `r`,
/// and above `r² (1 + 1e-9)` it is above `r`; the square root is correctly
/// rounded and monotone, so the answer is known. The band dwarfs the three
/// roundings in working out its ends. Inside it, for `r` outside 1e-100 to
/// 1e100 (where `r²` could leave the normal range), and for a NaN or
/// negative `q`, the root is taken as before. Any `r`: its square needn't
/// be exact.
#[inline]
pub fn sq_lt(q: f64, r: f64) -> bool {
    if r > 1e-100 && r < 1e100 {
        let r2 = r * r;
        if q >= 0.0 && q < r2 * (1.0 - 1e-9) {
            return true;
        }
        if q > r2 * (1.0 + 1e-9) {
            return false;
        }
    }
    q.sqrt() < r
}

/// `q.sqrt() >= r`, bit for bit: the same band as `sq_lt`.
#[inline]
pub fn sq_ge(q: f64, r: f64) -> bool {
    if r > 1e-100 && r < 1e100 {
        let r2 = r * r;
        if q > r2 * (1.0 + 1e-9) {
            return true;
        }
        if q >= 0.0 && q < r2 * (1.0 - 1e-9) {
            return false;
        }
    }
    q.sqrt() >= r
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
