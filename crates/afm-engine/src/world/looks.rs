//! Faces and looks (faces items F3, F5, F10 and F12; `docs/design/faces.md`).
//!
//! A person's looks are one stored number, [`Looks`], set when he is created
//! from a key of the world's seed and his id. It never draws from the
//! world's stream or his own, so every seed gives the same world as before.
//! Everything else is read from that number when a screen asks:
//! - his genes ([`Genes`]): heritage with mixed ancestry, the seed of the
//!   drawn head's face shape, skin, eye and natural hair colour, hair
//!   texture, beard growth, the genes for greying and a receding hairline,
//!   freckles and small skin marks;
//! - how he looks in one season ([`Appearance`]): haircut and dye, facial
//!   hair, scars and a broken nose, tattoos, earrings and build. It is a
//!   walk from age 14 with one keyed draw per year, so a season always gives
//!   the same looks and every year follows from the one before;
//! - his team headshots ([`Photo`]): one when he joins a team (a transfer,
//!   trade or signing) and a new one every 4 years on media day; high
//!   schoolers get a new one every school year.
//!
//! Looks never decide anything in the sim. The rates are Claude's estimates
//! from real roster photos and the faces research (proposed, not confirmed).

use serde::{Serialize, Serializer};

use super::ethnicity::Ethnicity;
use super::{Person, Stage, World};
use crate::player::Level;
use crate::positions::Position;
use crate::rng::Rng;

// --- Keys ----------------------------------------------------------------------------------

fn mix(a: u32, b: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.rotate_left(16) ^ 0x85EB_CA6B;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^ (h >> 16)
}

fn keyed(seed: u32, tag: u32, n: u32) -> Rng {
    Rng::new(mix(mix(seed, tag), n))
}

const SALT_PERSON: u32 = 0xFACE_0001;
const SALT_COACH: u32 = 0xFACE_0002;
const SALT_FASHION: u32 = 0xFACE_0003;
const T_GENES: u32 = 1;
const T_YEAR: u32 = 2;
const T_PHOTO: u32 = 3;
const T_SPLIT: u32 = 4;

/// The face recipe: one number per person (4 bytes). 0 means not set yet
/// (a world written before faces); `World::faces_after_load` fills it in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Looks(pub u32);

impl Looks {
    pub fn new(world_seed: u32, player_id: u32) -> Looks {
        Looks(mix(mix(world_seed, SALT_PERSON), player_id).max(1))
    }
    /// A coach's looks: read from his id, nothing stored.
    pub fn coach(world_seed: u32, coach_id: usize) -> Looks {
        Looks(mix(mix(world_seed, SALT_COACH), coach_id as u32).max(1))
    }
    pub fn is_set(self) -> bool {
        self.0 != 0
    }
}

// --- Serialising for the screens ----------------------------------------------------------------

fn eth_key<S: Serializer>(e: &Ethnicity, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(e.key())
}

fn mixed_ser<S: Serializer>(m: &Option<(Ethnicity, f32)>, s: S) -> Result<S::Ok, S::Error> {
    match m {
        Some((e, share)) => (e.key(), share).serialize(s),
        None => s.serialize_none(),
    }
}

fn hex<S: Serializer>(c: &[u8; 3], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]))
}

fn rgb(h: u32) -> [u8; 3] {
    [(h >> 16) as u8, (h >> 8) as u8, h as u8]
}

fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| (a[i] as f64 + (b[i] as f64 - a[i] as f64) * t).round() as u8)
}

// --- Genes ----------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EyeColour {
    Blue,
    Green,
    Hazel,
    LightBrown,
    Brown,
    DarkBrown,
}

