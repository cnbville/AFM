//! Real haggling (talk replies item 15, agreed by Cain 2026-11-19): a man
//! opens at his ask and gives part of the gap each round, less each time,
//! down to his walk-away number, at a pace set by his greed, his patience
//! and how much he wants the deal. Coaches you hire (`office::deals`,
//! `hiring::haggle`), agents on your pro's deal (`front_office::nfl`,
//! `judge_said`) and your own college player's money (`deals::own`) all
//! run on these rules; each desk keeps its own ask and walk-away and the
//! rounds said so far. Every size is proposed, not confirmed.
//!
//! Your better terms move his number by what they're worth to him: the
//! walk-away is read on the terms you put up (years, a title, the
//! play-calling, a guarantee), and his number moves by the same amount.

/// The share of the gap a middling man gives in his first round.
pub const GIVE: f64 = 0.4;
/// Each round after, he gives this share of what he gave the round before.
pub const DECAY: f64 = 0.6;
/// Within this share of his number he takes your offer rather than haggle
/// over the last little bit.
pub const CLOSE: f64 = 0.02;
/// The most rounds that move him; past them he holds his number.
pub const ROUNDS: u32 = 6;

/// His pace (0.15 to 0.7): greed slows him, wanting the deal and patience
/// speed him. `greed` is a z score (-1 to 1); `want` and `patience` are
/// 0 to 1, a half the middle.
pub fn pace(greed: f64, want: f64, patience: f64) -> f64 {
    (GIVE - 0.15 * greed + 0.2 * (want - 0.5) + 0.1 * (patience - 0.5)).clamp(0.15, 0.7)
}

/// What his answer to your number is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    /// He takes your number.
    Take,
    /// He gives some and names his new number.
    Counter(f64),
}

/// One round: his number now (`now`, his ask before any round), his
/// walk-away on your terms, the rounds he has given so far, his pace and
/// your offer. He takes an offer at his number (or within `CLOSE` of it,
/// never under his walk-away); else he moves the round's share of the gap
/// between his number and the higher of your offer and his walk-away.
pub fn step(now: f64, walk: f64, rounds: u32, pace: f64, offer: f64) -> Step {
    let now = now.max(walk);
    if offer >= walk && offer >= now * (1.0 - CLOSE) {
        return Step::Take;
    }
    if rounds >= ROUNDS {
        return Step::Counter(now);
    }
    let target = walk.max(offer.min(now));
    let give = pace * DECAY.powi(rounds as i32);
    Step::Counter((now - give * (now - target)).max(walk))
}

/// His number moved by what your terms are worth to him: his walk-away on
/// the terms he last countered against (`was`) and on yours (`walk`).
pub fn moved(now: f64, was: f64, walk: f64) -> f64 {
    (now + (walk - was)).max(walk)
}

/// A number the way it is said across the table: down to $50,000 from a
/// million up, else to $5,000 (a round that gives less than a step still
/// moves him a step); never under `floor`.
pub fn round_said(x: f64, floor: f64) -> f64 {
    let step = if x >= 1_000_000.0 { 50_000.0 } else { 5_000.0 };
    let r = (x / step + 1e-9).floor() * step;
    if r < floor { (floor / step - 1e-9).ceil() * step } else { r }
}

/// One round as he says it: his next number rounded the way it's said,
/// and a yes when that comes to your offer or under it.
pub fn said(now: f64, walk: f64, rounds: u32, pace: f64, offer: f64) -> Step {
    match step(now, walk, rounds, pace, offer) {
        Step::Counter(n) => {
            let r = round_said(n, walk).max(walk);
            if r <= offer { Step::Take } else { Step::Counter(r) }
        }
        s => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cain's example: he asks 3.1; 2.3 brings 2.9, then 2.6 brings 2.8,
    /// and a number at his comes to a yes.
    #[test]
    fn he_gives_less_each_round_and_never_under_his_walk_away() {
        let p = pace(0.0, 0.5, 0.5);
        let Step::Counter(a) = step(3.1e6, 2.6e6, 0, p, 2.3e6) else { panic!() };
        let a = round_said(a, 2.6e6);
        assert!((2.85e6..=2.95e6).contains(&a), "{a}");
        let Step::Counter(b) = step(a, 2.6e6, 1, p, 2.6e6) else { panic!() };
        let b = round_said(b, 2.6e6);
        assert!(b < a && b >= 2.6e6, "{b}");
        assert!(a - b < 3.1e6 - a, "he gives less the second round");
        assert_eq!(step(b, 2.6e6, 2, p, b), Step::Take);
        // Never under his walk-away, however long you go.
        let mut now = 3.1e6;
        for k in 0..10 {
            if let Step::Counter(n) = step(now, 2.6e6, k, p, 1.0e6) {
                assert!(n >= 2.6e6);
                now = n;
            }
        }
    }

    #[test]
    fn a_round_said_always_moves_him() {
        // 2.8 asked, 2.69 his walk-away: a small give still comes down a step.
        let Step::Counter(a) = said(2.8e6, 2.69e6, 1, 0.4, 2.1e6) else { panic!() };
        assert!((2.69e6..2.8e6).contains(&a), "{a}");
        // A number that rounds to your offer is a yes.
        assert_eq!(said(2.8e6, 2.6e6, 3, 0.4, 2.77e6), Step::Take);
    }

    #[test]
    fn a_greedy_man_gives_slower() {
        assert!(pace(1.0, 0.5, 0.5) < pace(-1.0, 0.5, 0.5));
        assert!(pace(0.0, 0.9, 0.5) > pace(0.0, 0.1, 0.5));
    }

    #[test]
    fn better_terms_move_his_number() {
        assert_eq!(moved(2.8e6, 2.6e6, 2.5e6), 2.7e6);
    }
}
