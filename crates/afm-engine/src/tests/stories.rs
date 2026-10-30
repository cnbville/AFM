//! The stories (lane 8): the bots that grade fourth downs by era, the
//! numbers each era lets a position wear, the line banks and their
//! blocklist of real legends, your answers to the asks from the desk, and
//! old world files loading with empty stories.

use crate::calendar::{Date, Level};
use crate::positions::Position;
use crate::world::desk::{Answer, Head, Mail};
use crate::world::seat::{Job, You};
use crate::world::stories::nfl::{self, Choice};
use crate::world::stories::{self as st, TeamKey, lines, numbers};
use crate::world::{World, WorldOptions};

/// A tiny world in the 2025 league year with you as an NFL head coach.
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

/// A letter on your desk with the two answers of an ask, answered.
fn answered_letter(date: Date, yes: u8, no: u8, target: u32, pick: u8) -> Mail {
    Mail {
        id: 9_000_000 + target as u64,
        date,
        minute: 600,
        head: Head::Memo,
        org: "Club".into(),
        org_line: String::new(),
        from: "Staff".into(),
        role: "Staff".into(),
        to: "You".into(),
        subject: "An ask".into(),
        body: Vec::new(),
        table: Vec::new(),
        attach: Vec::new(),
        sign: String::new(),
        keys: 0,
        about: None,
        answers: vec![
            Answer { label: "Yes".into(), does: String::new(), act: yes, target },
            Answer { label: "No".into(), does: String::new(), act: no, target },
        ],
        answered: Some(pick),
        answer_by: None,
        read: true,
        saved: false,
        loud: 2,
        ids: Vec::new(),
        org_team: None,
        why: Vec::new(),
    }
}

#[test]
fn the_fourth_down_bots_arrive_in_their_real_years() {
    assert_eq!(nfl::bot(Level::Nfl, 2006, 0), None);
    assert!(nfl::bot(Level::Nfl, 2007, 0).unwrap().contains("Advanced NFL Stats"));
    assert!(nfl::bot(Level::Nfl, 2014, 0).unwrap().contains("4th Down Bot"));
    assert!(nfl::bot(Level::Nfl, 2018, 0).unwrap().contains("EdjSports"));
    assert!(nfl::bot(Level::Nfl, 2025, 1).is_some());
    assert_eq!(nfl::bot(Level::College, 2017, 0), None);
    assert!(nfl::bot(Level::College, 2018, 0).is_some());
    for y in [1996, 2010, 2030] {
        assert_eq!(nfl::bot(Level::HighSchool, y, 0), None);
    }
}

#[test]
fn the_bot_goes_on_short_yardage_and_punts_deep_in_its_own_end() {
    // Fourth and one at the opponent's 40, tied in the second quarter.
    let (c, _) = nfl::read(40.0, 1.0, 0, 2, 2400.0, Level::Nfl, 2024);
    assert_eq!(c, Choice::Go);
    // Fourth and twelve at your own 20.
    let (c, _) = nfl::read(80.0, 12.0, 0, 2, 2400.0, Level::Nfl, 2024);
    assert_eq!(c, Choice::Punt);
    // Fourth and twelve at the opponent's 20: the kick.
    let (c, _) = nfl::read(20.0, 12.0, 0, 2, 2400.0, Level::Nfl, 2024);
    assert_eq!(c, Choice::FieldGoal);
    // Down ten with two minutes left: go whatever the spot.
    let (c, _) = nfl::read(70.0, 6.0, -10, 4, 120.0, Level::Nfl, 2024);
    assert_eq!(c, Choice::Go);
}

