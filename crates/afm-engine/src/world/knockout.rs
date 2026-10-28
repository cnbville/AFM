//! Test switches for the dynasty knockout worlds (thread "Stop college
//! dynasties for good", 2026-10-28). `AFM_KO` names one suspect to turn
//! up or off: `coach4`, `frozen` or `flatdev`. Unset, nothing changes.
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

/// The coach's and the position coach's recruiting count 4 times as much in a recruit's bond.
pub(crate) fn coach_bond() -> f64 {
    if which() == "coach4" { 4.0 } else { 1.0 }
}

/// Wins stop feeding a program's recent prestige, tradition, boosters and pipelines.
pub(crate) fn frozen_form() -> bool {
    which() == "frozen"
}

/// Every FBS player grows under the same coaching number.
pub(crate) fn flat_development() -> bool {
    which() == "flatdev"
}
