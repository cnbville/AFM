use super::mean;
use crate::player::{Level, ProspectOptions, generate_prospect};
use crate::rng::Rng;
use crate::tendencies::{
    Instruction, MID, Orders, Style, TEND_COUNT, Tend, Tendencies, YearCtx, develop_year, level_sd, obey_chance, scout,
};

fn prospects(n: usize) -> Vec<crate::player::Player> {
    let mut rng = Rng::new("tendencies");
    (0..n).map(|_| generate_prospect(&mut rng, ProspectOptions { age: Some(14), ..Default::default() })).collect()
}

fn sd(xs: &[f64]) -> f64 {
    let m = mean(xs);
    (xs.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / xs.len() as f64).sqrt()
}

#[test]
fn fourteen_year_olds_spread_wide_around_the_middle() {
    let ps = prospects(3000);
    for t in Tend::ALL {
        let v: Vec<f64> = ps.iter().map(|p| p.tend.get(t)).collect();
        assert!((mean(&v) - MID).abs() < 0.6, "{t:?} mean {}", mean(&v));
        assert!(sd(&v) > 3.5 && sd(&v) < 5.5, "{t:?} sd {}", sd(&v));
        assert!(v.iter().all(|x| (1.0..=20.0).contains(x)));
    }
}

#[test]
fn levels_narrow_the_spread() {
    let ps = prospects(2000);
    let spread = |level: Level| {
        let v: Vec<f64> = ps
            .iter()
            .map(|p| {
                Tendencies::at_level(p.id, level, &p.ceilings, p.body.weight, p.body.height, &p.personality)
                    .get(Tend::Bounce)
            })
            .collect();
        sd(&v)
    };
    assert!(spread(Level::NFL) < spread(Level::HS));
    assert!((spread(Level::NFL) - level_sd(Level::NFL)).abs() < 0.8);
}

#[test]
fn a_good_coach_takes_a_bad_habit_down() {
    let ps = prospects(400);
    let year = |teaching: f64| {
        let ctx = YearCtx {
            level: Level::FBS,
            teaching,
            style: Some(Style::default()),
            scheme: None,
            played: 1.0,
            major_injury: false,
            speed_lost: 0.0,
            centre: [0.0; TEND_COUNT],
        };
        mean(
            &ps.iter()
                .map(|p| {
                    let mut t = p.tend;
                    t.set(Tend::Reach, 18.0);
                    develop_year(&mut t, &p.ceilings, &p.personality, 19, &ctx, p.id);
                    18.0 - t.get(Tend::Reach)
                })
                .collect::<Vec<_>>(),
        )
    };
    let strong = year(17.0);
    let weak = year(5.0);
    assert!(strong > weak + 1.0, "strong {strong} weak {weak}");
    assert!(strong > 1.0 && strong < 4.5, "strong {strong}");
}

#[test]
fn habits_settle_with_age() {
    let ps = prospects(400);
    let moved = |age: u32| {
        let ctx = YearCtx {
            level: Level::NFL,
            teaching: 16.0,
            style: None,
            scheme: None,
            played: 1.0,
            major_injury: false,
            speed_lost: 0.0,
            centre: [0.0; TEND_COUNT],
        };
        mean(
            &ps.iter()
                .map(|p| {
                    let mut t = p.tend;
                    t.set(Tend::Gunslinger, 18.0);
                    develop_year(&mut t, &p.ceilings, &p.personality, age, &ctx, p.id);
                    (18.0 - t.get(Tend::Gunslinger)).abs()
                })
                .collect::<Vec<_>>(),
        )
    };
    assert!(moved(20) > 2.0 * moved(29), "20: {} 29: {}", moved(20), moved(29));
}

#[test]
fn a_low_discipline_player_follows_about_two_times_in_three() {
    let typical = obey_chance(10.0, 10.0, 0.5, 10.0);
    let loose = obey_chance(3.0, 10.0, 0.5, 10.0);
    assert!(typical > 0.8 && typical < 0.95, "typical {typical}");
    assert!(loose > 0.6 && loose < 0.75, "loose {loose}");
    assert!(obey_chance(10.0, 10.0, 0.5, 18.0) > typical);
}

#[test]
fn orders_set_their_tendency() {
    let o = Orders(vec![Instruction::Slide, Instruction::KneelDeeperThan(4.0)]);
    assert!(o.sets(Tend::Contact).is_some_and(|v| v < MID));
    assert_eq!(o.kneel_depth(), Some(4.0));
    assert!(o.sets(Tend::JumpRoutes).is_none());
}

#[test]
fn film_narrows_a_scouted_read() {
    let mut rng = Rng::new(7);
    let width = |games: f64, rng: &mut Rng| {
        mean(
            &(0..400)
                .map(|_| {
                    let r = scout(15.0, games, 12.0, rng);
                    (r.high - r.low) as f64
                })
                .collect::<Vec<_>>(),
        )
    };
    let few = width(4.0, &mut rng);
    let many = width(40.0, &mut rng);
    assert!(many < few, "few {few} many {many}");
}
