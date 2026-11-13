//! The inbox lane (Cain 2026-11-12): the portal comes as messages with the
//! names matched, not as a small line on the Continue card. A day's
//! entrants are one message, your own are another that needs you, and where
//! your former players landed is told once.

use afm_engine::calendar::{Event, Kind, Level as CalLevel, Loudness};
use afm_engine::player::PlayingTime;
use afm_engine::world::{PortalEntry, Stage};
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

fn subject_starting<'a>(rows: &'a [Value], start: &str) -> Vec<&'a Value> {
    rows.iter().filter(|r| r["subject"].as_str().is_some_and(|s| s.starts_with(start))).collect()
}

#[test]
fn the_portal_comes_as_messages_with_the_names_matched() {
    let dir = std::env::temp_dir().join(format!("afm-inbox-portal-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 12, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    // A day of entrants: thirty players at other schools, eight of them
    // five-star starters, and one of ours.
    let (pi, n, own_id) = {
        let w = g.world.as_mut().unwrap();
        let Some(TeamRef::College(pi)) = views::mine(w) else { panic!("a college seat") };
        let today = w.today;
        let entry =
            PortalEntry { entered: today, decide_by: today.add(9), why: "playing time", held: false, looking: false };
        let mut n = 0;
        for p in w.people.iter_mut() {
            if n < 30 && p.stage == Stage::College && p.college.is_some_and(|c| c != pi) && p.cal.portal.is_none() {
                if n < 8 {
                    p.stars = 5;
                    p.last_role = Some(PlayingTime::Starter);
                }
                p.cal.portal = Some(entry);
                n += 1;
            }
        }
        assert_eq!(n, 30, "enough college players at other schools");
        let own = w
            .people
            .iter_mut()
            .find(|p| p.stage == Stage::College && p.college == Some(pi) && p.cal.portal.is_none())
            .expect("a player of ours");
        own.cal.portal = Some(entry);
        (pi, n, own.player.id)
    };
    // Two Medium lines of my own school's, the kind the card keeps when it
    // runs long: a man of mine entering (a portal message carries him) and a
    // line of the same kind no message carries (it stays, so the card is live).
    {
        let w = g.world.as_mut().unwrap();
        let (today, at) = (w.today, w.now().add_minutes(60));
        for text in ["Ty Starter (QB, Home) enters the transfer portal.", "Home adds a transfer coach."] {
            let e = Event::new(today, CalLevel::College, Kind::Transfer, Loudness::Medium, text);
            w.calendar.post(e.program(pi).at(at));
        }
    }
    let card = ok(&mut g, "holiday", json!({ "days": 1 }));
    assert!(card.to_string().contains("adds a transfer coach"), "the card shows what no message carries: {card}");
    assert!(
        !card.to_string().contains("enters the transfer portal"),
        "the card lists the message, not the entrant: {card}"
    );
    let all = rows(&mut g);

    // The world has its own entrants on those days too, so find the day by the
    // size of the crowd I put in. The day is one message, with a count and the
    // names of the few that matter.
    let day: Vec<&Value> = subject_starting(&all, "Portal day:")
        .into_iter()
        .filter(|r| r["count"].as_u64().is_some_and(|c| c >= n as u64))
        .collect();
    assert_eq!(day.len(), 1, "one portal-day message for the day: {all:?}");
    let day = day[0];
    assert!(day["count"].as_u64().unwrap() >= n as u64, "{day}");
    assert_eq!(day["kind"], "news", "adding names to a list is not a need: {day}");
    let letter = ok(&mut g, "letter", json!({ "id": day["id"] }));
    let table = letter["table"].as_array().unwrap();
    assert!(
        table.iter().any(|r| r.as_array().is_some_and(|r| r.len() == 1 && r[0].as_str().unwrap().starts_with("## "))),
        "the table has its group headings: {letter}"
    );
    let named = table.iter().filter(|r| r[0].as_str().is_some_and(|c| c.starts_with("[[p:"))).count();
    assert!((8..=20).contains(&named), "the five-star starters and whoever fits are named, linked: {named}");
    let add = letter["answers"].as_array().unwrap();
    assert!(add.iter().any(|a| a["label"].as_str().unwrap().starts_with("Add all")), "{letter}");

    // "Add all named" puts every one of them on my players.
    let before = ok(&mut g, "my_players", json!({}))["rows"].as_array().unwrap().len();
    let pick = add.iter().position(|a| a["label"].as_str().unwrap().starts_with("Add all")).unwrap();
    ok(&mut g, "mail_answer", json!({ "id": day["id"], "pick": pick }));
    let mine = ok(&mut g, "my_players", json!({}));
    assert_eq!(mine["rows"].as_array().unwrap().len(), before + named, "{mine}");

    // Our own entrant is a message that needs us, with his name matched.
    let ours: Vec<&Value> = subject_starting(&all, "Portal:").into_iter().filter(|r| r["day"] == day["day"]).collect();
    assert_eq!(ours.len(), 1, "{all:?}");
    assert_eq!(ours[0]["kind"], "need");
    assert!(ok(&mut g, "mail", json!({}))["need"].as_u64().unwrap() >= 1);
    let l = ok(&mut g, "letter", json!({ "id": ours[0]["id"] }));
    assert!(l["table"].to_string().contains(&format!("[[p:{own_id}|")), "{l}");

    // Another day on: still no entrant lines on the card.
    let card = ok(&mut g, "holiday", json!({ "days": 1 }));
    assert!(!card.to_string().contains("enters the transfer portal"), "{card}");

    // A former player of ours who landed elsewhere is told once.
    let former = {
        let w = g.world.as_mut().unwrap();
        let j = (0..w.programs.len()).find(|&j| j != pi).unwrap();
        let id = {
            let p = w
                .people
                .iter_mut()
                .find(|p| p.player.id != own_id && p.stage == Stage::College && p.college == Some(pi))
                .unwrap();
            p.college = Some(j);
            p.cal.portal = None;
            p.player.id
        };
        w.front_office.college.left.push(id);
        id
    };
    ok(&mut g, "holiday", json!({ "days": 1 }));
    let told = |g: &mut Game| -> usize {
        let all = rows(g);
        let mark = format!("[[p:{former}|");
        subject_starting(&all, "Portal landings")
            .into_iter()
            .filter(|r| ok(g, "letter", json!({ "id": r["id"] }))["body"].to_string().contains(&mark))
            .count()
    };
    assert_eq!(told(&mut g), 1, "the landing of a man of ours is told, with his name matched");
    ok(&mut g, "holiday", json!({ "days": 2 }));
    assert_eq!(told(&mut g), 1, "he is not told again");
    // For looking at the inbox in a browser: AFM_DUMP_DIR gets the screens' data.
    if let Ok(out) = std::env::var("AFM_DUMP_DIR") {
        std::fs::create_dir_all(&out).unwrap();
        let mail = ok(&mut g, "mail", json!({}));
        let mut letters = serde_json::Map::new();
        for r in mail["rows"].as_array().unwrap() {
            let id = r["id"].as_str().unwrap().to_string();
            let l = ok(&mut g, "letter", json!({ "id": id }));
            letters.insert(id, l);
        }
        let status = ok(&mut g, "status", json!({}));
        let dump = json!({ "status": status, "mail": mail, "letters": letters });
        std::fs::write(format!("{out}/inbox.json"), dump.to_string()).unwrap();
    }
    std::fs::remove_dir_all(dir).ok();
}
