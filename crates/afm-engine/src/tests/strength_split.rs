//! Upper and lower body strength (physical study item 4b): the split, its
//! tie at creation, the squat reading the legs and older saves loading.

use crate::attributes::{ATTRIBUTE_COUNT, Attr, Attributes, OLD_ATTRIBUTE_COUNT};
use crate::player::{ProspectOptions, generate_prospect};
use crate::positions::Position;
use crate::rng::Rng;

fn athlete(id: u32) -> crate::player::Player {
    let mut rng = Rng::new(id ^ 0x51);
    generate_prospect(&mut rng, ProspectOptions { id: Some(id), ..Default::default() })
}

#[test]
fn an_older_save_loads_with_the_legs_as_strong_as_the_upper_body() {
    let mut a = Attributes::filled(50.0);
    a.set(Attr::Strength, 71.0);
    let old: Vec<f32> = (0..OLD_ATTRIBUTE_COUNT).map(|i| a.at(i) as f32).collect();
    let bytes = rmp_serde::to_vec(&old).unwrap();
    let back: Attributes = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back.get(Attr::LowerBody), 71.0);
    assert_eq!(back.get(Attr::Strength), 71.0);
    // 8-byte numbers load too, and a new save round-trips.
    let old64: Vec<f64> = old.iter().map(|&x| x as f64).collect();
    let back64: Attributes = rmp_serde::from_slice(&rmp_serde::to_vec(&old64).unwrap()).unwrap();
    assert_eq!(back64, back);
    let mut now = back;
    now.set(Attr::LowerBody, 64.0);
    let again: Attributes = rmp_serde::from_slice(&rmp_serde::to_vec(&now).unwrap()).unwrap();
    assert_eq!(again, now);
    let json: Attributes = serde_json::from_str(&serde_json::to_string(&old).unwrap()).unwrap();
    assert_eq!(json.get(Attr::LowerBody), 71.0);
    assert!(rmp_serde::from_slice::<Attributes>(&rmp_serde::to_vec(&vec![1.0f32; 30]).unwrap()).is_err());
    assert_eq!(ATTRIBUTE_COUNT, OLD_ATTRIBUTE_COUNT + 1);
}

#[test]
fn the_halves_move_together_mostly_but_men_differ() {
    let (mut u, mut l) = (Vec::new(), Vec::new());
    for id in 0..6000u32 {
        let p = athlete(4_400_000 + id);
        if p.position == Position::DT || p.position == Position::OT || p.position == Position::IOL {
            u.push(p.ceilings.get(Attr::Strength));
            l.push(p.ceilings.get(Attr::LowerBody));
        }
    }
    let n = u.len() as f64;
    assert!(n > 300.0, "{n} linemen");
    let (mu, ml) = (u.iter().sum::<f64>() / n, l.iter().sum::<f64>() / n);
    let cov = u.iter().zip(&l).map(|(a, b)| (a - mu) * (b - ml)).sum::<f64>() / n;
    let sd = |v: &[f64], m: f64| (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n).sqrt();
    let r = cov / (sd(&u, mu) * sd(&l, ml));
    assert!((mu - ml).abs() < 1.5, "upper {mu:.1} lower {ml:.1}");
    assert!(r > 0.6 && r < 0.97, "tie {r:.2}");
    let differ = u.iter().zip(&l).filter(|(a, b)| (*a - *b).abs() >= 5.0).count() as f64 / n;
    assert!(differ > 0.05, "only {differ:.2} of men differ by 5+");
}

#[test]
fn the_squat_and_clean_read_the_legs_and_the_bench_the_upper_body() {
    let mut p = athlete(4_500_001);
    p.age = 20;
    let a = crate::training::seat::test_row(&p, 7);
    p.attributes.add(Attr::LowerBody, 12.0);
    let b = crate::training::seat::test_row(&p, 7);
    assert_eq!(a.bench, b.bench);
    assert_eq!(a.reps, b.reps);
    assert!(b.squat > a.squat + 10, "squat {} > {}", a.squat, b.squat);
    assert!(b.clean > a.clean, "clean {} > {}", a.clean, b.clean);
}
