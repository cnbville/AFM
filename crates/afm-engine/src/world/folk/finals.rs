//! The picks a typed line asks about first (live talks, the core): a pick
//! that ends or starts someone's job or roster place, hands down a ruling
//! above a warning, or takes a deal outside the rules can't be undone, so a
//! typed line that reaches it shows what it heard and lands only when you
//! confirm (`scene::say` refuses it otherwise). A clicked chip is not
//! changed by this.
//!
//! One table by scene kind and code, from the sweep in
//! `afm/live-talks/final-picks.md`, checked again on the core's branch
//! (main 7691db7, 2026-11-13): the same 28 rows. Codes repeat across kinds
//! (70 is a trainer's call, a firing and a signee's release), so the code
//! alone is never enough. A scene that marks its own picks with
//! `PickKind::Final` needs no row here; a chips item that renumbers a
//! scene removes that kind's rows. The rule and the rows are Claude's:
//! proposed, not confirmed.

use super::scene::Kind;

/// (scene kind, pick code) of every pick that can't be undone.
pub const FINAL: [(Kind, u32); 28] = [
    // A December signee asks out: let him go, a visit that can release
    // him, or hold him to his signing.
    (Kind::SigneeRelease, 70),
    (Kind::SigneeRelease, 71),
    (Kind::SigneeRelease, 72),
    // A holdover's job: let him go, or let the room go.
    (Kind::Holdover, 22),
    (Kind::Holdover, 33),
    (Kind::Holdover, 34),
    // Letting a coach go (73, "he stays", is the only way back).
    (Kind::Firing, 70),
    (Kind::Firing, 71),
    (Kind::Firing, 72),
    // Every ruling above a warning (500).
    (Kind::Discipline, 501),
    (Kind::Discipline, 502),
    (Kind::Discipline, 503),
    (Kind::Discipline, 504),
    (Kind::Discipline, 505),
    (Kind::Discipline, 506),
    (Kind::Discipline, 507),
    (Kind::Discipline, 508),
    // An all-star invitation or a search firm's call, taken or turned down.
    (Kind::JobCall, 95),
    (Kind::JobCall, 96),
    (Kind::JobCall, 97),
    (Kind::JobCall, 98),
    // A high school coach's package: hiring him lets your coach go.
    (Kind::ReachOut, 87),
    // Deals outside the rules.
    (Kind::Backchannel, 151),
    (Kind::Backchannel, 152),
    (Kind::Backchannel, 153),
    (Kind::Backchannel, 181),
    (Kind::Backchannel, 191),
    (Kind::Backchannel, 192),
];

/// Whether the table lists this pick.
pub fn is_final(kind: Kind, code: u32) -> bool {
    FINAL.iter().any(|&(k, c)| k == kind && c == code)
}