impl EyeColour {
    pub fn phrase(self) -> &'static str {
        match self {
            EyeColour::Blue => "blue eyes",
            EyeColour::Green => "green eyes",
            EyeColour::Hazel => "hazel eyes",
            EyeColour::LightBrown => "light brown eyes",
            EyeColour::Brown => "brown eyes",
            EyeColour::DarkBrown => "dark brown eyes",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HairColour {
    Black,
    DarkBrown,
    Brown,
    LightBrown,
    Blond,
    Red,
}

impl HairColour {
    pub fn phrase(self) -> &'static str {
        match self {
            HairColour::Black => "black",
            HairColour::DarkBrown => "dark brown",
            HairColour::Brown => "brown",
            HairColour::LightBrown => "light brown",
            HairColour::Blond => "blond",
            HairColour::Red => "red",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Texture {
    Straight,
    Wavy,
    Curly,
    Coily,
}

/// How his beard grows: density and patchiness 0-1, how far it reaches up
/// the cheek and down the neck, and the age it fills in.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct BeardGenes {
    pub density: f32,
    pub patch: f32,
    pub cheek: f32,
    pub neck: f32,
    pub onset: f32,
}

/// Where a scar sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScarSpot {
    Chin,
    Brow,
    Temple,
    Cheek,
    NoseBridge,
    Lip,
}

impl ScarSpot {
    pub fn phrase(self) -> &'static str {
        match self {
            ScarSpot::Chin => "a small scar on the chin",
            ScarSpot::Brow => "a scar through one eyebrow",
            ScarSpot::Temple => "a scar above the temple",
            ScarSpot::Cheek => "a faint scar on the cheek",
            ScarSpot::NoseBridge => "a scar across the bridge of the nose",
            ScarSpot::Lip => "a small scar on the upper lip",
        }
    }
}

/// What he is born with.
#[derive(Clone, Debug, Serialize)]
pub struct Genes {
    #[serde(serialize_with = "eth_key")]
    pub heritage: Ethnicity,
    /// Mixed ancestry: the other group and its share (under one half). The
    /// name always follows the main group.
    #[serde(serialize_with = "mixed_ser")]
    pub mixed: Option<(Ethnicity, f32)>,
    /// Seed of the drawn head's face shape.
    pub face: u32,
    #[serde(serialize_with = "hex")]
    pub skin: [u8; 3],
    /// How dark his skin is, 0-1 (for the screens and the prompt).
    pub darkness: f32,
    pub eye: EyeColour,
    #[serde(serialize_with = "hex")]
    pub eye_rgb: [u8; 3],
    pub hair: HairColour,
    #[serde(serialize_with = "hex")]
    pub hair_rgb: [u8; 3],
    pub texture: Texture,
    pub beard: BeardGenes,
    /// The age his hairline reaches Norwood 3 and how fast it goes on
    /// (stages a year); None for a man who keeps his hairline.
    pub balding: Option<(f32, f32)>,
    /// The age greying starts and how fast (share of hair a year).
    pub grey_onset: f32,
    pub grey_rate: f32,
    pub freckles: f32,
    pub moles: u8,
    pub acne_scars: f32,
    /// Razor bumps when he shaves close (pseudofolliculitis barbae).
    pub razor_bumps: f32,
    /// Small dark facial bumps (dermatosis papulosa nigra), growing with age.
    pub dpn: f32,
    pub two_tone_lips: f32,
    /// An upper-lid crease, and its height in mm.
    pub lid_crease: bool,
    pub crease_mm: f32,
    pub tooth_gap: bool,
    pub childhood_scar: Option<ScarSpot>,
    /// A nose broken before high school, 0-1 crooked.
    pub childhood_nose: f32,
    /// BMI points to add before reading body fat (equal BMI holds more fat
    /// in Asian men and less in Polynesian and Black men; Deurenberg 1998).
    pub bmi_offset: f32,
    /// Where he sits in his position's muscle range, 0-1.
    pub muscle: f32,
}

fn gi(e: Ethnicity) -> usize {
    e as usize
}

/// Skin colour range per group (light end, dark end), from the v4 player
/// recipe (`newplayer_v4.py`); the drawn head's light makes the rest.
const SKIN: [(u32, u32); 7] = [
    (0xf0c8ae, 0xd29a78),
    (0x9a6646, 0x4a2c1e),
    (0xd8a47c, 0xa8734f),
    (0xb88159, 0x7a5034),
    (0xecc6a2, 0xc8996f),
    (0xddaa80, 0xa87450),
    (0xc99468, 0x5e3a24),
];

/// Mixed ancestry: the share of each group with a second heritage (Census
/// 2020 "alone or in combination" against "alone", children higher), and
/// the other group's weights (W, B, H, P, E, SE, S).
const MIXED: [f64; 7] = [0.05, 0.12, 0.15, 0.45, 0.17, 0.17, 0.08];
const PARTNER: [[f64; 7]; 7] = [
    [0.0, 0.32, 0.40, 0.04, 0.10, 0.09, 0.05],
    [0.70, 0.0, 0.22, 0.02, 0.02, 0.03, 0.01],
    [0.62, 0.28, 0.0, 0.02, 0.03, 0.04, 0.01],
    [0.48, 0.08, 0.10, 0.0, 0.16, 0.18, 0.0],
    [0.74, 0.05, 0.08, 0.02, 0.0, 0.10, 0.01],
    [0.66, 0.08, 0.10, 0.06, 0.08, 0.0, 0.02],
    [0.72, 0.06, 0.08, 0.0, 0.04, 0.04, 0.06],
];

fn eye_draw(e: Ethnicity, rng: &mut Rng) -> EyeColour {
    use EyeColour::*;
    match e {
        // Blue fell to about a quarter in White men born since 2000 (Grant and Lauderdale 2002).
        Ethnicity::White => rng.weighted(&[(Blue, 0.27), (Green, 0.09), (Hazel, 0.16), (Brown, 0.48)]),
        Ethnicity::Hispanic => {
            rng.weighted(&[(DarkBrown, 0.6), (Brown, 0.28), (Hazel, 0.08), (Green, 0.03), (Blue, 0.01)])
        }
        Ethnicity::SouthAsian => rng.weighted(&[(DarkBrown, 0.85), (Brown, 0.12), (LightBrown, 0.03)]),
        _ => rng.weighted(&[(DarkBrown, 0.9), (Brown, 0.1)]),
    }
}

fn eye_rgb(c: EyeColour, rng: &mut Rng) -> [u8; 3] {
    let (a, b) = match c {
        EyeColour::Blue => (0x4f6f8a, 0x5b7f9c),
        EyeColour::Green => (0x5d7a52, 0x6a7f4a),
        EyeColour::Hazel => (0x7a5a33, 0x86683a),
        EyeColour::LightBrown => (0x6b5030, 0x7a5a33),
        EyeColour::Brown => (0x3b2616, 0x4a3018),
        EyeColour::DarkBrown => (0x2a1a10, 0x3b2616),
    };
    lerp_rgb(rgb(a), rgb(b), rng.next())
}

fn hair_draw(e: Ethnicity, rng: &mut Rng) -> HairColour {
    use HairColour::*;
    match e {
        Ethnicity::White => {
            rng.weighted(&[(Blond, 0.12), (LightBrown, 0.25), (Brown, 0.30), (DarkBrown, 0.27), (Red, 0.06)])
        }
        Ethnicity::Hispanic => rng.weighted(&[(Black, 0.70), (DarkBrown, 0.24), (Brown, 0.05), (LightBrown, 0.01)]),
        Ethnicity::SouthAsian => rng.weighted(&[(Black, 0.92), (DarkBrown, 0.08)]),
        _ => Black,
    }
}

fn hair_rgb(c: HairColour, rng: &mut Rng) -> [u8; 3] {
    let (a, b) = match c {
        HairColour::Black => (0x110d0a, 0x1a1410),
        HairColour::DarkBrown => (0x2e2016, 0x3a2618),
        HairColour::Brown => (0x5a3e26, 0x6a4a2c),
        HairColour::LightBrown => (0x7c5e3a, 0x8a6a42),
        HairColour::Blond => (0xb8925a, 0xc9a46a),
        HairColour::Red => (0x8c4a2a, 0x9a5632),
    };
    lerp_rgb(rgb(a), rgb(b), rng.next())
}

fn texture_draw(e: Ethnicity, rng: &mut Rng) -> Texture {
    use Texture::*;
    match e {
        Ethnicity::White => rng.weighted(&[(Straight, 0.65), (Wavy, 0.27), (Curly, 0.08)]),
        Ethnicity::Black => rng.weighted(&[(Coily, 0.93), (Curly, 0.07)]),
        Ethnicity::Hispanic => rng.weighted(&[(Straight, 0.45), (Wavy, 0.40), (Curly, 0.13), (Coily, 0.02)]),
        Ethnicity::Polynesian => rng.weighted(&[(Wavy, 0.55), (Curly, 0.30), (Coily, 0.10), (Straight, 0.05)]),
        Ethnicity::EastAsian => rng.weighted(&[(Straight, 0.92), (Wavy, 0.08)]),
        Ethnicity::SoutheastAsian => rng.weighted(&[(Straight, 0.75), (Wavy, 0.20), (Curly, 0.05)]),
        Ethnicity::SouthAsian => rng.weighted(&[(Straight, 0.35), (Wavy, 0.50), (Curly, 0.15)]),
    }
}

/// What he is born with, from his looks number and heritage.
pub fn genes(looks: Looks, heritage: Ethnicity) -> Genes {
    let mut rng = keyed(looks.0, T_GENES, 0);
    let g = gi(heritage);
    let mixed = if rng.chance(MIXED[g]) {
        let w: Vec<(usize, f64)> = (0..7).map(|k| (k, PARTNER[g][k])).collect();
        let other = Ethnicity::ALL[rng.weighted(&w)];
        Some((other, rng.range(0.15, 0.5) as f32))
    } else {
        None
    };
    let face = rng.next_u32();
    // Skin: a place in the group's range, blended toward the other parent's.
    let (lo, hi) = SKIN[g];
    let t1 = rng.next();
    let mut skin = lerp_rgb(rgb(lo), rgb(hi), t1);
    let t2 = rng.next();
    if let Some((o, share)) = mixed {
        let (lo2, hi2) = SKIN[gi(o)];
        let other = lerp_rgb(rgb(lo2), rgb(hi2), t2);
        skin = lerp_rgb(skin, other, share as f64 * rng.range(0.7, 1.3));
    }
    let lum = (0.299 * skin[0] as f64 + 0.587 * skin[1] as f64 + 0.114 * skin[2] as f64) / 255.0;
    let darkness = ((0.82 - lum) / 0.55).clamp(0.0, 1.0) as f32;
    // Eyes, hair and texture come from one parent's side: the other side's
    // only now and then for eye and hair colour (dark is dominant), half the
    // time for texture.
    let side = |rng: &mut Rng, p: f64| match mixed {
        Some((o, share)) if rng.chance(share as f64 * p) => o,
        _ => heritage,
    };
    let eye_from = side(&mut rng, 0.5);
    let eye = eye_draw(eye_from, &mut rng);
    let eye_rgb = eye_rgb(eye, &mut rng);
    let hair_from = side(&mut rng, 0.4);
    let hair = hair_draw(hair_from, &mut rng);
    let hair_rgb = hair_rgb(hair, &mut rng);
    let tex_from = side(&mut rng, 1.0);
    let mut texture = texture_draw(tex_from, &mut rng);
    if let Some((o, share)) = mixed
        && (o == Ethnicity::Black || heritage == Ethnicity::Black)
        && share >= 0.25
    {
        texture = rng.weighted(&[(Texture::Curly, 0.55), (Texture::Coily, 0.35), (Texture::Wavy, 0.10)]);
    }
    let low_beard = matches!(heritage, Ethnicity::EastAsian | Ethnicity::SoutheastAsian);
    let beard = BeardGenes {
        density: if low_beard { rng.range(0.15, 0.6) } else { rng.range(0.4, 0.95) } as f32,
        patch: rng.range(0.1, 0.55) as f32,
        cheek: rng.range(0.15, 0.6) as f32,
        neck: rng.range(0.1, 0.4) as f32,
        onset: rng.normal(17.5, 1.2).clamp(15.0, 21.0) as f32,
    };
    // Hairline: Norwood 3 or worse at 18-29 in about 16% of White men, 2-3%
    // of Korean and Chinese men, 11% of Nigerian men (face-traits.md).
    let p_bald = [0.55, 0.25, 0.35, 0.2, 0.1, 0.15, 0.35][g];
    let balding = rng.chance(p_bald).then(|| (rng.normal(29.0, 9.0).max(17.0) as f32, rng.range(0.12, 0.3) as f32));
    let grey_at = [34.0, 43.0, 36.0, 38.0, 38.0, 39.0, 33.0][g];
    let grey_onset = rng.normal(grey_at, 6.0).clamp(22.0, 60.0) as f32;
    let grey_rate = rng.range(0.02, 0.05) as f32;
    let fr = match heritage {
        Ethnicity::White => match hair {
            HairColour::Red => 0.85,
            HairColour::Blond => 0.4,
            _ => 0.15,
        },
        Ethnicity::EastAsian => 0.04,
        Ethnicity::SoutheastAsian => 0.03,
        Ethnicity::Hispanic => 0.08,
        Ethnicity::Black => 0.01,
        _ => 0.02,
    };
    let freckles = if rng.chance(fr) { rng.range(0.3, 1.0) as f32 } else { 0.0 };
    let mole_mean = [2.5, 0.7, 1.6, 1.0, 1.3, 1.1, 1.1][g];
    let moles = (0..8).filter(|_| rng.chance(mole_mean / 8.0)).count() as u8;
    let acne_scars = if rng.chance(if heritage == Ethnicity::EastAsian { 0.30 } else { 0.22 }) {
        rng.range(0.2, 0.8) as f32
    } else {
        0.0
    };
    let pfb_p = match heritage {
        Ethnicity::Black => 0.55,
        Ethnicity::Hispanic => 0.10,
        _ => 0.05,
    };
    let razor_bumps = if rng.chance(pfb_p) { rng.range(0.4, 1.0) as f32 } else { 0.0 };
    let dpn = if heritage == Ethnicity::Black && rng.chance(0.10) { rng.range(0.3, 0.8) as f32 } else { 0.0 };
    let lip2_p = [0.05, 0.5, 0.1, 0.2, 0.05, 0.05, 0.2][g];
    let two_tone_lips = if rng.chance(lip2_p) { rng.range(0.5, 1.0) as f32 } else { 0.0 };
    let crease_p = [0.99, 0.99, 0.97, 0.95, 0.45, 0.85, 0.99][g];
    let lid_crease = rng.chance(crease_p);
    let crease_mean = [6.2, 7.2, 6.5, 6.5, 4.0, 6.0, 7.0][g];
    let crease_mm = rng.normal(crease_mean, 1.8).max(2.5) as f32;
    // A gap between the front teeth (midline diastema) in young men; braces
    // close most of them, so these are the ones left (estimate).
    let tooth_gap = rng.chance(if heritage == Ethnicity::Black { 0.08 } else { 0.035 });
    let childhood_scar = if rng.chance(0.09) {
        Some(rng.weighted(&[(ScarSpot::Chin, 0.55), (ScarSpot::Brow, 0.3), (ScarSpot::Lip, 0.15)]))
    } else {
        None
    };
    let childhood_nose = if rng.chance(0.03) { rng.range(0.2, 0.6) as f32 } else { 0.0 };
    let bmi_offset = [0.0, -1.3, 0.0, -4.5, 1.9, 3.0, 3.0][g] as f32;
    let muscle = rng.next() as f32;
    Genes {
        heritage,
        mixed,
        face,
        skin,
        darkness,
        eye,
        eye_rgb,
        hair,
        hair_rgb,
        texture,
        beard,
        balding,
        grey_onset,
        grey_rate,
        freckles,
        moles,
        acne_scars,
        razor_bumps,
        dpn,
        two_tone_lips,
        lid_crease,
        crease_mm,
        tooth_gap,
        childhood_scar,
        childhood_nose,
        bmi_offset,
        muscle,
    }
}

// --- Haircuts, beards and the rest -------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HairStyle {
    Shaved,
    Buzz,
    Crop,
    CropFade,
    SidePart,
    Flow,
    Long,
    ManBun,
    Mullet,
    TwoBlock,
    CurlyTop,
    Waves,
    SpongeTop,
    HighTop,
    Afro,
    Cornrows,
    Braids,
    Twists,
    Locs,
}

