//! One store for your ratings (coach ratings lane RE-bug): your coach
//! record is the only place your ratings, reputation and style live. The
//! career keeps who you are and the setup page's numbers as a record, and a
//! file from before the fix loads with nothing read from its old copy.

use crate::tests::legacy::{seat, you};
use crate::world::folk::persona::talk::feel;
use crate::world::folk::talks;
use crate::world::legacy::guys;
use crate::world::seat::{Career, Me, You};

fn mine(w: &crate::world::World) -> usize {
    w.career.as_ref().unwrap().coach
}

/// What you earn reaches what reads you: your read of people in a
/// one-on-one, the live talk's read, your best teaching group.
#[test]
fn what_you_earn_reaches_your_talks() {
    let mut w = seat("college");
    let c = mine(&w);
    let before = talks::sharpness(&w);
    assert_eq!(feel::your_relationships(&w), 12.0);
    w.coaches[c].ability.relationships += 4.0;
    assert!((talks::sharpness(&w) - before - 0.2).abs() < 1e-9, "four points are a fifth of the read");
    assert_eq!(feel::your_relationships(&w), 16.0);
    // And what you lose: a decline is read the same day.
    w.coaches[c].ability.relationships = 6.0;
    assert!((talks::sharpness(&w) - before + 0.3).abs() < 1e-9);
    w.coaches[c].ability.teaching[3] = 17.0;
    assert_eq!(guys::best_group(&w), 3);
    w.coaches[c].ability.teaching[5] = 19.0;
    assert_eq!(guys::best_group(&w), 5);
}

/// The record of day one never moves, whatever your coach record does.
#[test]
fn the_setup_record_stays_as_it_was() {
    let mut w = seat("hs");
    let c = mine(&w);
    let day_one = you();
    w.coaches[c].ability.relationships = 18.0;
    w.coaches[c].ability.recruiting = 4.0;
    w.coaches[c].reputation = 15.0;
    let me = &w.career.as_ref().unwrap().you;
    let s = me.at_setup();
    assert_eq!(s.relationships, day_one.relationships);
    assert_eq!(s.recruiting, day_one.recruiting);
    assert_eq!(s.reputation, day_one.reputation);
    assert_eq!(s.teaching, day_one.teaching);
    assert_eq!(s.yours, day_one.yours);
    assert_eq!(w.your_ability().unwrap().relationships, 18.0);
    assert_eq!(w.your_coach().unwrap().reputation, 15.0);
}

/// A save keeps its shape: what the career writes for you is the setup
/// form, byte for byte, so files from before the fix load and files from
/// after it hold nothing new.
#[test]
fn a_save_keeps_its_shape() {
    let mut form = you();
    form.aura = 11.5;
    form.road = "Former college player".into();
    let old = rmp_serde::to_vec_named(&form).unwrap();
    let me: Me = rmp_serde::from_slice(&old).expect("the old shape loads");
    assert_eq!(me.name, form.name);
    assert_eq!(me.aura(), 11.5);
    assert_eq!(rmp_serde::to_vec_named(&me).unwrap(), old, "written back as it was read");
    let back: You = rmp_serde::from_slice(&rmp_serde::to_vec_named(&me).unwrap()).unwrap();
    assert_eq!(back.relationships, form.relationships);
}

/// A file from before the fix: its copy of your ratings had drifted from
/// your coach record. Loaded, nothing reads the copy, and a file from
/// before Aura gets one, drawn once.
#[test]
fn an_old_file_reads_your_coach_record() {
    let mut w = seat("college");
    let c = mine(&w);
    // Three seasons of growth on the coach record, as the yearly step and
    // the takeover left it in an old file.
    w.coaches[c].ability.relationships = 14.5;
    w.coaches[c].ability.recruiting = 15.0;
    let sharp = talks::sharpness(&w);
    // The file's career, with the stale copy as old code wrote it.
    let mut file = serde_json::to_value(w.career.as_ref().unwrap()).unwrap();
    assert_eq!(file["you"]["relationships"], 12.0, "the file still holds the setup form");
    file["you"]["relationships"] = serde_json::json!(3.0);
    file["you"]["recruiting"] = serde_json::json!(3.0);
    file["you"]["aura"] = serde_json::json!(0.0);
    let career: Career = serde_json::from_value(file).expect("an old career loads");
    assert!(career.you.aura() >= 1.0, "a file from before Aura draws it on load");
    w.career = Some(career);
    assert_eq!(talks::sharpness(&w), sharp);
    assert_eq!(feel::your_relationships(&w), 14.5);
    assert_eq!(w.your_ability().unwrap().recruiting, 15.0);
}
