//! The final build's foundations (lane 1): birth dates (A1), held-back and
//! young-for-class boys (A2), Aura and fame (F15, F16, X6), linked names in
//! lines (L2) and old world files filling the new fields.

use crate::calendar::born::{self, age_on, class_of_birth, schoolboy};
use crate::calendar::events::{Link, add_ref, segments};
use crate::calendar::{Date, Event, Kind, Level, Loudness};
use crate::world::{Stage, World, WorldOptions};

fn tiny_world() -> World {
    World::empty(WorldOptions { scale: 0.004, ..Default::default() })
}

#[test]
fn a_class_runs_from_september_to_august_birthdays() {
    assert_eq!(class_of_birth(Date::ymd(2011, 9, 1)), 2030);
    assert_eq!(class_of_birth(Date::ymd(2012, 8, 31)), 2030);
    assert_eq!(class_of_birth(Date::ymd(2012, 9, 1)), 2031);
    assert_eq!(age_on(Date::ymd(2008, 3, 14), Date::ymd(2026, 3, 13)), 17);
    assert_eq!(age_on(Date::ymd(2008, 3, 14), Date::ymd(2026, 3, 14)), 18);
    // A leap-day birthday a year on falls back to the 28th.
    assert_eq!(born::shift_years(Date::ymd(2012, 2, 29), -1), Date::ymd(2011, 2, 28));
    // Engine age 14 in the 2026 season: born September 2011 to August 2012.
    for key in 0..2000u64 {
        let d = born::birth_date(key, 14, 2026);
        assert!(d >= Date::ymd(2011, 9, 1) && d <= Date::ymd(2012, 8, 31), "{key}: {d:?}");
        assert_eq!(age_on(d, Date::ymd(2026, 8, 31)), 14);
    }
}

#[test]
fn about_one_boy_in_ten_is_a_year_old_for_his_class() {
    let n = 200_000u64;
    let (mut held, mut young, mut summer, mut summer_held, mut fall, mut fall_held) = (0, 0, 0, 0, 0, 0);
    for key in 0..n {
        let (d, years) = schoolboy(key, 14, 2026);
        // Whatever his years, he sits in the class of 2030 (on time) or
        // one either side when he was held back or is young.
        assert_eq!(class_of_birth(d) + years, 2030);
        match years {
            1 => held += 1,
            -1 => young += 1,
            _ => {}
        }
        // The month he would have been born in, on time.
        let m = born::shift_years(d, years).month();
        if (6..=8).contains(&m) {
            summer += 1;
            summer_held += (years == 1) as u32;
        } else if (9..=11).contains(&m) {
            fall += 1;
            fall_held += (years == 1) as u32;
        }
    }
    let held = held as f64 / n as f64;
    let young = young as f64 / n as f64;
    assert!((0.085..0.115).contains(&held), "held back {held:.3}");
    assert!((0.004..0.013).contains(&young), "young for class {young:.4}");
    // Summer birthdays are held back most, fall birthdays least.
    let (s, f) = (summer_held as f64 / summer as f64, fall_held as f64 / fall as f64);
    assert!(s > 2.5 * f, "summer {s:.3}, fall {f:.3}");
}

#[test]
fn a_new_class_has_birthdays_and_some_older_boys() {
    let mut w = tiny_world();
    let before = w.people.len();
    let year = w.year;
    w.add_class_aged(14);
    let class = &w.people[before..];
    assert!(class.len() > 500);
    let mut older = 0;
    for p in class {
        assert_eq!(p.class_of, year + 4, "every freshman is in the class of {}", year + 4);
        assert_ne!(p.born, born::UNKNOWN);
        // His engine age is his age as the season starts.
        assert_eq!(p.age_on(Date::ymd(year, 8, 31)), p.player.age, "{}", p.name);
        let off = p.years_off_class();
        assert_eq!(p.player.age as i32, 14 + off);
        assert_eq!(p.school_age(), 14, "his grade reads as a freshman's");
        older += (off == 1) as usize;
    }
    let share = older as f64 / class.len() as f64;
    assert!((0.06..0.14).contains(&share), "held back {share:.3}");
    // Some of the held-back boys turn 19 before they graduate in May.
    let seniors_19 = class.iter().filter(|p| age_on(p.born, Date::ymd(year + 4, 5, 15)) >= 19).count();
    assert!(seniors_19 > 0);
    // A held-back boy is a year older in the body too: more mature at entry.
    let mean_of = |off: i32| {
        let v: Vec<f64> = class.iter().filter(|p| p.years_off_class() == off).map(|p| p.player.overall()).collect();
        super::mean(&v)
    };
    assert!(mean_of(1) > mean_of(0), "older {:.1}, on time {:.1}", mean_of(1), mean_of(0));
}