pub const HAIR_STYLES: [HairStyle; 19] = [
    HairStyle::Shaved,
    HairStyle::Buzz,
    HairStyle::Crop,
    HairStyle::CropFade,
    HairStyle::SidePart,
    HairStyle::Flow,
    HairStyle::Long,
    HairStyle::ManBun,
    HairStyle::Mullet,
    HairStyle::TwoBlock,
    HairStyle::CurlyTop,
    HairStyle::Waves,
    HairStyle::SpongeTop,
    HairStyle::HighTop,
    HairStyle::Afro,
    HairStyle::Cornrows,
    HairStyle::Braids,
    HairStyle::Twists,
    HairStyle::Locs,
];

/// How popular each haircut is among football players in 1985, 1995, 2005,
/// 2015 and 2025: for coily hair, then for straight to curly hair
/// (Claude's estimates from roster photos of each era; proposed, not
/// confirmed). The high-top fades of the late 80s, cornrows around 2000,
/// the locs of the 2010s, sponge twists and the mullet's return in the 2020s.
const COILY_ERA: [[f64; 5]; 19] = [
    [0.14, 0.20, 0.18, 0.10, 0.08],
    [0.40, 0.30, 0.22, 0.14, 0.10],
    [0.0; 5],
    [0.0; 5],
    [0.0; 5],
    [0.0; 5],
    [0.0; 5],
    [0.0; 5],
    [0.0; 5],
    [0.0; 5],
    [0.0, 0.0, 0.01, 0.03, 0.05],
    [0.04, 0.08, 0.12, 0.14, 0.13],
    [0.0, 0.02, 0.03, 0.08, 0.15],
    [0.25, 0.10, 0.01, 0.01, 0.01],
    [0.03, 0.03, 0.05, 0.03, 0.04],
    [0.02, 0.10, 0.17, 0.05, 0.03],
    [0.01, 0.03, 0.05, 0.05, 0.05],
    [0.0, 0.02, 0.03, 0.10, 0.16],
    [0.005, 0.015, 0.04, 0.10, 0.08],
];
const STRAIGHT_ERA: [[f64; 5]; 19] = [
    [0.03, 0.05, 0.06, 0.05, 0.04],
    [0.22, 0.22, 0.18, 0.14, 0.14],
    [0.30, 0.32, 0.30, 0.24, 0.20],
    [0.04, 0.05, 0.08, 0.18, 0.20],
    [0.16, 0.10, 0.07, 0.08, 0.07],
    [0.12, 0.12, 0.15, 0.14, 0.12],
    [0.08, 0.06, 0.07, 0.04, 0.03],
    [0.0, 0.0, 0.0, 0.05, 0.03],
    [0.10, 0.02, 0.0, 0.0, 0.07],
    [0.0, 0.0, 0.0, 0.04, 0.10],
    [0.0, 0.0, 0.01, 0.04, 0.08],
    [0.0; 5],
    [0.0; 5],
    [0.0; 5],
    [0.02, 0.02, 0.02, 0.02, 0.02],
    [0.0, 0.01, 0.01, 0.0, 0.0],
    [0.0, 0.0, 0.01, 0.01, 0.01],
    [0.0, 0.0, 0.01, 0.01, 0.02],
    [0.0, 0.005, 0.01, 0.02, 0.02],
];

fn era_value(row: &[f64; 5], year: i32) -> f64 {
    let x = ((year as f64 - 1985.0) / 10.0).clamp(0.0, 4.0);
    let i = (x.floor() as usize).min(3);
    row[i] + (row[i + 1] - row[i]) * (x - i as f64)
}

/// Fashion after 2025: each style drifts on the world's own key by decade,
/// and styles that were once big can come back (a high-top revival).
fn fashion(world_seed: u32, s: usize, row: &[f64; 5], year: i32) -> f64 {
    let base = era_value(row, year);
    if year <= 2025 {
        return base;
    }
    let peak = row.iter().copied().fold(0.0, f64::max);
    let floor = base.max(0.15 * peak);
    let at = |dec: i32| {
        let mut r = keyed(world_seed, SALT_FASHION, mix(dec as u32, s as u32));
        r.normal(0.0, 0.5)
    };
    let d = (year - 2025) as f64 / 10.0;
    let k = d.floor() as i32;
    let f = d - k as f64;
    let z = at(k) * (1.0 - f) + at(k + 1) * f;
    floor * (z * (d / 1.5).min(1.0)).exp()
}

