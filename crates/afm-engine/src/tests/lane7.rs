//! Lane 7: scouting reads and offers. Watching a player's film yourself
//! (bug 82), staking your name (SK40) and the athletes from outside
//! football (SK65).

use crate::world::scouting::{self as sc, Kind, readers};
use crate::world::seat::{Job, You};
use crate::world::{Stage, World, WorldOptions};

fn nfl_world() -> (World, usize) {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let t = (0..w.clubs.len())
        .filter(|&t| crate::world::franchise::active_in(&w, t, w.year))
        .max_by_key(|&t| w.clubs[t].roster.len())
        .unwrap();
    let teaching = vec![12.0; crate::world::coach::GROUP_COUNT];
    let you: You = serde_json::from_value(serde_json::json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
    }))
    .unwrap();
    w.start_career(&you, Job::Nfl(t), 7);
    (w, t)
}

/// Players with film: college players, else pros (a world with no
/// history has no college tape yet).
fn on_film(w: &World) -> Vec<u32> {
    let mut v: Vec<u32> = w
        .people
        .iter()
        .filter(|p| p.stage == Stage::College && sc::own::film_cost(w, p.player.id).is_ok())
        .map(|p| p.player.id)
        .take(40)
        .collect();
    if v.len() < 20 {
        v.extend(w.people.iter().filter(|p| p.stage == Stage::Pro).map(|p| p.player.id).take(40));
    }
    v
}

/// Draft prospects: a world with no history has no college players yet,
/// so a few young men are made college players for the rule.
fn prospects(w: &mut World) -> Vec<u32> {
    let mut v: Vec<u32> = w.people.iter().filter(|p| p.stage == Stage::College).map(|p| p.player.id).take(10).collect();
    for p in w.people.iter_mut().filter(|p| p.stage == Stage::Pro).take(10usize.saturating_sub(v.len())) {
        p.stage = Stage::College;
        p.pro = None;
        v.push(p.player.id);
    }
    v
}

#[test]
fn you_watch_film_today_on_your_own_hours() {
    let (mut w, _) = nfl_world();
    let ids = on_film(&w);
    assert!(ids.len() > 10, "a world with players on film");
    let you = readers::you(&w).unwrap() as u32 + readers::COACH;
    let budget = sc::own::film_budget(&w);
    assert!(budget > 0.0);
    let today = w.today;
    // The look lands today, under your name, as his film.
    let (kind, hours, _) = sc::own::film_cost(&w, ids[0]).unwrap();
    assert!(matches!(kind, Kind::CollegeFilm | Kind::ProFilm));
    let msg = sc::own::watch_film(&mut w, ids[0]).unwrap();
    assert!(msg.contains("hours"), "{msg}");
    let looks = &w.scouting.dept.looks[&ids[0]];
    assert!(looks.iter().any(|l| l.who == you && l.date == today && l.kind == kind));
    // Not the same player twice in a day, and the file says so first.
    assert!(sc::own::film_blocked(&w, ids[0]).is_some_and(|m| m.contains("today")));
    assert!(sc::own::watch_film(&mut w, ids[0]).is_err());
    // The week's hours run out.
    let mut spent = hours;
    for &id in &ids[1..] {
        let h = sc::own::film_cost(&w, id).unwrap().1;
        let blocked = sc::own::film_blocked(&w, id);
        if sc::own::watch_film(&mut w, id).is_err() {
            assert!(spent + h > budget, "refused with {spent} of {budget} spent");
            assert!(blocked.is_some_and(|m| m.contains("spent")), "the file shows why before the click");
            break;
        }
        spent += h;
    }
    assert!(spent <= budget + 1e-3);
    assert!((sc::own::film_left(&w) - (budget - spent)).abs() < 1e-3);
    // A new week brings the hours back.
    for _ in 0..7 {
        w.advance_day();
    }
    assert!((sc::own::film_left(&w) - sc::own::film_budget(&w)).abs() < 1e-3);
}

#[test]
fn a_name_staked_on_too_many_means_nothing() {
    let (mut w, _) = nfl_world();
    let ids = prospects(&mut w);
    for &id in ids.iter().take(sc::stake::OPEN_MAX) {
        sc::stake::stake(&mut w, id, false).unwrap();
    }
    assert!(sc::stake::stake(&mut w, ids[0], true).is_err(), "once on a player");
    assert!(sc::stake::stake(&mut w, ids[sc::stake::OPEN_MAX], true).is_err(), "five at once");
    assert!(sc::stake::on(&w, ids[0]).is_some());
}

#[test]
fn outside_athletes_come_in_their_window_and_sign_to_the_practice_squad() {
    let (mut w, t) = nfl_world();
    while w.today.parts().1 != 6 {
        w.advance_day();
    }
    let list = sc::outside::list(&w);
    assert!(!list.is_empty());
    // Reads are wide: nobody has seen him play football.
    assert!(list.iter().all(|a| a.read.1 - a.read.0 >= 10));
    let key = list[0].key;
    let before = w.clubs[t].roster.len();
    sc::outside::sign(&mut w, key).unwrap();
    assert!(w.clubs[t].roster.len() > before);
    // Signed once; gone from the list.
    assert!(sc::outside::list(&w).iter().all(|a| a.key != key));
    assert!(sc::outside::sign(&mut w, key).is_err());
}

#[test]
fn a_famous_name_pulls_recruits_for_two_seasons_unless_he_wins() {
    let (mut w, _) = nfl_world();
    let season = w.dates.season;
    let c = (0..w.coaches.len()).find(|&c| w.coaches[c].job.is_some()).unwrap();
    let co = &mut w.coaches[c];
    co.recent = vec![-0.5];
    co.fame = 17.0;
    co.job.as_mut().unwrap().since = season;
    assert!((co.name_pull(season) - 3.0).abs() < 1e-9, "a star: 3 points");
    co.fame = 14.0;
    assert!((co.name_pull(season) - 1.5).abs() < 1e-9, "a Pro Bowler: half");
    co.fame = 9.0;
    assert_eq!(co.name_pull(season), 0.0, "a name nobody knows");
    co.fame = 17.0;
    assert!((co.name_pull(season + 2) - 1.5).abs() < 1e-9, "fading in his third season");
    assert_eq!(co.name_pull(season + 3), 0.0, "gone by his fourth");
    co.recent = vec![0.4];
    assert!((co.name_pull(season + 3) - 3.0).abs() < 1e-9, "unless he wins");
}
