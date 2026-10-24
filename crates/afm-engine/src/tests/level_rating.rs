use crate::level_rating::{
    LevelGroup, LevelScale, Pool, SCALE_SEASONS, ScaleHistory, build_level_scale, level_rating, pooled_level_scale,
    season_anchors,
};
use crate::player::Level;
use crate::rng::Rng;

fn setup() -> (Vec<f64>, LevelScale) {
    let mut rng = Rng::new("level-rating");
    let pool: Vec<f64> = (0..11_000).map(|_| rng.normal(56.0, 9.0)).collect();
    let scale = build_level_scale(&pool, Pool::College, 1.0);
    (pool, scale)
}

#[test]
fn rises_with_overall_and_stays_on_the_1_99_scale() {
    let (_, scale) = setup();
    let mut last = 0;
    for k in 0..=200 {
        let r = level_rating(k as f64 * 0.5, Level::FBS, Some(&scale));
        assert!(r >= last && (1..=99).contains(&r));
        last = r;
    }
}

#[test]
fn puts_about_a_hundred_players_at_90_plus_and_saves_99_for_a_clear_no_1() {
    let (pool, scale) = setup();
    let rate = |x: f64| level_rating(x, Level::FBS, Some(&scale));
    let mut sorted = pool.clone();
    sorted.sort_by(|a, b| b.total_cmp(a));
    let ninety_plus = pool.iter().filter(|&&x| rate(x) >= 90).count();
    assert!(ninety_plus > 80 && ninety_plus < 130, "{ninety_plus}");
    assert!(rate(sorted[1]) < 98);
    assert!(rate(sorted[1] + 1.0) < 99);
    assert_eq!(rate(sorted[1] + 6.0), 99);
}

#[test]
fn a_strong_year_shows_more_90s_against_five_seasons_than_against_itself() {
    let mut rng = Rng::new("level-rating-pooled");
    let mut h = ScaleHistory { group: LevelGroup::College, start: Vec::new(), end: Vec::new() };
    for _ in 0..SCALE_SEASONS + 2 {
        let typical: Vec<f64> = (0..11_000).map(|_| rng.normal(56.0, 9.0)).collect();
        let a = season_anchors(&typical, Pool::College, 1.0);
        h.push(a.clone(), a);
    }
    assert_eq!(h.end.len(), SCALE_SEASONS);
    // A loaded year: everyone a point and a half better.
    let strong: Vec<f64> = (0..11_000).map(|_| rng.normal(57.5, 9.0)).collect();
    let alone = build_level_scale(&strong, Pool::College, 1.0);
    h.push(Vec::new(), season_anchors(&strong, Pool::College, 1.0));
    let pooled = pooled_level_scale(&h.end);
    let count = |s: &LevelScale| strong.iter().filter(|&&x| level_rating(x, Level::FBS, Some(s)) >= 90).count();
    let (n_alone, n_pooled) = (count(&alone), count(&pooled));
    assert!(n_alone > 80 && n_alone < 130, "{n_alone}");
    assert!(n_pooled > n_alone + 15, "{n_pooled} pooled, {n_alone} alone");
    // An empty season is left out of the pool, not read as a zero.
    assert!(pooled_level_scale(&h.start).points.len() >= 4);
}

#[test]
fn leaves_the_pro_game_on_the_absolute_scale() {
    let (_, scale) = setup();
    assert_eq!(level_rating(73.4, Level::NFL, Some(&scale)), 73);
}

#[test]
fn a_draft_board_puts_the_best_punter_in_the_fourth_round_not_the_top_20() {
    use crate::level_rating::PositionAligner;
    use crate::positions::Position;
    // A class of 2,600: every position on its own spread, kickers and
    // punters scored on their leg, higher than the rest.
    let mut rng = Rng::new("draft-scale");
    let mut samples: Vec<(Position, f64)> = Vec::new();
    for pos in Position::ALL {
        let (n, mid) = match pos {
            Position::K | Position::P | Position::LS => (60, 70.0),
            _ => (230, 52.0),
        };
        samples.extend((0..n).map(|_| (pos, rng.normal(mid, 8.0))));
    }
    let slots = 257;
    let rank_of_best = |a: &PositionAligner, pos: Position| {
        let best = samples.iter().filter(|s| s.0 == pos).map(|&(p, x)| a.align(p, x)).fold(f64::MIN, f64::max);
        samples.iter().filter(|&&(p, x)| a.align(p, x) > best).count() + 1
    };
    let plain = PositionAligner::new(&samples);
    assert!(rank_of_best(&plain, Position::P) <= 40, "lined up, the best punter is near the top");
    let draft = PositionAligner::for_draft(&samples, slots);
    let k = rank_of_best(&draft, Position::K);
    let p = rank_of_best(&draft, Position::P);
    let ls = rank_of_best(&draft, Position::LS);
    assert!((100..=135).contains(&k), "best kicker at {k}");
    assert!((115..=145).contains(&p), "best punter at {p}");
    assert!((165..=200).contains(&ls), "best long snapper at {ls}");
    // However high a club reads a kicker, he grades no higher than the
    // class's grade at the 18% mark (about pick 46).
    let high = draft.align(Position::K, 120.0);
    let capped = samples.iter().filter(|&&(p, x)| draft.align(p, x) > high).count() + 1;
    assert!((35..=60).contains(&capped), "a kicker read at 120 grades at {capped}");
    // Everyone else keeps his place.
    assert_eq!(draft.align(Position::QB, 60.0), plain.align(Position::QB, 60.0));
}