impl HairStyle {
    pub fn key(self) -> &'static str {
        match self {
            HairStyle::Shaved => "shaved",
            HairStyle::Buzz => "buzz",
            HairStyle::Crop => "crop",
            HairStyle::CropFade => "crop_fade",
            HairStyle::SidePart => "side_part",
            HairStyle::Flow => "flow",
            HairStyle::Long => "long",
            HairStyle::ManBun => "bun",
            HairStyle::Mullet => "mullet",
            HairStyle::TwoBlock => "two_block",
            HairStyle::CurlyTop => "curly_top",
            HairStyle::Waves => "waves",
            HairStyle::SpongeTop => "sponge_top",
            HairStyle::HighTop => "high_top",
            HairStyle::Afro => "afro",
            HairStyle::Cornrows => "cornrows",
            HairStyle::Braids => "braids",
            HairStyle::Twists => "twists",
            HairStyle::Locs => "locs",
        }
    }

    /// Kept long: grown over years and cut only now and then.
    fn grown(self) -> bool {
        matches!(self, HairStyle::Locs | HairStyle::Long | HairStyle::ManBun | HairStyle::Afro)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Dye {
    None,
    BlondTips,
    Bleached,
    Honey,
    Copper,
    TeamTips,
    GreyCover,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Beard {
    Clean,
    Stubble,
    Moustache,
    MousChin,
    Goatee,
    Circle,
    ChinStrap,
    FullShort,
    Full,
    FullLong,
}

const BEARDS: [Beard; 10] = [
    Beard::Clean,
    Beard::Stubble,
    Beard::Moustache,
    Beard::MousChin,
    Beard::Goatee,
    Beard::Circle,
    Beard::ChinStrap,
    Beard::FullShort,
    Beard::Full,
    Beard::FullLong,
];

/// Facial hair among grown players by era (1985-2025, as above): the
/// moustaches of the 80s, goatees and circle beards around 2000, the beard
/// boom after 2012 (Claude's estimates; proposed, not confirmed).
const BEARD_ERA: [[f64; 5]; 10] = [
    [0.58, 0.50, 0.42, 0.28, 0.24],
    [0.06, 0.08, 0.10, 0.14, 0.14],
    [0.12, 0.06, 0.03, 0.03, 0.05],
    [0.03, 0.05, 0.06, 0.05, 0.06],
    [0.08, 0.12, 0.14, 0.08, 0.06],
    [0.05, 0.10, 0.12, 0.10, 0.08],
    [0.02, 0.05, 0.07, 0.04, 0.03],
    [0.04, 0.03, 0.04, 0.16, 0.18],
    [0.02, 0.01, 0.02, 0.09, 0.12],
    [0.0, 0.0, 0.0, 0.03, 0.04],
];

impl Beard {
    pub fn key(self) -> &'static str {
        match self {
            Beard::Clean => "none",
            Beard::Stubble => "stubble",
            Beard::Moustache => "moustache",
            Beard::MousChin => "mous_chin",
            Beard::Goatee => "goatee",
            Beard::Circle => "circle",
            Beard::ChinStrap => "chin_strap",
            Beard::FullShort => "full_short",
            Beard::Full => "full",
            Beard::FullLong => "full_long",
        }
    }
    fn needs_density(self) -> bool {
        matches!(self, Beard::Circle | Beard::ChinStrap | Beard::FullShort | Beard::Full | Beard::FullLong)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Earrings {
    None,
    Stud,
    Studs,
    Hoops,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InkStyle {
    Script,
    Portrait,
    Religious,
    Tribal,
    Polynesian,
    Mixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Build {
    Lean,
    Athletic,
    Muscular,
    Solid,
    Thick,
    HeavySet,
}

impl Build {
    pub fn word(self) -> &'static str {
        match self {
            Build::Lean => "lean",
            Build::Athletic => "athletic",
            Build::Muscular => "muscular",
            Build::Solid => "solid",
            Build::Thick => "thick-set",
            Build::HeavySet => "heavy-set",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Expression {
    Smile,
    SlightSmile,
    GameFace,
    TightSmile,
    Smirk,
}

impl Expression {
    pub fn phrase(self) -> &'static str {
        match self {
            Expression::Smile => "a broad smile",
            Expression::SlightSmile => "a slight smile",
            Expression::GameFace => "a serious game face",
            Expression::TightSmile => "a closed-mouth smile",
            Expression::Smirk => "a half smirk",
        }
    }
}

/// Grooming rules where he plays: the service academies (military hair and
/// shaving standards, no earrings) and BYU's honor code (clean-shaven, no
/// long or extreme hair).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grooming {
    Free,
    Academy,
    HonorCode,
}

/// How he looks in one season.
#[derive(Clone, Debug, Serialize)]
pub struct Appearance {
    pub year: i32,
    pub age: u32,
    pub hair: HairStyle,
    /// Seasons he has worn it (locs and long hair grow).
    pub hair_years: u32,
    /// Length on top, inches.
    pub hair_len: f32,
    pub dye: Dye,
    pub grey: f32,
    /// Norwood stage of his hairline, 1 (full) to 7.
    pub norwood: f32,
    pub beard: Beard,
    pub beard_len: f32,
    pub scars: Vec<ScarSpot>,
    /// A nose broken at some point, 0 straight to 1 clearly crooked.
    pub nose: f32,
    /// 0 none, 1 one forearm, 2 both forearms, 3 a sleeve, 4 both arms sleeved.
    pub arm_ink: u8,
    pub neck_ink: bool,
    pub ink_style: InkStyle,
    pub earrings: Earrings,
    pub height: f32,
    pub weight: f32,
    pub bmi: f32,
    pub body_fat: f32,
    /// Face fullness for the drawn head, 0-1.
    pub face_fat: f32,
    pub muscle: f32,
    pub build: Build,
    pub glasses: bool,
}

/// One season of his life, as the looks walk reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeasonCtx {
    pub level: Option<Level>,
    pub grooming: Grooming,
    pub position: Option<Position>,
}

/// Typical BMI and body fat (%) by position for college and pro players
/// (combine bodies; DEXA studies of NFL players, Bosch 2014).
fn position_fat(pos: Option<Position>) -> (f64, f64, (f64, f64)) {
    use Position::*;
    match pos {
        Some(QB) => (28.5, 14.0, (0.55, 0.70)),
        Some(RB) => (30.0, 10.5, (0.80, 0.88)),
        Some(WR) => (26.5, 8.5, (0.75, 0.85)),
        Some(TE) => (31.0, 14.0, (0.72, 0.82)),
        Some(OT) => (37.0, 26.0, (0.60, 0.72)),
        Some(IOL) => (38.0, 27.0, (0.60, 0.72)),
        Some(EDGE) => (32.0, 14.0, (0.72, 0.82)),
        Some(DT) => (37.0, 24.0, (0.70, 0.80)),
        Some(LB) => (31.0, 13.0, (0.75, 0.85)),
        Some(CB) => (26.0, 8.0, (0.80, 0.88)),
        Some(S) => (27.5, 9.0, (0.80, 0.88)),
        Some(K) => (26.0, 15.0, (0.50, 0.62)),
        Some(P) => (27.0, 16.0, (0.50, 0.62)),
        Some(LS) => (31.0, 20.0, (0.55, 0.68)),
        // Not a player (a coach): an ordinary man of his size.
        None => (27.5, 24.0, (0.35, 0.5)),
    }
}

fn build_of(genes: &Genes, height: f64, weight: f64, age: u32, pos: Option<Position>, playing: bool) -> Appearance {
    let bmi = 703.0 * weight / (height * height).max(1.0);
    let (pos_bmi, pos_fat, (m0, m1)) = position_fat(pos);
    let eff = bmi + genes.bmi_offset as f64;
    let mut fat = pos_fat + 1.25 * (eff - pos_bmi);
    if !playing {
        fat += 3.0;
    }
    if age > 28 {
        fat += 0.3 * (age - 28) as f64;
    }
    if age < 18 {
        fat -= 1.0;
    }
    let fat = fat.clamp(5.0, 42.0);
    let muscle = (m0 + (m1 - m0) * genes.muscle as f64) * if playing { 1.0 } else { 0.85 };
    let build = if fat < 9.5 {
        if bmi >= 27.5 { Build::Muscular } else { Build::Lean }
    } else if fat < 14.0 {
        if bmi >= 30.0 { Build::Muscular } else { Build::Athletic }
    } else if fat < 19.0 {
        Build::Solid
    } else if fat < 25.0 {
        Build::Thick
    } else {
        Build::HeavySet
    };
    Appearance {
        year: 0,
        age,
        hair: HairStyle::Crop,
        hair_years: 0,
        hair_len: 1.5,
        dye: Dye::None,
        grey: 0.0,
        norwood: 1.0,
        beard: Beard::Clean,
        beard_len: 0.0,
        scars: Vec::new(),
        nose: 0.0,
        arm_ink: 0,
        neck_ink: false,
        ink_style: InkStyle::Script,
        earrings: Earrings::None,
        height: height as f32,
        weight: weight as f32,
        bmi: bmi as f32,
        body_fat: fat as f32,
        face_fat: ((fat - 8.0) / 22.0).clamp(0.0, 1.0) as f32,
        muscle: muscle as f32,
        build,
        glasses: false,
    }
}

fn norwood(genes: &Genes, age: f64) -> f64 {
    match genes.balding {
        None => 1.0 + 0.3 * ((age - 30.0) / 30.0).clamp(0.0, 1.0),
        Some((a3, speed)) => {
            let (a3, speed) = (a3 as f64, speed as f64);
            if age >= a3 {
                (3.0 + (age - a3) * speed).min(7.0)
            } else {
                1.0 + 2.0 * ((age - (a3 - 8.0)) / 8.0).clamp(0.0, 1.0)
            }
        }
    }
}

fn hair_weights(
    world_seed: u32,
    genes: &Genes,
    year: i32,
    age: u32,
    stage: f64,
    ctx: SeasonCtx,
    coach: bool,
) -> Vec<(HairStyle, f64)> {
    let coily = genes.texture == Texture::Coily;
    let table = if coily { &COILY_ERA } else { &STRAIGHT_ERA };
    let her = genes.heritage;
    HAIR_STYLES
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            // Grown men's hair follows the fashion of their own youth.
            let y = if coily || !coach { year } else { year - (age as i32 - 25).max(0) / 2 };
            let mut w = fashion(world_seed, i, &table[i], y);
            use HairStyle::*;
            match genes.texture {
                Texture::Straight => {
                    if matches!(s, CurlyTop | Afro | Locs | Braids | Twists | Cornrows) {
                        w *= 0.05;
                    }
                }
                Texture::Wavy => {
                    if matches!(s, Afro | Braids | Cornrows) {
                        w *= 0.2;
                    }
                }
                Texture::Curly => {
                    if matches!(s, SidePart | TwoBlock) {
                        w *= 0.3;
                    }
                    if matches!(s, CurlyTop | Afro) {
                        w *= 3.0;
                    }
                }
                Texture::Coily => {}
            }
            if s == TwoBlock && !matches!(her, Ethnicity::EastAsian | Ethnicity::SoutheastAsian) {
                w *= 0.05;
            }
            if her == Ethnicity::Polynesian && matches!(s, Long | ManBun | Locs) {
                w *= 4.0;
            }
            if age < 18 && matches!(s, Mullet | CurlyTop | SpongeTop) {
                w *= 1.5;
            }
            if age < 18 && s == Shaved {
                w *= 0.5;
            }
            if age >= 30 {
                if matches!(s, Shaved | Buzz) {
                    w *= 1.5;
                }
                if matches!(s, Mullet | TwoBlock | SpongeTop | CurlyTop | HighTop | Cornrows | Braids) {
                    w *= 0.5;
                }
            }
            if coach {
                if matches!(s, Locs | Long | ManBun | Braids | Cornrows | Twists | Afro | Mullet | SpongeTop) {
                    w *= 0.15;
                }
                if matches!(s, SidePart | Crop | Buzz) {
                    w *= 1.6;
                }
            }
            // A receding hairline: most take it short or shave it.
            if stage >= 3.0 {
                if s == Shaved {
                    w *= 1.0 + 1.5 * (stage - 2.0);
                } else if s == Buzz {
                    w *= 2.0;
                } else if matches!(s, Long | Flow | ManBun | SidePart | TwoBlock | Mullet) {
                    w *= 0.3;
                }
            }
            match ctx.grooming {
                Grooming::Academy if !matches!(s, Shaved | Buzz | Crop | CropFade | Waves | SidePart) => w = 0.0,
                Grooming::HonorCode if matches!(s, Long | ManBun | Locs | Braids | Mullet | Cornrows) => w *= 0.1,
                _ => {}
            }
            (s, w.max(0.0))
        })
        .collect()
}

fn beard_weights(genes: &Genes, year: i32, age: u32, ctx: SeasonCtx, coach: bool) -> Vec<(Beard, f64)> {
    let b = &genes.beard;
    let grown = age as f32 - b.onset;
    BEARDS
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let mut w = era_value(&BEARD_ERA[i], year.min(2025));
            let d = b.density as f64;
            let p = b.patch as f64;
            if s.needs_density() {
                w *= (d * d) * (1.25 - p);
            } else if matches!(s, Beard::Moustache | Beard::Goatee | Beard::MousChin) {
                w *= d.sqrt();
            } else {
                w *= 1.0 + 2.0 * (1.0 - d);
            }
            // Before the beard fills in: clean, a thin moustache or fuzz.
            if grown < -2.0 {
                w = if s == Beard::Clean { 1.0 } else { 0.0 };
            } else if grown < 0.0 {
                w *= match s {
                    Beard::Clean => 3.0,
                    Beard::Moustache | Beard::Stubble => 0.6,
                    _ => 0.05,
                };
            }
            if ctx.level == Some(Level::HS) && s.needs_density() {
                w *= 0.4;
            }
            // Razor bumps keep men from shaving close.
            if genes.razor_bumps > 0.0 && s == Beard::Clean {
                w *= 0.5;
            }
            if age >= 30 && s.needs_density() {
                w *= 1.2;
            }
            if coach && matches!(s, Beard::ChinStrap | Beard::FullLong | Beard::MousChin) {
                w *= 0.3;
            }
            match ctx.grooming {
                Grooming::Academy if s != Beard::Clean => w = 0.0,
                Grooming::HonorCode if !matches!(s, Beard::Clean | Beard::Moustache) => w = 0.0,
                _ => {}
            }
            (s, w.max(0.0))
        })
        .collect()
}

