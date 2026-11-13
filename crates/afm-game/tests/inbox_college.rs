//! The inbox lane, part B (Cain 2026-11-12): what a college seat hears in
//! its inbox besides the portal: the coaching carousel, the NCAA's letter,
//! a practice case found out, the Heisman finalists and the All-America
//! teams, each with the names linked.

use afm_engine::calendar::{Date, Event, Kind, Level as CalLevel, Loudness, Moment, Weekday};
use afm_engine::training::seat::Case;
use afm_engine::world::Stage;
use afm_engine::world::data::Division;
use afm_engine::world::news::{Channel, NewsItem, NewsKind};
use afm_engine::world::records::Honors;
use afm_game::Game;
use afm_game::views::{self, TeamRef};
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

fn rows(g: &mut Game) -> Vec<Value> {
    ok(g, "mail", json!({}))["rows"].as_array().unwrap().clone()
}

fn with<'a>(rows: &'a [Value], start: &str) -> Vec<&'a Value> {
    rows.iter().filter(|r| r["subject"].as_str().is_some_and(|s| s.starts_with(start))).collect()
}

fn news(kind: NewsKind, program: usize, date: Date, text: &str) -> NewsItem {
    NewsItem {
        date: Some(date),
        at: None,
        year: date.year(),
        kind,
        channel: Channel::News,
        program: Some(program),
        text: text.into(),
        refs: Vec::new(),
    }
}

#[test]
fn a_college_seat_hears_the_carousel_the_ncaa_and_the_honors_in_its_inbox() {
    let dir = std::env::temp_dir().join(format!("afm-inbox-college-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 12, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    let (days, own_id, own_name) = {
        let w = g.world.as_mut().unwrap();
        let Some(TeamRef::College(pi)) = views::mine(w) else { panic!("a college seat") };
        let today = w.today;
        // The first Monday three days out: its memo reads the Sunday before.
        let mut monday = today.add(3);
        while monday.weekday() != Weekday::Mon {
            monday = monday.add(1);
        }
        let sunday = monday.add(-1);
        let div: Division = w.programs[pi].division;
        let other = (0..w.programs.len()).find(|&p| p != pi && w.programs[p].division == div).expect("a rival program");
        let other_name = w.program_name(other).to_string();
        for (kind, text) in [
            (NewsKind::CoachFired, format!("{other_name} fires head coach Sam Lee after 3 seasons.")),
            (
                NewsKind::CoachHired,
                format!("{other_name} hires Pat Moe as offensive coordinator (from a high school job)."),
            ),
            (
                NewsKind::CoachExtended,
                format!("{other_name} extends head coach Kim Roe's contract through the 2030 season."),
            ),
        ] {
            w.news.push(news(kind, other, sunday, &text));
        }
        w.news.push(news(
            NewsKind::NcaaPenalty,
            pi,
            monday.add(-2),
            "The NCAA penalizes Home for a booster's payments to 3 recruits (about $90K): 6 scholarships over three seasons.",
        ));
        let case_day = monday.add(-3);
        w.training.cases.push(Case {
            date: case_day,
            week: case_day,
            what: vec!["a week over the practice limit".into()],
            how: "a player's parent called the compliance office".into(),
            penalty: "two practices are lost".into(),
        });
        // One of my men is a Heisman finalist and a first-team All-American
        // (the All-America line itself names nobody).
        let own = w
            .people
            .iter()
            .find(|p| p.stage == Stage::College && p.college == Some(pi))
            .map(|p| (p.player.id, p.name.clone()))
            .expect("a player of ours");
        let finalists = format!("Heisman finalists: {} (QB, Home), Other Guy (RB, Away).", own.1);
        let e = Event::new(sunday, CalLevel::College, Kind::Award, Loudness::Medium, finalists)
            .person(own.0, &own.1)
            .at(Moment::at(sunday, 18, 0));
        w.calendar.post(e);
        let aa_day = monday.add(-2);
        let e = Event::new(
            aa_day,
            CalLevel::College,
            Kind::Award,
            Loudness::Medium,
            "All-America teams are named: 25 players honored nationally.",
        )
        .at(Moment::at(aa_day, 12, 0));
        w.calendar.post(e);
        w.stats.honors.insert(
            own.0,
            Honors { name: own.1.clone(), all_america: vec![(aa_day.year(), div)], ..Honors::default() },
        );
        (today.days_to(monday.add(2)), own.0, own.1)
    };
    ok(&mut g, "holiday", json!({ "days": days }));
    let all = rows(&mut g);

    // The carousel: the Monday's memo has the firing in its table, the
    // assistant hire and the extension told in lines (the world's own moves
    // of an earlier Monday in the stretch may have a memo too).
    let carousel = with(&all, "Coaching carousel:");
    assert!(!carousel.is_empty(), "a carousel memo for the Monday: {all:?}");
    let mut letters = Vec::new();
    for r in &carousel {
        letters.push(ok(&mut g, "letter", json!({ "id": r["id"] })));
    }
    let letter = letters
        .iter()
        .find(|l| l["table"].to_string().contains("fires head coach Sam Lee"))
        .unwrap_or_else(|| panic!("the firing is in a carousel table: {letters:?}"));
    assert!(letter["body"].to_string().contains("extension"), "{letter}");
    assert!(letter["body"].to_string().contains("assistant"), "{letter}");

    // The NCAA's letter, from my athletic director.
    let ncaa = with(&all, "NCAA penalties");
    assert_eq!(ncaa.len(), 1, "{all:?}");
    let letter = ok(&mut g, "letter", json!({ "id": ncaa[0]["id"] }));
    assert!(letter["body"].to_string().contains("booster's payments"), "{letter}");

    // The practice case, from the compliance office.
    let case = with(&all, "Practice rules: a week over the practice limit");
    assert_eq!(case.len(), 1, "{all:?}");
    let letter = ok(&mut g, "letter", json!({ "id": case[0]["id"] }));
    assert!(letter["body"].to_string().contains("two practices are lost"), "{letter}");

    // The Heisman finalists: only my man is told, linked, with the field.
    let honors = with(&all, &format!("Honors: {own_name} is a Heisman finalist"));
    assert_eq!(honors.len(), 1, "{all:?}");
    let letter = ok(&mut g, "letter", json!({ "id": honors[0]["id"] }));
    assert!(letter["body"].to_string().contains(&format!("[[p:{own_id}|")), "{letter}");
    assert!(letter["body"].to_string().contains("Other Guy"), "{letter}");

    // The All-America teams: read from the honors book, a table of my men.
    let aa = with(&all, "All-America:");
    assert_eq!(aa.len(), 1, "{all:?}");
    let letter = ok(&mut g, "letter", json!({ "id": aa[0]["id"] }));
    assert!(letter["table"].to_string().contains(&format!("[[p:{own_id}|")), "{letter}");
}
