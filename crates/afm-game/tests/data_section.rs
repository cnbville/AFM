//! Lane L20, the Data section: a small world plays a season at a college
//! seat, and the record store fills at every level (single games, team
//! books, the NFL's and the states' books, rate records, the era column),
//! while every Data page answers at each level.

use afm_engine::world::marks::{CAREER, GAME, SEASON, Tier};
use afm_engine::world::records::{Book, Scope};
use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn the_books_fill_and_every_data_page_answers() {
    let dir = std::env::temp_dir().join(format!("afm-data-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 11, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));
    let start = g.world.as_ref().unwrap().year;
    // Through the college season, the high school season and the NFL's
    // regular season.
    for _ in 0..12 {
        ok(&mut g, "holiday", json!({ "days": 30 }));
        let w = g.world.as_ref().unwrap();
        if w.marks.leaders.iter().any(|l| l.scope == Scope::Nfl) {
            break;
        }
    }
    let w = g.world.as_ref().unwrap();
    assert!(w.year >= start);

    // Single games at every level, with the game they came from.
    for scope in [Scope::Nfl] {
        let list = w.stats.books.get(&(scope.clone(), Book::Game(0))).expect("the NFL's single-game passing list");
        assert!(!list.is_empty() && list.windows(2).all(|p| p[0].value >= p[1].value), "{scope:?}");
        assert!(list[0].game.is_some(), "a single game keeps its game");
    }
    let college_games =
        w.stats.books.keys().filter(|k| matches!(k.0, Scope::Division(_)) && matches!(k.1, Book::Game(_))).count();
    assert!(college_games > 0, "college single-game lists");
    let state_games =
        w.stats.books.keys().filter(|k| matches!(k.0, Scope::State(_)) && matches!(k.1, Book::Game(_))).count();
    assert!(state_games > 0, "state single-game lists");
    // The NFL's season and career lists, leaders and analytics rows.
    assert!(w.stats.books.get(&(Scope::Nfl, Book::Season(0))).is_some_and(|l| !l.is_empty()));
    assert!(w.stats.books.get(&(Scope::Nfl, Book::Career(2))).is_some_and(|l| !l.is_empty()));
    assert!(!w.marks.pro_rows.is_empty(), "the NFL's analytics rows");
    // Rate records hold only qualified seasons, best first.
    let rates = w.stats.books.get(&(Scope::Nfl, Book::RateSeason(0))).expect("NFL passer rating list");
    assert!(rates.windows(2).all(|p| p[0].value >= p[1].value));
    assert!(rates.iter().all(|e| e.value > 0.0 && e.value <= 158.4), "passer ratings stay in the formula's range");
    // Team books: every list sorted, no deeper than the tier allows, a
    // career once per player.
    for (tier, books) in [(Tier::College, &w.marks.college), (Tier::Nfl, &w.marks.nfl), (Tier::Hs, &w.marks.hs)] {
        assert!(books.iter().any(|b| !b.is_empty()), "{tier:?} team books");
        for b in books {
            assert!(b.windows(2).all(|p| p[0].list & 63 <= p[1].list & 63), "lists in order");
            for kind in [CAREER, SEASON, GAME] {
                for j in 0..afm_engine::world::marks::N {
                    let l = afm_engine::world::marks::team_list(b, kind, j);
                    assert!(l.len() <= tier.team_top());
                    assert!(l.windows(2).all(|p| p[0].value >= p[1].value));
                    if kind == CAREER {
                        let mut ids: Vec<u32> = l.iter().map(|m| m.id).collect();
                        ids.sort_unstable();
                        ids.dedup();
                        assert_eq!(ids.len(), l.len(), "one career mark each");
                    }
                }
            }
        }
    }
    // The era column: an NFL season leader carries one.
    let lead = w.marks.leaders.iter().find(|l| l.scope == Scope::Nfl && l.stat == 0).expect("the NFL's passing leader");
    assert!(lead.first.era > 0.0, "{:?}", lead.first);

    // Every Data page answers, at each level.
    let src = ok(&mut g, "data_sources", json!({}));
    assert_eq!(src["seat"], "college");
    for level in ["college", "nfl", "hs"] {
        for tab in ["career", "season", "game", "rates", "leaders", "team"] {
            let x = ok(&mut g, "stat_history", json!({ "level": level, "tab": tab }));
            assert_eq!(x["level"], level, "{x}");
            assert!(x.get("chasing").is_some() && x.get("fell").is_some());
        }
        let a = ok(&mut g, "analytics", json!({ "level": level, "group": "QB" }));
        assert!(a["columns"].as_array().is_some_and(|c| !c.is_empty()), "{level}: {a}");
    }
    let nfl = ok(&mut g, "stat_history", json!({ "level": "nfl", "tab": "season" }));
    let first = &nfl["lists"][0];
    assert_eq!(first["real"]["holder"], "Peyton Manning", "the real record on top");
    assert!(!first["rows"].as_array().unwrap().is_empty());
    let mine = ok(&mut g, "stat_history", json!({ "tab": "team" }));
    assert!(mine["team_book"]["lists"].as_array().is_some_and(|l| !l.is_empty()), "your team's book");
    let qb = ok(&mut g, "analytics", json!({ "level": "nfl", "group": "QB", "min": 0 }));
    if let Some(id) = qb["rows"][0]["id"].as_u64() {
        let card = ok(&mut g, "analytics_card", json!({ "id": id, "level": "nfl" }));
        assert!(card["metrics"].as_array().is_some_and(|m| !m.is_empty()), "{card}");
    }
    let roster = ok(&mut g, "roster", json!({}));
    let pid = roster["rows"][0]["id"].as_u64().unwrap();
    ok(&mut g, "analytics_card", json!({ "id": pid }));
    for tab in ["conferences", "states", "award"] {
        let x = ok(&mut g, "champions_more", json!({ "tab": tab }));
        assert!(x["rows"].is_array(), "{x}");
    }
    let confs = ok(&mut g, "champions_more", json!({ "tab": "conferences" }));
    assert!(!confs["rows"].as_array().unwrap().is_empty(), "a conference champion: {confs}");
    // The old record book link still lands, and the history answers.
    ok(&mut g, "history", json!({ "tab": "awards" }));
    // Names outlive the world's cleanup: every team book holder has one.
    let w = g.world.as_ref().unwrap();
    for b in w.marks.college.iter().chain(&w.marks.nfl).take(50) {
        for m in b {
            assert!(afm_engine::world::marks::name_of(w, m.id).is_some(), "a name for {}", m.id);
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}
