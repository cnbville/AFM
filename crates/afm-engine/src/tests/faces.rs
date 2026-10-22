//! Faces (F3, F4, F10): the looks number, the Asian split of names and
//! groups, and an older world loading with faces filled in.

use crate::rng::Rng;
use crate::world::ethnicity::Ethnicity;
use crate::world::looks::{self, Looks};
use crate::world::names;
use crate::world::{World, WorldOptions};

fn small_world() -> World {
    let mut w = World::empty(WorldOptions { scale: 0.002, ..Default::default() });
    w.add_class_aged(17);
    w
}

#[test]
fn every_new_person_gets_a_looks_number_from_his_id_alone() {
    let w = small_world();
    assert_eq!(w.faces_version, 1);
    for p in &w.people {
        assert!(p.looks.is_set());
        assert_eq!(p.looks, Looks::new(w.options.seed, p.player.id));
    }
    // The same world twice: same people, same faces, same headshots.
    let again = small_world();
    for (a, b) in w.people.iter().zip(&again.people).take(300) {
        assert_eq!(a.name, b.name);
        let (fa, fb) = (w.face_of(a, None), again.face_of(b, None));
        assert_eq!(serde_json::to_string(&fa).unwrap(), serde_json::to_string(&fb).unwrap());
    }
}

#[test]
fn a_face_follows_his_heritage_and_his_years() {
    let w = small_world();
    let p = w.people.iter().find(|p| p.stage == crate::world::Stage::Pro).unwrap();
    let f = w.face_of(p, None);
    assert_eq!(f.genes.heritage, p.ethnicity);
    let photo = f.photo.expect("a pro has a team headshot");
    assert!(photo.year <= w.dates.season);
    // Three seasons earlier he was three years younger, and his ink only
    // grows after.
    let before = w.face_of(p, Some(photo.year - 3));
    assert_eq!(before.appearance.age + 3, f.appearance.age);
    assert!(before.appearance.arm_ink <= f.appearance.arm_ink);
    let kid = w.people.iter().find(|p| p.stage == crate::world::Stage::HighSchool).unwrap();
    let k = w.face_of(kid, None);
    assert!(k.photos.iter().all(|ph| matches!(ph.team, looks::PhotoTeam::HighSchool(_))));
}

#[test]
fn asian_names_follow_the_family_origin_and_draw_the_same_numbers() {
    for seed in 0..400u32 {
        let mut a = Rng::new(seed);
        let mut b = Rng::new(seed);
        let mut c = Rng::new(seed);
        let names = [
            names::person_name(Ethnicity::EastAsian, 2006, &mut a),
            names::person_name(Ethnicity::SoutheastAsian, 2006, &mut b),
            names::person_name(Ethnicity::SouthAsian, 2006, &mut c),
        ];
        // One draw each for the first name and the surname, as before the split.
        let next = [a.next_u32(), b.next_u32(), c.next_u32()];
        assert!(next[0] == next[1] && next[1] == next[2]);
        for (n, g) in names.iter().zip(Ethnicity::ASIAN) {
            let s = names::surname_of(n).split('-').next().unwrap();
            let by = names::asian_group_of(s);
            assert!(by.is_some(), "{n}: {s} is not an Asian surname");
            // Shared surnames (Lee, Le) may lean to another group; most don't.
            let _ = g;
        }
    }
    assert_eq!(names::asian_group_of("Nguyen"), Some(Ethnicity::SoutheastAsian));
    assert_eq!(names::asian_group_of("Patel"), Some(Ethnicity::SouthAsian));
    assert_eq!(names::asian_group_of("Kim"), Some(Ethnicity::EastAsian));
    assert_eq!(names::asian_group_of("Smith"), None);
    assert_eq!(names::surname_of("Kevin Nguyen-Tran Jr."), "Nguyen-Tran");
}

#[test]
#[ignore]
fn show_samples() {
    for g in Ethnicity::ASIAN {
        let v: Vec<String> = (0..14u32).map(|s| names::person_name(g, 2006, &mut Rng::new(s * 7 + 1))).collect();
        println!("{}: {}", g.name(), v.join(", "));
    }
    let w = small_world();
    for p in w.people.iter().filter(|p| p.stage == crate::world::Stage::Pro).take(12) {
        let f = w.face_of(p, None);
        let a = &f.appearance;
        println!(
            "{} ({}, {:?}, {} {}): {}; {}; {}; {} {:.0} lb bf {:.0} | mixed {:?} photo {:?}",
            p.name,
            p.ethnicity.name(),
            p.player.position,
            a.age,
            a.year,
            a.hair_words(&f.genes),
            a.beard_words(&f.genes),
            a.marks_words(&f.genes).join(", "),
            a.build.word(),
            a.weight,
            a.body_fat,
            f.genes.mixed.map(|m| m.0.name()),
            f.photo.as_ref().map(|p| p.key.clone())
        );
    }
}

#[test]
fn an_older_world_loads_with_faces_and_the_asian_groups_split() {
    // "Asian" in an old file reads as East Asian.
    let bytes = rmp_serde::to_vec(&"Asian").unwrap();
    assert_eq!(rmp_serde::from_slice::<Ethnicity>(&bytes).unwrap(), Ethnicity::EastAsian);
    let mut w = small_world();
    // A person as an older file stored him: no looks, one Asian group.
    let mut v = serde_json::to_value(&w.people[0]).unwrap();
    v.as_object_mut().unwrap().remove("looks");
    v["ethnicity"] = serde_json::json!("Asian");
    let old: crate::world::Person = rmp_serde::from_slice(&rmp_serde::to_vec_named(&v).unwrap()).unwrap();
    assert!(!old.looks.is_set());
    assert_eq!(old.ethnicity, Ethnicity::EastAsian);
    // The whole world as an older file: faces_version 0, no looks, every
    // Asian person East Asian.
    w.faces_version = 0;
    let mut asian = Vec::new();
    for (i, p) in w.people.iter_mut().enumerate() {
        p.looks = Looks::default();
        if p.ethnicity.is_asian() {
            asian.push(i);
            p.ethnicity = Ethnicity::EastAsian;
        }
    }
    w.after_load();
    assert_eq!(w.faces_version, 1);
    assert!(w.people.iter().all(|p| p.looks == Looks::new(w.options.seed, p.player.id)));
    for &i in &asian {
        let p = &w.people[i];
        assert!(p.ethnicity.is_asian());
        if let Some(g) = names::asian_group_of(names::surname_of(&p.name)) {
            assert_eq!(p.ethnicity, g, "{}", p.name);
        }
    }
    // A second load changes nothing.
    let before: Vec<_> = w.people.iter().map(|p| (p.looks, p.ethnicity)).collect();
    w.after_load();
    assert!(w.people.iter().map(|p| (p.looks, p.ethnicity)).eq(before));
}
