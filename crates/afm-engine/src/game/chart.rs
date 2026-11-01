//! The line chart of a snap (update 2, lane 9): every block's rep, how it
//! ended and the move that beat it, the rush and the gap each man came
//! through, and the protection. Kept only on your own games' plays (games
//! that keep their setup, and live ones); every other game skips it. Small
//! fixed-size records: 16 bytes a rep and 8 a rusher in memory.

/// One block on one snap.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rep {
    pub blocker: u32,
    pub defender: u32,
    /// The pair's leverage when the block ended, times 50: about -50 the
    /// blocker drove or sealed him, +50 the defender came off it.
    pub lev: i8,
    /// How long the block lasted, tenths of a second.
    pub t: u8,
    /// The defender's last move: 0 speed, 1 power, 2 finesse.
    pub mv: u8,
    /// `PASS`, `SHED`, `DOUBLE`, `PANCAKE`.
    pub flags: u8,
    /// Where each lined up: the blocker 0 left tackle to 4 right tackle, 5
    /// a tight end, 6 a back, 7 anyone else; the defender 0 an end, 1 an
    /// inside lineman, 5 a linebacker, 6 a back.
    #[serde(default)]
    pub bpos: u8,
    #[serde(default)]
    pub dpos: u8,
}

/// A blocker's spot from his label.
pub fn blocker_code(label: &str) -> u8 {
    match label {
        "LT" => 0,
        "LG" => 1,
        "C" => 2,
        "RG" => 3,
        "RT" => 4,
        "TE" => 5,
        "RB" | "FB" => 6,
        _ => 7,
    }
}

/// A defender's spot from his label.
pub fn defender_code(label: &str) -> u8 {
    match label {
        "DE" => 0,
        "DT" => 1,
        "LB" => 5,
        _ => 6,
    }
}

impl Rep {
    pub const PASS: u8 = 1;
    /// The defender beat the block and came off it.
    pub const SHED: u8 = 2;
    /// A second blocker was on him.
    pub const DOUBLE: u8 = 4;
    /// The blocker put him on the ground.
    pub const PANCAKE: u8 = 8;

    pub fn pass(&self) -> bool {
        self.flags & Rep::PASS != 0
    }
    pub fn shed(&self) -> bool {
        self.flags & Rep::SHED != 0
    }
    pub fn double(&self) -> bool {
        self.flags & Rep::DOUBLE != 0
    }
    pub fn pancake(&self) -> bool {
        self.flags & Rep::PANCAKE != 0
    }
    pub fn secs(&self) -> f64 {
        self.t as f64 / 10.0
    }

    /// The rep of a block as it ends: `down` when the defender is on the
    /// ground, `secs` how long it lasted.
    pub fn of(e: &super::sim::Engagement, who: [(u32, &str); 2], down: bool, secs: f64) -> Rep {
        let [(blocker, bl), (defender, dl)] = who;
        let mut flags = 0;
        if e.pass {
            flags |= Rep::PASS;
        }
        if e.shed_at.is_some() {
            flags |= Rep::SHED;
        }
        if e.helper.is_some() {
            flags |= Rep::DOUBLE;
        }
        if !e.pass && down && e.shed_at.is_none() {
            flags |= Rep::PANCAKE;
        }
        Rep {
            blocker,
            defender,
            lev: (e.w * 50.0).round().clamp(-127.0, 127.0) as i8,
            t: (secs * 10.0).round().clamp(0.0, 255.0) as u8,
            mv: e.mv as u8,
            flags,
            bpos: blocker_code(bl),
            dpos: defender_code(dl),
        }
    }
}

/// One pass rusher on one snap.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rusher {
    pub id: u32,
    /// The gap he rushed, from the offense's left: 0 left edge, 1 left
    /// inside, 2 middle, 3 right inside, 4 right edge.
    pub gap: u8,
    /// `LINEMAN`, `PICKED`, `STUNT`.
    pub flags: u8,
}

impl Rusher {
    /// A down lineman (not a blitzer).
    pub const LINEMAN: u8 = 1;
    /// A blocker took him at the snap.
    pub const PICKED: u8 = 2;
    /// He was in the stunt (the man who went first or the one who looped).
    pub const STUNT: u8 = 4;
}

/// The protection a pass play used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Protection {
    /// A run, or no protection to speak of.
    #[default]
    None,
    /// Each lineman on the man across from him.
    Man,
    /// The line slid toward a rusher on the offense's left or right.
    SlideLeft,
    SlideRight,
    /// Seven or more kept in to block.
    Max,
}

impl Protection {
    pub fn name(self) -> &'static str {
        match self {
            Protection::None => "None",
            Protection::Man => "Man",
            Protection::SlideLeft => "Slide left",
            Protection::SlideRight => "Slide right",
            Protection::Max => "Max protect",
        }
    }
}

/// A snap's line chart.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LineChart {
    pub reps: Vec<Rep>,
    pub rush: Vec<Rusher>,
    pub protection: Protection,
    /// Men kept in to pass block.
    pub blockers: u8,
}

impl LineChart {
    /// The gap a rusher came through (None when he wasn't rushing).
    pub fn gap_of(&self, id: u32) -> Option<u8> {
        self.rush.iter().find(|r| r.id == id).map(|r| r.gap)
    }
}

/// The gap across the line at `y`, from the line's men's spots (`ol`, any
/// order): left of the offense's left tackle is 0, between the guards 2.
/// The offense's left is high y.
pub fn gap_at(y: f64, ol: &[f64]) -> u8 {
    let mut v: Vec<f64> = ol.to_vec();
    // From the offense's left (high y) to its right.
    v.sort_by(|a, b| b.total_cmp(a));
    if v.len() < 5 {
        return 2;
    }
    let k = v.iter().filter(|&&o| o > y).count();
    match k {
        0 => 0,
        1 => 1,
        2..=3 => 2,
        4 => 3,
        _ => 4,
    }
}