#[test]
fn each_era_lets_a_position_wear_its_own_numbers() {
    let nfl = |p, y| numbers::allowed(p, Level::Nfl, y);
    // Receivers wore 80-89 until 2004 opened 10-19.
    assert!(nfl(Position::WR, 2003).iter().all(|&n| (80..=89).contains(&n)));
    assert!(nfl(Position::WR, 2010).iter().any(|n| (10..=19).contains(n)));
    // Linebackers got 40-49 in 2015.
    assert!(!nfl(Position::LB, 2014).iter().any(|n| (40..=49).contains(n)));
    assert!(nfl(Position::LB, 2016).iter().any(|n| (40..=49).contains(n)));
    // 2021 opened single digits to backs, receivers and defenders; 2024 allowed 0.
    assert!(!nfl(Position::RB, 2020).iter().any(|&n| n < 10));
    assert!(nfl(Position::RB, 2021).iter().any(|&n| (1..=9).contains(&n)));
    assert!(!nfl(Position::CB, 2023).contains(&0));
    assert!(nfl(Position::CB, 2024).contains(&0));
    // Interior linemen stay ineligible at every level: 50-79.
    for level in [Level::HighSchool, Level::College, Level::Nfl] {
        assert!(numbers::allowed(Position::IOL, level, 2026).iter().all(|n| (50..=79).contains(n)));
    }
    assert!(!numbers::allowed(Position::QB, Level::College, 2019).contains(&0));
    assert!(numbers::allowed(Position::QB, Level::College, 2020).contains(&0));
}

#[test]
fn a_club_never_doubles_a_number_and_a_retired_one_stays_retired() {
    let (mut w, t) = nfl_world();
    let key = TeamKey::Nfl(t as u16);
    let nums = numbers::numbers(&w, key);
    assert!(!nums.is_empty());
    let mut seen = std::collections::HashSet::new();
    for n in nums.values() {
        assert!(seen.insert(*n), "number {n} worn twice");
    }
    // Retire one in use: nobody wears it after.
    let (&id, &n) = nums.iter().next().unwrap();
    let i = w.people.iter().position(|p| p.player.id == id).unwrap();
    let pos = w.people[i].player.position;
    w.stories.traditions.honors.push(crate::world::stories::traditions::TeamHonor {
        team: key,
        kind: crate::world::stories::traditions::HonorKind::RetiredNumber,
        player: 1,
        name: "A Legend".into(),
        pos,
        number: Some(n),
        year: 2020,
        years: "2005-2016".into(),
        ceremony_done: true,
    });
    let after = numbers::numbers(&w, key);
    assert!(after.values().all(|&m| m != n));
}

#[test]
fn the_line_banks_have_variety_and_never_reuse_a_legends_name() {
    for key in [
        "grade.agree",
        "grade.punt_wrong",
        "grade.go_wrong",
        "story.hired",
        "story.left",
        "mag.top",
        "doc.announce",
        "oneday.offer",
        "farewell.retire",
        "celebrate.flag",
        "story.rivalry_plain",
        "story.big_win",
    ] {
        assert!(lines::key_count(key) >= 3, "{key} has {} lines", lines::key_count(key));
    }
    let bank = &*lines::BANK;
    assert!(bank.blocked("Beast Mode"));
    assert!(bank.blocked("The Bus"));
    assert!(bank.blocked("the bus"));
    assert!(!bank.blocked("The Hammer of Akron"));
    // A name ending in s takes the apostrophe alone; a common noun keeps 's.
    assert_eq!(lines::tidy("illinois fans packed Illinois's stadium."), "Illinois fans packed Illinois' stadium.");
    assert_eq!(lines::tidy("The boss's office."), "The boss's office.");
}

#[test]
fn you_cannot_name_a_player_after_a_legend_or_someone_elses_player() {
    let (mut w, t) = nfl_world();
    let mine = w.clubs[t].roster[0];
    let id = w.people[mine].player.id;
    assert!(st::nick::give(&mut w, id, "Megatron").is_err());
    assert!(st::nick::give(&mut w, id, "").is_err());
    let other = (0..w.clubs.len()).find(|&c| c != t && !w.clubs[c].roster.is_empty()).unwrap();
    let theirs = w.people[w.clubs[other].roster[0]].player.id;
    assert!(st::nick::give(&mut w, theirs, "Big Train").is_err());
    // Your own: it sticks or it doesn't, and either way it is on file.
    assert!(st::nick::give(&mut w, id, "Big Train").is_ok());
    assert!(st::nick::all_of(&w.stories, st::nick::Who::Player(id)).iter().any(|n| n.name == "Big Train"));
}