fn beard_len(b: Beard, rng: &mut Rng) -> f32 {
    let (lo, hi) = match b {
        Beard::Clean => (0.0, 0.0),
        Beard::Stubble => (0.015, 0.03),
        Beard::Moustache | Beard::MousChin | Beard::Goatee | Beard::Circle | Beard::ChinStrap => (0.04, 0.09),
        Beard::FullShort => (0.06, 0.12),
        Beard::Full => (0.12, 0.2),
        Beard::FullLong => (0.22, 0.35),
    };
    rng.range(lo, hi) as f32
}

fn hair_len(s: HairStyle, years: u32, rng: &mut Rng) -> f32 {
    let y = years as f64;
    (match s {
        HairStyle::Shaved => 0.03,
        HairStyle::Buzz => rng.range(0.2, 0.5),
        HairStyle::Waves => rng.range(0.4, 0.7),
        HairStyle::Crop | HairStyle::SidePart | HairStyle::CropFade | HairStyle::TwoBlock => rng.range(1.5, 3.0),
        HairStyle::CurlyTop | HairStyle::SpongeTop => rng.range(1.5, 2.5),
        HairStyle::HighTop => rng.range(3.0, 5.0),
        HairStyle::Mullet => rng.range(2.0, 3.0),
        HairStyle::Flow => rng.range(4.0, 7.0),
        HairStyle::Cornrows | HairStyle::Braids => rng.range(3.0, 8.0),
        HairStyle::Twists => (2.0 + 1.5 * y).min(6.0),
        HairStyle::Afro => (2.5 + 1.5 * y).min(7.0),
        HairStyle::Long | HairStyle::ManBun => (8.0 + 3.0 * y).min(20.0),
        HairStyle::Locs => (3.0 + 4.0 * y).min(26.0),
    }) as f32
}

/// Lifetime tattoo and earring leanings per group (Polynesian tatau, more
/// ink and earrings among Black players; Claude's estimates).
const INK: [f64; 7] = [1.0, 1.3, 1.2, 2.5, 0.6, 0.9, 0.5];
const EAR: [f64; 7] = [0.5, 2.2, 1.3, 1.6, 0.5, 0.8, 0.4];
/// How common ink and earrings were by era (1985-2025, as the haircut
/// tables): few tattooed players in the 80s, the boom from the late 90s.
const INK_ERA: [f64; 5] = [0.25, 0.5, 0.9, 1.0, 1.0];
const NECK_ERA: [f64; 5] = [0.1, 0.3, 0.7, 1.0, 1.0];
const EAR_ERA: [f64; 5] = [0.4, 0.9, 1.2, 1.0, 0.9];

