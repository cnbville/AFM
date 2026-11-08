//! The Jan 1 start (Cain 2026-11-08): a world built to stop on January 1
//! stands on that morning with its history done. A college job taken then
//! packs the welcome meetings into the first two days, the staff calls
//! first; a team still playing has an interim on its sideline while you run
//! the rest; an NFL club's interim keeps its last games in the record book.
//! Builds a small world, so the quick check skips it (full tests run it).

use crate::calendar::Date;
use crate::world::folk::Team;
use crate::world::folk::scene::{self, Kind, Scene};
use crate::world::seat::{Job, Start, You};
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

/// One small Jan 1 world, built once and written to a file; each test
/// reads its own copy.
fn fresh() -> World {
    static FILE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    let file = FILE.get_or_init(|| {
        let w = World::new(WorldOptions {
            seed: 76,
            scale: 0.004,
            history_years: 1,
            engine_from: Some(2100),
            stop_jan1: true,
            ..Default::default()
        });
        let f = std::env::temp_dir().join(format!("afm-jan1-{}.afmworld", std::process::id()));
        crate::save::write_world(&w, &f, "test", "", None).unwrap();
        f
    });
    let mut w = crate::save::read_world(file).unwrap();
    w.after_load();
    w
}

fn scenes_of(w: &World) -> Vec<Scene> {
    let ids = &w.folk.takeover.as_ref().unwrap().scenes;
    ids.iter().map(|id| w.folk.scenes.iter().find(|s| s.id == *id).unwrap().clone()).collect()
}

fn at(s: &Scene) -> (Date, u16) {
    (s.date, s.minute.expect("a rush meeting has its hour"))
}

/// Plays the clock on to the morning of `d` (holidays stop on big days).
fn go_to(w: &mut World, d: Date) {
    for _ in 0..400 {
        if w.today >= d {
            return;
        }
        let before = w.now();
        w.holiday_to(d);
        if w.now() == before {
            w.advance_day();
        }
    }
    assert!(w.today >= d, "the clock never reached {}", d.pretty());
}

#[test]
fn a_jan1_world_stands_on_new_years_morning() {
    let w = fresh();
    assert_eq!(w.today, Date::ymd(2026, 1, 1));
    assert_eq!(w.year, 2025, "the last history season's winter");
    assert_eq!(w.history_left(), 0);
    assert!(w.options.stop_jan1);
    // The bowls and the playoff still to play, the NFL in its last weeks.
    assert!(w.college_fixtures(None).iter().any(|f| f.score.is_none() && !f.cancelled && f.date >= w.today));
    let s = w.nfl_season.as_ref().unwrap();
    assert!(s.started && !s.booked && s.champion.is_none());
    // December's signees can still be let out; the spring's can't.
    assert!(crate::world::recruiting::signees_waiting(&w, w.today));
    assert!(!crate::world::recruiting::signees_waiting(&w, w.dates.start));
}

#[test]
fn a_college_job_on_jan1_fills_two_days_staff_first_with_an_interim_on_the_sideline() {
    let mut w = fresh();
    let d0 = w.today;
    let p = (0..w.programs.len())
        .find(|&p| w.college_fixtures(Some(p)).iter().any(|f| f.score.is_none() && !f.cancelled && f.date >= d0))
        .expect("a team still playing");
    let game = w.college_fixtures(Some(p)).into_iter().find(|f| f.score.is_none() && !f.cancelled).unwrap();
    let had_interim = w.programs[p].sideline.is_some();
    let c = w.start_career_as(&you(), Start::Head(Job::College(p)), 5);
    let tk = w.folk.takeover.clone().unwrap();
    assert!(tk.rush, "a winter job is the rush");

    // The interim has the bowl: his game, the last staff's chart.
    let int = w.seat_interim().expect("an interim coaches the games left");
    assert_ne!(int, c);
    assert_eq!(w.programs[p].sideline, Some(int));
    assert_eq!(w.programs[p].head_coach, Some(c));
    assert!(w.seat_kickoff(game.date).is_none());
    assert!(w.seat_order().is_empty());
    assert!(w.seat_setup_today().is_none());
    assert_eq!(w.stint_record(c), (0, 0), "the interim's games aren't yours");
    // A title his team already won stays with the man you replaced.
    let won = w.college.as_ref().unwrap().results[p].best_title();
    let kept = &w.college.as_ref().unwrap().title_credited;
    if !had_interim {
        assert_eq!(kept.contains(&(p, won)), won != crate::world::coach::TitleKind::None);
    }

    // Both days busy, at their own hours, the staff before the roster.
    let v = scenes_of(&w);
    let day = |k: i32| v.iter().filter(|s| s.date == d0.add(k)).count();
    assert!(day(0) >= 5 && day(1) >= 7, "both days busy: {} and {}", day(0), day(1));
    assert!(v.iter().all(|s| s.date >= d0 && s.date <= d0.add(3)), "two days, the last boosters a little later");
    let first = v.iter().find(|s| s.kind == Kind::HireReaction).unwrap();
    assert_eq!(at(first), (d0, 7 * 60));
    let press = v.iter().find(|s| s.kind == Kind::PressIntro).unwrap();
    let staff: Vec<&Scene> = v.iter().filter(|s| s.kind == Kind::Holdover).collect();
    assert!(!staff.is_empty());
    for h in &staff {
        assert!(at(h) < at(press), "{} before the press conference", h.title);
    }
    let last_staff = staff.iter().map(|s| at(s)).max().unwrap();
    for s in v.iter().filter(|s| matches!(s.kind, Kind::FlightRisks | Kind::Recruiting | Kind::Leaders)) {
        assert!(at(s) > last_staff, "{} after the staff calls", s.title);
        assert_eq!(s.date, d0.add(1), "{} on day two", s.title);
    }
    let times: Vec<(Date, u16)> = v.iter().map(at).collect();
    assert!(times.windows(2).all(|x| x[0] <= x[1]), "the takeover's meetings are in order: {times:?}");
    assert_eq!(crate::world::folk::talks::week_budget(&w), 40.0, "the arrival week's people time");

    // December signees whose coach is gone may ask out; one let go.
    let asks: Vec<Scene> = w.folk.scenes.iter().filter(|s| s.kind == Kind::SigneeRelease).cloned().collect();
    assert_eq!(asks.len(), tk.asked_out.len());
    if let Some(s) = asks.first() {
        let i = w.people.iter().position(|x| x.player.id == s.a as u32).unwrap();
        assert_eq!(w.people[i].cal.signed.map(|x| x.program), Some(p), "he is still your signee");
        assert!(s.date > d0, "the calls come after the staff day");
        go_to(&mut w, s.date);
        let s = scene::answer(&mut w, s.id, 70).unwrap();
        assert!(s.done || s.picks.is_empty(), "{:?}", s.picks);
        assert!(w.people[i].cal.signed.is_none(), "let out of his signing");
    }
}

