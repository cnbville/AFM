use crate::level_rating::{LevelScale, Pool, build_level_scale, level_rating};
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
fn leaves_the_pro_game_on_the_absolute_scale() {
    let (_, scale) = setup();
    assert_eq!(level_rating(73.4, Level::NFL, Some(&scale)), 73);
}
