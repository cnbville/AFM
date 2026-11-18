//! Personal talk (`docs/design/personal-talk.md`, agreed 2026-11-17): a
//! family question gets his people, not "they're good, let's talk
//! football"; he stays on it; asking again what he just told you reads as
//! not listening; what he told you and what you told him are kept; asking
//! about his troubles too soon closes him; a talk where something personal
//! landed counts for the bond.

use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::home::{self, Layer};
use afm_engine::world::folk::persona::talk::store;
use serde_json::json;

use super::super::{ear::EAR, voice::VOICE};
use super::tests::{act, ear, game, say, talk, trust, voice};

/// The voice's card text for the last line said.
fn last_voice(f: &afm_ai::Fake) -> String {
    f.asked().into_iter().rev().find(|r| r.label == VOICE).map(|r| r.user).unwrap_or_default()
}

/// The ear's card text for the last line said.
fn last_ear(f: &afm_ai::Fake) -> String {
    f.asked().into_iter().rev().find(|r| r.label == EAR).map(|r| r.user).unwrap_or_default()
}

/// The part of the voice card the game released this turn: the list under
/// its header, up to the next section.
fn released(card: &str) -> String {
    let Some(start) = card.find("What the game decided you say this turn") else { return String::new() };
    card[start..].lines().skip(1).take_while(|l| l.starts_with("- ")).collect::<Vec<_>>().join("\n")
}

#[test]
fn a_family_question_gets_his_people_and_he_stays_on_it() {
    let (mut g, f, him) = game("personal-family", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "a check-in");
    let w = g.world.as_ref().unwrap();
    let bits = home::bits(w, Them::Player(him));
    assert!(bits.iter().any(|b| b.key == 1 && b.layer == Layer::Surface), "who raised him: {bits:?}");
    // The same man every time: his drawn details are keyed to him.
    assert_eq!(bits, home::bits(w, Them::Player(him)));
    // A plain line first, so his own ask could come up on the next.
    f.push(EAR, Ok(ear(vec![], vec![], false)));
    f.push(VOICE, Ok(voice("Morning, Coach.", "")));
    say(&mut g, id, "Morning.");
    f.push(EAR, Ok(ear(vec![act("question", "", "Home", "")], vec![], false)));
    f.push(VOICE, Ok(voice("She's good, Coach.", "")));
    let v = say(&mut g, id, "How's your mom doing?");
    assert_eq!(v["state"], "done", "{v}");
    let card = last_voice(&f);
    let said = released(&card);
    eprintln!("personal: his card on a family question:\n{card}");
    assert!(card.contains("Your life away from football"), "{card}");
    assert!(!said.contains("Nothing new at home to tell"), "{said}");
    assert!(!said.is_empty(), "{card}");
    // He stays on it: no ask of his own after a personal answer unless it
    // presses on him.
    let w = g.world.as_ref().unwrap();
    let wants = super::parts_of(afm_engine::world::folk::scene::Kind::OneOnOne).wanting.wants(w, Them::Player(him));
    let pressing = wants.iter().any(|x| !x.met && x.weight >= home::PRESSING);
    if !pressing {
        assert!(!card.contains("What you ask the coach yourself"), "{card}");
    }
    // What he told you is kept, and the ear sees it on his life item.
    let told = store::told_keys(w, Them::Player(him));
    assert!(!told.is_empty(), "kept what he told you");
    f.push(EAR, Ok(ear(vec![], vec![], false)));
    f.push(VOICE, Ok(voice("Yes sir.", "")));
    say(&mut g, id, "Good to hear.");
    let ear_card = last_ear(&f);
    assert!(ear_card.contains("what he has told you of it"), "{ear_card}");
}

#[test]
fn asking_again_what_he_just_told_you_reads_as_not_listening() {
    let (mut g, f, him) = game("personal-again", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "a check-in");
    f.push(EAR, Ok(ear(vec![act("question", "", "Home", "")], vec![], false)));
    f.push(VOICE, Ok(voice("Two sisters, Coach.", "")));
    say(&mut g, id, "Do you have any brothers or sisters?");
    let first = released(&last_voice(&f));
    let before = trust(&g, him);
    f.push(EAR, Ok(ear(vec![act("question", "", "Home", "")], vec![], false)));
    f.push(VOICE, Ok(voice("Like I said, Coach.", "")));
    say(&mut g, id, "Do you have any brothers or sisters?");
    let second = released(&last_voice(&f));
    eprintln!("personal: asked once: {first}\nasked again: {second}");
    assert!(second.contains("already told you"), "{second}");
    assert!(trust(&g, him) <= before, "asking again buys nothing: {before} -> {}", trust(&g, him));
}

