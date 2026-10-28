//! Update 3, lane 1: bodies and the speed core.

use super::{mean, prospects};
use crate::attributes::Attr;
use crate::body::{body_now, fill_share, forty_weight, play_speed};
use crate::development::{SeasonContext, develop_season, unrelated};
use crate::player::{Level, Player, PlayingTime, ProspectOptions, generate_prospect};
use crate::positions::Position;
use crate::rng::Rng;

fn at(pos: Position, n: usize, seed: &str) -> Vec<Player> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| generate_prospect(&mut rng, ProspectOptions { position: Some(pos), ..Default::default() })).collect()
}

fn shares(players: &[Player], age: f64) -> Vec<f64> {
    let mut v: Vec<f64> = players.iter().map(|p| fill_share(p, age)).collect();
    v.sort_by(f64::total_cmp);
    v
}

#[test]
fn fills_out_by_position_group_with_outliers() {
    let receivers = shares(&at(Position::WR, 4000, "fill-wr"), 18.0);
    let tackles = shares(&at(Position::OT, 4000, "fill-ot"), 18.0);
    let med = |v: &[f64]| v[v.len() / 2];
    // A12: receivers about 94% as freshmen, tackles about 90%.
    assert!((med(&receivers) - 0.94).abs() < 0.01, "receivers {}", med(&receivers));
    assert!((med(&tackles) - 0.90).abs() < 0.01, "tackles {}", med(&tackles));
    // A13: most within 3 points (5 for linemen), a few far out both ways.
    let within = |v: &[f64], m: f64, d: f64| v.iter().filter(|&&x| (x - m).abs() <= d).count() as f64 / v.len() as f64;
    assert!(within(&receivers, 0.94, 0.03) > 0.8);
    assert!(within(&tackles, 0.90, 0.05) > 0.8);
    assert!(receivers.iter().filter(|&&x| x >= 0.98).count() > 20, "a Metcalf arrives near full");
    assert!(tackles.iter().filter(|&&x| x <= 0.79).count() > 10, "a late-growing tackle arrives light");
    // Full by 21, linemen by 22.
    assert!(shares(&at(Position::WR, 500, "fill-wr21"), 21.0).iter().all(|&x| x == 1.0));
    assert!(shares(&at(Position::OT, 500, "fill-ot22"), 22.0).iter().all(|&x| x == 1.0));
}

#[test]
fn being_lighter_stops_adding_speed_below_175() {
    assert_eq!(forty_weight(155.0), forty_weight(175.0));
    assert!(forty_weight(200.0) > forty_weight(175.0));
    assert!((forty_weight(220.0)).abs() < 1e-12);
}

#[test]
fn plays_fast_by_burst_and_speed_per_position() {
    let mix = |p| play_speed(p, 80.0, 90.0).unwrap();
    assert!((mix(Position::DT) - 87.0).abs() < 1e-9);
    assert!((mix(Position::EDGE) - 86.0).abs() < 1e-9);
    assert!((mix(Position::RB) - 85.0).abs() < 1e-9);
    assert!((mix(Position::WR) - 84.0).abs() < 1e-9);
    assert!(play_speed(Position::K, 80.0, 90.0).is_none());
}

#[test]
fn only_clearly_unrelated_skills_stop_growing() {
    assert!(unrelated(Position::OT, Attr::ManCoverage, false));
    assert!(!unrelated(Position::QB, Attr::Catching, false), "a quarterback can become a receiver");
    assert!(!unrelated(Position::OT, Attr::Elusiveness, false), "elusiveness is rated for his size");
    assert!(!unrelated(Position::OT, Attr::Speed, false));
    assert!(!unrelated(Position::IOL, Attr::LongSnapping, true), "high school snappers play another spot");
    assert!(unrelated(Position::IOL, Attr::LongSnapping, false));
    // A tackle's man coverage stays where it was at 14.
    let mut rng = Rng::new("unrelated");
    for mut p in at(Position::OT, 50, "unrelated-ot") {
        let before = p.attributes.get(Attr::ManCoverage);
        while p.age < 22 {
            let level = if p.age <= 17 { Level::HS } else { Level::FBS };
            develop_season(&mut p, &SeasonContext::new(level, 12.0, PlayingTime::Starter), &mut rng);
        }
        assert!(p.attributes.get(Attr::ManCoverage) <= before + 1e-6);
    }
}