/// His looks at the season `year`, walking from age 14 with one keyed draw
/// a year. `ctx(year)` says where he was each season; `body(age)` gives his
/// height and weight that season.
#[allow(clippy::too_many_arguments)]
pub fn appearance(
    world_seed: u32,
    looks: Looks,
    genes: &Genes,
    birth_year: i32,
    year: i32,
    ctx: impl Fn(i32) -> SeasonCtx,
    body: impl Fn(u32) -> (f64, f64),
    coach: bool,
) -> Appearance {
    let age_at = |y: i32| (y - birth_year).max(0) as u32;
    let start = (birth_year + 14).min(year);
    let g = gi(genes.heritage);
    let mut hair = HairStyle::Crop;
    let mut hair_years = 0u32;
    let mut beard = Beard::Clean;
    let mut scars: Vec<ScarSpot> = genes.childhood_scar.into_iter().collect();
    let mut nose = genes.childhood_nose;
    let mut arm_ink = 0u8;
    let mut neck_ink = false;
    let mut earrings = Earrings::None;
    let mut dye = Dye::None;
    let mut rng0 = keyed(looks.0, T_YEAR, 0);
    let ink_style = if genes.heritage == Ethnicity::Polynesian && rng0.chance(0.7) {
        InkStyle::Polynesian
    } else {
        rng0.weighted(&[
            (InkStyle::Script, 0.3),
            (InkStyle::Religious, 0.2),
            (InkStyle::Portrait, 0.15),
            (InkStyle::Tribal, 0.1),
            (InkStyle::Mixed, 0.25),
        ])
    };
    let mut blen = 0.0f32;
    let mut hlen = 1.5f32;
    for y in start..=year {
        let mut rng = keyed(looks.0, T_YEAR, y as u32);
        let age = age_at(y);
        let c = ctx(y);
        let stage = norwood(genes, age as f64);
        // Haircut: a new one at 14, then now and then; locs are a commitment.
        let p_change = if y == start {
            1.0
        } else if hair == HairStyle::Locs {
            0.10
        } else if hair.grown() || matches!(hair, HairStyle::Braids | HairStyle::Cornrows | HairStyle::Twists) {
            0.2
        } else if age < 18 {
            0.35
        } else if age < 30 {
            0.28
        } else {
            0.15
        };
        let w = hair_weights(world_seed, genes, y, age, stage, c, coach);
        let allowed = w.iter().find(|e| e.0 == hair).is_some_and(|e| e.1 > 0.0);
        if !allowed || rng.chance(p_change) {
            let mut w = w;
            // Twists kept two years often turn into starter locs.
            let to_locs = hair == HairStyle::Twists && hair_years >= 2 && rng.chance(0.3);
            if y != start {
                for e in w.iter_mut().filter(|e| e.0 == hair) {
                    e.1 *= 0.3;
                }
            }
            let total: f64 = w.iter().map(|e| e.1).sum();
            let next = if to_locs && w.iter().any(|e| e.0 == HairStyle::Locs && e.1 > 0.0) {
                HairStyle::Locs
            } else if total > 0.0 {
                rng.weighted(&w)
            } else {
                HairStyle::Buzz
            };
            if next != hair {
                hair = next;
                hair_years = 0;
            }
        }
        hair_years += 1;
        hlen = hair_len(hair, hair_years - 1, &mut rng);
        // Dye lasts a season.
        let grey = (((age as f32) - genes.grey_onset) * genes.grey_rate).clamp(0.0, 1.0);
        let p_dye = match hair {
            HairStyle::Shaved => 0.0,
            HairStyle::Locs => 0.14,
            HairStyle::Twists | HairStyle::SpongeTop | HairStyle::Afro | HairStyle::Braids => 0.08,
            _ if genes.texture == Texture::Coily => 0.03,
            _ => 0.02 * if (1998..=2005).contains(&y) { 4.0 } else { 1.0 },
        } * if genes.heritage == Ethnicity::Polynesian { 2.0 } else { 1.0 }
            * if coach { 0.0 } else { 1.0 };
        dye = if c.grooming == Grooming::Academy {
            Dye::None
        } else if rng.chance(p_dye) {
            if genes.texture == Texture::Coily || hair == HairStyle::Locs {
                rng.weighted(&[(Dye::BlondTips, 0.45), (Dye::Bleached, 0.3), (Dye::Copper, 0.1), (Dye::TeamTips, 0.15)])
            } else if (1998..=2005).contains(&y) {
                rng.weighted(&[(Dye::BlondTips, 0.7), (Dye::Bleached, 0.2), (Dye::Honey, 0.1)])
            } else {
                rng.weighted(&[(Dye::Bleached, 0.45), (Dye::Honey, 0.3), (Dye::BlondTips, 0.15), (Dye::Copper, 0.1)])
            }
        } else if grey > 0.3 && age >= 35 && rng.chance(if coach { 0.12 } else { 0.08 }) {
            Dye::GreyCover
        } else {
            Dye::None
        };
        // Facial hair, decided each off-season.
        let bw = beard_weights(genes, y, age, c, coach);
        let ok = bw.iter().find(|e| e.0 == beard).is_some_and(|e| e.1 > 0.0);
        if y == start || !ok || rng.chance(0.35) {
            let total: f64 = bw.iter().map(|e| e.1).sum();
            beard = if total > 0.0 { rng.weighted(&bw) } else { Beard::Clean };
            blen = beard_len(beard, &mut rng);
        }
        // Marks from a season of contact football (looks only: the injury
        // model has no face injuries).
        let lineman = matches!(c.position, Some(Position::OT | Position::IOL | Position::DT | Position::EDGE));
        let contact = matches!(c.position, Some(Position::LB | Position::TE | Position::RB));
        if let Some(level) = c.level {
            let p_scar = match level {
                Level::HS => 0.004,
                Level::NFL => 0.008,
                _ => 0.006,
            };
            if rng.chance(p_scar) {
                let spot = rng.weighted(&[
                    (ScarSpot::Chin, 0.35),
                    (ScarSpot::Brow, 0.25),
                    (ScarSpot::Temple, 0.15),
                    (ScarSpot::Cheek, 0.1),
                    (ScarSpot::NoseBridge, 0.1),
                    (ScarSpot::Lip, 0.05),
                ]);
                if !scars.contains(&spot) {
                    scars.push(spot);
                }
            }
            let p_nose = if lineman {
                0.010
            } else if contact {
                0.006
            } else {
                0.004
            };
            if rng.chance(p_nose) {
                nose = (nose + rng.range(0.25, 0.7) as f32).min(1.0);
            }
        }
        // Tattoos: rare before 18 (parental consent), common in college and
        // the NFL (about half of NFL players show arm ink).
        if age >= 16 && !coach {
            let base = match c.level {
                Some(Level::HS) => {
                    if age < 18 {
                        0.02
                    } else {
                        0.05
                    }
                }
                Some(Level::NFL) => 0.10,
                Some(_) => 0.09,
                None => 0.04,
            };
            if arm_ink == 0 {
                if rng.chance(base * INK[g] * era_value(&INK_ERA, y)) {
                    arm_ink = 1;
                }
            } else if arm_ink < 4 && rng.chance(0.28) {
                arm_ink += 1;
            }
            let p_neck = match c.level {
                Some(Level::NFL) => 0.04,
                Some(Level::HS) => 0.0,
                _ => 0.025,
            } * if genes.heritage == Ethnicity::Black { 1.3 } else { 0.8 }
                * era_value(&NECK_ERA, y);
            if arm_ink >= 2 && !neck_ink && c.grooming != Grooming::Academy && rng.chance(p_neck) {
                neck_ink = true;
            }
        }
        // Earrings: put in and taken out over the years.
        if age >= 15 {
            if earrings == Earrings::None {
                let p = if coach { 0.002 } else { 0.025 } * EAR[g] * era_value(&EAR_ERA, y);
                if rng.chance(p) {
                    earrings = rng.weighted(&[(Earrings::Stud, 0.35), (Earrings::Studs, 0.5), (Earrings::Hoops, 0.15)]);
                }
            } else if rng.chance(if coach { 0.25 } else { 0.08 }) {
                earrings = Earrings::None;
            }
        }
    }
    let age = age_at(year);
    let c = ctx(year);
    let (h, wt) = body(age);
    let mut a = build_of(genes, h, wt, age, c.position, c.level.is_some());
    let mut rng = keyed(looks.0, T_YEAR, year as u32 ^ 0x8000_0000);
    a.year = year;
    a.hair = hair;
    a.hair_years = hair_years;
    a.hair_len = hlen;
    a.dye = dye;
    a.grey = (((age as f32) - genes.grey_onset) * genes.grey_rate).clamp(0.0, 1.0);
    a.norwood = norwood(genes, age as f64) as f32;
    a.beard = beard;
    a.beard_len = blen;
    a.scars = scars;
    a.nose = nose;
    a.arm_ink = arm_ink;
    a.neck_ink = neck_ink;
    a.ink_style = ink_style;
    a.earrings = if c.grooming == Grooming::Academy { Earrings::None } else { earrings };
    a.glasses = coach && rng.chance(if age >= 50 { 0.3 } else { 0.15 });
    a
}

// --- Headshots ---------------------------------------------------------------------------------------

/// Where a headshot was taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum PhotoTeam {
    None,
    HighSchool(usize),
    College(usize),
    Nfl(usize),
}

/// One team headshot.
#[derive(Clone, Debug, Serialize)]
pub struct Photo {
    /// Stable name of this headshot ("c55-2027"): the drawn picture is
    /// saved under it.
    pub key: String,
    pub year: i32,
    pub team: PhotoTeam,
    /// Taken when he joined the team (true) or on a media day.
    pub joined: bool,
    pub expression: Expression,
}

/// A run of seasons with one team.
#[derive(Clone, Copy, Debug)]
struct Stint {
    team: PhotoTeam,
    level: Level,
    first: i32,
    last: i32,
}

fn grooming_of(w: &World, team: PhotoTeam) -> Grooming {
    match team {
        PhotoTeam::College(p) => match w.programs.get(p).map(|p| p.real_name.as_str()) {
            Some("Army" | "Navy" | "Air Force") => Grooming::Academy,
            Some("BYU") => Grooming::HonorCode,
            _ => Grooming::Free,
        },
        _ => Grooming::Free,
    }
}