#[test]
fn your_answer_on_the_desk_settles_the_ask() {
    let (mut w, t) = nfl_world();
    let today = w.today;
    // A one-day contract ask, turned down from the letter.
    w.stories.careers.offer = Some((424_242, t as u16, today));
    let m = answered_letter(today, st::ACT_ONE_DAY_YES, st::ACT_ONE_DAY_NO, 424_242, 1);
    w.career.as_mut().unwrap().desk.mails.push(m);
    st::desk_answers(&mut w);
    assert_eq!(w.stories.careers.offer, None);
    assert!(w.stories.careers.one_day.iter().any(|x| x.0 == 424_242 && x.2 == u16::MAX));
    // A documentary ask, said yes to; a letter from another year does nothing.
    let year = w.year;
    w.stories.media.doc_offer = Some((year as i16, "Hard Knocks".into()));
    let old = answered_letter(today, st::ACT_DOC_YES, st::ACT_DOC_NO, (year - 1) as u32, 1);
    w.career.as_mut().unwrap().desk.mails.push(old);
    st::desk_answers(&mut w);
    assert_eq!(w.stories.media.doc_answer, None);
    let m = answered_letter(today, st::ACT_DOC_YES, st::ACT_DOC_NO, year as u32, 0);
    w.career.as_mut().unwrap().desk.mails.push(m);
    st::desk_answers(&mut w);
    assert_eq!(w.stories.media.doc_answer, Some(true));
}

#[test]
fn an_old_world_file_loads_with_empty_stories() {
    // A save from before the stories: every store missing.
    let empty = rmp_serde::to_vec_named(&serde_json::json!({})).unwrap();
    let s: st::Stories = rmp_serde::from_slice(&empty).unwrap();
    assert!(s.nicknames.is_empty() && s.story.entries.is_empty() && s.careers.offer.is_none());
    // A graded call written before the edge was kept.
    let call = serde_json::json!({
        "quarter": 4, "clock": 300, "togo": 2, "yardline": 45, "lead": -3,
        "chose": "Punt", "bot": "Go", "cost": 2.5, "result": "punted to the 10"
    });
    let c: nfl::Call = rmp_serde::from_slice(&rmp_serde::to_vec_named(&call).unwrap()).unwrap();
    assert_eq!(c.edge, 0.0);
    // And the stories round-trip with the world's own format.
    let (w, _) = nfl_world();
    let back: st::Stories = rmp_serde::from_slice(&rmp_serde::to_vec_named(&w.stories).unwrap()).unwrap();
    assert_eq!(back.story.entries.len(), w.stories.story.entries.len());
}

#[test]
fn a_count_of_one_takes_the_singular() {
    use crate::world::folk::noun;
    assert_eq!(noun(1, "interception"), "interception");
    assert_eq!(noun(0, "interception"), "interceptions");
    assert_eq!(noun(3_usize, "catch"), "catches");
    assert_eq!(noun(2_u32, "carry"), "carries");
    assert_eq!(noun(2, "day"), "days");
    assert_eq!(noun(1_i64, "practice injury"), "practice injury");
    assert_eq!(noun(5, "practice injury"), "practice injuries");
}

#[test]
fn a_bank_line_counting_one_takes_the_singular() {
    let t = "renamed a drink {days} days after, {years} years on, {pts} points, {n} is";
    let one = lines::Fill::new().set("days", "1").set("years", "1").set("pts", "1").set("n", "1");
    assert_eq!(lines::fill_for_test(t, &one).unwrap(), "renamed a drink 1 day after, 1 year on, 1 point, 1 is");
    let many = lines::Fill::new().set("days", "11").set("years", "3").set("pts", "21").set("n", "2");
    assert_eq!(lines::fill_for_test(t, &many).unwrap(), "renamed a drink 11 days after, 3 years on, 21 points, 2 is");
}