#[test]
fn running_backs_slow_down_at_most_a_year_and_a_half_early() {
    use crate::development::declines;
    // Speed declines from 27: a back's from 25.5, not 24.5.
    assert!(!declines(Position::RB, 0.0, Attr::Speed, 24.0));
    assert!(declines(Position::RB, 0.0, Attr::Speed, 25.0));
    assert!(!declines(Position::WR, 0.0, Attr::Speed, 25.0));
}

#[test]
fn every_season_played_wears_a_body() {
    let mut rng = Rng::new("wear");
    let mut back = at(Position::RB, 1, "wear-rb").remove(0);
    back.age = 24;
    develop_season(&mut back, &SeasonContext::new(Level::NFL, 12.0, PlayingTime::Starter), &mut rng);
    assert!(
        (back.wear - 1.0).abs() < 1e-9 || back.wear > 2.0,
        "a starting back's season adds 1.0 (plus 1.5 for a major injury)"
    );
    let mut kicker = at(Position::K, 1, "wear-k").remove(0);
    kicker.age = 24;
    let before = kicker.wear;
    develop_season(&mut kicker, &SeasonContext::new(Level::NFL, 12.0, PlayingTime::Starter), &mut rng);
    assert!(kicker.wear - before < 2.0);
    let mut backup = at(Position::LB, 1, "wear-lb").remove(0);
    backup.age = 20;
    develop_season(&mut backup, &SeasonContext::new(Level::FBS, 12.0, PlayingTime::Backup), &mut rng);
    assert!(backup.wear == 0.0 || backup.wear >= 1.5, "a backup's season adds none");
}

#[test]
fn the_weight_room_only_adds_weight() {
    let mut rng = Rng::new("weight-room");
    let mut gained = Vec::new();
    for mut p in at(Position::WR, 200, "weight-room-wr") {
        p.age = 18;
        let before = p.body.weight;
        develop_season(&mut p, &SeasonContext::new(Level::FBS, 12.0, PlayingTime::Starter), &mut rng);
        // Only the bad-weight event can add more than the room does; nothing takes weight off.
        assert!(p.body.weight >= before - 1e-9);
        gained.push(p.body.weight - before);
    }
    let mut light = at(Position::OT, 200, "weight-room-ot");
    for p in &mut light {
        p.body.weight -= 30.0;
        p.age = 18;
    }
    let before: Vec<f64> = light.iter().map(|p| p.body.weight).collect();
    for p in &mut light {
        develop_season(p, &SeasonContext::new(Level::FBS, 12.0, PlayingTime::Starter), &mut rng);
    }
    let after: Vec<f64> = light.iter().map(|p| p.body.weight).collect();
    assert!(mean(&after) - mean(&before) > 5.0, "light tackles bulk up: {}", mean(&after) - mean(&before));
    assert!(mean(&gained) < mean(&after) - mean(&before));
}

#[test]
fn a_senior_has_about_95_percent_of_his_top_speed() {
    let mut rng = Rng::new("senior-speed");
    let mut share = Vec::new();
    let mut late = Vec::new();
    for mut p in prospects(600, "senior-speed", 1.0) {
        while p.age < 18 {
            develop_season(&mut p, &SeasonContext::new(Level::HS, 10.0, PlayingTime::Starter), &mut rng);
        }
        let s = p.attributes.get(Attr::Speed) / p.ceilings.get(Attr::Speed);
        share.push(s);
        if p.bloom > 1.5 {
            late.push(s);
        }
    }
    assert!((mean(&share) - 0.95).abs() < 0.02, "seniors {}", mean(&share));
    assert!(mean(&late) < mean(&share), "late bloomers have less");
    // His body now carries his group's share.
    let p = &at(Position::WR, 1, "senior-body")[0];
    assert!(body_now(p).weight < p.body.weight);
}