impl World {
    /// The football season whose media day has passed: the season today's
    /// team photos are from.
    pub fn photo_season(&self) -> i32 {
        if self.today >= self.dates.media_days.0 { self.dates.season } else { self.dates.season - 1 }
    }

    fn stints(&self, p: &Person) -> Vec<Stint> {
        let season = self.dates.season;
        let mut out = Vec::new();
        let hs_first = p.class_of - 4;
        let hs_last = (p.class_of - 1).min(season);
        if hs_first <= hs_last {
            out.push(Stint { team: PhotoTeam::HighSchool(p.hs), level: Level::HS, first: hs_first, last: hs_last });
        }
        let level_of =
            |prog: usize| self.programs.get(prog).map(|pr| super::division_level(pr.division)).unwrap_or(Level::FBS);
        let mut college: Vec<Stint> = Vec::new();
        for &(y, prog, _, _) in &p.college_staff {
            match college.last_mut() {
                Some(s) if s.team == PhotoTeam::College(prog) && y <= s.last + 2 => s.last = s.last.max(y),
                _ => college.push(Stint { team: PhotoTeam::College(prog), level: level_of(prog), first: y, last: y }),
            }
        }
        if college.is_empty() && p.stage == Stage::Pro && !p.path.is_empty() {
            // A veteran from before history: his college years from his class.
            let prog = p.path[0];
            college.push(Stint {
                team: PhotoTeam::College(prog),
                level: level_of(prog),
                first: p.class_of,
                last: p.class_of + 3,
            });
        }
        if matches!(p.stage, Stage::College)
            && let Some(prog) = p.college
        {
            match college.last_mut() {
                Some(s) if s.team == PhotoTeam::College(prog) => s.last = s.last.max(season),
                _ => college.push(Stint {
                    team: PhotoTeam::College(prog),
                    level: level_of(prog),
                    first: season,
                    last: season,
                }),
            }
        }
        out.extend(college);
        if let Some(pro) = &p.pro {
            let mut nfl: Vec<Stint> = Vec::new();
            for s in &pro.seasons {
                let Some(t) = s.team else { continue };
                match nfl.last_mut() {
                    Some(n) if n.team == PhotoTeam::Nfl(t) && s.year <= n.last + 1 => n.last = n.last.max(s.year),
                    _ => nfl.push(Stint { team: PhotoTeam::Nfl(t), level: Level::NFL, first: s.year, last: s.year }),
                }
            }
            if let Some(t) = pro.team {
                match nfl.last_mut() {
                    Some(n) if n.team == PhotoTeam::Nfl(t) => n.last = n.last.max(season),
                    _ => nfl.push(Stint { team: PhotoTeam::Nfl(t), level: Level::NFL, first: season, last: season }),
                }
            }
            out.extend(nfl);
        }
        out
    }

    /// Every team headshot he has had, oldest first: one on joining each
    /// team, then every 4 years on media day (high schoolers every year).
    pub fn photos(&self, p: &Person) -> Vec<Photo> {
        let now = self.photo_season();
        let season = self.dates.season;
        let mut out = Vec::new();
        for s in self.stints(p) {
            let step = if s.level == Level::HS { 1 } else { 4 };
            let mut y = s.first;
            while y <= s.last {
                let joined = y == s.first && s.level != Level::HS;
                if y > now && !(joined && y <= season) {
                    break;
                }
                let (tag, id) = match s.team {
                    PhotoTeam::HighSchool(i) => ('h', i),
                    PhotoTeam::College(i) => ('c', i),
                    PhotoTeam::Nfl(i) => ('n', i),
                    PhotoTeam::None => ('x', 0),
                };
                let mut r = keyed(p.looks.0, T_PHOTO, mix(y as u32, id as u32));
                let expression = r.weighted(&[
                    (Expression::Smile, if s.level == Level::HS { 0.35 } else { 0.22 }),
                    (Expression::SlightSmile, 0.3),
                    (Expression::GameFace, if s.level == Level::NFL { 0.3 } else { 0.22 }),
                    (Expression::TightSmile, 0.12),
                    (Expression::Smirk, 0.08),
                ]);
                out.push(Photo { key: format!("{tag}{id}-{y}"), year: y, team: s.team, joined, expression });
                y += step;
            }
        }
        out
    }

    /// Where he was in a season, for the looks walk.
    fn season_ctx(&self, p: &Person, stints: &[Stint], year: i32) -> SeasonCtx {
        let s = stints.iter().rev().find(|s| s.first <= year && year <= s.last);
        let position = p
            .player
            .position_history
            .iter()
            .rev()
            .find(|&&(age, _)| (age as i32) <= year - (p.class_of - 18))
            .map(|&(_, pos)| pos)
            .or(Some(p.player.position));
        SeasonCtx {
            level: s.map(|s| s.level),
            grooming: s.map(|s| grooming_of(self, s.team)).unwrap_or(Grooming::Free),
            position,
        }
    }

    /// His genes and his looks in `year` (his current photo's season when
    /// None).
    pub fn face_of(&self, p: &Person, year: Option<i32>) -> Face {
        let looks = if p.looks.is_set() { p.looks } else { Looks::new(self.options.seed, p.player.id) };
        let genes = genes(looks, p.ethnicity);
        let photos = self.photos(p);
        let photo = photos.last().cloned();
        let year = year.or(photo.as_ref().map(|ph| ph.year)).unwrap_or(self.photo_season());
        let stints = self.stints(p);
        let birth = p.class_of - 18;
        let player = &p.player;
        let appearance = appearance(
            self.options.seed,
            looks,
            &genes,
            birth,
            year,
            |y| self.season_ctx(p, &stints, y),
            |age| {
                let mut q = player.clone();
                q.age = age;
                let b = crate::body::body_now(&q);
                (b.height, b.weight)
            },
            false,
        );
        Face { looks, genes, appearance, photo, photos }
    }

    /// A coach's face today: nothing stored, read from his id.
    pub fn coach_face(&self, coach: usize) -> Option<Face> {
        let c = self.coaches.get(coach)?;
        let looks = Looks::coach(self.options.seed, coach);
        let genes = genes(looks, c.ethnicity);
        let year = self.dates.season;
        let birth = year - c.age as i32;
        // Coaches are often old players: a lineman's frame for some.
        let mut r = keyed(looks.0, T_GENES, 7);
        let height = r.normal(72.5, 2.6);
        let bmi = r.normal(28.5, 3.2).clamp(22.0, 40.0) + if c.ex_pro { 1.0 } else { 0.0 };
        let weight = bmi * height * height / 703.0;
        let appearance = appearance(
            self.options.seed,
            looks,
            &genes,
            birth,
            year,
            |_| SeasonCtx { level: None, grooming: Grooming::Free, position: None },
            |_| (height, weight),
            true,
        );
        Some(Face { looks, genes, appearance, photo: None, photos: Vec::new() })
    }

    /// Fills in what a world written before faces didn't keep: a looks
    /// number for everyone, and the Asian groups split again by surname
    /// (the file stored one "Asian" group, which loads as East Asian).
    pub fn faces_after_load(&mut self) {
        if self.faces_version >= 1 {
            return;
        }
        let seed = self.options.seed;
        let split = |name: &str, key: u32| -> Ethnicity {
            super::names::asian_group_of(super::names::surname_of(name)).unwrap_or_else(|| {
                let mut r = keyed(seed, T_SPLIT, key);
                let [e, se, so] = super::ethnicity::ASIAN_SPLIT;
                r.weighted(&[(Ethnicity::EastAsian, e), (Ethnicity::SoutheastAsian, se), (Ethnicity::SouthAsian, so)])
            })
        };
        for p in &mut self.people {
            if !p.looks.is_set() {
                p.looks = Looks::new(seed, p.player.id);
            }
            if p.ethnicity == Ethnicity::EastAsian {
                p.ethnicity = split(&p.name, p.player.id);
            }
        }
        for (i, c) in self.coaches.iter_mut().enumerate() {
            if c.ethnicity == Ethnicity::EastAsian {
                c.ethnicity = split(&c.name, 0x4000_0000 ^ i as u32);
            }
        }
        for (i, a) in self.alumni.iter_mut().enumerate() {
            if a.ethnicity == Ethnicity::EastAsian {
                a.ethnicity = split(&a.name, 0x2000_0000 ^ i as u32);
            }
        }
        if let Some(c) = self.career.as_mut()
            && c.you.ethnicity == Ethnicity::EastAsian
        {
            c.you.ethnicity = split(&c.you.name, 0x1000_0000);
        }
        self.faces_version = 1;
    }
}

/// Everything a screen needs to show a face.
#[derive(Clone, Debug, Serialize)]
pub struct Face {
    #[serde(skip)]
    pub looks: Looks,
    pub genes: Genes,
    pub appearance: Appearance,
    /// His current team headshot (None for someone who never had one).
    pub photo: Option<Photo>,
    pub photos: Vec<Photo>,
}