#[test]
fn coaches_and_veterans_have_birthdays() {
    let w = tiny_world();
    for p in w.people.iter() {
        assert_ne!(p.born, born::UNKNOWN, "{}", p.name);
        let age = p.age_on(w.today) as i32;
        assert!((age - p.player.age as i32).abs() <= 1, "{}: {age} against {}", p.name, p.player.age);
    }
    for c in w.coaches.iter() {
        assert_ne!(c.born, born::UNKNOWN, "{}", c.name);
        let age = c.age_on(w.today) as i32;
        assert!((age - c.age as i32).abs() <= 1, "{}: {age} against {}", c.name, c.age);
    }
}

#[test]
fn aura_is_its_own_trait_with_a_small_link_to_ego_and_leadership() {
    let mut w = tiny_world();
    w.add_class_aged(14);
    let t: Vec<(f64, f64, f64)> = w
        .people
        .iter()
        .map(|p| {
            let t = p.player.personality.traits;
            (t.aura as f64, t.ego as f64, t.leadership as f64)
        })
        .collect();
    assert!(t.iter().all(|x| (1.0..=20.0).contains(&x.0)));
    let corr = |f: fn(&(f64, f64, f64)) -> f64| {
        let (a, b): (Vec<f64>, Vec<f64>) = t.iter().map(|x| (x.0, f(x))).unzip();
        let (ma, mb) = (super::mean(&a), super::mean(&b));
        let cov: f64 = a.iter().zip(&b).map(|(x, y)| (x - ma) * (y - mb)).sum();
        let va: f64 = a.iter().map(|x| (x - ma).powi(2)).sum();
        let vb: f64 = b.iter().map(|y| (y - mb).powi(2)).sum();
        cov / (va * vb).sqrt()
    };
    let mean_aura = super::mean(&t.iter().map(|x| x.0).collect::<Vec<_>>());
    assert!((9.3..10.7).contains(&mean_aura), "mean Aura {mean_aura:.2}");
    let (ego, lead) = (corr(|x| x.1), corr(|x| x.2));
    assert!((0.08..0.4).contains(&ego), "ego {ego:.2}");
    assert!((0.04..0.35).contains(&lead), "leadership {lead:.2}");
}

#[test]
fn aura_multiplies_fame_and_never_replaces_it() {
    use crate::world::fame::aura_appeal;
    let mut w = tiny_world();
    w.add_class_aged(14);
    let mut p = w.people[0].clone();
    let appeal = |p: &mut crate::world::Person, aura: u8, fame: f64| {
        p.player.personality.traits.aura = aura;
        aura_appeal(p, fame)
    };
    // A backup with the most Aura against a star with little.
    let backup = appeal(&mut p, 20, 0.12);
    let star = appeal(&mut p, 5, 0.8);
    assert!(star > 2.0 * backup, "star {star:.3}, backup {backup:.3}");
    // More Aura always helps a little; no fame, no appeal.
    assert!(appeal(&mut p, 15, 0.5) > appeal(&mut p, 10, 0.5));
    assert_eq!(appeal(&mut p, 20, 0.0), 0.0);
}

