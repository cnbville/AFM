//! Talks stay about today (talk facts item 30, rule 9 of
//! `afm/talk-facts/proposal.md`): the team talk and the press name your last
//! game only while it is fresh (`facts::GAME_FRESH_DAYS`). In April the room
//! no longer opens on January's loss, and no reporter asks about it. The
//! team talk's tone reads the same window: praise after a loss rings hollow
//! that week, not months later (Cain's yes, 2026-11-13).

use super::legacy::seat;
use crate::game::stats::GameStats;
use crate::world::World;
use crate::world::data::Division;
use crate::world::folk::scene::{self, Kind, Scene};
use crate::world::folk::{Team, facts, talks};
use crate::world::seat::SeatGame;

/// A name no other school has, so a line that names the game is found.
const OPP: &str = "Zzyzx Tech";

/// A college seat world whose last game was a 17-52 loss `ago` days ago.
fn after(ago: i32) -> (World, Team) {
    let mut w = seat("college");
    let team = Team::seat(&w).expect("your team");
    let (_, p) = team.parts();
    let opp = (0..w.programs.len()).find(|&q| q != p && w.programs[q].division == Division::FBS).unwrap();
    w.programs[opp].real_name = OPP.into();
    w.programs[opp].fictional_name = OPP.into();
    let stats = GameStats { score: [17, 52], ..Default::default() };
    w.seat_games.push(SeatGame {
        date: w.today.add(-ago),
        level: crate::calendar::Level::College,
        teams: [p, opp],
        names: ["Home".into(), "Away".into()],
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
        talks: Default::default(),
        whistle: None,
    });
    (w, team)
}

/// Everything the room says when you call the team together.
fn team_talk(w: &mut World) -> String {
    let id = talks::start_team_talk(w).expect("a team talk");
    let s = scene::open(w, id).expect("the scene opens");
    s.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n")
}

/// A weekly press availability, answered to the end: every line it wrote
/// and the questions it asked (as `Q` codes, four to a scene).
fn press(w: &mut World) -> (String, u64) {
    let mut s = Scene::new(Kind::Press, w.today, "Monday press availability");
    s.c = 1;
    s.stops = false;
    s.team = Team::seat(w);
    let id = scene::add(w, s);
    let mut s = scene::open(w, id).expect("the scene opens");
    let asked = s.a;
    let mut text = String::new();
    for _ in 0..6 {
        text.push_str(&s.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"));
        text.push('\n');
        let Some(code) = s.picks.first().map(|p| p.code) else { break };
        s = scene::answer(w, id, code).expect("an answer on the table");
        if s.done {
            text.push_str(&s.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"));
            break;
        }
    }
    (text, asked)
}

/// The press's `Q::LastGame` (its code is 4) is among the scene's questions.
fn asks_about_the_game(asked: u64) -> bool {
    (0..4).any(|k| (asked >> (8 * k)) & 0xFF == 4)
}

#[test]
fn a_game_is_fresh_for_a_week() {
    for (ago, fresh) in [(0, true), (2, true), (7, true), (8, false), (120, false)] {
        let (w, team) = after(ago);
        assert_eq!(facts::fresh_game(&w, team).is_some(), fresh, "{ago} days after the game");
        // The plain reader still has it; no talk reads it any more.
        assert!(facts::last_game(&w, team).is_some(), "{ago} days: the game is kept");
    }
    let (w, team) = after(2);
    assert_eq!(facts::fresh_game(&w, team), facts::last_game(&w, team));
}

#[test]
fn the_team_talk_names_a_game_from_this_week() {
    for ago in [1, 2, 7] {
        let (mut w, _) = after(ago);
        let text = team_talk(&mut w);
        assert!(text.contains("after the 17-52 loss to"), "{ago} days after: {text}");
        assert!(text.contains(OPP), "{ago} days after: {text}");
    }
}

#[test]
fn the_team_talk_stops_naming_an_old_game() {
    for ago in [8, 30, 120, 200] {
        let (mut w, _) = after(ago);
        let text = team_talk(&mut w);
        assert!(!text.contains("17-52") && !text.contains(OPP) && !text.contains("after the"), "{ago} days: {text}");
        assert!(text.contains("No game to talk about"), "{ago} days: {text}");
    }
}

#[test]
fn the_team_talk_looks_ahead_to_a_game_that_is_near() {
    let (mut w, team) = after(120);
    w.college = Some(crate::world::college::new_season(&w));
    let (date, opp, _) = facts::team_games(&w, team).into_iter().next().expect("a game on the schedule");
    // Ten days before the first game of the season: nothing fresh, a game near.
    w.today = date.add(-10);
    w.seat_games[0].date = w.today.add(-120);
    let text = team_talk(&mut w);
    assert!(text.contains("No game to talk about."), "{text}");
    assert!(text.contains(&format!("{} is next, in 10 days.", opp.link(&w))), "{text}");
    assert!(!text.contains("17-52"), "{text}");
}

/// The room's trust, summed, after a team talk answered with one move.
fn trust_after_talk(w: &mut World, code: u32) -> f32 {
    let team = Team::seat(w).expect("your team");
    let id = talks::start_team_talk(w).expect("a team talk");
    scene::open(w, id).expect("the scene opens");
    scene::answer(w, id, code).expect("the move is on the table");
    team.roster(w).iter().map(|&i| w.people[i].trust).sum()
}

#[test]
fn praise_turns_hollow_only_after_this_weeks_loss() {
    // The same room with no game in the file: praise as it plainly lands.
    let plain = |ago: i32| {
        let (mut w, _) = after(ago);
        w.seat_games.clear();
        trust_after_talk(&mut w, 204)
    };
    for ago in [1, 7] {
        let (mut w, _) = after(ago);
        assert!(trust_after_talk(&mut w, 204) < plain(ago), "{ago} days after a 17-52 loss, praise rings hollow");
    }
    for ago in [8, 30, 120] {
        let (mut w, _) = after(ago);
        assert_eq!(trust_after_talk(&mut w, 204), plain(ago), "{ago} days: an old loss no longer turns praise");
    }
}

#[test]
fn the_press_asks_about_this_weeks_game() {
    for ago in [1, 3, 7] {
        let (mut w, _) = after(ago);
        let (_, asked) = press(&mut w);
        assert!(asks_about_the_game(asked), "{ago} days after: the reporters ask about the game");
    }
}

#[test]
fn the_press_stops_asking_about_an_old_game() {
    for ago in [8, 30, 120, 200] {
        let (mut w, _) = after(ago);
        let (text, asked) = press(&mut w);
        assert!(!asks_about_the_game(asked), "{ago} days after: no question about the game");
        // None of the other questions fills in the old score or the old opponent.
        assert!(!text.contains("52-17") && !text.contains("17-52") && !text.contains(OPP), "{ago} days: {text}");
    }
}