// --- Words for the AI prompt and the screens -------------------------------------------------------

impl Appearance {
    /// The hair in words: "shoulder-length locs with bleached blond tips".
    pub fn hair_words(&self, genes: &Genes) -> String {
        let colour = if self.grey >= 0.6 {
            "grey".to_string()
        } else if self.grey >= 0.25 {
            format!("{}, greying", genes.hair.phrase())
        } else {
            genes.hair.phrase().to_string()
        };
        let style = match self.hair {
            HairStyle::Shaved => "a shaved head".to_string(),
            HairStyle::Buzz => {
                if genes.texture == Texture::Coily {
                    format!("a low {colour} buzz cut with a fade")
                } else {
                    format!("a short {colour} buzz cut")
                }
            }
            HairStyle::Crop => format!("a short textured {colour} crop"),
            HairStyle::CropFade => format!("{colour} hair in a skin fade with a longer textured top"),
            HairStyle::SidePart => format!("short {colour} hair with a side part"),
            HairStyle::Flow => format!("chin-length {colour} hair swept back (a flow)"),
            HairStyle::Long => format!("long {colour} hair past the shoulders"),
            HairStyle::ManBun => format!("long {colour} hair tied up in a bun"),
            HairStyle::Mullet => format!("a {colour} mullet"),
            HairStyle::TwoBlock => format!("a {colour} two-block cut with a long fringe"),
            HairStyle::CurlyTop => format!("a {colour} curly top over a fade"),
            HairStyle::Waves => format!("{colour} 360 waves with a low fade"),
            HairStyle::SpongeTop => format!("short {colour} sponge twists over a taper"),
            HairStyle::HighTop => format!("a {colour} high-top fade"),
            HairStyle::Afro => format!("a rounded {colour} afro"),
            HairStyle::Cornrows => format!("{colour} cornrows"),
            HairStyle::Braids => format!("{colour} box braids"),
            HairStyle::Twists => format!("{colour} two-strand twists"),
            HairStyle::Locs => {
                let len = if self.hair_len < 6.0 {
                    "short"
                } else if self.hair_len < 12.0 {
                    "shoulder-length"
                } else {
                    "long"
                };
                format!("{len} {colour} locs")
            }
        };
        let dye = match self.dye {
            Dye::None | Dye::GreyCover => "",
            Dye::BlondTips => " with bleached blond tips",
            Dye::Bleached => ", bleached blond",
            Dye::Honey => ", dyed honey blond",
            Dye::Copper => ", dyed copper red",
            Dye::TeamTips => " with tips dyed in his team's colour",
        };
        let line = if self.norwood >= 5.0 && self.hair != HairStyle::Shaved {
            ", balding on top"
        } else if self.norwood >= 3.0 && self.hair != HairStyle::Shaved {
            ", receding at the temples"
        } else {
            ""
        };
        format!("{style}{dye}{line}")
    }

    pub fn beard_words(&self, genes: &Genes) -> String {
        let patchy = genes.beard.patch > 0.42 && self.beard.needs_density();
        let s = match self.beard {
            Beard::Clean => "clean-shaven",
            Beard::Stubble => "stubble",
            Beard::Moustache => "a moustache",
            Beard::MousChin => "a moustache and a chin strip",
            Beard::Goatee => "a goatee",
            Beard::Circle => "a circle beard",
            Beard::ChinStrap => "a chin-strap beard",
            Beard::FullShort => "a short full beard",
            Beard::Full => "a full beard",
            Beard::FullLong => "a long full beard",
        };
        if patchy { format!("a patchy {}", s.trim_start_matches("a ")) } else { s.to_string() }
    }

    /// Marks, ink and small features, each in a few words.
    pub fn marks_words(&self, genes: &Genes) -> Vec<String> {
        let mut v: Vec<String> = self.scars.iter().map(|s| s.phrase().to_string()).collect();
        if self.nose >= 0.35 {
            v.push("a nose that has been broken, slightly crooked".into());
        } else if self.nose > 0.0 {
            v.push("a slight bump on the bridge of the nose".into());
        }
        let style = match self.ink_style {
            InkStyle::Script => "script",
            InkStyle::Portrait => "portrait",
            InkStyle::Religious => "religious",
            InkStyle::Tribal => "tribal",
            InkStyle::Polynesian => "Polynesian (tatau)",
            InkStyle::Mixed => "mixed",
        };
        match self.arm_ink {
            0 => {}
            1 => v.push(format!("a {style} tattoo on one forearm")),
            2 => v.push(format!("{style} tattoos on both forearms")),
            3 => v.push(format!("a full {style} tattoo sleeve on one arm")),
            _ => v.push(format!("{style} tattoo sleeves on both arms")),
        }
        if self.neck_ink {
            v.push("a tattoo on the side of the neck".into());
        }
        match self.earrings {
            Earrings::None => {}
            Earrings::Stud => v.push("a diamond stud in one ear".into()),
            Earrings::Studs => v.push("diamond studs in both ears".into()),
            Earrings::Hoops => v.push("small hoop earrings".into()),
        }
        if genes.tooth_gap {
            v.push("a small gap between his front teeth".into());
        }
        if genes.freckles >= 0.5 {
            v.push("freckles across the nose and cheeks".into());
        } else if genes.freckles > 0.0 {
            v.push("light freckles".into());
        }
        if genes.acne_scars >= 0.5 {
            v.push("light acne scarring on the cheeks".into());
        }
        if genes.moles >= 4 {
            v.push("a few small moles".into());
        }
        if self.glasses {
            v.push("glasses".into());
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_number_gives_the_same_face_and_years_follow_on() {
        let l = Looks::new(2026, 4101);
        assert_eq!(l, Looks::new(2026, 4101));
        assert_ne!(l, Looks::new(2026, 4102));
        assert_ne!(l, Looks::new(2027, 4101));
        let g = genes(l, Ethnicity::Black);
        let ctx = |_| SeasonCtx { level: Some(Level::FBS), grooming: Grooming::Free, position: Some(Position::WR) };
        let a = appearance(2026, l, &g, 2004, 2026, ctx, |_| (72.0, 200.0), false);
        let b = appearance(2026, l, &g, 2004, 2026, ctx, |_| (72.0, 200.0), false);
        assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
        assert_eq!(a.age, 22);
        // Marks and ink only build up.
        let later = appearance(2026, l, &g, 2004, 2032, ctx, |_| (72.0, 200.0), false);
        assert!(later.arm_ink >= a.arm_ink);
        assert!(later.scars.len() >= a.scars.len());
    }

    #[test]
    fn academies_shave_and_cut_short() {
        for id in 0..300 {
            let l = Looks::new(1, id);
            let g = genes(l, Ethnicity::ALL[id as usize % 7]);
            let ctx =
                |_| SeasonCtx { level: Some(Level::FBS), grooming: Grooming::Academy, position: Some(Position::LB) };
            let a = appearance(1, l, &g, 2005, 2026, ctx, |_| (73.0, 230.0), false);
            assert_eq!(a.beard, Beard::Clean);
            assert!(matches!(
                a.hair,
                HairStyle::Shaved
                    | HairStyle::Buzz
                    | HairStyle::Crop
                    | HairStyle::CropFade
                    | HairStyle::Waves
                    | HairStyle::SidePart
            ));
            assert_eq!(a.earrings, Earrings::None);
        }
    }

    #[test]
    fn rates_land_near_real_shares() {
        let n = 4000;
        let (mut ink, mut beard, mut locs, mut mixed) = (0, 0, 0, 0);
        for id in 0..n {
            let l = Looks::new(7, id);
            let g = genes(l, Ethnicity::Black);
            mixed += g.mixed.is_some() as u32;
            let ctx = |y: i32| SeasonCtx {
                level: Some(if y < 2018 {
                    Level::HS
                } else if y < 2022 {
                    Level::FBS
                } else {
                    Level::NFL
                }),
                grooming: Grooming::Free,
                position: Some(Position::CB),
            };
            let a = appearance(7, l, &g, 2000, 2026, ctx, |_| (72.0, 195.0), false);
            ink += (a.arm_ink > 0) as u32;
            beard += (a.beard != Beard::Clean) as u32;
            locs += (a.hair == HairStyle::Locs) as u32;
        }
        let share = |k: u32| k as f64 / n as f64;
        assert!((0.45..0.8).contains(&share(ink)), "arm ink {}", share(ink));
        assert!((0.5..0.9).contains(&share(beard)), "beards {}", share(beard));
        assert!((0.1..0.35).contains(&share(locs)), "locs {}", share(locs));
        assert!((0.08..0.16).contains(&share(mixed)), "mixed {}", share(mixed));
    }
}
