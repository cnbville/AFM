//! Test switches for the dynasty knockout worlds (thread "Stop college
//! dynasties for good", 2026-10-28). `AFM_KO` names one suspect to turn
//! up or off: `coach4`, `frozen`, `flatdev`, `brand85`, `local50` or `rel85`;
//! `+` joins several (`brand85+local50`). Unset, nothing changes.
//! Trial tooling only, never merged.

use std::sync::OnceLock;

fn which() -> &'static str {
    static KO: OnceLock<String> = OnceLock::new();
    KO.get_or_init(|| {
        let ko = std::env::var("AFM_KO").unwrap_or_default();
        if !ko.is_empty() {
            eprintln!("dynasty test switch on: AFM_KO={ko}");
        }
        ko
    })
}

fn has(part: &str) -> bool {
    which().split('+').any(|x| x == part)
}

/// The coach's and the position coach's recruiting count 4 times as much in a recruit's bond.
pub(crate) fn coach_bond() -> f64 {
    if has("coach4") { 4.0 } else { 1.0 }
}

/// Wins stop feeding a program's recent prestige, tradition, boosters and pipelines.
pub(crate) fn frozen_form() -> bool {
    has("frozen")
}

/// The highest brand a recruit reads: every program from 0.85 of the old
/// leader's size up reads the same name, and its 5-star pull with it.
pub(crate) fn brand_cap() -> f64 {
    if has("brand85") || has("rel85") { 0.85 } else { 1.0 }
}

/// Every FBS player grows under the same coaching number.
pub(crate) fn flat_development() -> bool {
    has("flatdev")
}

/// How much a 4- or 5-star weighs being close to home, his own stay-home
/// lean and the team he grew up with: half under `local50`, so blue chips
/// leave home about as often as real ones (real 41% sign in their home state).
pub(crate) fn local_scale(stars: u8) -> f64 {
    if stars >= 4 && has("local50") { 0.5 } else { 1.0 }
}

/// The prestige the other recruiting reads see (how far a staff's eyes
/// reach, the talent-over-need margin): capped at 0.85 under `rel85`, which
/// also caps the brand.
pub(crate) fn rel_read(rel: f64) -> f64 {
    if has("rel85") { rel.min(0.85) } else { rel }
}
