//! Personal talk (`docs/design/personal-talk.md`, `persona/home.rs`): what
//! a person can tell you about his life is read from what the world stores
//! and drawn on it, the same every time and never against it; openness and
//! depth follow his makeup and his trust in you.

use super::legacy::seat;
use crate::world::background::FamilyStructure;
use crate::world::folk::life;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::home::{self, Layer};

#[test]
fn his_life_is_read_from_his_home_and_drawn_the_same_every_time() {
    let mut w = seat("college");
    w.add_class_aged(17);
    let kids: Vec<usize> = (0..w.people.len()).take(400).collect();
    let mut foster = 0;
    for &i in &kids {
        let him = Them::Player(w.people[i].player.id);
        let bits = home::bits(&w, him);
        assert_eq!(bits, home::bits(&w, him), "keyed: the same every time");
        assert!(bits.iter().any(|b| b.key == 1), "who raised him: {bits:?}");
        let raised = &bits.iter().find(|b| b.key == 1).unwrap().words;
        match w.people[i].family.structure {
            FamilyStructure::Foster => {
                foster += 1;
                assert!(!bits.iter().any(|b| b.words.contains("His dad")), "no dad drawn for a foster kid: {bits:?}");
                assert!(bits.iter().any(|b| b.layer == Layer::Troubles), "{bits:?}");
            }
            FamilyStructure::Grandparents => assert!(raised.contains("grandmother"), "{raised}"),
            FamilyStructure::TwoParents => assert!(raised.contains("mom") && raised.contains("dad"), "{raised}"),
            FamilyStructure::SingleParent => assert!(raised.contains("on h"), "{raised}"),
        }
        // A brother at a program is the stored one.
        if let Some(j) = w.people[i].family.older_brother_at {
            let name = w.program_name(j).to_string();
            assert!(bits.iter().any(|b| b.key == 5 && b.words.contains(&name)), "{bits:?}");
        }
        let open = home::openness(&w, him);
        assert!((0.6..=1.4).contains(&open), "{open}");
        // The parent across talks about raising him.
        let kin = home::bits(&w, Them::Kin(w.people[i].player.id, 0));
        assert!(kin.iter().any(|b| b.key == 91), "{kin:?}");
    }
    eprintln!("personal talk: {} players read, {foster} in foster care", kids.len());
}

#[test]
fn a_coach_talks_about_his_own_family() {
    let w = seat("college");
    let me = w.career.as_ref().map(|c| c.coach);
    let c = (0..w.coaches.len()).find(|&c| Some(c) != me && life::family(&w, c).0).expect("a married coach");
    let bits = home::bits(&w, Them::Coach(c));
    assert!(bits.iter().any(|b| b.key == 60 && b.words.starts_with("He's married to")), "{bits:?}");
    let kids = life::family(&w, c).1.len();
    assert_eq!(bits.iter().any(|b| b.key == 61), kids > 0, "{bits:?}");
}

#[test]
fn a_man_who_doesnt_trust_you_keeps_to_the_surface() {
    let mut w = seat("college");
    w.add_class_aged(17);
    let i = 0;
    let him = Them::Player(w.people[i].player.id);
    w.people[i].trust = 0.1;
    // No bond: his trust in you, times his openness.
    let d = home::depth(&w, him);
    assert!(d <= Layer::Feelings, "{d:?}");
    let said = home::answer(&w, him, "Tell me about yourself", home::Asked::default());
    assert!(!said.released.is_empty() && said.released.len() <= 3, "{said:?}");
    assert!(said.new.iter().all(|(k, _)| *k < 40), "never his troubles unasked: {said:?}");
}
