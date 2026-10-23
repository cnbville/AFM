//! Social media in a scouting file (X5): followers and brand money come from
//! fame and Aura (`world::fame`, lane 1); a staff reads a prospect's accounts
//! for character (the Social look). This is only the words for a count.

/// Words for a follower count: 950, 12.4K, 1.2M.
pub fn words(f: u32) -> String {
    match f {
        0..=999 => format!("{f}"),
        1_000..=999_999 => format!("{:.1}K", f as f64 / 1000.0).replace(".0K", "K"),
        _ => format!("{:.1}M", f as f64 / 1_000_000.0).replace(".0M", "M"),
    }
}