#[test]
fn you_opening_up_is_kept_and_a_personal_talk_counts_for_the_bond() {
    let (mut g, f, him) = game("personal-opens", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "a check-in");
    let mut opens = act("opens", "", "", "");
    opens["words"] = json!("grew up poor in Georgia");
    f.push(EAR, Ok(ear(vec![opens], vec![], false)));
    f.push(VOICE, Ok(voice("No kidding, Coach. Whereabouts?", "")));
    let v = say(&mut g, id, "I grew up with nothing in Georgia, you know.");
    assert_eq!(v["state"], "done", "{v}");
    let card = last_voice(&f);
    assert!(card.contains("He tells you about his own life: grew up poor in Georgia"), "{card}");
    let w = g.world.as_ref().unwrap();
    let kept = store::shared(w, Them::Player(him));
    assert!(kept.iter().any(|(yours, x)| *yours && x.contains("Georgia")), "{kept:?}");
    assert!(store::open(w, id).is_some_and(|o| o.personal), "the talk is personal");
    // He has it on his card the next line.
    f.push(EAR, Ok(ear(vec![], vec![], false)));
    f.push(VOICE, Ok(voice("Yes sir.", "")));
    say(&mut g, id, "Anyway.");
    assert!(last_voice(&f).contains("What the coach has told you about his own life"), "{}", last_voice(&f));
    // Ending it, the bond hears it once this week.
    f.push(EAR, Ok(ear(vec![act("end", "", "", "")], vec![], false)));
    f.push(VOICE, Ok(voice("See you, Coach.", "")));
    say(&mut g, id, "See you tomorrow.");
    let w = g.world.as_ref().unwrap();
    assert!(w.folk.bonds.personal.contains_key(&him), "the bond counts the talk");
}

#[test]
fn his_troubles_asked_too_soon_close_him() {
    let (mut g, f, him) = game("personal-deep", true);
    let f = f.unwrap();
    {
        let w = g.world.as_mut().unwrap();
        let i = afm_engine::world::folk::person(w, him).unwrap();
        w.people[i].trust = 0.2;
    }
    let id = talk(&mut g, him, "a check-in");
    let w = g.world.as_ref().unwrap();
    assert_eq!(home::depth(w, Them::Player(him)), Layer::Surface, "a man who doesn't trust you");
    let before = trust(&g, him);
    f.push(EAR, Ok(ear(vec![act("question", "", "Life", "")], vec![], false)));
    f.push(VOICE, Ok(voice("I'd rather not, Coach.", "")));
    say(&mut g, id, "Is money tight at home? Tell me what's really going on.");
    let said = released(&last_voice(&f));
    assert!(said.contains("rather keep that to himself"), "{said}");
    assert!(trust(&g, him) < before, "prying costs: {before} -> {}", trust(&g, him));
    let w = g.world.as_ref().unwrap();
    assert!(store::open(w, id).is_some_and(|o| o.closed), "closed for the talk");
    // Then a plain question gets one thing, the surface.
    f.push(EAR, Ok(ear(vec![act("question", "", "Home", "")], vec![], false)));
    f.push(VOICE, Ok(voice("It's fine.", "")));
    say(&mut g, id, "Tell me about yourself.");
    let said = released(&last_voice(&f));
    assert_eq!(said.lines().count(), 1, "{said}");
}

#[test]
fn a_first_meeting_opens_on_a_hello_and_yours_is_never_a_dodge() {
    let (mut g, f, him) = game("personal-meet", true);
    let f = f.unwrap();
    {
        let w = g.world.as_ref().unwrap();
        assert!(home::first_meeting(w, him), "never sat down with you");
    }
    let id = talk(&mut g, him, "money");
    let s = super::tests::scene_of(&g, id);
    let opened = s.lines.iter().find(|l| l.by == 2).map(|l| l.text.clone()).unwrap_or_default();
    let bank = afm_engine::world::folk::voice::bank();
    let hellos: Vec<String> = ["good", "flat", "sour"]
        .iter()
        .flat_map(|m| bank.versions(&format!("talk.open.meet.{m}")))
        .map(|x| x.0.to_string())
        .collect();
    eprintln!("personal: his opening on a first meeting: {opened}");
    assert!(!opened.to_lowercase().contains("paid"), "no business first: {opened}");
    assert!(hellos.iter().any(|h| opened.starts_with(h.split(['[', '{']).next().unwrap_or(h).trim_end())), "{opened}");
    let before = trust(&g, him);
    // Even when the ear calls it a dodge, a hello on the first line isn't.
    let v = json!({ "acts": [act("greet", "", "", "")], "hits": [], "dodged": true, "not_talk": false });
    let reading = afm_ai::fake::body(&v.to_string(), 1200, 120);
    f.push(EAR, Ok(reading));
    f.push(VOICE, Ok(voice("Nice to meet you too, Coach.", "")));
    let v = say(&mut g, id, "First time meeting, nice to meet you.");
    assert_eq!(v["state"], "done", "{v}");
    let said = released(&last_voice(&f));
    assert!(said.contains("say hello back"), "{said}");
    assert!(trust(&g, him) >= before, "a hello costs nothing: {before} -> {}", trust(&g, him));
    let card = last_voice(&f);
    assert!(!card.contains("What you ask the coach yourself"), "he stays on the hello: {card}");
}