#[test]
fn fame_puts_famous_pros_ahead_and_the_year_book_sets_followers_and_money() {
    let mut w = tiny_world();
    w.year = 2024;
    w.add_class_aged(17);
    crate::world::fame::year_book(&mut w);
    let pros: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::Pro).collect();
    let kids: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).collect();
    assert!(!pros.is_empty() && !kids.is_empty());
    let avg = |v: &[usize], f: &dyn Fn(usize) -> f64| v.iter().map(|&i| f(i)).sum::<f64>() / v.len() as f64;
    let followers = |i: usize| w.people[i].followers as f64;
    assert!(avg(&pros, &followers) > 5.0 * avg(&kids, &followers));
    // Pros get endorsement money; high schoolers none.
    assert!(pros.iter().any(|&i| w.people[i].brand > 10_000.0));
    assert!(kids.iter().all(|&i| w.people[i].brand == 0.0));
    // A quarterback is better known than a guard of the same standing.
    let qb = pros.iter().filter(|&&i| w.people[i].player.position == crate::positions::Position::QB);
    let ol = pros.iter().filter(|&&i| w.people[i].player.position == crate::positions::Position::IOL);
    let fame_of = |it: &mut dyn Iterator<Item = &usize>| {
        let v: Vec<f64> = it.map(|&i| crate::world::fame::fame(&w, i)).collect();
        super::mean(&v)
    };
    assert!(fame_of(&mut qb.into_iter()) > fame_of(&mut ol.into_iter()));
}

#[test]
fn a_line_links_whole_names_and_keeps_its_text() {
    let text = "Allen Smith of Allentown East commits to Alabama; Allen is a 4-star.";
    let e = Event::new(Date::ymd(2026, 6, 1), Level::Recruiting, Kind::Commitment, Loudness::Small, text)
        .person(77, "Allen Smith")
        .school_link(3, "Alabama")
        .hs_link(9, "Allentown East")
        // Not in the text: skipped.
        .club_link(1, "Dallas Cowboys");
    assert_eq!(e.refs.len(), 3);
    let parts = segments(&e.text, &e.refs);
    let joined: String = parts.iter().map(|p| p.0).collect();
    assert_eq!(joined, text);
    let linked: Vec<(&str, Link)> = parts.iter().filter_map(|p| p.1.map(|l| (p.0, l))).collect();
    assert_eq!(
        linked,
        vec![("Allen Smith", Link::Person(77)), ("Allentown East", Link::School(9)), ("Alabama", Link::Program(3))]
    );
    // "Allen" alone is not inside "Allentown", and a taken span is skipped.
    let mut refs = e.refs.clone();
    add_ref(&mut refs, text, Link::Person(5), "Allen");
    let last = refs.iter().find(|r| r.to == Link::Person(5)).unwrap();
    assert_eq!(&text[last.at as usize..(last.at + last.len) as usize], "Allen");
    assert!(last.at as usize > text.find("Alabama").unwrap());
}

#[test]
fn an_old_world_file_fills_birthdays_and_aura_the_same_way_every_time() {
    let mut w = tiny_world();
    w.add_class_aged(15);
    // As a file from before the final build loads: no birthdays, no Aura.
    let reset = |w: &mut World| {
        for p in w.people.iter_mut() {
            p.born = born::UNKNOWN;
            p.player.personality.traits.aura = 0;
            p.followers = 0;
        }
        for c in w.coaches.iter_mut() {
            c.born = born::UNKNOWN;
        }
    };
    let snap = |w: &World| -> Vec<(Date, u8)> {
        w.people.iter().map(|p| (p.born, p.player.personality.traits.aura)).collect()
    };
    reset(&mut w);
    w.after_load();
    let first = snap(&w);
    reset(&mut w);
    w.after_load();
    assert_eq!(first, snap(&w));
    let a = &w;
    for x in a.people.iter() {
        assert_ne!(x.born, born::UNKNOWN);
        assert!(x.player.personality.traits.aura >= 1);
        // No held-back boys appear in an old file: every grade stays.
        assert_eq!(x.years_off_class(), 0, "{}", x.name);
        assert_eq!(x.school_age(), x.player.age);
    }
    assert!(a.coaches.iter().all(|c| c.born != born::UNKNOWN));
    // Pros get followers straight away.
    assert!(a.people.iter().any(|p| p.stage == Stage::Pro && p.followers > 0) || a.year < 2006);
}
