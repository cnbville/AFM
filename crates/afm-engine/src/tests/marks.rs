//! The record store's small parts (lane L20 review fixes).

use crate::world::marks::{CAREER, Mark, SEASON, career_aux, list_key, place_mark};

#[test]
fn career_marks_keep_games_past_255() {
    // A kicker's 18 seasons at one club: 306 games.
    let m = Mark { aux: career_aux(2001, 306), games: 255, list: list_key(CAREER, 0), ..Default::default() };
    assert_eq!(m.game_count(), 306);
    assert_eq!(m.first_year(), 2001);
    // An older file kept the first season alone in `aux`.
    let old = Mark { aux: 2001, games: 40, list: list_key(CAREER, 0), ..Default::default() };
    assert_eq!(old.game_count(), 40);
    assert_eq!(old.first_year(), 2001);
    let season = Mark { games: 12, year: 2010, list: list_key(SEASON, 0), ..Default::default() };
    assert_eq!(season.game_count(), 12);
}

#[test]
fn a_full_team_list_never_grows_its_book() {
    let top = 3;
    let mark = |value: f32, id: u32| Mark { value, id, year: 2020, list: list_key(SEASON, 2), ..Default::default() };
    let mut book: Vec<Mark> = vec![mark(300.0, 1), mark(200.0, 2), mark(100.0, 3)];
    book.shrink_to_fit();
    let cap = book.capacity();
    let old = place_mark(&mut book, mark(400.0, 4), top);
    assert_eq!(old.map(|m| m.id), Some(1), "the holder it moved off first place");
    assert_eq!(book.iter().map(|m| m.id).collect::<Vec<_>>(), [4, 1, 2]);
    assert_eq!(book.capacity(), cap, "a loaded full book keeps its size");
    assert!(place_mark(&mut book, mark(50.0, 5), top).is_none());
    assert_eq!(book.len(), 3);
}