#[test]
fn an_nfl_club_taken_in_season_keeps_its_last_games_on_the_interims_record() {
    let mut w = fresh();
    let s = w.nfl_season.clone().unwrap();
    let t = (0..w.nfl.len()).find(|&t| s.active[t] && w.nfl[t].head_coach.is_some()).unwrap();
    let old = w.nfl[t].head_coach.unwrap();
    let before = (w.coaches[old].wins, w.coaches[old].losses);
    let c = w.start_career_as(&you(), Start::Head(Job::Nfl(t)), 5);
    // The man you replaced keeps the games already played.
    assert_eq!((w.coaches[old].wins, w.coaches[old].losses), (before.0 + s.wins[t], before.1 + s.losses[t]));
    let tk = w.folk.takeover.clone().unwrap();
    assert!(tk.rush);
    let first = tk.interim.expect("a club still playing has an interim");
    assert!(w.nfl[t].oc == Some(first) || w.nfl[t].dc == Some(first), "a coordinator");
    assert_eq!(w.seat_interim(), Some(first));
    assert_eq!(w.nfl_head(t), Some(first));
    assert_eq!(w.folk.takeover.as_ref().unwrap().team, Some(Team::Nfl(t)));
    assert_eq!(w.stint_record(c), (0, 0), "the interim's games aren't yours");
    // Let him go at his holdover meeting: the other coordinator takes over.
    crate::world::staff::leave(&mut w, first, "fired");
    let int = w.folk.takeover.as_ref().unwrap().interim.expect("the other coordinator");
    assert_ne!(int, first);
    assert!(w.nfl[t].oc == Some(int) || w.nfl[t].dc == Some(int));
    assert_eq!(w.seat_interim(), Some(int));
    // The first team meeting waits for the offseason program.
    let v = scenes_of(&w);
    let meet = v.iter().find(|s| s.kind == Kind::TeamMeeting).unwrap();
    assert!(meet.date > w.today.add(30), "{}", meet.date.pretty());
    assert!(v.iter().filter(|s| s.kind != Kind::TeamMeeting).all(|s| s.date <= w.today.add(1)));

    // Black Monday: the book's season is the interim's, none of it yours.
    let at = tk.nfl_at.unwrap();
    let mine = (w.coaches[c].wins, w.coaches[c].losses);
    let his = (w.coaches[int].wins, w.coaches[int].losses);
    let bm = w.dates.black_monday;
    go_to(&mut w, bm.add(1));
    let book = w.nfl_seasons.iter().rev().find(|x| x.team == t && x.year == s.year).expect("the season's book");
    assert_eq!(book.coach, Some(old), "the coach of most of its games, as the books have it");
    assert_eq!((w.coaches[c].wins, w.coaches[c].losses), mine, "you coached none of it");
    assert!(!w.nfl_book.coach_awards.iter().any(|a| a.0 == s.year && a.4 == t && a.1 == "Coach of the Year"));
    assert_eq!(w.coaches[int].wins - his.0, book.wins - at.0);
    assert_eq!(w.coaches[int].losses - his.1, book.losses - at.1);
}

#[test]
fn an_nfl_assistant_on_jan1_works_under_the_head_coach_who_has_the_games() {
    let mut w = fresh();
    let s = w.nfl_season.clone().unwrap();
    let t = (0..w.nfl.len()).find(|&t| s.active[t] && w.nfl[t].head_coach.is_some()).unwrap();
    let head = w.nfl[t].head_coach;
    w.start_career_as(&you(), Start::Assistant(Job::Nfl(t), crate::world::coach::Role::OffensiveCoordinator), 5);
    assert_eq!(w.nfl[t].head_coach, head);
    assert!(w.folk.takeover.as_ref().is_none_or(|tk| tk.interim.is_none() && tk.nfl_at.is_none()));
    assert!(w.seat_interim().is_none());
    assert_eq!(w.nfl_head(t), head);
}
