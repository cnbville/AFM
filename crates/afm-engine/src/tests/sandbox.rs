//! Sandbox mode (Settings, a test switch): it marks the career for good,
//! lifts every switch the first time, keeps your own picks after that,
//! works on a save from before it, takes over the no-fire override, and
//! each lifted switch takes away only your own limit.

use crate::world::sandbox::{Sandbox, Switch};
use crate::world::scouting::film::time;
use crate::world::seat::{Job, You};
use crate::world::{World, WorldOptions};

fn you() -> You {
    let teaching = vec![12.0; crate::world::coach::GROUP_COUNT];
    serde_json::from_value(serde_json::json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
    }))
    .unwrap()
}

/// A tiny world with you as a college head coach.
fn college() -> (World, usize) {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let p = (0..w.programs.len()).find(|&p| w.programs[p].plays(w.year)).unwrap();
    w.start_career(&you(), Job::College(p), 7);
    (w, p)
}

#[test]
fn the_first_turn_on_marks_the_career_and_lifts_every_switch() {
    let (mut w, _) = college();
    assert!(Switch::ALL.iter().all(|&s| !w.sandbox(s)), "off at the start");
    w.set_sandbox(true);
    let today = w.today;
    let b = &w.career.as_ref().unwrap().sandbox;
    assert!(b.on && b.since == Some(today));
    assert!(Switch::ALL.iter().all(|&s| w.sandbox(s)), "every switch lifted the first time");
    // One switch back: only it comes back.
    w.set_sandbox_switch(Switch::FilmHours, false);
    assert!(!w.sandbox(Switch::FilmHours) && w.sandbox(Switch::NilBudget));
    // Off: nothing is lifted, the mark stays.
    w.set_sandbox(false);
    assert!(Switch::ALL.iter().all(|&s| !w.sandbox(s)));
    assert_eq!(w.career.as_ref().unwrap().sandbox.since, Some(today));
    // On again: your own picks stand.
    w.set_sandbox(true);
    assert!(!w.sandbox(Switch::FilmHours) && w.sandbox(Switch::NilBudget));
    // Every key reads back.
    for s in Switch::ALL {
        assert_eq!(Switch::from_key(s.key()), Some(s));
        assert!(!s.label().is_empty() && !s.what().is_empty() && !s.group().is_empty());
    }
}

#[test]
fn a_save_from_before_sandbox_mode_loads_with_it_off() {
    let (w, _) = college();
    let mut v = serde_json::to_value(w.career.as_ref().unwrap()).unwrap();
    v.as_object_mut().unwrap().remove("sandbox");
    let k: crate::world::seat::Career = serde_json::from_value(v).unwrap();
    assert_eq!(k.sandbox, Sandbox::default());
}

#[test]
fn the_no_fire_override_is_a_sandbox_switch_and_an_old_no_fire_save_loads_into_it() {
    let (mut w, _) = college();
    w.set_no_fire(true);
    let k = w.career.as_ref().unwrap();
    assert!(k.no_fire && k.sandbox.on && k.sandbox.lifted == vec!["no_fire".to_string()]);
    assert!(w.sandbox(Switch::NoFire) && !w.sandbox(Switch::FilmHours), "only no-fire lifted");
    // Sandbox off takes no-fire with it.
    w.set_sandbox(false);
    assert!(!w.career.as_ref().unwrap().no_fire);

    // A save from before: no-fire on, no sandbox field.
    let (mut old, _) = college();
    let since = crate::calendar::Date::ymd(2036, 3, 1);
    {
        let k = old.career.as_mut().unwrap();
        k.no_fire = true;
        k.no_fire_since = Some(since);
        k.sandbox = Sandbox::default();
    }
    crate::world::sandbox::after_load(&mut old);
    let b = &old.career.as_ref().unwrap().sandbox;
    assert!(b.on && b.since == Some(since) && b.lifted == vec!["no_fire".to_string()]);
    assert!(old.sandbox(Switch::NoFire));
}

#[test]
fn film_hours_never_run_out_and_the_eye_stays_fresh() {
    let (mut w, _) = college();
    // Without it, a week's film runs out.
    let mut spent = 0;
    while time::spend(&mut w, 3.0, None).is_ok() {
        spent += 1;
        assert!(spent < 200, "a real week ends");
    }
    w.set_sandbox(true);
    for _ in 0..100 {
        time::spend(&mut w, 3.0, None).expect("no cap on film hours");
    }
    assert!(time::left(&w) >= 12.0 && time::left(&w).is_finite());
    // The day's 300 hours would tire any eye: not a fresh one.
    assert_eq!(time::tiredness(&w), 0.0);
    w.set_sandbox_switch(Switch::FreshEye, false);
    assert!(time::tiredness(&w) > 0.5);
    // Sent home to rest: the fresh eye ignores it.
    w.scouting.film.rest_until = Some(w.today.add(3));
    assert!(time::rested(&w));
    w.set_sandbox_switch(Switch::FreshEye, true);
    assert!(!time::rested(&w));
    // A long week leaves wear where it is with the fresh eye, and adds to
    // it without.
    w.scouting.film.rest_until = None;
    w.scouting.film.work = time::Work::Junkie;
    w.scouting.film.wear = 0.5;
    let monday = w.today.add(7);
    time::week(&mut w, monday);
    assert_eq!(w.scouting.film.wear, 0.5, "no wear from a long week");
    w.set_sandbox_switch(Switch::FreshEye, false);
    w.scouting.film.work = time::Work::Junkie;
    time::week(&mut w, monday.add(7));
    assert!(w.scouting.film.wear > 0.5, "a real long week wears");
}

#[test]
fn your_week_never_fills_and_charges_nothing() {
    let (mut w, _) = college();
    use crate::world::folk::talks;
    assert!(!talks::spend(&mut w, 100.0), "a real week holds 10 to 16 hours");
    w.set_sandbox(true);
    let before = w.folk.mine.hours;
    assert!(talks::spend(&mut w, 100.0));
    talks::charge(&mut w, 12.0);
    assert_eq!(w.folk.mine.hours, before, "nothing is charged, so the week is true when it goes off");
    assert_eq!(talks::hours_left(&w), talks::week_budget(&w));
}

#[test]
fn the_nil_budget_lifts_for_your_program_only() {
    let (mut w, p) = college();
    use crate::world::front_office::college::budget;
    let other = (0..w.programs.len()).find(|&q| q != p).unwrap();
    let b = budget(&w, p);
    assert!(!b.unlimited && !b.fits(b.left + 1.0e9));
    w.set_sandbox(true);
    let b = budget(&w, p);
    assert!(b.unlimited && b.fits(1.0e12) && b.fits_coll(1.0e12), "yours: any number, in both pots");
    assert!(b.left.is_finite(), "what is left stays the true figure");
    assert!(!budget(&w, other).unlimited, "another program keeps its budget");
    // NIL Go: only your program's deals clear in full.
    assert_eq!(crate::world::recruiting::nil_go_free(&w), Some(p));
    w.set_sandbox_switch(Switch::NilGo, false);
    assert_eq!(crate::world::recruiting::nil_go_free(&w), None);
}
